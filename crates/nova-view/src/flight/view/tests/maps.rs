use super::*;
use crate::galaxy::update::MapUpdate;

// Tab, `\`, Return and the map key on the in-flight maps (`_MapFilter`).

#[test]
fn tab_on_the_course_map_draws_the_next_link_and_selects_it() {
    let mut view = FlightView::new(FakeCatalog {
        fan: true,
        ..catalog()
    });
    tap(&mut view, MAP);
    for (k, expected) in [(Key::Tab, 131), (Key::Char('\\'), 132), (Key::Tab, 131)] {
        view.input(&key(k, true));
        assert!(view.map_open(), "{k:?}");
        assert_eq!(
            nav_mode(&view),
            NavMode::Hyperspace(Some(SystemId(expected))),
            "{k:?}"
        );
        let map = view.course_map();
        assert_eq!(map.selected(), Some(SystemId(expected)));
        assert_eq!(map.hyper_selection(), Some(SystemId(expected)));
        assert_eq!(
            hyper_selected_links(&view),
            [(on_map_now(&view, 130), on_map_now(&view, expected))]
        );
        view.input(&key(k, false));
    }
    view.input(&held(Key::Tab));
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(132))),
        "a repeat cycles too"
    );
}

#[test]
fn tab_on_the_course_map_with_no_links_selects_none_and_the_map_selects_the_current_system() {
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    run_set(&mut view, "M132");
    view.settle_script();
    assert_eq!(system_of(&view), SystemId(132), "Barnard lists no links");
    tap(&mut view, MAP);
    tap(&mut view, Key::Tab);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
    assert_eq!(view.course_map().hyper_selection(), None);
    assert_eq!(hyper_selected_links(&view), []);
}

#[test]
fn tab_on_the_course_map_plays_no_sound() {
    let mut view = flight();
    tap(&mut view, MAP);
    view.take_sounds();
    tap(&mut view, Key::Tab);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn tab_on_the_course_map_redraws_the_nav_area_at_once() {
    let mut view = flight();
    tap(&mut view, MAP);
    let stale = NavDisplay::Stellar(Some("Stale".to_owned()));
    let alpha = NavMode::Hyperspace(Some(SystemId(131)));
    view.nav_scene = Some((alpha, stale.clone()));
    tap(&mut view, Key::Tab);
    assert_eq!(nav_mode(&view), alpha);
    assert_eq!(view.nav_shown(), view.nav_now(), "redrawn");
    assert_ne!(view.nav_shown(), stale);
    tap(&mut view, MAP);
    assert_eq!(nav_mode(&view), alpha, "no course to cue");
    assert_eq!(nav(&view), cueing_alpha(), "shown once the map closes");
}

#[test]
fn closing_the_map_cues_the_courses_hop_over_a_tab_selection() {
    let mut view = FlightView::new(FakeCatalog {
        fan: true,
        ..catalog()
    });
    plot(&mut view, 131);
    tap(&mut view, MAP);
    tap(&mut view, Key::Tab);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
    tap(&mut view, MAP);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(Some(SystemId(131))),
        "the close's cue (@0x13c16)"
    );
}

/// [`gated`] with Alpha Reborn standing for a hidden Alpha Centauri,
/// and hypergate 300 linked to 310, in Alpha Centauri, and 320, in
/// Barnard: the hypergate map is open, offering Alpha Reborn and
/// Barnard.
fn at_a_shadowed_gate() -> View {
    let mut view = FlightView::new(FakeCatalog {
        reborn: true,
        onward: true,
        gates: vec![
            gate_site(300, (0.0, 0.0), 0x1200, &[310, 320], 120),
            gate_site(310, (100.0, 200.0), 0x1000, &[300], 90),
            gate_site(320, (-50.0, 0.0), 0x1000, &[], 0),
        ],
        gate_listers: vec![
            (SystemId(131), StellarId(310)),
            (SystemId(132), StellarId(320)),
        ],
        ..gated()
    });
    change_galaxy(&mut view, hiding(131));
    view.tick(TICK);
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    view
}

#[test]
fn tab_on_the_hypergate_map_cycles_the_gates_destinations_shift_reversing_through_a_shadowed_system()
 {
    let mut view = at_a_shadowed_gate();
    let nav = nav_mode(&view);
    view.take_sounds();
    // `_FindSystemFromStellar` gives hidden 131 for 310, its only
    // lister, unresolved by place.
    for expected in [131, 132, 131] {
        tap(&mut view, Key::Tab);
        assert_eq!(view.course_map().selected(), Some(SystemId(expected)));
    }
    view.input(&key(Key::Shift, true));
    for expected in [132, 131] {
        tap(&mut view, Key::Char('\\'));
        assert_eq!(view.course_map().selected(), Some(SystemId(expected)));
    }
    view.input(&key(Key::Shift, false));
    assert_eq!(nav_mode(&view), nav, "no nav mode written");
    assert_eq!(view.course_map().hyper_selection(), None);
    assert_eq!(view.take_sounds(), []);
    tap(&mut view, MAP);
    assert_eq!(
        view.message(),
        Some(HYPERGATE_CANCELLED),
        "no link resolves to hidden 131 (@0x638cd-0x638ff)"
    );
    assert_eq!(system_of(&view), SystemId(130));
}

#[test]
fn closing_the_hypergate_map_on_a_tab_pick_enters_the_gate() {
    let mut view = at_a_shadowed_gate();
    tap(&mut view, Key::Tab);
    tap(&mut view, Key::Tab);
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
    view.close_map();
    assert_eq!(system_of(&view), SystemId(132));
    assert_eq!(player(&view).position, Vec2::new(-50.0, 0.0), "at 320");
}

#[test]
fn tab_on_the_bars_map_changes_nothing() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    let selected = view.course_map().selected();
    for k in [Key::Tab, Key::Char('\\')] {
        view.port_map_input(&key(k, true));
        view.port_map_input(&key(k, false));
        assert!(view.map_open(), "{k:?}");
        assert_eq!(
            view.session().map(Session::nav_mode),
            Ok(NavMode::Stellar(Some(StellarId(128)))),
            "{k:?}"
        );
        assert_eq!(view.course_map().selected(), selected, "{k:?}");
        assert_eq!(view.course_map().hyper_selection(), None, "{k:?}");
    }
}

// Item 3's update: what the owner's settling redraws (`_MapFilter`'s Tab
// and `\`, `_DoSystemMap`'s Clear Route).

/// Drags the open map from empty space a pixel left, too little for the
/// scroll to follow, and releases: the partial update kept.
fn jiggle_map(view: &mut View) {
    let (from, to) = (at(300.0, 200.0), at(299.0, 200.0));
    let button = |pressed, at| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    view.input(&button(true, from));
    view.input(&Input::PointerMoved(to));
    view.input(&button(false, to));
    assert_eq!(view.course_map().update(), Some(MapUpdate::Partial));
}

#[test]
fn tab_on_the_course_map_redraws_item_3_with_a_link_or_none() {
    let mut view = flight();
    tap(&mut view, MAP);
    jiggle_map(&mut view);
    tap(&mut view, Key::Tab);
    assert_eq!(view.course_map().update(), Some(MapUpdate::Full), "a link");
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    run_set(&mut view, "M132");
    view.settle_script();
    tap(&mut view, MAP);
    jiggle_map(&mut view);
    tap(&mut view, Key::Char('\\'));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert_eq!(
        view.course_map().update(),
        Some(MapUpdate::Full),
        "no link: the course cycle redraws anyway (@0x14032)"
    );
}

#[test]
fn tab_on_the_hypergate_map_redraws_item_3_as_it_picks() {
    let mut view = at_a_shadowed_gate();
    jiggle_map(&mut view);
    tap(&mut view, Key::Tab);
    assert_eq!(view.course_map().update(), Some(MapUpdate::Full));
}

#[test]
fn tab_on_the_bars_map_redraws_nothing() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    let (from, to) = (at(300.0, 200.0), at(299.0, 200.0));
    let button = |pressed, at| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    view.port_map_input(&button(true, from));
    view.port_map_input(&Input::PointerMoved(to));
    view.port_map_input(&button(false, to));
    assert_eq!(view.course_map().update(), Some(MapUpdate::Partial));
    view.port_map_input(&key(Key::Tab, true));
    assert_eq!(
        view.course_map().update(),
        Some(MapUpdate::Partial),
        "_onPlanet: the key falls through"
    );
}

#[test]
fn clear_route_redraws_item_3_by_c_or_its_button() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, MAP);
    jiggle_map(&mut view);
    tap(&mut view, Key::Char('c'));
    assert_eq!(view.session().map(|session| session.course().len()), Ok(0));
    assert_eq!(view.course_map().update(), Some(MapUpdate::Full), "C");
    let mut view = flight();
    tap(&mut view, MAP);
    jiggle_map(&mut view);
    tap(&mut view, Key::Char('C'));
    assert_eq!(
        view.course_map().update(),
        Some(MapUpdate::Full),
        "with no course, whatever the button"
    );
}

