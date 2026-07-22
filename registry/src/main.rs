//! `razor` - the registry CLI.
//!
//! Every funnel transition is a subcommand appending an event to the log;
//! `verify` and `bench` are the two that do real work (Lean checking, fuel
//! metering) before writing their events. `export` emits the derived state
//! for the site.

mod api;
mod model;
mod ui;
mod verify;

use model::{Entry, Event, State};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    check_flags(cmd, &args);
    let root = repo_root();
    let log_path = root.join("registry/data/events.jsonl");
    std::fs::create_dir_all(log_path.parent().unwrap()).ok();
    // Remote mode: with a remote configured (install.sh sets the public
    // registry as the default), participation commands run against it - the
    // CLI signs locally, the server sequences and verifies. `--local` or
    // RAZOR_REMOTE="" opts out; scripts that build local datasets do so.
    let log_path = remote_setup(cmd, &args, &root, log_path);

    match cmd {
        "propose" => append(&log_path, Event::Propose {
            id: req(&args, "--id"), title: req(&args, "--title"),
            body: opt(&args, "--body").unwrap_or_default(), author: req(&args, "--author"),
        }),
        "formalize" => append(&log_path, Event::Formalize {
            id: req(&args, "--id"), proposal: req(&args, "--proposal"),
            author: req(&args, "--author"), decl: req(&args, "--decl"),
            notes: opt(&args, "--notes").unwrap_or_default(),
            gloss: opt(&args, "--gloss").unwrap_or_default(),
        }),
        "certify" => append(&log_path, Event::Certify {
            statement: req(&args, "--statement"), kind: req(&args, "--kind"),
            decl: req(&args, "--decl"), notes: opt(&args, "--notes").unwrap_or_default(),
        }),
        "converge" => append(&log_path, Event::Converge {
            a: req(&args, "--a"), b: req(&args, "--b"), decl: req(&args, "--decl"),
        }),
        "implies" => append(&log_path, Event::Implies {
            a: req(&args, "--a"), b: req(&args, "--b"), decl: req(&args, "--decl"),
        }),
        "hole" => {
            let lean_type = req(&args, "--lean-type");
            let env = opt(&args, "--env");
            // A pin is permanent, so catch a malformed statement now, not
            // at the first submission: elaborate the type in the hole's
            // environment when the toolchain is available.
            if !has_flag(&args, "--unchecked") {
                precheck_lean_type(&root, env.as_deref(), &lean_type);
            }
            append(&log_path, Event::RegisterHole {
                id: req(&args, "--id"), title: req(&args, "--title"),
                statement: opt(&args, "--statement").unwrap_or_default(),
                lean_type,
                allowed_axioms: multi(&args, "--allow-axiom"),
                proposal: opt(&args, "--proposal"),
                env,
                bridge: None,
                author: opt(&args, "--author"),
            });
        }
        "round" => cmd_round(&log_path, &args),
        "seal-statement" => cmd_seal_statement(&log_path, &args),
        "reveal-statement" => cmd_reveal_statement(&log_path, &args),
        "bridge" => cmd_bridge(&log_path, &args),
        "split" => cmd_split(&log_path, &args),
        "submit" => cmd_submit(&root, &log_path, &args),
        "repin" => cmd_repin(&root, &log_path, &args),
        "propose-batch" => cmd_propose_batch(&log_path, &args),
        "remote" => cmd_remote(&args),
        // The id is positional, but the flag spellings every sibling command
        // uses are accepted too.
        "cite" => cmd_cite(&log_path, &opt(&args, "--submission")
            .or_else(|| opt(&args, "--hole"))
            .or_else(|| args.get(1).filter(|a| !a.starts_with("--")).cloned())
            .unwrap_or_default()),
        // The filer flag is --author like every sibling command; --by is
        // kept as an alias (it is the event's field name on the log).
        "supersede" => append(&log_path, Event::Supersede {
            hole: req(&args, "--hole"),
            by: opt(&args, "--author").or_else(|| opt(&args, "--by")).unwrap_or_else(|| {
                ui::die("missing --author (who files the mark; --by works too)");
            }),
            replacement: req(&args, "--replacement"),
            note: opt(&args, "--note").unwrap_or_default(),
        }),
        "challenge" => append(&log_path, Event::RegisterChallenge {
            id: req(&args, "--id"), title: req(&args, "--title"),
            spec_impl: req(&args, "--spec-impl"), obligation: req(&args, "--obligation"),
        }),
        "anvil-submit" => append(&log_path, Event::AnvilSubmit {
            id: req(&args, "--id"), challenge: req(&args, "--challenge"),
            impl_name: req(&args, "--impl"), solver: req(&args, "--solver"),
            proof_decl: opt(&args, "--proof-decl").unwrap_or_default(),
            refinement_hole: opt(&args, "--refinement-hole"),
        }),
        "curate" => append(&log_path, Event::Curate {
            curator: req(&args, "--curator"), target: req(&args, "--target"),
            note: opt(&args, "--note").unwrap_or_default(),
        }),
        "tag" => {
            let tag = req(&args, "--tag");
            if tag.is_empty() || tag.len() > 32
                || !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                ui::die("tags are lowercase letters, digits, and dashes (up to 32 chars), e.g. test-data");
            }
            append(&log_path, Event::Tag {
                target: req(&args, "--target"), tag,
                by: opt(&args, "--author").or_else(|| opt(&args, "--by")).unwrap_or_else(|| {
                    ui::die("missing --author (who files the tag; --by works too)");
                }),
                note: opt(&args, "--note").unwrap_or_default(),
            });
        }
        // --target is canonical (a bounty can fund a challenge too);
        // --hole is accepted because it is what people type.
        "fund" => append(&log_path, Event::Fund {
            target: opt(&args, "--target").or_else(|| opt(&args, "--hole")).unwrap_or_else(|| {
                ui::die("missing --target (the hole or challenge the bounty attaches to)");
            }),
            amount: req(&args, "--amount").parse().expect("--amount"),
            funder: req(&args, "--funder"),
            arch: opt(&args, "--arch"),
        }),
        "rig" => append(&log_path, Event::RegisterRig {
            id: req(&args, "--id"), owner: req(&args, "--owner"),
            arch: req(&args, "--arch"), tier: req(&args, "--tier"),
            note: opt(&args, "--note").unwrap_or_default(),
            runner: opt(&args, "--runner").unwrap_or_default(),
        }),
        "payout" => append(&log_path, Event::Payout {
            target: req(&args, "--target"), recipient: req(&args, "--recipient"),
            amount: req(&args, "--amount").parse().expect("--amount"),
            reason: opt(&args, "--reason").unwrap_or_default(),
        }),
        "seal" => {
            // Devex helper for solvers: compute the commitment for a private
            // proof file without touching the registry.
            let file = req(&args, "--file");
            let salt = req(&args, "--salt");
            println!("{}", commitment_of(&file, &salt));
        }
        "commit" => append(&log_path, Event::Commit {
            id: req(&args, "--id"), hole: req(&args, "--hole"),
            solver: req(&args, "--solver"), commitment: req(&args, "--commitment"),
        }),
        "reveal" => cmd_reveal(&root, &log_path, &req(&args, "--submission"),
            &req(&args, "--file"), &req(&args, "--salt"), &req(&args, "--decl")),
        "zk-route" => {
            // Attach a zero-knowledge route to an existing hole: run the
            // trusted setup and pin the verifying key, circuit size, and
            // the bridge tying constraint satisfaction to the hole's
            // pinned statement.
            let hole_id = req(&args, "--hole");
            let state = State::fold(load(&log_path));
            if !state.holes.contains_key(&hole_id) {
                ui::die(&format!("unknown hole: {hole_id} - a route attaches to an existing hole"));
            }
            let zk = root.join("target/release/zk-prover");
            let n = opt(&args, "--n").unwrap_or("4".into());
            let setup = run_json(&zk, &["setup", "--n", &n]);
            append(&log_path, Event::ZkRoute {
                id: req(&args, "--id"),
                hole: hole_id,
                vk_path: setup["vk"].as_str().unwrap().into(),
                vk_hash: setup["vk_hash"].as_str().unwrap().into(),
                constraints: setup["constraints"].as_u64().unwrap(),
                bridge_kind: opt(&args, "--bridge-kind").unwrap_or("theorem".into()),
                bridge: req(&args, "--bridge"),
                note: opt(&args, "--note").unwrap_or_default(),
            });
            ui::step(&format!("zk route registered {}",
                ui::dim(&format!("- {} constraints, vk {}…", setup["constraints"],
                    setup["vk_hash"].as_str().unwrap_or("?")))));
        }
        "zk-submit" => append(&log_path, Event::ZkSubmit {
            id: req(&args, "--id"), hole: req(&args, "--hole"),
            route: req(&args, "--route"),
            solver: req(&args, "--solver"), public: req(&args, "--public"),
            proof: req(&args, "--proof"),
        }),
        "zk-verify" => cmd_zk_verify(&root, &log_path, &req(&args, "--submission")),
        "verify" => cmd_verify(&root, &log_path, &req(&args, "--submission")),
        "recheck" => cmd_recheck(&root, &log_path, &req(&args, "--submission")),
        "upstream" => cmd_upstream(&root, &log_path, &args),
        "export-benchmark" => cmd_export_benchmark(&log_path, &args),
        "bench" => cmd_bench(&root, &log_path, &req(&args, "--challenge"),
            opt(&args, "--seed").map(|s| s.parse().expect("--seed")).unwrap_or(0xC0FFEE),
            opt(&args, "--iters").map(|s| s.parse().expect("--iters")).unwrap_or(10_000),
            opt(&args, "--rig")),
        "account" => cmd_account(&root, &log_path, &args),
        "profile" => cmd_profile(&log_path, args.get(1).map(String::as_str).unwrap_or("")),
        "status" => cmd_status(&log_path),
        "log" => {
            for e in load(&log_path) {
                println!("{}", serde_json::to_string(&e).unwrap());
            }
        }
        "corpus" => append(&log_path, Event::RecognizeCorpus {
            id: req(&args, "--id"), name: req(&args, "--name"),
            url: req(&args, "--url"), note: opt(&args, "--note").unwrap_or_default(),
            stats: multi(&args, "--stat").iter().map(|s| {
                let (k, v) = s.split_once('=').unwrap_or((s.as_str(), ""));
                (k.to_string(), v.to_string())
            }).collect(),
            source: req(&args, "--source"), as_of: req(&args, "--as-of"),
        }),
        "verify-log" => cmd_verify_log(&log_path),
        "export" => cmd_export(&log_path, &root.join(opt(&args, "--out").unwrap_or("site/data.json".into())),
            opt(&args, "--dataset")),
        "serve" => cmd_serve(&root, &log_path,
            &opt(&args, "--host").unwrap_or("127.0.0.1".into()),
            opt(&args, "--port").map(|p| p.parse().expect("--port")).unwrap_or(8420)),
        _ => {
            print_help(cmd);
            std::process::exit(if cmd == "help" { 0 } else { 2 });
        }
    }
}

fn print_help(cmd: &str) {
    if cmd != "help" {
        eprintln!("{} unknown command: {cmd}", ui::red("✕"));
    }
    let groups: &[(&str, &[(&str, &str)])] = &[
        ("the funnel", &[
            ("propose", "state a problem in plain language"),
            ("formalize", "file a candidate Lean statement for a proposal"),
            ("certify", "attach a sanity certificate to a statement"),
            ("converge", "prove two statements equivalent (they clump)"),
            ("bridge", "pin the equivalence of two statements as its own hole"),
            ("implies", "prove one statement implies another"),
            ("round", "open a challenge window: sealed readings until a deadline"),
            ("seal-statement", "commit a hash of your statement file - a reading, sealed"),
            ("reveal-statement", "open a statement seal; it enters the funnel with provenance"),
            ("hole", "pin an exact Lean statement as a solvable hole"),
            ("split", "reduce a hole to children plus a glue hole"),
            ("repin", "migrate a hole's wording (needs an equivalence proof)"),
            ("supersede", "mark a hole superseded by a better wording"),
            ("propose-batch", "append proposals from a JSONL file (ingestion)"),
        ]),
        ("solving", &[
            ("submit", "claim a hole with a proof declaration or a .lean file"),
            ("verify", "kernel-check a submission against the pinned type"),
            ("recheck", "independently re-verify a claimed solve (read-only)"),
            ("upstream", "draft a home-library PR from an admitted proof"),
        ]),
        ("value", &[
            ("curate", "a public, attributed pick - weighted by your admitted work"),
            ("tag", "an attributed label on any entity (test-data hides it from the marquee)"),
            ("fund", "put a bounty on one exact statement (caveat emptor)"),
            ("payout", "record a payment from a pool"),
        ]),
        ("private + zero-knowledge", &[
            ("seal", "hash a private proof file (no registry write)"),
            ("commit", "post the hash: priority without exposure"),
            ("reveal", "open a commitment; the registry rebuilds and checks it"),
            ("zk-route", "attach a Groth16 route to a hole (runs trusted setup)"),
            ("zk-submit", "submit a proof of knowledge - the witness stays home"),
            ("zk-verify", "check a zk submission against its route's key"),
        ]),
        ("the anvil", &[
            ("challenge", "open a verified-performance competition"),
            ("anvil-submit", "enter an implementation with its refinement proof"),
            ("bench", "fuel-metered and native leaderboard runs"),
            ("rig", "register hardware you bring to the boards (--runner runs the harness through a command, e.g. a Docker container)"),
        ]),
        ("people", &[
            ("account new", "claim a handle; generates your signing key"),
            ("account list", "everyone with a registered account"),
            ("profile <handle>", "one person's record, from the log alone"),
        ]),
        ("reading + auditing", &[
            ("status", "the whole registry, folded from the log"),
            ("cite", "a citation (BibTeX) for a hole or admitted proof"),
            ("log", "the raw event log, one JSON object per line"),
            ("corpus", "recognize an external verified corpus (e.g. Mathlib)"),
            ("export", "write site/data.json for the explorer"),
            ("export-benchmark", "emit open holes as prover-ready JSONL targets"),
            ("serve", "host the site; data.json re-derived per request"),
            ("verify-log", "audit every event signature against registered keys"),
            ("remote", "set/show the registry commands publish to (--local opts out)"),
        ]),
    ];
    println!();
    println!("  {}  {}", ui::accent("razor"), "the proof frontier, machine-checked");
    println!("  {}", ui::dim("a hole is a Lean statement with a sorry in it; the registry records"));
    println!("  {}", ui::dim("who states, funds, and fills them. admission is a kernel check."));
    println!();
    println!("  {} razor <command> {}", ui::dim("usage:"), ui::dim("[--flag value]..."));
    for (title, cmds) in groups {
        ui::section(title, None);
        for (name, desc) in *cmds {
            println!("  {}  {}", ui::cyan(&format!("{name:<18}")), desc);
        }
    }
    println!();
}

const SIGILS: &[&str] = &["∴", "∮", "∞", "ℵ", "λ", "Σ", "Δ", "Ψ", "Ω", "ξ", "φ", "π", "∂", "≅", "⊕", "∇"];

fn sigil_of(handle: &str) -> &'static str {
    let n: u32 = handle.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    SIGILS[n as usize % SIGILS.len()]
}

fn ask(prompt: &str, default: Option<&str>) -> String {
    use std::io::{BufRead, IsTerminal, Write};
    // Scripted callers (demo.sh, CI) get the default instead of a prompt.
    if !std::io::stdin().is_terminal() {
        return default.unwrap_or("").to_string();
    }
    let hint = default.map(|d| format!(" {}", ui::dim(&format!("[{d}]")))).unwrap_or_default();
    print!("{} {prompt}{hint} ", ui::accent("?"));
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line).ok();
    let line = line.trim().to_string();
    if line.is_empty() { default.unwrap_or("").to_string() } else { line }
}

