use super::*;

// Boarding.

/// Always repels boarders; never asked anything more.
#[derive(Debug)]
struct Repels;

impl BoardingRule for Repels {
    fn repels(&self, _booty: u16) -> bool {
        true
    }

    fn capture_odds(&self, _crew: &nova_sim::board::CaptureCrew, _chance: &mut dyn Chance) -> u8 {
        unreachable!("repelled")
    }

    fn captures(&self, _odds: u8, _chance: &mut dyn Chance) -> bool {
        unreachable!("repelled")
    }

    fn person_credits(&self, _credits: i32, _chance: &mut dyn Chance) -> i64 {
        unreachable!("repelled")
    }

    fn grant(
        &self,
        _grant: &nova_sim::grant::PersonGrant,
        _stock: &[nova_sim::grant::GrantStock],
        _free_mass: i64,
        _grant_max: nova_sim::RuleSource,
        _chance: &mut dyn Chance,
    ) -> Option<nova_sim::grant::Granted> {
        unreachable!("repelled")
    }
}

#[test]
fn b_on_a_disabled_aligned_slow_target_reports_a_boarding_once() {
    let mut view = beside(750, 750, 0, false);
    assert_eq!(view.take_boarding(), None);
    tap(&mut view, BOARD_KEY);
    let boarding = view.take_boarding().expect("boarded").view;
    assert_eq!((boarding.npc, boarding.ship), (NpcId(0), ShipId(129)));
    assert!(boarding.credits >= 1000, "{boarding:?}");
    assert_eq!(view.take_boarding(), None, "once");
    assert_eq!(view.boarding(), Some(boarding), "still under way");
    assert_eq!(view.message(), None);
    let session = view.session().expect("flying");
    assert!(session.npcs()[0].boarded);
    assert_eq!(BOARD_KEY, Key::Char('b'));
}

#[test]
fn each_refused_boarding_says_why_in_the_originals_words() {
    let mut intact = beside(750, 750, 0, true);
    tap(&mut intact, BOARD_KEY);
    assert_eq!(intact.take_boarding(), None);
    assert_lasts(&mut intact, CANT_BOARD, 360);
    let mut far = beside(800, 750, 0, false);
    tap(&mut far, BOARD_KEY);
    assert_eq!(far.message(), Some(NOT_CLOSE_ENOUGH));
    let mut fast = beside(750, 750, 0, false);
    fast.input(&key(Key::Up, true));
    ticks(&mut fast, 10);
    fast.input(&key(Key::Up, false));
    tap(&mut fast, BOARD_KEY);
    assert_eq!(fast.message(), Some(TOO_FAST_TO_BOARD));
    let mut repelled = beside(750, 750, 0, false).with_boarding_rule(Rc::new(Repels));
    tap(&mut repelled, BOARD_KEY);
    assert_eq!(repelled.take_boarding(), None);
    assert_lasts(&mut repelled, REPELLED, MESSAGE_TICKS);
    assert_eq!(MESSAGE_TICKS, 240);
    assert_eq!(
        [CANT_BOARD, NOT_CLOSE_ENOUGH, TOO_FAST_TO_BOARD, REPELLED,],
        [
            "You can't board this ship.",
            "You're not close enough to board this ship.",
            "You're moving too fast to board this ship.",
            "You were repelled while attempting to board this ship.",
        ]
    );
}

#[test]
fn a_misaligned_or_missing_target_says_nothing() {
    let mut aside = beside(750, 750, 90, false);
    tap(&mut aside, BOARD_KEY);
    assert_eq!(aside.message(), None);
    assert_eq!(aside.take_boarding(), None);
    let mut alone = flight();
    tap(&mut alone, BOARD_KEY);
    assert_eq!(alone.message(), None);
    assert_eq!(board_refusal_message(BoardRefusal::NoTarget), None);
    assert_eq!(board_refusal_message(BoardRefusal::Misaligned), None);
}

#[test]
fn a_cloaked_board_says_nothing() {
    // `_HandlePlayerBoardAttempt` returns at `_ShipIsCloaked` (@0x6503f)
    // with no line.
    assert_eq!(board_refusal_message(BoardRefusal::Cloaked), None);
}

