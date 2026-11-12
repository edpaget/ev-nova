//! The saved shapes: the private records each part of a pilot is saved
//! as, their conversions to and from the pilot's own types, and the
//! helpers that turn a part of the pilot into its saved list and back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::SaveError;
use super::story_state::SavedDialogState;
use super::storylines::SavedStory;
use nova_sim_ai::escort::EscortOrder;
use nova_sim_ai::traffic::stellar_defence::DefenceShips;
use nova_sim_base::reserves::{Gauge, Reserves};
use nova_sim_catalog::catalog::{
    CronId, GovtId, JunkId, MissionId, NebulaId, ShipId, StellarId, SystemId,
};
use nova_sim_catalog::control::Bit;
use nova_sim_catalog::voice::VoiceVariant;
use nova_sim_combat::fleet::{EscortMark, OrderWord};
use nova_sim_state::cron::CronState;
use nova_sim_state::date::GameDate;
use nova_sim_state::market::Good;
use nova_sim_state::outfit_effects::Rgb15;
use nova_sim_story::mission::aux_ships::AuxShips;
use nova_sim_story::mission::{
    ActiveMission, ActiveMissions, FailedBy, ShipSystem, SpecialShips, Stop,
};
use nova_sim_story::pilot::Gender;
use nova_sim_story::story::{Consumer, Countdown, GalaxyOverlay, SubscriptionId};

/// A save's date.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) struct SavedDate {
    pub(super) year: i32,
    pub(super) month: u8,
    pub(super) day: u8,
}

/// A saved reserve: how much the ship has, and can hold.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedGauge {
    pub(super) now: f32,
    pub(super) max: f32,
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
pub(super) struct SavedReserves {
    pub(super) shield: SavedGauge,
    pub(super) armor: SavedGauge,
    pub(super) fuel: SavedGauge,
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
pub(super) struct SavedEscort {
    pub(super) ship: i16,
    pub(super) reserves: SavedReserves,
    // Without this, serde would read a missing optional field as `None`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) order: Option<SavedOrder>,
    pub(super) carried: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) wage: Option<i64>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) person: Option<i16>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) mark: Option<SavedMark>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) voice: Option<SavedVoice>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) berth: Option<u32>,
}

/// A saved fighter's voice variant (see the module docs' version 36).
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum SavedVoice {
    Even,
    Odd,
}

impl From<VoiceVariant> for SavedVoice {
    fn from(variant: VoiceVariant) -> Self {
        match variant {
            VoiceVariant::Even => Self::Even,
            VoiceVariant::Odd => Self::Odd,
        }
    }
}

impl From<SavedVoice> for VoiceVariant {
    fn from(saved: SavedVoice) -> Self {
        match saved {
            SavedVoice::Even => Self::Even,
            SavedVoice::Odd => Self::Odd,
        }
    }
}

/// A saved escort's mark for the next shipyard.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum SavedMark {
    Upgrade,
    Sell,
}

impl From<EscortMark> for SavedMark {
    fn from(mark: EscortMark) -> Self {
        match mark {
            EscortMark::Upgrade => Self::Upgrade,
            EscortMark::Sell => Self::Sell,
        }
    }
}

impl From<SavedMark> for EscortMark {
    fn from(saved: SavedMark) -> Self {
        match saved {
            SavedMark::Upgrade => Self::Upgrade,
            SavedMark::Sell => Self::Sell,
        }
    }
}

/// A saved standing order.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum SavedOrder {
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
pub(super) enum SavedGender {
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
pub(super) struct SavedPaint {
    pub(super) r: u8,
    pub(super) g: u8,
    pub(super) b: u8,
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
pub(super) struct SavedRecord {
    pub(super) govt: i16,
    pub(super) record: i16,
}

/// A saved legal record in a system.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedSystemRecord {
    pub(super) system: i16,
    pub(super) record: i16,
}

/// A saved good: a standard commodity's number, or a `jünk`'s ID.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum SavedGood {
    Commodity(u8),
    Junk(i16),
}

/// A good held, and how many tons.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedCargo {
    pub(super) good: SavedGood,
    pub(super) tons: u32,
}

/// An event under way, the days it has left, and the stellar it is at.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedEvent {
    pub(super) disaster: i16,
    pub(super) days: u16,
    /// `None` for its record's own `Stellar`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) stellar: Option<i16>,
}

