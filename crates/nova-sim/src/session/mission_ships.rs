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
//! no script: [`ENTERED`] on every `EnteredSystem`, [`SHIP_FATE`] on
//! every `ShipFate` and [`TOOK_OFF`] on every `TookOff`. A landing's goal
//! checks are the slot's `{Missions, slot, hook}` hooks
//! [`GOALS`](super::missions::GOALS) and
//! [`RECHECK`](super::missions::RECHECK), in `_MissionLandCheck`'s order. The enter-system hook's effect reads the event's
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
//! [`check_goals`]): the landing's per slot, before its cargo and again
//! after its return, after the offer roll; the take-off's as the first flight frame after
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
//! (`_EnterMoreShips` @0x43abc-0x43ad4), whether or not a ship found a
//! slot. Its ships stay in the system,
//! losing their marks as the next think finds the slot free. The
//! original aborts each slot inside setup's slot loop; here the aborts
//! follow once the system's ships are all in, in slot order.
//! Each mission ship takes a ship-table slot below the spawn reserve
//! (`_MissionDudeSpawn` @0x3cd50, [`Session::spawn_reserve`]); one that
//! finds none does not come and changes no count: the setup goes on to
//! the next ship (@0x43128), and the timer, spent all the same, counts
//! as arrived only the ships that came (@0x43971), so with `ShipStart` 1
//! the rest are due again once the timer restarts.
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
//! under mission `Flags` 0x0800 when the `düde` lists it, and that type
//! becomes the slot's, so later setups bring it; else, under the flag,
//! the slot's type. It takes the first ship-table slot free below the
//! spawn reserve while the person still holds its own, so never the
//! person's. The person's NPC is then taken out of the system (ship
//! +0xb8 cleared, @0x62520), without being gone for good, and the
//! player's target moves to the ship. Nothing is swapped when the ship
//! cannot be spawned or finds no slot (@0x62481 → @0x62593): the person
//! stays, targeted as it was, and the mission's ship is left for a later
//! setup. The ship
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
//! **One ship type** (mission `Flags` 0x0800): at accept, before
//! `OnAccept`, the slot draws one type from its `düde` as the session
//! read it at start, among the types whose `AppearOn` holds then, or
//! with none among them all ([`Session::ships_at_accept`]); every ship of
//! the slot then flies it, on all three paths. A slot of the flag with
//! no type (a save from before it was kept, or a `düde` the catalog
//! could not give at start) draws it at its next spawn from the
//! traffic's `düde` and keeps it ([`Session::uniform_preset`]).
//!
//! **Mission escorts** are the [`mission_escorts`](super::mission_escorts)
//! module's: `ShipBehav` 1 ships join the player's formation as they come
//! ([`Session::mission_ships_came`]) and on every traffic tick, and leave
//! it as their marks are cleared.

use super::Session;
use crate::board::{BoardRefusal, Boarding, MissionBoardLine};
use std::collections::BTreeMap;

