# Phoneal Language Notes (Prototype)

This document records the implemented subset. It is not a promise that syntax is final.

## Lexical forms

- Keywords: `let`, `print`, `fn`, `return`
- Identifiers: ASCII letters or underscore followed by ASCII letters, digits, or underscores
- Integer literals: values in the signed 64-bit range
- String literals: double-quoted, single-line strings
- Operators: `=`, `+`, `-`, `*`, `/`, `->`
- Delimiters: `(`, `)`, `{`, `}`, `,`, `:`, `;`

## Statements and expressions

```phoneal
let answer = 6 * 7;
let greeting = "Hello, Phoneal";
print(answer);
print(greeting);

{
    let local = "block scope";
    print(local);
}
```

A declaration is immutable in the current prototype. Each block creates a lexical scope. A name can be shadowed in a nested block, but cannot be redeclared in the same scope.

## Typed functions

Function parameters and return types are explicitly annotated:

```phoneal
fn add(a: integer, b: integer) -> integer {
    return a + b;
}

fn greet(name: string) -> string {
    return "Hello, " + name;
}

print(add(20, 22));
print(greet("Phoneal"));
```

Supported type names are `integer` and `string`. The compiler checks each argument against its parameter type, checks return expressions against the declared return type, and rejects functions without a guaranteed return. Because the current language has no conditional statements or loops, a return in the function's straight-line statement sequence (including an unconditional nested block) guarantees completion. This analysis must be revisited when branching and loops are introduced.

Functions may call functions declared later and may recurse. Runtime recursion is capped at 128 active calls. Functions are top-level declarations and are not closures.

## Static types

The current type system has two primitive types: `integer` and `string`.

| Operator | Supported operands | Result |
| --- | --- | --- |
| `+` | integer + integer | integer |
| `+` | string + string | string |
| `-`, `*`, `/` | integer and integer | integer |

Mixed-type operations and unsupported string operations are rejected during semantic analysis when both operand types are known.

## Current limitations

No booleans, null, user-defined types, arrays, imports, loops, conditional statements, or user-defined operators. Integer arithmetic is checked at runtime for overflow and division by zero.
