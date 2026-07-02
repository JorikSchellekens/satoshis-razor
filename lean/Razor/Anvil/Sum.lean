import Razor.Gauss

/-!
Anvil challenge ANV-002: sum of 1..n.

Validity predicate: n < 2^32, so all arithmetic fits in u64 without wrapping
and the Nat-level models below are faithful to the Rust implementations
(anvil/impls/sum-loop, anvil/impls/sum-closed). The champion submission
replaces the O(n) loop with the branch-free-of-overflow closed form: since one
of n, n+1 is even, the division happens on the even factor and never truncates.
-/

namespace Razor.Anvil

open Razor

/-- Executable spec / model of sum-loop: the accumulation loop. -/
def sumLoopModel (n : Nat) : Nat := sumTo n

/-- Model of sum-closed: Gauss's closed form, dividing the even factor. -/
def sumClosedModel (n : Nat) : Nat :=
  if n % 2 = 0 then (n / 2) * (n + 1) else n * ((n + 1) / 2)

/-- ANV-002 admission proof for sum-closed: the closed form refines the spec
on every valid input (in fact on every Nat). -/
theorem closed_refines (n : Nat) : sumClosedModel n = sumLoopModel n := by
  have hg := gauss n
  have h2 : 2 * sumClosedModel n = n * (n + 1) := by
    unfold sumClosedModel
    split
    · have hdiv : n / 2 * 2 = n := Nat.div_mul_cancel (Nat.dvd_of_mod_eq_zero ‹n % 2 = 0›)
      calc 2 * (n / 2 * (n + 1)) = n / 2 * 2 * (n + 1) := by
            rw [← Nat.mul_assoc, Nat.mul_comm 2 (n / 2)]
        _ = n * (n + 1) := by rw [hdiv]
    · have hodd : (n + 1) % 2 = 0 := by omega
      have hdiv : (n + 1) / 2 * 2 = n + 1 := Nat.div_mul_cancel (Nat.dvd_of_mod_eq_zero hodd)
      calc 2 * (n * ((n + 1) / 2)) = n * ((n + 1) / 2 * 2) := by
            rw [Nat.mul_comm 2 (n * ((n + 1) / 2)), Nat.mul_assoc]
        _ = n * (n + 1) := by rw [hdiv]
  unfold sumLoopModel
  omega

/-- Instance checks. -/
example : sumClosedModel 100 = 5050 := by decide
example : sumClosedModel 101 = 5151 := by decide

end Razor.Anvil