/// A saved story consumer.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SavedConsumer {
    Offers,
    MissionShips,
    Missions,
    Storylines,
    Crons,
    Nebulae,
    Plugins,
    Fixture,
}

impl From<Consumer> for SavedConsumer {
    fn from(consumer: Consumer) -> Self {
        match consumer {
            Consumer::Offers => Self::Offers,
            Consumer::MissionShips => Self::MissionShips,
            Consumer::Missions => Self::Missions,
            Consumer::Storylines => Self::Storylines,
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
            SavedConsumer::Offers => Self::Offers,
            SavedConsumer::MissionShips => Self::MissionShips,
            SavedConsumer::Missions => Self::Missions,
            SavedConsumer::Storylines => Self::Storylines,
            SavedConsumer::Crons => Self::Crons,
            SavedConsumer::Nebulae => Self::Nebulae,
            SavedConsumer::Plugins => Self::Plugins,
            SavedConsumer::Fixture => Self::Fixture,
        }
    }
}

/// A saved once-only story subscription that has fired: its id.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedFired {
    pub(super) consumer: SavedConsumer,
    pub(super) record: i16,
    pub(super) hook: u8,
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
pub(super) struct SavedGovt {
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) govt: Option<i16>,
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
pub(super) struct SavedSystem {
    pub(super) system: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) visible: Option<bool>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) govt: Option<SavedGovt>,
}

/// A destroyed stellar's regeneration countdown.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedCountdown {
    pub(super) days_left: u16,
}

/// What the overlay changes of a stellar: `destroyed` is `None` while it
/// is not, and `dominated` whether the player dominated it.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedStellar {
    pub(super) stellar: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) govt: Option<SavedGovt>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) destroyed: Option<SavedCountdown>,
    pub(super) dominated: bool,
}

/// A stellar's defence ships left, with its raw `DefCount` as the save
/// knew it.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedDefenceShips {
    pub(super) stellar: i16,
    pub(super) left: i16,
    pub(super) def_count: i16,
}

/// Whether a nebula is active, as the overlay says.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedNebula {
    pub(super) nebula: i16,
    pub(super) active: bool,
}

/// The saved galaxy overlay, each list by ascending ID.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedGalaxy {
    pub(super) systems: Vec<SavedSystem>,
    pub(super) stellars: Vec<SavedStellar>,
    pub(super) nebulae: Vec<SavedNebula>,
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
                    dominated: change.dominated,
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
            overlay.set_dominated(id, stellar.dominated);
        }
        for nebula in saved.nebulae {
            overlay.set_nebula_active(NebulaId(nebula.nebula), Some(nebula.active));
        }
        overlay
    }
}

/// An outfit carried, and how many.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedOutfit {
    pub(super) outfit: i16,
    pub(super) count: u16,
}

/// A saved stellar a mission names, with the system it belongs to.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedStop {
    pub(super) stellar: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) system: Option<i16>,
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
pub(super) enum SavedFailure {
    Quick,
    Operator,
}

impl From<FailedBy> for SavedFailure {
    fn from(failed: FailedBy) -> Self {
        match failed {
            FailedBy::Quick => Self::Quick,
            FailedBy::Marked => Self::Operator,
        }
    }
}

impl From<SavedFailure> for FailedBy {
    fn from(saved: SavedFailure) -> Self {
        match saved {
            SavedFailure::Quick => Self::Quick,
            SavedFailure::Operator => Self::Marked,
        }
    }
}

/// Where a saved mission's special ships are; `null` for nowhere.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SavedShipSystem {
    At(i16),
    FollowsPlayer,
}

/// A saved mission's special ships.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedShips {
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) system: Option<SavedShipSystem>,
    pub(super) left: i16,
    pub(super) arrived: i16,
    pub(super) appear: i16,
    pub(super) name: String,
    pub(super) subtitle: String,
    pub(super) destroyed: i16,
    pub(super) boarded: i16,
    pub(super) disabled: i16,
    pub(super) chased_off: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) uniform_type: Option<i16>,
    pub(super) ship_done: bool,
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
            destroyed: ships.destroyed,
            boarded: ships.boarded,
            disabled: ships.disabled,
            chased_off: ships.chased_off,
            uniform_type: ships.uniform_type.map(|ship| ship.0),
            ship_done: ships.ship_done,
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
            destroyed: saved.destroyed,
            boarded: saved.boarded,
            disabled: saved.disabled,
            chased_off: saved.chased_off,
            uniform_type: saved.uniform_type.map(ShipId),
            ship_done: saved.ship_done,
        }
    }
}

