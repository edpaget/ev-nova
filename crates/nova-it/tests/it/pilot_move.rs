//! Moving the pilots out of their old directory with the real
//! filesystem: every file arrives byte for byte, and the old directory
//! goes.

use std::fs;

use nova::pilot_move::{DiskFiles, move_pilots, report_lines};

#[test]
fn every_pilot_file_arrives_byte_for_byte_and_the_old_directory_goes() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let old = tmp.path().join("config").join("nova").join("Pilots");
    let new = tmp.path().join("data").join("nova").join("Pilots");
    fs::create_dir_all(&old).expect("the old directory");
    let files: [(&str, Vec<u8>); 3] = [
        ("Ada.json", br#"{"name":"Ada"}"#.to_vec()),
        ("Last Pilot", b"Ada".to_vec()),
        ("Bob.json.tmp", (0..=255).collect()),
    ];
    for (name, bytes) in &files {
        fs::write(old.join(name), bytes).expect("writes");
    }
    let outcome = move_pilots(&mut DiskFiles, &old, &new);
    for (name, bytes) in &files {
        assert_eq!(&fs::read(new.join(name)).expect("moved"), bytes, "{name}");
    }
    assert!(!old.exists(), "the old directory is removed");
    assert!(outcome.problems.is_empty(), "{:?}", outcome.problems);
    assert_eq!(report_lines(&outcome).len(), 1, "one notice");
    let again = move_pilots(&mut DiskFiles, &old, &new);
    assert_eq!(report_lines(&again), Vec::<String>::new());
}
