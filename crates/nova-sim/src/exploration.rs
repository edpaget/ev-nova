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
//! Not modelled: the original's walk also explores the nebulae near each
//! system (`_ExploreNebulaeFromSystem`) and marks the systems next to an
//! explored one as seen for the map (+0x1ec); and it explores a system
//! to level 2 where `X` explores to level 1, which the pilot's one set of
//! explored systems does not tell apart.

use std::collections::{BTreeSet, VecDeque};

use crate::catalog::SystemId;
use crate::govt::Governments;
use crate::hyperspace::{HyperlinkRule, StarMap};
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
}
