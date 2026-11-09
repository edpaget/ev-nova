//! The stock look's scale and resolution, over synthetic stores built in
//! code: a new-game store of lineages and `shän` sizes, and stock art from
//! [`fixture`].

use std::collections::{BTreeMap, HashSet};

use nova_galaxy_format::fleet::Role;

use super::fixture::stock_art;
use super::*;
use crate::Record;
use crate::records::fixture::ShipAnimBuilder;
use crate::store::fixture::store_of;

#[test]
fn a_scale_is_the_hull_s_longer_side_over_the_art_s() {
    assert_eq!(Scale::of(64, 32).apply(10), 20);
    assert_eq!(Scale::of(30, 40).apply(40), 30);
    assert_eq!(
        (
            Scale::of(64, 32).numerator(),
            Scale::of(64, 32).denominator()
        ),
        (2, 1),
        "in lowest terms"
    );
}

#[test]
fn a_hull_with_no_size_is_one_to_one() {
    assert_eq!(Scale::of(0, 32), Scale::ONE);
    assert_eq!(Scale::of(12, 0), Scale::ONE);
    assert_eq!(Scale::ONE.apply(37), 37);
}

#[test]
fn applying_a_scale_rounds_and_never_gives_zero() {
    assert_eq!(Scale::of(2, 3).apply(4), 3, "2.67");
    assert_eq!(Scale::of(1, 2).apply(3), 2, "1.5 rounds up");
    assert_eq!(Scale::of(1, 3).apply(4), 1, "1.33");
    assert_eq!(Scale::of(1, 100).apply(3), 1, "0.03 is still a pixel");
    assert_eq!(Scale::of(1, 2).apply(0), 1);
}

#[test]
fn equal_ratios_are_equal_and_hash_alike() {
    assert_eq!(Scale::of(2, 4), Scale::of(1, 2));
    assert_eq!(Scale::of(30, 30), Scale::ONE);
    let set: HashSet<Scale> = [Scale::of(2, 4), Scale::of(1, 2), Scale::of(3, 6)].into();
    assert_eq!(set.len(), 1);
    assert_ne!(Scale::of(1, 2), Scale::of(2, 1));
}

/// The default stock `shïp` of `role`.
fn default_of(role: Role) -> i16 {
    ROLE_DEFAULTS
        .iter()
        .find(|(of, _)| *of == role)
        .map(|&(_, stock)| stock)
        .expect("every role has a default")
}

/// One new-game ship: its `shïp`, its chain, its role and its `shän`'s
/// beam and length (none for a ship with no size).
struct Hull {
    id: i16,
    chain: &'static [&'static str],
    role: Option<Role>,
    size: Option<(i16, i16)>,
}

/// The new game's store of `hulls`.
fn game(hulls: &[Hull]) -> GameData {
    let anims: Vec<_> = hulls
        .iter()
        .filter_map(|hull| {
            let (beam, length) = hull.size?;
            let anim = ShipAnimBuilder::new()
                .base_x_size(beam)
                .base_y_size(length)
                .build();
            Some((ShipAnim::TYPE, hull.id, anim))
        })
        .collect();
    let lines = hulls
        .iter()
        .map(|hull| {
            let line = ShipLine {
                chain: hull.chain.iter().map(|&id| id.to_owned()).collect(),
                role: hull.role,
            };
            (ShipId(hull.id), line)
        })
        .collect();
    store_of(&anims).with_ship_lines(lines)
}

/// The freighter `hauler`, 12 x 30.
fn hauler() -> Hull {
    Hull {
        id: 128,
        chain: &["hauler"],
        role: Some(Role::Freighter),
        size: Some((12, 30)),
    }
}

/// `armed-hauler`, a variant of the hauler.
fn armed_hauler() -> Hull {
    Hull {
        id: 129,
        chain: &["armed-hauler", "hauler"],
        ..hauler()
    }
}

fn overrides(entries: &[(&str, i16)]) -> BTreeMap<String, i16> {
    entries
        .iter()
        .map(|&(key, stock)| (key.to_owned(), stock))
        .collect()
}

fn borrow(stock: i16, scale: Scale) -> Borrow {
    Borrow {
        stock: ShipId(stock),
        scale,
    }
}

#[test]
fn a_hull_with_no_override_borrows_its_role_s_default_scaled_to_its_size() {
    let freighter = default_of(Role::Freighter);
    let stock = stock_art(&[(freighter, 40)]);
    let resolved = resolve(&game(&[hauler()]), &stock, &BTreeMap::new());
    assert_eq!(
        resolved.borrows,
        BTreeMap::from([(ShipId(128), borrow(freighter, Scale::of(30, 40)))])
    );
    assert_eq!(resolved.warnings, Vec::<String>::new());
}

#[test]
fn a_hull_s_own_override_wins_over_its_role() {
    let stock = stock_art(&[(default_of(Role::Freighter), 40), (300, 60)]);
    let resolved = resolve(&game(&[hauler()]), &stock, &overrides(&[("hauler", 300)]));
    assert_eq!(
        resolved.borrows,
        BTreeMap::from([(ShipId(128), borrow(300, Scale::of(30, 60)))])
    );
}

#[test]
fn a_variant_borrows_its_base_s_override_unless_it_has_its_own() {
    let stock = stock_art(&[(default_of(Role::Freighter), 40), (300, 60), (301, 30)]);
    let both = game(&[hauler(), armed_hauler()]);
    let based = resolve(&both, &stock, &overrides(&[("hauler", 300)]));
    assert_eq!(based.borrows[&ShipId(129)], borrow(300, Scale::of(30, 60)));
    let own = resolve(
        &both,
        &stock,
        &overrides(&[("hauler", 300), ("armed-hauler", 301)]),
    );
    assert_eq!(own.borrows[&ShipId(129)], borrow(301, Scale::ONE));
    assert_eq!(own.borrows[&ShipId(128)], borrow(300, Scale::of(30, 60)));
}

