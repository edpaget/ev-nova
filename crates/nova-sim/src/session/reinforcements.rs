//! The system's reinforcement fleet in a session: the calls the ships make
//! for it (`_AICallForReinforcements` @0x7f5a1-0x7f71e in the `EV Nova`
//! executable), the clock that brings it (`_HandleReinforcements`
//! @0x39048-0x392c3), the daily countdown (`_IncrementGameTime`
//! @0xb724-0xb759) and the inhibitors (`_ResetPlayerPrecalcedValues`
//! @0xc628-0xc68f). The rules are [`reinforcements`](crate::ai::reinforcements)'s;
//! this module gathers their facts and keeps their state.
//!
//! **Where each part runs**, against the original:
//!
//! | Original | Engine |
//! |---|---|
//! | `_BraveTraderAI` @0x8b6dc, `_HighLevelAIHandler` @0x8e0d7 and @0x8e70d, in each think | the think's [`Decision::calls_reinforcements`](crate::ai::Decision::calls_reinforcements), answered in slot order with the frame's other calls ([`Session::settle_line_queue`]), unforced, at the odds its think read |
//! | `_DoCommDialog` @0x9636d, battle help | the hail's Defend, forced (`Session::act`), at the ship's cached odds |
//! | `_AICallForReinforcements`' reads of +0xc914 (@0x7f60d, @0x7f69b) | the caller's cached odds ([`Npc::odds`](crate::traffic::npc::Npc::odds)) by the engine's reading of [`RuleKey::OddsCache`], worked out fresh by the other |
//! | `_HandleReinforcements` @0x44504, on both passes of `_DoPlayGameWork` | [`Session::tick_reinforcements`]: in [`Session::tick_traffic`] after the frame's calls, on every tick in flight, a jump's included, or held behind calls held behind a scan into [`Session::tick_combat`] (see the `comm_calls` module); and after double time's second pass ([`Session::tick_again`]) |
//! | `_IncrementGameTime` @0xb724-0xb759 | the daily step's reinforcements ([`Session::reinforcements_day`]) |
//! | `_ResetPlayerPrecalcedValues` @0xc628-0xc68f | [`Session::reset_precalced`], after the sabotage clock and before the trim |
//! | `_WarshipAI` @0x8bd36-0x8bdc8, `_InterceptorAI` @0x8d1de-0x8d27e | the coming fleet in the traffic's [`World`](crate::traffic::World) ([`Session::reinforcements_coming`]) |
//! | `_ResetPlayer` @0x1db17-0x1db30, the escape pod's `_ResetPlayer(0)` @0x68f2f | the rescue's reset ([`Session::reset_reinforcements_here`]) |
//! | `_ResetPlayer(1)` @0x1d9e8-0x1d9f4, before `_LoadPilotData` @0x76027-0x7605b | a session built afresh: no timer runs, and the saved countdowns are the pilot's ([`Pilot::reinforcement_days`](crate::pilot::Pilot::reinforcement_days)) |
//!
//! **The state.** Each system's timer (+0xc8, not saved, as the
//! original's isn't) is kept here, by system, so a departed system's timer
//! stands until the player returns, as only the player's system counts
//! down (`_HandleReinforcements` reads player +0x74), and nothing else
//! reads another system's. Each system's countdown (+0xc4) is the pilot's,
//! saved. The system's fleet, its `ReinfTime` and `ReinfIntrval` are the
//! traffic's table's ([`Traffic::reinforcement_fleet`](crate::traffic::Traffic::reinforcement_fleet)).
//!
//! **The line.** On the frame the fleet is announced, `snd ` 154 plays
//! ([`SimSound::ReinforcementsApproaching`], `_PlaySound` @0x3924b) and
//! then the line shows for 240 ticks
//! ([`SimMessage::ReinforcementsApproaching`], `_DisplayComm` @0x3928a):
//! `STR#` 2002 #306, a space, the fleet government's `MediumName`, a space
//! and #307, a missing entry read as empty. A government with no
//! `MediumName` announces nothing (@0x3914d). On its arrival the fleet's
//! own `Quote`, if it has one, shows on the tick it comes
//! ([`Session::announce_fleets`]).
//!
//! **The inhibitors.** Each reset sets the global inhibitor for an owned
//! `ModType` 44 outfit of `ModVal` -1, and every government whose
//! `Class1-4` holds another `ModVal` (@0xc630-0xc68f). By the engine's
//! reading of [`RuleKey::ReinforcementInhibitor`] they are never cleared,
//! as only `_InitObjects` clears them (@0x1d017, @0x1d064); by the other
//! they are worked out afresh at each reset.
//!
//! *Ordering against the original.* The original calls inside each
//! think, and counts its clock down after the frame's ships have moved;
//! the engine answers the calls after the frame's decisions and ticks the
//! clock after them, both before the frame is drawn, so a call and its
//! first tick fall on the same frame in both.
//!
//! *A jump's frames.* The original's clock counts down through every
//! frame of a jump for the system being left, until the arrival changes
//! player +0x74, and a fleet due then comes into it. The engine counts down
//! there too and brings the fleet into the traffic, which stands still
//! past the warp's lead-in and is dropped at the arrival.

