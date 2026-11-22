# Phoneal Virtual Machine

**Status:** Initial stack VM is implemented and CI-verified. It is not a sandbox.

## Entry points

- `run_bytecode(source)`: lexes, parses, checks semantics, lowers to typed IR, compiles bytecode, and executes it.
- `bytecode::execute_bytecode(&BytecodeProgram)`: executes an existing bytecode program after structural validation.

The existing `run(source)` API continues to use the AST interpreter.

## Runtime model

The VM uses a value stack, a global slot frame, and one local frame per active function call. Each call frame stores its function index, instruction pointer, local slots, and the operand-stack base at call time. Parameters occupy the first local slots in declaration order. Calls evaluate arguments left-to-right; the VM checks argument types before entering the callee. A return value is checked against the declared function result type before it is pushed for the caller.

Values are integers (`i64`), UTF-8 text, or booleans. Local and global slot metadata carries primitive types and mutability. A first store initializes a binding; later stores require mutability and matching type. Functions may read globals but the VM rejects global writes from function frames.

## Runtime checks

- Checked integer negation, addition, subtraction, multiplication, division, and remainder.
- Division/remainder by zero diagnostics and overflow diagnostics.
- Runtime type checks for arithmetic, comparisons, conditions, function arguments, function returns, and slot stores.
- Equality operands must have the same primitive type.
- Jump targets and slot/call references are structurally checked before execution.
- Function return requires a balanced operand stack relative to its call frame.
- Reaching an `Unreachable` function terminator is a runtime error.

## Limits

| Resource | Limit |
| --- | ---: |
| Executed bytecode instructions | 50,000 per run |
| Active function calls | 128 |
| Operand-stack values | 100,000 |
| Individual text value | 1 MiB |
| Captured output | 1 MiB |

The VM returns deterministic diagnostics when these limits are exceeded. These are in-process guards, not OS-enforced resource limits. There is no aggregate heap quota or wall-clock timeout.

## Security boundary

This VM is not safe for untrusted programs or hostile bytecode. Structural validation is not a security-grade verifier; it does not prove stack-depth consistency for arbitrary programs or establish memory safety/resource isolation. The VM has no host filesystem/network/process capabilities, but no process sandbox is provided either. Treat it as an experimental execution engine, not a sandbox.

A production sandbox requires independent bytecode verification, aggregate resource accounting, wall-clock enforcement outside the VM, deny-by-default capabilities, process isolation, hostile-input testing, and security review.
