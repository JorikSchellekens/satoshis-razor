//! Shared definitions for the ANV-100 EVM challenge.
//!
//! The opcode set, gas costs, and the fixed benchmark program are part of the
//! challenge, not of any submission - every implementation interprets the
//! same program on the same inputs, so their checksums must agree.
//!
//! Words are 64-bit for this demo (the Lean spec uses UInt64, which matches
//! u64 exactly, wrapping arithmetic included). The real EVM's 256-bit words
//! would use the same structure with four-limb arithmetic.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Stop,
    Add,
    Mul,
    Sub,
    Push(u64),
    Pop,
    Dup1,
    Swap1,
}

/// Yellow Paper gas costs for this subset.
pub fn cost(op: Op) -> u64 {
    match op {
        Op::Stop => 0,
        Op::Add | Op::Sub | Op::Push(_) | Op::Dup1 | Op::Swap1 => 3,
        Op::Mul => 5,
        Op::Pop => 2,
    }
}

/// Gas budget each `solve` call starts with.
pub const INITIAL_GAS: u64 = 10_000;

/// The fixed benchmark program. The stack starts as [x]; each block keeps the
/// depth at exactly one, so the program never underflows and never stops
/// early - the whole program is interpreted on every call.
pub const PROGRAM: &[Op] = &{
    // 12 rounds of: PUSH c, DUP1, MUL, ADD, PUSH c', SWAP1, SUB
    // (depth 1 -> 1, gas 23 per round), then STOP. Total gas: 276.
    const fn round(c: u64, c2: u64) -> [Op; 7] {
        [Op::Push(c), Op::Dup1, Op::Mul, Op::Add, Op::Push(c2), Op::Swap1, Op::Sub]
    }
    let mut p = [Op::Stop; 12 * 7 + 1];
    let mut i = 0;
    while i < 12 {
        let r = round(0x9E37_79B9 + i as u64, 0x85EB_CA77 ^ (i as u64) << 3);
        let mut j = 0;
        while j < 7 {
            p[i * 7 + j] = r[j];
            j += 1;
        }
        i += 1;
    }
    p
};

/// Fold an execution result into the u64 checksum `solve` returns.
pub fn checksum(result: Option<(u64, Vec<u64>)>) -> u64 {
    match result {
        None => 0xDEAD_BEEF,
        Some((gas, stack)) => stack
            .iter()
            .fold(gas, |acc, w| acc.rotate_left(7).wrapping_add(*w)),
    }
}
