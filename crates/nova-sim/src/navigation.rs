//! Navigation: the stellar the player has selected as the navigation
//! target.
//!
//! The stellars that can be selected are the system's navigation
//! defaults, its `sÿst`'s `NavDef`s, in their order (the Nova Bible: "if
//! you don't set a planet as a nav default ... you can't select it").
//!
//! The original (`_HandlePlayer` @0x68390 in `EV Nova.app`, read with
//! `otool -tV -arch i386`) keeps the selection in one slot of the
//! player's ship record (+0x6a its kind, 2 for a stellar and 3 for
//! hyperspace; +0x6c its index), and every input below is skipped while
//! the ship is jumping. It changes the selection by:
//!
//! - **keys 1-4 and F1-F4** (@0x69cf6-0x69f17; `_loadKeys` @0xce6c8
//!   hard-codes F1-F4 as 0x7a, 0x78, 0x63, 0x76): the stellar in that
//!   slot of the `NavDef`s, when there is one;
//! - **F5** (or Cmd-5): the nearest, through `_FindNearestStellar`
//!   (@0x276c), as the land key with no stellar selected;
//! - **a click in space** (@0x6a000-0x6a717): outside the 194-pixel status
//!   panel on the right (so never on the radar, which has no click path),
//!   and once no ship took the click, the first stellar in nav order whose
//!   sprite rect holds it, the rect grown by 16 pixels a side for a sprite
//!   no taller than 47 ([`stellar_under`]). A click on empty space changes
//!   nothing;
//! - **Nav Off**, the backquote key (`_loadKeys` @0xcd5f8 `navOff` 0x32,
//!   handled @0x69a17-0x69b63): clears it, the only key that does;
//! - **Hyper Select** (`\`, @0x69bb2): moves it to hyperspace.
//!
//! The land key (@0x6a8ac-0x6a95b) selects the nearest only when no
//! stellar is selected, and otherwise asks to dock at the selected one,
//! so it never cycles. `_FindNextStellarInSystem` (@0x25a2) has no caller:
//! the original has no stellar cycle key. [`next_after`] is the cycle
//! Hyper Select runs over the systems it offers.

use crate::catalog::{LandingSite, StellarId};
use crate::geometry::Vec2;
use crate::landing::UNKNOWN_STELLAR_RADIUS;

/// How far a click may land outside a small sprite and still hit it, in
/// pixels on each side: the original grows the sprite's rect by 16 on
/// every side before testing a click against it (`_HandlePlayer`
/// @0x6a5f9-0x6a6fc for stellars, and the same rule for ships before it).
pub const CLICK_MARGIN: f32 = 16.0;

/// The tallest sprite frame, in pixels, that gets the [`CLICK_MARGIN`]:
/// the original compares the frame height with 0x2f (47) and grows the
/// rect only when it is no taller.
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
/// included. The box is the sprite's frame centred on the stellar, grown
/// by [`CLICK_MARGIN`] on each side when the frame is no taller than
/// [`SMALL_SPRITE_HEIGHT`]; a stellar with no sprite is boxed by the
/// 64-pixel placeholder the views draw (twice
/// [`UNKNOWN_STELLAR_RADIUS`]). Landability does not count. `None` when
/// the click is on no stellar.
#[must_use]
pub fn stellar_under(sites: &[LandingSite], at: Vec2) -> Option<StellarId> {
    sites
        .iter()
        .find(|site| {
            let (half_width, half_height) = half_box(site);
            let offset = at - site.position;
            offset.x.abs() <= half_width && offset.y.abs() <= half_height
        })
        .map(|site| site.id)
}

/// Half the width and half the height of `site`'s click box.
fn half_box(site: &LandingSite) -> (f32, f32) {
    match site.frame_size {
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
    use crate::catalog::SystemId;
    use crate::testkit::planet;

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
            stellar_under(&sites, Vec2::new(100.0, 50.0)),
            Some(StellarId(128))
        );
        for corner in [(60.0, 20.0), (140.0, 80.0), (60.0, 80.0), (140.0, 20.0)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(corner.0, corner.1)),
                Some(StellarId(128)),
                "{corner:?}"
            );
        }
    }

    #[test]
    fn a_click_just_outside_a_stellars_sprite_picks_nothing() {
        let sites = [sized(128, 100.0, 50.0, Some((80, 60)))];
        for at in [(59.5, 50.0), (140.5, 50.0), (100.0, 19.5), (100.0, 80.5)] {
            assert_eq!(stellar_under(&sites, Vec2::new(at.0, at.1)), None, "{at:?}");
        }
    }

    #[test]
    fn a_small_sprite_is_clicked_with_16_pixels_to_spare_on_each_side() {
        let sites = [sized(128, 0.0, 0.0, Some((30, SMALL_SPRITE_HEIGHT)))];
        // Half of 30 is 15, half of 47 is 23.5: 16 more on each side.
        for at in [(31.0, 0.0), (-31.0, 0.0), (0.0, 39.5), (0.0, -39.5)] {
            assert_eq!(
                stellar_under(&sites, Vec2::new(at.0, at.1)),
                Some(StellarId(128)),
                "{at:?}"
            );
        }
        for at in [(31.5, 0.0), (-31.5, 0.0), (0.0, 40.0), (0.0, -40.0)] {
            assert_eq!(stellar_under(&sites, Vec2::new(at.0, at.1)), None, "{at:?}");
        }
    }

    #[test]
    fn a_sprite_48_pixels_high_gets_no_margin() {
        let sites = [sized(128, 0.0, 0.0, Some((30, 48)))];
        assert_eq!(
            stellar_under(&sites, Vec2::new(15.0, 24.0)),
            Some(StellarId(128))
        );
        assert_eq!(stellar_under(&sites, Vec2::new(15.5, 0.0)), None);
        assert_eq!(stellar_under(&sites, Vec2::new(0.0, 24.5)), None);
    }

    #[test]
    fn of_overlapping_stellars_the_first_in_nav_order_is_picked() {
        let sites = [
            sized(131, 10.0, 0.0, Some((100, 100))),
            sized(128, 0.0, 0.0, Some((100, 100))),
        ];
        assert_eq!(
            stellar_under(&sites, Vec2::new(5.0, 0.0)),
            Some(StellarId(131))
        );
        assert_eq!(
            stellar_under(&sites, Vec2::new(-45.0, 0.0)),
            Some(StellarId(128)),
            "only under the second"
        );
    }

    #[test]
    fn a_stellar_with_no_sprite_is_clicked_on_its_64_pixel_placeholder() {
        let sites = [sized(128, 0.0, 0.0, None)];
        assert_eq!(
            stellar_under(&sites, Vec2::new(32.0, -32.0)),
            Some(StellarId(128))
        );
        assert_eq!(stellar_under(&sites, Vec2::new(32.5, 0.0)), None);
        assert_eq!(stellar_under(&sites, Vec2::new(0.0, 32.5)), None);
    }

    #[test]
    fn no_stellars_nothing_under_the_click() {
        assert_eq!(stellar_under(&[], Vec2::ZERO), None);
    }
}