// The course map's nebulae by `RuleKey::NebulaPicture`.

/// The `PICT` each nebula the open course map draws is drawn from, and
/// whether it is drawn with `addMax`.
fn map_nebulae(view: &View) -> Vec<(i16, bool)> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match **command {
                DrawCommand::BlendedPicture { image, blend, .. } => {
                    Some((image.id, blend == crate::draw::Blend::Max))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A flight over Sol's nebula, explored, with the course map open: at
/// 0.5625 its rect is 106 pixels a side.
fn over_the_nebula(rules: Option<Rulebook>) -> View {
    let view = FlightView::new(FakeCatalog {
        nebula: true,
        ..catalog()
    });
    let mut view = match rules {
        Some(rules) => view.with_rules(rules),
        None => view,
    };
    change_galaxy(&mut view, |overlay| {
        overlay.set_nebula_explored(nova_sim::NebulaId(128), true);
    });
    view.tick(TICK);
    tap(&mut view, MAP);
    view
}

#[test]
fn the_course_map_draws_a_nebula_from_the_largest_picture_that_fits() {
    let view = over_the_nebula(None);
    assert_eq!(map_nebulae(&view), [(9500, true)], "30 fits, 120 doesn't");
}

#[test]
fn by_the_other_reading_the_course_map_draws_the_smallest_picture_that_covers() {
    let rules = Rulebook::default().with_override(RuleKey::NebulaPicture, RuleSource::Bible);
    let view = over_the_nebula(Some(rules));
    assert_eq!(map_nebulae(&view), [(9502, true)]);
}

#[test]
fn with_rules_switches_the_nebula_picture_back() {
    let bible = Rulebook::default().with_override(RuleKey::NebulaPicture, RuleSource::Bible);
    let view = FlightView::new(FakeCatalog {
        nebula: true,
        ..catalog()
    })
    .with_rules(bible)
    .with_rules(Rulebook::default());
    let mut view = view;
    change_galaxy(&mut view, |overlay| {
        overlay.set_nebula_explored(nova_sim::NebulaId(128), true);
    });
    view.tick(TICK);
    tap(&mut view, MAP);
    assert_eq!(map_nebulae(&view), [(9500, true)]);
}

#[test]
fn return_closes_the_course_map_with_the_close_beep_and_the_cue() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, NAV_OFF_KEY);
    tap(&mut view, MAP);
    view.take_sounds();
    view.input(&held(Key::Enter));
    assert!(view.map_open(), "a repeat doesn't close it");
    tap(&mut view, Key::Enter);
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
}

#[test]
fn return_on_the_hypergate_map_enters_or_cancels_in_silence() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    view.take_sounds();
    tap(&mut view, Key::Enter);
    assert!(!view.map_open());
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(map_sounds(&mut view), []);
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    view.take_sounds();
    tap(&mut view, Key::Enter);
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(map_sounds(&mut view), []);
}

#[test]
fn return_closes_the_bars_map_and_puts_back_the_bars_nav() {
    // The planner's click writes the nav in port, which the close shows.
    let mut view = planner(in_the_bar(None));
    view.open_bar_map();
    port_click(&mut view, 131);
    view.take_sounds();
    view.port_map_input(&key(Key::Enter, true));
    assert!(!view.map_open());
    assert_eq!(map_sounds(&mut view), []);
    assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
    assert_eq!(
        view.session().map(Session::nav_mode),
        Ok(NavMode::Stellar(Some(StellarId(128)))),
        "the bar's nav put back"
    );
}

#[test]
fn a_key_down_the_map_doesnt_take_closes_it_while_the_map_key_is_held() {
    let mut view = flight();
    view.input(&key(MAP, true));
    assert!(view.map_open());
    // Every key `_MapFilter` tests before the map key's close.
    for k in [
        Key::Tab,
        Key::Char('\\'),
        Key::Space,
        Key::Char('b'),
        Key::Char('c'),
        Key::Char('-'),
        Key::Char('='),
        Key::Char('+'),
        Key::Left,
        Key::Right,
        Key::Up,
        Key::Down,
        Key::Char('f'),
        Key::Char('s'),
    ] {
        tap(&mut view, k);
        assert!(view.map_open(), "{k:?}");
    }
    view.take_sounds();
    view.input(&held(LAND));
    assert!(view.map_open(), "a repeat never closes it");
    view.input(&key(LAND, true));
    assert!(!view.map_open(), "L with M held");
    assert_eq!(map_sounds(&mut view), [UiSound::MapClosed]);
    view.input(&key(LAND, false));
    view.input(&key(MAP, false));
    tap(&mut view, MAP);
    assert!(view.map_open());
    tap(&mut view, LAND);
    assert!(view.map_open(), "M let go");
}

#[test]
fn letting_go_of_the_keys_lets_go_of_the_map_key() {
    // The window loses focus with M still down, so its release never
    // arrives: the next key the map doesn't take leaves it open.
    let mut view = flight();
    view.input(&key(MAP, true));
    assert!(view.map_open());
    view.release_keys();
    tap(&mut view, LAND);
    assert!(view.map_open(), "in flight");
    let mut view = in_the_bar(None);
    view.open_bar_map();
    view.port_map_input(&held(MAP));
    view.release_keys();
    view.port_map_input(&key(Key::Tab, true));
    assert!(view.map_open(), "in port");
}

#[test]
fn a_modifier_is_no_key_down_and_never_closes_the_map() {
    // A Mac sends no key-down for a modifier alone, so `_MapFilter`
    // never sees one.
    let mut view = flight();
    view.input(&key(MAP, true));
    for k in [
        Key::Shift,
        Key::Control,
        Key::Alt,
        Key::Command,
        Key::CapsLock,
    ] {
        view.input(&key(k, true));
        assert!(view.map_open(), "{k:?}");
        view.input(&key(k, false));
    }
}

#[test]
fn in_port_tab_or_backslash_with_the_map_key_held_closes_the_map() {
    // Both fall through on a planet (@0x13d20).
    for k in [Key::Tab, Key::Char('\\')] {
        let mut view = in_the_bar(None);
        view.open_bar_map();
        view.port_map_input(&held(MAP));
        view.port_map_input(&key(k, true));
        assert!(!view.map_open(), "{k:?}");
    }
}

#[test]
fn closing_the_map_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
    // The course map, closed by its key or as Escape does; then the
    // hypergate map, cancelled and entered (`_PlayerEnterHypergate`).
    // The gate's Sol lists Earth and the Moon too, so its radar holds
    // them.
    let gated_sol = || {
        let mut gated = gated();
        gated.sites.extend(catalog().sites);
        gated
    };
    for (catalog, pick, by_key) in [
        (catalog(), false, true),
        (catalog(), false, false),
        (gated_sol(), false, true),
        (gated_sol(), true, true),
    ] {
        let gate = !catalog.gates.is_empty();
        let mut view = FlightView::new(catalog);
        ticks(&mut view, 3);
        if gate {
            land_now(&mut view);
            assert_eq!(view.course_map().mode(), MapMode::Hypergate);
        } else {
            tap(&mut view, MAP);
        }
        assert!(view.map_open());
        if pick {
            let alpha = on_map(&mut view, 131);
            click(&mut view, alpha);
        }
        let stale = RadarScene::default();
        assert_ne!(view.radar_now(), stale);
        view.radar_scene = Some(stale);
        let cadence = view.hud_cadence;
        if by_key {
            tap(&mut view, MAP);
        } else {
            view.close_map();
        }
        let case = format!("gate {gate}, pick {pick}, by key {by_key}");
        assert!(!view.map_open(), "{case}");
        assert_eq!(
            view.hud_cadence, cadence,
            "neither timed nor flipped: {case}"
        );
        assert_eq!(*view.radar_shown(), view.radar_now(), "{case}");
    }
}

#[test]
fn without_hyperspace_effects_a_hypergate_flashes_white() {
    let mut view = FlightView::new(gated()).with_hyperspace_effects(false);
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash(0.0))
    );
    view.tick(ARRIVAL_FLASH_FOR);
    assert_eq!(view.jump_effect(), None);
}

#[test]
fn a_hypergate_without_links_does_nothing() {
    let mut catalog = gated();
    catalog.gates[0].links = [None; 8];
    let mut view = FlightView::new(catalog);
    tap(&mut view, LAND);
    ticks(&mut view, 1);
    let cleared = view.message().map(str::to_owned);
    tap(&mut view, LAND);
    assert!(!view.map_open());
    assert_eq!(system_of(&view), SystemId(130));
    assert_eq!(view.message(), cleared.as_deref(), "nothing said");
    assert_eq!(nav_target(&view), None);
}

