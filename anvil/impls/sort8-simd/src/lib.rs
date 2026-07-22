//! ANV-003 contender: the Batcher network across 32 words at once.
//!
//! Same trade as the GPU lane, taken on the CPU: process the whole input
//! stream in batches instead of one word per call. A batch of 32 words is
//! transposed into structure-of-arrays form - eight arrays of 32 bytes,
//! one per byte position - and each of Batcher's 19 comparators becomes an
//! elementwise min/max over two 32-byte arrays. Plain loops over fixed
//! arrays of bytes: exactly the shape the compiler's auto-vectorizer
//! turns into 16-lane vector min/max instructions, sorting 16 words'
//! worth of one comparator per instruction. No unsafe, no intrinsics -
//! the vectorization is the optimizer's, the semantics are the loops'.
//!
//! Per word this computes exactly the Batcher odd-even merge network. The
//! Lean model is `Razor.Anvil.sortBatcher` and the admission proof
//! `Razor.Anvil.batcher_refines` covers all 2^64 inputs by SAT
//! (`bv_decide`); the differential check runs this lane against the
//! executable spec on the full benchmark stream.
//!
//! Like the GPU lane it is a whole-stream entry: the harness times the
//! batch pipeline end to end, input generation and repacking included.

const N: usize = 32;

#[inline(always)]
fn cswap_lanes(lanes: &mut [[u8; N]; 8], i: usize, j: usize) {
    // Row copies first: the two rows provably do not alias, so the
    // min/max loops below are free to become vector instructions.
    let (a, b) = (lanes[i], lanes[j]);
    for k in 0..N {
        lanes[i][k] = a[k].min(b[k]);
    }
    for k in 0..N {
        lanes[j][k] = a[k].max(b[k]);
    }
}

#[inline(always)]
fn sort_batch(words: &mut [u64; N]) {
    let mut lanes = [[0u8; N]; 8];
    for k in 0..N {
        let b = words[k].to_le_bytes();
        for i in 0..8 {
            lanes[i][k] = b[i];
        }
    }
    // Batcher's odd-even merge sort: sort both halves, then merge.
    cswap_lanes(&mut lanes, 0, 1); cswap_lanes(&mut lanes, 2, 3);
    cswap_lanes(&mut lanes, 4, 5); cswap_lanes(&mut lanes, 6, 7);
    cswap_lanes(&mut lanes, 0, 2); cswap_lanes(&mut lanes, 1, 3);
    cswap_lanes(&mut lanes, 4, 6); cswap_lanes(&mut lanes, 5, 7);
    cswap_lanes(&mut lanes, 1, 2); cswap_lanes(&mut lanes, 5, 6);
    cswap_lanes(&mut lanes, 0, 4); cswap_lanes(&mut lanes, 1, 5);
    cswap_lanes(&mut lanes, 2, 6); cswap_lanes(&mut lanes, 3, 7);
    cswap_lanes(&mut lanes, 2, 4); cswap_lanes(&mut lanes, 3, 5);
    cswap_lanes(&mut lanes, 1, 2); cswap_lanes(&mut lanes, 3, 4);
    cswap_lanes(&mut lanes, 5, 6);
    for k in 0..N {
        let mut b = [0u8; 8];
        for i in 0..8 {
            b[i] = lanes[i][k];
        }
        words[k] = u64::from_le_bytes(b);
    }
}

/// Single-word entry (used by differential spot checks; scoring runs the
/// batch pipeline).
pub fn solve(x: u64) -> u64 {
    let mut w = [x; N];
    sort_batch(&mut w);
    w[0]
}

/// Whole-stream differential entry.
pub fn solve_many(inputs: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(inputs.len());
    for chunk in inputs.chunks(N) {
        let mut w = [0u64; N];
        w[..chunk.len()].copy_from_slice(chunk);
        sort_batch(&mut w);
        out.extend_from_slice(&w[..chunk.len()]);
    }
    out
}

/// Whole-stream benchmark entry: same input stream and checksum contract
/// as every other lane, batched 32 words at a time.
pub fn bench_batch(seed: u64, iters: u64) -> u64 {
    let mut acc = 0u64;
    let mut buf = [0u64; N];
    let mut n = 0usize;
    for x in anvil_abi::input_stream(seed, iters, |x| x) {
        buf[n] = x;
        n += 1;
        if n == N {
            sort_batch(&mut buf);
            for &w in &buf {
                acc = acc.wrapping_add(w);
            }
            n = 0;
        }
    }
    if n > 0 {
        let mut tail = [0u64; N];
        tail[..n].copy_from_slice(&buf[..n]);
        sort_batch(&mut tail);
        for &w in &tail[..n] {
            acc = acc.wrapping_add(w);
        }
    }
    acc
}

anvil_abi::anvil_entry!(solve, |x| x);
