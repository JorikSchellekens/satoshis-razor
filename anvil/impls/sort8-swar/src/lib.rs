//! ANV-003 contender: the Batcher network, one whole layer per compare.
//!
//! The scalar lanes unpack the u64 into 8 bytes and compare-exchange one
//! pair at a time. This lane never unpacks, and it prices instructions the
//! way the fuel-metered board does: one full-width per-byte comparison
//! (the borrow trick from Hacker's Delight) produces the greater-or-equal
//! flag for every byte lane at once, so a whole network layer - however
//! many comparators it holds - costs one compare plus one masked
//! writeback. Batcher's odd-even merge network needs 6 layers, so the
//! entire sort is 6 compares and 6 writebacks of 64-bit arithmetic.
//!
//! Per byte lane: t = (a | 0x80) - (b & 0x7f) leaves its top bit set
//! exactly when the low 7 bits of a reach those of b, and no lane can
//! borrow from its neighbor because the minuend is at least 128 and the
//! subtrahend at most 127. Folding in the top bits gives unsigned a >= b
//! per lane; multiplying the flag bits by 0xFF smears them into full
//! byte masks (the flag bytes are disjoint, so the multiply cannot
//! carry). The min/max selects and the write back to (lane, lane+d) are
//! plain masking.
//!
//! The Lean model is `Razor.Anvil.sortSwar`, a term-for-term
//! transliteration of the arithmetic below; `Razor.Anvil.swar_sort_refines`
//! checks it against the bubble-sort spec on all 2^64 inputs by SAT
//! (`bv_decide`). Nobody has to eyeball the borrow analysis - the solver
//! did.

const H: u64 = 0x8080_8080_8080_8080;

/// One Batcher layer: compare-exchange bytes (l, l+d) for every lane l in
/// `m` (0xFF at each comparator's low byte; `sh` = 8*d). One full-width
/// compare serves every comparator in the layer, and the exchange is an
/// xor-swap: d holds a^b exactly in the lanes that must trade places, and
/// xor-ing d into both lanes trades them (when a = b, d is zero and the
/// swap is a no-op, so taking "a >= b" as the swap condition is safe).
#[inline(always)]
fn layer(x: u64, m: u64, sh: u32) -> u64 {
    let b = x >> sh;
    // Per-byte unsigned x >= b, flag in each lane's top bit.
    let t = (x | H) - (b & !H);
    let ge = H & ((x & !b) | (!(x ^ b) & t));
    // Smear each flag into a byte mask (flag bytes are disjoint: exact),
    // keep only this layer's lanes, and swap via xor.
    let d = (x ^ b) & m & (ge >> 7).wrapping_mul(0xFF);
    x ^ d ^ (d << sh)
}

pub fn solve(x: u64) -> u64 {
    // Batcher's odd-even merge sort: sort both halves, then merge.
    let x = layer(x, 0x00FF_00FF_00FF_00FF, 8);  // (0,1)(2,3)(4,5)(6,7)
    let x = layer(x, 0x0000_FFFF_0000_FFFF, 16); // (0,2)(1,3)(4,6)(5,7)
    let x = layer(x, 0x0000_FF00_0000_FF00, 8);  // (1,2)(5,6)
    let x = layer(x, 0x0000_0000_FFFF_FFFF, 32); // (0,4)(1,5)(2,6)(3,7)
    let x = layer(x, 0x0000_0000_FFFF_0000, 16); // (2,4)(3,5)
    let x = layer(x, 0x0000_FF00_FF00_FF00, 8);  // (1,2)(3,4)(5,6)
    x
}

anvil_abi::anvil_entry!(solve, |x| x);
