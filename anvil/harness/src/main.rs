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
];

const IMPLS: &[Impl] = &[
    cpu("popcount-naive", "popcount", popcount_naive::solve),
    cpu("popcount-swar", "popcount", popcount_swar::solve),
    cpu("sum-loop", "sum", sum_loop::solve),
    cpu("sum-closed", "sum", sum_closed::solve),
    cpu("sort8-bubble", "sort8", sort8_bubble::solve),
    cpu("sort8-network", "sort8", sort8_network::solve),
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
];

fn challenge(name: &str) -> &'static Challenge {
    CHALLENGES.iter().find(|c| c.name == name).unwrap_or_else(|| {
        eprintln!("unknown challenge: {name}");
        std::process::exit(2);
    })
}

fn implementation(name: &str) -> &'static Impl {
    IMPLS.iter().find(|i| i.name == name).unwrap_or_else(|| {
        eprintln!("unknown impl: {name}");
        std::process::exit(2);
    })
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
            let imp = implementation(&name);
            let ch = challenge(imp.challenge);
            if !(imp.avail)() {
                println!("{{\"skip\":\"this lane needs hardware this machine does not have (no GPU adapter)\"}}");
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
            let imp = implementation(&name);
            let ch = challenge(imp.challenge);
            if !(imp.avail)() {
                println!("{{\"skip\":\"this lane needs hardware this machine does not have (no GPU adapter)\"}}");
                return;
            }
            let inputs: Vec<u64> = anvil_abi::input_stream(seed, iters, ch.map).collect();
            let outputs: Vec<u64> = match imp.many {
                Some(m) => m(&inputs),
                None => inputs.iter().map(|&x| (imp.solve)(x)).collect(),
            };
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
