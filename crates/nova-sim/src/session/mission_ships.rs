//! The session's side of mission ships (see the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships"):
//! their state drawn as a mission is accepted, their setup with the
//! system's ships, the enter-system hook that restarts their appear
//! timer, the timer and the ships it brings in, their goals (see the
//! [`story`](crate::story) module's "Ship goals"), and what becomes of
//! them as they are destroyed, disabled or boarded and as their mission
//! ends.
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
//! restart. Its timer is drawn after the setup's draws, not before.
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
//! The moment the goals are met shows `ShipDoneText`, runs `OnShipDone`
//! and auto-aborts a mission of `Flags` 0x0001 ([`Session::auto_abort`]).
//! Escorted ships count while they are in the system: both spawn paths
//! raise [`Fate::Arrived`] for each ship they bring in, and watch the
//! ships of an unmet observe goal for their first sighting
//! ([`Session::watch_ship`]).
//!
//! **The counts** go up where the original counts them: a ship destroyed
//! as it is taken out of the fight, one disabled as the fight's tick
//! downs it, one boarded on the assist path of [`Session::board`]. The
//! first loss that fails its mission fails it there, at once
//! ([`SimMessage::MissionShipLost`] and the quick failure); the goal
//! checks follow as the `ShipFate` the loss raised settles.

