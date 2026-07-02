/-!
ZK circuit soundness - the zkGolf trust chain.

The Groth16 proof (zk/src/circuit.rs) attests that the R1CS constraints hold
for some witness. This file proves what those constraints *mean*: any witness
satisfying a comparator's constraints computes a genuine (min, max), and any
witness satisfying the 5-comparator network's constraints yields a sorted
permutation of the inputs. Together: "proof verifies" implies "the prover
knows a sorted arrangement of the committed list".

The model is over Int with the exact constraint shapes of the circuit. The
circuit operates in a ~2^255 field with all values range-checked below 2^8,
so no arithmetic wraps and the field constraints coincide with these integer
ones; that field-to-integer transfer is the remaining formalization gap,
registered as an open proposal (PRP-200) in the registry.
-/

namespace Razor.Zk

/-- Occurrence count over integer lists (the permutation measure). -/
def countZ (x : Int) : List Int → Nat
  | [] => 0
  | y :: ys => (if y = x then 1 else 0) + countZ x ys

/-- One comparator's constraints, exactly as enforced in R1CS:
boolean select, min-wiring, sum conservation, boolean range bits, and the
8-bit decomposition of `hi - lo`. -/
def Cmp (a b lo hi : Int) : Prop :=
  ∃ s d0 d1 d2 d3 d4 d5 d6 d7 : Int,
    s * (s - 1) = 0 ∧
    s * (b - a) = lo - a ∧
    a + b = lo + hi ∧
    d0 * (d0 - 1) = 0 ∧ d1 * (d1 - 1) = 0 ∧ d2 * (d2 - 1) = 0 ∧ d3 * (d3 - 1) = 0 ∧
    d4 * (d4 - 1) = 0 ∧ d5 * (d5 - 1) = 0 ∧ d6 * (d6 - 1) = 0 ∧ d7 * (d7 - 1) = 0 ∧
    hi - lo = d0 + 2 * d1 + 4 * d2 + 8 * d3 + 16 * d4 + 32 * d5 + 64 * d6 + 128 * d7

/-- A product being zero forces one factor to vanish (specialized to the
boolean constraint shape). -/
private theorem bool_of_sq {x : Int} (h : x * (x - 1) = 0) : x = 0 ∨ x = 1 := by
  rcases Int.mul_eq_zero.mp h with h | h <;> omega

/-- Comparator soundness: the constraints force `(lo, hi)` to be the ordered
rearrangement of `(a, b)`. -/
theorem comparator_sound {a b lo hi : Int} (h : Cmp a b lo hi) :
    lo ≤ hi ∧ ((lo = a ∧ hi = b) ∨ (lo = b ∧ hi = a)) := by
  obtain ⟨s, d0, d1, d2, d3, d4, d5, d6, d7,
    hs, hlo, hsum, h0, h1, h2, h3, h4, h5, h6, h7, hr⟩ := h
  have hb0 := bool_of_sq h0
  have hb1 := bool_of_sq h1
  have hb2 := bool_of_sq h2
  have hb3 := bool_of_sq h3
  have hb4 := bool_of_sq h4
  have hb5 := bool_of_sq h5
  have hb6 := bool_of_sq h6
  have hb7 := bool_of_sq h7
  rcases bool_of_sq hs with h | h <;> subst h <;> simp at hlo <;> omega

/-- Comparator preserves multiplicities. -/
theorem comparator_count {a b lo hi : Int} (h : Cmp a b lo hi) (v : Int) :
    countZ v [lo, hi] = countZ v [a, b] := by
  rcases (comparator_sound h).2 with ⟨ha, hb⟩ | ⟨ha, hb⟩ <;> subst ha <;> subst hb <;>
    simp only [countZ] <;> omega

/-- Network soundness. Wire flow of the optimal 5-comparator network on
4 wires, with comparators (0,1), (2,3), (0,2), (1,3), (2,3 after routing) =
NETWORK in zk/src/circuit.rs; final wires are [b0, c1, c2, b3].

Any witness satisfying all five comparators' constraints exhibits a sorted
permutation of the inputs. -/
theorem network_sound
    {x0 x1 x2 x3 a0 a1 a2 a3 b0 b1 b2 b3 c1 c2 : Int}
    (h1 : Cmp x0 x1 a0 a1) (h2 : Cmp x2 x3 a2 a3)
    (h3 : Cmp a0 a2 b0 b2) (h4 : Cmp a1 a3 b1 b3)
    (h5 : Cmp b1 b2 c1 c2) :
    (b0 ≤ c1 ∧ c1 ≤ c2 ∧ c2 ≤ b3) ∧
    ∀ v, countZ v [b0, c1, c2, b3] = countZ v [x0, x1, x2, x3] := by
  have s1 := comparator_sound h1
  have s2 := comparator_sound h2
  have s3 := comparator_sound h3
  have s4 := comparator_sound h4
  have s5 := comparator_sound h5
  constructor
  · rcases s1.2 with ⟨e1, e1'⟩ | ⟨e1, e1'⟩ <;>
    rcases s2.2 with ⟨e2, e2'⟩ | ⟨e2, e2'⟩ <;>
    rcases s3.2 with ⟨e3, e3'⟩ | ⟨e3, e3'⟩ <;>
    rcases s4.2 with ⟨e4, e4'⟩ | ⟨e4, e4'⟩ <;>
    rcases s5.2 with ⟨e5, e5'⟩ | ⟨e5, e5'⟩ <;>
    (have t1 := s1.1; have t2 := s2.1; have t3 := s3.1; have t4 := s4.1; have t5 := s5.1;
     omega)
  · intro v
    have p1 := comparator_count h1 v
    have p2 := comparator_count h2 v
    have p3 := comparator_count h3 v
    have p4 := comparator_count h4 v
    have p5 := comparator_count h5 v
    simp only [countZ] at p1 p2 p3 p4 p5 ⊢
    omega

end Razor.Zk
