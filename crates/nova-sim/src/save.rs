//! The save schema: a [`Pilot`] as text and back.
//!
//! A save is our own JSON, not the original's pilot file, so the schema
//! can grow with the game. Every save names its `version`. [`encode`]
//! writes the [`CURRENT`] version; [`decode`] reads any version up to it,
//! upgrading an older save one version at a time, each step a pure change
//! to the JSON that gives the fields it added their defaults, and then
//! reads the current version strictly: a field missing from it is an
//! error, not a default.
//!
//! - Version 1: the name, ship, system, the stellar last landed on, date,
//!   cash, reserves and course.
//! - Version 2: adds the explored systems and the legal records, none in
//!   a version 1 save.
//! - Version 3: adds the cargo held and the planetary events under way,
//!   none in an older save. A good or event whose record no longer
//!   exists is kept as saved.
//! - Version 4: adds the outfits the ship carries. An older save's ship
//!   carried its class's default items, which only the game data knows,
//!   so the upgrade saves them as `null`, "the ship's default items", and
//!   flying the pilot reads them ([`Session::fly`](crate::Session::fly));
//!   the next save lists them. An outfit whose record no longer exists is
//!   kept as saved.
//! - Version 5: adds the escorts, none in an older save.
//! - Version 6: adds each escort's standing order, none in an older save.
//! - Version 7: adds whether each escort is a carried fighter out of its
//!   bay, none in an older save.
//! - Version 8: adds each escort's daily wage, none (not hired) in an
//!   older save.
//! - Version 9: adds the persons gone for good and those holding a
//!   grudge against the player, none in an older save.
//!
//! IDs are saved as their raw numbers, the date as its year, month and
//! day, each reserve as how much the ship has and can hold, each good held
//! as its kind (`commodity` with its number, or `junk` with its ID) and
//! tons, each event as its `öops` ID and days left, each outfit as its
//! `oütf` ID and how many, and each escort as its `shïp` ID, reserves,
//! standing order (`defend`, `attack`, `hold`, `dock`, or `null` for
//! none), whether it is `carried`, and its `wage` in credits a day (`null`
//! for an escort that was not hired). The fighters aboard a bay are its
//! rounds, the ammunition outfits saved with the outfits; the fighters
//! out are the carried escorts. Which carrier a fighter came from is
//! never saved: it docks with the player's bay of its type. The persons
//! gone for good (`gone_persons`) and those holding a grudge (`grudges`)
//! are each a list of `përs` IDs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{DisasterId, GovtId, JunkId, OutfitId, PersonId, ShipId, StellarId, SystemId};
use crate::date::GameDate;
use crate::escort::EscortOrder;
use crate::market::Good;
use crate::pilot::{Escort, Pilot};
use crate::reserves::{Gauge, Reserves};

/// The version [`encode`] writes, and the newest [`decode`] reads.
pub const CURRENT: u64 = 9;

/// Why a save cannot be read. Each message is ready to display.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SaveError {
    /// A save from a newer version of the schema than this one reads.
    /// Worded after `STR#` 140 #34.
    #[error(
        "This pilot file was created with a different version of Nova, and can't be used \
         (it is version {version}, and this version of Nova reads up to {}).",
        CURRENT
    )]
    Newer {
        /// The save's version.
        version: u64,
    },
    /// Text that is not a save, or one whose values cannot be a pilot;
    /// why.
    #[error("This pilot file can't be used: {0}.")]
    Unusable(String),
}

/// A save's date.
#[derive(Serialize, Deserialize)]
struct SavedDate {
    year: i32,
    month: u8,
    day: u8,
}

/// A saved reserve: how much the ship has, and can hold.
#[derive(Serialize, Deserialize)]
struct SavedGauge {
    now: f32,
    max: f32,
}

impl From<Gauge> for SavedGauge {
    fn from(gauge: Gauge) -> Self {
        Self {
            now: gauge.now,
            max: gauge.max,
        }
    }
}

impl From<SavedGauge> for Gauge {
    fn from(saved: SavedGauge) -> Self {
        Self {
            now: saved.now,
            max: saved.max,
        }
    }
}

/// The saved shield, armour and fuel.
#[derive(Serialize, Deserialize)]
struct SavedReserves {
    shield: SavedGauge,
    armor: SavedGauge,
    fuel: SavedGauge,
}

impl From<Reserves> for SavedReserves {
    fn from(reserves: Reserves) -> Self {
        Self {
            shield: reserves.shield.into(),
            armor: reserves.armor.into(),
            fuel: reserves.fuel.into(),
        }
    }
}

