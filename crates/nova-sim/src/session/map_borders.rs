//! Which systems add to the galaxy map's borders: the session's side of
//! [`map_borders`] ([`Session::map_border_sources`]).
//!
//! `_AddSystemInfluenceToMap` (@0x11592) adds nothing for a system unless
//! it is active (+0x1eb, @0x115b2), known (+0x1ec, @0x115bf) and explored
//! (its own +0x90 > 0, @0x115cc), has a government (+0x8 ≠ −1, @0x115da)
//! without `Flags2` [`NO_INFLUENCE`] (@0x115f9), and is inhabited
//! (`_SystemIsInhabited` @0x11607). Inhabited is
//! [`exploration::inhabited`] as [`RuleKey::MapExplore`] reads it, as the
//! colour code reads it ([`Session::system_color_code`]).
//!
//! The coarse pass adds each place once, through the system standing for
//! the place's first system (`_FindActiveCoLocatedSystem` @0x4be0,
//! `_RecalcMapInfluence` @0x11a84-0x11ade); the fine pass walks every
//! system by ascending ID and adds the system standing for it, so a place
//! of two systems adds its system twice (@0x146fe-0x14727). The system
//! standing for a place is the one [`galaxy_map::draw_map`] resolves
//! links through ([`OverlaidGalaxy::same_place`]).
//!
//! The grid's cell cache, `sÿst` +0x4/+0x6, which `_AddSystemInfluenceToMap`
//! writes (@0x1164c-0x11657), is written and never read: no other routine
//! of the binary reads either word of a system.
//!
//! [`map_borders`]: crate::map_borders
//! [`galaxy_map::draw_map`]: crate::galaxy_map::draw_map
//! [`OverlaidGalaxy::same_place`]: crate::story::OverlaidGalaxy::same_place

use super::Session;
use crate::catalog::SystemId;
use crate::exploration;
use crate::govt::{NO_INFLUENCE, SMALL_INFLUENCE};
use crate::map_borders::{BorderSource, BorderSources};
use crate::rulebook::RuleKey;

impl Session {
    /// What the galaxy map's borders add now (see the module docs): the
    /// coarse pass's sources, one a place, the fine pass's walk, and the
    /// governments.
    #[must_use]
    pub fn map_border_sources(&self) -> BorderSources {
        let galaxy = &self.galaxy;
        let places = galaxy.map().places();
        let mut coarse = Vec::new();
        let mut walk = Vec::new();
        for system in galaxy.catalog_systems() {
            let Some(source) = galaxy
                .same_place(system)
                .and_then(|stand_in| self.border_source(stand_in))
            else {
                continue;
            };
            if places.head(system) == Some(system) {
                coarse.push(source);
            }
            walk.push((system, source));
        }
        BorderSources {
            coarse,
            walk,
            govts: self.statics.govts.clone(),
        }
    }

