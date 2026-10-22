use super::*;

// Traffic.

fn npc_states(view: &View) -> Vec<ShipState> {
    let session = view.session().expect("flying");
    session.npcs().iter().map(|npc| npc.state).collect()
}

#[test]
fn the_traffic_is_populated_when_the_flight_starts() {
    let (script, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    assert_eq!(npc_ids(&view), [], "not until the first tick");
    view.tick(TICK);
    let session = view.session().expect("flying");
    let npc = &session.npcs()[0];
    assert_eq!((npc.ship, npc.goal), (ShipId(129), Goal::Idle));
    assert_eq!(npc.state.position, Vec2::new(100.0, -100.0));
    assert_eq!(&script.borrow().asked[..7], [7, 7, 100, 1, 1500, 1500, 360]);
    ticks(&mut view, 3);
    assert_eq!(npc_ids(&view), [NpcId(0)], "populated once");
    let npc = &view.session().expect("flying").npcs()[0];
    assert_eq!(npc.goal, Goal::Idle, "deciding as the behaviour given says");
    assert_eq!(
        npc.state.position,
        Vec2::new(118.0, -100.0),
        "gliding right at its top speed, 6 a tick"
    );
}

#[test]
fn an_npc_is_drawn_with_its_own_sprite_where_it_is_facing_its_heading() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let list = drawn(&view);
    let frame = rotation_frame(90.0, NonZeroU16::new(72).expect("non-zero"));
    assert_eq!(
        sprites(&list),
        [
            (ImageKey::sprite(1128, 0), at(512.0, -216.0)),
            (ImageKey::sprite(1129, 1), at(812.0, 184.0)),
            (ImageKey::sprite(2001, frame), at(612.0, 284.0)),
            (ImageKey::sprite(2101, frame), at(612.0, 284.0)),
            (ImageKey::sprite(2201, frame), at(612.0, 284.0)),
            (ImageKey::sprite(2000, 0), VIEW_CENTER),
        ],
        "after the stellars, before the player, its glow and lights over it"
    );
}

#[test]
fn a_ship_waiting_at_its_gate_is_drawn_by_its_tint_and_any_ship_by_its_cloak() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let session = view.session().expect("flying");
    let mut npc = session.npcs()[0].clone();
    let opaque = CloakShown::Opaque;
    assert_eq!(session.cloak_shown(&npc), opaque, "flying");
    npc.mode = nova_sim::traffic::npc::Mode::JumpingIn { tick: 0 };
    assert_eq!(session.cloak_shown(&npc), opaque, "jumping in");
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
    assert_eq!(session.cloak_shown(&npc), opaque, "fading in at its gate");
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 30 };
    assert_eq!(session.cloak_shown(&npc), opaque);
    assert_eq!(npc_tint(&npc), None, "hidden at its gate");
    // Held there by the player's hit, its goal is 4, not 0x15, so
    // `_HandleShipDisplay`'s fade (@0x2c090) passes it by.
    npc.mode = nova_sim::traffic::npc::Mode::HeldAtGate { ticks_left: 3 };
    assert_eq!(npc_tint(&npc), Some(Color::WHITE), "held: drawn plain");
    assert_eq!(session.cloak_shown(&npc), opaque, "held");
    npc.cloak = CLOAKED;
    npc.mode = nova_sim::traffic::npc::Mode::Emerging { ticks_left: 3 };
    assert_eq!(
        session.cloak_shown(&npc),
        CloakShown::Hidden,
        "cloaked at its gate"
    );
}

#[test]
fn a_ship_the_player_cannot_see_for_its_cloak_is_not_drawn() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let session = view.session().expect("flying");
    let mut npc = session.npcs()[0].clone();
    npc.cloak = CLOAKED;
    assert_eq!(session.cloak_shown(&npc), CloakShown::Hidden, "cloaked");
    npc.cloak.direction = -1;
    npc.cloak.fade = 8.0;
    assert_eq!(
        session.cloak_shown(&npc),
        CloakShown::Opaque,
        "faded out enough to be seen"
    );
}

#[test]
fn a_cloaked_ship_is_on_the_radar_only_when_the_session_says() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let session = view.session().expect("flying");
    let npc = session.npcs()[0].clone();
    let blip = RadarShip {
        position: at(100.0, -100.0),
        targeted: false,
        iff: ShipIff::Other,
        heavy: false,
    };
    let at_blip = at(100.0, -100.0);
    assert_eq!(radar_ships(session, &[(&npc, at_blip, 0.0)]), [blip]);
    let mut cloaked = npc.clone();
    cloaked.cloak = CLOAKED;
    assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), []);
    cloaked.cloak_gear.on_radar = true;
    assert_eq!(radar_ships(session, &[(&cloaked, at_blip, 0.0)]), [blip]);
}

#[test]
fn the_play_area_is_the_screen_left_of_the_status_bar_about_the_ship() {
    let area = play_area(VIEW_SIZE);
    assert_eq!(
        area,
        nova_sim::cloak::Area {
            left: -512.0,
            top: -384.0,
            right: 1024.0 - 194.0 - 512.0,
            bottom: 384.0,
        }
    );
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    assert_eq!(view.session().expect("flying").play_area(), area, "told");
}

#[test]
fn an_npc_is_a_dim_blip_on_the_radar() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    // (100, -100) is 3.125 pixels each way: (929.125, 92.875).
    let blip = RadarMark::Pixel {
        x: 929,
        y: 92,
        color: RadarColor::Dim,
    };
    assert_eq!(blips(&view), [blip]);
    let list = drawn(&view);
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
fn an_npc_coming_out_of_a_gate_is_hidden_then_fades_in_but_always_on_the_radar() {
    let mut catalog = trafficked(&[130], 1, 129, 3);
    catalog.sites.push(LandingSite {
        flags2: nova_sim::gate::HYPERGATE,
        exit_angle: 90,
        ..site(141, (200.0, 100.0), StellarFlags::CAN_LAND)
    });
    // Setup brings nobody (an empty person roll); then an arrival
    // comes out of the gate, the third of the stellars.
    let (script, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    ticks(&mut view, 2);
    assert_eq!(&script.borrow().asked[1..9], [500, 7, 7, 100, 1, 360, 3, 3]);
    let mode = |view: &View| view.session().expect("flying").npcs()[0].mode;
    assert_eq!(mode(&view), Mode::Emerging { ticks_left: 59 });
    ticks(&mut view, 19);
    assert_eq!(mode(&view), Mode::Emerging { ticks_left: 40 });
    let list = drawn(&view);
    assert_eq!(npc_tints(&list), [], "hidden");
    assert_eq!(blips(&view).len(), 1, "on the radar all the same");
    ticks(&mut view, 32);
    assert_eq!(mode(&view), Mode::Emerging { ticks_left: 8 });
    let list = drawn(&view);
    assert_eq!(
        npc_tints(&list),
        [Color::rgba(255, 255, 255, 128)],
        "half in"
    );
    assert_eq!(blips(&view).len(), 1);
    ticks(&mut view, 8);
    assert_eq!(mode(&view), Mode::LeavingGate { tick: 0 });
    assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE]);
}

/// Idles, and answers a hit on itself by taking its attacker.
#[derive(Debug)]
struct Answering;

impl Behaviour for Answering {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn react(
        &self,
        npc: &nova_sim::Npc,
        strike: &nova_sim::Strike,
        _around: &nova_sim::Surroundings,
    ) -> nova_sim::Reaction {
        if strike.ship == nova_sim::ShipRef::Npc(npc.id) {
            nova_sim::Reaction {
                target: strike.by,
                provoked: strike.damage,
                goal: None,
            }
        } else {
            nova_sim::Reaction::default()
        }
    }
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
    let mut govt = damaging().govts.remove(0);
    govt.penalties.shoot = 5;
    catalog.govts = vec![govt];
    catalog
}

#[test]
fn a_ship_the_player_hits_in_its_gate_is_drawn_on_the_frame_of_the_hit() {
    // The original answers the hit inside `_HandleShot` (@0x44385),
    // before the display pass draws the frame.
    let mut catalog = volleying();
    catalog.sites.push(LandingSite {
        flags2: nova_sim::gate::HYPERGATE,
        exit_angle: 90,
        ..site(141, (0.0, -100.0), StellarFlags::CAN_LAND)
    });
    // Setup brings nobody; then an arrival comes out of the gate, the
    // third of the stellars, straight ahead of the player.
    let (_, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Answering));
    ticks(&mut view, 2);
    let npc = |view: &View| view.session().expect("flying").npcs()[0].clone();
    assert!(matches!(npc(&view).mode, Mode::Emerging { .. }));
    let left = |npc: &nova_sim::Npc| npc.reserves.shield.now + npc.reserves.armor.now;
    let mut shield = left(&npc(&view));
    view.input(&key(FIRE_KEY, true));
    for _ in 0..60 {
        view.tick(TICK);
        let hit = npc(&view);
        if left(&hit) < shield {
            assert!(
                matches!(hit.mode, Mode::HeldAtGate { .. }),
                "{:?}",
                hit.mode
            );
            assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE], "seen");
            return;
        }
        assert!(matches!(hit.mode, Mode::Emerging { .. }), "{:?}", hit.mode);
        shield = left(&hit);
    }
    panic!("never hit");
}

/// [`volleying`] with hypergate 141 at (0, -110), straight ahead of the
/// player, `with` changes made. A shot hits on the pass after the one it
/// moves into its target on, and at this distance that move falls on a
/// frame's advance pass, so the hit falls on its second.
fn gate_110_ahead(with: impl FnOnce(&mut FakeCatalog)) -> FakeCatalog {
    let mut catalog = volleying();
    catalog.sites.push(LandingSite {
        flags2: nova_sim::gate::HYPERGATE,
        exit_angle: 90,
        ..site(141, (0.0, -110.0), StellarFlags::CAN_LAND)
    });
    with(&mut catalog);
    catalog
}

/// `catalog`'s flight in double time, `draws` bringing an arrival out
/// of gate 141, which answers a hit by taking its attacker; fired at
/// until the frame on which the first hit lands, which, as a copy of
/// the session run through each frame's passes shows, is that frame's
/// second pass. Until then the arrival is hidden in the gate.
fn hit_on_the_second_pass(catalog: FakeCatalog, draws: &[u32]) -> View {
    let (_, chance) = scripted(draws);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Answering))
        .with_double_time(true);
    ticks(&mut view, 2);
    let npc = |view: &View| view.session().expect("flying").npcs()[0].clone();
    assert!(matches!(npc(&view).mode, Mode::Emerging { .. }));
    let left = |npc: &nova_sim::Npc| npc.reserves.shield.now + npc.reserves.armor.now;
    let shield = left(&npc(&view));
    // Whether a frame from `session`, run on a copy, hits the NPC on
    // its advance pass, and by the end of its second.
    let passes = |session: &Session, catalog: &FakeCatalog| {
        let mut copy = session.clone();
        copy.game_work(
            Controls::default(),
            catalog,
            &Answering,
            Rules::default(),
            &mut NeverFires,
        );
        let advance = left(&copy.npcs()[0]) < shield;
        copy.tick_again(
            Controls::default(),
            &Answering,
            Rules::default(),
            &mut NeverFires,
        );
        (advance, left(&copy.npcs()[0]) < shield)
    };
    view.input(&key(FIRE_KEY, true));
    for frame in 0..60 {
        let ahead = passes(view.session().expect("flying"), view.catalog());
        view.tick(TICK);
        if left(&npc(&view)) < shield {
            assert_eq!(ahead, (false, true), "frame {frame}: the second pass's hit");
            return view;
        }
        let mode = npc(&view).mode;
        assert!(matches!(mode, Mode::Emerging { .. }), "{mode:?}");
        assert_eq!(npc_tints(&drawn(&view)), [], "hidden");
    }
    panic!("never hit");
}

