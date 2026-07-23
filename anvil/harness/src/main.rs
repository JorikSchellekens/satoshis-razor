//! Anvil benchmark harness.
//!
//! Tier 1 (`tier1`): runs an implementation's wasm build with wasmtime fuel
//! metering. Fuel is a pure function of (wasm binary, seed, iters) - fully
//! deterministic, so any score can be settled by re-execution.
//!
//! Tier 2 (`native`): times the implementation natively on this machine
//! (median of repeats). Reproducible only statistically - this is the tier
//! that needs attestation in the full design.
//!
//! `check`: differential test of an implementation against the challenge's
//! executable spec on the exact input stream `bench` consumes. This is the
//! challenge-window certificate, not the admission proof - admission is the
//! Lean refinement theorem.
//!
//! Output is one JSON object on stdout.
//!
//! Whole-stream lanes: an implementation may process the entire benchmark
//! input stream at once instead of one word per call - that is how a GPU
//! lane amortizes its dispatch cost. Such a lane provides `batch` (the
//! whole-stream timed entry, same checksum contract) and `many` (the
//! whole-stream differential entry); `avail` reports whether the lane can
//! run on this machine at all, so a GPU lane on a GPU-less box reports
//! "not measurable" instead of failing.

use std::time::Instant;

struct Challenge {
    name: &'static str,
    map: fn(u64) -> u64,
    reference: fn(u64) -> u64,
}

struct Impl {
    name: &'static str,
    challenge: &'static str,
    solve: fn(u64) -> u64,
    /// Whole-stream timed entry: (seed, iters) -> checksum. None = the
    /// harness drives `solve` one word at a time.
    batch: Option<fn(u64, u64) -> u64>,
    /// Whole-stream differential entry. None = map `solve` over the inputs.
    many: Option<fn(&[u64]) -> Vec<u64>>,
    /// Whether the lane can run on this machine (a GPU lane needs a GPU).
    avail: fn() -> bool,
}

const fn cpu(name: &'static str, challenge: &'static str, solve: fn(u64) -> u64) -> Impl {
    Impl { name, challenge, solve, batch: None, many: None, avail: || true }
}

const CHALLENGES: &[Challenge] = &[
    Challenge { name: "popcount", map: |x| x, reference: popcount_naive::solve },
    Challenge { name: "sum", map: |x| x & 0xffff, reference: sum_loop::solve },
    Challenge { name: "sort8", map: |x| x, reference: sort8_bubble::solve },
    Challenge { name: "clz", map: |x| x >> (x & 63), reference: clz_naive::solve },
    Challenge { name: "bitrev", map: |x| x, reference: bitrev_naive::solve },
    Challenge { name: "evm", map: |x| x, reference: evm_ref::solve },
    Challenge { name: "siphash13", map: |x| x, reference: siphash13_ref::solve },
    Challenge { name: "crc64", map: |x| x, reference: crc64_bitwise::solve },
    Challenge { name: "morton", map: |x| x, reference: morton_naive::solve },
    Challenge { name: "sbox", map: |x| x, reference: sbox_scalar::solve },
];

