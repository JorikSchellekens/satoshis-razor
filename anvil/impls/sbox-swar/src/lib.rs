//! ANV-009 contender: the AES S-box on all 8 bytes at once, in registers,
//! constant time.
//!
//! Every operation below is a shift, AND, OR or XOR on the whole 64-bit
//! word, treating it as 8 byte lanes: the GF(2^8) multiplier processes one
//! multiplier bit per step across all lanes simultaneously, the Fermat
//! chain squares and multiplies packed words, and the affine transform is
//! lane-wise rotate-xor. No table, no branch on data, no memory access -
//! the execution trace is identical for every input, which is the property
//! real AES implementations pay for to close cache-timing side channels.
//! This lane makes that price a number on the board.
//!
//! The Lean model is `Razor.Anvil.sboxSwar`; the admission proof checks it
//! against the from-the-definition reference on all 2^64 inputs.

const L: u64 = 0x0101_0101_0101_0101;
const SEVENF: u64 = 0x7F7F_7F7F_7F7F_7F7F;

/// Smear each lane's low bit across its whole byte: 0x01 -> 0xFF.
#[inline(always)]
fn smear(m: u64) -> u64 {
    let m = m | m << 1;
    let m = m | m << 2;
    m | m << 4
}

/// Lane-wise GF(2^8) multiply: one multiplier bit per step, all lanes at
/// once. The reducer xors x^4+x^3+x+1 (0x1B) into lanes whose top bit
/// fell off, built from shifts of the 0/1 lane mask.
#[inline(always)]
fn gmul64(mut a: u64, mut b: u64) -> u64 {
    let mut r = 0u64;
    for _ in 0..8 {
        r ^= a & smear(b & L);
        b = (b >> 1) & SEVENF;
        let hi = (a >> 7) & L;
        a = ((a & SEVENF) << 1) ^ (hi | hi << 1 | hi << 3 | hi << 4);
    }
    r
}

/// Lane-wise a^254: the GF(2^8) inverse (and 0 at 0), by the same addition
/// chain the scalar reference uses.
#[inline(always)]
fn ginv64(a: u64) -> u64 {
    let p2 = gmul64(a, a);
    let p3 = gmul64(p2, a);
    let p6 = gmul64(p3, p3);
    let p12 = gmul64(p6, p6);
    let p15 = gmul64(p12, p3);
    let p30 = gmul64(p15, p15);
    let p60 = gmul64(p30, p30);
    let p120 = gmul64(p60, p60);
    let p240 = gmul64(p120, p120);
    let p252 = gmul64(p240, p12);
    gmul64(p252, p2)
}

/// Lane-wise rotate-left by n within each byte.
#[inline(always)]
fn rot8(s: u64, n: u32) -> u64 {
    let keep = (0xFFu64 >> n).wrapping_mul(L);
    let low = ((1u64 << n) - 1).wrapping_mul(L);
    ((s & keep) << n) | ((s >> (8 - n)) & low)
}

pub fn solve(x: u64) -> u64 {
    let s = ginv64(x);
    s ^ rot8(s, 1) ^ rot8(s, 2) ^ rot8(s, 3) ^ rot8(s, 4) ^ 0x6363_6363_6363_6363
}

anvil_abi::anvil_entry!(solve, |x| x);
