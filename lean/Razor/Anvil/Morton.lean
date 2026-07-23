import Std.Tactic.BVDecide

/-!
Anvil challenge ANV-008: Morton (Z-order) interleave of the two 32-bit
halves of a u64.

The executable specification is `mortonNaive`: take one bit from each half
per step, 32 steps, low half onto the even positions, high half onto the
odd (anvil/impls/morton-naive). The model walks from the top bit down
with shifting working copies so every shift amount is a literal.

Two contender models:

- `mortonSwar` (anvil/impls/morton-swar): five doubling spread steps per
  half - 16-bit groups, then 8, 4, 2, 1.
- `mortonPdep` (anvil/impls/morton-pdep): the semantics of x86-64 BMI2
  `pdep` - a 64-step deposit walk - applied to the constant even/odd
  masks. What the proof checks is the instruction's documented behavior
  against the naive interleave, so the lane leaning on the instruction is
  admitted on the instruction's actual contract.

Both admission proofs are settled by `bv_decide` over all 2^64 inputs.
-/

namespace Razor.Anvil

set_option maxRecDepth 16384
set_option maxHeartbeats 2000000

/-- One interleave step from the top: shift the next bit of each working
copy into the bottom of the accumulator. -/
def mortonAux : Nat → BitVec 64 → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, _, out => out
  | k + 1, lo, hi, out =>
    let pair := (((hi >>> 63) &&& 1#64) <<< 1) ||| ((lo >>> 63) &&& 1#64)
    mortonAux k (lo <<< 1) (hi <<< 1) ((out <<< 2) ||| pair)

/-- Executable spec: interleave the low half onto even bit positions and
the high half onto odd ones (model of morton-naive). The working copies
start with each half's bit 31 at the top. -/
def mortonNaive (x : BitVec 64) : BitVec 64 :=
  mortonAux 32 (x <<< 32) (x &&& 0xFFFFFFFF00000000#64) 0#64

/-- Five doubling spread steps: place the low 32 bits at even positions. -/
def mortonSpread (v : BitVec 64) : BitVec 64 :=
  let v := v &&& 0x00000000FFFFFFFF#64
  let v := (v ||| (v <<< 16)) &&& 0x0000FFFF0000FFFF#64
  let v := (v ||| (v <<< 8)) &&& 0x00FF00FF00FF00FF#64
  let v := (v ||| (v <<< 4)) &&& 0x0F0F0F0F0F0F0F0F#64
  let v := (v ||| (v <<< 2)) &&& 0x3333333333333333#64
  (v ||| (v <<< 1)) &&& 0x5555555555555555#64

/-- Model of morton-swar: spread each half, offset the high half by one. -/
def mortonSwar (x : BitVec 64) : BitVec 64 :=
  mortonSpread x ||| (mortonSpread (x >>> 32) <<< 1)

/-- One step of the `pdep` deposit walk: consume a mask bit; if it is set,
deposit the next source bit there. Positions are tracked by shifting the
accumulator down and inserting at the top, so after 64 steps each
deposited bit sits exactly at its mask bit's position. -/
def pdepAux : Nat → BitVec 64 → BitVec 64 → BitVec 64 → BitVec 64
  | 0, _, _, out => out
  | k + 1, src, mask, out =>
    let take := mask &&& 1#64 == 1#64
    let out := (out >>> 1) ||| (bif take then (src &&& 1#64) <<< 63 else 0#64)
    let src := bif take then src >>> 1 else src
    pdepAux k src (mask >>> 1) out

/-- The BMI2 `pdep` instruction: deposit the low bits of `src` at the
positions of the set bits of `mask`. -/
def pdep (src mask : BitVec 64) : BitVec 64 :=
  pdepAux 64 src mask 0#64

/-- Model of morton-pdep: one deposit onto the even positions, one onto
the odd. -/
def mortonPdep (x : BitVec 64) : BitVec 64 :=
  pdep x 0x5555555555555555#64 ||| pdep (x >>> 32) 0xAAAAAAAAAAAAAAAA#64

/-- ANV-008 admission proof for morton-swar: the spread steps agree with
the one-bit-at-a-time interleave on every input. -/
theorem morton_swar_refines (x : BitVec 64) : mortonSwar x = mortonNaive x := by
  simp (config := { maxSteps := 4000000 }) only [mortonSwar, mortonSpread, mortonNaive, mortonAux]
  bv_decide (config := { timeout := 300 })

/-- ANV-008 admission proof for morton-pdep: depositing onto the even and
odd masks agrees with the one-bit-at-a-time interleave on every input. -/
theorem morton_pdep_refines (x : BitVec 64) : mortonPdep x = mortonNaive x := by
  simp (config := { maxSteps := 4000000 }) only [mortonPdep, pdep, pdepAux, mortonNaive, mortonAux]
  bv_decide (config := { timeout := 300 })

/-- Instance checks (challenge-window certificates). -/
example : mortonNaive 0x00000000FFFFFFFF#64 = 0x5555555555555555#64 := by decide
example : mortonNaive 0xFFFFFFFF00000000#64 = 0xAAAAAAAAAAAAAAAA#64 := by decide
example : mortonNaive 0x0000000100000001#64 = 3#64 := by decide

end Razor.Anvil
