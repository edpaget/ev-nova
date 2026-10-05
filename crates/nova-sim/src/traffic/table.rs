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

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    DudeId, FleetId, FleetRecord, GovtId, OutfitRecord, ShipId, ShipRecord, SystemId,
    TrafficCatalog, WeaponId,
};
use crate::chance::Chance;
use crate::combat::armament::{Armament, Arsenal};
use crate::combat::hull::HullSpec;
use crate::govt::Governments;
use crate::outfitter::outfit_mods;
use crate::pilot::tally;
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
}

/// A ship type the traffic can spawn: how it performs, with its default
/// items, its `InherentAI`, and what it fights with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShipKind {
    /// Its stats.
    pub stats: ShipStats,
    /// Its `InherentAI`, raw.
    pub inherent_ai: i16,
    /// Its hull.
    pub hull: HullSpec,
    /// Its weapons: its stock weapons and those among its default items.
    pub armament: Armament,
    /// The rounds of each ammunition it carries.
    pub rounds: BTreeMap<WeaponId, u32>,
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
    /// The fleets whose `LinkSyst` matches it (their `AppearOn` treated as
    /// met).
    pub link_fleets: BTreeSet<FleetId>,
    /// The fleets it can spawn, from either list.
    pub fleets: BTreeMap<FleetId, FleetRecord>,
    /// Every ship type its `düde`s and fleets name that has a record.
    pub ships: BTreeMap<ShipId, ShipKind>,
}

impl SpawnTable {
    /// System `system`'s table, governed by `system_govt`, read from
    /// `catalog`, its fleets' links matched with the relations in
    /// `govts`, with each ship type's stats from its record in `ships`
    /// and its default items, the `oütf`s from `outfits`, as the player's
    /// are, and its hull and armament from `arsenal`. A system that cannot
    /// be read has no traffic.
    #[must_use]
    pub fn resolve(
        catalog: &(impl TrafficCatalog + ?Sized),
        system: SystemId,
        system_govt: Option<GovtId>,
        govts: &Governments,
        ships: &[ShipRecord],
        outfits: &[OutfitRecord],
        arsenal: &Arsenal,
    ) -> Self {
        let Some(traffic) = catalog.system_traffic(system) else {
            return Self::default();
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
        let wanted: BTreeSet<ShipId> = dude_records
            .values()
            .flat_map(|dude| dude.ships.iter().map(|&(ship, _)| ship))
            .chain(fleets.values().flat_map(|fleet: &FleetRecord| {
                fleet
                    .lead
                    .into_iter()
                    .chain(fleet.escorts.iter().map(|escort| escort.ship))
            }))
            .collect();
        let ships = ships
            .iter()
            .filter(|record| wanted.contains(&record.id))
            .map(|record| (record.id, kind(record, outfits, arsenal)))
            .collect();
        Self {
            avg_ships: u32::try_from(traffic.avg_ships).unwrap_or(0),
            dudes: decoded.dudes,
            dude_records,
            dude_fleets: decoded.fleets,
            link_fleets,
            fleets,
            ships,
        }
    }

    /// The ship types the table can spawn, by ascending ID.
    #[must_use]
    pub fn ship_types(&self) -> Vec<ShipId> {
        self.ships.keys().copied().collect()
    }
}

/// A ship of `record`, carrying its default items, armed from `arsenal`.
fn kind(record: &ShipRecord, outfits: &[OutfitRecord], arsenal: &Arsenal) -> ShipKind {
    let defaults = tally(record.defaults.iter().copied());
    let (armament, rounds) = arsenal.npc(record.id, &defaults, outfits);
    ShipKind {
        stats: ShipStats::new(record.fields, &outfit_mods(&defaults, outfits)),
        inherent_ai: record.inherent_ai,
        hull: arsenal.hull(record.id),
        armament,
        rounds,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::GovtRecord;
    use crate::catalog::{
        DudeRecord, EscortRecord, HullRecord, OutfitId, StockWeapon, SystemTraffic,
    };
    use crate::stats::MORE_SPEED;
    use crate::testkit::{Draws, FAST, govt, hull, outfit, ship, weapon};

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
    /// Düde 128 flies ship 200 and ship 999 (no record). Fleet 140 is led by
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
            })
        }

        fn dude(&self, id: DudeId) -> Option<DudeRecord> {
            self.dudes_asked.borrow_mut().push(id);
            (id == DudeId(128)).then(|| DudeRecord {
                ai_type: 3,
                govt: Some(GovtId(130)),
                ships: vec![(ShipId(200), 70), (ShipId(999), 30), (ShipId(201), -5)],
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
                appear_on: String::new(),
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
        )
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
            }
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
            [ShipId(200), ShipId(201), ShipId(202)],
            "ships 998 and 999 have no record, and 203 is not named"
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
            [1, 2, 3]
        );
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
                })
            }
            fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
                None
            }
            fn fleets(&self) -> Vec<FleetRecord> {
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
        );
        assert_eq!(table, SpawnTable::default());
    }
}
