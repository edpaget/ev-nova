//! The new game's hulls turn smoothly: over its checked-in fixture
//! (`nova_data::new_game::fixture::folder`), under the rulebook
//! `ruleset_rulebook` gives `Ruleset::NewGame`, a turning player's frame
//! says its hull turns continuously, and the model orientation the view
//! derives from the frame (`nova_view::flight::ship_orientation`) changes
//! on every tick and faces the heading. Under the stock ruleset's
//! readings the same turn holds the orientation still within a sprite
//! frame.
//!
//! What the hull's angle is under each reading is nova-sim-combat's rule
//! (`hull_turn`), and how the view turns a model by it is nova-view's
//! (`ship_model`), each pinned there; this pins that the ruleset, the
//! session's frame and the view's orientation meet.

// Headings compared exactly: whether a tick moved them at all.
#![allow(clippy::float_cmp)]

use nova::chance::SplitMix;
use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova_sim::combat::hull_turn::HullTurn;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{Controls, NovaAi, Rules, Session, Turn};
use nova_view::flight::ship_orientation;
use nova_view::model::Orientation;

use crate::new_game_travel::{fixture, pilot_for};

/// The ticks the player turns for.
const TICKS: usize = 12;

/// The player's heading and model orientation after each of [`TICKS`]
/// passes turning right over the fixture, under `ruleset`'s readings, and
/// the hull turn its frames say.
fn turning(ruleset: Ruleset) -> (Vec<(f32, Orientation)>, HullTurn) {
    let game = fixture();
    let rules = ruleset_rulebook(None, ruleset.profile().readings).0;
    let mut session = Session::load(&game.data, pilot_for(&game), &rules).expect("flies");
    let mut chance = SplitMix::new(7);
    let mut frame = FlightFrame::default();
    let controls = Controls {
        turn: Turn::Right,
        ..Controls::default()
    };
    let mut seen = Vec::new();
    for _ in 0..TICKS {
        let rescued = session.game_work(
            controls,
            &game.data,
            &NovaAi::from_rulebook(&rules),
            Rules::default(),
            &mut chance,
        );
        assert!(!rescued);
        session.fill_frame(&mut frame);
        let player = frame.player();
        let orientation = ship_orientation(&player.frames, player.heading, frame.hull_turn);
        seen.push((player.heading, orientation));
    }
    (seen, frame.hull_turn)
}

#[test]
fn under_the_new_game_a_turning_hulls_model_turns_every_tick_and_faces_its_heading() {
    let (seen, hull_turn) = turning(Ruleset::NewGame);
    assert_eq!(hull_turn, HullTurn::Continuous);
    for pair in seen.windows(2) {
        assert_ne!(pair[0].0, pair[1].0, "the ship turns every tick");
        assert_ne!(pair[0].1, pair[1].1, "and so does its model: {pair:?}");
    }
    for (heading, orientation) in &seen {
        let nose = orientation.rotate([0.0, -1.0, 0.0]).expect("turns");
        let (sin, cos) = heading.to_radians().sin_cos();
        let facing = [sin, -cos, 0.0];
        for (got, want) in nose.iter().zip(facing) {
            assert!((got - want).abs() < 1e-4, "{heading}: {nose:?}");
        }
    }
}

#[test]
fn under_the_stock_readings_a_turning_hulls_model_holds_within_a_frame() {
    let (seen, hull_turn) = turning(Ruleset::Stock);
    assert_eq!(hull_turn, HullTurn::ByFrame);
    assert!(
        seen.windows(2)
            .any(|pair| pair[0].0 != pair[1].0 && pair[0].1 == pair[1].1),
        "a tick turns the ship but not its model: {seen:?}"
    );
}
