//! The sorted-witness circuit, for any list length from 2 to 32.
//!
//! Public input:  h - a MiMC-style commitment to a secret list.
//! Private input: the list xs (each < 2^8) and, implicitly, its sorted
//! arrangement (the comparator select bits). The sorting network is
//! Batcher's odd-even mergesort generated for the list's length; for
//! length 4 it is exactly the 5-comparator network whose soundness is the
//! Lean theorem `Razor.Zk.network_sound`. For other lengths the
//! per-comparator theorem (`Razor.Zk.comparator_sound`) covers every gate,
//! and the whole-network theorem is an honest open hole anyone can
//! register and prove.
//!
//! Constraints:
//! 1. Commitment: h = sponge(xs) where the permutation is x -> (x + c_i)^5,
//!    a MiMC-like construction (demo-grade, not production-audited).
//! 2. A 5-comparator sorting network on 4 wires. Each comparator:
//!      - select bit s is boolean:          s * (s - 1) = 0
//!      - lo is one endpoint:               s * (b - a) = lo - a
//!      - hi is the other (linear):         a + b - lo - hi = 0
//!      - order really holds (range check): hi - lo = Σ 2^k d_k, d_k boolean
//!    The comparator's soundness - constraints imply {lo,hi} = {a,b} and
//!    lo ≤ hi - is proven in Lean (`Razor.Zk.comparator_sound`), and the
//!    network's soundness in `Razor.Zk.network_sound`. That is the zkGolf
//!    trust chain: Groth16 attests the constraints hold; Lean proves the
//!    constraints mean what the challenge says they mean.
//!
//! Values are 8-bit and the field is ~2^255, so no arithmetic wraps; the
//! Lean model works over Int with the same constraint shapes.

use ark_bls12_381::Fr;
use ark_ff::Field;
use ark_relations::lc;
use ark_relations::r1cs::{
    ConstraintSynthesizer, ConstraintSystemRef, LinearCombination, SynthesisError, Variable,
};

pub const MIMC_ROUNDS: usize = 64;
pub const RANGE_BITS: usize = 8;
pub const MIN_LIST: usize = 2;
pub const MAX_LIST: usize = 32;

/// Comparator pairs of Batcher's odd-even mergesort network on `n` wires.
/// For n = 4 this is the optimal 5-comparator network [(0,1), (2,3),
/// (0,2), (1,3), (1,2)] proven sound in Lean.
pub fn network(n: usize) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    let mut p = 1;
    while p < n {
        let mut k = p;
        while k >= 1 {
            let mut j = k % p;
            while j + k < n {
                for i in 0..k {
                    if j + i + k < n && (j + i) / (2 * p) == (j + i + k) / (2 * p) {
                        pairs.push((j + i, j + i + k));
                    }
                }
                j += 2 * k;
            }
            k /= 2;
        }
        p *= 2;
    }
    pairs
}

pub fn mimc_constants() -> Vec<Fr> {
    // Fixed, public constants for the demo.
    (0..MIMC_ROUNDS).map(|i| Fr::from((i as u64 + 1) * 7919)).collect()
}

/// The sponge computed natively (prover-side and for out-of-circuit checks).
pub fn commit(xs: &[u64]) -> Fr {
    let cs = mimc_constants();
    let mut state = Fr::from(0u64);
    for &x in xs {
        state += Fr::from(x);
        for c in &cs {
            state = (state + c).pow([5]);
        }
    }
    state
}

#[derive(Clone)]
pub struct SortedWitnessCircuit {
    /// Public commitment.
    pub hash: Option<Fr>,
    /// The secret list.
    pub xs: Option<Vec<u64>>,
    /// List length: fixes the circuit's shape (and so the proving key).
    pub n: usize,
}

