//! The stock NPCs' skill, government factor and bare class (see
//! `nova_sim::skill` and `nova_sim::stats::ShipStats::for_npc`). Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - The Arachnid (158) carries a Fusion Reactor (177, +20 `Speed`, +25
//!   `Accel`) by default: by the engine an NPC Arachnid flies at its bare
//!   `Speed` 300, with it by the other reading.
//! - The Pirate Starbridge (148) has `SkillVar` 30: its skill rolls from
//!   0.70 to 1.30.
//! - The Duellists (gövt 175) have `SkillMult` 200 and the Spanner (171)
//!   1: their ships fly at twice and a hundredth of their speed. Spanner's
//!   persons, 144 "Nuada's ex-boss" and 335 "John Cook", fly so.
//! - The second-hand Starbridge (368) carries two Battery Packs (256,
//!   +100 fuel each) and the second-hand Pegasus (366) three Carbon Fibers
//!   (180, +40 armour each): by the engine an NPC holds its class's `Fuel`
//!   and `Armor` alone.
// The stats are exact products of the records' fields.
#![allow(clippy::float_cmp)]

use crate::common;

use std::collections::BTreeMap;

use nova_data::GameData;
use nova_sim::combat::armament::Arsenal;
use nova_sim::flight::ShipState;
use nova_sim::govt::Governments;
use nova_sim::skill::random_skill_level;
use nova_sim::traffic::Traffic;
use nova_sim::traffic::npc::AiType;
use nova_sim::traffic::spawn::{Entry, NewShip};
use nova_sim::traffic::table::{ShipKind, SpawnTable, kind};
use nova_sim::{GovtId, NeverFires, PersonId, PilotCatalog, RuleSource, ShipId, TrafficCatalog};

fn opened() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// Stock class `ship` as an NPC flies it, its default items counted as
/// `items` says.
fn stock_kind(data: &GameData, ship: i16, items: RuleSource) -> ShipKind {
    let record = data
        .ships()
        .into_iter()
        .find(|record| record.id == ShipId(ship))
        .expect("the ship type reads");
    kind(&record, &data.outfits(), &Arsenal::read(data), items)
}

#[test]
fn an_npc_arachnid_flies_at_its_bare_speed_by_the_engine_and_with_its_reactor_by_the_other() {
    let Some(data) = opened() else {
        return;
    };
    let engine = stock_kind(&data, 158, RuleSource::Engine).stats.handling;
    let bible = stock_kind(&data, 158, RuleSource::Bible).stats.handling;
    assert_eq!(engine.max_speed, 3.0);
    assert_eq!(engine.accel, 450.0 / 5000.0);
    assert_eq!(bible.max_speed, 3.2);
    assert_eq!(bible.accel, 475.0 / 5000.0);
}

#[test]
fn a_skill_var_30_class_rolls_from_0_70_to_1_30() {
    let Some(data) = opened() else {
        return;
    };
    let starbridge = stock_kind(&data, 148, RuleSource::Engine);
    assert_eq!(starbridge.skill_var, 30);
    let rolled = |draw| {
        let mut chance = nova_sim_base::fixture::Draws::of(&[draw]);
        let skill = random_skill_level(starbridge.skill_var, &mut chance);
        assert_eq!(chance.asked, [61]);
        skill
    };
    assert_eq!(rolled(0), 0.7);
    assert_eq!(rolled(30), 1.0);
    assert_eq!(rolled(60), 1.3);
}

/// A ship of stock class `ship` of `govt`, brought into a system, as its
/// own handling flies it.
fn brought_in(data: &GameData, ship: i16, govt: i16) -> (f32, f32) {
    let govts = Governments::read(data);
    let mut traffic = Traffic::new();
    traffic.enter(
        SpawnTable {
            ships: BTreeMap::from([(ShipId(ship), stock_kind(data, ship, RuleSource::Engine))]),
            ..SpawnTable::default()
        },
        &mut NeverFires,
    );
    let came = traffic.bring_in_within(
        vec![NewShip {
            ship: ShipId(ship),
            govt: Some(GovtId(govt)),
            ai_type: AiType::WimpyTrader,
            lead: None,
            state: ShipState::default(),
            entry: Entry::Placed,
            aggression: 0,
            booty: None,
            info_types: 0,
            person: None,
            mission: None,
            aux: None,
            derelict: false,
            held: false,
            boarded: false,
            cloaked: false,
            quote: None,
            cargo: None,
        }],
        8,
        &govts,
        &mut NeverFires,
    );
    assert_eq!(came, 1);
    let npc = &traffic.npcs()[0];
    assert_eq!(npc.skill, 1.0, "the Arachnid's SkillVar 0");
    let own = npc.own_handling();
    (own.max_speed, own.accel)
}

#[test]
fn the_duellists_fly_at_twice_their_speed_and_the_spanner_at_a_hundredth() {
    let Some(data) = opened() else {
        return;
    };
    let govts = Governments::read(&data);
    assert_eq!(govts.skill_mult(Some(GovtId(175))), 2.0);
    assert_eq!(govts.skill_mult(Some(GovtId(171))), (0.01_f64) as f32);
    assert_eq!(govts.skill_mult(Some(GovtId(160))), 1.0, "SkillMult 0");
    let bare = stock_kind(&data, 158, RuleSource::Engine).stats.handling;
    assert_eq!(
        brought_in(&data, 158, 175),
        (bare.max_speed * 2.0, bare.accel * 2.0)
    );
    let spanner = (0.01_f64) as f32;
    assert_eq!(
        brought_in(&data, 158, 171),
        (bare.max_speed * spanner, bare.accel * spanner)
    );
    let persons = data.persons();
    for id in [144, 335] {
        let person = persons
            .iter()
            .find(|person| person.id == PersonId(id))
            .expect("the person reads");
        assert_eq!(person.govt, Some(GovtId(171)), "{id}");
    }
}

#[test]
fn an_npcs_fuel_and_armour_are_its_classs_alone_by_the_engine() {
    let Some(data) = opened() else {
        return;
    };
    let starbridge = |items| stock_kind(&data, 368, items).stats.fuel;
    assert_eq!(starbridge(RuleSource::Engine), 500.0);
    assert_eq!(starbridge(RuleSource::Bible), 700.0);
    let pegasus = |items| stock_kind(&data, 366, items).stats.armor;
    assert_eq!(pegasus(RuleSource::Engine), 115.0);
    assert_eq!(pegasus(RuleSource::Bible), 235.0);
}
