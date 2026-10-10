//! Mission offers over the stock data: a new pilot docked at the first
//! landable stellar of its starting system, its offers rolled as the
//! session settles with every roll 1, is offered missions at the mission
//! computer, each of `AvailLoc` 0 and passing every availability field,
//! and its mission computer lists them, each name cut at its ';' and each
//! name and offer text filled. Every `mïsn`'s texts fill every tag but the
//! ranks'. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::records::desc::Desc;
use nova_data::records::mission::Mission;
use nova_sim::catalog::PlaceNames;
use nova_sim::mission::{Stop, Venue};
use nova_sim::text_tags::{self, MissionTags, MissionValues, TagContext, TagNames};
use nova_sim::{
    CommCatalog, DateAffixes, GameDate, NeverFires, Pilot, PilotCatalog, RuleSource, Session,
    StellarId, SystemId, is_landable,
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

/// The stock names: the stellars' and systems', and `STR#` 4001's cargo.
struct StockNames {
    places: PlaceNames,
    cargo: Vec<String>,
}

impl TagNames for StockNames {
    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        self.places.stellars.get(&stellar).cloned()
    }

    fn system_name(&self, system: SystemId) -> Option<String> {
        self.places.systems.get(&system).cloned()
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
