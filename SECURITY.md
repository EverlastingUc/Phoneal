# Security Policy

Phoneal is an early experimental prototype and is not production-ready. Do not use it to execute untrusted programs.

The current interpreter has no sandbox, capability-based filesystem/network permissions, verified bytecode, signed package system, or independent security audit. Static type checking, lexical scopes, function arity checks, and the recursion limit are correctness and resource-bounding features, not security boundaries. The recursion limit is not a general CPU, memory, or wall-clock resource quota.

The security-first goal is not a claim that the current implementation is secure or unhackable.

Please report suspected vulnerabilities privately through GitHub's security reporting feature if enabled. Avoid publishing exploitable details in public issues.
