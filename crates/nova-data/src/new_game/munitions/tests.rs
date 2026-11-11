//! Launchers and bays over the format's test galaxy
//! (`super::super::tests::galaxy`: the blaster, the skiff carrying it on
//! its nose, and the armed skiff with its gun-mount kit), with a homing
//! rocket rack firing seekers and a drone bay launching drones added a
//! file at a time.

use std::path::Path;

use nova_galaxy_format::id::Id;
use nova_galaxy_format::source::MemSource;

use super::super::kits::HIDE_UNLESS_REQUIRED;
use super::super::outfits::{HOMING_TURN, MOD_WEAPON, OUTFIT_GUN, SHOT_SPEED, WEAPON_SECONDARY};
use super::super::records::{OUTFIT_TECH_NEVER, neutral_weapon};
use super::super::tests::galaxy;
use super::super::{NewGame, load};
use super::*;
use crate::records::weapon::Weapon;
use crate::wire::flags::Flags16;
use crate::wire::id::WeaponId;

fn loaded(source: &MemSource) -> NewGame {
    load(source, Path::new("games"))
}

fn id(text: &str) -> Id {
    Id::try_from(text.to_owned()).expect("an id")
}

fn problems(game: &NewGame) -> Vec<String> {
    game.problems.iter().map(ToString::to_string).collect()
}

fn record<T: crate::Registered + Clone>(game: &NewGame, nova: i16) -> T {
    (game.data.get::<T>(nova))
        .expect("present")
        .expect("decoded")
        .record
        .clone()
}

fn outfit(game: &NewGame, outfit: &str) -> Outfit {
    record(game, game.ids.outfits[&id(outfit)])
}

fn weapon_id(game: &NewGame, outfit: &str) -> i16 {
    game.ids.weapons[&id(outfit)]
}

fn weapon(game: &NewGame, outfit: &str) -> Weapon {
    record(game, weapon_id(game, outfit))
}

fn ship(game: &NewGame, ship: &str) -> Ship {
    record(game, game.ids.ships[&id(ship)])
}

/// The `(WeapCount, AmmoLoad)` of `ship`'s stock slot of `weapon`.
fn slot(ship: &Ship, weapon: i16) -> (i16, i16) {
    let types = ship.weap_type1_4.iter().chain(&ship.weap_type5_8);
    let counts = ship.weap_count1_4.iter().chain(&ship.weap_count5_8);
    let loads = ship.ammo_load1_4.iter().chain(&ship.ammo_load5_8);
    types
        .zip(counts.zip(loads))
        .find(|(held, _)| **held == Some(WeaponId(weapon)))
        .map(|(_, (&count, &load))| (count, load))
        .expect("a slot of the weapon")
}

/// A launcher `name` naming `words`, damage 20, range 900, reload 1.
fn launcher(name: &str, words: &str) -> String {
    format!(
        "---\nname: {name}\nmount: launcher\ndamage: 20\nrange: 900\nreload: 1\nprice: 3000\n\
         {words}---\n"
    )
}

/// An outfit `name` on `mount` of no damage, priced 50, naming `words`.
fn harmless(name: &str, mount: &str, words: &str) -> String {
    format!(
        "---\nname: {name}\nmount: {mount}\ndamage: 0\nrange: 0\nreload: 0\nprice: 50\n{words}---\n"
    )
}

/// A bay `name` naming `words`, range 300, reload 2.
fn bay(name: &str, words: &str) -> String {
    format!(
        "---\nname: {name}\nmount: bay\ndamage: 0\nrange: 300\nreload: 2\nprice: 4000\n{words}---\n"
    )
}

/// The skiff with its nose blaster and `extra` hardpoints.
fn skiff_with(extra: &str) -> String {
    format!(
        "---\nname: Skiff\nstyle: civilian\nrole: shuttle\nmodel: hull\nlivery: league-paint\n\
         size: {{length: 8, beam: 4, collision_radius: 4}}\nstats: {{hull: 50, cargo: 10}}\n\
         hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 3]\n    outfit: blaster\n\
         {extra}---\n"
    )
}

/// A hardpoint `id` of `mount` carrying `outfit`.
fn hardpoint(id: &str, mount: &str, outfit: &str) -> String {
    format!("  - id: {id}\n    mount: {mount}\n    position: [0, 3]\n    outfit: {outfit}\n")
}

