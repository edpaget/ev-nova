//! `nova-flatpak [OUT_DIR]`: builds the game from source with
//! `flatpak-builder` on the freedesktop runtime and packs it into
//! `ev-nova-x86_64-unknown-linux-gnu.flatpak` in `OUT_DIR` (by default
//! `target/flatpak/out`), on x86-64 Linux only. Run it through cargo, as
//! `mise run flatpak -- [OUT_DIR]`. See `nova_bundle::linux::flatpak`.
//!
//! It needs `flatpak` and `flatpak-builder` on `PATH`, and the Flathub
//! remote added for the user
//! (`flatpak remote-add --user --if-not-exists flathub
//! https://dl.flathub.org/repo/flathub.flatpakrepo`), which the runtime,
//! SDK and Rust extension are installed from when missing. It vendors the
//! crates with the network, then builds without it.
//!
//! Prints the bundle's path. Exits 2 on a usage error or a host other
//! than x86-64 Linux, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::linux::flatpak::host::flatpak_host;
use nova_bundle::linux::flatpak::plan::build_flatpak;
use nova_bundle::system::{StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-flatpak: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match flatpak_host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        &cwd,
        env::consts::OS,
        env::consts::ARCH,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-flatpak: {e}");
            return ExitCode::from(2);
        }
    };
    match build_flatpak(&mut StdProcess, &mut StdFiles, &host) {
        Ok(bundle) => {
            println!("{}", bundle.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-flatpak: {e}");
            ExitCode::FAILURE
        }
    }
}
