//! How a Shift-click on the in-flight course map writes the player's
//! course, as `_DoSystemMap` takes item 3 with Shift held
//! (@0x1325e-0x136d2), by [`RuleKey::MapPlotting`]'s engine reading
//! ([`shift_click`]). All addresses are in `EV Nova.app`'s `EV Nova`
//! (i386).
//!
//! The original's course is 32 words, slot 0 the system last reached and
//! slots 1-31 the hops; the engine's course is slots 1-31 (see the
//! [`course`](crate::course) module), and slot 0 is the course's start
//! (`Pilot::course_start`), which a click keeps, clears or sets as
//! [`RouteEdit::start`] says.
//! A "place" compares `_FindFirstCoLocatedSystem`, the place's head
//! ([`Places::head`]), a system not in the catalog being its own. In
//! order:
//!
//! 1. **The tidy**, the inlined `_CompactHyperRoute` (@0x13280-0x132c2):
//!    with slot 0 set the shift does nothing, and the clear when slot 1 is
//!    −1 is the empty course already, so on the engine's course it is a
//!    no-op.
//! 2. **The current system's place.** A pick there sets all 32 slots to −1
//!    (@0x132ec-0x13306), the start included, and the click goes on: the
//!    cut finds nothing, so the source is the current system.
//! 3. **The cut.** The first of slots 1-31, up to the first −1, at the
//!    pick's place, and every later slot, are set to −1 (@0x13310-0x1335c).
//!    The steps below run on the cut course, so the pick is put back when
//!    its new last hop links to it: a Shift-click on the course's last hop
//!    leaves the course as it was.
//! 4. **The length and the source.** With every slot full the length is 1
//!    (@0x1338e-0x133ab), so the source is the current system and the
//!    pick overwrites slot 1, keeping slots 2-31: the 32-slot wrap. With
//!    no hop, the source is the current system (@0x133ee-0x133fd).
//!    Otherwise it is the last hop's resolution
//!    (`_FindActiveCoLocatedSystem`, [`Places::stand_in`]), or the hop
//!    itself when that is −1 (@0x133bc-0x133ec). A course longer than
//!    [`ROUTE_HOPS`], which only the other reading's planner plots, is
//!    taken as full.
//! 5. **The remove** (@0x13466-0x134ba): the last slot at the pick's raw
//!    position is set to −1 and the click ends. The cut has taken every
//!    hop at the pick's place, so it can match only slot 0 with the
//!    length 1 of step 4. Left out: galaxy-map-fidelity-followups phase 4,
//!    which can read the stored start.
//! 6. **The append.** When the session's link test passes for the source
//!    (`can_append`, @0x1340f-0x13462), the start is set to the current
//!    system when it is −1 and kept otherwise (@0x134d3-0x134eb), the pick
//!    is written at the length and `showClearRouteButton` set
//!    (@0x134c9-0x13509); otherwise the
//!    course stays as steps 2 and 3 left it.
//!
//! No sound plays and no random draw is made.

use crate::catalog::SystemId;
use crate::hyperspace::Places;
#[cfg(doc)]
use crate::rulebook::RuleKey;

/// How many hops the original's course holds: slots 1-31 of its 32.
pub const ROUTE_HOPS: usize = 31;

/// What a Shift-click made of the course ([`shift_click`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteEdit {
    /// The course's start after it (slot 0): cleared by a pick at the
    /// current system's place (@0x132ec-0x13306), and set to the current
    /// system by an append when there is none (@0x134d3-0x134eb).
    pub start: Option<SystemId>,
    /// The course after it.
    pub course: Vec<SystemId>,
    /// Whether the pick was written, setting `showClearRouteButton`
    /// (@0x134c9-0x13509).
    pub appended: bool,
}

