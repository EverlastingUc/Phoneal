# Compiler Architecture

Phoneal currently uses a Rust-hosted compile-and-interpret pipeline:

```text
.phnl source
    -> Lexer (English keywords, comments, escapes, source locations)
    -> Recursive-descent Parser (AST, precedence, functions, branches, loops)
    -> Semantic Analysis (scopes, primitive types, signatures, branch return paths)
        -> AST Interpreter (checked arithmetic, short-circuit logic, function frames)
        -> Typed IR (typed expressions, declarations, calls, functions, and control flow)
            -> Bytecode Compiler (stack instructions, slots, calls, jumps)
                -> future bytecode verifier and VM

Front-end admission limits: 1 MiB source, 100,000 tokens, and 256 nested grammar levels.
```

## Typed IR boundary

`compile_to_ir(source)` runs the same lexer, parser, and semantic checks as the interpreter pipeline, then lowers the validated AST into a compiler-independent representation. `ir::validate` independently checks typed variable references, declarations, assignments, function calls, returns, boolean conditions, and loop-control placement. The IR retains source locations for statements, expressions, literals, operators, and parameters. The interpreter continues to execute the AST. `bytecode::compile(&ir)` and `compile_to_bytecode(source)` lower validated IR into a stack-oriented instruction stream. The compiler emits typed arithmetic operations, local/global slot access, function calls, conditional/loop jumps, and short-circuit branches. Jump targets are absolute instruction offsets within each function. The compiler runs structural checks on its output (jump ranges, slot references, call indices/arity, and terminators). These checks are not a security verifier. No serializer or VM exists yet.

## Current invariants

- Parsing completes before semantic analysis and execution.
- Same-scope redeclaration is rejected; nested shadowing is permitted.
- Bindings are immutable by default. Mutable bindings carry a runtime mutability flag; `set` resolves and validates its target before evaluating the right-hand side, then checks the value type before updating it.
- Variable types may be inferred from initializers or explicitly annotated; mismatched initializers are rejected before execution.
- Declaration initializers are evaluated before inserting the new binding.
- Function calls use a fresh parameter scope and do not inherit caller-local variables. Global bindings are read-only inside functions because the current interpreter gives each call a snapshot; static analysis and runtime both reject attempts to mutate those outer bindings.
- Assignment resolves and validates its target before evaluating the right-hand side. Runtime checks that the replacement value has the same primitive variant as the existing binding.
- Function signatures are collected before body analysis, allowing forward references and recursion.
- Function returns unwind through nested blocks and selected conditional branches.
- Non-boolean conditional and loop expressions are rejected statically.
- `break` and `continue` are rejected outside loops and consumed by the nearest enclosing loop at runtime.
- `return` propagates through loops and exits the current function.
- Every loop condition check consumes the shared execution-step budget, including empty infinite loops.
- `and` and `or` short-circuit at runtime.
- A non-unit function is accepted only when every reachable branch returns a value.
- Integer arithmetic uses checked operations.
- Active function calls are limited to 128. Every executed statement and every loop condition check consumes one unit from a per-run 50,000-step budget; exceeding it returns a runtime diagnostic.
- Source files, token streams, and recursive grammar nesting have explicit admission limits to avoid unbounded front-end growth.

## Not yet a security boundary

The current Rust interpreter does not isolate untrusted programs from host resources. It has no bytecode verifier, capability enforcement, instruction budget, memory quota, or sandbox process boundary. These are future engineering milestones, not implied by the current compiler pipeline.
