//! ANV-003 contender: Batcher's odd-even merge network, depth 6.
//!
//! The reigning scalar champion (sort8-network) uses the size-optimal
//! 19-comparator network arranged in 7 dependent layers. Batcher's
//! odd-even merge sort for 8 inputs uses the same number of comparators -
//! 19 - but arranges them in 6 layers: sort each half with a 5-comparator
//! network (3 layers), then merge (3 layers). One less serial step means
//! a shorter dependency chain for a superscalar core to schedule around.
//! Same instruction count, less critical path: that is the whole bet.
//!
//! The Lean model is `Razor.Anvil.sortBatcher`; the admission proof
//! `Razor.Anvil.batcher_refines` checks agreement with the bubble-sort
//! spec on all 2^64 inputs by SAT (`bv_decide`) - nobody has to trust
//! that this comparator order sorts.

#[inline(always)]
fn cswap(b: &mut [u8; 8], i: usize, j: usize) {
    let (lo, hi) = if b[i] <= b[j] { (b[i], b[j]) } else { (b[j], b[i]) };
    b[i] = lo;
    b[j] = hi;
}

pub fn solve(x: u64) -> u64 {
    let mut b = x.to_le_bytes();
    // Sort both halves: two odd-even 4-sorters, side by side.
    cswap(&mut b, 0, 1); cswap(&mut b, 2, 3); cswap(&mut b, 4, 5); cswap(&mut b, 6, 7);
    cswap(&mut b, 0, 2); cswap(&mut b, 1, 3); cswap(&mut b, 4, 6); cswap(&mut b, 5, 7);
    cswap(&mut b, 1, 2); cswap(&mut b, 5, 6);
    // Odd-even merge of the sorted halves.
    cswap(&mut b, 0, 4); cswap(&mut b, 1, 5); cswap(&mut b, 2, 6); cswap(&mut b, 3, 7);
    cswap(&mut b, 2, 4); cswap(&mut b, 3, 5);
    cswap(&mut b, 1, 2); cswap(&mut b, 3, 4); cswap(&mut b, 5, 6);
    u64::from_le_bytes(b)
}

anvil_abi::anvil_entry!(solve, |x| x);
