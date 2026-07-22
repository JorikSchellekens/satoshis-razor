import Razor.Anvil.Sort8

/-!
ANV-003: the packed (SWAR) lane. Split from Sort8.lean to keep the
verifier's per-module memory bounded.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384

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

end Razor.Anvil
