//! Which systems a map outfit explores ([`map_reveals`]), as the
//! [`MapReach`] its `ModVal` gives says (see
//! [`outfit_effects`](crate::outfit_effects)), and which systems are
//! inhabited ([`inhabited`]). Both follow the
//! [`RuleKey::MapExplore`](crate::RuleKey::MapExplore) rule.
//!
//! **Within N jumps** ([`MapReach::Jumps`]). By the engine,
//! `_AutoSetExploration` (@0xdaba) clears a visited set and calls
//! `_RecursiveAutoExplore` (@0xd942) on the player's system at depth 0.
//! That walk returns at once when the depth is above N or the system was
//! visited; otherwise it marks the system visited, explores it, and
//! recurses into each of its Con1 to Con16 links in order, each resolved
//! to the active system at the link's position, at depth + 1
//! ([`StarMap::ordered_jumps`]). It is depth first, so a system first
//! reached the long way round, too deep to go on from, is never entered
//! again by a shorter way, and the systems beyond it within N jumps are
//! missed. By the Bible ("how many jumps away from the current system to
//! explore"), every system within N jumps is explored, breadth first.
//! Both follow the session's [`HyperlinkRule`].
//!
//! **Inhabited independent systems** ([`MapReach::InhabitedIndependent`],
//! @0x44ddb-0x44e26): every system on the map with no government that is
//! [`inhabited`].
//!
//! **A government class** ([`MapReach::GovtClass`], @0x44e2e-0x44ea1):
//! every system whose government has the class in one of its four
//! `Class` fields; an independent system never.
//!
//! **Inhabited.** `_SystemIsInhabited` (@0x4c4a) reads only a system's
//! *first four* `NavDefs` slots ([`ENGINE_SLOTS`]): it is inhabited when
//! one of them holds a stellar whose runtime +0x30 has no bit 0x20 and
//! whose +0x34 has neither 0x1000 nor 0x2000. `_LoadObjectData` fills
//! +0x30 from the `spöb`'s `Flags` (@0x77361-0x7736a) and +0x34 from its
//! `Flags2` (@0x7736e-0x77378), so that is a stellar neither uninhabited
//! ([`UNINHABITED`], `Flags` 0x0020) nor a hypergate or wormhole
//! ([`GATES`], `Flags2` 0x1000 and 0x2000). By the Bible, any of the
//! system's stellars counts.
//!
//! **Shared by place** ([`explored_with`], [`shared_by_place`],
//! [`start_neighbours`]), as
//! [`RuleKey::ExploredByPlace`](crate::RuleKey::ExploredByPlace) says.
//! The head of a place is `_FindFirstCoLocatedSystem` (@0x4baa-0x4bdf),
//! [`Places::head`]: its lowest-ID system, hidden or not. By the engine,
//! exploring a system explores its head too: `_RecursiveAutoExplore`
//! raises the system (@0xd98f-0xd9af), then its head (@0xd9b6-0xda10),
//! so every map grant does; a jump's arrival (@0x6c16c-0x6c1b8), a
//! hypergate's exit (@0x63a04-0x63a49), a wormhole's exit
//! (@0x642ae-0x642f3) and the take-off (@0x63457-0x634a7) raise both.
//! `X` raises only its system (`_EvalSetExp` @0x15c71-0x15ca1). Each
//! rebuild point raises every system, hidden or not, to its head's level
//! whenever its head is another system (`_HandleStellarSystemVisibility`
//! @0x32b13-0x32b8f), so a member explored while its head is not shares
//! nothing. A new pilot explores, for each of its start system's Con
//! slots naming a visible system, that system's head (`_DoNewPilot`
//! @0x18f97-0x19003). By the other reading each system is explored on
//! its own, and a new pilot explores each visible neighbour itself. The
//! session applies these at its sites (its `explored` module).
//!
//! Not modelled here: the original's walk also explores the nebulae near
//! each system (`_ExploreNebulaeFromSystem`, crons-news-ranks phase 16).
//! The known flag (+0x1ec) and the explored levels 1 and 2 kept apart are
//! deferred to the fidelity roadmap (crons-news-ranks-fidelity phases 20
//! and 21): the pilot's one set of explored systems does not tell `X`'s
//! level 1 from a map's level 2.

