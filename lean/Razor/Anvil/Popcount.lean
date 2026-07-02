import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-001: population count (u64).

The executable specification is `popNaive`: a 64-iteration shift-and-add loop
over `BitVec 64`, mirroring the reference Rust implementation instruction for
instruction. The champion submission is the SWAR bit-trick implementation; its
Lean model is `popSwar`, and `swar_refines` is the machine-checked admission
proof that the two agree on all 2^64 inputs.

The models are hand-translated from the Rust sources (anvil/impls/*). In the
full pipeline this translation is produced by Charon + Aeneas; the proof
obligation is identical.

`swar_refines` is discharged by `bv_decide`: the equality is a quantifier-free
bit-vector identity, so Lean's verified SAT pipeline settles it - no human
insight required, which is exactly what makes AI-generated bit-trick
submissions safely admissible.
-/

namespace Razor.Anvil

set_option maxRecDepth 4096

/-- One shift-add step: consume the low bit into the accumulator. -/
def popStep (x acc : BitVec 64) : BitVec 64 × BitVec 64 :=
  (x >>> 1, acc + (x &&& 1#64))

/-- Fuel-indexed loop. -/
def popAux : Nat → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, acc => acc
  | n + 1, x, acc => popAux n (x >>> 1) (acc + (x &&& 1#64))

/-- Executable spec: the naive 64-iteration loop (model of popcount-naive). -/
def popNaive (x : BitVec 64) : BitVec 64 := popAux 64 x 0

/-- Model of popcount-swar: the SWAR bit-trick popcount. -/
def popSwar (x : BitVec 64) : BitVec 64 :=
  let m1  : BitVec 64 := 0x5555555555555555#64
  let m2  : BitVec 64 := 0x3333333333333333#64
  let m4  : BitVec 64 := 0x0f0f0f0f0f0f0f0f#64
  let h01 : BitVec 64 := 0x0101010101010101#64
  let a := x - ((x >>> 1) &&& m1)
  let b := (a &&& m2) + ((a >>> 2) &&& m2)
  let c := (b + (b >>> 4)) &&& m4
  (c * h01) >>> 56

/-- ANV-001 admission proof for popcount-swar: the SWAR implementation refines
the executable spec on every input. -/
theorem swar_refines (x : BitVec 64) : popSwar x = popNaive x := by
  simp only [popNaive, popAux, popSwar]
  bv_decide (config := { timeout := 300 })

/-- Instance checks (challenge-window certificates). -/
example : popNaive 0 = 0 := by decide
example : popNaive 0xffffffffffffffff = 64 := by decide
example : popNaive 0x8000000000000001 = 2 := by decide

end Razor.Anvil
