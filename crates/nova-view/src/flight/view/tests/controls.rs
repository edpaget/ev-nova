use super::*;
use nova_sim::DrawnShips as _;

// Double time.

/// Whether `view` draws the double-time icon, `cïcn` 20000 at the top
/// left.
fn double_time_drawn(view: &View) -> bool {
    drawn(view).iter().any(|command| {
        *command
            == DrawCommand::Picture {
                image: ImageKey::icon(20000),
                top_left: Point::new(0.0, 0.0),
            }
    })
}

#[test]
fn double_time_shows_its_icon_at_the_top_left_over_the_red_alert() {
    // Whether double time is on, and the alert, are the frame's
    // (`a_fill_writes_the_murk_the_background_double_time_the_gamma_and_the_traffic`,
    // `a_fill_writes_the_red_alert_icon_shown`).
    let mut frame = super::faked_hud::among(Vec::new());
    frame.double_time = true;
    frame.hud.red_alert = Some(nova_sim::AlertIcon::Red);
    let mut view = super::faked_world::world(catalog(), frame);
    view.alert_width = Some(24);
    let list = super::faked_world::world_list(&view);
    let commands: Vec<_> = list.iter().collect();
    let icon = commands
        .iter()
        .position(|command| icon_in(20000..=20000)(command))
        .expect("the icon is drawn");
    assert_eq!(
        *commands[icon],
        DrawCommand::Picture {
            image: ImageKey::icon(20000),
            top_left: Point::new(0.0, 0.0),
        }
    );
    assert!(
        icon_in(18000..=18001)(commands[icon - 1]),
        "just after the Red Alert's: {:?}",
        commands[icon - 1]
    );
    super::faked_hud::changes(&mut view, |frame| frame.double_time = false);
    assert!(!faked_double_time_drawn(&view), "off at once");
}

#[test]
fn double_time_reaches_the_session_on_the_next_tick_and_when_set() {
    let mut view = flight();
    view.set_double_time(true);
    assert!(view.session().expect("flying").double_time(), "when set");
    view.sim
        .session
        .as_mut()
        .expect("flying")
        .set_double_time_key(false);
    view.tick(TICK);
    assert!(view.session().expect("flying").double_time(), "each tick");
}

#[test]
fn double_time_survives_the_events_that_let_go_of_the_flight_keys() {
    let mut view = flight().with_double_time(true);
    tap(&mut view, MAP);
    tap(&mut view, MAP);
    view.tick(TICK);
    assert!(double_time_drawn(&view), "the map");
    plot(&mut view, 131);
    fly_out(&mut view);
    jump_now(&mut view);
    arrive_now(&mut view);
    view.tick(TICK);
    assert!(double_time_drawn(&view), "a jump");
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    let mut hailed = hailable(true).with_double_time(true);
    tap(&mut hailed, HAIL_KEY);
    assert!(hailed.take_hail().is_some());
    hailed.tick(TICK);
    assert!(double_time_drawn(&hailed), "a hail");
}

/// [`beside`] (intact) with the player holding 95 of fuel and NPC 0
/// refuelling it, hailed and hung up.
fn refuelling() -> View {
    let catalog = FakeCatalog {
        strings: vec![(3000, comm_strings())],
        ..boardable()
    };
    let session = SessionBuilder::flying(&catalog)
        .named("Low")
        .with_reserve(nova_sim::Reserve::Fuel, 95.0)
        .build();
    let (_, chance) = scripted(&placed(750, 750, 0));
    let mut view = FlightView::from_session(catalog, session.into())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_hail_options(HailOptions::empty().with(Rc::new(Refuel)));
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, HAIL_KEY);
    view.take_hail().expect("hailed");
    view.answer(0).expect("open");
    view.hang_up();
    view
}

#[test]
fn double_times_second_pass_gives_no_help_and_tells_no_comm() {
    for on in [false, true] {
        let mut view = refuelling().with_double_time(on);
        let mut ticks = 0;
        while view.message().is_none() {
            assert!(ticks < 20, "never refuelled: {on}");
            view.tick(TICK);
            ticks += 1;
        }
        // A tick of help on the advance pass alone: 95 to past 100,
        // one at a time, and done on the think after (`_HighLevelAIHandler`
        // tests done before the add, @0x8f548).
        assert_eq!(ticks, 7, "{on}");
        assert_eq!(reserves(&view).fuel.now, 101.0, "{on}");
        let note = view.message().expect("refuelled").to_owned();
        assert!(note.contains(":  Energy transfer complete, "), "{note}");
    }
}

#[test]
fn caps_lock_is_no_flight_key() {
    let mut view = super::faked_flight::at_rest(catalog());
    super::faked_flight::hold(&mut view, Key::CapsLock, true);
    assert!(view.held.is_empty());
    super::faked_flight::ticked(&mut view, 1);
    assert_eq!(
        super::faked_flight::last_controls(&view),
        Controls::default()
    );
    assert!(!view.double_time, "the app toggles it, not flight");
}

// The hover cursor (`_HandleMouse`).

/// The cursor sprites in `list`: frame and centre.
fn cursor_sprites(list: &DrawList) -> Vec<(u16, Point)> {
    sprites(list)
        .into_iter()
        .filter(|(image, _)| image.id == 651)
        .map(|(image, at)| (image.frame, at))
        .collect()
}

/// A point on screen over nothing at all.
const NOWHERE: Point = Point::new(40.0, 400.0);

