use super::*;

// The About dialog.

#[test]
fn i_opens_the_about_text_over_the_side_shown() {
    let data = data();
    let mut screen = with_dialogs(Rc::clone(&data));
    assert!(screen.about().is_none());
    let below = drawn(&screen);
    open_about(&mut screen);
    assert_eq!(about(&screen).lines()[..2], ["line 0", "line 1"]);
    assert_eq!(about(&screen).lines().len(), 10);
    // The side, the hint, then the dialog.
    let list = drawn(&screen);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let below: Vec<&DrawCommand> = below.iter().collect();
    assert_eq!(commands[..below.len()], below[..]);
    assert_eq!(
        drawn(about(&screen)).iter().collect::<Vec<_>>(),
        commands[below.len()..]
    );

    // From the galaxy map too.
    let mut screen = with_dialogs(data);
    screen.input(&key(Key::Tab, true));
    open_about(&mut screen);
}

#[test]
fn the_about_text_reads_its_conditional_text_for_a_pilot_not_yet_loaded() {
    assert_eq!(
        start_up_words(
            "By {G \"him\" \"her\"}{b1 \", set\" \"\"}{P30 \".\" \"!\"} <PN>",
            &*data(),
            nova_sim::text_parse::ParseRules::default()
        ),
        "By her. ",
        "conditional text first, then the tags (no pilot's name)"
    );
}

#[test]
fn the_about_text_follows_the_apps_rulebook() {
    let about_only = |text: &str| {
        loaded(&ForkBuilder::new().resource(Desc::TYPE, ABOUT_TEXT, None, &desc_of(text)))
    };
    let mut screen = with_dialogs(about_only("Hi {x there"));
    open_about(&mut screen);
    assert_eq!(about(&screen).lines(), ["Hi "], "the engine's reading");
    let mut screen = with_dialogs(about_only("Hi {x there"))
        .with_rules(Rulebook::default().with_override(RuleKey::StrayTextBrace, RuleSource::Bible));
    open_about(&mut screen);
    assert_eq!(
        about(&screen).lines(),
        ["Hi {x there"],
        "the stray brace copied as written"
    );
}

/// Game data holding only `dësc` `id` reading `text` and shïp 128,
/// "First", which `<PST>` names before any pilot.
fn start_up_text_only(id: i16, text: &str) -> Rc<GameData> {
    loaded(
        &ForkBuilder::new()
            .resource(Desc::TYPE, id, None, &desc_of(text))
            .resource(
                Ship::TYPE,
                128,
                Some(b"First"),
                &vec![0; Ship::SIZE.expect("fixed")],
            ),
    )
}

/// The lines main menu X shows over `data`, under `rules`.
fn x_lines(data: Rc<GameData>, rules: Rulebook) -> Vec<String> {
    let store = MemoryPilots::new();
    let mut screen = with_dialogs(data)
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared()
        .with_rules(rules);
    screen.input(&key(Key::Char('x'), true));
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::Note);
    about(&screen).lines().to_vec()
}

#[test]
fn the_about_text_fills_its_tags_as_before_any_pilot() {
    let mut screen = with_dialogs(start_up_text_only(
        ABOUT_TEXT,
        "Registered to: <PN>, a <PST>, {G \"he\" \"she\"}.",
    ));
    open_about(&mut screen);
    assert_eq!(
        about(&screen).lines().join(" "),
        "Registered to: , a First, she.",
        "no pilot's name, ship 128's class, the gender's second reading"
    );
}

#[test]
fn the_x_text_resolves_its_conditional_text_for_a_pilot_not_yet_loaded() {
    assert_eq!(
        x_lines(
            start_up_text_only(MENU_X_TEXT, "Hi {G \"sir\" \"ma'am\"} <PST>"),
            Rulebook::default()
        ),
        ["Hi ma'am First"]
    );
}

#[test]
fn the_x_text_follows_the_apps_rulebook() {
    let x_only = || start_up_text_only(MENU_X_TEXT, "Hi {x there");
    assert_eq!(
        x_lines(x_only(), Rulebook::default()),
        ["Hi "],
        "the engine's reading"
    );
    assert_eq!(
        x_lines(
            x_only(),
            Rulebook::default().with_override(RuleKey::StrayTextBrace, RuleSource::Bible)
        ),
        ["Hi {x there"],
        "the stray brace copied as written"
    );
}

