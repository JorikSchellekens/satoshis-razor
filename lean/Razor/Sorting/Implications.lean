import Razor.Sorting.ExploitV1
import Razor.Sorting.Solved
import Razor.Sorting.Convergence

/-!
The machine-checked relations between the sorting proposal's candidate
statements. These are the edges the registry's clumps are built from:

- `v2_convergence`: alice's and bob's independent formalizations of
  "a correct sorting function exists" are equivalent, so they form one
  clump with two independent members.
- `v2_implies_v1`: the clump's statement implies dave's original; the
  converse fails in the strongest possible way (v1 has a two-line proof,
  `v1_exploited`, while v2 needs a real sorting function). Together these
  place v1 strictly below v2 - the mechanical form of "v1 was a weaker
  reading of the proposal".
-/

namespace Razor.Sorting

/-- bob's formalization of the corrected statement, written against his
`SortedPairs` (indexed) definition of sortedness rather than alice's
inductive `SortedChain`. -/
def V2StatementPairs : Prop :=
  ∃ f : List Nat → List Nat, ∀ l, SortedPairs (f l) ∧ Perm l (f l)

/-- Convergence: the two independent formalizations are equivalent. -/
theorem v2_convergence : V2Statement ↔ V2StatementPairs := by
  constructor
  · intro ⟨f, h⟩
    exact ⟨f, fun l => ⟨(sorted_convergence (f l)).mp (h l).1, (h l).2⟩⟩
  · intro ⟨f, h⟩
    exact ⟨f, fun l => ⟨(sorted_convergence (f l)).mpr (h l).1, (h l).2⟩⟩

/-- The corrected statement implies the original: any sorted-permutation
function is in particular a sorted-output function. -/
theorem v2_implies_v1 : V2Statement → V1Statement :=
  fun ⟨f, h⟩ => ⟨f, fun l => (h l).1⟩

end Razor.Sorting
