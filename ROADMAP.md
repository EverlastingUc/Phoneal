# Phoneal Roadmap

This document records the roadmap state established by the current project specification and verified repository work. It does not invent acceptance criteria for phases that have not yet been specified.

## Verified phases

### Phase 9 — Loops and control flow

**Complete.** The Rust interpreter supports bounded `while` execution, nested loops, nearest-loop `break` and `continue`, boolean-only conditions, return propagation through loops, and shared execution-budget enforcement.

### Phase 10 — Language consistency and correctness

**Complete.** Parser stack usage was hardened without reducing the established nesting limit. Function/global mutation semantics and assignment evaluation order are consistent between semantic analysis and runtime checks.

### Phase 11 — Compiler frontend

**Complete.** The lexer, parser, AST, semantic analysis, structured diagnostics, type inference, function signatures, and control-flow validation form the validated-AST frontend.

### Phase 12 — Typed Intermediate Representation

**Complete and CI-verified.**

- Explicitly typed expressions, declarations, assignments, calls, and function signatures.
- Structured `if` and `while` nodes, plus explicit `return`, `break`, and `continue`.
- Compiler-independent unary and binary operator enums.
- Source locations retained for literals, operators, parameters, and available statement/expression nodes.
- Independent IR validation for variable resolution, binding mutability, type consistency, function signatures, return coverage, boolean conditions, and loop-control placement.
- Public Rust API: `compile_to_ir(source)`.
- Executable example: `examples/typed_ir.phnl`.

The AST interpreter remains available. Typed IR is consumed by the bytecode compiler, and the resulting stack instructions can be executed by the initial Phoneal VM through `run_bytecode(source)`.

### Phase 13 — Bytecode Compiler

**Complete and CI-verified.**

- Public source API: `compile_to_bytecode(source)`.
- Public IR API: `bytecode::compile(&TypedProgram)`.
- Stack-oriented opcodes for primitive constants, local/global slots, arithmetic, comparisons, printing, function calls, returns, and control flow.
- Branch and loop jump patching, including nearest-loop `break` / `continue`.
- Short-circuit compilation for `and` and `or`.
- Source locations retained on emitted instructions.
- Bytecode compiler diagnostics have a distinct `bytecode` stage.
- Structural validation checks jump targets, local/global slots, function-call indices and arity, and function/entry terminators.
- Tests cover arithmetic, function calls, control-flow targets, short-circuit structure, global reads in functions, source-location retention, and malformed bytecode structures.

This phase intentionally stops at bytecode generation. VM execution is tracked separately in Phase 14.

Expected pipeline direction:

```text
Phoneal Source
    -> Lexer
    -> Parser
    -> Semantic Analysis
    -> Typed IR
    -> Bytecode Compiler
    -> Phoneal VM
    -> Security Architecture
```

### Phase 14 — Initial Bytecode Virtual Machine

**Complete and CI-verified.**

- Public source API: `run_bytecode(source)`.
- Public VM API: `bytecode::execute_bytecode(&BytecodeProgram)`.
- Typed runtime values for integers, text, and booleans.
- Call frames with parameter/local slots and return-value propagation, including recursion.
- Runtime type and mutability checks for local/global stores; functions cannot mutate global bindings.
- Checked integer arithmetic, text concatenation, comparisons, and short-circuit execution.
- Runtime diagnostics retain instruction source locations where available.
- Local declaration-instruction offsets let the VM create a fresh binding whenever a loop re-enters the declaration site, including nested loop/block locals.
- Limits: 50,000 instructions, 128 active calls, 100,000 operand-stack values, 1 MiB per text value, and 1 MiB captured output.
- Tests cover arithmetic, recursion, mutable locals/globals, read-only globals in functions, overflow, division by zero, short-circuiting, step limits, and malformed bytecode.

The VM is an execution engine, not a sandbox. It uses a typed bytecode verifier for operand-stack and control-flow consistency, but has no aggregate heap quota, wall-clock deadline, capability system, host-resource isolation, or complete security-grade bytecode verifier.

### Phase 15 — Typed Bytecode Verification

**Complete and CI-verified.**

