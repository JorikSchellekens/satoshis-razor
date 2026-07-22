//! ANV-004 contender: count leading zeros by binary search.
//!
//! Six halving steps instead of up to 64 single-bit steps: check whether
//! the top half is empty, and if so add its width to the count and shift
//! the bottom half up. Every input takes exactly six comparisons, so the
//! running time does not depend on the answer - no branch predictor pain
//! on the challenge's spread-out input distribution.
//!
//! The Lean model is `Razor.Anvil.clzBinary`; the admission proof
//! `Razor.Anvil.clz_binary_refines` checks agreement with the naive scan on
//! all 2^64 inputs by SAT (`bv_decide`).

pub fn solve(x: u64) -> u64 {
    if x == 0 {
        return 64;
    }
    let mut x = x;
    let mut n: u64 = 0;
    if x >> 32 == 0 { n += 32; x <<= 32; }
    if x >> 48 == 0 { n += 16; x <<= 16; }
    if x >> 56 == 0 { n += 8;  x <<= 8; }
    if x >> 60 == 0 { n += 4;  x <<= 4; }
    if x >> 62 == 0 { n += 2;  x <<= 2; }
    if x >> 63 == 0 { n += 1; }
    n
}

anvil_abi::anvil_entry!(solve, |x| x >> (x & 63));
