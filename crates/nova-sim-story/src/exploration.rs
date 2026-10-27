//! Which walks a map outfit runs ([`map_walks`]), as the [`MapReach`]
//! its `ModVal` gives says (see
//! [`outfit_effects`](crate::outfit_effects)), and which systems are
//! inhabited ([`inhabited`]). Both follow the
//! [`RuleKey::MapExplore`](crate::RuleKey::MapExplore) rule; the walk
//! itself is the [`map_walk`](crate::map_walk) module's.
//!
//! **Within N jumps** ([`MapReach::Jumps`]): one walk from the player's
//! system, reaching N (`_GrantOutfitItem` @0x44da6-0x44dcb).
//!
//! **Inhabited independent systems** ([`MapReach::InhabitedIndependent`],
//! @0x44ddb-0x44e26): a walk reaching 0 over every system on the map with
//! no government that is [`inhabited`], by ascending ID (@0x44e12).
//!
//! **A government class** ([`MapReach::GovtClass`], @0x44e2e-0x44ea1): a
//! walk reaching 0 over every system whose government has the class in
//! one of its four `Class` fields, by ascending ID (@0x44e86); an
//! independent system never.
//!
//! The original tests each system as its loop reaches it, so a walk run
//! before could change the next test. It reads the system's visibility
//! (+0x1eb), government (+0x8) and stellars, which change only at a
//! refresh, and no `OnExplore` refreshes (no set operator writes the
//! galaxy overlay), so they are worked out first here.
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
//! The original's walk also explores the nebulae near each system
//! (`_ExploreNebulaeFromSystem`, not its head's): the session raises
//! `Explored` for each system reached and dispatches it at once (see the
//! [`map_walk`](crate::map_walk) and [`nebula`](crate::nebula) modules).
//!
//! **Known** ([`known_from`]). A system's known flag (+0x1ec) is what the
//! galaxy map draws besides the explored systems (`_DrawMap` @0xea0b,
//! @0xedd8) and what the escape search needs (@0xd166, @0xd250). A known
//! pass marks each visible (+0x1eb), existing (+0x1ed) and explored
//! system, and the system standing at the place of each of its 16 Con
//! slots. Each rebuild point clears every flag in its explored-share loop
//! (`_HandleStellarSystemVisibility` @0x32b7b) and then runs the pass
//! (@0x32b91-0x32c10); a map grant runs it after its walk, with no clear
//! (`_AutoSetExploration` @0xdb06-0xdbd4, which skips a slot above 0x7ff
//! and resolves by place inline); and the hypergate map marks each system
//! it arrows (`_DrawMap` @0xf457-0xf45f). `_InitObjects` clears them all
//! (@0x1ca83). The flag isn't saved: no save or load routine touches
//! +0x1ec, so a load's rebuild works it out again. The session keeps it
//! (its `explored` module).
//!
//! **Levels.** Each rule above reads only whether a system is explored
//! (level 1 or 2; see [`ExploredLevel`](crate::pilot::ExploredLevel)).
//! [`shared_by_place`] serves each level alike: the session shares the
//! explored systems, then the charted ones, raising each member to its
//! head's level.

use std::collections::BTreeSet;

use crate::catalog::SystemId;
use crate::govt::Governments;
use crate::hyperspace::{Places, StarMap};
use crate::outfit_effects::MapReach;
use crate::rulebook::RuleSource;
use crate::story::OverlaidGalaxy;

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

