use super::{verify, BytecodeFunction, BytecodeProgram, Instruction, Opcode};
use crate::{
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    types::PhonealType,
};

const MAGIC: &[u8; 4] = b"PHNB";
const VERSION: u16 = 2;
const HEADER_LENGTH: usize = 14;
const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_FUNCTIONS: usize = 4096;
const MAX_SLOTS: usize = 100_000;
const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 100_000;

fn format_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticStage::Bytecode, message)
}

fn write_u8(output: &mut Vec<u8>, value: u8) {
    output.push(value);
}

fn write_u32(output: &mut Vec<u8>, value: usize) -> Result<(), Diagnostic> {
    let value = u32::try_from(value)
        .map_err(|_| format_error("Bytecode field exceeds the file-format limit"))?;
    output.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_i64(output: &mut Vec<u8>, value: i64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn write_string(output: &mut Vec<u8>, value: &str) -> Result<(), Diagnostic> {
    if value.len() > MAX_STRING_BYTES {
        return Err(format_error("Bytecode string exceeds the file-format limit"));
    }
    write_u32(output, value.len())?;
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn type_tag(ty: PhonealType) -> u8 {
    match ty {
        PhonealType::Integer => 1,
        PhonealType::Text => 2,
        PhonealType::Boolean => 3,
    }
}

fn write_function(output: &mut Vec<u8>, function: &BytecodeFunction) -> Result<(), Diagnostic> {
    if function.code.len() > MAX_INSTRUCTIONS_PER_FUNCTION
        || function.local_count > MAX_SLOTS
        || function.parameter_count > MAX_SLOTS
    {
        return Err(format_error("Bytecode function exceeds the file-format limits"));
    }
    write_string(output, &function.name)?;
    write_u32(output, function.parameter_count)?;
    for ty in &function.parameter_types {
        write_u8(output, type_tag(*ty));
    }
    write_u32(output, function.local_count)?;
    for slot in 0..function.local_count {
        write_u8(output, type_tag(function.local_types[slot]));
        write_u8(output, u8::from(function.local_mutability[slot]));
        write_u32(output, function.local_declarations[slot].unwrap_or(u32::MAX as usize))?;
    }
    write_u8(output, function.return_type.map(type_tag).unwrap_or(0));
    write_u32(output, function.code.len())?;
    for instruction in &function.code {
        write_opcode(output, &instruction.opcode)?;
        match &instruction.location {
            Some(location) => {
                write_u8(output, 1);
                write_u32(output, location.line)?;
                write_u32(output, location.column)?;
            }
            None => write_u8(output, 0),
        }
    }
    Ok(())
}

fn write_opcode(output: &mut Vec<u8>, opcode: &Opcode) -> Result<(), Diagnostic> {
    match opcode {
        Opcode::PushInteger(value) => {
            write_u8(output, 0);
            write_i64(output, *value);
        }
        Opcode::PushText(value) => {
            write_u8(output, 1);
            write_string(output, value)?;
        }
        Opcode::PushBoolean(value) => {
            write_u8(output, 2);
            write_u8(output, u8::from(*value));
        }
        Opcode::LoadLocal(slot) => {
            write_u8(output, 3);
            write_u32(output, *slot)?;
        }
        Opcode::StoreLocal(slot) => {
            write_u8(output, 4);
            write_u32(output, *slot)?;
        }
        Opcode::LoadGlobal(slot) => {
            write_u8(output, 5);
            write_u32(output, *slot)?;
        }
        Opcode::StoreGlobal(slot) => {
            write_u8(output, 6);
            write_u32(output, *slot)?;
        }
        Opcode::ResetLocals { start, count } => {
            write_u8(output, 29);
            write_u32(output, *start)?;
            write_u32(output, *count)?;
        }
        Opcode::Pop => write_u8(output, 7),
        Opcode::Print => write_u8(output, 8),
        Opcode::Negate => write_u8(output, 9),
        Opcode::Not => write_u8(output, 10),
        Opcode::AddInteger => write_u8(output, 11),
        Opcode::AddText => write_u8(output, 12),
        Opcode::Subtract => write_u8(output, 13),
        Opcode::Multiply => write_u8(output, 14),
        Opcode::Divide => write_u8(output, 15),
        Opcode::Remainder => write_u8(output, 16),
        Opcode::Less => write_u8(output, 17),
        Opcode::LessEqual => write_u8(output, 18),
        Opcode::Greater => write_u8(output, 19),
        Opcode::GreaterEqual => write_u8(output, 20),
        Opcode::Equal => write_u8(output, 21),
        Opcode::NotEqual => write_u8(output, 22),
        Opcode::Jump(target) => {
            write_u8(output, 23);
            write_u32(output, *target)?;
        }
        Opcode::JumpIfFalse(target) => {
            write_u8(output, 24);
            write_u32(output, *target)?;
        }
        Opcode::Call { function, argument_count } => {
            write_u8(output, 25);
            write_u32(output, *function)?;
            write_u32(output, *argument_count)?;
        }
        Opcode::Return => write_u8(output, 26),
        Opcode::Halt => write_u8(output, 27),
        Opcode::Unreachable => write_u8(output, 28),
    }
    Ok(())
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Serializes a verified bytecode program into the versioned Phoneal bytecode container.
pub fn serialize(program: &BytecodeProgram) -> Result<Vec<u8>, Diagnostic> {
    verify(program)?;
    if program.global_count > MAX_SLOTS || program.functions.len() > MAX_FUNCTIONS {
        return Err(format_error("Bytecode program exceeds the file-format limits"));
    }

    let mut payload = Vec::new();
    write_u32(&mut payload, program.global_count)?;
    for (ty, mutable) in program.global_types.iter().zip(&program.global_mutability) {
        write_u8(&mut payload, type_tag(*ty));
        write_u8(&mut payload, u8::from(*mutable));
    }
    write_u32(&mut payload, program.functions.len())?;
    for function in &program.functions {
        write_function(&mut payload, function)?;
    }
    write_function(&mut payload, &program.entry)?;

    if payload.len() > MAX_FILE_BYTES - HEADER_LENGTH {
        return Err(format_error("Serialized bytecode exceeds the file-size limit"));
    }

    let mut output = Vec::with_capacity(HEADER_LENGTH + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&VERSION.to_le_bytes());
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(&crc32(&payload).to_le_bytes());
    output.extend_from_slice(&payload);
    Ok(output)
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Diagnostic> {
        let end = self.offset.checked_add(count)
            .ok_or_else(|| format_error("Bytecode offset overflow"))?;
        let slice = self.bytes.get(self.offset..end)
            .ok_or_else(|| format_error("Truncated bytecode file"))?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, Diagnostic> {
        Ok(self.take(1)?[0])
    }

    fn read_bool(&mut self) -> Result<bool, Diagnostic> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(format_error("Invalid boolean flag in bytecode file")),
        }
    }

    fn read_u32(&mut self) -> Result<usize, Diagnostic> {
        let bytes: [u8; 4] = self.take(4)?.try_into().expect("four bytes were requested");
        Ok(u32::from_le_bytes(bytes) as usize)
    }

    fn read_i64(&mut self) -> Result<i64, Diagnostic> {
        let bytes: [u8; 8] = self.take(8)?.try_into().expect("eight bytes were requested");
        Ok(i64::from_le_bytes(bytes))
    }

    fn read_string(&mut self) -> Result<String, Diagnostic> {
        let length = self.read_u32()?;
        if length > MAX_STRING_BYTES {
            return Err(format_error("Bytecode string exceeds the file-format limit"));
        }
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|_| format_error("Bytecode contains invalid UTF-8 text"))
    }

    fn read_type(&mut self) -> Result<PhonealType, Diagnostic> {
        match self.read_u8()? {
            1 => Ok(PhonealType::Integer),
            2 => Ok(PhonealType::Text),
            3 => Ok(PhonealType::Boolean),
            _ => Err(format_error("Unknown primitive type tag in bytecode file")),
        }
    }

    fn read_optional_type(&mut self) -> Result<Option<PhonealType>, Diagnostic> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => Ok(Some(PhonealType::Integer)),
            2 => Ok(Some(PhonealType::Text)),
            3 => Ok(Some(PhonealType::Boolean)),
            _ => Err(format_error("Unknown optional type tag in bytecode file")),
        }
    }

    fn read_opcode(&mut self) -> Result<Opcode, Diagnostic> {
        Ok(match self.read_u8()? {
            0 => Opcode::PushInteger(self.read_i64()?),
            1 => Opcode::PushText(self.read_string()?),
            2 => Opcode::PushBoolean(self.read_bool()?),
            3 => Opcode::LoadLocal(self.read_u32()?),
            4 => Opcode::StoreLocal(self.read_u32()?),
            5 => Opcode::LoadGlobal(self.read_u32()?),
            6 => Opcode::StoreGlobal(self.read_u32()?),
            7 => Opcode::Pop,
            8 => Opcode::Print,
            9 => Opcode::Negate,
            10 => Opcode::Not,
            11 => Opcode::AddInteger,
            12 => Opcode::AddText,
            13 => Opcode::Subtract,
            14 => Opcode::Multiply,
            15 => Opcode::Divide,
            16 => Opcode::Remainder,
            17 => Opcode::Less,
            18 => Opcode::LessEqual,
            19 => Opcode::Greater,
            20 => Opcode::GreaterEqual,
            21 => Opcode::Equal,
            22 => Opcode::NotEqual,
            23 => Opcode::Jump(self.read_u32()?),
            24 => Opcode::JumpIfFalse(self.read_u32()?),
            25 => Opcode::Call {
                function: self.read_u32()?,
                argument_count: self.read_u32()?,
            },
            26 => Opcode::Return,
            27 => Opcode::Halt,
            28 => Opcode::Unreachable,
            _ => return Err(format_error("Unknown opcode tag in bytecode file")),
        })
    }

    fn read_function(&mut self) -> Result<BytecodeFunction, Diagnostic> {
        let name = self.read_string()?;
        let parameter_count = self.read_u32()?;
        if parameter_count > MAX_SLOTS {
            return Err(format_error("Bytecode parameter count exceeds the file-format limit"));
        }
        let mut parameter_types = Vec::with_capacity(parameter_count);
        for _ in 0..parameter_count {
            parameter_types.push(self.read_type()?);
        }
        let local_count = self.read_u32()?;
        if local_count > MAX_SLOTS || parameter_count > local_count {
            return Err(format_error("Bytecode local count exceeds the file-format limit"));
        }
        let mut local_types = Vec::with_capacity(local_count);
        let mut local_mutability = Vec::with_capacity(local_count);
        let mut local_declarations = Vec::with_capacity(local_count);
        for _ in 0..local_count {
            local_types.push(self.read_type()?);
            local_mutability.push(self.read_bool()?);
            let declaration = self.read_u32()?;
            local_declarations.push((declaration != u32::MAX as usize).then_some(declaration));
        }
        let return_type = self.read_optional_type()?;
        let instruction_count = self.read_u32()?;
        if instruction_count == 0 || instruction_count > MAX_INSTRUCTIONS_PER_FUNCTION {
            return Err(format_error("Bytecode instruction count exceeds the file-format limit"));
        }
        let mut code = Vec::with_capacity(instruction_count);
        for _ in 0..instruction_count {
            let opcode = self.read_opcode()?;
            let location = match self.read_u8()? {
                0 => None,
                1 => Some(SourceLocation {
                    line: self.read_u32()?,
                    column: self.read_u32()?,
                }),
                _ => return Err(format_error("Invalid source-location flag in bytecode file")),
            };
            code.push(Instruction { opcode, location });
        }
        Ok(BytecodeFunction {
            name,
            parameter_count,
            parameter_types,
            local_count,
            local_types,
            local_mutability,
            local_declarations,
            return_type,
            code,
        })
    }
}

