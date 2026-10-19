//! What the main screen's pilot panel asks of the session
//! ([`Session::pilot_panel`], the session's side of
//! [`pilot_panel`](crate::pilot_panel); see that module for the
//! original's rules).

use super::Session;
use crate::catalog::PilotCatalog;
use crate::pilot_panel::PilotPanel;
use crate::rating::{RATING_STRINGS, string_index};

impl Session {
    /// What the main screen's pilot panel says about the pilot (see
    /// [`pilot_panel`](crate::pilot_panel)), the stellars the legal status
    /// reads taken from `catalog`. Reading it changes nothing.
    #[must_use]
    pub fn pilot_panel(&self, catalog: &impl PilotCatalog) -> PilotPanel {
        let words = self.setup.player_info_words();
        let rating = string_index(self.pilot.combat_rating());
        PilotPanel {
            pilot_name: self.pilot.name().to_owned(),
            ship_name: self.pilot.ship_name().unwrap_or_default().to_owned(),
            ship_class: self
                .ship_name(self.pilot.ship)
                .unwrap_or_default()
                .to_owned(),
            ship: self.pilot.ship,
            legal_status: self.legal_status(catalog, &words),
            combat_rating: self
                .setup
                .nth_string(RATING_STRINGS, rating + 1)
                .unwrap_or_default(),
            date: self.med_date_text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{CommCatalog, LandingSite, ShipId, SystemId};
    use crate::landing::StellarFlags;
    use crate::news::MISC_STRINGS;
    use crate::player_info::LEGAL_STATUS_STRINGS;
    use crate::session::tests::alternates;
    use crate::testkit::{FAST, FakePilotCatalog, planet, ship, starting};

    /// `STR#` 2002 with #396 "N/A"; `STR#` 134, each "status n"; `STR#`
    /// 138, each "rating n" for its entry n, but for the last, missing.
    #[derive(Debug)]
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                MISC_STRINGS => {
                    let mut strings = vec![String::new(); 400];
                    strings[395] = "N/A".to_owned();
                    strings
                }
                LEGAL_STATUS_STRINGS => (1..=18).map(|n| format!("status {n}")).collect(),
                RATING_STRINGS => (1..=10).map(|n| format!("rating {n}")).collect(),
                _ => Vec::new(),
            }
        }
    }

    /// [`alternates`] starting in 131, flying a recorded ship 128.
    fn flown() -> FakePilotCatalog {
        FakePilotCatalog {
            character: starting([Some(131), None, None, None]).character,
            ship_records: vec![ship(128, FAST)],
            ..alternates()
        }
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists))
    }

    #[test]
    fn the_panel_names_the_pilot_its_ship_and_its_class() {
        let catalog = flown();
        let mut session = session(&catalog);
        session.pilot.name = "Kenny".to_owned();
        session.pilot.ship_name = Some("Bitsy".to_owned());
        let panel = session.pilot_panel(&catalog);
        assert_eq!(panel.pilot_name, "Kenny");
        assert_eq!(panel.ship_name, "Bitsy");
        assert_eq!(panel.ship_class, "Ship 128");
        assert_eq!(
            Some(panel.ship_class.as_str()),
            session.ship_name(session.pilot.ship),
            "the player info's Ship Class"
        );
        assert_eq!(panel.ship, ShipId(128));
        assert_eq!(panel.date, session.med_date_text());
        session.pilot.ship_name = None;
        assert_eq!(session.pilot_panel(&catalog).ship_name, "", "no name");
    }

    #[test]
    fn the_legal_status_is_the_player_infos() {
        let catalog = flown();
        let mut session = session(&catalog);
        session.pilot.set_system_record(SystemId(131), 50);
        let panel = session.pilot_panel(&catalog);
        assert_eq!(
            panel.legal_status,
            session.player_info(&catalog).legal_status
        );
        assert_eq!(
            panel.legal_status,
            session.legal_status_in(&catalog, SystemId(131), &session.setup.player_info_words())
        );
        assert!(panel.legal_status.starts_with("status "), "{panel:?}");
    }

    #[test]
    fn the_legal_status_reads_n_a_in_an_uninhabited_system() {
        let mut catalog = flown();
        catalog.sites[1] = (
            SystemId(131),
            vec![LandingSite {
                flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                ..planet(140, 0.0, 0.0)
            }],
        );
        let session = session(&catalog);
        assert_eq!(session.pilot_panel(&catalog).legal_status, "N/A");
    }

    #[test]
    fn the_combat_rating_reads_str_138_by_its_index() {
        let catalog = flown();
        let mut session = session(&catalog);
        let mut rating_at = |rating: i64| {
            session.pilot.combat_rating = rating;
            session.pilot_panel(&catalog).combat_rating
        };
        assert_eq!(rating_at(0), "rating 1");
        assert_eq!(rating_at(150), "rating 3");
        assert_eq!(rating_at(25_599), "rating 10");
        assert_eq!(rating_at(25_600), "", "#11 is missing");
    }

    #[test]
    fn reading_the_panel_changes_nothing() {
        let catalog = flown();
        let session = session(&catalog);
        let before = session.pilot().clone();
        let first = session.pilot_panel(&catalog);
        assert_eq!(session.pilot(), &before);
        assert_eq!(session.pilot_panel(&catalog), first);
    }
}
