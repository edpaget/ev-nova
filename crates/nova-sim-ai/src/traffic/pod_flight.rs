//! An escape pod NPC: a ship of class 0x2ff ([`ESCAPE_POD_SHIP`]), flown
//! as `_AIDispatch` flies one (@0x8fb62-0x8fbba) in place of the rest of
//! the AI (`jmp 0x901f9`).
//!
//! - **It decides nothing**: no goal, fire command or target is taken on
//!   the AI timer (see [`Traffic::tick_in`](super::Traffic::tick_in)'s
//!   step 4).
//! - **It flies at full thrust along its heading** ([`controls`]):
//!   `_AIDispatch` sets its thrust to `_ShipAccelRate` (+0x30), its top
//!   speed to `_ShipMaxSpeed` (+0x34) and the heading it steers to to its
//!   own, truncated (+0x68 = `trunc(+0x44)`), so it never turns by more
//!   than the fraction of a degree. It flies by the same flight step as
//!   every NPC, so the thrust divisor, the per-axis caps and an
//!   inertialess hull's flight all apply.
//! - **It counts its wait down and leaves silently** ([`count_down`]): on
//!   the advance pass, as `_AIDispatch` runs only there, its AI wait
//!   (+0x4c, [`Mode::Adrift`]; none in any other mode) goes down a tick,
//!   and once it is below 0 (0.0 at 0xdd054) its slot is freed (+0xb8 =
//!   0, @0x8fbb3): no explosion, no fate, no event
//!   ([`Outcome::Freed`]). Every NPC spawner leaves +0x4c at 0
//!   (`_RandomShipSpawn` @0x3c3ff, `_GenericRandomShipSpawn` @0x3c9c7,
//!   `_MissionDudeSpawn` @0x3ced1), so a pod spawned is gone on its first
//!   tick.
//!
//! No stock `düde`, `flët`, `përs` or fighter bay names shïp 895, so only
//! plug-in data brings a pod NPC; the player's own pod is the player's
//! ship, never an NPC.
//!
//! Left out: `_LaunchFighter` sets a fighter's wait to its bay's `wëap`
//! +0x0 (@0x3d85b), which the fighters don't model, so a pod launched from
//! a bay (no stock bay launches one) is gone on its first tick too; and
//! `_HandleShip`'s AI-wait fire gate and countdown skip a pod (@0x34a6c,
//! @0x3502f), which this branch has neither of
//! (rdm:phase/ship-handling-fidelity/phase-20-npc-pod-sound-priority-and-fire-gate).

use super::autopilot::{Outcome, steer};
use super::npc::{Mode, Npc, NpcId};
use crate::combat::hull::ESCAPE_POD_SHIP;
use crate::flight::Controls;
use crate::handling::Handling;

/// Whether `npc` is an escape pod, of class [`ESCAPE_POD_SHIP`].
#[must_use]
pub fn is_pod(npc: &Npc) -> bool {
    npc.ship == ESCAPE_POD_SHIP
}

/// The controls a pod flies by: full thrust, steering to its own heading
/// truncated to the whole degree.
#[must_use]
pub fn controls(npc: &Npc, handling: &Handling) -> Controls {
    let heading = npc.state.heading;
    Controls {
        thrust: true,
        turn: steer(heading, heading.trunc(), handling),
        ..Controls::default()
    }
}

