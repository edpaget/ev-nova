//! The kinds that describe ships: hulls and their variants (`ships/`), the
//! mod kits fitted to them (`mods/`), the outfits fitted to their
//! hardpoints (`outfits/`), and the paint they wear (`liveries/`).
//!
//! A ship may name a `base` and inherit what it leaves out, and add mod
//! kits, so a modified civilian is a base hull plus kits. [`resolve`]
//! works out the finished ship.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::id::{Color, Id};
use crate::load::Galaxy;

/// A ship (`ships/<id>.md`): a hull class, or a variant of one.
///
/// A variant names a `base` ship and inherits every field it leaves out,
/// except `sold_at`: where a ship is sold is its own. Its `stats` are
/// merged field by field, its `hardpoints` are added to the base's (one
/// with the same `id` replaces the base's), and its `mods` are added to
/// the base's. A variant that leaves out `price` costs its base's plus
/// the kits it adds. The finished ship must end up with a `style`,
/// `role`, `model` and `size`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Ship {
    /// Display name (a placeholder).
    pub name: String,
    /// The ship this one is a variant of (`ships/<id>.md`).
    #[serde(default)]
    pub base: Option<Id>,
    /// The design language the hull is built in.
    #[serde(default)]
    pub style: Option<Style>,
    /// What the hull is for. Mod kits list the roles they fit.
    #[serde(default)]
    pub role: Option<Role>,
    /// The 3D model (`assets/<id>.md`, kind `model`).
    #[serde(default)]
    pub model: Option<Id>,
    /// The paint it wears (`liveries/<id>.md`).
    #[serde(default)]
    pub livery: Option<Id>,
    /// Its size, which the game uses for collisions. Written here, not
    /// read from the model: the model is scaled to `length`.
    #[serde(default)]
    pub size: Option<Size>,
    /// Placeholder numbers.
    #[serde(default)]
    pub stats: Stats,
    /// Where outfits mount.
    #[serde(default)]
    pub hardpoints: Vec<Hardpoint>,
    /// Mod kits fitted (`mods/<id>.md`), on top of the base's.
    #[serde(default)]
    pub mods: Vec<Id>,
    /// What it costs at a shipyard. A variant that leaves it out costs
    /// its base's, plus the price of each kit it adds.
    #[serde(default)]
    pub price: Option<u32>,
    /// The stellars whose shipyard sells it (`stellars/<id>.md`, each
    /// with the `shipyard` service). Left out or empty, no shipyard sells
    /// it. A variant doesn't inherit its base's.
    #[serde(default)]
    pub sold_at: Vec<Id>,
}

/// A ship's design language. See `mechanics/ships.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Style {
    /// Long, boxy, modular: converted transports.
    Association,
    /// One family of industrial, sleek lines.
    Crown,
    /// Spires, buttresses, thick dark armour.
    Empire,
    /// Small, curved, handcrafted.
    Commonwealth,
    /// Mismatched, welded from other ships.
    Union,
    /// Practical shapes, worn and functional.
    Civilian,
}

/// What a hull is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// Short passenger and cargo hops.
    Shuttle,
    /// Carries cargo.
    Freighter,
    /// Fast and small, for passengers and dispatches.
    Courier,
    /// Fast, agile and fragile.
    Racer,
    /// A small combat ship.
    Fighter,
    /// A fast raider.
    Raider,
    /// A light ship with a big gun.
    Gunboat,
    /// A small warship: patrol, escort, point defence.
    Corvette,
    /// A main line warship.
    Cruiser,
    /// Launches fighters and drones.
    Carrier,
}

impl Role {
    /// The role as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Shuttle => "shuttle",
            Role::Freighter => "freighter",
            Role::Courier => "courier",
            Role::Racer => "racer",
            Role::Fighter => "fighter",
            Role::Raider => "raider",
            Role::Gunboat => "gunboat",
            Role::Corvette => "corvette",
            Role::Cruiser => "cruiser",
            Role::Carrier => "carrier",
        }
    }
}

/// A ship's size in metres, seen from above.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Size {
    /// Nose to tail. The model is scaled to this.
    pub length: f64,
    /// Side to side.
    pub beam: f64,
    /// The radius of the circle the game collides.
    pub collision_radius: f64,
}