/// A saved mission's auxiliary ships.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedAux {
    pub(super) countdown: i16,
    pub(super) arrived: i16,
    pub(super) left: i16,
}

impl From<AuxShips> for SavedAux {
    fn from(aux: AuxShips) -> Self {
        Self {
            countdown: aux.countdown,
            arrived: aux.arrived,
            left: aux.left,
        }
    }
}

impl From<SavedAux> for AuxShips {
    fn from(saved: SavedAux) -> Self {
        Self {
            countdown: saved.countdown,
            arrived: saved.arrived,
            left: saved.left,
        }
    }
}

/// A saved active mission, in its slot.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedMission {
    pub(super) slot: u8,
    pub(super) mission: i16,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) destination: Option<SavedStop>,
    #[serde(rename = "return", deserialize_with = "Option::deserialize")]
    pub(super) return_to: Option<SavedStop>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) cargo_type: Option<i16>,
    pub(super) cargo_tons: i64,
    pub(super) pay: i64,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) days_left: Option<u32>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) deadline: Option<SavedDate>,
    pub(super) reached: bool,
    pub(super) goals_met: bool,
    pub(super) aboard: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) failed: Option<SavedFailure>,
    pub(super) ships: SavedShips,
    pub(super) aux: SavedAux,
}

impl SavedMission {
    /// `active`, in `slot`.
    pub(super) fn new(slot: u8, active: &ActiveMission) -> Self {
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
            aux: SavedAux::from(active.aux),
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
pub(super) fn date_of(saved: SavedDate) -> Result<GameDate, SaveError> {
    let SavedDate { year, month, day } = saved;
    GameDate::new(year, month, day)
        .ok_or_else(|| SaveError::Unusable(format!("no such date, {day}/{month}/{year}")))
}

/// The active missions `saved` lists, each in its slot, or why they
/// cannot be.
pub(super) fn missions_of(saved: Vec<SavedMission>) -> Result<ActiveMissions, SaveError> {
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
            aux: AuxShips::from(saved.aux),
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
pub(super) struct Saved {
    pub(super) version: u64,
    pub(super) name: String,
    pub(super) nickname: String,
    pub(super) ship: i16,
    /// `None` for a ship not yet named.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) ship_name: Option<String>,
    pub(super) system: i16,
    // Without this, serde would read a missing optional field as `None`.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) stellar: Option<i16>,
    pub(super) date: SavedDate,
    pub(super) cash: i64,
    pub(super) reserves: SavedReserves,
    pub(super) course: Vec<i16>,
    /// `None` for no start (the original's slot 0 at -1).
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) course_start: Option<i16>,
    pub(super) explored: Vec<i16>,
    pub(super) charted: Vec<i16>,
    pub(super) reinforcement_days: Vec<SavedReinforcementDays>,
    pub(super) legal: Vec<SavedRecord>,
    /// `None` until the records in each system are derived.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) system_legal: Option<Vec<SavedSystemRecord>>,
    pub(super) combat_rating: i64,
    pub(super) cargo: Vec<SavedCargo>,
    pub(super) events: Vec<SavedEvent>,
    /// `None` for the ship's default items, not yet read.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) outfits: Option<Vec<SavedOutfit>>,
    /// Whether the ship's stock weapons are among `outfits`: not in a
    /// save from before they were outfits.
    pub(super) stock_weapons_fitted: bool,
    pub(super) escorts: Vec<SavedEscort>,
    /// The class table's words, raw, in class order (see the module docs'
    /// version 37).
    pub(super) class_orders: [i16; 4],
    pub(super) gone_persons: Vec<i16>,
    pub(super) grudges: Vec<i16>,
    pub(super) bits: Vec<Bit>,
    pub(super) gender: SavedGender,
    pub(super) strict_play: bool,
    /// `None` before the first jump.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) jumped_from: Option<i16>,
    /// `None` when unpainted.
    #[serde(deserialize_with = "Option::deserialize")]
    pub(super) paint: Option<SavedPaint>,
    pub(super) story_fired: Vec<SavedFired>,
    pub(super) galaxy: SavedGalaxy,
    pub(super) missions: Vec<SavedMission>,
    pub(super) crons: Vec<SavedCron>,
    pub(super) ranks: Vec<i16>,
    pub(super) seen_intro: bool,
    pub(super) defence_ships: Vec<SavedDefenceShips>,
    pub(super) flags: Vec<String>,
    pub(super) dialogs: Vec<SavedDialogState>,
    pub(super) story: SavedStory,
}

/// A saved reinforcement countdown: a system and its days left.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedReinforcementDays {
    pub(super) system: i16,
    pub(super) days: u16,
}

/// The countdowns `days` holds as saved, by system.
pub(super) fn saved_reinforcement_days(
    days: &BTreeMap<SystemId, u16>,
) -> Vec<SavedReinforcementDays> {
    days.iter()
        .map(|(system, &days)| SavedReinforcementDays {
            system: system.0,
            days,
        })
        .collect()
}

/// The countdowns `saved` lists, those of none left out, as the pilot
/// keeps them.
pub(super) fn reinforcement_days_of(saved: Vec<SavedReinforcementDays>) -> BTreeMap<SystemId, u16> {
    saved
        .into_iter()
        .filter(|saved| saved.days > 0)
        .map(|saved| (SystemId(saved.system), saved.days))
        .collect()
}

/// A saved crön running or holding off: its ID and its counters.
#[derive(Serialize, Deserialize)]
pub(super) struct SavedCron {
    pub(super) cron: i16,
    pub(super) days: i16,
    pub(super) holdoff: i16,
}

/// The crons `crons` holds as saved: the active ones, by ID.
pub(super) fn saved_crons(crons: &BTreeMap<CronId, CronState>) -> Vec<SavedCron> {
    crons
        .iter()
        .filter(|(_, state)| state.active())
        .map(|(id, state)| SavedCron {
            cron: id.0,
            days: state.days,
            holdoff: state.holdoff,
        })
        .collect()
}

/// The crons `saved` lists that a load finds active
/// (`_LoadPilotData` @0x75fbc-0x76025).
pub(super) fn crons_of(saved: Vec<SavedCron>) -> BTreeMap<CronId, CronState> {
    saved
        .into_iter()
        .map(|saved| {
            let state = CronState {
                days: saved.days,
                holdoff: saved.holdoff,
            };
            (CronId(saved.cron), state)
        })
        .filter(|(_, state)| state.active())
        .collect()
}

/// The legal records `legal` holds as saved, by government.
pub(super) fn saved_legal(legal: &BTreeMap<GovtId, i16>) -> Vec<SavedRecord> {
    legal
        .iter()
        .map(|(govt, &record)| SavedRecord {
            govt: govt.0,
            record,
        })
        .collect()
}

/// The defence ships left `ships` holds, as a save keeps them.
pub(super) fn saved_defence_ships(ships: &DefenceShips) -> Vec<SavedDefenceShips> {
    ships
        .saved()
        .map(|(stellar, left, def_count)| SavedDefenceShips {
            stellar: stellar.0,
            left,
            def_count,
        })
        .collect()
}

/// The defence ships left `saved` keeps.
pub(super) fn defence_ships_of(saved: &[SavedDefenceShips]) -> DefenceShips {
    let mut ships = DefenceShips::default();
    for saved in saved {
        ships.set_left_of(StellarId(saved.stellar), saved.def_count, saved.left);
    }
    ships
}

/// The class table's word a saved `word` loads as: any negative one as -1,
/// unset, and any other raw (`_LoadPilotData`'s `js` @0x760de).
pub(super) fn class_word_of(word: i16) -> OrderWord {
    if word < 0 {
        OrderWord::NONE
    } else {
        OrderWord(word)
    }
}

/// The good and tons a saved hold entry keeps.
pub(super) fn cargo_of(saved: &SavedCargo) -> (Good, u32) {
    let good = match saved.good {
        SavedGood::Commodity(n) => Good::Commodity(n),
        SavedGood::Junk(id) => Good::Junk(JunkId(id)),
    };
    (good, saved.tons)
}

/// The systems explored a save lists, each charted one among them (see
/// the `upgrade` module docs' version 29).
pub(super) fn explored_of(
    explored: &[i16],
    charted: &[i16],
) -> std::collections::BTreeSet<SystemId> {
    explored
        .iter()
        .chain(charted)
        .copied()
        .map(SystemId)
        .collect()
}
