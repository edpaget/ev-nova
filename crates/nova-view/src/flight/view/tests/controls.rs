use super::*;
use nova_sim::DrawnShips as _;

// Double time.

#[test]
fn double_time_shows_its_icon_at_the_top_left_over_the_red_alert() {
    // Whether double time is on, and the alert, are the frame's
    // (`a_fill_writes_the_murk_the_background_double_time_the_gamma_and_the_traffic`,
    // `a_fill_writes_the_red_alert_icon_shown`).
    let mut frame = among(Vec::new());
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
pub(super) fn cursor_sprites(list: &DrawList) -> Vec<(u16, Point)> {
    sprites(list)
        .into_iter()
        .filter(|(image, _)| image.id == 651)
        .map(|(image, at)| (image.frame, at))
        .collect()
}

/// A point on screen over nothing at all.
pub(super) const NOWHERE: Point = Point::new(40.0, 400.0);

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

/// The last hover `view` asked of its flight
/// ([`FlightCommand::Hover`]): its point and boxes.
fn last_hover(view: &Faked) -> (Vec2, nova_sim::flight_frame::DrawnBoxes) {
    view.sim
        .asks()
        .iter()
        .rev()
        .find_map(|ask| match ask {
            FlightCommand::Hover { at, drawn } => Some((*at, drawn.clone())),
            _ => None,
        })
        .expect("asked")
}

#[test]
fn an_ask_doesnt_refill_the_kept_frame() {
    // The cursor's hover each step is an ask
    // (rdm:phase/view-sim-seam/phase-20-view-frame-fills-per-tick), which
    // changes nothing the frame shows, so a tick with it fills no more
    // than a quiet tick: the catch-up and the steps.
    let (mut view, id) = faked_with_npc();
    view.sim.set_hover(Some(nova_sim::Hover::Ship(id)));
    view.cursor_input(&Input::PointerMoved(npc_on_screen(&view, id)));
    view.tick(TICK);
    let (asked, fills) = (view.sim.asks().len(), view.sim.fills());
    view.tick(TICK);
    assert_eq!(view.sim.asks().len(), asked + 1, "the tick's hover asked");
    assert_eq!(view.sim.fills() - fills, 2, "and filled nothing");
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
    let (at, boxes) = last_hover(&view);
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
    let (_, boxes) = last_hover(&view);
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
    queue_map(&mut view, None, sol_drawing());
    view.cursor_input(&Input::PointerMoved(NOWHERE));
    view.open_map();
    assert!(view.map_open);
    assert_eq!(cursor_sprites(&super::faked_world::world_list(&view)), []);
}

// Keys held across a jump's end, an Option release or a repair
// (`_HandlePlayer`'s latches 0xdee04, 0xdee48 and 0xdee44).

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
    let npc = npc_129(0, Vec2::new(100.0, -100.0), 90.0);
    let view = super::faked_world::world(trafficked(&[130], 1, 129, 3), among(vec![npc]));
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
