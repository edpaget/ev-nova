use super::*;

// Landing.

#[test]
fn l_requests_clearance_an_early_second_l_beeps_too_far_and_a_later_l_lands() {
    // The Moon (129), in the scene, over the ship.
    let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
    assert_eq!(view.take_landing(), None, "requested, not landed");
    assert_eq!(view.session().expect("flying").landed(), None);
    assert_eq!(nav_target(&view), Some(StellarId(129)));
    assert_eq!(view.message(), None, "requested over it: silent");
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"], "the HUD shows it");
    assert_eq!(view.take_sounds(), [], "no sound for a silent request");
    assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
    assert_eq!(view.take_landing(), None, "no approach yet");
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "too far before the approach clears: _beepSnd[3] (@0x66c23 -> 0x676db, @0x676fe)"
    );

    view.input(&key(LAND, false));
    ticks(&mut view, 1);
    let ship = ship_name(&view);
    let cleared = format!("Cleared to land, {ship}. Commence final approach.");
    assert_eq!(view.message(), Some(cleared.as_str()));
    assert_eq!(message(&view), Some(message_drawn(&cleared)));
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TrafficControl)]);
    assert_eq!(view.input(&key(LAND, true)), ScreenAction::None);
    assert_eq!(
        view.session().expect("flying").landed(),
        Some(StellarId(129))
    );
    assert_eq!(view.take_landing(), Some(StellarId(129)));
    assert_eq!(view.take_landing(), None, "taken");
    assert_eq!(view.message(), None);
    assert_eq!(message(&view), None);
    assert_eq!(LAND_KEY, LAND);
}

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

