//! When the HUD refreshes, as `_DrawStatus` (@0x4c85d) times it.
//!
//! `_DrawStatus` counts in 60 Hz `TickCount` units (Mac ticks), not game
//! frames. The radar part runs when
//! `TickCount() >= _lastRadarUpdateTime + 15` (@0x4c911-0x4c923): it
//! records the time (@0x4c93d), flips the target's blink,
//! `_radarCount = (_radarCount + 1) % 2` (@0x4c93f-0x4c94f), and marks the
//! radar dirty (@0x4c9d5). The original
//! runs at 30 frames a second, about 2 Mac ticks a frame, so the test
//! passes on the 8th frame after the last refresh: [`RADAR_PERIOD_TICKS`].
//!
//! The nav area is redrawn when `TickCount() % 30 == 0`
//! (@0x4c899-0x4c8b6): at 2 Mac ticks a frame, on every 15th engine tick,
//! [`NAV_PERIOD_TICKS`]. Ours refreshes it on the first frame whose tick
//! passes a multiple of 15 since the last frame (`(last, now]` holds
//! one), not on `now % 15 == 0`: the original's fixed 2-tick frame lands
//! on every multiple, and the crossing test keeps that when our frames
//! skip ticks, without refreshing twice when a frame repeats a tick. The
//! original skips this redraw while the nav system is off (+0x6a == −1,
//! @0x4c892), which changes nothing seen: "Nav System Off" cannot change
//! without a selection change, which redraws the area at once.
//!
//! The target panel (`TickCount() % 45 == 0`, @0x4c8c2-0x4c8eb, with its
//! shield and type check inside the radar's block, @0x4c963-0x4c9b5) is
//! timed here by hud-fidelity phase 5.
//!
//! `_RedrawStatus` (@0x4cac0) redraws everything at once but leaves the
//! time and the blink as they are; the view does the same by taking a
//! fresh snapshot without advancing a [`HudCadence`].
//!
//! The reading of
//! [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence) says whether the
//! HUD's content waits for these refreshes (the engine's) or follows
//! every frame. The blink keeps this timing under both: it is a feature,
//! not a redraw artefact.

/// Engine ticks (1/30 s) between radar refreshes: 15 Mac ticks
/// (0.25 s) is 7.5 frames, and the `>=` test passes on the 8th (≈0.267 s).
pub const RADAR_PERIOD_TICKS: u64 = 8;

/// Engine ticks between nav area refreshes: `TickCount() % 30 == 0`
/// (@0x4c899-0x4c8b6) at 2 Mac ticks an engine tick, 0.5 s.
pub const NAV_PERIOD_TICKS: u64 = 15;

/// Which parts of the HUD a [`HudCadence::advance`] refreshes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudRefresh {
    /// The radar, which flips the blink.
    pub radar: bool,
    /// The nav area.
    pub nav: bool,
}

/// The HUD's refresh timer and the target's blink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudCadence {
    /// The engine tick of the last radar refresh, `None` before the first.
    last_radar: Option<u64>,
    /// The engine tick of the last advance, `None` before the first.
    last_tick: Option<u64>,
    /// Whether the target shows bright: `_radarCount` ≠ 0. Off at first,
    /// as `_InitStatus` (@0x49470) zeroes it.
    blink: bool,
}

impl HudCadence {
    /// Moves the timer to engine tick `now`, and says what refreshes. The
    /// radar: on the first call, then once [`RADAR_PERIOD_TICKS`] have
    /// passed since its last refresh, flipping the blink. The nav area: on
    /// the first call, then when a multiple of [`NAV_PERIOD_TICKS`] lies
    /// after the last call's tick and up to `now` (see the module docs).
    pub fn advance(&mut self, now: u64) -> HudRefresh {
        let radar = self
            .last_radar
            .is_none_or(|last| now >= last.saturating_add(RADAR_PERIOD_TICKS));
        if radar {
            self.last_radar = Some(now);
            self.blink = !self.blink;
        }
        let nav = self
            .last_tick
            .is_none_or(|last| now / NAV_PERIOD_TICKS > last / NAV_PERIOD_TICKS);
        self.last_tick = Some(now);
        HudRefresh { radar, nav }
    }