#[test]
fn a_ship_the_player_hits_in_its_gate_on_the_second_pass_is_drawn_on_the_frame_of_the_hit() {
    // `_HandleShot` (@0x44385) runs on double time's second pass too,
    // with `_DamageShip`'s reaction, so the hit is answered there,
    // before the next frame is drawn. Setup brings nobody; then an
    // arrival comes out of the gate, the third of the stellars.
    let view = hit_on_the_second_pass(gate_110_ahead(|_| ()), &[0, 0, 6, 6, 0, 0, 0, 2]);
    let mode = view.session().expect("flying").npcs()[0].mode;
    assert!(matches!(mode, Mode::HeldAtGate { .. }), "{mode:?}");
    assert_eq!(npc_tints(&drawn(&view)), [Color::WHITE], "seen");
}

#[test]
fn a_person_the_player_hits_in_its_gate_on_the_second_pass_says_its_quote_on_that_frame() {
    // `_DamageShip`'s reaction on the second pass turns it on the
    // player, and `_AIMakeShipAttackPlayer` says its quote there
    // (`_ShowPersonHailQuote` @0x89ce1), before the next frame.
    let catalog = gate_110_ahead(|catalog| {
        let ace = peopled();
        catalog.persons = ace.persons;
        catalog.persons[0].link_syst = -1;
        catalog.persons[0].flags = nova_sim::person::QUOTE_ON_ATTACK;
        catalog.strings = ace.strings;
    });
    // Setup's person roll comes up empty (`përs` 128); then Ace,
    // `përs` 600, comes out of the gate, the third of the stellars.
    let view = hit_on_the_second_pass(catalog, &[0, 0, 0, 0, 472, 0, 2]);
    let npc = &view.session().expect("flying").npcs()[0];
    assert!(
        matches!(npc.mode, Mode::HeldAtGate { .. }),
        "{:?}",
        npc.mode
    );
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

/// Sends every NPC into hypergate 141.
#[derive(Debug)]
struct Entering;

impl Behaviour for Entering {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::EnterGate(nova_sim::catalog::StellarId(141))
    }
}

#[test]
fn a_ship_entering_a_gate_fades_out_by_two_32nds_a_tick() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let mut npc = view.session().expect("flying").npcs()[0].clone();
    let gate = nova_sim::catalog::StellarId(141);
    let entering = |ticks_left| Mode::EnteringGate { gate, ticks_left };
    npc.mode = entering(15);
    assert_eq!(
        npc_tint(&npc),
        Some(Color::rgba(255, 255, 255, 239)),
        "30/32"
    );
    npc.mode = entering(8);
    assert_eq!(
        npc_tint(&npc),
        Some(Color::rgba(255, 255, 255, 128)),
        "half"
    );
    npc.mode = entering(1);
    assert_eq!(npc_tint(&npc), Some(Color::rgba(255, 255, 255, 16)), "2/32");
    npc.mode = entering(16);
    assert_eq!(npc_tint(&npc), Some(Color::WHITE));
}

#[test]
fn a_hypergate_animates_while_an_npc_heads_in_to_enter_it_and_the_npc_fades_into_it() {
    let busy = with_hypergate(trafficked(&[130], 1, 129, 3));
    // Setup places the ship at (700, 700), far from the gate.
    let (_, chance) = scripted(&placed(1450, 1450, 90));
    let mut view = FlightView::new(busy)
        .with_chance(chance)
        .with_behaviour(Rc::new(Entering));
    assert_eq!(gate_frames(&drawn(&view)), [0], "closed at first");
    let mut heading_in = Vec::new();
    let mut tints = Vec::new();
    let mut after = Vec::new();
    for _ in 0..2000 {
        ticks(&mut view, 1);
        let list = drawn(&view);
        let frames = gate_frames(&list);
        match view
            .session()
            .expect("flying")
            .npcs()
            .first()
            .map(|npc| npc.mode)
        {
            Some(Mode::Flying) => heading_in.extend(frames),
            Some(Mode::EnteringGate { .. }) => tints.extend(npc_tints(&list)),
            Some(_) => {}
            None => after.extend(frames),
        }
    }
    assert_eq!(heading_in.first(), Some(&0), "closed while it is far");
    assert!(heading_in.iter().any(|&frame| frame > 0), "{heading_in:?}");
    assert_eq!(tints.len(), 15, "drawn for 15 frames: {tints:?}");
    assert_eq!(tints.first(), Some(&Color::rgba(255, 255, 255, 239)));
    assert_eq!(tints.last(), Some(&Color::rgba(255, 255, 255, 16)));
    assert_eq!(after.last(), Some(&0), "closed once it is gone");
}

#[test]
fn a_hypergate_animates_while_an_npc_waits_in_it_and_rests_otherwise() {
    let busy = with_hypergate(trafficked(&[130], 1, 129, 3));
    // As in the test above: an arrival comes out of the gate.
    let (_, chance) = scripted(&[0, 0, 6, 6, 0, 0, 0, 2]);
    let mut view = FlightView::new(busy)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    assert_eq!(gate_frames(&drawn(&view)), [0], "closed at first");
    ticks(&mut view, 2);
    let mode = |view: &View| view.session().expect("flying").npcs()[0].mode;
    assert_eq!(mode(&view), Mode::Emerging { ticks_left: 59 });
    let mut frames = Vec::new();
    while matches!(mode(&view), Mode::Emerging { .. }) {
        frames.extend(gate_frames(&drawn(&view)));
        ticks(&mut view, 1);
    }
    assert_eq!(frames[..3], [1, 2, 3], "opening a frame a tick");
    assert!(frames.iter().all(|&frame| frame > 0), "{frames:?}");
    assert!(frames[3..].iter().all(|&frame| frame >= 3), "{frames:?}");
    assert!(frames.contains(&5), "its loop plays: {frames:?}");

    let mut quiet = FlightView::new(with_hypergate(catalog()));
    for _ in 0..60 {
        ticks(&mut quiet, 1);
        assert_eq!(gate_frames(&drawn(&quiet)), [0], "no ship in it");
    }
}

#[test]
fn an_npc_is_drawn_between_its_last_two_steps() {
    // At (0, 100) facing up, it heads for Earth, straight up.
    let mut draws = placed(750, 850, 0).to_vec();
    draws.push(0);
    let (_, chance) = scripted(&draws);
    let mut view = FlightView::new(trafficked(&[130], 1, 129, 1)).with_chance(chance);
    // The first step sets the system up; the NPC moves from the next.
    view.tick(TICK);
    assert_eq!(npc_states(&view)[0].position, Vec2::new(0.0, 100.0));
    view.tick(TICK + TICK / 2);
    let npc = &view.session().expect("flying").npcs()[0];
    assert_eq!(
        npc.goal,
        Goal::Land(StellarId(128)),
        "Nova's AI, by default: a trader's business"
    );
    let moved = npc.state.position.y - 100.0;
    assert!(moved < 0.0, "{npc:?}");
    let alpha = view.alpha();
    assert!(alpha > 0.4, "{alpha}");
    let shown = sprites(&drawn(&view))[2].1;
    let expected = view
        .camera()
        .world_to_screen(at(0.0, moved.mul_add(alpha, 100.0)));
    assert!(
        (shown.y - expected.y).abs() < 1e-3,
        "{shown:?} {expected:?}"
    );
    assert_eq!(shown.x, expected.x);
}

#[test]
fn an_npc_whose_sheet_cannot_be_read_is_a_crossed_box() {
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(trafficked(&[130], 1, 130, 3))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let list = drawn(&view);
    let mut boxed = DrawList::new();
    crossed_box(&mut boxed, at(612.0, 284.0), PLACEHOLDER_SIZE, PLACEHOLDER);
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
    let mut view = FlightView::new(trafficked(&[130, 131], 3, 129, 1));
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    ticks(&mut view, 5);
    assert_eq!(npc_ids(&view).len(), 3);
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)]
    );
    jump_to_alpha(&mut view);
    assert!(!npc_ids(&view).is_empty());
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "not again"
    );
}

#[test]
fn arriving_repopulates_from_the_new_system() {
    let mut view = FlightView::new(trafficked(&[131], 2, 129, 1));
    view.tick(TICK);
    assert_eq!(npc_ids(&view), []);
    jump_to_alpha(&mut view);
    assert_eq!(npc_ids(&view).len(), 2);
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "read on arrival"
    );
}

#[test]
fn the_traffic_flies_on_through_the_whole_warp_and_the_ship_moves_off() {
    // The traffic follows the ship out, then makes for planet 128 at
    // the centre as the ship jumps.
    let catalog = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..trafficked(&[130], 2, 129, 1)
    };
    let jumped = Rc::new(std::cell::Cell::new(false));
    let landing = Goal::Land(StellarId(128));
    let mut view = FlightView::new(catalog)
        .with_behaviour(Rc::new(InspectingUntil(Rc::clone(&jumped), landing)));
    plot(&mut view, 131);
    ready_to_jump(&mut view);
    jumped.set(true);
    view.input(&key(JUMP, true));
    assert!(view.jump_effect().is_some(), "jumping on J");
    let leaving = player(&view);
    let start = npc_states(&view);
    assert_eq!(start.len(), 2);
    view.tick(ms(500));
    assert!(view.session().expect("flying").in_warp_lead_in());
    assert_ne!(npc_states(&view), start, "the traffic flies on");
    while view.session().expect("flying").in_warp_lead_in() {
        view.tick(TICK);
    }
    let past = npc_states(&view);
    view.tick(ms(150));
    assert_ne!(npc_states(&view), past, "past the lead-in too");
    assert_ne!(
        player(&view).position,
        leaving.position,
        "the ship moves off"
    );
    assert_eq!(view.session().expect("flying").system(), SystemId(130));
    tick_to_arrival(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
}

#[test]
fn npcs_stand_still_while_the_map_is_open_or_the_ship_is_landed() {
    let catalog = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..trafficked(&[130], 2, 129, 1)
    };
    let mut view = FlightView::new(catalog);
    ticks(&mut view, 3);
    let before = npc_states(&view);
    assert_eq!(before.len(), 2);
    tap(&mut view, MAP);
    ticks(&mut view, 10);
    assert_eq!(npc_states(&view), before, "the map is open");
    tap(&mut view, MAP);
    land_now(&mut view);
    assert!(view.take_landing().is_some());
    let before = npc_states(&view);
    ticks(&mut view, 10);
    assert_eq!(npc_states(&view), before, "landed");
}

#[test]
fn taking_off_repopulates_the_system() {
    let catalog = FakeCatalog {
        sites: vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)],
        ..trafficked(&[130], 2, 129, 1)
    };
    let mut view = FlightView::new(catalog);
    view.tick(TICK);
    assert_eq!(npc_ids(&view), [NpcId(0), NpcId(1)]);
    land_now(&mut view);
    view.take_off().expect("took off");
    assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "before any tick");
    view.tick(TICK);
    assert_eq!(npc_ids(&view), [NpcId(2), NpcId(3)], "populated once");
}

/// [`trafficked`] with one ship 129 of AI 3 in 130, over planet 128
/// at the centre.
/// [`trafficked`] with the ship over an uninhabited 128, which it
/// lands on in one press, with no tick to bring the traffic in.
fn trafficked_over_128() -> FakeCatalog {
    FakeCatalog {
        sites: vec![site(
            128,
            (0.0, 0.0),
            StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
        )],
        ..trafficked(&[130], 1, 129, 3)
    }
}

/// Whether `list` draws a sprite at world point `world`.
fn sprite_at(view: &View, list: &DrawList, world: Vec2) -> bool {
    let screen = view.camera().world_to_screen(point(world));
    sprites(list).iter().any(|&(_, center)| center == screen)
}

#[test]
fn the_first_frame_after_take_off_draws_none_of_the_ships_from_before_the_landing() {
    let mut draws = placed(850, 650, 90).to_vec();
    // The take-off's escort-strength seed, then its population.
    draws.push(0);
    draws.extend(placed(650, 850, 90));
    let (_, chance) = scripted(&draws);
    let mut view = FlightView::new(trafficked_over_128())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    view.tick(TICK);
    let before = npc_states(&view)[0].position;
    assert_eq!(before, Vec2::new(100.0, -100.0));
    tap(&mut view, LAND);
    view.take_off().expect("took off");
    let after = npc_states(&view)[0].position;
    assert_eq!(after, Vec2::new(-100.0, 100.0));

    let list = drawn(&view);
    assert!(!sprite_at(&view, &list, before), "with no tick");
    assert!(sprite_at(&view, &list, after), "with no tick");

    let states = npc_states(&view);
    view.tick(TICK / 4);
    assert_eq!(npc_states(&view), states, "no step");
    let list = drawn(&view);
    assert!(!sprite_at(&view, &list, before), "after a frame of no step");
    assert!(sprite_at(&view, &list, after), "after a frame of no step");
}

