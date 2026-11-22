# Phoneal Roadmap

This document records the roadmap state established by the current project specification and verified repository work. It does not invent acceptance criteria for phases that have not yet been specified.

## Verified phases

### Phase 9 — Loops and control flow

**Complete.** The Rust interpreter supports bounded `while` execution, nested loops, nearest-loop `break` and `continue`, boolean-only conditions, return propagation through loops, and shared execution-budget enforcement.

### Phase 10 — Language consistency and correctness

**Complete.** Parser stack usage was hardened without reducing the established nesting limit. Function/global mutation semantics and assignment evaluation order are consistent between semantic analysis and runtime checks.

### Phase 11 — Compiler frontend

**Complete.** The lexer, parser, AST, semantic analysis, structured diagnostics, type inference, function signatures, and control-flow validation form the validated-AST frontend.

### Phase 12 — Typed Intermediate Representation

**Complete and CI-verified.**

- Explicitly typed expressions, declarations, assignments, calls, and function signatures.
- Structured `if` and `while` nodes, plus explicit `return`, `break`, and `continue`.
- Compiler-independent unary and binary operator enums.
- Source locations retained for literals, operators, parameters, and available statement/expression nodes.
- Independent IR validation for variable resolution, binding mutability, type consistency, function signatures, return coverage, boolean conditions, and loop-control placement.
- Public Rust API: `compile_to_ir(source)`.
- Executable example: `examples/typed_ir.phnl`.

The AST interpreter remains available. Typed IR is consumed by the bytecode compiler, and the resulting stack instructions can be executed by the initial Phoneal VM through `run_bytecode(source)`.

### Phase 13 — Bytecode Compiler

**Complete and CI-verified.**

- Public source API: `compile_to_bytecode(source)`.
- Public IR API: `bytecode::compile(&TypedProgram)`.
- Stack-oriented opcodes for primitive constants, local/global slots, arithmetic, comparisons, printing, function calls, returns, and control flow.
- Branch and loop jump patching, including nearest-loop `break` / `continue`.
- Short-circuit compilation for `and` and `or`.
- Source locations retained on emitted instructions.
- Bytecode compiler diagnostics have a distinct `bytecode` stage.
- Structural validation checks jump targets, local/global slots, function-call indices and arity, and function/entry terminators.
- Tests cover arithmetic, function calls, control-flow targets, short-circuit structure, global reads in functions, source-location retention, and malformed bytecode structures.

This phase intentionally stops at bytecode generation. VM execution is tracked separately in Phase 14.

Expected pipeline direction:

```text
Phoneal Source
    -> Lexer
    -> Parser
    -> Semantic Analysis
    -> Typed IR
    -> Bytecode Compiler
    -> Phoneal VM
    -> Security Architecture
```

### Phase 14 — Initial Bytecode Virtual Machine

**Complete and CI-verified.**

- Public source API: `run_bytecode(source)`.
- Public VM API: `bytecode::execute_bytecode(&BytecodeProgram)`.
- Typed runtime values for integers, text, and booleans.
- Call frames with parameter/local slots and return-value propagation, including recursion.
- Runtime type and mutability checks for local/global stores; functions cannot mutate global bindings.
- Checked integer arithmetic, text concatenation, comparisons, and short-circuit execution.
- Runtime diagnostics retain instruction source locations where available.
- `ResetLocals` reinitializes local slots declared inside a while-body on every iteration, including nested loop/block locals.
- Limits: 50,000 instructions, 128 active calls, 100,000 operand-stack values, 1 MiB per text value, and 1 MiB captured output.
- Tests cover arithmetic, recursion, mutable locals/globals, read-only globals in functions, overflow, division by zero, short-circuiting, step limits, and malformed bytecode.

The VM is an execution engine, not a sandbox. It uses a typed bytecode verifier for operand-stack and control-flow consistency, but has no aggregate heap quota, wall-clock deadline, capability system, host-resource isolation, or complete security-grade bytecode verifier.

### Phase 15 — Typed Bytecode Verification

**Complete and CI-verified.**

