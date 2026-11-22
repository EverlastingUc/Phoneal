# Security Policy

Phoneal is an experimental prototype and is not production-ready. Do not use either execution path to run untrusted programs.

The project has two execution paths:

- `run(source)` uses the original AST interpreter.
- `run_bytecode(source)` compiles through typed IR and executes the generated stack bytecode in the initial Phoneal VM.

Both paths enforce static types, lexical scope, checked integer arithmetic, bounded function-call depth, and per-run execution limits. The bytecode VM additionally validates instruction structure and typed operand-stack/control-flow consistency before execution, checks local/global slot metadata, enforces runtime value types and mutability, rejects global writes from functions, and enforces these limits:

- 50,000 executed VM instructions;
- 128 active function calls;
- 100,000 operand-stack values;
- 1 MiB per text value;
- 1 MiB captured output.

These are defensive runtime limits, not a security boundary. The VM does not impose an aggregate heap quota or wall-clock deadline. Text/local/global values can be copied, and the current limits do not prove total memory use is bounded.

## Missing security controls

Phoneal does not yet have:

- a security-grade bytecode verifier or a proof of stack-depth/type safety for arbitrary bytecode;
- aggregate heap accounting or OS-enforced memory limits;
- wall-clock timeouts enforced outside the VM process;
- a deny-by-default capability system;
- filesystem, network, process, or environment isolation;
- a sandbox process boundary;
- hostile-input fuzzing and independent security review.

Structural bytecode checks and typed stack-flow verification are intended to catch malformed compiler output and many invalid instruction streams. They do not prove global initialization across every call path, total memory use, or safe host-resource isolation, and must not be treated as protection against every malicious bytecode program.

The security-first goal is not a claim that Phoneal is secure or unhackable. A sandbox milestone will require a threat model, deny-by-default capability design, independent enforcement, resource accounting, hostile-input tests, and security review.

Please report suspected vulnerabilities privately through GitHub's security reporting feature if enabled. Avoid publishing exploitable details in public issues.
