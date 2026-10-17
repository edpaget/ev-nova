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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_answers_in_order_then_no_recording_each_percent() {
        let mut chance = Scripted::answering(&[true, false, true]);
        let answers = [10, 20, 30, 40].map(|percent| chance.fires(percent));
        assert_eq!(answers, [true, false, true, false]);
        assert_eq!(chance.asked, [10, 20, 30, 40]);
    }

    #[test]
    fn a_script_rolls_in_order_then_0_recording_each_number_of_sides() {
        let mut chance = Scripted::rolling(&[3, 5]);
        let rolls = [6, 8, 10].map(|sides| chance.roll(sides));
        assert_eq!(rolls, [3, 5, 0]);
        assert_eq!(chance.sides_asked, [6, 8, 10]);
    }

    #[test]
    fn an_answering_script_can_roll_too() {
        let mut chance = Scripted::answering(&[true]).and_rolling(&[4, 2]);
        assert!(chance.fires(50));
        assert_eq!(chance.roll(6), 4);
        assert_eq!(chance.roll(6), 2);
        assert!(!chance.fires(50));
    }

    #[test]
    fn a_script_draws_the_last_outcome() {
        let mut chance = Scripted::default();
        assert_eq!(chance.below(10), 9);
        assert_eq!(chance.below(1), 0);
    }
}
