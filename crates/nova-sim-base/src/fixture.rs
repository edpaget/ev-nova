//! Test support for the nova-sim crates' tests, behind the `fixture`
//! feature: a [`Chance`] that follows a script.

use crate::chance::{Chance, NeverFires};

/// A [`Chance`] that answers and rolls from a script (no, and 0, once it
/// runs out) and records each percent and each number of sides it is
/// asked.
#[derive(Debug, Default)]
pub struct Scripted {
    answers: Vec<bool>,
    rolls: Vec<u16>,
    /// Each percent it was asked to fire at, in order.
    pub asked: Vec<u8>,
    /// Each number of sides it was asked to roll, in order.
    pub sides_asked: Vec<u16>,
}

impl Scripted {
    /// Answers `answers`, in order, then no.
    #[must_use]
    pub fn answering(answers: &[bool]) -> Self {
        Self {
            answers: answers.iter().rev().copied().collect(),
            ..Self::default()
        }
    }

    /// Rolls `rolls`, in order, then 0.
    #[must_use]
    pub fn rolling(rolls: &[u16]) -> Self {
        Self {
            rolls: rolls.iter().rev().copied().collect(),
            ..Self::default()
        }
    }

    /// This script, rolling `rolls`, in order, then 0.
    #[must_use]
    pub fn and_rolling(self, rolls: &[u16]) -> Self {
        Self {
            rolls: rolls.iter().rev().copied().collect(),
            ..self
        }
    }
}

impl Chance for Scripted {
    fn fires(&mut self, percent: u8) -> bool {
        self.asked.push(percent);
        self.answers.pop().unwrap_or(false)
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.sides_asked.push(sides);
        self.rolls.pop().unwrap_or(0)
    }

    /// The last outcome, as [`NeverFires`] draws: no roll fires.
    fn below(&mut self, n: u32) -> u32 {
        NeverFires.below(n)
    }
}
