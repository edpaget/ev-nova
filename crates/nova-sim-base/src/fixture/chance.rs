//! The [`Chance`] port's contract.
//!
//! [`check_contract`] states what every source of randomness must answer,
//! whatever it draws: each answer in range, a roll of no sides 0, and a 0%
//! chance never firing. It says nothing about how draws are spread or in
//! what order they come, which the engine doesn't match to the original.
//!
//! It runs against [`NeverFires`](crate::chance::NeverFires) and this
//! module's mocks ([`Scripted`](super::Scripted), [`Draws`](super::Draws),
//! [`Ranged`](super::Ranged)) here; against `nova`'s real `SplitMix`; and
//! against nova-view's `StepRolls` and `SharedChance`. nova and nova-view
//! reach it as `nova_sim::fixture::chance`.

use crate::chance::Chance;

/// How many times each question is asked.
pub const CALLS: usize = 256;

/// The draw ranges the contract asks about.
pub const RANGES: [u32; 6] = [1, 2, 7, 100, 500, u32::MAX];

/// The roll sizes the contract asks about.
pub const SIDES: [u16; 5] = [1, 2, 6, 100, u16::MAX];

/// Checks the port's contract against `chance`, asking each question
/// [`CALLS`] times:
///
/// 1. Every `below(n)` is less than `n`, for `n` in [`RANGES`].
/// 2. `roll(0)` is 0, and every `roll(sides)` is less than `sides`, for
///    `sides` in [`SIDES`].
/// 3. `fires(0)` is false.
///
/// It doesn't ask `fires(100)`: [`NeverFires`](crate::chance::NeverFires),
/// the source under which nothing random happens, never fires.
///
/// # Errors
///
/// A message naming the first clause `chance` breaks.
pub fn check_contract(chance: &mut impl Chance) -> Result<(), String> {
    for n in RANGES {
        for _ in 0..CALLS {
            let draw = chance.below(n);
            if draw >= n {
                return Err(format!("below({n}) drew {draw}, not less than {n}"));
            }
        }
    }
    for _ in 0..CALLS {
        let roll = chance.roll(0);
        if roll != 0 {
            return Err(format!("roll(0) gave {roll}, not 0"));
        }
    }
    for sides in SIDES {
        for _ in 0..CALLS {
            let roll = chance.roll(sides);
            if roll >= sides {
                return Err(format!("roll({sides}) gave {roll}, not less than {sides}"));
            }
        }
    }
    for _ in 0..CALLS {
        if chance.fires(0) {
            return Err("a 0% chance fired".to_owned());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::fixture::{Draws, Ranged, Scripted};

    #[test]
    fn never_fires_meets_the_contract() {
        assert_eq!(check_contract(&mut NeverFires), Ok(()));
    }

    #[test]
    fn the_mocks_meet_the_contract() {
        assert_eq!(check_contract(&mut Scripted::default()), Ok(()));
        assert_eq!(check_contract(&mut Draws::default()), Ok(()));
        assert_eq!(check_contract(&mut Ranged::of(&[])), Ok(()));
    }

    /// [`NeverFires`] with one answer changed, on the `nth` time it is
    /// asked that question.
    #[derive(Default)]
    struct Broken {
        below: Option<(u32, u32)>,
        roll: Option<(u16, u16)>,
        fires_at_zero: bool,
        asked: usize,
    }

    impl Chance for Broken {
        fn fires(&mut self, percent: u8) -> bool {
            self.fires_at_zero && percent == 0
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked += 1;
            match self.below {
                Some((range, draw)) if range == n && self.asked > CALLS / 2 => draw,
                _ => NeverFires.below(n),
            }
        }

        fn roll(&mut self, sides: u16) -> u16 {
            match self.roll {
                Some((size, roll)) if size == sides => roll,
                _ => NeverFires.roll(sides),
            }
        }
    }

    fn broken(chance: Broken) -> String {
        let mut chance = chance;
        check_contract(&mut chance).expect_err("the broken source fails the contract")
    }

    #[test]
    fn rejects_a_draw_out_of_range() {
        for n in RANGES {
            let error = broken(Broken {
                below: Some((n, n)),
                ..Broken::default()
            });
            assert!(error.contains("below("), "{n}: {error}");
        }
    }

    #[test]
    fn rejects_a_roll_of_no_sides_that_is_not_zero() {
        let error = broken(Broken {
            roll: Some((0, 1)),
            ..Broken::default()
        });
        assert!(error.contains("roll(0)"), "{error}");
    }

    #[test]
    fn rejects_a_roll_out_of_range() {
        for sides in SIDES {
            let error = broken(Broken {
                roll: Some((sides, sides)),
                ..Broken::default()
            });
            assert!(error.contains("not less than"), "{sides}: {error}");
        }
    }

    #[test]
    fn rejects_a_zero_chance_that_fires() {
        let error = broken(Broken {
            fires_at_zero: true,
            ..Broken::default()
        });
        assert!(error.contains("0% chance"), "{error}");
    }
}