/// The test galaxy with the homing rocket rack, firing seekers 3 a
/// hardpoint, on two of the skiff's launcher hardpoints, and the seekers.
fn racked() -> MemSource {
    galaxy()
        .with(
            "outfits/rocket.md",
            &launcher(
                "Rocket rack",
                "guidance: homing\nammunition: {outfit: seekers, load: 3}\n",
            ),
        )
        .with("outfits/seekers.md", &harmless("Seekers", "launcher", ""))
        .with(
            "ships/skiff.md",
            &skiff_with(&format!(
                "{}{}",
                hardpoint("left", "launcher", "rocket"),
                hardpoint("right", "launcher", "rocket")
            )),
        )
}

/// The test galaxy with a drone bay, launching drones 2 a hardpoint, on a
/// carrier's one bay hardpoint, and the drone.
fn carrying() -> MemSource {
    galaxy()
        .with(
            "outfits/drone-bay.md",
            &bay("Drone bay", "fighters: {hull: drone, load: 2}\n"),
        )
        .with(
            "ships/drone.md",
            "---\nname: Drone\nstyle: crown\nrole: fighter\nmodel: hull\n\
             size: {length: 4, beam: 2, collision_radius: 2}\nstats: {hull: 20, speed: 600}\n---\n",
        )
        .with(
            "ships/carrier.md",
            &format!(
                "---\nname: Carrier\nstyle: crown\nrole: carrier\nmodel: hull\n\
                 size: {{length: 8, beam: 4, collision_radius: 4}}\nstats: {{hull: 300}}\n\
                 hardpoints:\n{}---\n",
                hardpoint("hangar", "bay", "drone-bay")
            ),
        )
}

// Launchers.

#[test]
fn a_homing_launcher_firing_ammunition_is_a_secondary_spending_its_own_rounds() {
    let game = loaded(&racked());
    assert_eq!(problems(&game), Vec::<String>::new());
    let rocket = weapon_id(&game, "rocket");
    assert_eq!(
        weapon(&game, "rocket"),
        Weapon {
            reload: 30,
            count: 90,
            mass_dmg: 20,
            energy_dmg: 20,
            guidance: 1,
            guided_turn: HOMING_TURN,
            ammo_type: rocket - FIRST_AMMO_WEAPON,
            flags: Flags16(WEAPON_SECONDARY),
            speed: SHOT_SPEED,
            exit_type: 2,
            ..neutral_weapon()
        }
    );
    assert_eq!(HOMING_TURN, 70, "7 degrees a tick");
    let seekers = outfit(&game, "seekers");
    assert_eq!((seekers.mod_type, seekers.mod_val), (MOD_AMMO, rocket));
    assert_eq!(MOD_AMMO, 3);
    assert_eq!(seekers.flags, Flags16(HIDE_UNLESS_REQUIRED), "no gun slot");
    assert_eq!(seekers.cost, 50);
    assert_eq!(
        (
            outfit(&game, "rocket").mod_type,
            outfit(&game, "rocket").mod_val
        ),
        (MOD_WEAPON, rocket)
    );
    assert_eq!(
        slot(&ship(&game, "skiff"), rocket),
        (2, 6),
        "two hardpoints of three rounds"
    );
    assert_eq!(
        slot(&ship(&game, "skiff"), weapon_id(&game, "blaster")),
        (1, 0),
        "the blaster fires without ammunition"
    );
    assert_eq!(
        slot(&ship(&game, "armed-skiff"), rocket),
        (2, 6),
        "inherited"
    );
}

#[test]
fn guidance_alone_homes_without_ammunition_and_ammunition_alone_flies_unguided() {
    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &launcher("Rocket rack", "guidance: homing\n"),
    ));
    let alone = weapon(&game, "rocket");
    assert_eq!(
        (
            alone.guidance,
            alone.guided_turn,
            alone.ammo_type,
            alone.flags
        ),
        (1, HOMING_TURN, -1, Flags16(0))
    );
    assert_eq!(outfit(&game, "seekers").mod_type, -1, "nobody's rounds");
    assert_eq!(
        slot(&ship(&game, "skiff"), weapon_id(&game, "rocket")),
        (2, 0)
    );

    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &launcher("Rocket rack", "ammunition: {outfit: seekers, load: 3}\n"),
    ));
    let rocket = weapon_id(&game, "rocket");
    let fed = weapon(&game, "rocket");
    assert_eq!(
        (fed.guidance, fed.guided_turn, fed.ammo_type, fed.flags),
        (-1, 0, rocket - FIRST_AMMO_WEAPON, Flags16(WEAPON_SECONDARY))
    );
    assert_eq!(slot(&ship(&game, "skiff"), rocket), (2, 6));
}