#[test]
fn the_about_dialog_uses_the_game_s_button_style() {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    let list = drawn(about(&screen));
    let done = list
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text {
                text, font, size, ..
            } if text == "Done" => Some((*font, *size)),
            _ => None,
        })
        .expect("the Done label");
    assert_eq!(done, (Font::Charcoal, 12.0), "stock, as there is no cölr");
}

#[test]
fn without_dialogs_i_does_nothing() {
    let mut screen = AppScreen::new(data());
    let before = drawn(&screen);
    assert_eq!(screen.input(&key(Key::Char('i'), true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(screen.about().is_none());
    assert_eq!(drawn(&screen), before);
}

/// The text of the last thing `screen` draws: the hint.
fn hint_text(screen: &AppScreen) -> String {
    match drawn(screen).iter().last() {
        Some(DrawCommand::Text { text, .. }) => text.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_hint_offers_i_only_with_dialogs() {
    let data = data();
    let without = hint_text(&AppScreen::new(Rc::clone(&data)));
    assert_eq!(without, "Tab: ships / galaxy map   F: fly");
    assert!(!without.contains("I:"), "{without}");
    assert_eq!(
        hint_text(&with_dialogs(data)),
        "Tab: ships / galaxy map   F: fly   I: about   P: preferences"
    );
}

#[test]
fn a_missing_dialog_or_text_opens_nothing() {
    let mut screen = AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
    screen.input(&key(Key::Char('i'), true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);

    let file = OneFile(ForkBuilder::new().build().bytes);
    let empty = Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"));
    let mut screen = with_dialogs(empty);
    screen.input(&key(Key::Char('i'), true));
    assert!(screen.about().is_none());
}

#[test]
fn an_i_repeat_or_release_opens_nothing() {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Char('i'), false));
    screen.input(&held(Key::Char('i')));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
}

#[test]
fn i_does_nothing_in_flight() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Char('i'), true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.about().is_none());
}

#[test]
fn while_open_input_reaches_only_the_dialog() {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    for input in [
        key(Key::Right, true),
        key(Key::Tab, true),
        key(Key::Char('f'), true),
        key(Key::Char('i'), true),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None);
    }
    assert_eq!(screen.showing(), Showing::About);
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
    assert!(screen.flight_view().is_none());
    screen.input(&key(Key::Down, true));
    screen.input(&held(Key::Down));
    assert_eq!(about(&screen).first_line(), 2);
}

#[test]
fn escape_closes_the_dialog_and_never_quits() {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    open_about(&mut screen);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    assert!(screen.about().is_none());
    assert_eq!(screen.input(&key(Key::Escape, false)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
}

#[test]
fn return_or_a_click_on_done_closes_the_dialog() {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);

    open_about(&mut screen);
    let done = about(&screen)
        .dialog()
        .item_bounds(1)
        .expect("Done")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        });
    }
    assert_eq!(screen.showing(), Showing::ShipBrowser);
}

#[test]
fn only_the_dialog_ticks_while_it_is_open() {
    let mut screen = with_dialogs(data());
    let tick = Duration::from_millis(100);
    screen.tick(tick);
    let frame = screen.ship_browser().frame();
    open_about(&mut screen);
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), frame, "paused below");
    screen.input(&key(Key::Escape, true));
    screen.tick(tick);
    assert_ne!(screen.ship_browser().frame(), frame);
}

#[test]
fn opening_cancels_a_drag_and_lets_go_of_the_keys_below() {
    let mut screen = with_dialogs(data());
    enter(&mut screen, 128);
    let camera_at = camera(&screen);
    screen.input(&key(Key::Char('d'), true));
    open_about(&mut screen);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::System);
    screen.tick(Duration::from_secs(1));
    assert_eq!(camera(&screen), camera_at, "D was let go");

    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    let view = *screen.galaxy_map().view().expect("the viewer");
    let at = Point::new(500.0, 400.0);
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    open_about(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&Input::PointerMoved(Point::new(600.0, 400.0)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "the drag was cancelled"
    );
}

#[test]
fn debug_shows_whether_there_are_dialogs() {
    let debug = format!("{:?}", with_dialogs(data()));
    assert!(debug.contains("dialogs: Some(Dialogs { .. })"), "{debug}");
    let debug = format!("{:?}", AppScreen::new(data()));
    assert!(debug.contains("dialogs: None"), "{debug}");
}

