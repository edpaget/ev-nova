use super::*;

// Hyperspace.

#[test]
fn the_course_map_starts_on_the_current_system_with_no_course() {
    let view = flight();
    assert!(!view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(view.course_map().current(), Some(SystemId(130)));
    assert_eq!(view.course_map().route(), []);
    assert_eq!(view.jump_effect(), None);
    assert_eq!((MAP_KEY, JUMP_KEY), (MAP, JUMP));
}

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

#[test]
fn a_plain_click_on_a_linked_system_selects_it_in_mode_3_and_redraws_the_nav_at_once() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert_eq!(nav(&view), [hud::NAV_OFF]);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert!(view.map_open());
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(
        nav(&view),
        [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED],
        "drawn while the map is open"
    );
    assert_eq!(course(&view), [], "the course untouched");
    assert_eq!(view.course_map().route(), []);
}

#[test]
fn a_plain_click_on_an_unlinked_known_system_lets_go_of_the_selection_and_keeps_the_course() {
    let mut view = flight();
    plot(&mut view, 131);
    know(&mut view, 132);
    tap(&mut view, MAP);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert_eq!(course(&view), [SystemId(131)], "the course kept");
    assert_eq!(view.course_map().route(), [SystemId(131)]);
}

#[test]
fn a_click_on_an_unknown_system_two_jumps_away_drags_and_selects_nothing() {
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    tap(&mut view, MAP);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(
        view.course_map().selected(),
        Some(SystemId(130)),
        "the preselect"
    );
    assert_eq!(nav_mode(&view), NavMode::Off);
}

#[test]
fn shift_clicks_build_a_course_cueing_nothing_until_the_close_and_the_jump_goes_there() {
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    run_set(&mut view, "X131");
    tap(&mut view, MAP);
    for hop in [131, 132] {
        let at = on_map(&mut view, hop);
        shift_click(&mut view, at);
        assert_eq!(nav_mode(&view), NavMode::Off, "nothing cued at {hop}");
    }
    assert_eq!(course(&view), [SystemId(131), SystemId(132)]);
    assert_eq!(view.course_map().route(), [SystemId(131), SystemId(132)]);
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
    tap(&mut view, MAP);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "cued"
    );
    fly_out(&mut view);
    jump_now(&mut view);
    tick_to_arrival(&mut view);
    assert_eq!(system_of(&view), SystemId(131));
}

#[test]
fn a_plain_click_after_a_shift_built_course_selects_and_the_close_cues_the_first_hop_back() {
    let mut view = FlightView::new(FakeCatalog {
        fan: true,
        ..catalog()
    });
    tap(&mut view, MAP);
    let alpha = on_map(&mut view, 131);
    shift_click(&mut view, alpha);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
    tap(&mut view, MAP);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "the close's cue (@0x13c16)"
    );
    assert_eq!(course(&view), [SystemId(131)]);
}

#[test]
fn a_shift_click_at_the_current_systems_place_empties_the_course_and_keeps_clear_route() {
    let mut view = flight();
    tap(&mut view, MAP);
    let alpha = on_map(&mut view, 131);
    shift_click(&mut view, alpha);
    assert!(clear_route_enabled(&view), "a hop written");
    let sol = on_map(&mut view, 130);
    shift_click(&mut view, sol);
    assert_eq!(course(&view), []);
    assert_eq!(view.course_map().route(), []);
    assert!(clear_route_enabled(&view), "the latch kept");
    assert_eq!(view.course_map().selected(), Some(SystemId(131)), "kept");
}

#[test]
fn by_the_planner_a_click_on_a_system_plots_the_course_there_and_the_map_shows_it() {
    let mut view = planner(flight());
    tap(&mut view, MAP);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert!(view.map_open());
    let course = [SystemId(131)];
    assert_eq!(view.session().expect("flying").course(), course);
    assert_eq!(view.course_map().route(), course);
    assert_eq!(view.course_map().current(), Some(SystemId(130)));
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "cued at once"
    );
    know(&mut view, 132);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(view.session().expect("flying").course(), []);
    assert_eq!(view.course_map().route(), [], "no route there");
}

#[test]
fn by_the_planner_a_click_with_no_route_disables_clear_route() {
    let mut view = planner(flight());
    tap(&mut view, MAP);
    assert!(!clear_route_enabled(&view), "no course");
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert!(clear_route_enabled(&view), "a course plotted");
    know(&mut view, 132);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(view.course_map().route(), [], "no route there");
    assert!(!clear_route_enabled(&view), "no course to clear");
}

#[test]
fn by_the_engine_no_click_turns_clear_route_off() {
    let mut view = flight();
    tap(&mut view, MAP);
    let alpha = on_map(&mut view, 131);
    shift_click(&mut view, alpha);
    assert!(clear_route_enabled(&view), "a hop written");
    click(&mut view, alpha);
    assert!(clear_route_enabled(&view), "a plain click keeps the latch");
    know(&mut view, 132);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert!(
        clear_route_enabled(&view),
        "an unlinked plain click keeps it"
    );
}