#[test]
fn two_launchers_naming_one_ammunition_share_the_lower_weapon_s_rounds() {
    let game = loaded(
        &racked()
            .with(
                "outfits/a-rack.md",
                &launcher("A rack", "ammunition: {outfit: seekers, load: 4}\n"),
            )
            .with(
                "ships/skiff.md",
                &skiff_with(&format!(
                    "{}{}",
                    hardpoint("left", "launcher", "rocket"),
                    hardpoint("right", "launcher", "a-rack")
                )),
            ),
    );
    let (lower, higher) = (weapon_id(&game, "a-rack"), weapon_id(&game, "rocket"));
    assert!(lower < higher, "a-rack numbers first");
    for rack in ["a-rack", "rocket"] {
        assert_eq!(
            weapon(&game, rack).ammo_type,
            lower - FIRST_AMMO_WEAPON,
            "{rack}"
        );
    }
    assert_eq!(outfit(&game, "seekers").mod_val, lower);
    let skiff = ship(&game, "skiff");
    assert_eq!(slot(&skiff, lower), (1, 4));
    assert_eq!(slot(&skiff, higher), (1, 3), "each its own load");
}

#[test]
fn a_kit_s_launcher_hardpoint_as_built_loads_the_ship_s_slot() {
    let game = loaded(&racked().with("ships/skiff.md", &skiff_with("")).with(
        "mods/gun-mount.md",
        &format!(
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
                     hardpoints:\n{}---\n",
            hardpoint("bed", "launcher", "rocket")
        ),
    ));
    let rocket = weapon_id(&game, "rocket");
    assert_eq!(slot(&ship(&game, "armed-skiff"), rocket), (1, 3));
    assert!(
        !(ship(&game, "skiff")
            .weap_type1_4
            .contains(&Some(WeaponId(rocket)))),
        "the bare hull has no rack"
    );
}

#[test]
fn a_load_past_i16_is_the_most() {
    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &launcher(
            "Rocket rack",
            "ammunition: {outfit: seekers, load: 4000000000}\n",
        ),
    ));
    assert_eq!(
        slot(&ship(&game, "skiff"), weapon_id(&game, "rocket")),
        (2, i16::MAX)
    );
}

/// The lines of `game`'s problems against `file`.
fn against(game: &NewGame, file: &str) -> Vec<String> {
    problems(game)
        .into_iter()
        .filter(|line| line.starts_with(&format!("{file}: ")))
        .collect()
}

#[test]
fn guidance_or_ammunition_off_a_launcher_and_fighters_off_a_bay_are_ignored() {
    let words = "guidance: homing\nammunition: {outfit: seekers, load: 3}\n\
                 fighters: {hull: skiff, load: 1}\n";
    let game = loaded(&racked().with(
        "outfits/blaster.md",
        &format!(
            "---\nname: Blaster\nmount: fixed\ndamage: 5\nrange: 300\nreload: 0.5\nprice: 100\n\
             {words}---\n"
        ),
    ));
    assert_eq!(
        against(&game, "outfits/blaster.md"),
        [
            "outfits/blaster.md: its `guidance` is read only on a launcher; it is ignored",
            "outfits/blaster.md: its `ammunition` is read only on a launcher; it is ignored",
            "outfits/blaster.md: its `fighters` is read only on a bay; it is ignored",
        ]
    );
    let blaster = weapon(&game, "blaster");
    assert_eq!(
        (blaster.guidance, blaster.ammo_type),
        (-1, -1),
        "a plain gun"
    );
    assert_eq!(
        outfit(&game, "seekers").mod_type,
        MOD_AMMO,
        "the rack's still"
    );

    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &launcher(
            "Rocket rack",
            "guidance: unguided\nfighters: {hull: skiff, load: 1}\n",
        ),
    ));
    assert_eq!(
        against(&game, "outfits/rocket.md"),
        ["outfits/rocket.md: its `fighters` is read only on a bay; it is ignored"],
        "`unguided` is no problem anywhere"
    );
}

