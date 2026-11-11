//! Ships as `shïp`s and `shän`s, the placeholder traffic `düde`, and
//! starts as `chär`s.
//!
//! The bible has no traffic, no start date and no Nova-style stats, so
//! some values are placeholders, each a constant here:
//!
//! - **Ships** ([`ship`]). Every ship file maps, a hull or a variant,
//!   through the format's `base` chain ([`resolve`]): its stats merged
//!   field by field down the chain, the nearest winning, before its kits;
//!   its `size`, `model` and `livery` the nearest; its hardpoints the
//!   chain's, a variant's replacing its base's of the same `id`, then its
//!   kits'; its kits its base's, then its own. The bible's `speed` is
//!   Nova's (speed/100 pixels a tick), `shields`, `hull` and `cargo` are
//!   its `Shield`, `Armor` and `Holds`. Its acceleration is its speed and
//!   its `Maneuver` its `handling` ([`maneuver`], the turn scale), both
//!   placeholders, and every ship holds [`HULL_FUEL`] and a crew of
//!   one. Its kits are its `DefaultItems`, one of each, so its stats come
//!   from them as a fitted kit's do (`super::kits`). Its chain's `fixed`
//!   and `launcher` hardpoints are its `MaxGun` and its `turret` ones its
//!   `MaxTur` (a kit adds its own through its effects). The weapon
//!   outfits its hardpoints and its kits' hardpoints carry are its stock
//!   weapons, one a hardpoint, so the traffic fires them. It contributes
//!   the `Require` bit of every kit `fits` set holding its `role`
//!   ([`FitBits`]), and the mount bit of each of its chain's hardpoints
//!   ([`FitBits::mounts`]): its kits' hardpoints come with the kits' own
//!   `Contribute`, since a pilot owns its hull's kits. Its `Cost` is the
//!   chain's `price` ([`Resolved::price`]: a variant without one costs its
//!   base's plus the kits it adds), held within `i32`, 0 with none; a hull
//!   sold nowhere keeps it, for the shipyard's trade-in. Sold at the yards
//!   of its `sold_at`, it takes their tech number ([`super::yards`]) and
//!   `BuyRandom` 100; sold nowhere, `TechLevel` 0x7fff, which no yard
//!   reaches and which keeps the low-tech discount off its trade-in, and
//!   `BuyRandom` 0. `HireRandom` stays 0: no ship is for hire. A chain
//!   that loops, or names a base that isn't there, leaves the ship out.
//! - **Size and firing positions** ([`ship_anim`]). A ship's `shän` sets
//!   `BaseXSize` (the beam) and `BaseYSize` (the length), in pixels at
//!   [`PX_PER_METRE`]: the simulation hovers at that size, and collides
//!   at it under the engine's reading. Its `collision_radius`, which a
//!   `shän` has no field for, rides on the store in whole pixels at the
//!   same scale ([`collision_radius`], `GameData::collision_radius`), and
//!   the simulation strikes the ship as that disc under the rulebook's
//!   `data_collision` reading (`nova_sim::combat::data_collision`).
//!   Its hardpoints, its chain's and then its kits', are its exit points:
//!   a `fixed` one a gun point, a `turret` one a turret point, a
//!   `launcher` one a guided point, a `bay` none, at its `position` in
//!   whole pixels ([`spot`]), at height 0: the simulation is flat. Where
//!   each stands on the hull's model in 3D, and the muzzle its outfit's
//!   model gives a weapon, are the mounts' ([`super::mounts`]). Each kind's
//!   four points cycle through that
//!   kind's hardpoints in order ([`Spot::exit_points`], the one rule the
//!   player's points are rebuilt by too); a kind with none keeps the
//!   centre. It names no sprite: the view draws the ship's stand-in,
//!   its model from above or its size box at the same scale
//!   ([`super::looks`]), or, with the opt-in stock look on, stock sprites
//!   from the player's own data scaled to that size
//!   ([`crate::stock_look`]). The store's fittings keep each ship's chain
//!   hardpoints too, for the player's exit points to be rebuilt from its
//!   class and the kits it carries (`crate::store::Fittings`).
//! - **Traffic** ([`civilian_traffic`]). One `düde`,
//!   [`CIVILIAN_DUDE`], "Civilian traffic":
//!   every hull (no `base`, no `mods`) of `style: civilian`, at even odds,
//!   flown as wimpy traders of no government. Every system names it in
//!   `DudeTypes[0]` at 100%, with [`TRAFFIC_SHIPS`] ships on average.
//!   A faction holding a `take_point` gets its guards' `düde`
//!   ([`super::blockades`]); variants in traffic and factions' traffic
//!   fleets wait for the bible to author traffic.
//! - **Starts** ([`character`]). Each start is a `chär` in its ship, in
//!   its system, with its cash, on [`START_DATE`].

