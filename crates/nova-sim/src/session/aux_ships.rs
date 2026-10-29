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
//! active (@0x43757), on the freed slot's stale record, and so does the
//! engine's reading of [`RuleKey::AuxAbortFallThrough`]: when the stale
//! appear timer runs out that frame, the special ships come and the
//! mission auto-aborts a second time (@0x43abc-0x43ad4). By the other
//! reading the part reads the slot again and skips it once freed. No
//! stock mission of `Flags` 0x0001 has auxiliary ships.
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
//! nothing back. Nor does one the drop keeps: at a setup the fleet's
//! escorts (`_DeleteAllComputerShips(0)` @0x3bf7e-0x3bfb6), so a
//! captured aux ship keeps its mark and gives one back only at the
//! first setup after its release.
//!
//! [`AuxShips::tick`]: crate::mission::aux_ships::AuxShips::tick
//! [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
//! [`AuxShips::give_back`]: crate::mission::aux_ships::AuxShips::give_back

use crate::pilot::Pilot;
use std::collections::BTreeSet;

use super::{MissionRun, Session, Statics};
use crate::catalog::TrafficCatalog;
use crate::chance::Chance;
use crate::mission::Stop;
use crate::mission::aux_ships::{aux_dude, aux_system_allows};
use crate::mission::ships::LOST_AT_GATE;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::Entrance;
use crate::traffic::SPAWN_RESERVE;
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
    /// that finds a slot below the spawn reserve counted
    /// ([`AuxShips::came`]): one `_SpecificDudeSpawn` finds none for
    /// (reserve 8 @0x435a8, -1 @0x435c7) goes on to the next
    /// (@0x4372f), counting nothing, so it is still to come. Then a
    /// mission of `Flags` 0x0001 auto-aborts. No save is due.
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
        let record = self.statics.lifecycle_record(active.mission);
        let coming = active.aux.coming(&record.aux);
        let holder = |stop: Option<Stop>| {
            stop.and_then(|stop| {
                self.chart
                    .holder_of(&self.pilot, &self.statics, stop.stellar)
            })
        };
        let holders = (holder(active.destination), holder(active.return_to));
        let map = self.chart.galaxy.map();
        if coming <= 0
            || !aux_system_allows(
                record.aux.syst,
                self.pilot.system,
                holders,
                map,
                &self.statics.govts,
            )
        {
            return;
        }
        let Some(dude) = aux_dude(&record.aux) else {
            return;
        };
        let items = self.setup.npc_items();
        self.space.traffic.table_mut().add_mission_dudes(
            catalog,
            &[dude],
            &self.statics.ships,
            &self.statics.outfits,
            &self.statics.arsenal,
            items,
        );
        let spawn = AuxSpawn {
            slot,
            dude,
            count: coming,
        };
        let ships = spawn::arriving_aux_ships(
            self.space.traffic.table(),
            &spawn,
            self.appear_gate(),
            chance,
        );
        let came = self.space.traffic.bring_in_within(
            ships,
            self.setup.spawn_reserve(SPAWN_RESERVE),
            &self.statics.govts,
            &mut *chance,
        );
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
        restart_aux(&mut self.pilot, &self.statics, slot, chance);
    }

    /// The take-off's restart of every active mission's auxiliary ships,
    /// in slot order ([`Session::restart_aux`]), drawing on `chance`; the
    /// launch's too ([`Session::open_at_launch`]).
    pub(super) fn restart_every_aux(&mut self, chance: &mut dyn Chance) {
        restart_every_aux(&mut self.pilot, &self.statics, chance);
    }

    /// Open Pilot's restart of every active mission's auxiliary ships, in
    /// slot order ([`AuxShips::restart_from_main_screen`]), drawing on
    /// `chance`: `_MainScreenChoice(1)`'s loop over the 16 slots
    /// (@0x1a918-0x1a982), run once the pilot is loaded. No save is due.
    /// Which slot draws first decides nothing the player sees (the
    /// engine's generator isn't the original's).
    ///
    /// [`AuxShips::restart_from_main_screen`]: crate::mission::aux_ships::AuxShips::restart_from_main_screen
    pub(crate) fn restart_aux_from_main_screen(&mut self, chance: &mut dyn Chance) {
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            let Some(active) = self.pilot.missions.get(slot) else {
                continue;
            };
            let record = self.statics.lifecycle_record(active.mission);
            if let Some(active) = self.pilot.missions.get_mut(slot) {
                active
                    .aux
                    .restart_from_main_screen(&record.aux, record.flags, chance);
            }
        }
    }

    /// The setup of a pilot opened at launch, once the load has run (its
    /// sweep with it), drawing on `chance`, as `_OpenLastPilotFile` and
    /// `_DoMainScreen`'s once-a-launch setup do:
    ///
    /// - every active mission's auxiliary ships restarted, in slot order,
    ///   in the unguarded form the take-off uses ([`AuxShips::restart`],
    ///   @0x765d4-0x76637): unlike [`Session::restart_aux_from_main_screen`]
    ///   it redraws the countdown and zeroes those arrived even with none
    ///   left to come;
    /// - the player's system explored (`_ExploreNebulaeFromSystem(current)`
    ///   @0x1affe), raising `Explored` for it to settle.
    ///
    /// No save is due. Open Pilot runs neither, nor does a return from
    /// play. The original sweeps after the load and this runs after the
    /// session's own sweep; both settle before the main menu's first
    /// frame.
    ///
    /// [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
    pub(crate) fn open_at_launch(&mut self, chance: &mut dyn Chance) {
        self.run
            .open_at_launch(&mut self.pilot, &self.statics, chance);
    }

    /// The player's exit from a hypergate or wormhole, after the
    /// enter-system hook's restart of the mission in `slot`: under the
    /// engine's reading of [`RuleKey::AuxGate`], an `AuxShipSyst` of -1
    /// loses its ships until the next restart, its countdown 0x7fff
    /// (@0x6729b-0x672ac). Nothing under the other reading.
    pub(super) fn lose_aux_at_gate(&mut self, slot: u8) {
        if self.setup.rules.source_for(RuleKey::AuxGate) != RuleSource::Engine {
            return;
        }
        let Some(active) = self.pilot.missions.get(slot) else {
            return;
        };
        if self.statics.lifecycle_record(active.mission).aux.syst != ANY_SYSTEM {
            return;
        }
        if let Some(active) = self.pilot.missions.get_mut(slot) {
            active.aux.countdown = LOST_AT_GATE;
        }
    }

    /// The system's ships dropped but those in `kept`: each other
    /// auxiliary ship of an active slot gives one back to it, capped at
    /// the slot's count (`_DeleteAllComputerShips` @0x3bfe8-0x3c024). A
    /// kept ship, one `_DeleteAllComputerShips(0)` skips whole
    /// (@0x3bf7e-0x3bfb6), gives nothing back.
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
            let record = self.statics.lifecycle_record(active.mission);
            if let Some(active) = self.pilot.missions.get_mut(slot) {
                active.aux.give_back(&record.aux);
            }
        }
    }

    /// The enter-system hook's auxiliary part for the mission in `slot`
    /// on an entry `how` it runs on, drawing on `chance`: the restart,
    /// then on a gate's exit the loss of `AuxShipSyst` -1. On a gate's
    /// exit it runs after that frame's arrivals, where the original's hook
    /// restarts the countdown before them, so the countdown's one tick of
    /// that frame is lost; it crosses no boundary the player sees, so it
    /// is not built (see [`Session::mission_ship_arrivals`]).
    pub(super) fn aux_on_entry(&mut self, slot: u8, how: Entrance, chance: &mut dyn Chance) {
        self.restart_aux(slot, chance);
        if how == Entrance::Gate {
            self.lose_aux_at_gate(slot);
        }
    }
}

