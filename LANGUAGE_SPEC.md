# Phoneal Language Specification

The canonical syntax reference is [Syntax.md](Syntax.md).

## Implemented by the current Rust prototype

- English declarations with inference or explicit annotations: `create name = expression;` and `create age: integer = 18;`
- Explicit annotation compatibility is checked during semantic analysis.
- Typed functions: `function name(value: integer) returns integer { ... }`
- Primitive types: `integer`, `text`, `boolean`
- Integer, text, and boolean literals
- Checked integer arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparisons: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical operators: `and`, `or`, `not` with short-circuit evaluation
- `if`, `else if`, and `else`, with boolean-only conditions
- `while` loops with boolean-only conditions, including nested loops, `break`, and `continue`
- Explicit mutable bindings (`create mutable`) and type-checked `set` assignment; runtime also validates assignment target mutability and value type before updating
- Functions may read top-level bindings but cannot mutate outer-scope bindings
- Nested lexical scopes and shadowing
- Typed function arguments, return checking, forward references, and recursion
- Multi-error semantic diagnostics with source locations
- Single-line `#` comments, block comments, and escaped strings

## Typed intermediate representation

The Rust API exposes `compile_to_ir(source)`, which runs lexing, parsing, semantic analysis, and typed-IR lowering. The IR has explicit primitive types on expressions and declarations, typed function signatures and calls, structured `if` / `while` control flow, and explicit return/break/continue nodes. `ir::validate` checks variable references, declaration and assignment types, mutability, function signatures, boolean conditions, return types, and loop-control placement. The interpreter still executes the AST through `run(source)`. The bytecode path is available separately through `compile_to_bytecode(source)` and `run_bytecode(source)`. Source locations are preserved in IR and emitted instructions.

## Example

```phoneal
function factorial(number: integer) returns integer {
    if (number <= 1) { return 1; }
    else { return number * factorial(number - 1); }
}
print(factorial(5));
```

## Bytecode compiler and VM

The Rust API exposes `compile_to_bytecode(source)` and `run_bytecode(source)`. The stack VM supports typed local/global frames, function calls and recursion, checked arithmetic, comparisons, branches, loops, short-circuit operators, and runtime mutability/type checks. It enforces instruction, call-depth, operand-stack, individual text-value, and captured-output limits. It is not a security sandbox.

## Not implemented yet

For-each loops, decimals, lists, maps, user-defined types, exception handling, module imports, capability requests, security-grade bytecode verification, serialization, aggregate heap quotas, wall-clock enforcement, and sandboxing remain future milestones. Package Manager, Standard Library, and self-hosting compiler are explicitly deferred.
