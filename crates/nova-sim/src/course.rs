//! The player's course at the rebuild points ([`rewritten`]) and after an
//! arrival ([`after_arrival`]), as
//! [`RuleKey::RouteRewrite`](crate::RuleKey::RouteRewrite) says.
//!
//! The original's course is 32 words at 0x3b51c8. Slot 0 is the system
//! last reached: a plot leaves it there, and `_SpoolNextHyperRouteDest`
//! (0x12103-0x121a2) shifts the course left once slot 1's stand-in is the
//! player's system's stand-in. Slot 1 is the next hop. The engine's
//! course is the original's slots 1 and up, so its first hop is the
//! original's slot 1.
//!
//! **Rewrite and cut** ([`rewritten`]). `_HandleStellarSystemVisibility`
//! runs it first (@0x32ab3-0x32b0d), so every rebuild point runs it. For
//! each slot, an empty one (-1) is skipped (@0x32abe); a visible hop is
//! left as it is, even when another visible system shadows it at its
//! place (the +0x1eb test @0x32ad4); a hidden hop is overwritten with the
//! system `_FindActiveCoLocatedSystem` gives (@0x32ae1-0x32ae6), its
//! [`Places::stand_in`]; and when that is -1, the slot and every later
//! slot are set to -1 (@0x32aed-0x32b02), cutting the course for good. No
//! sound plays.
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
//! **Branches left out.**
//! - The rewrite of slot 0, the system last reached (@0x32abb, %esi = 0),
//!   which the engine doesn't store: crons-news-ranks-fidelity phase 23.
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
//! - A load's rewrite reads the default rule, not the pilot's: task
//!   load-reads-rules-before-fly.
//! - The debug key's rebuild (@0x6dad1) has no counterpart.

use crate::catalog::SystemId;
use crate::hyperspace::Places;
use crate::rulebook::RuleSource;

/// `course` at a rebuild point: by the engine, each hidden hop rewritten
/// to the system at its place, and the course cut at the first hidden hop
/// with none (`_HandleStellarSystemVisibility` @0x32ab3-0x32b0d); by the
/// other reading, unchanged.
#[must_use]
pub fn rewritten(course: &[SystemId], places: &Places, source: RuleSource) -> Vec<SystemId> {
    match source {
        RuleSource::Bible => course.to_vec(),
        RuleSource::Engine => course
            .iter()
            .map_while(|&hop| {
                if places.visible(hop) {
                    Some(hop)
                } else {
                    places.stand_in(hop)
                }
            })
            .collect(),
    }
}

/// `course` after an arrival at `arrived` that made `hops` of its hops:
/// those dropped. An arrival that made none, by the engine, clears the
/// course when its first hop stands elsewhere and drops that hop when it
/// stands at `arrived` (@0x6c472-0x6c4c6); by the other reading it keeps
/// the course.
#[must_use]
pub fn after_arrival(
    course: &[SystemId],
    hops: usize,
    arrived: SystemId,
    places: &Places,
    source: RuleSource,
) -> Vec<SystemId> {
    if hops > 0 || source == RuleSource::Bible {
        return course[hops.min(course.len())..].to_vec();
    }
    match course.split_first() {
        Some((&first, rest)) if places.stand_in(first) == places.stand_in(arrived) => rest.to_vec(),
        _ => Vec::new(),
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
            rewritten(&list(&[131, 132]), &places(&[131]), RuleSource::Engine),
            list(&[134, 132])
        );
    }

    #[test]
    fn a_visible_hop_is_left_as_it_is_even_when_shadowed() {
        let places = places(&[]);
        assert_eq!(places.stand_in(SystemId(134)), Some(SystemId(131)));
        assert_eq!(
            rewritten(&list(&[134, 132]), &places, RuleSource::Engine),
            list(&[134, 132])
        );
    }

    #[test]
    fn the_first_hop_with_no_stand_in_cuts_it_and_the_rest() {
        // 132 is hidden alone at its place; 131 is hidden with visible 134.
        let hidden = places(&[132, 131]);
        assert_eq!(
            rewritten(&list(&[134, 132, 131]), &hidden, RuleSource::Engine),
            list(&[134]),
            "131 after the cut has a stand-in, but stays cut"
        );
        assert_eq!(
            rewritten(&list(&[132, 133]), &hidden, RuleSource::Engine),
            list(&[])
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
            assert_eq!(rewritten(&course, &hidden, RuleSource::Bible), course);
        }
    }

    #[test]
    fn the_hops_made_are_dropped_by_both_readings() {
        let course = list(&[132, 133, 131]);
        for source in RuleSource::ALL {
            let after = |hops| after_arrival(&course, hops, SystemId(133), &places(&[]), source);
            assert_eq!(after(1), list(&[133, 131]));
            assert_eq!(after(2), list(&[131]));
            assert_eq!(after(5), list(&[]), "clamped");
        }
    }

    #[test]
    fn an_arrival_off_the_course_clears_it_when_its_next_hop_stands_elsewhere() {
        assert_eq!(
            after_arrival(
                &list(&[133, 132]),
                0,
                SystemId(131),
                &places(&[]),
                RuleSource::Engine
            ),
            list(&[])
        );
    }

    #[test]
    fn an_arrival_off_the_course_drops_a_next_hop_standing_at_the_arrival() {
        assert_eq!(
            after_arrival(
                &list(&[131, 132, 133]),
                0,
                SystemId(134),
                &places(&[131]),
                RuleSource::Engine
            ),
            list(&[132, 133])
        );
    }

    #[test]
    fn an_arrival_off_no_course_leaves_none() {
        for source in RuleSource::ALL {
            assert_eq!(
                after_arrival(&[], 0, SystemId(131), &places(&[]), source),
                list(&[])
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
                after_arrival(&course, 0, SystemId(arrived), &hidden, RuleSource::Bible),
                course
            );
        }
    }
}
