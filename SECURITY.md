# Security Policy

Phoneal is an experimental prototype and is not production-ready. Do not use the current interpreter to execute untrusted programs.

The current implementation provides static type checking, lexical scopes, checked integer arithmetic, source diagnostics, a bounded function-call depth, and front-end limits on source size, token count, and parser nesting. These are correctness and limited robustness measures, not a security boundary.

These front-end limits reduce some accidental or hostile-input exhaustion risks. The interpreter also enforces a 50,000 executed-statement budget and a 128-call depth limit. The statement budget is not a wall-clock, heap, or output quota, and none of these controls forms a sandbox or security boundary.

There is currently no bytecode verifier, virtual machine, capability enforcement, filesystem/network isolation, process isolation, memory quota, instruction budget, or wall-clock execution limit. The current interpreter must be treated as trusted-code-only.

The security-first goal is not a claim that Phoneal is secure or unhackable. The sandbox milestone will require a threat model, deny-by-default capability design, independent enforcement, resource accounting, hostile-input tests, and security review.

Please report suspected vulnerabilities privately through GitHub's security reporting feature if enabled. Avoid publishing exploitable details in public issues.
