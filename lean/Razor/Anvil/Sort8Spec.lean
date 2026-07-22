import Razor.Anvil.Sort8

/-!
ANV-003: the reference program's own correctness certificates. Split
from Sort8.lean to keep the verifier's per-module memory bounded.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384

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
