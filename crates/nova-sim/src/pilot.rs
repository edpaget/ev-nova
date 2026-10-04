//! The pilot: everything about the player that outlives a flight and goes
//! into a save.
//!
//! A new pilot starts as the first `chär` by ascending ID says: its ship,
//! in the first of its starting systems that exists, on its starting date,
//! with its cash (none, when the `chär`'s is negative) and its legal
//! records, and with the ship's shield, armour and fuel full. It has
//! explored only the system it starts in. It holds no cargo, and no
//! planetary event is under way.
//!
//! A [`Session`](crate::Session) flies a pilot and changes it as the rules
//! say.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{DisasterId, GovtId, PilotCatalog, ShipId, StartError, StellarId, SystemId};
use crate::date::GameDate;
use crate::market::Good;
use crate::reserves::Reserves;

/// Everything about the player that a save keeps.
#[derive(Clone, Debug, PartialEq)]
pub struct Pilot {
    /// The pilot's name; empty for a pilot that is never saved.
    pub(crate) name: String,
    /// The ship class flown.
    pub(crate) ship: ShipId,
    /// The system the ship is in.
    pub(crate) system: SystemId,
    /// The stellar last landed on in that system, if any.
    pub(crate) stellar: Option<StellarId>,
    /// Today's date.
    pub(crate) date: GameDate,
    /// Credits.
    pub(crate) cash: i64,
    /// The ship's shield, armour and fuel.
    pub(crate) reserves: Reserves,
    /// The systems still to jump to, in order, ending at the destination.
    pub(crate) course: Vec<SystemId>,
    /// Every system the pilot has been in.
    pub(crate) explored: BTreeSet<SystemId>,
    /// The legal record with each government that has one.
    pub(crate) legal: BTreeMap<GovtId, i16>,
    /// The tons held of each good the ship carries; none of a good not
    /// listed.
    pub(crate) cargo: BTreeMap<Good, u32>,
    /// The planetary events under way, each `öops` with the days it has
    /// left.
    pub(crate) events: BTreeMap<DisasterId, u16>,
}

impl Pilot {
    /// A new pilot named `name`, read from `catalog`'s first `chär`.
    ///
    /// # Errors
    ///
    /// When there is no `chär`, it cannot be read, it names no ship or one
    /// that cannot be read, or none of its starting systems exists.
    pub fn new(catalog: &impl PilotCatalog, name: &str) -> Result<Self, StartError> {
        let character = catalog.first_character()?;
        let ship = character.ship.ok_or(StartError::NoShip)?;
        let fields = catalog
            .ship_fields(ship)
            .map_err(|reason| StartError::Ship(ship, reason))?;
        let system = character
            .systems
            .into_iter()
            .flatten()
            .find(|&id| catalog.system_exists(id))
            .ok_or(StartError::NoStartingSystem(character.systems))?;
        Ok(Self {
            name: name.to_owned(),
            ship,
            system,
            stellar: None,
            date: GameDate::from_start(character.start),
            cash: i64::from(character.cash.max(0)),
            reserves: Reserves::from_fields(fields),
            course: Vec::new(),
            explored: BTreeSet::from([system]),
            legal: character.legal.into_iter().flatten().collect(),
            cargo: BTreeMap::new(),
            events: BTreeMap::new(),
        })
    }

    /// The pilot's name; empty for a pilot that is never saved.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The ship class flown.
    #[must_use]
    pub fn ship(&self) -> ShipId {
        self.ship
    }

    /// The system the ship is in.
    #[must_use]
    pub fn system(&self) -> SystemId {
        self.system
    }

    /// The stellar last landed on in the system the ship is in, if any.
    #[must_use]
    pub fn stellar(&self) -> Option<StellarId> {
        self.stellar
    }

    /// Today's date.
    #[must_use]
    pub fn date(&self) -> GameDate {
        self.date
    }

    /// Credits.
    #[must_use]
    pub fn cash(&self) -> i64 {
        self.cash
    }

    /// Sets the credits to `cash`.
    pub fn set_cash(&mut self, cash: i64) {
        self.cash = cash;
    }

    /// The ship's shield, armour and fuel.
    #[must_use]
    pub fn reserves(&self) -> Reserves {
        self.reserves
    }

