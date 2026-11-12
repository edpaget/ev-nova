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
//!   and once no ship took the click, the first of the 16 sprite slots
//!   whose rect holds it ([`stellar_under`]). The rebuild hands the slots
//!   out in ascending `spöb` ID (@0x32ee4-0x32fda), so of overlapping
//!   stellars the lowest ID wins. The rect ([`SpriteRect`]) is the frame
//!   the slot shows, grown by 16 pixels a side when it is no wider than 47
//!   (@0x6a649-0x6a671), and `_PtInRect` (@0x6a686) leaves its right and
//!   bottom edges out. A slot frozen on a stale frame is clicked by that
//!   frame's rect from the centre less 16, and a slot showing no frame by
//!   no rect (`nova_sim::Session::click_in_space`). The one selected in
//!   mode 2 is passed over so the walk goes on to any under it
//!   ([`stellar_under_past`], @0x6a69d → @0x6a703). A click on empty space
//!   changes nothing;
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
//! Hyper Select's
//! [`HyperSelectRule::OneJumpCourse`](crate::hyperspace::HyperSelectRule::OneJumpCourse)
//! reading runs over the systems it offers; the engine's walk of the Con
//! slots is `nova_sim::Session::select_next_system`. The session's `nav`
//! module has every setter.

use crate::catalog::{LandingSite, StellarId};
use crate::geometry::Vec2;

pub use nova_sim_state::shown::navigation::{
    CLICK_MARGIN, NavMode, SMALL_SPRITE_WIDTH, SpriteRect,
};

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
/// the lowest `spöb` ID whose rect holds `at` ([`SpriteRect::holds`]),
/// the order the original's slots are walked in. `rect` gives each
/// stellar's slot rect, or `None` for a slot with no rect, which is never
/// clicked. Landability does not count. `None` when the click is on no
/// stellar.
#[must_use]
pub fn stellar_under(
    sites: &[LandingSite],
    at: Vec2,
    rect: impl Fn(&LandingSite) -> Option<SpriteRect>,
) -> Option<StellarId> {
    stellar_under_past(sites, at, None, rect)
}

