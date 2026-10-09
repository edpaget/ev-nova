//! The stellars shown that have been dead since the last rebuild point:
//! the original's sprite slots holding the frames its rebuild
//! (`_HandleStellarSystemVisibility` @0x32aa5) gave a destroyed stellar.
//! See the module docs' "Built by phase 9" and "Checked for phase 9".
//!
//! At a rebuild point the original frees each stellar master no stellar of
//! the player's system uses, empties every cloned slot, and clones into
//! each slot the master of the graphic its stellar now shows: `DeadType`'s
//! when destroyed (@0x32f0f-0x32f2f). A slot left empty, as one of a
//! destroyed stellar whose `DeadType` has no sheet is, gets graphic type
//! 0's frames (`spïn` 1000, @0x33244-0x332d0). From then on, the per-frame
//! sprite tick (`_HandleStellarSprites` @0x2e77b-0x2e7e5) re-clones the
//! `Type` master into the slot of a stellar it finds whole, reloading the
//! master; one it finds destroyed with no `DeadType` sheet keeps whatever
//! frames it has. So a stellar is in [`Rebuilt`] from a rebuild point that
//! found it destroyed until a flight frame finds it whole, whatever `Y`
//! and `U` do between frames.
//!
//! The state is shared by the readings that depend on it:
//!
//! - story-runtime phase 9: the flight view draws such a stellar with no
//!   `DeadType` sheet with graphic type 0
//!   ([`RuleKey::DeadTypeFallback`](crate::RuleKey::DeadTypeFallback));
//! - story-runtime phase 10: the masters loaded and the slots left with
//!   no frame, which the player's landing distance reads
//!   ([`RuleKey::LandingDistance`](crate::RuleKey::LandingDistance)). A
//!   rebuild point loads the master of each graphic a stellar shown
//!   shows, freeing every other, and graphic type 0's when it fills an
//!   empty slot; a slot is left with no frame when its stellar shows
//!   nothing that can be read and graphic type 0 cannot be read either.
//!   A flight frame loads the master of each graphic shown that can be
//!   read and fills its slot. So a destroyed stellar's `Type` master is
//!   unloaded from a rebuild point until a frame finds it whole, unless
//!   another stellar shown keeps it loaded;
//! - story-runtime phase 12: the frame count and frame index kept from the
//!   last rebuild, re-read on `SessionEvent::StellarsRebuilt` and
//!   `SessionEvent::SystemChanged`.

use std::collections::BTreeSet;

use crate::catalog::{LandingSite, StellarId};
use crate::landing::TypeWidth;

/// The stellar sprites as the original's last rebuild point and the
/// flight frames since left them (see the module docs): the stellars shown
/// that were destroyed at the last rebuild point and have not been seen
/// whole at a flight frame since, the graphics whose master is loaded, and
/// the stellars whose slot shows no frame. Never saved: a load is itself a
/// rebuild point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rebuilt {
    dead: BTreeSet<StellarId>,
    masters: BTreeSet<i16>,
    empty: BTreeSet<StellarId>,
}

/// The graphic `site` shows, when `destroyed` says whether it is: its
/// `Type` while whole, its `DeadType` while destroyed, each only when its
/// sheet can be read, so that the original can clone its master.
fn shown_graphic(site: &LandingSite, destroyed: bool) -> Option<i16> {
    if destroyed {
        site.dead_graphic
    } else {
        site.frame_size.map(|_| site.graphic)
    }
}

impl Rebuilt {
    /// A rebuild point for the stellars `shown`: those that `destroyed`
    /// holds destroyed become the dead set, and every other leaves it; the
    /// masters loaded become the graphics they show, plus graphic type 0
    /// when some slot is left empty and `fallback_reads`, the original
    /// filling it with graphic type 0; and a slot is left with no frame
    /// when its stellar shows nothing that can be read and the fallback
    /// cannot be read either. Whether the dead set changed.
    pub(crate) fn rebuild(
        &mut self,
        shown: &[LandingSite],
        destroyed: impl Fn(StellarId) -> bool,
        fallback_reads: bool,
    ) -> bool {
        let dead: BTreeSet<StellarId> = shown
            .iter()
            .map(|site| site.id)
            .filter(|&id| destroyed(id))
            .collect();
        let changed = dead != self.dead;
        self.dead = dead;
        let graphics: Vec<(StellarId, Option<i16>)> = shown
            .iter()
            .map(|site| (site.id, shown_graphic(site, destroyed(site.id))))
            .collect();
        self.masters = graphics
            .iter()
            .filter_map(|&(_, graphic)| graphic)
            .collect();
        let unfilled = graphics
            .iter()
            .filter(|(_, graphic)| graphic.is_none())
            .map(|&(id, _)| id);
        if fallback_reads {
            if unfilled.clone().next().is_some() {
                self.masters.insert(0);
            }
            self.empty.clear();
        } else {
            self.empty = unfilled.collect();
        }
        changed
    }

