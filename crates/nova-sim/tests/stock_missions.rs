//! Mission offers over the stock data: a new pilot docked at the first
//! landable stellar of its starting system, its offers rolled as the
//! session settles with every roll 1, is offered missions at the mission
//! computer, each of `AvailLoc` 0 and passing every availability field,
//! and its mission computer lists them, each name cut at its ';' and each
//! name and offer text filled. Every `mïsn`'s texts fill every tag but the
//! ranks'. A stock cargo mission loads its cargo as it is accepted, and
//! landing at its destination unloads it, pays and runs its `OnSuccess`.
//! The new pilot is offered the tutorial at the spaceport. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::desc::Desc;
use nova_data::records::mission::Mission;
use nova_sim::catalog::MissionRecord;
use nova_sim::catalog::{ListedStellar, PlaceNames};
use nova_sim::control::SetOp;
use nova_sim::mission::screens::SPACEPORT;
use nova_sim::mission::{Offer, Stop, Venue, VenueOffer};
use nova_sim::story::listers_of;
use nova_sim::text_tags::{self, MissionTags, MissionValues, TagContext, TagNames};
use nova_sim::{
    Bit, CommCatalog, DateAffixes, GameDate, GovtId, MissionId, NeverFires, Pilot, PilotCatalog,
    RuleSource, Session, StellarId, SystemId, is_landable,
};

/// A new stock pilot docked at the first landable stellar of its system,
/// through a save that says so, flown and settled with every roll 1.
fn docked_new_pilot(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(site.id), "docked");
    let _ = session.settle_script(data, &mut NeverFires);
    session
}

#[test]
fn a_new_stock_pilot_is_offered_missions_that_pass_every_field() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = docked_new_pilot(&data);
    let offers = session.mission_offers(Venue::Computer);
    assert!(!offers.is_empty(), "the mission computer offers something");
    let missions = data.missions();
    for offer in offers {
        let record = missions
            .iter()
            .find(|record| record.id == offer.mission)
            .expect("a mïsn");
        assert_eq!(record.avail_loc, 0, "{:?} at the computer", offer.mission);
        assert_eq!(
            session.offer_refusals(offer.mission),
            Some(Vec::new()),
            "{:?} passes every field",
            offer.mission
        );
    }
}

#[test]
fn a_new_stock_pilot_is_offered_the_tutorial_at_the_spaceport() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_new_pilot(&data);
    let docked = session.landed().expect("docked");
    let site = data
        .landing_sites(session.system())
        .into_iter()
        .find(|site| site.id == docked)
        .expect("the docked stellar");
    assert_eq!(
        site.govt,
        Some(GovtId(128)),
        "AvailStel 10000 names gövt 128"
    );
    let Some(VenueOffer::Prompt(prompt)) = session.next_offer(SPACEPORT, &mut NeverFires) else {
        panic!("the spaceport offers");
    };
    assert_eq!(prompt.mission, MissionId(251), "Head to Sol;Tutorial 001");
    assert!(prompt.can_refuse());
    assert!(
        prompt
            .text
            .starts_with("As you wander around a spaceport for the first time"),
        "{}",
        prompt.text
    );
}

/// The stock names: the stellars' and systems', and `STR#` 4001's cargo.
struct StockNames {
    places: PlaceNames,
    cargo: Vec<String>,
    /// Every listed stellar, for the system a tag names each in.
    listed: Vec<ListedStellar>,
}

impl TagNames for StockNames {
    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        self.places.stellars.get(&stellar).cloned()
    }

    fn system_name(&self, system: SystemId) -> Option<String> {
        self.places.systems.get(&system).cloned()
    }

    /// The lowest system listing it: `_FindSystemFromStellar`'s answer
    /// with no system visible, enough for a tag to fill.
    fn stellar_system(&self, stellar: StellarId) -> Option<SystemId> {
        listers_of(&self.listed, stellar).first().copied()
    }

    fn cargo_name(&self, cargo: u8) -> Option<String> {
        self.cargo.get(usize::from(cargo)).cloned()
    }
}

