//! Without the `dev-tools` feature (the default), egui is not linked into
//! the `nova` binary.
//!
//! The probe reads the very binary cargo built for this test run, with the
//! same features, and finds no `egui` in it. Code built by default must
//! therefore never hold that word in a string literal; doc comments are
//! fine. `tests/dev_tools_build_links_egui.rs` is the positive control:
//! with the feature on, the same probe finds egui.
//!
//! The default build and the `dev-tools` build must not share a target
//! directory, or one would relink the other's binary under this test:
//! `mise run test-dev-tools` and `mise run dev` use their own.

#![cfg(not(feature = "dev-tools"))]

mod probe;

#[test]
fn the_default_build_does_not_link_egui() {
    let binary = probe::nova_binary();
    assert!(binary.len() > 1_000_000, "{} bytes", binary.len());
    assert!(
        !probe::contains(&binary, probe::EGUI),
        "egui is linked into the default build of {}: either default-built code \
         uses egui, or this target directory last built nova with `--features \
         dev-tools` (use `mise run test-dev-tools`, which has its own)",
        env!("CARGO_BIN_EXE_nova")
    );
}