use std::collections::BTreeMap;

use nova_galaxy_format::fleet::{
    Hardpoint, Mount, ResolveError, Resolved, Ship as BibleShip, Style, resolve,
};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::{Doc, Galaxy, Problem};
use nova_galaxy_format::model::Start;

use super::ids::{CHARACTER_IDS, CIVILIAN_DUDE, IdTable, SHIP_IDS, after_pins};
use super::kits::{ALWAYS, FitBits};
use super::map::{absent, number};
use super::records::{
    OUTFIT_TECH_NEVER, neutral_character, neutral_dude, neutral_ship, neutral_ship_anim,
};
use crate::decode::Entry;
use crate::records::character::Character;
use crate::records::dude::Dude;
use crate::records::ship::Ship;
use crate::records::ship_anim::ShipAnim;
use crate::registry::AnyRecord;
use crate::store::{Spot, SpotKind};
use crate::wire::flags::Flags64;
use crate::wire::id::{OutfitId, ShipId, SystemId, WeaponId};

/// Pixels a metre of a hull's `size` takes (provisional: the scale is a
/// guess, kept here alone).
pub const PX_PER_METRE: f64 = 1.0;

/// Every hull's fuel: 300, three jumps, a placeholder above the 99 the
/// engine's reading needs for an NPC to jump out where a ruleset lets it.
pub const HULL_FUEL: i16 = 300;

/// The new game's turn scale: the `Maneuver`, in tenths of a degree a
/// tick, that `handling` makes. A hull's `Maneuver` and a kit's turn
/// change ([`super::kits::turn_change`]) both read their `handling`
/// through it, so the two can't drift apart.
///
/// The scale reads `handling` as `Maneuver` itself. It is a placeholder
/// reading for the new game, not the original's: the bible gives
/// `handling` no unit, and the original's ships carry a `Maneuver` of
/// their own. It is chosen so a kit's handling cost can be felt. The
/// player turns at least 1° a tick (`nova_sim_base::handling::MIN_TURN_RATE`,
/// `Maneuver` 10), and the bible's hulls, `handling` 15 to 85, turn 1.5°
/// to 8.5° a tick on this scale, all above that floor. So a kit's
/// `handling` -10 takes a whole degree off: the sample's light freighter
/// (35) turns 3.5° a tick, and 2.5° with armour plating fitted. An
/// earlier quarter scale put every hull under 40 at the floor, kit or
/// none.
#[must_use]
pub const fn maneuver(handling: i64) -> i64 {
    handling
}

/// How many ships each system's traffic holds on average (`AvgShips`, a
/// placeholder).
pub const TRAFFIC_SHIPS: i16 = 4;

/// The AI type the civilian traffic flies as: 1, a wimpy trader.
const WIMPY_TRADER: i16 = 1;

/// The date every start begins on, as (day, month, year): 1 January 1200,
/// a placeholder until the bible dates its starts.
pub const START_DATE: (i16, i16, i16) = (1, 1, 1200);

/// How many default items and stock weapons a `shïp` holds.
const SHIP_SLOTS: usize = 8;

/// How many default items or stock weapons each of a `shïp`'s two arrays
/// holds.
const ARRAY_SLOTS: usize = 4;

/// The ships of `galaxy` that are hulls: no `base` and no `mods`.
pub fn hulls(galaxy: &Galaxy) -> impl Iterator<Item = (&Id, &Doc<BibleShip>)> {
    galaxy
        .ships
        .iter()
        .filter(|(_, doc)| doc.data.base.is_none() && doc.data.mods.is_empty())
}

