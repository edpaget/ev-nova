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
//! The target panel is redrawn when `TickCount() % 45 == 0` while there
//! is a target (@0x4c8b9-0x4c8eb). At 2 Mac ticks a frame `TickCount`
//! keeps one parity from frame to frame, so the frames land on every
//! other multiple of 45: every 90 Mac ticks, every 45th engine tick
//! (1.5 s), [`TARGET_PERIOD_TICKS`], refreshed by the same crossing test
//! as the nav area. The redraw may so settle a frame later than an exact
//! `% 45` frame, which the player never sees. Inside the radar's block
//! (@0x4c963-0x4c994) it is also redrawn when the shield it was drawn at,
//! kept as a whole number (`_lastTargetedShipShields`, @0x4ad27), no
//! longer equals the target's shield, or its ship type has changed: a
//! recharging or falling shield, fractional, fails that test at every
//! radar refresh, so the panel follows it every 8 engine ticks, and holds
//! still while the shield sits on the whole number it was drawn at. Its
//! armour and condition are not watched: an armour change or a disabling
//! shows at the next 45-tick redraw. A change of target, or its loss,
//! redraws it at once (`_statusRedraw[6]`, e.g. `_HandleShipDisplay`
//! @0x2b551 and the selection keys in `_HandlePlayer` @0x686f9). This is
//! [`retake_target`]. The original skips the 45-tick redraw with no
//! target, which changes nothing seen: "No Target" cannot change without
//! a change of target.
//!
//! The secondary weapon line (`_statusRedraw[3]`) is redrawn only on
//! events, and each change of its content marks it (firing @0x62d11,
//! selection and the dialogs among them), so drawing it every frame
//! shows the same: it has no timer here.
//!
//! `_RedrawStatus` (@0x4cac0) redraws everything at once, then calls
//! `_DrawStatus` (@0x4cc74), whose radar timer can record the time and
//! flip the blink when 15 Mac ticks have passed. The view takes a fresh
//! snapshot without advancing a [`HudCadence`], leaving the time and the
//! blink to the next advance: in flight that shows the same, as the
//! frame's own `_DrawStatus` would flip it in the same frame. In port,
//! where flight doesn't advance, the blink stays as landing left it
//! through the port's redraws too; the original's flips there show only
//! in the blank radar's centre arrow, which no landable stock stellar is
//! far enough out to draw.
//!
//! In port ours keeps no Mac-tick clock, so the port's `_DrawStatus`
//! calls run none of these timers. Each is recorded here, not built:
//!
//! - **The 15-Mac-tick radar gate** (@0x4c911-0x4c923). Recorded, not
//!   built, by operator decision on 2026-10-15. While landed the radar is
//!   blank: the player's pixel, IFF's black fill and cyan player, and the
//!   centre arrow. So a radar redraw the gate skips can hide only an IFF
//!   change, when an outfitter order that buys or sells IFF comes within
//!   15 Mac ticks (0.25 s) of the last radar redraw, as selling then
//!   buying IFF with the outfitter's B and S keys can. Ours retakes the
//!   radar at every port redraw. A refused order runs no timer in the
//!   original (@0x5c09f-0x5c0ba, @0x5c5d5-0x5c5dd), and one `_DrawStatus`
//!   follows a whole multi-item buy (@0x5c5a1).
//! - **Its blink flip** (@0x4c93d-0x4c94f), in the same branch, shows
//!   only in the blank radar's centre arrow, as above. Ours keeps
//!   landing's blink.
//! - **`% 30` nav** (@0x4c88a-0x4c8b6). Recorded, not built, by operator
//!   decision on 2026-10-15. While docked the nav area holds what the
//!   port's last nav redraw drew (`FlightView::nav_shown`), so this redraw
//!   would show a landed `M` or `N`'s write (`_EvalSetExp`
//!   @0x156f2-0x15705, @0x158fd) a script made since: the destination's
//!   first stellar, or "No Destination". On 1 in 30 of the port's
//!   `_DrawStatus` calls the original shows it there, earlier than ours,
//!   which waits for the next nav redraw it models. Nothing in stock does
//!   that: every stock `M472` leaves the port after it (`Q25048`).
//! - **`% 45` target** (@0x4c8b9-0x4c8eb) is unreachable in port. It needs
//!   a target, and landing clears it (`_PlayerLandOnStellar` @0x6332f), as
//!   a ship purchase does (@0x5f014). Nothing sets one while docked, and
//!   ours' `Session::dock` clears it too.
//! - **The port points where the timer runs with no hook in ours**: the
//!   port opening's `_DrawStatus` (@0x5fc5c), a recharge's (@0x5fe05;
//!   ours redraws only the fuel gauge, `FlightView::recharge`), a trade's
//!   (@0x5e33a, @0x5e59e), the mission computer's close (@0x600a6) and
//!   `_DoPlayerInfoDialog`'s `_RefreshSAT` (@0x5a725). None of these can
//!   change the blank radar, and no outfitter order can follow one within
//!   15 Mac ticks, as the outfitter has to be opened first. Their blink
//!   flips show only in the arrow.
//!
//! The reading of
//! [`RuleKey::HudCadence`](nova_sim::RuleKey::HudCadence) says whether the
//! HUD's content waits for these refreshes (the engine's) or follows
//! every frame. The blink keeps this timing under both: it is a feature,
//! not a redraw artefact.

