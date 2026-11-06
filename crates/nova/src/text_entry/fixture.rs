//! The [`TextEntry`] port's fake and its contract, for this crate's tests
//! and its integration tests (the `fixture` feature).

use std::cell::RefCell;
use std::rc::Rc;

use super::TextEntry;
use crate::app::PxRect;

/// A call the app made on a [`FakeTextEntry`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryCall {
    /// Entry started at this rect.
    Start(PxRect),
    /// Entry stopped.
    Stop,
}

/// A scripted [`TextEntry`]: it answers `soft_keyboard` as built, and
/// records each start and stop. Its clones share the record, so a test
/// that hands one to the app keeps another to read.
#[derive(Clone, Debug, Default)]
pub struct FakeTextEntry {
    soft_keyboard: bool,
    calls: Rc<RefCell<Vec<EntryCall>>>,
}

impl FakeTextEntry {
    /// A fake with no soft keyboard and nothing recorded.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fake whose platform has a soft keyboard.
    #[must_use]
    pub fn with_soft_keyboard() -> Self {
        Self {
            soft_keyboard: true,
            ..Self::default()
        }
    }

    /// Every call made so far, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<EntryCall> {
        self.calls.borrow().clone()
    }

    /// Every call made so far, in order, forgetting them.
    #[must_use]
    pub fn take_calls(&self) -> Vec<EntryCall> {
        std::mem::take(&mut *self.calls.borrow_mut())
    }
}

impl TextEntry for FakeTextEntry {
    fn soft_keyboard(&self) -> bool {
        self.soft_keyboard
    }

    fn start(&mut self, field: PxRect) {
        self.calls.borrow_mut().push(EntryCall::Start(field));
    }

    fn stop(&mut self) {
        self.calls.borrow_mut().push(EntryCall::Stop);
    }
}

/// Checks `entry` against the port's contract by a fixed script: start,
/// start again at another rect, stop, stop again, start at a rect of no
/// size, start at a rect partly off the window, and stop.
///
/// - `soft_keyboard` gives the same answer before, during and after;
/// - nothing panics. An adapter tests itself over a fake platform that
///   refuses what the real one refuses, so this means it asks nothing the
///   platform refuses.
///
/// # Errors
///
/// The first broken clause, described.
pub fn check_contract(entry: &mut dyn TextEntry) -> Result<(), String> {
    let rect = |x, y, width, height| PxRect {
        x,
        y,
        width,
        height,
    };
    let before = entry.soft_keyboard();
    let script: [Option<PxRect>; 7] = [
        Some(rect(100, 200, 300, 32)),
        Some(rect(120, 260, 200, 32)),
        None,
        None,
        Some(rect(50, 50, 0, 0)),
        Some(rect(3000, 3000, 400, 40)),
        None,
    ];
    for (step, call) in script.into_iter().enumerate() {
        match call {
            Some(field) => entry.start(field),
            None => entry.stop(),
        }
        if entry.soft_keyboard() != before {
            return Err(format!(
                "soft_keyboard changed from {before} after step {step} ({call:?})"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fake_records_starts_and_stops_in_order() {
        let fake = FakeTextEntry::new();
        let mut given = fake.clone();
        let at = PxRect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };
        given.start(at);
        given.stop();
        assert_eq!(fake.calls(), [EntryCall::Start(at), EntryCall::Stop]);
        assert_eq!(fake.take_calls().len(), 2);
        assert_eq!(fake.calls(), [], "taken");
        assert!(!fake.soft_keyboard());
        assert!(FakeTextEntry::with_soft_keyboard().soft_keyboard());
    }

    #[test]
    fn the_fake_meets_the_contract() {
        assert_eq!(check_contract(&mut FakeTextEntry::new()), Ok(()));
        assert_eq!(
            check_contract(&mut FakeTextEntry::with_soft_keyboard()),
            Ok(())
        );
    }

    /// A platform whose answer flips on each start.
    struct Flipping(bool);

    impl TextEntry for Flipping {
        fn soft_keyboard(&self) -> bool {
            self.0
        }

        fn start(&mut self, _field: PxRect) {
            self.0 = !self.0;
        }

        fn stop(&mut self) {}
    }

    #[test]
    fn a_fake_whose_soft_keyboard_flips_fails_the_contract() {
        let refused = check_contract(&mut Flipping(false)).expect_err("refused");
        assert!(refused.contains("soft_keyboard changed"), "{refused}");
    }
}
