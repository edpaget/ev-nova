//! An NPC's tractor hold (see the
//! [`tractor_hold`](crate::combat::tractor_hold) module): what
//! `_HandleShip` does with its lock each frame, and the third of its
//! speed, turn and thrust while another ship holds it.
//!
//! **The frame's hold**, for every NPC the flight reaches, on both passes:
//!
//! - **The pull** ([`pull`], `_HandleShip` @0x33fc0-0x3413d), after its
//!   ion block and before it moves: its velocity pulled toward its
//!   holder's by the world's [`tractor_step`](super::World::tractor_step).
//! - **The release and lapse** ([`release`], @0x353ba-0x35449), after its
//!   turn (@0x34460-0x3458e), thrust (@0x34a54-0x34d93), glow's rush test
//!   (@0x34f57) and fire (@0x34ffd): released when its holder is gone,
//!   disabled or dying, then aged, on the advance pass alone, and lapsed.
//!   `_ShipTurnRate`, `_ShipAccelRate` and `_ShipMaxSpeed` read the lock
//!   (+0xc8dc), so a ship released or lapsed on a pass turns, thrusts and
//!   glows held on that pass. Only the two early exits for a ship type
//!   above 0x2ff and type +0xa 0xd8f1 (@0x33594-0x335c8) skip both, and
//!   no path between them leaves `_HandleShip`.
//!
//! **A freed holder** ([`Holders::and_gone`]): the pull reads the holder's
//! slot (+0x20/+0x24, @0x3401f-0x34032) with no +0xb8 test, so a ship
//! whose holder's slot was freed is pulled once more by the slot's record,
//! then released, at its next step. Every standing record of the gone
//! ships ([`GoneShips`](super::gone::GoneShips)) is a holder so, whatever
//! freed it: a leave at its slot of the AI loop (pulled and released at
//! every slot on that pass), a deadly stellar's kill after the ship loop
//! or a swap on the hail in a dialog (at the next pass, double time's
//! second when there is one), a gate entry or a wreck at its display in
//! the ship loop (a later slot on that pass, by [`Holders::freed`]; an
//! earlier one at the next pass). The release lets go of every such lock
//! at the held ship's first step after the free, so a lock naming a gone
//! ship is always one whose step hasn't come since. A dying holder lets
//! go of every ship it holds at that ship's next step (@0x3540c), so a
//! wreck's free reaches a hold only through a lock laid after that step,
//! by a beam its firer's death doesn't end, which the record pulls and
//! releases alike.
//!
//! **A berth taken** ([`Traffic::holds_follow_berth`]): +0xc8dc is a slot
//! index, which a spawn rewrites and no free or spawn clears on another
//! ship (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), so a ship whose
//! holder's berth is taken before its step is held, and pulled, by the
//! newcomer. A launch into a berth since the pass began (an escape ship's
//! `_LaunchFighter` @0x354ea) is read as it stands ([`Holders::launched`]).
//! The player's hold on a taken berth is `_HandlePlayer`'s
//! (@0x68436-0x684b7), left out (see
//! rdm:phase/combat-ai-fidelity-cross-roadmap/phase-17-left-out-of-phase-15-handle-ship).
//!
//! The player's
//! hold releases first and isn't pulled on the frame it is released
//! (`_HandlePlayer` @0x68436-0x684b7, see
//! [`player_pass`](crate::combat::tractor_hold::player_pass)), so a
//! holder gone that way pulls the player not at all.
//!
//! The hold reads its holder's velocity as the pass began, where the
//! original reads the holder's slot live, so a holder of a lower slot has
//! already moved this frame: a one-pass lag settled before the frame is
//! drawn, crossing no boundary the player sees.
//!
//! **The held handling** ([`Npc::own_handling`]): every reader of an
//! NPC's `_ShipMaxSpeed`, `_ShipTurnRate` and `_ShipAccelRate` reads it,
//! after its skill and its government's factor
//! ([`Npc::class_handling`], see the [`skill`](crate::traffic::skill)
//! module) and before the player's led factor, the flight's divisor and
//! its ionization.