#[test]
fn holding_l_holds_the_greeting_back_until_it_is_let_go() {
    let mut view = flight_among(vec![site(129, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    view.input(&key(LAND, true));
    assert_eq!(view.message(), None, "requested over it: silent");
    ticks(&mut view, 5);
    assert_eq!(view.message(), None, "held");
    view.input(&key(LAND, false));
    ticks(&mut view, 1);
    let ship = ship_name(&view);
    assert_eq!(
        view.message(),
        Some(format!("Cleared to land, {ship}. Commence final approach.").as_str())
    );
}

#[test]
fn an_uninhabited_stellar_out_of_reach_says_too_far_over_no_response() {
    let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
    let mut view = flight_among(vec![site(128, (0.0, -900.0), uninhabited)]);
    tap(&mut view, LAND);
    assert_lasts(&mut view, TOO_FAR_PLANET, TOO_FAR_TICKS);
    let mut view = flight_among(vec![site(128, (0.0, 0.0), uninhabited)]);
    tap(&mut view, LAND);
    assert_eq!(view.take_landing(), Some(StellarId(128)), "one press");
}

#[test]
fn a_denial_says_so_and_selects_anyway() {
    let strict = LandingSite {
        min_status: 1,
        ..site(
            129,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::STATION,
        )
    };
    let mut view = flight_among(vec![strict]);
    tap(&mut view, LAND);
    assert_eq!(nav_target(&view), Some(StellarId(129)), "selected anyway");
    assert_lasts(&mut view, "Docking request denied.", 250);
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
fn each_refusal_says_why_in_the_originals_words() {
    // L, then L again: the first requests clearance where it can.
    let refused = |sites: Vec<LandingSite>| {
        let mut view = flight_among(sites);
        tap(&mut view, LAND);
        tap(&mut view, LAND);
        assert_eq!(view.take_landing(), None);
        view.message().map(str::to_owned)
    };
    let centre = |flags, min_status| LandingSite {
        min_status,
        ..site(140, (0.0, 0.0), flags)
    };
    let station = StellarFlags::CAN_LAND | StellarFlags::STATION;
    assert_eq!(
        refused(vec![site(140, (0.0, -600.0), StellarFlags::CAN_LAND)]).as_deref(),
        Some("You're too far away to land on this planet.")
    );
    assert_eq!(
        refused(vec![site(140, (0.0, -600.0), station)]).as_deref(),
        Some("You're too far away to dock at this station.")
    );
    assert_eq!(
        refused(vec![centre(StellarFlags::CAN_LAND, 1)]).as_deref(),
        Some("Landing request denied.")
    );
    assert_eq!(
        refused(vec![centre(station, 32767)]).as_deref(),
        Some("Docking request denied.")
    );
    // Only a selected stellar reaches the dock request's not-landable
    // line (@0x6a945-0x6a94c, @0x66977-0x669ae).
    let mut view = flight_among(vec![centre(0, 0)]);
    tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
    tap(&mut view, LAND);
    assert_eq!(nav_target(&view), Some(StellarId(140)), "kept");
    assert_eq!(
        view.message(),
        Some("Your ship is unable to land on  . The planet's environment is too hostile."),
        "140 is not in Sol's scene: no name"
    );
}

/// Says `earlier` in `view`, taps L, and asserts the message line
/// still says `earlier`: the press said nothing and cleared nothing.
#[track_caller]
fn l_says_nothing(view: &mut View) {
    view.say_for("earlier", MESSAGE_TICKS);
    tap(view, LAND);
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.message(), Some("earlier"), "unchanged");
}

#[test]
fn l_with_no_stellars_says_nothing() {
    // Nothing to select, so no dock request (@0x6a94c).
    let mut view = flight_among(Vec::new());
    l_says_nothing(&mut view);
}

#[test]
fn l_with_nothing_landable_and_no_target_says_nothing() {
    // `_FindNearestStellar` gives -1: no dock request (@0x6a94c).
    let mut view = flight_among(vec![site(140, (0.0, 0.0), 0)]);
    l_says_nothing(&mut view);
    assert_eq!(nav_target(&view), None, "nothing selected");
    let mut fresh = flight_among(vec![site(140, (0.0, 0.0), 0)]);
    tap(&mut fresh, LAND);
    assert_eq!(fresh.message(), None, "no line either");
}

#[test]
fn l_during_a_jump_says_nothing() {
    // The jump words send the land key to 0x6a95d (@0x6a8cb-0x6a8e3).
    for mut view in through_a_jump(flight) {
        l_says_nothing(&mut view);
    }
}

#[test]
fn a_ship_moving_too_fast_is_told_so() {
    let big = |flags| LandingSite {
        frame_size: Some((400, 400)),
        ..site(140, (0.0, 0.0), flags)
    };
    for (flags, expected) in [
        (
            StellarFlags::CAN_LAND,
            "You're moving too fast to land on this planet.",
        ),
        (
            StellarFlags::CAN_LAND | StellarFlags::STATION,
            "You're moving too fast to dock at this station.",
        ),
    ] {
        let mut view = flight_among(vec![big(flags)]);
        view.input(&key(Key::Up, true));
        // 25 ticks of `Accel` / 5000, the speed 15 of the port's former
        // 3000 gave.
        ticks(&mut view, 25);
        land_now(&mut view);
        assert_eq!(view.take_landing(), None);
        assert_eq!(view.message(), Some(expected));
        assert_eq!(nav_target(&view), Some(StellarId(140)), "kept: try again");
    }
}

#[test]
fn a_new_message_replaces_the_old_and_its_time() {
    let mut view = flight();
    view.say_for("A", 420);
    ticks(&mut view, 10);
    view.say_for("B", 240);
    assert_lasts(&mut view, "B", 240);
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

#[test]
fn starting_a_jump_clears_only_too_close() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, JUMP);
    assert_eq!(view.message(), Some(TOO_CLOSE));
    fly_out(&mut view);
    assert_eq!(view.message(), Some(TOO_CLOSE), "still shown");
    tap(&mut view, JUMP);
    assert!(view.preparing_jump(), "accepted");
    assert_eq!(view.message(), None, "#42 cleared");

    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, JUMP);
    assert!(view.preparing_jump(), "accepted");
    assert_eq!(view.message(), Some(MISSION_FAILED), "kept");
}

#[test]
fn too_far_at_a_hypergate_or_wormhole_says_so_for_360_ticks() {
    for (flags2, text) in [(0x1000, TOO_FAR_HYPERGATE), (0x2000, TOO_FAR_WORMHOLE)] {
        let mut view = flight_among(vec![LandingSite {
            flags2,
            ..site(128, (0.0, -600.0), StellarFlags::CAN_LAND)
        }]);
        tap(&mut view, LAND);
        tap(&mut view, LAND);
        assert_lasts(&mut view, text, TOO_FAR_TICKS);
    }
}

#[test]
fn too_fast_at_a_hypergate_or_wormhole_says_so_for_360_ticks() {
    for (flags2, text) in [(0x1000, TOO_FAST_HYPERGATE), (0x2000, TOO_FAST_WORMHOLE)] {
        let mut view = flight_among(vec![LandingSite {
            flags2,
            frame_size: Some((400, 400)),
            ..site(128, (0.0, 0.0), StellarFlags::CAN_LAND)
        }]);
        view.input(&key(Key::Up, true));
        // 25 ticks of `Accel` / 5000, the speed 15 of the port's former
        // 3000 gave.
        ticks(&mut view, 25);
        land_now(&mut view);
        assert_eq!(view.take_landing(), None);
        assert_lasts(&mut view, text, TOO_FAR_TICKS);
    }
}

#[test]
fn a_not_landable_station_or_planet_is_named_for_250_ticks() {
    for (flags, text) in [
        (
            StellarFlags::STATION,
            "Your ship is unable to dock at  Earth. The station's hull integrity is too unstable.",
        ),
        (
            0,
            "Your ship is unable to land on  Earth. The planet's environment is too hostile.",
        ),
    ] {
        // Selected: only a selected stellar reaches the dock request
        // (@0x6a945-0x6a94c).
        let mut view = flight_among(vec![site(128, (0.0, 0.0), flags)]);
        tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
        tap(&mut view, LAND);
        assert_lasts(&mut view, text, LANDING_TICKS);
    }
}

#[test]
fn the_refusal_is_drawn_until_its_ticks_have_passed() {
    let mut view = flight();
    ticks(&mut view, 3);
    land_now(&mut view);
    let shown = Some(message_drawn(TOO_FAR_PLANET));
    assert_eq!(message(&view), shown);
    ticks(&mut view, 359);
    assert_eq!(message(&view), shown);
    assert_eq!(view.message(), Some(TOO_FAR_PLANET));
    ticks(&mut view, 1);
    assert_eq!(message(&view), None);
    assert_eq!(view.message(), None);
    assert_eq!(TOO_FAR_TICKS, 360);
}

#[test]
fn the_message_draws_over_the_starfield_and_under_the_ships() {
    let mut view = flight();
    land_now(&mut view);
    let list = drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let mut stars = DrawList::new();
    starfield::draw(&mut stars, &view.camera());
    let stars: Vec<&DrawCommand> = stars.iter().collect();
    assert!(!stars.is_empty());
    assert_eq!(commands[..stars.len()], stars[..], "the starfield first");
    assert_eq!(
        commands[stars.len()],
        &message_drawn(TOO_FAR_PLANET),
        "then the message"
    );
    let sprite = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Sprite { .. }))
        .expect("the player's sprite");
    assert!(stars.len() < sprite, "the ship over it");
}