fn cmd_account(root: &PathBuf, log_path: &PathBuf, args: &[String]) {
    match args.get(1).map(String::as_str) {
        Some("new") => {
            use std::io::IsTerminal;
            let state = State::fold(load(log_path));
            // Without a terminal there is nobody to re-prompt: a bad or
            // missing handle is fatal, not a retry loop.
            let scripted = !std::io::stdin().is_terminal();
            let handle = loop {
                let h = opt(args, "--handle").unwrap_or_else(|| ask("handle (lowercase, dashes ok):", None));
                if h.is_empty() || !h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                    eprintln!("  {} handles are lowercase letters, digits, and dashes", ui::red("✕"));
                    if scripted || opt(args, "--handle").is_some() { std::process::exit(2); }
                } else if state.accounts.contains_key(&h) {
                    eprintln!("  {} '{h}' is taken", ui::red("✕"));
                    if scripted || opt(args, "--handle").is_some() { std::process::exit(2); }
                } else {
                    break h;
                }
            };
            let display = opt(args, "--display").unwrap_or_else(|| ask("display name:", Some(&handle)));
            let about = opt(args, "--about").unwrap_or_else(|| ask("one line about you:", Some("")));
            let github = opt(args, "--github")
                .unwrap_or_else(|| ask("github username (optional, bridges your existing identity):", Some("")));
            let display = if display.is_empty() { handle.clone() } else { display };

            // A real Ed25519 keypair: the signing key stays local, the
            // verifying key goes on the log as the account's pubkey. Every
            // later event by this handle is signed, so a registered handle
            // cannot be impersonated (razor verify-log checks the chain).
            let sk = generate_signing_key();
            let pubkey = hex_of(sk.verifying_key().as_bytes());
            // The key goes to the per-user directory (~/.config/razor/keys
            // unless RAZOR_KEYS_DIR overrides it): it is your identity, so
            // it must survive demo runs and even deleting the clone.
            let keydir = keys_dirs(log_path).into_iter().next().expect("keys dir");
            std::fs::create_dir_all(&keydir).ok();
            let keyfile = keydir.join(format!("{handle}.secret"));
            std::fs::write(&keyfile, hex_of(&sk.to_bytes())).expect("write key");
            let _ = root;

            // A chosen sigil wins; otherwise one is derived from the handle.
            let sigil = match opt(args, "--sigil") {
                Some(s) if s.chars().count() == 1 && !s.chars().next().unwrap().is_control() => s,
                Some(s) => ui::die(&format!("--sigil must be a single character, got {s:?}")),
                None => sigil_of(&handle).to_string(),
            };
            append(log_path, Event::RegisterAccount {
                handle: handle.clone(), display: display.clone(), about,
                sigil: sigil.clone(), pubkey: pubkey.clone(), github: github.clone(),
            });
            let key = keyfile.display().to_string();
            let mut lines = vec![
                (format!("{sigil}  {display} (@{handle})"),
                 format!("{sigil}  {} {}", ui::bold(&display), ui::dim(&format!("(@{handle})")))),
                ("welcome to the frontier.".into(), "welcome to the frontier.".into()),
                (format!("key   {key}"), format!("{}   {}", ui::dim("key"), ui::dim(&key))),
                ("back it up - the key signs everything you do and cannot be regenerated".into(),
                 ui::dim("back it up - the key signs everything you do and cannot be regenerated")),
                (format!("next  razor profile {handle}"),
                 format!("{}  razor profile {handle}", ui::dim("next"))),
            ];
            if !github.is_empty() {
                lines.push((format!("link  github.com/{github}"),
                    format!("{}  github.com/{github}", ui::dim("link"))));
            }
            ui::card(&lines);
            if !github.is_empty() {
                println!("  {}", ui::dim("to make the github link checkable by anyone, publish this line"));
                println!("  {}", ui::dim(&format!("from that account (a public gist, or your profile README):")));
                println!();
                println!("    razor:{pubkey}");
                println!();
            }
        }
        Some("list") => {
            let state = State::fold(load(log_path));
            let w = state.accounts.values().map(|a| a.handle.chars().count()).max().unwrap_or(0);
            for a in state.accounts.values() {
                let gh = if a.github.is_empty() { String::new() }
                    else { format!("  {}", ui::dim(&format!("github.com/{}", a.github))) };
                println!("  {}  {}  {}  {}{gh}",
                    ui::accent(&a.sigil),
                    ui::cyan(&format!("@{:<w$}", a.handle)),
                    ui::bold(&a.display),
                    ui::dim(&a.about));
            }
        }
        _ => ui::die("usage: razor account <new|list> [--handle H --display D --about A]"),
    }
}

fn cmd_profile(log_path: &PathBuf, handle: &str) {
    let mut state = State::fold(load(log_path));
    state.settle_admissions();
    state.aggregate_people();
    let Some(p) = state.people.get(handle) else {
        ui::die(&format!("no activity recorded for '{handle}'"));
    };
    let (sigil, display) = p.account.as_ref()
        .map(|a| (a.sigil.as_str(), a.display.as_str()))
        .unwrap_or(("·", handle));
    println!();
    println!("  {}  {} {}{}",
        ui::accent(sigil), ui::bold(display), ui::dim(&format!("(@{handle})")),
        p.account.as_ref().map(|a| if a.about.is_empty() { String::new() } else { format!("  {}", ui::dim(&a.about)) }).unwrap_or_default());
    if let Some(a) = &p.account {
        if !a.github.is_empty() {
            println!("     {}", ui::dim(&format!("github.com/{}  (checkable: that account publishes razor:{}…)",
                a.github, &a.pubkey[..12.min(a.pubkey.len())])));
        }
    }
    println!();
    let dot = ui::dim("·");
    println!("     {} solved {dot} {} rejected {dot} {} top spots {dot} {} earned {dot} {} funded",
        ui::green(&p.solved.to_string()), ui::red(&p.rejected.to_string()),
        ui::gold(&p.top_spots.to_string()),
        ui::gold(&ui::commas(p.payouts_total)), ui::commas(p.funded_total));
    if !p.submissions.is_empty() {
        println!();
        println!("     {}", ui::dim("submissions"));
        for (seq, id, target, kind, outcome) in &p.submissions {
            let mark = match outcome.as_str() {
                "admitted" => ui::green("✓"), "rejected" => ui::red("✕"),
                "sealed" => ui::gold("⏣"), _ => ui::dim("·"),
            };
            println!("       {mark} {} {}  {}  {target}  {}",
                ui::dim(&format!("#{seq}")), ui::cyan(id), ui::dim("→"), ui::dim(&format!("({kind})")));
        }
    }
    if !p.lanes.is_empty() {
        println!();
        println!("     {}", ui::dim("anvil lanes"));
        for (ch, imp, board, score, unit, leader) in &p.lanes {
            println!("       {} {ch} {}  {}  {} {unit}",
                if *leader { ui::gold("♛") } else { " ".into() },
                ui::bold(imp), ui::dim(&format!("[{board}]")), format!("{score:.2}"));
        }
    }
    if !p.proposals.is_empty() {
        println!("     {}  {}", ui::dim("proposals"), p.proposals.join(", "));
    }
    if !p.open_holes_authored.is_empty() {
        println!("     {}  {}  {}", ui::dim("waiting on"), p.open_holes_authored.join(", "),
            ui::dim("(open holes under their proposals)"));
    }
    println!();
}

fn commitment_of(file: &str, salt: &str) -> String {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(file).unwrap_or_else(|e| {
        ui::die(&format!("cannot read {file}: {e}"));
    });
    let mut h = Sha256::new();
    h.update(&bytes);
    h.update(salt.as_bytes());
    format!("{:x}", h.finalize())
}

fn cmd_reveal(root: &PathBuf, log_path: &PathBuf, submission: &str, file: &str, salt: &str, decl: &str) {
    let state = State::fold(load(log_path));
    let sub = state
        .holes
        .values()
        .flat_map(|h| h.submissions.iter())
        .find(|s| s.id == submission)
        .unwrap_or_else(|| {
            ui::die(&format!("unknown submission: {submission}"));
        });
    let Some(commitment) = &sub.commitment else {
        ui::die(&format!("{submission} is not a private submission (no commitment on record)"));
    };
    let actual = commitment_of(file, salt);
    if &actual != commitment {
        ui::verdict(false, &format!("file+salt hashes to {actual}, committed was {commitment}"));
        std::process::exit(1);
    }
    ui::step(&format!("commitment verified {}", ui::dim(&format!("sha256(file ‖ salt) matches {}…", &commitment[..16]))));

    // Install the revealed file as Razor.Private.<SubmissionId> and build it.
    let modname = submission.replace(|c: char| !c.is_ascii_alphanumeric(), "");
    let module = format!("Razor.Private.S{modname}");
    let dest = root.join(format!("lean/Razor/Private/S{modname}.lean"));
    std::fs::create_dir_all(dest.parent().unwrap()).expect("mkdir Private");
    std::fs::copy(file, &dest).expect("install revealed file");
    ui::step(&format!("installed as {} {}", ui::cyan(&module), ui::dim("- building…")));
    let build = std::process::Command::new("lake")
        .arg("build")
        .current_dir(root.join("lean"))
        .output()
        .expect("lake build");
    if !build.status.success() {
        let _ = std::fs::remove_file(&dest);
        ui::verdict(false, "revealed file does not compile");
        println!("{}", String::from_utf8_lossy(&build.stderr));
        std::process::exit(1);
    }
    append(log_path, Event::Reveal {
        submission: submission.into(),
        decl: decl.into(),
        module,
    });
    ui::step(&format!("revealed {} razor verify --submission {submission}", ui::dim("- next:")));
}

/// Open a challenge window on a proposal: a dated invitation for sealed
/// readings. Nothing is enforced - a late seal or reveal simply carries its
/// own timestamps, and the blindness math reads event order, not the dates.
fn cmd_round(log_path: &PathBuf, args: &[String]) {
    let proposal = req(args, "--proposal");
    let state = State::fold(load(log_path));
    if !state.proposals.contains_key(&proposal) {
        ui::die(&format!("unknown proposal: {proposal}"));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let closes_at: u64 = match (opt(args, "--closes-at"), opt(args, "--days")) {
        (Some(t), _) => t.parse().expect("--closes-at (unix seconds)"),
        (None, Some(d)) => now + d.parse::<u64>().expect("--days") * 86_400,
        (None, None) => ui::die("give --closes-at <unix seconds> or --days <n>"),
    };
    let reveal_by: u64 = match (opt(args, "--reveal-by"), opt(args, "--reveal-days")) {
        (Some(t), _) => t.parse().expect("--reveal-by (unix seconds)"),
        (None, Some(d)) => closes_at + d.parse::<u64>().expect("--reveal-days") * 86_400,
        (None, None) => closes_at + 7 * 86_400,
    };
    if reveal_by < closes_at {
        ui::die("--reveal-by is before --closes-at");
    }
    let id = req(args, "--id");
    append(log_path, Event::OpenRound {
        id: id.clone(), proposal: proposal.clone(), author: req(args, "--author"),
        closes_at, reveal_by, note: opt(args, "--note").unwrap_or_default(),
    });
    ui::step(&format!("challenge window {} open on {}", ui::bold(&id), ui::cyan(&proposal)));
    ui::kv("sealing", &format!("until unix {closes_at} ({})", ui::dim(&days_from(now, closes_at))));
    ui::kv("reveals", &format!("by unix {reveal_by} ({})", ui::dim(&days_from(now, reveal_by))));
    ui::kv("next", &ui::dim("participants: razor seal --file your-statement.lean --salt <salt>, then razor seal-statement"));
}

fn days_from(now: u64, ts: u64) -> String {
    if ts <= now { return "past".into() }
    format!("{} days from now", (ts - now + 86_399) / 86_400)
}

/// File a sealed reading: the commitment (from `razor seal`) goes on the
/// log now; the statement file and salt stay on the author's machine until
/// `razor reveal-statement`.
fn cmd_seal_statement(log_path: &PathBuf, args: &[String]) {
    let proposal = req(args, "--proposal");
    let state = State::fold(load(log_path));
    if !state.proposals.contains_key(&proposal) {
        ui::die(&format!("unknown proposal: {proposal}"));
    }
    let id = req(args, "--id");
    append(log_path, Event::SealStatement {
        id: id.clone(), proposal: proposal.clone(),
        author: req(args, "--author"), commitment: req(args, "--commitment"),
    });
    ui::step(&format!("sealed {} {}", ui::bold(&id),
        ui::dim(&format!("- your reading of {proposal} is timestamped, unseen. Keep the file and salt; next: razor reveal-statement --seal {id}"))));
}

/// Open a statement seal: check the file+salt against the commitment, then
/// file the reading as an ordinary candidate statement carrying its seal's
/// provenance. Statements revealed this way can be *provably* mutually
/// blind: each sealed before the other was revealed.
fn cmd_reveal_statement(log_path: &PathBuf, args: &[String]) {
    let seal_id = req(args, "--seal");
    let file = req(args, "--file");
    let salt = req(args, "--salt");
    let state = State::fold(load(log_path));
    let Some(seal) = state.seals.get(&seal_id) else {
        ui::die(&format!("unknown seal: {seal_id}"));
    };
    if let Some(stm) = &seal.statement {
        ui::die(&format!("{seal_id} is already revealed as {stm}"));
    }
    let actual = commitment_of(&file, &salt);
    if actual != seal.commitment {
        ui::verdict(false, &format!("file+salt hashes to {actual}, committed was {}", seal.commitment));
        std::process::exit(1);
    }
    ui::step(&format!("commitment verified {}",
        ui::dim(&format!("sha256(file ‖ salt) matches {}…", &seal.commitment[..16]))));
    let statement = req(args, "--id");
    let decl = req(args, "--decl");
    // The reveal claims the file defines `decl`; nothing checks that until
    // someone bridges against it, so a typo here is worth a warning now.
    if let Ok(contents) = std::fs::read_to_string(&file) {
        let last = decl.rsplit('.').next().unwrap_or(&decl);
        if !contents.contains(last) {
            eprintln!("  {} {}", ui::gold("⚠"), ui::dim(&format!(
                "{file} does not mention {last} - check --decl; a statement whose declaration \
                 nobody can locate cannot be bridged or converged with")));
        }
    }
    // In remote mode the server re-checks the commitment itself, so the
    // reveal carries the file and salt along.
    if remote().is_some() {
        let bytes = std::fs::read(&file).unwrap_or_else(|e| {
            ui::die(&format!("cannot read {file}: {e}"));
        });
        set_remote_attachments(serde_json::json!({
            "file_b64": api::base64_encode(&bytes),
            "salt": salt,
        }));
    }
    append(log_path, Event::RevealStatement {
        seal: seal_id.clone(),
        statement: statement.clone(),
        author: seal.author.clone(),
        decl,
        gloss: opt(args, "--gloss").unwrap_or_default(),
        notes: opt(args, "--notes").unwrap_or_default(),
    });
    // Keep the revealed file (and its salt, now public) next to the log, so
    // anyone replaying it can re-verify the commitment and read the Lean.
    // In remote mode the server persists its own copy and the mirror
    // delivers it; writing one here too would collide with the next pull.
    if remote().is_none() {
        persist_statement_file(log_path, &statement, &std::fs::read(&file).unwrap_or_default(), &salt);
    }
    ui::step(&format!("revealed {} {}", ui::bold(&statement),
        ui::dim(&format!("- in the funnel with sealed provenance (committed at event {})", seal.seq))));
}

/// Write a revealed statement's file and salt under registry/data/statements:
/// public, mirrored, and sufficient for any third party to re-verify
/// sha256(file ‖ salt) against the sealed commitment on the log.
pub fn persist_statement_file(log_path: &PathBuf, statement: &str, bytes: &[u8], salt: &str) {
    let Some(data_dir) = log_path.parent() else { return };
    let dir = data_dir.join("statements");
    std::fs::create_dir_all(&dir).ok();
    let _ = std::fs::write(dir.join(format!("{statement}.lean")), bytes);
    let _ = std::fs::write(dir.join(format!("{statement}.salt")), salt);
}

/// Pin the equivalence of two candidate statements as its own hole. The
/// pinned type is composed mechanically - `(a's decl) ↔ (b's decl)` - so
/// there is nothing to get subtly wrong, and the proof goes through the
/// ordinary submit/verify path: kernel-checked, attributed, fundable. An
/// admitted proof merges the two statements' clumps.
fn cmd_bridge(log_path: &PathBuf, args: &[String]) {
    let a = req(args, "--a");
    let b = req(args, "--b");
    if a == b {
        ui::die("a statement is trivially equivalent to itself");
    }
    let state = State::fold(load(log_path));
    let (Some(sa), Some(sb)) = (state.statements.get(&a), state.statements.get(&b)) else {
        ui::die("both --a and --b must be filed candidate statements");
    };
    if sa.proposal != sb.proposal {
        ui::die(&format!("{a} and {b} read different proposals - a bridge joins two readings of the same one"));
    }
    // The bridge verifies in one environment, which must define both decls.
    let env_a = state.holes.values().find(|h| h.statement == a).and_then(|h| h.env.clone());
    let env_b = state.holes.values().find(|h| h.statement == b).and_then(|h| h.env.clone());
    let env = opt(args, "--env").or_else(|| match (&env_a, &env_b) {
        (Some(x), Some(y)) if x != y => ui::die(&format!(
            "{a} and {b} verify in different environments ({x} vs {y}); a bridge needs one \
             environment defining both decls - restate one side there, then pass --env")),
        (x, y) => x.clone().or_else(|| y.clone()),
    });
    let id = req(args, "--id");
    let lean_type = format!("({}) ↔ ({})", sa.decl, sb.decl);
    // The composed statement references both decls; they may live in the
    // package already or arrive later in the proof file itself. Say which
    // now, so a typo'd decl is caught before the pin is permanent.
    for (stmt, decl) in [(&a, &sa.decl), (&b, &sb.decl)] {
        if !decl_in_packages(&repo_root(), decl) {
            eprintln!("  {} {}", ui::gold("⚠"), ui::dim(&format!(
                "{stmt}'s declaration {decl} is not defined in the checked-in packages - \
                 the bridge is provable only if the proof file submitted to it defines it")));
        }
    }
    append(log_path, Event::RegisterHole {
        id: id.clone(),
        title: opt(args, "--title")
            .unwrap_or_else(|| format!("bridge: {a} and {b} state the same problem")),
        statement: String::new(),
        lean_type: lean_type.clone(),
        allowed_axioms: multi(args, "--allow-axiom"),
        proposal: Some(sa.proposal.clone()),
        env,
        bridge: Some((a.clone(), b.clone())),
        author: opt(args, "--author").or_else(|| opt(args, "--by")),
    });
    ui::step(&format!("bridge {} registered  {} {} {}",
        ui::bold(&id), ui::cyan(&a), ui::dim("≡?"), ui::cyan(&b)));
    ui::kv("pinned", &lean_type);
    ui::kv("next", &ui::dim(&format!(
        "prove it like any hole: razor submit --hole {id} … - an admitted proof merges the clumps")));
}

/// The verification environment a hole was registered with: the Lean
/// package directory and its root import.
fn env_of(root: &PathBuf, hole: &model::Hole) -> (PathBuf, &'static str) {
    match hole.env.as_deref() {
        Some("mathlib") => (root.join("lean-mathlib"), "RazorMathlib"),
        _ => (root.join("lean"), "Razor"),
    }
}

fn require_env_ready(lean_dir: &PathBuf, root_import: &str) {
    if root_import == "RazorMathlib" && !lean_dir.join(".lake/packages/mathlib").exists() {
        eprintln!("{} this hole verifies in the Mathlib environment, which has not been fetched yet.", ui::red("✕"));
        eprintln!("  {}", ui::dim("run ./mathlib-env.sh once (several GB of prebuilt cache), then retry."));
        std::process::exit(2);
    }
}

/// A hole's pinned statement is permanent, so `razor hole` elaborates the
/// type before appending anything: a typo caught here is a typo that never
/// reaches the log. Skipped with --unchecked, when the environment is not
/// fetched, or when the toolchain is missing (the pin is then taken as
/// written, exactly as before).
fn precheck_lean_type(root: &PathBuf, env: Option<&str>, lean_type: &str) {
    let (lean_dir, root_import) = match env {
        Some("mathlib") => (root.join("lean-mathlib"), "RazorMathlib"),
        _ => (root.join("lean"), "Razor"),
    };
    if root_import == "RazorMathlib" && !lean_dir.join(".lake/packages/mathlib").exists() {
        eprintln!("  {} {}", ui::gold("⚠"), ui::dim(
            "Mathlib environment not fetched - pinning the statement unchecked (./mathlib-env.sh to enable the check)"));
        return;
    }
    let check = format!("import {root_import}\nexample : Prop := ({lean_type})\n");
    let path = lean_dir.join(".razor-pin-check.lean");
    if std::fs::write(&path, check).is_err() { return; }
    let out = std::process::Command::new("lake")
        .args(["env", "lean", ".razor-pin-check.lean"])
        .current_dir(&lean_dir)
        .output();
    let _ = std::fs::remove_file(&path);
    match out {
        Err(_) => eprintln!("  {} {}", ui::gold("⚠"), ui::dim(
            "lake not on PATH - pinning the statement unchecked (open a new shell after install.sh to enable the check)")),
        Ok(o) if !o.status.success() => {
            let all = format!("{}{}", String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr));
            // Errors inside the check file are the statement's own; anything
            // else (unbuilt package, toolchain trouble) is not the user's
            // statement failing, so the pin proceeds unchecked.
            if !all.contains(".razor-pin-check.lean") {
                eprintln!("  {} {}", ui::gold("⚠"), ui::dim(
                    "could not elaborate the pin (package not built?) - pinning the statement unchecked"));
                return;
            }
            eprintln!("{} the pinned statement does not elaborate as written:", ui::red("✕"));
            for l in all.lines().filter(|l| !l.trim().is_empty()) {
                eprintln!("  {}", ui::dim(l));
            }
            eprintln!("  {}", ui::dim(
                "fix the --lean-type (a pin is permanent), or pass --unchecked to pin it anyway"));
            std::process::exit(2);
        }
        Ok(_) => {}
    }
}

