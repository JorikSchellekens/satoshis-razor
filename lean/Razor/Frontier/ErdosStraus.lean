/-!
The Erdos-Straus conjecture, stated in core Lean by clearing denominators.

The usual statement says: for every integer n >= 2 there are positive
integers x, y, z with 4/n = 1/x + 1/y + 1/z. Multiplying both sides by
n*x*y*z turns this into an equation between natural numbers, which needs no
rational-number library:

    4*x*y*z = n*(y*z + x*z + x*y)

The two forms are equivalent for positive x, y, z; the Mathlib environment
(lean-mathlib/) carries the rational-number form, and proving the two
statements equivalent is itself a registered convergence task.

This is a real open problem (open since 1948). Nothing here proves it; this
file pins the statement and its machine-checked certificates.
-/

namespace Razor.Frontier

/-- Erdos-Straus over the naturals, denominators cleared. -/
def ErdosStraus : Prop :=
  ∀ n : Nat, 2 ≤ n → ∃ x y z : Nat,
    0 < x ∧ 0 < y ∧ 0 < z ∧ 4 * x * y * z = n * (y * z + x * z + x * y)

/-- Instance-check certificate: the n = 2 case, witnessed by 1, 2, 2
(that is, 4/2 = 1/1 + 1/2 + 1/2). -/
theorem erdosStraus_case_two : ∃ x y z : Nat,
    0 < x ∧ 0 < y ∧ 0 < z ∧ 4 * x * y * z = 2 * (y * z + x * z + x * y) :=
  ⟨1, 2, 2, by decide⟩

/-- Instance-check certificate: the n = 3 case, witnessed by 1, 4, 12
(4/3 = 1/1 + 1/4 + 1/12). -/
theorem erdosStraus_case_three : ∃ x y z : Nat,
    0 < x ∧ 0 < y ∧ 0 < z ∧ 4 * x * y * z = 3 * (y * z + x * z + x * y) :=
  ⟨1, 4, 12, by decide⟩

/-- Non-vacuity certificate: the hypothesis 2 <= n is satisfiable. -/
theorem erdosStraus_nonvacuous : ∃ n : Nat, 2 ≤ n := ⟨2, by decide⟩

end Razor.Frontier
