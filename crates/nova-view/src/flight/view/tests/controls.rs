use super::*;

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
    let mut view = threatened(QUIET).with_double_time(true);
    assert!(view.double_time());
    ticks(&mut view, 60);
    let list = drawn(&view);
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
    view.set_double_time(false);
    assert!(!view.double_time());
    assert!(!double_time_drawn(&view), "off at once");
    view.tick(TICK);
    assert!(!double_time_drawn(&view));
    assert!(!double_time_drawn(&flight()), "off to begin with");
}

#[test]
fn double_time_runs_a_second_pass_each_tick() {
    for (on, steps) in [(false, 3), (true, 6)] {
        let mut view = flight().with_double_time(on);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 3);
        assert_eq!(player(&view), launched(THRUST, steps), "{on}");
    }
}

#[test]
fn double_time_reaches_the_session_on_the_next_tick_and_when_set() {
    let mut view = flight();
    view.set_double_time(true);
    assert!(view.session().expect("flying").double_time(), "when set");
    view.session
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
    let catalog = boardable();
    let pilot = Pilot::new(&catalog, "Low").expect("starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["reserves"]["fuel"]["now"] = serde_json::json!(95.0);
    let pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let (_, chance) = scripted(&placed(750, 750, 0));
    let mut view = FlightView::with_pilot(catalog, pilot)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_hail_options(HailOptions::empty().with(Rc::new(Refuel)));
    view.catalog.strings = vec![(3000, comm_strings())];
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
fn double_time_ages_the_effects_on_both_passes() {
    let detonated = || {
        let mut view = fighting(detonating(), &[]);
        detonate(&mut view);
        view
    };
    let mut twice = detonated();
    ticks(&mut twice, 2);
    let mut doubled = detonated().with_double_time(true);
    doubled.tick(TICK);
    assert_eq!(doubled.effects().explosions()[0].frame, 2.0);
    assert_eq!(doubled.effects(), twice.effects());
}

#[test]
fn a_shot_detonating_on_the_second_pass_is_drawn_that_tick() {
    // Its one move, on the advance pass, spends it; its end of life
    // comes in the next pass's hit pass.
    let lasting_one = || fighting(detonating(), &[]);
    let mut single = lasting_one();
    single.input(&key(FIRE_KEY, true));
    single.tick(TICK);
    assert!(single.effects().explosions().is_empty(), "not yet");
    let mut view = lasting_one().with_double_time(true);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert_eq!(view.effects().explosions().len(), 1);
    first(&drawn(&view), sprite_of(400));
}

#[test]
fn caps_lock_is_no_flight_key() {
    let mut view = flight();
    view.input(&key(Key::CapsLock, true));
    assert!(view.held.is_empty());
    ticks(&mut view, 3);
    assert_eq!(player(&view), start());
    assert!(!view.double_time(), "the app toggles it, not flight");
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

/// A flight with one still NPC, drawn after a tick, and its id.
fn flight_with_npc(catalog: FakeCatalog) -> (View, NpcId) {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let id = npc_ids(&view)[0];
    (view, id)
}

/// A point on screen over nothing at all.
const NOWHERE: Point = Point::new(40.0, 400.0);

#[test]
fn a_left_click_on_a_drawn_ship_targets_it_with_the_target_beep() {
    let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let over = npc_on_screen(&view, id);
    assert!(over.x < VIEW_SIZE.0 - STATUS_BAR_WIDTH, "{over:?} in space");
    view.take_sounds();
    click(&mut view, over);
    assert_eq!(
        view.session().expect("flying").target().map(|npc| npc.id),
        Some(id)
    );
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
}

#[test]
fn a_click_on_the_players_own_ship_lets_go_of_the_target_with_the_clear_beep() {
    let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let over = npc_on_screen(&view, id);
    assert!(
        (over.x - VIEW_CENTER.x).abs() > 64.0 || (over.y - VIEW_CENTER.y).abs() > 64.0,
        "the NPC at {over:?} sits clear of the player's ship"
    );
    tap(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(id));
    view.take_sounds();
    // The camera centres on the player, so its ship is drawn at the centre.
    click(&mut view, VIEW_CENTER);
    assert_eq!(target_of(&view), None);
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetCleared)]);
}

