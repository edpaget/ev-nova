//! One type per kind of content file. Each is the shape of the file's front
//! matter; the Markdown body is kept alongside as prose.
//!
//! These types are the source of truth for the format: the JSON Schemas in
//! `data/schema/` are generated from them (`galaxy schema`), and a test
//! fails if the committed schemas fall behind.

use schemars::JsonSchema;
use serde::Deserialize;

use crate::id::{Color, DialogRef, Flag, Fraction, Harshness, Id, PointRef};

/// An `[x, y]` position. On the galaxy map and inside a system, x grows
/// rightward and y grows downward.
pub type Position = [i32; 2];

/// A region (`regions/<id>.md`): a stretch of space with one harshness.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Region {
    /// Display name (a placeholder).
    pub name: String,
    /// The harshness of every system in the region, unless a system
    /// overrides it.
    pub harshness: Harshness,
    /// The region's tint on the generated map.
    pub color: Color,
}

/// A faction (`factions/<id>.md`): a major power, or a minor or local
/// group that can own systems, stations and jump points.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Faction {
    /// Display name (a placeholder).
    pub name: String,
    /// Major power or minor faction.
    pub kind: FactionKind,
    /// The faction's colour on the generated map.
    pub color: Color,
}

/// Major power or minor faction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum FactionKind {
    /// One of the five powers.
    Major,
    /// Anything smaller: leagues, warlords, enclaves, communities.
    Minor,
}

/// A star system (`systems/<id>.md`).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct System {
    /// Display name (a placeholder).
    pub name: String,
    /// The region it lies in (`regions/<id>.md`).
    pub region: Id,
    /// The faction that governs it (`factions/<id>.md`). Omit for space
    /// nobody governs.
    #[serde(default)]
    pub government: Option<Id>,
    /// Overrides the region's harshness for this system.
    #[serde(default)]
    pub harshness: Option<Harshness>,
    /// Position on the galaxy map.
    pub map: Position,
    /// Effects that apply everywhere in the system.
    #[serde(default)]
    pub hazards: Vec<Hazard>,
    /// The specific places where ships enter and leave hyperspace.
    #[serde(default)]
    pub jump_points: Vec<JumpPoint>,
}

/// A system-wide hazard. See `mechanics/systems-and-regions.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Hazard {
    /// Drains shields.
    Radiation,
    /// Disables secondary weapons for a time.
    IonStorm,
    /// Radar shows only what is in visual range.
    SensorDead,
    /// Flares sweep the system on a schedule.
    SolarFlares,
    /// Gravity around the star bends ships and shots.
    GravityWell,
    /// Fog that cuts everyone's sensors.
    Nebula,
}

/// One end of a jump link: a place in a system where ships jump out to
/// another system's point, and arrive from it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JumpPoint {
    /// Unique within the system.
    pub id: Id,
    /// Where the point sits in the system.
    pub position: Position,
    /// The point ships arrive at, written `system/point`. That point must
    /// lead back here, unless this one is `one_way`.
    pub to: PointRef,
    /// How well jumps through this point work, out or in. A jump uses
    /// the worse of its two ends.
    pub stability: Stability,
    /// Who holds the point (`factions/<id>.md`). Omit if nobody does.
    #[serde(default)]
    pub owner: Option<Id>,
    /// The link runs only from here: the arrival point leads somewhere
    /// else, or nowhere.
    #[serde(default)]
    pub one_way: bool,
}

/// How well jumps through a point work. A jump between two points uses
/// the worse of their two stabilities, whichever end it leaves from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Stability {
    /// Jumps through it always work.
    Stable,
    /// Jumps through it sometimes fail. A failed jump doesn't happen: the
    /// ship stays at the point it jumped from.
    Unstable,
    /// Jumps through it never happen.
    Dead,
}

impl Stability {
    /// The stability as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Stability::Stable => "stable",
            Stability::Unstable => "unstable",
            Stability::Dead => "dead",
        }
    }

    /// The less stable of the two: stable, then unstable, then dead.
    #[must_use]
    pub fn worse(self, other: Stability) -> Stability {
        self.max(other)
    }
}

