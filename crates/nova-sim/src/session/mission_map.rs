//! The marks a mission dialog gives the map it opens: the green mission
//! system and the arrow flag ([`MapMarks`]), as each of the four dialogs
//! that open `_DoSystemMap` on the map key writes `_greenMissionSyst`
//! and `_multiArrowsOnSingleSystem` before the call. All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! | Dialog | Green system | Arrow flag |
//! |---|---|---|
//! | An offer ([`Session::offer_map_marks`], `_OfferOneMission` @0xa3059-0xa30e8) | the offer's | 1 (@0xa3064) |
//! | The mission computer ([`Session::computer_map_marks`], `_DoMissionDialog` @0xa3ce3-0xa3d51) | the selected offer's; none with no row selected | 1 (@0xa3ce0) |
//! | Mission info ([`Session::mission_info_map_marks`], `_DoMissionInfoDialog` @0x9e3d3-0x9e43f) | the selected held mission's | 0 (@0x9e3e2) |
//! | The accept's briefing (`Session::briefing_map_marks`, `_DoMissionAccept` @0xa1e8d-0xa1eda) | the accepted mission's | 1 (@0xa1e98) |
//!
//! **The green system** is −1 unless the mission's `Flags` has
//! [`MARKS_DESTINATION`] (@0xa307c, @0xa3d0d, @0x9e40c, @0xa1ea8). With
//! it, it is the owner (+0x14) of the travel stellar, or of the return
//! stellar when there is no travel stellar, as it stands (@0xa30a7,
//! @0xa30e3, @0xa3d46, @0x9e434, @0xa1ed5): ours' holder
//! ([`OverlaidGalaxy::holder`](crate::story::OverlaidGalaxy::holder)),
//! with no active test, unlike the red arrows'
//! ([`Session::map_mission_systems`]). A stellar with no owner gives no
//! green system. An offer's stellars are its
//! [`destination`](crate::mission::offer::Offer::destination) and
//! [`return_to`](crate::mission::offer::Offer::return_to) (`_briefMissionInfo`
//! +0 and +8), a held mission's its own (+0 and +4). There is no
//! "reached" test: mission info and the briefing mark the travel
//! stellar's system after the destination is reached, while the red
//! arrow has moved to the return.
//!
//! **The arrow flag** keeps the green system's red arrow for the offer,
//! the computer and the briefing, and clears it for mission info.
//!
//! None of this plays a sound or makes a random draw. The word width of
//! `_greenMissionSyst`, compared raw, is not modelled apart.

use super::Session;
use crate::catalog::{MissionId, StellarId, SystemId};
use crate::galaxy_map::{MARKS_DESTINATION, MapMarks};

impl Session {
    /// The marks of the map an offer of `mission` opens (see the module
    /// docs): the offer on the docked visit, or in flight the offer from a
    /// ship the offer dialog shows; multiple arrows allowed. A mission not
    /// offered gives [`MapMarks::NONE`].
    #[must_use]
    pub fn offer_map_marks(&self, mission: MissionId) -> MapMarks {
        let green = self.offer_of(mission).and_then(|offer| {
            self.green_system(
                mission,
                offer.destination.map(|stop| stop.stellar),
                offer.return_to.map(|stop| stop.stellar),
            )
        });
        MapMarks {
            green,
            multi_arrows: true,
        }
    }

    /// The marks of the map the mission computer opens with `selected`
    /// the offer of its selected row, if any (see the module docs): none
    /// with no row selected, otherwise the offer's
    /// ([`Session::offer_map_marks`]).
    #[must_use]
    pub fn computer_map_marks(&self, selected: Option<MissionId>) -> MapMarks {
        selected.map_or(MapMarks::NONE, |mission| self.offer_map_marks(mission))
    }

    /// The marks of the map mission info opens with the mission held in
    /// `slot` selected, if any (see the module docs): its green system,
    /// and the green system's red arrow cleared whether or not there is
    /// one.
    #[must_use]
    pub fn mission_info_map_marks(&self, slot: Option<u8>) -> MapMarks {
        MapMarks {
            green: slot.and_then(|slot| self.held_green(slot)),
            multi_arrows: false,
        }
    }

    /// The marks of the map the briefing of the mission just accepted
    /// into `slot` opens (see the module docs), multiple arrows allowed:
    /// worked out as its text is queued, as `_DoMissionAccept` writes
    /// them just before `_ShowTextDialog` (@0xa1e8d-0xa1f0e).
    pub(super) fn briefing_map_marks(&self, slot: u8) -> MapMarks {
        MapMarks {
            green: self.held_green(slot),
            multi_arrows: true,
        }
    }

    /// The green system of the mission held in `slot`, if any.
    fn held_green(&self, slot: u8) -> Option<SystemId> {
        let held = self.pilot.missions.get(slot)?;
        self.green_system(
            held.mission,
            held.destination.map(|stop| stop.stellar),
            held.return_to.map(|stop| stop.stellar),
        )
    }

