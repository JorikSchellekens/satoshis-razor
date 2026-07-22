import Razor.Sorries.Open

/-!
Statement migration (the repin demonstration).

A pinned statement can rot: the library it is written against renames
things, or a style refactor changes how the same Prop is spelled. The
registry's answer is `razor repin`: a sorry moves to a new wording only if
a proof that the two wordings are equivalent kernel-checks. This file
holds that equivalence for sorry RZR-104, whose original wording binds its
lists implicitly and whose refactored wording binds them explicitly.
-/

namespace Razor.Sorting

/-- RZR-104's two wordings - implicit and explicit list binders - are one
problem. `razor repin` kernel-checks exactly this iff before migrating
the sorry; the old wording, the new wording, and this proof all stay on
the log. -/
theorem merge_sorted_binder_equiv :
    (∀ (l₁ l₂ : List Nat), SortedChain l₁ → SortedChain l₂ →
        SortedChain (merge l₁ l₂)) ↔
    (∀ {l₁ l₂ : List Nat}, SortedChain l₁ → SortedChain l₂ →
        SortedChain (merge l₁ l₂)) := by
  constructor
  · intro h l₁ l₂
    exact h l₁ l₂
  · intro h l₁ l₂
    exact @h l₁ l₂

end Razor.Sorting
