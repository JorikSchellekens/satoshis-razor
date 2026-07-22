import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-009: the AES S-box applied to each of the 8 bytes of a
u64.

The executable specification is `sboxScalar`: per byte, invert in GF(2^8)
mod x^8+x^4+x^3+x+1 by Fermat (x^254, which also sends 0 to 0), then the
standard affine rotate-xor - the FIPS-197 definition computed from
scratch, no tables (anvil/impls/sbox-scalar). Bytes are modeled in the low
8 bits of a `BitVec 64` with explicit `&&& 0xFF` masks mirroring the Rust
u8 arithmetic.

Two contender models:

- `sboxTable` (anvil/impls/sbox-table): the 256-byte lookup table,
  verbatim, as a two-level nibble select. `sbox_table_refines` checks the
  lookup against the from-the-definition spec on every input - a
  kernel-checked audit of all 256 memoized entries.
- `sboxSwar` (anvil/impls/sbox-swar): the same field arithmetic on all 8
  byte lanes at once, pure shift/and/or/xor - the constant-time form.
  `sbox_swar_refines` checks it likewise.

Proof architecture: the monolithic 8-byte goals are too large to unfold in
one piece (the inversion's addition chain re-uses every intermediate, so
full expansion is a ~250-multiplication tree per byte). The SAT work is
therefore factored into byte-sized certificates - one generic
lookup-vs-definition lemma for the table, and per-lane extraction lemmas
for the packed arithmetic (`gmul64_lane*`, `affine_lane*`) - and the
challenge theorems assemble them by rewriting.
-/

namespace Razor.Anvil

set_option maxRecDepth 65536
set_option maxHeartbeats 16000000
set_option Elab.async false

/-! ### The spec: GF(2^8) inversion + affine, one byte at a time -/

/-- One shift-and-add GF(2^8) multiply step on bytes held in the low 8
bits: `a`, `b`, accumulator `r`. -/
def gmul8Aux : Nat → BitVec 64 → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, _, r => r
  | k + 1, a, b, r =>
    let r := bif b &&& 1#64 == 1#64 then r ^^^ a else r
    let b := b >>> 1
    let hi := a &&& 0x80#64
    let a := (a <<< 1) &&& 0xFF#64
    let a := bif hi != 0#64 then a ^^^ 0x1B#64 else a
    gmul8Aux k a b r

/-- GF(2^8) multiplication mod x^8+x^4+x^3+x+1. -/
def gmul8 (a b : BitVec 64) : BitVec 64 := gmul8Aux 8 a b 0#64

/-- a^254 in GF(2^8): the inverse for a ≠ 0, and 0 at 0, by the
2-3-6-12-15-30-60-120-240-252-254 addition chain. -/
def ginv8 (a : BitVec 64) : BitVec 64 :=
  let p2 := gmul8 a a
  let p3 := gmul8 p2 a
  let p6 := gmul8 p3 p3
  let p12 := gmul8 p6 p6
  let p15 := gmul8 p12 p3
  let p30 := gmul8 p15 p15
  let p60 := gmul8 p30 p30
  let p120 := gmul8 p60 p60
  let p240 := gmul8 p120 p120
  let p252 := gmul8 p240 p12
  gmul8 p252 p2

/-- Rotate a byte held in the low 8 bits left by `n`. -/
def rotl8 (s : BitVec 64) (n : Nat) : BitVec 64 :=
  ((s <<< n) ||| (s >>> (8 - n))) &&& 0xFF#64

/-- The affine transform on one byte. -/
def sboxAffine8 (s : BitVec 64) : BitVec 64 :=
  s ^^^ rotl8 s 1 ^^^ rotl8 s 2 ^^^ rotl8 s 3 ^^^ rotl8 s 4 ^^^ 0x63#64

/-- The S-box on one byte: inversion then the affine transform. -/
def sbox8 (a : BitVec 64) : BitVec 64 := sboxAffine8 (ginv8 a)

