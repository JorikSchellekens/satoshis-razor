//! `razor` - the registry CLI.
//!
//! Every funnel transition is a subcommand appending an event to the log;
//! `verify` and `bench` are the two that do real work (Lean checking, fuel
//! metering) before writing their events. `export` emits the derived state
//! for the site.

mod model;
mod ui;
mod verify;

use model::{Entry, Event, State};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let root = repo_root();
    let log_path = root.join("registry/data/events.jsonl");
    std::fs::create_dir_all(log_path.parent().unwrap()).ok();

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
        "hole" => append(&log_path, Event::RegisterHole {
            id: req(&args, "--id"), title: req(&args, "--title"),
            statement: opt(&args, "--statement").unwrap_or_default(),
            lean_type: req(&args, "--lean-type"),
            allowed_axioms: multi(&args, "--allow-axiom"),
            proposal: opt(&args, "--proposal"),
            env: opt(&args, "--env"),
        }),
        "split" => cmd_split(&log_path, &args),
        "submit" => append(&log_path, Event::Submit {
            id: req(&args, "--id"), hole: req(&args, "--hole"),
            solver: req(&args, "--solver"), decl: req(&args, "--decl"),
        }),
        "supersede" => append(&log_path, Event::Supersede {
            hole: req(&args, "--hole"), by: req(&args, "--by"),
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
        "fund" => append(&log_path, Event::Fund {
            target: req(&args, "--target"),
            amount: req(&args, "--amount").parse().expect("--amount"),
            funder: req(&args, "--funder"),
            arch: opt(&args, "--arch"),
        }),
        "rig" => append(&log_path, Event::RegisterRig {
            id: req(&args, "--id"), owner: req(&args, "--owner"),
            arch: req(&args, "--arch"), tier: req(&args, "--tier"),
            note: opt(&args, "--note").unwrap_or_default(),
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
            ("implies", "prove one statement implies another"),
            ("hole", "pin an exact Lean statement as a solvable hole"),
            ("split", "reduce a hole to children plus a glue hole"),
            ("supersede", "mark a hole superseded by a better wording"),
        ]),
        ("solving", &[
            ("submit", "claim a hole with a proof declaration"),
            ("verify", "kernel-check a submission against the pinned type"),
        ]),
        ("value", &[
            ("curate", "a public, attributed pick - weighted by your admitted work"),
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
            ("rig", "register hardware you bring to the boards"),
        ]),
        ("people", &[
            ("account new", "claim a handle; generates your signing key"),
            ("account list", "everyone with a registered account"),
            ("profile <handle>", "one person's record, from the log alone"),
        ]),
        ("reading + auditing", &[
            ("status", "the whole registry, folded from the log"),
            ("log", "the raw event log, one JSON object per line"),
            ("corpus", "recognize an external verified corpus (e.g. Mathlib)"),
            ("export", "write site/data.json for the explorer"),
            ("serve", "host the site; data.json re-derived per request"),
            ("verify-log", "audit every event signature against registered keys"),
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
    use std::io::{BufRead, Write};
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
            let state = State::fold(load(log_path));
            let handle = loop {
                let h = opt(args, "--handle").unwrap_or_else(|| ask("handle (lowercase, dashes ok):", None));
                if h.is_empty() || !h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                    eprintln!("  {} handles are lowercase letters, digits, and dashes", ui::red("✕"));
                    if opt(args, "--handle").is_some() { std::process::exit(2); }
                } else if state.accounts.contains_key(&h) {
                    eprintln!("  {} '{h}' is taken", ui::red("✕"));
                    if opt(args, "--handle").is_some() { std::process::exit(2); }
                } else {
                    break h;
                }
            };
            let display = opt(args, "--display").unwrap_or_else(|| ask("display name:", Some(&handle)));
            let about = opt(args, "--about").unwrap_or_else(|| ask("one line about you:", Some("")));
            let display = if display.is_empty() { handle.clone() } else { display };

            // A real Ed25519 keypair: the signing key stays local, the
            // verifying key goes on the log as the account's pubkey. Every
            // later event by this handle is signed, so a registered handle
            // cannot be impersonated (razor verify-log checks the chain).
            let sk = generate_signing_key();
            let pubkey = hex_of(sk.verifying_key().as_bytes());
            let keydir = root.join("registry/data/keys");
            std::fs::create_dir_all(&keydir).ok();
            let keyfile = keydir.join(format!("{handle}.secret"));
            std::fs::write(&keyfile, hex_of(&sk.to_bytes())).expect("write key");

            let sigil = sigil_of(&handle);
            append(log_path, Event::RegisterAccount {
                handle: handle.clone(), display: display.clone(), about,
                sigil: sigil.into(), pubkey,
            });
            let key = keyfile.display().to_string();
            ui::card(&[
                (format!("{sigil}  {display} (@{handle})"),
                 format!("{sigil}  {} {}", ui::bold(&display), ui::dim(&format!("(@{handle})")))),
                ("welcome to the frontier.".into(), "welcome to the frontier.".into()),
                (format!("key   {key}"), format!("{}   {}", ui::dim("key"), ui::dim(&key))),
                (format!("next  razor profile {handle}"),
                 format!("{}  razor profile {handle}", ui::dim("next"))),
            ]);
        }
        Some("list") => {
            let state = State::fold(load(log_path));
            let w = state.accounts.values().map(|a| a.handle.chars().count()).max().unwrap_or(0);
            for a in state.accounts.values() {
                println!("  {}  {}  {}  {}",
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

fn cmd_verify(root: &PathBuf, log_path: &PathBuf, submission: &str) {
    let state = State::fold(load(log_path));
    let (hole, sub) = state
        .holes
        .values()
        .find_map(|h| h.submissions.iter().find(|s| s.id == submission).map(|s| (h, s)))
        .unwrap_or_else(|| {
            ui::die(&format!("unknown submission: {submission}"));
        });
    if !sub.revealed {
        ui::die(&format!("{} is committed but not yet revealed - nothing to verify", sub.id));
    }
    ui::step(&format!("verifying {} {} {}", ui::bold(&sub.id), ui::dim("against"), ui::cyan(&hole.id)));
    ui::kv("claims", &sub.decl);
    ui::kv("pinned", &hole.lean_type);
    // Pick the verification environment the hole was registered with.
    let (lean_dir, root_import) = match hole.env.as_deref() {
        Some("mathlib") => (root.join("lean-mathlib"), "RazorMathlib"),
        _ => (root.join("lean"), "Razor"),
    };
    if root_import == "RazorMathlib" && !lean_dir.join(".lake/packages/mathlib").exists() {
        eprintln!("{} this hole verifies in the Mathlib environment, which has not been fetched yet.", ui::red("✕"));
        eprintln!("  {}", ui::dim("run ./mathlib-env.sh once (several GB of prebuilt cache), then retry."));
        std::process::exit(2);
    }
    let t0 = std::time::Instant::now();
    let v = verify::verify(&lean_dir, root_import, &hole.lean_type, &sub.decl, &hole.allowed_axioms, sub.module.as_deref());
    let cost_ms = t0.elapsed().as_millis() as u64;
    ui::kv("axioms", &if v.axioms.is_empty() { ui::dim("none") } else { v.axioms.join(", ") });
    ui::kv("kernel", &format!("{cost_ms} ms"));
    ui::verdict(v.admitted, if v.admitted { "" } else { &v.detail });
    let recipient = sub.solver.clone();
    let hole_id = hole.id.clone();
    let pool = hole.pool;
    let already_solved = hole.status == "solved";
    append(log_path, Event::Verdict {
        submission: submission.into(),
        admitted: v.admitted,
        axioms: v.axioms,
        detail: v.detail,
        cost_ms,
    });
    // A bounty pays for the literal statement, first admitted proof, no
    // adjudication - the funder took the fidelity risk when they funded it.
    if v.admitted && !already_solved && pool > 0 {
        append(log_path, Event::Payout {
            target: hole_id.clone(),
            recipient,
            amount: pool,
            reason: format!("first admitted proof of {hole_id}, exactly as pinned"),
        });
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
    let harness = root.join("target/release/anvil-harness");
    for entry in ch.entries.iter().filter(|e| e.admitted) {
        let wasm = root.join(format!(
            "target/wasm32-unknown-unknown/release/{}.wasm",
            entry.impl_name.replace('-', "_")
        ));
        // Differential certificate first: an impl that disagrees with the
        // executable spec never gets a score (belt and braces on top of the proof).
        let check = run_json(&harness, &["check", "--impl", &entry.impl_name, "--seed", &seed.to_string(), "--iters", &iters.to_string()]);
        if check.get("pass") != Some(&serde_json::Value::Bool(true)) {
            eprintln!("{} differential check FAILED for {}, skipping", ui::red("✕"), entry.impl_name);
            continue;
        }
        let run_tier1 = rig.as_ref().is_none_or(|r| r.tier == "wasm-fuel");
        let run_native = rig.as_ref().is_none_or(|r| r.tier == "native");
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
            let tn = run_json(&harness, &["native", "--impl", &entry.impl_name, "--seed", &seed.to_string(), "--iters", &iters.to_string()]);
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
    state.aggregate_splits();
    state.aggregate_people();
    let arrow = ui::dim("→");
    ui::section("proposals", Some(state.proposals.len()));
    for p in state.proposals.values() {
        println!("  {}  {}  {}", ui::cyan(&format!("{:<9}", p.id)), p.title,
            ui::dim(&format!("[{} statements]", p.statements.len())));
        for c in &p.clumps {
            let tag = match (c.dominant, c.proven) {
                (true, true) => format!("{} {}", ui::gold("◆ dominant"), ui::green("· proven")),
                (true, false) => ui::gold("◆ dominant"),
                (false, true) => format!("{} {}", ui::dim("◇ clump"), ui::green("· proven")),
                (false, false) => ui::dim("◇ clump"),
            };
            println!("             {tag}  {}  {}",
                ui::dim(&format!("weight {}", c.weight)), c.members.join(&format!(" {} ", ui::dim("≡"))));
        }
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
    state.aggregate_splits();
    state.aggregate_people();
    // Attach the Lean source of each hole's pinned definitions, so the site
    // shows what a name unfolds to instead of just the name.
    let index = lean_decl_index(&repo_root());
    for h in state.holes.values_mut() {
        h.lean_source = resolve_lean_sources(&index, &h.lean_type);
    }
    let mut json = serde_json::to_value(&state).unwrap();
    // Label which dataset this export came from so the site can say so:
    // "demo" is the scripted walkthrough with fictional participants,
    // "live" is the real registry.
    json["dataset"] = serde_json::Value::String(dataset.into());
    (serde_json::to_string_pretty(&json).unwrap(), state.events.len())
}

fn cmd_export(log_path: &PathBuf, out: &PathBuf, dataset: Option<String>) {
    let (json, n) = export_string(log_path, dataset.as_deref().unwrap_or("live"));
    std::fs::create_dir_all(out.parent().unwrap()).ok();
    std::fs::write(out, &json).expect("write export");
    ui::step(&format!("exported {} events {} {}", ui::bold(&n.to_string()), ui::dim("→"), out.display()));
}

/// Serve the site with data.json re-derived from the log on demand, so the
/// pages update live as events are appended. Read-only: writes still go
/// through the CLI (and through it, the append-only log).
fn cmd_serve(root: &PathBuf, log_path: &PathBuf, host: &str, port: u16) {
    use std::io::{BufRead, BufReader, Write};
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
    let mut cache: Option<(u64, u64, String)> = None; // (len, mtime, json)
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut req_line = String::new();
        if reader.read_line(&mut req_line).is_err() {
            continue;
        }
        let path = req_line.split_whitespace().nth(1).unwrap_or("/");
        let path = path.split('?').next().unwrap_or("/");
        let (status, ctype, body): (&str, &str, Vec<u8>) = if path == "/data.json" {
            let meta = std::fs::metadata(log_path).ok();
            let key = meta
                .map(|m| (m.len(), m.modified().ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs()).unwrap_or(0)))
                .unwrap_or((0, 0));
            let fresh = match &cache {
                Some((l, t, _)) if (*l, *t) == key => false,
                _ => true,
            };
            if fresh {
                let (json, _) = export_string(log_path, &dataset);
                cache = Some((key.0, key.1, json));
            }
            ("200 OK", "application/json", cache.as_ref().unwrap().2.clone().into_bytes())
        } else {
            let rel = if path == "/" { "index.html" } else { path.trim_start_matches('/') };
            if rel.contains("..") {
                ("400 Bad Request", "text/plain", b"no".to_vec())
            } else {
                let file = root.join("site").join(rel);
                match std::fs::read(&file) {
                    Ok(bytes) => {
                        let ctype = match file.extension().and_then(|e| e.to_str()) {
                            Some("html") => "text/html; charset=utf-8",
                            Some("css") => "text/css",
                            Some("js") => "text/javascript",
                            Some("json") => "application/json",
                            Some("svg") => "image/svg+xml",
                            Some("sh") => "text/plain",
                            _ => "application/octet-stream",
                        };
                        ("200 OK", ctype, bytes)
                    }
                    Err(_) => ("404 Not Found", "text/plain", b"not found".to_vec()),
                }
            }
        };
        let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n", body.len());
        let _ = stream.write_all(&body);
    }
}

// ---------------- lean source index ----------------
// A pinned type like "Razor.Frontier.FLT" is a *name*; the reader must be
// able to see what it unfolds to. These functions build an index of every
// declaration in the Lean packages (fully qualified name -> source text,
// including the doc comment) and resolve a hole's pinned type to the
// definitions it mentions, transitively.

const LEAN_DECL_KEYWORDS: [&str; 6] = ["def ", "theorem ", "abbrev ", "structure ", "inductive ", "lemma "];

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
            ui::die("not inside the satoshis-razor repo (no lean/lakefile.toml found upward)");
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

fn append(path: &PathBuf, event: Event) {
    let entries = load(path);
    // Sign if the acting handle holds a local key; refuse to append in a
    // registered handle's name without its key. Handles that never
    // registered an account stay open and unsigned.
    let sig = match event.actor() {
        Some(actor) => {
            let actor = actor.to_string();
            let keyfile = path.parent().unwrap().join("keys").join(format!("{actor}.secret"));
            let registered = entries.iter().any(|e| matches!(&e.event,
                Event::RegisterAccount { handle, .. } if *handle == actor));
            match std::fs::read_to_string(&keyfile) {
                Ok(hex) => {
                    use ed25519_dalek::Signer;
                    let sk = signing_key_from_hex(hex.trim()).expect("malformed key file");
                    let msg = serde_json::to_string(&event).unwrap();
                    Some(hex_of(&sk.sign(msg.as_bytes()).to_bytes()))
                }
                Err(_) if registered => {
                    ui::die(&format!("'{actor}' is a registered handle and this machine has no key for it \
                        ({}) - refusing to append in their name", keyfile.display()));
                }
                Err(_) => None,
            }
        }
        None => None,
    };
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

fn run_json(bin: &PathBuf, args: &[&str]) -> serde_json::Value {
    let out = std::process::Command::new(bin).args(args).output().expect("run harness");
    if !out.status.success() && args[0] != "check" {
        eprintln!("{} harness failed: {}", ui::red("✕"), String::from_utf8_lossy(&out.stderr));
        std::process::exit(1);
    }
    serde_json::from_slice(&out.stdout).expect("harness json")
}

fn req(args: &[String], flag: &str) -> String {
    opt(args, flag).unwrap_or_else(|| {
        ui::die(&format!("missing {flag}"));
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
