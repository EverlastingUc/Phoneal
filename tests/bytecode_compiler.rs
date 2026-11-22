use phoneal::{
    bytecode::Opcode,
    compile_to_bytecode, run_bytecode,
};

#[test]
fn public_api_compiles_source_to_bytecode() {
    let program = compile_to_bytecode("create answer = 6 * 7; print(answer);").unwrap();
    assert_eq!(program.global_count, 1);
    assert_eq!(program.entry.name, "<entry>");
    assert!(program.entry.code.iter().any(|instruction| instruction.opcode == Opcode::Multiply));
}

#[test]
fn bytecode_vm_matches_ast_interpreter_on_supported_programs() {
    let programs = [
        "create mutable total = 0; while (total < 4) { set total = total + 1; if (total == 2) { continue; } print(total); }",
        "function factorial(n: integer) returns integer { if (n <= 1) { return 1; } else { return n * factorial(n - 1); } } print(factorial(6));",
        "create outer = 1; { create outer = 2; print(outer); } print(outer);",
        "create message = \"Phone\" + \"al\"; create enabled = not false and (2 < 3); print(message); print(enabled);",
    ];
    for source in programs {
        assert_eq!(phoneal::run(source), run_bytecode(source), "program: {source}");
    }
}

#[test]
fn bytecode_retains_source_locations() {
    let program = compile_to_bytecode("print(42);").unwrap();
    assert!(program.entry.code.iter().any(|instruction| instruction.location.is_some()));
}

#[test]
fn compiler_rejects_semantically_invalid_source() {
    let error = compile_to_bytecode("create value = missing;").unwrap_err();
    assert!(error.message.contains("Undefined variable"));
}

#[test]
fn public_api_compiles_and_runs_bytecode() {
    assert_eq!(
        run_bytecode("create mutable total = 0; while (total < 3) { set total = total + 1; } print(total);"),
        Ok(vec!["3".to_owned()])
    );
}
