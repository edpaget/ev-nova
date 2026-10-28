use super::*;

// Hyperspace.

#[test]
fn m_opens_the_map_over_everything_and_lets_go_of_the_flight_keys() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    assert_eq!(view.input(&key(MAP, true)), ScreenAction::None);
    assert!(view.map_open());
    let mut map = DrawList::new();
    view.course_map().draw(&mut map);
    let list = drawn(&view);
    let over = list.len() - map.len();
    assert_eq!(
        list.iter().skip(over).collect::<Vec<_>>(),
        map.iter().collect::<Vec<_>>(),
        "the map over everything"
    );
    // Holding M, or letting it go, keeps it open.
    view.input(&held(MAP));
    view.input(&key(MAP, false));
    assert!(view.map_open());
    view.input(&key(MAP, true));
    assert!(!view.map_open());
    ticks(&mut view, 5);
    assert_eq!(player(&view), start(), "Up was let go");
}

#[test]
fn while_the_map_is_open_flight_is_paused_and_keys_go_to_the_map() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    let flying = player(&view);
    tap(&mut view, MAP);
    let fitted = view.course_map().transform();
    ticks(&mut view, 30);
    assert_eq!(player(&view), flying, "paused");
    view.input(&key(Key::Left, true));
    assert_ne!(
        view.course_map().transform(),
        fitted,
        "Left scrolls the map"
    );
    view.input(&key(LAND, true));
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), None);
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.jump_effect(), None);
    tap(&mut view, MAP);
    ticks(&mut view, 1);
    assert_eq!(player(&view).heading, 0.0, "Left went to the map");
}

/// The land hint as the fixture words it, for the Moon, the nearest
/// stellar to the centre.
const MOON_HINT: &str = "Welcome to Nova - it would be a good idea to start by landing on Moon and having a look around. Hit 'L' to request landing clearance, then hit it again to land.";

#[test]
fn the_land_hint_words_a_station_as_docking() {
    assert_eq!(
        land_hint(true, "Earth Station", "L"),
        "Welcome to Nova - it would be a good idea to start by docking at Earth Station and having a look around. Hit 'L' to request landing clearance, then hit it again to land."
    );
    assert_eq!(land_hint(false, "Moon", "L"), MOON_HINT);
}

#[test]
fn the_hints_name_letter_keys_as_str_129_does() {
    assert_eq!(key_label(LAND_KEY).as_deref(), Some("L"));
    assert_eq!(key_label(MAP_KEY).as_deref(), Some("M"));
    assert_eq!(key_label(JUMP_KEY).as_deref(), Some("J"));
    assert_eq!(key_label(Key::Function(5)), None, "not yet named");
}

#[test]
fn a_repeat_or_release_of_j_does_nothing() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&held(JUMP));
    view.input(&key(JUMP, false));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), None);
}

#[test]
fn a_real_flight_meets_the_contract_through_a_jump() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    meets_the_contract(&view, "ready, the check's J begun");
    tap(&mut view, JUMP);
    assert!(view.frame.jump_under_way && !view.frame.warping);
    meets_the_contract(&view, "preparing");
    jump_now(&mut view);
    assert!(view.frame.warping);
    meets_the_contract(&view, "warping");
    let mut flight = view.sim.clone();
    let mut frame = nova_sim::flight_frame::FlightFrame::default();
    let mut report = nova_sim::flight_frame::StepReport::default();
    for _ in 0..2000 {
        flight.frame(&mut frame);
        if frame.warp_end_due {
            break;
        }
        flight.step(
            &nova_sim::flight_frame::StepInput::default(),
            1,
            &mut report,
        );
    }
    assert!(frame.warp_end_due, "the warp ended");
    assert_eq!(
        crate::flight::sim_port::fixture::check_contract(&mut flight),
        Ok(()),
        "at the warp's end"
    );
}

/// Whether `list` draws the fade quad translucent: part of a fade.
fn translucent_fade(list: &DrawList) -> bool {
    fade(list).is_some_and(|(_, color)| color.a < u8::MAX)
}

