import Mathlib

/-!
Statements pinned against the Mathlib environment. Nothing here is proven;
these are the Props that hole solutions must inhabit. Certificates (instance
checks and the like) live alongside and ARE proven.

Conventions:
- Where Mathlib already defines the Prop (as it does for Fermat's Last
  Theorem), the hole pins Mathlib's own name and this file adds nothing:
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

/-!
Fermat's Last Theorem needs no definition here: Mathlib already states it.
The registry's mathlib-environment FLT holes pin Mathlib's own Props:

- `FermatLastTheorem` - the full theorem, the Prop the Imperial FLT
  project is filling.
- `FermatLastTheoremWith ℕ n` / `FermatLastTheoremFor n` - fixed-exponent
  cases, for splits.

A proof accepted by Mathlib closes the hole with no restatement and no
translation risk.
-/

end RazorMathlib