#[test]
fn the_credits_message_says_no_amount() {
    // #116, " ", `*0x3b557c` (#33, always the plural), " ", #108
    // (`_DoPlunderDialog` @0x93b40-0x93bec).
    assert_eq!(STOLE_CREDITS, "You stole all the credits from this ship.");
    for credits in [1, 5750, 1_000_000] {
        assert_eq!(
            plunder_message(Taken::Credits(credits), None, None).as_deref(),
            Some(STOLE_CREDITS),
            "{credits}"
        );
    }
}

// The Board key's latch (`*0xdee18`).

/// [`beside`]'s disabled trader under the player, before the player
/// targets it.
fn beside_untargeted() -> View {
    let (_, chance) = scripted(&placed(750, 750, 0));
    let mut view = FlightView::new(boardable())
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_disable_rule(Rc::new(DisablesTheStrong));
    view.tick(TICK);
    view
}

/// How many refusal beeps `view` raised since its sounds were last
/// taken.
fn refusals(view: &mut View) -> usize {
    view.take_sounds()
        .into_iter()
        .filter(|sound| *sound == Sound::Sim(SimSound::Refused))
        .count()
}

#[test]
fn holding_b_with_no_target_boards_once_a_target_is_taken() {
    // No target returns before the latch is set (@0x65031 → @0x6504c),
    // and `_HandlePlayer` tries again every frame B is down
    // (@0x6a98b-0x6a99c).
    let mut view = beside_untargeted();
    view.input(&key(BOARD_KEY, true));
    ticks(&mut view, 3);
    assert_eq!(view.take_boarding(), None, "no target");
    tap(&mut view, TARGET_KEY);
    assert_eq!(view.take_boarding(), None, "not yet");
    ticks(&mut view, 1);
    assert!(view.take_boarding().is_some(), "the next frame boards");
}

#[test]
fn holding_b_after_a_refusal_says_it_once_until_released() {
    let mut view = beside(800, 750, 0, false);
    view.take_sounds();
    view.input(&key(BOARD_KEY, true));
    assert_eq!(view.message(), Some(NOT_CLOSE_ENOUGH));
    ticks(&mut view, 5);
    assert_eq!(refusals(&mut view), 1, "latched (@0x6504c)");
    view.input(&key(BOARD_KEY, false));
    ticks(&mut view, 1);
    view.input(&key(BOARD_KEY, true));
    assert_eq!(refusals(&mut view), 1, "a fresh press");
    view.input(&key(BOARD_KEY, false));
    ticks(&mut view, 1);
    assert_eq!(refusals(&mut view), 0);
}

#[test]
fn holding_b_over_a_former_escort_repairs_it_once_and_latches_until_released() {
    // A board that passes the checks latches (`*0xdee18` @0x6504c), so the
    // escort repaired, intact now, isn't boarded again while B is held.
    let mut view = beside(750, 750, 0, false);
    let Ok(session) = &mut view.sim.session else {
        panic!("flying");
    };
    let record = nova_sim::Escort {
        ship: ShipId(129),
        reserves: nova_sim::reserves::Reserves::full(30.0, 45.0, 300.0),
        order: None,
        carried: false,
        wage: Some(30),
        person: None,
        mark: None,
    };
    nova_sim::fixture::remember_escort(session, NpcId(0), record);
    view.take_sounds();
    view.input(&key(BOARD_KEY, true));
    ticks(&mut view, 5);
    assert_eq!(view.message(), Some(ESCORT_REPAIRED));
    let sounds = view.take_sounds();
    let count = |sound: SimSound| {
        sounds
            .iter()
            .filter(|&&heard| heard == Sound::Sim(sound))
            .count()
    };
    assert_eq!(count(SimSound::EscortRepaired), 1, "{sounds:?}");
    assert_eq!(count(SimSound::Refused), 0, "no second attempt: {sounds:?}");
    assert_eq!(view.take_boarding(), None, "no plunder");
    view.input(&key(BOARD_KEY, false));
    ticks(&mut view, 1);
    view.input(&key(BOARD_KEY, true));
    assert_eq!(refusals(&mut view), 1, "released and pressed again");
}