use super::Session;
use crate::board::{Boarding, MissionBoardLine};
use crate::catalog::{DudeId, MissionId, MissionRecord, TrafficCatalog};
use crate::chance::Chance;
use crate::control::Script;
use crate::geometry::Vec2;
use crate::message::SimMessage;
use crate::mission::active::AUTO_ABORT;
use crate::mission::ships::{ShipBehav, ShipFacts, ShipStart, appear_timer};
use crate::mission::{
    ActiveMission, AssistLine, FailedBy, GoalFacts, LossFacts, ShipGoal, ShipSystem, SpecialShips,
    assists, check_goals, fails_on_destroyed, fails_on_disabled, goals_met_moment,
};
use crate::session::mission_screens::MissionTextKind;
use crate::story::{
    Consumer, Entrance, EventFilter, Fate, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::traffic::npc::{Npc, NpcId};
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
/// docked accept or a landing.
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
        let facts = ShipFacts {
            player_system: self.pilot.system,
            destination: active.destination,
            return_to: active.return_to,
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

    /// Whether the enter-system hook restarts the appear timer of the
    /// mission in `slot`, `active` of `record`: `ShipStart` 1, ships
    /// left, and the player in their system (@0x99f5b-0x99f8a).
    fn hook_restarts(&self, record: &MissionRecord, active: &ActiveMission) -> bool {
        ShipStart::from_code(record.ships.start) == ShipStart::ComesLater
            && active.ships.left > 0
            && self.ships_here(active.ships.system)
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
        if !restarts_on(how) || !self.hook_restarts(&record, active) {
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
            here: seen.here || self.ships_here(ships.system),
            escorted: self
                .npcs()
                .iter()
                .any(|npc| npc.mission.is_some_and(|mark| mark.slot == slot)),
            sighted: seen.sighted,
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
        if goals_met_moment(facts.met, check, failed_before) {
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
        if record.flags & AUTO_ABORT != 0 && self.pilot.missions.holds(slot, record.id) {
            self.auto_abort(slot, record, chance);
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
    /// by an entry its enter-system hook runs on, a slot the hook is
    /// about to restart brings none (see the module docs).
    pub(super) fn setup_mission_ships(
        &self,
        table: &SpawnTable,
        entering: bool,
        chance: &mut (impl Chance + ?Sized),
    ) -> Vec<NewShip> {
        let nav = self.nav_slots();
        let mut ships = Vec::new();
        for (slot, active) in self.pilot.missions.iter() {
            let record = self.lifecycle_record(active.mission);
            let Some(dude) = record.ships.dude else {
                continue;
            };
            let state = &active.ships;
            if !self.ships_here(state.system)
                || state.left <= 0
                || state.appear >= 0
                || (entering && self.hook_restarts(&record, active))
            {
                continue;
            }
            let spawn = MissionSpawn {
                slot,
                dude,
                count: state.left,
                start: ShipStart::from_code(record.ships.start),
                behav: ShipBehav::from_code(record.ships.behav),
                nav: &nav,
            };
            ships.extend(spawn::mission_ships(table, &spawn, self.gate(), chance));
        }
        ships
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
    /// each counted arrived. (A mission of `Flags` 0x0001 then
    /// auto-aborts: rdm:phase/mission-ships/phase-10-auto-abort-at-spawn.)
    pub(super) fn mission_ship_arrivals(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            let Some(active) = self.pilot.missions.get(slot) else {
                continue;
            };
            let record = self.lifecycle_record(active.mission);
            let state = active.ships.clone();
            if !self.ships_here(state.system) {
                continue;
            }
            let coming = if ShipStart::from_code(record.ships.start) == ShipStart::ComesLater {
                state.left - state.arrived
            } else {
                state.left
            };
            if coming <= 0 {
                continue;
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
                continue;
            }
            let count = self.bring_in_mission_ships(catalog, slot, &record, coming, chance);
            if let Some(active) = self.pilot.missions.get_mut(slot) {
                active.ships.arrived = active.ships.arrived.saturating_add(count);
            }
        }
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

    /// Leaves the special ships of the mission in `slot` as meeting its
    /// goal in flight would, then checks its goals as the subscriptions
    /// do, so the moment's `ShipDoneText`, `OnShipDone` and auto-abort
    /// run, drawing on `chance`: all of them arrived; for a destroy goal,
    /// all destroyed and none left; for a disable goal, all disabled; for
    /// a board or rescue goal, all boarded; for an observe goal, one seen
    /// with them here; for an escort goal, all of them brought into the
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
    /// (`_HandlePlayerBoardAttempt` @0x652a2-0x657e3), the player's
    /// velocity matched already: on the assist path ([`assists`]) its
    /// slot counts it boarded, and the comm line is said; then it is
    /// marked boarded (@0x65f43), with no `BoardPenalty`, person's offer
    /// or plunder. A ship of a failed mission is boarded as ordinary
    /// traffic: none. The assist path's ship +0x4c (the AI's wait
    /// countdown, 100 @0x65746) is not kept.
    pub(super) fn board_mission_ship(&mut self, npc: &Npc) -> Option<Boarding> {
        let mark = npc.mission?;
        let mut line = None;
        if let Some(active) = self.pilot.missions.get(mark.slot) {
            if active.failed.is_some() {
                return None;
            }
            let record = self.lifecycle_record(active.mission);
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
        Some(Boarding::MissionShip(line))
    }

    /// The special ships just brought into the system, each numbered
    /// above `last` (every one with none): each raises [`Fate::Arrived`],
    /// and each of a mission whose goal is to observe them, not yet met,
    /// is watched for its first sighting ([`Session::watch_ship`]).
    pub(super) fn mission_ships_came(&mut self, last: Option<NpcId>) {
        let came: Vec<(NpcId, u8)> = self
            .npcs()
            .iter()
            .filter(|npc| last.is_none_or(|last| npc.id > last))
            .filter_map(|npc| Some((npc.id, npc.mission?.slot)))
            .collect();
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
    /// on as ordinary traffic, or, while the player is docked, is taken
    /// out of the system.
    pub(super) fn release_mission_ships(&mut self, slot: u8) {
        let marked: Vec<NpcId> = self
            .npcs()
            .iter()
            .filter(|npc| npc.mission.is_some_and(|mark| mark.slot == slot))
            .map(|npc| npc.id)
            .collect();
        for id in marked {
            if self.landed.is_some() {
                self.traffic.remove(id);
            } else if let Some(npc) = self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id) {
                npc.mission = None;
            }
        }
    }

    /// Clears the mark of every special ship whose slot is free, as
    /// `_AIDispatch` does on each think (@0x8fbcd-0x8fc3a): it flies on as
    /// ordinary traffic. A mission still in its slot keeps its ships
    /// marked, failed or not.
    pub(super) fn unmark_inactive_mission_ships(&mut self) {
        let missions = &self.pilot.missions;
        for npc in self.traffic.npcs_mut() {
            if npc
                .mission
                .is_some_and(|mark| missions.get(mark.slot).is_none())
            {
                npc.mission = None;
            }
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
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::Goal;
    use crate::ai::NovaAi;
    use crate::board::MissionBoardLine;
    use crate::catalog::{
        CommCatalog, DudeRecord, GovtId, MissionId, MissionShip, PersonId, PersonRecord, ShipId,
        StellarId, SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::control::SetExpr;
    use crate::message::SimMessage;
    use crate::mission::FailedBy;
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
    fn shipping(syst: i16, start: i16) -> MissionShip {
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
    fn ships_catalog(ships: MissionShip) -> FakePilotCatalog {
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

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// A session of `catalog`, named from [`Names`], docked at 128 with
    /// mission 140 accepted into slot 0.
    fn accepted(catalog: &FakePilotCatalog) -> Session {
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

    /// Takes off, settles, and runs the first traffic tick, which sets
    /// the system up.
    fn take_off(session: &mut Session, catalog: &FakePilotCatalog) {
        session.take_off(&mut NeverFires).expect("takes off");
        settle(session, catalog);
        tick(session, catalog);
    }

    /// Jumps from the centre, where `fly_out` heads straight out, to `to`,
    /// and settles.
    fn jump_to(session: &mut Session, catalog: &FakePilotCatalog, to: i16) {
        session.player.position = Vec2::ZERO;
        assert_eq!(jump(session, catalog, to), Some(SystemId(to)));
        settle(session, catalog);
    }

    fn tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
    }

    /// The slots of the mission ships in the system, in order.
    fn marked(session: &Session) -> Vec<u8> {
        session
            .npcs()
            .iter()
            .filter_map(|npc| Some(npc.mission?.slot))
            .collect()
    }

    fn ships_of(session: &Session) -> SpecialShips {
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
            }
        );
        assert_eq!(
            session.mission_text(0, "<SN> waits."),
            "Rover waits.",
            "<SN> reads the name drawn"
        );
    }

    #[test]
    fn the_ships_are_drawn_before_on_accept_runs() {
        // OnAccept moves the player to 131: the ships' -1 is still 130.
        let mut catalog = ships_catalog(shipping(-1, 0));
        catalog.missions[0].on_accept = crate::control::Script::parse("M131");
        let session = accepted(&catalog);
        assert_eq!(ships_of(&session).system, ShipSystem::At(SystemId(130)));
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
    fn ticks(session: &mut Session, catalog: &FakePilotCatalog, n: usize) {
        for _ in 0..n {
            tick(session, catalog);
        }
    }

    #[test]
    fn ships_starting_1_set_up_none_and_come_in_by_hyperspace_when_their_timer_runs_out() {
        let catalog = ships_catalog(shipping(-1, 1));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "none at setup");
        assert_eq!(
            ships_of(&session).appear,
            199,
            "the setup tick counts nothing"
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
                .all(|npc| npc.mission.is_some_and(|mark| !mark.cloaked))
        );
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert!(
            session
                .npcs()
                .iter()
                .all(|npc| npc.mission.is_some_and(|mark| mark.cloaked))
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
            slot: 5,
            behav: ShipBehav::Own,
            cloaked: false,
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
    fn an_auto_aborted_missions_ships_stay_and_lose_their_mark_on_the_next_tick() {
        // An auto-abort is reached today only at accept, with no ships;
        // rdm:phase/mission-ships/phase-10-auto-abort-at-spawn reaches it
        // once they are in.
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let record = catalog.missions[0].clone();
        session.auto_abort(0, &record, &mut NeverFires);
        assert!(session.pilot().missions().get(0).is_none(), "aborted");
        assert_eq!(marked(&session), [0, 0], "until the AI next thinks");
        tick(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 2, "they stay");
        assert!(marked(&session).is_empty());
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
    fn goal_catalog(goal: i16, flags: u16) -> FakePilotCatalog {
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
            .map(|npc| (npc.id, npc.condition, npc.govt))
            .collect();
        session.traffic.npcs_mut()[index].condition = Condition::Disabled;
        session.raise_disabled(&was, &mut NeverFires);
    }

    /// A session of `catalog` with mission 140 accepted and its ships in
    /// the system around the player.
    fn in_flight(catalog: &FakePilotCatalog) -> Session {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        assert_eq!(marked(&session), [0, 0]);
        session
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
            assert_eq!(bits(&session), [5], "{goal}: OnFailure ran");
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
        destroy(&mut session, 0);
        assert_eq!(failed(&session), Some(FailedBy::Quick));
        assert_eq!(bits(&session), [5]);
        assert_eq!(session.take_messages(), [SimMessage::MissionShipLost]);
        destroy(&mut session, 0);
        assert_eq!(session.take_messages(), [], "fails once");
        assert_eq!(bits(&session), [5], "OnFailure ran once");
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
        assert_eq!(bits(&session), Vec::<u16>::new());
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
        assert_eq!(bits(&session), [5]);
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
            slot: 5,
            behav: ShipBehav::Own,
            cloaked: false,
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
                    slot: 5,
                    behav: ShipBehav::Own,
                    cloaked: false,
                });
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
                slot: 5,
                behav: ShipBehav::Own,
                cloaked: false,
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
    fn an_auto_abort_mission_of_no_ship_count_accepted_docked_aborts_as_it_takes_off() {
        let none = MissionShip {
            count: -1,
            ..MissionShip::NONE
        };
        let catalog = shipless(none, 0x0001);
        let mut session = accepted(&catalog);
        assert!(session.pilot().missions().get(0).is_some(), "not at accept");
        session.take_off(&mut NeverFires).expect("takes off");
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(bits(&session), [7], "OnAbort ran");
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

    #[test]
    fn a_mission_of_no_goal_with_ships_and_flag_0x0001_is_left_unmet() {
        let catalog = shipless(shipping(-1, 0), 0x0001);
        let mut session = in_flight(&catalog);
        settle(&mut session, &catalog);
        assert!(!goals_met(&session));
        assert!(session.pilot().missions().get(0).is_some(), "not aborted");
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
        session.player = crate::flight::ShipState::default();
        let npc = &mut session.traffic.npcs_mut()[0];
        npc.state = crate::flight::ShipState::default();
        npc.condition = Condition::Disabled;
        session.target = Some(session.npcs()[0].id);
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
        for (goal, count, flags, pickup) in [
            (5, 2, 0x0001, -1),
            (2, 1, 0, -1),
            (2, 1, 0x0001, 2),
            (0, 1, 0x0001, -1),
        ] {
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

    #[test]
    fn a_failed_missions_ship_is_boarded_as_ordinary_traffic() {
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
        assert_eq!(crimes, [crate::legal::Crime::Board]);
        assert_eq!(ships_of(&session).boarded, 0);
    }

    #[test]
    fn a_ship_whose_slot_is_free_is_boarded_without_plunder() {
        let catalog = rescuing(5, 1, 0x0001);
        let mut session = in_flight_with(&catalog, 1);
        if let Some(npc) = session.traffic.npcs_mut().first_mut() {
            npc.mission = Some(crate::traffic::npc::MissionMark {
                slot: 5,
                behav: ShipBehav::Own,
                cloaked: false,
            });
        }
        let (boarding, crimes) = board_first(&mut session);
        assert_eq!(boarding, Ok(crate::board::Boarding::MissionShip(None)));
        assert_eq!(crimes, []);
    }

    /// [`in_flight`] for a mission of `count` ships.
    fn in_flight_with(catalog: &FakePilotCatalog, count: i16) -> Session {
        let mut session = accepted(catalog);
        take_off(&mut session, catalog);
        assert_eq!(
            marked(&session).len(),
            usize::try_from(count).expect("count")
        );
        settle(&mut session, catalog);
        session
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
                goal: 6,
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
}
