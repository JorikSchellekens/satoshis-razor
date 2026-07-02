import Razor.Sorting.Defs

/-!
Decomposition DEC-103: hole RZR-103v2 split into two subgoals.

Anyone may decompose a hole by posting subgoals plus a machine-checked glue
proof (see Glue.lean). These are the two subgoals; each was solved
independently and is attributed independently.
-/

namespace Razor.Sorting

/-- Subgoal RZR-103v2.a: insertion preserves chain-sortedness. -/
theorem insert_sorted (x : Nat) {l : List Nat} (h : SortedChain l) :
    SortedChain (insert x l) := by
  induction h with
  | nil => exact .single x
  | single y =>
    simp only [insert]
    split
    · exact .cons ‹x ≤ y› (.single y)
    · exact .cons (Nat.le_of_not_le ‹¬x ≤ y›) (.single x)
  | @cons a b ys hab hchain ih =>
    simp only [insert]
    split
    · exact .cons ‹x ≤ a› (.cons hab hchain)
    · simp only [insert] at ih ⊢
      by_cases hxb : x ≤ b
      · simp only [if_pos hxb] at ih ⊢
        exact .cons (Nat.le_of_not_le ‹¬x ≤ a›) ih
      · simp only [if_neg hxb] at ih ⊢
        exact .cons hab ih

/-- Subgoal RZR-103v2.b: insertion adds exactly one occurrence of the inserted
element and preserves all other counts. -/
theorem insert_count (a x : Nat) (l : List Nat) :
    count a (insert x l) = (if x = a then 1 else 0) + count a l := by
  induction l with
  | nil => simp [insert, count]
  | cons y ys ih =>
    simp only [insert]
    split
    · simp [count]
    · simp only [count, ih]
      omega

end Razor.Sorting