#[test]
fn a_tap_of_b_still_boards() {
    // Pressed and released before the next frame, through the press.
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    assert!(view.take_boarding().is_some());
}

#[test]
fn a_held_b_while_cloaked_boards_when_the_cloak_drops() {
    // The cloak returns before the latch is set (@0x6503f → @0x6504c).
    let mut view = beside(750, 750, 0, false);
    let cloak = |view: &mut View, to| {
        let Ok(session) = &mut view.sim.session else {
            panic!("flying");
        };
        nova_sim::fixture::set_player_cloak(session, to);
    };
    cloak(
        &mut view,
        nova_sim::cloak::Cloak {
            fade: nova_sim::cloak::FADE_FULL,
            direction: 0,
        },
    );
    view.input(&key(BOARD_KEY, true));
    ticks(&mut view, 2);
    assert_eq!(view.take_boarding(), None, "cloaked");
    cloak(&mut view, nova_sim::cloak::Cloak::default());
    ticks(&mut view, 1);
    assert!(view.take_boarding().is_some(), "the cloak dropped");
}

#[test]
fn boarding_keeps_the_message() {
    let mut view = beside(750, 750, 0, false);
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, BOARD_KEY);
    view.take_boarding().expect("boarded");
    assert_eq!(view.message(), Some(MISSION_FAILED), "under the dialog");
    assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
    assert_lasts(&mut view, MISSION_FAILED, 240);
}

#[test]
fn plundering_goes_through_the_session_and_says_what_it_took() {
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    let boarding = view.take_boarding().expect("boarded").view;
    let cash = view.pilot().expect("a pilot").cash();
    assert_eq!(
        view.plunder(Take::Credits),
        Taken::Credits(boarding.credits)
    );
    assert_eq!(
        view.pilot().expect("a pilot").cash(),
        cash + boarding.credits
    );
    assert_eq!(view.boarding().map(|now| now.credits), Some(0));
    assert_lasts(&mut view, STOLE_CREDITS, 239);
    assert_eq!(view.plunder(Take::Abort), Taken::Aborted);
    assert_eq!(view.boarding(), None);
}

/// The plunder dialog's loop counts the message line down a tick on each
/// press that leaves it open (`_HandleComm` @0x949a1): the line already
/// shown, and a take's own, which `_DisplayComm` sets to 240 first.
#[test]
fn each_press_that_keeps_the_plunder_dialog_open_counts_the_message_a_tick() {
    let mut view = beside(750, 750, 0, false);
    view.say_for(MISSION_FAILED, 240);
    tap(&mut view, BOARD_KEY);
    view.take_boarding().expect("boarded");
    assert_eq!(view.plunder(Take::Miss), Taken::Missed);
    assert_eq!(view.plunder(Take::Miss), Taken::Missed);
    assert_lasts(&mut view, MISSION_FAILED, 238);
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    view.take_boarding().expect("boarded");
    assert!(matches!(view.plunder(Take::Credits), Taken::Credits(_)));
    let mut line = nova_sim::CommLine::default();
    line.show(240);
    line.tick();
    assert_eq!(
        view.session().expect("flying").comm_line(),
        line,
        "the session's line in step with the message"
    );
    let said = view.message().expect("said").to_owned();
    assert_lasts(&mut view, &said, 239);
}

/// `_HandleComm` hides the floating map once `TickCount` is more than
/// 500 past its show (@0x30262-0x30285), at each press that keeps the
/// dialog open; the time the dialog has been open counts.
#[test]
fn a_press_hides_the_floating_map_once_500_ticks_have_passed() {
    for (held, up) in [
        (Duration::from_secs(8), true),
        (Duration::from_secs(9), false),
    ] {
        let mut view = beside(750, 750, 0, false);
        tap(&mut view, BOARD_KEY);
        view.take_boarding().expect("boarded");
        let now = crate::time::sixtieths(view.elapsed);
        view.floating.show(now);
        assert_eq!(view.plunder_held(Take::Miss, held), Taken::Missed);
        assert_eq!(view.floating.is_up(), up, "{held:?}");
    }
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    view.take_boarding().expect("boarded");
    let now = crate::time::sixtieths(view.elapsed);
    view.floating.show(now);
    assert_eq!(
        view.plunder_held(Take::Abort, Duration::from_secs(60)),
        Taken::Aborted
    );
    assert!(
        view.floating.is_up(),
        "a press that closes the dialog counts nothing"
    );
}