/// Whether `decl`'s final segment is defined in a checked-in .lean file of
/// either package. A textual scan used only for warnings - the kernel is
/// the authority at verification time.
fn decl_in_packages(root: &PathBuf, decl: &str) -> bool {
    let last = decl.rsplit('.').next().unwrap_or(decl);
    fn scan(dir: &std::path::Path, last: &str) -> bool {
        let Ok(rd) = std::fs::read_dir(dir) else { return false };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == ".lake") { continue; }
                if scan(&p, last) { return true; }
            } else if p.extension().is_some_and(|e| e == "lean") {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if ["def ", "theorem ", "abbrev ", "inductive ", "structure ", "lemma "]
                        .iter().any(|n| s.contains(&format!("{n}{last}"))) { return true; }
                }
            }
        }
        false
    }
    ["lean/Razor", "lean-mathlib/RazorMathlib"].iter().any(|d| scan(&root.join(d), last))
}

/// Claim a hole. Two forms:
///   razor submit --hole H --solver S --decl Name.Of.Proof
///   razor submit --hole H --solver S --decl Name --file proof.lean
/// With --file, the CLI installs the file into the hole's Lean package as a
/// fresh module and builds it - no package surgery by the solver. The decl
/// must be the fully qualified name of the proof inside that file.
fn cmd_submit(root: &PathBuf, log_path: &PathBuf, args: &[String]) {
    let id = req(args, "--id");
    let hole_id = req(args, "--hole");
    let solver = req(args, "--solver");
    let decl = req(args, "--decl");
    // Remote mode: send the claim (and the proof file, if any) to the
    // registry; it installs, kernel-checks in its sandbox, and answers with
    // the verdict - submit and verify in one call.
    if let Some(url) = remote() {
        let state = State::fold(load(log_path));
        let Some(hole) = state.holes.get(&hole_id) else {
            // The classic trap: a demo-dataset hole exists in the local log
            // but not on the public registry. Say so instead of "unknown".
            let local = State::fold(load(&root.join("registry/data/events.jsonl")));
            if local.holes.contains_key(&hole_id) {
                ui::die(&format!("{hole_id} exists in your local log but not on {url} - \
                    it is demo/local data; re-run with --local to work against your local registry"));
            }
            ui::die(&format!("unknown hole: {hole_id}"));
        };
        let (_, root_import) = env_of(root, hole);
        let (module, file_b64) = match opt(args, "--file") {
            Some(file) => {
                let bytes = std::fs::read(&file).unwrap_or_else(|e| {
                    ui::die(&format!("cannot read {file}: {e}"));
                });
                (Some(submission_module(root_import, &id)), Some(api::base64_encode(&bytes)))
            }
            None => (None, None),
        };
        let event = Event::Submit {
            id: id.clone(), hole: hole_id.clone(), solver, decl, module,
        };
        let sig = sign_event(log_path, &event);
        let mut body = serde_json::json!({ "event": event, "sig": sig });
        if let Some(f) = file_b64 {
            body["file_b64"] = f.into();
        }
        ui::step(&format!("submitting {} {}", ui::bold(&id),
            ui::dim(&format!("to {url} - the kernel check runs there, this can take a minute"))));
        match http_post_json(&format!("{url}/api/submit"), &body) {
            Ok(v) => print_remote_verdict(&v),
            Err(e) => ui::die(&format!("the remote registry refused it: {e}")),
        }
        return;
    }
    let module = opt(args, "--file").map(|file| {
        let state = State::fold(load(log_path));
        let Some(hole) = state.holes.get(&hole_id) else {
            ui::die(&format!("unknown hole: {hole_id}"));
        };
        let (lean_dir, root_import) = env_of(root, hole);
        require_env_ready(&lean_dir, root_import);
        let module = submission_module(root_import, &id);
        let dest = lean_dir.join(module.replace('.', "/") + ".lean");
        std::fs::create_dir_all(dest.parent().unwrap()).expect("mkdir Submissions");
        std::fs::copy(&file, &dest).unwrap_or_else(|e| {
            ui::die(&format!("cannot install {file}: {e}"));
        });
        ui::step(&format!("installed as {} {}", ui::cyan(&module), ui::dim("- building…")));
        let build = std::process::Command::new("lake")
            .arg("build")
            .current_dir(&lean_dir)
            .output()
            .expect("lake build");
        if !build.status.success() {
            let _ = std::fs::remove_file(&dest);
            ui::verdict(false, "submitted file does not compile");
            // Lake reports diagnostics on stdout and only a summary on
            // stderr - show the error blocks from both.
            let all = format!("{}{}", String::from_utf8_lossy(&build.stdout),
                String::from_utf8_lossy(&build.stderr));
            for l in verify::error_lines(&all) {
                println!("  {l}");
            }
            std::process::exit(1);
        }
        module
    });
    append(log_path, Event::Submit {
        id: id.clone(), hole: hole_id.clone(), solver, decl, module,
    });
    // With a remote configured, a bare `razor verify` would go there and
    // not find this local submission - the hint must carry the --local.
    let local = if remote_configured() { "--local " } else { "" };
    ui::step(&format!("submitted {} {}", ui::bold(&id),
        ui::dim(&format!("- next: razor verify {local}--submission {id}"))));
}

/// Migrate a hole's pinned statement to a new wording. The registry only
/// accepts the repin if `--equiv-decl` kernel-checks as a proof of
/// `new ↔ old` in the hole's environment; the old wording, the new wording,
/// and the equivalence stay on the log. This is how a hole survives library
/// churn (Mathlib renames, definition refactors) without losing its
/// history: proofs admitted against the old wording remain valid because
/// the equivalence is itself a checked theorem.
fn cmd_repin(root: &PathBuf, log_path: &PathBuf, args: &[String]) {
    let hole_id = req(args, "--hole");
    let author = req(args, "--author");
    let new_type = req(args, "--lean-type");
    let equiv = req(args, "--equiv-decl");
    let note = opt(args, "--note").unwrap_or_default();
    let state = State::fold(load(log_path));
    let Some(hole) = state.holes.get(&hole_id) else {
        ui::die(&format!("unknown hole: {hole_id}"));
    };
    let old_type = hole.lean_type.clone();
    if old_type == new_type {
        ui::die("the new wording is identical to the current one");
    }
    let (lean_dir, root_import) = env_of(root, hole);
    require_env_ready(&lean_dir, root_import);
    ui::step(&format!("repinning {} {}", ui::cyan(&hole_id), ui::dim("- equivalence must kernel-check")));
    ui::kv("old", &old_type);
    ui::kv("new", &new_type);
    ui::kv("equiv", &equiv);
    let iff_type = format!("({new_type}) ↔ ({old_type})");
    let module = find_decl_module(&lean_dir, root_import, &equiv);
    let v = verify::verify(&lean_dir, root_import, &iff_type, &equiv, &hole.allowed_axioms, module.as_deref());
    ui::verdict(v.admitted, if v.admitted {
        "wordings are provably equivalent; hole repinned"
    } else {
        &v.detail
    });
    if !v.admitted {
        std::process::exit(1);
    }
    append(log_path, Event::Repin {
        hole: hole_id.clone(), author, lean_type: new_type, equiv_decl: equiv, note,
    });
    ui::kv("kept", &ui::dim("old wording + equivalence proof stay on the log; prior admissions remain valid"));
}

/// Append proposals in bulk from a JSONL file: one {"id","title","body"}
/// object per line. Lines whose id is already a proposal are skipped, so
/// re-running an ingestion snapshot is idempotent.
fn cmd_propose_batch(log_path: &PathBuf, args: &[String]) {
    let file = req(args, "--file");
    let author = req(args, "--author");
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| {
        ui::die(&format!("cannot read {file}: {e}"));
    });
    let existing: std::collections::BTreeSet<String> =
        State::fold(load(log_path)).proposals.keys().cloned().collect();
    APPEND_QUIET.store(true, std::sync::atomic::Ordering::Relaxed);
    let (mut added, mut skipped) = (0u64, 0u64);
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() { continue }
        let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
            ui::die(&format!("{file}:{}: bad JSON: {e}", n + 1));
        });
        let id = v["id"].as_str().unwrap_or_else(|| {
            ui::die(&format!("{file}:{}: missing \"id\"", n + 1));
        }).to_string();
        if existing.contains(&id) {
            skipped += 1;
            continue;
        }
        append(log_path, Event::Propose {
            id,
            title: v["title"].as_str().unwrap_or_default().into(),
            body: v["body"].as_str().unwrap_or_default().into(),
            author: author.clone(),
        });
        added += 1;
    }
    ui::step(&format!("{} proposals appended{}", ui::bold(&added.to_string()),
        if skipped > 0 { ui::dim(&format!(" ({skipped} already on the log, skipped)")) } else { String::new() }));
}