#[test]
fn a_hypergate_whose_links_lead_nowhere_opens_an_empty_map_that_cancels() {
    let mut catalog = gated();
    catalog.gates[0].links[0] = Some(StellarId(999));
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    assert!(view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_ne!(view.message(), Some(HYPERGATE_CANCELLED), "not yet");
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(system_of(&view), SystemId(130));
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
}

#[test]
fn l_twice_at_a_wormhole_passes_through_it_with_a_flash() {
    let mut view = FlightView::new(holed());
    land_now(&mut view);
    assert!(!view.map_open());
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(player(&view).position, Vec2::new(-300.0, 40.0));
    assert_eq!(
        view.message(),
        Some("Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.")
    );
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash(0.0))
    );
    let mut plain = FlightView::new(holed()).with_hyperspace_effects(false);
    land_now(&mut plain);
    assert_eq!(
        plain.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash(0.0)),
        "whatever the preference"
    );
    let passed = "Passing through a wormhole into the Alpha Centauri system on June 23, 1177 NC.";
    assert_lasts(&mut plain, passed, 240);
}

#[test]
fn coming_out_of_a_hypergate_says_so_for_360_ticks() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    tap(&mut view, MAP);
    let exited = "Exiting hypergate in the Alpha Centauri system on June 23, 1177 NC.";
    assert_lasts(&mut view, exited, 360);
}

#[test]
fn a_wormhole_with_no_exit_says_why() {
    let mut catalog = holed();
    catalog.gates.truncate(1);
    let mut view = FlightView::new(catalog);
    land_now(&mut view);
    assert_eq!(system_of(&view), SystemId(130));
    assert_eq!(view.jump_effect(), None);
    let why = "Your ship is unable to enter this wormhole - the radiation levels are too extreme.";
    assert_lasts(&mut view, why, GATE_REFUSAL_TICKS);
}

/// The sprites of ship 129's sheet (`rlëD` 2001) drawn.
fn ship_129_sprites(list: &DrawList) -> usize {
    sprites(list)
        .iter()
        .filter(|(image, _)| image.id == 2001)
        .count()
}

#[test]
fn an_attacker_following_through_a_wormhole_is_not_drawn_while_it_emerges_but_is_on_radar() {
    let mut catalog = FakeCatalog {
        sites: holed().sites,
        gates: holed().gates,
        gate_listers: holed().gate_listers,
        ..trafficked(&[130, 131], 0, 129, 3)
    };
    catalog.dudes[0].1.govt = Some(GovtId(140));
    catalog.govts = vec![nova_sim::GovtRecord {
        id: GovtId(140),
        name: String::new(),
        flags: 0,
        flags2: nova_sim::govt::ALWAYS_FOLLOWS_THROUGH_WORMHOLE,
        crime_tol: 0,
        initial_rec: 0,
        penalties: nova_sim::Penalties::default(),
        max_odds: 100,
        classes: [-1; 4],
        allies: [-1; 4],
        enemies: [-1; 4],
        comm_name: String::new(),
        scan_mask: 0,
        scan_fine: 0,
        news_pic: -1,
        require: 0,
        inh_jam: [0; 4],
    }];
    // One ship of düde 128 (ship 129) attacking the player and
    // following it (`ShipSyst` -6).
    catalog.missions = vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            syst: -6,
            dude: Some(nova_sim::DudeId(128)),
            behav: 0,
            start: 0,
            ..MissionShip::NONE
        },
        ..errand(140)
    }];
    let mut view = FlightView::new(catalog).with_behaviour(Rc::new(Still));
    run_set(&mut view, "S140");
    settle_session(&mut view);
    land_now(&mut view);
    assert_eq!(system_of(&view), SystemId(131));
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert!(
        session.npcs().iter().any(Npc::emerging),
        "the attacker emerges after the player"
    );
    let list = drawn(&view);
    assert_eq!(
        sprites(&list),
        [
            (ImageKey::sprite(1140, 0), at(812.0, 344.0)),
            (ImageKey::sprite(2000, 0), VIEW_CENTER),
        ],
        "Proxima and the player, not the ship emerging"
    );
    // Once the radar has refreshed, the ship is on it, still emerging.
    ticks(&mut view, 8);
    let session = view.session().expect("flying");
    assert!(session.npcs().iter().any(Npc::emerging));
    assert_eq!(blips(&view).len(), 1, "on radar while it emerges");
    while view
        .session()
        .expect("flying")
        .npcs()
        .iter()
        .any(Npc::emerging)
    {
        view.tick(TICK);
    }
    assert_eq!(ship_129_sprites(&drawn(&view)), 1, "drawn once out");
}

#[test]
fn the_escort_menu_closes_once_its_only_mission_escort_is_disabled() {
    let mut catalog = trafficked(&[130], 0, 129, 3);
    // One ship of düde 128 escorting the player (`ShipBehav` 1).
    catalog.missions = vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            syst: -1,
            dude: Some(nova_sim::DudeId(128)),
            behav: 1,
            start: 0,
            ..MissionShip::NONE
        },
        ..errand(140)
    }];
    let mut view = FlightView::new(catalog).with_behaviour(Rc::new(Still));
    run_set(&mut view, "S140");
    settle_session(&mut view);
    view.tick(TICK);
    let escort = view
        .session()
        .expect("flying")
        .npcs()
        .iter()
        .find(|npc| npc.mission_escort())
        .map(|npc| npc.id)
        .expect("a mission escort");
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open(), "a mission escort opens it");
    nova_sim::fixture::disable(view.session.as_mut().expect("flying"), escort);
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert!(
        session.escort_menu().iter().any(|row| row.present),
        "its row stays"
    );
    assert!(!view.escort_menu().is_open(), "no escort is flying");
}

/// The NPCs of [`escorted_on_a_mission`].
#[derive(Clone, Copy)]
struct Ships {
    /// The traffic ship, ship 129.
    traffic: NpcId,
    /// The hired escort, ship 130, drawn as a crossed box.
    hired: NpcId,
    /// The mission escort, ship 129.
    mission: NpcId,
}

/// [`escorted`]'s flight with mission 140's one ship of düde 128
/// (ship 129, whose sheet is 2001) escorting the player too, as
/// `ShipBehav` 1, set up with the system, and room for a traffic ship
/// beside it: in NPC order the mission escort, the traffic ship and
/// the hired escort.
fn escorted_on_a_mission() -> (View, Ships) {
    let mut catalog = boardable();
    for record in &mut catalog.ships {
        if record.id == ShipId(130) {
            record.escort_type = 2;
            record.inherent_ai = 3;
        }
    }
    // Room for a traffic ship beside the mission's.
    for (_, traffic) in &mut catalog.traffic {
        traffic.avg_ships = 2;
    }
    catalog.missions = vec![MissionRecord {
        ships: MissionShip {
            count: 1,
            syst: -1,
            dude: Some(nova_sim::DudeId(128)),
            behav: 1,
            start: 0,
            ..MissionShip::NONE
        },
        ..errand(140)
    }];
    let pilot = with_escorts(&Pilot::new(&catalog, "Escorted").expect("starts"), &[130]);
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    run_set(&mut view, "S140");
    settle_session(&mut view);
    view.tick(TICK);
    let session = view.session().expect("flying");
    let find = |is: fn(&Npc) -> bool| {
        let found: Vec<NpcId> = session
            .npcs()
            .iter()
            .filter(|npc| is(npc))
            .map(|npc| npc.id)
            .collect();
        assert_eq!(found.len(), 1, "{:?}", session.npcs());
        found[0]
    };
    let ships = Ships {
        traffic: find(|npc| !npc.escorts_player()),
        hired: find(|npc| npc.escort.is_some()),
        mission: find(Npc::mission_escort),
    };
    assert_eq!(session.npcs().len(), 3);
    let order: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
    assert_eq!(
        order,
        [ships.mission, ships.traffic, ships.hired],
        "the traffic ship between the escorts in NPC order"
    );
    (view, ships)
}

/// The index in `list` of the player's sprite (sheet 2000).
fn player_drawn(list: &DrawList) -> usize {
    list.iter()
        .position(
            |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 2000),
        )
        .expect("the player drawn")
}

/// Where `view` draws its traffic ship, its hired escort, its mission
/// escort and the player, as indices into what it draws.
fn ship_indices(view: &View, ships: Ships) -> [usize; 4] {
    let list = drawn(view);
    let at = |id| drawn_at(&list, npc_on_screen(view, id));
    let ats = [ships.traffic, ships.hired, ships.mission].map(|id| npc_on_screen(view, id));
    assert!(
        ats.iter().enumerate().all(|(i, a)| ats[i + 1..]
            .iter()
            .all(|b| (a.x - b.x).abs() > 1.0 || (a.y - b.y).abs() > 1.0)),
        "each ship apart: {ats:?}"
    );
    [
        at(ships.traffic),
        at(ships.hired),
        at(ships.mission),
        player_drawn(&list),
    ]
}

