import Razor.Anvil.Sort8

/-!
ANV-003: the Batcher-network lanes (sort8-batcher, sort8-simd). Split
from Sort8.lean so each module carries one SAT check: the registry's
verifier runs on machines with far less memory than a dev laptop.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384

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

end Razor.Anvil