/// Letting a boarding go with no dialog to show it is silent.
#[test]
fn a_boarding_let_go_is_silent() {
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    view.take_boarding().expect("boarded");
    view.take_sounds();
    view.let_go_of_boarding();
    assert_eq!(view.boarding(), None);
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn the_plunder_messages_are_the_originals() {
    let food = Some("Food");
    let message = |taken| plunder_message(taken, food, Some("Rockets"));
    let cargo = |stored| Taken::Cargo {
        good: Good::Commodity(0),
        stored,
    };
    assert_eq!(message(cargo(0)).as_deref(), Some(NO_CARGO_STORED));
    assert_eq!(
        message(cargo(1)).as_deref(),
        Some("You salvaged 1 ton of Food from this ship.")
    );
    assert_eq!(
        message(cargo(12)).as_deref(),
        Some("You salvaged 12 tons of Food from this ship.")
    );
    assert_eq!(
        plunder_message(cargo(3), None, None).as_deref(),
        Some("You salvaged 3 tons of cargo from this ship."),
        "a good with no name"
    );
    assert_eq!(
        message(Taken::Credits(5750)).as_deref(),
        Some(STOLE_CREDITS)
    );
    let ammo = |count| Taken::Ammo {
        outfit: OutfitId(310),
        count,
    };
    assert_eq!(message(ammo(0)).as_deref(), Some(NO_AMMO_STORED));
    assert_eq!(
        message(ammo(7)).as_deref(),
        Some("You salvaged 7 Rockets from this ship.")
    );
    let energy = |stored, full| Taken::Energy {
        offered: 170,
        stored,
        full,
    };
    assert_eq!(message(energy(0, true)).as_deref(), Some(NO_ENERGY_STORED));
    assert_eq!(
        message(energy(170, false)).as_deref(),
        Some(ALL_ENERGY_STORED)
    );
    assert_eq!(message(energy(100, true)).as_deref(), Some(ENERGY_FILLED));
    assert_eq!(message(Taken::Tripped).as_deref(), Some(SELF_DESTRUCT));
    assert_eq!(
        message(Taken::CaptureFailed).as_deref(),
        Some(CAPTURE_FAILED)
    );
    assert_eq!(message(Taken::FleetFull).as_deref(), Some(FLEET_FULL));
    assert_eq!(message(Taken::Escorted).as_deref(), Some(ASSIGNED_ESCORT));
    for silent in [
        Taken::Captured,
        Taken::Aborted,
        Taken::Nothing,
        Taken::Missed,
    ] {
        assert_eq!(message(silent), None, "{silent:?}");
    }
    assert_eq!(
        [
            NO_CARGO_STORED,
            NO_AMMO_STORED,
            NO_ENERGY_STORED,
            ALL_ENERGY_STORED,
            ENERGY_FILLED,
            SELF_DESTRUCT,
            CAPTURE_FAILED,
            FLEET_FULL,
            ASSIGNED_ESCORT,
            RETAINED_OLD_SHIP,
            LOST_OLD_SHIP,
        ],
        [
            "You couldn't store any of the cargo you plundered from this ship.",
            "You couldn't store any of the ammo you plundered from this ship.",
            "You couldn't store any of the energy you transferred from this ship.",
            "You transferred all of this ship's energy to your reactors and batteries.",
            "You filled your reactors and batteries with energy from this ship.",
            "Oops! You tripped this ship's security self-destruct mechanism.",
            "Your attempt to capture this ship was unsuccessful.",
            "You already have the maximum possible number of escorts.",
            "You assigned this ship to your fleet of escorts.",
            "You retained your old ship as an escort.",
            "You were unable to retain your old ship as an escort.",
        ]
    );
    assert_eq!(assigned_message(Assigned::Escort), ASSIGNED_ESCORT);
    assert_eq!(assigned_message(Assigned::MyShip), RETAINED_OLD_SHIP);
    assert_eq!(assigned_message(Assigned::Abandoned), LOST_OLD_SHIP);
}

#[test]
fn the_target_panel_is_redrawn_once_the_plunder_dialog_closes() {
    let mut view = beside(750, 750, 0, true);
    view.tick(TICK);
    while now_tick(&view) % TARGET_PERIOD_TICKS != 1 {
        view.tick(TICK);
    }
    let mut view = view.with_disable_rule(Rc::new(DisablesTheStrong));
    view.tick(TICK);
    assert_eq!(status_now(&view), Some(nova_sim::TargetStatus::Disabled));
    assert_ne!(status_shown(&view), status_now(&view), "held");
    tap(&mut view, BOARD_KEY);
    let boarding = view.take_boarding().expect("boarded").view;
    assert_eq!(
        view.plunder(Take::Credits),
        Taken::Credits(boarding.credits)
    );
    assert_eq!(
        *view.target_shown(&view.frame_now()),
        view.target_now(&view.frame_now()),
        "redrawn at once"
    );
    assert_eq!(status_shown(&view), Some(nova_sim::TargetStatus::Disabled));
}

#[test]
fn use_as_escort_says_so() {
    let mut view = captured();
    view.tick(TICK / 2);
    let alpha = view.alpha();
    assert!(alpha > 0.0, "between steps");
    assert_eq!(view.assign(Assignment::Escort), Some(Assigned::Escort));
    assert_eq!(view.alpha(), alpha, "the old ship flies on");
    let pilot = view.pilot().expect("a pilot");
    assert_eq!(pilot.escorts().len(), 1);
    assert!(view.take_save_due());
    assert_lasts(&mut view, ASSIGNED_ESCORT, 240);
}

#[test]
fn use_as_my_ship_flies_and_draws_the_captured_ship() {
    let mut view = captured();
    view.tick(TICK / 2);
    assert!(view.alpha() > 0.0, "between steps");
    let mut asked = view.catalog().sheets_asked.borrow().clone();
    assert_eq!(view.assign(Assignment::MyShip), Some(Assigned::MyShip));
    assert_eq!(view.alpha(), 0.0, "drawn where it is");
    assert_eq!(view.message(), Some(RETAINED_OLD_SHIP));
    assert_eq!(view.session().expect("flying").ship(), ShipId(129));
    asked.push(ShipId(129));
    assert_eq!(*view.catalog().sheets_asked.borrow(), asked, "read once");
    view.settle_script();
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        asked,
        "not again once settled"
    );
    let drawn_player = sprites(&drawn(&view))
        .into_iter()
        .filter(|&(_, center)| center == PLAYER_AT)
        .map(|(image, _)| image)
        .collect::<Vec<_>>();
    assert_eq!(
        drawn_player,
        [ImageKey::sprite(2001, 0), ImageKey::sprite(2201, 0)],
        "ship 129's sheet, and its lights"
    );
    assert_eq!(view.assign(Assignment::MyShip), None, "nothing awaits");
    assert_lasts(&mut view, RETAINED_OLD_SHIP, 360);
}

