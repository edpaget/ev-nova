//! The session's side of auxiliary ships (see the
//! [`mission::aux_ships`](crate::mission::aux_ships) module and the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Auxiliary
//! ships"): their countdown and the ships it brings in, its restarts,
//! the gate exit that loses them, and the survivors given back as the
//! system's ships are dropped.
//!
//! **In flight**, each traffic tick runs `_EnterMoreShips`' frame
//! ([`Session::mission_ship_arrivals`]): for each active slot in order,
//! the auxiliary part first ([`Session::aux_arrivals`]), then the special
//! ships' part, then the countdown counts down one
//! ([`AuxShips::tick`]), wherever the player is. On the gate exit's
//! arrivals frame ([`Session::gate_exit_pass`], under the engine's
//! reading of [`RuleKey::GateFollow`](crate::rulebook::RuleKey::GateFollow)),
//! whose enter-system hook is still to run, every slot's
//! auxiliary part is left to the hook, which restarts each countdown:
//! the original's hook comes first, so its countdown, just restarted,
//! brings none (its one tick there is lost here, the restart coming
//! after).
//!
//! **The auto-abort**: a mission of `Flags` 0x0001 auto-aborts once
//! after its auxiliary ships' spawn ran, whether or not a ship came
//! (@0x4373a-0x43752), its ships staying. The original then runs the
//! special ships' part on the slot without testing again that it is
//! active (@0x43757); here that part reads the slot again and skips it
//! once freed. No stock mission of `Flags` 0x0001 has auxiliary ships.
//!
//! **The restarts** are the enter-system hook's (see the
//! `mission_ships` module) and the take-off's: each restarts every
//! active slot's auxiliary ships ([`AuxShips::restart`]), the hook after
//! that slot's special ships' restart, and [`Session::take_off`] in slot
//! order after the stay's days (@0x634df-0x6353c), before it arms the
//! escort-strength timer and sets the system up, so its draws come
//! before the take-off's setup's. On a hypergate's or
//! wormhole's exit a slot of `AuxShipSyst` -1 then loses its ships until
//! the next restart, as [`RuleKey::AuxGate`] reads.
//!
//! **The survivors**: as the system's ships are dropped (a setup, an
//! entry that takes only the fleet along, or a fresh traffic), each
//! auxiliary ship still in the system gives one back to its slot's ships
//! left to come ([`AuxShips::give_back`]), so without `Flags` 0x0010 the
//! next visit brings those still alive, and those destroyed never come
//! again. A ship that jumped out or landed is gone already, and gives
//! nothing back.
//!
//! [`AuxShips::tick`]: crate::mission::aux_ships::AuxShips::tick
//! [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
//! [`AuxShips::give_back`]: crate::mission::aux_ships::AuxShips::give_back

use std::collections::BTreeSet;

use super::Session;
use crate::catalog::TrafficCatalog;
use crate::chance::Chance;
use crate::mission::Stop;
use crate::mission::aux_ships::{aux_dude, aux_system_allows};
use crate::mission::ships::LOST_AT_GATE;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::Entrance;
use crate::traffic::npc::NpcId;
use crate::traffic::spawn::{self, AuxSpawn};

/// The `AuxShipSyst` a gate exit loses (-1 @0x672a5).
const ANY_SYSTEM: i16 = -1;

