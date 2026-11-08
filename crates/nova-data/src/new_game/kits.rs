//! Mod kits as `oütf`s sold at the outfitter, with the hull roles each
//! fits as `Require` bits and the yards that fit it as tech levels.
//!
//! The bible's yard is a stellar with the `outfitter` service: Nova's
//! outfitter already lists, prices, limits, requires, slots, buys, sells
//! and refits, so a kit is an outfit there. The `shipyard` sells hulls
//! only.
//!
//! - **Numbering** ([`number_kits`]). The kits share
//!   [`OUTFIT_IDS`] with the outfits, numbered after the last outfit's ID
//!   in kit id order, so adding a kit renumbers no outfit.
//! - **The `oütf`** ([`kit`], over [`neutral_outfit`]): its `name`;
//!   `Cost` its `price`; massless; `Max` 1, fitted once; `BuyRandom` 100,
//!   for sale every day; `Flags` [`HIDE_UNLESS_REQUIRED`], so a kit that
//!   doesn't fit the hull isn't listed; and not persistent, so it goes
//!   with the hull when the ship changes.
//! - **Effects** ([`effect_slots`]), in the four `ModType`/`ModVal` slots,
//!   in this order, an effect of 0 taking none: `hull` as
//!   [`mod_type::MORE_ARMOR`], `shields` as [`mod_type::MORE_SHIELD`],
//!   `speed` as [`mod_type::MORE_SPEED`] (top speed only: acceleration
//!   stays the hull's placeholder), `handling` as
//!   [`mod_type::TURN_CHANGE`] on the hull's turn scale ([`turn_change`]),
//!   `cargo` as [`mod_type::MORE_CARGO`], then its `fixed` and `launcher`
//!   hardpoints as [`mod_type::MORE_GUNS`] and its `turret` ones as
//!   [`mod_type::MORE_TURRETS`]. `hidden_cargo` isn't mapped: nothing in
//!   the new game is contraband cargo yet.
//! - **Its hardpoints' outfits** come with it: its `OnPurchase` grants
//!   each (`G<oütf>`) and its `OnSell` takes each back (`D<oütf>`), so a
//!   gun mount's gun arrives in the turret slot the kit adds and leaves
//!   when it is sold.
//! - **Which hulls** ([`FitBits`]). Every distinct `fits` set, its roles
//!   sorted, gets a `Require` bit, numbered in kit id order. Each ship
//!   class `Contribute`s the bit of every set holding its role
//!   ([`FitBits::contributed`]), and each kit `Require`s its set's.
//! - **Which yards** (`TechLevel`): a legal kit [`KIT_TECH_LEGAL`], a
//!   lawless one [`KIT_TECH_LAWLESS`]. A law-keeping stellar has
//!   [`YARD_TECH_LEGAL`] and a lawless or unowned one
//!   [`YARD_TECH_LAWLESS`] (`super::map`), so a legal yard fits legal kits
//!   and a lawless yard both.
//! - **The law** (`ScanMask`): a lawless kit carries [`LAWLESS_KIT_SCAN`],
//!   the bit a law-keeping government's scan looks for (`super::map`); a
//!   legal one none.
//! - **Fittings** ([`kit_fittings`]): each kit's `attention` and its
//!   hardpoints' places, on the store's side table.
//!
//! # Problems
//!
//! - A kit needing more than four slots keeps the first four and names
//!   the rest.
//! - A hardpoint's `outfit` naming no outfit is left out of the scripts.
//! - Past 64 distinct `fits` sets, a kit of a later set is left out.
//! - A kit past the last `oütf` ID is left out.

use std::collections::BTreeMap;

use nova_galaxy_format::fleet::{ModKit, Mount, Role, Yard};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};

use super::fleet::{HANDLING_PER_TURN, spot};
use super::ids::OUTFIT_IDS;
use super::map::absent;
use super::records::neutral_outfit;
use crate::decode::Entry;
use crate::records::outfit::Outfit;
use crate::registry::AnyRecord;
use crate::store::Kit;
use crate::wire::flags::{Flags16, Flags64};
use crate::wire::string::MacString;

