//! The session's side of mission ships (see the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships"):
//! their state drawn as a mission is accepted, their setup with the
//! system's ships, the enter-system hook that restarts their appear
//! timer, the timer and the ships it brings in, their goals (see the
//! [`story`](crate::story) module's "Ship goals"), and what becomes of
//! them as they are destroyed, disabled or boarded and as their mission
//! ends. A mission's auxiliary ships are the
//! [`aux_ships`](super::aux_ships) module's: their countdown is drawn at
//! accept, first, before the special ships' appear timer; the
//! enter-system hook and the take-off restart it; and each frame of
//! the timers brings them in before the special ships of the same slot.
//!
//! **The hooks** are no loop of their own: each slot filled registers
//! [`ship_hook`]s, `{MissionShips, slot, hook}`, with a blank guard and
//! no script: [`ENTERED`] on every `EnteredSystem`, [`LANDED`] on every
//! `Landed`, [`SHIP_FATE`] on every `ShipFate` and [`TOOK_OFF`] on every
//! `TookOff`. The enter-system hook's effect reads the event's
//! [`Entrance`]. It restarts the timer on a jump's arrival, a gate's or
//! wormhole's exit and an `M` or `N` in flight, never on a landed move
//! nor a relocation (`_MissionHandlePlayerEnteredNewSystem`
//! @0x99f5b-0x99fae). The original runs it before the system's ships are
//! set up (@0x6c15d before @0x6c1f9, @0x67159 before @0x672e0), so a
//! `ShipStart` 1 slot it restarts brings none at that setup. Here the
//! setup comes first, as the system is entered, and the hook as the
//! entry settles: the setup on such an entry ([`Session::populate`]
//! through the entry table) leaves out every slot the hook is about to
//! restart. Its timer is drawn after the setup's draws, not before. On a
//! hypergate's or wormhole's exit the original's loop A then overwrites
//! the restart of every slot whose ships follow the player (`ShipSyst`
//! -6, @0x6716a-0x67294), so, as the engine reads
//! [`RuleKey::GateFollow`], the hook leaves such a slot alone there
//! ([`Session::gate_decides`]).
//!
//! **Following through a gate** is the [`gate_follow`](super::gate_follow)
//! module's: at a gate's exit a -6 slot of escorts is set up at once, one
//! of attackers follows the player on its `düde` government's `Flags2`
//! or odds, its fresh ships waiting in the exit gate before they attack,
//! and any other loses its ships until the hook next restarts a
//! `ShipStart` 1 timer.
//!
//! **The goals** are checked by every hook but in docked entries, the
//! enter-system hook after its restart, as the original's per-frame poll
//! and its land check check them (`_MissionObjectivesCheck` @0x9e79e; see
//! [`check_goals`]): the landing's before the missions' landing hooks, as
//! [`Consumer::MissionShips`] sorts before [`Consumer::Missions`], and
//! after the offer roll; the take-off's as the first flight frame after
//! a docked accept or a landing; every `ShipFate`'s, a sighting of one of
//! the slot's own ships counting as seen. An accept in flight queues its
//! slot's check for the end of [`Session::settle_script`]'s settling, so
//! a mission an `S` starts is checked after the script that started it.
//! The take-off's check comes after the system it took off into is set
//! up, as the original sets it up (@0x674c2) before its first flight
//! frame: [`Session::take_off`] sets it up, its ships in and its
//! auto-aborts done, before the hook runs.
//! The moment the goals are met shows `ShipDoneText`, runs `OnShipDone`
//! and auto-aborts a mission of `Flags` 0x0001 ([`Session::auto_abort`]).
//! A mission of no goal is met at its first check, unless its ships come
//! later (`ShipStart` 1) with `Flags` 0x0001: then it waits for them
//! (see [`check_goals`]).
//!
//! **The auto-abort as the ships spawn**: a mission of `Flags` 0x0001
//! also auto-aborts, whatever its goal, once per slot as its special
//! ships spawn: after the system's setup, for each slot the setup took,
//! whether or not a ship came (`_SetupShipsInSystem` @0x43169-0x4317e),
//! and after its appear timer brought them in by hyperspace
//! (`_EnterMoreShips` @0x43abc-0x43ad4). Its ships stay in the system,
//! losing their marks as the next think finds the slot free. The
//! original aborts each slot inside setup's slot loop; here the aborts
//! follow once the system's ships are all in, in slot order.
//! Escorted ships count while they are in the system: both spawn paths
//! raise [`Fate::Arrived`] for each ship they bring in, and watch the
//! ships of an unmet observe goal for their first sighting
//! ([`Session::watch_ship`]).
//!
//! **The counts** go up where the original counts them: a ship destroyed
//! as it is taken out of the fight, one disabled as the fight's tick
//! downs it, one boarded on the pickup path or the assist path of
//! [`Session::board`] (a `PickupMode` 2 mission's whatever its goal, once
//! its cargo is aboard; a pickup refused refuses the board), and
//! one that leaves the system as the traffic's tick takes it out
//! ([`Fate::Departed`]), chased off and its ships left lowered as the
//! ships-left rule says ([`RuleKey::LeftRespawn`]; a landing leaves only
//! as [`RuleKey::LandedLeaves`] says, and a ship that leaves through a
//! gate departs but counts only as [`RuleKey::GateExitCounts`] says). The
//! first loss that fails its mission fails it there, at once
//! ([`SimMessage::MissionShipLost`] and the quick failure); the goal
//! checks follow as the `ShipFate` the loss raised settles.
//!
//! **No crime** against a marked ship changes the legal records: the
//! original's `_SlapWithPenalty` (@0x9bbc) slaps only for a mission slot
//! of -1 (@0x9c10), and disabling, destroying and boarding pass the
//! ship's mark (`_DamageShip` @0x3adb3, @0x3ae08;
//! `_HandlePlayerBoardAttempt` @0x6582b). It reads the mark alone, so a
//! ship of an active mission, of a failed one, and one whose slot is
//! freed with its mark not yet cleared all escape it
//! ([`Session::board`]; the fight's `punish`). A failed mission's ship is
//! plundered as it is boarded, but uncharged.
//!
//! **Rescue ships** (`ShipGoal` 5) are held disabled. The original's
//! `_IsDisabled` reads mission state: a marked ship (never the player)
//! whose slot is active, whose `ShipGoal` is 5, whose goals are not yet
//! met (+2) and which is not boarded (+0xb9) reads disabled at any
//! armour (@0x2d24-0x2d69); failed (+3) is not read. Here a condition is
//! stored, and the fight recomputes it from armour, so the session works
//! the hold out for each NPC ([`Session::mission_ship_held`], over
//! [`held_disabled`]) and hands it to the fight
//! ([`Fighter::held_disabled`](crate::combat::Fighter::held_disabled)),
//! which keeps a held ship disabled with no new disabling, so no
//! [`Fate::Disabled`] and no disabled count, as `_DamageShip` counts
//! only a ship newly disabled (@0x3a9ad-0x3a9b2); its armour does not
//! regenerate, as `_ShipArmorRechargeRate` gives none while disabled
//! (@0x2e1d). A held ship whose armour is gone still breaks up. Each
//! held ship is stored disabled as it comes, by any of the three paths
//! ([`Session::mission_ships_came`]), so the AI, boarding, hails and the
//! target panel read it disabled from its first frame. Once it is
//! boarded, its goals are met, or it loses its mark, the next fight's
//! tick reads its armour again: a swapped ship at full armour comes back
//! intact, with no event. At setup a rescue ship also starts derelict
//! (see the [`traffic::spawn`](crate::traffic::spawn) module's "Mission
//! ships"), so it stays disabled by its armour once released. The target
//! panel shows "Waiting" for a held ship whose armour is not below its
//! threshold (`_DrawStatusTarg` @0x4bab1-0x4bb87;
//! [`Session::target_status`]).
//!
//! **The swap** (`_HandlePlayerCommunication` @0x62421-0x6252f): a person
//! of `Flags` 0x0040 whose mission is accepted on its hail, with one of
//! the mission's ships left, is swapped for that ship at the start of the
//! settling that follows ([`Session::settle_script`]), before any move
//! `OnAccept` queued. The ship comes through `_MissionDudeSpawn`
//! ([`spawn::swapped_mission_ship`]), marked with the slot, and takes the
//! person's position, velocity and heading; it flies the person's type
//! under mission `Flags` 0x0800 when the `düde` lists it. The person's
//! NPC is taken out of the system (ship +0xb8 cleared), without being
//! gone for good, and the player's target moves to the ship. The ship
//! raises [`Fate::Arrived`] as the other paths' ships do. It joins none
//! of the hired escorts, and no count of the slot changes: it is the
//! slot's one ship left, so a later setup brings one, never a second
//! beside it, and phase 1's lifetime rule releases it as it does any
//! mission ship. Of a `ShipBehav` 1 mission (stock 132) it joins the
//! player's formation as a mission escort, as every `ShipBehav` 1 ship
//! does (see the [`mission_escorts`](super::mission_escorts) module); of
//! another `ShipBehav`, its `ShipGoal` 3 fly-in-form (@0x62509) follows a
//! leader of -1 and it keeps its own AI.
//!
//! **Mission escorts** are the [`mission_escorts`](super::mission_escorts)
//! module's: `ShipBehav` 1 ships join the player's formation as they come
//! ([`Session::mission_ships_came`]) and on every traffic tick, and leave
//! it as their marks are cleared.

use super::Session;
use crate::board::{BoardRefusal, Boarding, MissionBoardLine};
use crate::catalog::{DudeId, MissionId, MissionRecord, ShipId, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::control::Script;
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::mission::active::AUTO_ABORT;
use crate::mission::cargo::PICKUP_BOARDING;
use crate::mission::ships::{ShipBehav, ShipFacts, ShipStart, appear_timer, swapped_ship_type};
use crate::mission::{
    ActiveMission, AssistLine, Departure, FailedBy, GoalFacts, LossFacts, ShipGoal, ShipSystem,
    SpecialShips, assists, check_goals, departure, fails_on_destroyed, fails_on_disabled,
    goals_met_moment, held_disabled, met_before,
};
use crate::rulebook::{RuleKey, RuleSource};
use crate::session::mission_screens::MissionTextKind;
use crate::story::{
    Consumer, Entrance, EventFilter, Fate, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{MissionMark, Npc, NpcId};
use crate::traffic::spawn::{self, MissionSpawn, NewShip};
use crate::traffic::table::SpawnTable;

/// The appear timer's highest value that counts down (32000 @0x437f5).
const COUNTED: i16 = 32_000;

/// What a goal check takes as seen of a mission's ships.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sighting {
    /// One of them was just seen.
    sighted: bool,
    /// They are taken to be here, wherever they are (the walker's).
    here: bool,
}

/// How a mission ship was lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Loss {
    /// Destroyed.
    Destroyed,
    /// Disabled.
    Disabled,
}

/// A slot's enter-system hook, on `EnteredSystem`: the appear timer's
/// restart, then in flight its goal check.
pub const ENTERED: u8 = 0;
/// A slot's goal check on `Landed`, before the missions' landing hooks
/// (`_MissionLandCheck` @0xa1a0c).
pub const LANDED: u8 = 1;
/// A slot's goal check on every `ShipFate`.
pub const SHIP_FATE: u8 = 2;
/// A slot's goal check on `TookOff`: the first flight frame after a
/// docked accept or a landing, held until the system is set up.
pub const TOOK_OFF: u8 = 3;

/// Every hook a slot registers, each on its event, in order.
const HOOKS: [(u8, StoryEventKind); 4] = [
    (ENTERED, StoryEventKind::EnteredSystem),
    (LANDED, StoryEventKind::Landed),
    (SHIP_FATE, StoryEventKind::ShipFate),
    (TOOK_OFF, StoryEventKind::TookOff),
];

/// Slot `slot`'s subscription for `hook`.
#[must_use]
pub const fn ship_hook(slot: u8, hook: u8) -> SubscriptionId {
    SubscriptionId {
        consumer: Consumer::MissionShips,
        record: slot as i16,
        hook,
    }
}

/// Every subscription slot `slot` registers for its ships.
pub fn ship_hooks(slot: u8) -> impl Iterator<Item = SubscriptionId> {
    HOOKS
        .into_iter()
        .map(move |(hook, _)| ship_hook(slot, hook))
}

/// Whether the enter-system hook runs on an entry `how`: a jump's
/// arrival (@0x6c15d), a gate's or wormhole's exit (@0x67159) and an `M`
/// or `N` in flight (@0x159a8).
fn restarts_on(how: Entrance) -> bool {
    matches!(
        how,
        Entrance::Jump | Entrance::Gate | Entrance::ScriptMove { landed: false }
    )
}

impl Session {
    /// The special ships of `record`, just accepted as `active`, as
    /// `_LoadCurrentMissionData` loads them before `OnAccept`
    /// ([`SpecialShips::at_accept`]), from the player's system now, over
    /// the overlaid map and the governments, their names from the string
    /// lists ([`Session::strings`]), drawing on `chance`.
    pub(super) fn ships_at_accept(
        &self,
        record: &MissionRecord,
        active: &ActiveMission,
        chance: &mut dyn Chance,
    ) -> SpecialShips {
        let held =
            |stop: Option<crate::mission::Stop>| stop.map(|stop| self.holder_of(stop.stellar));
        let facts = ShipFacts {
            player_system: self.pilot.system,
            destination: held(active.destination),
            return_to: held(active.return_to),
        };
        SpecialShips::at_accept(
            &record.ships,
            &facts,
            (self.galaxy.map(), &self.govts),
            self.strings(),
            chance,
        )
    }

    /// Registers `slot`'s hooks ([`ship_hook`]): its enter-system hook
    /// and its goal checks, each with a blank guard and no script.
    pub(super) fn register_ship_hooks(&mut self, slot: u8) {
        for (hook, kind) in HOOKS {
            let subscription = Subscription::new(
                ship_hook(slot, hook),
                EventFilter::Any(kind),
                Script::default(),
            );
            let registered = self.register_story(subscription);
            debug_assert!(registered.is_ok(), "a slot filled registers once");
        }
    }

    /// Whether ships in `system` are in the player's system
    /// ([`ShipSystem::with_player`]), over the overlaid map's places.
    fn ships_here(&self, system: ShipSystem) -> bool {
        system.with_player(self.pilot.system, self.galaxy.map().places())
    }

    /// Whether the enter-system hook, on an entry `how`, restarts the
    /// appear timer of `active`, a mission of `record`: an entry it runs
    /// on, `ShipStart` 1, ships left, and the player in their system
    /// (@0x99f5b-0x99f8a). Not on a gate's exit for ships following the
    /// player while the exit decides for them
    /// ([`Session::gate_decides`]), as the original's loop A overwrites
    /// the restart.
    fn hook_restarts(&self, record: &MissionRecord, active: &ActiveMission, how: Entrance) -> bool {
        restarts_on(how)
            && ShipStart::from_code(record.ships.start) == ShipStart::ComesLater
            && active.ships.left > 0
            && self.ships_here(active.ships.system)
            && !self.gate_decides(how, active.ships.system)
    }

    /// Slot `id.record`'s hook `id.hook` on `event`, drawing on `chance`
    /// (see the module docs): the enter-system hook restarts the timer,
    /// then checks the goals in flight; the other hooks check the goals,
    /// a ship of the slot sighted on a `ShipFate` passed on as seen.
    pub(super) fn mission_ships_effect(
        &mut self,
        id: SubscriptionId,
        event: &StoryEvent,
        chance: &mut dyn Chance,
    ) {
        let Ok(slot) = u8::try_from(id.record) else {
            return;
        };
        match (id.hook, *event) {
            (ENTERED, StoryEvent::EnteredSystem { how, .. }) => {
                self.restart_on_entry(slot, how, chance);
                if restarts_on(how) {
                    self.aux_on_entry(slot, how, chance);
                }
                if self.landed.is_none() {
                    self.check_ship_goals(slot, false, chance);
                }
            }
            (LANDED, StoryEvent::Landed { .. }) | (TOOK_OFF, StoryEvent::TookOff { .. }) => {
                self.check_ship_goals(slot, false, chance);
            }
            (SHIP_FATE, StoryEvent::ShipFate { ship, fate }) => {
                let sighted = fate == Fate::Observed && self.marked_here(ship, slot);
                self.check_ship_goals(slot, sighted, chance);
            }
            _ => {}
        }
    }

