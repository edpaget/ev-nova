use super::*;

// Building.

#[test]
fn it_starts_the_session_in_its_system_at_rest() {
    let view = flight();
    let session = view.session().expect("flying");
    assert_eq!(session.ship(), ShipId(128));
    assert_eq!(session.system(), SystemId(130));
    assert_eq!(session.handling(), handling());
    assert_eq!(*session.player(), start());
    let scene = view.scene().expect("a scene");
    assert_eq!(scene.id(), SystemId(130));
    assert_eq!(scene.stellars().len(), 2);
    assert_eq!(view.camera().center(), at(0.0, 0.0));
    assert_eq!(view.shown_position(), at(0.0, 0.0));
    assert_eq!(view.frame(), Some(0));
}

/// [`catalog`] with `dësc`s 3000 "{b5 "set" "clear"}, sir.", 3001
/// "<SRK128>" and 3002 "{G "<SRK128>" "x"}".
fn described() -> FakeCatalog {
    FakeCatalog {
        descriptions: vec![
            (3000, "{b5 \"set\" \"clear\"}, sir.".to_owned()),
            (3001, "<SRK128>".to_owned()),
            (3002, "{G \"x\" \"<SRK128>\"} ".to_owned()),
        ],
        ..catalog()
    }
}

#[test]
fn a_loaded_description_reads_the_pilot_as_it_is_now() {
    let mut view = FlightView::new(described());
    assert_eq!(view.load_description(3000).as_deref(), Ok("clear, sir."));
    view.session
        .as_mut()
        .expect("flying")
        .set_control_bit(nova_sim::Bit::new(5).expect("a bit"), true);
    assert_eq!(view.load_description(3000).as_deref(), Ok("set, sir."));
}

#[test]
fn a_loaded_description_resets_and_parses_the_rank_governments() {
    let mut view = FlightView::new(described());
    let srk = |view: &View| view.session().expect("flying").plain_text("<SRK128>");
    assert_eq!(srk(&view), "<SRK128>", "no government remembered");
    assert_eq!(view.load_description(3001).as_deref(), Ok("<SRK128>"));
    assert_eq!(srk(&view), "captain", "<SRK128> remembered, crossed");
    assert_eq!(
        view.load_description(9999),
        Err("no dësc 9999".to_owned()),
        "a missing one"
    );
    assert_eq!(srk(&view), "captain", "kept by a missing one");
    assert_eq!(view.load_description(3002).as_deref(), Ok("x "));
    assert_eq!(
        srk(&view),
        "<SRK128>",
        "reset, and the string not chosen never scanned"
    );
}

#[test]
fn load_description_records_the_descs_movie() {
    let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
    let mut view = FlightView::new(FakeCatalog {
        desc_movies: vec![(3000, movie.clone())],
        ..described()
    });
    assert_eq!(view.loaded_after_movie(), None, "none loaded yet");
    let _ = view.load_description(3000);
    assert_eq!(
        view.loaded_after_movie(),
        Some(movie),
        "a STR# text shown now plays it after (@0x5b40b)"
    );
    let _ = view.load_description(3001);
    assert_eq!(view.loaded_after_movie(), None, "a dësc naming none");
}

#[test]
fn an_unreadable_desc_empties_it() {
    let movie = nova_sim::DescMovie::from_desc("Port.mov", 0x0001).expect("named");
    let mut view = FlightView::new(FakeCatalog {
        desc_movies: vec![(3000, movie)],
        ..described()
    });
    let _ = view.load_description(3001);
    let _ = view.load_description(3000);
    assert!(view.load_description(9999).is_err());
    assert_eq!(
        view.loaded_after_movie(),
        None,
        "as a missing dësc empties the MovieFile (@0x71a67-0x71a77)"
    );
    let srk = view.session().expect("flying").plain_text("<SRK128>");
    assert_eq!(srk, "<SRK128>", "and keeps the governments 3000 left");
}

#[test]
fn with_no_session_a_loaded_description_reads_a_new_pilot() {
    let mut view = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..described()
    });
    assert_eq!(view.load_description(3000).as_deref(), Ok("clear, sir."));
    assert_eq!(view.load_description(3002).as_deref(), Ok("x "));
}

#[test]
fn a_session_that_cannot_start_says_why_and_draws_nothing_else() {
    let failed = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let mut view = FlightView::new(failed);
    assert_eq!(view.session().err(), Some("no chär to start from"));
    assert!(view.scene().is_none());
    assert_eq!(view.frame(), None);
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(
        drawn(&view).iter().cloned().collect::<Vec<_>>(),
        [DrawCommand::Text {
            text: "Cannot start flight: no chär to start from".to_owned(),
            font: Font::Geneva,
            origin: TITLE,
            size: TITLE_SIZE,
            wrap_width: None,
            color: Color::ERROR,
        },]
    );
}

#[test]
fn the_unstarted_flight_shows_help_only_when_turned_on() {
    let failed = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let view = FlightView::new(failed).with_overlays(FlightOverlays {
        title: false,
        help: true,
    });
    let list = drawn(&view);
    let help = overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM);
    assert_eq!(list.iter().last(), Some(&help));
    assert_eq!(list.iter().count(), 2, "the error line and the help");
}

// Flying.

#[test]
fn holding_up_thrusts_the_ship_up_and_the_camera_follows() {
    let mut view = flight();
    assert_eq!(view.input(&key(Key::Up, true)), ScreenAction::None);
    view.tick(Duration::from_secs(1));
    let expected = stepped(start(), THRUST, 30);
    assert_eq!(player(&view), expected);
    // 0.06 more a tick for 30 ticks: 0.06 x 465 = 27.9 up.
    assert!(expected.position.y < -27.0, "{expected:?}");
    assert_eq!(expected.position.x, 0.0);
    assert_eq!(view.camera().center(), view.shown_position());
    let shown = view.shown_position();
    // Drawn at most a step behind.
    let behind = shown.y - expected.position.y;
    assert!((0.0..=3.0 + 1e-4).contains(&behind), "{shown:?}");
}