#[test]
fn a_left_click_on_a_drawn_ship_targets_it_with_the_target_beep() {
    // What the click targets, and its beep, are nova-sim's
    // (`a_click_in_space_on_a_ship_drawn_answers_the_ship_it_targets`).
    let (mut view, id) = faked_with_npc();
    let over = npc_on_screen(&view, id);
    assert!(over.x < VIEW_SIZE.0 - STATUS_BAR_WIDTH, "{over:?} in space");
    view.click_in_space(over);
    let [FlightCommand::ClickInSpace { at, drawn }] = sent(&view) else {
        panic!("{:?}", sent(&view));
    };
    let world = view.camera().screen_to_world(over);
    assert_eq!(*at, Vec2::new(world.x, world.y));
    assert!(drawn.npcs.iter().any(|&(npc, _)| npc == id), "{drawn:?}");
}

#[test]
fn a_click_on_the_players_own_ship_lets_go_of_the_target_with_the_clear_beep() {
    // What a click on the player's ship does is nova-sim's
    // (`a_click_in_space_on_the_players_ship_lets_go_of_the_target`).
    let (mut view, _) = faked_with_npc();
    // The camera centres on the player, so its ship is drawn at the centre.
    view.click_in_space(PLAYER_AT);
    let [FlightCommand::ClickInSpace { at, drawn }] = sent(&view) else {
        panic!("{:?}", sent(&view));
    };
    assert_eq!(*at, Vec2::ZERO, "the player's ship's centre");
    assert!(drawn.player.is_some(), "boxed");
}

#[test]
fn a_click_on_the_players_own_ship_with_no_target_is_silent() {
    // A click that changes nothing is refused (`check_contract`'s click
    // clause; `a_click_in_space_on_nothing_is_refused_and_changes_nothing`):
    // no sound is drained for it.
    let (mut view, _) = faked_with_npc();
    view.click_in_space(PLAYER_AT);
    assert_eq!(sent(&view).len(), 1);
    assert_eq!(view.drained_sounds(), []);
}

#[test]
fn a_click_on_a_ship_targets_it_during_a_jump() {
    // A still NPC at (0, -750), where the ship flies out past.
    let mut catalog = trafficked(&[130], 1, 129, 3);
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.fields.speed = 0;
        }
    }
    let (_, chance) = scripted(&placed(750, 0, 0));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let id = npc_ids(&view)[0];
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.session().expect("flying").preparing_jump().is_some());
    let over = npc_on_screen(&view, id);
    assert!(
        (0.0..VIEW_SIZE.0 - STATUS_BAR_WIDTH).contains(&over.x)
            && (0.0..VIEW_SIZE.1).contains(&over.y),
        "{over:?} in space"
    );
    view.take_sounds();
    click(&mut view, over);
    assert_eq!(
        view.session().expect("flying").target().map(|npc| npc.id),
        Some(id)
    );
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
}

#[test]
fn the_cursor_animates_over_an_npc_and_rests_off_it() {
    // What the pointer is over is the flight's answer (`FlightSim::hover`).
    let (mut view, id) = faked_with_npc();
    let over = npc_on_screen(&view, id);
    view.sim.set_hover(Some(nova_sim::Hover::Ship(id)));
    view.cursor_input(&Input::PointerMoved(over));
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(0, over)]
    );
    super::faked_flight::ticked(&mut view, 2);
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(1, over)]
    );
    view.sim.set_hover(None);
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    super::faked_flight::ticked(&mut view, 1);
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(0, NOWHERE)]
    );
    super::faked_flight::ticked(&mut view, 4);
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(0, NOWHERE)],
        "rests"
    );
}

#[test]
fn a_button_press_shows_the_cursor_where_it_is() {
    let mut view = super::faked_flight::at_rest(catalog());
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [],
        "no pointer yet"
    );
    view.cursor_input(&Input::PointerButton {
        button: MouseButton::Right,
        pressed: true,
        at: NOWHERE,
    });
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(0, NOWHERE)]
    );
}

#[test]
fn the_cursor_draws_in_the_world_under_the_hud_and_the_jump_fade() {
    let cursor_at = |list: &DrawList| {
        list.iter()
            .position(
                |command| matches!(unclipped(command), DrawCommand::Sprite { image, .. } if image.id == 651),
            )
            .expect("drawn")
    };
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    let list = drawn(&view);
    let cursor = cursor_at(&list);
    // A sprite of the target layer, clipped to the play area, so the
    // status bar is never under it.
    assert!(cursor < hud_at(&list), "under the HUD");

    let mut view = fading_in_on_course_to_barnard();
    view.input(&Input::PointerMoved(NOWHERE));
    let list = drawn(&view);
    let (at_fade, _) = fade(&list).expect("fading in");
    assert!(cursor_at(&list) < at_fade, "under the fade");
}

#[test]
fn a_cloaked_npc_is_hovered_only_with_a_scanner_that_targets_cloaked_ships() {
    // Whether a cloaked ship is hovered is nova-sim's
    // (`hover_skips_a_cloaked_ship_without_the_cloak_bit_and_takes_it_with_it`,
    // `hover_skips_an_untargetable_ship_without_the_scanner_and_takes_it_with_it`);
    // the cursor animates as the flight answers.
    for (hovered, frame) in [(false, 0), (true, 1)] {
        let (mut view, id) = faked_with_npc();
        let over = npc_on_screen(&view, id);
        view.sim
            .set_hover(hovered.then_some(nova_sim::Hover::Ship(id)));
        view.cursor_input(&Input::PointerMoved(over));
        super::faked_flight::ticked(&mut view, 2);
        let frames: Vec<u16> = cursor_sprites(&super::faked_world::world_list(&view))
            .into_iter()
            .map(|(frame, _)| frame)
            .collect();
        assert_eq!(frames, [frame], "hovered: {hovered}");
    }
}

