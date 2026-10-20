//! Exploration shared by place on the session, as
//! [`RuleKey::ExploredByPlace`] reads it (see
//! [`exploration`](crate::exploration) for the rules themselves), at the
//! level each writer gives ([`ExploredLevel`]; the original's `sÿst`
//! +0x90). Every writer only raises a level, never lowers it.
//!
//! **Raises that include the head** ([`Session::explore_place`]): each
//! raises the system and, by the engine, its place's head.
//!
//! - Level 1: a jump's arrival, the head then the system
//!   (@0x6c184-0x6c1b8), a hypergate's exit (@0x63a1a-0x63a49) and a
//!   wormhole's exit (@0x642c4-0x642f3): the entry's explore step (see the
//!   `entry` module).
//! - Level 2: the take-off (`_PlayerLandOnStellar` raises the head, then
//!   the system, @0x6346d-0x6349e, then `_AutoSetExploration(system, 0,
//!   2)` raises both again @0x634b1-0x634c8): [`Session::take_off`].
//! - Level 2: a map grant: `_AutoSetExploration` → `_RecursiveAutoExplore`
//!   raises each reached system (@0xd9a6-0xd9af) and its head
//!   (@0xda07-0xda10), for the N-jump (@0x44dcb), inhabited-independent
//!   (@0x44e12) and government-class (@0x44e86) grants alike (see the
//!   `outfits` module).
//!
//! Only the system itself raises
//! [`StoryEvent::Explored`](crate::story::StoryEvent::Explored): the
//! original's
//! `_ExploreNebulaeFromSystem` runs on it alone. A hypergate's or
//! wormhole's exit raises none: the original's exits write the explored
//! level directly and explore no nebula (see the `nebulae` module).
//!
//! **A raise that does not include the head.** `X` raises only its system,
//! to level 1 (`_EvalSetExp` @0x15c71-0x15ca1; `Session::explore_system`).
//!
//! **Rebuild points** ([`Session::share_explored`]):
//! `_HandleStellarSystemVisibility` raises every system, hidden or not,
//! to its head's level whenever its head is another system
//! (@0x32b13-0x32b8f, @0x32b5a-0x32b6b). The session shares first thing at
//! each rebuild point, docked or not (see the `stellars` module's
//! "Rebuild points"), raising no event: a level-1 head explores its
//! members, a level-2 head charts them. A load's rebuild shares under the
//! default reading, as `Session::fly` runs before `Session::with_rules`
//! (task load-reads-rules-before-fly).
//!
//! **A new pilot** ([`Session::explore_start_neighbours`]): `_DoNewPilot`
//! clears every level, then, for each of the start system's Con slots
//! naming a visible system, sets the head of that system's place to level
//! 1 (@0x18f97-0x19003, @0x18fe6), and last the start system's head
//! (@0x19005-0x1902e). Both readings explore the neighbours, as the
//! neighbour loop is the original's and the Bible says nothing of it: by
//! the engine each neighbour's head, by the other reading the neighbour
//! itself. The start system is explored by `Pilot::new`, and its head by
//! the rebuild `Session::begin` ends with, under the engine; visibility
//! is the galaxy as `Session::fly` refreshed it for the new pilot's bits.
//!
//! **A load** keeps each system's level, as the original's pilot file
//! does (`_LoadPilotData` @0x7553f-0x75565; see the save's version 29).
//!
//! **Known** ([`Session::is_known`]; see the
//! [`exploration`](crate::exploration) module's "Known"). Each rebuild
//! point works the known set out afresh right after its share
//! (`Session::rework_known`, @0x32b7b, @0x32b91-0x32c10), so a new
//! pilot, a loaded one and a fresh session are known from before their
//! first frame, as each ends in a rebuild. A map grant adds its pass,
//! with no clear, after its walk (`Session::add_known`, @0xdb06-0xdbd4).
//! The take-off's `_AutoSetExploration` runs the same pass
//! (@0x634c8), but needs none of its own here: the dock request's rebuild
//! point ends the take-off (@0x674c7; `Session::lift_off`) and works the
//! known set out afresh from what the take-off explored, as the
//! original's does after the pass. The hypergate map's arrows mark their
//! systems ([`Session::mark_known`], @0xf45f).
//!
//! Left to others: `_ResetPlayer`'s neighbours (@0x1da0f-0x1da7a) and the
//! escape pod's exploration (@0x690cb), roadmap player-death. The
//! arrival message's second raise (@0x70ee6-0x70f2a) changes nothing after
//! the arrival's own and is not modelled.

use std::collections::BTreeSet;