use crate::catalog::{DudeId, MissionId, MissionRecord, PilotCatalog, ShipId, TrafficCatalog};
use crate::chance::Chance;
use crate::combat::hull::Condition;
use crate::control::{Script, Test};
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::mission::active::AUTO_ABORT;
use crate::mission::cargo::PICKUP_BOARDING;
use crate::mission::screens::HIDDEN;
use crate::mission::ships::{
    NOT_APPEARING, ShipBehav, ShipFacts, ShipStart, UNIFORM_SHIP_TYPE, appear_timer,
    swapped_ship_type,
};
use crate::mission::{
    ActiveMission, AssistLine, Departure, FailedBy, GoalFacts, LossFacts, ShipGoal, ShipSystem,
    SpecialShips, assists, check_goals, departure, fails_on_destroyed, fails_on_disabled,
    goals_met_moment, held_disabled, met_before, plain_board_counts,
};
use crate::rulebook::{RuleKey, RuleSource};
use crate::session::mission_screens::MissionTextKind;
use crate::sound::SimSound;
use crate::story::{
    Consumer, Entrance, EventFilter, Fate, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::traffic::SPAWN_RESERVE;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{MissionMark, Npc, NpcId};
use crate::traffic::spawn::{self, MissionSpawn, NewShip};
use crate::traffic::table::{SpawnDude, SpawnTable, dude_ship_type};

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
/// A slot's goal check on every `ShipFate`.
pub const SHIP_FATE: u8 = 2;
/// A slot's goal check on `TookOff`: the first flight frame after a
/// docked accept or a landing, held until the system is set up.
pub const TOOK_OFF: u8 = 3;

/// Every hook a slot registers, each on its event, in order.
const HOOKS: [(u8, StoryEventKind); 3] = [
    (ENTERED, StoryEventKind::EnteredSystem),
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

/// The `düde`s of `missions` of `Flags` 0x0800 ([`UNIFORM_SHIP_TYPE`])
/// with a `ShipDude`, read from `catalog` ([`PilotCatalog::mission_dude`]),
/// each as the traffic spawns from it: what such a mission draws its
/// ships' one type from at accept. One that cannot be read is left out,
/// and its type is drawn at the ships' first spawn.
pub(super) fn uniform_dudes(
    missions: &[MissionRecord],
    catalog: &(impl PilotCatalog + ?Sized),
) -> BTreeMap<DudeId, SpawnDude> {
    missions
        .iter()
        .filter(|record| record.flags & UNIFORM_SHIP_TYPE != 0)
        .filter_map(|record| record.ships.dude)
        .filter_map(|dude| Some((dude, SpawnDude::from(&catalog.mission_dude(dude)?))))
        .collect()
}

/// What the special ships' part did to a slot's timer
/// ([`Session::special_part`]): the timer after it, and how many came
/// when it ran out.
struct SpecialPart {
    appear: i16,
    came: Option<i16>,
}

impl Session {
    /// The special ships of `record`, just accepted as `active`, as
    /// `_LoadCurrentMissionData` loads them before `OnAccept`
    /// ([`SpecialShips::at_accept`]), from the player's system now, over
    /// the overlaid map and the governments, their names from the string
    /// lists ([`Session::strings`]), drawing on `chance`. Under `Flags`
    /// 0x0800 their one type is drawn from their `düde` as the session
    /// read it ([`dude_ship_type`]), its `AppearOn` read now, before
    /// `OnAccept` runs.
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
        let mut draw_type = |dude: DudeId, chance: &mut dyn Chance| {
            let dude = self.uniform_dudes.get(&dude)?;
            dude_ship_type(dude, |ship| self.appear_on(ship), self.gate(), chance)
        };
        SpecialShips::at_accept(
            &record.ships,
            record.flags,
            &facts,
            (self.galaxy.map(), &self.govts),
            self.strings(),
            &mut draw_type,
            chance,
        )
    }

    /// Ship type `ship`'s `AppearOn`: none when it has no record.
    fn appear_on(&self, ship: ShipId) -> Option<&Test> {
        self.ships
            .iter()
            .find(|record| record.id == ship)
            .map(|record| &record.appear_on)
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
            (TOOK_OFF, StoryEvent::TookOff { .. }) => {
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
        let done_before = ships.ship_done;
        let check = check_goals(&facts);
        if (check.met != facts.met || (check.failed && !failed_before))
            && let Some(active) = self.slot_holding(slot, record.id)
        {
            active.goals_met = check.met;
            if check.failed {
                active.failed.get_or_insert(FailedBy::Marked);
            }
        }
        let once = self.rules.source_for(RuleKey::ShipDoneOnce) == RuleSource::Bible;
        if goals_met_moment(met_before(&facts), check, failed_before) && !(once && done_before) {
            self.ship_goals_met(slot, &record, chance);
        }
    }

    /// The moment the goals of `record`, the mission in `slot`, are met
    /// (@0x9ec92-0x9ed87), drawing on `chance`: its moment marked come
    /// ([`SpecialShips::ship_done`], under either reading of
    /// [`RuleKey::ShipDoneOnce`]), its `ShipDoneText`, its
    /// `OnShipDone` with a `ShipGoal`, then with `Flags` 0x0001 its
    /// auto-abort, while the slot still holds it.
    fn ship_goals_met(&mut self, slot: u8, record: &MissionRecord, chance: &mut dyn Chance) {
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.ships.ship_done = true;
        }
        self.show_mission_text(MissionTextKind::ShipDone, record.id, Some(slot), chance);
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
        &mut self,
        table: &SpawnTable,
        entering: Option<Entrance>,
        chance: &mut (impl Chance + ?Sized),
    ) -> (Vec<NewShip>, Vec<(u8, MissionId)>) {
        let nav = self.nav_slots();
        let mut ships = Vec::new();
        let mut set_up = Vec::new();
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            let Some(active) = self.pilot.missions.get(slot) else {
                continue;
            };
            let record = self.lifecycle_record(active.mission);
            let state = &active.ships;
            let left = state.left;
            let expired = active.days_left == Some(0);
            if !self.ships_here(state.system)
                || left <= 0
                || state.appear >= 0
                || entering.is_some_and(|how| self.hook_restarts(&record, active, how))
            {
                continue;
            }
            set_up.push((slot, record.id));
            let Some(dude) = record.ships.dude else {
                continue;
            };
            let preset = self.uniform_preset(slot, &record, Some(table), chance);
            let spawn = MissionSpawn {
                slot,
                dude,
                count: left,
                goal: ShipGoal::from_code(record.ships.goal),
                start: ShipStart::from_code(record.ships.start),
                behav: ShipBehav::from_code(record.ships.behav),
                nav: &nav,
                preset,
                expired,
            };
            ships.extend(spawn::mission_ships(table, &spawn, self.gate(), chance));
        }
        (ships, set_up)
    }

    /// The one ship type the special ships of `record`, the mission in
    /// `slot`, fly under `Flags` 0x0800 ([`UNIFORM_SHIP_TYPE`]): the
    /// slot's ([`SpecialShips::uniform_type`]), or, for a slot that has
    /// none, one drawn now from its `düde` in `table` (else the traffic's
    /// table) as at accept ([`dude_ship_type`]) on `chance`, and kept.
    /// None without the flag, so each ship draws its own.
    fn uniform_preset(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        table: Option<&SpawnTable>,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<ShipId> {
        let kept = self.pilot.missions.get(slot)?.ships.uniform_type;
        self.uniform_preset_kept(slot, record, kept, table, chance)
    }

    /// [`Session::uniform_preset`] with `kept` as the slot's kept type, as
    /// read by the caller: the freed slot's stale record for the
    /// fall-through after an auxiliary auto-abort.
    fn uniform_preset_kept(
        &mut self,
        slot: u8,
        record: &MissionRecord,
        kept: Option<ShipId>,
        table: Option<&SpawnTable>,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<ShipId> {
        if record.flags & UNIFORM_SHIP_TYPE == 0 {
            return None;
        }
        if kept.is_some() {
            return kept;
        }
        let table = table.unwrap_or_else(|| self.traffic.table());
        let dude = table.dude_records.get(&record.ships.dude?)?;
        let drawn = dude_ship_type(dude, |ship| table.appear_on(ship), self.gate(), chance);
        if let Some(active) = self.slot_holding(slot, record.id) {
            active.ships.uniform_type = drawn;
        }
        drawn
    }

    /// The way from the player's system to the one it last jumped from,
    /// on the map: none before a jump, or when either is not on it.
    fn way_back(&self) -> Option<Vec2> {
        let from = self.galaxy.position(self.pilot.jumped_from?)?;
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
    ///
    /// Each slot's auxiliary part ([`Session::aux_arrivals`]) comes first.
    /// When its auto-abort frees the slot, under the engine's reading of
    /// [`RuleKey::AuxAbortFallThrough`] the special part runs on the
    /// freed slot's record as it stood before
    /// ([`Session::freed_special_arrivals`], @0x43757-0x43ad4); by the
    /// other reading the freed slot is skipped. The stale countdown tick
    /// that follows (@0x43ae8-0x43af8) is not built: it hits a dead record
    /// the next accept redraws (@0xa0e5f).
    ///
    /// On the gate exit's arrivals frame (`hook_due`) each slot's
    /// auxiliary part and countdown tick are left to the enter-system
    /// hook, which restarts the countdown after this frame where the
    /// original restarts it first, so one countdown tick is lost. That
    /// crosses no boundary the player sees, so it is not built.
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
                let before = self.pilot.missions.get(slot).cloned();
                self.aux_arrivals(catalog, slot, chance);
                let freed = !self.pilot.missions.holds(slot, mission);
                if freed
                    && self.rules.source_for(RuleKey::AuxAbortFallThrough) == RuleSource::Engine
                {
                    if let Some(before) = before {
                        self.freed_special_arrivals(catalog, slot, &before, chance);
                    }
                    // The countdown tick (@0x43ae8-0x43af8) hits the freed
                    // record, which the next accept redraws (@0xa0e5f):
                    // not built, as no one can see it.
                    continue;
                }
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
        let active = active.clone();
        if entering.is_some_and(|how| self.hook_restarts(&record, &active, how)) {
            return;
        }
        let Some(part) = self.special_part(catalog, slot, &record, &active, chance) else {
            return;
        };
        // The timer is kept, not saved, as it runs: no save is due.
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.ships.appear = part.appear;
        }
        let Some(count) = part.came else {
            return;
        };
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.ships.arrived = active.ships.arrived.saturating_add(count);
        }
        self.abort_on_spawn(slot, &record, &mut &mut *chance);
    }

    /// The special ships' part of `_EnterMoreShips` run on `before`, the
    /// record of `slot` as it stood before its auxiliary part's
    /// auto-abort freed it, as the original falls into the part at
    /// @0x43757 with no new test of the slot's active byte, under the
    /// engine's reading of [`RuleKey::AuxAbortFallThrough`], drawing on
    /// `chance`. When the stale timer runs out, the special ships come,
    /// marked with the freed slot (the sweep after the frame's arrivals,
    /// `unmark_inactive_mission_ships`, makes them traffic), and a mission of `Flags` 0x0001 auto-aborts again
    /// (@0x43abc-0x43ad4; [`Session::auto_abort_freed`]). Nothing is
    /// written back to the freed slot: the stale timer's countdown when
    /// it does not run out (@0x437ec-0x437fc) and its arrivals hit a dead
    /// record, which the next accept reloads, so they are not built.
    fn freed_special_arrivals(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        before: &ActiveMission,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let record = self.lifecycle_record(before.mission);
        let Some(part) = self.special_part(catalog, slot, &record, before, chance) else {
            return;
        };
        if part.came.is_some() && record.flags & AUTO_ABORT != 0 {
            self.auto_abort_freed(slot, &record, before.pay, &mut &mut *chance);
        }
    }

    /// The core of the special ships' part (@0x43757-0x43aa5) for `active`,
    /// a mission of `record` in `slot`, drawing on `chance`: nothing
    /// unless its ships are here with any to come (ships left, less those
    /// arrived for `ShipStart` 1); else a running timer counts down (from
    /// 32000 or below), and at 0 it stops and they come in by hyperspace.
    /// Writes nothing to the slot.
    fn special_part(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        record: &MissionRecord,
        active: &ActiveMission,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<SpecialPart> {
        let state = &active.ships;
        if !self.ships_here(state.system) {
            return None;
        }
        let coming = if ShipStart::from_code(record.ships.start) == ShipStart::ComesLater {
            state.left - state.arrived
        } else {
            state.left
        };
        if coming <= 0 {
            return None;
        }
        let mut appear = state.appear;
        if (1..=COUNTED).contains(&appear) {
            appear -= 1;
        }
        // With any to come, ships are left.
        if appear != 0 {
            return Some(SpecialPart { appear, came: None });
        }
        let kept = state.uniform_type;
        let count = self.bring_in_mission_ships(catalog, slot, record, kept, coming, chance);
        Some(SpecialPart {
            appear: -1,
            came: Some(count),
        })
    }

    /// Brings `coming` special ships of `record`, the mission in `slot`,
    /// its kept uniform type `kept`, into the system by hyperspace ([`spawn::arriving_mission_ships`]),
    /// their `düde`'s ships read from `catalog`, drawing on `chance`
    /// (@0x43821-0x43aa5): how many came, none for a mission of no
    /// `düde`. Each takes a slot below the spawn reserve
    /// ([`Session::spawn_reserve`]), and one that finds none does not come
    /// and is not counted, as `_EnterMoreShips` skips the count and the
    /// placement on `_MissionDudeSpawn`'s -1 (@0x43964 → @0x43aaa).
    fn bring_in_mission_ships(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        record: &MissionRecord,
        kept: Option<ShipId>,
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
        let preset = self.uniform_preset_kept(slot, record, kept, None, chance);
        let spawn = MissionSpawn {
            slot,
            dude,
            count: coming,
            goal: ShipGoal::from_code(record.ships.goal),
            start: ShipStart::from_code(record.ships.start),
            behav: ShipBehav::from_code(record.ships.behav),
            nav: &nav,
            preset,
            // Only the setup reads the time left (@0x42fe1-0x42ffa).
            expired: false,
        };
        let ships = spawn::arriving_mission_ships(
            self.traffic.table(),
            &spawn,
            self.way_back(),
            self.gate(),
            chance,
        );
        let last = self.last_npc();
        let came = self
            .traffic
            .bring_in_within(ships, self.spawn_reserve(SPAWN_RESERVE));
        self.mission_ships_came(last);
        i16::try_from(came).unwrap_or(i16::MAX)
    }

    /// Swaps the person's NPC `npc`, its mission just accepted on its hail
    /// into `slot`, for that mission's one ship (see the module docs),
    /// the `düde` read from `catalog`, drawing on `chance`
    /// (`_HandlePlayerCommunication` @0x6244b-0x6252f): the ship, of the
    /// person's type under mission `Flags` 0x0800 when the `düde` lists it
    /// ([`swapped_ship_type`]), kept as the slot's type, else of the
    /// slot's type ([`Session::uniform_preset`]), takes the person's place
    /// ([`spawn::swapped_mission_ship`]); the person's NPC is taken out,
    /// every other NPC letting it go; the ship raises [`Fate::Arrived`];
    /// and it becomes the player's target when the person was, its
    /// brackets settled (@0x62538; see the `brackets` module). The
    /// original also marks the target panel for redraw
    /// (`_statusRedraw[6]`, @0x62543); ours gets that from the ship's
    /// fresh `NpcId`, on which `nova_view::flight::cadence::retake_target`
    /// redraws the panel at once. By the engine's reading of
    /// [`RuleKey::SwapArrives`] no count of the slot changes and its appear
    /// timer runs on; by the other, the ship is the timer's arrival: the
    /// timer stops and, for `ShipStart` 1, the arrived count goes up by
    /// one. The ship takes its slot below the spawn reserve
    /// ([`Session::spawn_reserve`]) while the person still holds its own,
    /// as the original frees the person's slot only after the spawn
    /// (@0x62520). Nothing when the NPC is no longer here, the slot no
    /// longer holds a mission of a `düde`, or the ship cannot be spawned
    /// or finds no slot (@0x62481 → @0x62593): the person stays, targeted
    /// as it was. Of a `ShipBehav` 1 mission it joins the player's
    /// formation as it comes ([`Session::mission_ships_came`]); of
    /// `ShipBehav` 0 it hunts the player. Of any other, for a `ShipGoal` 3
    /// mission, it holds where it came ([`Goal::Hold`](crate::ai::Goal::Hold)), coasting, as
    /// `_AIMakeEscortFlyInForm` (@0x62509, @0x89bbd-0x89bcd) gives it goal
    /// 10 with no leader to follow, until provoked or its type's think
    /// lifts the hold; under [`RuleKey::LeaderlessSwapHold`]'s other
    /// reading, and for any other `ShipGoal`, it decides as its `düde`'s AI
    /// (see the [`mission_escorts`](super::mission_escorts) module).
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
        let swapped = swapped_ship_type(record.flags, &dude_ships, person_ship);
        self.traffic.table_mut().add_mission_dudes(
            catalog,
            &[dude],
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        // `_MissionDudeSpawn` finds its slot (@0x3cd50) before it hands the
        // slot the person's type (@0x3cd7e) or draws one (@0x3cdea): with
        // no slot nothing of the mission changes.
        let reserve = self.spawn_reserve(SPAWN_RESERVE);
        if self.traffic.free_berth(reserve).is_none() {
            return;
        }
        let preset = if let Some(ship) = swapped {
            if let Some(active) = self.slot_holding(slot, record.id) {
                active.ships.uniform_type = Some(ship);
            }
            Some(ship)
        } else {
            self.uniform_preset(slot, &record, None, chance)
        };
        let spawn = MissionSpawn {
            slot,
            dude,
            count: 1,
            goal: ShipGoal::from_code(record.ships.goal),
            start: ShipStart::from_code(record.ships.start),
            behav: ShipBehav::from_code(record.ships.behav),
            nav: &[],
            preset: None,
            // Only the setup reads the time left (@0x42fe1-0x42ffa).
            expired: false,
        };
        let Some(mut ship) = spawn::swapped_mission_ship(
            self.traffic.table(),
            &spawn,
            state,
            preset,
            self.gate(),
            chance,
        ) else {
            return;
        };
        if self.rules.source_for(RuleKey::LeaderlessSwapHold) == RuleSource::Bible {
            ship.held = false;
        }
        let last = self.last_npc();
        let came = self.traffic.bring_in_within(vec![ship], reserve);
        if came == 0 {
            return;
        }
        let targeted = self.target == Some(npc);
        self.drop_quarry(npc);
        self.traffic.remove(npc);
        // The ship brought in above is numbered last.
        let swapped_in = self.last_npc();
        if targeted {
            self.target = swapped_in;
            self.target_fly_in.settle();
        }
        // The hail's dialog closes on the player's target, now the ship
        // swapped in (@0x6252f), which `_offeringShipIndex` takes
        // (@0x625a4): `<OSN>` names its slot, which holds no person. The
        // original swaps as the dialog closes, when the hailed ship is
        // always the target; this settling comes after, so a target the
        // player moved since is kept, and `<OSN>` names the swapped-in
        // ship all the same.
        if let Some(npc) = swapped_in.and_then(|id| self.npc(id)) {
            self.offering_ship = Some(super::person_offers::OfferingShip { berth: npc.berth });
        }
        if self.rules.source_for(RuleKey::SwapArrives) == RuleSource::Bible
            && let Some(active) = self.slot_holding(slot, record.id)
        {
            active.ships.appear = NOT_APPEARING;
            if ShipStart::from_code(record.ships.start) == ShipStart::ComesLater {
                active.ships.arrived = active.ships.arrived.saturating_add(1);
            }
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
            let kept = active.ships.uniform_type;
            self.bring_in_mission_ships(catalog, slot, &record, kept, count, chance);
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
    /// ([`assists`]), on which its slot counts it boarded, the comm line
    /// is said and the ship holds its course for 100 ticks (its AI wait
    /// +0x4c, @0x65746; see the `board_attempt` module). Off both paths,
    /// a board counts only by the other reading of
    /// [`RuleKey::PlainBoardCounts`] ([`plain_board_counts`]), with no
    /// comm line. Each comm line said, on the pickup path and the assist
    /// path, beeps [`SimSound::MissionShipBoarded`] (`_beepSnd[4]`
    /// @0x6556b, @0x656ee); the refused pickup's `_beepSnd[3]` (@0x65ffe)
    /// is the board refusal's. Then it is marked boarded (@0x65f43), with
    /// no `BoardPenalty`, person's offer or plunder. Neither path tests
    /// whether the mission failed, so a failed mission's ship takes them
    /// too, and is then boarded for plunder, still with no `BoardPenalty`
    /// (the slap @0x6582b passes its mark; see [`Session::board`]): none
    /// here, its line held for [`Session::take_board_line`].
    pub(super) fn board_mission_ship(
        &mut self,
        npc: &Npc,
    ) -> Option<Result<Boarding, BoardRefusal>> {
        let mark = npc.mission?;
        let mut line = None;
        let mut failed = false;
        if let Some(active) = self.pilot.missions.get(mark.slot) {
            failed = active.failed.is_some();
            let record = self.lifecycle_record(active.mission);
            let cargo = active.cargo_type;
            let goal = ShipGoal::from_code(record.ships.goal);
            let counts = if record.pickup_mode == PICKUP_BOARDING {
                if !self.pick_up_boarded_cargo(mark.slot) {
                    return Some(Err(BoardRefusal::CantBoard));
                }
                line = Some(MissionBoardLine::Retrieved { cargo });
                true
            } else {
                let assist = assists(goal, record.flags, record.pickup_mode, active.ships.left);
                if let Some(assist) = assist {
                    line = Some(match assist {
                        AssistLine::Boarded => MissionBoardLine::Boarded,
                        AssistLine::Refuelled => MissionBoardLine::Refuelled { ship: npc.ship },
                    });
                    self.hold_boarded(npc.id);
                }
                assist.is_some()
                    || plain_board_counts(goal, self.rules.source_for(RuleKey::PlainBoardCounts))
            };
            if counts && let Some(active) = self.slot_holding(mark.slot, record.id) {
                active.ships.boarded = active.ships.boarded.saturating_add(1);
            }
        }
        if line.is_some() {
            self.sounds.push(SimSound::MissionShipBoarded);
        }
        if failed {
            self.board_line = line;
            return None;
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
    /// left, nor did an escape pod freed (see `nova_sim::traffic::pod_flight`).
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
                    // An escape pod's slot freed silently (+0xb8 = 0
                    // @0x8fbb3) raises no fate.
                    Outcome::Landed(_)
                    | Outcome::Docked { .. }
                    | Outcome::Flying
                    | Outcome::Freed => {
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
    /// ([`Session::quick_failure`], @0x2dbba, @0x3af71). A hidden mission
    /// (`Flags` 0x0400) fails only by the other reading of
    /// [`RuleKey::HiddenLossFails`], and then with no message. Its slot and
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
            hidden_fails: self.rules.source_for(RuleKey::HiddenLossFails) == RuleSource::Bible,
        };
        let fails = match loss {
            Loss::Destroyed => fails_on_destroyed(facts),
            Loss::Disabled => fails_on_disabled(facts),
        };
        if fails {
            if record.flags & HIDDEN == 0 {
                self.say(SimMessage::MissionShipLost);
                self.sounds.push(SimSound::MissionShipLost);
            }
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
                uniform_type: None,
                ship_done: false,
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

    /// Fills the system with disabled ships, which never leave, until
    /// `berths` of the ship table are taken.
    pub(in crate::session) fn crowd(session: &mut Session, berths: usize) {
        while session.npcs().len() < berths {
            let mut hulk = crate::testkit::npc(0, crate::stats::ShipStats::default());
            hulk.condition = Condition::Disabled;
            session.traffic.add_npc(hulk);
        }
    }

    /// A session of `catalog` taken off with mission 140's ships due on
    /// the next tick, `berths` of the ship table taken.
    fn due_crowded(catalog: &FakePilotCatalog, berths: usize) -> Session {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        ticks(&mut session, catalog, 198);
        crowd(&mut session, berths);
        session
    }

    #[test]
    fn ships_coming_on_their_timer_find_no_slot_once_55_ships_are_in_and_arrive_none() {
        let catalog = crate::testkit::far_129_reachable(ships_catalog(shipping(-1, 1)));
        let mut session = due_crowded(&catalog, 55);
        let left = ships_of(&session).left;
        tick(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "no slot (@0x43964)");
        let ships = ships_of(&session);
        assert_eq!(
            (ships.arrived, ships.appear, ships.left),
            (0, -1, left),
            "the timer spent, none counted (@0x4382c, @0x43971)"
        );
        ticks(&mut session, &catalog, 300);
        assert!(marked(&session).is_empty(), "not due again in this system");
        jump_to(&mut session, &catalog, 131);
        jump_to(&mut session, &catalog, 130);
        assert_eq!(ships_of(&session).appear, 199, "restarted on arrival");
        ticks(&mut session, &catalog, 199);
        assert_eq!(marked(&session), [0, 0], "with room they come");
        assert_eq!(ships_of(&session).arrived, 2);
    }

    #[test]
    fn a_timer_with_room_for_one_of_three_brings_one_and_counts_one() {
        let catalog = ships_catalog(MissionShip {
            count: 3,
            ..shipping(-1, 1)
        });
        let mut session = due_crowded(&catalog, 54);
        tick(&mut session, &catalog);
        let marked: Vec<u32> = session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some())
            .map(|npc| npc.berth)
            .collect();
        assert_eq!(marked, [55], "the one slot left");
        assert_eq!(ships_of(&session).arrived, 1, "only the one that came");
    }

    #[test]
    fn by_the_bible_reading_of_spawn_reserve_mission_ships_take_slots_above_55() {
        let catalog = ships_catalog(shipping(-1, 1));
        let rules =
            crate::Rulebook::default().with_override(RuleKey::SpawnReserve, RuleSource::Bible);
        let mut session = accepted(&catalog).with_rules(rules);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 198);
        crowd(&mut session, 55);
        tick(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        assert_eq!(ships_of(&session).arrived, 2);
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

    /// `session`'s pilot saved and loaded into a new session, its
    /// `jumped_from` first set by `edit`.
    fn saved_and_loaded(
        session: &Session,
        catalog: &FakePilotCatalog,
        edit: impl FnOnce(&mut crate::pilot::Pilot),
    ) -> Session {
        let mut pilot = session.pilot().clone();
        edit(&mut pilot);
        let pilot = crate::save::decode(&crate::save::encode(&pilot)).expect("loads");
        Session::fly(catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Names))
    }

    /// Runs the loaded session's first traffic tick, which sets its
    /// system up, then the ships' appear timer down.
    fn until_they_appear(session: &mut Session, catalog: &FakePilotCatalog) {
        tick(session, catalog);
        let appear = ships_of(session).appear;
        assert!(appear > 0, "running");
        ticks(session, catalog, usize::try_from(appear).expect("positive"));
    }

    #[test]
    fn ships_coming_in_after_a_load_enter_from_the_system_jumped_from_before_the_save() {
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        let mut session = saved_and_loaded(&session, &catalog, |_| {});
        until_they_appear(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        for npc in session.npcs().iter().filter(|npc| npc.mission.is_some()) {
            assert!(npc.state.position.x < -1500.0, "{:?}", npc.state.position);
        }
    }

    #[test]
    fn ships_coming_in_after_loading_a_save_with_no_jumped_from_still_arrive() {
        let catalog = ships_catalog(shipping(131, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        let mut session = saved_and_loaded(&session, &catalog, |pilot| pilot.jumped_from = None);
        until_they_appear(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
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
        session.nav = crate::navigation::NavMode::Off;
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

    /// How many [`SimSound::MissionShipLost`] beeps are pending, taken.
    fn lost_beeps(session: &mut Session) -> usize {
        session
            .take_sounds()
            .iter()
            .filter(|sound| **sound == crate::SimSound::MissionShipLost)
            .count()
    }

    /// How many [`SimSound::MissionShipBoarded`] beeps are pending, taken.
    fn boarded_beeps(session: &mut Session) -> usize {
        session
            .take_sounds()
            .iter()
            .filter(|sound| **sound == crate::SimSound::MissionShipBoarded)
            .count()
    }

    #[test]
    fn the_first_ship_destroyed_fails_an_escort_disable_or_unboarded_rescue_at_once() {
        for goal in [1, 3, 5] {
            let catalog = goal_catalog(goal, 0);
            let mut session = in_flight(&catalog);
            let _ = session.take_messages();
            let _ = session.take_sounds();
            destroy(&mut session, 0);
            assert_eq!(
                session.take_messages(),
                [SimMessage::MissionShipLost],
                "{goal}"
            );
            assert_eq!(lost_beeps(&mut session), 1, "{goal}: _beepSnd[1]");
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
        assert_eq!(lost_beeps(&mut session), 1);
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), [], "fails once");
        assert_eq!(lost_beeps(&mut session), 0, "beeps once");
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
        assert_eq!(lost_beeps(&mut session), 0);
        assert_eq!(failed(&session), None);
        assert_eq!(
            bits(&session),
            [6],
            "only the take-off's check's OnShipDone"
        );
        let ships = ships_of(&session);
        assert_eq!((ships.destroyed, ships.disabled), (1, 1));
    }

    /// The rulebook with [`RuleKey::HiddenLossFails`] read as `source`.
    fn hidden_loss(source: crate::RuleSource) -> crate::Rulebook {
        crate::Rulebook::default().with_override(RuleKey::HiddenLossFails, source)
    }

    #[test]
    fn by_the_engine_a_hidden_rescue_losing_a_ship_is_still_held_and_not_failed() {
        let catalog = goal_catalog(5, 0x0400);
        let mut session = in_flight(&catalog).with_rules(hidden_loss(crate::RuleSource::Engine));
        let _ = session.take_messages();
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), []);
        assert_eq!(lost_beeps(&mut session), 0);
        assert_eq!(failed(&session), None);
        assert_eq!(bits(&session), [] as [u16; 0]);
        assert!(session.pilot().missions().get(0).is_some(), "still held");
    }

    #[test]
    fn by_the_other_reading_a_hidden_mission_losing_a_ship_fails_quietly() {
        for can_abort in [1, 0] {
            let mut catalog = goal_catalog(5, 0x0400);
            catalog.missions[0].can_abort = can_abort;
            let mut session = in_flight(&catalog).with_rules(hidden_loss(crate::RuleSource::Bible));
            let _ = session.take_messages();
            destroy(&mut session, 0);
            assert_eq!(session.take_messages(), [], "{can_abort}: no message");
            assert_eq!(lost_beeps(&mut session), 0, "{can_abort}: no beep");
            assert_eq!(bits(&session), [5], "{can_abort}: OnFailure ran");
            if can_abort == 0 {
                assert_eq!(failed(&session), Some(FailedBy::Quick), "held, failed");
            } else {
                assert!(session.pilot().missions().get(0).is_none(), "it ended");
            }
        }
    }

    #[test]
    fn the_first_escort_disabled_fails_its_mission_at_once() {
        let catalog = goal_catalog(3, 0);
        let mut session = in_flight(&catalog);
        let _ = session.take_messages();
        let _ = session.take_sounds();
        disable(&mut session, 1);
        assert_eq!(session.take_messages(), [SimMessage::MissionShipLost]);
        assert_eq!(lost_beeps(&mut session), 1, "_beepSnd[1]");
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
    fn a_ship_jumping_out_on_double_times_second_pass_raises_its_departure() {
        let catalog = goal_catalog(CHASE_OFF, 0);
        let mut session = in_flight(&catalog);
        let id = session.npcs()[0].id;
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.goal = Goal::JumpOut;
        npc.state.position = Vec2::new(0.0, -100_000.0);
        session.tick_again(
            crate::flight::Controls::default(),
            crate::Rules::default(),
            &mut NeverFires,
        );
        assert!(session.npcs().iter().all(|npc| npc.id != id), "gone");
        assert!(departed(&session, id));
        assert_eq!(chased_and_left(&session), (1, 1));
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
            .take_texts_only()
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
        let hooks = [ENTERED, SHIP_FATE, TOOK_OFF];
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

    /// The rulebook with [`RuleKey::ShipDoneOnce`] read as `source`.
    fn ship_done_once(source: crate::RuleSource) -> crate::Rulebook {
        crate::Rulebook::default().with_override(RuleKey::ShipDoneOnce, source)
    }

    /// How many `ShipDone` texts were shown since last taken.
    fn ship_dones(session: &mut Session) -> usize {
        shown(session)
            .iter()
            .filter(|(kind, _)| *kind == MissionTextKind::ShipDone)
            .count()
    }

    /// [`jump_to`], the tank filled first.
    fn jump_refuelled(session: &mut Session, catalog: &FakePilotCatalog, to: i16) {
        let fuel = &mut session.pilot.reserves.fuel;
        fuel.now = fuel.max;
        jump_to(session, catalog, to);
    }

    /// [`returning`]'s escort mission taken off with, then away from its
    /// ships and back twice, under `source` of [`RuleKey::ShipDoneOnce`]:
    /// the `ShipDone` texts shown at each moment its goals could be met,
    /// and the bits set by then.
    fn escort_round_trips(source: crate::RuleSource) -> (Vec<usize>, Vec<u16>) {
        let catalog = returning(3, 0);
        let mut session = accepted(&catalog).with_rules(ship_done_once(source));
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(goals_met(&session), "{source:?}: met at take-off");
        let mut done = vec![ship_dones(&mut session)];
        for _ in 0..2 {
            jump_refuelled(&mut session, &catalog, 131);
            assert!(!goals_met(&session), "{source:?}: unmet without them");
            jump_refuelled(&mut session, &catalog, 130);
            settle(&mut session, &catalog);
            assert!(goals_met(&session), "{source:?}: met again");
            done.push(ship_dones(&mut session));
        }
        assert!(ships_of(&session).ship_done, "{source:?}: its moment came");
        (done, bits(&session))
    }

    #[test]
    fn an_escort_goal_runs_its_ship_done_each_time_its_ships_come_back_by_the_engine() {
        let (done, bits) = escort_round_trips(crate::RuleSource::Engine);
        assert_eq!(done, [1, 1, 1]);
        assert_eq!(bits, [6]);
    }

    #[test]
    fn by_the_other_reading_an_escort_goals_ship_done_runs_once() {
        let (done, bits) = escort_round_trips(crate::RuleSource::Bible);
        assert_eq!(done, [1, 0, 0]);
        assert_eq!(bits, [6], "OnShipDone ran at take-off");
    }

    #[test]
    fn by_the_other_reading_a_later_moment_runs_no_ship_done_script() {
        let mut catalog = returning(3, 0);
        catalog.missions[0].on_ship_done = Script::parse("^b6");
        for (source, toggled) in [
            (crate::RuleSource::Engine, true),
            (crate::RuleSource::Bible, false),
        ] {
            let mut session = accepted(&catalog).with_rules(ship_done_once(source));
            take_off(&mut session, &catalog);
            settle(&mut session, &catalog);
            assert_eq!(bits(&session), [6], "{source:?}");
            jump_refuelled(&mut session, &catalog, 131);
            jump_refuelled(&mut session, &catalog, 130);
            settle(&mut session, &catalog);
            assert_eq!(
                bits(&session).is_empty(),
                toggled,
                "{source:?}: toggled back"
            );
        }
    }

    #[test]
    fn the_ship_done_flag_survives_a_save_so_the_other_reading_still_runs_it_once() {
        let catalog = returning(3, 0);
        let bible = ship_done_once(crate::RuleSource::Bible);
        let mut session = accepted(&catalog).with_rules(bible);
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert_eq!(ship_dones(&mut session), 1);
        jump_refuelled(&mut session, &catalog, 131);
        assert!(!goals_met(&session));
        let mut session = saved_and_loaded(&session, &catalog, |_| {}).with_rules(bible);
        assert!(ships_of(&session).ship_done, "kept");
        jump_refuelled(&mut session, &catalog, 130);
        settle(&mut session, &catalog);
        assert!(goals_met(&session), "met again");
        assert_eq!(ship_dones(&mut session), 0, "not again after the load");
    }

    #[test]
    fn an_escort_goal_met_at_its_return_has_had_no_moment_so_the_other_reading_still_runs_it() {
        let mut catalog = returning(3, 0);
        catalog.missions[0].ships.syst = 131;
        catalog.missions[0].travel_stel = 140;
        let mut session = accepted(&catalog).with_rules(ship_done_once(crate::RuleSource::Bible));
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(!goals_met(&session), "the trailing check unmet it again");
        // The escort branch alone, as the landing's return hook runs it
        // (@0xa1a99-0xa1ab1): goals met, destination not reached.
        session.mission_effect(
            crate::session::missions::slot_hook(0, crate::session::missions::RETURN),
            &StoryEvent::Landed {
                stellar: StellarId(128),
            },
            &mut NeverFires,
        );
        assert!(goals_met(&session), "the escort branch met it");
        assert!(!ships_of(&session).ship_done, "with no moment");
        assert_eq!(ship_dones(&mut session), 0);
        take_off(&mut session, &catalog);
        assert!(!goals_met(&session), "unmet without them");
        jump_to(&mut session, &catalog, 131);
        settle(&mut session, &catalog);
        assert!(goals_met(&session), "met with them");
        assert_eq!(ship_dones(&mut session), 1, "its first moment");
        assert!(ships_of(&session).ship_done);
    }

    #[test]
    fn an_escort_mission_landed_on_at_its_return_without_its_ships_completes() {
        let mut catalog = returning(3, 0);
        catalog.missions[0].ships.syst = 131;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert!(marked(&session).is_empty());
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(
            session.pilot().missions().get(0).is_none(),
            "the escort branch met it, none destroyed (@0xa1a99-0xa1ab1)"
        );
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
    fn a_flag_0x0001_mission_auto_aborts_when_its_timer_fires_with_no_slot() {
        let mut catalog = returning(0, 0x0001);
        catalog.missions[0].ships.start = 1;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 198);
        crowd(&mut session, 55);
        tick(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "none came");
        assert!(
            session.pilot().missions().get(0).is_none(),
            "auto-aborted all the same (@0x43abc)"
        );
        assert_eq!(bits(&session), [7], "OnAbort once");
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

    /// The take-off's pin: `_portDialog` is cleared at @0x60251, before
    /// `_SetupShipsInSystem` @0x674c2, so `_AbortMission` skips the
    /// removal at @0x99b20 and the ships stay as traffic.
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

    // Missions started mid-landing (`_MissionLandCheck` reads each slot
    // as it reaches it, @0xa19f7-0xa1aec).

    /// Mission `id` of no goal, offered at 128 and completed back there,
    /// paying `pay`, running `on_success`.
    fn back_home(id: i16, pay: i32, on_success: &str) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            return_stel: 128,
            pay_val: pay,
            on_success: Script::parse(on_success),
            ..mission(id)
        }
    }

    /// Mission `id`'s texts: its `ShipDoneText` and its `CompText`, when
    /// given.
    fn texts(id: i16, ship_done: Option<&str>, comp: Option<&str>) -> crate::catalog::MissionTexts {
        crate::catalog::MissionTexts {
            ship_done: ship_done.map(str::to_owned),
            comp: comp.map(str::to_owned),
            ..crate::catalog::MissionTexts::named(MissionId(id), "Run")
        }
    }

    fn comp(text: &str) -> (MissionTextKind, String) {
        (MissionTextKind::Comp, text.to_owned())
    }

    /// [`catalog`] with `missions` and their `texts`: a session of it
    /// docked at 128 with `accepted` accepted in turn, taken off (each
    /// one's goals checked) and in flight at 128, its texts taken.
    fn launched(
        missions: Vec<MissionRecord>,
        mission_texts: Vec<crate::catalog::MissionTexts>,
        accepted: &[i16],
    ) -> (FakePilotCatalog, Session) {
        let mut catalog = catalog();
        catalog.missions = missions;
        catalog.mission_texts = mission_texts;
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        for &id in accepted {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
            settle(&mut session, &catalog);
        }
        take_off(&mut session, &catalog);
        let _ = shown(&mut session);
        (catalog, session)
    }

    /// Lands `session` on 128 and settles.
    fn land_home(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(land_now(session), Ok(StellarId(128)));
        settle(session, catalog);
    }

    fn held(session: &Session) -> Vec<(u8, i16)> {
        session
            .pilot()
            .missions()
            .iter()
            .map(|(slot, active)| (slot, active.mission.0))
            .collect()
    }

    /// Slot 0's 140 completes at 128 with `OnSuccess` `S141 S142`: 141
    /// fills the slot just freed, 142 slot 1.
    fn two_started() -> (FakePilotCatalog, Session) {
        launched(
            vec![
                back_home(140, 0, "S141 S142"),
                back_home(141, 300, ""),
                back_home(142, 500, "b42"),
            ],
            vec![
                texts(140, None, Some("140 paid.")),
                texts(141, Some("141 met."), Some("141 paid.")),
                texts(142, None, Some("142 paid.")),
            ],
            &[140],
        )
    }

    #[test]
    fn a_mission_a_second_s_in_an_earlier_slots_on_success_starts_completes_at_that_landing() {
        let (catalog, mut session) = two_started();
        let cash = session.pilot().cash();
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [(0, 141)], "142 completed, slot 1 free");
        assert!(session.control_bit(crate::control::Bit::new(42).expect("in range")));
        assert_eq!(session.pilot().cash(), cash + 500);
        assert_eq!(
            shown(&mut session),
            [comp("140 paid."), ship_done("141 met."), comp("142 paid.")]
        );
    }

    #[test]
    fn the_mission_an_s_in_on_success_puts_in_the_freed_slot_is_goal_checked_but_not_completed_at_that_landing()
     {
        let (catalog, mut session) = two_started();
        land_home(&mut session, &catalog);
        let _ = shown(&mut session);
        assert!(goals_met(&session), "the trailing check met 141's goals");
        let cash = session.pilot().cash();
        take_off(&mut session, &catalog);
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), []);
        assert_eq!(shown(&mut session), [comp("141 paid.")]);
        assert_eq!(session.pilot().cash(), cash + 300);
    }

    #[test]
    fn a_mission_started_mid_landing_with_flags_0x0001_auto_aborts_at_that_landing() {
        let (catalog, mut session) = launched(
            vec![
                back_home(140, 0, "S143"),
                MissionRecord {
                    flags: AUTO_ABORT,
                    on_abort: Script::parse("b43"),
                    ..back_home(143, 0, "")
                },
            ],
            vec![texts(143, Some("143 met."), None)],
            &[140],
        );
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [], "143 auto-aborted by the trailing check");
        assert!(session.control_bit(crate::control::Bit::new(43).expect("in range")));
        assert_eq!(shown(&mut session), [ship_done("143 met.")]);
    }

    #[test]
    fn a_mission_started_into_a_lower_slot_than_the_one_running_waits_for_the_take_off() {
        let (catalog, mut session) = launched(
            vec![
                back_home(140, 0, ""),
                back_home(144, 0, "S141"),
                back_home(141, 0, ""),
            ],
            vec![texts(141, Some("141 met."), None)],
            &[140, 144],
        );
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [(0, 141)]);
        assert_eq!(shown(&mut session), [], "slot 0 is not visited again");
        assert!(!goals_met(&session));
        take_off(&mut session, &catalog);
        assert_eq!(shown(&mut session), [ship_done("141 met.")]);
    }

    /// `_DoMissionFailure` runs `OnFailure` (@0xa02ef) before it frees the
    /// slot (@0xa034f), so an `S` there takes another slot, which the
    /// landing reaches later.
    #[test]
    fn an_s_in_on_failure_takes_a_later_slot_checked_in_full_at_that_landing() {
        let (catalog, mut session) = launched(
            vec![
                MissionRecord {
                    on_failure: Script::parse("S142"),
                    ..back_home(140, 0, "")
                },
                back_home(142, 500, "b42"),
            ],
            vec![texts(142, None, Some("142 paid."))],
            &[140],
        );
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [], "140 failed, 142 completed in slot 1");
        assert!(session.control_bit(crate::control::Bit::new(42).expect("in range")));
        assert_eq!(shown(&mut session), [comp("142 paid.")]);
    }

    #[test]
    fn an_s_in_an_auto_aborts_on_abort_refilling_the_slot_gets_its_return_check_at_that_landing() {
        let (catalog, mut session) = launched(
            vec![
                MissionRecord {
                    on_abort: Script::parse("S142"),
                    ..back_home(145, 0, "")
                },
                back_home(142, 500, "b42"),
            ],
            vec![texts(142, None, Some("142 paid."))],
            &[145],
        );
        // Unmet, with `Flags` 0x0001: the landing's leading check meets
        // its goals and auto-aborts it.
        if let Some(record) = session
            .missions
            .iter_mut()
            .find(|record| record.id == MissionId(145))
        {
            record.flags = AUTO_ABORT;
        }
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.goals_met = false;
        }
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [], "142 completed in slot 0");
        assert!(session.control_bit(crate::control::Bit::new(42).expect("in range")));
        assert_eq!(shown(&mut session), [comp("142 paid.")]);
    }

    /// [`returning`]'s goalless mission 140 of `Flags` 0x0001 in flight,
    /// its two ships here and its goals unmet, `OnAbort` `on_abort`.
    fn auto_aborting_in_flight(on_abort: &str) -> (FakePilotCatalog, Session) {
        let mut catalog = returning(-1, AUTO_ABORT);
        catalog.missions[0].on_abort = Script::parse(on_abort);
        catalog.missions.push(mission(141));
        let mut session = in_flight(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.goals_met = false;
        }
        (catalog, session)
    }

    #[test]
    fn a_flag_0x0001_mission_whose_goals_the_landing_meets_takes_its_ships_out_while_docked() {
        let (catalog, mut session) = auto_aborting_in_flight("b7");
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [], "auto-aborted by the landing's check");
        assert!(bits(&session).contains(&7), "OnAbort ran");
        assert!(
            session.npcs().is_empty(),
            "removed under the port dialog (@0x99b22)"
        );
    }

    #[test]
    fn an_s_in_the_on_abort_of_a_landing_auto_abort_starts_a_mission_with_no_ships_carried_over() {
        let (catalog, mut session) = auto_aborting_in_flight("S141");
        land_home(&mut session, &catalog);
        assert_eq!(
            held(&session),
            [(1, 141)],
            "OnAbort runs before the slot is freed"
        );
        assert!(marked(&session).is_empty(), "140's ships not carried over");
        assert!(session.npcs().is_empty());
    }

    /// [`returning`]'s escort mission in flight with none of its ships
    /// left in the system, its goals unmet and `destroyed` destroyed.
    fn escort_gone(destroyed: i16) -> (FakePilotCatalog, Session) {
        let catalog = returning(3, 0);
        let mut session = in_flight(&catalog);
        for npc in session.traffic.npcs_mut() {
            npc.mission = None;
        }
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.goals_met = false;
            active.ships.destroyed = destroyed;
        }
        let _ = shown(&mut session);
        (catalog, session)
    }

    #[test]
    fn an_escort_mission_at_its_return_with_none_destroyed_completes_with_its_ships_gone() {
        let (catalog, mut session) = escort_gone(0);
        let cash = session.pilot().cash();
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), []);
        assert_eq!(shown(&mut session), [comp("Paid.")]);
        assert_eq!(session.pilot().cash(), cash + 1000);
    }

    #[test]
    fn an_escort_mission_at_its_return_with_one_destroyed_fails_before_the_escort_branch() {
        let (catalog, mut session) = escort_gone(1);
        let cash = session.pilot().cash();
        assert!(!bits(&session).contains(&5));
        land_home(&mut session, &catalog);
        assert_eq!(held(&session), [], "the slot is freed");
        assert!(bits(&session).contains(&5), "OnFailure ran");
        assert!(!shown(&mut session).contains(&comp("Paid.")));
        assert_eq!(session.pilot().cash(), cash);
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

        fn system_penalties(
            &self,
            _crime: crate::legal::Crime,
            _victim: Option<GovtId>,
            _from: crate::catalog::SystemId,
            _map: &dyn crate::legal::CrimeMap,
            _govts: &crate::govt::Governments,
        ) -> Vec<(crate::catalog::SystemId, f32)> {
            Vec::new()
        }
    }

    /// [`returning`] of `goal` with `count` ships, `Flags` `flags`, its
    /// auto-abort paying with `Flags2` 0x0002.
    pub(in crate::session) fn rescuing(goal: i16, count: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = returning(goal, flags);
        catalog.missions[0].ships.count = count;
        catalog.missions[0].flags2 = 0x0002;
        catalog
    }

    /// Boards NPC 0, disabled under the player at rest, both facing up,
    /// as `law` says: the boarding, and the crimes `law` was asked of.
    pub(in crate::session) fn board_first(
        session: &mut Session,
    ) -> (
        Result<crate::board::Boarding, crate::board::BoardRefusal>,
        Vec<crate::legal::Crime>,
    ) {
        board_at(session, 0, Vec2::ZERO)
    }

    /// The velocity a ship boarded drifts at in [`board_at`]: within the
    /// boarding speed of the player at rest, so a matched velocity shows.
    pub(in crate::session) const DRIFT: Vec2 = Vec2::new(0.25, -0.125);

    /// [`board_first`], boarding NPC `index`, which drifts at `velocity`
    /// as the player is at rest.
    pub(in crate::session) fn board_at(
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
        let _ = session.take_sounds();
        let (boarding, crimes) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Boarded
            )))
        );
        assert_eq!(boarded_beeps(&mut session), 1, "_beepSnd[4]");
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
        let _ = session.take_sounds();
        let (boarding, _) = board_first(&mut session);
        assert_eq!(
            boarding,
            Ok(crate::board::Boarding::MissionShip(Some(
                MissionBoardLine::Refuelled { ship: ShipId(200) }
            )))
        );
        assert_eq!(boarded_beeps(&mut session), 1, "_beepSnd[4]");
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
            let mut session =
                in_flight_with(&catalog, count).with_rules(plain_board(crate::RuleSource::Engine));
            let _ = session.take_sounds();
            let (boarding, crimes) = board_first(&mut session);
            assert_eq!(boarded_beeps(&mut session), 0, "{goal}: nothing said");
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

    /// The rulebook with [`RuleKey::PlainBoardCounts`] read as `source`.
    fn plain_board(source: crate::RuleSource) -> crate::Rulebook {
        crate::Rulebook::default().with_override(RuleKey::PlainBoardCounts, source)
    }

    #[test]
    fn by_the_other_reading_any_board_of_a_board_or_rescue_ship_counts() {
        for (goal, count, flags, boarded) in [(5, 2, 0x0001, 1), (2, 1, 0, 1), (0, 1, 0x0001, 0)] {
            let catalog = rescuing(goal, count, flags);
            let mut session =
                in_flight_with(&catalog, count).with_rules(plain_board(crate::RuleSource::Bible));
            let _ = shown(&mut session);
            let _ = session.take_sounds();
            let (boarding, crimes) = board_first(&mut session);
            assert_eq!(boarded_beeps(&mut session), 0, "{goal}: nothing said");
            assert_eq!(
                boarding,
                Ok(crate::board::Boarding::MissionShip(None)),
                "{goal}"
            );
            assert_eq!(crimes, [], "{goal}");
            assert!(session.npcs()[0].boarded, "{goal}");
            assert_eq!(ships_of(&session).boarded, boarded, "{goal}");
            settle(&mut session, &catalog);
            let met = goal == 2;
            assert_eq!(goals_met(&session), met, "{goal}");
            assert_eq!(
                shown(&mut session).contains(&ship_done("Rover done.")),
                met,
                "{goal}"
            );
        }
    }

    /// [`rescuing`] whose mission carries 5 tons of cargo type 1, picked
    /// up as its special ship is boarded (`PickupMode` 2).
    pub(in crate::session) fn picking_up(goal: i16, count: i16, flags: u16) -> FakePilotCatalog {
        let mut catalog = rescuing(goal, count, flags);
        catalog.missions[0].pickup_mode = 2;
        catalog.missions[0].cargo_type = 1;
        catalog.missions[0].cargo_qty = 5;
        catalog
    }

    pub(in crate::session) fn cargo_aboard(session: &Session) -> bool {
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
            let _ = session.take_sounds();
            let (boarding, crimes) = board_at(&mut session, 0, DRIFT);
            assert_eq!(boarded_beeps(&mut session), 1, "{goal}: _beepSnd[4]");
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
        let _ = session.take_sounds();
        let (boarding, crimes) = board_at(&mut session, 0, DRIFT);
        assert_eq!(boarding, Err(crate::board::BoardRefusal::CantBoard));
        assert_eq!(
            boarded_beeps(&mut session),
            0,
            "the refusal's snd 153 is the board refusal's, not this line's"
        );
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

    /// The airlocks among the sounds raised since the last taken.
    fn airlocks(session: &mut Session) -> usize {
        session
            .take_sounds()
            .into_iter()
            .filter(|sound| *sound == crate::sound::SimSound::Boarded)
            .count()
    }

    /// A mission's special ship skips the plunder and its airlock
    /// (@0x657b8-0x657e3 → @0x65f43), whether its mission stands or its
    /// slot is free; a failed mission's ship goes on to the plunder,
    /// after `_DoGoodSamaritan`, and plays it (@0x65eb3 → @0x65f00).
    #[test]
    fn only_a_failed_missions_ship_plays_the_airlock() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        let _ = session.take_sounds();
        let (boarding, _) = board_first(&mut session);
        assert!(
            matches!(boarding, Ok(crate::board::Boarding::MissionShip(_))),
            "{boarding:?}"
        );
        assert_eq!(airlocks(&mut session), 0, "the mission stands");

        let mut session = in_flight_with(&catalog, 1);
        if let Some(npc) = session.traffic.npcs_mut().first_mut() {
            npc.mission = Some(crate::traffic::npc::MissionMark {
                order: None,
                slot: 5,
                behav: ShipBehav::Own,
                formation: None,
            });
        }
        let _ = session.take_sounds();
        let _ = board_first(&mut session);
        assert_eq!(airlocks(&mut session), 0, "a free slot");

        let mut session = in_flight_with(&catalog, 1);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.failed = Some(FailedBy::Marked);
        }
        let _ = session.take_sounds();
        let (boarding, _) = board_first(&mut session);
        assert!(
            matches!(boarding, Ok(crate::board::Boarding::Opened(_))),
            "{boarding:?}"
        );
        assert_eq!(airlocks(&mut session), 1, "the mission failed");
    }

    /// [`in_flight`] for a mission of `count` ships.
    pub(in crate::session) fn in_flight_with(catalog: &FakePilotCatalog, count: i16) -> Session {
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

    /// [`goal_catalog`]'s rescue mission 140, of `CanAbort` `can_abort`,
    /// accepted with `days_left` and failed or not, then taken off from:
    /// the system set up.
    fn timed_rescue(can_abort: i16, days_left: u32, failed: bool) -> Session {
        let mut catalog = goal_catalog(5, 0);
        catalog.missions[0].can_abort = can_abort;
        catalog.missions[0].time_limit = 10;
        let mut session = accepted(&catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.days_left = Some(days_left);
            active.failed = failed.then_some(FailedBy::Quick);
        }
        take_off_unaborted(&mut session, &catalog);
        session
    }

    fn boarded(session: &Session) -> Vec<bool> {
        session.npcs().iter().map(|npc| npc.boarded).collect()
    }

    #[test]
    fn a_timed_rescue_mission_out_of_time_sets_up_its_ships_boarded_and_unheld() {
        let mut session = timed_rescue(0, 0, true);
        assert_eq!(marked(&session), [0, 0]);
        assert_eq!(boarded(&session), [true; 2]);
        assert_eq!(conditions(&session), [Condition::Disabled; 2], "cut armour");
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| !session.mission_ship_held(npc))
        );
        assert!(board_first(&mut session).0.is_err(), "boarded already");
        full_armour(&mut session);
        fight(&mut session, 1);
        assert_eq!(conditions(&session), [Condition::Intact; 2], "not held");
    }

    #[test]
    fn a_timed_rescue_mission_with_days_left_sets_up_its_ships_held() {
        let mut session = timed_rescue(0, 5, false);
        assert_eq!(boarded(&session), [false; 2]);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| session.mission_ship_held(npc))
        );
        full_armour(&mut session);
        fight(&mut session, 30);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
    }

    #[test]
    fn an_abortable_rescue_mission_expiring_as_it_takes_off_sets_up_its_ships_then_fails() {
        // The original sets the system up before its objectives check
        // fails the mission, so the ships come in boarded.
        let session = timed_rescue(1, 0, false);
        assert_eq!(boarded(&session), [true; 2]);
        assert!(session.pilot().missions().get(0).is_none(), "failed, ended");
    }

    #[test]
    fn an_abortable_rescue_mission_expiring_on_its_jump_sets_up_its_ships_then_fails() {
        let mut catalog = goal_catalog(5, 0);
        catalog.missions[0].ships.syst = 131;
        catalog.missions[0].time_limit = 10;
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), Vec::<u8>::new());
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.days_left = Some(1);
        }
        jump_to(&mut session, &catalog, 131);
        assert_eq!(boarded(&session), [true; 2]);
        assert!(session.pilot().missions().get(0).is_none(), "failed, ended");
    }

    #[test]
    #[allow(clippy::float_cmp)] // whole numbers
    fn a_rescue_ship_on_a_hull_with_default_armour_starts_at_its_base_armours_cut() {
        let mut catalog = goal_catalog(5, 0);
        catalog.outfits.push(crate::testkit::outfit(
            300,
            &[(crate::stats::MORE_ARMOR, 500)],
        ));
        catalog.ship_records[1].defaults = vec![(crate::catalog::OutfitId(300), 1)];
        let session = in_flight(&catalog);
        assert_eq!(conditions(&session), [Condition::Disabled; 2]);
        for npc in session.npcs() {
            assert_eq!(npc.stats.armor, f32::from(FAST.armor) + 500.0);
            assert!(
                (npc.reserves.armor.now - f32::from(FAST.armor).mul_add(0.33, -1.0)).abs() < 1e-4,
                "{}",
                npc.reserves.armor.now
            );
        }
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
                crate::person::derelict_armor(f32::from(FAST.armor), npc.hull.tough),
                "from the ship type's base Armor"
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
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| matches!(npc.mode, Mode::Coasting { .. })),
            "the glide's wait counts down while it is held (@0x35026)"
        );
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
        // A new target, not Ace's id reused: the original marks the target
        // panel for redraw (`_statusRedraw[6]`, @0x62543), and ours redraws
        // it at once on the target's `NpcId` changing
        // (`nova_view::flight::cadence::retake_target`).
        assert_ne!(ship.id, NpcId(0), "a new target");
        assert!(!session.pilot().gone(PersonId(600)), "not gone for good");
    }

    #[test]
    fn by_the_engine_after_a_swap_osn_names_the_swapped_in_ships_slot() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let session = hailing_ace(&catalog);
        assert_eq!(session.plain_text("<OSN>"), "Ace", "hailed (@0x621de)");
        let session = swapped(&catalog);
        assert_eq!(
            session.plain_text("<OSN>"),
            "[Error]",
            "the target swapped in (@0x6252f, @0x625a4) holds no person (@0x3cb48)"
        );
        let mut kept = swapping(0);
        kept.persons[0].flags = 0;
        let session = swapped(&kept);
        assert!(person_here(&session), "no swap");
        assert_eq!(session.plain_text("<OSN>"), "Ace", "the hailed ship still");
    }

    /// [`hailing_ace`] with Ace moved to berth 2 before the hail, which
    /// remembers Ace, and the lower berth 1 left free: the ship swapped in
    /// takes berth 1, not the hailed ship's.
    fn hailing_ace_at_berth_2(catalog: &FakePilotCatalog) -> Session {
        let pilot = crate::pilot::Pilot::new(catalog, "Stock").expect("a pilot");
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_strings(Rc::new(Names));
        session.populate(catalog, &mut NeverFires);
        let ace = &mut session.traffic.npcs_mut()[0];
        ace.state = ACE_AT;
        ace.berth = 2;
        session.traffic.set_berth_person(2, "Ace");
        session.traffic.forget_berth_person(1);
        session.target = Some(NpcId(0));
        let hailed = session.open_hail(catalog, &HailOptions::default(), &mut NeverFires);
        assert!(matches!(hailed, Ok(Hailed::Offer(_))), "{hailed:?}");
        assert_eq!(session.plain_text("<OSN>"), "Ace", "hailed at berth 2");
        session
    }

    /// The berth of the one mission ship here after a swap.
    fn swapped_in_berth(session: &Session) -> u32 {
        let ships: Vec<_> = session
            .npcs()
            .iter()
            .filter(|npc| npc.person.is_none())
            .collect();
        assert_eq!(ships.len(), 1, "the swapped-in ship alone");
        ships[0].berth
    }

    #[test]
    fn by_the_engine_after_a_swap_osn_follows_the_swapped_in_ship_to_its_new_slot() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace_at_berth_2(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        let berth = swapped_in_berth(&session);
        assert_eq!(berth, 1, "the lowest free berth, not the hailed ship's");
        assert_eq!(
            session.offering_ship,
            Some(super::super::person_offers::OfferingShip { berth }),
            "the target swapped in (@0x6252f, @0x625a4)"
        );
        assert_eq!(
            session.plain_text("<OSN>"),
            "[Error]",
            "its slot holds no person, though Ace's old slot still names Ace"
        );
        assert_eq!(session.traffic.berth_person(2), Some("Ace"));
    }

    #[test]
    fn by_the_engine_a_swap_settled_after_the_target_moved_still_names_the_swapped_in_ships_slot() {
        // The original swaps as the hail's dialog closes, when the hailed
        // ship is always the player's target, and `_offeringShipIndex`
        // takes the ship swapped into its place (@0x6252f, @0x625a4). The
        // engine swaps at the settling after, so a target moved in
        // between is kept, and `<OSN>` still names the swapped-in ship.
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace_at_berth_2(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        session.target = None;
        settle(&mut session, &catalog);
        assert!(!person_here(&session), "swapped all the same");
        assert_eq!(session.target, None, "the player's target is kept");
        let berth = swapped_in_berth(&session);
        assert_eq!(
            session.offering_ship,
            Some(super::super::person_offers::OfferingShip { berth }),
        );
        assert_eq!(session.plain_text("<OSN>"), "[Error]");
    }

    /// [`swapped`], under `source` of [`RuleKey::LeaderlessSwapHold`].
    fn swapped_by(catalog: &FakePilotCatalog, source: crate::RuleSource) -> Session {
        swapped_under(
            catalog,
            crate::Rulebook::default().with_override(RuleKey::LeaderlessSwapHold, source),
        )
    }

    /// [`swapped`], under `rules`.
    fn swapped_under(catalog: &FakePilotCatalog, rules: crate::Rulebook) -> Session {
        let mut session = hailing_ace(catalog).with_rules(rules);
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, catalog);
        session
    }

    /// [`swapping`] a person of `Flags` 0x0040 for mission 140's ship of
    /// `ShipStart` `start`, swapped under `source` of
    /// [`RuleKey::SwapArrives`], then ticked past its appear timer as it
    /// stood after the swap: the timer and arrived count after the swap,
    /// and the slots of the mission ships here after the ticks.
    fn swap_then_timer(start: i16, source: crate::RuleSource) -> (i16, i16, Vec<u8>) {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.missions[0].ships.start = start;
        let mut session = swapped_under(
            &catalog,
            crate::Rulebook::default().with_override(RuleKey::SwapArrives, source),
        );
        assert_eq!(marked(&session), [0], "{start} {source:?}: the swap");
        let SpecialShips {
            appear, arrived, ..
        } = ships_of(&session);
        ticks(
            &mut session,
            &catalog,
            usize::try_from(appear.max(0)).expect("positive") + 1,
        );
        (appear, arrived, marked(&session))
    }

    #[test]
    fn by_the_engine_the_timer_brings_a_second_ship_beside_a_swapped_one() {
        for (start, arrived) in [(1, 0), (2, 1)] {
            let (appear, now_arrived, marked) = swap_then_timer(start, crate::RuleSource::Engine);
            assert!(appear > 0, "{start}: the timer runs on");
            assert_eq!(now_arrived, arrived, "{start}: no count changes");
            assert_eq!(marked, [0, 0], "{start}");
        }
    }

    #[test]
    fn by_the_other_reading_no_ship_arrives_beside_a_swapped_one_on_the_timer() {
        for start in [1, 2] {
            let (appear, arrived, marked) = swap_then_timer(start, crate::RuleSource::Bible);
            assert_eq!(appear, crate::mission::ships::NOT_APPEARING, "{start}");
            assert_eq!(arrived, 1, "{start}: the swap is the arrival");
            assert_eq!(marked, [0], "{start}");
        }
    }

    #[test]
    fn a_leaderless_shipgoal_3_swap_holds_where_the_person_was() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = swapped_by(&catalog, crate::RuleSource::Engine);
        let id = session.npcs()[0].id;
        assert_eq!(session.npcs()[0].goal, Goal::Hold);
        let ticks = 120;
        for _ in 0..ticks {
            session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        }
        let ship = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("still here, not landed");
        assert_eq!(ship.goal, Goal::Hold);
        assert_eq!(ship.state.velocity, ACE_AT.velocity, "coasting");
        let drift = ACE_AT.position + ACE_AT.velocity * ticks as f32;
        assert!(
            (ship.state.position - drift).length() < 1e-2,
            "on Ace's velocity: {:?}",
            ship.state.position
        );
    }

    #[test]
    fn by_the_other_reading_a_leaderless_swap_decides_as_its_dudes_ai() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = swapped_by(&catalog, crate::RuleSource::Bible);
        let id = session.npcs()[0].id;
        assert_eq!(session.npcs()[0].goal, Goal::Idle);
        for _ in 0..120 {
            session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        }
        let goal = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .map(|npc| npc.goal);
        assert!(
            matches!(goal, None | Some(Goal::Land(_) | Goal::JumpOut)),
            "{goal:?}"
        );
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
    // One ship type (`Flags` 0x0800).

    /// Draws as [`NeverFires`] does, but below 37, the weights of
    /// [`two_types`]' düde, it gives 0, then 36, in turn: a type drawn
    /// for each ship differs from the last.
    #[derive(Default)]
    struct Alternating {
        k: u32,
    }

    impl Chance for Alternating {
        fn fires(&mut self, percent: u8) -> bool {
            NeverFires.fires(percent)
        }

        fn below(&mut self, n: u32) -> u32 {
            if n != 37 {
                return NeverFires.below(n);
            }
            self.k += 1;
            if self.k % 2 == 1 { 0 } else { 36 }
        }

        fn roll(&mut self, sides: u16) -> u16 {
            NeverFires.roll(sides)
        }
    }

    /// [`ships_catalog`] with mission 140 of `Flags` `flags` and three
    /// ships starting `start`, of düde 150 flying ship 200 at 20 and ship
    /// 129 at 17.
    fn two_types(flags: u16, start: i16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(MissionShip {
            count: 3,
            ..shipping(-1, start)
        });
        catalog.ship_records.push(ship(129, FAST));
        catalog.dudes[0].1.ships = vec![(ShipId(200), 20), (ShipId(129), 17)];
        catalog.missions[0].flags = flags;
        catalog
    }

    /// A session of `catalog`, docked at 128 with mission 140 accepted
    /// into slot 0, drawing on `chance`.
    fn accepted_drawing(catalog: &FakePilotCatalog, chance: &mut dyn Chance) -> Session {
        let mut session = Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Names));
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        assert_eq!(session.accept_mission(MissionId(140), chance), Ok(0));
        settle(&mut session, catalog);
        session
    }

    /// The ship types of the mission ships in the system, in order.
    fn marked_types(session: &Session) -> Vec<ShipId> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some())
            .map(|npc| npc.ship)
            .collect()
    }

    #[test]
    fn a_0x0800_mission_sets_up_all_its_ships_of_the_type_drawn_at_accept() {
        let catalog = two_types(0x0800, 0);
        let mut chance = Alternating::default();
        let mut session = accepted_drawing(&catalog, &mut chance);
        assert_eq!(ships_of(&session).uniform_type, Some(ShipId(200)));
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(marked_types(&session), [ShipId(200); 3]);
    }

    #[test]
    fn a_mission_without_0x0800_draws_a_type_per_ship() {
        let catalog = two_types(0, 0);
        let mut chance = Alternating::default();
        let mut session = accepted_drawing(&catalog, &mut chance);
        assert_eq!(ships_of(&session).uniform_type, None);
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert_eq!(
            marked_types(&session),
            [ShipId(200), ShipId(129), ShipId(200)]
        );
    }

    #[test]
    fn a_0x0800_missions_later_arrivals_fly_the_accept_type() {
        let catalog = two_types(0x0800, 1);
        let mut chance = Alternating::default();
        let mut session = accepted_drawing(&catalog, &mut chance);
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert!(marked(&session).is_empty(), "none at setup");
        for _ in 0..199 {
            session.tick_traffic(&catalog, &NovaAi::default(), &mut chance);
        }
        assert_eq!(marked_types(&session), [ShipId(200); 3]);
    }

    #[test]
    fn the_accept_draw_reads_appear_on_before_on_accept() {
        let mut catalog = two_types(0x0800, 0);
        catalog.ship_records[1].appear_on = Test::parse("b5");
        catalog.missions[0].on_accept = Script::parse("b5");
        let mut chance = Alternating::default();
        let session = accepted_drawing(&catalog, &mut chance);
        let bit = crate::control::Bit::new(5).expect("a bit");
        assert!(session.pilot().control_bits().get(bit), "OnAccept ran");
        assert_eq!(
            ships_of(&session).uniform_type,
            Some(ShipId(129)),
            "200 could not appear yet"
        );
    }

    #[test]
    fn a_0x0800_slot_without_a_type_draws_one_at_its_next_spawn_and_keeps_it() {
        let catalog = two_types(0x0800, 0);
        let mut chance = Alternating::default();
        let mut session = accepted_drawing(&catalog, &mut chance);
        forget_uniform_type(&mut session);
        session.take_off(&catalog, &mut chance).expect("takes off");
        let types = marked_types(&session);
        assert_eq!(types.len(), 3);
        assert!(types.iter().all(|ship| *ship == types[0]), "{types:?}");
        assert_eq!(ships_of(&session).uniform_type, Some(types[0]), "kept");
    }

    /// Clears slot 0's kept type, as a save from before it was kept
    /// loads it.
    fn forget_uniform_type(session: &mut Session) {
        session
            .pilot
            .missions
            .get_mut(0)
            .expect("held")
            .ships
            .uniform_type = None;
    }

    #[test]
    fn a_0x0800_slot_without_a_type_draws_one_for_its_arrivals_and_keeps_it() {
        let catalog = two_types(0x0800, 1);
        let mut chance = Alternating::default();
        let mut session = accepted_drawing(&catalog, &mut chance);
        session.take_off(&catalog, &mut chance).expect("takes off");
        assert!(marked(&session).is_empty(), "none at setup");
        forget_uniform_type(&mut session);
        for _ in 0..199 {
            session.tick_traffic(&catalog, &NovaAi::default(), &mut chance);
        }
        let types = marked_types(&session);
        assert_eq!(types.len(), 3);
        assert!(types.iter().all(|ship| *ship == types[0]), "{types:?}");
        assert_eq!(ships_of(&session).uniform_type, Some(types[0]), "kept");
    }

    #[test]
    fn after_a_0x0800_swap_a_later_setup_spawns_the_persons_type() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = swapped(&catalog);
        assert_eq!(session.npcs()[0].ship, ShipId(129), "Ace's type");
        assert_eq!(ships_of(&session).uniform_type, Some(ShipId(129)));
        // Drawn afresh, the düde would give 200.
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(marked_types(&session), [ShipId(129)]);
    }

    /// Every draw its first outcome.
    struct First;

    impl Chance for First {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, _n: u32) -> u32 {
            0
        }
    }

    /// [`hailing_ace`] for a [`swapping`] mission whose düde flies ship
    /// 200 or ship 128, but not Ace's 129, with mission 140 accepted on
    /// [`First`].
    fn accepted_lacking_aces_type() -> (FakePilotCatalog, Session) {
        let mut catalog = swapping(REPLACED_BY_MISSION_SHIP);
        catalog.dudes[0].1.ships = vec![(ShipId(200), 50), (ShipId(128), 50)];
        let mut session = hailing_ace(&catalog);
        assert_eq!(session.accept_mission(MissionId(140), &mut First), Ok(0));
        (catalog, session)
    }

    #[test]
    fn a_0x0800_swap_whose_dude_lacks_the_persons_type_flies_the_accept_type() {
        let (catalog, mut session) = accepted_lacking_aces_type();
        assert_eq!(ships_of(&session).uniform_type, Some(ShipId(200)));
        // The swap, drawing afresh, would give 128.
        settle(&mut session, &catalog);
        assert_eq!(marked_types(&session), [ShipId(200)]);
        assert_eq!(session.npcs()[0].state, ACE_AT, "swapped");
    }

    #[test]
    fn a_0x0800_swap_without_a_kept_type_draws_one_from_the_dude_and_keeps_it() {
        /// Draws as [`NeverFires`] does, but its first draw below 100,
        /// the weights of the düde, gives 0: ship 200 first, 128 after.
        #[derive(Default)]
        struct FirstOnce {
            drawn: bool,
        }
        impl Chance for FirstOnce {
            fn fires(&mut self, percent: u8) -> bool {
                NeverFires.fires(percent)
            }
            fn below(&mut self, n: u32) -> u32 {
                if n != 100 || std::mem::replace(&mut self.drawn, true) {
                    return NeverFires.below(n);
                }
                0
            }
            fn roll(&mut self, sides: u16) -> u16 {
                NeverFires.roll(sides)
            }
        }
        let (catalog, mut session) = accepted_lacking_aces_type();
        forget_uniform_type(&mut session);
        let _ = session.settle_script(&catalog, &mut FirstOnce::default());
        assert_eq!(session.npcs()[0].state, ACE_AT, "swapped");
        assert_eq!(ships_of(&session).uniform_type, Some(ShipId(200)), "kept");
        // A ship drawing its own type after the slot's would give 128.
        assert_eq!(marked_types(&session), [ShipId(200)]);
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
                session
                    .refuse_mission(MissionId(140), &mut NeverFires)
                    .map(|_| ()),
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
    fn the_swap_settles_the_brackets_even_with_the_ship_on_screen() {
        // @0x62538 writes 0 with no on-screen test, unlike the pick tail's
        // 256 for a target on screen (@0x6a5c9).
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        session.target_fly_in.start();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        let ship = session.target().expect("targeted").clone();
        assert!(
            session.npc_in_play_area(&ship, crate::brackets::sprite_in_area),
            "on screen"
        );
        assert_eq!(session.target_brackets().expect("bracketed").expand, 0.0);
        session.tick(crate::Controls::default());
        assert_eq!(session.target_brackets().expect("bracketed").expand, 0.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_hail_that_swaps_nothing_leaves_the_fly_in_running() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        session.target_fly_in.start();
        assert_eq!(
            session
                .refuse_mission(MissionId(140), &mut NeverFires)
                .map(|_| ()),
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
        let before = SpecialShips {
            // Only Ace's type, kept under 0x0800.
            uniform_type: Some(ShipId(129)),
            // And the escort goal's moment, met as it arrives.
            ship_done: true,
            ..ships_of(&session)
        };
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
    fn a_swap_with_no_slot_leaves_the_person_targeted_and_the_mission_as_it_was() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let mut session = hailing_ace(&catalog);
        crowd(&mut session, 55);
        let npcs = session.npcs().len();
        assert_eq!(
            session.accept_mission(MissionId(140), &mut NeverFires),
            Ok(0)
        );
        let before = ships_of(&session);
        settle(&mut session, &catalog);
        assert!(person_here(&session), "Ace stays (@0x62481 → @0x62593)");
        assert_eq!(session.npcs().len(), npcs);
        assert!(marked(&session).is_empty(), "no ship came");
        assert_eq!(session.target, Some(NpcId(0)), "Ace still targeted");
        assert_eq!(ships_of(&session), before, "no count changes");
        assert_eq!(before.left, 1);
        let active = session.pilot().missions().get(0).expect("held");
        assert!(!active.goals_met, "no arrival raised");
    }

    #[test]
    fn the_swapped_ship_takes_the_first_berth_free_while_the_person_holds_its_own() {
        let catalog = swapping(REPLACED_BY_MISSION_SHIP);
        let session = hailing_ace(&catalog);
        assert_eq!(session.npcs()[0].berth, 1, "Ace at berth 1");
        let session = swapped(&catalog);
        let ship = session
            .npcs()
            .iter()
            .find(|npc| npc.mission.is_some())
            .expect("swapped in");
        assert_eq!(ship.berth, 2, "taken while Ace held 1 (@0x62520)");
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