#[test]
fn a_click_on_the_players_own_ship_with_no_target_is_silent() {
    let (mut view, _) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    view.take_sounds();
    click(&mut view, VIEW_CENTER);
    assert_eq!(target_of(&view), None);
    assert_eq!(view.take_sounds(), []);
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
    let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let over = npc_on_screen(&view, id);
    view.input(&Input::PointerMoved(over));
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, over)]);
    ticks(&mut view, 2);
    assert_eq!(cursor_sprites(&drawn(&view)), [(1, over)]);
    view.input(&Input::PointerMoved(NOWHERE));
    ticks(&mut view, 1);
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
    ticks(&mut view, 4);
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)], "rests");
}

#[test]
fn a_button_press_shows_the_cursor_where_it_is() {
    let mut view = flight();
    assert_eq!(cursor_sprites(&drawn(&view)), [], "no pointer yet");
    view.input(&Input::PointerButton {
        button: MouseButton::Right,
        pressed: true,
        at: NOWHERE,
    });
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
}

#[test]
fn the_cursor_draws_over_the_hud_and_under_the_jump_fade() {
    let cursor_at = |list: &DrawList| {
        list.iter()
            .position(
                |command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 651),
            )
            .expect("drawn")
    };
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    let list = drawn(&view);
    let cursor = cursor_at(&list);
    assert!(cursor > hud_at(&list), "over the HUD");
    assert_eq!(cursor, list.len() - 1, "last, with no fade");

    let mut view = fading_in_on_course_to_barnard();
    view.input(&Input::PointerMoved(NOWHERE));
    let list = drawn(&view);
    let (at_fade, _) = fade(&list).expect("fading in");
    assert!(cursor_at(&list) < at_fade, "under the fade");
}

#[test]
fn a_cloaked_npc_is_hovered_only_with_a_scanner_that_targets_cloaked_ships() {
    let cloaking = |scanned: bool| {
        let mut scanner = radar_outfits();
        let mut mods = [(0, 0); 4];
        mods[1] = (nova_sim::cloak::MOD_SCANNER, nova_sim::cloak::SCAN_TARGET);
        scanner[0].mods = mods;
        let catalog = FakeCatalog {
            outfits: scanner,
            defaults: if scanned {
                vec![(OutfitId(300), 1)]
            } else {
                Vec::new()
            },
            ..trafficked(&[130], 1, 129, 3)
        };
        let (mut view, id) = flight_with_npc(catalog);
        let session = view.session.as_mut().expect("flying");
        nova_sim::fixture::cloak(session, id, CLOAKED);
        let session = view.session().expect("flying");
        assert!(!session.scanner().on_screen, "cannot see it");
        assert_eq!(session.scanner().targets, scanned);
        let over = npc_on_screen(&view, id);
        view.input(&Input::PointerMoved(over));
        ticks(&mut view, 2);
        let frames: Vec<u16> = cursor_sprites(&drawn(&view))
            .into_iter()
            .map(|(frame, _)| frame)
            .collect();
        frames
    };
    assert_eq!(cloaking(false), [0], "rests");
    assert_eq!(
        cloaking(true),
        [1],
        "animates over the ship it does not draw"
    );
}

#[test]
fn the_players_own_ship_is_hovered_only_while_it_holds_a_target() {
    let (mut view, _) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let frame = |view: &mut View| {
        view.input(&Input::PointerMoved(VIEW_CENTER));
        ticks(view, 2);
        cursor_sprites(&drawn(view))
    };
    assert_eq!(frame(&mut view), [(0, VIEW_CENTER)], "no target");
    view.input(&key(TARGET_KEY, true));
    view.input(&key(TARGET_KEY, false));
    assert!(view.session().expect("flying").target().is_some());
    assert_eq!(frame(&mut view), [(1, VIEW_CENTER)], "a target");
}

