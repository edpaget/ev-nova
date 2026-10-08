//! The pilot: everything about the player that outlives a flight and goes
//! into a save.
//!
//! A new pilot starts as the first `chär` by ascending ID says: its ship,
//! in the first of its starting systems that exists, on its starting date,
//! with its cash (none, when the `chär`'s is negative) and its legal
//! records, its ship named after its class (the `chär`'s own christening
//! prompt is not shown yet), owning its ship's stock weapons and their `AmmoLoad` as
//! outfits ([`Arsenal::stock_outfits`]) and then its default items
//! (repeated slots adding up), as `_DoNewPilot` fits them (@0x18f48,
//! @0x18f4d), and with the ship's shield, armour and fuel full at what it
//! and those outfits can hold ([`crate::stats`]). It has explored only the system it
//! starts in. It holds no cargo, no planetary event is under way, and it
//! has no escorts.
//!
//! The fleet is a list of [`Escort`] records, each a ship class with its
//! reserves and its standing order, in the order the ships joined; a ship
//! captured joins it ([`Session::assign`](crate::Session::assign)), and
//! so does each fighter launched from the player's bays, marked
//! `carried`, until it docks again, and each ship hired in the bar
//! ([`Session::hire`](crate::Session::hire)), with the daily `wage` it
//! was hired at, and each person who joins the player, linked to its
//! `person`. In flight the session flies each as
//! an NPC beside the player. The carried fighters do not count towards
//! the fleet's most ([`Pilot::escort_count`]).
//!
//! The pilot also keeps the persons gone for good, destroyed without an
//! escape pod or captured, who never appear again, and those holding a
//! grudge against the player (see [`person`](crate::person)); a new pilot
//! has neither.
//!
//! It holds the 10,000 control bits missions and storylines script with
//! (see [`control`](crate::control)), every one clear on a new pilot, and
//! its [`Gender`], which the new-pilot dialog asks for and the `G` test
//! reads; a new pilot is male unless the dialog says otherwise, as the
//! stock dialog's Gender menu starts on Male.
//!
//! It keeps its ship's paint, which a paint outfit (`ModType` 43) gives
//! (see [`outfit_effects`](crate::outfit_effects)): a new pilot's ship is
//! unpainted (the original's (32, 32, 32), `_ResetPlayer` @0x1db4e), and
//! buying a ship unpaints it again (`_DoShipyardDialog` @0x5f022). The
//! paint is saved, as the original's pilot file keeps it
//! (`_WritePilotData` @0x727f2); drawing it is not done yet.
//!
//! It keeps its ship's name, the original's `_shipName`: a new pilot's
//! ship is named after its class (the `shïp`'s name; the original's
//! new-pilot dialog asks for one, `_CullNameString` @0x19195, which is
//! not shown yet), and a pilot from a save made before ships had names
//! is named after its class when flown. The `T` set operator renames it
//! (see the session's `ship_change` module); buying or capturing a ship
//! leaves the name as it is for now (the shipyard's name dialog is not
//! shown yet), and nothing displays it yet.
//!
//! A [`Session`](crate::Session) flies a pilot and changes it as the rules
//! say.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    CombatCatalog, DisasterId, GovtId, OutfitId, PersonId, PilotCatalog, ShipId, ShipRecord,
    StartError, StellarId, SystemId,
};
use crate::combat::armament::Arsenal;
use crate::control::{Bit, ControlBitSet};
use crate::date::GameDate;
use crate::escort::EscortOrder;
use crate::market::{ActiveEvent, Good};
use crate::outfit_effects::Rgb15;
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
    /// The ship's name. `None` for a ship never named: a save from
    /// before ships had names. Flying the pilot names it after its class.
    pub(crate) ship_name: Option<String>,
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
    /// The planetary events under way: each `öops` with the days it has
    /// left and the stellar it is at.
    pub(crate) events: BTreeMap<DisasterId, ActiveEvent>,
    /// How many of each outfit the ship carries; none of an outfit not
    /// listed.
    pub(crate) outfits: BTreeMap<OutfitId, u16>,
    /// Whether the ship still carries its class's default items, not yet
    /// read into `outfits`: a save from before outfits were kept. Flying
    /// the pilot reads them.
    pub(crate) default_outfits_pending: bool,
    /// Whether the ship still mounts its class's stock weapons and their
    /// `AmmoLoad` beside `outfits`, not yet fitted as outfits: a save from
    /// before stock weapons were outfits. Flying the pilot fits them.
    pub(crate) stock_weapons_pending: bool,
    /// The fleet: every ship escorting the player, in the order it joined.
    pub(crate) escorts: Vec<Escort>,
    /// The persons gone for good, who never appear again (see
    /// [`person`](crate::person)).
    pub(crate) gone_persons: BTreeSet<PersonId>,
    /// The persons holding a grudge against the player.
    pub(crate) grudges: BTreeSet<PersonId>,
    /// The control bits.
    pub(crate) bits: ControlBitSet,
    /// The player's gender.
    pub(crate) gender: Gender,
    /// The ship's paint; `None` when unpainted.
    pub(crate) paint: Option<Rgb15>,
}

