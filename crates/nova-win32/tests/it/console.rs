//! The console adapter against the facts nova's `StdConsole` mock
//! assumes, under a test runner: the test process is a console program
//! whose stderr the runner captures, so the handle is there, and there is
//! no parent console left to attach to.

use nova_win32::{StdHandle, attach_parent_console, stderr_handle};

#[test]
fn under_a_test_runner_stderr_is_present() {
    assert_eq!(stderr_handle(), StdHandle::Present);
}

#[test]
fn a_process_with_a_console_attaches_to_no_other() {
    assert!(!attach_parent_console());
    assert_eq!(stderr_handle(), StdHandle::Present, "stderr is untouched");
}
