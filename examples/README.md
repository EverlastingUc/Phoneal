# Phoneal examples

These files are executable programs for the **currently implemented** language subset.
Run an individual example with:

    cargo run -- run examples/hello.phnl

The CI script executes every supported program listed below and verifies that the
intentionally invalid type-checking example is rejected.

| File | Demonstrates |
| --- | --- |
| `hello.phnl` | Declarations and output |
| `conditions.phnl` | Booleans, comparisons, and conditionals |
| `factorial.phnl` | Typed functions, recursion, and returns |
| `functions.phnl` | Function declarations and calls |
| `scopes.phnl` | Nested lexical scopes |
| `strings.phnl` | Text values and concatenation |
| `type_error.phnl` | An intentionally invalid program rejected by `check` |

Mutable assignment and loops are design targets, not implemented language features.
They are deliberately not presented as runnable examples yet. See [Syntax.md](../Syntax.md)
for the authoritative implementation-status list.
