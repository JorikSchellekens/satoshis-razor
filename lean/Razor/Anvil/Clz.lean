import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-004: count leading zeros (u64).

The executable specification is `clzNaive`: examine the top bit and either
stop or shift left and count, at most 64 times - a straight transliteration
of the reference Rust implementation (anvil/impls/clz-naive). The contender
is `clzBinary`: six halving steps, each testing whether the remaining top
half is empty (anvil/impls/clz-branchless).

`clz_binary_refines` is the machine-checked admission proof that the two
agree on all 2^64 inputs, discharged by `bv_decide`: the equality is a
quantifier-free bit-vector identity, so Lean's verified SAT pipeline
settles it - the same route that admitted the SWAR popcount.

The models are hand-translated from the Rust sources. In the full pipeline
this translation is produced by Charon + Aeneas; the proof obligation is
identical.
-/

namespace Razor.Anvil

set_option maxRecDepth 8192

/-- Fuel-indexed scan from the top bit: stop at the first set bit, else
shift left and count. -/
def clzAux : Nat → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, n => n
  | k + 1, x, n =>
    bif (x &&& 0x8000000000000000#64) != 0#64 then n
    else clzAux k (x <<< 1) (n + 1)

/-- Executable spec: the one-bit-at-a-time scan (model of clz-naive). -/
def clzNaive (x : BitVec 64) : BitVec 64 := clzAux 64 x 0

/-- Model of clz-branchless: binary search in six halving steps. Written
with one `bif` per updated value (no tuples) so the whole body is plain
bit-vector arithmetic that `bv_decide` can bit-blast. -/
def clzBinary (x : BitVec 64) : BitVec 64 :=
  bif x == 0#64 then 64#64 else
  let n1 := bif x >>> 32 == 0#64 then 32#64 else 0#64
  let x1 := bif x >>> 32 == 0#64 then x <<< 32 else x
  let n2 := bif x1 >>> 48 == 0#64 then n1 + 16 else n1
  let x2 := bif x1 >>> 48 == 0#64 then x1 <<< 16 else x1
  let n3 := bif x2 >>> 56 == 0#64 then n2 + 8 else n2
  let x3 := bif x2 >>> 56 == 0#64 then x2 <<< 8 else x2
  let n4 := bif x3 >>> 60 == 0#64 then n3 + 4 else n3
  let x4 := bif x3 >>> 60 == 0#64 then x3 <<< 4 else x3
  let n5 := bif x4 >>> 62 == 0#64 then n4 + 2 else n4
  let x5 := bif x4 >>> 62 == 0#64 then x4 <<< 2 else x4
  bif x5 >>> 63 == 0#64 then n5 + 1 else n5

/-- ANV-004 admission proof for clz-branchless: the binary search agrees
with the one-bit-at-a-time scan on every input. -/
theorem clz_binary_refines (x : BitVec 64) : clzBinary x = clzNaive x := by
  simp only [clzNaive, clzAux, clzBinary]
  bv_decide (config := { timeout := 300 })

/-- Instance checks (challenge-window certificates). -/
example : clzNaive 0 = 64 := by decide
example : clzNaive 1 = 63 := by decide
example : clzNaive 0x8000000000000000 = 0 := by decide
example : clzNaive 0x00ff000000000000 = 8 := by decide

end Razor.Anvil