#[test]
fn closing_the_map_abandons_its_drag() {
    let mut view = flight();
    tap(&mut view, MAP);
    let empty = Point::new(250.0, 150.0);
    view.input(&Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed: true,
        at: empty,
    });
    view.close_map();
    assert!(!view.map_open());
    tap(&mut view, MAP);
    let place = view.course_map().place().expect("in flight").scroll();
    view.input(&Input::PointerMoved(Point::new(300.0, 150.0)));
    view.input(&Input::PointerButton {
        button: crate::MouseButton::Left,
        pressed: false,
        at: empty,
    });
    assert_eq!(
        view.course_map().place().expect("in flight").scroll(),
        place,
        "no drag"
    );
    assert_eq!(
        view.course_map().selected(),
        Some(SystemId(130)),
        "the preselect"
    );
    assert_eq!(view.session().expect("flying").course(), []);
}

#[test]
fn opening_the_map_selects_the_current_system_when_nothing_is_selected_in_mode_3() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    // The panel is Sol's: its ports are Earth and the Moon.
    assert!(
        matches!(
            view.course_map().panel().map(|panel| &panel.strip),
            Some(nova_sim::map_panel::Strip::Info {
                ports: nova_sim::map_panel::Ports::Listed(names),
                ..
            }) if names.len() == 2
        ),
        "{:?}",
        view.course_map().panel()
    );
    assert_eq!(hyper_selected_links(&view), []);
    tap(&mut view, MAP);
    plot(&mut view, 131);
    tap(&mut view, NAV_OFF_KEY);
    assert_eq!(nav_mode(&view), NavMode::Off);
    tap(&mut view, MAP);
    assert_eq!(
        view.course_map().selected(),
        Some(SystemId(130)),
        "the current system, not the course's 131"
    );
    assert_eq!(hyper_selected_links(&view), []);
}

#[test]
fn opening_the_map_selects_and_draws_the_system_hyper_selected() {
    let mut view = FlightView::new(FakeCatalog {
        fan: true,
        ..catalog()
    });
    plot(&mut view, 131);
    tap(&mut view, Key::Char('\\'));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
    tap(&mut view, MAP);
    let map = view.course_map();
    assert_eq!(map.selected(), Some(SystemId(132)));
    assert_eq!(map.route(), [SystemId(131)], "the course kept");
    assert_eq!(
        hyper_selected_links(&view),
        [(on_map_now(&view, 130), on_map_now(&view, 132))]
    );
    assert_eq!(
        map.panel().map(|panel| panel.column.body.clone()),
        Some(nova_sim::map_panel::ColumnBody::Unknown),
        "the panel is Barnard's, unexplored"
    );
}

#[test]
fn a_click_on_the_map_draws_the_link_it_cues() {
    let mut view = flight();
    tap(&mut view, MAP);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert!(view.map_open());
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(
        hyper_selected_links(&view),
        [(on_map_now(&view, 130), on_map_now(&view, 131))]
    );
}

#[test]
fn a_click_at_a_shared_place_selects_the_lowest_accepted_system_every_time() {
    // `_DoSystemMap`'s click (@0x12dd4-0x1314b) never reads the
    // preselect, and has no stack cycling: it takes the lowest visible
    // system at the place it accepts, as the system standing there.
    let mut view = FlightView::new(FakeCatalog {
        reborn: true,
        ..catalog()
    });
    tap(&mut view, Key::Char('\\'));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    tap(&mut view, MAP);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    let alpha = on_map(&mut view, 131);
    assert_eq!(
        alpha,
        on_map(&mut view, 133),
        "Alpha Reborn stands there too"
    );
    click(&mut view, alpha);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)), "not 133");
    assert_eq!(course(&view), [], "a plain click");
    click(&mut view, alpha);
    assert_eq!(
        view.course_map().selected(),
        Some(SystemId(131)),
        "a click again is the same click"
    );
    shift_click(&mut view, alpha);
    assert_eq!(course(&view), [SystemId(131)]);
}

#[test]
fn j_says_why_a_jump_is_refused_in_the_originals_words() {
    let mut view = flight();
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), None, "a seasoned pilot, nothing selected");
    assert_eq!(view.take_sounds(), [], "and no beep");
    plot(&mut view, 131);
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.take_sounds(), [REFUSED]);
    assert_eq!(message(&view), Some(message_drawn(TOO_CLOSE)));
    assert_lasts(&mut view, TOO_CLOSE, 360);

    let mut dry = dry();
    plot(&mut dry, 131);
    fly_out(&mut dry);
    dry.take_sounds();
    dry.input(&key(JUMP, true));
    assert_eq!(dry.jump_effect(), None);
    assert_eq!(dry.take_sounds(), [REFUSED]);
    assert_lasts(&mut dry, NO_FUEL, 240);
}

/// A flight whose ship holds 50 fuel, short of a jump's.
fn dry() -> View {
    FlightView::new(FakeCatalog {
        fields: ShipFields { fuel: 50, ..FIELDS },
        ..catalog()
    })
}