fn year_of_ts(ts: u64) -> i64 {
    // Days-to-civil-year, Gregorian (Howard Hinnant's algorithm, year part).
    let z = (ts / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    if mp >= 10 { y + 1 } else { y }
}

/// sha256 (hex) of the raw log up to and including event `seq` - a content
/// hash a citation can pin, checkable by anyone holding the log.
fn log_hash_through(log_path: &PathBuf, seq: u64) -> String {
    use sha2::{Digest, Sha256};
    let text = std::fs::read_to_string(log_path).unwrap_or_default();
    let mut h = Sha256::new();
    for line in text.lines().take(seq as usize + 1) {
        h.update(line.as_bytes());
        h.update(b"\n");
    }
    format!("{:x}", h.finalize())
}

/// Emit a BibTeX citation for a hole or a submission. The citation pins the
/// event's sequence number and a hash of the log up to it, so the cited
/// fact is checkable: anyone with the log can recompute the hash and re-run
/// the verification.
fn cmd_cite(log_path: &PathBuf, id: &str) {
    if id.is_empty() {
        ui::die("usage: razor cite <hole-or-submission-id>");
    }
    let mut state = State::fold(load(log_path));
    state.aggregate_people();
    let display_of = |handle: &str| -> String {
        state.accounts.get(handle)
            .map(|a| format!("{} (@{})", a.display, a.handle))
            .unwrap_or_else(|| handle.to_string())
    };
    // A submission: cite the proof.
    let as_sub = state.holes.values()
        .find_map(|h| h.submissions.iter().find(|s| s.id == id).map(|s| (h, s)));
    let (key, title, author, seq, note) = if let Some((hole, sub)) = as_sub {
        let seq = state.events.iter().find(|e| matches!(&e.event,
            Event::Submit { id: sid, .. } | Event::Commit { id: sid, .. } if sid == id))
            .map(|e| e.seq)
            .unwrap_or_else(|| ui::die(&format!("no log event for submission {id}")));
        let status = match &sub.verdict {
            Some((true, ..)) => "kernel-checked and admitted".to_string(),
            Some((false, ..)) => "rejected by the verifier".to_string(),
            None => "not yet verified".to_string(),
        };
        (format!("razor-{id}"),
         format!("A machine-checked proof of {}: {}", hole.id, hole.title),
         display_of(&sub.solver),
         seq,
         format!("{status}; pinned statement: {}", hole.lean_type))
    } else if let Some(hole) = state.holes.get(id) {
        let seq = state.events.iter().find(|e| matches!(&e.event,
            Event::RegisterHole { id: hid, .. } if hid == id))
            .map(|e| e.seq)
            .unwrap_or(0);
        // Whoever pinned the hole is its author; the proposal's author is
        // the fallback for holes from before the field existed.
        let author = hole.registered_by.as_ref().map(|h| display_of(h))
            .or_else(|| hole.proposal.as_ref()
                .and_then(|p| state.proposals.get(p))
                .map(|p| display_of(&p.author)))
            .unwrap_or_else(|| "the registry".into());
        let status = match hole.status.as_str() {
            "solved" => format!("solved (submission {})", hole.solved_by.clone().unwrap_or_default()),
            s => s.to_string(),
        };
        (format!("razor-{id}"),
         format!("{}: {}", hole.id, hole.title),
         author, seq,
         format!("{status}; pinned Lean statement: {}", hole.lean_type))
    } else {
        ui::die(&format!("unknown hole or submission: {id}"));
    };
    let ts = state.events.get(seq as usize).map(|e| e.ts).unwrap_or(0);
    let hash = log_hash_through(log_path, seq);
    println!("@misc{{{key},");
    println!("  title        = {{{title}}},");
    println!("  author       = {{{author}}},");
    println!("  year         = {{{}}},", year_of_ts(ts));
    println!("  howpublished = {{Satoshi's Razor registry, event {seq}}},");
    println!("  note         = {{{note}. Log sha256 through event {seq}: {hash}.");
    println!("                  Recheck with `razor verify-log` and `razor verify`.}}");
    println!("}}");
}

fn cmd_zk_verify(root: &PathBuf, log_path: &PathBuf, submission: &str) {
    let state = State::fold(load(log_path));
    // The full proof lives on the log; state keeps only a prefix.
    let (hole_id, route_id, solver, public, proof) = state
        .events
        .iter()
        .find_map(|e| match &e.event {
            Event::ZkSubmit { id, hole, route, solver, public, proof } if id == submission => {
                Some((hole.clone(), route.clone(), solver.clone(), public.clone(), proof.clone()))
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            ui::die(&format!("unknown zk submission: {submission}"));
        });
    let hole = state.holes.get(&hole_id).expect("hole for submission");
    let route = hole.zk_routes.iter().find(|r| r.id == route_id).unwrap_or_else(|| {
        ui::die(&format!("hole {hole_id} has no zk route {route_id}"));
    });
    ui::step(&format!("verifying {} {} {}", ui::bold(submission), ui::dim("against"), ui::cyan(&hole_id)));
    ui::kv("vk", &format!("{}…  {}", &route.vk_hash[..16], ui::dim(&format!("({} constraints)", route.constraints))));
    ui::kv("bridge", &format!("{} {}", ui::dim(&format!("[{}]", route.bridge_kind)), route.bridge));
    let zk = root.join("target/release/zk-prover");
    let out = std::process::Command::new(&zk)
        .args(["verify", "--vk", root.join(&route.vk_path).to_str().unwrap(), "--proof", &proof, "--public", &public])
        .output()
        .expect("run zk-prover");
    let admitted = out.status.success();
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // zk-prover speaks JSON; keep the human-readable reason.
    let detail = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v["reason"].as_str().map(String::from))
        .unwrap_or(raw);
    ui::verdict(admitted, if admitted { "the witness was never seen" } else { &detail });
    let pool = hole.pool;
    let already_solved = hole.status == "solved";
    append(log_path, Event::Verdict {
        submission: submission.into(),
        admitted,
        axioms: vec![],
        detail,
        cost_ms: 0,
    });
    if admitted && !already_solved && pool > 0 {
        append(log_path, Event::Payout {
            target: hole_id.clone(),
            recipient: solver,
            amount: pool,
            reason: format!("first admitted zk solution of {hole_id}"),
        });
    }
}

pub struct VerifyOutcome {
    pub admitted: bool,
    pub axioms: Vec<String>,
    pub detail: String,
    pub cost_ms: u64,
    pub pinned: String,
    pub payout: u64,
}

/// The verification core, shared by the CLI and the serve API: kernel-check
/// a revealed submission against its hole's pinned statement, record the
/// verdict (and the payout, when a bounty is taken) on the log. Appends run
/// under `log_lock`; the kernel check itself does not hold it.
fn verify_and_record(
    root: &PathBuf,
    log_path: &PathBuf,
    submission: &str,
    log_lock: &std::sync::Mutex<()>,
) -> Result<VerifyOutcome, String> {
    let state = State::fold(load(log_path));
    let Some((hole, sub)) = state
        .holes
        .values()
        .find_map(|h| h.submissions.iter().find(|s| s.id == submission).map(|s| (h, s)))
    else {
        return Err(format!("unknown submission: {submission}"));
    };
    if !sub.revealed {
        return Err(format!("{} is committed but not yet revealed - nothing to verify", sub.id));
    }
    ui::step(&format!("verifying {} {} {}", ui::bold(&sub.id), ui::dim("against"), ui::cyan(&hole.id)));
    ui::kv("claims", &sub.decl);
    ui::kv("pinned", &hole.lean_type);
    // Pick the verification environment the hole was registered with.
    let (lean_dir, root_import) = env_of(root, hole);
    if root_import == "RazorMathlib" && !lean_dir.join(".lake/packages/mathlib").exists() {
        return Err("this hole verifies in the Mathlib environment, which has not been fetched yet - \
            run ./mathlib-env.sh once (several GB of prebuilt cache), then retry".into());
    }
    let module = sub.module.clone()
        .or_else(|| find_decl_module(&lean_dir, root_import, &sub.decl));
    let t0 = std::time::Instant::now();
    let v = verify::verify(&lean_dir, root_import, &hole.lean_type, &sub.decl, &hole.allowed_axioms, module.as_deref());
    let cost_ms = t0.elapsed().as_millis() as u64;
    // Infrastructure failures (no toolchain, no container runtime) are not
    // verdicts; nothing is recorded and the caller sees why.
    if let Some(msg) = v.detail.strip_prefix("checker-unavailable: ") {
        return Err(msg.to_string());
    }
    ui::kv("axioms", &if v.axioms.is_empty() { ui::dim("none") } else { v.axioms.join(", ") });
    ui::kv("kernel", &format!("{cost_ms} ms"));
    ui::verdict(v.admitted, if v.admitted { "" } else { &v.detail });
    let recipient = sub.solver.clone();
    let hole_id = hole.id.clone();
    let pool = hole.pool;
    let already_solved = hole.status == "solved";
    let pinned = hole.lean_type.clone();
    let outcome = VerifyOutcome {
        admitted: v.admitted,
        axioms: v.axioms.clone(),
        detail: v.detail.clone(),
        cost_ms,
        pinned,
        payout: if v.admitted && !already_solved { pool } else { 0 },
    };
    let _guard = log_lock.lock().unwrap();
    append_entry(log_path, Event::Verdict {
        submission: submission.into(),
        admitted: v.admitted,
        axioms: v.axioms,
        detail: v.detail,
        cost_ms,
    }, None);
    // A bounty pays for the literal statement, first admitted proof, no
    // adjudication - the funder took the fidelity risk when they funded it.
    if outcome.payout > 0 {
        append_entry(log_path, Event::Payout {
            target: hole_id.clone(),
            recipient,
            amount: pool,
            reason: format!("first admitted proof of {hole_id}, exactly as pinned"),
        }, None);
    }
    Ok(outcome)
}

fn print_remote_verdict(v: &serde_json::Value) {
    if let Some(p) = v["pinned"].as_str() {
        ui::kv("pinned", p);
    }
    let axioms: Vec<String> = v["axioms"].as_array().map(|a| a.iter()
        .filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    ui::kv("axioms", &if axioms.is_empty() { ui::dim("none") } else { axioms.join(", ") });
    if let Some(ms) = v["cost_ms"].as_u64() {
        ui::kv("kernel", &format!("{ms} ms (on the remote)"));
    }
    let admitted = v["admitted"].as_bool() == Some(true);
    ui::verdict(admitted, if admitted { "" } else { v["detail"].as_str().unwrap_or("") });
    if let Some(p) = v["payout"].as_u64().filter(|p| *p > 0) {
        ui::step(&format!("bounty paid: {}", ui::gold(&ui::commas(p))));
    }
    if !admitted {
        std::process::exit(1);
    }
}

fn cmd_verify(root: &PathBuf, log_path: &PathBuf, submission: &str) {
    // Remote mode: the server runs the check in its sandbox and answers
    // with the verdict it recorded.
    if let Some(url) = remote() {
        ui::step(&format!("verifying {} {}", ui::bold(submission),
            ui::dim(&format!("on {url} - the kernel check runs there"))));
        match http_post_json(&format!("{url}/api/verify"),
            &serde_json::json!({ "submission": submission })) {
            Ok(v) => print_remote_verdict(&v),
            Err(e) => {
                // The classic trap, verify edition: the submission is on the
                // local log (a --local submit), not on the remote.
                if e.contains("unknown submission") {
                    let local = State::fold(load(&root.join("registry/data/events.jsonl")));
                    if local.holes.values().any(|h| h.submissions.iter().any(|s| s.id == submission)) {
                        ui::die(&format!("{submission} exists on your local log but not on {url} - \
                            it was submitted with --local; re-run: razor verify --local --submission {submission}"));
                    }
                }
                ui::die(&format!("the remote registry refused it: {e}"))
            }
        }
        return;
    }
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match verify_and_record(root, log_path, submission, &LOCK) {
        Ok(outcome) => {
            // The rejection is recorded either way; the exit code lets
            // scripts and CI read the verdict without parsing output.
            if !outcome.admitted {
                std::process::exit(1);
            }
        }
        Err(m) => ui::die(&m),
    }
}

/// Signature status of the event at `seq`: Some(true) means a valid
/// signature from the actor's registered key, Some(false) means missing or
/// invalid, None means the actor never registered (open participation).
fn event_sig_status(entries: &[Entry], seq: u64) -> Option<bool> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let mut keys: std::collections::BTreeMap<String, VerifyingKey> = Default::default();
    for e in entries {
        if let Event::RegisterAccount { handle, pubkey, .. } = &e.event {
            if let Some(vk) = bytes_of_hex(pubkey)
                .and_then(|b| <[u8; 32]>::try_from(b).ok())
                .and_then(|b| VerifyingKey::from_bytes(&b).ok())
            {
                keys.insert(handle.clone(), vk);
            }
        }
        if e.seq == seq {
            let actor = e.event.actor()?;
            let vk = keys.get(actor)?;
            let msg = serde_json::to_string(&e.event).unwrap();
            return Some(e.sig.as_deref()
                .and_then(bytes_of_hex)
                .and_then(|b| <[u8; 64]>::try_from(b).ok())
                .map(|b| vk.verify(msg.as_bytes(), &Signature::from_bytes(&b)).is_ok())
                .unwrap_or(false));
        }
    }
    None
}

/// Independently re-verify a claimed solve, writing nothing. This is the
/// command every citation points at: it replays the kernel check against
/// the pinned statement, audits the signature on the claim, pins the log
/// hash, and compares the fresh result with the verdict on the log. A
/// "machine X solved open problem Y" claim reduces to running this and
/// reading one line.
fn cmd_recheck(root: &PathBuf, log_path: &PathBuf, submission: &str) {
    let state = State::fold(load(log_path));
    let (hole, sub) = state
        .holes
        .values()
        .find_map(|h| h.submissions.iter().find(|s| s.id == submission).map(|s| (h, s)))
        .unwrap_or_else(|| {
            ui::die(&format!("unknown submission: {submission}"));
        });
    if !sub.revealed {
        ui::die(&format!("{} is committed but not yet revealed - nothing to recheck", sub.id));
    }
    let Some((recorded, ..)) = &sub.verdict else {
        ui::die(&format!("{submission} has no verdict on the log yet - run razor verify first"));
    };
    let vseq = state.events.iter().rev()
        .find(|e| matches!(&e.event, Event::Verdict { submission: s, .. } if s == submission))
        .map(|e| e.seq)
        .unwrap_or_else(|| ui::die(&format!("no verdict event for {submission}")));
    let claim_seq = state.events.iter()
        .find(|e| matches!(&e.event,
            Event::Submit { id, .. } | Event::Commit { id, .. } if id == submission))
        .map(|e| e.seq)
        .unwrap_or_else(|| ui::die(&format!("no claim event for {submission}")));
    ui::step(&format!("rechecking {} {} {} {}", ui::bold(&sub.id), ui::dim("against"),
        ui::cyan(&hole.id), ui::dim("(read-only - the log is not touched)")));
    ui::kv("claims", &sub.decl);
    ui::kv("pinned", &hole.lean_type);
    ui::kv("recorded", &format!("{} at event {vseq}",
        if *recorded { ui::green("admitted") } else { ui::red("rejected") }));
    ui::kv("log hash", &format!("{} {}", log_hash_through(log_path, vseq),
        ui::dim(&format!("(sha256 through event {vseq} - compare with the citation)"))));
    ui::kv("signature", &match event_sig_status(&state.events, claim_seq) {
        Some(true) => format!("{} {}", ui::green("valid"),
            ui::dim(&format!("- Ed25519 by @{}, event {claim_seq}", sub.solver))),
        Some(false) => format!("{} {}", ui::red("INVALID"),
            ui::dim("- the claim is not signed by the solver's registered key")),
        None => ui::dim(&if state.accounts.contains_key(&sub.solver) {
            format!("none - the claim predates @{}'s account registration (unsigned)", sub.solver)
        } else {
            format!("none - @{} has no registered account (open participation)", sub.solver)
        }),
    });
    let (lean_dir, root_import) = env_of(root, hole);
    require_env_ready(&lean_dir, root_import);
    let module = sub.module.clone()
        .or_else(|| find_decl_module(&lean_dir, root_import, &sub.decl));
    // A file-submission must be present as its module before the check can
    // build it; a checkout older than the submission does not have it yet.
    // Fetch it from the remote - the server hands out exactly the file its
    // verifier installed - rather than failing with a misleading verdict.
    if let Some(m) = &module {
        let dest = lean_dir.join(m.replace('.', "/") + ".lean");
        if !dest.exists() {
            let fetched = remote()
                .and_then(|url| http_get(&format!("{url}/api/submission?id={submission}")).ok())
                .and_then(|body| serde_json::from_str::<serde_json::Value>(&body).ok())
                .and_then(|v| v["file_b64"].as_str().and_then(api::base64_decode));
            match fetched {
                Some(bytes) => {
                    std::fs::create_dir_all(dest.parent().unwrap()).ok();
                    std::fs::write(&dest, &bytes).unwrap_or_else(|e| {
                        ui::die(&format!("cannot install {}: {e}", dest.display()));
                    });
                    ui::step(&format!("fetched the submission file {}",
                        ui::dim(&format!("- {m} is newer than this checkout"))));
                }
                None => ui::die(&format!(
                    "this checkout does not contain the submission's module {m} - the submission \
                     is newer than your checkout; git pull (the mirror carries the file), then retry")),
            }
        }
    }
    let t0 = std::time::Instant::now();
    let v = verify::verify(&lean_dir, root_import, &hole.lean_type, &sub.decl,
        &hole.allowed_axioms, module.as_deref());
    ui::kv("axioms", &if v.axioms.is_empty() { ui::dim("none") } else { v.axioms.join(", ") });
    ui::kv("kernel", &format!("{} ms", t0.elapsed().as_millis()));
    ui::verdict(v.admitted, if v.admitted { "" } else { &v.detail });
    if v.admitted == *recorded {
        ui::step(&format!("recheck {} the recorded verdict", ui::green("agrees with")));
    } else {
        eprintln!("  {} recheck {} the recorded verdict - your toolchain or checkout may differ \
            from the verifier's (git pull and retry); if the statement itself migrated, see razor repin",
            ui::red("✕"), ui::red("DISAGREES with"));
        std::process::exit(1);
    }
}

/// Carry an admitted proof to its home library. Without --pr this drafts
/// the contribution: a self-contained .lean file holding the proof source
/// under a provenance header that pins the registry facts (submission,
/// verdict event, log hash), ready to adapt into a Mathlib pull request.
/// With --pr it records where the proof landed; the hole then shows as
/// upstreamed everywhere. The registry measures itself by upstreamed
/// proofs, not admitted ones: a proof is only useful where people build on
/// it.
fn cmd_upstream(root: &PathBuf, log_path: &PathBuf, args: &[String]) {
    let hole_id = req(args, "--hole");
    let state = State::fold(load(log_path));
    let Some(hole) = state.holes.get(&hole_id) else {
        ui::die(&format!("unknown hole: {hole_id}"));
    };
    if let Some(pr) = opt(args, "--pr") {
        append(log_path, Event::Upstream {
            hole: hole_id.clone(), by: req(args, "--by"), pr_url: pr.clone(),
            note: opt(args, "--note").unwrap_or_default(),
        });
        ui::step(&format!("recorded: {} upstreamed {}", ui::cyan(&hole_id), ui::dim(&format!("- {pr}"))));
        return;
    }
    if hole.status != "solved" {
        ui::die(&format!("{hole_id} is still open - only an admitted proof can be upstreamed"));
    }
    let solved_by = hole.solved_by.clone().unwrap_or_default();
    let Some(sub) = hole.submissions.iter().find(|s| s.id == solved_by) else {
        ui::die(&format!("{hole_id} was solved by a zero-knowledge submission - there is no proof text to upstream"));
    };
    let vseq = state.events.iter().rev()
        .find(|e| matches!(&e.event, Event::Verdict { submission: s, .. } if *s == sub.id))
        .map(|e| e.seq).unwrap_or(0);
    let hash = log_hash_through(log_path, vseq);
    let solver = state.accounts.get(&sub.solver)
        .map(|a| format!("{} (@{})", a.display, a.handle))
        .unwrap_or_else(|| sub.solver.clone());
    let (lean_dir, _) = env_of(root, hole);
    // The proof source: the installed module file if the proof arrived as
    // one, otherwise the declaration's source from the package index.
    let source = sub.module.as_ref()
        .and_then(|m| std::fs::read_to_string(lean_dir.join(m.replace('.', "/") + ".lean")).ok())
        .or_else(|| lean_decl_index(root).get(&sub.decl).map(|(src, ns)| {
            let mut s = String::new();
            if !ns.is_empty() { s.push_str(&format!("namespace {ns}\n\n")); }
            s.push_str(src);
            if !ns.is_empty() { s.push_str(&format!("\n\nend {ns}")); }
            s
        }));
    let target = if hole.env.as_deref() == Some("mathlib") { "Mathlib" } else { "its home library" };
    let mut text = format!(
        "/-\n{}: {}\n\nProved by {}, admitted by kernel check in the Satoshi's Razor registry.\n\
         Provenance: submission {}, verdict event {}, log sha256 through that\n\
         event: {}.\n\
         Independent recheck: razor recheck --submission {}\n\
         Pinned statement: {}\n-/\n\n",
        hole.id, hole.title, solver, sub.id, vseq, hash, sub.id, hole.lean_type);
    match source {
        Some(src) => text.push_str(&src),
        None => text.push_str(&format!(
            "-- The proof lives at {} in the registry's Lean package; inline it here.\n\
             theorem {} : {} := {}\n",
            sub.decl,
            hole.id.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "_"),
            hole.lean_type, sub.decl)),
    }
    if !text.ends_with('\n') { text.push('\n'); }
    let out = opt(args, "--out").unwrap_or_else(|| format!("upstream/{hole_id}.lean"));
    let dest = root.join(&out);
    std::fs::create_dir_all(dest.parent().unwrap()).ok();
    std::fs::write(&dest, &text).unwrap_or_else(|e| ui::die(&format!("cannot write {out}: {e}")));
    ui::step(&format!("drafted {} {}", ui::bold(&out), ui::dim(&format!("- a contribution draft for {target}, with registry provenance"))));
    ui::kv("next", &ui::dim(&format!("adapt naming/placement to {target}'s conventions and open the PR")));
    ui::kv("then", &ui::dim(&format!("razor upstream --hole {hole_id} --pr <url> --by <you>  (records it; the hole shows as upstreamed)")));
}