const IMPLS: &[Impl] = &[
    cpu("popcount-naive", "popcount", popcount_naive::solve),
    cpu("popcount-swar", "popcount", popcount_swar::solve),
    cpu("sum-loop", "sum", sum_loop::solve),
    cpu("sum-closed", "sum", sum_closed::solve),
    cpu("sort8-bubble", "sort8", sort8_bubble::solve),
    cpu("sort8-network", "sort8", sort8_network::solve),
    cpu("sort8-batcher", "sort8", sort8_batcher::solve),
    cpu("sort8-swar", "sort8", sort8_swar::solve),
    Impl {
        name: "sort8-simd",
        challenge: "sort8",
        solve: sort8_simd::solve,
        batch: Some(sort8_simd::bench_batch),
        many: Some(sort8_simd::solve_many),
        avail: || true,
    },
    Impl {
        name: "sort8-gpu",
        challenge: "sort8",
        solve: sort8_gpu::solve,
        batch: Some(sort8_gpu::bench_batch),
        many: Some(sort8_gpu::solve_many),
        avail: sort8_gpu::available,
    },
    cpu("clz-naive", "clz", clz_naive::solve),
    cpu("clz-branchless", "clz", clz_branchless::solve),
    cpu("bitrev-naive", "bitrev", bitrev_naive::solve),
    cpu("bitrev-swar", "bitrev", bitrev_swar::solve),
    cpu("evm-ref", "evm", evm_ref::solve),
    cpu("evm-tos", "evm", evm_tos::solve),
    cpu("siphash13-ref", "siphash13", siphash13_ref::solve),
    Impl {
        name: "siphash13-stream",
        challenge: "siphash13",
        solve: siphash13_stream::solve,
        batch: Some(siphash13_stream::bench_batch),
        many: Some(siphash13_stream::solve_many),
        avail: || true,
    },
    cpu("crc64-bitwise", "crc64", crc64_bitwise::solve),
    cpu("crc64-nibble", "crc64", crc64_nibble::solve),
    cpu("morton-naive", "morton", morton_naive::solve),
    cpu("morton-swar", "morton", morton_swar::solve),
    Impl {
        name: "morton-pdep",
        challenge: "morton",
        solve: morton_pdep::solve,
        batch: None,
        many: None,
        avail: morton_pdep::available,
    },
    cpu("sbox-scalar", "sbox", sbox_scalar::solve),
    cpu("sbox-table", "sbox", sbox_table::solve),
    cpu("sbox-swar", "sbox", sbox_swar::solve),
];

fn challenge(name: &str) -> &'static Challenge {
    CHALLENGES.iter().find(|c| c.name == name).unwrap_or_else(|| {
        eprintln!("unknown challenge: {name}");
        std::process::exit(2);
    })
}

fn implementation(name: &str) -> Option<&'static Impl> {
    IMPLS.iter().find(|i| i.name == name)
}

/// An external lane: an implementation that is not compiled into this
/// harness. It lives in `anvil/lanes/<name>/lane.json` and brings its own
/// artifacts - a native executable that speaks the harness's own protocol
/// (`native --seed S --iters I --repeats R` printing the score JSON, and
/// `many --seed S --iters I` writing one little-endian u64 output per
/// input word to stdout), and/or a wasm build with the standard
/// `bench`/`solve_one` exports. That is the whole contract, so a lane can
/// be written in any language; the differential check and the admission
/// proof gate it exactly like a built-in lane.
struct ExternalLane {
    challenge: String,
    native: Option<std::path::PathBuf>,
    wasm: Option<std::path::PathBuf>,
    arch: Option<String>,
}

impl ExternalLane {
    /// The native artifact, if it can run on this machine. A lane that
    /// declares an `arch` (hand-written assembly, ISA intrinsics) only
    /// offers its native build on that architecture; elsewhere it falls
    /// back to its wasm build or reports itself not measurable.
    fn native_here(&self) -> Option<&std::path::PathBuf> {
        match &self.arch {
            Some(a) if a != std::env::consts::ARCH => None,
            _ => self.native.as_ref(),
        }
    }
}

/// Where external lanes live: `anvil/lanes` under the current directory,
/// or under the repository the harness binary was built in (so a rig
/// runner invoked from an arbitrary working directory still finds them).
fn lanes_dir() -> Option<std::path::PathBuf> {
    let local = std::path::PathBuf::from("anvil/lanes");
    if local.is_dir() {
        return Some(local);
    }
    let exe = std::env::current_exe().ok()?;
    let repo = exe.parent()?.parent()?.parent()?;
    let d = repo.join("anvil/lanes");
    d.is_dir().then_some(d)
}

