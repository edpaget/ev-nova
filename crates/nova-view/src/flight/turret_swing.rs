//! A mounted turret's swing toward its aim: [`TurretSwings`], each ship's
//! turret turned on its hull toward the angle its
//! [`FrameShip::turret_facing`] asks for at [`TURRET_TURN_PER_PASS`], in
//! place of a snap to it (rdm:phase/rendering-3d/phase-7-turrets-turn-at-a-rate).
//!
//! The original draws no turrets on its ships, so nothing here is ported:
//! the swing is the new game's look, and only a ship drawn as a model
//! with a turning mount shows it (the view's `ship_models` module). The
//! simulation never reads it: the readout it follows is the session's,
//! and the view sends nothing for it, so it changes neither what a ship
//! fires at nor when.
//!
//! - **Measured on the hull.** The swing is the turret's angle relative
//!   to its hull, in degrees. Its target is the facing less the hull's
//!   angle (`FrameSpec::hull_angle` at the pass's heading and the frame's
//!   [`HullTurn`]), or 0, the nose, with no facing. So a turret on the
//!   nose stays there while its ship turns, and one held on a world aim
//!   tracks it as the hull turns under it, lagging only where the hull
//!   out-turns it.
//! - **Stepped once a pass shown**, at the pass's gain
//!   ([`SpeedMult::times`]), as the view steps its glows and flashes: a
//!   frame of N passes swings N steps, a double-time pass held for the
//!   frame after swings when it is let go, and a frame that runs no pass
//!   (a pause, the map, a dialog, a landing) or a pass at no gain swings
//!   none.
//! - **The short way** across 360, an exact half turn turning the positive
//!   way, landing on the target without overshoot ([`swung_toward`]).
//!   An aim that changes mid-swing is turned toward from where the turret
//!   stands.
//! - **A ship first seen**, or one whose class changed under the same
//!   [`ShipRef`] (the player's new ship, its pod, a reused id), stands at
//!   its target, swinging from nowhere. A ship a pass leaves out is let
//!   go. The view clears the whole table where it lets go of the NPCs'
//!   displays: a change of system, whose NPC ids start again, and a
//!   landing.
//! - **Drawn**: a settled turret, or a ship no pass has shown, draws at the
//!   frame's own facing, so once a swing ends the commands are the
//!   readout's; a swinging one at the hull's drawn angle plus its swing
//!   ([`TurretSwings::facing`]).

use std::collections::BTreeMap;

use nova_sim_frame::combat::hull_turn::HullTurn;
use nova_sim_frame::flight_frame::FrameShip;
use nova_sim_frame::{ShipId, ShipRef, SpeedMult};

/// How far a turret swings on its hull a pass at full gain, in degrees:
/// 360 a second at 30 passes, a half turn in 15 passes. It out-turns every
/// sample hull (4 to 8 degrees a tick), so a turret keeps a world aim
/// while its ship turns, and settles within the sample mass driver's
/// reload of 15 passes.
pub const TURRET_TURN_PER_PASS: f32 = 12.0;

/// `angle` in degrees wrapped to (−180, 180].
fn wrapped(angle: f32) -> f32 {
    let turned = angle.rem_euclid(360.0);
    if turned > 180.0 {
        turned - 360.0
    } else {
        turned
    }
}

/// `current` swung at most `step` degrees toward `target` the short way
/// round (an exact half turn the positive way), and whether it reached
/// it: within a step it lands on `target` exactly; otherwise it moves a
/// whole step, wrapped to (−180, 180].
#[must_use]
pub fn swung_toward(current: f32, target: f32, step: f32) -> (f32, bool) {
    let left = wrapped(target - current);
    if left.abs() <= step {
        (target, true)
    } else {
        (wrapped(current + step.copysign(left)), false)
    }
}

/// One ship's turret swing.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Swing {
    /// The class it was swung on.
    class: ShipId,
    /// The turret's angle on its hull, in degrees.
    turn: f32,
    /// Whether it stands at its target.
    settled: bool,
}

/// Each ship's turret swing (see the module docs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TurretSwings {
    swings: BTreeMap<ShipRef, Swing>,
}

