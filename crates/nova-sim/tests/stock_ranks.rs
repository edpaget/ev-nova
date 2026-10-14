//! The ranks over the stock data: a stock rank granted by its mission's
//! `OnSuccess` shows in the pilot info's ranks line, names the pilot in
//! the text tags and pays its salary. Skips, passing, when `NOVA_DATA` is
//! unset.
//!
//! Rank 128, "Federation Naval Rank of Commander;Fed 1" (`Weight` 1,
//! `Salary` 200, no cap, `ConvName` and `ShortName` "Commander"), is
//! granted by a stock mïsn whose `OnSuccess` holds `K128`.
//!
//! The protection ranks, rank 148 "; Rebel 1" (govt 141, `Flags` 0x0144)
//! and 153-158, the "Duel protector" ranks (govts 129-135, 0x0140), are
//! lost to a crime against their government.
//!
//! Rank 147, "Have Access to Hypergate System" (govt 183, `Flags`
//! 0x0208), blesses landing at the 19 hypergates (`spöb` 1400-1418, govt
//! 183, `MinStatus` 32767), which no record clears.

mod common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::news;
use nova_sim::player_info::Words;
use nova_sim::rank::{self, Blessed, Held, LOST_TO_CRIME, LOST_TO_VIOLENCE};
use nova_sim::{
    CommCatalog, Crime, Governments, GovtId, NeverFires, PilotCatalog, RankId, RuleSource, Session,
    SetExpr, SetOp, Standing, StellarId, cleared,
};

#[test]
fn a_stock_rank_granted_by_its_mission_shows_in_the_pilot_info_and_pays() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let granting = data
        .missions()
        .into_iter()
        .find(|mission| {
            mission
                .on_success
                .tree()
                .is_some_and(|expr| expr.ops.contains(&SetOp::ActivateRank(RankId(128))))
        })
        .expect("a stock mïsn grants rank 128");
    let start = || {
        Session::start(&*data)
            .expect("the stock first chär starts")
            .with_strings(data.clone())
    };
    let (mut ranked, mut control) = (start(), start());
    assert_eq!(ranked.pilot_info_ranks().line, "", "a new pilot has none");
    ranked.run_script(&granting.on_success, &mut NeverFires);
    assert!(ranked.pilot().ranks().contains(&RankId(128)));
    let info = ranked.pilot_info_ranks();
    assert_eq!(info.line, "Federation Naval Rank of Commander");
    assert_eq!(info.salary, 200);
    assert_eq!(ranked.plain_text("<PRK>"), "Commander");
    assert_eq!(control.plain_text("<PRK>"), "captain", "STR# 2002 #341");
    // The control runs the same script, then `L128`: no stock rank
    // sweeps, so only the rank tells the two apart.
    control.run_script(&granting.on_success, &mut NeverFires);
    control.run_set(&SetExpr::parse("L128").expect("parses"), &mut NeverFires);
    assert!(!control.pilot().ranks().contains(&RankId(128)));
    assert_eq!(ranked.pilot().cash(), control.pilot().cash());
    ranked.let_days_pass(1, &mut NeverFires);
    control.let_days_pass(1, &mut NeverFires);
    assert_eq!(
        ranked.pilot().cash(),
        control.pilot().cash() + 200,
        "a day's salary"
    );
}

#[test]
fn a_stock_rank_granted_by_a_set_expression_shows_on_the_player_infos_honors() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let mut session = Session::start(&*data)
        .expect("the stock first chär starts")
        .with_strings(data.clone());
    let info = session.player_info(&*data);
    assert_eq!(info.honors, None, "a new pilot has none");
    assert_eq!(
        info.legal_status, "No Record",
        "no record: STR# 134 #2 (_legalStatusStrings[1])"
    );
    session.run_set(&SetExpr::parse("K128").expect("parses"), &mut NeverFires);
    assert_eq!(
        session.player_info(&*data).honors.as_deref(),
        Some("Your ranks and honors:\r\rFederation Naval Rank of Commander\r")
    );
}

