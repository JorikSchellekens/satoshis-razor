//! ANV-001 champion submission: SWAR popcount.
//!
//! Branch-free, loop-free: pairwise bit sums, then nibble folding, then a
//! multiply to horizontally sum the bytes. Lean model: `Razor.Anvil.popSwar`;
//! admission proof: `Razor.Anvil.swar_refines` (settled by `bv_decide` - the
//! equivalence with the spec on all 2^64 inputs is a SAT instance, so no
//! human needed to trust this trick).

pub fn solve(x: u64) -> u64 {
    const M1: u64 = 0x5555_5555_5555_5555;
    const M2: u64 = 0x3333_3333_3333_3333;
    const M4: u64 = 0x0f0f_0f0f_0f0f_0f0f;
    const H01: u64 = 0x0101_0101_0101_0101;
    let a = x - ((x >> 1) & M1);
    let b = (a & M2) + ((a >> 2) & M2);
    let c = (b + (b >> 4)) & M4;
    c.wrapping_mul(H01) >> 56
}

anvil_abi::anvil_entry!(solve, |x| x);
