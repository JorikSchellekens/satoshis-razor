//! The remote participation API, served by `razor serve`.
//!
//! Three endpoints turn the hosted registry into the CLI's default target:
//!
//!   GET  /api/log?since=N   the event log from seq N (JSONL, as committed)
//!   POST /api/event         a signed (or unsigned) log event
//!   POST /api/submit        a proof submission + optional .lean file;
//!                           verified immediately, verdict in the response
//!   POST /api/verify        re-run verification for a pending submission
//!
//! The server is a sequencer, not an authority: it validates the same
//! invariants the CLI validates, requires a valid Ed25519 signature for any
//! registered handle, runs kernel checks in a throwaway container, and
//! publishes every append to the public mirror. Anyone can replay the log
//! and re-check every claim without trusting it.

use crate::model::{Event, State};
use crate::{load, ui};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// Set after every successful append; the mirror thread pushes and clears.
pub static MIRROR_DIRTY: AtomicBool = AtomicBool::new(false);

// ---------------- request plumbing ----------------

pub struct Request {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub body: Vec<u8>,
    pub ip: String,
}

/// Read one HTTP request (line, headers, body) from the stream.
pub fn read_request(reader: &mut impl std::io::BufRead, peer: &str) -> Option<Request> {
    let mut req_line = String::new();
    reader.read_line(&mut req_line).ok()?;
    let mut parts = req_line.split_whitespace();
    let method = parts.next()?.to_string();
    let full = parts.next().unwrap_or("/");
    let (path, query) = match full.split_once('?') {
        Some((p, q)) => (p.to_string(), Some(q.to_string())),
        None => (full.to_string(), None),
    };
    let mut content_len = 0usize;
    let mut ip = peer.to_string();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            if k == "content-length" {
                content_len = v.parse().unwrap_or(0);
            }
            // Behind the reverse proxy every peer is localhost; the proxy
            // forwards the real client address.
            if k == "x-forwarded-for" {
                if let Some(first) = v.split(',').next() {
                    ip = first.trim().to_string();
                }
            }
        }
    }
    // 2 MB cap: the largest legitimate body is a base64 proof file.
    if content_len > 2 * 1024 * 1024 {
        return None;
    }
    let mut body = vec![0u8; content_len];
    reader.read_exact(&mut body).ok()?;
    Some(Request { method, path, query, body, ip })
}

// ---------------- rate limiting ----------------

#[derive(Default)]
pub struct Limiter {
    hits: std::collections::HashMap<String, Vec<u64>>,
}

impl Limiter {
    /// Allow `max` hits per hour per key.
    pub fn allow(&mut self, key: &str, max: usize) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let v = self.hits.entry(key.to_string()).or_default();
        v.retain(|t| now.saturating_sub(*t) < 3600);
        if v.len() >= max {
            return false;
        }
        v.push(now);
        true
    }
}

// ---------------- responses ----------------

pub fn json_response(status: &str, value: serde_json::Value) -> (String, String, Vec<u8>) {
    (status.to_string(), "application/json".to_string(), value.to_string().into_bytes())
}

fn ok(value: serde_json::Value) -> (String, String, Vec<u8>) {
    let mut v = value;
    v["ok"] = serde_json::Value::Bool(true);
    json_response("200 OK", v)
}

fn err(status: &str, msg: &str) -> (String, String, Vec<u8>) {
    json_response(status, serde_json::json!({ "ok": false, "error": msg }))
}

// ---------------- endpoint: GET /api/log ----------------

pub fn get_log(log_path: &PathBuf, query: Option<&str>) -> (String, String, Vec<u8>) {
    let since: usize = query
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("since=")))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let text = std::fs::read_to_string(log_path).unwrap_or_default();
    let tail: String = text.lines().skip(since).map(|l| format!("{l}\n")).collect();
    ("200 OK".into(), "application/jsonl".into(), tail.into_bytes())
}

// ---------------- endpoint: GET /api/submission ----------------

