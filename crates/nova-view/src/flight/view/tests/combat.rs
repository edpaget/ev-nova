use super::faked_hud::{barred, barred_by, flown, hud_tick};
use super::faked_world::{shows, world, world_list};
use super::*;
use nova_sim::flight_frame::{FrameShip, FrameTarget, PassReport, ShipPlace, StepReport, StepRun};

// Traffic.

#[test]
fn an_npc_is_drawn_with_its_own_sprite_where_it_is_facing_its_heading() {
    let mut view = world(
        trafficked(&[130], 1, 129, 3),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(90.0)]),
    );
    // A tick in, the Moon a frame on.
    view.step_stellars(TICK);
    let list = world_list(&view);
    let frame = rotation_frame(90.0, NonZeroU16::new(72).expect("non-zero"));
    assert_eq!(
        sprites(&list),
        [
            (ImageKey::sprite(1128, 0), at(415.0, -216.0)),
            (ImageKey::sprite(1129, 1), at(715.0, 184.0)),
            (ImageKey::sprite(2001, frame), at(515.0, 284.0)),
            (ImageKey::sprite(2101, frame), at(515.0, 284.0)),
            (ImageKey::sprite(2201, frame), at(515.0, 284.0)),
            (ImageKey::sprite(2000, 0), PLAYER_AT),
        ],
        "after the stellars, before the player, its glow and lights over it"
    );
}

/// NPC 0 of ship 129 at (100, -100) facing `heading`, 72 frames a turn,
/// its glow at 32 and its steady lights at 32, as [`trafficked`]'s
/// arrivals are drawn.
fn lit_npc(heading: f32) -> nova_sim::flight_frame::FrameShip {
    nova_sim::flight_frame::FrameShip {
        frames: nova_sim::combat::ship_frames::FrameSpec {
            frames: NonZeroU16::new(72).expect("non-zero"),
            lights: true,
            ..nova_sim::combat::ship_frames::FrameSpec::NONE
        },
        drawn: nova_sim::combat::ship_frames::Drawn {
            lights: Some(32),
            ..nova_sim::combat::ship_frames::Drawn::default()
        },
        glow: 32,
        ..npc_129(0, Vec2::new(100.0, -100.0), heading)
    }
}

#[test]
fn a_ship_waiting_at_its_gate_is_drawn_by_its_tint() {
    // How a waiting, held or cloaked ship's cloak shows is the session's
    // (nova-sim's `a_fully_cloaked_ship_is_faint_when_led_or_scanned_at_any_range_and_hidden_otherwise`
    // and `every_ship_still_fading_is_drawn_fading_led_or_not_and_no_fade_is_opaque`);
    // a held ship is at no gate (`GateFade::of`, nova-sim's
    // `an_npcs_gate_fade_follows_its_mode`).
    assert_eq!(
        npc_tint(GateFade::Emerging { ticks_left: 30 }),
        None,
        "hidden at its gate"
    );
    // Held there by the player's hit, its goal is 4, not 0x15, so
    // `_HandleShipDisplay`'s fade (@0x2c090) passes it by.
    assert_eq!(
        npc_tint(GateFade::None),
        Some(Color::WHITE),
        "held: drawn plain"
    );
}

#[test]
fn a_ship_its_record_puts_on_the_radar_is_a_blip_and_one_it_keeps_off_is_not() {
    // Whether a cloaked ship shows there is the session's: pinned by
    // nova-sim's
    // `a_cloaked_ship_is_on_the_radar_only_when_its_cloak_shows_there_or_the_player_scans_for_it`
    // and the frame's `a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order`.
    let npc = record::npc(0, 129, Vec2::new(100.0, -100.0), 0.0);
    let at_blip = at(100.0, -100.0);
    let blip = RadarShip {
        position: at_blip,
        targeted: false,
        iff: ShipIff::Other,
        heavy: false,
    };
    assert_eq!(radar_ships(None, &[(&npc, at_blip, 0.0)]), [blip]);
    let off = nova_sim::flight_frame::FrameShip {
        on_radar: false,
        ..npc
    };
    assert_eq!(radar_ships(None, &[(&off, at_blip, 0.0)]), []);
    let threat = nova_sim::flight_frame::FrameShip {
        iff: ShipIff::Threat,
        mass: 100.0,
        ..npc
    };
    assert_eq!(
        radar_ships(Some(NpcId(0)), &[(&threat, at_blip, 0.0)]),
        [RadarShip {
            targeted: true,
            iff: ShipIff::Threat,
            heavy: true,
            ..blip
        }]
    );
    let light = nova_sim::flight_frame::FrameShip { mass: 99.0, ..npc };
    assert_eq!(
        radar_ships(Some(NpcId(1)), &[(&light, at_blip, 0.0)]),
        [blip],
        "not the target, and not above 99"
    );
}

#[test]
fn the_play_area_is_the_screen_left_of_the_status_bar_about_the_ship() {
    let area = play_area(VIEW_SIZE, crate::system::camera::Anchor::PlayAreaCentre);
    assert_eq!(
        area,
        nova_sim::cloak::Area {
            left: -415.0,
            top: -384.0,
            right: 1024.0 - 194.0 - 415.0,
            bottom: 384.0,
        }
    );
    assert_eq!(
        play_area(VIEW_SIZE, crate::system::camera::Anchor::ViewCentre),
        nova_sim::cloak::Area {
            left: -512.0,
            top: -384.0,
            right: 1024.0 - 194.0 - 512.0,
            bottom: 384.0,
        }
    );
    let mut view = world(
        trafficked(&[130], 1, 129, 3),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    view.step_frame(1, view.anchor());
    assert_eq!(view.sim.inputs()[0].play_area, area, "told");
}

#[test]
fn the_play_area_anchor_centres_the_ship_in_the_play_area() {
    let anchor = crate::system::camera::Anchor::PlayAreaCentre;
    for width in [800.0, 1024.0, 1280.0, 1366.0] {
        let view = (width, 768.0);
        let area = play_area(view, anchor);
        let screen_mid = (area.right - area.left) * 0.5;
        assert_eq!(screen_mid, anchor.point(view).x, "width {width}");
        assert_eq!(area.left + area.right, 0.0, "width {width}");
    }
}

#[test]
fn an_npc_is_a_dim_blip_on_the_radar() {
    // Where the traffic puts it is the session's; its record puts it on
    // the radar (`a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order`).
    let view = barred(
        trafficked(&[130], 1, 129, 3),
        among(vec![npc_129(0, Vec2::new(100.0, -100.0), 90.0)]),
    );
    // (100, -100) is 3.125 pixels each way: (929.125, 92.875).
    let blip = RadarMark::Pixel {
        x: 929,
        y: 92,
        color: RadarColor::Dim,
    };
    assert_eq!(blips(&view), [blip]);
    let list = flown(&view);
    assert!(
        list.iter().any(|command| *command
            == DrawCommand::Dot {
                center: at(929.5, 92.5),
                size: 1.0,
                color: layout().dim_radar,
            }),
        "{list:?}"
    );
}

/// The tints the NPC sprite (ship 129's sheet, 2001) is drawn with,
/// plainly or colorized.
fn npc_tints(list: &DrawList) -> Vec<Color> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, tint, .. }
            | DrawCommand::Colorized { image, tint, .. }
                if image.id == 2001 =>
            {
                Some(*tint)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn an_npc_coming_out_of_a_gate_is_hidden_then_fades_in() {
    // When it comes out and how long it waits is the session's
    // (nova-sim's emerge tests); it is on the radar all along, by its
    // record (`a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order`,
    // the radar's `a_ship_its_record_puts_on_the_radar_is_a_blip_and_one_it_keeps_off_is_not`).
    let mut view = world(
        trafficked(&[130], 1, 129, 3),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(0.0)]),
    );
    let at_gate = |view: &mut Faked, gate| {
        let mut frame = view.frame.clone();
        frame.ships[1].gate = gate;
        shows(view, frame);
        npc_tints(&world_list(view))
    };
    assert_eq!(
        at_gate(&mut view, GateFade::Emerging { ticks_left: 40 }),
        [],
        "hidden"
    );
    assert_eq!(
        at_gate(&mut view, GateFade::Emerging { ticks_left: 8 }),
        [Color::rgba(255, 255, 255, 128)],
        "half in"
    );
    assert_eq!(at_gate(&mut view, GateFade::None), [Color::WHITE]);
}

