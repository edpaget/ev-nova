//! The FFI module for WinSparkle: [`WinSparkleDll`] loads `WinSparkle.dll`
//! at run time with `libloading`, resolves every call the game makes up
//! front, and offers each as a safe method taking Rust strings. WinSparkle's
//! shutdown request carries no context, so the closure it runs is kept in
//! one process-wide slot, as WinSparkle itself is one per process, and a
//! safe `extern "C"` trampoline runs it.
//!
//! Every `unsafe` block here carries a `SAFETY` comment. The calls and
//! their types are `include/winsparkle.h`'s, WinSparkle 0.9.4: every one
//! `__cdecl`, the C ABI on x64; strings are NUL-terminated, narrow ones
//! UTF-8, wide ones UTF-16 (`wchar_t`).

#![allow(unsafe_code)]

use std::ffi::{CString, c_char, c_int};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use libloading::os::windows::{
    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, Library,
};

/// The DLL's file name, beside `nova.exe`.
pub const FILE: &str = "WinSparkle.dll";

/// What runs when WinSparkle asks the game to quit, from a thread of its
/// own.
pub type ShutdownRequest = Box<dyn Fn() + Send>;

/// Why the DLL couldn't be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The DLL at `path` didn't load: missing, not a DLL, or for another
    /// architecture.
    Load {
        /// The DLL's path.
        path: PathBuf,
        /// The loader's message.
        message: String,
    },
    /// It loaded, but lacks a call the game makes: not WinSparkle, or a
    /// version without it.
    Symbol {
        /// The call's name.
        name: &'static str,
        /// The loader's message.
        message: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load { path, message } => {
                write!(f, "could not load {}: {message}", path.display())
            }
            Self::Symbol { name, message } => write!(f, "{FILE} has no {name}: {message}"),
        }
    }
}

impl std::error::Error for LoadError {}

type SetNarrow = unsafe extern "C" fn(*const c_char);
type SetKey = unsafe extern "C" fn(*const c_char) -> c_int;
type SetDetails = unsafe extern "C" fn(*const u16, *const u16, *const u16);
type SetWide = unsafe extern "C" fn(*const u16);
type SetInt = unsafe extern "C" fn(c_int);
type Callback = extern "C" fn();
type SetCallback = unsafe extern "C" fn(Option<Callback>);
type Call = unsafe extern "C" fn();

/// The closure WinSparkle's shutdown request runs, once given.
static SHUTDOWN: Mutex<Option<ShutdownRequest>> = Mutex::new(None);

/// Puts `on_request` in the slot, replacing any closure there.
fn store(on_request: Option<ShutdownRequest>) {
    *SHUTDOWN.lock().unwrap_or_else(PoisonError::into_inner) = on_request;
}

/// The callback WinSparkle calls, from its own thread: runs the stored
/// closure, if any. A panic in it is caught, so nothing unwinds into C,
/// and the next request still runs.
extern "C" fn shutdown_requested() {
    let Ok(slot) = SHUTDOWN.lock() else {
        return;
    };
    if let Some(on_request) = slot.as_ref() {
        let _ = catch_unwind(AssertUnwindSafe(on_request));
    }
}

/// `text` up to its first NUL, as a C string.
fn narrow(text: &str) -> CString {
    let text = text.split('\0').next().unwrap_or_default();
    CString::new(text).unwrap_or_default()
}

/// `text` up to its first NUL, as NUL-terminated UTF-16.
fn wide(text: &str) -> Vec<u16> {
    let text = text.split('\0').next().unwrap_or_default();
    text.encode_utf16().chain([0]).collect()
}

/// WinSparkle, loaded, with every call the game makes resolved. The
/// library stays loaded as long as this lives, so every pointer stays
/// valid.
pub struct WinSparkleDll {
    init: Call,
    cleanup: Call,
    set_appcast_url: SetNarrow,
    set_eddsa_public_key: SetKey,
    set_app_details: SetDetails,
    set_app_build_version: SetWide,
    set_automatic_check_for_updates: SetInt,
    set_shutdown_request_callback: SetCallback,
    check_update_with_ui: Call,
    _library: Library,
}

/// Resolves `name` in `library` as a `T`.
///
/// # Safety
///
/// `T` must be the symbol's real type.
unsafe fn symbol<T: Copy>(library: &Library, name: &'static str) -> Result<T, LoadError> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    // SAFETY: the caller guarantees `T` is the symbol's type; the value is
    // copied out of the `Symbol`, and stays valid while `library` is
    // loaded, which `WinSparkleDll` keeps it.
    unsafe { library.get::<T>(&bytes) }
        .map(|symbol| *symbol)
        .map_err(|error| LoadError::Symbol {
            name,
            message: error.to_string(),
        })
}