- Independent data-flow analysis for every function and the entry instruction stream.
- Operand-stack type tracking for constants, local/global loads and stores, arithmetic, comparisons, equality, conditions, calls, returns, and output.
- Control-flow merge validation ensures all incoming paths agree on the operand-stack type sequence.
- Function calls are checked against parameter types and return types.
- Function bytecode cannot write global slots.
- Definite local initialization is tracked across branches and loops; reads from uninitialized local slots are rejected.
- Return and halt instructions require balanced operand stacks.
- The compiler verifies generated bytecode before returning it, and the VM verifies again before execution.

This is a typed correctness verifier, not a complete security boundary. It does not prove global initialization across all call paths or impose aggregate memory/wall-clock limits.

### Phase 16 — PHNB Bytecode Serialization

**Complete and CI-verified.**

- Versioned binary container with `PHNB` magic and version 1.
- Little-endian header, payload length, and CRC-32 integrity check.
- Serializes function metadata, primitive types, mutability, opcodes, and source locations.
- Deserializer bounds file size, strings, function count, slot counts, and instruction counts.
- Rejects invalid tags, flags, UTF-8, truncated payloads, length mismatches, trailing data, and checksum failures.
- Deserialization runs structural and typed bytecode verification before returning a program.
- Public APIs: `bytecode::serialize` and `bytecode::deserialize`.

CRC-32 detects accidental corruption; it is not cryptographic authentication. PHNB files are not trusted merely because they deserialize successfully.

### Phase 17 — Bytecode Disassembler

**Complete and CI-verified.**

- Public API: `bytecode::disassemble(&BytecodeProgram)`.
- Verifies the program before producing output.
- Prints function signatures, local counts, instruction offsets, opcodes, call targets, jump targets, and source locations.
- Tests cover functions, calls, branches, locations, and invalid bytecode rejection.

The listing is a debugging representation, not a source-level decompiler.

### Phase 18 — CLI Bytecode Workflow

**Complete and CI-verified.**

- `phnl build <file.phnl> --output <file.phnb>` compiles, verifies, and serializes a source program.
- `phnl inspect <file.phnb>` deserializes, verifies, and disassembles a bytecode file.
- `phnl run-bytecode <file.phnl>` compiles and executes source through the bytecode VM.
- Existing `phnl run` remains the AST interpreter path; `phnl check` remains the frontend validation path.
- End-to-end CLI tests cover build, inspect, bytecode execution, checking, and extension validation.

### Phase 19 — Precompiled Bytecode Execution CLI

**Complete and CI-verified.**

- `phnl execute <file.phnb>` reads an existing PHNB file.
- CLI reads are capped at 1 MiB for source and 16 MiB for bytecode before decoding.
- Deserialization performs file bounds, CRC-32, structural, and typed stack-flow verification before execution.
- The verified program runs in the bytecode VM and prints captured output.
- CLI integration tests execute a file produced by `phnl build`.

This command does not make PHNB files safe for untrusted use; the VM is not a sandbox.

### Phase 20 — CLI Help and Version Flags

**Complete and CI-verified.**

- `phnl --help` and `phnl -h` print command usage.
- `phnl --version` and `phnl -V` print the package version from Cargo metadata.
- CLI integration tests cover both flags.

### Phase 21 — Loop-Scoped Local Lifetimes

**Complete; CI verification pending.**

- Added `ResetLocals` bytecode for the contiguous local-slot range allocated inside each while body.
- The VM clears those slots at the beginning of every iteration, preserving fresh immutable/mutable bindings across repeated block execution.
- Nested loops reset only their own body range while outer-loop locals remain intact.
- `break` and `continue` flow back through the loop condition/body entry, so the next iteration performs the reset.
- Typed verification tracks definite local initialization and clears initialization state at reset points.
- Tests cover immutable loop locals, nested loops, `continue`, serialization, and disassembly.

## Latest verified implementation

- Code commit: `677c795c79a68c27ea15d5e1e3bde8bcc01f5924`
- CI: [Phoneal CI #45059936966](https://github.com/EverlastingUc/Phoneal/actions/runs/45059936966)
- Tests: **150 passed, 0 failed** (93 unit, 42 compiler-pipeline, 8 bytecode integration, 4 CLI, 3 typed IR)
- Checks: formatting, Clippy, build, executable examples, and tests all passed.

## Explicit exclusions

Do not implement the Package Manager, Standard Library, or self-hosting compiler unless the project owner changes the scope.
