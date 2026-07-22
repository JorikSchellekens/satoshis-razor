/-!
Fermat's Last Theorem, stated in core Lean, with two registered splits.

A split reduces a parent sorry to child sorries plus a glue theorem: a proof
that the children jointly imply the parent. The glue takes the child
*statements* as hypotheses, so it can be proven and kernel-checked while
every child is still open - that is what makes a split trustworthy before
any of the hard work is done.

Two splits of the same parent are registered here on purpose:

* Split A is the classical reduction: FLT for exponent 4, FLT for every odd
  prime exponent, and the arithmetic fact that every exponent >= 3 has a
  divisor that is 4 or an odd prime.

* Split B mirrors the actual FLT project at Imperial College London
  (https://github.com/ImperialCollegeLondon/FLT): the cases n = 3 and n = 4
  are already in Mathlib, and the Wiles/Taylor-Wiles machinery the project
  formalizes targets prime exponents >= 5 (blueprint, "Reduction to n >= 5
  and prime"). Split A's "every odd prime" child turns out to be the wrong
  cut for that strategy - the Frey curve argument needs p >= 5 - so Split B
  is the refactored decomposition. Split A is not deleted: its glue is a
  true theorem and stays one.
-/

namespace Razor.Frontier

/-- Fermat's Last Theorem over the naturals: for every exponent `n >= 3`
there are no positive `x`, `y`, `z` with `x^n + y^n = z^n`. -/
def FLT : Prop :=
  ∀ n x y z : Nat, 3 ≤ n → 0 < x → 0 < y → 0 < z → x ^ n + y ^ n ≠ z ^ n

/-- Fermat's Last Theorem for one fixed exponent `n`. -/
def FLTFor (n : Nat) : Prop :=
  ∀ x y z : Nat, 0 < x → 0 < y → 0 < z → x ^ n + y ^ n ≠ z ^ n

/-- `p` is an odd prime: greater than 2, and divisible only by 1 and itself. -/
def OddPrime (p : Nat) : Prop :=
  2 < p ∧ ∀ d, d ∣ p → d = 1 ∨ d = p

/-- `p` is a prime that is at least 5 - the exponents the FLT project's
modularity machinery actually targets. -/
def PrimeGE5 (p : Nat) : Prop :=
  5 ≤ p ∧ ∀ d, d ∣ p → d = 1 ∨ d = p

/-- Split A, child: FLT for every odd prime exponent. -/
def FLTOddPrimes : Prop := ∀ p, OddPrime p → FLTFor p

/-- Split B, child: FLT for every prime exponent >= 5. -/
def FLTPrimesGE5 : Prop := ∀ p, PrimeGE5 p → FLTFor p

/-- Split A, child: every exponent `n >= 3` factors as `k * m` where the
cofactor `m` is 4 or an odd prime. This is the arithmetic content of the
classical reduction, stated as its own sorry rather than buried in the glue. -/
def ExponentReduction : Prop :=
  ∀ n, 3 ≤ n → ∃ k m, n = k * m ∧ (m = 4 ∨ OddPrime m)

/-- Split B, child: every exponent `n >= 3` factors as `k * m` where the
cofactor `m` is 3, 4, or a prime >= 5. -/
def ExponentReductionFine : Prop :=
  ∀ n, 3 ≤ n → ∃ k m, n = k * m ∧ (m = 3 ∨ m = 4 ∨ PrimeGE5 m)

/-- The descent step both glues share: a counterexample for exponent
`k * m` is a counterexample for exponent `m`, because
`x ^ (k * m) = (x ^ k) ^ m`. -/
theorem fltFor_of_factor {k m : Nat} (h : FLTFor m) : FLTFor (k * m) := by
  intro x y z hx hy hz heq
  rw [Nat.pow_mul, Nat.pow_mul, Nat.pow_mul] at heq
  exact h (x ^ k) (y ^ k) (z ^ k)
    (Nat.pow_pos hx) (Nat.pow_pos hy) (Nat.pow_pos hz) heq

/-- Split A glue: exponent 4, the odd primes, and the exponent reduction
jointly give FLT. Kernel-checked while all three children are open. -/
theorem fltSplitA_glue : FLTFor 4 → FLTOddPrimes → ExponentReduction → FLT := by
  intro h4 hodd hred n x y z hn hx hy hz
  obtain ⟨k, m, hnkm, hm⟩ := hred n hn
  have hm' : FLTFor m := by
    cases hm with
    | inl h => exact h ▸ h4
    | inr h => exact hodd m h
  exact hnkm ▸ fltFor_of_factor hm' x y z hx hy hz

/-- Split B glue: exponents 3 and 4, the primes >= 5, and the finer
exponent reduction jointly give FLT. This is the FLT project's own
top-level decomposition. -/
theorem fltSplitB_glue :
    FLTFor 3 → FLTFor 4 → FLTPrimesGE5 → ExponentReductionFine → FLT := by
  intro h3 h4 hp5 hred n x y z hn hx hy hz
  obtain ⟨k, m, hnkm, hm⟩ := hred n hn
  have hm' : FLTFor m := by
    cases hm with
    | inl h => exact h ▸ h3
    | inr h =>
      cases h with
      | inl h => exact h ▸ h4
      | inr h => exact hp5 m h
  exact hnkm ▸ fltFor_of_factor hm' x y z hx hy hz

-- ── certificates ────────────────────────────────────────────────────

/-- Instance check for the FLT statement: the near-miss 3, 4, 5 at
exponent 3 really is rejected (27 + 64 = 91, not 125). -/
theorem flt_instance_check : (3 : Nat) ^ 3 + 4 ^ 3 ≠ 5 ^ 3 := by decide

/-- Non-vacuity for `OddPrime`: 3 qualifies. -/
theorem oddPrime_three : OddPrime 3 := by
  refine ⟨by decide, fun d hd => ?_⟩
  have hle : d ≤ 3 := Nat.le_of_dvd (by decide) hd
  obtain ⟨c, hc⟩ := hd
  cases d with
  | zero => omega
  | succ d =>
    cases d with
    | zero => exact Or.inl rfl
    | succ d =>
      cases d with
      | zero => omega
      | succ d =>
        have : d = 0 := by omega
        subst this
        exact Or.inr rfl

/-- Non-vacuity for `PrimeGE5`: 5 qualifies. -/
theorem primeGE5_five : PrimeGE5 5 := by
  refine ⟨by decide, fun d hd => ?_⟩
  have hle : d ≤ 5 := Nat.le_of_dvd (by decide) hd
  obtain ⟨c, hc⟩ := hd
  cases d with
  | zero => omega
  | succ d =>
    cases d with
    | zero => exact Or.inl rfl
    | succ d =>
      cases d with
      | zero => omega
      | succ d =>
        cases d with
        | zero => omega
        | succ d =>
          cases d with
          | zero => omega
          | succ d =>
            have : d = 0 := by omega
            subst this
            exact Or.inr rfl

/-- Instance check for split A's exponent reduction: 12 = 3 * 4. -/
theorem exponentReduction_case_twelve :
    ∃ k m, 12 = k * m ∧ (m = 4 ∨ OddPrime m) :=
  ⟨3, 4, by decide, Or.inl rfl⟩

/-- Instance check for split B's exponent reduction: 10 = 2 * 5, and 5 is
a prime >= 5. -/
theorem exponentReductionFine_case_ten :
    ∃ k m, 10 = k * m ∧ (m = 3 ∨ m = 4 ∨ PrimeGE5 m) :=
  ⟨2, 5, by decide, Or.inr (Or.inr primeGE5_five)⟩

end Razor.Frontier
