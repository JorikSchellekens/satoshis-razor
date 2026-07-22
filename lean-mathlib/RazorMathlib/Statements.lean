import Mathlib

/-!
Statements pinned against the Mathlib environment. Nothing here is proven;
these are the Props that sorry solutions must inhabit. Certificates (instance
checks and the like) live alongside and ARE proven.

Conventions:
- Where Mathlib already defines the Prop (as it does for Fermat's Last
  Theorem), the sorry pins Mathlib's own name and this file adds nothing:
  a proof written for Mathlib is, verbatim, a proof for the registry.
- Where Mathlib has no name for the problem, the Prop is defined here
  using Mathlib's definitions, following the statement conventions of
  google-deepmind/formal-conjectures where an entry exists.
- The umbrella `import Mathlib` is deliberate: statements must not rot when
  Mathlib reorganizes its module tree. Pinning to exact modules is a build
  optimization the registry does not need.
-/

namespace RazorMathlib

/-- Erdos-Straus, stated over the rationals with Mathlib's definitions:
for every integer n >= 2 there are positive integers x, y, z with
4/n = 1/x + 1/y + 1/z. -/
def ErdosStrausRat : Prop :=
  ∀ n : ℕ, 2 ≤ n → ∃ x y z : ℕ, 0 < x ∧ 0 < y ∧ 0 < z ∧
    (4 : ℚ) / n = 1 / x + 1 / y + 1 / z

/-- Instance-check certificate: the n = 2 case is witnessed by 1, 2, 2. -/
theorem erdosStrausRat_case_two :
    (4 : ℚ) / 2 = 1 / 1 + 1 / 2 + 1 / 2 := by norm_num

/-- Erdos-Turan conjecture on additive bases (1941). If every sufficiently
large natural number is a sum of two elements of `A`, then the number of
representations `n = a + b` with `a, b ∈ A` cannot stay bounded. Ordered
pairs are counted; boundedness of ordered and unordered counts is
equivalent, so the choice is a convention, not a strengthening. -/
def ErdosTuranAdditiveBasis : Prop :=
  ∀ A : Set ℕ,
    (∀ᶠ n in Filter.atTop, ∃ p : ℕ × ℕ, p.1 ∈ A ∧ p.2 ∈ A ∧ p.1 + p.2 = n) →
    ∀ C : ℕ, ∃ n : ℕ,
      C < {p : ℕ × ℕ | p.1 ∈ A ∧ p.2 ∈ A ∧ p.1 + p.2 = n}.ncard

/-! ## Open conjectures across domains

Each `def` below is the Prop a sorry pins. Every statement follows the
literature's standard form; the gloss filed with each candidate statement
is the reading to compare against. -/

/-- Goldbach's conjecture (1742): every even number greater than 2 is the
sum of two primes. -/
def Goldbach : Prop :=
  ∀ n : ℕ, Even n → 4 ≤ n → ∃ p q : ℕ, p.Prime ∧ q.Prime ∧ p + q = n

/-- Instance-check certificate: 4 = 2 + 2. -/
theorem goldbach_case_four : Nat.Prime 2 ∧ 2 + 2 = 4 := by norm_num

/-- The twin prime conjecture: there are infinitely many primes `p` with
`p + 2` also prime. Stated as unboundedness, the standard rendering of
"infinitely many" over ℕ. -/
def TwinPrimes : Prop :=
  ∀ N : ℕ, ∃ p : ℕ, N ≤ p ∧ p.Prime ∧ (p + 2).Prime

/-- Legendre's conjecture: between consecutive squares there is always a
prime. -/
def LegendreConjecture : Prop :=
  ∀ n : ℕ, 0 < n → ∃ p : ℕ, p.Prime ∧ n ^ 2 < p ∧ p < (n + 1) ^ 2

/-- Landau's fourth problem: there are infinitely many primes of the form
`n ^ 2 + 1`. -/
def LandauNearSquarePrimes : Prop :=
  ∀ N : ℕ, ∃ n : ℕ, N ≤ n ∧ (n ^ 2 + 1).Prime