    /// The enter-system hook of the mission in `slot` on an entry `how`,
    /// drawing on `chance`: restarts its special ships' appear timer
    /// ([`appear_timer`]) and zeroes their arrived count.
    fn restart_on_entry(&mut self, slot: u8, how: Entrance, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        if !self.hook_restarts(&record, active, how) {
            return;
        }
        let appear = appear_timer(&record.ships, chance);
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.ships.appear = appear;
            active.ships.arrived = 0;
        }
    }

    /// Whether the NPC `ship` is in the system, a special ship of the
    /// mission in `slot`.
    fn marked_here(&self, ship: NpcId, slot: u8) -> bool {
        self.npcs()
            .iter()
            .any(|npc| npc.id == ship && npc.mission.is_some_and(|mark| mark.slot == slot))
    }

    /// Checks the goals of the mission in `slot` (`_MissionObjectivesCheck`
    /// @0x9e79e; see [`check_goals`]), one of its ships `sighted` or not,
    /// drawing on `chance`: its goals met as the check finds, and failed,
    /// nothing run, when the check fails it ([`FailedBy::Marked`]). The
    /// moment its goals are met ([`goals_met_moment`]) shows its
    /// `ShipDoneText` filled for the slot (@0x9ed0f), runs its
    /// `OnShipDone` with the slot as the script's mission when it has a
    /// `ShipGoal` (@0x9ed70), and then, with `Flags` 0x0001, auto-aborts it
    /// (@0x9ed87). Its escorted ships are those of the slot in the system
    /// now (@0x9e931-0x9e962). Nothing for a free slot.
    pub(super) fn check_ship_goals(&mut self, slot: u8, sighted: bool, chance: &mut dyn Chance) {
        let here = false;
        self.check_goals_seen(slot, Sighting { sighted, here }, chance);
    }

    /// [`Session::check_ship_goals`], one of the ships seen as `seen`
    /// says.
    fn check_goals_seen(&mut self, slot: u8, seen: Sighting, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let ships = &active.ships;
        let facts = GoalFacts {
            goal: ShipGoal::from_code(record.ships.goal),
            count: record.ships.count,
            flags: record.flags,
            left: ships.left,
            arrived: ships.arrived,
            destroyed: ships.destroyed,
            boarded: ships.boarded,
            disabled: ships.disabled,
            chased_off: ships.chased_off,
            here: seen.here || self.ships_here(ships.system),
            escorted: self
                .npcs()
                .iter()
                .any(|npc| npc.mission.is_some_and(|mark| mark.slot == slot)),
            sighted: seen.sighted,
            start: ShipStart::from_code(record.ships.start),
            met: active.goals_met,
        };
        let failed_before = active.failed.is_some();
        let check = check_goals(&facts);
        if (check.met != facts.met || (check.failed && !failed_before))
            && let Some(active) = self.slot_holding(slot, record.id)
        {
            active.goals_met = check.met;
            if check.failed {
                active.failed.get_or_insert(FailedBy::Marked);
            }
        }
        if goals_met_moment(met_before(&facts), check, failed_before) {
            self.ship_goals_met(slot, &record, chance);
        }
    }

    /// The moment the goals of `record`, the mission in `slot`, are met
    /// (@0x9ec92-0x9ed87), drawing on `chance`: its `ShipDoneText`, its
    /// `OnShipDone` with a `ShipGoal`, then with `Flags` 0x0001 its
    /// auto-abort, while the slot still holds it.
    fn ship_goals_met(&mut self, slot: u8, record: &MissionRecord, chance: &mut dyn Chance) {
        self.show_mission_text(MissionTextKind::ShipDone, record.id, Some(slot));
        if ShipGoal::from_code(record.ships.goal) != ShipGoal::None {
            self.run_mission_script(slot, &record.on_ship_done, chance);
        }
        self.abort_on_spawn(slot, record, chance);
    }

    /// Auto-aborts `record`, the mission in `slot`, when it has `Flags`
    /// 0x0001 and the slot still holds it ([`Session::auto_abort`]),
    /// drawing on `chance`: as its goals are met, and as its ships are
    /// set up or come in by hyperspace (see the module docs).
    pub(super) fn abort_on_spawn(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        chance: &mut dyn Chance,
    ) {
        if record.flags & AUTO_ABORT != 0 && self.pilot.missions.holds(slot, record.id) {
            self.auto_abort(slot, record, chance);
        }
    }

    /// Auto-aborts each mission of `Flags` 0x0001 among `set_up`, the
    /// slots and missions the system's setup took, in order, drawing on
    /// `chance` (`_SetupShipsInSystem` @0x43169-0x4317e).
    pub(super) fn abort_set_up(&mut self, set_up: &[(u8, MissionId)], chance: &mut dyn Chance) {
        for &(slot, mission) in set_up {
            let record = self.lifecycle_record(mission);
            self.abort_on_spawn(slot, &record, chance);
        }
    }

    /// The system's `NavDefs` slots: each its stellar's position, none
    /// for an empty slot (or one whose stellar cannot be read).
    fn nav_slots(&self) -> Vec<Option<Vec2>> {
        let mut sites = self.sites.iter();
        self.galaxy
            .map()
            .stellars(self.pilot.system)
            .iter()
            .map(|slot| slot.and_then(|_| sites.next()).map(|site| site.position))
            .collect()
    }

    /// The `düde`s of the active missions' special ships, in slot order.
    pub(super) fn mission_dudes(&self) -> Vec<DudeId> {
        self.pilot
            .missions
            .iter()
            .filter_map(|(_, active)| self.lifecycle_record(active.mission).ships.dude)
            .collect()
    }

    /// The active missions' special ships set up in the system from
    /// `table`, drawing on `chance`, in slot order (`_SetupShipsInSystem`
    /// @0x42dc4-0x43194): a slot whose ships are here, with ships left and
    /// its appear timer not running, brings ships left of them
    /// ([`spawn::mission_ships`]). `entering`
    /// by an entry whose enter-system hook is still to run, a slot the
    /// hook is about to restart brings none (see the module docs). With the ships,
    /// the slots it took and their missions, in order, a slot of no
    /// `düde` among them: each is auto-aborted with `Flags` 0x0001
    /// ([`Session::abort_set_up`]).
    pub(super) fn setup_mission_ships(
        &self,
        table: &SpawnTable,
        entering: Option<Entrance>,
        chance: &mut (impl Chance + ?Sized),
    ) -> (Vec<NewShip>, Vec<(u8, MissionId)>) {
        let nav = self.nav_slots();
        let mut ships = Vec::new();
        let mut set_up = Vec::new();
        for (slot, active) in self.pilot.missions.iter() {
            let record = self.lifecycle_record(active.mission);
            let state = &active.ships;
            if !self.ships_here(state.system)
                || state.left <= 0
                || state.appear >= 0
                || entering.is_some_and(|how| self.hook_restarts(&record, active, how))
            {
                continue;
            }
            set_up.push((slot, record.id));
            let Some(dude) = record.ships.dude else {
                continue;
            };
            let spawn = MissionSpawn {
                slot,
                dude,
                count: state.left,
                goal: ShipGoal::from_code(record.ships.goal),
                start: ShipStart::from_code(record.ships.start),
                behav: ShipBehav::from_code(record.ships.behav),
                nav: &nav,
            };
            ships.extend(spawn::mission_ships(table, &spawn, self.gate(), chance));
        }
        (ships, set_up)
    }

    /// The way from the player's system to the one it last jumped from,
    /// on the map: none before a jump, or when either is not on it.
    fn way_back(&self) -> Option<Vec2> {
        let from = self.galaxy.position(self.jumped_from?)?;
        let here = self.galaxy.position(self.pilot.system)?;
        Some(from - here)
    }

    /// A tick of the special ships' appear timers in flight, as
    /// `_EnterMoreShips` (@0x43757-0x43ad9) runs them every frame, the
    /// ships read from `catalog` and drawn on `chance`: for each slot in
    /// order whose ships are here with any to come (ships left, less
    /// those arrived for `ShipStart` 1), a running timer counts down
    /// (from 32000 or below), and at 0 it stops and
    /// they come in by hyperspace ([`spawn::arriving_mission_ships`]),
    /// each counted arrived. Once all have come, a mission of `Flags`
    /// 0x0001 auto-aborts (@0x43abc-0x43ad4), its ships staying. As the
    /// system is `entering` by a way in whose enter-system hook is still
    /// to run, if any, a slot the hook is about to restart is left to it.
    pub(super) fn mission_ship_arrivals(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        entering: Option<Entrance>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let hook_due = entering.is_some_and(restarts_on);
        let slots: Vec<(u8, MissionId)> = self
            .pilot
            .missions
            .iter()
            .map(|(slot, active)| (slot, active.mission))
            .collect();
        for (slot, mission) in slots {
            if !hook_due {
                self.aux_arrivals(catalog, slot, chance);
            }
            self.special_arrivals(catalog, slot, entering, chance);
            if !hook_due {
                self.tick_aux(slot, mission);
            }
        }
    }

    /// The special ships' part of [`Session::mission_ship_arrivals`] for
    /// the slot `slot`, read afresh: nothing once it is free.
    fn special_arrivals(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        entering: Option<Entrance>,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let state = active.ships.clone();
        if !self.ships_here(state.system)
            || entering.is_some_and(|how| self.hook_restarts(&record, active, how))
        {
            return;
        }
        let coming = if ShipStart::from_code(record.ships.start) == ShipStart::ComesLater {
            state.left - state.arrived
        } else {
            state.left
        };
        if coming <= 0 {
            return;
        }
        let mut appear = state.appear;
        if (1..=COUNTED).contains(&appear) {
            appear -= 1;
        }
        // With any to come, ships are left.
        let fires = appear == 0;
        if fires {
            appear = -1;
        }
        // The timer is kept, not saved, as it runs: no save is due.
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.ships.appear = appear;
        }
        if !fires {
            return;
        }
        let count = self.bring_in_mission_ships(catalog, slot, &record, coming, chance);
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.ships.arrived = active.ships.arrived.saturating_add(count);
        }
        self.abort_on_spawn(slot, &record, &mut &mut *chance);
    }

    /// Brings `coming` special ships of `record`, the mission in `slot`,
    /// into the system by hyperspace ([`spawn::arriving_mission_ships`]),
    /// their `düde`'s ships read from `catalog`, drawing on `chance`
    /// (@0x43821-0x43aa5): how many came, none for a mission of no
    /// `düde`.
    fn bring_in_mission_ships(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        record: &MissionRecord,
        coming: i16,
        chance: &mut (impl Chance + ?Sized),
    ) -> i16 {
        let Some(dude) = record.ships.dude else {
            return 0;
        };
        self.traffic.table_mut().add_mission_dudes(
            catalog,
            &[dude],
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        let nav = Vec::new();
        let spawn = MissionSpawn {
            slot,
            dude,
            count: coming,
            goal: ShipGoal::from_code(record.ships.goal),
            start: ShipStart::from_code(record.ships.start),
            behav: ShipBehav::from_code(record.ships.behav),
            nav: &nav,
        };
        let ships = spawn::arriving_mission_ships(
            self.traffic.table(),
            &spawn,
            self.way_back(),
            self.gate(),
            chance,
        );
        let count = i16::try_from(ships.len()).unwrap_or(i16::MAX);
        let last = self.last_npc();
        self.traffic.bring_in(ships);
        self.mission_ships_came(last);
        count
    }

    /// Swaps the person's NPC `npc`, its mission just accepted on its hail
    /// into `slot`, for that mission's one ship (see the module docs),
    /// the `düde` read from `catalog`, drawing on `chance`
    /// (`_HandlePlayerCommunication` @0x6244b-0x6252f): the ship, of the
    /// person's type under mission `Flags` 0x0800 when the `düde` lists it
    /// ([`swapped_ship_type`]), takes the person's place
    /// ([`spawn::swapped_mission_ship`]); the person's NPC is taken out,
    /// every other NPC letting it go; the ship raises [`Fate::Arrived`];
    /// and it becomes the player's target when the person was, its
    /// brackets settled (@0x62538; see the `brackets` module). No count
    /// of the slot changes. Nothing when the NPC is no longer here, the
    /// slot no longer holds a mission of a `düde`, or the ship cannot be
    /// spawned. Of a `ShipBehav` 1 mission it joins the player's
    /// formation as it comes ([`Session::mission_ships_came`]); of
    /// another, it keeps its `düde`'s AI, as `_AIMakeEscortFlyInForm`
    /// (@0x62509) gives it no leader to follow (see the
    /// [`mission_escorts`](super::mission_escorts) module).
    pub(super) fn swap_in_mission_ship(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        npc: NpcId,
        slot: u8,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let Some(person) = self.npcs().iter().find(|ship| ship.id == npc) else {
            return;
        };
        let (state, person_ship) = (person.state, person.ship);
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let Some(dude) = record.ships.dude else {
            return;
        };
        let dude_ships: Vec<ShipId> = catalog
            .dude(dude)
            .map(|dude| dude.ships.iter().map(|&(ship, _)| ship).collect())
            .unwrap_or_default();
        let preset = swapped_ship_type(record.flags, &dude_ships, person_ship);
        self.traffic.table_mut().add_mission_dudes(
            catalog,
            &[dude],
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        let spawn = MissionSpawn {
            slot,
            dude,
            count: 1,
            goal: ShipGoal::from_code(record.ships.goal),
            start: ShipStart::from_code(record.ships.start),
            behav: ShipBehav::from_code(record.ships.behav),
            nav: &[],
        };
        let Some(ship) = spawn::swapped_mission_ship(
            self.traffic.table(),
            &spawn,
            state,
            preset,
            self.gate(),
            chance,
        ) else {
            return;
        };
        let targeted = self.target == Some(npc);
        self.drop_quarry(npc);
        self.traffic.remove(npc);
        let last = self.last_npc();
        self.traffic.bring_in(vec![ship]);
        if targeted {
            self.target = self.last_npc();
            self.target_fly_in.settle();
        }
        self.mission_ships_came(last);
    }

    /// Leaves the special ships of the mission in `slot` as meeting its
    /// goal in flight would, then checks its goals as the subscriptions
    /// do, so the moment's `ShipDoneText`, `OnShipDone` and auto-abort
    /// run, drawing on `chance`: all of them arrived; for a destroy goal,
    /// all destroyed and none left; for a disable goal, all disabled; for
    /// a board or rescue goal, all boarded; for a chase-off goal, all
    /// chased off and none left; for an observe goal, one seen with them
    /// here; for an escort goal, all of them brought into the
    /// system the ship is in now, docked or not, through the arrivals'
    /// spawn path, their `düde` read from `catalog`. Whether the mission
    /// has such a goal: none for a free slot, a mission of no ships, or of
    /// no goal this meets. Built only for the storyline walker
    /// (`fixture`), its one caller.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn meet_ship_goal(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        chance: &mut dyn Chance,
    ) -> bool {
        let Some(active) = self.pilot.missions.get(slot) else {
            return false;
        };
        let record = self.lifecycle_record(active.mission);
        let count = record.ships.count;
        let goal = ShipGoal::from_code(record.ships.goal);
        let meets = matches!(
            goal,
            ShipGoal::Destroy
                | ShipGoal::Disable
                | ShipGoal::Board
                | ShipGoal::Escort
                | ShipGoal::Observe
                | ShipGoal::Rescue
                | ShipGoal::ChaseOff
        );
        if count <= 0 || !meets {
            return false;
        }
        if goal == ShipGoal::Escort {
            self.bring_in_mission_ships(catalog, slot, &record, count, chance);
        }
        if let Some(active) = self.slot_holding(slot, record.id) {
            let ships = &mut active.ships;
            ships.arrived = count;
            match goal {
                ShipGoal::Destroy => {
                    ships.destroyed = count;
                    ships.left = 0;
                }
                ShipGoal::Disable => ships.disabled = count,
                ShipGoal::Board | ShipGoal::Rescue => ships.boarded = count,
                ShipGoal::ChaseOff => {
                    ships.chased_off = count;
                    ships.left = 0;
                }
                _ => {}
            }
        }
        let observed = goal == ShipGoal::Observe;
        self.check_goals_seen(
            slot,
            Sighting {
                sighted: observed,
                here: observed,
            },
            chance,
        );
        true
    }

    /// Boards `npc` when it is a mission's special ship
    /// (`_HandlePlayerBoardAttempt` @0x652a2-0x657e3), before the
    /// player's velocity is matched. A mission of `PickupMode` 2 takes
    /// the pickup path (@0x652ee-0x65570): its cargo is picked up
    /// ([`Session::pick_up_boarded_cargo`]), and once aboard its slot
    /// counts the ship boarded, whatever its goal (@0x653a7), and the
    /// comm line names the cargo; a pickup refused refuses the board
    /// ([`BoardRefusal::CantBoard`], @0x65fdb), its dialog shown and
    /// nothing else changed. Any other takes the assist path
    /// ([`assists`]), on which its slot counts it boarded and the comm
    /// line is said. Then it is marked boarded (@0x65f43), with no
    /// `BoardPenalty`, person's offer or plunder. A ship of a failed
    /// mission is boarded for plunder, but still with no `BoardPenalty`
    /// (the slap @0x6582b passes its mark; see [`Session::board`]):
    /// none. The assist path's ship +0x4c (the AI's wait countdown, 100
    /// @0x65746) is not kept.
    pub(super) fn board_mission_ship(
        &mut self,
        npc: &Npc,
    ) -> Option<Result<Boarding, BoardRefusal>> {
        let mark = npc.mission?;
        let mut line = None;
        if let Some(active) = self.pilot.missions.get(mark.slot) {
            if active.failed.is_some() {
                return None;
            }
            let record = self.lifecycle_record(active.mission);
            let cargo = active.cargo_type;
            if record.pickup_mode == PICKUP_BOARDING {
                if !self.pick_up_boarded_cargo(mark.slot) {
                    return Some(Err(BoardRefusal::CantBoard));
                }
                if let Some(active) = self.slot_holding(mark.slot, record.id) {
                    active.ships.boarded = active.ships.boarded.saturating_add(1);
                }
                self.mark_boarded(npc.id);
                return Some(Ok(Boarding::MissionShip(Some(
                    MissionBoardLine::Retrieved { cargo },
                ))));
            }
            let assist = assists(
                ShipGoal::from_code(record.ships.goal),
                record.flags,
                record.pickup_mode,
                active.ships.left,
            );
            if let Some(assist) = assist {
                line = Some(match assist {
                    AssistLine::Boarded => MissionBoardLine::Boarded,
                    AssistLine::Refuelled => MissionBoardLine::Refuelled { ship: npc.ship },
                });
                if let Some(active) = self.slot_holding(mark.slot, record.id) {
                    active.ships.boarded = active.ships.boarded.saturating_add(1);
                }
            }
        }
        self.mark_boarded(npc.id);
        Some(Ok(Boarding::MissionShip(line)))
    }

    /// Whether `npc` is held disabled whatever its armour, as
    /// `_IsDisabled` reads it (@0x2d24-0x2d69; see [`held_disabled`]): a
    /// special ship of the mission in its slot whose `ShipGoal` is to
    /// rescue them, its goals not yet met and the ship not yet boarded.
    /// A ship with no mark, or of a free slot, is not held.
    pub(crate) fn mission_ship_held(&self, npc: &Npc) -> bool {
        let Some(active) = npc
            .mission
            .and_then(|mark| self.pilot.missions.get(mark.slot))
        else {
            return false;
        };
        let goal = self
            .mission_record(active.mission)
            .map_or(ShipGoal::None, |record| {
                ShipGoal::from_code(record.ships.goal)
            });
        held_disabled(goal, active.goals_met, npc.boarded)
    }

    /// The special ships just brought into the system, each numbered
    /// above `last` (every one with none): each held disabled
    /// ([`Session::mission_ship_held`]) is stored disabled, so it reads so
    /// before any fight; each that may join the player's formation joins
    /// it ([`Session::join_mission_escorts`]); each raises
    /// [`Fate::Arrived`], and each of a
    /// mission whose goal is to observe them, not yet met, is watched for
    /// its first sighting ([`Session::watch_ship`]).
    pub(super) fn mission_ships_came(&mut self, last: Option<NpcId>) {
        let came: Vec<(NpcId, u8)> = self
            .npcs()
            .iter()
            .filter(|npc| last.is_none_or(|last| npc.id > last))
            .filter_map(|npc| Some((npc.id, npc.mission?.slot)))
            .collect();
        let held: Vec<NpcId> = self
            .npcs()
            .iter()
            .filter(|npc| came.iter().any(|&(id, _)| id == npc.id) && self.mission_ship_held(npc))
            .map(|npc| npc.id)
            .collect();
        for npc in self.traffic.npcs_mut() {
            if held.contains(&npc.id) {
                npc.condition = Condition::Disabled;
            }
        }
        self.join_mission_escorts();
        for (ship, slot) in came {
            self.raise(StoryEvent::ShipFate {
                ship,
                fate: Fate::Arrived,
            });
            let observing = self.pilot.missions.get(slot).is_some_and(|active| {
                !active.goals_met
                    && ShipGoal::from_code(self.lifecycle_record(active.mission).ships.goal)
                        == ShipGoal::Observe
            });
            if observing {
                self.watch_ship(ship);
            }
        }
    }

    /// NPC `id`, destroyed, drawing on `chance`: when it is a special ship
    /// of a mission in its slot, the mission fails at once when this loss
    /// fails it ([`fails_on_destroyed`]); then, while the slot still holds
    /// it, its ships destroyed go up by one and its ships left down by one
    /// while above none, so it is not brought in again
    /// (`_HandleShipDisplay` @0x2daa2-0x2dbf1).
    pub(super) fn count_lost_mission_ship(&mut self, id: NpcId, chance: &mut dyn Chance) {
        let Some((slot, mission)) = self.lose_mission_ship(id, Loss::Destroyed, chance) else {
            return;
        };
        if let Some(active) = self.slot_holding(slot, mission) {
            active.ships.destroyed = active.ships.destroyed.saturating_add(1);
            if active.ships.left > 0 {
                active.ships.left -= 1;
            }
        }
    }

    /// The marks of the special ships in the system, each with its NPC:
    /// what [`Session::mission_ships_left`] reads once the traffic's tick
    /// has taken those that left out.
    pub(super) fn mission_marks(&self) -> Vec<(NpcId, MissionMark)> {
        self.npcs()
            .iter()
            .filter_map(|npc| Some((npc.id, npc.mission?)))
            .collect()
    }

    /// The special ships among those the traffic's last tick took out,
    /// marked as `marks` had them before it, in the order they left: each
    /// that left the system raises [`Fate::Departed`], then its slot
    /// counts it ([`Session::count_left_mission_ship`]) when it counts.
    /// One that jumped out left and counts; one that landed left and
    /// counts only under [`RuleKey::LandedLeaves`]'s other reading, as the
    /// original's waits at the stellar; one that entered a gate left, as
    /// the original frees its slot, but counts only under
    /// [`RuleKey::GateExitCounts`]'s other reading; a docked fighter never
    /// left.
    pub(super) fn mission_ships_left(&mut self, marks: &[(NpcId, MissionMark)]) {
        let landed_leaves = self.rules.source_for(RuleKey::LandedLeaves) == RuleSource::Bible;
        let gate_exit_counts = self.rules.source_for(RuleKey::GateExitCounts) == RuleSource::Bible;
        let left: Vec<(NpcId, MissionMark, bool)> = self
            .traffic
            .departed()
            .iter()
            .filter_map(|&(id, outcome)| {
                let counts = match outcome {
                    Outcome::JumpedOut => true,
                    Outcome::Landed(_) if landed_leaves => true,
                    Outcome::EnteredGate(_) => gate_exit_counts,
                    Outcome::Landed(_) | Outcome::Docked { .. } | Outcome::Flying => {
                        return None;
                    }
                };
                let (_, mark) = marks.iter().find(|(ship, _)| *ship == id)?;
                Some((id, *mark, counts))
            })
            .collect();
        for (ship, mark, counts) in left {
            self.raise(StoryEvent::ShipFate {
                ship,
                fate: Fate::Departed,
            });
            if counts {
                self.count_left_mission_ship(mark);
            }
        }
    }

    /// A special ship marked `mark` that left the system, its NPC gone
    /// already: while its slot holds a mission, its ships chased off and
    /// left change as [`departure`] says, over the slot's `ShipGoal`, its
    /// ships following the player or not, and the ships-left rule
    /// ([`RuleKey::LeftRespawn`]) (`_LowLevelAIHandler` @0x85960-0x859aa).
    /// A failed mission's ship counts too, as the original tests no
    /// failure; leaving fails nothing.
    pub(super) fn count_left_mission_ship(&mut self, mark: MissionMark) {
        let Some(active) = self.pilot.missions.get(mark.slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let counted = departure(
            ShipGoal::from_code(record.ships.goal),
            active.ships.system == ShipSystem::FollowsPlayer,
            self.rules.source_for(RuleKey::LeftRespawn),
        );
        if counted == Departure::default() {
            return;
        }
        if let Some(active) = self.slot_holding(mark.slot, record.id) {
            let ships = &mut active.ships;
            if counted.chased_off {
                ships.chased_off = ships.chased_off.saturating_add(1);
            }
            if counted.lowers_left && ships.left > 0 {
                ships.left -= 1;
            }
        }
    }

    /// NPC `id`, newly disabled, drawing on `chance`: when it is a special
    /// ship of a mission in its slot, the mission fails at once when this
    /// loss fails it ([`fails_on_disabled`]); then, while the slot still
    /// holds it, its ships disabled go up by one (`_DamageShip`
    /// @0x3ae68-0x3af8b). The original's test that the ship is no stellar's
    /// defence ship (+0x8c of -1, @0x3ae79) holds for every mission ship.
    pub(super) fn count_disabled_mission_ship(&mut self, id: NpcId, chance: &mut dyn Chance) {
        let Some((slot, mission)) = self.lose_mission_ship(id, Loss::Disabled, chance) else {
            return;
        };
        if let Some(active) = self.slot_holding(slot, mission) {
            active.ships.disabled = active.ships.disabled.saturating_add(1);
        }
    }

    /// NPC `id` lost as `loss` says, drawing on `chance`: when it is a
    /// special ship of a mission in its slot, and the loss fails the
    /// mission, [`SimMessage::MissionShipLost`] and the quick failure
    /// ([`Session::quick_failure`], @0x2dbba, @0x3af71). Its slot and
    /// mission, or none for a ship of no mission held.
    fn lose_mission_ship(
        &mut self,
        id: NpcId,
        loss: Loss,
        chance: &mut dyn Chance,
    ) -> Option<(u8, MissionId)> {
        let npc = self.npcs().iter().find(|npc| npc.id == id)?;
        let (mark, boarded) = (npc.mission?, npc.boarded);
        let active = self.pilot.missions.get(mark.slot)?;
        let record = self.lifecycle_record(active.mission);
        let facts = LossFacts {
            goal: ShipGoal::from_code(record.ships.goal),
            flags: record.flags,
            failed: active.failed.is_some(),
            before: match loss {
                Loss::Destroyed => active.ships.destroyed,
                Loss::Disabled => active.ships.disabled,
            },
            boarded,
        };
        let fails = match loss {
            Loss::Destroyed => fails_on_destroyed(facts),
            Loss::Disabled => fails_on_disabled(facts),
        };
        if fails {
            self.messages.push(SimMessage::MissionShipLost);
            self.quick_failure(mark.slot, &record, chance);
        }
        Some((mark.slot, record.id))
    }

    /// The special ships of the mission in `slot`, aborted
    /// (`_AbortMission` @0x99ab5-0x99b24): each loses its mark and flies
    /// on as ordinary traffic, a mission escort leaving the formation and
    /// the system (see the [`mission_escorts`](super::mission_escorts)
    /// module); or, while the player is docked, each is taken out of the
    /// system.
    pub(super) fn release_mission_ships(&mut self, slot: u8) {
        let marked: Vec<NpcId> = self
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some_and(|mark| mark.slot == slot))
            .map(|npc| npc.id)
            .collect();
        self.unmark_mission_ships(&marked);
    }

    /// Clears the mark of every special ship whose slot is free, as
    /// `_AIDispatch` does on each think (@0x8fbcd-0x8fc96): it flies on as
    /// ordinary traffic, a mission escort leaving the formation and the
    /// system. A mission still in its slot keeps its ships marked, failed
    /// or not.
    pub(super) fn unmark_inactive_mission_ships(&mut self) {
        let missions = &self.pilot.missions;
        let freed: Vec<NpcId> = self
            .npcs()
            .iter()
            .filter(|npc| {
                npc.mission
                    .is_some_and(|mark| missions.get(mark.slot).is_none())
            })
            .map(|npc| npc.id)
            .collect();
        self.unmark_mission_ships(&freed);
    }

    /// The special ships `ids` lose their marks
    /// ([`Session::unmark_mission_ship`]), or, while the player is
    /// docked, are taken out of the system; the formation re-forms when a
    /// mission escort left it.
    fn unmark_mission_ships(&mut self, ids: &[NpcId]) {
        let mut reform = false;
        for &id in ids {
            if self.landed.is_some() {
                self.traffic.remove(id);
            } else {
                reform |= self.unmark_mission_ship(id);
            }
        }
        if reform {
            self.reform();
        }
    }

    /// The mission ship `npc`'s name or subtitle, as `pick` takes it from
    /// its slot's ships, while it is marked and that is not empty.
    pub(super) fn mission_ship_text(
        &self,
        mark: Option<crate::traffic::npc::MissionMark>,
        pick: impl Fn(&SpecialShips) -> &str,
    ) -> Option<&str> {
        let active = self.pilot.missions.get(mark?.slot)?;
        Some(pick(&active.ships)).filter(|text| !text.is_empty())
    }
}

