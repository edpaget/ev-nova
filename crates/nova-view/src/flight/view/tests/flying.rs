use super::faked_flight::{at_rest, hold, last_controls, ship_at, tick_to, ticked};
use super::faked_hud::{barred, barred_by, flown};
use super::*;
use nova_sim::Face;
use nova_sim::flight_frame::FrameRadarStellar;
use nova_sim::status_cargo::StatusCargo;

// Building.

// Flying.

#[test]
fn holding_up_thrusts_the_ship_up_and_the_camera_follows() {
    // How far a thrust carries the ship is nova-sim's
    // (`thrust_accelerates_along_the_heading_and_then_moves`).
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, true);
    tick_to(&mut view, Some(ship_at(Vec2::new(0.0, -23.4), 0.0)));
    assert!(last_controls(&view).thrust, "Up held");
    tick_to(&mut view, Some(ship_at(Vec2::new(0.0, -26.1), 0.0)));
    view.alpha = 0.5;
    assert_eq!(view.camera().center(), view.shown_position());
    let shown = view.shown_position();
    assert_eq!(shown.x, 0.0);
    // Drawn at most a step behind.
    assert!((-26.1..=-23.4).contains(&shown.y), "{shown:?}");
}

#[test]
fn releasing_up_coasts() {
    // That a coast keeps its velocity is nova-sim's (`a_ship_coasts_without_drag`).
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, true);
    ticked(&mut view, 1);
    hold(&mut view, Key::Up, false);
    ticked(&mut view, 1);
    let thrusts: Vec<bool> = view
        .sim
        .inputs()
        .iter()
        .map(|input| input.controls.thrust)
        .collect();
    assert_eq!(thrusts, [true, false]);
}

#[test]
fn left_and_right_turn_the_ship_and_its_frame() {
    // The turn rate is nova-sim's
    // (`right_turns_clockwise_and_left_anticlockwise_at_the_turn_rate`,
    // `turning_alone_does_not_move_the_ship`).
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Right, true);
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 30.0)));
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 33.0)));
    assert_eq!(last_controls(&view).turn, Turn::Right);
    // Drawn a step behind.
    assert_eq!(view.shown_heading(), 30.0);
    assert_eq!(view.frame(), Some(3));
    hold(&mut view, Key::Right, false);
    hold(&mut view, Key::Left, true);
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 330.0)));
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 327.0)));
    assert_eq!(last_controls(&view).turn, Turn::Left);
    assert_eq!(view.frame(), Some(33));
}

#[test]
fn left_and_right_together_cancel() {
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Left, true);
    hold(&mut view, Key::Right, true);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).turn, Turn::None);
}

#[test]
fn z_is_the_afterburner_key_held() {
    assert_eq!(AFTERBURNER_KEY, Key::Char('z'));
    let mut view = at_rest(catalog());
    assert!(!view.controls().afterburner);
    hold(&mut view, AFTERBURNER_KEY, true);
    assert!(view.controls().afterburner);
    hold(&mut view, AFTERBURNER_KEY, false);
    assert!(!view.controls().afterburner);
}

#[test]
fn up_and_z_held_with_an_afterburner_pass_the_top_speed_and_burn_fuel() {
    // How far past the top speed it goes, what it burns and how it bleeds
    // back are nova-sim's
    // (`thrust_and_afterburner_reach_1_8_times_top_speed_on_an_axis_a_tick_behind_the_box`,
    // `the_fuel_drains_to_none_and_the_afterburner_stops_the_tick_after`,
    // `released_the_box_bleeds_at_accel_x_0_4_a_tick_holding_the_velocity_to_it`).
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, true);
    hold(&mut view, AFTERBURNER_KEY, true);
    ticked(&mut view, 1);
    let controls = last_controls(&view);
    assert!(controls.thrust && controls.afterburner, "{controls:?}");
    hold(&mut view, AFTERBURNER_KEY, false);
    ticked(&mut view, 1);
    let controls = last_controls(&view);
    assert!(controls.thrust && !controls.afterburner, "{controls:?}");
}

