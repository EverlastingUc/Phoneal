use super::{verify, BytecodeFunction, BytecodeProgram, Opcode};
use crate::{
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    types::PhonealType,
};
use std::fmt;

const MAX_EXECUTION_STEPS: usize = 50_000;
const MAX_CALL_DEPTH: usize = 128;
const MAX_STACK_VALUES: usize = 100_000;
const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_OUTPUT_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Text(String),
    Boolean(bool),
}

impl Value {
    fn same_type(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Integer(_), Self::Integer(_))
                | (Self::Text(_), Self::Text(_))
                | (Self::Boolean(_), Self::Boolean(_))
        )
    }

    fn has_type(&self, ty: PhonealType) -> bool {
        matches!(
            (self, ty),
            (Self::Integer(_), PhonealType::Integer)
                | (Self::Text(_), PhonealType::Text)
                | (Self::Boolean(_), PhonealType::Boolean)
        )
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => write!(formatter, "{value}"),
            Self::Text(value) => formatter.write_str(value),
            Self::Boolean(value) => write!(formatter, "{value}"),
        }
    }
}

struct Frame {
    function: Option<usize>,
    instruction: usize,
    locals: Vec<Option<Value>>,
    stack_base: usize,
}

struct Vm<'a> {
    program: &'a BytecodeProgram,
    globals: Vec<Option<Value>>,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    output: Vec<String>,
    output_bytes: usize,
    steps: usize,
}

impl<'a> Vm<'a> {
    fn new(program: &'a BytecodeProgram) -> Self {
        Self {
            program,
            globals: vec![None; program.global_count],
            stack: Vec::new(),
            frames: vec![Frame {
                function: None,
                instruction: 0,
                locals: vec![None; program.entry.local_count],
                stack_base: 0,
            }],
            output: Vec::new(),
            output_bytes: 0,
            steps: 0,
        }
    }

    fn function(&self, frame: usize) -> &BytecodeFunction {
        match self.frames[frame].function {
            Some(index) => &self.program.functions[index],
            None => &self.program.entry,
        }
    }

    fn fail(message: impl Into<String>, location: Option<&SourceLocation>) -> Diagnostic {
        match location {
            Some(location) => Diagnostic::at(
                DiagnosticStage::Runtime,
                message,
                location.line,
                location.column,
            ),
            None => Diagnostic::new(DiagnosticStage::Runtime, message),
        }
    }

    fn pop(&mut self, location: Option<&SourceLocation>) -> Result<Value, Diagnostic> {
        self.stack
            .pop()
            .ok_or_else(|| Self::fail("Bytecode stack underflow", location))
    }

    fn push(&mut self, value: Value, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        if self.stack.len() >= MAX_STACK_VALUES {
            return Err(Self::fail("Bytecode operand stack limit exceeded", location));
        }
        if let Value::Text(text) = &value {
            if text.len() > MAX_TEXT_BYTES {
                return Err(Self::fail("Text value exceeds the VM size limit", location));
            }
        }
        self.stack.push(value);
        Ok(())
    }

    fn local_metadata(&self, frame_index: usize, slot: usize, location: Option<&SourceLocation>) -> Result<(PhonealType, bool), Diagnostic> {
        let function = self.function(frame_index);
        let ty = *function.local_types.get(slot)
            .ok_or_else(|| Self::fail("Local slot is outside the current frame", location))?;
        let mutable = *function.local_mutability.get(slot)
            .ok_or_else(|| Self::fail("Local mutability metadata is missing", location))?;
        Ok((ty, mutable))
    }

    fn load_local(&mut self, frame_index: usize, slot: usize, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        let value = self.frames[frame_index].locals.get(slot)
            .ok_or_else(|| Self::fail("Local slot is outside the current frame", location))?
            .clone()
            .ok_or_else(|| Self::fail("Read from an uninitialized local variable", location))?;
        self.push(value, location)
    }

    fn store_local(&mut self, frame_index: usize, slot: usize, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        let value = self.pop(location)?;
        let (expected, mutable) = self.local_metadata(frame_index, slot, location)?;
        let target = self.frames[frame_index].locals.get_mut(slot)
            .ok_or_else(|| Self::fail("Local slot is outside the current frame", location))?;
        if !value.has_type(expected) {
            return Err(Self::fail("Local assignment violates its declared type", location));
        }
        if target.is_some() && !mutable {
            return Err(Self::fail("Cannot assign to an immutable local variable", location));
        }
        *target = Some(value);
        Ok(())
    }

