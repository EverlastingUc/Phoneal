use super::{validate_structure, BytecodeFunction, BytecodeProgram, Opcode};
use crate::{
    diagnostic::{Diagnostic, DiagnosticStage},
    types::PhonealType,
};
use std::collections::VecDeque;

const MAX_VERIFIED_STACK_DEPTH: usize = 100_000;

fn invalid(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticStage::Bytecode, message)
}

fn pop_type(stack: &mut Vec<PhonealType>, expected: Option<PhonealType>) -> Result<PhonealType, Diagnostic> {
    let actual = stack.pop().ok_or_else(|| invalid("Bytecode operand stack underflow"))?;
    if expected.is_some_and(|expected| expected != actual) {
        return Err(invalid(format!(
            "Bytecode expected {} on the operand stack, found {}",
            expected.unwrap(),
            actual
        )));
    }
    Ok(actual)
}

fn push_type(stack: &mut Vec<PhonealType>, ty: PhonealType) -> Result<(), Diagnostic> {
    if stack.len() >= MAX_VERIFIED_STACK_DEPTH {
        return Err(invalid("Bytecode operand stack exceeds the verifier limit"));
    }
    stack.push(ty);
    Ok(())
}

fn enqueue(
    target: usize,
    stack: &[PhonealType],
    states: &mut [Option<Vec<PhonealType>>],
    pending: &mut VecDeque<usize>,
) -> Result<(), Diagnostic> {
    let state = states.get_mut(target).ok_or_else(|| invalid("Control flow leaves the function instruction stream"))?;
    match state {
        Some(existing) if existing != stack => {
            Err(invalid("Control-flow paths reach an instruction with incompatible operand stacks"))
        }
        Some(_) => Ok(()),
        None => {
            *state = Some(stack.to_vec());
            pending.push_back(target);
            Ok(())
        }
    }
}

fn verify_function(
    function: &BytecodeFunction,
    program: &BytecodeProgram,
    is_entry: bool,
) -> Result<(), Diagnostic> {
    let mut states = vec![None; function.code.len()];
    let mut pending = VecDeque::new();
    states[0] = Some(Vec::new());
    pending.push_back(0);

    while let Some(ip) = pending.pop_front() {
        let mut stack = states[ip].clone().expect("queued instructions have a stack state");
        let opcode = &function.code[ip].opcode;
        let mut successors = Vec::with_capacity(2);
        let mut terminal = false;

        match opcode {
            Opcode::PushInteger(_) => push_type(&mut stack, PhonealType::Integer)?,
            Opcode::PushText(_) => push_type(&mut stack, PhonealType::Text)?,
            Opcode::PushBoolean(_) => push_type(&mut stack, PhonealType::Boolean)?,
            Opcode::LoadLocal(slot) => push_type(&mut stack, function.local_types[*slot])?,
            Opcode::StoreLocal(slot) => {
                pop_type(&mut stack, Some(function.local_types[*slot]))?;
            }
            Opcode::LoadGlobal(slot) => push_type(&mut stack, program.global_types[*slot])?,
            Opcode::StoreGlobal(slot) => {
                if !is_entry {
                    return Err(invalid("Function bytecode cannot write global slots"));
                }
                pop_type(&mut stack, Some(program.global_types[*slot]))?;
            }
            Opcode::Pop | Opcode::Print => {
                pop_type(&mut stack, None)?;
            }
            Opcode::Negate => {
                pop_type(&mut stack, Some(PhonealType::Integer))?;
                push_type(&mut stack, PhonealType::Integer)?;
            }
            Opcode::Not => {
                pop_type(&mut stack, Some(PhonealType::Boolean))?;
                push_type(&mut stack, PhonealType::Boolean)?;
            }
            Opcode::AddInteger | Opcode::Subtract | Opcode::Multiply | Opcode::Divide | Opcode::Remainder => {
                pop_type(&mut stack, Some(PhonealType::Integer))?;
                pop_type(&mut stack, Some(PhonealType::Integer))?;
                push_type(&mut stack, PhonealType::Integer)?;
            }
            Opcode::AddText => {
                pop_type(&mut stack, Some(PhonealType::Text))?;
                pop_type(&mut stack, Some(PhonealType::Text))?;
                push_type(&mut stack, PhonealType::Text)?;
            }
            Opcode::Less | Opcode::LessEqual | Opcode::Greater | Opcode::GreaterEqual => {
                pop_type(&mut stack, Some(PhonealType::Integer))?;
                pop_type(&mut stack, Some(PhonealType::Integer))?;
                push_type(&mut stack, PhonealType::Boolean)?;
            }
            Opcode::Equal | Opcode::NotEqual => {
                let right = pop_type(&mut stack, None)?;
                pop_type(&mut stack, Some(right))?;
                push_type(&mut stack, PhonealType::Boolean)?;
            }
            Opcode::Jump(target) => successors.push(*target),
            Opcode::JumpIfFalse(target) => {
                pop_type(&mut stack, Some(PhonealType::Boolean))?;
                successors.push(ip + 1);
                successors.push(*target);
            }
            Opcode::Call { function: target, argument_count } => {
                let callee = program.functions.get(*target).ok_or_else(|| invalid("Call references an unknown function"))?;
                if *argument_count != callee.parameter_count || *argument_count != callee.parameter_types.len() {
                    return Err(invalid("Call argument count does not match the target function signature"));
                }
                for expected in callee.parameter_types.iter().rev() {
                    pop_type(&mut stack, Some(*expected))?;
                }
                let result = callee.return_type.ok_or_else(|| invalid("Called function has no return type"))?;
                push_type(&mut stack, result)?;
            }
            Opcode::Return => {
                if is_entry {
                    return Err(invalid("Entry bytecode cannot return"));
                }
                let expected = function.return_type.ok_or_else(|| invalid("Function has no return type"))?;
                pop_type(&mut stack, Some(expected))?;
                if !stack.is_empty() {
                    return Err(invalid("Function return leaves extra values on the operand stack"));
                }
                terminal = true;
            }
            Opcode::Halt => {
                if !is_entry {
                    return Err(invalid("Function bytecode cannot halt"));
                }
                if !stack.is_empty() {
                    return Err(invalid("Entry halt leaves values on the operand stack"));
                }
                terminal = true;
            }
            Opcode::Unreachable => {
                if is_entry {
                    return Err(invalid("Entry bytecode cannot contain an unreachable function terminator"));
                }
                terminal = true;
            }
        }

        if !terminal {
            if successors.is_empty() {
                successors.push(ip + 1);
            }
            for target in successors {
                enqueue(target, &stack, &mut states, &mut pending)?;
            }
        }
    }
    Ok(())
}

