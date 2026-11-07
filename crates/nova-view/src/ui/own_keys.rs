//! A component's own reader of its commands, for the keys sent straight
//! to it.
//!
//! A widget inside a screen (a scrolling text, a check box, a picker)
//! takes its keys two ways. Its host, which reads every input, resolves
//! a key itself and hands the widget the command (each widget's
//! `command`), so the widget never sees a stale modifier state from the
//! inputs the host kept. A caller that drives the widget alone sends it
//! inputs (its `input`), which it reads through this reader. The reader
//! is no part of what the widget shows, so it compares equal to any
//! other, and the widgets keep their `PartialEq`.

use std::ops::{Deref, DerefMut};

use crate::keymap::{CommandReader, ScreenContexts};

/// A component's own [`CommandReader`] (see the module docs), boxed, as
/// most components' keys come through their host instead.
#[derive(Clone, Debug)]
pub(crate) struct OwnKeys(Box<CommandReader>);

impl OwnKeys {
    /// A reader of `screen`'s contexts over the default bindings.
    pub(crate) fn new(screen: ScreenContexts) -> Self {
        Self(Box::new(CommandReader::new(screen)))
    }
}

impl PartialEq for OwnKeys {
    /// Always: the keys a component has seen are no part of its value.
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Deref for OwnKeys {
    type Target = CommandReader;

    fn deref(&self) -> &CommandReader {
        &self.0
    }
}

impl DerefMut for OwnKeys {
    fn deref_mut(&mut self) -> &mut CommandReader {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Input, Key};
    use crate::keymap::contexts;

    #[test]
    fn two_readers_compare_equal_whatever_they_have_seen() {
        let mut seen = OwnKeys::new(contexts::DIALOG);
        seen.follow(&Input::Key {
            key: Key::ArrowDown,
            pressed: true,
            repeat: false,
        });
        assert!(seen.anything_down());
        assert_eq!(seen, OwnKeys::new(contexts::DIALOG));
    }
}