#[test]
fn an_npc_coming_out_of_a_gate_is_on_the_radar_all_along() {
    // When it comes out is the session's (nova-sim's emerge tests); its
    // record keeps it on the radar.
    let emerging = FrameShip {
        gate: GateFade::Emerging { ticks_left: 40 },
        ..lit_npc(0.0)
    };
    let view = barred(trafficked(&[130], 1, 129, 3), among(vec![emerging]));
    assert_eq!(
        npc_tints(&super::faked_world::world_list(&view)),
        [],
        "hidden"
    );
    assert_eq!(blips(&view).len(), 1, "on the radar all the same");
}

/// [`armed`] with a volley the answer flag answers at a ship the player
/// can't target, hidden in its gate: two guns of 5 energy damage firing
/// together, each of `Reload` 30, at ships of govt 140, of `ShootPenalty`
/// 5. The first hit of the volley is a stray one, forgiven below 50
/// stray provocation with no roll, and adds 1.75 × 30 to it; the second,
/// on the same tick, reads 52.5 and is answered (see
/// `nova_sim::ai::answer_flag`).
fn volleying() -> FakeCatalog {
    let mut catalog = armed(1, &[BLASTER, UNSEEN]);
    for weapon in &mut catalog.weapons {
        weapon.energy_dmg = 5;
        weapon.reload = 30;
    }
    let mut govt = super::adapter::damaging().govts.remove(0);
    govt.penalties.shoot = 5;
    catalog.govts = vec![govt];
    catalog
}

#[test]
fn a_ship_held_at_its_gate_is_drawn_plain_on_the_frame_it_is_held() {
    // The hit is answered inside the pass, before the frame is drawn,
    // holding it at its gate: the session's (nova-sim's
    // `session::step::tests::each_pass_reports_the_session_as_it_stood_after_it`
    // and the gate-follow tests); the frame after the pass holds it at no
    // gate, and it is drawn plain on that frame.
    let mut view = world(
        volleying(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(0.0)]),
    );
    let mut frame = view.frame.clone();
    frame.ships[1].gate = GateFade::Emerging { ticks_left: 40 };
    shows(&mut view, frame.clone());
    assert_eq!(npc_tints(&world_list(&view)), [], "hidden");
    frame.ships[1].gate = GateFade::None;
    view.sim.set_frame(frame);
    view.step_frame(1, view.anchor());
    assert_eq!(npc_tints(&world_list(&view)), [Color::WHITE], "seen");
}

#[test]
fn a_ship_held_at_its_gate_on_the_second_pass_is_drawn_on_that_frame() {
    // Double time's second pass answers the hit too (`_HandleShot`
    // @0x44385 runs on both): the session's (nova-sim's
    // `session::step::tests::a_step_runs_a_second_pass_only_in_double_time`).
    // The frame after the step's two passes is drawn.
    let mut view = world(
        volleying(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(0.0)]),
    );
    let mut held = view.frame.clone();
    held.ships[1].gate = GateFade::None;
    let pass = |frame: &FlightFrame| PassReport {
        ships: frame.ships.clone(),
        double_time: true,
        ..PassReport::default()
    };
    let mut waiting = held.clone();
    waiting.ships[1].gate = GateFade::Emerging { ticks_left: 40 };
    view.sim.set_frame(held.clone());
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass(&waiting), pass(&held)],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, view.anchor());
    assert_eq!(npc_tints(&world_list(&view)), [Color::WHITE], "seen");
}

#[test]
fn a_quote_said_on_a_second_pass_is_on_the_message_line_that_frame() {
    // `_DamageShip`'s reaction on the second pass turns it on the
    // player, and `_AIMakeShipAttackPlayer` says its quote there
    // (`_ShowPersonHailQuote` @0x89ce1), before the next frame: the
    // session's (nova-sim's `the_quotes_said_before_a_pass_come_with_that_pass`).
    let mut view = world(
        volleying(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(0.0)]),
    );
    let quote = nova_sim::PersonQuote {
        npc: NpcId(0),
        text: "Ace: Prepare to die, !".to_owned(),
    };
    let frame = view.frame.clone();
    let pass = |quotes| PassReport {
        ships: frame.ships.clone(),
        double_time: true,
        quotes,
        ..PassReport::default()
    };
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass(Vec::new()), pass(vec![quote])],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, view.anchor());
    assert_eq!(view.message(), Some("Ace: Prepare to die, !"));
}

/// `catalog` with hypergate 141 at (200, 100) in Sol, of 6 frames
/// stepping every tick (`AnimDelay` 0, as stock's), drawing `rlëD` 1141.
fn with_hypergate(mut catalog: FakeCatalog) -> FakeCatalog {
    catalog.sites.push(LandingSite {
        flags2: nova_sim::gate::HYPERGATE,
        exit_angle: 90,
        animation: nova_sim::catalog::SiteAnimation {
            type_frames: 6,
            ..nova_sim::catalog::SiteAnimation::default()
        },
        ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
    });
    catalog.sol_more.push(stellar(141, "Gate", (200, 100), 6));
    catalog
}

/// The frames hypergate 141's sheet is drawn on.
fn gate_frames(list: &DrawList) -> Vec<u16> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Sprite { image, .. } if image.id == 1141 => Some(image.frame),
            _ => None,
        })
        .collect()
}

