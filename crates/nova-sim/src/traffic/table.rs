//! What a system's traffic is drawn from: its `DudeTypes` decoded, the
//! fleets its `LinkSyst`s match, and the stats of every ship type it can
//! spawn, resolved once when the system is entered.
//!
//! The values are the original's (`_LoadObjectData` and `_SpawnFleet` in
//! the `EV Nova` executable):
//!
//! - A `DudeTypes` entry of 128-639 is a `düde`, and one of -128 to -383
//!   the `flët` with ID `|v|`; any other value is unused. The `düde`s'
//!   `% Prob` are rescaled to sum to 100 when their sum is above 0 and not
//!   100, each becoming `trunc(p * 100.0 / sum)`; the fleets' are kept as
//!   they are. A negative `% Prob` counts as none.
//! - A fleet's `LinkSyst` ([`FleetLink`]) says which systems it may
//!   appear in.
//! - A weighted [`pick`] draws `Rand(sum) + 1` and takes the first entry
//!   whose cumulative weight reaches it.
//! - The persons ([`SpawnPerson`]) kept are those whose `LinkSyst`
//!   allows the system ([`PersonLink`]), whom one of its Person slots
//!   names, or who escorts the player, and whose ship type has a record,
//!   each flying its ship as [`person::fit`] fits it.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    DudeId, FleetId, FleetRecord, GovtId, OutfitRecord, PersonId, PersonRecord, ShipId, ShipRecord,
    SystemId, TrafficCatalog, WeaponId,
};
use crate::chance::Chance;
use crate::combat::armament::{Armament, Arsenal};
use crate::combat::hull::{Condition, HullSpec};
use crate::control::Test;
use crate::escort::EscortClass;
use crate::govt::Governments;
use crate::outfitter::outfit_mods;
use crate::person::{self, PersonLink, PersonRules};
use crate::pilot::tally;
use crate::reserves::Reserves;
use crate::stats::ShipStats;

/// The lowest `DudeTypes` value that is a `düde`.
pub const FIRST_DUDE: i16 = 128;
/// The highest `DudeTypes` value that is a `düde`.
pub const LAST_DUDE: i16 = 639;
/// The `DudeTypes` value nearest 0 that is a fleet, `flët` 128.
pub const FIRST_FLEET: i16 = -128;
/// The `DudeTypes` value farthest from 0 that is a fleet, `flët` 383.
pub const LAST_FLEET: i16 = -383;
/// What the `düde`s' `% Prob` are rescaled to sum to.
pub const DUDE_PERCENT: u32 = 100;

/// A system's `DudeTypes`, decoded: its `düde`s, their `% Prob` rescaled,
/// and its fleets with theirs as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DudeTypes {
    /// Each `düde`, with its weight, in record order.
    pub dudes: Vec<(DudeId, u32)>,
    /// Each fleet, with its `% Prob`, in record order.
    pub fleets: Vec<(FleetId, u32)>,
}

/// Decodes `raw` `DudeTypes`, each with its `% Prob` (see the module
/// docs).
#[must_use]
pub fn decode_dude_types(raw: &[(i16, i16); 8]) -> DudeTypes {
    let odds = |prob: i16| u32::try_from(prob).unwrap_or(0);
    let mut decoded = DudeTypes::default();
    for &(value, prob) in raw {
        if (FIRST_DUDE..=LAST_DUDE).contains(&value) {
            decoded.dudes.push((DudeId(value), odds(prob)));
        } else if (LAST_FLEET..=FIRST_FLEET).contains(&value) {
            decoded.fleets.push((FleetId(-value), odds(prob)));
        }
    }
    let sum: u32 = decoded.dudes.iter().map(|&(_, weight)| weight).sum();
    for (_, weight) in &mut decoded.dudes {
        // The original's `trunc(p * 100.0 / sum)` in doubles is this whole
        // division exactly, and leaves a sum of 100 as it is; a sum of
        // none leaves each none.
        if let Some(scaled) = (*weight * DUDE_PERCENT).checked_div(sum) {
            *weight = scaled;
        }
    }
    decoded
}

/// Which systems a fleet may appear in, from its `LinkSyst`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleetLink {
    /// -1: any system.
    Any,
    /// 128-9999: this system.
    System(SystemId),
    /// 10000-14999: a system governed by this government.
    Govt(GovtId),
    /// 15000-19999: a system governed by an ally of this government.
    AlliesOf(GovtId),
    /// 20000-24999: a system with a government, other than this one.
    NotGovt(GovtId),
    /// 25000-29999: a system governed by an enemy of this government.
    EnemiesOf(GovtId),
    /// Anything else: no system.
    Never,
}

