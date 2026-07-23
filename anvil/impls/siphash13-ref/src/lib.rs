//! ANV-006 reference: SipHash-1-3 of one 8-byte message, straight from the
//! paper.
//!
//! SipHash is the keyed hash that guards most hash tables in production
//! (Rust's hashbrown shipped SipHash-1-3 as its default for years); this
//! challenge fixes the key to the classic test key from the SipHash paper
//! (k = 00 01 02 ... 0f) and hashes exactly one 8-byte little-endian
//! message. The reference keeps the paper's shape: a compression-round loop
//! per message block, the length block, then the finalization-round loop.
//!
//! The Lean model is `Razor.Anvil.sip13Ref`; the round function and message
//! schedule were validated against the official SipHash-2-4 test vectors
//! (same round, different round counts) and cross-checked against the
//! `siphasher` crate in this crate's tests.

const K0: u64 = 0x0706050403020100;
const K1: u64 = 0x0F0E0D0C0B0A0908;
const C_ROUNDS: u32 = 1;
const D_ROUNDS: u32 = 3;

#[inline(always)]
fn round(v: &mut [u64; 4]) {
    v[0] = v[0].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(13);
    v[1] ^= v[0];
    v[0] = v[0].rotate_left(32);
    v[2] = v[2].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(16);
    v[3] ^= v[2];
    v[0] = v[0].wrapping_add(v[3]);
    v[3] = v[3].rotate_left(21);
    v[3] ^= v[0];
    v[2] = v[2].wrapping_add(v[1]);
    v[1] = v[1].rotate_left(17);
    v[1] ^= v[2];
    v[2] = v[2].rotate_left(32);
}

pub fn solve(x: u64) -> u64 {
    let mut v = [
        K0 ^ 0x736F6D6570736575,
        K1 ^ 0x646F72616E646F6D,
        K0 ^ 0x6C7967656E657261,
        K1 ^ 0x7465646279746573,
    ];
    // The message block.
    v[3] ^= x;
    for _ in 0..C_ROUNDS {
        round(&mut v);
    }
    v[0] ^= x;
    // The length block: the message is exactly 8 bytes, so the final block
    // is empty except for the length byte at the top.
    let b = 8u64 << 56;
    v[3] ^= b;
    for _ in 0..C_ROUNDS {
        round(&mut v);
    }
    v[0] ^= b;
    // Finalization.
    v[2] ^= 0xFF;
    for _ in 0..D_ROUNDS {
        round(&mut v);
    }
    v[0] ^ v[1] ^ v[2] ^ v[3]
}

anvil_abi::anvil_entry!(solve, |x| x);

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use std::hash::Hasher;

    /// Cross-check against an independent SipHash-1-3 implementation.
    #[test]
    fn matches_siphasher_crate() {
        for x in [0u64, 1, 0x0706050403020100, 0xDEADBEEFDEADBEEF, u64::MAX] {
            let mut h = siphasher::sip::SipHasher13::new_with_keys(super::K0, super::K1);
            h.write(&x.to_le_bytes());
            assert_eq!(super::solve(x), h.finish(), "x = {x:#x}");
        }
    }

    /// Values from the independently written Python model (scratch check).
    #[test]
    fn pinned_values() {
        assert_eq!(super::solve(0), 0x5CB96F6BA2A4FCFC);
        assert_eq!(super::solve(1), 0x32C5EA5CE472F19B);
        assert_eq!(super::solve(0x0706050403020100), 0x369095118D299A8E);
        assert_eq!(super::solve(0xDEADBEEFDEADBEEF), 0x365B9B6CC6292417);
    }
}
