//! ANV-006 contender: SipHash-1-3, unrolled per word and interleaved four
//! wide across the stream (eight-wide was measured too: it spills registers
//! on an M3 and loses).
//!
//! One SipHash is a chain: every add feeds the next rotate, so a single
//! hash can never fill the pipeline. Two changes, one per level:
//!
//! - Per word, the round loops are unrolled straight-line and the length
//!   block folded to its constant (`8 << 56`) - this is the shape the Lean
//!   model `Razor.Anvil.sip13Inline` transliterates, and the admission
//!   proof shows it equals the paper-shaped reference on all 2^64 inputs.
//! - Across the stream, the whole-batch entry hashes four independent
//!   words in lockstep, so the out-of-order core always has four
//!   dependency chains in flight. Same function per word - only the
//!   schedule changes, which is why the per-word proof covers this lane.

const K0: u64 = 0x0706050403020100;
const K1: u64 = 0x0F0E0D0C0B0A0908;

const V0: u64 = K0 ^ 0x736F6D6570736575;
const V1: u64 = K1 ^ 0x646F72616E646F6D;
const V2: u64 = K0 ^ 0x6C7967656E657261;
const V3: u64 = K1 ^ 0x7465646279746573;
const LEN_BLOCK: u64 = 8 << 56;

macro_rules! sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {
        $v0 = $v0.wrapping_add($v1);
        $v1 = $v1.rotate_left(13) ^ $v0;
        $v0 = $v0.rotate_left(32);
        $v2 = $v2.wrapping_add($v3);
        $v3 = $v3.rotate_left(16) ^ $v2;
        $v0 = $v0.wrapping_add($v3);
        $v3 = $v3.rotate_left(21) ^ $v0;
        $v2 = $v2.wrapping_add($v1);
        $v1 = $v1.rotate_left(17) ^ $v2;
        $v2 = $v2.rotate_left(32);
    };
}

#[inline(always)]
pub fn solve(x: u64) -> u64 {
    let mut v0 = V0;
    let mut v1 = V1;
    let mut v2 = V2;
    let mut v3 = V3 ^ x;
    sipround!(v0, v1, v2, v3);
    v0 ^= x;
    v3 ^= LEN_BLOCK;
    sipround!(v0, v1, v2, v3);
    v0 ^= LEN_BLOCK;
    v2 ^= 0xFF;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    v0 ^ v1 ^ v2 ^ v3
}

/// Four hashes in lockstep: every line below is four independent copies of
/// the scalar step, so four dependency chains interleave in the core.
#[inline(always)]
fn solve4(x: [u64; 4]) -> [u64; 4] {
    let mut v0 = [V0; 4];
    let mut v1 = [V1; 4];
    let mut v2 = [V2; 4];
    let mut v3 = [V3; 4];
    for l in 0..4 {
        v3[l] ^= x[l];
    }
    round4(&mut v0, &mut v1, &mut v2, &mut v3);
    for l in 0..4 {
        v0[l] ^= x[l];
        v3[l] ^= LEN_BLOCK;
    }
    round4(&mut v0, &mut v1, &mut v2, &mut v3);
    for l in 0..4 {
        v0[l] ^= LEN_BLOCK;
        v2[l] ^= 0xFF;
    }
    round4(&mut v0, &mut v1, &mut v2, &mut v3);
    round4(&mut v0, &mut v1, &mut v2, &mut v3);
    round4(&mut v0, &mut v1, &mut v2, &mut v3);
    let mut out = [0u64; 4];
    for l in 0..4 {
        out[l] = v0[l] ^ v1[l] ^ v2[l] ^ v3[l];
    }
    out
}

#[inline(always)]
fn round4(v0: &mut [u64; 4], v1: &mut [u64; 4], v2: &mut [u64; 4], v3: &mut [u64; 4]) {
    for l in 0..4 {
        v0[l] = v0[l].wrapping_add(v1[l]);
        v1[l] = v1[l].rotate_left(13) ^ v0[l];
        v0[l] = v0[l].rotate_left(32);
    }
    for l in 0..4 {
        v2[l] = v2[l].wrapping_add(v3[l]);
        v3[l] = v3[l].rotate_left(16) ^ v2[l];
    }
    for l in 0..4 {
        v0[l] = v0[l].wrapping_add(v3[l]);
        v3[l] = v3[l].rotate_left(21) ^ v0[l];
    }
    for l in 0..4 {
        v2[l] = v2[l].wrapping_add(v1[l]);
        v1[l] = v1[l].rotate_left(17) ^ v2[l];
        v2[l] = v2[l].rotate_left(32);
    }
}

/// Whole-stream timed entry: same input stream and checksum contract as the
/// scalar path, hashed four words at a time.
pub fn bench_batch(seed: u64, iters: u64) -> u64 {
    let mut inputs = anvil_abi::input_stream(seed, iters, |x| x);
    let mut acc = 0u64;
    let mut group = [0u64; 4];
    let mut fill = 0usize;
    for _ in 0..iters {
        group[fill] = inputs.next().expect("stream length");
        fill += 1;
        if fill == 4 {
            for r in solve4(group) {
                acc = acc.wrapping_add(r);
            }
            fill = 0;
        }
    }
    for &x in &group[..fill] {
        acc = acc.wrapping_add(solve(x));
    }
    acc
}

/// Whole-stream differential entry: the exact outputs the batch path
/// produces, one per input.
pub fn solve_many(inputs: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(inputs.len());
    let mut chunks = inputs.chunks_exact(4);
    for c in &mut chunks {
        out.extend(solve4([c[0], c[1], c[2], c[3]]));
    }
    for &x in chunks.remainder() {
        out.push(solve(x));
    }
    out
}

anvil_abi::anvil_entry!(solve, |x| x);
