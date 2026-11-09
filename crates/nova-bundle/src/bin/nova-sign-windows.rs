//! `nova-sign-windows <FILE>...`: signs and verifies the files in place
//! with Azure Artifact Signing, through `signtool` and its dlib (Windows
//! only). Run it through cargo, as `mise run sign-windows -- <FILE>...`.
//! See `nova_bundle::authenticode`.
//!
//! `ARTIFACT_SIGNING_ENDPOINT`, `ARTIFACT_SIGNING_ACCOUNT` and
//! `ARTIFACT_SIGNING_PROFILE` name the account and certificate profile, and
//! the dlib signs in through the Azure CLI, so `az login` (in CI,
//! `azure/login`) must come first. With none of the three set it logs
//! "unsigned" and signs nothing. It downloads the tools, so it needs the
//! network.
//!
//! Exits 2 on a usage error, a non-Windows host or a partial set of
//! variables, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::authenticode::host::sign_host;
use nova_bundle::authenticode::{Authenticode, sign_files};
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-sign-windows: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match sign_host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        &cwd,
        env::consts::OS,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-sign-windows: {e}");
            return ExitCode::from(2);
        }
    };
    let signing = match Authenticode::from_env(&StdEnv) {
        Ok(signing) => signing,
        Err(e) => {
            eprintln!("nova-sign-windows: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("nova-sign-windows: {}", signing.describe());
    match sign_files(
        &mut StdProcess,
        &mut StdFiles,
        &host.cargo,
        &signing,
        &host.files,
    ) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nova-sign-windows: {e}");
            ExitCode::FAILURE
        }
    }
}