#[test]
fn a_hired_and_a_mission_escort_draw_in_the_escorts_layer() {
    let (view, ships) = escorted_on_a_mission();
    let session = view.session().expect("flying");
    let berth = |id| {
        let npc = session.npcs().iter().find(|npc| npc.id == id);
        npc.expect("here").berth
    };
    assert!(berth(ships.hired) < berth(ships.mission));
    let [traffic, hired, led, player] = ship_indices(&view, ships);
    assert!(
        hired < led,
        "the escorts in the order they entered their layer, by berth: {hired} {led}"
    );
    assert!(led < traffic, "under the other ships: {led} {traffic}");
    assert!(traffic < player, "the player on top: {traffic} {player}");
}

#[test]
fn a_disabled_ship_draws_under_the_escorts() {
    let (mut view, ships) = escorted_on_a_mission();
    let session = view.session.as_mut().expect("flying");
    nova_sim::fixture::disable(session, ships.traffic);
    let [traffic, hired, led, player] = ship_indices(&view, ships);
    assert!(traffic < hired, "{traffic} {hired}");
    assert!(hired < led && led < player, "{hired} {led} {player}");
}

#[test]
fn cloak_alpha_maps_each_shown_state_to_a_tint_alpha() {
    assert_eq!(cloak_alpha(CloakShown::Opaque), Some(255));
    assert_eq!(cloak_alpha(CloakShown::Translucent(0x1e)), Some(135));
    assert_eq!(cloak_alpha(CloakShown::Translucent(0)), Some(255));
    assert_eq!(
        cloak_alpha(CloakShown::Translucent(0x20)),
        Some(128),
        "127.5, rounded to nearest"
    );
    assert_eq!(cloak_alpha(CloakShown::Hidden), None);
}

#[test]
fn a_fully_cloaked_escort_is_drawn_at_its_translucent_alpha() {
    let (mut view, ships) = escorted_on_a_mission();
    let session = view.session.as_mut().expect("flying");
    nova_sim::fixture::cloak(session, ships.mission, CLOAKED);
    let list = drawn(&view);
    let at = npc_on_screen(&view, ships.mission);
    let tint = list.iter().find_map(|command| match *command {
        DrawCommand::Sprite {
            image,
            center,
            tint,
            ..
        } if image.id == 2001 && center == at => Some(tint),
        _ => None,
    });
    assert_eq!(tint.map(|tint| tint.a), Some(135));
}

// The floating map.

/// The floating map's frame's centre, where Sol is drawn.
const FRAME_CENTRE: Point = Point::new(686.0, 278.0);
/// A point in the frame that is in no box, where [`with_sites`] puts
/// stellar 128.
const FRAME_MISS: Point = Point::new(700.0, 300.0);
/// Where [`with_sites`] puts stellar 129, outside the frame.
const OUTSIDE_FRAME: Point = Point::new(512.0, 284.0);

/// [`near`]'s flight with Sol's stellars 128 under [`FRAME_MISS`] and
/// 129 under [`OUTSIDE_FRAME`], as the camera starts.
fn with_sites() -> View {
    let view = FlightView::new(FakeCatalog {
        near: true,
        sites: vec![
            site(128, (188.0, -84.0), StellarFlags::CAN_LAND),
            site(129, (0.0, -100.0), StellarFlags::CAN_LAND),
        ],
        ..catalog()
    });
    assert_eq!(on_screen(&view, (188.0, -84.0)), FRAME_MISS);
    assert_eq!(on_screen(&view, (0.0, -100.0)), OUTSIDE_FRAME);
    view
}

/// Where the floating map draws systems' ovals, in draw order: the
/// centre of each run of 1-pixel rows in a colour code's colour, each
/// row the one after the last, inside the frame. Its black fills cover
/// but draw nothing.
fn floating_dots(view: &View) -> Vec<Point> {
    let codes = [
        DARK_GREY,
        LIGHT_GREY,
        BRIGHT_GREEN,
        CLEARED,
        DENIED,
        HOSTILE,
    ];
    let frame = floating::frame(VIEW_SIZE);
    let mut ovals: Vec<(Color, f32, Bounds)> = Vec::new();
    for command in &drawn(view) {
        let DrawCommand::Line {
            from,
            to,
            width,
            color,
        } = *command
        else {
            continue;
        };
        let opaque = Color { a: 255, ..color };
        if width != 1.0 || from.y != to.y || !codes.contains(&opaque) {
            continue;
        }
        if !floating::contains(frame, from) {
            continue;
        }
        let row = Bounds {
            min: Point::new(from.x.min(to.x), from.y - 0.5),
            max: Point::new(from.x.max(to.x), from.y + 0.5),
        };
        match ovals.last_mut() {
            Some((last, top, area))
                if *last == opaque && (row.min.y == *top || row.min.y == *top + 1.0) =>
            {
                *top = row.min.y;
                area.min.x = area.min.x.min(row.min.x);
                area.max.x = area.max.x.max(row.max.x);
                area.max.y = row.max.y;
            }
            _ => ovals.push((opaque, row.min.y, row)),
        }
    }
    ovals
        .into_iter()
        .map(|(_, _, area)| area.center())
        .collect()
}

/// Where the floating map draws Proxima Near (134) at the zoom whose
/// scale is `scale`: the pixel its point truncates to.
fn near_at(scale: f32) -> Point {
    Point::new(
        (686.0 + 40.0 * scale).trunc(),
        (278.0 + 30.0 * scale).trunc(),
    )
}

#[test]
fn holding_h_shows_the_floating_map_bordered_and_centred_on_sol() {
    let mut view = flight();
    assert_eq!(floating_border(&drawn(&view)), [], "down at first");
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    let list = drawn(&view);
    let edge = |from: (f32, f32), to: (f32, f32)| DrawCommand::Line {
        from: at(from.0, from.1),
        to: at(to.0, to.1),
        width: 1.0,
        color: MENU_COLORS.border,
    };
    assert_eq!(
        floating_border(&list),
        [
            edge((558.0, 150.0), (814.0, 150.0)),
            edge((814.0, 150.0), (814.0, 406.0)),
            edge((814.0, 406.0), (558.0, 406.0)),
            edge((558.0, 406.0), (558.0, 150.0)),
        ]
    );
    assert_eq!(floating_dots(&view), [FRAME_CENTRE], "Sol, at the centre");
    assert!(
        list.iter().any(|command| matches!(command,
            DrawCommand::Line { color, .. } if *color == CYAN)),
        "the current system's dot in it"
    );
}

#[test]
fn holding_h_keeps_it_full_and_letting_go_starts_the_fade() {
    let mut view = flight();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.input(&held(HYPERSPACE_MODE_KEY));
    for _ in 0..100 {
        view.tick(TICK);
    }
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255), "shown again each frame");
    view.tick(mac_ticks(1));
    assert_eq!(floating_alpha(&view), Some(247), "31 of 32 at 119");
    view.tick(mac_ticks(31));
    assert_eq!(floating_alpha(&view), None, "faded out at 150");
}

#[test]
fn a_tap_of_h_shows_it_before_the_next_frame() {
    let mut view = flight();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(floating_alpha(&view), Some(255));
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255));
    view.tick(mac_ticks(1));
    assert_eq!(floating_alpha(&view), Some(247), "not shown again");
}

#[test]
fn in_double_time_the_fade_keeps_its_ticks() {
    let mut view = flight().with_double_time(true);
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255));
    view.tick(mac_ticks(1));
    assert_eq!(floating_alpha(&view), Some(247));
    view.tick(mac_ticks(31));
    assert_eq!(floating_alpha(&view), None);
}

#[test]
fn h_held_through_a_jump_doesnt_show_it() {
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.tick(TICK);
        assert_eq!(floating_alpha(&view), None, "{stage}");
    }
}

#[test]
fn backslash_held_in_mode_3_shows_it_through_a_jump() {
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        assert!(matches!(nav_mode(&view), NavMode::Hyperspace(_)), "{stage}");
        view.input(&key(HYPER_SELECT_KEY, true));
        assert_eq!(floating_alpha(&view), None, "{stage}: no press");
        view.tick(mac_ticks(1));
        assert_eq!(floating_alpha(&view), Some(255), "{stage}");
    }
}

#[test]
fn backslash_held_out_of_mode_3_doesnt_show_it() {
    let mut view = FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    });
    run_set(&mut view, "M132");
    view.settle_script();
    assert_eq!(system_of(&view), SystemId(132), "Barnard lists no links");
    view.input(&key(HYPER_SELECT_KEY, true));
    assert_eq!(nav_mode(&view), NavMode::Off);
    view.tick(TICK);
    assert_eq!(floating_alpha(&view), None);
}

#[test]
fn h_held_in_the_take_off_grace_shows_it() {
    let mut view = flight_among(vec![site(140, (6.0, -8.0), StellarFlags::CAN_LAND)]);
    land_now(&mut view);
    view.take_off().expect("takes off");
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    ticks(&mut view, 14);
    assert!(!view.takes(PlayerCommand::Map), "still in the grace");
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255), "held through the grace");
}