use std::collections::BTreeMap;

use super::gone::GoneShip;
use super::{Traffic, World};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::tractor_hold::{Holder, held_by_other, held_handling, npc_pull, npc_release};
use crate::handling::Handling;
use crate::traffic::npc::{Npc, NpcId};

impl Npc {
    /// Whether a ship other than itself holds it in a tractor beam
    /// ([`held_by_other`]).
    #[must_use]
    pub fn held(&self) -> bool {
        held_by_other(self.tractor, ShipRef::Npc(self.id))
    }

    /// Its own handling: its class's at its skill and its government's
    /// factor ([`Npc::class_handling`]), at a third of its speed, turn and
    /// thrust while another ship holds it ([`held_handling`], its class's
    /// turn the floor's). The speed and thrust are none below none, as the
    /// class's already are.
    #[must_use]
    pub fn own_handling(&self) -> Handling {
        let class = self.class_handling();
        if self.held() {
            held_handling(class, self.hull.class_turn)
        } else {
            class
        }
    }
}

/// The ships a hold may name, as the pass began: each one's velocity and
/// whether it is intact.
#[derive(Clone, Debug, Default)]
pub struct Holders {
    /// The player's, while it flies in the system.
    player: Option<Holder>,
    /// Each NPC's.
    npcs: BTreeMap<NpcId, Holder>,
}

impl Holders {
    /// The holders among `npcs` and the player of `world`.
    #[must_use]
    pub fn of(world: &World, npcs: &[Npc]) -> Self {
        Self {
            player: world.player.map(|player| Holder::There {
                velocity: player.state.velocity,
                intact: player.condition == Condition::Intact,
            }),
            npcs: npcs.iter().map(|npc| (npc.id, there(npc))).collect(),
        }
    }

    /// The holders, `gone` among them, the freed slots' standing records
    /// ([`GoneShips`](super::gone::GoneShips)), each by its record:
    /// `_HandleShip`'s pull reads the holder's slot (+0x20/+0x24,
    /// @0x3401f-0x34032) with no +0xb8 test, and its release tests the slot
    /// (@0x353ba-0x35449), so a ship held by one is pulled by its record,
    /// then released (see the module docs).
    #[must_use]
    pub fn and_gone(mut self, gone: &[GoneShip]) -> Self {
        for gone in gone {
            self.npcs.entry(gone.ship.id).or_insert(record(&gone.ship));
        }
        self
    }

    /// `freed`, freed at its own step of the ship loop, is read by its
    /// record from there on (see the module docs).
    pub fn freed(&mut self, freed: &Npc) {
        self.npcs.insert(freed.id, record(freed));
    }

    /// `launched`, launched into a berth since the pass began, is read as
    /// it stands, unless the pass already holds it (see the module docs).
    pub fn launched(&mut self, launched: &Npc) {
        (self.npcs.entry(launched.id)).or_insert_with(|| there(launched));
    }

    /// `ship` as a hold reads it: gone when it is not in the system.
    fn holder(&self, ship: ShipRef) -> Holder {
        match ship {
            ShipRef::Player => self.player,
            ShipRef::Npc(id) => self.npcs.get(&id).copied(),
        }
        .unwrap_or(Holder::Gone)
    }
}

/// `npc` as a hold reads it in the system: its velocity, and intact
/// unless disabled or dying.
fn there(npc: &Npc) -> Holder {
    Holder::There {
        velocity: npc.state.velocity,
        intact: npc.condition == Condition::Intact,
    }
}

/// `npc`'s freed slot as a hold reads it: its record's velocity, and
/// inactive, so the release lets go.
fn record(npc: &Npc) -> Holder {
    Holder::There {
        velocity: npc.state.velocity,
        intact: false,
    }
}

