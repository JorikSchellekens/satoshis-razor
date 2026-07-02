import Mathlib.Data.Rat.Defs
import Mathlib.Tactic.NormNum

/-!
Statements pinned against the Mathlib environment. Nothing here is proven;
these are the Props that hole solutions must inhabit. Certificates (instance
checks and the like) live alongside and ARE proven.
-/

namespace RazorMathlib

/-- Erdos-Straus, stated over the rationals in Mathlib's vocabulary:
for every integer n >= 2 there are positive integers x, y, z with
4/n = 1/x + 1/y + 1/z. -/
def ErdosStrausRat : Prop :=
  ∀ n : ℕ, 2 ≤ n → ∃ x y z : ℕ, 0 < x ∧ 0 < y ∧ 0 < z ∧
    (4 : ℚ) / n = 1 / x + 1 / y + 1 / z

/-- Instance-check certificate: the n = 2 case is witnessed by 1, 2, 2. -/
theorem erdosStrausRat_case_two :
    (4 : ℚ) / 2 = 1 / 1 + 1 / 2 + 1 / 2 := by norm_num

end RazorMathlib