#[test]
fn j_while_disabled_is_silent() {
    for condition in [
        Condition::Disabled,
        Condition::Dying {
            ticks_left: 100,
            starting: false,
        },
    ] {
        let mut view = new_pilot(flight());
        let Ok(session) = &mut view.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_condition(session, condition);
        view.input(&key(JUMP, true));
        assert_eq!(view.message(), None, "{condition:?}");
        let session = view.session().expect("flying");
        assert_eq!(session.preparing_jump(), None, "{condition:?}");
        assert!(
            !view.take_sounds().contains(&REFUSED),
            "{condition:?}: no beep"
        );
        // Made intact again, the same J is told: the gate, not a dead key.
        let Ok(session) = &mut view.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_condition(session, Condition::Intact);
        view.input(&key(JUMP, true));
        assert_eq!(view.take_sounds(), [REFUSED], "{condition:?}");
        assert_eq!(view.message(), Some(NO_DESTINATION), "{condition:?}");
    }
}

#[test]
fn a_new_pilots_j_with_nothing_selected_says_no_destination_for_360() {
    let mut view = new_pilot(flight());
    view.input(&key(JUMP, true));
    assert_eq!(view.take_sounds(), [REFUSED]);
    assert_eq!(message(&view), Some(message_drawn(NO_DESTINATION)));
    assert_lasts(&mut view, NO_DESTINATION, 360);
}

#[test]
fn a_new_pilot_with_a_hyperspace_selection_is_never_told_no_destination() {
    let mut view = new_pilot(flight());
    plot(&mut view, 131);
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), Some(TOO_CLOSE), "the zone's refusal");
    assert_eq!(view.take_sounds(), [REFUSED]);
}

#[test]
fn a_new_pilot_short_of_fuel_is_not_told() {
    let mut view = new_pilot(dry());
    plot(&mut view, 131);
    fly_out(&mut view);
    view.message = None;
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), None);
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn a_new_pilot_short_of_fuel_is_told_by_the_other_reading() {
    let rules = Rulebook::default().with_override(RuleKey::NewbieFuelRefusal, RuleSource::Bible);
    let mut view = new_pilot(dry().with_rules(rules));
    plot(&mut view, 131);
    fly_out(&mut view);
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.take_sounds(), [REFUSED]);
    assert_eq!(view.message(), Some(NO_FUEL));
    assert_eq!(view.message.as_ref().map(|said| said.ticks), Some(240));
}

/// The land hint as the fixture words it, for the Moon, the nearest
/// stellar to the centre.
const MOON_HINT: &str = "Welcome to Nova - it would be a good idea to start by landing on Moon and having a look around. Hit 'L' to request landing clearance, then hit it again to land.";

#[test]
fn the_land_hint_names_the_stellar_and_l() {
    let mut view = flight();
    view.begin();
    view.take_sounds();
    ticks(&mut view, 1);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TutorialHint)]);
    assert_eq!(message(&view), Some(message_drawn(MOON_HINT)));
    assert_lasts(&mut view, MOON_HINT, 512);
}

#[test]
fn the_land_hint_words_a_station_as_docking() {
    assert_eq!(
        land_hint(true, "Earth Station", "L"),
        "Welcome to Nova - it would be a good idea to start by docking at Earth Station and having a look around. Hit 'L' to request landing clearance, then hit it again to land."
    );
    assert_eq!(land_hint(false, "Moon", "L"), MOON_HINT);
}

#[test]
fn the_hyperspace_hint_names_m_and_j() {
    let hint = "Now you should probably hyperspace to another system. Hit 'M' to access the map, select a nearby system, then move outward and hit 'J' to begin your jump.";
    assert_eq!(hyperspace_hint("M", "J"), hint);
    // At (50, 50), where a new pilot starts.
    let mut view = new_pilot(flight_among(vec![site(
        128,
        (50.0, 50.0),
        StellarFlags::CAN_LAND,
    )]));
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    view.take_off().expect("takes off");
    view.take_sounds();
    // The take-off's frame is -14: the hint comes at 0.
    ticks(&mut view, 14);
    assert_eq!(view.message(), None);
    ticks(&mut view, 1);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TutorialHint)]);
    assert_lasts(&mut view, hint, 512);
}

#[test]
fn far_enough_and_nothing_out_here_last_512() {
    let mut view = new_pilot(flight());
    fly_far(&mut view);
    // The ship drifts on outward: #30 shows on the first sixtieth
    // frame past 1,414 px, until #31 replaces it past 3,162 px.
    let distance = |view: &View| player(view).position.length();
    let mut far = None;
    for tick in 0..2000 {
        ticks(&mut view, 1);
        match (view.message(), far) {
            (Some(FAR_ENOUGH), None) => far = Some((tick, distance(&view))),
            (Some(FAR_ENOUGH), Some(_)) => {}
            (Some(NOTHING_OUT_HERE), Some((since, at))) => {
                assert!(at > 1414.3, "{at}");
                assert!(distance(&view) > 3162.3, "{}", distance(&view));
                assert!(tick - since < 512, "#30 shown throughout");
                assert_lasts(&mut view, NOTHING_OUT_HERE, 512);
                return;
            }
            (shown, _) => panic!("{shown:?} at tick {tick}"),
        }
    }
    panic!("#31 never came: {:?}", player(&view));
}