impl MissionRun {
    /// The setup of a pilot opened at launch, once the load has run (its
    /// sweep with it), drawing on `chance`, as `_OpenLastPilotFile` and
    /// `_DoMainScreen`'s once-a-launch setup do:
    ///
    /// - every active mission's auxiliary ships restarted, in slot order,
    ///   in the unguarded form the take-off uses ([`AuxShips::restart`],
    ///   @0x765d4-0x76637): unlike [`Session::restart_aux_from_main_screen`]
    ///   it redraws the countdown and zeroes those arrived even with none
    ///   left to come;
    /// - the player's system explored (`_ExploreNebulaeFromSystem(current)`
    ///   @0x1affe), raising `Explored` for it to settle.
    ///
    /// No save is due. Open Pilot runs neither, nor does a return from
    /// play. The original sweeps after the load and this runs after the
    /// session's own sweep; both settle before the main menu's first
    /// frame.
    ///
    /// [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
    fn open_at_launch(&mut self, pilot: &mut Pilot, statics: &Statics, chance: &mut dyn Chance) {
        restart_every_aux(pilot, statics, chance);
        self.explore_here(pilot);
    }
}

/// The restart of the auxiliary ships of the mission in `slot`, if
/// any, by the enter-system hook or the take-off
/// ([`AuxShips::restart`]), drawing on `chance`.
///
/// [`AuxShips::restart`]: crate::mission::aux_ships::AuxShips::restart
fn restart_aux(pilot: &mut Pilot, statics: &Statics, slot: u8, chance: &mut dyn Chance) {
    let Some(active) = pilot.missions.get(slot) else {
        return;
    };
    let record = statics.lifecycle_record(active.mission);
    if let Some(active) = pilot.missions.get_mut(slot) {
        active.aux.restart(&record.aux, record.flags, chance);
    }
}

