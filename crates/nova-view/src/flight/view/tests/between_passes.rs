//! What a double-time frame reads out while it holds the first pass's
//! state (see the `between_passes` module): each readout and the whole
//! draw list are those of a flight showing that state as it stands, and
//! none is the flight's after its second pass.

use super::faked_hud::{barred, flown, npc_of_140};
use super::*;
use crate::flight::hud::NavDisplay;
use crate::system::camera::Anchor;
use nova_sim::flight_frame::{FrameShip, FrameTarget, PassReport, StepReport, StepRun};
use nova_sim::{AlertIcon, JumpReadiness, TargetStatus};

/// The ship a frame draws the player's as.
fn anchor() -> Anchor {
    Anchor::in_flight(RuleSource::Engine)
}

/// The system [`first_pass`] selects, and has explored.
const SELECTED: SystemId = SystemId(131);

/// The flight after a second pass: the player alone, hidden, with
/// nothing to show.
fn second_pass() -> FlightFrame {
    let mut frame = among(Vec::new());
    frame.ships[0].position = Vec2::new(20.0, 0.0);
    frame.ships[0].shown = nova_sim::Drawn::Hidden;
    frame.ships[0].framed = None;
    frame.jump_readiness = JumpReadiness::Blocked;
    frame
}

/// The flight between the passes, different from [`second_pass`] in all
/// that a frame reads out.
fn first_pass() -> FlightFrame {
    let npc: FrameShip = npc_of_140(0);
    let mut frame = among(vec![npc]);
    frame.ships[0].drawn.set = 1;
    frame.jump_readiness = JumpReadiness::Clear;
    frame.beams = vec![super::faked_world::beam(
        ShipRef::Player,
        BLASTER,
        Vec2::ZERO,
        Vec2::new(0.0, -100.0),
    )];
    frame.effective_murk = 40;
    frame.system_background = Some(0x0007_0F12);
    frame.double_time = true;
    frame.nav_mode = NavMode::Hyperspace(Some(SELECTED));
    frame.explored = vec![SELECTED];
    frame.target = Some(FrameTarget {
        npc: NpcId(0),
        name: "Ace".to_owned(),
        subtitle: None,
        status: TargetStatus::Disabled,
        tag: None,
    });
    frame.hud.target_brackets = super::faked_world::targeting(0).target_brackets;
    frame.hud.red_alert = Some(AlertIcon::Red);
    frame.hud.reserves.fuel.now = 7.0;
    frame.hud.radar_gear.iff = true;
    frame.hud.cargo.cash = 111;
    frame
}

/// A pass that left the ships as `frame` has them.
fn pass_of(frame: &FlightFrame) -> PassReport {
    PassReport {
        ships: frame.ships.clone(),
        ..PassReport::default()
    }
}

/// A view over a flight showing `frame`, saying a line.
fn viewing(frame: FlightFrame) -> Faked {
    let catalog = armed(1, &[BLASTER]);
    let sheet = catalog.ship_sheet(ShipId(128));
    let mut view = barred(catalog, frame);
    view.sheet = sheet;
    view.alert_width = Some(32);
    view.say_for("Hello", 600);
    view
}

/// A view that stepped once in single time to `frame`, at alpha 1.
fn stepped_to(frame: FlightFrame) -> Faked {
    let pass = pass_of(&frame);
    let mut view = viewing(frame);
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, anchor());
    view.alpha = 1.0;
    view
}

/// A view that stepped once in double time, past [`first_pass`] to
/// [`second_pass`], at alpha 1.
fn held() -> Faked {
    let first = first_pass();
    let mut view = viewing(second_pass());
    let report = super::stepping::doubled(pass_of(&first), pass_of(&second_pass()), first);
    view.sim.queue_report(report);
    view.step_frame(1, anchor());
    view.alpha = 1.0;
    view
}

/// What `view` reads out, by name, its HUD's snapshots taken afresh.
fn readouts(view: &mut Faked) -> Vec<(&'static str, String)> {
    view.redraw_whole();
    let camera = view.camera();
    vec![
        ("beams", format!("{:?}", view.shown_beams(&camera))),
        ("murk", format!("{:?}", view.seen_murk())),
        ("sprite frame", format!("{:?}", view.frame())),
        ("boxes", format!("{:?}", view.drawn_boxes())),
        ("radar", format!("{:?}", view.radar_scene)),
        ("reserves", format!("{:?}", view.reserves_scene)),
        ("target", format!("{:?}", view.target_scene)),
        ("nav", format!("{:?}", view.nav_scene)),
        ("nav now", format!("{:?}", view.nav_now())),
        ("draw list", format!("{:?}", flown(view))),
    ]
}

#[test]
fn a_double_time_frame_reads_out_the_first_passs_state() {
    let held = readouts(&mut held());
    let first = readouts(&mut stepped_to(first_pass()));
    let second = readouts(&mut stepped_to(second_pass()));
    for ((name, held), ((_, first), (_, second))) in held.iter().zip(first.iter().zip(&second)) {
        assert_eq!(held, first, "{name}");
        assert_ne!(held, second, "{name}");
    }
}

#[test]
fn the_nav_snapshot_stands_while_it_was_taken_for_the_mode_drawn() {
    let mut view = held();
    view.retake_nav();
    if let Some((_, shown)) = &mut view.nav_scene {
        *shown = NavDisplay::Off;
    }
    assert_eq!(view.nav_shown(view.drawn()), NavDisplay::Off);
}

#[test]
fn the_radars_static_rolls_on_the_interference_drawn() {
    let mut first = first_pass();
    first.hud.interference = 100;
    let mut view = viewing(second_pass());
    view.effects_chance = rolling_one();
    let report = super::stepping::doubled(pass_of(&first), pass_of(&first), first);
    view.sim.queue_report(report);
    view.step_frame(1, anchor());
    view.advance_hud();
    assert!(view.radar_static.is_some(), "full interference");
}

#[test]
fn a_target_met_only_in_the_frame_drawn_is_taken_for_the_panel_as_the_hud_advances() {
    let mut view = held();
    view.advance_hud();
    view.target_scene = Some((None, None));
    view.advance_hud();
    let drawn = view.target_scene.as_ref().and_then(|(drawn, _)| *drawn);
    assert_eq!(drawn.map(|drawn| drawn.npc), Some(NpcId(0)));
}