/// Verifies stack types and control-flow stack consistency for compiler bytecode.
///
/// This is a typed bytecode verifier, not a complete security boundary. It does not prove
/// definite initialization, enforce aggregate memory quotas, or isolate host resources.
pub fn verify(program: &BytecodeProgram) -> Result<(), Diagnostic> {
    validate_structure(program)?;
    for function in &program.functions {
        verify_function(function, program, false)?;
    }
    verify_function(&program.entry, program, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_bytecode;

    #[test]
    fn compiler_output_passes_typed_verification() {
        let program = compile_to_bytecode(
            "function twice(value: integer) returns integer { return value * 2; } create mutable n = 0; while (n < 3) { set n = twice(n + 1); } print(n);",
        ).unwrap();
        verify(&program).unwrap();
    }

    #[test]
    fn rejects_operand_stack_underflow() {
        let mut program = compile_to_bytecode("print(1);").unwrap();
        program.entry.code[0].opcode = Opcode::Print;
        assert!(verify(&program).unwrap_err().message.contains("underflow"));
    }

    #[test]
    fn rejects_operator_type_mismatches() {
        let mut program = compile_to_bytecode("print(1 + 2);").unwrap();
        let add = program.entry.code.iter().position(|instruction| matches!(&instruction.opcode, Opcode::AddInteger)).unwrap();
        program.entry.code[add].opcode = Opcode::AddText;
        assert!(verify(&program).unwrap_err().message.contains("expected text"));
    }

    #[test]
    fn rejects_incompatible_branch_stack_states() {
        let mut program = compile_to_bytecode(
            "if (true) { print(1); } else { print(2); }",
        ).unwrap();
        let else_print = program.entry.code.iter().rposition(|instruction| matches!(&instruction.opcode, Opcode::Print)).unwrap();
        program.entry.code[else_print].opcode = Opcode::PushInteger(2);
        assert!(verify(&program).unwrap_err().message.contains("incompatible operand stacks"));
    }

    #[test]
    fn rejects_global_writes_from_functions() {
        let mut program = compile_to_bytecode("create mutable total = 0; function read() returns integer { return total; } print(read());").unwrap();
        let function = &mut program.functions[0];
        function.code.insert(0, super::super::Instruction { opcode: Opcode::PushInteger(1), location: None });
        function.code.insert(1, super::super::Instruction { opcode: Opcode::StoreGlobal(0), location: None });
        assert!(verify(&program).unwrap_err().message.contains("cannot write global"));
    }
}
