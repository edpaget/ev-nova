//! The opt-in stock look: the new game's hulls drawn in stock EV Nova
//! sprites from the player's own `Nova Files`, in place of their
//! stand-ins.
//!
//! The bible has models, not sprites, so each new-game hull draws as its
//! stand-in (`crate::new_game::looks`). A player who owns EV Nova may turn
//! on, in the settings file, a look that borrows a stock `shïp`'s sprites
//! for each hull instead (`nova::stock_look`). This module decides what
//! each hull borrows ([`resolve`]); the view draws it
//! (`nova_view::flight::StockShipLooks`) and the renderer decodes it
//! (`nova_render::GameImages::with_stock_art`), both from the read-only
//! [`StockArt`].
//!
//! # What each hull borrows
//!
//! A hull's pick is, in order:
//!
//! 1. the player's override for its own bible id;
//! 2. the player's override for its nearest base, walking its chain
//!    ([`ShipLine::chain`]), so a variant, and so a mod-kit hull, borrows
//!    its base's look;
//! 3. the default for its role ([`ROLE_DEFAULTS`]);
//! 4. otherwise nothing: it draws its stand-in.
//!
//! The borrowed art is scaled to the hull's size ([`Scale`]): the hull's
//! longer `shän` side over the stock base frame's longer side.
//!
//! # Fallbacks
//!
//! Nothing panics, and each kind of trouble is one warning, counting the
//! hulls and naming the first ([`resolve`]): hulls with no pick, picks
//! that aren't in the player's data or whose sheet doesn't read, and
//! overrides naming no new-game ship. Each of those hulls draws its
//! stand-in.
//!
//! # Nothing is written
//!
//! The stock art is copyrighted. Nothing here or in the stores reads it
//! for anything but drawing: [`ROLE_DEFAULTS`] and the overrides are stock
//! IDs alone, the new game's store never holds stock data, and no cache,
//! save or build ever holds what [`StockArt`] reads.

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use nova_galaxy_format::fleet::Role;

use crate::records::ship_anim::ShipAnim;
use crate::store::{GameData, ShipLine, StockArt};
use crate::wire::id::ShipId;

/// The stock `shïp` each role borrows by default: an ordinary stock ship
/// of the role, tied to no storyline, whose base sheet reads and whose
/// base frame is nearest the role's usual size in the bible. Only stock
/// IDs, no art. Each is a one-line edit, and the player can override any
/// hull in the settings file.
///
/// | Role | Stock `shïp` |
/// |---|---|
/// | shuttle | 128, Shuttle (24 px) |
/// | freighter | 136, Terrapin (40 px) |
/// | courier | 129, Heavy Shuttle (32 px) |
/// | racer | 135, Lightning (48 px) |
/// | fighter | 167, Viper (32 px) |
/// | raider | 166, Pirate Viper (32 px) |
/// | gunboat | 157, Thunderhead (48 px) |
/// | corvette | 137, Valkyrie (48 px) |
/// | cruiser | 131, Leviathan (144 px) |
/// | carrier | 153, Aurora Carrier (184 px) |
pub const ROLE_DEFAULTS: &[(Role, i16)] = &[
    (Role::Shuttle, 128),
    (Role::Freighter, 136),
    (Role::Courier, 129),
    (Role::Racer, 135),
    (Role::Fighter, 167),
    (Role::Raider, 166),
    (Role::Gunboat, 157),
    (Role::Corvette, 137),
    (Role::Cruiser, 131),
    (Role::Carrier, 153),
];

/// How much borrowed art is scaled: a whole-number ratio, `numerator`
/// over `denominator`, in lowest terms, so equal ratios are equal keys.
/// Integers, as it rides in the renderer's image keys, which hash and
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Scale {
    numerator: u32,
    denominator: u32,
}

impl Scale {
    /// Drawn at its own size.
    pub const ONE: Self = Self {
        numerator: 1,
        denominator: 1,
    };

    /// Art whose longer side is `art` pixels drawn with its longer side
    /// `hull` pixels; 1:1 when either is 0 (a hull with no size).
    #[must_use]
    pub fn of(hull: u32, art: u32) -> Self {
        if hull == 0 || art == 0 {
            return Self::ONE;
        }
        let common = gcd(hull, art);
        Self {
            numerator: hull / common,
            denominator: art / common,
        }
    }

