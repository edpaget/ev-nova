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
//! The course and its start are always saved. Whether a load keeps them
//! is [`RuleKey::LoadedCourse`](nova_sim_base::rulebook::RuleKey::LoadedCourse)'s: by the
//! engine's reading `Session::load` drops them, as the original's open
//! clears `_hyperRoute`.
//!
//! The system last jumped from is always saved too. Whether a load keeps
//! it is [`RuleKey::LoadedJumpedFrom`](nova_sim_base::rulebook::RuleKey::LoadedJumpedFrom)'s:
//! by the engine's reading `Session::load` forgets it, as the original's
//! open sets player +0x94 to -1.
//!
//! The upgrade module (`upgrade`) lists what each version added and
//! how an older save is upgraded.
//!
//! IDs are saved as their raw numbers, the nickname (`nickname`) as its
//! text, the ship's name (`ship_name`) as
//! its text, or `null` for a ship not yet named, the date as its year, month and
//! day, each reserve as how much the ship has and can hold, each good held
//! as its kind (`commodity` with its number, or `junk` with its ID) and
//! tons, each event as its `öops` ID, days left and the `spöb` ID it is
//! at (`null` for its record's own), each outfit as its
//! `oütf` ID and how many, and each escort as its `shïp` ID, reserves,
//! standing order (`defend`, `attack`, `hold`, `dock`, or `null` for
//! none), whether it is `carried`, its `wage` in credits a day (`null`
//! for an escort that was not hired), and the `person` it is (a `përs`
//! ID, or `null`; one whose record is gone is kept as saved), its
//! `mark` (`upgrade`, `sell` or `null`), its `voice` and its `berth`. The class table
//! (`class_orders`) is four raw words in the escort menu's class order
//! (fighters, medium ships, warships, freighters): -1 unset, 0 formation, 1
//! defend, 2 attack, 3 return, 4 hold. The
//! fighters aboard a bay are its
//! rounds, the ammunition outfits saved with the outfits; the fighters
//! out are the carried escorts. Which carrier a fighter came from is
//! never saved: it docks with the player's bay of its type. The persons
//! gone for good (`gone_persons`) and those holding a grudge (`grudges`)
//! are each a list of `përs` IDs. The control bits (`bits`) are the
//! numbers of the bits set, ascending, and the `gender` is `male` or
//! `female`. Strict Play (`strict_play`) is `true` or `false`. The system last jumped from (`jumped_from`) is its `sÿst` ID, or `null` before the first jump. The combat rating (`combat_rating`) is its number. The `paint` is `null` when unpainted, or its red, green and
//! blue, each 0 to 31 (`{"r": 31, "g": 0, "b": 4}`). The once-only story
//! subscriptions fired (`story_fired`) are each its consumer
//! (`offers`, `mission_ships`, `missions`, `storylines`, `crons`,
//! `nebulae`, `plugins` or `fixture`), record and hook (`{"consumer": "missions", "record": 3,
//! "hook": 1}`), listed by consumer, record and hook. The galaxy overlay
//! (`galaxy`) is three lists, each by ascending ID: `systems`, each its
//! `sÿst` ID, whether it is `visible` and its `govt`; `stellars`, each its
//! `spöb` ID, its `govt` and, while it is `destroyed`, its regeneration
//! countdown (`{"days_left": 3}`), else `null`; and `nebulae`, each its
//! `nëbu` ID and whether it is `active`; whether a nebula is explored
//! is never saved, as the original's load forgets it (`_ResetPlayer`
//! @0x1da86; see the [`nebula`](nova_sim_story::nebula) module). A `visible` or `govt` of `null`
//! changes nothing; a `govt` override is `{"govt": id}`, or `{"govt":
//! null}` for independent (`{"system": 131, "visible": false, "govt":
//! {"govt": 128}}`). A system's `visible` is written as the overlay holds
//! it (`false` hidden by the last refresh of its `Visibility`, `true` a
//! chain head the player's move showed), but never trusted: a load
//! recomputes it from the bits before anything reads it (crons-news-ranks
//! phase 5), as the original saves no visibility.
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
//! `operator` (marked, by `F` or a ship goal's check, nothing run yet),
//! and its special `ships`: their `system` (`{"at":
//! 130}`, `"follows_player"`, or `null` for nowhere), how many are `left`
//! and have `arrived`, their `appear` timer (-1 when it does not run),
//! their drawn `name` and `subtitle`, how many have been
//! `destroyed`, `boarded`, `disabled` and `chased_off`, the one ship
//! type they fly (`uniform_type`, a `shïp` ID, or `null` for a type drawn
//! for each), and whether their goals-met moment has come (`ship_done`).
//! A slot past 15
//! is unusable.
//!
//! The crons (`crons`) are those running or holding off, by ascending
//! ID: each its `crön` ID (`cron`), its `days` left (-1 in its
//! post-holdoff) and its `holdoff` (`{"cron": 156, "days": 105,
//! "holdoff": 0}`). A load keeps an entry only while its `days` or
//! `holdoff` is 0 or more, as the original derives a slot's `active`
//! (`_LoadPilotData` @0x75fbc-0x76025), so an entry of -1 and -1 is an
//! inactive crön, and a save leaves the inactive ones out. One whose
//! record is gone is kept as saved, and flying the pilot drops it
//! (`nova_sim::Session::fly`).
//!
//! The active ranks (`ranks`) are their `ränk` IDs, ascending, as the
//! original saves one flag for each rank (`_WritePilotData` @0x72910).
//! The recent rank `<RRK>` reads is not saved, as the original's isn't.
//! One whose record is gone is kept as saved, and flying the pilot drops
//! it, as `_LoadPilotData` does (@0x75e47-0x75eb6).
//!
//! The world flags set (`flags`) are their names, ascending, and the story
//! states (`dialogs`) are each an Ink `file` with its adapter's `state` as
//! text, by file ascending (see the `story_state` module). A file the data
//! no longer has keeps its state as saved.

