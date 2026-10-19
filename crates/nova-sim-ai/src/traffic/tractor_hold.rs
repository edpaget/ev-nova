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
//! before the flight's divisor and its ionization.

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

    /// Its own handling: its stats', at a third of its speed, turn and
    /// thrust while another ship holds it ([`held_handling`], its class's
    /// turn the floor's).
    #[must_use]
    pub fn own_handling(&self) -> Handling {
        if self.held() {
            held_handling(self.stats.handling, self.hull.class_turn)
        } else {
            self.stats.handling
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
}