/// How well a jump from the point `from` to the point `to` works: as well
/// as the worse of its two ends. This holds for two-way and one-way links
/// alike, so an unstable or dead arrival point affects jumps in. How
/// often an unstable jump fails is engine tuning, not data.
#[must_use]
pub fn jump_stability(from: &JumpPoint, to: &JumpPoint) -> Stability {
    from.stability.worse(to.stability)
}

/// A planet, moon, station or outpost (`stellars/<id>.md`).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stellar {
    /// Display name (a placeholder).
    pub name: String,
    /// What it is.
    pub kind: StellarKind,
    /// The system it is in (`systems/<id>.md`).
    pub system: Id,
    /// Where it sits in the system.
    pub position: Position,
    /// Who runs it (`factions/<id>.md`). Omit if nobody does.
    #[serde(default)]
    pub owner: Option<Id>,
    /// What a landed pilot can use.
    #[serde(default)]
    pub services: Vec<Service>,
    /// How it looks: a generated look (planets and moons) or an asset
    /// (`assets/<id>.md`: a texture for a planet or moon, a model for a
    /// station or outpost).
    #[serde(default)]
    pub visual: Option<Visual>,
}

/// How a stellar looks: one or the other.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Visual {
    /// Drawn by a planet shader from these parameters.
    Generated(Look),
    /// A specific piece of art (`assets/<id>.md`).
    Asset(Id),
}

/// The parameters of a generated planet or moon. Provisional until a
/// planet shader settles them: `data/README.md` ("Generated planets:
/// left for the engine") lists what is still open.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// The kind of world the shader draws.
    #[serde(rename = "type")]
    pub kind: PlanetType,
    /// One to four colours, from most to least of the surface.
    pub palette: Vec<Color>,
    /// How much of the surface clouds cover.
    #[serde(default = "no_fraction")]
    pub clouds: Fraction,
    /// Rings, if it has any.
    #[serde(default)]
    pub rings: Option<Rings>,
    /// A glow around the limb, if it has an atmosphere.
    #[serde(default)]
    pub atmosphere: Option<Atmosphere>,
    /// Picks one world among those the parameters allow.
    pub seed: u32,
}

fn no_fraction() -> Fraction {
    Fraction::try_from(0.0).expect("0 is a fraction")
}

/// The kind of world a planet shader draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlanetType {
    /// Bare rock and craters.
    Rocky,
    /// Sand and dry rock.
    Desert,
    /// Mostly water.
    Ocean,
    /// Land, water and life.
    Garden,
    /// Ice and snow.
    Ice,
    /// Molten.
    Lava,
    /// Banded clouds, no surface.
    GasGiant,
}

impl PlanetType {
    /// The type as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PlanetType::Rocky => "rocky",
            PlanetType::Desert => "desert",
            PlanetType::Ocean => "ocean",
            PlanetType::Garden => "garden",
            PlanetType::Ice => "ice",
            PlanetType::Lava => "lava",
            PlanetType::GasGiant => "gas-giant",
        }
    }
}

/// A planet's rings, measured in planet radii from its centre.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rings {
    /// Where the rings start, more than 1.
    pub inner: f64,
    /// Where they end, more than `inner`.
    pub outer: f64,
    /// Their colour.
    pub color: Color,
}

/// A planet's atmosphere.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Atmosphere {
    /// The glow's colour.
    pub color: Color,
    /// How thick it is, as a fraction of the planet's radius.
    pub thickness: Fraction,
}

/// What a stellar body is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StellarKind {
    /// A planet.
    Planet,
    /// A moon.
    Moon,
    /// A station.
    Station,
    /// A small outpost: a rock, a hulk, a camp.
    Outpost,
}

/// A service at a stellar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Service {
    /// Bar: rumours, people, some missions.
    Bar,
    /// Commodity exchange.
    Trade,
    /// Outfitter.
    Outfitter,
    /// Shipyard.
    Shipyard,
    /// Fuel.
    Refuel,
}

/// A storyline (`storylines/<id>.md`): a thread of missions.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Storyline {
    /// Display title.
    pub title: String,
    /// Main, minor, or the shared spine every playthrough sees.
    pub kind: StorylineKind,
    /// The faction it belongs to (`factions/<id>.md`), if any.
    #[serde(default)]
    pub faction: Option<Id>,
}

