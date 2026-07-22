import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-005: reverse the bits of a u64.

The executable specification is `revNaive`: 64 iterations, each moving one
bit from the bottom of the input to the bottom of the shifting result - a
straight transliteration of the reference Rust implementation
(anvil/impls/bitrev-naive). The contender is `revSwar`: six mask-and-shift
layers that swap adjacent bits, then pairs, nibbles, bytes, byte pairs, and
finally the two 32-bit halves (anvil/impls/bitrev-swar).

`rev_swar_refines` is the machine-checked admission proof that the two
agree on all 2^64 inputs, discharged by `bv_decide`.

The models are hand-translated from the Rust sources. In the full pipeline
this translation is produced by Charon + Aeneas; the proof obligation is
identical.
-/

namespace Razor.Anvil

set_option maxRecDepth 8192

/-- Fuel-indexed loop: shift one bit per step from input to result. -/
def revAux : Nat → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, r => r
  | k + 1, x, r => revAux k (x >>> 1) ((r <<< 1) ||| (x &&& 1#64))

/-- Executable spec: the one-bit-at-a-time reversal (model of bitrev-naive). -/
def revNaive (x : BitVec 64) : BitVec 64 := revAux 64 x 0

/-- Model of bitrev-swar: six swap layers, doubling the swapped width. -/
def revSwar (x : BitVec 64) : BitVec 64 :=
  let x := ((x >>> 1) &&& 0x5555555555555555#64) ||| ((x &&& 0x5555555555555555#64) <<< 1)
  let x := ((x >>> 2) &&& 0x3333333333333333#64) ||| ((x &&& 0x3333333333333333#64) <<< 2)
  let x := ((x >>> 4) &&& 0x0f0f0f0f0f0f0f0f#64) ||| ((x &&& 0x0f0f0f0f0f0f0f0f#64) <<< 4)
  let x := ((x >>> 8) &&& 0x00ff00ff00ff00ff#64) ||| ((x &&& 0x00ff00ff00ff00ff#64) <<< 8)
  let x := ((x >>> 16) &&& 0x0000ffff0000ffff#64) ||| ((x &&& 0x0000ffff0000ffff#64) <<< 16)
  (x >>> 32) ||| (x <<< 32)

/-- ANV-005 admission proof for bitrev-swar: the six swap layers agree with
the one-bit-at-a-time reversal on every input. -/
theorem rev_swar_refines (x : BitVec 64) : revSwar x = revNaive x := by
  simp only [revNaive, revAux, revSwar]
  bv_decide (config := { timeout := 300 })

/-- Instance checks (challenge-window certificates). -/
example : revNaive 0 = 0 := by decide
example : revNaive 1 = 0x8000000000000000 := by decide
example : revNaive 0x00000000000000ff = 0xff00000000000000 := by decide

end Razor.Anvil
