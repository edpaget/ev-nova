//! The player's course at the rebuild points ([`rewritten`]) and after an
//! arrival ([`after_arrival`]), as
//! [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says.
//!
//! The original's course is 32 words at 0x3b51c8. Slot 0 is the system
//! the course was last started or advanced from: Clear Route and the gate
//! exits restart it there, a Shift-click's append sets it when it is -1,
//! and `_SpoolNextHyperRouteDest` (0x12103-0x121a2) shifts the course left
//! once slot 1's stand-in is the player's system's stand-in. Only `M` and
//! `N` (@0x1570e-0x159ad) move the player without touching it. Slot 1 is
//! the next hop. The engine keeps slot 0 as `Pilot::course_start` and the
//! original's slots 1 and up as `Pilot::course`, so its first hop is the
//! original's slot 1. How a Shift-click on the course map writes the
//! course is the [`map_route`](crate::map_route) module's.
//!
//! **Rewrite and cut** ([`rewritten`]). `_HandleStellarSystemVisibility`
//! runs it first (@0x32aaa-0x32b0d), so every rebuild point runs it. It
//! starts at slot 0, the course's start (%esi = 0 @0x32aaa). For each
//! slot, an empty one (-1) is skipped (@0x32abe); a visible hop is
//! left as it is, even when another visible system shadows it at its
//! place (the +0x1eb test @0x32ad4); a hidden hop is overwritten with the
//! system `_FindActiveCoLocatedSystem` gives (@0x32ae1-0x32ae6), its
//! [`Places::stand_in`]; and when that is -1, the slot and every later
//! slot are set to -1 (@0x32aed-0x32b02), cutting the course for good.
//! A start hidden with no stand-in so cuts the whole course, start and
//! hops; this is reachable only after an `M` or `N` keeps a course whose
//! start later hides, as every other writer leaves the start at the
//! player's place, which always has a visible system. A hidden start's
//! stand-in shares its place, so rewriting it is unobservable, but it is
//! done. No sound plays.
//!
//! **Arrival clear** ([`after_arrival`], @0x6c472-0x6c4c6). The course is
//! cleared only when slot 1 is set, the jump flag at -0x2c9(%ebp) is
//! clear, and slot 1's stand-in differs from the player's system's. The
//! flag is set @0x6be01 when the jump went to the course's next hop on a
//! multi-jump ship, and cleared @0x6be85 otherwise: no course, a jump off
//! the course, or a one-jump ship. Then `_SpoolNextHyperRouteDest` runs
//! (@0x6c4c8). A one-jump ship that flew the course finds slot 1 standing
//! at the arrival, so the hop is dropped and nothing is cleared; a jump
//! off the course that finds slot 1 standing at the arrival (only when
//! the arrival's galaxy refresh @0x6bf3f moves stand-ins between the
//! jump's decision @0x6bd67-0x6bd83 and the clear) drops that hop
//! instead. In engine terms, an arrival that made hops drops them, and
//! one that made none clears the course when its first hop stands
//! elsewhere, and drops that hop when it stands at the arrival. No sound
//! plays.
//!
//! By the other reading the course keeps its IDs: each hop is resolved by
//! place only when it is used, and an arrival drops only the hops made.
//! Between rebuild points both readings resolve each hop by place when it
//! is used (`_CueNextHyperRouteDest` @0xe2fa, the arrival's
//! `_FindActiveCoLocatedSystem` @0x6bd67 and @0x6bdc1), so the rewrite
//! only makes that resolution stick at each rebuild point. The move out of
//! a hidden system (@0x998f1-0x99a36) doesn't touch the course.
//!
//! An arrival leaves the start at the last hop made, raw, and a course
//! with no hops left has no start, as `_SpoolNextHyperRouteDest` and the
//! arrival clear leave slot 0.
//!
//! **Branches left out.**
//! - `_DrawMap` draws the course from slot 0 (@0xe661-0xe7a2), where the
//!   engine's map draws it from the current system:
//!   crons-news-ranks-fidelity phase 39.
//! - A multi-jump hop with no stand-in partway through the chain
//!   (@0x6bdc6-0x6bdcc → 0x6bdfe → 0x6be0b): the original writes system
//!   -1 as the player's system. The engine stops the chain at the previous
//!   hop, on purpose.
//! - Raw-link readers. `_ShowFloatingMap` preselects the hyperspace
//!   target's raw Con link (@0x1148f-0x114c8), only when that system is
//!   visible (@0x114be), without resolving it by place; the engine has no
//!   floating map. `_RecursiveEscapeStellarSearch` follows raw Con links
//!   (@0xd13e, @0xd230) to systems that are visible (+0x1eb @0xd159,
//!   @0xd247), known (+0x1ec @0xd166, @0xd250) and explored (+0x90 > 0
//!   @0xd173-0xd17b, @0xd259-0xd261), and needs the stellar active (+0x44
//!   @0xd1b2): player-death phase 9, whose rebuild point (@0x69135) runs
//!   the rewrite through the session's rebuild.
//! - The debug key's rebuild (@0x6dad1) has no counterpart.