#[test]
fn a_turns_the_ship_toward_its_target_and_alt_a_toward_the_nav_stellar() {
    // How the face key turns the ship is nova-sim's
    // (`the_auto_turn_turns_while_more_than_a_turn_off_and_stops_without_snapping`,
    // `while_the_auto_turn_aims_the_turn_keys_do_nothing`).
    assert_eq!(FACE_TARGET_KEY, Key::Char('a'));
    let mut view = at_rest(catalog());
    hold(&mut view, FACE_TARGET_KEY, true);
    hold(&mut view, Key::Left, true);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).face, Face::Target);
    hold(&mut view, Key::Left, false);
    hold(&mut view, Key::Alt, true);
    ticked(&mut view, 1);
    assert_eq!(last_controls(&view).face, Face::Stellar, "Alt-A");
    // Let go of, the turn keys turn the ship again.
    hold(&mut view, FACE_TARGET_KEY, false);
    hold(&mut view, Key::Alt, false);
    hold(&mut view, Key::Right, true);
    ticked(&mut view, 1);
    let controls = last_controls(&view);
    assert_eq!((controls.face, controls.turn), (Face::Off, Turn::Right));
}

#[test]
fn down_turns_the_ship_against_its_motion() {
    // Where reverse aims, a turn short of 180 and holding Left off, is
    // nova-sim's (`reverse_aims_against_the_velocity_from_0_05_on_either_axis`,
    // `the_auto_turn_turns_while_more_than_a_turn_off_and_stops_without_snapping`).
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Down, true);
    hold(&mut view, Key::Left, true);
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 177.0)));
    let controls = last_controls(&view);
    assert!(controls.reverse, "{controls:?}");
    assert_eq!(controls.turn, Turn::Left, "sent; the session holds it off");
    ticked(&mut view, 1);
    assert_eq!(view.frame(), Some(17));
}

#[test]
fn a_release_without_a_press_does_nothing() {
    let mut view = at_rest(catalog());
    hold(&mut view, Key::Up, false);
    ticked(&mut view, 1);
    assert!(view.held.is_empty());
    assert_eq!(last_controls(&view), Controls::default());
}

// Interpolation.

#[test]
fn the_ship_is_drawn_alpha_of_the_way_from_the_last_step_to_this_one() {
    let mut view = at_rest(catalog());
    let first = Vec2::new(0.0, -0.06);
    tick_to(&mut view, Some(ship_at(first, 0.0)));
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(0.0, 0.0), "a step behind");
    view.alpha = 0.5;
    let half = view.shown_position();
    assert!((half.y - first.y / 2.0).abs() < 1e-6, "{half:?}");
    assert_eq!(half.x, 0.0);
    assert_eq!(view.camera().center(), half);
    // The next step, and back to its start.
    tick_to(&mut view, Some(ship_at(Vec2::new(0.0, -0.18), 0.0)));
    assert_eq!(view.shown_position(), at(first.x, first.y));
}

#[test]
fn mid_flight_the_ship_is_drawn_between_its_last_two_positions() {
    let mut view = at_rest(catalog());
    let before = Vec2::new(12.0, -9.0);
    let after = Vec2::new(13.5, -10.0);
    tick_to(&mut view, Some(ship_at(before, 45.0)));
    tick_to(&mut view, Some(ship_at(after, 45.0)));
    view.alpha = 0.25;
    let shown = view.shown_position();
    let expected = |from: f32, to: f32| from + (to - from) * view.alpha();
    assert!((shown.x - expected(before.x, after.x)).abs() < 1e-4);
    assert!((shown.y - expected(before.y, after.y)).abs() < 1e-4);
}

