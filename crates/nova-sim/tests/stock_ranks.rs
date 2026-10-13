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

mod common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::rank::{self, Held, LOST_TO_CRIME, LOST_TO_VIOLENCE};
use nova_sim::{
    Crime, Governments, GovtId, NeverFires, PilotCatalog, RankId, RuleSource, Session, SetExpr,
    SetOp,
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
