//! The stellars shown that have been dead since the last rebuild point:
//! the original's sprite slots holding the frames its rebuild
//! (`_HandleStellarSystemVisibility` @0x32aa5) gave a destroyed stellar.
//! See the module docs' "Built by phase 9" and "Checked for phase 9".
//!
//! At a rebuild point the original frees each stellar master no stellar of
//! the player's system uses, empties every cloned slot, and clones into
//! each slot the master of the graphic its stellar now shows: `DeadType`'s
//! when destroyed (@0x32f0f-0x32f2f). The rebuild means a slot left
//! empty, as one of a destroyed stellar whose `DeadType` has no sheet is,
//! or one of a whole stellar whose `Type` has none, to get graphic type
//! 0's frames (`spïn` 1000, its fill @0x33231-0x332d0); its own code
//! faults on such a slot first (@0x33096, see the module docs' "Checked
//! for stellar-fidelity phase 3"), and the engine follows the intent.
//! From then on, the per-frame sprite tick (`_HandleStellarSprites`
//! @0x2e77b-0x2e7e5) re-clones the master of the graphic each stellar
//! shows into its slot, reloading it; one showing a graphic with no sheet
//! keeps whatever frames its slot has. So a stellar is in [`Rebuilt`]'s
//! dead set from a rebuild point that found it destroyed until a flight
//! frame finds it whole, whatever `Y` and `U` do between frames, and
//! [`Rebuilt::held`] names the sheet each slot holds.
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
//! - stellar-fidelity phase 1: the frame count a stellar animates over, the
//!   count of the sheet its slot held at the last rebuild point
//!   (@0x331c9), kept here as that sheet ([`DrawnSheet`]) and read through
//!   `Session::counted_sheet`
//!   ([`RuleKey::StellarFrameCount`](crate::RuleKey::StellarFrameCount)).
//!   The frame index is the view's, re-read on
//!   `SessionEvent::StellarsRebuilt` and laid out afresh on
//!   `SessionEvent::SystemChanged`;
//! - stellar-fidelity phase 3: the sheet each slot holds
//!   ([`Rebuilt::held`]), which a stellar showing no graphic that can be
//!   read draws, clicks and is shot by: a destroyed one with no `DeadType`
//!   sheet ([`RuleKey::DeadTypeFallback`](crate::RuleKey::DeadTypeFallback))
//!   and a whole one with no `Type` sheet
//!   ([`RuleKey::TypeSheetFallback`](crate::RuleKey::TypeSheetFallback)),
//!   read through `Session::drawn_sheet`;
//! - stellar-fidelity phase 8: whether the frames each slot holds count
//!   none, its `spïn`'s `xTiles` × `yTiles` 0 or less, so that a rebuild
//!   point leaves it with no frame and a flight frame's clone keeps the
//!   frame it showed ([`Rebuilt::showing`], [`Rebuilt::frozen`]), and
//!   whether the count the last rebuild point copied was none
//!   ([`Rebuilt::still`])
//!   ([`RuleKey::UncountedSheet`](crate::RuleKey::UncountedSheet)), read
//!   through `Session::drawn_sheet`, `Session::frame_frozen` and
//!   `Session::counts_no_frames`.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{LandingSite, StellarId};
use crate::landing::TypeWidth;
use crate::rulebook::RuleSource;

/// Which of its sheets a stellar shown draws (see the `story` module's
/// "Built by phase 11" and "Built by stellar-fidelity phase 1"): decided
/// once, by
/// `Session::drawn_sheet`, and read by the click box and the views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawnSheet {
    /// Its `Type` sheet.
    #[default]
    Type,
    /// Its `DeadType` sheet.
    Dead,
    /// Graphic type 0's sheet (`spïn` 1000), which the original's rebuild
    /// fills an empty sprite slot with.
    TypeZero,
    /// No frame: its sprite slot holds the frames of a sheet that reads
    /// but shows none of them, as a slot cloning a `spïn` whose `xTiles` ×
    /// `yTiles` is 0 or less is left from a rebuild point (see the `story`
    /// module's "Built by stellar-fidelity phase 8"). Nothing is drawn.
    NoFrame,
}

/// What graphic type 0's sheet (`spïn` 1000) gives the original's rebuild
/// when it fills a sprite slot the clone left empty (@0x332a1-0x332d0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    /// It cannot be read: the fill clones nothing, and the slot shows no
    /// frame.
    Unreadable,
    /// It reads, but its `spïn`'s `xTiles` × `yTiles` is 0 or less: the
    /// fill clones it, and the slot holds its frames and shows none
    /// (`_SWSetCurrentFrameIndex` @0xa90e7-0xa90ee, called @0x332d0).
    CountsNone,
    /// It reads and counts frames: the slot shows one.
    Counts,
}

impl Fallback {
    /// The fallback whose sheet `reads` and whose `spïn` `counts` frames.
    #[must_use]
    pub fn new(reads: bool, counts: bool) -> Self {
        match (reads, counts) {
            (false, _) => Self::Unreadable,
            (true, false) => Self::CountsNone,
            (true, true) => Self::Counts,
        }
    }
}