/// Numbers the ships of `galaxy` whose `base` chain resolves within
/// [`SHIP_IDS`], each `pins` names at its pin and the others after the
/// highest pin ([`super::ids`]); each whose chain loops or names a base that isn't there
/// is reported and left out, a base that failed to load unreported.
pub fn number_ships(
    galaxy: &Galaxy,
    pins: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let resolved: Vec<_> = galaxy
        .ships
        .iter()
        .filter(|(id, doc)| match resolve(galaxy, id) {
            Ok(_) => true,
            Err(ResolveError::Cycle(chain)) => {
                let chain: Vec<&str> = chain.iter().map(Id::as_str).collect();
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its `base` chain loops ({}); it is left out",
                        chain.join(" -> ")
                    ),
                ));
                false
            }
            Err(ResolveError::MissingBase(base)) => {
                if absent(galaxy, &galaxy.ships, "ships", &base) {
                    problems.push(Problem::new(
                        &doc.path,
                        format!("its base `{base}` is not in ships/; it is left out"),
                    ));
                }
                false
            }
        })
        .collect();
    let mut free = after_pins(SHIP_IDS, pins.values());
    number(resolved, pins, &mut free, "shïp", problems)
}

/// `stat` as a record's word: none is 0, and one past `i16` the most.
fn word(stat: Option<u32>) -> i16 {
    stat.map_or(0, |stat| saturated(stat.into()))
}

/// `value` held within `i16`.
pub(super) fn saturated(value: i64) -> i16 {
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let value = value.clamp(i16::MIN.into(), i16::MAX.into()) as i16;
    value
}

/// A count as a record's word.
fn count(n: usize) -> i16 {
    i16::try_from(n).unwrap_or(i16::MAX)
}

/// How many of `ship`'s hardpoints are its chain's: those before its
/// kits'.
pub(super) fn chain_len(galaxy: &Galaxy, ship: &Resolved) -> usize {
    let kits: usize = (ship.mods.iter())
        .filter_map(|kit| galaxy.mods.get(kit))
        .map(|kit| kit.data.hardpoints.len())
        .sum();
    ship.hardpoints.len() - kits
}

/// The ship `id` of `galaxy` resolved; its numbering checked the chain.
fn resolved<'a>(galaxy: &'a Galaxy, id: &Id) -> Option<Resolved<'a>> {
    resolve(galaxy, id).ok()
}

/// The neutral `shïp` with `name`, as written, as its short, long and
/// comm names: the format has one name, and the screens read three
/// (the grid cell, the Info panel's title, the comm dialog's class line).
/// Its `Subtitle` stays blank: nothing in the format derives one.
fn named_ship(name: &str) -> Ship {
    Ship {
        short_name: name.into(),
        comm_name: name.into(),
        long_name: name.into(),
        ..neutral_ship()
    }
}

/// The `shïp` of the ship `id`, numbered `nova` (see the module docs), its
/// names its `name` ([`named_ship`]), its
/// kits' `oütf`s and its weapons' `wëap`s from `ids`, its `Contribute` its
/// role's kit sets in `fit` and its chain hardpoints' mounts, sold at the
/// yards holding the tech number `sold` when it is sold. A ship whose
/// chain doesn't resolve is the neutral `shïp` with its names.
#[must_use]
pub fn ship(
    galaxy: &Galaxy,
    ids: &IdTable,
    fit: &FitBits,
    id: &Id,
    nova: i16,
    sold: Option<i16>,
    problems: &mut Vec<Problem>,
) -> Entry<AnyRecord> {
    let doc = &galaxy.ships[id];
    let Some(resolved) = resolved(galaxy, id) else {
        return Entry {
            id: nova,
            name: Some(doc.data.name.clone()),
            record: AnyRecord::Ship(Box::new(named_ship(&doc.data.name))),
        };
    };
    let stats = resolved.stats;
    let speed = word(stats.speed);
    let chain = &resolved.hardpoints[..chain_len(galaxy, &resolved)];
    let mounted = |mounts: &[Mount]| {
        count(
            chain
                .iter()
                .filter(|placed| mounts.contains(&placed.hardpoint.mount))
                .count(),
        )
    };
    let mut ship = Ship {
        speed,
        accel: speed,
        maneuver: stats
            .handling
            .map_or(0, |handling| saturated(maneuver(handling.into()))),
        shield: word(stats.shields),
        armor: word(stats.hull),
        holds: word(stats.cargo),
        fuel: HULL_FUEL,
        crew: 1,
        max_gun: mounted(&[Mount::Fixed, Mount::Launcher]),
        max_tur: mounted(&[Mount::Turret]),
        contribute: Flags64(
            fit.contributed(resolved.role)
                | FitBits::mounts(chain.iter().map(|placed| placed.hardpoint)),
        ),
        cost: resolved
            .price
            .map_or(0, |price| i32::try_from(price).unwrap_or(i32::MAX)),
        tech_level: sold.unwrap_or(OUTFIT_TECH_NEVER),
        buy_random: if sold.is_some() { ALWAYS } else { 0 },
        ..named_ship(&doc.data.name)
    };
    let items = default_items(galaxy, ids, doc, &resolved, problems);
    for (slot, &(item, n)) in items.iter().enumerate() {
        let item = Some(OutfitId(item));
        if slot < ARRAY_SLOTS {
            ship.default_items1_4[slot] = item;
            ship.item_count1_4[slot] = n;
        } else {
            ship.default_items5_8[slot - ARRAY_SLOTS] = item;
            ship.item_count5_8[slot - ARRAY_SLOTS] = n;
        }
    }
    let weapons = stock_weapons(galaxy, ids, doc, &resolved, chain.len(), problems);
    for (slot, &(weapon, n)) in weapons.iter().enumerate() {
        let weapon = Some(WeaponId(weapon));
        if slot < ARRAY_SLOTS {
            ship.weap_type1_4[slot] = weapon;
            ship.weap_count1_4[slot] = n;
        } else {
            ship.weap_type5_8[slot - ARRAY_SLOTS] = weapon;
            ship.weap_count5_8[slot - ARRAY_SLOTS] = n;
        }
    }
    Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::Ship(Box::new(ship)),
    }
}

