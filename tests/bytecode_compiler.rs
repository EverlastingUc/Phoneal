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
        "create mutable count = 0; while (count < 3) { create snapshot = count; set count = count + 1; if (count == 2) { continue; } print(snapshot); }",
    ];
    for source in programs {
        assert_eq!(phoneal::run(source), run_bytecode(source), "program: {source}");
    }
}

#[test]
fn serialized_bytecode_can_be_loaded_and_executed() {
    let source = "function square(value: integer) returns integer { return value * value; } print(square(9));";
    let program = compile_to_bytecode(source).unwrap();
    let bytes = phoneal::bytecode::serialize(&program).unwrap();
    let loaded = phoneal::bytecode::deserialize(&bytes).unwrap();
    assert_eq!(phoneal::bytecode::execute_bytecode(&loaded).unwrap(), vec!["81"]);
}

#[test]
fn disassembler_exposes_function_and_instruction_details() {
    let program = compile_to_bytecode(
        "function twice(value: integer) returns integer { return value * 2; } print(twice(4));",
    ).unwrap();
    let listing = phoneal::bytecode::disassemble(&program).unwrap();
    assert!(listing.contains("twice(integer) -> integer"));
    assert!(listing.contains("Call #0 twice"));
    assert!(listing.contains("Multiply"));
}

#[test]
fn bytecode_vm_runtime_errors_preserve_source_locations() {
    let error = run_bytecode("print(1 / 0);").unwrap_err();
    assert!(error.location.is_some());
    assert!(error.message.contains("Division by zero"));
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