#[test]
fn a_hull_with_no_size_borrows_at_the_art_s_own_size() {
    let stock = stock_art(&[(default_of(Role::Freighter), 40)]);
    let sizeless = Hull {
        size: None,
        ..hauler()
    };
    let resolved = resolve(&game(&[sizeless]), &stock, &BTreeMap::new());
    assert_eq!(resolved.borrows[&ShipId(128)].scale, Scale::ONE);
}

#[test]
fn hulls_with_no_pick_are_left_out_with_one_warning_naming_the_first() {
    let roleless = |id, chain| Hull {
        id,
        chain,
        role: None,
        size: Some((10, 10)),
    };
    let one = resolve(
        &game(&[roleless(130, &["drone"])]),
        &stock_art(&[]),
        &BTreeMap::new(),
    );
    assert_eq!(one.borrows, BTreeMap::new());
    assert_eq!(
        one.warnings,
        [
            "1 hull borrows no stock ship, starting with `drone`: no override names it or a \
          base of it, and it has no role; it draws its stand-in"
        ]
    );
    let two = resolve(
        &game(&[roleless(130, &["drone"]), roleless(131, &["buoy"])]),
        &stock_art(&[]),
        &BTreeMap::new(),
    );
    assert_eq!(
        two.warnings,
        [
            "2 hulls borrow no stock ship, starting with `drone`: no override names it or a \
          base of it, and it has no role; each draws its stand-in"
        ]
    );
}

#[test]
fn picks_not_in_the_stock_data_are_left_out_with_one_warning_naming_the_first() {
    let stock = stock_art(&[]);
    let resolved = resolve(
        &game(&[hauler(), armed_hauler()]),
        &stock,
        &overrides(&[("hauler", 300)]),
    );
    assert_eq!(resolved.borrows, BTreeMap::new());
    assert_eq!(
        resolved.warnings,
        [
            "2 hulls can't borrow their stock ship, starting with `hauler`'s shïp 300: no shïp \
          300; each draws its stand-in"
        ]
    );
}

#[test]
fn a_pick_whose_sheet_does_not_read_is_left_out_with_its_reason() {
    use crate::records::fixture::ShipBuilder;
    use crate::records::ship::Ship;
    // A stock shïp with no shän.
    let stock = StockArt::new(store_of(&[(Ship::TYPE, 300, ShipBuilder::new().build())]));
    let resolved = resolve(&game(&[hauler()]), &stock, &overrides(&[("hauler", 300)]));
    assert_eq!(resolved.borrows, BTreeMap::new());
    let reason = stock
        .ship_sprite_info(ShipId(300))
        .expect_err("no sheet")
        .to_string();
    assert_eq!(
        resolved.warnings,
        [format!(
            "1 hull can't borrow its stock ship, starting with `hauler`'s shïp 300: {reason}; \
             it draws its stand-in"
        )]
    );
}

#[test]
fn overrides_naming_no_new_game_ship_are_one_warning_naming_the_first() {
    let stock = stock_art(&[(default_of(Role::Freighter), 40), (300, 60)]);
    let resolved = resolve(
        &game(&[hauler()]),
        &stock,
        &overrides(&[("zeppelin", 300), ("blimp", 300), ("hauler", 300)]),
    );
    assert_eq!(resolved.borrows[&ShipId(128)].stock, ShipId(300));
    assert_eq!(
        resolved.warnings,
        ["2 overrides name no new-game ship, starting with `blimp`; they are ignored"]
    );
}

#[test]
fn every_kind_of_trouble_is_one_warning_each_in_a_fixed_order() {
    let roleless = Hull {
        id: 130,
        chain: &["drone"],
        role: None,
        size: None,
    };
    let resolved = resolve(
        &game(&[hauler(), roleless]),
        &stock_art(&[]),
        &overrides(&[("zeppelin", 300)]),
    );
    assert_eq!(resolved.warnings.len(), 3, "{:?}", resolved.warnings);
    assert!(resolved.warnings[0].contains("borrows no stock ship"));
    assert!(resolved.warnings[1].contains("can't borrow"));
    assert!(resolved.warnings[2].contains("no new-game ship"));
}

#[test]
fn no_overrides_and_an_empty_stock_store_give_no_borrows_and_one_warning() {
    let resolved = resolve(
        &game(&[hauler(), armed_hauler()]),
        &stock_art(&[]),
        &BTreeMap::new(),
    );
    assert_eq!(resolved.borrows, BTreeMap::new());
    assert_eq!(resolved.warnings.len(), 1, "{:?}", resolved.warnings);
}

#[test]
fn a_stock_store_s_ships_have_no_lines_so_nothing_borrows() {
    let stock = stock_art(&[(default_of(Role::Freighter), 40)]);
    let resolved = resolve(&GameData::default(), &stock, &BTreeMap::new());
    assert_eq!(resolved, Resolved::default());
}

#[test]
fn every_role_has_one_default() {
    for role in [
        Role::Shuttle,
        Role::Freighter,
        Role::Courier,
        Role::Racer,
        Role::Fighter,
        Role::Raider,
        Role::Gunboat,
        Role::Corvette,
        Role::Cruiser,
        Role::Carrier,
    ] {
        let found = ROLE_DEFAULTS.iter().filter(|(of, _)| *of == role).count();
        assert_eq!(found, 1, "{role:?}");
    }
}