#[test]
fn the_heading_is_drawn_the_short_way_round_across_0() {
    let mut view = at_rest(catalog());
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 357.0)));
    tick_to(&mut view, Some(ship_at(Vec2::ZERO, 0.0)));
    view.alpha = 0.5;
    // From 357 to 0: half way is 358.5, not 178.5.
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
            help: false,
            cheats: false,
        }
    );
    let view = barred(catalog(), among(Vec::new()));
    assert_eq!(view.overlays(), FlightOverlays::default());
    let view = view.with_overlays(TITLED);
    assert_eq!(view.overlays(), TITLED);
    let texts = texts(&flown(&barred(catalog(), among(Vec::new()))));
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
        cheats: false,
    };
    let list = flown(&barred(catalog(), among(Vec::new())).with_overlays(titled));
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
        cheats: false,
    };
    let list = flown(&barred(catalog(), among(Vec::new())).with_overlays(helped));
    let before = before_the_hud(&list);
    assert_eq!(
        before.last(),
        Some(&overlay(HELP, HELP_AT, OVERLAY_SIZE, Color::DIM)),
        "over the flight, under the HUD"
    );
    assert!(!texts(&list).contains(&"Sol (sÿst 130)".to_owned()));
    let both = flown(&barred(catalog(), among(Vec::new())).with_overlays(TITLED));
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
    let mut frame = in_sol_with(vec![ship_at(Vec2::ZERO, 0.0)]);
    frame.hud.reserves = ShipStats::new(FIELDS, &[]).full();
    let mut view = barred(starry(), frame);
    view.reset_stars();
    let list = flown(&view);
    let mut expected = DrawList::new();
    // The stars, the stellars and the ship are the world, the HUD over it.
    expected.world_in(view.play_rect(), |expected| {
        for star in faked_stars(&view) {
            expected.push(star);
        }
        scene::draw_stellars(expected, view.scene().expect("a scene"), &view.camera());
        expected.sprite(ImageKey::sprite(2000, 0), PLAYER_AT, Color::WHITE);
    });
    let bar = view.status_bar.as_ref().expect("a status bar");
    hud::draw(
        &mut expected,
        bar,
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
            cargo: Some(&StatusCargo::default()),
            disabled: false,
        },
        None,
        VIEW_SIZE.0,
    );
    let origin = hud::bar_origin(bar, VIEW_SIZE.0);
    target::draw_target_panel(&mut expected, &bar.layout, origin, None, None);
    target::draw_secondary(&mut expected, &bar.layout, origin, None, None);
    assert_eq!(list, expected);
    assert_eq!(list.iter().take_while(|c| is_star(c)).count(), 20);
    assert_eq!(
        sprites(&list),
        [
            (ImageKey::sprite(1128, 0), at(415.0, -216.0)),
            (ImageKey::sprite(1129, 0), at(715.0, 184.0)),
            (ImageKey::sprite(2000, 0), PLAYER_AT),
        ]
    );
    assert_eq!(
        HELP,
        "Arrows: fly   A: face target   Space: fire   Ctrl: secondary   W: weapon   Tab: next target   R: nearest   B: board   Y: hail   Cmd-K: jettison   F/D/V/C: escorts   Alt-C: dock   E: escort menu   L: land   M: map   J: jump   \\: next system   P: player info   I: mission info   Cmd-,: preferences   Esc: leave"
    );
    assert_eq!((TITLE, HELP_AT), (at(16.0, 32.0), at(16.0, 744.0)));
}

#[test]
fn the_ship_stays_at_the_play_areas_centre_as_the_stellars_go_by() {
    let mut view = barred(starry(), in_sol_with(vec![ship_at(Vec2::ZERO, 0.0)]));
    view.reset_stars();
    tick_to(&mut view, Some(ship_at(Vec2::new(40.0, -90.0), 125.0)));
    tick_to(&mut view, None);
    let list = flown(&view);
    let shown = sprites(&list);
    let camera = view.camera();
    assert_eq!(
        shown[2],
        (
            ImageKey::sprite(2000, view.frame().expect("a sheet")),
            PLAYER_AT
        )
    );
    assert_ne!(view.frame(), Some(0));
    assert_eq!(shown[0].1, camera.world_to_screen(at(0.0, -600.0)));
    assert_ne!(shown[0].1, at(415.0, -216.0), "the stellars moved");
    let stars = faked_stars(&view);
    assert_eq!(stars.len(), 20);
    assert!(list.iter().take(stars.len()).eq(stars.iter()), "the stars");
}

#[test]
fn the_stellars_animate_with_time() {
    let mut view = at_rest(catalog());
    ticked(&mut view, 1);
    assert_eq!(sprites(&flown(&view))[1].0, ImageKey::sprite(1129, 1));
    ticked(&mut view, 2);
    assert_eq!(sprites(&flown(&view))[1].0, ImageKey::sprite(1129, 3));
}

