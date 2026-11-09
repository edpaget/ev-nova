//! The new-game fixture's outfitters played headless through the
//! session's own commands: Hearth Dock, a law-keeping yard, fits the legal
//! cargo racks and sells the mass driver loose, which its `sold_at` names;
//! Verge Workshop, an unowned outpost, a lawless yard, fits the gun mount,
//! which adds a turret hardpoint. The mass driver, a turret, is listed
//! only for a hull with a turret hardpoint, and only at Hearth Dock.
//!
//! This pins only that the fixture's data, the new game's mapping
//! (`nova_data::new_game`) and the outfitter meet. The rules are pinned
//! in their crates: the mapping (tech numbers, `SpecialTech` slots, the
//! mount bits) in nova-data, the outfitter's listing and buying in
//! nova-sim-port.

use nova_data::new_game::fixture::folder;
use nova_sim::outfitter::OutfitOrder;
use nova_sim::{Direction, OutfitId};

use crate::new_game_play::Play;
use crate::new_game_travel::{fixture, nova_of};

/// The cash every pilot here starts with.
const CASH: i64 = 20_000;

/// The fixture's start in Hearth, with [`CASH`] credits, owning one of
/// each of `owned` (kits by their bible ids).
fn play(owned: &[&str]) -> Play {
    let game = fixture();
    let owned: Vec<OutfitId> = owned
        .iter()
        .map(|id| OutfitId(nova_of(&game.ids.kits, id)))
        .collect();
    Play::new(&folder(), "hearth", |pilot| {
        pilot.cash = CASH;
        for kit in owned {
            pilot.outfits.insert(kit, 1);
        }
    })
}

fn kit(play: &Play, id: &str) -> OutfitId {
    OutfitId(nova_of(&play.game.ids.kits, id))
}

fn loose(play: &Play, id: &str) -> OutfitId {
    OutfitId(nova_of(&play.game.ids.outfits, id))
}

/// The outfits the outfitter where the ship is docked lists.
fn listed(play: &mut Play) -> Vec<OutfitId> {
    let outfitter = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter");
    outfitter.rows.iter().map(|row| row.id).collect()
}

/// Buys one `outfit` where the ship is docked: whether the outfitter sold
/// it.
fn buy(play: &mut Play, outfit: OutfitId) -> bool {
    let order = OutfitOrder {
        outfit,
        direction: Direction::Buy,
    };
    play.session.outfit(order, &mut play.chance).is_ok()
}

/// The turrets the ship carries and the most it can.
fn turrets(play: &mut Play) -> (i32, i32) {
    let armed = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter")
        .armed;
    (armed.turrets, armed.turret_limit)
}

/// From Hearth to Verge by the side road, landed at Verge Workshop.
fn to_verge_workshop(play: &mut Play) {
    play.go_through("hearth/side-road");
    play.land("verge-workshop");
}

#[test]
fn a_legal_kit_is_fitted_at_a_law_keeping_yard_and_a_lawless_one_only_at_a_lawless_yard() {
    let mut play = play(&[]);
    let racks = kit(&play, "cargo-racks");
    let mount = kit(&play, "gun-mount");

    play.land("hearth-dock");
    let legal = listed(&mut play);
    assert!(legal.contains(&racks), "{legal:?}");
    assert!(!legal.contains(&mount), "{legal:?}");
    let bare = play.session.stats().capacity;
    assert!(buy(&mut play, racks));
    assert_eq!(play.session.pilot().cash(), CASH - 2000);
    assert_eq!(play.session.stats().capacity, bare + 10);

    to_verge_workshop(&mut play);
    let lawless = listed(&mut play);
    assert!(lawless.contains(&mount), "{lawless:?}");
    assert_eq!(turrets(&mut play), (0, 0));
    assert!(buy(&mut play, mount));
    assert_eq!(play.session.pilot().cash(), CASH - 2000 - 3000);
    assert_eq!(play.session.stats().capacity, bare + 10 - 6);
    assert_eq!(turrets(&mut play), (0, 1), "a turret hardpoint, no gun");
}

#[test]
fn a_loose_weapon_is_listed_only_for_a_hull_with_a_hardpoint_of_its_mount() {
    let mut bare = play(&[]);
    let driver = loose(&bare, "mass-driver");
    bare.land("hearth-dock");
    let listed_bare = listed(&mut bare);
    assert!(!listed_bare.contains(&driver), "{listed_bare:?}");
    assert!(!buy(&mut bare, driver), "no turret hardpoint");
    assert_eq!(bare.session.pilot().owned(driver), 0);

    let mut mounted = play(&["gun-mount"]);
    mounted.land("hearth-dock");
    let listed_mounted = listed(&mut mounted);
    assert!(listed_mounted.contains(&driver), "{listed_mounted:?}");
    assert!(buy(&mut mounted, driver));
    assert_eq!(mounted.session.pilot().cash(), CASH - 2500);
    assert_eq!(mounted.session.pilot().owned(driver), 1);
    assert_eq!(turrets(&mut mounted), (1, 1));
}

#[test]
fn a_loose_outfit_is_sold_where_the_data_sells_it_and_nowhere_else() {
    let mut play = play(&["gun-mount"]);
    let driver = loose(&play, "mass-driver");

    to_verge_workshop(&mut play);
    let workshop = listed(&mut play);
    assert!(!workshop.contains(&driver), "{workshop:?}");
    assert!(!buy(&mut play, driver), "not sold here");
    assert_eq!(play.session.pilot().cash(), CASH);

    play.go_through("verge/side-road");
    play.land("hearth-dock");
    let dock = listed(&mut play);
    assert!(dock.contains(&driver), "{dock:?}");
    assert!(buy(&mut play, driver));
    assert_eq!(play.session.pilot().owned(driver), 1);
    assert_eq!(play.session.pilot().cash(), CASH - 2500);
}
