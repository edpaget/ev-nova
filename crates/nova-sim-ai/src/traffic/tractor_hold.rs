//! An NPC's tractor hold (see the
//! [`tractor_hold`](crate::combat::tractor_hold) module): what
//! `_HandleShip` does with its lock each frame, and the third of its
//! speed, turn and thrust while another ship holds it.
//!
//! **The frame's hold** ([`hold`], `_HandleShip` @0x33fc0-0x3413d and
//! @0x353ba-0x35449): its velocity pulled toward its holder's by the
//! world's [`tractor_step`](super::World::tractor_step), then released
//! when its holder is gone, disabled or dying, then aged and lapsed. It
//! runs for every NPC the flight reaches, on both passes, aging on the
//! advance pass alone, before the ship moves and after its ion block.
//!
//! Two ordering notes, neither crossing a boundary the player sees. The
//! hold reads its holder's velocity as the pass began, where the original
//! reads the holder's slot live, so a holder of a lower slot has already
//! moved this frame: a one-pass lag settled before the frame is drawn.
//! And the release and lapse run before the move rather than after it;
//! the pull still comes before both, so the ship is pulled on as many
//! frames as the original's.
//!
//! **The held handling** ([`Npc::own_handling`]): every reader of an
//! NPC's `_ShipMaxSpeed`, `_ShipTurnRate` and `_ShipAccelRate` reads it,
//! after its skill and its government's factor
//! ([`Npc::class_handling`], see the [`skill`](crate::traffic::skill)
//! module) and before the player's led factor, the flight's divisor and
//! its ionization.

use std::collections::BTreeMap;

use super::World;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::combat::tractor_hold::{Holder, held_by_other, held_handling, npc_pass};
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
        let there = |velocity, condition| Holder::There {
            velocity,
            intact: condition == Condition::Intact,
        };
        Self {
            player: world
                .player
                .map(|player| there(player.state.velocity, player.condition)),
            npcs: npcs
                .iter()
                .map(|npc| (npc.id, there(npc.state.velocity, npc.condition)))
                .collect(),
        }
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

/// `npc`'s frame's hold among `holders` at the pull's `step`, aging on an
/// `advance` pass alone (see the module docs).
pub fn hold(npc: &mut Npc, holders: &Holders, step: f32, advance: bool) {
    let me = ShipRef::Npc(npc.id);
    npc_pass(
        &mut npc.tractor,
        me,
        &mut npc.state.velocity,
        |ship| holders.holder(ship),
        step,
        advance,
    );
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
    use nova_sim_base::fixture::{Draws, FAST};

    const HOLDER: NpcId = NpcId(2);

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
