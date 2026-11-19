# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** .phnl  
**Status:** Early experimental Rust-hosted prototype

Phoneal is an experimental security-first, general-purpose programming language. Its long-term goals include memory safety, explicit capabilities, sandboxed execution, and verifiable security properties.

## Current implementation

The Rust prototype includes a hand-written lexer, recursive-descent parser, arithmetic expressions, immutable `let` declarations, and `print(...)` statements. The semantic pass checks variable declarations, undefined names, and primitive expression types before execution. It can report multiple independent semantic diagnostics in one pass.

The current primitive types are `integer` and `string`. Integer addition/subtraction/multiplication/division are supported; string concatenation uses `+`. Runtime integer arithmetic is checked for overflow and division by zero. CLI diagnostics include source context and a caret.

This is not yet a bytecode compiler, sandbox, or production-ready secure language. Lexing and parsing still stop at the first error. The language has one program-wide scope.

## Run

    cargo run -- run examples/hello.phnl
    cargo run -- run examples/strings.phnl
    cargo run -- check examples/hello.phnl
    cargo run -- check examples/type_error.phnl
    cargo test

## Compiler pipeline

    Source -> Lexer -> Parser -> Semantic Analysis -> Interpreter

## Project documents

- [Implemented language subset](LANGUAGE_SPEC.md)
- [Compiler architecture](docs/architecture.md)
- [Diagnostic behavior](docs/diagnostics.md)
- [Security limitations](SECURITY.md)

## Roadmap

1. Parser recovery and complete source spans.
2. Lexical scopes and functions.
3. Typed intermediate representation.
4. Bytecode generation and verification.
5. Restricted Rust-based virtual machine.
6. Capability-based permissions and sandboxing.
7. Self-hosting after the language and bootstrap process stabilize.