#[test]
fn a_ship_entering_a_gate_fades_out_by_two_32nds_a_tick() {
    let entering = |ticks_left| GateFade::Entering { ticks_left };
    assert_eq!(
        npc_tint(entering(15)),
        Some(Color::rgba(255, 255, 255, 239)),
        "30/32"
    );
    assert_eq!(
        npc_tint(entering(8)),
        Some(Color::rgba(255, 255, 255, 128)),
        "half"
    );
    assert_eq!(
        npc_tint(entering(1)),
        Some(Color::rgba(255, 255, 255, 16)),
        "2/32"
    );
    assert_eq!(npc_tint(entering(16)), Some(Color::WHITE));
}

#[test]
fn a_hypergate_is_drawn_on_the_frame_its_record_gives_and_an_npc_entering_it_fading() {
    // When the gate opens, as an NPC heads in or waits in it, is the
    // session's (nova-sim's gate sprite tests, `gate_sprite`); the view
    // draws the frame each record gives, laid out and after each step.
    let busy = with_hypergate(trafficked(&[130], 1, 129, 3));
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc(0.0)]);
    let mut gate = record::stellar(StellarId(141));
    gate.gate_frame = Some(0);
    frame.stellars.push(gate);
    let mut view = world(busy, frame);
    assert_eq!(gate_frames(&world_list(&view)), [0], "closed at first");
    let mut shown = Vec::new();
    let mut tints = Vec::new();
    for (open, ticks_left) in [(1, 15), (2, 8), (3, 1)] {
        let mut frame = view.frame.clone();
        frame.stellars[2].gate_frame = Some(open);
        frame.ships[1].gate = GateFade::Entering { ticks_left };
        view.sim.set_frame(frame);
        view.step_frame(1, view.anchor());
        if let Some(scene) = &mut view.scene {
            super::super::show_gate_frames(scene, &view.frame);
        }
        let list = world_list(&view);
        shown.extend(gate_frames(&list));
        tints.extend(npc_tints(&list));
    }
    assert_eq!(shown, [1, 2, 3], "the frames the records gave");
    assert_eq!(
        tints,
        [
            Color::rgba(255, 255, 255, 239),
            Color::rgba(255, 255, 255, 128),
            Color::rgba(255, 255, 255, 16)
        ]
    );
}

#[test]
fn an_npc_is_drawn_between_its_last_two_steps() {
    // Where it heads, Nova's AI's business, is the session's.
    let mut view = world(
        trafficked(&[130], 1, 129, 1),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            npc_129(0, Vec2::new(0.0, 100.0), 0.0),
        ]),
    );
    let mut moved = view.frame.clone();
    moved.ships[1].position = Vec2::new(0.0, 94.0);
    view.sim.set_frame(moved);
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            before: vec![
                (ShipRef::Player, ShipPlace::default()),
                (
                    ShipRef::Npc(NpcId(0)),
                    ShipPlace {
                        position: Vec2::new(0.0, 100.0),
                        heading: 0.0,
                    },
                ),
            ],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, view.anchor());
    view.alpha = 0.5;
    let shown = sprites(&world_list(&view))[2].1;
    let expected = view.camera().world_to_screen(at(0.0, 97.0));
    assert!(
        (shown.y - expected.y).abs() < 1e-3,
        "{shown:?} {expected:?}"
    );
    assert_eq!(shown.x, expected.x);
}

#[test]
fn an_npc_whose_sheet_cannot_be_read_is_a_crossed_box() {
    let mut npc = lit_npc(90.0);
    npc.class = ShipId(130);
    npc.framed = Some(ShipId(130));
    let view = world(
        trafficked(&[130], 1, 130, 3),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), npc]),
    );
    let list = world_list(&view);
    let mut boxed = DrawList::new();
    crossed_box(&mut boxed, at(515.0, 284.0), PLACEHOLDER_SIZE, PLACEHOLDER);
    let commands: Vec<_> = list.iter().cloned().collect();
    let expected: Vec<_> = boxed.iter().cloned().collect();
    assert!(
        commands.windows(expected.len()).any(|run| run == expected),
        "{commands:?}"
    );
    assert_eq!(sprites(&list).len(), 3, "two stellars and the player");
}

#[test]
fn each_ship_types_sheet_is_read_once() {
    // Which types the traffic brings is the session's.
    let mut view = world(
        trafficked(&[130, 131], 3, 129, 1),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    let mut frame = in_sol_with(vec![
        the_player_at(Vec2::ZERO, 0.0),
        npc_129(0, Vec2::ZERO, 0.0),
        npc_129(1, Vec2::ZERO, 0.0),
        npc_129(2, Vec2::ZERO, 0.0),
    ]);
    frame.traffic_ships = vec![ShipId(129)];
    shows(&mut view, frame.clone());
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)]
    );
    frame.system = SystemId(131);
    shows(&mut view, frame);
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "not again"
    );
}

/// [`trafficked`] in 130 and 131, with 131's traffic düde 129 flying
/// ships 129 and 130 where 130's flies only 129.
fn trafficked_with_new_types_in_131() -> FakeCatalog {
    let mut catalog = trafficked(&[130, 131], 2, 129, 1);
    let mut dude_types = [(-1, 0); 8];
    dude_types[0] = (129, 100);
    catalog.traffic[1].1.dude_types = dude_types;
    catalog.dudes.push((
        nova_sim::DudeId(129),
        nova_sim::DudeRecord {
            ai_type: 1,
            govt: None,
            ships: vec![(ShipId(129), 1), (ShipId(130), 1)],
            booty: 0,
            info_types: 0,
        },
    ));
    catalog
}

#[test]
fn the_traffics_types_cards_are_read_in_one_batch_each_new_type_once() {
    let mut view = world(
        trafficked_with_new_types_in_131(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    let mut frame = in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]);
    frame.traffic_ships = vec![ShipId(129)];
    shows(&mut view, frame.clone());
    assert_eq!(*view.catalog().cards_asked.borrow(), [vec![ShipId(129)]]);
    frame.traffic_ships = vec![ShipId(129), ShipId(130)];
    shows(&mut view, frame);
    assert_eq!(
        *view.catalog().cards_asked.borrow(),
        [vec![ShipId(129)], vec![ShipId(130)]],
        "only the type new to 131"
    );
}