impl ConstraintSynthesizer<Fr> for SortedWitnessCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let n = self.n;
        let hash_input = cs.new_input_variable(|| self.hash.ok_or(SynthesisError::AssignmentMissing))?;
        let xs = self.xs;
        let val = |i: usize| xs.as_ref().map(|v| Fr::from(v[i]));

        // The secret list: allocated exactly once, shared by the commitment
        // sponge and the sorting network (a separate allocation per
        // sub-circuit would let a prover commit one list and sort another).
        let mut x_lcs: Vec<LinearCombination<Fr>> = Vec::new();
        for i in 0..n {
            let xi = cs.new_witness_variable(|| val(i).ok_or(SynthesisError::AssignmentMissing))?;
            x_lcs.push(lc!() + xi);
        }

        // 1. Commitment sponge over the shared variables.
        let constants = mimc_constants();
        let mut state_val = xs.as_ref().map(|_| Fr::from(0u64));
        let mut state_lc: LinearCombination<Fr> = lc!();
        for i in 0..n {
            state_val = state_val.zip(val(i)).map(|(s, v)| s + v);
            state_lc = state_lc + x_lcs[i].clone();
            for c in &constants {
                let t_val = state_val.map(|s| s + c);
                let t2_val = t_val.map(|t| t * t);
                let t4_val = t2_val.map(|t| t * t);
                let t5_val = t4_val.zip(t_val).map(|(a, b)| a * b);
                let t_lc = state_lc.clone() + (*c, Variable::One);
                let t2 = cs.new_witness_variable(|| t2_val.ok_or(SynthesisError::AssignmentMissing))?;
                let t4 = cs.new_witness_variable(|| t4_val.ok_or(SynthesisError::AssignmentMissing))?;
                let t5 = cs.new_witness_variable(|| t5_val.ok_or(SynthesisError::AssignmentMissing))?;
                cs.enforce_constraint(t_lc.clone(), t_lc.clone(), lc!() + t2)?;
                cs.enforce_constraint(lc!() + t2, lc!() + t2, lc!() + t4)?;
                cs.enforce_constraint(lc!() + t4, t_lc, lc!() + t5)?;
                state_val = t5_val;
                state_lc = lc!() + t5;
            }
        }
        cs.enforce_constraint(state_lc, lc!() + Variable::One, lc!() + hash_input)?;

        // 2. Sorting network over the same variables.
        let mut wire_vals: Vec<Option<Fr>> = (0..n).map(val).collect();
        let mut wire_lcs = x_lcs;

        for (i, j) in network(n) {
            let a_val = wire_vals[i];
            let b_val = wire_vals[j];
            let lt = |a: Fr, b: Fr| fr_to_u64(b) < fr_to_u64(a);
            let s_val = a_val.zip(b_val).map(|(a, b)| if lt(a, b) { Fr::ONE } else { Fr::from(0u64) });
            let lo_val = a_val.zip(b_val).map(|(a, b)| if lt(a, b) { b } else { a });
            let hi_val = a_val.zip(b_val).map(|(a, b)| if lt(a, b) { a } else { b });

            let s = cs.new_witness_variable(|| s_val.ok_or(SynthesisError::AssignmentMissing))?;
            let lo = cs.new_witness_variable(|| lo_val.ok_or(SynthesisError::AssignmentMissing))?;
            let hi = cs.new_witness_variable(|| hi_val.ok_or(SynthesisError::AssignmentMissing))?;
            let a_lc = wire_lcs[i].clone();
            let b_lc = wire_lcs[j].clone();

            // s boolean
            cs.enforce_constraint(lc!() + s, (lc!() + s) - (Fr::ONE, Variable::One), lc!())?;
            // s * (b - a) = lo - a
            cs.enforce_constraint(lc!() + s, b_lc.clone() - a_lc.clone(), (lc!() + lo) - a_lc.clone())?;
            // a + b = lo + hi
            cs.enforce_constraint(
                a_lc + b_lc - lo - hi,
                lc!() + Variable::One,
                lc!(),
            )?;
            // hi - lo in [0, 2^8): bit decomposition
            let d_val = hi_val.zip(lo_val).map(|(h, l)| h - l);
            let mut recomposed: LinearCombination<Fr> = lc!();
            for k in 0..RANGE_BITS {
                let bit_val = d_val.map(|d| Fr::from((fr_to_u64(d) >> k) & 1));
                let bk = cs.new_witness_variable(|| bit_val.ok_or(SynthesisError::AssignmentMissing))?;
                cs.enforce_constraint(lc!() + bk, (lc!() + bk) - (Fr::ONE, Variable::One), lc!())?;
                recomposed = recomposed + (Fr::from(1u64 << k), bk);
            }
            cs.enforce_constraint(
                recomposed - hi + lo,
                lc!() + Variable::One,
                lc!(),
            )?;

            wire_vals[i] = lo_val;
            wire_vals[j] = hi_val;
            wire_lcs[i] = lc!() + lo;
            wire_lcs[j] = lc!() + hi;
        }
        Ok(())
    }
}

pub fn fr_to_u64(f: Fr) -> u64 {
    use ark_ff::{BigInteger, PrimeField};
    let bytes = f.into_bigint().to_bytes_le();
    u64::from_le_bytes(bytes[..8].try_into().unwrap())
}