/// The `oütf` `ModType`s a kit's effects take, as the engine's ship stats
/// and outfitter read them (`nova_sim::stats` and `nova_sim::outfitter`).
pub mod mod_type {
    /// Adds cargo space, in tons.
    pub const MORE_CARGO: i16 = 2;
    /// Adds shield points.
    pub const MORE_SHIELD: i16 = 4;
    /// Adds armour points.
    pub const MORE_ARMOR: i16 = 6;
    /// Adds top speed, on `Speed`'s scale.
    pub const MORE_SPEED: i16 = 8;
    /// Changes the turn rate, [`super::TURN_MOD_PER_MANEUVER`] a unit of
    /// `Maneuver`.
    pub const TURN_CHANGE: i16 = 9;
    /// Changes `MaxGun`.
    pub const MORE_GUNS: i16 = 45;
    /// Changes `MaxTur`.
    pub const MORE_TURRETS: i16 = 46;
}

/// A turn-change `ModVal` per unit of `Maneuver` (the engine's
/// `nova_sim::stats::TURN_MOD_PER_MANEUVER`).
pub const TURN_MOD_PER_MANEUVER: i32 = 10;

/// The `oütf` `Flags` bit of an outfit not listed unless the player meets
/// its `Require` (`nova_sim::outfitter::OutfitFlags::HIDE_UNLESS_REQUIRED`).
pub const HIDE_UNLESS_REQUIRED: u16 = 0x0100;

/// A legal kit's `TechLevel`: any yard lists it.
pub const KIT_TECH_LEGAL: i16 = 1;
/// A lawless kit's `TechLevel`: only a lawless yard lists it.
pub const KIT_TECH_LAWLESS: i16 = 2;
/// A law-keeping stellar's `TechLevel`: it lists legal kits.
pub const YARD_TECH_LEGAL: i16 = KIT_TECH_LEGAL;
/// A lawless or unowned stellar's `TechLevel`: it lists every kit.
pub const YARD_TECH_LAWLESS: i16 = KIT_TECH_LAWLESS;

/// The `ScanMask` bit of a lawless kit, which a law-keeping government's
/// scan looks for.
pub const LAWLESS_KIT_SCAN: u16 = 0x0001;

/// A law-keeping government's `SmugPenalty`: what a smuggling conviction
/// costs the player's record with it, and what makes its traffic control
/// scan (a placeholder until the bible gives the law's numbers).
pub const LAW_SMUGGLE_PENALTY: i16 = 10;

/// A law-keeping government's `ScanFine`: the credits a find costs (a
/// placeholder until the bible gives the law's numbers).
pub const LAW_SCAN_FINE: i16 = 500;

/// Every kit's `BuyRandom`: always for sale.
const ALWAYS: i16 = 100;

/// How many `ModType` slots an `oütf` has.
const SLOTS: usize = 4;

/// The `Require` bits of the kits' `fits` sets.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FitBits {
    /// Each set, its roles sorted, and its bit.
    sets: Vec<(Vec<Role>, u64)>,
    /// Each kit given a bit, and its set's bit.
    kits: BTreeMap<Id, u64>,
}

impl FitBits {
    /// Gives each distinct `fits` set of `galaxy`'s kits a bit, in kit id
    /// order; a kit of a set past the 64th is reported and left out.
    pub fn of(galaxy: &Galaxy, problems: &mut Vec<Problem>) -> Self {
        let mut bits = Self::default();
        for (id, doc) in &galaxy.mods {
            let mut set = doc.data.fits.clone();
            set.sort_unstable();
            set.dedup();
            let bit = match bits.sets.iter().find(|(known, _)| *known == set) {
                Some(&(_, bit)) => bit,
                None if bits.sets.len() < 64 => {
                    let bit = 1u64 << bits.sets.len();
                    bits.sets.push((set, bit));
                    bit
                }
                None => {
                    problems.push(Problem::new(
                        &doc.path,
                        "its `fits` is past the 64th distinct set of roles; it is left out",
                    ));
                    continue;
                }
            };
            bits.kits.insert(id.clone(), bit);
        }
        bits
    }