/// Emit the frontier as machine-consumable proving targets: one JSON
/// object per open hole, in the shape prover benchmarks (miniF2F and its
/// descendants) already consume - a header, a formal statement ending in
/// sorry, and the informal text it came from, plus the hole's recorded
/// fidelity facts. This is how the frontier flows to where the provers
/// are; a claimed solve comes back through razor submit / verify /
/// recheck.
fn cmd_export_benchmark(log_path: &PathBuf, args: &[String]) {
    let mut state = State::fold(load(log_path));
    state.aggregate_clumps();
    state.aggregate_fidelity();
    let all = args.iter().any(|a| a == "--all");
    let index = lean_decl_index(&repo_root());
    let mut lines: Vec<String> = vec![];
    for h in state.holes.values() {
        if !all && h.status != "open" { continue; }
        // The header a consumer needs: when the pinned type only uses names
        // the underlying library itself defines (e.g. Mathlib's own
        // FermatLastTheorem), the library import suffices; when it uses
        // names defined in this repo's Lean packages, the package import is
        // required and the statement is only checkable against a checkout.
        let local = lean_idents(&h.lean_type).iter().any(|w| index.contains_key(w));
        let (header, env) = match (h.env.as_deref(), local) {
            (Some("mathlib"), false) => ("import Mathlib", "mathlib"),
            (Some("mathlib"), true) => ("import RazorMathlib", "mathlib"),
            (_, _) => ("import Razor", "core"),
        };
        let name: String = h.id.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let informal = h.proposal.as_ref()
            .and_then(|p| state.proposals.get(p))
            .map(|p| if p.body.is_empty() { p.title.clone() } else { format!("{}. {}", p.title, p.body) })
            .unwrap_or_else(|| h.title.clone());
        let obj = serde_json::json!({
            "id": h.id,
            "title": h.title,
            "env": env,
            "header": header,
            "formal_statement": format!("theorem razor_{name} : {} := by sorry", h.lean_type),
            "lean_type": h.lean_type,
            "allowed_axioms": h.allowed_axioms,
            "informal": informal,
            "status": h.status,
            "fidelity": serde_json::to_value(&h.fidelity).unwrap(),
        });
        lines.push(obj.to_string());
    }
    let text = lines.join("\n") + "\n";
    match opt(args, "--out") {
        Some(out) => {
            let dest = repo_root().join(&out);
            std::fs::create_dir_all(dest.parent().unwrap()).ok();
            std::fs::write(&dest, &text).unwrap_or_else(|e| ui::die(&format!("cannot write {out}: {e}")));
            ui::step(&format!("exported {} proving targets {} {}",
                ui::bold(&lines.len().to_string()), ui::dim("→"), out));
        }
        None => print!("{text}"),
    }
}

fn cmd_bench(root: &PathBuf, log_path: &PathBuf, challenge_id: &str, seed: u64, iters: u64, rig_id: Option<String>) {
    let mut state = State::fold(load(log_path));
    state.settle_admissions();
    // With --rig, run only that rig's tier and stamp its arch and id on the
    // scores. The rig owner runs this on the hardware they brought.
    let rig = rig_id.as_deref().map(|r| {
        state.rigs.get(r).cloned().unwrap_or_else(|| {
            ui::die(&format!("unknown rig: {r} (register with `razor rig`)"));
        })
    });
    let ch = state.challenges.get(challenge_id).unwrap_or_else(|| {
        ui::die(&format!("unknown challenge: {challenge_id}"));
    });
    if remote().is_some() && rig.is_none() {
        ui::die("publishing scores to a remote registry requires --rig: register the machine \
            with `razor rig`, then bench through it, so every public score names the hardware \
            it was measured on and carries the rig owner's signature");
    }
    let harness = root.join("target/release/anvil-harness");
    for entry in ch.entries.iter().filter(|e| e.admitted) {
        let wasm = root.join(format!(
            "target/wasm32-unknown-unknown/release/{}.wasm",
            entry.impl_name.replace('-', "_")
        ));
        // Differential certificate first: an impl that disagrees with the
        // executable spec never gets a score (belt and braces on top of the proof).
        let check = run_json(&harness, &["check", "--impl", &entry.impl_name, "--seed", &seed.to_string(), "--iters", &iters.to_string()]);
        if let Some(why) = check.get("skip").and_then(|v| v.as_str()) {
            // The lane cannot run in this environment at all (a GPU lane on
            // a machine with no GPU): not a failure, just not measurable here.
            eprintln!("  {} {} not measurable on this machine: {why}", ui::gold("⚠"), entry.impl_name);
            continue;
        }
        if check.get("pass") != Some(&serde_json::Value::Bool(true)) {
            eprintln!("{} differential check FAILED for {}, skipping", ui::red("✕"), entry.impl_name);
            continue;
        }
        // A lane with no wasm build (GPU lanes are native-only) simply has
        // no wasm-fuel score; its native scores stand on their own.
        let run_tier1 = rig.as_ref().is_none_or(|r| r.tier == "wasm-fuel") && wasm.exists();
        let run_native = rig.as_ref().is_none_or(|r| r.tier == "native");
        if rig.as_ref().is_none_or(|r| r.tier == "wasm-fuel") && !wasm.exists() {
            eprintln!("  {} {} has no wasm build - skipping the wasm-fuel board", ui::dim("·"), entry.impl_name);
        }
        if run_tier1 {
            let t1 = run_json(&harness, &["tier1", "--wasm", wasm.to_str().unwrap(), "--seed", &seed.to_string(), "--iters", &iters.to_string()]);
            append(log_path, Event::Bench {
                submission: entry.id.clone(),
                tier: "wasm-fuel".into(),
                arch: "wasm32".into(),
                score: t1["fuel_per_op"].as_f64().unwrap(),
                unit: "fuel/op".into(),
                checksum: t1["checksum"].as_u64().unwrap(),
                rig: rig.as_ref().map(|r| r.id.clone()),
            });
            ui::step(&format!("{}  {} {}", ui::bold(&entry.impl_name),
                t1["fuel_per_op"], ui::dim("fuel/op")));
        }
        if run_native {
            // A rig with a runner executes the harness through that command
            // (e.g. inside a Docker container), so the measurement really
            // happens in the rig's environment, not on this host.
            let native_args = ["native", "--impl", &entry.impl_name, "--seed", &seed.to_string(), "--iters", &iters.to_string()];
            let tn = match rig.as_ref().filter(|r| !r.runner.is_empty()) {
                Some(r) => run_json_via(&r.runner, &native_args),
                None => run_json(&harness, &native_args),
            };
            // The rig's environment may lack hardware this lane needs even
            // when this host has it (a GPU lane timed on a GPU-less rig).
            if let Some(why) = tn.get("skip").and_then(|v| v.as_str()) {
                eprintln!("  {} {} not measurable on this rig: {why}", ui::gold("⚠"), entry.impl_name);
                continue;
            }
            append(log_path, Event::Bench {
                submission: entry.id.clone(),
                tier: "native".into(),
                arch: rig.as_ref().map(|r| r.arch.clone())
                    .unwrap_or_else(|| tn["arch"].as_str().unwrap().into()),
                score: tn["ns_per_op"].as_f64().unwrap(),
                unit: "ns/op".into(),
                checksum: tn["checksum"].as_u64().unwrap(),
                rig: rig.as_ref().map(|r| r.id.clone()),
            });
            ui::step(&format!("{}  {} {}", ui::bold(&entry.impl_name),
                tn["ns_per_op"], ui::dim("ns/op native")));
        }
    }
    print_leaderboards(&State::fold(load(log_path)), Some(challenge_id));
}

/// Register a split: one named way of reducing a parent hole to child
/// holes. The children must already be registered holes; the glue hole is
/// created here, with its statement composed mechanically from the pinned
/// types - `(child 1) → ... → (child n) → parent` - so there is nothing
/// for the splitter to get subtly wrong. Proving the glue (through the
/// ordinary submit/verify path) is what makes the split load-bearing.
fn cmd_split(log_path: &PathBuf, args: &[String]) {
    let id = req(args, "--id");
    let parent_id = req(args, "--parent");
    let author = req(args, "--author");
    let children = multi(args, "--child");
    let note = opt(args, "--note").unwrap_or_default();
    if children.is_empty() {
        ui::die("a split needs at least one --child");
    }
    let state = State::fold(load(log_path));
    let Some(parent) = state.holes.get(&parent_id) else {
        ui::die(&format!("unknown parent hole: {parent_id}"));
    };
    let mut glue_type = String::new();
    for c in &children {
        let Some(ch) = state.holes.get(c) else {
            ui::die(&format!("unknown child hole: {c} (register it with `razor hole` first)"));
        };
        if ch.env != parent.env {
            ui::die(&format!("child {c} verifies in a different environment than {parent_id}; a split cannot cross environments"));
        }
        glue_type.push_str(&format!("({}) → ", ch.lean_type));
    }
    glue_type.push_str(&parent.lean_type);
    let glue_id = format!("{id}-glue");
    append(log_path, Event::RegisterHole {
        id: glue_id.clone(),
        title: format!("glue of split {id}: children jointly imply {parent_id}"),
        statement: String::new(),
        lean_type: glue_type.clone(),
        allowed_axioms: parent.allowed_axioms.clone(),
        proposal: parent.proposal.clone(),
        env: parent.env.clone(),
        bridge: None,
        author: Some(author.clone()),
    });
    append(log_path, Event::Split {
        id: id.clone(), parent: parent_id.clone(), author,
        children: children.clone(), glue: glue_id.clone(), note,
    });
    ui::step(&format!("split {} registered  {} {} [{}]",
        ui::bold(&id), ui::cyan(&parent_id), ui::dim("←"), children.join(", ")));
    ui::kv("glue", &ui::cyan(&glue_id));
    ui::kv("pinned", &glue_type);
    ui::kv("next", &ui::dim(&format!("prove it and verify like any hole: razor submit --hole {glue_id} …")));
}