use nova_sim::{NpcId, ShipId};

/// Engine ticks (1/30 s) between radar refreshes: 15 Mac ticks
/// (0.25 s) is 7.5 frames, and the `>=` test passes on the 8th (≈0.267 s).
pub const RADAR_PERIOD_TICKS: u64 = 8;

/// Engine ticks between nav area refreshes: `TickCount() % 30 == 0`
/// (@0x4c899-0x4c8b6) at 2 Mac ticks an engine tick, 0.5 s.
pub const NAV_PERIOD_TICKS: u64 = 15;

/// Engine ticks between the target panel's timed redraws: `TickCount()
/// % 45 == 0` (@0x4c8b9-0x4c8eb) lands on every other multiple of 45 at
/// 2 Mac ticks an engine tick, every 90 Mac ticks, 1.5 s.
pub const TARGET_PERIOD_TICKS: u64 = 45;

/// Which parts of the HUD a [`HudCadence::advance`] refreshes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HudRefresh {
    /// The radar, which flips the blink.
    pub radar: bool,
    /// The nav area.
    pub nav: bool,
    /// The target panel's timed redraw.
    pub target: bool,
}

/// What the target panel was drawn for, or is to be: which NPC, its ship
/// type and its shield.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drawn {
    /// The NPC.
    pub npc: NpcId,
    /// Its ship type.
    pub ship: ShipId,
    /// Its shield points.
    pub shield: f32,
}

