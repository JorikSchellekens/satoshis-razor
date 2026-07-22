//! Terminal presentation: one place for every color, glyph, and layout
//! device the CLI prints. All color goes through `paint`, which turns
//! itself off when stdout is not a terminal, when NO_COLOR is set, or when
//! TERM is dumb - so piped output stays clean. RAZOR_COLOR=always forces
//! color on (for pagers and captures).

use std::io::IsTerminal;
use std::sync::OnceLock;

pub const RULE_W: usize = 62;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        if std::env::var("RAZOR_COLOR").as_deref() == Ok("always") {
            return true;
        }
        std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
            && std::io::stdout().is_terminal()
    })
}

pub fn paint(code: &str, s: &str) -> String {
    if enabled() { format!("\x1b[{code}m{s}\x1b[0m") } else { s.to_string() }
}

pub fn bold(s: &str) -> String { paint("1", s) }
pub fn dim(s: &str) -> String { paint("2", s) }
pub fn accent(s: &str) -> String { paint("1;36", s) } // the chalk cyan of the site
pub fn cyan(s: &str) -> String { paint("36", s) }
pub fn green(s: &str) -> String { paint("32", s) }
pub fn red(s: &str) -> String { paint("31", s) }
pub fn gold(s: &str) -> String { paint("33", s) }

/// `1234567` → `1,234,567`.
pub fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A step marker for actions the CLI takes: `▸ message`.
pub fn step(msg: &str) {
    println!("{} {msg}", accent("▸"));
}

/// A section rule: `── title ───────────────── 7 ──`.
pub fn section(title: &str, count: Option<usize>) {
    let tail = count.map(|n| format!(" {n} ")).unwrap_or_default();
    let used = 3 + title.chars().count() + 1 + tail.chars().count() + 2;
    let fill = "─".repeat(RULE_W.saturating_sub(used));
    println!();
    println!("{} {} {}{}{}", dim("──"), bold(title), dim(&fill), dim(&tail), dim("──"));
}

/// Status chip for a sorry: `● open` / `✓ solved`.
pub fn chip(status: &str) -> String {
    match status {
        "solved" => green("✓ solved"),
        "open" => cyan("○ open  "),
        other => dim(&format!("{other:<8}")),
    }
}

/// A bounty pool, in gold: `⛀ 4,000`.
pub fn pool(amount: u64) -> String {
    gold(&format!("⛀ {}", commas(amount)))
}

/// Aligned key-value detail line under a step: `    key  value`.
pub fn kv(key: &str, value: &str) {
    println!("  {:>9}  {value}", dim(key));
}

/// The verdict bar - the one moment of real weight in the CLI.
pub fn verdict(admitted: bool, note: &str) {
    println!();
    if admitted {
        println!("  {} {}", green("┃"), green(&format!("ADMITTED{}{}", if note.is_empty() { "" } else { "  " }, dim(note))));
    } else {
        println!("  {} {}", red("┃"), red("REJECTED"));
        if !note.is_empty() {
            println!("  {} {}", red("┃"), dim(note));
        }
    }
    println!();
}

/// Fatal error: `✕ message` to stderr, exit 2.
pub fn die(msg: &str) -> ! {
    eprintln!("{} {msg}", red("✕"));
    std::process::exit(2);
}

/// A rounded box around lines (used for the account card). Width adapts to
/// the longest line; `line` values are (text, already-styled-text) pairs so
/// width is computed on the plain text.
pub fn card(lines: &[(String, String)]) {
    let w = lines.iter().map(|(plain, _)| plain.chars().count()).max().unwrap_or(0);
    println!();
    println!("  {}", accent(&format!("╭{}╮", "─".repeat(w + 4))));
    for (plain, styled) in lines {
        let pad = " ".repeat(w - plain.chars().count());
        println!("  {}  {styled}{pad}  {}", accent("│"), accent("│"));
    }
    println!("  {}", accent(&format!("╰{}╯", "─".repeat(w + 4))));
    println!();
}