/// Hand out the .lean file a submission was installed as, so `razor
/// recheck` works from a checkout older than the submission. Read-only:
/// the same bytes are in the mirror, this is just the low-latency path.
pub fn get_submission(root: &PathBuf, log_path: &PathBuf, query: Option<&str>) -> (String, String, Vec<u8>) {
    let Some(id) = query
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("id=")))
        .filter(|id| sane_id(id))
    else {
        return err("400 Bad Request", "usage: /api/submission?id=SUB-...");
    };
    let state = State::fold(load(log_path));
    let Some((sorry, sub)) = state.sorries.values()
        .find_map(|h| h.submissions.iter().find(|s| s.id == id).map(|s| (h, s)))
    else {
        return err("404 Not Found", "unknown submission");
    };
    let Some(module) = &sub.module else {
        return err("404 Not Found", "this submission has no installed file (it names a package declaration)");
    };
    let (lean_dir, _) = crate::env_of(root, sorry);
    let path = lean_dir.join(module.replace('.', "/") + ".lean");
    match std::fs::read(&path) {
        Ok(bytes) => ok(serde_json::json!({
            "submission": id, "module": module, "file_b64": base64_encode(&bytes),
        })),
        Err(_) => err("404 Not Found",
            "the submission's file is not installed here (rejected files are removed)"),
    }
}

// ---------------- endpoint: POST /api/event ----------------