/// What kind of storyline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StorylineKind {
    /// A main storyline: exclusive, committed at its crossroads.
    Main,
    /// A minor storyline.
    Minor,
    /// Shared events every playthrough sees, such as the first failure.
    Spine,
}

impl StorylineKind {
    /// The kind as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            StorylineKind::Main => "main",
            StorylineKind::Minor => "minor",
            StorylineKind::Spine => "spine",
        }
    }
}

/// A mission (`missions/<id>.md`).
///
/// A mission with an `offer` is offered at a stellar once its `requires`
/// hold. A mission without one is an event: it happens by itself once its
/// `requires` hold. A mission with no `objective` is done when accepted
/// (or, for an event, when it happens).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Mission {
    /// Display title.
    pub title: String,
    /// The storyline it belongs to (`storylines/<id>.md`).
    pub storyline: Id,
    /// Which part of the storyline it falls in.
    pub act: Act,
    /// What must hold before it is offered or happens.
    #[serde(default)]
    pub requires: Conditions,
    /// Where, by whom and with what dialog it is offered. Omit for an event.
    #[serde(default)]
    pub offer: Option<Offer>,
    /// What the player must do.
    #[serde(default)]
    pub objective: Option<Objective>,
    /// The dialog when the objective is done.
    #[serde(default)]
    pub complete: Option<Completion>,
    /// State set when the offer is made (or the event happens).
    #[serde(default)]
    pub on_offer: Effects,
    /// State set when the player accepts.
    #[serde(default)]
    pub on_accept: Effects,
    /// State set when the player declines.
    #[serde(default)]
    pub on_decline: Effects,
    /// State set when the objective is done.
    #[serde(default)]
    pub on_success: Effects,
}

impl Mission {
    /// Its effects, each with its field name.
    #[must_use]
    pub fn effects(&self) -> [(&'static str, &Effects); 4] {
        [
            ("on_offer", &self.on_offer),
            ("on_accept", &self.on_accept),
            ("on_decline", &self.on_decline),
            ("on_success", &self.on_success),
        ]
    }
}

/// Which part of a storyline a mission falls in. See `story/structure.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Act {
    /// Available to everyone, locking nothing.
    Open,
    /// The offer that commits the player, and what surrounds it.
    Crossroads,
    /// After the player commits.
    Committed,
}

impl Act {
    /// Every act, in the order a storyline goes through them.
    pub const ALL: [Act; 3] = [Act::Open, Act::Crossroads, Act::Committed];

    /// The act as written in data.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Act::Open => "open",
            Act::Crossroads => "crossroads",
            Act::Committed => "committed",
        }
    }
}

/// Where and how a mission is offered.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    /// The stellar it is offered at (`stellars/<id>.md`).
    pub at: Id,
    /// The character who offers it (`characters/<id>.md`), if anyone in
    /// particular.
    #[serde(default)]
    pub giver: Option<Id>,
    /// The offer's dialog. Every path ends in an outcome tag:
    /// `# outcome: accept`, `# outcome: decline` or `# outcome: defer`.
    pub dialog: DialogRef,
}

/// How an offer's dialog ends: the value of the Ink tag
/// `# outcome: <value>` on the path the player took. The game then
/// applies `on_accept` or `on_decline`, or offers the mission again later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The player takes the mission (`on_accept`).
    Accept,
    /// The player turns it down (`on_decline`).
    Decline,
    /// Ask again later: nothing is applied.
    Defer,
}

impl Outcome {
    /// Every outcome, in the order they are listed in messages.
    pub const ALL: [Outcome; 3] = [Outcome::Accept, Outcome::Decline, Outcome::Defer];

    /// The outcome as written in the tag.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Accept => "accept",
            Outcome::Decline => "decline",
            Outcome::Defer => "defer",
        }
    }

    /// The outcome a tag value names, if it names one.
    #[must_use]
    pub fn parse(value: &str) -> Option<Outcome> {
        Outcome::ALL.into_iter().find(|o| o.as_str() == value)
    }
}

