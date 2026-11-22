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