/// The stellar sprites as the original's last rebuild point and the
/// flight frames since left them (see the module docs): the stellars shown
/// that were destroyed at the last rebuild point and have not been seen
/// whole at a flight frame since, the graphics whose master is loaded, the
/// stellars whose slot shows no frame, the sheet whose frame count
/// each stellar shown animates over, and the sheet whose frames each
/// stellar shown's slot holds, each where that is not its `Type`; and,
/// for a `spïn` that counts no frames (stellar-fidelity phase 8), whether
/// the frames each slot holds count none, the sheet whose frame a slot
/// kept through such a clone, and the stellars whose last rebuild point
/// copied a count of none. Never saved: a load is itself a rebuild point.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rebuilt {
    dead: BTreeSet<StellarId>,
    masters: BTreeSet<i16>,
    empty: BTreeSet<StellarId>,
    counted: BTreeMap<StellarId, DrawnSheet>,
    held: BTreeMap<StellarId, DrawnSheet>,
    /// Whether the frames each slot holds count none (+0x54 ≤ 0), for the
    /// stellars shown at the last rebuild point or cloned since; one with
    /// none is read as never cloned.
    uncounted: BTreeMap<StellarId, bool>,
    /// The sheet whose frame a slot shows (+0x50), where a clone that
    /// counts none kept an earlier sheet's frame.
    stale: BTreeMap<StellarId, DrawnSheet>,
    /// The stellars whose count the last rebuild point copied
    /// (@0x331c6-0x331c9) was none.
    still: BTreeSet<StellarId>,
}

/// The graphic `site` shows, when `destroyed` says whether it is: its
/// `Type` while whole, its `DeadType` while destroyed, each only when its
/// sheet can be read, so that the original can clone its master; with
/// whether its `spïn` counts frames, which every sheet does unless
/// `honours_counts`, [`RuleKey::UncountedSheet`](crate::RuleKey::UncountedSheet)'s
/// engine reading.
fn shown_graphic(site: &LandingSite, destroyed: bool, honours_counts: bool) -> Option<(i16, bool)> {
    if destroyed {
        site.dead_graphic
            .map(|graphic| (graphic, !honours_counts || site.dead_counts_frames))
    } else {
        site.frame_size
            .map(|_| (site.graphic, !honours_counts || site.type_counts_frames))
    }
}

/// The stellars `map` marks true.
fn marked(map: &BTreeMap<StellarId, bool>) -> BTreeSet<StellarId> {
    map.iter()
        .filter(|&(_, &marked)| marked)
        .map(|(&id, _)| id)
        .collect()
}

impl Rebuilt {
    /// A rebuild point for the stellars `shown`: those that `destroyed`
    /// holds destroyed become the dead set, and every other leaves it.
    /// Every slot is emptied, then each clones the master of the graphic
    /// its stellar shows, which is loaded: it shows a frame when that
    /// graphic counts frames and none when it counts none, keeping its
    /// count. A slot whose stellar shows nothing that can be read keeps
    /// the frames it held, and the `fallback`, graphic type 0, fills it
    /// when it reads, its master loaded: the slot then holds graphic type
    /// 0's frames and shows one exactly when they count. Each stellar
    /// shown counts the frames of the sheet its slot held as its clone
    /// failed or succeeded (@0x331c9), before the fill: the clone's sheet
    /// when it shows a graphic that can be read, and otherwise what its
    /// slot held before this point, its `Type` before any, counting none
    /// when its `Type` counts none. So a stellar showing nothing that can
    /// be read counts graphic type 0's from its second rebuild point.
    /// `reading` is [`RuleKey::UncountedSheet`](crate::RuleKey::UncountedSheet)'s:
    /// by its other reading every sheet that reads counts frames. Whether
    /// the dead set, any counted or held sheet, a slot's frame or a count
    /// changed.
    pub fn rebuild(
        &mut self,
        shown: &[LandingSite],
        destroyed: impl Fn(StellarId) -> bool,
        fallback: Fallback,
        reading: RuleSource,
    ) -> bool {
        let honours_counts = reading == RuleSource::Engine;
        let fallback = match fallback {
            Fallback::CountsNone if !honours_counts => Fallback::Counts,
            fallback => fallback,
        };
        let dead: BTreeSet<StellarId> = shown
            .iter()
            .map(|site| site.id)
            .filter(|&id| destroyed(id))
            .collect();
        let mut masters = BTreeSet::new();
        let mut counted = BTreeMap::new();
        let mut held = BTreeMap::new();
        let mut uncounted = BTreeMap::new();
        let mut still = BTreeSet::new();
        let mut empty = BTreeSet::new();
        for site in shown {
            let id = site.id;
            let before = (
                self.held(id),
                self.uncounted
                    .get(&id)
                    .copied()
                    .unwrap_or(honours_counts && !site.type_counts_frames),
            );
            let clone = if dead.contains(&id) {
                DrawnSheet::Dead
            } else {
                DrawnSheet::Type
            };
            // What the slot counts as the count is copied, what it holds
            // after the fill, and whether it shows a frame.
            let (counts, holds, framed) =
                match shown_graphic(site, dead.contains(&id), honours_counts) {
                    Some((graphic, counts_frames)) => {
                        masters.insert(graphic);
                        let clone = (clone, !counts_frames);
                        (clone, clone, counts_frames)
                    }
                    None => match fallback {
                        Fallback::Unreadable => (before, before, false),
                        Fallback::CountsNone | Fallback::Counts => {
                            masters.insert(0);
                            let fills = fallback == Fallback::Counts;
                            (before, (DrawnSheet::TypeZero, !fills), fills)
                        }
                    },
                };
            if counts.0 != DrawnSheet::Type {
                counted.insert(id, counts.0);
            }
            if counts.1 {
                still.insert(id);
            }
            if holds.0 != DrawnSheet::Type {
                held.insert(id, holds.0);
            }
            uncounted.insert(id, holds.1);
            if !framed {
                empty.insert(id);
            }
        }
        let changed = dead != self.dead
            || counted != self.counted
            || held != self.held
            || empty != self.empty
            || marked(&uncounted) != marked(&self.uncounted)
            || !self.stale.is_empty()
            || still != self.still;
        self.dead = dead;
        self.masters = masters;
        self.empty = empty;
        self.counted = counted;
        self.held = held;
        self.uncounted = uncounted;
        self.stale.clear();
        self.still = still;
        changed
    }