use std::collections::{BTreeSet, VecDeque};

use crate::catalog::SystemId;
use crate::govt::Governments;
use crate::hyperspace::{HyperlinkRule, Places, StarMap};
use crate::outfit_effects::MapReach;
use crate::rulebook::RuleSource;

/// How many of a system's first `NavDefs` slots the engine reads for
/// whether it is inhabited (@0x4c4f).
pub const ENGINE_SLOTS: usize = 4;
/// The `spöb` `Flags` bit of an uninhabited stellar.
pub const UNINHABITED: u32 = 0x0020;
/// The `spöb` `Flags2` bits of a hypergate (0x1000) and a wormhole
/// (0x2000).
pub const GATES: u16 = 0x3000;

/// Whether a system whose `NavDefs` slots hold stellars of these `Flags`
/// and `Flags2` (`None` for an empty slot) is inhabited, as `source` says
/// (see the module docs).
#[must_use]
pub fn inhabited(stellars: &[Option<(u32, u16)>], source: RuleSource) -> bool {
    let read = match source {
        RuleSource::Engine => stellars.len().min(ENGINE_SLOTS),
        RuleSource::Bible => stellars.len(),
    };
    stellars[..read]
        .iter()
        .flatten()
        .any(|&(flags, flags2)| flags & UNINHABITED == 0 && flags2 & GATES == 0)
}

/// The systems a map reaching `reach` explores from the player's system
/// `from`, along `map`'s hyperlinks under `hyperlinks`, with `govts`'
/// classes, as `source` says (see the module docs).
#[must_use]
pub fn map_reveals(
    reach: MapReach,
    from: SystemId,
    map: &StarMap,
    hyperlinks: HyperlinkRule,
    govts: &Governments,
    source: RuleSource,
) -> BTreeSet<SystemId> {
    match reach {
        MapReach::Jumps(most) => match source {
            RuleSource::Engine => {
                let mut visited = BTreeSet::new();
                walk(map, hyperlinks, from, 0, most, &mut visited);
                visited
            }
            RuleSource::Bible => within(map, hyperlinks, from, most),
        },
        MapReach::InhabitedIndependent => map
            .systems()
            .filter(|&id| map.govt(id).is_none() && inhabited(map.stellars(id), source))
            .collect(),
        MapReach::GovtClass(class) => map
            .systems()
            .filter(|&id| {
                map.govt(id)
                    .and_then(|govt| govts.get(govt))
                    .is_some_and(|record| record.classes.contains(&class))
            })
            .collect(),
        MapReach::Nothing => BTreeSet::new(),
    }
}

/// The engine's depth-first walk from `at`, `depth` jumps out, to at most
/// `most` (`_RecursiveAutoExplore` @0xd942).
fn walk(
    map: &StarMap,
    hyperlinks: HyperlinkRule,
    at: SystemId,
    depth: u32,
    most: u16,
    visited: &mut BTreeSet<SystemId>,
) {
    if depth > u32::from(most) || !visited.insert(at) {
        return;
    }
    for next in map.ordered_jumps(at, hyperlinks) {
        walk(map, hyperlinks, next, depth + 1, most, visited);
    }
}

/// The systems that exploring `system` explores, as `source` says (see
/// the module docs): `system` itself, then, by the engine, its place's
/// head when that is another system. A system at no place explores only
/// itself.
#[must_use]
pub fn explored_with(system: SystemId, places: &Places, source: RuleSource) -> Vec<SystemId> {
    let mut explored = vec![system];
    if source == RuleSource::Engine
        && let Some(head) = places.head(system)
        && head != system
    {
        explored.push(head);
    }
    explored
}

/// The systems a rebuild point explores, of those not in `explored`, as
/// `source` says (see the module docs): by the engine, every system at
/// the place of an explored head, hidden or not; by the other reading,
/// none.
#[must_use]
pub fn shared_by_place(
    explored: &BTreeSet<SystemId>,
    places: &Places,
    source: RuleSource,
) -> BTreeSet<SystemId> {
    if source == RuleSource::Bible {
        return BTreeSet::new();
    }
    explored
        .iter()
        .filter(|&&head| places.head(head) == Some(head))
        .flat_map(|&head| places.at(head))
        .filter(|member| !explored.contains(member))
        .collect()
}