#[test]
fn a_ship_type_first_flown_at_take_off_is_drawn_with_its_sprite_at_once() {
    // The take-off's escort-strength seed, then its population.
    let mut draws = vec![0];
    draws.extend(placed(650, 850, 90));
    let (_, chance) = scripted(&draws);
    let mut view = FlightView::new(trafficked_over_128())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still));
    tap(&mut view, LAND);
    assert!(view.take_landing().is_some(), "landed before any traffic");
    view.take_off().expect("took off");
    let after = npc_states(&view)[0].position;
    let list = drawn(&view);
    let mut boxed = DrawList::new();
    crossed_box(
        &mut boxed,
        view.camera().world_to_screen(point(after)),
        PLACEHOLDER_SIZE,
        PLACEHOLDER,
    );
    let commands: Vec<_> = list.iter().cloned().collect();
    let expected: Vec<_> = boxed.iter().cloned().collect();
    assert!(
        !commands.windows(expected.len()).any(|run| run == expected),
        "no placeholder"
    );
    assert!(sprite_at(&view, &list, after), "its own sprite");
}

// Combat.

#[test]
fn the_fight_asks_the_disable_rule_given_each_step_in_flight() {
    let rule = Rc::new(Counting::default());
    let uninhabited = StellarFlags::CAN_LAND | StellarFlags::UNINHABITED;
    let mut view =
        flight_among(vec![site(140, (0.0, 0.0), uninhabited)]).with_disable_rule(rule.clone());
    view.tick(TICK);
    assert_eq!(rule.asked.get(), 1, "the player, the only ship");
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 3);
    view.input(&key(MAP_KEY, true));
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 3, "not while the map is open");
    view.input(&key(MAP_KEY, true));
    tap(&mut view, LAND);
    assert!(view.take_landing().is_some(), "one press, uninhabited");
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 3, "not while landed");
}

#[test]
fn the_sessions_diagnostics_come_through_the_screen() {
    let exclusive = nova_sim::WeaponRecord {
        id: nova_sim::WeaponId(181),
        reload: 20,
        count: 12,
        mass_dmg: 2,
        energy_dmg: 2,
        guidance: -1,
        speed: 1000,
        ammo_type: -1,
        graphic: 0,
        inaccuracy: 0,
        impact: 5,
        explod_type: -1,
        prox_radius: 5,
        blast_radius: 6,
        flags: 0,
        seeker: 0,
        flags2: 0,
        flags3: 0x0020,
        decay: 0,
        beam_length: 0,
        beam_width: 0,
        falloff: 0,
        li_density: 0,
        burst_count: 0,
        burst_reload: 0,
        guided_turn: 0,
        durability: 0,
        sub_count: 0,
        sub_type: None,
        sub_theta: 0,
        sub_limit: 0,
        max_ammo: 0,
        prox_safety: 0,
        recoil: 0,
        exit_type: -1,
        ionization: 0,
        ionize_color: 0,
        jam_vuln: [0; 4],
        frame: None,
        hit_particles: 0,
        hit_part_life: 0,
        hit_part_vel: 0,
        hit_part_color: 0,
    };
    let hull = nova_sim::HullRecord {
        id: ShipId(129),
        flags: 0,
        flags2: 0,
        flags3: 0,
        death_delay: 0,
        pod_count: 0,
        explode1: -1,
        explode2: -1,
        mass: 0,
        weapons: vec![nova_sim::StockWeapon {
            weapon: nova_sim::WeaponId(181),
            count: 1,
            ammo: 0,
        }],
        size: None,
        height: None,
        strength: 0,
        key_carried: None,
        cost: 0,
        speed: 0,
        exits: None,
        display: None,
        escort_class: nova_sim::EscortClass::Freighter,
    };
    let (_, chance) = scripted(&placed(850, 650, 90));
    let catalog = FakeCatalog {
        weapons: vec![exclusive],
        hulls: vec![hull],
        looks: vec![(181, Ok(WeaponLook::default()))],
        ..trafficked(&[130], 1, 129, 3)
    };
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Firing));
    assert_eq!(view.take_diagnostics(), []);
    ticks(&mut view, 3);
    assert_eq!(
        view.take_diagnostics(),
        [Diagnostic::Sim(SimDiagnostic::UnimplementedWeaponFlag {
            weapon: nova_sim::WeaponId(181),
            field: nova_sim::combat::flags::FlagField::Flags3,
            bit: 0x0020
        })]
    );
    ticks(&mut view, 30);
    assert_eq!(view.take_diagnostics(), [], "once");
}

#[test]
fn a_look_whose_sheet_cannot_be_read_still_sounds_and_is_reported_once() {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.looks[0].1 = Ok(WeaponLook {
        sheet: Some(Err("no spïn 3005".to_owned())),
        ..look("Blaster", 208, None)
    });
    catalog.booms.push((
        129,
        Ok(BoomLook {
            sheet: Err("no spïn 401".to_owned()),
            advance: 1.0,
            sound: None,
        }),
    ));
    let mut view = fighting(catalog, &[]);
    assert_eq!(
        view.take_diagnostics(),
        [
            Diagnostic::Unreadable("wëap 128: no spïn 3005".to_owned()),
            Diagnostic::Unreadable("no wëap 150".to_owned()),
            Diagnostic::Unreadable("bööm 129: no spïn 401".to_owned()),
        ],
        "and the unseen weapon, which has no look at all"
    );
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    let sounds: Vec<_> = combat_sounds(&mut view).iter().map(|s| s.sound).collect();
    assert_eq!(sounds, [nova_sim::SoundId(208)], "the blaster is heard");
    ticks(&mut view, 3);
    assert_eq!(view.take_diagnostics(), [], "once");
}

// Combat controls and display.

#[test]
fn tab_targets_the_next_npc_and_alt_r_the_nearest() {
    let mut view = fighting(armed(2, &[]), &two_npcs());
    assert_eq!(target_of(&view), None);
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), Some(NpcId(0)));
    view.input(&held(Key::Tab));
    assert_eq!(target_of(&view), Some(NpcId(0)), "a repeat does nothing");
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), None, "past the last");
    alt_tap(&mut view, NEAREST_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    tap(&mut view, Key::Tab);
    view.input(&key(Key::Alt, true));
    view.input(&held(NEAREST_KEY));
    assert_eq!(target_of(&view), None, "a repeat does nothing");
    assert_eq!((TARGET_KEY, NEAREST_KEY), (Key::Tab, Key::Char('r')));
}

#[test]
fn a_target_key_press_beeps_once_and_its_repeat_is_silent() {
    let mut view = fighting(armed(2, &[]), &two_npcs());
    view.take_sounds();
    view.input(&key(Key::Tab, true));
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::TargetSelected)]);
    view.input(&held(Key::Tab));
    view.input(&key(Key::Tab, false));
    assert_eq!(view.take_sounds(), [], "a repeat of Tab");
    alt_tap(&mut view, NEAREST_KEY);
    view.take_sounds();
    alt_tap(&mut view, NEAREST_KEY);
    assert_eq!(view.take_sounds(), [], "a fresh Alt-R on the same ship");
    // No ship threatens, so an acting R would beep the refusal.
    let before = target_of(&view);
    view.input(&held(NEAREST_KEY));
    assert_eq!(view.take_sounds(), [], "a repeat of R");
    assert_eq!(target_of(&view), before);
    view.input(&key(NEAREST_KEY, false));
    tap(&mut view, NEAREST_KEY);
    assert_eq!(
        view.take_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "a fresh R finds no threat"
    );
}

#[test]
fn r_targets_the_nearest_threat_over_a_nearer_ship_and_brackets_it_red() {
    let mut view = fighting(armed(2, &[]), &two_npcs()).with_behaviour(Rc::new(Threatening));
    tap(&mut view, NEAREST_KEY);
    assert_eq!(target_of(&view), None, "no threat yet: unchanged");
    view.tick(TICK);
    tap(&mut view, NEAREST_KEY);
    assert_eq!(
        target_of(&view),
        Some(NpcId(0)),
        "the threat, though farther"
    );
    let list = drawn(&view);
    assert_eq!(bracket_icons(&list), [10008, 10009, 10010, 10011], "red");
    alt_tap(&mut view, NEAREST_KEY);
    assert_eq!(target_of(&view), Some(NpcId(1)), "Alt-R: the nearer");
    let list = drawn(&view);
    assert_eq!(bracket_icons(&list), [10012, 10013, 10014, 10015], "yellow");
}

#[test]
fn space_fires_the_primaries_and_control_the_secondary_while_held() {
    let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
    tap(&mut view, SELECT_KEY);
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 2);
    view.input(&held(FIRE_KEY));
    ticks(&mut view, 1);
    assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 0));
    view.input(&key(FIRE_KEY, false));
    ticks(&mut view, 2);
    assert_eq!(shots_of(&view, BLASTER), 3, "let go");
    view.input(&key(SECONDARY_KEY, true));
    ticks(&mut view, 2);
    assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (3, 2));
    view.input(&key(SECONDARY_KEY, false));
    ticks(&mut view, 1);
    assert_eq!(shots_of(&view, ROCKET), 2, "let go");
    assert_eq!((FIRE_KEY, SECONDARY_KEY), (Key::Space, Key::Control));
}

#[test]
fn the_flight_keys_are_held_until_let_go_or_released() {
    let mut view = flight();
    assert!(!view.holds_keys());
    view.input(&key(Key::Up, true));
    assert!(view.holds_keys());
    view.input(&key(Key::Up, false));
    assert!(!view.holds_keys(), "let go");
    view.input(&key(Key::Up, true));
    view.release_keys();
    assert!(!view.holds_keys(), "released");
}

#[test]
fn letting_go_of_the_keys_lets_go_of_the_fire_keys() {
    let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
    tap(&mut view, SELECT_KEY);
    view.input(&key(FIRE_KEY, true));
    view.input(&key(SECONDARY_KEY, true));
    ticks(&mut view, 1);
    view.release_keys();
    ticks(&mut view, 2);
    assert_eq!((shots_of(&view, BLASTER), shots_of(&view, ROCKET)), (1, 1));
}

#[test]
fn w_selects_the_next_secondary_and_alt_w_the_one_before() {
    let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE, TORCH]), &[]);
    assert_eq!(secondary_of(&view), None);
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(ROCKET));
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(MISSILE));
    view.input(&held(SELECT_KEY));
    assert_eq!(secondary_of(&view), Some(MISSILE), "a repeat does nothing");
    view.input(&key(Key::Alt, true));
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(ROCKET));
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(TORCH), "wrapping");
    view.input(&key(Key::Alt, false));
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(ROCKET));
    view.input(&key(Key::Alt, true));
    view.release_keys();
    tap(&mut view, SELECT_KEY);
    assert_eq!(secondary_of(&view), Some(MISSILE), "Alt let go");
    assert_eq!(SELECT_KEY, Key::Char('w'));
}

#[test]
fn the_combat_keys_do_nothing_while_the_map_is_open() {
    let mut view = fighting(armed(2, &[BLASTER, ROCKET, MISSILE]), &two_npcs());
    tap(&mut view, MAP);
    for k in [Key::Tab, NEAREST_KEY, SELECT_KEY, FIRE_KEY, SECONDARY_KEY] {
        view.input(&key(k, true));
    }
    tap(&mut view, MAP);
    ticks(&mut view, 2);
    assert_eq!(target_of(&view), None);
    assert_eq!(secondary_of(&view), None);
    assert_eq!(view.session().expect("flying").shots(), []);
}