#[cfg(test)]
pub(super) mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::Goal;
    use crate::ai::NovaAi;
    use crate::board::MissionBoardLine;
    use crate::catalog::{
        AuxShip, CommCatalog, DudeRecord, GovtId, MissionId, MissionShip, PersonId, PersonRecord,
        ShipId, StellarId, SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::cloak::Cloak;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::control::SetExpr;
    use crate::gate::GateKind;
    use crate::message::SimMessage;
    use crate::mission::FailedBy;
    use crate::mission::aux_ships::AuxShips;
    use crate::session::mission_screens::MissionTextKind;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, jump, land_now, mission, ship};
    use crate::traffic::npc::{Mode, Npc};

    /// `STR#` 7000 names; 7001 subtitles.
    struct Names;

    impl CommCatalog for Names {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                7000 => vec!["Ace".into(), "Rover".into()],
                7001 => vec!["Bounty Hunter".into()],
                _ => Vec::new(),
            }
        }
    }

    /// Mission 140, offered anywhere, its two special ships of düde 150
    /// as `ships` says: `ShipSyst` `syst`, `ShipStart` `start`,
    /// `ShipBehav` -1, named from 7000 and subtitled from 7001.
    pub(in crate::session) fn shipping(syst: i16, start: i16) -> MissionShip {
        MissionShip {
            count: 2,
            goal: -1,
            syst,
            dude: Some(crate::catalog::DudeId(150)),
            behav: -1,
            start,
            name_id: Some(7000),
            subtitle: Some(7001),
        }
    }

    /// [`catalog`] with düde 150 (warships of govt 131 flying ship 200)
    /// and mission 140 with `ships`.
    pub(in crate::session) fn ships_catalog(ships: MissionShip) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.ship_records = vec![ship(128, FAST), ship(200, FAST)];
        // 130's nav slots hold 128 and 129, 131's 140.
        catalog.star_map[0].stellars = vec![Some((0, 0)); 2];
        catalog.star_map[1].stellars = vec![Some((0, 0))];
        catalog.dudes = vec![(
            crate::catalog::DudeId(150),
            DudeRecord {
                ai_type: 3,
                govt: Some(GovtId(131)),
                ships: vec![(ShipId(200), 100)],
                booty: 0,
                info_types: 0,
            },
        )];
        catalog.missions = vec![MissionRecord {
            ships,
            ..mission(140)
        }];
        catalog
    }

    /// `count` auxiliary ships of düde 151 into `AuxShipSyst` `syst`.
    pub(in crate::session) fn aux_ships(count: i16, syst: i16) -> AuxShip {
        AuxShip {
            count,
            dude: Some(crate::catalog::DudeId(151)),
            syst,
        }
    }

    /// [`ships_catalog`] with düde 151 (traders of govt 129 flying ship
    /// 201) and mission 140 of `Flags` `flags` with `aux` too.
    pub(in crate::session) fn aux_catalog(
        ships: MissionShip,
        aux: AuxShip,
        flags: u16,
    ) -> FakePilotCatalog {
        with_aux(ships_catalog(ships), aux, flags)
    }

    /// `catalog`, built on [`ships_catalog`], with düde 151 (traders of
    /// govt 129 flying ship 201) and its mission 140 of `Flags` `flags`
    /// with `aux`.
    pub(in crate::session) fn with_aux(
        mut catalog: FakePilotCatalog,
        aux: AuxShip,
        flags: u16,
    ) -> FakePilotCatalog {
        catalog.ship_records.push(ship(201, FAST));
        catalog.dudes.push((
            crate::catalog::DudeId(151),
            DudeRecord {
                ai_type: 1,
                govt: Some(GovtId(129)),
                ships: vec![(ShipId(201), 100)],
                booty: 0,
                info_types: 0,
            },
        ));
        catalog.missions[0].aux = aux;
        catalog.missions[0].flags = flags;
        catalog
    }

    pub(in crate::session) fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// A session of `catalog`, named from [`Names`], docked at 128 with
    /// mission 140 accepted into slot 0.
    pub(in crate::session) fn accepted(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Names));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, catalog);
        session
    }

    /// Takes off, which sets the system up, and settles.
    pub(in crate::session) fn take_off(session: &mut Session, catalog: &FakePilotCatalog) {
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        settle(session, catalog);
    }

    /// Jumps from the centre, where `fly_out` heads straight out, to `to`,
    /// and settles.
    pub(in crate::session) fn jump_to(session: &mut Session, catalog: &FakePilotCatalog, to: i16) {
        session.player.position = Vec2::ZERO;
        assert_eq!(jump(session, catalog, to), Some(SystemId(to)));
        settle(session, catalog);
    }

    pub(in crate::session) fn tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
    }

    /// The slots of the mission ships in the system, in order.
    pub(in crate::session) fn marked(session: &Session) -> Vec<u8> {
        session
            .npcs()
            .iter()
            .filter_map(|npc| Some(npc.mission?.slot))
            .collect()
    }

    pub(in crate::session) fn ships_of(session: &Session) -> SpecialShips {
        session
            .pilot()
            .missions()
            .get(0)
            .expect("held")
            .ships
            .clone()
    }

    // Accept.

    #[test]
    fn an_accepted_mission_holds_its_ships_drawn_as_it_is_accepted() {
        let catalog = ships_catalog(shipping(-1, 1));
        let session = accepted(&catalog);
        assert_eq!(
            ships_of(&session),
            SpecialShips {
                system: ShipSystem::At(SystemId(130)),
                left: 2,
                arrived: 0,
                appear: 199,
                name: "Rover".into(),
                subtitle: "Bounty Hunter".into(),
                destroyed: 0,
                boarded: 0,
                disabled: 0,
                chased_off: 0,
            }
        );
        assert_eq!(
            session.mission_text(0, "<SN> waits."),
            "Rover waits.",
            "<SN> reads the name drawn"
        );
    }

    #[test]
    fn the_aux_countdown_is_drawn_first_at_accept_for_every_mission() {
        for aux in [AuxShip::NONE, aux_ships(3, -1)] {
            let catalog = aux_catalog(shipping(-1, 1), aux, 0);
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_strings(Rc::new(Names));
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            settle(&mut session, &catalog);
            let mut draws = crate::testkit::Draws::of(&[5, 0, 1, 0]);
            assert_eq!(session.accept_mission(MissionId(140), &mut draws), Ok(0));
            assert_eq!(
                draws.asked,
                [70, 100, 2, 1],
                "the countdown, then the appear timer, name and subtitle"
            );
            let active = session.pilot().missions().get(0).expect("held");
            assert_eq!(
                active.aux,
                AuxShips {
                    countdown: 75,
                    arrived: 0,
                    left: aux.count.max(0),
                }
            );
            assert_eq!(active.ships.name, "Rover");
        }
    }

    #[test]
    fn the_ships_are_drawn_before_on_accept_runs() {
        // OnAccept moves the player to 131: the ships' -1 is still 130.
        let mut catalog = ships_catalog(shipping(-1, 0));
        catalog.missions[0].on_accept = crate::control::Script::parse("M131");
        let session = accepted(&catalog);
        assert_eq!(ships_of(&session).system, ShipSystem::At(SystemId(130)));
    }

    #[test]
    fn mission_ships_of_minus_4_go_to_the_returns_holder_at_accept_rescued_included() {
        // 128, lower, shadows the player's 130 at its place: 130 owns
        // nothing, and holds its own 128 by the rescue.
        let held_at = |syst| {
            let mut catalog = ships_catalog(shipping(syst, 0));
            catalog
                .star_map
                .push(crate::testkit::star(128, (0.0, 0.0), &[]));
            let mission = &mut catalog.missions[0];
            mission.avail_stel = 128;
            mission.travel_stel = 140;
            mission.return_stel = 128;
            ships_of(&accepted(&catalog)).system
        };
        assert_eq!(
            held_at(-4),
            ShipSystem::At(SystemId(130)),
            "the return's rescued holder (@0xa1031-0xa104d)"
        );
        assert_eq!(
            held_at(-3),
            ShipSystem::At(SystemId(131)),
            "the destination's owner"
        );
    }

    /// [`accepted`], with 131 hidden between the offer and the accept.
    fn accepted_with_131_hidden(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Names));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        assert!(session.change_galaxy(catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        }));
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, catalog);
        session
    }

    #[test]
    fn mission_ships_of_minus_4_go_nowhere_for_a_return_no_system_holds_at_accept() {
        let mut catalog = ships_catalog(shipping(-4, 0));
        catalog.missions[0].return_stel = 140;
        let session = accepted_with_131_hidden(&catalog);
        let stored = session.pilot().missions().get(0).expect("held").return_to;
        assert_eq!(
            stored.and_then(|stop| stop.system),
            Some(SystemId(131)),
            "the offer stored 131, which held 140 then"
        );
        assert_eq!(
            ships_of(&session).system,
            ShipSystem::Nowhere,
            "only the hidden 131 lists 140, and the player's 130 does not: no holder (@0xa1031-0xa104d)"
        );
    }

    #[test]
    fn mission_ships_of_minus_3_go_nowhere_for_a_destination_no_system_holds_and_no_return() {
        let mut catalog = ships_catalog(shipping(-3, 0));
        catalog.missions[0].travel_stel = 140;
        let session = accepted_with_131_hidden(&catalog);
        let stored = session.pilot().missions().get(0).expect("held").destination;
        assert_eq!(
            stored.and_then(|stop| stop.system),
            Some(SystemId(131)),
            "the offer stored 131, which held 140 then"
        );
        assert_eq!(
            ships_of(&session).system,
            ShipSystem::Nowhere,
            "140 has no holder now, and no return to fall back on"
        );
    }

    // Setup.

    #[test]
    fn the_ships_are_set_up_in_their_system_as_the_ship_takes_off() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        let npc = &session.npcs()[0];
        assert_eq!((npc.ship, npc.govt), (ShipId(200), Some(GovtId(131))));
        assert_eq!(npc.mode, Mode::Flying, "in place, not jumping in");
    }

    #[test]
    fn another_system_gets_none_and_a_jump_into_theirs_sets_them_up() {
        let catalog = ships_catalog(shipping(131, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), Vec::<u8>::new());
        jump_to(&mut session, &catalog, 131);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn ships_following_the_player_are_set_up_in_any_system() {
        let catalog = ships_catalog(shipping(-6, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn a_mission_with_no_ships_left_or_nowhere_sets_none_up() {
        let none = MissionShip {
            count: 0,
            ..shipping(-1, 0)
        };
        let catalog = ships_catalog(none);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty());
        let catalog = ships_catalog(shipping(0, 0));
        let mut session = accepted(&catalog);
        assert_eq!(ships_of(&session).system, ShipSystem::Nowhere);
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn a_ship_start_of_minus_n_sets_the_ships_up_at_that_nav_slots_stellar() {
        let catalog = ships_catalog(shipping(-1, -2));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        for npc in session.npcs() {
            assert_eq!(npc.state.position, Vec2::new(2000.0, 0.0), "stellar 129");
        }
    }

    // The appear timer.

    /// Ticks the traffic `n` times.
    pub(in crate::session) fn ticks(session: &mut Session, catalog: &FakePilotCatalog, n: usize) {
        for _ in 0..n {
            tick(session, catalog);
        }
    }

    #[test]
    fn ships_starting_1_set_up_none_and_come_in_by_hyperspace_when_their_timer_runs_out() {
        let catalog = crate::testkit::far_129_reachable(ships_catalog(shipping(-1, 1)));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "none at setup");
        assert_eq!(
            ships_of(&session).appear,
            199,
            "the take-off's setup counts nothing"
        );
        ticks(&mut session, &catalog, 198);
        assert_eq!(ships_of(&session).appear, 1);
        assert!(marked(&session).is_empty());
        tick(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| matches!(npc.mode, Mode::JumpingIn { .. }))
        );
        assert_eq!(
            (ships_of(&session).appear, ships_of(&session).arrived),
            (-1, 2)
        );
        ticks(&mut session, &catalog, 300);
        assert_eq!(marked(&session).len(), 2, "no more come");
        // A later take-off sets them up in place.
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        assert!(session.npcs().iter().all(|npc| npc.mode == Mode::Flying));
    }

    #[test]
    fn the_timer_counting_down_makes_no_save_due() {
        let catalog = ships_catalog(shipping(-1, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let _ = session.take_save_due();
        ticks(&mut session, &catalog, 5);
        assert_eq!(ships_of(&session).appear, 194);
        assert!(!session.take_save_due());
    }

    #[test]
    fn a_mission_started_in_flight_brings_its_dudes_ships_in_on_its_timer() {
        // `S` in flight, after the system was set up without the düde.
        let catalog = ships_catalog(shipping(-1, 1));
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Names));
        tick(&mut session, &catalog);
        assert!(session.traffic.table().dude_records.is_empty());
        session.run_set(&SetExpr::parse("S140").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(ships_of(&session).appear, 199);
        ticks(&mut session, &catalog, 199);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn only_the_ships_starting_1_not_yet_arrived_come_in() {
        let catalog = ships_catalog(MissionShip {
            count: 3,
            ..shipping(-1, 1)
        });
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.arrived = 1;
            active.ships.appear = 1;
        }
        tick(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0], "3 left, 1 arrived");
        assert_eq!(ships_of(&session).arrived, 3);
    }

    #[test]
    fn the_timer_stands_while_the_player_is_elsewhere() {
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 250);
        assert_eq!(ships_of(&session).appear, 199);
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn ships_starting_2_first_come_in_uncloaked_and_later_set_up_cloaked() {
        let catalog = ships_catalog(shipping(-1, 2));
        let mut session = accepted(&catalog);
        assert_eq!(ships_of(&session).arrived, 2, "all arrived at accept");
        take_off(&mut session, &catalog);
        assert!(
            marked(&session).is_empty(),
            "none at setup while the timer runs"
        );
        ticks(&mut session, &catalog, 199);
        assert_eq!(marked(&session), [0, 0]);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| npc.mission.is_some() && npc.cloak == Cloak::default())
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| npc.mission.is_some() && npc.cloak.direction == 1),
            "fading in"
        );
    }

    #[test]
    fn ships_coming_in_after_a_jump_enter_from_the_system_jumped_from() {
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        let appear = ships_of(&session).appear;
        assert!(appear > 0, "restarted on arrival");
        ticks(
            &mut session,
            &catalog,
            usize::try_from(appear).expect("positive"),
        );
        assert_eq!(marked(&session), [0, 0]);
        // 130 is at (0, 0) and 131 at (600, 0): they come from the left.
        for npc in session.npcs() {
            assert!(npc.state.position.x < -1500.0, "{:?}", npc.state.position);
        }
    }

    // The enter-system hook.

    /// The timer and arrived count after `how` into `system` is raised
    /// and settled, the ships' appear timer first set to `appear` with
    /// `arrived` arrived.
    fn after_entering(
        catalog: &FakePilotCatalog,
        session: &mut Session,
        how: Entrance,
    ) -> (i16, i16) {
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = -1;
            active.ships.arrived = 2;
        }
        session.raise(StoryEvent::EnteredSystem {
            system: session.pilot.system,
            how,
        });
        settle(session, catalog);
        let ships = ships_of(session);
        (ships.appear, ships.arrived)
    }

    #[test]
    fn the_hook_restarts_the_timer_on_a_jump_a_gate_and_a_move_in_flight_only() {
        let catalog = ships_catalog(shipping(-1, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        for how in [
            Entrance::Jump,
            Entrance::Gate,
            Entrance::ScriptMove { landed: false },
        ] {
            assert_eq!(
                after_entering(&catalog, &mut session, how),
                (199, 0),
                "{how:?}"
            );
        }
        for how in [Entrance::ScriptMove { landed: true }, Entrance::Relocate] {
            assert_eq!(
                after_entering(&catalog, &mut session, how),
                (-1, 2),
                "{how:?}"
            );
        }
    }

    #[test]
    fn the_hook_restarts_ships_of_an_escort_goal_escorting_the_player_to_30() {
        let escorts = MissionShip {
            goal: 3,
            behav: 1,
            ..shipping(-1, 1)
        };
        let catalog = ships_catalog(escorts);
        let mut session = accepted(&catalog);
        assert_eq!(ships_of(&session).appear, 30);
        take_off(&mut session, &catalog);
        assert_eq!(
            after_entering(&catalog, &mut session, Entrance::Jump),
            (30, 0)
        );
    }

    #[test]
    fn the_hook_leaves_ships_of_another_start_another_system_or_none_left() {
        for (ships, left) in [
            (shipping(-1, 0), 2),
            (shipping(-1, 2), 2),
            (shipping(131, 1), 2),
            (shipping(-1, 1), 0),
        ] {
            let catalog = ships_catalog(ships);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            if let Some(active) = session.pilot.missions.get_mut(0) {
                active.ships.left = left;
            }
            assert_eq!(
                after_entering(&catalog, &mut session, Entrance::Jump),
                (-1, 2),
                "{ships:?} {left}"
            );
        }
        let catalog = ships_catalog(shipping(-6, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(
            after_entering(&catalog, &mut session, Entrance::Jump),
            (199, 0),
            "following the player, anywhere"
        );
    }

    #[test]
    fn a_jump_into_the_ships_system_sets_up_none_the_hook_restarts() {
        // The hook would have restarted the timer before the setup.
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = -1;
        }
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        assert!(marked(&session).is_empty(), "none set up");
        assert_eq!(ships_of(&session).appear, 199, "restarted");
        // A take-off there, with no hook, after the timer ran out sets
        // them up.
        let catalog_ticks = 199;
        ticks(&mut session, &catalog, catalog_ticks);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn a_move_in_flight_into_the_ships_system_restarts_their_timer() {
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = -1;
        }
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(ships_of(&session).appear, 199);
    }

    // Setup on each way in.

    #[test]
    fn a_gate_exit_into_the_ships_system_sets_them_up() {
        use crate::session::gate_follow::tests::{exit_gate, gated};
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = gated(shipping(131, 0), 0);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            assert!(marked(&session).is_empty(), "{kind:?}");
            exit_gate(&mut session, &catalog, kind, &mut NeverFires);
            assert_eq!(marked(&session), [0, 0], "{kind:?}");
            assert!(!session.npcs().iter().any(Npc::emerging), "{kind:?}");
        }
    }

    /// The rules with `MoveArrival` read as `source`.
    fn move_arrival(source: RuleSource) -> crate::Rulebook {
        crate::Rulebook::default().with_override(RuleKey::MoveArrival, source)
    }

    /// Lands at 140, at 131's centre, settles, and takes off again.
    fn land_and_take_off_in_131(session: &mut Session, catalog: &FakePilotCatalog) {
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.nav_target = None;
        assert_eq!(land_now(session), Ok(StellarId(140)));
        settle(session, catalog);
        take_off(session, catalog);
    }

    #[test]
    fn a_move_in_flight_into_the_ships_system_as_an_arrival_sets_them_up_at_once() {
        let catalog = ships_catalog(shipping(131, 0));
        let mut session = accepted(&catalog).with_rules(move_arrival(RuleSource::Bible));
        take_off(&mut session, &catalog);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn a_move_in_flight_by_the_engine_sets_up_none_until_the_next_setup() {
        // As the original's: `M` calls no `_SetupShipsInSystem`
        // (@0x158a4-0x159ad).
        let catalog = ships_catalog(shipping(131, 0));
        let mut session = accepted(&catalog).with_rules(move_arrival(RuleSource::Engine));
        take_off(&mut session, &catalog);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        ticks(&mut session, &catalog, 5);
        assert_eq!(session.system(), SystemId(131));
        assert!(marked(&session).is_empty(), "none until the next setup");
        land_and_take_off_in_131(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn a_move_in_flight_by_the_engine_leaves_attackers_following_the_player_behind() {
        // The original moves only the ships the player leads (@0x15807-0x1583e).
        let catalog = ships_catalog(MissionShip {
            behav: 0,
            ..shipping(-6, 0)
        });
        let mut session = accepted(&catalog).with_rules(move_arrival(RuleSource::Engine));
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "left behind");
        assert_eq!(ships_of(&session).left, 2);
        land_and_take_off_in_131(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0], "set up again");
    }

    #[test]
    fn a_take_off_restarts_no_timer() {
        let catalog = ships_catalog(shipping(-1, 1));
        let mut session = accepted(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = 5;
        }
        take_off(&mut session, &catalog);
        assert_eq!(ships_of(&session).appear, 5);
    }

    // Names.

    #[test]
    fn a_mission_ship_carries_its_missions_name_and_subtitle_while_marked() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let npc = session.npcs()[0].clone();
        assert_eq!(session.npc_name(&npc), Some("Rover"));
        assert_eq!(session.npc_subtitle(&npc), Some("Bounty Hunter"));
        let unmarked = Npc {
            mission: None,
            ..npc
        };
        assert_eq!(session.npc_name(&unmarked), Some("Ship 200"));
        assert_eq!(session.npc_subtitle(&unmarked), None);
    }

    #[test]
    fn a_mission_ship_with_no_name_is_called_after_its_ship() {
        let unnamed = MissionShip {
            name_id: None,
            subtitle: None,
            ..shipping(-1, 0)
        };
        let catalog = ships_catalog(unnamed);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let npc = session.npcs()[0].clone();
        assert_eq!(session.npc_name(&npc), Some("Ship 200"));
        assert_eq!(session.npc_subtitle(&npc), None);
    }

    #[test]
    fn a_marked_ship_with_no_subtitle_shows_its_types_not_its_persons() {
        let mut catalog = ships_catalog(MissionShip {
            name_id: None,
            subtitle: None,
            ..shipping(-1, 0)
        });
        catalog.persons = vec![PersonRecord {
            name: "Ace".into(),
            subtitle: "Top Gun".into(),
            ..crate::testkit::person(600, 200)
        }];
        let mut slots = [(None, 0); 8];
        slots[0] = (Some(PersonId(600)), 100);
        catalog.traffic = vec![(
            SystemId(130),
            SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons: slots,
            },
        )];
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let person = session.npcs().iter().find_map(|npc| npc.person);
        assert!(person.is_some(), "Ace came");
        let marked = Npc {
            person,
            ..session.npcs()[0].clone()
        };
        assert!(marked.mission.is_some());
        assert_eq!(session.npc_name(&marked), Some("Ace"), "the person's name");
        assert_eq!(session.npc_subtitle(&marked), None, "the type's subtitle");
        let unmarked = Npc {
            mission: None,
            ..marked
        };
        assert_eq!(session.npc_subtitle(&unmarked), Some("Top Gun"));
    }

    #[test]
    fn a_mission_ship_of_a_persons_name_does_not_keep_the_person_out() {
        // The mission's ships are named Rover, as is person 600, whom
        // system 130's first Person slot names at 100 %.
        let mut catalog = ships_catalog(MissionShip {
            name_id: Some(7000),
            ..shipping(-1, 0)
        });
        catalog.persons = vec![PersonRecord {
            name: "Rover".into(),
            ai_type: 3,
            ..crate::testkit::person(600, 200)
        }];
        let mut slots = [(None, 0); 8];
        slots[0] = (Some(PersonId(600)), 100);
        catalog.traffic = vec![(
            SystemId(130),
            SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons: slots,
            },
        )];
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let names: Vec<(Option<&str>, bool)> = session
            .npcs()
            .iter()
            .map(|npc| (session.npc_name(npc), npc.person.is_some()))
            .collect();
        assert_eq!(
            names,
            [
                (Some("Rover"), false),
                (Some("Rover"), false),
                (Some("Rover"), true)
            ]
        );
    }

    // Lifetime.

    #[test]
    fn a_destroyed_mission_ship_lowers_the_ships_left_so_one_fewer_comes() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let id = session.npcs()[0].id;
        session.count_lost_mission_ship(id, &mut NeverFires);
        assert_eq!(ships_of(&session).left, 1);
        session.traffic.remove(id);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0]);
        let other = session.npcs()[0].id;
        session.count_lost_mission_ship(other, &mut NeverFires);
        session.count_lost_mission_ship(other, &mut NeverFires);
        assert_eq!(ships_of(&session).left, 0, "never below none");
    }

    #[test]
    fn a_ship_destroyed_in_the_fight_is_counted() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[0].condition = crate::combat::hull::Condition::Destroyed;
        session.take_out_the_destroyed(&mut NeverFires);
        assert_eq!(ships_of(&session).left, 1);
        assert_eq!(marked(&session), [0]);
    }

    #[test]
    fn a_ship_that_is_not_a_mission_ship_counts_nothing() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[0].mission = None;
        let id = session.npcs()[0].id;
        session.count_lost_mission_ship(id, &mut NeverFires);
        assert_eq!(ships_of(&session).left, 2);
    }

    #[test]
    fn only_a_mission_ship_makes_a_save_due() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let _ = session.take_save_due();
        let (first, second) = (session.npcs()[0].id, session.npcs()[1].id);
        session.traffic.npcs_mut()[1].mission = None;
        session.count_lost_mission_ship(second, &mut NeverFires);
        assert!(!session.take_save_due(), "not a mission ship");
        session.count_lost_mission_ship(first, &mut NeverFires);
        assert_eq!(ships_of(&session).left, 1);
        assert!(session.take_save_due(), "counted");
        session.count_lost_mission_ship(first, &mut NeverFires);
        session.count_lost_mission_ship(first, &mut NeverFires);
        assert_eq!(
            (ships_of(&session).left, ships_of(&session).destroyed),
            (0, 3),
            "none left, still counted destroyed"
        );
    }

    #[test]
    fn an_aborted_missions_ships_lose_their_mark_and_stay_as_ordinary_traffic() {
        let hunters = MissionShip {
            behav: 0,
            ..shipping(-1, 0)
        };
        // Wimpy traders, which never attack unprovoked.
        let mut catalog = ships_catalog(hunters);
        catalog.dudes[0].1.ai_type = 1;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(session.npcs()[0].goal, Goal::Attack(ShipRef::Player));
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_eq!(session.npcs().len(), 2, "they stay");
        assert!(marked(&session).is_empty());
        let world = session.world();
        let around = world.around(session.npcs());
        let ai = NovaAi::default();
        let decided =
            crate::ai::Behaviour::decide(&ai, &session.npcs()[0], &around, &mut NeverFires);
        assert_ne!(decided, Goal::Attack(ShipRef::Player), "no longer hunting");
    }

    #[test]
    fn an_aborted_missions_ships_are_taken_out_while_docked() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0], "still there while docked");
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert!(session.npcs().is_empty());
    }

    #[test]
    fn an_abort_leaves_another_slots_ships_marked() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[1].mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 5,
            behav: ShipBehav::Own,
            formation: None,
        });
        session.release_mission_ships(0);
        assert_eq!(marked(&session), [5]);
    }

    #[test]
    fn a_completed_missions_ships_stay_while_docked_and_are_not_set_up_again() {
        // Offered at 128 and completed back there.
        let mut catalog = ships_catalog(shipping(-1, 0));
        catalog.missions[0].avail_stel = 128;
        catalog.missions[0].return_stel = 128;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(session.npcs().len(), 2, "not taken out, as an abort would");
        assert_eq!(marked(&session), [0, 0], "until the AI next thinks");
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "the slot brings none");
    }

    #[test]
    fn a_failed_mission_still_in_its_slot_keeps_its_ships_marked() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(crate::mission::FailedBy::Quick);
        }
        tick(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn a_captured_mission_ship_loses_its_mark_and_the_ships_left_stay() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let id = session.npcs()[0].id;
        session.lose_captured(id);
        assert_eq!(marked(&session), [0]);
        assert_eq!(ships_of(&session).left, 2);
    }

    // Losses.

    /// [`ships_catalog`] whose mission 140 has two ships of `goal` here
    /// (`ShipSyst` -1, `ShipStart` 0), `Flags` `flags`, `OnFailure` b5
    /// and `OnShipDone` b6.
    pub(in crate::session) fn goal_catalog(goal: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(MissionShip {
            goal,
            ..shipping(-1, 0)
        });
        catalog.missions[0].flags = flags;
        catalog.missions[0].on_failure = Script::parse("b5");
        catalog.missions[0].on_ship_done = Script::parse("b6");
        catalog
    }

    /// The control bits set.
    fn bits(session: &Session) -> Vec<u16> {
        session
            .pilot()
            .bits
            .iter()
            .map(crate::control::Bit::get)
            .collect()
    }

    /// Destroys the NPC at `index`, as the fight does once it has broken
    /// up.
    fn destroy(session: &mut Session, index: usize) {
        session.traffic.npcs_mut()[index].condition = Condition::Destroyed;
        session.take_out_the_destroyed(&mut NeverFires);
    }

    /// Disables the NPC at `index`, as a hit does.
    fn disable(session: &mut Session, index: usize) {
        let was: Vec<_> = session
            .npcs()
            .iter()
            .map(crate::session::NpcBefore::of)
            .collect();
        session.traffic.npcs_mut()[index].condition = Condition::Disabled;
        session.raise_disabled(&was, &mut NeverFires);
    }

    /// A session of `catalog` with mission 140 accepted and its ships in
    /// the system around the player.
    pub(in crate::session) fn in_flight(catalog: &FakePilotCatalog) -> Session {
        let mut session = accepted(catalog);
        take_off_unaborted(&mut session, catalog);
        assert_eq!(marked(&session), [0, 0]);
        session
    }

    /// [`take_off`], mission 140's ships set up as if it had no `Flags`
    /// 0x0001, whose auto-abort would end it as they come: they are here
    /// with it still active, as a swap leaves them.
    fn take_off_unaborted(session: &mut Session, catalog: &FakePilotCatalog) {
        let record = session
            .missions
            .iter_mut()
            .find(|record| record.id == MissionId(140))
            .expect("140");
        let flags = record.flags;
        record.flags &= !AUTO_ABORT;
        take_off(session, catalog);
        if let Some(record) = session
            .missions
            .iter_mut()
            .find(|record| record.id == MissionId(140))
        {
            record.flags = flags;
        }
    }

    fn failed(session: &Session) -> Option<FailedBy> {
        session
            .pilot()
            .missions()
            .get(0)
            .and_then(|active| active.failed)
    }

    #[test]
    fn the_first_ship_destroyed_fails_an_escort_disable_or_unboarded_rescue_at_once() {
        for goal in [1, 3, 5] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            let _ = session.take_messages();
            destroy(&mut session, 0);
            assert_eq!(
                session.take_messages(),
                [SimMessage::MissionShipLost],
                "{goal}"
            );
            // An escort goal's take-off check, after the setup, met it
            // with its ships here: OnShipDone (b6) ran.
            let ran: &[u16] = if goal == 3 { &[5, 6] } else { &[5] };
            assert_eq!(bits(&session), ran, "{goal}: OnFailure ran");
            assert!(
                session.pilot().missions().get(0).is_none(),
                "{goal}: abortable, so it ends"
            );
        }
    }

    #[test]
    fn an_unabortable_mission_stays_failed_and_a_second_loss_fails_nothing_more() {
        let mut catalog = goal_catalog(3, 0);
        catalog.missions[0].can_abort = 0;
        catalog.missions[0].on_failure = Script::parse("^b5");
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        assert_eq!(bits(&session), [6], "the take-off's check met it");
        destroy(&mut session, 0);
        assert_eq!(failed(&session), Some(FailedBy::Quick));
        assert_eq!(bits(&session), [5, 6]);
        assert_eq!(session.take_messages(), [SimMessage::MissionShipLost]);
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), [], "fails once");
        assert_eq!(bits(&session), [5, 6], "OnFailure ran once");
        let ships = ships_of(&session);
        assert_eq!((ships.destroyed, ships.left), (2, 0), "both counted");
    }

    #[test]
    fn a_boarded_rescue_ship_destroyed_fails_nothing() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        session.traffic.npcs_mut()[0].boarded = true;
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(failed(&session), None);
        assert_eq!(ships_of(&session).destroyed, 1);
    }

    #[test]
    fn a_destroy_or_observe_goal_counts_a_kill_and_fails_nothing() {
        for goal in [-1, 0, 4] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            let _ = session.take_messages();
            destroy(&mut session, 0);
            assert_eq!(session.take_messages(), [], "{goal}");
            assert_eq!(failed(&session), None, "{goal}");
            let ships = ships_of(&session);
            assert_eq!((ships.destroyed, ships.left), (1, 1), "{goal}");
        }
    }

    #[test]
    fn a_hidden_mission_loses_its_ship_with_no_message_and_no_failure() {
        let catalog = goal_catalog(3, 0x0400);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        destroy(&mut session, 0);
        disable(&mut session, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(failed(&session), None);
        assert_eq!(
            bits(&session),
            [6],
            "only the take-off's check's OnShipDone"
        );
        let ships = ships_of(&session);
        assert_eq!((ships.destroyed, ships.disabled), (1, 1));
    }

    #[test]
    fn the_first_escort_disabled_fails_its_mission_at_once() {
        let catalog = goal_catalog(3, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        disable(&mut session, 1);
        assert_eq!(session.take_messages(), [SimMessage::MissionShipLost]);
        assert_eq!(bits(&session), [5, 6], "OnShipDone at the take-off's check");
        assert!(session.pilot().missions().get(0).is_none());
    }

    #[test]
    fn a_disable_goals_ship_disabled_is_counted_and_fails_nothing() {
        let catalog = goal_catalog(1, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        disable(&mut session, 0);
        disable(&mut session, 1);
        assert_eq!(session.take_messages(), []);
        assert_eq!(failed(&session), None);
        assert_eq!(ships_of(&session).disabled, 2);
        assert_eq!(ships_of(&session).left, 2, "disabling leaves them");
    }

    #[test]
    fn a_ship_of_no_mission_or_of_a_free_slot_counts_nothing() {
        let catalog = goal_catalog(3, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        session.traffic.npcs_mut()[0].mission = None;
        session.traffic.npcs_mut()[1].mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 5,
            behav: ShipBehav::Own,
            formation: None,
        });
        disable(&mut session, 0);
        destroy(&mut session, 1);
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(ships_of(&session), {
            let catalog = goal_catalog(3, 0);
            ships_of(&in_flight(&catalog))
        });
    }

    // Departures.

    /// `ShipGoal` 6: chase them off.
    const CHASE_OFF: i16 = 6;

    /// A mark of slot `slot`'s ships.
    fn mark(slot: u8) -> crate::traffic::npc::MissionMark {
        crate::traffic::npc::MissionMark {
            order: None,
            slot,
            behav: ShipBehav::Own,
            formation: None,
        }
    }

    /// The ships chased off and left of slot 0's mission.
    fn chased_and_left(session: &Session) -> (i16, i16) {
        let ships = ships_of(session);
        (ships.chased_off, ships.left)
    }

    /// [`goal_catalog`] whose ships follow the player (`ShipSyst` -6).
    fn following_catalog(goal: i16) -> FakePilotCatalog {
        let mut catalog = goal_catalog(goal, 0);
        catalog.missions[0].ships.syst = -6;
        catalog
    }

    #[test]
    fn a_chase_off_ship_that_left_is_chased_off_and_lowers_the_ships_left() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_save_due();
        session.count_left_mission_ship(mark(0));
        assert_eq!(chased_and_left(&session), (1, 1));
        assert!(session.take_save_due());
        session.count_left_mission_ship(mark(0));
        session.count_left_mission_ship(mark(0));
        assert_eq!(chased_and_left(&session), (3, 0), "never below none");
    }

    #[test]
    fn a_ship_of_another_goal_that_left_counts_nothing_by_the_engine() {
        for goal in [-1, 0, 1, 2, 3, 4, 5] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            let _ = session.take_save_due();
            session.count_left_mission_ship(mark(0));
            assert_eq!(chased_and_left(&session), (0, 2), "{goal}");
            assert!(!session.take_save_due(), "{goal}");
        }
    }

    #[test]
    fn by_the_other_reading_a_ship_of_another_goal_that_left_lowers_the_ships_left() {
        let catalog = goal_catalog(0, 0);
        let mut session = in_flight(&catalog).with_rules(
            crate::Rulebook::default()
                .with_override(crate::RuleKey::LeftRespawn, crate::RuleSource::Bible),
        );
        session.count_left_mission_ship(mark(0));
        assert_eq!(chased_and_left(&session), (0, 1));
    }

    #[test]
    fn a_ship_following_the_player_that_left_counts_nothing_by_either_reading() {
        for source in [crate::RuleSource::Engine, crate::RuleSource::Bible] {
            for goal in [0, CHASE_OFF] {
                let catalog = following_catalog(goal);
                let mut session = in_flight(&catalog).with_rules(
                    crate::Rulebook::default().with_override(crate::RuleKey::LeftRespawn, source),
                );
                session.count_left_mission_ship(mark(0));
                assert_eq!(chased_and_left(&session), (0, 2), "{source:?} {goal}");
            }
        }
    }

    #[test]
    fn a_ship_of_a_free_slot_that_left_counts_nothing() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        session.count_left_mission_ship(mark(5));
        assert_eq!(chased_and_left(&session), (0, 2));
    }

    #[test]
    fn a_failed_missions_chase_off_ship_that_left_still_counts() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Quick);
        }
        session.count_left_mission_ship(mark(0));
        assert_eq!(chased_and_left(&session), (1, 1));
    }

    /// Thinks nothing new: each ship keeps its goal.
    #[derive(Debug)]
    struct Keeping;

    impl crate::ai::Behaviour for Keeping {
        fn decide(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            npc.goal
        }
    }

    /// Sends the NPC at `index` out of the system by hyperspace, from far
    /// out, on one traffic tick.
    fn jump_out(session: &mut Session, catalog: &FakePilotCatalog, index: usize) {
        let npc = &mut session.traffic.npcs_mut()[index];
        npc.goal = Goal::JumpOut;
        npc.state.position = Vec2::new(0.0, -100_000.0);
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// Lands the NPC at `index` on the system's first stellar, from rest
    /// over it, on one traffic tick.
    fn land_ship(session: &mut Session, catalog: &FakePilotCatalog, index: usize) {
        let site = session.sites[0];
        let npc = &mut session.traffic.npcs_mut()[index];
        npc.goal = Goal::Land(site.id);
        npc.state.position = site.position;
        npc.state.velocity = Vec2::ZERO;
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// Takes the NPC at `index` out of the system through gate 300, its
    /// entry's wait running out on one traffic tick.
    fn gate_out(session: &mut Session, catalog: &FakePilotCatalog, index: usize) {
        let npc = &mut session.traffic.npcs_mut()[index];
        npc.goal = Goal::EnterGate(StellarId(300));
        npc.mode = Mode::EnteringGate {
            gate: StellarId(300),
            ticks_left: 1,
        };
        session.tick_traffic(catalog, &Keeping, &mut NeverFires);
    }

    /// Whether a [`Fate::Departed`] is raised for `ship`, not yet settled.
    fn departed(session: &Session, ship: NpcId) -> bool {
        session.story_raised().contains(&StoryEvent::ShipFate {
            ship,
            fate: Fate::Departed,
        })
    }

    #[test]
    fn a_chase_off_ship_jumping_out_raises_its_departure_and_is_counted() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let id = session.npcs()[0].id;
        jump_out(&mut session, &catalog, 0);
        assert!(session.npcs().iter().all(|npc| npc.id != id), "gone");
        assert!(departed(&session, id));
        assert_eq!(chased_and_left(&session), (1, 1));
        assert_eq!(marked(&session), [0]);
    }

    #[test]
    fn a_ship_of_no_mission_jumping_out_raises_nothing() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        session.traffic.npcs_mut()[0].mission = None;
        let id = session.npcs()[0].id;
        jump_out(&mut session, &catalog, 0);
        assert!(!departed(&session, id));
        assert_eq!(chased_and_left(&session), (0, 2));
    }

    #[test]
    fn a_chase_off_goal_is_met_by_its_last_ship_leaving() {
        let catalog = returning(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let _ = shown(&mut session);
        jump_out(&mut session, &catalog, 0);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        assert_eq!(shown(&mut session), []);
        jump_out(&mut session, &catalog, 0);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6]);
    }

    #[test]
    fn a_chase_off_goal_is_met_by_a_ship_destroyed_and_one_leaving() {
        let catalog = returning(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        jump_out(&mut session, &catalog, 0);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
        let ships = ships_of(&session);
        assert_eq!((ships.destroyed, ships.chased_off, ships.left), (1, 1, 0));
        assert_eq!(bits(&session), [6]);
    }

    #[test]
    fn an_auto_abort_chase_off_mission_ends_as_its_last_ship_leaves() {
        let catalog = returning(CHASE_OFF, 0x0001);
        let mut session = in_flight(&catalog);
        jump_out(&mut session, &catalog, 0);
        jump_out(&mut session, &catalog, 0);
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "aborted");
        assert_eq!(bits(&session), [6, 7], "OnShipDone, then OnAbort");
    }

    #[test]
    fn a_chase_off_ship_that_lands_counts_nothing_by_the_engine() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let id = session.npcs()[0].id;
        land_ship(&mut session, &catalog, 0);
        assert!(session.npcs().iter().all(|npc| npc.id != id), "gone");
        assert!(!departed(&session, id));
        assert_eq!(chased_and_left(&session), (0, 2));
    }

    #[test]
    fn by_the_other_reading_a_chase_off_ship_that_lands_leaves_the_system() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog).with_rules(
            crate::Rulebook::default()
                .with_override(crate::RuleKey::LandedLeaves, crate::RuleSource::Bible),
        );
        let id = session.npcs()[0].id;
        land_ship(&mut session, &catalog, 0);
        assert!(departed(&session, id));
        assert_eq!(chased_and_left(&session), (1, 1));
    }

    #[test]
    fn a_chase_off_ship_leaving_through_a_gate_departs_but_counts_nothing_by_the_engine() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let id = session.npcs()[0].id;
        gate_out(&mut session, &catalog, 0);
        assert!(session.npcs().iter().all(|npc| npc.id != id), "gone");
        assert!(departed(&session, id), "the poll sees it gone");
        assert_eq!(chased_and_left(&session), (0, 2));
        settle(&mut session, &catalog);
        assert_eq!(
            set_up_again(&mut session, &catalog),
            [0, 0],
            "it comes back"
        );
    }

    #[test]
    fn by_the_other_reading_a_chase_off_ship_leaving_through_a_gate_is_counted() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog).with_rules(
            crate::Rulebook::default()
                .with_override(crate::RuleKey::GateExitCounts, crate::RuleSource::Bible),
        );
        let id = session.npcs()[0].id;
        gate_out(&mut session, &catalog, 0);
        assert!(departed(&session, id));
        assert_eq!(chased_and_left(&session), (1, 1));
    }

    /// The mission ships set up as the player lands at 128 and takes off
    /// again.
    fn set_up_again(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<u8> {
        assert_eq!(land_now(session), Ok(StellarId(128)));
        settle(session, catalog);
        take_off(session, catalog);
        marked(session)
    }

    #[test]
    fn a_chase_off_ship_that_left_is_not_brought_back_but_another_goals_is() {
        for (goal, again) in [(CHASE_OFF, vec![0]), (0, vec![0, 0])] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            jump_out(&mut session, &catalog, 0);
            settle(&mut session, &catalog);
            assert_eq!(set_up_again(&mut session, &catalog), again, "{goal}");
        }
    }

    #[test]
    fn by_the_other_reading_no_ship_that_left_is_brought_back() {
        for goal in [CHASE_OFF, 0] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog).with_rules(
                crate::Rulebook::default()
                    .with_override(crate::RuleKey::LeftRespawn, crate::RuleSource::Bible),
            );
            jump_out(&mut session, &catalog, 0);
            settle(&mut session, &catalog);
            assert_eq!(set_up_again(&mut session, &catalog), [0], "{goal}");
        }
    }

    #[test]
    fn a_chase_off_ship_following_the_player_that_left_is_not_counted() {
        for source in [crate::RuleSource::Engine, crate::RuleSource::Bible] {
            let catalog = following_catalog(CHASE_OFF);
            let mut session = in_flight(&catalog).with_rules(
                crate::Rulebook::default().with_override(crate::RuleKey::LeftRespawn, source),
            );
            jump_out(&mut session, &catalog, 0);
            jump_out(&mut session, &catalog, 0);
            settle(&mut session, &catalog);
            assert_eq!(chased_and_left(&session), (0, 2), "{source:?}");
            assert!(!goals_met(&session), "{source:?}");
        }
    }

    // Goals.

    /// [`goal_catalog`] whose mission 140 is offered at 128 and completed
    /// back there,
    /// paying 1000, with a `ShipDoneText` and a `CompText`.
    fn returning(goal: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = goal_catalog(goal, flags);
        catalog.missions[0].avail_stel = 128;
        catalog.missions[0].return_stel = 128;
        catalog.missions[0].pay_val = 1000;
        catalog.missions[0].on_abort = Script::parse("b7");
        catalog.mission_texts = vec![crate::catalog::MissionTexts {
            ship_done: Some("<SN> done.".to_owned()),
            comp: Some("Paid.".to_owned()),
            ..crate::catalog::MissionTexts::named(MissionId(140), "Hunt")
        }];
        catalog
    }

    /// The kinds of the mission texts shown since last taken.
    fn shown(session: &mut Session) -> Vec<(MissionTextKind, String)> {
        session
            .take_mission_texts()
            .into_iter()
            .map(|shown| (shown.kind, shown.text))
            .collect()
    }

    fn ship_done(text: &str) -> (MissionTextKind, String) {
        (MissionTextKind::ShipDone, text.to_owned())
    }

    fn goals_met(session: &Session) -> bool {
        session
            .pilot()
            .missions()
            .get(0)
            .is_some_and(|active| active.goals_met)
    }

    #[test]
    fn a_slot_filled_registers_its_goal_hooks_and_freeing_it_unregisters_them() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        let hooks = [ENTERED, LANDED, SHIP_FATE, TOOK_OFF];
        for hook in hooks {
            let id = ship_hook(0, hook);
            assert!(session.unregister_story(id), "{hook}");
            let again = Subscription::new(id, EventFilter::Any(kind_of(hook)), Script::default());
            assert_eq!(session.register_story(again), Ok(()));
        }
        let pilot = session.pilot().clone();
        let mut flown = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        for hook in hooks {
            assert!(!session.unregister_story(ship_hook(0, hook)), "{hook}");
            assert!(flown.unregister_story(ship_hook(0, hook)), "flown: {hook}");
        }
    }

    /// The event kind each hook is on.
    fn kind_of(hook: u8) -> StoryEventKind {
        match hook {
            ENTERED => StoryEventKind::EnteredSystem,
            LANDED => StoryEventKind::Landed,
            SHIP_FATE => StoryEventKind::ShipFate,
            _ => StoryEventKind::TookOff,
        }
    }

    #[test]
    fn a_destroy_goal_is_met_by_its_last_kill_showing_ship_done_and_running_on_ship_done() {
        let catalog = returning(0, 0);
        let mut session = in_flight(&catalog);
        let _ = shown(&mut session);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        assert_eq!(shown(&mut session), []);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6]);
    }

    #[test]
    fn only_a_check_that_changes_the_goals_makes_a_save_due() {
        let catalog = returning(0, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_save_due();
        session.check_ship_goals(0, false, &mut NeverFires);
        assert!(!session.take_save_due(), "unmet, as it was");
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.destroyed = 2;
            active.ships.left = 0;
        }
        session.check_ship_goals(0, false, &mut NeverFires);
        assert!(goals_met(&session));
        assert!(session.take_save_due(), "met now");
        session.check_ship_goals(0, false, &mut NeverFires);
        assert!(!session.take_save_due(), "met, as it was");
    }

    #[test]
    fn a_disable_goal_is_met_by_disabling_them_all() {
        let catalog = returning(1, 0);
        let mut session = in_flight(&catalog);
        let _ = shown(&mut session);
        disable(&mut session, 0);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        disable(&mut session, 1);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6]);
    }

    #[test]
    fn an_auto_abort_destroy_mission_ends_on_its_last_kill() {
        let catalog = returning(0, 0x0001);
        let mut session = in_flight(&catalog);
        let _ = shown(&mut session);
        destroy(&mut session, 0);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "aborted");
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6, 7], "OnShipDone, then OnAbort");
    }

    #[test]
    fn no_moment_comes_for_a_failed_mission_or_goals_met_already() {
        for (failed, met) in [(true, false), (false, true)] {
            let catalog = returning(0, 0);
            let mut session = in_flight(&catalog);
            let _ = shown(&mut session);
            if let Some(active) = session.pilot.missions.get_mut(0) {
                active.failed = failed.then_some(FailedBy::Quick);
                active.goals_met = met;
            }
            destroy(&mut session, 0);
            destroy(&mut session, 0);
            settle(&mut session, &catalog);
            assert_eq!(shown(&mut session), [], "{failed} {met}");
            assert_eq!(bits(&session), Vec::<u16>::new(), "{failed} {met}");
        }
    }

    #[test]
    fn a_hidden_disable_mission_losing_a_ship_is_marked_failed_and_fails_at_its_return() {
        let catalog = returning(1, 0x0400);
        let mut session = in_flight(&catalog);
        destroy(&mut session, 0);
        assert_eq!(failed(&session), None, "no quick failure while hidden");
        settle(&mut session, &catalog);
        assert_eq!(failed(&session), Some(FailedBy::Marked));
        assert_eq!(bits(&session), Vec::<u16>::new(), "nothing ran yet");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "failed");
        assert_eq!(bits(&session), [5], "OnFailure ran at the return");
    }

    #[test]
    fn a_destroy_goal_met_in_flight_completes_the_mission_at_its_return() {
        let catalog = returning(0, 0);
        let mut session = in_flight(&catalog);
        let cash = session.pilot().cash;
        let _ = shown(&mut session);
        destroy(&mut session, 0);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6]);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(session.pilot().cash, cash + 1000);
        assert_eq!(
            shown(&mut session),
            [(MissionTextKind::Comp, "Paid.".to_owned())]
        );
    }

    #[test]
    fn landing_before_the_goal_is_met_does_not_complete_it() {
        let catalog = returning(0, 0);
        let mut session = in_flight(&catalog);
        destroy(&mut session, 0);
        settle(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "still held");
    }

    #[test]
    fn an_escort_goal_is_met_while_its_ships_are_with_the_player() {
        let catalog = returning(3, 0);
        let mut session = accepted(&catalog);
        assert!(!goals_met(&session));
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(goals_met(&session), "met as they come in");
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        jump_to(&mut session, &catalog, 131);
        assert!(marked(&session).is_empty());
        assert!(!goals_met(&session), "unmet without them");
        jump_to(&mut session, &catalog, 130);
        settle(&mut session, &catalog);
        assert!(goals_met(&session), "met again");
        assert_eq!(
            shown(&mut session),
            [ship_done("Rover done.")],
            "a second ShipDone"
        );
        session.player = crate::flight::ShipState::default();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
    }

    #[test]
    fn an_escort_mission_landed_on_without_its_ships_is_not_completed() {
        let mut catalog = returning(3, 0);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(marked(&session).is_empty());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "still held");
        assert!(!goals_met(&session));
    }

    // Observing.

    #[test]
    fn an_observe_goal_is_met_once_the_flight_tick_sees_a_ship_of_it() {
        let catalog = returning(4, 0);
        let mut session = in_flight(&catalog);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session), "not seen until the next tick");
        let _ = shown(&mut session);
        tick(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
        assert_eq!(shown(&mut session), [ship_done("Rover done.")], "once");
        assert_eq!(bits(&session), [6]);
        tick(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert_eq!(shown(&mut session), [], "seen once");
    }

    /// [`in_flight`] of an observe goal, `change`d before the next flight
    /// tick: whether that tick's sighting meets the goal.
    fn observed_after(change: impl FnOnce(&mut Session)) -> bool {
        let catalog = returning(4, 0);
        let mut session = in_flight(&catalog);
        settle(&mut session, &catalog);
        change(&mut session);
        tick(&mut session, &catalog);
        settle(&mut session, &catalog);
        goals_met(&session)
    }

    #[test]
    fn an_observe_goal_needs_its_ships_here_all_arrived_and_its_own_ship_seen() {
        assert!(observed_after(|_| {}));
        assert!(!observed_after(|session| {
            if let Some(active) = session.pilot.missions.get_mut(0) {
                active.ships.system = ShipSystem::At(SystemId(131));
            }
        }));
        assert!(!observed_after(|session| {
            if let Some(active) = session.pilot.missions.get_mut(0) {
                active.ships.arrived = 1;
            }
        }));
        assert!(!observed_after(|session| {
            for npc in session.traffic.npcs_mut() {
                npc.mission = Some(crate::traffic::npc::MissionMark {
                    order: None,
                    slot: 5,
                    behav: ShipBehav::Own,
                    formation: None,
                });
            }
        }));
    }

    #[test]
    fn an_observe_goal_on_cloaked_ships_is_unmet_while_they_are_hidden_and_met_once_seen() {
        let full = Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        assert!(!observed_after(|session| {
            for npc in session.traffic.npcs_mut() {
                npc.cloak = full;
            }
        }));
        let catalog = returning(4, 0);
        let mut session = in_flight(&catalog);
        settle(&mut session, &catalog);
        for npc in session.traffic.npcs_mut() {
            npc.cloak = full;
        }
        let mut ticks = 0;
        while !goals_met(&session) {
            assert!(
                session.npcs().iter().all(|npc| npc.cloak.is_cloaked()),
                "met only once seen"
            );
            tick(&mut session, &catalog);
            settle(&mut session, &catalog);
            ticks += 1;
        }
        assert_eq!(ticks, 32, "each uncloaks on its think, and fades out");
    }

    #[test]
    fn an_observe_goal_on_ships_that_can_cloak_is_met_only_once_one_is_on_screen() {
        let able = crate::cloak::CloakGear {
            device: true,
            ..crate::cloak::CloakGear::default()
        };
        assert!(!observed_after(|session| {
            let far = session.player().position + Vec2::new(0.0, 5000.0);
            for npc in session.traffic.npcs_mut() {
                npc.cloak_gear = able;
                npc.state.position = far;
            }
        }));
        assert!(observed_after(|session| {
            let near = session.player().position + Vec2::new(0.0, 100.0);
            for npc in session.traffic.npcs_mut() {
                npc.cloak_gear = able;
                npc.state.position = near;
            }
        }));
    }

    #[test]
    fn only_a_sighting_of_one_of_its_own_ships_meets_an_observe_goal() {
        // Ship 0 is another slot's, and the only one watched; ship 1 of
        // this mission's is in the system, unseen.
        assert!(!observed_after(|session| {
            let first = session.npcs()[0].id;
            session.traffic.npcs_mut()[0].mission = Some(crate::traffic::npc::MissionMark {
                order: None,
                slot: 5,
                behav: ShipBehav::Own,
                formation: None,
            });
            session.story.watched.clear();
            session.watch_ship(first);
        }));
    }

    #[test]
    fn only_the_ships_numbered_after_the_last_before_are_taken_as_come() {
        let catalog = returning(4, 0);
        let mut session = in_flight(&catalog);
        session.drop_raised();
        session.story.watched.clear();
        let (first, second) = (session.npcs()[0].id, session.npcs()[1].id);
        session.mission_ships_came(Some(first));
        assert_eq!(
            session.story_raised(),
            [StoryEvent::ShipFate {
                ship: second,
                fate: Fate::Arrived
            }]
        );
        assert_eq!(session.story.watched, [second].into());
    }

    #[test]
    fn ships_of_an_observe_goal_met_already_or_of_another_goal_are_not_watched() {
        let catalog = returning(0, 0);
        let session = in_flight(&catalog);
        assert!(session.story.watched.is_empty(), "a destroy goal");
        let mut catalog = returning(4, 0);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.goals_met = true;
        }
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(marked(&session), [0, 0]);
        assert!(session.story.watched.is_empty(), "met already");
    }

    #[test]
    fn ships_of_an_observe_goal_coming_in_on_their_timer_are_watched() {
        let mut catalog = returning(4, 0);
        catalog.missions[0].ships.start = 1;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 198);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        // The tick that brings them in sees them.
        tick(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        settle(&mut session, &catalog);
        assert!(goals_met(&session));
    }

    // The first check after an accept.

    /// [`returning`] with `ships`, `Flags` `flags` and no return.
    fn shipless(ships: MissionShip, flags: u16) -> FakePilotCatalog {
        let mut catalog = returning(-1, flags);
        catalog.missions[0].ships = ships;
        catalog.missions[0].return_stel = -1;
        catalog
    }

    #[test]
    fn a_mission_accepted_has_its_goals_unmet_until_its_first_check() {
        let mut catalog = returning(-1, 0);
        catalog.missions[0].ships = MissionShip::NONE;
        let mut session = accepted(&catalog);
        assert!(!goals_met(&session), "zeroed at accept");
        assert_eq!(shown(&mut session), [], "no moment docked");
        take_off(&mut session, &catalog);
        assert!(goals_met(&session), "the take-off checks them");
        assert_eq!(shown(&mut session), [ship_done(" done.")], "no ship named");
        assert_eq!(
            bits(&session),
            Vec::<u16>::new(),
            "no OnShipDone with no goal"
        );
        session.player = crate::flight::ShipState::default();
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
    }

    #[test]
    fn an_auto_abort_mission_started_in_flight_aborts_as_the_script_settles() {
        let none = MissionShip {
            count: -1,
            ..MissionShip::NONE
        };
        let catalog = shipless(none, 0x0001);
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Names));
        tick(&mut session, &catalog);
        session.run_set(&SetExpr::parse("S140").expect("parses"), &mut NeverFires);
        assert!(session.pilot().missions().get(0).is_some());
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7], "OnAbort ran");
    }

    // No goal with ships and flag 0x0001.

    #[test]
    fn a_no_goal_auto_abort_mission_whose_ships_are_elsewhere_aborts_at_its_first_check() {
        let mut catalog = shipless(shipping(131, 0), 0x0001);
        catalog.missions[0].on_ship_done = Script::parse("b6");
        let mut session = accepted(&catalog);
        assert!(session.pilot().missions().get(0).is_some(), "not at accept");
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [7], "OnAbort once, no OnShipDone");
    }

    #[test]
    fn a_no_goal_auto_abort_mission_whose_ships_come_later_stays_until_they_come() {
        let catalog = shipless(shipping(131, 1), 0x0001);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session), "the take-off's check");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(!goals_met(&session), "the landing's check");
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        assert!(!goals_met(&session), "the entry's check");
        let appear = ships_of(&session).appear;
        ticks(
            &mut session,
            &catalog,
            usize::try_from(appear - 1).expect("positive"),
        );
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "still active");
        assert_eq!(shown(&mut session), []);
        tick(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 2, "they came");
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        settle(&mut session, &catalog);
        assert_eq!(bits(&session), [7]);
        assert_eq!(shown(&mut session), [], "no moment");
    }

    // The auto-abort as the ships spawn.

    #[test]
    fn a_flag_0x0001_mission_aborts_once_its_ships_come_in_by_hyperspace() {
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.start = 1;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 198);
        assert!(
            session.pilot().missions().get(0).is_some(),
            "the timer runs"
        );
        assert_eq!(bits(&session), Vec::<u16>::new());
        tick(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7], "OnAbort once");
        // The same tick's think after the arrivals clears their marks.
        assert_eq!(session.npcs().len(), 2, "they stay");
        assert!(marked(&session).is_empty());
        ticks(&mut session, &catalog, 2);
        assert_eq!(session.npcs().len(), 2, "as traffic");
        settle(&mut session, &catalog);
        assert_eq!(bits(&session), [7], "no OnShipDone");
    }

    #[test]
    fn a_flag_0x0001_mission_whose_ships_have_not_come_stays() {
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.start = 1;
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 300);
        assert!(session.pilot().missions().get(0).is_some(), "not here");
        assert_eq!(bits(&session), Vec::<u16>::new());
    }

    #[test]
    fn a_flag_0x0001_mission_aborts_once_its_ships_are_set_up_on_take_off() {
        let catalog = returning(0, 0x0001);
        let mut session = accepted(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(
            session.pilot().missions().get(0).is_none(),
            "auto-aborted as the take-off sets the system up"
        );
        settle(&mut session, &catalog);
        assert_eq!(bits(&session), [7], "OnAbort once");
        assert_eq!(marked(&session), [0, 0], "until the AI next thinks");
        tick(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 2, "they stay");
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn a_flag_0x0001_mission_aborts_once_its_ships_are_set_up_on_a_jump_into_their_system() {
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "elsewhere");
        jump_to(&mut session, &catalog, 131);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7]);
        tick(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 2, "they stay");
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn a_slot_the_setup_skips_is_not_aborted() {
        // Its ships elsewhere.
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "elsewhere");
        // Its appear timer running.
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.start = 2;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "timer runs");
        // Entering, the slot the enter-system hook is about to restart.
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.start = 1;
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = -1;
            active.ships.arrived = 2;
        }
        jump_to(&mut session, &catalog, 131);
        assert!(session.pilot().missions().get(0).is_some(), "restarted");
        assert!(ships_of(&session).appear > 0);
        assert_eq!(bits(&session), Vec::<u16>::new());
    }

    #[test]
    fn a_flag_0x0001_slot_of_no_dude_the_setup_takes_aborts_with_no_ship() {
        // _MissionDudeSpawn fails for each, and the call still comes.
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.dude = None;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(session.npcs().is_empty());
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7]);
    }

    #[test]
    fn two_flag_0x0001_slots_set_up_together_abort_in_slot_order() {
        // 140's OnAbort sets b7; 141's clears it and sets b8: 140 first
        // leaves b8 alone.
        let mut catalog = returning(0, 0x0001);
        let mut second = catalog.missions[0].clone();
        second.id = MissionId(141);
        second.on_abort = Script::parse("!b7 b8");
        catalog.missions.push(second);
        let mut session = accepted(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(141), &mut NeverFires),
            Ok(1)
        );
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert!(session.pilot().missions().iter().next().is_none(), "both");
        assert_eq!(bits(&session), [8], "140, then 141");
        assert_eq!(marked(&session), [0, 0, 1, 1]);
    }

    #[test]
    fn with_flag_0x0008_the_setups_auto_abort_takes_the_fuel() {
        let catalog = returning(0, 0x0009);
        let mut session = accepted(&catalog);
        let fuel = session.reserves().fuel.now;
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert!((session.reserves().fuel.now - (fuel - 100.0)).abs() < f32::EPSILON);
    }

    // The take-off's order.

    #[test]
    fn a_no_goal_auto_abort_missions_ships_here_come_before_the_take_offs_check() {
        // The setup brings them and auto-aborts it; the take-off's goal
        // check then finds the slot free, so no moment comes.
        let catalog = shipless(shipping(-1, 0), 0x0001);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7], "OnAbort once");
        assert_eq!(shown(&mut session), [], "no ShipDoneText");
        assert_eq!(marked(&session), [0, 0], "they spawned");
        tick(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 2, "they stay as traffic");
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn the_take_offs_goal_check_follows_the_setup() {
        let none = MissionShip {
            count: -1,
            ..MissionShip::NONE
        };
        let catalog = shipless(none, 0x0001);
        let mut session = accepted(&catalog);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(session.pilot().missions().get(0).is_some(), "not yet");
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(shown(&mut session), [ship_done(" done.")]);
    }

    // A landing's order.

    #[test]
    fn a_landing_rolls_the_offers_then_checks_the_goals_then_runs_the_missions_hooks() {
        use crate::session::missions::{ARRIVAL, FAILURE, REOFFER, ROLL, SUCCESS, slot_hook};
        let catalog = ships_catalog(shipping(-1, 0));
        let session = accepted(&catalog);
        assert_eq!(
            session.story_matching(&StoryEvent::Landed {
                stellar: StellarId(128)
            }),
            [
                ROLL,
                ship_hook(0, LANDED),
                slot_hook(0, ARRIVAL),
                slot_hook(0, FAILURE),
                slot_hook(0, SUCCESS),
                REOFFER,
            ]
        );
    }

    #[test]
    fn a_goal_the_landing_check_meets_completes_its_mission_at_that_landing() {
        // A destroy goal whose last kill came while docked: the landing's
        // own check meets it before the missions' landing hooks.
        let catalog = returning(0, 0);
        let mut session = in_flight(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.destroyed = 2;
            active.ships.left = 0;
        }
        let _ = shown(&mut session);
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(
            shown(&mut session),
            [
                ship_done("Rover done."),
                (MissionTextKind::Comp, "Paid.".to_owned())
            ]
        );
        assert!(
            !session
                .mission_offers(crate::mission::Venue::Computer)
                .is_empty(),
            "the offers were rolled"
        );
    }

    // Boarding.

    /// Records each crime it is asked about, and changes nothing.
    #[derive(Debug, Default)]
    struct Witness {
        seen: std::cell::RefCell<Vec<crate::legal::Crime>>,
    }

    impl crate::legal::LegalCode for Witness {
        fn penalties(
            &self,
            crime: crate::legal::Crime,
            _victim: Option<GovtId>,
            _govts: &crate::govt::Governments,
        ) -> Vec<(GovtId, i32)> {
            self.seen.borrow_mut().push(crime);
            Vec::new()
        }
    }

    /// [`returning`] of `goal` with `count` ships, `Flags` `flags`, its
    /// auto-abort paying with `Flags2` 0x0002.
    fn rescuing(goal: i16, count: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = returning(goal, flags);
        catalog.missions[0].ships.count = count;
        catalog.missions[0].flags2 = 0x0002;
        catalog
    }

    /// Boards NPC 0, disabled under the player at rest, both facing up,
    /// as `law` says: the boarding, and the crimes `law` was asked of.
    fn board_first(
        session: &mut Session,
    ) -> (
        Result<crate::board::Boarding, crate::board::BoardRefusal>,
        Vec<crate::legal::Crime>,
    ) {
        board_at(session, 0, Vec2::ZERO)
    }

    /// The velocity a ship boarded drifts at in [`board_at`]: within the
    /// boarding speed of the player at rest, so a matched velocity shows.
    const DRIFT: Vec2 = Vec2::new(0.25, -0.125);

    /// [`board_first`], boarding NPC `index`, which drifts at `velocity`
    /// as the player is at rest.
    fn board_at(
        session: &mut Session,
        index: usize,
        velocity: Vec2,
    ) -> (
        Result<crate::board::Boarding, crate::board::BoardRefusal>,
        Vec<crate::legal::Crime>,
    ) {
        session.player = crate::flight::ShipState::default();
        let npc = &mut session.traffic.npcs_mut()[index];
        npc.state = crate::flight::ShipState {
            velocity,
            ..crate::flight::ShipState::default()
        };
        npc.condition = Condition::Disabled;
        session.target = Some(session.npcs()[index].id);
        let law = Witness::default();
        let boarding = session.board(
            &law,
            &crate::board::NovaBoarding::default(),
            &mut NeverFires,
        );
        (boarding, law.seen.take())
    }

    #[test]
    fn the_assist_path_counts_a_rescue_ship_boarded_and_its_goal_auto_aborts_the_mission() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let _ = shown(&mut session);
        let (boarding, crimes) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Boarded
            )))
        );
        assert_eq!(crimes, [], "no BoardPenalty");
        assert_eq!(session.boarding(), None, "no plunder");
        assert!(session.npcs()[0].boarded);
        assert_eq!(ships_of(&session).boarded, 1);
        assert!(session.story_raised().contains(&StoryEvent::ShipFate {
            ship: session.npcs()[0].id,
            fate: Fate::Boarded,
        }));
        let cash = session.pilot().cash;
        settle(&mut session, &catalog);
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6, 7], "OnShipDone, then OnAbort");
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(session.pilot().cash, cash + 1000, "Flags2 0x0002 pays");
    }

    #[test]
    fn with_flag_0x0008_the_assist_path_says_the_refuel_and_the_auto_abort_takes_the_fuel() {
        let catalog = rescuing(5, 1, 0x0409);
        let mut session = in_flight_with(&catalog, 1);
        let fuel = session.reserves().fuel.now;
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Refuelled { ship: ShipId(200) }
            )))
        );
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert!((session.reserves().fuel.now - (fuel - 100.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn a_board_goal_is_met_on_the_assist_path_too() {
        let catalog = rescuing(2, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let _ = shown(&mut session);
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Boarded
            )))
        );
        assert_eq!(ships_of(&session).boarded, 1);
        settle(&mut session, &catalog);
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6, 7], "OnShipDone, then OnAbort");
    }

    #[test]
    fn a_mission_ship_boarded_off_the_assist_path_is_marked_boarded_and_not_counted() {
        for (goal, count, flags, pickup) in [(5, 2, 0x0001, -1), (2, 1, 0, -1), (0, 1, 0x0001, -1)]
        {
            let mut catalog = rescuing(goal, count, flags);
            catalog.missions[0].pickup_mode = pickup;
            let mut session = in_flight_with(&catalog, count);
            let (boarding, crimes) = board_first(&mut session);
            assert_eq!(
                boarding,
                Ok(crate::board::Boarding::MissionShip(None)),
                "{goal} {count} {flags:#x} {pickup}"
            );
            assert_eq!(crimes, [], "{goal}");
            assert_eq!(session.boarding(), None, "{goal}");
            assert!(session.npcs()[0].boarded, "{goal}");
            assert_eq!(ships_of(&session).boarded, 0, "{goal}");
            settle(&mut session, &catalog);
            assert!(!goals_met(&session), "{goal}");
        }
    }

    /// [`rescuing`] whose mission carries 5 tons of cargo type 1, picked
    /// up as its special ship is boarded (`PickupMode` 2).
    fn picking_up(goal: i16, count: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = rescuing(goal, count, flags);
        catalog.missions[0].pickup_mode = 2;
        catalog.missions[0].cargo_type = 1;
        catalog.missions[0].cargo_qty = 5;
        catalog
    }

    fn cargo_aboard(session: &Session) -> bool {
        session.pilot().missions().get(0).expect("held").aboard
    }

    fn boarded_fates(session: &Session) -> usize {
        session
            .story_raised()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    StoryEvent::ShipFate {
                        fate: Fate::Boarded,
                        ..
                    }
                )
            })
            .count()
    }

    /// `_HandlePlayerBoardAttempt`'s pickup path (@0x652ee-0x65570): the
    /// pickup passes, the slot counts the ship boarded (@0x653a7), it is
    /// marked boarded and its cargo goes aboard (@0x653ed), the line
    /// names the cargo, and the velocity is matched (@0x6578f).
    #[test]
    fn boarding_a_pickup_ship_takes_its_cargo_and_counts_toward_a_board_or_rescue_goal() {
        for goal in [2, 5] {
            let catalog = picking_up(goal, 1, 0x0001);
            let mut session = in_flight_with(&catalog, 1);
            let _ = shown(&mut session);
            assert!(!cargo_aboard(&session), "{goal}");
            let (boarding, crimes) = board_at(&mut session, 0, DRIFT);
            assert_eq!(
                boarding,
                Ok(crate::board::Boarding::MissionShip(Some(
                    MissionBoardLine::Retrieved { cargo: Some(1) }
                ))),
                "{goal}"
            );
            assert_eq!(crimes, [], "no BoardPenalty");
            assert_eq!(session.boarding(), None, "no plunder");
            assert!(session.strikes.is_empty(), "no help called");
            assert!(cargo_aboard(&session), "{goal}");
            assert_eq!(ships_of(&session).boarded, 1, "{goal}");
            assert!(session.npcs()[0].boarded, "{goal}");
            assert_eq!(boarded_fates(&session), 1, "{goal}");
            assert_eq!(session.player.velocity, DRIFT, "{goal}");
            assert_eq!(shown(&mut session), [], "no LoadCargText");
            settle(&mut session, &catalog);
            assert_eq!(shown(&mut session), [ship_done("Rover done.")], "{goal}");
            assert_eq!(bits(&session), [6, 7], "OnShipDone, then OnAbort");
        }
    }

    /// @0x653a7 reads no `ShipGoal` and no `Flags`: the pickup path is
    /// taken, and counts, ahead of any assist.
    #[test]
    fn a_pickup_ship_boarded_counts_whatever_its_goal_and_flags() {
        for (goal, flags) in [(0, 0), (-1, 0), (5, 0x0409)] {
            let catalog = picking_up(goal, 1, flags);
            let mut session = in_flight_with(&catalog, 1);
            let (boarding, _) = board_first(&mut session);
            assert_eq!(
                boarding,
                Ok(crate::board::Boarding::MissionShip(Some(
                    MissionBoardLine::Retrieved { cargo: Some(1) }
                ))),
                "{goal} {flags:#x}"
            );
            assert_eq!(ships_of(&session).boarded, 1, "{goal} {flags:#x}");
            assert!(cargo_aboard(&session), "{goal} {flags:#x}");
        }
    }

    /// The original re-runs the pickup for each ship of the mission
    /// boarded, its cargo aboard already counted against the free space:
    /// a second ship boarded with room for both tons counts again.
    #[test]
    fn a_second_pickup_ship_of_the_mission_boarded_with_room_counts_again() {
        let catalog = picking_up(2, 2, 0);
        let mut session = in_flight_with(&catalog, 2);
        let (first, _) = board_at(&mut session, 0, Vec2::ZERO);
        assert!(first.is_ok(), "{first:?}");
        let (second, _) = board_at(&mut session, 1, Vec2::ZERO);
        assert_eq!(
            second,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Retrieved { cargo: Some(1) }
            )))
        );
        assert_eq!(ships_of(&session).boarded, 2);
    }

    /// `_DoMissionCargoPickup` refuses (@0x98292-0x9830b) and the board
    /// jumps to @0x65fdb: the dialog, then #130, and nothing else: no
    /// count, no mark, no velocity match.
    #[test]
    fn a_pickup_refused_for_room_refuses_the_board_and_changes_nothing_else() {
        let catalog = picking_up(2, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let _ = shown(&mut session);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.cargo_tons = 100_000;
        }
        let (boarding, crimes) = board_at(&mut session, 0, DRIFT);
        assert_eq!(boarding, Err(crate::board::BoardRefusal::CantBoard));
        assert_eq!(crimes, []);
        assert!(session.strikes.is_empty(), "no help called");
        assert_eq!(session.boarding(), None);
        assert!(!cargo_aboard(&session));
        assert_eq!(ships_of(&session).boarded, 0);
        assert!(!session.npcs()[0].boarded);
        assert_eq!(boarded_fates(&session), 0);
        assert_eq!(session.player.velocity, Vec2::ZERO, "not matched");
        assert_eq!(
            shown(&mut session),
            [(
                MissionTextKind::NoRoom,
                crate::mission::cargo::NO_HOLDS_TO_LOAD.to_owned()
            )]
        );
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
    }

    /// The original's slap @0x6582b passes the boarded ship's mark, so
    /// `_SlapWithPenalty` charges no `BoardPenalty` (@0x9c10), though the
    /// ship missed the no-plunder branch (@0x657df-0x657e3).
    #[test]
    fn a_failed_missions_ship_is_boarded_for_plunder_without_board_penalty() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
        let (boarding, crimes) = board_first(&mut session);
        assert!(
            matches!(boarding, Ok(crate::board::Boarding::Opened(_))),
            "{boarding:?}"
        );
        assert_eq!(crimes, [], "no BoardPenalty");
        assert_eq!(ships_of(&session).boarded, 0);
    }

    #[test]
    fn a_ship_whose_slot_is_free_is_boarded_without_plunder() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        if let Some(npc) = session.traffic.npcs_mut().first_mut() {
            npc.mission = Some(crate::traffic::npc::MissionMark {
                order: None,
                slot: 5,
                behav: ShipBehav::Own,
                formation: None,
            });
        }
        let (boarding, crimes) = board_first(&mut session);
        assert_eq!(boarding, Ok(crate::board::Boarding::MissionShip(None)));
        assert_eq!(crimes, []);
    }

    /// [`in_flight`] for a mission of `count` ships.
    fn in_flight_with(catalog: &FakePilotCatalog, count: i16) -> Session {
        let mut session = accepted(catalog);
        take_off_unaborted(&mut session, catalog);
        assert_eq!(
            marked(&session).len(),
            usize::try_from(count).expect("count")
        );
        settle(&mut session, catalog);
        session
    }

    // Rescue ships held disabled.

    /// `ticks` of the fight, by Nova's rules.
    fn fight(session: &mut Session, ticks: usize) {
        for _ in 0..ticks {
            session.tick_combat(crate::combat::Rules::default(), &mut NeverFires);
        }
    }

    /// Fills the armour of every NPC in the system.
    fn full_armour(session: &mut Session) {
        for npc in session.traffic.npcs_mut() {
            npc.reserves.armor.now = npc.reserves.armor.max;
        }
    }

    fn conditions(session: &Session) -> Vec<Condition> {
        session.npcs().iter().map(|npc| npc.condition).collect()
    }

    fn disabled_fates(session: &Session) -> usize {
        session
            .story_raised()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    StoryEvent::ShipFate {
                        fate: Fate::Disabled,
                        ..
                    }
                )
            })
            .count()
    }

    #[test]
    fn a_rescue_ship_set_up_in_flight_is_disabled_and_stays_so_at_full_armour() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        assert_eq!(conditions(&session), [Condition::Disabled; 2], "at spawn");
        full_armour(&mut session);
        let _ = session.take_messages();
        fight(&mut session, 30);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
        assert_eq!(disabled_fates(&session), 0, "never newly disabled");
        assert_eq!(ships_of(&session).disabled, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(failed(&session), None);
    }

    #[test]
    #[allow(clippy::float_cmp)] // the reserves are set to these exact values
    fn a_rescue_ship_set_up_in_flight_starts_derelict_and_stays_disabled_once_boarded() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        for npc in session.npcs() {
            assert!(npc.stats.armor > 0.0);
            assert_eq!(npc.reserves.shield.now, 0.0);
            assert_eq!(
                npc.reserves.armor.now,
                crate::person::derelict_armor(npc.stats.armor, npc.hull.tough)
            );
        }
        for npc in session.traffic.npcs_mut() {
            npc.boarded = true;
        }
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
        assert_eq!(disabled_fates(&session), 0);

        let catalog = goal_catalog(2, 0);
        let session = in_flight(&catalog);
        for npc in session.npcs() {
            assert_eq!(npc.reserves.shield.now, npc.reserves.shield.max);
            assert_eq!(npc.reserves.armor.now, npc.reserves.armor.max);
        }
    }

    #[test]
    fn a_ship_of_another_goal_at_full_armour_is_intact() {
        for goal in [-1, 0, 1, 2, 3, 4, 6] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            assert_eq!(conditions(&session), [Condition::Intact; 2], "{goal}");
            fight(&mut session, 1);
            assert_eq!(conditions(&session), [Condition::Intact; 2], "{goal}");
        }
    }

    #[test]
    fn a_failed_rescue_missions_ships_stay_held() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
        full_armour(&mut session);
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
    }

    #[test]
    fn a_held_rescue_ship_is_boarded_as_it_spawned_and_then_released() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        assert_eq!(conditions(&session), [Condition::Disabled]);
        session.player = crate::flight::ShipState::default();
        session.traffic.npcs_mut()[0].state = crate::flight::ShipState::default();
        session.target = Some(session.npcs()[0].id);
        let boarding = session.board(
            &Witness::default(),
            &crate::board::NovaBoarding::default(),
            &mut NeverFires,
        );
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Boarded
            )))
        );
        assert_eq!(ships_of(&session).boarded, 1);
        full_armour(&mut session);
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Intact], "released");
        assert_eq!(disabled_fates(&session), 0);
    }

    #[test]
    fn a_rescue_ship_is_released_once_its_goal_is_met_unboarded() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        assert!(session.meet_ship_goal(&catalog, 0, &mut NeverFires));
        assert!(goals_met(&session));
        assert!(session.npcs().iter().all(|npc| !npc.boarded));
        full_armour(&mut session);
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Intact; 2]);
    }

    #[test]
    fn a_rescue_ship_unmarked_or_of_a_freed_slot_is_released() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        session.traffic.npcs_mut()[0].mission = None;
        session.traffic.npcs_mut()[1].mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 5,
            behav: ShipBehav::Own,
            formation: None,
        });
        full_armour(&mut session);
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Intact; 2]);
    }

    #[test]
    fn a_held_rescue_ship_that_repairs_itself_gets_its_armour_back_and_stays_disabled() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        for npc in session.traffic.npcs_mut() {
            npc.self_repair = true;
        }
        let mut chance = crate::testkit::Draws::of(&[0, 0]);
        session.self_repair(crate::combat::Rules::default(), &mut chance);
        assert_eq!(chance.asked, [crate::self_repair::REPAIR_ODDS; 2]);
        assert_eq!(conditions(&session), [Condition::Disabled; 2], "held");
        for npc in session.npcs() {
            let repaired =
                crate::self_repair::repaired_armor(npc.reserves.armor.max, npc.hull.tough);
            assert!((npc.reserves.armor.now - repaired).abs() < 1e-4);
        }
    }

    #[test]
    fn a_rescue_ship_held_at_low_armour_stays_disabled_once_released() {
        let catalog = goal_catalog(5, 0);
        let mut session = in_flight(&catalog);
        for npc in session.traffic.npcs_mut() {
            npc.boarded = true;
            npc.reserves.armor.now = 1.0;
        }
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
        assert_eq!(disabled_fates(&session), 0);
    }

    #[test]
    fn a_rescue_ship_coming_in_by_hyperspace_ends_its_glide_held_and_can_be_boarded() {
        let catalog = ships_catalog(MissionShip {
            goal: 5,
            ..shipping(-1, 1)
        });
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 199);
        assert_eq!(marked(&session), [0, 0]);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| matches!(npc.mode, Mode::JumpingIn { .. }))
        );
        // The fight holds them disabled through the glide, as
        // `_AIDispatch` lets a disabled ship's jump in run on
        // (@0x8fccb-0x8fd08).
        for _ in 0..crate::traffic::spawn::JUMP_IN_TICKS {
            fight(&mut session, 1);
            tick(&mut session, &catalog);
        }
        assert!(session.npcs().iter().all(|npc| npc.mode == Mode::Flying));
        assert_eq!(conditions(&session), [Condition::Disabled; 2], "held");
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| npc.state.position.length() < 1100.0),
            "glided in to about 1000 out"
        );
        session.player = session.npcs()[0].state;
        session.target = Some(session.npcs()[0].id);
        let boarding = session.board(
            &Witness::default(),
            &crate::board::NovaBoarding::default(),
            &mut NeverFires,
        );
        assert!(
            matches!(boarding, Ok(crate::board::Boarding::MissionShip(_))),
            "{boarding:?}"
        );
    }

    // The walker's hook.

    #[test]
    fn meeting_a_ship_goal_leaves_the_ships_as_flight_would_and_runs_the_moment() {
        for (goal, destroyed, disabled, boarded, left) in [
            (0, 2, 0, 0, 0),
            (1, 0, 2, 0, 2),
            (2, 0, 0, 2, 2),
            (5, 0, 0, 2, 2),
            (4, 0, 0, 0, 2),
        ] {
            let mut catalog = returning(goal, 0);
            catalog.missions[0].ships.start = 1;
            let mut session = accepted(&catalog);
            let _ = shown(&mut session);
            assert!(
                session.meet_ship_goal(&catalog, 0, &mut NeverFires),
                "{goal}"
            );
            let ships = ships_of(&session);
            assert_eq!(
                (
                    ships.destroyed,
                    ships.disabled,
                    ships.boarded,
                    ships.left,
                    ships.arrived
                ),
                (destroyed, disabled, boarded, left, 2),
                "{goal}"
            );
            assert!(goals_met(&session), "{goal}");
            assert_eq!(shown(&mut session), [ship_done("Rover done.")], "{goal}");
            assert_eq!(bits(&session), [6], "{goal}");
        }
    }

    #[test]
    fn meeting_a_chase_off_goal_chases_them_all_off() {
        let mut catalog = returning(CHASE_OFF, 0);
        catalog.missions[0].ships.start = 1;
        let mut session = accepted(&catalog);
        let _ = shown(&mut session);
        assert!(session.meet_ship_goal(&catalog, 0, &mut NeverFires));
        let ships = ships_of(&session);
        assert_eq!(
            (ships.chased_off, ships.destroyed, ships.left, ships.arrived),
            (2, 0, 0, 2)
        );
        assert!(goals_met(&session));
        assert_eq!(shown(&mut session), [ship_done("Rover done.")]);
        assert_eq!(bits(&session), [6]);
    }

    #[test]
    fn meeting_an_observe_goal_needs_no_ship_here() {
        let mut catalog = returning(4, 0);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        assert!(session.meet_ship_goal(&catalog, 0, &mut NeverFires));
        assert!(goals_met(&session));
    }

    #[test]
    fn meeting_an_escort_goal_brings_its_ships_in_where_the_ship_is() {
        let mut catalog = returning(3, 0);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        assert!(marked(&session).is_empty());
        assert!(session.meet_ship_goal(&catalog, 0, &mut NeverFires));
        assert_eq!(marked(&session), [0, 0], "docked at 128");
        assert!(goals_met(&session));
        let stellar = session.landed().expect("docked");
        let system = session.system();
        session
            .land_at(&catalog, system, stellar)
            .expect("docks again");
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
    }

    #[test]
    fn a_free_slot_or_a_mission_of_no_ships_or_no_goal_has_no_goal_to_meet() {
        let catalog = returning(0, 0);
        let mut session = accepted(&catalog);
        assert!(
            !session.meet_ship_goal(&catalog, 1, &mut NeverFires),
            "free"
        );
        for ships in [
            MissionShip::NONE,
            MissionShip {
                goal: -1,
                ..shipping(-1, 0)
            },
            MissionShip {
                goal: 7,
                ..shipping(-1, 0)
            },
            MissionShip {
                count: 0,
                goal: 0,
                ..shipping(-1, 0)
            },
        ] {
            let mut catalog = returning(0, 0);
            catalog.missions[0].ships = ships;
            let mut session = accepted(&catalog);
            assert!(
                !session.meet_ship_goal(&catalog, 0, &mut NeverFires),
                "{ships:?}"
            );
            assert_eq!(ships_of(&session).destroyed, 0, "{ships:?}");
        }
    }
    // The swap.

    use crate::flight::ShipState;
    use crate::hail::HailOptions;
    use crate::person::{GONE_ON_ACCEPT, LEAVES_ON_ACCEPT, REPLACED_BY_MISSION_SHIP};
    use crate::session::Hailed;

    /// Where Ace flies when hailed: at (300, -200), moving (2, -1),
    /// heading 135.
    const ACE_AT: ShipState = ShipState {
        position: Vec2::new(300.0, -200.0),
        velocity: Vec2::new(2.0, -1.0),
        heading: 135.0,
    };

    /// [`ships_catalog`] with mission 140 offered from a ship (`AvailLoc`
    /// 2), of `Flags` 0x0800, its one escort ship (`ShipCount` 1,
    /// `ShipGoal` 3, `ShipSyst` -6, `ShipStart` 0, `ShipBehav` -1) of düde
    /// 150, which flies ships 129 and 200; and "Ace" (`përs` 600, of
    /// `Flags` `flags`, flying ship 129) linked to it, whom system 130's
    /// first Person slot names at 100 %.
    fn swapping(flags: u16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(MissionShip {
            count: 1,
            goal: 3,
            syst: -6,
            ..shipping(-6, 0)
        });
        catalog.ship_records.push(ship(129, FAST));
        catalog.dudes[0].1.ships = vec![(ShipId(129), 50), (ShipId(200), 50)];
        catalog.missions[0].avail_loc = 2;
        catalog.missions[0].flags = 0x0800;
        catalog.persons = vec![PersonRecord {
            name: "Ace".into(),
            flags,
            link_mission: Some(140),
            ..crate::testkit::person(600, 129)
        }];
        let mut slots = [(None, 0); 8];
        slots[0] = (Some(PersonId(600)), 100);
        catalog.traffic = vec![(
            SystemId(130),
            SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons: slots,
            },
        )];
        catalog
    }

    /// A pilot flying in system 130 among `catalog`'s Ace (NPC 0), at
    /// [`ACE_AT`] and targeted, hailed: Ace offers mission 140.
    fn hailing_ace(catalog: &FakePilotCatalog) -> Session {
        let pilot = crate::pilot::Pilot::new(catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Names));
        session.populate(catalog, &mut NeverFires);
        session.traffic.npcs_mut()[0].state = ACE_AT;
        session.target = Some(NpcId(0));
        let hailed = session.open_hail(catalog, &HailOptions::default(), &mut NeverFires);
        assert!(matches!(hailed, Ok(Hailed::Offer(_))), "{hailed:?}");
        session
    }

    /// [`hailing_ace`], mission 140 accepted into slot 0, and settled.
    fn swapped(catalog: &FakePilotCatalog) -> Session {
        let mut session = hailing_ace(catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, catalog);
        session
    }

    fn person_here(session: &Session) -> bool {
        session.npcs().iter().any(|npc| npc.person.is_some())
    }

    #[test]
    fn accepted_on_its_hail_a_person_of_0x0040_is_replaced_by_its_missions_ship() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let session = swapped(&catalog);
        assert!(!person_here(&session), "Ace is gone");
        assert_eq!(marked(&session), [0]);
        let ship = &session.npcs()[0];
        assert_eq!(ship.state, ACE_AT, "in Ace's place");
        assert_eq!(ship.ship, ShipId(129), "flying Ace's type");
        assert_eq!(ship.govt, Some(GovtId(131)), "the düde's");
        assert_eq!(session.target, Some(ship.id), "now the target");
        assert!(!session.pilot().gone(PersonId(600)), "not gone for good");
    }

    #[test]
    fn without_mission_flags_0x0800_the_swapped_ships_type_is_drawn_from_its_dude() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].flags = 0;
        catalog.dudes[0].1.ships = vec![(ShipId(200), 100)];
        let session = swapped(&catalog);
        assert_eq!(marked(&session), [0]);
        assert_eq!(session.npcs()[0].ship, ShipId(200));
        assert_eq!(session.npcs()[0].state, ACE_AT);
        let mut unlisted = swapping(REPLACED_BY_MISSION_SHIP);
        unlisted.dudes[0].1.ships = vec![(ShipId(200), 100)];
        let session = swapped(&unlisted);
        assert_eq!(session.npcs()[0].ship, ShipId(200), "0x0800, not listed");
    }
    /// [`hailing_ace`], mission 140 refused or accepted as `accept` says,
    /// and settled: whether Ace was swapped for a mission ship.
    fn swaps(catalog: &FakePilotCatalog, accept: bool) -> bool {
        let mut session = hailing_ace(catalog);
        if accept {
            assert_eq!(
                session.accept_mission(MissionId(140), &mut NeverFires),
                Ok(0)
            );
        } else {
            assert_eq!(
                session.refuse_mission(MissionId(140), &mut NeverFires),
                Ok(())
            );
        }
        settle(&mut session, catalog);
        let swapped = !marked(&session).is_empty();
        assert_eq!(person_here(&session), !swapped);
        swapped
    }

    #[test]
    fn no_swap_for_a_refusal_a_person_without_0x0040_or_other_than_one_ship_left() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        assert!(!swaps(&catalog, false), "refused");
        assert!(!swaps(&swapping(0), true), "no 0x0040");
        for count in [2, 0] {
            let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
            catalog.missions[0].ships.count = count;
            assert!(!swaps(&catalog, true), "ShipCount {count}");
        }
    }

    #[test]
    fn no_swap_when_on_accept_ends_the_mission() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].on_accept = Script::parse("A140");
        let mut session = hailing_ace(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        assert!(session.pilot().missions().get(0).is_none(), "ended");
        settle(&mut session, &catalog);
        assert!(person_here(&session));
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn no_swap_when_the_person_or_its_missions_dude_is_gone_by_the_settling() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        session.traffic.remove(NpcId(0));
        settle(&mut session, &catalog);
        assert!(session.npcs().is_empty(), "Ace left: nothing comes");
        let mut dudeless = swapping(REPLACED_BY_MISSION_SHIP);
        dudeless.dudes.clear();
        assert!(!swaps(&dudeless, true), "no düde record");
        let mut no_dude = swapping(REPLACED_BY_MISSION_SHIP);
        no_dude.missions[0].ships.dude = None;
        assert!(!swaps(&no_dude, true), "no ShipDude");
    }

    #[test]
    fn a_boarding_offer_never_swaps() {
        use crate::board::NovaBoarding;
        use crate::legal::NovaLaw;
        use crate::person::MISSION_ON_BOARD;
        let catalog = swapping(REPLACED_BY_MISSION_SHIP | MISSION_ON_BOARD);
        let pilot = crate::pilot::Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        let mut state = *session.player();
        state.velocity = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        let ace = &mut session.traffic.npcs_mut()[0];
        ace.state = state;
        ace.condition = Condition::Disabled;
        session.target = Some(NpcId(0));
        let boarded = session.board(
            &NovaLaw::default(),
            &NovaBoarding::default(),
            &mut NeverFires,
        );
        assert!(
            matches!(boarded, Ok(crate::board::Boarding::Offered(_))),
            "{boarded:?}"
        );
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        assert!(person_here(&session));
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn a_rescue_goal_swaps_too() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.goal = 5;
        assert!(swaps(&catalog, true));
    }

    #[test]
    fn a_swapped_rescue_ship_is_disabled_at_once_with_full_reserves_until_boarded() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.goal = 5;
        catalog.missions[0].flags = 0x0409;
        let mut session = swapped(&catalog);
        assert_eq!(marked(&session), [0]);
        let ship = &session.npcs()[0];
        assert_eq!(ship.condition, Condition::Disabled, "at once");
        assert_eq!(ship.reserves, ship.stats.full(), "full reserves");
        fight(&mut session, 10);
        assert_eq!(conditions(&session), [Condition::Disabled]);
        assert_eq!(disabled_fates(&session), 0);
        session.player = ShipState::default();
        session.traffic.npcs_mut()[0].state = ShipState::default();
        let boarding = session.board(
            &Witness::default(),
            &crate::board::NovaBoarding::default(),
            &mut NeverFires,
        );
        assert!(
            matches!(
                boarding,
                Ok(crate::board::Boarding::MissionShip(Some(
                    MissionBoardLine::Refuelled { .. }
                )))
            ),
            "{boarding:?}"
        );
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Intact], "released");
    }

    #[test]
    fn a_swapped_person_of_0x0100_is_gone_for_good_and_0x0800_changes_nothing() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP | GONE_ON_ACCEPT);
        let session = swapped(&catalog);
        assert!(session.pilot().gone(PersonId(600)));
        assert_eq!(marked(&session), [0]);
        let catalog = swapping(REPLACED_BY_MISSION_SHIP | LEAVES_ON_ACCEPT);
        let session = swapped(&catalog);
        assert!(!person_here(&session), "it left by the swap");
        assert_eq!(marked(&session), [0]);
        assert_eq!(session.npcs()[0].state, ACE_AT);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn the_ship_swapped_in_for_a_hailed_person_is_targeted_settled() {
        // @0x62538: the expand is set to 0 as the target moves.
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        session.target_fly_in.start();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        let brackets = session.target_brackets().expect("bracketed");
        assert_eq!(Some(brackets.npc), session.target);
        assert_eq!(brackets.expand, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_hail_that_swaps_nothing_leaves_the_fly_in_running() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        session.target_fly_in.start();
        assert_eq!(
            session.refuse_mission(MissionId(140), &mut NeverFires),
            Ok(())
        );
        settle(&mut session, &catalog);
        assert_eq!(session.target, Some(NpcId(0)));
        assert_eq!(session.target_brackets().expect("bracketed").expand, 196.0);
    }

    #[test]
    fn the_swap_moves_no_target_but_the_persons() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        session.target = None;
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        assert_eq!(marked(&session), [0]);
        assert_eq!(session.target, None);
    }

    #[test]
    fn other_ships_let_the_swapped_person_go() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        let mut hunter = crate::testkit::npc(9, session.npcs()[0].stats);
        hunter.target = Some(ShipRef::Npc(NpcId(0)));
        hunter.goal = Goal::Attack(ShipRef::Npc(NpcId(0)));
        session.traffic.add_npc(hunter);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        let hunter = session
            .npcs()
            .iter()
            .find(|npc| npc.mission.is_none())
            .expect("the hunter");
        assert_eq!(hunter.target, None);
    }

    // Not in the fleet; no second ship.

    #[test]
    fn the_swapped_ship_joins_no_fleet_and_counts_as_the_slots_one_ship() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        let escorts = session.pilot().escorts.clone();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        let before = ships_of(&session);
        settle(&mut session, &catalog);
        assert_eq!(session.pilot().escorts, escorts);
        assert_eq!(session.npcs()[0].escort, None);
        assert_eq!(ships_of(&session), before, "no count changes");
        assert_eq!((before.left, before.arrived), (1, 1));
        for _ in 0..3 {
            tick(&mut session, &catalog);
        }
        assert_eq!(marked(&session), [0], "no second one comes");
        assert_eq!(ships_of(&session), before);
    }

    #[test]
    fn the_swapped_ship_meets_an_escort_goal_as_it_arrives() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let session = swapped(&catalog);
        assert!(session.pilot().missions().get(0).expect("held").goals_met);
        let mut observed = swapping(REPLACED_BY_MISSION_SHIP);
        observed.missions[0].ships.goal = 4;
        let session = swapped(&observed);
        let ship = session.npcs()[0].id;
        assert_eq!(session.story.watched, [ship].into(), "arrived: watched");
    }

    #[test]
    fn ships_following_the_player_set_up_one_in_the_next_system() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = swapped(&catalog);
        jump_to(&mut session, &catalog, 131);
        assert_eq!(marked(&session), [0], "one, not two");
        assert_eq!(ships_of(&session).left, 1);
    }

    #[test]
    fn ships_of_the_accepting_system_set_up_one_as_the_ship_takes_off_again() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.syst = -1;
        let mut session = swapped(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0], "one, not two");
        assert_eq!(ships_of(&session).left, 1);
    }

    // Flying with the player.

    #[test]
    fn a_swapped_ship_of_ship_behav_1_joins_the_formation_and_flies_in_it_with_the_player() {
        // Stock 132's shape: ShipGoal 3, ShipBehav 1, mission Flags 0x0800.
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.behav = 1;
        let mut session = swapped(&catalog);
        let ship = &session.npcs()[0];
        assert_eq!(ship.state, ACE_AT, "in Ace's place");
        assert!(ship.mission_escort());
        assert_eq!(ship.escort, None);
        let place = ship.formation().expect("in formation");
        assert_eq!((place.slot, place.ships), (2, 2));
        assert_eq!(ship.goal, Goal::Formation { guard: None });
        session.player.velocity = Vec2::new(1.0, 0.0);
        for _ in 0..600 {
            tick(&mut session, &catalog);
            session.player.position = session.player.position + session.player.velocity;
        }
        let ship = &session.npcs()[0];
        let off = place.position(&session.player) - ship.state.position;
        assert!(
            off.x.abs() <= crate::escort::KEEP_FORMATION
                && off.y.abs() <= crate::escort::KEEP_FORMATION,
            "{off:?}"
        );
        assert_eq!(ship.state.velocity, session.player.velocity, "with it");
    }

    #[test]
    fn a_swapped_ship_of_ship_goal_3_and_another_behaviour_does_not_join() {
        // Its fly-in-form follows a leader of -1 (step 1 of the phase):
        // it never forms on the player.
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        assert_eq!(catalog.missions[0].ships.behav, -1);
        let mut session = swapped(&catalog);
        tick(&mut session, &catalog);
        let ship = &session.npcs()[0];
        assert_eq!(marked(&session), [0]);
        assert!(!ship.escorts_player());
        assert_eq!(ship.formation(), None);
    }

    // Released by the lifetime rule.

    #[test]
    fn an_aborted_swap_missions_ship_loses_its_mark_and_stays() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = swapped(&catalog);
        let id = session.npcs()[0].id;
        assert_eq!(session.abort_mission(0, &mut NeverFires), Ok(()));
        assert_eq!(session.npcs()[0].id, id, "it stays");
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn a_completed_swap_missions_ship_is_released() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].avail_stel = 128;
        catalog.missions[0].return_stel = 128;
        let mut session = swapped(&catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn destroying_the_swapped_ship_leaves_none_to_come() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.goal = 0;
        let mut session = swapped(&catalog);
        let id = session.npcs()[0].id;
        session.count_lost_mission_ship(id, &mut NeverFires);
        assert_eq!(ships_of(&session).left, 0);
    }

    // Person escorts.

    #[test]
    fn under_the_join_reading_a_person_who_may_join_opens_the_comm_dialog_and_is_not_swapped() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.persons[0].mission_ship = Some(catalog.missions[0].ships);
        let bible = HailOptions::nova(&crate::rulebook::Rulebook::default().with_override(
            crate::rulebook::RuleKey::PersonJoin,
            crate::rulebook::RuleSource::Bible,
        ));
        let pilot = crate::pilot::Pilot::new(&catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.populate(&catalog, &mut NeverFires);
        session.target = Some(NpcId(0));
        let hailed = session.open_hail(&catalog, &bible, &mut NeverFires);
        let Ok(Hailed::Comm(view)) = hailed else {
            panic!("the comm dialog: {hailed:?}");
        };
        assert!(
            view.options
                .iter()
                .any(|button| button.label == "Use As Escort")
        );
        settle(&mut session, &catalog);
        assert!(person_here(&session));
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn the_swapped_ship_never_offers_to_join() {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.persons[0].mission_ship = Some(catalog.missions[0].ships);
        let mut session = swapped(&catalog);
        session.hang_up();
        let bible = HailOptions::nova(&crate::rulebook::Rulebook::default().with_override(
            crate::rulebook::RuleKey::PersonJoin,
            crate::rulebook::RuleSource::Bible,
        ));
        let hailed = session.open_hail(&catalog, &bible, &mut NeverFires);
        let Ok(Hailed::Comm(view)) = hailed else {
            panic!("the comm dialog: {hailed:?}");
        };
        assert!(
            view.options
                .iter()
                .all(|button| button.label != "Use As Escort")
        );
    }

    // Hails.

    /// [`ships_catalog`] of `ShipGoal` `goal` and `ShipBehav` `behav`,
    /// its ships' government 131 taking bribes, and the ship comm strings
    /// "c<n>" in `STR#` 3000.
    fn hailing(goal: i16, behav: i16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(MissionShip {
            goal,
            behav,
            ..shipping(-1, 0)
        });
        catalog.govts.push(crate::catalog::GovtRecord {
            flags: crate::hail::BRIBABLE_WARSHIPS,
            ..crate::testkit::govt(131)
        });
        catalog
            .strings
            .push((3000, (1..=200).map(|n| format!("c{n}")).collect()));
        catalog
    }

    /// A session of `catalog` in flight with its mission's ships, the
    /// first targeted, its pilot flying the "Pathfinder"; `hostile` has
    /// that ship attacking the player.
    fn hailing_session(catalog: &FakePilotCatalog, hostile: bool) -> Session {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        session.pilot.ship_name = Some("Pathfinder".to_owned());
        assert_eq!(marked(&session), [0, 0]);
        let npc = &mut session.traffic.npcs_mut()[0];
        if hostile {
            npc.goal = Goal::Attack(ShipRef::Player);
        }
        session.target = Some(npc.id);
        session
    }

    /// The opening line of the targeted ship and whether it is hostile, in
    /// variant 4, the one [`NeverFires`] draws.
    fn opening_line(session: &mut Session, catalog: &FakePilotCatalog) -> (String, bool) {
        let view = session
            .hail(catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        (view.reply, view.hostile)
    }

    #[test]
    fn a_mission_ship_to_escort_is_glad_to_see_the_players_ship() {
        let catalog = hailing(3, -1);
        let mut session = hailing_session(&catalog, false);
        assert_eq!(
            opening_line(&mut session, &catalog),
            ("c45Pathfinder.".to_owned(), false)
        );
    }

    #[test]
    fn a_mission_ship_to_destroy_asks_what_the_player_wants() {
        let catalog = hailing(0, -1);
        let mut session = hailing_session(&catalog, false);
        assert_eq!(
            opening_line(&mut session, &catalog),
            ("c15".to_owned(), false)
        );
    }

    #[test]
    fn a_hostile_mission_ship_to_board_opens_the_channel() {
        let catalog = hailing(2, -1);
        let mut session = hailing_session(&catalog, true);
        assert_eq!(
            opening_line(&mut session, &catalog),
            ("c5".to_owned(), true)
        );
    }

    #[test]
    fn a_ship_still_marked_after_its_mission_left_its_slot_opens_the_channel() {
        let catalog = hailing(3, 0);
        let mut session = hailing_session(&catalog, true);
        let record = catalog.missions[0].clone();
        session.auto_abort(0, &record, &mut NeverFires);
        assert_eq!(marked(&session), [0, 0], "until the AI next thinks");
        assert_eq!(
            opening_line(&mut session, &catalog),
            ("c5".to_owned(), true),
            "neither its goal nor its behaviour, nor its attitude"
        );
        let view = session
            .answer(1, &catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        assert!(view.asking.is_some(), "bribed as any ship is");
    }

    #[test]
    fn a_mission_ship_attacking_the_player_will_not_be_bribed() {
        let catalog = hailing(0, 0);
        let mut session = hailing_session(&catalog, true);
        let view = session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        assert_eq!(view.reply, "c15");
        assert_eq!(view.options[1].label, "Beg For Mercy");
        let mut draws = crate::testkit::Draws::of(&[]);
        let view = session
            .answer(1, &catalog, &HailOptions::default(), &mut draws)
            .expect("answered");
        assert_eq!(view.reply, "c100");
        assert!(view.asking.is_none(), "no price asked");
        assert!(draws.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn a_mission_ship_of_its_own_behaviour_names_its_price() {
        let catalog = hailing(0, -1);
        let mut session = hailing_session(&catalog, true);
        session
            .hail(&catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        let view = session
            .answer(1, &catalog, &HailOptions::default(), &mut NeverFires)
            .expect("answered");
        assert!(view.asking.is_some(), "a price asked");
    }
}
