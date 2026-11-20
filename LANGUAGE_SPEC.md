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
- Explicit mutable bindings (`create mutable`) and type-checked `set` assignment
- Nested lexical scopes and shadowing
- Typed function arguments, return checking, forward references, and recursion
- Multi-error semantic diagnostics with source locations
- Single-line `#` comments, block comments, and escaped strings

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