#[test]
fn keys_are_ignored_while_the_ship_turns_to_jump() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    let pressed = player(&view);
    let mut alongside = flight();
    plot(&mut alongside, 131);
    fly_out(&mut alongside);
    alongside.input(&key(JUMP, true));
    for k in [Key::Up, Key::Left, Key::Down, MAP, LAND, JUMP] {
        assert_eq!(view.input(&key(k, true)), ScreenAction::None);
    }
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    for _ in 0..10 {
        view.tick(TICK);
        alongside.tick(TICK);
    }
    assert_eq!(player(&view), player(&alongside), "flown alike");
    assert_ne!(player(&view), pressed, "the session flies on");
    assert_eq!(view.jump_effect(), None);
    let session = view.session().expect("flying");
    assert_eq!(session.preparing_jump(), Some(SystemId(131)));
    assert_eq!(session.nav_target(), None);
    assert_eq!(view.take_landing(), None);
}

#[test]
fn keys_are_ignored_while_the_jump_plays() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(Key::Up, true));
    jump_now(&mut view);
    view.tick(ms(300));
    view.input(&key(Key::Up, false));
    view.input(&key(Key::Left, true));
    view.input(&key(MAP, true));
    view.input(&key(LAND, true));
    view.input(&key(JUMP, true));
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    view.input(&key(Key::Left, false));
    arrive_now(&mut view);
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.take_landing(), None);
    let arrived = player(&view);
    ticks(&mut view, 5);
    assert_eq!(
        player(&view),
        stepped(arrived, Controls::default(), 5),
        "neither thrusting nor turning"
    );
}

#[test]
fn keys_are_ignored_while_the_old_system_fades_out() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    tick_until_lit(&mut view);
    view.tick(ms(500));
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(_))
    ));
    for k in [Key::Up, Key::Left, MAP, LAND, JUMP] {
        assert_eq!(view.input(&key(k, true)), ScreenAction::None);
        assert_eq!(view.input(&key(k, false)), ScreenAction::None);
    }
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    assert_eq!(view.take_landing(), None);
    arrive_now(&mut view);
    let arrived = player(&view);
    ticks(&mut view, 5);
    assert_eq!(
        player(&view),
        stepped(arrived, Controls::default(), 5),
        "neither thrusting nor turning"
    );
    assert_eq!(view.session().expect("flying").preparing_jump(), None);
}

#[test]
fn the_target_keys_are_taken_through_a_jump() {
    for mut view in through_a_jump(|| fighting(armed(2, &[]), &two_npcs())) {
        let stage = stage(&view);
        assert_eq!(target_of(&view), None, "{stage}");
        tap(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}");
        tap(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), Some(NpcId(1)), "{stage}: steps on");
        view.input(&key(Key::Shift, true));
        tap(&mut view, TARGET_KEY);
        view.input(&key(Key::Shift, false));
        assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}: Shift-Tab");
        tap(&mut view, TARGET_KEY);
        tap(&mut view, TARGET_KEY);
        assert_eq!(target_of(&view), None, "{stage}: past the last");
        let session = view.session().expect("flying");
        let from = session.player().position;
        let nearest = session
            .npcs()
            .iter()
            .min_by(|a, b| {
                let a = (a.state.position - from).length();
                let b = (b.state.position - from).length();
                a.total_cmp(&b)
            })
            .map(|npc| npc.id);
        alt_tap(&mut view, NEAREST_KEY);
        assert_eq!(target_of(&view), nearest, "{stage}: Option-R");
        assert!(nearest.is_some());
    }
}

#[test]
fn option_backquote_clears_the_target_through_a_jump_and_nav_off_is_ignored() {
    let targeted = || {
        let mut view = fighting(armed(2, &[]), &two_npcs());
        tap(&mut view, TARGET_KEY);
        view
    };
    for mut view in through_a_jump(targeted) {
        let stage = stage(&view);
        let mode = nav_mode(&view);
        let _ = view.take_sounds();
        tap(&mut view, NAV_OFF_KEY);
        assert_eq!(nav_mode(&view), mode, "{stage}: Nav Off ignored");
        assert_eq!(target_of(&view), Some(NpcId(0)), "{stage}");
        alt_tap(&mut view, NAV_OFF_KEY);
        assert_eq!(target_of(&view), None, "{stage}: Option-`");
        assert_eq!(nav_mode(&view), mode, "{stage}");
        assert!(
            view.take_sounds()
                .contains(&Sound::Sim(SimSound::TargetCleared)),
            "{stage}"
        );
    }
}

