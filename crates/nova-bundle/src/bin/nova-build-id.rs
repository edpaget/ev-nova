//! `nova-build-id`: prints the build ID (see `nova_bundle::build_id`):
//! `NOVA_BUILD_ID` when it is set, else the committer time of HEAD in the
//! git checkout of the working directory. Run it as `mise run build-id`.
//!
//! Exits 1 when there is no ID to print.

use std::env;
use std::process::ExitCode;

use nova_bundle::build_id::resolve;
use nova_bundle::system::{StdEnv, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-build-id: the working directory: {e}");
            return ExitCode::FAILURE;
        }
    };
    match resolve(&StdEnv, &mut StdProcess, &cwd) {
        Ok(id) => {
            println!("{id}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-build-id: {e}");
            ExitCode::FAILURE
        }
    }
}