    fn load_global(&mut self, slot: usize, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        let value = self.globals.get(slot)
            .ok_or_else(|| Self::fail("Global slot is outside the program frame", location))?
            .clone()
            .ok_or_else(|| Self::fail("Read from an uninitialized global variable", location))?;
        self.push(value, location)
    }

    fn store_global(&mut self, frame_index: usize, slot: usize, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        if self.frames[frame_index].function.is_some() {
            return Err(Self::fail("Functions cannot mutate global bindings", location));
        }
        let value = self.pop(location)?;
        let expected = *self.program.global_types.get(slot)
            .ok_or_else(|| Self::fail("Global slot is outside the program frame", location))?;
        let mutable = *self.program.global_mutability.get(slot)
            .ok_or_else(|| Self::fail("Global mutability metadata is missing", location))?;
        let target = self.globals.get_mut(slot)
            .ok_or_else(|| Self::fail("Global slot is outside the program frame", location))?;
        if !value.has_type(expected) {
            return Err(Self::fail("Global assignment violates its declared type", location));
        }
        if target.is_some() && !mutable {
            return Err(Self::fail("Cannot assign to an immutable global variable", location));
        }
        *target = Some(value);
        Ok(())
    }

    fn call(&mut self, function: usize, argument_count: usize, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        let (parameter_count, parameter_types, local_count) = {
            let callee = self.program.functions.get(function)
                .ok_or_else(|| Self::fail("Call references an unknown function", location))?;
            (callee.parameter_count, callee.parameter_types.clone(), callee.local_count)
        };
        if argument_count != parameter_count || argument_count != parameter_types.len() {
            return Err(Self::fail("Function call argument count does not match its signature", location));
        }
        if self.frames.len().saturating_sub(1) >= MAX_CALL_DEPTH {
            return Err(Self::fail(format!("Maximum function call depth ({MAX_CALL_DEPTH}) exceeded"), location));
        }
        if self.stack.len() < argument_count {
            return Err(Self::fail("Not enough arguments on the bytecode stack", location));
        }
        let mut arguments = Vec::with_capacity(argument_count);
        for _ in 0..argument_count {
            arguments.push(self.pop(location)?);
        }
        arguments.reverse();
        for (argument, expected) in arguments.iter().zip(&parameter_types) {
            if !argument.has_type(*expected) {
                return Err(Self::fail("Function argument type does not match its signature", location));
            }
        }
        let mut locals = vec![None; local_count];
        for (slot, argument) in arguments.into_iter().enumerate() {
            locals[slot] = Some(argument);
        }
        self.frames.push(Frame {
            function: Some(function),
            instruction: 0,
            locals,
            stack_base: self.stack.len(),
        });
        Ok(())
    }

    fn return_from_function(&mut self, location: Option<&SourceLocation>) -> Result<(), Diagnostic> {
        let value = self.pop(location)?;
        let (function_index, base) = {
            let frame = self.frames.last().ok_or_else(|| Self::fail("Return without a call frame", location))?;
            let function_index = frame.function.ok_or_else(|| Self::fail("Entry bytecode cannot return", location))?;
            (function_index, frame.stack_base)
        };
        let expected = self.program.functions[function_index].return_type
            .ok_or_else(|| Self::fail("Function return type metadata is missing", location))?;
        if !value.has_type(expected) {
            return Err(Self::fail("Function returned a value of the wrong type", location));
        }
        if self.stack.len() != base {
            return Err(Self::fail("Function returned with an unbalanced operand stack", location));
        }
        self.frames.pop();
        self.stack.truncate(base);
        self.push(value, location)
    }