#[test]
fn weapon_select_and_safety_are_taken_through_a_jump() {
    for mut view in through_a_jump(|| fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[])) {
        let stage = stage(&view);
        assert_eq!(secondary_of(&view), None, "{stage}");
        tap_and_step(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(ROCKET), "{stage}");
        tap_and_step(&mut view, SELECT_KEY);
        assert_eq!(secondary_of(&view), Some(MISSILE), "{stage}");
        alt_tap(&mut view, SELECT_KEY);
        ticks(&mut view, 1);
        assert_eq!(secondary_of(&view), Some(ROCKET), "{stage}: Option-W");
        view.input(&key(DESELECT_KEY, true));
        ticks(&mut view, 1);
        assert_eq!(secondary_of(&view), None, "{stage}: Weapon Safety");
    }
}

#[test]
fn board_is_taken_through_a_jump() {
    for mut view in through_a_jump(|| beside(750, 750, 0, true)) {
        let stage = stage(&view);
        tap(&mut view, BOARD_KEY);
        assert_eq!(view.take_boarding(), None, "{stage}");
        assert_eq!(view.message(), Some(CANT_BOARD), "{stage}");
    }
}

#[test]
fn hail_is_refused_silently_through_a_jump() {
    for mut view in through_a_jump(|| hailable(true)) {
        let stage = stage(&view);
        tap(&mut view, HAIL_KEY);
        assert_eq!(view.take_hail(), None, "{stage}");
        assert_eq!(view.take_offer(), None, "{stage}");
        assert_eq!(view.message(), None, "{stage}");
    }
}

/// [`trading`]'s flight, its hold bought full of food and taken off.
fn laden() -> View {
    let mut view = test_flight::new(trading());
    land_now(&mut view);
    view.settle_script();
    assert!(view.trade(BUY_FOOD).is_ok());
    view.take_off().expect("takes off");
    assert!(cargo_aboard(&view) > 0);
    // Past the take-off grace, so the map opens for the course.
    ticks(&mut view, nova_sim::TAKEOFF_GRACE_TICKS);
    view.message = None;
    view
}

#[test]
fn command_k_jettisons_through_a_jump() {
    for mut view in through_a_jump(laden) {
        let stage = stage(&view);
        command_tap(&mut view, JETTISON_KEY, false);
        assert_eq!(cargo_aboard(&view), 0, "{stage}");
        assert_eq!(view.message(), Some(CARGO_JETTISONED), "{stage}");
    }
}

/// Whether [`FlightView::taken_during_jump`] takes `key` with Option
/// held or not and the escort menu open or shut, each of the four.
fn taken_with(key: Key) -> [bool; 4] {
    [(false, false), (true, false), (false, true), (true, true)]
        .map(|(option, menu_open)| View::taken_during_jump(key, option, menu_open))
}

#[test]
fn the_keys_a_jump_takes_are_the_originals_ungated_ones() {
    let always = [
        TARGET_KEY,
        NEAREST_KEY,
        SELECT_KEY,
        BOARD_KEY,
        HAIL_KEY,
        JETTISON_KEY,
        MENU_KEY,
        MENU_CLOSE_KEY,
        ESCORT_ATTACK_KEY,
        ESCORT_DEFEND_KEY,
        ESCORT_HOLD_KEY,
        ESCORT_RECALL_KEY,
        ZOOM_OUT_KEY,
        ZOOM_IN_KEY,
        ZOOM_IN_KEYPAD_KEY,
    ];
    for key in always {
        assert_eq!(taken_with(key), [true; 4], "{key:?}");
    }
    assert_eq!(
        taken_with(NAV_OFF_KEY),
        [false, true, false, true],
        "with Option"
    );
    for key in GROUP_KEYS {
        assert_eq!(
            taken_with(key),
            [false, false, true, true],
            "{key:?}: menu open"
        );
    }
    let never = [
        LAND_KEY,
        MAP_KEY,
        JUMP_KEY,
        MISSION_INFO_KEY,
        PLAYER_INFO_KEY,
        HYPERSPACE_MODE_KEY,
        HYPER_SELECT_KEY,
        Key::Function(1),
        Key::Function(4),
        NEAREST_STELLAR_KEY,
        Key::Up,
        FIRE_KEY,
    ];
    for key in never {
        assert_eq!(taken_with(key), [false; 4], "{key:?}");
    }
}

