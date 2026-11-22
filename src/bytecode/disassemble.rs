use super::{verify, BytecodeFunction, BytecodeProgram, Opcode};
use crate::diagnostic::{Diagnostic, DiagnosticStage};
use std::fmt::Write;

/// Produces a stable, human-readable listing of a verified bytecode program.
pub fn disassemble(program: &BytecodeProgram) -> Result<String, Diagnostic> {
    verify(program)?;
    let mut output = String::new();
    writeln!(&mut output, "Phoneal bytecode (PHNB v1)").expect("writing to String cannot fail");
    writeln!(&mut output, "Globals: {}", program.global_count).expect("writing to String cannot fail");
    for (index, function) in program.functions.iter().enumerate() {
        write_function(&mut output, index, function, program)?;
    }
    writeln!(&mut output).expect("writing to String cannot fail");
    write!(&mut output, "Entry ").expect("writing to String cannot fail");
    write_function_body(&mut output, &program.entry, program)?;
    Ok(output)
}

fn write_function(
    output: &mut String,
    index: usize,
    function: &BytecodeFunction,
    program: &BytecodeProgram,
) -> Result<(), Diagnostic> {
    writeln!(output).expect("writing to String cannot fail");
    writeln!(output, "Function #{index}").expect("writing to String cannot fail");
    write_function_body(output, function, program)
}

fn write_function_body(
    output: &mut String,
    function: &BytecodeFunction,
    program: &BytecodeProgram,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "{}({}) -> {} [locals: {}]",
        function.name,
        function.parameter_types.iter().map(ToString::to_string).collect::<Vec<_>>().join(", "),
        function.return_type.map_or_else(|| "entry".to_owned(), |ty| ty.to_string()),
        function.local_count
    ).expect("writing to String cannot fail");

    for (offset, instruction) in function.code.iter().enumerate() {
        write!(output, "  {offset:04}  ").expect("writing to String cannot fail");
        write_opcode(output, &instruction.opcode, program)?;
        if let Some(location) = &instruction.location {
            write!(output, "    ; {}:{}", location.line, location.column)
                .expect("writing to String cannot fail");
        }
        writeln!(output).expect("writing to String cannot fail");
    }
    Ok(())
}

fn write_opcode(
    output: &mut String,
    opcode: &Opcode,
    program: &BytecodeProgram,
) -> Result<(), Diagnostic> {
    let result = match opcode {
        Opcode::PushInteger(value) => write!(output, "PushInteger {value}"),
        Opcode::PushText(value) => write!(output, "PushText {:?}", value),
        Opcode::PushBoolean(value) => write!(output, "PushBoolean {value}"),
        Opcode::LoadLocal(slot) => write!(output, "LoadLocal {slot}"),
        Opcode::StoreLocal(slot) => write!(output, "StoreLocal {slot}"),
        Opcode::LoadGlobal(slot) => write!(output, "LoadGlobal {slot}"),
        Opcode::StoreGlobal(slot) => write!(output, "StoreGlobal {slot}"),
        Opcode::ResetLocals { start, count } => write!(output, "ResetLocals {start}..{}", *start + *count),
        Opcode::Pop => write!(output, "Pop"),
        Opcode::Print => write!(output, "Print"),
        Opcode::Negate => write!(output, "Negate"),
        Opcode::Not => write!(output, "Not"),
        Opcode::AddInteger => write!(output, "AddInteger"),
        Opcode::AddText => write!(output, "AddText"),
        Opcode::Subtract => write!(output, "Subtract"),
        Opcode::Multiply => write!(output, "Multiply"),
        Opcode::Divide => write!(output, "Divide"),
        Opcode::Remainder => write!(output, "Remainder"),
        Opcode::Less => write!(output, "Less"),
        Opcode::LessEqual => write!(output, "LessEqual"),
        Opcode::Greater => write!(output, "Greater"),
        Opcode::GreaterEqual => write!(output, "GreaterEqual"),
        Opcode::Equal => write!(output, "Equal"),
        Opcode::NotEqual => write!(output, "NotEqual"),
        Opcode::Jump(target) => write!(output, "Jump {target:04}"),
        Opcode::JumpIfFalse(target) => write!(output, "JumpIfFalse {target:04}"),
        Opcode::Call { function, argument_count } => {
            let target_name = program.functions[*function].name.as_str();
            write!(output, "Call #{function} {target_name} ({argument_count} args)")
        }
        Opcode::Return => write!(output, "Return"),
        Opcode::Halt => write!(output, "Halt"),
        Opcode::Unreachable => write!(output, "Unreachable"),
    };
    result.map_err(|_| Diagnostic::new(DiagnosticStage::Bytecode, "Could not format bytecode instruction"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_bytecode;

    #[test]
    fn disassembles_functions_calls_and_source_locations() {
        let program = compile_to_bytecode(
            "function double(value: integer) returns integer { return value * 2; } print(double(21));",
        ).unwrap();
        let listing = disassemble(&program).unwrap();
        assert!(listing.contains("Function #0"));
        assert!(listing.contains("double(integer) -> integer"));
        assert!(listing.contains("Call #0 double (1 args)"));
        assert!(listing.contains("; 1:"));
    }

    #[test]
    fn disassembles_jump_targets() {
        let program = compile_to_bytecode("if (true) { print(1); } else { print(2); }").unwrap();
        let listing = disassemble(&program).unwrap();
        assert!(listing.contains("JumpIfFalse"));
        assert!(listing.contains("Jump "));
    }

    #[test]
    fn refuses_to_disassemble_invalid_bytecode() {
        let mut program = compile_to_bytecode("print(1);").unwrap();
        program.entry.code[0].opcode = Opcode::Jump(usize::MAX);
        assert!(disassemble(&program).unwrap_err().message.contains("Jump target"));
    }
}
