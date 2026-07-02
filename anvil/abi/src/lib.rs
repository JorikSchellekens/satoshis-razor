//! Anvil challenge ABI.
//!
//! An implementation crate defines `solve(u64) -> u64` and invokes
//! `anvil_entry!(solve, <input mapper>)`. The macro generates the exports the
//! harness drives, identically for the wasm (Tier 1, fuel-metered) and native
//! (Tier 2, wall-clock) builds:
//!
//! - `bench(seed, iters) -> u64`: runs `solve` on `iters` deterministically
//!   generated inputs and returns a checksum. The input stream is a function
//!   of `seed` alone, so every implementation of a challenge sees the same
//!   inputs and must produce the same checksum.
//! - `solve_one(x) -> u64`: single call, used for differential testing
//!   against the challenge's executable spec.
//!
//! The input mapper is fixed per challenge (part of the pinned harness, not
//! of the submission): it projects the raw xorshift stream into the
//! challenge's valid input domain, mirroring the spec's validity predicate.

#[macro_export]
macro_rules! anvil_entry {
    ($solve:path, $map:expr) => {
        // The C ABI exports exist only in the wasm build; natively the
        // harness links the impl crates side by side and calls `solve`
        // directly, so unmangled symbols would collide.
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn bench(seed: u64, iters: u64) -> u64 {
            let map: fn(u64) -> u64 = $map;
            let mut state: u64 = seed | 1;
            let mut acc: u64 = 0;
            let mut i: u64 = 0;
            while i < iters {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                acc = acc.wrapping_add($solve(map(state)));
                i += 1;
            }
            acc
        }

        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn solve_one(x: u64) -> u64 {
            $solve(x)
        }
    };
}

/// Host-side twin of the generated `bench` export: same generator, same
/// checksum, so native scores and differential checks are comparable with
/// wasm runs bit for bit.
pub fn bench_host(solve: fn(u64) -> u64, map: fn(u64) -> u64, seed: u64, iters: u64) -> u64 {
    input_stream(seed, iters, map).fold(0u64, |acc, x| acc.wrapping_add(solve(x)))
}

/// The xorshift input stream, reproduced host-side so the harness can run
/// differential checks on exactly the inputs `bench` consumes.
pub fn input_stream(seed: u64, iters: u64, map: fn(u64) -> u64) -> impl Iterator<Item = u64> {
    let mut state = seed | 1;
    (0..iters).map(move |_| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        map(state)
    })
}
