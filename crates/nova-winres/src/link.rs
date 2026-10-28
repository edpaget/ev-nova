//! The linker arguments `nova.exe` is built with, by target environment.

/// The main thread's stack, in bytes: 8 MiB, what macOS and Linux give a
/// program's main thread by default.
///
/// Windows reserves only the 1 MiB its linker defaults to. nova's debug
/// `main` with the `dev-tools` feature needs more than that before it
/// even parses its arguments: on `windows-latest` (CI run 40518362148)
/// `windows_exe::a_usage_error_with_stderr_redirected_prints_there_and_shows_no_box`
/// died with `thread 'main' has overflowed its stack` and exit code
/// 0xC00000FD (`STATUS_STACK_OVERFLOW`) instead of the usage error's 2.
/// `nova::main`'s own frame is the cause: unoptimised, it holds every
/// value `main` keeps by value, about 2.2 MiB on aarch64 macOS, and takes
/// it all on entry. The game loop's frames below it (the runner's event
/// handler about 1.1 MiB, `AppScreen::with_rulebook` about 0.6 MiB) bring
/// the debug path to nearly 5 MiB. Matching the other platforms' stack
/// keeps a frame that fits there from overflowing only on Windows.
pub const MAIN_STACK_BYTES: u32 = 8 * 1024 * 1024;

/// The arguments the build script passes the linker for `nova.exe` when
/// the target's environment (`CARGO_CFG_TARGET_ENV`) is `target_env`.
///
/// For MSVC: `/MANIFEST:NO`, since the resource script embeds the only
/// manifest and MSVC's linker would otherwise make its own, and
/// `/STACK:` [`MAIN_STACK_BYTES`]. Any other environment gets none: the
/// build only supports MSVC on Windows.
#[must_use]
pub fn link_args(target_env: &str) -> Vec<String> {
    match target_env {
        "msvc" => vec![
            "/MANIFEST:NO".to_owned(),
            format!("/STACK:{MAIN_STACK_BYTES}"),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msvc_links_without_its_own_manifest_and_with_an_8_mib_main_stack() {
        assert_eq!(link_args("msvc"), ["/MANIFEST:NO", "/STACK:8388608"]);
    }

    #[test]
    fn another_environment_gets_no_arguments() {
        assert!(link_args("gnu").is_empty());
        assert!(link_args("").is_empty());
    }
}