#[test]
fn the_fire_keys_pressed_and_let_go_during_a_jump_fire_nothing() {
    let mut view = fighting(armed(0, &[BLASTER, ROCKET, MISSILE]), &[]);
    tap(&mut view, SELECT_KEY);
    plot(&mut view, 131);
    fly_out(&mut view);
    view.input(&key(JUMP, true));
    for k in [FIRE_KEY, SECONDARY_KEY] {
        view.input(&key(k, true));
    }
    while view.jump_effect().is_none() {
        view.tick(TICK);
        assert_eq!(view.session().expect("flying").shots(), [], "braking");
    }
    for k in [FIRE_KEY, SECONDARY_KEY] {
        view.input(&key(k, false));
    }
    arrive_now(&mut view);
    ticks(&mut view, 2);
    assert_eq!(secondary_of(&view), Some(ROCKET));
    assert_eq!(view.session().expect("flying").shots(), []);
}

/// The combat sounds taken.
fn combat_sounds(view: &mut View) -> Vec<CombatSound> {
    view.take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Combat(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

#[test]
fn every_steps_fight_reaches_the_effects() {
    let mut view = fighting(armed(0, &[BLASTER]), &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK * 3);
    assert_eq!(shots_of(&view, BLASTER), 3, "three steps");
    let fired = CombatSound {
        sound: nova_sim::SoundId(208),
        offset: (0, 0),
        priority: u16::from(nova_sim::combat::PLAYER_FIRE_PRIORITY),
        until_stopped: false,
    };
    assert_eq!(combat_sounds(&mut view), [fired; 3]);
    assert_eq!(combat_sounds(&mut view), [], "taken");
}

#[test]
fn the_sounds_are_the_sessions_then_the_fights() {
    let mut view = fighting(armed(0, &[BLASTER]), &[]);
    view.input(&key(Key::Up, true));
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert_eq!(
        view.take_sounds(),
        [
            Sound::Sim(SimSound::ThrustStarted),
            Sound::Combat(CombatSound {
                sound: nova_sim::SoundId(208),
                offset: (0, 0),
                priority: u16::from(nova_sim::combat::PLAYER_FIRE_PRIORITY),
                until_stopped: false,
            })
        ]
    );
}

fn line_in(color: Color) -> impl Fn(&DrawCommand) -> bool {
    move |command| matches!(command, DrawCommand::Line { color: c, .. } if *c == color)
}

/// The flight with an NPC 100 pixels above the player, the player
/// firing its blaster (which explodes where it hits) and two beams, one
/// drawn under the ships and one over them, at it.
fn firing_at_an_npc() -> View {
    let catalog = armed(1, &[BLASTER, UNDER, OVER]);
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 5);
    view
}

#[test]
fn the_fight_is_drawn_between_the_stellars_and_the_hud_in_order() {
    let view = firing_at_an_npc().with_overlays(TITLED);
    let list = drawn(&view);
    let stellar = first(&list, sprite_of(1128));
    let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
    let npc = first(&list, sprite_of(2001));
    let ship = first(&list, sprite_of(2000));
    let shot = first(&list, sprite_of(3500));
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let explosion = first(&list, sprite_of(400));
    let brackets = first(&list, icon_in(10008..=10023));
    let title = first(
        &list,
        |c| matches!(c, DrawCommand::Text { origin, .. } if *origin == TITLE),
    );
    let hud = hud_at(&list);
    // An unguided shot is in the shot layer 0x3b52f0, under the intact
    // ships (`_SetupSpriteWorld` @0x21d0c-0x21e24).
    let order = [
        stellar, under, shot, npc, ship, over, explosion, brackets, title, hud,
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    let DrawCommand::Sprite { tint, .. } = list.iter().nth(explosion).cloned().expect("drawn")
    else {
        panic!("a sprite")
    };
    assert_eq!(tint, translucent());
}

#[test]
fn a_shot_is_drawn_with_its_frame_where_it_flies() {
    let mut view = fighting(armed(0, &[BLASTER]), &[]);
    // Turned 45 degrees right, so the shot flies across and up.
    view.input(&key(Key::Right, true));
    ticks(&mut view, 15);
    view.input(&key(Key::Right, false));
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    view.tick(TICK / 2);
    let session = view.session().expect("flying");
    let shot = session.shots()[0];
    let alpha = view.alpha();
    let shown = at(
        shot.position.x - shot.velocity.x * (1.0 - alpha),
        shot.position.y - shot.velocity.y * (1.0 - alpha),
    );
    let list = drawn(&view);
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
    let mut view = fighting(armed(0, &[BLASTER]), &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    view.tick(TICK / 2);
    let list = drawn(&view);
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

/// Every NPC heads for planet 128 and holds its trigger.
#[derive(Debug)]
struct LandingFiring;

impl Behaviour for LandingFiring {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Land(StellarId(128))
    }

    fn trigger(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
        }
    }
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
fn a_beams_fade_is_shown_as_the_session_counts_it() {
    let mut catalog = armed(0, &[OVER]);
    for weapon in &mut catalog.weapons {
        if weapon.id == OVER {
            *weapon = nova_sim::WeaponRecord {
                count: 1,
                reload: 100,
                decay: 1,
                falloff: 10,
                ..*weapon
            };
        }
    }
    let mut view = fighting(catalog, &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    ticks(&mut view, 2);
    let fade = view.session().expect("flying").beams()[0].fade;
    assert!(fade > 0, "fading");
    let shown: Vec<u32> = view
        .shown_beams(&view.camera())
        .iter()
        .map(|beam| beam.fade)
        .collect();
    assert_eq!(shown, [fade]);
}

#[test]
fn a_beam_is_drawn_from_its_firer_where_the_firer_is_drawn() {
    let mut catalog = armed(1, &[OVER]);
    catalog.hulls[1] = hull_of(129, &[UNDER]);
    let (_, chance) = scripted(&placed(850, 650, 0));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(LandingFiring));
    view.tick(TICK);
    view.input(&key(Key::Up, true));
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 5);
    view.tick(TICK / 2);
    let session = view.session().expect("flying");
    let npc = &session.npcs()[0];
    assert_ne!(
        view.npc_previous[&npc.id].position, npc.state.position,
        "moving"
    );
    assert_ne!(view.previous.position, session.player().position, "moving");
    let list = drawn(&view);
    let camera = view.camera();
    let player = camera.world_to_screen(view.shown_position());
    let over = beam_starts(&list, Color::rgba(0, 255, 0, 255));
    assert!(!over.is_empty());
    assert!(
        over.iter().all(|&start| start == player),
        "the player's, from the player as drawn: {over:?}"
    );
    let (npc_shown, _) = view.shown_npc(npc);
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

#[test]
fn a_ship_breaking_up_goes_off_about_where_it_is() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    catalog.hulls[1].death_delay = 30;
    catalog.hulls[1].explode1 = 0;
    // 100 above, facing away: it flies on up the line of fire.
    let mut view = fighting(catalog, &placed(750, 650, 0));
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    let mut breaking = None;
    for _ in 0..40 {
        view.tick(TICK);
        let session = view.session().expect("flying");
        if let Some(npc) = session.npcs().first()
            && let Condition::Dying { ticks_left, .. } = npc.condition
            && ticks_left < 19
            && !view.effects().explosions().is_empty()
        {
            breaking = Some(npc.state.position - npc.state.velocity);
            break;
        }
    }
    // Where it was before the tick's flight step moved it on.
    let at = breaking.expect("an explosion as it breaks up");
    // Never firing, the effects' chance draws the last outcome: a
    // quarter of its 40-pixel sprite less one out, each way.
    assert_eq!(
        view.effects().explosions()[0].at,
        Point::new(at.x + 9.0, at.y + 9.0)
    );
}

#[test]
fn the_players_ship_breaking_up_goes_off_about_where_it_is() {
    // The NPC, straight above the player and facing it, fires a
    // blaster that kills at a hit; the player's ship, 80 pixels wide
    // (the NPC's 40), of DeathDelay 30, breaks up for 88 ticks: three
    // times it, less 2.
    let mut catalog = armed(1, &[]);
    catalog.sheet = Ok(ShipSheet {
        frame_width: 80,
        ..sheet()
    });
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    catalog.hulls[0].death_delay = 30;
    catalog.hulls[0].explode1 = 0;
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    let mut breaking = None;
    for _ in 0..120 {
        view.tick(TICK);
        let session = view.session().expect("flying");
        if let Condition::Dying { ticks_left, .. } = session.player_condition()
            && ticks_left < 19
            && !view.effects().explosions().is_empty()
        {
            breaking = Some(session.player().position);
            break;
        }
    }
    let at = breaking.expect("an explosion as the player's ship breaks up");
    // Never firing, the effects' chance draws the last outcome: a
    // quarter of the player's 80-pixel sprite less one out, each way.
    assert_eq!(
        view.effects().explosions()[0].at,
        Point::new(at.x + 19.0, at.y + 19.0)
    );
}

/// The fight of [`armed`] with the NPC straight above the player and
/// facing it, firing a blaster of `mass_dmg` that ignores shields at
/// it, ticked until the player is no longer intact: the line then.
fn downed_line(mass_dmg: i16) -> (Condition, Option<String>) {
    let mut catalog = armed(1, &[]);
    catalog.weapons[0].mass_dmg = mass_dmg;
    catalog.weapons[0].flags = 0x0020;
    catalog.hulls[0].death_delay = 30;
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    for _ in 0..400 {
        view.tick(TICK);
        let condition = view.session().expect("flying").player_condition();
        if condition != Condition::Intact {
            return (condition, view.message().map(str::to_owned));
        }
    }
    panic!("the player was never downed");
}

#[test]
fn the_player_broken_up_at_a_hit_is_left_told_ship_destroyed() {
    let (condition, line) = downed_line(1000);
    assert!(
        matches!(condition, Condition::Dying { .. }),
        "{condition:?}"
    );
    assert_eq!(
        line.as_deref(),
        Some("Ship destroyed."),
        "#288 replaced #287"
    );
}

#[test]
fn the_player_disabled_is_told_ship_disabled() {
    let (condition, line) = downed_line(1);
    assert_eq!(condition, Condition::Disabled);
    assert_eq!(line.as_deref(), Some("Ship disabled."));
}

#[test]
fn command_and_x_together_hold_the_eject() {
    assert_eq!(EJECT_KEY, Key::Char('x'));
    let mut view = ejecting_view(ejecting_in(100, &[188]), &[Key::Command, EJECT_KEY]);
    assert_eq!(
        until_ejected(&mut view),
        (true, false),
        "before any breakup"
    );
}

#[test]
fn command_alone_or_x_alone_does_not() {
    for keys in [Key::Command, EJECT_KEY] {
        let keys = [keys];
        let mut view = ejecting_view(ejecting_in(100, &[188]), &keys);
        assert_eq!(until_ejected(&mut view), (false, true), "{keys:?}");
    }
}

#[test]
fn an_ejected_player_takes_no_key() {
    let mut view = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
    assert!(until_ejected(&mut view).0);
    view.input(&key(MAP_KEY, true));
    assert!(!view.map_open, "no map");
    view.input(&key(MAP_KEY, false));
    view.input(&key(LAND_KEY, true));
    let session = view.session().expect("flying");
    assert_eq!(session.nav_target(), None, "no landing request");
    assert!(!session.takes(PlayerCommand::Land));
}

#[test]
fn the_pod_is_rescued_after_350_ticks_and_the_rescue_stops_stepping_with_its_desc() {
    let mut catalog = ejecting_in(100, &[188, 187]);
    catalog.home = true;
    let mut view = ejecting_view(catalog, &[]);
    assert!(until_ejected(&mut view).0);
    assert_eq!(view.take_rescue_desc(), None);
    let mut ticks = 0;
    let desc = loop {
        ticks += 1;
        view.tick(TICK);
        if let Some(desc) = view.take_rescue_desc() {
            break desc;
        }
        assert!(ticks <= nova_sim::POD_TICKS, "no rescue");
    };
    assert_eq!(desc, 13999);
    assert_eq!(ticks, nova_sim::POD_TICKS);
    assert_eq!(view.take_rescue_desc(), None, "once");
    let session = view.session().expect("flying");
    assert_eq!(session.ship(), ShipId(128));
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(session.player().position, Vec2::new(50.0, 50.0));
    assert_eq!(
        view.previous.position,
        Vec2::new(50.0, 50.0),
        "laid out where the rescue put it: nothing else stepped"
    );
}

#[test]
fn a_first_tick_eject_rolls_no_puff_for_the_player() {
    let mut view = ejecting_view(ejecting_in(100, &[188, 187]), &[]);
    assert_eq!(until_ejected(&mut view), (true, false));
    let session = view.session().expect("flying");
    let breaking = super::super::dying(session, &view.sheet, &view.npc_sheets);
    assert_eq!(breaking.len(), 1, "the hulk alone");
    assert_eq!((breaking[0].ticks_left, breaking[0].starting), (100, true));
}

/// The player's layers drawn, by `rlëD`: its ship 2000, glow 2100
/// and lights 2200 (the NPC's are 2001, 2101 and 2201).
fn player_layers(view: &View) -> Vec<i16> {
    ship_sprites(view)
        .iter()
        .map(|(image, _)| image.id)
        .filter(|id| [2000, 2100, 2200].contains(id))
        .collect()
}

#[test]
fn the_players_ship_is_drawn_whole_while_breaking_up_then_loses_its_sprite_then_vanishes() {
    // A breakup of one tick, so the engine glow built up by the
    // thrust held from the start has not faded by the explosion: a
    // breaking-up ship takes no controls.
    let mut catalog = killed_in(1);
    catalog.sheet = blinking(Blink::STEADY).sheet;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    // Thrusting with the NPC holding fire, so the glow builds up.
    view.input(&key(Key::Up, true));
    for _ in 0..10 {
        view.tick(TICK);
    }
    let mut view = view.with_behaviour(Rc::new(Firing));
    until_the_player_explodes(&mut view, |view| {
        let ids = player_layers(view);
        assert!(ids.contains(&2000), "the ship: {ids:?}");
        assert!(ids.contains(&2200), "its lights: {ids:?}");
    });
    // At a base of 6 or more the flicker (`glow_level`'s roll of 6,
    // less 4) cannot hide the glow.
    let glow = view.session().expect("flying").engine_glow();
    assert!(glow >= 6, "the glow is still lit as it explodes: {glow}");
    assert_eq!(
        player_layers(&view),
        [2100, 2200],
        "its glow and lights, but no ship, on the explosion's tick"
    );
    // The explosion, `bööm` 128's 3 frames, draws over its 3 ticks.
    for after in 0..5 {
        if after < 3 {
            assert!(!view.effects().explosions().is_empty(), "tick {after}");
            let list = drawn(&view);
            assert!(
                list.iter().any(sprite_of(400)),
                "the explosion is drawn on tick {after}: {list:?}"
            );
        }
        view.tick(TICK);
        assert!(
            matches!(fate_of(&view), PlayerFate::Holding { .. }),
            "{:?}",
            fate_of(&view)
        );
        assert_eq!(player_layers(&view), [0_i16; 0], "nothing of it after");
    }
}

#[test]
fn the_explosion_tick_sounds_the_klaxon_then_halts_it_before_the_explosion() {
    // DeathDelay 2: a timer of 6, set on the tick after the hit
    // (@0x43e29), so 3 ticks of klaxon, then the explosion's.
    let mut view = fighting(killed_in(2), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    let klaxon = Sound::Sim(SimSound::Klaxon);
    let mut breaking_up = Vec::new();
    for _ in 0..60 {
        view.tick(TICK);
        let sounds = view.take_sounds();
        match view.session().expect("flying").player_fate() {
            PlayerFate::BreakingUp { .. } => {
                breaking_up.push(sounds.iter().filter(|&&sound| sound == klaxon).count());
            }
            PlayerFate::Exploded => {
                assert_eq!(
                    breaking_up,
                    [0, 0, 1, 1, 1],
                    "none as it starts, nor as its timer is set"
                );
                let first_combat = sounds
                    .iter()
                    .position(|sound| matches!(sound, Sound::Combat(_)));
                let klaxons: Vec<usize> = (0..sounds.len())
                    .filter(|&at| sounds[at] == klaxon)
                    .collect();
                assert!(first_combat.is_some(), "the explosion sounds: {sounds:?}");
                assert_eq!(klaxons.len(), 1, "{sounds:?}");
                let halt = Sound::Sim(SimSound::Halt(&nova_sim::PLAYER_EXPLOSION_HALTS));
                let halts: Vec<usize> =
                    (0..sounds.len()).filter(|&at| sounds[at] == halt).collect();
                assert_eq!(halts, [klaxons[0] + 1], "halted next: {sounds:?}");
                assert!(halts[0] < first_combat.unwrap_or(0), "{sounds:?}");
                return;
            }
            PlayerFate::Flying | PlayerFate::Disabled => {}
            fate @ (PlayerFate::Holding { .. }
            | PlayerFate::Fading
            | PlayerFate::GameOver
            | PlayerFate::Ejected { .. }) => {
                panic!("past the explosion's tick unseen: {fate:?}")
            }
        }
    }
    panic!("the player's ship never exploded");
}

fn breakup_ticks_left(view: &View) -> u32 {
    match fate_of(view) {
        PlayerFate::BreakingUp { ticks_left, .. } => ticks_left,
        fate => panic!("not breaking up: {fate:?}"),
    }
}

#[test]
fn the_map_does_not_open_while_the_player_breaks_up() {
    let mut view = breaking_up();
    let before = breakup_ticks_left(&view);
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    view.tick(TICK);
    assert!(
        breakup_ticks_left(&view) < before,
        "the flight goes on, unpaused"
    );
}

#[test]
fn mission_info_and_landing_do_nothing_while_the_player_breaks_up() {
    let mut view = breaking_up();
    let nav = nav_target(&view);
    let _ = view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_sounds(), [], "silently (@0x70445, @0x7044f)");
    tap(&mut view, LAND_KEY);
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.take_landing(), None);
    assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_eq!(nav_target(&view), nav);
}

#[test]
fn targeting_and_escort_keys_still_work_while_the_player_breaks_up() {
    let mut view = breaking_up();
    assert_eq!(target_of(&view), None);
    tap(&mut view, TARGET_KEY);
    assert_eq!(target_of(&view), Some(NpcId(0)));
    tap(&mut view, MENU_KEY);
    assert_eq!(
        view.message(),
        Some(NO_ESCORTS),
        "the escort menu's key is taken"
    );
}

#[test]
fn an_exploded_player_takes_no_command() {
    let mut view = exploded();
    let target = target_of(&view);
    let nav = nav_target(&view);
    for k in [MAP_KEY, TARGET_KEY, LAND_KEY, MISSION_INFO_KEY, MENU_KEY] {
        tap(&mut view, k);
    }
    assert!(!view.map_open());
    assert_eq!(target_of(&view), target);
    assert_eq!(nav_target(&view), nav);
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.take_landing(), None);
    assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_ne!(view.message(), Some(NO_ESCORTS));
}

