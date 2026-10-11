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
//! - Version 10: adds the person each escort is, none in an older save.
//! - Version 11: adds the control bits, every one clear in an older save,
//!   and the player's gender, male in an older save.
//! - Version 12: adds the ship's paint, unpainted in an older save.
//! - Version 13: adds the ship's name. An older save's ship was never
//!   named, so the upgrade saves `null`, and flying the pilot names it
//!   after its class ([`Session::fly`](crate::Session::fly)). The next
//!   save keeps that name.
//! - Version 14: a ship's stock weapons and their `AmmoLoad` are outfits
//!   it owns. An older save's ship mounted its class's stock weapons
//!   beside its outfits, which only the game data knows, so the upgrade
//!   marks them not fitted (`stock_weapons_fitted`), and flying the pilot
//!   adds them to the outfits saved
//!   ([`Session::fly`](crate::Session::fly)); the next save lists them.
//! - Version 15: adds the stellar each event under way is at, kept from
//!   the day it starts, as the original keeps and saves it. An older
//!   save's events were all at their record's `Stellar`, which only the
//!   game data knows, so the upgrade saves `null`, read as the record's
//!   own stellar. The next save keeps `null`.
//! - Version 16: adds the once-only story subscriptions that have fired,
//!   none in an older save.
//! - Version 17: adds the galaxy overlay, what play has changed of the
//!   galaxy, nothing in an older save.
//! - Version 18: a destroyed stellar holds its regeneration countdown. A
//!   version 17 save held only whether it was destroyed, which no version
//!   17 writer ever set, so the upgrade gives a destroyed one 0 days left:
//!   due to regenerate on its next day, as a `Y` with a `DeadTime` of 0
//!   leaves it.
//! - Version 19: adds the combat rating. No older save kept one, so the
//!   upgrade gives 0, the original's value for a pilot with no `chär`
//!   (@0x76f70).
//! - Version 20: adds the active missions, none in an older save.
//! - Version 21: adds the legal record in each system (`system_legal`,
//!   each its `system` and `record`, or `null` until derived). An older
//!   save kept only the record with each government, so the upgrade
//!   saves `null`, and flying the pilot derives each system's
//!   ([`Session::fly`](crate::Session::fly)).
//! - Version 22: adds each active mission's special ships (`ships`). An
//!   older save kept none of their state, so the upgrade gives each
//!   mission none left, nowhere, with no timer and no name: an old save's
//!   mission spawns no ship, and its `<SN>` reads empty.
//!
//! IDs are saved as their raw numbers, the ship's name (`ship_name`) as
//! its text, or `null` for a ship not yet named, the date as its year, month and
//! day, each reserve as how much the ship has and can hold, each good held
//! as its kind (`commodity` with its number, or `junk` with its ID) and
//! tons, each event as its `öops` ID, days left and the `spöb` ID it is
//! at (`null` for its record's own), each outfit as its
//! `oütf` ID and how many, and each escort as its `shïp` ID, reserves,
//! standing order (`defend`, `attack`, `hold`, `dock`, or `null` for
//! none), whether it is `carried`, its `wage` in credits a day (`null`
//! for an escort that was not hired), and the `person` it is (a `përs`
//! ID, or `null`; one whose record is gone is kept as saved). The
//! fighters aboard a bay are its
//! rounds, the ammunition outfits saved with the outfits; the fighters
//! out are the carried escorts. Which carrier a fighter came from is
//! never saved: it docks with the player's bay of its type. The persons
//! gone for good (`gone_persons`) and those holding a grudge (`grudges`)
//! are each a list of `përs` IDs. The control bits (`bits`) are the
//! numbers of the bits set, ascending, and the `gender` is `male` or
//! `female`. The combat rating (`combat_rating`) is its number. The `paint` is `null` when unpainted, or its red, green and
//! blue, each 0 to 31 (`{"r": 31, "g": 0, "b": 4}`). The once-only story
//! subscriptions fired (`story_fired`) are each its consumer
//! (`mission_ships`, `missions`, `crons`, `nebulae`, `plugins` or
//! `fixture`), record and hook (`{"consumer": "missions", "record": 3,
//! "hook": 1}`), listed by consumer, record and hook. The galaxy overlay
//! (`galaxy`) is three lists, each by ascending ID: `systems`, each its
//! `sÿst` ID, whether it is `visible` and its `govt`; `stellars`, each its
//! `spöb` ID, its `govt` and, while it is `destroyed`, its regeneration
//! countdown (`{"days_left": 3}`), else `null`; and `nebulae`, each its
//! `nëbu` ID and whether it is `active`. A `visible` or `govt` of `null`
//! changes nothing; a `govt` override is `{"govt": id}`, or `{"govt":
//! null}` for independent (`{"system": 131, "visible": false, "govt":
//! {"govt": 128}}`).
//!
//! The active missions (`missions`) are listed by slot, the free slots
//! left out: each its `slot` (0 to 15), its `mïsn` ID (`mission`), its
//! `destination` and `return`, each a `spöb` ID with the `sÿst` it belongs
//! to (`{"stellar": 150, "system": 132}`, the system `null` when no
//! visible system listed it) or `null` for none, its `cargo_type` (raw, or
//! `null`), `cargo_tons` and `pay`, its `days_left` and `deadline` date,
//! each `null` without a `TimeLimit`, whether its destination is
//! `reached`, its `goals_met` and its cargo `aboard`, and how it
//! `failed`: `null`, `quick` (at once, its `OnFailure` run) or
//! `operator` (by `F`), and its special `ships`: their `system` (`{"at":
//! 130}`, `"follows_player"`, or `null` for nowhere), how many are `left`
//! and have `arrived`, their `appear` timer (-1 when it does not run),
//! and their drawn `name` and `subtitle`. A slot past 15 is unusable.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{
    DisasterId, GovtId, JunkId, MissionId, NebulaId, OutfitId, PersonId, ShipId, StellarId,
    SystemId,
};
use crate::control::Bit;
use crate::date::GameDate;
use crate::escort::EscortOrder;
use crate::market::{ActiveEvent, Good};
use crate::mission::{ActiveMission, ActiveMissions, FailedBy, ShipSystem, SpecialShips, Stop};
use crate::outfit_effects::Rgb15;
use crate::pilot::{Escort, Gender, Pilot};
use crate::reserves::{Gauge, Reserves};
use crate::story::{Consumer, Countdown, GalaxyOverlay, SubscriptionId};

/// The version [`encode`] writes, and the newest [`decode`] reads.
pub const CURRENT: u64 = 22;

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
#[derive(Clone, Copy, Serialize, Deserialize)]
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
/// is a carried fighter, the wage it was hired at, if it was, and the
/// person it is, if any.
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
    #[serde(deserialize_with = "Option::deserialize")]
    person: Option<i16>,
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

/// A saved gender.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SavedGender {
    Male,
    Female,
}

impl From<Gender> for SavedGender {
    fn from(gender: Gender) -> Self {
        match gender {
            Gender::Male => Self::Male,
            Gender::Female => Self::Female,
        }
    }
}

impl From<SavedGender> for Gender {
    fn from(saved: SavedGender) -> Self {
        match saved {
            SavedGender::Male => Self::Male,
            SavedGender::Female => Self::Female,
        }
    }
}

/// A saved paint.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct SavedPaint {
    r: u8,
    g: u8,
    b: u8,
}