#[test]
fn cancelling_and_releasing_reach_the_dialog_while_open() {
    let mut screen = with_dialogs(data());
    open_about(&mut screen);
    let done = about(&screen)
        .dialog()
        .item_bounds(1)
        .expect("Done")
        .center();
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: done,
    };
    screen.input(&button(true));
    screen.release_keys();
    screen.cancel_pointer();
    screen.input(&button(false));
    assert_eq!(screen.showing(), Showing::About, "the click was abandoned");
}

// The Preferences dialog.

fn quiet() -> Prefs {
    Prefs {
        sound: nova_view::SoundPrefs {
            sound: true,
            music: false,
            effects_level: 3,
            music_level: 5,
        },
        hyperspace_effects: true,
        quicktime_movies: true,
        smoke_trails: true,
        weapon_effects: true,
        fullscreen: nova_view::FullscreenKind::Borderless,
        scaling: nova_view::Scaling::Native,
    }
}

#[test]
fn p_opens_the_preferences_over_ships_and_galaxy() {
    let mut screen = with_dialogs(data());
    assert!(screen.preferences().is_none());
    screen.input(&key(PREFS, true));
    assert_eq!(screen.showing(), Showing::Preferences);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);

    screen.input(&key(Key::Tab, true));
    screen.input(&key(PREFS, true));
    assert_eq!(screen.showing(), Showing::Preferences);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
}

#[test]
fn command_comma_opens_the_preferences_on_every_side() {
    let mut screen = with_dialogs(data());
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences, "the ship browser");
    screen.input(&key(Key::Escape, true));

    fly(&mut screen);
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences, "flight");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "Escape only closes it");

    screen.input(&key(MAP, true));
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences, "over the map");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::FlightMap);
    screen.input(&key(Key::Escape, true));

    land_twice(&mut screen);
    assert_eq!(screen.showing(), Showing::Spaceport);
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Preferences, "the spaceport");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Spaceport, "still landed");
}

#[test]
fn a_comma_without_command_or_after_it_is_let_go_opens_nothing() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PREFERENCES_KEY, true));
    assert_eq!(screen.showing(), Showing::Flight, "no Command");
    screen.input(&key(Key::Command, true));
    screen.input(&held(PREFERENCES_KEY));
    assert_eq!(screen.showing(), Showing::Flight, "a repeat");
    screen.input(&key(Key::Command, false));
    screen.input(&key(PREFERENCES_KEY, true));
    assert_eq!(screen.showing(), Showing::Flight, "Command let go");
    screen.input(&key(Key::Command, true));
    screen.release_keys();
    screen.input(&key(PREFERENCES_KEY, true));
    assert_eq!(screen.showing(), Showing::Flight, "the window lost focus");
}

#[test]
fn the_dialog_opens_on_the_prefs_the_router_was_given() {
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    assert_eq!(screen.prefs(), quiet());
    open_prefs(&mut screen);
    assert_eq!(prefs_dialog(&screen).prefs(), quiet());
    assert_eq!(AppScreen::new(data()).prefs(), Prefs::default());
}

#[test]
fn an_offer_of_display_modes_shows_in_the_open_dialog_alone() {
    let fhd = nova_view::VideoMode {
        width: 1920,
        height: 1080,
        refresh_millihertz: 60_000,
    };
    let exclusive = nova_view::FullscreenKind::Exclusive(fhd);
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    screen.offer_display(&[fhd], exclusive, exclusive);
    assert!(screen.preferences().is_none());
    assert_eq!(screen.prefs(), quiet(), "no dialog, no change");
    open_prefs(&mut screen);
    screen.offer_display(&[fhd], exclusive, exclusive);
    let picker = prefs_dialog(&screen).display();
    assert_eq!(
        picker.entries(),
        [nova_view::FullscreenKind::Borderless, exclusive]
    );
    assert_eq!(picker.selected(), exclusive);
    assert_eq!(
        screen.prefs(),
        Prefs {
            fullscreen: exclusive,
            ..quiet()
        },
        "the dialog's next opening starts there"
    );
    assert_eq!(screen.take_prefs(), None, "an offer is no change");
}

