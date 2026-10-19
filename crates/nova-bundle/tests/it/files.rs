//! `StdFiles` against the real filesystem, in a temporary directory each
//! test owns.

use std::fs;
use std::io;

use nova_bundle::ports::Files;
use nova_bundle::system::StdFiles;

#[test]
fn creates_nested_directories_and_writes_files() {
    let dir = tempfile::tempdir().expect("temp dir");
    let nested = dir.path().join("a").join("b");
    StdFiles.create_dir_all(&nested).expect("create");
    let file = nested.join("f.txt");
    StdFiles.write(&file, b"one").expect("write");
    StdFiles.write(&file, b"two").expect("rewrite");
    assert_eq!(fs::read(&file).expect("read"), b"two");
}

#[test]
fn copies_a_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    fs::write(&from, b"bytes").expect("seed");
    StdFiles.copy(&from, &to).expect("copy");
    assert_eq!(fs::read(&to).expect("read"), b"bytes");
}

#[test]
fn a_copy_of_a_missing_file_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let error = StdFiles
        .copy(&dir.path().join("missing"), &dir.path().join("to"))
        .expect_err("fails");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[test]
fn removes_a_tree_and_an_absent_one_is_fine() {
    let dir = tempfile::tempdir().expect("temp dir");
    let tree = dir.path().join("tree");
    fs::create_dir_all(tree.join("inner")).expect("seed");
    fs::write(tree.join("inner").join("f"), b"x").expect("seed");
    StdFiles.remove_dir_all(&tree).expect("remove");
    assert!(!tree.exists());
    StdFiles.remove_dir_all(&tree).expect("absent is fine");
}

#[test]
fn removing_a_file_as_a_directory_fails() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("file");
    fs::write(&file, b"x").expect("seed");
    assert!(StdFiles.remove_dir_all(&file).is_err());
}

#[cfg(unix)]
#[test]
fn makes_a_symbolic_link() {
    let dir = tempfile::tempdir().expect("temp dir");
    let link = dir.path().join("Applications");
    StdFiles
        .symlink(std::path::Path::new("/Applications"), &link)
        .expect("link");
    assert_eq!(
        fs::read_link(&link).expect("a link"),
        std::path::Path::new("/Applications")
    );
}

#[cfg(not(unix))]
#[test]
fn a_symbolic_link_is_unsupported_off_unix() {
    let dir = tempfile::tempdir().expect("temp dir");
    let error = StdFiles
        .symlink(std::path::Path::new("/Applications"), &dir.path().join("l"))
        .expect_err("unsupported");
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
}