impl From<Rgb15> for SavedPaint {
    fn from(paint: Rgb15) -> Self {
        Self {
            r: paint.r,
            g: paint.g,
            b: paint.b,
        }
    }
}

impl From<SavedPaint> for Rgb15 {
    fn from(saved: SavedPaint) -> Self {
        Self {
            r: saved.r,
            g: saved.g,
            b: saved.b,
        }
    }
}

/// A saved legal record.
#[derive(Serialize, Deserialize)]
struct SavedRecord {
    govt: i16,
    record: i16,
}

/// A saved legal record in a system.
#[derive(Serialize, Deserialize)]
struct SavedSystemRecord {
    system: i16,
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

/// An event under way, the days it has left, and the stellar it is at.
#[derive(Serialize, Deserialize)]
struct SavedEvent {
    disaster: i16,
    days: u16,
    /// `None` for its record's own `Stellar`.
    #[serde(deserialize_with = "Option::deserialize")]
    stellar: Option<i16>,
}

/// A saved story consumer.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedConsumer {
    MissionShips,
    Missions,
    Crons,
    Nebulae,
    Plugins,
    Fixture,
}

impl From<Consumer> for SavedConsumer {
    fn from(consumer: Consumer) -> Self {
        match consumer {
            Consumer::MissionShips => Self::MissionShips,
            Consumer::Missions => Self::Missions,
            Consumer::Crons => Self::Crons,
            Consumer::Nebulae => Self::Nebulae,
            Consumer::Plugins => Self::Plugins,
            Consumer::Fixture => Self::Fixture,
        }
    }
}

impl From<SavedConsumer> for Consumer {
    fn from(saved: SavedConsumer) -> Self {
        match saved {
            SavedConsumer::MissionShips => Self::MissionShips,
            SavedConsumer::Missions => Self::Missions,
            SavedConsumer::Crons => Self::Crons,
            SavedConsumer::Nebulae => Self::Nebulae,
            SavedConsumer::Plugins => Self::Plugins,
            SavedConsumer::Fixture => Self::Fixture,
        }
    }
}

/// A saved once-only story subscription that has fired: its id.
#[derive(Serialize, Deserialize)]
struct SavedFired {
    consumer: SavedConsumer,
    record: i16,
    hook: u8,
}

impl From<SubscriptionId> for SavedFired {
    fn from(id: SubscriptionId) -> Self {
        Self {
            consumer: id.consumer.into(),
            record: id.record,
            hook: id.hook,
        }
    }
}

impl From<SavedFired> for SubscriptionId {
    fn from(saved: SavedFired) -> Self {
        Self {
            consumer: saved.consumer.into(),
            record: saved.record,
            hook: saved.hook,
        }
    }
}

/// A saved government override: `None` for independent.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct SavedGovt {
    #[serde(deserialize_with = "Option::deserialize")]
    govt: Option<i16>,
}

impl From<Option<GovtId>> for SavedGovt {
    fn from(govt: Option<GovtId>) -> Self {
        Self {
            govt: govt.map(|govt| govt.0),
        }
    }
}

impl From<SavedGovt> for Option<GovtId> {
    fn from(saved: SavedGovt) -> Self {
        saved.govt.map(GovtId)
    }
}

/// What the overlay changes of a system.
#[derive(Serialize, Deserialize)]
struct SavedSystem {
    system: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    visible: Option<bool>,
    #[serde(deserialize_with = "Option::deserialize")]
    govt: Option<SavedGovt>,
}

/// A destroyed stellar's regeneration countdown.
#[derive(Serialize, Deserialize)]
struct SavedCountdown {
    days_left: u16,
}

/// What the overlay changes of a stellar: `destroyed` is `None` while it
/// is not.
#[derive(Serialize, Deserialize)]
struct SavedStellar {
    stellar: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    govt: Option<SavedGovt>,
    #[serde(deserialize_with = "Option::deserialize")]
    destroyed: Option<SavedCountdown>,
}

/// Whether a nebula is active, as the overlay says.
#[derive(Serialize, Deserialize)]
struct SavedNebula {
    nebula: i16,
    active: bool,
}

/// The saved galaxy overlay, each list by ascending ID.
#[derive(Serialize, Deserialize)]
struct SavedGalaxy {
    systems: Vec<SavedSystem>,
    stellars: Vec<SavedStellar>,
    nebulae: Vec<SavedNebula>,
}

impl From<&GalaxyOverlay> for SavedGalaxy {
    fn from(overlay: &GalaxyOverlay) -> Self {
        Self {
            systems: overlay
                .systems()
                .map(|(id, change)| SavedSystem {
                    system: id.0,
                    visible: change.visible,
                    govt: change.govt.map(SavedGovt::from),
                })
                .collect(),
            stellars: overlay
                .stellars()
                .map(|(id, change)| SavedStellar {
                    stellar: id.0,
                    govt: change.govt.map(SavedGovt::from),
                    destroyed: change.destroyed.map(|countdown| SavedCountdown {
                        days_left: countdown.days_left,
                    }),
                })
                .collect(),
            nebulae: overlay
                .nebulae()
                .map(|(id, active)| SavedNebula {
                    nebula: id.0,
                    active,
                })
                .collect(),
        }
    }
}

impl From<SavedGalaxy> for GalaxyOverlay {
    fn from(saved: SavedGalaxy) -> Self {
        let mut overlay = Self::default();
        for system in saved.systems {
            let id = SystemId(system.system);
            overlay.set_visible(id, system.visible);
            overlay.set_govt(id, system.govt.map(Option::from));
        }
        for stellar in saved.stellars {
            let id = StellarId(stellar.stellar);
            overlay.set_stellar_govt(id, stellar.govt.map(Option::from));
            let destroyed = stellar.destroyed.map(|saved| Countdown {
                days_left: saved.days_left,
            });
            overlay.set_destroyed(id, destroyed);
        }
        for nebula in saved.nebulae {
            overlay.set_nebula_active(NebulaId(nebula.nebula), Some(nebula.active));
        }
        overlay
    }
}

/// An outfit carried, and how many.
#[derive(Serialize, Deserialize)]
struct SavedOutfit {
    outfit: i16,
    count: u16,
}

/// A saved stellar a mission names, with the system it belongs to.
#[derive(Serialize, Deserialize)]
struct SavedStop {
    stellar: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    system: Option<i16>,
}

impl From<Stop> for SavedStop {
    fn from(stop: Stop) -> Self {
        Self {
            stellar: stop.stellar.0,
            system: stop.system.map(|system| system.0),
        }
    }
}

impl From<SavedStop> for Stop {
    fn from(saved: SavedStop) -> Self {
        Self {
            stellar: StellarId(saved.stellar),
            system: saved.system.map(SystemId),
        }
    }
}

/// How a saved mission failed.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedFailure {
    Quick,
    Operator,
}

impl From<FailedBy> for SavedFailure {
    fn from(failed: FailedBy) -> Self {
        match failed {
            FailedBy::Quick => Self::Quick,
            FailedBy::Operator => Self::Operator,
        }
    }
}

impl From<SavedFailure> for FailedBy {
    fn from(saved: SavedFailure) -> Self {
        match saved {
            SavedFailure::Quick => Self::Quick,
            SavedFailure::Operator => Self::Operator,
        }
    }
}

/// Where a saved mission's special ships are; `null` for nowhere.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SavedShipSystem {
    At(i16),
    FollowsPlayer,
}