#[test]
fn h_held_once_exploded_doesnt_show_it() {
    let mut view = exploded();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    assert_eq!(floating_alpha(&view), None);
}

#[test]
fn backslash_held_in_mode_3_once_exploded_doesnt_show_it() {
    let mut view = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    tap(&mut view, HYPERSPACE_MODE_KEY);
    tap(&mut view, MENU_CLOSE_KEY);
    until_the_player_explodes(&mut view, |_| {});
    view.tick(TICK);
    assert!(matches!(nav_mode(&view), NavMode::Hyperspace(_)), "kept");
    view.input(&key(HYPER_SELECT_KEY, true));
    view.tick(TICK);
    assert_eq!(floating_alpha(&view), None);
}

#[test]
fn it_hides_once_more_than_500_ticks_have_passed() {
    let mut view = with_sites();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.tick(mac_ticks(500));
    click(&mut view, FRAME_MISS);
    assert_eq!(nav_target(&view), None, "still up at 500: swallowed");
    view.tick(mac_ticks(1));
    click(&mut view, FRAME_MISS);
    assert_eq!(nav_target(&view), Some(StellarId(128)), "hidden at 501");
}

#[test]
fn return_hides_it_through_a_jump_too() {
    let mut view = flight();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    tap(&mut view, MENU_CLOSE_KEY);
    assert_eq!(floating_alpha(&view), None);
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        tap(&mut view, HYPER_SELECT_KEY);
        view.input(&key(HYPER_SELECT_KEY, true));
        view.tick(TICK);
        view.input(&key(HYPER_SELECT_KEY, false));
        assert_eq!(floating_alpha(&view), Some(255), "{stage}");
        tap(&mut view, MENU_CLOSE_KEY);
        assert_eq!(floating_alpha(&view), None, "{stage}");
    }
}

#[test]
fn a_click_on_the_centre_selects_mode_3_with_nothing_and_beeps() {
    let mut view = near();
    plot(&mut view, 131);
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    view.take_sounds();
    click(&mut view, FRAME_CENTRE);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
    view.tick(TICK);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
    assert_eq!(course(&view), [SystemId(131)], "the course kept");
}

#[test]
fn a_click_in_a_links_box_selects_it_in_mode_3_and_beeps() {
    let mut view = near();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    view.take_sounds();
    // Proxima Near, 40 right and 30 down at 16/9: (757.1, 331.3).
    click(&mut view, at(757.0, 331.0));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(134))));
    assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
}

#[test]
fn away_from_the_origin_the_map_centres_on_the_current_system_and_clicks_follow() {
    let mut view = near();
    run_set(&mut view, "M134");
    view.settle_script();
    assert_eq!(system_of(&view), SystemId(134), "Proxima Near, at (40, 30)");
    tap(&mut view, HYPERSPACE_MODE_KEY);
    // Sol, 40 left and 30 up at 16/9: (614.9, 224.7).
    let sol = Point::new(
        (686.0 - 40.0 * (16.0_f32 / 9.0)).trunc(),
        (278.0 - 30.0 * (16.0_f32 / 9.0)).trunc(),
    );
    assert_eq!(
        floating_dots(&view),
        [sol, FRAME_CENTRE],
        "Sol, then Proxima Near at the centre, by ascending ID"
    );
    view.take_sounds();
    click(&mut view, at(614.0, 224.0));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(130))));
    assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
}

#[test]
fn a_miss_in_the_frame_picks_nothing_and_plays_nothing() {
    let mut view = with_sites();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.take_sounds();
    click(&mut view, FRAME_MISS);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert_eq!(view.take_sounds(), []);
    click(&mut view, OUTSIDE_FRAME);
    assert_eq!(
        nav_target(&view),
        Some(StellarId(129)),
        "outside: a stellar"
    );
    let mut view = with_sites();
    click(&mut view, FRAME_MISS);
    assert_eq!(nav_target(&view), Some(StellarId(128)), "the map down");
}

#[test]
fn a_click_is_taken_while_the_map_is_faded_out() {
    let mut view = with_sites();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.tick(mac_ticks(200));
    assert_eq!(floating_alpha(&view), None, "faded out");
    click(&mut view, FRAME_MISS);
    assert_eq!(nav_target(&view), None, "swallowed");
    view.take_sounds();
    click(&mut view, FRAME_CENTRE);
    assert_eq!(sims(&mut view), [SimSound::HyperSelected]);
}

#[test]
fn a_hit_shows_the_map_again() {
    let mut view = near();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.tick(mac_ticks(100));
    click(&mut view, FRAME_CENTRE);
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255));
    view.tick(mac_ticks(1));
    assert_eq!(floating_alpha(&view), Some(247));
}

#[test]
fn a_click_in_the_frame_during_a_jump_selects_nothing() {
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        view.input(&key(HYPER_SELECT_KEY, true));
        view.tick(TICK);
        assert_eq!(floating_alpha(&view), Some(255), "{stage}");
        let mode = nav_mode(&view);
        view.take_sounds();
        click(&mut view, FRAME_CENTRE);
        assert_eq!(nav_mode(&view), mode, "{stage}");
        assert!(
            !sims(&mut view).contains(&SimSound::HyperSelected),
            "{stage}"
        );
    }
}

#[test]
fn minus_zooms_out_a_step_with_snd_150() {
    let mut view = near();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.take_sounds();
    assert_eq!(floating_dots(&view), [FRAME_CENTRE, near_at(16.0 / 9.0)]);
    tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(floating_dots(&view), [FRAME_CENTRE, near_at(4.0 / 3.0)]);
    assert_eq!(view.take_sounds(), [Sound::Ui(UiSound::FloatingMapZoomed)]);
    assert_eq!(ZOOM_OUT_KEY, Key::Char('-'));
}

#[test]
fn equals_and_keypad_plus_zoom_in_a_step() {
    for k in [ZOOM_IN_KEY, ZOOM_IN_KEYPAD_KEY] {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.take_sounds();
        tap(&mut view, k);
        assert_eq!(floating_dots(&view)[1], near_at(64.0 / 27.0), "{k:?}");
        assert_eq!(
            view.take_sounds(),
            [Sound::Ui(UiSound::FloatingMapZoomed)],
            "{k:?}"
        );
    }
    assert_eq!(ZOOM_IN_KEY, Key::Char('='));
    assert_eq!(ZOOM_IN_KEYPAD_KEY, Key::Char('+'));
}