    /// The green system of `mission`'s map, its travel stellar
    /// `destination` and its return stellar `return_to` (see the module
    /// docs).
    fn green_system(
        &self,
        mission: MissionId,
        destination: Option<StellarId>,
        return_to: Option<StellarId>,
    ) -> Option<SystemId> {
        let flags = self
            .statics
            .mission_record(mission)
            .map_or(0, |record| record.flags);
        if flags & MARKS_DESTINATION == 0 {
            return None;
        }
        let stellar = destination.or(return_to)?;
        self.galaxy
            .holder(self.statics.listers(stellar), self.pilot.system)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::MissionRecord;
    use crate::catalog::MissionTexts;
    use crate::chance::NeverFires;
    use crate::galaxy_map::DrawnSystem;
    use crate::mission::Venue;
    use crate::mission::active::ActiveMission;
    use crate::mission::offer::{Offer, Stop};
    use crate::session::MissionTextKind;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission, planet};

    /// [`catalog`] with system 132 holding stellar 150, and missions at
    /// the computer: 300 to 140 (in 131) and back at 150 (in 132), of
    /// `Flags` 0x0100; 301 to 140 without it; 302 back at 150 alone and
    /// 303 going nowhere, both with it.
    fn marking() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.systems.push(SystemId(132));
        catalog
            .sites
            .push((SystemId(132), vec![planet(150, 0.0, 0.0)]));
        let record = |id: i16, flags: u16, travel_stel: i16, return_stel: i16| MissionRecord {
            flags,
            travel_stel,
            return_stel,
            ..mission(id)
        };
        catalog.missions = vec![
            record(300, MARKS_DESTINATION, 140, 150),
            record(301, 0, 140, -1),
            record(302, MARKS_DESTINATION, -1, 150),
            record(303, MARKS_DESTINATION, -1, -1),
        ];
        catalog
    }

    /// A session of `catalog` landed on 128, its offers rolled.
    fn landed(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session).map(|stellar| stellar.0), Ok(128));
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    fn green(system: i16, multi_arrows: bool) -> MapMarks {
        MapMarks {
            green: Some(SystemId(system)),
            multi_arrows,
        }
    }

    fn stop(stellar: i16) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: None,
        }
    }

    /// `session` holding `mission` to `to`, back at `back`: its slot.
    fn holding(session: &mut Session, mission: i16, to: Option<i16>, back: Option<i16>) -> u8 {
        let offer = Offer {
            mission: MissionId(mission),
            venue: Venue::Computer,
            destination: to.map(stop),
            return_to: back.map(stop),
            cargo_type: None,
            cargo_tons: 0,
            pay: 0,
            deadline: None,
        };
        let record = session
            .statics
            .mission_record(MissionId(mission))
            .cloned()
            .expect("a record");
        let accepted = ActiveMission::accepted(&record, &offer, session.pilot.date);
        session.pilot.missions.fill(accepted).expect("a slot")
    }

    // The offer.

    #[test]
    fn an_offer_of_flag_0x0100_marks_its_destinations_system_green() {
        let session = landed(&marking());
        assert_eq!(
            session.offer_map_marks(MissionId(300)),
            green(131, true),
            "140's, not the return's 150"
        );
    }

    #[test]
    fn an_offer_without_flag_0x0100_marks_none() {
        let session = landed(&marking());
        assert_eq!(session.offer_map_marks(MissionId(301)), MapMarks::NONE);
    }

    #[test]
    fn an_offer_with_no_destination_marks_its_returns_system() {
        let session = landed(&marking());
        assert_eq!(session.offer_map_marks(MissionId(302)), green(132, true));
    }

    #[test]
    fn an_offer_going_nowhere_marks_none() {
        let session = landed(&marking());
        assert_eq!(session.offer_map_marks(MissionId(303)), MapMarks::NONE);
    }

    #[test]
    fn a_mission_not_offered_marks_none() {
        let session = landed(&marking());
        assert_eq!(session.offer_map_marks(MissionId(999)), MapMarks::NONE);
        let session = Session::start(&marking()).expect("starts");
        assert_eq!(
            session.offer_map_marks(MissionId(300)),
            MapMarks::NONE,
            "in flight, with no ship's offer shown"
        );
    }

    // The computer.

    #[test]
    fn the_computer_marks_its_selected_offers_system() {
        let session = landed(&marking());
        assert_eq!(
            session.computer_map_marks(Some(MissionId(300))),
            green(131, true)
        );
        assert_eq!(
            session.computer_map_marks(Some(MissionId(301))),
            MapMarks::NONE
        );
    }

    #[test]
    fn the_computer_with_no_row_selected_marks_none_with_multiple_arrows() {
        let session = landed(&marking());
        assert_eq!(session.computer_map_marks(None), MapMarks::NONE);
    }

    // Mission info.

    #[test]
    fn mission_info_marks_the_selected_missions_destination_and_clears_its_red_arrow() {
        let mut session = Session::start(&marking()).expect("starts");
        let slot = holding(&mut session, 300, Some(140), Some(150));
        assert_eq!(
            session.mission_info_map_marks(Some(slot)),
            green(131, false)
        );
        session.pilot.missions.get_mut(slot).expect("held").reached = true;
        assert_eq!(
            session.mission_info_map_marks(Some(slot)),
            green(131, false),
            "no reached test"
        );
        let missions = session.map_mission_systems();
        assert_eq!(missions, [SystemId(132)], "the red arrow at the return");
        let drawing =
            session.map_drawing(None, &missions, session.mission_info_map_marks(Some(slot)));
        let arrows = |id: i16| {
            drawing
                .systems
                .iter()
                .find(|system| system.id == SystemId(id))
                .map(|system| (system.red_arrow, system.green_arrow))
        };
        assert_eq!(arrows(131), Some((false, true)), "green at the destination");
        assert_eq!(arrows(132), Some((true, false)), "red at the return");
    }

    #[test]
    fn mission_info_with_no_selection_marks_no_green_and_clears_red_arrows() {
        let mut session = Session::start(&marking()).expect("starts");
        holding(&mut session, 300, Some(140), None);
        let none = MapMarks {
            green: None,
            multi_arrows: false,
        };
        assert_eq!(session.mission_info_map_marks(None), none);
        assert_eq!(
            session.mission_info_map_marks(Some(9)),
            none,
            "an empty slot"
        );
        let slot = holding(&mut session, 301, Some(140), None);
        assert_eq!(
            session.mission_info_map_marks(Some(slot)),
            none,
            "without 0x0100"
        );
    }

    #[test]
    fn a_destination_whose_holder_is_hidden_is_still_marked() {
        // The player in 131, hidden, which lists 140: 131 holds it by the
        // rescue, but is not active, so no red arrow marks it.
        let catalog = marking();
        let mut session = Session::start(&catalog).expect("starts");
        session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        });
        session.pilot.system = SystemId(131);
        let slot = holding(&mut session, 300, Some(140), None);
        assert_eq!(session.map_mission_systems(), [], "no active holder");
        assert_eq!(
            session.mission_info_map_marks(Some(slot)),
            green(131, false)
        );
    }

    #[test]
    fn a_stellar_with_no_holder_marks_none() {
        let mut session = Session::start(&marking()).expect("starts");
        let slot = holding(&mut session, 300, Some(999), Some(150));
        assert_eq!(
            session.mission_info_map_marks(Some(slot)),
            MapMarks {
                green: None,
                multi_arrows: false,
            },
            "999 has no holder, and the return is read only with no travel stellar"
        );
    }

    // The briefing.

    #[test]
    fn the_briefings_marks_are_the_held_missions_with_multiple_arrows() {
        let mut session = Session::start(&marking()).expect("starts");
        let slot = holding(&mut session, 302, None, Some(150));
        assert_eq!(session.briefing_map_marks(slot), green(132, true));
        let slot = holding(&mut session, 301, Some(140), None);
        assert_eq!(session.briefing_map_marks(slot), MapMarks::NONE);
    }

    #[test]
    fn the_accepts_briefing_text_carries_the_accepted_missions_marks_and_no_other_text() {
        let mut catalog = marking();
        catalog.mission_texts = vec![MissionTexts {
            brief: Some("Off you go.".to_owned()),
            refuse: Some("Pity.".to_owned()),
            ..MissionTexts::named(MissionId(300), "Run")
        }];
        let mut session = landed(&catalog);
        session
            .accept_mission(MissionId(300), &mut NeverFires)
            .expect("accepted");
        let texts = session.take_texts_only();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].kind, MissionTextKind::Brief);
        assert_eq!(texts[0].map, Some(green(131, true)));
        let mut session = landed(&catalog);
        let _ = session.refuse_mission(MissionId(300), &mut NeverFires);
        let texts = session.take_texts_only();
        assert_eq!(texts[0].kind, MissionTextKind::Refuse);
        assert_eq!(texts[0].map, None, "no map from a refusal's text");
    }

    // The drawing.

    #[test]
    fn an_offers_marks_put_the_green_arrow_beside_a_held_missions_red_one() {
        let mut session = landed(&marking());
        holding(&mut session, 301, Some(140), None);
        let missions = session.map_mission_systems();
        assert_eq!(missions, [SystemId(131)]);
        let marks = session.offer_map_marks(MissionId(300));
        let drawing = session.map_drawing(None, &missions, marks);
        let system: Option<DrawnSystem> = drawing
            .systems
            .iter()
            .find(|system| system.id == SystemId(131))
            .copied();
        assert!(system.is_some_and(|system| system.red_arrow && system.green_arrow));
    }
}
