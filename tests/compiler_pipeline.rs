use phoneal::{compile_with_diagnostics, run};

#[test]
fn runs_a_complete_integer_program() {
    let output = run("let answer = (6 + 1) * 6; print(answer);").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn runs_string_concatenation_program() {
    let output = run("let greeting = \"Hello, \" + \"Phoneal\"; print(greeting);").unwrap();
    assert_eq!(output, ["Hello, Phoneal"]);
}

#[test]
fn check_collects_name_and_type_errors_in_one_pass() {
    let errors = compile_with_diagnostics("print(unknown); let bad = 1 + \"text\";").unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors[0].message.contains("Undefined variable"));
    assert!(errors[1].message.contains("cannot be applied"));
}

#[test]
fn valid_programs_are_accepted_by_check_pipeline() {
    assert!(compile_with_diagnostics("let count = 10 / 2; print(count);").is_ok());
}

#[test]
fn lexical_shadowing_runs_end_to_end() {
    let output =
        run("let color = \"blue\"; { let color = \"red\"; print(color); } print(color);").unwrap();
    assert_eq!(output, ["red", "blue"]);
}

#[test]
fn nested_block_diagnostics_keep_source_locations() {
    let errors = compile_with_diagnostics("{ print(unknown); }").unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].location.as_ref().unwrap().column, 9);
}

#[test]
fn function_without_parameters_returns_a_value() {
    let output = run("fn answer() -> integer { return 42; } print(answer());").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn function_can_read_a_previously_declared_global() {
    let output =
        run("let factor = 6; fn multiply(value: integer) -> integer { return value * factor; } print(multiply(7));")
            .unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn function_parameters_shadow_global_names() {
    let output = run(
        "let value = 100; fn identity(value: integer) -> integer { return value; } print(identity(9)); print(value);",
    )
    .unwrap();
    assert_eq!(output, ["9", "100"]);
}

#[test]
fn nested_calls_work_in_variable_initializers() {
    let output = run("fn inc(n: integer) -> integer { return n + 1; } fn twice(n: integer) -> integer { return n * 2; } let result = twice(inc(20)); print(result);").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn semantic_analysis_rejects_duplicate_function_names() {
    let errors =
        compile_with_diagnostics("fn f() -> integer { return 1; } fn f() -> integer { return 2; }")
            .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("already declared")));
}

#[test]
fn semantic_analysis_rejects_duplicate_parameter_names() {
    let errors = compile_with_diagnostics("fn f(x: integer, x: integer) -> integer { return x; }")
        .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Duplicate parameter")));
}

#[test]
fn semantic_analysis_rejects_nested_function_declarations() {
    let errors = compile_with_diagnostics("{ fn nested() { return 1; } }").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("top level")));
}

#[test]
fn semantic_analysis_rejects_return_outside_function() {
    let errors = compile_with_diagnostics("return 42;").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("only valid inside")));
}

#[test]
fn semantic_analysis_rejects_undefined_function() {
    let errors = compile_with_diagnostics("print(missing(1));").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Undefined function")));
}

#[test]
fn semantic_analysis_rejects_wrong_function_arity() {
    let errors = compile_with_diagnostics(
        "fn add(a: integer, b: integer) -> integer { return a + b; } print(add(1));",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expects 2 argument")));
}

#[test]
fn function_without_return_is_rejected_during_analysis() {
    let errors = compile_with_diagnostics("fn empty() -> integer { let value = 1; }").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not return")));
}

#[test]
fn recursive_calls_are_bounded() {
    let error = run("fn repeat(n: integer) -> integer { return repeat(n); } print(repeat(1));")
        .unwrap_err();
    assert!(error.message.contains("Maximum function call depth"));
}

#[test]
fn function_local_variables_do_not_escape() {
    let errors = compile_with_diagnostics(
        "fn make() -> integer { let hidden = 7; return hidden; } print(hidden);",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Undefined variable 'hidden'")));
}

#[test]
fn arithmetic_precedence_is_preserved() {
    assert_eq!(run("print(2 + 3 * 4);").unwrap(), ["14"]);
}

#[test]
fn string_concatenation_is_preserved() {
    assert_eq!(run("print(\"Phone\" + \"al\");").unwrap(), ["Phoneal"]);
}

#[test]
fn duplicate_variable_declarations_are_rejected() {
    let errors = compile_with_diagnostics("let x = 1; let x = 2;").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("already declared")));
}

#[test]
fn block_local_variables_do_not_escape() {
    let errors = compile_with_diagnostics("{ let local = 1; } print(local);").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Undefined variable")));
}

#[test]
fn integer_overflow_is_reported_at_runtime() {
    let error = run("print(9223372036854775807 + 1);").unwrap_err();
    assert!(error.message.contains("Integer overflow"));
}

#[test]
fn division_by_zero_is_reported_at_runtime() {
    let error = run("print(10 / 0);").unwrap_err();
    assert!(error.message.contains("Division by zero"));
}

#[test]
fn nested_scopes_can_read_outer_bindings() {
    assert_eq!(
        run("let base = 40; { let answer = base + 2; print(answer); }").unwrap(),
        ["42"]
    );
}

#[test]
fn typed_functions_accept_matching_arguments_and_returns() {
    assert_eq!(
        run("fn add(a: integer, b: integer) -> integer { return a + b; } print(add(20, 22));")
            .unwrap(),
        ["42"]
    );
}

#[test]
fn typed_string_functions_execute() {
    assert_eq!(run("fn greet(name: string) -> string { return \"Hello, \" + name; } print(greet(\"Phoneal\"));").unwrap(), ["Hello, Phoneal"]);
}

#[test]
fn compiler_rejects_wrong_function_argument_type() {
    let errors = compile_with_diagnostics(
        "fn identity(value: integer) -> integer { return value; } print(identity(\"bad\"));",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expects integer, received string")));
}

#[test]
fn compiler_rejects_wrong_function_return_type() {
    let errors =
        compile_with_diagnostics("fn invalid() -> integer { return \"bad\"; }").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Return type mismatch")));
}

#[test]
fn compiler_rejects_functions_without_guaranteed_return() {
    let errors = compile_with_diagnostics("fn invalid() -> integer { let x = 1; }").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not return")));
}

#[test]
fn compiler_accepts_return_inside_unconditional_nested_block() {
    assert!(compile_with_diagnostics("fn valid() -> integer { { return 42; } }").is_ok());
}

#[test]
fn typed_recursive_call_type_is_known_during_analysis() {
    assert!(
        compile_with_diagnostics("fn recurse(n: integer) -> integer { return recurse(n); }")
            .is_ok()
    );
}

#[test]
fn compiler_rejects_unknown_type_names() {
    let errors =
        compile_with_diagnostics("fn f(value: boolean) -> integer { return 1; }").unwrap_err();
    assert!(errors[0].message.contains("Unknown type"));
}