#[test]
fn a_ship_without_a_sheet_is_a_placeholder_with_the_reason() {
    let sheetless = FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..catalog()
    };
    let view = barred(sheetless, among(Vec::new()));
    assert_eq!(view.frame(), None);
    let list = flown(&view);
    assert_eq!(sprites(&list).len(), 2, "the stellars only");
    let mut expected = DrawList::new();
    crossed_box(&mut expected, PLAYER_AT, PLACEHOLDER_SIZE, PLACEHOLDER);
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
                PLAYER_AT.x - PLACEHOLDER_SIZE / 2.0,
                PLAYER_AT.y + PLACEHOLDER_SIZE / 2.0 + MESSAGE_GAP
            ),
            OVERLAY_SIZE,
            Color::ERROR
        ))
    );
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
    // That the glow goes out as the ship coasts is nova-sim's
    // (`the_engine_glow_ramps_up_with_thrust_and_down_after_it`).
    let view = at_rest(layered());
    let view = lit(view, Some(32), 0);
    assert_eq!(
        super::faked_world::faked_ship_sprites(&view),
        [
            (ImageKey::sprite(2000, 0), PLAYER_AT),
            (ImageKey::sprite(2200, 0), PLAYER_AT),
        ]
    );
}

#[test]
fn a_thrusting_ship_draws_its_glow_between_it_and_its_lights() {
    // A base of 6: drawn at every roll.
    let view = lit(at_rest(layered()), Some(32), 6);
    let centre = view.camera().world_to_screen(view.shown_position());
    assert_eq!(
        super::faked_world::faked_ship_sprites(&view),
        [
            (ImageKey::sprite(2000, 0), centre),
            (ImageKey::sprite(2100, 0), centre),
            (ImageKey::sprite(2200, 0), centre),
        ]
    );
}

#[test]
fn a_thrusting_ships_glow_and_lights_or_onto_its_normal_sprite() {
    let view = lit(at_rest(layered()), Some(32), 6);
    assert_eq!(
        faked_ship_blends(&view),
        [(2000, Blend::Normal), (2100, Blend::Or), (2200, Blend::Or),]
    );
    assert!(
        !super::faked_world::world_list(&view)
            .iter()
            .any(|command| matches!(
                command,
                DrawCommand::Sprite {
                    blend: Blend::Additive,
                    ..
                }
            )),
        "nothing in a thrusting, lit frame is added"
    );
    let view = lit(view, Some(32), 0);
    assert_eq!(
        faked_ship_blends(&view),
        [(2000, Blend::Normal), (2200, Blend::Or)],
        "coasting"
    );
}

#[test]
fn the_layers_turn_with_the_ship_between_steps() {
    let mut view = lit(at_rest(layered()), Some(32), 6);
    let mut turned = |heading| {
        let mut ship = view.frame.player();
        ship.heading = heading;
        tick_to(&mut view, Some(ship));
    };
    turned(147.0);
    turned(150.0);
    view.alpha = 0.5;
    assert_ne!(view.shown_heading(), view.frame.player().heading, "between");
    let frame = view.frame().expect("a sheet");
    assert_eq!(frame, 14, "148.5, truncated: {}", view.shown_heading());
    let centre = view.camera().world_to_screen(view.shown_position());
    assert_eq!(
        super::faked_world::faked_ship_sprites(&view),
        [
            (ImageKey::sprite(2000, frame), centre),
            (ImageKey::sprite(2100, frame), centre),
            (ImageKey::sprite(2200, frame % 12), centre),
        ]
    );
}

#[test]
fn a_ship_without_layers_draws_only_its_sprite() {
    let view = lit(at_rest(catalog()), Some(32), 6);
    let ids: Vec<_> = super::faked_world::faked_ship_sprites(&view)
        .iter()
        .map(|(i, _)| i.id)
        .collect();
    assert_eq!(ids, [2000]);
}

#[test]
fn a_ship_without_a_sheet_draws_no_layers() {
    let view = at_rest(FakeCatalog {
        sheet: Err("no shän 128 for shïp 128".to_owned()),
        ..layered()
    });
    let view = lit(view, Some(32), 6);
    assert_eq!(super::faked_world::faked_ship_sprites(&view), []);
}