/// Deserializes a bytecode file, checks its checksum and format bounds, then verifies the program.
pub fn deserialize(bytes: &[u8]) -> Result<BytecodeProgram, Diagnostic> {
    if bytes.len() < HEADER_LENGTH || bytes.len() > MAX_FILE_BYTES {
        return Err(format_error("Bytecode file size is invalid"));
    }
    if bytes.get(..4) != Some(MAGIC.as_slice()) {
        return Err(format_error("Invalid Phoneal bytecode magic header"));
    }
    let version = u16::from_le_bytes(bytes[4..6].try_into().expect("header has a version"));
    if version != VERSION {
        return Err(format_error(format!("Unsupported Phoneal bytecode version {version}")));
    }
    let payload_length = u32::from_le_bytes(bytes[6..10].try_into().expect("header has a payload length")) as usize;
    let expected_crc = u32::from_le_bytes(bytes[10..14].try_into().expect("header has a checksum"));
    if payload_length != bytes.len() - HEADER_LENGTH {
        return Err(format_error("Bytecode payload length does not match the file size"));
    }
    let payload = &bytes[HEADER_LENGTH..];
    if crc32(payload) != expected_crc {
        return Err(format_error("Bytecode checksum mismatch"));
    }

    let mut reader = Reader { bytes: payload, offset: 0 };
    let global_count = reader.read_u32()?;
    if global_count > MAX_SLOTS {
        return Err(format_error("Bytecode global count exceeds the file-format limit"));
    }
    let mut global_types = Vec::with_capacity(global_count);
    let mut global_mutability = Vec::with_capacity(global_count);
    for _ in 0..global_count {
        global_types.push(reader.read_type()?);
        global_mutability.push(reader.read_bool()?);
    }
    let function_count = reader.read_u32()?;
    if function_count > MAX_FUNCTIONS {
        return Err(format_error("Bytecode function count exceeds the file-format limit"));
    }
    let mut functions = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        functions.push(reader.read_function()?);
    }
    let entry = reader.read_function()?;
    if reader.offset != payload.len() {
        return Err(format_error("Bytecode file contains trailing payload data"));
    }

    let program = BytecodeProgram {
        functions,
        entry,
        global_count,
        global_types,
        global_mutability,
    };
    verify(&program)?;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_to_bytecode;

    #[test]
    fn serializes_and_deserializes_bytecode_losslessly() {
        let program = compile_to_bytecode(
            "function double(value: integer) returns integer { return value * 2; } create mutable count = 0; while (count < 3) { create snapshot = count; set count = double(count + 1); print(snapshot); } print(count);",
        ).unwrap();
        let bytes = serialize(&program).unwrap();
        assert_eq!(deserialize(&bytes).unwrap(), program);
    }

    #[test]
    fn decoder_handles_truncated_and_arbitrary_bounded_inputs_without_panicking() {
        let valid = serialize(&compile_to_bytecode("print(42);").unwrap()).unwrap();
        for end in 0..valid.len() {
            assert!(deserialize(&valid[..end]).is_err());
        }

        let mut seed = 0xA5A5_1234_u32;
        for length in 0..256 {
            let mut bytes = Vec::with_capacity(length);
            for _ in 0..length {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                bytes.push(seed as u8);
            }
            let _ = deserialize(&bytes);
        }
    }

    #[test]
    fn rejects_bad_magic_version_and_checksum() {
        let program = compile_to_bytecode("print(42);").unwrap();
        let bytes = serialize(&program).unwrap();

        let mut bad_magic = bytes.clone();
        bad_magic[0] = b'X';
        assert!(deserialize(&bad_magic).unwrap_err().message.contains("magic"));

        let mut bad_version = bytes.clone();
        bad_version[4] = 99;
        assert!(deserialize(&bad_version).unwrap_err().message.contains("version"));

        let mut bad_checksum = bytes;
        *bad_checksum.last_mut().unwrap() ^= 0xFF;
        assert!(deserialize(&bad_checksum).unwrap_err().message.contains("checksum"));
    }

    #[test]
    fn rejects_truncated_and_trailing_data() {
        let bytes = serialize(&compile_to_bytecode("print(1);").unwrap()).unwrap();
        assert!(deserialize(&bytes[..bytes.len() - 1]).unwrap_err().message.contains("payload length"));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(deserialize(&trailing).unwrap_err().message.contains("payload length"));
    }

    #[test]
    fn refuses_to_serialize_invalid_bytecode() {
        let mut program = compile_to_bytecode("print(1);").unwrap();
        program.entry.code[0].opcode = Opcode::Jump(usize::MAX);
        assert!(serialize(&program).is_err());
    }

    #[test]
    fn rejects_unknown_opcodes_with_a_valid_checksum() {
        let program = compile_to_bytecode("print(1);").unwrap();
        let mut bytes = serialize(&program).unwrap();
        let payload = &mut bytes[HEADER_LENGTH..];
        let mut offset = 0;
        offset += 4; // global count
        offset += 4; // function count
        let name_length = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4 + name_length;
        offset += 4; // parameter count
        offset += 4; // local count
        offset += 1; // return type
        offset += 4; // instruction count
        payload[offset] = 255;
        let checksum = crc32(payload);
        bytes[10..14].copy_from_slice(&checksum.to_le_bytes());
        assert!(deserialize(&bytes).unwrap_err().message.contains("Unknown opcode"));
    }
}