/// A saved mission's special ships.
#[derive(Serialize, Deserialize)]
struct SavedShips {
    #[serde(deserialize_with = "Option::deserialize")]
    system: Option<SavedShipSystem>,
    left: i16,
    arrived: i16,
    appear: i16,
    name: String,
    subtitle: String,
}

impl From<&SpecialShips> for SavedShips {
    fn from(ships: &SpecialShips) -> Self {
        Self {
            system: match ships.system {
                ShipSystem::At(system) => Some(SavedShipSystem::At(system.0)),
                ShipSystem::FollowsPlayer => Some(SavedShipSystem::FollowsPlayer),
                ShipSystem::Nowhere => None,
            },
            left: ships.left,
            arrived: ships.arrived,
            appear: ships.appear,
            name: ships.name.clone(),
            subtitle: ships.subtitle.clone(),
        }
    }
}

impl From<SavedShips> for SpecialShips {
    fn from(saved: SavedShips) -> Self {
        Self {
            system: match saved.system {
                Some(SavedShipSystem::At(system)) => ShipSystem::At(SystemId(system)),
                Some(SavedShipSystem::FollowsPlayer) => ShipSystem::FollowsPlayer,
                None => ShipSystem::Nowhere,
            },
            left: saved.left,
            arrived: saved.arrived,
            appear: saved.appear,
            name: saved.name,
            subtitle: saved.subtitle,
        }
    }
}

/// A saved active mission, in its slot.
#[derive(Serialize, Deserialize)]
struct SavedMission {
    slot: u8,
    mission: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    destination: Option<SavedStop>,
    #[serde(rename = "return", deserialize_with = "Option::deserialize")]
    return_to: Option<SavedStop>,
    #[serde(deserialize_with = "Option::deserialize")]
    cargo_type: Option<i16>,
    cargo_tons: i64,
    pay: i64,
    #[serde(deserialize_with = "Option::deserialize")]
    days_left: Option<u32>,
    #[serde(deserialize_with = "Option::deserialize")]
    deadline: Option<SavedDate>,
    reached: bool,
    goals_met: bool,
    aboard: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    failed: Option<SavedFailure>,
    ships: SavedShips,
}

impl SavedMission {
    /// `active`, in `slot`.
    fn new(slot: u8, active: &ActiveMission) -> Self {
        Self {
            slot,
            mission: active.mission.0,
            destination: active.destination.map(SavedStop::from),
            return_to: active.return_to.map(SavedStop::from),
            cargo_type: active.cargo_type,
            cargo_tons: active.cargo_tons,
            pay: active.pay,
            days_left: active.days_left,
            deadline: active.deadline.map(SavedDate::from),
            reached: active.reached,
            goals_met: active.goals_met,
            aboard: active.aboard,
            failed: active.failed.map(SavedFailure::from),
            ships: SavedShips::from(&active.ships),
        }
    }
}

impl From<GameDate> for SavedDate {
    fn from(date: GameDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
            day: date.day(),
        }
    }
}

/// The date `saved` names, or why it names none.
fn date_of(saved: SavedDate) -> Result<GameDate, SaveError> {
    let SavedDate { year, month, day } = saved;
    GameDate::new(year, month, day)
        .ok_or_else(|| SaveError::Unusable(format!("no such date, {day}/{month}/{year}")))
}

/// The active missions `saved` lists, each in its slot, or why they
/// cannot be.
fn missions_of(saved: Vec<SavedMission>) -> Result<ActiveMissions, SaveError> {
    let mut missions = ActiveMissions::default();
    for saved in saved {
        let active = ActiveMission {
            mission: MissionId(saved.mission),
            destination: saved.destination.map(Stop::from),
            return_to: saved.return_to.map(Stop::from),
            cargo_type: saved.cargo_type,
            cargo_tons: saved.cargo_tons,
            pay: saved.pay,
            days_left: saved.days_left,
            deadline: saved.deadline.map(date_of).transpose()?,
            reached: saved.reached,
            goals_met: saved.goals_met,
            aboard: saved.aboard,
            failed: saved.failed.map(FailedBy::from),
            ships: SpecialShips::from(saved.ships),
        };
        if !missions.put(saved.slot, active) {
            return Err(SaveError::Unusable(format!(
                "no such mission slot, {}",
                saved.slot
            )));
        }
    }
    Ok(missions)
}

/// The current version's save. No field has a default: a missing one is
/// an error.
#[derive(Serialize, Deserialize)]
struct Saved {
    version: u64,
    name: String,
    ship: i16,
    /// `None` for a ship not yet named.
    #[serde(deserialize_with = "Option::deserialize")]
    ship_name: Option<String>,
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
    /// `None` until the records in each system are derived.
    #[serde(deserialize_with = "Option::deserialize")]
    system_legal: Option<Vec<SavedSystemRecord>>,
    combat_rating: i64,
    cargo: Vec<SavedCargo>,
    events: Vec<SavedEvent>,
    /// `None` for the ship's default items, not yet read.
    #[serde(deserialize_with = "Option::deserialize")]
    outfits: Option<Vec<SavedOutfit>>,
    /// Whether the ship's stock weapons are among `outfits`: not in a
    /// save from before they were outfits.
    stock_weapons_fitted: bool,
    escorts: Vec<SavedEscort>,
    gone_persons: Vec<i16>,
    grudges: Vec<i16>,
    bits: Vec<Bit>,
    gender: SavedGender,
    /// `None` when unpainted.
    #[serde(deserialize_with = "Option::deserialize")]
    paint: Option<SavedPaint>,
    story_fired: Vec<SavedFired>,
    galaxy: SavedGalaxy,
    missions: Vec<SavedMission>,
}