    /// A flight frame's sprite tick over the stellars `shown`: every
    /// stellar that `destroyed` holds whole leaves the dead set, and each
    /// stellar showing a graphic that can be read has its master loaded
    /// and its frames cloned into its slot.
    pub(crate) fn frame(&mut self, shown: &[LandingSite], destroyed: impl Fn(StellarId) -> bool) {
        self.dead.retain(|&id| destroyed(id));
        for site in shown {
            if let Some(graphic) = shown_graphic(site, destroyed(site.id)) {
                self.masters.insert(graphic);
                self.empty.remove(&site.id);
            }
        }
    }

    /// Whether `stellar` has been dead since the last rebuild point.
    pub(crate) fn dead(&self, stellar: StellarId) -> bool {
        self.dead.contains(&stellar)
    }

    /// Whether `stellar`'s sprite slot shows a frame.
    pub(crate) fn shows_frame(&self, stellar: StellarId) -> bool {
        !self.empty.contains(&stellar)
    }

    /// Which width the original's `_GetStellarSpriteXSize` reads of
    /// `site`'s `Type` master: unloaded unless its sheet can be read and
    /// its master is loaded; out of range when its `spïn` gives no frame
    /// count; otherwise the frame's.
    pub(crate) fn type_width(&self, site: &LandingSite) -> TypeWidth {
        if site.frame_size.is_none() || !self.masters.contains(&site.graphic) {
            TypeWidth::Unloaded
        } else if site.type_counts_frames {
            TypeWidth::Frame
        } else {
            TypeWidth::OutOfRange
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::testkit::planet;

    /// Planets `ids`, each of its own graphic type, its ID less 100, with
    /// none to show while destroyed.
    fn ids(ids: &[i16]) -> Vec<LandingSite> {
        ids.iter()
            .map(|&id| LandingSite {
                graphic: id - 100,
                ..planet(id, 0.0, 0.0)
            })
            .collect()
    }

    /// A destroyed state: these IDs are destroyed.
    fn destroyed(ids: &[i16]) -> impl Fn(StellarId) -> bool {
        let set: BTreeSet<i16> = ids.iter().copied().collect();
        move |id: StellarId| set.contains(&id.0)
    }

    fn dead(rebuilt: &Rebuilt, ids: &[i16]) -> Vec<bool> {
        ids.iter().map(|&id| rebuilt.dead(StellarId(id))).collect()
    }

    #[test]
    fn a_rebuild_records_exactly_the_shown_stellars_destroyed() {
        let mut rebuilt = Rebuilt::default();
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, false], "none at first");
        assert!(rebuilt.rebuild(&ids(&[128, 129, 130]), destroyed(&[129, 130, 140]), true));
        assert_eq!(
            dead(&rebuilt, &[128, 129, 130, 140]),
            [false, true, true, false],
            "140 is not shown"
        );
    }

