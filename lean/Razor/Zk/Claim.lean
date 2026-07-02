import Razor.Zk.Soundness

/-!
The pinned meaning of a zk-routed hole.

A Groth16 proof against the sorted-witness circuit establishes knowledge of
a witness for one specific public commitment. This file states, in Lean,
exactly what that establishes: `SortedWitnessFor h` says the commitment `h`
opens to four 8-bit values that have a sorted rearrangement.

`commitN` is the circuit's commitment computed over the naturals modulo the
BLS12-381 scalar field order: absorb each value, then 64 rounds of
`s ↦ (s + c)^5 mod p` with the fixed public constants `c_i = (i + 1) * 7919`
(zk/src/circuit.rs, `commit`). All circuit values are 8-bit and the field
has ~2^255 elements, so nothing wraps; the field-to-integer transfer that
makes this identification rigorous is the open proposal PRP-200.
-/

namespace Razor.Zk

/-- Order of the BLS12-381 scalar field: the modulus every circuit value
lives under. -/
def frP : Nat :=
  52435875175126190479447740508185965837690552500527637822603658699938581184513

/-- One MiMC-style round: `s ↦ (s + c)^5 mod p`. -/
def mimcRound (s c : Nat) : Nat := ((s + c) ^ 5) % frP

/-- Absorb one value into the sponge state: add it, then run all 64 rounds
with the fixed public constants. -/
def absorb (s x : Nat) : Nat :=
  (List.range 64).foldl (fun st i => mimcRound st ((i + 1) * 7919)) ((s + x) % frP)

/-- The circuit's commitment to a list, over the naturals mod `frP`.
Matches `commit` in zk/src/circuit.rs value for value (the public input is
the little-endian decoding of the prover's hex output). -/
def commitN (xs : List Nat) : Nat := xs.foldl absorb 0

/-- What an admitted proof against the sorted-witness circuit establishes
for public commitment `h`: the commitment opens to four 8-bit values, and a
sorted rearrangement of those values exists. The prover moreover *knows*
the opening - knowledge is a property of the proof system, one step
stronger than this existential. -/
def SortedWitnessFor (h : Nat) : Prop :=
  ∃ x0 x1 x2 x3 : Nat,
    x0 < 256 ∧ x1 < 256 ∧ x2 < 256 ∧ x3 < 256 ∧
    commitN [x0, x1, x2, x3] = h ∧
    ∃ y0 y1 y2 y3 : Int,
      y0 ≤ y1 ∧ y1 ≤ y2 ∧ y2 ≤ y3 ∧
      ∀ v : Int, countZ v [y0, y1, y2, y3] = countZ v [(x0 : Int), (x1 : Int), (x2 : Int), (x3 : Int)]

end Razor.Zk