/// `pilot` as the current version's save: pretty JSON.
#[must_use]
pub fn encode(pilot: &Pilot) -> String {
    let saved = Saved {
        version: CURRENT,
        name: pilot.name.clone(),
        ship: pilot.ship.0,
        ship_name: pilot.ship_name.clone(),
        system: pilot.system.0,
        stellar: pilot.stellar.map(|stellar| stellar.0),
        date: pilot.date.into(),
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
        system_legal: pilot.system_legal.as_ref().map(|legal| {
            legal
                .iter()
                .map(|(system, &record)| SavedSystemRecord {
                    system: system.0,
                    record,
                })
                .collect()
        }),
        combat_rating: pilot.combat_rating,
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
            .map(|(id, active)| SavedEvent {
                disaster: id.0,
                days: active.days,
                stellar: active.stellar.map(|stellar| stellar.0),
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
        stock_weapons_fitted: !pilot.stock_weapons_pending,
        escorts: pilot
            .escorts
            .iter()
            .map(|escort| SavedEscort {
                ship: escort.ship.0,
                reserves: escort.reserves.into(),
                order: escort.order.map(SavedOrder::from),
                carried: escort.carried,
                wage: escort.wage,
                person: escort.person.map(|person| person.0),
            })
            .collect(),
        gone_persons: pilot.gone_persons.iter().map(|person| person.0).collect(),
        grudges: pilot.grudges.iter().map(|person| person.0).collect(),
        bits: pilot.bits.iter().collect(),
        gender: pilot.gender.into(),
        paint: pilot.paint.map(SavedPaint::from),
        story_fired: pilot
            .story_fired
            .iter()
            .copied()
            .map(SavedFired::from)
            .collect(),
        galaxy: (&pilot.galaxy).into(),
        missions: pilot
            .missions
            .iter()
            .map(|(slot, active)| SavedMission::new(slot, active))
            .collect(),
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
    person_escorts,
    bits_and_gender,
    paint,
    ship_names,
    stock_weapons,
    event_stellars,
    story_fired,
    galaxy,
    countdowns,
    combat_rating,
    active_missions,
    system_records,
    mission_ships,
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
    add_to_each(save, "escorts", "order", &Value::Null);
}

/// Version 6 to 7: no escort is a carried fighter.
fn carried_fighters(save: &mut Value) {
    add_to_each(save, "escorts", "carried", &Value::Bool(false));
}

/// Version 7 to 8: no escort was hired, so none has a wage.
fn hired_escorts(save: &mut Value) {
    add_to_each(save, "escorts", "wage", &Value::Null);
}

/// Version 8 to 9: every person alive, and no grudges.
fn persons(save: &mut Value) {
    add_empty(save, &["gone_persons", "grudges"]);
}

/// Version 9 to 10: no escort is a person.
fn person_escorts(save: &mut Value) {
    add_to_each(save, "escorts", "person", &Value::Null);
}

/// Version 10 to 11: the ship's stock weapons, not yet fitted as outfits.
fn stock_weapons(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("stock_weapons_fitted".to_owned(), Value::Bool(false));
    }
}

/// Version 10 to 11: every control bit clear, and the player male.
fn bits_and_gender(save: &mut Value) {
    add_empty(save, &["bits"]);
    if let Some(object) = save.as_object_mut() {
        object.insert("gender".to_owned(), Value::from("male"));
    }
}

/// Version 11 to 12: the ship unpainted.
fn paint(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("paint".to_owned(), Value::Null);
    }
}

/// Version 12 to 13: the ship not yet named.
fn ship_names(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("ship_name".to_owned(), Value::Null);
    }
}

/// Version 14 to 15: every event at its record's own stellar.
fn event_stellars(save: &mut Value) {
    add_to_each(save, "events", "stellar", &Value::Null);
}

/// Version 15 to 16: no once-only story subscription has fired.
fn story_fired(save: &mut Value) {
    add_empty(save, &["story_fired"]);
}

/// Version 16 to 17: the galaxy as the catalog has it.
fn galaxy(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert(
            "galaxy".to_owned(),
            serde_json::json!({"systems": [], "stellars": [], "nebulae": []}),
        );
    }
}

/// Version 17 to 18: a destroyed stellar due to regenerate on its next
/// day, one not destroyed with no countdown.
fn countdowns(save: &mut Value) {
    let Some(stellars) = save
        .pointer_mut("/galaxy/stellars")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for stellar in stellars {
        if let Some(destroyed) = stellar.get_mut("destroyed") {
            *destroyed = match destroyed {
                Value::Bool(true) => serde_json::json!({"days_left": 0}),
                _ => Value::Null,
            };
        }
    }
}

/// Version 18 to 19: a combat rating of 0.
fn combat_rating(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("combat_rating".to_owned(), Value::from(0));
    }
}

/// Version 19 to 20: no mission active.
fn active_missions(save: &mut Value) {
    add_empty(save, &["missions"]);
}

/// Version 20 to 21: the records in each system not yet derived.
fn system_records(save: &mut Value) {
    if let Some(object) = save.as_object_mut() {
        object.insert("system_legal".to_owned(), Value::Null);
    }
}

/// Version 21 to 22: each active mission with no special ships left,
/// nowhere, with no timer and no name.
fn mission_ships(save: &mut Value) {
    let none = serde_json::json!({
        "system": null,
        "left": 0,
        "arrived": 0,
        "appear": -1,
        "name": "",
        "subtitle": ""
    });
    add_to_each(save, "missions", "ships", &none);
}