/// The dialog when a mission's objective is done.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Completion {
    /// The stellar the player returns to (`stellars/<id>.md`), if any.
    #[serde(default)]
    pub at: Option<Id>,
    /// The dialog.
    pub dialog: DialogRef,
}

/// What a mission asks the player to do.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Objective {
    /// Carry cargo to a stellar.
    Deliver {
        /// What the cargo is.
        cargo: String,
        /// How much.
        tons: u32,
        /// The stellar to deliver to (`stellars/<id>.md`).
        to: Id,
    },
    /// Take a jump point from whoever holds it.
    TakePoint {
        /// The point (`system/point`).
        point: PointRef,
        /// The faction holding it (`factions/<id>.md`).
        held_by: Id,
    },
}

/// Conditions that must all hold. `all`, `any` and `none` nest further
/// groups of conditions, each one ANDed inside like this one. An empty
/// list is the same as leaving it out.
// The doc is the schema's description, which game-bible commits, so it
// keeps its wording.
#[allow(clippy::doc_markdown)]
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Conditions {
    /// Missions that must be done (`missions/<id>.md`).
    #[serde(default)]
    pub missions_done: Vec<Id>,
    /// Flags that must be set.
    #[serde(default)]
    pub flags: Vec<Flag>,
    /// Flags that must not be set.
    #[serde(default)]
    pub flags_unset: Vec<Flag>,
    /// Jump points that must be in a given state.
    #[serde(default)]
    pub points: Vec<PointState>,
    /// The earliest day of the game it can happen.
    #[serde(default)]
    pub not_before_day: Option<u32>,
    /// Groups that must every one hold. Use it to combine two `any`
    /// groups, since a YAML map can't hold two `any` keys.
    #[serde(default)]
    pub all: Vec<Conditions>,
    /// Groups of which at least one must hold (OR).
    #[serde(default)]
    pub any: Vec<Conditions>,
    /// Groups of which none may hold (NOT). For a single flag, use
    /// `flags_unset`.
    #[serde(default)]
    pub none: Vec<Conditions>,
}

impl Conditions {
    /// This group and every group nested in it, depth first, each with its
    /// path from here: `""` for this one, then `"all[0]"`, `"any[1]"`,
    /// `"any[1].none[0]"` and so on, `all` before `any` before `none`.
    #[must_use]
    pub fn groups(&self) -> Vec<(String, &Conditions)> {
        let mut groups = vec![(String::new(), self)];
        for (name, list) in [("all", &self.all), ("any", &self.any), ("none", &self.none)] {
            for (i, group) in list.iter().enumerate() {
                for (path, nested) in group.groups() {
                    let at = format!("{name}[{i}]");
                    let path = if path.is_empty() {
                        at
                    } else {
                        format!("{at}.{path}")
                    };
                    groups.push((path, nested));
                }
            }
        }
        groups
    }
}

/// World state a mission sets.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    /// Flags to set.
    #[serde(default)]
    pub flags: Vec<Flag>,
    /// Flags to clear.
    #[serde(default)]
    pub clear_flags: Vec<Flag>,
    /// Jump points whose owner or stability changes.
    #[serde(default)]
    pub points: Vec<PointState>,
    /// News that breaks when this happens (`news/<id>.md`), spreading
    /// from its `from` system.
    #[serde(default)]
    pub news: Vec<Id>,
}

/// A piece of news (`news/<id>.md`), released by a mission's effects. The
/// body is the news as told.
///
/// It starts in one system and travels on ships. The engine works out the
/// day each system hears it from the jump graph, so no arrival days are
/// written here.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct News {
    /// Its headline (a placeholder).
    pub headline: String,
    /// The system where it breaks (`systems/<id>.md`).
    pub from: Id,
}

/// A jump point's owner, stability or both: a condition to test or a change
/// to make. At least one of the two is given.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointState {
    /// The point (`system/point`).
    pub point: PointRef,
    /// The owner (`factions/<id>.md`).
    #[serde(default)]
    pub owner: Option<Id>,
    /// The stability.
    #[serde(default)]
    pub stability: Option<Stability>,
}