#[test]
fn a_long_message_wraps_in_the_box_by_the_metrics() {
    let mut view = flight().with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
    land_now(&mut view);
    let text = vec!["abcd"; 60].join(" ");
    view.say_for(text.clone(), MESSAGE_TICKS);
    let mut expected = DrawList::new();
    hud::draw_message(
        &mut expected,
        &text,
        VIEW_SIZE,
        Some(&crate::text::fixture::MonoMetrics),
    );
    let list = drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let first = commands
        .iter()
        .position(|c| matches!(c, DrawCommand::Text { origin, .. } if *origin == message_at()))
        .expect("the message");
    let expected: Vec<&DrawCommand> = expected.iter().collect();
    assert_eq!(expected.len(), 3, "two whole lines and a clipped third");
    assert_eq!(commands[first..first + 3], expected[..]);
}

#[test]
fn a_new_refusal_shows_afresh() {
    let mut view = flight();
    land_now(&mut view);
    assert_eq!(view.message(), Some(TOO_FAR_PLANET));
    view.input(&key(LAND, true));
    ticks(&mut view, 300);
    view.input(&key(LAND, false));
    view.input(&key(LAND, true));
    ticks(&mut view, 100);
    assert_eq!(
        view.message(),
        Some(TOO_FAR_PLANET),
        "360 ticks from the second"
    );
}

