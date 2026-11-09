//! The Windows tools the installer's packer drives, run for real through
//! `StdProcess` with the argument lists the core builds: `certutil`'s
//! digest as the check reads it, and `tar` (Windows' own bsdtar, which
//! reads zip) unpacking the named members of a zip shaped as WinSparkle's
//! release, stripped, and the x64 directories of zips shaped as the two
//! signing tools' NuGet packages (a `.nupkg` is a zip). Each works in a
//! temporary directory it owns.

use std::fs;
use std::path::Path;

use nova_bundle::authenticode::Account;
use nova_bundle::authenticode::tools::{self, AuthenticodeLayout, DLIB, SIGNTOOL};
use nova_bundle::bundle::{BundleError, Step};
use nova_bundle::digest::{DigestTool, Pin, check_sha256};
use nova_bundle::installer::layout::InstallerLayout;
use nova_bundle::installer::winsparkle::{self, DLL_MEMBER, LICENCES, TOP};
use nova_bundle::ports::{Invocation, Process};
use nova_bundle::system::StdProcess;

#[test]
fn certutil_digests_a_file_as_the_check_reads_it() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("abc");
    fs::write(&file, b"abc").expect("seed");
    let abc = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let pin = Pin {
        what: "abc",
        sha256: abc,
    };
    check_sha256(&mut StdProcess, &file, pin, DigestTool::Certutil).expect("the digest of abc");
    let error = check_sha256(
        &mut StdProcess,
        &file,
        winsparkle::DLL_PIN,
        DigestTool::Certutil,
    )
    .expect_err("another digest");
    assert!(
        matches!(&error, BundleError::Checksum { found, .. } if found == abc),
        "{error:?}"
    );
}

/// Writes `bytes` to `path`, making its directories.
fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("dirs");
    fs::write(path, bytes).expect("write");
}

#[test]
fn tar_unpacks_the_named_zip_members_stripped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let tree = dir.path().join("tree");
    let top = tree.join(TOP);
    let release = top.join("x64").join("Release");
    write(&release.join("WinSparkle.dll"), b"the dll");
    write(&release.join("WinSparkle.lib"), b"not wanted");
    write(&top.join("COPYING"), b"mit");
    write(&top.join("COPYING.expat"), b"expat");
    write(&top.join("README.md"), b"not wanted");
    let target = dir.path().join("target");
    let layout = InstallerLayout::new(&target, None, None).expect("layout");
    fs::create_dir_all(&layout.downloads).expect("downloads");
    StdProcess
        .run(
            &Invocation::new("tar")
                .arg("-a")
                .arg("-cf")
                .arg(layout.winsparkle_archive())
                .arg("-C")
                .arg(&tree)
                .arg(TOP),
        )
        .expect("the fixture zip");
    let unpacks: Vec<Invocation> = winsparkle::stage(&layout)
        .into_iter()
        .filter_map(|step| match step {
            Step::Run(invocation) if invocation.program == "tar" => Some(invocation),
            _ => None,
        })
        .collect();
    assert_eq!(unpacks.len(), 2, "{unpacks:?}");
    for unpack in &unpacks {
        StdProcess.run(unpack).expect("unpacked");
    }
    let mut names: Vec<String> = fs::read_dir(&layout.downloads)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "COPYING",
            "COPYING.expat",
            "WinSparkle-0.9.4.zip",
            "WinSparkle.dll"
        ]
    );
    assert_eq!(
        fs::read(layout.winsparkle_dll()).expect("the dll"),
        b"the dll"
    );
    for (member, _) in LICENCES {
        assert!(layout.winsparkle_licence(member).is_file(), "{member}");
    }
    assert!(DLL_MEMBER.starts_with(TOP));
}

/// The names of the files in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("listed")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Zips the tree at `tree` (its `bin` directory and `[Content_Types].xml`,
/// as a `.nupkg` holds them) to `nupkg`.
fn zip(tree: &Path, nupkg: &Path) {
    StdProcess
        .run(
            &Invocation::new("tar")
                .arg("-a")
                .arg("-cf")
                .arg(nupkg)
                .arg("-C")
                .arg(tree)
                .args(["bin", "[Content_Types].xml"]),
        )
        .expect("the fixture nupkg");
}

#[test]
fn tar_unpacks_the_signing_tools_from_nupkg_shaped_zips() {
    let dir = tempfile::tempdir().expect("temp dir");
    let layout = AuthenticodeLayout::new(&dir.path().join("target"));
    fs::create_dir_all(&layout.downloads).expect("downloads");
    fs::create_dir_all(&layout.dlib_dir).expect("dlib dir");
    fs::create_dir_all(&layout.signtool_dir).expect("signtool dir");

    let dlib_tree = dir.path().join("dlib-tree");
    let x64 = dlib_tree.join("bin").join("x64");
    write(&x64.join("Azure.CodeSigning.Dlib.dll"), b"the dlib");
    write(&x64.join("Azure.Core.dll"), b"a dependency");
    write(
        &dlib_tree
            .join("bin")
            .join("x86")
            .join("Azure.CodeSigning.Dlib.dll"),
        b"x86",
    );
    write(&dlib_tree.join("[Content_Types].xml"), b"<Types/>");
    zip(&dlib_tree, &layout.nupkg(&DLIB));

    let sdk_tree = dir.path().join("sdk-tree");
    let bin = sdk_tree.join("bin").join("10.0.26100.0");
    write(&bin.join("x64").join("signtool.exe"), b"signtool");
    write(&bin.join("x64").join("signtool.exe.manifest"), b"manifest");
    write(&bin.join("x86").join("signtool.exe"), b"x86");
    write(&sdk_tree.join("[Content_Types].xml"), b"<Types/>");
    zip(&sdk_tree, &layout.nupkg(&SIGNTOOL));

    let account = Account {
        endpoint: "https://eus.codesigning.azure.net/".into(),
        account: "a".into(),
        profile: "p".into(),
        correlation: None,
    };
    let unpacks: Vec<Invocation> = tools::stage(&layout, &account)
        .into_iter()
        .filter_map(|step| match step {
            Step::Run(invocation) if invocation.program == "tar" => Some(invocation),
            _ => None,
        })
        .collect();
    assert_eq!(unpacks.len(), 2, "{unpacks:?}");
    for unpack in &unpacks {
        StdProcess.run(unpack).expect("unpacked");
    }
    assert_eq!(
        names(&layout.dlib_dir),
        ["Azure.CodeSigning.Dlib.dll", "Azure.Core.dll"]
    );
    assert_eq!(fs::read(layout.dlib()).expect("the dlib"), b"the dlib");
    assert_eq!(
        names(&layout.signtool_dir),
        ["signtool.exe", "signtool.exe.manifest"]
    );
    assert_eq!(
        fs::read(layout.signtool_exe()).expect("signtool"),
        b"signtool"
    );
}