    /// `system`'s source, a visible system standing for a place, when it
    /// passes `_AddSystemInfluenceToMap`'s flag tests (@0x115b2-0x1160e).
    fn border_source(&self, system: SystemId) -> Option<BorderSource> {
        let galaxy = &self.galaxy;
        if !self.known.contains(&system) || !self.pilot.has_explored(system) {
            return None;
        }
        let govt = galaxy.govt(system)?;
        if self.statics.govts.flag2(Some(govt), NO_INFLUENCE) {
            return None;
        }
        let inhabited = exploration::inhabited(
            galaxy.map().stellars(system),
            self.rules.source_for(RuleKey::MapExplore),
        );
        let position = galaxy.position(system)?;
        // A system's map position is a whole `i16` in its `sÿst`.
        #[allow(clippy::cast_possible_truncation)]
        let pos = (position.x as i16, position.y as i16);
        inhabited.then_some(BorderSource {
            system,
            pos,
            govt,
            small: self.statics.govts.flag2(Some(govt), SMALL_INFLUENCE),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::catalog::{GovtId, GovtRecord, StarSystem, SystemVisibility};
    use crate::control::Test;
    use crate::landing::StellarFlags;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::testkit::{FakePilotCatalog, catalog, govt, star};

    const HOME: Option<(u32, u16)> = Some((StellarFlags::CAN_LAND, 0));

    fn lived(system: StarSystem, govt: Option<i16>, stellars: &[Option<(u32, u16)>]) -> StarSystem {
        StarSystem {
            govt: govt.map(GovtId),
            stellars: stellars.to_vec(),
            ..system
        }
    }

    /// 130 (the start) and 131, the Federation's (128), inhabited; 132 of
    /// the Civvies (129, `Flags2` 0x0004); 133 independent; 134 the
    /// Federation's, its only lived-in stellar in the fifth slot; 135 the
    /// Rebellion's (130, `Flags2` 0x0002); 136 hidden; 137, hidden, and
    /// 138 at one place.
    fn bordered() -> FakePilotCatalog {
        let uninhabited = Some((StellarFlags::UNINHABITED, 0));
        let base = catalog();
        let ids = [130, 131, 132, 133, 134, 135, 136, 137, 138];
        FakePilotCatalog {
            systems: ids.map(SystemId).to_vec(),
            star_map: vec![
                lived(star(130, (0.0, 0.0), &[131]), Some(128), &[HOME]),
                lived(star(131, (600.0, 0.0), &[]), Some(128), &[HOME]),
                lived(star(132, (0.0, 600.0), &[]), Some(129), &[HOME]),
                lived(star(133, (-600.0, 0.0), &[]), None, &[HOME]),
                lived(
                    star(134, (0.0, -600.0), &[]),
                    Some(128),
                    &[uninhabited, None, None, None, HOME],
                ),
                lived(star(135, (300.0, 300.0), &[]), Some(130), &[HOME]),
                lived(star(136, (-300.0, 300.0), &[]), Some(128), &[HOME]),
                lived(star(137, (-300.0, -300.0), &[]), Some(130), &[HOME]),
                lived(star(138, (-300.0, -300.0), &[]), Some(128), &[HOME]),
            ],
            govts: vec![
                govt(128),
                GovtRecord {
                    flags2: NO_INFLUENCE,
                    ..govt(129)
                },
                GovtRecord {
                    flags2: SMALL_INFLUENCE,
                    ..govt(130)
                },
            ],
            visibility: [136, 137]
                .map(|id| SystemVisibility {
                    id: SystemId(id),
                    visibility: Test::parse("b1"),
                })
                .to_vec(),
            ..base
        }
    }

    /// A session of `catalog` under `rules` with every system explored
    /// and known.
    fn everything_seen(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules);
        for id in 130..=138 {
            session.pilot.explore(SystemId(id));
        }
        session.known = (130..=138).map(SystemId).collect();
        session
    }

    fn coarse(sources: &BorderSources) -> Vec<i16> {
        sources
            .coarse
            .iter()
            .map(|source| source.system.0)
            .collect()
    }

    fn walked(sources: &BorderSources) -> Vec<(i16, i16)> {
        sources
            .walk
            .iter()
            .map(|(walked, source)| (walked.0, source.system.0))
            .collect()
    }

    #[test]
    fn every_seen_inhabited_influencing_system_adds_once_a_place() {
        let catalog = bordered();
        let session = everything_seen(&catalog, Rulebook::default());
        let sources = session.map_border_sources();
        assert_eq!(
            coarse(&sources),
            [130, 131, 135, 138],
            "not 132 (0x0004), 133 (independent), 134 (uninhabited) or 136 \
             (hidden); 137's place by 138"
        );
        assert_eq!(
            sources.coarse[0],
            BorderSource {
                system: SystemId(130),
                pos: (0, 0),
                govt: GovtId(128),
                small: false
            }
        );
        assert_eq!(sources.govts, crate::govt::Governments::read(&catalog));
    }

    #[test]
    fn the_walk_runs_by_ascending_id_and_adds_a_shared_places_system_for_each() {
        let catalog = bordered();
        let session = everything_seen(&catalog, Rulebook::default());
        let sources = session.map_border_sources();
        assert_eq!(
            walked(&sources),
            [(130, 130), (131, 131), (135, 135), (137, 138), (138, 138)]
        );
        assert_eq!(sources.walk[3].1, sources.coarse[3], "the same source");
    }

    #[test]
    fn an_inactive_system_adds_none() {
        let mut catalog = bordered();
        catalog.visibility.push(SystemVisibility {
            id: SystemId(131),
            visibility: Test::parse("b1"),
        });
        let session = everything_seen(&catalog, Rulebook::default());
        assert!(!coarse(&session.map_border_sources()).contains(&131));
    }

    #[test]
    fn an_unknown_system_adds_none() {
        let catalog = bordered();
        let mut session = everything_seen(&catalog, Rulebook::default());
        session.known.remove(&SystemId(131));
        let sources = session.map_border_sources();
        assert_eq!(coarse(&sources), [130, 135, 138]);
        assert!(walked(&sources).iter().all(|&(_, by)| by != 131));
    }

    #[test]
    fn an_unexplored_system_adds_none() {
        let catalog = bordered();
        let mut session = everything_seen(&catalog, Rulebook::default());
        session.pilot.explored = BTreeSet::from([SystemId(130), SystemId(135), SystemId(138)]);
        assert_eq!(coarse(&session.map_border_sources()), [130, 135, 138]);
    }

    #[test]
    fn the_stand_ins_own_flags_count_not_the_systems_it_stands_for() {
        // 137, hidden, is unknown and unexplored: 138 stands for it all the
        // same. 138 unknown adds neither.
        let catalog = bordered();
        let mut session = everything_seen(&catalog, Rulebook::default());
        session.known.remove(&SystemId(137));
        session.pilot.explored.remove(&SystemId(137));
        assert_eq!(coarse(&session.map_border_sources()), [130, 131, 135, 138]);
        session.known.remove(&SystemId(138));
        let sources = session.map_border_sources();
        assert_eq!(coarse(&sources), [130, 131, 135]);
        assert_eq!(walked(&sources), [(130, 130), (131, 131), (135, 135)]);
    }

    #[test]
    fn an_independent_system_and_one_of_a_flags2_0x0004_government_add_none() {
        let mut catalog = bordered();
        catalog.govts[1].flags2 = 0;
        let session = everything_seen(&catalog, Rulebook::default());
        assert_eq!(
            coarse(&session.map_border_sources()),
            [130, 131, 132, 135, 138],
            "132 once 129 lacks 0x0004; 133, independent, never"
        );
    }

    #[test]
    fn the_overlays_government_is_the_systems() {
        let catalog = bordered();
        let mut session = everything_seen(&catalog, Rulebook::default());
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_govt(SystemId(133), Some(Some(GovtId(128))));
            overlay.set_govt(SystemId(131), Some(None));
        });
        assert_eq!(coarse(&session.map_border_sources()), [130, 133, 135, 138]);
    }

    #[test]
    fn an_uninhabited_system_adds_none_but_the_bibles_any_stellar_counts() {
        let catalog = bordered();
        let session = everything_seen(&catalog, Rulebook::default());
        assert!(!coarse(&session.map_border_sources()).contains(&134));
        let bible = Rulebook::default().with_override(RuleKey::MapExplore, RuleSource::Bible);
        let session = everything_seen(&catalog, bible);
        assert!(
            coarse(&session.map_border_sources()).contains(&134),
            "its fifth slot lived in"
        );
    }

    #[test]
    fn flags2_0x0002_sets_small() {
        let catalog = bordered();
        let session = everything_seen(&catalog, Rulebook::default());
        let small: Vec<(i16, bool)> = session
            .map_border_sources()
            .coarse
            .iter()
            .map(|source| (source.system.0, source.small))
            .collect();
        assert_eq!(
            small,
            [(130, false), (131, false), (135, true), (138, false)]
        );
    }
}
