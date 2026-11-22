# Phoneal Bytecode File Format

**Format:** PHNB version 1  
**Extension:** `.phnb`  
**Status:** Serializer/deserializer implemented and CI-verified.

## Container header

All multi-byte integers use little-endian encoding.

| Offset | Size | Field |
| --- | ---: | --- |
| 0 | 4 bytes | ASCII magic `PHNB` |
| 4 | 2 bytes | Format version (`1`) |
| 6 | 4 bytes | Payload length in bytes |
| 10 | 4 bytes | CRC-32 of the payload |
| 14 | variable | Payload |

The CRC-32 detects accidental corruption. It is not a cryptographic signature, authentication mechanism, or proof of trustworthiness.

## Payload layout

1. Global slot count (`u32`).
2. For each global slot: primitive type tag, mutability flag.
3. Function count (`u32`).
4. Each function, in index order.
5. Entry function.

Function layout:

1. UTF-8 function name (length-prefixed).
2. Parameter count and parameter type tags.
3. Local slot count.
4. For each local slot: primitive type tag and mutability flag.
5. Return type tag (`0` means no return type; otherwise a primitive type tag).
6. Instruction count.
7. Instructions, each followed by a source-location flag and optional line/column pair.

Primitive type tags: `1 = integer`, `2 = text`, `3 = boolean`.

## Opcode tags

| Tag | Opcode | Additional operands |
| ---: | --- | --- |
| 0 | PushInteger | signed `i64` |
| 1 | PushText | length-prefixed UTF-8 |
| 2 | PushBoolean | boolean byte |
| 3–6 | LoadLocal, StoreLocal, LoadGlobal, StoreGlobal | slot `u32` |
| 7–10 | Pop, Print, Negate, Not | none |
| 11–16 | AddInteger, AddText, Subtract, Multiply, Divide, Remainder | none |
| 17–22 | Less, LessEqual, Greater, GreaterEqual, Equal, NotEqual | none |
| 23–24 | Jump, JumpIfFalse | absolute instruction index `u32` |
| 25 | Call | function index and argument count |
| 26–28 | Return, Halt, Unreachable | none |
| 29 | ResetLocals | start slot and count (`u32`) |

Source-location flag `0` means absent; flag `1` is followed by line and column as `u32`. Other flags are invalid.

## Decoder limits and validation

- Maximum complete file: 16 MiB.
- Maximum individual string: 1 MiB.
- Maximum functions: 4,096.
- Maximum global or local slots: 100,000.
- Maximum instructions per function: 100,000.
- Unknown type/opcode tags, invalid UTF-8, malformed flags, truncation, length mismatches, trailing payload, and checksum failures are rejected.
- Deserialization runs structural validation and typed bytecode verification before returning a program.

These bounds constrain file parsing, not aggregate VM heap usage. A valid file is not necessarily trusted or safe for execution. The checksum is not authentication; untrusted bytecode still requires the project's full future sandbox controls.

## Rust APIs

- `bytecode::serialize(&program)` returns a PHNB byte vector.
- `bytecode::deserialize(&bytes)` returns a structurally and typed-verified `BytecodeProgram`.
- The VM can execute a deserialized program using `bytecode::execute_bytecode(&program)`.