impl TurretSwings {
    /// Swings each of a pass's `ships` that `swings` picks (the view's
    /// those with a model) one pass at `gain`, their hulls turned under
    /// `hull_turn`, and lets go of every ship the pass leaves out.
    pub fn step(
        &mut self,
        ships: &[FrameShip],
        hull_turn: HullTurn,
        gain: SpeedMult,
        swings: impl Fn(ShipId) -> bool,
    ) {
        let step = gain.times(TURRET_TURN_PER_PASS);
        let mut kept = BTreeMap::new();
        for ship in ships.iter().filter(|ship| swings(ship.class)) {
            let angle = ship.frames.hull_angle(ship.heading, hull_turn);
            let target = ship
                .turret_facing
                .map_or(0.0, |facing| wrapped(facing - angle));
            let swing = match self.swings.get(&ship.ship) {
                Some(swing) if swing.class == ship.class => {
                    let (turn, settled) = swung_toward(swing.turn, target, step);
                    Swing {
                        class: ship.class,
                        turn,
                        settled,
                    }
                }
                _ => Swing {
                    class: ship.class,
                    turn: target,
                    settled: true,
                },
            };
            kept.insert(ship.ship, swing);
        }
        self.swings = kept;
    }

    /// The world heading `ship`'s turrets draw at, its hull drawn at
    /// `hull_angle`: the frame's own facing while settled or unseen, its
    /// hull's angle plus its swing while it swings.
    #[must_use]
    pub fn facing(&self, ship: &FrameShip, hull_angle: f32) -> Option<f32> {
        match self.swings.get(&ship.ship) {
            Some(swing) if !swing.settled => Some(hull_angle + swing.turn),
            _ => ship.turret_facing,
        }
    }