#[test]
fn the_player_infos_stock_words_are_str_2002s() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    assert_eq!(
        Words::from_list(&data.string_list(news::MISC_STRINGS)),
        Words::STOCK
    );
}

/// Whether a crime against a ship of `victim` takes rank `id` held alone,
/// by the engine.
fn takes(data: &GameData, id: i16, crime: Crime, victim: Option<GovtId>) -> bool {
    let records = PilotCatalog::ranks(data);
    let mut active = std::collections::BTreeSet::from([RankId(id)]);
    let mut recent = None;
    rank::lose_to_crime(
        &records,
        Held {
            active: &mut active,
            recent: &mut recent,
        },
        crime,
        victim,
        &Governments::read(data),
        RuleSource::Engine,
        RuleSource::Engine,
    );
    active.is_empty()
}

#[test]
fn stock_protection_ranks_are_lost_to_crimes_against_their_government() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let records = PilotCatalog::ranks(&data);
    for id in [148, 153, 154, 155, 156, 157, 158] {
        let record = records
            .iter()
            .find(|record| record.id == RankId(id))
            .expect("a stock rank");
        assert_ne!(record.flags & LOST_TO_CRIME, 0, "{id}");
        let govt = record.affil_govt;
        assert!(govt.is_some(), "{id}: affiliated");
        assert!(!takes(&data, id, Crime::Shoot, govt), "{id}: a hit");
        assert!(takes(&data, id, Crime::Board, govt), "{id}: a boarding");
        assert!(
            takes(&data, id, Crime::Smuggle, govt),
            "{id}: contraband its government's scan finds"
        );
    }
    let rebel = records
        .iter()
        .find(|record| record.id == RankId(148))
        .expect("rank 148");
    assert_eq!(rebel.affil_govt, Some(GovtId(141)));
    assert_ne!(rebel.flags & LOST_TO_VIOLENCE, 0);
    assert!(
        !Governments::read(&data).allies(Some(GovtId(128)), Some(GovtId(141))),
        "the Federation is no Rebel ally"
    );
    assert!(
        !takes(&data, 148, Crime::Kill, Some(GovtId(128))),
        "a Federation ship's killing"
    );
}

#[test]
fn the_hypergate_access_rank_clears_every_hypergate() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let records = PilotCatalog::ranks(&data);
    let govts = Governments::read(&data);
    let blessing = |id: i16| {
        rank::blessing(
            &records,
            &std::collections::BTreeSet::from([RankId(id)]),
            &govts,
            RuleSource::Engine,
        )
    };
    let hypergate_access = blessing(147);
    let hypergate_govt = Some(GovtId(183));
    assert!(hypergate_access.blesses(hypergate_govt, Blessed::Landing));
    assert!(!hypergate_access.blesses(hypergate_govt, Blessed::Peace));
    let mut gates: Vec<_> = data
        .star_map()
        .into_iter()
        .flat_map(|system| data.landing_sites(system.id))
        .filter(|site| site.govt == hypergate_govt)
        .collect();
    gates.sort_by_key(|site| site.id);
    gates.dedup_by_key(|site| site.id);
    assert_eq!(
        gates.iter().map(|site| site.id).collect::<Vec<_>>(),
        (1400..=1418).map(StellarId).collect::<Vec<_>>()
    );
    for gate in &gates {
        assert_eq!(gate.min_status, 32767, "{:?}", gate.id);
        let standing = |blessed| Standing {
            record: 32767,
            meets_require: true,
            mission_stellar: false,
            blessed,
        };
        for source in RuleSource::ALL {
            assert!(
                !cleared(gate, standing(false), source),
                "{:?} {source:?}",
                gate.id
            );
            assert!(
                cleared(gate, standing(true), source),
                "{:?} {source:?}",
                gate.id
            );
        }
    }
    let commander = blessing(128);
    for which in [Blessed::Peace, Blessed::Landing] {
        assert!(commander.blesses(Some(GovtId(128)), which), "{which:?}");
    }
}