#[test]
fn an_npc_of_a_cost_minus_one_type_is_not_drawn_but_is_still_hovered() {
    let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let over = npc_on_screen(&view, id);
    let near = |list: &DrawList| {
        list.iter().any(|command| match *command {
            DrawCommand::Sprite { center, image, .. } => {
                image.id != 651
                    && (center.x - over.x).abs() < 0.01
                    && (center.y - over.y).abs() < 0.01
            }
            _ => false,
        })
    };
    assert!(near(&drawn(&view)), "drawn while shown");
    nova_sim::fixture::hide(view.session.as_mut().expect("flying"), id);
    assert!(view.session().expect("flying").is_hidden(ShipRef::Npc(id)));
    assert!(!near(&drawn(&view)), "not drawn hidden");
    view.input(&Input::PointerMoved(over));
    ticks(&mut view, 2);
    assert_eq!(
        cursor_sprites(&drawn(&view)),
        [(1, over)],
        "animates over where it would be drawn"
    );
}

/// [`SlotSprites`] whose `slot` last cloned `shïp` 131's 100-pixel
/// frames, as a flight that showed one there leaves it.
fn cloned_131(slot: nova_sim::ship_layer::Slot) -> SlotSprites {
    let mut sprites = SlotSprites::default();
    sprites.frames.clone_frames(slot, ShipId(131));
    sprites
}

/// Whether the pointer at `over` hovers a ship in `view`, after the
/// two ticks the cursor takes to animate.
fn hovers(view: &mut View, over: Point) -> bool {
    view.input(&Input::PointerMoved(over));
    ticks(view, 2);
    cursor_sprites(&drawn(view))
        .first()
        .is_some_and(|&(frame, _)| frame == 1)
}

#[test]
fn a_hidden_ship_is_hovered_by_the_box_its_berth_last_drew() {
    let engine = Rulebook::default;
    let hover = |rules: Rulebook, off: f32| {
        let (mut view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
        let session = view.session.as_mut().expect("flying");
        nova_sim::fixture::hide(session, id);
        let berth = session
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
            .berth;
        let mut view = view
            .with_rules(rules)
            .with_slot_sprites(cloned_131(nova_sim::ship_layer::Slot::Berth(berth)));
        let carried = sheets(&view).contains(&ShipId(131));
        assert_eq!(
            carried,
            rules == engine(),
            "read as the history is carried in"
        );
        let over = npc_on_screen(&view, id);
        hovers(&mut view, Point::new(over.x, over.y + off))
    };
    assert!(hover(engine(), 40.0), "inside shïp 131's 100-pixel frame");
    assert!(!hover(engine(), 60.0), "outside it");
    let own = || {
        Rulebook::default().with_override(
            nova_sim::RuleKey::HiddenShipHoverBox,
            nova_sim::RuleSource::Bible,
        )
    };
    assert!(hover(own(), 30.0), "inside its own 40-pixel frame's margin");
    assert!(!hover(own(), 40.0), "boxed by its own 40-pixel frame");
}

#[test]
fn a_hidden_player_is_hovered_by_the_box_its_slot_last_drew() {
    let hover = |rules: Rulebook, off: f32| {
        let catalog = FakeCatalog {
            hulls: vec![nova_sim::HullRecord {
                cost: -1,
                ..hull_of(128, &[])
            }],
            ..trafficked(&[130], 1, 129, 3)
        };
        let (view, _) = flight_with_npc(catalog);
        let mut view = view
            .with_rules(rules)
            .with_slot_sprites(cloned_131(nova_sim::ship_layer::Slot::Player));
        assert!(view.session().expect("flying").is_hidden(ShipRef::Player));
        view.input(&key(TARGET_KEY, true));
        view.input(&key(TARGET_KEY, false));
        assert!(view.session().expect("flying").target().is_some());
        hovers(&mut view, Point::new(VIEW_CENTER.x, VIEW_CENTER.y + off))
    };
    let engine = Rulebook::default;
    assert!(hover(engine(), 40.0), "inside shïp 131's 100-pixel frame");
    assert!(!hover(engine(), 60.0));
    let own = || {
        Rulebook::default().with_override(
            nova_sim::RuleKey::HiddenShipHoverBox,
            nova_sim::RuleSource::Bible,
        )
    };
    assert!(hover(own(), 30.0));
    assert!(!hover(own(), 40.0), "boxed by its own sheet");
}

/// [`flight_with_npc`] by `rules`, its NPC gone once drawn and a copy come
/// into its berth 200 pixels to the right, not yet displayed there: the
/// view, the copy, where the first was drawn and where the copy is.
fn refilled(rules: Rulebook) -> (View, NpcId, Point, Point) {
    let (view, id) = flight_with_npc(trafficked(&[130], 1, 129, 3));
    let mut view = view.with_rules(rules);
    let was = npc_on_screen(&view, id);
    let session = view.session.as_mut().expect("flying");
    let at = session
        .npcs()
        .iter()
        .find(|npc| npc.id == id)
        .expect("there")
        .state
        .position
        + Vec2::new(200.0, 0.0);
    let copy = nova_sim::fixture::refill(session, id, at).expect("refilled");
    let now = npc_on_screen(&view, copy);
    (view, copy, was, now)
}

/// The id of `view`'s target.
fn targeted(view: &View) -> Option<NpcId> {
    view.session().expect("flying").target().map(|npc| npc.id)
}

/// Where `view` boxes NPC `id` for the hover and the click, on screen.
fn boxed_at(view: &View, id: NpcId) -> Option<Point> {
    let ships = ViewShips {
        view,
        session: view.session().expect("flying"),
    };
    let drawn = ships.npc(id)?;
    Some(
        view.camera()
            .world_to_screen(Point::new(drawn.centre.x, drawn.centre.y)),
    )
}

#[test]
fn an_undisplayed_ship_is_hovered_and_clicked_at_its_slots_last_place_not_its_own() {
    // `_HandleMouse` and the click block box a slot's hull sprite rect
    // (@0x38177, `_HandlePlayer` @0x6a4ab-0x6a4f4), which only a display
    // places (@0x2cd9d): where its last ship was last laid out.
    let (mut view, copy, was, now) = refilled(Rulebook::default());
    let boxed = boxed_at(&view, copy).expect("boxed");
    assert!(
        (boxed.x - was.x).abs() <= 1.0 && (boxed.y - was.y).abs() <= 1.0,
        "{boxed:?} at {was:?}"
    );
    click(&mut view, now);
    assert_eq!(targeted(&view), None, "not where it is");
    click(&mut view, was);
    assert_eq!(targeted(&view), Some(copy), "where its slot was laid out");
}

#[test]
fn by_the_other_reading_an_undisplayed_ship_is_hovered_and_clicked_where_it_is() {
    let rules = Rulebook::default().with_override(
        nova_sim::RuleKey::UndisplayedShips,
        nova_sim::RuleSource::Bible,
    );
    let (mut view, copy, was, now) = refilled(rules);
    assert_eq!(boxed_at(&view, copy), Some(now));
    click(&mut view, was);
    assert_eq!(targeted(&view), None, "not where its slot was laid out");
    click(&mut view, now);
    assert_eq!(targeted(&view), Some(copy), "where it is");
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
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&Input::PointerLeft);
    assert_eq!(cursor_sprites(&drawn(&view)), []);
}

