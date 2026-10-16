//! Nova's clock: time counted in ticks of 1/30 s, and in the Mac's
//! ticks of 1/60 s ([`sixtieths`]).

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

/// One Mac tick: 1/60 s, `TickCount()`'s unit, truncated to whole
/// nanoseconds (16,666,666 ns), as [`FRAME_PERIOD_NANOS`] truncates 1/30 s.
pub(crate) const MAC_TICK_NANOS: u128 = 1_000_000_000 / 60;

/// How many whole Mac ticks fit in `elapsed`, rounded down. Truncating
/// the period makes `nova_sim::mission::screens::mac_ticks(n)` count as
/// exactly n, and one tick of 1/30 s as exactly 2.
pub(crate) fn sixtieths(elapsed: Duration) -> u128 {
    elapsed.as_nanos() / MAC_TICK_NANOS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixtieths_count_whole_mac_ticks_rounding_down() {
        use nova_sim::mission::screens::mac_ticks;
        assert_eq!(MAC_TICK_NANOS, 16_666_666);
        for n in [0, 1, 2, 118, 119, 150, 500, 501, 100_000] {
            assert_eq!(sixtieths(mac_ticks(n)), u128::from(n), "{n}");
        }
        assert_eq!(sixtieths(Duration::from_secs(1) / 30), 2);
        assert_eq!(sixtieths(Duration::from_nanos(16_666_665)), 0);
        assert_eq!(sixtieths(Duration::from_nanos(16_666_666)), 1);
    }

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
