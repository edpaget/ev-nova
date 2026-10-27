use super::*;

// Landing.

#[test]
fn the_picks_roll_their_ranges_and_decide_only_their_words() {
    // A planet: Rand(3), then Rand(2); 0 names the stellar, and 0
    // adds the ship.
    for (draws, opening, named) in [([0, 0], 0, 0), ([2, 1], 2, 1), ([1, 0], 1, 0)] {
        let (script, mut chance) = scripted(&draws);
        let picks = RequestPicks::roll(DockKind::Planet, &mut chance);
        assert_eq!(picks, RequestPicks { opening, named });
        assert_eq!(script.borrow().asked, [3, 2]);
    }
    let (script, mut chance) = scripted(&[1, 1]);
    assert_eq!(
        RequestPicks::roll(DockKind::Hypergate, &mut chance),
        RequestPicks {
            opening: 1,
            named: 1
        }
    );
    assert_eq!(script.borrow().asked, [2, 2], "a hypergate's Rand(2)");
    let (script, mut chance) = scripted(&[2, 1]);
    assert_eq!(
        GreetingPicks::roll(DockKind::Station, &mut chance),
        GreetingPicks {
            opening: 2,
            welcome: 1
        }
    );
    assert_eq!(script.borrow().asked, [3, 2]);
    let (script, mut chance) = scripted(&[1]);
    assert_eq!(
        GreetingPicks::roll(DockKind::Hypergate, &mut chance).opening,
        1
    );
    assert_eq!(script.borrow().asked, [3], "no second sentence");
    // Only 0 names, only 0 adds the ship, and anything but 0 welcomes.
    let line = |opening, named| {
        request_line(
            DockKind::Planet,
            "Earth",
            "Kestrel",
            RequestPicks { opening, named },
        )
    };
    assert!(line(0, 1).starts_with("Earth "));
    assert!(!line(1, 1).contains("Earth"));
    assert!(!line(2, 1).contains("Earth"));
    assert!(line(1, 0).contains(", Kestrel"));
    assert!(!line(1, 1).contains("Kestrel"));
    let greeting = |welcome| {
        approach_greeting(
            DockKind::Planet,
            "Earth",
            "Kestrel",
            GreetingPicks {
                opening: 2,
                welcome,
            },
            None,
            nova_sim::RuleSource::Engine,
        )
    };
    assert!(greeting(0).ends_with(COMMENCE_FINAL_APPROACH));
    assert!(greeting(1).ends_with("Welcome to Earth. "));
    assert!(greeting(2).ends_with("Welcome to Earth. "));
}

#[test]
fn a_repeat_or_release_of_l_does_nothing() {
    let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
    view.input(&held(LAND));
    view.input(&key(LAND, false));
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.session().expect("flying").landed(), None);
    assert_eq!(nav_target(&view), None, "nothing selected");
    let mut far = flight();
    far.input(&held(LAND));
    assert_eq!(far.message(), None, "no refusal either");
}

#[test]
fn a_message_waits_out_the_map() {
    let mut view = flight();
    view.say_for("A", 240);
    ticks(&mut view, 40);
    tap(&mut view, MAP_KEY);
    assert!(view.map_open());
    ticks(&mut view, 300);
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    assert_lasts(&mut view, "A", 200);
}
