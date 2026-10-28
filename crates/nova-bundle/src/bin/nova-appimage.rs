//! `nova-appimage [OUT_DIR]`: builds `nova` and packs it, with the Linux
//! desktop file, icons and metainfo, into
//! `ev-nova-x86_64-unknown-linux-gnu.AppImage` in `OUT_DIR` (by default
//! `target/appimage/out`), on x86-64 Linux only. Run it through cargo, as
//! `mise run appimage -- [OUT_DIR]`. See `nova_bundle::linux::appimage`.
//!
//! `$NOVA_APPIMAGETOOL` names the `appimagetool` to run, else the one on
//! `PATH`. `$NOVA_APPIMAGE_RUNTIME` names the type2 runtime to embed, and
//! must be set: appimagetool would otherwise download one. Built where
//! there is no FUSE (a container), appimagetool itself needs
//! `APPIMAGE_EXTRACT_AND_RUN=1`.
//!
//! Prints the AppImage's path. Exits 2 on a usage error or a host other
//! than x86-64 Linux, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::linux::appimage::host::appimage_host;
use nova_bundle::linux::appimage::plan::build_appimage;
use nova_bundle::linux::appimage::{RUNTIME_VAR, TOOL_VAR};
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-appimage: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match appimage_host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        env::var_os(TOOL_VAR),
        env::var_os(RUNTIME_VAR),
        &cwd,
        env::consts::OS,
        env::consts::ARCH,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-appimage: {e}");
            return ExitCode::from(2);
        }
    };
    match build_appimage(&mut StdProcess, &mut StdFiles, &StdEnv, &host) {
        Ok(appimage) => {
            println!("{}", appimage.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-appimage: {e}");
            ExitCode::FAILURE
        }
    }
}