use std::collections::{BTreeMap, BTreeSet};

use super::Session;
use crate::ai::odds::cached_odds_at;
use crate::ai::reinforcements::{
    self, Called, Clock, Coming, Event, Inhibitors, ReinforcementFacts,
};
use crate::catalog::{GovtId, SystemId, outfit_mods};
use crate::chance::Chance;
use crate::message::SimMessage;
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;
use crate::traffic::npc::NpcId;

/// The `STR#` of the reinforcement line's words (0x7d2 @0x39175).
pub const REINFORCEMENT_LINES: i16 = 2002;

/// The entry before the fleet government's name: "Sensors detect"
/// (0x132 @0x3916d).
pub const SENSORS_DETECT: usize = 306;

/// The entry after it: "reinforcement fleet approaching." (0x133
/// @0x39205).
pub const FLEET_APPROACHING: usize = 307;

/// An owned outfit's `ModType` that inhibits reinforcements (0x2c
/// @0xc628).
pub const INHIBITOR: i16 = 44;

/// The inhibitor's `ModVal` that inhibits every government (-1 @0xc630).
pub const EVERY_GOVERNMENT: i16 = -1;

/// The session's reinforcement state (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Reinforcements {
    /// Each system's timer (+0xc8), the ticks until its called fleet
    /// comes; a system not listed reads 0, none coming.
    pub(super) timers: BTreeMap<SystemId, i32>,
    /// The global inhibitor (`*0x3b510c`).
    pub(super) every_government: bool,
    /// The governments inhibited (+0x82).
    pub(super) inhibited: BTreeSet<GovtId>,
}

impl Session {
    /// The clock of the player's system: its timer and the pilot's
    /// countdown there.
    fn clock_here(&self) -> Clock {
        let system = self.pilot.system;
        Clock {
            timer: self
                .space
                .reinforcements
                .timers
                .get(&system)
                .copied()
                .unwrap_or(0),
            countdown: self
                .pilot
                .reinforcement_days
                .get(&system)
                .copied()
                .unwrap_or(0),
        }
    }

    /// Sets the clock of the player's system to `clock`.
    fn set_clock_here(&mut self, clock: Clock) {
        let system = self.pilot.system;
        if clock.timer == 0 {
            self.space.reinforcements.timers.remove(&system);
        } else {
            self.space.reinforcements.timers.insert(system, clock.timer);
        }
        if clock.countdown == 0 {
            self.pilot.reinforcement_days.remove(&system);
        } else {
            self.pilot
                .reinforcement_days
                .insert(system, clock.countdown);
        }
    }

    /// The inhibitors a call of a ship of `ship_govt` for a fleet of
    /// `fleet_govt` reads (@0x7f6b2-0x7f6ed).
    fn inhibitors(&self, ship_govt: Option<GovtId>, fleet_govt: Option<GovtId>) -> Inhibitors {
        let inhibited = |govt: Option<GovtId>| {
            govt.is_some_and(|govt| self.space.reinforcements.inhibited.contains(&govt))
        };
        Inhibitors {
            global: self.space.reinforcements.every_government,
            ship_govt: inhibited(ship_govt),
            fleet_govt: inhibited(fleet_govt),
        }
    }

