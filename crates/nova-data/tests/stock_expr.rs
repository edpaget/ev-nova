//! Every control-bit expression in the stock EV Nova data parses.
//!
//! The game data is copyrighted and never committed, so this test gets its
//! location from `common`, the only place `NOVA_DATA` is read, and skips,
//! passing, when it is unset.

mod common;

use nova_data::store::GameData;
use nova_data::{EXPR_FIELDS, ExprKind, check_expressions};

use common::nova_data;

#[test]
fn every_stock_expression_parses() {
    let Some(dir) = nova_data() else { return };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let errors = check_expressions(&data);
    for error in &errors {
        eprintln!("{error}");
    }
    assert!(
        errors.is_empty(),
        "{} expressions do not parse",
        errors.len()
    );

    // The check parsed something: a broken field table could otherwise pass
    // by reading nothing. The stock data has 1444 test and 2097 set
    // expressions that are not blank.
    let (mut tests, mut sets) = (0, 0);
    for field in EXPR_FIELDS {
        for &id in data.ids(field.res_type) {
            let entry = data
                .get_any(field.res_type, id)
                .expect("listed")
                .expect("the stock records decode");
            let text = (field.text)(entry.record).expect("a record of the field's type");
            if !text.is_empty() {
                match field.kind {
                    ExprKind::Test => tests += 1,
                    ExprKind::Set => sets += 1,
                }
            }
        }
    }
    assert_eq!((tests, sets), (1444, 2097));
}