#[test]
fn the_scaling_offered_is_listed_in_every_preferences_dialog_on_the_one_running() {
    use nova_view::Scaling::{Native, SharpBilinear, SuperXbr};
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    open_prefs(&mut screen);
    assert_eq!(
        prefs_dialog(&screen).scaling().entries(),
        [Native, SuperXbr],
        "native and Super-xBR before an offer"
    );
    screen.input(&key(Key::Escape, true));
    screen.offer_scaling(&[Native, SharpBilinear, SuperXbr], SharpBilinear);
    assert_eq!(
        screen.prefs(),
        Prefs {
            scaling: SharpBilinear,
            ..quiet()
        }
    );
    assert_eq!(screen.take_prefs(), None, "an offer is no change");
    for _ in 0..2 {
        open_prefs(&mut screen);
        let list = prefs_dialog(&screen).scaling();
        assert_eq!(list.entries(), [Native, SharpBilinear, SuperXbr]);
        assert_eq!(list.selected(), SharpBilinear);
        screen.input(&key(Key::Escape, true));
    }
}

#[test]
fn a_scaling_pick_comes_out_once_as_a_change_and_the_dialog_reopens_on_it() {
    use nova_view::Scaling::{Native, SharpBilinear, SuperXbr};
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    screen.offer_scaling(&[Native, SharpBilinear, SuperXbr], Native);
    open_prefs(&mut screen);
    let down = prefs_dialog(&screen).scaling().rects().down.center();
    click_at(&mut screen, down);
    assert_eq!(screen.take_prefs(), None, "held while open");
    screen.input(&key(Key::Enter, true));
    assert_eq!(
        screen.take_prefs().map(|prefs| prefs.scaling),
        Some(SharpBilinear)
    );
    assert_eq!(screen.take_prefs(), None, "once");
    open_prefs(&mut screen);
    assert_eq!(prefs_dialog(&screen).scaling().selected(), SharpBilinear);
}

#[test]
fn a_pick_in_the_dialog_comes_out_once_after_it_closes() {
    let fhd = nova_view::VideoMode {
        width: 1920,
        height: 1080,
        refresh_millihertz: 60_000,
    };
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    assert_eq!(screen.take_display_choice(), None);
    open_prefs(&mut screen);
    let borderless = nova_view::FullscreenKind::Borderless;
    screen.offer_display(&[fhd], borderless, borderless);
    let down = prefs_dialog(&screen).display().rects().down.center();
    click_at(&mut screen, down);
    assert_eq!(screen.take_display_choice(), None, "held while open");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser, "closed");
    let picked = nova_view::FullscreenKind::Exclusive(fhd);
    assert_eq!(screen.take_display_choice(), Some(picked));
    assert_eq!(screen.take_display_choice(), None, "once");
    assert_eq!(screen.prefs().fullscreen, picked);
}

#[test]
fn each_change_comes_out_once_and_the_dialog_reopens_on_it() {
    let mut screen = with_dialogs(data()).with_prefs(quiet());
    assert_eq!(screen.take_prefs(), None);
    open_prefs(&mut screen);
    let music = prefs_dialog(&screen).music().rect().center();
    click_at(&mut screen, music);
    let changed = Prefs {
        sound: nova_view::SoundPrefs {
            music: true,
            ..quiet().sound
        },
        ..quiet()
    };
    assert_eq!(screen.take_prefs(), Some(changed));
    assert_eq!(screen.take_prefs(), None, "once");
    assert_eq!(screen.prefs(), changed);
    let up = prefs_dialog(&screen).effects_volume().rects().up.center();
    click_at(&mut screen, up);
    screen.input(&key(Key::Enter, true));
    assert!(screen.preferences().is_none(), "closed");
    let louder = Prefs {
        sound: nova_view::SoundPrefs {
            effects_level: 4,
            ..changed.sound
        },
        ..changed
    };
    assert_eq!(screen.take_prefs(), Some(louder));
    open_prefs(&mut screen);
    assert_eq!(prefs_dialog(&screen).prefs(), louder);
}

fn no_effects() -> Prefs {
    Prefs {
        hyperspace_effects: false,
        ..quiet()
    }
}

fn flight_effects(screen: &AppScreen) -> bool {
    screen.flight_view().expect("flying").hyperspace_effects()
}

#[test]
fn flight_is_built_with_the_routers_hyperspace_effects() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert!(flight_effects(&screen), "on by default");
    let mut screen = with_dialogs(data()).with_prefs(no_effects());
    fly(&mut screen);
    assert!(!flight_effects(&screen));
}

#[test]
fn a_change_in_the_dialog_reaches_flight_already_built() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).hyperspace_effects().rect().center();
    click_at(&mut screen, point);
    assert!(!flight_effects(&screen));
    assert_eq!(
        screen.take_prefs(),
        Some(Prefs {
            hyperspace_effects: false,
            ..Prefs::default()
        })
    );
    assert_eq!(screen.take_prefs(), None, "once");
    click_at(&mut screen, point);
    assert!(flight_effects(&screen), "back on");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(flight_effects(&screen));
}

