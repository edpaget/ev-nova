//! Where the player's escorts arrive as it enters a system, as
//! `_SetupShipsInSystem` (@0x42b61) places them, the engine reading of
//! [`RuleKey::EscortArrival`] (the findings, with addresses, are in
//! [`escort_arrival`](crate::escort_arrival)); see
//! [`Session::place_entering_escorts`].
//!
//! - **The provisional snap** ([`Session::snap_kept_escorts`],
//!   @0x42c8e-0x42ca1): every entry first snaps the fleet's kept escorts,
//!   the hired escorts and fighters, onto a formation numbered and spaced
//!   without the mission escorts, which `_SetupShipsInSystem` sets up only
//!   after it.
//! - **The arrival push** ([`Space::push_arriving_escorts`],
//!   @0x42ca6-0x42d9e): on a jump's arrival alone, each kept escort is
//!   pushed 891.7344 back along its heading, gains 50 along it, and
//!   glides in ([`Mode::JumpingIn`]) as a ship arriving by hyperspace
//!   does, to the lower of its own top speed and the player's, then flies
//!   back to formation. A gate exit leaves them where the snap put them,
//!   for the `gate_escorts` module to lead out of the gate.
//! - **The re-snap** ([`Session::resnap_led_positions`],
//!   @0x4319a-0x431b2): an entry by no way in that names an entrance (a
//!   take-off, a pilot flown, the rescue) moves every ship the player
//!   leads onto its final slot, counted with the mission escorts. A hired
//!   escort or fighter takes the player's state, as `_RespawnEscort`
//!   already gave it the player's velocity; a mission escort takes the
//!   slot's position and the player's heading (@0x430e4-0x430ef) and
//!   keeps its own velocity and speed.
//!
//! None of it plays a sound. By the other reading every ship the player
//! leads is snapped onto its final slot with the player's state.

use std::collections::BTreeSet;

use super::entry::Entry;
use super::{Session, Space};
use crate::ai::swarm::{self, FormationParent};
use crate::escort;
use crate::escort_arrival::pushed;
use crate::flight::ShipState;
use crate::rulebook::{RuleKey, RuleSource};
use crate::traffic::npc::{Mode, NpcId};

impl Session {
    /// Places the fleet's escorts, just entered the system `entry`'s way
    /// (none for the start and a take-off), as the
    /// [`RuleKey::EscortArrival`] source says (see the module docs): by
    /// the engine the provisional snap, then on a jump's arrival the push,
    /// and by a way in that names no entrance ([`Entry::entrance`]) the
    /// re-snap. By the other reading every escort is snapped onto its
    /// final slot ([`Session::snap`]), and by a way in that names no
    /// entrance every mission escort too
    /// ([`Session::snap_mission_escorts`]).
    pub(super) fn place_entering_escorts(&mut self, entry: Option<Entry>) {
        let resnap = entry.and_then(Entry::entrance).is_none();
        if self.setup.rules.source_for(RuleKey::EscortArrival) != RuleSource::Engine {
            self.reform();
            for id in self.space.escorts.fleet.clone().into_iter().flatten() {
                self.snap(id);
            }
            if resnap {
                self.snap_mission_escorts();
            }
            return;
        }
        self.snap_kept_escorts();
        if matches!(entry, Some(Entry::Jump { .. })) {
            self.space.push_arriving_escorts();
        }
        if resnap {
            self.resnap_led_positions();
        }
    }