    /// NPC `id` calls for its system's reinforcement fleet, `force`d past
    /// the strength test or not (`_AICallForReinforcements(ship, force)`,
    /// see the module docs): its facts gathered now. The odds against it
    /// are its slot's cached ones (+0xc914 @0x7f60d, @0x7f69b) by the
    /// engine's reading of [`RuleKey::OddsCache`]: `think_odds`, as its
    /// think read them, for a call a think made (the odds pass ran after
    /// the think, before this answer), else its cached odds now, as
    /// battle help's call between frames reads them. By the other reading
    /// they are worked out fresh now ([`cached_odds_at`],
    /// which the session only hands the think's odds). A ship not in the
    /// system calls nothing.
    pub(super) fn call_for_reinforcements(
        &mut self,
        id: NpcId,
        force: bool,
        think_odds: Option<f32>,
    ) {
        let fleet = self.space.traffic.reinforcement_fleet();
        let facts = {
            let world = self.world();
            let around = world.around(self.npcs());
            let Some(npc) = around.npc(id) else {
                return;
            };
            let fleet_govt = fleet.and_then(|fleet| fleet.govt);
            ReinforcementFacts {
                has_govt: npc.govt.is_some(),
                fleet,
                clock: self.clock_here(),
                odds: cached_odds_at(npc, &around, think_odds),
                allied: self.statics.govts.allies(npc.govt, fleet_govt),
                fleet_max_odds: self.statics.govts.max_odds(fleet_govt),
                force,
                inhibitors: self.inhibitors(npc.govt, fleet_govt),
            }
        };
        let called = reinforcements::call(&facts);
        if called != Called::No {
            let mut clock = facts.clock;
            clock.answer(called);
            self.set_clock_here(clock);
        }
    }

    /// A frame of the player's system's reinforcement clock
    /// (`_HandleReinforcements`, see the module docs), drawing the
    /// arriving fleet on `chance`: in flight, a jump's frames included,
    /// once the system is set up.
    pub(super) fn tick_reinforcements(&mut self, chance: &mut (impl Chance + ?Sized)) {
        if self.landed.is_some() || self.space.traffic_due {
            return;
        }
        let fleet = self.space.traffic.reinforcement_fleet();
        let name = fleet
            .and_then(|fleet| fleet.govt)
            .and_then(|govt| self.statics.govts.get(govt))
            .map(|record| record.medium_name.clone())
            .unwrap_or_default();
        let mut clock = self.clock_here();
        let event = reinforcements::tick(&mut clock, fleet, !name.is_empty());
        self.set_clock_here(clock);
        match event {
            Some(Event::Announce) => {
                let line = |entry| {
                    self.setup
                        .nth_string(REINFORCEMENT_LINES, entry)
                        .unwrap_or_default()
                };
                let text = format!(
                    "{} {name} {}",
                    line(SENSORS_DETECT),
                    line(FLEET_APPROACHING)
                );
                self.out.sounds.push(SimSound::ReinforcementsApproaching);
                self.say(SimMessage::ReinforcementsApproaching(text));
            }
            Some(Event::Arrive) => {
                self.step_traffic(|traffic, world| traffic.bring_reinforcements(&world, chance));
                self.announce_fleets(chance);
            }
            None => {}
        }
    }

    /// The fleet coming to the player's system, while its timer runs: what
    /// the warships' and interceptors' odds retreat read.
    pub(super) fn reinforcements_coming(&self) -> Option<Coming> {
        let fleet = self.space.traffic.reinforcement_fleet()?;
        self.clock_here()
            .coming()
            .then_some(Coming { govt: fleet.govt })
    }

    /// The daily step's reinforcements: each visible system's countdown
    /// goes down by one (@0xb724-0xb759).
    pub(super) fn reinforcements_day(&mut self) {
        let galaxy = &self.galaxy;
        for (system, days) in &mut self.pilot.reinforcement_days {
            if galaxy.visible(*system) {
                *days = reinforcements::day(*days);
            }
        }
        self.pilot.reinforcement_days.retain(|_, days| *days > 0);
    }

    /// `_ResetPlayer`'s part (@0x1db17-0x1db30): the player's system's
    /// timer and countdown cleared.
    pub(super) fn reset_reinforcements_here(&mut self) {
        self.set_clock_here(Clock::default());
    }

    /// The reset's inhibitors (@0xc628-0xc68f): set by each owned
    /// `ModType` 44 outfit, on top of those set before by the engine's
    /// reading of [`RuleKey::ReinforcementInhibitor`], in place of them by
    /// the other.
    pub(super) fn reset_inhibitors(&mut self) {
        if self.setup.rules.source_for(RuleKey::ReinforcementInhibitor) == RuleSource::Bible {
            self.space.reinforcements.every_government = false;
            self.space.reinforcements.inhibited.clear();
        }
        for owned in outfit_mods(&self.pilot.outfits, &self.statics.outfits) {
            if owned.mod_type != INHIBITOR {
                continue;
            }
            if owned.mod_val == EVERY_GOVERNMENT {
                self.space.reinforcements.every_government = true;
                continue;
            }
            let classed: Vec<GovtId> = self
                .statics
                .govts
                .ids()
                .filter(|&govt| {
                    self.statics
                        .govts
                        .get(govt)
                        .is_some_and(|record| record.classes.contains(&owned.mod_val))
                })
                .collect();
            self.space.reinforcements.inhibited.extend(classed);
        }
    }
}

#[cfg(test)]
mod tests;
