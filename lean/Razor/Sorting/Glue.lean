import Razor.Sorting.Defs

/-!
Decomposition DEC-103: the glue proof.

A decomposition is only admitted to the registry with a machine-checked proof
that the subgoals jointly imply the parent. Note the glue takes the subgoal
*statements* as hypotheses: it was posted and verified before either subgoal
was solved, which is what makes permissionless decomposition trustless.
-/

namespace Razor.Sorting

/-- DEC-103 glue: the two subgoals imply RZR-103v2's body for `isort`. -/
theorem glue
    (sub_a : ∀ x {l : List Nat}, SortedChain l → SortedChain (insert x l))
    (sub_b : ∀ a x (l : List Nat),
      count a (insert x l) = (if x = a then 1 else 0) + count a l) :
    ∀ l, SortedChain (isort l) ∧ Perm l (isort l) := by
  intro l
  induction l with
  | nil => exact ⟨.nil, fun _ => rfl⟩
  | cons x xs ih =>
    refine ⟨sub_a x ih.1, fun a => ?_⟩
    have hcount := ih.2 a
    simp only [isort, count, sub_b]
    omega

end Razor.Sorting