mod schema;
mod story_state;
mod storylines;
mod upgrade;

use serde::Deserialize;
use serde_json::Value;

use nova_sim_ai::escort::EscortOrder;
use nova_sim_catalog::catalog::{
    DisasterId, GovtId, OutfitId, PersonId, RankId, ShipId, StellarId, SystemId,
};
use nova_sim_catalog::voice::VoiceVariant;
use nova_sim_combat::fleet::EscortMark;
use nova_sim_state::market::{ActiveEvent, Good};
use nova_sim_state::outfit_effects::Rgb15;
use nova_sim_story::pilot::{Escort, Pilot};
use nova_sim_story::story::SubscriptionId;
use schema::{
    Saved, SavedCargo, SavedEscort, SavedEvent, SavedFired, SavedGood, SavedMark, SavedMission,
    SavedOrder, SavedOutfit, SavedPaint, SavedSystemRecord, SavedVoice, cargo_of, class_word_of,
    crons_of, date_of, defence_ships_of, explored_of, missions_of, reinforcement_days_of,
    saved_crons, saved_defence_ships, saved_legal, saved_reinforcement_days,
};
use story_state::{dialogs_of, flags_of, saved_dialogs, saved_flags};
use storylines::{saved_story, story_of};
use upgrade::UPGRADES;

/// The version [`encode`] writes, and the newest [`decode`] reads.
pub const CURRENT: u64 = 41;

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

/// `pilot` as the current version's save: pretty JSON.
#[must_use]
#[allow(clippy::too_many_lines)] // one field of the save for each of the pilot's
pub fn encode(pilot: &Pilot) -> String {
    let saved = Saved {
        version: CURRENT,
        name: pilot.name.clone(),
        nickname: pilot.nickname.clone(),
        ship: pilot.ship.0,
        ship_name: pilot.ship_name.clone(),
        system: pilot.system.0,
        stellar: pilot.stellar.map(|stellar| stellar.0),
        date: pilot.date.into(),
        cash: pilot.cash,
        reserves: pilot.reserves.into(),
        course: pilot.course.iter().map(|system| system.0).collect(),
        course_start: pilot.course_start.map(|system| system.0),
        explored: pilot.explored.iter().map(|system| system.0).collect(),
        charted: pilot.charted.iter().map(|system| system.0).collect(),
        reinforcement_days: saved_reinforcement_days(&pilot.reinforcement_days),
        legal: saved_legal(&pilot.legal),
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
                mark: escort.mark.map(SavedMark::from),
                voice: escort.saved_voice().map(SavedVoice::from),
                berth: escort.berth,
            })
            .collect(),
        class_orders: pilot.class_orders.map(|word| word.0),
        gone_persons: pilot.gone_persons.iter().map(|person| person.0).collect(),
        grudges: pilot.grudges.iter().map(|person| person.0).collect(),
        bits: pilot.bits.iter().collect(),
        gender: pilot.gender.into(),
        strict_play: pilot.strict_play,
        jumped_from: pilot.jumped_from.map(|system| system.0),
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
        crons: saved_crons(&pilot.crons),
        ranks: pilot.ranks.iter().map(|rank| rank.0).collect(),
        seen_intro: pilot.seen_intro,
        defence_ships: saved_defence_ships(&pilot.defence_ships),
        flags: saved_flags(&pilot.flags),
        dialogs: saved_dialogs(&pilot.dialogs),
        story: saved_story(&pilot.story),
    };
    serde_json::to_string_pretty(&saved).expect("plain values always serialise")
}

