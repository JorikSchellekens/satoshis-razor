import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-003: sort the 8 bytes of a u64.

The input word is read as 8 bytes (byte 0 = least significant); the output is
the same bytes in ascending order, packed back the same way.

The executable specification is `sortBubble`: bubble sort with fixed passes,
mirroring the reference Rust implementation (anvil/impls/sort8-bubble).
Because the passes are fixed, bubble sort on 8 elements is exactly a sequence
of 28 adjacent compare-swaps - straight-line code, faithful to model.

The champion submission is `sortNetwork`: the size-optimal sorting network
for 8 inputs, 19 comparators (anvil/impls/sort8-network). `network_refines`
is the machine-checked admission proof that the two agree on all 2^64
inputs, discharged by `bv_decide` - nobody has to see why 19 comparators in
this exact order sort every input; the SAT certificate settles it.

The models are hand-translated from the Rust sources. In the full pipeline
this translation is produced by Charon + Aeneas; the proof obligation is
identical.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384

/-- Byte lane `i` of the word (zero-extended into the full width, which keeps
every comparator in `BitVec 64` - the shape `bv_decide` digests best). -/
def lane (x : BitVec 64) (i : Nat) : BitVec 64 :=
  (x >>> (8 * i)) &&& 0xff#64

/-- Write byte lane `i` (assumes `v < 256`, which every `lane` satisfies). -/
def setLane (x : BitVec 64) (i : Nat) (v : BitVec 64) : BitVec 64 :=
  (x &&& ~~~(0xff#64 <<< (8 * i))) ||| (v <<< (8 * i))

/-- One comparator: after `cswap x i j`, lane `i` holds the smaller byte and
lane `j` the larger. Both implementations are compositions of this. -/
def cswap (x : BitVec 64) (i j : Nat) : BitVec 64 :=
  let a := lane x i
  let b := lane x j
  bif a.ule b then x else setLane (setLane x i b) j a

/-- Executable spec (model of sort8-bubble): fixed-pass bubble sort, written
out as its 28 adjacent compare-swaps, pass by pass. -/
def sortBubble (x : BitVec 64) : BitVec 64 :=
  -- pass 1: j = 0..6
  let x := cswap x 0 1; let x := cswap x 1 2; let x := cswap x 2 3
  let x := cswap x 3 4; let x := cswap x 4 5; let x := cswap x 5 6
  let x := cswap x 6 7
  -- pass 2: j = 0..5
  let x := cswap x 0 1; let x := cswap x 1 2; let x := cswap x 2 3
  let x := cswap x 3 4; let x := cswap x 4 5; let x := cswap x 5 6
  -- pass 3: j = 0..4
  let x := cswap x 0 1; let x := cswap x 1 2; let x := cswap x 2 3
  let x := cswap x 3 4; let x := cswap x 4 5
  -- pass 4: j = 0..3
  let x := cswap x 0 1; let x := cswap x 1 2; let x := cswap x 2 3
  let x := cswap x 3 4
  -- pass 5: j = 0..2
  let x := cswap x 0 1; let x := cswap x 1 2; let x := cswap x 2 3
  -- pass 6: j = 0..1
  let x := cswap x 0 1; let x := cswap x 1 2
  -- pass 7: j = 0
  cswap x 0 1

/-- Model of sort8-network: the 19-comparator size-optimal sorting network
for 8 inputs. -/
def sortNetwork (x : BitVec 64) : BitVec 64 :=
  let x := cswap x 0 1; let x := cswap x 2 3; let x := cswap x 4 5; let x := cswap x 6 7
  let x := cswap x 0 2; let x := cswap x 1 3; let x := cswap x 4 6; let x := cswap x 5 7
  let x := cswap x 1 2; let x := cswap x 5 6; let x := cswap x 0 4; let x := cswap x 3 7
  let x := cswap x 1 5; let x := cswap x 2 6
  let x := cswap x 1 4; let x := cswap x 3 6
  let x := cswap x 2 4; let x := cswap x 3 5
  cswap x 3 4

/-- ANV-003 admission proof for sort8-network: the 19-comparator network
agrees with the bubble-sort spec on every input. -/
theorem network_refines (x : BitVec 64) : sortNetwork x = sortBubble x := by
  simp only [sortNetwork, sortBubble, cswap, lane, setLane]
  bv_decide (config := { timeout := 300 })

/-- Instance checks (challenge-window certificates). -/
example : sortBubble 0x0102030405060708 = 0x0807060504030201 := by decide
example : sortNetwork 0x0102030405060708 = 0x0807060504030201 := by decide
example : sortBubble 0x00ff00ff00ff00ff = 0xffffffff00000000 := by decide

end Razor.Anvil