/// The player's gender, which the `G` test reads (true when male). The
/// stock new-pilot dialog's Gender menu lists Male, then Female, and starts
/// on Male.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Gender {
    /// Male: the `G` test holds.
    #[default]
    Male,
    /// Female.
    Female,
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
    /// Whether it is a fighter launched from one of the player's bays,
    /// out of its bay (see [`bay`](crate::bay)), rather than an escort of
    /// its own.
    pub carried: bool,
    /// The daily wage it was hired at in the bar, which may be none; `None`
    /// for an escort that was not hired (a ship captured, or the old ship
    /// kept after "Use As My Ship"). Which wage a hired escort is paid
    /// each day follows [`Session::with_escort_wage`](crate::Session::with_escort_wage)
    /// (see [`hire`](crate::hire)).
    pub wage: Option<i64>,
    /// The person (`përs`) this escort is, flying as itself, its name,
    /// ship and loadout its record's (see [`person`](crate::person));
    /// none for any other escort. One whose person no longer has a record
    /// flies as an ordinary escort of its ship class.
    pub person: Option<PersonId>,
}

impl Escort {
    /// Whether it was hired in the bar: it has a wage, even of none.
    #[must_use]
    pub fn hired(&self) -> bool {
        self.wage.is_some()
    }
}

impl Pilot {
    /// A new pilot named `name`, read from `catalog`'s first `chär`.
    ///
    /// # Errors
    ///
    /// When there is no `chär`, it cannot be read, it names no ship or one
    /// that cannot be read, or none of its starting systems exists.
    pub fn new(
        catalog: &(impl PilotCatalog + CombatCatalog),
        name: &str,
    ) -> Result<Self, StartError> {
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
        let records = catalog.outfits();
        let stock = Arsenal::read(catalog).stock_outfits(ship, &records);
        let outfits = merged(&stock, &default_outfits(catalog, ship));
        let stats = ShipStats::new(fields, &outfit_mods(&outfits, &records));
        Ok(Self {
            name: name.to_owned(),
            ship,
            ship_name: Some(class_name(&catalog.ships(), ship)),
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
            stock_weapons_pending: false,
            escorts: Vec::new(),
            gone_persons: BTreeSet::new(),
            grudges: BTreeSet::new(),
            bits: ControlBitSet::new(),
            gender: Gender::default(),
            paint: None,
        })
    }

    /// The pilot with `gender`, everything else unchanged.
    #[must_use]
    pub fn with_gender(self, gender: Gender) -> Self {
        Self { gender, ..self }
    }

    /// The player's gender.
    #[must_use]
    pub fn gender(&self) -> Gender {
        self.gender
    }

    /// The ship's paint, `None` when it is unpainted.
    #[must_use]
    pub fn paint(&self) -> Option<Rgb15> {
        self.paint
    }

    /// Whether control bit `bit` is set.
    #[must_use]
    pub fn control_bit(&self, bit: Bit) -> bool {
        self.bits.get(bit)
    }

    /// The control bits.
    #[must_use]
    pub fn control_bits(&self) -> &ControlBitSet {
        &self.bits
    }

    /// The control bits, to write.
    pub(crate) fn bits_mut(&mut self) -> &mut ControlBitSet {
        &mut self.bits
    }

    /// The fleet: every ship escorting the player, in the order it joined.
    #[must_use]
    pub fn escorts(&self) -> &[Escort] {
        &self.escorts
    }