    /// The bit kit `id` requires, if it was given one.
    #[must_use]
    pub fn required(&self, id: &Id) -> Option<u64> {
        self.kits.get(id).copied()
    }

    /// The bits a ship class of `role` contributes: every set holding it.
    #[must_use]
    pub fn contributed(&self, role: Option<Role>) -> u64 {
        let Some(role) = role else { return 0 };
        self.sets
            .iter()
            .filter(|(set, _)| set.contains(&role))
            .fold(0, |bits, &(_, bit)| bits | bit)
    }
}

/// Numbers the kits of `galaxy` that have a fit bit in `fit`, in id
/// order, within [`OUTFIT_IDS`] after `outfits`' last ID (128 with none),
/// reporting each past its end.
pub fn number_kits(
    galaxy: &Galaxy,
    outfits: &BTreeMap<Id, i16>,
    fit: &FitBits,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let first = outfits
        .values()
        .max()
        .map_or(*OUTFIT_IDS.start(), |last| last.saturating_add(1));
    let last = *OUTFIT_IDS.end();
    let mut next = first..=last;
    let mut kits = BTreeMap::new();
    for (id, doc) in &galaxy.mods {
        if fit.required(id).is_none() {
            continue;
        }
        match next.next() {
            Some(nova) => {
                kits.insert(id.clone(), nova);
            }
            None => problems.push(Problem::new(
                &doc.path,
                format!("it is past the last oütf ID, {last}; it is left out"),
            )),
        }
    }
    kits
}

/// `handling` on the hull's turn scale as a turn-change `ModVal`: × 10
/// over 4, rounded to the nearest, halves away from 0, held within `i16`.
#[must_use]
pub fn turn_change(handling: i32) -> i16 {
    let value = (f64::from(handling) * f64::from(TURN_MOD_PER_MANEUVER)
        / f64::from(HANDLING_PER_TURN))
    .round();
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let value = value.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
    value
}

/// `value` held within `i16`.
fn word(value: i64) -> i16 {
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let value = value.clamp(i16::MIN.into(), i16::MAX.into()) as i16;
    value
}

/// The `ModType`/`ModVal` pairs `kit` needs, in slot order, each with the
/// name of what it maps (see the module docs).
#[must_use]
pub fn effect_slots(kit: &ModKit) -> Vec<(&'static str, i16, i16)> {
    let e = kit.effects;
    let count = |mounts: &[Mount]| {
        i64::try_from(
            kit.hardpoints
                .iter()
                .filter(|h| mounts.contains(&h.mount))
                .count(),
        )
        .unwrap_or(i64::MAX)
    };
    [
        ("hull", mod_type::MORE_ARMOR, word(e.hull.into())),
        ("shields", mod_type::MORE_SHIELD, word(e.shields.into())),
        ("speed", mod_type::MORE_SPEED, word(e.speed.into())),
        ("handling", mod_type::TURN_CHANGE, turn_change(e.handling)),
        ("cargo", mod_type::MORE_CARGO, word(e.cargo.into())),
        (
            "gun hardpoints",
            mod_type::MORE_GUNS,
            word(count(&[Mount::Fixed, Mount::Launcher])),
        ),
        (
            "turret hardpoints",
            mod_type::MORE_TURRETS,
            word(count(&[Mount::Turret])),
        ),
    ]
    .into_iter()
    .filter(|&(_, _, value)| value != 0)
    .collect()
}