/// Where `view` draws [`catalog`]'s stellar 129, checked to be in the
/// space a click picks from, clear of the status panel.
fn stellar_129_on_screen(view: &View) -> Point {
    let drawn = on_screen(view, (300.0, -200.0));
    assert!(
        (0.0..VIEW_SIZE.0 - STATUS_BAR_WIDTH).contains(&drawn.x)
            && (0.0..VIEW_SIZE.1).contains(&drawn.y),
        "{drawn:?} is in space"
    );
    drawn
}

#[test]
fn a_click_still_picks_a_stellar_while_the_player_breaks_up() {
    let mut view = breaking_up();
    assert_eq!(nav_target(&view), None);
    let stellar = stellar_129_on_screen(&view);
    click(&mut view, stellar);
    assert_eq!(nav_target(&view), Some(StellarId(129)));
}

#[test]
fn a_click_picks_no_stellar_once_the_player_has_exploded() {
    let mut view = exploded();
    let nav = nav_target(&view);
    assert_ne!(nav, Some(StellarId(129)));
    let stellar = stellar_129_on_screen(&view);
    click(&mut view, stellar);
    assert_eq!(nav_target(&view), nav);
}

/// Home (`Key::Other` on this platform) has no binding, so it does
/// nothing in every state, including the dead one the original reads it
/// in (0x6884c, `_ResetPlayer(1)`, left out). This pins the unbound key,
/// not the dead-state lockout, which `an_exploded_player_takes_no_command`
/// and `a_click_picks_no_stellar_once_the_player_has_exploded` cover.
#[test]
fn home_is_unbound_whether_flying_or_exploded() {
    let mut flying = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    assert!(matches!(fate_of(&flying), PlayerFate::Flying));
    assert_eq!(flying.input(&key(Key::Other, true)), ScreenAction::None);
    assert_eq!(flying.input(&key(Key::Other, false)), ScreenAction::None);

    let mut view = exploded();
    let pilot = view.pilot().cloned();
    assert_eq!(view.input(&key(Key::Other, true)), ScreenAction::None);
    assert_eq!(view.input(&key(Key::Other, false)), ScreenAction::None);
    assert!(matches!(fate_of(&view), PlayerFate::Holding { .. }));
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.take_landing(), None);
    assert!(view.take_hail().is_none());
    assert!(view.take_boarding().is_none());
    assert_eq!(view.pilot().cloned(), pilot);
}

/// A flight under fire from [`killed_in`]`(1)`'s NPC, stepped to the
/// player's explosion's tick.
fn at_the_explosion() -> View {
    let mut view = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    until_the_player_explodes(&mut view, |_| {});
    view
}

/// The shots in flight in `view`'s session.
fn shots_in(view: &View) -> Vec<nova_sim::combat::projectile::Shot> {
    view.session().expect("flying").shots().to_vec()
}

/// The alpha of the black covering the whole screen, if the last
/// thing `view` draws is one.
fn black_over(view: &View) -> Option<u8> {
    match drawn(view).iter().last() {
        Some(DrawCommand::Line { width, color, .. })
            if *width == VIEW_SIZE.1 && (color.r, color.g, color.b) == (0, 0, 0) =>
        {
            Some(color.a)
        }
        _ => None,
    }
}

/// [`at_the_explosion`], stepped one tick at a time through the hold
/// to its expiry, which the view reports.
fn held_to_expiry() -> View {
    let mut view = at_the_explosion();
    for _ in 0..=nova_sim::HOLD_TICKS {
        view.tick(TICK);
    }
    assert!(view.take_hold_expired());
    view
}

#[test]
fn after_the_explosion_the_scene_runs_on_without_the_ship_until_the_hold_expires() {
    let mut view = at_the_explosion();
    let mut shots_moved = 0;
    for tick in 1..=nova_sim::HOLD_TICKS {
        let before = shots_in(&view);
        view.tick(TICK);
        assert_eq!(player_layers(&view), [0_i16; 0], "dead tick {tick}");
        assert!(
            ship_sprites(&view)
                .iter()
                .any(|(image, _)| image.id == 2001),
            "the NPC is drawn on dead tick {tick}"
        );
        assert!(!view.take_hold_expired(), "dead tick {tick}");
        if shots_in(&view) != before {
            shots_moved += 1;
        }
    }
    assert_eq!(shots_moved, nova_sim::HOLD_TICKS, "the fight runs on");
    assert_eq!(
        fate_of(&view),
        PlayerFate::Holding { ticks_left: 0 },
        "no ticks left, but not expired"
    );
    assert!(!view.game_over());
    let before = shots_in(&view);
    view.tick(TICK);
    assert_ne!(shots_in(&view), before, "the 243rd step runs");
    assert!(view.take_hold_expired(), "and the hold expires on it");
    assert!(!view.take_hold_expired(), "once");
}

#[test]
fn a_long_frame_stops_stepping_on_the_hold_expirys_step() {
    let stepped = held_to_expiry();
    let mut view = at_the_explosion();
    for _ in 0..nova_sim::HOLD_TICKS - 2 {
        view.tick(TICK);
    }
    view.tick(TICK * 8);
    assert!(view.take_hold_expired());
    assert_eq!(shots_in(&view), shots_in(&stepped), "no step after it");
    assert_eq!(
        view.session().expect("flying").npcs(),
        stepped.session().expect("flying").npcs()
    );
}

