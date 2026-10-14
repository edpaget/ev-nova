//! Exploration shared by place on the session, as
//! [`RuleKey::ExploredByPlace`] reads it (see
//! [`exploration`](crate::exploration) for the rules themselves).
//!
//! **Raises that include the head** ([`Session::explore_place`]): each
//! explores the system and, by the engine, its place's head.
//!
//! - A jump's arrival (@0x6c16c-0x6c1b8), a hypergate's exit
//!   (@0x63a04-0x63a49) and a wormhole's exit (@0x642ae-0x642f3): the
//!   entry's explore step (see the `entry` module).
//! - The take-off (`_PlayerLandOnStellar` @0x63457-0x634a7):
//!   [`Session::take_off`].
//! - A map grant: `_AutoSetExploration` → `_RecursiveAutoExplore`
//!   raises each reached system's head (@0xd9b6-0xda10), for the N-jump
//!   (@0x44dcb), inhabited-independent (@0x44e12) and government-class
//!   (@0x44e86) grants alike (see the `outfits` module).
//!
//! Only the system itself raises
//! [`StoryEvent::Explored`](crate::story::StoryEvent::Explored): the
//! original's
//! `_ExploreNebulaeFromSystem` runs on it alone. A hypergate's or
//! wormhole's exit raises none: the original's exits write the explored
//! level directly and explore no nebula (see the `nebulae` module).
//!
//! **A raise that does not include the head.** `X` raises only its system
//! (`_EvalSetExp` @0x15c71-0x15ca1; `Session::explore_system`).
//!
//! **Rebuild points** ([`Session::share_explored`]):
//! `_HandleStellarSystemVisibility` raises every system, hidden or not,
//! to its head's level whenever its head is another system
//! (@0x32b13-0x32b8f). The session shares first thing at each rebuild
//! point, docked or not (see the `stellars` module's "Rebuild points"),
//! raising no event. A load's rebuild shares under the default reading,
//! as `Session::fly` runs before `Session::with_rules` (task
//! load-reads-rules-before-fly).
//!
//! **A new pilot** ([`Session::explore_start_neighbours`]): `_DoNewPilot`
//! clears every level, then, for each of the start system's Con slots
//! naming a visible system, sets the head of that system's place
//! (@0x18f97-0x19003), and last the start system's head
//! (@0x19005-0x1902e). Both readings explore the neighbours, as the
//! neighbour loop is the original's and the Bible says nothing of it: by
//! the engine each neighbour's head, by the other reading the neighbour
//! itself. The start system is explored by `Pilot::new`, and its head by
//! the rebuild `Session::begin` ends with, under the engine; visibility
//! is the galaxy as `Session::fly` refreshed it for the new pilot's bits.
//!
//! Left to others: `_ResetPlayer`'s neighbours (@0x1da0f-0x1da7a) and the
//! escape pod's exploration (@0x690cb), roadmap player-death; the known
//! flag and the levels 1 and 2 kept apart, the fidelity roadmap. The
//! arrival message's second raise (@0x70ee6-0x70f2a) changes nothing after
//! the arrival's own and is not modelled.

use super::Session;
use crate::catalog::SystemId;
use crate::exploration::{explored_with, shared_by_place, start_neighbours};
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// How the session shares exploration by place
    /// ([`RuleKey::ExploredByPlace`]).
    fn explored_by_place(&self) -> RuleSource {
        self.rules.source_for(RuleKey::ExploredByPlace)
    }

    /// Explores `system` and, by the engine, its place's head (see the
    /// module docs). Raises no event.
    pub(super) fn explore_place(&mut self, system: SystemId) {
        let source = self.explored_by_place();
        for explored in explored_with(system, self.galaxy.map().places(), source) {
            self.pilot.explore(explored);
        }
    }

    /// A rebuild point's share (see the module docs): by the engine,
    /// every system at an explored head's place is explored. Raises no
    /// event.
    pub(super) fn share_explored(&mut self) {
        let source = self.explored_by_place();
        let shared = shared_by_place(&self.pilot.explored, self.galaxy.map().places(), source);
        for system in shared {
            self.pilot.explore(system);
        }
    }

    /// A new pilot's neighbours (see the module docs): each of the start
    /// system's Con slots naming a visible system explores, by the
    /// engine, that system's head, and by the other reading the system.
    pub(super) fn explore_start_neighbours(&mut self) {
        let source = self.explored_by_place();
        let galaxy = &self.galaxy;
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

    #[test]
    fn a_jump_into_a_member_explores_the_places_head() {
        let catalog = shadowed(catalog());
        for source in RuleSource::ALL {
            let mut session = flown(&catalog, source);
            assert!(!session.galaxy().visible(SystemId(127)));
            assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
            assert!(explored(&session, 131));
            assert_eq!(
                explored(&session, 127),
                source == RuleSource::Engine,
                "{source:?}"
            );
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
            assert!(explored(&session, 131));
            assert_eq!(
                explored(&session, 127),
                source == RuleSource::Engine,
                "{source:?}"
            );
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
            assert!(explored(&session, 131));
            assert_eq!(
                explored(&session, 127),
                source == RuleSource::Engine,
                "{source:?}"
            );
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
        }
    }

    #[test]
    fn exploring_a_head_raises_nothing_more() {
        let catalog = shadowed(catalog());
        let mut session = flown(&catalog, RuleSource::Engine);
        assert!(!explored(&session, 127));
        let before: Vec<SystemId> = session.pilot().explored().collect();
        session.explore_place(SystemId(127));
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
        session.explore_place(SystemId(133));
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
        }
    }
}
