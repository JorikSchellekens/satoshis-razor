//! ANV-004 reference implementation: count leading zeros, one bit at a time.
//!
//! This is the executable specification - a straight transliteration of the
//! Lean model `Razor.Anvil.clzNaive`: examine the top bit, and either stop
//! or shift left and count, at most 64 times. The loop shape is part of the
//! spec so the Lean model matches instruction for instruction.
//!
//! The challenge's input mapper is `x >> (x & 63)`: shifting a random word
//! by a random amount spreads the answer over the whole 0..=64 range, so a
//! scan-from-the-top loop really pays for its worst cases. On raw random
//! words the top bit is set half the time and this loop would win by
//! exiting immediately - which would make the challenge about the input
//! distribution, not about counting zeros.

pub fn solve(x: u64) -> u64 {
    let mut x = x;
    let mut n: u64 = 0;
    let mut i = 0;
    while i < 64 {
        if x & 0x8000_0000_0000_0000 != 0 {
            return n;
        }
        n += 1;
        x <<= 1;
        i += 1;
    }
    n
}

anvil_abi::anvil_entry!(solve, |x| x >> (x & 63));