#[test]
fn releasing_up_coasts() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 10);
    view.input(&key(Key::Up, false));
    let coasting = player(&view);
    ticks(&mut view, 10);
    let later = player(&view);
    assert_eq!(later.velocity, coasting.velocity);
    assert_eq!(later, stepped(coasting, Controls::default(), 10));
}

#[test]
fn left_and_right_turn_the_ship_and_its_frame() {
    let mut view = flight();
    view.input(&key(Key::Right, true));
    ticks(&mut view, 11);
    // 30° after 10 steps, drawn a step behind.
    assert_eq!(player(&view).heading, 33.0);
    assert_eq!(view.shown_heading(), 30.0);
    assert_eq!(view.frame(), Some(3));
    view.input(&key(Key::Right, false));
    view.input(&key(Key::Left, true));
    ticks(&mut view, 21);
    assert_eq!(player(&view).heading, 330.0);
    assert_eq!(view.frame(), Some(33));
    assert_eq!(player(&view).velocity, Vec2::ZERO, "turning alone");
}

#[test]
fn left_and_right_together_cancel() {
    let mut view = flight();
    view.input(&key(Key::Left, true));
    view.input(&key(Key::Right, true));
    ticks(&mut view, 5);
    assert_eq!(player(&view).heading, 0.0);
}

#[test]
fn down_turns_the_ship_against_its_motion() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 30);
    view.input(&key(Key::Up, false));
    view.input(&key(Key::Down, true));
    ticks(&mut view, 70);
    // Within a turn of 180, without snapping onto it.
    let rate = view.session().expect("flying").handling().turn_rate;
    assert!(
        nova_sim::player_turn::settled(
            player(&view).heading,
            rate,
            180,
            nova_sim::player_turn::PlayerTurn::WholeDegrees
        ),
        "{:?}",
        player(&view)
    );
    let stopped = player(&view).heading;
    assert_eq!(stopped, 177.0, "a turn short");
    ticks(&mut view, 5);
    assert_eq!(player(&view).heading, stopped, "it stays there");
    assert_eq!(view.frame(), Some(17));
    // Left or Right overrides it.
    view.input(&key(Key::Left, true));
    ticks(&mut view, 1);
    assert!((player(&view).heading - 177.0).abs() < 1e-3);
}

#[test]
fn repeats_hold_a_key_and_releasing_the_keys_stops_the_thrust() {
    let mut view = flight();
    view.input(&held(Key::Up));
    ticks(&mut view, 5);
    assert_eq!(player(&view), stepped(start(), THRUST, 5));
    view.release_keys();
    let coasting = player(&view);
    ticks(&mut view, 5);
    assert_eq!(player(&view), stepped(coasting, Controls::default(), 5));
}

#[test]
fn a_release_without_a_press_does_nothing() {
    let mut view = flight();
    view.input(&key(Key::Up, false));
    ticks(&mut view, 3);
    assert_eq!(player(&view), start());
}

#[test]
fn other_input_changes_nothing_and_never_quits() {
    let mut view = flight();
    let others = [
        key(Key::Escape, true),
        key(Key::Space, true),
        key(Key::Tab, true),
        key(Key::Space, true),
        key(Key::Enter, true),
        key(Key::Char('w'), true),
        key(Key::Char('w'), true),
        key(Key::Other, true),
        Input::PointerMoved(at(1.0, 2.0)),
        Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed: true,
            at: at(1.0, 2.0),
        },
    ];
    for input in others {
        assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
    }
    ticks(&mut view, 5);
    assert_eq!(player(&view), start());
}

// Interpolation.

#[test]
fn the_ship_is_drawn_alpha_of_the_way_from_the_last_step_to_this_one() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    view.tick(TICK);
    let first = stepped(start(), THRUST, 1);
    assert_eq!(player(&view), first);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(0.0, 0.0), "a step behind");
    view.tick(TICK / 2);
    assert!((view.alpha() - 0.5).abs() < 1e-6, "{}", view.alpha());
    assert_eq!(player(&view), first, "no step yet");
    let half = view.shown_position();
    assert!((half.y - first.position.y / 2.0).abs() < 1e-6, "{half:?}");
    assert_eq!(half.x, 0.0);
    assert_eq!(view.camera().center(), half);
    // The rest of the tick: one more step, and back to its start.
    view.tick(REST_OF_TICK);
    let second = stepped(first, THRUST, 1);
    assert_eq!(player(&view), second);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(
        view.shown_position(),
        at(first.position.x, first.position.y)
    );
}

#[test]
fn mid_flight_the_ship_is_drawn_between_its_last_two_positions() {
    let mut view = flight();
    view.input(&key(Key::Right, true));
    ticks(&mut view, 15);
    view.input(&key(Key::Right, false));
    view.input(&key(Key::Up, true));
    ticks(&mut view, 20);
    let before = player(&view);
    ticks(&mut view, 1);
    let after = player(&view);
    view.tick(TICK / 4);
    let shown = view.shown_position();
    let expected = |from: f32, to: f32| from + (to - from) * view.alpha();
    assert!(
        before.position.x > 1.0 && before.position.y < -1.0,
        "{before:?}"
    );
    assert!((shown.x - expected(before.position.x, after.position.x)).abs() < 1e-4);
    assert!((shown.y - expected(before.position.y, after.position.y)).abs() < 1e-4);
}

#[test]
fn the_heading_is_drawn_the_short_way_round_across_0() {
    let mut view = flight();
    view.input(&key(Key::Left, true));
    view.tick(TICK);
    assert_eq!(player(&view).heading, 357.0);
    view.tick(TICK / 2);
    assert!(
        (view.shown_heading() - 358.5).abs() < 1e-3,
        "{}",
        view.shown_heading()
    );
    view.input(&key(Key::Left, false));
    view.input(&key(Key::Right, true));
    view.tick(REST_OF_TICK);
    view.tick(TICK / 2);
    // From 357 to 0: half way is 358.5 again, not 178.5.
    assert_eq!(player(&view).heading, 0.0);
    assert!(
        (view.shown_heading() - 358.5).abs() < 1e-3,
        "{}",
        view.shown_heading()
    );
}