/// The event types a remote client may append. Everything here is either a
/// pure statement of intent (propose, curate, fund ...) or carries its own
/// proof of validity (a commitment, a signature, a hash check). Types that
/// assert kernel facts without a kernel check (converge, implies, certify),
/// and types only the verifier may write (verdict, payout), are refused.
fn remote_allowed(event: &Event) -> Result<(), &'static str> {
    match event {
        Event::Propose { .. } | Event::Formalize { .. } | Event::SealStatement { .. }
        | Event::RevealStatement { .. } | Event::RegisterSorry { .. } | Event::OpenRound { .. }
        | Event::Curate { .. } | Event::Supersede { .. } | Event::Fund { .. }
        | Event::RegisterAccount { .. } | Event::Commit { .. } | Event::Split { .. }
        | Event::Tag { .. } => Ok(()),
        // The forge. A challenge, a lane, and a rig are statements of
        // intent; a score is a physical measurement no server can re-run,
        // so it must name a registered rig and carry the rig owner's
        // signature - the score's trust model is exactly its rig's.
        Event::RegisterChallenge { .. } | Event::AnvilSubmit { .. }
        | Event::RegisterRig { .. } | Event::Bench { .. } | Event::PinWorkload { .. } => Ok(()),
        Event::Converge { .. } | Event::Implies { .. } =>
            Err("converge/implies assert an equivalence without a kernel check and are not accepted \
                 remotely - use `razor bridge` (the equivalence becomes a sorry, proven like any other)"),
        Event::Certify { .. } =>
            Err("certify is not accepted remotely yet"),
        Event::Submit { .. } =>
            Err("use /api/submit (razor submit) so the proof is verified in the same call"),
        Event::Verdict { .. } | Event::Payout { .. } =>
            Err("verdicts and payouts are written only by the verifier"),
        _ => Err("this event type is not accepted remotely - run it against a local registry (--local)"),
    }
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn sane_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// The semantic checks the CLI performs locally, replayed against the
/// server's own state. The server is the authority; a hand-rolled client
/// gets exactly the same rules as the shipped one.
fn validate(state: &State, event: &Event, attachments: Option<&serde_json::Value>) -> Result<(), String> {
    let no = |msg: &str| Err(msg.to_string());
    match event {
        Event::Propose { id, title, .. } => {
            if !sane_id(id) { return no("proposal id: letters, digits, dashes, up to 64 chars"); }
            if state.proposals.contains_key(id) { return no("proposal id already exists"); }
            if title.trim().is_empty() { return no("a proposal needs a title"); }
        }
        Event::Formalize { id, proposal, decl, author, .. } => {
            if !sane_id(id) { return no("statement id: letters, digits, dashes, up to 64 chars"); }
            if state.statements.contains_key(id) { return no("statement id already exists"); }
            if !state.proposals.contains_key(proposal) { return no("unknown proposal"); }
            if decl.trim().is_empty() || author.trim().is_empty() { return no("formalize needs --decl and --author"); }
        }
        Event::SealStatement { id, proposal, commitment, author } => {
            if !sane_id(id) { return no("seal id: letters, digits, dashes, up to 64 chars"); }
            if state.seals.contains_key(id) { return no("seal id already exists"); }
            if !state.proposals.contains_key(proposal) { return no("unknown proposal"); }
            if !is_hex(commitment, 64) { return no("commitment must be 64 hex chars (razor seal prints it)"); }
            if author.trim().is_empty() { return no("seal-statement needs --author"); }
        }
        Event::RevealStatement { seal, statement, decl, .. } => {
            let Some(s) = state.seals.get(seal) else { return no("unknown seal") };
            if s.statement.is_some() { return no("this seal is already revealed"); }
            if !sane_id(statement) { return no("statement id: letters, digits, dashes, up to 64 chars"); }
            if state.statements.contains_key(statement) { return no("statement id already exists"); }
            if decl.trim().is_empty() { return no("reveal-statement needs --decl"); }
            // The commitment check is what makes sealed provenance a fact:
            // the reveal must carry the file and salt that hash to it.
            let Some(att) = attachments else {
                return no("reveal-statement needs the statement file and salt (the CLI sends them)");
            };
            let file_b64 = att["file_b64"].as_str().unwrap_or_default();
            let salt = att["salt"].as_str().unwrap_or_default();
            let Some(bytes) = base64_decode(file_b64) else { return no("malformed file_b64") };
            if bytes.len() > 512 * 1024 { return no("statement file too large (512 KB cap)"); }
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(&bytes);
            h.update(salt.as_bytes());
            if format!("{:x}", h.finalize()) != s.commitment {
                return no("sha256(file ‖ salt) does not match the sealed commitment");
            }
        }
        Event::RegisterSorry { id, lean_type, proposal, env, bridge, .. } => {
            if !sane_id(id) { return no("sorry id: letters, digits, dashes, up to 64 chars"); }
            if state.sorries.contains_key(id) { return no("sorry id already exists"); }
            if lean_type.trim().is_empty() { return no("a sorry needs --lean-type"); }
            // A pin with shell-escape residue or control characters can
            // never elaborate; refuse it before it is permanent.
            if lean_type.contains("\\x") || lean_type.contains("\\u")
                || lean_type.chars().any(|c| c.is_control() && c != '\n') {
                return no("the pinned type contains escape sequences or control characters - \
                    it would never elaborate; check your shell quoting and re-send the literal statement");
            }
            if !matches!(env.as_deref(), None | Some("mathlib")) { return no("env must be omitted (core) or 'mathlib'"); }
            if let Some(p) = proposal {
                if !state.proposals.contains_key(p) { return no("unknown proposal"); }
            }
            if let Some((a, b)) = bridge {
                let (Some(sa), Some(sb)) = (state.statements.get(a), state.statements.get(b)) else {
                    return no("bridge: unknown statement");
                };
                if sa.proposal != sb.proposal { return no("bridge: the statements answer different proposals"); }
                let composed = format!("({}) ↔ ({})", sa.decl, sb.decl);
                if *lean_type != composed {
                    return Err(format!("bridge type must be composed mechanically: expected {composed}"));
                }
            }
        }
        Event::OpenRound { id, proposal, closes_at, reveal_by, author, .. } => {
            if !sane_id(id) { return no("round id: letters, digits, dashes, up to 64 chars"); }
            if state.rounds.contains_key(id) { return no("round id already exists"); }
            if !state.proposals.contains_key(proposal) { return no("unknown proposal"); }
            if reveal_by <= closes_at { return no("--reveal-by must be after --closes-at"); }
            if author.trim().is_empty() { return no("round needs --author"); }
        }
        Event::Curate { curator, target, .. } => {
            if curator.trim().is_empty() { return no("curate needs --curator"); }
            if !state.sorries.contains_key(target) && !state.proposals.contains_key(target) {
                return no("unknown curation target");
            }
        }
        Event::Supersede { sorry, replacement, by, .. } => {
            if by.trim().is_empty() { return no("supersede needs --by"); }
            if !state.sorries.contains_key(sorry) || !state.sorries.contains_key(replacement) {
                return no("unknown sorry");
            }
        }
        Event::Tag { target, tag, by, .. } => {
            if by.trim().is_empty() { return no("tag needs --author"); }
            if tag.is_empty() || tag.len() > 32
                || !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return no("tags are lowercase letters, digits, and dashes (up to 32 chars)");
            }
            if !state.sorries.contains_key(target) && !state.proposals.contains_key(target)
                && !state.statements.contains_key(target) && !state.accounts.contains_key(target)
                && !state.challenges.contains_key(target) {
                return no("unknown tag target");
            }
        }
        Event::Fund { target, amount, funder, .. } => {
            if funder.trim().is_empty() { return no("fund needs --funder"); }
            if *amount == 0 { return no("fund needs a positive --amount"); }
            if !state.sorries.contains_key(target) && !state.challenges.contains_key(target) {
                return no("unknown funding target");
            }
        }
        Event::RegisterAccount { handle, pubkey, .. } => {
            if handle.is_empty() || handle.len() > 32
                || !handle.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return no("handles are lowercase letters, digits, and dashes (up to 32 chars)");
            }
            if state.accounts.contains_key(handle) { return no("handle is taken"); }
            if !is_hex(pubkey, 64) { return no("pubkey must be 64 hex chars (razor account new generates it)"); }
        }
        Event::Commit { id, sorry, commitment, solver } => {
            if !sane_id(id) { return no("submission id: letters, digits, dashes, up to 64 chars"); }
            if submission_exists(state, id) { return no("submission id already exists"); }
            if !state.sorries.contains_key(sorry) { return no("unknown sorry"); }
            if !is_hex(commitment, 64) { return no("commitment must be 64 hex chars (razor seal prints it)"); }
            if solver.trim().is_empty() { return no("commit needs --solver"); }
        }
        Event::Split { parent, children, glue, author, .. } => {
            if author.trim().is_empty() { return no("split needs --author"); }
            if !state.sorries.contains_key(parent) { return no("unknown parent sorry"); }
            if !state.sorries.contains_key(glue) { return no("unknown glue sorry - register it first"); }
            if children.is_empty() { return no("a split needs at least one child"); }
            for c in children {
                if !state.sorries.contains_key(c) { return Err(format!("unknown child sorry: {c}")); }
            }
        }
        Event::RegisterChallenge { id, title, spec_impl, obligation, seed, iters } => {
            if !sane_id(id) { return no("challenge id: letters, digits, dashes, up to 64 chars"); }
            if state.challenges.contains_key(id) { return no("challenge id already exists"); }
            if title.trim().is_empty() || spec_impl.trim().is_empty() || obligation.trim().is_empty() {
                return no("a challenge needs --title, --spec-impl, and --obligation");
            }
            // New challenges pin their workload at registration; without a
            // pin, scores at different workloads would silently mix.
            if seed.is_none() || iters.is_none() || *iters == Some(0) {
                return no("a challenge pins its benchmark workload at registration: pass --iters (and optionally --seed)");
            }
        }
        Event::PinWorkload { challenge, iters, .. } => {
            let Some(c) = state.challenges.get(challenge) else { return no("unknown challenge") };
            if c.workload.is_some() {
                return no("this challenge's workload is already pinned - a pin is permanent, because \
                    changing it would silently re-price every recorded score");
            }
            if *iters == 0 { return no("a workload needs a positive word count (--iters)"); }
        }
        Event::AnvilSubmit { id, challenge, impl_name, solver, refinement_sorry, .. } => {
            if !sane_id(id) { return no("submission id: letters, digits, dashes, up to 64 chars"); }
            let Some(c) = state.challenges.get(challenge) else { return no("unknown challenge") };
            if c.entries.iter().any(|e| e.id == *id) { return no("submission id already exists"); }
            if c.entries.iter().any(|e| e.impl_name == *impl_name) {
                return no("an entry with this implementation name already exists on the challenge");
            }
            if !sane_id(impl_name) { return no("impl name: letters, digits, dashes, up to 64 chars"); }
            if solver.trim().is_empty() { return no("forge-submit needs --solver"); }
            if let Some(h) = refinement_sorry {
                if !state.sorries.contains_key(h) { return no("unknown refinement sorry - register it first"); }
            }
        }
        Event::RegisterRig { id, owner, arch, tier, .. } => {
            if !sane_id(id) { return no("rig id: letters, digits, dashes, up to 64 chars"); }
            if state.rigs.contains_key(id) { return no("rig id already exists"); }
            if owner.trim().is_empty() || arch.trim().is_empty() { return no("a rig needs --owner and --arch"); }
            if !matches!(tier.as_str(), "wasm-fuel" | "native") {
                return no("rig tier must be wasm-fuel or native");
            }
        }
        Event::Bench { submission, tier, arch, score, unit, rig, seed, iters, .. } => {
            let Some(rig_id) = rig else {
                return no("a remote score must name the rig it was measured on (razor bench --rig)");
            };
            let Some(r) = state.rigs.get(rig_id) else { return no("unknown rig - register it first") };
            if r.tier != *tier {
                return Err(format!("rig {rig_id} is a {} rig; it cannot report {tier} scores", r.tier));
            }
            let found = state.challenges.values()
                .find_map(|c| c.entries.iter().find(|e| e.id == *submission).map(|e| (c, e)));
            let Some((chal, entry)) = found else { return no("unknown forge submission") };
            if !entry.admitted && !entry.is_reference {
                return no("this lane is not admitted yet - its refinement proof must verify first");
            }
            // Scores are comparable only at the challenge's pinned
            // workload; the registry accepts nothing else remotely.
            let Some((ps, pi)) = chal.workload else {
                return no("this challenge's workload is not pinned yet - pin it first (razor workload), \
                    then bench; scores at unpinned workloads are not comparable");
            };
            if *seed != Some(ps) || *iters != Some(pi) {
                return Err(format!(
                    "this challenge pins its workload at seed {ps}, {pi} words per run; \
                     the score was measured at a different workload and would not be comparable \
                     (razor bench uses the pin automatically)"));
            }
            if !score.is_finite() || *score <= 0.0 { return no("a score must be a positive number"); }
            if arch.trim().is_empty() || unit.trim().is_empty() { return no("a score needs an arch and a unit"); }
        }
        _ => return no("unreachable: event type already filtered"),
    }
    Ok(())
}