#[test]
fn nav_and_screen_keys_are_ignored_through_a_jump() {
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        let braking = view.preparing_jump();
        let mode = nav_mode(&view);
        let course = view.session().expect("flying").course().to_vec();
        for k in [
            LAND,
            MAP,
            JUMP,
            MISSION_INFO_KEY,
            HYPERSPACE_MODE_KEY,
            HYPER_SELECT_KEY,
            NAV_OFF_KEY,
            STELLAR_SLOT_KEYS[0][0],
            STELLAR_SLOT_KEYS[0][1],
            NEAREST_STELLAR_KEY,
        ] {
            tap(&mut view, k);
        }
        command_tap(&mut view, NEAREST_STELLAR_COMMAND_KEY, false);
        // The slot block's frame, still in the jump, takes nothing.
        view.tick(TICK);
        assert!(view.jump_under_way(), "{stage}: still jumping");
        assert_eq!(nav_target(&view), None, "{stage}: no slot pick");
        command_tap(&mut view, NEAREST_STELLAR_COMMAND_KEY, false);
        view.tick(TICK);
        assert!(view.jump_under_way(), "{stage}: still jumping");
        assert_eq!(nav_target(&view), None, "{stage}: Command-5 picks nothing");
        let earth = on_screen(&view, (0.0, -600.0));
        click(&mut view, earth);
        assert!(!view.map_open(), "{stage}");
        assert_eq!(view.take_landing(), None, "{stage}");
        assert!(view.take_mission_info_request().is_none(), "{stage}");
        assert_eq!(nav_mode(&view), mode, "{stage}");
        assert_eq!(nav_target(&view), None, "{stage}");
        let session = view.session().expect("flying");
        assert_eq!(session.course(), course, "{stage}");
        assert_eq!(view.message(), None, "{stage}");
        assert_eq!(view.preparing_jump(), braking, "{stage}");
        assert_eq!(view.jump_effect().is_some(), !braking, "{stage}");
    }
}

#[test]
fn a_slot_key_pressed_in_a_jump_and_held_selects_after_it() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    view.tick(ms(300));
    view.input(&key(STELLAR_SLOT_KEYS[0][0], true));
    tick_to_arrival(&mut view);
    assert_eq!(nav_target(&view), None, "the arrival frame is the jump's");
    view.tick(TICK);
    assert_eq!(
        nav_target(&view),
        Some(StellarId(140)),
        "the first after it"
    );
}

#[test]
fn a_slot_key_held_from_before_a_jump_does_nothing_after_it() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(STELLAR_SLOT_KEYS[0][0], true));
    view.tick(TICK);
    assert!(nav_target(&view).is_some(), "selected before the jump");
    plot(&mut view, 131);
    jump_now(&mut view);
    tick_to_arrival(&mut view);
    ticks(&mut view, 3);
    assert_eq!(nav_target(&view), None, "latched through it");
}

#[test]
fn a_flight_key_held_through_arrival_acts_at_once() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    view.tick(ms(300));
    view.input(&key(Key::Up, true));
    tick_to_arrival(&mut view);
    let arrived = player(&view);
    view.tick(TICK);
    assert_eq!(player(&view), stepped(arrived, THRUST, 1), "thrusting");
    assert_ne!(
        player(&view),
        stepped(arrived, Controls::default(), 1),
        "not coasting"
    );
}

#[test]
fn a_key_held_when_j_is_pressed_stays_held() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(Key::Up, true));
    jump_now(&mut view);
    tick_to_arrival(&mut view);
    let arrived = player(&view);
    view.tick(TICK);
    assert_eq!(player(&view), stepped(arrived, THRUST, 1), "thrusting");
    assert_ne!(
        player(&view),
        stepped(arrived, Controls::default(), 1),
        "not coasting"
    );
}