#[test]
fn j_held_by_a_tractor_beam_says_36_with_the_refusal_beep() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    let session = view.session.as_mut().expect("flying");
    nova_sim::fixture::set_player_tractor(
        session,
        Some(nova_sim::combat::impact::TractorLock::by(
            nova_sim::combat::ShipRef::Npc(nova_sim::NpcId(7)),
        )),
    );
    view.take_sounds();
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None);
    assert!(!view.preparing_jump());
    assert_eq!(view.take_sounds(), [REFUSED]);
    assert_eq!(message(&view), Some(message_drawn(TRACTORED)));
    assert_lasts(&mut view, TRACTORED, 360);
}

#[test]
fn starting_a_jump_clears_the_far_enough_hint() {
    for text in [FAR_ENOUGH, NOTHING_OUT_HERE, TOO_CLOSE, TRACTORED] {
        let mut view = flight();
        plot(&mut view, 131);
        fly_out(&mut view);
        view.say_for(text, 512);
        view.input(&key(JUMP, true));
        assert!(view.preparing_jump() || view.jump_effect().is_some());
        assert_eq!(view.message(), None, "{text}");
    }
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.say_for(NO_FUEL, 240);
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), Some(NO_FUEL), "any other stays");
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
fn j_while_landed_is_refused_until_take_off() {
    let mut view = flight_among(vec![site(140, (0.0, 0.0), StellarFlags::CAN_LAND)]);
    plot(&mut view, 131);
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(140)));
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.session().expect("flying").jumping(), None);
    assert_eq!(view.message(), Some(TAKE_OFF_FIRST));
    view.take_off();
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), Some(TOO_CLOSE), "flying again");
}

/// Where the ship's sprite is drawn.
fn ship_at(list: &DrawList) -> usize {
    list.iter()
        .position(|c| matches!(c, DrawCommand::Sprite { image, .. } if image.id == 2000))
        .expect("the ship")
}

#[test]
fn a_jump_moves_off_fades_to_white_arrives_on_the_warp_end_and_fades_in() {
    let mut view = flight().with_overlays(TITLED);
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let leaving = player(&view);
    let fuel_leaving = reserves(&view).fuel.now;
    assert_eq!(view.message(), None);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Warp)
    );
    assert_eq!(
        view.session().expect("flying").jumping(),
        Some(SystemId(131))
    );

    // The ship warps; nothing fades until the session lights the screen.
    let mut moved = false;
    for _ in 0..2000 {
        view.tick(TICK);
        moved |= player(&view).position != leaving.position;
        if view.session().expect("flying").hyper_gamma() > 0.0 {
            break;
        }
        assert_eq!(fade(&drawn(&view)), None, "unlit");
    }
    assert!(moved, "the ship moves off before the white");
    let east = player(&view).position.x - leaving.position.x;
    assert!(east > 50.0, "it rushes east: {east}");

    // The old system fades to white from that tick, HUD and all.
    let list = drawn(&view);
    let (at_fade, color) = fade(&list).expect("a fade");
    assert!(color.a > 0 && color.a < 10, "{color:?}");
    assert!(ship_at(&list) < hud_at(&list) && hud_at(&list) < at_fade);
    assert_eq!(at_fade, list.len() - 1, "the fade is drawn last");
    assert!(texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
    let first = fade(&list).map(|f| f.1.a);
    view.tick(ms(750));
    assert!(fade(&drawn(&view)).map(|f| f.1.a) > first, "growing");

    // It arrives on the step the session ends the warp, and the new
    // system fades in from white.
    while view.session().expect("flying").system() == SystemId(130) {
        assert!(!view.session().expect("flying").warp_end_due());
        view.tick(TICK);
    }
    let session = view.session().expect("flying");
    assert_eq!(session.system(), SystemId(131));
    assert_eq!(session.jumping(), None);
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(130), SystemId(131)]
    );
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.course_map().route(), []);
    // 1350 out on Sol's side, at top speed (`nova_sim::jump_arrival`).
    let arrived = player(&view);
    let top = view.session().expect("flying").top_speed();
    assert!(
        (arrived.position - Vec2::new(-1350.0, 0.0)).length() < 1e-3,
        "{arrived:?}"
    );
    assert!(
        (arrived.velocity.length() - top).abs() < 1e-4,
        "{arrived:?}"
    );
    assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
    assert_eq!(
        view.shown_position(),
        at(arrived.position.x, arrived.position.y)
    );
    let list = drawn(&view);
    let (at_fade, color) = fade(&list).expect("a fade");
    assert_eq!(color.a, 255, "the arrival frame, white");
    assert!(texts(&list).contains(&"Alpha Centauri (sÿst 131)".to_owned()));
    assert_eq!(at_fade, list.len() - 1, "the fade over everything");
    view.tick(ms(750));
    assert!(fade(&drawn(&view)).map(|f| f.1.a) < Some(255), "shrinking");

    // Then plain flight.
    view.tick(ms(800));
    assert_eq!(view.jump_effect(), None);
    let list = drawn(&view);
    assert_eq!(fade(&list), None);
    let stars = drawn_stars(&view);
    assert!(list.iter().take(stars.len()).eq(stars.iter()));
    let flying = player(&view);
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(player(&view), stepped(flying, THRUST, 3));
    assert_ne!(player(&view), flying, "it flies again");
}