- Independent data-flow analysis for every function and the entry instruction stream.
- Operand-stack type tracking for constants, local/global loads and stores, arithmetic, comparisons, equality, conditions, calls, returns, and output.
- Control-flow merge validation ensures all incoming paths agree on the operand-stack type sequence.
- Function calls are checked against parameter types and return types.
- Function bytecode cannot write global slots.
- Definite local initialization is tracked across branches and loops; reads from uninitialized local slots are rejected.
- Return and halt instructions require balanced operand stacks.
- The compiler verifies generated bytecode before returning it, and the VM verifies again before execution.

This is a typed correctness verifier, not a complete security boundary. It does not prove global initialization across all call paths or impose aggregate memory/wall-clock limits.

### Phase 16 — PHNB Bytecode Serialization

**Complete and CI-verified.**

- Versioned binary container with `PHNB` magic and version 2.
- Little-endian header, payload length, and CRC-32 integrity check.
- Serializes function metadata, primitive types, mutability, opcodes, and source locations.
- Deserializer bounds file size, strings, function count, slot counts, and instruction counts.
- Rejects invalid tags, flags, UTF-8, truncated payloads, length mismatches, trailing data, and checksum failures.
- Deserialization runs structural and typed bytecode verification before returning a program.
- Public APIs: `bytecode::serialize` and `bytecode::deserialize`.

CRC-32 detects accidental corruption; it is not cryptographic authentication. PHNB files are not trusted merely because they deserialize successfully.

### Phase 17 — Bytecode Disassembler

**Complete and CI-verified.**

- Public API: `bytecode::disassemble(&BytecodeProgram)`.
- Verifies the program before producing output.
- Prints function signatures, local counts, instruction offsets, opcodes, call targets, jump targets, and source locations.
- Tests cover functions, calls, branches, locations, and invalid bytecode rejection.

The listing is a debugging representation, not a source-level decompiler.

### Phase 18 — CLI Bytecode Workflow

**Complete and CI-verified.**

- `phnl build <file.phnl> --output <file.phnb>` compiles, verifies, and serializes a source program.
- `phnl inspect <file.phnb>` deserializes, verifies, and disassembles a bytecode file.
- `phnl run-bytecode <file.phnl>` compiles and executes source through the bytecode VM.
- Existing `phnl run` remains the AST interpreter path; `phnl check` remains the frontend validation path.
- End-to-end CLI tests cover build, inspect, bytecode execution, checking, and extension validation.

### Phase 19 — Precompiled Bytecode Execution CLI

**Complete and CI-verified.**

- `phnl execute <file.phnb>` reads an existing PHNB file.
- CLI reads are capped at 1 MiB for source and 16 MiB for bytecode before decoding.
- Deserialization performs file bounds, CRC-32, structural, and typed stack-flow verification before execution.
- The verified program runs in the bytecode VM and prints captured output.
- CLI integration tests execute a file produced by `phnl build`.

This command does not make PHNB files safe for untrusted use; the VM is not a sandbox.

### Phase 20 — CLI Help and Version Flags

**Complete and CI-verified.**

- `phnl --help` and `phnl -h` print command usage.
- `phnl --version` and `phnl -V` print the package version from Cargo metadata.
- CLI integration tests cover both flags.

### Phase 21 — Loop-Scoped Local Lifetimes

**Complete and CI-verified.**

- Added per-local declaration-instruction offsets to bytecode frame metadata.
- The VM treats execution of that exact instruction as a fresh binding initialization, allowing immutable and mutable locals declared inside loop bodies to be reinitialized on each iteration.
- Nested loops and blocks naturally reuse their declaration sites without clearing outer-scope locals.
- `break` and `continue` flow back through the loop condition/body entry, so the next iteration performs the reset.
- Typed verification tracks definite local initialization and permits repeated execution only at declared initialization sites.
- Tests cover immutable loop locals, nested loops, `continue`, serialization, and disassembly.

### Phase 22 — Security Architecture and Threat Model

**Architecture specification complete; enforcement not implemented.**

- Documented attacker model, assets, trust boundaries, and explicit non-goals.
- Defined a parent-broker / restricted-worker process architecture.
- Defined deny-by-default capability classes and resource-scoping requirements.
- Specified OS-enforced resource controls, bounded IPC, worker launch requirements, and a hostile-input/escape-test release gate.
- Explicitly kept source compilation inside the worker boundary for untrusted programs.

