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
| `mutable.phnl` | Explicit mutable bindings and type-checked reassignment |
| `loops.phnl` | Bounded while loops, `break`, and `continue` |
| `typed_ir.phnl` | Typed functions, arithmetic, mutable bindings, and loops |

Mutable assignment and while-loop control flow are implemented and demonstrated above. For-each loops remain a design target. See [Syntax.md](../Syntax.md)
for the authoritative implementation-status list.


### Typed IR

`typed_ir.phnl` exercises typed function calls, arithmetic, mutable bindings, and loop control through the current interpreter. The integration tests compile the same language constructs through `compile_to_ir` and validate their typed representation.
