//! A ship's turret drawn swinging toward its aim at the rate
//! ([`TurretSwings`](crate::flight::turret_swing::TurretSwings), whose own
//! tests pin the swing), stepped once a pass the view shows: over
//! several frames rather than in one, twice as far in a frame of two
//! passes and not at all in a frame of none, as far for the same passes
//! however frames group them, back to the nose once the
//! readout drops, afresh after a change of system or a landing, at a
//! faded ship's alpha, and with nothing the view sends to the flight
//! changed. On [`FakeFlight`], with `ship_models`' hand-built table: each
//! pass reported holds the very ships the frame shown holds, NPC 0 of
//! ship 130 at 90°, which the table gives a turret.

use nova_sim::cloak::CloakShown;
use nova_sim::combat::hull_turn::HullTurn;
use nova_sim::flight_frame::{FlightEvent, GateFade, PassReport, StepReport, StepRun};
use nova_sim::landing::LandOutcome;
use nova_sim::{LandPress, SessionEvent};

use super::faked_world::world_list;
use super::ship_models::{
    BED, NPC_HULL, alpha_of, expected, expected_both, expected_player, faded, frame, models,
    npc_jitter, table, turret_turns, view_of,
};
use super::*;
use crate::flight::turret_swing::TurretSwings;
use crate::flight::world_models::WorldModels;
use crate::system::camera::Anchor;

/// The NPC's hull angle in every frame here.
const HULL: f32 = 90.0;

/// The frame `ship_models` draws, the NPC's turrets at `facing`, edited by
/// `edit`.
fn aimed_by(facing: Option<f32>, edit: impl FnOnce(&mut FrameShip)) -> FlightFrame {
    frame(0.0, HullTurn::Continuous, |ships| {
        ships[1].turret_facing = facing;
        edit(&mut ships[1]);
    })
}

/// [`aimed_by`] with no edit.
fn aimed(facing: Option<f32>) -> FlightFrame {
    aimed_by(facing, |_| {})
}

/// A view of [`aimed`]`(None)` by `models`, no pass shown yet.
fn viewing(models: WorldModels) -> Faked {
    view_of(aimed(None), models)
}

/// A step whose passes are `passes` passes of `shown`'s ships.
fn step_of(shown: &FlightFrame, passes: usize) -> StepRun {
    let pass = PassReport {
        ships: shown.ships.clone(),
        ..PassReport::default()
    };
    StepRun {
        passes: vec![pass; passes],
        ..StepRun::default()
    }
}

/// Runs a frame of `view` whose flight reports `steps` and shows `shown`
/// from then on.
fn run(view: &mut Faked, shown: FlightFrame, steps: Vec<StepRun>) {
    let count = u32::try_from(steps.len()).expect("a frame's steps");
    let report = StepReport {
        steps,
        stopped: false,
    };
    view.sim.queue_report_showing(report, shown);
    view.step_frame(count, Anchor::in_flight(RuleSource::Engine));
}

/// Runs a frame of `view` of one step of `passes` passes of `shown`, and
/// gives the NPC's turret's turn on its hull as it is then drawn, in
/// degrees.
fn frame_of(view: &mut Faked, shown: FlightFrame, passes: usize) -> f32 {
    let step = step_of(&shown, passes);
    run(view, shown, vec![step]);
    npc_turn(view)
}

/// The NPC's turret's turn on its hull as `view` draws it, in degrees.
fn npc_turn(view: &Faked) -> f32 {
    let [turn] = turret_turns(&world_list(view))[..] else {
        panic!("the NPC's turret alone");
    };
    turn.to_degrees()
}

/// Whether `a` and `b` agree to within a rounding.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_turret_swings_to_its_aim_at_twelve_a_pass_over_several_frames() {
    let mut view = viewing(table());
    assert_eq!(frame_of(&mut view, aimed(None), 1), 0.0, "on the nose");
    let target = 200.0 - HULL;
    let mut turns = Vec::new();
    for _ in 0..12 {
        turns.push(frame_of(&mut view, aimed(Some(200.0)), 1));
    }
    // 12, 24, … 108, then 110 and there it stays.
    for (k, turn) in turns.iter().enumerate().take(9) {
        let want = 12.0 * f32::from(u8::try_from(k + 1).expect("small"));
        assert!(near(*turn, want), "frame {k}: {turns:?}");
    }
    for turn in &turns[9..] {
        assert!(near(*turn, target), "{turns:?}");
    }
    // Settled, it draws the readout's own commands.
    let list = world_list(&view);
    assert_eq!(turret_turns(&list), [(200.0_f32 - 90.0).to_radians()]);
    assert_eq!(models(&list), expected_both(&view, HullTurn::Continuous));
}