#[test]
fn the_players_own_ship_is_hovered_only_while_it_holds_a_target() {
    // When the player's own ship is hovered is nova-sim's
    // (`nova_sim_ai`'s `the_players_own_ship_is_hovered_only_while_it_holds_a_target`).
    let (mut view, _) = faked_with_npc();
    view.cursor_input(&Input::PointerMoved(PLAYER_AT));
    super::faked_flight::ticked(&mut view, 2);
    let (at, boxes) = view.sim.hovers().pop().expect("asked");
    assert_eq!(at, Vec2::ZERO, "asked at the player's ship");
    assert!(boxes.player.is_some());
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(0, PLAYER_AT)],
        "no target"
    );
    view.sim.set_hover(Some(nova_sim::Hover::Player));
    super::faked_flight::ticked(&mut view, 2);
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(1, PLAYER_AT)],
        "a target"
    );
}

#[test]
fn an_npc_of_a_cost_minus_one_type_is_not_drawn_but_is_still_hovered() {
    // That a ship of a cost -1 type is hidden is nova-sim's
    // (`a_hidden_ship_is_framed_by_its_berths_last_shown_type`).
    let (mut view, id) = faked_with_npc();
    let over = npc_on_screen(&view, id);
    let near = |list: &DrawList| {
        list.iter().any(|command| match *unclipped(command) {
            DrawCommand::Sprite { center, image, .. } => {
                image.id != 651
                    && (center.x - over.x).abs() < 0.01
                    && (center.y - over.y).abs() < 0.01
            }
            _ => false,
        })
    };
    assert!(
        near(&super::faked_world::world_list(&view)),
        "drawn while shown"
    );
    // A hidden ship's slot draws nothing: the frame draws it in no layer.
    super::faked_hud::changes(&mut view, |frame| {
        frame.drawn.retain(|&(ship, _)| ship != ShipRef::Npc(id));
    });
    assert!(
        !near(&super::faked_world::world_list(&view)),
        "not drawn hidden"
    );
    view.sim.set_hover(Some(nova_sim::Hover::Ship(id)));
    view.cursor_input(&Input::PointerMoved(over));
    super::faked_flight::ticked(&mut view, 2);
    assert_eq!(
        cursor_sprites(&super::faked_world::world_list(&view)),
        [(1, over)],
        "animates over where it would be drawn"
    );
    let (_, boxes) = view.sim.hovers().pop().expect("asked");
    assert!(boxes.npcs.iter().any(|&(npc, _)| npc == id), "boxed");
}

#[test]
fn a_hidden_ship_is_hovered_by_the_box_its_berth_last_drew() {
    // Which frames box a hidden ship is nova-sim's, by its rule
    // (`a_hidden_ship_is_framed_by_its_berths_last_shown_type`,
    // `the_other_hover_box_reading_frames_a_hidden_ship_by_its_own_type`):
    // the view boxes it by the frames its record names.
    for (framed, width) in [(131, 100), (129, 40)] {
        let (mut view, id) = faked_with_npc();
        let mut frame = view.frame.clone();
        frame.drawn.retain(|&(ship, _)| ship != ShipRef::Npc(id));
        frame.ships[1].framed = Some(ShipId(framed));
        super::faked_hud::shows(&mut view, frame);
        let boxed = view.drawn_boxes().npc(id).expect("boxed");
        assert_eq!((boxed.width, boxed.height), (width, width), "{framed}");
    }
}

#[test]
fn a_hidden_player_is_hovered_by_the_box_its_slot_last_drew() {
    // Which frames box a hidden player is nova-sim's
    // (`a_hidden_player_is_framed_by_its_last_shown_ship`,
    // `the_other_hover_box_reading_frames_a_hidden_ship_by_its_own_type`).
    for (framed, width) in [(131, 100), (128, 40)] {
        let (mut view, _) = faked_with_npc();
        let mut frame = view.frame.clone();
        frame.ships[0].shown = nova_sim::player_fate::Drawn::Hidden;
        frame.ships[0].framed = Some(ShipId(framed));
        super::faked_hud::shows(&mut view, frame);
        let boxed = view.drawn_boxes().player.expect("boxed");
        assert_eq!((boxed.width, boxed.height), (width, width), "{framed}");
    }
}