    /// How many escorts the fleet holds, leaving out the carried
    /// fighters, which do not count towards its most (`_CanHireEscorts`
    /// counts AI type 6 alone).
    #[must_use]
    pub fn escort_count(&self) -> usize {
        self.escorts.iter().filter(|escort| !escort.carried).count()
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

    /// The ship's name; `None` for a ship never named (see
    /// [`Session::fly`](crate::Session::fly)).
    #[must_use]
    pub fn ship_name(&self) -> Option<&str> {
        self.ship_name.as_deref()
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
        self.events.iter().map(|(&id, active)| (id, active.days))
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

    /// Whether person `id` is gone for good, never to appear again.
    #[must_use]
    pub fn gone(&self, id: PersonId) -> bool {
        self.gone_persons.contains(&id)
    }

    /// Whether person `id` holds a grudge against the player.
    #[must_use]
    pub fn grudge(&self, id: PersonId) -> bool {
        self.grudges.contains(&id)
    }

    /// Every person gone for good, by ascending ID.
    pub fn gone_persons(&self) -> impl Iterator<Item = PersonId> + '_ {
        self.gone_persons.iter().copied()
    }

    /// Every person holding a grudge against the player, by ascending ID.
    pub fn grudges(&self) -> impl Iterator<Item = PersonId> + '_ {
        self.grudges.iter().copied()
    }
}

/// The name of ship class `ship` among `records`, as a ship of it is
/// named until something renames it: empty when it has no record.
pub(crate) fn class_name(records: &[ShipRecord], ship: ShipId) -> String {
    records
        .iter()
        .find(|record| record.id == ship)
        .map(|record| record.name.clone())
        .unwrap_or_default()
}

/// Ship `ship`'s default items from `catalog`, each with how many: repeated
/// slots add up, and none of an item is not listed.
pub(crate) fn default_outfits(
    catalog: &impl PilotCatalog,
    ship: ShipId,
) -> BTreeMap<OutfitId, u16> {
    tally(catalog.default_outfits(ship))
}