/// The `oütf` of the kit `doc`, numbered `nova`, requiring `require`, its
/// hardpoints' outfits given and taken back by their `oütf`s in `outfits`
/// (see the module docs).
#[must_use]
pub fn kit(
    galaxy: &Galaxy,
    outfits: &BTreeMap<Id, i16>,
    nova: i16,
    doc: &Doc<ModKit>,
    require: u64,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let data = &doc.data;
    let slots = effect_slots(data);
    if slots.len() > SLOTS {
        let left: Vec<String> = slots[SLOTS..]
            .iter()
            .map(|(name, _, _)| format!("`{name}`"))
            .collect();
        problems.push(Problem::new(
            &doc.path,
            format!(
                "it needs more than four effect slots; {} left out",
                left.join(", ")
            ),
        ));
    }
    let mut pairs = [(-1, 0); SLOTS];
    for (pair, &(_, mod_type, value)) in pairs.iter_mut().zip(&slots) {
        *pair = (mod_type, value);
    }
    let mut given = Vec::new();
    for hardpoint in &data.hardpoints {
        let Some(outfit) = &hardpoint.outfit else {
            continue;
        };
        match outfits.get(outfit) {
            Some(&id) => given.push(id),
            None if absent(galaxy, &galaxy.outfits, "outfits", outfit) => {
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its hardpoint `{}`'s outfit `{outfit}` is not in outfits/; it isn't given with the kit",
                        hardpoint.id
                    ),
                ));
            }
            None => {}
        }
    }
    let script = |op: char| {
        let ops: Vec<String> = given.iter().map(|id| format!("{op}{id}")).collect();
        MacString::from(ops.join(" ").as_str())
    };
    let lawless = data.fitted_at == Yard::Lawless;
    let outfit = Outfit {
        cost: i32::try_from(data.price).unwrap_or(i32::MAX),
        max: 1,
        buy_random: ALWAYS,
        flags: Flags16(HIDE_UNLESS_REQUIRED),
        tech_level: if lawless {
            KIT_TECH_LAWLESS
        } else {
            KIT_TECH_LEGAL
        },
        scan_mask: Flags16(if lawless { LAWLESS_KIT_SCAN } else { 0 }),
        require: Flags64(require),
        mod_type: pairs[0].0,
        mod_val: pairs[0].1,
        mod_type2: pairs[1].0,
        mod_val2: pairs[1].1,
        mod_type3: pairs[2].0,
        mod_val3: pairs[2].1,
        mod_type4: pairs[3].0,
        mod_val4: pairs[3].1,
        on_purchase: script('G'),
        on_sell: script('D'),
        ..neutral_outfit()
    };
    Entry {
        id: nova,
        name: Some(data.name.clone()),
        record: AnyRecord::Outfit(Box::new(outfit)),
    }
}