/// The take-off's restart of every active mission's auxiliary ships,
/// in slot order ([`Session::restart_aux`]), drawing on `chance`; the
/// launch's too ([`Session::open_at_launch`]).
fn restart_every_aux(pilot: &mut Pilot, statics: &Statics, chance: &mut dyn Chance) {
    let slots: Vec<u8> = pilot.missions.iter().map(|(slot, _)| slot).collect();
    for slot in slots {
        restart_aux(pilot, statics, slot, chance);
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
        accepted, aux_catalog, aux_ships, crowd, jump_to, marked, settle, shipping, take_off, tick,
        ticks, with_aux,
    };
    use crate::story::StoryEvent;
    use crate::testkit::{Draws, FAST, FakePilotCatalog, land_now, ship};

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
        session.space.traffic.npcs_mut()[index].condition = Condition::Destroyed;
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
    fn aux_ships_read_appear_on_as_the_last_galaxy_refresh_saw_it_by_the_engine() {
        // `_EnterMoreShips` → `_HyperSpawnFleet`/`_SelectShipFieldFromDude`
        // read shïp +0xa41 (@0x43bd3/@0x43dcf), cached at the landing's
        // refresh: b5, set since, is unseen by the engine.
        let mut catalog = crate::testkit::far_129_reachable(aux_only(3, -1, 0));
        let mut gated = ship(202, FAST);
        gated.appear_on = crate::control::Test::parse("!b5");
        catalog.ship_records.push(gated);
        for record in &mut catalog.ship_records {
            if record.id == ShipId(201) {
                record.appear_on = crate::control::Test::parse("b5");
            }
        }
        catalog.dudes[1].1.ships = vec![(ShipId(201), 50), (ShipId(202), 50)];
        for (source, drawn) in [(RuleSource::Engine, 202), (RuleSource::Bible, 201)] {
            let mut session = Session::start(&catalog)
                .expect("starts")
                .with_rules(Rulebook::default().with_override(RuleKey::SpawnAvailability, source));
            assert_eq!(land_now(&mut session), Ok(StellarId(128)));
            settle(&mut session, &catalog);
            session.set_control_bit(crate::control::Bit::new(5).expect("a bit"), true);
            assert_eq!(
                session.accept_mission(MissionId(140), &mut NeverFires),
                Ok(0)
            );
            settle(&mut session, &catalog);
            take_off(&mut session, &catalog);
            ticks(&mut session, &catalog, 140);
            let types: Vec<ShipId> = session
                .npcs()
                .iter()
                .filter(|npc| npc.aux == Some(0))
                .map(|npc| npc.ship)
                .collect();
            assert_eq!(types, [ShipId(drawn); 3], "{source:?}");
        }
    }

    #[test]
    fn aux_ships_find_no_slot_once_fifty_five_ships_are_in_the_system_and_keep_their_count() {
        let catalog = crate::testkit::far_129_reachable(aux_only(3, -1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 139);
        crowd(&mut session, 55);
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 0, "no slot");
        assert_eq!(
            aux_of(&session),
            AuxShips {
                countdown: 0,
                arrived: 0,
                left: 3
            },
            "none counted (@0x435c7 → @0x4372f)"
        );
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        ticks(&mut session, &catalog, 139);
        crowd(&mut session, 54);
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 1, "the one slot left");
        assert_eq!(
            (aux_of(&session).arrived, aux_of(&session).left),
            (1, 2),
            "only the one that came"
        );
        tick(&mut session, &catalog);
        assert_eq!(aux_here(&session), 1, "the rest still wait for a slot");
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
        // Only a system the catalog can read lists its stellars.
        catalog.systems.push(SystemId(132));
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
            session.galaxy().holder(
                StellarId(128),
                session.statics.listers(StellarId(128)),
                SystemId(130)
            ),
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
        session.space.traffic.remove(ids[1]);
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

    /// [`aux_only`]'s mission 140 of three auxiliary ships and its twin
    /// 141, made by `twin`; 140's three come in, the player lands at 128,
    /// 140 is aborted there, its ships staying marked with slot 0, and
    /// 141 is accepted into the freed slot, its ships left to come set
    /// to 0 as though they had all come and died.
    fn stale_marks(
        twin: impl FnOnce(&mut crate::catalog::MissionRecord),
    ) -> (FakePilotCatalog, Session) {
        let mut catalog = aux_only(3, -1, 0);
        let mut record = crate::catalog::MissionRecord {
            id: MissionId(141),
            ..catalog.missions[0].clone()
        };
        twin(&mut record);
        catalog.missions.push(record);
        let mut session = accepted(&catalog);
        let _ = three_in(&mut session, &catalog);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert!(session.abort_mission(0, &mut NeverFires).is_ok());
        settle(&mut session, &catalog);
        assert_eq!(aux_here(&session), 3, "still marked with slot 0");
        assert_eq!(
            session.accept_mission(MissionId(141), &mut NeverFires),
            Ok(0)
        );
        settle(&mut session, &catalog);
        let aux = aux_of(&session);
        set_aux(&mut session, AuxShips { left: 0, ..aux });
        (catalog, session)
    }

    /// Nothing at a mission's end clears an auxiliary ship's mark
    /// (`_AbortMission` @0x99a92-0x99bdb), and the drop tests only the
    /// slot's active byte (@0x3bfe8), so a survivor gives one back to
    /// whichever mission holds its slot.
    #[test]
    fn a_survivor_of_a_freed_slot_gives_one_back_to_the_next_mission_in_it() {
        let (catalog, mut session) = stale_marks(|_| {});
        take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 3, "the drop gave 140's back to 141");
        ticks(&mut session, &catalog, 140);
        assert_eq!(aux_here(&session), 3, "141's ships come");
        assert_eq!(aux_of(&session).arrived, 3);
    }

    #[test]
    fn a_stale_give_back_is_capped_at_the_new_missions_count() {
        let (catalog, mut session) = stale_marks(|twin| twin.aux.count = 1);
        take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 1);
    }

    #[test]
    fn a_stale_mark_gives_nothing_to_a_new_mission_of_no_aux_ships() {
        let (catalog, mut session) = stale_marks(|twin| twin.aux.dude = None);
        take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 0);
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
    /// `_HandlePlayerBoardAttempt` @0x65bbf setting its leader 0, the capture
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

    // Kept across a setup.

    /// `_DeleteAllComputerShips(0)` skips a ship of AI type above 4 that
    /// the player leads, defending no stellar, of no mission slot and
    /// not disabled (@0x3bf7e-0x3bfb6): a captured aux ship gives
    /// nothing back.
    #[test]
    fn a_setup_gives_back_no_aux_ship_for_an_escort_it_keeps() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let ids = three_in(&mut session, &catalog);
        session.fleet_up(ids[0], None);
        land_and_take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 2, "the escort kept");
    }

    /// The board capture never writes +0x8a (@0x65bb1-0x65c76), and a
    /// kept ship keeps every field.
    #[test]
    fn an_escort_kept_across_a_setup_keeps_its_aux_mark() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let ids = three_in(&mut session, &catalog);
        session.fleet_up(ids[0], None);
        land_and_take_off(&mut session, &catalog);
        let escort = session.space.escorts.fleet[0].expect("placed");
        assert_eq!(session.space.npc(escort).expect("in").aux, Some(0));
    }

    /// Each escort keeps its own mark: one with none (as a hired escort,
    /// `_SpawnEscort` clearing +0x8a @0x3ff8c) stays without one.
    #[test]
    fn each_escort_kept_across_a_setup_keeps_its_own_mark() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let ids = three_in(&mut session, &catalog);
        session.fleet_up(ids[0], None);
        session.space.npc_mut(ids[0]).expect("in").aux = None;
        session.fleet_up(ids[1], None);
        land_and_take_off(&mut session, &catalog);
        let marks: Vec<Option<u8>> = session
            .space
            .escorts
            .fleet
            .iter()
            .map(|id| session.space.npc(id.expect("placed")).expect("in").aux)
            .collect();
        assert_eq!(marks, vec![None, Some(0)]);
        assert_eq!(aux_of(&session).left, 1, "only the third given back");
    }

    /// Released, it is no longer kept, so the next setup gives it back
    /// (@0x3bfe8-0x3c024).
    #[test]
    fn an_escort_released_after_a_setup_gives_its_aux_ship_back_at_the_next() {
        let catalog = aux_only(3, -1, 0);
        let mut session = accepted(&catalog);
        let ids = three_in(&mut session, &catalog);
        session.fleet_up(ids[0], None);
        land_and_take_off(&mut session, &catalog);
        let escort = session.space.escorts.fleet[0].expect("placed");
        session.release(escort);
        land_and_take_off(&mut session, &catalog);
        assert_eq!(aux_of(&session).left, 3, "the released one given back");
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
        assert_eq!(session.space.traffic.berths_taken(), 3, "held");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(
            session.space.traffic.berths_taken(),
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

    /// What one frame of a `Flags` 0x0001 mission's two auxiliary ships
    /// coming does, its special ships (two, `ShipStart` 1) here with
    /// their appear timer at `appear`, under `reading` of
    /// [`RuleKey::AuxAbortFallThrough`]: the session after it, the
    /// mission's `OnAbort` toggling bit 7 and setting bit 8, with
    /// `DatePostInc` 1, `Flags` 0x0008 and `Flags2` 0x0002 paying 1000.
    struct FellThrough {
        session: Session,
        catalog: FakePilotCatalog,
        cash: i64,
        fuel: f32,
        date: crate::date::GameDate,
        pay: i64,
    }

    fn fall_through(appear: i16, reading: RuleSource) -> FellThrough {
        fall_through_with(appear, reading, |_| {}, |_| {})
    }

    /// [`fall_through`] with `edit` applied to its catalog, and `before`
    /// to the session just before the frame.
    fn fall_through_with(
        appear: i16,
        reading: RuleSource,
        edit: impl FnOnce(&mut FakePilotCatalog),
        before: impl FnOnce(&mut Session),
    ) -> FellThrough {
        let mut catalog = aux_catalog(
            shipping(-1, 1),
            aux_ships(2, -1),
            AUTO_ABORT | crate::mission::active::AUTO_ABORT_FUEL,
        );
        let record = &mut catalog.missions[0];
        record.on_abort = crate::control::Script::parse("^b7 b8");
        record.date_post_inc = 1;
        record.flags2 = crate::mission::active::AUTO_ABORT_PAYS;
        record.pay_val = 1000;
        edit(&mut catalog);
        let mut session = accepted(&catalog)
            .with_rules(Rulebook::default().with_override(RuleKey::AuxAbortFallThrough, reading));
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
        let pay = session.pilot().missions().get(0).expect("held").pay;
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = appear;
        }
        let (cash, fuel, date) = (
            session.pilot().cash,
            session.reserves().fuel.now,
            session.pilot().date(),
        );
        before(&mut session);
        tick(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "auto-aborted");
        assert_eq!(aux_here(&session), 2, "its aux ships stay");
        FellThrough {
            session,
            catalog,
            cash,
            fuel,
            date,
            pay,
        }
    }

    /// The special ships (of ship 200) in the system, marked or not.
    fn special_here(session: &Session) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == ShipId(200))
            .count()
    }

    /// The bits 7 and 8 the `OnAbort` of [`fall_through`] left.
    fn abort_bits(session: &Session) -> (bool, bool) {
        let bit = |n| session.control_bit(crate::control::Bit::new(n).expect("a bit"));
        (bit(7), bit(8))
    }

    #[test]
    fn a_0x0001_mission_auto_aborted_by_its_aux_ships_brings_its_special_ships_as_its_timer_runs_out()
     {
        let FellThrough {
            mut session,
            catalog,
            ..
        } = fall_through(1, RuleSource::Engine);
        assert_eq!(special_here(&session), 2, "they came");
        assert!(
            marked(&session).is_empty(),
            "the frame's sweep unmarked them, their slot free"
        );
        tick(&mut session, &catalog);
        assert_eq!(special_here(&session), 2, "they stay as traffic");
        assert_eq!(session.npcs().len(), 4);
    }

    #[test]
    fn a_0x0800_mission_falling_through_brings_its_special_ships_of_the_kept_type() {
        // A draw on `NeverFires` gives ship 129, the düde's last type;
        // 200 is kept.
        let FellThrough { session, .. } = fall_through_with(
            1,
            RuleSource::Engine,
            |catalog| {
                catalog.ship_records.push(ship(129, FAST));
                catalog.dudes[0].1.ships = vec![(ShipId(200), 20), (ShipId(129), 17)];
                catalog.missions[0].flags |= 0x0800;
            },
            |session| {
                if let Some(active) = session.pilot.missions.get_mut(0) {
                    active.ships.uniform_type = Some(ShipId(200));
                }
            },
        );
        let special: Vec<ShipId> = session
            .npcs()
            .iter()
            .map(|npc| npc.ship)
            .filter(|ship| *ship != ShipId(201))
            .collect();
        assert_eq!(special, [ShipId(200); 2], "the kept type, not a draw");
    }

    #[test]
    fn the_fall_through_auto_aborts_again() {
        let fell = fall_through(1, RuleSource::Engine);
        let session = &fell.session;
        assert_eq!(abort_bits(session), (false, true), "OnAbort ran twice");
        assert_eq!(session.pilot().cash, fell.cash + 2 * fell.pay, "paid twice");
        assert!(fell.pay > 0);
        assert_eq!(
            session.pilot().date(),
            fell.date.next_day().next_day(),
            "two days passed"
        );
        let taken = fell.fuel - session.reserves().fuel.now;
        assert!(
            (taken - 2.0 * crate::mission::active::AUTO_ABORT_FUEL_TAKEN).abs() < 1e-3,
            "the fuel taken twice: {taken}"
        );
    }

    #[test]
    fn the_fall_through_brings_nothing_while_the_special_timer_still_runs() {
        let fell = fall_through(5, RuleSource::Engine);
        let session = &fell.session;
        assert_eq!(special_here(session), 0);
        assert_eq!(abort_bits(session), (true, true), "OnAbort ran once");
        assert_eq!(session.pilot().cash, fell.cash + fell.pay, "paid once");
        assert_eq!(session.pilot().date(), fell.date.next_day());
    }

    #[test]
    fn by_the_other_reading_the_freed_slot_is_skipped() {
        let fell = fall_through(1, RuleSource::Bible);
        let session = &fell.session;
        assert_eq!(special_here(session), 0, "the timer fired no spawn");
        assert_eq!(abort_bits(session), (true, true), "OnAbort ran once");
        assert_eq!(session.pilot().cash, fell.cash + fell.pay, "paid once");
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
        session.run.raise(StoryEvent::EnteredSystem {
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
        session.run.raise(StoryEvent::EnteredSystem {
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

    #[test]
    fn open_pilots_restart_draws_each_slot_with_ships_left_in_slot_order() {
        let (_, mut session) = two_slots();
        let spent = |left| AuxShips {
            countdown: 5,
            arrived: 2,
            left,
        };
        for (slot, left) in [(0, 1), (1, 0)] {
            if let Some(active) = session.pilot.missions.get_mut(slot) {
                active.aux = spent(left);
            }
        }
        let _ = session.take_save_due();
        let mut draws = Draws::of(&[9]);
        session.restart_aux_from_main_screen(&mut draws);
        let aux = |slot| session.pilot().missions().get(slot).expect("held").aux;
        assert_eq!(
            aux(0),
            AuxShips {
                countdown: 79,
                arrived: 0,
                left: 1
            }
        );
        assert_eq!(aux(1), spent(0), "none left: as saved");
        assert_eq!(draws.asked, [70], "one draw, slot 0's");
        assert!(!session.take_save_due(), "no save is due");
    }

    #[test]
    fn the_launchs_restart_redraws_every_slot_even_with_none_left() {
        // `_OpenLastPilotFile`'s loop has no +0x6e guard (@0x765d4-0x76637).
        let (_, mut session) = two_slots();
        let spent = |left| AuxShips {
            countdown: 5,
            arrived: 2,
            left,
        };
        for (slot, left) in [(0, 1), (1, 0)] {
            if let Some(active) = session.pilot.missions.get_mut(slot) {
                active.aux = spent(left);
            }
        }
        let _ = session.take_save_due();
        let mut draws = Draws::of(&[9, 69]);
        session.open_at_launch(&mut draws);
        let aux = |slot| session.pilot().missions().get(slot).expect("held").aux;
        for (slot, left) in [(0, 1), (1, 0)] {
            let restarted = aux(slot);
            assert!(
                (70..140).contains(&restarted.countdown),
                "slot {slot}: Rand(70)+70, got {}",
                restarted.countdown
            );
            assert_eq!(
                (restarted.arrived, restarted.left),
                (0, left),
                "slot {slot}"
            );
        }
        assert_eq!(draws.asked, [70, 70], "a draw for each slot");
        assert!(!session.take_save_due(), "no save is due");
    }

    #[test]
    fn the_launchs_restart_refills_every_visits_ships() {
        let catalog = crate::testkit::far_129_reachable(aux_only(3, -1, AUX_EVERY_VISIT));
        let mut session = accepted(&catalog);
        set_aux(
            &mut session,
            AuxShips {
                countdown: 0,
                arrived: 3,
                left: 0,
            },
        );
        session.open_at_launch(&mut Draws::of(&[3]));
        assert_eq!(
            aux_of(&session),
            AuxShips {
                countdown: 73,
                arrived: 0,
                left: 3
            }
        );
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
        session.run.raise(StoryEvent::EnteredSystem {
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