/// What a new pilot explores from its start system's Con slots `links`,
/// as `source` says (see the module docs): for each slot naming a system
/// `visible` holds, by the engine that system's head (itself at no
/// place), by the other reading the system itself.
#[must_use]
pub fn start_neighbours(
    links: &[SystemId],
    visible: impl Fn(SystemId) -> bool,
    places: &Places,
    source: RuleSource,
) -> Vec<SystemId> {
    links
        .iter()
        .copied()
        .filter(|&link| visible(link))
        .map(|link| match source {
            RuleSource::Engine => places.head(link).unwrap_or(link),
            RuleSource::Bible => link,
        })
        .collect()
}

/// Every system within `most` jumps of `from`, `from` among them.
fn within(
    map: &StarMap,
    hyperlinks: HyperlinkRule,
    from: SystemId,
    most: u16,
) -> BTreeSet<SystemId> {
    let mut reached = BTreeSet::from([from]);
    let mut queue = VecDeque::from([(from, 0)]);
    while let Some((at, depth)) = queue.pop_front() {
        if depth == most {
            continue;
        }
        for next in map.jumps(at, hyperlinks) {
            if reached.insert(next) {
                queue.push_back((next, depth + 1));
            }
        }
    }
    reached
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, GovtRecord, StarSystem};
    use crate::testkit::{govt, star};

    fn ids(list: &[i16]) -> BTreeSet<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    fn reveals(
        map: &StarMap,
        reach: MapReach,
        from: i16,
        source: RuleSource,
    ) -> BTreeSet<SystemId> {
        map_reveals(
            reach,
            SystemId(from),
            map,
            HyperlinkRule::Engine,
            &Governments::default(),
            source,
        )
    }

    /// 128 - 129 - 130 - 131 in a line, each linking the next and back.
    fn line() -> StarMap {
        StarMap::new(vec![
            star(128, (0.0, 0.0), &[129]),
            star(129, (1.0, 0.0), &[128, 130]),
            star(130, (2.0, 0.0), &[129, 131]),
            star(131, (3.0, 0.0), &[130]),
        ])
    }

    #[test]
    fn a_map_of_n_jumps_explores_the_line_that_far_both_ways() {
        let map = line();
        for source in RuleSource::ALL {
            assert_eq!(
                reveals(&map, MapReach::Jumps(1), 128, source),
                ids(&[128, 129])
            );
            assert_eq!(
                reveals(&map, MapReach::Jumps(2), 128, source),
                ids(&[128, 129, 130])
            );
            assert_eq!(
                reveals(&map, MapReach::Jumps(1), 130, source),
                ids(&[129, 130, 131])
            );
            assert_eq!(
                reveals(&map, MapReach::Jumps(9), 129, source),
                ids(&[128, 129, 130, 131])
            );
            assert_eq!(reveals(&map, MapReach::Nothing, 129, source), ids(&[]));
        }
    }

    /// A (128) lists B (129) then C (130); B lists A and C; C lists A, B
    /// and D (131); D lists C.
    fn diamond() -> StarMap {
        StarMap::new(vec![
            star(128, (0.0, 0.0), &[129, 130]),
            star(129, (1.0, 0.0), &[128, 130]),
            star(130, (1.0, 1.0), &[128, 129, 131]),
            star(131, (2.0, 1.0), &[130]),
        ])
    }

    #[test]
    fn the_engines_depth_first_walk_misses_a_system_the_bibles_reaches() {
        // The engine goes A, B (1), C (2, too deep to go on), and never
        // enters C again from A at 1, so D, two jumps from A, is missed.
        let map = diamond();
        assert_eq!(
            reveals(&map, MapReach::Jumps(2), 128, RuleSource::Engine),
            ids(&[128, 129, 130])
        );
        assert_eq!(
            reveals(&map, MapReach::Jumps(2), 128, RuleSource::Bible),
            ids(&[128, 129, 130, 131])
        );
        assert_eq!(
            reveals(&map, MapReach::Jumps(3), 128, RuleSource::Engine),
            ids(&[128, 129, 130, 131])
        );
    }

    #[test]
    fn the_walk_follows_the_hyperlink_rule() {
        // 129 lists 128, which does not list it back.
        let map = StarMap::new(vec![
            star(128, (0.0, 0.0), &[]),
            star(129, (1.0, 0.0), &[128]),
        ]);
        for source in RuleSource::ALL {
            let explored = |rule| {
                map_reveals(
                    MapReach::Jumps(1),
                    SystemId(128),
                    &map,
                    rule,
                    &Governments::default(),
                    source,
                )
            };
            assert_eq!(explored(HyperlinkRule::Engine), ids(&[128]), "{source:?}");
            assert_eq!(
                explored(HyperlinkRule::BothWays),
                ids(&[128, 129]),
                "{source:?}"
            );
        }
    }

    const LIVED_IN: Option<(u32, u16)> = Some((0x0001, 0));
    const EMPTY: Option<(u32, u16)> = Some((UNINHABITED, 0));

    fn with(system: StarSystem, govt: Option<i16>, stellars: &[Option<(u32, u16)>]) -> StarSystem {
        StarSystem {
            govt: govt.map(GovtId),
            stellars: stellars.to_vec(),
            ..system
        }
    }

    #[test]
    fn the_inhabited_independent_systems_are_those_with_no_government_and_a_lived_in_stellar() {
        let map = StarMap::new(vec![
            with(star(128, (0.0, 0.0), &[]), None, &[LIVED_IN]),
            with(star(129, (1.0, 0.0), &[]), Some(140), &[LIVED_IN]),
            with(star(130, (2.0, 0.0), &[]), None, &[EMPTY]),
            with(star(131, (3.0, 0.0), &[]), None, &[]),
            with(
                star(132, (4.0, 0.0), &[]),
                None,
                &[None, None, None, None, LIVED_IN],
            ),
        ]);
        assert_eq!(
            reveals(
                &map,
                MapReach::InhabitedIndependent,
                129,
                RuleSource::Engine
            ),
            ids(&[128])
        );
        assert_eq!(
            reveals(&map, MapReach::InhabitedIndependent, 129, RuleSource::Bible),
            ids(&[128, 132]),
            "any stellar counts"
        );
    }

    fn classed(id: i16, classes: [i16; 4]) -> GovtRecord {
        GovtRecord {
            classes,
            ..govt(id)
        }
    }

    #[test]
    fn a_government_class_map_explores_the_systems_of_any_government_of_that_class() {
        let map = StarMap::new(vec![
            with(star(128, (0.0, 0.0), &[]), Some(140), &[]),
            with(star(129, (1.0, 0.0), &[]), Some(141), &[]),
            with(star(130, (2.0, 0.0), &[]), Some(142), &[]),
            with(star(131, (3.0, 0.0), &[]), None, &[]),
            with(star(132, (4.0, 0.0), &[]), Some(150), &[]),
        ]);
        let govts = Governments::new([
            classed(140, [3, -1, -1, -1]),
            classed(141, [-1, -1, -1, 3]),
            classed(142, [2, 4, -1, -1]),
        ]);
        for source in RuleSource::ALL {
            let explored = |class| {
                map_reveals(
                    MapReach::GovtClass(class),
                    SystemId(131),
                    &map,
                    HyperlinkRule::Engine,
                    &govts,
                    source,
                )
            };
            assert_eq!(explored(3), ids(&[128, 129]));
            assert_eq!(explored(4), ids(&[130]));
            assert_eq!(explored(5), ids(&[]));
        }
    }

    #[test]
    fn the_engine_judges_inhabited_by_the_first_four_slots_and_the_bible_by_any() {
        for source in RuleSource::ALL {
            assert!(inhabited(&[LIVED_IN], source));
            assert!(inhabited(&[None, EMPTY, LIVED_IN], source));
            assert!(!inhabited(&[], source));
            assert!(!inhabited(&[None, EMPTY], source));
            assert!(!inhabited(&[Some((0x0021, 0))], source), "uninhabited");
            assert!(!inhabited(&[Some((0x0001, 0x1000))], source), "a hypergate");
            assert!(!inhabited(&[Some((0x0001, 0x2000))], source), "a wormhole");
            assert!(inhabited(&[Some((0x0001, 0x0FFF))], source), "other bits");
            assert!(inhabited(&[Some((!UNINHABITED, 0xCFFF))], source));
        }
        let fifth = [None, EMPTY, None, EMPTY, LIVED_IN];
        assert!(!inhabited(&fifth, RuleSource::Engine));
        assert!(inhabited(&fifth, RuleSource::Bible));
        assert!(inhabited(&fifth[1..], RuleSource::Engine), "the fourth");
    }

    /// 130, 131 and 132 at one place, 130 its head; 133 elsewhere; 131
    /// hidden.
    fn place() -> Places {
        Places::new(
            &[
                star(130, (5.0, 5.0), &[]),
                star(131, (5.0, 5.0), &[]),
                star(132, (5.0, 5.0), &[]),
                star(133, (9.0, 9.0), &[]),
            ],
            &ids(&[131]),
            crate::hyperspace::CoLocated::Lowest,
        )
    }

    fn list(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    #[test]
    fn exploring_a_member_explores_its_head_by_the_engine() {
        let places = place();
        for member in [131, 132] {
            assert_eq!(
                explored_with(SystemId(member), &places, RuleSource::Engine),
                list(&[member, 130])
            );
            assert_eq!(
                explored_with(SystemId(member), &places, RuleSource::Bible),
                list(&[member])
            );
        }
    }

    #[test]
    fn exploring_a_head_explores_only_itself() {
        let places = place();
        for source in RuleSource::ALL {
            assert_eq!(explored_with(SystemId(130), &places, source), list(&[130]));
            assert_eq!(explored_with(SystemId(133), &places, source), list(&[133]));
        }
    }

    #[test]
    fn a_system_at_no_place_explores_itself() {
        let places = place();
        for source in RuleSource::ALL {
            assert_eq!(explored_with(SystemId(200), &places, source), list(&[200]));
        }
    }

    #[test]
    fn a_rebuild_shares_an_explored_head_with_every_member_hidden_or_not() {
        let places = place();
        assert_eq!(
            shared_by_place(&ids(&[130]), &places, RuleSource::Engine),
            ids(&[131, 132])
        );
        assert_eq!(
            shared_by_place(&ids(&[130, 133]), &places, RuleSource::Engine),
            ids(&[131, 132]),
            "a place of one shares nothing"
        );
        assert_eq!(
            shared_by_place(&ids(&[130]), &places, RuleSource::Bible),
            ids(&[])
        );
    }

    #[test]
    fn an_explored_member_shares_nothing_while_its_head_is_unexplored() {
        let places = place();
        for source in RuleSource::ALL {
            assert_eq!(shared_by_place(&ids(&[131]), &places, source), ids(&[]));
            assert_eq!(
                shared_by_place(&ids(&[200]), &places, source),
                ids(&[]),
                "a system at no place"
            );
        }
    }

    #[test]
    fn nothing_new_is_shared_twice() {
        let places = place();
        assert_eq!(
            shared_by_place(&ids(&[130, 131, 132]), &places, RuleSource::Engine),
            ids(&[])
        );
        assert_eq!(
            shared_by_place(&ids(&[130, 131]), &places, RuleSource::Engine),
            ids(&[132])
        );
    }

    /// 131 and 132 visible, 133 visible, 134 hidden.
    fn visible(system: SystemId) -> bool {
        system != SystemId(134)
    }

    #[test]
    fn a_new_pilot_explores_each_visible_neighbours_head_by_the_engine() {
        let places = place();
        let links = list(&[131, 133, 134]);
        assert_eq!(
            start_neighbours(&links, visible, &places, RuleSource::Engine),
            list(&[130, 133])
        );
        assert_eq!(
            start_neighbours(&links, visible, &places, RuleSource::Bible),
            list(&[131, 133])
        );
    }

    #[test]
    fn a_hidden_neighbour_is_skipped_under_both_readings() {
        let places = place();
        for source in RuleSource::ALL {
            assert_eq!(
                start_neighbours(&list(&[134]), visible, &places, source),
                list(&[])
            );
        }
    }

    #[test]
    fn a_neighbour_at_no_place_is_its_own_head() {
        let places = place();
        for source in RuleSource::ALL {
            assert_eq!(
                start_neighbours(&list(&[200]), visible, &places, source),
                list(&[200])
            );
        }
    }
}
