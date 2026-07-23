//! ANV-007 contender: CRC-64/XZ four bits at a time through a 16-entry
//! table.
//!
//! Each step retires a nibble: shift the register down four and xor in the
//! precomputed remainder for the four bits that fell off. Sixteen steps
//! instead of sixty-four, and the whole table lives in two cache lines.
//!
//! The table below is data, and precomputed data is exactly the kind of
//! thing that silently rots: the Lean model `Razor.Anvil.crcNibble`
//! contains these sixteen constants verbatim, and the admission proof
//! checks the table-driven walk against the bit-at-a-time reference on all
//! 2^64 inputs - a wrong entry anywhere and the proof does not exist.

const NIB: [u64; 16] = [
    0x0000000000000000,
    0x7D9BA13851336649,
    0xFB374270A266CC92,
    0x86ACE348F355AADB,
    0x64B62BCAEBC387A1,
    0x192D8AF2BAF0E1E8,
    0x9F8169BA49A54B33,
    0xE21AC88218962D7A,
    0xC96C5795D7870F42,
    0xB4F7F6AD86B4690B,
    0x325B15E575E1C3D0,
    0x4FC0B4DD24D2A599,
    0xADDA7C5F3C4488E3,
    0xD041DD676D77EEAA,
    0x56ED3E2F9E224471,
    0x2B769F17CF112238,
];

pub fn solve(x: u64) -> u64 {
    let mut crc = !0u64 ^ x;
    for _ in 0..16 {
        crc = (crc >> 4) ^ NIB[(crc & 0xF) as usize];
    }
    !crc
}

anvil_abi::anvil_entry!(solve, |x| x);