/// Every tag of `_MungeBriefing`'s table but the ranks'.
const TAGS: [&str; 15] = [
    "<DST>", "<DSY>", "<RST>", "<RSY>", "<CT>", "<CQ>", "<SN>", "<DL>", "<PN>", "<PNN>", "<PSN>",
    "<PST>", "<OSN>", "<PAY>", "<REG>",
];

#[test]
fn every_stock_missions_texts_fill_every_tag_but_the_ranks() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let names = StockNames {
        places: data.place_names(),
        cargo: CommCatalog::string_list(&data, 4001),
        listed: data.listed_stellars(),
    };
    let listed = data.listed_stellars();
    let stop = |at: usize| Stop {
        stellar: listed[at].id,
        system: listed[at].systems.first().copied(),
    };
    let today = GameDate::new(1177, 6, 23).expect("exists");
    let affixes = DateAffixes {
        prefix: String::new(),
        suffix: " NC".to_owned(),
    };
    let context = |cargo: i16| TagContext {
        pilot: "Stock",
        ship_type: "Shuttle",
        ship_name: "Kestrel",
        person: Some("Bounty Hunter"),
        ranks: nova_sim::rank::RankNames::none(),
        mission: Some(MissionTags {
            values: MissionValues {
                destination: Some(stop(0)),
                return_to: Some(stop(1)),
                cargo_type: Some(cargo),
                cargo_tons: 10,
                pay: Some(25_000),
                deadline: Some(today.next_day()),
                ship_name: Some("Rover".to_owned()),
            },
            cash: 1000,
            today,
            date_affixes: &affixes,
            pay_millions: RuleSource::Engine,
        }),
    };
    let mut texts = 0;
    let mut tagged = 0;
    for (id, mission) in data.records::<Mission>() {
        let record = mission.expect("every stock mïsn decodes").record;
        let fields = [
            record.brief_text,
            record.quick_brief,
            record.load_carg_text,
            record.dump_cargo_text,
            record.comp_text,
            record.fail_text,
            record.ship_done_text,
            record.refuse_text,
        ];
        let offer = 4000 + (id - 128);
        let descs = fields
            .into_iter()
            .flatten()
            .map(|desc| desc.0)
            .chain([offer]);
        for desc in descs {
            let Some(Ok(text)) = data.get::<Desc>(desc) else {
                continue;
            };
            let text = text.record.text.as_str().to_owned();
            texts += 1;
            if TAGS.iter().any(|tag| text.contains(tag)) {
                tagged += 1;
            }
            let filled = text_tags::expand(&text, &context(0), &names);
            for tag in TAGS {
                assert!(!filled.contains(tag), "mïsn {id}, dësc {desc}: {tag} left");
            }
            assert!(!filled.contains(text_tags::UNSET), "mïsn {id}, dësc {desc}");
        }
    }
    assert!(texts > 1000, "{texts} texts read");
    assert!(tagged > 100, "{tagged} texts with tags");
    assert_eq!(
        text_tags::expand("<DST>", &context(0), &names),
        names.places.stellars[&listed[0].id],
        "the stock stellar's name"
    );
    assert_eq!(names.cargo[6], "*passengers");
    assert_eq!(text_tags::expand("<CT>", &context(6), &names), "passengers");
}

#[test]
fn a_new_stock_pilots_mission_computer_lists_its_offers_named_and_filled() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let session = docked_new_pilot(&data);
    let computer = session.mission_computer().expect("the computer opens");
    assert_eq!(
        computer.rows.len(),
        session.mission_offers(Venue::Computer).len()
    );
    assert!(!computer.rows.is_empty());
    for row in &computer.rows {
        assert!(!row.name.is_empty(), "{:?} is named", row.mission);
        assert!(!row.name.contains(';'), "{:?}: {}", row.mission, row.name);
        for tag in TAGS {
            assert!(!row.name.contains(tag), "{:?}: {}", row.mission, row.name);
            assert!(!row.text.contains(tag), "{:?}: {}", row.mission, row.text);
        }
    }
    assert!(
        computer.rows.iter().any(|row| !row.text.is_empty()),
        "the offers have texts"
    );
}