    /// Whether the target shows bright.
    #[must_use]
    pub fn blink_on(&self) -> bool {
        self.blink
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Advances a fresh cadence through `ticks`, giving each refresh and
    /// the blink after it.
    fn run(ticks: impl IntoIterator<Item = u64>) -> Vec<(u64, bool, bool)> {
        let mut cadence = HudCadence::default();
        ticks
            .into_iter()
            .map(|now| {
                let refreshed = cadence.advance(now).radar;
                (now, refreshed, cadence.blink_on())
            })
            .collect()
    }

    #[test]
    fn the_radar_refreshes_every_eight_ticks_flipping_the_blink() {
        assert_eq!(RADAR_PERIOD_TICKS, 8);
        assert!(!HudCadence::default().blink_on(), "off before the first");
        let seen = run(0..=17);
        let refreshes: Vec<_> = seen.iter().filter(|(_, r, _)| *r).collect();
        assert_eq!(
            refreshes,
            [&(0, true, true), &(8, true, false), &(16, true, true)]
        );
        for (now, _, blink) in seen {
            assert_eq!(blink, (now / 8) % 2 == 0, "tick {now}");
        }
    }

    #[test]
    fn the_period_counts_from_the_last_refresh_not_the_first() {
        // Frames that skip ticks: the next refresh is due 8 after 3, at 11.
        let seen = run([3, 10, 11, 18, 19]);
        assert_eq!(
            seen,
            [
                (3, true, true),
                (10, false, true),
                (11, true, false),
                (18, false, false),
                (19, true, true)
            ]
        );
    }

    #[test]
    fn a_late_tick_refreshes_once() {
        let seen = run([0, 100, 101, 108]);
        assert_eq!(
            seen,
            [
                (0, true, true),
                (100, true, false),
                (101, false, false),
                (108, true, true)
            ]
        );
    }

    #[test]
    fn the_end_of_time_does_not_overflow() {
        let mut cadence = HudCadence::default();
        assert!(cadence.advance(u64::MAX - 1).radar);
        assert!(cadence.advance(u64::MAX).radar, "the due time saturates");
        assert!(!cadence.advance(u64::MAX).nav, "the tick repeated");
    }

    /// The ticks among `ticks` on which the nav area refreshes.
    fn nav_refreshes(ticks: impl IntoIterator<Item = u64>) -> Vec<u64> {
        let mut cadence = HudCadence::default();
        ticks
            .into_iter()
            .filter(|&now| cadence.advance(now).nav)
            .collect()
    }

    #[test]
    fn the_nav_refreshes_on_every_fifteenth_tick() {
        assert_eq!(NAV_PERIOD_TICKS, 15);
        assert_eq!(nav_refreshes(0..=45), [0, 15, 30, 45]);
    }

    #[test]
    fn the_nav_refreshes_first_whatever_the_tick() {
        assert_eq!(nav_refreshes([7, 8, 14, 15, 16]), [7, 15]);
    }

    #[test]
    fn a_frame_that_skips_past_a_multiple_still_refreshes_the_nav() {
        assert_eq!(nav_refreshes([0, 14, 16, 29, 31, 100]), [0, 16, 31, 100]);
    }

    #[test]
    fn a_repeated_tick_refreshes_the_nav_once() {
        assert_eq!(nav_refreshes([0, 15, 15, 15, 30, 30]), [0, 15, 30]);
    }

    #[test]
    fn the_nav_and_the_radar_keep_their_own_times() {
        let mut cadence = HudCadence::default();
        let seen: Vec<_> = [0, 8, 15, 16, 23, 30]
            .into_iter()
            .map(|now| cadence.advance(now))
            .collect();
        let refresh = |radar, nav| HudRefresh { radar, nav };
        assert_eq!(
            seen,
            [
                refresh(true, true),
                refresh(true, false),
                refresh(false, true),
                refresh(true, false),
                refresh(false, false),
                refresh(true, true)
            ]
        );
    }
}
