use std::{env, fs, process};

fn print_usage() {
    eprintln!("Phoneal 0.1.0 — Rust-hosted language prototype");
    eprintln!("Usage: phnl <run|check> <file.phnl>");
}

fn report(diagnostics: Vec<phoneal::diagnostic::Diagnostic>, source: &str) {
    for diagnostic in diagnostics {
        eprintln!("Phoneal: {}", diagnostic.render(source));
    }
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 || !matches!(arguments[1].as_str(), "run" | "check") {
        print_usage();
        process::exit(2);
    }

    let file_path = &arguments[2];
    if !file_path.ends_with(".phnl") {
        eprintln!("Expected a .phnl file");
        process::exit(2);
    }

    let source = fs::read_to_string(file_path).unwrap_or_else(|error| {
        eprintln!("Cannot read {file_path}: {error}");
        process::exit(1);
    });

    if arguments[1] == "check" {
        match phoneal::compile_with_diagnostics(&source) {
            Ok(_) => println!("Syntax and semantic checks passed: {file_path}"),
            Err(diagnostics) => {
                report(diagnostics, &source);
                process::exit(1);
            }
        }
    } else {
        match phoneal::run(&source) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
            }
            Err(error) => {
                eprintln!("Phoneal: {}", error.render(&source));
                process::exit(1);
            }
        }
    }
}