/// A named character (`characters/<id>.md`).
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Character {
    /// Display name (a placeholder).
    pub name: String,
    /// Pronouns, e.g. `they/them`.
    pub pronouns: String,
    /// Their part in the world, in a few words.
    pub role: String,
    /// The faction they belong to (`factions/<id>.md`), if any.
    #[serde(default)]
    pub faction: Option<Id>,
    /// Where they are usually found (`stellars/<id>.md`).
    #[serde(default)]
    pub home: Option<Id>,
    /// Whether they can join the crew.
    #[serde(default)]
    pub companion: bool,
    /// Storylines they appear in (`storylines/<id>.md`).
    #[serde(default)]
    pub storylines: Vec<Id>,
}

/// A start (`starts/<id>.md`): where and in what a new pilot begins. The
/// game offers the first start, in id order.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Start {
    /// Display name (a placeholder).
    pub name: String,
    /// The system the pilot starts in (`systems/<id>.md`).
    pub system: Id,
    /// The ship the pilot starts in (`ships/<id>.md`): a hull, with no
    /// `base` and no `mods`.
    pub ship: Id,
    /// The credits the pilot starts with.
    pub cash: u32,
}

/// A kind of content file: one folder of the data folder, one file per item.
pub trait Kind {
    /// The kind's name in messages, e.g. `system`.
    const NAME: &'static str;
    /// Its folder in the data folder, e.g. `systems`.
    const DIR: &'static str;
}

/// Implements [`Kind`] for each type: `Type => "name" in "folder"`.
macro_rules! kinds {
    ($($t:ident => $name:literal in $dir:literal),* $(,)?) => {
        $(impl $crate::model::Kind for $t {
            const NAME: &'static str = $name;
            const DIR: &'static str = $dir;
        })*
    };
}
pub(crate) use kinds;

