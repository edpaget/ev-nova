//! The pilot desk over a flight: a thin mapping from the [`PilotDesk`]
//! port to the flight's own desk ([`DevDesk`]: one raw reading, one edit
//! entry), with the names of systems and stellars from the galaxy the
//! course map already read.
//!
//! The flight decides each [`PilotEdit`] and its refusal; the desk only
//! names what the flight reads.

use nova_sim::catalog::{StellarId, SystemId};

use super::pilot::{DevDesk, EditRefusal, PilotDesk, PilotEdit, PilotSheet, Place};
use crate::galaxy::GalaxyModel;

/// The flight's desk `desk`, as a [`PilotDesk`], naming the places from
/// `galaxy`.
pub struct FlightDesk<'a, D: ?Sized> {
    desk: &'a mut D,
    galaxy: &'a GalaxyModel,
}

impl<'a, D: DevDesk + ?Sized> FlightDesk<'a, D> {
    /// The desk over `desk`, naming the places from `galaxy`.
    pub fn new(desk: &'a mut D, galaxy: &'a GalaxyModel) -> Self {
        Self { desk, galaxy }
    }
}

impl<D: ?Sized> FlightDesk<'_, D> {
    /// System `id`, named by the galaxy, or by its ID when it has no entry.
    fn system_place(&self, id: SystemId) -> Place<SystemId> {
        let name = self
            .galaxy
            .system(id)
            .map(|system| system.entry.name.clone());
        Place {
            id,
            name: name.unwrap_or_else(|| id.0.to_string()),
        }
    }

    /// Stellar `id` of system `system`, named as [`Self::system_place`]
    /// names a system.
    fn stellar_place(&self, system: SystemId, id: StellarId) -> Place<StellarId> {
        let name = self
            .galaxy_stellars(system)
            .into_iter()
            .find(|stellar| stellar.id == id)
            .map(|stellar| stellar.name);
        Place {
            id,
            name: name.unwrap_or_else(|| id.0.to_string()),
        }
    }

    /// The stellars the galaxy lists for `system`.
    fn galaxy_stellars(&self, system: SystemId) -> Vec<Place<StellarId>> {
        self.galaxy
            .system(system)
            .map(|system| {
                system
                    .entry
                    .stellars
                    .iter()
                    .map(|stellar| Place {
                        id: stellar.id,
                        name: stellar.name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The sheet is the flight's reading ([`DevDesk::reading`]), its places
/// named; an edit goes to the flight ([`DevDesk::edit`]).
impl<D: DevDesk + ?Sized> PilotDesk for FlightDesk<'_, D> {
    fn sheet(&self) -> Option<PilotSheet> {
        let reading = self.desk.reading()?;
        Some(PilotSheet {
            name: reading.name,
            credits: reading.credits,
            shield: reading.reserves.shield,
            armor: reading.reserves.armor,
            fuel: reading.reserves.fuel,
            date: reading.date,
            date_text: reading.date_text,
            system: self.system_place(reading.system),
            landed: reading
                .landed
                .map(|stellar| self.stellar_place(reading.system, stellar)),
            bits: reading.bits,
        })
    }

    fn systems(&self) -> Vec<Place<SystemId>> {
        self.galaxy
            .systems()
            .iter()
            .map(|system| Place {
                id: system.entry.id,
                name: system.entry.name.clone(),
            })
            .collect()
    }

    fn stellars(&self, system: SystemId) -> Vec<Place<StellarId>> {
        self.galaxy_stellars(system)
    }

    fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal> {
        self.desk.edit(edit)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::{Bit, DeskReading, GameDate, Gauge, Reserve, Reserves};

    use super::*;
    use crate::flight::sim_port::fixture::FakeFlight;
    use crate::galaxy::catalog::{Galaxy, StellarEntry, SystemEntry};

    fn system(id: i16, name: &str, stellars: &[(i16, &str)]) -> SystemEntry {
        SystemEntry {
            id: SystemId(id),
            name: name.to_owned(),
            x: 0,
            y: 0,
            links: Vec::new(),
            govt: None,
            stellars: stellars
                .iter()
                .map(|&(id, name)| StellarEntry {
                    id: StellarId(id),
                    name: name.to_owned(),
                })
                .collect(),
        }
    }

    /// Sol (130) holds Earth (128), and Alpha Centauri (131) holds Proxima
    /// (140) and a hypergate (141).
    fn galaxy() -> GalaxyModel {
        GalaxyModel::new(Galaxy {
            systems: vec![
                system(131, "Alpha Centauri", &[(140, "Proxima"), (141, "Gate")]),
                system(130, "Sol", &[(128, "Earth")]),
            ],
            ..Galaxy::default()
        })
    }

    fn place<Id>(id: Id, name: &str) -> Place<Id> {
        Place {
            id,
            name: name.to_owned(),
        }
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// Ada, landed on Earth (128) in Sol (130) on 23 June 1177, the fuel
    /// at 120 of 300, bits 7 and 42 set.
    fn landed() -> DeskReading {
        DeskReading {
            name: "Ada".to_owned(),
            credits: 1000,
            reserves: Reserves {
                shield: Gauge::full(100.0),
                armor: Gauge::full(50.0),
                fuel: Gauge {
                    now: 120.0,
                    max: 300.0,
                },
            },
            date: GameDate::new(1177, 6, 23).expect("a date"),
            date_text: "June 23, 1177 NC".to_owned(),
            system: SystemId(130),
            landed: Some(StellarId(128)),
            bits: vec![bit(7), bit(42)],
        }
    }

    /// A fake flight whose desk reads `reading`.
    fn reading(reading: DeskReading) -> FakeFlight {
        let mut flight = FakeFlight::new();
        flight.desk_mut().reading = Some(reading);
        flight
    }

    #[test]
    fn the_sheet_is_the_flights_reading_with_its_places_named() {
        let mut flight = reading(landed());
        let galaxy = galaxy();
        let desk = FlightDesk::new(&mut flight, &galaxy);
        assert_eq!(
            desk.sheet(),
            Some(PilotSheet {
                name: "Ada".to_owned(),
                credits: 1000,
                shield: Gauge::full(100.0),
                armor: Gauge::full(50.0),
                fuel: Gauge {
                    now: 120.0,
                    max: 300.0
                },
                date: GameDate::new(1177, 6, 23).expect("a date"),
                date_text: "June 23, 1177 NC".to_owned(),
                system: place(SystemId(130), "Sol"),
                landed: Some(place(StellarId(128), "Earth")),
                bits: vec![bit(7), bit(42)],
            })
        );
        assert_eq!(
            desk.sheet().expect("a pilot").gauge(Reserve::Fuel).now,
            120.0
        );
    }

    #[test]
    fn in_flight_the_sheet_has_no_stellar_and_a_place_unnamed_is_its_id() {
        let galaxy = GalaxyModel::new(Galaxy::default());
        let mut flying = reading(DeskReading {
            landed: None,
            ..landed()
        });
        let sheet = FlightDesk::new(&mut flying, &galaxy)
            .sheet()
            .expect("a pilot");
        assert_eq!(sheet.landed, None);
        assert_eq!(sheet.system, place(SystemId(130), "130"));
        let mut docked = reading(landed());
        assert_eq!(
            FlightDesk::new(&mut docked, &galaxy)
                .sheet()
                .expect("a pilot")
                .landed,
            Some(place(StellarId(128), "128"))
        );
    }

    #[test]
    fn a_flight_with_no_reading_has_no_sheet() {
        let mut flight = FakeFlight::new();
        let galaxy = galaxy();
        assert_eq!(FlightDesk::new(&mut flight, &galaxy).sheet(), None);
    }

    #[test]
    fn the_places_are_the_galaxys() {
        let mut flight = FakeFlight::new();
        let galaxy = galaxy();
        let desk = FlightDesk::new(&mut flight, &galaxy);
        assert_eq!(
            desk.systems(),
            [
                place(SystemId(130), "Sol"),
                place(SystemId(131), "Alpha Centauri")
            ]
        );
        assert_eq!(
            desk.stellars(SystemId(131)),
            [
                place(StellarId(140), "Proxima"),
                place(StellarId(141), "Gate")
            ]
        );
        assert_eq!(desk.stellars(SystemId(999)), []);
    }

    #[test]
    fn each_edit_goes_to_the_flight_and_its_answer_comes_back() {
        let mut flight = reading(landed());
        let galaxy = galaxy();
        let date = GameDate::new(1180, 1, 2).expect("a date");
        let edits = [
            PilotEdit::Credits(77),
            PilotEdit::Reserve(Reserve::Shield, 1000.0),
            PilotEdit::Date(date),
            PilotEdit::Bit(bit(9), true),
            PilotEdit::MoveTo {
                system: SystemId(131),
                stellar: StellarId(140),
            },
        ];
        let mut desk = FlightDesk::new(&mut flight, &galaxy);
        for edit in edits {
            assert_eq!(desk.edit(edit), Ok(()), "{edit:?}");
        }
        assert_eq!(flight.edits(), edits);
    }

    #[test]
    fn a_refused_edit_is_refused_as_the_flight_says() {
        let mut flight = reading(landed());
        flight.desk_mut().refusal = Some(EditRefusal::NotLandable);
        let galaxy = galaxy();
        let to = PilotEdit::MoveTo {
            system: SystemId(131),
            stellar: StellarId(141),
        };
        assert_eq!(
            FlightDesk::new(&mut flight, &galaxy).edit(to),
            Err(EditRefusal::NotLandable)
        );
        assert_eq!(flight.edits(), [to]);
    }
}
