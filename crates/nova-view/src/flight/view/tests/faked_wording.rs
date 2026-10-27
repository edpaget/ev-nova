//! The words of the flight's messages over a scripted flight's names
//! ([`FakeFlight::names_mut`], rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks):
//! the session's messages (`sim_message_text`), a mission ship's board
//! lines (`mission_board_message`) and the lines that name a stellar
//! (`message_text`), each naming what the flight names. That the session
//! names each as its records do is nova-sim's; that the adapter forwards
//! the session's names is `tests/names.rs`'.

use super::*;
use nova_sim::SelfDestructLine;
use nova_sim::flight_frame::FlightEvent;

/// The words `message` says over `names`, millions by the engine.
fn text(message: &SimMessage, names: &FakeFlight) -> Option<String> {
    sim_message_text(message, names, RuleSource::Engine)
}

/// A flight naming the player's ship Kestrel, ship class 128 Shuttle and
/// 129 Viper (long name Federation Viper), outfit 275 "bomb", "bombs",
/// `jünk` 150 "bio-weapons", and `STR#` 4001's cargoes Food and
/// *Passengers.
fn naming() -> FakeFlight {
    let mut fake = FakeFlight::new();
    let names = fake.names_mut();
    names.player_ship_name = Some("Kestrel".to_owned());
    names.ship_names = vec![
        (ShipId(128), "Shuttle".to_owned()),
        (ShipId(129), "Viper".to_owned()),
    ];
    names.ship_long_names = vec![(ShipId(129), "Federation Viper".to_owned())];
    names.outfit_lc_names = vec![(OutfitId(275), ("bomb".to_owned(), "bombs".to_owned()))];
    names.junk_lc_names = vec![(nova_sim::JunkId(150), "bio-weapons".to_owned())];
    names.cargo_names = vec!["Food".to_owned(), "*Passengers".to_owned()];
    fake
}