impl From<SavedReserves> for Reserves {
    fn from(saved: SavedReserves) -> Self {
        Self {
            shield: saved.shield.into(),
            armor: saved.armor.into(),
            fuel: saved.fuel.into(),
        }
    }
}

/// A saved escort: its ship class, reserves, standing order, whether it
/// is a carried fighter, and the wage it was hired at, if it was.
#[derive(Serialize, Deserialize)]
struct SavedEscort {
    ship: i16,
    reserves: SavedReserves,
    // Without this, serde would read a missing optional field as `None`.
    #[serde(deserialize_with = "Option::deserialize")]
    order: Option<SavedOrder>,
    carried: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    wage: Option<i64>,
}

/// A saved standing order.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SavedOrder {
    Defend,
    Attack,
    Hold,
    Dock,
}

impl From<EscortOrder> for SavedOrder {
    fn from(order: EscortOrder) -> Self {
        match order {
            EscortOrder::Defend => Self::Defend,
            EscortOrder::Attack => Self::Attack,
            EscortOrder::Hold => Self::Hold,
            EscortOrder::Dock => Self::Dock,
        }
    }
}

impl From<SavedOrder> for EscortOrder {
    fn from(saved: SavedOrder) -> Self {
        match saved {
            SavedOrder::Defend => Self::Defend,
            SavedOrder::Attack => Self::Attack,
            SavedOrder::Hold => Self::Hold,
            SavedOrder::Dock => Self::Dock,
        }
    }
}

/// A saved legal record.
#[derive(Serialize, Deserialize)]
struct SavedRecord {
    govt: i16,
    record: i16,
}

/// A saved good: a standard commodity's number, or a `jünk`'s ID.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SavedGood {
    Commodity(u8),
    Junk(i16),
}

/// A good held, and how many tons.
#[derive(Serialize, Deserialize)]
struct SavedCargo {
    good: SavedGood,
    tons: u32,
}

/// An event under way, and the days it has left.
#[derive(Serialize, Deserialize)]
struct SavedEvent {
    disaster: i16,
    days: u16,
}

/// An outfit carried, and how many.
#[derive(Serialize, Deserialize)]
struct SavedOutfit {
    outfit: i16,
    count: u16,
}

/// The current version's save. No field has a default: a missing one is
/// an error.
#[derive(Serialize, Deserialize)]
struct Saved {
    version: u64,
    name: String,
    ship: i16,
    system: i16,
    // Without this, serde would read a missing optional field as `None`.
    #[serde(deserialize_with = "Option::deserialize")]
    stellar: Option<i16>,
    date: SavedDate,
    cash: i64,
    reserves: SavedReserves,
    course: Vec<i16>,
    explored: Vec<i16>,
    legal: Vec<SavedRecord>,
    cargo: Vec<SavedCargo>,
    events: Vec<SavedEvent>,
    /// `None` for the ship's default items, not yet read.
    #[serde(deserialize_with = "Option::deserialize")]
    outfits: Option<Vec<SavedOutfit>>,
    escorts: Vec<SavedEscort>,
    gone_persons: Vec<i16>,
    grudges: Vec<i16>,
}

/// `pilot` as the current version's save: pretty JSON.
#[must_use]
pub fn encode(pilot: &Pilot) -> String {
    let saved = Saved {
        version: CURRENT,
        name: pilot.name.clone(),
        ship: pilot.ship.0,
        system: pilot.system.0,
        stellar: pilot.stellar.map(|stellar| stellar.0),
        date: SavedDate {
            year: pilot.date.year(),
            month: pilot.date.month(),
            day: pilot.date.day(),
        },
        cash: pilot.cash,
        reserves: pilot.reserves.into(),
        course: pilot.course.iter().map(|system| system.0).collect(),
        explored: pilot.explored.iter().map(|system| system.0).collect(),
        legal: pilot
            .legal
            .iter()
            .map(|(govt, &record)| SavedRecord {
                govt: govt.0,
                record,
            })
            .collect(),
        cargo: pilot
            .cargo
            .iter()
            .map(|(&good, &tons)| SavedCargo {
                good: match good {
                    Good::Commodity(n) => SavedGood::Commodity(n),
                    Good::Junk(id) => SavedGood::Junk(id.0),
                },
                tons,
            })
            .collect(),
        events: pilot
            .events
            .iter()
            .map(|(id, &days)| SavedEvent {
                disaster: id.0,
                days,
            })
            .collect(),
        outfits: (!pilot.default_outfits_pending).then(|| {
            pilot
                .outfits
                .iter()
                .map(|(id, &count)| SavedOutfit {
                    outfit: id.0,
                    count,
                })
                .collect()
        }),
        escorts: pilot
            .escorts
            .iter()
            .map(|escort| SavedEscort {
                ship: escort.ship.0,
                reserves: escort.reserves.into(),
                order: escort.order.map(SavedOrder::from),
                carried: escort.carried,
                wage: escort.wage,
            })
            .collect(),
        gone_persons: pilot.gone_persons.iter().map(|person| person.0).collect(),
        grudges: pilot.grudges.iter().map(|person| person.0).collect(),
    };
    serde_json::to_string_pretty(&saved).expect("plain values always serialise")
}

