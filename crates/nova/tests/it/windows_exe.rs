//! `nova.exe` as Windows reads it: its subsystem and its resources, read
//! from the built file as PE structures with `object`, and its usage error
//! with a redirected stderr. Their contents (the icon's sizes, the version
//! numbers and strings, the manifest's settings) are nova-winres's tests;
//! these check that the build embeds them.

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

#[test]
fn nova_exe_embeds_its_manifest_version_and_icon() {
    let data = exe();
    let manifest = resource(&data, pe::RT_MANIFEST, nova_winres::MANIFEST_ID).expect("a manifest");
    assert_eq!(
        manifest,
        include_bytes!("../../../../packaging/windows/nova.exe.manifest"),
        "packaging/windows/nova.exe.manifest, as it is"
    );
    assert!(
        resource(&data, pe::RT_VERSION, nova_winres::VERSION_ID).is_some(),
        "VERSIONINFO"
    );
    assert!(
        resource(&data, pe::RT_GROUP_ICON, nova_winres::ICON_ID).is_some(),
        "the icon group the window loads"
    );
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
