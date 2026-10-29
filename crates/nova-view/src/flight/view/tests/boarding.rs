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
            CAPTURE_DECLINED,
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
            "You decided not to capture this ship after all.",
        ]
    );
    assert_eq!(assigned_message(Assigned::Escort), ASSIGNED_ESCORT);
    assert_eq!(assigned_message(Assigned::MyShip), RETAINED_OLD_SHIP);
    assert_eq!(assigned_message(Assigned::Abandoned), LOST_OLD_SHIP);
    assert_eq!(assigned_message(Assigned::Declined), CAPTURE_DECLINED);
    assert_eq!(assigned_message(Assigned::Naming), "", "says nothing");
    assert_eq!(assigned_ticks(Assigned::Declined), MESSAGE_TICKS);
    assert_eq!(assigned_ticks(Assigned::Escort), MESSAGE_TICKS);
    assert_eq!(assigned_ticks(Assigned::MyShip), OLD_SHIP_TICKS);
    assert_eq!(assigned_ticks(Assigned::Abandoned), OLD_SHIP_TICKS);
}

#[test]
fn the_help_line_names_board() {
    assert!(HELP.contains("B: board"), "{HELP}");
}

// Hailing.

#[test]
fn the_help_line_names_hail() {
    assert!(HELP.contains("Y: hail"), "{HELP}");
}
