//! Mission ships at a hypergate or wormhole exit, as the engine reads
//! [`RuleKey::GateFollow`]: what the exit makes of each slot whose ships
//! follow the player (`ShipSyst` -6), drawn before the system is entered
//! ([`Session::exit_gate_slots`], over [`through_gate`]).
//!
//! The original's gate path (`_HandlePlayerDockRequest` @0x67159-0x674a5)
//! runs the enter-system hook, then loop A over the slots, then the
//! system's setup, then one `_EnterMoreShips` frame, then loop B. Loop A
//! overwrites the hook's restart of a -6 slot, so here the hook leaves a
//! -6 slot alone on a gate's exit (see the `mission_ships` module), and
//! the exit's setup takes a -6 slot of escorts at once. The follow rolls
//! are drawn before the system is entered, as every caller of the entry
//! draws first, where the original draws them after the hook's restarts.
//!
//! By the other reading the exit leaves the slots as a jump does.

use super::Session;
use crate::catalog::TrafficCatalog;
use crate::chance::Chance;
use crate::gate::GateKind;
use crate::mission::ShipSystem;
use crate::mission::ships::{GateSlot, ShipBehav, through_gate};
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::Entrance;

impl Session {
    /// Whether the engine's reading of [`RuleKey::GateFollow`] decides
    /// what an entry `how` does to ships in `system`: a gate's exit, the
    /// ships following the player.
    pub(super) fn gate_decides(&self, how: Entrance, system: ShipSystem) -> bool {
        how == Entrance::Gate
            && system == ShipSystem::FollowsPlayer
            && self.rules.source_for(RuleKey::GateFollow) == RuleSource::Engine
    }

