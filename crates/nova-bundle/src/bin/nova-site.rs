//! `nova-site [--flatpakref] [OUT_DIR]`: writes the download page,
//! `index.html`, into `OUT_DIR` (by default `target/site`). With
//! `--flatpakref`, which `ci.yml` passes when the deploy serves the signed
//! Flatpak repo, the Linux row leads with the nightly's `.flatpakref`.
//! Run it through cargo, as `mise run site -- [OUT_DIR]`. It runs on any
//! host. See `nova_bundle::site`.
//!
//! `nova-site --archive <TARGET>` and `--archive-dir <TARGET>` print the
//! portable archive's file name or top-level directory for that target
//! instead, for `ci.yml`'s packaging. See `nova_bundle::archive`.
//!
//! Prints the page's path or the name. Exits 2 on a usage error and 1
//! when the write fails.

use std::env;
use std::process::ExitCode;

use nova_bundle::site::host::{Command, command};
use nova_bundle::site::{stock, write_site};
use nova_bundle::system::StdFiles;

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("nova-site: the working directory: {e}");
            return ExitCode::from(2);
        }
    };
    let command = match command(env::args_os().skip(1).collect(), &cwd) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("nova-site: {e}");
            return ExitCode::from(2);
        }
    };
    match command {
        Command::ArchiveFile(archive) => println!("{}", archive.file),
        Command::ArchiveDir(archive) => println!("{}", archive.dir),
        Command::Write { out, flatpak_repo } => {
            match write_site(&mut StdFiles, &out, &stock(flatpak_repo)) {
                Ok(page) => println!("{}", page.display()),
                Err(e) => {
                    eprintln!("nova-site: {}: {e}", out.display());
                    return ExitCode::FAILURE;
                }
            }
        }
    }
    ExitCode::SUCCESS
}