fn external(name: &str) -> Option<ExternalLane> {
    let dir = lanes_dir()?.join(name);
    let raw = std::fs::read_to_string(dir.join("lane.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let path_of = |key: &str| {
        v.get(key)
            .and_then(|p| p.as_str())
            .map(|p| dir.join(p))
            .filter(|p| p.exists())
    };
    Some(ExternalLane {
        challenge: v.get("challenge")?.as_str()?.to_string(),
        native: path_of("native"),
        wasm: path_of("wasm"),
        arch: v.get("arch").and_then(|a| a.as_str()).map(str::to_string),
    })
}

fn unknown_impl(name: &str) -> ! {
    eprintln!("unknown impl: {name} (not built in, and no anvil/lanes/{name}/lane.json)");
    std::process::exit(2);
}

/// Run an external lane's `many` entry (or its wasm `solve_one`) over the
/// input stream.
fn external_outputs(ext: &ExternalLane, inputs: &[u64], seed: u64, iters: u64) -> Vec<u64> {
    if let Some(exe) = ext.native_here() {
        let out = std::process::Command::new(exe)
            .args(["many", "--seed", &seed.to_string(), "--iters", &iters.to_string()])
            .output()
            .unwrap_or_else(|e| {
                eprintln!("external lane failed to run: {e}");
                std::process::exit(2);
            });
        if !out.status.success() {
            eprintln!("external lane 'many' failed: {}", String::from_utf8_lossy(&out.stderr));
            std::process::exit(2);
        }
        return out
            .stdout
            .chunks_exact(8)
            .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
            .collect();
    }
    if let Some(wasm) = &ext.wasm {
        return run_wasm_many(wasm.to_str().unwrap(), inputs);
    }
    eprintln!("external lane has neither a native executable nor a wasm build");
    std::process::exit(2);
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).map(|i| args.get(i + 1).cloned().unwrap_or_default())
}