#[test]
fn the_warm_up_reads_the_coming_traffics_sheets_and_cards_so_the_first_step_reads_none() {
    // Which types are coming is the session's
    // (`Session::coming_traffic_ships`).
    let mut view = world(
        trafficked(&[130, 131], 3, 129, 1),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    view.sim.set_coming_traffic(vec![ShipId(129)]);
    assert!(!view.has_read_traffic(ShipId(129)), "nothing read yet");
    view.warm_up_traffic();
    assert!(view.has_read_traffic(ShipId(129)), "its sheet and card");
    assert!(!view.has_read_traffic(ShipId(130)), "not coming");
    let sheets = view.catalog().sheets_asked.borrow().clone();
    let cards = view.catalog().cards_asked.borrow().clone();
    assert_eq!(sheets, [ShipId(128), ShipId(129)]);
    assert_eq!(cards, [vec![ShipId(129)]]);
    let card = view.cards.remove(&ShipId(129)).expect("read");
    assert!(!view.has_read_traffic(ShipId(129)), "its sheet alone");
    view.cards.insert(ShipId(129), card);
    let mut frame = in_sol_with(vec![
        the_player_at(Vec2::ZERO, 0.0),
        npc_129(0, Vec2::ZERO, 0.0),
    ]);
    frame.traffic_ships = vec![ShipId(129)];
    shows(&mut view, frame);
    assert_eq!(*view.catalog().sheets_asked.borrow(), sheets);
    assert_eq!(*view.catalog().cards_asked.borrow(), cards);
}

// Combat.

// Combat controls and display.

#[test]
fn r_targets_the_nearest_threat_over_a_nearer_ship_and_brackets_it_red() {
    // Which ship R picks is nova-sim's
    // (`selecting_the_nearest_threat_picks_the_nearest_npc_fighting_the_player`),
    // and its standing (`standing_is_disabled_first_then_escort_then_threat_then_neutral`).
    let mut view = barred(armed(2, &[]), among(vec![lit_npc(180.0)]));
    view.press_nearest_key();
    view.track_held(&key(Key::Alt, true));
    view.press_nearest_key();
    assert_eq!(
        sent(&view),
        [
            FlightCommand::SelectTarget(nova_sim::TargetPick::NearestThreat),
            FlightCommand::SelectTarget(nova_sim::TargetPick::Nearest),
        ]
    );
    let bracketed = |view: &mut Faked, standing| {
        let mut frame = view.frame.clone();
        frame.target = Some(record::target(0));
        frame.hud.target_brackets = Some(nova_sim::brackets::TargetBrackets {
            npc: NpcId(0),
            expand: 0.0,
            standing,
        });
        shows(view, frame);
        bracket_icons(&flown(view))
    };
    assert_eq!(
        bracketed(&mut view, Standing::Hostile),
        [10008, 10009, 10010, 10011],
        "red"
    );
    assert_eq!(
        bracketed(&mut view, Standing::Neutral),
        [10012, 10013, 10014, 10015],
        "yellow"
    );
}

pub(super) fn line_in(color: Color) -> impl Fn(&DrawCommand) -> bool {
    move |command| matches!(command, DrawCommand::Line { color: c, .. } if *c == color)
}

/// A scripted fight: the player at the centre and NPC 0 100 above it,
/// the player's blaster shot between them, its beam drawn under the
/// ships and its beam over them at the NPC, and `bööm` 128 going off at
/// the NPC, in a system of effective murk `murk`.
fn firing_at_an_npc_with_murk(catalog: FakeCatalog, murk: u8) -> Faked {
    let npc = Vec2::new(0.0, -100.0);
    let mut frame = FlightFrame {
        shots: vec![super::faked_world::shot_of(
            0,
            BLASTER,
            Vec2::new(0.0, -50.0),
            0.0,
        )],
        beams: vec![
            super::faked_world::beam(ShipRef::Player, UNDER, Vec2::ZERO, npc),
            super::faked_world::beam(ShipRef::Player, OVER, Vec2::new(5.0, 0.0), npc),
        ],
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), lit_npc_at(npc, 180.0)])
    };
    frame.effective_murk = murk;
    let mut view = world(catalog, frame);
    super::faked_world::raise(&mut view, &[exploding_at(npc)]);
    view
}

/// `bööm` 128 going off at `at`.
fn exploding_at(at: Vec2) -> CombatEvent {
    CombatEvent::Exploded {
        at,
        explosion: nova_sim::catalog::Explosion {
            boom: BoomId(128),
            extra: false,
        },
        size: 0.0,
    }
}

/// [`lit_npc`] at `at`.
fn lit_npc_at(at: Vec2, heading: f32) -> nova_sim::flight_frame::FrameShip {
    nova_sim::flight_frame::FrameShip {
        position: at,
        ..lit_npc(heading)
    }
}

#[test]
fn the_fight_is_drawn_between_the_stellars_in_order() {
    // The world under the title and the HUD: layers.rs's
    // `a_fights_scene_and_its_target_layer_are_the_world_and_its_hud_and_every_text_overlay`.
    let mut view = firing_at_an_npc_with_murk(armed(1, &[BLASTER, UNDER, OVER]), 0);
    let list = super::faked_world::world_list_with(&mut view, &super::faked_world::targeting(0));
    let stellar = first(&list, sprite_of(1128));
    let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
    let npc = first(&list, sprite_of(2001));
    let ship = first(&list, sprite_of(2000));
    let shot = first(&list, sprite_of(3500));
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let explosion = first(&list, sprite_of(400));
    let brackets = first(&list, icon_in(10008..=10023));
    // An unguided shot is in the shot layer 0x3b52f0, under the intact
    // ships, and the explosion in `_explodLayer` 0x3b52b8, under the target
    // layer's brackets and then its beams (`_SetupSpriteWorld`
    // @0x21d0c-0x21e24).
    let order = [stellar, under, shot, npc, ship, explosion, brackets, over];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    let DrawCommand::Sprite { tint, blend, .. } =
        list.iter().nth(explosion).cloned().expect("drawn")
    else {
        panic!("a sprite")
    };
    // A clear system: added at full, unmurked (`_HandleExplods`
    // @0x2f7b0-0x2f7e2).
    assert_eq!((blend, tint), (Blend::Or, lights_tint(32)));
}

#[test]
fn an_explosion_in_a_murky_system_is_murked_as_the_player_sees_it() {
    let view = firing_at_an_npc_with_murk(armed(1, &[BLASTER, UNDER, OVER]), 100);
    let boom = view.effects().explosions()[0].at;
    let level =
        crate::flight::murk::sprite_murk(nova_sim::Vec2::new(boom.x, boom.y), Vec2::ZERO, 100);
    assert!(level > 0, "murked where it went off: {boom:?}");
    let list = world_list(&view);
    let explosion = first(&list, sprite_of(400));
    let DrawCommand::Sprite { tint, blend, .. } =
        list.iter().nth(explosion).cloned().expect("drawn")
    else {
        panic!("a sprite")
    };
    assert_eq!(
        (blend, tint),
        (
            Blend::Or,
            crate::flight::murk::murk_tint(lights_tint(32), level)
        )
    );
}

