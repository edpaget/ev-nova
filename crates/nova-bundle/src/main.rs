//! `nova-bundle [OUT_DIR]`: builds `Nova.app` into
//! `ev-nova-universal-apple-darwin.dmg` (macOS only). Run it through cargo,
//! as `mise run bundle -- [OUT_DIR]`: cargo sets `CARGO` and, from
//! `.cargo/config.toml`, `MACOSX_DEPLOYMENT_TARGET`, the app's minimum
//! macOS. See the crate's library docs.
//!
//! Exits 2 on a usage error or a non-macOS host, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::bundle::bundle;
use nova_bundle::host::host;
use nova_bundle::system::{StdFiles, StdProcess};

fn main() -> ExitCode {
    if !cfg!(target_os = "macos") {
        eprintln!("nova-bundle: bundling needs macOS's lipo, codesign and hdiutil");
        return ExitCode::from(2);
    }
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-bundle: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        env::var("MACOSX_DEPLOYMENT_TARGET").ok(),
        &cwd,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-bundle: {e}");
            return ExitCode::from(2);
        }
    };
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
