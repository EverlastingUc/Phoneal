# Phoneal Security Architecture

**Status:** Threat model, capability policy primitives, bounded PHIP stream framing, typed payload schemas, per-session sequencing, and an experimental one-shot worker process are implemented. OS sandboxing, peer authentication, capability brokering, and worker resource enforcement are not implemented.

This document defines the boundary required before Phoneal can be described as safe for untrusted programs. The current AST interpreter and bytecode VM are experimental in-process execution engines. Runtime checks and bytecode verification are defense-in-depth, not a security boundary.

## 1. Security objectives

The sandbox must protect the host from a malicious Phoneal source file or PHNB bytecode file attempting to:

- read or modify files outside explicitly granted locations;
- access the network without an explicit grant;
- read environment variables, credentials, process arguments, or inherited handles;
- spawn processes or load native libraries;
- exhaust memory, CPU, wall-clock time, file descriptors, process slots, or output;
- exploit parser, decoder, verifier, VM, or host-broker implementation bugs;
- confuse the parent process through malformed or oversized IPC messages.

The design follows **deny by default**, **least privilege**, **separation of privilege**, **bounded input**, and **independent enforcement**.

## 2. Threat model

### Adversary

Assume the program author controls all source text, PHNB bytes, runtime input, and timing. Assume bytecode may be deliberately malformed rather than compiler-generated. Treat every worker-to-parent message as untrusted.

### Assets

- Host filesystem contents and integrity.
- Network access and internal services.
- Environment variables, credentials, and inherited process handles.
- Host CPU, memory, process table, file descriptors, and disk capacity.
- Parent/broker integrity and availability.
- Confidentiality and integrity of other concurrently running jobs.

### Trust boundaries

1. **CLI/API boundary:** source files, bytecode files, command arguments, and configuration are untrusted.
2. **Parent-to-worker boundary:** only bounded, versioned messages cross IPC.
3. **Worker boundary:** compiler, bytecode decoder, verifier, and VM are treated as potentially exploitable.
4. **Worker-to-broker boundary:** capability requests are untrusted and independently authorized by the parent.
5. **OS boundary:** the operating system must enforce process, filesystem, network, and resource restrictions independently of VM checks.

### Explicit non-goals

The initial threat model does not promise protection against kernel vulnerabilities, hypervisor compromise, hardware side channels, or denial of service against the entire host by a privileged administrator.

## 3. Target process architecture

The security boundary is a separate worker process. Running untrusted code in a thread, task, or ordinary in-process VM is insufficient.

```text
Phoneal CLI / Embedding Host
          |
          v
     Parent Broker
     - validate request size
     - load explicit policy
     - create isolated workspace
     - spawn restricted worker
     - enforce wall-clock deadline
     - authorize capability requests
     - cap output and IPC frames
          |
          | bounded, versioned IPC
          v
     Sandbox Worker
     - frontend/compiler
     - PHNB decoder
     - structural + typed verifier
     - bytecode VM
     - no ambient host access
          |
          | capability request only
          v
     Parent Capability Broker
     - validate capability handle
     - validate resource scope
     - perform narrowly scoped operation
     - return bounded result
```

Compilation of untrusted source must occur inside the worker, not in the privileged parent. The parent may perform basic byte-count admission checks, but it must not parse or execute untrusted language content before the worker boundary.

## 4. Capability model

Capabilities are explicit grants for one execution. No capability is inherited from the parent process or inferred from a path, hostname, environment variable, or user identity.

Candidate capability classes:

| Capability | Scope | Initial policy |
| --- | --- | --- |
| Console output | Bounded output channel | Explicit runtime channel; capped |
| Console input | One input stream | Denied unless granted |
| File read | Canonical path roots and read-only mode | Denied |
| File write | Canonical path roots and write mode | Denied |
| Network connect | Explicit host/port allowlist | Denied |
| Clock | Monotonic or wall-clock access | Denied unless granted |
| Randomness | OS-backed random source | Denied unless granted |
| Environment read | Named variables | Denied |
| Process spawn | No scope in initial design | Permanently denied in initial sandbox |

A capability grant must be narrow, per-run, and revocable when the worker exits. The worker receives opaque handles, not ambient OS permissions. The parent validates every request against the handle's capability and scope.

File access must defend against path traversal, symlink escape, hard-link surprises, and time-of-check/time-of-use races. Canonicalizing a path and then opening it later is not sufficient by itself; implementation must use race-resistant OS APIs and verify the opened object remains beneath the granted root.

Network access must resolve and validate destinations at connection time, not merely trust a hostname string checked earlier. Redirects, DNS rebinding, IPv4/IPv6 variants, and local/private address ranges need explicit policy.

### Current policy API

The Rust API exposes `phoneal::security::Capability`, `CapabilityPolicy`, and `CapabilityDenied`. `CapabilityPolicy::default()` denies every capability; grants are explicit and revocable. This is a policy data structure only. No VM opcode, standard-library function, host interface, or broker currently consumes these grants, and it must not be mistaken for OS-enforced authority.

## 5. Resource enforcement

VM counters remain useful for deterministic language-level limits, but the parent and operating system must enforce independent outer limits.

| Resource | Current in-process guard | Required outer enforcement |
| --- | --- | --- |
| Source bytes | 1 MiB frontend/CLI admission | Bounded IPC/input staging |
| PHNB bytes | 16 MiB decoder/CLI admission | Bounded IPC/input staging |
| VM instructions | 50,000 per run | CPU-time limit and parent deadline |
| Call depth | 128 | Worker stack/process memory limit |
| Operand stack | 100,000 values | OS memory limit |
| Text value | 1 MiB | Aggregate memory limit |
| Captured output | 1 MiB | Parent-side IPC/output cap |
| Heap/locals/globals | No aggregate quota | OS-enforced memory limit |
| Wall-clock time | No independent deadline | Parent timer kills worker |
| Processes | No process sandbox | OS process/job limit |
| File descriptors | No explicit cap | OS descriptor limit |