#[test]
fn taking_off_draws_the_ship_at_the_stellar_on_the_next_frame() {
    let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(140)));
    assert_eq!(view.take_off(), Some(StellarId(140)));
    assert_eq!(view.session().expect("flying").landed(), None);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(6.0, -8.0));
    assert_eq!(view.camera().center(), at(6.0, -8.0));
    assert_eq!(view.take_off(), None, "not landed");
    // It flies on from there.
    view.input(&key(Key::Up, true));
    ticks(&mut view, 2);
    assert!(player(&view).position.y < -8.0, "{:?}", player(&view));
}

#[test]
fn a_refusal_is_forgotten_once_the_ship_lands() {
    // 20 wide: within 35 on each axis is over it.
    let mut view = flight_among(vec![site(140, (0.0, -40.0), StellarFlags::CAN_LAND)]);
    land_now(&mut view);
    assert_eq!(view.message(), Some(TOO_FAR_PLANET));
    // Nudge the ship and drift over the planet.
    view.input(&key(Key::Up, true));
    ticks(&mut view, 1);
    view.input(&key(Key::Up, false));
    for _ in 0..100 {
        if player(&view).position.y <= -6.0 {
            break;
        }
        ticks(&mut view, 1);
    }
    assert!(player(&view).position.y <= -6.0, "{:?}", player(&view));
    assert_eq!(view.message(), Some(TOO_FAR_PLANET), "still on screen");
    view.input(&key(LAND, true));
    assert_eq!(view.take_landing(), Some(StellarId(140)));
    assert_eq!(view.message(), None);
    assert_eq!(message(&view), None);
    assert_eq!(view.take_off(), Some(StellarId(140)));
    let greeting = view.message().expect("the greeting");
    assert!(greeting.starts_with(TAKE_OFF_LEADS[0]), "{greeting}");
}

#[test]
fn landing_in_a_flight_that_never_started_does_nothing() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    view.input(&key(LAND, true));
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.message(), None);
    assert_eq!(view.take_off(), None);
}

/// Only the landing beep sounds at touchdown: the stellar's own sound
/// is its spaceport's ambient sound, which plays once the spaceport is
/// frontmost (see the spaceport's
/// `the_ambient_sound_plays_on_the_first_frame_then_after_each_wait`).
#[test]
fn l_over_a_stellar_sounds_only_the_landing_at_touchdown() {
    let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    assert_eq!(view.take_sounds(), []);
    tap(&mut view, LAND);
    assert_eq!(view.take_sounds(), [], "a request over it is silent");
    ticks(&mut view, 1);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::TrafficControl)],
        "the greeting"
    );
    tap(&mut view, LAND);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Landed)]);
    assert_eq!(view.take_sounds(), [], "taken");
    view.take_off();
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TookOff)]);
}

#[test]
fn m_i_and_p_are_refused_for_15_ticks_after_a_take_off() {
    let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(140)));
    assert_eq!(view.take_off(), Some(StellarId(140)));
    let _ = view.take_sounds();
    for tick in 0..15 {
        tap(&mut view, MAP_KEY);
        assert!(!view.map_open(), "after {tick} ticks");
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(view.take_mission_info_request(), None, "after {tick} ticks");
        assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS), "silently");
        tap(&mut view, PLAYER_INFO_KEY);
        assert!(!view.take_player_info_request(), "after {tick} ticks");
        assert_eq!(
            view.take_sounds(),
            [Sound::Sim(SimSound::Refused)],
            "after {tick} ticks"
        );
        ticks(&mut view, 1);
    }
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS), "after 15 ticks");
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "no active missions, with the beep (@0x70557)"
    );
    tap(&mut view, PLAYER_INFO_KEY);
    assert!(view.take_player_info_request(), "after 15 ticks");
    tap(&mut view, MAP_KEY);
    assert!(view.map_open(), "after 15 ticks");
}

#[test]
fn thrust_sounds_as_it_starts_and_stops() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStarted)]);
    view.input(&key(Key::Up, false));
    ticks(&mut view, 1);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::ThrustStopped)]);
}

#[test]
fn a_flight_that_never_started_makes_no_sounds() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(view.take_sounds(), []);
}