/// One `_AIDispatch` pass over the pods among `npcs`: each one's wait goes
/// down a tick, and each whose wait goes below 0 is taken out. The pods
/// taken out, in order, each [`Outcome::Freed`].
pub fn count_down(npcs: &mut Vec<Npc>) -> Vec<(NpcId, Outcome)> {
    let mut freed = Vec::new();
    npcs.retain_mut(|npc| {
        if !is_pod(npc) {
            return true;
        }
        match npc.mode {
            Mode::Adrift { ticks_left } if ticks_left > 0 => {
                npc.mode = Mode::Adrift {
                    ticks_left: ticks_left - 1,
                };
                true
            }
            _ => {
                freed.push((npc.id, Outcome::Freed));
                false
            }
        }
    });
    freed
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::{Behaviour, Goal, Surroundings};
    use crate::chance::Chance;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::{Traffic, World};
    use nova_sim_base::fixture::{Draws, FAST};
    use std::cell::RefCell;

    /// Records each NPC that decides, and keeps its goal.
    #[derive(Debug, Default)]
    struct Recording {
        decided: RefCell<Vec<NpcId>>,
    }

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.decided.borrow_mut().push(npc.id);
            npc.goal
        }
    }

    /// An escape pod at the centre, at rest, facing `heading`, in `mode`.
    fn pod(heading: f32, mode: Mode) -> Npc {
        let mut pod = npc(0, ShipStats::new(FAST, &[]));
        pod.ship = ESCAPE_POD_SHIP;
        pod.mode = mode;
        pod.state = ShipState {
            heading,
            ..ShipState::default()
        };
        pod
    }

    /// Traffic holding `npcs`.
    fn holding(npcs: Vec<Npc>) -> Traffic {
        let mut traffic = Traffic::new();
        for npc in npcs {
            traffic.add_npc(npc);
        }
        traffic
    }

    fn tick(traffic: &mut Traffic, behaviour: &Recording) {
        traffic.tick_in(behaviour, World::new(&[]), &[], &mut Draws::of(&[]));
    }

    #[test]
    fn a_pod_with_a_wait_of_2_thrusts_along_its_heading_for_2_ticks_then_is_freed() {
        let behaviour = Recording::default();
        let mut traffic = holding(vec![pod(90.0, Mode::Adrift { ticks_left: 2 })]);
        let mut seen = Vec::new();
        for _ in 0..2 {
            tick(&mut traffic, &behaviour);
            let pod = &traffic.npcs()[0];
            seen.push((pod.mode, pod.state.heading));
            assert!(pod.state.velocity.x > 0.0, "thrusting along 90°");
            assert!(pod.state.velocity.y.abs() < 1e-6, "none across it");
            assert!(traffic.departed().is_empty());
        }
        assert_eq!(
            seen,
            [
                (Mode::Adrift { ticks_left: 1 }, 90.0),
                (Mode::Adrift { ticks_left: 0 }, 90.0)
            ]
        );
        let speed = traffic.npcs()[0].state.velocity.x;
        assert!(speed > 0.0);
        tick(&mut traffic, &behaviour);
        assert!(traffic.npcs().is_empty(), "below 0: gone");
        assert_eq!(traffic.departed(), [(NpcId(0), Outcome::Freed)]);
        assert!(behaviour.decided.borrow().is_empty(), "it never decided");
    }

    #[test]
    fn a_pod_with_no_wait_is_gone_on_its_first_tick_and_the_others_fly_on() {
        let behaviour = Recording::default();
        let mut other = npc(0, ShipStats::new(FAST, &[]));
        other.state.position = Vec2::new(500.0, 0.0);
        let mut traffic = holding(vec![other, pod(0.0, Mode::Flying)]);
        tick(&mut traffic, &behaviour);
        assert_eq!(traffic.npcs().len(), 1);
        assert_eq!(traffic.npcs()[0].id, NpcId(0), "the other ship stays");
        assert_eq!(traffic.departed(), [(NpcId(1), Outcome::Freed)]);
        assert_eq!(*behaviour.decided.borrow(), [NpcId(0)], "the other decides");
    }

    #[test]
    fn a_pods_heading_never_turns_past_its_whole_degree() {
        let behaviour = Recording::default();
        for heading in [0.0, 45.0, 45.5, 359.9] {
            let mut traffic = holding(vec![pod(heading, Mode::Adrift { ticks_left: 5 })]);
            for _ in 0..5 {
                tick(&mut traffic, &behaviour);
                assert_eq!(traffic.npcs()[0].state.heading, heading);
            }
        }
    }

    #[test]
    fn a_pod_moves_again_on_the_second_pass_without_counting_down() {
        let behaviour = Recording::default();
        let mut traffic = holding(vec![pod(180.0, Mode::Adrift { ticks_left: 1 })]);
        tick(&mut traffic, &behaviour);
        let before = traffic.npcs()[0].state;
        traffic.move_in(World::new(&[]), &mut Draws::of(&[]));
        let pod = &traffic.npcs()[0];
        assert_eq!(pod.mode, Mode::Adrift { ticks_left: 0 }, "no countdown");
        assert!(pod.state.position.y > before.position.y, "it moves down");
    }

    #[test]
    fn a_pods_controls_are_full_thrust_to_its_whole_heading() {
        let handling = Handling {
            turn_rate: 0.4,
            ..Handling::default()
        };
        let controls = controls(&pod(45.9, Mode::Flying), &handling);
        assert!(controls.thrust);
        assert_eq!(controls.turn, crate::flight::Turn::Left, "back to 45°");
        let none = super::controls(&pod(45.0, Mode::Flying), &handling);
        assert_eq!(none.turn, crate::flight::Turn::None);
        assert!(!is_pod(&npc(0, ShipStats::default())));
    }
}
