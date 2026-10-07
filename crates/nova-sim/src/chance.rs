//! The randomness port: whether a percentage chance fires, and which of
//! so many equal outcomes a roll gives.
//!
//! The rules that roll dice (each day's `öops` events, see
//! [`market`](crate::market)) ask a [`Chance`] the caller supplies, so they
//! stay deterministic under test. The real source lives at the program's
//! edge.

/// Whether a chance of so many percent fires, and which outcome a roll
/// gives, once per question.
pub trait Chance {
    /// Whether a `percent` % chance (0 to 100) fires this time.
    fn fires(&mut self, percent: u8) -> bool;
    /// Which of `sides` equal outcomes, from 0 to `sides - 1`, a roll gives
    /// this time: the original's `_Rand(sides)`. 0 when `sides` is 0.
    fn roll(&mut self, sides: u16) -> u16;
}

/// A borrowed source is a source.
impl<T: Chance + ?Sized> Chance for &mut T {
    fn fires(&mut self, percent: u8) -> bool {
        (**self).fires(percent)
    }

    fn roll(&mut self, sides: u16) -> u16 {
        (**self).roll(sides)
    }
}

/// A source that never fires and always rolls the first outcome: nothing
/// random ever happens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NeverFires;

impl Chance for NeverFires {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fires on every question.
    struct Always;

    impl Chance for Always {
        fn fires(&mut self, _percent: u8) -> bool {
            true
        }

        /// The last of the outcomes.
        fn roll(&mut self, sides: u16) -> u16 {
            sides.saturating_sub(1)
        }
    }

    fn ask(mut chance: impl Chance, percent: u8) -> bool {
        chance.fires(percent)
    }

    #[test]
    fn never_fires_never_fires() {
        for percent in [0, 1, 50, 100, 255] {
            assert!(!ask(NeverFires, percent), "{percent}");
        }
    }

    #[test]
    fn a_borrowed_source_answers_as_the_source() {
        assert!(ask(&mut Always, 0));
        assert!(!ask(&mut NeverFires, 100));
        let mut boxed: Box<dyn Chance> = Box::new(Always);
        assert!(ask(&mut *boxed, 10));
    }

    fn roll(mut chance: impl Chance, sides: u16) -> u16 {
        chance.roll(sides)
    }

    #[test]
    fn never_fires_rolls_the_first_outcome() {
        for sides in [0, 1, 2, 8, 360, u16::MAX] {
            assert_eq!(roll(NeverFires, sides), 0, "{sides}");
        }
    }

    #[test]
    fn a_borrowed_source_rolls_as_the_source() {
        assert_eq!(roll(&mut Always, 8), 7);
        assert_eq!(roll(&mut NeverFires, 8), 0);
        let mut boxed: Box<dyn Chance> = Box::new(Always);
        assert_eq!(roll(&mut *boxed, 360), 359);
    }
}
