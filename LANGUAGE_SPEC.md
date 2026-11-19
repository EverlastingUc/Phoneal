# Phoneal Language Notes (Prototype)

This document records the implemented subset. It is not a promise that syntax is final.

## Lexical forms

- Keywords: `let`, `print`
- Identifiers: ASCII letters or underscore followed by ASCII letters, digits, or underscores
- Integer literals: non-negative values in the signed 64-bit range; negative values are not yet expressible as literals
- String literals: double-quoted, single-line strings
- Operators: `=`, `+`, `-`, `*`, `/`
- Delimiters: `(`, `)`, `;`

## Statements

```phoneal
let answer = 6 * 7;
let greeting = "Hello, Phoneal";
print(answer);
print(greeting);
```

A declaration is immutable in the current prototype and is visible after its initializer. Redeclaration in the program scope is rejected.

## Static types

The current type system has two types: `integer` and `string`.

| Operator | Supported operands | Result |
| --- | --- | --- |
| `+` | integer + integer | integer |
| `+` | string + string | string |
| `-`, `*`, `/` | integer and integer | integer |

Mixed-type operations and string subtraction/multiplication/division are rejected during semantic analysis, before interpretation.

## Current limitations

No booleans, null, functions, user-defined types, arrays, block scopes, imports, loops, or user-defined operators. Integer arithmetic is checked at runtime for overflow and division by zero.
