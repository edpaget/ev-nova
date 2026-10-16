//! What the galaxy map asks of the session: its drawing
//! ([`Session::map_drawing`], the session's side of
//! [`galaxy_map::draw_map`]), the colour each system is framed in
//! ([`Session::system_color_code`], `_ColorCodeSystem`), the mission
//! systems its red arrows mark ([`Session::map_mission_systems`],
//! `_RecalcMissionSystsForMap`) and the floating map's selection
//! ([`Session::floating_preselection`]). See the
//! [`galaxy_map`] and [`iff`](crate::iff) modules for
//! the original's rules.
//!
//! **The colour code's inputs.** A system's head is its place's lowest-ID
//! system ([`Places::head`](crate::hyperspace::Places::head)) and the
//! system standing for it [`Places::stand_in`](crate::hyperspace::Places::stand_in).
//! The stellars read are every stellar the system standing for it lists
//! ([`ListedStellar`](crate::catalog::ListedStellar)), each active while
//! a visible system holds it
//! ([`OverlaidGalaxy::active_holder`](crate::story::OverlaidGalaxy::active_holder),
//! as `_HandleStellarSystemVisibility` sets +0x44 from its holder's
//! visibility after the rescue, @0x32c58-0x32cdf), with the overlay's
//! government and destruction. Their order doesn't matter, as a bucket
//! only asks whether it has any. Each is dominated by its always-dominated
//! bit (`Session::iff_dominated`) and blessed by the session's blessing
//! table ([`Session::blessing`]), as the radar's IFF reads both. Then:
//!
//! - **Inhabited** is [`exploration::inhabited`] as
//!   [`RuleKey::MapExplore`] reads it: by the engine the first four
//!   `NavDefs` slots, `_SystemIsInhabited` inlined (@0xd776-0xd7b7); by
//!   the other reading any stellar.
//! - **Explored** is read at the head by
//!   [`RuleKey::ExploredByPlace`]'s engine reading (@0xd75b-0xd764), and
//!   at the system itself by its other reading, where each system is
//!   explored on its own.
//! - **The record** is the player's record in the head by
//!   [`RuleKey::LegalRecords`]' engine reading (`_playerRecord[head]`,
//!   @0xd864-0xd8a1), and the record with the system's government (0 for
//!   an independent one) by the Bible's.

use std::collections::BTreeMap;

use super::Session;
use crate::catalog::{ListedStellar, SystemId};
use crate::exploration;
use crate::galaxy_map::{self, HIDES_ARROW, MARKS_SHIPS, MapDrawing, MapState};
use crate::iff::{self, MapStellar, Standing, SystemColorCode};
use crate::mission::ships::ShipSystem;
use crate::navigation::NavMode;
use crate::rank::{Blessed, Blessing};
use crate::rulebook::{RuleKey, RuleSource};

/// How many entries `_missionSyst` holds.
const MISSION_SYSTEMS: usize = 32;

/// What the colour codes of one drawing read once: the blessing table and
/// each system's listed stellars.
struct Coding<'a> {
    blessing: Blessing,
    listed: BTreeMap<SystemId, Vec<&'a ListedStellar>>,
}