fn arg_u64(args: &[String], flag: &str, default: u64) -> u64 {
    arg(args, flag).map(|v| v.parse().expect(flag)).unwrap_or(default)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let seed = arg_u64(&args, "--seed", 0xC0FFEE);
    let iters = arg_u64(&args, "--iters", 10_000);

    match cmd {
        "tier1" => {
            let wasm = arg(&args, "--wasm").expect("--wasm <path>");
            let (fuel, checksum) = run_wasm(&wasm, seed, iters);
            println!(
                "{{\"tier\":\"wasm-fuel\",\"wasm\":\"{wasm}\",\"seed\":{seed},\"iters\":{iters},\"fuel\":{fuel},\"fuel_per_op\":{:.2},\"checksum\":{checksum}}}",
                fuel as f64 / iters as f64
            );
        }
        "native" => {
            let name = arg(&args, "--impl").expect("--impl <name>");
            let Some(imp) = implementation(&name) else {
                // External lane: its executable speaks this same protocol,
                // so the score JSON is its own report, passed through.
                let Some(ext) = external(&name) else { unknown_impl(&name) };
                let Some(exe) = ext.native_here().cloned() else {
                    let why = match &ext.arch {
                        Some(a) if a != std::env::consts::ARCH => format!(
                            "this lane's native build targets {a}; this machine is {}",
                            std::env::consts::ARCH
                        ),
                        _ => "this lane has no native build for this machine".to_string(),
                    };
                    println!("{{\"skip\":\"{why}\"}}");
                    return;
                };
                let repeats = arg_u64(&args, "--repeats", 9);
                let out = std::process::Command::new(&exe)
                    .args(["native", "--seed", &seed.to_string(), "--iters", &iters.to_string(),
                           "--repeats", &repeats.to_string()])
                    .output()
                    .unwrap_or_else(|e| { eprintln!("external lane failed to run: {e}"); std::process::exit(2) });
                let text = String::from_utf8_lossy(&out.stdout);
                let line = text.lines().find(|l| l.trim_start().starts_with('{')).unwrap_or_else(|| {
                    eprintln!("external lane printed no JSON: {}", String::from_utf8_lossy(&out.stderr));
                    std::process::exit(2)
                });
                println!("{}", line.trim());
                return;
            };
            let ch = challenge(imp.challenge);
            if !(imp.avail)() {
                println!("{{\"skip\":\"this lane needs hardware this machine does not have (a GPU adapter, or an x86-64 CPU instruction)\"}}");
                return;
            }
            let repeats = arg_u64(&args, "--repeats", 9) as usize;
            let run = |_: usize| -> u64 {
                match imp.batch {
                    Some(b) => b(seed, iters),
                    None => anvil_abi::bench_host(imp.solve, ch.map, seed, iters),
                }
            };
            // Warm-up, then median of repeats.
            let checksum = run(0);
            let mut times: Vec<u128> = (0..repeats)
                .map(|r| {
                    let t = Instant::now();
                    let c = run(r);
                    assert_eq!(c, checksum);
                    t.elapsed().as_nanos()
                })
                .collect();
            times.sort();
            let median = times[times.len() / 2];
            println!(
                "{{\"tier\":\"native\",\"arch\":\"{}\",\"impl\":\"{name}\",\"seed\":{seed},\"iters\":{iters},\"ns\":{median},\"ns_per_op\":{:.2},\"checksum\":{checksum}}}",
                std::env::consts::ARCH,
                median as f64 / iters as f64
            );
        }
        "check" => {
            let name = arg(&args, "--impl").expect("--impl <name>");
            let (ch, outputs, inputs);
            match implementation(&name) {
                Some(imp) => {
                    ch = challenge(imp.challenge);
                    if !(imp.avail)() {
                        println!("{{\"skip\":\"this lane needs hardware this machine does not have (a GPU adapter, or an x86-64 CPU instruction)\"}}");
                        return;
                    }
                    inputs = anvil_abi::input_stream(seed, iters, ch.map).collect::<Vec<u64>>();
                    outputs = match imp.many {
                        Some(m) => m(&inputs),
                        None => inputs.iter().map(|&x| (imp.solve)(x)).collect(),
                    };
                }
                None => {
                    let Some(ext) = external(&name) else { unknown_impl(&name) };
                    if ext.native_here().is_none() && ext.wasm.is_none() {
                        println!("{{\"skip\":\"this lane cannot run on this machine (its native build is architecture-specific and it has no wasm build)\"}}");
                        return;
                    }
                    ch = challenge(&ext.challenge);
                    inputs = anvil_abi::input_stream(seed, iters, ch.map).collect::<Vec<u64>>();
                    outputs = external_outputs(&ext, &inputs, seed, iters);
                    if outputs.len() != inputs.len() {
                        eprintln!("external lane returned {} outputs for {} inputs", outputs.len(), inputs.len());
                        std::process::exit(2);
                    }
                }
            }
            let mut mismatches = 0u64;
            let mut first: Option<u64> = None;
            for (&x, &y) in inputs.iter().zip(outputs.iter()) {
                if y != (ch.reference)(x) {
                    mismatches += 1;
                    first.get_or_insert(x);
                }
            }
            let pass = mismatches == 0;
            let first = first.map(|x| x.to_string()).unwrap_or_else(|| "null".into());
            println!(
                "{{\"tier\":\"differential\",\"impl\":\"{name}\",\"challenge\":\"{}\",\"cases\":{iters},\"mismatches\":{mismatches},\"first_mismatch\":{first},\"pass\":{pass}}}",
                ch.name
            );
            if !pass {
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: anvil-harness <tier1|native|check> [--wasm P] [--impl N] [--seed S] [--iters I] [--repeats R]");
            std::process::exit(2);
        }
    }
}

/// Drive a wasm lane's `solve_one` export over explicit inputs - the
/// differential path for an external lane that ships only a wasm build.
fn run_wasm_many(path: &str, inputs: &[u64]) -> Vec<u64> {
    use wasmtime::{Engine, Instance, Module, Store};
    let engine = Engine::default();
    let module = Module::from_file(&engine, path).expect("load wasm");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");
    let solve_one = instance
        .get_typed_func::<u64, u64>(&mut store, "solve_one")
        .expect("solve_one export");
    inputs.iter().map(|&x| solve_one.call(&mut store, x).expect("solve_one call")).collect()
}

fn run_wasm(path: &str, seed: u64, iters: u64) -> (u64, u64) {
    use wasmtime::{Config, Engine, Instance, Module, Store};
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).expect("engine");
    let module = Module::from_file(&engine, path).expect("load wasm");
    let mut store = Store::new(&engine, ());
    store.set_fuel(u64::MAX).expect("set fuel");
    let instance = Instance::new(&mut store, &module, &[]).expect("instantiate");
    let bench = instance
        .get_typed_func::<(u64, u64), u64>(&mut store, "bench")
        .expect("bench export");
    let before = store.get_fuel().expect("fuel");
    let checksum = bench.call(&mut store, (seed, iters)).expect("bench call");
    let after = store.get_fuel().expect("fuel");
    (before - after, checksum)
}
