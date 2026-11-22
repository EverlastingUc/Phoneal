use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn temp_path(extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "phoneal_cli_{}_{}.{}",
        std::process::id(),
        nonce,
        extension
    ))
}

#[test]
fn cli_builds_inspects_and_runs_phnb_files() {
    let source_path = temp_path("phnl");
    let output_path = temp_path("phnb");
    fs::write(
        &source_path,
        "function double(value: integer) returns integer { return value * 2; } print(double(9));",
    ).unwrap();

    let binary = env!("CARGO_BIN_EXE_phnl");
    let build = Command::new(binary)
        .args(["build", source_path.to_str().unwrap(), "--output", output_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(build.status.success(), "{}", String::from_utf8_lossy(&build.stderr));
    assert!(output_path.exists());

    let inspect = Command::new(binary)
        .args(["inspect", output_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(inspect.status.success(), "{}", String::from_utf8_lossy(&inspect.stderr));
    let listing = String::from_utf8(inspect.stdout).unwrap();
    assert!(listing.contains("double(integer) -> integer"));
    assert!(listing.contains("Call #0 double"));

    let execute = Command::new(binary)
        .args(["execute", output_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(execute.status.success(), "{}", String::from_utf8_lossy(&execute.stderr));
    assert_eq!(String::from_utf8(execute.stdout).unwrap().trim(), "18");

    let run = Command::new(binary)
        .args(["run-bytecode", source_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8(run.stdout).unwrap().trim(), "18");

    let check = Command::new(binary)
        .args(["check", source_path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stderr));

    fs::remove_file(source_path).unwrap();
    fs::remove_file(output_path).unwrap();
}

#[test]
fn cli_rejects_wrong_file_extensions() {
    let binary = env!("CARGO_BIN_EXE_phnl");
    let run = Command::new(binary).args(["inspect", "not-a-bytecode.txt"]).output().unwrap();
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains(".phnb"));
}
