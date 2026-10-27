//! The cloak key, U held.

use super::*;

/// Ship 128 carrying outfit 300, a cloak of no usage.
fn cloaking() -> FakeCatalog {
    FakeCatalog {
        outfits: vec![OutfitRecord {
            id: OutfitId(300),
            name: "Cloaking Device".to_owned(),
            short_name: "Cloaking Device".to_owned(),
            disp_weight: 0,
            mass: 0,
            tech_level: 1,
            max: 1,
            flags: 0,
            cost: 0,
            mods: [(nova_sim::cloak::MOD_CLOAK, 0), (0, 0), (0, 0), (0, 0)],
            contribute: 0,
            require: 0,
            require_govt: -1,
            buy_random: 100,
            availability: nova_sim::Test::default(),
            item_class: 0,
            lc_name: "cloaking device".to_owned(),
            lc_plural: "cloaking devices".to_owned(),
            scan_mask: 0,
            on_purchase: nova_sim::Script::default(),
            on_sell: nova_sim::Script::default(),
        }],
        defaults: vec![(OutfitId(300), 1)],
        ..catalog()
    }
}

#[test]
fn cloak_key_is_u() {
    // `cloakKey`, keymap slot 0x52, keycode 0x20 (`_loadKeys`
    // @0xce461-0xce4d4).
    assert_eq!(CLOAK_KEY, Key::Char('u'));
}

#[test]
fn holding_u_cloaks_a_player_owning_a_cloak() {
    let mut view = FlightView::new(cloaking());
    view.input(&key(CLOAK_KEY, true));
    view.take_sounds();
    ticks(&mut view, 1);
    let sounds = sims(&mut view);
    assert!(sounds.contains(&SimSound::Cloaked), "{sounds:?}");
    let cloak = view.session().expect("flying").player_cloak();
    assert_eq!(cloak.direction, 1, "fading in");
    view.input(&key(CLOAK_KEY, false));
    ticks(&mut view, 40);
    let cloak = view.session().expect("flying").player_cloak();
    assert!(cloak.is_cloaked(), "{cloak:?}");
}

#[test]
fn u_without_a_cloak_beeps_refused() {
    let mut view = flight();
    view.input(&key(CLOAK_KEY, true));
    view.take_sounds();
    ticks(&mut view, 1);
    let sounds = sims(&mut view);
    assert!(sounds.contains(&SimSound::Refused), "{sounds:?}");
}
