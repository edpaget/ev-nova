//! The randomness port: whether a percentage chance fires.
//!
//! The rules that roll dice (each day's `öops` events, see
//! [`market`](crate::market)) ask a [`Chance`] the caller supplies, so
//! they stay deterministic under test. The real source lives at the
//! program's edge.

/// Whether a chance of so many percent fires, once per question.
pub trait Chance {
    /// Whether a `percent` % chance (0 to 100) fires this time.
    fn fires(&mut self, percent: u8) -> bool;
}

/// A borrowed source is a source.
impl<T: Chance + ?Sized> Chance for &mut T {
    fn fires(&mut self, percent: u8) -> bool {
        (**self).fires(percent)
    }
}

/// A source that never fires: nothing random ever happens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NeverFires;

impl Chance for NeverFires {
    fn fires(&mut self, _percent: u8) -> bool {
        false
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
}
