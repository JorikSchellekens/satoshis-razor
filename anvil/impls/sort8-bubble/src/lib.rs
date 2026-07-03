//! ANV-003 reference implementation: bubble sort over the 8 bytes of a u64.
//!
//! This is the executable specification. The input word is read as 8 bytes
//! (least significant byte = position 0); the output is the same bytes in
//! ascending order, packed back the same way. Bubble sort with fixed passes
//! is chosen as the spec because its shape is data-independent: it is exactly
//! a sequence of 28 compare-swaps of adjacent positions, which the Lean model
//! `Razor.Anvil.sortBubble` mirrors comparator for comparator.

pub fn solve(x: u64) -> u64 {
    let mut b = x.to_le_bytes();
    let mut i = 0;
    while i < 7 {
        let mut j = 0;
        while j < 7 - i {
            if b[j] > b[j + 1] {
                b.swap(j, j + 1);
            }
            j += 1;
        }
        i += 1;
    }
    u64::from_le_bytes(b)
}

anvil_abi::anvil_entry!(solve, |x| x);