fn flight_view(screen: &AppScreen) -> &FlightView<Rc<GameData>> {
    screen.flight_view().expect("flying")
}

#[test]
fn flight_is_built_with_the_routers_smoke_trails_and_weapon_effects() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert!(flight_view(&screen).smoke_trails(), "on by default");
    assert!(flight_view(&screen).weapon_effects(), "on by default");
    for (smoke_trails, weapon_effects) in [(false, true), (true, false)] {
        let mut screen = with_dialogs(data()).with_prefs(Prefs {
            smoke_trails,
            weapon_effects,
            ..quiet()
        });
        fly(&mut screen);
        assert_eq!(flight_view(&screen).smoke_trails(), smoke_trails);
        assert_eq!(flight_view(&screen).weapon_effects(), weapon_effects);
    }
}

#[test]
fn a_smoke_trails_change_in_the_dialog_reaches_flight_already_built() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).smoke_trails().rect().center();
    click_at(&mut screen, point);
    assert!(!flight_view(&screen).smoke_trails());
    assert_eq!(
        screen.take_prefs(),
        Some(Prefs {
            smoke_trails: false,
            ..Prefs::default()
        }),
        "reported, to be saved"
    );
    click_at(&mut screen, point);
    assert!(flight_view(&screen).smoke_trails(), "back on");
}

#[test]
fn by_the_engine_a_weapon_effects_change_waits_for_the_next_launch() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    open_prefs(&mut screen);
    assert!(!prefs_dialog(&screen).relaunch_notice_shown());
    let point = prefs_dialog(&screen).weapon_effects().rect().center();
    click_at(&mut screen, point);
    assert!(prefs_dialog(&screen).relaunch_notice_shown());
    assert!(
        flight_view(&screen).weapon_effects(),
        "the running flight keeps the launch value"
    );
    assert_eq!(
        screen.take_prefs(),
        Some(Prefs {
            weapon_effects: false,
            ..Prefs::default()
        }),
        "reported, to be saved"
    );
    screen.input(&key(Key::Escape, true));
    open_prefs(&mut screen);
    assert!(
        prefs_dialog(&screen).relaunch_notice_shown(),
        "opened again, it still differs from launch"
    );

    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).weapon_effects().rect().center();
    click_at(&mut screen, point);
    screen.input(&key(Key::Escape, true));
    fly(&mut screen);
    assert!(
        flight_view(&screen).weapon_effects(),
        "a flight built later this run launches with it too"
    );

    let mut relaunched = with_dialogs(data()).with_prefs(Prefs {
        weapon_effects: false,
        ..Prefs::default()
    });
    fly(&mut relaunched);
    assert!(
        !flight_view(&relaunched).weapon_effects(),
        "the next launch"
    );
}

#[test]
fn by_the_other_reading_a_weapon_effects_change_reaches_flight_at_once() {
    let rules = nova_sim::Rulebook::default().with_override(
        nova_sim::RuleKey::WeaponEffectsNextLaunch,
        nova_sim::RuleSource::Bible,
    );
    let mut screen = with_dialogs(data()).with_rules(rules);
    fly(&mut screen);
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).weapon_effects().rect().center();
    click_at(&mut screen, point);
    assert!(!flight_view(&screen).weapon_effects());
    assert!(!prefs_dialog(&screen).relaunch_notice_shown(), "no notice");

    let mut screen = with_dialogs(data()).with_rules(rules);
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).weapon_effects().rect().center();
    click_at(&mut screen, point);
    screen.input(&key(Key::Escape, true));
    fly(&mut screen);
    assert!(
        !flight_view(&screen).weapon_effects(),
        "and a flight built later"
    );
}

fn alert_quiet(screen: &AppScreen) -> bool {
    screen.flight_view().expect("flying").alert_quiet()
}

#[test]
fn flight_is_built_with_the_routers_sound_prefs() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    assert!(!alert_quiet(&screen), "loud by default");
    let mut screen = with_dialogs(data()).with_prefs(Prefs {
        sound: nova_view::SoundPrefs {
            effects_level: 1,
            ..quiet().sound
        },
        ..quiet()
    });
    fly(&mut screen);
    assert!(alert_quiet(&screen));
}

