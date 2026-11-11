//! Outfits as `oütf`s: something a pilot can own, so a mission can grant
//! and remove one ([`super::missions`]), and a weapon when it is one.
//!
//! Each outfit is numbered within [`OUTFIT_IDS`](super::ids::OUTFIT_IDS) ([`number_outfits`]) and
//! maps to an `oütf` ([`outfit`], over [`named_outfit`]) with the
//! outfit's `name`, as its `ShortName` and lower-cased as its `LCName`,
//! and its `price` as `Cost`. It weighs nothing and has no
//! limit on how many are owned.
//!
//! **Its mount.** Its `Require` is its `mount`'s bit
//! ([`FitBits::mount_bit`]) and its `Flags` hold
//! [`HIDE_UNLESS_REQUIRED`], so an outfitter neither lists nor sells it
//! for a ship with no hardpoint of that mount: a hull's chain hardpoints
//! and each owned kit's contribute their mounts' bits
//! ([`super::kits`]). A mission's grant and a kit's `OnPurchase` read no
//! `Require`, so they give it whatever the ship's mounts.
//!
//! **Where it is sold.** An outfit whose `sold_at` names a yard takes that
//! yard set's tech number as `TechLevel` and `BuyRandom` 100, always
//! offered, and each yard it names holds the number in a `SpecialTech`
//! slot ([`super::yards`]), so it is listed there and at no other yard.
//! One sold nowhere keeps the original's "never listed" `TechLevel`
//! ([`OUTFIT_TECH_NEVER`](super::records::OUTFIT_TECH_NEVER)) and
//! `BuyRandom` 0, so no outfitter lists it under either reading of the
//! tech level rule.
//!
//! **Weapons** ([`is_weapon`]). An outfit with `damage` above 0 and a
//! `fixed`, `turret` or `launcher` mount is a weapon, and so is a bay
//! naming `fighters` it can launch ([`launched_hull`]), whatever its
//! `damage`: it is numbered again within [`WEAPON_IDS`]
//! ([`number_weapons`]), in outfit id order and pinned by the outfit's
//! id, and maps to a `wëap` ([`weapon`]) as well. Its `oütf`'s first
//! `ModType` is [`MOD_WEAPON`] naming that `wëap`, and its `Flags`
//! [`OUTFIT_GUN`] (fixed, launcher) or [`OUTFIT_TURRET`] (turret), so it
//! takes a gun or a turret slot, on top of [`HIDE_UNLESS_REQUIRED`]; a
//! bay takes neither. The `wëap`, over [`neutral_weapon`]:
//!
//! - `MassDmg` and `EnergyDmg` are its `damage`;
//! - `Reload` is its `reload` seconds at [`TICKS_PER_SECOND`], rounded, at
//!   least 1;
//! - `Speed` is [`SHOT_SPEED`], 10 pixels a tick (a placeholder), and
//!   `Count`, the shot's life, its `range` over those 10 pixels, rounded
//!   up, at least 1: for a bay, the push its fighter is launched at and
//!   how long it glides;
//! - `Guidance` is -1, unguided, for a fixed gun or a launcher, 1, homing,
//!   for a launcher naming `guidance: homing`, with a `GuidedTurn` of
//!   [`HOMING_TURN`] (a placeholder: a homing shot of none never steers),
//!   4, turreted and unguided, for a turret, and 99, a fighter bay, for a
//!   bay; `ExitType` 0, 1 or 2 by the mount ([`exit_type`]), so a shot
//!   leaves from its kind of exit point, and -1, the centre, for a bay;
//! - a launcher firing an `ammunition` and a bay launching a hull take the
//!   `AmmoType` [`super::munitions`] works out and `Flags`
//!   [`WEAPON_SECONDARY`], on the second trigger, as every stock missile
//!   launcher and bay is; any other weapon uses no ammunition (`AmmoType`
//!   -1), on the first trigger;
//! - `MaxAmmo` is 0, no limit of its own;
//! - it draws as the view's placeholder box (its `Graphic` 0 names a
//!   `spïn` the new game never holds), plays no sound and explodes into
//!   nothing.
//!
//! **Ammunition.** An outfit a launcher's `ammunition` names, whose
//! rounds resolve ([`super::munitions`]), keeps its name, price, mount
//! and sale, and its first `ModType` is [`MOD_AMMO`] naming its rounds'
//! `wëap` ([`Fitted::Rounds`]). A bay's fighters are an `oütf` of their
//! own ([`super::munitions::fighters`]).
//!
//! A `bay` naming no `fighters`, like an outfit doing no damage, stays an
//! `oütf` that does nothing. The outfit's `model` isn't part of its
//! records: it is the model its hardpoints show and the weapon's muzzle
//! reach, kept on the store ([`super::mounts`]).
//!
//! A weapon past the last `wëap` ID is reported and stays an `oütf` that
//! does nothing.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use nova_galaxy_format::fleet::{Guidance, Mount, Outfit as BibleOutfit};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};

