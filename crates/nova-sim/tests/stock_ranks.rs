//! The ranks over the stock data: a stock rank granted by its mission's
//! `OnSuccess` shows in the pilot info's ranks line, names the pilot in
//! the text tags and pays its salary. Skips, passing, when `NOVA_DATA` is
//! unset.
//!
//! Rank 128, "Federation Naval Rank of Commander;Fed 1" (`Weight` 1,
//! `Salary` 200, no cap, `ConvName` and `ShortName` "Commander"), is
//! granted by a stock mïsn whose `OnSuccess` holds `K128`.

mod common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::{NeverFires, PilotCatalog, RankId, Session, SetExpr, SetOp};

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
