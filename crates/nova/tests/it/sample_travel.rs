//! The new game flown over game-bible's sample (the `data` folder
//! `GAME_DATA` names), as `new_game_travel` flies the checked-in fixture:
//! the player goes through every jump point's gate and comes out at its
//! partner's, and the traffic comes out of and goes into every gate under
//! the new game's readings. Skips, passing, when `GAME_DATA` is unset.
//!
//! It reads the gates from the mapped data (`NewGame::ids.gates`, one per
//! jump point), not from counts written here, so the sample can grow.
//! While the sample has no start, New Pilot can't fly it: that check says
//! so and skips, and the rest fly the fixture's start moved into the
//! sample's first hull.

use nova_data::new_game::{self, NewGame};
use nova_sim::Session;

use crate::common::game_data;
use crate::new_game_travel::{
    TRAFFIC_TICKS, fly_through, gates, new_game_rules, of_system, pilot_for, systems, usable_gates,
    watch_traffic,
};

/// The sample, mapped, with nothing wrong.
fn sample() -> Option<NewGame> {
    let dir = game_data()?;
    let game = new_game::open(&dir).expect("GAME_DATA lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    Some(game)
}

#[test]
fn new_pilot_starts_the_sample_once_it_has_a_start() {
    let Some(game) = sample() else { return };
    if game.ids.starts.is_empty() {
        eprintln!("skipping: the sample has no start yet");
        return;
    }
    let session = Session::start(&game.data).expect("the sample's start flies");
    assert!(
        game.ids
            .systems
            .values()
            .any(|&nova| nova == session.system().0)
    );
}

#[test]
fn the_player_goes_through_every_gate_of_the_sample_and_comes_out_at_its_partner() {
    let Some(game) = sample() else { return };
    let gates = gates(&game);
    assert_eq!(gates.len(), game.ids.gates.len(), "every jump point");
    let rules = new_game_rules();
    let pilot = pilot_for(&game);
    for gate in &gates {
        fly_through(&game.data, &rules, &pilot, gate);
    }
}

#[test]
fn under_the_new_game_the_sample_s_traffic_comes_out_of_and_goes_into_every_gate() {
    let Some(game) = sample() else { return };
    let gates = gates(&game);
    let rules = new_game_rules();
    let pilot = pilot_for(&game);
    for system in systems(&gates) {
        let wanted = of_system(&gates, system);
        assert_eq!(
            usable_gates(&game.data, &rules, system),
            wanted,
            "{system:?}: every gate is one to come out of and leave by"
        );
        let seen = watch_traffic(
            &game.data,
            &rules,
            &pilot,
            system,
            &wanted,
            TRAFFIC_TICKS,
            7,
        );
        assert_eq!(seen.emerged, wanted, "{system:?}: out of every gate");
        assert_eq!(seen.entered, wanted, "{system:?}: into every gate");
    }
}
