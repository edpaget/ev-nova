//! `nova-installer [--exes <DIR>] [OUT_DIR]`: packs the built `nova.exe`
//! and `nova-dump.exe` into `ev-nova-x86_64-pc-windows-msvc.msi` with
//! WiX's `wix` (Windows only). Run it through cargo, as
//! `mise run pack-installer -- --exes <DIR> [OUT_DIR]`, or build and pack
//! in one go with `mise run installer`. See `nova_bundle::installer`.
//!
//! `$NOVA_WIX` names the `wix` to run, else the one on `PATH`. Not `$WIX`:
//! WiX v3's installer sets that to its own folder, which is not a program.
//! It never builds the executables. It downloads WinSparkle, so it needs
//! the network.
//!
//! `$NOVA_CHANNEL` is the channel the executables were built for, as
//! nova's build script read it: the packer prints on stderr which update
//! feed that build names (`nova_bundle::channel::FeedStatus`).
//!
//! `ARTIFACT_SIGNING_ENDPOINT`, `ARTIFACT_SIGNING_ACCOUNT` and
//! `ARTIFACT_SIGNING_PROFILE`, all set, sign the staged `WinSparkle.dll`
//! (after its pin check, before `wix build`) and the `.msi` with Artifact
//! Signing, which needs a prior `az login`; with none set the installer is
//! unsigned (`nova_bundle::authenticode`). The packer prints which.
//!
//! Exits 2 on a usage error, a non-Windows host, an unknown channel or a
//! partial set of signing variables, and 1 when a step fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::authenticode::Authenticode;
use nova_bundle::channel;
use nova_bundle::installer::host::installer_host;
use nova_bundle::installer::plan::pack;
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-installer: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match installer_host(
        env::args_os().skip(1).collect(),
        env::var_os("CARGO"),
        env::var_os("NOVA_WIX"),
        &cwd,
        env::consts::OS,
    ) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("nova-installer: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match channel::from_env(&StdEnv) {
        Ok(channel) => host.for_channel(channel),
        Err(e) => {
            eprintln!("nova-installer: {e}");
            return ExitCode::from(2);
        }
    };
    let host = match Authenticode::from_env(&StdEnv) {
        Ok(signing) => host.for_signing(signing),
        Err(e) => {
            eprintln!("nova-installer: {e}");
            return ExitCode::from(2);
        }
    };
    match pack(&mut StdProcess, &mut StdFiles, &host) {
        Ok(packed) => {
            eprintln!("nova-installer: {}", packed.feed.describe());
            eprintln!("nova-installer: {}", packed.signing.describe());
            println!("{}", packed.msi.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("nova-installer: {e}");
            ExitCode::FAILURE
        }
    }
}