/// `a` and `b` together, saturating.
pub(crate) fn merged(
    a: &BTreeMap<OutfitId, u16>,
    b: &BTreeMap<OutfitId, u16>,
) -> BTreeMap<OutfitId, u16> {
    let mut both = a.clone();
    for (&id, &count) in b {
        let owned = both.entry(id).or_default();
        *owned = owned.saturating_add(count);
    }
    both
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
    use crate::catalog::{CharacterStart, GovtId, PersonId, ShipId, StartError, SystemId};
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
    fn a_new_pilots_ship_is_named_after_its_class() {
        let named = FakePilotCatalog {
            ship_records: vec![crate::testkit::ship(128, FAST)],
            ..catalog()
        };
        let pilot = Pilot::new(&named, "Ada").expect("starts");
        assert_eq!(pilot.ship_name(), Some("Ship 128"));
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(pilot.ship_name(), Some(""), "no record of its class");
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
                on_start: crate::control::Script::default(),
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
    fn a_new_pilot_owns_its_ships_stock_weapons_and_rounds_then_its_default_items() {
        use crate::catalog::{HullRecord, OutfitId, StockWeapon, WeaponId, WeaponRecord};
        use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
        use crate::testkit::{hull, outfit, weapon};
        let stock = |id, count, ammo| StockWeapon {
            weapon: WeaponId(id),
            count,
            ammo,
        };
        let catalog = FakePilotCatalog {
            weapons: vec![
                weapon(150),
                WeaponRecord {
                    ammo_type: 10,
                    ..weapon(138)
                },
            ],
            hulls: vec![HullRecord {
                weapons: vec![stock(150, 2, 0), stock(138, 1, 20)],
                ..hull(128)
            }],
            outfits: vec![
                outfit(200, &[(MOD_WEAPON, 150)]),
                outfit(201, &[(MOD_AMMO, 138)]),
                outfit(207, &[(MOD_WEAPON, 138)]),
            ],
            defaults: vec![(ShipId(128), vec![(OutfitId(200), 1)])],
            ..catalog()
        };
        let pilot = Pilot::new(&catalog, "").expect("starts");
        assert_eq!(
            pilot.outfits().collect::<Vec<_>>(),
            [(OutfitId(200), 3), (OutfitId(201), 20), (OutfitId(207), 1)],
            "the stock weapons, then the default items on top"
        );
        assert!(!pilot.stock_weapons_pending);
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
        let active = |days| ActiveEvent {
            days,
            stellar: Some(StellarId(140)),
        };
        pilot.events = BTreeMap::from([(DisasterId(130), active(4)), (DisasterId(128), active(9))]);
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
    fn a_new_pilot_has_lost_no_person_and_holds_no_grudge() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert!(!pilot.gone(PersonId(151)));
        assert!(!pilot.grudge(PersonId(510)));
        assert_eq!(pilot.gone_persons().count(), 0);
        assert_eq!(pilot.grudges().count(), 0);
    }

    #[test]
    fn the_persons_gone_and_the_grudges_read_as_they_are_kept() {
        let mut pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        pilot.gone_persons.insert(PersonId(151));
        pilot.grudges.insert(PersonId(510));
        assert!(pilot.gone(PersonId(151)));
        assert!(!pilot.gone(PersonId(510)));
        assert!(pilot.grudge(PersonId(510)));
        assert!(!pilot.grudge(PersonId(151)));
        assert_eq!(pilot.gone_persons().collect::<Vec<_>>(), [PersonId(151)]);
        assert_eq!(pilot.grudges().collect::<Vec<_>>(), [PersonId(510)]);
    }

    #[test]
    fn a_new_pilot_has_no_control_bit_set_and_is_male() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(pilot.bits, ControlBitSet::new());
        assert!(!pilot.control_bit(Bit::new(0).expect("in range")));
        assert_eq!(pilot.gender(), Gender::Male);
        assert_eq!(Gender::default(), Gender::Male);
    }

    #[test]
    fn a_new_pilots_ship_is_unpainted() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        assert_eq!(pilot.paint(), None);
        let painted = Pilot {
            paint: Some(Rgb15 { r: 1, g: 2, b: 3 }),
            ..pilot
        };
        assert_eq!(painted.paint(), Some(Rgb15 { r: 1, g: 2, b: 3 }));
    }

    #[test]
    fn a_pilot_with_a_gender_changes_only_that() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        let female = pilot.clone().with_gender(Gender::Female);
        assert_eq!(female.gender(), Gender::Female);
        assert_eq!(
            Pilot {
                gender: Gender::Male,
                ..female.clone()
            },
            pilot
        );
        assert_eq!(female.with_gender(Gender::Male).gender(), Gender::Male);
    }

    #[test]
    fn a_control_bit_reads_what_was_written() {
        let mut pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        let bit = Bit::new(9999).expect("in range");
        pilot.bits.set(bit);
        assert!(pilot.control_bit(bit));
        assert!(!pilot.control_bit(Bit::new(9998).expect("in range")));
        assert_eq!(pilot.control_bits().iter().collect::<Vec<_>>(), [bit]);
        pilot.bits.clear(bit);
        assert!(!pilot.control_bit(bit));
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
            carried: false,
            wage: None,
            person: None,
        };
        pilot.escorts.push(escort);
        assert_eq!(pilot.escorts(), [escort]);
    }

    #[test]
    fn the_escort_count_leaves_out_the_carried_fighters() {
        let mut pilot = Pilot::new(&catalog(), "").expect("starts");
        assert_eq!(pilot.escort_count(), 0);
        let escort = |carried| Escort {
            ship: ShipId(130),
            reserves: Reserves::full(10.0, 20.0, 30.0),
            order: None,
            carried,
            wage: None,
            person: None,
        };
        pilot.escorts = vec![escort(true), escort(false), escort(true), escort(false)];
        assert_eq!(pilot.escort_count(), 2);
        pilot.escorts.push(escort(false));
        assert_eq!(pilot.escort_count(), 3);
    }

    #[test]
    fn an_escort_is_hired_only_with_a_wage_even_of_none() {
        let escort = |wage| Escort {
            ship: ShipId(130),
            reserves: Reserves::full(10.0, 20.0, 30.0),
            order: None,
            carried: false,
            wage,
            person: None,
        };
        assert!(!escort(None).hired(), "captured");
        assert!(escort(Some(0)).hired());
        assert!(escort(Some(100)).hired());
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