/// The walks a map reaching `reach` runs from the player's system
/// `from`, over `map` with `govts`' classes, inhabited as `source` says
/// (see the module docs): each walk's origin and reach, in order.
#[must_use]
pub fn map_walks(
    reach: MapReach,
    from: SystemId,
    map: &StarMap,
    govts: &Governments,
    source: RuleSource,
) -> Vec<(SystemId, u16)> {
    match reach {
        MapReach::Jumps(most) => vec![(from, most)],
        MapReach::InhabitedIndependent => map
            .systems()
            .filter(|&id| map.govt(id).is_none() && inhabited(map.stellars(id), source))
            .map(|id| (id, 0))
            .collect(),
        MapReach::GovtClass(class) => map
            .systems()
            .filter(|&id| {
                map.govt(id)
                    .and_then(|govt| govts.get(govt))
                    .is_some_and(|record| record.classes.contains(&class))
            })
            .map(|id| (id, 0))
            .collect(),
        MapReach::Nothing => Vec::new(),
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

/// How many Con slots a system has, `Con1-Con16`: those the known pass
/// reads.
const CON_SLOTS: usize = 16;

/// The systems a known pass marks, with `explored` the systems explored
/// (see the module docs): each system of `galaxy`'s catalog that is
/// visible (+0x1eb) and explored (+0x90 > 0), and the system standing for
/// each of its first 16 Con slots by place, a slot naming a system the
/// catalog does not hold, or with none visible at its place, marking
/// nothing (`_HandleStellarSystemVisibility` @0x32b91-0x32c10,
/// `_AutoSetExploration` @0xdb06-0xdbd4).
#[must_use]
pub fn known_from(galaxy: &OverlaidGalaxy, explored: &BTreeSet<SystemId>) -> BTreeSet<SystemId> {
    let mut known = BTreeSet::new();
    for system in galaxy.catalog_systems() {
        if !galaxy.visible(system) || !explored.contains(&system) {
            continue;
        }
        known.insert(system);
        for &slot in galaxy.con_slots(system).iter().take(CON_SLOTS) {
            if galaxy.exists(slot)
                && let Some(link) = galaxy.same_place(slot)
            {
                known.insert(link);
            }
        }
    }
    known
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, GovtRecord, StarSystem};
    use crate::story::OverlaidGalaxy;
    use nova_sim_catalog::fixture::{govt, star};

    fn ids(list: &[i16]) -> BTreeSet<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// The walks `reach` runs from `from` over `map`, with no
    /// governments: each origin with its reach.
    fn walks(map: &StarMap, reach: MapReach, from: i16, source: RuleSource) -> Vec<(i16, u16)> {
        map_walks(reach, SystemId(from), map, &Governments::default(), source)
            .into_iter()
            .map(|(system, reach)| (system.0, reach))
            .collect()
    }

    #[test]
    fn a_map_of_n_jumps_runs_one_walk_from_the_players_system_and_nothing_none() {
        let map = StarMap::new(vec![star(129, (0.0, 0.0), &[])]);
        for source in RuleSource::ALL {
            assert_eq!(walks(&map, MapReach::Jumps(3), 129, source), [(129, 3)]);
            assert_eq!(walks(&map, MapReach::Jumps(1), 200, source), [(200, 1)]);
            assert_eq!(walks(&map, MapReach::Nothing, 129, source), []);
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
            walks(
                &map,
                MapReach::InhabitedIndependent,
                129,
                RuleSource::Engine
            ),
            [(128, 0)]
        );
        assert_eq!(
            walks(&map, MapReach::InhabitedIndependent, 129, RuleSource::Bible),
            [(128, 0), (132, 0)],
            "any stellar counts, each walked on its own by ascending ID"
        );
    }

    fn classed(id: i16, classes: [i16; 4]) -> GovtRecord {
        GovtRecord {
            classes,
            ..govt(id)
        }
    }

    #[test]
    fn a_government_class_map_walks_each_system_of_any_government_of_that_class() {
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
            let walked = |class| {
                map_walks(
                    MapReach::GovtClass(class),
                    SystemId(131),
                    &map,
                    &govts,
                    source,
                )
            };
            assert_eq!(walked(3), [(SystemId(128), 0), (SystemId(129), 0)]);
            assert_eq!(walked(4), [(SystemId(130), 0)]);
            assert_eq!(walked(5), []);
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

    /// 130 lists 131, 140 (not in the catalog), 132 and itself; 131 lists
    /// 133; 132 is hidden, with 134 at its place; 135 is hidden and lists
    /// 136; 137 lists 138.
    fn known_galaxy() -> OverlaidGalaxy {
        let mut overlay = crate::story::GalaxyOverlay::default();
        overlay.set_hidden(&ids(&[132, 135]));
        OverlaidGalaxy::new(
            vec![
                star(130, (0.0, 0.0), &[131, 140, 132, 130]),
                star(131, (1.0, 0.0), &[133]),
                star(132, (2.0, 0.0), &[]),
                star(133, (3.0, 0.0), &[]),
                star(134, (2.0, 0.0), &[]),
                star(135, (5.0, 0.0), &[136]),
                star(136, (6.0, 0.0), &[]),
                star(137, (7.0, 0.0), &[138]),
                star(138, (8.0, 0.0), &[]),
            ],
            overlay,
        )
    }

    #[test]
    fn the_known_pass_marks_each_visible_explored_system_and_its_links_by_place() {
        let galaxy = known_galaxy();
        assert_eq!(
            known_from(&galaxy, &ids(&[130, 135, 136, 138, 140])),
            ids(&[130, 131, 134, 136, 138]),
            "130 and its links, 132 by its stand-in 134 and 140 not at all; \
             135 hidden and 140 not in the catalog mark nothing; 136 and 138 \
             explored mark themselves, and 137, unexplored, not its link"
        );
        assert_eq!(known_from(&galaxy, &ids(&[])), ids(&[]));
    }

    #[test]
    fn the_known_pass_reads_only_the_first_sixteen_slots() {
        let mut links: Vec<i16> = vec![-1; 16];
        links.push(131);
        let galaxy = OverlaidGalaxy::new(
            vec![star(130, (0.0, 0.0), &links), star(131, (1.0, 0.0), &[])],
            crate::story::GalaxyOverlay::default(),
        );
        assert_eq!(known_from(&galaxy, &ids(&[130])), ids(&[130]));
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