/// The upgrade from each version to the next: `UPGRADES[n - 1]` takes a
/// version `n` save to version `n + 1`.
const UPGRADES: [fn(&mut Value); (CURRENT - 1) as usize] = [
    explored_and_legal,
    cargo_and_events,
    default_outfits,
    escorts,
    escort_orders,
    carried_fighters,
    hired_escorts,
    persons,
];

/// Version 1 to 2: nothing explored, and no legal records.
fn explored_and_legal(save: &mut Value) {
    add_empty(save, &["explored", "legal"]);
}

/// Version 2 to 3: no cargo, and no events under way.
fn cargo_and_events(save: &mut Value) {
    add_empty(save, &["cargo", "events"]);
}

/// Version 3 to 4: the ship's default items, not yet read.
fn default_outfits(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("outfits".to_owned(), Value::Null);
    }
}

/// Version 4 to 5: no escorts.
fn escorts(save: &mut Value) {
    add_empty(save, &["escorts"]);
}

/// Version 5 to 6: no escort has a standing order.
fn escort_orders(save: &mut Value) {
    add_to_escorts(save, "order", &Value::Null);
}

/// Version 6 to 7: no escort is a carried fighter.
fn carried_fighters(save: &mut Value) {
    add_to_escorts(save, "carried", &Value::Bool(false));
}

/// Version 7 to 8: no escort was hired, so none has a wage.
fn hired_escorts(save: &mut Value) {
    add_to_escorts(save, "wage", &Value::Null);
}

/// Version 8 to 9: every person alive, and no grudges.
fn persons(save: &mut Value) {
    add_empty(save, &["gone_persons", "grudges"]);
}

/// Adds `field` to every escort in `save`, as `value`.
fn add_to_escorts(save: &mut Value, field: &str, value: &Value) {
    let Some(escorts) = save.get_mut("escorts").and_then(Value::as_array_mut) else {
        return;
    };
    for escort in escorts {
        if let Some(escort) = escort.as_object_mut() {
            escort.insert(field.to_owned(), value.clone());
        }
    }
}

/// Adds each of `fields` to `save` as an empty list.
fn add_empty(save: &mut Value, fields: &[&str]) {
    if let Some(object) = save.as_object_mut() {
        for field in fields {
            object.insert((*field).to_owned(), Value::Array(Vec::new()));
        }
    }
}

