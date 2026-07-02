import Razor.Sorting.Defs

/-!
Statement hole STM-102: what does "sorted" mean?

Two authors independently formalized sortedness: `SortedChain` (adjacent
elements ordered) and `SortedPairs` (all index pairs ordered). The convergence
certificate below is the machine-checked proof that the two independent
formalizations agree - the strongest fidelity evidence the funnel can produce.
-/

namespace Razor.Sorting

/-- A chain-sorted list's head is a lower bound for its tail. -/
theorem head_le_of_chain {x : Nat} {l : List Nat} (h : SortedChain (x :: l)) :
    ∀ k, (hk : k < l.length) → x ≤ l[k] := by
  induction l generalizing x with
  | nil => intro k hk; cases hk
  | cons y ys ih =>
    intro k hk
    cases h with
    | cons hxy hyys =>
      cases k with
      | zero => simpa using hxy
      | succ k =>
        have := ih hyys k (by simpa using Nat.lt_of_succ_lt_succ hk)
        simpa using Nat.le_trans hxy this

/-- Chain sortedness is preserved by dropping the head. -/
theorem chain_tail {x : Nat} {l : List Nat} (h : SortedChain (x :: l)) :
    SortedChain l := by
  cases h with
  | single => exact .nil
  | cons _ h => exact h

/-- Convergence, direction 1: the adjacent-pairs formalization implies the
all-pairs formalization. -/
theorem sortedPairs_of_sortedChain {l : List Nat} (h : SortedChain l) :
    SortedPairs l := by
  induction l with
  | nil => intro i j hi _ _; cases hi
  | cons x xs ih =>
    intro i j hi hj hij
    cases i with
    | zero =>
      cases j with
      | zero => simp
      | succ j =>
        simpa using head_le_of_chain h j (by simpa using Nat.lt_of_succ_lt_succ hj)
    | succ i =>
      cases j with
      | zero => cases hij
      | succ j =>
        simpa using ih (chain_tail h) i j
          (Nat.lt_of_succ_lt_succ hi) (Nat.lt_of_succ_lt_succ hj)
          (Nat.le_of_succ_le_succ hij)

/-- Convergence, direction 2: the all-pairs formalization implies the
adjacent-pairs formalization. -/
theorem sortedChain_of_sortedPairs {l : List Nat} (h : SortedPairs l) :
    SortedChain l := by
  induction l with
  | nil => exact .nil
  | cons x xs ih =>
    cases xs with
    | nil => exact .single x
    | cons y ys =>
      refine .cons ?_ (ih ?_)
      · exact h 0 1 (by simp) (by simp) (by omega)
      · intro i j hi hj hij
        have := h (i + 1) (j + 1)
          (by simpa using Nat.succ_lt_succ hi) (by simpa using Nat.succ_lt_succ hj)
          (Nat.succ_le_succ hij)
        simpa only [List.getElem_cons_succ] using this

/-- STM-102 convergence certificate: the two independent formalizations of
sortedness are equivalent. -/
theorem sorted_convergence (l : List Nat) : SortedChain l ↔ SortedPairs l :=
  ⟨sortedPairs_of_sortedChain, sortedChain_of_sortedPairs⟩

end Razor.Sorting