// Drawing.

/// The radar's pixels drawn, in either radar colour, row by row as
/// drawn.
fn dots_on_radar(list: &DrawList) -> Vec<(i32, i32)> {
    let layout = layout();
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Dot {
                center,
                size: 1.0,
                color,
            } if color == layout.dim_radar || color == layout.bright_radar => {
                Some(((center.x - 0.5) as i32, (center.y - 0.5) as i32))
            }
            _ => None,
        })
        .collect()
}

/// The 2×2 oval a small stellar at radar pixel (`x`, `y`) draws.
fn small_oval(x: i32, y: i32) -> RadarMark {
    RadarMark::Oval {
        rect: IRect::around(x, y, 1),
        color: RadarColor::Dim,
    }
}

#[test]
fn the_title_and_help_are_not_drawn_by_default() {
    assert_eq!(
        FlightOverlays::default(),
        FlightOverlays {
            title: false,
            help: false
        }
    );
    assert_eq!(flight().overlays(), FlightOverlays::default());
    assert_eq!(flight().with_overlays(TITLED).overlays(), TITLED);
    let texts = texts(&drawn(&flight()));
    assert!(!texts.contains(&"Sol (sÿst 130)".to_owned()), "{texts:?}");
    assert!(!texts.contains(&HELP.to_owned()), "{texts:?}");
}

/// The flight's draw list's commands up to the HUD's first, whose
/// status bar background is the first picture.
fn before_the_hud(list: &DrawList) -> Vec<DrawCommand> {
    list.iter()
        .take_while(|c| !matches!(c, DrawCommand::Picture { .. }))
        .cloned()
        .collect()
}

#[test]
fn the_title_draws_when_turned_on() {
    let titled = FlightOverlays {
        title: true,
        help: false,
    };
    let list = drawn(&flight().with_overlays(titled));
    let before = before_the_hud(&list);
    assert_eq!(
        before.last(),
        Some(&overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE)),
        "over the flight, under the HUD"
    );
    assert!(!texts(&list).contains(&HELP.to_owned()));
}

#[test]
fn the_help_line_draws_when_turned_on() {
    let helped = FlightOverlays {
        title: false,
        help: true,
    };
    let list = drawn(&flight().with_overlays(helped));
    let before = before_the_hud(&list);
    assert_eq!(
        before.last(),
        Some(&overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM)),
        "over the flight, under the HUD"
    );
    assert!(!texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
    let both = drawn(&flight().with_overlays(TITLED));
    let before = before_the_hud(&both);
    assert_eq!(
        before[before.len() - 2..],
        [
            overlay("Sol (sÿst 130)", TITLE, TITLE_SIZE, Color::WHITE),
            overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM),
        ],
        "the title, then the help"
    );
}

#[test]
fn it_draws_the_stars_the_stellars_the_ship_then_the_hud() {
    let view = flight();
    let list = drawn(&view);
    let mut expected = DrawList::new();
    starfield::draw(&mut expected, &view.camera());
    scene::draw_stellars(
        &mut expected,
        view.scene().expect("a scene"),
        &view.camera(),
        Duration::ZERO,
    );
    expected.sprite(ImageKey::sprite(2000, 0), VIEW_CENTER, Color::WHITE);
    hud::draw(
        &mut expected,
        view.status_bar().expect("a status bar"),
        &HudState {
            radar: &RadarScene {
                player: at(0.0, 0.0),
                stellars: vec![
                    RadarStellar {
                        position: at(0.0, -600.0),
                        square: false,
                        sprite_width: Some(20),
                        iff: StellarIff::Unlandable,
                    },
                    RadarStellar {
                        position: at(300.0, -200.0),
                        square: false,
                        sprite_width: Some(20),
                        iff: StellarIff::Unlandable,
                    },
                ],
                ..RadarScene::default()
            },
            blink_on: false,
            reserves: ShipStats::new(FIELDS, &[]).full(),
            nav: NavDisplay::Off,
            cargo: Some(&view.session().expect("flying").status_cargo()),
            disabled: false,
        },
        None,
    );
    let bar = view.status_bar().expect("a status bar");
    let origin = hud::bar_origin(bar);
    target::draw_target_panel(&mut expected, &bar.layout, origin, None, None);
    target::draw_secondary(&mut expected, &bar.layout, origin, None, None);
    assert_eq!(list, expected);
    assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
    assert_eq!(
        sprites(&list),
        [
            (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
            (ImageKey::sprite(1129, 0), at(812.0, 184.0)),
            (ImageKey::sprite(2000, 0), VIEW_CENTER),
        ]
    );
    assert_eq!(
        HELP,
        "Arrows: fly   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: player info   I: mission info   Cmd-,: preferences   Esc: leave"
    );
    assert_eq!((TITLE, HELP_AT), (at(16.0, 32.0), at(16.0, 744.0)));
}

#[test]
fn the_ship_stays_at_the_screens_centre_as_the_stellars_go_by() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    view.input(&key(Key::Right, true));
    view.tick(Duration::from_millis(1500));
    let list = drawn(&view);
    let shown = sprites(&list);
    let camera = view.camera();
    assert_eq!(
        shown[2],
        (
            ImageKey::sprite(2000, view.frame().expect("a sheet")),
            VIEW_CENTER
        )
    );
    assert_ne!(view.frame(), Some(0));
    assert_eq!(shown[0].1, camera.world_to_screen(at(0.0, -600.0)));
    assert_ne!(shown[0].1, at(512.0, -216.0), "the stellars moved");
    let mut stars = DrawList::new();
    starfield::draw(&mut stars, &camera);
    assert!(
        list.iter().take(stars.len()).eq(stars.iter()),
        "stars under the camera"
    );
}