/// The pilot `text` saves, at any version up to [`CURRENT`].
///
/// # Errors
///
/// [`SaveError::Newer`] for a save from a newer version, and
/// [`SaveError::Unusable`] for text that is not a save (not JSON, no
/// version, a missing field or one of the wrong type) or whose values
/// cannot be a pilot's (a date the calendar does not have).
pub fn decode(text: &str) -> Result<Pilot, SaveError> {
    let unusable = |reason: String| SaveError::Unusable(reason);
    let mut save: Value =
        serde_json::from_str(text).map_err(|error| unusable(error.to_string()))?;
    let version = save
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| unusable("no version".to_owned()))?;
    if version > CURRENT {
        return Err(SaveError::Newer { version });
    }
    let from = usize::try_from(version)
        .ok()
        .and_then(|version| version.checked_sub(1))
        .ok_or_else(|| unusable(format!("no such version, {version}")))?;
    for upgrade in &UPGRADES[from..] {
        upgrade(&mut save);
    }
    let saved = Saved::deserialize(save).map_err(|error| unusable(error.to_string()))?;
    let SavedDate { year, month, day } = saved.date;
    let date = GameDate::new(year, month, day)
        .ok_or_else(|| unusable(format!("no such date, {day}/{month}/{year}")))?;
    Ok(Pilot {
        name: saved.name,
        ship: ShipId(saved.ship),
        system: SystemId(saved.system),
        stellar: saved.stellar.map(StellarId),
        date,
        cash: saved.cash,
        reserves: saved.reserves.into(),
        course: saved.course.into_iter().map(SystemId).collect(),
        explored: saved.explored.into_iter().map(SystemId).collect(),
        legal: saved
            .legal
            .into_iter()
            .map(|saved| (GovtId(saved.govt), saved.record))
            .collect(),
        cargo: saved
            .cargo
            .into_iter()
            .map(|saved| {
                let good = match saved.good {
                    SavedGood::Commodity(n) => Good::Commodity(n),
                    SavedGood::Junk(id) => Good::Junk(JunkId(id)),
                };
                (good, saved.tons)
            })
            .collect(),
        events: saved
            .events
            .into_iter()
            .map(|saved| (DisasterId(saved.disaster), saved.days))
            .collect(),
        default_outfits_pending: saved.outfits.is_none(),
        outfits: saved
            .outfits
            .unwrap_or_default()
            .into_iter()
            .map(|saved| (OutfitId(saved.outfit), saved.count))
            .collect(),
        escorts: saved
            .escorts
            .into_iter()
            .map(|saved| Escort {
                ship: ShipId(saved.ship),
                reserves: saved.reserves.into(),
                order: saved.order.map(EscortOrder::from),
                carried: saved.carried,
                wage: saved.wage,
            })
            .collect(),
        gone_persons: saved.gone_persons.into_iter().map(PersonId).collect(),
        grudges: saved.grudges.into_iter().map(PersonId).collect(),
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::catalog::{
        DisasterId, GovtId, JunkId, OutfitId, PersonId, ShipId, StellarId, SystemId,
    };
    use crate::date::GameDate;
    use crate::escort::EscortOrder;
    use crate::market::Good;
    use crate::pilot::Escort;
    use crate::reserves::{Gauge, Reserves};

    /// A pilot with every field away from its default.
    fn seasoned() -> Pilot {
        Pilot {
            name: "Ada Lovelace".to_owned(),
            ship: ShipId(140),
            system: SystemId(131),
            stellar: Some(StellarId(150)),
            date: GameDate::new(1180, 2, 29).expect("a leap day"),
            cash: 9_876_543_210,
            reserves: Reserves {
                shield: Gauge {
                    now: 12.5,
                    max: 300.0,
                },
                armor: Gauge {
                    now: 0.25,
                    max: 45.0,
                },
                fuel: Gauge {
                    now: 201.125,
                    max: 400.0,
                },
            },
            course: vec![SystemId(132), SystemId(133)],
            explored: BTreeSet::from([SystemId(130), SystemId(131), SystemId(200)]),
            legal: BTreeMap::from([(GovtId(128), -40), (GovtId(129), 300)]),
            cargo: BTreeMap::from([(Good::Commodity(2), 7), (Good::Junk(JunkId(146)), 2)]),
            events: BTreeMap::from([(DisasterId(128), 12), (DisasterId(130), 1)]),
            outfits: BTreeMap::from([(OutfitId(256), 3), (OutfitId(128), 1)]),
            default_outfits_pending: false,
            escorts: vec![
                Escort {
                    ship: ShipId(130),
                    reserves: Reserves {
                        shield: Gauge {
                            now: 4.5,
                            max: 50.0,
                        },
                        armor: Gauge {
                            now: 20.0,
                            max: 40.0,
                        },
                        fuel: Gauge {
                            now: 100.0,
                            max: 200.0,
                        },
                    },
                    order: Some(EscortOrder::Defend),
                    carried: false,
                    wage: None,
                },
                Escort {
                    ship: ShipId(141),
                    reserves: Reserves::full(1000.0, 500.0, 400.0),
                    order: None,
                    carried: false,
                    wage: None,
                },
                Escort {
                    ship: ShipId(144),
                    reserves: Reserves::full(60.0, 40.0, 300.0),
                    order: Some(EscortOrder::Dock),
                    carried: true,
                    wage: None,
                },
                Escort {
                    ship: ShipId(128),
                    reserves: Reserves::full(30.0, 45.0, 300.0),
                    order: Some(EscortOrder::Hold),
                    carried: false,
                    wage: Some(100),
                },
            ],
            gone_persons: BTreeSet::from([PersonId(151), PersonId(600)]),
            grudges: BTreeSet::from([PersonId(510)]),
        }
    }

    #[test]
    fn every_field_survives_a_round_trip() {
        let pilot = seasoned();
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_pilot_without_a_stellar_course_or_records_survives_too() {
        let pilot = Pilot {
            stellar: None,
            course: Vec::new(),
            explored: BTreeSet::new(),
            legal: BTreeMap::new(),
            cargo: BTreeMap::new(),
            events: BTreeMap::new(),
            outfits: BTreeMap::new(),
            escorts: Vec::new(),
            gone_persons: BTreeSet::new(),
            grudges: BTreeSet::new(),
            cash: -5,
            ..seasoned()
        };
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_pilot_hurt_in_a_fight_survives_a_round_trip() {
        // A shield driven below none, and worn armour: all a fight leaves
        // on a pilot that a save keeps.
        let pilot = Pilot {
            reserves: Reserves {
                shield: Gauge {
                    now: -3.0,
                    max: 30.0,
                },
                armor: Gauge {
                    now: 6.5,
                    max: 45.0,
                },
                fuel: Gauge {
                    now: 90.0,
                    max: 300.0,
                },
            },
            ..seasoned()
        };
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_save_is_pretty_json_at_the_current_version() {
        let text = encode(&seasoned());
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(CURRENT, 9);
        assert_eq!(value["gone_persons"], serde_json::json!([151, 600]));
        assert_eq!(value["grudges"], serde_json::json!([510]));
        assert_eq!(value["name"], "Ada Lovelace");
        assert_eq!(value["ship"], 140);
        assert_eq!(value["stellar"], 150);
        assert_eq!(
            value["date"],
            serde_json::json!({"year": 1180, "month": 2, "day": 29})
        );
        assert_eq!(value["reserves"]["fuel"]["now"], 201.125);
        assert_eq!(value["explored"], serde_json::json!([130, 131, 200]));
        assert_eq!(
            value["legal"],
            serde_json::json!([{"govt": 128, "record": -40}, {"govt": 129, "record": 300}])
        );
        assert_eq!(
            value["cargo"],
            serde_json::json!([
                {"good": {"commodity": 2}, "tons": 7},
                {"good": {"junk": 146}, "tons": 2}
            ])
        );
        assert_eq!(
            value["events"],
            serde_json::json!([{"disaster": 128, "days": 12}, {"disaster": 130, "days": 1}])
        );
        assert_eq!(
            value["outfits"],
            serde_json::json!([{"outfit": 128, "count": 1}, {"outfit": 256, "count": 3}])
        );
        assert_eq!(
            value["escorts"][0],
            serde_json::json!({
                "ship": 130,
                "reserves": {
                    "shield": {"now": 4.5, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                },
                "order": "defend",
                "carried": false,
                "wage": null
            })
        );
        assert_eq!(value["escorts"][1]["ship"], 141);
        assert_eq!(value["escorts"][1]["order"], serde_json::Value::Null);
        assert_eq!(value["escorts"][2]["ship"], 144);
        assert_eq!(value["escorts"][2]["order"], "dock");
        assert_eq!(value["escorts"][2]["carried"], true);
        assert_eq!(value["escorts"][2]["wage"], serde_json::Value::Null);
        assert_eq!(value["escorts"][3]["ship"], 128);
        assert_eq!(value["escorts"][3]["wage"], 100);
        assert!(text.contains("\n  \"version\": 9"), "{text}");
    }

    /// A version 1 save: before explored systems and legal records.
    const VERSION_1: &str = r#"{
        "version": 1,
        "name": "Old Timer",
        "ship": 128,
        "system": 130,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 1000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [131]
    }"#;

    #[test]
    fn a_version_1_save_loads_with_nothing_explored_and_no_records() {
        let pilot = decode(VERSION_1).expect("loads");
        assert_eq!(pilot.name(), "Old Timer");
        assert_eq!(pilot.ship(), ShipId(128));
        assert_eq!(pilot.system(), SystemId(130));
        assert_eq!(pilot.stellar(), None);
        assert_eq!(pilot.date(), GameDate::new(1177, 6, 24).expect("a date"));
        assert_eq!(pilot.cash(), 1000);
        assert_eq!(
            pilot.reserves().fuel,
            Gauge {
                now: 200.0,
                max: 300.0
            }
        );
        assert_eq!(pilot.course(), [SystemId(131)]);
        assert_eq!(pilot.explored().count(), 0);
        assert_eq!(pilot.legal_records().count(), 0);
    }

    /// A version 2 save: before cargo and events.
    const VERSION_2: &str = r#"{
        "version": 2,
        "name": "Trader",
        "ship": 128,
        "system": 130,
        "stellar": 140,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 2500,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [130],
        "legal": [{"govt": 128, "record": 5}]
    }"#;

    #[test]
    fn a_version_2_save_loads_with_no_cargo_and_no_events() {
        let pilot = decode(VERSION_2).expect("loads");
        assert_eq!(pilot.name(), "Trader");
        assert_eq!(pilot.stellar(), Some(StellarId(140)));
        assert_eq!(pilot.cash(), 2500);
        assert_eq!(pilot.explored().collect::<Vec<_>>(), [SystemId(130)]);
        assert_eq!(pilot.legal_record(GovtId(128)), 5);
        assert_eq!(pilot.cargo().count(), 0);
        assert_eq!(pilot.events().count(), 0);
        let old = decode(VERSION_1).expect("loads");
        assert_eq!((old.cargo().count(), old.events().count()), (0, 0));
    }

    /// A version 3 save: before outfits.
    const VERSION_3: &str = r#"{
        "version": 3,
        "name": "Hauler",
        "ship": 128,
        "system": 130,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 300,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [130],
        "legal": [],
        "cargo": [{"good": {"commodity": 0}, "tons": 4}],
        "events": []
    }"#;

    #[test]
    fn an_older_save_loads_with_the_ships_default_items_pending() {
        for text in [VERSION_3, VERSION_2, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(pilot.default_outfits_pending, "{text}");
            assert_eq!(pilot.outfits().count(), 0, "{text}");
        }
        let pilot = decode(VERSION_3).expect("loads");
        assert_eq!(pilot.held(Good::Commodity(0)), 4);
        // Saved again before it flies, it still says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["outfits"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 4 save: before escorts.
    const VERSION_4: &str = r#"{
        "version": 4,
        "name": "Privateer",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 128, "count": 2}]
    }"#;

    #[test]
    fn a_version_4_save_loads_with_an_empty_fleet() {
        let pilot = decode(VERSION_4).expect("loads");
        assert_eq!(pilot.name(), "Privateer");
        assert_eq!(pilot.ship(), ShipId(129));
        assert_eq!(pilot.owned(OutfitId(128)), 2);
        assert!(!pilot.default_outfits_pending);
        assert_eq!(pilot.escorts(), []);
        // Saved again, it is a current save with no escorts.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 5 save: before standing orders.
    const VERSION_5: &str = r#"{
        "version": 5,
        "name": "Admiral",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                }
            },
            {
                "ship": 141,
                "reserves": {
                    "shield": {"now": 1000.0, "max": 1000.0},
                    "armor": {"now": 500.0, "max": 500.0},
                    "fuel": {"now": 400.0, "max": 400.0}
                }
            }
        ]
    }"#;

    #[test]
    fn a_version_5_saves_escorts_load_with_no_standing_order() {
        let pilot = decode(VERSION_5).expect("loads");
        assert_eq!(pilot.name(), "Admiral");
        assert_eq!(
            pilot.escorts(),
            [
                Escort {
                    ship: ShipId(130),
                    reserves: Reserves {
                        shield: Gauge {
                            now: 5.0,
                            max: 50.0
                        },
                        armor: Gauge {
                            now: 20.0,
                            max: 40.0
                        },
                        fuel: Gauge {
                            now: 100.0,
                            max: 200.0
                        },
                    },
                    order: None,
                    carried: false,
                    wage: None,
                },
                Escort {
                    ship: ShipId(141),
                    reserves: Reserves::full(1000.0, 500.0, 400.0),
                    order: None,
                    carried: false,
                    wage: None,
                },
            ]
        );
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["order"], serde_json::Value::Null);
        assert_eq!(value["escorts"][1]["order"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    /// A version 6 save: before carried fighters.
    const VERSION_6: &str = r#"{
        "version": 6,
        "name": "Commodore",
        "ship": 129,
        "system": 131,
        "stellar": null,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 158, "count": 3}],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                },
                "order": "attack"
            },
            {
                "ship": 141,
                "reserves": {
                    "shield": {"now": 1000.0, "max": 1000.0},
                    "armor": {"now": 500.0, "max": 500.0},
                    "fuel": {"now": 400.0, "max": 400.0}
                },
                "order": null
            }
        ]
    }"#;

    #[test]
    fn a_version_6_saves_escorts_load_as_escorts_not_carried_fighters() {
        let pilot = decode(VERSION_6).expect("loads");
        assert_eq!(pilot.name(), "Commodore");
        assert_eq!(pilot.owned(OutfitId(158)), 3);
        assert_eq!(
            pilot
                .escorts()
                .iter()
                .map(|escort| (escort.ship, escort.order, escort.carried))
                .collect::<Vec<_>>(),
            [
                (ShipId(130), Some(EscortOrder::Attack), false),
                (ShipId(141), None, false)
            ]
        );
        assert_eq!(pilot.escort_count(), 2);
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["carried"], false);
        assert_eq!(value["escorts"][1]["carried"], false);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1] {
            assert!(decode(text).is_ok(), "{text}");
        }
        let fleet = decode(VERSION_5).expect("loads");
        assert!(fleet.escorts().iter().all(|escort| !escort.carried));
    }

    /// A version 7 save: before hired escorts' wages.
    const VERSION_7: &str = r#"{
        "version": 7,
        "name": "Fleet Admiral",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 158, "count": 2}],
        "escorts": [
            {
                "ship": 130,
                "reserves": {
                    "shield": {"now": 5.0, "max": 50.0},
                    "armor": {"now": 20.0, "max": 40.0},
                    "fuel": {"now": 100.0, "max": 200.0}
                },
                "order": "hold",
                "carried": false
            },
            {
                "ship": 144,
                "reserves": {
                    "shield": {"now": 60.0, "max": 60.0},
                    "armor": {"now": 40.0, "max": 40.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": true
            }
        ]
    }"#;

    #[test]
    fn a_version_7_saves_escorts_load_as_not_hired() {
        let pilot = decode(VERSION_7).expect("loads");
        assert_eq!(pilot.name(), "Fleet Admiral");
        assert_eq!(
            pilot
                .escorts()
                .iter()
                .map(|escort| (escort.ship, escort.order, escort.carried, escort.wage))
                .collect::<Vec<_>>(),
            [
                (ShipId(130), Some(EscortOrder::Hold), false, None),
                (ShipId(144), None, true, None)
            ]
        );
        assert!(pilot.escorts().iter().all(|escort| !escort.hired()));
        // Saved again, it is a current save whose escorts say so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["wage"], serde_json::Value::Null);
        assert_eq!(value["escorts"][1]["wage"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert!(
                pilot.escorts().iter().all(|escort| escort.wage.is_none()),
                "{text}"
            );
        }
    }

    /// A version 8 save: before the persons gone and the grudges.
    const VERSION_8: &str = r#"{
        "version": 8,
        "name": "Mercenary",
        "ship": 129,
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [],
        "escorts": [
            {
                "ship": 128,
                "reserves": {
                    "shield": {"now": 30.0, "max": 30.0},
                    "armor": {"now": 45.0, "max": 45.0},
                    "fuel": {"now": 300.0, "max": 300.0}
                },
                "order": null,
                "carried": false,
                "wage": 100
            }
        ]
    }"#;

    #[test]
    fn a_version_8_save_loads_with_every_person_alive_and_no_grudge() {
        let pilot = decode(VERSION_8).expect("loads");
        assert_eq!(pilot.name(), "Mercenary");
        assert_eq!(pilot.escorts()[0].wage, Some(100));
        assert_eq!(pilot.gone_persons().count(), 0);
        assert_eq!(pilot.grudges().count(), 0);
        // Saved again, it is a current save that says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], 9);
        assert_eq!(value["gone_persons"], serde_json::json!([]));
        assert_eq!(value["grudges"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(
                (pilot.gone_persons().count(), pilot.grudges().count()),
                (0, 0),
                "{text}"
            );
        }
    }

    #[test]
    fn a_current_save_missing_its_persons_or_grudges_or_of_no_list_is_unusable() {
        for field in ["gone_persons", "grudges"] {
            let mut value: serde_json::Value =
                serde_json::from_str(&encode(&seasoned())).expect("JSON");
            if let Some(object) = value.as_object_mut() {
                object.remove(field);
            }
            assert!(unusable(&value.to_string()).contains(field), "{field}");
        }
        let text =
            encode(&seasoned()).replace("\"grudges\": [\n    510\n  ]", "\"grudges\": \"lots\"");
        assert!(text.contains("lots"), "{text}");
        assert!(unusable(&text).contains("lots"), "{}", unusable(&text));
    }

    #[test]
    fn every_wage_survives_a_round_trip() {
        for wage in [None, Some(0), Some(1), Some(175), Some(i64::MAX)] {
            let mut pilot = seasoned();
            pilot.escorts[1].wage = wage;
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{wage:?}");
        }
    }

    #[test]
    fn a_current_escort_missing_its_wage_or_of_no_number_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(escort) = value["escorts"][3].as_object_mut() {
            escort.remove("wage");
        }
        assert!(unusable(&value.to_string()).contains("wage"));
        let text = encode(&seasoned()).replace("\"wage\": 100", "\"wage\": \"lots\"");
        assert!(text.contains("lots"));
        assert!(unusable(&text).contains("lots"), "{}", unusable(&text));
    }

    #[test]
    fn a_current_escort_missing_whether_it_is_carried_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(escort) = value["escorts"][2].as_object_mut() {
            escort.remove("carried");
        }
        assert!(unusable(&value.to_string()).contains("carried"));
    }

    #[test]
    fn every_standing_order_survives_a_round_trip() {
        for order in [
            None,
            Some(EscortOrder::Defend),
            Some(EscortOrder::Attack),
            Some(EscortOrder::Hold),
            Some(EscortOrder::Dock),
        ] {
            let mut pilot = seasoned();
            pilot.escorts[1].order = order;
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{order:?}");
        }
        let mut pilot = seasoned();
        pilot.escorts[0].order = Some(EscortOrder::Attack);
        pilot.escorts[1].order = Some(EscortOrder::Hold);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["escorts"][0]["order"], "attack");
        assert_eq!(value["escorts"][1]["order"], "hold");
    }

    #[test]
    fn a_current_escort_missing_its_order_or_of_no_known_order_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(escort) = value["escorts"][1].as_object_mut() {
            escort.remove("order");
        }
        assert!(unusable(&value.to_string()).contains("order"));
        let text = encode(&seasoned()).replace("\"defend\"", "\"charge\"");
        assert!(unusable(&text).contains("charge"), "{}", unusable(&text));
    }

    #[test]
    fn an_older_save_loads_with_an_empty_fleet() {
        for text in [VERSION_3, VERSION_2, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.escorts(), [], "{text}");
        }
    }

    #[test]
    fn a_current_save_lists_its_outfits_even_when_none() {
        assert!(!seasoned().default_outfits_pending);
        let bare = Pilot {
            outfits: BTreeMap::new(),
            ..seasoned()
        };
        let value: serde_json::Value = serde_json::from_str(&encode(&bare)).expect("JSON");
        assert_eq!(value["outfits"], serde_json::json!([]));
        let decoded = decode(&encode(&bare)).expect("loads");
        assert!(!decoded.default_outfits_pending);
    }

    #[test]
    fn a_saved_outfit_whose_record_is_gone_is_kept() {
        let mut pilot = seasoned();
        pilot.outfits.insert(OutfitId(-3), 9);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_save_from_a_newer_version_is_refused() {
        let newer = encode(&seasoned()).replace("\"version\": 9", "\"version\": 10");
        assert_eq!(decode(&newer), Err(SaveError::Newer { version: 10 }));
        assert_eq!(
            SaveError::Newer { version: 10 }.to_string(),
            "This pilot file was created with a different version of Nova, and can't be used \
             (it is version 10, and this version of Nova reads up to 9)."
        );
    }

    fn unusable(text: &str) -> String {
        match decode(text) {
            Err(SaveError::Unusable(reason)) => reason,
            other => panic!("{text}: {other:?}"),
        }
    }

    #[test]
    fn text_that_is_not_a_save_is_unusable() {
        for text in [
            "",
            "not json",
            "[1, 2]",
            "{}",
            r#"{"version": "2"}"#,
            r#"{"version": -1}"#,
        ] {
            assert!(!unusable(text).is_empty(), "{text}");
        }
        assert_eq!(unusable(r#"{"version": 0}"#), "no such version, 0");
        assert_eq!(unusable("{}"), "no version");
    }

    #[test]
    fn a_current_save_missing_a_field_is_unusable() {
        let full: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        let serde_json::Value::Object(fields) = full else {
            panic!("an object");
        };
        for name in fields.keys().filter(|name| *name != "version") {
            let mut missing = fields.clone();
            missing.remove(name);
            let text = serde_json::Value::Object(missing).to_string();
            let reason = unusable(&text);
            assert!(reason.contains(name.as_str()), "{name}: {reason}");
        }
    }

    #[test]
    fn a_saved_good_or_event_whose_record_is_gone_is_kept() {
        let mut pilot = seasoned();
        pilot.cargo.insert(Good::Commodity(9), 4);
        pilot.cargo.insert(Good::Junk(JunkId(-7)), 1);
        pilot.events.insert(DisasterId(999), 3);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_saved_good_of_no_known_kind_is_unusable() {
        let text = encode(&seasoned()).replace("\"commodity\": 2", "\"spice\": 2");
        assert!(unusable(&text).contains("spice"), "{}", unusable(&text));
    }

    #[test]
    fn a_version_1_save_missing_a_field_is_unusable() {
        let text = VERSION_1.replace(r#""course": [131]"#, r#""cours": [131]"#);
        assert!(unusable(&text).contains("course"));
    }

    #[test]
    fn an_impossible_date_is_unusable() {
        let text = VERSION_1.replace(r#""day": 24"#, r#""day": 31"#);
        assert_eq!(unusable(&text), "no such date, 31/6/1177");
        let text = VERSION_1.replace(r#""month": 6"#, r#""month": 13"#);
        assert_eq!(unusable(&text), "no such date, 24/13/1177");
    }

    #[test]
    fn an_unusable_save_reads_as_a_sentence() {
        assert_eq!(
            SaveError::Unusable("no version".to_owned()).to_string(),
            "This pilot file can't be used: no version."
        );
    }
}