#[test]
fn each_jettison_message_is_worded_and_the_arrivals_are_not_among_them() {
    let names = FakeFlight::new();
    let said = |message| text(&message, &names);
    assert_eq!(
        said(SimMessage::MissionFailed).as_deref(),
        Some(MISSION_FAILED)
    );
    assert_eq!(
        said(SimMessage::CargoJettisoned).as_deref(),
        Some(CARGO_JETTISONED)
    );
    assert_eq!(
        said(SimMessage::NonMissionCargoJettisoned).as_deref(),
        Some(NON_MISSION_CARGO_JETTISONED)
    );
    assert_eq!(said(SimMessage::Arrived(SystemId(130))), None);
    assert_eq!(said(SimMessage::ExitedHypergate(SystemId(130))), None);
    assert_eq!(said(SimMessage::PassedWormhole(SystemId(130))), None);
    assert_eq!(said(SimMessage::Buoy(HELLO.to_owned())), None);
    for message in [
        SimMessage::MissionFailed,
        SimMessage::CargoJettisoned,
        SimMessage::NonMissionCargoJettisoned,
        SimMessage::MissionScanned,
        SimMessage::Smuggling {
            scanner: ShipId(129),
            ship_name: None,
            fine: 0,
        },
    ] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn a_former_escort_rejoining_says_escort_or_fighter_repaired() {
    assert_eq!(ESCORT_REPAIRED, "Escort repaired.");
    assert_eq!(FIGHTER_REPAIRED, "Fighter repaired.");
    let names = FakeFlight::new();
    assert_eq!(
        text(&SimMessage::EscortRepaired, &names).as_deref(),
        Some(ESCORT_REPAIRED)
    );
    assert_eq!(
        text(&SimMessage::FighterRepaired, &names).as_deref(),
        Some(FIGHTER_REPAIRED)
    );
    for message in [SimMessage::EscortRepaired, SimMessage::FighterRepaired] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn fighter_captured_says_the_originals_words() {
    // `STR#` 2002 #129.
    assert_eq!(FIGHTER_CAPTURED, "Fighter captured.");
    assert_eq!(
        text(&SimMessage::FighterCaptured, &FakeFlight::new()).as_deref(),
        Some(FIGHTER_CAPTURED)
    );
    assert_eq!(arrival_lead(&SimMessage::FighterCaptured), None);
}

#[test]
fn a_sabotage_notice_names_the_outfit_for_its_count() {
    assert_eq!(
        (YOUR, HAS_EXPLODED, HAVE_EXPLODED),
        ("Your", "has exploded!", "have exploded!")
    );
    let names = naming();
    let sabotaged = |one| SimMessage::Sabotaged {
        outfit: Some((OutfitId(275), one)),
    };
    assert_eq!(
        text(&sabotaged(true), &names).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        text(&sabotaged(false), &names).as_deref(),
        Some("Your bombs have exploded!")
    );
    assert_eq!(arrival_lead(&sabotaged(true)), None);
}

#[test]
fn a_bomb_notice_names_the_outfit_with_no_empty_name_check() {
    let names = naming();
    let exploded = |outfit, one| SimMessage::BombExploded {
        outfit: OutfitId(outfit),
        one,
    };
    assert_eq!(
        text(&exploded(275, true), &names).as_deref(),
        Some("Your bomb has exploded!")
    );
    assert_eq!(
        text(&exploded(275, false), &names).as_deref(),
        Some("Your bombs have exploded!")
    );
    assert_eq!(
        text(&exploded(999, true), &names).as_deref(),
        Some("Your  has exploded!"),
        "an empty name is spelled out as it is"
    );
    assert_eq!(arrival_lead(&exploded(275, true)), None);
}

#[test]
fn the_leet_sabotage_notice_says_someone_set_up_the_bomb() {
    assert_eq!(
        text(&SimMessage::BombSetUp, &FakeFlight::new()).as_deref(),
        Some("Someone set up us the bomb!")
    );
    assert_eq!(arrival_lead(&SimMessage::BombSetUp), None);
}

#[test]
fn a_sabotage_notice_without_an_outfit_says_a_bomb_exploded() {
    let message = SimMessage::Sabotaged { outfit: None };
    assert_eq!(
        text(&message, &naming()).as_deref(),
        Some("A bomb has exploded onboard your ship!")
    );
    assert_eq!(
        BOMB_EXPLODED_ONBOARD,
        "A bomb has exploded onboard your ship!"
    );
}

#[test]
fn a_collapsed_jump_says_the_hyperspace_field_collapsed() {
    assert_eq!(
        text(&SimMessage::JumpCollapsed, &FakeFlight::new()).as_deref(),
        Some("Ship disabled - hyperspace field collapsed.")
    );
    assert_eq!(
        JUMP_COLLAPSED,
        "Ship disabled - hyperspace field collapsed."
    );
    assert_eq!(arrival_lead(&SimMessage::JumpCollapsed), None);
}

#[test]
fn the_players_repair_droids_say_propulsion_systems_repaired() {
    assert_eq!(
        text(&SimMessage::PropulsionRepaired, &FakeFlight::new()).as_deref(),
        Some("Propulsion systems repaired.")
    );
    assert_eq!(PROPULSION_REPAIRED, "Propulsion systems repaired.");
    assert_eq!(arrival_lead(&SimMessage::PropulsionRepaired), None);
}

#[test]
fn the_escape_ship_says_escaping_in_its_long_name() {
    assert_eq!(ESCAPING_IN, "Escaping in");
    let names = naming();
    let escaping = |ship| SimMessage::EscapingIn { ship: ShipId(ship) };
    assert_eq!(
        text(&escaping(129), &names).as_deref(),
        Some("Escaping in Federation Viper.")
    );
    assert_eq!(
        text(&escaping(999), &names).as_deref(),
        Some("Escaping in ."),
        "a class with no long name"
    );
    assert_eq!(arrival_lead(&escaping(129)), None);
}

#[test]
fn a_mission_ship_boarded_on_the_assist_path_says_its_line() {
    assert_eq!(TARGET_BOARDED, "Target ship has been boarded.");
    assert_eq!(
        refuelled_message("Freighter", "Kestrel"),
        "Freighter:  Energy transfer complete, Kestrel."
    );
    let names = naming();
    assert_eq!(
        mission_board_message(MissionBoardLine::Boarded, &names),
        TARGET_BOARDED
    );
    assert_eq!(
        mission_board_message(MissionBoardLine::Refuelled { ship: ShipId(129) }, &names),
        "Viper:  Energy transfer complete, Kestrel.",
        "the ship's class and the player's ship as the flight names them"
    );
}

#[test]
fn a_pickup_ships_line_reads_the_cargo_name_from_str_4001() {
    let names = naming();
    let said = |cargo| mission_board_message(MissionBoardLine::Retrieved { cargo }, &names);
    assert_eq!(said(Some(0)), "You retrieved  the Food from this ship.");
    assert_eq!(said(Some(1)), "You retrieved  Passengers from this ship.");
    assert_eq!(said(Some(2)), "You retrieved  the  from this ship.");
    assert_eq!(said(None), "You retrieved  the  from this ship.");
    assert_eq!(
        said(Some(256)),
        "You retrieved  the  from this ship.",
        "no cargo past 255"
    );
}

#[test]
fn a_mission_ship_lost_says_str_2002_284() {
    assert_eq!(
        text(&SimMessage::MissionShipLost, &FakeFlight::new()).as_deref(),
        Some(MISSION_FAILED)
    );
    assert_eq!(arrival_lead(&SimMessage::MissionShipLost), None);
}

#[test]
fn a_fleets_arrival_quote_says_its_line() {
    let quote = SimMessage::FleetArrivalQuote("Hail 9".to_owned());
    assert_eq!(text(&quote, &FakeFlight::new()).as_deref(), Some("Hail 9"));
    assert_eq!(arrival_lead(&quote), None);
}

#[test]
fn a_distress_call_a_pre_scan_hail_reinforcements_and_wait_for_me_say_their_text_as_given() {
    let names = FakeFlight::new();
    let wait = "Viper:  Wait for me!";
    let help = "Ship 129:  Help!";
    let hail = "Ship 129:  Kestrel, halt.";
    let coming = "Sensors detect Federation reinforcement fleet approaching.";
    for (message, said) in [
        (SimMessage::CallForHelp(help.to_owned()), help),
        (SimMessage::ScanHail(hail.to_owned()), hail),
        (SimMessage::WaitForMe(wait.to_owned()), wait),
        (
            SimMessage::ReinforcementsApproaching(coming.to_owned()),
            coming,
        ),
    ] {
        assert_eq!(text(&message, &names).as_deref(), Some(said));
        assert_eq!(arrival_lead(&message), None);
    }
}

#[test]
fn the_players_downed_lines_are_str_2002_287_288_and_284() {
    let names = FakeFlight::new();
    for (message, said) in [
        (SimMessage::ShipDisabled, "Ship disabled."),
        (SimMessage::ShipDestroyed, "Ship destroyed."),
        (SimMessage::DownedMissionFailed, "Mission failed."),
    ] {
        assert_eq!(text(&message, &names).as_deref(), Some(said), "{message:?}");
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

#[test]
fn the_no_active_missions_message_is_str_2002_354() {
    assert_eq!(
        text(&SimMessage::NoActiveMissions, &FakeFlight::new()).as_deref(),
        Some(NO_ACTIVE_MISSIONS)
    );
    assert_eq!(arrival_lead(&SimMessage::NoActiveMissions), None);
}

#[test]
fn the_deadline_message_is_str_2002_285() {
    assert_eq!(
        text(&SimMessage::DeadlineMissed, &FakeFlight::new()).as_deref(),
        Some(TIME_LIMIT_EXCEEDED)
    );
    assert_eq!(arrival_lead(&SimMessage::DeadlineMissed), None);
    assert_eq!(TIME_LIMIT_EXCEEDED, "Time limit exceeded - mission failed.");
}

/// A contraband notice names the scanner by its person's name when it
/// is one (`_ScanPlayer` @0x7ebd1-0x7ec0c), else its ship class, and
/// the outfit or `jünk` as the flight names them.
#[test]
fn a_contraband_notice_reads_its_names_from_the_flight() {
    let mut names = naming();
    names.names_mut().outfit_lc_names =
        vec![(OutfitId(275), ("blaster".to_owned(), "blasters".to_owned()))];
    let outfit = |person: Option<&str>| SimMessage::IllegalOutfit {
        scanner: ShipId(128),
        person: person.map(str::to_owned),
        outfit: OutfitId(275),
        one: true,
        fine: 500,
    };
    assert_eq!(
        text(&outfit(Some("Ace")), &names),
        Some(illegal_outfit_message(
            "Ace", "Kestrel", true, "blaster", "blasters", 500
        ))
    );
    assert_eq!(
        text(&outfit(None), &names),
        Some(illegal_outfit_message(
            "Shuttle", "Kestrel", true, "blaster", "blasters", 500
        ))
    );
    let junk = |id: i16| SimMessage::IllegalJunk {
        scanner: ShipId(128),
        person: Some("Ace".to_owned()),
        junk: nova_sim::JunkId(id),
        fine: 0,
    };
    assert_eq!(
        text(&junk(150), &names),
        Some(illegal_junk_message("Ace", "Kestrel", "bio-weapons", 0))
    );
    assert_eq!(
        text(&junk(151), &names),
        Some(illegal_junk_message("Ace", "Kestrel", "", 0)),
        "no name"
    );
    assert_eq!(arrival_lead(&junk(150)), None);
    assert_eq!(arrival_lead(&outfit(None)), None);
}

/// The found mission's drawn ship name names the scanner when it has
/// one (`_ScanPlayer` @0x7e7ce); otherwise its ship class does.
#[test]
fn the_smuggling_notice_names_the_scanner_by_the_found_missions_ship_name() {
    let names = naming();
    let said = |ship_name: Option<&str>| {
        text(
            &SimMessage::Smuggling {
                scanner: ShipId(128),
                ship_name: ship_name.map(str::to_owned),
                fine: 500,
            },
            &names,
        )
    };
    assert_eq!(
        said(Some("Rover")),
        Some(smuggling_message("Rover", "Kestrel", 500))
    );
    assert_eq!(
        said(None),
        Some(smuggling_message("Shuttle", "Kestrel", 500))
    );
}

/// A plunder's lines read as `_AIPlunderShipContents` builds them, the
/// numbers' millions by the reading given, and lead no arrival.
#[test]
fn a_plunders_lines_are_its_cargo_and_its_stolen_ship() {
    let names = FakeFlight::new();
    let stolen = SimMessage::CargoStolen {
        tons: 2,
        credits: 1_234_567,
    };
    assert_eq!(
        sim_message_text(&stolen, &names, RuleSource::Engine).as_deref(),
        Some("2 tons of cargo and 1.23M credits stolen!")
    );
    assert_eq!(
        sim_message_text(&stolen, &names, RuleSource::Bible).as_deref(),
        Some("2 tons of cargo and 1,234,567 credits stolen!")
    );
    assert_eq!(
        text(&SimMessage::ShipStolen { fighter: true }, &names).as_deref(),
        Some("Fighter stolen!")
    );
    assert_eq!(arrival_lead(&stolen), None);
}

#[test]
fn the_self_destructs_lines_say_its_stock_words() {
    let names = FakeFlight::new();
    let said = |line| text(&SimMessage::SelfDestruct(line), &names);
    assert_eq!(
        said(SelfDestructLine::Initiated).as_deref(),
        Some("Self-destruct sequence initiated.")
    );
    assert_eq!(
        said(SelfDestructLine::In {
            seconds: 4,
            singular: false
        })
        .as_deref(),
        Some("Self-destruct in 4 seconds.")
    );
    assert_eq!(
        said(SelfDestructLine::In {
            seconds: 1,
            singular: false
        })
        .as_deref(),
        Some("Self-destruct in 1 seconds."),
        "the engine always says #389"
    );
    assert_eq!(
        said(SelfDestructLine::In {
            seconds: 1,
            singular: true
        })
        .as_deref(),
        Some("Self-destruct in 1 second.")
    );
    assert_eq!(
        said(SelfDestructLine::Cancelled).as_deref(),
        Some("Self-destruct sequence cancelled.")
    );
    assert_eq!(
        said(SelfDestructLine::NiceDay).as_deref(),
        Some("Have a nice day.")
    );
    for message in [
        SimMessage::SelfDestruct(SelfDestructLine::Initiated),
        SimMessage::SelfDestruct(SelfDestructLine::In {
            seconds: 2,
            singular: false,
        }),
        SimMessage::SelfDestruct(SelfDestructLine::Cancelled),
        SimMessage::SelfDestruct(SelfDestructLine::NiceDay),
    ] {
        assert_eq!(arrival_lead(&message), None, "{message:?}");
    }
}

/// The messages that name a stellar name one the scene doesn't hold by
/// the flight's name for it ([`MapSource::stellar_name`]), as the nav
/// readout and the refusals do: a dock request and a land hint at "Hole",
/// which another system owns.
///
/// [`MapSource::stellar_name`]: crate::flight::sim_port::MapSource::stellar_name
#[test]
fn a_dock_request_and_a_land_hint_name_an_unseen_stellar_by_its_spob_name() {
    let mut names = naming();
    names.map_mut().stellar_names = vec![(StellarId(150), "Hole".to_owned())];
    let (_, mut chance) = scripted(&[0, 0]);
    let request = SimMessage::DockRequested {
        stellar: StellarId(150),
        kind: DockKind::Planet,
    };
    let said = message_text(&request, &names, None, &mut chance, RuleSource::Engine)
        .expect("a dock request is worded");
    assert_eq!(
        said,
        request_line(
            DockKind::Planet,
            "Hole",
            "Kestrel",
            RequestPicks {
                opening: 0,
                named: 0
            }
        )
    );
    let hint = SimMessage::Tutorial(nova_sim::TutorialHint::Land {
        stellar: StellarId(150),
        station: false,
    });
    assert_eq!(
        message_text(&hint, &names, None, &mut chance, RuleSource::Engine),
        Some(land_hint(false, "Hole", "L"))
    );
}

// The view's own wording: its held millions reading and its chance.

/// A view over a flight in Sol by `millions`'s reading of
/// [`RuleKey::PayMillions`], the flight naming the player's ship Kestrel
/// and stellar 128 Earth.
fn worded_by(millions: RuleSource) -> Faked {
    let rules = Rulebook::default().with_override(RuleKey::PayMillions, millions);
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol());
    fake.names_mut().player_ship_name = Some("Kestrel".to_owned());
    fake.map_mut().stellar_names = vec![(StellarId(128), "Earth".to_owned())];
    FlightView::scripted(catalog(), fake, &rules)
}

#[test]
fn a_stolen_cargos_line_says_its_numbers_by_the_views_millions_reading() {
    let stolen = SimMessage::CargoStolen {
        tons: 2,
        credits: 1_234_567,
    };
    for (millions, said) in [
        (
            RuleSource::Engine,
            "2 tons of cargo and 1.23M credits stolen!",
        ),
        (
            RuleSource::Bible,
            "2 tons of cargo and 1,234,567 credits stolen!",
        ),
    ] {
        let mut view = worded_by(millions);
        view.sim.queue_event(FlightEvent::Message(stolen.clone()));
        view.say_messages();
        assert_eq!(view.message(), Some(said), "{millions:?}");
    }
}

#[test]
fn an_approach_greetings_fee_is_said_by_the_views_millions_reading() {
    let cleared = SimMessage::DockCleared {
        stellar: StellarId(128),
        kind: DockKind::Planet,
        fee: Some(2_500_000),
    };
    for (millions, fee) in [
        (RuleSource::Engine, "[Landing fee is 2.50M credits.]"),
        (RuleSource::Bible, "[Landing fee is 2,500,000 credits.]"),
    ] {
        let mut view = worded_by(millions);
        let said = view.word(vec![cleared.clone()]);
        let expected = approach_greeting(
            DockKind::Planet,
            "Earth",
            "Kestrel",
            GreetingPicks {
                opening: 0,
                welcome: 0,
            },
            Some(2_500_000),
            millions,
        );
        assert_eq!(said, [(expected.clone(), cleared.ticks())], "{millions:?}");
        assert!(expected.ends_with(fee), "{expected}");
    }
}

#[test]
fn the_wordings_picks_draw_on_the_views_chance() {
    let mut view = worded_by(RuleSource::Engine);
    let (script, chance) = scripted(&[2, 1]);
    view.words_chance = chance;
    let request = SimMessage::DockRequested {
        stellar: StellarId(128),
        kind: DockKind::Planet,
    };
    let said = view.word(vec![request]);
    assert_eq!(
        said[0].0,
        request_line(
            DockKind::Planet,
            "Earth",
            "Kestrel",
            RequestPicks {
                opening: 2,
                named: 1
            }
        )
    );
    assert_eq!(script.borrow().asked, [3, 2], "Rand(3), then Rand(2)");
}