/// Adds one of `item` to `counts`, keeping their first order.
fn add_one(counts: &mut Vec<(i16, i16)>, item: i16) {
    match counts.iter_mut().find(|(known, _)| *known == item) {
        Some((_, n)) => *n = n.saturating_add(1),
        None => counts.push((item, 1)),
    }
}

/// The first eight of `ship`'s kits as `oütf`s, each with how many, a kit
/// `doc` names itself that isn't in `mods/` reported, and those past the
/// eighth reported (see the module docs).
fn default_items(
    galaxy: &Galaxy,
    ids: &IdTable,
    doc: &Doc<BibleShip>,
    ship: &Resolved,
    problems: &mut Vec<Problem>,
) -> Vec<(i16, i16)> {
    for kit in &doc.data.mods {
        if absent(galaxy, &galaxy.mods, "mods", kit) {
            problems.push(Problem::new(
                &doc.path,
                format!("its mod kit `{kit}` is not in mods/; it is left out"),
            ));
        }
    }
    let mut items = Vec::new();
    for kit in &ship.mods {
        if let Some(&nova) = ids.kits.get(kit) {
            add_one(&mut items, nova);
        }
    }
    keep_slots(items, &doc.path, "default items", problems)
}

/// The first eight of `counts`, the rest, if any, reported against `file`
/// as `what`.
fn keep_slots(
    mut counts: Vec<(i16, i16)>,
    file: &str,
    what: &str,
    problems: &mut Vec<Problem>,
) -> Vec<(i16, i16)> {
    if counts.len() > SHIP_SLOTS {
        problems.push(Problem::new(
            file,
            format!(
                "it has more than eight {what}; {} are left out",
                counts.len() - SHIP_SLOTS
            ),
        ));
        counts.truncate(SHIP_SLOTS);
    }
    counts
}

/// The `wëap` of each weapon outfit `ship`'s hardpoints carry, one a
/// hardpoint, in hardpoint order, a hardpoint outfit of `doc`'s own (one
/// of its first `chain` hardpoints, added by `doc` itself) that isn't in
/// `outfits/` reported.
fn stock_weapons(
    galaxy: &Galaxy,
    ids: &IdTable,
    doc: &Doc<BibleShip>,
    ship: &Resolved,
    chain: usize,
    problems: &mut Vec<Problem>,
) -> Vec<(i16, i16)> {
    let mut weapons = Vec::new();
    for (at, placed) in ship.hardpoints.iter().enumerate() {
        let Some(outfit) = &placed.hardpoint.outfit else {
            continue;
        };
        let own = at < chain && *placed.from == ship.chain[0];
        if own && absent(galaxy, &galaxy.outfits, "outfits", outfit) {
            problems.push(Problem::new(
                &doc.path,
                format!(
                    "its hardpoint `{}`'s outfit `{outfit}` is not in outfits/; it is left out",
                    placed.hardpoint.id
                ),
            ));
        }
        if let Some(&weapon) = ids.weapons.get(outfit) {
            add_one(&mut weapons, weapon);
        }
    }
    keep_slots(weapons, &doc.path, "weapons", problems)
}

