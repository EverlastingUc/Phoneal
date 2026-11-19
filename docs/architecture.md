# Compiler Architecture

Phoneal currently uses a Rust-hosted compile-and-interpret pipeline:

```text
.phnl source
    -> Lexer (tokens with line/column)
    -> Recursive-descent Parser (AST)
    -> Semantic Analysis (names + inferred primitive types)
    -> Interpreter (checked i64 arithmetic)
```

## Invariants

- Parsing must finish before semantic analysis begins.
- A variable initializer is checked before the declaration enters the symbol table.
- A duplicate declaration never replaces the first declaration.
- Semantic analysis accumulates independent diagnostics instead of stopping at the first one.
- Runtime arithmetic uses checked integer operations; debug/release overflow behavior must not change language semantics.

## Planned boundaries

The AST is currently the shared representation between parsing, semantic analysis, and interpretation. A future typed intermediate representation should be introduced before bytecode generation; the AST should not become the VM instruction format.
