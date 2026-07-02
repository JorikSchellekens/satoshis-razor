//! ANV-100 optimized submission: the top of the stack lives in a register.
//!
//! Most opcodes read or write the top of the stack. Keeping it in a local
//! variable (`tos`) instead of behind a Vec index removes a bounds-checked
//! memory access from nearly every instruction. Lean model:
//! `Razor.Evm.execTos`; admission proof: `Razor.Evm.tos_refines` - the
//! register-cached interpreter agrees with the specification on every
//! program, gas budget, and stack.

use evm_common::{checksum, cost, Op, INITIAL_GAS, PROGRAM};

pub fn run(program: &[Op], mut gas: u64, stack: Vec<u64>) -> Option<(u64, Vec<u64>)> {
    let mut rest = stack;
    // Invariant (as in the Lean model): tos None means the stack is empty.
    let mut tos: Option<u64> = rest.pop();

    let finish = |gas: u64, tos: Option<u64>, mut rest: Vec<u64>| {
        if let Some(t) = tos {
            rest.push(t);
        }
        Some((gas, rest))
    };

    for &op in program {
        if gas < cost(op) {
            return None;
        }
        gas -= cost(op);
        match op {
            Op::Stop => return finish(gas, tos, rest),
            Op::Add => {
                let t = tos?;
                let b = rest.pop()?;
                tos = Some(t.wrapping_add(b));
            }
            Op::Mul => {
                let t = tos?;
                let b = rest.pop()?;
                tos = Some(t.wrapping_mul(b));
            }
            Op::Sub => {
                let t = tos?;
                let b = rest.pop()?;
                tos = Some(t.wrapping_sub(b));
            }
            Op::Push(i) => {
                if let Some(t) = tos {
                    rest.push(t);
                }
                tos = Some(i);
            }
            Op::Pop => {
                tos?;
                tos = rest.pop();
            }
            Op::Dup1 => {
                let t = tos?;
                rest.push(t);
            }
            Op::Swap1 => {
                let t = tos?;
                let b = rest.pop()?;
                rest.push(t);
                tos = Some(b);
            }
        }
    }
    finish(gas, tos, rest)
}

pub fn solve(x: u64) -> u64 {
    checksum(run(PROGRAM, INITIAL_GAS, vec![x]))
}

anvil_abi::anvil_entry!(solve, |x| x);
