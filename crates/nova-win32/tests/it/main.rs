//! nova-win32's integration tests, one test binary: each test is a module.
//! They run only on Windows, where the crate has its API.

#[cfg(windows)]
mod console;
#[cfg(windows)]
mod winsparkle;