    /// The provisional snap (see the module docs): each of the fleet's
    /// escorts whose formation parent is the player is put on its slot of
    /// a formation of them alone and the player, with the player's state,
    /// then the formation re-forms ([`Session::reform`]) so each escort's
    /// duty holds its final slot, counted with the mission escorts, as the
    /// original recomputes the formation every frame.
    ///
    /// The engine sets the mission ships up before the escorts, where the
    /// original does it after. Nothing in the mission ships' setup reads
    /// an escort's place and both settle within the entry, before any
    /// frame is drawn, so the order is not filed.
    pub(super) fn snap_kept_escorts(&mut self) {
        let fleet: BTreeSet<NpcId> = self.space.escorts.fleet.iter().flatten().copied().collect();
        let kept = self.player_led().filter(|npc| {
            fleet.contains(&npc.id) && swarm::formation_parent(npc) == Some(FormationParent::Player)
        });
        let slots = escort::number_formation(self.ship.fit.hull.sprite_size, kept);
        let player = self.ship.player;
        for (id, slot) in slots {
            if let Some(npc) = self.space.npc_mut(id) {
                npc.state = ShipState {
                    position: slot.position(&player),
                    ..player
                };
            }
        }
        self.reform();
    }

    /// The re-snap (see the module docs): every ship the player leads
    /// whose formation parent is the player goes onto its slot, which
    /// [`Session::snap_kept_escorts`] has re-formed: a mission escort by
    /// position, with the player's heading, keeping its velocity and
    /// speed; any other with the player's state ([`Session::snap`]).
    pub(super) fn resnap_led_positions(&mut self) {
        let player = self.ship.player;
        for (id, slot) in self.formation_slots() {
            let Some(npc) = self.space.npc_mut(id) else {
                continue;
            };
            if npc.mission_escort() {
                npc.state.position = slot.position(&player);
                npc.state.heading = player.heading;
            } else {
                self.snap(id);
            }
        }
    }
}

