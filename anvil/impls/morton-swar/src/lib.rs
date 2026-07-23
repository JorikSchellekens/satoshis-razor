//! ANV-008 contender: Morton interleave by five doubling spread steps per
//! half.
//!
//! Each step doubles the gap between the bits that are already placed:
//! 16-bit groups, then 8, 4, 2, 1. Ten masked shift-or steps total instead
//! of a 32-iteration bit loop.
//!
//! The Lean model is `Razor.Anvil.mortonSwar`; the admission proof checks
//! it against the one-bit-at-a-time reference on all 2^64 inputs by SAT.

#[inline(always)]
fn spread(v: u64) -> u64 {
    let v = v & 0x0000_0000_FFFF_FFFF;
    let v = (v | v << 16) & 0x0000_FFFF_0000_FFFF;
    let v = (v | v << 8) & 0x00FF_00FF_00FF_00FF;
    let v = (v | v << 4) & 0x0F0F_0F0F_0F0F_0F0F;
    let v = (v | v << 2) & 0x3333_3333_3333_3333;
    (v | v << 1) & 0x5555_5555_5555_5555
}

pub fn solve(x: u64) -> u64 {
    spread(x) | spread(x >> 32) << 1
}

anvil_abi::anvil_entry!(solve, |x| x);
