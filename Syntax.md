# Phoneal Syntax Reference

**Status:** Canonical language design; implementation status is tracked at the end of this document  
**File extension:** `.phnl`  
**Pronunciation:** Pho-ni-yal

This document is the intended source of truth for Phoneal's human-facing syntax. The implementation must not be described as supporting a feature until the lexer, parser, semantic checker, runtime/VM, tests, and examples support it.

The syntax is deliberately English-oriented: use complete words such as `create`, `function`, `returns`, `and`, and `or` instead of abbreviations or punctuation-heavy spellings. Familiar arithmetic and comparison symbols remain because they are concise and widely understood.

## 1. Design rules

1. Prefer readable English keywords over abbreviations.
2. Keep one spelling for each language construct; do not add aliases without a strong reason.
3. Make common programs easy to read from top to bottom.
4. Require explicit types on function parameters and return values.
5. Keep blocks visually obvious with braces.
6. Make mutability explicit.
7. Make permissions explicit. A program requesting a capability does not grant that capability to itself.
8. Reject invalid programs before execution wherever possible.
9. Every documented construct needs positive and negative tests.
10. Never describe the language or sandbox as unhackable.

## 2. Lexical rules

### 2.1 Names

Identifiers start with an ASCII letter or underscore and may contain ASCII letters, digits, and underscores. Names are case-sensitive.

```phoneal
user_name
total2
_privateValue
```

### 2.2 Comments

```phoneal
# A single-line comment

/*
   A multi-line comment.
*/
```

### 2.3 Strings

Strings use double quotes. Escape sequences are interpreted rather than copied literally.

```phoneal
create message = "Hello, Phoneal";
create quote = "She said: \"Hello\"";
create line = "First line\nSecond line";
create tab = "Name:\tPhoneal";
create slash = "C:\\Phoneal";
```

Required escapes: `\\`, `\"`, `\n`, `\r`, and `\t`. Invalid or incomplete escapes are compile-time errors. Multiline strings use an explicit future syntax and are not implied by ordinary quoted strings.

### 2.4 Statement endings

A semicolon ends a simple statement. Braced blocks do not need a semicolon after their closing brace. Newlines are whitespace, not implicit statement terminators.

## 3. Variables and constants

### 3.1 Immutable variables

`create` declares an immutable binding. The type is inferred when possible.

```phoneal
create answer = 42;
create greeting = "Hello";
create is_ready = true;

# Optional explicit type annotations
create age: integer = 18;
create name: text = "Ever";
create enabled: boolean = true;
```

### 3.2 Mutable variables

Mutability must be explicit.

```phoneal
create mutable score = 0;
set score = score + 10;
```

A binding cannot be assigned a value of a different type. A name cannot be declared twice in the same scope. Nested blocks may shadow an outer name.

### 3.3 Explicit types

```phoneal
create age: integer = 18;
create name: text = "Ever";
create enabled: boolean = true;
```

## 4. Primitive and compound types

The intended core type vocabulary is:

| Type | Meaning |
| --- | --- |
| `integer` | Signed 64-bit whole number |
| `decimal` | Floating-point number; exact representation/semantics must be specified before implementation |
| `text` | UTF-8 text |
| `boolean` | `true` or `false` |
| `unit` | No meaningful return value |

Compound type forms:

- `list<integer>`
- `map<text, integer>`
- `optional<text>`

The syntax is reserved here; each type becomes supported only after its runtime representation and safety rules are implemented.

## 5. Literals and expressions

```phoneal
create whole = 42;
create ratio = 3.5;
create title = "Phoneal";
create ready = true;
create not_ready = false;
create values = [10, 20, 30];
```

### 5.1 Arithmetic

```phoneal
create sum = 10 + 5;
create difference = 10 - 5;
create product = 10 * 5;
create quotient = 10 / 5;
create remainder = 10 % 3;
```

