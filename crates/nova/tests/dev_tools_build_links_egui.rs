//! With the `dev-tools` feature, egui is linked into the `nova` binary: the
//! positive control for `tests/default_build_links_no_egui.rs`, showing that
//! the same probe finds egui when it is there. `mise run test-dev-tools`
//! runs it, in its own target directory.

#![cfg(feature = "dev-tools")]

mod probe;

#[test]
fn the_dev_tools_build_links_egui() {
    let binary = probe::nova_binary();
    assert!(
        probe::contains(&binary, probe::EGUI),
        "no egui in the dev-tools build of {}",
        env!("CARGO_BIN_EXE_nova")
    );
}
