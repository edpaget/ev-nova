//! The distress calls and pre-scan hails over the stock data
//! (`_AICallForHelp` @0x82ffb-0x83257, `_InterceptorAI` @0x8ce4b-0x8cff5;
//! see the session's `comm_calls` module). Skips, passing, when
//! `NOVA_DATA` is unset.
//!
//! `_LoadStrings` fills the twenty distress lines from `STR#` 5003
//! (@0x71bf3-0x71c27); the pre-scan hail picks among `STR#` 2002
//! #381-383 (@0x8cf62-0x8cf82); and a government of `Flags2` 0x0008 never
//! calls (@0x8301e-0x83041).

use crate::common;

use nova_data::GameData;
use nova_sim::{CommCatalog, Governments, GovtId};

#[test]
fn str_5003_holds_twenty_distress_lines() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let lines = CommCatalog::string_list(&data, 5003);
    assert_eq!(lines.len(), 20);
    assert!(lines.iter().all(|line| !line.is_empty()), "{lines:?}");
    assert_eq!(lines[0], "Somebody help us! We're under attack!");
}

#[test]
fn str_2002_381_to_383_are_the_pre_scan_hails_words() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let strings = CommCatalog::string_list(&data, 2002);
    assert_eq!(
        strings.get(380..383),
        Some(
            &[
                "prepare to be scanned".to_owned(),
                "prepare for a cargo inspection".to_owned(),
                "hold position for a cargo inspection".to_owned(),
            ][..]
        )
    );
}

#[test]
fn eight_stock_governments_are_quiet_and_never_call_for_help() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let govts = Governments::read(&data);
    let quiet: Vec<i16> = govts
        .ids()
        .filter(|&id| govts.flag2(Some(id), nova_sim::hail::QUIET))
        .map(|GovtId(id)| id)
        .collect();
    assert_eq!(quiet, [138, 139, 140, 148, 159, 163, 183, 184]);
}