#[test]
fn fire_held_through_a_jump_fires_only_after_arrival() {
    let mut view = fighting(armed(0, &[BLASTER]), &[]);
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.preparing_jump(), "braking");
    view.input(&key(FIRE_KEY, true));
    while view.jump_effect().is_none() {
        view.tick(TICK);
        assert_eq!(shots_of(&view, BLASTER), 0, "braking");
    }
    tick_to_arrival(&mut view);
    assert_eq!(shots_of(&view, BLASTER), 0);
    view.tick(TICK);
    assert_eq!(shots_of(&view, BLASTER), 1, "fired on the first tick");
}

/// The fade's alpha as drawn, if it is.
fn fade_alpha(view: &View) -> Option<u8> {
    fade(&drawn(view)).map(|(_, color)| color.a)
}

#[test]
fn the_ship_flies_under_the_arrival_fade_in() {
    let mut view = fading_in_on_course_to_barnard();
    let arrived = player(&view);
    let before = fade_alpha(&view).expect("fading in");
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(player(&view), stepped(arrived, THRUST, 3));
    let after = fade_alpha(&view).expect("still fading in");
    assert!(after < before, "{after} < {before}");
}

#[test]
fn the_arrival_fade_plays_on_and_shows_over_the_map() {
    let mut view = fading_in_on_course_to_barnard();
    tap(&mut view, MAP);
    assert!(view.map_open(), "the map opens under the fade");
    let list = drawn(&view);
    let (at_fade, color) = fade(&list).expect("the fade, over the map");
    assert_eq!(at_fade, list.len() - 1, "drawn last");
    let before = color.a;
    view.tick(ms(100));
    let after = fade_alpha(&view).expect("fading on with the map open");
    assert!(after < before, "{after} < {before}");
    view.tick(crate::flight::jump::FADE_IN_FOR);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(view.jump_effect(), None, "the fade played out");
    assert_eq!(fade(&drawn(&view)), None);
}

#[test]
fn the_stellars_go_on_animating_while_the_jump_plays() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let moon = |view: &View| sprites(&drawn(view))[1].0.frame;
    let before = moon(&view);
    view.tick(TICK);
    assert_eq!(moon(&view), (before + 1) % 4, "a frame a tick");
    view.tick(TICK * 2);
    assert_eq!(moon(&view), (before + 3) % 4);
    assert!(view.jump_effect().is_some());
}

#[test]
fn cancelling_the_pointer_abandons_a_drag_on_the_open_map() {
    let mut view = flight();
    tap(&mut view, MAP);
    let empty = Point::new(250.0, 150.0);
    let left = |pressed| Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed,
        at: empty,
    };
    view.input(&left(true));
    view.cancel_pointer();
    let place = view.course_map().place().expect("in flight").scroll();
    view.input(&Input::PointerMoved(Point::new(300.0, 150.0)));
    view.input(&left(false));
    assert_eq!(
        view.course_map().place().expect("in flight").scroll(),
        place,
        "no drag"
    );
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    assert_eq!(view.session().expect("flying").course(), []);
    assert!(view.map_open());
}

// Hyper Select.

/// A flight from Sol, which lists Alpha Centauri then Barnard.
fn fanned() -> View {
    test_flight::new(FakeCatalog {
        fan: true,
        ..catalog()
    })
}

/// The system selected in nav mode 3.
fn hyper_selected(view: &View) -> Option<SystemId> {
    nav_mode(view).hyperspace()
}

#[test]
fn a_repeat_or_release_of_the_hyper_select_key_does_not_cycle() {
    let mut view = fanned();
    view.input(&held(HYPER_SELECT));
    view.input(&key(HYPER_SELECT, false));
    assert_eq!(hyper_selected(&view), None);
    tap(&mut view, HYPER_SELECT);
    view.input(&held(HYPER_SELECT));
    view.input(&key(HYPER_SELECT, false));
    assert_eq!(hyper_selected(&view), Some(SystemId(131)));
}

#[test]
fn hyper_select_with_the_map_open_changes_nothing() {
    let mut view = fanned();
    tap(&mut view, MAP);
    tap(&mut view, HYPER_SELECT);
    assert!(view.map_open());
    assert_eq!(course(&view), []);
}

// Hypergates and wormholes.

