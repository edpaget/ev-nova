//! Navigation: the player's nav mode and its selection, and how a
//! stellar is picked.
//!
//! The stellars that can be selected are the system's navigation
//! defaults, its `sÿst`'s `NavDef`s, in their order (the Nova Bible: "if
//! you don't set a planet as a nav default ... you can't select it").
//!
//! The original (`_HandlePlayer` @0x68390 in `EV Nova.app`, read with
//! `otool -tV -arch i386`) keeps the player's nav mode in two words of the
//! player's ship record ([`NavMode`]): +0x6a the mode (−1 off, 2 a
//! stellar, 3 hyperspace) and +0x6c its selection. Every input below is
//! skipped while the ship is jumping. The keys that set it:
//!
//! - **keys 1-4 and F1-F4** (@0x69cf6-0x69f17; `_loadKeys` @0xce6c8
//!   hard-codes F1-F4 as 0x7a, 0x78, 0x63, 0x76): the stellar in that
//!   slot of the `NavDef`s, when there is one, in mode 2;
//! - **F5** (or Cmd-5): the nearest, through `_FindNearestStellar`
//!   (@0x276c), as the land key with no stellar selected;
//! - **a click in space** (@0x6a000-0x6a717): outside the 194-pixel status
//!   panel on the right (so never on the radar, which has no click path),
//!   and once no ship took the click, the first stellar in nav order whose
//!   sprite rect, the frame of the sheet drawn for it, holds it, the rect
//!   grown by 16 pixels a side for a sprite no taller than 47
//!   ([`stellar_under`]), the one selected in mode 2 passed over so the
//!   walk goes on to any under it ([`stellar_under_past`], @0x6a69d →
//!   @0x6a703). A click on empty space changes nothing;
//! - **Nav Off**, the backquote key (`_loadKeys` @0xcd5f8 `navOff` 0x32,
//!   handled @0x69a94-0x69b63): the mode off, the course kept;
//! - **Hyperspace Mode**, H (`hyperMode` 0x04, @0x69b76-0x69b89): mode 3,
//!   the course's next hop cued;
//! - **Hyper Select** (`\`, @0x69bb2-0x69cec): mode 3, the next of the
//!   system's links.
//!
//! The land key (@0x6a8ac-0x6a95b) selects the nearest only when no
//! stellar is selected, and otherwise asks to dock at the selected one,
//! so it never cycles. `_FindNextStellarInSystem` (@0x25a2) has no caller:
//! the original has no stellar cycle key. [`next_after`] is the cycle
//! Hyper Select runs over the systems it offers. The session's
//! `nav` module has every setter.

use crate::catalog::{LandingSite, StellarId, SystemId};
use crate::geometry::Vec2;
use crate::landing::UNKNOWN_STELLAR_RADIUS;

/// The player's nav mode and its selection: the original's player +0x6a
/// and +0x6c. Mode 4, the hypergate map's, is not modelled: the
/// hypergate resets it to −1 (@0x63fa2) before any status draw. The
/// mode is not saved: `_ResetPlayer` sets −1 / −1 (@0x1d4cd-0x1d4d5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavMode {
    /// Mode −1: the nav system off, nothing selected.
    #[default]
    Off,
    /// Mode 2: the stellar selected, `None` for none (+0x6c −1).
    Stellar(Option<StellarId>),
    /// Mode 3: the system selected, `None` for none (+0x6c −1). The
    /// original keeps a slot of the current system's links, resolved by
    /// place whenever it is read (`_FindActiveCoLocatedSystem` @0x4be0);
    /// here it is the system, resolved by place as it is read.
    Hyperspace(Option<SystemId>),
}

impl NavMode {
    /// The mode kept and its selection let go (+0x6c −1), as a take-off,
    /// a gate's exit or an in-flight `M` writes it.
    #[must_use]
    pub fn deselected(self) -> Self {
        match self {
            Self::Off => Self::Off,
            Self::Stellar(_) => Self::Stellar(None),
            Self::Hyperspace(_) => Self::Hyperspace(None),
        }
    }

    /// The stellar selected in mode 2, the one every reader of the
    /// stellar selection asks for: `None` in any other mode.
    #[must_use]
    pub fn stellar(self) -> Option<StellarId> {
        match self {
            Self::Stellar(stellar) => stellar,
            _ => None,
        }
    }

    /// The system selected in mode 3, the one the galaxy map draws its
    /// selected link to (`_DrawMap` @0xebd8-0xed31): `None` in any other
    /// mode.
    #[must_use]
    pub fn hyperspace(self) -> Option<SystemId> {
        match self {
            Self::Hyperspace(system) => system,
            _ => None,
        }
    }
}

/// How far a click may land outside a small sprite and still hit it, in
/// pixels on each side: the original grows the sprite's rect by 16 on
/// every side before testing a click against it (`_HandlePlayer`
/// @0x6a5f9-0x6a6fc for stellars, and the same rule for ships before it).
pub const CLICK_MARGIN: f32 = 16.0;