/// The pilot `text` saves, at any version up to [`CURRENT`].
///
/// # Errors
///
/// [`SaveError::Newer`] for a save from a newer version, and
/// [`SaveError::Unusable`] for text that is not a save (not JSON, no
/// version, a missing field or one of the wrong type) or whose values
/// cannot be a pilot's (a date the calendar does not have).
#[allow(clippy::too_many_lines)] // one field of the pilot for each of the save's
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
    let story = story_of(saved.story)?;
    Ok(Pilot {
        name: saved.name,
        nickname: saved.nickname,
        ship: ShipId(saved.ship),
        ship_name: saved.ship_name,
        system: SystemId(saved.system),
        stellar: saved.stellar.map(StellarId),
        date,
        cash: saved.cash,
        reserves: saved.reserves.into(),
        course: saved.course.into_iter().map(SystemId).collect(),
        course_start: saved.course_start.map(SystemId),
        explored: explored_of(&saved.explored, &saved.charted),
        charted: saved.charted.into_iter().map(SystemId).collect(),
        reinforcement_days: reinforcement_days_of(saved.reinforcement_days),
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
        cargo: saved.cargo.iter().map(cargo_of).collect(),
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
                mark: saved.mark.map(EscortMark::from),
                voice: saved.voice.map(VoiceVariant::from),
                berth: saved.berth,
            })
            .collect(),
        class_orders: saved.class_orders.map(class_word_of),
        gone_persons: saved.gone_persons.into_iter().map(PersonId).collect(),
        grudges: saved.grudges.into_iter().map(PersonId).collect(),
        bits: saved.bits.into_iter().collect(),
        gender: saved.gender.into(),
        strict_play: saved.strict_play,
        jumped_from: saved.jumped_from.map(SystemId),
        paint: saved.paint.map(Rgb15::from),
        story_fired: saved
            .story_fired
            .into_iter()
            .map(SubscriptionId::from)
            .collect(),
        galaxy: saved.galaxy.into(),
        missions,
        crons: crons_of(saved.crons),
        ranks: saved.ranks.into_iter().map(RankId).collect(),
        seen_intro: saved.seen_intro,
        defence_ships: defence_ships_of(&saved.defence_ships),
        flags: flags_of(saved.flags),
        dialogs: dialogs_of(saved.dialogs),
        story,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::upgrade::add_to_each;
    use super::*;
    use nova_sim_ai::escort::EscortOrder;
    use nova_sim_base::reserves::{Gauge, Reserves};
    use nova_sim_catalog::catalog::{CronId, MissionId};
    use nova_sim_catalog::catalog::{
        DisasterId, GovtId, JunkId, NebulaId, OutfitId, PersonId, RankId, ShipId, StellarId,
        SystemId,
    };
    use nova_sim_catalog::control::Bit;
    use nova_sim_catalog::control::ControlBitSet;
    use nova_sim_combat::fleet::OrderWord;
    use nova_sim_state::cron::CronState;
    use nova_sim_state::date::GameDate;
    use nova_sim_state::market::Good;
    use nova_sim_story::mission::aux_ships::AuxShips;
    use nova_sim_story::mission::{
        ActiveMission, ActiveMissions, FailedBy, ShipSystem, SpecialShips, Stop,
    };
    use nova_sim_story::pilot::Gender;
    use nova_sim_story::pilot::{Escort, ExploredLevel};
    use nova_sim_story::story::{Consumer, Countdown, GalaxyOverlay, SubscriptionId};

    /// An event under way with `days` left, at `stellar`.
    pub(super) fn active(days: u16, stellar: Option<StellarId>) -> ActiveEvent {
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

    /// The escorts of [`seasoned`]: each order, carried, hired and a
    /// person.
    fn seasoned_escorts() -> Vec<Escort> {
        vec![
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
                mark: None,
                voice: None,
                berth: None,
            },
            Escort {
                ship: ShipId(141),
                reserves: Reserves::full(1000.0, 500.0, 400.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
                voice: None,
                berth: None,
            },
            Escort {
                ship: ShipId(144),
                reserves: Reserves::full(60.0, 40.0, 300.0),
                order: Some(EscortOrder::Dock),
                carried: true,
                wage: None,
                person: None,
                mark: None,
                voice: None,
                berth: None,
            },
            Escort {
                ship: ShipId(128),
                reserves: Reserves::full(30.0, 45.0, 300.0),
                order: Some(EscortOrder::Hold),
                carried: false,
                wage: Some(100),
                person: None,
                mark: None,
                voice: None,
                berth: None,
            },
            Escort {
                ship: ShipId(136),
                reserves: Reserves::full(200.0, 100.0, 300.0),
                order: None,
                carried: false,
                wage: None,
                person: Some(PersonId(128)),
                mark: None,
                voice: None,
                berth: None,
            },
        ]
    }

    /// A pilot with every field away from its default.
    pub(super) fn seasoned() -> Pilot {
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
            course_start: Some(SystemId(131)),
            explored: BTreeSet::from([SystemId(130), SystemId(131), SystemId(200)]),
            charted: BTreeSet::from([SystemId(131)]),
            reinforcement_days: BTreeMap::from([(SystemId(130), 2), (SystemId(200), 15)]),
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
            escorts: seasoned_escorts(),
            class_orders: [OrderWord(3), OrderWord(0), OrderWord(2), OrderWord(-1)],
            gone_persons: BTreeSet::from([PersonId(151), PersonId(600)]),
            grudges: BTreeSet::from([PersonId(510)]),
            bits: [0, 512, 9999].into_iter().filter_map(Bit::new).collect(),
            gender: Gender::Female,
            strict_play: false,
            jumped_from: Some(SystemId(130)),
            paint: Some(Rgb15 { r: 31, g: 0, b: 4 }),
            story_fired: all_fired(),
            galaxy: overlaid(),
            missions: busy(),
            crons: holding_off(),
            ranks: BTreeSet::from([RankId(140), RankId(128)]),
            seen_intro: true,
            nickname: String::new(),
            defence_ships: nova_sim_ai::traffic::stellar_defence::DefenceShips::default(),
            flags: nova_sim_story::flags::WorldFlags::default(),
            dialogs: nova_sim_story::dialog::states::DialogStates::default(),
            story: nova_sim_story::storylines::state::StoryState::default(),
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
                    destroyed: 2,
                    boarded: 1,
                    disabled: 4,
                    chased_off: 5,
                    uniform_type: Some(ShipId(137)),
                    ship_done: false,
                },
                aux: AuxShips {
                    countdown: 33,
                    arrived: 1,
                    left: -2,
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
                failed: Some(FailedBy::Marked),
                ships: SpecialShips {
                    system: ShipSystem::FollowsPlayer,
                    ship_done: true,
                    ..SpecialShips::default()
                },
                aux: AuxShips::default(),
            },
        );
        missions
    }

    /// A once-only subscription of every consumer fired, out of order.
    fn all_fired() -> BTreeSet<SubscriptionId> {
        BTreeSet::from([
            fired(Consumer::Plugins, 128, 0),
            fired(Consumer::Offers, 0, 0),
            fired(Consumer::Missions, 3, 1),
            fired(Consumer::MissionShips, 15, 2),
            fired(Consumer::Missions, 3, 0),
            fired(Consumer::Crons, -5, 9),
            fired(Consumer::Storylines, 0, 1),
            fired(Consumer::Nebulae, 200, 0),
            fired(Consumer::Fixture, 1, 255),
        ])
    }

    pub(super) fn fired(consumer: Consumer, record: i16, hook: u8) -> SubscriptionId {
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
            charted: BTreeSet::new(),
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
        assert_eq!(CURRENT, 41);
        assert_eq!(value["nickname"], "");
        assert_eq!(value["seen_intro"], true);
        assert_eq!(value["course_start"], 131);
        assert_eq!(value["explored"], serde_json::json!([130, 131, 200]));
        assert_eq!(value["charted"], serde_json::json!([131]));
        assert_eq!(
            value["reinforcement_days"],
            serde_json::json!([{"system": 130, "days": 2}, {"system": 200, "days": 15}])
        );
        assert_eq!(
            value["galaxy"],
            serde_json::json!({
                "systems": [
                    {"system": 131, "visible": false, "govt": null},
                    {"system": 200, "visible": null, "govt": {"govt": null}},
                    {"system": 300, "visible": true, "govt": {"govt": 140}}
                ],
                "stellars": [
                    {"stellar": 150, "govt": {"govt": null}, "destroyed": null, "dominated": false},
                    {"stellar": 151, "govt": {"govt": 128},
                        "destroyed": {"days_left": 3}, "dominated": false},
                    {"stellar": 152, "govt": null, "destroyed": {"days_left": 0}, "dominated": false}
                ],
                "nebulae": [
                    {"nebula": 128, "active": false},
                    {"nebula": 129, "active": true}
                ]
            }),
            "by ID"
        );
        assert_eq!(value["stock_weapons_fitted"], true);
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
                "person": null,
                "mark": null,
                "voice": null,
                "berth": null
            })
        );
        assert_eq!(value["strict_play"], false);
        assert_eq!(value["jumped_from"], 130);
        assert!(text.contains("\n  \"version\": 41"), "{text}");
    }

    #[test]
    fn a_save_lists_each_escorts_own_fields() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(value["escorts"][4]["person"], 128);
        assert_eq!(value["escorts"][1]["ship"], 141);
        assert_eq!(value["escorts"][1]["order"], serde_json::Value::Null);
        assert_eq!(value["escorts"][2]["ship"], 144);
        assert_eq!(value["escorts"][2]["order"], "dock");
        assert_eq!(value["escorts"][2]["carried"], true);
        assert_eq!(value["escorts"][2]["wage"], serde_json::Value::Null);
        assert_eq!(value["escorts"][3]["ship"], 128);
        assert_eq!(value["escorts"][3]["wage"], 100);
    }

    #[test]
    fn the_fired_subscriptions_are_saved_by_consumer_record_and_hook() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(
            value["story_fired"],
            serde_json::json!([
                {"consumer": "offers", "record": 0, "hook": 0},
                {"consumer": "mission_ships", "record": 15, "hook": 2},
                {"consumer": "missions", "record": 3, "hook": 0},
                {"consumer": "missions", "record": 3, "hook": 1},
                {"consumer": "storylines", "record": 0, "hook": 1},
                {"consumer": "crons", "record": -5, "hook": 9},
                {"consumer": "nebulae", "record": 200, "hook": 0},
                {"consumer": "plugins", "record": 128, "hook": 0},
                {"consumer": "fixture", "record": 1, "hook": 255}
            ]),
            "by consumer, record and hook"
        );
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
    fn strict_play_either_way_survives_a_round_trip() {
        for strict_play in [false, true] {
            let pilot = seasoned().with_strict_play(strict_play);
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{strict_play}");
        }
    }

    #[test]
    fn a_current_save_missing_strict_play_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value
            .as_object_mut()
            .expect("an object")
            .remove("strict_play");
        let text = value.to_string();
        assert!(
            unusable(&text).contains("strict_play"),
            "{}",
            unusable(&text)
        );
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
    fn the_records_in_each_system_are_saved_by_system() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(
            value["system_legal"],
            serde_json::json!([{"system": 130, "record": -12}, {"system": 131, "record": 7}])
        );
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

    /// Crön 156 in its pre-holdoff, and crön 129 in its post-holdoff.
    fn holding_off() -> BTreeMap<CronId, CronState> {
        BTreeMap::from([
            (
                CronId(156),
                CronState {
                    days: 105,
                    holdoff: 2,
                },
            ),
            (
                CronId(129),
                CronState {
                    days: -1,
                    holdoff: 3,
                },
            ),
        ])
    }

    #[test]
    fn the_crons_running_or_holding_off_are_saved_by_id() {
        let pilot = seasoned();
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["version"], CURRENT);
        assert_eq!(
            value["crons"],
            serde_json::json!([
                {"cron": 129, "days": -1, "holdoff": 3},
                {"cron": 156, "days": 105, "holdoff": 2}
            ])
        );
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn an_inactive_cron_is_left_out_of_the_save() {
        let mut crons = holding_off();
        crons.insert(
            CronId(200),
            CronState {
                days: -1,
                holdoff: -1,
            },
        );
        let pilot = Pilot {
            crons,
            ..seasoned()
        };
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(
            value["crons"]
                .as_array()
                .expect("a list")
                .iter()
                .map(|cron| cron["cron"].clone())
                .collect::<Vec<_>>(),
            [129, 156]
        );
    }

    /// The current save of `seasoned`, its crons `crons`.
    fn with_crons(crons: serde_json::Value) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["crons"] = crons;
        value.to_string()
    }

    #[test]
    fn a_saved_cron_is_active_while_its_days_or_holdoff_are_0_or_more() {
        let pilot = decode(&with_crons(serde_json::json!([
            {"cron": 128, "days": -1, "holdoff": -1},
            {"cron": 129, "days": 0, "holdoff": -1},
            {"cron": 130, "days": -1, "holdoff": 0},
            {"cron": 131, "days": -2, "holdoff": -5}
        ])))
        .expect("loads");
        assert_eq!(
            pilot.crons(),
            &BTreeMap::from([
                (
                    CronId(129),
                    CronState {
                        days: 0,
                        holdoff: -1
                    }
                ),
                (
                    CronId(130),
                    CronState {
                        days: -1,
                        holdoff: 0
                    }
                ),
            ]),
            "_LoadPilotData @0x75fbc-0x76025"
        );
    }

    #[test]
    fn a_current_save_missing_its_crons_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(object) = value.as_object_mut() {
            object.remove("crons");
        }
        assert!(unusable(&value.to_string()).contains("crons"));
    }

    #[test]
    fn the_active_ranks_are_saved_by_id() {
        let pilot = seasoned();
        let value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        assert_eq!(value["ranks"], serde_json::json!([128, 140]));
        assert_eq!(decode(&encode(&pilot)), Ok(pilot));
    }

    #[test]
    fn a_current_save_missing_its_ranks_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        if let Some(object) = value.as_object_mut() {
            object.remove("ranks");
        }
        assert!(unusable(&value.to_string()).contains("ranks"));
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
                        "subtitle": "Hunter",
                        "destroyed": 2,
                        "boarded": 1,
                        "disabled": 4,
                        "chased_off": 5,
                        "uniform_type": 137,
                        "ship_done": false
                    },
                    "aux": {"countdown": 33, "arrived": 1, "left": -2}
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
                        "subtitle": "",
                        "destroyed": 0,
                        "boarded": 0,
                        "disabled": 0,
                        "chased_off": 0,
                        "uniform_type": null,
                        "ship_done": true
                    },
                    "aux": {"countdown": 0, "arrived": 0, "left": 0}
                }
            ])
        );
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
        assert_eq!(
            back.missions().get(2).map(|active| active.ships.chased_off),
            Some(5)
        );
        assert_eq!(
            back.missions().get(2).map(|active| active.aux),
            Some(AuxShips {
                countdown: 33,
                arrived: 1,
                left: -2,
            })
        );
    }

    #[test]
    fn a_mission_ships_uniform_type_survives_a_save_and_load() {
        let pilot = seasoned();
        let back = decode(&encode(&pilot)).expect("loads");
        let uniform = |slot| {
            back.missions()
                .get(slot)
                .map(|active| active.ships.uniform_type)
        };
        assert_eq!(uniform(2), Some(Some(ShipId(137))));
        assert_eq!(uniform(9), Some(None));
    }

    #[test]
    fn a_missions_ship_done_survives_a_save_and_load() {
        let pilot = seasoned();
        let back = decode(&encode(&pilot)).expect("loads");
        let done = |slot| {
            back.missions()
                .get(slot)
                .map(|active| active.ships.ship_done)
        };
        assert_eq!(done(2), Some(false));
        assert_eq!(done(9), Some(true));
        assert_eq!(back, pilot);
    }

    /// A pilot whose second escort is marked for sale and fourth for an
    /// upgrade.
    pub(super) fn marked() -> Pilot {
        let mut pilot = seasoned();
        pilot.escorts[1].mark = Some(EscortMark::Sell);
        pilot.escorts[3].mark = Some(EscortMark::Upgrade);
        pilot
    }

    #[test]
    fn an_escorts_mark_survives_a_round_trip() {
        let pilot = marked();
        let text = encode(&pilot);
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let marks: Vec<_> = (0..5)
            .map(|i| value["escorts"][i]["mark"].clone())
            .collect();
        assert_eq!(
            marks,
            [
                serde_json::Value::Null,
                "sell".into(),
                serde_json::Value::Null,
                "upgrade".into(),
                serde_json::Value::Null
            ]
        );
        assert_eq!(decode(&text), Ok(pilot));
    }

    #[test]
    fn a_current_save_missing_an_escorts_mark_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_33(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        let text = value.to_string();
        assert!(unusable(&text).contains("mark"), "{}", unusable(&text));
    }

    /// [`seasoned`] dominating stellar 150, with stellar 151's defence
    /// ships left at 3 of `DefCount` 7006 and 152's at 0 of 5.
    pub(super) fn dominating() -> Pilot {
        let mut pilot = seasoned();
        pilot.galaxy.set_dominated(StellarId(150), true);
        pilot.defence_ships.set_left_of(StellarId(151), 7006, 3);
        pilot.defence_ships.set_left_of(StellarId(152), 5, 0);
        pilot
    }

    #[test]
    fn a_domination_and_the_defence_ships_left_survive_a_round_trip() {
        let pilot = dominating();
        let text = encode(&pilot);
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            value["galaxy"]["stellars"],
            serde_json::json!([
                {"stellar": 150, "govt": {"govt": null}, "destroyed": null, "dominated": true},
                {"stellar": 151, "govt": {"govt": 128}, "destroyed": {"days_left": 3}, "dominated": false},
                {"stellar": 152, "govt": null, "destroyed": {"days_left": 0}, "dominated": false}
            ])
        );
        assert_eq!(
            value["defence_ships"],
            serde_json::json!([
                {"stellar": 151, "left": 3, "def_count": 7006},
                {"stellar": 152, "left": 0, "def_count": 5}
            ])
        );
        assert_eq!(decode(&text), Ok(pilot));
    }

    #[test]
    fn a_current_save_missing_a_stellars_domination_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_34(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        let text = value.to_string();
        assert!(unusable(&text).contains("dominated"), "{}", unusable(&text));
    }

    #[test]
    fn the_nickname_survives_a_round_trip() {
        let pilot = seasoned().with_nickname("Hunter".to_owned());
        let back = decode(&encode(&pilot)).expect("loads");
        assert_eq!(back.nickname(), "Hunter");
        assert_eq!(back, pilot);
    }

    #[test]
    fn a_current_save_missing_nickname_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_32(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        add_to_each(&mut value, "escorts", "mark", &serde_json::Value::Null);
        let text = value.to_string();
        assert!(unusable(&text).contains("nickname"), "{}", unusable(&text));
    }

    #[test]
    fn the_course_start_survives_a_round_trip_either_way() {
        for course_start in [None, Some(SystemId(130))] {
            let pilot = Pilot {
                course_start,
                ..seasoned()
            };
            let back = decode(&encode(&pilot)).expect("loads");
            assert_eq!(back.course_start(), course_start);
            assert_eq!(back, pilot, "{course_start:?}");
        }
    }

    #[test]
    fn a_current_save_missing_course_start_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_31(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        add_to_each(&mut value, "escorts", "mark", &serde_json::Value::Null);
        value["nickname"] = serde_json::json!("");
        let text = value.to_string();
        assert!(
            unusable(&text).contains("course_start"),
            "{}",
            unusable(&text)
        );
    }

    #[test]
    fn whether_the_intro_screen_was_seen_survives_a_round_trip_either_way() {
        for seen_intro in [false, true] {
            let pilot = Pilot {
                seen_intro,
                ..seasoned()
            };
            let back = decode(&encode(&pilot)).expect("loads");
            assert_eq!(back.seen_intro(), seen_intro);
            assert_eq!(back, pilot, "{seen_intro}");
        }
    }

    #[test]
    fn a_current_save_missing_seen_intro_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_30(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        add_to_each(&mut value, "escorts", "mark", &serde_json::Value::Null);
        value["course_start"] = serde_json::json!(131);
        value["nickname"] = serde_json::json!("");
        let text = value.to_string();
        assert!(
            unusable(&text).contains("seen_intro"),
            "{}",
            unusable(&text)
        );
    }

    #[test]
    fn the_reinforcement_countdowns_survive_a_round_trip() {
        let pilot = seasoned();
        let back = decode(&encode(&pilot)).expect("loads");
        assert_eq!(
            back.reinforcement_days,
            BTreeMap::from([(SystemId(130), 2), (SystemId(200), 15)])
        );
        assert_eq!(back, pilot);
    }

    #[test]
    fn a_saved_countdown_of_none_loads_as_none_kept() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["reinforcement_days"] = serde_json::json!([
            {"system": 130, "days": 0},
            {"system": 131, "days": 4}
        ]);
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(
            pilot.reinforcement_days,
            BTreeMap::from([(SystemId(131), 4)])
        );
    }

    #[test]
    fn a_current_save_missing_reinforcement_days_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value
            .as_object_mut()
            .expect("an object")
            .remove("reinforcement_days");
        let text = value.to_string();
        assert!(
            unusable(&text).contains("reinforcement_days"),
            "{}",
            unusable(&text)
        );
    }

    #[test]
    fn the_explored_levels_survive_a_round_trip() {
        let pilot = seasoned();
        let back = decode(&encode(&pilot)).expect("loads");
        assert_eq!(back.explored_level(SystemId(131)), ExploredLevel::Charted);
        assert_eq!(back.explored_level(SystemId(130)), ExploredLevel::Explored);
        assert_eq!(back, pilot);
    }

    #[test]
    fn a_charted_system_missing_from_the_explored_loads_explored_too() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["explored"] = serde_json::json!([130]);
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(pilot.explored_level(SystemId(131)), ExploredLevel::Charted);
        assert_eq!(
            pilot.explored().collect::<Vec<_>>(),
            [SystemId(130), SystemId(131)]
        );
    }

    #[test]
    fn a_current_save_missing_charted_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value.as_object_mut().expect("an object").remove("charted");
        let text = value.to_string();
        assert!(unusable(&text).contains("charted"), "{}", unusable(&text));
    }

    #[test]
    fn a_current_save_missing_ship_done_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_27(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        add_to_each(&mut value, "escorts", "mark", &serde_json::Value::Null);
        value["charted"] = serde_json::json!([]);
        value["reinforcement_days"] = serde_json::json!([]);
        value["seen_intro"] = serde_json::json!(true);
        value["course_start"] = serde_json::json!(131);
        value["nickname"] = serde_json::json!("");
        let text = value.to_string();
        assert!(unusable(&text).contains("ship_done"), "{}", unusable(&text));
    }

    #[test]
    fn jumped_from_either_way_survives_a_round_trip() {
        for jumped_from in [None, Some(SystemId(130))] {
            let pilot = Pilot {
                jumped_from,
                ..seasoned()
            };
            assert_eq!(decode(&encode(&pilot)), Ok(pilot), "{jumped_from:?}");
        }
    }

    #[test]
    fn a_current_save_missing_jumped_from_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value
            .as_object_mut()
            .expect("an object")
            .remove("jumped_from");
        let text = value.to_string();
        assert!(
            unusable(&text).contains("jumped_from"),
            "{}",
            unusable(&text)
        );
    }

    #[test]
    fn a_current_save_missing_uniform_type_is_unusable() {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_25(&seasoned())).expect("JSON");
        value["version"] = serde_json::Value::from(CURRENT);
        add_to_each(&mut value, "escorts", "mark", &serde_json::Value::Null);
        let text = value.to_string();
        assert!(
            unusable(&text).contains("uniform_type"),
            "{}",
            unusable(&text)
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
    fn a_nebulas_explored_flag_is_never_saved() {
        let mut overlay = GalaxyOverlay::default();
        overlay.set_nebula_explored(NebulaId(128), true);
        overlay.set_nebula_active(NebulaId(129), Some(false));
        overlay.set_nebula_explored(NebulaId(129), true);
        let pilot = Pilot {
            galaxy: overlay,
            ..seasoned()
        };
        let text = encode(&pilot);
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            value["galaxy"]["nebulae"],
            serde_json::json!([{"nebula": 129, "active": false}]),
            "no explored key, and 128, only explored, not listed"
        );
        let decoded = decode(&text).expect("decodes").galaxy;
        assert_eq!(decoded.nebula(NebulaId(129)), Some(false));
        assert!(!decoded.nebula_explored(NebulaId(128)));
        assert!(!decoded.nebula_explored(NebulaId(129)));
        let mut expected = GalaxyOverlay::default();
        expected.set_nebula_active(NebulaId(129), Some(false));
        assert_eq!(decoded, expected, "nothing explored");
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
        let newer = encode(&seasoned()).replace("\"version\": 41", "\"version\": 42");
        assert_eq!(decode(&newer), Err(SaveError::Newer { version: 42 }));
        assert_eq!(
            SaveError::Newer { version: 42 }.to_string(),
            "This pilot file was created with a different version of Nova, and can't be used \
             (it is version 42, and this version of Nova reads up to 41)."
        );
    }

    /// `pilot` flying a carried fighter of `voice`, and a hired escort.
    pub(super) fn with_a_fighter(mut pilot: Pilot, voice: VoiceVariant) -> Pilot {
        let escort = pilot.escorts.first().copied().map_or_else(
            || Escort {
                ship: ShipId(136),
                reserves: Reserves::full(10.0, 10.0, 100.0),
                order: None,
                carried: false,
                wage: None,
                person: None,
                mark: None,
                voice: None,
                berth: None,
            },
            |escort| Escort {
                voice: None,
                berth: None,
                ..escort
            },
        );
        pilot.escorts = vec![
            Escort {
                carried: true,
                voice: Some(voice),
                ..escort
            },
            Escort {
                carried: false,
                ..escort
            },
        ];
        pilot
    }

    #[test]
    fn a_hired_escort_saves_null_whatever_its_record_holds() {
        let mut pilot = with_a_fighter(seasoned(), VoiceVariant::Odd);
        pilot.escorts[1].voice = Some(VoiceVariant::Odd);
        let text = encode(&pilot);
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["escorts"][0]["voice"], "odd");
        assert_eq!(value["escorts"][1]["voice"], serde_json::Value::Null);
        let decoded = decode(&text).expect("loads");
        assert_eq!(decoded.escorts[1].voice, None);
    }

    #[test]
    fn a_carried_fighters_variant_round_trips() {
        for voice in [VoiceVariant::Even, VoiceVariant::Odd] {
            let pilot = with_a_fighter(seasoned(), voice);
            let text = encode(&pilot);
            assert_eq!(decode(&text), Ok(pilot.clone()));
            let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
            assert_eq!(
                value["escorts"][0]["voice"],
                match voice {
                    VoiceVariant::Even => "even",
                    VoiceVariant::Odd => "odd",
                }
            );
        }
    }

    #[test]
    fn a_save_missing_an_escorts_voice_at_the_current_version_is_refused() {
        let pilot = with_a_fighter(seasoned(), VoiceVariant::Odd);
        let mut value: serde_json::Value = serde_json::from_str(&encode(&pilot)).expect("JSON");
        value["escorts"][0]
            .as_object_mut()
            .expect("an escort")
            .remove("voice");
        assert!(unusable(&value.to_string()).contains("voice"));
    }

    // Each escort's berth, the slot `_WritePilotData` indexes its tables by
    // (@0x725f6-0x72711).

    #[test]
    fn an_escorts_berth_round_trips_none_included() {
        let mut pilot = seasoned();
        pilot.escorts[0].berth = Some(3);
        pilot.escorts[2].berth = Some(1);
        let text = encode(&pilot);
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["escorts"][0]["berth"], 3);
        assert_eq!(value["escorts"][1]["berth"], serde_json::Value::Null);
        assert_eq!(value["escorts"][2]["berth"], 1);
        assert_eq!(decode(&text), Ok(pilot));
    }

    #[test]
    fn a_save_missing_an_escorts_berth_at_the_current_version_is_refused() {
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["escorts"][0]
            .as_object_mut()
            .expect("an escort")
            .remove("berth");
        assert!(unusable(&value.to_string()).contains("berth"));
    }

    // The class table (`_WritePilotData` @0x72b01-0x72b25, `_LoadPilotData`
    // @0x760c2-0x760ea).

    #[test]
    fn a_save_keeps_the_class_table_raw_in_class_order() {
        let value: serde_json::Value = serde_json::from_str(&encode(&seasoned())).expect("JSON");
        assert_eq!(value["class_orders"], serde_json::json!([3, 0, 2, -1]));
    }

    #[test]
    fn a_saved_negative_class_word_loads_unset_and_any_other_raw() {
        // `js` @0x760de.
        let mut value: serde_json::Value =
            serde_json::from_str(&encode(&seasoned())).expect("JSON");
        value["class_orders"] = serde_json::json!([-7, 5, -1, 0]);
        let pilot = decode(&value.to_string()).expect("loads");
        assert_eq!(
            pilot.class_orders,
            [OrderWord(-1), OrderWord(5), OrderWord(-1), OrderWord(0)]
        );
    }

    pub(super) fn unusable(text: &str) -> String {
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

    /// `pilot` saved at version 25: no mission's ships keep a type, and
    /// no system jumped from.
    pub(super) fn at_version_25(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_26(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(25);
        for mission in value["missions"].as_array_mut().expect("a list") {
            mission["ships"]
                .as_object_mut()
                .expect("an object")
                .remove("uniform_type");
        }
        value.to_string()
    }

    /// `pilot` saved at version 27: no mission keeps whether its
    /// goals-met moment has come, and no system is kept charted.
    pub(super) fn at_version_27(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_28(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(27);
        for mission in value["missions"].as_array_mut().expect("a list") {
            mission["ships"]
                .as_object_mut()
                .expect("an object")
                .remove("ship_done");
        }
        value.to_string()
    }

    /// `pilot` saved at version 30: whether it has seen the intro screen
    /// not kept, nor the course's start.
    pub(super) fn at_version_30(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_31(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(30);
        value
            .as_object_mut()
            .expect("an object")
            .remove("seen_intro");
        value.to_string()
    }

    /// `pilot` saved at version 31: the course's start not kept, nor the
    /// nickname.
    pub(super) fn at_version_31(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_32(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(31);
        value
            .as_object_mut()
            .expect("an object")
            .remove("course_start");
        value.to_string()
    }

    /// `pilot` saved at version 32: no nickname kept, nor any escort's
    /// mark.
    pub(super) fn at_version_32(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_33(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(32);
        value.as_object_mut().expect("an object").remove("nickname");
        value.to_string()
    }

    /// `pilot` saved at version 33: no escort's mark kept.
    pub(super) fn at_version_33(pilot: &Pilot) -> String {
        let mut value: serde_json::Value = serde_json::from_str(&encode(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(33);
        for escort in value["escorts"].as_array_mut().expect("a list") {
            escort.as_object_mut().expect("an object").remove("mark");
        }
        value.to_string()
    }

    /// `pilot` saved at version 34: no domination or defence ships left
    /// kept.
    pub(super) fn at_version_34(pilot: &Pilot) -> String {
        let mut value: serde_json::Value = serde_json::from_str(&encode(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(34);
        for stellar in value["galaxy"]["stellars"].as_array_mut().expect("a list") {
            stellar
                .as_object_mut()
                .expect("an object")
                .remove("dominated");
        }
        value
            .as_object_mut()
            .expect("an object")
            .remove("defence_ships");
        value.to_string()
    }

    /// A version 1 save: before explored systems and legal records.
    pub(super) const VERSION_1: &str = r#"{
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

    /// `pilot` saved at version 26: no system jumped from, and no
    /// mission's ship done kept.
    pub(super) fn at_version_26(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_27(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(26);
        value
            .as_object_mut()
            .expect("an object")
            .remove("jumped_from");
        value.to_string()
    }

    /// `pilot` saved at version 28: no system is kept charted, and no
    /// reinforcement countdown.
    pub(super) fn at_version_28(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_29(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(28);
        value.as_object_mut().expect("an object").remove("charted");
        value.to_string()
    }

    /// `pilot` saved at version 29: no reinforcement countdown kept, nor
    /// whether it has seen the intro screen.
    pub(super) fn at_version_29(pilot: &Pilot) -> String {
        let mut value: serde_json::Value =
            serde_json::from_str(&at_version_30(pilot)).expect("JSON");
        value["version"] = serde_json::Value::from(29);
        value
            .as_object_mut()
            .expect("an object")
            .remove("reinforcement_days");
        value.to_string()
    }
}