fn cmd_status(log_path: &PathBuf) {
    let mut state = State::fold(load(log_path));
    state.settle_admissions();
    state.aggregate_clumps();
    state.aggregate_fidelity();
    state.aggregate_splits();
    state.aggregate_people();
    let arrow = ui::dim("→");
    ui::section("proposals", Some(state.proposals.len()));
    // With ingested catalogues on the log there can be hundreds of
    // proposals; the terminal shows the ones with activity plus a window of
    // the rest. Everything is in the site and `razor log`.
    let (active, idle): (Vec<_>, Vec<_>) = state.proposals.values()
        .partition(|p| !p.statements.is_empty());
    let window = 12usize.saturating_sub(active.len());
    let hidden = idle.len().saturating_sub(window);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    for p in active.iter().chain(idle.iter().take(window)) {
        println!("  {}  {}  {}", ui::cyan(&format!("{:<9}", p.id)), p.title,
            ui::dim(&format!("[{} statements]", p.statements.len())));
        for rid in &p.rounds {
            let Some(r) = state.rounds.get(rid) else { continue };
            let phase = if now < r.closes_at {
                format!("{} {}", ui::gold("◷ challenge window open"),
                    ui::dim(&format!("- sealed readings invited, closes {}", days_from(now, r.closes_at))))
            } else if now < r.reveal_by {
                format!("{} {}", ui::gold("◷ reveal phase"),
                    ui::dim(&format!("- reveals due {}", days_from(now, r.reveal_by))))
            } else {
                ui::dim(&format!("◷ window {rid} closed"))
            };
            println!("             {phase}");
        }
        let pending = p.seals.iter()
            .filter(|s| state.seals.get(*s).is_some_and(|x| x.statement.is_none()))
            .count();
        if pending > 0 {
            println!("             {}", ui::gold(&format!("⏣ {pending} sealed reading{} awaiting reveal",
                if pending == 1 { "" } else { "s" })));
        }
        for c in &p.clumps {
            let tag = match (c.dominant, c.proven) {
                (true, true) => format!("{} {}", ui::gold("◆ dominant"), ui::green("· proven")),
                (true, false) => ui::gold("◆ dominant"),
                (false, true) => format!("{} {}", ui::dim("◇ clump"), ui::green("· proven")),
                (false, false) => ui::dim("◇ clump"),
            };
            let blind = if c.independent >= 2 {
                format!(" {}", ui::green(&format!("· {} written blind", c.independent)))
            } else { String::new() };
            println!("             {tag}  {}{blind}  {}",
                ui::dim(&format!("weight {}", c.weight)), c.members.join(&format!(" {} ", ui::dim("≡"))));
        }
    }
    if hidden > 0 {
        println!("  {}", ui::dim(&format!("… and {hidden} more awaiting formalization (browse them on the site, or `razor log`)")));
    }
    ui::section("statements", Some(state.statements.len()));
    for s in state.statements.values() {
        println!("  {}  {}  {}",
            ui::cyan(&format!("{:<12}", s.id)), format!("by {}", s.author),
            ui::dim(&format!("certs {} · converges {} · implies {}",
                s.certificates.len(), s.convergences.len(), s.implies.len())));
    }
    ui::section("holes", Some(state.holes.len()));
    for h in state.holes.values() {
        let extra = match h.status.as_str() {
            "solved" => format!("  {}", ui::dim(&format!("by {}", h.solved_by.clone().unwrap_or_default()))),
            _ => String::new(),
        };
        let pool = if h.pool > 0 { format!("  {}", ui::pool(h.pool)) } else { String::new() };
        println!("  {}  {}  {}{}{}", ui::cyan(&format!("{:<16}", h.id)), ui::chip(&h.status), h.title, pool, extra);
        let f = &h.fidelity;
        let certs = if f.certificates > 0 { format!(" · {} certificates", f.certificates) } else { String::new() };
        if f.converged {
            println!("      {} {}", ui::green(&format!("⚖ {} formalization authors, equivalence kernel-checked", f.authors)), ui::dim(&certs));
        } else if f.authors == 1 {
            println!("      {}", ui::dim(&format!("⚖ one formalization author - unconverged{certs}")));
        } else {
            println!("      {}", ui::dim("⚖ pinned directly - no formalization trail on record"));
        }
        if let Some(pr) = &h.upstreamed {
            println!("      {}", ui::gold(&format!("↥ upstreamed - {pr}")));
        }
        for (_old, _new, equiv) in &h.repins {
            println!("      {}", ui::dim(&format!("⟲ repinned - wording migrated, equivalence kernel-checked ({equiv})")));
        }
        for (by, replacement, note) in &h.superseded_by {
            let n = if note.is_empty() { String::new() } else { format!(": {note}") };
            println!("      {}", ui::dim(&format!("↷ marked superseded by {replacement} (by {by}{n})")));
        }
        for sp in &h.splits {
            let state_tag = if sp.complete { ui::green("complete") } else { ui::dim("in progress") };
            println!("      {} {} {}  {state_tag}  glue {} {}  {}",
                ui::dim("├"), ui::bold(&sp.id), ui::dim(&format!("by {}", sp.author)),
                ui::cyan(&sp.glue.0),
                if sp.glue.1 == "solved" { ui::green("(solved)") } else { ui::dim("(open)") },
                ui::dim(&format!("children {}/{} solved", sp.solved_children, sp.children.len())));
            for (i, (c, st)) in sp.children.iter().enumerate() {
                let tee = if i + 1 == sp.children.len() { "└" } else { "├" };
                let mark = if st == "solved" { ui::green("✓") } else { ui::cyan("○") };
                println!("      {}   {tee} {mark} {c}", ui::dim("│"));
            }
        }
        for r in &h.zk_routes {
            println!("      {}", ui::dim(&format!("◍ zk route {} - {} constraints, vk {}…, bridge [{}] {}",
                r.id, r.constraints, &r.vk_hash[..8], r.bridge_kind, r.bridge)));
        }
        for z in &h.zk_submissions {
            match &z.verdict {
                Some((true, _)) => println!("      {} zk {} by {} {}",
                    ui::green("✓"), z.id, z.solver, ui::dim(&format!("via {} - proof verified, witness never revealed", z.route))),
                Some((false, why)) => println!("      {} zk {} by {} {}",
                    ui::red("✕"), z.id, z.solver, ui::dim(&format!("- {why}"))),
                None => println!("      {} zk {} by {} {}",
                    ui::dim("·"), z.id, z.solver, ui::dim("- unverified")),
            }
        }
    }
    ui::section("anvil", Some(state.challenges.len()));
    print_leaderboards(&state, None);
    if !state.curations.is_empty() {
        ui::section("curations", Some(state.curations.len()));
        for (who, target, note) in &state.curations {
            let weight = 1 + state.people.get(who).map(|p| p.solved).unwrap_or(0);
            println!("  {} {who} {arrow} {}  {}{}",
                ui::gold("☆"), ui::cyan(target), ui::dim(&format!("(weight {weight})")),
                if note.is_empty() { String::new() } else { ui::dim(&format!(": {note}")) });
        }
    }
    if !state.payouts.is_empty() {
        ui::section("payouts", Some(state.payouts.len()));
        for (target, who, amt, why) in &state.payouts {
            println!("  {} {arrow} {} for {}  {}",
                ui::gold(&ui::commas(*amt)), ui::bold(who), ui::cyan(target), ui::dim(why));
        }
    }
    println!();
}

fn print_leaderboards(state: &State, only: Option<&str>) {
    for c in state.challenges.values() {
        if only.is_some_and(|id| id != c.id) {
            continue;
        }
        println!("  {}  {}", ui::cyan(&format!("{:<9}", c.id)), ui::bold(&c.title));
        let mut boards: std::collections::BTreeMap<String, Vec<(f64, &str, &str, &str, Option<&str>)>> = Default::default();
        for e in &c.entries {
            for s in &e.scores {
                boards.entry(format!("{}/{}", s.tier, s.arch)).or_default().push((
                    s.score, e.impl_name.as_str(), s.unit.as_str(),
                    if e.is_reference { "spec" } else { "submission" },
                    s.rig.as_deref(),
                ));
            }
        }
        for (board, mut rows) in boards {
            rows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let arch = board.split('/').nth(1).unwrap_or_default();
            let pool = c.arch_pools.get(arch)
                .map(|p| format!("  {}", ui::pool(*p)))
                .unwrap_or_default();
            let rig = rows.iter().find_map(|r| r.4)
                .and_then(|rid| state.rigs.get(rid))
                .map(|r| ui::dim(&format!("  rig: {} ({})", r.id, r.owner)))
                .unwrap_or_default();
            println!("    {}{pool}{rig}", ui::bold(&format!("[{board}]")));
            for (i, (score, name, unit, kind, _)) in rows.iter().enumerate() {
                let crown = if i == 0 { ui::gold("♛") } else { " ".into() };
                let name_col = if i == 0 { ui::bold(&format!("{name:<16}")) } else { format!("{name:<16}") };
                println!("      {crown} {name_col} {score:>10.2} {}  {}",
                    ui::dim(unit), ui::dim(&format!("({kind})")));
            }
        }
    }
}

fn export_string(log_path: &PathBuf, dataset: &str) -> (String, usize) {
    let mut state = State::fold(load(log_path));
    state.settle_admissions();
    state.aggregate_clumps();
    state.aggregate_fidelity();
    state.aggregate_splits();
    state.aggregate_people();
    // Attach the Lean source of each hole's pinned definitions, so the site
    // shows what a name unfolds to instead of just the name.
    let index = lean_decl_index(&repo_root());
    for h in state.holes.values_mut() {
        h.lean_source = resolve_lean_sources(&index, &h.lean_type);
        if h.env.as_deref() == Some("mathlib") {
            // Identifiers in the pinned type that resolve in Mathlib rather
            // than locally: the site links each to the Mathlib docs. When
            // *nothing* in the type is local, the hole pins Mathlib's own
            // statement - the strongest fidelity fact there is.
            let idents = lean_idents(&h.lean_type);
            h.fidelity.canonical = !idents.iter().any(|w| index.contains_key(w));
            h.mathlib_names = idents.into_iter()
                .filter(|w| !index.contains_key(w))
                .filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase()))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter().collect();
        }
    }
    // Attach the Lean source of revealed statement files (persisted next to
    // the log), so a sealed reading's actual Lean is readable on the site.
    if let Some(data_dir) = log_path.parent() {
        for st in state.statements.values_mut() {
            if st.seal.is_some() {
                if let Ok(src) = std::fs::read_to_string(
                    data_dir.join("statements").join(format!("{}.lean", st.id))) {
                    st.source = src;
                }
            }
        }
    }
    // Attach each verdict's log position and the log hash through it, so a
    // hole page can show the exact `razor recheck` / `razor cite` facts.
    let verdict_seqs: std::collections::HashMap<String, u64> = state.events.iter()
        .filter_map(|e| match &e.event {
            Event::Verdict { submission, .. } => Some((submission.clone(), e.seq)),
            _ => None,
        })
        .collect();
    for h in state.holes.values_mut() {
        for s in h.submissions.iter_mut() {
            if let Some(&seq) = verdict_seqs.get(&s.id) {
                s.verdict_seq = Some(seq);
                s.log_hash = Some(log_hash_through(log_path, seq));
            }
        }
    }
    let mut json = serde_json::to_value(&state).unwrap();
    // Label which dataset this export came from so the site can say so:
    // "demo" is the scripted walkthrough with fictional participants,
    // "live" is the real registry.
    json["dataset"] = serde_json::Value::String(dataset.into());
    // Compact: data.json is fetched on every page view, and with a
    // thousand events on the log pretty-printing costs real bandwidth.
    (serde_json::to_string(&json).unwrap(), state.events.len())
}

fn cmd_export(log_path: &PathBuf, out: &PathBuf, dataset: Option<String>) {
    let (json, n) = export_string(log_path, dataset.as_deref().unwrap_or("live"));
    std::fs::create_dir_all(out.parent().unwrap()).ok();
    std::fs::write(out, &json).expect("write export");
    ui::step(&format!("exported {} events {} {}", ui::bold(&n.to_string()), ui::dim("→"), out.display()));
}

/// Serve the site with data.json re-derived from the log on demand, plus
/// the participation API (/api/event, /api/submit, /api/verify, /api/log).
/// Each connection gets a thread; appends serialize on a log lock, kernel
/// checks on a verify lock. With RAZOR_MIRROR set, every append is pushed
/// to the public repository.
fn cmd_serve(root: &PathBuf, log_path: &PathBuf, host: &str, port: u16) {
    use std::io::BufReader;
    use std::sync::{Arc, Mutex};
    // Reuse the dataset label of the last export, so serving after demo.sh
    // keeps saying "demo".
    let dataset = std::fs::read_to_string(root.join("site/data.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v["dataset"].as_str().map(String::from))
        .unwrap_or_else(|| "live".into());
    let listener = std::net::TcpListener::bind((host, port)).expect("bind");
    ui::step(&format!("serving {}  {}", ui::bold(&format!("http://{host}:{port}")),
        ui::dim(&format!("(dataset: {dataset}; data.json re-derived from the log on every request)"))));
    if std::env::var("RAZOR_MIRROR").is_ok_and(|v| !v.trim().is_empty()) {
        api::spawn_mirror(root.clone());
        ui::step(&ui::dim("mirror on: every append is committed and pushed to the public repository"));
    }

    struct Ctx {
        root: PathBuf,
        log_path: PathBuf,
        dataset: String,
        cache: Mutex<Option<(u64, u64, String)>>, // (len, mtime, json)
        log_lock: Mutex<()>,
        verify_lock: Mutex<()>,
        limiter: Mutex<api::Limiter>,
    }
    let ctx = Arc::new(Ctx {
        root: root.clone(),
        log_path: log_path.clone(),
        dataset,
        cache: Mutex::new(None),
        log_lock: Mutex::new(()),
        verify_lock: Mutex::new(()),
        limiter: Mutex::new(api::Limiter::default()),
    });

    fn handle(mut stream: std::net::TcpStream, ctx: &Ctx) {
        use std::io::Write;
        let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        });
        let Some(req) = api::read_request(&mut reader, &peer) else { return };
        let query = req.query.as_deref();
        let (status, ctype, body): (String, String, Vec<u8>) = if req.path.starts_with("/api/") {
            match (req.method.as_str(), req.path.as_str()) {
                ("GET", "/api/log") => api::get_log(&ctx.log_path, query),
                ("GET", "/api/submission") => api::get_submission(&ctx.root, &ctx.log_path, query),
                ("POST", "/api/event") => {
                    if !ctx.limiter.lock().unwrap().allow(&format!("e:{}", req.ip), 120) {
                        api::json_response("429 Too Many Requests",
                            serde_json::json!({"ok": false, "error": "rate limited - try again later"}))
                    } else {
                        api::post_event(&ctx.root, &ctx.log_path, &req.body, &ctx.log_lock)
                    }
                }
                ("POST", "/api/submit") | ("POST", "/api/verify") => {
                    if !ctx.limiter.lock().unwrap().allow(&format!("v:{}", req.ip), 12) {
                        api::json_response("429 Too Many Requests",
                            serde_json::json!({"ok": false, "error": "rate limited - verification is capped per hour, try again later"}))
                    } else if req.path == "/api/submit" {
                        api::post_submit(&ctx.root, &ctx.log_path, &req.body, &ctx.log_lock, &ctx.verify_lock)
                    } else {
                        api::post_verify(&ctx.root, &ctx.log_path, &req.body, &ctx.log_lock, &ctx.verify_lock)
                    }
                }
                _ => api::json_response("404 Not Found",
                    serde_json::json!({"ok": false, "error": "unknown api endpoint"})),
            }
        } else if req.path == "/data.json" {
            let meta = std::fs::metadata(&ctx.log_path).ok();
            let key = meta
                .map(|m| (m.len(), m.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()).unwrap_or(0)))
                .unwrap_or((0, 0));
            let mut cache = ctx.cache.lock().unwrap();
            let fresh = match &*cache {
                Some((l, t, _)) if (*l, *t) == key => false,
                _ => true,
            };
            if fresh {
                let (json, _) = export_string(&ctx.log_path, &ctx.dataset);
                *cache = Some((key.0, key.1, json));
            }
            ("200 OK".into(), "application/json".into(),
                cache.as_ref().unwrap().2.clone().into_bytes())
        } else if req.path == "/install.sh" {
            // The installer lives at the repo root - the single source of
            // truth CI and checkouts use. Serving a copy from site/ once let
            // the two drift.
            match std::fs::read(ctx.root.join("install.sh")) {
                Ok(bytes) => ("200 OK".into(), "text/plain".into(), bytes),
                Err(_) => ("404 Not Found".into(), "text/plain".into(), b"not found".to_vec()),
            }
        } else {
            let rel = if req.path == "/" { "index.html" } else { req.path.trim_start_matches('/') };
            if rel.contains("..") {
                ("400 Bad Request".into(), "text/plain".into(), b"no".to_vec())
            } else {
                let file = ctx.root.join("site").join(rel);
                match std::fs::read(&file) {
                    Ok(bytes) => {
                        let ctype = match file.extension().and_then(|e| e.to_str()) {
                            Some("html") => "text/html; charset=utf-8",
                            Some("css") => "text/css",
                            Some("js") => "text/javascript",
                            Some("json") => "application/json",
                            Some("svg") => "image/svg+xml",
                            Some("png") => "image/png",
                            Some("sh") => "text/plain",
                            _ => "application/octet-stream",
                        };
                        // Detail pages set their titles from data.json in the
                        // browser, which link-preview crawlers never run. For
                        // hole.html?id=X etc, stamp the entity's title and a
                        // description into the served HTML so a shared link
                        // shows the mathematics, not the generic page name.
                        let bytes = match (ctype.starts_with("text/html"), query.and_then(query_id)) {
                            (true, Some(id)) => match detail_meta(&ctx.log_path, rel, &id) {
                                Some((title, desc)) => stamp_meta(
                                    String::from_utf8_lossy(&bytes).into_owned(), &title, &desc,
                                ).into_bytes(),
                                None => bytes,
                            },
                            _ => bytes,
                        };
                        ("200 OK".into(), ctype.into(), bytes)
                    }
                    Err(_) => ("404 Not Found".into(), "text/plain".into(), b"not found".to_vec()),
                }
            }
        };
        let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n", body.len());
        let _ = stream.write_all(&body);
    }

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let ctx = Arc::clone(&ctx);
        std::thread::spawn(move || handle(stream, &ctx));
    }
}

