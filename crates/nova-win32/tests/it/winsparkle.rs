//! `WinSparkleDll` loading real DLLs: a path with none, a system DLL that
//! isn't WinSparkle, and, opt-in, the packed WinSparkle itself.
//!
//! The last skips, passing, unless `NOVA_WINSPARKLE_DLL` names a
//! `WinSparkle.dll`: CI's Windows `build` leg sets it to the one the
//! installer's packer staged. It resolves every call and checks keys, and
//! never calls `win_sparkle_init`, so it reaches no network and shows no
//! window.

use std::env;
use std::path::PathBuf;

use nova_win32::{WINSPARKLE_FILE, WinSparkleDll, WinSparkleLoadError};

#[test]
fn a_missing_dll_is_a_load_error_naming_its_path() {
    // A folder of the test's own that is never made.
    let path = env::temp_dir()
        .join(format!("nova-win32-no-dll-{}", std::process::id()))
        .join(WINSPARKLE_FILE);
    let Err(error) = WinSparkleDll::load(&path) else {
        panic!("loaded {}", path.display());
    };
    let WinSparkleLoadError::Load { path: named, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(named, &path);
    assert!(
        error.to_string().contains(&path.display().to_string()),
        "{error}"
    );
}

#[test]
fn a_dll_without_the_symbols_is_a_symbol_error() {
    // A real DLL, already in every process, that isn't WinSparkle.
    let system_root = env::var_os("SystemRoot").expect("SystemRoot");
    let kernel32 = PathBuf::from(system_root)
        .join("System32")
        .join("kernel32.dll");
    let Err(error) = WinSparkleDll::load(&kernel32) else {
        panic!("kernel32.dll loaded as WinSparkle");
    };
    let WinSparkleLoadError::Symbol { name, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*name, "win_sparkle_init");
    assert!(error.to_string().contains("win_sparkle_init"), "{error}");
}

/// RFC 8032's TEST 1 public key: a published test vector, valid Ed25519.
const TEST_KEY: &str = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";

#[test]
fn the_real_dll_resolves_every_symbol_and_checks_keys() {
    let Some(path) = env::var_os("NOVA_WINSPARKLE_DLL").filter(|v| !v.is_empty()) else {
        eprintln!("NOVA_WINSPARKLE_DLL is unset: skipping the WinSparkle DLL test");
        return;
    };
    let path = std::path::absolute(PathBuf::from(path)).expect("an absolute path");
    let mut dll = WinSparkleDll::load(&path).unwrap_or_else(|e| panic!("{e}"));
    assert!(dll.set_eddsa_public_key(TEST_KEY), "a valid key");
    assert!(!dll.set_eddsa_public_key("not a key"), "an invalid one");
    eprintln!("WinSparkle DLL test ran against {}", path.display());
}
