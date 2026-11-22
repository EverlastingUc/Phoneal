use crate::{
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    ir::{
        validate, BinaryOperator, TypedExpression, TypedExpressionKind, TypedFunction,
        TypedProgram, TypedStatement, TypedStatementKind, UnaryOperator,
    },
    types::PhonealType,
};
use std::collections::HashMap;

mod format;
mod validate;
mod vm;
mod verify;
pub use format::{deserialize, serialize};
pub use validate::validate_structure;
pub use verify::verify;
pub use vm::{execute as execute_bytecode, Value as BytecodeValue};

/// An instruction in Phoneal's compiler-facing stack bytecode.
#[derive(Debug, Clone, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub location: Option<SourceLocation>,
}

/// Stack-machine operations. Jump targets are absolute instruction offsets within a function.
#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    PushInteger(i64),
    PushText(String),
    PushBoolean(bool),
    LoadLocal(usize),
    StoreLocal(usize),
    LoadGlobal(usize),
    StoreGlobal(usize),
    Pop,
    Print,
    Negate,
    Not,
    AddInteger,
    AddText,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    Jump(usize),
    JumpIfFalse(usize),
    Call { function: usize, argument_count: usize },
    Return,
    Halt,
    /// Marks the end of a function whose validated control flow always returns.
    Unreachable,
}

/// Compiled function metadata and its instruction stream.
#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeFunction {
    pub name: String,
    pub parameter_count: usize,
    pub parameter_types: Vec<PhonealType>,
    pub local_count: usize,
    pub local_types: Vec<PhonealType>,
    pub local_mutability: Vec<bool>,
    pub return_type: Option<PhonealType>,
    pub code: Vec<Instruction>,
}

/// Bytecode output consumed by the prototype VM; it is not a security-verified executable image.
#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeProgram {
    pub functions: Vec<BytecodeFunction>,
    pub entry: BytecodeFunction,
    pub global_count: usize,
    pub global_types: Vec<PhonealType>,
    pub global_mutability: Vec<bool>,
}

#[derive(Debug, Clone, Copy)]
enum Storage {
    Local(usize),
    Global(usize),
}

struct FunctionCompiler<'a> {
    function_ids: &'a HashMap<String, usize>,
    globals: &'a HashMap<String, usize>,
    scopes: Vec<HashMap<String, Storage>>,
    code: Vec<Instruction>,
    local_count: usize,
    local_types: Vec<PhonealType>,
    local_mutability: Vec<bool>,
    loops: Vec<LoopPatch>,
}

#[derive(Default)]
struct LoopPatch {
    continue_target: usize,
    breaks: Vec<usize>,
}

impl<'a> FunctionCompiler<'a> {
    fn new(
        function_ids: &'a HashMap<String, usize>,
        globals: &'a HashMap<String, usize>,
        parameters: impl IntoIterator<Item = (String, usize, PhonealType)>,
    ) -> Self {
        let parameters: Vec<_> = parameters.into_iter().collect();
        let mut parameter_scope = HashMap::new();
        let mut local_types = vec![PhonealType::Integer; parameters.len()];
        for (name, slot, ty) in parameters {
            local_types[slot] = ty;
            parameter_scope.insert(name, Storage::Local(slot));
        }
        Self {
            function_ids,
            globals,
            scopes: vec![globals
                .iter()
                .map(|(name, slot)| (name.clone(), Storage::Global(*slot)))
                .collect(), parameter_scope],
            code: Vec::new(),
            local_count: local_types.len(),
            local_mutability: vec![false; local_types.len()],
            local_types,
            loops: Vec::new(),
        }
    }

    fn emit(&mut self, opcode: Opcode, location: Option<&SourceLocation>) -> usize {
        let index = self.code.len();
        self.code.push(Instruction { opcode, location: location.cloned() });
        index
    }

    fn patch_jump(&mut self, instruction: usize, target: usize) {
        match &mut self.code[instruction].opcode {
            Opcode::Jump(address) | Opcode::JumpIfFalse(address) => *address = target,
            _ => unreachable!("only jump instructions are patched"),
        }
    }

    fn declare(&mut self, name: &str, global: bool, ty: PhonealType, mutable: bool) -> Storage {
        if global {
            return Storage::Global(self.globals[name]);
        }
        let slot = self.local_count;
        self.local_count += 1;
        self.local_types.push(ty);
        self.local_mutability.push(mutable);
        self.scopes.last_mut().expect("scope exists").insert(name.to_owned(), Storage::Local(slot));
        Storage::Local(slot)
    }

