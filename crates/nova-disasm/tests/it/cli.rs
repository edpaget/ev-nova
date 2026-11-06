//! The built binary, end to end where no real EV Nova is needed.

use std::process::Command;

#[test]
fn a_missing_binary_exits_2_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let output = Command::new(env!("CARGO_BIN_EXE_nova-disasm"))
        .arg("--binary")
        .arg(&missing)
        .args(["fn", "_HandlePlayer"])
        .env("CARGO_TARGET_DIR", dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "EV Nova binary not found at {}",
            missing.display()
        )),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn a_usage_error_exits_2() {
    let output = Command::new(env!("CARGO_BIN_EXE_nova-disasm"))
        .arg("dump")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage: nova-disasm"));
}