#[test]
fn the_fade_starts_on_the_tick_the_screen_is_lit_and_arrival_waits_for_the_session() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let lit = tick_until_lit(&mut view);
    assert!(lit > 1, "{lit}");
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(_))
    ));
    // The fade runs on display time; the arrival comes when the session
    // ends the warp, before the fade is through: (v - 55) × 5 lights late.
    let mut fading = 0;
    while view.session().expect("flying").system() == SystemId(130) {
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeOut(_))
        ));
        view.tick(TICK);
        fading += 1;
    }
    assert!(fading > 1 && fading < 45, "{fading}");
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0)),
        "white at arrival"
    );
}

/// A flight warping to Alpha Centauri, its screen lit when `lit` and its
/// fade to white under way, given up as `give_up` gives the session's
/// jump up within a tick, then ticked once more.
fn given_up(lit: bool, give_up: fn(&mut Session)) -> View {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    if lit {
        tick_until_lit(&mut view);
        ticks(&mut view, 2);
        assert!(matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::FadeOut(_))
        ));
    } else {
        assert!(view.session().expect("flying").hyper_gamma() <= 0.0);
    }
    let Ok(session) = &mut view.session else {
        panic!("flying");
    };
    give_up(session);
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert_eq!(session.jumping(), None, "lit: {lit}");
    assert_eq!(session.preparing_jump(), None, "lit: {lit}");
    assert_eq!(session.system(), SystemId(130), "no arrival");
    view.tick(TICK);
    view
}

/// Disables the player: its jump collapses on the next step.
fn disable_player(session: &mut Session) {
    nova_sim::fixture::set_player_condition(session, Condition::Disabled);
}

/// A way to give the session's jump up, named.
type GiveUp = (&'static str, fn(&mut Session));

/// The ways a jump is given up before arrival: collapsed by a disable,
/// and cut by a sabotage kill outfit.
const GIVE_UPS: [GiveUp; 2] = [
    ("collapsed", disable_player),
    ("sabotaged", nova_sim::fixture::sabotage_kill),
];

#[test]
fn a_jump_given_up_before_the_screen_is_lit_drops_its_effect() {
    for (how, give_up) in GIVE_UPS {
        let mut view = given_up(false, give_up);
        assert_eq!(view.jump_effect(), None, "{how}");
        assert_eq!(fade(&drawn(&view)), None, "{how}: nothing white");
        view.tick(ms(3000));
        assert_eq!(fade(&drawn(&view)), None, "{how}: nor later");
    }
}

#[test]
fn a_jump_given_up_once_the_screen_is_lit_fades_back_from_white() {
    for (how, give_up) in GIVE_UPS {
        let mut view = given_up(true, give_up);
        assert!(
            matches!(
                view.jump_effect().map(JumpEffect::phase),
                Some(JumpPhase::FadeIn(_))
            ),
            "{how}: {:?}",
            view.jump_effect()
        );
        let (_, color) = fade(&drawn(&view)).expect("a fade");
        assert!(color.a > 240, "{how}: from white, {color:?}");
        view.tick(ms(750));
        let (_, color) = fade(&drawn(&view)).expect("a fade");
        assert!(color.a < 200, "{how}: falling back, {color:?}");
        view.tick(FADE_IN_FOR);
        assert_eq!(view.jump_effect(), None, "{how}");
        assert_eq!(fade(&drawn(&view)), None, "{how}: the white gone");
    }
}

#[test]
fn a_player_whose_jump_collapsed_can_jump_again_once_intact() {
    for lit in [false, true] {
        let mut view = given_up(lit, disable_player);
        let Ok(session) = &mut view.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_condition(session, Condition::Intact);
        view.take_sounds();
        view.input(&key(PLAYER_INFO_KEY, true));
        view.input(&key(PLAYER_INFO_KEY, false));
        assert!(view.take_player_info_request(), "lit: {lit}: P is taken");
        assert!(!view.take_sounds().contains(&REFUSED), "lit: {lit}");
        tap(&mut view, JUMP);
        let session = view.session().expect("flying");
        assert!(
            session.preparing_jump().is_some() || session.jumping().is_some(),
            "lit: {lit}: jumping again"
        );
    }
}

#[test]
fn hyperspace_effects_default_on() {
    let mut view = flight();
    assert!(view.hyperspace_effects());
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    tick_until_lit(&mut view);
    view.tick(ms(750));
    assert!(translucent_fade(&drawn(&view)), "fading out");
}

/// Whether `list` draws the fade quad translucent: part of a fade.
fn translucent_fade(list: &DrawList) -> bool {
    fade(list).is_some_and(|(_, color)| color.a < u8::MAX)
}

#[test]
fn with_effects_off_a_jump_moves_off_then_arrives_without_fading() {
    let mut view = flight().with_hyperspace_effects(false);
    assert!(!view.hyperspace_effects());
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let leaving = player(&view);

    // The ship warps away, lighting nothing.
    while view.session().expect("flying").system() == SystemId(130) {
        assert_eq!(fade(&drawn(&view)), None);
        assert!(view.session().expect("flying").hyper_gamma() < 0.0);
        view.tick(TICK);
    }
    let arriving = Vec2::new(-1350.0, 0.0);
    assert!((leaving.position - arriving).length() > 1.0);

    // On the warp's end the ship arrives and the screen is white once.
    let session = view.session().expect("flying");
    assert_eq!(session.system(), SystemId(131));
    assert_eq!(session.jumping(), None);
    let list = drawn(&view);
    assert_eq!(
        fade(&list),
        Some((list.len() - 1, Color::WHITE)),
        "opaque white over everything"
    );
    assert!(view.jump_effect().is_some(), "flashing");
    assert!(
        (player(&view).position - arriving).length() < 1e-3,
        "{:?}",
        player(&view)
    );

    // Then plain flight, with no fade in.
    view.tick(ms(33));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(fade(&drawn(&view)), None);
}

#[test]
fn set_hyperspace_effects_changes_the_next_jump() {
    let mut view = flight().with_hyperspace_effects(false);
    view.set_hyperspace_effects(true);
    assert!(view.hyperspace_effects());
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    tick_until_lit(&mut view);
    view.tick(ms(750));
    assert!(translucent_fade(&drawn(&view)), "fading");

    let mut view = flight();
    view.set_hyperspace_effects(false);
    assert!(!view.hyperspace_effects());
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let mut faded = false;
    while view.session().expect("flying").system() == SystemId(130) {
        view.tick(TICK);
        faded |= translucent_fade(&drawn(&view));
    }
    assert!(!faded, "never fades");
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
}

#[test]
fn the_sabotage_death_descs_come_from_the_pilots_outfits() {
    let mut sabotaged = multi_jumping();
    sabotaged.outfits[0].mods[0] = (nova_sim::SABOTAGE_KILL, 5000);
    assert_eq!(FlightView::new(sabotaged).sabotage_death_descs(), [5000]);
    assert_eq!(flight().sabotage_death_descs(), Vec::<i16>::new());
}

#[test]
fn a_multi_jump_plays_one_effect_and_loads_only_the_final_system() {
    let mut view = planner(FlightView::new(multi_jumping())).with_overlays(TITLED);
    plot(&mut view, 132);
    assert_eq!(
        view.session().expect("flying").course(),
        [SystemId(131), SystemId(132)]
    );
    fly_out(&mut view);
    jump_now(&mut view);
    let fuel_leaving = reserves(&view).fuel.now;
    assert!(view.jump_effect().is_some(), "jumping");

    tick_to_arrival(&mut view);
    let session = view.session().expect("flying");
    assert_eq!(session.system(), SystemId(132));
    assert_eq!(session.course(), []);
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(130), SystemId(132)],
        "Alpha Centauri is never loaded"
    );
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
    assert_eq!(reserves(&view).fuel.now, fuel_leaving - 100.0);
    assert!(texts(&drawn(&view)).contains(&"Barnard (sÿst 132)".to_owned()));
    assert_eq!(
        view.message(),
        Some("Jumping into the Barnard system on June 24th, 1177 NC. No stellar objects present."),
        "names the final system"
    );
    assert!(view.jump_effect().is_some(), "fading in");

    view.tick(ms(1500));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.session().expect("flying").jumping(), None);
    for _ in 0..30 {
        view.tick(TICK);
        assert_eq!(view.jump_effect(), None, "no second effect");
    }
    assert_eq!(view.session().expect("flying").system(), SystemId(132));
}

