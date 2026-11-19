# Phoneal Language Notes (Prototype)

This document records the implemented subset. It is not a promise that syntax is final.

## Lexical forms

- Keywords: `let`, `print`
- Identifiers: ASCII letters or underscore followed by ASCII letters, digits, or underscores
- Integer literals: values in the signed 64-bit range
- String literals: double-quoted, single-line strings
- Operators: `=`, `+`, `-`, `*`, `/`
- Delimiters: `(`, `)`, `{`, `}`, `;`

## Statements

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

A declaration is immutable in the current prototype. Each block creates a lexical scope. A name can be shadowed in a nested block, but cannot be redeclared in the same scope. An inner block can read outer declarations; declarations inside a block are not visible after the block ends.

## Static types

The current type system has two types: `integer` and `string`.

| Operator | Supported operands | Result |
| --- | --- | --- |
| `+` | integer + integer | integer |
| `+` | string + string | string |
| `-`, `*`, `/` | integer and integer | integer |

Mixed-type operations and string subtraction/multiplication/division are rejected during semantic analysis, before interpretation.

## Current limitations

No booleans, null, functions, user-defined types, arrays, imports, loops, or user-defined operators. Integer arithmetic is checked at runtime for overflow and division by zero.
