import Razor.Frontier.ErdosStraus

namespace AuditR3

/-- Erdos-Straus, my own reading: for every natural n >= 2 there are
positive naturals x, y, z with 4xyz = n(xy + yz + zx) - the cleared
denominator form of 4/n = 1/x + 1/y + 1/z. -/
def ErdosStrausR3 : Prop :=
  ∀ n : Nat, 2 ≤ n → ∃ x y z : Nat,
    0 < x ∧ 0 < y ∧ 0 < z ∧
    4 * (x * y * z) = n * (x * y + y * z + z * x)

/-- The two readings are the same statement up to reassociation and
commutativity of + and *. -/
theorem bridge_es : AuditR3.ErdosStrausR3 ↔ Razor.Frontier.ErdosStraus := by
  constructor
  · intro h n hn
    obtain ⟨x, y, z, hx, hy, hz, he⟩ := h n hn
    refine ⟨x, y, z, hx, hy, hz, ?_⟩
    simpa [Nat.mul_assoc, Nat.mul_comm, Nat.mul_left_comm,
           Nat.add_comm, Nat.add_left_comm, Nat.add_assoc] using he
  · intro h n hn
    obtain ⟨x, y, z, hx, hy, hz, he⟩ := h n hn
    refine ⟨x, y, z, hx, hy, hz, ?_⟩
    simpa [Nat.mul_assoc, Nat.mul_comm, Nat.mul_left_comm,
           Nat.add_comm, Nat.add_left_comm, Nat.add_assoc] using he

end AuditR3