#[test]
fn a_key_pressed_in_flight_fades_the_cursor_at_the_next_step() {
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(Key::Char('q'), true));
    ticks(&mut view, 1);
    let tint = |view: &View| {
        drawn(view).iter().find_map(|command| match *command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
            _ => None,
        })
    };
    assert_eq!(tint(&view), Some(247), "31 of 32");
    view.input(&key(Key::Up, true));
    ticks(&mut view, 2);
    assert_eq!(tint(&view), Some(231), "held: 29 of 32");
}

#[test]
fn z_held_fades_the_cursor_as_a_flight_key_with_no_afterburner_carried() {
    let tint_after_two = |k: Key| {
        let mut view = flight();
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&key(k, true));
        ticks(&mut view, 2);
        drawn(&view).iter().find_map(|command| match *command {
            DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
            _ => None,
        })
    };
    assert_eq!(tint_after_two(AFTERBURNER_KEY), Some(239), "held: 30 of 32");
    assert_ne!(tint_after_two(Key::Char('q')), Some(239), "a key not held");
}

#[test]
fn the_cursor_is_not_drawn_while_the_map_is_open() {
    let mut view = flight();
    view.input(&Input::PointerMoved(NOWHERE));
    view.input(&key(MAP_KEY, true));
    assert!(view.map_open());
    assert_eq!(cursor_sprites(&drawn(&view)), []);
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
    let mut view = flight();
    plot(&mut view, 131);
    view.input(&key(Key::Alt, true));
    view.input(&key(NAV_OFF_KEY, true));
    ticks(&mut view, 3);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(Some(SystemId(131))));
    assert_eq!(count(&mut view, SimSound::NavSelected), 0, "Option held");
    view.input(&key(Key::Alt, false));
    assert_eq!(
        count(&mut view, SimSound::NavSelected),
        0,
        "at the next frame"
    );
    view.tick(TICK);
    assert_eq!(count(&mut view, SimSound::NavSelected), 1);
    assert_eq!(nav_mode(&view), NavMode::Off);
    ticks(&mut view, 2);
    assert_eq!(count(&mut view, SimSound::NavSelected), 0, "once");
}