impl FleetLink {
    /// The link a raw `LinkSyst` gives.
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        let govt = |base: i16| GovtId(raw - base + 128);
        match raw {
            -1 => Self::Any,
            128..=9999 => Self::System(SystemId(raw)),
            10_000..=14_999 => Self::Govt(govt(10_000)),
            15_000..=19_999 => Self::AlliesOf(govt(15_000)),
            20_000..=24_999 => Self::NotGovt(govt(20_000)),
            25_000..=29_999 => Self::EnemiesOf(govt(25_000)),
            _ => Self::Never,
        }
    }

    /// Whether system `system`, governed by `system_govt`, matches, with
    /// the relations between governments in `govts` (`_SpawnFleet`
    /// @0x42768-0x4289d): the allies of a government include it, so its
    /// own systems match; an independent system matches neither allies
    /// nor enemies.
    #[must_use]
    pub fn matches(
        self,
        system: SystemId,
        system_govt: Option<GovtId>,
        govts: &Governments,
    ) -> bool {
        match self {
            Self::Any => true,
            Self::System(id) => id == system,
            Self::Govt(govt) => system_govt == Some(govt),
            Self::NotGovt(govt) => system_govt.is_some_and(|own| own != govt),
            Self::AlliesOf(govt) => system_govt.is_some() && govts.allies(Some(govt), system_govt),
            Self::EnemiesOf(govt) => {
                system_govt.is_some() && govts.enemies(Some(govt), system_govt)
            }
            Self::Never => false,
        }
    }
}

/// One of `weighted`, by weight: a draw of `Rand(sum) + 1` against the
/// cumulative weights. `None`, and no draw, when the weights sum to none.
pub fn pick<T: Copy>(weighted: &[(T, u32)], chance: &mut (impl Chance + ?Sized)) -> Option<T> {
    let sum = weighted
        .iter()
        .fold(0_u32, |sum, &(_, weight)| sum.saturating_add(weight));
    if sum == 0 {
        return None;
    }
    let roll = chance.below(sum) + 1;
    let mut total = 0_u32;
    weighted.iter().find_map(|&(item, weight)| {
        total = total.saturating_add(weight);
        (roll <= total).then_some(item)
    })
}

/// A `düde` as the traffic spawns from it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpawnDude {
    /// Its `AIType`, raw.
    pub ai_type: i16,
    /// Its government.
    pub govt: Option<GovtId>,
    /// Its ships, each with its weight.
    pub ships: Vec<(ShipId, u32)>,
    /// Its `Booty` flags.
    pub booty: u16,
    /// Its `InfoTypes` flags.
    pub info_types: u16,
}

/// A ship type the traffic can spawn: how it performs, with its default
/// items, its `InherentAI`, and what it fights with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShipKind {
    /// Its stats.
    pub stats: ShipStats,
    /// Its `InherentAI`, raw.
    pub inherent_ai: i16,
    /// Its class as an escort.
    pub escort_class: EscortClass,
    /// Its hull.
    pub hull: HullSpec,
    /// Its weapons: its stock weapons and those among its default items.
    pub armament: Armament,
    /// The rounds of each ammunition it carries.
    pub rounds: BTreeMap<WeaponId, u32>,
    /// Its `AppearOn`: while it does not hold, a `düde` or a fleet's
    /// escorts bring none of it (see [`spawn`](crate::traffic::spawn)).
    pub appear_on: Test,
}

/// A person as the traffic spawns it: its record, whether its `LinkSyst`
/// allows the system, the ship it flies, fitted (see [`person::fit`]),
/// the reserves and condition it starts in, and whether its government
/// is derelict.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnPerson {
    /// Its `përs`.
    pub record: PersonRecord,
    /// Whether its `LinkSyst` allows the system; a person a Person slot
    /// names may not.
    pub linked: bool,
    /// Its ship, with its weapons, rounds and scaled stats.
    pub kind: ShipKind,
    /// The shield, armour and fuel it starts with.
    pub reserves: Reserves,
    /// How it starts: disabled when derelict.
    pub condition: Condition,
    /// Whether its government is derelict (`gövt` `Flags` 0x0800).
    pub derelict: bool,
}

/// Everything a system's traffic is drawn from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpawnTable {
    /// Its `AvgShips`; none below none.
    pub avg_ships: u32,
    /// Its `düde`s, each with its weight.
    pub dudes: Vec<(DudeId, u32)>,
    /// The `düde`s that can be read.
    pub dude_records: BTreeMap<DudeId, SpawnDude>,
    /// The fleets its `DudeTypes` name, each with its `% Prob`.
    pub dude_fleets: Vec<(FleetId, u32)>,
    /// The fleets whose `LinkSyst` matches it. Their `AppearOn` is tested
    /// when one spawns (see [`spawn`](crate::traffic::spawn)), so a bit
    /// set in the system counts without resolving the table again.
    pub link_fleets: BTreeSet<FleetId>,
    /// The fleets it can spawn, from either list.
    pub fleets: BTreeMap<FleetId, FleetRecord>,
    /// Every ship type its `düde`s, fleets and persons name that has a
    /// record.
    pub ships: BTreeMap<ShipId, ShipKind>,
    /// The persons linked to it or named in its Person slots whose ship
    /// has a record.
    pub persons: BTreeMap<PersonId, SpawnPerson>,
    /// Its Person slots that name a person, each with its chance, in
    /// order.
    pub person_slots: Vec<(PersonId, i16)>,
}

