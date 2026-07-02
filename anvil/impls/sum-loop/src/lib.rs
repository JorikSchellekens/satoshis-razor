//! ANV-002 reference implementation: O(n) accumulation loop.
//!
//! Executable spec, matching the Lean model `Razor.Anvil.sumLoopModel`.
//! Valid inputs are n < 2^32 (the harness mapper restricts further, to
//! n < 2^16, so the loop tier stays benchmarkable); no u64 wrapping occurs.

pub fn solve(n: u64) -> u64 {
    let mut sum: u64 = 0;
    let mut i: u64 = 1;
    while i <= n {
        sum += i;
        i += 1;
    }
    sum
}

anvil_abi::anvil_entry!(solve, |x| x & 0xffff);
