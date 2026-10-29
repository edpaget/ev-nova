//! The `nova-build-id` binary, with `NOVA_BUILD_ID` set on its own
//! process only, so no git is needed and nothing outside the child changes.

use std::process::Command;

fn nova_build_id(value: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nova-build-id"))
        .env("NOVA_BUILD_ID", value)
        .output()
        .expect("nova-build-id runs")
}

#[test]
fn prints_the_variable_s_id() {
    let output = nova_build_id("20010203040506");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf-8"),
        "20010203040506\n"
    );
}

#[test]
fn a_malformed_variable_fails_and_names_the_value() {
    let output = nova_build_id("2001-02-03");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).expect("utf-8");
    assert!(stderr.contains("`2001-02-03`"), "{stderr}");
}