#[test]
fn a_sound_change_in_the_dialog_reaches_flight_already_built() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).sound().rect().center();
    click_at(&mut screen, point);
    assert!(alert_quiet(&screen), "sound off");
    click_at(&mut screen, point);
    assert!(!alert_quiet(&screen), "back on");
}

#[test]
fn a_change_before_flight_is_built_reaches_it_when_it_is() {
    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    let point = prefs_dialog(&screen).hyperspace_effects().rect().center();
    click_at(&mut screen, point);
    screen.input(&key(Key::Escape, true));
    fly(&mut screen);
    assert!(!flight_effects(&screen));
}

#[test]
fn a_p_repeat_or_release_opens_nothing_and_reaches_nothing() {
    let mut screen = with_dialogs(data());
    for input in [held(PREFS), key(PREFS, false)] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::ShipBrowser, "{input:?}");
    }
    fly(&mut screen);
    for input in [held(PREFS), key(PREFS, false)] {
        assert_eq!(screen.input(&input), ScreenAction::None);
        assert_eq!(screen.showing(), Showing::Flight, "{input:?}");
    }
    open_prefs(&mut screen);
    screen.input(&key(PREFS, false));
    screen.input(&held(PREFS));
    assert_eq!(screen.showing(), Showing::Preferences, "still open");
}

#[test]
fn without_dialogs_p_does_nothing() {
    let mut screen = AppScreen::new(data());
    let before = drawn(&screen);
    assert_eq!(screen.input(&key(PREFS, true)), ScreenAction::None);
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert_eq!(drawn(&screen), before);
    fly(&mut screen);
    command_comma(&mut screen);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.preferences().is_none());
    screen.input(&key(PREFS, true));
    assert_eq!(screen.showing(), Showing::Flight, "P is the player info");
    assert!(
        screen.flight_player_info().is_none(),
        "no dialogs to show it"
    );
}

#[test]
fn a_missing_preferences_dialog_opens_nothing() {
    let mut screen = AppScreen::new(data()).with_dialogs(Rc::new(NoDialogs), Rc::new(MonoMetrics));
    screen.input(&key(PREFS, true));
    assert_eq!(screen.showing(), Showing::ShipBrowser);
    assert!(screen.preferences().is_none());
}

#[test]
fn flight_is_paused_and_lets_go_of_its_keys_while_the_dialog_is_open() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK);
    let moving = ship(&screen);
    open_prefs(&mut screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen), moving, "paused");
    // Up's release goes to the dialog.
    screen.input(&key(Key::Up, false));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    let coasting = ship(&screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
}

#[test]
fn while_the_preferences_are_open_input_reaches_only_them() {
    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    for input in [
        key(Key::Right, true),
        key(Key::Char('f'), true),
        key(Key::Char('i'), true),
        key(PREFS, true),
    ] {
        assert_eq!(screen.input(&input), ScreenAction::None);
    }
    assert_eq!(screen.showing(), Showing::Preferences);
    assert_eq!(screen.ship_browser().selected(), Some(ShipId(128)));
    assert!(screen.flight_view().is_none() && screen.about().is_none());
    screen.input(&key(Key::Tab, true));
    assert_eq!(
        prefs_dialog(&screen).focus(),
        Some(nova_view::ui::prefs::Focus::Music),
        "Tab moves the dialog's focus"
    );
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::None);
    assert_eq!(screen.input(&key(Key::Escape, true)), ScreenAction::Quit);
}

#[test]
fn the_dialog_is_drawn_over_the_side_and_the_hint() {
    let mut screen = with_dialogs(data());
    let below = drawn(&screen);
    open_prefs(&mut screen);
    let list = drawn(&screen);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let below: Vec<&DrawCommand> = below.iter().collect();
    assert_eq!(commands[..below.len()], below[..]);
    assert_eq!(
        drawn(prefs_dialog(&screen)).iter().collect::<Vec<_>>(),
        commands[below.len()..]
    );
}

#[test]
fn only_the_dialog_ticks_while_it_is_open_over_the_ships() {
    let mut screen = with_dialogs(data());
    let tick = Duration::from_millis(100);
    screen.tick(tick);
    let frame = screen.ship_browser().frame();
    open_prefs(&mut screen);
    screen.tick(tick);
    assert_eq!(screen.ship_browser().frame(), frame, "paused below");
    screen.input(&key(Key::Escape, true));
    screen.tick(tick);
    assert_ne!(screen.ship_browser().frame(), frame);
}