/// A ship's placeholder numbers. A variant overrides only those it gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    /// Damage it takes before it breaks up.
    #[serde(default)]
    pub hull: Option<u32>,
    /// Damage its shields absorb.
    #[serde(default)]
    pub shields: Option<u32>,
    /// Top speed.
    #[serde(default)]
    pub speed: Option<u32>,
    /// How well it turns, 0 to 100: the coarse turn stat. It sets the turn
    /// rate only when nobody in the chain gives `turn`.
    #[serde(default)]
    pub handling: Option<u32>,
    /// Tons of cargo.
    #[serde(default)]
    pub cargo: Option<u32>,
    /// How fast it picks up speed, on `speed`'s scale: an acceleration
    /// equal to its `speed` reaches top speed in 50 ticks. Left out, it is
    /// the hull's `speed`.
    #[serde(default)]
    pub acceleration: Option<u32>,
    /// Its turn rate in tenths of a degree a tick (10 is a degree a tick,
    /// about 30° a second). Given anywhere in the chain, it wins over
    /// `handling`. Left out, the turn rate is the `handling`.
    #[serde(default)]
    pub turn: Option<u32>,
    /// The fuel it holds: 100 is one jump. Left out, 300.
    #[serde(default)]
    pub fuel: Option<u32>,
    /// Its crew, a head count. Left out, 1.
    #[serde(default)]
    pub crew: Option<u32>,
    /// How fast its shields regenerate, in thousandths of a point a tick
    /// (300 is 0.3 a tick, 9 a second). Left out, 0: they don't.
    #[serde(default)]
    pub shield_recharge: Option<u32>,
    /// How fast its hull repairs itself, in thousandths of a point a tick.
    /// Left out, 0: it doesn't.
    #[serde(default)]
    pub hull_recharge: Option<u32>,
    /// Its mass in tons, which sets how long a jump takes: under 100 is one
    /// day, 100 to 199 two, 200 and up three. Left out, 0.
    #[serde(default)]
    pub mass: Option<u32>,
}

impl Stats {
    /// These stats over `base`'s: each one this gives, else `base`'s.
    fn over(self, base: Stats) -> Stats {
        Stats {
            hull: self.hull.or(base.hull),
            shields: self.shields.or(base.shields),
            speed: self.speed.or(base.speed),
            handling: self.handling.or(base.handling),
            cargo: self.cargo.or(base.cargo),
            acceleration: self.acceleration.or(base.acceleration),
            turn: self.turn.or(base.turn),
            fuel: self.fuel.or(base.fuel),
            crew: self.crew.or(base.crew),
            shield_recharge: self.shield_recharge.or(base.shield_recharge),
            hull_recharge: self.hull_recharge.or(base.hull_recharge),
            mass: self.mass.or(base.mass),
        }
    }
}

/// A place on a hull where an outfit mounts.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Hardpoint {
    /// Unique within the finished ship.
    pub id: Id,
    /// What can mount here.
    pub mount: Mount,
    /// `[x, y]` in metres from the middle of the ship seen from above:
    /// x toward starboard (right), y toward the nose. It must lie on the
    /// hull: inside `size` and inside the model's footprint. There is no
    /// height: the mount sits on top of the hull there, on the highest
    /// surface under it once the model is scaled to `length` (on the
    /// plane for a placeholder model).
    pub position: [f64; 2],
    /// The outfit fitted here as built (`outfits/<id>.md`).
    #[serde(default)]
    pub outfit: Option<Id>,
}

/// What a hardpoint takes, and what an outfit needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Mount {
    /// A forward-firing gun.
    Fixed,
    /// A gun that turns.
    Turret,
    /// Missiles or torpedoes.
    Launcher,
    /// A launch bay for fighters or drones.
    Bay,
}

impl Mount {
    /// The mount as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Mount::Fixed => "fixed",
            Mount::Turret => "turret",
            Mount::Launcher => "launcher",
            Mount::Bay => "bay",
        }
    }
}

/// A mod kit (`mods/<id>.md`): a change to a hull, with its costs.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModKit {
    /// Display name (a placeholder).
    pub name: String,
    /// The roles of hull it can be fitted to.
    pub fits: Vec<Role>,
    /// Which yards will fit it.
    pub fitted_at: Yard,
    /// What fitting it costs.
    pub price: u32,
    /// What it changes.
    #[serde(default)]
    pub effects: Effects,
    /// Hardpoints it adds.
    #[serde(default)]
    pub hardpoints: Vec<Hardpoint>,
}

/// Which yards fit a mod kit. See "Modification as a system" in
/// `mechanics/ships.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Yard {
    /// Any yard, including the legal yards of the Association and the
    /// Crown.
    Legal,
    /// Only lawless yards: pirates, the Union workshops.
    Lawless,
}

/// What a mod kit changes. Each number is added to the hull's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    /// Change to hull.
    #[serde(default)]
    pub hull: i32,
    /// Change to shields.
    #[serde(default)]
    pub shields: i32,
    /// Change to top speed.
    #[serde(default)]
    pub speed: i32,
    /// Change to handling.
    #[serde(default)]
    pub handling: i32,
    /// Change to cargo, in tons.
    #[serde(default)]
    pub cargo: i32,
    /// Tons of cargo hidden from scans.
    #[serde(default)]
    pub hidden_cargo: u32,
    /// How much more the law notices the ship: scans, suspicion.
    #[serde(default)]
    pub attention: u32,
    /// Change to acceleration, on `speed`'s scale.
    #[serde(default)]
    pub acceleration: i32,
    /// Change to fuel: 100 is one jump.
    #[serde(default)]
    pub fuel: i32,
    /// Change to shield recharge, in thousandths of a point a tick.
    #[serde(default)]
    pub shield_recharge: i32,
    /// Change to hull recharge, in thousandths of a point a tick.
    #[serde(default)]
    pub hull_recharge: i32,
}