#[test]
fn a_shot_is_drawn_with_its_frame_where_it_flies() {
    // Turned 45 degrees right, so the shot flies across and up.
    let shot = nova_sim::flight_frame::FrameShot {
        velocity: Vec2::new(14.0, -14.0),
        ..super::faked_world::shot_of(0, BLASTER, Vec2::new(28.0, -28.0), 45.0)
    };
    let mut view = world(
        armed(0, &[BLASTER]),
        FlightFrame {
            shots: vec![shot],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 45.0)])
        },
    );
    view.alpha = 0.5;
    let shown = at(
        shot.position.x - shot.velocity.x * 0.5,
        shot.position.y - shot.velocity.y * 0.5,
    );
    let list = world_list(&view);
    let drawn_at = sprites(&list)
        .into_iter()
        .find(|(image, _)| image.id == 3500)
        .expect("the shot");
    assert_eq!(drawn_at.0, ImageKey::sprite(3500, 4), "heading 45");
    let expected = view.camera().world_to_screen(shown);
    assert!(
        (drawn_at.1.x - expected.x).abs() < 1e-3 && (drawn_at.1.y - expected.y).abs() < 1e-3,
        "{drawn_at:?} {expected:?}"
    );
}

#[test]
fn a_shot_is_drawn_under_the_intact_player_in_the_shot_layer() {
    let view = world(
        armed(0, &[BLASTER]),
        FlightFrame {
            shots: vec![super::faked_world::shot_of(
                0,
                BLASTER,
                Vec2::new(0.0, -20.0),
                0.0,
            )],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
        },
    );
    let list = world_list(&view);
    let shot = list
        .iter()
        .position(
            |command| matches!(*command, DrawCommand::Sprite { image, .. } if image.id == 3500),
        )
        .expect("the shot");
    let centre = view.camera().world_to_screen(view.shown_position());
    assert!(
        shot < drawn_at(&list, centre),
        "under the player (0x3b52f0)"
    );
}

/// Where the beams in `color` start, on screen: the lines a pixel wide,
/// not the HUD's bars.
fn beam_starts(list: &DrawList, color: Color) -> Vec<Point> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Line {
                from,
                color: c,
                width,
                ..
            } if c == color && width == 1.0 => Some(from),
            _ => None,
        })
        .collect()
}

#[test]
fn a_beams_fade_is_shown_as_its_record_counts_it() {
    // How the session counts it down is nova-sim-combat's beam tests.
    let mut beam =
        super::faked_world::beam(ShipRef::Player, OVER, Vec2::ZERO, Vec2::new(0.0, -200.0));
    beam.fade = 7;
    let view = world(
        armed(0, &[OVER]),
        FlightFrame {
            beams: vec![beam],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
        },
    );
    let shown: Vec<u32> = view
        .shown_beams(&view.camera())
        .iter()
        .map(|beam| beam.fade)
        .collect();
    assert_eq!(shown, [7]);
}

#[test]
fn a_beam_is_drawn_from_its_firer_where_the_firer_is_drawn() {
    let mut catalog = armed(1, &[OVER]);
    catalog.hulls[1] = hull_of(129, &[UNDER]);
    let player = Vec2::new(0.0, -40.0);
    let npc = Vec2::new(20.0, -150.0);
    let frame = FlightFrame {
        beams: vec![
            super::faked_world::beam(ShipRef::Player, OVER, player, npc),
            super::faked_world::beam(ShipRef::Npc(NpcId(0)), UNDER, npc, player),
        ],
        ..in_sol_with(vec![the_player_at(player, 0.0), lit_npc_at(npc, 180.0)])
    };
    let mut view = world(catalog, frame);
    // Both moving: a step ago each was 10 behind.
    view.previous = ShipPlace {
        position: player + Vec2::new(0.0, 10.0),
        heading: 0.0,
    };
    view.npc_previous = vec![(
        NpcId(0),
        ShipPlace {
            position: npc + Vec2::new(0.0, 10.0),
            heading: 180.0,
        },
    )];
    view.alpha = 0.5;
    let list = world_list(&view);
    let camera = view.camera();
    let player = camera.world_to_screen(view.shown_position());
    let over = beam_starts(&list, Color::rgba(0, 255, 0, 255));
    assert!(!over.is_empty());
    assert!(
        over.iter().all(|&start| start == player),
        "the player's, from the player as drawn: {over:?}"
    );
    let (npc_shown, _) = view.shown_npc(&npc_record(&view, NpcId(0)));
    let under = beam_starts(&list, Color::rgba(0, 0, 255, 255));
    assert!(!under.is_empty());
    let expected = camera.world_to_screen(npc_shown);
    assert!(
        under
            .iter()
            .all(|start| (start.x - expected.x).abs() < 1e-3
                && (start.y - expected.y).abs() < 1e-3),
        "the NPC's, from the NPC as drawn: {under:?} {expected:?}"
    );
}

/// The player's layers drawn in `view`: its ship, its glow and its
/// lights.
fn faked_player_layers(view: &Faked) -> Vec<i16> {
    super::faked_world::faked_ship_sprites(view)
        .iter()
        .map(|(image, _)| image.id)
        .filter(|id| [2000, 2100, 2200].contains(id))
        .collect()
}

#[test]
fn the_players_ship_is_drawn_whole_while_breaking_up_then_without_its_sprite_then_not_at_all() {
    // When its fate draws it so is the session's (nova-sim-combat's
    // player_fate tests, `the_explosion_tick_sounds_the_klaxon_then_halts_it_before_the_explosion`
    // stays real for phase 9).
    let mut catalog = killed_in(1);
    catalog.sheet = blinking(Blink::STEADY).sheet;
    let breaking = nova_sim::flight_frame::FrameShip {
        glow: 12,
        drawn: nova_sim::combat::ship_frames::Drawn {
            lights: Some(32),
            ..nova_sim::combat::ship_frames::Drawn::default()
        },
        frames: nova_sim::combat::ship_frames::FrameSpec {
            lights: true,
            ..nova_sim::combat::ship_frames::FrameSpec::NONE
        },
        ..the_player_at(Vec2::ZERO, 0.0)
    };
    let mut view = world(catalog, in_sol_with(vec![breaking]));
    let ids = faked_player_layers(&view);
    assert!(ids.contains(&2000), "the ship: {ids:?}");
    assert!(ids.contains(&2200), "its lights: {ids:?}");
    let mut frame = view.frame.clone();
    frame.ships[0].shown = nova_sim::Drawn::SpriteHidden;
    shows(&mut view, frame.clone());
    assert_eq!(
        faked_player_layers(&view),
        [2100, 2200],
        "its glow and lights, but no ship, on the explosion's tick"
    );
    frame.ships[0].shown = nova_sim::Drawn::Hidden;
    shows(&mut view, frame);
    assert_eq!(
        faked_player_layers(&view),
        [0_i16; 0],
        "nothing of it after"
    );
}

/// Whether `list` draws `run` as one unbroken run of commands.
fn draws_run(list: &DrawList, run: &DrawList) -> bool {
    let commands: Vec<_> = list.iter().cloned().collect();
    let expected: Vec<_> = run.iter().cloned().collect();
    commands
        .windows(expected.len())
        .any(|window| window == expected)
}

