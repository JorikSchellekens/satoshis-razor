import Razor

/-!
Nina's private solution to RZR-106 (isort is idempotent).

This file lives on nina's machine. She commits sha256(file || salt) to the
registry for a priority timestamp, and reveals the file only when ready -
nobody can front-run a hash. On reveal the registry re-hashes the file,
checks the commitment, installs it as Razor.Private.<submission>, and runs
the ordinary verifier.
-/

namespace Razor.Private.Nina

open Razor.Sorting

/-- Sorting a sorted list is the identity. -/
theorem isort_of_sorted {l : List Nat} (h : SortedChain l) : isort l = l := by
  induction h with
  | nil => rfl
  | single x => rfl
  | @cons a b ys hab hchain ih =>
    simp only [isort] at ih ⊢
    rw [ih]
    simp only [Razor.Sorting.insert]
    split
    · rfl
    · exact absurd hab ‹¬a ≤ b›

/-- RZR-106: insertion sort is idempotent. -/
theorem isort_idempotent (l : List Nat) : isort (isort l) = isort l :=
  isort_of_sorted (glue (fun x _ h => insert_sorted x h) insert_count l).1

end Razor.Private.Nina
