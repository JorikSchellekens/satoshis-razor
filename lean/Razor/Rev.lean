/-!
Hole RZR-002 (solved): reversal is an involution.

Deliberately proved for a from-scratch accumulator-based reversal rather than
`List.reverse` (for which core already ships the lemma) - the non-triviality
certificate for this hole is precisely that the statement is about a fresh
definition, not a library restatement.
-/

namespace Razor

/-- Accumulator-based list reversal. -/
def revAux (acc : List α) : List α → List α
  | [] => acc
  | x :: xs => revAux (x :: acc) xs

def rev (l : List α) : List α := revAux [] l

theorem revAux_eq (l acc : List α) : revAux acc l = revAux [] l ++ acc := by
  induction l generalizing acc with
  | nil => rfl
  | cons x xs ih =>
    simp only [revAux]
    rw [ih (x :: acc), ih [x]]
    simp

theorem rev_append_singleton (l : List α) (x : α) :
    revAux [] (l ++ [x]) = x :: revAux [] l := by
  induction l with
  | nil => rfl
  | cons y ys ih =>
    simp only [List.cons_append, revAux]
    rw [revAux_eq, ih, revAux_eq (acc := [y])]
    simp

/-- RZR-002: reversing twice is the identity. -/
theorem rev_rev (l : List α) : rev (rev l) = l := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    simp only [rev, revAux] at ih ⊢
    rw [revAux_eq xs [x], rev_append_singleton, ih]

end Razor
