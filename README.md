# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** .phnl  
**Status:** Early experimental Rust-hosted prototype

Phoneal is an experimental security-first, general-purpose programming language. Its long-term goals include memory safety, explicit capabilities, sandboxed execution, and verifiable security properties.

## Current state

The first milestone is a small Rust-hosted interpreter prototype with a hand-written lexer, recursive-descent parser, arithmetic expressions, let declarations, and print statements. The compiler API returns structured diagnostics with a stage and optional source location. The parser normalizes token streams so an EOF sentinel is always present.

This is not yet a bytecode compiler, sandbox, or production-ready secure language.

## Run

    cargo run -- run examples/hello.phnl
    cargo run -- check examples/hello.phnl
    cargo test

## Diagnostics

Errors identify the responsible stage (lexer, parser, or runtime). Lexical and parsing errors include a 1-based line and column when a source location is available.

## Roadmap

1. Stabilize grammar and diagnostics.
2. Add functions, scopes, and static type checking.
3. Define a typed intermediate representation.
4. Implement bytecode generation and verification.
5. Build a restricted Rust-based virtual machine.
6. Add capability-based permissions and sandboxing.
7. Work toward self-hosting after the language stabilizes.

See SECURITY.md for limitations.
