//! Against the real EV Nova binary: skips, passing, unless `NOVA_DATA`
//! points at a `Nova Files` folder inside `EV Nova.app` and `otool` is
//! installed.

use std::path::PathBuf;

use nova_disasm::cache::{self, LoadError};
use nova_disasm::index::Index;
use nova_disasm::ports::ToolError;
use nova_disasm::std_adapters::{FsCache, StdBinary, StdTools};

/// `<NOVA_DATA>/../../MacOS/EV Nova`: `Nova Files` sits in the bundle's
/// `Contents/Resources`.
fn binary() -> Option<PathBuf> {
    let data = PathBuf::from(std::env::var_os("NOVA_DATA")?);
    let binary = data.parent()?.parent()?.join("MacOS").join("EV Nova");
    binary.is_file().then_some(binary)
}

fn index() -> Option<Index> {
    let Some(binary) = binary() else {
        eprintln!("skipping: NOVA_DATA does not lead to EV Nova's binary");
        return None;
    };
    let dir = tempfile::tempdir().expect("a temporary directory");
    let cache = FsCache::new(dir.path().to_path_buf());
    let mut warn = Vec::new();
    match cache::load(&binary, &StdBinary, &StdTools::default(), &cache, &mut warn) {
        Ok(listings) => Some(Index::build(&listings)),
        Err(LoadError::Tool(ToolError::Missing { program })) => {
            eprintln!("skipping: {program} is not installed");
            None
        }
        Err(err) => panic!("{err}"),
    }
}

#[test]
fn stock_handle_player_is_indexed_with_named_calls() {
    let Some(index) = index() else { return };
    let found = index.find("_HandlePlayer");
    let handle = found
        .iter()
        .find(|f| f.name == "_HandlePlayer")
        .expect("_HandlePlayer");
    assert_eq!(handle.start, 0x68390);

    let f = index.function_named("_HandlePlayer").unwrap();
    let next = index
        .functions()
        .iter()
        .find(|g| g.start > f.start)
        .expect("a next function");
    assert_eq!(f.end, next.start, "ends at the next label");

    let callees = index.callees(f);
    assert!(
        callees.named.iter().any(|c| c.name.starts_with('_')),
        "{callees:?}"
    );
}

#[test]
fn stock_the_last_symbol_in_a_section_ends_with_the_section() {
    let Some(index) = index() else { return };
    // _C.308.109470 is the last symbol in __TEXT,__const, which ends at
    // 0xddfa0; the next symbol (___progname, 0xde920) is in __DATA.
    let found = index.find("_C.308.109470");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!((found[0].start, found[0].end), (0xdd740, Some(0xddfa0)));
    // A CFString in __DATA,__cfstring, before any symbol there.
    assert!(index.find("0xde1a8").is_empty());
}
