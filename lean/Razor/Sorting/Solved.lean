import Razor.Sorting.Defs
import Razor.Sorting.Subgoals
import Razor.Sorting.Glue

/-!
Sorry RZR-103v2 - SOLVED.

The dominant clump's statement of the sorting sorry: the function must produce a
sorted *permutation* of its input. Closed by composing the decomposition's
glue proof with the two solved subgoals.
-/

namespace Razor.Sorting

/-- The v2 statement, exactly as pinned by its sorry. -/
def V2Statement : Prop :=
  ∃ f : List Nat → List Nat, ∀ l, SortedChain (f l) ∧ Perm l (f l)

/-- Split DEC-103's glue, exactly in the split contract's shape: the two
child statements imply the parent statement as pinned. Provable - and
kernel-checked - while both children are still open. -/
theorem glue_v2
    (sub_a : ∀ x {l : List Nat}, SortedChain l → SortedChain (insert x l))
    (sub_b : ∀ a x (l : List Nat),
      count a (insert x l) = (if x = a then 1 else 0) + count a l) :
    V2Statement :=
  ⟨isort, glue sub_a sub_b⟩

/-- RZR-103v2: insertion sort, with correctness assembled from the
decomposition graph. -/
theorem v2_solution : V2Statement :=
  ⟨isort, glue (fun x _ h => insert_sorted x h) insert_count⟩

end Razor.Sorting