#[test]
fn a_frame_of_two_passes_swings_twice_as_far_and_one_of_none_not_at_all() {
    let mut view = viewing(table());
    frame_of(&mut view, aimed(None), 1);
    assert!(near(frame_of(&mut view, aimed(Some(200.0)), 2), 24.0));
    // A frame running no step (a pause, a dialog) leaves it where it is.
    run(&mut view, aimed(Some(200.0)), Vec::new());
    assert!(near(npc_turn(&view), 24.0));
    // Two steps of a pass each swing two passes.
    let shown = aimed(Some(200.0));
    let steps = vec![step_of(&shown, 1), step_of(&shown, 1)];
    run(&mut view, shown, steps);
    assert!(near(npc_turn(&view), 48.0));
}

#[test]
fn the_same_passes_swing_as_far_however_frames_group_them() {
    // Three passes toward 200, shown as one frame of three, three frames of
    // one, and a frame of two then one of one.
    let swung = |grouping: &[usize]| {
        let mut view = viewing(table());
        frame_of(&mut view, aimed(None), 1);
        for &passes in grouping {
            frame_of(&mut view, aimed(Some(200.0)), passes);
        }
        npc_turn(&view)
    };
    let one_frame = swung(&[3]);
    assert!(
        one_frame > 0.0 && one_frame < 200.0 - HULL,
        "partway: {one_frame}"
    );
    assert!(near(swung(&[1, 1, 1]), one_frame));
    assert!(near(swung(&[2, 1]), one_frame));
}

#[test]
fn a_turret_whose_readout_drops_swings_back_to_the_nose() {
    let mut view = viewing(table());
    frame_of(&mut view, aimed(Some(130.0)), 1);
    assert!(near(npc_turn(&view), 40.0), "first seen at its aim");
    let back: Vec<f32> = (0..4)
        .map(|_| frame_of(&mut view, aimed(None), 1))
        .collect();
    assert!(near(back[0], 28.0) && near(back[1], 16.0), "{back:?}");
    assert!(near(back[2], 4.0), "{back:?}");
    assert_eq!(back[3], 0.0, "{back:?}");
}

#[test]
fn a_change_of_system_lets_the_swings_go() {
    let mut view = viewing(table());
    frame_of(&mut view, aimed(None), 1);
    assert!(near(frame_of(&mut view, aimed(Some(200.0)), 1), 12.0));
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        ..aimed(Some(200.0))
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.arrived(&[]);
    assert_eq!(view.turret_swings, TurretSwings::default());
    assert!(near(npc_turn(&view), 110.0), "at its readout");
}

#[test]
fn a_landing_lets_the_swings_go() {
    let mut view = viewing(table());
    frame_of(&mut view, aimed(None), 1);
    frame_of(&mut view, aimed(Some(200.0)), 1);
    assert_ne!(view.turret_swings, TurretSwings::default());
    view.sim
        .queue_answer(Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(
            StellarId(128),
        ))));
    view.land();
    assert_eq!(view.turret_swings, TurretSwings::default());
}

#[test]
fn a_faded_ship_draws_its_swinging_turret_at_its_fade() {
    let cloak = CloakShown::Fading(21.5);
    let cloaked = |facing| aimed_by(facing, |npc| npc.cloak = cloak);
    let mut view = viewing(table());
    frame_of(&mut view, cloaked(None), 1);
    frame_of(&mut view, cloaked(Some(200.0)), 1);
    // Drawn as a ship whose turrets face 12 round from its nose.
    let mut swinging = *view.drawn().npcs().next().expect("the npc");
    swinging.turret_facing = Some(HULL + 12.0);
    let (at, heading) = view.shown_npc(&swinging);
    let npc = expected(
        &swinging,
        (
            view.camera().sprite_centre(at, super::ship_models::DART),
            heading,
            HullTurn::Continuous,
        ),
        NPC_HULL,
        &[BED],
    );
    let alpha = alpha_of(cloak, GateFade::None);
    assert!(alpha < 1.0);
    let mut want = faded(npc, npc_jitter(&view, cloak), alpha);
    want.extend(expected_player(&view, HullTurn::Continuous));
    assert_eq!(models(&world_list(&view)), want);
}

#[test]
fn the_swing_sends_the_flight_nothing() {
    let mut fixed = table();
    for mounts in fixed.mounts.kits.values_mut() {
        for mount in mounts {
            mount.turns = false;
        }
    }
    let script = [None, Some(200.0), Some(200.0), Some(-40.0), None, None];
    let flown = |models: WorldModels| {
        let mut view = viewing(models);
        let turns: Vec<f32> = script
            .iter()
            .map(|&facing| {
                frame_of(&mut view, aimed(facing), 1);
                turret_turns(&world_list(&view))
                    .first()
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect();
        (view, turns)
    };
    let (turning, swung) = flown(table());
    let (still, unswung) = flown(fixed);
    assert!(
        swung.iter().any(|turn| *turn != 0.0),
        "it swings: {swung:?}"
    );
    assert!(unswung.iter().all(|turn| *turn == 0.0), "{unswung:?}");
    assert_eq!(turning.sim.stepped_calls(), still.sim.stepped_calls());
    assert_eq!(turning.sim.applied(), still.sim.applied());
    assert_eq!(turning.sim.asks(), still.sim.asks());
}