/// `metres` in whole pixels at [`PX_PER_METRE`], held within `i16`: the
/// one conversion a hardpoint's place, a collision radius and a muzzle's
/// reach share.
pub(super) fn pixels(metres: f64) -> i16 {
    let px = (metres * PX_PER_METRE).round();
    // Held within `i16` first, so the cast can't truncate.
    #[allow(clippy::cast_possible_truncation)]
    let px = px.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
    px
}

/// Where `hardpoint` fires from: its kind of exit point by its mount, at
/// its `position` in whole pixels at [`PX_PER_METRE`], +x right and +y
/// ahead as the `shän`'s are; none for a `bay`.
#[must_use]
pub fn spot(hardpoint: &Hardpoint) -> Option<Spot> {
    let kind = match hardpoint.mount {
        Mount::Fixed => SpotKind::Gun,
        Mount::Turret => SpotKind::Turret,
        Mount::Launcher => SpotKind::Guided,
        Mount::Bay => return None,
    };
    let [x, y] = hardpoint.position;
    Some(Spot {
        kind,
        at: (pixels(x), pixels(y)),
    })
}

/// The ship `id`'s chain hardpoints as spots, for the store's fittings.
#[must_use]
pub fn chain_spots(galaxy: &Galaxy, id: &Id) -> Vec<Spot> {
    let Some(ship) = resolved(galaxy, id) else {
        return Vec::new();
    };
    let chain = chain_len(galaxy, &ship);
    ship.hardpoints[..chain]
        .iter()
        .filter_map(|placed| spot(placed.hardpoint))
        .collect()
}

/// The ship `id`'s collision radius, its nearest `size`'s
/// `collision_radius` in whole pixels at [`PX_PER_METRE`] (held within
/// `i16`, as every size is); none for a ship with no `size`,
/// and none for a radius that isn't finite or isn't a whole positive pixel
/// once rounded (NaN, an infinity, 0, a negative, or under half a pixel),
/// so such a ship collides as the engine reads it.
#[must_use]
pub fn collision_radius(galaxy: &Galaxy, id: &Id) -> Option<i16> {
    let radius = resolved(galaxy, id)?.size?.collision_radius;
    if !radius.is_finite() {
        return None;
    }
    Some(pixels(radius)).filter(|&px| px > 0)
}

/// Every mapped ship's collision radius ([`collision_radius`]), by its
/// `shïp`, for the store ([`GameData::with_collision_radii`]).
///
/// [`GameData::with_collision_radii`]: crate::store::GameData::with_collision_radii
#[must_use]
pub fn collision_radii(galaxy: &Galaxy, ids: &IdTable) -> BTreeMap<ShipId, i16> {
    ids.ships
        .iter()
        .filter_map(|(id, &nova)| Some((ShipId(nova), collision_radius(galaxy, id)?)))
        .collect()
}

/// The `shän` of the ship `id`, numbered `nova`, its size and its exit
/// points; none for a ship with no `size`, which the simulation collides
/// as its default disc. A kind with more than four hardpoints is reported
/// against `file`.
#[must_use]
pub fn ship_anim(
    galaxy: &Galaxy,
    id: &Id,
    nova: i16,
    problems: &mut Vec<Problem>,
) -> Option<Entry<AnyRecord>> {
    let doc = &galaxy.ships[id];
    let ship = resolved(galaxy, id)?;
    let size = ship.size?;
    let spots: Vec<Spot> = ship
        .hardpoints
        .iter()
        .filter_map(|placed| spot(placed.hardpoint))
        .collect();
    let mut anim = ShipAnim {
        base_x_size: pixels(size.beam),
        base_y_size: pixels(size.length),
        ..neutral_ship_anim()
    };
    for (kind, name, x, y) in [
        (
            SpotKind::Gun,
            "gun",
            &mut anim.gun_pos_x,
            &mut anim.gun_pos_y,
        ),
        (
            SpotKind::Turret,
            "turret",
            &mut anim.turret_pos_x,
            &mut anim.turret_pos_y,
        ),
        (
            SpotKind::Guided,
            "launcher",
            &mut anim.guided_pos_x,
            &mut anim.guided_pos_y,
        ),
    ] {
        let Some((points, more)) = Spot::exit_points(&spots, kind) else {
            continue;
        };
        if more {
            problems.push(Problem::new(
                &doc.path,
                format!(
                    "it has more than four {name} hardpoints; the first four are its firing positions"
                ),
            ));
        }
        for (n, (px, py)) in points.into_iter().enumerate() {
            x[n] = px;
            y[n] = py;
        }
    }
    Some(Entry {
        id: nova,
        name: Some(doc.data.name.clone()),
        record: AnyRecord::ShipAnim(Box::new(anim)),
    })
}

