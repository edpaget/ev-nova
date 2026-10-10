//! The session's side of mission cargo (see the
//! [`mission`](crate::mission) module's "Cargo"): loading it as a
//! mission is accepted.

use super::Session;
use super::mission_screens::MissionTextKind;
use crate::catalog::MissionRecord;
use crate::mission::cargo::loads_at_accept;

impl Session {
    /// Loads the cargo of `record`, just accepted into `slot`, as the
    /// accept does after `BriefText` (`_DoMissionAccept`
    /// @0xa1f28-0xa1fbf): with `PickupMode` 0 it is aboard, whatever its
    /// type, and its `LoadCargText` is shown, filled for the slot.
    /// Nothing when the slot no longer holds the mission.
    pub(super) fn load_at_accept(&mut self, slot: u8, record: &MissionRecord) {
        if !loads_at_accept(record) {
            return;
        }
        let Some(active) = self.slot_holding(slot, record.id) else {
            return;
        };
        active.aboard = true;
        self.show_mission_text(MissionTextKind::LoadCargo, record.id, Some(slot));
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{CommCatalog, MissionId, MissionTexts, PlaceNames, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::control::Script;
    use crate::session::MissionText;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission};

    /// `STR#` 4001, the cargo names.
    #[derive(Debug)]
    struct Cargoes;

    impl CommCatalog for Cargoes {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                4001 => ["food", "industrial", "medical"]
                    .map(str::to_owned)
                    .to_vec(),
                _ => Vec::new(),
            }
        }
    }

    /// [`catalog`] with 131 linked back to 130, stellar 129 named Luna,
    /// `missions` and `texts`.
    fn cargo_catalog(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.place_names = PlaceNames {
            stellars: [(StellarId(129), "Luna".to_owned())].into(),
            systems: [(SystemId(130), "Sol".to_owned())].into(),
        };
        catalog.missions = missions;
        catalog.mission_texts = texts;
        catalog
    }

    /// A session of `catalog`, told [`Cargoes`], landed on 128 and
    /// settled, every roll 1, its texts taken.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Cargoes));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        let _ = session.take_mission_texts();
        session
    }

    /// Mission `id`, offered at 128, to 129, carrying 5 tons of medical
    /// goods, loaded as `pickup_mode` says and unloaded as
    /// `drop_off_mode` says.
    fn courier(id: i16, pickup_mode: i16, drop_off_mode: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            cargo_type: 2,
            cargo_qty: 5,
            pickup_mode,
            drop_off_mode,
            ..mission(id)
        }
    }

    /// Mission `id`'s texts: briefed "Brief.", with these load and dump
    /// texts.
    fn texts(id: i16, load: Option<&str>, dump: Option<&str>) -> MissionTexts {
        MissionTexts {
            brief: Some("Brief.".to_owned()),
            load_cargo: load.map(str::to_owned),
            dump_cargo: dump.map(str::to_owned),
            ..MissionTexts::named(MissionId(id), "Courier")
        }
    }

    fn text(kind: MissionTextKind, mission: i16, text: &str) -> MissionText {
        MissionText {
            kind,
            mission: MissionId(mission),
            text: text.to_owned(),
        }
    }

    const LOAD: &str = "Loaded <CQ> tons of <CT> for <DST>.";

    fn aboard(session: &Session, slot: u8) -> bool {
        session.pilot().missions().get(slot).expect("held").aboard
    }

    #[test]
    fn accepting_a_pickup_mode_0_mission_loads_its_cargo_after_its_brief() {
        let catalog = cargo_catalog(vec![courier(140, 0, 0)], vec![texts(140, Some(LOAD), None)]);
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert!(aboard(&session, slot));
        assert_eq!(
            session.take_mission_texts(),
            [
                text(MissionTextKind::Brief, 140, "Brief."),
                text(
                    MissionTextKind::LoadCargo,
                    140,
                    "Loaded 5 tons of medical for Luna."
                ),
            ],
            "filled for the slot"
        );
        assert_eq!(session.pilot().total_cargo(), 5);
    }

    #[test]
    fn a_pickup_mode_0_mission_with_no_cargo_type_is_aboard_too() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                cargo_type: -1,
                ..courier(140, 0, 0)
            }],
            vec![texts(140, None, None)],
        );
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert!(aboard(&session, slot));
        assert_eq!(
            session.take_mission_texts(),
            [text(MissionTextKind::Brief, 140, "Brief.")],
            "no LoadCargText"
        );
    }

    #[test]
    fn the_other_pickup_modes_load_nothing_at_accept() {
        for mode in [1, 2, -1] {
            let catalog = cargo_catalog(
                vec![courier(140, mode, 0)],
                vec![texts(140, Some(LOAD), None)],
            );
            let mut session = docked(&catalog);
            let slot = session
                .accept_mission(MissionId(140), &mut NeverFires)
                .expect("accepted");
            assert!(!aboard(&session, slot), "{mode}");
            assert_eq!(
                session.take_mission_texts(),
                [text(MissionTextKind::Brief, 140, "Brief.")],
                "{mode}"
            );
        }
    }

    #[test]
    fn an_on_accept_that_aborts_its_own_mission_loads_nothing() {
        let catalog = cargo_catalog(
            vec![MissionRecord {
                on_accept: Script::parse("A140"),
                ..courier(140, 0, 0)
            }],
            vec![texts(140, Some(LOAD), None)],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.pilot().total_cargo(), 0);
        assert!(
            session
                .take_mission_texts()
                .iter()
                .all(|shown| shown.kind != MissionTextKind::LoadCargo)
        );
    }
}