use crate::catalog::SystemId;
use crate::hyperspace::Places;
use crate::rulebook::RuleSource;

/// The course's start and hops at a rebuild point. By the engine
/// (`_HandleStellarSystemVisibility` @0x32aaa-0x32b0d), an empty `start`
/// is skipped, a visible one kept, and a hidden one rewritten to the
/// system at its place; a hidden start with none empties the whole
/// course. Then each hidden hop is rewritten to the system at its place,
/// and the hops are cut at the first hidden one with none. By the other
/// reading both come back unchanged.
#[must_use]
pub fn rewritten(
    start: Option<SystemId>,
    course: &[SystemId],
    places: &Places,
    source: RuleSource,
) -> (Option<SystemId>, Vec<SystemId>) {
    let rewrite = |system: SystemId| {
        if places.visible(system) {
            Some(system)
        } else {
            places.stand_in(system)
        }
    };
    match source {
        RuleSource::Bible => (start, course.to_vec()),
        RuleSource::Engine => match start.map(rewrite) {
            Some(None) => (None, Vec::new()),
            start => (
                start.flatten(),
                course.iter().map_while(|&hop| rewrite(hop)).collect(),
            ),
        },
    }
}

/// The course's start and hops after an arrival at `arrived` that made
/// `hops` of its hops: those dropped, and the start the last one made,
/// raw (`_SpoolNextHyperRouteDest` @0x12103-0x121a2). An arrival that
/// made none, by the engine, clears the course when its first hop stands
/// elsewhere and drops that hop, raw, into the start when it stands at
/// `arrived` (@0x6c472-0x6c4c6); by the other reading it keeps both. A
/// course with no hops left has no start, by both readings, as the
/// original empties all 32 slots when slot 1 is -1.
#[must_use]
pub fn after_arrival(
    start: Option<SystemId>,
    course: &[SystemId],
    hops: usize,
    arrived: SystemId,
    places: &Places,
    source: RuleSource,
) -> (Option<SystemId>, Vec<SystemId>) {
    let (start, rest) = if hops > 0 {
        let made = hops.min(course.len());
        (
            made.checked_sub(1)
                .and_then(|last| course.get(last))
                .copied(),
            &course[made..],
        )
    } else if source == RuleSource::Bible {
        (start, course)
    } else {
        match course.split_first() {
            Some((&first, rest)) if places.stand_in(first) == places.stand_in(arrived) => {
                (Some(first), rest)
            }
            _ => (None, &[][..]),
        }
    };
    if rest.is_empty() {
        (None, Vec::new())
    } else {
        (start, rest.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::hyperspace::CoLocated;
    use crate::testkit::star;

    fn list(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// 131 and 134 at one place, 132 and 133 each at its own, with
    /// `hidden` hidden.
    fn places(hidden: &[i16]) -> Places {
        Places::new(
            &[
                star(131, (1.0, 1.0), &[]),
                star(132, (2.0, 2.0), &[]),
                star(133, (3.0, 3.0), &[]),
                star(134, (1.0, 1.0), &[]),
            ],
            &hidden
                .iter()
                .copied()
                .map(SystemId)
                .collect::<BTreeSet<_>>(),
            CoLocated::Lowest,
        )
    }

    #[test]
    fn a_hidden_hop_is_rewritten_to_the_system_at_its_place() {
        assert_eq!(
            rewritten(
                None,
                &list(&[131, 132]),
                &places(&[131]),
                RuleSource::Engine
            ),
            (None, list(&[134, 132]))
        );
    }

    #[test]
    fn a_visible_hop_is_left_as_it_is_even_when_shadowed() {
        let places = places(&[]);
        assert_eq!(places.stand_in(SystemId(134)), Some(SystemId(131)));
        assert_eq!(
            rewritten(None, &list(&[134, 132]), &places, RuleSource::Engine),
            (None, list(&[134, 132]))
        );
    }

    #[test]
    fn the_first_hop_with_no_stand_in_cuts_it_and_the_rest() {
        // 132 is hidden alone at its place; 131 is hidden with visible 134.
        let hidden = places(&[132, 131]);
        assert_eq!(
            rewritten(None, &list(&[134, 132, 131]), &hidden, RuleSource::Engine),
            (None, list(&[134])),
            "131 after the cut has a stand-in, but stays cut"
        );
        assert_eq!(
            rewritten(None, &list(&[132, 133]), &hidden, RuleSource::Engine),
            (None, list(&[]))
        );
    }

    #[test]
    fn by_the_bible_the_course_is_kept() {
        for (course, hidden) in [
            (list(&[131, 132]), places(&[131])),
            (list(&[134, 132]), places(&[])),
            (list(&[134, 132, 131]), places(&[132, 131])),
            (list(&[132, 133]), places(&[132])),
        ] {
            assert_eq!(
                rewritten(Some(SystemId(132)), &course, &hidden, RuleSource::Bible),
                (Some(SystemId(132)), course)
            );
        }
    }

    #[test]
    fn the_hops_made_are_dropped_by_both_readings() {
        let course = list(&[132, 133, 131]);
        for source in RuleSource::ALL {
            let after =
                |hops| after_arrival(None, &course, hops, SystemId(133), &places(&[]), source);
            assert_eq!(after(1), (Some(SystemId(132)), list(&[133, 131])));
            assert_eq!(after(2), (Some(SystemId(133)), list(&[131])));
            assert_eq!(after(3), (None, list(&[])), "the course finished");
            assert_eq!(after(5), (None, list(&[])), "clamped");
        }
    }

    #[test]
    fn an_arrival_off_the_course_clears_it_when_its_next_hop_stands_elsewhere() {
        assert_eq!(
            after_arrival(
                None,
                &list(&[133, 132]),
                0,
                SystemId(131),
                &places(&[]),
                RuleSource::Engine
            ),
            (None, list(&[]))
        );
    }

    #[test]
    fn an_arrival_off_the_course_drops_a_next_hop_standing_at_the_arrival() {
        assert_eq!(
            after_arrival(
                None,
                &list(&[131, 132, 133]),
                0,
                SystemId(134),
                &places(&[131]),
                RuleSource::Engine
            ),
            (Some(SystemId(131)), list(&[132, 133])),
            "the start is the hop dropped, raw"
        );
    }

    #[test]
    fn an_arrival_off_no_course_leaves_none() {
        for source in RuleSource::ALL {
            assert_eq!(
                after_arrival(None, &[], 0, SystemId(131), &places(&[]), source),
                (None, list(&[]))
            );
        }
    }

    #[test]
    fn by_the_bible_an_arrival_off_the_course_keeps_it() {
        for (course, arrived, hidden) in [
            (list(&[133, 132]), 131, places(&[])),
            (list(&[131, 132, 133]), 134, places(&[131])),
        ] {
            assert_eq!(
                after_arrival(
                    Some(SystemId(133)),
                    &course,
                    0,
                    SystemId(arrived),
                    &hidden,
                    RuleSource::Bible
                ),
                (Some(SystemId(133)), course)
            );
        }
    }

    #[test]
    fn a_hidden_start_with_no_stand_in_empties_the_whole_course() {
        // 132 is hidden alone at its place; every hop is visible.
        assert_eq!(
            rewritten(
                Some(SystemId(132)),
                &list(&[133, 131]),
                &places(&[132]),
                RuleSource::Engine
            ),
            (None, list(&[]))
        );
    }

    #[test]
    fn a_hidden_start_with_a_stand_in_becomes_it_and_keeps_the_hops() {
        assert_eq!(
            rewritten(
                Some(SystemId(131)),
                &list(&[132, 133]),
                &places(&[131]),
                RuleSource::Engine
            ),
            (Some(SystemId(134)), list(&[132, 133]))
        );
    }

    #[test]
    fn a_visible_start_is_kept_even_when_shadowed() {
        for start in [131, 134] {
            assert_eq!(
                rewritten(
                    Some(SystemId(start)),
                    &list(&[132]),
                    &places(&[]),
                    RuleSource::Engine
                ),
                (Some(SystemId(start)), list(&[132]))
            );
        }
    }

    #[test]
    fn an_empty_start_is_skipped_and_the_hops_are_rewritten() {
        assert_eq!(
            rewritten(
                None,
                &list(&[131, 132]),
                &places(&[131]),
                RuleSource::Engine
            ),
            (None, list(&[134, 132]))
        );
    }

    #[test]
    fn by_the_bible_a_hidden_start_is_kept() {
        assert_eq!(
            rewritten(
                Some(SystemId(132)),
                &list(&[133]),
                &places(&[132]),
                RuleSource::Bible
            ),
            (Some(SystemId(132)), list(&[133]))
        );
    }

    #[test]
    fn an_arrival_off_the_course_clears_its_start_too() {
        assert_eq!(
            after_arrival(
                Some(SystemId(132)),
                &list(&[133, 132]),
                0,
                SystemId(131),
                &places(&[]),
                RuleSource::Engine
            ),
            (None, list(&[]))
        );
    }
}
