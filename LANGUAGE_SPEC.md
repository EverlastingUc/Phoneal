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

The Rust API exposes `compile_to_ir(source)`, which runs lexing, parsing, semantic analysis, and typed-IR lowering. The IR has explicit primitive types on expressions and declarations, typed function signatures and calls, structured `if` / `while` control flow, and explicit return/break/continue nodes. `ir::validate` checks variable references, declaration and assignment types, mutability, function signatures, boolean conditions, return types, and loop-control placement. The interpreter still executes the AST; bytecode generation and VM execution are later milestones. Source locations are preserved for statements and expressions, including literals.

## Example

```phoneal
function factorial(number: integer) returns integer {
    if (number <= 1) { return 1; }
    else { return number * factorial(number - 1); }
}
print(factorial(5));
```

## Not implemented yet

For-each loops, decimals, lists, maps, user-defined types, exception handling, module imports, capability requests, bytecode, a virtual machine, and sandboxing remain future milestones. Package Manager, Standard Library, and self-hosting compiler are explicitly deferred.