#[test]
fn opening_cancels_a_drag_below() {
    let mut screen = with_dialogs(data());
    screen.input(&key(Key::Tab, true));
    let view = *screen.galaxy_map().view().expect("the viewer");
    screen.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: Point::new(500.0, 400.0),
    });
    open_prefs(&mut screen);
    screen.input(&key(Key::Escape, true));
    screen.input(&Input::PointerMoved(Point::new(600.0, 400.0)));
    assert_eq!(
        *screen.galaxy_map().view().expect("the viewer"),
        view,
        "the drag was cancelled"
    );
}

#[test]
fn cancelling_and_releasing_reach_the_preferences_while_open() {
    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    let ok = prefs_dialog(&screen)
        .dialog()
        .item_bounds(1)
        .expect("OK")
        .center();
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: ok,
    };
    screen.input(&button(true));
    screen.release_keys();
    screen.cancel_pointer();
    screen.input(&button(false));
    assert_eq!(
        screen.showing(),
        Showing::Preferences,
        "the click was abandoned"
    );
}

#[test]
fn closing_by_ok_is_silent() {
    let mut screen = with_dialogs(data());
    open_prefs(&mut screen);
    let ok = prefs_dialog(&screen)
        .dialog()
        .item_bounds(1)
        .expect("OK")
        .center();
    assert_eq!(press_and_release(&mut screen, ok), [vec![], vec![]]);
    assert!(screen.preferences().is_none(), "closed");
}

// The player info screen.

/// Clicks `button` on the player info screen open over flight, or
/// else over the spaceport.
fn click_player_info(screen: &mut AppScreen, button: PlayerInfoButton) {
    let open = screen
        .flight_player_info()
        .or_else(|| screen.spaceport_view()?.player_info())
        .expect("the player info is open");
    let at = open.button_bounds(button).expect("laid out").center();
    click_at(screen, at);
}

#[test]
fn p_in_flight_opens_the_player_info_over_the_paused_flight() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(Key::Up, true));
    screen.tick(TICK);
    let moving = ship(&screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    assert_eq!(screen.showing(), Showing::PlayerInfo);
    assert_eq!(screen.now_showing(), Some(Showing::PlayerInfo));
    let open = screen.flight_player_info().expect("open");
    assert_eq!(open.problem(), None, "laid out by DLOG 1017");
    assert_eq!(open.info().credits, pilot(&screen).cash());
    let list = drawn(&screen);
    let over = drawn(open);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    assert_eq!(
        commands[commands.len() - over.iter().count()..],
        over.iter().collect::<Vec<_>>()[..],
        "drawn over flight"
    );
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen), moving, "paused");
    screen.input(&key(PLAYER_INFO_KEY, false));
    screen.input(&key(Key::Up, false));
    screen.input(&key(Key::Tab, true));
    assert_eq!(
        screen.flight_player_info().map(PlayerInfoScreen::tab),
        Some(Tab::Cargo),
        "it takes the input"
    );
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "Escape closes it alone");
    let coasting = ship(&screen);
    screen.tick(TICK * 5);
    assert_eq!(ship(&screen).velocity, coasting.velocity, "no thrust");
}

#[test]
fn p_opens_the_player_info_over_the_spaceport_and_its_dialogs() {
    let store = MemoryPilots::new();
    let check = |screen: &mut AppScreen, what: &str| {
        screen.input(&key(PLAYER_INFO_KEY, true));
        let open = spaceport(screen).player_info().expect(what);
        assert_eq!(open.problem(), None, "{what}");
        assert_eq!(screen.showing(), Showing::Spaceport, "{what}");
        screen.input(&key(Key::Escape, true));
        assert!(spaceport(screen).player_info().is_none(), "{what}");
    };
    let mut screen = landed_trader(&store);
    check(&mut screen, "the spaceport");
    click_port_item(&mut screen, 7);
    check(&mut screen, "the trade center");
    assert!(spaceport(&screen).open_trade().is_some(), "still trading");
    let mut screen = landed_shipyard(&MemoryPilots::new());
    click_port_item(&mut screen, 9);
    check(&mut screen, "the shipyard");
    select_first_ship(&mut screen);
    screen.input(&key(nova_view::spaceport::shipyard::INFO_KEY, true));
    assert!(
        spaceport(&screen)
            .open_shipyard()
            .and_then(ShipyardScreen::info_panel)
            .is_some()
    );
    check(&mut screen, "its Info panel");
    assert!(
        spaceport(&screen)
            .open_shipyard()
            .and_then(ShipyardScreen::info_panel)
            .is_some(),
        "back to the panel"
    );
    let mut screen = landed_at_venues(&MemoryPilots::new(), venue_data(SPACEPORT));
    assert!(spaceport(&screen).offer_dialog().is_some());
    check(&mut screen, "the spaceport's offer");
    assert!(spaceport(&screen).offer_dialog().is_some(), "still offered");
}