#[test]
fn a_launcher_of_no_damage_ignores_its_guidance_and_ammunition() {
    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &harmless(
            "Rocket rack",
            "launcher",
            "guidance: homing\nammunition: {outfit: seekers, load: 3}\n",
        ),
    ));
    assert_eq!(
        against(&game, "outfits/rocket.md"),
        [
            "outfits/rocket.md: it does no damage, so it is no weapon; its `guidance` is ignored",
            "outfits/rocket.md: it does no damage, so it is no weapon; its `ammunition` is ignored",
        ]
    );
    assert!(!game.ids.weapons.contains_key(&id("rocket")));
    assert_eq!(outfit(&game, "seekers").mod_type, -1);
}

#[test]
fn a_launcher_whose_ammunition_is_missing_or_a_weapon_fires_without_ammunition() {
    let game = loaded(&racked().with(
        "outfits/rocket.md",
        &launcher(
            "Rocket rack",
            "guidance: homing\nammunition: {outfit: lost, load: 3}\n",
        ),
    ));
    assert_eq!(
        against(&game, "outfits/rocket.md"),
        [
            "outfits/rocket.md: its ammunition `lost` is not in outfits/; it fires without ammunition"
        ]
    );
    let rocket = weapon(&game, "rocket");
    assert_eq!(
        (rocket.guidance, rocket.ammo_type, rocket.flags),
        (1, -1, Flags16(0))
    );
    assert_eq!(
        slot(&ship(&game, "skiff"), weapon_id(&game, "rocket")),
        (2, 0)
    );

    for rounds in ["blaster", "rocket"] {
        let game = loaded(&racked().with(
            "outfits/rocket.md",
            &launcher(
                "Rocket rack",
                &format!("ammunition: {{outfit: {rounds}, load: 3}}\n"),
            ),
        ));
        assert_eq!(
            against(&game, "outfits/rocket.md"),
            [format!(
                "outfits/rocket.md: its ammunition `{rounds}` is a weapon; it fires without ammunition"
            )]
        );
        assert_eq!(weapon(&game, "rocket").ammo_type, -1, "{rounds}");
        assert_eq!(outfit(&game, rounds).mod_type, MOD_WEAPON, "{rounds}");
    }
}

#[test]
fn a_launcher_whose_ammunition_is_a_bay_launching_a_hull_fires_without_ammunition() {
    let game = loaded(&carrying().with(
        "outfits/rocket.md",
        &launcher("Rocket rack", "ammunition: {outfit: drone-bay, load: 3}\n"),
    ));
    assert_eq!(
        against(&game, "outfits/rocket.md"),
        ["outfits/rocket.md: its ammunition `drone-bay` is a weapon; it fires without ammunition"]
    );
    let rocket = weapon(&game, "rocket");
    assert_eq!((rocket.ammo_type, rocket.flags), (-1, Flags16(0)));
    let bay = outfit(&game, "drone-bay");
    assert_eq!(
        (bay.mod_type, bay.mod_val),
        (MOD_WEAPON, weapon_id(&game, "drone-bay")),
        "the bay's still a weapon"
    );
}

#[test]
fn ammunition_that_failed_to_load_is_not_reported_again() {
    let game = loaded(&racked().with("outfits/seekers.md", "---\nname: Seekers\n---\n"));
    assert_eq!(
        against(&game, "outfits/rocket.md"),
        Vec::<String>::new(),
        "{:?}",
        problems(&game)
    );
    assert_eq!(weapon(&game, "rocket").ammo_type, -1);
}

#[test]
fn ammunition_past_the_last_outfit_id_has_no_rounds_to_fire() {
    // 510 harmless outfits, the blaster and the rack fill the 512 `oütf`
    // IDs; the seekers, last by id, are left out.
    let source = (0..510).fold(racked(), |source, n| {
        source.with(&format!("outfits/a{n:03}.md"), &harmless("A", "fixed", ""))
    });
    let game = loaded(&source);
    assert!(!game.ids.outfits.contains_key(&id("seekers")));
    assert!(
        problems(&game).contains(
            &"outfits/seekers.md: it is past the last oütf ID, 639; it is left out".to_owned()
        ),
        "{:?}",
        problems(&game)
    );
    assert_eq!(weapon(&game, "rocket").ammo_type, -1, "no rounds to spend");
    assert_eq!(
        slot(&ship(&game, "skiff"), weapon_id(&game, "rocket")),
        (2, 0)
    );
}

