# Security Policy

Phoneal is an experimental prototype and is not production-ready. Do not use the current interpreter to execute untrusted programs.

The compiler frontend now also lowers semantically checked programs into a typed intermediate representation and validates variable references, declarations, assignment mutability and types, function signatures, boolean conditions, return types, and loop-control placement. This is a compiler correctness layer, not a security boundary. The current implementation provides static type checking, lexical scopes, immutable-by-default bindings with explicit checked mutation, runtime assignment target/type checks, checked integer arithmetic, source diagnostics, a bounded function-call depth, and front-end limits on source size, token count, and parser nesting. These are correctness and limited robustness measures, not a security boundary.

These front-end limits reduce some accidental or hostile-input exhaustion risks. The interpreter enforces a shared 50,000-step per-run budget. Executed statements and each `while` condition check consume steps, so even an empty infinite loop terminates with a diagnostic. It also enforces a 128-call depth limit. These are interpreter-level limits, not OS-enforced wall-clock, heap, or output quotas, and they do not form a sandbox or security boundary.

There is currently no bytecode verifier, virtual machine, capability enforcement, filesystem/network isolation, process isolation, memory quota, instruction budget, or wall-clock execution limit. The current interpreter must be treated as trusted-code-only.

The security-first goal is not a claim that Phoneal is secure or unhackable. The sandbox milestone will require a threat model, deny-by-default capability design, independent enforcement, resource accounting, hostile-input tests, and security review.

Please report suspected vulnerabilities privately through GitHub's security reporting feature if enabled. Avoid publishing exploitable details in public issues.