#[test]
fn the_stellars_animate_with_time() {
    let mut view = flight();
    ticks(&mut view, 1);
    assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 1));
    ticks(&mut view, 2);
    assert_eq!(sprites(&drawn(&view))[1].0, ImageKey::sprite(1129, 3));
}

#[test]
fn a_ship_without_a_sheet_is_a_placeholder_with_the_reason() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..catalog()
    };
    let view = FlightView::new(sheetless);
    assert_eq!(view.frame(), None);
    let list = drawn(&view);
    assert_eq!(sprites(&list).len(), 2, "the stellars only");
    let mut expected = DrawList::new();
    crossed_box(&mut expected, VIEW_CENTER, PLACEHOLDER_SIZE, PLACEHOLDER);
    assert_eq!(
        lines(&list)[..expected.len()],
        *lines(&expected),
        "then the HUD's"
    );
    let reason = list
        .iter()
        .find(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite")))
        .cloned();
    assert_eq!(
        reason,
        Some(overlay(
            "Sprite unavailable: no shän 128 for shïp 128",
            at(
                VIEW_CENTER.x - PLACEHOLDER_SIZE / 2.0,
                VIEW_CENTER.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP
            ),
            OVERLAY_SIZE,
            Color::ERROR
        ))
    );
    assert!(matches!(list.iter().next(), Some(DrawCommand::Dot { .. })));
}

// The engine glow and running lights.

/// [`sheet`] with a 36-frame glow, `rlëD` 2100, and 12-frame lights,
/// `rlëD` 2200.
fn layered() -> FakeCatalog {
    FakeCatalog {
        sheet: Ok(ShipSheet {
            glow: Some(layer(2100, 36)),
            lights: Some(layer(2200, 12)),
            ..sheet()
        }),
        ..catalog()
    }
}

#[test]
fn a_coasting_ship_draws_its_lights_over_it_and_no_glow() {
    let mut view = FlightView::new(layered());
    assert_eq!(
        ship_sprites(&view),
        [
            (ImageKey::sprite(2000, 0), VIEW_CENTER),
            (ImageKey::sprite(2200, 0), VIEW_CENTER),
        ]
    );
    view.input(&key(Key::Up, true));
    assert_eq!(ship_sprites(&view).len(), 2, "held, but not yet flown");
    ticks(&mut view, 3);
    view.input(&key(Key::Up, false));
    // As many ticks coasting as thrusting: the glow has faded out.
    ticks(&mut view, 3);
    let session = view.session().expect("flying");
    assert!(!session.thrusting());
    assert_eq!(session.engine_glow(), 0);
    assert_eq!(
        ship_sprites(&view)
            .iter()
            .map(|(image, _)| image.id)
            .collect::<Vec<_>>(),
        [2000, 2200],
        "coasting"
    );
}

#[test]
fn a_thrusting_ship_draws_its_glow_between_it_and_its_lights() {
    let mut view = FlightView::new(layered());
    view.input(&key(Key::Up, true));
    // A base of 6: drawn at every roll.
    ticks(&mut view, 6);
    let centre = view.camera().world_to_screen(view.shown_position());
    assert_eq!(
        ship_sprites(&view),
        [
            (ImageKey::sprite(2000, 0), centre),
            (ImageKey::sprite(2100, 0), centre),
            (ImageKey::sprite(2200, 0), centre),
        ]
    );
}

/// The ship's sprites' image ids and blends, as [`ship_sprites`].
fn ship_blends(view: &View) -> Vec<(i16, Blend)> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, blend, .. } if (2000..2300).contains(&image.id) => {
                Some((image.id, *blend))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_thrusting_ships_glow_and_lights_or_onto_its_normal_sprite() {
    let mut view = FlightView::new(layered());
    view.input(&key(Key::Up, true));
    ticks(&mut view, 6);
    assert_eq!(
        ship_blends(&view),
        [(2000, Blend::Normal), (2100, Blend::Or), (2200, Blend::Or),]
    );
    assert!(
        !drawn(&view).iter().any(|command| matches!(
            command,
            DrawCommand::Sprite {
                blend: Blend::Additive,
                ..
            }
        )),
        "nothing in a thrusting, lit frame is added"
    );
    view.input(&key(Key::Up, false));
    ticks(&mut view, 6);
    assert_eq!(view.session().expect("flying").engine_glow(), 0);
    assert_eq!(
        ship_blends(&view),
        [(2000, Blend::Normal), (2200, Blend::Or)],
        "coasting"
    );
}

#[test]
fn the_layers_turn_with_the_ship_between_steps() {
    let mut view = FlightView::new(layered());
    view.input(&key(Key::Up, true));
    view.input(&key(Key::Right, true));
    // 50 steps and a half: the display is between two steps.
    ticks(&mut view, 50);
    view.tick(TICK / 2);
    assert_ne!(view.shown_heading(), player(&view).heading, "between");
    let frame = view.frame().expect("a sheet");
    assert_eq!(frame, 14, "148.5, truncated: {}", view.shown_heading());
    let centre = view.camera().world_to_screen(view.shown_position());
    assert_eq!(
        ship_sprites(&view),
        [
            (ImageKey::sprite(2000, frame), centre),
            (ImageKey::sprite(2100, frame), centre),
            (ImageKey::sprite(2200, frame % 12), centre),
        ]
    );
}

#[test]
fn landing_while_thrusting_puts_the_glow_out() {
    let mut view = FlightView::new(FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..layered()
    });
    view.input(&key(Key::Up, true));
    ticks(&mut view, 6);
    assert_eq!(ship_sprites(&view).len(), 3, "glowing");
    land_now(&mut view);
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert_eq!(view.session().expect("flying").engine_glow(), 0);
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2000, 2200], "landed");
}

#[test]
fn beginning_a_jump_while_thrusting_puts_the_glow_out() {
    let mut view = FlightView::new(layered());
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(Key::Up, true));
    ticks(&mut view, 6);
    assert_eq!(ship_sprites(&view).len(), 3, "glowing");
    jump_now(&mut view);
    assert_eq!(view.session().expect("flying").engine_glow(), 0);
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2000, 2200], "jumping");
}