pub fn submission_exists(state: &State, id: &str) -> bool {
    state.sorries.values().any(|h| h.submissions.iter().any(|s| s.id == id)
        || h.zk_submissions.iter().any(|z| z.id == id))
}

/// A registered handle's event must carry a valid signature by its key; a
/// new account registration must be self-signed by the pubkey it declares.
fn check_signature(state: &State, event: &Event, sig: Option<&str>) -> Result<(), String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let verify_with = |pubkey_hex: &str, what: &str| -> Result<(), String> {
        let Some(sig) = sig else {
            return Err(format!("{what} requires an Ed25519 signature (the CLI signs with your key)"));
        };
        let vk = crate::bytes_of_hex(pubkey_hex)
            .and_then(|b| <[u8; 32]>::try_from(b).ok())
            .and_then(|b| VerifyingKey::from_bytes(&b).ok())
            .ok_or("malformed pubkey")?;
        let sig = crate::bytes_of_hex(sig)
            .and_then(|b| <[u8; 64]>::try_from(b).ok())
            .map(|b| Signature::from_bytes(&b))
            .ok_or("malformed signature")?;
        crate::model::canonical_forms(event).iter()
            .any(|msg| vk.verify(msg.as_bytes(), &sig).is_ok())
            .then_some(())
            .ok_or("signature does not verify against the registered key".to_string())
    };
    if let Event::RegisterAccount { pubkey, .. } = event {
        return verify_with(pubkey, "account registration");
    }
    // A bench score is signed by the owner of the rig it names: scores are
    // physical measurements, and the rig's owner is who vouches for them.
    if let Event::Bench { rig: Some(rig_id), .. } = event {
        if let Some(owner) = state.rigs.get(rig_id).map(|r| r.owner.clone()) {
            if let Some(acct) = state.accounts.get(&owner) {
                return verify_with(&acct.pubkey,
                    &format!("rig {rig_id} belongs to registered handle '@{owner}'; its scores"));
            }
        }
        return Ok(());
    }
    if let Some(actor) = event.actor() {
        if let Some(acct) = state.accounts.get(actor) {
            return verify_with(&acct.pubkey, &format!("'@{actor}' is a registered handle; this event"));
        }
    }
    Ok(())
}