/// The kit `kit`'s fittings: its attention and its hardpoints' places
/// ([`spot`]), a bay's none.
#[must_use]
pub fn kit_fittings(kit: &ModKit) -> Kit {
    Kit {
        attention: kit.effects.attention,
        spots: kit.hardpoints.iter().filter_map(spot).collect(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use nova_galaxy_format::source::MemSource;

    use super::super::tests::galaxy;
    use super::super::{NewGame, load};
    use super::*;
    use crate::store::{Spot, SpotKind};
    use crate::wire::id::OutfitId;

    fn loaded(source: &MemSource) -> NewGame {
        load(source, Path::new("games"))
    }

    fn id(text: &str) -> Id {
        Id::try_from(text.to_owned()).expect("an id")
    }

    fn outfit(game: &NewGame, nova: i16) -> Outfit {
        game.data
            .get::<Outfit>(nova)
            .expect("present")
            .expect("decoded")
            .record
            .clone()
    }

    fn kit_of(game: &NewGame, kit: &str) -> Outfit {
        outfit(game, game.ids.kits[&id(kit)])
    }

    fn slots(outfit: &Outfit) -> [(i16, i16); 4] {
        [
            (outfit.mod_type, outfit.mod_val),
            (outfit.mod_type2, outfit.mod_val2),
            (outfit.mod_type3, outfit.mod_val3),
            (outfit.mod_type4, outfit.mod_val4),
        ]
    }

    fn problems(game: &NewGame) -> Vec<String> {
        game.problems.iter().map(ToString::to_string).collect()
    }

    const MASS_DRIVER: &str = "---\nname: Mass driver\nmount: turret\ndamage: 12\nrange: 600\n\
                               reload: 0.5\nprice: 2500\n---\n";

    const ARMOUR_PLATING: &str = "---\nname: Armour plating\nfits: [shuttle, freighter, courier]\n\
                                  fitted_at: legal\nprice: 6000\n\
                                  effects: {hull: 300, speed: -40, handling: -10, cargo: -10, attention: 1}\n---\n";

    const GUN_MOUNT: &str = "---\nname: Gun mount\nfits: [shuttle, freighter]\nfitted_at: lawless\n\
                             price: 4000\neffects: {cargo: -6, handling: -10, attention: 2}\n\
                             hardpoints:\n  - id: gun-bed\n    mount: turret\n    position: [0, -2]\n\
                             \x20   outfit: mass-driver\n---\n";

    /// The fixture galaxy with the sample's two kits and its mass driver.
    fn sample_kits() -> MemSource {
        galaxy()
            .with("outfits/mass-driver.md", MASS_DRIVER)
            .with("mods/armour-plating.md", ARMOUR_PLATING)
            .with("mods/gun-mount.md", GUN_MOUNT)
    }

    #[test]
    fn kits_number_after_the_outfits_in_kit_id_order() {
        let game = loaded(&sample_kits());
        assert_eq!(problems(&game), Vec::<String>::new());
        let outfits: Vec<_> = game.ids.outfits.values().copied().collect();
        assert_eq!(outfits, [128, 129], "the blaster and the mass driver");
        let kits: Vec<_> = game
            .ids
            .kits
            .iter()
            .map(|(id, &n)| (id.to_string(), n))
            .collect();
        assert_eq!(
            kits,
            [
                ("armour-plating".to_owned(), 130),
                ("gun-mount".to_owned(), 131)
            ]
        );
    }

    #[test]
    fn armour_plating_takes_four_slots_in_order() {
        let plating = kit_of(&loaded(&sample_kits()), "armour-plating");
        assert_eq!(slots(&plating), [(6, 300), (8, -40), (9, -25), (2, -10)]);
    }

    #[test]
    fn the_gun_mount_adds_a_turret_slot_and_gives_its_gun() {
        let game = loaded(&sample_kits());
        let mount = kit_of(&game, "gun-mount");
        assert_eq!(slots(&mount), [(9, -25), (2, -6), (46, 1), (-1, 0)]);
        let driver = game.ids.outfits[&id("mass-driver")];
        assert_eq!(driver, 129);
        assert_eq!(mount.on_purchase.as_str(), "G129");
        assert_eq!(mount.on_sell.as_str(), "D129");
    }

    #[test]
    fn a_fixed_or_launcher_hardpoint_adds_a_gun_slot() {
        let game = loaded(&galaxy().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: lawless\nprice: 100\n\
             hardpoints:\n  - id: a\n    mount: fixed\n    position: [0, 1]\n\
             \x20 - id: b\n    mount: launcher\n    position: [0, 2]\n\
             \x20 - id: c\n    mount: bay\n    position: [0, 3]\n---\n",
        ));
        assert_eq!(
            slots(&kit_of(&game, "gun-mount")),
            [(45, 2), (-1, 0), (-1, 0), (-1, 0)]
        );
    }

    #[test]
    fn a_kit_is_fitted_once_sold_every_day_hidden_unless_it_fits_and_costs_its_price() {
        let plating = kit_of(&loaded(&sample_kits()), "armour-plating");
        assert_eq!(plating.max, 1);
        assert_eq!(plating.buy_random, 100);
        assert_eq!(plating.flags, Flags16(HIDE_UNLESS_REQUIRED));
        assert_eq!(HIDE_UNLESS_REQUIRED, 0x0100);
        assert_eq!(plating.cost, 6000);
        assert_eq!(plating.mass, 0);
        assert_eq!(plating.require_govt, -1);
    }

    #[test]
    fn a_legal_kit_is_tech_1_unscanned_and_a_lawless_one_tech_2_scanned() {
        let game = loaded(&sample_kits());
        let plating = kit_of(&game, "armour-plating");
        assert_eq!(
            (plating.tech_level, plating.scan_mask),
            (KIT_TECH_LEGAL, Flags16(0))
        );
        let mount = kit_of(&game, "gun-mount");
        assert_eq!(
            (mount.tech_level, mount.scan_mask),
            (KIT_TECH_LAWLESS, Flags16(LAWLESS_KIT_SCAN))
        );
        assert_eq!((KIT_TECH_LEGAL, KIT_TECH_LAWLESS), (1, 2));
        assert_eq!(LAWLESS_KIT_SCAN, 0x0001);
    }

    #[test]
    fn handling_is_on_the_hull_s_turn_scale_halves_away_from_zero() {
        assert_eq!(turn_change(-10), -25);
        assert_eq!(turn_change(1), 3);
        assert_eq!(turn_change(-1), -3);
        assert_eq!(turn_change(4), 10);
        assert_eq!(turn_change(i32::MAX), i16::MAX);
        assert_eq!(turn_change(i32::MIN), i16::MIN);
    }

    #[test]
    fn a_kit_needing_five_slots_keeps_four_and_reports_the_fifth() {
        let game = loaded(&galaxy().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: legal\nprice: 100\n\
             effects: {hull: 1, shields: 2, speed: 3, handling: 4, cargo: 5}\n---\n",
        ));
        assert_eq!(
            problems(&game),
            ["mods/gun-mount.md: it needs more than four effect slots; `cargo` left out"]
        );
        assert_eq!(
            slots(&kit_of(&game, "gun-mount")),
            [(6, 1), (4, 2), (8, 3), (9, 10)]
        );
    }

    #[test]
    fn a_hardpoint_outfit_that_is_not_there_is_reported_and_not_given() {
        let game = loaded(&galaxy().with("mods/gun-mount.md", GUN_MOUNT));
        assert_eq!(
            problems(&game),
            [
                "mods/gun-mount.md: its hardpoint `gun-bed`'s outfit `mass-driver` is not in \
                 outfits/; it isn't given with the kit"
            ]
        );
        let mount = kit_of(&game, "gun-mount");
        assert_eq!(
            (mount.on_purchase.as_str(), mount.on_sell.as_str()),
            ("", "")
        );
    }

    #[test]
    fn an_effect_past_i16_is_held_to_the_nearest() {
        let game = loaded(&galaxy().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: legal\nprice: 100\n\
             effects: {hull: 40000, cargo: -40000, speed: 7}\n---\n",
        ));
        assert_eq!(
            slots(&kit_of(&game, "gun-mount")),
            [(6, i16::MAX), (8, 7), (2, i16::MIN), (-1, 0)]
        );
    }

    #[test]
    fn a_hardpoint_outfit_that_failed_to_load_is_not_reported_again() {
        let game = loaded(&galaxy().with("mods/gun-mount.md", GUN_MOUNT).with(
            "outfits/mass-driver.md",
            "---\nname: Mass driver\nmount: sideways\n---\n",
        ));
        let lines = problems(&game);
        assert!(
            !lines.iter().any(|line| line.starts_with("mods/")),
            "{lines:?}"
        );
    }

    #[test]
    fn two_hardpoint_outfits_are_given_in_order() {
        let game = loaded(&galaxy().with(
            "mods/gun-mount.md",
            "---\nname: Gun mount\nfits: [shuttle]\nfitted_at: legal\nprice: 100\n\
             hardpoints:\n  - id: a\n    mount: fixed\n    position: [0, 1]\n    outfit: blaster\n\
             \x20 - id: b\n    mount: fixed\n    position: [0, 2]\n    outfit: blaster\n---\n",
        ));
        let mount = kit_of(&game, "gun-mount");
        assert_eq!(mount.on_purchase.as_str(), "G128 G128");
        assert_eq!(mount.on_sell.as_str(), "D128 D128");
    }

    #[test]
    fn kits_of_one_fits_set_share_a_bit_and_a_role_contributes_every_set_holding_it() {
        let galaxy_source = sample_kits()
            .with(
                "mods/hidden-hold.md",
                "---\nname: Hidden hold\nfits: [freighter, shuttle]\nfitted_at: lawless\nprice: 1\n---\n",
            )
            .with(
                "mods/racing-fins.md",
                "---\nname: Fins\nfits: [racer]\nfitted_at: legal\nprice: 1\n---\n",
            );
        let game = loaded(&galaxy_source);
        let require = |kit: &str| kit_of(&game, kit).require.bits();
        assert_eq!(require("armour-plating"), 0b001);
        assert_eq!(require("gun-mount"), 0b010);
        assert_eq!(require("hidden-hold"), 0b010, "the same set, sorted");
        assert_eq!(require("racing-fins"), 0b100);

        let (loaded_galaxy, _) = nova_galaxy_format::load::load(&galaxy_source);
        let fit = FitBits::of(&loaded_galaxy, &mut Vec::new());
        assert_eq!(fit.contributed(Some(Role::Freighter)), 0b011);
        assert_eq!(fit.contributed(Some(Role::Courier)), 0b001);
        assert_eq!(fit.contributed(Some(Role::Racer)), 0b100);
        assert_eq!(fit.contributed(Some(Role::Cruiser)), 0);
        assert_eq!(fit.contributed(None), 0);
    }

    #[test]
    fn a_kit_past_the_64th_fits_set_is_reported_and_left_out() {
        const ROLES: [&str; 7] = [
            "shuttle",
            "freighter",
            "courier",
            "racer",
            "fighter",
            "raider",
            "gunboat",
        ];
        // 7 roles give 127 non-empty sets: number the first 66 by their
        // bits.
        let source = (1..=65u32).fold(galaxy(), |source, n| {
            let roles: Vec<&str> = (0..7)
                .filter(|bit| n & (1 << bit) != 0)
                .map(|bit| ROLES[bit])
                .collect();
            source.with(
                &format!("mods/k{n:03}.md"),
                &format!(
                    "---\nname: K\nfits: [{}]\nfitted_at: legal\nprice: 1\n---\n",
                    roles.join(", ")
                ),
            )
        });
        let game = loaded(&source);
        let lines = problems(&game);
        assert_eq!(
            lines,
            ["mods/k065.md: its `fits` is past the 64th distinct set of roles; it is left out"]
        );
        assert!(!game.ids.kits.contains_key(&id("k065")));
        assert!(game.ids.kits.contains_key(&id("k064")));
    }

    #[test]
    fn a_kit_past_the_last_outfit_id_is_a_problem_and_left_out() {
        let pass = "---\nname: A\nmount: fixed\ndamage: 0\nrange: 0\nreload: 1\nprice: 0\n---\n";
        let source = (0..511).fold(galaxy(), |source, n| {
            source.with(&format!("outfits/a{n:03}.md"), pass)
        });
        let game = loaded(&source);
        assert_eq!(
            problems(&game),
            ["mods/gun-mount.md: it is past the last oütf ID, 639; it is left out"]
        );
        assert!(game.ids.kits.is_empty());
    }

    #[test]
    fn the_side_table_holds_each_kit_s_attention_and_spots() {
        let game = loaded(&sample_kits());
        let kits = &game.data.fittings().kits;
        assert_eq!(
            kits[&OutfitId(130)],
            Kit {
                attention: 1,
                spots: Vec::new(),
            }
        );
        assert_eq!(
            kits[&OutfitId(131)],
            Kit {
                attention: 2,
                spots: vec![Spot {
                    kind: SpotKind::Turret,
                    at: (0, -2),
                }],
            }
        );
    }
}
