# Phoneal Security Architecture

**Status:** Threat model and target architecture documented. No process sandbox or capability enforcement is implemented yet.

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

## 7. IPC protocol requirements

The worker protocol must be versioned and length-prefixed. Every frame must have:

- a fixed magic and protocol version;
- a bounded frame length checked before allocation;
- a message type and request identifier;
- a strictly validated payload schema;
- explicit success/error responses;
- no implicit filesystem paths or arbitrary command strings.

Unknown message types, invalid lengths, duplicate or out-of-order request IDs, malformed UTF-8, oversized responses, and unexpected EOF must fail closed. The broker must not interpret a worker-provided path or hostname as authorization.

## 8. Required security test matrix

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

## 9. Implementation sequence

1. Finalize capability names, scopes, and policy semantics.
2. Define and test the bounded IPC protocol.
3. Build a parent/worker process split with no host capabilities.
4. Add the first supported OS sandbox adapter and fail-closed platform detection.
5. Add OS memory, CPU, process, descriptor, and parent wall-clock limits.
6. Add read-only workspace access through the broker.
7. Add scoped file writes only after path-race tests pass.
8. Add optional network access only after destination policy and DNS tests pass.
9. Fuzz the decoder, verifier, VM, and IPC parser.
10. Run the escape-test matrix and obtain independent security review.

## 10. Current security statement

Phoneal currently has typed bytecode verification, runtime checks, and in-process execution budgets. It does **not** have capability enforcement, a restricted host interface, filesystem/network isolation, an OS-enforced memory limit, a parent-enforced wall-clock deadline, or a worker-process sandbox.

Do not run untrusted Phoneal programs or PHNB files in a privileged process. Do not describe the current VM as secure, sandboxed, or escape-proof.
