# Phoneal Language Notes (Prototype)

This document records the implemented subset. It is not a promise that syntax is final.

## Lexical forms

- Keywords: `let`, `print`, `fn`, `return`
- Identifiers: ASCII letters or underscore followed by ASCII letters, digits, or underscores
- Integer literals: values in the signed 64-bit range
- String literals: double-quoted, single-line strings
- Operators: `=`, `+`, `-`, `*`, `/`
- Delimiters: `(`, `)`, `{`, `}`, `,`, `;`

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

## Functions (provisional syntax)

```phoneal
fn add(a, b) {
    return a + b;
}

fn square(value) {
    return value * value;
}

let answer = add(20, 22);
print(answer);
print(square(7));
```

Functions are declared with `fn`, accept zero or more positional parameters, and return one expression using `return expression;`. Calls use ordinary expression syntax, so they can be nested or used in initializers. Functions can call functions declared later and can call themselves recursively. Runtime recursion is capped at 128 active calls to bound accidental stack exhaustion.

The current prototype checks that called functions exist, argument counts match, parameter names are not duplicated, and `return` occurs inside a function. Parameter and return types are not yet part of the language syntax; full static type checking across function boundaries is future work. Functions are not closures and do not capture caller-local variables.

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
