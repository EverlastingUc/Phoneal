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

## Latest verified state

- Latest verified `main` commit: `ddca55bb54ee1b1b8e840cf9515bbb4e51f38b6d`
- Successful CI: [Phoneal CI #45057359543](https://github.com/EverlastingUc/Phoneal/actions/runs/45057359543)
- Tests: **115 passed, 0 failed** (67 unit, 42 compiler-pipeline, 3 typed-IR, 3 bytecode-compiler)

- Branch: `main`
- Verified implementation commit: `5495482e92b3024e0a68d64151edefae8d53d087`
- Successful GitHub Actions run: [Phoneal CI #45055580840](https://github.com/EverlastingUc/Phoneal/actions/runs/45055580840)
- Unit tests: **59 passed**
- Compiler-pipeline integration tests: **42 passed**
- Typed-IR integration tests: **3 passed**
- Total: **104 passed, 0 failed**

The successful CI run executed `cargo fmt --all`, `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features`, `cargo build --locked`, `bash scripts/check_examples.sh`, and `cargo test --locked`.

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