#[test]
fn the_jump_effect_waits_for_the_session_to_begin_the_jump() {
    // fly_out leaves the ship racing north, facing it; Alpha Centauri
    // is east.
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None, "not on J");
    assert_eq!(view.message(), None);
    let session = view.session().expect("flying");
    assert_eq!(session.preparing_jump(), Some(SystemId(131)));
    assert_eq!(session.jumping(), None);
    let pressed = player(&view);
    view.tick(TICK);
    assert_ne!(player(&view), pressed, "the session keeps ticking");
    let mut waited = 1;
    while view.session().expect("flying").jumping().is_none() {
        assert_eq!(view.jump_effect(), None, "tick {waited}");
        view.tick(TICK);
        waited += 1;
        assert!(waited < 1000, "the jump never began");
    }
    assert!(view.jump_effect().is_some(), "the effect, on that tick");
    let rate = view.session().expect("flying").handling().turn_rate;
    assert!(
        nova_sim::player_turn::settled(
            player(&view).heading,
            rate,
            90,
            nova_sim::player_turn::PlayerTurn::WholeDegrees
        ),
        "facing it, within a turn: {:?}",
        player(&view)
    );
    assert!(waited > 30, "braked and turned first: {waited}");
}

#[test]
fn a_ship_ready_to_jump_begins_its_jump_on_j() {
    // Turned east at rest, then drifting out slowly that way: facing
    // Alpha Centauri and slow enough, it needs no pre-jump stage.
    let mut view = flight();
    plot(&mut view, 131);
    view.input(&key(Key::Right, true));
    ticks(&mut view, 30);
    view.input(&key(Key::Right, false));
    view.input(&key(Key::Up, true));
    ticks(&mut view, 10);
    view.input(&key(Key::Up, false));
    for _ in 0..2000 {
        if player(&view).position.length() >= MIN_JUMP_DISTANCE {
            break;
        }
        view.tick(TICK);
    }
    let ship = player(&view);
    assert_eq!(ship.heading, 90.0, "{ship:?}");
    assert!(ship.position.length() >= MIN_JUMP_DISTANCE, "{ship:?}");
    view.input(&key(JUMP, true));
    let effect = view.jump_effect().expect("jumping on J");
    assert_eq!(effect.phase(), JumpPhase::Warp);
    assert_eq!(
        view.session().expect("flying").jumping(),
        Some(SystemId(131))
    );
    view.tick(TICK);
    assert_eq!(player(&view).heading, ship.heading, "facing the bearing");
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
    let mut view = FlightView::new(trading());
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
fn j_during_the_arrival_fade_in_begins_the_next_jump() {
    let mut view = fading_in_on_course_to_barnard();
    let arrival = view.message().expect("the arrival message").to_owned();
    view.input(&key(JUMP, true));
    assert_eq!(view.message(), Some(arrival.as_str()), "not refused");
    let session = view.session().expect("flying");
    assert!(
        session.preparing_jump() == Some(SystemId(132)) || session.jumping() == Some(SystemId(132)),
        "{:?} {:?}",
        session.preparing_jump(),
        session.jumping()
    );
    let pressed = fade_alpha(&view).expect("still fading in");
    view.tick(TICK);
    let fading = fade_alpha(&view).expect("the old fade plays on");
    assert!(fading < pressed, "{fading} < {pressed}");
    for _ in 0..1000 {
        if view.session().expect("flying").jumping().is_some() {
            break;
        }
        view.tick(TICK);
    }
    assert_eq!(
        view.session().expect("flying").jumping(),
        Some(SystemId(132))
    );
    let effect = view.jump_effect().expect("the next jump's effect");
    assert!(matches!(
        effect.phase(),
        JumpPhase::Warp | JumpPhase::FadeOut(_)
    ));
    arrive_now(&mut view);
    assert_eq!(system_of(&view), SystemId(132));
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
fn arriving_shows_the_jump_message_on_the_message_line() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    let session = view.session().expect("flying");
    let leaving = session.system();
    for _ in 0..1000 {
        if view.session().expect("flying").system() != leaving {
            break;
        }
        view.tick(TICK);
    }
    let date = view.session().expect("flying").long_date_text();
    let name = view.scene().expect("arrived").name().to_owned();
    let text = arrival_message(&name, &date, true);
    assert!(text.contains("Alpha Centauri"), "{text}");
    assert_eq!(view.message(), Some(text.as_str()));
    assert_eq!(message(&view), Some(message_drawn(&text)));
    assert_lasts(&mut view, &text, 240);
    assert_eq!(message(&view), None);
}

#[test]
fn a_refusal_shown_before_the_jump_is_replaced_by_the_arrival() {
    let mut view = new_pilot(flight());
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    view.input(&key(JUMP, false));
    assert_eq!(view.message(), Some(NO_DESTINATION));
    plot(&mut view, 131);
    assert_eq!(view.message(), Some(NO_DESTINATION), "still on screen");
    view.input(&key(JUMP, true));
    assert_eq!(
        view.message(),
        Some(NO_DESTINATION),
        "starting the jump clears only #42"
    );
    jump_now(&mut view);
    arrive_now(&mut view);
    let shown = view.message().expect("the arrival message");
    assert!(shown.starts_with(JUMPING_INTO), "{shown}");
}

#[test]
fn map_and_jump_in_a_flight_that_never_started_do_nothing_much() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(view.course_map().current(), None);
    view.input(&key(JUMP, true));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), None);
    tap(&mut view, MAP);
    assert!(view.map_open());
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert_eq!(view.course_map().route(), []);
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
    FlightView::new(FakeCatalog {
        fan: true,
        ..catalog()
    })
}