/// [`stellar_under`], passing over `past`: the click in space's walk,
/// which goes on past the stellar selected in mode 2 to the next one
/// under the click (@0x6a69d → @0x6a703).
#[must_use]
pub fn stellar_under_past(
    sites: &[LandingSite],
    at: Vec2,
    past: Option<StellarId>,
    rect: impl Fn(&LandingSite) -> Option<SpriteRect>,
) -> Option<StellarId> {
    sites
        .iter()
        .filter(|site| Some(site.id) != past)
        .filter(|site| rect(site).is_some_and(|rect| rect.holds(at)))
        .map(|site| site.id)
        .min()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemId;
    use nova_sim_catalog::fixture::planet;

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

    /// The site's frame centred on it, or no rect without a frame.
    fn centred(site: &LandingSite) -> Option<SpriteRect> {
        site.frame_size
            .map(|size| SpriteRect::centred(site.position, size))
    }

    /// What a click at (`x`, `y`) picks among `sites`, each boxed centred.
    fn pick(sites: &[LandingSite], x: f32, y: f32) -> Option<StellarId> {
        stellar_under(sites, Vec2::new(x, y), centred)
    }

    #[test]
    fn a_click_inside_a_stellars_sprite_picks_it() {
        let sites = [sized(128, 100.0, 50.0, Some((80, 60)))];
        assert_eq!(pick(&sites, 100.0, 50.0), Some(StellarId(128)));
        for corner in [(60.0, 20.0), (139.5, 79.5), (60.0, 79.5), (139.5, 20.0)] {
            assert_eq!(
                pick(&sites, corner.0, corner.1),
                Some(StellarId(128)),
                "{corner:?}"
            );
        }
        for corner in [(140.0, 80.0), (60.0, 80.0), (140.0, 20.0)] {
            assert_eq!(pick(&sites, corner.0, corner.1), None, "{corner:?}");
        }
    }

    #[test]
    fn a_click_just_outside_a_stellars_sprite_picks_nothing() {
        let sites = [sized(128, 100.0, 50.0, Some((80, 60)))];
        for at in [(59.5, 50.0), (140.0, 50.0), (100.0, 19.5), (100.0, 80.0)] {
            assert_eq!(pick(&sites, at.0, at.1), None, "{at:?}");
        }
    }

    #[test]
    fn a_small_sprite_is_clicked_with_16_pixels_to_spare_on_each_side() {
        let sites = [sized(128, 0.0, 0.0, Some((SMALL_SPRITE_WIDTH, 30)))];
        // 47 wide puts its left at -23 and its right at 24; 30 high spans
        // [-15, 15): 16 more on each side.
        for at in [(-39.0, 0.0), (39.5, 0.0), (0.0, -31.0), (0.0, 30.5)] {
            assert_eq!(pick(&sites, at.0, at.1), Some(StellarId(128)), "{at:?}");
        }
        for at in [(-39.5, 0.0), (40.0, 0.0), (0.0, -31.5), (0.0, 31.0)] {
            assert_eq!(pick(&sites, at.0, at.1), None, "{at:?}");
        }
    }

    #[test]
    fn a_sprite_48_pixels_wide_gets_no_margin() {
        let sites = [sized(128, 0.0, 0.0, Some((48, 30)))];
        assert_eq!(pick(&sites, -24.0, -15.0), Some(StellarId(128)));
        assert_eq!(pick(&sites, 23.5, 14.5), Some(StellarId(128)));
        assert_eq!(pick(&sites, -24.5, 0.0), None);
        assert_eq!(pick(&sites, 24.0, 0.0), None);
        assert_eq!(pick(&sites, 0.0, 15.0), None);
    }

    #[test]
    fn a_40_by_60_sheet_gets_the_margin_and_a_60_by_40_one_does_not() {
        let tall = [sized(128, 0.0, 0.0, Some((40, 60)))];
        for at in [(-36.0, 0.0), (35.5, 0.0), (0.0, -46.0), (0.0, 45.5)] {
            assert_eq!(pick(&tall, at.0, at.1), Some(StellarId(128)), "{at:?}");
        }
        for at in [(36.0, 0.0), (0.0, 46.0), (-36.5, 0.0), (0.0, -46.5)] {
            assert_eq!(pick(&tall, at.0, at.1), None, "{at:?}");
        }
        let wide = [sized(128, 0.0, 0.0, Some((60, 40)))];
        assert_eq!(pick(&wide, -30.0, -20.0), Some(StellarId(128)));
        for at in [(0.0, -21.0), (0.0, 20.5), (-30.5, 0.0), (30.0, 0.0)] {
            assert_eq!(pick(&wide, at.0, at.1), None, "{at:?}");
        }
    }

    #[test]
    fn a_click_on_the_right_or_bottom_edge_misses() {
        let sites = [sized(128, 100.0, 50.0, Some((100, 100)))];
        assert_eq!(pick(&sites, 50.0, 0.0), Some(StellarId(128)));
        assert_eq!(pick(&sites, 149.5, 99.5), Some(StellarId(128)));
        assert_eq!(pick(&sites, 150.0, 50.0), None);
        assert_eq!(pick(&sites, 100.0, 100.0), None);
    }

    #[test]
    fn an_odd_width_puts_its_extra_pixel_on_the_right() {
        let sites = [sized(128, 0.0, 0.0, Some((61, 61)))];
        assert_eq!(pick(&sites, 30.5, 0.0), Some(StellarId(128)));
        assert_eq!(pick(&sites, 0.0, 30.5), Some(StellarId(128)));
        assert_eq!(pick(&sites, -30.5, 0.0), None);
        assert_eq!(pick(&sites, 0.0, -30.5), None);
    }

    #[test]
    fn of_overlapping_stellars_the_lowest_id_is_picked() {
        let high_first = [planet(200, 10.0, 0.0), planet(130, 0.0, 0.0)];
        let low_first = [planet(130, 0.0, 0.0), planet(200, 10.0, 0.0)];
        for sites in [&high_first, &low_first] {
            assert_eq!(pick(sites, 5.0, 0.0), Some(StellarId(130)));
            assert_eq!(
                stellar_under_past(sites, Vec2::new(5.0, 0.0), Some(StellarId(130)), centred),
                Some(StellarId(200)),
                "past the lowest"
            );
            assert_eq!(pick(sites, 55.0, 0.0), Some(StellarId(200)), "only 200");
            assert_eq!(pick(sites, -45.0, 0.0), Some(StellarId(130)), "only 130");
        }
    }

    #[test]
    fn a_stellar_with_no_rect_is_never_clicked() {
        let sites = [sized(128, 0.0, 0.0, None)];
        for at in [(0.0, 0.0), (10.0, -10.0)] {
            assert_eq!(pick(&sites, at.0, at.1), None, "{at:?}");
        }
    }

    #[test]
    fn the_click_box_is_the_rect_the_caller_gives() {
        let sites = [sized(128, 0.0, 0.0, Some((40, 40)))];
        let drawn = |site: &LandingSite| Some(SpriteRect::centred(site.position, (200, 200)));
        assert_eq!(
            stellar_under(&sites, Vec2::new(90.0, 0.0), drawn),
            Some(StellarId(128))
        );
        assert_eq!(stellar_under(&sites, Vec2::new(110.0, 0.0), drawn), None);
    }

    #[test]
    fn no_stellars_nothing_under_the_click() {
        assert_eq!(pick(&[], 0.0, 0.0), None);
    }
}