#[test]
fn at_hold_expiry_flight_stops_and_nothing_fades_until_the_fade_begins() {
    let mut view = held_to_expiry();
    let shots = shots_in(&view);
    for _ in 0..30 {
        view.tick(TICK);
        assert_eq!(shots_in(&view), shots, "nothing moves");
        assert_eq!(black_over(&view), None, "nothing fades");
        assert_eq!(fate_of(&view), PlayerFate::Holding { ticks_left: 0 });
        assert!(!view.game_over());
    }
    view.begin_fade();
    assert_eq!(fate_of(&view), PlayerFate::Fading);
}

#[test]
fn the_death_fade_goes_to_black_over_a_second_then_the_game_is_over() {
    let mut view = held_to_expiry();
    view.begin_fade();
    let shots = shots_in(&view);
    view.tick(Duration::from_millis(500));
    assert_eq!(black_over(&view), Some(128), "half way, over the HUD");
    assert!(!view.game_over());
    assert_eq!(shots_in(&view), shots, "the scene is frozen under it");
    view.tick(Duration::from_millis(499));
    assert!(!view.game_over());
    assert_eq!(fate_of(&view), PlayerFate::Fading);
    view.tick(Duration::from_millis(1));
    assert_eq!(black_over(&view), Some(255));
    assert!(view.game_over());
    assert_eq!(fate_of(&view), PlayerFate::GameOver);
    assert_eq!(shots_in(&view), shots);
}

#[test]
fn beginning_the_fade_before_the_hold_expires_does_nothing() {
    let mut view = at_the_explosion();
    view.tick(TICK);
    view.begin_fade();
    view.tick(Duration::from_millis(1500));
    assert_eq!(black_over(&view), None);
    assert!(!view.game_over());
    assert!(matches!(fate_of(&view), PlayerFate::Holding { .. }));
    while !view.take_hold_expired() {
        view.tick(TICK);
    }
    view.tick(Duration::from_millis(1500));
    assert_eq!(black_over(&view), None, "nor once the hold expires");
    assert!(!view.game_over());
}

#[test]
fn only_a_pilot_with_a_session_whose_ship_is_breaking_up_or_destroyed_is_dying() {
    let flying = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    assert!(!flying.is_dying(), "alive");
    assert!(at_the_explosion().is_dying(), "destroyed");
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..killed_in(1)
    });
    assert!(!failed.is_dying(), "no session, no pilot dying");
}

#[test]
fn only_a_living_pilot_may_be_saved() {
    let flying = fighting(killed_in(1), &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    assert!(flying.may_save());
    assert!(!at_the_explosion().may_save());
    let mut failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..killed_in(1)
    });
    assert!(failed.session().is_err());
    assert!(!failed.may_save(), "no session, no pilot to save");
    assert!(!failed.take_hold_expired());
    assert!(!failed.game_over());
    failed.begin_fade();
    failed.tick(Duration::from_secs(2));
    assert!(!failed.game_over());
}

#[test]
fn only_a_strict_pilot_ending_play_dying_is_deleted() {
    let strict = |catalog: FakeCatalog| {
        let pilot = Pilot::new(&catalog, "Ada")
            .expect("starts")
            .with_strict_play(true);
        let (_, chance) = scripted(&placed(750, 650, 180));
        let mut view = FlightView::with_pilot(catalog, pilot)
            .with_chance(chance)
            .with_behaviour(Rc::new(Firing));
        view.tick(TICK);
        view
    };
    let mut view = strict(killed_in(1));
    assert!(!view.deletes_pilot_on_ending_play(), "flying");
    until_the_player_explodes(&mut view, |_| {});
    assert!(view.deletes_pilot_on_ending_play(), "exploded");
    assert!(
        !at_the_explosion().deletes_pilot_on_ending_play(),
        "not a strict pilot"
    );
    let failed = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..killed_in(1)
    });
    assert!(
        !failed.deletes_pilot_on_ending_play(),
        "no session, no pilot to delete"
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
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_behaviour(Rc::new(Firing));
    let placeholder = |view: &View| {
        let mut boxed = DrawList::new();
        let at = view.camera().world_to_screen(view.shown_position());
        crossed_box(&mut boxed, at, PLACEHOLDER_SIZE, PLACEHOLDER);
        boxed
    };
    until_the_player_explodes(&mut view, |view| {
        let list = drawn(view);
        assert!(sprite_unavailable(&list), "while breaking up");
        assert!(draws_run(&list, &placeholder(view)), "{list:?}");
    });
    // The explosion's tick, then the ones after.
    for _ in 0..3 {
        let list = drawn(&view);
        assert!(!sprite_unavailable(&list), "{list:?}");
        assert!(!draws_run(&list, &placeholder(&view)), "{list:?}");
        view.tick(TICK);
    }
}

#[test]
fn a_shot_whose_look_cannot_be_read_is_a_crossed_box() {
    let mut view = fighting(armed(0, &[UNSEEN]), &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    let list = drawn(&view);
    let shot = view.session().expect("flying").shots()[0];
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
fn a_destroyed_npc_explodes_and_scatters_debris_on_the_frames_after() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    // Drifting up, so the explosion is heard from where the player is.
    view.input(&key(Key::Up, true));
    view.tick(TICK);
    view.input(&key(Key::Up, false));
    view.input(&key(FIRE_KEY, true));
    let mut destroyed = false;
    for _ in 0..10 {
        view.tick(TICK);
        if view.session().expect("flying").npcs().is_empty() {
            destroyed = true;
            break;
        }
    }
    assert!(destroyed, "{:?}", view.session().expect("flying").npcs());
    assert_eq!(target_of(&view), None);
    let effects = view.effects();
    assert_eq!(effects.explosions().len(), 1);
    assert_eq!(effects.explosions()[0].boom, BoomId(128));
    assert_eq!(effects.particles().len(), 16);
    let list = drawn(&view);
    let explosion = first(&list, sprite_of(400));
    assert_eq!(
        list.iter()
            .skip(explosion)
            .filter(|c| matches!(c, DrawCommand::Dot { color, .. } if *color == DEBRIS_COLOR))
            .count(),
        16,
        "drawn after the explosion"
    );
    let player = view.session().expect("flying").player().position;
    assert!(player.y < 0.0, "{player:?}");
    let boom = view.effects().explosions()[0].at;
    let heard = CombatSound {
        sound: nova_sim::SoundId(302),
        offset: (
            (boom.x - player.x).round() as i32,
            (boom.y - player.y).round() as i32,
        ),
        priority: crate::flight::effects::EXPLOSION_PRIORITY,
        until_stopped: false,
    };
    assert!(
        combat_sounds(&mut view).contains(&heard),
        "the explosion is heard: {heard:?}"
    );
}

#[test]
fn an_npc_destroyed_under_the_originals_debris_reading_scatters_none() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    let rules = Rulebook::default().with_override(RuleKey::ShipDebris, RuleSource::Engine);
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_rules(rules);
    tap(&mut view, Key::Tab);
    view.input(&key(FIRE_KEY, true));
    for _ in 0..10 {
        view.tick(TICK);
        if view.session().expect("flying").npcs().is_empty() {
            break;
        }
    }
    assert!(view.session().expect("flying").npcs().is_empty());
    let effects = view.effects();
    assert_eq!(effects.explosions().len(), 1, "its Explode2 still");
    assert_eq!(effects.explosions()[0].boom, BoomId(128));
    assert_eq!(effects.particles(), [], "no debris");
}

/// [`armed`] with a blaster whose hits throw off 5 orange particles
/// of `HitPartLife` 30, flown once at NPC 0 ahead: the flight on the
/// tick its shot hits, and the NPC.
fn hit_once() -> (View, ShipRef) {
    hit_once_by(5, Rulebook::default())
}

/// [`hit_once`] with `particles` thrown off, by `rules`.
fn hit_once_by(particles: i16, rules: Rulebook) -> (View, ShipRef) {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 1;
    catalog.weapons[0].explod_type = -1;
    catalog.weapons[0].hit_particles = particles;
    catalog.weapons[0].hit_part_vel = 100;
    catalog.weapons[0].hit_part_life = 30;
    catalog.weapons[0].hit_part_color = 0x00FF_8000;
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_rules(rules);
    tap(&mut view, Key::Tab);
    let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    for _ in 0..20 {
        if view.shield_flash(npc).is_some() {
            return (view, npc);
        }
        view.tick(TICK);
    }
    panic!("never hit: {:?}", view.session().expect("flying").npcs());
}

#[test]
fn an_npc_hit_flashes_its_shield_32_on_the_tick_of_the_hit_then_31() {
    let (mut view, npc) = hit_once();
    assert_eq!(view.shield_flash(npc), Some(32.0));
    view.tick(TICK);
    assert_eq!(view.shield_flash(npc), Some(31.0));
    assert_eq!(view.shield_flash(ShipRef::Player), None, "not hit");
}

#[test]
fn a_hit_with_hit_particles_sprays_them_on_the_npc_in_its_colour() {
    let (view, _) = hit_once();
    let orange = crate::Color::rgba(0xFF, 0x80, 0x00, 255);
    let particles = view.effects().particles();
    assert_eq!(particles.len(), 5, "{particles:?}");
    assert!(particles.iter().all(|piece| piece.color == orange));
}

#[test]
fn under_the_combat_caps_a_hit_keeps_at_most_1024_particles() {
    let (view, _) = hit_once_by(1100, Rulebook::default());
    assert_eq!(
        view.effects().particles().len(),
        1100,
        "uncapped by default"
    );
    let caps = Rulebook::default().with_override(RuleKey::CombatCaps, RuleSource::Engine);
    let (view, _) = hit_once_by(1100, caps);
    assert_eq!(view.effects().particles().len(), 1024);
}

#[test]
fn a_beams_hit_flashes_the_npcs_shield_from_the_frame_after() {
    let mut catalog = armed(1, &[UNDER]);
    catalog.weapons[4].beam_length = 400;
    catalog.weapons[4].energy_dmg = 1;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    let shield = |view: &View| {
        view.session().expect("flying").npcs()[0]
            .reserves
            .shield
            .now
    };
    let full = shield(&view);
    let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    let mut ticks = 0;
    while shield(&view) == full {
        assert!(ticks < 20, "never hit");
        view.tick(TICK);
        ticks += 1;
    }
    assert_eq!(view.shield_flash(npc), None, "shown the frame after");
    view.tick(TICK);
    assert_eq!(view.shield_flash(npc), Some(32.0));
}

#[test]
fn a_destroyed_npcs_shield_flash_is_dropped() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 20;
    catalog.weapons[0].mass_dmg = 20;
    catalog.weapons[0].explod_type = -1;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    let npc = ShipRef::Npc(view.session().expect("flying").npcs()[0].id);
    view.input(&key(FIRE_KEY, true));
    let mut flashed = false;
    for _ in 0..20 {
        view.tick(TICK);
        if view.session().expect("flying").npcs().is_empty() {
            break;
        }
        flashed |= view.shield_flash(npc) == Some(32.0);
    }
    assert!(
        view.session().expect("flying").npcs().is_empty(),
        "destroyed"
    );
    assert!(flashed, "hit before it was destroyed");
    assert_eq!(view.shield_flash(npc), None);
    view.tick(TICK);
    assert_eq!(view.shield_flash(npc), None, "nor on the next tick");
}

#[test]
fn landing_clears_the_effects() {
    let mut catalog = detonating();
    catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    let mut view = fighting(catalog, &[]);
    detonate(&mut view);
    land_now(&mut view);
    assert!(view.take_landing().is_some());
    assert_eq!(*view.effects(), Effects::default());
}

#[test]
fn arriving_clears_the_effects() {
    let mut view = fighting(detonating(), &[]);
    plot(&mut view, 131);
    fly_out(&mut view);
    detonate(&mut view);
    jump_now(&mut view);
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(*view.effects(), Effects::default());
}

/// [`armed`]'s NPC, shïp 129, of `DeathDelay` 4 and one crew pod, which
/// one hit of the player's blaster breaks up: the pod comes out on its
/// explosion tick, at a timer of 2.
fn podding() -> FakeCatalog {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 200;
    catalog.weapons[0].mass_dmg = 200;
    catalog.hulls[1].death_delay = 4;
    catalog.hulls[1].pod_count = 1;
    catalog
}