Integer overflow and division by zero must produce controlled diagnostics, never undefined behavior or a process crash.

### 5.2 Comparison

```phoneal
create same = age == 18;
create different = age != 18;
create younger = age < 18;
create adult = age >= 18;
```

Supported spellings: `==`, `!=`, `<`, `<=`, `>`, `>=`.

### 5.3 Logical operations

Use English words:

```phoneal
create allowed = is_ready and age >= 18;
create visible = is_admin or is_owner;
create blocked = not allowed;
```

Logical operations require boolean operands and must short-circuit: the right side of `and` is skipped when the left side is false; the right side of `or` is skipped when the left side is true.

### 5.4 Operator precedence

From highest to lowest:

1. Parentheses and function calls
2. Unary `not` and unary numeric negation
3. `*`, `/`, `%`
4. `+`, `-`
5. `<`, `<=`, `>`, `>=`
6. `==`, `!=`
7. `and`
8. `or`

Operators at the same binary precedence level associate left-to-right unless a later specification explicitly states otherwise.

## 6. Output

```phoneal
print("Hello, world!");
print(6 * 7);
```

`print` writes a value to the program's standard output. It does not grant filesystem access.

## 7. Conditional statements

```phoneal
if (score >= 100) {
    print("Excellent");
} else if (score >= 50) {
    print("Good");
} else {
    print("Keep practising");
}
```

Conditions must evaluate to `boolean`. There is no implicit truthiness: integers and text cannot be used as conditions.

## 8. Loops

### 8.1 While loop

```phoneal
create mutable count = 0;

while (count < 5) {
    print(count);
    set count = count + 1;
}
```

### 8.2 For-each loop

```phoneal
create numbers = [10, 20, 30];

for number in numbers {
    print(number);
}
```

### 8.3 Loop control

```phoneal
for number in numbers {
    if (number == 20) {
        continue;
    }

    if (number == 30) {
        break;
    }

    print(number);
}
```

`break` and `continue` are valid only inside loops. Loop execution must be subject to sandbox instruction/time limits.

## 9. Functions

Use the complete keyword `function` and the word `returns`.

```phoneal
function add(first: integer, second: integer) returns integer {
    return first + second;
}

function greet(name: text) returns text {
    return "Hello, " + name;
}

print(add(20, 22));
print(greet("Phoneal"));
```

A function with no meaningful result uses `unit`:

```phoneal
function show_message(message: text) returns unit {
    print(message);
    return;
}
```

Rules:

- Parameters and return types are explicit.
- Arguments are positional in the first language version.
- Functions are declared at top level in the initial model.
- Forward calls and recursion are supported only when the compiler can validate signatures.
- Every non-`unit` function must return a value on every reachable path.
- Runtime recursion and total execution are bounded by sandbox policy.

## 10. Lists and maps

Intended list syntax:

```phoneal
create numbers: list<integer> = [10, 20, 30];
print(numbers[0]);
```

Intended map syntax:

```phoneal
create ages: map<text, integer> = {
    "Alex": 20,
    "Sam": 22
};
print(ages["Alex"]);
```

Indexing must be bounds-checked. Map key and value types must be checked. Mutation operations must make ownership/aliasing rules explicit before they are enabled.

## 11. User-defined data types

Intended record syntax:

```phoneal
type Person {
    name: text,
    age: integer
}

create person = Person {
    name: "Alex",
    age: 20
};

print(person.name);
```

Intended choice/variant syntax:

```phoneal
choice DownloadResult {
    Success(path: text),
    Failed(reason: text)
}
```

These declarations are design targets, not claims about the current implementation.

## 12. Error handling

Intended explicit error-handling syntax:

```phoneal
try {
    perform_task();
} catch error {
    print(error.message);
}
```

Intended error raising syntax:

```phoneal
throw "Invalid input";
```

Error handling must not bypass memory safety, capability checks, or resource accounting. Exact error types and propagation semantics must be specified before implementation.