#[test]
fn a_ship_without_layers_draws_only_its_sprite() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert!(view.session().expect("flying").thrusting());
    let ids: Vec<_> = ship_sprites(&view).iter().map(|(i, _)| i.id).collect();
    assert_eq!(ids, [2000]);
}

#[test]
fn a_ship_without_a_sheet_draws_no_layers() {
    let mut view = FlightView::new(FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..layered()
    });
    view.input(&key(Key::Up, true));
    ticks(&mut view, 3);
    assert_eq!(ship_sprites(&view), []);
}

/// Every command drawing the glow, `rlëD` 2100, whatever its blend.
fn glow_commands(view: &View) -> Vec<DrawCommand> {
    drawn(view)
        .iter()
        .filter(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2100))
        .cloned()
        .collect()
}

/// The glow commands for a base of `base` at `tick`: the glow drawn by
/// OR at the level [`glow_level`] gives over the ship's centre, on its
/// frame, or none when it is hidden.
fn glow_for(view: &View, base: u8, tick: u64) -> Vec<DrawCommand> {
    let frame = view.frame().expect("a sheet") % 36;
    let center = view.camera().world_to_screen(view.shown_position());
    glow_level(base, tick, &view.glow_rolls)
        .map(|level| DrawCommand::Sprite {
            image: ImageKey::sprite(2100, frame),
            center,
            tint: lights_tint(level),
            blend: Blend::Or,
        })
        .into_iter()
        .collect()
}

#[test]
fn the_glow_ramps_in_and_flickers_at_the_glow_function_level() {
    let mut view = FlightView::new(layered());
    view.input(&key(Key::Up, true));
    let mut cruising = Vec::new();
    for n in 1..=40_u8 {
        ticks(&mut view, 1);
        let base = n.min(GLOW_CRUISE);
        assert_eq!(view.session().expect("flying").engine_glow(), base);
        let want = glow_for(&view, base, u64::from(n));
        assert_eq!(glow_commands(&view), want, "tick {n}");
        if n >= 6 {
            assert_eq!(want.len(), 1, "a base of 6 or more always shows");
        }
        if n >= 24 {
            cruising.extend(want);
        }
    }
    cruising.dedup();
    assert!(cruising.len() >= 2, "it flickers: {cruising:?}");
}

#[test]
fn releasing_up_fades_the_glow_out() {
    let mut view = FlightView::new(layered());
    view.input(&key(Key::Up, true));
    ticks(&mut view, 30);
    assert_eq!(view.session().expect("flying").engine_glow(), GLOW_CRUISE);
    view.input(&key(Key::Up, false));
    for k in 1..=40_u8 {
        ticks(&mut view, 1);
        let base = GLOW_CRUISE.saturating_sub(k);
        assert_eq!(view.session().expect("flying").engine_glow(), base);
        let want = glow_for(&view, base, 30 + u64::from(k));
        assert_eq!(glow_commands(&view), want, "{k} ticks after");
        if k == 3 {
            let level = glow_level(21, 33, &view.glow_rolls).expect("still drawn");
            let alpha = lights_tint(level).a;
            assert!(alpha < 255 && alpha >= lights_tint(17).a, "{alpha}");
            assert_eq!(want.len(), 1, "fading, not out");
        }
        if k >= 24 {
            assert_eq!(want, [], "out {k} ticks after");
        }
    }
}

// The running lights' blinking.

/// The Shuttle's blink: a double flash every 40 ticks, lit at ticks
/// 1-3 and 10-12.
const SHUTTLE: Blink = Blink {
    mode: 1,
    a: 4,
    b: 1,
    c: 2,
    d: 20,
};

/// The tints the lights, `rlëD` 2200, are drawn by OR with.
fn lights_tints(view: &View) -> Vec<Color> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite {
                image,
                tint,
                blend: Blend::Or,
                ..
            } if image.id == 2200 => Some(*tint),
            _ => None,
        })
        .collect()
}

/// The lights' alphas drawn: one, or none when they are off.
fn lights_alphas(view: &View) -> Vec<u8> {
    lights_tints(view).iter().map(|tint| tint.a).collect()
}

#[test]
fn the_shuttles_lights_blink_with_game_time() {
    let mut view = FlightView::new(blinking(SHUTTLE));
    assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 0: off");
    assert_eq!(ship_sprites(&view)[0].0.id, 2000, "the ship is drawn");
    ticks(&mut view, 1);
    assert_eq!(lights_tints(&view), [Color::WHITE], "tick 1: on");
    ticks(&mut view, 3);
    assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4: off");
    ticks(&mut view, 6);
    assert_eq!(lights_alphas(&view), [255], "tick 10: on again");
    ticks(&mut view, 3);
    assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 13: the gap");
}

#[test]
fn a_pulsing_light_is_ored_at_its_level() {
    // The stock triangle: 10 to 31.
    let triangle = Blink {
        mode: 2,
        a: 10,
        b: 75,
        c: 32,
        d: 75,
    };
    let mut view = FlightView::new(blinking(triangle));
    assert_eq!(lights_alphas(&view), [0_u8; 0], "0.75: not drawn");
    ticks(&mut view, 1);
    assert_eq!(lights_alphas(&view), [8], "level 1");
    ticks(&mut view, 40);
    assert_eq!(lights_alphas(&view), [247], "level 31");
}

/// Every command drawing the lights, `rlëD` 2200, whatever its blend.
fn lights_commands(view: &View) -> Vec<DrawCommand> {
    drawn(view)
        .iter()
        .filter(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2200))
        .cloned()
        .collect()
}

/// The lights drawn by OR with `tint` over the coasting ship's centre.
fn lights_at(tint: Color) -> DrawCommand {
    DrawCommand::Sprite {
        image: ImageKey::sprite(2200, 0),
        center: VIEW_CENTER,
        tint,
        blend: Blend::Or,
    }
}

