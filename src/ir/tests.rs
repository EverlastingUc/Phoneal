use super::*;
use crate::diagnostic::Diagnostic;

    use super::*;
    use crate::{lexer::Lexer, parser::Parser, semantic::analyze};

    fn lower_source(source: &str) -> Result<TypedProgram, Diagnostic> {
        let tokens = Lexer::new(source).scan()?;
        let statements = Parser::new(tokens).parse()?;
        analyze(&statements)?;
        lower(&statements)
    }

    #[test]
    fn lowers_typed_declarations_expressions_and_functions() {
        let program = lower_source(
            "function add(a: integer, b: integer) returns integer { return a + b; } create result = add(20, 22); print(result);",
        )
        .unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(
            program.functions[0].parameters[0].location,
            SourceLocation {
                line: 1,
                column: 14
            }
        );
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            &program.statements[0].kind,
            TypedStatementKind::Let {
                ty: PhonealType::Integer,
                ..
            }
        ));
    }

    #[test]
    fn preserves_available_source_locations() {
        let program = lower_source("create value = 42; print(value);").unwrap();
        assert_eq!(
            program.statements[0].location,
            Some(SourceLocation { line: 1, column: 8 })
        );
        match &program.statements[0].kind {
            TypedStatementKind::Let { value, .. } => assert_eq!(
                value.location,
                Some(SourceLocation {
                    line: 1,
                    column: 16
                })
            ),
            _ => panic!("expected declaration"),
        }
        match &program.statements[1].kind {
            TypedStatementKind::Print(expression) => {
                assert_eq!(
                    expression.location,
                    Some(SourceLocation {
                        line: 1,
                        column: 26
                    })
                );
            }
            _ => panic!("expected print statement"),
        }
    }

    #[test]
    fn rejects_invalid_ir_types_and_loop_control() {
        let mut program = lower_source("while (true) { break; }").unwrap();
        program.statements = vec![TypedStatement {
            kind: TypedStatementKind::Break,
            location: None,
        }];
        assert!(validate(&program).is_err());

        let mut program = lower_source("print(1);").unwrap();
        if let TypedStatementKind::Print(expression) = &mut program.statements[0].kind {
            expression.ty = PhonealType::Text;
        }
        assert!(validate(&program).is_err());
    }

    #[test]
    fn validator_rejects_undefined_variables_and_immutable_assignments() {
        let mut program = lower_source("print(1);").unwrap();
        if let TypedStatementKind::Print(expression) = &mut program.statements[0].kind {
            expression.kind = TypedExpressionKind::Variable("missing".into());
        }
        assert!(validate(&program).is_err());

        let mut program = lower_source("create value = 1;").unwrap();
        let location = SourceLocation { line: 1, column: 1 };
        program.statements.push(TypedStatement {
            kind: TypedStatementKind::Assign {
                name: "value".into(),
                value: TypedExpression {
                    kind: TypedExpressionKind::Integer(2),
                    ty: PhonealType::Integer,
                    location: Some(location.clone()),
                },
            },
            location: Some(location),
        });
        assert!(validate(&program).is_err());
    }

    #[test]
    fn validator_rejects_function_mutation_of_outer_bindings() {
        let mut program = lower_source(
            "create mutable total = 0; function update() returns integer { return total; }",
        )
        .unwrap();
        let location = SourceLocation { line: 1, column: 1 };
        program.functions[0].body[0] = TypedStatement {
            kind: TypedStatementKind::Assign {
                name: "total".into(),
                value: TypedExpression {
                    kind: TypedExpressionKind::Integer(1),
                    ty: PhonealType::Integer,
                    location: Some(location.clone()),
                },
            },
            location: Some(location),
        };
        assert!(validate(&program).is_err());
    }

    #[test]
    fn lowers_continue_break_and_return_inside_nested_control_flow() {
        let program = lower_source(
            "function select(value: integer) returns integer { while (true) { if (value == 0) { continue; } break; } return value; }",
        )
        .unwrap();
        assert!(matches!(
            &program.functions[0].body[0].kind,
            TypedStatementKind::While { .. }
        ));
    }

    #[test]
    fn validator_rejects_functions_without_return_coverage() {
        let mut program = lower_source(
            "function choose(flag: boolean) returns integer { if (flag) { return 1; } else { return 2; } }",
        )
        .unwrap();
        assert!(program.functions[0].body.pop().is_some());
        assert!(validate(&program).is_err());
    }

    #[test]
    fn rejects_invalid_source_before_lowering() {
        assert!(lower_source("while (1) { print(1); }").is_err());
        assert!(lower_source("break;").is_err());
    }
