use phoneal::{
    compile_to_ir,
    ir::{TypedExpressionKind, TypedStatementKind},
    types::PhonealType,
};

#[test]
fn compiles_functions_and_control_flow_to_typed_ir() {
    let program = compile_to_ir(
        "function increment(value: integer) returns integer { return value + 1; } create mutable count = 0; while (count < 3) { set count = increment(count); } print(count);",
    )
    .unwrap();

    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].return_type, PhonealType::Integer);
    assert!(matches!(
        &program.functions[0].body[0].kind,
        TypedStatementKind::Return(_)
    ));
    assert!(matches!(
        &program.statements[1].kind,
        TypedStatementKind::While { .. }
    ));
}

#[test]
fn typed_ir_preserves_expression_result_types() {
    let program = compile_to_ir("create answer = 6 * 7; print(answer == 42);").unwrap();

    match &program.statements[1].kind {
        TypedStatementKind::Print(expression) => {
            assert_eq!(expression.ty, PhonealType::Boolean);
            assert!(matches!(
                &expression.kind,
                TypedExpressionKind::Binary { .. }
            ));
        }
        _ => panic!("expected print statement"),
    }
}

#[test]
fn rejects_programs_that_do_not_pass_semantic_analysis() {
    assert!(compile_to_ir("while (1) { print(1); }").is_err());
    assert!(compile_to_ir("continue;").is_err());
}