#[test]
fn an_undisplayed_ship_is_hovered_and_clicked_at_its_slots_last_place_not_its_own() {
    // `_HandleMouse` and the click block box a slot's hull sprite rect
    // (@0x38177, `_HandlePlayer` @0x6a4ab-0x6a4f4), which only a display
    // places (@0x2cd9d): where its last ship was last laid out. Where
    // that is, by the rule, is nova-sim's
    // (`a_ship_its_slot_has_not_displayed_is_hovered_where_the_slot_last_was`).
    let (mut view, id) = faked_with_npc();
    let was = npc_on_screen(&view, id);
    let place = view.frame.ships[1].position;
    super::faked_hud::changes(&mut view, |frame| {
        frame.ships[1].position = place + Vec2::new(200.0, 0.0);
        frame.ships[1].hover = nova_sim::HoverPlace::Stale(place);
    });
    let boxed = view.drawn_boxes().npc(id).expect("boxed");
    let boxed = view
        .camera()
        .world_to_screen(Point::new(boxed.centre.x, boxed.centre.y));
    assert!(
        (boxed.x - was.x).abs() <= 1.0 && (boxed.y - was.y).abs() <= 1.0,
        "{boxed:?} at {was:?}"
    );
    view.click_in_space(was);
    let [FlightCommand::ClickInSpace { drawn, .. }] = sent(&view) else {
        panic!("{:?}", sent(&view));
    };
    assert!(drawn.npcs.iter().any(|&(npc, _)| npc == id), "the box sent");
}

#[test]
fn by_the_other_reading_an_undisplayed_ship_is_hovered_and_clicked_where_it_is() {
    // (`by_the_other_reading_an_undisplayed_ship_is_hovered_where_it_is`.)
    let (mut view, id) = faked_with_npc();
    let place = view.frame.ships[1].position;
    super::faked_hud::changes(&mut view, |frame| {
        frame.ships[1].position = place + Vec2::new(200.0, 0.0);
        frame.ships[1].hover = nova_sim::HoverPlace::Own;
    });
    let now = npc_on_screen(&view, id);
    let boxed = view.drawn_boxes().npc(id).expect("boxed");
    assert_eq!(
        view.camera()
            .world_to_screen(Point::new(boxed.centre.x, boxed.centre.y)),
        now
    );
}

#[test]
fn a_flight_hands_its_stellar_sprite_pool_to_its_session_and_back() {
    use nova_sim::story::sprite_pool::{SlotSprite, StellarSpritePool};
    let mut pool = StellarSpritePool::default();
    pool.rebuild((2000..2016).map(nova_sim::StellarId));
    pool.place(nova_sim::StellarId(2015), (-7, 9));
    pool.rebuild([]);
    let view = flight().with_stellar_sprite_pool(pool);
    let ghost = SlotSprite::Ghost { top_left: (-7, 9) };
    let carried = view.stellar_sprite_pool().expect("flying");
    assert_eq!(carried.sprite_of(nova_sim::StellarId(2015), false), ghost);
    assert_eq!(
        view.session()
            .expect("flying")
            .stellar_sprite_pool()
            .sprite_of(nova_sim::StellarId(2015), false),
        ghost
    );
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    })
    .with_stellar_sprite_pool(StellarSpritePool::default());
    assert_eq!(failed.stellar_sprite_pool(), None);
}

#[test]
fn a_flight_hands_its_kept_berths_to_its_session_and_back() {
    let mut traffic = nova_sim::Traffic::new();
    traffic.set_berth_person(2, "Ace");
    let kept = traffic.kept_berths();
    let view = flight().with_kept_berths(kept.clone());
    assert_eq!(view.session().expect("flying").kept_berths(), kept);
    assert_eq!(view.kept_berths(), Some(kept.clone()));
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    })
    .with_kept_berths(kept);
    assert_eq!(failed.kept_berths(), None);
}

/// A new pilot of [`catalog`] begun, with `carried` handed on, if any:
/// whether it explored Alpha Centauri (131), Sol's neighbour.
fn begun_explores_131(carried: Option<nova_sim::LastRefresh>) -> bool {
    let catalog = catalog();
    let view = flying_as(catalog, "Ada");
    let mut view = match carried {
        Some(carried) => view.with_last_refresh(carried),
        None => view,
    };
    view.begin();
    view.session()
        .expect("flying")
        .pilot()
        .has_explored(SystemId(131))
}

#[test]
fn a_flight_hands_the_last_refresh_to_its_new_pilots_neighbour_pass_and_back() {
    let hiding = nova_sim::LastRefresh::hiding([SystemId(131)]);
    assert!(begun_explores_131(None), "the launch's");
    assert!(!begun_explores_131(Some(hiding.clone())), "the one carried");
    // 131 hidden until the `OnStart` sets bit 50.
    let mut shown_on_start = starting_with("b50");
    shown_on_start
        .visibility
        .push(nova_sim::catalog::SystemVisibility {
            id: SystemId(131),
            visibility: nova_sim::control::Test::parse("b50"),
        });
    let mut view = flying_as(shown_on_start, "Ada");
    view.begin();
    assert_eq!(view.last_refresh(), Some(hiding.clone()), "before OnStart");
    view.enter_game();
    assert_eq!(view.last_refresh(), Some(nova_sim::LastRefresh::default()));
    let mut failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    })
    .with_last_refresh(hiding);
    failed.enter_game();
    assert_eq!(failed.last_refresh(), None);
}

#[test]
fn a_flight_hands_its_slot_sprites_to_its_session_and_back() {
    let mut sprites = SlotSprites::default();
    sprites.layers.put(
        nova_sim::ship_layer::Slot::Berth(2),
        nova_sim::ship_layer::ShipLayer::Led,
    );
    sprites
        .frames
        .clone_frames(nova_sim::ship_layer::Slot::Berth(2), ShipId(140));
    let view = flight().with_slot_sprites(sprites.clone());
    assert_eq!(view.session().expect("flying").slot_sprites(), &sprites);
    assert_eq!(view.slot_sprites(), Some(&sprites));
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    })
    .with_slot_sprites(sprites);
    assert_eq!(failed.slot_sprites(), None);
}