/// The `id` value from a query string, minimally percent-decoded (entity
/// ids are plain ASCII, but a linking page may still encode them).
fn query_id(query: &str) -> Option<String> {
    let raw = query.split('&').find_map(|kv| kv.strip_prefix("id="))?;
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            match u8::from_str_radix(&hex, 16) {
                Ok(b) => out.push(b as char),
                Err(_) => { out.push('%'); out.push_str(&hex); }
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Title and description for a detail page's entity, from a fold of the log.
fn detail_meta(log_path: &PathBuf, rel: &str, id: &str) -> Option<(String, String)> {
    let mut state = State::fold(load(log_path));
    let brand = |t: String| format!("{t} — Satoshi's Razor");
    match rel {
        "hole.html" => state.holes.get(id).map(|h| (
            brand(format!("{}: {}", h.id, h.title)),
            format!("{} · a solution must prove exactly: {}", h.status, h.lean_type),
        )),
        "proposal.html" => state.proposals.get(id).map(|p| (
            brand(format!("{}: {}", p.id, p.title)),
            p.body.clone(),
        )),
        "statement.html" => state.statements.get(id).map(|st| (
            brand(format!("{}: candidate Lean statement", st.id)),
            if st.gloss.is_empty() { format!("declared as {}", st.decl) } else { st.gloss.clone() },
        )),
        "person.html" => {
            state.aggregate_people();
            state.people.get(id).map(|p| (
                brand(format!("@{id}")),
                format!("{} accepted proof{} — a profile derived entirely from the event log",
                    p.solved, if p.solved == 1 { "" } else { "s" }),
            ))
        }
        _ => None,
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Replace the page's <title> and inject description + OpenGraph tags,
/// dropping the page's own generic ones so nothing is duplicated.
fn stamp_meta(html: String, title: &str, desc: &str) -> String {
    let t = html_escape(title);
    let d = html_escape(&desc.chars().take(200).collect::<String>());
    let html: String = html.lines()
        .filter(|l| !(l.contains("name=\"description\"")
            || l.contains("property=\"og:title\"")
            || l.contains("property=\"og:description\"")))
        .collect::<Vec<_>>()
        .join("\n");
    match (html.find("<title>"), html.find("</title>")) {
        (Some(a), Some(b)) if a < b => format!(
            "{}<title>{t}</title>\n<meta name=\"description\" content=\"{d}\">\n<meta property=\"og:title\" content=\"{t}\">\n<meta property=\"og:description\" content=\"{d}\">{}",
            &html[..a], &html[b + "</title>".len()..],
        ),
        _ => html,
    }
}

// ---------------- lean source index ----------------
// A pinned type like "Razor.Frontier.FLT" is a *name*; the reader must be
// able to see what it unfolds to. These functions build an index of every
// declaration in the Lean packages (fully qualified name -> source text,
// including the doc comment) and resolve a hole's pinned type to the
// definitions it mentions, transitively.

const LEAN_DECL_KEYWORDS: [&str; 6] = ["def ", "theorem ", "abbrev ", "structure ", "inductive ", "lemma "];

/// The module that defines `decl` inside the package at `lean_dir`, so the
/// checker can import it. The package builds every module under its glob,
/// but the generated check file only imports the root module - a decl in a
/// file nothing imports would otherwise be invisible to verification, and
/// contributors should not need to know that.
fn find_decl_module(lean_dir: &PathBuf, root_import: &str, decl: &str) -> Option<String> {
    fn walk(dir: &PathBuf, lean_dir: &PathBuf, decl: &str) -> Option<String> {
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(m) = walk(&path, lean_dir, decl) {
                    return Some(m);
                }
            } else if path.extension().is_some_and(|e| e == "lean") {
                let mut idx = std::collections::BTreeMap::new();
                scan_lean_file(&path, &mut idx);
                if idx.contains_key(decl) {
                    let rel = path.strip_prefix(lean_dir).ok()?.with_extension("");
                    return Some(rel.components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("."));
                }
            }
        }
        None
    }
    walk(&lean_dir.join(root_import), lean_dir, decl)
        .filter(|m| m != root_import)
}

fn lean_decl_index(root: &PathBuf) -> std::collections::BTreeMap<String, (String, String)> {
    let mut index = std::collections::BTreeMap::new();
    for pkg in ["lean/Razor", "lean-mathlib/RazorMathlib"] {
        let dir = root.join(pkg);
        if dir.exists() {
            scan_lean_dir(&dir, &mut index);
        }
    }
    index
}

fn scan_lean_dir(dir: &PathBuf, index: &mut std::collections::BTreeMap<String, (String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_lean_dir(&path, index);
        } else if path.extension().is_some_and(|e| e == "lean") {
            scan_lean_file(&path, index);
        }
    }
}

fn scan_lean_file(path: &PathBuf, index: &mut std::collections::BTreeMap<String, (String, String)>) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let mut ns: Vec<String> = vec![];
    let starts_item = |l: &str| -> bool {
        let ls = l.strip_prefix("private ").unwrap_or(l);
        l.starts_with("/--") || l.starts_with("/-!") || l.starts_with("-- ")
            || l.starts_with("namespace ") || l.starts_with("end") || l.starts_with("import ")
            || LEAN_DECL_KEYWORDS.iter().any(|k| ls.starts_with(k))
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if let Some(rest) = line.strip_prefix("namespace ") {
            ns.push(rest.trim().to_string());
            i += 1;
            continue;
        }
        if line == "end" || line.starts_with("end ") {
            ns.pop();
            i += 1;
            continue;
        }
        // A declaration, optionally preceded by its doc comment.
        let start = i;
        let mut j = i;
        if line.starts_with("/--") {
            while j < lines.len() && !lines[j].contains("-/") {
                j += 1;
            }
            j += 1;
        }
        let decl_line = lines.get(j).copied().unwrap_or("");
        let private = decl_line.starts_with("private ");
        let stripped = decl_line.strip_prefix("private ").unwrap_or(decl_line);
        if let Some(kw) = LEAN_DECL_KEYWORDS.iter().find(|k| stripped.starts_with(*k)) {
            let name: String = stripped[kw.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '\'')
                .collect();
            let mut k = j + 1;
            while k < lines.len() && !starts_item(lines[k]) {
                k += 1;
            }
            if !private && !name.is_empty() {
                let src = lines[start..k].join("\n").trim_end().to_string();
                let fq = if ns.is_empty() { name } else { format!("{}.{}", ns.join("."), name) };
                index.insert(fq, (src, ns.join(".")));
            }
            i = k;
            continue;
        }
        i += 1;
    }
}

fn lean_idents(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' || c == '.' || c == '\'' {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn resolve_lean_sources(
    index: &std::collections::BTreeMap<String, (String, String)>,
    lean_type: &str,
) -> Vec<(String, String)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut order: Vec<String> = vec![];
    let mut queue: Vec<String> = lean_idents(lean_type)
        .into_iter()
        .filter(|w| index.contains_key(w))
        .collect();
    while let Some(name) = queue.first().cloned() {
        queue.remove(0);
        if !seen.insert(name.clone()) {
            continue;
        }
        order.push(name.clone());
        if order.len() >= 20 {
            break;
        }
        let (src, decl_ns) = &index[&name];
        for w in lean_idents(src) {
            // References inside a namespace are usually unqualified; try the
            // token as-is and qualified by the declaration's namespace.
            let qualified = if decl_ns.is_empty() { w.clone() } else { format!("{decl_ns}.{w}") };
            for cand in [w, qualified] {
                if index.contains_key(&cand) && !seen.contains(&cand) && !queue.contains(&cand) {
                    queue.push(cand);
                }
            }
        }
    }
    order
        .into_iter()
        .map(|n| {
            let src = index[&n].0.clone();
            (n, src)
        })
        .collect()
}

// ---------------- plumbing ----------------

fn repo_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("cwd");
    loop {
        if dir.join("lean/lakefile.toml").exists() {
            return dir;
        }
        if !dir.pop() {
            ui::die("razor runs from inside a satoshis-razor checkout (no lean/lakefile.toml found \
                upward) - cd into your clone first; the install one-liner creates it as \
                ./satoshis-razor in the directory you ran it from");
        }
    }
}

fn load(path: &PathBuf) -> Vec<Entry> {
    let Ok(text) = std::fs::read_to_string(path) else { return vec![] };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("corrupt event log line"))
        .collect()
}

/// Where signing keys live. New keys go to the first entry; lookups walk
/// all of them. The per-user directory is the default home so that keys
/// survive demo runs, `git clean`, and even deleting the clone; the
/// repo-local directory keeps demo/seed keys (which set RAZOR_KEYS_DIR)
/// and pre-existing keys working.
fn keys_dirs(log_path: &PathBuf) -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Ok(d) = std::env::var("RAZOR_KEYS_DIR") {
        if !d.trim().is_empty() { dirs.push(PathBuf::from(d)); }
    } else {
        let cfg = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")));
        if let Ok(cfg) = cfg { dirs.push(cfg.join("razor/keys")); }
    }
    dirs.push(log_path.parent().unwrap().join("keys"));
    dirs
}

fn find_key(log_path: &PathBuf, handle: &str) -> Option<PathBuf> {
    keys_dirs(log_path).into_iter()
        .map(|d| d.join(format!("{handle}.secret")))
        .find(|p| p.exists())
}

/// Sign the event if the acting handle holds a local key; refuse to act in
/// a registered handle's name without its key. Handles that never
/// registered an account stay open and unsigned.
fn sign_event(path: &PathBuf, event: &Event) -> Option<String> {
    // A bench score has no actor field of its own: it is signed by the
    // owner of the rig it was measured on, looked up from the log.
    let actor = match event {
        Event::Bench { rig: Some(r), .. } => {
            let state = State::fold(load(path));
            state.rigs.get(r).map(|rig| rig.owner.clone())?
        }
        _ => event.actor()?.to_string(),
    };
    let entries = load(path);
    let registered = entries.iter().any(|e| matches!(&e.event,
        Event::RegisterAccount { handle, .. } if *handle == actor));
    match find_key(path, &actor).map(std::fs::read_to_string) {
        Some(Ok(hex)) => {
            use ed25519_dalek::Signer;
            let sk = signing_key_from_hex(hex.trim()).expect("malformed key file");
            let msg = serde_json::to_string(event).unwrap();
            Some(hex_of(&sk.sign(msg.as_bytes()).to_bytes()))
        }
        _ if registered => {
            let looked = keys_dirs(path).iter()
                .map(|d| d.join(format!("{actor}.secret")).display().to_string())
                .collect::<Vec<_>>().join(" or ");
            ui::die(&format!("'{actor}' is a registered handle and this machine has no key for it \
                (looked in {looked}) - refusing to append in their name"));
        }
        _ => None,
    }
}

/// Append an already-signed entry to the log, assigning the next seq.
fn append_entry(path: &PathBuf, event: Event, sig: Option<String>) -> Entry {
    let entries = load(path);
    let entry = Entry {
        seq: entries.len() as u64,
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        sig,
        event,
    };
    let mut line = serde_json::to_string(&entry).unwrap();
    line.push('\n');
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).expect("open log");
    f.write_all(line.as_bytes()).expect("append event");
    entry
}

fn append(path: &PathBuf, event: Event) {
    let sig = sign_event(path, &event);
    // Remote mode: publish to the configured registry instead of the local
    // file. The server re-validates and assigns the sequence number; the
    // returned entry extends the local cache so multi-append commands see
    // their own earlier events.
    if let Some(url) = remote() {
        let mut body = serde_json::json!({ "event": event, "sig": sig });
        if let Some(att) = take_remote_attachments() {
            body["attachments"] = att;
        }
        match http_post_json(&format!("{url}/api/event"), &body) {
            Ok(v) => {
                if let Some(entry) = v.get("entry") {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
                        let _ = writeln!(f, "{entry}");
                    }
                }
                ui::step(&format!("published {}", ui::dim(&format!("- event #{} on {url}", v["seq"]))));
            }
            Err(e) => ui::die(&format!("the remote registry refused it: {e}")),
        }
        return;
    }
    let entry = append_entry(path, event, sig);
    // The same feedback the remote path gives: what landed, and where.
    if !APPEND_QUIET.load(std::sync::atomic::Ordering::Relaxed) {
        ui::step(&format!("recorded {}", ui::dim(&format!("- event #{} on the local log", entry.seq))));
    }
}

/// Bulk callers (propose-batch) silence the per-event confirmation.
static APPEND_QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

// Remote-mode plumbing: which commands may target a remote, the configured
// url, and per-command attachments (reveal-statement sends its file+salt so
// the server can check the commitment).

const REMOTE_CMDS: &[&str] = &[
    "propose", "formalize", "seal-statement", "reveal-statement", "round", "bridge",
    "hole", "split", "curate", "tag", "supersede", "fund", "commit", "submit", "verify",
    "account", "status", "profile", "cite", "verify-log", "log", "recheck",
    // The anvil: challenges, lanes, rigs, and scores are ordinary log events.
    // Measurements happen on the rig owner's machine; what the remote gets is
    // the signed score event - exactly the trust model rigs declare.
    "challenge", "anvil-submit", "rig", "bench",
];

/// Commands the remote refuses (they assert kernel facts without a check,
/// or need machinery the remote does not expose). With a remote configured
/// they stop with an explanation instead of silently acting locally.
const REMOTE_REFUSED: &[(&str, &str)] = &[
    ("converge", "converge records an equivalence without a kernel check - on the public registry, \
        use `razor bridge` and prove it; pass --local to write a local registry"),
    ("implies", "implies records an implication without a kernel check on the receiving side - \
        pass --local to write a local registry"),
    ("certify", "certify is not accepted remotely yet - pass --local to write a local registry"),
    ("reveal", "revealing a committed proof is not supported remotely yet - pass --local, or \
        submit the proof file directly with `razor submit --file`"),
    ("repin", "repin is a maintainer operation - pass --local to run it against a local registry"),
];

static REMOTE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
static ATTACHMENTS: std::sync::Mutex<Option<serde_json::Value>> = std::sync::Mutex::new(None);

fn remote() -> Option<&'static str> {
    REMOTE.get().and_then(|o| o.as_deref())
}

/// Whether a default remote is configured at all - even when this run
/// bypassed it with --local. Hints printed by local runs use this to keep
/// their suggested next commands local too.
fn remote_configured() -> bool {
    remote().is_some()
        || match std::env::var("RAZOR_REMOTE") {
            Ok(v) => !v.trim().is_empty(),
            Err(_) => remote_config_path()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .is_some_and(|s| !s.trim().is_empty()),
        }
}

fn set_remote_attachments(v: serde_json::Value) {
    *ATTACHMENTS.lock().unwrap() = Some(v);
}

fn take_remote_attachments() -> Option<serde_json::Value> {
    ATTACHMENTS.lock().unwrap().take()
}

fn remote_config_path() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config/razor/remote"))
}

