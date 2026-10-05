//! The pilot: everything about the player that outlives a flight and goes
//! into a save.
//!
//! A new pilot starts as the first `chär` by ascending ID says: its ship,
//! in the first of its starting systems that exists, on its starting date,
//! with its cash (none, when the `chär`'s is negative) and its legal
//! records, owning its ship's default items (repeated slots adding up),
//! and with the ship's shield, armour and fuel full at what it and those
//! items can hold ([`crate::stats`]). It has explored only the system it
//! starts in. It holds no cargo, no planetary event is under way, and it
//! has no escorts.
//!
//! The fleet is a list of [`Escort`] records, each a ship class with its
//! reserves, in the order the ships joined; a ship captured joins it
//! ([`Session::assign`](crate::Session::assign)).
//!
//! A [`Session`](crate::Session) flies a pilot and changes it as the rules
//! say.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    DisasterId, GovtId, OutfitId, PilotCatalog, ShipId, StartError, StellarId, SystemId,
};
use crate::date::GameDate;
use crate::escort::EscortOrder;
use crate::market::Good;
use crate::outfitter::outfit_mods;
use crate::reserves::Reserves;
use crate::stats::ShipStats;

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
    /// How many of each outfit the ship carries; none of an outfit not
    /// listed.
    pub(crate) outfits: BTreeMap<OutfitId, u16>,
    /// Whether the ship still carries its class's default items, not yet
    /// read into `outfits`: a save from before outfits were kept. Flying
    /// the pilot reads them.
    pub(crate) default_outfits_pending: bool,
    /// The fleet: every ship escorting the player, in the order it joined.
    pub(crate) escorts: Vec<Escort>,
}

/// A ship in the player's fleet: its class, its shield, armour and fuel,
/// and the standing order it follows. In flight the session flies it as
/// an NPC beside the player ([`Session`](crate::Session)); the record
/// keeps what a save keeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Escort {
    /// The escort's ship class.
    pub ship: ShipId,
    /// Its shield, armour and fuel.
    pub reserves: Reserves,
    /// Its standing order; none keeps formation (see
    /// [`escort`](crate::escort)).
    pub order: Option<EscortOrder>,
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
        let outfits = default_outfits(catalog, ship);
        let stats = ShipStats::new(fields, &outfit_mods(&outfits, &catalog.outfits()));
        Ok(Self {
            name: name.to_owned(),
            ship,
            system,
            stellar: None,
            date: GameDate::from_start(character.start),
            cash: i64::from(character.cash.max(0)),
            reserves: stats.full(),
            course: Vec::new(),
            explored: BTreeSet::from([system]),
            legal: character.legal.into_iter().flatten().collect(),
            cargo: BTreeMap::new(),
            events: BTreeMap::new(),
            outfits,
            default_outfits_pending: false,
            escorts: Vec::new(),
        })
    }

    /// The fleet: every ship escorting the player, in the order it joined.
    #[must_use]
    pub fn escorts(&self) -> &[Escort] {
        &self.escorts
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

    /// How many of `outfit` the ship carries.
    #[must_use]
    pub fn owned(&self, outfit: OutfitId) -> u16 {
        self.outfits.get(&outfit).copied().unwrap_or(0)
    }

    /// Every outfit the ship carries, by `oütf` ID, with how many.
    pub fn outfits(&self) -> impl Iterator<Item = (OutfitId, u16)> + '_ {
        self.outfits.iter().map(|(&id, &count)| (id, count))
    }
}

/// Ship `ship`'s default items from `catalog`, each with how many: repeated
/// slots add up, and none of an item is not listed.
pub(crate) fn default_outfits(
    catalog: &impl PilotCatalog,
    ship: ShipId,
) -> BTreeMap<OutfitId, u16> {
    tally(catalog.default_outfits(ship))
}

/// `items`, each an outfit with a count, as how many of each: repeats add
/// up, saturating, and none of an item is not listed.
pub(crate) fn tally(items: impl IntoIterator<Item = (OutfitId, u16)>) -> BTreeMap<OutfitId, u16> {
    let mut outfits = BTreeMap::new();
    for (id, count) in items {
        let owned: &mut u16 = outfits.entry(id).or_default();
        *owned = owned.saturating_add(count);
    }
    outfits.retain(|_, count| *count > 0);
    outfits
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{CharacterStart, GovtId, ShipId, StartError, SystemId};
    use crate::date::GameDate;
    use crate::reserves::Gauge;
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
        assert_eq!(
            pilot.reserves(),
            crate::stats::ShipStats::new(FAST, &[]).full()
        );
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
    fn a_new_pilot_owns_its_ships_default_items_with_repeats_adding_up() {
        use crate::catalog::OutfitId;
        use crate::stats::MORE_FUEL;
        use crate::testkit::outfit;
        let catalog = FakePilotCatalog {
            defaults: vec![(
                ShipId(128),
                vec![
                    (OutfitId(200), 2),
                    (OutfitId(201), 0),
                    (OutfitId(200), 1),
                    (OutfitId(999), 1),
                    (OutfitId(202), u16::MAX),
                    (OutfitId(202), 4),
                ],
            )],
            outfits: vec![outfit(200, &[(MORE_FUEL, 100)])],
            ..catalog()
        };
        let pilot = Pilot::new(&catalog, "").expect("starts");
        assert_eq!(
            pilot.outfits().collect::<Vec<_>>(),
            [
                (OutfitId(200), 3),
                (OutfitId(202), u16::MAX),
                (OutfitId(999), 1)
            ],
            "none of a count of 0; an item with no oütf is kept"
        );
        assert_eq!(pilot.owned(OutfitId(200)), 3);
        assert_eq!(pilot.owned(OutfitId(201)), 0);
        assert_eq!(pilot.reserves().fuel, Gauge::full(600.0), "three tanks");
        assert!(!pilot.default_outfits_pending);
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
    fn a_new_pilot_has_an_empty_fleet() {
        let pilot = Pilot::new(&catalog(), "").expect("starts");
        assert_eq!(pilot.escorts(), []);
    }

    #[test]
    fn the_fleet_reads_as_it_is_kept() {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        let escort = Escort {
            ship: ShipId(130),
            reserves: Reserves::full(10.0, 20.0, 30.0),
            order: Some(EscortOrder::Hold),
        };
        pilot.escorts.push(escort);
        assert_eq!(pilot.escorts(), [escort]);
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