/// The first bit `record`'s `OnSuccess` sets that nothing after it in the
/// script clears or toggles.
fn bit_set_on_success(record: &MissionRecord) -> Option<Bit> {
    let ops = &record.on_success.tree()?.ops;
    ops.iter().enumerate().find_map(|(at, op)| {
        let SetOp::Set(bit) = op else {
            return None;
        };
        let undone = ops[at + 1..].iter().any(
            |later| matches!(later, SetOp::Clear(other) | SetOp::Toggle(other) if other == bit),
        );
        (!undone).then_some(*bit)
    })
}

/// Whether `offer` of `record` carries cargo loaded as it is accepted
/// (`PickupMode` 0) and unloaded at its destination or its return, which
/// is the destination (`ReturnStel` -1), paying credits, with no ship.
fn carries_cargo_there_and_back(offer: &Offer, record: &MissionRecord) -> bool {
    offer.cargo_type.is_some_and(|cargo| cargo >= 0)
        && offer.cargo_tons > 0
        && offer.pay > 0
        && offer.destination.is_some()
        && offer.return_to == offer.destination
        && record.return_stel == -1
        && record.pickup_mode == 0
        && matches!(record.drop_off_mode, 0 | 1)
        && record.ships.count <= 0
}

/// `date`, `days` days on.
fn days_after(date: GameDate, days: i16) -> GameDate {
    (0..days.max(0)).fold(date, |date, _| date.next_day())
}

#[test]
fn a_stock_cargo_mission_loads_on_accept_and_completes_at_its_destination() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_new_pilot(&data);
    let missions = data.missions();
    let record_of = |offer: &Offer| {
        missions
            .iter()
            .find(|record| record.id == offer.mission)
            .expect("a mïsn")
    };
    // The first such offer whose `OnSuccess` sets a bit, to see it run.
    // None of the stock missions whose `OnSuccess` sets one is offered to
    // a new pilot (each waits on the story's bits), so otherwise the
    // first such offer: its `OnSuccess` is empty, and the success that
    // runs it is seen by its pay and its `DatePostInc` days.
    let offers = session.mission_offers(Venue::Computer);
    let qualifies = |offer: &&Offer| carries_cargo_there_and_back(offer, record_of(offer));
    let offer = offers
        .iter()
        .filter(qualifies)
        .find(|offer| bit_set_on_success(record_of(offer)).is_some())
        .or_else(|| offers.iter().find(qualifies))
        .cloned()
        .expect("a stock cargo mission to and from one stellar");
    let record = record_of(&offer);
    let bit = bit_set_on_success(record);
    let slot = session
        .accept_mission(offer.mission, &mut NeverFires)
        .expect("accepted");
    let active = session.pilot().missions().get(slot).expect("held");
    assert!(active.aboard, "{:?} loaded at accept", offer.mission);
    assert_eq!(session.pilot().total_cargo(), offer.cargo_tons);
    let cash = session.pilot().cash();
    let today = session.date();

    // Docked at the destination, before the deadline.
    let destination = offer.destination.expect("a destination");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(session.pilot())).expect("JSON");
    save["stellar"] = serde_json::json!(destination.stellar.0);
    save["system"] = serde_json::json!(destination.system.expect("a system").0);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(&data, there).expect("flies");
    assert_eq!(session.landed(), Some(destination.stellar), "docked there");
    let _ = session.settle_script(&data, &mut NeverFires);

    assert!(
        !session.pilot().missions().holds(slot, offer.mission),
        "{:?} completed",
        offer.mission
    );
    assert_eq!(session.pilot().total_cargo(), 0, "unloaded");
    assert_eq!(session.pilot().cash(), cash + offer.pay, "paid");
    assert_eq!(
        session.date(),
        days_after(today, record.date_post_inc),
        "its DatePostInc days, after OnSuccess"
    );
    if let Some(bit) = bit {
        assert!(session.control_bit(bit), "OnSuccess set {bit:?}");
    }
}
