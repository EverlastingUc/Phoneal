# Compiler Architecture

Phoneal currently uses a Rust-hosted compile-and-interpret pipeline:

```text
.phnl source
    -> Lexer (tokens with line/column)
    -> Recursive-descent Parser (AST)
    -> Semantic Analysis (lexical scopes, function names/arity, primitive types)
    -> Interpreter (checked i64 arithmetic, lexical environments, function call frames)
```

## Scope and call-frame invariants

- The global program scope exists for the entire execution.
- Every block pushes one new scope and always pops it on normal completion or return/error propagation.
- Name lookup searches from the innermost scope outwards.
- A declaration initializer is evaluated before the new name is inserted.
- Same-scope redeclaration is rejected; nested shadowing is permitted.
- Function calls create a fresh parameter scope and do not inherit caller-local scopes.
- The current global scope is copied into a function call frame, so global bindings available at call time can be read.
- Function return values unwind nested blocks and terminate the current call.
- Active function calls are limited to 128 to bound stack growth.

## Compiler invariants

- Parsing must finish before semantic analysis begins.
- A duplicate declaration never replaces the first declaration.
- Semantic analysis accumulates independent diagnostics instead of stopping at the first one.
- Runtime arithmetic uses checked integer operations; debug/release overflow behavior must not change language semantics.

The AST is currently shared between parsing, semantic analysis, and interpretation. A future typed intermediate representation should be introduced before bytecode generation; the AST should not become the VM instruction format.