The parent deadline must not depend on the worker cooperating. On timeout or cancellation, the parent kills the worker and its entire process group/job, closes IPC, and discards partial untrusted state.

## 6. Worker launch requirements

Before the sandbox can be called functional, the launcher must:

- use an unprivileged identity;
- create a fresh, empty or tightly scoped working directory;
- scrub environment variables and process metadata not required by the worker;
- pass only the IPC descriptors/handles explicitly required;
- close all unrelated inherited descriptors/handles;
- disable network access by default;
- expose no host filesystem except explicitly granted mounts/roots;
- deny process creation and native library loading;
- apply OS-enforced memory, CPU, process, descriptor, and wall-clock limits;
- kill the entire worker process tree on completion, timeout, or protocol failure;
- avoid executing user code in the parent process.

Platform-specific enforcement must be implemented and tested independently. A portable interface must not silently fall back to an unrestricted process when a platform adapter is unavailable.

## 7. IPC protocol

PHIP version 1 uses the `PHIP` magic, a fixed-width little-endian header, request ID, payload length, CRC-32, and a 1 MiB payload cap. CRC-32 detects accidental corruption; it is not authentication.

The API exposes typed `Message` payloads and `CapabilityId` identifiers. `Frame::from_message` encodes a typed message, `Frame::message` validates a frame's payload against its message kind, and `Frame::decode_message` performs both framing and payload validation.

Payload contracts:

| Message | Payload |
| --- | --- |
| Hello | Minimum and maximum supported protocol versions (two little-endian u16 values); the range must include version 1 |
| RunSource | Non-empty UTF-8 source text |
| RunBytecode | Non-empty bytecode bytes |
| Stdout / Stderr | Opaque output bytes |
| CapabilityRequest | Exactly one known capability identifier |
| CapabilityResponse | Exactly one byte: 0 (denied) or 1 (granted) |
| Completed / Cancel | Empty payload |
| Error | Little-endian u16 error code followed by non-empty UTF-8 text |

Unknown message kinds and capability identifiers, invalid version ranges, invalid UTF-8, incorrect fixed payload lengths, invalid boolean encodings, and empty execution/error messages are rejected.

The library now reads and writes bounded frames on byte streams. `Session` requires a request-zero Hello handshake, enforces endpoint message direction, permits one active request, matches responses to that request, and rejects non-monotonic request IDs within the session. This does not provide peer authentication, capability authorization, or replay protection across newly created sessions. A valid frame or session transition is not proof that a message is authorized.

## 8. Experimental worker process

`phnl run-isolated <file.phnl>` starts a fresh copy of the Phoneal executable in hidden `__worker` mode. The parent reads source with the existing 1 MiB admission limit and sends it over stdin using PHIP frames. The worker performs compilation and bytecode execution, then returns bounded output/error frames over stdout. The worker starts with a cleared environment and null stderr; stdin/stdout are dedicated to the protocol. The parent validates the handshake and request ID and waits for worker termination.

This is process separation, not a sandbox. The worker still inherits the parent working directory, user identity, filesystem permissions, network access, OS limits, and process privileges. There is no wall-clock watchdog, process-tree containment, authentication, or capability broker. The hidden worker entry point is not a security boundary against a hostile local user.

## 9. Required security test matrix

The sandbox release gate must include:

1. Oversized source, PHNB, IPC, text, and output inputs.
2. Malformed PHNB headers, lengths, checksums, tags, jumps, stack types, and frame metadata.
3. Decoder/verifier/VM fuzzing with panic-free failure behavior.
4. Path traversal, symlink escape, hard links, and TOCTOU attempts.
5. Read/write attempts outside granted roots.
6. DNS rebinding, IPv4/IPv6 variants, localhost, link-local, and private network access.
7. Environment, inherited descriptor, process-spawn, and native-library access attempts.
8. CPU loops, deep recursion, stack exhaustion, aggregate heap exhaustion, and output floods.
9. Parent timeout, cancellation, worker crash, malformed IPC, and process-tree cleanup.
10. Concurrent jobs attempting to access one another's workspaces.
11. Platform-specific escape tests for every supported operating system.
12. Independent review of the threat model, broker, and OS launcher.

A passing unit-test suite is necessary but not sufficient. Security review and hostile-input testing are release gates.

## 10. Implementation sequence

1. Finalize capability names, scopes, and policy semantics.
2. Define and test bounded IPC framing and typed payload schemas.
3. Add request sequencing and replay/out-of-order rejection.
4. Build a parent/worker process split with no host capabilities.
5. Add the first supported OS sandbox adapter and fail-closed platform detection.
6. Add OS memory, CPU, process, descriptor, and parent wall-clock limits.
7. Add read-only workspace access through the broker.
8. Add scoped file writes only after path-race tests pass.
9. Add optional network access only after destination policy and DNS tests pass.
10. Fuzz the decoder, verifier, VM, and IPC parser.
11. Run the escape-test matrix and obtain independent security review.

## 11. Current security statement

Phoneal currently has typed bytecode verification, runtime checks, and in-process execution budgets. It does **not** have capability enforcement, a restricted host interface, filesystem/network isolation, an OS-enforced memory limit, a parent-enforced wall-clock deadline, or a worker-process sandbox.

Do not run untrusted Phoneal programs or PHNB files in a privileged process. Do not describe the current VM as secure, sandboxed, or escape-proof.