/// Whether the target panel drawn for `shown` is taken afresh for `now`,
/// on a frame that refreshed `refresh` (see the module docs): at once
/// when the target has changed, been lost or been gained; on the timed
/// redraw; and on a radar refresh when the shield drawn, kept whole, is
/// not the shield now, or the ship type has changed.
// The original compares the whole shield drawn with the shield now
// exactly (`ucomiss` @0x4c978): a fractional shield never equals it.
#[allow(clippy::float_cmp)]
#[must_use]
pub fn retake_target(shown: Option<&Drawn>, now: Option<&Drawn>, refresh: HudRefresh) -> bool {
    match (shown, now) {
        (None, None) => refresh.target,
        (Some(shown), Some(now)) if shown.npc == now.npc => {
            refresh.target
                || refresh.radar && (shown.shield.trunc() != now.shield || shown.ship != now.ship)
        }
        _ => true,
    }
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
    /// passed since its last refresh, flipping the blink. The nav area and
    /// the target panel: on the first call, then when a multiple of
    /// [`NAV_PERIOD_TICKS`] or [`TARGET_PERIOD_TICKS`] lies after the last
    /// call's tick and up to `now` (see the module docs).
    pub fn advance(&mut self, now: u64) -> HudRefresh {
        let radar = self
            .last_radar
            .is_none_or(|last| now >= last.saturating_add(RADAR_PERIOD_TICKS));
        if radar {
            self.last_radar = Some(now);
            self.blink = !self.blink;
        }
        let crossed = |period: u64| {
            self.last_tick
                .is_none_or(|last| now / period > last / period)
        };
        let (nav, target) = (crossed(NAV_PERIOD_TICKS), crossed(TARGET_PERIOD_TICKS));
        self.last_tick = Some(now);
        HudRefresh { radar, nav, target }
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
    fn the_nav_the_radar_and_the_target_keep_their_own_times() {
        let mut cadence = HudCadence::default();
        let seen: Vec<_> = [0, 8, 15, 16, 23, 30, 45, 46]
            .into_iter()
            .map(|now| cadence.advance(now))
            .collect();
        let refresh = |radar, nav, target| HudRefresh { radar, nav, target };
        assert_eq!(
            seen,
            [
                refresh(true, true, true),
                refresh(true, false, false),
                refresh(false, true, false),
                refresh(true, false, false),
                refresh(false, false, false),
                refresh(true, true, false),
                refresh(true, true, true),
                refresh(false, false, false),
            ]
        );
    }

    /// The ticks among `ticks` on which the target panel's timed redraw
    /// falls.
    fn target_refreshes(ticks: impl IntoIterator<Item = u64>) -> Vec<u64> {
        let mut cadence = HudCadence::default();
        ticks
            .into_iter()
            .filter(|&now| cadence.advance(now).target)
            .collect()
    }

    #[test]
    fn the_target_refreshes_on_every_forty_fifth_tick() {
        assert_eq!(TARGET_PERIOD_TICKS, 45);
        assert_eq!(target_refreshes(0..=90), [0, 45, 90]);
    }

    #[test]
    fn the_target_refreshes_first_whatever_the_tick_then_on_crossings() {
        assert_eq!(target_refreshes([7, 44, 46, 89, 91, 200]), [7, 46, 91, 200]);
        assert_eq!(target_refreshes([0, 45, 45, 90, 90]), [0, 45, 90]);
    }

    const RADAR: HudRefresh = HudRefresh {
        radar: true,
        nav: false,
        target: false,
    };
    const TIMED: HudRefresh = HudRefresh {
        radar: false,
        nav: false,
        target: true,
    };
    const NEITHER: HudRefresh = HudRefresh {
        radar: false,
        nav: true,
        target: false,
    };

    fn drawn(npc: u32, ship: i16, shield: f32) -> Drawn {
        Drawn {
            npc: NpcId(npc),
            ship: ShipId(ship),
            shield,
        }
    }

    /// [`retake_target`] for `shown` and `now` on each of a frame with
    /// no radar or timed refresh, a radar refresh and a timed one.
    fn retaken(shown: Option<Drawn>, now: Option<Drawn>) -> [bool; 3] {
        [NEITHER, RADAR, TIMED].map(|refresh| retake_target(shown.as_ref(), now.as_ref(), refresh))
    }

    #[test]
    fn a_change_of_target_retakes_it_at_once() {
        let one = Some(drawn(1, 128, 10.0));
        let two = Some(drawn(2, 128, 10.0));
        assert_eq!(retaken(one, two), [true; 3], "changed");
        assert_eq!(retaken(one, None), [true; 3], "lost");
        assert_eq!(retaken(None, two), [true; 3], "gained");
        assert_eq!(retaken(None, None), [false, false, true], "none still");
    }

    #[test]
    fn an_unchanged_whole_shield_waits_for_the_timed_redraw() {
        let shown = Some(drawn(1, 128, 10.0));
        assert_eq!(retaken(shown, shown), [false, false, true]);
        let drawn_fraction = Some(drawn(1, 128, 10.7));
        assert_eq!(
            retaken(drawn_fraction, Some(drawn(1, 128, 10.0))),
            [false, false, true],
            "kept whole as drawn"
        );
    }

    #[test]
    fn a_changed_or_fractional_shield_or_type_shows_at_the_next_radar_refresh() {
        let shown = Some(drawn(1, 128, 10.0));
        let changed = [false, true, true];
        assert_eq!(retaken(shown, Some(drawn(1, 128, 9.0))), changed);
        assert_eq!(retaken(shown, Some(drawn(1, 128, 10.5))), changed);
        assert_eq!(retaken(shown, Some(drawn(1, 129, 10.0))), changed);
    }
}
