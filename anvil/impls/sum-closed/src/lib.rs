//! ANV-002 champion submission: Gauss's closed form, O(1).
//!
//! Divides whichever of n, n+1 is even, so the division is exact and nothing
//! overflows for valid inputs (n < 2^32). Lean model:
//! `Razor.Anvil.sumClosedModel`; admission proof:
//! `Razor.Anvil.closed_refines` (an induction on the Gauss formula, not a
//! SAT instance - algebraic submissions need algebraic proofs).

pub fn solve(n: u64) -> u64 {
    if n % 2 == 0 {
        (n / 2) * (n + 1)
    } else {
        n * ((n + 1) / 2)
    }
}

anvil_abi::anvil_entry!(solve, |x| x & 0xffff);
