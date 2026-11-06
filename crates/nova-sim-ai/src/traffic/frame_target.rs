//! The target a deciding NPC's fire reads: the one the frame ends with.
//! `_AIDispatch` runs the high level and then the low level in the same
//! dispatch (@0x901e5-0x901f4), so the low level's fire reads +0x70 as
//! the high level left it. A drop that clears the target is in place
//! before the trigger (the traffic's goal step writes it); one that holds
//! it ([`Decision::holds_target`](crate::ai::Decision::holds_target), the
//! ally drop @0x8e2d2) is handed to the trigger here, so sub 0's
//! `_AIEscortFireUnprovoked` (@0x8521e-0x85236) fires at it, and drops it
//! in the same dispatch when it is down (+0x70 = -1, @0x80e9d).
//!
//! Order 0x14, the mine's fire at asteroid slot 0, writes +0x70 -1 on each
//! frame it fires (@0x8843a), before it calls `_AIFireGunUntargeted`, for
//! any type: so a [`Goal::Mine`] frame that fires at slot 0
//! ([`fire_untargeted::fires_at_rock`]) ends with no target, whatever the
//! target step or the decision's word. The trigger is worked out on a
//! deciding tick alone, as the mine's fire is, so this -1 is too.

use crate::ai::{Behaviour, Goal, Surroundings, fire_untargeted};
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::traffic::npc::Npc;

/// What `npc`, deciding this tick, fires among `around` as `behaviour`
/// says, and the target the frame ends with, its decision's last word
/// being `last_word` ([`Decision::last_target`](crate::ai::Decision::last_target)):
/// a cleared target stands, and a held one unless, on a goal-0 frame, sub
/// 0's unprovoked fire drops it ([`gate_wait`](Behaviour::gate_wait)'s,
/// which that fire is); with neither, the target step's answer. A target
/// step that works its answer out afresh (a defender's think, a mission
/// hunt) would refill a cleared or held one. A mining frame that fires at
/// slot 0 ends with none (see the module docs).
// `Decision::last_target`'s own shape: none for no word, then the word.
#[allow(clippy::option_option)]
pub(crate) fn decided_command(
    behaviour: &(impl Behaviour + ?Sized),
    npc: &Npc,
    last_word: Option<Option<ShipRef>>,
    around: &Surroundings,
) -> (Trigger, Option<ShipRef>) {
    let (trigger, target) = stepped(behaviour, npc, last_word, around);
    let fires_at_rock = npc.goal == Goal::Mine && fire_untargeted::fires_at_rock(npc, around);
    (trigger, target.filter(|_| !fires_at_rock))
}

/// [`decided_command`] before order 0x14's -1: the decision's word, or the
/// target step's answer.
#[allow(clippy::option_option)]
fn stepped(
    behaviour: &(impl Behaviour + ?Sized),
    npc: &Npc,
    last_word: Option<Option<ShipRef>>,
    around: &Surroundings,
) -> (Trigger, Option<ShipRef>) {
    match last_word {
        Some(Some(held)) => {
            let framed = Npc {
                target: Some(held),
                ..npc.clone()
            };
            let dropped =
                framed.goal == Goal::Idle && behaviour.gate_wait(&framed, around).drops_target;
            (
                behaviour.trigger(&framed, around),
                (!dropped).then_some(held),
            )
        }
        Some(None) => (behaviour.trigger(npc, around), None),
        None => (
            behaviour.trigger(npc, around),
            behaviour.target(npc, around),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::n;
    use crate::chance::Chance;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use nova_sim_base::fixture::FAST;
    use nova_sim_combat::asteroid::Asteroid;

    /// A behaviour whose target step answers the target the ship holds,
    /// and which fires nothing.
    #[derive(Debug)]
    struct Holding;

    impl Behaviour for Holding {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, _: &Npc, _: &Surroundings) -> Trigger {
            Trigger::default()
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// Slot 0, at rest at the origin.
    fn rock() -> Asteroid {
        Asteroid {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 30,
            kind: 0,
        }
    }

    /// A ship in [`Goal::Mine`] at (`x`, `y`) heading `heading`, its class
    /// turn 2 (a mining box of 120), holding NPC 2.
    fn mining_at(x: f32, y: f32, heading: f32) -> Npc {
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]));
        npc.goal = Goal::Mine;
        npc.hull.class_turn = 2.0;
        npc.state.position = Vec2::new(x, y);
        npc.state.heading = heading;
        npc.target = Some(n(2));
        npc
    }

    /// The target `npc`'s frame ends with, slot 0 at the origin, its
    /// decision's last word `last_word`.
    #[allow(clippy::option_option)]
    fn ends_with(npc: &Npc, last_word: Option<Option<ShipRef>>) -> Option<ShipRef> {
        let npcs = [npc.clone()];
        let around = Surroundings {
            mining_rock: Some(rock()),
            ..Surroundings::new(&[], &npcs)
        };
        decided_command(&Holding, npc, last_word, &around).1
    }

    #[test]
    fn order_0x14_firing_at_slot_0_clears_the_target_whatever_the_step_answers() {
        // 100 off on each axis, facing the bearing 314: order 0x14 fires,
        // writing +0x70 -1 (@0x8843a).
        let firing = mining_at(100.0, 100.0, 314.0);
        assert_eq!(ends_with(&firing, None), None);
        assert_eq!(ends_with(&firing, Some(Some(n(2)))), None, "a held word");
    }

    #[test]
    fn a_mining_ship_outside_its_box_or_facing_away_keeps_its_target() {
        let chasing = mining_at(121.0, 0.0, 270.0);
        assert_eq!(ends_with(&chasing, None), Some(n(2)), "order 0x13");
        let away = mining_at(100.0, 100.0, 134.0);
        assert_eq!(ends_with(&away, None), Some(n(2)), "facing away");
        let idle = Npc {
            goal: Goal::Idle,
            ..mining_at(100.0, 100.0, 314.0)
        };
        assert_eq!(ends_with(&idle, None), Some(n(2)), "no order 0x14");
    }
}
