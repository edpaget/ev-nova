//! Nova's clock: time counted in ticks of 1/30 s.

use std::time::Duration;

/// One tick: 1/30 s, Nova's animation unit, truncated to whole nanoseconds
/// (33,333,333 ns), which is exactly what `Duration::from_secs(1) / 30`
/// gives. Every tick of exactly 1/30 s therefore counts as exactly one
/// tick, and shorter ones add up. The truncation runs the clock 10 ns a
/// second fast.
pub(crate) const FRAME_PERIOD_NANOS: u128 = 1_000_000_000 / 30;

/// How many whole ticks fit in `elapsed`, rounded down.
pub(crate) fn ticks(elapsed: Duration) -> u128 {
    elapsed.as_nanos() / FRAME_PERIOD_NANOS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tick_is_a_thirtieth_of_a_second_truncated_to_nanoseconds() {
        assert_eq!(FRAME_PERIOD_NANOS, 33_333_333);
        assert_eq!(FRAME_PERIOD_NANOS, (Duration::from_secs(1) / 30).as_nanos());
    }

    #[test]
    fn ticks_count_whole_periods_rounding_down() {
        let tick = Duration::from_secs(1) / 30;
        assert_eq!(ticks(Duration::ZERO), 0);
        assert_eq!(ticks(Duration::from_nanos(33_333_332)), 0);
        assert_eq!(ticks(tick), 1);
        assert_eq!(ticks(Duration::from_nanos(66_666_665)), 1);
        assert_eq!(ticks(Duration::from_nanos(66_666_666)), 2);
        assert_eq!(ticks(tick * 3001), 3001);
        assert_eq!(ticks(Duration::from_secs(1)), 30);
    }
}