#[test]
fn a_zoom_at_its_limit_changes_nothing_and_says_nothing() {
    let mut view = near();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    tap(&mut view, ZOOM_IN_KEY);
    view.take_sounds();
    tap(&mut view, ZOOM_IN_KEY);
    assert_eq!(floating_dots(&view)[1], near_at(64.0 / 27.0));
    assert_eq!(view.take_sounds(), []);
    for _ in 0..6 {
        tap(&mut view, ZOOM_OUT_KEY);
    }
    assert_eq!(floating_dots(&view)[1], near_at(27.0 / 64.0));
    view.take_sounds();
    tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(floating_dots(&view)[1], near_at(27.0 / 64.0));
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn a_zoom_key_does_nothing_while_another_key_is_held() {
    for k in [
        Key::Shift,
        Key::Alt,
        Key::Control,
        Key::Command,
        HYPERSPACE_MODE_KEY,
        HYPER_SELECT_KEY,
        NAV_OFF_KEY,
        JUMP_KEY,
        Key::Up,
    ] {
        let mut view = near();
        tap(&mut view, HYPERSPACE_MODE_KEY);
        view.input(&key(k, true));
        view.take_sounds();
        tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(floating_dots(&view)[1], near_at(16.0 / 9.0), "{k:?}");
        assert_eq!(view.take_sounds(), [], "{k:?}");
        view.input(&key(k, false));
        tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(floating_dots(&view)[1], near_at(4.0 / 3.0), "{k:?} let go");
    }
}

#[test]
fn a_zoom_key_does_nothing_with_the_map_down() {
    let mut view = near();
    tap(&mut view, ZOOM_OUT_KEY);
    tap(&mut view, ZOOM_IN_KEY);
    assert_eq!(view.take_sounds(), []);
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(floating_dots(&view)[1], near_at(16.0 / 9.0));
}

#[test]
fn a_zoom_shows_the_map_again() {
    let mut view = near();
    tap(&mut view, HYPERSPACE_MODE_KEY);
    view.tick(mac_ticks(140));
    tap(&mut view, ZOOM_OUT_KEY);
    view.tick(mac_ticks(118));
    assert_eq!(floating_alpha(&view), Some(255));
}

#[test]
fn zooms_are_taken_through_a_jump_and_refused_once_exploded() {
    for mut view in through_a_jump(near) {
        let stage = stage(&view);
        view.input(&key(HYPER_SELECT_KEY, true));
        view.tick(TICK);
        view.input(&key(HYPER_SELECT_KEY, false));
        view.take_sounds();
        tap(&mut view, ZOOM_OUT_KEY);
        assert!(
            view.take_sounds()
                .contains(&Sound::Ui(UiSound::FloatingMapZoomed)),
            "{stage}"
        );
        assert_eq!(view.floating.zoom(), 4, "{stage}");
    }
    let mut view = exploded();
    view.floating.show(sixtieths(view.elapsed));
    view.take_sounds();
    tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(view.floating.zoom(), floating::START_ZOOM);
    assert!(
        !view
            .take_sounds()
            .contains(&Sound::Ui(UiSound::FloatingMapZoomed))
    );
}

#[test]
fn the_map_is_drawn_after_the_double_time_icon_and_before_the_escort_menu() {
    let mut view = escorted().with_double_time(true);
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    tap(&mut view, HYPERSPACE_MODE_KEY);
    let list = drawn(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let icon = commands
        .iter()
        .position(|command| icon_in(20000..=20000)(command))
        .expect("the double-time icon");
    let border = floating_border(&list);
    let map = commands
        .iter()
        .position(|command| **command == border[0])
        .expect("the floating map");
    let menu = commands
        .iter()
        .position(
            |command| matches!(command, DrawCommand::Line { color, .. } if *color == Color::BLACK),
        )
        .expect("the escort menu's black");
    assert!(icon < map && map < menu, "{icon} < {map} < {menu}");
}

/// The rects of `color`, whatever their alpha, in `list`, as (left,
/// top, right, bottom).
fn rects_of(list: &DrawList, of: Color) -> Vec<(f32, f32, f32, f32)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Line {
                from,
                to,
                width,
                color,
            } if Color { a: 255, ..color } == of && from.y == to.y => Some((
                from.x.min(to.x),
                from.y - width / 2.0,
                from.x.max(to.x),
                from.y + width / 2.0,
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn the_floating_map_draws_the_sessions_selection_clipped_to_the_frame() {
    let mut view = flight();
    tap(&mut view, HYPER_SELECT_KEY);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(view.course_map().hyper_selection(), None, "never opened");
    let list = drawn(&view);
    assert_eq!(
        rects_of(&list, DIM_GREEN),
        [(690.0, 278.0, 814.0, 279.0)],
        "from the edge of Sol's oval to the frame's, drawn once"
    );
    assert_eq!(
        rects_of(&list, LESS_DARK_GREY),
        [],
        "the link under it is cut away"
    );
}

#[test]
fn the_floating_map_draws_the_course_as_its_show_took_it() {
    let mut view = flight();
    view.session
        .as_mut()
        .expect("flying")
        .plot_course(SystemId(131))
        .expect("a route");
    assert_eq!(
        view.course_map().route(),
        [],
        "the course map never shown it"
    );
    tap(&mut view, HYPERSPACE_MODE_KEY);
    let course = rects_of(&drawn(&view), BRIGHT_GREEN);
    assert_eq!(
        course,
        [(690.0, 279.0, 814.0, 280.0)],
        "from the edge of Sol's oval to the frame's, at pen 2, its top \
         row under Sol's link"
    );
    assert_eq!(
        rects_of(&drawn(&view), DIM_GREEN),
        [(690.0, 278.0, 814.0, 279.0)],
        "the link to 131, the hop the plot cued, drawn after the course \
         over it, and over its own grey"
    );
    assert_eq!(rects_of(&drawn(&view), LESS_DARK_GREY), []);
    let session = view.session.as_mut().expect("flying");
    let _ = session.plot_course(SystemId(130));
    assert_ne!(session.course(), [SystemId(131)], "the course changed");
    assert_eq!(
        rects_of(&drawn(&view), BRIGHT_GREEN),
        course,
        "until the next show"
    );
}

/// The names the floating map draws, clipped to its frame.
fn floating_names(view: &View) -> Vec<String> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match command.as_ref() {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn above_a_zoom_of_1_1_the_floating_map_names_only_the_system_its_show_selected() {
    let mut view = near();
    run_set(&mut view, "X134");
    tap(&mut view, HYPER_SELECT_KEY);
    tap(&mut view, HYPER_SELECT_KEY);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(134))));
    assert_eq!(
        floating_names(&view),
        ["Sol", "Proxima Near"],
        "every explored system in the frame at 16/9"
    );
    for _ in 0..3 {
        tap(&mut view, ZOOM_OUT_KEY);
    }
    assert_eq!(floating_names(&view), ["Proxima Near"], "at 3/4");
    tap(&mut view, NAV_OFF_KEY);
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(floating_names(&view), [] as [&str; 0], "shown with none");
}

#[test]
fn the_floating_map_takes_its_mission_arrows_as_it_shows() {
    let mut catalog = catalog();
    catalog.missions = vec![MissionRecord {
        travel_stel: 140,
        ..errand(130)
    }];
    let mut view = FlightView::new(catalog);
    tap(&mut view, HYPERSPACE_MODE_KEY);
    let red = |view: &View| {
        view.floating_picture
            .as_ref()
            .expect("shown")
            .systems
            .iter()
            .any(|system| system.id == SystemId(131) && system.red_arrow)
    };
    assert!(!red(&view), "no mission");
    run_set(&mut view, "S130");
    view.settle_script();
    assert!(!red(&view), "not until the next show");
    tap(&mut view, HYPERSPACE_MODE_KEY);
    assert!(red(&view), "Alpha Centauri, 140's system, arrowed");
}

#[test]
fn the_hypergate_maps_offers_get_their_ovals_from_its_second_draw() {
    // The gate leads to Barnard (132), which nothing has made known.
    let mut view = FlightView::new(FakeCatalog {
        gate_listers: vec![(SystemId(132), StellarId(310))],
        ..gated()
    });
    land_now(&mut view);
    let drawn_systems = |view: &View| -> Vec<SystemId> {
        view.course_map()
            .picture()
            .expect("drawn")
            .systems
            .iter()
            .map(|system| system.id)
            .collect()
    };
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(
        view.course_map().picture().expect("drawn").gate_arrows,
        [SystemId(132)]
    );
    assert!(!drawn_systems(&view).contains(&SystemId(132)), "first draw");
    assert!(
        view.session().expect("flying").is_known(SystemId(132)),
        "marked known as its arrow was drawn"
    );
    view.tick(TICK);
    assert!(!drawn_systems(&view).contains(&SystemId(132)), "a frame on");
    tap(&mut view, Key::Tab);
    assert!(drawn_systems(&view).contains(&SystemId(132)), "after Tab");
}

#[test]
fn a_click_on_the_hypergate_map_draws_it_again() {
    let mut view = FlightView::new(FakeCatalog {
        gate_listers: vec![(SystemId(132), StellarId(310))],
        ..gated()
    });
    land_now(&mut view);
    let sol = on_map(&mut view, 130);
    click(&mut view, sol);
    let drawn: Vec<SystemId> = view
        .course_map()
        .picture()
        .expect("drawn")
        .systems
        .iter()
        .map(|system| system.id)
        .collect();
    assert!(drawn.contains(&SystemId(132)));
}

#[test]
fn the_course_map_draws_the_sessions_known_and_explored_systems() {
    let mut view = flight();
    tap(&mut view, MAP);
    let picture = view.course_map().picture().expect("drawn").clone();
    let ids: Vec<i16> = picture.systems.iter().map(|system| system.id.0).collect();
    assert_eq!(ids, [130, 131], "Sol and its known link, not Barnard");
    assert_eq!(picture.named, [SystemId(130)]);
    assert_eq!(picture.current, Some(SystemId(130)));
    run_set(&mut view, "X132");
    view.tick(TICK);
    let ids: Vec<i16> = view
        .course_map()
        .picture()
        .expect("drawn")
        .systems
        .iter()
        .map(|system| system.id.0)
        .collect();
    assert_eq!(ids, [130, 131, 132], "explored while the map is open");
}

// The map panel (`_MapDialogUpdate`): the session's panels, drawn from
// the dialog's origin (212, 128), so the strip's left is 220 and top 564,
// and the column's left is 686 and top 136.

/// `STR#` 2002 with the panel's words, every other string empty.
fn panel_strings() -> (i16, Vec<String>) {
    let mut list = vec![String::new(); 400];
    for (n, word) in [
        (308, "Pick a destination"),
        (309, "Ports:"),
        (310, "<Unknown>"),
        (327, "Goods Traded:"),
        (328, "Services:"),
        (329, "Trading"),
        (330, "Outfitting"),
        (331, "Shipyard"),
        (335, "None"),
        (337, "Current System:"),
        (338, "Selected System:"),
        (339, "Destination System:"),
        (340, "Hypergate Destination:"),
    ] {
        list[n - 1] = word.to_owned();
    }
    (nova_sim::news::MISC_STRINGS, list)
}

/// `view` with the panel's words among its string lists.
fn worded(view: View) -> View {
    let strings = FakeCatalog {
        strings: vec![panel_strings()],
        ..catalog()
    };
    view.with_strings(Rc::new(strings))
}

/// The Geneva 9 text drawn with its baseline at (`x`, `baseline`), if
/// any: the panels' lines.
fn panel_text(list: &DrawList, (x, baseline): (f32, f32)) -> Option<String> {
    list.iter().find_map(|command| match command {
        DrawCommand::Text {
            text, origin, size, ..
        } if *size == 9.0 && *origin == Point::new(x, baseline - 9.0) => Some(text.clone()),
        _ => None,
    })
}

/// Where the column's title, the Ports list, the first goods row and the
/// first two services rows are drawn.
const TITLE_AT: (f32, f32) = (696.0, 148.0);
const PORTS_AT: (f32, f32) = (265.0, 576.0);
const GOODS_AT: (f32, f32) = (701.0, 304.0);
const SERVICES_AT: (f32, f32) = (701.0, 399.0);
const SECOND_SERVICE_AT: (f32, f32) = (701.0, 411.0);

fn title(view: &View) -> Option<String> {
    panel_text(&drawn(view), TITLE_AT)
}

#[test]
fn the_course_maps_panel_lists_a_landable_stellar_and_leaves_out_an_uninhabited_and_an_unlandable_one()
 {
    // Phase 15: `_MapDialogUpdate` @0x10373-0x10504.
    let mut place_names = nova_sim::catalog::PlaceNames::default();
    for (id, name) in [(128, "Earth"), (129, "Jupiter"), (150, "Rock")] {
        place_names.stellars.insert(StellarId(id), name.to_owned());
    }
    let mut view = worded(FlightView::new(FakeCatalog {
        sites: vec![
            site(128, (0.0, -600.0), StellarFlags::CAN_LAND),
            site(
                129,
                (300.0, -200.0),
                StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ),
            site(150, (-300.0, 0.0), 0),
        ],
        place_names,
        ..catalog()
    }));
    tap(&mut view, MAP);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    let list = drawn(&view);
    assert_eq!(panel_text(&list, (230.0, 576.0)).as_deref(), Some("Ports:"));
    assert_eq!(panel_text(&list, PORTS_AT).as_deref(), Some("Earth"));
    let texts = texts(&list);
    assert!(
        !texts
            .iter()
            .any(|text| text.contains("Jupiter") || text.contains("Rock")),
        "{texts:?}"
    );
}

#[test]
fn the_course_maps_panel_titles_the_current_destination_and_selected_systems_and_follows_a_click() {
    // Phase 16: `_MapDialogUpdate` @0x10b3f-0x10c76.
    let mut view = worded(FlightView::new(FakeCatalog { ..catalog() }));
    tap(&mut view, MAP);
    assert_eq!(nav_mode(&view), NavMode::Off);
    assert_eq!(title(&view).as_deref(), Some("Current System:"));
    assert_eq!(
        panel_text(&drawn(&view), PORTS_AT).as_deref(),
        Some(", "),
        "Sol's two unnamed ports"
    );
    // A click on a linked system selects it in mode 3.
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(title(&view).as_deref(), Some("Destination System:"));
    assert_eq!(
        panel_text(&drawn(&view), PORTS_AT).as_deref(),
        Some("<Unknown>"),
        "the panel follows the click to unexplored Alpha Centauri"
    );
    // Off mode 3, any other system is the selected one.
    tap(&mut view, MAP);
    tap(&mut view, NAV_OFF_KEY);
    tap(&mut view, MAP);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    assert_eq!(title(&view).as_deref(), Some("Current System:"));
    know(&mut view, 132);
    let barnard = on_map(&mut view, 132);
    click(&mut view, barnard);
    assert_eq!(view.course_map().selected(), Some(SystemId(132)));
    assert_eq!(nav_mode(&view), NavMode::Off, "no link to Barnard");
    assert_eq!(title(&view).as_deref(), Some("Selected System:"));
}

#[test]
fn the_hypergate_maps_panel_prompts_then_titles_a_destination_its_owner_offers() {
    // Phase 16: `_MapDialogUpdate` @0x10213-0x102cd and @0x10ba2-0x10c34.
    // Gate 300 links to 310, listed by Alpha Reborn (133), shadowed at
    // Alpha Centauri's place, and by Proxima Near (134): its owner is 134,
    // not its lowest lister, 133.
    let mut view = worded(FlightView::new(FakeCatalog {
        reborn: true,
        near: true,
        gates: vec![
            gate_site(300, (0.0, 0.0), 0x1200, &[310], 120),
            gate_site(310, (100.0, 200.0), 0x1000, &[300], 90),
        ],
        gate_listers: vec![
            (SystemId(133), StellarId(310)),
            (SystemId(134), StellarId(310)),
        ],
        ..gated()
    }));
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(view.course_map().offered(), [SystemId(134)]);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    // The prompt at 430 + 594 / 2, unmeasured, on the strip's top + 24.
    let list = drawn(&view);
    assert_eq!(
        panel_text(&list, (517.0, 588.0)).as_deref(),
        Some("Pick a destination")
    );
    assert_eq!(panel_text(&list, PORTS_AT), None, "the prompt alone");
    assert_eq!(title(&view).as_deref(), Some("Current System:"));
    let near = on_map(&mut view, 134);
    click(&mut view, near);
    assert_eq!(view.course_map().selected(), Some(SystemId(134)));
    assert_eq!(title(&view).as_deref(), Some("Hypergate Destination:"));
}

#[test]
fn the_course_maps_panel_shows_a_level_1_systems_goods_and_services_unknown_and_a_level_2_systems_listed()
 {
    // Phase 21: `_MapDialogUpdate` @0x10f34, @0x1107d.
    let earth = StellarFlags::CAN_LAND | 0x0002 | 0x0004 | 0x1000_0000;
    let mut view = worded(FlightView::new(FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), earth)],
        sol_navs: vec![Some((earth, 0))],
        commodities: CommodityStrings {
            names: [
                "Food",
                "Industrial",
                "Medical",
                "Luxury",
                "Metal",
                "Equipment",
            ]
            .map(str::to_owned)
            .to_vec(),
            ..CommodityStrings::default()
        },
        ..catalog()
    }));
    tap(&mut view, MAP);
    let list = drawn(&view);
    assert_eq!(panel_text(&list, GOODS_AT).as_deref(), Some("<Unknown>"));
    assert_eq!(panel_text(&list, SERVICES_AT).as_deref(), Some("<Unknown>"));
    tap(&mut view, MAP);
    land_now(&mut view);
    assert_eq!(view.take_off(), Some(StellarId(128)));
    // Past the take-off's grace, which refuses the map key.
    ticks(&mut view, 60);
    tap(&mut view, MAP);
    assert!(view.map_open());
    let list = drawn(&view);
    assert_eq!(panel_text(&list, GOODS_AT).as_deref(), Some("Food"));
    assert_eq!(panel_text(&list, SERVICES_AT).as_deref(), Some("Trading"));
    assert_eq!(
        panel_text(&list, SECOND_SERVICE_AT).as_deref(),
        Some("Outfitting")
    );
}

