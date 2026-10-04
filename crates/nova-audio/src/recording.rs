//! A recording [`Audio`] mock, for tests: it plays nothing and keeps every
//! command it is given.
//!
//! Available to this crate's tests and, through the `recording` feature,
//! to other crates' tests. The log is shared, so a test keeps a handle to
//! it after moving the mock into the app.

use std::cell::RefCell;
use std::rc::Rc;

use crate::port::{Audio, AudioCommand};

/// The commands an [`Audio`] was given, in order.
pub type AudioLog = Rc<RefCell<Vec<AudioCommand>>>;

/// Records every command; plays nothing.
#[derive(Clone, Debug, Default)]
pub struct RecordingAudio {
    log: AudioLog,
}

impl RecordingAudio {
    /// A mock with an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A handle to the log, which sees every command given after it too.
    #[must_use]
    pub fn log(&self) -> AudioLog {
        Rc::clone(&self.log)
    }
}

impl Audio for RecordingAudio {
    fn run(&mut self, command: AudioCommand) {
        self.log.borrow_mut().push(command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_records_every_command_in_order_for_every_handle() {
        let mut audio = RecordingAudio::new();
        let before = audio.log();
        audio.run(AudioCommand::StopEffects);
        let after = audio.log();
        audio.run(AudioCommand::StopMusic);
        let expected = [AudioCommand::StopEffects, AudioCommand::StopMusic];
        assert_eq!(*before.borrow(), expected);
        assert_eq!(*after.borrow(), expected);
    }
}