// Bays.

#[test]
fn a_hull_carrying_a_bay_that_names_no_fighters_is_still_launched() {
    let game = loaded(&carrying().with("outfits/hold.md", &bay("Hold", "")).with(
        "ships/drone.md",
        &format!(
            "---\nname: Drone\nstyle: crown\nrole: fighter\nmodel: hull\n\
                 size: {{length: 4, beam: 2, collision_radius: 2}}\n\
                 hardpoints:\n{}---\n",
            hardpoint("hold", "bay", "hold")
        ),
    ));
    assert_eq!(against(&game, "outfits/drone-bay.md"), Vec::<String>::new());
    assert!(game.ids.weapons.contains_key(&id("drone-bay")));
    assert_eq!(ship(&game, "drone").escort_type, FIGHTER_ESCORT);
}

#[test]
fn a_bay_naming_fighters_is_a_secondary_launching_its_hull() {
    let game = loaded(&carrying());
    assert_eq!(problems(&game), Vec::<String>::new());
    let drone = game.ids.ships[&id("drone")];
    let bay = weapon_id(&game, "drone-bay");
    assert_eq!(
        weapon(&game, "drone-bay"),
        Weapon {
            reload: 60,
            count: 30,
            guidance: 99,
            ammo_type: drone,
            flags: Flags16(WEAPON_SECONDARY),
            speed: SHOT_SPEED,
            exit_type: -1,
            ..neutral_weapon()
        }
    );
    let own = outfit(&game, "drone-bay");
    assert_eq!((own.mod_type, own.mod_val), (MOD_WEAPON, bay));
    assert_eq!(
        own.flags,
        Flags16(HIDE_UNLESS_REQUIRED),
        "no gun or turret slot"
    );
    let carrier = ship(&game, "carrier");
    assert_eq!(carrier.max_gun, 0, "a bay takes no gun slot");
    assert_eq!(slot(&carrier, bay), (1, 2));
}

#[test]
fn a_bay_s_fighters_are_an_outfit_after_the_kits_named_for_the_hull_and_never_sold() {
    let game = loaded(&carrying());
    let kit = game.ids.kits[&id("gun-mount")];
    let nova = game.ids.fighters[&id("drone-bay")];
    assert_eq!(nova, kit + 1, "after the kits");
    assert_eq!(
        game.ids.fighters.len(),
        1,
        "one for the one bay that launches"
    );
    let entry = game
        .data
        .get::<Outfit>(nova)
        .expect("present")
        .expect("decoded");
    assert_eq!(entry.name, Some("Drone"));
    let fighters = entry.record.clone();
    assert_eq!(
        fighters,
        Outfit {
            mod_type: MOD_AMMO,
            mod_val: weapon_id(&game, "drone-bay"),
            ..named_outfit("Drone")
        }
    );
    assert_eq!(
        (
            fighters.cost,
            fighters.tech_level,
            fighters.buy_random,
            fighters.mass,
            fighters.max
        ),
        (0, OUTFIT_TECH_NEVER, 0, 0, i16::MAX)
    );
    assert_eq!(fighters.lc_name.as_str(), "drone");
}

#[test]
fn the_hull_a_bay_launches_is_a_fighter_and_no_other_hull_is() {
    let game = loaded(&carrying());
    assert_eq!(ship(&game, "drone").escort_type, FIGHTER_ESCORT);
    assert_eq!(FIGHTER_ESCORT, 0);
    for other in ["carrier", "skiff", "armed-skiff"] {
        assert_eq!(ship(&game, other).escort_type, -1, "{other}");
    }
}

#[test]
fn a_bay_naming_no_fighters_stays_an_outfit_that_does_nothing() {
    let game = loaded(&carrying().with("outfits/drone-bay.md", &bay("Drone bay", "")));
    assert!(!game.ids.weapons.contains_key(&id("drone-bay")));
    assert!(game.ids.fighters.is_empty());
    assert_eq!(ship(&game, "drone").escort_type, -1);
    assert_eq!(outfit(&game, "drone-bay").mod_type, -1);
}