    #[test]
    fn a_rebuild_reports_a_change_only_when_the_set_changed() {
        let mut rebuilt = Rebuilt::default();
        assert!(
            !rebuilt.rebuild(&ids(&[128]), destroyed(&[]), true),
            "none still"
        );
        assert!(rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), true));
        assert!(
            !rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), true),
            "the same"
        );
        assert!(rebuilt.rebuild(&ids(&[128, 129]), destroyed(&[128, 129]), true));
        assert!(
            rebuilt.rebuild(&ids(&[129]), destroyed(&[128, 129]), true),
            "one less"
        );
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        assert!(
            rebuilt.rebuild(&ids(&[]), destroyed(&[129]), true),
            "emptied"
        );
        assert_eq!(dead(&rebuilt, &[129]), [false]);
    }

    #[test]
    fn a_frame_drops_a_stellar_seen_whole_and_keeps_one_still_destroyed() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&ids(&[128, 129]), destroyed(&[128, 129]), true);
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[129]));
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128, 129]));
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true], "not back");
    }

    #[test]
    fn y_u_y_with_no_frame_between_keeps_it_and_a_frame_while_whole_drops_it() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), true);
        // `U` then `Y`: nothing is told until a frame, which finds it
        // destroyed again.
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128]));
        assert!(rebuilt.dead(StellarId(128)));
        // `U`, a frame while whole, then `Y`.
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[]));
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128]));
        assert!(!rebuilt.dead(StellarId(128)));
    }

    /// Planet `id` of graphic type `graphic`, showing `dead_graphic`
    /// while destroyed.
    fn shows(id: i16, graphic: i16, dead_graphic: Option<i16>) -> LandingSite {
        LandingSite {
            graphic,
            dead_graphic,
            ..planet(id, 0.0, 0.0)
        }
    }

    fn widths(rebuilt: &Rebuilt, sites: &[LandingSite]) -> Vec<TypeWidth> {
        sites.iter().map(|site| rebuilt.type_width(site)).collect()
    }

    fn frames(rebuilt: &Rebuilt, ids: &[i16]) -> Vec<bool> {
        ids.iter()
            .map(|&id| rebuilt.shows_frame(StellarId(id)))
            .collect()
    }

    use TypeWidth::{Frame, OutOfRange, Unloaded};

    #[test]
    fn before_any_rebuild_no_master_is_loaded_and_every_slot_shows_a_frame() {
        let rebuilt = Rebuilt::default();
        assert_eq!(widths(&rebuilt, &[shows(128, 4, None)]), [Unloaded]);
        assert_eq!(frames(&rebuilt, &[128]), [true]);
    }

    #[test]
    fn a_rebuild_loads_exactly_the_graphics_shown() {
        let sites = [
            shows(128, 4, Some(9)),
            shows(129, 5, Some(4)),
            shows(130, 6, None),
        ];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[129]), true);
        assert_eq!(rebuilt.masters, BTreeSet::from([4, 6]), "129 shows 4");
        assert_eq!(widths(&rebuilt, &sites), [Frame, Unloaded, Frame]);
        assert_eq!(frames(&rebuilt, &[128, 129, 130]), [true; 3]);
    }

    #[test]
    fn a_destroyed_stellar_whose_dead_type_reads_frees_its_type() {
        let sites = [shows(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), true);
        assert_eq!(widths(&rebuilt, &sites), [Frame]);
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded]);
        assert_eq!(rebuilt.masters, BTreeSet::from([9]));
    }

    #[test]
    fn another_stellar_showing_the_same_graphic_keeps_it() {
        let sites = [shows(128, 4, Some(9)), shows(129, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        assert_eq!(widths(&rebuilt, &sites), [Frame, Frame]);
        // Shown as another's `DeadType` keeps it too.
        let sites = [shows(128, 4, Some(9)), shows(129, 7, Some(4))];
        rebuilt.rebuild(&sites, destroyed(&[128, 129]), true);
        assert_eq!(widths(&rebuilt, &sites), [Frame, Unloaded]);
    }

    #[test]
    fn a_type_with_no_sheet_is_never_loaded() {
        let unreadable = LandingSite {
            frame_size: None,
            ..shows(128, 4, None)
        };
        let sites = [unreadable, shows(129, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), true);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded, Frame]);
    }

    #[test]
    fn a_loaded_type_that_counts_no_frames_is_out_of_range() {
        let sites = [LandingSite {
            type_counts_frames: false,
            ..shows(128, 4, Some(9))
        }];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), true);
        assert_eq!(widths(&rebuilt, &sites), [OutOfRange]);
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded], "freed");
    }

    #[test]
    fn a_frame_that_finds_it_whole_loads_its_type_again() {
        let sites = [shows(128, 4, Some(9)), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        rebuilt.frame(&sites, destroyed(&[128]));
        assert_eq!(
            widths(&rebuilt, &sites),
            [Unloaded, Frame],
            "still destroyed"
        );
        rebuilt.frame(&sites, destroyed(&[]));
        assert_eq!(widths(&rebuilt, &sites), [Frame, Frame]);
        rebuilt.frame(&sites, destroyed(&[128]));
        assert_eq!(
            widths(&rebuilt, &sites),
            [Frame, Frame],
            "kept until a rebuild"
        );
    }

    #[test]
    fn y_u_y_with_no_frame_between_keeps_the_master_unloaded() {
        let sites = [shows(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        // `U` then `Y`: the next frame finds it destroyed again.
        rebuilt.frame(&sites, destroyed(&[128]));
        assert_eq!(widths(&rebuilt, &sites), [Unloaded]);
    }

    #[test]
    fn a_slot_left_empty_loads_graphic_type_0_when_the_fallback_reads() {
        let sites = [shows(128, 4, None), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        assert_eq!(rebuilt.masters, BTreeSet::from([0, 5]));
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true]);
        rebuilt.rebuild(&sites, destroyed(&[]), true);
        assert_eq!(rebuilt.masters, BTreeSet::from([4, 5]), "none left empty");
    }

    #[test]
    fn a_slot_left_empty_shows_no_frame_when_the_fallback_does_not_read() {
        let sites = [shows(128, 4, None), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), false);
        assert_eq!(rebuilt.masters, BTreeSet::from([5]));
        assert_eq!(frames(&rebuilt, &[128, 129]), [false, true]);
        rebuilt.rebuild(&sites, destroyed(&[]), false);
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true], "whole again");
        rebuilt.rebuild(&sites, destroyed(&[128]), false);
        rebuilt.rebuild(&sites, destroyed(&[128]), true);
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true], "filled");
    }

    #[test]
    fn a_frame_with_a_readable_graphic_fills_an_empty_slot() {
        let sites = [shows(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), false);
        rebuilt.frame(&sites, destroyed(&[128]));
        assert_eq!(frames(&rebuilt, &[128]), [false], "nothing to clone");
        rebuilt.frame(&sites, destroyed(&[]));
        assert_eq!(frames(&rebuilt, &[128]), [true]);
        rebuilt.frame(&sites, destroyed(&[128]));
        assert_eq!(frames(&rebuilt, &[128]), [true], "keeps its frames");
    }
}
