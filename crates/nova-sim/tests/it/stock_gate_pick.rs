//! The land key's and F5's nearest pick over the stock data
//! (stellar-fidelity phase 14, `_FindNearestStellar` @0x276c-0x2888):
//! wormhole 466 is listed by Spica (144) and K-005 (436, visible while
//! `!b330`), and Spica, the lower visible lister, owns it. In K-005 an
//! untargeted L within 500 pixels of its spot there selects it and is
//! refused as not landable, F5 does nothing, and from further off L
//! passes it over. Skips, passing, when `NOVA_DATA` is unset.

use crate::common;

use nova_data::GameData;
use nova_sim::flight::{Controls, Face};
use nova_sim::geometry::Vec2;
use nova_sim::pilot::Pilot;
use nova_sim::{
    LandingRefusal, PilotCatalog, RuleKey, RuleSource, Rulebook, Session, SimSound, StellarId,
    StellarPick, SystemId,
};

/// K-005.
const K005: SystemId = SystemId(436);
/// The wormhole Spica owns and K-005 lists.
const WORMHOLE: StellarId = StellarId(466);

/// A new stock pilot flying in K-005 by `rules`, at its centre: the
/// pilot's stellar is `None`, so the flight starts there.
fn in_k005(data: &GameData, rules: Rulebook) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::Value::Null;
    save["system"] = serde_json::json!(K005.0);
    let there = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let session = Session::fly(data, there).expect("flies").with_rules(rules);
    assert_eq!(session.system(), K005);
    assert_eq!(session.player().position, Vec2::ZERO, "at the centre");
    assert_eq!(session.stellars_shown().count(), 0, "466 is Spica's");
    session
}

/// The wormhole's spot in K-005.
fn wormhole_spot(data: &GameData) -> Vec2 {
    data.landing_sites(K005)
        .into_iter()
        .find(|site| site.id == WORMHOLE)
        .expect("K-005 lists 466")
        .position
}

/// The sounds `session` raised since it was last drained, in order.
fn sounds(session: &mut Session) -> Vec<SimSound> {
    common::sounds(&common::drained(session))
}

fn refused() -> LandingRefusal {
    LandingRefusal::NotLandable {
        stellar: WORMHOLE,
        station: false,
    }
}

#[test]
fn in_k005_from_the_centre_l_passes_the_far_wormhole_over() {
    let Some(data) = common::opened() else {
        return;
    };
    let spot = wormhole_spot(&data);
    assert!(spot.length() > 500.0, "{spot:?}");
    let mut session = in_k005(&data, Rulebook::default());
    assert_eq!(session.land(), Err(LandingRefusal::NothingLandable));
    assert_eq!(session.nav_target(), None);
    assert_eq!(sounds(&mut session), [], "silent");
}

#[test]
fn in_k005_within_500_of_the_wormhole_l_selects_it_and_is_refused_and_f5_does_nothing() {
    let Some(data) = common::opened() else {
        return;
    };
    let spot = wormhole_spot(&data);
    // By the other reading L selects the wormhole from anywhere, which
    // lets the ship face it on the way.
    let any_distance = Rulebook::default().with_override(RuleKey::GatePickRange, RuleSource::Bible);
    let mut session = in_k005(&data, any_distance);
    assert_eq!(session.land(), Err(refused()));
    assert_eq!(session.nav_target(), Some(WORMHOLE));
    let towards = Controls {
        thrust: true,
        face: Face::Stellar,
        ..Controls::default()
    };
    let mut ticks = 0;
    while (session.player().position - spot).length() > 300.0 {
        crate::passes::work(&mut session, &data, towards);
        ticks += 1;
        assert!(ticks < 20_000, "reaches the wormhole");
    }
    assert!(session.nav_off());
    let mut session = session.with_rules(Rulebook::default());
    assert!((session.player().position - spot).length() < 500.0);
    sounds(&mut session);

    assert_eq!(session.select_stellar(StellarPick::Nearest), None, "F5");
    assert_eq!(session.nav_target(), None);
    assert_eq!(sounds(&mut session), [], "F5 is silent");

    assert_eq!(session.land(), Err(refused()));
    assert_eq!(
        session.nav_target(),
        Some(WORMHOLE),
        "selected all the same"
    );
    assert_eq!(sounds(&mut session), [SimSound::Refused]);
}
