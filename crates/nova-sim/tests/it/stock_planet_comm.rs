//! The planet comm dialog over the stock data (play-feel-4 phase 2,
//! `_DoPlanetCommDialog` @0x96949): its strings where it reads them,
//! every stock `OnDominate` and `OnRelease` parsing, and Earth's demand
//! fought, sending up its wave of 6. Skips, passing, when `NOVA_DATA` is
//! unset.

use crate::common;

use nova_data::GameData;
use nova_sim::flight_frame::{Answer, FlightCommand};
use nova_sim::pilot::Pilot;
use nova_sim::planet_comm::{
    DEMAND_TRIBUTE, DOMINATED_STATUS, FORBIDDEN, GREETINGS, HOSTILE, OFFER_BRIBE, OWNED,
    PlanetCommItem, PlanetReply, RELEASE, STATUS_LABEL, type_label,
};
use nova_sim::{
    ApplyCollaborators, Chance, CommCatalog, HailOptions, Hailed, NovaBoarding, NovaLaw,
    PilotCatalog, Session, StellarId, StellarPick, SystemId,
};

/// The system that lists `stellar`.
fn system_of(data: &GameData, stellar: StellarId) -> SystemId {
    data.star_map()
        .into_iter()
        .find(|system| {
            data.landing_sites(system.id)
                .iter()
                .any(|site| site.id == stellar)
        })
        .expect("a system lists it")
        .id
}

#[test]
fn the_planet_comm_dialogs_strings_are_where_it_reads_them() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let earth = |reply: PlanetReply| reply.say(0, "Earth", &data);
    assert_eq!(
        earth(PlanetReply::ChannelOpen),
        "Communications channel open to Earth."
    );
    assert_eq!(earth(PlanetReply::WhatDoYouWant), "What is it you want?");
    assert_eq!(earth(PlanetReply::NoResponse), "No response.");
    assert_eq!(earth(PlanetReply::Greetings), "Nice to meet you.");
    assert_eq!(earth(PlanetReply::Laugh), "Don't make us laugh.");
    assert_eq!(
        earth(PlanetReply::Fight),
        "It's you who should surrender, once you see our mighty ships coming at you!"
    );
    assert_eq!(
        earth(PlanetReply::Dominated { station: false }),
        "All right, all right already!  The planet agrees to pay you tribute."
    );
    assert_eq!(
        earth(PlanetReply::Released { station: true }),
        "You release the station from its debt to you."
    );
    assert_eq!(
        earth(PlanetReply::BribeAsked),
        "We'll let you slip by the security barrier if you pay us."
    );
    assert_eq!(earth(PlanetReply::BribeDeclined), "Yeah, you wish.");
    assert_eq!(earth(PlanetReply::NoBribe), "Yeah, right.");
    assert_eq!(earth(PlanetReply::WastingTime), "Stop wasting our time.");
    assert_eq!(type_label(0, &data), "[Earth/Luna System]");
    let buttons = CommCatalog::string_list(&data, 150);
    for (n, text) in [
        (22, GREETINGS),
        (24, OFFER_BRIBE),
        (32, RELEASE),
        (45, DEMAND_TRIBUTE),
    ] {
        assert_eq!(buttons[n - 1], text, "STR# 150 #{n}");
    }
    let messages = CommCatalog::string_list(&data, 2002);
    for (n, text) in [
        (171, OWNED),
        (172, DOMINATED_STATUS),
        (173, FORBIDDEN),
        (174, HOSTILE),
        (196, STATUS_LABEL),
    ] {
        assert_eq!(messages[n - 1], text, "STR# 2002 #{n}");
    }
}

#[test]
fn every_stock_on_dominate_and_on_release_parses() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let scripts = data.stellar_scripts();
    assert!(!scripts.is_empty(), "every spöb read");
    let written: Vec<_> = scripts
        .iter()
        .flat_map(|script| [&script.on_dominate, &script.on_release])
        .filter(|script| **script != nova_sim::control::Script::default())
        .collect();
    assert!(
        written.iter().all(|script| script.tree().is_some()),
        "each parses"
    );
}

/// Every draw its last outcome, no percentage firing.
struct Last;

impl Chance for Last {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

/// Runs `command` on `session` over `data`, by Nova's law.
fn apply(session: &mut Session, data: &GameData, command: FlightCommand) -> Answer {
    session.apply(
        command,
        &mut ApplyCollaborators {
            catalog: Some(data),
            chance: &mut Last,
            law: &NovaLaw::default(),
            boarding_rule: &NovaBoarding::default(),
            hail_options: &HailOptions::default(),
        },
    )
}

#[test]
fn earths_demand_fought_sends_up_its_wave_of_six() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let earth = StellarId(128);
    let sol = system_of(&data, earth);
    let pilot = Pilot::new(&data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(earth.0);
    save["system"] = serde_json::json!(sol.0);
    save["combat_rating"] = serde_json::json!(20_000);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, there).expect("flies");
    // The first pass fills the system.
    crate::passes::work(&mut session, &data, nova_sim::Controls::default());
    // Earth's own `NavDefs` slot, as the slot keys read it.
    let slot = data
        .nav_def_slots()
        .into_iter()
        .find(|(system, _)| *system == sol)
        .and_then(|(_, slots)| slots.iter().position(|&slot| slot == Some(earth)))
        .expect("Sol lists Earth");
    assert_eq!(session.select_stellar(StellarPick::Slot(slot)), Some(earth));
    let answer = apply(&mut session, &data, FlightCommand::Hail { option: true });
    assert!(
        matches!(&answer, Answer::Hailed(Hailed::Stellar(view)) if view.stellar == earth),
        "{answer:?}"
    );
    let answer = apply(
        &mut session,
        &data,
        FlightCommand::PressPlanetComm(PlanetCommItem::Third),
    );
    let Answer::PlanetComm(Some(view)) = answer else {
        panic!("still open: {answer:?}");
    };
    assert_eq!(
        view.reply, "Eat vacuum, sucker - our defense fleet is on its way.",
        "STR# 3002 #(4 + 11)"
    );
    let up = session
        .npcs()
        .iter()
        .filter(|npc| npc.defends == Some(earth))
        .count();
    assert_eq!(up, 6, "7006 % 10");
    assert_eq!(
        nova_sim::fixture::defence_ships_left(&session, earth),
        Some(594)
    );
}