    /// The player's exit from a gate of `kind`, before the system is
    /// entered: under the engine's reading of [`RuleKey::GateFollow`],
    /// each active slot, in order, gets the appear timer and arrived count
    /// [`through_gate`] gives it, its `düde`'s government read from
    /// `catalog`, drawing on `chance` (loop A, @0x6716a-0x67294). Nothing
    /// under the other reading.
    pub(super) fn exit_gate_slots(
        &mut self,
        kind: GateKind,
        catalog: &(impl TrafficCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) {
        if self.rules.source_for(RuleKey::GateFollow) != RuleSource::Engine {
            return;
        }
        let slots: Vec<u8> = self.pilot.missions.iter().map(|(slot, _)| slot).collect();
        for slot in slots {
            let Some(active) = self.pilot.missions.get(slot) else {
                continue;
            };
            let record = self.lifecycle_record(active.mission);
            let govt = record
                .ships
                .dude
                .and_then(|dude| catalog.dude(dude))
                .and_then(|dude| dude.govt);
            let facts = GateSlot {
                system: active.ships.system,
                behav: ShipBehav::from_code(record.ships.behav),
                left: active.ships.left,
                flags2: govt.map(|govt| self.govts.get(govt).map_or(0, |govt| govt.flags2)),
            };
            let Some(timer) = through_gate(kind, &facts, chance) else {
                continue;
            };
            if let Some(active) = self.slot_holding(slot, record.id) {
                active.ships.appear = timer.appear;
                active.ships.arrived = timer.arrived;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GateSite, MissionShip, StellarId, SystemId};
    use crate::chance::NeverFires;
    use crate::gate::{HYPERGATE, WORMHOLE};
    use crate::geometry::Vec2;
    use crate::govt::NEVER_FOLLOWS_THROUGH_HYPERGATE;
    use crate::mission::ships::LOST_AT_GATE;
    use crate::rulebook::Rulebook;
    use crate::session::LandPress;
    use crate::session::mission_ships::tests::{
        accepted, jump_to, marked, settle, shipping, ships_catalog, ships_of, take_off, ticks,
    };
    use crate::testkit::{FakePilotCatalog, govt, land_now, planet};

    /// Where hypergate 300 is in 130.
    const HYPERGATE_AT: Vec2 = Vec2::new(-3000.0, 0.0);
    /// Where wormhole 400 is in 130.
    const WORMHOLE_AT: Vec2 = Vec2::new(0.0, 3000.0);

    /// Two special ships of düde 150 following the player (`ShipSyst`
    /// -6) of `ShipBehav` `behav` and `ShipStart` `start`.
    pub(in crate::session) fn following(behav: i16, start: i16) -> MissionShip {
        MissionShip {
            behav,
            ..shipping(-6, start)
        }
    }

    /// Stellar `id` of `system` at `at` with `flags2`, linked to `links`,
    /// heading ships out on `exit_angle`.
    fn gate_site(id: i16, system: i16, at: Vec2, flags2: u16, links: &[i16]) -> GateSite {
        let mut slots = [None; 8];
        for (slot, &link) in slots.iter_mut().zip(links) {
            *slot = Some(StellarId(link));
        }
        GateSite {
            id: StellarId(id),
            system: SystemId(system),
            position: at,
            flags2,
            links: slots,
            exit_angle: 90,
        }
    }

    /// [`ships_catalog`] with `ships`, düde 150's government 131 of
    /// `Flags2` `flags2`; in 130, hypergate 300 at (-3000, 0), linked to
    /// 310 in 131 at (100, 200), and unlinked wormhole 400 at (0, 3000),
    /// leading to unlinked wormhole 410 in 131 at (-300, 40).
    pub(in crate::session) fn gated(ships: MissionShip, flags2: u16) -> FakePilotCatalog {
        let mut catalog = ships_catalog(ships);
        catalog.govts = vec![crate::catalog::GovtRecord {
            flags2,
            ..govt(131)
        }];
        let mut hypergate = planet(300, HYPERGATE_AT.x, HYPERGATE_AT.y);
        hypergate.flags2 = HYPERGATE;
        let mut wormhole = planet(400, WORMHOLE_AT.x, WORMHOLE_AT.y);
        wormhole.flags2 = WORMHOLE;
        if let Some((_, sites)) = catalog
            .sites
            .iter_mut()
            .find(|(system, _)| *system == SystemId(130))
        {
            sites.extend([hypergate, wormhole]);
        }
        catalog.gates = vec![
            gate_site(300, 130, HYPERGATE_AT, HYPERGATE, &[310]),
            gate_site(310, 131, Vec2::new(100.0, 200.0), HYPERGATE, &[300]),
            gate_site(400, 130, WORMHOLE_AT, WORMHOLE, &[]),
            gate_site(410, 131, Vec2::new(-300.0, 40.0), WORMHOLE, &[]),
        ];
        catalog
    }

    /// Flies from the gate of `kind` in 130 into 131, drawing on
    /// `chance`, and settles.
    pub(in crate::session) fn exit_gate(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        kind: GateKind,
        chance: &mut dyn Chance,
    ) {
        session.player.position = match kind {
            GateKind::Hypergate => HYPERGATE_AT,
            GateKind::Wormhole => WORMHOLE_AT,
        };
        session.player.velocity = Vec2::ZERO;
        session.nav_target = None;
        session.land().expect("selects");
        assert!(matches!(session.land(), Ok(LandPress::AtGate { kind: at, .. }) if at == kind));
        let out = match kind {
            GateKind::Hypergate => session.enter_hypergate(Some(SystemId(131)), catalog, chance),
            GateKind::Wormhole => session.enter_wormhole(catalog, chance),
        };
        assert_eq!(out, Ok(SystemId(131)));
        settle(session, catalog);
    }

    /// The rules with [`RuleKey::GateFollow`] read as `source`.
    fn reading(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::GateFollow, source)
    }

    /// The timer and arrived count of slot 0's ships.
    fn timer(session: &Session) -> (i16, i16) {
        let ships = ships_of(session);
        (ships.appear, ships.arrived)
    }

    #[test]
    fn attackers_that_do_not_follow_through_a_hypergate_are_lost_for_good() {
        let catalog = gated(following(0, 0), NEVER_FOLLOWS_THROUGH_HYPERGATE);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0], "following the player");
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert!(marked(&session).is_empty(), "none at the exit");
        assert_eq!(timer(&session), (LOST_AT_GATE, 2));
        ticks(&mut session, &catalog, 300);
        assert!(marked(&session).is_empty(), "none come");
        jump_to(&mut session, &catalog, 130);
        ticks(&mut session, &catalog, 300);
        session.player.position = Vec2::new(30.0, -40.0);
        session.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        take_off(&mut session, &catalog);
        assert!(marked(&session).is_empty(), "nor at a jump or a take-off");
        assert_eq!(timer(&session), (LOST_AT_GATE, 2));
    }

