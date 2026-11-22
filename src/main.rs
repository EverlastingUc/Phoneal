use std::{env, fs, process};

fn print_usage() {
    eprintln!("Phoneal 0.1.0 — Rust-hosted language prototype");
    eprintln!("Usage:");
    eprintln!("  phnl run <file.phnl>");
    eprintln!("  phnl run-bytecode <file.phnl>");
    eprintln!("  phnl check <file.phnl>");
    eprintln!("  phnl build <file.phnl> --output <file.phnb>");
    eprintln!("  phnl inspect <file.phnb>");
    eprintln!("  phnl execute <file.phnb>");
}

fn read_source(path: &str) -> Result<String, String> {
    if !path.ends_with(".phnl") {
        return Err("Expected a .phnl source file".to_owned());
    }
    fs::read_to_string(path).map_err(|error| format!("Cannot read {path}: {error}"))
}

fn read_bytecode(path: &str) -> Result<Vec<u8>, String> {
    if !path.ends_with(".phnb") {
        return Err("Expected a .phnb bytecode file".to_owned());
    }
    fs::read(path).map_err(|error| format!("Cannot read {path}: {error}"))
}

fn run_source(path: &str, use_bytecode: bool) -> Result<(), String> {
    let source = read_source(path)?;
    let result = if use_bytecode {
        phoneal::run_bytecode(&source)
    } else {
        phoneal::run(&source)
    };
    match result {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            Ok(())
        }
        Err(error) => Err(format!("Phoneal: {}", error.render(&source))),
    }
}

fn check_source(path: &str) -> Result<(), String> {
    let source = read_source(path)?;
    match phoneal::compile_with_diagnostics(&source) {
        Ok(_) => {
            println!("Syntax and semantic checks passed: {path}");
            Ok(())
        }
        Err(diagnostics) => {
            for diagnostic in diagnostics {
                eprintln!("Phoneal: {}", diagnostic.render(&source));
            }
            Err("Source validation failed".to_owned())
        }
    }
}

fn build_bytecode(source_path: &str, output_path: &str) -> Result<(), String> {
    let source = read_source(source_path)?;
    if !output_path.ends_with(".phnb") {
        return Err("Build output must use the .phnb extension".to_owned());
    }
    let program = phoneal::compile_to_bytecode(&source)
        .map_err(|error| format!("Phoneal: {}", error.render(&source)))?;
    let bytes = phoneal::bytecode::serialize(&program)
        .map_err(|error| format!("Phoneal: {}", error))?;
    fs::write(output_path, &bytes)
        .map_err(|error| format!("Cannot write {output_path}: {error}"))?;
    println!("Built {output_path} ({} bytes)", bytes.len());
    Ok(())
}

fn execute_bytecode_file(path: &str) -> Result<(), String> {
    let bytes = read_bytecode(path)?;
    let program = phoneal::bytecode::deserialize(&bytes)
        .map_err(|error| format!("Phoneal: {error}"))?;
    let lines = phoneal::bytecode::execute_bytecode(&program)
        .map_err(|error| format!("Phoneal: {error}"))?;
    for line in lines {
        println!("{line}");
    }
    Ok(())
}

fn inspect_bytecode(path: &str) -> Result<(), String> {
    let bytes = read_bytecode(path)?;
    let program = phoneal::bytecode::deserialize(&bytes)
        .map_err(|error| format!("Phoneal: {error}"))?;
    let listing = phoneal::bytecode::disassemble(&program)
        .map_err(|error| format!("Phoneal: {error}"))?;
    print!("{listing}");
    Ok(())
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    let result = match arguments.as_slice() {
        [_, command, source] if command == "run" => run_source(source, false),
        [_, command, source] if command == "run-bytecode" => run_source(source, true),
        [_, command, source] if command == "check" => check_source(source),
        [_, command, source, flag, output] if command == "build" && flag == "--output" => {
            build_bytecode(source, output)
        }
        [_, command, file] if command == "inspect" => inspect_bytecode(file),
        [_, command, file] if command == "execute" => execute_bytecode_file(file),
        _ => {
            print_usage();
            process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("{error}");
        process::exit(1);
    }
}
