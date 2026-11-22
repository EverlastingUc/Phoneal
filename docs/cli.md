# Phoneal CLI

The Rust CLI currently exposes five commands:

| Command | Behavior |
| --- | --- |
| `phnl run <file.phnl>` | Execute source with the AST interpreter |
| `phnl run-bytecode <file.phnl>` | Compile source and execute it in the bytecode VM |
| `phnl check <file.phnl>` | Run lexer, parser, and semantic validation |
| `phnl build <file.phnl> --output <file.phnb>` | Compile, verify, and serialize PHNB bytecode |
| `phnl inspect <file.phnb>` | Deserialize, verify, and print bytecode disassembly |

## Examples

```sh
phnl check examples/factorial.phnl
phnl run examples/factorial.phnl
phnl run-bytecode examples/factorial.phnl
phnl build examples/factorial.phnl --output factorial.phnb
phnl inspect factorial.phnb
```

## Execution distinction

`run` uses the original AST interpreter. `run-bytecode` uses the compiler and initial VM. Both are experimental and are not security sandboxes. The CLI does not implement policy files, capability grants, or host-resource isolation. A `--policy` argument is a future design target only.
