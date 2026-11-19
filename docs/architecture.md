# Compiler Architecture

Phoneal currently uses a Rust-hosted compile-and-interpret pipeline:

```text
.phnl source
    -> Lexer (English keywords, comments, escapes, source locations)
    -> Recursive-descent Parser (AST, precedence, functions, branches)
    -> Semantic Analysis (scopes, primitive types, signatures, branch return paths)
    -> Interpreter (checked arithmetic, short-circuit logic, function frames)

Front-end admission limits: 1 MiB source, 100,000 tokens, and 256 nested grammar levels.
```

## Current invariants

- Parsing completes before semantic analysis and execution.
- Same-scope redeclaration is rejected; nested shadowing is permitted.
- Variable types may be inferred from initializers or explicitly annotated; mismatched initializers are rejected before execution.
- Declaration initializers are evaluated before inserting the new binding.
- Function calls use a fresh parameter scope and do not inherit caller-local variables.
- Function signatures are collected before body analysis, allowing forward references and recursion.
- Function returns unwind through nested blocks and selected conditional branches.
- Non-boolean conditional expressions are rejected statically.
- `and` and `or` short-circuit at runtime.
- A non-unit function is accepted only when every reachable branch returns a value.
- Integer arithmetic uses checked operations.
- Active function calls are limited to 128. Every executed statement consumes one unit from a per-run 50,000-step budget; exceeding it returns a runtime diagnostic.
- Source files, token streams, and recursive grammar nesting have explicit admission limits to avoid unbounded front-end growth.

## Not yet a security boundary

The current Rust interpreter does not isolate untrusted programs from host resources. It has no bytecode verifier, capability enforcement, instruction budget, memory quota, or sandbox process boundary. These are future engineering milestones, not implied by the current compiler pipeline.
