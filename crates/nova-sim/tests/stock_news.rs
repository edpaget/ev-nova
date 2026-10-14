//! The bar's news over the stock data. Skips, passing, when `NOVA_DATA`
//! is unset.
//!
//! - Every stock `crön`'s news fields read as the engine's loader reads
//!   them: stock data uses 0 for an unused slot and for no `IndNewsStr`,
//!   and those are none.
//! - Crön 231 "News: Player discovers way to free Vell-os" (`EnableOn
//!   b8906 & !b8907`, `Random` 100, no holdoff, `NewsGovt1` 130 Polaris
//!   with `STR#` 15083) running shows its story at Merrol (`spöb` 171 in
//!   Aldebaran, `sÿst` 141; `Flags` 0x49, a bar), of govt 141 Rebellion:
//!   Rebellion (class 10, allies 16) and Polaris (class 16, allies 10)
//!   are allied both ways and neither is derelict. By the other reading
//!   of `news_govt_match` only Polaris's stellars show it, and Merrol
//!   shows `STR#` 8101's generic news.
//! - `öops` 128 "An enormous food surplus" (`Stellar` 137 Port Kane,
//!   commodity 0, `PriceDelta` -15) tells its story from `STR#` 2002's
//!   pieces and `STR#` 4001's "food".

mod common;

use std::rc::Rc;

use nova_data::GameData;
use nova_data::records::cron::Cron;
use nova_sim::catalog::ListedStellar;
use nova_sim::news::{self, CARGO_NAMES, COMMERCIALS, GENERIC_NEWS, StoryWords};
use nova_sim::story::listers_of;
use nova_sim::{
    Bit, Chance, CommCatalog, CronId, DisasterId, GameDate, GovtId, PilotCatalog, PlaceNames,
    RuleKey, RuleSource, Rulebook, Session, StellarId, SystemId, TagNames,
};

fn bit(n: u16) -> Bit {
    Bit::new(n).expect("a bit")
}

/// Never fires; every draw and roll is the last outcome, so only a
/// `crön` of `Random` 100 or more starts.
struct Last;

impl Chance for Last {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

#[test]
fn every_stock_crons_unused_news_fields_read_as_none() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let crons = data.crons();
    assert!(crons.len() > 100, "{} crons", crons.len());
    let mut zero_slots = 0;
    let mut zero_independent = 0;
    for record in &crons {
        let raw = data
            .get::<Cron>(record.id.0)
            .expect("listed")
            .expect("decodes")
            .record;
        let raw_slot = |govt: Option<GovtId>| govt.map_or(-1, |id| id.0);
        let expected: Vec<(GovtId, i16)> = raw
            .news_govt
            .iter()
            .zip(raw.govt_news_str)
            .filter(|&(&govt, _)| raw_slot(govt) >= 128)
            .map(|(&govt, news)| (govt.expect("a government"), news.map_or(-1, |id| id.0)))
            .collect();
        assert_eq!(
            news::slots(record).collect::<Vec<_>>(),
            expected,
            "crön {}",
            record.id.0
        );
        for (_, news) in news::slots(record) {
            assert!(news > 0, "crön {}'s used slot has a story", record.id.0);
        }
        zero_slots += record.news_govt.iter().filter(|&&govt| govt == 0).count();
        let ind = raw.ind_news_str.map_or(-1, |id| id.0);
        if ind <= 0 {
            zero_independent += usize::from(ind == 0);
            assert_eq!(news::independent(record), None, "crön {}", record.id.0);
        } else {
            assert_eq!(news::independent(record), Some(ind), "crön {}", record.id.0);
        }
    }
    assert!(zero_slots > 300, "stock has 367 slots of 0: {zero_slots}");
    assert!(zero_independent > 50, "IndNews 0: {zero_independent}");
}

/// A new stock pilot reading the stock strings, its rules `rules`, with
/// crön 231 running and docked at Merrol.
fn at_merrol(data: &Rc<GameData>, rules: Rulebook) -> Session {
    let mut session = Session::start(data.as_ref())
        .expect("the stock first chär starts")
        .with_strings(Rc::clone(data) as Rc<dyn CommCatalog>)
        .with_rules(rules);
    let year = session.date().year();
    // Outside crön 156's 1 Sep to 30 Dec, so it is not running.
    session.set_date(GameDate::new(year, 3, 1).expect("a date"));
    session.set_control_bit(bit(8906), true);
    session.set_control_bit(bit(8907), false);
    session.let_days_pass(1, &mut Last);
    let state = session.pilot().crons().get(&CronId(231)).copied();
    assert_eq!(state.map(|state| state.holdoff), Some(0), "crön 231 runs");
    session
        .land_at(data.as_ref(), SystemId(141), StellarId(171))
        .expect("Merrol is landable");
    session
}

#[test]
fn a_running_stock_crons_news_shows_at_an_allied_governments_spaceport() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let story = data.string_list(15_083);
    assert_eq!(story.len(), 1, "STR# 15083 has one story");
    let commercials = data.string_list(COMMERCIALS);
    let mut session = at_merrol(&data, Rulebook::default());
    assert!(session.open_bar(&mut Last), "Merrol has a bar");
    let news = session.news().expect("drawn").clone();
    assert_eq!(news.story, story[0]);
    assert_eq!(
        Some(&news.commercial),
        commercials.last(),
        "STR# 8100's last string, as the roll is the last"
    );

    let bible = Rulebook::default().with_override(RuleKey::NewsGovtMatch, RuleSource::Bible);
    let mut session = at_merrol(&data, bible);
    assert!(session.open_bar(&mut Last));
    let generic = data.string_list(GENERIC_NEWS);
    assert_eq!(
        session.news().map(|news| &news.story),
        generic.last(),
        "only Polaris's stellars show crön 231's news"
    );
}

/// The stock names: the stellars' and systems' from their resources, the
/// cargo names from `STR#` 4001.
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

#[test]
fn the_food_surplus_makes_the_news_at_port_kane() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let surplus = data
        .disasters()
        .into_iter()
        .find(|event| event.id == DisasterId(128))
        .expect("öops 128");
    assert_eq!((surplus.stellar, surplus.commodity), (137, 0));
    let names = StockNames {
        places: data.place_names(),
        cargo: data.string_list(CARGO_NAMES),
        listed: data.listed_stellars(),
    };
    assert_eq!(
        news::oops_story(
            &surplus,
            Some(StellarId(137)),
            &StoryWords::read(&data),
            &names
        )
        .as_deref(),
        Some(
            "Today's top news: An enormous food surplus has lowered the price of food on Port Kane."
        )
    );
}