kinds! {
    Region => "region" in "regions",
    Faction => "faction" in "factions",
    System => "system" in "systems",
    Stellar => "stellar" in "stellars",
    Storyline => "storyline" in "storylines",
    Mission => "mission" in "missions",
    News => "news" in "news",
    Character => "character" in "characters",
    Start => "start" in "starts",
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
    fn a_planet_type_is_named_as_in_data() {
        let types = [
            (PlanetType::Rocky, "rocky"),
            (PlanetType::Desert, "desert"),
            (PlanetType::Ocean, "ocean"),
            (PlanetType::Garden, "garden"),
            (PlanetType::Ice, "ice"),
            (PlanetType::Lava, "lava"),
            (PlanetType::GasGiant, "gas-giant"),
        ];
        for (kind, text) in types {
            assert_eq!(kind.as_str(), text);
        }
    }

    #[test]
    fn a_system_reads_its_jump_points() {
        let system: System = read(
            "name: Marrow\nregion: commonwealth-rim\ngovernment: commonwealth\nmap: [10, 20]\n\
             hazards: [ion-storm]\njump_points:\n  - id: far-gate\n    position: [900, -200]\n\
             \x20   to: brightwater/marrow-gate\n    stability: unstable\n",
        )
        .unwrap();
        assert_eq!(system.government.unwrap().as_str(), "commonwealth");
        assert_eq!(system.hazards, [Hazard::IonStorm]);
        let point = &system.jump_points[0];
        assert_eq!(point.to.to_string(), "brightwater/marrow-gate");
        assert_eq!(point.stability, Stability::Unstable);
        assert_eq!(point.owner, None);
        assert!(!point.one_way);
    }

    #[test]
    fn a_start_reads_its_name_system_ship_and_cash() {
        let start: Start =
            read("name: Dock hand\nsystem: marrow\nship: shuttle\ncash: 5000\n").unwrap();
        assert_eq!(start.name, "Dock hand");
        assert_eq!(start.system.as_str(), "marrow");
        assert_eq!(start.ship.as_str(), "shuttle");
        assert_eq!(start.cash, 5000);
    }

    #[test]
    fn a_start_refuses_a_field_it_does_not_have() {
        let error =
            read::<Start>("name: Dock hand\nsystem: marrow\nship: shuttle\ncash: 5000\ndate: 1\n")
                .unwrap_err();
        assert!(error.contains("unknown field `date`"), "{error}");
        let error = read::<Start>("name: Dock hand\nsystem: marrow\nship: shuttle\n").unwrap_err();
        assert!(error.contains("missing field `cash`"), "{error}");
    }

    /// Every name the schema gives `T`'s values.
    fn schema_names<T: JsonSchema>() -> std::collections::BTreeSet<String> {
        fn walk(value: &serde_json::Value, names: &mut std::collections::BTreeSet<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, v) in map {
                        match (key.as_str(), v) {
                            ("const", serde_json::Value::String(s)) => {
                                names.insert(s.clone());
                            }
                            ("enum", serde_json::Value::Array(all)) => {
                                names.extend(
                                    all.iter().filter_map(|n| n.as_str()).map(String::from),
                                );
                            }
                            _ => walk(v, names),
                        }
                    }
                }
                serde_json::Value::Array(all) => all.iter().for_each(|v| walk(v, names)),
                _ => {}
            }
        }
        let schema = schemars::schema_for!(T);
        let mut names = std::collections::BTreeSet::new();
        walk(schema.as_value(), &mut names);
        names
    }

    /// `as_str` gives each of `all` the name data writes it as, and `all`
    /// is every value the schema names.
    fn assert_data_names<T>(all: &[T], as_str: fn(T) -> &'static str)
    where
        T: JsonSchema + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
    {
        for &value in all {
            let name = as_str(value);
            assert_eq!(read::<T>(&format!("{name}\n")), Ok(value), "{name}");
        }
        let names = all.iter().map(|&v| as_str(v).to_owned()).collect();
        assert_eq!(schema_names::<T>(), names);
    }

    #[test]
    fn a_stability_is_named_as_in_data() {
        use Stability::{Dead, Stable, Unstable};
        assert_data_names(&[Stable, Unstable, Dead], Stability::as_str);
    }

    #[test]
    fn an_act_is_named_as_in_data() {
        assert_data_names(&Act::ALL, Act::as_str);
    }

    #[test]
    fn a_storyline_kind_is_named_as_in_data() {
        use StorylineKind::{Main, Minor, Spine};
        assert_data_names(&[Main, Minor, Spine], StorylineKind::as_str);
    }

    #[test]
    fn the_worse_of_two_stabilities_is_the_less_stable() {
        use Stability::{Dead, Stable, Unstable};
        let table = [
            (Stable, Stable, Stable),
            (Stable, Unstable, Unstable),
            (Stable, Dead, Dead),
            (Unstable, Stable, Unstable),
            (Unstable, Unstable, Unstable),
            (Unstable, Dead, Dead),
            (Dead, Stable, Dead),
            (Dead, Unstable, Dead),
            (Dead, Dead, Dead),
        ];
        for (a, b, worse) in table {
            assert_eq!(a.worse(b), worse, "{a:?} and {b:?}");
        }
    }

    #[test]
    fn a_jump_is_as_stable_as_its_worse_end() {
        let point = |stability: &str| -> JumpPoint {
            read(&format!(
                "id: p\nposition: [0, 0]\nto: s/q\nstability: {stability}\n"
            ))
            .unwrap()
        };
        let (stable, unstable, dead) = (point("stable"), point("unstable"), point("dead"));
        assert_eq!(jump_stability(&stable, &stable), Stability::Stable);
        assert_eq!(jump_stability(&stable, &unstable), Stability::Unstable);
        assert_eq!(jump_stability(&unstable, &stable), Stability::Unstable);
        assert_eq!(jump_stability(&dead, &unstable), Stability::Dead);
        assert_eq!(jump_stability(&stable, &dead), Stability::Dead);
    }

    #[test]
    fn an_unknown_hazard_is_an_error() {
        let err =
            read::<System>("name: M\nregion: r\nmap: [0, 0]\nhazards: [ion_storm]\n").unwrap_err();
        assert!(err.contains("ion_storm"), "{err}");
    }

    #[test]
    fn a_map_position_has_two_numbers() {
        assert!(read::<System>("name: M\nregion: r\nmap: [0]\n").is_err());
    }

    #[test]
    fn a_mission_reads_its_objective_and_effects() {
        let mission: Mission = read(
            "title: Take the point\nstoryline: holding-the-point\nact: committed\n\
             requires:\n  flags: [committed_to_the_point]\n\
             objective:\n  take_point:\n    point: marrow/far-gate\n    held_by: harrow-band\n\
             on_success:\n  points:\n    - point: marrow/far-gate\n      owner: rim-community\n",
        )
        .unwrap();
        assert!(matches!(
            mission.objective,
            Some(Objective::TakePoint { .. })
        ));
        assert_eq!(mission.requires.flags[0].as_str(), "committed_to_the_point");
        assert_eq!(
            mission.on_success.points[0]
                .owner
                .as_ref()
                .unwrap()
                .as_str(),
            "rim-community"
        );
        assert!(mission.offer.is_none());
    }

    #[test]
    fn a_mission_lists_its_effects_with_their_fields() {
        let mission: Mission = read(&format!(
            "{MISSION}on_offer:\n  flags: [offered]\non_accept:\n  flags: [accepted]\n\
             on_decline:\n  flags: [declined]\non_success:\n  flags: [succeeded]\n"
        ))
        .unwrap();
        let listed: Vec<_> = (mission.effects().iter())
            .map(|(field, e)| (*field, e.flags[0].as_str()))
            .collect();
        assert_eq!(
            listed,
            [
                ("on_offer", "offered"),
                ("on_accept", "accepted"),
                ("on_decline", "declined"),
                ("on_success", "succeeded"),
            ]
        );
        // Every field holding effects is listed.
        let schema = schemars::schema_for!(Mission);
        let properties = schema.as_value()["properties"].as_object().unwrap();
        let fields: Vec<_> = (properties.iter())
            .filter(|(_, p)| p.to_string().contains("/Effects\""))
            .map(|(field, _)| field.as_str())
            .collect();
        let mut names: Vec<_> = listed.iter().map(|(field, _)| *field).collect();
        names.sort_unstable();
        assert_eq!(fields, names);
    }

    const MISSION: &str = "title: T\nstoryline: s\nact: open\n";

    #[test]
    fn a_news_item_reads_its_headline_and_where_it_starts() {
        let news: News = read("headline: The road fails\nfrom: luyten-crossing\n").unwrap();
        assert_eq!(news.headline, "The road fails");
        assert_eq!(news.from.as_str(), "luyten-crossing");
        let err = read::<News>("headline: H\nfrom: s\nreaches: [t]\n").unwrap_err();
        assert!(err.contains("reaches"), "{err}");
    }

    #[test]
    fn an_effect_releases_news() {
        let mission: Mission = read(&format!(
            "{MISSION}on_offer:\n  news: [road-fails, toll-rises]\n"
        ))
        .unwrap();
        let ids: Vec<_> = mission.on_offer.news.iter().map(Id::as_str).collect();
        assert_eq!(ids, ["road-fails", "toll-rises"]);
        assert!(mission.on_success.news.is_empty());
    }

    #[test]
    fn conditions_group_with_any_and_none() {
        let mission: Mission = read(&format!(
            "{MISSION}requires:\n  flags: [x]\n  any:\n    - flags: [a]\n    - missions_done: [m]\n\
             \x20 none:\n    - flags: [b]\n"
        ))
        .unwrap();
        let r = &mission.requires;
        assert_eq!(r.flags[0].as_str(), "x");
        assert_eq!(r.any.len(), 2);
        assert_eq!(r.any[0].flags[0].as_str(), "a");
        assert_eq!(r.any[1].missions_done[0].as_str(), "m");
        assert_eq!(r.none[0].flags[0].as_str(), "b");
        assert!(r.all.is_empty());
    }

    #[test]
    fn all_combines_two_any_groups() {
        let mission: Mission = read(&format!(
            "{MISSION}requires:\n  all:\n    - any: [{{flags: [a]}}, {{flags: [b]}}]\n\
             \x20   - any: [{{flags: [c]}}, {{not_before_day: 3}}]\n"
        ))
        .unwrap();
        let all = &mission.requires.all;
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].any[0].flags[0].as_str(), "c");
        assert_eq!(all[1].any[1].not_before_day, Some(3));
    }

    #[test]
    fn an_unknown_field_inside_a_group_is_an_error_on_its_line() {
        let err = read::<Mission>(&format!(
            "{MISSION}requires:\n  any:\n    - flags: [a]\n    - flag: [b]\n"
        ))
        .unwrap_err();
        assert!(err.contains("flag"), "{err}");
        assert!(err.contains("line 8, column 7"), "{err}");
    }

    #[test]
    fn groups_lists_each_group_depth_first_with_its_path() {
        let mission: Mission = read(&format!(
            "{MISSION}requires:\n  any:\n    - flags: [a]\n    - all: [{{flags: [c]}}, {{flags: [d]}}]\n\
             \x20 none:\n    - flags: [b]\n"
        ))
        .unwrap();
        let groups = mission.requires.groups();
        let paths: Vec<_> = groups.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "",
                "any[0]",
                "any[1]",
                "any[1].all[0]",
                "any[1].all[1]",
                "none[0]"
            ]
        );
        assert_eq!(groups[4].1.flags[0].as_str(), "d");
        assert!(std::ptr::eq(groups[0].1, &raw const mission.requires));
    }

    #[test]
    fn conditions_without_groups_are_one_group() {
        let mission: Mission = read(MISSION).unwrap();
        let paths: Vec<_> = mission
            .requires
            .groups()
            .into_iter()
            .map(|(path, _)| path)
            .collect();
        assert_eq!(paths, [""]);
    }

    #[test]
    fn a_stellar_has_a_generated_look_or_an_asset() {
        let planet: Stellar = read(
            "name: P\nkind: planet\nsystem: s\nposition: [0, 0]\nvisual:\n  generated:\n\
             \x20   type: gas-giant\n    palette: [\"#c9a36b\", \"#8a5a3c\"]\n    clouds: 0.3\n\
             \x20   rings: {inner: 1.4, outer: 2.2, color: \"#d8c8a8\"}\n\
             \x20   atmosphere: {color: \"#f0e0c0\", thickness: 0.05}\n    seed: 4417\n",
        )
        .unwrap();
        let Some(Visual::Generated(look)) = planet.visual else {
            panic!("generated");
        };
        assert_eq!(look.kind, PlanetType::GasGiant);
        assert_eq!(look.clouds.get(), 0.3);
        assert_eq!(look.rings.unwrap().outer, 2.2);
        let station: Stellar = read(
            "name: S\nkind: station\nsystem: s\nposition: [0, 0]\nvisual: {asset: station-hub}\n",
        )
        .unwrap();
        assert!(matches!(station.visual, Some(Visual::Asset(id)) if id.as_str() == "station-hub"));
    }

    #[test]
    fn a_visual_is_one_or_the_other() {
        let both = read::<Stellar>(
            "name: S\nkind: moon\nsystem: s\nposition: [0, 0]\n\
             visual: {asset: a, generated: {type: ice, palette: [\"#ffffff\"], seed: 1}}\n",
        );
        assert!(both.is_err());
    }

    #[test]
    fn an_outcome_parses_from_its_tag_value() {
        assert_eq!(Outcome::parse("accept"), Some(Outcome::Accept));
        assert_eq!(Outcome::parse("decline"), Some(Outcome::Decline));
        assert_eq!(Outcome::parse("defer"), Some(Outcome::Defer));
        assert_eq!(Outcome::parse("later"), None);
        assert_eq!(Outcome::parse("Accept"), None);
    }

    #[test]
    fn each_outcome_round_trips_through_its_text() {
        let texts: Vec<_> = Outcome::ALL.iter().map(|o| o.as_str()).collect();
        assert_eq!(texts, ["accept", "decline", "defer"]);
        for outcome in Outcome::ALL {
            assert_eq!(Outcome::parse(outcome.as_str()), Some(outcome));
        }
    }

    #[test]
    fn an_objective_rejects_fields_it_does_not_have() {
        let err = read::<Mission>(
            "title: T\nstoryline: s\nact: open\nobjective:\n  deliver:\n    cargo: Seed\n    \
             tons: 3\n    to: yard\n    reward: 100\n",
        )
        .unwrap_err();
        assert!(err.contains("reward"), "{err}");
    }
}