    fn execute(mut self) -> Result<Vec<String>, Diagnostic> {
        loop {
            if self.steps >= MAX_EXECUTION_STEPS {
                return Err(Self::fail("VM execution step budget exceeded", None));
            }
            self.steps += 1;
            let frame_index = self.frames.len() - 1;
            let function = self.function(frame_index);
            let ip = self.frames[frame_index].instruction;
            let instruction = function.code.get(ip)
                .ok_or_else(|| Self::fail("Instruction pointer escaped function bytecode", None))?;
            let opcode = instruction.opcode.clone();
            let location = instruction.location.clone();
            self.frames[frame_index].instruction += 1;

            match opcode {
                Opcode::PushInteger(value) => self.push(Value::Integer(value), location.as_ref())?,
                Opcode::PushText(value) => self.push(Value::Text(value), location.as_ref())?,
                Opcode::PushBoolean(value) => self.push(Value::Boolean(value), location.as_ref())?,
                Opcode::LoadLocal(slot) => self.load_local(frame_index, slot, location.as_ref())?,
                Opcode::StoreLocal(slot) => self.store_local(frame_index, slot, location.as_ref())?,
                Opcode::LoadGlobal(slot) => self.load_global(slot, location.as_ref())?,
                Opcode::StoreGlobal(slot) => self.store_global(frame_index, slot, location.as_ref())?,
                Opcode::Pop => { self.pop(location.as_ref())?; }
                Opcode::Print => {
                    let value = self.pop(location.as_ref())?;
                    let rendered = value.to_string();
                    self.output_bytes = self.output_bytes.saturating_add(rendered.len());
                    if self.output_bytes > MAX_OUTPUT_BYTES {
                        return Err(Self::fail("VM output limit exceeded", location.as_ref()));
                    }
                    self.output.push(rendered);
                }
                Opcode::Negate => {
                    let value = self.pop(location.as_ref())?;
                    let Value::Integer(value) = value else {
                        return Err(Self::fail("Negate expects an integer", location.as_ref()));
                    };
                    let value = value.checked_neg().ok_or_else(|| Self::fail("Integer overflow", location.as_ref()))?;
                    self.push(Value::Integer(value), location.as_ref())?;
                }
                Opcode::Not => {
                    let value = self.pop(location.as_ref())?;
                    let Value::Boolean(value) = value else {
                        return Err(Self::fail("Not expects a boolean", location.as_ref()));
                    };
                    self.push(Value::Boolean(!value), location.as_ref())?;
                }
                Opcode::AddInteger | Opcode::Subtract | Opcode::Multiply | Opcode::Divide | Opcode::Remainder => {
                    let right = self.pop(location.as_ref())?;
                    let left = self.pop(location.as_ref())?;
                    let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
                        return Err(Self::fail("Integer arithmetic received a non-integer operand", location.as_ref()));
                    };
                    let result = match opcode {
                        Opcode::AddInteger => left.checked_add(right),
                        Opcode::Subtract => left.checked_sub(right),
                        Opcode::Multiply => left.checked_mul(right),
                        Opcode::Divide if right == 0 => return Err(Self::fail("Division by zero", location.as_ref())),
                        Opcode::Divide => left.checked_div(right),
                        Opcode::Remainder if right == 0 => return Err(Self::fail("Division by zero", location.as_ref())),
                        Opcode::Remainder => left.checked_rem(right),
                        _ => unreachable!(),
                    }.ok_or_else(|| Self::fail("Integer overflow", location.as_ref()))?;
                    self.push(Value::Integer(result), location.as_ref())?;
                }
                Opcode::AddText => {
                    let right = self.pop(location.as_ref())?;
                    let left = self.pop(location.as_ref())?;
                    let (Value::Text(left), Value::Text(right)) = (left, right) else {
                        return Err(Self::fail("Text concatenation received a non-text operand", location.as_ref()));
                    };
                    if left.len().saturating_add(right.len()) > MAX_TEXT_BYTES {
                        return Err(Self::fail("Text value exceeds the VM size limit", location.as_ref()));
                    }
                    self.push(Value::Text(left + &right), location.as_ref())?;
                }
                Opcode::Less | Opcode::LessEqual | Opcode::Greater | Opcode::GreaterEqual => {
                    let right = self.pop(location.as_ref())?;
                    let left = self.pop(location.as_ref())?;
                    let (Value::Integer(left), Value::Integer(right)) = (left, right) else {
                        return Err(Self::fail("Ordered comparison expects integer operands", location.as_ref()));
                    };
                    let result = match opcode {
                        Opcode::Less => left < right,
                        Opcode::LessEqual => left <= right,
                        Opcode::Greater => left > right,
                        Opcode::GreaterEqual => left >= right,
                        _ => unreachable!(),
                    };
                    self.push(Value::Boolean(result), location.as_ref())?;
                }
                Opcode::Equal | Opcode::NotEqual => {
                    let right = self.pop(location.as_ref())?;
                    let left = self.pop(location.as_ref())?;
                    if !left.same_type(&right) {
                        return Err(Self::fail("Equality comparison received values of different types", location.as_ref()));
                    }
                    let equal = left == right;
                    self.push(Value::Boolean(if matches!(opcode, Opcode::Equal) { equal } else { !equal }), location.as_ref())?;
                }
                Opcode::Jump(target) => self.frames[frame_index].instruction = target,
                Opcode::JumpIfFalse(target) => {
                    let condition = self.pop(location.as_ref())?;
                    let Value::Boolean(condition) = condition else {
                        return Err(Self::fail("Conditional jump expects a boolean", location.as_ref()));
                    };
                    if !condition {
                        self.frames[frame_index].instruction = target;
                    }
                }
                Opcode::Call { function, argument_count } => self.call(function, argument_count, location.as_ref())?,
                Opcode::Return => self.return_from_function(location.as_ref())?,
                Opcode::Halt => {
                    if self.frames.len() != 1 || !self.stack.is_empty() {
                        return Err(Self::fail("Entry halted with active calls or an unbalanced operand stack", location.as_ref()));
                    }
                    return Ok(self.output);
                }
                Opcode::Unreachable => return Err(Self::fail("Reached an unreachable function terminator", location.as_ref())),
            }
        }
    }
}

