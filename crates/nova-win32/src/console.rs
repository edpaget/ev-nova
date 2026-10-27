//! The FFI module: `GetStdHandle`, `GetFileType` and `AttachConsole`.
//!
//! Every `unsafe` block here carries a `SAFETY` comment.

#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{FILE_TYPE_UNKNOWN, GetFileType};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
};

/// Whether the process has a standard error handle that writes somewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StdHandle {
    /// None: null, `INVALID_HANDLE_VALUE`, or a handle whose file type is
    /// unknown (a console handle inherited by a process with no console).
    Missing,
    /// A console, a file or a pipe.
    Present,
}

/// The process's standard error handle, as [`GetStdHandle`] gives it now:
/// Rust's std looks it up on every write, so this is what `eprintln!`
/// would write to.
#[must_use]
pub fn stderr_handle() -> StdHandle {
    // SAFETY: `GetStdHandle` takes a constant and reads process state; it
    // has no preconditions.
    let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return StdHandle::Missing;
    }
    // SAFETY: `handle` is the process's own standard handle, neither null
    // nor invalid; `GetFileType` only queries it, and an unusable handle
    // gives `FILE_TYPE_UNKNOWN`, not undefined behaviour.
    let file_type = unsafe { GetFileType(handle) };
    if file_type == FILE_TYPE_UNKNOWN {
        StdHandle::Missing
    } else {
        StdHandle::Present
    }
}

/// Attaches the process to the console of the process that started it,
/// which also gives it that console's standard handles; whether it did.
/// It does not when the parent has no console (Explorer) or the process
/// already has one.
#[must_use]
pub fn attach_parent_console() -> bool {
    // SAFETY: `AttachConsole` takes a process id (here the constant for
    // the parent) and changes only the process's console; it has no
    // preconditions, and fails cleanly when there is no console to attach.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}
