//! Domination over the stock data (play-feel-4 phase 3): Earth dominated
//! at run time pays its tribute each day (`_DoStellarTimePassage`
//! @0x40867-0x408d4), and stays dominated across a save, its defence
//! ships left at 0 (`_LoadPilotData` @0x75d70-0x75db0). Skips, passing,
//! when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::player_info::tribute_of;
use nova_sim::{NeverFires, PilotCatalog, Session, StellarId, fixture, save};

const EARTH: StellarId = StellarId(128);

/// The cash a day adds in `session`.
fn day_pays(session: &mut Session) -> i64 {
    let before = session.pilot().cash;
    session.let_days_pass(1, &mut NeverFires);
    session.pilot().cash - before
}

#[test]
fn earth_dominated_at_run_time_pays_its_tribute_and_stays_dominated_across_a_save() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let listed = data.listed_stellars();
    let earth = listed
        .iter()
        .find(|listed| listed.id == EARTH)
        .expect("a system lists Earth");
    let tribute = tribute_of(earth);
    assert_eq!(tribute, 10_000, "Earth's Tribute");
    let mut free = Session::start(&data).expect("the stock first chär starts");
    let mut dominating = Session::start(&data).expect("the stock first chär starts");
    fixture::dominate(&mut dominating, EARTH, true);
    let base = day_pays(&mut free);
    assert_eq!(day_pays(&mut dominating), base + tribute, "one day");
    let text = save::encode(dominating.pilot());
    let pilot = save::decode(&text).expect("loads");
    let mut back = Session::fly(&data, pilot).expect("flies");
    assert!(back.pilot().galaxy.stellar(EARTH).dominated, "kept");
    assert_eq!(
        back.pilot().defence_ships.left_of(EARTH, earth.def_count),
        0,
        "a dominated stellar's count loads as 0"
    );
    assert_eq!(
        day_pays(&mut back),
        day_pays(&mut free) + tribute,
        "still paying"
    );
}