use super::{Session, Setup};
use crate::catalog::SystemId;
use crate::exploration::{explored_with, known_from, shared_by_place, start_neighbours};
use crate::pilot::ExploredLevel;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// Raises `system` and, by the engine, its place's head to `level`
    /// when lower (see the module docs). Raises no event.
    pub(super) fn explore_place(&mut self, system: SystemId, level: ExploredLevel) {
        let source = self.setup.explored_by_place();
        for explored in explored_with(system, self.chart.galaxy.map().places(), source) {
            self.pilot.raise_explored(explored, level);
        }
    }

    /// A rebuild point's share (see the module docs): by the engine,
    /// every system at an explored head's place is raised to the head's
    /// level. Raises no event.
    pub(super) fn share_explored(&mut self) {
        let source = self.setup.explored_by_place();
        let places = self.chart.galaxy.map().places();
        let explored = shared_by_place(&self.pilot.explored, places, source);
        let charted = shared_by_place(&self.pilot.charted, places, source);
        for system in explored {
            self.pilot.explore(system);
        }
        for system in charted {
            self.pilot.chart(system);
        }
    }

    /// Whether `system` is known (+0x1ec; see the module docs' "Known").
    #[must_use]
    pub fn is_known(&self, system: SystemId) -> bool {
        self.chart.known.contains(&system)
    }

    /// The systems known, by ascending ID.
    #[must_use]
    pub fn known(&self) -> &BTreeSet<SystemId> {
        &self.chart.known
    }

    /// A rebuild point's known pass (see the module docs' "Known"): every
    /// flag cleared (`_HandleStellarSystemVisibility` @0x32b7b), then each
    /// visible, explored system and its links by place marked
    /// (@0x32b91-0x32c10).
    pub(super) fn rework_known(&mut self) {
        self.chart.known = known_from(&self.chart.galaxy, &self.pilot.explored);
    }

    /// A map grant's known pass, after its walk, with no clear
    /// (`_AutoSetExploration` @0xdb06-0xdbd4).
    pub(super) fn add_known(&mut self) {
        let known = known_from(&self.chart.galaxy, &self.pilot.explored);
        self.chart.known.extend(known);
    }

    /// Marks `systems` known, as the hypergate map marks each system it
    /// arrows (`_DrawMap` @0xf457-0xf45f): the flight view marks the
    /// gate's offers as its map opens (see the
    /// [`galaxy_map`](crate::galaxy_map) module).
    pub fn mark_known(&mut self, systems: impl IntoIterator<Item = SystemId>) {
        self.chart.known.extend(systems);
    }

    /// A new pilot's neighbours (see the module docs): each of the start
    /// system's Con slots naming a visible system explores, by the
    /// engine, that system's head, and by the other reading the system.
    pub(super) fn explore_start_neighbours(&mut self) {
        let source = self.setup.explored_by_place();
        let galaxy = &self.chart.galaxy;
        let neighbours = start_neighbours(
            galaxy.con_slots(self.pilot.system),
            |system| galaxy.exists(system) && galaxy.visible(system),
            galaxy.map().places(),
            source,
        );
        for system in neighbours {
            self.pilot.explore(system);
        }
    }
}