pub fn post_event(
    root: &PathBuf,
    log_path: &PathBuf,
    body: &[u8],
    log_lock: &Mutex<()>,
) -> (String, String, Vec<u8>) {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return err("400 Bad Request", "body must be JSON: {event, sig?, attachments?}");
    };
    let Ok(event) = serde_json::from_value::<Event>(v["event"].clone()) else {
        return err("400 Bad Request", "unrecognized event shape");
    };
    if let Err(m) = remote_allowed(&event) {
        return err("403 Forbidden", m);
    }
    let sig = v["sig"].as_str().map(String::from);
    let attachments = if v["attachments"].is_object() { Some(&v["attachments"]) } else { None };

    // Append under the log lock so seq assignment and validation see a
    // consistent state.
    let _guard = log_lock.lock().unwrap();
    let mut state = State::fold(load(log_path));
    // A lane whose refinement sorry has an admitted proof is admitted; the
    // bench validation below needs that settled.
    state.settle_admissions();
    if let Err(m) = check_signature(&state, &event, sig.as_deref()) {
        return err("403 Forbidden", &m);
    }
    if let Err(m) = validate(&state, &event, attachments) {
        return err("400 Bad Request", &m);
    }
    // A verified reveal's file and salt become public record: persisted
    // next to the log and mirrored, so any third party can re-verify
    // sha256(file ‖ salt) against the sealed commitment - and read the Lean.
    if let (Event::RevealStatement { statement, .. }, Some(att)) = (&event, attachments) {
        if let (Some(bytes), Some(salt)) = (
            att["file_b64"].as_str().and_then(base64_decode),
            att["salt"].as_str(),
        ) {
            crate::persist_statement_file(log_path, statement, &bytes, salt);
        }
    }
    let entry = crate::append_entry(log_path, event, sig);
    let _ = root; // reserved for future per-event side effects
    MIRROR_DIRTY.store(true, Ordering::SeqCst);
    ok(serde_json::json!({ "seq": entry.seq, "ts": entry.ts, "entry": entry }))
}

