//! ANV-100 reference implementation: a line-by-line transliteration of the
//! Lean specification `Razor.Evm.execSpec`. The stack is a plain Vec; every
//! opcode pops and pushes exactly as the spec says.

use evm_common::{checksum, cost, Op, INITIAL_GAS, PROGRAM};

pub fn run(program: &[Op], mut gas: u64, mut stack: Vec<u64>) -> Option<(u64, Vec<u64>)> {
    for &op in program {
        if gas < cost(op) {
            return None;
        }
        gas -= cost(op);
        match op {
            Op::Stop => return Some((gas, stack)),
            Op::Add => {
                let a = stack.pop()?;
                let b = stack.pop()?;
                stack.push(a.wrapping_add(b));
            }
            Op::Mul => {
                let a = stack.pop()?;
                let b = stack.pop()?;
                stack.push(a.wrapping_mul(b));
            }
            Op::Sub => {
                let a = stack.pop()?;
                let b = stack.pop()?;
                stack.push(a.wrapping_sub(b));
            }
            Op::Push(i) => stack.push(i),
            Op::Pop => {
                stack.pop()?;
            }
            Op::Dup1 => {
                let a = *stack.last()?;
                stack.push(a);
            }
            Op::Swap1 => {
                let n = stack.len();
                if n < 2 {
                    return None;
                }
                stack.swap(n - 1, n - 2);
            }
        }
    }
    Some((gas, stack))
}

pub fn solve(x: u64) -> u64 {
    checksum(run(PROGRAM, INITIAL_GAS, vec![x]))
}

anvil_abi::anvil_entry!(solve, |x| x);