/// Executes compiler-produced bytecode with bounded steps, call depth, operand stack,
/// individual text values, and captured output. This VM is not a security sandbox.
pub fn execute(program: &BytecodeProgram) -> Result<Vec<String>, Diagnostic> {
    verify(program).map_err(|error| {
        Diagnostic::new(DiagnosticStage::Runtime, format!("Invalid bytecode structure: {}", error.message))
    })?;
    Vm::new(program).execute()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_bytecode;

    fn run(source: &str) -> Result<Vec<String>, String> {
        let program = compile_to_bytecode(source).map_err(|error| error.to_string())?;
        execute(&program).map_err(|error| error.to_string())
    }

    #[test]
    fn executes_arithmetic_functions_and_output() {
        assert_eq!(
            run("function add(a: integer, b: integer) returns integer { return a + b; } print(add(20, 22));"),
            Ok(vec!["42".to_owned()])
        );
    }

    #[test]
    fn executes_mutable_locals_and_globals() {
        assert_eq!(
            run("create mutable total = 0; function double(n: integer) returns integer { return n * 2; } while (total < 3) { set total = total + 1; print(double(total)); }"),
            Ok(vec!["2".to_owned(), "4".to_owned(), "6".to_owned()])
        );
    }

    #[test]
    fn enforces_immutable_bindings_at_runtime() {
        let mut program = compile_to_bytecode("create value = 1; print(value);").unwrap();
        let store = program.entry.code.iter().position(|instruction| matches!(&instruction.opcode, Opcode::StoreGlobal(_))).unwrap();
        let slot = match &program.entry.code[store].opcode { Opcode::StoreGlobal(slot) => *slot, _ => unreachable!() };
        let insertion = program.entry.code.len() - 1;
        program.entry.code.insert(insertion, crate::bytecode::Instruction { opcode: Opcode::PushInteger(2), location: None });
        program.entry.code.insert(insertion + 1, crate::bytecode::Instruction { opcode: Opcode::StoreGlobal(slot), location: None });
        assert!(execute(&program).unwrap_err().message.contains("immutable global"));
    }

    #[test]
    fn executes_mutable_function_locals() {
        assert_eq!(
            run("function count_from(start: integer) returns integer { create mutable current = start; while (current < 3) { set current = current + 1; } return current; } print(count_from(0));"),
            Ok(vec!["3".to_owned()])
        );
    }

    #[test]
    fn enforces_global_read_only_inside_functions() {
        let mut program = compile_to_bytecode("create mutable total = 1; function read() returns integer { return total; } print(read());").unwrap();
        let function = &mut program.functions[0];
        function.code.insert(0, crate::bytecode::Instruction { opcode: Opcode::PushInteger(2), location: None });
        function.code.insert(1, crate::bytecode::Instruction { opcode: Opcode::StoreGlobal(0), location: None });
        assert!(execute(&program).unwrap_err().message.contains("write global"));
    }

    #[test]
    fn rejects_division_by_zero_and_overflow() {
        assert!(run("print(1 / 0);").unwrap_err().contains("Division by zero"));
        assert!(run("print(9223372036854775807 + 1);").unwrap_err().contains("Integer overflow"));
    }

    #[test]
    fn short_circuit_operators_skip_the_right_operand() {
        assert_eq!(run("print(false and (1 / 0 == 0)); print(true or (1 / 0 == 0));"), Ok(vec!["false".into(), "true".into()]));
    }

    #[test]
    fn enforces_vm_step_budget() {
        let error = run("while (true) {}").unwrap_err();
        assert!(error.contains("step budget"));
    }

    #[test]
    fn supports_nested_and_recursive_calls() {
        assert_eq!(
            run("function factorial(n: integer) returns integer { if (n <= 1) { return 1; } else { return n * factorial(n - 1); } } print(factorial(5));"),
            Ok(vec!["120".into()])
        );
    }

    #[test]
    fn rejects_malformed_bytecode_before_execution() {
        let mut program = compile_to_bytecode("print(1);").unwrap();
        program.entry.code[0].opcode = Opcode::Jump(usize::MAX);
        assert!(execute(&program).unwrap_err().message.contains("Invalid bytecode structure"));
    }
}