/// Fires at the targeted NPC until its pod is adrift.
fn eject_a_pod(view: &mut View) {
    tap(view, Key::Tab);
    view.input(&key(FIRE_KEY, true));
    for _ in 0..40 {
        view.tick(TICK);
        if !view.effects().pods().is_empty() {
            view.input(&key(FIRE_KEY, false));
            return;
        }
    }
    panic!("no pod came out");
}

/// How many escape pods `view` draws, from the pods' sheet (`rlëD` 1095).
fn pods_drawn(view: &View) -> usize {
    let mut list = DrawList::new();
    view.draw(&mut list);
    list.into_iter()
        .filter(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 1095))
        .count()
}

#[test]
fn a_dying_npcs_pod_is_drawn_and_heard_until_landing_clears_it() {
    let mut catalog = podding();
    catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    let mut view = fighting(catalog, &placed(750, 650, 180));
    eject_a_pod(&mut view);
    assert_eq!(pods_drawn(&view), 1);
    let heard: Vec<nova_sim::SoundId> = view
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Combat(sound) => Some(sound.sound),
            _ => None,
        })
        .collect();
    assert!(heard.contains(&nova_sim::SoundId(372)), "{heard:?}");
    land_now(&mut view);
    assert!(view.take_landing().is_some());
    assert!(view.effects().pods().is_empty());
    assert_eq!(pods_drawn(&view), 0);
}

#[test]
fn arriving_leaves_no_pod_drawn() {
    let mut view = fighting(podding(), &placed(750, 650, 180));
    plot(&mut view, 131);
    fly_out(&mut view);
    let ahead = player(&view).position + nova_sim::Vec2::new(0.0, -80.0);
    let npc = view.session().expect("flying").npcs()[0].id;
    nova_sim::fixture::place(view.session.as_mut().expect("flying"), npc, ahead);
    let placed_at = view.session().expect("flying").npcs()[0].state;
    assert_eq!(
        (placed_at.position, placed_at.velocity),
        (ahead, nova_sim::Vec2::ZERO),
        "placed ahead, at rest"
    );
    eject_a_pod(&mut view);
    jump_now(&mut view);
    assert!(
        !view.effects().pods().is_empty(),
        "still adrift as the jump begins"
    );
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert!(view.effects().pods().is_empty());
    assert_eq!(pods_drawn(&view), 0);
}

/// [`detonating`] with a blast of radius 100 doing 1 energy damage,
/// which reaches the player who fired it.
fn self_blasting() -> FakeCatalog {
    let mut catalog = detonating();
    catalog.weapons[0].blast_radius = 100;
    catalog.weapons[0].energy_dmg = 1;
    catalog
}

/// Fires one [`self_blasting`] shot and ticks until its blast flashes
/// the player's shield.
fn flash_the_player(view: &mut View) {
    detonate(view);
    for _ in 0..5 {
        if view.shield_flash(ShipRef::Player).is_some() {
            return;
        }
        view.tick(TICK);
    }
    panic!("the player was never hit: {:?}", player(view));
}

#[test]
fn landing_clears_the_shield_flashes() {
    let mut catalog = self_blasting();
    catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    let mut view = fighting(catalog, &[]);
    flash_the_player(&mut view);
    land_now(&mut view);
    assert!(view.take_landing().is_some());
    assert_eq!(view.shield_flash(ShipRef::Player), None);
}

#[test]
fn passing_through_a_wormhole_clears_the_shield_flashes() {
    let holes = holed();
    let catalog = FakeCatalog {
        sites: holes.sites,
        gates: holes.gates,
        gate_listers: holes.gate_listers,
        ..self_blasting()
    };
    let mut view = fighting(catalog, &[]);
    flash_the_player(&mut view);
    land_now(&mut view);
    assert_eq!(system_of(&view), SystemId(131));
    assert_eq!(view.shield_flash(ShipRef::Player), None);
}

#[test]
fn the_target_is_bracketed_and_its_panel_shows_it() {
    let view = firing_at_an_npc();
    let session = view.session().expect("flying");
    let npc = &session.npcs()[0];
    let list = drawn(&view);
    let (shown, _) = view.shown_npc(npc);
    let Some(Ok(sheet)) = view.npc_sheets.get(&npc.ship) else {
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
    let bar = view.status_bar().expect("a status bar");
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
    let mut view = firing_at_an_npc();
    tap(&mut view, STELLAR_SLOT_KEYS[0][0]);
    let list = drawn(&view);
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
    let catalog = armed(1, &[]);
    let mut view = fighting(catalog, &placed(750, 650, 180)).with_disable_rule(Rc::new(Disabling));
    tap(&mut view, Key::Tab);
    view.tick(TICK);
    let session = view.session().expect("flying");
    assert_eq!(session.npcs()[0].condition, Condition::Disabled);
    let list = drawn(&view);
    assert_eq!(bracket_icons(&list), [10020, 10021, 10022, 10023]);
    assert!(texts(&list).contains(&target::DISABLED.to_owned()));
}

/// The ship bracket `cïcn`s drawn (10008-10023), and where.
fn ship_bracket_corners(view: &View) -> Vec<(i16, Point)> {
    drawn(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } if icon_in(10008..=10023)(command) => {
                Some((image.id, *top_left))
            }
            _ => None,
        })
        .collect()
}

/// Where `view` draws its target's sprite.
fn target_on_screen(view: &View) -> Point {
    let session = view.session().expect("flying");
    let npc = session.target().expect("a target");
    view.camera().world_to_screen(view.shown_npc(npc).0)
}

#[test]
fn targeting_a_ship_on_screen_flies_its_brackets_in_then_settles_them() {
    let mut view = fighting(armed(1, &[]), &placed(750, 650, 180));
    assert_eq!(ship_bracket_corners(&view), []);
    tap(&mut view, Key::Tab);
    // Ship 129, 40 x 40: half 20; neutral, from 10012.
    assert_eq!(
        ship_bracket_corners(&view),
        corners_at(10012, target_on_screen(&view), 20.0 + 196.0),
        "a frame drawn before the next tick"
    );
    for expand in [196.0, 136.0, 76.0, 16.0, 0.0, 0.0] {
        view.tick(TICK);
        assert_eq!(
            ship_bracket_corners(&view),
            corners_at(10012, target_on_screen(&view), 20.0 + expand),
            "{expand}"
        );
    }
}

#[test]
fn a_target_whose_sheet_cannot_be_read_is_bracketed_round_its_crossed_box() {
    let mut view = fighting(trafficked(&[130], 1, 130, 3), &placed(850, 650, 90));
    tap(&mut view, Key::Tab);
    ticks(&mut view, 5);
    // Its 64 x 64 box: half 32.
    assert_eq!(
        ship_bracket_corners(&view),
        corners_at(10012, target_on_screen(&view), PLACEHOLDER_SIZE / 2.0)
    );
}

/// An unarmed flight targeting its one NPC, intact, its panel drawn
/// two ticks after a timed redraw.
fn targeting_after_a_redraw(view: View) -> View {
    let mut view = view;
    while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    tap(&mut view, Key::Tab);
    view.tick(TICK);
    assert_eq!(status_shown(&view), status_now(&view));
    assert!(
        matches!(status_shown(&view), Some(nova_sim::TargetStatus::Shield(_))),
        "{:?}",
        status_shown(&view)
    );
    view
}

#[test]
fn the_target_panel_holds_a_disabling_until_its_next_timed_redraw() {
    let view = fighting(armed(1, &[]), &placed(750, 650, 180));
    let view = targeting_after_a_redraw(view);
    let held = status_shown(&view);
    let mut view = view.with_disable_rule(Rc::new(Disabling));
    view.tick(TICK);
    assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
    while !now_tick(&view).is_multiple_of(TARGET_PERIOD_TICKS) {
        assert_eq!(status_shown(&view), held, "tick {}", now_tick(&view));
        assert!(!texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
        view.tick(TICK);
    }
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
    assert!(texts(&drawn(&view)).contains(&target::DISABLED.to_owned()));
}

#[test]
fn read_the_bibles_way_the_target_panel_follows_every_frame() {
    let view = every_frame(fighting(armed(1, &[]), &placed(750, 650, 180)));
    let view = targeting_after_a_redraw(view);
    let mut view = view.with_disable_rule(Rc::new(Disabling));
    view.tick(TICK);
    assert_ne!(now_tick(&view) % TARGET_PERIOD_TICKS, 0);
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
}

#[test]
fn redrawing_the_status_retakes_the_target_panel() {
    let view = fighting(armed(1, &[]), &placed(750, 650, 180));
    let view = targeting_after_a_redraw(view);
    let mut view = view.with_disable_rule(Rc::new(Disabling));
    view.tick(TICK);
    assert_ne!(status_shown(&view), status_now(&view), "held");
    view.redraw_status();
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
}

#[test]
fn a_change_of_target_shows_at_once() {
    let mut view = fighting(armed(2, &[]), &two_npcs());
    tap(&mut view, Key::Tab);
    view.tick(TICK);
    let drawn_for = |view: &View| {
        view.target_scene
            .as_ref()
            .and_then(|(drawn, _)| drawn.map(|drawn| drawn.npc))
    };
    assert_eq!(drawn_for(&view), Some(NpcId(0)));
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), Some(NpcId(1)));
    assert_eq!(drawn_for(&view), Some(NpcId(0)), "not yet retaken");
    assert_eq!(*view.target_shown(), view.target_now(), "shown at once");
    view.tick(TICK);
    assert_eq!(drawn_for(&view), Some(NpcId(1)), "retaken");
    tap(&mut view, Key::Tab);
    assert_eq!(target_of(&view), None);
    assert_eq!(*view.target_shown(), None, "lost at once");
    let list = texts(&drawn(&view));
    assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
}

#[test]
fn a_falling_shield_shows_at_the_radar_refreshes_only() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].energy_dmg = 1;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    tap(&mut view, Key::Tab);
    view.input(&key(FIRE_KEY, true));
    let mut shown = status_shown(&view);
    let mut changes = 0;
    let mut held = 0;
    for _ in 0..240 {
        let (blink, before) = (view.hud_cadence.blink_on(), now_tick(&view));
        view.tick(TICK);
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
        if target_of(&view).is_none() {
            break;
        }
    }
    view.input(&key(FIRE_KEY, false));
    assert!(changes > 1, "the shield fell: {changes}");
    assert!(held > 0, "and was held between refreshes");
}

#[test]
fn a_disabled_player_has_no_fuel_gauge() {
    let mut catalog = armed(1, &[]);
    catalog.hulls[1] = hull_of(129, &[BLASTER]);
    let mut view = fighting(catalog, &placed(750, 650, 180))
        .with_behaviour(Rc::new(Firing))
        .with_disable_rule(Rc::new(Disabling));
    let colors = |view: &View| -> Vec<Color> {
        lines(&drawn(view))
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Line { color, .. } => Some(color),
                _ => None,
            })
            .collect()
    };
    let layout = layout();
    assert!(colors(&view).contains(&layout.fuel_full), "intact");
    for _ in 0..40 {
        if view.session().expect("flying").player_condition() == Condition::Disabled {
            break;
        }
        view.tick(TICK);
    }
    assert_eq!(
        view.session().expect("flying").player_condition(),
        Condition::Disabled
    );
    let shown = colors(&view);
    assert!(!shown.contains(&layout.fuel_full), "{shown:?}");
    assert!(!shown.contains(&layout.fuel_partial), "{shown:?}");
    assert!(shown.contains(&layout.armor_color), "{shown:?}");
}

#[test]
fn the_hud_shows_no_target_and_the_secondary_weapon() {
    let mut view = fighting(armed(0, &[BLASTER, ROCKET]), &[]);
    let list = texts(&drawn(&view));
    assert!(list.contains(&NO_TARGET.to_owned()), "{list:?}");
    assert!(list.contains(&NO_SECONDARY.to_owned()), "{list:?}");
    tap(&mut view, SELECT_KEY);
    let list = texts(&drawn(&view));
    assert!(list.contains(&"Rocket".to_owned()), "{list:?}");
    let unarmed = texts(&drawn(&fighting(armed(0, &[BLASTER]), &[])));
    assert!(unarmed.contains(&NO_SECONDARY.to_owned()), "{unarmed:?}");
}

