//! ANV-008 reference: interleave the two 32-bit halves of a u64, one bit
//! at a time.
//!
//! The output is the Morton (Z-order) code of the pair (low half, high
//! half): bit i of the low half lands at even position 2i, bit i of the
//! high half at odd position 2i+1. Z-order codes turn 2-D locality into
//! 1-D locality, which is why they index spatial databases and GPU
//! texture layouts.
//!
//! The Lean model is `Razor.Anvil.mortonNaive`.

pub fn solve(x: u64) -> u64 {
    let lo = x & 0xFFFF_FFFF;
    let hi = x >> 32;
    let mut out = 0u64;
    for i in 0..32 {
        out |= ((lo >> i) & 1) << (2 * i);
        out |= ((hi >> i) & 1) << (2 * i + 1);
    }
    out
}

anvil_abi::anvil_entry!(solve, |x| x);