/// An outfit (`outfits/<id>.md`): a weapon or system that fits a
/// hardpoint.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Outfit {
    /// Display name (a placeholder).
    pub name: String,
    /// The hardpoint it needs.
    pub mount: Mount,
    /// Damage per hit (a placeholder).
    pub damage: u32,
    /// Range (a placeholder).
    pub range: u32,
    /// Seconds between shots.
    pub reload: f64,
    /// What it costs.
    pub price: u32,
    /// Its model on the hull (`assets/<id>.md`, kind `model`), if it
    /// shows.
    #[serde(default)]
    pub model: Option<Id>,
    /// The stellars whose outfitter sells it (`stellars/<id>.md`, each
    /// with the `outfitter` service). Left out or empty, no outfitter
    /// sells it: it comes only with a mod kit, a hull's hardpoint or a
    /// mission.
    #[serde(default)]
    pub sold_at: Vec<Id>,
}

/// A livery (`liveries/<id>.md`): paint and marks, so one model can serve
/// several owners.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Livery {
    /// Display name (a placeholder).
    pub name: String,
    /// The faction that wears it (`factions/<id>.md`), if any.
    #[serde(default)]
    pub faction: Option<Id>,
    /// The colours the materials its model's `paint` names take.
    pub palette: Palette,
    /// Marks painted on (`assets/<id>.md`, kind `texture`).
    #[serde(default)]
    pub decals: Vec<Id>,
}

/// A livery's colours.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    /// The main hull colour.
    pub primary: Color,
    /// Panels and stripes.
    pub secondary: Color,
    /// Small details and lights.
    pub trim: Color,
}

/// A ship with its `base` chain and mod kits worked out.
#[derive(Debug)]
pub struct Resolved<'a> {
    /// The ship's own name.
    pub name: &'a str,
    /// The ship, then its base, then its base's base, and so on.
    pub chain: Vec<Id>,
    /// The nearest `style` in the chain.
    pub style: Option<Style>,
    /// The nearest `role` in the chain.
    pub role: Option<Role>,
    /// The nearest `model` in the chain.
    pub model: Option<&'a Id>,
    /// The nearest `livery` in the chain.
    pub livery: Option<&'a Id>,
    /// The nearest `size` in the chain.
    pub size: Option<Size>,
    /// Every mod kit fitted, from the root base down. A kit that doesn't
    /// exist is listed but adds nothing.
    pub mods: Vec<Id>,
    /// Every hardpoint: the hull's, then each kit's. Ids may repeat if a
    /// kit reuses one (`check` reports it).
    pub hardpoints: Vec<Placed<'a>>,
    /// The chain's stats merged field by field, the nearest given winning,
    /// before any kit: a stat nobody in the chain gives is `None`.
    pub stats: Stats,
    /// The stats a kit can change, with every kit's effects added. A stat
    /// nobody gives counts as 0.
    pub totals: Totals,
    /// What it costs: the nearest `price` in the chain, plus the price of
    /// every kit added below the ship that gave it (by the ship itself
    /// and each base between). A kit that doesn't exist adds nothing.
    /// `None` when no ship in the chain gives a price. Wide, so a sum of
    /// prices can't overflow.
    pub price: Option<u64>,
}

/// A hardpoint and the ship or mod kit that adds it.
#[derive(Debug)]
pub struct Placed<'a> {
    /// The hardpoint.
    pub hardpoint: &'a Hardpoint,
    /// The ship (`ships/`) or mod kit (`mods/`) that adds it.
    pub from: &'a Id,
}

/// Each number a kit can change, with every kit's effects added. Signed,
/// so a kit that takes away more than the hull has shows as negative.
///
/// A number nobody in the chain gives counts as 0 here, before the kits'
/// effects are added. That differs from what a game falls back to for a
/// hull that leaves the word out (its `speed` for `acceleration`, 300 for
/// `fuel`): `acceleration` and `fuel` here are the chain's own value plus
/// the kits', not what the hull flies with. `turn`, `crew` and `mass` are
/// left out, since no kit changes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    /// Hull.
    pub hull: i64,
    /// Shields.
    pub shields: i64,
    /// Top speed.
    pub speed: i64,
    /// Handling.
    pub handling: i64,
    /// Cargo in tons.
    pub cargo: i64,
    /// Hidden cargo in tons.
    pub hidden_cargo: i64,
    /// How much the law notices it.
    pub attention: i64,
    /// Acceleration.
    pub acceleration: i64,
    /// Fuel: 100 is one jump.
    pub fuel: i64,
    /// Shield recharge, in thousandths of a point a tick.
    pub shield_recharge: i64,
    /// Hull recharge, in thousandths of a point a tick.
    pub hull_recharge: i64,
}

impl Totals {
    /// The chain's merged `stats` before any kit, a stat nobody gives
    /// counting as 0.
    fn of(stats: Stats) -> Totals {
        let value = |v: Option<u32>| i64::from(v.unwrap_or(0));
        Totals {
            hull: value(stats.hull),
            shields: value(stats.shields),
            speed: value(stats.speed),
            handling: value(stats.handling),
            cargo: value(stats.cargo),
            hidden_cargo: 0,
            attention: 0,
            acceleration: value(stats.acceleration),
            fuel: value(stats.fuel),
            shield_recharge: value(stats.shield_recharge),
            hull_recharge: value(stats.hull_recharge),
        }
    }