impl Session {
    /// What the colour codes read, worked out once.
    fn coding(&self) -> Coding<'_> {
        let mut listed: BTreeMap<SystemId, Vec<&ListedStellar>> = BTreeMap::new();
        for stellar in &self.listed_stellars {
            for &system in &stellar.systems {
                listed.entry(system).or_default().push(stellar);
            }
        }
        Coding {
            blessing: self.blessing(),
            listed,
        }
    }

    /// What the galaxy map frames `system` in, as `_ColorCodeSystem`
    /// works it out (see the module docs).
    #[must_use]
    pub fn system_color_code(&self, system: SystemId) -> SystemColorCode {
        self.color_code(&self.coding(), system)
    }

    /// [`Session::system_color_code`] with `coding` read already.
    fn color_code(&self, coding: &Coding<'_>, system: SystemId) -> SystemColorCode {
        let places = self.galaxy.map().places();
        let head = places.head(system);
        let stood = if self.galaxy.exists(system) {
            places.stand_in(system)
        } else {
            None
        };
        let explored = match self.rules.source_for(RuleKey::ExploredByPlace) {
            RuleSource::Engine => head.is_some_and(|head| self.pilot.has_explored(head)),
            RuleSource::Bible => self.pilot.has_explored(system),
        };
        let record = match self.rules.source_for(RuleKey::LegalRecords) {
            RuleSource::Engine => head.map_or(0, |head| self.pilot.system_record(head)),
            RuleSource::Bible => self
                .galaxy
                .govt(system)
                .map_or(0, |govt| self.pilot.legal_record(govt)),
        };
        let Some(active) = stood else {
            return iff::system_color_code(false, explored, false, record, &[]);
        };
        let inhabited = exploration::inhabited(
            self.galaxy.map().stellars(active),
            self.rules.source_for(RuleKey::MapExplore),
        );
        let stellars: Vec<MapStellar> = coding
            .listed
            .get(&active)
            .into_iter()
            .flatten()
            .map(|stellar| {
                let govt = self
                    .galaxy
                    .overlay()
                    .stellar(stellar.id)
                    .govt
                    .unwrap_or(stellar.govt);
                MapStellar {
                    active: self
                        .galaxy
                        .active_holder(&stellar.systems, self.pilot.system)
                        .is_some(),
                    flags: stellar.flags,
                    flags2: stellar.flags2,
                    destroyed: self.galaxy.destroyed(stellar.id),
                    min_status: stellar.min_status,
                    standing: Standing {
                        dominated: Self::dominated_by(stellar.flags2),
                        blessed: coding.blessing.blesses(govt, Blessed::Landing),
                    },
                }
            })
            .collect();
        iff::system_color_code(true, explored, inhabited, record, &stellars)
    }

    /// The mission systems the galaxy map's red arrows mark, as
    /// `_RecalcMissionSystsForMap` works them out (@0xe1b5-0xe2d1; see the
    /// [`galaxy_map`] module): for each mission held,
    /// in slot order, its travel stellar's system while the stellar is
    /// active, else none, its return stellar's in its place once its
    /// destination is reached, when that is another stellar and active;
    /// pushed unless its `Flags` has [`HIDES_ARROW`]. Then, with
    /// [`MARKS_SHIPS`] and not [`HIDES_ARROW`], ships left and a ship
    /// system, the system standing for the ship system. At most 32.
    ///
    /// A stellar's system is its holder (+0x14) while that is visible
    /// ([`OverlaidGalaxy::active_holder`](crate::story::OverlaidGalaxy::active_holder)):
    /// active (+0x44) exactly then. The ship system is +0x10 as the accept
    /// resolved it ([`ShipSystem`]): one that follows the player (−6) or
    /// is nowhere (−1) is below 0 and pushes nothing (@0xe28c-0xe28f).
    #[must_use]
    pub fn map_mission_systems(&self) -> Vec<SystemId> {
        let active = |stellar| {
            self.galaxy
                .active_holder(self.listers(stellar), self.pilot.system)
        };
        let mut systems = Vec::new();
        for (_, mission) in self.pilot.missions().iter() {
            let destination = mission.destination.map(|stop| stop.stellar);
            let mut system = destination.and_then(active);
            if let Some(back) = mission.return_to.map(|stop| stop.stellar)
                && Some(back) != destination
                && mission.reached
                && let Some(holder) = active(back)
            {
                system = Some(holder);
            }
            let flags = self
                .mission_record(mission.mission)
                .map_or(0, |record| record.flags);
            let hidden = flags & HIDES_ARROW != 0;
            if let Some(system) = system
                && !hidden
            {
                systems.push(system);
            }
            if flags & MARKS_SHIPS != 0
                && !hidden
                && mission.ships.left > 0
                && let ShipSystem::At(at) = mission.ships.system
                && let Some(standing) = self.galaxy.map().places().stand_in(at)
            {
                systems.push(standing);
            }
        }
        systems.truncate(MISSION_SYSTEMS);
        systems
    }

    /// The system the floating map selects as it shows, as
    /// `_ShowFloatingMap` sets `_selSystem` (@0x1147b-0x114c8): in mode 3
    /// with a Con slot selected (+0x6c 0-15), that slot's system, raw,
    /// when it is visible (+0x1eb); otherwise none. An empty slot (−1),
    /// whose +0x1eb the original reads out of bounds, selects none. The
    /// floating map's names read it above a zoom of 1.1 (`_DrawMap`
    /// @0xf52b-0xf533).
    #[must_use]
    pub fn floating_preselection(&self) -> Option<SystemId> {
        let slot = self.hyper_slot()?;
        let system = *self.galaxy.con_slots(self.pilot.system).get(slot)?;
        (self.galaxy.exists(system) && self.galaxy.visible(system)).then_some(system)
    }

    /// The current system's Con slot selected in mode 3 (+0x6c), if any:
    /// the one the raw selection stands in (`Session::selected_slot`),
    /// always one of the first 16.
    fn hyper_slot(&self) -> Option<usize> {
        let NavMode::Hyperspace(Some(selected)) = self.nav else {
            return None;
        };
        self.selected_slot(selected)
    }

    /// What the galaxy map draws now ([`galaxy_map::draw_map`]): on the
    /// hypergate map when `hypergate` holds the systems its gate arrows,
    /// with `missions` the mission systems taken as it opened
    /// ([`Session::map_mission_systems`]), no green mission system and
    /// multiple arrows allowed (see the [`galaxy_map`]
    /// module), each system framed as [`Session::system_color_code`]
    /// says. The course is the one saved, raw. The hypergate map's
    /// arrows mark their systems known as they are drawn: the caller
    /// does that with [`Session::mark_known`] once it has drawn them.
    #[must_use]
    pub fn map_drawing(&self, hypergate: Option<&[SystemId]>, missions: &[SystemId]) -> MapDrawing {
        let coding = self.coding();
        let state = MapState {
            galaxy: &self.galaxy,
            explored: &self.pilot.explored,
            known: &self.known,
            current: self.pilot.system,
            course: &self.pilot.course,
            hyper_slot: self.hyper_slot(),
            hypergate,
            missions,
            green: None,
            multi_arrows: true,
            pairing: self.rules.source_for(RuleKey::MapLinkPairing),
        };
        galaxy_map::draw_map(&state, |system| self.color_code(&coding, system))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, LandingSite, MissionId, StarSystem, StellarId};
    use crate::landing::StellarFlags;
    use crate::mission::Venue;
    use crate::mission::active::ActiveMission;
    use crate::mission::offer::{Offer, Stop};
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::ranks::tests::blessing_rank;
    use crate::testkit::{FakePilotCatalog, catalog, mission, planet, star};

    /// A stellar of `MinStatus` `min_status` and government `govt`.
    fn port(id: i16, min_status: i16, govt: Option<i16>) -> LandingSite {
        LandingSite {
            min_status,
            govt: govt.map(GovtId),
            ..planet(id, 0.0, 0.0)
        }
    }

    /// 130 (the start) lists 131 and 132. 131 holds stellar 140 of
    /// `MinStatus` 10 (government 140, which rank 200 blesses), 132 the
    /// uninhabited 150 first, then 151 (`MinStatus` 0) in its fifth slot;
    /// 133 is far off.
    fn coloured() -> FakePilotCatalog {
        let home = Some((StellarFlags::CAN_LAND, 0));
        let lived = |system: StarSystem, stellars: &[Option<(u32, u16)>]| StarSystem {
            stellars: stellars.to_vec(),
            ..system
        };
        let base = catalog();
        FakePilotCatalog {
            ranks: vec![blessing_rank()],
            systems: [130, 131, 132, 133].map(SystemId).to_vec(),
            star_map: vec![
                lived(star(130, (0.0, 0.0), &[131, 132]), &[home]),
                lived(star(131, (600.0, 0.0), &[]), &[home]),
                lived(
                    star(132, (0.0, 600.0), &[]),
                    &[Some((StellarFlags::UNINHABITED, 0)), None, None, None, home],
                ),
                star(133, (-600.0, 0.0), &[]),
            ],
            sites: vec![
                (SystemId(130), vec![planet(128, 0.0, 0.0)]),
                (SystemId(131), vec![port(140, 10, Some(140))]),
                (
                    SystemId(132),
                    vec![
                        LandingSite {
                            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                            ..planet(150, 0.0, 0.0)
                        },
                        port(151, 0, None),
                    ],
                ),
            ],
            ..base
        }
    }

    fn flown(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules)
    }

    #[test]
    fn the_session_codes_each_case_from_its_own_state() {
        let catalog = coloured();
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Unexplored
        );
        assert_eq!(
            session.system_color_code(SystemId(999)),
            SystemColorCode::Unexplored,
            "no system"
        );
        for system in [131, 132] {
            session.pilot.explore(SystemId(system));
        }
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Denied,
            "record 0 below MinStatus 10"
        );
        session.pilot.set_system_record(SystemId(131), -3);
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Hostile
        );
        session.pilot.set_system_record(SystemId(131), 10);
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Cleared
        );
        assert_eq!(
            session.system_color_code(SystemId(132)),
            SystemColorCode::Uninhabited,
            "only its fifth slot is lived in"
        );
    }

    #[test]
    fn the_map_explore_rule_says_which_slots_are_inhabited() {
        let catalog = coloured();
        let rules = Rulebook::default().with_override(RuleKey::MapExplore, RuleSource::Bible);
        let mut session = flown(&catalog, rules);
        session.pilot.explore(SystemId(132));
        assert_eq!(
            session.system_color_code(SystemId(132)),
            SystemColorCode::Cleared,
            "any stellar counts: 151 lands every record"
        );
    }

    #[test]
    fn the_sessions_blessing_turns_a_refused_system_blue() {
        let catalog = coloured();
        let mut session = flown(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(131));
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Denied
        );
        session.pilot.ranks.insert(crate::catalog::RankId(200));
        session.reset_precalced();
        assert_eq!(
            session.system_color_code(SystemId(131)),
            SystemColorCode::Cleared,
            "government 140 blesses landing"
        );
    }

    /// [`coloured`] with 134 at 131's place, its head, hidden.
    fn shadowed() -> FakePilotCatalog {
        let mut catalog = coloured();
        catalog.star_map.push(star(129, (600.0, 0.0), &[]));
        catalog.systems.push(SystemId(129));
        catalog.visibility.push(crate::catalog::SystemVisibility {
            id: SystemId(129),
            visibility: crate::control::Test::parse("b1"),
        });
        catalog
    }

    #[test]
    fn the_head_is_read_for_the_explored_state_and_record_by_the_engine() {
        let catalog = shadowed();
        for (explored_by_place, legal) in [
            (RuleSource::Engine, RuleSource::Engine),
            (RuleSource::Bible, RuleSource::Bible),
        ] {
            let rules = Rulebook::default()
                .with_override(RuleKey::ExploredByPlace, explored_by_place)
                .with_override(RuleKey::LegalRecords, legal);
            let mut session = flown(&catalog, rules);
            assert!(!session.galaxy().visible(SystemId(129)), "the head hidden");
            session.pilot.explore(SystemId(131));
            session.pilot.set_system_record(SystemId(131), 10);
            let code = session.system_color_code(SystemId(131));
            match explored_by_place {
                RuleSource::Engine => {
                    assert_eq!(code, SystemColorCode::Unexplored, "129 unexplored");
                }
                RuleSource::Bible => {
                    assert_eq!(code, SystemColorCode::Denied, "the Bible's record 0");
                }
            }
            session.pilot.explore(SystemId(129));
            session.pilot.set_system_record(SystemId(129), -5);
            let code = session.system_color_code(SystemId(131));
            match legal {
                RuleSource::Engine => {
                    assert_eq!(code, SystemColorCode::Hostile, "the head's record");
                }
                RuleSource::Bible => assert_eq!(code, SystemColorCode::Denied),
            }
        }
    }

    // The mission systems.

    /// [`coloured`] with mission 300 of `flags`.
    fn on_mission(flags: u16) -> FakePilotCatalog {
        FakePilotCatalog {
            missions: vec![crate::catalog::MissionRecord {
                flags,
                ..mission(300)
            }],
            ..coloured()
        }
    }

    fn stop(stellar: i16) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: None,
        }
    }

    /// `session` holding mission 300 to `to`, back at `back`.
    fn holding(session: &mut Session, to: Option<i16>, back: Option<i16>) -> u8 {
        let offer = Offer {
            mission: MissionId(300),
            venue: Venue::Computer,
            destination: to.map(stop),
            return_to: back.map(stop),
            cargo_type: None,
            cargo_tons: 0,
            pay: 0,
            deadline: None,
        };
        let accepted = ActiveMission::accepted(&mission(300), &offer, session.pilot.date);
        session.pilot.missions.fill(accepted).expect("a slot")
    }

    #[test]
    fn a_mission_marks_its_destination_then_its_return_once_reached() {
        let catalog = on_mission(0);
        let mut session = flown(&catalog, Rulebook::default());
        let slot = holding(&mut session, Some(140), Some(151));
        assert_eq!(session.map_mission_systems(), [SystemId(131)]);
        session.pilot.missions.get_mut(slot).expect("held").reached = true;
        assert_eq!(session.map_mission_systems(), [SystemId(132)]);
        let mut session = flown(&catalog, Rulebook::default());
        let slot = holding(&mut session, Some(140), Some(140));
        session.pilot.missions.get_mut(slot).expect("held").reached = true;
        assert_eq!(
            session.map_mission_systems(),
            [SystemId(131)],
            "back at the same stellar"
        );
        let mut session = flown(&catalog, Rulebook::default());
        holding(&mut session, Some(999), None);
        assert_eq!(session.map_mission_systems(), [], "no system holds it");
    }

    #[test]
    fn flag_0x0002_hides_a_missions_systems() {
        let catalog = on_mission(HIDES_ARROW | MARKS_SHIPS);
        let mut session = flown(&catalog, Rulebook::default());
        let slot = holding(&mut session, Some(140), None);
        let ships = &mut session.pilot.missions.get_mut(slot).expect("held").ships;
        ships.left = 1;
        ships.system = ShipSystem::At(SystemId(133));
        assert_eq!(session.map_mission_systems(), []);
    }

    #[test]
    fn flag_0x0200_marks_the_ships_system_by_place_while_ships_are_left() {
        let catalog = on_mission(MARKS_SHIPS);
        let mut session = flown(&catalog, Rulebook::default());
        let slot = holding(&mut session, None, None);
        assert_eq!(session.map_mission_systems(), [], "no ships left");
        let ships = &mut session.pilot.missions.get_mut(slot).expect("held").ships;
        ships.system = ShipSystem::At(SystemId(133));
        assert_eq!(session.map_mission_systems(), [], "none left there");
        let ships = &mut session.pilot.missions.get_mut(slot).expect("held").ships;
        ships.left = 2;
        ships.system = ShipSystem::FollowsPlayer;
        assert_eq!(session.map_mission_systems(), [], "-6 is below 0");
        let ships = &mut session.pilot.missions.get_mut(slot).expect("held").ships;
        ships.system = ShipSystem::At(SystemId(133));
        assert_eq!(session.map_mission_systems(), [SystemId(133)]);
        let catalog = on_mission(0);
        let mut session = flown(&catalog, Rulebook::default());
        let slot = holding(&mut session, None, None);
        let ships = &mut session.pilot.missions.get_mut(slot).expect("held").ships;
        ships.left = 2;
        ships.system = ShipSystem::At(SystemId(133));
        assert_eq!(session.map_mission_systems(), [], "without 0x0200");
    }

    // The floating map's selection.

    #[test]
    fn the_floating_preselection_is_the_raw_mode_3_slot_when_visible() {
        let catalog = shadowed();
        let mut session = flown(&catalog, Rulebook::default());
        assert_eq!(session.floating_preselection(), None, "nav off");
        session.set_nav(NavMode::Hyperspace(Some(SystemId(131))));
        assert_eq!(session.floating_preselection(), Some(SystemId(131)));
        session.set_nav(NavMode::Hyperspace(None));
        assert_eq!(session.floating_preselection(), None, "no selection");
        session.set_nav(NavMode::Stellar(None));
        assert_eq!(session.floating_preselection(), None, "mode 2");
    }

    #[test]
    fn a_hidden_or_empty_slot_preselects_nothing_and_the_slot_is_raw() {
        // 130 lists an empty slot, then 129 (hidden, at 131's place), then
        // 131.
        let mut catalog = shadowed();
        catalog.star_map[0].links = [-1, 129, 131].map(SystemId).to_vec();
        let mut session = flown(&catalog, Rulebook::default());
        session.set_nav(NavMode::Hyperspace(Some(SystemId(129))));
        assert_eq!(
            session.floating_preselection(),
            None,
            "129 hidden, not resolved to 131"
        );
        session.set_nav(NavMode::Hyperspace(Some(SystemId(-1))));
        assert_eq!(session.floating_preselection(), None, "the empty slot");
    }

    // The drawing.

    #[test]
    fn the_drawing_pairs_links_as_the_rulebook_says() {
        // 132 lists 131 one way; 131, lower, qualifies first.
        let mut catalog = coloured();
        catalog.star_map[2].links = vec![SystemId(131)];
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MapLinkPairing, source);
            let mut session = flown(&catalog, rules);
            for system in [131, 132] {
                session.pilot.explore(SystemId(system));
            }
            session.rework_known();
            let links: Vec<(i16, i16)> = session
                .map_drawing(None, &[])
                .links
                .iter()
                .map(|link| (link.from.0, link.to.0))
                .collect();
            let one_way = links.contains(&(132, 131));
            assert_eq!(
                one_way,
                source == RuleSource::Bible,
                "{source:?}: {links:?}"
            );
        }
    }

    #[test]
    fn the_drawing_reads_the_sessions_known_set_course_and_rules() {
        let catalog = coloured();
        let mut session = flown(&catalog, Rulebook::default());
        session.pilot.course = vec![SystemId(131), SystemId(133)];
        let drawing = session.map_drawing(None, &[]);
        let drawn: Vec<SystemId> = drawing.systems.iter().map(|system| system.id).collect();
        assert_eq!(drawn, [130, 131, 132].map(SystemId), "known, not 133");
        assert_eq!(drawing.course.len(), 2);
        assert_eq!(drawing.named, [SystemId(130)]);
        let gate = [SystemId(133)];
        let drawing = session.map_drawing(Some(&gate), &[SystemId(133)]);
        assert!(drawing.course.is_empty());
        assert_eq!(drawing.gate_arrows, gate);
        let drawn: Vec<SystemId> = drawing.systems.iter().map(|system| system.id).collect();
        assert_eq!(drawn, [130, 131, 132, 133].map(SystemId), "a mission's");
        assert!(!session.is_known(SystemId(133)), "the caller marks it");
    }
}