## 13. Scope and visibility

Braces create lexical scopes. Names declared inside a block do not escape it. The initial language uses top-level functions and does not imply closures.

Visibility keywords such as `public` and `private` are reserved for a later module/type system; they are not currently accepted syntax.

## 14. Capability requests and sandbox policy

Phoneal's security model is deny-by-default. A source program may declare the capabilities it needs, but the declaration is only a request. The host, launcher, or administrator decides whether to grant it.

Intended source declarations:

```phoneal
request permission file.read("data/");
request permission file.write("output/");
request permission network.connect("api.example.com");
```

A program cannot grant itself permissions, widen a granted path, disable the sandbox, or access ambient host resources.

Intended policy file example:

```phoneal-policy
allow file.read "data/";
allow file.write "output/";
deny network;
limit memory 64 MB;
limit execution 5 seconds;
limit call_depth 128;
```

Policy syntax is a separate format from ordinary `.phnl` source. The policy loader must reject unknown permissions, malformed limits, path traversal, and conflicting rules. The host must enforce policy independently of program assertions.

The initial sandbox target is a restricted Phoneal virtual machine with:

- no ambient filesystem, network, process, or environment access;
- explicit capability handles for granted operations;
- verified bytecode before execution;
- bounded memory, call depth, instruction count, and wall-clock time;
- checked arithmetic and bounds-checked memory operations;
- deterministic denial diagnostics;
- no native-code escape hatch in the untrusted execution mode.

A Rust interpreter alone is not a sandbox. The sandbox is not complete until the VM boundary, capability enforcement, resource accounting, hostile-input tests, and security review exist.

## 15. CLI interface

Intended command shape:

```text
phnl check <file.phnl>
phnl run <file.phnl> --policy <file.policy>
phnl build <file.phnl> --output <file.phnb>
phnl inspect <file.phnb>
```

The exact bytecode format and build options will be documented when implemented. Running untrusted programs must require an explicit sandbox policy or a restrictive built-in default.

## 16. Reserved words

The planned reserved-word set is:

`create`, `mutable`, `set`, `function`, `returns`, `return`, `if`, `else`, `while`, `for`, `in`, `break`, `continue`, `and`, `or`, `not`, `true`, `false`, `print`, `type`, `choice`, `try`, `catch`, `throw`, `request`, `permission`.

A word becomes reserved in the implementation only when its syntax is supported and covered by tests. Until then, the lexer must not silently treat it as a working language construct.

## 17. Front-end resource limits

The Rust prototype rejects source files larger than **1 MiB**, more than **100,000 non-EOF tokens**, and grammar recursion deeper than **256 nested levels**. Runtime function calls are limited to **128 active calls**, and each run is limited to **50,000 executed statements**. These are defensive resource limits, not a sandbox or a complete denial-of-service defense: they do not impose a wall-clock deadline, heap quota, or output quota.

## 18. Implementation status

The current Rust prototype implements the following subset of this specification:

- Implemented: `create` (with optional `mutable` and optional `: integer`, `: text`, or `: boolean` annotations), `set` assignment with mutability and type checks, `print`, `function`, `returns`, `integer`, `text`, `boolean`, integer/text/boolean literals, arithmetic, comparisons, `and`/`or`/`not`, `if`/`else if`/`else`, typed function calls and returns, nested lexical scopes, source-located diagnostics, comments, escaped strings, and front-end source/token/nesting limits, call-depth limit, and per-run statement budget.
- Not yet implemented: loops, decimal values, lists, maps, user-defined types, error handling, capability requests, bytecode, VM, and sandbox.
- Explicitly deferred by project owner: Package Manager, Standard Library, and self-hosting compiler.

The Package Manager, Standard Library, and self-hosting compiler are explicitly deferred by the project owner. This list must be updated as each feature lands. Documentation must never mark a feature complete before its end-to-end tests pass.