/// Adds `field` to every entry of `save`'s `list`, as `value`.
fn add_to_each(save: &mut Value, list: &str, field: &str, value: &Value) {
    let Some(entries) = save.get_mut(list).and_then(Value::as_array_mut) else {
        return;
    };
    for entry in entries {
        if let Some(entry) = entry.as_object_mut() {
            entry.insert(field.to_owned(), value.clone());
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
    let date = date_of(saved.date)?;
    let missions = missions_of(saved.missions)?;
    Ok(Pilot {
        name: saved.name,
        ship: ShipId(saved.ship),
        ship_name: saved.ship_name,
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
        system_legal: saved.system_legal.map(|legal| {
            legal
                .into_iter()
                .map(|saved| (SystemId(saved.system), saved.record))
                .collect()
        }),
        combat_rating: saved.combat_rating,
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
            .map(|saved| {
                let stellar = saved.stellar.map(StellarId);
                let days = saved.days;
                (DisasterId(saved.disaster), ActiveEvent { days, stellar })
            })
            .collect(),
        default_outfits_pending: saved.outfits.is_none(),
        stock_weapons_pending: !saved.stock_weapons_fitted,
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
                person: saved.person.map(PersonId),
            })
            .collect(),
        gone_persons: saved.gone_persons.into_iter().map(PersonId).collect(),
        grudges: saved.grudges.into_iter().map(PersonId).collect(),
        bits: saved.bits.into_iter().collect(),
        gender: saved.gender.into(),
        paint: saved.paint.map(Rgb15::from),
        story_fired: saved
            .story_fired
            .into_iter()
            .map(SubscriptionId::from)
            .collect(),
        galaxy: saved.galaxy.into(),
        missions,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::catalog::{
        DisasterId, GovtId, JunkId, NebulaId, OutfitId, PersonId, ShipId, StellarId, SystemId,
    };
    use crate::control::ControlBitSet;
    use crate::date::GameDate;
    use crate::escort::EscortOrder;
    use crate::market::Good;
    use crate::pilot::Escort;
    use crate::reserves::{Gauge, Reserves};
    use crate::story::{Consumer, Countdown, GalaxyOverlay, SubscriptionId};

    /// An event under way with `days` left, at `stellar`.
    fn active(days: u16, stellar: Option<StellarId>) -> ActiveEvent {
        ActiveEvent { days, stellar }
    }

    /// An overlay with every kind of change, set out of order.
    fn overlaid() -> GalaxyOverlay {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(300), Some(true));
        overlay.set_govt(SystemId(300), Some(Some(GovtId(140))));
        overlay.set_visible(SystemId(131), Some(false));
        overlay.set_govt(SystemId(200), Some(None));
        overlay.set_stellar_govt(StellarId(151), Some(Some(GovtId(128))));
        overlay.set_destroyed(StellarId(151), Some(Countdown { days_left: 3 }));
        overlay.set_stellar_govt(StellarId(150), Some(None));
        overlay.set_destroyed(StellarId(152), Some(Countdown { days_left: 0 }));
        overlay.set_nebula_active(NebulaId(129), Some(true));
        overlay.set_nebula_active(NebulaId(128), Some(false));
        overlay
    }

    /// A pilot with every field away from its default.
    fn seasoned() -> Pilot {
        Pilot {
            name: "Ada Lovelace".to_owned(),
            ship: ShipId(140),
            ship_name: Some("Kestrel".to_owned()),
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
            system_legal: Some(BTreeMap::from([(SystemId(130), -12), (SystemId(131), 7)])),
            combat_rating: 12_345,
            cargo: BTreeMap::from([(Good::Commodity(2), 7), (Good::Junk(JunkId(146)), 2)]),
            events: BTreeMap::from([
                (DisasterId(128), active(12, Some(StellarId(150)))),
                (DisasterId(130), active(1, None)),
            ]),
            outfits: BTreeMap::from([(OutfitId(256), 3), (OutfitId(128), 1)]),
            default_outfits_pending: false,
            stock_weapons_pending: false,
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
                    person: None,
                },
                Escort {
                    ship: ShipId(141),
                    reserves: Reserves::full(1000.0, 500.0, 400.0),
                    order: None,
                    carried: false,
                    wage: None,
                    person: None,
                },
                Escort {
                    ship: ShipId(144),
                    reserves: Reserves::full(60.0, 40.0, 300.0),
                    order: Some(EscortOrder::Dock),
                    carried: true,
                    wage: None,
                    person: None,
                },
                Escort {
                    ship: ShipId(128),
                    reserves: Reserves::full(30.0, 45.0, 300.0),
                    order: Some(EscortOrder::Hold),
                    carried: false,
                    wage: Some(100),
                    person: None,
                },
                Escort {
                    ship: ShipId(136),
                    reserves: Reserves::full(200.0, 100.0, 300.0),
                    order: None,
                    carried: false,
                    wage: None,
                    person: Some(PersonId(128)),
                },
            ],
            gone_persons: BTreeSet::from([PersonId(151), PersonId(600)]),
            grudges: BTreeSet::from([PersonId(510)]),
            bits: [0, 512, 9999].into_iter().filter_map(Bit::new).collect(),
            gender: Gender::Female,
            paint: Some(Rgb15 { r: 31, g: 0, b: 4 }),
            story_fired: all_fired(),
            galaxy: overlaid(),
            missions: busy(),
        }
    }

    /// Slots 2 and 9 taken, every field set, one failed at once with a
    /// deadline and its cargo aboard, one by `F` with none.
    fn busy() -> ActiveMissions {
        let stop = |stellar, system| {
            Some(Stop {
                stellar: StellarId(stellar),
                system,
            })
        };
        let mut missions = ActiveMissions::default();
        missions.put(
            2,
            ActiveMission {
                mission: MissionId(140),
                destination: stop(150, Some(SystemId(132))),
                return_to: stop(128, None),
                cargo_type: Some(1000),
                cargo_tons: 9_000_000_000,
                pay: -40_050,
                days_left: Some(4_000_000_000),
                deadline: GameDate::new(1180, 3, 1),
                reached: true,
                goals_met: false,
                aboard: true,
                failed: Some(FailedBy::Quick),
                ships: SpecialShips {
                    system: ShipSystem::At(SystemId(131)),
                    left: 3,
                    arrived: 1,
                    appear: 57,
                    name: "Rover".to_owned(),
                    subtitle: "Hunter".to_owned(),
                },
            },
        );
        missions.put(
            9,
            ActiveMission {
                mission: MissionId(141),
                destination: None,
                return_to: None,
                cargo_type: None,
                cargo_tons: 0,
                pay: 500,
                days_left: None,
                deadline: None,
                reached: false,
                goals_met: true,
                aboard: false,
                failed: Some(FailedBy::Operator),
                ships: SpecialShips {
                    system: ShipSystem::FollowsPlayer,
                    ..SpecialShips::default()
                },
            },
        );
        missions
    }

    /// A once-only subscription of every consumer fired, out of order.
    fn all_fired() -> BTreeSet<SubscriptionId> {
        BTreeSet::from([
            fired(Consumer::Plugins, 128, 0),
            fired(Consumer::Missions, 3, 1),
            fired(Consumer::MissionShips, 15, 2),
            fired(Consumer::Missions, 3, 0),
            fired(Consumer::Crons, -5, 9),
            fired(Consumer::Nebulae, 200, 0),
            fired(Consumer::Fixture, 1, 255),
        ])
    }

    fn fired(consumer: Consumer, record: i16, hook: u8) -> SubscriptionId {
        SubscriptionId {
            consumer,
            record,
            hook,
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
        assert_eq!(CURRENT, 22);
        assert_eq!(
            value["galaxy"],
            serde_json::json!({
                "systems": [
                    {"system": 131, "visible": false, "govt": null},
                    {"system": 200, "visible": null, "govt": {"govt": null}},
                    {"system": 300, "visible": true, "govt": {"govt": 140}}
                ],
                "stellars": [
                    {"stellar": 150, "govt": {"govt": null}, "destroyed": null},
                    {"stellar": 151, "govt": {"govt": 128}, "destroyed": {"days_left": 3}},
                    {"stellar": 152, "govt": null, "destroyed": {"days_left": 0}}
                ],
                "nebulae": [
                    {"nebula": 128, "active": false},
                    {"nebula": 129, "active": true}
                ]
            }),
            "by ID"
        );
        assert_eq!(value["stock_weapons_fitted"], true);
        assert_eq!(
            value["story_fired"],
            serde_json::json!([
                {"consumer": "mission_ships", "record": 15, "hook": 2},
                {"consumer": "missions", "record": 3, "hook": 0},
                {"consumer": "missions", "record": 3, "hook": 1},
                {"consumer": "crons", "record": -5, "hook": 9},
                {"consumer": "nebulae", "record": 200, "hook": 0},
                {"consumer": "plugins", "record": 128, "hook": 0},
                {"consumer": "fixture", "record": 1, "hook": 255}
            ]),
            "by consumer, record and hook"
        );
        assert_eq!(value["ship_name"], "Kestrel");
        assert_eq!(value["paint"], serde_json::json!({"r": 31, "g": 0, "b": 4}));
        assert_eq!(value["bits"], serde_json::json!([0, 512, 9999]));
        assert_eq!(value["gender"], "female");
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
            serde_json::json!([
                {"disaster": 128, "days": 12, "stellar": 150},
                {"disaster": 130, "days": 1, "stellar": null}
            ])
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
                "wage": null,
                "person": null
            })
        );
        assert_eq!(value["escorts"][4]["person"], 128);
        assert_eq!(value["escorts"][1]["ship"], 141);
        assert_eq!(value["escorts"][1]["order"], serde_json::Value::Null);
        assert_eq!(value["escorts"][2]["ship"], 144);
        assert_eq!(value["escorts"][2]["order"], "dock");
        assert_eq!(value["escorts"][2]["carried"], true);
        assert_eq!(value["escorts"][2]["wage"], serde_json::Value::Null);
        assert_eq!(value["escorts"][3]["ship"], 128);
        assert_eq!(value["escorts"][3]["wage"], 100);
        assert!(text.contains("\n  \"version\": 22"), "{text}");
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
                    person: None,
                },
                Escort {
                    ship: ShipId(141),
                    reserves: Reserves::full(1000.0, 500.0, 400.0),
                    order: None,
                    carried: false,
                    wage: None,
                    person: None,
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
        assert_eq!(value["version"], CURRENT);
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

    /// A version 9 save: before an escort's person.
    const VERSION_9: &str = r#"{
        "version": 9,
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
        ],
        "gone_persons": [151],
        "grudges": [510]
    }"#;

    #[test]
    fn a_version_9_saves_escorts_load_as_no_person() {
        let pilot = decode(VERSION_9).expect("loads");
        assert_eq!(pilot.name(), "Mercenary");
        assert_eq!(pilot.escorts()[0].person, None);
        assert_eq!(pilot.escorts()[0].wage, Some(100));
        assert!(pilot.gone(PersonId(151)));
        assert!(pilot.grudge(PersonId(510)));
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["escorts"][0]["person"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert!(
                pilot.escorts().iter().all(|escort| escort.person.is_none()),
                "{text}"
            );
        }
    }

    /// A version 10 save: before the control bits and gender.
    const VERSION_10: &str = r#"{
        "version": 10,
        "name": "Wanderer",
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
                "wage": 100,
                "person": 128
            }
        ],
        "gone_persons": [151],
        "grudges": []
    }"#;

    #[test]
    fn a_version_10_save_loads_with_every_bit_clear_and_male() {
        let pilot = decode(VERSION_10).expect("loads");
        assert_eq!(pilot.name(), "Wanderer");
        assert_eq!(pilot.escorts()[0].person, Some(PersonId(128)));
        assert!(pilot.gone(PersonId(151)));
        assert_eq!(pilot.control_bits(), &ControlBitSet::new());
        assert_eq!(pilot.gender(), Gender::Male);
        // Saved again, it is a current save that says so.
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["bits"], serde_json::json!([]));
        assert_eq!(value["gender"], "male");
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2,
            VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.control_bits().iter().count(), 0, "{text}");
            assert_eq!(pilot.gender(), Gender::Male, "{text}");
        }
    }

    /// A version 11 save: before the ship's paint.
    const VERSION_11: &str = r#"{
        "version": 11,
        "name": "Painter",
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
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female"
    }"#;

    #[test]
    fn a_version_11_save_loads_unpainted() {
        let pilot = decode(VERSION_11).expect("loads");
        assert_eq!(pilot.name(), "Painter");
        assert_eq!(pilot.gender(), Gender::Female);
        assert_eq!(pilot.paint(), None);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["paint"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_10, VERSION_5, VERSION_1] {
            assert_eq!(decode(text).expect("loads").paint(), None, "{text}");
        }
    }

    /// A version 12 save: before the ship had a name.
    const VERSION_12: &str = r#"{
        "version": 12,
        "name": "Veteran",
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
        "outfits": [{"outfit": 200, "count": 1}],
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": {"r": 1, "g": 2, "b": 3}
    }"#;

    #[test]
    fn a_version_12_save_loads_with_its_ship_not_yet_named() {
        let pilot = decode(VERSION_12).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), None);
        assert_eq!(pilot.paint(), Some(Rgb15 { r: 1, g: 2, b: 3 }));
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["ship_name"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_11, VERSION_10, VERSION_5, VERSION_1] {
            assert_eq!(decode(text).expect("loads").ship_name(), None, "{text}");
        }
    }

    #[test]
    fn any_ship_name_or_none_survives_a_round_trip_and_a_current_save_must_say() {
        for ship_name in [None, Some(String::new()), Some("* the Bold".to_owned())] {
            let pilot = Pilot {
                ship_name: ship_name.clone(),
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{ship_name:?}");
        }
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value
            .as_object_mut()
            .expect("an object")
            .remove("ship_name");
        let text = value.to_string();
        assert!(unusable(&text).contains("ship_name"), "{}", unusable(&text));
    }

    #[test]
    fn any_paint_or_none_survives_a_round_trip_and_a_current_save_must_say() {
        for paint in [None, Some(Rgb15 { r: 0, g: 31, b: 0 })] {
            let pilot = Pilot {
                paint,
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{paint:?}");
        }
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value.as_object_mut().expect("an object").remove("paint");
        let text = value.to_string();
        assert!(unusable(&text).contains("paint"), "{}", unusable(&text));
    }

    #[test]
    fn every_gender_and_any_bits_survive_a_round_trip() {
        for gender in [Gender::Male, Gender::Female] {
            let pilot = Pilot {
                gender,
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{gender:?}");
        }
        let pilot = Pilot {
            bits: ControlBitSet::new(),
            ..seasoned()
        };
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_current_save_with_a_bit_past_9999_or_of_no_known_gender_is_unusable() {
        let text = encode(&seasoned()).replace("9999", "10000");
        assert!(text.contains("10000"), "{text}");
        assert!(unusable(&text).contains("10000"), "{}", unusable(&text));
        let text = encode(&seasoned()).replace("\"female\"", "\"other\"");
        assert!(text.contains("other"), "{text}");
        assert!(unusable(&text).contains("other"), "{}", unusable(&text));
    }

    /// A version 13 save: before stock weapons were outfits.
    const VERSION_13: &str = r#"{
        "version": 13,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "outfits": [{"outfit": 200, "count": 1}],
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_13_save_loads_with_its_stock_weapons_pending() {
        let pilot = decode(VERSION_13).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert!(pilot.stock_weapons_pending);
        assert!(!pilot.default_outfits_pending);
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["stock_weapons_fitted"], false);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6,
            VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert!(decode(text).expect("loads").stock_weapons_pending, "{text}");
        }
        assert!(
            !decode(&encode(&seasoned()))
                .expect("loads")
                .stock_weapons_pending
        );
    }

    #[test]
    fn a_save_keeps_the_ship_name() {
        let pilot = seasoned();
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["ship_name"], "Kestrel");
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        let mut unnamed = seasoned();
        unnamed.ship_name = None;
        let value: serde_json::Value = serde_json::from_str(&encode(&unnamed)).expect("JSON");
        assert_eq!(value["ship_name"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&unnamed)), Ok(unnamed));
    }

    #[test]
    fn every_save_from_before_ship_names_loads_with_its_ship_not_yet_named() {
        let pilot = decode(VERSION_12).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), None);
        assert_eq!(pilot.owned(OutfitId(200)), 1);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["ship_name"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5,
            VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert_eq!(decode(text).expect("loads").ship_name(), None, "{text}");
        }
    }

    /// A version 14 save: before events kept the stellar they are at.
    const VERSION_14: &str = r#"{
        "version": 14,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "events": [{"disaster": 128, "days": 12}],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_14_save_loads_its_events_at_their_records_stellar() {
        let pilot = decode(VERSION_14).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.ship_name(), Some("Kestrel"));
        assert_eq!(
            pilot.events,
            BTreeMap::from([(DisasterId(128), active(12, None))])
        );
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["events"],
            serde_json::json!([{"disaster": 128, "days": 12, "stellar": null}])
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8, VERSION_7,
            VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            assert_eq!(decode(text).expect("loads").events().count(), 0, "{text}");
        }
    }

    /// A version 15 save: before the story's once-fired subscriptions.
    const VERSION_15: &str = r#"{
        "version": 15,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "events": [{"disaster": 128, "days": 12, "stellar": 150}],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null
    }"#;

    #[test]
    fn a_version_15_save_loads_with_no_story_subscription_fired() {
        let pilot = decode(VERSION_15).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(
            pilot.events,
            BTreeMap::from([(DisasterId(128), active(12, Some(StellarId(150))))])
        );
        assert_eq!(pilot.story_fired, BTreeSet::new());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["story_fired"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9, VERSION_8,
            VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.story_fired, BTreeSet::new(), "{text}");
        }
    }

    /// A version 16 save: before the galaxy overlay.
    const VERSION_16: &str = r#"{
        "version": 16,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [{"consumer": "missions", "record": 3, "hook": 1}]
    }"#;

    #[test]
    fn a_version_16_save_loads_with_the_galaxy_as_the_catalog_has_it() {
        let pilot = decode(VERSION_16).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.course(), [SystemId(132)]);
        assert_eq!(
            pilot.story_fired,
            BTreeSet::from([fired(Consumer::Missions, 3, 1)])
        );
        assert_eq!(pilot.galaxy, GalaxyOverlay::default());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["galaxy"],
            serde_json::json!({"systems": [], "stellars": [], "nebulae": []})
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10, VERSION_9,
            VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.galaxy, GalaxyOverlay::default(), "{text}");
        }
    }

    /// A version 17 save: a destroyed stellar held no countdown.
    const VERSION_17: &str = r#"{
        "version": 17,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [],
        "galaxy": {
            "systems": [{"system": 131, "visible": false, "govt": null}],
            "stellars": [
                {"stellar": 150, "govt": {"govt": 128}, "destroyed": false},
                {"stellar": 151, "govt": null, "destroyed": true}
            ],
            "nebulae": []
        }
    }"#;

    #[test]
    fn a_version_17_saves_destroyed_stellar_is_due_to_regenerate_on_its_next_day() {
        let pilot = decode(VERSION_17).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        let mut expected = GalaxyOverlay::default();
        expected.set_visible(SystemId(131), Some(false));
        expected.set_stellar_govt(StellarId(150), Some(Some(GovtId(128))));
        expected.set_destroyed(StellarId(151), Some(Countdown { days_left: 0 }));
        assert_eq!(pilot.galaxy, expected);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["galaxy"]["stellars"],
            serde_json::json!([
                {"stellar": 150, "govt": {"govt": 128}, "destroyed": null},
                {"stellar": 151, "govt": null, "destroyed": {"days_left": 0}}
            ])
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_16, VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11, VERSION_10,
            VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4, VERSION_3, VERSION_2,
            VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.galaxy, GalaxyOverlay::default(), "{text}");
        }
    }

    /// A version 18 save: before the combat rating.
    const VERSION_18: &str = r#"{
        "version": 18,
        "name": "Veteran",
        "ship": 129,
        "ship_name": "Kestrel",
        "system": 131,
        "stellar": 150,
        "date": {"year": 1177, "month": 6, "day": 24},
        "cash": 4000,
        "reserves": {
            "shield": {"now": 30.0, "max": 30.0},
            "armor": {"now": 45.0, "max": 45.0},
            "fuel": {"now": 200.0, "max": 300.0}
        },
        "course": [132],
        "explored": [131],
        "legal": [],
        "cargo": [],
        "events": [],
        "outfits": [{"outfit": 200, "count": 1}],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [7],
        "gender": "female",
        "paint": null,
        "story_fired": [],
        "galaxy": {
            "systems": [],
            "stellars": [{"stellar": 151, "govt": null, "destroyed": {"days_left": 2}}],
            "nebulae": []
        }
    }"#;

    /// A version 19 save: before the active missions.
    const VERSION_19: &str = r#"{
        "version": 19,
        "name": "Courier",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []}
    }"#;

    #[test]
    fn the_records_in_each_system_are_saved_by_system() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(
            value["system_legal"],
            serde_json::json!([{"system": 130, "record": -12}, {"system": 131, "record": 7}])
        );
    }

    /// A version 20 save: before the legal record in each system.
    const VERSION_20: &str = r#"{
        "version": 20,
        "name": "Lawful",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "legal": [{"govt": 128, "record": 25}],
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": []
    }"#;

    #[test]
    fn a_version_20_save_loads_with_its_system_records_pending() {
        let pilot = decode(VERSION_20).expect("loads");
        assert_eq!(pilot.name(), "Lawful");
        assert!(!pilot.system_records_derived());
        assert_eq!(pilot.legal_record(GovtId(128)), 25, "kept");
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["system_legal"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_19, VERSION_18, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert!(!pilot.system_records_derived(), "{text}");
        }
    }

    #[test]
    fn a_pilot_with_its_system_records_pending_saves_null() {
        let pilot = Pilot {
            system_legal: None,
            ..seasoned()
        };
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["system_legal"], serde_json::Value::Null);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        let derived_empty = Pilot {
            system_legal: Some(BTreeMap::new()),
            ..seasoned()
        };
        let value: serde_json::Value = serde_json::from_str(&encode(&derived_empty)).expect("JSON");
        assert_eq!(value["system_legal"], serde_json::json!([]));
        assert_eq!(decode(&encode(&derived_empty)), Ok(derived_empty));
    }

    #[test]
    fn a_current_save_missing_its_system_records_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(object) = value.as_object_mut() {
            object.remove("system_legal");
        }
        assert!(unusable(&value.to_string()).contains("system_legal"));
    }

    #[test]
    fn an_older_save_loads_with_no_active_missions() {
        let pilot = decode(VERSION_19).expect("loads");
        assert_eq!(pilot.name(), "Courier");
        assert_eq!(pilot.combat_rating(), 7);
        assert_eq!(pilot.missions(), &ActiveMissions::default());
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["missions"], serde_json::json!([]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [VERSION_18, VERSION_17, VERSION_1] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.missions(), &ActiveMissions::default(), "{text}");
        }
    }

    #[test]
    fn the_active_missions_are_saved_by_slot() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(
            value["missions"],
            serde_json::json!([
                {
                    "slot": 2,
                    "mission": 140,
                    "destination": {"stellar": 150, "system": 132},
                    "return": {"stellar": 128, "system": null},
                    "cargo_type": 1000,
                    "cargo_tons": 9_000_000_000_i64,
                    "pay": -40_050,
                    "days_left": 4_000_000_000_u32,
                    "deadline": {"year": 1180, "month": 3, "day": 1},
                    "reached": true,
                    "goals_met": false,
                    "aboard": true,
                    "failed": "quick",
                    "ships": {
                        "system": {"at": 131},
                        "left": 3,
                        "arrived": 1,
                        "appear": 57,
                        "name": "Rover",
                        "subtitle": "Hunter"
                    }
                },
                {
                    "slot": 9,
                    "mission": 141,
                    "destination": null,
                    "return": null,
                    "cargo_type": null,
                    "cargo_tons": 0,
                    "pay": 500,
                    "days_left": null,
                    "deadline": null,
                    "reached": false,
                    "goals_met": true,
                    "aboard": false,
                    "failed": "operator",
                    "ships": {
                        "system": "follows_player",
                        "left": 0,
                        "arrived": 0,
                        "appear": -1,
                        "name": "",
                        "subtitle": ""
                    }
                }
            ])
        );
    }

    /// A version 21 save: before the active missions' special ships.
    const VERSION_21: &str = r#"{
        "version": 21,
        "name": "Hunter",
        "ship": 129,
        "ship_name": "Kestrel",
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
        "system_legal": null,
        "combat_rating": 7,
        "cargo": [],
        "events": [],
        "outfits": [],
        "stock_weapons_fitted": true,
        "escorts": [],
        "gone_persons": [],
        "grudges": [],
        "bits": [],
        "gender": "male",
        "paint": null,
        "story_fired": [],
        "galaxy": {"systems": [], "stellars": [], "nebulae": []},
        "missions": [{
            "slot": 4,
            "mission": 140,
            "destination": null,
            "return": null,
            "cargo_type": null,
            "cargo_tons": 0,
            "pay": 500,
            "days_left": null,
            "deadline": null,
            "reached": false,
            "goals_met": false,
            "aboard": false,
            "failed": null
        }]
    }"#;

    #[test]
    fn a_version_21_saves_missions_load_with_no_special_ships() {
        let pilot = decode(VERSION_21).expect("loads");
        let active = pilot.missions().get(4).expect("held");
        assert_eq!(active.mission, MissionId(140));
        assert_eq!(active.ships, SpecialShips::default());
        assert_eq!(active.ships.system, ShipSystem::Nowhere);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn the_special_ships_survive_a_round_trip() {
        let pilot = seasoned();
        let back = decode(&encode(&pilot)).expect("loads");
        assert_eq!(back.missions(), pilot.missions());
        assert_eq!(
            back.missions().get(2).map(|active| active.ships.system),
            Some(ShipSystem::At(SystemId(131)))
        );
    }

    #[test]
    fn a_mission_in_no_slot_or_on_no_date_is_unusable() {
        let with_slot = |slot: i64, day: u8| {
            let mut value: serde_json::Value =
                serde_json::from_str(&encode(&seasoned())).expect("JSON");
            value["missions"][0]["slot"] = serde_json::json!(slot);
            value["missions"][0]["deadline"]["day"] = serde_json::json!(day);
            value.to_string()
        };
        assert_eq!(unusable(&with_slot(16, 1)), "no such mission slot, 16");
        assert_eq!(unusable(&with_slot(2, 32)), "no such date, 32/3/1180");
        assert!(!unusable(&with_slot(-1, 1)).is_empty());
        assert_eq!(
            decode(&with_slot(15, 1)).map(|pilot| pilot.missions().holding(MissionId(140))),
            Ok(vec![15])
        );
    }

    #[test]
    fn an_older_save_loads_with_a_combat_rating_of_0() {
        let pilot = decode(VERSION_18).expect("loads");
        assert_eq!(pilot.name(), "Veteran");
        assert_eq!(pilot.combat_rating(), 0);
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(value["combat_rating"], 0);
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        for text in [
            VERSION_17, VERSION_16, VERSION_15, VERSION_14, VERSION_13, VERSION_12, VERSION_11,
            VERSION_10, VERSION_9, VERSION_8, VERSION_7, VERSION_6, VERSION_5, VERSION_4,
            VERSION_3, VERSION_2, VERSION_1,
        ] {
            let pilot = decode(text).expect("loads");
            assert_eq!(pilot.combat_rating(), 0, "{text}");
        }
    }

    #[test]
    fn any_combat_rating_survives_a_round_trip() {
        for combat_rating in [i64::MIN, -3, 0, 10_000_009, i64::MAX] {
            let pilot = Pilot {
                combat_rating,
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{combat_rating}");
        }
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(value["combat_rating"], 12_345, "its number");
    }

    #[test]
    fn any_overlay_survives_a_round_trip() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(i16::MIN), Some(false));
        overlay.set_govt(SystemId(i16::MAX), Some(Some(GovtId(i16::MIN))));
        overlay.set_stellar_govt(StellarId(-1), Some(None));
        overlay.set_destroyed(
            StellarId(i16::MAX),
            Some(Countdown {
                days_left: u16::MAX,
            }),
        );
        overlay.set_nebula_active(NebulaId(i16::MAX), Some(false));
        for galaxy in [overlay, GalaxyOverlay::default(), overlaid()] {
            let pilot = Pilot {
                galaxy,
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot));
        }
    }

    #[test]
    fn a_current_save_missing_any_part_of_its_galaxy_is_unusable() {
        let full: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        for (list, field) in [
            ("", "systems"),
            ("", "stellars"),
            ("", "nebulae"),
            ("systems", "system"),
            ("systems", "visible"),
            ("systems", "govt"),
            ("stellars", "stellar"),
            ("stellars", "govt"),
            ("stellars", "destroyed"),
            ("nebulae", "nebula"),
            ("nebulae", "active"),
        ] {
            let mut value = full.clone();
            let galaxy = &mut value["galaxy"];
            let object = if list.is_empty() {
                galaxy.as_object_mut()
            } else {
                galaxy[list][0].as_object_mut()
            };
            object.expect("an object").remove(field);
            let reason = unusable(&value.to_string());
            assert!(reason.contains(field), "{list} {field}: {reason}");
        }
        let mut value = full.clone();
        if let Some(govt) = value["galaxy"]["systems"][1]["govt"].as_object_mut() {
            govt.remove("govt");
        }
        assert!(
            unusable(&value.to_string()).contains("govt"),
            "an empty override"
        );
        let mut value = full;
        if let Some(countdown) = value["galaxy"]["stellars"][1]["destroyed"].as_object_mut() {
            countdown.remove("days_left");
        }
        assert!(
            unusable(&value.to_string()).contains("days_left"),
            "an empty countdown"
        );
    }

    #[test]
    fn every_consumer_fired_survives_a_round_trip() {
        use Consumer::{Crons, Fixture, MissionShips, Missions, Nebulae, Plugins};
        for consumer in [MissionShips, Missions, Crons, Nebulae, Plugins, Fixture] {
            let mut pilot = seasoned();
            pilot.story_fired = BTreeSet::from([fired(consumer, i16::MIN, u8::MAX)]);
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{consumer:?}");
        }
    }

    #[test]
    fn a_current_save_missing_its_fired_subscriptions_or_of_no_known_consumer_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(object) = value.as_object_mut() {
            object.remove("story_fired");
        }
        assert!(unusable(&value.to_string()).contains("story_fired"));
        let text =
            encode(&seasoned()).replace("\"consumer\": \"nebulae\"", "\"consumer\": \"comets\"");
        assert!(unusable(&text).contains("comets"), "{}", unusable(&text));
    }

    #[test]
    fn a_current_save_event_missing_its_stellar_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(event) = value["events"][0].as_object_mut() {
            event.remove("stellar");
        }
        assert!(unusable(&value.to_string()).contains("stellar"));
    }

    #[test]
    fn a_current_save_missing_whether_its_stock_weapons_are_fitted_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(object) = value.as_object_mut() {
            object.remove("stock_weapons_fitted");
        }
        assert!(unusable(&value.to_string()).contains("stock_weapons_fitted"));
    }

    #[test]
    fn every_escorts_person_survives_a_round_trip() {
        for person in [None, Some(PersonId(128)), Some(PersonId(1151))] {
            let mut pilot = seasoned();
            pilot.escorts[0].person = person;
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{person:?}");
        }
    }

    #[test]
    fn a_current_escort_missing_its_person_or_of_no_number_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(escort) = value["escorts"][4].as_object_mut() {
            escort.remove("person");
        }
        assert!(unusable(&value.to_string()).contains("person"));
        let text = encode(&seasoned()).replace("\"person\": 128", "\"person\": \"Ace\"");
        assert!(text.contains("Ace"));
        assert!(unusable(&text).contains("Ace"), "{}", unusable(&text));
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
        let newer = encode(&seasoned()).replace("\"version\": 22", "\"version\": 23");
        assert_eq!(decode(&newer), Err(SaveError::Newer { version: 23 }));
        assert_eq!(
            SaveError::Newer { version: 23 }.to_string(),
            "This pilot file was created with a different version of Nova, and can't be used \
             (it is version 23, and this version of Nova reads up to 22)."
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
        pilot.events.insert(DisasterId(999), active(3, None));
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