#[test]
fn the_plunder_taken_is_named_as_the_session_names_it() {
    let mut catalog = boardable();
    catalog.fields.holds = 20;
    catalog.dudes[0].1.booty = 0x0041;
    catalog.commodities = CommodityStrings {
        names: vec!["Food".to_owned()],
        name_patches: Default::default(),
        base_prices: vec!["75".to_owned()],
        price_patches: Default::default(),
    };
    for record in &mut catalog.ships {
        if record.id == ShipId(129) {
            record.fields.holds = 20;
        }
    }
    catalog.weapons = vec![nova_sim::WeaponRecord {
        ammo_type: 12,
        ..gun(ROCKET, 0x0002, -1, -1)
    }];
    let rockets = |id, ammo| nova_sim::HullRecord {
        strength: 5,
        weapons: vec![nova_sim::StockWeapon {
            weapon: ROCKET,
            count: 1,
            ammo,
        }],
        ..hull_of(id, &[])
    };
    catalog.hulls = vec![rockets(128, 0), rockets(129, 2)];
    catalog.hulls[0].strength = 0;
    catalog.outfits = vec![
        holding(250, ROCKET),
        OutfitRecord {
            id: OutfitId(310),
            name: "Rockets".to_owned(),
            short_name: "Rockets".to_owned(),
            disp_weight: 0,
            mass: 1,
            tech_level: 1,
            max: 10,
            flags: 0,
            cost: 100,
            mods: [
                (nova_sim::combat::armament::MOD_AMMO, 140),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "rocket".to_owned(),
            lc_plural: "rockets".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        },
    ];
    let (_, chance) = scripted(&placed(750, 750, 0));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_disable_rule(Rc::new(DisablesTheStrong));
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, BOARD_KEY);
    let boarding = view.take_boarding().expect("boarded");
    assert_eq!(boarding.view.cargo, Some((Good::Commodity(0), 19)));
    assert_eq!(boarding.view.ammo, Some((OutfitId(310), 2)));
    let named = |shown: &PlunderShown| (shown.good.clone(), shown.outfit.clone());
    assert_eq!(
        named(&boarding),
        (Some("Food".to_owned()), Some("Rockets".to_owned())),
        "the dialog names them as the messages do"
    );
    assert_eq!(view.plunder_shown().as_ref(), Some(&boarding));
    view.plunder(Take::Cargo);
    assert_eq!(
        view.message(),
        Some("You salvaged 19 tons of Food from this ship.")
    );
    let shown = view.plunder_shown().expect("still under way");
    assert_eq!(named(&shown), (None, Some("Rockets".to_owned())));
    view.plunder(Take::Ammo);
    assert_eq!(
        view.message(),
        Some("You salvaged 2 Rockets from this ship.")
    );
    let shown = view.plunder_shown().expect("still under way");
    assert_eq!(named(&shown), (None, None), "nothing left to name");
}