#[test]
fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
    // The stock triangle, 10 to 31, never full: the original draws
    // every level with its translucent blit, which ORs in the lights
    // scaled by level/32 (see `lights_tint`).
    let triangle = Blink {
        mode: 2,
        a: 10,
        b: 75,
        c: 32,
        d: 75,
    };
    let rolls = HashedRolls::new(0);
    let mut view = FlightView::new(blinking(triangle));
    let mut levels = Vec::new();
    for tick in 0..=101 {
        let level = lights_level(&triangle, tick, &rolls);
        let want: Vec<_> = level
            .map(|n| lights_at(lights_tint(n)))
            .into_iter()
            .collect();
        assert_eq!(lights_commands(&view), want, "tick {tick}");
        levels.extend(level);
        ticks(&mut view, 1);
    }
    assert!(levels.contains(&10), "the lowest level is drawn");
    assert!(levels.contains(&31), "the highest level is drawn");

    // A steady light is full: ORed whole.
    let view = FlightView::new(blinking(Blink::STEADY));
    assert_eq!(lights_commands(&view), [lights_at(Color::WHITE)]);
}

#[test]
fn steady_lights_are_ored_at_full_from_the_start() {
    let view = FlightView::new(blinking(Blink::STEADY));
    assert_eq!(lights_tints(&view), [Color::WHITE]);
}

#[test]
fn the_glow_still_shows_while_the_lights_are_off() {
    let mut view = FlightView::new(blinking(SHUTTLE));
    view.input(&key(Key::Up, true));
    // Tick 13: the Shuttle's lights are dark, and a base of 13 always
    // shows the glow.
    ticks(&mut view, 13);
    assert!(view.session().expect("flying").thrusting());
    assert_eq!(
        ship_blends(&view),
        [(2000, Blend::Normal), (2100, Blend::Or)]
    );
    let glow = drawn(&view).iter().find_map(|command| match command {
        DrawCommand::Sprite { image, tint, .. } if image.id == 2100 => Some(*tint),
        _ => None,
    });
    let level = glow_level(13, 13, &view.glow_rolls).expect("drawn");
    assert_eq!(glow, Some(lights_tint(level)));
}

#[test]
fn the_blink_pauses_while_the_map_is_open() {
    let mut view = FlightView::new(blinking(SHUTTLE));
    ticks(&mut view, 3);
    assert_eq!(lights_alphas(&view), [255], "tick 3");
    view.input(&key(MAP_KEY, true));
    assert!(view.map_open());
    // Tick 103 would be dark.
    ticks(&mut view, 100);
    view.close_map();
    assert_eq!(lights_alphas(&view), [255], "still tick 3");
    ticks(&mut view, 1);
    assert_eq!(lights_alphas(&view), [0_u8; 0], "tick 4");
}

#[test]
fn random_lights_roll_on_hashed_rolls() {
    // A=5 B=12 C=3: one of 8 levels from 5, every 4 ticks.
    let random = Blink {
        mode: 3,
        a: 5,
        b: 12,
        c: 3,
        d: 0,
    };
    let rolls = HashedRolls::new(0);
    let expected = |change| lights_tint(5 + rolls.roll(change, 8) as u8).a;
    let mut view = FlightView::new(blinking(random));
    assert_eq!(lights_alphas(&view), [expected(0)]);
    ticks(&mut view, 4);
    assert_eq!(lights_alphas(&view), [expected(1)]);
}

// The HUD.