#[test]
fn the_flights_metrics_measure_the_course_maps_panel() {
    // The date set against the strip's right edge: 1016 − width − 5.
    let mut view = worded(flight()).with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
    tap(&mut view, MAP);
    let date = view.session().expect("flying").med_date_text();
    let width =
        crate::text::fixture::MonoMetrics::advance(Font::Geneva, 9.0) * date.chars().count() as f32;
    assert_eq!(
        panel_text(&drawn(&view), (801.0 - width, 600.0)),
        Some(date)
    );
}

#[test]
fn a_click_on_empty_space_drags_and_keeps_the_selection_and_its_panel() {
    // A press that selects nothing drags (`_DoSystemMap` @0x137e9),
    // leaving `_selSystem` as it was.
    let mut view = worded(flight());
    tap(&mut view, MAP);
    assert_eq!(title(&view).as_deref(), Some("Current System:"));
    click(&mut view, Point::new(250.0, 150.0));
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    assert_eq!(title(&view).as_deref(), Some("Current System:"));
    assert_eq!(
        panel_text(&drawn(&view), PORTS_AT).as_deref(),
        Some(", "),
        "Sol's two unnamed ports"
    );
}

// Where the in-flight maps look (`_mapZoom`, `_mapScroll`), and what is
// drawn behind their dialog.

/// The course map's zoom and scroll.
fn map_place(view: &View) -> (f64, (i32, i32)) {
    let place = view.course_map().place().expect("an in-flight map");
    (place.zoom(), place.scroll())
}

/// Where system `id` is in the galaxy, as the course map's scroll holds
/// it.
fn scroll_of(view: &View, id: i16) -> (i32, i32) {
    let at = view
        .course_map()
        .model()
        .position(SystemId(id))
        .expect("on the map");
    (at.x as i32, at.y as i32)
}

