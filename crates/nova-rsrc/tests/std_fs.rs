//! Tests for the `StdForkReader` adapter against real files in a temporary
//! directory owned by each test.

use std::io;

use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, LoadError, ResType, ResourceFile, StdForkReader};

const PICT: ResType = ResType(*b"PICT");

fn fixture_bytes() -> Vec<u8> {
    ForkBuilder::new()
        .resource(PICT, 128, Some(b"Splash"), b"pixels")
        .build()
        .bytes
}

#[test]
fn reads_the_data_fork() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Example.ndat");
    std::fs::write(&path, fixture_bytes()).expect("write fixture");

    let bytes = StdForkReader
        .read_fork(&path, Fork::Data)
        .expect("readable");
    assert_eq!(bytes, Some(fixture_bytes()));
}

#[test]
fn missing_file_is_an_error() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("missing.ndat");

    let error = StdForkReader
        .read_fork(&path, Fork::Data)
        .expect_err("missing file");
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(matches!(
        ResourceFile::open(&path),
        Err(LoadError::Io { path: p, .. }) if p == path
    ));
}

#[test]
fn plain_file_has_no_resource_fork() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("plain.ndat");
    std::fs::write(&path, b"").expect("write empty file");

    let fork = StdForkReader
        .read_fork(&path, Fork::Resource)
        .expect("readable");
    assert_eq!(fork, None);
    assert!(matches!(
        ResourceFile::open(&path),
        Err(LoadError::NoResourceFork { path: p }) if p == path
    ));
}

#[test]
fn missing_file_has_no_resource_fork() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fork = StdForkReader
        .read_fork(&dir.path().join("missing.ndat"), Fork::Resource)
        .expect("absent, not an error");
    assert_eq!(fork, None);
}

#[test]
fn open_parses_a_flattened_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Example.ndat");
    std::fs::write(&path, fixture_bytes()).expect("write fixture");

    let file = ResourceFile::open(&path).expect("opens");
    let res = file.get(PICT, 128).expect("present");
    assert_eq!(res.name(), Some("Splash"));
    assert_eq!(res.data(), b"pixels");
}

#[cfg(target_os = "macos")]
#[test]
fn open_reads_a_real_resource_fork() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Old Plug-in");
    std::fs::write(&path, b"").expect("create file with empty data fork");
    if let Err(error) = write_named_fork(&path, &fixture_bytes()) {
        eprintln!("skipping: cannot write a resource fork here: {error}");
        return;
    }

    let fork = StdForkReader
        .read_fork(&path, Fork::Resource)
        .expect("readable");
    assert_eq!(fork, Some(fixture_bytes()));

    let file = ResourceFile::open(&path).expect("opens via the resource fork");
    assert_eq!(file.get(PICT, 128).expect("present").data(), b"pixels");
}

#[cfg(target_os = "macos")]
fn write_named_fork(path: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    std::fs::write(path.join("..namedfork/rsrc"), bytes)
}

#[cfg(target_os = "macos")]
#[test]
fn resource_fork_errors_other_than_not_found_are_reported() {
    use std::os::unix::fs::PermissionsExt;

    // An unreadable resource fork must not be mistaken for an absent one.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("Locked Plug-in");
    std::fs::write(&path, b"").expect("create file");
    if let Err(error) = write_named_fork(&path, &fixture_bytes()) {
        eprintln!("skipping: cannot write a resource fork here: {error}");
        return;
    }
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    let enforced = std::fs::read(&path).is_err();
    let result = StdForkReader.read_fork(&path, Fork::Resource);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("chmod back");
    if !enforced {
        eprintln!("skipping: permissions are not enforced (running as root?)");
        return;
    }
    let error = result.expect_err("unreadable resource fork");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}
