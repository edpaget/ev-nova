//! The session's side of mission ships (see the
//! [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships"):
//! their state drawn as a mission is accepted, their setup with the
//! system's ships, the enter-system hook that restarts their appear
//! timer, the timer and the ships it brings in, and what becomes of them
//! as they are destroyed and as their mission ends.
//!
//! **The hook** is no loop of its own: each slot filled registers
//! [`ship_hook`], `{MissionShips, slot, 0}`, on every `EnteredSystem`,
//! with a blank guard and no script, and its effect reads the event's
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

use super::Session;
use crate::catalog::{DudeId, MissionRecord, TrafficCatalog};
use crate::chance::Chance;
use crate::control::Script;
use crate::geometry::Vec2;
use crate::mission::ships::{ShipBehav, ShipFacts, ShipStart, appear_timer};
use crate::mission::{ActiveMission, ShipSystem, SpecialShips};
use crate::story::{
    Consumer, Entrance, EventFilter, StoryEvent, StoryEventKind, Subscription, SubscriptionId,
};
use crate::traffic::npc::NpcId;
use crate::traffic::spawn::{self, MissionSpawn, NewShip};
use crate::traffic::table::SpawnTable;

/// The appear timer's highest value that counts down (32000 @0x437f5).
const COUNTED: i16 = 32_000;

/// Slot `slot`'s enter-system hook.
#[must_use]
pub const fn ship_hook(slot: u8) -> SubscriptionId {
    SubscriptionId {
        consumer: Consumer::MissionShips,
        record: slot as i16,
        hook: 0,
    }
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

    /// Registers `slot`'s enter-system hook ([`ship_hook`]).
    pub(super) fn register_ship_hook(&mut self, slot: u8) {
        let subscription = Subscription::new(
            ship_hook(slot),
            EventFilter::Any(StoryEventKind::EnteredSystem),
            Script::default(),
        );
        let registered = self.register_story(subscription);
        debug_assert!(registered.is_ok(), "a slot filled registers once");
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

    /// Slot `id.record`'s enter-system hook on `event`, drawing on
    /// `chance` (see the module docs): restarts its special ships'
    /// appear timer ([`appear_timer`]) and zeroes their arrived count.
    pub(super) fn mission_ships_effect(
        &mut self,
        id: SubscriptionId,
        event: &StoryEvent,
        chance: &mut dyn Chance,
    ) {
        let &StoryEvent::EnteredSystem { how, .. } = event else {
            return;
        };
        let Ok(slot) = u8::try_from(id.record) else {
            return;
        };
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
            let Some(dude) = record.ships.dude.filter(|_| fires) else {
                continue;
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
            self.traffic.bring_in(ships);
            if let Some(active) = self.pilot.missions.get_mut(slot) {
                active.ships.arrived = active.ships.arrived.saturating_add(count);
            }
        }
    }

    /// NPC `id`, destroyed: when it is a mission's special ship, its
    /// slot's ships left go down by one while above none
    /// (`_HandleShipDisplay` @0x2dbc5-0x2dbf1), so it is not brought in
    /// again.
    pub(super) fn count_lost_mission_ship(&mut self, id: NpcId) {
        let Some(mark) = self
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .and_then(|npc| npc.mission)
        else {
            return;
        };
        if let Some(active) = self.pilot.missions.get_mut(mark.slot)
            && active.ships.left > 0
        {
            active.ships.left -= 1;
            self.save_due = true;
        }
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
    use crate::catalog::{
        CommCatalog, DudeRecord, GovtId, MissionId, MissionShip, PersonId, PersonRecord, ShipId,
        StellarId, SystemId, SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::control::SetExpr;
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
        session.count_lost_mission_ship(id);
        assert_eq!(ships_of(&session).left, 1);
        session.traffic.remove(id);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0]);
        let other = session.npcs()[0].id;
        session.count_lost_mission_ship(other);
        session.count_lost_mission_ship(other);
        assert_eq!(ships_of(&session).left, 0, "never below none");
    }

    #[test]
    fn a_ship_destroyed_in_the_fight_is_counted() {
        let catalog = ships_catalog(shipping(-1, 0));
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        session.traffic.npcs_mut()[0].condition = crate::combat::hull::Condition::Destroyed;
        session.take_out_the_destroyed();
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
        session.count_lost_mission_ship(id);
        assert_eq!(ships_of(&session).left, 2);
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
}