    fn resolve(&self, name: &str) -> Option<Storage> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name).copied())
    }

    fn load(&mut self, storage: Storage, location: Option<&SourceLocation>) {
        self.emit(match storage {
            Storage::Local(slot) => Opcode::LoadLocal(slot),
            Storage::Global(slot) => Opcode::LoadGlobal(slot),
        }, location);
    }

    fn store(&mut self, storage: Storage, location: Option<&SourceLocation>) {
        self.emit(match storage {
            Storage::Local(slot) => Opcode::StoreLocal(slot),
            Storage::Global(slot) => Opcode::StoreGlobal(slot),
        }, location);
    }

    fn compile_statements(&mut self, statements: &[TypedStatement], top_level: bool) -> Result<(), Diagnostic> {
        for statement in statements {
            self.compile_statement(statement, top_level)?;
        }
        Ok(())
    }

    fn compile_statement(&mut self, statement: &TypedStatement, top_level: bool) -> Result<(), Diagnostic> {
        let location = statement.location.as_ref();
        match &statement.kind {
            TypedStatementKind::Let { name, mutable, ty, value } => {
                self.compile_expression(value)?;
                let storage = self.declare(name, top_level && self.scopes.len() == 2, *ty, *mutable);
                self.store(storage, location);
            }
            TypedStatementKind::Assign { name, value } => {
                let storage = self.resolve(name).ok_or_else(|| self.error("Bytecode compiler could not resolve assignment target", location))?;
                self.compile_expression(value)?;
                self.store(storage, location);
            }
            TypedStatementKind::Print(value) => {
                self.compile_expression(value)?;
                self.emit(Opcode::Print, location);
            }
            TypedStatementKind::Return(value) => {
                self.compile_expression(value)?;
                self.emit(Opcode::Return, location);
            }
            TypedStatementKind::Block(body) => {
                self.scopes.push(HashMap::new());
                self.compile_statements(body, false)?;
                self.scopes.pop();
            }
            TypedStatementKind::If { condition, then_branch, else_branch } => {
                self.compile_expression(condition)?;
                let false_jump = self.emit(Opcode::JumpIfFalse(usize::MAX), location);
                self.scopes.push(HashMap::new());
                self.compile_statements(then_branch, false)?;
                self.scopes.pop();
                if let Some(other) = else_branch {
                    let end_jump = self.emit(Opcode::Jump(usize::MAX), location);
                    self.patch_jump(false_jump, self.code.len());
                    self.scopes.push(HashMap::new());
                    self.compile_statements(other, false)?;
                    self.scopes.pop();
                    self.patch_jump(end_jump, self.code.len());
                } else {
                    self.patch_jump(false_jump, self.code.len());
                }
            }
            TypedStatementKind::While { condition, body } => {
                let condition_start = self.code.len();
                self.compile_expression(condition)?;
                let exit_jump = self.emit(Opcode::JumpIfFalse(usize::MAX), location);
                self.loops.push(LoopPatch { continue_target: condition_start, breaks: Vec::new() });
                self.scopes.push(HashMap::new());
                self.compile_statements(body, false)?;
                self.scopes.pop();
                self.emit(Opcode::Jump(condition_start), location);
                let exit = self.code.len();
                self.patch_jump(exit_jump, exit);
                let loop_patch = self.loops.pop().expect("loop patch exists");
                for jump in loop_patch.breaks {
                    self.patch_jump(jump, exit);
                }
            }
            TypedStatementKind::Break => {
                if self.loops.is_empty() {
                    return Err(self.error("Break outside loop in bytecode compiler", location));
                }
                let jump = self.emit(Opcode::Jump(usize::MAX), location);
                self.loops.last_mut().expect("loop was checked").breaks.push(jump);
            }
            TypedStatementKind::Continue => {
                let target = self.loops.last().ok_or_else(|| self.error("Continue outside loop in bytecode compiler", location))?.continue_target;
                self.emit(Opcode::Jump(target), location);
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expression: &TypedExpression) -> Result<(), Diagnostic> {
        let location = expression.location.as_ref();
        match &expression.kind {
            TypedExpressionKind::Integer(value) => { self.emit(Opcode::PushInteger(*value), location); }
            TypedExpressionKind::Text(value) => { self.emit(Opcode::PushText(value.clone()), location); }
            TypedExpressionKind::Boolean(value) => { self.emit(Opcode::PushBoolean(*value), location); }
            TypedExpressionKind::Variable(name) => {
                let storage = self.resolve(name).ok_or_else(|| self.error("Bytecode compiler could not resolve variable", location))?;
                self.load(storage, location);
            }
            TypedExpressionKind::Call { name, arguments } => {
                for argument in arguments { self.compile_expression(argument)?; }
                let function = *self.function_ids.get(name).ok_or_else(|| self.error("Bytecode compiler could not resolve function", location))?;
                self.emit(Opcode::Call { function, argument_count: arguments.len() }, location);
            }
            TypedExpressionKind::Unary { operator, operand } => {
                self.compile_expression(operand)?;
                self.emit(match operator { UnaryOperator::Negate => Opcode::Negate, UnaryOperator::Not => Opcode::Not }, location);
            }
            TypedExpressionKind::Binary { left, operator, right } => {
                self.compile_expression(left)?;
                if *operator == BinaryOperator::And {
                    let false_jump = self.emit(Opcode::JumpIfFalse(usize::MAX), location);
                    self.compile_expression(right)?;
                    let end_jump = self.emit(Opcode::Jump(usize::MAX), location);
                    self.patch_jump(false_jump, self.code.len());
                    self.emit(Opcode::PushBoolean(false), location);
                    self.patch_jump(end_jump, self.code.len());
                    return Ok(());
                }
                if *operator == BinaryOperator::Or {
                    let evaluate_right = self.emit(Opcode::JumpIfFalse(usize::MAX), location);
                    self.emit(Opcode::PushBoolean(true), location);
                    let end_jump = self.emit(Opcode::Jump(usize::MAX), location);
                    self.patch_jump(evaluate_right, self.code.len());
                    self.compile_expression(right)?;
                    self.patch_jump(end_jump, self.code.len());
                    return Ok(());
                }
                self.compile_expression(right)?;
                let opcode = match operator {
                    BinaryOperator::Add if expression.ty == PhonealType::Text => Opcode::AddText,
                    BinaryOperator::Add => Opcode::AddInteger,
                    BinaryOperator::Subtract => Opcode::Subtract,
                    BinaryOperator::Multiply => Opcode::Multiply,
                    BinaryOperator::Divide => Opcode::Divide,
                    BinaryOperator::Remainder => Opcode::Remainder,
                    BinaryOperator::Less => Opcode::Less,
                    BinaryOperator::LessEqual => Opcode::LessEqual,
                    BinaryOperator::Greater => Opcode::Greater,
                    BinaryOperator::GreaterEqual => Opcode::GreaterEqual,
                    BinaryOperator::Equal => Opcode::Equal,
                    BinaryOperator::NotEqual => Opcode::NotEqual,
                    BinaryOperator::And | BinaryOperator::Or => unreachable!(),
                };
                self.emit(opcode, location);
            }
        }
        Ok(())
    }

    fn error(&self, message: &str, location: Option<&SourceLocation>) -> Diagnostic {
        match location {
            Some(location) => Diagnostic::at(DiagnosticStage::Bytecode, message, location.line, location.column),
            None => Diagnostic::new(DiagnosticStage::Bytecode, message),
        }
    }
}

/// Compiles a validated typed IR program into stack bytecode.
///
/// The result is an intermediate compiler artifact only: it is not verified or executable
/// until the Phoneal VM and bytecode verifier are implemented.
pub fn compile(program: &TypedProgram) -> Result<BytecodeProgram, Diagnostic> {
    validate(program).map_err(|error| Diagnostic::new(DiagnosticStage::Bytecode, format!("Invalid typed IR: {}", error.message)))?;

    let function_ids: HashMap<_, _> = program.functions.iter().enumerate()
        .map(|(index, function)| (function.name.clone(), index)).collect();
    let global_bindings: Vec<_> = program.statements.iter().filter_map(|statement| {
        if let TypedStatementKind::Let { name, ty, mutable, .. } = &statement.kind {
            Some((name.clone(), *ty, *mutable))
        } else { None }
    }).collect();
    let globals: HashMap<_, _> = global_bindings.iter().enumerate()
        .map(|(index, (name, _, _))| (name.clone(), index)).collect();
    let global_types = global_bindings.iter().map(|(_, ty, _)| *ty).collect::<Vec<_>>();
    let global_mutability = global_bindings.iter().map(|(_, _, mutable)| *mutable).collect::<Vec<_>>();

    let mut functions = Vec::with_capacity(program.functions.len());
    for function in &program.functions {
        functions.push(compile_function(function, &function_ids, &globals)?);
    }

    let mut entry = FunctionCompiler::new(&function_ids, &globals, std::iter::empty());
    entry.compile_statements(&program.statements, true)?;
    entry.emit(Opcode::Halt, None);
    let bytecode = BytecodeProgram {
        functions,
        entry: BytecodeFunction {
            name: "<entry>".to_owned(),
            parameter_count: 0,
            local_count: entry.local_count,
            local_types: entry.local_types,
            local_mutability: entry.local_mutability,
            parameter_types: Vec::new(),
            return_type: None,
            code: entry.code,
        },
        global_count: globals.len(),
        global_types,
        global_mutability,
    };
    verify(&bytecode)?;
    Ok(bytecode)
}

fn compile_function(
    function: &TypedFunction,
    function_ids: &HashMap<String, usize>,
    globals: &HashMap<String, usize>,
) -> Result<BytecodeFunction, Diagnostic> {
    let parameters = function.parameters.iter().enumerate()
        .map(|(slot, parameter)| (parameter.name.clone(), slot, parameter.ty)).collect::<Vec<_>>();
    let parameter_types = function.parameters.iter().map(|parameter| parameter.ty).collect::<Vec<_>>();
    let mut compiler = FunctionCompiler::new(function_ids, globals, parameters);
    compiler.compile_statements(&function.body, false)?;
    compiler.emit(Opcode::Unreachable, Some(&function.location));
    Ok(BytecodeFunction {
        name: function.name.clone(),
        parameter_count: function.parameters.len(),
        parameter_types,
        local_count: compiler.local_count,
        local_types: compiler.local_types,
        local_mutability: compiler.local_mutability,
        return_type: Some(function.return_type),
        code: compiler.code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_ir;

    fn compile_source(source: &str) -> BytecodeProgram {
        compile(&compile_to_ir(source).unwrap()).unwrap()
    }

    #[test]
    fn emits_typed_arithmetic_and_output() {
        let bytecode = compile_source("create answer = 6 * 7; print(answer);");
        assert_eq!(bytecode.global_count, 1);
        assert!(bytecode.entry.code.iter().any(|instruction| instruction.opcode == Opcode::Multiply));
        assert!(bytecode.entry.code.iter().any(|instruction| instruction.opcode == Opcode::Print));
    }

    #[test]
    fn emits_function_calls_and_returns() {
        let bytecode = compile_source("function add(a: integer, b: integer) returns integer { return a + b; } print(add(20, 22));");
        assert_eq!(bytecode.functions.len(), 1);
        assert_eq!(bytecode.functions[0].parameter_count, 2);
        assert!(bytecode.functions[0].code.iter().any(|instruction| matches!(&instruction.opcode, Opcode::Return)));
        assert!(bytecode.entry.code.iter().any(|instruction| matches!(&instruction.opcode, Opcode::Call { function: 0, argument_count: 2 })));
    }

    #[test]
    fn patches_branch_and_loop_jumps() {
        let bytecode = compile_source("create mutable n = 0; if (true) { print(1); } else { print(2); } while (n < 2) { set n = n + 1; if (n == 1) { continue; } break; }");
        for function in bytecode.functions.iter().chain(std::iter::once(&bytecode.entry)) {
            for instruction in &function.code {
                match &instruction.opcode {
                    Opcode::Jump(target) | Opcode::JumpIfFalse(target) => assert!(*target <= function.code.len()),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn preserves_short_circuit_control_flow() {
        let bytecode = compile_source("print(false and (1 / 0 == 0)); print(true or (1 / 0 == 0));");
        assert!(bytecode.entry.code.iter().any(|instruction| matches!(&instruction.opcode, Opcode::JumpIfFalse(_))));
        assert!(bytecode.entry.code.iter().any(|instruction| matches!(&instruction.opcode, Opcode::PushBoolean(false))));
        assert!(bytecode.entry.code.iter().any(|instruction| matches!(&instruction.opcode, Opcode::PushBoolean(true))));
    }

    #[test]
    fn resolves_global_reads_inside_functions() {
        let bytecode = compile_source("create answer = 42; function read() returns integer { return answer; } print(read());");
        assert!(bytecode.functions[0].code.iter().any(|instruction| instruction.opcode == Opcode::LoadGlobal(0)));
    }
}