#[test]
fn the_help_line_names_board() {
    assert!(HELP.contains("B: board"), "{HELP}");
}

// Hailing.

#[test]
fn the_screens_hail_options_are_listed() {
    let mut view = hailable(true);
    view = view.with_hail_options(HailOptions::empty());
    tap(&mut view, HAIL_KEY);
    assert_eq!(view.take_hail().expect("hailed").options, []);
}

#[test]
fn the_help_line_names_hail() {
    assert!(HELP.contains("Y: hail"), "{HELP}");
}

// Boarding grants.

/// [`granting`]'s flight, by `rule`, with Ace listed and placed on
/// the player, targeted and boarded.
fn boarded_ace(count: i16, rule: Rc<dyn BoardingRule>) -> View {
    let (_, chance) = scripted(&[99, 750, 750, 0]);
    let mut view = FlightView::new(granting(count))
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_boarding_rule(rule);
    view.tick(TICK);
    tap(&mut view, TARGET_KEY);
    tap(&mut view, BOARD_KEY);
    assert!(view.take_boarding().is_some(), "boarded");
    view
}

#[test]
fn boarding_a_granting_person_says_what_it_retrieved_for_240_ticks() {
    let mut view = boarded_ace(2, Rc::new(Grants));
    let said = "You retrieved two spare parts from this ship.";
    assert_eq!(view.message(), Some(said));
    assert_eq!(view.pilot().expect("a pilot").owned(OutfitId(200)), 2);
    assert_lasts(&mut view, said, GRANT_TICKS);
    let view = boarded_ace(1, Rc::new(Grants));
    assert_eq!(
        view.message(),
        Some("You retrieved a spare part from this ship.")
    );
}

#[test]
fn boarding_a_person_that_grants_nothing_says_nothing() {
    let view = boarded_ace(2, Rc::new(Sure));
    assert_eq!(view.message(), None);
}