/// The tallest sprite frame, in pixels, that gets the [`CLICK_MARGIN`]:
/// the engine compares the frame height with 0x2f (47) and grows the
/// rect only when it is no taller. The original's test reads the width,
/// right less left (@0x6a649-0x6a656): story-runtime phase 20
/// (click-target-shape).
pub const SMALL_SPRITE_HEIGHT: u32 = 47;

/// The item after `current` in the cycle `items`: the one after it,
/// wrapping from the last to the first, or the first when `current` is
/// `None` or not among them. `None` when there are no items. Hyper
/// Select's systems cycle this way.
#[must_use]
pub fn next_after<T: Copy + PartialEq>(items: &[T], current: Option<T>) -> Option<T> {
    let after = current
        .and_then(|current| items.iter().position(|&item| item == current))
        .map_or(0, |at| at + 1);
    items.get(after).or_else(|| items.first()).copied()
}

/// The stellar a click at `at` (in system coordinates) picks among
/// `sites`, as the original's click in space does once no ship took it:
/// the first, in navigation order, whose hit box holds `at`, edges
/// included. The box is the frame of the sheet drawn for the stellar,
/// which `drawn` gives (the original tests the sprite slot's current
/// frame rect, so a destroyed stellar is clicked by the sheet it shows,
/// story-runtime phase 11), centred on the stellar and grown by
/// [`CLICK_MARGIN`] on each side when the frame is no taller than
/// [`SMALL_SPRITE_HEIGHT`]; when `drawn` gives `None` the box is the
/// 64-pixel placeholder the views draw (twice
/// [`UNKNOWN_STELLAR_RADIUS`]). Landability does not count. `None` when
/// the click is on no stellar.
#[must_use]
pub fn stellar_under(
    sites: &[LandingSite],
    at: Vec2,
    drawn: impl Fn(&LandingSite) -> Option<(u32, u32)>,
) -> Option<StellarId> {
    stellar_under_past(sites, at, None, drawn)
}

/// [`stellar_under`], passing over `past`: the click in space's walk,
/// which goes on past the stellar selected in mode 2 to the next one
/// under the click (@0x6a69d → @0x6a703).
#[must_use]
pub fn stellar_under_past(
    sites: &[LandingSite],
    at: Vec2,
    past: Option<StellarId>,
    drawn: impl Fn(&LandingSite) -> Option<(u32, u32)>,
) -> Option<StellarId> {
    sites
        .iter()
        .filter(|site| Some(site.id) != past)
        .find(|site| {
            let (half_width, half_height) = half_box(drawn(site));
            let offset = at - site.position;
            offset.x.abs() <= half_width && offset.y.abs() <= half_height
        })
        .map(|site| site.id)
}

