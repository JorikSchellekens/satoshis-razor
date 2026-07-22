import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-006: SipHash-1-3 of one 8-byte message, fixed key.

The executable specification is `sip13Ref`: the paper-shaped computation -
initialize the four state words from the key, absorb the message block
with one compression round, absorb the length block with one more, then
three finalization rounds (anvil/impls/siphash13-ref). The state is packed
into one 256-bit word so the round function is a plain bit-vector map with
no tuples for the SAT pipeline to stumble on.

The contender model is `sip13Inline`: the same hash unrolled straight-line
with the key-derived initial words and the constant length block folded in
(anvil/impls/siphash13-stream). `sip13_inline_refines` checks agreement on
all 2^64 inputs by `bv_decide`.

The models are hand-translated from the Rust sources; the Rust reference
is itself cross-checked against the official SipHash test vectors and an
independent implementation in its crate tests.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384
set_option maxHeartbeats 2000000

/-- One SipRound on the packed state `v3 ++ v2 ++ v1 ++ v0`. -/
def sipRound (s : BitVec 256) : BitVec 256 :=
  let v0 := s.extractLsb' 0 64
  let v1 := s.extractLsb' 64 64
  let v2 := s.extractLsb' 128 64
  let v3 := s.extractLsb' 192 64
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  v3 ++ v2 ++ v1 ++ v0

/-- `k` SipRounds. -/
def sipRounds : Nat → BitVec 256 → BitVec 256
  | 0, s => s
  | k + 1, s => sipRounds k (sipRound s)

/-- The challenge's fixed key: the classic test key from the SipHash paper,
k = 00 01 02 ... 0f as two little-endian words. -/
def sipK0 : BitVec 64 := 0x0706050403020100#64
def sipK1 : BitVec 64 := 0x0F0E0D0C0B0A0908#64

/-- Executable spec: SipHash-1-3 of the 8-byte little-endian message `x`
(model of siphash13-ref). One compression round per block, three
finalization rounds; the length block of an exactly-8-byte message is just
`8 <<< 56`. -/
def sip13Ref (x : BitVec 64) : BitVec 64 :=
  let s : BitVec 256 :=
    (sipK1 ^^^ 0x7465646279746573#64) ++ (sipK0 ^^^ 0x6C7967656E657261#64) ++
    (sipK1 ^^^ 0x646F72616E646F6D#64) ++ (sipK0 ^^^ 0x736F6D6570736575#64)
  -- message block: v3 ^= m, one round, v0 ^= m
  let s := s ^^^ ((x.zeroExtend 256) <<< 192)
  let s := sipRounds 1 s
  let s := s ^^^ x.zeroExtend 256
  -- length block
  let b : BitVec 64 := 0x0800000000000000#64
  let s := s ^^^ ((b.zeroExtend 256) <<< 192)
  let s := sipRounds 1 s
  let s := s ^^^ b.zeroExtend 256
  -- finalization: v2 ^= 0xff, three rounds, fold the state
  let s := s ^^^ (((0xFF#64 : BitVec 64).zeroExtend 256) <<< 128)
  let s := sipRounds 3 s
  (s.extractLsb' 0 64) ^^^ (s.extractLsb' 64 64) ^^^
  (s.extractLsb' 128 64) ^^^ (s.extractLsb' 192 64)

/-- Model of siphash13-stream's per-word function: the same hash fully
unrolled, with the key-derived initial words and the constant length block
folded to literals. -/
def sip13Inline (x : BitVec 64) : BitVec 64 :=
  let v0 : BitVec 64 := 0x7469686173716475#64
  let v1 : BitVec 64 := 0x6b617f6d656e6665#64
  let v2 : BitVec 64 := 0x6b7f62616d677361#64
  let v3 : BitVec 64 := 0x7b6b696e727e6c7b#64 ^^^ x
  -- round 1 (message block)
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  let v0 := v0 ^^^ x
  let v3 := v3 ^^^ 0x0800000000000000#64
  -- round 2 (length block)
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  let v0 := v0 ^^^ 0x0800000000000000#64
  let v2 := v2 ^^^ 0xFF#64
  -- rounds 3-5 (finalization)
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  let v0 := v0 + v1
  let v1 := (v1.rotateLeft 13) ^^^ v0
  let v0 := v0.rotateLeft 32
  let v2 := v2 + v3
  let v3 := (v3.rotateLeft 16) ^^^ v2
  let v0 := v0 + v3
  let v3 := (v3.rotateLeft 21) ^^^ v0
  let v2 := v2 + v1
  let v1 := (v1.rotateLeft 17) ^^^ v2
  let v2 := v2.rotateLeft 32
  v0 ^^^ v1 ^^^ v2 ^^^ v3

/-- ANV-006 admission proof for siphash13-stream: the unrolled,
constant-folded hash agrees with the paper-shaped spec on every input. -/
theorem sip13_inline_refines (x : BitVec 64) : sip13Inline x = sip13Ref x := by
  simp (config := { maxSteps := 4000000 }) only [sip13Inline, sip13Ref, sipRounds, sipRound, sipK0, sipK1]
  bv_decide (config := { timeout := 300 })

end Razor.Anvil