/// `course`, started at `start` and flown from `current`, after a
/// Shift-click picking `pick`, as `_DoSystemMap` writes it (see the
/// module docs), `can_append` the session's link test of the source:
/// whether the pick may be added after it.
#[must_use]
pub fn shift_click(
    start: Option<SystemId>,
    course: &[SystemId],
    current: SystemId,
    pick: SystemId,
    places: &Places,
    can_append: impl Fn(SystemId) -> bool,
) -> RouteEdit {
    let place = |system: SystemId| places.head(system).unwrap_or(system);
    let mut course = course.to_vec();
    let mut start = start;
    if place(pick) == place(current) {
        course.clear();
        start = None;
    }
    if let Some(cut) = course
        .iter()
        .take(ROUTE_HOPS)
        .position(|&hop| place(hop) == place(pick))
    {
        course.truncate(cut);
    }
    let full = course.len() >= ROUTE_HOPS;
    let source = match course.last() {
        Some(_) if full => current,
        Some(&last) => places.stand_in(last).unwrap_or(last),
        None => current,
    };
    let appended = can_append(source);
    if appended {
        start = start.or(Some(current));
        if full {
            course[0] = pick;
        } else {
            course.push(pick);
        }
    }
    RouteEdit {
        start,
        course,
        appended,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::hyperspace::CoLocated;
    use crate::testkit::star;

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// 130-133 each at its own place, 134 at 131's and 135 at 130's, with
    /// `hidden` hidden.
    fn places(hidden: &[i16]) -> Places {
        Places::new(
            &[
                star(130, (0.0, 0.0), &[]),
                star(131, (1.0, 1.0), &[]),
                star(132, (2.0, 2.0), &[]),
                star(133, (3.0, 3.0), &[]),
                star(134, (1.0, 1.0), &[]),
                star(135, (0.0, 0.0), &[]),
            ],
            &hidden
                .iter()
                .copied()
                .map(SystemId)
                .collect::<BTreeSet<_>>(),
            CoLocated::Lowest,
        )
    }

    /// The Shift-click from 130, its link test passing for `sources`
    /// alone.
    fn click(course: &[i16], pick: i16, places: &Places, sources: &[i16]) -> RouteEdit {
        click_from(None, course, pick, places, sources)
    }

    /// [`click`] with the course started at `start`.
    fn click_from(
        start: Option<i16>,
        course: &[i16],
        pick: i16,
        places: &Places,
        sources: &[i16],
    ) -> RouteEdit {
        let sources = ids(sources);
        shift_click(
            start.map(SystemId),
            &ids(course),
            SystemId(130),
            SystemId(pick),
            places,
            |source| sources.contains(&source),
        )
    }

    /// The edit of a course with no start: an append starts it at 130.
    fn edit(course: &[i16], appended: bool) -> RouteEdit {
        RouteEdit {
            start: appended.then_some(SystemId(130)),
            course: ids(course),
            appended,
        }
    }

    #[test]
    fn an_append_to_a_course_with_no_start_starts_it_at_the_current_system() {
        assert_eq!(
            click_from(None, &[131], 132, &places(&[]), &[131]).start,
            Some(SystemId(130))
        );
    }

    #[test]
    fn an_append_keeps_a_start_elsewhere() {
        assert_eq!(
            click_from(Some(133), &[131], 132, &places(&[]), &[131]).start,
            Some(SystemId(133))
        );
    }

    #[test]
    fn a_refused_pick_keeps_the_start() {
        assert_eq!(
            click_from(Some(133), &[131], 132, &places(&[]), &[]).start,
            Some(SystemId(133))
        );
    }

    #[test]
    fn a_pick_at_the_current_systems_place_clears_the_start_before_the_append() {
        assert_eq!(
            click_from(Some(133), &[131], 135, &places(&[]), &[]).start,
            None
        );
        assert_eq!(
            click_from(Some(133), &[131], 135, &places(&[]), &[130]).start,
            Some(SystemId(130)),
            "the append starts it again"
        );
    }

    #[test]
    fn the_full_course_wrap_keeps_the_start() {
        let full = long(ROUTE_HOPS);
        assert_eq!(
            click_from(Some(133), &full, 131, &places(&[]), &[130]).start,
            Some(SystemId(133))
        );
    }

    #[test]
    fn a_pick_is_appended_to_an_empty_course_from_the_current_system() {
        assert_eq!(click(&[], 131, &places(&[]), &[130]), edit(&[131], true));
    }

    #[test]
    fn a_pick_is_appended_to_a_course_from_its_last_hop() {
        assert_eq!(
            click(&[131], 132, &places(&[]), &[131]),
            edit(&[131, 132], true)
        );
        assert_eq!(
            click(&[131], 132, &places(&[]), &[130]),
            edit(&[131], false),
            "the current system's link is not the last hop's"
        );
    }

    #[test]
    fn a_refused_pick_leaves_the_course_unchanged() {
        assert_eq!(
            click(&[131, 132], 133, &places(&[]), &[]),
            edit(&[131, 132], false)
        );
    }

    #[test]
    fn the_source_is_the_last_hops_stand_in() {
        // 131 hidden, with 134 visible at its place.
        assert_eq!(
            click(&[131], 132, &places(&[131]), &[134]),
            edit(&[131, 132], true)
        );
        assert_eq!(
            click(&[131], 132, &places(&[131]), &[131]),
            edit(&[131], false),
            "not the raw hop"
        );
    }

    #[test]
    fn the_source_is_the_hop_itself_when_nothing_stands_at_its_place() {
        assert_eq!(
            click(&[132], 133, &places(&[132]), &[132]),
            edit(&[132, 133], true)
        );
    }

    #[test]
    fn a_hop_shift_clicked_cuts_the_course_there_and_is_put_back_when_linked() {
        assert_eq!(
            click(&[131, 132, 133, 134], 132, &places(&[]), &[131]),
            edit(&[131, 132], true)
        );
        assert_eq!(
            click(&[131, 132, 133, 134], 132, &places(&[]), &[]),
            edit(&[131], false)
        );
    }

    #[test]
    fn the_last_hop_shift_clicked_leaves_the_course_as_it_was() {
        assert_eq!(
            click(&[131, 132], 132, &places(&[]), &[131]),
            edit(&[131, 132], true)
        );
    }

    #[test]
    fn a_pick_at_the_current_systems_place_clears_the_course() {
        assert_eq!(click(&[131, 132], 130, &places(&[]), &[]), edit(&[], false));
        assert_eq!(
            click(&[131, 132], 135, &places(&[]), &[]),
            edit(&[], false),
            "135 shares 130's place"
        );
    }

    #[test]
    fn a_pick_at_the_current_systems_place_it_links_to_is_appended_to_the_cleared_course() {
        assert_eq!(
            click(&[131, 132], 135, &places(&[]), &[130]),
            edit(&[135], true)
        );
    }

    #[test]
    fn a_pick_at_a_hops_place_under_another_id_cuts_there() {
        // 134 shares 131's place.
        assert_eq!(
            click(&[132, 131, 133], 134, &places(&[]), &[]),
            edit(&[132], false)
        );
        assert_eq!(
            click(&[132, 131, 133], 134, &places(&[]), &[132]),
            edit(&[132, 134], true)
        );
    }

    /// A course of `hops` hops, each 132 or 133 in turn.
    fn long(hops: usize) -> Vec<i16> {
        (0..hops).map(|hop| 132 + (hop % 2) as i16).collect()
    }

    #[test]
    fn a_full_course_has_its_first_hop_overwritten_from_the_current_system() {
        let full = long(ROUTE_HOPS);
        let mut wrapped = full.clone();
        wrapped[0] = 131;
        assert_eq!(
            click(&full, 131, &places(&[]), &[130]),
            edit(&wrapped, true)
        );
        assert_eq!(
            click(&full, 131, &places(&[]), &[133]),
            edit(&full, false),
            "the last hop is not the source"
        );
    }

    #[test]
    fn a_course_longer_than_the_slots_is_taken_as_full() {
        let longer = long(ROUTE_HOPS + 3);
        let mut wrapped = longer.clone();
        wrapped[0] = 131;
        assert_eq!(
            click(&longer, 131, &places(&[]), &[130]),
            edit(&wrapped, true)
        );
    }

    #[test]
    fn the_cut_reads_only_the_first_31_hops() {
        // 131 only past slot 31: no cut, and the course is full.
        let mut longer = long(ROUTE_HOPS);
        longer.push(131);
        let mut wrapped = longer.clone();
        wrapped[0] = 131;
        assert_eq!(
            click(&longer, 131, &places(&[]), &[130]),
            edit(&wrapped, true)
        );
    }

    #[test]
    fn a_course_one_short_of_full_is_appended_to() {
        let short = long(ROUTE_HOPS - 1);
        let mut appended = short.clone();
        appended.push(131);
        assert_eq!(
            click(&short, 131, &places(&[]), &[133]),
            edit(&appended, true)
        );
    }
}