use super::ids::{WEAPON_IDS, after_pins};
use super::kits::{ALWAYS, FitBits, HIDE_UNLESS_REQUIRED};
use super::map::number;
use super::munitions::launched_hull;
use super::records::{named_outfit, neutral_weapon};
use crate::decode::Entry;
use crate::records::outfit::Outfit;
use crate::records::weapon::Weapon;
use crate::registry::AnyRecord;
use crate::wire::flags::{Flags16, Flags64};

/// The `oütf` `ModType` of a weapon: its `ModVal` names the `wëap`
/// (`nova_sim::combat::armament::MOD_WEAPON`).
pub const MOD_WEAPON: i16 = 1;

/// The `oütf` `Flags` bit of a fixed gun, which takes a gun slot
/// (`nova_sim::outfitter::OutfitFlags::GUN`).
pub const OUTFIT_GUN: u16 = 0x0001;

/// The `oütf` `Flags` bit of a turret, which takes a turret slot
/// (`nova_sim::outfitter::OutfitFlags::TURRET`).
pub const OUTFIT_TURRET: u16 = 0x0002;

/// The `oütf` `ModType` of ammunition: its `ModVal` names the `wëap`
/// whose rounds it is (`nova_sim::combat::armament::MOD_AMMO`).
pub const MOD_AMMO: i16 = 3;

/// The `wëap` `Flags` bit of a weapon on the second trigger
/// (`nova_sim::combat::weapon::SECONDARY`): a launcher firing ammunition,
/// as every stock missile launcher is, and a fighter bay.
pub const WEAPON_SECONDARY: u16 = 0x0002;

/// Every weapon's shot speed: 1000, ten pixels a tick (a placeholder).
pub const SHOT_SPEED: i16 = 1000;

/// A homing launcher's `GuidedTurn`: 70, seven degrees a tick (a
/// placeholder, the stock IR missile's). A homing shot of no `GuidedTurn`
/// never steers.
pub const HOMING_TURN: i16 = 70;

/// The `Guidance` of a homing shot.
const HOMING: i16 = 1;

/// The `Guidance` of a fighter bay, whose `AmmoType` is the `shïp` it
/// launches.
const FIGHTER_BAY: i16 = 99;

/// The pixels a shot at [`SHOT_SPEED`] flies a tick.
const PIXELS_PER_TICK: f64 = 10.0;

/// The ticks in a second: a weapon's `reload` in seconds is its `Reload`
/// in ticks at this rate.
pub const TICKS_PER_SECOND: f64 = 30.0;

/// The `Guidance` of a turret that fires unguided shots.
const TURRETED: i16 = 4;

/// Numbers the outfits of `galaxy` within
/// [`OUTFIT_IDS`](super::ids::OUTFIT_IDS): each `pins`
/// names at its pin, and each other from `free`, which the kits number on
/// from after.
pub fn number_outfits(
    galaxy: &Galaxy,
    pins: &BTreeMap<Id, i16>,
    free: &mut RangeInclusive<i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    number(&galaxy.outfits, pins, free, "oütf", problems)
}

/// Whether `outfit` is a weapon: it does damage from a `fixed`, `turret`
/// or `launcher` mount.
#[must_use]
pub fn is_weapon(outfit: &BibleOutfit) -> bool {
    outfit.damage > 0 && matches!(outfit.mount, Mount::Fixed | Mount::Turret | Mount::Launcher)
}

/// Numbers the outfits of `galaxy` that are weapons within [`WEAPON_IDS`],
/// in id order, each `pins` names (by its outfit's id) at its pin and the
/// others after the highest pin: each [`is_weapon`], and each bay that
/// launches a hull of `ships` ([`launched_hull`]).
pub fn number_weapons(
    galaxy: &Galaxy,
    ships: &BTreeMap<Id, i16>,
    pins: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let weapons = galaxy.outfits.iter().filter(|(_, doc)| {
        is_weapon(&doc.data) || launched_hull(galaxy, ships, &doc.data).is_some()
    });
    let mut free = after_pins(WEAPON_IDS, pins.values());
    number(weapons, pins, &mut free, "wëap", problems)
}

