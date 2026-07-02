/-!
Anvil challenge ANV-100: an EVM interpreter.

The specification is an interpreter for a subset of EVM opcodes over a stack
machine, with the Yellow Paper's gas costs for these operations. For this demo
the machine word is 64 bits (`UInt64`, matching Rust's `u64` exactly, wrapping
arithmetic included); the real EVM uses 256-bit words, which would use the
same structure with four-limb arithmetic.

`execSpec` is the specification: the reference Rust implementation
(anvil/impls/evm-ref) is a line-by-line transliteration of it.

`execTos` is the model of the optimized submission (anvil/impls/evm-tos): an
interpreter that keeps the top of the stack in a register instead of in the
stack container - a standard interpreter optimization. `tos_refines` proves
the two agree on every program, every gas budget, and every starting stack.
-/

namespace Razor.Evm

/-- The supported opcodes. `push` carries its immediate. -/
inductive Op where
  | stop
  | add
  | mul
  | sub
  | push (imm : UInt64)
  | pop
  | dup1
  | swap1
deriving Repr, DecidableEq

/-- Gas cost of each opcode (Yellow Paper: ADD/SUB/PUSH/DUP/SWAP are 3,
MUL is 5, POP is 2, STOP is 0). -/
def cost : Op → UInt64
  | .stop => 0
  | .add => 3
  | .mul => 5
  | .sub => 3
  | .push _ => 3
  | .pop => 2
  | .dup1 => 3
  | .swap1 => 3

/-- The specification interpreter. Returns `none` on stack underflow or
out-of-gas; otherwise `some (remaining gas, final stack)`. -/
def execSpec : List Op → UInt64 → List UInt64 → Option (UInt64 × List UInt64)
  | [], gas, st => some (gas, st)
  | op :: p, gas, st =>
    if gas < cost op then none
    else
      match op, st with
      | .stop, st => some (gas - cost .stop, st)
      | .add, a :: b :: st => execSpec p (gas - cost .add) ((a + b) :: st)
      | .mul, a :: b :: st => execSpec p (gas - cost .mul) ((a * b) :: st)
      | .sub, a :: b :: st => execSpec p (gas - cost .sub) ((a - b) :: st)
      | .push i, st => execSpec p (gas - cost (.push i)) (i :: st)
      | .pop, _ :: st => execSpec p (gas - cost .pop) st
      | .dup1, a :: st => execSpec p (gas - cost .dup1) (a :: a :: st)
      | .swap1, a :: b :: st => execSpec p (gas - cost .swap1) (b :: a :: st)
      | _, _ => none

/-- Model of the optimized interpreter: the top of the stack lives in the
`tos` argument (a register in the Rust implementation); `rest` is the stack
below it. `none` for `tos` means the stack is empty. -/
def execTos : List Op → UInt64 → Option UInt64 → List UInt64 → Option (UInt64 × List UInt64)
  | [], gas, none, _ => some (gas, [])
  | [], gas, some t, rest => some (gas, t :: rest)
  | op :: p, gas, tos, rest =>
    if gas < cost op then none
    else
      match op, tos, rest with
      | .stop, none, _ => some (gas - cost .stop, [])
      | .stop, some t, rest => some (gas - cost .stop, t :: rest)
      | .add, some t, b :: rest => execTos p (gas - cost .add) (some (t + b)) rest
      | .mul, some t, b :: rest => execTos p (gas - cost .mul) (some (t * b)) rest
      | .sub, some t, b :: rest => execTos p (gas - cost .sub) (some (t - b)) rest
      | .push i, tos, rest =>
        match tos with
        | none => execTos p (gas - cost (.push i)) (some i) rest
        | some t => execTos p (gas - cost (.push i)) (some i) (t :: rest)
      | .pop, some _, b :: rest => execTos p (gas - cost .pop) (some b) rest
      | .pop, some _, [] => execTos p (gas - cost .pop) none []
      | .dup1, some t, rest => execTos p (gas - cost .dup1) (some t) (t :: rest)
      | .swap1, some t, b :: rest => execTos p (gas - cost .swap1) (some b) (t :: rest)
      | _, _, _ => none

/-- The stack `execTos` models: register contents on top of the container. -/
def stackOf : Option UInt64 → List UInt64 → List UInt64
  | none, _ => []
  | some t, rest => t :: rest

/-- ANV-100 admission proof for evm-tos: the register-cached interpreter
agrees with the specification on every program, gas budget, and stack.
(The `none` case carries the empty-stack invariant.) -/
theorem tos_refines (p : List Op) :
    ∀ (gas : UInt64) (tos : Option UInt64) (rest : List UInt64),
      (tos = none → rest = []) →
      execTos p gas tos rest = execSpec p gas (stackOf tos rest) := by
  induction p with
  | nil =>
    intro gas tos rest hinv
    cases tos with
    | none => simp [execTos, execSpec, stackOf]
    | some t => simp [execTos, execSpec, stackOf]
  | cons op p ih =>
    intro gas tos rest hinv
    have ihS : ∀ g t r, execTos p g (some t) r = execSpec p g (t :: r) :=
      fun g t r => ih g (some t) r (by intro h; cases h)
    have ihN : ∀ g, execTos p g none [] = execSpec p g [] :=
      fun g => ih g none [] (fun _ => rfl)
    cases tos with
    | none =>
      have hr : rest = [] := hinv rfl
      subst hr
      cases op <;> simp [execTos, execSpec, stackOf, ihS, ihN]
    | some t =>
      cases rest with
      | nil => cases op <;> simp [execTos, execSpec, stackOf, ihS, ihN]
      | cons b r => cases op <;> simp [execTos, execSpec, stackOf, ihS, ihN]

end Razor.Evm
