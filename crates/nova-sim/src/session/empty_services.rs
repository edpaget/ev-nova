//! Which of the port's services have nothing to list
//! ([`EmptyServices`], [`RuleKey::EmptyServices`]): the Mission BBS
//! while no offer of the visit at the mission computer is available, and
//! the bar's Hire Escort while the bar lists no ship for hire. It reads
//! the mission run and the bar together, so it is the session's.
//!
//! By the engine reading (the default) nothing is named and nothing is
//! drawn: the original enables the BBS of every inhabited stellar
//! (`_DrawPlanetButtons` @0x2968d, wherever `_canDoMissions` holds,
//! @0x5fab6, @0x4f069), answering #353 when nothing is available
//! (`_DoMissionDialog` @0xa38bd-0xa3956), and greys Hire Escort only
//! with no room (`_CanHireEscorts`; `_DrawBarButtons` @0x26e1d, @0x5795),
//! answering #224 with nothing for hire (`_DoShipyardDialog` @0x5e8ab,
//! @0x5e8da). By the other reading the bar's list is built on the
//! flight's chance, its day's rolls kept as a hire list's are
//! (`Port::hire_rolls`), so the list the bar opens later is the same.

use super::Session;
use crate::chance::Chance;
use crate::flight_frame::EmptyServices;
use crate::rulebook::{RuleKey, RuleSource};

impl Session {
    /// Which of the port's services have nothing to list now, under
    /// [`RuleKey::EmptyServices`] (see the module docs): by the engine
    /// reading always [`EmptyServices::default`], drawing nothing; by the
    /// other, the BBS when the computer lists nothing available
    /// ([`Session::computer_lists_none`]) and Hire Escort when the bar
    /// lists no ship ([`Session::escorts_for_hire`]), the day's hire rolls
    /// not drawn yet drawn on `chance`. In flight, neither.
    pub(crate) fn empty_services(&mut self, chance: &mut dyn Chance) -> EmptyServices {
        if self.setup.rules.source_for(RuleKey::EmptyServices) == RuleSource::Engine
            || self.mode.landed().is_none()
        {
            return EmptyServices::default();
        }
        EmptyServices {
            mission_bbs: self.computer_lists_none(),
            hire_escort: self
                .escorts_for_hire(chance)
                .is_some_and(|list| list.rows.is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MissionId;
    use crate::chance::NeverFires;
    use crate::mission::{ActiveMission, Offer, SLOTS, Venue};
    use crate::rulebook::Rulebook;
    use crate::session::ComputerRefusal;
    use crate::session::mission_map::tests::landed;
    use crate::session::screen::port::tests::{FirstFires, port};
    use crate::testkit::{FakePilotCatalog, mission};

    /// The other reading of [`RuleKey::EmptyServices`].
    fn greying() -> Rulebook {
        Rulebook::default().with_override(RuleKey::EmptyServices, RuleSource::Bible)
    }

    /// [`port`]'s bar and shops at 128, with every class's `HireRandom`
    /// `hire` and `missions` the only `mïsn`.
    fn bar(hire: i16, missions: Vec<crate::catalog::MissionRecord>) -> FakePilotCatalog {
        let mut catalog = port();
        for record in &mut catalog.ship_records {
            record.hire_random = hire;
        }
        catalog.missions = missions;
        catalog
    }

    /// Mission 300, at the mission computer, to 140.
    fn at_the_computer() -> crate::catalog::MissionRecord {
        crate::catalog::MissionRecord {
            travel_stel: 140,
            ..mission(300)
        }
    }

    #[test]
    fn with_no_ship_for_hire_and_no_computer_offer_both_are_empty() {
        let catalog = bar(0, Vec::new());
        let mut session = landed(&catalog).with_rules(greying());
        assert_eq!(
            session.empty_services(&mut NeverFires),
            EmptyServices {
                mission_bbs: true,
                hire_escort: true,
            }
        );
    }

    #[test]
    fn a_ship_for_hire_fills_hire_escort() {
        let catalog = bar(100, Vec::new());
        let mut session = landed(&catalog).with_rules(greying());
        let empty = session.empty_services(&mut NeverFires);
        assert!(!empty.hire_escort, "{empty:?}");
        assert!(empty.mission_bbs, "{empty:?}");
    }

    #[test]
    fn the_hire_list_is_built_on_the_chance_and_its_rolls_kept() {
        let catalog = bar(50, Vec::new());
        let mut session = landed(&catalog).with_rules(greying());
        let mut chance = FirstFires::default();
        let first = session.empty_services(&mut chance);
        assert!(chance.draws > 0, "precondition: the hire rolls drawn");
        let drawn = chance.draws;
        assert_eq!(session.empty_services(&mut chance), first, "kept");
        assert_eq!(chance.draws, drawn, "drawn once");
        assert_eq!(
            session
                .escorts_for_hire(&mut chance)
                .is_some_and(|list| list.rows.is_empty()),
            first.hire_escort,
            "the bar lists what was read"
        );
    }

    #[test]
    fn a_computer_offer_fills_the_bbs_until_it_is_accepted() {
        let catalog = bar(0, vec![at_the_computer()]);
        let mut session = landed(&catalog).with_rules(greying());
        assert!(
            !session.empty_services(&mut NeverFires).mission_bbs,
            "offered"
        );
        session
            .accept_mission(MissionId(300), &mut NeverFires)
            .expect("accepted");
        assert!(session.empty_services(&mut NeverFires).mission_bbs, "taken");
    }

    #[test]
    fn with_every_slot_taken_an_offer_still_fills_the_bbs() {
        let catalog = bar(0, vec![at_the_computer()]);
        let mut session = landed(&catalog).with_rules(greying());
        let offer = Offer {
            mission: MissionId(150),
            ..session.mission_offers(Venue::Computer)[0].clone()
        };
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(150), &offer, session.date());
            session.pilot.missions.fill(active);
        }
        assert_eq!(
            session.mission_computer(),
            Err(ComputerRefusal::SlotsFull),
            "precondition: every slot taken"
        );
        assert!(!session.empty_services(&mut NeverFires).mission_bbs);
    }

    #[test]
    fn by_the_engine_nothing_is_empty_and_nothing_is_drawn() {
        let catalog = bar(50, Vec::new());
        let mut session = landed(&catalog);
        assert_eq!(
            session.setup.rules.source_for(RuleKey::EmptyServices),
            RuleSource::Engine,
            "precondition: the default reading"
        );
        let before = session.clone();
        let mut chance = FirstFires::default();
        assert_eq!(
            session.empty_services(&mut chance),
            EmptyServices::default()
        );
        assert_eq!(chance.draws, 0, "no hire roll drawn");
        assert_eq!(session, before, "no roll kept");
    }

    #[test]
    fn in_flight_neither_is_empty() {
        let catalog = bar(0, Vec::new());
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_rules(greying());
        assert_eq!(
            session.empty_services(&mut NeverFires),
            EmptyServices::default()
        );
    }
}