impl SpawnTable {
    /// System `system`'s table, governed by `system_govt`, read from
    /// `catalog`, its fleets' links matched with the relations in
    /// `govts`, with each ship type's stats from its record in `ships`
    /// and its default items, the `oütf`s from `outfits`, as the player's
    /// are, and its hull and armament from `arsenal`; its persons' links
    /// and ships as `rules` says. The persons of `fleet`, the player's
    /// escorts, are kept wherever they are, so each flies as itself in
    /// every system. A system that cannot be read has no traffic, and
    /// keeps only the fleet's persons.
    #[must_use]
    // Each is a separate input the session reads once and keeps.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        catalog: &(impl TrafficCatalog + ?Sized),
        system: SystemId,
        system_govt: Option<GovtId>,
        govts: &Governments,
        ships: &[ShipRecord],
        outfits: &[OutfitRecord],
        arsenal: &Arsenal,
        rules: &dyn PersonRules,
        fleet: &BTreeSet<PersonId>,
    ) -> Self {
        let Some(traffic) = catalog.system_traffic(system) else {
            if fleet.is_empty() {
                return Self::default();
            }
            let mut persons = persons(
                catalog,
                (system, system_govt, govts),
                (&[], fleet),
                (ships, outfits, arsenal),
                rules,
            );
            persons.retain(|id, _| fleet.contains(id));
            let wanted: BTreeSet<ShipId> = persons
                .values()
                .filter_map(|person| person.record.ship)
                .collect();
            return Self {
                ships: kinds(ships, &wanted, outfits, arsenal),
                persons,
                ..Self::default()
            };
        };
        let decoded = decode_dude_types(&traffic.dude_types);
        let dude_records: BTreeMap<_, _> = decoded
            .dudes
            .iter()
            .filter_map(|&(id, _)| {
                let dude = catalog.dude(id)?;
                let ships = dude
                    .ships
                    .iter()
                    .map(|&(ship, prob)| (ship, u32::try_from(prob).unwrap_or(0)))
                    .collect();
                Some((
                    id,
                    SpawnDude {
                        ai_type: dude.ai_type,
                        govt: dude.govt,
                        ships,
                        booty: dude.booty,
                        info_types: dude.info_types,
                    },
                ))
            })
            .collect();
        let named: BTreeSet<_> = decoded.fleets.iter().map(|&(id, _)| id).collect();
        let mut link_fleets = BTreeSet::new();
        let mut fleets = BTreeMap::new();
        for fleet in catalog.fleets() {
            let linked = FleetLink::decode(fleet.link_syst).matches(system, system_govt, govts);
            if linked {
                link_fleets.insert(fleet.id);
            }
            if linked || named.contains(&fleet.id) {
                fleets.insert(fleet.id, fleet);
            }
        }
        let person_slots: Vec<(PersonId, i16)> = traffic
            .persons
            .iter()
            .filter_map(|&(id, prob)| Some((id?, prob)))
            .collect();
        let persons = persons(
            catalog,
            (system, system_govt, govts),
            (&person_slots, fleet),
            (ships, outfits, arsenal),
            rules,
        );
        let wanted: BTreeSet<ShipId> = dude_records
            .values()
            .flat_map(|dude| dude.ships.iter().map(|&(ship, _)| ship))
            .chain(fleets.values().flat_map(|fleet: &FleetRecord| {
                fleet
                    .lead
                    .into_iter()
                    .chain(fleet.escorts.iter().map(|escort| escort.ship))
            }))
            .chain(persons.values().filter_map(|person| person.record.ship))
            .collect();
        let ships = kinds(ships, &wanted, outfits, arsenal);
        Self {
            avg_ships: u32::try_from(traffic.avg_ships).unwrap_or(0),
            dudes: decoded.dudes,
            dude_records,
            dude_fleets: decoded.fleets,
            link_fleets,
            fleets,
            ships,
            persons,
            person_slots,
        }
    }

    /// The ship types the table can spawn, by ascending ID.
    #[must_use]
    pub fn ship_types(&self) -> Vec<ShipId> {
        self.ships.keys().copied().collect()
    }
}

/// Each of `ships` whose ID is `wanted`, as the traffic flies it.
fn kinds(
    ships: &[ShipRecord],
    wanted: &BTreeSet<ShipId>,
    outfits: &[OutfitRecord],
    arsenal: &Arsenal,
) -> BTreeMap<ShipId, ShipKind> {
    ships
        .iter()
        .filter(|record| wanted.contains(&record.id))
        .map(|record| (record.id, kind(record, outfits, arsenal)))
        .collect()
}

