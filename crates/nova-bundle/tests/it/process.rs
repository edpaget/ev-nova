//! `StdProcess` running real programs: cargo, which every platform has,
//! and this crate's own `nova-build-id`, which prints the variable it is
//! given.

use nova_bundle::ports::{Invocation, Process, RunError};
use nova_bundle::system::StdProcess;

fn cargo() -> Invocation {
    Invocation::new(env!("CARGO"))
}

#[test]
fn a_captured_run_returns_its_standard_output() {
    let stdout = StdProcess
        .run(&cargo().arg("--version").captured())
        .expect("cargo runs");
    let text = String::from_utf8(stdout).expect("utf-8");
    assert!(text.starts_with("cargo "), "{text}");
}

#[test]
fn an_uncaptured_run_returns_nothing() {
    let stdout = StdProcess
        .run(&cargo().arg("--version"))
        .expect("cargo runs");
    assert!(stdout.is_empty());
}

#[test]
fn a_failing_program_is_an_error_with_its_exit_code() {
    for invocation in [
        cargo().arg("no-such-subcommand-for-nova-bundle"),
        cargo().arg("no-such-subcommand-for-nova-bundle").captured(),
    ] {
        let error = StdProcess.run(&invocation).expect_err("fails");
        assert!(
            matches!(error, RunError::Failed(Some(code)) if code != 0),
            "{error:?}"
        );
    }
}

#[test]
fn a_missing_program_is_a_spawn_error() {
    for invocation in [
        Invocation::new("no-such-program-for-nova-bundle"),
        Invocation::new("no-such-program-for-nova-bundle").captured(),
    ] {
        let error = StdProcess.run(&invocation).expect_err("fails");
        assert!(matches!(error, RunError::Spawn(_)), "{error:?}");
    }
}

#[test]
fn std_process_sets_the_invocation_s_environment() {
    let stdout = StdProcess
        .run(
            &Invocation::new(env!("CARGO_BIN_EXE_nova-build-id"))
                .env("NOVA_BUILD_ID", "20010203040506")
                .captured(),
        )
        .expect("nova-build-id runs");
    assert_eq!(
        String::from_utf8(stdout).expect("utf-8"),
        "20010203040506\n"
    );
}
