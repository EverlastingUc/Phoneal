use phoneal::{compile_with_diagnostics, run};

#[test]
fn runs_a_complete_integer_program() {
    let output = run("create answer = (6 + 1) * 6; print(answer);").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn runs_string_concatenation_program() {
    let output = run("create greeting = \"Hello, \" + \"Phoneal\"; print(greeting);").unwrap();
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
    assert!(compile_with_diagnostics("create count = 10 / 2; print(count);").is_ok());
}

#[test]
fn lexical_shadowing_runs_end_to_end() {
    let output =
        run("create color = \"blue\"; { let color = \"red\"; print(color); } print(color);")
            .unwrap();
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
    let output = run("function answer() returns integer { return 42; } print(answer());").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn function_can_read_a_previously_declared_global() {
    let output =
        run("create factor = 6; function multiply(value: integer) returns integer { return value * factor; } print(multiply(7));")
            .unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn function_parameters_shadow_global_names() {
    let output = run(
        "create value = 100; function identity(value: integer) returns integer { return value; } print(identity(9)); print(value);",
    )
    .unwrap();
    assert_eq!(output, ["9", "100"]);
}

#[test]
fn nested_calls_work_in_variable_initializers() {
    let output = run("function inc(n: integer) returns integer { return n + 1; } function twice(n: integer) returns integer { return n * 2; } create result = twice(inc(20)); print(result);").unwrap();
    assert_eq!(output, ["42"]);
}

#[test]
fn semantic_analysis_rejects_duplicate_function_names() {
    let errors = compile_with_diagnostics(
        "function f() returns integer { return 1; } function f() returns integer { return 2; }",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("already declared")));
}

#[test]
fn semantic_analysis_rejects_duplicate_parameter_names() {
    let errors = compile_with_diagnostics(
        "function f(x: integer, x: integer) returns integer { return x; }",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Duplicate parameter")));
}

#[test]
fn semantic_analysis_rejects_nested_function_declarations() {
    let errors = compile_with_diagnostics("{ function nested() returns integer { return 1; } }")
        .unwrap_err();
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
        "function add(a: integer, b: integer) returns integer { return a + b; } print(add(1));",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expects 2 argument")));
}

#[test]
fn function_without_return_is_rejected_during_analysis() {
    let errors = compile_with_diagnostics("function empty() returns integer { create value = 1; }")
        .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not return")));
}

#[test]
fn recursive_calls_are_bounded() {
    let error =
        run("function repeat(n: integer) returns integer { return repeat(n); } print(repeat(1));")
            .unwrap_err();
    assert!(error.message.contains("Maximum function call depth"));
}

#[test]
fn function_local_variables_do_not_escape() {
    let errors = compile_with_diagnostics(
        "function make() returns integer { create hidden = 7; return hidden; } print(hidden);",
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
    let errors = compile_with_diagnostics("create x = 1; create x = 2;").unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("already declared")));
}

#[test]
fn block_local_variables_do_not_escape() {
    let errors = compile_with_diagnostics("{ create local = 1; } print(local);").unwrap_err();
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
        run("create base = 40; { create answer = base + 2; print(answer); }").unwrap(),
        ["42"]
    );
}

#[test]
fn typed_functions_accept_matching_arguments_and_returns() {
    assert_eq!(
        run("function add(a: integer, b: integer) returns integer { return a + b; } print(add(20, 22));")
            .unwrap(),
        ["42"]
    );
}

#[test]
fn typed_string_functions_execute() {
    assert_eq!(run("function greet(name: text) returns string { return \"Hello, \" + name; } print(greet(\"Phoneal\"));").unwrap(), ["Hello, Phoneal"]);
}

#[test]
fn compiler_rejects_wrong_function_argument_type() {
    let errors = compile_with_diagnostics(
        "function identity(value: integer) returns integer { return value; } print(identity(\"bad\"));",
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("expects integer, received text")));
}

#[test]
fn compiler_rejects_wrong_function_return_type() {
    let errors = compile_with_diagnostics("function invalid() returns integer { return \"bad\"; }")
        .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("Return type mismatch")));
}

#[test]
fn compiler_rejects_functions_without_guaranteed_return() {
    let errors = compile_with_diagnostics("function invalid() returns integer { create x = 1; }")
        .unwrap_err();
    assert!(errors
        .iter()
        .any(|error| error.message.contains("does not return")));
}

#[test]
fn compiler_accepts_return_inside_unconditional_nested_block() {
    assert!(
        compile_with_diagnostics("function valid() returns integer { { return 42; } }").is_ok()
    );
}

#[test]
fn typed_recursive_call_type_is_known_during_analysis() {
    assert!(compile_with_diagnostics(
        "function recurse(n: integer) returns integer { return recurse(n); }"
    )
    .is_ok());
}

#[test]
fn compiler_rejects_unknown_type_names() {
    let errors =
        compile_with_diagnostics("function f(value: decimal) returns integer { return 1; }")
            .unwrap_err();
    assert!(errors[0].message.contains("Unknown type"));
}

#[test]
fn boolean_conditions_and_short_circuit_execute_end_to_end() {
    assert_eq!(run("create age = 20; if (age >= 18 and true) { print(\"adult\"); } else { print(\"minor\"); } if (false and 1 / 0 == 0) { print(\"wrong\"); } else { print(\"short-circuited\"); }").unwrap(),["adult","short-circuited"]);
}
#[test]
fn recursive_factorial_executes_with_if_else() {
    assert_eq!(run("function factorial(n: integer) returns integer { if (n <= 1) { return 1; } else { return n * factorial(n - 1); } } print(factorial(5));").unwrap(),["120"]);
}
#[test]
fn function_return_analysis_requires_both_if_branches() {
    assert!(compile_with_diagnostics("function choose(flag: boolean) returns integer { if (flag) { return 1; } else { return 2; } }").is_ok());
    assert!(compile_with_diagnostics(
        "function choose(flag: boolean) returns integer { if (flag) { return 1; } }"
    )
    .is_err());
}
#[test]
fn rejects_integer_as_boolean_condition() {
    let errors = compile_with_diagnostics("if (42) { print(1); }").unwrap_err();
    assert!(errors.iter().any(|e| e.message.contains("must be boolean")));
}
#[test]
fn old_abbreviated_syntax_is_not_accepted() {
    assert!(compile_with_diagnostics("let value = 1;").is_err());
    assert!(compile_with_diagnostics("fn value() -> integer { return 1; }").is_err());
}
