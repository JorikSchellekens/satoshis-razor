//! ANV-001 reference implementation: naive shift-and-add popcount.
//!
//! This is the executable specification - a straight transliteration of the
//! Lean model `Razor.Anvil.popNaive` (64 fixed iterations; the loop shape is
//! part of the spec so the Lean model matches instruction for instruction).

pub fn solve(x: u64) -> u64 {
    let mut x = x;
    let mut count: u64 = 0;
    let mut i = 0;
    while i < 64 {
        count += x & 1;
        x >>= 1;
        i += 1;
    }
    count
}

anvil_abi::anvil_entry!(solve, |x| x);
