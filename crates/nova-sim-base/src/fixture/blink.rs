//! A [`BlinkChance`] mock for the ship lights' tests.

use std::cell::RefCell;

use crate::blink::BlinkChance;

/// A [`BlinkChance`] that answers each roll by its change: change `k`
/// gets the script's `k`-th answer, the last answer repeating past the
/// script's end (0 for an empty script). It records every
/// `(change, sides)` it is asked, in order.
#[derive(Debug, Default)]
pub struct ScriptedRolls {
    answers: Vec<u16>,
    asked: RefCell<Vec<(u64, u16)>>,
}

impl ScriptedRolls {
    /// Answers change `k` with `answers[k]`, the last repeating.
    #[must_use]
    pub fn new(answers: &[u16]) -> Self {
        Self {
            answers: answers.to_vec(),
            asked: RefCell::default(),
        }
    }

    /// The answer for `change`, without recording a roll.
    #[must_use]
    pub fn answer(&self, change: u64) -> u16 {
        let at = usize::try_from(change).unwrap_or(usize::MAX);
        self.answers
            .get(at)
            .or(self.answers.last())
            .copied()
            .unwrap_or(0)
    }

    /// Every `(change, sides)` rolled so far, in order.
    #[must_use]
    pub fn asked(&self) -> Vec<(u64, u16)> {
        self.asked.borrow().clone()
    }
}

impl BlinkChance for ScriptedRolls {
    fn roll(&self, change: u64, sides: u16) -> u16 {
        self.asked.borrow_mut().push((change, sides));
        self.answer(change)
    }
}
