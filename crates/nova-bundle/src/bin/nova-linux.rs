//! `nova-linux [OUT_DIR]`: writes the Linux desktop integration (the
//! desktop file, the AppStream metainfo and the hicolor icons) as a
//! `share/` tree into `OUT_DIR` (by default `target/linux`), to install
//! into `~/.local/share`. Run it through cargo, as
//! `mise run linux-desktop -- [OUT_DIR]`. It runs on any host. See
//! `nova_bundle::linux`.
//!
//! Prints the output directory. Exits 2 on a usage error and 1 when a
//! step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::linux::host::linux_host;
use nova_bundle::linux::share::write_share;
use nova_bundle::system::{StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-linux: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match linux_host(env::args_os().skip(1).collect(), env::var_os("CARGO"), &cwd) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-linux: {e}");
            return ExitCode::from(2);
        }
    };
    match write_share(&mut StdProcess, &mut StdFiles, &host) {
        Ok(out) => {
            println!("{}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-linux: {e}");
            ExitCode::FAILURE
        }
    }
}