    #[test]
    fn by_the_other_reading_the_exits_setup_sets_them_up_again() {
        let catalog = gated(following(0, 0), NEVER_FOLLOWS_THROUGH_HYPERGATE);
        let mut session = accepted(&catalog).with_rules(reading(RuleSource::Bible));
        take_off(&mut session, &catalog);
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert_eq!(marked(&session), [0, 0]);
        assert_eq!(timer(&session), (-1, 2), "untouched");
    }

    #[test]
    fn lost_attackers_starting_1_come_on_the_timer_the_next_jump_restarts() {
        let catalog = gated(following(0, 1), NEVER_FOLLOWS_THROUGH_HYPERGATE);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert!(marked(&session).is_empty());
        assert_eq!(
            timer(&session),
            (LOST_AT_GATE, 2),
            "the hook did not restart it"
        );
        ticks(&mut session, &catalog, 300);
        assert!(marked(&session).is_empty());
        jump_to(&mut session, &catalog, 130);
        assert_eq!(timer(&session), (199, 0), "restarted");
        ticks(&mut session, &catalog, 199);
        assert_eq!(marked(&session), [0, 0]);
    }

    #[test]
    fn by_the_other_reading_the_hook_restarts_a_slot_starting_1_at_the_exit() {
        let catalog = gated(following(0, 1), NEVER_FOLLOWS_THROUGH_HYPERGATE);
        let mut session = accepted(&catalog).with_rules(reading(RuleSource::Bible));
        take_off(&mut session, &catalog);
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert_eq!(timer(&session), (199, 0));
    }

    #[test]
    fn ships_of_their_own_behaviour_are_lost_through_a_wormhole() {
        let catalog = gated(following(-1, 0), 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        assert_eq!(marked(&session), [0, 0]);
        exit_gate(&mut session, &catalog, GateKind::Wormhole, &mut NeverFires);
        assert!(marked(&session).is_empty());
        assert_eq!(timer(&session), (LOST_AT_GATE, 2));
        ticks(&mut session, &catalog, 300);
        assert!(marked(&session).is_empty());
    }

    #[test]
    fn escorts_following_the_player_are_set_up_at_the_exit_at_once_in_formation() {
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let catalog = gated(following(1, 1), 0);
            let mut session = accepted(&catalog);
            take_off(&mut session, &catalog);
            assert!(marked(&session).is_empty(), "{kind:?}: they come later");
            exit_gate(&mut session, &catalog, kind, &mut NeverFires);
            assert_eq!(marked(&session), [0, 0], "{kind:?}");
            assert!(
                session
                    .npcs()
                    .iter()
                    .all(crate::traffic::npc::Npc::mission_escort),
                "{kind:?}: in formation"
            );
            assert_eq!(timer(&session), (-1, 0), "{kind:?}: not restarted");
        }
        let catalog = gated(following(1, 1), 0);
        let mut session = accepted(&catalog).with_rules(reading(RuleSource::Bible));
        take_off(&mut session, &catalog);
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert!(
            marked(&session).is_empty(),
            "by the other reading, on the hook's timer"
        );
        assert_eq!(timer(&session), (199, 0));
    }

    #[test]
    fn a_slot_of_a_fixed_system_is_left_to_the_hook() {
        let catalog = gated(shipping(131, 1), 0);
        let mut session = accepted(&catalog);
        take_off(&mut session, &catalog);
        if let Some(active) = session.pilot.missions.get_mut(0) {
            active.ships.appear = -1;
        }
        exit_gate(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        assert!(marked(&session).is_empty(), "none set up the hook restarts");
        assert_eq!(timer(&session), (199, 0), "restarted");
    }
}
