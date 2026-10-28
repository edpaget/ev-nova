//! `nova-appcast publish --dist <DIR> --assets <FILE> --current <DIR>
//! --feeds <DIR> --prune-list <FILE>`: CI's `publish` job's update-feed
//! logic. It signs the stable installers in `--dist`, copies each to its
//! build-ID name beside it, and writes the two nightly feeds into
//! `--feeds`, when the public key is committed and `UPDATE_SIGNING_KEY`
//! holds its seed; otherwise it prints a `::warning::` and writes none.
//! Either way it writes the release assets to delete into `--prune-list`.
//! Run it through cargo, as `mise run appcast -- publish ...`, or run the
//! built binary with cargo on `PATH`, as CI's signing step does so that
//! nothing compiles beside the seed: it reads the spec with `$CARGO
//! metadata` (else `cargo metadata`), and the build ID from
//! `NOVA_BUILD_ID` (else HEAD's committer time). See `nova_bundle::appcast`.
//!
//! `nova-appcast public-key` prints the public key of the seed in
//! `UPDATE_SIGNING_KEY`, as base64, for the operator to commit.
//!
//! Exits 2 on a usage error, 3 when the current feeds already offer this
//! very build (a re-run whose feeds went up; nothing is written), and 1
//! on any other failure, a build older than the current feeds' included.

use std::env;
use std::ffi::OsString;
use std::process::ExitCode;

use nova_bundle::appcast::host::{Command, USAGE_STATUS, command, exit_status};
use nova_bundle::appcast::publish::{public_key, run};
use nova_bundle::system::{Ed25519Signer, StdEnv, StdFiles, StdProcess};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-appcast: the working directory: {e}");
            return ExitCode::from(USAGE_STATUS);
        }
    };
    let command = match command(env::args_os().skip(1).collect(), &cwd) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("nova-appcast: {e}");
            return ExitCode::from(USAGE_STATUS);
        }
    };
    let result = match command {
        Command::PublicKey => {
            public_key(&StdEnv, Ed25519Signer::from_base64_seed).map(|key| println!("{key}"))
        }
        Command::Publish(paths) => {
            let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
            run(
                &mut StdProcess,
                &StdEnv,
                &StdFiles,
                &mut StdFiles,
                &cargo,
                &cwd,
                &paths,
                Ed25519Signer::from_base64_seed,
            )
            .map(|published| {
                if let Some(reason) = published.skipped {
                    println!(
                        "::warning title=Update feeds not published::{}",
                        reason.message()
                    );
                }
                for feed in &published.feeds {
                    println!("{}", feed.display());
                }
            })
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nova-appcast: {e}");
            ExitCode::from(exit_status(&e))
        }
    }
}
