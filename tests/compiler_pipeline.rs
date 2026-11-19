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