#[test]
fn the_glow_ramps_in_and_flickers_at_the_glow_function_level() {
    // The base's ramp is nova-sim's
    // (`the_engine_glow_ramps_up_with_thrust_and_down_after_it`); the view
    // draws its level at each tick.
    let mut view = at_rest(layered());
    let mut cruising = Vec::new();
    for n in 1..=40_u8 {
        let base = n.min(GLOW_CRUISE);
        let mut ship = view.frame.player();
        ship.glow = base;
        tick_to(&mut view, Some(ship));
        let want = faked_glow_for(&view, base, u64::from(n));
        assert_eq!(faked_glow_commands(&view), want, "tick {n}");
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
    let mut view = at_rest(layered());
    ticked(&mut view, 30);
    for k in 1..=40_u8 {
        let base = GLOW_CRUISE.saturating_sub(k);
        let mut ship = view.frame.player();
        ship.glow = base;
        tick_to(&mut view, Some(ship));
        let want = faked_glow_for(&view, base, 30 + u64::from(k));
        assert_eq!(faked_glow_commands(&view), want, "{k} ticks after");
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

#[test]
fn the_shuttles_lights_blink_with_game_time() {
    // When the Shuttle's lights are lit is nova-sim's
    // (`the_shuttle_double_flashes_every_40_ticks`); its record says.
    let view = lit(at_rest(blinking(SHUTTLE)), None, 0);
    assert_eq!(faked_lights_alphas(&view), [0_u8; 0], "off");
    assert_eq!(
        super::faked_world::faked_ship_sprites(&view)[0].0.id,
        2000,
        "the ship is drawn"
    );
    let view = lit(view, Some(32), 0);
    assert_eq!(faked_lights_tints(&view), [Color::WHITE], "on");
}

#[test]
fn a_pulsing_light_is_ored_at_its_level() {
    // The stock triangle's levels, 10 to 31, are nova-sim's
    // (`the_stock_triangle_pulses_between_10_and_31_every_58_ticks`).
    let view = lit(at_rest(blinking(SHUTTLE)), None, 0);
    assert_eq!(faked_lights_alphas(&view), [0_u8; 0], "level 0: not drawn");
    let view = lit(view, Some(1), 0);
    assert_eq!(faked_lights_alphas(&view), [8], "level 1");
    let view = lit(view, Some(31), 0);
    assert_eq!(faked_lights_alphas(&view), [247], "level 31");
}

/// The lights drawn by OR with `tint` over the coasting ship's centre.
fn lights_at(tint: Color) -> DrawCommand {
    DrawCommand::Sprite {
        image: ImageKey::sprite(2200, 0),
        center: PLAYER_AT,
        tint,
        blend: Blend::Or,
    }
}

#[test]
fn lights_are_ored_scaled_by_level_below_full_and_whole_at_full() {
    // The original draws every level below full with its translucent
    // blit, which ORs in the lights scaled by level/32 (see
    // `lights_tint`); full is ORed whole.
    let mut view = lit(at_rest(blinking(SHUTTLE)), None, 0);
    for level in 1..=32 {
        view = lit(view, Some(level), 0);
        assert_eq!(
            faked_lights_commands(&view),
            [lights_at(lights_tint(level))],
            "level {level}"
        );
    }
    assert_eq!(lights_tint(32), Color::WHITE);
}

#[test]
fn steady_lights_are_ored_at_full_from_the_first_frame() {
    // That a steady light is lit full before the first frame is nova-sim's
    // (`modes_other_than_one_to_three_hold_the_lights_steady_at_full`,
    // `prepare_all_sprites_steps_the_players_fade_and_every_npcs_once`).
    let view = lit(at_rest(blinking(Blink::STEADY)), Some(32), 0);
    assert_eq!(faked_lights_tints(&view), [Color::WHITE]);
}

#[test]
fn the_glow_still_shows_while_the_lights_are_off() {
    let mut view = at_rest(blinking(SHUTTLE));
    ticked(&mut view, 12);
    let mut ship = view.frame.player();
    ship.glow = 13;
    ship.drawn.lights = None;
    tick_to(&mut view, Some(ship));
    assert_eq!(
        faked_ship_blends(&view),
        [(2000, Blend::Normal), (2100, Blend::Or)]
    );
    let glow = super::faked_world::world_list(&view)
        .iter()
        .find_map(|command| match command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 2100 => Some(*tint),
            _ => None,
        });
    let level = glow_level(13, 13, &view.glow_rolls).expect("drawn");
    assert_eq!(glow, Some(lights_tint(level)));
}

// The HUD.

#[test]
fn the_status_bar_alone_is_flights_bar_and_nothing_else() {
    let mut frame = among(Vec::new());
    frame.landed = Some(StellarId(128));
    let view = barred(catalog(), frame).with_overlays(TITLED);
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
    let flown = flown(&view);
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
    let view = barred(barless, among(Vec::new()));
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    assert_eq!(bar.len(), 0, "nothing, not even why");
}

#[test]
fn the_hud_shows_the_status_bar_full_bars_and_the_stellars_on_radar() {
    let mut frame = among(Vec::new());
    frame.hud.reserves = ShipStats::new(FIELDS, &[]).full();
    let view = barred(catalog(), frame);
    let list = flown(&view);
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
fn a_stellars_radar_shape_and_type_width_reach_the_radar() {
    // The shape and the `Type` width are the session's
    // (`session::frame::radar`): none of these three is in the scene.
    let listed = |id, (x, y), square, width| FrameRadarStellar {
        square,
        type_width: Some(width),
        ..record::radar_stellar(StellarId(id), Vec2::new(x, y))
    };
    let mut frame = among(Vec::new());
    frame.radar_stellars.extend([
        listed(150, (640.0, 0.0), true, 300),
        listed(151, (-640.0, 320.0), false, 120),
        listed(152, (0.0, 640.0), false, 250),
    ]);
    let view = barred(catalog(), frame);
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

#[test]
fn by_the_engine_the_radar_holds_between_refreshes_eight_ticks_apart() {
    let mut view = barred(catalog(), among(Vec::new()));
    let mut y = 0.0;
    let mut fly = |view: &mut Faked| {
        y -= 1.0;
        tick_to(view, Some(ship_at(Vec2::new(0.0, y), 0.0)));
    };
    fly(&mut view);
    let refreshed = view.shown_position();
    assert_eq!(
        faked_radar_player(&view),
        refreshed,
        "refreshed on the first"
    );
    for tick in 2..=8 {
        fly(&mut view);
        assert_eq!(faked_radar_player(&view), refreshed, "held on tick {tick}");
    }
    assert_ne!(view.shown_position(), refreshed, "the ship moved");
    fly(&mut view);
    assert_eq!(faked_radar_player(&view), view.shown_position(), "the 9th");
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
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible);
    let mut view = barred_by(catalog(), among(Vec::new()), &rules);
    let before = dots_on_radar(&flown(&view));
    for n in 1..=45 {
        tick_to(
            &mut view,
            Some(ship_at(Vec2::new(0.0, -2.0 * n as f32), 0.0)),
        );
        assert_eq!(faked_radar_player(&view), view.shown_position());
    }
    view.alpha = 0.5;
    let shown = view.shown_position();
    assert!(shown.y < -10.0, "{shown:?}");
    assert_eq!(faked_radar_player(&view), shown, "between ticks too");
    let after = dots_on_radar(&flown(&view));
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
fn the_targets_blip_blinks_bright_every_eight_ticks_under_both_readings() {
    // Which ship Tab picks is nova-sim's
    // (`a_target_key_answers_the_target_it_picks`).
    for every in [false, true] {
        let rules = if every {
            Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible)
        } else {
            Rulebook::default()
        };
        let npc = npc_129(0, Vec2::new(100.0, -100.0), 90.0);
        let mut view = barred_by(trafficked(&[130], 1, 129, 3), among(vec![npc]), &rules);
        ticked(&mut view, 1);
        super::faked_hud::changes(&mut view, |frame| frame.target = Some(record::target(0)));
        ticked(&mut view, 7);
        let mut seen = Vec::new();
        for _ in 0..24 {
            ticked(&mut view, 1);
            let [RadarMark::Pixel { color, .. }] = blips(&view)[..] else {
                panic!("one blip: {:?}", blips(&view));
            };
            seen.push(color);
        }
        let mut expected = vec![RadarColor::Dim; 8];
        expected.extend([RadarColor::Bright; 8]);
        expected.extend([RadarColor::Dim; 8]);
        assert_eq!(seen, expected, "every frame: {every}");
        let list = flown(&view);
        let bright = list.iter().filter(|command| {
            matches!(command, DrawCommand::Dot { color, .. } if *color == layout().bright_radar)
        });
        assert_eq!(bright.count(), 1, "the player alone, the target dim");
    }
}

#[test]
fn the_bars_show_the_sessions_reserves() {
    // A new pilot's full reserves are nova-sim's
    // (`a_fill_writes_the_ships_reserves`).
    let reserves = ShipStats::new(FIELDS, &[]).full();
    let mut frame = among(Vec::new());
    frame.hud.reserves = reserves;
    let view = barred(catalog(), frame);
    let mut expected = DrawList::new();
    hud::draw(
        &mut expected,
        view.status_bar.as_ref().expect("a status bar"),
        &HudState {
            radar: &RadarScene::default(),
            blink_on: false,
            reserves,
            nav: NavDisplay::Off,
            cargo: None,
            disabled: false,
        },
        None,
        VIEW_SIZE.0,
    );
    assert_eq!(lines(&flown(&view)), lines(&expected));
}

#[test]
fn an_unreadable_status_bar_says_why_and_flight_goes_on() {
    let mut view = at_rest(catalog());
    view.status_bar = Err("no ïntf 128".to_owned());
    hold(&mut view, Key::Up, true);
    ticked(&mut view, 5);
    assert!(last_controls(&view).thrust, "flight goes on");
    let list = flown(&view);
    let mut reason = DrawList::new();
    hud::draw_unavailable(&mut reason, "no ïntf 128", VIEW_SIZE.0);
    assert_eq!(list.iter().last(), reason.iter().next());
    assert_eq!(lines(&list), [], "no bars");
    assert_eq!(dots_on_radar(&list), [], "no radar");
    assert_eq!(sprites(&list).len(), 3, "stellars and ship");
}

// Frame rate.

/// `view` with its player's record lit as `lights` says and glowing at
/// `glow`, shown from now on.
fn lit(mut view: Faked, lights: Option<u8>, glow: u8) -> Faked {
    super::faked_hud::changes(&mut view, |frame| {
        frame.ships[0].drawn.lights = lights;
        frame.ships[0].glow = glow;
    });
    view
}

/// The ship's sprites' image ids and blends `view`'s world draws.
fn faked_ship_blends(view: &Faked) -> Vec<(i16, Blend)> {
    super::faked_world::world_list(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, blend, .. } if (2000..2300).contains(&image.id) => {
                Some((image.id, *blend))
            }
            _ => None,
        })
        .collect()
}

/// Every command `view`'s world draws the glow, `rlëD` 2100, with.
fn faked_glow_commands(view: &Faked) -> Vec<DrawCommand> {
    super::faked_world::world_list(view)
        .iter()
        .filter(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2100))
        .cloned()
        .collect()
}

