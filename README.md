# Phoneal

**Pronunciation:** Pho-ni-yal  
**Source extension:** `.phnl`  
**Status:** Early experimental Rust-hosted prototype

Phoneal is an experimental security-first, general-purpose programming language. Its long-term goals include memory safety, explicit capabilities, sandboxed execution, and verifiable security properties.

## Current state

The first milestone is a small Rust-hosted interpreter prototype with a hand-written lexer, recursive-descent parser, arithmetic expressions, `let` declarations, and `print(...)` statements. This is not yet a bytecode compiler, sandbox, or production-ready secure language.

## Run

```bash
cargo run -- run examples/hello.phnl
cargo run -- check examples/hello.phnl
cargo test
```

## Roadmap

1. Stabilize grammar and diagnostics.
2. Add functions, scopes, and static type checking.
3. Define a typed intermediate representation.
4. Implement bytecode generation and verification.
5. Build a restricted Rust-based virtual machine.
6. Add capability-based permissions and sandboxing.
7. Work toward self-hosting after the language stabilizes.

See [SECURITY.md](SECURITY.md) for limitations.