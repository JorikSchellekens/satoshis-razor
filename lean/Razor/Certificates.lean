import Razor.Gauss
import Razor.Sorting.Defs
import Razor.Sorting.ExploitV1

/-!
Challenge-window certificates.

Machine-checkable evidence accumulated while a candidate statement waits for
clump formation. These cannot prove fidelity, but each one closes off a class of
silent formalization bugs.
-/

namespace Razor.Certificates

open Razor Razor.Sorting

/-- Non-vacuity for STM-102: sorted lists exist (and not just trivially). -/
theorem sorted_nonvacuous : ∃ l : List Nat, l.length ≥ 3 ∧ SortedChain l :=
  ⟨[1, 2, 3], by decide, .cons (by omega) (.cons (by omega) (.single 3))⟩

/-- Falsifiability for STM-102: unsorted lists exist, so `SortedChain` is not
a vacuously true predicate. -/
theorem sorted_falsifiable : ∃ l : List Nat, ¬ SortedChain l := by
  refine ⟨[2, 1], fun h => ?_⟩
  cases h with
  | cons h _ => omega

/-- Instance check for RZR-001: the Gauss formula on a concrete case. -/
example : sumTo 100 = 5050 := by decide

/-- Instance check: `count` behaves as expected on a concrete list. -/
example : count 2 [1, 2, 2, 3] = 2 := by decide

/-- The certificate that would have killed RZR-103 v1 during its challenge
window, had anyone run it: the statement is *trivial* - provable by a function
that does no sorting. In the live funnel this proof arrived as a paid
submission instead (see ExploitV1.lean); the exploit-as-audit rule made its
discovery profitable rather than optional. -/
theorem v1_triviality_certificate : V1Statement := v1_exploited

end Razor.Certificates