// ---------------- endpoint: POST /api/submit and /api/verify ----------------

pub fn post_submit(
    root: &PathBuf,
    log_path: &PathBuf,
    body: &[u8],
    log_lock: &Mutex<()>,
    verify_lock: &Mutex<()>,
) -> (String, String, Vec<u8>) {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return err("400 Bad Request", "body must be JSON");
    };
    let Ok(event) = serde_json::from_value::<Event>(v["event"].clone()) else {
        return err("400 Bad Request", "unrecognized event shape");
    };
    let Event::Submit { id, sorry: sorry_id, solver, decl, module } = event.clone() else {
        return err("400 Bad Request", "event must be a submit");
    };
    let sig = v["sig"].as_str().map(String::from);

    {
        let _guard = log_lock.lock().unwrap();
        let state = State::fold(load(log_path));
        if !sane_id(&id) {
            return err("400 Bad Request", "submission id: letters, digits, dashes, up to 64 chars");
        }
        if submission_exists(&state, &id) {
            return err("400 Bad Request", "submission id already exists");
        }
        let Some(sorry) = state.sorries.get(&sorry_id) else {
            return err("400 Bad Request", "unknown sorry");
        };
        if sorry.env.as_deref() == Some("mathlib") {
            return err("501 Not Implemented",
                "this sorry verifies in the Mathlib environment, which this remote does not offer yet - \
                 verify locally (razor verify --local) and contact the maintainer");
        }
        if solver.trim().is_empty() || decl.trim().is_empty() {
            return err("400 Bad Request", "submit needs --solver and --decl");
        }
        if let Err(m) = check_signature(&state, &event, sig.as_deref()) {
            return err("403 Forbidden", &m);
        }
        // Install the proof file, if one was sent, exactly where the local
        // CLI would install it. The module name is deterministic from the
        // id, and the client signs the event with it included.
        if let Some(file_b64) = v["file_b64"].as_str() {
            let Some(bytes) = base64_decode(file_b64) else {
                return err("400 Bad Request", "malformed file_b64");
            };
            if bytes.len() > 512 * 1024 {
                return err("400 Bad Request", "proof file too large (512 KB cap)");
            }
            let (lean_dir, root_import) = crate::env_of(root, sorry);
            let expected = crate::submission_module(root_import, &id);
            if module.as_deref() != Some(expected.as_str()) {
                return err("400 Bad Request", &format!("module must be {expected} (the CLI sets it)"));
            }
            let dest = lean_dir.join(expected.replace('.', "/") + ".lean");
            std::fs::create_dir_all(dest.parent().unwrap()).ok();
            if std::fs::write(&dest, &bytes).is_err() {
                return err("500 Internal Server Error", "could not install the file");
            }
        } else if module.is_some() {
            return err("400 Bad Request", "module set but no file sent");
        }
        crate::append_entry(log_path, event, sig);
        MIRROR_DIRTY.store(true, Ordering::SeqCst);
    }

    // The kernel check runs outside the log lock (it takes seconds to
    // minutes) and serialized with other checks via the verify lock.
    run_verification(root, log_path, &id, log_lock, verify_lock)
}

