//! `nova-flatpak-repo stage --work <DIR> --pages <DIR>`, with
//! `--bundle-dir <DIR>` for the nightly, and
//! `nova-flatpak-repo sign --work <DIR> --pages <DIR> --dist <DIR>`:
//! CI's `flatpak-repo` job, which publishes the GPG-signed Flatpak
//! repository the Pages site serves at `/flatpak/`, on x86-64 Linux only.
//! See `nova_bundle::linux::flatpak::repo`.
//!
//! `stage` holds no secret and does all the network I/O: it mirrors the
//! live repo from Pages, GPG-verified against the committed
//! `packaging/linux/flatpak/repo-key.asc`, and, for the nightly, imports
//! the plain bundle in `--bundle-dir`. For beta (`NOVA_CHANNEL=beta`) it
//! takes no `--bundle-dir` and imports nothing. `sign` holds the secret
//! key (`FLATPAK_SIGNING_KEY`) alone: it commits the build onto
//! `NOVA_CHANNEL`'s branch (for beta, the nightly branch's commit of the
//! build `NOVA_BUILD_ID` names, copied with no rebuild), signs, prunes and
//! makes static deltas, writes `ev-nova-<channel>.flatpakref` for every
//! channel the repo holds beside the repo in `--pages`, and the
//! repo-following bundle into `--dist`. Run the built binary with cargo on `PATH`, as CI does, so
//! nothing compiles beside the key: it reads the spec with `$CARGO
//! metadata` (else `cargo metadata`), and the build ID from
//! `NOVA_BUILD_ID` (else HEAD's committer time). `mise run flatpak-repo
//! -- ...` builds and runs it. It needs `gpg`, `gpgconf`, `ostree`,
//! `flatpak`, `curl` and GNU `find` on `PATH`.
//!
//! Without `NOVA_CHANNEL`, the committed key, or the secret (`stage`
//! reads `FLATPAK_SIGNING_KEY_SET=true` for it), `stage` prints one
//! `::warning title=Flatpak repo not published::` line and exits 4, and
//! writes nothing; `sign` exits 4 the same way with a plain line. Exits 2
//! on a usage error, an unknown `NOVA_CHANNEL` or a host other than x86-64
//! Linux, and 1 on any other failure.

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use nova_bundle::channel;
use nova_bundle::linux::flatpak::repo::host::{Command, USAGE_STATUS, command, report};
use nova_bundle::linux::flatpak::repo::plan::{sign_repo, stage_repo};
use nova_bundle::linux::flatpak::repo::{RepoHost, url};
use nova_bundle::system::{StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-flatpak-repo: the working directory: {e}");
            return ExitCode::from(USAGE_STATUS);
        }
    };
    let channel = match channel::from_env(&StdEnv) {
        Ok(channel) => channel,
        Err(e) => {
            eprintln!("nova-flatpak-repo: {e}");
            return ExitCode::from(USAGE_STATUS);
        }
    };
    let command = match command(
        env::args_os().skip(1).collect(),
        &cwd,
        env::consts::OS,
        env::consts::ARCH,
        channel,
    ) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("nova-flatpak-repo: {e}");
            return ExitCode::from(USAGE_STATUS);
        }
    };
    let host = RepoHost {
        cargo: env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")),
        channel,
        url: url(),
        key: None,
    };
    let result = match &command {
        Command::Stage(paths) => stage_repo(&mut StdProcess, &mut StdFiles, &StdEnv, &host, paths),
        Command::Sign(paths) => sign_repo(&mut StdProcess, &mut StdFiles, &StdEnv, &host, paths),
    };
    let report = report(&command, &result);
    for line in &report.stdout {
        println!("{line}");
    }
    for line in &report.stderr {
        eprintln!("{line}");
    }
    ExitCode::from(report.status)
}
