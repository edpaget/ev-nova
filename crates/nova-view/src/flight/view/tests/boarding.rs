use super::*;

// Boarding.

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

#[test]
fn a_tap_of_b_still_boards() {
    // Pressed and released before the next frame, through the press.
    let mut view = beside(750, 750, 0, false);
    tap(&mut view, BOARD_KEY);
    assert!(view.take_boarding().is_some());
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

/// The session's words for the board ([`FlightView::WORDS`]), which the
/// concrete callers hand the board and the plunder: a grant's outfit
/// named by its record, a mission ship's line from the session's
/// strings, and nothing for a session that failed. Phase 12 retires them
/// with the rest of the wording.
#[test]
fn the_sessions_words_name_a_grants_outfit_and_a_mission_ships_line() {
    let named = FakeCatalog {
        outfits: vec![holding(200, ROCKET)],
        ..catalog()
    };
    let view = FlightView::new(named);
    let words = View::WORDS;
    assert_eq!(
        (words.outfit_names)(&view, OutfitId(200)),
        ("gun 200".to_owned(), "guns 200".to_owned())
    );
    assert_eq!(
        (words.outfit_names)(&view, OutfitId(999)),
        (String::new(), String::new()),
        "no record"
    );
    assert_eq!(
        (words.board_line)(&view, nova_sim::MissionBoardLine::Boarded),
        TARGET_BOARDED
    );
    let broken = FlightView::new(FakeCatalog {
        character: Err(StartError::NoCharacter),
        ..catalog()
    });
    assert_eq!(
        (words.board_line)(&broken, nova_sim::MissionBoardLine::Boarded),
        ""
    );
    assert_eq!(
        (words.outfit_names)(&broken, OutfitId(200)),
        (String::new(), String::new())
    );
    assert_eq!(
        (words.plunder_names)(&broken, Some(Good::Commodity(0)), Some(OutfitId(200))),
        (None, None)
    );
}
