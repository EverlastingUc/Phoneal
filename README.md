# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** `.phnl`  
**Status:** Experimental Rust-hosted language prototype

Phoneal is a security-first, general-purpose programming language project. The design prioritizes readable English-oriented syntax, static checking, explicit capabilities, and eventually sandboxed execution.

## Current implementation

- `create` declarations with inferred or explicit primitive types (`create age: integer = 18;`), explicit mutable bindings (`create mutable score = 0;`), checked `set` assignment, and `print(...)`
- `integer`, `text`, and `boolean` primitive types
- Arithmetic, comparisons, and English logical operators (`and`, `or`, `not`)
- `if` / `else if` / `else` and `while` with boolean-only conditions
- Nested loops with `break` and `continue`
- Nested lexical scopes
- Typed top-level `function` declarations using `returns`
- Function calls, recursion, and control-flow-aware return checking
- Functions can read global bindings but cannot mutate outer-scope bindings
- Typed intermediate representation via `compile_to_ir(source)`, with source locations and independent IR validation
- Stack-bytecode compiler via `compile_to_bytecode(source)`, with typed opcodes, local/global slots, function-call targets, patched branches/loops, and short-circuit control flow
- Typed bytecode verifier for stack types, control-flow merges, function signatures, definite local initialization, and global-write restrictions
- Versioned PHNB v2 bytecode serialization/deserialization with bounded parsing, CRC-32 integrity checks, and verification on load
- Human-readable bytecode disassembler via `bytecode::disassemble`
- Deny-by-default capability policy primitives in `phoneal::security` and bounded PHIP v1 IPC frames in `phoneal::security::ipc` (not yet connected to host APIs, a worker process, or a sandbox)
- Bytecode VM via `run_bytecode(source)` / `bytecode::execute_bytecode`, with checked arithmetic, typed frames, loop-scope local resets, mutability enforcement, recursion and execution limits
- Source-located lexer, parser, semantic, bytecode, and runtime diagnostics
- Comments and escaped string literals
- CI-verified executable language examples (see `examples/README.md`)
- Front-end/interpreter limits: 1 MiB source, 100,000 tokens, 256 nested grammar levels, 128 active calls, and 50,000 interpreter steps
- Bytecode VM limits: 50,000 instructions, 128 active calls, 100,000 operand-stack values, 1 MiB per text value, and 1 MiB captured output
- CLI file reads are bounded to 1 MiB for source and 16 MiB for PHNB files

```phoneal
function factorial(number: integer) returns integer {
    if (number <= 1) { return 1; }
    else { return number * factorial(number - 1); }
}
print(factorial(5));
```

## Run

    cargo run -- --help
    cargo run -- --version

    cargo run -- run examples/hello.phnl
    cargo run -- run examples/conditions.phnl
    cargo run -- run examples/factorial.phnl
    cargo run -- run examples/loops.phnl
    cargo run -- check examples/functions.phnl
    cargo run -- run-bytecode examples/loops.phnl
    cargo run -- build examples/factorial.phnl --output factorial.phnb
    cargo run -- inspect factorial.phnb
    cargo run -- execute factorial.phnb
    cargo test
    bash scripts/check_examples.sh

## Pipeline

    Source -> Lexer -> Parser -> Semantic Analysis -> AST Interpreter
                                      \-> Typed IR -> Bytecode Compiler -> Typed Verifier -> Bytecode VM

The bytecode compiler and initial VM are implemented. `run(source)` still uses the AST interpreter; `run_bytecode(source)` uses the bytecode VM. The VM has runtime checks and bounded execution, but no security-grade bytecode verifier, heap quota, wall-clock limit, capability isolation, or process sandbox. Do not execute untrusted Phoneal programs with it.

## Documentation

- [Bytecode instruction format and compiler contract](docs/bytecode.md)
- [PHNB file format](docs/bytecode-format.md)
- [Bytecode VM runtime contract](docs/vm.md)
- [Command-line interface](docs/cli.md)
- [Security architecture and threat model](docs/security-architecture.md)
- [Complete syntax design and implementation status](Syntax.md)
- [Implemented language subset](LANGUAGE_SPEC.md)
- [Compiler architecture](docs/architecture.md)
- [Diagnostic behavior](docs/diagnostics.md)
- [Security limitations](SECURITY.md)

## Deferred by project owner

Package Manager, Standard Library, and self-hosting compiler are intentionally excluded until the language, VM, and sandbox foundations are ready.
