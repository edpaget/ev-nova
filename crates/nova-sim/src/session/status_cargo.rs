//! The session's side of [`status_cargo`](crate::status_cargo): the facts
//! the status bar's cargo area is decided from, gathered from the pilot,
//! the fleet's holds, the `jünk` records and the string lists.

use super::{PlayerShip, Session, Setup, Statics};
use crate::market::Good;
use crate::pilot::Pilot;
use crate::status_cargo::{
    self, CARGO_ABBREVIATIONS, COMMODITY_ROWS, CargoFacts, MissionCargo, STATUS_CARGO_STRINGS,
    StatusCargo,
};

impl Session {
    /// What the status bar's cargo area lists (see
    /// [`status_cargo::status_cargo`]).
    #[must_use]
    pub fn status_cargo(&self) -> StatusCargo {
        self.ship
            .status_cargo(&self.pilot, &self.statics, &self.setup)
    }
}

impl PlayerShip {
    /// What the status bar's cargo area lists (see
    /// [`status_cargo::status_cargo`]).
    #[must_use]
    fn status_cargo(&self, pilot: &Pilot, statics: &Statics, setup: &Setup) -> StatusCargo {
        let mut commodity_tons = [0; COMMODITY_ROWS];
        let mut junk = Vec::new();
        for (good, tons) in pilot.cargo() {
            match good {
                Good::Commodity(n) => {
                    if let Some(held) = commodity_tons.get_mut(usize::from(n)) {
                        *held = i64::from(tons);
                    }
                }
                Good::Junk(id) => junk.push((
                    statics.goods.abbrev(id).unwrap_or_default().to_owned(),
                    i64::from(tons),
                )),
            }
        }
        let strings = setup.strings();
        status_cargo::status_cargo(&CargoFacts {
            commodity_tons,
            commodity_labels: strings.string_list(STATUS_CARGO_STRINGS),
            fleet_holds: i64::from(self.capacity(pilot, statics)),
            total_cargo: pilot.total_cargo(),
            missions: pilot
                .missions()
                .iter()
                .map(|(_, active)| MissionCargo {
                    aboard: active.aboard,
                    cargo_type: active.cargo_type,
                })
                .collect(),
            cargo_names: strings.string_list(CARGO_ABBREVIATIONS),
            junk,
            cash: pilot.cash(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{
        CommCatalog, JunkId, JunkRecord, MissionId, MissionRecord, MissionTexts, StellarId,
    };
    use crate::chance::NeverFires;
    use crate::control::Test;
    use crate::status_cargo::{CommodityRow, Special};
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission};

    /// `STR#` 4003, the rows' labels, and 4002, the cargo names.
    #[derive(Debug)]
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            let strings: &[&str] = match id {
                4003 => &["Food:", "Ind:", "Med:", "LuxG:", "Met:", "Equ:"],
                4002 => &[
                    "Food",
                    "IndGds",
                    "MedGds",
                    "LuxGds",
                    "Metal",
                    "Equpt",
                    "Passengers",
                ],
                _ => &[],
            };
            strings.iter().map(|&s| s.to_owned()).collect()
        }
    }

    /// [`catalog`] with mission 140, offered at 128, carrying 5 tons of
    /// passengers (type 6) aboard from its accept, and `jünk` 146,
    /// opals, abbreviated "Opl".
    fn cargo_catalog() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.missions = vec![MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            cargo_type: 6,
            cargo_qty: 5,
            pickup_mode: 0,
            ..mission(140)
        }];
        catalog.mission_texts = vec![MissionTexts::named(MissionId(140), "Ferry")];
        catalog.junk = vec![JunkRecord {
            id: JunkId(146),
            name: "Opals".to_owned(),
            abbrev: "Opl".to_owned(),
            base_price: 100,
            sold_at: Vec::new(),
            bought_at: Vec::new(),
            buy_on: Test::default(),
            sell_on: Test::default(),
            flags: 0,
            scan_mask: 0,
            lc_name: String::new(),
        }];
        catalog
    }

    #[test]
    fn the_cargo_area_reads_the_hold_the_missions_the_junk_and_the_cash() {
        let catalog = cargo_catalog();
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session.pilot.cargo.insert(Good::Commodity(1), 7);
        session.pilot.cargo.insert(Good::Junk(JunkId(146)), 2);
        session.pilot.cash = 12_345;

        let cargo = session.status_cargo();
        assert_eq!(
            cargo.rows[1],
            Some(CommodityRow {
                label: "Ind:".to_owned(),
                tons: 7
            })
        );
        assert_eq!(cargo.rows.iter().flatten().count(), 1, "{cargo:?}");
        let capacity = i64::from(session.capacity());
        assert!(capacity > 9, "room for the cargo: {capacity}");
        assert_eq!(cargo.free, capacity - 9);
        assert_eq!(cargo.special, Some(Special::Named("Opl".to_owned())));
        assert_eq!(cargo.cash, 12_345);

        session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        let cargo = session.status_cargo();
        assert_eq!(
            cargo.special,
            Some(Special::Named("Passengers".to_owned())),
            "one mission's cargo wins over one jünk"
        );
        assert_eq!(cargo.free, capacity - 14, "the mission's cargo counts");
    }
}
