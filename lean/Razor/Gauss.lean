/-!
Hole RZR-001 (solved): the Gauss sum formula.

Informal conjecture: the sum of the first n natural numbers is n(n+1)/2.
Formalized without division to avoid truncation subtleties: 2 * sumTo n = n * (n + 1).
-/

namespace Razor

/-- Sum of `0 + 1 + ... + n`. -/
def sumTo : Nat → Nat
  | 0 => 0
  | n + 1 => sumTo n + (n + 1)

/-- RZR-001: Gauss's formula, stated multiplicatively. -/
theorem gauss (n : Nat) : 2 * sumTo n = n * (n + 1) := by
  induction n with
  | zero => rfl
  | succ n ih =>
    simp only [sumTo, Nat.mul_add, ih, Nat.add_mul, Nat.mul_add]
    omega

end Razor