#[test]
fn cancelling_the_pointer_forgets_the_cursor_until_the_next_move() {
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
    assert_eq!(view.cursor_at(), Some(NOWHERE));
    view.cancel_pointer();
    assert_eq!(cursor_sprites(&drawn(&view)), [], "not at the old point");
    assert_eq!(view.cursor_at(), None);
    let other = Point::new(60.0, 420.0);
    view.input(&Input::PointerMoved(other));
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, other)]);
    assert_eq!(view.cursor_at(), Some(other));
}

#[test]
fn the_cursor_is_hidden_once_the_pointer_leaves_the_window() {
    let mut view = super::faked_flight::at_rest(catalog());
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    view.cursor_input(&Input::PointerLeft);
    assert_eq!(cursor_sprites(&super::faked_world::world_list(&view)), []);
}

#[test]
fn a_key_pressed_in_flight_fades_the_cursor_at_the_next_step() {
    let mut view = super::faked_flight::at_rest(catalog());
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    super::faked_flight::hold(&mut view, Key::Char('q'), true);
    super::faked_flight::ticked(&mut view, 1);
    assert_eq!(faked_cursor_alpha(&view), Some(247), "31 of 32");
    super::faked_flight::hold(&mut view, Key::Up, true);
    super::faked_flight::ticked(&mut view, 2);
    assert_eq!(faked_cursor_alpha(&view), Some(231), "held: 29 of 32");
}

#[test]
fn z_held_fades_the_cursor_as_a_flight_key_with_no_afterburner_carried() {
    assert_eq!(
        faked_alpha_holding(AFTERBURNER_KEY),
        Some(239),
        "held: 30 of 32"
    );
    assert_ne!(
        faked_alpha_holding(Key::Char('q')),
        Some(239),
        "a key not held"
    );
    assert_eq!(
        faked_alpha_holding(SELECT_KEY),
        Some(239),
        "W: every pass, @0x6ac2a"
    );
    assert_eq!(
        faked_alpha_holding(DESELECT_KEY),
        faked_alpha_holding(Key::Char('q')),
        "S with none selected: only its press, @0x6ade1"
    );
}

#[test]
fn b_held_fades_the_cursor_as_a_flight_key() {
    // `_HandlePlayer` sets the cursor's fade every frame B is down
    // (@0x6a98b-0x6a999), a target or not.
    assert_eq!(faked_alpha_holding(BOARD_KEY), Some(239), "held: 30 of 32");
    assert_ne!(
        faked_alpha_holding(Key::Char('q')),
        Some(239),
        "a key not held"
    );
}

#[test]
fn the_cursor_is_clipped_to_the_play_area_short_of_the_status_bar() {
    let mut view = super::faked_flight::at_rest(catalog());
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    let list = super::faked_world::world_list(&view);
    let clips: Vec<Bounds> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { clip, command } if sprite_of(651)(command) => Some(*clip),
            _ => None,
        })
        .collect();
    // `_SetupSpriteWorld`'s sprite world: the window less 194 at the
    // right (`subw $0xc2` @0x21b66).
    assert_eq!(
        clips,
        [Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(VIEW_SIZE.0 - STATUS_BAR_WIDTH, VIEW_SIZE.1),
        }]
    );
}

#[test]
fn the_cursor_is_not_drawn_while_the_map_is_open() {
    let mut view = super::faked_flight::at_rest(catalog());
    view.sim.map_mut().drawing = Some(sol_drawing());
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    view.open_map();
    assert!(view.map_open);
    assert_eq!(cursor_sprites(&super::faked_world::world_list(&view)), []);
}

#[test]
fn flight_hides_the_system_pointer_unless_the_map_is_open_or_it_has_no_cursor() {
    let mut view = flight();
    assert!(view.hides_pointer());
    view.input(&key(MAP_KEY, true));
    assert!(!view.hides_pointer(), "the map");
    let no_cursor = FakeCatalog {
        cursor_sheet: Err("no spïn 650".to_owned()),
        ..catalog()
    };
    assert!(!FlightView::new(no_cursor).hides_pointer());
    let failed = FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    };
    assert!(!FlightView::new(failed).hides_pointer());
}

// Keys held across a jump's end, an Option release or a repair
// (`_HandlePlayer`'s latches 0xdee04, 0xdee48 and 0xdee44).

/// A flight on to Barnard's Star ([`FakeCatalog::onward`]) with a
/// course plotted to `to` and flown out, 300 ms into the jump's
/// streak, its sounds let go. The planner plots it, as Barnard's Star
/// is unknown past the unexplored Alpha Centauri.
fn streaking_to(to: i16) -> View {
    let mut view = planner(FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    }));
    plot(&mut view, to);
    fly_out(&mut view);
    jump_now(&mut view);
    view.tick(ms(300));
    assert!(view.jump_effect().is_some());
    view.take_sounds();
    view
}

/// Ticks `view` through its arrival tick, asserting that no `sound`
/// was raised before or on it, then once more, giving how many times
/// that tick raised `sound`; then once more again, asserting it is not
/// raised a second time.
fn after_arrival(view: &mut View, sound: SimSound) -> (NavMode, usize, NavMode) {
    tick_to_arrival(view);
    assert_eq!(count(view, sound), 0, "not through the jump nor on arrival");
    let arrived = nav_mode(view);
    view.tick(TICK);
    let acted = count(view, sound);
    let after = nav_mode(view);
    view.tick(TICK);
    assert_eq!(count(view, sound), 0, "once only");
    assert_eq!(nav_mode(view), after, "nothing more");
    (arrived, acted, after)
}