/// Half the width and half the height of the click box around a drawn
/// frame of `frame_size`.
pub(crate) fn half_box(frame_size: Option<(u32, u32)>) -> (f32, f32) {
    match frame_size {
        None => (UNKNOWN_STELLAR_RADIUS, UNKNOWN_STELLAR_RADIUS),
        Some((width, height)) => {
            let margin = if height <= SMALL_SPRITE_HEIGHT {
                CLICK_MARGIN
            } else {
                0.0
            };
            (width as f32 / 2.0 + margin, height as f32 / 2.0 + margin)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::planet;

    #[test]
    fn the_nav_mode_starts_off() {
        assert_eq!(NavMode::default(), NavMode::Off);
    }

    #[test]
    fn deselecting_keeps_the_mode_and_lets_go_of_the_selection() {
        let cases = [
            (NavMode::Off, NavMode::Off),
            (
                NavMode::Stellar(Some(StellarId(128))),
                NavMode::Stellar(None),
            ),
            (NavMode::Stellar(None), NavMode::Stellar(None)),
            (
                NavMode::Hyperspace(Some(SystemId(131))),
                NavMode::Hyperspace(None),
            ),
            (NavMode::Hyperspace(None), NavMode::Hyperspace(None)),
        ];
        for (mode, deselected) in cases {
            assert_eq!(mode.deselected(), deselected, "{mode:?}");
        }
    }

    #[test]
    fn only_mode_2_with_a_stellar_names_a_stellar() {
        assert_eq!(
            NavMode::Stellar(Some(StellarId(128))).stellar(),
            Some(StellarId(128))
        );
        for mode in [
            NavMode::Off,
            NavMode::Stellar(None),
            NavMode::Hyperspace(Some(SystemId(128))),
            NavMode::Hyperspace(None),
        ] {
            assert_eq!(mode.stellar(), None, "{mode:?}");
        }
    }

    #[test]
    fn only_mode_3_with_a_system_names_a_hyperspace_selection() {
        assert_eq!(
            NavMode::Hyperspace(Some(SystemId(131))).hyperspace(),
            Some(SystemId(131))
        );
        for mode in [
            NavMode::Off,
            NavMode::Stellar(Some(StellarId(128))),
            NavMode::Stellar(None),
            NavMode::Hyperspace(None),
        ] {
            assert_eq!(mode.hyperspace(), None, "{mode:?}");
        }
    }

    #[test]
    fn next_after_cycles_any_list_and_restarts_from_a_missing_current() {
        let systems = [SystemId(134), SystemId(131), SystemId(135)];
        assert_eq!(next_after(&systems, None), Some(SystemId(134)));
        assert_eq!(
            next_after(&systems, Some(SystemId(134))),
            Some(SystemId(131))
        );
        assert_eq!(
            next_after(&systems, Some(SystemId(135))),
            Some(SystemId(134)),
            "wraps"
        );
        assert_eq!(
            next_after(&systems, Some(SystemId(7))),
            Some(SystemId(134)),
            "missing"
        );
        assert_eq!(next_after::<SystemId>(&[], Some(SystemId(7))), None);
    }

    fn sized(id: i16, x: f32, y: f32, frame: Option<(u32, u32)>) -> LandingSite {
        LandingSite {
            frame_size: frame,
            ..planet(id, x, y)
        }
    }

    #[test]
    fn a_click_inside_a_stellars_sprite_picks_it() {
        let sites = [sized(128, 100.0, 50.0, Some((80, 60)))];
        assert_eq!(
            stellar_under(&sites, Vec2::new(100.0, 50.0), |site| site.frame_size),
            Some(StellarId(128))
        );
        for corner in [(60.0, 20.0), (140.0, 80.0), (60.0, 80.0), (140.0, 20.0)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(corner.0, corner.1), |site| site
                    .frame_size),
                Some(StellarId(128)),
                "{corner:?}"
            );
        }
    }

    #[test]
    fn a_click_just_outside_a_stellars_sprite_picks_nothing() {
        let sites = [sized(128, 100.0, 50.0, Some((80, 60)))];
        for at in [(59.5, 50.0), (140.5, 50.0), (100.0, 19.5), (100.0, 80.5)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(at.0, at.1), |site| site.frame_size),
                None,
                "{at:?}"
            );
        }
    }

    #[test]
    fn a_small_sprite_is_clicked_with_16_pixels_to_spare_on_each_side() {
        let sites = [sized(128, 0.0, 0.0, Some((30, SMALL_SPRITE_HEIGHT)))];
        // Half of 30 is 15, half of 47 is 23.5: 16 more on each side.
        for at in [(31.0, 0.0), (-31.0, 0.0), (0.0, 39.5), (0.0, -39.5)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(at.0, at.1), |site| site.frame_size),
                Some(StellarId(128)),
                "{at:?}"
            );
        }
        for at in [(31.5, 0.0), (-31.5, 0.0), (0.0, 40.0), (0.0, -40.0)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(at.0, at.1), |site| site.frame_size),
                None,
                "{at:?}"
            );
        }
    }

    #[test]
    fn a_sprite_48_pixels_high_gets_no_margin() {
        let sites = [sized(128, 0.0, 0.0, Some((30, 48)))];
        assert_eq!(
            stellar_under(&sites, Vec2::new(15.0, 24.0), |site| site.frame_size),
            Some(StellarId(128))
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(15.5, 0.0), |site| site.frame_size),
            None
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(0.0, 24.5), |site| site.frame_size),
            None
        );
    }

    #[test]
    fn of_overlapping_stellars_the_first_in_nav_order_is_picked() {
        let sites = [
            sized(131, 10.0, 0.0, Some((100, 100))),
            sized(128, 0.0, 0.0, Some((100, 100))),
        ];
        assert_eq!(
            stellar_under(&sites, Vec2::new(5.0, 0.0), |site| site.frame_size),
            Some(StellarId(131))
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(-45.0, 0.0), |site| site.frame_size),
            Some(StellarId(128)),
            "only under the second"
        );
    }

    #[test]
    fn a_stellar_with_no_sprite_is_clicked_on_its_64_pixel_placeholder() {
        let sites = [sized(128, 0.0, 0.0, None)];
        assert_eq!(
            stellar_under(&sites, Vec2::new(32.0, -32.0), |site| site.frame_size),
            Some(StellarId(128))
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(32.5, 0.0), |site| site.frame_size),
            None
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(0.0, 32.5), |site| site.frame_size),
            None
        );
    }

    #[test]
    fn the_click_box_is_the_size_the_caller_says_is_drawn() {
        let sites = [sized(128, 0.0, 0.0, Some((40, 40)))];
        let drawn = |_: &LandingSite| Some((200, 200));
        assert_eq!(
            stellar_under(&sites, Vec2::new(90.0, 0.0), drawn),
            Some(StellarId(128))
        );
        assert_eq!(stellar_under(&sites, Vec2::new(110.0, 0.0), drawn), None);
    }

    #[test]
    fn no_stellars_nothing_under_the_click() {
        assert_eq!(stellar_under(&[], Vec2::ZERO, |site| site.frame_size), None);
    }
}
