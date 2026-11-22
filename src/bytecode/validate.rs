use super::{BytecodeFunction, BytecodeProgram, Opcode};
use crate::diagnostic::{Diagnostic, DiagnosticStage};

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticStage::Bytecode, message)
}

fn validate_function(
    function: &BytecodeFunction,
    program: &BytecodeProgram,
    is_entry: bool,
) -> Result<(), Diagnostic> {
    if function.code.is_empty() {
        return Err(invalid("Bytecode function has no instructions"));
    }
    if function.local_types.len() != function.local_count
        || function.local_mutability.len() != function.local_count
        || function.parameter_types.len() != function.parameter_count
        || function.parameter_count > function.local_count
    {
        return Err(invalid(format!("Bytecode function '{}' has inconsistent frame metadata", function.name)));
    }
    if !is_entry && function.return_type.is_none() {
        return Err(invalid(format!("Bytecode function '{}' has no return type", function.name)));
    }
    if function.parameter_types.iter().enumerate().any(|(slot, ty)| {
        function.local_types.get(slot) != Some(ty) || function.local_mutability.get(slot) != Some(&false)
    }) {
        return Err(invalid(format!("Bytecode function '{}' has inconsistent parameter slots", function.name)));
    }
    if is_entry && function.return_type.is_some() {
        return Err(invalid("Bytecode entry must not declare a function return type"));
    }
    if is_entry {
        if !matches!(function.code.last().map(|instruction| &instruction.opcode), Some(Opcode::Halt)) {
            return Err(invalid("Bytecode entry function must end with halt"));
        }
    } else if !matches!(function.code.last().map(|instruction| &instruction.opcode), Some(Opcode::Unreachable)) {
        return Err(invalid(format!("Bytecode function '{}' must end with unreachable", function.name)));
    }

    for (index, instruction) in function.code.iter().enumerate() {
        match &instruction.opcode {
            Opcode::Halt if !is_entry || index + 1 != function.code.len() => {
                return Err(invalid("Halt is only valid as the final entry instruction"));
            }
            Opcode::Unreachable if is_entry || index + 1 != function.code.len() => {
                return Err(invalid("Unreachable is only valid as the final function instruction"));
            }
            Opcode::LoadLocal(slot) | Opcode::StoreLocal(slot) if *slot >= function.local_count => {
                return Err(invalid(format!("Local slot {slot} is outside function '{}' frame", function.name)));
            }
            Opcode::ResetLocals { start, count } => {
                let end = start.checked_add(*count)
                    .ok_or_else(|| invalid("Local reset range overflows"))?;
                if *start < function.parameter_count || end > function.local_count {
                    return Err(invalid("Local reset range is outside the non-parameter local frame"));
                }
            }

            Opcode::LoadGlobal(slot) | Opcode::StoreGlobal(slot) if *slot >= program.global_count => {
                return Err(invalid(format!("Global slot {slot} is outside the program global frame")));
            }
            Opcode::Jump(target) | Opcode::JumpIfFalse(target) if *target >= function.code.len() => {
                return Err(invalid(format!("Jump target {target} is outside function '{}' code", function.name)));
            }
            Opcode::Call { function: target, argument_count } => {
                let callee = program.functions.get(*target)
                    .ok_or_else(|| invalid(format!("Call references unknown function index {target}")))?;
                if *argument_count != callee.parameter_count {
                    return Err(invalid(format!(
                        "Call to '{}' supplies {argument_count} argument(s), expected {}",
                        callee.name, callee.parameter_count
                    )));
                }
            }
            Opcode::Return if is_entry => return Err(invalid("Entry bytecode cannot return from a function")),
            _ => {}
        }
    }
    Ok(())
}

/// Checks compiler-output structure and instruction references.
///
/// This is not a security verifier: it does not establish stack safety, resource bounds,
/// capability isolation, or safe execution of hostile bytecode.
pub fn validate_structure(program: &BytecodeProgram) -> Result<(), Diagnostic> {
    if program.global_types.len() != program.global_count
        || program.global_mutability.len() != program.global_count
    {
        return Err(invalid("Bytecode program has inconsistent global metadata"));
    }
    for function in &program.functions {
        validate_function(function, program, false)?;
    }
    validate_function(&program.entry, program, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_bytecode;

    #[test]
    fn compiler_output_passes_structural_validation() {
        let program = compile_to_bytecode(
            "function double(value: integer) returns integer { return value * 2; } create mutable n = 0; while (n < 3) { set n = double(n + 1); } print(n);",
        ).unwrap();
        validate_structure(&program).unwrap();
    }

    #[test]
    fn rejects_inconsistent_parameter_slot_metadata() {
        let mut program = compile_to_bytecode(
            "function identity(value: integer) returns integer { return value; } print(identity(1));",
        ).unwrap();
        program.functions[0].parameter_types[0] = crate::types::PhonealType::Boolean;
        assert!(validate_structure(&program).unwrap_err().message.contains("parameter slots"));
    }

    #[test]
    fn rejects_out_of_range_jump_targets() {
        let mut program = compile_to_bytecode("print(true);").unwrap();
        program.entry.code[0].opcode = Opcode::Jump(usize::MAX);
        assert!(validate_structure(&program).unwrap_err().message.contains("Jump target"));
    }

    #[test]
    fn rejects_out_of_range_local_slots() {
        let mut program = compile_to_bytecode("function value() returns integer { create local = 1; return local; } print(value());").unwrap();
        program.functions[0].code[1].opcode = Opcode::LoadLocal(99);
        assert!(validate_structure(&program).unwrap_err().message.contains("Local slot"));
    }
}