#[test]
fn h_pressed_before_j_and_held_through_the_jump_acts_the_tick_after_arrival() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    jump_now(&mut view);
    view.take_sounds();
    let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
    assert_eq!(arrived, NavMode::Off, "the course flown");
    assert_eq!(acted, 1);
    assert_eq!(after, NavMode::Hyperspace(None));
    assert_eq!(floating_alpha(&view), Some(255), "the map up");
}

#[test]
fn h_held_through_a_jump_that_cues_the_next_hop_only_beeps() {
    let mut view = streaking_to(132);
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
    assert_eq!(arrived, NavMode::Hyperspace(Some(SystemId(132))));
    assert_eq!((acted, after), (1, arrived), "the selection kept");
}

#[test]
fn h_first_pressed_during_the_jump_and_held_acts_after_arrival() {
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        view.input(&key(HYPERSPACE_MODE_KEY, true));
        view.take_sounds();
        let (_, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
        assert_eq!((acted, after), (1, NavMode::Hyperspace(None)), "{stage}");
    }
}

#[test]
fn h_pressed_during_the_jump_and_let_go_before_arrival_does_nothing_after() {
    let mut view = streaking_to(131);
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    let (_, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
    assert_eq!((acted, after), (0, NavMode::Off));
}

#[test]
fn nav_off_held_through_the_jump_turns_the_nav_off_after_arrival() {
    let mut view = streaking_to(132);
    view.input(&key(NAV_OFF_KEY, true));
    let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
    assert_eq!(arrived, NavMode::Hyperspace(Some(SystemId(132))));
    assert_eq!((acted, after), (1, NavMode::Off));
}

#[test]
fn option_backquote_turns_into_nav_off_when_option_is_let_go() {
    // That the nav block's press turns the nav off is nova-sim's
    // (`the_nav_block_turns_the_nav_off_or_to_hyperspace`).
    let mut view = faked(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        ..in_sol()
    });
    view.track_held(&key(Key::Alt, true));
    faked_press(&mut view, NAV_OFF_KEY);
    for _ in 0..3 {
        view.nav_held_frame();
    }
    assert_eq!(block_presses(&view), 0, "Option held");
    view.track_held(&key(Key::Alt, false));
    assert_eq!(block_presses(&view), 0, "at the next frame");
    view.nav_held_frame();
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::NavModeKeys {
            nav_off: true,
            h: false
        })
    );
    view.nav_held_frame();
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 1, "once");
}

#[test]
fn h_held_blocks_a_press_of_nav_off() {
    let mut view = faked(in_sol());
    faked_press(&mut view, HYPERSPACE_MODE_KEY);
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 1, "H's");
    faked_press(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 1, "` blocked");
}

#[test]
fn nav_off_held_blocks_a_press_of_h_which_still_shows_the_map() {
    let mut view = faked_sol(in_sol());
    faked_press(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 1, "`'s");
    faked_press(&mut view, HYPERSPACE_MODE_KEY);
    assert!(view.floating.is_up(), "on the press's own frame");
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 1, "H blocked");
}

