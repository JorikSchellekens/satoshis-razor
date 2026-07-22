-- alice's sealed reading of PRP-100 (registered as Razor.Sorting.V2Statement)
def V2Statement : Prop :=
  ∃ f : List Nat → List Nat, ∀ l : List Nat, SortedChain (f l) ∧ Perm l (f l)