/// The `ExitType` a shot from a hardpoint of `mount` leaves from: 0 the
/// gun points for `fixed`, 1 the turret points for `turret`, 2 the guided
/// points for `launcher`; none for a `bay`.
#[must_use]
pub fn exit_type(mount: Mount) -> Option<i16> {
    match mount {
        Mount::Fixed => Some(0),
        Mount::Turret => Some(1),
        Mount::Launcher => Some(2),
        Mount::Bay => None,
    }
}

/// `value` as a record's word, one past `i16` the most.
fn word(value: u32) -> i16 {
    i16::try_from(value).unwrap_or(i16::MAX)
}

/// `value`, a whole number of ticks or pixels worked in doubles, as a
/// record's word, at least 1 (a value that is not a number is 1).
fn at_least_one(value: f64) -> i16 {
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let value = value.clamp(1.0, f64::from(i16::MAX)) as i16;
    value.max(1)
}

/// What an outfit's first mod is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fitted {
    /// None: it does nothing.
    Nothing,
    /// A weapon, of this `wëap`.
    Weapon(i16),
    /// Ammunition, the rounds of this `wëap` ([`super::munitions`]).
    Rounds(i16),
}

/// The `oütf` of the outfit `doc`, numbered `nova`, its first mod
/// `fitted`, sold at the yards holding the tech number `sold` when it is
/// sold (see the module docs).
#[must_use]
pub fn outfit(
    nova: i16,
    doc: &Doc<BibleOutfit>,
    fitted: Fitted,
    sold: Option<i16>,
) -> Entry<AnyRecord> {
    let mut outfit = Outfit {
        cost: i32::try_from(doc.data.price).unwrap_or(i32::MAX),
        flags: Flags16(HIDE_UNLESS_REQUIRED),
        require: Flags64(FitBits::mount_bit(doc.data.mount)),
        ..named_outfit(&doc.data.name)
    };
    if let Some(sold) = sold {
        outfit.tech_level = sold;
        outfit.buy_random = ALWAYS;
    }
    match fitted {
        Fitted::Nothing => {}
        Fitted::Weapon(weapon) => {
            outfit.mod_type = MOD_WEAPON;
            outfit.mod_val = weapon;
            let slot = match doc.data.mount {
                Mount::Turret => OUTFIT_TURRET,
                Mount::Bay => 0,
                Mount::Fixed | Mount::Launcher => OUTFIT_GUN,
            };
            outfit.flags = Flags16(HIDE_UNLESS_REQUIRED | slot);
        }
        Fitted::Rounds(weapon) => {
            outfit.mod_type = MOD_AMMO;
            outfit.mod_val = weapon;
        }
    }
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::Outfit(Box::new(outfit)),
    }
}