    /// A flight frame's sprite tick over the stellars `shown`: every
    /// stellar that `destroyed` holds whole leaves the dead set, and each
    /// stellar showing a graphic that can be read has its master loaded
    /// and its frames cloned into its slot. A clone that counts frames
    /// shows one; one that counts none keeps the frame the slot showed,
    /// none if it showed none (`_SWSetCurrentFrameIndex` @0xa90e7-0xa90ee),
    /// by [`RuleKey::UncountedSheet`](crate::RuleKey::UncountedSheet)'s
    /// engine reading, `reading`. A stellar showing none keeps what its
    /// slot holds, the clone of a null master failing (@0xac448-0xac462).
    pub fn frame(
        &mut self,
        shown: &[LandingSite],
        destroyed: impl Fn(StellarId) -> bool,
        reading: RuleSource,
    ) {
        let honours_counts = reading == RuleSource::Engine;
        self.dead.retain(|&id| destroyed(id));
        for site in shown {
            let dead = destroyed(site.id);
            let Some((graphic, counts)) = shown_graphic(site, dead, honours_counts) else {
                continue;
            };
            let showing = self.showing(site.id);
            self.masters.insert(graphic);
            if dead {
                self.held.insert(site.id, DrawnSheet::Dead);
            } else {
                self.held.remove(&site.id);
            }
            self.uncounted.insert(site.id, !counts);
            if counts {
                self.empty.remove(&site.id);
                self.stale.remove(&site.id);
            } else if let Some(showing) = showing {
                self.stale.insert(site.id, showing);
            }
        }
    }

    /// The sheet whose frames `stellar`'s sprite slot holds: the clone of
    /// the last rebuild point or flight frame that could clone one, graphic
    /// type 0's where a rebuild point filled the slot with it, and its
    /// `Type` before any. A slot a rebuild point left with no frame keeps
    /// naming what it held, as the original keeps its count (+0x54); read
    /// it with [`Rebuilt::shows_frame`].
    #[must_use]
    pub fn held(&self, stellar: StellarId) -> DrawnSheet {
        self.held.get(&stellar).copied().unwrap_or(DrawnSheet::Type)
    }

    /// Whether `stellar` has been dead since the last rebuild point.
    #[must_use]
    pub fn dead(&self, stellar: StellarId) -> bool {
        self.dead.contains(&stellar)
    }

    /// The sheet whose frame count `stellar` animates over: the one its
    /// slot held at the last rebuild point, its `Type` before any.
    #[must_use]
    pub fn counted(&self, stellar: StellarId) -> DrawnSheet {
        self.counted
            .get(&stellar)
            .copied()
            .unwrap_or(DrawnSheet::Type)
    }

    /// Whether `stellar`'s sprite slot shows a frame.
    #[must_use]
    pub fn shows_frame(&self, stellar: StellarId) -> bool {
        !self.empty.contains(&stellar)
    }

    /// The sheet whose frame `stellar`'s sprite slot shows, or `None` when
    /// it shows none: the frame a clone that counts none kept, else that
    /// of the sheet it holds ([`Rebuilt::held`]).
    #[must_use]
    pub fn showing(&self, stellar: StellarId) -> Option<DrawnSheet> {
        self.shows_frame(stellar).then(|| {
            self.stale
                .get(&stellar)
                .copied()
                .unwrap_or_else(|| self.held(stellar))
        })
    }

    /// Whether the frames `stellar`'s sprite slot holds count none.
    fn uncounted(&self, stellar: StellarId) -> bool {
        self.uncounted.get(&stellar) == Some(&true)
    }

