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

/-! ### Batcher's odd-even merge network (models sort8-batcher / sort8-simd)

Same 19 comparators as the size-optimal network, arranged as: sort both
halves with two 5-comparator sorters, then odd-even merge. The sort8-simd
lane computes exactly this network per word - its batching is evaluation
strategy, not semantics. -/

/-- Model of sort8-batcher and (per word) sort8-simd. -/
def sortBatcher (x : BitVec 64) : BitVec 64 :=
  let x := cswap x 0 1; let x := cswap x 2 3; let x := cswap x 4 5; let x := cswap x 6 7
  let x := cswap x 0 2; let x := cswap x 1 3; let x := cswap x 4 6; let x := cswap x 5 7
  let x := cswap x 1 2; let x := cswap x 5 6
  let x := cswap x 0 4; let x := cswap x 1 5; let x := cswap x 2 6; let x := cswap x 3 7
  let x := cswap x 2 4; let x := cswap x 3 5
  let x := cswap x 1 2; let x := cswap x 3 4; let x := cswap x 5 6
  x

/-- Admission proof for the Batcher-network lanes: agreement with the
bubble-sort spec on every input. -/
theorem batcher_refines (x : BitVec 64) : sortBatcher x = sortBubble x := by
  simp only [sortBatcher, sortBubble, cswap, lane, setLane]
  bv_decide (config := { timeout := 300 })

/-! ### The SWAR lane (models sort8-swar)

One full-width per-byte comparison per Batcher layer, exchange by xor.
The borrow analysis lives in anvil/impls/sort8-swar; here it is simply
transliterated and settled by SAT. -/

/-- One Batcher layer on packed bytes: compare-exchange (l, l+d) for every
lane l carrying 0xFF in `m`, with `sh = 8*d`. -/
def swarLayer (x m : BitVec 64) (sh : Nat) : BitVec 64 :=
  let h : BitVec 64 := 0x8080808080808080#64
  let b := x >>> sh
  let t := (x ||| h) - (b &&& ~~~h)
  let ge := h &&& ((x &&& ~~~b) ||| (~~~(x ^^^ b) &&& t))
  let d := (x ^^^ b) &&& m &&& ((ge >>> 7) * 0xFF#64)
  x ^^^ d ^^^ (d <<< sh)

/-- Model of sort8-swar: Batcher's six layers, each one SWAR step. -/
def sortSwar (x : BitVec 64) : BitVec 64 :=
  let x := swarLayer x 0x00FF00FF00FF00FF#64 8
  let x := swarLayer x 0x0000FFFF0000FFFF#64 16
  let x := swarLayer x 0x0000FF000000FF00#64 8
  let x := swarLayer x 0x00000000FFFFFFFF#64 32
  let x := swarLayer x 0x00000000FFFF0000#64 16
  let x := swarLayer x 0x0000FF00FF00FF00#64 8
  x

/-- ANV-003 admission proof for sort8-swar: the packed evaluation agrees
with the bubble-sort spec on every input. -/
theorem swar_sort_refines (x : BitVec 64) : sortSwar x = sortBubble x := by
  simp only [sortSwar, swarLayer, sortBubble, cswap, lane, setLane]
  bv_decide (config := { timeout := 300 })

/-! ### The spec's own correctness

A refinement proof transfers trust to the reference; these two theorems
give the reference something intrinsic to be trusted FOR. Together they
say `sortBubble` really sorts: the output bytes ascend, and every byte
value occurs exactly as often in the output as in the input - nothing
reordered away, nothing invented. Every admitted lane inherits both
through its refinement proof. -/

/-- The spec sorts: adjacent output bytes ascend, on every input. -/
theorem sortBubble_sorted (x : BitVec 64) :
    lane (sortBubble x) 0 ≤ lane (sortBubble x) 1 ∧
    lane (sortBubble x) 1 ≤ lane (sortBubble x) 2 ∧
    lane (sortBubble x) 2 ≤ lane (sortBubble x) 3 ∧
    lane (sortBubble x) 3 ≤ lane (sortBubble x) 4 ∧
    lane (sortBubble x) 4 ≤ lane (sortBubble x) 5 ∧
    lane (sortBubble x) 5 ≤ lane (sortBubble x) 6 ∧
    lane (sortBubble x) 6 ≤ lane (sortBubble x) 7 := by
  simp only [sortBubble, cswap, lane, setLane]
  bv_decide (config := { timeout := 300 })

/-- 1 if byte lane `i` of `x` holds the value `v`, else 0. -/
def laneIs (x : BitVec 64) (i : Nat) (v : BitVec 8) : BitVec 8 :=
  bif lane x i == v.zeroExtend 64 then 1#8 else 0#8

/-- How many of the 8 byte lanes of `x` hold the value `v` (0..8, so an
8-bit count never wraps). -/
def countLanes (v : BitVec 8) (x : BitVec 64) : BitVec 8 :=
  laneIs x 0 v + laneIs x 1 v + laneIs x 2 v + laneIs x 3 v +
  laneIs x 4 v + laneIs x 5 v + laneIs x 6 v + laneIs x 7 v

/-- The spec permutes: for every input and every byte value, the output
holds that value exactly as often as the input does. -/
theorem sortBubble_perm (x : BitVec 64) (v : BitVec 8) :
    countLanes v (sortBubble x) = countLanes v x := by
  simp only [countLanes, laneIs, sortBubble, cswap, lane, setLane]
  bv_decide (config := { timeout := 300 })

end Razor.Anvil