#[test]
fn h_held_blocks_a_press_of_nav_off() {
    let mut view = flight();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    view.take_sounds();
    view.input(&key(NAV_OFF_KEY, true));
    view.tick(TICK);
    assert_eq!(sims(&mut view), []);
    assert_eq!(nav_mode(&view), NavMode::Hyperspace(None));
}

#[test]
fn nav_off_held_blocks_a_press_of_h_which_still_shows_the_map() {
    let mut view = flight();
    view.input(&key(NAV_OFF_KEY, true));
    view.tick(TICK);
    view.take_sounds();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    assert_eq!(floating_alpha(&view), Some(255), "on the press's own frame");
    view.tick(TICK);
    assert_eq!(sims(&mut view), []);
    assert_eq!(nav_mode(&view), NavMode::Off);
}

#[test]
fn a_key_of_the_block_let_go_frees_the_other_held_to_press_again() {
    let mut view = flight();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.input(&key(NAV_OFF_KEY, true));
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    view.tick(TICK);
    view.take_sounds();
    view.input(&key(NAV_OFF_KEY, false));
    view.tick(TICK);
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    assert_eq!(
        count(&mut view, SimSound::NavSelected),
        1,
        "both up a frame"
    );
}

#[test]
fn each_fresh_press_of_h_or_nav_off_acts_with_no_tick_between() {
    for k in [HYPERSPACE_MODE_KEY, NAV_OFF_KEY] {
        let mut view = flight();
        tap(&mut view, k);
        tap(&mut view, k);
        assert_eq!(count(&mut view, SimSound::NavSelected), 2, "{k:?}");
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
    let mut view = fighting(armed(2, &[]), &two_npcs());
    view.input(&key(Key::Alt, true));
    view.input(&key(NAV_OFF_KEY, true));
    view.tick(TICK);
    view.take_sounds();
    tap(&mut view, NEAREST_KEY);
    assert!(target_of(&view).is_some(), "Option-R picks");
    view.take_sounds();
    view.tick(TICK);
    assert_eq!(target_of(&view), None);
    assert_eq!(count(&mut view, SimSound::TargetCleared), 1);
}

#[test]
fn another_key_pressed_while_option_backquote_is_held_leaves_the_target_to_the_tick() {
    let mut view = fighting(armed(2, &[]), &two_npcs());
    view.input(&key(Key::Alt, true));
    view.input(&key(NAV_OFF_KEY, true));
    view.tick(TICK);
    alt_tap(&mut view, NEAREST_KEY);
    view.input(&key(Key::Alt, true));
    let picked = target_of(&view);
    assert!(picked.is_some());
    tap(&mut view, Key::Char('q'));
    assert_eq!(target_of(&view), picked, "no press of the block");
}

#[test]
fn option_backquote_held_blocks_h_but_not_its_map_nor_with_h_held_its_target() {
    let mut view = fighting(armed(2, &[]), &two_npcs());
    view.input(&key(Key::Alt, true));
    view.input(&key(NAV_OFF_KEY, true));
    view.tick(TICK);
    view.take_sounds();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    assert_eq!(floating_alpha(&view), Some(255), "the map shown");
    view.tick(TICK);
    assert_eq!(count(&mut view, SimSound::NavSelected), 0, "H blocked");
    assert_eq!(nav_mode(&view), NavMode::Off);
    tap(&mut view, NEAREST_KEY);
    assert!(target_of(&view).is_some());
    view.tick(TICK);
    assert!(target_of(&view).is_some(), "the latch held by H");
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