    /// Whether `stellar`'s sprite slot shows a frame frozen: the frames it
    /// holds count none, so every `_SWSetCurrentFrameIndex` refuses and
    /// the frame it showed stays, placed as a 32-pixel sprite
    /// (`_GetSpriteXSize`/`_GetSpriteYSize` @0x26d4-0x2731).
    #[must_use]
    pub fn frozen(&self, stellar: StellarId) -> bool {
        self.uncounted(stellar) && self.shows_frame(stellar)
    }

    /// Whether the count the last rebuild point copied into `stellar`
    /// (@0x331c6-0x331c9) was none, so it does not animate until the next
    /// (`_HandleStellarSprites` @0x2e802).
    #[must_use]
    pub fn still(&self, stellar: StellarId) -> bool {
        self.still.contains(&stellar)
    }

    /// Which width the original's `_GetStellarSpriteXSize` reads of
    /// `site`'s `Type` master: unloaded unless its sheet can be read and
    /// its master is loaded; out of range when its `spïn` gives no frame
    /// count; otherwise the frame's.
    #[must_use]
    pub fn type_width(&self, site: &LandingSite) -> TypeWidth {
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
    use crate::landing::{LandingReach, StellarShown, landing_reach};
    use Fallback::{Counts, CountsNone, Unreadable};
    use RuleSource::{Bible, Engine};
    use nova_sim_catalog::fixture::planet;

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
        assert!(rebuilt.rebuild(
            &ids(&[128, 129, 130]),
            destroyed(&[129, 130, 140]),
            Counts,
            Engine
        ));
        assert_eq!(
            dead(&rebuilt, &[128, 129, 130, 140]),
            [false, true, true, false],
            "140 is not shown"
        );
    }