    /// Adds one kit's effects.
    fn add(&mut self, e: Effects) {
        self.hull += i64::from(e.hull);
        self.shields += i64::from(e.shields);
        self.speed += i64::from(e.speed);
        self.handling += i64::from(e.handling);
        self.cargo += i64::from(e.cargo);
        self.hidden_cargo += i64::from(e.hidden_cargo);
        self.attention += i64::from(e.attention);
        self.acceleration += i64::from(e.acceleration);
        self.fuel += i64::from(e.fuel);
        self.shield_recharge += i64::from(e.shield_recharge);
        self.hull_recharge += i64::from(e.hull_recharge);
    }

    /// Each total with its name, in a fixed order.
    #[must_use]
    pub fn named(&self) -> [(&'static str, i64); 11] {
        [
            ("hull", self.hull),
            ("shields", self.shields),
            ("speed", self.speed),
            ("handling", self.handling),
            ("cargo", self.cargo),
            ("hidden_cargo", self.hidden_cargo),
            ("attention", self.attention),
            ("acceleration", self.acceleration),
            ("fuel", self.fuel),
            ("shield_recharge", self.shield_recharge),
            ("hull_recharge", self.hull_recharge),
        ]
    }
}

/// Why a ship can't be resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolveError {
    /// The `base` chain comes back to a ship already in it. Holds the
    /// chain up to and including the repeat.
    Cycle(Vec<Id>),
    /// A `base` names a ship that doesn't exist.
    MissingBase(Id),
}

/// Works out the ship `id` in `galaxy` from its `base` chain and mods.
pub fn resolve<'a>(galaxy: &'a Galaxy, id: &Id) -> Result<Resolved<'a>, ResolveError> {
    let mut chain: Vec<(&'a Id, &'a Ship)> = Vec::new();
    let mut next = galaxy
        .ships
        .get_key_value(id)
        .ok_or_else(|| ResolveError::MissingBase(id.clone()))?;
    loop {
        if chain.iter().any(|(seen, _)| *seen == next.0) {
            let mut ids: Vec<Id> = chain.iter().map(|(i, _)| (*i).clone()).collect();
            ids.push(next.0.clone());
            return Err(ResolveError::Cycle(ids));
        }
        chain.push((next.0, &next.1.data));
        let Some(base) = &next.1.data.base else { break };
        next = galaxy
            .ships
            .get_key_value(base)
            .ok_or_else(|| ResolveError::MissingBase(base.clone()))?;
    }

    let nearest = |f: &dyn Fn(&'a Ship) -> bool| chain.iter().map(|(_, s)| *s).find(|s| f(s));
    let mut stats = Stats::default();
    let mut hardpoints: Vec<Placed<'a>> = Vec::new();
    let mut mods = Vec::new();
    for (ship_id, ship) in chain.iter().rev() {
        stats = ship.stats.over(stats);
        for hardpoint in &ship.hardpoints {
            let placed = Placed {
                hardpoint,
                from: ship_id,
            };
            match hardpoints
                .iter_mut()
                .find(|p| p.hardpoint.id == hardpoint.id)
            {
                Some(existing) => *existing = placed,
                None => hardpoints.push(placed),
            }
        }
        mods.extend(ship.mods.iter().cloned());
    }
    let mut totals = Totals::of(stats);
    for (kit_id, kit) in mods.iter().filter_map(|m| galaxy.mods.get_key_value(m)) {
        totals.add(kit.data.effects);
        hardpoints.extend(kit.data.hardpoints.iter().map(|hardpoint| Placed {
            hardpoint,
            from: kit_id,
        }));
    }

    let kit_price = |kit: &Id| galaxy.mods.get(kit).map_or(0, |k| u64::from(k.data.price));
    let mut added = 0;
    let mut price = None;
    for (_, ship) in &chain {
        if let Some(own) = ship.price {
            price = Some(u64::from(own) + added);
            break;
        }
        added += ship.mods.iter().map(kit_price).sum::<u64>();
    }

    Ok(Resolved {
        price,
        name: &chain[0].1.name,
        chain: chain.iter().map(|(i, _)| (*i).clone()).collect(),
        style: nearest(&|s| s.style.is_some()).and_then(|s| s.style),
        role: nearest(&|s| s.role.is_some()).and_then(|s| s.role),
        model: nearest(&|s| s.model.is_some()).and_then(|s| s.model.as_ref()),
        livery: nearest(&|s| s.livery.is_some()).and_then(|s| s.livery.as_ref()),
        size: nearest(&|s| s.size.is_some()).and_then(|s| s.size),
        mods,
        hardpoints,
        stats,
        totals,
    })
}