    /// Lets go of every swing.
    pub fn clear(&mut self) {
        self.swings.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_frame::NpcId;
    use nova_sim_frame::combat::ship_frames::FrameSpec;

    use super::*;

    const NPC: ShipRef = ShipRef::Npc(NpcId(3));
    const CLASS: ShipId = ShipId(130);

    #[test]
    fn a_swing_moves_a_step_at_most_and_lands_on_its_target() {
        assert_eq!(swung_toward(0.0, 30.0, 12.0), (12.0, false));
        assert_eq!(swung_toward(12.0, 30.0, 12.0), (24.0, false));
        assert_eq!(swung_toward(24.0, 30.0, 12.0), (30.0, true));
        assert_eq!(swung_toward(30.0, 30.0, 12.0), (30.0, true));
        assert_eq!(swung_toward(0.0, -30.0, 12.0), (-12.0, false));
        // Exactly a step away lands.
        assert_eq!(swung_toward(0.0, 12.0, 12.0), (12.0, true));
    }

    #[test]
    fn a_swing_turns_the_short_way_and_a_half_turn_the_positive_way() {
        // 350 to 10 goes +20, through 0.
        assert_eq!(swung_toward(350.0, 10.0, 12.0), (2.0, false));
        assert_eq!(swung_toward(2.0, 10.0, 12.0), (10.0, true));
        // 10 to 350 goes −20.
        assert_eq!(swung_toward(10.0, 350.0, 12.0), (-2.0, false));
        assert_eq!(swung_toward(-170.0, 170.0, 12.0), (178.0, false));
        assert_eq!(swung_toward(0.0, 180.0, 12.0), (12.0, false));
        assert_eq!(swung_toward(0.0, -180.0, 12.0), (12.0, false));
        assert_eq!(swung_toward(90.0, -90.0, 12.0), (102.0, false));
    }

    #[test]
    fn no_step_swings_nothing() {
        assert_eq!(swung_toward(40.0, 100.0, 0.0), (40.0, false));
        assert_eq!(swung_toward(40.0, 40.0, 0.0), (40.0, true));
    }

    /// NPC [`NPC`] of `class` facing `heading` with its turrets at `facing`.
    fn ship(class: ShipId, heading: f32, facing: Option<f32>) -> FrameShip {
        FrameShip {
            ship: NPC,
            class,
            heading,
            turret_facing: facing,
            ..FrameShip::default()
        }
    }

    /// Steps `swings` over `ships` once at `gain`, every class swinging,
    /// turned continuously.
    fn step_at(swings: &mut TurretSwings, ships: &[FrameShip], gain: SpeedMult) {
        swings.step(ships, HullTurn::Continuous, gain, |_| true);
    }

    /// Steps `swings` over `ships` once at full gain.
    fn step(swings: &mut TurretSwings, ships: &[FrameShip]) {
        step_at(swings, ships, SpeedMult::FULL);
    }

    /// The turret's angle on its hull as `swings` draws `ship`, its hull at
    /// its own heading (the frame spec's none turns continuously).
    fn turn_of(swings: &TurretSwings, ship: &FrameShip) -> f32 {
        swings
            .facing(ship, ship.heading)
            .map_or(0.0, |facing| facing - ship.heading)
    }

    /// The turret's angle on `ship`'s hull after each of `passes` full
    /// passes of it, from standing at the nose.
    fn swung(from: FrameShip, to: FrameShip, passes: usize) -> Vec<f32> {
        let mut swings = TurretSwings::default();
        step(&mut swings, &[from]);
        (0..passes)
            .map(|_| {
                step(&mut swings, &[to]);
                turn_of(&swings, &to)
            })
            .collect()
    }

    #[test]
    fn a_turret_swings_at_the_rate_to_its_aim_and_stops() {
        let to = ship(CLASS, 90.0, Some(130.0));
        let turns = swung(ship(CLASS, 90.0, None), to, 5);
        assert_eq!(turns, [12.0, 24.0, 36.0, 40.0, 40.0]);
    }

    #[test]
    fn a_settled_turret_draws_at_the_readout_and_a_swinging_one_on_its_hull() {
        let mut swings = TurretSwings::default();
        let nose = ship(CLASS, 90.0, None);
        step(&mut swings, &[nose]);
        assert_eq!(swings.facing(&nose, 90.0), None, "None stays None");
        let aimed = ship(CLASS, 90.0, Some(130.0));
        step(&mut swings, &[aimed]);
        // Drawn at a hull angle of its own, 95.
        assert_eq!(swings.facing(&aimed, 95.0), Some(95.0 + 12.0));
        for _ in 0..3 {
            step(&mut swings, &[aimed]);
        }
        assert_eq!(swings.facing(&aimed, 95.0), Some(130.0), "the readout");
    }

    #[test]
    fn an_aim_changed_mid_swing_is_turned_toward_from_where_it_stands() {
        let mut swings = TurretSwings::default();
        step(&mut swings, &[ship(CLASS, 0.0, None)]);
        let first = ship(CLASS, 0.0, Some(90.0));
        step(&mut swings, &[first]);
        step(&mut swings, &[first]);
        assert_eq!(turn_of(&swings, &first), 24.0);
        let second = ship(CLASS, 0.0, Some(-30.0));
        step(&mut swings, &[second]);
        assert_eq!(turn_of(&swings, &second), 12.0);
        step(&mut swings, &[second]);
        assert_eq!(turn_of(&swings, &second), 0.0);
    }

    #[test]
    fn a_turret_whose_readout_drops_swings_back_to_the_nose() {
        let aimed = ship(CLASS, 0.0, Some(30.0));
        let turns = swung(aimed, ship(CLASS, 0.0, None), 3);
        assert_eq!(turns, [18.0, 6.0, 0.0]);
    }

    #[test]
    fn a_ship_first_seen_or_of_a_new_class_stands_at_its_readout() {
        let mut swings = TurretSwings::default();
        let aimed = ship(CLASS, 10.0, Some(150.0));
        step(&mut swings, &[aimed]);
        assert_eq!(swings.facing(&aimed, 10.0), Some(150.0));
        assert_eq!(swings.swings[&NPC].turn, 140.0);
        assert!(swings.swings[&NPC].settled);
        let pod = ship(ShipId(131), 10.0, Some(-50.0));
        step(&mut swings, &[pod]);
        assert_eq!(swings.facing(&pod, 10.0), Some(-50.0));
        assert_eq!(swings.swings[&NPC].turn, -60.0);
        assert_eq!(swings.swings[&NPC].class, ShipId(131));
        assert!(swings.swings[&NPC].settled);
    }

    #[test]
    fn a_ship_no_pass_showed_draws_at_its_readout() {
        let swings = TurretSwings::default();
        let aimed = ship(CLASS, 10.0, Some(150.0));
        assert_eq!(swings.facing(&aimed, 12.0), Some(150.0));
    }

    #[test]
    fn a_pass_at_no_gain_swings_nothing_and_at_half_gain_half() {
        let nose = ship(CLASS, 0.0, None);
        let aimed = ship(CLASS, 0.0, Some(90.0));
        let mut swings = TurretSwings::default();
        step(&mut swings, &[nose]);
        step_at(&mut swings, &[aimed], SpeedMult::ZERO);
        assert_eq!(turn_of(&swings, &aimed), 0.0);
        assert_eq!(swings.facing(&aimed, 0.0), Some(0.0), "swinging, unmoved");
        step_at(&mut swings, &[aimed], SpeedMult::new(0.5));
        assert_eq!(turn_of(&swings, &aimed), 6.0);
    }

    #[test]
    fn a_nose_turret_stays_on_the_nose_as_its_hull_turns() {
        let mut swings = TurretSwings::default();
        for heading in [0.0, 30.0, 60.0, 200.0] {
            let turning = ship(CLASS, heading, None);
            step(&mut swings, &[turning]);
            assert_eq!(swings.facing(&turning, heading), None, "{heading}");
        }
    }

    #[test]
    fn a_turret_on_a_fixed_aim_tracks_it_as_its_hull_turns_under_it() {
        let mut swings = TurretSwings::default();
        // The hull turns 8 a pass under an aim at 100: the turret's
        // target on it falls 8 a pass, within its step.
        for heading in [0.0, 8.0, 16.0, 24.0] {
            let turning = ship(CLASS, heading, Some(100.0));
            step(&mut swings, &[turning]);
            assert_eq!(swings.facing(&turning, heading), Some(100.0), "{heading}");
            assert_eq!(swings.swings[&NPC].turn, 100.0 - heading);
        }
        // Out-turned, 30 in a pass, it lags by 30 − 12.
        let fast = ship(CLASS, 54.0, Some(100.0));
        step(&mut swings, &[fast]);
        assert_eq!(swings.swings[&NPC].turn, 76.0 - 12.0);
        assert!(!swings.swings[&NPC].settled);
    }

    #[test]
    fn a_hull_turned_by_frame_is_measured_at_its_frame() {
        let frames = FrameSpec {
            frames: std::num::NonZeroU16::new(36).expect("36 frames"),
            ..FrameSpec::NONE
        };
        let aimed = FrameShip {
            frames,
            ..ship(CLASS, 4.0, Some(100.0))
        };
        let mut swings = TurretSwings::default();
        swings.step(&[aimed], HullTurn::ByFrame, SpeedMult::FULL, |_| true);
        let angle = frames.hull_angle(4.0, HullTurn::ByFrame);
        assert_ne!(angle, 4.0, "a frame's angle");
        assert_eq!(swings.swings[&NPC].turn, 100.0 - angle);
    }

    #[test]
    fn a_ship_a_pass_leaves_out_or_unpicked_is_let_go() {
        let mut swings = TurretSwings::default();
        let other = FrameShip {
            ship: ShipRef::Player,
            ..ship(ShipId(128), 0.0, None)
        };
        step(&mut swings, &[ship(CLASS, 0.0, None), other]);
        assert_eq!(swings.swings.len(), 2);
        step(&mut swings, &[other]);
        assert!(!swings.swings.contains_key(&NPC));
        swings.step(&[other], HullTurn::Continuous, SpeedMult::FULL, |class| {
            class == CLASS
        });
        assert!(swings.swings.is_empty(), "the player's class not picked");
    }

    #[test]
    fn clear_lets_go_of_every_swing() {
        let mut swings = TurretSwings::default();
        step(&mut swings, &[ship(CLASS, 0.0, None)]);
        swings.clear();
        assert_eq!(swings, TurretSwings::default());
    }
}