impl Space {
    /// The arrival push (see the module docs): each of the fleet's
    /// escorts in the system is pushed back for its glide in
    /// ([`pushed`]) and starts it. Its warp-out stamp is already clear,
    /// as each entering escort is a fresh NPC.
    pub(super) fn push_arriving_escorts(&mut self) {
        for id in self.escorts.fleet.clone().into_iter().flatten() {
            if let Some(npc) = self.npc_mut(id) {
                npc.state = pushed(npc.state);
                npc.mode = Mode::JumpingIn { tick: 0 };
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{HullRecord, ShipId, SystemId};
    use crate::chance::NeverFires;
    use crate::escort::{FormationSlot, KEEP_FORMATION};
    use crate::gate::{GateArrivalRule, GateKind};
    use crate::geometry::Vec2;
    use crate::pilot::Escort;
    use crate::reserves::Reserves;
    use crate::rulebook::Rulebook;
    use crate::session::gate_follow::tests::{exit_gate_raising, following, gated};
    use crate::session::mission_ships::tests::{accepted, jump_to, take_off, tick};
    use crate::testkit::{FAST, FakePilotCatalog, hull, ship};
    use crate::traffic::npc::Npc;
    use crate::traffic::spawn::glide_speed;

    /// The hired escort's ship type, of no sprite.
    const HIRED: ShipId = ShipId(202);
    /// How wide the mission escort's ship type, 200, is (its hull's 100):
    /// wider than the player's and the hired escort's, so it widens the
    /// formation.
    const WIDE: f32 = 100.0;

    /// A hired escort of [`HIRED`].
    fn hired() -> Escort {
        Escort {
            ship: HIRED,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// A catalog whose mission 140 has one ship, of ship 200 [`WIDE`]
    /// pixels across, that escorts the player (`ShipBehav` 1, `ShipSyst`
    /// -6), from the start; the hired escort's [`HIRED`] has no sprite.
    fn catalog() -> FakePilotCatalog {
        let mut catalog = gated(following(1, 0), 0);
        catalog.missions[0].ships.count = 1;
        catalog.ship_records.push(ship(HIRED.0, FAST));
        catalog.hulls.push(HullRecord {
            size: Some(100),
            ..hull(200)
        });
        catalog
    }

    /// In flight in 130 with a hired escort and mission 140's escort,
    /// under `rules`, the hired escort taken on before the take-off: it
    /// takes the lower berth.
    fn flying(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = accepted(catalog).with_rules(rules);
        session.pilot.escorts.push(hired());
        take_off(&mut session, catalog);
        session
    }

    /// In flight in 130 under `rules`, the hired escort taken on after
    /// the take-off, so the mission escort takes the lower berth, and
    /// keeps it through a jump.
    fn hired_in_flight(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = accepted(catalog).with_rules(rules);
        take_off(&mut session, catalog);
        session.pilot.escorts.push(hired());
        session.space.escorts.fleet = vec![session.place_escort(0, &mut NeverFires)];
        session.reform();
        session
    }

    /// The other reading of `escort_arrival`.
    fn other() -> Rulebook {
        Rulebook::default().with_override(RuleKey::EscortArrival, RuleSource::Bible)
    }

    /// The hired escort's NPC.
    fn hired_npc(session: &Session) -> Npc {
        let id = session.space.escorts.fleet[0].expect("placed");
        session.space.npc(id).expect("here").clone()
    }

    /// The mission escort's NPC.
    fn mission_npc(session: &Session) -> Npc {
        session
            .player_led()
            .find(|npc| npc.mission_escort())
            .expect("a mission escort")
            .clone()
    }

    /// Slot `slot` of the player's formation of the hired escort alone.
    fn alone(session: &Session) -> FormationSlot {
        FormationSlot {
            slot: 2,
            ships: 2,
            spacing: escort::spacing(session.ship.fit.hull.sprite_size, &[None]),
        }
    }

    /// Slot `slot` of the player's formation of both escorts.
    fn both(session: &Session, slot: u8) -> FormationSlot {
        FormationSlot {
            slot,
            ships: 3,
            spacing: escort::spacing(session.ship.fit.hull.sprite_size, &[None, Some(WIDE)]),
        }
    }

    /// The player's state at `slot`'s position.
    fn at(session: &Session, slot: FormationSlot) -> ShipState {
        let player = session.ship.player;
        ShipState {
            position: slot.position(&player),
            ..player
        }
    }

    /// From [`hired_in_flight`], jumps from 130 to 131, stopping at the
    /// arrival.
    fn jumped(rules: Rulebook) -> (FakePilotCatalog, Session) {
        let catalog = catalog();
        let session = jumped_in(&catalog, rules);
        (catalog, session)
    }

    /// [`jumped`], of `catalog`.
    fn jumped_in(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        let mut session = hired_in_flight(catalog, rules);
        jump_to(&mut session, catalog, 131);
        assert_eq!(session.pilot.system, SystemId(131));
        let (hired, mission) = (hired_npc(&session), mission_npc(&session));
        assert!(mission.berth < hired.berth, "the mission escort first");
        session
    }

    #[test]
    fn the_two_formations_put_the_hired_escort_in_different_places() {
        let catalog = catalog();
        let session = flying(&catalog, Rulebook::default());
        assert_ne!(alone(&session).spacing, both(&session, 2).spacing);
        assert_ne!(
            at(&session, alone(&session)),
            at(&session, both(&session, 2))
        );
        assert_ne!(
            at(&session, alone(&session)),
            at(&session, both(&session, 3))
        );
    }

    #[test]
    fn on_a_jumps_arrival_the_kept_escort_is_snapped_to_a_formation_counted_without_the_mission_escorts()
     {
        let (_, session) = jumped(Rulebook::default());
        let hired = hired_npc(&session);
        assert_eq!(hired.state, pushed(at(&session, alone(&session))));
        assert_eq!(
            hired.formation(),
            Some(both(&session, 3)),
            "its duty on its final slot"
        );
        assert_eq!(mission_npc(&session).formation(), Some(both(&session, 2)));
    }

    #[test]
    fn on_a_jumps_arrival_the_kept_escort_is_pushed_back_and_glides_in() {
        let (catalog, mut session) = jumped(Rulebook::default());
        let player = session.ship.player;
        let hired = hired_npc(&session);
        let heading = crate::trig::whole(player.heading);
        let snapped = at(&session, alone(&session));
        let back = snapped.position - hired.state.position;
        assert!((back.length() - 891.734_4).abs() < 1e-3, "{back:?}");
        assert!(
            (back - crate::trig::accel(heading, 891.734_4)).length() < 1e-3,
            "back along the heading: {back:?}"
        );
        assert_eq!(
            hired.state.velocity,
            player.velocity + crate::trig::accel(heading, 50.0)
        );
        assert_eq!(hired.state.heading, player.heading);
        assert_eq!(hired.mode, Mode::JumpingIn { tick: 0 });
        let mission = mission_npc(&session);
        assert_eq!(mission.mode, Mode::Flying, "not pushed");

        // The escort and the player both fly `FAST`, led at the same 1.5
        // times: the lower of the two is the player's 9. From 50, 1.165
        // slower a tick, the glide ends on tick 35, at 9.225.
        assert!((session.top_speed() - 9.0).abs() < 1e-4);
        assert_eq!(glides_in(&mut session, &catalog), 35);
        assert_eq!(
            hired_npc(&session).goal,
            crate::ai::Goal::Formation { guard: None }
        );
        let off = |session: &Session| {
            let npc = hired_npc(session);
            let off = npc
                .formation()
                .expect("a slot")
                .position(&session.ship.player)
                - npc.state.position;
            off.x.abs().max(off.y.abs())
        };
        for _ in 0..600 {
            tick(&mut session, &catalog);
        }
        assert!(off(&session) <= KEEP_FORMATION, "{}", off(&session));
    }

    /// Ticks the traffic, the player held still, while the hired escort
    /// glides in: each tick at [`glide_speed`] along its heading, until its
    /// next tick's speed is at or below the lower of its own top speed and
    /// the player's. The tick it ends on.
    fn glides_in(session: &mut Session, catalog: &FakePilotCatalog) -> u32 {
        session.ship.player.velocity = Vec2::ZERO;
        let cap = hired_npc(session)
            .stats
            .handling
            .max_speed
            .min(session.top_speed() / 1.5)
            * 1.5;
        for k in 0..60 {
            tick(session, catalog);
            let npc = hired_npc(session);
            let speed = npc.state.velocity.length();
            assert!((speed - glide_speed(k)).abs() < 1e-3, "tick {k}: {speed}");
            assert!(
                (npc.state.velocity - crate::flight::facing(npc.state.heading) * speed).length()
                    < 1e-3,
                "along its heading"
            );
            if npc.mode == Mode::Flying {
                assert!(glide_speed(k + 1) <= cap, "tick {k}");
                return k;
            }
            assert_eq!(npc.mode, Mode::JumpingIn { tick: k + 1 });
            assert!(glide_speed(k + 1) > cap, "tick {k}");
        }
        panic!("the glide never ended");
    }

    #[test]
    fn a_slower_escort_ends_its_glide_at_its_own_top_speed() {
        // The hired escort 4 a tick, 6 led, to the player's 9: from 50,
        // 1.165 slower a tick, its glide ends on tick 37, at 6.895.
        let mut catalog = catalog();
        let record = catalog
            .ship_records
            .iter_mut()
            .find(|record| record.id == HIRED)
            .expect("the hired escort's");
        *record = ship(HIRED.0, crate::handling::ShipFields { speed: 400, ..FAST });
        let mut session = jumped_in(&catalog, Rulebook::default());
        assert!((session.top_speed() - 9.0).abs() < 1e-4);
        assert_eq!(glides_in(&mut session, &catalog), 37);
    }

    #[test]
    fn a_take_off_resnaps_every_led_ship_onto_its_final_slot_a_mission_escort_by_position() {
        let catalog = catalog();
        let session = flying(&catalog, Rulebook::default());
        let player = session.ship.player;
        let hired = hired_npc(&session);
        let mission = mission_npc(&session);
        assert!(hired.berth < mission.berth);
        assert_eq!(hired.formation(), Some(both(&session, 2)));
        assert_eq!(hired.state, at(&session, both(&session, 2)));
        assert_eq!(hired.mode, Mode::Flying);
        assert_eq!(mission.formation(), Some(both(&session, 3)));
        assert_eq!(mission.state.position, both(&session, 3).position(&player));
        assert_eq!(mission.state.heading, player.heading);
        assert_eq!(mission.mode, Mode::Flying);
    }

    #[test]
    fn the_resnap_keeps_a_mission_escorts_own_velocity_and_speed() {
        let catalog = catalog();
        let mut session = flying(&catalog, Rulebook::default());
        let id = mission_npc(&session).id;
        let npc = session.space.npc_mut(id).expect("here");
        npc.state.velocity = Vec2::new(1.5, -2.5);
        npc.state.heading = 33.0;
        npc.speed = 2.0;
        session.resnap_led_positions();
        let mission = mission_npc(&session);
        assert_eq!(mission.state.velocity, Vec2::new(1.5, -2.5));
        assert_eq!(mission.speed, 2.0);
        assert_eq!(mission.state.heading, session.ship.player.heading);
        assert_eq!(
            mission.state.position,
            both(&session, 3).position(&session.ship.player)
        );
    }

    #[test]
    fn a_gate_exit_pushes_no_escort() {
        let catalog = catalog();
        let mut session = hired_in_flight(&catalog, Rulebook::default());
        exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        let npc = hired_npc(&session);
        assert!(npc.emerging(), "led out of the gate: {:?}", npc.mode);
        // Like a jump, it stays on its provisional slot.
        let mut session = hired_in_flight(&catalog, Rulebook::default())
            .with_gate_arrival(GateArrivalRule::LikeJump);
        exit_gate_raising(&mut session, &catalog, GateKind::Hypergate, &mut NeverFires);
        let npc = hired_npc(&session);
        assert_eq!(npc.mode, Mode::Flying, "like a jump");
        assert_eq!(npc.state, at(&session, alone(&session)));
    }

    #[test]
    fn a_loaded_pilots_escorts_start_on_their_final_slots() {
        let catalog = catalog();
        let mut pilot = crate::pilot::Pilot::new(&catalog, "Ada").expect("starts");
        pilot.escorts = vec![hired(), hired()];
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        tick(&mut session, &catalog);
        let fleet: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        assert_eq!(fleet.len(), 2);
        for (slot, id) in [2, 3].into_iter().zip(fleet) {
            let npc = session.space.npc(id).expect("here");
            let slot = FormationSlot {
                slot,
                ships: 3,
                spacing: escort::spacing(session.ship.fit.hull.sprite_size, &[None, None]),
            };
            assert_eq!(npc.state, at(&session, slot));
            assert_eq!(npc.mode, Mode::Flying);
        }
    }

    #[test]
    fn by_the_other_reading_a_jumps_arrival_snaps_every_escort_onto_its_final_slot() {
        let (_, session) = jumped(other());
        let hired = hired_npc(&session);
        assert_eq!(hired.formation(), Some(both(&session, 3)));
        assert_eq!(hired.state, at(&session, both(&session, 3)));
        assert_eq!(hired.mode, Mode::Flying);
    }

    #[test]
    fn by_the_other_reading_a_take_off_gives_the_mission_escort_the_players_state() {
        let catalog = catalog();
        let mut session = flying(&catalog, other());
        let hired = hired_npc(&session);
        assert_eq!(hired.state, at(&session, both(&session, 2)));
        let mission = mission_npc(&session);
        assert_eq!(mission.state, at(&session, both(&session, 3)));
        // Its own velocity is overwritten with the player's.
        let id = mission.id;
        session.space.npc_mut(id).expect("here").state.velocity = Vec2::new(1.5, -2.5);
        session.place_entering_escorts(None);
        assert_eq!(mission_npc(&session).state, at(&session, both(&session, 3)));
    }
}