/-- Infinitude of Sophie Germain primes: infinitely many primes `p` with
`2 p + 1` also prime. -/
def SophieGermainPrimesInfinite : Prop :=
  ∀ N : ℕ, ∃ p : ℕ, N ≤ p ∧ p.Prime ∧ (2 * p + 1).Prime

/-- No odd perfect number exists. `Nat.Perfect` is Mathlib's own notion:
the proper divisors of `n` sum to `n`. -/
def NoOddPerfectNumber : Prop :=
  ¬ ∃ n : ℕ, Odd n ∧ Nat.Perfect n

/-- Brocard's problem: `n! + 1` is a perfect square only for
`n = 4, 5, 7`. (For `n ≤ 3` the values 2, 2, 3, 7 are not squares, so no
positivity guard is needed.) -/
def BrocardProblem : Prop :=
  ∀ n m : ℕ, n.factorial + 1 = m ^ 2 → n = 4 ∨ n = 5 ∨ n = 7

/-- Beal's conjecture: if `A^x + B^y = C^z` with `A, B, C` positive and
all exponents at least 3, then `A`, `B`, `C` share a prime factor. -/
def BealConjecture : Prop :=
  ∀ A B C x y z : ℕ, 0 < A → 0 < B → 0 < C → 3 ≤ x → 3 ≤ y → 3 ≤ z →
    A ^ x + B ^ y = C ^ z → ∃ p : ℕ, p.Prime ∧ p ∣ A ∧ p ∣ B ∧ p ∣ C

/-- The Collatz step: halve an even number, send an odd `n` to `3 n + 1`. -/
def collatzStep (n : ℕ) : ℕ := if n % 2 = 0 then n / 2 else 3 * n + 1

/-- The Collatz conjecture: iterating the step from any positive start
reaches 1. -/
def CollatzConjecture : Prop :=
  ∀ n : ℕ, 0 < n → ∃ k : ℕ, collatzStep^[k] n = 1

/-- Instance-check certificate: 6 reaches 1 in eight steps. -/
theorem collatz_case_six : collatzStep^[8] 6 = 1 := by decide

/-- Frankl's union-closed sets conjecture: a finite union-closed family
of finite sets with a nonempty member has an element that belongs to at
least half of the members. -/
def FranklUnionClosed : Prop :=
  ∀ F : Finset (Finset ℕ),
    (∀ A ∈ F, ∀ B ∈ F, A ∪ B ∈ F) →
    (∃ A ∈ F, A.Nonempty) →
    ∃ x : ℕ, F.card ≤ 2 * (F.filter (fun A => x ∈ A)).card

/-- Sendov's conjecture: if every root of a complex polynomial of degree
at least 2 lies in the closed unit disk, then within distance 1 of each
root there is a root of the derivative. -/
def SendovConjecture : Prop :=
  ∀ p : Polynomial ℂ, 2 ≤ p.natDegree →
    (∀ z ∈ p.roots, ‖z‖ ≤ 1) →
    ∀ z ∈ p.roots, ∃ w ∈ (Polynomial.derivative p).roots, ‖z - w‖ ≤ 1

/-- The Erdos-Gyarfas conjecture: every finite simple graph in which
every vertex has degree at least 3 contains a cycle whose length is a
power of two. -/
def ErdosGyarfas : Prop :=
  ∀ (V : Type) [Fintype V] (G : SimpleGraph V) [DecidableRel G.Adj],
    (∀ v : V, 3 ≤ G.degree v) →
    ∃ (v : V) (c : G.Walk v v), c.IsCycle ∧ ∃ k : ℕ, c.length = 2 ^ k

/-!
The Riemann hypothesis needs no definition here: Mathlib already states
it as `RiemannHypothesis`, and the sorry pins Mathlib's own name.
-/

/-!
Fermat's Last Theorem needs no definition here: Mathlib already states it.
The registry's mathlib-environment FLT sorries pin Mathlib's own Props:

- `FermatLastTheorem` - the full theorem, the Prop the Imperial FLT
  project is filling.
- `FermatLastTheoremWith ℕ n` / `FermatLastTheoremFor n` - fixed-exponent
  cases, for splits.

A proof accepted by Mathlib closes the sorry with no restatement and no
translation risk.
-/

end RazorMathlib