    /// The systems still to jump to, in order, ending at the destination.
    #[must_use]
    pub fn course(&self) -> &[SystemId] {
        &self.course
    }

    /// Every system the pilot has been in, by ascending ID.
    pub fn explored(&self) -> impl Iterator<Item = SystemId> + '_ {
        self.explored.iter().copied()
    }

    /// Whether the pilot has been in `system`.
    #[must_use]
    pub fn has_explored(&self, system: SystemId) -> bool {
        self.explored.contains(&system)
    }

    /// Marks `system` explored, and says whether it was new.
    pub fn explore(&mut self, system: SystemId) -> bool {
        self.explored.insert(system)
    }

    /// The legal record with `govt`: 0 when it has none.
    #[must_use]
    pub fn legal_record(&self, govt: GovtId) -> i16 {
        self.legal.get(&govt).copied().unwrap_or(0)
    }

    /// Sets the legal record with `govt` to `record`.
    pub fn set_legal_record(&mut self, govt: GovtId, record: i16) {
        self.legal.insert(govt, record);
    }

    /// Every government with a legal record, by ascending ID, and the
    /// record.
    pub fn legal_records(&self) -> impl Iterator<Item = (GovtId, i16)> + '_ {
        self.legal.iter().map(|(&govt, &record)| (govt, record))
    }

    /// The tons of `good` held.
    #[must_use]
    pub fn held(&self, good: Good) -> u32 {
        self.cargo.get(&good).copied().unwrap_or(0)
    }

    /// Every good held, in [`Good`] order, with the tons held.
    pub fn cargo(&self) -> impl Iterator<Item = (Good, u32)> + '_ {
        self.cargo.iter().map(|(&good, &tons)| (good, tons))
    }

    /// Every planetary event under way, by `öops` ID, with the days it has
    /// left.
    pub fn events(&self) -> impl Iterator<Item = (DisasterId, u16)> + '_ {
        self.events.iter().map(|(&id, &days)| (id, days))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{CharacterStart, GovtId, ShipId, StartError, SystemId};
    use crate::date::GameDate;
    use crate::reserves::{Gauge, Reserves};
    use crate::testkit::{FAST, FakePilotCatalog, START, catalog, starting};

    #[test]
    fn a_new_pilot_flies_the_first_chärs_ship_from_its_first_system_that_exists() {
        let pilot =
            Pilot::new(&starting([None, Some(999), Some(131), Some(130)]), "Ada").expect("starts");
        assert_eq!(pilot.name(), "Ada");
        assert_eq!(pilot.ship(), ShipId(128));
        assert_eq!(pilot.system(), SystemId(131));
        assert_eq!(pilot.stellar(), None);
        assert_eq!(pilot.date(), GameDate::from_start(START));
        assert_eq!(pilot.course(), []);
    }

    #[test]
    fn a_new_pilots_reserves_are_full_from_its_ship() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(pilot.reserves(), Reserves::from_fields(FAST));
        assert_eq!(pilot.reserves().fuel, Gauge::full(300.0));
    }

    /// The catalog with the first `chär` holding `cash` and these legal
    /// records.
    fn holding(cash: i32, legal: [Option<(i16, i16)>; 4]) -> FakePilotCatalog {
        FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: START,
                cash,
                legal: legal.map(|slot| slot.map(|(govt, record)| (GovtId(govt), record))),
            }),
            ..catalog()
        }
    }

    #[test]
    fn a_new_pilot_has_the_chärs_cash_and_none_below_zero() {
        let cash = |amount| Pilot::new(&holding(amount, [None; 4]), "").map(|p| p.cash());
        assert_eq!(cash(25_000), Ok(25_000));
        assert_eq!(cash(i32::MAX), Ok(i64::from(i32::MAX)));
        assert_eq!(cash(0), Ok(0));
        assert_eq!(cash(-1), Ok(0));
        assert_eq!(cash(i32::MIN), Ok(0));
    }

    #[test]
    fn a_new_pilot_has_the_chärs_legal_records_and_0_elsewhere() {
        let catalog = holding(0, [Some((129, 50)), None, Some((131, -20)), Some((140, 0))]);
        let pilot = Pilot::new(&catalog, "").expect("starts");
        assert_eq!(pilot.legal_record(GovtId(129)), 50);
        assert_eq!(pilot.legal_record(GovtId(131)), -20);
        assert_eq!(pilot.legal_record(GovtId(140)), 0);
        assert_eq!(pilot.legal_record(GovtId(128)), 0, "unset");
        assert_eq!(
            pilot.legal_records().collect::<Vec<_>>(),
            [(GovtId(129), 50), (GovtId(131), -20), (GovtId(140), 0)]
        );
        let repeated = holding(0, [Some((129, 50)), Some((129, 7)), None, None]);
        let pilot = Pilot::new(&repeated, "").expect("starts");
        assert_eq!(pilot.legal_record(GovtId(129)), 7, "the later slot wins");
    }

    #[test]
    fn a_new_pilot_has_explored_only_its_starting_system() {
        let pilot = Pilot::new(&catalog(), "").expect("starts");
        assert_eq!(pilot.explored().collect::<Vec<_>>(), [SystemId(130)]);
        assert!(pilot.has_explored(SystemId(130)));
        assert!(!pilot.has_explored(SystemId(131)));
    }

    #[test]
    fn a_new_pilot_holds_no_cargo_and_no_event_is_under_way() {
        let pilot = Pilot::new(&catalog(), "").expect("starts");
        assert_eq!(pilot.cargo().count(), 0);
        assert_eq!(pilot.held(Good::Commodity(0)), 0);
        assert_eq!(pilot.events().count(), 0);
    }

    #[test]
    fn cargo_and_events_read_as_they_are_held() {
        use crate::catalog::JunkId;
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.cargo = BTreeMap::from([(Good::Junk(JunkId(146)), 2), (Good::Commodity(3), 7)]);
        pilot.events = BTreeMap::from([(DisasterId(130), 4), (DisasterId(128), 9)]);
        assert_eq!(pilot.held(Good::Commodity(3)), 7);
        assert_eq!(pilot.held(Good::Junk(JunkId(146))), 2);
        assert_eq!(pilot.held(Good::Commodity(0)), 0);
        assert_eq!(
            pilot.cargo().collect::<Vec<_>>(),
            [(Good::Commodity(3), 7), (Good::Junk(JunkId(146)), 2)]
        );
        assert_eq!(
            pilot.events().collect::<Vec<_>>(),
            [(DisasterId(128), 9), (DisasterId(130), 4)]
        );
    }

    #[test]
    fn exploring_adds_a_system_once() {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        assert!(pilot.explore(SystemId(132)));
        assert!(!pilot.explore(SystemId(132)), "already explored");
        assert!(pilot.explore(SystemId(131)));
        assert_eq!(
            pilot.explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131), SystemId(132)]
        );
    }

    #[test]
    fn cash_and_legal_records_can_be_set() {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        pilot.set_cash(-12);
        assert_eq!(pilot.cash(), -12);
        pilot.set_legal_record(GovtId(128), 300);
        assert_eq!(pilot.legal_record(GovtId(128)), 300);
    }

    #[test]
    fn a_new_pilot_fails_as_a_session_would() {
        let shipless = FakePilotCatalog {
            character: Ok(CharacterStart {
                ship: None,
                systems: [Some(SystemId(130)), None, None, None],
                ..CharacterStart::default()
            }),
            ..catalog()
        };
        assert_eq!(Pilot::new(&shipless, ""), Err(StartError::NoShip));
        let missing = FakePilotCatalog {
            character: Err(StartError::NoCharacter),
            ..catalog()
        };
        assert_eq!(Pilot::new(&missing, ""), Err(StartError::NoCharacter));
        let unreadable = FakePilotCatalog {
            ships: Vec::new(),
            ..catalog()
        };
        assert_eq!(
            Pilot::new(&unreadable, ""),
            Err(StartError::Ship(ShipId(128), "no shïp 128".to_owned()))
        );
        assert_eq!(
            Pilot::new(&starting([None, Some(999), None, None]), ""),
            Err(StartError::NoStartingSystem([
                None,
                Some(SystemId(999)),
                None,
                None
            ]))
        );
    }
}
