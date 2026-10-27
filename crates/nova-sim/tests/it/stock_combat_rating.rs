//! The combat rating's words over the stock data: the player info's
//! Combat Rating row and the main screen's pilot panel both read them
//! from `STR#` 138 by the rating (`_DrawCombatRatingString` @0x57e5), so
//! they agree, and a new pilot's are `STR#` 138 #1. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::rating::RATING_STRINGS;
use nova_sim::{CommCatalog, Session};

#[test]
fn the_player_infos_combat_rating_is_the_pilot_panels() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let session = Session::start(&*data)
        .expect("the stock first chär starts")
        .with_strings(data.clone());
    assert_eq!(session.pilot().combat_rating(), 0, "a new pilot");
    let shown = session.player_info(&*data).combat_rating;
    assert_eq!(shown, session.pilot_panel(&*data).combat_rating);
    assert!(!shown.is_empty());
    assert_eq!(Some(&shown), data.string_list(RATING_STRINGS).first());
}
