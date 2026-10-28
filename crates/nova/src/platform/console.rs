//! Where stderr goes on a Windows-subsystem program, decided before
//! anything is written to it ([`connect_stderr`]), over the
//! [`StdConsole`] port; the real port is `Win32Console`, on Windows only.
//!
//! `nova.exe` is built for the Windows subsystem, so Explorer opens no
//! console for it, and a process started that way has no standard
//! handles. Started from `cmd`, PowerShell or Windows Terminal (which
//! hosts both through `ConPTY`), it attaches to that terminal's console
//! (`AttachConsole(ATTACH_PARENT_PROCESS)`), so usage errors and warnings
//! print there. Rust's std looks the handle up on every write and every
//! `is_terminal()`, caching nothing, so after the attach `eprintln!` and
//! `io::stderr()` reach the console with no reopening. Neither `cmd` nor
//! PowerShell waits for a Windows-subsystem program, so the prompt comes
//! back at once and the game's lines print after it; Ctrl+C in that
//! terminal, or closing it, ends the game as it would any program attached
//! to it. A handle the process was given (`nova.exe 2> log.txt`, or a
//! launcher's pipe) is kept: it already has a reader.
//!
//! [`ConsoleTerminal`] gives the answer to the startup report. Unlike on
//! macOS, where a Finder-launched app's stderr is open but goes to
//! `/dev/null`, so only a terminal counts, here a file or a pipe counts
//! too: only stderr going nowhere shows the fatal error in a message box.

use crate::startup::Terminal;

/// The process's standard error handle, as the port sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stderr {
    /// There is none, or it writes nowhere.
    Missing,
    /// It writes to a console, a file or a pipe.
    Present,
}

/// The console calls the decision needs: the port.
pub trait StdConsole {
    /// The standard error handle now.
    fn stderr(&self) -> Stderr;
    /// Attaches to the console of the process that started this one;
    /// whether it did.
    fn attach_parent(&mut self) -> bool;
}

/// Where stderr goes once [`connect_stderr`] has run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StderrGoes {
    /// To the handle the process was given.
    Redirected,
    /// To the console of the terminal that started the game.
    Attached,
    /// Nowhere: no handle and no console to attach to.
    Nowhere,
}

/// Connects stderr: keeps a handle the process was given, and otherwise
/// attaches to the parent's console.
pub fn connect_stderr(console: &mut dyn StdConsole) -> StderrGoes {
    match console.stderr() {
        Stderr::Present => StderrGoes::Redirected,
        Stderr::Missing if console.attach_parent() => StderrGoes::Attached,
        Stderr::Missing => StderrGoes::Nowhere,
    }
}

/// A [`Terminal`] over where stderr goes: someone reads it unless it goes
/// nowhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsoleTerminal(pub StderrGoes);

impl Terminal for ConsoleTerminal {
    fn stderr_is_terminal(&self) -> bool {
        self.0 != StderrGoes::Nowhere
    }
}

/// The real [`StdConsole`]: the process's own, through nova-win32.
#[cfg(windows)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Win32Console;

#[cfg(windows)]
impl StdConsole for Win32Console {
    fn stderr(&self) -> Stderr {
        match nova_win32::stderr_handle() {
            nova_win32::StdHandle::Missing => Stderr::Missing,
            nova_win32::StdHandle::Present => Stderr::Present,
        }
    }

    fn attach_parent(&mut self) -> bool {
        nova_win32::attach_parent_console()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A console with a canned handle and attach result, which counts
    /// its attaches.
    struct Console {
        stderr: Stderr,
        attaches: bool,
        attached: usize,
    }

    impl Console {
        fn new(stderr: Stderr, attaches: bool) -> Self {
            Self {
                stderr,
                attaches,
                attached: 0,
            }
        }
    }

    impl StdConsole for Console {
        fn stderr(&self) -> Stderr {
            self.stderr
        }

        fn attach_parent(&mut self) -> bool {
            self.attached += 1;
            self.attaches
        }
    }

    #[test]
    fn a_handle_given_is_kept_and_nothing_is_attached() {
        for attaches in [true, false] {
            let mut console = Console::new(Stderr::Present, attaches);
            assert_eq!(connect_stderr(&mut console), StderrGoes::Redirected);
            assert_eq!(console.attached, 0);
        }
    }

    #[test]
    fn without_a_handle_the_parents_console_is_attached_once() {
        let mut console = Console::new(Stderr::Missing, true);
        assert_eq!(connect_stderr(&mut console), StderrGoes::Attached);
        assert_eq!(console.attached, 1);
    }

    #[test]
    fn without_a_handle_or_a_parent_console_stderr_goes_nowhere() {
        let mut console = Console::new(Stderr::Missing, false);
        assert_eq!(connect_stderr(&mut console), StderrGoes::Nowhere);
        assert_eq!(console.attached, 1);
    }

    #[test]
    fn only_nowhere_is_no_terminal() {
        assert!(ConsoleTerminal(StderrGoes::Redirected).stderr_is_terminal());
        assert!(ConsoleTerminal(StderrGoes::Attached).stderr_is_terminal());
        assert!(!ConsoleTerminal(StderrGoes::Nowhere).stderr_is_terminal());
    }
}
