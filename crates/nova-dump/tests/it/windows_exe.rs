//! `nova-dump.exe` as Windows reads it, from the built file with `object`.

use object::pe;
use object::read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64};

#[test]
fn nova_dump_exe_stays_a_console_program() {
    let data = std::fs::read(env!("CARGO_BIN_EXE_nova-dump")).expect("nova-dump.exe");
    let file = PeFile64::parse(data.as_slice()).expect("a 64-bit PE");
    assert_eq!(
        file.nt_headers().optional_header().subsystem(),
        pe::IMAGE_SUBSYSTEM_WINDOWS_CUI
    );
}