#[test]
fn the_gate_messages_read_as_the_original() {
    assert_eq!(
        arrival_message_with(EXITING_HYPERGATE, "Alpha Centauri", DATE, true),
        "Exiting hypergate in the Alpha Centauri system on June 23rd, 1177 NC."
    );
    assert_eq!(
        arrival_message_with(PASSING_WORMHOLE, "Barnard", DATE, false),
        "Passing through a wormhole into the Barnard system on June 23rd, 1177 NC. \
         No stellar objects present."
    );
    assert_eq!(
        arrival_message_with(JUMPING_INTO, "Sol", DATE, true),
        arrival_message("Sol", DATE, true)
    );
    assert_eq!(
        gate_refusal_message(GateKind::Hypergate),
        "Your ship is unable to enter this hypergate - it is offline."
    );
    assert_eq!(
        gate_refusal_message(GateKind::Wormhole),
        "Your ship is unable to enter this wormhole - the radiation levels are too extreme."
    );
    assert_eq!(HYPERGATE_CANCELLED, "Hypergate jump cancelled.");
    assert_eq!(HYPERGATE_DENIED, "Hypergate usage denied.");
}

#[test]
fn the_course_map_makes_no_map_sound_on_its_own_input_or_ms_repeats() {
    let mut view = flight();
    tap(&mut view, MAP);
    view.take_sounds();
    view.input(&held(MAP));
    view.input(&key(MAP, false));
    for k in [Key::Up, Key::Down, Key::Left, Key::Right] {
        tap(&mut view, k);
    }
    let somewhere = on_map(&mut view, 131);
    click(&mut view, somewhere);
    assert!(view.map_open());
    assert_eq!(map_sounds(&mut view), []);
}

#[test]
fn a_refused_m_makes_no_map_sound() {
    let mut view = breaking_up();
    view.take_sounds();
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), [], "breaking up");
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        view.take_sounds();
        tap(&mut view, MAP);
        assert!(!view.map_open(), "{stage}");
        assert_eq!(map_sounds(&mut view), [], "{stage}");
    }
    let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(140)));
    assert_eq!(view.take_off(), Some(StellarId(140)));
    view.take_sounds();
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), [], "the take-off grace");
}

#[test]
fn a_hypergates_greeting_is_one_of_its_own_by_a_roll() {
    let mut view = test_flight::new(gated());
    tap(&mut view, LAND);
    assert_eq!(view.message(), None, "requested over it: silent");
    ticks(&mut view, 1);
    let ship = ship_name(&view);
    let first = format!("Cleared for hypergate entry, {ship}. ");
    assert_eq!(view.message(), Some(first.as_str()), "the first outcome");
    let last: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    let mut view = new_on(gated(), SharedChance::new(last));
    tap(&mut view, LAND);
    ticks(&mut view, 1);
    assert_lasts(&mut view, "You are cleared for hypergate entry. ", 250);
}

/// Taps I in `view`, its sounds taken first: nothing is asked for,
/// nothing said and nothing sounded.
fn i_does_nothing_and_says_nothing(view: &mut View, why: &str) {
    let _ = view.take_sounds();
    let message = view.message().map(str::to_owned);
    tap(view, MISSION_INFO_KEY);
    assert_eq!(view.take_mission_info_request(), None, "{why}");
    assert_eq!(view.message(), message.as_deref(), "{why}");
    assert_eq!(view.take_sounds(), [], "{why}");
}

#[test]
fn i_during_a_jump_does_nothing_and_says_nothing() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.session().expect("flying").preparing_jump().is_some());
    i_does_nothing_and_says_nothing(&mut view, "turning to jump (@0x70424)");
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    i_does_nothing_and_says_nothing(&mut view, "streaking (@0x70424)");
}

#[test]
fn i_in_the_escape_pod_does_nothing_and_says_nothing() {
    let mut pod = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
    assert!(until_ejected(&mut pod).0);
    i_does_nothing_and_says_nothing(&mut pod, "in the pod (@0x7042e)");
}

#[test]
fn i_while_disabled_is_taken() {
    let mut view = disabled_player();
    let _ = view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
}

#[test]
fn the_ship_flies_under_a_hypergates_fade_in() {
    let mut view = test_flight::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    let arrived = player(&view);
    assert_eq!(arrived.position, Vec2::new(100.0, 200.0));
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(player(&view), stepped(arrived, THRUST, 3));
    assert_ne!(player(&view).position, arrived.position, "it flies");
    assert!(translucent_fade(&drawn(&view)), "still fading in");
}
