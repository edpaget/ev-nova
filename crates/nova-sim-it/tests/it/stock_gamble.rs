//! The bar's Gamble over the stock data. Skips, passing, when `NOVA_DATA`
//! is unset.
//!
//! - `STR#` 2002 holds the texts the race shows: #361 with no cash, #370
//!   the winnings line's label and #372 the asked bet's prompt; `STR#`
//!   150 #59 titles the asked bet's OK.
//! - A race at Merrol's bar (`spöb` 171 in Aldebaran, `sÿst` 141; `Flags`
//!   0x49) takes its bet and names a result `dësc` (32760 + the winner)
//!   that the stock data holds with text, `Graphic` 0 (no picture: only
//!   one above 127 picks the picture variant) and "Race N.mov" played
//!   before it; the Help's `dësc`, 32764, is there too.

use crate::common;

use nova_data::GameData;
use nova_data::records::desc::Desc;
use nova_sim::gamble::{
    AMOUNT_TO_BET, BET, Bet, FIRST_RACE_DESC, NO_CASH_TO_BET, RACE_HELP_DESC, YOUR_WINNINGS,
};
use nova_sim::{Chance, CommCatalog, DescMovie, Session, StellarId, SystemId};

/// Never fires; every draw and roll is the outcome it is set to.
struct Rolls(u32);

impl Chance for Rolls {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.min(n - 1)
    }
}

#[test]
fn str_2002_and_150_hold_the_races_words() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let texts = data.string_list(2002);
    assert_eq!(texts[360], NO_CASH_TO_BET, "#361");
    assert_eq!(texts[369], YOUR_WINNINGS, "#370");
    assert_eq!(texts[371], AMOUNT_TO_BET, "#372");
    assert_eq!(data.string_list(150)[58], BET, "#59");
}

#[test]
fn a_race_at_a_stock_bar_names_a_result_desc_the_stock_data_holds() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for winner in 0..4 {
        let mut session = Session::start(&data).expect("the stock first chär starts");
        session
            .land_at(&data, SystemId(141), StellarId(171))
            .expect("Merrol is landable");
        assert!(session.open_bar(&mut Rolls(0)), "Merrol has a bar");
        let cash = session.pilot().cash;
        let run = session
            .race(0, Bet::House, &mut Rolls(winner))
            .expect("a race");
        assert_eq!(session.pilot().cash, cash - 1000, "the house bet taken");
        assert_eq!(
            run.desc(),
            FIRST_RACE_DESC + i16::try_from(winner).expect("small")
        );
        let desc = data
            .get::<Desc>(run.desc())
            .expect("the result's dësc is in the stock data")
            .expect("decodes")
            .record
            .clone();
        assert!(!desc.text.as_str().is_empty(), "dësc {}", run.desc());
        assert_eq!(
            desc.graphic.map(|pict| pict.0),
            Some(0),
            "dësc {}: Graphic 0, so the plain text dialog",
            run.desc()
        );
        assert_eq!(
            DescMovie::from_desc(desc.movie_file.as_str(), desc.flags.0),
            DescMovie::from_desc(&format!("Race {}.mov", winner + 1), 0),
            "dësc {}: its movie before it",
            run.desc()
        );
    }
    let help = data
        .get::<Desc>(RACE_HELP_DESC)
        .expect("the Help's dësc is in the stock data")
        .expect("decodes");
    assert!(!help.record.text.as_str().is_empty());
}