impl WinSparkleDll {
    /// Loads the DLL at `path`, an absolute path, searching only its own
    /// folder and the system's for what it needs (never the current
    /// directory), and resolves every call.
    ///
    /// # Errors
    ///
    /// When it doesn't load, or lacks a call.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        // SAFETY: loading a DLL runs its `DllMain`; WinSparkle's only sets
        // up its own state. The flags keep the search to the DLL's folder
        // and the system's.
        let library = unsafe {
            Library::load_with_flags(
                path,
                LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
            )
        }
        .map_err(|error| LoadError::Load {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        // SAFETY (each): the types are `winsparkle.h`'s declarations of
        // these calls, in the C ABI.
        unsafe {
            Ok(Self {
                init: symbol(&library, "win_sparkle_init")?,
                cleanup: symbol(&library, "win_sparkle_cleanup")?,
                set_appcast_url: symbol(&library, "win_sparkle_set_appcast_url")?,
                set_eddsa_public_key: symbol(&library, "win_sparkle_set_eddsa_public_key")?,
                set_app_details: symbol(&library, "win_sparkle_set_app_details")?,
                set_app_build_version: symbol(&library, "win_sparkle_set_app_build_version")?,
                set_automatic_check_for_updates: symbol(
                    &library,
                    "win_sparkle_set_automatic_check_for_updates",
                )?,
                set_shutdown_request_callback: symbol(
                    &library,
                    "win_sparkle_set_shutdown_request_callback",
                )?,
                check_update_with_ui: symbol(&library, "win_sparkle_check_update_with_ui")?,
                _library: library,
            })
        }
    }

    /// `win_sparkle_set_appcast_url`: the feed to check.
    pub fn set_appcast_url(&mut self, url: &str) {
        let url = narrow(url);
        // SAFETY: a NUL-terminated string that outlives the call, which
        // copies it.
        unsafe { (self.set_appcast_url)(url.as_ptr()) }
    }

    /// `win_sparkle_set_eddsa_public_key`: the key, standard base64.
    /// Whether WinSparkle took it as a valid key.
    pub fn set_eddsa_public_key(&mut self, key: &str) -> bool {
        let key = narrow(key);
        // SAFETY: a NUL-terminated string that outlives the call, which
        // copies it.
        unsafe { (self.set_eddsa_public_key)(key.as_ptr()) != 0 }
    }

    /// `win_sparkle_set_app_details`: the company, the app and the version
    /// it displays.
    pub fn set_app_details(&mut self, company: &str, app: &str, version: &str) {
        let (company, app, version) = (wide(company), wide(app), wide(version));
        // SAFETY: NUL-terminated UTF-16 strings that outlive the call, which
        // copies them.
        unsafe { (self.set_app_details)(company.as_ptr(), app.as_ptr(), version.as_ptr()) }
    }

    /// `win_sparkle_set_app_build_version`: the build ID.
    pub fn set_app_build_version(&mut self, build: &str) {
        let build = wide(build);
        // SAFETY: a NUL-terminated UTF-16 string that outlives the call,
        // which copies it.
        unsafe { (self.set_app_build_version)(build.as_ptr()) }
    }

    /// `win_sparkle_set_automatic_check_for_updates`.
    pub fn set_automatic_check_for_updates(&mut self, on: bool) {
        // SAFETY: takes an int and stores it.
        unsafe { (self.set_automatic_check_for_updates)(c_int::from(on)) }
    }

    /// `win_sparkle_set_shutdown_request_callback`: `on_request` runs, on
    /// WinSparkle's thread, when it asks the game to quit. It replaces any
    /// closure given before, for the whole process.
    pub fn set_shutdown_request_callback(&mut self, on_request: ShutdownRequest) {
        store(Some(on_request));
        // SAFETY: `shutdown_requested` is an `extern "C" fn()` that lives
        // as long as the program and never unwinds.
        unsafe { (self.set_shutdown_request_callback)(Some(shutdown_requested)) }
    }

    /// `win_sparkle_init`: starts the background checks.
    pub fn init(&mut self) {
        // SAFETY: takes nothing; the setters ran first, as it requires.
        unsafe { (self.init)() }
    }

    /// `win_sparkle_cleanup`: stops it.
    pub fn cleanup(&mut self) {
        // SAFETY: takes nothing; safe to call once WinSparkle is loaded.
        unsafe { (self.cleanup)() }
    }

    /// `win_sparkle_check_update_with_ui`: checks now, with its dialog.
    pub fn check_update_with_ui(&mut self) {
        // SAFETY: takes nothing; called after `init`.
        unsafe { (self.check_update_with_ui)() }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn strings_are_cut_at_a_nul_and_terminated() {
        assert_eq!(narrow("a\0b").as_bytes(), b"a");
        assert_eq!(narrow("feed").as_bytes(), b"feed");
        assert_eq!(wide("é\0x"), ["é".encode_utf16().next().expect("é"), 0]);
        assert_eq!(wide(""), [0]);
    }

    /// One test, because the slot is the process's.
    #[test]
    fn the_trampoline_calls_the_stored_closure_and_survives_none_and_a_panic() {
        store(None);
        shutdown_requested();
        let (sender, receiver) = mpsc::channel();
        let sends = move || {
            let sender = sender.clone();
            Box::new(move || {
                let _ = sender.send(());
            }) as ShutdownRequest
        };
        store(Some(sends()));
        std::thread::spawn(|| shutdown_requested())
            .join()
            .expect("asked from another thread");
        assert_eq!(receiver.try_recv(), Ok(()));
        assert!(receiver.try_recv().is_err(), "exactly once");
        store(Some(Box::new(|| panic!("a broken closure"))));
        shutdown_requested();
        store(Some(sends()));
        shutdown_requested();
        assert_eq!(receiver.try_recv(), Ok(()), "still working");
        store(None);
    }
}
