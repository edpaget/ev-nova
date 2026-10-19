//! Test support for the nova-sim crates' tests, behind the `fixture`
//! feature: [`Chance`] mocks and a ship class's fields. [`Scripted`]
//! follows a script, [`Draws`] draws from a queue, and [`Ranged`] draws by
//! the range asked. [`FAST`] is a fast, light ship class.

use std::collections::VecDeque;

use crate::chance::{Chance, NeverFires};
use crate::handling::ShipFields;

/// A fast, light ship class: `Speed` 600, `Accel` 900, `Maneuver` 30,
/// shield 30, armour 45, fuel 300 (no regeneration), 20 tons of hold,
/// mass 40 with 30 free, `Contribute` bit 0x1 and no recharge or
/// `Flags2`.
pub const FAST: ShipFields = ShipFields {
    speed: 600,
    accel: 900,
    maneuver: 30,
    shield: 30,
    armor: 45,
    fuel: 300,
    fuel_regen: 0,
    holds: 20,
    mass: 40,
    free_mass: 30,
    contribute: 0x1,
    shield_rech: 0,
    armor_rech: 0,
    flags: 0,
    flags2: 0,
    ionize_max: 0,
    deionize: 0,
};

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

/// A [`Chance`] whose draws come from a queue (the last outcome, `n - 1`,
/// once it runs out, so no roll fires) and that records each `n` it is
/// asked. It never fires a percentage.
#[derive(Debug, Default)]
pub struct Draws {
    queue: VecDeque<u32>,
    /// Each `n` it was asked to draw below, in order.
    pub asked: Vec<u32>,
}

impl Draws {
    /// Draws `draws`, in order, then the last outcome.
    #[must_use]
    pub fn of(draws: &[u32]) -> Self {
        Self {
            queue: draws.iter().copied().collect(),
            asked: Vec::new(),
        }
    }
}

impl Chance for Draws {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    /// The next draw queued; one outside `0..n` is a mistake in the test.
    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        match self.queue.pop_front() {
            Some(draw) => {
                assert!(draw < n, "drew {draw} below {n}");
                draw
            }
            None => n.saturating_sub(1),
        }
    }
}

/// A chance keyed by range, not by the order of its draws: each
/// `below(n)` gives the outcome given for `n`, or `n - 1` for a range
/// given none, and each range asked is kept in `asked`. Every `fires` is
/// false.
pub struct Ranged {
    outcomes: Vec<(u32, u32)>,
    /// Each range it was asked to draw below, in order.
    pub asked: Vec<u32>,
}

impl Ranged {
    /// `outcomes`, each a range and what a draw below it gives.
    #[must_use]
    pub fn of(outcomes: &[(u32, u32)]) -> Self {
        Self {
            outcomes: outcomes.to_vec(),
            asked: Vec::new(),
        }
    }

    /// Whether a draw below `n` was asked.
    #[must_use]
    pub fn was_asked(&self, n: u32) -> bool {
        self.asked.contains(&n)
    }
}

impl Chance for Ranged {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    /// The outcome given for `n`; one outside `0..n` is a mistake in the
    /// test.
    fn below(&mut self, n: u32) -> u32 {
        self.asked.push(n);
        let outcome = self
            .outcomes
            .iter()
            .find(|&&(range, _)| range == n)
            .map_or(n.saturating_sub(1), |&(_, outcome)| outcome);
        assert!(outcome < n, "drew {outcome} below {n}");
        outcome
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

    #[test]
    fn draws_come_in_queue_order_then_the_last_outcome() {
        let mut chance = Draws::of(&[3, 0, 7]);
        let draws = [10, 5, 8, 6, 1].map(|n| chance.below(n));
        assert_eq!(draws, [3, 0, 7, 5, 0]);
    }

    #[test]
    fn draws_record_each_n_asked_in_order() {
        let mut chance = Draws::of(&[2]);
        chance.below(4);
        chance.below(9);
        chance.below(9);
        assert_eq!(chance.asked, [4, 9, 9]);
    }

    #[test]
    fn draws_never_fire() {
        let mut chance = Draws::of(&[1]);
        assert!(!chance.fires(100));
        assert_eq!(chance.below(2), 1, "a fire spends no draw");
    }

    #[test]
    #[should_panic(expected = "drew 5 below 5")]
    fn a_queued_draw_outside_its_range_is_a_mistake() {
        Draws::of(&[5]).below(5);
    }

    #[test]
    fn a_range_gives_its_outcome_else_the_last_recording_each_range() {
        let mut chance = Ranged::of(&[(10, 3), (4, 0)]);
        let draws = [10, 6, 4, 10].map(|n| chance.below(n));
        assert_eq!(draws, [3, 5, 0, 3]);
        assert_eq!(chance.asked, [10, 6, 4, 10]);
        assert!(chance.was_asked(6));
        assert!(!chance.was_asked(7));
    }

    #[test]
    fn a_ranged_chance_never_fires() {
        assert!(!Ranged::of(&[]).fires(100));
    }

    #[test]
    #[should_panic(expected = "drew 4 below 3")]
    fn a_ranged_outcome_outside_its_range_is_a_mistake() {
        Ranged::of(&[(3, 4)]).below(3);
    }
}
