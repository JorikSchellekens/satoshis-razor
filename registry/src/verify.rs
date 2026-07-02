//! The verifier: the registry's only trusted component.
//!
//! A submission claims that declaration `decl` in the Lean package inhabits
//! the hole's pinned statement. Verification is two real checks against the
//! pinned toolchain:
//!
//! 1. Fidelity: an `example : <pinned type> := <decl>` must elaborate - the
//!    declaration's type must be (definitionally) the ratified statement,
//!    so a submission cannot smuggle in a different theorem under the name.
//! 2. Hygiene: `#print axioms <decl>` must report only allowed axioms.
//!    `sorryAx` (or any unlisted axiom) is a rejection.

use std::io::Write;
use std::process::Command;

pub struct Verdict {
    pub admitted: bool,
    pub axioms: Vec<String>,
    pub detail: String,
}

const BASE_AXIOMS: &[&str] = &["propext", "Classical.choice", "Quot.sound"];

pub fn verify(
    lean_dir: &std::path::Path,
    root_import: &str,
    lean_type: &str,
    decl: &str,
    allowed_extra: &[String],
    extra_module: Option<&str>,
) -> Verdict {
    let extra_import = extra_module.map(|m| format!("import {m}\n")).unwrap_or_default();
    let check = format!(
        "import {root_import}\n{extra_import}set_option maxRecDepth 4096 in\nexample : {lean_type} := @{decl}\n#print axioms {decl}\n"
    );
    let path = lean_dir.join(".razor-check.lean");
    let mut f = std::fs::File::create(&path).expect("write check file");
    f.write_all(check.as_bytes()).expect("write check file");

    // A submission is untrusted code: Lean elaboration can run arbitrary
    // programs before the kernel ever sees the proof. The check therefore
    // runs with network access denied (sandbox-exec on macOS, bubblewrap on
    // Linux) and under a hard timeout.
    let mut cmd = checker_command(lean_dir);
    cmd.current_dir(lean_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let timeout_s: u64 = std::env::var("RAZOR_VERIFY_TIMEOUT")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let mut child = cmd.spawn().expect("run checker");
    let start = std::time::Instant::now();
    let out = loop {
        match child.try_wait().expect("wait on checker") {
            Some(_) => break child.wait_with_output().expect("collect checker output"),
            None if start.elapsed().as_secs() >= timeout_s => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&path);
                return Verdict {
                    admitted: false,
                    axioms: vec![],
                    detail: format!("checker killed after the {timeout_s} s time limit"),
                };
            }
            None => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_file(&path);

    if !out.status.success() {
        return Verdict {
            admitted: false,
            axioms: vec![],
            detail: format!("statement check failed: {}", compact(&format!("{stdout}{stderr}"))),
        };
    }

    let axioms = parse_axioms(&stdout);
    let disallowed: Vec<&String> = axioms
        .iter()
        .filter(|a| {
            !BASE_AXIOMS.contains(&a.as_str()) && !allowed_extra.iter().any(|x| a.contains(x.as_str()))
        })
        .collect();

    if !disallowed.is_empty() {
        let names: Vec<String> = disallowed.iter().map(|s| s.to_string()).collect();
        return Verdict {
            admitted: false,
            axioms,
            detail: format!("disallowed axioms: {}", names.join(", ")),
        };
    }

    Verdict { admitted: true, axioms, detail: "statement matches pinned type; axioms clean".into() }
}

fn parse_axioms(stdout: &str) -> Vec<String> {
    // Formats: "'X' does not depend on any axioms" or
    // "'X' depends on axioms: [a, b, c]"
    if stdout.contains("does not depend on any axioms") {
        return vec![];
    }
    let Some(start) = stdout.find('[') else { return vec![] };
    let Some(end) = stdout[start..].find(']') else { return vec![] };
    stdout[start + 1..start + end]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn compact(s: &str) -> String {
    let s: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.len() > 400 { format!("{}…", &s[..400]) } else { s }
}

/// Build the `lake env lean .razor-check.lean` invocation inside the
/// lightest sandbox this platform offers, denying network access. Set
/// RAZOR_NO_SANDBOX=1 to opt out (e.g. when the process already runs
/// inside a container that is itself the sandbox).
fn checker_command(lean_dir: &std::path::Path) -> Command {
    if std::env::var_os("RAZOR_NO_SANDBOX").is_none() {
        if cfg!(target_os = "macos") && std::path::Path::new("/usr/bin/sandbox-exec").exists() {
            let mut c = Command::new("/usr/bin/sandbox-exec");
            c.arg("-p")
                .arg("(version 1)(allow default)(deny network*)")
                .args(["lake", "env", "lean", ".razor-check.lean"]);
            return c;
        }
        if cfg!(target_os = "linux") {
            let has_bwrap = Command::new("sh").args(["-c", "command -v bwrap"])
                .output().map(|o| o.status.success()).unwrap_or(false);
            if has_bwrap {
                let mut c = Command::new("bwrap");
                c.args(["--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc",
                        "--tmpfs", "/tmp", "--unshare-net", "--die-with-parent"])
                    .arg("--bind").arg(lean_dir).arg(lean_dir)
                    .args(["lake", "env", "lean", ".razor-check.lean"]);
                return c;
            }
        }
        eprintln!("  warning: no sandbox found (sandbox-exec / bwrap) - checker runs unsandboxed");
    }
    let mut c = Command::new("lake");
    c.args(["env", "lean", ".razor-check.lean"]);
    c
}
