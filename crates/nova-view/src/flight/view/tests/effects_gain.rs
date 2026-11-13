//! The frame's gain the view steps its explosions and smoke by: the
//! pass's [`PassReport::speed_mult`], `_gSpeedMult` as `_HandleExplods`
//! (@0x2f655, @0x2f811) and `_HandleSmoke` (@0x2e3d6) read it, which
//! `step_effects` hands to [`Effects::show_step`] and to the shot looks'
//! [`SmokePuffs::step`]; the escape pods are the simulation's
//! (`nova_sim::escape_pod`); and the arrival pass's, which the arrival's
//! explosions take from the frame the catch-up refilled. Each expected
//! value is the effect's own step at the same gain, on a clone of the
//! state before the pass.

use nova_sim_frame::SpeedMult;
use nova_sim_frame::flight_frame::{PassReport, StepReport, StepRun};

use super::stepping::{fight, frame, one_step, stepping};
use super::*;
use crate::flight::effects::Breakups;
use crate::flight::smoke::SmokePuffs;

const HALF: SpeedMult = SpeedMult::new(0.5);

/// A pass at `gain` raising `combat`.
fn at(gain: SpeedMult, combat: Vec<CombatEvent>) -> PassReport {
    PassReport {
        speed_mult: gain,
        ..fight(combat)
    }
}

/// Type 0's explosion going off at the centre.
fn exploded() -> CombatEvent {
    CombatEvent::Exploded {
        at: Vec2::ZERO,
        explosion: nova_sim_frame::catalog::Explosion::decode(0).expect("type 0"),
        size: 0.0,
    }
}

/// `effects` after a show step at `gain`, in the scene `view` lays out
/// for a pass of `pass`.
fn shown_at(view: &Faked, mut effects: Effects, pass: &PassReport, gain: SpeedMult) -> Effects {
    let scene = view.presentation.scene(pass.player_position);
    effects.show_step(&Breakups::default(), &scene, &view.looks, gain);
    effects
}

#[test]
fn an_explosion_advances_at_its_passs_gain() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![at(HALF, vec![exploded()])]));
    let before = view.effects().clone();
    assert_eq!(before.explosions().len(), 1, "set off");
    let pass = at(HALF, vec![]);
    frame(&mut view, one_step(vec![pass.clone()]));
    let expected = shown_at(&view, before.clone(), &pass, HALF).explosions();
    assert_eq!(view.effects().explosions(), expected);
    let full = shown_at(&view, before, &pass, SpeedMult::FULL).explosions();
    assert_ne!(expected, full, "the gain shows");
}

#[test]
fn a_puff_ages_at_its_passs_gain() {
    use super::shots::{blaster_shot, pass_with, shooting, smoking_trailing_blaster};
    let mut view = shooting(smoking_trailing_blaster());
    let mut tick = 0;
    while view.shot_looks.smoke().puffs().is_empty() {
        pass_with(
            &mut view,
            vec![blaster_shot(tick)],
            tick % 3 == 2,
            Vec::new(),
        );
        tick += 1;
    }
    let before: SmokePuffs = view.shot_looks.smoke().clone();
    let pass = PassReport {
        speed_mult: HALF,
        ships: view.frame.ships.clone(),
        player_position: view.frame.player().position,
        ..PassReport::default()
    };
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, Anchor::in_flight(RuleSource::Engine));
    let mut expected = before.clone();
    expected.step(HALF);
    assert_eq!(view.shot_looks.smoke().puffs(), expected.puffs());
    let mut full = before;
    full.step(SpeedMult::FULL);
    assert_ne!(expected.puffs(), full.puffs(), "the gain shows");
}

#[test]
fn the_arrival_passs_explosion_shows_in_the_new_scene_and_steps_once_at_its_gain() {
    use super::faked_travel::{arrival_report, arrived_in_alpha, jump_now, now_shows};
    let boom = nova_sim_frame::catalog::Explosion::decode(0).expect("type 0");
    let event = CombatEvent::ExplodedHeard {
        at: Vec2::ZERO,
        explosion: boom,
    };
    let mut view = super::faked_world::world(
        armed(0, &[BLASTER]),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    jump_now(&mut view);
    now_shows(&mut view, super::frames::under_way(true));
    // The arrival's days raised a `Y` (@0x43e19), reported with the rest
    // of its pass, which runs at half gain.
    let mut report = arrival_report(0, nova_sim_frame::flight_frame::ShipPlace::default());
    report.steps[0].passes[0].combat = vec![event];
    report.steps[0].passes[0].speed_mult = HALF;
    view.sim.queue_report(report);
    arrived_in_alpha(&mut view);
    view.run_steps(1, None);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    let arrived = |gain| {
        let scene = view.presentation.scene(Vec2::ZERO);
        let mut effects = Effects::default();
        let mut chance = view.effects_chance.clone();
        effects.apply(&event, &scene, &view.looks, &mut chance);
        effects.step(&[], &scene, &view.looks, gain, &mut chance);
        effects.explosions()
    };
    let expected = arrived(HALF);
    assert_eq!(view.effects().explosions(), expected);
    assert_ne!(expected, arrived(SpeedMult::FULL), "the gain shows");
}

#[test]
fn a_pass_at_the_default_gain_steps_the_effects_as_a_full_gain_step() {
    let mut view = stepping(armed(1, &[BLASTER]));
    frame(&mut view, one_step(vec![fight(vec![exploded()])]));
    let before = view.effects().clone();
    let pass = fight(vec![]);
    assert_eq!(pass.speed_mult, SpeedMult::FULL, "the default");
    frame(&mut view, one_step(vec![pass.clone()]));
    let expected = shown_at(&view, before, &pass, SpeedMult::FULL);
    assert_eq!(view.effects().explosions(), expected.explosions());
}

#[test]
fn a_pass_at_the_default_gain_ages_a_puff_as_a_full_gain_step() {
    use super::shots::{blaster_shot, pass_with, shooting, smoking_trailing_blaster};
    let mut view = shooting(smoking_trailing_blaster());
    let mut tick = 0;
    while view.shot_looks.smoke().puffs().is_empty() {
        pass_with(
            &mut view,
            vec![blaster_shot(tick)],
            tick % 3 == 2,
            Vec::new(),
        );
        tick += 1;
    }
    let mut expected: SmokePuffs = view.shot_looks.smoke().clone();
    pass_with(&mut view, Vec::new(), false, Vec::new());
    expected.step(SpeedMult::FULL);
    assert_eq!(view.shot_looks.smoke().puffs(), expected.puffs());
}
