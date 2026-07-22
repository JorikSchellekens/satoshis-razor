import Razor.Sorting.Defs

/-!
The open frontier.

Sorries that are registered and unsolved. Each `sorry` here is a
target: fill it, submit the file, and the registry verifier checks it against
the pinned statement with no sorry and no extra axioms.
-/

namespace Razor.Sorting

/-- Merge of two lists (part of the RZR-104 challenge context). -/
def merge : List Nat → List Nat → List Nat
  | [], ys => ys
  | xs, [] => xs
  | x :: xs, y :: ys =>
    if x ≤ y then x :: merge xs (y :: ys) else y :: merge (x :: xs) ys

/-- RZR-104 (OPEN): merging two sorted lists yields a sorted list.
Decomposed subgoal of the merge-sort correctness proposal PRP-104. -/
theorem merge_sorted {l₁ l₂ : List Nat}
    (h₁ : SortedChain l₁) (h₂ : SortedChain l₂) :
    SortedChain (merge l₁ l₂) := by
  sorry

/-- RZR-105 (OPEN): merge preserves element counts. -/
theorem merge_count (a : Nat) (l₁ l₂ : List Nat) :
    count a (merge l₁ l₂) = count a l₁ + count a l₂ := by
  sorry

/-- RZR-106 (OPEN): insertion sort is idempotent. -/
theorem isort_idempotent (l : List Nat) : isort (isort l) = isort l := by
  sorry

end Razor.Sorting
