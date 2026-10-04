//! The randomness port: whether a percentage chance fires, and uniform
//! draws.
//!
//! The rules that roll dice (each day's `öops` events, see
//! [`market`](crate::market), and the NPC traffic, see
//! [`traffic`](crate::traffic)) ask a [`Chance`] the caller supplies, so
//! they stay deterministic under test. The real source lives at the
//! program's edge.
//!
//! Every roll on a draw is written as "`below(n) < k` fires": the
//! original's `Rand(500)` landing on 0 or 1 is `below(500) < 2`, and its
//! `Rand(7) == 0` is `below(7) < 1`. So a source whose draws are always
//! the last outcome, `n - 1`, never fires a roll.

/// Whether a chance of so many percent fires, once per question, and
/// uniform draws.
pub trait Chance {
    /// Whether a `percent` % chance (0 to 100) fires this time.
    fn fires(&mut self, percent: u8) -> bool;
    /// A uniform draw in `0..n`. Callers never pass 0.
    fn below(&mut self, n: u32) -> u32;
}

/// A borrowed source is a source.
impl<T: Chance + ?Sized> Chance for &mut T {
    fn fires(&mut self, percent: u8) -> bool {
        (**self).fires(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        (**self).below(n)
    }
}

/// A source that never fires: nothing random ever happens. Each draw is
/// the last outcome, `n - 1`, so no roll fires, and a weighted pick takes
/// the last entry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NeverFires;

impl Chance for NeverFires {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fires on every question, and every draw is 0.
    struct Always;

    impl Chance for Always {
        fn fires(&mut self, _percent: u8) -> bool {
            true
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
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

    fn draw(mut chance: impl Chance, n: u32) -> u32 {
        chance.below(n)
    }

    #[test]
    fn never_fires_draws_the_last_outcome() {
        for (n, last) in [(1, 0), (2, 1), (7, 6), (500, 499), (u32::MAX, u32::MAX - 1)] {
            assert_eq!(draw(NeverFires, n), last, "{n}");
        }
        assert_eq!(draw(NeverFires, 0), 0, "never asked, but never wraps");
    }

    #[test]
    fn a_borrowed_or_boxed_source_draws_as_the_source() {
        assert_eq!(draw(&mut Always, 7), 0);
        assert_eq!(draw(&mut NeverFires, 7), 6);
        let mut boxed: Box<dyn Chance> = Box::new(NeverFires);
        assert_eq!(draw(&mut *boxed, 300), 299);
        let mut always: Box<dyn Chance> = Box::new(Always);
        assert_eq!(draw(&mut *always, 300), 0);
    }
}