fn remote_setup(cmd: &str, args: &[String], root: &PathBuf, log_path: PathBuf) -> PathBuf {
    let explicit_local = args.iter().any(|a| a == "--local");
    let url = if explicit_local {
        None
    } else {
        match std::env::var("RAZOR_REMOTE") {
            Ok(v) if v.trim().is_empty() => None, // RAZOR_REMOTE="" forces local
            Ok(v) => Some(v.trim().trim_end_matches('/').to_string()),
            Err(_) => remote_config_path()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .filter(|s| !s.is_empty()),
        }
    };
    if url.is_some() {
        if let Some((_, why)) = REMOTE_REFUSED.iter().find(|(c, _)| *c == cmd) {
            ui::die(why);
        }
    }
    let url = url.filter(|_| REMOTE_CMDS.contains(&cmd));
    let Some(u) = url else {
        REMOTE.set(None).ok();
        return log_path;
    };
    // Refresh the local view of the remote log; validation below runs
    // against current remote state, and the canonical server re-validates.
    let text = http_get(&format!("{u}/api/log")).unwrap_or_else(|e| {
        ui::die(&format!("cannot reach the remote registry at {u} ({e}) - retry, or run against \
            your local registry with --local"));
    });
    let cache = root.join("registry/data/remote.jsonl");
    std::fs::create_dir_all(cache.parent().unwrap()).ok();
    std::fs::write(&cache, text).expect("write remote cache");
    REMOTE.set(Some(u)).ok();
    cache
}

fn cmd_remote(args: &[String]) {
    let Some(p) = remote_config_path() else { ui::die("no HOME - cannot store config") };
    match args.get(1).map(String::as_str) {
        None => match std::fs::read_to_string(&p) {
            Ok(u) if !u.trim().is_empty() => {
                println!("{}", u.trim());
                println!("{}", ui::dim("participation commands publish there; --local opts out per command"));
            }
            _ => println!("{}", ui::dim("no remote configured - commands run against the local registry")),
        },
        Some("off") | Some("none") | Some("clear") => {
            let _ = std::fs::remove_file(&p);
            ui::step("remote cleared - commands run against the local registry");
        }
        Some(url) if url.starts_with("http") => {
            std::fs::create_dir_all(p.parent().unwrap()).ok();
            std::fs::write(&p, format!("{}\n", url.trim_end_matches('/'))).expect("write config");
            ui::step(&format!("remote set to {} {}", ui::bold(url),
                ui::dim("- participation commands publish there (--local opts out)")));
        }
        _ => ui::die("usage: razor remote [<url> | off]"),
    }
}

// HTTP through curl: universally present, TLS included, no new
// dependencies. The server always answers JSON with an `ok` field.

fn http_get(url: &str) -> Result<String, String> {
    let out = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "60", "-f", url])
        .output()
        .map_err(|e| format!("curl unavailable: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn http_post_json(url: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    use std::io::Write;
    let mut child = std::process::Command::new("curl")
        .args(["-sS", "--max-time", "400", "-X", "POST",
               "-H", "Content-Type: application/json", "--data-binary", "@-", url])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl unavailable: {e}"))?;
    child.stdin.take().unwrap().write_all(body.to_string().as_bytes())
        .map_err(|e| format!("send failed: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("curl failed: {e}"))?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|_| {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if err.is_empty() {
            format!("unexpected response: {}", String::from_utf8_lossy(&out.stdout).trim())
        } else {
            err
        }
    })?;
    if v["ok"].as_bool() == Some(true) {
        Ok(v)
    } else {
        Err(v["error"].as_str().unwrap_or("remote error").to_string())
    }
}

fn submission_module(root_import: &str, id: &str) -> String {
    let modname: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{root_import}.Submissions.S{modname}")
}

// ---------------- keys and signatures ----------------

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn bytes_of_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()).collect()
}

fn generate_signing_key() -> ed25519_dalek::SigningKey {
    let mut seed = [0u8; 32];
    use std::io::Read;
    std::fs::File::open("/dev/urandom").expect("open /dev/urandom")
        .read_exact(&mut seed).expect("read entropy");
    ed25519_dalek::SigningKey::from_bytes(&seed)
}

fn signing_key_from_hex(hex: &str) -> Option<ed25519_dalek::SigningKey> {
    let bytes: [u8; 32] = bytes_of_hex(hex)?.try_into().ok()?;
    Some(ed25519_dalek::SigningKey::from_bytes(&bytes))
}

/// Audit every event on the log against the registered pubkeys: an event by
/// a registered handle must carry a valid signature from that handle's key.
/// Events by handles that never registered are reported as open (unsigned).
fn cmd_verify_log(log_path: &PathBuf) {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let entries = load(log_path);
    let mut keys: std::collections::BTreeMap<String, VerifyingKey> = Default::default();
    let (mut signed, mut open, mut bad) = (0u64, 0u64, 0u64);
    for e in &entries {
        // A RegisterAccount event introduces its own key, and must be
        // self-signed by it.
        if let Event::RegisterAccount { handle, pubkey, .. } = &e.event {
            if let Some(vk) = bytes_of_hex(pubkey)
                .and_then(|b| <[u8; 32]>::try_from(b).ok())
                .and_then(|b| VerifyingKey::from_bytes(&b).ok())
            {
                keys.insert(handle.clone(), vk);
            }
        }
        let Some(actor) = e.event.actor() else { continue };
        let Some(vk) = keys.get(actor) else { open += 1; continue };
        let msg = serde_json::to_string(&e.event).unwrap();
        let ok = e.sig.as_deref()
            .and_then(bytes_of_hex)
            .and_then(|b| <[u8; 64]>::try_from(b).ok())
            .map(|b| vk.verify(msg.as_bytes(), &Signature::from_bytes(&b)).is_ok())
            .unwrap_or(false);
        if ok {
            signed += 1;
        } else {
            bad += 1;
            println!("  {} #{} {} by '{}': {}", ui::red("✕"), e.seq,
                serde_json::to_value(&e.event).unwrap()["type"].as_str().unwrap_or("?"),
                actor,
                if e.sig.is_some() { "signature does not verify" } else { "registered handle, no signature" });
        }
    }
    let dot = ui::dim("·");
    println!("{} {} events {dot} {} signed and verified {dot} {} {dot} {}",
        if bad == 0 { ui::green("✓") } else { ui::red("✕") },
        ui::bold(&entries.len().to_string()),
        ui::green(&signed.to_string()),
        ui::dim(&format!("{open} by unregistered handles (open participation)")),
        if bad == 0 { ui::dim("0 bad") } else { ui::red(&format!("{bad} bad")) });
    if bad > 0 {
        std::process::exit(1);
    }
}

/// Run the harness through a rig's runner command prefix (split on
/// whitespace), e.g. `docker run --rm satoshis-anvil-rig native --impl …`.
fn run_json_via(runner: &str, args: &[&str]) -> serde_json::Value {
    let mut words = runner.split_whitespace();
    let program = words.next().unwrap_or_else(|| ui::die("empty rig runner"));
    let out = std::process::Command::new(program)
        .args(words)
        .args(args)
        .output()
        .unwrap_or_else(|e| ui::die(&format!("cannot run rig runner `{runner}`: {e}")));
    if !out.status.success() {
        eprintln!("{} rig runner failed: {}", ui::red("✕"), String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }
    serde_json::from_slice(&out.stdout).expect("harness json")
}

fn run_json(bin: &PathBuf, args: &[&str]) -> serde_json::Value {
    let out = std::process::Command::new(bin).args(args).output().expect("run harness");
    if !out.status.success() && args[0] != "check" {
        eprintln!("{} harness failed: {}", ui::red("✕"), String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }
    serde_json::from_slice(&out.stdout).expect("harness json")
}

/// Per-command usage line and accepted flags. Checked before dispatch: an
/// unrecognized --flag is an error, not a silent no-op - a typo'd or
/// unsupported flag that is dropped without a word loses data on an
/// append-only log (`--sigil`, `--author` were once lost exactly that way).
struct CmdSpec {
    name: &'static str,
    usage: &'static str,
    flags: &'static [&'static str],
}

/// Flags that take no value; every other flag consumes the next argument.
const BOOL_FLAGS: &[&str] = &["--local", "--unchecked", "--all"];

const CMD_SPECS: &[CmdSpec] = &[
    CmdSpec { name: "propose", usage: "razor propose --id P --title T --author A [--body B]",
        flags: &["--id", "--title", "--author", "--body"] },
    CmdSpec { name: "formalize", usage: "razor formalize --id S --proposal P --author A --decl D [--gloss G --notes N]",
        flags: &["--id", "--proposal", "--author", "--decl", "--gloss", "--notes"] },
    CmdSpec { name: "certify", usage: "razor certify --statement S --kind K --decl D [--notes N]",
        flags: &["--statement", "--kind", "--decl", "--notes"] },
    CmdSpec { name: "converge", usage: "razor converge --a S1 --b S2 --decl D",
        flags: &["--a", "--b", "--decl"] },
    CmdSpec { name: "implies", usage: "razor implies --a S1 --b S2 --decl D",
        flags: &["--a", "--b", "--decl"] },
    CmdSpec { name: "hole", usage: "razor hole --id H --title T --lean-type TY [--author A --proposal P --env mathlib --statement S --allow-axiom AX --unchecked]",
        flags: &["--id", "--title", "--lean-type", "--author", "--proposal", "--env", "--statement", "--allow-axiom"] },
    CmdSpec { name: "round", usage: "razor round --id R --proposal P --author A (--days N | --closes-at TS) [--reveal-days N | --reveal-by TS] [--note N]",
        flags: &["--id", "--proposal", "--author", "--days", "--closes-at", "--reveal-days", "--reveal-by", "--note"] },
    CmdSpec { name: "seal-statement", usage: "razor seal-statement --id SEAL --proposal P --author A --commitment HASH",
        flags: &["--id", "--proposal", "--author", "--commitment"] },
    CmdSpec { name: "reveal-statement", usage: "razor reveal-statement --seal SEAL --id S --file F --salt SALT --decl D [--gloss G --notes N]",
        flags: &["--seal", "--id", "--file", "--salt", "--decl", "--gloss", "--notes"] },
    CmdSpec { name: "bridge", usage: "razor bridge --id H --a S1 --b S2 [--author A --title T --env E --allow-axiom AX]",
        flags: &["--id", "--a", "--b", "--author", "--by", "--title", "--env", "--allow-axiom"] },
    CmdSpec { name: "split", usage: "razor split --id SPL --parent H --author A --child C [--child C2 ...] [--note N]",
        flags: &["--id", "--parent", "--author", "--child", "--note"] },
    CmdSpec { name: "submit", usage: "razor submit --id SUB --hole H --solver S --decl D [--file F.lean]",
        flags: &["--id", "--hole", "--solver", "--decl", "--file"] },
    CmdSpec { name: "repin", usage: "razor repin --hole H --author A --lean-type TY --equiv-decl D [--note N]",
        flags: &["--hole", "--author", "--lean-type", "--equiv-decl", "--note"] },
    CmdSpec { name: "propose-batch", usage: "razor propose-batch --file F.jsonl --author A",
        flags: &["--file", "--author"] },
    CmdSpec { name: "cite", usage: "razor cite <id>  (or --submission SUB / --hole H)",
        flags: &["--submission", "--hole"] },
    CmdSpec { name: "supersede", usage: "razor supersede --hole H --replacement H2 --author A [--note N]",
        flags: &["--hole", "--replacement", "--author", "--by", "--note"] },
    CmdSpec { name: "challenge", usage: "razor challenge --id C --title T --spec-impl I --obligation O",
        flags: &["--id", "--title", "--spec-impl", "--obligation"] },
    CmdSpec { name: "anvil-submit", usage: "razor anvil-submit --id A --challenge C --impl I --solver S [--proof-decl D --refinement-hole H]",
        flags: &["--id", "--challenge", "--impl", "--solver", "--proof-decl", "--refinement-hole"] },
    CmdSpec { name: "curate", usage: "razor curate --curator A --target ID [--note N]",
        flags: &["--curator", "--target", "--note"] },
    CmdSpec { name: "tag", usage: "razor tag --target ID --tag LABEL --author A [--note N]",
        flags: &["--target", "--tag", "--author", "--by", "--note"] },
    CmdSpec { name: "fund", usage: "razor fund --target ID --amount N --funder A [--arch ARCH]",
        flags: &["--target", "--hole", "--amount", "--funder", "--arch"] },
    CmdSpec { name: "rig", usage: "razor rig --id R --owner A --arch ARCH --tier T [--runner CMD --note N]",
        flags: &["--id", "--owner", "--arch", "--tier", "--runner", "--note"] },
    CmdSpec { name: "payout", usage: "razor payout --target ID --recipient A --amount N [--reason R]",
        flags: &["--target", "--recipient", "--amount", "--reason"] },
    CmdSpec { name: "seal", usage: "razor seal --file F --salt SALT",
        flags: &["--file", "--salt"] },
    CmdSpec { name: "commit", usage: "razor commit --id SUB --hole H --solver S --commitment HASH",
        flags: &["--id", "--hole", "--solver", "--commitment"] },
    CmdSpec { name: "reveal", usage: "razor reveal --submission SUB --file F --salt SALT --decl D",
        flags: &["--submission", "--file", "--salt", "--decl"] },
    CmdSpec { name: "zk-route", usage: "razor zk-route --id R --hole H --bridge DECL [--bridge-kind theorem --n N --note NOTE]",
        flags: &["--id", "--hole", "--bridge", "--bridge-kind", "--n", "--note"] },
    CmdSpec { name: "zk-submit", usage: "razor zk-submit --id SUB --hole H --route R --solver S --public HEX --proof HEX",
        flags: &["--id", "--hole", "--route", "--solver", "--public", "--proof"] },
    CmdSpec { name: "zk-verify", usage: "razor zk-verify --submission SUB", flags: &["--submission"] },
    CmdSpec { name: "verify", usage: "razor verify --submission SUB", flags: &["--submission"] },
    CmdSpec { name: "recheck", usage: "razor recheck --submission SUB", flags: &["--submission"] },
    CmdSpec { name: "upstream", usage: "razor upstream --hole H [--out F.lean | --pr URL --by A --note N]",
        flags: &["--hole", "--out", "--pr", "--by", "--note"] },
    CmdSpec { name: "export-benchmark", usage: "razor export-benchmark [--out F.jsonl --all]",
        flags: &["--out"] },
    CmdSpec { name: "bench", usage: "razor bench --challenge C [--seed N --iters N --rig R]",
        flags: &["--challenge", "--seed", "--iters", "--rig"] },
    CmdSpec { name: "account", usage: "razor account <new|list> [--handle H --display D --about A --github G --sigil S]",
        flags: &["--handle", "--display", "--about", "--github", "--sigil"] },
    CmdSpec { name: "corpus", usage: "razor corpus --id C --name N --url U --source S --as-of DATE [--stat k=v ... --note N]",
        flags: &["--id", "--name", "--url", "--source", "--as-of", "--stat", "--note"] },
    CmdSpec { name: "export", usage: "razor export [--out F.json --dataset NAME]",
        flags: &["--out", "--dataset"] },
    CmdSpec { name: "serve", usage: "razor serve [--host H --port P]",
        flags: &["--host", "--port"] },
];

static USAGE_LINE: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Refuse unknown flags for any command with a spec, and remember the usage
/// line so missing-flag errors can print it.
fn check_flags(cmd: &str, args: &[String]) {
    let Some(spec) = CMD_SPECS.iter().find(|s| s.name == cmd) else { return };
    USAGE_LINE.set(spec.usage).ok();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        if a.starts_with("--") && !spec.flags.contains(&a) && !BOOL_FLAGS.contains(&a) {
            ui::die(&format!("unknown flag {a} for razor {cmd}\n  usage: {}", spec.usage));
        }
        // A known value-taking flag owns the next argument, so a value that
        // happens to start with -- is not misread as a flag.
        i += if a.starts_with("--") && !BOOL_FLAGS.contains(&a) { 2 } else { 1 };
    }
}

fn req(args: &[String], flag: &str) -> String {
    opt(args, flag).unwrap_or_else(|| {
        match USAGE_LINE.get() {
            Some(u) => ui::die(&format!("missing {flag}\n  usage: {u}")),
            None => ui::die(&format!("missing {flag}")),
        }
    })
}

fn opt(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

fn multi(args: &[String], flag: &str) -> Vec<String> {
    args.iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == flag)
        .filter_map(|(i, _)| args.get(i + 1).cloned())
        .collect()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}
