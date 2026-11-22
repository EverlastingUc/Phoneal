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
- Nested loops with `break` and `continue
- Nested lexical scopes
- Typed top-level `function` declarations using `returns`
- Function calls, recursion, and control-flow-aware return checking
- Functions can read global bindings but cannot mutate outer-scope bindings
- Typed intermediate representation via `compile_to_ir(source)`, with source locations and IR validation
- Source-located lexer, parser, semantic, and runtime diagnostics
- Comments and escaped string literals
- CI-verified executable examples (see `examples/README.md`)
- Resource limits: 1 MiB source, 100,000 tokens, 256 nested grammar levels, 128 active calls, and 50,000 execution steps per run (including loop condition checks)

```phoneal
function factorial(number: integer) returns integer {
    if (number <= 1) { return 1; }
    else { return number * factorial(number - 1); }
}
print(factorial(5));
```

## Run

    cargo run -- run examples/hello.phnl
    cargo run -- run examples/conditions.phnl
    cargo run -- run examples/factorial.phnl
    cargo run -- run examples/loops.phnl
    cargo run -- check examples/functions.phnl
    cargo test
    bash scripts/check_examples.sh

## Pipeline

    Source -> Lexer -> Parser -> Semantic Analysis -> AST Interpreter
                                      \\-> Typed IR -> future Bytecode Compiler -> VM

This is still an interpreter prototype. It is not a bytecode compiler, virtual machine, or sandbox. Do not execute untrusted Phoneal programs with it.

## Documentation

- [Complete syntax design and implementation status](Syntax.md)
- [Implemented language subset](LANGUAGE_SPEC.md)
- [Compiler architecture](docs/architecture.md)
- [Diagnostic behavior](docs/diagnostics.md)
- [Security limitations](SECURITY.md)

## Deferred by project owner

Package Manager, Standard Library, and self-hosting compiler are intentionally excluded until the language, VM, and sandbox foundations are ready.
