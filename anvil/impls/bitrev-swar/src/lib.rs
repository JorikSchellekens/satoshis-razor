//! ANV-005 contender: reverse the bits of a u64 in six mask-and-shift layers.
//!
//! Swap adjacent bits, then adjacent pairs, then nibbles, bytes, byte
//! pairs, and finally the two 32-bit halves: 6 layers of straight-line
//! bit arithmetic instead of a 64-iteration loop. The same
//! divide-and-conquer trick as the SWAR popcount, and admitted the same
//! way: the Lean model is `Razor.Anvil.revSwar`, and
//! `Razor.Anvil.rev_swar_refines` checks agreement with the one-bit-at-a-
//! time spec on all 2^64 inputs by SAT (`bv_decide`).

pub fn solve(x: u64) -> u64 {
    let mut x = x;
    x = ((x >> 1) & 0x5555_5555_5555_5555) | ((x & 0x5555_5555_5555_5555) << 1);
    x = ((x >> 2) & 0x3333_3333_3333_3333) | ((x & 0x3333_3333_3333_3333) << 2);
    x = ((x >> 4) & 0x0f0f_0f0f_0f0f_0f0f) | ((x & 0x0f0f_0f0f_0f0f_0f0f) << 4);
    x = ((x >> 8) & 0x00ff_00ff_00ff_00ff) | ((x & 0x00ff_00ff_00ff_00ff) << 8);
    x = ((x >> 16) & 0x0000_ffff_0000_ffff) | ((x & 0x0000_ffff_0000_ffff) << 16);
    x = (x >> 32) | (x << 32);
    x
}

anvil_abi::anvil_entry!(solve, |x| x);