    /// The ratio's numerator.
    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    /// The ratio's denominator.
    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }

    /// `px` scaled, rounded to the nearest whole pixel, and never below 1:
    /// the one rule every scaled dimension follows.
    #[must_use]
    pub fn apply(self, px: u32) -> u32 {
        let (numerator, denominator) = (u64::from(self.numerator), u64::from(self.denominator));
        let scaled = (u64::from(px) * numerator + denominator / 2) / denominator;
        u32::try_from(scaled).unwrap_or(u32::MAX).max(1)
    }
}

/// The greatest common divisor of `a` and `b`, neither 0.
const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// What one hull borrows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Borrow {
    /// The stock `shïp` whose sprites it draws.
    pub stock: ShipId,
    /// How much they are scaled.
    pub scale: Scale,
}

/// What [`resolve`] decided: each hull's borrow, and the warnings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Resolved {
    /// Each hull that borrows, by its new-game `shïp`.
    pub borrows: BTreeMap<ShipId, Borrow>,
    /// At most one warning of each kind (see the module docs).
    pub warnings: Vec<String>,
}

/// What each of `game`'s ships borrows from `stock`, under the player's
/// `overrides` (a bible ship id to a stock `shïp` ID), by the precedence in
/// the module docs.
#[must_use]
pub fn resolve(game: &GameData, stock: &StockArt, overrides: &BTreeMap<String, i16>) -> Resolved {
    let mut borrows = BTreeMap::new();
    let mut unmapped = Vec::new();
    let mut unreadable = Vec::new();
    let mut ships = BTreeSet::new();
    for (ship, line) in game.ship_lines() {
        ships.extend(line.chain.first().map(String::as_str));
        let Some(pick) = pick(line, overrides) else {
            unmapped.push(line);
            continue;
        };
        let art = match stock.ship_sprite_info(ShipId(pick)) {
            Ok(sprite) => longer(
                sprite.sheet.info.frame_width,
                sprite.sheet.info.frame_height,
            ),
            Err(error) => {
                unreadable.push((line, pick, error.to_string()));
                continue;
            }
        };
        let hull = game
            .get::<ShipAnim>(ship.0)
            .and_then(Result::ok)
            .map_or(0, |anim| {
                let side = |px: i16| u32::try_from(px).unwrap_or(0);
                longer(side(anim.record.base_x_size), side(anim.record.base_y_size))
            });
        let scale = Scale::of(hull, art);
        borrows.insert(
            ship,
            Borrow {
                stock: ShipId(pick),
                scale,
            },
        );
    }
    let mut warnings = Vec::new();
    if let Some(first) = unmapped.first() {
        let (hulls, borrow, draws) = match unmapped.len() {
            1 => ("1 hull".to_owned(), "borrows", "it draws its stand-in"),
            n => (format!("{n} hulls"), "borrow", "each draws its stand-in"),
        };
        warnings.push(format!(
            "{hulls} {borrow} no stock ship, starting with `{}`: no override names it or a base \
             of it, and it has no role; {draws}",
            own_id(first)
        ));
    }
    if let Some((first, pick, why)) = unreadable.first() {
        let (hulls, their, draws) = match unreadable.len() {
            1 => ("1 hull".to_owned(), "its", "it draws its stand-in"),
            n => (format!("{n} hulls"), "their", "each draws its stand-in"),
        };
        warnings.push(format!(
            "{hulls} can't borrow {their} stock ship, starting with `{}`'s shïp {pick}: {why}; \
             {draws}",
            own_id(first)
        ));
    }
    let strays: Vec<&String> = overrides
        .keys()
        .filter(|key| !ships.contains(key.as_str()))
        .collect();
    if let Some(first) = strays.first() {
        let (count, name, they) = match strays.len() {
            1 => ("1 override".to_owned(), "names", "it is"),
            n => (format!("{n} overrides"), "name", "they are"),
        };
        warnings.push(format!(
            "{count} {name} no new-game ship, starting with `{first}`; {they} ignored"
        ));
    }
    Resolved { borrows, warnings }
}

/// The stock `shïp` `line`'s ship borrows: the override for the nearest
/// ship of its chain that has one, else its role's default.
fn pick(line: &ShipLine, overrides: &BTreeMap<String, i16>) -> Option<i16> {
    line.chain
        .iter()
        .find_map(|id| overrides.get(id).copied())
        .or_else(|| {
            let role = line.role?;
            ROLE_DEFAULTS
                .iter()
                .find(|(of, _)| *of == role)
                .map(|&(_, stock)| stock)
        })
}

/// The ship's own bible id, the head of its chain.
fn own_id(line: &ShipLine) -> &str {
    line.chain.first().map_or("", String::as_str)
}

/// The longer of `width` and `height`.
fn longer(width: u32, height: u32) -> u32 {
    width.max(height)
}