/// The system selected in nav mode 3.
fn hyper_selected(view: &View) -> Option<SystemId> {
    nav_mode(view).hyperspace()
}

#[test]
fn the_hyper_select_key_selects_the_next_listed_system() {
    let mut view = fanned();
    assert_eq!(HYPER_SELECT_KEY, HYPER_SELECT);
    for expected in [131, 132, 131] {
        assert_eq!(view.input(&key(HYPER_SELECT, true)), ScreenAction::None);
        view.input(&key(HYPER_SELECT, false));
        assert_eq!(hyper_selected(&view), Some(SystemId(expected)));
        assert_eq!(course(&view), [], "no course plotted");
    }
}

#[test]
fn hyper_select_in_flight_keeps_the_course() {
    let mut view = fanned();
    plot(&mut view, 131);
    tap(&mut view, HYPER_SELECT);
    assert_eq!(course(&view), [SystemId(131)]);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
}

#[test]
fn the_hud_nav_area_shows_the_selected_system() {
    let mut view = flight();
    tap(&mut view, LAND);
    assert_eq!(nav(&view), [hud::NAV_STELLAR, "Moon"]);
    tap(&mut view, HYPER_SELECT);
    assert_eq!(nav_target(&view), None);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
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
fn hyper_select_is_ignored_while_a_jump_prepares_or_plays() {
    let mut view = fanned();
    tap(&mut view, HYPER_SELECT);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.preparing_jump(), "braking and turning");
    tap(&mut view, HYPER_SELECT);
    assert_eq!(hyper_selected(&view), Some(SystemId(131)));
    jump_now(&mut view);
    tap(&mut view, HYPER_SELECT);
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
fn m_beeps_as_it_opens_and_closes_the_course_map() {
    let mut view = flight();
    view.take_sounds();
    tap(&mut view, MAP);
    assert!(view.map_open());
    assert_eq!(ui_sounds(&mut view), [UiSound::MapOpened], "opened");
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(ui_sounds(&mut view), [UiSound::MapClosed], "closed");
}

#[test]
fn escape_closing_the_course_map_beeps_its_close() {
    let mut view = flight();
    tap(&mut view, MAP);
    view.take_sounds();
    view.close_map();
    assert_eq!(ui_sounds(&mut view), [UiSound::MapClosed]);
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
fn the_hypergate_map_makes_no_map_sound() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(map_sounds(&mut view), [], "opened");
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(map_sounds(&mut view), [], "closed on a pick");
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    view.take_sounds();
    view.close_map();
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(map_sounds(&mut view), [], "cancelled");
}

#[test]
fn a_hypergates_greeting_is_one_of_its_own_by_a_roll() {
    let mut view = FlightView::new(gated());
    tap(&mut view, LAND);
    assert_eq!(view.message(), None, "requested over it: silent");
    ticks(&mut view, 1);
    let ship = ship_name(&view);
    let first = format!("Cleared for hypergate entry, {ship}. ");
    assert_eq!(view.message(), Some(first.as_str()), "the first outcome");
    let last: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Always::default()));
    let mut view = FlightView::new(gated()).with_chance(SharedChance::new(last));
    tap(&mut view, LAND);
    ticks(&mut view, 1);
    assert_lasts(&mut view, "You are cleared for hypergate entry. ", 250);
}