/// The civilian traffic `düde`: the civilian hulls of `ships` (no `base`,
/// no `mods`) with the 16 lowest IDs, which pins can make differ from the
/// first 16 by bible id, in ID order, at even odds; none without a civilian
/// hull.
#[must_use]
pub fn civilian_traffic(galaxy: &Galaxy, ships: &BTreeMap<Id, i16>) -> Option<Entry<AnyRecord>> {
    let mut civilian: Vec<i16> = ships
        .iter()
        .filter(|(ship, _)| {
            hulls(galaxy)
                .any(|(hull, doc)| hull == *ship && doc.data.style == Some(Style::Civilian))
        })
        .map(|(_, &nova)| nova)
        .collect();
    // `ships` iterates in bible id order, which pins can break.
    civilian.sort_unstable();
    civilian.truncate(16);
    if civilian.is_empty() {
        return None;
    }
    let odds = i16::try_from(100 / civilian.len()).unwrap_or(1).max(1);
    let mut dude = Dude {
        ai_type: WIMPY_TRADER,
        ..neutral_dude()
    };
    for (slot, &nova) in civilian.iter().enumerate() {
        dude.ship_type[slot] = Some(ShipId(nova));
        dude.probability[slot] = odds;
    }
    Some(Entry {
        id: CIVILIAN_DUDE,
        name: Some("Civilian traffic".to_owned()),
        record: AnyRecord::Dude(Box::new(dude)),
    })
}

/// The starts of `galaxy` whose ship is mapped, a hull or a variant, and
/// whose system has a record, numbered within [`CHARACTER_IDS`], each
/// `pins` names at its pin and the others after the highest pin, so a
/// start added later never comes before a pinned one. Each other is reported
/// and left out, but not for a file that failed to load or a record left
/// out for its ID.
pub fn number_starts(
    galaxy: &Galaxy,
    ids: &IdTable,
    pins: &BTreeMap<Id, i16>,
    problems: &mut Vec<Problem>,
) -> BTreeMap<Id, i16> {
    let mapped: Vec<(&Id, &Doc<Start>)> = galaxy
        .starts
        .iter()
        .filter(|(_, doc)| {
            let start = &doc.data;
            let has_ship = ids.ships.contains_key(&start.ship);
            // A ship left out for its chain or its ID is reported already.
            if !has_ship && absent(galaxy, &galaxy.ships, "ships", &start.ship) {
                problems.push(Problem::new(
                    &doc.path,
                    format!("its ship `{}` is not in ships/; it is left out", start.ship),
                ));
            }
            let has_system = ids.systems.contains_key(&start.system);
            if !has_system && absent(galaxy, &galaxy.systems, "systems", &start.system) {
                problems.push(Problem::new(
                    &doc.path,
                    format!(
                        "its system `{}` is not in systems/; it is left out",
                        start.system
                    ),
                ));
            }
            has_ship && has_system
        })
        .collect();
    let mut free = after_pins(CHARACTER_IDS, pins.values());
    number(mapped, pins, &mut free, "chär", problems)
}

/// The `chär` of the start `doc`, numbered `nova` (see the module docs).
#[must_use]
pub fn character(ids: &IdTable, nova: i16, doc: &Doc<Start>) -> Entry<AnyRecord> {
    let start = &doc.data;
    let mut system = [None; 4];
    system[0] = ids.systems.get(&start.system).copied().map(SystemId);
    let (day, month, year) = START_DATE;
    let character = Character {
        cash: i32::try_from(start.cash).unwrap_or(i32::MAX),
        ship_type: ids.ships.get(&start.ship).copied().map(ShipId),
        system,
        unknown_0x134: day,
        unknown_0x136: month,
        start_year: year,
        ..neutral_character()
    };
    Entry {
        id: nova,
        name: Some(start.name.clone()),
        record: AnyRecord::Character(Box::new(character)),
    }
}