impl Traffic {
    /// Every NPC's hold by `old`, whose berth `new` takes, is held by `new`:
    /// +0xc8dc is a slot, which a spawn rewrites, leaving every other
    /// ship's (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b). This is the
    /// original's slot link, whatever the reading of the parent link
    /// ([`parent_slot`](super::parent_slot)), which is the escort port's
    /// routing. Each such hold is one whose step hasn't come since `old`'s
    /// slot was freed, as that step lets go (see the module docs).
    pub(super) fn holds_follow_berth(&mut self, old: NpcId, new: NpcId) {
        let (old, new) = (ShipRef::Npc(old), ShipRef::Npc(new));
        for lock in self.npcs.iter_mut().filter_map(|npc| npc.tractor.as_mut()) {
            if lock.holder == old {
                lock.holder = new;
            }
        }
    }
}

/// `npc`'s frame's pull among `holders` at the pull's `step` ([`npc_pull`],
/// see the module docs), before its move.
pub fn pull(npc: &mut Npc, holders: &Holders, step: f32) {
    let me = ShipRef::Npc(npc.id);
    let holder = |ship| holders.holder(ship);
    npc_pull(npc.tractor, me, &mut npc.state.velocity, holder, step);
}

/// `npc`'s frame's release among `holders` and lapse, aging on an
/// `advance` pass alone ([`npc_release`], see the module docs), after its
/// flight.
pub fn release(npc: &mut Npc, holders: &Holders, advance: bool) {
    let me = ShipRef::Npc(npc.id);
    npc_release(&mut npc.tractor, me, |ship| holders.holder(ship), advance);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::impact::TractorLock;
    use crate::flight::DISABLED_DRAG;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::Traffic;
    use crate::traffic::fixture::npc;
    use crate::traffic::gone::Freed;
    use nova_sim_base::fixture::{Draws, FAST};

    const HOLDER: NpcId = NpcId(2);
    const LAPSE: u32 = crate::combat::tractor_hold::LAPSE_TICKS;

    /// NPC `id` moving at `velocity`.
    fn moving(id: u32, velocity: Vec2) -> Npc {
        let mut ship = npc(id, ShipStats::new(FAST, &[]));
        ship.state.velocity = velocity;
        ship
    }

    /// NPC 1, disabled so it drifts, at `velocity`, held by `holder`.
    fn held_by(holder: ShipRef, velocity: Vec2) -> Npc {
        let mut ship = moving(1, velocity);
        ship.condition = Condition::Disabled;
        ship.tractor = Some(TractorLock::by(holder));
        ship
    }

    /// A traffic of `npcs`.
    fn traffic_of(npcs: Vec<Npc>) -> Traffic {
        let mut traffic = Traffic::new();
        traffic.npcs = npcs;
        traffic
    }

    /// A world of no stellars whose tractor pulls by `step`.
    fn pulling(step: f32) -> World<'static> {
        World {
            tractor_step: step,
            ..World::new(&[])
        }
    }

    /// Moves `traffic` again in `world`, as double time's second pass.
    fn again(traffic: &mut Traffic, world: &World) {
        traffic.move_in(*world, &mut Draws::of(&[]));
    }

    fn velocity(traffic: &Traffic) -> Vec2 {
        traffic.npcs[0].state.velocity
    }

    /// `velocity` as a disabled ship's drift leaves it after its move.
    fn drifted(velocity: Vec2) -> Vec2 {
        velocity * DISABLED_DRAG
    }

    #[test]
    fn a_held_npc_is_pulled_toward_its_npc_holders_velocity_while_disabled() {
        let mut traffic = traffic_of(vec![
            held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, -5.0)),
            moving(2, Vec2::new(1.0, 1.0)),
        ]);
        again(&mut traffic, &pulling(0.5));
        assert_eq!(velocity(&traffic), drifted(Vec2::new(4.5, -4.5)));
        assert!(traffic.npcs[0].tractor.is_some());
    }

    #[test]
    fn a_held_npc_is_pulled_toward_the_players_velocity() {
        let mut traffic = traffic_of(vec![held_by(ShipRef::Player, Vec2::new(5.0, 0.0))]);
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.state.velocity = Vec2::new(0.0, 2.0);
        let world = World {
            player: Some(player),
            ..pulling(0.5)
        };
        again(&mut traffic, &world);
        assert_eq!(velocity(&traffic), drifted(Vec2::new(4.5, 0.5)));
    }

    #[test]
    fn a_self_held_npc_brakes_toward_rest() {
        let mut traffic = traffic_of(vec![held_by(ShipRef::Npc(NpcId(1)), Vec2::new(5.0, -0.25))]);
        again(&mut traffic, &pulling(0.5));
        assert_eq!(velocity(&traffic), drifted(Vec2::new(4.5, 0.0)));
    }

    #[test]
    fn a_ship_held_by_a_holder_that_left_at_its_slot_is_pulled_by_its_record_then_released() {
        // `_HandleShip`'s pull reads the holder's slot (+0x20/+0x24,
        // @0x3401f-0x34032) with no +0xb8 test, after the whole AI loop,
        // and its release tests the slot later (@0x353ba-0x35449): every
        // ship held by a ship that left in the AI loop, at any slot, is
        // pulled once by the record, then let go.
        for berth in [1, 5] {
            let mut held = held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0));
            held.berth = berth;
            let mut traffic = traffic_of(vec![held]);
            let mut holder = moving(2, Vec2::new(1.0, 0.0));
            holder.berth = 3;
            // Recorded at its slot, as `dispatch_leave` records a leaver.
            traffic.left_the_system(holder.clone(), Freed::InDispatch);
            traffic.left_in_dispatch.push(holder);
            traffic.fly_holding(
                pulling(0.5),
                &std::collections::BTreeSet::new(),
                &mut Draws::of(&[]),
            );
            assert_eq!(velocity(&traffic), drifted(Vec2::new(4.5, 0.0)), "{berth}");
            assert_eq!(traffic.npcs[0].tractor, None, "{berth}: let go");
            // Double time's second pass has no AI loop: the advance pass
            // let go, so it pulls nothing.
            again(&mut traffic, &pulling(0.5));
            assert_eq!(
                velocity(&traffic),
                drifted(drifted(Vec2::new(4.5, 0.0))),
                "{berth}"
            );
        }
    }

    #[test]
    fn a_held_npc_is_released_when_its_holder_leaves_or_is_disabled_or_dying() {
        let gone = traffic_of(vec![held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0))]);
        let mut disabled = moving(2, Vec2::ZERO);
        disabled.condition = Condition::Disabled;
        let mut dying = moving(2, Vec2::ZERO);
        dying.condition = Condition::Dying {
            ticks_left: 30,
            starting: false,
        };
        for (mut traffic, pulled) in [
            (gone, 5.0),
            (
                traffic_of(vec![
                    held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
                    disabled,
                ]),
                4.5,
            ),
            (
                traffic_of(vec![
                    held_by(ShipRef::Npc(HOLDER), Vec2::new(5.0, 0.0)),
                    dying,
                ]),
                4.5,
            ),
        ] {
            again(&mut traffic, &pulling(0.5));
            assert_eq!(traffic.npcs[0].tractor, None);
            assert_eq!(velocity(&traffic).x, pulled * DISABLED_DRAG);
        }
    }

    #[test]
    fn an_npc_held_by_the_player_away_is_released_and_not_pulled() {
        let mut traffic = traffic_of(vec![held_by(ShipRef::Player, Vec2::new(5.0, 0.0))]);
        again(&mut traffic, &pulling(0.5));
        assert_eq!(traffic.npcs[0].tractor, None);
        assert_eq!(velocity(&traffic), drifted(Vec2::new(5.0, 0.0)));
    }

    #[test]
    fn a_hold_lapses_15_advance_passes_after_its_touch_still_pulled_on_the_15th() {
        #[derive(Debug)]
        struct Keep;
        impl crate::ai::Behaviour for Keep {
            fn decide(
                &self,
                npc: &Npc,
                _around: &crate::ai::Surroundings,
                _chance: &mut dyn crate::chance::Chance,
            ) -> crate::ai::Goal {
                npc.goal
            }
        }
        let mut traffic = traffic_of(vec![
            held_by(ShipRef::Npc(HOLDER), Vec2::new(20.0, 0.0)),
            moving(2, Vec2::ZERO),
        ]);
        for pass in 1..=14 {
            traffic.tick_in(&Keep, pulling(0.5), &[], &mut Draws::of(&[]));
            again(&mut traffic, &pulling(0.5));
            let lock = traffic.npcs[0].tractor.expect("held");
            assert_eq!(
                lock.ticks_since_touch, pass,
                "aged on the advance pass alone"
            );
        }
        let before = velocity(&traffic);
        traffic.tick_in(&Keep, pulling(0.5), &[], &mut Draws::of(&[]));
        assert_eq!(
            velocity(&traffic),
            drifted(Vec2::new(before.x - 0.5, 0.0)),
            "pulled on the 15th"
        );
        assert_eq!(traffic.npcs[0].tractor, None, "lapsed");
        let before = velocity(&traffic);
        traffic.tick_in(&Keep, pulling(0.5), &[], &mut Draws::of(&[]));
        assert_eq!(velocity(&traffic), drifted(before), "no lock, no pull");
    }

    // The held handling's readers.

    /// NPC 1 of `FAST`, its class turning 3° a tick, held by `holder`.
    fn holding(holder: Option<ShipRef>) -> Npc {
        let mut ship = moving(1, Vec2::ZERO);
        ship.hull.class_turn = 3.0;
        ship.tractor = holder.map(TractorLock::by);
        ship
    }

    fn third(value: f32) -> f32 {
        (f64::from(value) * 0.333) as f32
    }

    fn sixth(value: f32) -> f32 {
        (f64::from(value) * 0.1665) as f32
    }

    #[test]
    fn a_held_npcs_own_handling_is_a_third_of_its_speed_and_turn_and_a_sixth_of_its_thrust() {
        let plain = holding(None);
        let own = plain.stats.handling;
        let held = holding(Some(ShipRef::Npc(HOLDER)));
        assert!(held.held());
        assert_eq!(
            held.own_handling(),
            Handling {
                max_speed: third(own.max_speed),
                turn_rate: 1.0,
                accel: sixth(own.accel),
            },
            "3° a tick a third is under the floor"
        );
        let self_held = holding(Some(ShipRef::Npc(NpcId(1))));
        assert!(!self_held.held());
        assert_eq!(self_held.own_handling(), own);
        assert_eq!(plain.own_handling(), own);
    }

    #[test]
    fn a_held_npcs_top_speed_is_a_third_of_its_own() {
        use crate::traffic::autopilot::TopSpeeds;
        let plain = holding(None);
        let held = holding(Some(ShipRef::Player));
        let own = plain.stats.handling.max_speed;
        assert_eq!(held.top_speed(1.5), third(own));
        assert_eq!(TopSpeeds::own_of(&held).own, third(own));
        let self_held = holding(Some(ShipRef::Npc(NpcId(1))));
        assert_eq!(self_held.top_speed(1.5), own);
        assert_eq!(TopSpeeds::own_of(&self_held).own, own);
    }

    #[test]
    fn a_held_npc_turns_and_thrusts_slowed_before_its_ionization() {
        use crate::flight::FlightRules;
        let mut held = holding(Some(ShipRef::Player));
        held.stats.handling.turn_rate = 9.0;
        held.stats.ion.class_capacity = 400.0;
        held.ion.charge = 200.0;
        let own = held.stats.handling;
        let slowed = held.ionized_handling();
        assert_eq!(slowed.turn_rate, third(9.0) * 0.5);
        assert_eq!(slowed.accel, (f64::from(sixth(own.accel)) * 0.5) as f32);
        assert_eq!(held.turn_rate(), slowed.turn_rate);
        let divided = FlightRules {
            thrust_divisor: core::num::NonZeroU32::new(2500).expect("not 0"),
            ..FlightRules::default()
        };
        let flying = held.flying_handling(1.0, divided);
        let thrust = f64::from(sixth(own.accel)) * 2.0;
        assert_eq!(flying.accel, (f64::from(thrust as f32) * 0.5) as f32);
        assert_eq!(flying.turn_rate, slowed.turn_rate);
    }

    #[test]
    fn a_held_npc_flown_by_the_autopilot_thrusts_a_sixth_and_turns_a_third() {
        use crate::flight::FlightRules;
        use crate::traffic::autopilot::{TopSpeeds, fly};
        let flown = |holder: Option<ShipRef>, heading: f32| {
            let mut ship = holding(holder);
            ship.stats.handling.turn_rate = 9.0;
            ship.state.position = Vec2::new(0.0, -100.0);
            ship.state.heading = heading;
            ship.goal = crate::ai::Goal::JumpOut;
            let speeds = TopSpeeds::own_of(&ship);
            fly(
                &mut ship,
                &[],
                &BTreeMap::new(),
                None,
                false,
                crate::traffic::autopilot::Beside::default(),
                speeds,
                FlightRules::default(),
                crate::traffic::autopilot::Readings::default(),
                &crate::ai::NO_WEAPONS,
                &mut nova_sim_base::NeverFires,
            );
            let turned = crate::flight::shortest_turn(heading, ship.state.heading).abs();
            (ship.state.velocity.length(), turned)
        };
        // Running out up the screen, away from the centre.
        let other = Some(ShipRef::Npc(HOLDER));
        let (plain_speed, _) = flown(None, 0.0);
        let (held_speed, _) = flown(other, 0.0);
        assert!(plain_speed > 0.0);
        assert!(
            (held_speed - plain_speed * 0.1665).abs() < 1e-6,
            "{held_speed} of {plain_speed}"
        );
        let (_, plain_turn) = flown(None, 90.0);
        let (_, held_turn) = flown(other, 90.0);
        assert_eq!(plain_turn, 9.0);
        assert!((held_turn - 9.0 * 0.333).abs() < 1e-4, "{held_turn}");
        let (self_speed, _) = flown(Some(ShipRef::Npc(NpcId(1))), 0.0);
        assert_eq!(self_speed, plain_speed, "a self-held ship isn't slowed");
    }

    /// NPC 1 running out up the screen from (0, -100), heading `heading`,
    /// at rest, its class turning 9° a tick, held by `holder` if any; NPC 2
    /// at rest, intact or disabled as `holder_disabled` says.
    fn running_out(holder: Option<ShipRef>, heading: f32, holder_disabled: bool) -> Traffic {
        let mut ship = holding(holder);
        ship.stats.handling.turn_rate = 9.0;
        ship.state.position = Vec2::new(0.0, -100.0);
        ship.state.heading = heading;
        ship.goal = crate::ai::Goal::JumpOut;
        let mut other = moving(2, Vec2::ZERO);
        other.state.position = Vec2::new(500.0, 500.0);
        if holder_disabled {
            other.condition = Condition::Disabled;
        }
        traffic_of(vec![ship, other])
    }

    /// NPC 1's speed and the degrees it turned after `fly` flew `traffic`.
    fn speed_and_turn(mut traffic: Traffic, fly: impl Fn(&mut Traffic)) -> (f32, f32, bool) {
        let heading = traffic.npcs[0].state.heading;
        fly(&mut traffic);
        let ship = &traffic.npcs[0];
        let turned = crate::flight::shortest_turn(heading, ship.state.heading).abs();
        (ship.state.velocity.length(), turned, ship.tractor.is_some())
    }

    #[test]
    fn a_ship_released_on_a_pass_turns_and_thrusts_held_on_that_pass() {
        // `_HandleShip` releases (@0x353ba-0x35421) after its turn
        // (@0x34460-0x3458e) and thrust (@0x34a54-0x34d93), whose
        // `_ShipTurnRate` and `_ShipAccelRate` read +0xc8dc: so the pass
        // that lets go of a disabled holder still flies held.
        let other = Some(ShipRef::Npc(HOLDER));
        for fly in [
            (|traffic: &mut Traffic| again(traffic, &pulling(0.5))) as fn(&mut Traffic),
            |traffic| {
                traffic.fly_holding(
                    pulling(0.5),
                    &std::collections::BTreeSet::new(),
                    &mut Draws::of(&[]),
                );
            },
        ] {
            let (plain_speed, ..) = speed_and_turn(running_out(None, 0.0, true), fly);
            let (held_speed, _, kept) = speed_and_turn(running_out(other, 0.0, true), fly);
            assert!(plain_speed > 0.0);
            assert!(
                (held_speed - plain_speed * 0.1665).abs() < 1e-6,
                "{held_speed} of {plain_speed}"
            );
            assert!(!kept, "released after the flight");
            let (_, plain_turn, _) = speed_and_turn(running_out(None, 90.0, true), fly);
            let (_, held_turn, _) = speed_and_turn(running_out(other, 90.0, true), fly);
            assert_eq!(plain_turn, 9.0);
            assert!((held_turn - 9.0 * 0.333).abs() < 1e-4, "{held_turn}");
        }
    }

    #[test]
    fn a_docking_ship_released_on_a_pass_turns_held_on_that_pass() {
        // The docking flight turns by `_ShipTurnRate` at `_HandleShip`'s
        // step (@0x344b0), which still reads the lock the release lets go
        // of after it (@0x353ba-0x35421).
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let turned = |holder: Option<ShipRef>| {
            let mut traffic = running_out(holder, 90.0, true);
            traffic.npcs[0].goal = crate::ai::Goal::Idle;
            traffic.npcs[0].docking = Some(Docking {
                ship: ShipRef::Npc(NpcId(3)),
                sub: DockSub::Approach,
            });
            let mut victim = moving(3, Vec2::ZERO);
            victim.state.position = Vec2::new(0.0, -2100.0);
            traffic.npcs.push(victim);
            traffic.dock_in_dispatch(0, &pulling(0.5), &mut crate::chance::NeverFires);
            speed_and_turn(traffic, |traffic| again(traffic, &pulling(0.5)))
        };
        let (_, plain, _) = turned(None);
        let (_, held, kept) = turned(Some(ShipRef::Npc(HOLDER)));
        assert_eq!(plain, 9.0);
        assert!((held - 9.0 * 0.333).abs() < 1e-4, "{held}");
        assert!(!kept, "released after the flight");
    }

    #[test]
    fn a_lapsed_hold_still_slows_the_pass_it_lapses_on() {
        // The lapse (@0x3542b-0x35440) comes after the thrust too.
        let lapsing = |holder| {
            let mut traffic = running_out(holder, 0.0, false);
            if let Some(lock) = traffic.npcs[0].tractor.as_mut() {
                lock.ticks_since_touch = LAPSE - 1;
            }
            traffic
        };
        let advance = |traffic: &mut Traffic| {
            traffic.fly_holding(
                pulling(0.5),
                &std::collections::BTreeSet::new(),
                &mut Draws::of(&[]),
            );
        };
        let (plain_speed, ..) = speed_and_turn(lapsing(None), advance);
        let (held_speed, _, kept) = speed_and_turn(lapsing(Some(ShipRef::Npc(HOLDER))), advance);
        assert!(!kept, "lapsed");
        assert!(
            (held_speed - plain_speed * 0.1665).abs() < 1e-6,
            "{held_speed} of {plain_speed}"
        );
    }

    #[test]
    fn a_held_npcs_rush_is_measured_against_its_held_thrust() {
        use crate::flight::{Controls, FlightRules};
        use crate::traffic::ship_display::{Thrust, record};
        let flight = FlightRules::default();
        let mut ship = holding(Some(ShipRef::Player));
        let own = flight.handling(ship.own_handling());
        // A rush, 2.75 times its held thrust: well under its unheld one.
        let rush = Handling {
            accel: own.accel * 2.75,
            ..own
        };
        let thrust = Controls {
            thrust: true,
            ..Controls::default()
        };
        record(&mut ship, &rush, thrust, flight);
        assert_eq!(ship.display.drive.thrust, Thrust::Rush);
        record(&mut ship, &own, thrust, flight);
        assert_eq!(ship.display.drive.thrust, Thrust::Plain);
    }
}
