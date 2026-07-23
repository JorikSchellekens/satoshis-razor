//! ANV-007 reference: CRC-64/XZ of the 8 bytes of a u64, one bit at a time.
//!
//! CRC-64/XZ (reflected ECMA-182 polynomial, init and final-xor all ones)
//! is the checksum inside every .xz archive. For a message of exactly 8
//! little-endian bytes, the byte-at-a-time reflected loop collapses to:
//! xor the whole word into the register, then take 64 single-bit steps.
//! That equivalence was checked against the byte-wise definition (which
//! itself reproduces the catalog check value for "123456789") on random
//! inputs before this was pinned as the reference.
//!
//! The Lean model is `Razor.Anvil.crcBitwise`.

const POLY: u64 = 0xC96C5795D7870F42;

pub fn solve(x: u64) -> u64 {
    let mut crc = !0u64 ^ x;
    for _ in 0..64 {
        crc = (crc >> 1) ^ if crc & 1 == 1 { POLY } else { 0 };
    }
    !crc
}

anvil_abi::anvil_entry!(solve, |x| x);

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    /// Byte-at-a-time CRC-64/XZ, the textbook form the word trick must match.
    fn crc64_bytes(data: &[u8]) -> u64 {
        let mut crc = !0u64;
        for &b in data {
            crc ^= b as u64;
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 == 1 { super::POLY } else { 0 };
            }
        }
        !crc
    }

    #[test]
    fn catalog_check_value() {
        assert_eq!(crc64_bytes(b"123456789"), 0x995DC9BBDF1939FA);
    }

    #[test]
    fn word_trick_matches_bytewise() {
        let mut state = 0x1234_5678_9ABC_DEF0u64;
        for _ in 0..5000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            assert_eq!(super::solve(state), crc64_bytes(&state.to_le_bytes()));
        }
    }
}
