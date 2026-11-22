# Phoneal Bytecode Compiler Contract

**Status:** Compiler and initial VM are implemented and CI-verified. Bytecode serialization and security-grade verification are not implemented.

## Compilation entry points

- Rust API: `compile_to_bytecode(source)` performs lexing, parsing, semantic analysis, typed-IR lowering, and bytecode generation.
- IR API: `bytecode::compile(&TypedProgram)` compiles an existing typed IR program.
- Output: `BytecodeProgram`, containing indexed functions, an entry function, and a global-slot count.

The compiler validates the typed IR before lowering it. It then validates the resulting bytecode structure before returning it.

## Function and storage model

- Each function has a fixed local frame.
- Parameters occupy the first local slots, in declaration order.
- Locals receive distinct slots; shadowed bindings receive separate slots.
- Top-level bindings occupy global slots.
- Functions may load global bindings, but the current language semantics prohibit functions from mutating outer/global bindings.
- Function references use indices into `BytecodeProgram.functions`.
- The entry code is stored separately as `BytecodeProgram.entry`. Frame metadata records local/parameter/global types and mutability for runtime checks.

## Instruction contract

The bytecode is stack-oriented. The initial VM is exposed as `bytecode::execute_bytecode(&BytecodeProgram)`; the end-to-end source API is `run_bytecode(source)`.

| Opcode family | Stack effect | Meaning |
| --- | --- | --- |
| `PushInteger`, `PushText`, `PushBoolean` | +1 | Push a constant |
| `LoadLocal`, `LoadGlobal` | +1 | Load a slot |
| `StoreLocal`, `StoreGlobal` | -1 | Store and consume the top value |
| `Pop` | -1 | Discard the top value |
| `Print` | -1 | Print and consume the top value |
| `Negate`, `Not` | 0 | Replace the top value with its unary result |
| Arithmetic/comparison/equality | -1 | Consume two values and push one result |
| `Jump` | 0 | Set the instruction pointer to an absolute offset |
| `JumpIfFalse` | -1 | Consume a boolean; jump when it is false |
| `Call` | `1 - argument_count` | Consume arguments and push the function result |
| `Return` | -1 | Return the top value from the current function |
| `Halt` | 0 | End the entry function |
| `Unreachable` | 0 | Explicit marker after validated function code |

Binary expressions evaluate their left operand before their right operand. Call arguments are emitted in declaration order. Jump offsets are absolute instruction indices within the current function, never offsets into another function.

## Control flow

- `if` and `else` compile to conditional and unconditional jumps.
- `while` compiles a condition block, conditional exit, body, and backward jump.
- `break` emits a forward jump patched to the nearest enclosing loop's exit.
- `continue` jumps to the nearest enclosing loop's condition.
- `and` and `or` use conditional jumps so the right operand is evaluated only when required.

## Structural validation

`bytecode::validate_structure` checks:

- non-empty instruction streams and required entry/function terminators;
- local and global slot bounds;
- jump targets within the current function;
- function-call indices and argument counts;
- no function-level `Halt` and no entry-level `Return`.

This is deliberately **not** a security verifier. It does not prove stack-depth consistency, validate runtime value types, enforce instruction or memory quotas, restrict host capabilities, or make hostile bytecode safe to execute.

## VM execution

The initial VM executes these instructions with typed local/global slots, mutability enforcement, function frames, checked arithmetic, and bounded instruction/call/stack/text/output usage. See [vm.md](vm.md) for its runtime contract. The original `run(source)` API still uses the AST interpreter; `run_bytecode(source)` selects the bytecode path.

## Deliberate non-goals

No bytecode file format, serializer/deserializer, disassembler, security-grade bytecode verifier, sandbox process, capability system, aggregate heap quota, or OS-enforced wall-clock/memory limits are included. Structural validation is not a security boundary.
