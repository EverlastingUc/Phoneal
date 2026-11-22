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

The interpreter still executes the AST. The typed IR is the compiler-facing representation; it is not yet consumed by bytecode generation or a VM.

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

This phase intentionally stops at bytecode generation. It does not claim bytecode verification, serialization, VM execution, or sandboxing.

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

## Explicit exclusions

Do not implement the Package Manager, Standard Library, or self-hosting compiler unless the project owner changes the scope.