crate::model::kinds! {
    Ship => "ship" in "ships",
    ModKit => "mod" in "mods",
    Outfit => "outfit" in "outfits",
    Livery => "livery" in "liveries",
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;

    fn read<T: serde::de::DeserializeOwned>(yaml: &str) -> Result<T, String> {
        parse::<T>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    #[test]
    fn a_hull_reads_its_size_stats_and_hardpoints() {
        let ship: Ship = read(
            "name: Shuttle\nstyle: civilian\nrole: shuttle\nmodel: civilian-shuttle\n\
             livery: civilian-plain\nsize: {length: 18, beam: 14.5, collision_radius: 9}\n\
             stats: {hull: 120, cargo: 20}\n\
             hardpoints:\n  - id: nose\n    mount: fixed\n    position: [0, 8]\n",
        )
        .unwrap();
        assert_eq!(ship.style, Some(Style::Civilian));
        assert_eq!(ship.size.unwrap().beam, 14.5);
        assert_eq!(ship.stats.cargo, Some(20));
        assert_eq!(ship.stats.speed, None);
        assert_eq!(ship.hardpoints[0].mount, Mount::Fixed);
        assert!(ship.base.is_none() && ship.mods.is_empty());
    }

    #[test]
    fn a_variant_names_its_base_and_mods() {
        let ship: Ship =
            read("name: Technical\nbase: shuttle\nmods: [gun-mount]\nlivery: slogans\n").unwrap();
        assert_eq!(ship.base.unwrap().as_str(), "shuttle");
        assert_eq!(ship.mods[0].as_str(), "gun-mount");
    }

    #[test]
    fn an_unknown_style_or_role_is_an_error() {
        assert!(read::<Ship>("name: X\nstyle: gothic\n").is_err());
        assert!(read::<Ship>("name: X\nrole: battleship\n").is_err());
    }

    #[test]
    fn a_role_and_a_mount_are_named_as_in_data() {
        let roles = [
            (Role::Shuttle, "shuttle"),
            (Role::Freighter, "freighter"),
            (Role::Courier, "courier"),
            (Role::Racer, "racer"),
            (Role::Fighter, "fighter"),
            (Role::Raider, "raider"),
            (Role::Gunboat, "gunboat"),
            (Role::Corvette, "corvette"),
            (Role::Cruiser, "cruiser"),
            (Role::Carrier, "carrier"),
        ];
        for (role, text) in roles {
            assert_eq!(role.as_str(), text);
        }
        let mounts = [
            (Mount::Fixed, "fixed"),
            (Mount::Turret, "turret"),
            (Mount::Launcher, "launcher"),
            (Mount::Bay, "bay"),
        ];
        for (mount, text) in mounts {
            assert_eq!(mount.as_str(), text);
        }
    }

    #[test]
    fn the_totals_are_named_in_a_fixed_order() {
        let totals = Totals {
            hull: 1,
            shields: 2,
            speed: 3,
            handling: 4,
            cargo: 5,
            hidden_cargo: 6,
            attention: 7,
            acceleration: 8,
            fuel: 9,
            shield_recharge: 10,
            hull_recharge: 11,
        };
        assert_eq!(
            totals.named(),
            [
                ("hull", 1),
                ("shields", 2),
                ("speed", 3),
                ("handling", 4),
                ("cargo", 5),
                ("hidden_cargo", 6),
                ("attention", 7),
                ("acceleration", 8),
                ("fuel", 9),
                ("shield_recharge", 10),
                ("hull_recharge", 11),
            ]
        );
    }

    #[test]
    fn a_mod_kit_reads_its_effects() {
        let kit: ModKit = read(
            "name: Gun mount\nfits: [shuttle, freighter]\nfitted_at: lawless\nprice: 4000\n\
             effects: {cargo: -6, handling: -10, attention: 2}\n\
             hardpoints:\n  - id: bed\n    mount: turret\n    position: [0, -3]\n",
        )
        .unwrap();
        assert_eq!(kit.fitted_at, Yard::Lawless);
        assert_eq!(kit.effects.cargo, -6);
        assert_eq!(kit.effects.hull, 0);
        assert_eq!(kit.effects.attention, 2);
        assert_eq!(kit.hardpoints[0].mount, Mount::Turret);
    }

    #[test]
    fn a_hull_reads_its_acceleration_turn_fuel_crew_recharges_and_mass() {
        let ship: Ship = read(
            "name: Hauler\nstats: {acceleration: 500, turn: 60, fuel: 400, crew: 3, \
             shield_recharge: 300, hull_recharge: 50, mass: 80}\n",
        )
        .unwrap();
        let s = ship.stats;
        assert_eq!(
            (s.acceleration, s.turn, s.fuel, s.crew),
            (Some(500), Some(60), Some(400), Some(3))
        );
        assert_eq!(
            (s.shield_recharge, s.hull_recharge, s.mass),
            (Some(300), Some(50), Some(80))
        );
    }

    #[test]
    fn a_mod_kit_reads_its_acceleration_fuel_and_recharge_effects() {
        let kit: ModKit = read(
            "name: Tune\nfits: [shuttle]\nfitted_at: legal\nprice: 10\n\
             effects: {acceleration: -50, fuel: 100, shield_recharge: 200, hull_recharge: 20}\n",
        )
        .unwrap();
        let e = kit.effects;
        assert_eq!(
            (e.acceleration, e.fuel, e.shield_recharge, e.hull_recharge),
            (-50, 100, 200, 20)
        );
        assert_eq!(e.speed, 0);
    }

    #[test]
    fn an_outfit_reads_its_mount() {
        let outfit: Outfit = read(
            "name: Mass driver\nmount: turret\ndamage: 12\nrange: 600\nreload: 0.5\n\
             price: 2500\nmodel: turret-mount\n",
        )
        .unwrap();
        assert_eq!(outfit.mount, Mount::Turret);
        assert_eq!(outfit.model.unwrap().as_str(), "turret-mount");
        assert!(outfit.sold_at.is_empty(), "sold nowhere when left out");
    }

    #[test]
    fn an_outfit_lists_the_stellars_that_sell_it() {
        let outfit: Outfit = read(
            "name: Mass driver\nmount: turret\ndamage: 12\nrange: 600\nreload: 0.5\n\
             price: 2500\nsold_at: [marrow-yard, luyten-exchange]\n",
        )
        .unwrap();
        let yards: Vec<&str> = outfit.sold_at.iter().map(Id::as_str).collect();
        assert_eq!(yards, ["marrow-yard", "luyten-exchange"]);
    }

    #[test]
    fn an_outfit_sold_at_a_malformed_id_is_an_error() {
        let read = read::<Outfit>(
            "name: Mass driver\nmount: turret\ndamage: 12\nrange: 600\nreload: 0.5\n\
             price: 2500\nsold_at: [Marrow Yard]\n",
        );
        assert!(read.is_err(), "{read:?}");
    }

    #[test]
    fn a_hull_reads_its_price_and_the_stellars_that_sell_it() {
        let ship: Ship =
            read("name: Cutter\nprice: 40000\nsold_at: [hearth-dock, marrow-yard]\n").unwrap();
        assert_eq!(ship.price, Some(40000));
        let yards: Vec<&str> = ship.sold_at.iter().map(Id::as_str).collect();
        assert_eq!(yards, ["hearth-dock", "marrow-yard"]);

        let bare: Ship = read("name: Cutter\n").unwrap();
        assert_eq!(bare.price, None);
        assert!(bare.sold_at.is_empty(), "sold nowhere when left out");
    }

    #[test]
    fn a_ship_sold_at_a_malformed_id_is_an_error() {
        let read = read::<Ship>("name: Cutter\nprice: 40000\nsold_at: [Hearth Dock]\n");
        assert!(read.is_err(), "{read:?}");
    }

    #[test]
    fn a_livery_reads_its_palette() {
        let livery: Livery = read(
            "name: Registry\nfaction: association\n\
             palette: {primary: \"#e8ecf0\", secondary: \"#2f7fd1\", trim: \"#f2b134\"}\n\
             decals: [registry-numbers]\n",
        )
        .unwrap();
        assert_eq!(livery.palette.secondary.as_str(), "#2f7fd1");
        assert_eq!(livery.decals.len(), 1);
    }
    mod resolving {
        use super::*;
        use crate::fixture;
        use crate::load::{Galaxy, load};

        fn galaxy_with(extra: &[(&str, &str)]) -> Galaxy {
            let mut source = fixture::galaxy();
            for (path, text) in extra {
                source = source.with(path, text);
            }
            let (galaxy, problems) = load(&source);
            assert!(problems.is_empty(), "{problems:?}");
            galaxy
        }

        fn id(s: &str) -> Id {
            Id::try_from(s.to_string()).unwrap()
        }

        #[test]
        fn a_variant_inherits_what_it_leaves_out() {
            let galaxy = galaxy_with(&[(
                "ships/fast-skiff.md",
                "---\nname: Fast skiff\nbase: skiff\nstats: {speed: 400}\n---\n",
            )]);
            let ship = resolve(&galaxy, &id("fast-skiff")).unwrap();
            assert_eq!(ship.name, "Fast skiff");
            assert_eq!(ship.chain, [id("fast-skiff"), id("skiff")]);
            assert_eq!(ship.role, Some(Role::Shuttle));
            assert_eq!(ship.model.map(Id::as_str), Some("hull"));
            assert_eq!(ship.size.unwrap().length, 8.0);
            assert_eq!(ship.totals.speed, 400);
            assert_eq!(ship.totals.hull, 50, "merged field by field");
        }

        #[test]
        fn a_two_step_chain_merges_its_stats_field_by_field_nearest_first() {
            let galaxy = galaxy_with(&[
                (
                    "ships/fast-skiff.md",
                    "---\nname: Fast skiff\nbase: skiff\nstats: {speed: 400, cargo: 4}\n---\n",
                ),
                (
                    "ships/faster-skiff.md",
                    "---\nname: Faster skiff\nbase: fast-skiff\nstats: {speed: 500}\n---\n",
                ),
            ]);
            let ship = resolve(&galaxy, &id("faster-skiff")).unwrap();
            assert_eq!(
                ship.stats,
                Stats {
                    hull: Some(50),
                    shields: None,
                    speed: Some(500),
                    handling: None,
                    cargo: Some(4),
                    acceleration: None,
                    turn: None,
                    fuel: None,
                    crew: None,
                    shield_recharge: None,
                    hull_recharge: None,
                    mass: None,
                }
            );
        }

        #[test]
        fn a_variant_inherits_its_base_s_hull_numbers_and_overrides_those_it_gives() {
            let galaxy = galaxy_with(&[
                (
                    "ships/heavy-skiff.md",
                    "---\nname: Heavy skiff\nbase: skiff\nstats: {acceleration: 500, turn: 60, \
                     fuel: 400, crew: 3, shield_recharge: 300, hull_recharge: 50, mass: 80}\n---\n",
                ),
                (
                    "ships/heavier-skiff.md",
                    "---\nname: Heavier skiff\nbase: heavy-skiff\nstats: {mass: 150}\n---\n",
                ),
            ]);
            let s = resolve(&galaxy, &id("heavier-skiff")).unwrap().stats;
            assert_eq!(
                (s.acceleration, s.turn, s.fuel, s.crew),
                (Some(500), Some(60), Some(400), Some(3))
            );
            assert_eq!(
                (s.shield_recharge, s.hull_recharge, s.mass),
                (Some(300), Some(50), Some(150))
            );
        }

        #[test]
        fn a_variant_naming_only_handling_keeps_its_base_s_turn() {
            let galaxy = galaxy_with(&[
                (
                    "ships/turning-skiff.md",
                    "---\nname: Turning skiff\nbase: skiff\nstats: {turn: 60}\n---\n",
                ),
                (
                    "ships/handy-skiff.md",
                    "---\nname: Handy skiff\nbase: turning-skiff\nstats: {handling: 80}\n---\n",
                ),
            ]);
            let s = resolve(&galaxy, &id("handy-skiff")).unwrap().stats;
            assert_eq!((s.turn, s.handling), (Some(60), Some(80)));
        }

        #[test]
        fn a_kit_adds_its_acceleration_fuel_and_recharge_effects_to_the_totals() {
            let galaxy = galaxy_with(&[
                (
                    "mods/tune-kit.md",
                    "---\nname: Tune kit\nfits: [shuttle]\nfitted_at: legal\nprice: 10\n\
                     effects: {acceleration: -50, fuel: 100, shield_recharge: 200, \
                     hull_recharge: 20}\n---\n",
                ),
                (
                    "ships/tuned-skiff.md",
                    "---\nname: Tuned\nbase: skiff\nstats: {acceleration: 360, fuel: 400, \
                     shield_recharge: 300}\nmods: [tune-kit]\n---\n",
                ),
            ]);
            let t = resolve(&galaxy, &id("tuned-skiff")).unwrap().totals;
            assert_eq!(
                (t.acceleration, t.fuel, t.shield_recharge, t.hull_recharge),
                (310, 500, 500, 20),
                "a recharge nobody gives counts as 0"
            );
        }

        #[test]
        fn a_total_nobody_in_the_chain_gives_counts_as_0_not_the_game_s_default() {
            let galaxy = galaxy_with(&[
                (
                    "mods/slow-kit.md",
                    "---\nname: Slow kit\nfits: [shuttle]\nfitted_at: legal\nprice: 10\n\
                     effects: {acceleration: -50}\n---\n",
                ),
                (
                    "ships/slow-skiff.md",
                    "---\nname: Slow\nbase: skiff\nstats: {speed: 300}\nmods: [slow-kit]\n---\n",
                ),
            ]);
            let t = resolve(&galaxy, &id("slow-skiff")).unwrap().totals;
            assert_eq!(
                (t.acceleration, t.fuel),
                (-50, 0),
                "not speed - 50, not 300"
            );
        }

        #[test]
        fn a_kit_changes_the_totals_but_not_the_stats() {
            let galaxy = galaxy_with(&[]);
            let ship = resolve(&galaxy, &id("armed-skiff")).unwrap();
            assert_eq!(ship.stats.cargo, Some(10), "the chain's own");
            assert_eq!(ship.totals.cargo, 8, "the kit's -2 added");
        }

        #[test]
        fn a_variant_hardpoint_replaces_the_base_one_with_its_id() {
            let galaxy = galaxy_with(&[(
                "ships/odd-skiff.md",
                "---\nname: Odd\nbase: skiff\nhardpoints:\n  - id: nose\n    mount: launcher\n\
                 \x20   position: [0, 2]\n  - id: tail\n    mount: fixed\n    position: [0, -3]\n---\n",
            )]);
            let ship = resolve(&galaxy, &id("odd-skiff")).unwrap();
            let mounts: Vec<_> = ship
                .hardpoints
                .iter()
                .map(|p| (p.hardpoint.id.as_str(), p.hardpoint.mount, p.from.as_str()))
                .collect();
            assert_eq!(
                mounts,
                [
                    ("nose", Mount::Launcher, "odd-skiff"),
                    ("tail", Mount::Fixed, "odd-skiff")
                ]
            );
        }

        #[test]
        fn mods_add_their_hardpoints_and_effects() {
            let galaxy = galaxy_with(&[]);
            let ship = resolve(&galaxy, &id("armed-skiff")).unwrap();
            assert_eq!(ship.mods, [id("gun-mount")]);
            assert_eq!(ship.totals.cargo, 8);
            assert_eq!(ship.totals.attention, 2);
            let from: Vec<_> = ship.hardpoints.iter().map(|p| p.from.as_str()).collect();
            assert_eq!(from, ["skiff", "gun-mount"]);
        }

        #[test]
        fn a_kit_adds_each_of_its_effects_to_the_hull_s() {
            let galaxy = galaxy_with(&[
                (
                    "mods/tune-kit.md",
                    "---\nname: Tune kit\nfits: [shuttle]\nfitted_at: legal\nprice: 10\n\
                     effects: {hull: 5, shields: 3, speed: 7, handling: 2, hidden_cargo: 4}\n---\n",
                ),
                (
                    "ships/tuned-skiff.md",
                    "---\nname: Tuned\nbase: skiff\nstats: {shields: 20, speed: 300, handling: 40}\n\
                     mods: [tune-kit]\n---\n",
                ),
            ]);
            let totals = resolve(&galaxy, &id("tuned-skiff")).unwrap().totals;
            assert_eq!(
                totals,
                Totals {
                    hull: 55,
                    shields: 23,
                    speed: 307,
                    handling: 42,
                    cargo: 10,
                    hidden_cargo: 4,
                    attention: 0,
                    acceleration: 0,
                    fuel: 0,
                    shield_recharge: 0,
                    hull_recharge: 0,
                }
            );
        }

        const RACK: (&str, &str) = (
            "mods/rack.md",
            "---\nname: Rack\nfits: [shuttle]\nfitted_at: legal\nprice: 30\n---\n",
        );
        const PLATE: (&str, &str) = (
            "mods/plate.md",
            "---\nname: Plate\nfits: [shuttle]\nfitted_at: legal\nprice: 7\n---\n",
        );

        fn price_of(extra: &[(&str, &str)], ship: &str) -> Option<u64> {
            resolve(&galaxy_with(extra), &id(ship)).unwrap().price
        }

        #[test]
        fn a_hull_s_price_is_its_own() {
            let price = price_of(
                &[(
                    "ships/priced.md",
                    "---\nname: Priced\nbase: skiff\nprice: 900\n---\n",
                )],
                "priced",
            );
            assert_eq!(price, Some(900));
        }

        #[test]
        fn a_variant_with_no_price_costs_its_base_s_and_its_own_kits() {
            let price = price_of(
                &[
                    RACK,
                    PLATE,
                    (
                        "ships/priced.md",
                        "---\nname: Priced\nbase: skiff\nprice: 900\nmods: [rack]\n---\n",
                    ),
                    (
                        "ships/plated.md",
                        "---\nname: Plated\nbase: priced\nmods: [plate]\n---\n",
                    ),
                ],
                "plated",
            );
            assert_eq!(price, Some(907), "the base carries its rack already");
        }

        #[test]
        fn a_variant_s_own_price_wins_and_adds_no_kits() {
            let price = price_of(
                &[
                    PLATE,
                    (
                        "ships/priced.md",
                        "---\nname: Priced\nbase: skiff\nprice: 900\n---\n",
                    ),
                    (
                        "ships/plated.md",
                        "---\nname: Plated\nbase: priced\nprice: 1200\nmods: [plate]\n---\n",
                    ),
                ],
                "plated",
            );
            assert_eq!(price, Some(1200));
        }

        #[test]
        fn a_grand_variant_with_no_price_adds_the_kits_of_every_ship_below_the_price() {
            let price = price_of(
                &[
                    RACK,
                    PLATE,
                    (
                        "ships/priced.md",
                        "---\nname: Priced\nbase: skiff\nprice: 900\n---\n",
                    ),
                    (
                        "ships/racked.md",
                        "---\nname: Racked\nbase: priced\nmods: [rack]\n---\n",
                    ),
                    (
                        "ships/plated.md",
                        "---\nname: Plated\nbase: racked\nmods: [plate]\n---\n",
                    ),
                ],
                "plated",
            );
            assert_eq!(price, Some(937));
        }

        #[test]
        fn a_kit_not_in_mods_adds_nothing_to_the_price() {
            let price = price_of(
                &[
                    PLATE,
                    (
                        "ships/priced.md",
                        "---\nname: Priced\nbase: skiff\nprice: 900\n---\n",
                    ),
                    (
                        "ships/odd.md",
                        "---\nname: Odd\nbase: priced\nmods: [plate, zeppelin-kit]\n---\n",
                    ),
                ],
                "odd",
            );
            assert_eq!(price, Some(907));
        }

        #[test]
        fn a_chain_with_no_price_has_none() {
            assert_eq!(price_of(&[], "armed-skiff"), None);
            assert_eq!(price_of(&[], "skiff"), None);
        }

        #[test]
        fn a_base_chain_that_loops_is_an_error() {
            let galaxy = galaxy_with(&[
                ("ships/a.md", "---\nname: A\nbase: b\n---\n"),
                ("ships/b.md", "---\nname: B\nbase: a\n---\n"),
            ]);
            let err = resolve(&galaxy, &id("a")).unwrap_err();
            assert_eq!(err, ResolveError::Cycle(vec![id("a"), id("b"), id("a")]));
        }

        #[test]
        fn a_missing_base_is_an_error() {
            let galaxy = galaxy_with(&[("ships/a.md", "---\nname: A\nbase: zeppelin\n---\n")]);
            assert_eq!(
                resolve(&galaxy, &id("a")).unwrap_err(),
                ResolveError::MissingBase(id("zeppelin"))
            );
        }
    }
}