fn sprite_unavailable(list: &DrawList) -> bool {
    list.iter().any(
        |c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with("Sprite unavailable")),
    )
}

#[test]
fn an_exploded_player_without_a_sheet_draws_no_placeholder() {
    let mut catalog = killed_in(5);
    catalog.sheet = Err("no shän 128 for shïp 128".to_owned());
    let mut view = world(catalog, in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    let placeholder = |view: &Faked| {
        let mut boxed = DrawList::new();
        let at = view.camera().world_to_screen(view.shown_position());
        crossed_box(&mut boxed, at, PLACEHOLDER_SIZE, PLACEHOLDER);
        boxed
    };
    let list = world_list(&view);
    assert!(sprite_unavailable(&list), "while breaking up");
    assert!(draws_run(&list, &placeholder(&view)), "{list:?}");
    // The explosion's tick, then the ones after.
    for shown in [nova_sim::Drawn::SpriteHidden, nova_sim::Drawn::Hidden] {
        let mut frame = view.frame.clone();
        frame.ships[0].shown = shown;
        shows(&mut view, frame);
        let list = world_list(&view);
        assert!(!sprite_unavailable(&list), "{list:?}");
        assert!(!draws_run(&list, &placeholder(&view)), "{list:?}");
    }
}

#[test]
fn a_shot_whose_look_cannot_be_read_is_a_crossed_box() {
    let shot = nova_sim::flight_frame::FrameShot {
        velocity: Vec2::new(0.0, -20.0),
        ..super::faked_world::shot_of(0, UNSEEN, Vec2::new(0.0, -20.0), 0.0)
    };
    let view = world(
        armed(0, &[UNSEEN]),
        FlightFrame {
            shots: vec![shot],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
        },
    );
    let list = world_list(&view);
    let mut boxed = DrawList::new();
    // A whole step on, it is drawn a step behind, where it left.
    assert_eq!(view.alpha(), 0.0);
    let left = shot.position - shot.velocity;
    let centre = view.camera().world_to_screen(at(left.x, left.y));
    crossed_box(
        &mut boxed,
        centre,
        weapons::SHOT_PLACEHOLDER_SIZE,
        PLACEHOLDER,
    );
    let commands: Vec<_> = list.iter().cloned().collect();
    let expected: Vec<_> = boxed.iter().cloned().collect();
    assert!(
        commands.windows(expected.len()).any(|run| run == expected),
        "{commands:?}"
    );
}

#[test]
fn a_destroyed_npcs_debris_is_drawn_after_its_explosion() {
    // Its explosion, its debris and the sound heard from the player are
    // stepping.rs's `a_destroyed_ship_explodes_heard_from_the_player_and_scatters_debris`;
    // what destroys it is the session's.
    let mut view = world(
        armed(1, &[BLASTER]),
        in_sol_with(vec![the_player_at(Vec2::new(0.0, -6.0), 0.0)]),
    );
    super::faked_world::raise(
        &mut view,
        &[CombatEvent::Destroyed {
            ship: ShipRef::Npc(NpcId(0)),
            ship_type: ShipId(129),
            at: Vec2::new(0.0, -100.0),
            velocity: Vec2::ZERO,
            explosion: Some(nova_sim::catalog::Explosion {
                boom: BoomId(128),
                extra: false,
            }),
            size: 32.0,
        }],
    );
    let effects = view.effects();
    assert_eq!(effects.explosions().len(), 1);
    assert_eq!(effects.explosions()[0].boom, BoomId(128));
    assert_eq!(effects.particles().len(), 16);
    let list = world_list(&view);
    let explosion = first(&list, sprite_of(400));
    assert_eq!(
        list.iter()
            .skip(explosion)
            .filter(|c| matches!(c, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR))
            .count(),
        16,
        "drawn after the explosion"
    );
}

/// [`armed`]'s NPC, shïp 129, of `DeathDelay` 4 and one crew pod, which
/// one hit of the player's blaster breaks up: the pod comes out on its
/// explosion tick, at a timer of 2.
pub(super) fn podding() -> FakeCatalog {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 200;
    catalog.weapons[0].mass_dmg = 200;
    catalog.hulls[1].death_delay = 4;
    catalog.hulls[1].pod_count = 1;
    catalog
}

#[test]
fn the_target_is_bracketed_and_its_panel_shows_it() {
    let target = FrameTarget {
        name: "Ship 129".to_owned(),
        tag: Some(nova_sim::TargetTag::Code(GovtId(140))),
        ..record::target(0)
    };
    let mut frame = targeted(
        vec![FrameShip {
            govt: Some(GovtId(140)),
            ..lit_npc(180.0)
        }],
        target,
    );
    frame.hud = super::faked_world::targeting(0);
    let view = barred(armed(1, &[]), frame);
    let list = flown(&view);
    let (shown, _) = view.shown_npc(&npc_record(&view, NpcId(0)));
    let Some(Ok(sheet)) = view.npc_sheets.get(&ShipId(129)) else {
        panic!("a sheet");
    };
    let mut expected = DrawList::new();
    brackets::draw_ship(
        &mut expected,
        view.camera().world_to_screen(shown),
        Some((sheet.frame_width, sheet.frame_height)),
        0.0,
        Standing::Neutral,
    );
    let commands: Vec<_> = list.iter().cloned().collect();
    let expected: Vec<_> = expected.iter().cloned().collect();
    assert!(
        commands.windows(4).any(|run| run == expected),
        "{commands:?}"
    );
    let bar = view.status_bar.as_ref().expect("a status bar");
    let mut panel = DrawList::new();
    target::draw_target_panel(
        &mut panel,
        &bar.layout,
        hud::bar_origin(bar, VIEW_SIZE.0),
        Some(&TargetPanel {
            name: "Ship 129".to_owned(),
            subtitle: "Light Transport".to_owned(),
            picture: Some(3001),
            status: nova_sim::TargetStatus::Shield(100),
            tag: Some(Tag::Code("Fed.".to_owned())),
        }),
        None,
    );
    let expected: Vec<_> = panel.iter().cloned().collect();
    assert!(
        commands.windows(expected.len()).any(|run| run == expected),
        "{commands:?}"
    );
}

#[test]
fn the_nav_brackets_draw_under_the_ship_brackets() {
    // Which stellar a slot key selects is the session's.
    let mut view = firing_at_an_npc_with_murk(armed(1, &[BLASTER, UNDER, OVER]), 0);
    let hud = nova_sim::flight_frame::FrameHud {
        nav_brackets: Some(nova_sim::brackets::NavBrackets {
            stellar: StellarId(128),
            position: Vec2::new(0.0, -600.0),
            expand: 0.0,
            dominated: false,
        }),
        ..super::faked_world::targeting(0)
    };
    let list = super::faked_world::world_list_with(&mut view, &hud);
    let nav = first(&list, icon_in(10000..=10007));
    let ship = first(&list, icon_in(10008..=10023));
    let npc = first(&list, sprite_of(2001));
    assert!(npc < nav && nav + 4 == ship, "{npc} {nav} {ship}");
    assert_eq!(
        bracket_icons(&list),
        [10000, 10001, 10002, 10003, 10012, 10013, 10014, 10015]
    );
}

#[test]
fn a_disabled_targets_brackets_are_grey() {
    // That a disabled ship's standing is grey is nova-sim's
    // (`standing_is_disabled_first_then_escort_then_threat_then_neutral`),
    // and its status line (`a_fill_writes_the_targets_name_subtitle_status_and_tag`).
    let target = FrameTarget {
        status: nova_sim::TargetStatus::Disabled,
        ..record::target(0)
    };
    let mut frame = targeted(vec![lit_npc(180.0)], target);
    frame.hud.target_brackets = Some(nova_sim::brackets::TargetBrackets {
        npc: NpcId(0),
        expand: 0.0,
        standing: Standing::Disabled,
    });
    let list = flown(&barred(armed(1, &[]), frame));
    assert_eq!(bracket_icons(&list), [10020, 10021, 10022, 10023]);
    assert!(texts(&list).contains(&target::DISABLED.to_owned()));
}

#[test]
fn targeting_a_ship_on_screen_flies_its_brackets_in_then_settles_them() {
    // The fly-in's expand on each tick is nova-sim's
    // (`a_new_target_on_screen_flies_in_from_256`); the view draws each.
    let mut frame = targeted(vec![lit_npc(180.0)], record::target(0));
    frame.hud = super::faked_world::targeting(0);
    let mut view = barred(armed(1, &[]), frame);
    let corners = |view: &Faked| -> Vec<(i16, Point)> {
        flown(view)
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Picture { image, top_left } if icon_in(10008..=10023)(command) => {
                    Some((image.id, *top_left))
                }
                _ => None,
            })
            .collect()
    };
    let on_screen = |view: &Faked| {
        view.camera()
            .world_to_screen(view.shown_npc(&npc_record(view, NpcId(0))).0)
    };
    for expand in [196.0, 136.0, 76.0, 16.0, 0.0] {
        let mut frame = view.frame.clone();
        if let Some(brackets) = &mut frame.hud.target_brackets {
            brackets.expand = expand;
        }
        shows(&mut view, frame);
        // Ship 129, 40 x 40: half 20; neutral, from 10012.
        assert_eq!(
            corners(&view),
            corners_at(10012, on_screen(&view), 20.0 + expand),
            "{expand}"
        );
    }
}