#[test]
fn a_fresh_flights_map_opens_on_the_current_system_at_0_5625() {
    let mut pilot = nova_sim::Pilot::new(&catalog(), "Ada").expect("starts");
    pilot.system = SystemId(131);
    let mut view = FlightView::with_pilot(catalog(), pilot);
    tap(&mut view, MAP);
    assert!(view.map_open());
    assert_eq!(map_place(&view), (0.5625, (600, 0)));
    assert_eq!(view.course_map_zoom(), Some(0.5625));
}

#[test]
fn the_maps_zoom_and_scroll_are_kept_between_opens() {
    let mut view = flight();
    tap(&mut view, MAP);
    tap(&mut view, Key::Char('='));
    tap(&mut view, Key::Right);
    view.input(&held(Key::Right));
    assert_eq!(map_place(&view), (0.421_875, (20, 0)));
    tap(&mut view, MAP);
    assert!(!view.map_open());
    ticks(&mut view, 5);
    tap(&mut view, MAP);
    assert_eq!(map_place(&view), (0.421_875, (20, 0)), "where it was left");
}

#[test]
fn a_zoom_given_to_the_flight_is_the_maps() {
    let view = flight().with_course_map_zoom(1.777_776);
    assert_eq!(view.course_map_zoom(), Some(1.777_776));
    assert_eq!(map_place(&view).0, 1.777_776);
}

#[test]
fn opening_the_map_guards_a_zoom_of_0_or_less() {
    let mut view = flight().with_course_map_zoom(0.0);
    tap(&mut view, MAP);
    assert_eq!(map_place(&view).0, 1.0);
}

#[test]
fn a_jumps_arrival_puts_the_scroll_back_on_the_current_system_keeping_the_zoom() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, MAP);
    tap(&mut view, Key::Char('-'));
    tap(&mut view, MAP);
    view.map.centre_on_system(SystemId(132));
    let zoom = map_place(&view).0;
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(map_place(&view), (zoom, scroll_of(&view, 131)));
}

#[test]
fn a_hypergates_exit_puts_the_scroll_back_on_the_current_system() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    view.map.centre_on_system(SystemId(132));
    tap(&mut view, MAP);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(map_place(&view).1, scroll_of(&view, 131));
}

#[test]
fn the_escape_pods_rescue_puts_the_scroll_back_on_the_current_system() {
    let mut catalog = ejecting_in(100, &[188, 187]);
    catalog.home = true;
    let mut view = ejecting_view(catalog, &[]);
    assert!(until_ejected(&mut view).0);
    view.map.centre_on_system(SystemId(132));
    for _ in 0..nova_sim::POD_TICKS {
        view.tick(TICK);
        if view.take_rescue_desc().is_some() {
            break;
        }
    }
    let system = view.session().expect("flying").system();
    assert_eq!(map_place(&view).1, scroll_of(&view, system.0));
}

#[test]
fn a_floating_map_show_puts_the_scroll_back_on_the_current_system() {
    let mut view = flight();
    view.map.centre_on_system(SystemId(131));
    assert_eq!(map_place(&view).1, (600, 0));
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    assert_eq!(floating_alpha(&view), Some(255), "the floating map up");
    assert_eq!(map_place(&view).1, scroll_of(&view, 130));
}

#[test]
fn the_bars_map_shows_the_scroll_the_flight_left() {
    let mut view = in_the_bar(None);
    view.map.centre_on_system(SystemId(132));
    view.open_bar_map();
    assert_eq!(map_place(&view).1, (0, 600));
}

#[test]
fn the_hypergate_map_draws_the_dialog_with_clear_route_disabled() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    let list = drawn(&view);
    let clear_route = list.iter().find_map(|command| match command {
        DrawCommand::Text { text, color, .. } if text == "Clear Route" => Some(*color),
        _ => None,
    });
    assert_eq!(
        clear_route,
        Some(crate::ui::button::ButtonStyle::STOCK.grey)
    );
}

/// The index in `list` of the map's background, `PICT` 8509.
fn background_at(list: &DrawList) -> Option<usize> {
    list.iter().position(|command| {
        matches!(command, DrawCommand::StretchedPicture { image, .. }
            if *image == ImageKey::picture(crate::galaxy::dialog::BACKGROUND))
    })
}

#[test]
fn with_the_map_open_in_flight_the_paused_scene_is_drawn_behind_the_dialog() {
    let mut view = flight();
    let shut = drawn(&view);
    tap(&mut view, MAP);
    let list = drawn(&view);
    let background = background_at(&list).expect("the dialog");
    let sprite = list
        .iter()
        .position(|command| matches!(command, DrawCommand::Sprite { .. }))
        .expect("the scene's sprites");
    assert!(sprite < background, "{sprite} {background}");
    // The frame shut, less the cursor, then the dialog.
    let scene: Vec<&DrawCommand> = list.iter().take(background - 1).collect();
    let before: Vec<&DrawCommand> = shut.iter().take(scene.len()).collect();
    assert_eq!(scene, before);
}

#[test]
fn in_port_the_map_is_drawn_alone() {
    let mut view = in_the_bar(None);
    view.open_bar_map();
    let list = drawn(&view);
    assert_eq!(background_at(&list), Some(1), "the dialog's black, then it");
}

// Clear Route, C and the open latch (`_DoSystemMap` item 8,
// @0x12c69-0x12dbf; `showClearRouteButton`, @0x12396-0x123c3).

/// Where Clear Route is on the screen.
fn clear_route_button() -> Point {
    crate::galaxy::buttons::MapButtons::new(crate::galaxy::dialog::DIALOG_ORIGIN)
        .rect(crate::galaxy::buttons::MapButton::ClearRoute)
        .center()
}

#[test]
fn the_map_opens_with_clear_route_enabled_by_a_course_and_never_on_the_hypergate_map() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert!(!clear_route_enabled(&view), "no course");
    tap(&mut view, MAP);
    plot(&mut view, 131);
    tap(&mut view, MAP);
    assert!(clear_route_enabled(&view), "a course");
    let mut view = FlightView::new(gated());
    view.session
        .as_mut()
        .expect("flying")
        .plot_course(SystemId(131))
        .expect("a route");
    land_now(&mut view);
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert!(!clear_route_enabled(&view), "the hypergate map");
}

#[test]
fn clear_route_by_its_button_clears_the_course_lets_go_and_redraws_the_nav() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, MAP);
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_UNEXPLORED]);
    click(&mut view, clear_route_button());
    assert!(view.map_open());
    assert_eq!(course(&view), []);
    assert_eq!(view.course_map().route(), []);
    assert_eq!(
        nav_mode(&view),
        NavMode::Hyperspace(None),
        "let go, mode kept"
    );
    assert_eq!(nav(&view), [hud::NAV_HYPERSPACE, hud::NAV_NO_DESTINATION]);
    assert!(!clear_route_enabled(&view), "disabled after");
    tap(&mut view, MAP);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None), "nothing to cue");
}

#[test]
fn c_clears_the_route_whether_clear_route_is_enabled_or_not() {
    let mut view = flight();
    plot(&mut view, 131);
    tap(&mut view, MAP);
    tap(&mut view, Key::Char('c'));
    assert_eq!(course(&view), []);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
    assert!(!clear_route_enabled(&view));
    // Clear Route is disabled now, and C still clears.
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    tap(&mut view, Key::Char('c'));
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None), "C again");
    assert!(view.map_open(), "C doesn't close the map");
}

#[test]
fn c_on_the_hypergate_map_drops_the_pick_so_the_close_cancels() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    tap(&mut view, Key::Char('c'));
    tap(&mut view, Key::Enter);
    assert!(!view.map_open());
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(system_of(&view), SystemId(130));
}

#[test]
fn a_click_on_the_hypergate_maps_current_system_drops_the_pick_so_the_close_cancels() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    let sol = on_map(&mut view, 130);
    click(&mut view, sol);
    assert_eq!(view.course_map().selected(), Some(SystemId(130)));
    tap(&mut view, Key::Enter);
    assert_eq!(view.message(), Some(HYPERGATE_CANCELLED));
    assert_eq!(system_of(&view), SystemId(130));
}

#[test]
fn shift_on_the_hypergate_map_is_a_plain_click() {
    let mut view = FlightView::new(gated());
    land_now(&mut view);
    let alpha = on_map(&mut view, 131);
    shift_click(&mut view, alpha);
    assert_eq!(course(&view), [], "no hop written");
    tap(&mut view, Key::Enter);
    assert_eq!(system_of(&view), SystemId(131), "entered");
}

#[test]
fn a_shift_click_writing_a_hop_enables_clear_route() {
    let mut view = flight();
    tap(&mut view, MAP);
    assert!(!clear_route_enabled(&view));
    let alpha = on_map(&mut view, 131);
    click(&mut view, alpha);
    assert!(!clear_route_enabled(&view), "a plain click writes no hop");
    shift_click(&mut view, alpha);
    assert!(clear_route_enabled(&view));
    assert_eq!(view.course_map().route(), [SystemId(131)], "shown");
}
