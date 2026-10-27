//! `nova-bundle [OUT_DIR]`: builds `Nova.app` into
//! `ev-nova-aarch64-apple-darwin.dmg` (macOS only). Run it through cargo,
//! as `mise run bundle -- [OUT_DIR]`: cargo sets `CARGO` and, from
//! `.cargo/config.toml`, `MACOSX_DEPLOYMENT_TARGET`, the app's minimum
//! macOS. See the crate's library docs.
//!
//! Signs with Developer ID and notarizes when the six `MACOS_*`/`APPLE_*`
//! signing variables are set, and ad-hoc when none is (see
//! `nova_bundle::signing`).
//!
//! Exits 2 on a usage error, a non-macOS host or partly set signing
//! variables, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::bundle::bundle;
use nova_bundle::host::host;
use nova_bundle::signing::Signing;
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-bundle: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let signing = match Signing::from_env(&StdEnv) {
        Ok(signing) => signing,
        Err(e) => {
            eprintln!("nova-bundle: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        env::var("MACOSX_DEPLOYMENT_TARGET").ok(),
        &cwd,
        env::consts::OS,
        signing,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-bundle: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("nova-bundle: {}", host.signing.describe());
    match bundle(&mut StdProcess, &mut StdFiles, &host) {
        Ok(dmg) => {
            println!("{}", dmg.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-bundle: {e}");
            ExitCode::FAILURE
        }
    }
}