#[test]
fn a_target_whose_sheet_cannot_be_read_is_bracketed_round_its_crossed_box() {
    let mut npc = lit_npc(90.0);
    npc.class = ShipId(130);
    npc.framed = Some(ShipId(130));
    let mut view = world(
        trafficked(&[130], 1, 130, 3),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), npc]),
    );
    let list = super::faked_world::world_list_with(&mut view, &super::faked_world::targeting(0));
    let corners: Vec<(i16, Point)> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(10008..=10023)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect();
    let target = view
        .camera()
        .world_to_screen(view.shown_npc(&npc_record(&view, NpcId(0))).0);
    // Its 64 x 64 box: half 32.
    assert_eq!(corners, corners_at(10012, target, PLACEHOLDER_SIZE / 2.0));
}

#[test]
fn the_target_panel_holds_a_disabling_until_its_next_timed_redraw() {
    let mut view = faked_targeting_after_a_redraw(&Rulebook::default());
    let held = status_shown(&view);
    disable_the_target(&mut view);
    hud_tick(&mut view);
    assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
    while !now_tick(&view).is_multiple_of(TARGET_PERIOD_TICKS) {
        assert_eq!(status_shown(&view), held, "tick {}", now_tick(&view));
        assert!(!texts(&flown(&view)).contains(&target::DISABLED.to_owned()));
        hud_tick(&mut view);
    }
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    assert!(texts(&flown(&view)).contains(&target::DISABLED.to_owned()));
}

#[test]
fn read_the_bibles_way_the_target_panel_follows_every_frame() {
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, RuleSource::Bible);
    let mut view = faked_targeting_after_a_redraw(&rules);
    disable_the_target(&mut view);
    hud_tick(&mut view);
    assert_ne!(now_tick(&view) % TARGET_PERIOD_TICKS, 0);
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
}

#[test]
fn redrawing_the_status_retakes_the_target_panel() {
    let mut view = faked_targeting_after_a_redraw(&Rulebook::default());
    disable_the_target(&mut view);
    hud_tick(&mut view);
    assert_ne!(status_shown(&view), status_now(&view), "held");
    view.redraw_status();
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
}

#[test]
fn a_change_of_target_shows_at_once() {
    let mut view = barred(
        armed(2, &[]),
        among(vec![lit_npc(180.0), npc_129(1, Vec2::new(-50.0, 0.0), 0.0)]),
    );
    let target = |view: &mut Faked, id: Option<u32>| {
        let mut frame = view.frame.clone();
        frame.target = id.map(record::target);
        shows(view, frame);
    };
    target(&mut view, Some(0));
    hud_tick(&mut view);
    let drawn_for = |view: &Faked| {
        view.target_scene
            .as_ref()
            .and_then(|(drawn, _)| drawn.map(|drawn| drawn.npc))
    };
    assert_eq!(drawn_for(&view), Some(NpcId(0)));
    target(&mut view, Some(1));
    assert_eq!(drawn_for(&view), Some(NpcId(0)), "not yet retaken");
    assert_eq!(
        *view.target_shown(&view.frame),
        view.target_now(&view.frame),
        "shown at once"
    );
    hud_tick(&mut view);
    assert_eq!(drawn_for(&view), Some(NpcId(1)), "retaken");
    target(&mut view, None);
    assert_eq!(*view.target_shown(&view.frame), None, "lost at once");
    let list = texts(&flown(&view));
    assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
}

#[test]
fn a_falling_shield_shows_at_the_radar_refreshes_only() {
    // How fast the shield falls is the fight's; the panel follows it at
    // the radar's refreshes and its own timed redraws.
    let mut view = barred(
        armed(1, &[]),
        targeted(vec![lit_npc(180.0)], record::target(0)),
    );
    let mut shown = status_shown(&view);
    let mut changes = 0;
    let mut held = 0;
    for tick in 0..240_u8 {
        let mut frame = view.frame.clone();
        let falling = 100 - u32::from(tick / 3);
        frame.ships[1].shield.now = f32::from(100 - tick / 3);
        if let Some(target) = &mut frame.target {
            target.status = nova_sim::TargetStatus::Shield(falling);
        }
        shows(&mut view, frame);
        let (blink, before) = (view.hud_cadence.blink_on(), now_tick(&view));
        hud_tick(&mut view);
        let radar = view.hud_cadence.blink_on() != blink;
        let timed = now_tick(&view) / TARGET_PERIOD_TICKS > before / TARGET_PERIOD_TICKS;
        let now = status_shown(&view);
        if now != shown {
            changes += 1;
            assert!(radar || timed, "tick {}: {now:?}", now_tick(&view));
            assert_eq!(now, status_now(&view), "tick {}", now_tick(&view));
        }
        if now != status_now(&view) {
            held += 1;
        }
        shown = now;
    }
    assert!(changes > 1, "the shield fell: {changes}");
    assert!(held > 0, "and was held between refreshes");
}

