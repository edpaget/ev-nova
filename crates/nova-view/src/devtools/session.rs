//! The pilot desk over a live session: a thin mapping from the
//! [`PilotDesk`] port to the [`Session`]'s reads and edits, with the names
//! of systems and stellars from the galaxy the course map already read.
//!
//! Each [`PilotEdit`] goes to the one `Session` method that makes it (see
//! `nova_sim`'s session edits); the session decides, and the adapter only
//! maps its refusal.

use nova_sim::catalog::{PilotCatalog, StellarId, SystemId};
use nova_sim::{RelocateRefusal, Session};

use super::pilot::{EditRefusal, PilotDesk, PilotEdit, PilotSheet, Place};
use crate::galaxy::GalaxyModel;

/// The session flying, as a [`PilotDesk`], with the catalog a move reads
/// and the galaxy that names the places.
pub struct SessionDesk<'a, C: ?Sized> {
    session: &'a mut Session,
    catalog: &'a C,
    galaxy: &'a GalaxyModel,
}

impl<'a, C: PilotCatalog + ?Sized> SessionDesk<'a, C> {
    /// The desk over `session`, moving through `catalog` and naming the
    /// places from `galaxy`.
    pub fn new(session: &'a mut Session, catalog: &'a C, galaxy: &'a GalaxyModel) -> Self {
        Self {
            session,
            catalog,
            galaxy,
        }
    }
}

impl<C: ?Sized> SessionDesk<'_, C> {
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

/// The desk's refusal for the session's.
fn refusal(refusal: RelocateRefusal) -> EditRefusal {
    match refusal {
        RelocateRefusal::InFlight => EditRefusal::InFlight,
        RelocateRefusal::NoSystem => EditRefusal::NoSystem,
        RelocateRefusal::NoStellar => EditRefusal::NoStellar,
        RelocateRefusal::NotLandable => EditRefusal::NotLandable,
    }
}

