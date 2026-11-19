# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** `.phnl`  
**Status:** Experimental Rust-hosted language prototype

Phoneal is a security-first, general-purpose programming language project. The design prioritizes readable English-oriented syntax, static checking, explicit capabilities, and eventually sandboxed execution.

## Current implementation

- `create` declarations and `print(...)`
- `integer`, `text`, and `boolean` primitive types
- Arithmetic, comparisons, and English logical operators (`and`, `or`, `not`)
- `if` / `else if` / `else` with boolean-only conditions
- Nested lexical scopes
- Typed top-level `function` declarations using `returns`
- Function calls, recursion, and control-flow-aware return checking
- Source-located lexer, parser, semantic, and runtime diagnostics
- Comments and escaped string literals
- CI-verified executable examples (see `examples/README.md`)
- Front-end input limits: 1 MiB source, 100,000 tokens, and 256 nested grammar levels

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
    cargo run -- check examples/functions.phnl
    cargo test
    bash scripts/check_examples.sh

## Pipeline

    Source -> Lexer -> Parser -> Semantic Analysis -> Rust Interpreter

This is still an interpreter prototype. It is not a bytecode compiler, virtual machine, or sandbox. Do not execute untrusted Phoneal programs with it.

## Documentation

- [Complete syntax design and implementation status](Syntax.md)
- [Implemented language subset](LANGUAGE_SPEC.md)
- [Compiler architecture](docs/architecture.md)
- [Diagnostic behavior](docs/diagnostics.md)
- [Security limitations](SECURITY.md)

## Deferred by project owner

Package Manager, Standard Library, and self-hosting compiler are intentionally excluded until the language, VM, and sandbox foundations are ready.