#[test]
fn a_disabled_player_has_no_fuel_gauge() {
    // When the player is disabled is the session's; the frame's record
    // says so (`a_fill_writes_every_ship_the_player_first_then_each_npc_in_the_systems_order`).
    let mut frame = among(Vec::new());
    frame.hud.reserves = ShipStats::new(FIELDS, &[]).full();
    let mut view = barred(armed(1, &[]), frame);
    let colors = |view: &Faked| -> Vec<Color> {
        lines(&flown(view))
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Line { color, .. } => Some(color),
                _ => None,
            })
            .collect()
    };
    let layout = layout();
    assert!(colors(&view).contains(&layout.fuel_full), "intact");
    let mut frame = view.frame.clone();
    frame.ships[0].condition = Condition::Disabled;
    shows(&mut view, frame);
    let shown = colors(&view);
    assert!(!shown.contains(&layout.fuel_full), "{shown:?}");
    assert!(!shown.contains(&layout.fuel_partial), "{shown:?}");
    assert!(shown.contains(&layout.armor_color), "{shown:?}");
}

#[test]
fn the_hud_shows_no_target_and_the_secondary_weapon() {
    // Which secondary a press selects on the next step is nova-sim's
    // (`a_select_press_goes_in_weap_id_order_and_backwards_with_option`,
    // `weapon_select_answers_done_and_the_next_steps_pass_takes_it`).
    let mut view = barred(armed(0, &[BLASTER, ROCKET]), among(Vec::new()));
    let list = texts(&flown(&view));
    assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
    assert!(list.contains(&NO_SECONDARY.to_owned()), "{list:?}");
    view.press_weapon_select();
    assert_eq!(
        sent(&view),
        [FlightCommand::SelectWeapon { backwards: false }]
    );
    let mut frame = view.frame.clone();
    frame.hud.secondary = Some(ROCKET);
    shows(&mut view, frame);
    let list = texts(&flown(&view));
    assert!(list.contains(&"Rocket".to_owned()), "{list:?}");
}

#[test]
fn the_panel_is_laid_out_by_the_metrics_given() {
    let mut view = barred(armed(0, &[BLASTER]), among(Vec::new()));
    view.metrics = Some(Metrics(Rc::new(crate::text::fixture::MonoMetrics)));
    let list = flown(&view);
    let origin = list
        .iter()
        .find_map(|c| match c {
            DrawCommand::Text { text, origin, .. } if text == NO_TARGET => Some(*origin),
            _ => None,
        })
        .expect("No Target");
    // Centred: 9 characters of 6 in the 176 across from 838.
    assert_eq!(origin.x, 899.0);
}

#[test]
fn the_metrics_debug_as_their_name() {
    let metrics = Metrics(Rc::new(crate::text::fixture::MonoMetrics));
    assert_eq!(format!("{metrics:?}"), "Metrics");
}

#[test]
fn the_help_line_names_the_combat_keys() {
    for name in ["Space", "Ctrl", "W", "Tab", "R:"] {
        assert!(HELP.contains(name), "{name}: {HELP}");
    }
}

// Point defence.

#[test]
fn a_missile_shot_down_is_drawn_exploding_and_unheard() {
    // What point defence shoots down is the session's.
    let mut view = world(
        defending(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    super::faked_world::raise(
        &mut view,
        &[CombatEvent::ShotDown {
            at: Vec2::new(0.0, -50.0),
        }],
    );
    let explosion = world_list(&view)
        .iter()
        .any(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 400));
    assert!(explosion, "bööm 128 drawn");
    let sounds: Vec<CombatSound> = view
        .drained_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Combat(sound) => Some(sound),
            _ => None,
        })
        .collect();
    assert!(
        sounds
            .iter()
            .all(|sound| sound.sound != nova_sim::SoundId(302)),
        "{sounds:?}"
    );
}

#[test]
fn a_ship_the_player_shoves_is_drawn_moved_on_the_frame_of_the_hit() {
    // The push lands in the ship's velocity before `_HandleShip` moves it,
    // so the frame after the pass has it moved: the session's (nova-sim's
    // `session::game_work::tests::an_npc_pushed_by_a_shot_moves_by_the_push_in_the_tick_it_lands`).
    // The view draws it on its way from where it was to where the frame
    // has it.
    let mut view = world(
        armed(1, &[BLASTER]),
        in_sol_with(vec![
            the_player_at(Vec2::ZERO, 0.0),
            lit_npc_at(Vec2::new(0.0, -100.0), 180.0),
        ]),
    );
    let before = sprites(&world_list(&view));
    let mut shoved = view.frame.clone();
    shoved.ships[1].position = Vec2::new(0.0, -130.0);
    view.sim.set_frame(shoved);
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            before: vec![
                (ShipRef::Player, ShipPlace::default()),
                (
                    ShipRef::Npc(NpcId(0)),
                    ShipPlace {
                        position: Vec2::new(0.0, -100.0),
                        heading: 180.0,
                    },
                ),
            ],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, view.anchor());
    view.alpha = 0.5;
    assert_ne!(sprites(&world_list(&view)), before, "drawn moved");
}

/// A view by `rules` whose target, NPC 0, intact, was taken just after a
/// timed redraw of the panel, its panel drawn a tick later.
fn faked_targeting_after_a_redraw(rules: &Rulebook) -> Faked {
    let mut view = barred_by(armed(1, &[]), among(vec![lit_npc(180.0)]), rules);
    while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
        hud_tick(&mut view);
    }
    let mut frame = view.frame.clone();
    frame.target = Some(record::target(0));
    shows(&mut view, frame);
    hud_tick(&mut view);
    assert_eq!(status_shown(&view), status_now(&view));
    assert!(
        matches!(status_shown(&view), Some(nova_sim::TargetStatus::Shield(_))),
        "{:?}",
        status_shown(&view)
    );
    view
}

/// The target, NPC 0, disabled from now on, as the session shows it.
fn disable_the_target(view: &mut Faked) {
    let mut frame = view.frame.clone();
    frame.ships[1].condition = Condition::Disabled;
    if let Some(target) = &mut frame.target {
        target.status = nova_sim::TargetStatus::Disabled;
    }
    shows(view, frame);
}
