import Razor

namespace Razor.Demo.MergeCount

open Razor.Sorting

theorem merge_count_solution (a : Nat) (l₁ l₂ : List Nat) :
    count a (merge l₁ l₂) = count a l₁ + count a l₂ := by
  induction l₁ generalizing l₂ with
  | nil => simp [merge, count]
  | cons x xs ih =>
    induction l₂ with
    | nil => simp [merge, count]
    | cons y ys ih₂ =>
      simp only [merge]
      split
      · simp only [count, ih (y :: ys)]
        omega
      · simp only [count] at ih₂ ⊢
        omega

end Razor.Demo.MergeCount
