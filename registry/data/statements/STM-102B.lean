-- bob's sealed reading of PRP-100 (registered as Razor.Sorting.V2StatementPairs)
def V2StatementPairs : Prop :=
  ∃ f : List Nat → List Nat, ∀ l : List Nat, SortedPairs (f l) ∧ Perm l (f l)