#[test]
fn a_hypergate_the_pilot_may_not_use_denies_it_in_its_own_words() {
    let mut catalog = gated();
    catalog.sites[0].min_status = 100;
    let mut view = FlightView::new(catalog);
    tap(&mut view, LAND);
    assert_lasts(&mut view, HYPERGATE_DENIED, 250);
}

#[test]
fn an_offline_gate_is_refused_in_its_own_words() {
    for (flags2, kind) in [(0x1000, GateKind::Hypergate), (0x2000, GateKind::Wormhole)] {
        let mut view = flight_among(vec![LandingSite {
            flags2,
            ..site(300, (0.0, 0.0), StellarFlags::STATION)
        }]);
        tap_slot(&mut view, STELLAR_SLOT_KEYS[0][0]);
        tap(&mut view, LAND);
        let refusal = gate_refusal_message(kind);
        assert_lasts(&mut view, &refusal, 250);
    }
    let mut planet = flight_among(vec![site(128, (0.0, 0.0), 0)]);
    tap_slot(&mut planet, STELLAR_SLOT_KEYS[0][0]);
    tap(&mut planet, LAND);
    let line = "Your ship is unable to land on  Earth. The planet's environment is too hostile.";
    assert_lasts(&mut planet, line, 250);
}

#[test]
fn l_twice_at_a_hypergate_opens_the_map_offering_its_links() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    assert!(view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(view.take_landing(), None, "not landed");
    assert_eq!(system_of(&view), SystemId(130));
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
fn closing_the_hypergate_map_on_a_pick_comes_out_of_its_gate() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(player(&view).position, Vec2::new(100.0, 200.0));
    assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Centauri"));
    assert_eq!(
        view.message(),
        Some("Exiting hypergate in the Alpha Centauri system on June 23rd, 1177 NC.")
    );
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    assert_eq!(view.shown_position(), Point::new(100.0, 200.0));
    view.tick(FADE_IN_FOR);
    assert_eq!(view.jump_effect(), None, "the fade is over");
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(
        view.course_map().current(),
        Some(SystemId(131)),
        "the map follows"
    );
}

#[test]
fn the_ship_flies_under_a_hypergates_fade_in() {
    let mut view = FlightView::new(gated());
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

#[test]
fn a_hypergate_whose_gates_system_is_hidden_is_picked_by_the_one_at_its_place() {
    let mut view = FlightView::new(FakeCatalog {
        reborn: true,
        ..gated()
    });
    change_galaxy(&mut view, hiding(131));
    view.tick(TICK);
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    let reborn = on_map(&mut view, 133);
    click(&mut view, reborn);
    tap(&mut view, MAP);
    assert_eq!(system_of(&view), SystemId(133));
    assert_eq!(player(&view).position, Vec2::new(100.0, 200.0), "at 310");
    assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Reborn"));
}

#[test]
fn escape_on_the_hypergate_map_enters_the_gate_too() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    view.close_map();
    assert_eq!(system_of(&view), SystemId(131));
}

#[test]
fn closing_the_hypergate_map_without_a_pick_cancels() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(system_of(&view), SystemId(130));
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(view.jump_effect(), None);
    assert_eq!(nav_target(&view), None);
    // The map is a course map again.
    tap(&mut view, MAP);
    assert_eq!(view.course_map().mode(), MapMode::Course);
    tap(&mut view, MAP);
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED), "nothing new");
    assert_lasts(&mut view, HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS);
}