/// The `wëap` of the weapon outfit `doc`, numbered `nova`, of the
/// `AmmoType` `ammo` when it fires rounds ([`super::munitions`]; see the
/// module docs).
#[must_use]
pub fn weapon(nova: i16, doc: &Doc<BibleOutfit>, ammo: Option<i16>) -> Entry<AnyRecord> {
    let data = &doc.data;
    let damage = word(data.damage);
    let (guidance, guided_turn) = match data.mount {
        Mount::Turret => (TURRETED, 0),
        Mount::Bay => (FIGHTER_BAY, 0),
        Mount::Launcher if data.guidance == Guidance::Homing => (HOMING, HOMING_TURN),
        Mount::Fixed | Mount::Launcher => (-1, 0),
    };
    let weapon = Weapon {
        reload: at_least_one((data.reload * TICKS_PER_SECOND).round()),
        count: at_least_one((f64::from(data.range) / PIXELS_PER_TICK).ceil()),
        mass_dmg: damage,
        energy_dmg: damage,
        guidance,
        guided_turn,
        ammo_type: ammo.unwrap_or(-1),
        flags: Flags16(if ammo.is_some() { WEAPON_SECONDARY } else { 0 }),
        speed: SHOT_SPEED,
        exit_type: exit_type(data.mount).unwrap_or(-1),
        ..neutral_weapon()
    };
    Entry {
        id: nova,
        name: Some(data.name.clone()),
        record: AnyRecord::Weapon(Box::new(weapon)),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::super::kits::{FitBits, HIDE_UNLESS_REQUIRED};
    use super::super::records::OUTFIT_TECH_NEVER;
    use super::super::tests::galaxy;
    use super::super::{NewGame, load};
    use super::*;
    use crate::decode::Record;

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("games"))
    }

    fn named(game: &NewGame, nova: i16) -> (String, Outfit) {
        let entry = (game.data.get::<Outfit>(nova))
            .expect("present")
            .expect("decoded");
        (entry.name.expect("named").to_owned(), entry.record.clone())
    }

    fn weapon_of(game: &NewGame, nova: i16) -> (String, Weapon) {
        let entry = (game.data.get::<Weapon>(nova))
            .expect("present")
            .expect("decoded");
        (entry.name.expect("named").to_owned(), entry.record.clone())
    }

    fn pass(name: &str, price: u32) -> String {
        format!(
            "---\nname: {name}\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\nprice: {price}\n---\n"
        )
    }

    /// An outfit `name` on a `mount` mount of `damage`, `range` and
    /// `reload`.
    fn gun(name: &str, mount: &str, damage: u32, range: u32, reload: f64) -> String {
        format!(
            "---\nname: {name}\nmount: {mount}\ndamage: {damage}\nrange: {range}\n\
             reload: {reload}\nprice: 2500\n---\n"
        )
    }

    /// The fixture galaxy with its blaster doing no damage.
    fn harmless() -> MemSource {
        galaxy().with("outfits/blaster.md", &pass("Blaster", 100))
    }

    #[test]
    fn an_outfit_maps_at_128_with_its_name_and_price() {
        let game = loaded(&galaxy());
        assert_eq!(
            game.ids.outfits.iter().collect::<Vec<_>>(),
            [(&Id::try_from("blaster".to_owned()).unwrap(), &128)],
            "the blaster alone, the gun mount after it"
        );
        let (name, outfit) = named(&game, 128);
        assert_eq!(name, "Blaster");
        assert_eq!(outfit.cost, 100);
    }

    #[test]
    fn an_outfit_s_short_name_is_its_name_and_its_lower_case_name_lower_cased() {
        let (_, outfit) = named(&loaded(&galaxy()), 128);
        assert_eq!(
            (
                outfit.short_name.as_str(),
                outfit.lc_name.as_str(),
                outfit.lc_plural.as_str()
            ),
            ("Blaster", "blaster", ""),
            "no plural derives"
        );
    }

    #[test]
    fn an_outfit_weighs_nothing_does_nothing_has_no_limit_and_is_never_sold() {
        let (_, outfit) = named(&loaded(&harmless()), 128);
        assert_eq!(outfit.mass, 0);
        assert_eq!(outfit.max, i16::MAX);
        assert_eq!(outfit.tech_level, OUTFIT_TECH_NEVER);
        assert_eq!(OUTFIT_TECH_NEVER, 0x7fff);
        let slots = [
            (outfit.mod_type, outfit.mod_val),
            (outfit.mod_type2, outfit.mod_val2),
            (outfit.mod_type3, outfit.mod_val3),
            (outfit.mod_type4, outfit.mod_val4),
        ];
        assert_eq!(slots, [(-1, 0); 4]);
        assert_eq!(outfit.buy_random, 0);
        assert_eq!(outfit.require_govt, -1);
        assert_eq!(outfit.flags, Flags16(HIDE_UNLESS_REQUIRED));
    }

    #[test]
    fn an_outfit_requires_its_mount_s_bit_and_hides_unless_the_ship_has_one() {
        for (mount, damage) in [
            (Mount::Fixed, 5),
            (Mount::Turret, 5),
            (Mount::Launcher, 5),
            (Mount::Bay, 5),
            (Mount::Fixed, 0),
        ] {
            let text = gun("Blaster", mount.as_str(), damage, 300, 0.5);
            let (_, outfit) = named(&loaded(&galaxy().with("outfits/blaster.md", &text)), 128);
            assert_eq!(outfit.require.bits(), FitBits::mount_bit(mount), "{text}");
            assert_ne!(
                outfit.flags.bits() & HIDE_UNLESS_REQUIRED,
                0,
                "{text}: hidden on a hull without the mount"
            );
        }
    }

    #[test]
    fn two_outfits_number_in_id_order() {
        let game = loaded(&galaxy().with("outfits/anchor.md", &pass("Anchor", 7)));
        assert_eq!(
            game.ids.outfits.values().copied().collect::<Vec<_>>(),
            [128, 129]
        );
        assert_eq!(named(&game, 128).0, "Anchor");
        assert_eq!(named(&game, 129).0, "Blaster");
    }

    #[test]
    fn an_outfit_past_the_last_outfit_id_is_a_problem_and_left_out() {
        let source = (0..512).fold(harmless(), |source, n| {
            source.with(&format!("outfits/a{n:03}.md"), &pass("A", 0))
        });
        let game = loaded(&source);
        let lines: Vec<_> = game.problems.iter().map(ToString::to_string).collect();
        assert_eq!(
            lines,
            [
                "outfits/blaster.md: it is past the last oütf ID, 639; it is left out",
                "mods/gun-mount.md: it is past the last oütf ID, 639; it is left out",
            ]
        );
        assert_eq!(game.data.ids(Outfit::TYPE).len(), 512);
        assert!(
            !game
                .ids
                .outfits
                .contains_key(&Id::try_from("blaster".to_owned()).unwrap())
        );
    }

    #[test]
    fn a_price_past_i32_is_the_most() {
        let game = loaded(&galaxy().with("outfits/blaster.md", &pass("Blaster", u32::MAX)));
        assert_eq!(named(&game, 128).1.cost, i32::MAX);
    }

    #[test]
    fn a_turret_outfit_is_a_weapon_at_128() {
        let game = loaded(&galaxy().with(
            "outfits/blaster.md",
            &gun("Mass driver", "turret", 12, 600, 0.5),
        ));
        assert_eq!(game.data.ids(Weapon::TYPE), [128]);
        assert_eq!(
            game.ids.weapons.iter().collect::<Vec<_>>(),
            [(&Id::try_from("blaster".to_owned()).unwrap(), &128)]
        );
        let (name, weapon) = weapon_of(&game, 128);
        assert_eq!(name, "Mass driver");
        let expected = Weapon {
            reload: 15,
            count: 60,
            mass_dmg: 12,
            energy_dmg: 12,
            guidance: 4,
            speed: SHOT_SPEED,
            exit_type: 1,
            ..neutral_weapon()
        };
        assert_eq!(
            weapon, expected,
            "turreted, from a turret, drawn as the box"
        );
        assert_eq!(
            weapon.graphic, 0,
            "a shot spawns only for a graphic of 0-255"
        );
        assert_eq!(SHOT_SPEED, 1000);
        let (_, outfit) = named(&game, 128);
        assert_eq!((outfit.mod_type, outfit.mod_val), (MOD_WEAPON, 128));
        assert_eq!(outfit.flags, Flags16(OUTFIT_TURRET | HIDE_UNLESS_REQUIRED));
        assert_eq!(outfit.tech_level, OUTFIT_TECH_NEVER, "still never sold");
        assert_eq!(outfit.mass, 0);
        assert_eq!(outfit.buy_random, 0);
    }

    /// A guard for a launcher naming neither `guidance` nor `ammunition`:
    /// it maps whole as it did before launchers read either, so it passes
    /// before and after they do.
    #[test]
    fn a_fixed_gun_or_a_launcher_is_a_gun_and_unguided() {
        for (fits, exit) in [(Mount::Fixed, 0), (Mount::Launcher, 2)] {
            let mount = fits.as_str();
            let game =
                loaded(&galaxy().with("outfits/blaster.md", &gun("Blaster", mount, 5, 300, 0.5)));
            let (_, weapon) = weapon_of(&game, 128);
            assert_eq!(
                weapon,
                Weapon {
                    reload: 15,
                    count: 30,
                    mass_dmg: 5,
                    energy_dmg: 5,
                    guidance: -1,
                    speed: SHOT_SPEED,
                    exit_type: exit,
                    ..neutral_weapon()
                },
                "{mount}: unguided, of no ammunition, no turn and no flags"
            );
            let expected = Outfit {
                cost: 2500,
                flags: Flags16(OUTFIT_GUN | HIDE_UNLESS_REQUIRED),
                require: Flags64(FitBits::mount_bit(fits)),
                mod_type: MOD_WEAPON,
                mod_val: 128,
                ..named_outfit("Blaster")
            };
            assert_eq!(named(&game, 128).1, expected, "{mount}");
            let skiff = game
                .data
                .get::<crate::records::ship::Ship>(
                    game.ids.ships[&Id::try_from("skiff".to_owned()).unwrap()],
                )
                .expect("present")
                .expect("decoded")
                .record
                .clone();
            assert_eq!(skiff.weap_type1_4[0], Some(crate::wire::id::WeaponId(128)));
            assert_eq!(
                (skiff.ammo_load1_4, skiff.ammo_load5_8),
                ([0; 4], [0; 4]),
                "{mount}: its hull carries no rounds"
            );
            assert_eq!(skiff.escort_type, -1, "{mount}");
        }
    }

    #[test]
    fn a_reload_rounds_to_ticks_and_a_range_rounds_up_to_ticks_each_at_least_one() {
        let game = loaded(&galaxy().with("outfits/blaster.md", &gun("B", "fixed", 5, 601, 0.51)));
        let (_, weapon) = weapon_of(&game, 128);
        assert_eq!((weapon.reload, weapon.count), (15, 61));
        let game = loaded(&galaxy().with("outfits/blaster.md", &gun("B", "fixed", 5, 0, 0.0)));
        let (_, weapon) = weapon_of(&game, 128);
        assert_eq!((weapon.reload, weapon.count), (1, 1));
        let game = loaded(&galaxy().with("outfits/blaster.md", &gun("B", "fixed", 5, 9, 0.02)));
        let (_, weapon) = weapon_of(&game, 128);
        assert_eq!((weapon.reload, weapon.count), (1, 1));
    }

    #[test]
    fn a_weapon_of_no_exit_mount_leaves_from_the_centre() {
        let (galaxy, _) = nova_galaxy_format::load::load(
            &galaxy().with("outfits/blaster.md", &gun("Bay", "bay", 30, 900, 2.0)),
        );
        let doc = &galaxy.outfits[&Id::try_from("blaster".to_owned()).unwrap()];
        let AnyRecord::Weapon(weapon) = weapon(128, doc, None).record else {
            panic!("a wëap");
        };
        assert_eq!(weapon.exit_type, -1);
    }

    #[test]
    fn a_damage_past_i16_is_the_most() {
        let game =
            loaded(&galaxy().with("outfits/blaster.md", &gun("B", "fixed", u32::MAX, 300, 0.5)));
        let (_, weapon) = weapon_of(&game, 128);
        assert_eq!((weapon.mass_dmg, weapon.energy_dmg), (i16::MAX, i16::MAX));
    }

    /// A guard for a bay naming no `fighters`: it stays an `oütf` that does
    /// nothing, as before bays launched anything, so it passes before and
    /// after they do.
    #[test]
    fn an_outfit_of_no_damage_or_a_bay_is_no_weapon() {
        for text in [pass("Blaster", 100), gun("Bay", "bay", 30, 900, 2.0)] {
            let game = loaded(&galaxy().with("outfits/blaster.md", &text));
            assert_eq!(game.data.ids(Weapon::TYPE), Vec::<i16>::new(), "{text}");
            assert!(game.ids.weapons.is_empty());
            let (_, outfit) = named(&game, 128);
            assert_eq!(
                (outfit.mod_type, outfit.flags.bits()),
                (-1, HIDE_UNLESS_REQUIRED),
                "{text}"
            );
        }
    }

    #[test]
    fn weapons_number_in_outfit_id_order_skipping_what_is_no_weapon() {
        let game = loaded(
            &galaxy()
                .with("outfits/anchor.md", &pass("Anchor", 7))
                .with("outfits/cannon.md", &gun("Cannon", "fixed", 9, 300, 1.0)),
        );
        let weapons: Vec<_> = game
            .ids
            .weapons
            .iter()
            .map(|(id, &n)| (id.to_string(), n))
            .collect();
        assert_eq!(
            weapons,
            [("blaster".to_owned(), 128), ("cannon".to_owned(), 129)]
        );
        assert_eq!(
            named(&game, 130).1.mod_val,
            129,
            "the cannon's oütf names its wëap"
        );
    }

    #[test]
    fn a_weapon_past_the_last_weapon_id_is_a_problem_and_does_nothing() {
        let source = (0..256).fold(galaxy(), |source, n| {
            source.with(
                &format!("outfits/a{n:03}.md"),
                &gun("A", "fixed", 1, 10, 1.0),
            )
        });
        let game = loaded(&source);
        let past: Vec<_> = game
            .problems
            .iter()
            .map(ToString::to_string)
            .filter(|line| line.contains("wëap"))
            .collect();
        assert_eq!(
            past,
            ["outfits/blaster.md: it is past the last wëap ID, 383; it is left out"]
        );
        let blaster = game.ids.outfits[&Id::try_from("blaster".to_owned()).unwrap()];
        assert_eq!(named(&game, blaster).1.mod_type, -1);
    }
}