#[test]
fn a_key_of_the_block_let_go_frees_the_other_held_to_press_again() {
    let mut view = faked(in_sol());
    faked_press(&mut view, HYPERSPACE_MODE_KEY);
    faked_press(&mut view, NAV_OFF_KEY);
    faked_release(&mut view, HYPERSPACE_MODE_KEY);
    view.nav_held_frame();
    let before = block_presses(&view);
    faked_release(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    faked_press(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(block_presses(&view) - before, 1, "both up a frame");
}

#[test]
fn each_fresh_press_of_h_or_nav_off_acts_with_no_tick_between() {
    for k in [HYPERSPACE_MODE_KEY, NAV_OFF_KEY] {
        let mut view = faked(in_sol());
        for _ in 0..2 {
            faked_press(&mut view, k);
            faked_release(&mut view, k);
        }
        assert_eq!(block_presses(&view), 2, "{k:?}");
    }
}

#[test]
fn h_and_nav_off_held_through_the_jump_act_once_together_after_arrival() {
    let mut view = streaking_to(131);
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.input(&key(NAV_OFF_KEY, true));
    let (arrived, acted, after) = after_arrival(&mut view, SimSound::NavSelected);
    assert_eq!(arrived, NavMode::Off);
    assert_eq!((acted, after), (1, NavMode::Hyperspace(None)));
}

#[test]
fn option_backquote_held_lets_go_of_a_target_picked_the_next_tick() {
    // Whether a target is let go is nova-sim's
    // (`option_backquote_lets_go_of_the_target`).
    let mut view = faked(in_sol());
    view.track_held(&key(Key::Alt, true));
    faked_press(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    faked_press(&mut view, NEAREST_KEY);
    faked_release(&mut view, NEAREST_KEY);
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::SelectTarget(nova_sim::TargetPick::Nearest)),
        "Option-R picks"
    );
    view.nav_held_frame();
    assert_eq!(sent(&view).last(), Some(&FlightCommand::ClearTarget));
}

#[test]
fn another_key_pressed_while_option_backquote_is_held_leaves_the_target_to_the_tick() {
    let mut view = faked(in_sol());
    view.track_held(&key(Key::Alt, true));
    faked_press(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    faked_press(&mut view, NEAREST_KEY);
    faked_release(&mut view, NEAREST_KEY);
    let picked = sent(&view).len();
    faked_press(&mut view, Key::Char('q'));
    faked_release(&mut view, Key::Char('q'));
    assert_eq!(sent(&view).len(), picked, "no press of the block");
}

#[test]
fn option_backquote_held_blocks_h_but_not_its_map_nor_with_h_held_its_target() {
    let mut view = faked_sol(in_sol());
    view.track_held(&key(Key::Alt, true));
    faked_press(&mut view, NAV_OFF_KEY);
    view.nav_held_frame();
    faked_press(&mut view, HYPERSPACE_MODE_KEY);
    assert!(view.floating.is_up(), "the map shown");
    view.nav_held_frame();
    assert_eq!(block_presses(&view), 0, "H blocked");
    faked_press(&mut view, NEAREST_KEY);
    faked_release(&mut view, NEAREST_KEY);
    let picked = sent(&view).len();
    view.nav_held_frame();
    assert_eq!(sent(&view).len(), picked, "the latch held by H");
}

/// The system Hyper Select picks first from Alpha Centauri (131), the
/// first of its links.
fn first_link_of_131() -> NavMode {
    let mut view = flight();
    run_set(&mut view, "M131");
    view.settle_script();
    tap(&mut view, HYPER_SELECT_KEY);
    nav_mode(&view)
}

#[test]
fn backslash_first_pressed_during_the_jump_and_held_acts_after_arrival() {
    let first = first_link_of_131();
    assert!(matches!(first, NavMode::Hyperspace(Some(_))));
    for mut view in through_a_jump(flight) {
        let stage = stage(&view);
        view.input(&key(HYPER_SELECT_KEY, true));
        view.take_sounds();
        let (_, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
        assert_eq!((acted, after), (1, first), "{stage}");
        assert_eq!(floating_alpha(&view), Some(255), "{stage}: the map up");
    }
}

#[test]
fn backslash_pressed_before_j_and_held_through_does_nothing_after() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(HYPER_SELECT_KEY, true));
    assert_eq!(count(&mut view, SimSound::HyperSelected), 1);
    jump_now(&mut view);
    view.take_sounds();
    let (arrived, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
    assert_eq!((acted, after), (0, arrived));
}

#[test]
fn backslash_let_go_and_pressed_again_during_the_jump_acts_after_arrival() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(HYPER_SELECT_KEY, true));
    jump_now(&mut view);
    view.input(&key(HYPER_SELECT_KEY, false));
    view.tick(TICK);
    view.input(&key(HYPER_SELECT_KEY, true));
    view.take_sounds();
    let (_, acted, after) = after_arrival(&mut view, SimSound::HyperSelected);
    assert_eq!(acted, 1);
    assert!(matches!(after, NavMode::Hyperspace(Some(_))));
}

/// Ticks `view` once with the player held disabled.
fn tick_disabled(view: &mut View) {
    condition(view, Condition::Disabled);
    view.tick(TICK);
}

#[test]
fn j_held_into_a_disable_acts_again_on_the_first_intact_tick() {
    let mut view = new_pilot(flight());
    view.input(&key(JUMP, true));
    assert_eq!(view.take_sounds(), [REFUSED]);
    assert_eq!(view.message(), Some(NO_DESTINATION));
    view.tick(TICK);
    assert!(!view.take_sounds().contains(&REFUSED), "held: no repeat");
    tick_disabled(&mut view);
    view.message = None;
    condition(&mut view, Condition::Intact);
    view.tick(TICK);
    assert_eq!(
        view.take_sounds()
            .iter()
            .filter(|&&sound| sound == REFUSED)
            .count(),
        1
    );
    assert_eq!(view.message(), Some(NO_DESTINATION));
    ticks(&mut view, 2);
    assert!(!view.take_sounds().contains(&REFUSED), "once");
}

#[test]
fn j_first_pressed_while_disabled_and_held_jumps_on_the_first_intact_tick() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    condition(&mut view, Condition::Disabled);
    view.input(&key(JUMP, true));
    tick_disabled(&mut view);
    tick_disabled(&mut view);
    assert!(
        !view.preparing_jump() && view.jump_effect().is_none(),
        "silent"
    );
    condition(&mut view, Condition::Intact);
    view.tick(TICK);
    assert!(view.preparing_jump() || view.jump_effect().is_some());
}

#[test]
fn j_let_go_while_disabled_does_nothing_on_the_repair() {
    let mut view = flight();
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    condition(&mut view, Condition::Disabled);
    view.input(&key(JUMP, false));
    tick_disabled(&mut view);
    condition(&mut view, Condition::Intact);
    ticks(&mut view, 2);
    assert!(!view.preparing_jump() && view.jump_effect().is_none());
}

#[test]
fn j_held_while_intact_is_refused_once() {
    let mut view = new_pilot(flight());
    view.input(&key(JUMP, true));
    ticks(&mut view, 5);
    let refused = view
        .take_sounds()
        .iter()
        .filter(|&&sound| sound == REFUSED)
        .count();
    assert_eq!(refused, 1);
}

