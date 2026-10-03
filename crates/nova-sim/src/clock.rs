//! The fixed-step clock: the simulation advances in whole ticks of 1/30 s,
//! Nova's own rate, whatever the display's frame rate.
//!
//! [`FixedStep`] adds each frame's time to what is left over from the frames
//! before and says how many whole ticks to run, carrying the remainder. The
//! steps a run of frames adds up to depend only on the total time, never on
//! how it was cut into frames, so the simulation gives the same results at
//! any frame rate. The remainder, as a fraction of a tick, is how far the
//! display is between the last step and the next, for interpolation.

use std::time::Duration;

/// How many simulation steps run in a second: the original's rate.
pub const TICKS_PER_SECOND: u32 = 30;

/// One step: 1/30 s truncated to whole nanoseconds (33,333,333 ns), the
/// same period as the views' animation clock.
pub const TICK: Duration = Duration::from_nanos(1_000_000_000 / TICKS_PER_SECOND as u64);

/// The most steps one [`FixedStep::advance`] runs: a second's worth. Time
/// beyond that (a stall, or the app paused in a debugger) is dropped, so
/// the simulation never falls further and further behind trying to catch
/// up.
pub const MAX_STEPS: u32 = TICKS_PER_SECOND;

/// What one frame's time comes to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Steps {
    /// How many whole steps to run now.
    pub steps: u32,
    /// How far into the next step the time left over reaches, as a fraction
    /// of a tick in `[0, 1)`.
    pub alpha: f32,
}

/// Turns frame times into whole steps, carrying the remainder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixedStep {
    /// Time not yet stepped: always less than a tick.
    carry: Duration,
}

impl FixedStep {
    /// A clock with nothing carried.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a frame of `dt` and returns how many steps to run.
    pub fn advance(&mut self, dt: Duration) -> Steps {
        let total = self.carry.saturating_add(dt).as_nanos();
        let tick = TICK.as_nanos();
        let whole = total / tick;
        let left = total % tick;
        self.carry = Duration::from_nanos(left as u64);
        // In f64, then to f32: even a nanosecond short of a tick (1 - 3e-8)
        // rounds down to the f32 below 1, never to 1.
        let alpha = (left as f64 / tick as f64) as f32;
        Steps {
            steps: whole.min(u128::from(MAX_STEPS)) as u32,
            alpha,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const TICK_NANOS: u64 = 33_333_333;

    fn nanos(n: u64) -> Duration {
        Duration::from_nanos(n)
    }

    #[test]
    fn a_tick_is_a_thirtieth_of_a_second_truncated_to_nanoseconds() {
        assert_eq!(TICK, Duration::from_nanos(TICK_NANOS));
        assert_eq!(TICK, Duration::from_secs(1) / TICKS_PER_SECOND);
        assert_eq!(TICKS_PER_SECOND, 30);
    }

    #[test]
    fn whole_ticks_step_and_the_remainder_carries_over() {
        let mut clock = FixedStep::new();
        assert_eq!(clock.advance(Duration::ZERO).steps, 0);
        assert_eq!(clock.advance(TICK).steps, 1);
        assert_eq!(clock.advance(TICK * 3).steps, 3);
        // Two halves make a whole.
        assert_eq!(clock.advance(nanos(TICK_NANOS / 2)).steps, 0);
        assert_eq!(clock.advance(nanos(TICK_NANOS - TICK_NANOS / 2)).steps, 1);
        // A tick and a bit, then the bit's complement and a tick.
        assert_eq!(clock.advance(nanos(TICK_NANOS + 10)).steps, 1);
        assert_eq!(clock.advance(nanos(TICK_NANOS - 10)).steps, 1);
        assert_eq!(clock.advance(nanos(TICK_NANOS - 1)).steps, 0);
        assert_eq!(clock.advance(nanos(1)).steps, 1);
    }

    #[test]
    fn alpha_is_the_carried_fraction_of_a_tick() {
        let mut clock = FixedStep::new();
        assert_eq!(clock.advance(Duration::ZERO).alpha, 0.0);
        assert_eq!(clock.advance(TICK).alpha, 0.0);
        let quarter = clock.advance(nanos(TICK_NANOS / 4));
        assert!((quarter.alpha - 0.25).abs() < 1e-6, "{quarter:?}");
        let three_quarters = clock.advance(nanos(TICK_NANOS / 2));
        assert!(
            (three_quarters.alpha - 0.75).abs() < 1e-6,
            "{three_quarters:?}"
        );
        assert_eq!(three_quarters.steps, 0);
        let past = clock.advance(nanos(TICK_NANOS / 2));
        assert_eq!(past.steps, 1);
        assert!((past.alpha - 0.25).abs() < 1e-6, "{past:?}");
    }

    #[test]
    fn alpha_stays_below_1_a_nanosecond_short_of_a_tick() {
        let mut clock = FixedStep::new();
        let almost = clock.advance(nanos(TICK_NANOS - 1));
        assert_eq!(almost.steps, 0);
        assert_eq!(almost.alpha, f32::from_bits(1.0_f32.to_bits() - 1));
    }

    #[test]
    fn a_long_stall_runs_at_most_a_seconds_steps_and_drops_the_rest() {
        assert_eq!(MAX_STEPS, 30);
        let mut clock = FixedStep::new();
        // Five seconds of ticks and half a tick.
        let stall = clock.advance(TICK * 150 + nanos(TICK_NANOS / 2));
        assert_eq!(stall.steps, MAX_STEPS);
        assert!((stall.alpha - 0.5).abs() < 1e-6, "{stall:?}");
        // The half tick is still carried; the dropped seconds are not.
        assert_eq!(clock.advance(nanos(TICK_NANOS - TICK_NANOS / 2)).steps, 1);
        // Exactly the cap is not cut.
        let mut clock = FixedStep::new();
        assert_eq!(clock.advance(TICK * MAX_STEPS).steps, MAX_STEPS);
        assert_eq!(clock.advance(TICK * (MAX_STEPS + 1)).steps, MAX_STEPS);
        assert_eq!(
            clock.advance(nanos(TICK_NANOS - 1)).steps,
            0,
            "nothing carried"
        );
    }

    /// The steps run by `halves` half-seconds of frames at `fps`, each
    /// redraw at `n / fps` seconds exactly (to the nanosecond), at every
    /// half-second boundary.
    fn steps_at_half_seconds(fps: u64, halves: u64) -> Vec<u32> {
        let mut clock = FixedStep::new();
        let mut last = 0;
        let mut total = 0;
        let mut at_boundaries = Vec::new();
        for frame in 1..=fps * halves / 2 {
            let now = frame * 1_000_000_000 / fps;
            total += clock.advance(nanos(now - last)).steps;
            last = now;
            if (frame * 2) % fps == 0 {
                at_boundaries.push(total);
            }
        }
        at_boundaries
    }

    #[test]
    fn the_same_time_gives_the_same_steps_at_30_60_and_120_fps() {
        let at_30 = steps_at_half_seconds(30, 20);
        assert_eq!(at_30.len(), 20);
        assert_eq!(at_30[..4], [15, 30, 45, 60]);
        assert_eq!(steps_at_half_seconds(60, 20), at_30);
        assert_eq!(steps_at_half_seconds(120, 20), at_30);
    }
}
