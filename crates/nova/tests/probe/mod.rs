//! Reading the `nova` binary that cargo built for this very test run, to
//! tell whether egui was linked into it.
//!
//! `CARGO_BIN_EXE_nova` is the binary built with this test's own features.
//! A linked egui leaves its name in the binary on every platform: in
//! mangled symbol names, and in the source paths of its panic locations
//! (`…/egui-0.36.2/src/…`).

use std::path::Path;

/// The byte string a linked egui leaves in the binary.
pub const EGUI: &[u8] = b"egui";

/// Whether `needle` occurs anywhere in `haystack`. `needle` is not empty.
pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The bytes of the `nova` binary built for this test run.
pub fn nova_binary() -> Vec<u8> {
    let path = Path::new(env!("CARGO_BIN_EXE_nova"));
    std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()))
}

#[test]
fn contains_finds_a_needle_anywhere_and_nothing_else() {
    assert!(contains(b"xxeguixx", EGUI), "in the middle");
    assert!(contains(b"eguixx", EGUI), "at the start");
    assert!(contains(b"xxegui", EGUI), "at the end");
    assert!(contains(b"egui", EGUI), "the whole haystack");
    assert!(!contains(b"xxegxuixx", EGUI), "absent");
    assert!(!contains(b"EGUI egu gui", EGUI), "case and partial matches");
    assert!(!contains(b"egu", EGUI), "a needle longer than the haystack");
    assert!(!contains(b"", EGUI), "an empty haystack");
}