/// The persons of `catalog` linked to `system`, governed by `system_govt`
/// with the relations in `govts`, named in `slots` or in the player's
/// `fleet`, whose ship has a record among `ships`, each flying it fitted,
/// its default items from `outfits` and armed from `arsenal`, as `rules`
/// say.
fn persons(
    catalog: &(impl TrafficCatalog + ?Sized),
    (system, system_govt, govts): (SystemId, Option<GovtId>, &Governments),
    (slots, fleet): (&[(PersonId, i16)], &BTreeSet<PersonId>),
    (ships, outfits, arsenal): (&[ShipRecord], &[OutfitRecord], &Arsenal),
    rules: &dyn PersonRules,
) -> BTreeMap<PersonId, SpawnPerson> {
    catalog
        .persons()
        .into_iter()
        .filter_map(|record| {
            let linked = PersonLink::decode(record.link_syst).matches(
                system,
                system_govt,
                govts,
                rules.link_slip(),
            );
            let slotted = slots.iter().any(|&(id, _)| id == record.id);
            if !linked && !slotted && !fleet.contains(&record.id) {
                return None;
            }
            let ship = ships.iter().find(|ship| Some(ship.id) == record.ship)?;
            let derelict = govts.derelict(record.govt);
            let (kind, reserves, condition) = person::fit(
                &kind(ship, outfits, arsenal),
                &record,
                arsenal,
                rules.shield_mod(),
                derelict,
            );
            Some((
                record.id,
                SpawnPerson {
                    record,
                    linked,
                    kind,
                    reserves,
                    condition,
                    derelict,
                },
            ))
        })
        .collect()
}

