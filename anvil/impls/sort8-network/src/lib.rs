//! ANV-003 champion submission: a 19-comparator sorting network.
//!
//! Sorts the 8 bytes of a u64 with the size-optimal sorting network for 8
//! inputs (19 comparators - proven minimal). Each comparator is a branch-free
//! min/max pair, so the whole sort is straight-line code with no
//! data-dependent branches: nothing for the branch predictor to miss.
//!
//! Why should anyone believe 19 comparators in this exact order sort every
//! input? Nobody has to: the Lean model is `Razor.Anvil.sortNetwork`, and the
//! admission proof `Razor.Anvil.network_refines` checks agreement with the
//! bubble-sort spec on all 2^64 inputs by SAT (`bv_decide`) - the same route
//! that admitted the SWAR popcount trick.

#[inline(always)]
fn cswap(b: &mut [u8; 8], i: usize, j: usize) {
    let (lo, hi) = if b[i] <= b[j] { (b[i], b[j]) } else { (b[j], b[i]) };
    b[i] = lo;
    b[j] = hi;
}

pub fn solve(x: u64) -> u64 {
    let mut b = x.to_le_bytes();
    // Layer 1
    cswap(&mut b, 0, 1); cswap(&mut b, 2, 3); cswap(&mut b, 4, 5); cswap(&mut b, 6, 7);
    // Layer 2
    cswap(&mut b, 0, 2); cswap(&mut b, 1, 3); cswap(&mut b, 4, 6); cswap(&mut b, 5, 7);
    // Layer 3
    cswap(&mut b, 1, 2); cswap(&mut b, 5, 6); cswap(&mut b, 0, 4); cswap(&mut b, 3, 7);
    // Layer 4
    cswap(&mut b, 1, 5); cswap(&mut b, 2, 6);
    // Layer 5
    cswap(&mut b, 1, 4); cswap(&mut b, 3, 6);
    // Layer 6
    cswap(&mut b, 2, 4); cswap(&mut b, 3, 5);
    // Layer 7
    cswap(&mut b, 3, 4);
    u64::from_le_bytes(b)
}

anvil_abi::anvil_entry!(solve, |x| x);
