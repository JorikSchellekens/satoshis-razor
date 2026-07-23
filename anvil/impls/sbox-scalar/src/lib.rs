//! ANV-009 reference: the AES S-box applied to each of the 8 bytes,
//! computed from its definition.
//!
//! The S-box is the only nonlinear step in AES (SubBytes): invert the byte
//! in GF(2^8) mod x^8+x^4+x^3+x+1 (zero maps to zero - Fermat's x^254
//! handles that case for free), then apply the standard affine transform.
//! This reference computes exactly that, per byte: a shift-and-add GF
//! multiplier, inversion by the 2-3-12-15-240-252-254 addition chain, then
//! the affine rotate-xor. No tables anywhere - the executable spec IS the
//! standard's math, which is what makes beating it meaningful: a table
//! lane is claiming its 256 memoized bytes equal this computation, and the
//! admission proof checks every entry.
//!
//! The Lean model is `Razor.Anvil.sboxScalar`.

fn gmul(mut a: u8, mut b: u8) -> u8 {
    let mut r = 0u8;
    for _ in 0..8 {
        if b & 1 == 1 {
            r ^= a;
        }
        b >>= 1;
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1B;
        }
    }
    r
}

/// a^254 in GF(2^8): the inverse for a != 0, and 0 for 0.
fn ginv(a: u8) -> u8 {
    let p2 = gmul(a, a);
    let p3 = gmul(p2, a);
    let p6 = gmul(p3, p3);
    let p12 = gmul(p6, p6);
    let p15 = gmul(p12, p3);
    let p30 = gmul(p15, p15);
    let p60 = gmul(p30, p30);
    let p120 = gmul(p60, p60);
    let p240 = gmul(p120, p120);
    let p252 = gmul(p240, p12);
    gmul(p252, p2)
}

fn sbox(a: u8) -> u8 {
    let s = ginv(a);
    s ^ s.rotate_left(1) ^ s.rotate_left(2) ^ s.rotate_left(3) ^ s.rotate_left(4) ^ 0x63
}

pub fn solve(x: u64) -> u64 {
    let mut out = 0u64;
    for i in 0..8 {
        out |= (sbox((x >> (8 * i)) as u8) as u64) << (8 * i);
    }
    out
}

anvil_abi::anvil_entry!(solve, |x| x);

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    /// The FIPS-197 S-box's known entries: the head row and spot values.
    #[test]
    fn known_entries() {
        for (input, expected) in [
            (0x00u8, 0x63u8),
            (0x01, 0x7C),
            (0x02, 0x77),
            (0x03, 0x7B),
            (0x04, 0xF2),
            (0x53, 0xED),
            (0xFF, 0x16),
        ] {
            assert_eq!(super::sbox(input), expected, "sbox({input:#04x})");
        }
    }
}