pub fn post_verify(
    root: &PathBuf,
    log_path: &PathBuf,
    body: &[u8],
    log_lock: &Mutex<()>,
    verify_lock: &Mutex<()>,
) -> (String, String, Vec<u8>) {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(body) else {
        return err("400 Bad Request", "body must be JSON: {submission}");
    };
    let Some(id) = v["submission"].as_str() else {
        return err("400 Bad Request", "body must be JSON: {submission}");
    };
    run_verification(root, log_path, id, log_lock, verify_lock)
}

fn run_verification(
    root: &PathBuf,
    log_path: &PathBuf,
    id: &str,
    log_lock: &Mutex<()>,
    verify_lock: &Mutex<()>,
) -> (String, String, Vec<u8>) {
    let _v = verify_lock.lock().unwrap();
    match crate::verify_and_record(root, log_path, id, log_lock) {
        Ok(outcome) => {
            // A rejected file-submission leaves no source in the tree
            // (mirroring the local CLI, which removes a file that fails to
            // build) - otherwise the mirror would push code that breaks
            // everyone's `lake build`. The rejection itself stays recorded.
            if !outcome.admitted {
                let state = State::fold(load(log_path));
                if let Some((sorry, sub)) = state.sorries.values()
                    .find_map(|h| h.submissions.iter().find(|s| s.id == id).map(|s| (h, s)))
                {
                    if let Some(module) = &sub.module {
                        let (lean_dir, _) = crate::env_of(root, sorry);
                        let _ = std::fs::remove_file(lean_dir.join(module.replace('.', "/") + ".lean"));
                    }
                }
            }
            MIRROR_DIRTY.store(true, Ordering::SeqCst);
            ok(serde_json::json!({
                "submission": id,
                "admitted": outcome.admitted,
                "axioms": outcome.axioms,
                "detail": outcome.detail,
                "cost_ms": outcome.cost_ms,
                "pinned": outcome.pinned,
                "payout": outcome.payout,
            }))
        }
        Err(m) => err("400 Bad Request", &m),
    }
}

// ---------------- the mirror ----------------

/// Publish every append to the public repository. The box's log is the
/// canonical one; GitHub is the transparency mirror auditors replay.
pub fn spawn_mirror(root: PathBuf) {
    // Flush once at startup: appends from before a restart may still be
    // sitting uncommitted in the tree.
    MIRROR_DIRTY.store(true, Ordering::SeqCst);
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        if !MIRROR_DIRTY.swap(false, Ordering::SeqCst) {
            continue;
        }
        let sh = |cmd: &str| {
            std::process::Command::new("sh").args(["-c", cmd]).current_dir(&root)
                .output()
                .map(|o| (o.status.success(),
                    String::from_utf8_lossy(&o.stderr).trim().to_string()))
                .unwrap_or((false, "spawn failed".into()))
        };
        let seq = [
            // Add each path on its own - one missing directory must not
            // abort staging the log itself.
            "git add registry/data/events.jsonl; \
             for d in lean/Razor/Submissions lean-mathlib/RazorMathlib/Submissions \
                      registry/data/statements; do \
               [ -d \"$d\" ] && git add \"$d\"; done; true",
            "git -c user.name='razor mirror' -c user.email='razor@mempoolsurfer.com' commit -q -m 'registry: append events' || true",
            "git pull --rebase --autostash -q",
            "git push -q",
        ];
        for cmd in seq {
            let (ok, errtxt) = sh(cmd);
            if !ok {
                eprintln!("  {} mirror step failed ({cmd}): {errtxt}", ui::gold("⚠"));
                MIRROR_DIRTY.store(true, Ordering::SeqCst);
                break;
            }
        }
    });
}

// ---------------- base64 (no new dependencies) ----------------

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut idx = [255u8; 256];
    for (i, c) in T.iter().enumerate() {
        idx[*c as usize] = i as u8;
    }
    let s: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &b in &s {
        if b == b'=' {
            break;
        }
        let v = idx[b as usize];
        if v == 255 {
            return None;
        }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

pub fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}