/// A ship of `record`, carrying its default items, armed from `arsenal`.
pub(crate) fn kind(record: &ShipRecord, outfits: &[OutfitRecord], arsenal: &Arsenal) -> ShipKind {
    let defaults = tally(record.defaults.iter().copied());
    let (armament, rounds) = arsenal.npc(record.id, &defaults, outfits);
    ShipKind {
        stats: ShipStats::new(record.fields, &outfit_mods(&defaults, outfits)),
        inherent_ai: record.inherent_ai,
        escort_class: EscortClass::of(record.escort_type, record.inherent_ai, record.fields.mass),
        hull: arsenal.hull(record.id),
        armament,
        rounds,
        appear_on: record.appear_on.clone(),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::GovtRecord;
    use crate::catalog::PersonId;
    use crate::catalog::{
        DudeRecord, EscortRecord, HullRecord, OutfitId, PersonRecord, StockWeapon, SystemTraffic,
    };
    use crate::escort::EscortClass;
    use crate::handling::ShipFields;
    use crate::person::NovaPersons;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::MORE_SPEED;
    use crate::testkit::{Draws, FAST, govt, hull, outfit, person, ship, weapon};

    const UNUSED: (i16, i16) = (-1, 0);

    fn types(entries: &[(i16, i16)]) -> [(i16, i16); 8] {
        let mut raw = [UNUSED; 8];
        raw[..entries.len()].copy_from_slice(entries);
        raw
    }

    #[test]
    fn dude_types_split_into_dudes_and_fleets_at_their_edges() {
        let decoded = decode_dude_types(&types(&[
            (128, 10),
            (639, 20),
            (-128, 5),
            (-383, 6),
            (-1, 7),
            (640, 8),
            (127, 9),
            (-384, 4),
        ]));
        assert_eq!(
            decoded,
            DudeTypes {
                dudes: vec![(DudeId(128), 33), (DudeId(639), 66)],
                fleets: vec![(FleetId(128), 5), (FleetId(383), 6)],
            }
        );
        assert_eq!(
            [FIRST_DUDE, LAST_DUDE, FIRST_FLEET, LAST_FLEET],
            [128, 639, -128, -383]
        );
        let edges = decode_dude_types(&types(&[(-127, 50), (0, 50)]));
        assert_eq!(edges, DudeTypes::default(), "-127 and 0 are unused");
    }

    fn dude_weights(probs: &[i16]) -> Vec<u32> {
        let entries: Vec<_> = (0_i16..)
            .zip(probs)
            .map(|(slot, &prob)| (128 + slot, prob))
            .collect();
        decode_dude_types(&types(&entries))
            .dudes
            .into_iter()
            .map(|(_, weight)| weight)
            .collect()
    }

    #[test]
    fn dude_odds_are_rescaled_to_100_and_truncated() {
        assert_eq!(dude_weights(&[60, 40]), [60, 40], "already 100");
        assert_eq!(dude_weights(&[0, 0]), [0, 0], "none to rescale");
        assert_eq!(dude_weights(&[25, 25]), [50, 50], "50 doubles");
        assert_eq!(dude_weights(&[1, 1, 1]), [33, 33, 33], "truncated");
        assert_eq!(dude_weights(&[2, 1]), [66, 33]);
        assert_eq!(dude_weights(&[150, 50]), [75, 25], "above 100 shrinks");
        assert_eq!(dude_weights(&[-10, 30]), [0, 100], "negative is none");
        assert_eq!(dude_weights(&[]), Vec::<u32>::new());
        assert_eq!(DUDE_PERCENT, 100);
    }

    #[test]
    fn fleet_odds_are_kept_as_they_are() {
        let decoded = decode_dude_types(&types(&[(-129, 20), (-130, 150), (-131, -5)]));
        assert_eq!(
            decoded.fleets,
            [(FleetId(129), 20), (FleetId(130), 150), (FleetId(131), 0)]
        );
    }

    #[test]
    fn link_syst_decodes_at_each_ranges_edges() {
        for (raw, link) in [
            (-1, FleetLink::Any),
            (128, FleetLink::System(SystemId(128))),
            (9999, FleetLink::System(SystemId(9999))),
            (10_000, FleetLink::Govt(GovtId(128))),
            (14_999, FleetLink::Govt(GovtId(5127))),
            (15_000, FleetLink::AlliesOf(GovtId(128))),
            (19_999, FleetLink::AlliesOf(GovtId(5127))),
            (20_000, FleetLink::NotGovt(GovtId(128))),
            (24_999, FleetLink::NotGovt(GovtId(5127))),
            (25_000, FleetLink::EnemiesOf(GovtId(128))),
            (29_999, FleetLink::EnemiesOf(GovtId(5127))),
            (30_000, FleetLink::Never),
            (127, FleetLink::Never),
            (0, FleetLink::Never),
            (-2, FleetLink::Never),
            (i16::MIN, FleetLink::Never),
            (i16::MAX, FleetLink::Never),
        ] {
            assert_eq!(FleetLink::decode(raw), link, "{raw}");
        }
    }

    /// Governments 128 and 129 are allies (class 1 and its ally); 130 is
    /// at war with 128 (class 3, listing class 1); 131 is xenophobic;
    /// 133 is derelict;
    /// 132 is neutral to all.
    fn relations() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..govt(130)
            },
            GovtRecord {
                flags: crate::govt::XENOPHOBIC,
                ..govt(131)
            },
            govt(132),
            GovtRecord {
                flags: crate::govt::DERELICT,
                ..govt(133)
            },
        ])
    }

    #[test]
    fn each_link_matches_its_systems() {
        const HERE: SystemId = SystemId(130);
        let govts = relations();
        let (a, b) = (Some(GovtId(128)), Some(GovtId(129)));
        let matches = |raw: i16, govt| FleetLink::decode(raw).matches(HERE, govt, &govts);
        assert!(matches(-1, None) && matches(-1, a));
        assert!(matches(130, None) && !matches(131, None));
        assert!(matches(10_000, a) && !matches(10_000, b) && !matches(10_000, None));
        assert!(matches(20_000, b) && !matches(20_000, a));
        assert!(!matches(20_000, None), "an independent system has no govt");
        assert!(!matches(30_000, a) && !matches(0, None));
    }

    #[test]
    fn an_allies_link_matches_the_governments_own_systems_and_its_allies() {
        let govts = relations();
        let matches = |raw: i16, govt: Option<i16>| {
            FleetLink::decode(raw).matches(SystemId(130), govt.map(GovtId), &govts)
        };
        assert!(matches(15_000, Some(128)), "its own");
        assert!(matches(15_000, Some(129)), "an ally's");
        assert!(matches(15_001, Some(128)), "the other way round");
        assert!(!matches(15_000, Some(132)), "a neutral's");
        assert!(!matches(15_000, Some(130)), "an enemy's");
        assert!(!matches(15_000, None), "an independent system");
    }

    #[test]
    fn an_enemies_link_matches_an_enemys_systems_and_a_xenophobes() {
        let govts = relations();
        let matches = |raw: i16, govt: Option<i16>| {
            FleetLink::decode(raw).matches(SystemId(130), govt.map(GovtId), &govts)
        };
        assert!(matches(25_000, Some(130)), "an enemy's");
        assert!(matches(25_002, Some(128)), "the other way round");
        assert!(matches(25_000, Some(131)), "a xenophobe-ruled system");
        assert!(matches(25_003, Some(132)), "a xenophobe's link");
        assert!(!matches(25_000, Some(128)), "its own");
        assert!(!matches(25_000, Some(129)), "an ally's");
        assert!(!matches(25_000, Some(132)), "a neutral's");
        assert!(!matches(25_000, None), "an independent system");
    }

    #[test]
    fn a_pick_draws_one_more_than_below_the_sum_against_the_running_total() {
        let weighted = [('a', 10), ('b', 0), ('c', 30), ('d', 60)];
        for (draw, expected) in [
            (0, 'a'),
            (9, 'a'),
            (10, 'c'),
            (39, 'c'),
            (40, 'd'),
            (99, 'd'),
        ] {
            let mut chance = Draws::of(&[draw]);
            assert_eq!(pick(&weighted, &mut chance), Some(expected), "{draw}");
            assert_eq!(chance.asked, [100]);
        }
    }

    #[test]
    fn a_pick_over_no_weight_draws_nothing() {
        let mut chance = Draws::of(&[]);
        assert_eq!(pick::<char>(&[], &mut chance), None);
        assert_eq!(pick(&[('a', 0), ('b', 0)], &mut chance), None);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn under_never_fires_a_pick_takes_the_last_weighted_entry() {
        let weighted = [('a', 10), ('b', 5), ('c', 0)];
        assert_eq!(pick(&weighted, &mut crate::NeverFires), Some('b'));
    }

    /// System 130 (Federation, govt 128) names düde 128 at 60 and düde 129
    /// (unreadable) at 40, and fleet 140 at 20, with 4 ships on average.
    /// Düde 128 flies ship 200 and ship 999 (no record). Person 510, of
    /// derelict govt 133 and `ShieldMod` 200, links to Federation systems
    /// and flies ship 203; person 600, linked to system 131, is named in
    /// the system's second Person slot at 50 %, and 605, of no record, in
    /// its third at 20 %; 601 links to system 131 only; 602 links anywhere
    /// but flies ship 999; and 603's `LinkSyst` 2 slips to system 130.
    /// Fleet 140 is led by
    /// ship 201 with ship 202 escorting; fleet 141 links to any system,
    /// fleet 142 to Federation systems, fleet 143 to system 131, fleet
    /// 144 to Federation systems but led by nothing readable, fleet 145 to
    /// the systems of govt 129's allies and fleet 146 to those of its
    /// enemies.
    #[derive(Default)]
    struct Traffic {
        dudes_asked: RefCell<Vec<DudeId>>,
    }

    impl TrafficCatalog for Traffic {
        fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
            (id == SystemId(130)).then_some(SystemTraffic {
                dude_types: types(&[(128, 60), (129, 40), (-140, 20)]),
                avg_ships: 4,
                persons: [
                    (None, 0),
                    (Some(PersonId(600)), 50),
                    (Some(PersonId(605)), 20),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                ],
            })
        }

        fn dude(&self, id: DudeId) -> Option<DudeRecord> {
            self.dudes_asked.borrow_mut().push(id);
            (id == DudeId(128)).then(|| DudeRecord {
                ai_type: 3,
                govt: Some(GovtId(130)),
                ships: vec![(ShipId(200), 70), (ShipId(999), 30), (ShipId(201), -5)],
                booty: 0x0041,
                info_types: 0x4005,
            })
        }

        fn fleets(&self) -> Vec<FleetRecord> {
            let fleet = |id, lead, escort, link_syst| FleetRecord {
                id: FleetId(id),
                lead: Some(ShipId(lead)),
                escorts: vec![EscortRecord {
                    ship: ShipId(escort),
                    min: 1,
                    max: 2,
                }],
                govt: Some(GovtId(131)),
                link_syst,
                appear_on: Test::default(),
            };
            vec![
                fleet(140, 201, 202, 131),
                fleet(141, 201, 202, -1),
                fleet(142, 202, 201, 10_000),
                fleet(143, 202, 202, 131),
                fleet(144, 998, 202, 10_000),
                fleet(145, 202, 202, 15_001),
                fleet(146, 202, 202, 25_001),
            ]
        }

        fn persons(&self) -> Vec<PersonRecord> {
            vec![
                PersonRecord {
                    link_syst: 10_000,
                    shield_mod: 200,
                    govt: Some(GovtId(133)),
                    ..person(510, 203)
                },
                PersonRecord {
                    link_syst: 131,
                    ..person(600, 200)
                },
                PersonRecord {
                    link_syst: 131,
                    ..person(601, 200)
                },
                person(602, 999),
                PersonRecord {
                    link_syst: 2,
                    ..person(603, 200)
                },
            ]
        }
    }

    /// Ships 200-203, each with its own `InherentAI`; ship 200 carries a
    /// speed booster by default.
    fn records() -> Vec<ShipRecord> {
        let mut records: Vec<_> = (200..=203)
            .map(|id| ShipRecord {
                inherent_ai: id - 199,
                ..ship(id, FAST)
            })
            .collect();
        records[0].defaults = vec![(OutfitId(300), 1), (OutfitId(301), 6)];
        records[2].escort_type = 2;
        records
    }

    /// Ship 200 carries two of weapon 128 and a 30-pixel `shän`, and
    /// its default rockets (outfit 301, rounds of weapon 138).
    fn arsenal() -> Arsenal {
        Arsenal::new(
            &[weapon(128), weapon(138)],
            vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 2,
                    ammo: 0,
                }],
                size: Some(30),
                ..hull(200)
            }],
        )
    }

    fn resolved() -> SpawnTable {
        resolved_by(NovaPersons::default())
    }

    fn resolved_by(rules: NovaPersons) -> SpawnTable {
        resolved_with(rules, &BTreeSet::new())
    }

    /// The table by `rules`, the persons of `fleet` in the player's
    /// fleet.
    fn resolved_with(rules: NovaPersons, fleet: &BTreeSet<PersonId>) -> SpawnTable {
        SpawnTable::resolve(
            &Traffic::default(),
            SystemId(130),
            Some(GovtId(128)),
            &relations(),
            &records(),
            &[
                outfit(300, &[(MORE_SPEED, 100)]),
                outfit(301, &[(crate::combat::armament::MOD_AMMO, 138)]),
            ],
            &arsenal(),
            &rules,
            fleet,
        )
    }

    #[test]
    fn a_table_keeps_the_persons_of_the_players_fleet_fitted_wherever_it_is() {
        let fleet = BTreeSet::from([PersonId(601), PersonId(602), PersonId(700)]);
        let table = resolved_with(NovaPersons::default(), &fleet);
        assert_eq!(
            table.persons.keys().copied().collect::<Vec<_>>(),
            [PersonId(510), PersonId(600), PersonId(601), PersonId(603)],
            "601 for the fleet; 602's ship has no record, and 700 none at all"
        );
        let escort = &table.persons[&PersonId(601)];
        assert!(!escort.linked, "neither linked nor slotted here");
        assert_eq!(escort.record, Traffic::default().persons()[2]);
        assert_eq!(
            escort.kind,
            table.persons[&PersonId(600)].kind,
            "fitted as any other"
        );
        assert!(table.ship_types().contains(&ShipId(200)), "its ship");
        assert!(
            resolved_with(NovaPersons::default(), &BTreeSet::from([PersonId(510)])).persons
                [&PersonId(510)]
                .linked,
            "a fleet person linked here is linked"
        );
    }

    #[test]
    fn a_system_that_cannot_be_read_keeps_only_the_fleets_persons() {
        let catalog = Traffic::default();
        let table = SpawnTable::resolve(
            &catalog,
            SystemId(131),
            None,
            &Governments::default(),
            &records(),
            &[],
            &Arsenal::default(),
            &NovaPersons::default(),
            &BTreeSet::from([PersonId(601)]),
        );
        assert_eq!(
            table.persons.keys().copied().collect::<Vec<_>>(),
            [PersonId(601)]
        );
        assert_eq!(table.ship_types(), [ShipId(200)]);
        assert_eq!((table.avg_ships, table.dudes.len()), (0, 0));
        assert!(table.person_slots.is_empty());
    }

    #[test]
    fn a_table_keeps_the_persons_linked_here_or_slotted_whose_ship_has_a_record() {
        let table = resolved();
        assert_eq!(
            table.persons.keys().copied().collect::<Vec<_>>(),
            [PersonId(510), PersonId(600), PersonId(603)],
            "601 is neither linked nor slotted; 602's ship has no record"
        );
        assert!(table.persons[&PersonId(510)].linked);
        assert!(!table.persons[&PersonId(600)].linked, "slotted only");
        assert!(table.persons[&PersonId(603)].linked, "by the engine's slip");
        assert_eq!(
            table.persons[&PersonId(600)].record,
            Traffic::default().persons()[1]
        );
        assert_eq!(
            table.person_slots,
            [(PersonId(600), 50), (PersonId(605), 20)],
            "every slot naming a person, in order"
        );
        let bible = NovaPersons::from_rulebook(
            &Rulebook::default().with_override(RuleKey::LinkSystSlip, RuleSource::Bible),
        );
        assert_eq!(
            resolved_by(bible)
                .persons
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [PersonId(510), PersonId(600)],
            "no slip by the Bible"
        );
    }

    #[test]
    fn a_tables_persons_fly_their_fitted_ships_and_their_ship_types_are_listed() {
        let table = resolved();
        let ace = &table.persons[&PersonId(510)];
        assert_eq!(ace.kind.stats.shield, 2.0 * f32::from(FAST.shield));
        assert_eq!(
            ace.kind.stats.armor,
            2.0 * f32::from(FAST.armor),
            "by the engine"
        );
        assert!(ace.derelict, "govt 133 is derelict");
        assert_eq!(ace.condition, Condition::Disabled);
        assert_eq!(ace.reserves.shield.now, 0.0);
        let bible = NovaPersons::from_rulebook(
            &Rulebook::default().with_override(RuleKey::ShieldMod, RuleSource::Bible),
        );
        let ace = &resolved_by(bible).persons[&PersonId(510)];
        assert_eq!(ace.kind.stats.armor, f32::from(FAST.armor));
        let plain = &table.persons[&PersonId(600)];
        assert!(!plain.derelict);
        assert_eq!(plain.condition, Condition::Intact);
        assert_eq!(plain.reserves, plain.kind.stats.full());
        assert!(table.ship_types().contains(&ShipId(203)), "510's ship");
    }

    #[test]
    fn a_table_holds_the_systems_dudes_and_fleets() {
        let table = resolved();
        assert_eq!(table.avg_ships, 4);
        assert_eq!(table.dudes, [(DudeId(128), 60), (DudeId(129), 40)]);
        assert_eq!(
            table.dude_records.keys().copied().collect::<Vec<_>>(),
            [DudeId(128)],
            "the unreadable one has no record"
        );
        assert_eq!(
            table.dude_records[&DudeId(128)],
            SpawnDude {
                ai_type: 3,
                govt: Some(GovtId(130)),
                ships: vec![(ShipId(200), 70), (ShipId(999), 30), (ShipId(201), 0)],
                booty: 0x0041,
                info_types: 0x4005,
            },
            "its booty and info types with it"
        );
        assert_eq!(table.dude_fleets, [(FleetId(140), 20)]);
        assert_eq!(
            table.link_fleets,
            BTreeSet::from([FleetId(141), FleetId(142), FleetId(144), FleetId(145)])
        );
        assert_eq!(
            table.fleets.keys().copied().collect::<Vec<_>>(),
            [
                FleetId(140),
                FleetId(141),
                FleetId(142),
                FleetId(144),
                FleetId(145)
            ],
            "fleet 143 links elsewhere and is not named; 146 needs an enemy system"
        );
    }

    #[test]
    fn each_ship_type_named_gets_the_players_stats_and_its_inherent_ai() {
        let table = resolved();
        assert_eq!(
            table.ship_types(),
            [ShipId(200), ShipId(201), ShipId(202), ShipId(203)],
            "ships 998 and 999 have no record, and 203 is person 510's"
        );
        let boosted = ShipStats::new(
            FAST,
            &[crate::fuel::OutfitMod {
                mod_type: MORE_SPEED,
                mod_val: 100,
                count: 1,
            }],
        );
        assert_eq!(table.ships[&ShipId(200)].stats, boosted);
        assert_eq!(table.ships[&ShipId(201)].stats, ShipStats::new(FAST, &[]));
        assert_eq!(
            table
                .ships
                .values()
                .map(|kind| kind.inherent_ai)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn each_ship_type_named_gets_its_escort_class() {
        let table = resolved();
        assert_eq!(
            table
                .ships
                .values()
                .map(|kind| kind.escort_class)
                .collect::<Vec<_>>(),
            [
                EscortClass::Freighter,
                EscortClass::Freighter,
                EscortClass::Warship,
                EscortClass::Fighter
            ],
            "worked out for the traders, and 202's own"
        );
        let heavy = ShipRecord {
            inherent_ai: 3,
            fields: ShipFields { mass: 120, ..FAST },
            ..ship(204, FAST)
        };
        assert_eq!(
            kind(&heavy, &[], &arsenal()).escort_class,
            EscortClass::Medium,
            "by its mass"
        );
    }

    #[test]
    fn a_ship_type_carries_its_appear_on() {
        let gated = ShipRecord {
            appear_on: Test::parse("b7"),
            ..ship(204, FAST)
        };
        assert_eq!(kind(&gated, &[], &arsenal()).appear_on, Test::parse("b7"));
        assert_eq!(ShipKind::default().appear_on, Test::default(), "blank");
    }

    #[test]
    fn each_ship_type_named_is_armed_from_the_arsenal() {
        let table = resolved();
        let armed = &table.ships[&ShipId(200)];
        assert_eq!(armed.hull, arsenal().hull(ShipId(200)));
        assert!(
            (armed.hull.hit_radius - 9.9).abs() < 1e-5,
            "{:?}",
            armed.hull
        );
        assert_eq!(
            armed
                .armament
                .mounts()
                .iter()
                .map(|mount| (mount.spec.id, mount.count))
                .collect::<Vec<_>>(),
            [(WeaponId(128), 2)]
        );
        assert_eq!(armed.rounds, BTreeMap::from([(WeaponId(138), 6)]));
        let unarmed = &table.ships[&ShipId(201)];
        assert_eq!(unarmed.armament, Armament::default());
        assert_eq!(unarmed.hull, HullSpec::default());
        assert!(unarmed.rounds.is_empty());
    }

    #[test]
    fn a_system_that_cannot_be_read_has_no_traffic() {
        let catalog = Traffic::default();
        let table = SpawnTable::resolve(
            &catalog,
            SystemId(131),
            None,
            &Governments::default(),
            &records(),
            &[],
            &Arsenal::default(),
            &NovaPersons::default(),
            &BTreeSet::new(),
        );
        assert_eq!(table, SpawnTable::default());
        assert!(catalog.dudes_asked.borrow().is_empty());
    }

    #[test]
    fn negative_average_ships_is_none() {
        struct Negative;
        impl TrafficCatalog for Negative {
            fn system_traffic(&self, _id: SystemId) -> Option<SystemTraffic> {
                Some(SystemTraffic {
                    dude_types: [UNUSED; 8],
                    avg_ships: -3,
                    persons: Default::default(),
                })
            }
            fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
                None
            }
            fn fleets(&self) -> Vec<FleetRecord> {
                Vec::new()
            }
            fn persons(&self) -> Vec<PersonRecord> {
                Vec::new()
            }
        }
        let table = SpawnTable::resolve(
            &Negative,
            SystemId(130),
            None,
            &Governments::default(),
            &[],
            &[],
            &Arsenal::default(),
            &NovaPersons::default(),
            &BTreeSet::new(),
        );
        assert_eq!(table, SpawnTable::default());
    }
}
