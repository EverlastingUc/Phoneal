# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** .phnl  
**Status:** Early experimental Rust-hosted prototype

Phoneal is an experimental security-first, general-purpose programming language. Its long-term goals include memory safety, explicit capabilities, sandboxed execution, and verifiable security properties.

## Current state

The Rust-hosted prototype has a hand-written lexer, recursive-descent parser, arithmetic expressions, let declarations, and print statements. Compilation now includes a semantic-analysis pass that rejects duplicate declarations and undefined variables before interpretation.

The language currently has one program-wide variable scope. Semantic diagnostics identify their stage, but do not yet include source locations because AST nodes do not yet retain source spans.

This is not yet a bytecode compiler, sandbox, or production-ready secure language.

## Run

    cargo run -- run examples/hello.phnl
    cargo run -- check examples/hello.phnl
    cargo test

## Compiler pipeline

    Source -> Lexer -> Parser -> Semantic Analysis -> Interpreter

## Roadmap

1. Preserve source spans in AST nodes and improve diagnostics.
2. Add functions, scopes, and static type checking.
3. Define a typed intermediate representation.
4. Implement bytecode generation and verification.
5. Build a restricted Rust-based virtual machine.
6. Add capability-based permissions and sandboxing.
7. Work toward self-hosting after the language stabilizes.

See SECURITY.md for limitations.
