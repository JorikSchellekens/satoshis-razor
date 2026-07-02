/-!
The sorting saga: shared definitions.

Proposal PRP-100: "there is a provably correct sorting function for lists of
naturals". Everything downstream of that proposal - the botched v1 statement, the
two independent Sorted formalizations, the convergent v2, its decomposition, and the
final solution - lives under `Razor.Sorting`.
-/

namespace Razor.Sorting

/-- Occurrence count of `x` in a list. The permutation relation is defined
through counts, keeping the development independent of any library multiset. -/
def count (x : Nat) : List Nat → Nat
  | [] => 0
  | y :: ys => (if y = x then 1 else 0) + count x ys

/-- Two lists are permutations of each other iff every value occurs equally often. -/
def Perm (l₁ l₂ : List Nat) : Prop := ∀ x, count x l₁ = count x l₂

/-- Formalization A of sortedness (statement hole STM-102, author A):
each element is at most its successor. -/
inductive SortedChain : List Nat → Prop
  | nil : SortedChain []
  | single (x : Nat) : SortedChain [x]
  | cons {x y : Nat} {ys : List Nat} :
      x ≤ y → SortedChain (y :: ys) → SortedChain (x :: y :: ys)

/-- Formalization B of sortedness (statement hole STM-102, author B, independent):
every pair of positions is ordered. -/
def SortedPairs (l : List Nat) : Prop :=
  ∀ i j, (hi : i < l.length) → (hj : j < l.length) → i ≤ j → l[i] ≤ l[j]

/-- Insertion into a sorted list. -/
def insert (x : Nat) : List Nat → List Nat
  | [] => [x]
  | y :: ys => if x ≤ y then x :: y :: ys else y :: insert x ys

/-- Insertion sort: the submitted implementation for hole RZR-103v2. -/
def isort : List Nat → List Nat
  | [] => []
  | x :: xs => insert x (isort xs)

end Razor.Sorting
