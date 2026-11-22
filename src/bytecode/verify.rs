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

#[derive(Clone)]
struct FlowState {
    stack: Vec<PhonealType>,
    initialized_locals: Vec<bool>,
}

fn enqueue(
    target: usize,
    incoming: &FlowState,
    states: &mut [Option<FlowState>],
    pending: &mut VecDeque<usize>,
) -> Result<(), Diagnostic> {
    let state = states.get_mut(target).ok_or_else(|| invalid("Control flow leaves the function instruction stream"))?;
    match state {
        Some(existing) => {
            if existing.stack != incoming.stack {
                return Err(invalid("Control-flow paths reach an instruction with incompatible operand stacks"));
            }
            let mut changed = false;
            for (existing_initialized, incoming_initialized) in existing.initialized_locals.iter_mut().zip(&incoming.initialized_locals) {
                let merged = *existing_initialized && *incoming_initialized;
                if merged != *existing_initialized {
                    *existing_initialized = merged;
                    changed = true;
                }
            }
            if changed {
                pending.push_back(target);
            }
            Ok(())
        }
        None => {
            *state = Some(incoming.clone());
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
    let mut initial_locals = vec![false; function.local_count];
    initial_locals[..function.parameter_count].fill(true);
    let mut states = vec![None; function.code.len()];
    let mut pending = VecDeque::new();
    states[0] = Some(FlowState { stack: Vec::new(), initialized_locals: initial_locals });
    pending.push_back(0);

    while let Some(ip) = pending.pop_front() {
        let mut state = states[ip].clone().expect("queued instructions have a flow state");
        let opcode = &function.code[ip].opcode;
        let mut successors = Vec::with_capacity(2);
        let mut terminal = false;

        match opcode {
            Opcode::PushInteger(_) => push_type(&mut state.stack, PhonealType::Integer)?,
            Opcode::PushText(_) => push_type(&mut state.stack, PhonealType::Text)?,
            Opcode::PushBoolean(_) => push_type(&mut state.stack, PhonealType::Boolean)?,
            Opcode::LoadLocal(slot) => {
                if !state.initialized_locals[*slot] {
                    return Err(invalid(format!("Read from uninitialized local slot {slot}")));
                }
                push_type(&mut state.stack, function.local_types[*slot])?;
            }
            Opcode::StoreLocal(slot) => {
                pop_type(&mut state.stack, Some(function.local_types[*slot]))?;
                let is_declaration = function.local_declarations[*slot] == Some(ip);
                if !is_declaration && !state.initialized_locals[*slot] {
                    return Err(invalid(format!("Assignment to uninitialized local slot {slot}")));
                }
                if !is_declaration && !function.local_mutability[*slot] {
                    return Err(invalid(format!("Assignment to immutable local slot {slot}")));
                }
                state.initialized_locals[*slot] = true;
            }

            Opcode::LoadGlobal(slot) => push_type(&mut state.stack, program.global_types[*slot])?,
            Opcode::StoreGlobal(slot) => {
                if !is_entry {
                    return Err(invalid("Function bytecode cannot write global slots"));
                }
                pop_type(&mut state.stack, Some(program.global_types[*slot]))?;
            }
            Opcode::Pop | Opcode::Print => {
                pop_type(&mut state.stack, None)?;
            }
            Opcode::Negate => {
                pop_type(&mut state.stack, Some(PhonealType::Integer))?;
                push_type(&mut state.stack, PhonealType::Integer)?;
            }
            Opcode::Not => {
                pop_type(&mut state.stack, Some(PhonealType::Boolean))?;
                push_type(&mut state.stack, PhonealType::Boolean)?;
            }
            Opcode::AddInteger | Opcode::Subtract | Opcode::Multiply | Opcode::Divide | Opcode::Remainder => {
                pop_type(&mut state.stack, Some(PhonealType::Integer))?;
                pop_type(&mut state.stack, Some(PhonealType::Integer))?;
                push_type(&mut state.stack, PhonealType::Integer)?;
            }
            Opcode::AddText => {
                pop_type(&mut state.stack, Some(PhonealType::Text))?;
                pop_type(&mut state.stack, Some(PhonealType::Text))?;
                push_type(&mut state.stack, PhonealType::Text)?;
            }
            Opcode::Less | Opcode::LessEqual | Opcode::Greater | Opcode::GreaterEqual => {
                pop_type(&mut state.stack, Some(PhonealType::Integer))?;
                pop_type(&mut state.stack, Some(PhonealType::Integer))?;
                push_type(&mut state.stack, PhonealType::Boolean)?;
            }
            Opcode::Equal | Opcode::NotEqual => {
                let right = pop_type(&mut state.stack, None)?;
                pop_type(&mut state.stack, Some(right))?;
                push_type(&mut state.stack, PhonealType::Boolean)?;
            }
            Opcode::Jump(target) => successors.push(*target),
            Opcode::JumpIfFalse(target) => {
                pop_type(&mut state.stack, Some(PhonealType::Boolean))?;
                successors.push(ip + 1);
                successors.push(*target);
            }
            Opcode::Call { function: target, argument_count } => {
                let callee = program.functions.get(*target).ok_or_else(|| invalid("Call references an unknown function"))?;
                if *argument_count != callee.parameter_count || *argument_count != callee.parameter_types.len() {
                    return Err(invalid("Call argument count does not match the target function signature"));
                }
                for expected in callee.parameter_types.iter().rev() {
                    pop_type(&mut state.stack, Some(*expected))?;
                }
                let result = callee.return_type.ok_or_else(|| invalid("Called function has no return type"))?;
                push_type(&mut state.stack, result)?;
            }
            Opcode::Return => {
                if is_entry {
                    return Err(invalid("Entry bytecode cannot return"));
                }
                let expected = function.return_type.ok_or_else(|| invalid("Function has no return type"))?;
                pop_type(&mut state.stack, Some(expected))?;
                if !state.stack.is_empty() {
                    return Err(invalid("Function return leaves extra values on the operand stack"));
                }
                terminal = true;
            }
            Opcode::Halt => {
                if !is_entry {
                    return Err(invalid("Function bytecode cannot halt"));
                }
                if !state.stack.is_empty() {
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
                enqueue(target, &state, &mut states, &mut pending)?;
            }
        }
    }
    Ok(())
}

/// Verifies stack types and control-flow stack consistency for compiler bytecode.
///
/// This is a typed bytecode verifier, not a complete security boundary. It tracks local-slot initialization
/// but does not prove global initialization across all call paths or isolate host resources.
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
    fn rejects_store_to_uninitialized_local_slot() {
        let mut program = compile_to_bytecode(
            "function maybe(flag: boolean) returns integer { create mutable value = 0; if (flag) { set value = 1; } return value; } print(maybe(true));",
        ).unwrap();
        let function = &mut program.functions[0];
        let stores = function.code.iter().enumerate()
            .filter_map(|(offset, instruction)| {
                matches!(&instruction.opcode, Opcode::StoreLocal(1)).then_some(offset)
            })
            .collect::<Vec<_>>();
        assert_eq!(stores.len(), 2);
        function.local_declarations[1] = Some(stores[1]);
        assert!(verify(&program).unwrap_err().message.contains("uninitialized local"));
    }

    #[test]
    fn rejects_store_to_immutable_local_outside_declaration() {
        let mut program = compile_to_bytecode(
            "function value() returns integer { create answer = 1; return answer; } print(value());",
        ).unwrap();
        let function = &mut program.functions[0];
        let return_offset = function.code.iter().position(|instruction| matches!(&instruction.opcode, Opcode::Return)).unwrap();
        function.code.insert(return_offset, super::super::Instruction { opcode: Opcode::PushInteger(2), location: None });
        function.code.insert(return_offset + 1, super::super::Instruction { opcode: Opcode::StoreLocal(0), location: None });
        assert!(verify(&program).unwrap_err().message.contains("immutable local"));
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