/// The glow commands for a base of `base` at `tick`: the glow drawn by
/// OR at the level `glow_level` gives over the ship's centre, on its
/// frame, or none when it is hidden.
fn faked_glow_for(view: &Faked, base: u8, tick: u64) -> Vec<DrawCommand> {
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

/// Every command `view`'s world draws the lights, `rlëD` 2200, with.
fn faked_lights_commands(view: &Faked) -> Vec<DrawCommand> {
    super::faked_world::world_list(view)
        .iter()
        .filter(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2200))
        .cloned()
        .collect()
}

/// The tints `view` ORs the lights with.
fn faked_lights_tints(view: &Faked) -> Vec<Color> {
    faked_lights_commands(view)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite {
                tint,
                blend: Blend::Or,
                ..
            } => Some(tint),
            _ => None,
        })
        .collect()
}

/// The lights' alphas `view` draws: one, or none when they are off.
fn faked_lights_alphas(view: &Faked) -> Vec<u8> {
    faked_lights_tints(view).iter().map(|tint| tint.a).collect()
}

/// The radar's player position on `view`.
fn faked_radar_player(view: &Faked) -> Point {
    view.radar_shown(&view.frame).player
}

/// The stars `view` draws, in order.
fn faked_stars(view: &Faked) -> Vec<DrawCommand> {
    super::faked_world::world_list(view)
        .iter()
        .filter(|command| is_star(command))
        .cloned()
        .collect()
}
