//! A course over game-bible's sample (the `data` folder `GAME_DATA`
//! names), planned and flown gate by gate, by hand and on the gate
//! autopilot, as `new_game_routes` flies the checked-in fixture, and each
//! of the sample's lanes drawn in its look. Skips, passing, when
//! `GAME_DATA` is unset.
//!
//! The sample's lanes: brightwater–luyten-crossing, brightwater–marrow,
//! harrow-drift–marrow, kestrel-march–luyten-crossing,
//! luyten-crossing–ninefold, luyten-crossing–marrow and marrow–lantern
//! are stable both ways; harrow-drift–kestrel-march is dead both ways; and
//! harrow-drift → lantern (`smugglers-drop`) is unstable and one-way.

use nova_data::new_game::{self, NewGame};
use nova_sim::catalog::GateStability::{Dead, Stable, Unstable};
use nova_sim::map_lanes::Back;

use crate::common::game_data;
use crate::new_game_routes::{fly_by_hand, fly_on_autopilot, flying_from, look_between, system};
use crate::new_game_travel::new_game_rules;

/// The sample, mapped, with nothing wrong.
fn sample() -> Option<NewGame> {
    let dir = game_data()?;
    let game = new_game::open(&dir).expect("GAME_DATA lists");
    let problems: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
    assert_eq!(problems, Vec::<String>::new());
    Some(game)
}

#[test]
fn the_samples_longest_course_is_planned_and_flown_by_hand_gate_by_gate() {
    let Some(game) = sample() else { return };
    let rules = new_game_rules();
    let [ninefold, luyten, marrow, lantern] =
        ["ninefold", "luyten-crossing", "marrow", "lantern"].map(|name| system(&game, name));
    let mut session = flying_from(&game, &rules, ninefold);
    assert_eq!(
        session.plot_course(lantern),
        Ok(&[luyten, marrow, lantern][..]),
        "3 hops"
    );
    assert_eq!(
        fly_by_hand(&game, &rules, &mut session),
        [ninefold, luyten, marrow, lantern]
    );
}

#[test]
fn the_samples_longest_course_is_flown_on_the_autopilot_gate_by_gate() {
    let Some(game) = sample() else { return };
    let rules = new_game_rules();
    let [ninefold, luyten, marrow, lantern] =
        ["ninefold", "luyten-crossing", "marrow", "lantern"].map(|name| system(&game, name));
    let mut session = flying_from(&game, &rules, ninefold);
    assert_eq!(
        session.plot_course(lantern),
        Ok(&[luyten, marrow, lantern][..]),
        "3 hops"
    );
    assert_eq!(
        fly_on_autopilot(&game, &rules, &mut session),
        [ninefold, luyten, marrow, lantern]
    );
    assert_eq!(session.course(), []);
}

#[test]
fn each_of_the_samples_lanes_is_drawn_in_its_look() {
    let Some(game) = sample() else { return };
    let session = flying_from(&game, &new_game_rules(), system(&game, "marrow"));
    let both = |stability| (Some(stability), Back::Way(Some(stability)));
    for (a, b, look) in [
        ("brightwater", "luyten-crossing", both(Stable)),
        ("brightwater", "marrow", both(Stable)),
        ("harrow-drift", "marrow", both(Stable)),
        ("kestrel-march", "luyten-crossing", both(Stable)),
        ("luyten-crossing", "ninefold", both(Stable)),
        ("luyten-crossing", "marrow", both(Stable)),
        ("marrow", "lantern", both(Stable)),
        ("harrow-drift", "kestrel-march", both(Dead)),
        ("harrow-drift", "lantern", (Some(Unstable), Back::OneWay)),
    ] {
        let drawn = look_between(&session, system(&game, a), system(&game, b));
        assert_eq!((drawn.out, drawn.back), look, "{a}–{b}");
    }
    let lantern = look_between(
        &session,
        system(&game, "lantern"),
        system(&game, "harrow-drift"),
    );
    assert_eq!(lantern.out, None, "no lane from lantern to harrow-drift");
}
