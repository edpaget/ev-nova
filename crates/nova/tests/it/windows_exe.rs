//! `nova.exe` as Windows reads it: its subsystem and its resources, read
//! from the built file as PE structures with `object`, and its usage error
//! with a redirected stderr. The manifest reads back byte for byte, the
//! version numbers from `VS_FIXEDFILEINFO` and the icon sizes from the
//! group icon's directory; the rest of their contents (the version
//! strings, the manifest's settings) are nova-winres's tests.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use object::LittleEndian as LE;
use object::pe;
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64, ResourceDirectoryTable};

/// The built `nova.exe`.
fn exe() -> Vec<u8> {
    std::fs::read(env!("CARGO_BIN_EXE_nova")).expect("nova.exe")
}

/// The entry of `table` with the id `id`.
fn by_id<'a>(
    table: &ResourceDirectoryTable<'a>,
    id: u16,
) -> Option<&'a pe::ImageResourceDirectoryEntry> {
    table
        .entries
        .iter()
        .find(|entry| entry.name_or_id().id() == Some(id))
}

/// The bytes of resource `id` of type `kind`, in its first language.
fn resource(data: &[u8], kind: u16, id: u16) -> Option<Vec<u8>> {
    let file = PeFile64::parse(data).expect("a 64-bit PE");
    let sections = file.section_table();
    let rsrc = file
        .data_directories()
        .resource_directory(data, &sections)
        .expect("a readable resource directory")?;
    let root = rsrc.root().expect("a root table");
    let names = by_id(&root, kind)?.data(rsrc).ok()?.table()?;
    let languages = by_id(&names, id)?.data(rsrc).ok()?.table()?;
    let entry = languages.entries.first()?.data(rsrc).ok()?.data()?;
    let bytes = sections.pe_data_at(data, entry.offset_to_data.get(LE))?;
    Some(bytes.get(..entry.size.get(LE) as usize)?.to_vec())
}

#[test]
fn nova_exe_is_a_windows_gui_program() {
    let data = exe();
    let file = PeFile64::parse(data.as_slice()).expect("a 64-bit PE");
    assert_eq!(
        file.nt_headers().optional_header().subsystem(),
        pe::IMAGE_SUBSYSTEM_WINDOWS_GUI
    );
}

/// The little-endian `u16` at `at` in `bytes`.
fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// The little-endian `u32` at `at` in `bytes`.
fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"))
}

/// The file and product versions of the `VS_FIXEDFILEINFO` in a
/// `VERSIONINFO` blob, each as `[MS, LS]`: found by its signature, which
/// is DWORD-aligned, and followed by the struct version.
fn fixed_versions(blob: &[u8]) -> ([u32; 2], [u32; 2]) {
    let at = (0..blob.len().saturating_sub(23))
        .step_by(4)
        .find(|&at| u32_at(blob, at) == 0xFEEF_04BD)
        .expect("a VS_FIXEDFILEINFO");
    (
        [u32_at(blob, at + 8), u32_at(blob, at + 12)],
        [u32_at(blob, at + 16), u32_at(blob, at + 20)],
    )
}

/// Each entry of a `GRPICONDIR` as its width in pixels (a byte of 0 is
/// 256) and the id of its `RT_ICON`.
fn group_entries(group: &[u8]) -> Vec<(u32, u16)> {
    (0..usize::from(u16_at(group, 4)))
        .map(|n| {
            let entry = 6 + 14 * n;
            let width = match group[entry] {
                0 => 256,
                width => u32::from(width),
            };
            (width, u16_at(group, entry + 12))
        })
        .collect()
}

#[test]
fn nova_exe_embeds_its_manifest_version_and_icon() {
    let data = exe();
    let manifest = resource(&data, pe::RT_MANIFEST, nova_winres::MANIFEST_ID).expect("a manifest");
    assert_eq!(
        manifest,
        include_bytes!("../../../../packaging/windows/nova.exe.manifest"),
        "packaging/windows/nova.exe.manifest, as it is"
    );

    let version = resource(&data, pe::RT_VERSION, nova_winres::VERSION_ID).expect("VERSIONINFO");
    let [a, b, c, d] = nova_winres::version_quad(env!("CARGO_PKG_VERSION"))
        .map(|quad| quad.map(u32::from))
        .expect("the package version as four numbers");
    let quad = [a << 16 | b, c << 16 | d];
    assert_eq!(
        fixed_versions(&version),
        (quad, quad),
        "file and product versions"
    );

    let group = resource(&data, pe::RT_GROUP_ICON, nova_winres::ICON_ID)
        .expect("the icon group the window loads");
    let entries = group_entries(&group);
    assert_eq!(
        entries.iter().map(|&(width, _)| width).collect::<Vec<_>>(),
        nova_winres::ICON_SIZES,
        "the icon's sizes"
    );
    for (width, id) in entries {
        assert!(
            resource(&data, pe::RT_ICON, id).is_some(),
            "the {width} px icon, RT_ICON {id}"
        );
    }
}

#[test]
fn a_usage_error_with_stderr_redirected_prints_there_and_shows_no_box() {
    let home = tempfile::tempdir().expect("a temporary directory");
    let mut child = Command::new(env!("CARGO_BIN_EXE_nova"))
        .args(["a", "b"])
        .env_remove("NOVA_DATA")
        // Its startup log and settings in a directory of the test's own.
        .env("LOCALAPPDATA", home.path())
        .env("APPDATA", home.path())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("nova.exe starts");
    // A box would wait for a click: never wait on it.
    let deadline = Instant::now() + Duration::from_secs(60);
    let status = loop {
        if let Some(status) = child.try_wait().expect("waits") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("nova.exe did not exit: a message box?");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("piped")
        .read_to_string(&mut stderr)
        .expect("UTF-8");
    assert_eq!(status.code(), Some(2), "{stderr}");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        ["usage: nova [NOVA_FILES_DIR] (or set NOVA_DATA to the Nova Files directory)"]
    );
}
