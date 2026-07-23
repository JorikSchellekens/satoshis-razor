//! ANV-008 contender: Morton interleave in two `pdep` instructions.
//!
//! BMI2's `pdep` deposits the low bits of its source at the positions of
//! the set bits of its mask - interleaving is one deposit onto the even
//! positions and one onto the odd. This lane only exists on x86-64 with
//! BMI2 (post-2013 Intel/AMD); everywhere else the harness reports it as
//! not measurable, the same way the GPU lane behaves on a GPU-less box.
//!
//! The Lean model `Razor.Anvil.mortonPdep` transliterates `pdep` itself
//! (a 64-step deposit walk) applied to the two constant masks, and the
//! admission proof checks that against the reference interleave on all
//! 2^64 inputs - so what is proven is the semantics of the instruction
//! this lane leans on, not a paraphrase of it.

pub fn available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("bmi2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
unsafe fn morton_bmi2(x: u64) -> u64 {
    use core::arch::x86_64::_pdep_u64;
    _pdep_u64(x, 0x5555_5555_5555_5555) | _pdep_u64(x >> 32, 0xAAAA_AAAA_AAAA_AAAA)
}

pub fn solve(x: u64) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        assert!(available(), "morton-pdep needs BMI2");
        unsafe { morton_bmi2(x) }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = x;
        unreachable!("morton-pdep only runs on x86_64 with BMI2; the harness gates on available()")
    }
}

anvil_abi::anvil_entry!(solve, |x| x);
