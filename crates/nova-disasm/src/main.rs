//! The `nova-disasm` binary: wires the std adapters into
//! [`nova_disasm::app::run`].

use std::io;
use std::process::ExitCode;

use nova_disasm::app::{Ports, run};
use nova_disasm::locate::cache_dir;
use nova_disasm::std_adapters::{FsCache, StdBinary, StdEnv, StdTools, StdWorkspace};

fn main() -> ExitCode {
    let workspace = StdWorkspace::from_build();
    let cache = FsCache::new(cache_dir(&StdEnv, &workspace));
    let tools = StdTools::default();
    let ports = Ports {
        binary: &StdBinary,
        tools: &tools,
        cache: &cache,
        env: &StdEnv,
        workspace: &workspace,
    };
    ExitCode::from(run(
        std::env::args_os().skip(1),
        &ports,
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ))
}