/-- Executable spec: the S-box on each of the 8 bytes (model of
sbox-scalar). -/
def sboxScalar (x : BitVec 64) : BitVec 64 :=
  (sbox8 (x &&& 0xFF#64)) |||
  ((sbox8 ((x >>> 8) &&& 0xFF#64)) <<< 8) |||
  ((sbox8 ((x >>> 16) &&& 0xFF#64)) <<< 16) |||
  ((sbox8 ((x >>> 24) &&& 0xFF#64)) <<< 24) |||
  ((sbox8 ((x >>> 32) &&& 0xFF#64)) <<< 32) |||
  ((sbox8 ((x >>> 40) &&& 0xFF#64)) <<< 40) |||
  ((sbox8 ((x >>> 48) &&& 0xFF#64)) <<< 48) |||
  ((sbox8 ((x >>> 56) &&& 0xFF#64)) <<< 56)

/-! ### The table lane: 256 memoized bytes, verbatim -/

/-- Row 0x0 of the S-box table. -/
def sboxRow0 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x63#64 else bif n == 1#64 then 0x7c#64 else bif n == 2#64 then 0x77#64 else bif n == 3#64 then 0x7b#64 else bif n == 4#64 then 0xf2#64 else bif n == 5#64 then 0x6b#64 else bif n == 6#64 then 0x6f#64 else bif n == 7#64 then 0xc5#64 else bif n == 8#64 then 0x30#64 else bif n == 9#64 then 0x1#64 else bif n == 10#64 then 0x67#64 else bif n == 11#64 then 0x2b#64 else bif n == 12#64 then 0xfe#64 else bif n == 13#64 then 0xd7#64 else bif n == 14#64 then 0xab#64 else 0x76#64

/-- Row 0x1 of the S-box table. -/
def sboxRow1 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xca#64 else bif n == 1#64 then 0x82#64 else bif n == 2#64 then 0xc9#64 else bif n == 3#64 then 0x7d#64 else bif n == 4#64 then 0xfa#64 else bif n == 5#64 then 0x59#64 else bif n == 6#64 then 0x47#64 else bif n == 7#64 then 0xf0#64 else bif n == 8#64 then 0xad#64 else bif n == 9#64 then 0xd4#64 else bif n == 10#64 then 0xa2#64 else bif n == 11#64 then 0xaf#64 else bif n == 12#64 then 0x9c#64 else bif n == 13#64 then 0xa4#64 else bif n == 14#64 then 0x72#64 else 0xc0#64

/-- Row 0x2 of the S-box table. -/
def sboxRow2 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xb7#64 else bif n == 1#64 then 0xfd#64 else bif n == 2#64 then 0x93#64 else bif n == 3#64 then 0x26#64 else bif n == 4#64 then 0x36#64 else bif n == 5#64 then 0x3f#64 else bif n == 6#64 then 0xf7#64 else bif n == 7#64 then 0xcc#64 else bif n == 8#64 then 0x34#64 else bif n == 9#64 then 0xa5#64 else bif n == 10#64 then 0xe5#64 else bif n == 11#64 then 0xf1#64 else bif n == 12#64 then 0x71#64 else bif n == 13#64 then 0xd8#64 else bif n == 14#64 then 0x31#64 else 0x15#64

/-- Row 0x3 of the S-box table. -/
def sboxRow3 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x4#64 else bif n == 1#64 then 0xc7#64 else bif n == 2#64 then 0x23#64 else bif n == 3#64 then 0xc3#64 else bif n == 4#64 then 0x18#64 else bif n == 5#64 then 0x96#64 else bif n == 6#64 then 0x5#64 else bif n == 7#64 then 0x9a#64 else bif n == 8#64 then 0x7#64 else bif n == 9#64 then 0x12#64 else bif n == 10#64 then 0x80#64 else bif n == 11#64 then 0xe2#64 else bif n == 12#64 then 0xeb#64 else bif n == 13#64 then 0x27#64 else bif n == 14#64 then 0xb2#64 else 0x75#64

/-- Row 0x4 of the S-box table. -/
def sboxRow4 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x9#64 else bif n == 1#64 then 0x83#64 else bif n == 2#64 then 0x2c#64 else bif n == 3#64 then 0x1a#64 else bif n == 4#64 then 0x1b#64 else bif n == 5#64 then 0x6e#64 else bif n == 6#64 then 0x5a#64 else bif n == 7#64 then 0xa0#64 else bif n == 8#64 then 0x52#64 else bif n == 9#64 then 0x3b#64 else bif n == 10#64 then 0xd6#64 else bif n == 11#64 then 0xb3#64 else bif n == 12#64 then 0x29#64 else bif n == 13#64 then 0xe3#64 else bif n == 14#64 then 0x2f#64 else 0x84#64

/-- Row 0x5 of the S-box table. -/
def sboxRow5 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x53#64 else bif n == 1#64 then 0xd1#64 else bif n == 2#64 then 0x0#64 else bif n == 3#64 then 0xed#64 else bif n == 4#64 then 0x20#64 else bif n == 5#64 then 0xfc#64 else bif n == 6#64 then 0xb1#64 else bif n == 7#64 then 0x5b#64 else bif n == 8#64 then 0x6a#64 else bif n == 9#64 then 0xcb#64 else bif n == 10#64 then 0xbe#64 else bif n == 11#64 then 0x39#64 else bif n == 12#64 then 0x4a#64 else bif n == 13#64 then 0x4c#64 else bif n == 14#64 then 0x58#64 else 0xcf#64

/-- Row 0x6 of the S-box table. -/
def sboxRow6 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xd0#64 else bif n == 1#64 then 0xef#64 else bif n == 2#64 then 0xaa#64 else bif n == 3#64 then 0xfb#64 else bif n == 4#64 then 0x43#64 else bif n == 5#64 then 0x4d#64 else bif n == 6#64 then 0x33#64 else bif n == 7#64 then 0x85#64 else bif n == 8#64 then 0x45#64 else bif n == 9#64 then 0xf9#64 else bif n == 10#64 then 0x2#64 else bif n == 11#64 then 0x7f#64 else bif n == 12#64 then 0x50#64 else bif n == 13#64 then 0x3c#64 else bif n == 14#64 then 0x9f#64 else 0xa8#64

/-- Row 0x7 of the S-box table. -/
def sboxRow7 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x51#64 else bif n == 1#64 then 0xa3#64 else bif n == 2#64 then 0x40#64 else bif n == 3#64 then 0x8f#64 else bif n == 4#64 then 0x92#64 else bif n == 5#64 then 0x9d#64 else bif n == 6#64 then 0x38#64 else bif n == 7#64 then 0xf5#64 else bif n == 8#64 then 0xbc#64 else bif n == 9#64 then 0xb6#64 else bif n == 10#64 then 0xda#64 else bif n == 11#64 then 0x21#64 else bif n == 12#64 then 0x10#64 else bif n == 13#64 then 0xff#64 else bif n == 14#64 then 0xf3#64 else 0xd2#64

/-- Row 0x8 of the S-box table. -/
def sboxRow8 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xcd#64 else bif n == 1#64 then 0xc#64 else bif n == 2#64 then 0x13#64 else bif n == 3#64 then 0xec#64 else bif n == 4#64 then 0x5f#64 else bif n == 5#64 then 0x97#64 else bif n == 6#64 then 0x44#64 else bif n == 7#64 then 0x17#64 else bif n == 8#64 then 0xc4#64 else bif n == 9#64 then 0xa7#64 else bif n == 10#64 then 0x7e#64 else bif n == 11#64 then 0x3d#64 else bif n == 12#64 then 0x64#64 else bif n == 13#64 then 0x5d#64 else bif n == 14#64 then 0x19#64 else 0x73#64

/-- Row 0x9 of the S-box table. -/
def sboxRow9 (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x60#64 else bif n == 1#64 then 0x81#64 else bif n == 2#64 then 0x4f#64 else bif n == 3#64 then 0xdc#64 else bif n == 4#64 then 0x22#64 else bif n == 5#64 then 0x2a#64 else bif n == 6#64 then 0x90#64 else bif n == 7#64 then 0x88#64 else bif n == 8#64 then 0x46#64 else bif n == 9#64 then 0xee#64 else bif n == 10#64 then 0xb8#64 else bif n == 11#64 then 0x14#64 else bif n == 12#64 then 0xde#64 else bif n == 13#64 then 0x5e#64 else bif n == 14#64 then 0xb#64 else 0xdb#64

/-- Row 0xa of the S-box table. -/
def sboxRowA (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xe0#64 else bif n == 1#64 then 0x32#64 else bif n == 2#64 then 0x3a#64 else bif n == 3#64 then 0xa#64 else bif n == 4#64 then 0x49#64 else bif n == 5#64 then 0x6#64 else bif n == 6#64 then 0x24#64 else bif n == 7#64 then 0x5c#64 else bif n == 8#64 then 0xc2#64 else bif n == 9#64 then 0xd3#64 else bif n == 10#64 then 0xac#64 else bif n == 11#64 then 0x62#64 else bif n == 12#64 then 0x91#64 else bif n == 13#64 then 0x95#64 else bif n == 14#64 then 0xe4#64 else 0x79#64

/-- Row 0xb of the S-box table. -/
def sboxRowB (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xe7#64 else bif n == 1#64 then 0xc8#64 else bif n == 2#64 then 0x37#64 else bif n == 3#64 then 0x6d#64 else bif n == 4#64 then 0x8d#64 else bif n == 5#64 then 0xd5#64 else bif n == 6#64 then 0x4e#64 else bif n == 7#64 then 0xa9#64 else bif n == 8#64 then 0x6c#64 else bif n == 9#64 then 0x56#64 else bif n == 10#64 then 0xf4#64 else bif n == 11#64 then 0xea#64 else bif n == 12#64 then 0x65#64 else bif n == 13#64 then 0x7a#64 else bif n == 14#64 then 0xae#64 else 0x8#64

/-- Row 0xc of the S-box table. -/
def sboxRowC (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xba#64 else bif n == 1#64 then 0x78#64 else bif n == 2#64 then 0x25#64 else bif n == 3#64 then 0x2e#64 else bif n == 4#64 then 0x1c#64 else bif n == 5#64 then 0xa6#64 else bif n == 6#64 then 0xb4#64 else bif n == 7#64 then 0xc6#64 else bif n == 8#64 then 0xe8#64 else bif n == 9#64 then 0xdd#64 else bif n == 10#64 then 0x74#64 else bif n == 11#64 then 0x1f#64 else bif n == 12#64 then 0x4b#64 else bif n == 13#64 then 0xbd#64 else bif n == 14#64 then 0x8b#64 else 0x8a#64

/-- Row 0xd of the S-box table. -/
def sboxRowD (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x70#64 else bif n == 1#64 then 0x3e#64 else bif n == 2#64 then 0xb5#64 else bif n == 3#64 then 0x66#64 else bif n == 4#64 then 0x48#64 else bif n == 5#64 then 0x3#64 else bif n == 6#64 then 0xf6#64 else bif n == 7#64 then 0xe#64 else bif n == 8#64 then 0x61#64 else bif n == 9#64 then 0x35#64 else bif n == 10#64 then 0x57#64 else bif n == 11#64 then 0xb9#64 else bif n == 12#64 then 0x86#64 else bif n == 13#64 then 0xc1#64 else bif n == 14#64 then 0x1d#64 else 0x9e#64

/-- Row 0xe of the S-box table. -/
def sboxRowE (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0xe1#64 else bif n == 1#64 then 0xf8#64 else bif n == 2#64 then 0x98#64 else bif n == 3#64 then 0x11#64 else bif n == 4#64 then 0x69#64 else bif n == 5#64 then 0xd9#64 else bif n == 6#64 then 0x8e#64 else bif n == 7#64 then 0x94#64 else bif n == 8#64 then 0x9b#64 else bif n == 9#64 then 0x1e#64 else bif n == 10#64 then 0x87#64 else bif n == 11#64 then 0xe9#64 else bif n == 12#64 then 0xce#64 else bif n == 13#64 then 0x55#64 else bif n == 14#64 then 0x28#64 else 0xdf#64

/-- Row 0xf of the S-box table. -/
def sboxRowF (n : BitVec 64) : BitVec 64 :=
  bif n == 0#64 then 0x8c#64 else bif n == 1#64 then 0xa1#64 else bif n == 2#64 then 0x89#64 else bif n == 3#64 then 0xd#64 else bif n == 4#64 then 0xbf#64 else bif n == 5#64 then 0xe6#64 else bif n == 6#64 then 0x42#64 else bif n == 7#64 then 0x68#64 else bif n == 8#64 then 0x41#64 else bif n == 9#64 then 0x99#64 else bif n == 10#64 then 0x2d#64 else bif n == 11#64 then 0xf#64 else bif n == 12#64 then 0xb0#64 else bif n == 13#64 then 0x54#64 else bif n == 14#64 then 0xbb#64 else 0x16#64

/-- The 256-entry lookup as a two-level nibble select. -/
def sboxLookup (b : BitVec 64) : BitVec 64 :=
  let hi := (b >>> 4) &&& 0xF#64
  let lo := b &&& 0xF#64
  bif hi == 0#64 then sboxRow0 lo else
  bif hi == 1#64 then sboxRow1 lo else
  bif hi == 2#64 then sboxRow2 lo else
  bif hi == 3#64 then sboxRow3 lo else
  bif hi == 4#64 then sboxRow4 lo else
  bif hi == 5#64 then sboxRow5 lo else
  bif hi == 6#64 then sboxRow6 lo else
  bif hi == 7#64 then sboxRow7 lo else
  bif hi == 8#64 then sboxRow8 lo else
  bif hi == 9#64 then sboxRow9 lo else
  bif hi == 10#64 then sboxRowA lo else
  bif hi == 11#64 then sboxRowB lo else
  bif hi == 12#64 then sboxRowC lo else
  bif hi == 13#64 then sboxRowD lo else
  bif hi == 14#64 then sboxRowE lo else
  sboxRowF lo

/-- Model of sbox-table: the memoized lookup on each byte. -/
def sboxTable (x : BitVec 64) : BitVec 64 :=
  (sboxLookup (x &&& 0xFF#64)) |||
  ((sboxLookup ((x >>> 8) &&& 0xFF#64)) <<< 8) |||
  ((sboxLookup ((x >>> 16) &&& 0xFF#64)) <<< 16) |||
  ((sboxLookup ((x >>> 24) &&& 0xFF#64)) <<< 24) |||
  ((sboxLookup ((x >>> 32) &&& 0xFF#64)) <<< 32) |||
  ((sboxLookup ((x >>> 40) &&& 0xFF#64)) <<< 40) |||
  ((sboxLookup ((x >>> 48) &&& 0xFF#64)) <<< 48) |||
  ((sboxLookup ((x >>> 56) &&& 0xFF#64)) <<< 56)

/-! ### The constant-time lane: all 8 lanes at once, pure bitwise -/

/-- Smear each lane's low bit across its byte: 0x01 → 0xFF. -/
def smear64 (m : BitVec 64) : BitVec 64 :=
  let m := m ||| (m <<< 1)
  let m := m ||| (m <<< 2)
  m ||| (m <<< 4)

/-- One lane-wise GF(2^8) multiply step: all 8 byte lanes at once. -/
def gmul64Aux : Nat → BitVec 64 → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, _, r => r
  | k + 1, a, b, r =>
    let r := r ^^^ (a &&& smear64 (b &&& 0x0101010101010101#64))
    let b := (b >>> 1) &&& 0x7F7F7F7F7F7F7F7F#64
    let hi := (a >>> 7) &&& 0x0101010101010101#64
    let a := ((a &&& 0x7F7F7F7F7F7F7F7F#64) <<< 1) ^^^
             (hi ||| (hi <<< 1) ||| (hi <<< 3) ||| (hi <<< 4))
    gmul64Aux k a b r

/-- Lane-wise GF(2^8) multiplication. -/
def gmul64 (a b : BitVec 64) : BitVec 64 := gmul64Aux 8 a b 0#64

/-- Lane-wise a^254, the same addition chain as the scalar spec. -/
def ginv64 (a : BitVec 64) : BitVec 64 :=
  let p2 := gmul64 a a
  let p3 := gmul64 p2 a
  let p6 := gmul64 p3 p3
  let p12 := gmul64 p6 p6
  let p15 := gmul64 p12 p3
  let p30 := gmul64 p15 p15
  let p60 := gmul64 p30 p30
  let p120 := gmul64 p60 p60
  let p240 := gmul64 p120 p120
  let p252 := gmul64 p240 p12
  gmul64 p252 p2

/-- The affine transform on all 8 lanes, with per-lane keep/carry masks. -/
def sboxAffine64 (s : BitVec 64) : BitVec 64 :=
  let r1 := ((s &&& 0x7F7F7F7F7F7F7F7F#64) <<< 1) ||| ((s >>> 7) &&& 0x0101010101010101#64)
  let r2 := ((s &&& 0x3F3F3F3F3F3F3F3F#64) <<< 2) ||| ((s >>> 6) &&& 0x0303030303030303#64)
  let r3 := ((s &&& 0x1F1F1F1F1F1F1F1F#64) <<< 3) ||| ((s >>> 5) &&& 0x0707070707070707#64)
  let r4 := ((s &&& 0x0F0F0F0F0F0F0F0F#64) <<< 4) ||| ((s >>> 4) &&& 0x0F0F0F0F0F0F0F0F#64)
  s ^^^ r1 ^^^ r2 ^^^ r3 ^^^ r4 ^^^ 0x6363636363636363#64

/-- Model of sbox-swar: lane-wise inversion, then the affine transform. -/
def sboxSwar (x : BitVec 64) : BitVec 64 := sboxAffine64 (ginv64 x)

/-! ### Byte-sized SAT certificates -/

/-- The table core: every one of the 256 entries of the lookup equals the
from-the-definition S-box, checked by compiled evaluation over the whole
byte domain (`native_decide` - the same `Lean.ofReduceBool` route the SAT
pipeline itself relies on). Stated over `BitVec 8` so the quantifier is
finite. -/
theorem sboxTableSound :
    ∀ b : BitVec 8, sboxLookup (b.setWidth 64) = sbox8 (b.setWidth 64) := by
  native_decide

/-- Widening a masked byte through 8 bits is the identity. -/
theorem maskRound (y : BitVec 64) :
    ((y &&& 0xFF#64).setWidth 8).setWidth 64 = y &&& 0xFF#64 := by
  bv_decide (config := { timeout := 300 })

/-- One generic byte's lookup equals the from-the-definition S-box. -/
theorem sboxLookup_eq_sbox8 (b : BitVec 64) :
    sboxLookup (b &&& 0xFF#64) = sbox8 (b &&& 0xFF#64) := by
  rw [← maskRound b]
  exact sboxTableSound ((b &&& 0xFF#64).setWidth 8)

/-- Lane extraction commutes with the packed GF multiply: byte `i` of
`gmul64 a b` is `gmul8` of byte `i` of each operand. One small SAT check
per lane. -/
theorem gmul64_lane0 (a b : BitVec 64) :
    (gmul64 a b &&& 0xFF#64) = gmul8 (a &&& 0xFF#64) (b &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane1 (a b : BitVec 64) :
    ((gmul64 a b >>> 8) &&& 0xFF#64) = gmul8 ((a >>> 8) &&& 0xFF#64) ((b >>> 8) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane2 (a b : BitVec 64) :
    ((gmul64 a b >>> 16) &&& 0xFF#64) = gmul8 ((a >>> 16) &&& 0xFF#64) ((b >>> 16) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane3 (a b : BitVec 64) :
    ((gmul64 a b >>> 24) &&& 0xFF#64) = gmul8 ((a >>> 24) &&& 0xFF#64) ((b >>> 24) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane4 (a b : BitVec 64) :
    ((gmul64 a b >>> 32) &&& 0xFF#64) = gmul8 ((a >>> 32) &&& 0xFF#64) ((b >>> 32) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane5 (a b : BitVec 64) :
    ((gmul64 a b >>> 40) &&& 0xFF#64) = gmul8 ((a >>> 40) &&& 0xFF#64) ((b >>> 40) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane6 (a b : BitVec 64) :
    ((gmul64 a b >>> 48) &&& 0xFF#64) = gmul8 ((a >>> 48) &&& 0xFF#64) ((b >>> 48) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })
theorem gmul64_lane7 (a b : BitVec 64) :
    ((gmul64 a b >>> 56) &&& 0xFF#64) = gmul8 ((a >>> 56) &&& 0xFF#64) ((b >>> 56) &&& 0xFF#64) := by
  simp (config := { maxSteps := 8000000 }) only [gmul64, gmul64Aux, smear64, gmul8, gmul8Aux]
  bv_decide (config := { timeout := 300 })

/-- Lane extraction commutes with the packed affine transform. -/
theorem affine_lane0 (s : BitVec 64) :
    (sboxAffine64 s &&& 0xFF#64) = sboxAffine8 (s &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane1 (s : BitVec 64) :
    ((sboxAffine64 s >>> 8) &&& 0xFF#64) = sboxAffine8 ((s >>> 8) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane2 (s : BitVec 64) :
    ((sboxAffine64 s >>> 16) &&& 0xFF#64) = sboxAffine8 ((s >>> 16) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane3 (s : BitVec 64) :
    ((sboxAffine64 s >>> 24) &&& 0xFF#64) = sboxAffine8 ((s >>> 24) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane4 (s : BitVec 64) :
    ((sboxAffine64 s >>> 32) &&& 0xFF#64) = sboxAffine8 ((s >>> 32) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane5 (s : BitVec 64) :
    ((sboxAffine64 s >>> 40) &&& 0xFF#64) = sboxAffine8 ((s >>> 40) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane6 (s : BitVec 64) :
    ((sboxAffine64 s >>> 48) &&& 0xFF#64) = sboxAffine8 ((s >>> 48) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })
theorem affine_lane7 (s : BitVec 64) :
    ((sboxAffine64 s >>> 56) &&& 0xFF#64) = sboxAffine8 ((s >>> 56) &&& 0xFF#64) := by
  simp only [sboxAffine64, sboxAffine8, rotl8]
  bv_decide (config := { timeout := 300 })

/-- A 64-bit word is the OR of its eight extracted bytes. -/
theorem bytesRecompose (y : BitVec 64) :
    y =
  (y &&& 0xFF#64) |||
  (((y >>> 8) &&& 0xFF#64) <<< 8) |||
  (((y >>> 16) &&& 0xFF#64) <<< 16) |||
  (((y >>> 24) &&& 0xFF#64) <<< 24) |||
  (((y >>> 32) &&& 0xFF#64) <<< 32) |||
  (((y >>> 40) &&& 0xFF#64) <<< 40) |||
  (((y >>> 48) &&& 0xFF#64) <<< 48) |||
  (((y >>> 56) &&& 0xFF#64) <<< 56) := by
  bv_decide (config := { timeout := 300 })

/-- Byte `i` of the packed S-box equals the scalar S-box of byte `i`:
push the lane extraction through the affine transform and the whole
inversion chain with the lemmas above, then both sides are the same
`gmul8` tree. -/
theorem sbox_swar_lane0 (x : BitVec 64) :
    (sboxSwar x &&& 0xFF#64) = sbox8 (x &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane0]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane0]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane1 (x : BitVec 64) :
    ((sboxSwar x >>> 8) &&& 0xFF#64) = sbox8 ((x >>> 8) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane1]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane1]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane2 (x : BitVec 64) :
    ((sboxSwar x >>> 16) &&& 0xFF#64) = sbox8 ((x >>> 16) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane2]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane2]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane3 (x : BitVec 64) :
    ((sboxSwar x >>> 24) &&& 0xFF#64) = sbox8 ((x >>> 24) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane3]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane3]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane4 (x : BitVec 64) :
    ((sboxSwar x >>> 32) &&& 0xFF#64) = sbox8 ((x >>> 32) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane4]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane4]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane5 (x : BitVec 64) :
    ((sboxSwar x >>> 40) &&& 0xFF#64) = sbox8 ((x >>> 40) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane5]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane5]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane6 (x : BitVec 64) :
    ((sboxSwar x >>> 48) &&& 0xFF#64) = sbox8 ((x >>> 48) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane6]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane6]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]
theorem sbox_swar_lane7 (x : BitVec 64) :
    ((sboxSwar x >>> 56) &&& 0xFF#64) = sbox8 ((x >>> 56) &&& 0xFF#64) := by
  simp only [sboxSwar]
  rw [affine_lane7]
  simp (config := { maxSteps := 8000000 }) only [ginv64, gmul64_lane7]
  simp (config := { maxSteps := 8000000 }) only [sbox8, ginv8]

/-! ### The challenge theorems -/

/-- ANV-009 admission proof for sbox-table: every one of the 256 memoized
entries agrees with the from-the-definition spec. -/
theorem sbox_table_refines (x : BitVec 64) : sboxTable x = sboxScalar x := by
  simp only [sboxTable, sboxScalar, sboxLookup_eq_sbox8]

/-- ANV-009 admission proof for sbox-swar: the packed constant-time field
arithmetic agrees with the from-the-definition spec on every input. -/
theorem sbox_swar_refines (x : BitVec 64) : sboxSwar x = sboxScalar x := by
  rw [bytesRecompose (sboxSwar x)]
  rw [sbox_swar_lane0 x, sbox_swar_lane1 x, sbox_swar_lane2 x, sbox_swar_lane3 x, sbox_swar_lane4 x, sbox_swar_lane5 x, sbox_swar_lane6 x, sbox_swar_lane7 x]
  simp only [sboxScalar]

end Razor.Anvil