impl<C: PilotCatalog + ?Sized> PilotDesk for SessionDesk<'_, C> {
    fn sheet(&self) -> Option<PilotSheet> {
        let session = &*self.session;
        let pilot = session.pilot();
        let reserves = session.reserves();
        let system = session.system();
        Some(PilotSheet {
            name: pilot.name().to_owned(),
            credits: pilot.cash(),
            shield: reserves.shield,
            armor: reserves.armor,
            fuel: reserves.fuel,
            date: session.date(),
            date_text: session.date_text(),
            system: self.system_place(system),
            landed: session
                .landed()
                .map(|stellar| self.stellar_place(system, stellar)),
            bits: pilot.control_bits().iter().collect(),
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
        let session = &mut *self.session;
        match edit {
            PilotEdit::Credits(credits) => session.set_credits(credits),
            PilotEdit::Reserve(reserve, amount) => {
                session.set_reserve(reserve, amount);
            }
            PilotEdit::Date(date) => session.set_date(date),
            PilotEdit::MoveTo { system, stellar } => session
                .relocate(self.catalog, system, stellar)
                .map_err(refusal)?,
            PilotEdit::Bit(bit, on) => session.set_control_bit(bit, on),
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::catalog::{
        CharacterStart, CombatCatalog, CommodityStrings, DateAffixes, DisasterRecord, GateSite,
        GovtRecord, HullRecord, JunkRecord, LandingSite, OutfitId, OutfitRecord, ShipId,
        ShipRecord, StarSystem, StartDate, StartError, WeaponRecord,
    };
    use nova_sim::{Bit, GameDate, Gauge, Pilot, Reserve, ShipFields, Vec2};

    use super::*;
    use crate::galaxy::catalog::{Galaxy, StellarEntry, SystemEntry};

    const CAN_LAND: u32 = 0x01;

    /// The first `chär` flies ship 128 from Sol (130) on 23 June 1177.
    /// Sol holds Earth (128), over which the ship starts, and Alpha
    /// Centauri (131) holds Proxima (140) and a hypergate (141).
    struct Catalog;

    fn site(id: i16, (x, y): (f32, f32), flags2: u16) -> LandingSite {
        LandingSite {
            id: StellarId(id),
            position: Vec2::new(x, y),
            frame_size: Some((100, 100)),
            flags: CAN_LAND,
            min_status: 0,
            landing_sound: None,
            tech_level: 0,
            special_tech: [0; 8],
            govt: None,
            flags2,
        }
    }

    impl PilotCatalog for Catalog {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                cash: 1000,
                legal: [None; 4],
                on_start: nova_sim::Script::default(),
            })
        }

        fn ship_fields(&self, _id: ShipId) -> Result<ShipFields, String> {
            Ok(ShipFields {
                speed: 300,
                accel: 300,
                maneuver: 10,
                shield: 100,
                armor: 50,
                fuel: 300,
                ..ShipFields::default()
            })
        }

        fn default_outfits(&self, _id: ShipId) -> Vec<(OutfitId, u16)> {
            Vec::new()
        }

        fn outfits(&self) -> Vec<OutfitRecord> {
            Vec::new()
        }

        fn ships(&self) -> Vec<ShipRecord> {
            Vec::new()
        }

        fn system_exists(&self, id: SystemId) -> bool {
            matches!(id.0, 130 | 131)
        }

        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            match system.0 {
                130 => vec![site(128, (0.0, 0.0), 0)],
                131 => vec![site(140, (40.0, 30.0), 0), site(141, (0.0, 500.0), 0x1000)],
                _ => Vec::new(),
            }
        }

        fn star_map(&self) -> Vec<StarSystem> {
            Vec::new()
        }

        fn commodity_strings(&self) -> CommodityStrings {
            CommodityStrings::default()
        }

        fn junk(&self) -> Vec<JunkRecord> {
            Vec::new()
        }

        fn disasters(&self) -> Vec<DisasterRecord> {
            Vec::new()
        }

        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            Vec::new()
        }

        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: String::new(),
                suffix: " NC".to_owned(),
            }
        }

        fn gate_sites(&self) -> Vec<GateSite> {
            Vec::new()
        }
    }

    impl CombatCatalog for Catalog {
        fn weapons(&self) -> Vec<WeaponRecord> {
            Vec::new()
        }

        fn hulls(&self) -> Vec<HullRecord> {
            Vec::new()
        }

        fn governments(&self) -> Vec<GovtRecord> {
            Vec::new()
        }
    }

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

    fn galaxy() -> GalaxyModel {
        GalaxyModel::new(Galaxy {
            systems: vec![
                system(131, "Alpha Centauri", &[(140, "Proxima"), (141, "Gate")]),
                system(130, "Sol", &[(128, "Earth")]),
            ],
            ..Galaxy::default()
        })
    }

    fn session() -> Session {
        Session::fly(&Catalog, Pilot::new(&Catalog, "Ada").expect("starts")).expect("flies")
    }

    fn landed() -> Session {
        let mut session = session();
        while session.landed().is_none() {
            session.land().expect("lands");
        }
        session.take_save_due();
        session
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

    #[test]
    fn the_sheet_reads_the_session() {
        let mut session = landed();
        session.set_control_bit(bit(42), true);
        session.set_control_bit(bit(7), true);
        session.set_reserve(Reserve::Fuel, 120.0);
        let galaxy = galaxy();
        let desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
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
    }

    #[test]
    fn in_flight_the_sheet_has_no_stellar_and_a_place_unnamed_is_its_id() {
        let mut session = session();
        let galaxy = GalaxyModel::new(Galaxy::default());
        let desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
        let sheet = desk.sheet().expect("a pilot");
        assert_eq!(sheet.landed, None);
        assert_eq!(sheet.system, place(SystemId(130), "130"));
        let mut session = landed();
        let desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
        assert_eq!(
            desk.sheet().expect("a pilot").landed,
            Some(place(StellarId(128), "128"))
        );
    }

    #[test]
    fn the_places_are_the_galaxys() {
        let mut session = session();
        let galaxy = galaxy();
        let desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
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
    fn each_edit_reaches_the_session() {
        let mut session = landed();
        let galaxy = galaxy();
        let mut desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
        let date = GameDate::new(1180, 1, 2).expect("a date");
        for edit in [
            PilotEdit::Credits(77),
            PilotEdit::Reserve(Reserve::Shield, 1000.0),
            PilotEdit::Reserve(Reserve::Armor, 5.0),
            PilotEdit::Reserve(Reserve::Fuel, -5.0),
            PilotEdit::Date(date),
            PilotEdit::Bit(bit(9), true),
            PilotEdit::Bit(bit(10), true),
            PilotEdit::Bit(bit(10), false),
        ] {
            assert_eq!(desk.edit(edit), Ok(()), "{edit:?}");
        }
        assert_eq!(session.pilot().cash(), 77);
        assert_eq!(session.reserves().shield.now, 100.0, "held within");
        assert_eq!(session.reserves().armor.now, 5.0);
        assert_eq!(session.reserves().fuel.now, 0.0);
        assert_eq!(session.date(), date);
        assert!(session.control_bit(bit(9)));
        assert!(!session.control_bit(bit(10)));
        assert!(session.take_save_due());
    }

    #[test]
    fn a_move_lands_the_pilot_at_the_stellar() {
        let mut session = landed();
        let galaxy = galaxy();
        let mut desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
        let to = |stellar| PilotEdit::MoveTo {
            system: SystemId(131),
            stellar: StellarId(stellar),
        };
        assert_eq!(desk.edit(to(141)), Err(EditRefusal::NotLandable));
        assert_eq!(desk.edit(to(999)), Err(EditRefusal::NoStellar));
        assert_eq!(
            desk.edit(PilotEdit::MoveTo {
                system: SystemId(132),
                stellar: StellarId(140)
            }),
            Err(EditRefusal::NoSystem)
        );
        assert_eq!(desk.edit(to(140)), Ok(()));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(session.landed(), Some(StellarId(140)));
    }

    #[test]
    fn a_move_in_flight_is_refused_and_changes_nothing() {
        let mut session = session();
        let before = session.clone();
        let galaxy = galaxy();
        let mut desk = SessionDesk::new(&mut session, &Catalog, &galaxy);
        assert_eq!(
            desk.edit(PilotEdit::MoveTo {
                system: SystemId(131),
                stellar: StellarId(140)
            }),
            Err(EditRefusal::InFlight)
        );
        assert_eq!(session, before);
    }

    #[test]
    fn each_refusal_maps_to_its_own() {
        for (from, to) in [
            (RelocateRefusal::InFlight, EditRefusal::InFlight),
            (RelocateRefusal::NoSystem, EditRefusal::NoSystem),
            (RelocateRefusal::NoStellar, EditRefusal::NoStellar),
            (RelocateRefusal::NotLandable, EditRefusal::NotLandable),
        ] {
            assert_eq!(refusal(from), to);
        }
    }
}