    #[test]
    fn a_rebuild_reports_a_change_only_when_the_set_or_a_counted_sheet_changed() {
        let mut rebuilt = Rebuilt::default();
        assert!(
            !rebuilt.rebuild(&ids(&[128]), destroyed(&[]), Counts, Engine),
            "none still"
        );
        assert!(rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), Counts, Engine));
        assert!(
            rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), Counts, Engine),
            "the same set, but 128 now counts graphic type 0's frames"
        );
        assert!(
            !rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), Counts, Engine),
            "the same"
        );
        assert!(rebuilt.rebuild(&ids(&[128, 129]), destroyed(&[128, 129]), Counts, Engine));
        assert!(
            rebuilt.rebuild(&ids(&[129]), destroyed(&[128, 129]), Counts, Engine),
            "one less"
        );
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        assert!(
            rebuilt.rebuild(&ids(&[]), destroyed(&[129]), Counts, Engine),
            "emptied"
        );
        assert_eq!(dead(&rebuilt, &[129]), [false]);
    }

    #[test]
    fn a_frame_drops_a_stellar_seen_whole_and_keeps_one_still_destroyed() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&ids(&[128, 129]), destroyed(&[128, 129]), Counts, Engine);
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[129]), Engine);
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true]);
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128, 129]), Engine);
        assert_eq!(dead(&rebuilt, &[128, 129]), [false, true], "not back");
    }

    #[test]
    fn y_u_y_with_no_frame_between_keeps_it_and_a_frame_while_whole_drops_it() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&ids(&[128]), destroyed(&[128]), Counts, Engine);
        // `U` then `Y`: nothing is told until a frame, which finds it
        // destroyed again.
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128]), Engine);
        assert!(rebuilt.dead(StellarId(128)));
        // `U`, a frame while whole, then `Y`.
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[]), Engine);
        rebuilt.frame(&ids(&[128, 129]), destroyed(&[128]), Engine);
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
        rebuilt.rebuild(&sites, destroyed(&[129]), Counts, Engine);
        assert_eq!(rebuilt.masters, BTreeSet::from([4, 6]), "129 shows 4");
        assert_eq!(widths(&rebuilt, &sites), [Frame, Unloaded, Frame]);
        assert_eq!(frames(&rebuilt, &[128, 129, 130]), [true; 3]);
    }

    #[test]
    fn a_destroyed_stellar_whose_dead_type_reads_frees_its_type() {
        let sites = [shows(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [Frame]);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded]);
        assert_eq!(rebuilt.masters, BTreeSet::from([9]));
    }

    #[test]
    fn another_stellar_showing_the_same_graphic_keeps_it() {
        let sites = [shows(128, 4, Some(9)), shows(129, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [Frame, Frame]);
        // Shown as another's `DeadType` keeps it too.
        let sites = [shows(128, 4, Some(9)), shows(129, 7, Some(4))];
        rebuilt.rebuild(&sites, destroyed(&[128, 129]), Counts, Engine);
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
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded, Frame]);
    }

    #[test]
    fn a_loaded_type_that_counts_no_frames_is_out_of_range() {
        let sites = [LandingSite {
            type_counts_frames: false,
            ..shows(128, 4, Some(9))
        }];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [OutOfRange]);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded], "freed");
    }

    #[test]
    fn a_frame_that_finds_it_whole_loads_its_type_again() {
        let sites = [shows(128, 4, Some(9)), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(
            widths(&rebuilt, &sites),
            [Unloaded, Frame],
            "still destroyed"
        );
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(widths(&rebuilt, &sites), [Frame, Frame]);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
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
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        // `U` then `Y`: the next frame finds it destroyed again.
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(widths(&rebuilt, &sites), [Unloaded]);
    }

    #[test]
    fn a_slot_left_empty_loads_graphic_type_0_when_the_fallback_reads() {
        let sites = [shows(128, 4, None), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(rebuilt.masters, BTreeSet::from([0, 5]));
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true]);
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(rebuilt.masters, BTreeSet::from([4, 5]), "none left empty");
    }

    #[test]
    fn a_slot_left_empty_shows_no_frame_when_the_fallback_does_not_read() {
        let sites = [shows(128, 4, None), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        assert_eq!(rebuilt.masters, BTreeSet::from([5]));
        assert_eq!(frames(&rebuilt, &[128, 129]), [false, true]);
        rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine);
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true], "whole again");
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true], "filled");
    }

    #[test]
    fn a_frame_with_a_readable_graphic_fills_an_empty_slot() {
        let sites = [shows(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false], "nothing to clone");
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(frames(&rebuilt, &[128]), [true]);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(frames(&rebuilt, &[128]), [true], "keeps its frames");
    }

    // The counted sheet (stellar-fidelity phase 1).

    fn counted(rebuilt: &Rebuilt, ids: &[i16]) -> Vec<DrawnSheet> {
        ids.iter()
            .map(|&id| rebuilt.counted(StellarId(id)))
            .collect()
    }

    use DrawnSheet::{Dead, Type, TypeZero};

    #[test]
    fn before_any_rebuild_every_stellar_counts_its_type() {
        assert_eq!(counted(&Rebuilt::default(), &[128]), [Type]);
    }

    #[test]
    fn a_rebuild_counts_a_whole_stellars_type_and_a_destroyed_ones_dead_type() {
        let sites = [shows(128, 4, Some(9)), shows(129, 5, Some(8))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[129]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128, 129]), [Type, Dead]);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128, 129]), [Dead, Type]);
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_counts_graphic_type_0_from_its_second_rebuild() {
        let sites = [shows(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type], "its slot held Type");
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [TypeZero], "graphic type 0's");
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type], "whole again");
    }

    #[test]
    fn with_graphic_type_0_unreadable_it_counts_its_type() {
        let sites = [shows(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type]);
    }

    #[test]
    fn a_stellar_seen_whole_since_counts_its_type_again() {
        let sites = [shows(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        // `U`, a frame while whole, then `Y`: the frame cloned `Type` back.
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(counted(&rebuilt, &[128]), [TypeZero], "until a rebuild");
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type]);
    }

    #[test]
    fn a_stellar_no_longer_shown_counts_its_type() {
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&[shows(128, 4, Some(9))], destroyed(&[128]), Counts, Engine);
        rebuilt.rebuild(&[shows(129, 5, None)], destroyed(&[128]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type]);
    }

    // What a slot holds (stellar-fidelity phase 3).

    /// Planet `id` of graphic type `graphic`, whose `Type` sheet cannot be
    /// read, showing `dead_graphic` while destroyed.
    fn unreadable(id: i16, graphic: i16, dead_graphic: Option<i16>) -> LandingSite {
        LandingSite {
            frame_size: None,
            ..shows(id, graphic, dead_graphic)
        }
    }

    fn held(rebuilt: &Rebuilt, ids: &[i16]) -> Vec<DrawnSheet> {
        ids.iter().map(|&id| rebuilt.held(StellarId(id))).collect()
    }

    #[test]
    fn a_whole_stellar_with_no_type_sheet_holds_graphic_type_0_from_a_rebuild() {
        let sites = [unreadable(128, 4, None), shows(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        assert_eq!(held(&rebuilt, &[128, 129]), [Type, Type], "before any");
        assert!(rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine));
        assert_eq!(held(&rebuilt, &[128, 129]), [TypeZero, Type]);
        assert_eq!(rebuilt.masters, BTreeSet::from([0, 5]));
        assert_eq!(frames(&rebuilt, &[128, 129]), [true, true]);
    }

    #[test]
    fn frames_while_it_is_whole_keep_graphic_type_0() {
        let sites = [unreadable(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(held(&rebuilt, &[128]), [TypeZero], "the clone fails");
        assert_eq!(rebuilt.masters, BTreeSet::from([0]));
    }

    #[test]
    fn a_frame_that_finds_it_destroyed_with_a_dead_type_holds_that_and_keeps_it_after_u() {
        let sites = [unreadable(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        // `Y`, then a frame.
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(held(&rebuilt, &[128]), [Dead]);
        // `U`, then frames while whole.
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(held(&rebuilt, &[128]), [Dead], "the `Type` clone fails");
    }

    #[test]
    fn a_rebuild_puts_graphic_type_0_back() {
        let sites = [unreadable(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert!(
            rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine),
            "what its slot holds changed"
        );
        assert_eq!(held(&rebuilt, &[128]), [TypeZero]);
    }

    #[test]
    fn a_rebuild_finding_it_destroyed_holds_its_dead_type_and_drops_one_no_longer_shown() {
        let sites = [unreadable(128, 4, Some(9)), unreadable(129, 5, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(held(&rebuilt, &[128, 129]), [Dead, TypeZero]);
        rebuilt.rebuild(&sites[..1], destroyed(&[128]), Counts, Engine);
        assert_eq!(held(&rebuilt, &[128, 129]), [Dead, Type], "129 not shown");
        let readable = [shows(128, 4, Some(9))];
        rebuilt.rebuild(&readable, destroyed(&[]), Counts, Engine);
        assert_eq!(held(&rebuilt, &[128]), [Type], "its `Type` cloned in");
    }

    #[test]
    fn with_graphic_type_0_unreadable_its_slot_shows_no_frame_and_keeps_what_it_held() {
        let sites = [unreadable(128, 4, Some(9))];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false]);
        assert_eq!(held(&rebuilt, &[128]), [Type]);
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(frames(&rebuilt, &[128]), [true]);
        assert_eq!(held(&rebuilt, &[128]), [Dead]);
        rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false]);
        assert_eq!(held(&rebuilt, &[128]), [Dead], "the original keeps +0x54");
        assert_eq!(counted(&rebuilt, &[128]), [Dead]);
    }

    #[test]
    fn a_whole_stellar_with_no_type_sheet_counts_graphic_type_0_from_its_second_rebuild() {
        let sites = [unreadable(128, 4, None)];
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(counted(&rebuilt, &[128]), [Type], "its slot held Type");
        assert!(
            rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine),
            "now counts graphic type 0's"
        );
        assert_eq!(counted(&rebuilt, &[128]), [TypeZero]);
        assert!(
            !rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine),
            "the same"
        );
    }

    /// Every sequence of up to `steps` steps, each a flight frame or a
    /// rebuild point, with the stellar destroyed or whole at each.
    fn walks(steps: u32) -> impl Iterator<Item = Vec<(bool, bool)>> {
        (1..=steps).flat_map(|len| {
            (0..4u32.pow(len)).map(move |code| {
                (0..len)
                    .map(|i| {
                        let step = code / 4u32.pow(i) % 4;
                        (step & 1 == 1, step & 2 == 2)
                    })
                    .collect()
            })
        })
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_holds_graphic_type_0_exactly_while_dead() {
        let sites = [shows(128, 4, None)];
        let me = StellarId(128);
        for fallback in [Counts, Unreadable] {
            let fallback_reads = fallback == Counts;
            for walk in walks(5) {
                let mut rebuilt = Rebuilt::default();
                for &(is_rebuild, is_destroyed) in &walk {
                    let state = destroyed(if is_destroyed { &[128] } else { &[] });
                    if is_rebuild {
                        rebuilt.rebuild(&sites, state, fallback, Engine);
                    } else {
                        rebuilt.frame(&sites, state, Engine);
                    }
                    if fallback_reads {
                        assert_eq!(rebuilt.held(me) == TypeZero, rebuilt.dead(me), "{walk:?}");
                    } else {
                        assert_eq!(!rebuilt.shows_frame(me), rebuilt.dead(me), "{walk:?}");
                        assert_eq!(rebuilt.held(me), Type, "{walk:?}");
                    }
                }
            }
        }
    }

    // A `spïn` that counts no frames (stellar-fidelity phase 8).

    /// Planet `id` of graphic type 4, showing graphic type 9 while
    /// destroyed, whose `Type` counts frames when `type_counts` and whose
    /// `DeadType` does when `dead_counts`.
    fn counting(id: i16, type_counts: bool, dead_counts: bool) -> LandingSite {
        LandingSite {
            type_counts_frames: type_counts,
            dead_counts_frames: dead_counts,
            ..shows(id, 4, Some(9))
        }
    }

    /// The landing reach the original's dock request gives `site` by what
    /// `rebuilt` left of its slot and its `Type` master.
    fn reach(rebuilt: &Rebuilt, site: &LandingSite) -> LandingReach {
        let shown = StellarShown {
            frame: rebuilt.shows_frame(site.id),
            type_width: rebuilt.type_width(site),
        };
        landing_reach(site, shown, Engine)
    }

    #[test]
    fn a_rebuild_point_showing_a_type_that_counts_no_frames_leaves_its_slot_frameless_then_a_frame_leaves_it_so()
     {
        let site = counting(128, false, true);
        let sites = [site];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false]);
        assert_eq!(widths(&rebuilt, &sites), [OutOfRange]);
        assert_eq!(reach(&rebuilt, &site), LandingReach::Box(75.0));
        assert_eq!(held(&rebuilt, &[128]), [Type]);
        assert_eq!(rebuilt.showing(me), None);
        assert!(rebuilt.still(me));
        assert!(!rebuilt.frozen(me), "it shows no frame to freeze");
        assert_eq!(rebuilt.masters, BTreeSet::from([4]), "no fill");
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false], "still frameless");
        assert_eq!(reach(&rebuilt, &site), LandingReach::Box(75.0));
        assert!(rebuilt.still(me));
    }

    #[test]
    fn a_frame_cloning_an_uncounted_type_keeps_the_frame_the_slot_shows() {
        let site = counting(128, false, true);
        let sites = [site];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        // `Y`, then a frame: its `DeadType` counts, so the slot shows it.
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(rebuilt.showing(me), Some(Dead));
        assert!(!rebuilt.frozen(me));
        // `U`, then a frame: the `Type` clone keeps the `DeadType` frame.
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(rebuilt.showing(me), Some(Dead));
        assert!(rebuilt.frozen(me));
        assert_eq!(held(&rebuilt, &[128]), [Type]);
        assert_eq!(widths(&rebuilt, &sites), [OutOfRange]);
        assert_eq!(reach(&rebuilt, &site), LandingReach::Box(56.0));
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(rebuilt.showing(me), Some(Dead), "kept, not replaced");
        // `Y`, then a frame: it counts again.
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert!(!rebuilt.frozen(me));
        assert_eq!(rebuilt.showing(me), Some(Dead));
        assert!(rebuilt.stale.is_empty());
        // `U`, a frame, then a rebuild point while destroyed.
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert!(rebuilt.frozen(me));
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert!(rebuilt.stale.is_empty(), "a rebuild point clears it");
        assert_eq!(rebuilt.showing(me), Some(Dead));
        assert!(!rebuilt.frozen(me));
    }

    #[test]
    fn a_frame_cloning_an_uncounted_graphic_into_an_empty_slot_leaves_it_empty() {
        let sites = [counting(128, false, true)];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(rebuilt.showing(me), None);
        assert!(!rebuilt.frozen(me));
        assert!(rebuilt.stale.is_empty());
    }

    #[test]
    fn a_frame_cloning_an_uncounted_graphic_keeps_its_types_frame_before_any_rebuild() {
        let sites = [counting(128, true, false)];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.frame(&sites, destroyed(&[128]), Engine);
        assert_eq!(rebuilt.showing(me), Some(Type));
        assert!(rebuilt.frozen(me));
        assert_eq!(held(&rebuilt, &[128]), [Dead]);
        assert_eq!(rebuilt.masters, BTreeSet::from([9]));
    }

    #[test]
    fn a_dead_type_that_counts_no_frames_leaves_a_destroyed_stellars_slot_frameless_at_a_rebuild_point()
     {
        let sites = [counting(128, true, false)];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert_eq!(frames(&rebuilt, &[128]), [false]);
        assert_eq!(held(&rebuilt, &[128]), [Dead]);
        assert_eq!(counted(&rebuilt, &[128]), [Dead]);
        assert!(rebuilt.still(me));
        assert_eq!(rebuilt.masters, BTreeSet::from([9]), "no fill");
        // `U`, then a frame: its `Type` counts and fills the slot.
        rebuilt.frame(&sites, destroyed(&[]), Engine);
        assert_eq!(rebuilt.showing(me), Some(Type));
        assert!(rebuilt.still(me), "until the next rebuild point");
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert!(!rebuilt.still(me));
    }

    #[test]
    fn graphic_type_0_that_counts_no_frames_fills_the_slot_but_shows_no_frame() {
        let me = StellarId(128);
        for (sites, ruined) in [
            ([unreadable(128, 4, None)], &[][..]),
            ([shows(128, 4, None)], &[128][..]),
        ] {
            let mut rebuilt = Rebuilt::default();
            rebuilt.rebuild(&sites, destroyed(ruined), CountsNone, Engine);
            assert!(rebuilt.masters.contains(&0), "loaded by the fill");
            assert_eq!(held(&rebuilt, &[128]), [TypeZero]);
            assert!(rebuilt.uncounted(me));
            assert_eq!(frames(&rebuilt, &[128]), [false]);
            assert!(!rebuilt.still(me), "the fill comes after the count");
            rebuilt.rebuild(&sites, destroyed(ruined), CountsNone, Engine);
            assert!(rebuilt.still(me), "graphic type 0's count from the second");
        }
    }

    #[test]
    fn a_failed_clone_at_a_rebuild_point_counts_what_the_slot_held() {
        // `Type` counts none; no `DeadType` sheet.
        let sites = [LandingSite {
            type_counts_frames: false,
            ..shows(128, 4, None)
        }];
        let me = StellarId(128);
        let mut rebuilt = Rebuilt::default();
        rebuilt.rebuild(&sites, destroyed(&[]), Counts, Engine);
        assert!(rebuilt.still(me));
        rebuilt.rebuild(&sites, destroyed(&[128]), Unreadable, Engine);
        assert!(rebuilt.still(me), "it held the uncounted `Type`");
        assert_eq!(held(&rebuilt, &[128]), [Type]);
        assert_eq!(frames(&rebuilt, &[128]), [false]);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert!(rebuilt.still(me), "the fill comes after the count");
        assert_eq!(held(&rebuilt, &[128]), [TypeZero]);
        assert_eq!(frames(&rebuilt, &[128]), [true]);
        rebuilt.rebuild(&sites, destroyed(&[128]), Counts, Engine);
        assert!(!rebuilt.still(me), "graphic type 0 counts");
    }

    #[test]
    fn a_stellar_never_cloned_counts_none_when_its_type_counts_none() {
        let me = StellarId(128);
        for (type_counts, still) in [(false, true), (true, false)] {
            let sites = [LandingSite {
                type_counts_frames: type_counts,
                ..unreadable(128, 4, None)
            }];
            let mut rebuilt = Rebuilt::default();
            rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine);
            assert_eq!(rebuilt.still(me), still, "{type_counts}");
            let mut rebuilt = Rebuilt::default();
            rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Bible);
            assert!(!rebuilt.still(me), "by the other reading");
        }
    }

    #[test]
    fn a_rebuild_point_reports_a_change_in_the_slots_frame_or_count() {
        let mut rebuilt = Rebuilt::default();
        let uncounted = [counting(128, false, true)];
        assert!(
            rebuilt.rebuild(&uncounted, destroyed(&[]), Counts, Engine),
            "its slot shows no frame and counts none"
        );
        assert!(!rebuilt.rebuild(&uncounted, destroyed(&[]), Counts, Engine));
        let counted = [counting(128, true, true)];
        assert!(rebuilt.rebuild(&counted, destroyed(&[]), Counts, Engine));
        assert!(!rebuilt.rebuild(&counted, destroyed(&[]), Counts, Engine));
        // Only the frame: graphic type 0 cannot fill a `Type` that cannot
        // be read.
        let mut rebuilt = Rebuilt::default();
        let sites = [unreadable(128, 4, None)];
        assert!(rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine));
        assert!(!rebuilt.rebuild(&sites, destroyed(&[]), Unreadable, Engine));
    }

    #[test]
    fn by_the_other_reading_an_uncounted_sheet_counts() {
        let me = StellarId(128);
        for fallback in [Counts, CountsNone, Unreadable] {
            let counts = if fallback == CountsNone {
                Counts
            } else {
                fallback
            };
            for (type_counts, dead_counts, dead) in [
                (false, false, Some(9)),
                (false, true, Some(9)),
                (true, false, Some(9)),
                (false, true, None),
            ] {
                let uncounted = [LandingSite {
                    type_counts_frames: type_counts,
                    dead_counts_frames: dead_counts,
                    ..shows(128, 4, dead)
                }];
                let counting = [LandingSite {
                    type_counts_frames: true,
                    dead_counts_frames: true,
                    ..shows(128, 4, dead)
                }];
                for walk in walks(4) {
                    let mut bible = Rebuilt::default();
                    let mut today = Rebuilt::default();
                    for &(is_rebuild, is_destroyed) in &walk {
                        let ruined: &[i16] = if is_destroyed { &[128] } else { &[] };
                        if is_rebuild {
                            bible.rebuild(&uncounted, destroyed(ruined), fallback, Bible);
                            today.rebuild(&counting, destroyed(ruined), counts, Engine);
                        } else {
                            bible.frame(&uncounted, destroyed(ruined), Bible);
                            today.frame(&counting, destroyed(ruined), Engine);
                        }
                        assert_eq!(bible, today, "{walk:?}");
                        assert!(!bible.frozen(me) && !bible.still(me), "{walk:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn an_uncounted_type_freezes_a_frame_exactly_while_found_whole_and_empties_from_a_rebuild() {
        let sites = [counting(128, false, true)];
        let me = StellarId(128);
        for reading in [Engine, Bible] {
            for walk in walks(5) {
                let mut rebuilt = Rebuilt::default();
                // Whether the last rebuild point found it whole and no
                // frame has found it destroyed since.
                let mut emptied = false;
                for &(is_rebuild, is_destroyed) in &walk {
                    let state = destroyed(if is_destroyed { &[128] } else { &[] });
                    if is_rebuild {
                        rebuilt.rebuild(&sites, state, Counts, reading);
                        emptied = !is_destroyed;
                    } else {
                        rebuilt.frame(&sites, state, reading);
                        emptied &= !is_destroyed;
                    }
                    if reading == Engine {
                        assert_eq!(
                            rebuilt.frozen(me),
                            !is_destroyed && rebuilt.shows_frame(me),
                            "{walk:?}"
                        );
                        assert_eq!(!rebuilt.shows_frame(me), emptied, "{walk:?}");
                    } else {
                        assert!(rebuilt.shows_frame(me), "{walk:?}");
                        assert!(!rebuilt.frozen(me), "{walk:?}");
                    }
                }
            }
        }
    }
}
