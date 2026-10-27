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
fn the_selecting_l_shows_the_requests_line_then_a_tick_near_shows_the_greeting() {
    // The Moon (129) at (300, −200), the nearer of Sol's two.
    let mut view = flight();
    let ship = ship_name(&view);
    tap(&mut view, LAND);
    assert_eq!(nav_target(&view), Some(StellarId(129)));
    let line = format!("Moon traffic control reads you, {ship}. Begin initial approach.");
    assert_eq!(view.message(), Some(line.as_str()));
    assert_lasts(&mut view, &line, 250);
    // Flown within 249 pixels, the approach greets on that tick, as a
    // message of its own.
    let mut view = flight_among(vec![site(129, (0.0, -300.0), StellarFlags::CAN_LAND)]);
    tap(&mut view, LAND);
    let line = format!("Moon traffic control reads you, {ship}. Begin initial approach.");
    assert_eq!(view.message(), Some(line.as_str()));
    view.input(&key(Key::Up, true));
    let greeting = format!("Cleared to land, {ship}. Commence final approach.");
    let mut far = true;
    for _ in 0..200 {
        let before = view.session().expect("flying").player().position;
        ticks(&mut view, 1);
        if view.message() == Some(greeting.as_str()) {
            let at = view.session().expect("flying").player().position;
            assert!((at.y + 300.0).trunc().abs() <= 249.0, "{at:?}");
            assert!((before.y + 300.0).trunc().abs() > 249.0, "{before:?}");
            far = false;
            break;
        }
        assert_eq!(
            view.message(),
            Some(line.as_str()),
            "{before:?} {greeting:?}"
        );
    }
    assert!(!far, "never greeted");
    view.input(&key(Key::Up, false));
    assert_lasts(&mut view, &greeting, 250);
}

/// Over a stellar charging 1,000 credits with `flags2`, the pilot holding
/// as much: L, a tick, and the greeting shown.
fn greeted_at_a_fee(flags2: u16) -> (View, String) {
    let site = LandingSite {
        fee: 1000,
        flags2,
        ..site(129, (6.0, -8.0), StellarFlags::CAN_LAND)
    };
    let mut view = FlightView::new(FakeCatalog {
        sites: vec![site],
        character: Ok(CharacterStart {
            cash: 1000,
            ..catalog().character.expect("a chär")
        }),
        ..catalog()
    });
    tap(&mut view, LAND);
    ticks(&mut view, 1);
    let greeting = view.message().expect("greeted").to_owned();
    (view, greeting)
}

#[test]
fn the_greeting_at_a_stellar_with_a_fee_states_it() {
    let (mut view, greeting) = greeted_at_a_fee(0);
    let ship = ship_name(&view);
    assert_eq!(
        greeting,
        format!(
            "Cleared to land, {ship}. Commence final approach.  [Landing fee is 1,000 credits.]"
        )
    );
    assert_lasts(&mut view, &greeting, 250);
}

#[test]
fn a_dominated_stellars_or_a_hypergates_greeting_states_no_fee() {
    // Domination waives it (@0x64d92-0x64d96).
    let (view, greeting) = greeted_at_a_fee(nova_sim::market::DOMINATED);
    let ship = ship_name(&view);
    assert_eq!(
        greeting,
        format!("Cleared to land, {ship}. Commence final approach.")
    );
    let (_, gate) = greeted_at_a_fee(nova_sim::gate::HYPERGATE);
    assert!(gate.starts_with("Cleared for hypergate entry"), "{gate}");
    assert!(!gate.contains('['), "{gate}");
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

/// The messages that name a stellar name one the scene doesn't hold by
/// its spöb name too, as the nav readout and the refusals do: a dock
/// request and a land hint at "Hole", which another system owns.
#[test]
fn a_dock_request_and_a_land_hint_name_an_unseen_stellar_by_its_spob_name() {
    let view = flight_with_unseen(site(150, (0.0, 300.0), StellarFlags::CAN_LAND));
    let session = view.session().expect("flying");
    let (_, mut chance) = scripted(&[0, 0]);
    let request = SimMessage::DockRequested {
        stellar: StellarId(150),
        kind: DockKind::Planet,
    };
    let text = message_text(
        &request,
        session,
        view.scene.as_ref(),
        &mut chance,
        RuleSource::Engine,
    )
    .expect("a dock request is worded");
    assert!(text.contains("Hole"), "{text}");
    let hint = SimMessage::Tutorial(nova_sim::TutorialHint::Land {
        stellar: StellarId(150),
        station: false,
    });
    assert_eq!(
        message_text(
            &hint,
            session,
            view.scene.as_ref(),
            &mut chance,
            RuleSource::Engine
        ),
        Some(land_hint(false, "Hole", "L"))
    );
}