impl Session {
    /// The auxiliary part of `_EnterMoreShips`' frame for the mission in
    /// `slot` (@0x434be-0x43752), its `düde` read from `catalog`, drawing
    /// on `chance`: when some are coming ([`AuxShips::coming`]) and their
    /// `AuxShipSyst` allows the player's system ([`aux_system_allows`]),
    /// they come in by hyperspace ([`spawn::arriving_aux_ships`]), each
    /// counted ([`AuxShips::came`]); then a mission of `Flags` 0x0001
    /// auto-aborts. No save is due.
    ///
    /// [`AuxShips::coming`]: crate::mission::aux_ships::AuxShips::coming
    /// [`AuxShips::came`]: crate::mission::aux_ships::AuxShips::came
    pub(super) fn aux_arrivals(
        &mut self,
        catalog: &(impl TrafficCatalog + ?Sized),
        slot: u8,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        let coming = active.aux.coming(&record.aux);
        let holder = |stop: Option<Stop>| stop.and_then(|stop| self.holder_of(stop.stellar));
        let holders = (holder(active.destination), holder(active.return_to));
        let places = self.galaxy.map().places();
        if coming <= 0 || !aux_system_allows(record.aux.syst, self.pilot.system, holders, places) {
            return;
        }
        let Some(dude) = aux_dude(&record.aux) else {
            return;
        };
        self.traffic.table_mut().add_mission_dudes(
            catalog,
            &[dude],
            &self.ships,
            &self.outfits,
            &self.arsenal,
        );
        let spawn = AuxSpawn {
            slot,
            dude,
            count: coming,
        };
        let ships = spawn::arriving_aux_ships(self.traffic.table(), &spawn, self.gate(), chance);
        let came = ships.len();
        self.traffic.bring_in(ships);
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            for _ in 0..came {
                active.aux.came(record.flags);
            }
        }
        self.abort_on_spawn(slot, &record, &mut &mut *chance);
    }

    /// One frame of the auxiliary countdown of the mission `mission` in
    /// `slot`, while the slot holds it ([`AuxShips::tick`],
    /// @0x43ae8-0x43af8). No save is due.
    ///
    /// [`AuxShips::tick`]: crate::mission::aux_ships::AuxShips::tick
    pub(super) fn tick_aux(&mut self, slot: u8, mission: crate::catalog::MissionId) {
        if let Some(active) = self
            .pilot
            .missions
            .get_mut(slot)
            .filter(|active| active.mission == mission)
        {
            active.aux.tick();
        }
    }

    /// The restart of the auxiliary ships of the mission in `slot`, if
    /// any, by the enter-system hook or the take-off
    /// ([`AuxShips::restart`]), drawing on `chance`.
    ///
    /// [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
    pub(super) fn restart_aux(&mut self, slot: u8, chance: &mut dyn Chance) {
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        let record = self.lifecycle_record(active.mission);
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.aux.restart(&record.aux, record.flags, chance);
        }
    }

    /// The take-off's restart of every active mission's auxiliary ships,
    /// in slot order ([`Session::restart_aux`]), drawing on `chance`.
    pub(super) fn restart_every_aux(&mut self, chance: &mut dyn Chance) {
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            self.restart_aux(slot, chance);
        }
    }

    /// The player's exit from a hypergate or wormhole, after the
    /// enter-system hook's restart of the mission in `slot`: under the
    /// engine's reading of [`RuleKey::AuxGate`], an `AuxShipSyst` of -1
    /// loses its ships until the next restart, its countdown 0x7fff
    /// (@0x6729b-0x672ac). Nothing under the other reading.
    pub(super) fn lose_aux_at_gate(&mut self, slot: u8) {
        if self.rules.source_for(RuleKey::AuxGate) != RuleSource::Engine {
            return;
        }
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        if self.lifecycle_record(active.mission).aux.syst != ANY_SYSTEM {
            return;
        }
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.aux.countdown = LOST_AT_GATE;
        }
    }

    /// The system's ships dropped but those in `kept`: each other
    /// auxiliary ship of an active slot gives one back to it
    /// (`_DeleteAllComputerShips` @0x3bfce-0x3c028).
    pub(super) fn give_back_aux_ships(&mut self, kept: &BTreeSet<NpcId>) {
        let slots: Vec<u8> = self
            .npcs()
            .iter()
            .filter(|npc| !kept.contains(&npc.id))
            .filter_map(|npc| npc.aux)
            .collect();
        for slot in slots {
            let Some(active) = self.pilot.missions.get(slot) else {
                continue;
            };
            let record = self.lifecycle_record(active.mission);
            if let Some(active) = self.pilot.missions.get_mut(slot) {
                active.aux.give_back(&record.aux);
            }
        }
    }

    /// The enter-system hook's auxiliary part for the mission in `slot`
    /// on an entry `how` it runs on, drawing on `chance`: the restart,
    /// then on a gate's exit the loss of `AuxShipSyst` -1.
    pub(super) fn aux_on_entry(&mut self, slot: u8, how: Entrance, chance: &mut dyn Chance) {
        self.restart_aux(slot, chance);
        if how == Entrance::Gate {
            self.lose_aux_at_gate(slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, MissionId, MissionShip, ShipId, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::combat::hull::Condition;
    use crate::control::SetExpr;
    use crate::gate::GateKind;
    use crate::mission::active::AUTO_ABORT;
    use crate::mission::aux_ships::{AUX_EVERY_VISIT, AuxShips};
    use crate::rulebook::Rulebook;
    use crate::session::gate_follow::tests::{exit_gate, gated};
    use crate::session::mission_ships::tests::{
        accepted, aux_catalog, aux_ships, jump_to, marked, settle, shipping, take_off, tick, ticks,
        with_aux,
    };
    use crate::story::StoryEvent;
    use crate::testkit::{Draws, FakePilotCatalog, land_now};
    use crate::traffic::npc::Mode;

    /// Mission 140's auxiliary ships.
    fn aux_of(session: &Session) -> AuxShips {
        session.pilot().missions().get(0).expect("held").aux
    }

    /// Sets mission 140's auxiliary ships to `aux`.
    fn set_aux(session: &mut Session, aux: AuxShips) {
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.aux = aux;
        }
    }

    /// The auxiliary ships of slot 0 in the system.
    fn aux_here(session: &Session) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.aux == Some(0))
            .count()
    }

    /// Mission 140 with `count` auxiliary ships into `syst`, of `Flags`
    /// `flags`, and no special ships.
    fn aux_only(count: i16, syst: i16, flags: u16) -> FakePilotCatalog {
        aux_catalog(MissionShip::NONE, aux_ships(count, syst), flags)
    }

    /// Lands at 128 and takes off again, setting the system up.
    fn land_and_take_off(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(land_now(session), Ok(StellarId(128)));
        settle(session, catalog);
        take_off(session, catalog);
    }

    /// Destroys the NPC at `index`, as the fight does once it has broken
    /// up.
    fn destroy(session: &mut Session, index: usize) {
        session.traffic.npcs_mut()[index].condition = Condition::Destroyed;
        session.take_out_the_destroyed(&mut NeverFires);
    }

    // In flight.

    #[test]
    fn aux_ships_come_in_unmarked_by_hyperspace_once_their_countdown_runs_out() {
        let catalog = crate::testkit::far_129_reachable(aux_only(3, -1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(
            aux_of(&session).countdown,
            139,
            "restarted on take-off, its setup counting nothing"
        );
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_of(&session).countdown, 0);
        assert_eq!(aux_here(&session), 0, "they come the frame after");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 3, "all of them at once");
        for npc in session.npcs().iter().filter(|npc| npc.aux.is_some()) {
            assert_eq!(npc.mission, None, "ordinary traffic");
            assert_eq!((npc.ship, npc.govt), (ShipId(201), Some(GovtId(129))));
            assert!(matches!(npc.mode, Mode::JumpingIn { .. }));
        }
        assert!(marked(&session).is_empty());
        assert_eq!(
            aux_of(&session),
            AuxShips {
                countdown: 0,
                arrived: 3,
                left: 0
            }
        );
        ticks(&mut session, &catalog, 300);
        assert_eq!(aux_here(&session), 3, "no more come");
    }

    #[test]
    fn aux_ships_come_in_no_system_their_aux_ship_syst_refuses_but_count_down() {
        let catalog = aux_only(2, 131, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 300);
        assert_eq!(aux_here(&session), 0);
        assert_eq!(aux_of(&session).countdown, 0, "counted down anywhere");
        jump_to(&mut session, &catalog, 131);
        assert_eq!(aux_of(&session).countdown, 139, "restarted on arrival");
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 2);
    }

    /// [`aux_only`] with `count` auxiliary ships into `syst`, mission 140
    /// offered at 128, going to 140 (in 131) and returning to 128 (in
    /// 130).
    fn stopping(count: i16, syst: i16) -> FakePilotCatalog {
        let mut catalog = aux_only(count, syst, 0);
        catalog.missions[0].avail_stel = 128;
        catalog.missions[0].travel_stel = 140;
        catalog.missions[0].return_stel = 128;
        catalog
    }

    #[test]
    fn aux_ships_of_minus_2_come_in_at_the_destination_not_the_return_stop() {
        let catalog = stopping(2, -2);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(130), "the return stop's");
        ticks(&mut session, &catalog, 300);
        assert_eq!(aux_here(&session), 0, "not at the return stop");
        jump_to(&mut session, &catalog, 131);
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_here(&session), 0, "not before the countdown runs out");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 2, "at the destination");
    }

    #[test]
    fn aux_ships_of_minus_3_come_in_at_the_return_stop_not_the_destination() {
        let catalog = stopping(2, -3);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        ticks(&mut session, &catalog, 300);
        assert_eq!(aux_here(&session), 0, "not at the destination");
        jump_to(&mut session, &catalog, 130);
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_here(&session), 0, "not before the countdown runs out");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 2, "at the return stop");
    }

    #[test]
    fn aux_ships_of_minus_2_come_where_the_destination_is_held_now() {
        let mut catalog = stopping(2, -2);
        // 132 lists 140 as well as 131, which owns it at the offer.
        catalog
            .sites
            .push((SystemId(132), vec![crate::testkit::planet(140, 0.0, 0.0)]));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        jump_to(&mut session, &catalog, 131);
        jump_to(&mut session, &catalog, 132);
        ticks(&mut session, &catalog, 300);
        assert_eq!(aux_here(&session), 0, "131 holds 140");
        assert_eq!(aux_of(&session).countdown, 0, "they would come now");
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        }));
        tick(&mut session, &catalog);
        assert_eq!(
            aux_here(&session),
            2,
            "132 owns 140 now, read live (@0x99322-0x99336)"
        );
    }

    #[test]
    fn aux_ships_of_minus_2_never_come_for_a_destination_no_system_holds() {
        let catalog = stopping(2, -2);
        let mut session = accepted(&catalog);
        // As an offer resolved while the player's 130 held 140 would store it.
        if let Some(destination) = session
            .pilot
            .missions
            .get_mut(0)
            .and_then(|active| active.destination.as_mut())
        {
            destination.system = Some(SystemId(130));
        }
        take_off(&mut session, &catalog);
        assert!(session.change_galaxy(&catalog, |overlay| {
            overlay.set_visible(SystemId(131), Some(false));
        }));
        assert_eq!(session.system(), SystemId(130));
        ticks(&mut session, &catalog, 300);
        assert_eq!(
            aux_here(&session),
            0,
            "only the hidden 131 lists 140 and 130 does not: no owner, and no fallback (@0x99322-0x99336)"
        );
        assert_eq!(aux_of(&session).arrived, 0, "none came and went");
    }

    #[test]
    fn aux_ships_of_minus_3_come_at_a_return_rescued_into_the_players_system() {
        let mut catalog = stopping(2, -3);
        // 128, lower, shadows the player's 130 at its place, so 130 owns
        // nothing and holds its own 128 and 129 by the rescue.
        catalog
            .star_map
            .push(crate::testkit::star(128, (0.0, 0.0), &[]));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(130));
        assert_eq!(
            session
                .galaxy()
                .holder(session.listers(StellarId(128)), SystemId(130)),
            Some(SystemId(130)),
            "rescued"
        );
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_here(&session), 0, "not before the countdown runs out");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 2, "at the return stop's holder");
    }

    #[test]
    fn by_the_engine_aux_ships_of_a_visible_system_shadowed_at_its_place_come_in_at_the_lowest() {
        let mut catalog = aux_only(2, 134, 0);
        catalog
            .star_map
            .push(crate::testkit::star(134, (0.0, 0.0), &[]));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_here(&session), 0, "not before the countdown runs out");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 2, "130 stands for 134, though visible");
    }

    #[test]
    fn aux_ships_of_a_hidden_system_come_in_at_the_visible_one_standing_for_it() {
        let mut catalog = aux_only(2, 134, 0);
        catalog
            .star_map
            .push(crate::testkit::star(134, (0.0, 0.0), &[]));
        crate::testkit::hideable(&mut catalog);
        let mut session = accepted(&catalog).with_rules(
            Rulebook::default().with_override(RuleKey::ColocatedResolution, RuleSource::Bible),
        );
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 0, "134 visible stands for itself");
        assert!(crate::session::tests::hide(&mut session, &catalog, &[134]));
        assert_eq!(session.system(), SystemId(130));
        jump_to(&mut session, &catalog, 131);
        jump_to(&mut session, &catalog, 130);
        ticks(&mut session, &catalog, 139);
        assert_eq!(aux_here(&session), 0, "not before the countdown runs out");
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 2, "130 stands for the hidden 134");
    }

    #[test]
    fn a_lost_countdown_stands() {
        let catalog = aux_only(2, -1, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        set_aux(
            &mut session,
            AuxShips {
                countdown: LOST_AT_GATE,
                arrived: 0,
                left: 2,
            },
        );
        ticks(&mut session, &catalog, 50);
        assert_eq!(aux_of(&session).countdown, LOST_AT_GATE);
        assert_eq!(aux_here(&session), 0);
    }

    #[test]
    fn the_countdown_makes_no_save_due() {
        let catalog = aux_only(2, -1, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        let _ = session.take_save_due();
        ticks(&mut session, &catalog, 145);
        assert_eq!(aux_here(&session), 2);
        assert!(!session.take_save_due());
    }

    #[test]
    fn without_0x0010_all_come_at_first_and_later_only_the_survivors() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 3);
        let first = session
            .npcs()
            .iter()
            .position(|npc| npc.aux.is_some())
            .expect("one");
        destroy(&mut session, first);
        assert_eq!(aux_here(&session), 2);
        land_and_take_off(&mut session, &catalog);
        assert_eq!(aux_here(&session), 0, "dropped");
        assert_eq!(
            aux_of(&session),
            AuxShips {
                countdown: 139,
                arrived: 0,
                left: 2
            },
            "the two survivors given back"
        );
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 2);
        // None destroyed this time, but they leave the system by a jump:
        // all given back again.
        jump_to(&mut session, &catalog, 131);
        assert_eq!(aux_of(&session).left, 2);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 2);
    }

    #[test]
    fn with_0x0010_all_come_on_every_visit_kills_or_not() {
        let catalog = aux_only(3, -1, AUX_EVERY_VISIT);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 3);
        assert_eq!(aux_of(&session).left, 3, "none taken off");
        let first = session
            .npcs()
            .iter()
            .position(|npc| npc.aux.is_some())
            .expect("one");
        destroy(&mut session, first);
        land_and_take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 3);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 3);
    }

    // The survivors.

    #[test]
    fn each_aux_ship_dropped_gives_one_back_up_to_the_count_but_those_kept() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 140);
        let ids: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.aux.is_some())
            .map(|npc| npc.id)
            .collect();
        assert_eq!(ids.len(), 3);
        session.give_back_aux_ships(&BTreeSet::from([ids[0]]));
        assert_eq!(aux_of(&session).left, 2, "one kept");
        let aux = aux_of(&session);
        set_aux(&mut session, AuxShips { left: 2, ..aux });
        session.give_back_aux_ships(&BTreeSet::new());
        assert_eq!(aux_of(&session).left, 3, "capped at the count");
        // A ship gone already (jumped out, landed) gives nothing.
        session.traffic.remove(ids[1]);
        let aux = aux_of(&session);
        set_aux(&mut session, AuxShips { left: 0, ..aux });
        session.give_back_aux_ships(&BTreeSet::new());
        assert_eq!(aux_of(&session).left, 2);
    }

    #[test]
    fn a_free_slots_aux_ships_give_nothing_back() {
        let catalog = aux_only(2, -1, 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 2);
        assert!(session.abort_mission(0, &mut NeverFires).is_ok());
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none());
        assert_eq!(aux_here(&session), 2, "they stay as traffic");
        session.give_back_aux_ships(&BTreeSet::new());
        assert!(session.pilot().missions().get(0).is_none());
    }

    /// Mission 140's three auxiliary ships come into the system in
    /// flight, none left to come; their ids, in order.
    fn three_in(session: &mut Session, catalog: &FakePilotCatalog) -> Vec<NpcId> {
        take_off(session, catalog);
        ticks(session, catalog, 140);
        let ids: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| npc.aux.is_some())
            .map(|npc| npc.id)
            .collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(aux_of(session).left, 0);
        ids
    }

    /// The rules with [`RuleKey::MoveLeftBehind`] read as the Bible.
    fn dropped_at_the_move() -> Rulebook {
        Rulebook::default().with_override(RuleKey::MoveLeftBehind, RuleSource::Bible)
    }

    /// Under the original only a ship with leader 0 is carried
    /// (`_EvalSetExp` @0x15807-0x1583e): an aux ship only once captured,
    /// `_DoShipCapture` @0x41419 setting its leader 0, the capture
    /// keeping `npc.aux`. By the other reading of
    /// [`RuleKey::MoveLeftBehind`], the rest are given back at the move.
    #[test]
    fn a_move_in_flight_to_another_system_gives_back_the_aux_ships_but_those_the_player_leads() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog).with_rules(dropped_at_the_move());
        let ids = three_in(&mut session, &catalog);
        session.fleet_up(ids[0], None);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(aux_here(&session), 1, "the captured one comes");
        assert_eq!(aux_of(&session).left, 2, "the one the player leads kept");
    }

    #[test]
    fn a_move_in_flight_to_another_system_gives_back_up_to_the_count() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog).with_rules(dropped_at_the_move());
        let _ = three_in(&mut session, &catalog);
        let aux = aux_of(&session);
        set_aux(&mut session, AuxShips { left: 2, ..aux });
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(aux_here(&session), 0, "retargeted");
        assert_eq!(aux_of(&session).left, 3, "capped at the count");
    }

    /// By the engine the ships left behind are freed by the display pass
    /// (`_HandleShipDisplay` @0x2ded6-0x2dee7), which gives nothing back,
    /// before any `_DeleteAllComputerShips` could.
    #[test]
    fn a_move_in_flight_loses_the_aux_ships_it_leaves_behind() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let _ = three_in(&mut session, &catalog);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(aux_here(&session), 0, "left behind");
        assert_eq!(aux_of(&session).left, 0, "none given back at the move");
        assert!(session.take_save_due());
        let saved = crate::save::decode(&crate::save::encode(session.pilot())).expect("decodes");
        assert_eq!(
            saved.missions().get(0).expect("held").aux.left,
            0,
            "and none in the save"
        );
        tick(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 0, "nor as they are freed");
        assert!(land_now(&mut session).is_ok());
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 0, "nor by the setup after");
    }

    #[test]
    fn a_setup_after_the_move_drops_the_ships_left_behind_and_gives_back_none() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let _ = three_in(&mut session, &catalog);
        session.run_set(&SetExpr::parse("M131").expect("parses"), &mut NeverFires);
        settle(&mut session, &catalog);
        assert_eq!(session.traffic.berths_taken(), 3, "held");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(
            session.traffic.berths_taken(),
            session.npcs().len(),
            "dropped"
        );
        assert_eq!(aux_of(&session).left, 0, "none given back");
    }

    #[test]
    fn a_relocation_gives_back_the_aux_ships() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let _ = three_in(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert_eq!(aux_here(&session), 3, "still in the system, landed");
        session
            .relocate(&catalog, SystemId(131), StellarId(140))
            .expect("relocates");
        assert_eq!(aux_here(&session), 0, "the traffic reset");
        assert_eq!(aux_of(&session).left, 3);
    }

    // The auto-abort.

    #[test]
    fn a_0x0001_mission_auto_aborts_as_its_aux_ships_come_and_its_special_part_spawns_none() {
        let catalog = aux_catalog(shipping(-1, 1), aux_ships(2, -1), AUTO_ABORT);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_some(), "waits");
        set_aux(
            &mut session,
            AuxShips {
                countdown: 0,
                arrived: 0,
                left: 2,
            },
        );
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = 1;
        }
        tick(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(aux_here(&session), 2, "its aux ships stay");
        assert!(marked(&session).is_empty(), "no special ship after");
        assert_eq!(
            session.npcs().len(),
            2,
            "the special ships' timer fired no spawn"
        );
    }

    #[test]
    fn a_0x0001_mission_auto_aborts_even_when_no_aux_ship_could_come() {
        let mut catalog = aux_only(2, -1, AUTO_ABORT);
        catalog.dudes[1].1.ships = vec![(ShipId(201), 0)];
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        set_aux(
            &mut session,
            AuxShips {
                countdown: 0,
                arrived: 0,
                left: 2,
            },
        );
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 0);
        assert!(session.pilot().missions().get(0).is_none());
    }

    // The restarts.

    /// Mission 140's aux ships after `how` into the system is raised and
    /// settled, first set to a countdown of 5, 2 arrived and 1 left.
    fn after_entering(
        catalog: &FakePilotCatalog,
        session: &mut Session,
        how: Entrance,
    ) -> AuxShips {
        set_aux(
            session,
            AuxShips {
                countdown: 5,
                arrived: 2,
                left: 1,
            },
        );
        session.raise(StoryEvent::EnteredSystem {
            system: session.pilot.system,
            how,
        });
        settle(session, catalog);
        aux_of(session)
    }

    #[test]
    fn the_hook_restarts_the_aux_ships_on_a_jump_a_gate_and_a_move_in_flight_only() {
        for (flags, left) in [(0, 1), (AUX_EVERY_VISIT, 3)] {
            let catalog = aux_only(3, 130, flags);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            for how in [
                Entrance::Jump,
                Entrance::Gate,
                Entrance::ScriptMove { landed: false },
            ] {
                assert_eq!(
                    after_entering(&catalog, &mut session, how),
                    AuxShips {
                        countdown: 139,
                        arrived: 0,
                        left
                    },
                    "{how:?} {flags}"
                );
            }
            for how in [Entrance::ScriptMove { landed: true }, Entrance::Relocate] {
                assert_eq!(
                    after_entering(&catalog, &mut session, how),
                    AuxShips {
                        countdown: 5,
                        arrived: 2,
                        left: 1
                    },
                    "{how:?} {flags}"
                );
            }
        }
    }

    #[test]
    fn the_take_off_restarts_the_aux_ships() {
        for (flags, left) in [(0, 1), (AUX_EVERY_VISIT, 3)] {
            let catalog = aux_only(3, 131, flags);
            let mut session = accepted(&catalog);
            set_aux(
                &mut session,
                AuxShips {
                    countdown: 5,
                    arrived: 2,
                    left: 1,
                },
            );
            take_off(&mut session, &catalog);
            assert_eq!(
                aux_of(&session),
                AuxShips {
                    countdown: 139,
                    arrived: 0,
                    left
                },
                "{flags}"
            );
        }
    }

    /// [`aux_only`]'s mission 140 and its twin 141, each of special
    /// ships starting 1 and auxiliary ships, both accepted.
    fn two_slots() -> (FakePilotCatalog, Session) {
        let catalog = aux_catalog(shipping(-1, 1), aux_ships(2, -1), 0);
        let mut catalog = catalog;
        let twin = crate::catalog::MissionRecord {
            id: MissionId(141),
            ..catalog.missions[0].clone()
        };
        catalog.missions.push(twin);
        let mut session = accepted(&catalog);
        assert_eq!(
            session.accept_mission(MissionId(141), &mut NeverFires),
            Ok(1)
        );
        settle(&mut session, &catalog);
        (catalog, session)
    }

    #[test]
    fn the_hook_draws_each_slots_special_timer_then_its_aux_countdown_in_slot_order() {
        let (catalog, mut session) = two_slots();
        take_off(&mut session, &catalog);
        session.raise(StoryEvent::EnteredSystem {
            system: session.pilot.system,
            how: Entrance::Jump,
        });
        let mut draws = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut draws);
        assert_eq!(draws.asked, [100, 70, 100, 70]);
    }

    #[test]
    fn the_take_offs_aux_draws_come_before_its_setup() {
        let (catalog, mut session) = two_slots();
        let mut draws = Draws::of(&[]);
        assert_eq!(session.take_off(&catalog, &mut draws), Some(StellarId(128)));
        assert_eq!(
            draws.asked[..3],
            [70, 70, crate::traffic::strength::SEED_SPREAD],
            "each slot's, in order, then the escort-strength timer, then the setup"
        );
        let mut draws = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut draws);
        assert!(!draws.asked.contains(&70), "the hook restarts none");
    }

    // The gate exit.

    /// [`gated`], mission 140 of no special ships with `count` auxiliary
    /// ships into `syst`.
    fn gated_aux(count: i16, syst: i16) -> FakePilotCatalog {
        with_aux(gated(MissionShip::NONE, 0), aux_ships(count, syst), 0)
    }

    #[test]
    fn a_gate_exit_loses_the_aux_ships_of_minus_1_until_the_next_restart() {
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = gated_aux(2, -1);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            let mut draws = Draws::of(&[]);
            exit_gate(&mut session, &catalog, kind, &mut draws);
            assert_eq!(aux_of(&session).countdown, LOST_AT_GATE, "{kind:?}");
            ticks(&mut session, &catalog, 300);
            assert_eq!(aux_here(&session), 0, "{kind:?}");
            jump_to(&mut session, &catalog, 130);
            assert_eq!(aux_of(&session).countdown, 139, "restarted by a jump");
        }
    }

    #[test]
    fn a_gate_exit_still_draws_the_lost_countdown() {
        let catalog = gated_aux(2, -1);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.raise(StoryEvent::EnteredSystem {
            system: session.pilot.system,
            how: Entrance::Gate,
        });
        let mut draws = Draws::of(&[]);
        let _ = session.settle_script(&catalog, &mut draws);
        assert_eq!(draws.asked, [70]);
        assert_eq!(aux_of(&session).countdown, LOST_AT_GATE);
    }

    #[test]
    fn a_gate_exit_loses_no_other_aux_system() {
        for syst in [-6, -2, -3, 131] {
            let catalog = gated_aux(2, syst);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
            assert_eq!(aux_of(&session).countdown, 139, "{syst}");
        }
    }

    #[test]
    fn by_the_other_reading_a_gate_exit_restarts_minus_1_as_a_jump_does() {
        let catalog = gated_aux(2, -1);
        let rules = crate::Rulebook::default().with_override(RuleKey::AuxGate, RuleSource::Bible);
        let mut session = accepted(&catalog).with_rules(rules);
        take_off(&mut session, &catalog);
        exit_gate(&mut session, &catalog, GateKind::Wormhole, &mut NeverFires);
        assert_eq!(aux_of(&session).countdown, 139);
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 2);
    }

    #[test]
    fn the_gate_exits_arrivals_frame_brings_no_aux_ship() {
        let catalog = gated_aux(2, 131);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        set_aux(
            &mut session,
            AuxShips {
                countdown: 0,
                arrived: 0,
                left: 2,
            },
        );
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(aux_here(&session), 0, "left to the hook");
        assert_eq!(aux_of(&session).countdown, 139, "which restarted it");
    }
}