#[test]
fn the_panel_is_laid_out_by_the_metrics_given() {
    let view = fighting(armed(0, &[BLASTER]), &[])
        .with_metrics(Rc::new(crate::text::fixture::MonoMetrics));
    let list = drawn(&view);
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
fn the_effects_roll_on_their_own_chance() {
    let mut catalog = armed(1, &[BLASTER]);
    catalog.weapons[0].mass_dmg = 1000;
    catalog.weapons[0].flags = 0x0020;
    catalog.weapons[0].explod_type = -1;
    let (_, chance) = scripted(&placed(750, 650, 180));
    let (effects, effects_chance) = scripted(&[]);
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_effects_chance(effects_chance);
    view.tick(TICK);
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 10);
    assert!(view.session().expect("flying").npcs().is_empty());
    assert_eq!(
        effects.borrow().asked,
        [360, 101, 31].repeat(16),
        "the debris, drawn on the effects' own chance"
    );
}

/// A view firing the player's looped beam, of a 3-tick sound, each step,
/// by `rules`.
fn firing_a_looped_beam(rules: Rulebook) -> View {
    let mut catalog = armed(0, &[UNDER]);
    catalog.weapons[4].reload = 0;
    catalog.weapons[4].count = 5;
    catalog.looks[4].1 = Ok(WeaponLook {
        flags: weapons::LOOPED_SOUND,
        sound: Some(nova_sim::SoundId(220)),
        sound_ticks: Some(3),
        ..WeaponLook::default()
    });
    let mut view = fighting(catalog, &[]).with_rules(rules);
    view.input(&key(FIRE_KEY, true));
    view
}

#[test]
fn by_default_a_looped_weapon_is_heard_every_shot_waiting_on_any_voice() {
    let mut view = firing_a_looped_beam(Rulebook::default());
    ticks(&mut view, 4);
    assert_eq!(
        view.session().expect("flying").beams().len(),
        4,
        "a beam fired each step"
    );
    let heard = combat_sounds(&mut view);
    assert_eq!(heard.len(), 4, "the audio side decides: {heard:?}");
    assert!(heard.iter().all(|sound| sound.until_stopped), "{heard:?}");
}

#[test]
fn by_the_other_reading_a_looped_weapon_is_heard_again_once_its_sound_has_played_out() {
    let rules = Rulebook::default().with_override(RuleKey::LoopedSoundWait, RuleSource::Bible);
    let mut view = firing_a_looped_beam(rules);
    ticks(&mut view, 4);
    let heard = combat_sounds(&mut view);
    assert_eq!(heard.len(), 2, "on the first and fourth");
    assert!(heard.iter().all(|sound| !sound.until_stopped), "{heard:?}");
    view.input(&key(FIRE_KEY, false));
    ticks(&mut view, 1);
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 1);
    assert_eq!(combat_sounds(&mut view), [], "still playing");
    view.input(&key(FIRE_KEY, false));
    ticks(&mut view, 3);
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 1);
    assert_eq!(combat_sounds(&mut view).len(), 1, "played out");
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

/// Says no missile is hostile, counting the times it is asked.
#[derive(Debug, Default)]
struct Unalarmed {
    asked: std::cell::Cell<usize>,
}

impl nova_sim::PointDefenceRule for Unalarmed {
    fn hostile(
        &self,
        _defender: nova_sim::combat::defence::Side,
        _firer: nova_sim::combat::defence::Side,
        _govts: &nova_sim::Governments,
    ) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Every NPC idles, targets the player and holds its trigger.
#[derive(Debug)]
struct Attacking;

impl Behaviour for Attacking {
    fn decide(
        &self,
        _npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Idle
    }

    fn trigger(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> nova_sim::Trigger {
        nova_sim::Trigger {
            primary: true,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: false,
            keep: None,
        }
    }

    fn target(&self, _npc: &nova_sim::Npc, _around: &nova_sim::Surroundings) -> Option<ShipRef> {
        Some(ShipRef::Player)
    }
}

/// `catalog`'s flight with its NPC attacking from 200 pixels above
/// the player, after the step that populates the system.
fn attacked(catalog: FakeCatalog) -> View {
    let (_, chance) = scripted(&placed(750, 550, 180));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Attacking));
    view.tick(TICK);
    view
}

#[test]
fn the_fight_asks_the_point_defence_rule_given_each_step_in_flight() {
    let rule = Rc::new(Unalarmed::default());
    let asking = Rulebook::default().with_override(RuleKey::PdHostility, RuleSource::Bible);
    let mut view = attacked(defending())
        .with_rules(asking)
        .with_point_defence_rule(rule.clone());
    view.tick(TICK);
    assert_eq!(shots_of(&view, IR), 1, "fired at the player");
    assert_eq!(
        rule.asked.get(),
        0,
        "after the player's point defence, which runs before the NPCs fire"
    );
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 2, "of the one missile, each step");
    assert_eq!(shots_of(&view, QUAD), 0, "never hostile");
    view.input(&key(MAP_KEY, true));
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 2, "not while the map is open");
    view.input(&key(MAP_KEY, true));
    tap(&mut view, LAND);
    assert!(view.take_landing().is_some(), "one press, uninhabited");
    ticks(&mut view, 2);
    assert_eq!(rule.asked.get(), 2, "not while landed");
}

#[test]
fn a_missile_shot_down_is_drawn_exploding_and_unheard() {
    let mut view = attacked(defending());
    let mut sounds = Vec::new();
    let mut exploded = false;
    for _ in 0..12 {
        view.tick(TICK);
        sounds.extend(combat_sounds(&mut view));
        let explosion = drawn(&view)
            .iter()
            .any(|command| matches!(command, DrawCommand::Sprite { image, .. } if image.id == 400));
        if explosion {
            exploded = true;
            break;
        }
    }
    assert!(exploded, "bööm 128 drawn");
    assert_eq!(shots_of(&view, IR), 0, "shot down");
    assert!(
        sounds
            .iter()
            .all(|sound| sound.sound != nova_sim::SoundId(302)),
        "{sounds:?}"
    );
}

/// [`armed`] with a blaster doing 10 energy and 10 mass damage, the
/// warship armed with a harmless gun of unlimited ammunition (an unarmed
/// warship is fully dry, and drops its attack at once, `_WarshipAI`
/// @0x8c0e7-0x8c117), and govt 140 that costs 3 to disable a ship of.
fn damaging() -> FakeCatalog {
    let catalog = armed(1, &[BLASTER]);
    FakeCatalog {
        weapons: vec![
            nova_sim::WeaponRecord {
                reload: 5,
                mass_dmg: 10,
                energy_dmg: 10,
                ..gun(BLASTER, 0, -1, -1)
            },
            gun(UNSEEN, 0, -1, -1),
        ],
        hulls: vec![hull_of(128, &[BLASTER]), hull_of(129, &[UNSEEN])],
        govts: vec![nova_sim::GovtRecord {
            id: GovtId(140),
            name: String::new(),
            flags: 0,
            flags2: 0,
            crime_tol: 0,
            initial_rec: 0,
            penalties: nova_sim::Penalties {
                disable: 3,
                ..nova_sim::Penalties::default()
            },
            max_odds: 100,
            classes: [-1; 4],
            allies: [-1; 4],
            enemies: [-1; 4],
            comm_name: String::new(),
            medium_name: String::new(),
            scan_mask: 0,
            scan_fine: 0,
            news_pic: -1,
            require: 0,
            inh_jam: [0; 4],
            skill_mult: 0,
        }],
        ..catalog
    }
}

#[test]
fn by_default_npcs_fight_by_novas_ai_and_crimes_by_novas_law() {
    // The warship 100 above, facing away: it flies on up the line of
    // fire.
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::new(damaging()).with_chance(chance);
    view.tick(TICK);
    // Aimed at it: a stray hit the answer flag forgives.
    tap(&mut view, TARGET_KEY);
    view.input(&key(FIRE_KEY, true));
    let mut answered = false;
    for _ in 0..120 {
        view.tick(TICK);
        let session = view.session().expect("flying");
        answered |= session
            .npcs()
            .first()
            .is_some_and(|npc| npc.goal == Goal::Attack(ShipRef::Player));
        if session
            .npcs()
            .first()
            .is_some_and(|npc| npc.condition == Condition::Disabled)
        {
            break;
        }
    }
    assert!(answered, "the warship hit turned on the player");
    let session = view.session().expect("flying");
    assert_eq!(session.npcs()[0].condition, Condition::Disabled);
    assert_eq!(session.pilot().legal_record(GovtId(140)), -3);
}

/// Records each crime, and changes nothing.
#[derive(Debug, Default)]
struct Witness {
    seen: RefCell<Vec<nova_sim::Crime>>,
}

impl nova_sim::LegalCode for Witness {
    fn penalties(
        &self,
        crime: nova_sim::Crime,
        _victim: Option<GovtId>,
        _govts: &nova_sim::Governments,
    ) -> Vec<(GovtId, i32)> {
        self.seen.borrow_mut().push(crime);
        Vec::new()
    }

    fn system_penalties(
        &self,
        _crime: nova_sim::Crime,
        _victim: Option<GovtId>,
        _from: nova_sim::SystemId,
        _map: &dyn nova_sim::CrimeMap,
        _govts: &nova_sim::Governments,
    ) -> Vec<(nova_sim::SystemId, f32)> {
        Vec::new()
    }
}

#[test]
fn the_flight_convicts_by_the_law_it_is_given() {
    let witness = Rc::new(Witness::default());
    // 100 above, facing away: it flies on up the line of fire.
    let (_, chance) = scripted(&placed(750, 650, 0));
    let mut view = FlightView::new(damaging())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_law(witness.clone());
    view.tick(TICK);
    view.input(&key(FIRE_KEY, true));
    for _ in 0..120 {
        view.tick(TICK);
        if view.session().expect("flying").npcs()[0].condition == Condition::Disabled {
            break;
        }
    }
    assert_eq!(*witness.seen.borrow(), [nova_sim::Crime::Disable]);
    let session = view.session().expect("flying");
    assert_eq!(session.pilot().legal_record(GovtId(140)), 0, "as it says");
}

#[test]
fn a_ship_the_player_shoves_is_drawn_moved_on_the_frame_of_the_hit() {
    // The push lands in the ship's velocity (`_DamageShip`
    // @0x3a9b8-0x3ab84) before `_HandleShip` moves it (@0x34274-0x342aa),
    // and the frame drawn after the pass shows it moved: the view steps
    // the advance pass in `_DoPlayGameWork`'s order, as a copy run through
    // `Session::game_work` does, not as the separate parts do.
    let mut catalog = armed(1, &[BLASTER]);
    for weapon in &mut catalog.weapons {
        weapon.energy_dmg = 5;
        weapon.impact = 3000;
    }
    catalog.hulls[1].mass = 40;
    let mut view = fighting(catalog, &placed(750, 650, 180));
    view.input(&key(FIRE_KEY, true));
    let npc = |session: &Session| session.npcs()[0].clone();
    let shield = npc(view.session().expect("flying")).reserves.shield.now;
    for _ in 0..60 {
        let session = view.session().expect("flying");
        let mut in_order = session.clone();
        in_order.hold_fire(true, false);
        in_order.game_work(
            Controls::default(),
            view.catalog(),
            &Still,
            Rules::default(),
            &mut NeverFires,
        );
        let mut by_parts = session.clone();
        by_parts.hold_fire(true, false);
        by_parts.tick(Controls::default());
        by_parts.tick_traffic(view.catalog(), &Still, &mut NeverFires);
        by_parts.tick_combat(Rules::default(), &mut NeverFires);
        let drawn_before = sprites(&drawn(&view));
        view.tick(TICK);
        let now = npc(view.session().expect("flying"));
        if now.reserves.shield.now < shield {
            assert_eq!(now.state, npc(&in_order).state, "in the original's order");
            assert_ne!(now.state.position, npc(&by_parts).state.position, "pushed");
            assert_ne!(sprites(&drawn(&view)), drawn_before, "drawn moved");
            return;
        }
    }
    panic!("never hit");
}
