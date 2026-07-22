//! ANV-005 reference implementation: reverse the bits of a u64, one at a time.
//!
//! This is the executable specification - a straight transliteration of the
//! Lean model `Razor.Anvil.revNaive`: 64 iterations, each moving one bit
//! from the bottom of the input to the bottom of the (shifting) result.

pub fn solve(x: u64) -> u64 {
    let mut x = x;
    let mut r: u64 = 0;
    let mut i = 0;
    while i < 64 {
        r = (r << 1) | (x & 1);
        x >>= 1;
        i += 1;
    }
    r
}

anvil_abi::anvil_entry!(solve, |x| x);
