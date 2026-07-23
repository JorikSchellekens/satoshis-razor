//! Event log and derived state.
//!
//! The registry is an append-only JSONL event log; all state is a fold over
//! it. In the full design the log is a chain and admission verdicts are
//! ZK-verified; here the log is a file and verdicts are produced by actually
//! running the Lean checker (see verify.rs), which is the same trust story
//! minus the settlement layer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Stage 1: informal proposal.
    Propose { id: String, title: String, body: String, author: String },
    /// Stage 2: a candidate formalization of a proposal. `gloss` is the
    /// author's own plain-language reading of their Lean statement -
    /// reviewers compare gloss to proposal (easy) and statement to gloss
    /// (local) instead of statement to proposal in one leap.
    Formalize { id: String, proposal: String, author: String, decl: String, notes: String, #[serde(default)] gloss: String },
    /// Challenge-window certificate attached to a candidate statement.
    Certify { statement: String, kind: String, decl: String, notes: String },
    /// Machine-checked equivalence between two candidate statements (convergence).
    Converge { a: String, b: String, decl: String },
    /// Machine-checked one-way implication between candidate statements:
    /// `decl` proves a -> b. Together with the converse's absence this
    /// mechanically exposes "b is strictly weaker" - no adjudication needed.
    Implies { a: String, b: String, decl: String },
    /// A reading window on a proposal: a dated invitation to file *sealed*
    /// candidate statements (hash commitments) until `closes_at`, then
    /// reveal them by `reveal_by`. The window is a coordination signal, not
    /// a gate - the registry never enforces it and a late seal or reveal
    /// simply carries its own timestamps. What sealing buys is a checkable
    /// fact: two statements each committed before the other was revealed
    /// were written blind to each other, which turns the funnel's
    /// independence assumption into a recorded fact.
    OpenRound { id: String, proposal: String, author: String, closes_at: u64, reveal_by: u64, #[serde(default)] note: String },
    /// A sealed candidate statement: sha256(file ‖ salt) of a statement
    /// file the author keeps on their own machine - the same commitment
    /// scheme as private proof submissions. Establishes when the reading
    /// existed without showing it to the other formalizers.
    SealStatement { id: String, proposal: String, author: String, commitment: String },
    /// Opens a statement seal: the revealed file hashed to the commitment
    /// (checked by the CLI before this event is appended), and the reading
    /// it contains enters the funnel as candidate statement `statement` -
    /// an ordinary Formalize, plus the provenance of its seal.
    RevealStatement { seal: String, statement: String, author: String, decl: String, #[serde(default)] gloss: String, #[serde(default)] notes: String },
    /// A ratified sorry: a pinned Lean statement waiting for a proof.
    /// (Wire compatibility: before 2026-07 the log called a sorry a "hole" -
    /// `register_hole` events and `hole` fields deserialize forever.)
    #[serde(alias = "register_hole")]
    RegisterSorry {
        id: String,
        title: String,
        statement: String,
        /// Pinned Lean type the solving declaration must inhabit.
        lean_type: String,
        /// Extra axiom substrings allowed beyond the standard three.
        #[serde(default)]
        allowed_axioms: Vec<String>,
        #[serde(default)]
        proposal: Option<String>,
        /// Verification environment: "core" (default, core Lean only) or
        /// "mathlib" (statements written with Mathlib's definitions, checked in
        /// the lean-mathlib package - see mathlib-env.sh).
        #[serde(default)]
        env: Option<String>,
        /// Set on a bridge sorry: the two candidate statements whose
        /// equivalence this sorry pins. The CLI composes the pinned type
        /// mechanically - `(a's decl) ↔ (b's decl)` - so an admitted proof
        /// is a kernel-checked equivalence, and the two statements' clumps
        /// merge. This is `converge` routed through the ordinary
        /// submit/verify path: attributed, fundable, and checked.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bridge: Option<(String, String)>,
        /// Who registered the pin. Absent on registry-seeded sorries and on
        /// events from before this field existed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        author: Option<String>,
    },
    /// Stage 3: a split - one named way of reducing a parent sorry to child
    /// sorries plus a glue sorry. The glue sorry's pinned statement is exactly
    /// `(child 1) → ... → (child n) → parent`, composed by the CLI from
    /// types that are already pinned, so an admitted glue proof is a
    /// kernel-checked fact that the children jointly suffice. Several
    /// splits of one parent coexist; a split is never edited or deleted -
    /// refactoring a decomposition means registering another split, because
    /// a proven glue is a true theorem no matter which plan people follow.
    Split {
        id: String,
        parent: String,
        author: String,
        children: Vec<String>,
        /// Sorry id carrying the composed glue statement.
        glue: String,
        #[serde(default)]
        note: String,
    },
    /// A claimed solution: `decl` (in the Lean package) allegedly closes `sorry`.
    /// `module` is set when the proof arrived as a standalone file that the
    /// CLI installed into the package (razor submit --file).
    Submit {
        id: String, #[serde(alias = "hole")] sorry: String, solver: String, decl: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module: Option<String>,
    },
    /// Private submission, step 1: a hash commitment to a proof file the
    /// solver keeps on their own machine. Establishes priority without
    /// revealing anything; nobody can front-run a hash.
    Commit { id: String, #[serde(alias = "hole")] sorry: String, solver: String, commitment: String },
    /// Private submission, step 2: the revealed file hashed to the
    /// commitment (checked by the CLI before this event is appended) and was
    /// installed as Lean module `module`; `decl` is the claimed solution.
    Reveal { submission: String, decl: String, module: String },
    /// Verifier verdict for a submission (written by `razor verify`).
    /// `cost_ms` records how long the kernel check took: a fact, recorded so
    /// "this statement was trivially provable" is visible without anyone
    /// having to rule on it.
    Verdict { submission: String, admitted: bool, axioms: Vec<String>, detail: String, #[serde(default)] cost_ms: u64 },
    /// Statement migration: repin a sorry's exact statement to a new wording,
    /// justified by a machine-checked equivalence proof (`equiv_decl` proves
    /// `new ↔ old` and is kernel-checked by the CLI before this event is
    /// appended). This is how a sorry survives toolchain and library churn:
    /// the old wording, the new wording, and the equivalence all stay on the
    /// log, so proofs admitted against either wording remain valid - truth
    /// transfers along the proven equivalence.
    Repin {
        #[serde(alias = "hole")]
        sorry: String,
        author: String,
        /// The new pinned Lean type.
        lean_type: String,
        /// Kernel-checked declaration proving new ↔ old.
        equiv_decl: String,
        #[serde(default)]
        note: String,
    },
    /// An admitted proof left the registry for a home library (normally
    /// Mathlib): `pr_url` is the pull request or commit that carried it.
    /// The registry measures itself by these - a proof that lands upstream
    /// is one the rest of formal mathematics actually builds on.
    Upstream { #[serde(alias = "hole")] sorry: String, by: String, pr_url: String, #[serde(default)] note: String },
    /// An attributed, weighted opinion that `replacement` states the same
    /// problem better than `sorry`. Nothing closes: the sorry stays exactly as
    /// provable as before and its proofs still count. Marks are weighted by
    /// the filer's verified record, like curations, and several marks - even
    /// disagreeing ones - can coexist on one sorry.
    Supersede { #[serde(alias = "hole")] sorry: String, by: String, replacement: String, #[serde(default)] note: String },
    /// An attributed label on any recorded entity (a sorry, proposal,
    /// statement, account, or challenge). Like a curation or a supersession
    /// mark, it changes no status and closes nothing: it is a signed, public
    /// note that travels with the target. The one tag the site itself acts
    /// on is `test-data` - tagged items are de-emphasized in default views
    /// and left out of the homepage marquee, with the tag and its filer
    /// shown wherever the item appears.
    Tag { target: String, tag: String, by: String, #[serde(default)] note: String },
    /// Anvil: register a performance challenge over a ratified spec.
    /// `seed` and `iters` pin the benchmark workload: the input-stream seed
    /// and the number of words per run. Scores at any other workload are
    /// not comparable and never enter the leaderboards. Absent on
    /// challenges from before workloads were pinned; those are pinned
    /// after the fact by a `PinWorkload` event.
    RegisterChallenge {
        id: String,
        title: String,
        spec_impl: String,
        /// Pinned refinement obligation template, described for humans.
        obligation: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seed: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        iters: Option<u64>,
    },
    /// Anvil: pin the benchmark workload of a challenge registered before
    /// workloads were part of registration. Valid only while the challenge
    /// has no pin; the first pin is permanent - like every pinned thing on
    /// the log, changing it would silently re-price every recorded score.
    PinWorkload { challenge: String, seed: u64, iters: u64, by: String },
    /// Anvil: an implementation submission (code + refinement proof decl).
    AnvilSubmit {
        id: String,
        challenge: String,
        impl_name: String,
        solver: String,
        /// Lean theorem: model refines spec. Empty for the reference impl.
        proof_decl: String,
        /// Sorry carrying the pinned refinement statement (verified like any sorry).
        #[serde(default)]
        #[serde(alias = "refinement_hole")]
        refinement_sorry: Option<String>,
    },
    /// Anvil: a measured score for an admitted submission. `rig` names the
    /// hardware it was measured on (None = the reference environment).
    /// `seed` and `iters` record the workload the score was measured at;
    /// only scores at the challenge's pinned workload enter leaderboards.
    /// Absent on scores from before workloads were recorded - those stay
    /// on the log as history but rank nothing.
    Bench {
        submission: String, tier: String, arch: String, score: f64, unit: String, checksum: u64,
        #[serde(default)] rig: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")] seed: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")] iters: Option<u64>,
    },
    /// A benchmark rig: hardware a bounty provider selects or brings to the
    /// table. Scores recorded through a rig carry its architecture; a rig
    /// owner runs `razor bench --rig <id>` on their own machine. `runner` is
    /// an optional command prefix the harness is executed through (for
    /// example `docker run --rm <image>`), so a rig can be a container or a
    /// virtual machine rather than the host itself. Empty = run directly.
    RegisterRig { id: String, owner: String, arch: String, tier: String, note: String, #[serde(default, skip_serializing_if = "String::is_empty")] runner: String },
    /// An account: a handle someone claims from the CLI. `pubkey` is the
    /// hash of a locally held secret; the registry never stores the secret.
    /// `github` is an optional bridge to an existing identity: to make it
    /// checkable by anyone, publish the pubkey from your GitHub account (a
    /// gist or profile repo containing `razor:<pubkey>`).
    RegisterAccount {
        handle: String, display: String, about: String, sigil: String, pubkey: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        github: String,
    },
    /// A zk route: an attachment that makes an existing sorry solvable by a
    /// zero-knowledge proof. It pins a Groth16 verifying key and the bridge
    /// tying circuit satisfaction to the sorry's pinned statement.
    /// `bridge_kind` is "theorem" - `bridge` names a kernel-checked Lean
    /// theorem that the constraints imply the statement - or "binary-hash" -
    /// `bridge` is the hash of a proof-checker binary executed inside a
    /// zkVM (the universal route). The universal route's bridge can never be
    /// a Lean theorem: "the kernel accepts a proof of A implies A" is Lean's
    /// own soundness, unprovable in Lean, so it stays an auditable claim.
    /// `constraints` is the circuit size, which is also the golf score.
    /// Several routes can coexist on one sorry; a route is never edited.
    ZkRoute {
        id: String,
        #[serde(alias = "hole")]
        sorry: String,
        vk_path: String,
        vk_hash: String,
        constraints: u64,
        bridge_kind: String,
        bridge: String,
        #[serde(default)]
        note: String,
    },
    /// A ZK submission: proof + public inputs, no witness. Targets a sorry
    /// through one of its registered routes.
    ZkSubmit { id: String, #[serde(alias = "hole")] sorry: String, route: String, solver: String, public: String, proof: String },
    /// A curation: a public, attributed mark that the curator considers the
    /// target (a proposal, statement, or sorry) worth working on. Costless to
    /// file, but weighted by the curator's verified work on the record, so
    /// taste is scoreable the same way proofs are.
    Curate { curator: String, target: String, note: String },
    /// A bounty attached to one exact pinned statement (a sorry or
    /// anvil challenge) - never to a proposal. The funder pays for the
    /// literal statement as written: the first admitted proof takes the
    /// pool, degenerate proofs included, with no adjudication and no
    /// refunds. Whether the statement deserves that confidence is what
    /// clumps, certificates, and glosses exist to inform. With `arch`, the
    /// pool is reserved for that architecture's crown.
    Fund { target: String, amount: u64, funder: String, #[serde(default)] arch: Option<String> },
    /// Payout on an admitted solution / crown change.
    Payout { target: String, recipient: String, amount: u64, reason: String },
    /// Recognition of an external body of already-verified work (for example
    /// Mathlib). The registry does not re-verify or duplicate it; it records
    /// the corpus so sorries can be closed by citation to it and so the site
    /// can show what is already done. `stats` are sourced numbers, not
    /// registry-generated ones - `source` and `as_of` say where they came
    /// from and when.
    RecognizeCorpus {
        id: String,
        name: String,
        url: String,
        note: String,
        stats: Vec<(String, String)>,
        source: String,
        as_of: String,
    },
}

/// The canonical JSON forms an event's signature may be over. New clients
/// sign the current serialization; events signed before the sorry -> sorry
/// wire rename were signed over the old key names, so verification accepts
/// either form. The legacy form is rebuilt deterministically by renaming
/// the keys back on the serialized string.
pub fn canonical_forms(event: &Event) -> Vec<String> {
    let new = serde_json::to_string(event).unwrap();
    let legacy = new
        .replace("\"type\":\"register_sorry\"", "\"type\":\"register_hole\"")
        .replace("\"refinement_sorry\":", "\"refinement_hole\":")
        .replace("\"sorry\":", "\"hole\":");
    if legacy == new { vec![new] } else { vec![new, legacy] }
}

impl Event {
    /// The handle acting in this event, if the event has one. Verdicts and
    /// payouts are written by the registry itself and have no actor.
    pub fn actor(&self) -> Option<&str> {
        match self {
            Event::Propose { author, .. } => Some(author),
            Event::Formalize { author, .. } => Some(author),
            Event::OpenRound { author, .. } => Some(author),
            Event::SealStatement { author, .. } => Some(author),
            Event::RevealStatement { author, .. } => Some(author),
            Event::Split { author, .. } => Some(author),
            Event::Repin { author, .. } => Some(author),
            Event::Upstream { by, .. } => Some(by),
            Event::Submit { solver, .. } => Some(solver),
            Event::Commit { solver, .. } => Some(solver),
            Event::ZkSubmit { solver, .. } => Some(solver),
            Event::AnvilSubmit { solver, .. } => Some(solver),
            Event::Curate { curator, .. } => Some(curator),
            Event::Supersede { by, .. } => Some(by),
            Event::Tag { by, .. } => Some(by),
            Event::RegisterSorry { author, .. } => author.as_deref(),
            Event::Fund { funder, .. } => Some(funder),
            Event::RegisterRig { owner, .. } => Some(owner),
            Event::PinWorkload { by, .. } => Some(by),
            Event::RegisterAccount { handle, .. } => Some(handle),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub seq: u64,
    pub ts: u64,
    /// Ed25519 signature (hex) over the canonical JSON of `event`, by the
    /// acting handle's registered key. Absent for events whose actor never
    /// registered an account - participation stays open; what a signature
    /// adds is that a *registered* handle cannot be impersonated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
    #[serde(flatten)]
    pub event: Event,
}

// ---------------- derived state ----------------

#[derive(Serialize, Default, Clone, Debug)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub body: String,
    pub author: String,
    pub statements: Vec<String>,
    /// Derived: candidate statements grouped by machine-checked equivalence.
    pub clumps: Vec<Clump>,
    /// Reading windows opened on this proposal, in log order.
    #[serde(default)]
    pub rounds: Vec<String>,
    /// Statement seals filed on this proposal, in log order (revealed or not).
    #[serde(default)]
    pub seals: Vec<String>,
}

/// A reading window, as recorded on the log. Never enforced: it is a
/// dated invitation, and the trust math (mutual blindness of seals) is
/// computed from event order, not from these dates.
#[derive(Serialize, Clone, Debug)]
pub struct Round {
    pub id: String,
    pub proposal: String,
    pub author: String,
    pub closes_at: u64,
    pub reveal_by: u64,
    pub note: String,
    pub opened_ts: u64,
}

/// A statement seal: a commitment to a reading that may or may not have
/// been revealed yet. `statement` is filled when the reveal lands.
#[derive(Serialize, Clone, Debug)]
pub struct Seal {
    pub id: String,
    pub proposal: String,
    pub author: String,
    pub commitment: String,
    pub seq: u64,
    pub ts: u64,
    pub statement: Option<String>,
}

/// A clump: candidate statements of one proposal proven pairwise equivalent.
/// Its weight counts distinct authors (an independence proxy - equivalent
/// statements from one author are one voice). A clump is dominant when it is
/// the unique heaviest clump with at least two independent members.
/// `proven` is a recorded fact, not a judgment: some member's sorry has an
/// admitted proof (truth transfers along equivalence edges to the whole
/// clump). The registry never rules on why: a proven, weight-1, off-dominant
/// clump speaks for itself.
#[derive(Serialize, Clone, Debug)]
pub struct Clump {
    pub members: Vec<String>,
    pub weight: usize,
    pub dominant: bool,
    pub proven: bool,
    /// The strongest independence fact on record: the size of the largest
    /// set of distinct-author members that are pairwise mutually blind -
    /// each sealed (or filed) before every other was revealed, so none
    /// could have seen another's Lean. Weight counts *claimed* independence
    /// (distinct authors); this counts *provable* independence.
    #[serde(default)]
    pub independent: usize,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Statement {
    pub id: String,
    pub proposal: String,
    pub author: String,
    pub decl: String,
    pub notes: String,
    pub gloss: String,
    pub certificates: Vec<(String, String, String)>, // kind, decl, notes
    pub convergences: Vec<(String, String)>,         // other statement, decl
    pub implies: Vec<(String, String)>,              // weaker statement, decl
    pub implied_by: Vec<(String, String)>,           // stronger statement, decl
    /// Log seq of the event that made this statement public (its Formalize,
    /// or its RevealStatement). Blindness math reads event order, not walls.
    #[serde(default)]
    pub filed_seq: u64,
    /// Log seq of the seal commitment, for sealed statements: proof the
    /// reading existed - unseen - at that point in the log.
    #[serde(default)]
    pub sealed_seq: Option<u64>,
    /// The seal this statement was revealed from, if it was sealed.
    #[serde(default)]
    pub seal: Option<String>,
    /// Filled at export time: the revealed statement file's Lean source,
    /// when it is persisted under registry/data/statements. Lets the site
    /// show the actual Lean a sealed reading contained.
    #[serde(default)]
    pub source: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Submission {
    pub id: String,
    pub sorry: String,
    pub solver: String,
    pub decl: String,
    pub verdict: Option<(bool, Vec<String>, String)>,
    /// Filled at export time: the log seq of the verdict event, and the
    /// sha256 of the log through it - what `razor recheck` and `razor cite`
    /// pin, shown on the site so a reader can re-derive the verdict.
    #[serde(default)]
    pub verdict_seq: Option<u64>,
    #[serde(default)]
    pub log_hash: Option<String>,
    /// Present on private submissions: the sha256 commitment.
    pub commitment: Option<String>,
    /// Lean module the revealed file was installed as (private path only).
    pub module: Option<String>,
    /// false while committed-but-unrevealed.
    pub revealed: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Sorry {
    pub id: String,
    pub title: String,
    pub statement: String,
    pub lean_type: String,
    pub allowed_axioms: Vec<String>,
    pub proposal: Option<String>,
    pub env: Option<String>,
    /// Set on a bridge sorry: the two statements whose equivalence it pins.
    /// When it is solved, the two statements' clumps merge.
    #[serde(default)]
    pub bridge: Option<(String, String)>,
    /// Who registered the pin (None on registry-seeded and legacy sorries).
    #[serde(default)]
    pub registered_by: Option<String>,
    pub status: String, // open | solved
    pub solved_by: Option<String>,
    /// Statement migrations, oldest first: (old type, new type, equivalence
    /// decl). `lean_type` above is always the latest wording; the history
    /// is kept so proofs admitted against an earlier wording stay auditable.
    #[serde(default)]
    pub repins: Vec<(String, String, String)>,
    /// Recorded fidelity facts about the pinned statement - see `Fidelity`.
    /// Derived, filled by `aggregate_fidelity`.
    #[serde(default)]
    pub fidelity: Fidelity,
    /// Set when an admitted proof of this sorry was carried to a home
    /// library: the pull request or commit URL, from an `Upstream` event.
    #[serde(default)]
    pub upstreamed: Option<String>,
    /// Supersession marks filed against this sorry: (by, replacement, note).
    /// Opinions, weighted by the reader; the sorry itself never closes.
    pub superseded_by: Vec<(String, String, String)>,
    pub submissions: Vec<Submission>,
    /// Zero-knowledge routes registered against this sorry (see `ZkRoute`).
    pub zk_routes: Vec<ZkRouteRec>,
    /// Zero-knowledge submissions: Groth16 proofs against a route's
    /// verifying key. An admitted one solves the sorry like any proof.
    pub zk_submissions: Vec<ZkSubmission>,
    /// Filled at export time: the Lean source of the definitions the pinned
    /// type mentions (transitively), so a reader can audit the statement
    /// without a checkout. (name, source) pairs, pinned name first.
    #[serde(default)]
    pub lean_source: Vec<(String, String)>,
    /// Filled at export time, Mathlib-environment sorries only: identifiers in
    /// the pinned type that resolve in Mathlib rather than locally, so the
    /// site can link each one to the Mathlib documentation.
    #[serde(default)]
    pub mathlib_names: Vec<String>,
    /// Derived, filled by `aggregate_splits`: every registered way of
    /// reducing this sorry to child sorries.
    pub splits: Vec<SplitView>,
    /// Derived: split ids this sorry serves in, as a child or as the glue.
    pub part_of: Vec<String>,
    pub pool: u64,
}

/// Recorded facts about how much independent scrutiny a sorry's pinned
/// statement has survived. The hardest problem in formalization is not
/// proving a statement but trusting that the statement is the theorem it
/// claims to be; these are the log's answers, with no judgment encoded -
/// the reader weighs them. All counts are over the statement's equivalence
/// clump, because kernel-checked equivalence transfers scrutiny.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Fidelity {
    /// Distinct authors across the clump - independent formalizations.
    pub authors: usize,
    /// At least two independent authors' statements, proven equivalent by
    /// kernel check. The strongest mechanical evidence a formalization is
    /// faithful: two people read the same words and their Lean agrees.
    pub converged: bool,
    /// Provably independent authors in the clump: the largest set of
    /// distinct-author members pairwise sealed before one another's
    /// reveals. `authors` counts claimed independence; this counts what
    /// the log can prove.
    #[serde(default)]
    pub independent: usize,
    /// Sanity certificates attached across the clump.
    pub certificates: usize,
    /// Wording migrations survived, each one a kernel-checked equivalence
    /// to the previous wording (see `Repin`).
    pub repins: usize,
    /// The pinned type resolves entirely to the home library's own names
    /// (e.g. Mathlib's `FermatLastTheorem`): the statement is not a local
    /// translation at all, it is the library's canonical wording. Filled at
    /// export time, since deciding it needs the local declaration index.
    pub canonical: bool,
}

/// A split, as recorded on the log.
#[derive(Serialize, Clone, Debug)]
pub struct SplitRec {
    pub id: String,
    pub parent: String,
    pub author: String,
    pub children: Vec<String>,
    pub glue: String,
    pub note: String,
}

/// One split of a sorry, with the current status of every part. All fields
/// are recorded facts: `complete` means the glue and every child have
/// admitted proofs - at that point the parent is provable by composition.
#[derive(Serialize, Clone, Debug)]
pub struct SplitView {
    pub id: String,
    pub author: String,
    pub note: String,
    pub children: Vec<(String, String)>, // (sorry id, status)
    pub glue: (String, String),          // (sorry id, status)
    pub solved_children: usize,
    pub complete: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct AnvilEntry {
    pub id: String,
    pub challenge: String,
    pub impl_name: String,
    pub solver: String,
    pub proof_decl: String,
    pub refinement_sorry: Option<String>,
    pub admitted: bool,
    pub is_reference: bool,
    pub scores: Vec<Score>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Score {
    pub tier: String,
    pub arch: String,
    pub score: f64,
    pub unit: String,
    pub checksum: u64,
    pub rig: Option<String>,
    /// The workload the score was measured at (always the challenge's pin
    /// for scores that made it into the derived state).
    pub seed: Option<u64>,
    pub iters: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Challenge {
    pub id: String,
    pub title: String,
    pub spec_impl: String,
    pub obligation: String,
    /// The pinned benchmark workload (seed, words per run). None only on
    /// legacy challenges that have not been pinned yet; those accept no
    /// new remote scores until they are.
    pub workload: Option<(u64, u64)>,
    pub entries: Vec<AnvilEntry>,
    pub pool: u64,
    /// Architecture-reserved pools: arch → amount.
    pub arch_pools: BTreeMap<String, u64>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Account {
    pub handle: String,
    pub display: String,
    pub about: String,
    pub sigil: String,
    pub pubkey: String,
    #[serde(default)]
    pub github: String,
}

/// Everything the log knows about one participant, registered or not.
#[derive(Serialize, Clone, Debug, Default)]
pub struct Person {
    pub handle: String,
    pub account: Option<Account>,
    pub first_seen: Option<u64>, // event seq
    /// (seq, submission id, sorry/challenge, kind, outcome) - kind is
    /// "proof" | "zk" | "forge" | "commit"; outcome "admitted" | "rejected" | "pending" | "sealed".
    pub submissions: Vec<(u64, String, String, String, String)>,
    pub solved: u64,
    pub rejected: u64,
    /// Anvil lanes: (challenge, impl, best per board "tier/arch" -> score unit, leader?)
    pub lanes: Vec<(String, String, String, f64, String, bool)>,
    pub top_spots: u64,
    pub payouts_total: u64,
    pub funded_total: u64,
    pub proposals: Vec<String>,
    pub statements: Vec<String>,
    /// Open sorries under this person's proposals - work they are waiting on.
    pub open_sorries_authored: Vec<String>,
    pub rigs: Vec<String>,
    /// Targets this person has curated.
    pub curated: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ZkSubmission {
    pub id: String,
    pub route: String,
    pub solver: String,
    pub public: String,
    pub proof_prefix: String,
    pub verdict: Option<(bool, String)>,
}

/// A zk route attached to a sorry, as recorded on the log.
#[derive(Serialize, Clone, Debug)]
pub struct ZkRouteRec {
    pub id: String,
    pub vk_path: String,
    pub vk_hash: String,
    pub constraints: u64,
    pub bridge_kind: String, // "theorem" | "binary-hash"
    pub bridge: String,
    pub note: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Corpus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub note: String,
    pub stats: Vec<(String, String)>,
    pub source: String,
    pub as_of: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Rig {
    pub id: String,
    pub owner: String,
    pub arch: String,
    pub tier: String,
    pub note: String,
    /// Command prefix the harness runs through (e.g. `docker run --rm <image>`).
    /// Empty means the harness binary is executed directly on the host.
    pub runner: String,
}

#[derive(Serialize, Default, Debug)]
pub struct State {
    pub proposals: BTreeMap<String, Proposal>,
    pub statements: BTreeMap<String, Statement>,
    /// Reading windows, by id.
    pub rounds: BTreeMap<String, Round>,
    /// Statement seals, by id - revealed and pending alike.
    pub seals: BTreeMap<String, Seal>,
    pub sorries: BTreeMap<String, Sorry>,
    pub challenges: BTreeMap<String, Challenge>,
    pub rigs: BTreeMap<String, Rig>,
    pub corpora: BTreeMap<String, Corpus>,
    pub accounts: BTreeMap<String, Account>,
    /// Derived, filled by `aggregate_people` before export.
    pub people: BTreeMap<String, Person>,
    /// (curator, target, note) in log order.
    pub curations: Vec<(String, String, String)>,
    /// Supersession marks: (by, sorry, replacement, note). Attributed,
    /// weighted opinions that one wording replaces another; nothing closes.
    pub supersessions: Vec<(String, String, String, String)>,
    /// Tags: (by, target, tag, note) in log order. Attributed labels; the
    /// site de-emphasizes `test-data`-tagged items but nothing closes.
    pub tags: Vec<(String, String, String, String)>,
    /// Splits in log order; per-sorry views are derived by `aggregate_splits`.
    pub splits: Vec<SplitRec>,
    pub payouts: Vec<(String, String, u64, String)>,
    pub events: Vec<Entry>,
}

impl State {
    pub fn fold(entries: Vec<Entry>) -> State {
        let mut s = State::default();
        for e in &entries {
            s.apply(e.clone());
        }
        s.events = entries;
        s
    }

    fn apply(&mut self, entry: Entry) {
        let (seq, ts) = (entry.seq, entry.ts);
        match entry.event {
            Event::Propose { id, title, body, author } => {
                // Ingested catalogue rows occasionally carry raw HTML
                // entities ("Stolper&ndash;Samuelson"); the log keeps them
                // as recorded, the derived state shows them decoded.
                self.proposals.insert(id.clone(), Proposal {
                    id, title: html_unescape(&title), body: html_unescape(&body),
                    author, statements: vec![], clumps: vec![],
                    rounds: vec![], seals: vec![],
                });
            }
            Event::Formalize { id, proposal, author, decl, notes, gloss } => {
                if let Some(p) = self.proposals.get_mut(&proposal) {
                    p.statements.push(id.clone());
                }
                self.statements.insert(id.clone(), Statement {
                    id, proposal, author, decl, notes, gloss,
                    certificates: vec![], convergences: vec![],
                    implies: vec![], implied_by: vec![],
                    filed_seq: seq, sealed_seq: None, seal: None,
                    source: String::new(),
                });
            }
            Event::OpenRound { id, proposal, author, closes_at, reveal_by, note } => {
                if let Some(p) = self.proposals.get_mut(&proposal) {
                    p.rounds.push(id.clone());
                }
                self.rounds.insert(id.clone(), Round {
                    id, proposal, author, closes_at, reveal_by, note, opened_ts: ts,
                });
            }
            Event::SealStatement { id, proposal, author, commitment } => {
                if let Some(p) = self.proposals.get_mut(&proposal) {
                    p.seals.push(id.clone());
                }
                self.seals.insert(id.clone(), Seal {
                    id, proposal, author, commitment, seq, ts, statement: None,
                });
            }
            Event::RevealStatement { seal, statement, decl, gloss, notes, .. } => {
                // The statement inherits the seal's proposal and author: the
                // sealed commitment is the priority claim, and the CLI
                // checked the revealed file against it before appending.
                let Some(s) = self.seals.get_mut(&seal) else { return };
                let (proposal, author, sealed_seq) = (s.proposal.clone(), s.author.clone(), s.seq);
                s.statement = Some(statement.clone());
                if let Some(p) = self.proposals.get_mut(&proposal) {
                    p.statements.push(statement.clone());
                }
                self.statements.insert(statement.clone(), Statement {
                    id: statement, proposal, author, decl, notes, gloss,
                    certificates: vec![], convergences: vec![],
                    implies: vec![], implied_by: vec![],
                    filed_seq: seq, sealed_seq: Some(sealed_seq), seal: Some(seal),
                    source: String::new(),
                });
            }
            Event::Certify { statement, kind, decl, notes } => {
                if let Some(st) = self.statements.get_mut(&statement) {
                    st.certificates.push((kind, decl, notes));
                }
            }
            Event::Converge { a, b, decl } => {
                if let Some(st) = self.statements.get_mut(&a) {
                    st.convergences.push((b.clone(), decl.clone()));
                }
                if let Some(st) = self.statements.get_mut(&b) {
                    st.convergences.push((a, decl));
                }
            }
            Event::Implies { a, b, decl } => {
                if let Some(st) = self.statements.get_mut(&a) {
                    st.implies.push((b.clone(), decl.clone()));
                }
                if let Some(st) = self.statements.get_mut(&b) {
                    st.implied_by.push((a, decl));
                }
            }
            Event::RegisterSorry { id, title, statement, lean_type, allowed_axioms, proposal, env, bridge, author } => {
                self.sorries.insert(id.clone(), Sorry {
                    id, title, statement, lean_type, allowed_axioms, proposal, env, bridge,
                    registered_by: author,
                    status: "open".into(), solved_by: None, repins: vec![],
                    fidelity: Fidelity::default(), upstreamed: None, superseded_by: vec![],
                    zk_routes: vec![], zk_submissions: vec![],
                    lean_source: vec![], mathlib_names: vec![],
                    submissions: vec![], splits: vec![], part_of: vec![],
                    pool: 0,
                });
            }
            Event::Split { id, parent, author, children, glue, note } => {
                self.splits.push(SplitRec { id, parent, author, children, glue, note });
            }
            Event::Submit { id, sorry, solver, decl, module } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.submissions.push(Submission {
                        id, sorry: h.id.clone(), solver, decl, verdict: None,
                        verdict_seq: None, log_hash: None,
                        commitment: None, module, revealed: true,
                    });
                }
            }
            Event::Repin { sorry, lean_type, equiv_decl, .. } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    let old = std::mem::replace(&mut h.lean_type, lean_type.clone());
                    h.repins.push((old, lean_type, equiv_decl));
                }
            }
            Event::Commit { id, sorry, solver, commitment } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.submissions.push(Submission {
                        id, sorry: h.id.clone(), solver, decl: String::new(), verdict: None,
                        verdict_seq: None, log_hash: None,
                        commitment: Some(commitment), module: None, revealed: false,
                    });
                }
            }
            Event::Reveal { submission, decl, module } => {
                for h in self.sorries.values_mut() {
                    if let Some(sub) = h.submissions.iter_mut().find(|s| s.id == submission) {
                        sub.decl = decl.clone();
                        sub.module = Some(module.clone());
                        sub.revealed = true;
                    }
                }
            }
            Event::Verdict { submission, admitted, axioms, detail, .. } => {
                for h in self.sorries.values_mut() {
                    if let Some(sub) = h.submissions.iter_mut().find(|s| s.id == submission) {
                        sub.verdict = Some((admitted, axioms.clone(), detail.clone()));
                        if admitted && h.status == "open" {
                            h.status = "solved".into();
                            h.solved_by = Some(submission.clone());
                        }
                    }
                }
                for c in self.challenges.values_mut() {
                    if let Some(en) = c.entries.iter_mut().find(|e| {
                        e.refinement_sorry.as_deref() == Some(submission.as_str()) || e.id == submission
                    }) {
                        en.admitted = admitted;
                    }
                }
                for h in self.sorries.values_mut() {
                    if let Some(sub) = h.zk_submissions.iter_mut().find(|s| s.id == submission) {
                        sub.verdict = Some((admitted, detail.clone()));
                        if admitted && h.status == "open" {
                            h.status = "solved".into();
                            h.solved_by = Some(submission.clone());
                        }
                    }
                }
            }
            Event::Upstream { sorry, pr_url, .. } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.upstreamed = Some(pr_url);
                }
            }
            Event::Supersede { sorry, by, replacement, note } => {
                self.supersessions.push((by.clone(), sorry.clone(), replacement.clone(), note.clone()));
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.superseded_by.push((by, replacement, note));
                }
            }
            Event::RegisterChallenge { id, title, spec_impl, obligation, seed, iters } => {
                self.challenges.insert(id.clone(), Challenge {
                    id, title, spec_impl, obligation,
                    workload: seed.zip(iters),
                    entries: vec![], pool: 0,
                    arch_pools: BTreeMap::new(),
                });
            }
            Event::PinWorkload { challenge, seed, iters, .. } => {
                if let Some(c) = self.challenges.get_mut(&challenge) {
                    // First pin wins; a second pin event is invalid and,
                    // if one ever slipped onto the log, changes nothing.
                    if c.workload.is_none() {
                        c.workload = Some((seed, iters));
                    }
                }
            }
            Event::ZkRoute { id, sorry, vk_path, vk_hash, constraints, bridge_kind, bridge, note } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.zk_routes.push(ZkRouteRec {
                        id, vk_path, vk_hash, constraints, bridge_kind, bridge, note,
                    });
                }
            }
            Event::ZkSubmit { id, sorry, route, solver, public, proof } => {
                if let Some(h) = self.sorries.get_mut(&sorry) {
                    h.zk_submissions.push(ZkSubmission {
                        id, route, solver, public,
                        proof_prefix: proof.chars().take(24).collect(),
                        verdict: None,
                    });
                }
            }
            Event::RegisterRig { id, owner, arch, tier, note, runner } => {
                self.rigs.insert(id.clone(), Rig { id, owner, arch, tier, note, runner });
            }
            Event::RegisterAccount { handle, display, about, sigil, pubkey, github } => {
                self.accounts.insert(handle.clone(), Account { handle, display, about, sigil, pubkey, github });
            }
            Event::AnvilSubmit { id, challenge, impl_name, solver, proof_decl, refinement_sorry } => {
                if let Some(c) = self.challenges.get_mut(&challenge) {
                    let is_reference = impl_name == c.spec_impl;
                    c.entries.push(AnvilEntry {
                        id, challenge: c.id.clone(), impl_name, solver, proof_decl,
                        refinement_sorry, admitted: is_reference, is_reference,
                        scores: vec![],
                    });
                }
            }
            Event::Bench { submission, tier, arch, score, unit, checksum, rig, seed, iters } => {
                for c in self.challenges.values_mut() {
                    if let Some(en) = c.entries.iter_mut().find(|e| e.id == submission) {
                        // Only scores at the challenge's pinned workload
                        // rank: a score at any other workload (or with no
                        // recorded workload) is not comparable and stays
                        // on the log as history without entering the
                        // leaderboard.
                        if let Some((ps, pi)) = c.workload {
                            if seed != Some(ps) || iters != Some(pi) {
                                continue;
                            }
                        }
                        // A re-run replaces the earlier measurement for the
                        // same (tier, arch, rig) - the log keeps every run,
                        // the leaderboard shows one row per lane per rig.
                        en.scores.retain(|s| !(s.tier == tier && s.arch == arch && s.rig == rig));
                        en.scores.push(Score {
                            tier: tier.clone(), arch: arch.clone(), score,
                            unit: unit.clone(), checksum, rig: rig.clone(),
                            seed, iters,
                        });
                    }
                }
            }
            Event::Curate { curator, target, note } => {
                self.curations.push((curator, target, note));
            }
            Event::Tag { target, tag, by, note } => {
                self.tags.push((by, target, tag, note));
            }
            Event::Fund { target, amount, arch, .. } => {
                if let Some(h) = self.sorries.get_mut(&target) {
                    h.pool += amount;
                } else if let Some(c) = self.challenges.get_mut(&target) {
                    match arch {
                        Some(a) => *c.arch_pools.entry(a).or_insert(0) += amount,
                        None => c.pool += amount,
                    }
                }
            }
            Event::Payout { target, recipient, amount, reason } => {
                self.payouts.push((target, recipient, amount, reason));
            }
            Event::RecognizeCorpus { id, name, url, note, stats, source, as_of } => {
                self.corpora.insert(id.clone(), Corpus { id, name, url, note, stats, source, as_of });
            }
        }
    }

    /// Build the per-person view of the log. Every name that appears as a
    /// solver, author, endorser, funder, owner, or recipient gets a profile;
    /// registered accounts get their display name and sigil attached.
    pub fn aggregate_people(&mut self) {
        let mut people: BTreeMap<String, Person> = BTreeMap::new();
        let touch = |people: &mut BTreeMap<String, Person>, name: &str, seq: u64| {
            let p = people.entry(name.to_string()).or_insert_with(|| Person {
                handle: name.to_string(),
                ..Default::default()
            });
            if p.first_seen.is_none() {
                p.first_seen = Some(seq);
            }
        };

        // First pass: activity from the raw log (order preserved).
        for e in &self.events.clone() {
            let seq = e.seq;
            match &e.event {
                Event::Propose { id, author, .. } => {
                    touch(&mut people, author, seq);
                    people.get_mut(author).unwrap().proposals.push(id.clone());
                }
                Event::Formalize { id, author, .. } => {
                    touch(&mut people, author, seq);
                    people.get_mut(author).unwrap().statements.push(id.clone());
                }
                Event::OpenRound { author, .. } => {
                    touch(&mut people, author, seq);
                }
                Event::SealStatement { id, proposal, author, .. } => {
                    touch(&mut people, author, seq);
                    people.get_mut(author).unwrap().submissions.push(
                        (seq, id.clone(), proposal.clone(), "statement-seal".into(), "sealed".into()));
                }
                Event::RevealStatement { seal, statement, author, .. } => {
                    touch(&mut people, author, seq);
                    people.get_mut(author).unwrap().statements.push(statement.clone());
                    for p in people.values_mut() {
                        for s in p.submissions.iter_mut() {
                            if s.1 == *seal && s.3 == "statement-seal" {
                                s.4 = "revealed".into();
                            }
                        }
                    }
                }
                Event::Submit { id, sorry, solver, .. } => {
                    touch(&mut people, solver, seq);
                    people.get_mut(solver).unwrap().submissions.push(
                        (seq, id.clone(), sorry.clone(), "proof".into(), "pending".into()));
                }
                Event::Commit { id, sorry, solver, .. } => {
                    touch(&mut people, solver, seq);
                    people.get_mut(solver).unwrap().submissions.push(
                        (seq, id.clone(), sorry.clone(), "commit".into(), "sealed".into()));
                }
                Event::ZkSubmit { id, sorry, solver, .. } => {
                    touch(&mut people, solver, seq);
                    people.get_mut(solver).unwrap().submissions.push(
                        (seq, id.clone(), sorry.clone(), "zk".into(), "pending".into()));
                }
                Event::AnvilSubmit { id, challenge, solver, impl_name, .. } => {
                    touch(&mut people, solver, seq);
                    people.get_mut(solver).unwrap().submissions.push(
                        (seq, id.clone(), format!("{challenge} ({impl_name})"), "forge".into(), "pending".into()));
                }
                Event::Verdict { submission, admitted, .. } => {
                    for p in people.values_mut() {
                        for s in p.submissions.iter_mut() {
                            if s.1 == *submission {
                                // A re-verification supersedes the earlier
                                // verdict: each submission counts once, by
                                // its latest verdict.
                                match s.4.as_str() {
                                    "admitted" => p.solved = p.solved.saturating_sub(1),
                                    "rejected" => p.rejected = p.rejected.saturating_sub(1),
                                    _ => {}
                                }
                                s.4 = if *admitted { "admitted".into() } else { "rejected".into() };
                                if *admitted { p.solved += 1 } else { p.rejected += 1 }
                            }
                        }
                    }
                }
                Event::Reveal { submission, .. } => {
                    for p in people.values_mut() {
                        for s in p.submissions.iter_mut() {
                            if s.1 == *submission && s.4 == "sealed" {
                                s.4 = "pending".into();
                            }
                        }
                    }
                }
                Event::Fund { funder, amount, .. } => {
                    touch(&mut people, funder, seq);
                    people.get_mut(funder).unwrap().funded_total += amount;
                }
                Event::Payout { recipient, amount, .. } => {
                    touch(&mut people, recipient, seq);
                    people.get_mut(recipient).unwrap().payouts_total += amount;
                }
                Event::RegisterRig { id, owner, .. } => {
                    touch(&mut people, owner, seq);
                    people.get_mut(owner).unwrap().rigs.push(id.clone());
                }
                Event::Supersede { by, .. } => {
                    touch(&mut people, by, seq);
                }
                Event::Curate { curator, target, .. } => {
                    touch(&mut people, curator, seq);
                    people.get_mut(curator).unwrap().curated.push(target.clone());
                }
                _ => {}
            }
        }

        // Second pass: anvil lanes and current best-per-board standings.
        // (An anvil submission is admitted through its refinement sorry, so
        // reflect that in the person's submission outcome too.)
        for c in self.challenges.values() {
            for en in &c.entries {
                if en.admitted {
                    for p in people.values_mut() {
                        for s in p.submissions.iter_mut() {
                            if s.1 == en.id {
                                s.4 = "admitted".into();
                            }
                        }
                    }
                }
            }
        }
        for c in self.challenges.values() {
            // best score per board across the whole challenge
            let mut best: BTreeMap<String, f64> = BTreeMap::new();
            for en in &c.entries {
                for s in &en.scores {
                    let b = best.entry(format!("{}/{}", s.tier, s.arch)).or_insert(f64::MAX);
                    if s.score < *b { *b = s.score; }
                }
            }
            for en in &c.entries {
                let mut per_board: BTreeMap<String, (f64, String)> = BTreeMap::new();
                for s in &en.scores {
                    let key = format!("{}/{}", s.tier, s.arch);
                    let e = per_board.entry(key).or_insert((f64::MAX, s.unit.clone()));
                    if s.score < e.0 { *e = (s.score, s.unit.clone()); }
                }
                let solver = en.solver.clone();
                touch(&mut people, &solver, 0);
                let p = people.get_mut(&solver).unwrap();
                for (board, (score, unit)) in per_board {
                    // The reference baseline never "leads": it is the bar
                    // entries must clear, not a competitor. A board it
                    // still tops simply has no leader yet.
                    let leader = !en.is_reference
                        && best.get(&board).is_some_and(|b| (score - b).abs() < f64::EPSILON);
                    if leader { p.top_spots += 1; }
                    p.lanes.push((c.id.clone(), en.impl_name.clone(), board, score, unit, leader));
                }
            }
        }

        // Third pass: open sorries under proposals a person authored.
        for h in self.sorries.values() {
            if h.status != "open" { continue; }
            if let Some(prop) = &h.proposal {
                for p in people.values_mut() {
                    if p.proposals.contains(prop) {
                        p.open_sorries_authored.push(h.id.clone());
                    }
                }
            }
        }

        for (handle, acct) in &self.accounts {
            touch(&mut people, handle, 0);
            people.get_mut(handle).unwrap().account = Some(acct.clone());
        }
        self.people = people;
    }

    /// Attach every split to its parent sorry with the current status of
    /// each part, and mark children and glue sorries as serving in it. Call
    /// after fold, before export.
    pub fn aggregate_splits(&mut self) {
        let status_of = |sorries: &BTreeMap<String, Sorry>, id: &str| {
            sorries.get(id).map(|h| h.status.clone()).unwrap_or_else(|| "unknown".into())
        };
        for rec in self.splits.clone() {
            let children: Vec<(String, String)> = rec.children.iter()
                .map(|c| (c.clone(), status_of(&self.sorries, c)))
                .collect();
            let glue = (rec.glue.clone(), status_of(&self.sorries, &rec.glue));
            let solved_children = children.iter().filter(|(_, s)| s == "solved").count();
            let complete = glue.1 == "solved" && solved_children == children.len();
            for part in rec.children.iter().chain(std::iter::once(&rec.glue)) {
                if let Some(h) = self.sorries.get_mut(part) {
                    h.part_of.push(rec.id.clone());
                }
            }
            if let Some(h) = self.sorries.get_mut(&rec.parent) {
                h.splits.push(SplitView {
                    id: rec.id, author: rec.author, note: rec.note,
                    children, glue, solved_children, complete,
                });
            }
        }
    }

    /// Group each proposal's candidate statements into clumps: connected
    /// components under machine-checked equivalence. Call after fold.
    pub fn aggregate_clumps(&mut self) {
        // A solved bridge sorry is a kernel-checked equivalence of its two
        // statements: inject it as a convergence edge on both (the decl is
        // the admitted proof), so clumps merge exactly as a converge event
        // would - except this edge went through the verifier.
        let bridge_edges: Vec<(String, String, String)> = self.sorries.values()
            .filter(|h| h.status == "solved")
            .filter_map(|h| {
                let (a, b) = h.bridge.clone()?;
                let decl = h.solved_by.as_ref()
                    .and_then(|sid| h.submissions.iter().find(|s| &s.id == sid))
                    .map(|s| s.decl.clone())
                    .unwrap_or_else(|| format!("bridge {}", h.id));
                Some((a, b, decl))
            })
            .collect();
        for (a, b, decl) in bridge_edges {
            if let Some(st) = self.statements.get_mut(&a) {
                if !st.convergences.iter().any(|(o, _)| o == &b) {
                    st.convergences.push((b.clone(), decl.clone()));
                }
            }
            if let Some(st) = self.statements.get_mut(&b) {
                if !st.convergences.iter().any(|(o, _)| o == &a) {
                    st.convergences.push((a.clone(), decl));
                }
            }
        }
        for prop in self.proposals.values_mut() {
            let ids: Vec<String> = prop.statements.clone();
            if ids.is_empty() { continue; }
            let index: BTreeMap<&str, usize> =
                ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
            let mut parent: Vec<usize> = (0..ids.len()).collect();
            fn find(parent: &mut Vec<usize>, i: usize) -> usize {
                if parent[i] != i {
                    let r = find(parent, parent[i]);
                    parent[i] = r;
                }
                parent[i]
            }
            for (i, sid) in ids.iter().enumerate() {
                if let Some(st) = self.statements.get(sid) {
                    for (other, _) in &st.convergences {
                        if let Some(&j) = index.get(other.as_str()) {
                            let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                            if ri != rj { parent[ri] = rj; }
                        }
                    }
                }
            }
            let mut groups: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            for (i, sid) in ids.iter().enumerate() {
                let r = find(&mut parent, i);
                groups.entry(r).or_default().push(sid.clone());
            }
            let mut clumps: Vec<Clump> = groups.into_values().map(|members| {
                let authors: std::collections::BTreeSet<&str> = members.iter()
                    .filter_map(|m| self.statements.get(m).map(|s| s.author.as_str()))
                    .collect();
                let proven = self.sorries.values().any(|h|
                    h.status == "solved" && members.contains(&h.statement));
                let meta: Vec<(String, u64, u64)> = members.iter()
                    .filter_map(|m| self.statements.get(m))
                    .map(|s| (s.author.clone(), s.sealed_seq.unwrap_or(s.filed_seq), s.filed_seq))
                    .collect();
                let independent = max_mutually_blind(&meta);
                Clump { members, weight: authors.len(), dominant: false, proven, independent }
            }).collect();
            // dominant: unique heaviest clump with >= 2 independent members
            // (a singleton is never dominant)
            let best = clumps.iter().map(|c| c.weight).max().unwrap_or(0);
            let heaviest = clumps.iter().filter(|c| c.weight == best).count();
            if best >= 2 && heaviest == 1 {
                for c in clumps.iter_mut() {
                    if c.weight == best { c.dominant = true; }
                }
            }
            clumps.sort_by(|a, b| b.weight.cmp(&a.weight));
            prop.clumps = clumps;
        }
    }

    /// Fill each sorry's fidelity facts from its statement's equivalence
    /// clump. Call after `aggregate_clumps`.
    pub fn aggregate_fidelity(&mut self) {
        let mut per_sorry: Vec<(String, Fidelity)> = vec![];
        for h in self.sorries.values() {
            if h.statement.is_empty() {
                per_sorry.push((h.id.clone(), Fidelity { repins: h.repins.len(), ..Default::default() }));
                continue;
            }
            // The clump containing this sorry's statement, if the proposal
            // has been clumped.
            let clump = h.proposal.as_ref()
                .and_then(|p| self.proposals.get(p))
                .and_then(|p| p.clumps.iter().find(|c| c.members.contains(&h.statement)));
            let f = match clump {
                Some(c) => {
                    let certificates = c.members.iter()
                        .filter_map(|m| self.statements.get(m))
                        .map(|s| s.certificates.len())
                        .sum();
                    Fidelity {
                        authors: c.weight,
                        converged: c.weight >= 2,
                        independent: c.independent,
                        certificates,
                        repins: h.repins.len(),
                        canonical: false,
                    }
                }
                None => Fidelity {
                    authors: if self.statements.contains_key(&h.statement) { 1 } else { 0 },
                    converged: false,
                    independent: if self.statements.contains_key(&h.statement) { 1 } else { 0 },
                    certificates: self.statements.get(&h.statement)
                        .map(|s| s.certificates.len()).unwrap_or(0),
                    repins: h.repins.len(),
                    canonical: false,
                },
            };
            per_sorry.push((h.id.clone(), f));
        }
        for (id, f) in per_sorry {
            if let Some(h) = self.sorries.get_mut(&id) {
                h.fidelity = f;
            }
        }
    }

    /// Anvil submissions whose refinement sorry was solved get admitted.
    /// (Refinement proofs are verified through the ordinary sorry machinery.)
    pub fn settle_admissions(&mut self) {
        let solved: Vec<String> = self
            .sorries
            .values()
            .filter(|h| h.status == "solved")
            .map(|h| h.id.clone())
            .collect();
        for c in self.challenges.values_mut() {
            for en in c.entries.iter_mut() {
                if let Some(rh) = &en.refinement_sorry {
                    if solved.contains(rh) {
                        en.admitted = true;
                    }
                }
            }
        }
    }
}

/// The largest set of distinct-author statements that are pairwise
/// mutually blind - each one committed (sealed) before every other was
/// revealed, so neither author could have seen the other's Lean. Input is
/// (author, commit_seq, reveal_seq) per statement; an unsealed statement
/// has commit_seq == reveal_seq (its filing made it public instantly), so
/// two unsealed statements are never provably blind to each other.
///
/// Exact over the first 16 members - clumps are small, and truncation can
/// only undercount independence, never overstate it.
fn max_mutually_blind(members: &[(String, u64, u64)]) -> usize {
    let n = members.len().min(16);
    if n == 0 {
        return 0;
    }
    let blind = |x: &(String, u64, u64), y: &(String, u64, u64)| x.1 < y.2 && y.1 < x.2;
    let mut best = 1;
    for mask in 1u32..(1u32 << n) {
        let idx: Vec<usize> = (0..n).filter(|i| mask & (1 << i) != 0).collect();
        if idx.len() <= best {
            continue;
        }
        let mut authors = std::collections::BTreeSet::new();
        let ok = idx.iter().all(|&i| authors.insert(members[i].0.as_str()))
            && idx.iter().enumerate().all(|(k, &i)|
                idx[k + 1..].iter().all(|&j| blind(&members[i], &members[j])));
        if ok {
            best = idx.len();
        }
    }
    best
}

/// Decode the handful of HTML entities that show up in ingested catalogue
/// text. Unknown entities pass through unchanged.
fn html_unescape(s: &str) -> String {
    if !s.contains('&') { return s.to_string(); }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[..rest.len().min(12)].find(';');
        let Some(end) = end else { out.push('&'); rest = &rest[1..]; continue };
        let ent = &rest[1..end];
        let decoded = match ent {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'),
            "quot" => Some('"'), "apos" | "#39" => Some('\''),
            "ndash" => Some('\u{2013}'), "mdash" => Some('\u{2014}'),
            "nbsp" => Some(' '), "rsquo" => Some('\u{2019}'), "lsquo" => Some('\u{2018}'),
            _ => ent.strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| ent.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => { out.push(c); rest = &rest[end + 1..]; }
            None => { out.push('&'); rest = &rest[1..]; }
        }
    }
    out.push_str(rest);
    out
}
