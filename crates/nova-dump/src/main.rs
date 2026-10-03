//! The `nova-dump` binary: wires the std adapters into [`nova_dump::app::run`].

use std::io;
use std::process::ExitCode;

use nova_dump::app::run;
use nova_dump::fs::{StdDataSource, StdOutputRoot};

fn main() -> ExitCode {
    ExitCode::from(run(
        std::env::args_os().skip(1),
        &StdDataSource,
        &StdOutputRoot,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