#[test]
fn a_bay_whose_hull_is_missing_or_carries_a_bay_launches_nothing() {
    let game = loaded(&carrying().with(
        "outfits/drone-bay.md",
        &bay("Drone bay", "fighters: {hull: lost, load: 2}\n"),
    ));
    assert_eq!(
        against(&game, "outfits/drone-bay.md"),
        ["outfits/drone-bay.md: its fighters' hull `lost` is not in ships/; it launches nothing"]
    );
    assert!(!game.ids.weapons.contains_key(&id("drone-bay")));
    assert!(game.ids.fighters.is_empty());
    assert_eq!(outfit(&game, "drone-bay").mod_type, -1);

    let game = loaded(&carrying().with(
        "outfits/drone-bay.md",
        &bay("Drone bay", "fighters: {hull: carrier, load: 2}\n"),
    ));
    assert_eq!(
        against(&game, "outfits/drone-bay.md"),
        [
            "outfits/drone-bay.md: its fighters' hull `carrier` carries a bay of its own; \
             it launches nothing"
        ]
    );
    assert!(!game.ids.weapons.contains_key(&id("drone-bay")));
    assert_eq!(ship(&game, "carrier").escort_type, -1);
}

#[test]
fn a_hull_carrying_a_bay_through_its_kit_launches_nothing_either() {
    let game = loaded(
        &carrying()
            .with(
                "outfits/drone-bay.md",
                &bay("Drone bay", "fighters: {hull: armed-skiff, load: 2}\n"),
            )
            .with(
                "mods/gun-mount.md",
                &format!(
                    "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
                     hardpoints:\n{}---\n",
                    hardpoint("bed", "bay", "drone-bay")
                ),
            ),
    );
    assert_eq!(
        against(&game, "outfits/drone-bay.md"),
        [
            "outfits/drone-bay.md: its fighters' hull `armed-skiff` carries a bay of its own; \
             it launches nothing"
        ]
    );
}

#[test]
fn a_hull_left_out_for_its_chain_is_not_reported_again() {
    let game = loaded(&carrying().with("ships/drone.md", "---\nname: Drone\nbase: nowhere\n---\n"));
    assert_eq!(
        against(&game, "outfits/drone-bay.md"),
        Vec::<String>::new(),
        "{:?}",
        problems(&game)
    );
    assert!(!game.ids.weapons.contains_key(&id("drone-bay")));
}

#[test]
fn a_bay_past_the_last_weapon_id_gets_no_fighters() {
    let source = (0..255).fold(carrying(), |source, n| {
        source.with(
            &format!("outfits/a{n:03}.md"),
            "---\nname: A\nmount: fixed\ndamage: 1\nrange: 10\nreload: 1\nprice: 1\n---\n",
        )
    });
    let game = loaded(&source);
    assert!(
        problems(&game).contains(
            &"outfits/drone-bay.md: it is past the last wëap ID, 383; it is left out".to_owned()
        ),
        "{:?}",
        problems(&game)
    );
    assert!(game.ids.fighters.is_empty());
    assert_eq!(ship(&game, "drone").escort_type, -1);
}

#[test]
fn a_bay_and_a_launcher_on_one_hull_load_each_slot() {
    let game = loaded(
        &racked()
            .with(
                "outfits/drone-bay.md",
                &bay("Drone bay", "fighters: {hull: drone, load: 2}\n"),
            )
            .with(
                "ships/drone.md",
                "---\nname: Drone\nstyle: crown\nrole: fighter\nmodel: hull\n\
         size: {length: 4, beam: 2, collision_radius: 2}\n---\n",
            )
            .with(
                "ships/skiff.md",
                &skiff_with(&format!(
                    "{}{}{}",
                    hardpoint("left", "launcher", "rocket"),
                    hardpoint("hangar", "bay", "drone-bay"),
                    hardpoint("hold", "bay", "drone-bay"),
                )),
            ),
    );
    let skiff = ship(&game, "skiff");
    assert_eq!(slot(&skiff, weapon_id(&game, "rocket")), (1, 3));
    assert_eq!(slot(&skiff, weapon_id(&game, "drone-bay")), (2, 4));
    assert_eq!(
        Flags16(OUTFIT_GUN | HIDE_UNLESS_REQUIRED),
        outfit(&game, "rocket").flags,
        "the rack takes a gun slot"
    );
}