#[test]
fn j_held_through_a_jump_with_a_hop_to_go_does_not_begin_the_next() {
    let mut view = planner(FlightView::new(FakeCatalog {
        onward: true,
        ..catalog()
    }));
    plot(&mut view, 132);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    tick_to_arrival(&mut view);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(132))));
    let arrival = view.message().map(str::to_owned);
    ticks(&mut view, 3);
    assert!(!view.preparing_jump());
    assert_eq!(view.session().expect("flying").jumping(), None);
    assert_eq!(view.message().map(str::to_owned), arrival, "nor refused");
}

#[test]
fn j_held_through_a_jump_does_nothing_after_arrival() {
    let mut view = new_pilot(flight());
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    assert!(view.preparing_jump());
    view.take_sounds();
    tick_to_arrival(&mut view);
    view.take_sounds();
    ticks(&mut view, 3);
    assert!(!view.take_sounds().contains(&REFUSED));
    assert!(!view.preparing_jump());
    assert_eq!(view.session().expect("flying").jumping(), None);
}

/// The cursor's alpha two ticks after `k` is pressed and held in `view`.
fn cursor_alpha_holding(mut view: View, k: Key) -> Option<u8> {
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(k, true));
    ticks(&mut view, 2);
    drawn(&view)
        .iter()
        .find_map(|command| match *unclipped(command) {
            DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
            _ => None,
        })
}

#[test]
fn a_slot_key_held_fades_the_cursor_while_the_slot_block_polls_it() {
    let pressed_only = faked_alpha_holding(Key::Char('q'));
    assert_ne!(pressed_only, Some(239));
    for k in [
        STELLAR_SLOT_KEYS[1][0],
        STELLAR_SLOT_KEYS[1][1],
        NEAREST_STELLAR_KEY,
    ] {
        assert_eq!(faked_alpha_holding(k), Some(239), "{k:?}");
    }
    assert_eq!(
        faked_alpha_holding(NEAREST_STELLAR_COMMAND_KEY),
        pressed_only,
        "5 without Command"
    );
    let mut menu = super::faked_flight::at_rest(catalog());
    let mut rows = menu.frame.hud.escort_rows;
    rows[0].present = true;
    menu.escort_menu.toggle(menu.elapsed, &rows);
    assert!(menu.escort_menu.is_open());
    assert_eq!(
        faked_alpha_of(menu, STELLAR_SLOT_KEYS[1][0]),
        pressed_only,
        "the menu's 2: its loop sets no fade"
    );
}

#[test]
fn a_slot_key_held_during_a_jump_fades_the_cursor_only_by_its_press() {
    for (view, other) in through_a_jump(flight)
        .into_iter()
        .zip(through_a_jump(flight))
    {
        let stage = stage(&view);
        assert_eq!(
            cursor_alpha_holding(*view, STELLAR_SLOT_KEYS[1][1]),
            cursor_alpha_holding(*other, Key::Char('q')),
            "{stage}"
        );
    }
}

/// Whether `view`'s world draws the double-time icon, `cïcn` 20000 at the
/// top left.
fn faked_double_time_drawn(view: &Faked) -> bool {
    super::faked_world::world_list(view).iter().any(|command| {
        *command
            == DrawCommand::Picture {
                image: ImageKey::icon(20000),
                top_left: Point::new(0.0, 0.0),
            }
    })
}

/// A view over a flight showing the player at rest and NPC 0 of ship 129
/// at (100, -100), and its id.
fn faked_with_npc() -> (Faked, NpcId) {
    let npc = super::faked_world::npc_129(0, Vec2::new(100.0, -100.0), 90.0);
    let view = super::faked_world::world(
        trafficked(&[130], 1, 129, 3),
        super::faked_hud::among(vec![npc]),
    );
    (view, NpcId(0))
}

/// The cursor's alpha `view`'s world draws it with, if it draws it.
fn faked_cursor_alpha(view: &Faked) -> Option<u8> {
    super::faked_world::world_list(view)
        .iter()
        .find_map(|command| match *unclipped(command) {
            DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
            _ => None,
        })
}

/// The cursor's alpha two ticks after `k` is pressed and held in `view`.
fn faked_alpha_of(mut view: Faked, k: Key) -> Option<u8> {
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    super::faked_flight::hold(&mut view, k, true);
    super::faked_flight::ticked(&mut view, 2);
    faked_cursor_alpha(&view)
}

/// [`faked_alpha_of`] a view at rest.
fn faked_alpha_holding(k: Key) -> Option<u8> {
    faked_alpha_of(super::faked_flight::at_rest(catalog()), k)
}

/// A press of `k` as flight's input dispatches the keys these tests use:
/// the cursor told, its key's routine, then the key tracked as held.
fn faked_press(view: &mut Faked, k: Key) {
    let input = key(k, true);
    view.cursor_input(&input);
    match k {
        HYPER_SELECT_KEY | HYPERSPACE_MODE_KEY => view.hyperspace_key(k),
        NEAREST_KEY => view.press_nearest_key(),
        TARGET_KEY => view.press_target_key(),
        _ => view.nav_key(k),
    }
    view.track_held(&input);
}

/// A release of `k`, as flight's input tracks it.
fn faked_release(view: &mut Faked, k: Key) {
    let input = key(k, false);
    view.cursor_input(&input);
    view.track_held(&input);
}

/// How many Nav Off / Hyperspace Mode block presses `view` sent.
fn block_presses(view: &Faked) -> usize {
    sent(view)
        .iter()
        .filter(|command| matches!(command, FlightCommand::NavModeKeys { .. }))
        .count()
}