/// The player info's background pictures `open` draws, each with its
/// clip.
fn player_info_background(open: &PlayerInfoScreen) -> Vec<DrawCommand> {
    use nova_view::ui::player_info::BACKGROUND;
    drawn(open)
        .iter()
        .filter(|command| match command {
            DrawCommand::Clipped { command, .. } => matches!(
                **command,
                DrawCommand::Picture { ref image, .. }
                | DrawCommand::StretchedPicture { ref image, .. }
                    if BACKGROUND.contains(image)
            ),
            _ => false,
        })
        .cloned()
        .collect()
}

/// What `open` draws of the fixture data's player info background: its
/// upper cap alone ([`UPPER_CAP_SIZE`]), at its own size over the
/// window's top-left, as neither the middle nor the lower cap is there.
fn data_background(open: &PlayerInfoScreen) -> Vec<DrawCommand> {
    let window = open.bounds().expect("laid out");
    let (width, height) = UPPER_CAP_SIZE;
    vec![DrawCommand::Clipped {
        clip: window,
        command: Box::new(DrawCommand::StretchedPicture {
            image: nova_view::ImageKey::picture(8518),
            top_left: window.min,
            width: f32::from(width),
            height: f32::from(height),
        }),
    }]
}

#[test]
fn the_flight_player_info_draws_the_game_datas_background_pictures() {
    let mut screen = with_dialogs(data());
    fly(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    let open = screen.flight_player_info().expect("open");
    assert_eq!(
        player_info_background(open),
        data_background(open),
        "the 300 x 30 upper cap the data holds, and no middle or lower cap"
    );
}

#[test]
fn the_spaceport_player_info_draws_the_game_datas_background_pictures() {
    let mut screen = landed_trader(&MemoryPilots::new());
    screen.input(&key(PLAYER_INFO_KEY, true));
    let open = spaceport(&screen).player_info().expect("open");
    assert_eq!(
        player_info_background(open),
        data_background(open),
        "the 300 x 30 upper cap the data holds, and no middle or lower cap"
    );
}

#[test]
fn confirming_jettison_empties_the_hold_and_closes_it_landed_or_flying() {
    let store = MemoryPilots::new();
    let mut screen = landed_trader(&store);
    click_port_item(&mut screen, 7);
    screen.input(&key(Key::Char('b'), true));
    assert_eq!(pilot(&screen).held(FOOD), 10);
    let writes = store.writes();
    screen.input(&key(PLAYER_INFO_KEY, true));
    click_player_info(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
    click_player_info(&mut screen, PlayerInfoButton::Jettison);
    screen.input(&key(Key::Enter, true));
    assert!(spaceport(&screen).player_info().is_none(), "closed");
    assert_eq!(pilot(&screen).held(FOOD), 0, "jettisoned");
    let open = spaceport(&screen).open_trade().expect("trading");
    assert_eq!(open.market().row(FOOD).map(|row| row.held), Some(0));
    assert_eq!(store.writes(), writes + 1, "saved");
    screen.input(&key(Key::Char('b'), true));
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight, "taken off");
    assert_eq!(pilot(&screen).held(FOOD), 3, "the cash left buys 3");
    past_the_takeoff_grace(&mut screen);
    screen.input(&key(PLAYER_INFO_KEY, true));
    click_player_info(&mut screen, PlayerInfoButton::Tab(Tab::Cargo));
    click_player_info(&mut screen, PlayerInfoButton::Jettison);
    screen.input(&key(Key::Enter, true));
    assert!(screen.flight_player_info().is_none(), "closed");
    assert_eq!(pilot(&screen).held(FOOD), 0, "jettisoned in flight");
}

#[test]
fn the_flight_help_offers_p_and_command_comma() {
    assert!(nova_view::flight::view::HELP.contains("P: player info"));
    assert!(nova_view::flight::view::HELP.contains("Cmd-,: preferences"));
}