#[test]
fn a_new_pilot_shows_the_default_status_bar() {
    let view = flight();
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: layout(),
            background: Some(hud::Background {
                id: 700,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
}

#[test]
fn the_status_bar_alone_is_flights_bar_and_nothing_else() {
    let view = landed_view().with_overlays(TITLED);
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    let background = DrawCommand::Picture {
        image: ImageKey::picture(700),
        top_left: at(830.0, 0.0),
    };
    assert_eq!(bar.iter().next(), Some(&background));
    assert_eq!(dots_on_radar(&bar), [(926, 96)], "the blank radar");
    assert_eq!(sprites(&bar), [], "no stellars or ship");
    assert!(!texts(&bar).contains(&HELP.to_owned()), "no help line");
    let flown = drawn(&view);
    let from = flown
        .iter()
        .position(|command| *command == background)
        .expect("flight draws the bar");
    assert!(from > 0, "flight draws the stars first");
    assert_eq!(
        bar.iter().collect::<Vec<_>>(),
        flown.iter().skip(from).collect::<Vec<_>>(),
        "the bar as flight draws it"
    );
    let barless = FakeCatalog {
        bar: Err("no ïntf 128".to_owned()),
        ..catalog()
    };
    let view = FlightView::with_pilot(barless, docked_pilot("Ada"));
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    assert_eq!(bar.len(), 0, "nothing, not even why");
}

#[test]
fn the_hud_shows_the_status_bar_full_bars_and_the_stellars_on_radar() {
    let view = flight();
    let list = drawn(&view);
    assert!(
        list.iter().any(|c| *c
            == DrawCommand::Picture {
                image: ImageKey::picture(700),
                top_left: at(830.0, 0.0),
            }),
        "{list:?}"
    );
    // The shïp's shield 40, armour 60 and fuel 250: all of it in the
    // full colour, then the half jump past the two whole ones (from
    // 119.2 pixels) over it.
    let width = 149.0;
    let bar = |top: f32, from: f32, to: f32, color| DrawCommand::Line {
        from: at(865.0 + from, top + 3.5),
        to: at(865.0 + to, top + 3.5),
        width: 7.0,
        color,
    };
    let layout = layout();
    assert_eq!(
        lines(&list),
        [
            bar(199.0, 0.0, width, layout.shield_color),
            bar(216.0, 0.0, width, layout.armor_color),
            bar(234.0, 0.0, width, layout.fuel_full),
            bar(234.0, 119.0, width, layout.fuel_partial),
        ]
    );
    // Earth, 600 up, is 18.75 pixels up the radar, at 77.25; the Moon,
    // (300, -200), at (935.375, 89.75): small ovals, dim, then the
    // player, bright.
    assert_eq!(
        dots_on_radar(&list),
        [
            (925, 76),
            (926, 76),
            (925, 77),
            (926, 77),
            (934, 88),
            (935, 88),
            (934, 89),
            (935, 89),
            (926, 96)
        ]
    );
    assert_eq!(
        radar_marks(&view),
        [small_oval(926, 77), small_oval(935, 89), PLAYER_PIXEL]
    );
    // No system name in the nav area (the dev title still names it).
    assert!(!texts(&list).contains(&"Sol".to_owned()), "{list:?}");
}

#[test]
fn a_stellars_radar_shape_and_sprite_width_reach_the_radar() {
    // `w` x `h` frames: the radar reads the width, never the height.
    let sized = |id, at, (w, h): (u32, u32), square| StellarContents {
        sprite: Ok(StellarSheet {
            image_id: 1000 + id,
            frames: NonZeroU16::MIN,
            frame_width: w,
            frame_height: h,
        }),
        radar_square: square,
        ..stellar(id, "Extra", at, 1)
    };
    let mut sites = catalog().sites;
    sites.extend([
        site(150, (640.0, 0.0), StellarFlags::STATION),
        site(151, (-640.0, 320.0), 0),
        site(152, (0.0, 640.0), 0),
    ]);
    let view = FlightView::new(FakeCatalog {
        sites,
        sol_more: vec![
            sized(150, (640, 0), (300, 300), true),
            sized(151, (-640, 320), (120, 60), false),
            sized(152, (0, 640), (250, 150), false),
        ],
        ..catalog()
    });
    let dim = RadarColor::Dim;
    // A station is a 2×2 square whatever its size; past 89 pixels wide
    // an oval grows 2 a side, past 199, 3.
    assert_eq!(
        radar_marks(&view),
        [
            small_oval(926, 77),
            small_oval(935, 89),
            RadarMark::Square {
                rect: IRect::around(946, 96, 1),
                color: dim,
            },
            RadarMark::Oval {
                rect: IRect::around(906, 106, 2),
                color: dim,
            },
            RadarMark::Oval {
                rect: IRect::around(926, 116, 3),
                color: dim,
            },
            PLAYER_PIXEL,
        ]
    );
}

/// The radar's player position: where it was when the radar last
/// refreshed, or now.
fn radar_player(view: &View) -> Point {
    view.radar_shown().player
}

#[test]
fn by_the_engine_the_radar_holds_between_refreshes_eight_ticks_apart() {
    let mut view = flight();
    view.input(&key(Key::Up, true));
    ticks(&mut view, 1);
    let refreshed = view.shown_position();
    assert_eq!(radar_player(&view), refreshed, "refreshed on the first");
    for tick in 2..=8 {
        ticks(&mut view, 1);
        assert_eq!(radar_player(&view), refreshed, "held on tick {tick}");
    }
    assert_ne!(view.shown_position(), refreshed, "the ship moved");
    ticks(&mut view, 1);
    assert_eq!(radar_player(&view), view.shown_position(), "the 9th");
    assert_eq!(
        radar_marks(&view)[0],
        small_oval(
            926,
            radar::radar_offset(-600.0, view.shown_position().y, 96)
        )
    );
}

#[test]
fn by_the_other_reading_the_radar_follows_the_ship_every_frame() {
    let mut view = every_frame(flight());
    let before = dots_on_radar(&drawn(&view));
    view.input(&key(Key::Up, true));
    for _ in 0..45 {
        ticks(&mut view, 1);
        assert_eq!(radar_player(&view), view.shown_position());
    }
    view.tick(TICK / 2);
    let shown = view.shown_position();
    assert!(shown.y < -10.0, "{shown:?}");
    assert_eq!(radar_player(&view), shown, "between ticks too");
    let after = dots_on_radar(&drawn(&view));
    let y = |world| radar::radar_offset(world, shown.y, 96);
    assert_eq!(
        radar_marks(&view),
        [
            small_oval(926, y(-600.0)),
            small_oval(935, y(-200.0)),
            PLAYER_PIXEL
        ]
    );
    assert!(after[0].1 > before[0].1, "{before:?} {after:?}");
}

#[test]
fn while_landed_the_radar_shows_only_the_player_until_take_off() {
    for every in [false, true] {
        let view = flight_among(vec![
            site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
            site(129, (6.0, -8.0), StellarFlags::CAN_LAND),
        ]);
        let mut view = if every { every_frame(view) } else { view };
        ticks(&mut view, 3);
        let flying = radar_marks(&view);
        assert_eq!(flying.len(), 3, "the Sun, the Moon and the player");
        land_now(&mut view);
        assert!(view.session().expect("flying").landed().is_some());
        assert!(view.radar_shown().blank, "at once, as _RedrawStatus");
        assert_eq!(radar_marks(&view), [PLAYER_PIXEL]);
        let list = drawn(&view);
        assert_eq!(dots_on_radar(&list), [(926, 96)]);
        view.take_off().expect("took off");
        assert!(!view.radar_shown().blank, "at once on take-off");
        assert_eq!(radar_marks(&view).len(), 3);
    }
}

#[test]
fn a_new_system_shows_on_the_radar_at_once_leaving_the_timer_and_blink() {
    let mut view = landed_view();
    ticks(&mut view, 3);
    let cadence = view.hud_cadence;
    move_to(&mut view, 131, 140);
    view.resync();
    assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
    let shown = view.radar_shown();
    assert_eq!(shown.player, view.shown_position());
    let stellars: Vec<Point> = shown.stellars.iter().map(|s| s.position).collect();
    let scene: Vec<Point> = view
        .scene()
        .expect("a scene")
        .stellars()
        .iter()
        .map(|s| s.position)
        .collect();
    assert_eq!(stellars, scene, "Alpha Centauri's");
}

#[test]
fn an_arrival_shows_the_new_systems_stellars_at_once() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    // Coast a tick, so the arrival falls between the radar's refreshes and
    // the test shows the arrival's own refresh.
    view.tick(TICK);
    jump_now(&mut view);
    let leaving = view.session().expect("flying").system();
    for _ in 0..1000 {
        let cadence = view.hud_cadence;
        view.tick(TICK);
        if view.session().expect("flying").system() != leaving {
            let now = u64::try_from(crate::time::ticks(view.elapsed)).expect("in range");
            let mut probe = cadence;
            assert!(!probe.advance(now).radar, "not a refresh tick");
            assert_eq!(view.hud_cadence.blink_on(), cadence.blink_on());
            break;
        }
    }
    assert_ne!(view.session().expect("flying").system(), leaving);
    let stellars: Vec<Point> = view
        .radar_shown()
        .stellars
        .iter()
        .map(|s| s.position)
        .collect();
    assert_eq!(stellars, [at(0.0, 0.0)], "Alpha Centauri's");
    assert_eq!(view.radar_shown().player, view.shown_position());
}

#[test]
fn the_targets_blip_blinks_bright_every_eight_ticks_under_both_readings() {
    for every in [false, true] {
        let (_, chance) = scripted(&placed(850, 650, 90));
        let view = FlightView::new(trafficked(&[130], 1, 129, 3))
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        let mut view = if every { every_frame(view) } else { view };
        view.tick(TICK);
        tap(&mut view, TARGET_KEY);
        assert!(view.session().expect("flying").target().is_some());
        ticks(&mut view, 7);
        let mut seen = Vec::new();
        for _ in 0..24 {
            ticks(&mut view, 1);
            let [RadarMark::Pixel { color, .. }] = blips(&view)[..] else {
                panic!("one blip: {:?}", blips(&view));
            };
            seen.push(color);
        }
        let mut expected = vec![RadarColor::Dim; 8];
        expected.extend([RadarColor::Bright; 8]);
        expected.extend([RadarColor::Dim; 8]);
        assert_eq!(seen, expected, "every frame: {every}");
        let list = drawn(&view);
        let bright = list.iter().filter(|command| {
            matches!(command, DrawCommand::Dot { color, .. } if *color == layout().bright_radar)
        });
        assert_eq!(bright.count(), 1, "the player alone, the target dim");
    }
}

#[test]
fn the_bars_show_the_sessions_reserves() {
    let view = flight();
    let reserves = reserves(&view);
    assert_eq!(reserves, ShipStats::new(FIELDS, &[]).full());
    let mut expected = DrawList::new();
    hud::draw(
        &mut expected,
        view.status_bar().expect("a status bar"),
        &HudState {
            radar: &RadarScene::default(),
            blink_on: false,
            reserves,
            nav: NavDisplay::Off,
            cargo: None,
            disabled: false,
        },
        None,
    );
    assert_eq!(lines(&drawn(&view)), lines(&expected));
}

#[test]
fn an_unreadable_status_bar_says_why_and_flight_goes_on() {
    let barless = FakeCatalog {
        bar: Err("no ïntf 128".to_owned()),
        ..catalog()
    };
    let mut view = FlightView::new(barless);
    assert_eq!(view.status_bar(), Err("no ïntf 128"));
    view.input(&key(Key::Up, true));
    ticks(&mut view, 5);
    assert_eq!(player(&view), stepped(start(), THRUST, 5));
    let list = drawn(&view);
    let mut reason = DrawList::new();
    hud::draw_unavailable(&mut reason, "no ïntf 128");
    assert_eq!(list.iter().last(), reason.iter().next());
    assert_eq!(lines(&list), [], "no bars");
    assert_eq!(dots_on_radar(&list), [], "no radar");
    assert_eq!(sprites(&list).len(), 3, "stellars and ship");
}

#[test]
fn a_ship_type_whose_government_has_its_own_interface_shows_that_bar() {
    let view = FlightView::new(governed(catalog(), 128, 129));
    assert_eq!(
        view.status_bar(),
        Ok(&StatusBar {
            layout: bar_129(),
            background: Some(hud::Background {
                id: 705,
                width: 194.0,
                height: 767.0,
            }),
        })
    );
    let list = drawn(&view);
    assert!(
        list.iter().any(|c| *c
            == DrawCommand::Picture {
                image: ImageKey::picture(705),
                top_left: at(830.0, 0.0),
            }),
        "{list:?}"
    );
    for interface in [127, 0, -1] {
        let view = FlightView::new(governed(catalog(), 128, interface));
        assert_eq!(
            view.status_bar().map(|bar| &bar.layout),
            Ok(&layout()),
            "{interface}"
        );
    }
}

#[test]
fn a_session_that_cannot_start_has_no_status_bar() {
    let failed = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    let view = FlightView::new(failed);
    assert_eq!(view.status_bar(), Err("no chär to start from"));
}

// Frame rate.

/// Flies a script for three seconds of frames at `fps`, each redraw at
/// `n / fps` seconds to the nanosecond: thrust for a second, turn
/// right for half a second, reverse for a second, then coast.
fn fly(fps: u64) -> ShipState {
    let mut view = flight();
    let script = [
        (0, Key::Up, true),
        (2, Key::Up, false),
        (2, Key::Right, true),
        (3, Key::Right, false),
        (3, Key::Down, true),
        (5, Key::Down, false),
    ];
    let mut last = 0;
    for frame in 1..=3 * fps {
        let half_second = (frame - 1) * 2 / fps;
        if (frame - 1) * 2 % fps == 0 {
            for &(_, k, pressed) in script.iter().filter(|(at, ..)| *at == half_second) {
                view.input(&key(k, pressed));
            }
        }
        let now = frame * 1_000_000_000 / fps;
        view.tick(Duration::from_nanos(now - last));
        last = now;
    }
    player(&view)
}

#[test]
fn the_same_keys_fly_the_same_at_30_60_and_120_fps() {
    let at_30 = fly(30);
    assert!(at_30.heading > 0.0, "{at_30:?}");
    assert!(at_30.position.y < 0.0, "{at_30:?}");
    assert_eq!(fly(60), at_30);
    assert_eq!(fly(120), at_30);
}