This phase delivers the design baseline only. It does not add a sandbox, capability enforcement, restricted host APIs, filesystem/network isolation, or OS-enforced resource limits. See [security-architecture.md](docs/security-architecture.md).

### Phase 23 — Capability Policy Primitives

**Implemented and CI-verified.**

- Added public `phoneal::security::Capability` names for console input, file read/write, network connect, clock, and randomness.
- Added `CapabilityPolicy`, defaulting to deny-all, with explicit grant, revoke, query, and authorization methods.
- Added stable grant iteration and typed denial errors.
- Unit tests cover default denial, exact grants, non-implication, revocation, and deterministic listing.

This is a policy model only. No VM operation, host API, IPC broker, or OS permission enforcement consumes these grants yet.

### Phase 24 — Bounded IPC Frame Protocol

**Complete and CI-verified.**

- Added PHIP protocol version 1 with fixed magic, version, message kind, reserved flags, request ID, payload length, and CRC-32.
- Added a 1 MiB maximum payload and strict length, magic, version, flags, kind, and checksum validation.
- Added frame kinds for handshake, source/bytecode execution, output, capability request/response, completion, error, and cancellation.
- Added round-trip tests for all message kinds and malformed/truncated/oversized frame tests.

This is a framing library only. It does not create IPC transports, validate message-specific payload schemas, authorize requests, or spawn a worker.

### Phase 25 — Typed IPC Payload Schemas

**Complete and CI-verified.**

- Added typed `Message` variants for every PHIP v1 message kind.
- Added explicit `CapabilityId` wire identifiers.
- Added strict payload validation for version ranges, UTF-8 source/error text, fixed-width fields, known capability identifiers, boolean encodings, and empty control messages.
- Added typed frame encode/decode helpers while retaining raw frame access for lower-level transport tests.

This phase defines message shape only. Request sequencing is tracked separately in Phase 26.

### Phase 26 — IPC Session Sequencing

**Complete and CI-verified.**

- Added parent and worker endpoint roles.
- Requires a request-zero protocol handshake before other messages.
- Enforces message direction for each endpoint.
- Tracks one active request per session.
- Requires strictly increasing, non-zero request IDs and rejects replayed or out-of-order requests within a session.
- Matches worker output, capability requests, completion, errors, cancellation, and capability responses to the active request.
- Keeps cancellation separate from completion; the worker must explicitly finish the request after handling cancellation.

This is a state-validation API only. It does not write to a transport, authenticate peers, authorize capability requests, or protect against replay across newly created sessions.

### Phase 27 — Parent/Worker Process Separation

**Complete and CI-verified.**

- Added bounded stream reads/writes for PHIP frames, including truncated-header and truncated-payload rejection.
- Added a hidden one-shot worker entry point to the Phoneal executable.
- Added `phnl run-isolated <file.phnl>`; the parent performs bounded file admission and sends source over PHIP stdin/stdout.
- The worker performs compilation and bytecode execution, then returns typed output, completion, or error frames.
- Parent and worker validate the Hello handshake and active request ID.
- Worker is launched with a cleared environment, piped protocol streams, and stderr disconnected from the protocol channel.
- Added end-to-end tests for successful isolated execution, missing handshake rejection, and language-error propagation.

This is an experimental process split only. The worker still inherits the parent OS identity, current directory, filesystem permissions, network access, and ambient OS privileges. There is no watchdog, OS resource limit, process-tree containment, peer authentication, or capability broker. Do not run untrusted programs with this command and treat it as a security sandbox.

## Latest verified implementation

- Code commit: `0e216ab5339d597693ff1cfc685cb8c96e70d431`
- CI: [Phoneal CI #45608071474](https://github.com/EverlastingUc/Phoneal/actions/runs/45608071474)
- Tests: **162 passed, 0 failed** (105 unit, 42 compiler-pipeline, 8 bytecode integration, 4 CLI, 3 typed IR)

## Explicit exclusions

Do not implement the Package Manager, Standard Library, or self-hosting compiler unless the project owner changes the scope.