impl Setup {
    /// How the session shares exploration by place
    /// ([`RuleKey::ExploredByPlace`]).
    fn explored_by_place(&self) -> RuleSource {
        self.rules.source_for(RuleKey::ExploredByPlace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::SystemVisibility;
    use crate::chance::NeverFires;
    use crate::control::Test;
    use crate::pilot::Pilot;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{at_gate, at_wormhole, gated, wormholes};
    use crate::testkit::{
        FakePilotCatalog, Scripted, catalog, hideable, hiding_bit, jump, land_now, star,
    };

    /// `base` with 129 at 130's place and 127 at 131's, each the head of
    /// its place and hidden by its `Visibility` (while bit 1 is clear),
    /// every other system [`hideable`].
    fn shadowed(mut base: FakePilotCatalog) -> FakePilotCatalog {
        for head in [(129, 0.0), (127, 600.0)] {
            base.star_map.push(star(head.0, (head.1, 0.0), &[]));
            base.visibility.push(SystemVisibility {
                id: SystemId(head.0),
                visibility: Test::parse("b1"),
            });
        }
        hideable(&mut base);
        base
    }

    fn rules(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::ExploredByPlace, source)
    }

    /// `catalog`'s new pilot flown, sharing exploration as `source` says.
    fn flown(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "").expect("starts"))
            .expect("flies")
            .with_rules(rules(source))
    }

    fn explored(session: &Session, system: i16) -> bool {
        session.pilot().has_explored(SystemId(system))
    }

    /// The arrival into 131 raised it, and by the engine its head 127, to
    /// level 1 (@0x6c184-0x6c1b8, @0x63a1a-0x63a49, @0x642c4-0x642f3).
    fn assert_arrived_at_level_1(session: &Session, source: RuleSource) {
        assert_eq!(level(session, 131), ExploredLevel::Explored, "{source:?}");
        let head = if source == RuleSource::Engine {
            ExploredLevel::Explored
        } else {
            ExploredLevel::Unexplored
        };
        assert_eq!(level(session, 127), head, "{source:?}");
    }

    #[test]
    fn an_arrival_leaves_a_charted_system_and_head_charted() {
        let jumps = shadowed(catalog());
        let gates = shadowed(gated());
        let holes = shadowed(wormholes());
        for source in RuleSource::ALL {
            let chart = |mut session: Session| {
                session.pilot.chart(SystemId(131));
                session.pilot.chart(SystemId(127));
                session
            };
            let still_charted = |session: &Session, how: &str| {
                assert_eq!(
                    level(session, 131),
                    ExploredLevel::Charted,
                    "{source:?} {how}"
                );
                assert_eq!(
                    level(session, 127),
                    ExploredLevel::Charted,
                    "{source:?} {how}"
                );
            };
            let mut session = chart(flown(&jumps, source));
            assert_eq!(jump(&mut session, &jumps, 131), Some(SystemId(131)));
            still_charted(&session, "a jump");
            let mut session = chart(at_gate(&gates).with_rules(rules(source)));
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &gates, &mut Scripted::default()),
                Ok(SystemId(131))
            );
            still_charted(&session, "a gate");
            let mut session = chart(at_wormhole(&holes).with_rules(rules(source)));
            assert_eq!(
                session.enter_wormhole(&holes, &mut Scripted::rolling(&[0])),
                Ok(SystemId(131))
            );
            still_charted(&session, "a wormhole");
        }
    }

    #[test]
    fn a_jump_into_a_member_explores_the_places_head() {
        let catalog = shadowed(catalog());
        for source in RuleSource::ALL {
            let mut session = flown(&catalog, source);
            assert!(!session.galaxy().visible(SystemId(127)));
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
            assert_arrived_at_level_1(&session, source);
        }
    }

    #[test]
    fn a_gate_exit_explores_the_places_head() {
        let catalog = shadowed(gated());
        for source in RuleSource::ALL {
            let mut session = at_gate(&catalog).with_rules(rules(source));
            assert!(!session.galaxy().visible(SystemId(127)));
            assert_eq!(
                session.enter_hypergate(Some(SystemId(131)), &catalog, &mut Scripted::default()),
                Ok(SystemId(131))
            );
            assert_arrived_at_level_1(&session, source);
        }
    }

    #[test]
    fn a_wormhole_exit_explores_the_places_head() {
        let catalog = shadowed(wormholes());
        for source in RuleSource::ALL {
            let mut session = at_wormhole(&catalog).with_rules(rules(source));
            assert_eq!(
                session.enter_wormhole(&catalog, &mut Scripted::rolling(&[0])),
                Ok(SystemId(131))
            );
            assert_arrived_at_level_1(&session, source);
        }
    }

    #[test]
    fn a_take_off_explores_the_places_head() {
        let catalog = shadowed(catalog());
        for source in RuleSource::ALL {
            let mut session = flown(&catalog, source);
            land_now(&mut session).expect("lands");
            assert!(!explored(&session, 129), "not by the landing");
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                explored(&session, 129),
                source == RuleSource::Engine,
                "{source:?}"
            );
            // Level 2 (`_PlayerLandOnStellar` @0x6346d-0x6349e).
            assert_eq!(level(&session, 130), ExploredLevel::Charted, "{source:?}");
            let head = if source == RuleSource::Engine {
                ExploredLevel::Charted
            } else {
                ExploredLevel::Unexplored
            };
            assert_eq!(level(&session, 129), head, "{source:?}");
        }
    }

    #[test]
    fn exploring_a_head_raises_nothing_more() {
        let catalog = shadowed(catalog());
        let mut session = flown(&catalog, RuleSource::Engine);
        assert!(!explored(&session, 127));
        let before: Vec<SystemId> = session.pilot().explored().collect();
        session.explore_place(SystemId(127), ExploredLevel::Explored);
        let after: Vec<SystemId> = session.pilot().explored().collect();
        assert_eq!(after.len(), before.len() + 1);
        assert!(explored(&session, 127));
        assert!(!explored(&session, 131), "not its member");
    }

    #[test]
    fn exploring_a_system_at_no_place_raises_only_itself() {
        let catalog = shadowed(catalog());
        let mut session = flown(&catalog, RuleSource::Engine);
        let before: Vec<SystemId> = session.pilot().explored().collect();
        session.explore_place(SystemId(133), ExploredLevel::Explored);
        let after: Vec<SystemId> = session.pilot().explored().collect();
        assert_eq!(after.len(), before.len() + 1);
        assert!(explored(&session, 133));
    }

    #[test]
    fn a_rebuild_point_shares_an_explored_head_with_its_place() {
        let catalog = shadowed(catalog());
        for source in RuleSource::ALL {
            let mut session = flown(&catalog, source);
            session.pilot.explore(SystemId(127));
            assert!(!explored(&session, 131));
            session.share_explored();
            assert_eq!(
                explored(&session, 131),
                source == RuleSource::Engine,
                "{source:?}"
            );
        }
    }

    fn level(session: &Session, system: i16) -> ExploredLevel {
        session.pilot().explored_level(SystemId(system))
    }

    #[test]
    fn a_rebuild_point_shares_each_heads_level_with_its_place() {
        let catalog = shadowed(catalog());
        for source in RuleSource::ALL {
            let shared = |level: ExploredLevel| {
                if source == RuleSource::Engine {
                    level
                } else {
                    ExploredLevel::Unexplored
                }
            };
            // 127 charted: its member 131 is charted.
            let mut session = flown(&catalog, source);
            session.pilot.chart(SystemId(127));
            session.share_explored();
            assert_eq!(
                level(&session, 131),
                shared(ExploredLevel::Charted),
                "{source:?}"
            );
            // 127 at level 1: 131 is explored, not charted.
            let mut session = flown(&catalog, source);
            session.pilot.explore(SystemId(127));
            session.share_explored();
            assert_eq!(
                level(&session, 131),
                shared(ExploredLevel::Explored),
                "{source:?}"
            );
            // 130 charted under 129 at level 1: 130 stays charted, and 129
            // takes nothing from its member.
            let mut session = flown(&catalog, source);
            session.pilot.explore(SystemId(129));
            session.pilot.chart(SystemId(130));
            session.share_explored();
            assert_eq!(level(&session, 130), ExploredLevel::Charted, "{source:?}");
            assert_eq!(level(&session, 129), ExploredLevel::Explored, "{source:?}");
        }
    }

    #[test]
    fn a_new_pilot_explores_each_visible_neighbours_head() {
        // 130 lists 131 (127's place), 133, 132 hidden, and 999, no system.
        let mut catalog = shadowed(catalog());
        catalog.star_map[0].links = [131, 133, 132, 999].map(SystemId).to_vec();
        for source in RuleSource::ALL {
            let mut pilot = Pilot::new(&catalog, "").expect("starts");
            pilot.bits_mut().set(hiding_bit(132));
            let mut session = Session::fly(&catalog, pilot)
                .expect("flies")
                .with_rules(rules(source));
            session.explore_start_neighbours();
            let explored: Vec<SystemId> = session.pilot().explored().collect();
            let expected: &[i16] = match source {
                RuleSource::Engine => &[127, 130, 133],
                RuleSource::Bible => &[130, 131, 133],
            };
            assert_eq!(
                explored,
                expected.iter().copied().map(SystemId).collect::<Vec<_>>(),
                "{source:?}"
            );
            // At level 1 (`_DoNewPilot` @0x18fe6, @0x1902e).
            assert_eq!(session.pilot().charted().count(), 0, "{source:?}");
        }
    }

    // Known.

    fn known(session: &Session) -> Vec<i16> {
        session.known().iter().map(|system| system.0).collect()
    }

    #[test]
    fn a_new_a_started_and_a_loaded_pilot_are_known_before_the_first_frame() {
        // 130, explored, lists 131; 131 lists 132.
        let catalog = catalog();
        let pilot = Pilot::new(&catalog, "").expect("starts");
        let session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert_eq!(known(&session), [130, 131]);
        assert_eq!(
            known(&Session::start(&catalog).expect("starts")),
            [130, 131]
        );
        let loaded = Session::load(&catalog, pilot, &Rulebook::default()).expect("loads");
        assert_eq!(known(&loaded), [130, 131]);
        assert!(loaded.is_known(SystemId(131)));
        assert!(!loaded.is_known(SystemId(132)));
    }

    #[test]
    fn a_rebuild_point_works_the_known_set_out_afresh() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.mark_known([SystemId(133)]);
        session.pilot.explore(SystemId(131));
        assert_eq!(known(&session), [130, 131, 133], "nothing until then");
        session.rebuild_quietly();
        assert_eq!(
            known(&session),
            [130, 131, 132],
            "133 forgotten, 132 learnt"
        );
    }

    #[test]
    fn marking_known_adds_and_forgets_nothing() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.mark_known([SystemId(133), SystemId(130)]);
        assert_eq!(known(&session), [130, 131, 133]);
    }
}
