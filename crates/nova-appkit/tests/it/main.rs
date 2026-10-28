//! nova-appkit's integration tests, built as one test binary with
//! `harness = false`, so its `main` runs on the process's main thread,
//! where `AppKit` can build the real menu bar.
//!
//! The binary speaks as much of libtest's command line as `cargo test`
//! and `cargo nextest` ask of it: `--list` (with `--format terse`, and
//! `--ignored`, which lists none) prints each test as `name: test`, and a
//! run takes an optional name filter (`--exact` for an exact match),
//! ignoring the other flags. Each test is a module of this binary; add it
//! to [`TESTS`].

use std::process::ExitCode;

#[cfg(target_os = "macos")]
mod apple_events;
#[cfg(target_os = "macos")]
mod menu_bar;
#[cfg(target_os = "macos")]
mod modal;

/// Every test, by name. Each panics on failure.
#[cfg(target_os = "macos")]
const TESTS: &[(&str, fn())] = &[
    (
        "apple_events::the_installed_handlers_send_the_games_commands_until_dropped",
        apple_events::the_installed_handlers_send_the_games_commands_until_dropped,
    ),
    (
        "menu_bar::the_installed_bar_is_the_applications_and_sends_the_games_commands",
        menu_bar::the_installed_bar_is_the_applications_and_sends_the_games_commands,
    ),
    (
        "modal::with_no_modal_running_it_does_nothing",
        modal::with_no_modal_running_it_does_nothing,
    ),
    (
        "modal::it_aborts_a_running_modal_session",
        modal::it_aborts_a_running_modal_session,
    ),
];

#[cfg(not(target_os = "macos"))]
const TESTS: &[(&str, fn())] = &[];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    if flag("--list") {
        if !flag("--ignored") {
            for (name, _) in TESTS {
                println!("{name}: test");
            }
        }
        return ExitCode::SUCCESS;
    }
    let filter = args.iter().find(|arg| !arg.starts_with('-'));
    let exact = flag("--exact");
    let chosen = TESTS.iter().filter(|(name, _)| match filter {
        None => true,
        Some(filter) if exact => name == filter,
        Some(filter) => name.contains(filter.as_str()),
    });
    let mut ran = 0;
    for (name, test) in chosen {
        test();
        println!("test {name} ... ok");
        ran += 1;
    }
    println!("test result: ok. {ran} passed");
    ExitCode::SUCCESS
}
