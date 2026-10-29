//! The catalog ports: what a flight session starts from
//! ([`PilotCatalog`]), what its NPC traffic is spawned from
//! ([`TrafficCatalog`]), what its ships fight with ([`CombatCatalog`]),
//! what its asteroid fields hold ([`AsteroidCatalog`]) and
//! the game's string lists ([`CommCatalog`]), which a hailed ship answers
//! from and the `T` set operator names the ship from, in the simulation's
//! own terms.

use std::collections::BTreeMap;
use std::rc::Rc;

pub use nova_data::{
    BoomId, CronId, DudeId, FleetId, GovtId, JunkId, MissionId, NebulaId, OutfitId, PersonId,
    RankId, ShipId, SoundId, StellarId, SystemId, WeaponId,
};

use crate::blink::Blink;
use crate::control::{Script, Test};
use crate::geometry::Vec2;
use crate::handling::ShipFields;
use crate::voice::Voices;
use crate::warp::SoundLength;
use nova_sim_base::fuel::OutfitMod;

/// A new pilot's start, from the first `chär`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterStart {
    /// The starting `shïp`, if it names one.
    pub ship: Option<ShipId>,
    /// The starting `sÿst`s, in order; any may be unused.
    pub systems: [Option<SystemId>; 4],
    /// The starting date, raw.
    pub start: StartDate,
    /// The starting credits, raw: the `nova_sim::pilot` rules
    /// decide what a negative amount means.
    pub cash: i32,
    /// The starting legal records, `Govt1-4` with `Status1-4`: each
    /// government and the record with it, or `None` for an unused slot.
    pub legal: [Option<(GovtId, i16)>; 4],
    /// The starting combat rating, raw `Kills`.
    pub kills: i16,
    /// Its `OnStart` set expression, parsed: run once on a new pilot,
    /// before its first save (see the session's `hooks`).
    pub on_start: Script,
}

/// A new pilot's starting date, raw from the `chär`: the
/// `nova_sim::date` rules decide what the values mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartDate {
    /// The day of the month.
    pub day: i16,
    /// The month, 1 (January) to 12.
    pub month: i16,
    /// The year.
    pub year: i16,
}

/// What the date is wrapped in wherever it is displayed: the first
/// `chär`'s `DatePrefix` and `DateSuffix`, verbatim.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateAffixes {
    /// `DatePrefix`: put before the date.
    pub prefix: String,
    /// `DateSuffix`: put after the date (stock " NC").
    pub suffix: String,
}

/// A star system on the map, raw from its `sÿst`: the
/// `nova_sim::hyperspace` rules decide which links count.
#[derive(Clone, Debug, PartialEq)]
pub struct StarSystem {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its map position, `xPos` and `yPos`.
    pub position: Vec2,
    /// Its hyperlinks, `Con1-Con16`, in record order: they may repeat,
    /// point at itself or at a system that does not exist. An empty slot
    /// before the last link is -1, in its place, so a link's index is its
    /// slot number, as the original's loader keeps them
    /// (`_LoadObjectData` @0x7806d-0x780b8).
    pub links: Vec<SystemId>,
    /// Its controlling government, `Govt`, or `None` when it is
    /// independent (-1).
    pub govt: Option<GovtId>,
    /// Each of its `NavDefs` slots, in order: the stellar's `spöb` `Flags`
    /// and `Flags2`, raw, or `None` for an empty slot or a stellar that
    /// cannot be read. The `nova_sim::exploration` rules read
    /// whether the system is inhabited from them.
    pub stellars: Vec<Option<(u32, u16)>>,
    /// Its `Interference`, raw: the radar's static
    /// (`nova_sim::iff::interference`).
    pub interference: i16,
    /// Its `Murk`, raw: how murky its space is, which
    /// `nova_sim::murk::loaded_murk` loads as the original does, and the
    /// galaxy map panel's "visibility" hazard (`nova_sim::map_panel`;
    /// `_MapDialogUpdate` @0x1086c-0x1090c reads it at runtime +0xbc).
    pub murk: i16,
    /// Its `Message`, raw: the message buoy a hyperspace arrival shows,
    /// −1 for none (`nova_sim::StarMap::buoy`).
    pub message: i16,
    /// Its `BkgndColor`, raw `00RRGGBB`: the colour the original erases
    /// the flight message's box in while the player is here
    /// (`_SetSystemBackgroundBackColor` @0x6e06-0x6e46, reading the
    /// `RGBColor` `_LoadObjectData` builds from it @0x77d85-0x77dc5).
    pub background: u32,
}

/// A stellar the player might land on, raw from its `spöb`; the
/// `nova_sim::landing` rules decide what the values mean.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LandingSite {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its centre, `xPos` and `yPos`, in pixels from the system's centre.
    pub position: Vec2,
    /// Its sprite's frame width and height in pixels, or `None` when it
    /// has no sprite that can be read.
    pub frame_size: Option<(u32, u32)>,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `MinStatus`: the legal record below which landing is refused.
    pub min_status: i16,
    /// Its `TechLevel`: the `nova_sim::outfitter` sells outfits
    /// up to it.
    pub tech_level: i16,
    /// Its `SpecialTech` 1-8, in order: the outfitter also sells outfits of
    /// exactly these tech levels.
    pub special_tech: [i16; 8],
    /// Its `Govt`, or `None` when it is independent.
    pub govt: Option<GovtId>,
    /// Its `Flags2`, raw: the `nova_sim::gate` rules read its
    /// hypergate (0x1000) and wormhole (0x2000) bits, and the
    /// `nova_sim::market` its always-dominated bit
    /// (`nova_sim::market::DOMINATED`, 0x0020), and the
    /// `nova_sim::outfitter` its bit that buys back owned
    /// outfits (`nova_sim::outfitter::BUYS_OWNED_OUTFITS`,
    /// 0x0400).
    pub flags2: u16,
    /// Whether it is destroyed: never in the catalog, which gives `false`;
    /// the galaxy overlay stamps it
    /// (`nova_sim::story::OverlaidGalaxy::site`).
    pub destroyed: bool,
    /// Its `Type`, raw: the graphic type whose master the original's
    /// landing distance reads (see `nova_sim::landing`).
    pub graphic: i16,
    /// Its `DeadType`, when that is 0 to 255 and its sheet can be read:
    /// the graphic it shows while destroyed.
    pub dead_graphic: Option<i16>,
    /// The frame width and height of its `DeadType`'s sheet, `Some`
    /// exactly when [`dead_graphic`](Self::dead_graphic) is: the box a
    /// click on it reads while destroyed (story-runtime phase 11).
    pub dead_frame_size: Option<(u32, u32)>,
    /// Whether its `Type`'s `spïn` gives a frame count above 0, `xTiles` ×
    /// `yTiles`: the original reads a width of 32 for a loaded `Type`
    /// master without one (`_LoadMasterStellarSprite` @0x1ed8c-0x1ed97,
    /// `_GetStellarSpriteXSize` @0xaced-0xacff).
    pub type_counts_frames: bool,
    /// Whether its `DeadType`'s `spïn` gives a frame count above 0,
    /// `xTiles` × `yTiles`, as [`type_counts_frames`](Self::type_counts_frames)
    /// for its `Type`: a sprite slot cloning a master without one keeps
    /// the frame it showed (`_SWSetCurrentFrameIndex` @0xa90e7-0xa90ee;
    /// stellar-fidelity phase 8). `true` where
    /// [`dead_graphic`](Self::dead_graphic) is `None`, so it means
    /// something only beside it.
    pub dead_counts_frames: bool,
    /// Its `CustSndID`, raw, as [`GateSite::exit_angle`]: the heading an
    /// NPC comes out of it on when it is a hypergate or wormhole and the
    /// value is 0 to 359 (`nova_sim::gate::exit_heading`).
    pub exit_angle: i16,
    /// Its `Tribute`, raw: what it pays a day while dominated, as
    /// `nova_sim::player_info::tribute_of` reads it.
    pub tribute: i16,
    /// Its `Gravity`, raw: the galaxy map panel's "gravity shear" hazard
    /// when not 0 (`nova_sim::map_panel`; `_MapDialogUpdate`
    /// @0x1061f-0x106a9 reads it at runtime +0x46c).
    pub gravity: i16,
    /// Its `Fee`, raw: the Bible's "fee that is deducted from the player's
    /// credits when landing on this stellar". `_HandlePlayerDockRequest`
    /// and `_HandlePlayerDockApproach` read it at runtime +0x38;
    /// `nova_sim::landing::fee_due` decides when it is due.
    pub fee: i32,
    /// Its animation fields and frame counts, which a hypergate's sprite
    /// reads (`nova_sim::gate_sprite`).
    pub animation: SiteAnimation,
    /// Its `DefenseDude`: the `düde` its defence ships are drawn from, or
    /// `None` for an ID below 128, as `_LoadObjectData` keeps runtime
    /// +0x464 at -1 then (@0x77603, @0x7770c-0x77725). See
    /// `nova_sim_ai::traffic::stellar_defence`.
    pub defense_dude: Option<DudeId>,
    /// Its `DefCount`, raw (record +0x1e into runtime +0x468, @0x775eb):
    /// the wave size and the count of defence ships it starts with, as
    /// `nova_sim_ai::traffic::stellar_defence` decodes them.
    pub def_count: i16,
}

/// A stellar's animation fields, raw from its `spöb`, and the frame counts
/// of its sheets, as the original's sprite tick reads them
/// (`_LoadObjectData` @0x7761b-0x77631, `_LoadMasterStellarSprite`
/// @0x1ed8c-0x1ed97).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SiteAnimation {
    /// Its `AnimDelay`, raw.
    pub delay: i16,
    /// Its `Frame0Bias`, raw.
    pub frame0_bias: i16,
    /// Its `CustPicID`, raw: a hypergate's open frame, when it is one.
    pub open_frame: i16,
    /// Its `Type`'s `spïn` frame count, `xTiles` × `yTiles`: 0 when that
    /// does not read or is not above 0, and at most 65535.
    pub type_frames: u16,
    /// Its `DeadType`'s, likewise, when [`dead_graphic`](LandingSite::dead_graphic)
    /// is `Some`; 0 otherwise.
    pub dead_frames: u16,
}

/// A stellar a hypergate or wormhole may lead to, raw from its `spöb`: the
/// `nova_sim::gate` rules decide what the values mean. Any stellar a
/// system lists may be one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GateSite {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its centre, `xPos` and `yPos`, in pixels from the system's centre.
    pub position: Vec2,
    /// Its `Flags2`, raw.
    pub flags2: u16,
    /// Its `HyperLink1-8`, in slot order, `None` for an unused one (-1).
    /// The Bible's other unused value, 0, is kept: no system lists a
    /// stellar 0, so it leads nowhere.
    pub links: [Option<StellarId>; 8],
    /// Its `CustSndID`, raw: the heading a ship comes out of it on, when
    /// it is 0 to 359.
    pub exit_angle: i16,
}

/// A stellar some system lists, raw from its `spöb`: what a mission's
/// stellars are chosen by (see `nova_sim::mission`), without the
/// sprites a [`LandingSite`] reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedStellar {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Every `sÿst` that can be read and lists it, by ascending ID, each
    /// once: the galaxy refresh gives it to the lowest visible one not
    /// shadowed at its place
    /// (`nova_sim::story::OverlaidGalaxy::owner`),
    /// the player's system holds it when none does
    /// (`nova_sim::story::OverlaidGalaxy::holder`),
    /// and `_FindSystemFromStellar` finds it in the lowest visible one
    /// (`nova_sim::story::OverlaidGalaxy::find_system_from_stellar`).
    pub systems: Vec<SystemId>,
    /// Its `Flags`.
    pub flags: u32,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `Govt`, or `None` when it is independent.
    pub govt: Option<GovtId>,
    /// Its `MinStatus`: what the galaxy map's colour code weighs the
    /// player's record against (see `nova_sim::iff`'s "Systems on the
    /// galaxy map").
    pub min_status: i16,
}

/// A system's `Visibility`, raw from its `sÿst`: the galaxy refresh
/// evaluates it into the overlay (see the `nova_sim::story` module's
/// "Visibility and governments").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemVisibility {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its `Visibility`, parsed: a blank one always holds, and one that did
    /// not parse never does, so the system is hidden.
    pub visibility: Test,
}

/// The stellars' and systems' names, by ID, that the text tags read
/// (see `nova_sim::text_tags`): each `spöb` and `sÿst` that can
/// be read, named as the game shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaceNames {
    /// Each stellar's name.
    pub stellars: BTreeMap<StellarId, String>,
    /// Each system's name.
    pub systems: BTreeMap<SystemId, String>,
}

/// What the original compares of two stellars to count them as the same
/// stop of a mission (`_StellarsAreIdentical` @0xab35-0xac90): a `spöb`'s
/// place in its system and its name, as `_LoadObjectData` keeps them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StellarMark {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its raw `xPos` and `yPos` (@0x7733b-0x7734d).
    pub place: (i16, i16),
    /// Its resource name up to its last ';', trailing spaces and ';'s
    /// dropped (@0x772ab-0x77308), empty when it has none. Kept whole:
    /// `nova_sim::mission::StellarMarks` compares the 30
    /// characters the original's record keeps (@0x77310-0x7732a).
    pub name: String,
}

/// The first `bööm`, explosion type 0.
pub const FIRST_BOOM: i16 = 128;
/// How many explosion types there are.
pub const BOOM_TYPES: i16 = 64;
/// What an explosion type is offset by to add extra small explosions.
pub const EXTRA_EXPLOSIONS: i16 = 1000;

/// The first explosion sound, `snd ` 300: `_LoadSounds` loads 300-363
/// into the explosion handles (0x3b5428, from @0x1c200).
pub const FIRST_EXPLOSION_SOUND: i16 = 300;

/// The `snd ` an explosion of a `bööm` whose `SoundIndex` is `index`
/// plays, `snd ` 300 + `index`: none outside 0-63, as `_CreateExplosion`
/// plays only those (@0x3f8dc-0x3f8e5).
#[must_use]
pub fn explosion_sound(index: i16) -> Option<SoundId> {
    (0..BOOM_TYPES)
        .contains(&index)
        .then(|| SoundId(FIRST_EXPLOSION_SOUND + index))
}

/// An explosion: its `bööm`, and whether extra type-0 explosions go off
/// around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Explosion {
    /// Its `bööm`.
    pub boom: BoomId,
    /// Whether a random number of type-0 explosions go off around it.
    pub extra: bool,
}

impl Explosion {
    /// The explosion an `ExplodType`, `Explode1` or `Explode2` of `raw`
    /// means, if any.
    #[must_use]
    pub fn decode(raw: i16) -> Option<Self> {
        let (kind, extra) = if raw >= EXTRA_EXPLOSIONS {
            (raw - EXTRA_EXPLOSIONS, true)
        } else {
            (raw, false)
        };
        (0..BOOM_TYPES).contains(&kind).then_some(Self {
            boom: BoomId(FIRST_BOOM + kind),
            extra,
        })
    }
}

/// What a stellar's destruction and regeneration read from its `spöb`
/// (see the `nova_sim::story` module's "Galaxy overlay"): the
/// story's wreck rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StellarLife {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its `Strength`: 0 or -1 for an invincible stellar.
    pub strength: i32,
    /// Its `DeadTime`: the days it stays destroyed, -1 for never.
    pub dead_time: i16,
    /// Its `Flags2`, raw: the wreck rules read whether it starts the game
    /// destroyed from it.
    pub flags2: u16,
    /// Its `ExplodType`: the explosion it makes as it is destroyed, if
    /// any.
    pub explosion: Option<Explosion>,
    /// Its `OnDestroy`, run as a shot destroys it.
    pub on_destroy: Script,
    /// Its `OnRegen`, run as it regenerates on its own.
    pub on_regen: Script,
}

/// What the planet comm dialog runs of a stellar's `spöb` as Demand
/// Tribute dominates it and Release releases it (`_DoPlanetCommDialog`
/// @0x97966, @0x97527 in the `EV Nova` executable).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StellarScripts {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its `OnDominate` (stellar +0x68).
    pub on_dominate: Script,
    /// Its `OnRelease` (stellar +0x167).
    pub on_release: Script,
}

/// An outfit, raw from its `oütf`: the `nova_sim::outfitter`
/// rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutfitRecord {
    /// The `oütf`'s ID.
    pub id: OutfitId,
    /// Its name: the resource's name, or its `LCName` when the resource
    /// has none.
    pub name: String,
    /// Its `ShortName`, raw: a literal `\n` splits it in two lines.
    pub short_name: String,
    /// Its `DispWeight`: higher shows nearer the top.
    pub disp_weight: i16,
    /// Its `Mass`, in tons.
    pub mass: i16,
    /// Its `TechLevel`.
    pub tech_level: i16,
    /// Its `Max`: how many the player can own.
    pub max: i16,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Cost`.
    pub cost: i32,
    /// Its `ModType` and `ModVal` pairs 1-4, in order: raw, except that a
    /// weapon, ammunition or cleaned government (`ModType` 1, 3 or 21)
    /// `ModVal` is the ID the loader reads it as (see [`loaded_mod_val`]).
    pub mods: [(i16, i16); 4],
    /// Its `Contribute` bits.
    pub contribute: u64,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `RequireGovt`, raw.
    pub require_govt: i16,
    /// Its `BuyRandom`, raw: the percent chance a day that it is for
    /// sale, as `nova_sim::outfitter` reads it.
    pub buy_random: i16,
    /// Its `Availability` control-bit test, parsed.
    pub availability: Test,
    /// Its `OnPurchase` set expression, parsed: run once for each unit
    /// bought (see the session's `hooks`).
    pub on_purchase: Script,
    /// Its `OnSell` set expression, parsed: run once for each unit sold.
    pub on_sell: Script,
    /// Its `ItemClass`, raw: the class a person's `GrantClass` grants
    /// from (see `nova_sim::grant`).
    pub item_class: i16,
    /// Its `LCName`, raw: its lower-case singular name.
    pub lc_name: String,
    /// Its `LCPlural`, raw: its lower-case plural name.
    pub lc_plural: String,
    /// Its `ScanMask`, raw: a scan finds it owned when this shares a bit
    /// with the scanner's government's (see `nova_sim::scan`).
    pub scan_mask: u16,
}

/// An `oütf` `ModVal` as the original's loader reads it, as an ID.
///
/// For a weapon (`ModType` 1), ammunition (3) or the government a record
/// is cleaned with (21), `_LoadObjectData` (@0x78dcc-0x78df7) turns a
/// `ModVal` above 127 into an index by subtracting 128 and leaves 0..=127
/// (and a negative value) as it is; index i names ID i + 128. So 0..=127
/// names ID `mod_val + 128`, 128 and up names itself, and a negative value
/// names no record and stays negative. Every other `ModType` is untouched.
#[must_use]
pub fn loaded_mod_val(mod_type: i16, mod_val: i16) -> i16 {
    match (mod_type, mod_val) {
        (1 | 3 | 21, 0..=127) => mod_val + 128,
        _ => mod_val,
    }
}

/// The `ModType` of marines: a positive `ModVal` adds crew, a negative one
/// defends against boarders. The player's boarding odds read it
/// (`nova_sim_port::board`), and so does a pirate's plunder
/// (`nova_sim::ai::plunder_contents`).
pub const MARINES: i16 = 25;

/// A ship class, raw from its `shïp`: the `nova_sim::shipyard`
/// rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipRecord {
    /// The `shïp`'s ID.
    pub id: ShipId,
    /// Its name: the resource's name up to any ';' (the rest is a
    /// designer's note), or its `ShortName` on one line (each literal `\n`
    /// a space) when the resource has none.
    pub name: String,
    /// Its `ShortName`, raw: a literal `\n` splits it in two lines.
    pub short_name: String,
    /// Its `Long Name`.
    pub long_name: String,
    /// Its handling, reserve, cargo and mass fields.
    pub fields: ShipFields,
    /// Its `DefaultItems`, raw: each item's `oütf` ID with its count, in
    /// slot order, a negative count as none (as
    /// [`PilotCatalog::default_outfits`] gives them).
    pub defaults: Vec<(OutfitId, u16)>,
    /// Its `DefaultItems` slot by slot, 1-8: each slot's `oütf` ID and
    /// raw count, none where the slot names no item. A pirate's plunder
    /// reads an item's place (see `nova_sim::ai::plunder_contents`).
    pub default_slots: [Option<(OutfitId, i16)>; 8],
    /// Its `Strength`, raw: a pirate's plunder reads it signed (see
    /// `nova_sim::ai::plunder_contents`).
    pub strength: i16,
    /// Its `Cost`.
    pub cost: i32,
    /// Its `TechLevel`.
    pub tech_level: i16,
    /// Its `BuyRandom`: the percent chance a day it is for sale.
    pub buy_random: i16,
    /// Its `HireRandom`: the percent chance a day it is for hire in the
    /// bar (see `nova_sim::hire`).
    pub hire_random: i16,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `Availability` control-bit test, parsed.
    pub availability: Test,
    /// Its `AppearOn` control-bit test, parsed. The Bible: "Ships of this
    /// type will not show up in dude resources if this expression
    /// evaluates to false" (see `nova_sim::traffic::spawn`).
    pub appear_on: Test,
    /// Its `OnPurchase` set expression, parsed: run when the player buys
    /// one (see the session's `hooks`).
    pub on_purchase: Script,
    /// Its `OnCapture` set expression, parsed: run when the player
    /// captures one, as an escort or as its own ship.
    pub on_capture: Script,
    /// Its `OnRetire` set expression, parsed: run when the player trades
    /// one in or leaves it for a captured ship.
    pub on_retire: Script,
    /// Its `Flags`, raw: 0x8000 marks a fighter the player can escape in
    /// (see `nova_sim::escape`), and its jump-speed bits give its
    /// warp factor ([`warp_factor`](crate::warp::warp_factor)).
    pub flags: u16,
    /// Its `Flags3`.
    pub flags3: u16,
    /// Its `DispWeight`: higher shows nearer the top.
    pub disp_weight: i16,
    /// Its `MaxGun`.
    pub max_gun: i16,
    /// Its `MaxTur`.
    pub max_tur: i16,
    /// Its `Length`, in metres.
    pub length: i16,
    /// Its `Crew`.
    pub crew: i16,
    /// Its `InherentAI`, raw: the AI type (1-4) it flies with when no
    /// `düde` gives one, and always as a fleet's ship.
    pub inherent_ai: i16,
    /// Its `CommName`: what it is called when hailed.
    pub comm_name: String,
    /// The government whose attributes (its flags) it takes on, from its
    /// `InherentGovt`: a `gövt` ID, or one + 1000 ("attributes only");
    /// none for -1 or + 2000 ("combat only").
    pub inherent_govt: Option<GovtId>,
    /// The government it fights as, from its `InherentGovt`: a `gövt` ID,
    /// or one + 2000 ("combat only"); none for -1 or + 1000 ("attributes
    /// only"). A mission's `AvailShipType` 2128-3384 reads it beside
    /// [`inherent_govt`](Self::inherent_govt) (see
    /// `nova_sim::mission`).
    pub combat_govt: Option<GovtId>,
    /// Its `EscortType`, raw: the class it has as the player's escort
    /// (see [`EscortClass::of`](crate::catalog::EscortClass::of)).
    pub escort_type: i16,
    /// Its `SkillVar`, raw: how far an NPC's skill, rolled as it spawns,
    /// strays from 1.0, in hundredths (class +0x9f8, `_RandomSkillLevel`
    /// @0x6922; see `nova_sim::skill`).
    pub skill_var: i16,
    /// Its `Subtitle` (class +0xac, `_LoadObjectData` @0x7b0f1): the
    /// escort dialog's third title line, under the `CommName`.
    pub subtitle: String,
    /// The class an escort of it is upgraded to: its `UpgradeTo` when the
    /// raw ID is 128 or more, none otherwise (`_LoadObjectData`
    /// @0x7a3de-0x7a3f7, class +0xa22).
    pub upgrade_to: Option<ShipId>,
    /// What upgrading an escort of it costs: its `EscUpgrdCost`, a
    /// negative one read as 0 (`_LoadObjectData` @0x7a410-0x7a417, class
    /// +0x64).
    pub esc_upgrd_cost: i64,
    /// What selling a captured escort of it pays: its `EscSellValue`, or
    /// `Cost` x 0.1 truncated when that is 0 or less (`_LoadObjectData`
    /// @0x7a42e-0x7a442, the double at 0xdd110, class +0x68).
    pub esc_sell_value: i64,
}

/// The standard commodities, raw from their string lists: `STR#` 4000
/// "All Cargo" names them and `STR#` 4004 "Base Prices" prices them, the
/// nth string for commodity n (from 0). A plug-in's `'STR '` 9000 + n
/// patches commodity n's 4000 string, and its `'STR '` 9300 + n patches
/// commodity n's 4004 string. The `nova_sim::market` rules decide
/// which string names and which prices each commodity, and how a price
/// is read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommodityStrings {
    /// Every string of `STR#` 4000, in order; none when it is missing.
    pub names: Vec<String>,
    /// `'STR '` 9000-9005, the nth for commodity n.
    pub name_patches: [StringPatch; 6],
    /// Every string of `STR#` 4004, in order; none when it is missing.
    pub base_prices: Vec<String>,
    /// `'STR '` 9300-9305, the nth for commodity n.
    pub price_patches: [StringPatch; 6],
}

/// A plug-in's `'STR '` patch of one `STR#` string (the Bible's
/// Appendix III).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum StringPatch {
    /// No `'STR '` with that ID: the `STR#` string stands.
    #[default]
    Absent,
    /// The `'STR '` text, whatever it holds.
    Text(String),
    /// A `'STR '` with that ID and no bytes at all. What the original
    /// then reads depends on the strings it patches: see
    /// `nova_sim::market`.
    Empty,
    /// A `'STR '` with that ID whose length byte runs past its data. What
    /// the original then reads depends on the strings it patches: see
    /// `nova_sim::market`.
    Unreadable,
}

impl StringPatch {
    /// What this patch shows in place of the `STR#` line it patches: its
    /// text, an empty line for one that is [`Empty`](Self::Empty) or
    /// [`Unreadable`](Self::Unreadable), or `None` when it is
    /// [`Absent`](Self::Absent) and the line stands.
    ///
    /// `_LoadPluginString` (@0x71a8e-0x71af4) calls `GetResource`, not
    /// `Get1Resource`, so a `STR ` in any open file patches, the base
    /// data's included. It answers that the `STR ` exists even when it is
    /// empty, skipping the copy (@0x71ac5), and copies one too short for
    /// its length byte raw, so the original shows whatever its buffer held
    /// before. The engine cannot model stale buffers and shows an empty
    /// line for both; a caller whose buffer the original reuses in a way
    /// the player can see (the market's prices) reads the patch itself.
    #[must_use]
    pub fn shown(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Empty | Self::Unreadable => Some(""),
            Self::Absent => None,
        }
    }
}

/// The ID of the `STR ` that patches line `index` of a `STR#` whose
/// patches start `offset` before it (`_LoadPluginString(offset + index)`,
/// @0x71a8e), or `None` when the sum does not fit.
///
/// The original adds in 32 bits and cuts the sum to 16, so a large
/// `index` looks up a negative ID. The engine keeps wider numbers and
/// gives no ID on overflow; the caller then looks up no `STR `
/// ([`StringPatch::Absent`]).
#[must_use]
pub fn patch_id(index: i16, offset: i16) -> Option<i16> {
    index.checked_add(offset)
}

/// The string line `index` (from 1) of a `STR#` shows, given the `STR `
/// `patch` of its [`patch_id`] and `lines`, the `STR#` whole: what the
/// patch [shows](StringPatch::shown) when there is one, else `lines`'
/// #`index` (`GetIndString`), else `None`, an index the list does not
/// hold: 0, below 0, past its count, or with no `STR#` at all. What a
/// missing line shows is the caller's.
#[must_use]
pub fn patched_string(index: i16, patch: StringPatch, lines: &[String]) -> Option<String> {
    match patch {
        StringPatch::Text(text) => Some(text),
        StringPatch::Absent => usize::try_from(index)
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|at| lines.get(at))
            .cloned(),
        patch => patch.shown().map(str::to_owned),
    }
}

/// A special commodity, raw from its `jünk`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunkRecord {
    /// The `jünk`'s ID.
    pub id: JunkId,
    /// Its name: the resource's name, or its `LCName` when the resource
    /// has none.
    pub name: String,
    /// Its `Abbrev`: the name the status bar's cargo area shows of it
    /// (`_DrawStatusCargo` @0x4c69a-0x4c6dd).
    pub abbrev: String,
    /// Its `BasePrice`.
    pub base_price: i16,
    /// Its `SoldAt` stellars, the unused (-1) slots left out.
    pub sold_at: Vec<StellarId>,
    /// Its `BoughtAt` stellars, the unused (-1) slots left out.
    pub bought_at: Vec<StellarId>,
    /// Its `BuyOn` control-bit test, parsed.
    pub buy_on: Test,
    /// Its `SellOn` control-bit test, parsed.
    pub sell_on: Test,
    /// Its `Flags`, raw: `nova_sim::market::TRIBBLES` and
    /// `nova_sim::market::PERISHABLE` are read.
    pub flags: u16,
    /// Its `ScanMask`, raw: a scan finds it held when this shares a bit
    /// with the scanner's government's (see `nova_sim::scan`).
    pub scan_mask: u16,
    /// Its `LCName`, raw: what a scan that finds it calls it.
    pub lc_name: String,
}

/// An `öops` resource's ID: a planetary event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisasterId(pub i16);

/// A planetary event that moves one commodity's price, raw from its
/// `öops`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisasterRecord {
    /// The `öops`'s ID.
    pub id: DisasterId,
    /// Its resource name up to its last ';', trailing spaces and ';'s
    /// dropped, as the loader keeps it (`_LoadObjectData` @0x7cdd3). The
    /// commodity exchange and the news show it while it is active.
    pub name: String,
    /// Its `Stellar`, raw: a `spöb` ID, -1 for any stellar or -2 for none
    /// (news only).
    pub stellar: i16,
    /// Its `Commodity`: 0 food, 1 industrial, and so on.
    pub commodity: i16,
    /// Its `PriceDelta`.
    pub price_delta: i16,
    /// Its `Duration`, in days.
    pub duration: i16,
    /// Its `Freq`: the percent chance each day that it starts.
    pub freq: i16,
    /// Its `ActivateOn` control-bit test, parsed.
    pub activate_on: Test,
}

/// A system's traffic, raw from its `sÿst`: the
/// `nova_sim::traffic` rules decide what the values mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemTraffic {
    /// Its `DudeTypes`, each with its `% Prob`, in record order: a `düde`
    /// ID, a negated `flët` ID, or an unused slot.
    pub dude_types: [(i16, i16); 8],
    /// Its `AvgShips`.
    pub avg_ships: i16,
    /// Its Person slots, each with its chance (the undocumented `sÿst`
    /// words at 0x7E), in record order: a `përs`, or an unused slot.
    pub persons: [(Option<PersonId>, i16); 8],
}

/// A system's reinforcement fleet, from its `sÿst`, as `_LoadObjectData`
/// keeps it (@0x78281-0x78318): a `ReinfFleet` above 127 names the
/// `flët`, and only then is a `ReinfIntrval` of 0 or less read as 1
/// (@0x782cb-0x782e9); a `ReinfFleet` of 127 or less names none
/// (@0x782f4). The `ReinfTime` is kept raw. See
/// `nova_sim::ai::reinforcements` for what they mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemReinforcements {
    /// Its `ReinfFleet`.
    pub fleet: FleetId,
    /// Its `ReinfTime`, in ticks: how long a called fleet takes to come.
    pub time: i16,
    /// Its `ReinfIntrval`, at least 1: the days before the fleet can be
    /// called again.
    pub interval: i16,
}

impl SystemReinforcements {
    /// The reinforcements a `sÿst` whose `ReinfFleet` is `fleet` (raw,
    /// -1 for none), `ReinfTime` `time` and `ReinfIntrval` `interval`
    /// keeps (see the type's docs).
    #[must_use]
    pub fn normalised(fleet: i16, time: i16, interval: i16) -> Option<Self> {
        (fleet > LAST_UNNAMED_REINFORCEMENT).then_some(Self {
            fleet: FleetId(fleet),
            time,
            interval: interval.max(1),
        })
    }
}

/// The highest `ReinfFleet` that names no fleet (`cmpw $0x7f`
/// @0x782cb).
const LAST_UNNAMED_REINFORCEMENT: i16 = 127;

/// A `düde`, raw: the `nova_sim::traffic` rules decide what the
/// values mean.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DudeRecord {
    /// Its `AIType`.
    pub ai_type: i16,
    /// Its `Govt`, or `None` for independent (-1).
    pub govt: Option<GovtId>,
    /// Its `ShipType` slots that name a ship, each with its
    /// `Probability`, in record order; the unused (-1) slots left out.
    pub ships: Vec<(ShipId, i16)>,
    /// Its `Booty` flags: what boarding one of its ships yields (see
    /// `nova_sim::board`).
    pub booty: u16,
    /// Its `InfoTypes` flags: what its ships say when hailed (see
    /// `nova_sim::hail`).
    pub info_types: u16,
}

/// One of a fleet's escort types, raw from its `flët`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortRecord {
    /// Its `EscortType`.
    pub ship: ShipId,
    /// Its `Min`.
    pub min: i16,
    /// Its `Max`.
    pub max: i16,
}

/// A fleet, raw from its `flët`: the `nova_sim::traffic` rules
/// decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FleetRecord {
    /// The `flët`'s ID.
    pub id: FleetId,
    /// Its `LeadShipType`, if it names one.
    pub lead: Option<ShipId>,
    /// Its escort types that name a ship, in record order; the unused
    /// (-1) slots left out.
    pub escorts: Vec<EscortRecord>,
    /// Its `Govt`, or `None` for independent (-1).
    pub govt: Option<GovtId>,
    /// Its `LinkSyst`, raw.
    pub link_syst: i16,
    /// Its `AppearOn` control-bit test, parsed.
    pub appear_on: Test,
    /// Its `Quote`: the `STR#` an announced arrival of it shows a line
    /// of, when above 0 (+0x1e > 0, `_HyperSpawnFleet` @0x42621-0x42629;
    /// see `nova_sim::traffic::quote`).
    pub quote: Option<i16>,
    /// Its `Flags`, raw: 0x0001 gives its trader ships random cargo (see
    /// `nova_sim::traffic::spawn::fleet`).
    pub flags: u16,
}

/// One of a person's weapon slots that names a weapon, raw from its
/// `përs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonWeapon {
    /// Its `WeapType`.
    pub weapon: WeaponId,
    /// Its `WeapCount`: how many more of the weapon than the ship carries.
    pub count: i16,
    /// Its `AmmoLoad`: how many more rounds of the weapon's ammunition.
    pub ammo: i16,
}

/// A person, raw from its `përs`: the `nova_sim::person` rules
/// decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonRecord {
    /// The `përs`'s ID.
    pub id: PersonId,
    /// Its name: the resource's name up to its last ';', trailing spaces
    /// and ';'s dropped, at most 29 characters (`_LoadObjectData`
    /// @0x7c57d-0x7c5f3); empty when the resource has none.
    pub name: String,
    /// Its `LinkSyst`, raw.
    pub link_syst: i16,
    /// Its `Govt`, or `None` for independent (-1).
    pub govt: Option<GovtId>,
    /// Its `AIType`, raw.
    pub ai_type: i16,
    /// Its `Aggress`, raw.
    pub aggress: i16,
    /// Its `Coward`: the percent of its shields below which it runs.
    pub coward: i16,
    /// Its `ShipType`, if it names one.
    pub ship: Option<ShipId>,
    /// Its weapon slots that name a weapon (`WeapType` above 127), in
    /// slot order.
    pub weapons: Vec<PersonWeapon>,
    /// Its `Credits`.
    pub credits: i32,
    /// Its `ShieldMod`: a percent; negative is invincible.
    pub shield_mod: i16,
    /// Its `HailPict`, when it is 128 or more.
    pub hail_pict: Option<i16>,
    /// Its `CommQuote`: an entry in `STR#` 7100.
    pub comm_quote: i16,
    /// Its `HailQuote`: an entry in `STR#` 7101.
    pub hail_quote: i16,
    /// Its `LinkMission`, when it is above 127.
    pub link_mission: Option<i16>,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `ActiveOn` control-bit test, parsed.
    pub active_on: Test,
    /// Its subtitle, the 64-byte string at 0x13A.
    pub subtitle: String,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `GrantClass`, raw: the `ItemClass` of the outfits boarding it
    /// may grant (see `nova_sim::grant`).
    pub grant_class: i16,
    /// Its `GrantCount`, raw: the most outfits a grant gives.
    pub grant_count: i16,
    /// Its `GrantProb`, raw: the percent chance of a grant.
    pub grant_prob: i16,
    /// Its `LinkMission`'s special ships: none without a `LinkMission`,
    /// or when the mission cannot be read.
    pub mission_ship: Option<MissionShip>,
}

/// A mission's special ships, raw from its `mïsn`: as many as its
/// `ShipCount`, with its `ShipGoal`, and where, of what and how they
/// spawn (see the `nova_sim::traffic::spawn` module's
/// "Mission ships").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionShip {
    /// Its `ShipCount`.
    pub count: i16,
    /// Its `ShipGoal`.
    pub goal: i16,
    /// Its `ShipSyst`, raw: the system they are in, encoded.
    pub syst: i16,
    /// Its `ShipDude`: the `düde` they are drawn from, none at -1.
    pub dude: Option<DudeId>,
    /// Its `ShipBehav`, raw.
    pub behav: i16,
    /// Its `ShipStart`, raw: where and when they appear.
    pub start: i16,
    /// Its `ShipNameID`: the `STR#` their name is drawn from, none at -1.
    pub name_id: Option<i16>,
    /// Its `ShipSubtitle`: the `STR#` their subtitle is drawn from, none
    /// at -1.
    pub subtitle: Option<i16>,
}

impl MissionShip {
    /// No special ships: a `ShipCount` of 0, no goal, a `ShipSyst` that
    /// names no system, no `düde`, no behaviour, names or subtitles.
    pub const NONE: Self = Self {
        count: 0,
        goal: -1,
        syst: 0,
        dude: None,
        behav: -1,
        start: 0,
        name_id: None,
        subtitle: None,
    };
}

/// A mission, raw from its `mïsn`: the fields its offer reads. The
/// `nova_sim::mission` rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionRecord {
    /// The `mïsn`'s ID.
    pub id: MissionId,
    /// Its `AvailStel`, raw: where it is offered.
    pub avail_stel: i16,
    /// Its `AvailLoc`, raw: the venue it is offered at.
    pub avail_loc: i16,
    /// Its `AvailRecord`, raw: the legal record it asks.
    pub avail_record: i16,
    /// Its `AvailRating`, raw: the combat rating it asks.
    pub avail_rating: i16,
    /// Its `AvailRandom`, raw: the percent chance a visit offers it.
    pub avail_random: i16,
    /// Its `AvailShipType`, raw: the ship it asks.
    pub avail_ship_type: i16,
    /// Its `AvailBits` control-bit test, parsed.
    pub avail_bits: Test,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `TravelStel`, raw: the destination.
    pub travel_stel: i16,
    /// Its `ReturnStel`, raw: where it is completed.
    pub return_stel: i16,
    /// Its `CargoType`, raw.
    pub cargo_type: i16,
    /// Its `CargoQty`, raw: tons, or a range below -1.
    pub cargo_qty: i16,
    /// Its `PickupMode`, raw: when its cargo is loaded (0 at the accept, 1
    /// at the destination, 2 boarding its special ship; any other never).
    pub pickup_mode: i16,
    /// Its `DropOffMode`, raw: when its cargo is unloaded (0 at the
    /// destination, 1 at the return; any other never).
    pub drop_off_mode: i16,
    /// Its `ScanMask`: the bits a scanning government's `ScanMask` finds
    /// its cargo by.
    pub scan_mask: u16,
    /// Its `PayVal`, raw.
    pub pay_val: i32,
    /// Its `TimeLimit`: the days it gives, none at 0 or below.
    pub time_limit: i16,
    /// Its `DispWeight`: higher shows nearer the top.
    pub disp_weight: i16,
    /// Its `OnAccept`, run as it is accepted.
    pub on_accept: Script,
    /// Its `OnRefuse`, run as its offer is refused.
    pub on_refuse: Script,
    /// Its `OnSuccess`, run as it is completed.
    pub on_success: Script,
    /// Its `OnFailure`, run as it fails.
    pub on_failure: Script,
    /// Its `OnAbort`, run as it is aborted.
    pub on_abort: Script,
    /// Its `OnShipDone`, run as its ship goals are met (see the
    /// `mission_ships` module of the session).
    pub on_ship_done: Script,
    /// Its `CanAbort`, raw: the player may abort it unless it is 0.
    pub can_abort: i16,
    /// Its `CompGovt`: the government whose record its end changes, none
    /// outside 128-383 (`_LoadCurrentMissionData` @0xa0c04-0xa0c6e).
    pub comp_govt: Option<GovtId>,
    /// Its `CompReward`: 0 with no `CompGovt`, as the original loads it.
    pub comp_reward: i16,
    /// Its `DatePostInc`: the days its success, or its auto-abort, passes.
    pub date_post_inc: i16,
    /// Its `ShipCount` and `ShipGoal`.
    pub ships: MissionShip,
    /// Its `AuxShipCount`, `AuxShipDude` and `AuxShipSyst`.
    pub aux: AuxShip,
}

/// A mission's auxiliary ships, raw from its `mïsn`: ordinary traffic of
/// its `AuxShipDude` that comes into the systems its `AuxShipSyst` allows
/// on a countdown (see the `nova_sim::mission::aux_ships` module).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuxShip {
    /// Its `AuxShipCount`.
    pub count: i16,
    /// Its `AuxShipDude`: the `düde` they are drawn from, none at -1.
    pub dude: Option<DudeId>,
    /// Its `AuxShipSyst`, raw: the systems they come into, encoded.
    pub syst: i16,
}

impl AuxShip {
    /// No auxiliary ships: an `AuxShipCount` of 0, no `düde`, and an
    /// `AuxShipSyst` that names no system.
    pub const NONE: Self = Self {
        count: 0,
        dude: None,
        syst: 0,
    };
}

/// A timed event, raw from its `crön`: the fields the daily cron pass
/// reads, which decides what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CronRecord {
    /// The `crön`'s ID.
    pub id: CronId,
    /// Its `FirstDay`, raw.
    pub first_day: i16,
    /// Its `FirstMonth`, raw.
    pub first_month: i16,
    /// Its `FirstYear`, raw.
    pub first_year: i16,
    /// Its `LastDay`, raw.
    pub last_day: i16,
    /// Its `LastMonth`, raw.
    pub last_month: i16,
    /// Its `LastYear`, raw.
    pub last_year: i16,
    /// Its `Random`, raw: the chance it starts on a day it may.
    pub random: i16,
    /// Its `Duration`, raw: the days it runs; below 0 disables it.
    pub duration: i16,
    /// Its `PreHoldoff`, raw: the days between starting and `OnStart`.
    pub pre_holdoff: i16,
    /// Its `PostHoldoff`, raw: the days after `OnEnd` before it may start
    /// again.
    pub post_holdoff: i16,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `EnableOn` control-bit test, parsed.
    pub enable_on: Test,
    /// Its `OnStart`, run as it starts.
    pub on_start: Script,
    /// Its `OnEnd`, run as it ends.
    pub on_end: Script,
    /// Its `Contribute` bits, given to the player while it runs.
    pub contribute: u64,
    /// Its `Require` bits, asked of the player before it starts.
    pub require: u64,
    /// Its `NewsGovt1-4`, raw: -1 or, in stock data, 0 for unused; the
    /// `nova_sim::news` module reads it.
    pub news_govt: [i16; 4],
    /// Its `GovtNewsStr1-4`, raw: -1 or, in stock data, 0 for unused; the
    /// `nova_sim::news` module reads it.
    pub govt_news: [i16; 4],
    /// Its `IndNewsStr`, raw: -1 or, in stock data, 0 for unused; the
    /// `nova_sim::news` module reads it.
    pub ind_news: i16,
}

/// A rank, from its `ränk` as `_LoadObjectData` keeps it
/// (@0x7da37-0x7dc4d): the fields the `nova_sim::rank` module
/// reads, which decides what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankRecord {
    /// The `ränk`'s ID.
    pub id: RankId,
    /// Its full name: the resource name up to its last ';', trailing
    /// spaces and ';'s dropped (@0x7db7d-0x7dbc0), empty when it has none.
    pub name: String,
    /// Its `Weight`, raw.
    pub weight: i16,
    /// Its `AffilGovt`: `None` for -1 or any ID up to 127, which the
    /// original loads as -1 (@0x7dab0-0x7dac3).
    pub affil_govt: Option<GovtId>,
    /// Its `Salary`, raw: credits a day, a fee when negative.
    pub salary: i64,
    /// Its `SalaryCap`, raw: 0 or less is no cap.
    pub salary_cap: i64,
    /// Its `Contribute` bits, given to the player while it is active.
    pub contribute: u64,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `ConvName`, read by `<PRK>`.
    pub conv_name: String,
    /// Its `ShortName`, read by `<SRK>`.
    pub short_name: String,
    /// Its `PriceMod`, a percentage of the shipyard's prices at its
    /// government's stellars; 0 or below loads as 100 (@0x7db33-0x7db3a).
    pub price_mod: i16,
}

/// The first `nëbu` the original has a slot for.
pub const FIRST_NEBULA: i16 = 128;
/// The last `nëbu` the original has a slot for: 32 slots in all.
pub const LAST_NEBULA: i16 = 159;

/// A nebula, from its `nëbu` as `_LoadObjectData` keeps it
/// (@0x7cf44-0x7d025): the fields the `nova_sim::nebula` module
/// reads, which decides what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NebulaRecord {
    /// The `nëbu`'s ID.
    pub id: NebulaId,
    /// Its `XPos`: the rectangle's left edge on the map.
    pub left: i16,
    /// Its `YPos`: the rectangle's top edge on the map.
    pub top: i16,
    /// Its `XSize`, raw.
    pub width: i16,
    /// Its `YSize`, raw.
    pub height: i16,
    /// Its `ActiveOn` control-bit test, parsed: blank holds.
    pub active_on: Test,
    /// Its `OnExplore`, run as an exploration first reaches it.
    pub on_explore: Script,
}

/// What a mission's screens show of it, read from its `mïsn` and its
/// `dësc`s (see the `nova_sim::mission` module's "Screens"): each
/// text as written, its tags not yet filled, and `None` for an ID of -1
/// or a `dësc` that cannot be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionTexts {
    /// The `mïsn`'s ID.
    pub id: MissionId,
    /// Its name as the game shows it: the resource name up to its last
    /// ';', trailing spaces and ';'s dropped (`_TrimResourceName`
    /// @0x717ac, from `_InitMissions` @0x980bb).
    pub name: String,
    /// Its offer text: `dësc` 4000 + the mission's index (ID - 128).
    pub offer: Option<String>,
    /// Its `BriefText`, shown as it is accepted.
    pub brief: Option<String>,
    /// Its `QuickBrief`, the mission info screen's text.
    pub quick_brief: Option<String>,
    /// Its `CompText`, shown as it is completed.
    pub comp: Option<String>,
    /// Its `FailText`, shown as it fails at its return.
    pub fail: Option<String>,
    /// Its `RefuseText`, shown as its offer is refused.
    pub refuse: Option<String>,
    /// Whether its `RefuseText` is not -1, whether or not its `dësc` can
    /// be read: what `_OfferOneMission` tests to hide the offer dialog
    /// and load the refuse text (@0xa2e70), so one naming a missing
    /// `dësc` hides the dialog and shows nothing.
    pub refuse_named: bool,
    /// Its `LoadCargText`, shown as its cargo is loaded.
    pub load_cargo: Option<String>,
    /// Its `DumpCargoText`, shown as its cargo is unloaded.
    pub dump_cargo: Option<String>,
    /// Its `ShipDoneText`, shown as its ship goals are met.
    pub ship_done: Option<String>,
    /// Its `AcceptButton`, raw.
    pub accept_button: String,
    /// Its `RefuseButton`, raw.
    pub refuse_button: String,
    /// The `Graphic` of each text's `dësc`, the `PICT` shown with it.
    pub graphics: TextGraphics,
    /// The movie each text's `dësc` names, played around its dialog.
    pub movies: TextMovies,
}

/// The `Graphic` of each of a mission's text `dësc`s, raw: none for -1,
/// or a `dësc` that is missing or does not decode. A `dësc` the original
/// cannot find reads as graphic 0 (`_LoadDescription` @0x71a7c-0x71a81),
/// which shows no picture either.
///
/// The view decides whether a graphic is shown: the original shows the
/// picture variant of a dialog for a graphic above 127 (`_OfferOneMission`
/// @0xa2344, `_ShowTextDialog` @0x5a89a).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextGraphics {
    /// The offer's, `dësc` 4000 + the mission's index.
    pub offer: Option<i16>,
    /// The `BriefText`'s.
    pub brief: Option<i16>,
    /// The `CompText`'s.
    pub comp: Option<i16>,
    /// The `FailText`'s.
    pub fail: Option<i16>,
    /// The `RefuseText`'s.
    pub refuse: Option<i16>,
    /// The `LoadCargText`'s.
    pub load_cargo: Option<i16>,
    /// The `DumpCargoText`'s.
    pub dump_cargo: Option<i16>,
    /// The `ShipDoneText`'s.
    pub ship_done: Option<i16>,
}

/// The QuickTime movie a `dësc` names (`MovieFile`), and how its `Flags`
/// play it: `_ShowTextDialog` (@0x5a76e-0x5a7ab, @0x5b40b-0x5b44f) and
/// `_OfferOneMission` (@0xa2273-0xa22ab, @0xa3580-0xa35b8) pass `Flags`
/// 0x0002 and 0x0004 to `_PlayQTMovie` (@0x55957).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescMovie {
    /// The movie file's name, as decoded: a bare file name.
    pub file: String,
    /// `Flags` 0x0001: it plays after the dialog, not before it.
    pub after: bool,
    /// `Flags` 0x0002: it plays at double size.
    pub double_size: bool,
    /// `Flags` 0x0004: it plays cinematic, the screen faded to black
    /// around it.
    pub cinematic: bool,
}

impl DescMovie {
    /// The movie of a `dësc` whose `MovieFile` is `file` and whose `Flags`
    /// are `flags`: none for an empty name, as each call site tests its
    /// first byte (@0xa2279, @0x5a784). Any other name is kept as it is,
    /// "\x01" too (`_LoadDescription` copies it as it stands,
    /// @0x71895-0x718ba).
    #[must_use]
    pub fn from_desc(file: &str, flags: u16) -> Option<Self> {
        (!file.is_empty()).then(|| Self {
            file: file.to_owned(),
            after: flags & 0x0001 != 0,
            double_size: flags & 0x0002 != 0,
            cinematic: flags & 0x0004 != 0,
        })
    }
}

/// The movie of each of a mission's text `dësc`s: none for -1, a `dësc`
/// that is missing or does not decode, or one with no `MovieFile`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextMovies {
    /// The offer's, `dësc` 4000 + the mission's index.
    pub offer: Option<DescMovie>,
    /// The `BriefText`'s.
    pub brief: Option<DescMovie>,
    /// The `QuickBrief`'s, which the mission info screen loads for the
    /// row selected (`_MissionInfoFilter` @0x9d89f) and never plays.
    pub quick_brief: Option<DescMovie>,
    /// The `CompText`'s.
    pub comp: Option<DescMovie>,
    /// The `FailText`'s.
    pub fail: Option<DescMovie>,
    /// The `RefuseText`'s.
    pub refuse: Option<DescMovie>,
    /// The `LoadCargText`'s.
    pub load_cargo: Option<DescMovie>,
    /// The `DumpCargoText`'s.
    pub dump_cargo: Option<DescMovie>,
    /// The `ShipDoneText`'s.
    pub ship_done: Option<DescMovie>,
}

impl MissionTexts {
    /// Mission `id` named `name`, with no text and blank labels.
    #[must_use]
    pub fn named(id: MissionId, name: &str) -> Self {
        Self {
            id,
            name: name.to_owned(),
            offer: None,
            brief: None,
            quick_brief: None,
            comp: None,
            fail: None,
            refuse: None,
            refuse_named: false,
            load_cargo: None,
            dump_cargo: None,
            ship_done: None,
            accept_button: String::new(),
            refuse_button: String::new(),
            graphics: TextGraphics::default(),
            movies: TextMovies::default(),
        }
    }
}

/// The game data a system's NPC traffic is spawned from: its `sÿst`'s
/// traffic, the `düde`s it names, the `flët`s and the `përs`.
pub trait TrafficCatalog {
    /// System `id`'s traffic, or `None` when it cannot be read.
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic>;
    /// System `id`'s reinforcement fleet ([`SystemReinforcements`]), or
    /// `None` when it has none or cannot be read. A catalog that keeps no
    /// reinforcements gives none.
    fn system_reinforcements(&self, _id: SystemId) -> Option<SystemReinforcements> {
        None
    }
    /// `düde` `id`, or `None` when it cannot be read.
    fn dude(&self, id: DudeId) -> Option<DudeRecord>;
    /// Every `flët` that can be read, by ascending ID.
    fn fleets(&self) -> Vec<FleetRecord>;
    /// Every `përs` that can be read, by ascending ID.
    fn persons(&self) -> Vec<PersonRecord>;
}

/// A borrowed catalog is a catalog.
impl<T: TrafficCatalog + ?Sized> TrafficCatalog for &T {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        (**self).system_traffic(id)
    }

    fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
        (**self).system_reinforcements(id)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).dude(id)
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        (**self).fleets()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        (**self).persons()
    }
}

/// A shared catalog is a catalog.
impl<T: TrafficCatalog + ?Sized> TrafficCatalog for Rc<T> {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        (**self).system_traffic(id)
    }

    fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
        (**self).system_reinforcements(id)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).dude(id)
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        (**self).fleets()
    }

    fn persons(&self) -> Vec<PersonRecord> {
        (**self).persons()
    }
}

/// A weapon, raw from its `wëap`: the `nova_sim::combat` rules
/// decide what the values mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeaponRecord {
    /// The `wëap`'s ID.
    pub id: WeaponId,
    /// Its `Reload`, in ticks.
    pub reload: i16,
    /// Its `Count`: a shot's or beam's life, in ticks.
    pub count: i16,
    /// Its `MassDmg`.
    pub mass_dmg: i16,
    /// Its `EnergyDmg`.
    pub energy_dmg: i16,
    /// Its `Guidance`.
    pub guidance: i16,
    /// Its `Speed`, in pixels a tick x100.
    pub speed: i16,
    /// Its `AmmoType`, encoded.
    pub ammo_type: i16,
    /// Its `Graphic`: the `spïn` its shots are drawn from, less 3000
    /// (`_SpawnShot` reads it at +0xa).
    pub graphic: i16,
    /// Its `Inaccuracy`, in degrees.
    pub inaccuracy: i16,
    /// Its `Impact`.
    pub impact: i16,
    /// Its `ExplodType`, encoded.
    pub explod_type: i16,
    /// Its `ProxRadius`, in pixels.
    pub prox_radius: i16,
    /// Its `BlastRadius`, in pixels.
    pub blast_radius: i16,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Seeker` flags.
    pub seeker: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `Flags3`.
    pub flags3: u16,
    /// Its `Decay`.
    pub decay: i16,
    /// Its `BeamLength`, in pixels.
    pub beam_length: i16,
    /// Its `BeamWidth`: a beam's width, and a spinning shot's passes a
    /// frame (`_HandleShot` @0x358a7).
    pub beam_width: i16,
    /// Its `Falloff`: how fast a beam's corona fades, and how long a
    /// decaying beam fades for.
    pub falloff: i16,
    /// Its `LiDensity`: above none for a lightning beam.
    pub li_density: i16,
    /// Its `BurstCount`.
    pub burst_count: i16,
    /// Its `BurstReload`, in ticks.
    pub burst_reload: i16,
    /// Its `GuidedTurn`: a homing shot's turn a tick, in tenths of a
    /// degree.
    pub guided_turn: i16,
    /// Its `Durability`: what point defence must take off a shot of it.
    pub durability: i16,
    /// Its `SubCount`: how many sub-munitions a shot releases.
    pub sub_count: i16,
    /// Its `SubType`: the sub-munitions' `wëap`, if any.
    pub sub_type: Option<WeaponId>,
    /// Its `SubTheta`, in degrees: the sub-munitions' spread, negative
    /// for a starburst.
    pub sub_theta: i16,
    /// Its `SubLimit`: how many generations of sub-munitions there may
    /// be; none or below for no limit.
    pub sub_limit: i16,
    /// Its `MaxAmmo`: how many rounds each launcher of it holds at most;
    /// none or below for no limit of its own.
    pub max_ammo: i16,
    /// Its `ProxSafety`: the ticks a shot of it flies before its fuse is
    /// armed.
    pub prox_safety: i16,
    /// Its `HitParticles`: how many particles a hit throws off.
    pub hit_particles: i16,
    /// Its `HitPartLife`: the hit particles' least life, in ticks.
    pub hit_part_life: i16,
    /// Its `HitPartVel`: the hit particles' speed, in hundredths of a
    /// pixel a tick.
    pub hit_part_vel: i16,
    /// Its `HitPartColor`, `00RRGGBB`.
    pub hit_part_color: u32,
    /// Its `Recoil`: the push its firer takes away from its heading, over
    /// the firer's mass; none at 0 or -1.
    pub recoil: i16,
    /// Its `ExitType`: which of its ship's exit points it fires from, 0
    /// to 3; any other value fires from the centre.
    pub exit_type: i16,
    /// Its `Ionization`: the ion charge it adds to a ship it hits.
    pub ionization: i16,
    /// Its `IonizeColor`, `00RRGGBB`.
    pub ionize_color: u32,
    /// Its `JamVuln1-4`: each jamming type's share, in percent, that a
    /// shot rolls its vulnerability to at launch (see
    /// `nova_sim::combat::jamming`).
    pub jam_vuln: [i16; 4],
    /// The frame size of its shots' sprite, the sheet of `spïn` 3000 +
    /// its `Graphic`, which point defence meets missiles by (see
    /// `nova_sim::combat::defence`); none for a `Graphic` below
    /// 0 or a sheet that cannot be read.
    pub frame: Option<FrameSize>,
}

/// A sprite frame's size, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSize {
    /// Its width.
    pub width: u32,
    /// Its height.
    pub height: u32,
}

/// One of a ship class's stock weapons, raw from its `shïp`: a `WeapType`
/// slot that names a weapon, with its `WeapCount` and `AmmoLoad`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StockWeapon {
    /// The `WeapType`.
    pub weapon: WeaponId,
    /// The `WeapCount`.
    pub count: i16,
    /// The `AmmoLoad`.
    pub ammo: i16,
}

/// A ship class's weapon exit points, raw from its `shän` (see
/// `nova_sim::combat::exits`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipExits {
    /// Each `ExitType`'s four (x, y) points, in pixels from the centre with
    /// the ship facing up: `GunPosX/Y`, `TurretPosX/Y`, `GuidedPosX/Y` and
    /// `BeamPosX/Y`, in that order.
    pub points: [[(i16, i16); 4]; 4],
    /// Each point's z offset, in the same order: `GunPosZ` to `BeamPosZ`.
    pub z: [[i16; 4]; 4],
    /// `UpCompressX` and `UpCompressY`, in percent.
    pub up_compress: (i16, i16),
    /// `DnCompressX` and `DnCompressY`, in percent.
    pub down_compress: (i16, i16),
    /// `FramesPer`: the frames in a turn.
    pub frames: i16,
}

/// A ship class's display fields, raw from its `shän`: what
/// `_HandleShipDisplay` picks the frames it draws by (see
/// `nova_sim::combat::ship_frames`). Its `FramesPer` and compression are
/// in its [`ShipExits`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipDisplay {
    /// Its `Flags`: 0x0001 banking, 0x0002 folding, 0x0004 key-carried,
    /// 0x0008 animated, 0x0010 animation stopped while disabled, 0x0020
    /// alt layer hidden while disabled, 0x0040 lights out while disabled,
    /// 0x0080 unfolding to fire, 0x0100 frame-angle table.
    pub flags: u16,
    /// Its `BaseSetCount`.
    pub base_sets: i16,
    /// Its `AnimDelay`, in frames.
    pub anim_delay: i16,
    /// Its `AltSetCount`.
    pub alt_sets: i16,
    /// Whether it names an alt image: `AltImageID` above 0.
    pub alt_image: bool,
    /// Its `BlinkMode` and `BlinkValA`-`D`, raw.
    pub blink: Blink,
    /// Whether it names a lights image: `LightImageID` above 0, without
    /// which `_HandleShipDisplay` runs no lights block (@0x2b681-0x2b6a2).
    pub lights_image: bool,
}

/// An escort's class, which the escort menu groups it by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EscortClass {
    /// 0: a fighter.
    Fighter,
    /// 1: a medium ship.
    Medium,
    /// 2: a warship.
    Warship,
    /// 3: a freighter; the default, the class a ship of no record works
    /// out to (`InherentAI` none).
    #[default]
    Freighter,
}

/// The heaviest `Mass` of a fighter whose class is worked out.
pub const FIGHTER_MASS: i16 = 49;
/// The heaviest `Mass` of a medium ship whose class is worked out.
pub const MEDIUM_MASS: i16 = 199;
/// The highest `InherentAI` of a freighter whose class is worked out.
pub const FREIGHTER_AI: i16 = 2;

impl EscortClass {
    /// Every class, in the escort menu's order.
    pub const ALL: [Self; 4] = [Self::Fighter, Self::Medium, Self::Warship, Self::Freighter];

    /// The class of a `shïp` of `EscortType` `escort_type`, `InherentAI`
    /// `inherent_ai` and `Mass` `mass` (see the `nova_sim::escort` module
    /// docs).
    #[must_use]
    pub fn of(escort_type: i16, inherent_ai: i16, mass: i16) -> Self {
        match escort_type {
            0 => Self::Fighter,
            1 => Self::Medium,
            2 => Self::Warship,
            3 => Self::Freighter,
            _ if inherent_ai <= FREIGHTER_AI => Self::Freighter,
            _ if mass <= FIGHTER_MASS => Self::Fighter,
            _ if mass <= MEDIUM_MASS => Self::Medium,
            _ => Self::Warship,
        }
    }

    /// Its row in the escort menu (`STR#` 2002 #140-#143).
    #[must_use]
    pub const fn menu_label(self) -> &'static str {
        match self {
            Self::Fighter => "Fighters",
            Self::Medium => "Medium Ships",
            Self::Warship => "Warships",
            Self::Freighter => "Freighters",
        }
    }

    /// What a command's message calls it (`STR#` 2002 #135-#138).
    #[must_use]
    pub const fn message_form(self) -> &'static str {
        match self {
            Self::Fighter => "Fighters",
            Self::Medium => "Medium ships",
            Self::Warship => "Warships",
            Self::Freighter => "Freighters",
        }
    }
}

/// What a ship class fights with and how it dies, raw from its `shïp` and
/// its `shän`: the `nova_sim::combat` rules decide what the values
/// mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HullRecord {
    /// The `shïp`'s ID.
    pub id: ShipId,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `Flags3`.
    pub flags3: u16,
    /// Its `DeathDelay`, in ticks.
    pub death_delay: i16,
    /// Its `PodCount`, raw: the crew pods a dying ship of the type ejects
    /// (see `nova_sim::npc_escape`).
    pub pod_count: i16,
    /// Its `Explode1`, encoded.
    pub explode1: i16,
    /// Its `Explode2`, encoded.
    pub explode2: i16,
    /// Its `Mass`, in tons.
    pub mass: i16,
    /// Its `WeapType` slots 1-8 that name a weapon, in slot order.
    pub weapons: Vec<StockWeapon>,
    /// Its `shän`'s `BaseXSize`, in pixels, or `None` when it has no
    /// `shän` that can be read.
    pub size: Option<i16>,
    /// Its `shän`'s `BaseYSize`, in pixels, or `None` when it has no
    /// `shän` that can be read.
    pub height: Option<i16>,
    /// Its `Strength`: how strong it counts in a fight.
    pub strength: i16,
    /// Its `KeyCarried`: the ship a weapon of `Flags2` 0x0080 needs
    /// aboard, raw; none for -1.
    pub key_carried: Option<ShipId>,
    /// Its `Cost`, raw: -1 hides a ship of the type (see
    /// `nova_sim::combat::hull::HullSpec::hidden`).
    pub cost: i32,
    /// Its `Speed`, raw (see
    /// `nova_sim::combat::hull::HullSpec::class_speed`).
    pub speed: i16,
    /// Its `shän`'s weapon exit points, or `None` when it has no `shän`
    /// that can be read.
    pub exits: Option<ShipExits>,
    /// Its `shän`'s display fields, or `None` when it has no `shän` that
    /// can be read.
    pub display: Option<ShipDisplay>,
    /// Its escort class, from its `EscortType`, `InherentAI` and `Mass`
    /// ([`EscortClass::of`]), which sets which turret shots miss it (see
    /// `nova_sim::combat::hull::HullSpec::miss_class`).
    pub escort_class: EscortClass,
}

/// A government's penalties for crimes against its ships, raw from its
/// `gövt`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Penalties {
    /// Its `SmugPenalty`.
    pub smuggle: i16,
    /// Its `DisabPenalty`.
    pub disable: i16,
    /// Its `BoardPenalty`.
    pub board: i16,
    /// Its `KillPenalty`.
    pub kill: i16,
    /// Its `ShootPenalty`.
    pub shoot: i16,
}

/// A government, raw from its `gövt`: the `nova_sim::govt` and
/// `nova_sim::legal` rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovtRecord {
    /// The `gövt`'s ID.
    pub id: GovtId,
    /// Its name as the game shows it: the resource name cut before its
    /// last ';', with the spaces and ';'s before that dropped, and at most
    /// 30 characters (`_LoadObjectData` @0x7bde9-0x7be51; stock 186
    /// "Federation;hates Temmin Shard" shows "Federation"). Empty for an
    /// unnamed resource.
    pub name: String,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `CrimeTol`.
    pub crime_tol: i16,
    /// Its `InitialRec`: the legal record a system it governs starts
    /// with (`_ResetPlayer` @0x1d9a4-0x1da0b reads it at +0x52).
    pub initial_rec: i16,
    /// Its penalties.
    pub penalties: Penalties,
    /// Its `MaxOdds`, in percent.
    pub max_odds: i16,
    /// Its `Class1-4`: -1 for an unused slot.
    pub classes: [i16; 4],
    /// Its `Ally1-4`: the classes it is allied with, -1 for an unused
    /// slot.
    pub allies: [i16; 4],
    /// Its `Enemy1-4`: the classes it is at war with, -1 for an unused
    /// slot.
    pub enemies: [i16; 4],
    /// Its `CommName`: what its ships are called by when hailed.
    pub comm_name: String,
    /// Its `MediumName` (`gövt` +0x64, which `_LoadObjectData` copies into
    /// the name table at `*0x3b5554`, @0x7bea1-0x7bea8): the name the
    /// reinforcement line gives its fleet (see
    /// `nova_sim::ai::reinforcements`).
    pub medium_name: String,
    /// Its `ScanMask`: the bits it finds a mission's cargo by.
    pub scan_mask: u16,
    /// Its `ScanFine`: the credits a smuggler is fined, or below 0 that
    /// percent of the cash.
    pub scan_fine: i16,
    /// Its `NewsPic`: the `PICT` the news is drawn over at a stellar it
    /// governs (`gövt` +0xAE, kept raw; `_DoNewsDialog` reads it at
    /// runtime +0x44).
    pub news_pic: i16,
    /// Its `Require`: the 64 bits a pilot must contribute to land at a
    /// stellar it governs (`gövt` +0x54; `_PlayerMeetsRequirements`
    /// @0x776f, read by `_HandlePlayerDockRequest` @0x67940-0x6796b).
    pub require: u64,
    /// Its `InhJam1-4`: the jamming of each type its ships' classes take
    /// through their `InherentGovt` (`gövt` runtime +0x56; see
    /// `nova_sim::combat::jamming`).
    pub inh_jam: [i16; 4],
    /// Its `SkillMult`, raw: the factor its ships' top speed and thrust
    /// take is `SkillMult` / 100, or 1.0 when it is 0 or less (runtime
    /// +0x64, `_LoadObjectData` @0x7bb69-0x7bb9b; see
    /// `nova_sim::skill`).
    pub skill_mult: i16,
}

/// The game data a session's ships fight with: the `wëap`s, each
/// `shïp`'s combat fields, and the `gövt`s.
pub trait CombatCatalog {
    /// Every `wëap` that can be read, by ascending ID.
    fn weapons(&self) -> Vec<WeaponRecord>;
    /// Every `shïp` that can be read, by ascending ID, with its `shän`'s
    /// size.
    fn hulls(&self) -> Vec<HullRecord>;
    /// Every `gövt` that can be read, by ascending ID.
    fn governments(&self) -> Vec<GovtRecord>;
}

/// A borrowed catalog is a catalog.
impl<T: CombatCatalog + ?Sized> CombatCatalog for &T {
    fn weapons(&self) -> Vec<WeaponRecord> {
        (**self).weapons()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        (**self).hulls()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        (**self).governments()
    }
}

/// A shared catalog is a catalog.
impl<T: CombatCatalog + ?Sized> CombatCatalog for Rc<T> {
    fn weapons(&self) -> Vec<WeaponRecord> {
        (**self).weapons()
    }

    fn hulls(&self) -> Vec<HullRecord> {
        (**self).hulls()
    }

    fn governments(&self) -> Vec<GovtRecord> {
        (**self).governments()
    }
}

/// How many asteroid types there are: `röid` 128-143, one bit each of
/// `AstTypes`.
pub const KINDS: usize = 16;

/// One asteroid type, `röid` 128 + `kind`, with its master sprite's
/// frames and frame size (see `nova_sim::asteroid`), raw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsteroidTypeRecord {
    /// Its type, 0-15: `röid` 128 + `kind`, bit `kind` of a system's
    /// `AstTypes`.
    pub kind: u8,
    /// Its `Strength`, raw.
    pub strength: i16,
    /// Its `SpinRate`, raw: frames a tick, in hundredths.
    pub spin_rate: i16,
    /// Its master sprite's frame count, `spïn` 800 + `kind`'s `xTiles` ×
    /// `yTiles`: 0 when it does not read or is not above 0.
    pub frames: u16,
    /// Its master sprite's frame width and height, when the sheet reads.
    pub frame_size: Option<(u32, u32)>,
    /// Its `YieldType`, raw: 0-6 a commodity, 1000-1127 a `jünk`.
    pub yield_type: i16,
    /// Its `YieldQty`, raw: the boxes it drops on average.
    pub yield_qty: i16,
    /// Its `PartCount`: the particles it throws off as it is destroyed.
    pub part_count: i16,
    /// Its `PartColor`, `00RRGGBB`.
    pub part_color: u32,
    /// Its `FragType1` and `FragType2`, raw: a `röid` ID (or a kind
    /// 0-15), -1 for none.
    pub frag_types: [i16; 2],
    /// Its `FragCount`: the fragments it breaks into on average.
    pub frag_count: i16,
    /// Its `ExplodeType`, encoded: -1 for none.
    pub explode_type: i16,
    /// Its `Mass`: what a shot's `Impact` pushes against.
    pub mass: i16,
}

/// A record of kind 0 with nothing set: no strength, spin, frames or
/// sprite, no particles or mass, and -1 (none) for its `YieldType`,
/// `FragType`s and `ExplodeType`.
impl Default for AsteroidTypeRecord {
    fn default() -> Self {
        Self {
            kind: 0,
            strength: 0,
            spin_rate: 0,
            frames: 0,
            frame_size: None,
            yield_type: -1,
            yield_qty: 0,
            part_count: 0,
            part_color: 0,
            frag_types: [-1, -1],
            frag_count: 0,
            explode_type: -1,
            mass: 0,
        }
    }
}

/// One `sÿst`'s asteroid field: its `Asteroids` and `AstTypes`, raw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AsteroidFieldRecord {
    /// The system.
    pub system: SystemId,
    /// Its `Asteroids`: how many asteroids it holds at most, 0-16.
    pub count: i16,
    /// Its `AstTypes`: bit `n` set for each `röid` 128 + `n` it holds.
    pub types: u16,
}

/// The game data a session's asteroid fields are read from (see
/// `nova_sim::asteroid`).
pub trait AsteroidCatalog {
    /// `röid` 128 + `kind`, `kind` 0-15, for each that reads, by kind,
    /// with `spïn` 800 + `kind`'s frames and frame size.
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord>;
    /// Every `sÿst` that reads: its `Asteroids` and `AstTypes`, raw.
    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord>;
    /// The frame width and height of each box graphic 0-4, `spïn` 500 +
    /// graphic, by graphic: none where the sheet does not read. A scoop
    /// tests a ship against the box's frame (see `nova_sim::scoop`).
    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS];
}

/// The cargo boxes' graphics: `spïn` 500-504.
pub const BOX_GRAPHICS: usize = 5;

/// A borrowed catalog is a catalog.
impl<T: AsteroidCatalog + ?Sized> AsteroidCatalog for &T {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        (**self).asteroid_types()
    }

    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
        (**self).asteroid_fields()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS] {
        (**self).box_frame_sizes()
    }
}

/// A shared catalog is a catalog.
impl<T: AsteroidCatalog + ?Sized> AsteroidCatalog for Rc<T> {
    fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
        (**self).asteroid_types()
    }

    fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
        (**self).asteroid_fields()
    }

    fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS] {
        (**self).box_frame_sizes()
    }
}

/// The game's string lists (`STR#`), read whole: a hailed ship's words
/// come from them, and the names the `T` set operator gives the ship.
pub trait CommCatalog {
    /// Every string of `STR#` `id`, in order; none when it is missing or
    /// cannot be read.
    fn string_list(&self, id: i16) -> Vec<String>;

    /// Whether the data holds any `l33t` resource, which turns on the
    /// original's `_leetMode` (`_LoadObjectData` @0x7e28c-0x7e2a2; see
    /// `nova_sim::text_tags`). None by default.
    fn leet_mode(&self) -> bool {
        false
    }

    /// The `'STR '` `id`, raw, from whichever open file holds it, the base
    /// data's or a plug-in's, as `_LoadPluginString` (@0x71a8e) looks it up
    /// with `GetResource`: a person's hail quote's patch (see
    /// `nova_sim::person::hail_quote_text`), and one string
    /// of a `STR#` (see [`strings`](crate::strings)). None by default.
    fn plugin_string(&self, id: i16) -> StringPatch {
        let _ = id;
        StringPatch::Absent
    }
}

/// A borrowed catalog is a catalog.
impl<T: CommCatalog + ?Sized> CommCatalog for &T {
    fn string_list(&self, id: i16) -> Vec<String> {
        (**self).string_list(id)
    }

    fn leet_mode(&self) -> bool {
        (**self).leet_mode()
    }

    fn plugin_string(&self, id: i16) -> StringPatch {
        (**self).plugin_string(id)
    }
}

/// A shared catalog is a catalog.
impl<T: CommCatalog + ?Sized> CommCatalog for Rc<T> {
    fn string_list(&self, id: i16) -> Vec<String> {
        (**self).string_list(id)
    }

    fn leet_mode(&self) -> bool {
        (**self).leet_mode()
    }

    fn plugin_string(&self, id: i16) -> StringPatch {
        (**self).plugin_string(id)
    }
}

/// Why a flight session could not start. Each message is ready to display.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StartError {
    /// There is no `chär` at all.
    #[error("no chär to start from")]
    NoCharacter,
    /// The first `chär` does not decode; the decoder's message.
    #[error("{0}")]
    Character(String),
    /// The first `chär` names no ship.
    #[error("the first chär has no ship")]
    NoShip,
    /// The starting ship cannot be read; why.
    #[error("{1}")]
    Ship(ShipId, String),
    /// None of the first `chär`'s starting systems exists.
    #[error("none of the first chär's starting systems ({}) exists", slots(.0))]
    NoStartingSystem([Option<SystemId>; 4]),
    /// The system a saved pilot is in no longer exists.
    #[error("the pilot's system, sÿst {}, does not exist", .0.0)]
    NoSystem(SystemId),
}

/// The starting system slots as text: each ID, or "none".
fn slots(systems: &[Option<SystemId>; 4]) -> String {
    systems
        .iter()
        .map(|slot| slot.map_or_else(|| "none".to_owned(), |id| id.0.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The game data a flight session starts from.
pub trait PilotCatalog {
    /// The first `chär` by ascending ID: its ship and starting systems.
    fn first_character(&self) -> Result<CharacterStart, StartError>;
    /// Ship `id`'s handling and reserve fields, or why they cannot be read.
    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String>;
    /// Ship `id`'s default outfits, its `DefaultItems`, raw: each item's
    /// `oütf` ID with its count, in slot order, a negative count as none.
    /// None for a ship that cannot be read.
    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)>;
    /// Every `oütf` that can be read, by ascending ID.
    fn outfits(&self) -> Vec<OutfitRecord>;
    /// Every `shïp` that can be read, by ascending ID.
    fn ships(&self) -> Vec<ShipRecord>;
    /// Whether system `id` exists and can be read.
    fn system_exists(&self, id: SystemId) -> bool;
    /// The stellars of system `id` that can be read, in its `nav_def`
    /// order; none for a system that cannot be read.
    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite>;
    /// Every system that can be read, by ascending ID, with its map
    /// position and hyperlinks.
    fn star_map(&self) -> Vec<StarSystem>;
    /// The standard commodities' names and base prices.
    fn commodity_strings(&self) -> CommodityStrings;
    /// The `'STR '` `id`, raw, from whichever file holds it, the base
    /// data's or a plug-in's: a message buoy's patch (see the
    /// [`buoy`](crate::buoy) module).
    fn string_patch(&self, id: i16) -> StringPatch;
    /// Every string of `STR#` 1000, the message buoys', in order; none
    /// when it is missing.
    fn buoy_strings(&self) -> Vec<String>;
    /// Every `jünk` that can be read, by ascending ID.
    fn junk(&self) -> Vec<JunkRecord>;
    /// Every `öops` that can be read, by ascending ID.
    fn disasters(&self) -> Vec<DisasterRecord>;
    /// Every `spöb` that can be read, by ascending ID, with its `Flags`,
    /// raw.
    fn stellar_flags(&self) -> Vec<(StellarId, u32)>;
    /// Every `spöb` that can be read, by ascending ID, whose `Weapon` names
    /// a `wëap` (128 or more), with it. A `Weapon` of 127 or less is none
    /// (`_LoadObjectData` @0x7779f-0x777b2): such a stellar has no gun.
    fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)>;
    /// The first `chär`'s `DatePrefix` and `DateSuffix`, or none (both
    /// empty) when there is no `chär` or it does not decode.
    fn date_affixes(&self) -> DateAffixes;
    /// Every stellar some system that can be read lists, that can be read
    /// itself, by ascending ID: where a hypergate or wormhole may lead. The
    /// system each is in is resolved from the listed stellars
    /// ([`PilotCatalog::listed_stellars`]), owner-first (see the
    /// `nova_sim::gate` module).
    fn gate_sites(&self) -> Vec<GateSite>;
    /// Every `spöb` that can be read, by ascending ID, with what its
    /// destruction and regeneration read.
    fn stellar_lives(&self) -> Vec<StellarLife>;
    /// Every `spöb` that can be read, by ascending ID, with the scripts the
    /// planet comm dialog runs as it dominates and releases it (see the
    /// `nova_sim::planet_comm` module).
    fn stellar_scripts(&self) -> Vec<StellarScripts>;
    /// The frame width and height of graphic type 0's sheet (`spïn`
    /// 1000), or `None` when it cannot be read: the frames the original's
    /// rebuild gives a sprite slot it left empty.
    fn fallback_frame_size(&self) -> Option<(u32, u32)>;
    /// Whether graphic type 0's `spïn` (1000) gives a frame count above 0,
    /// `xTiles` × `yTiles`: a slot the original's rebuild fills with it
    /// shows no frame when it does not (`_SWSetCurrentFrameIndex`
    /// @0xa90e7-0xa90ee, called @0x332d0; stellar-fidelity phase 8).
    /// Means something only beside a [`fallback_frame_size`] that is
    /// `Some`. `true` unless overridden, as every stock `spïn` counts.
    ///
    /// [`fallback_frame_size`]: PilotCatalog::fallback_frame_size
    fn fallback_counts_frames(&self) -> bool {
        true
    }
    /// Every `mïsn` that can be read, by ascending ID.
    fn missions(&self) -> Vec<MissionRecord>;
    /// What the screens show of every `mïsn` that can be read, by
    /// ascending ID.
    fn mission_texts(&self) -> Vec<MissionTexts>;
    /// Every stellar some system that can be read lists, that can be read
    /// itself, by ascending ID, with every system listing it.
    fn listed_stellars(&self) -> Vec<ListedStellar>;
    /// Every system that can be read, by ascending ID, with its
    /// `Visibility`.
    fn system_visibility(&self) -> Vec<SystemVisibility>;
    /// Every stellar's and system's name that can be read.
    fn place_names(&self) -> PlaceNames;
    /// Every `spöb` that can be read, by ascending ID, with the place and
    /// name a mission's stop is matched by.
    fn stellar_marks(&self) -> Vec<StellarMark>;
    /// Every `crön` that can be read, by ascending ID.
    fn crons(&self) -> Vec<CronRecord>;
    /// Every `ränk` 128-255 that can be read, by ascending ID.
    fn ranks(&self) -> Vec<RankRecord>;
    /// Every `nëbu` 128-159 that can be read, by ascending ID.
    fn nebulae(&self) -> Vec<NebulaRecord>;
    /// The length of the warp-up sound, `snd ` 128, which the warp's
    /// clock reads its duration from
    /// ([`sound_duration`](crate::warp::sound_duration)); none when there
    /// is none or it cannot be read, as by default.
    fn warp_sound(&self) -> Option<SoundLength> {
        None
    }
    /// The length of the double-time warp-up sound, `snd ` 129 "Warp up
    /// x2", whose duration the warp's clock reads in double time
    /// ([`warp_sound_duration`](crate::warp::warp_sound_duration)); none
    /// when there is none or it cannot be read, as by default.
    fn warp_sound_double_time(&self) -> Option<SoundLength> {
        None
    }
    /// The length of the warp-out sound, `snd ` 130, which every
    /// arrival plays and whose play state decides whether a jump's
    /// natural end plays it again
    /// ([`WarpOut`](crate::warp::WarpOut)); none when there is none or it
    /// cannot be read, as by default.
    fn warp_out_sound(&self) -> Option<SoundLength> {
        None
    }
    /// Each `bööm` that can be read and has a sound, by ascending ID, with
    /// the `snd ` its explosion plays ([`explosion_sound`]): what a `Y`'s
    /// explosion sounds (see `nova_sim`'s `y_explosion` module). None by
    /// default.
    fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
        Vec::new()
    }
    /// The escorts' voices: every voice clip, `snd ` 1000-1799, with its
    /// length, and each `gövt`'s `VoiceType` (see the
    /// [`voice`](crate::voice) module); none by default, no clip and no
    /// government's voice, so no escort speaks.
    fn voices(&self) -> Voices {
        Voices::default()
    }
    /// `düde` `id`, as [`TrafficCatalog::dude`] reads it: what a mission
    /// of `Flags` 0x0800 draws its ships' one type from at accept (see
    /// `nova_sim::mission::SpecialShips::uniform_type`).
    /// None when it cannot be read, as by default: the type is then drawn
    /// at the ships' first spawn.
    fn mission_dude(&self, _id: DudeId) -> Option<DudeRecord> {
        None
    }
    /// Each system's first stellar that can be read, the first of its
    /// [`PilotCatalog::landing_sites`], by ascending system ID; a system
    /// with none is left out. A landed `M` selects it (see the `nav`
    /// module of `nova_sim::Session`): the original's first
    /// `NavDefs` slot that isn't −1, as its loader clears the slot of a
    /// `spöb` that didn't load (@0x780f8-0x7817f). By default it is read
    /// through [`PilotCatalog::star_map`] and the landing sites of each
    /// system that exists; a catalog that reads sprites for its landing
    /// sites reads it without them.
    fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
        self.star_map()
            .into_iter()
            .filter(|system| self.system_exists(system.id))
            .filter_map(|system| Some((system.id, self.landing_sites(system.id).first()?.id)))
            .collect()
    }
}

/// A borrowed catalog is a catalog.
impl<T: PilotCatalog + ?Sized> PilotCatalog for &T {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        (**self).default_outfits(id)
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        (**self).outfits()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        (**self).ships()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        (**self).landing_sites(system)
    }

    fn star_map(&self) -> Vec<StarSystem> {
        (**self).star_map()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        (**self).commodity_strings()
    }

    fn string_patch(&self, id: i16) -> StringPatch {
        (**self).string_patch(id)
    }

    fn buoy_strings(&self) -> Vec<String> {
        (**self).buoy_strings()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        (**self).stellar_flags()
    }

    fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
        (**self).stellar_weapons()
    }

    fn date_affixes(&self) -> DateAffixes {
        (**self).date_affixes()
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        (**self).gate_sites()
    }

    fn stellar_lives(&self) -> Vec<StellarLife> {
        (**self).stellar_lives()
    }

    fn stellar_scripts(&self) -> Vec<StellarScripts> {
        (**self).stellar_scripts()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        (**self).fallback_frame_size()
    }

    fn fallback_counts_frames(&self) -> bool {
        (**self).fallback_counts_frames()
    }

    fn missions(&self) -> Vec<MissionRecord> {
        (**self).missions()
    }

    fn mission_texts(&self) -> Vec<MissionTexts> {
        (**self).mission_texts()
    }

    fn listed_stellars(&self) -> Vec<ListedStellar> {
        (**self).listed_stellars()
    }

    fn system_visibility(&self) -> Vec<SystemVisibility> {
        (**self).system_visibility()
    }

    fn place_names(&self) -> PlaceNames {
        (**self).place_names()
    }

    fn stellar_marks(&self) -> Vec<StellarMark> {
        (**self).stellar_marks()
    }

    fn crons(&self) -> Vec<CronRecord> {
        (**self).crons()
    }

    fn ranks(&self) -> Vec<RankRecord> {
        (**self).ranks()
    }

    fn nebulae(&self) -> Vec<NebulaRecord> {
        (**self).nebulae()
    }

    fn warp_sound(&self) -> Option<SoundLength> {
        (**self).warp_sound()
    }

    fn warp_sound_double_time(&self) -> Option<SoundLength> {
        (**self).warp_sound_double_time()
    }

    fn warp_out_sound(&self) -> Option<SoundLength> {
        (**self).warp_out_sound()
    }

    fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
        (**self).boom_sounds()
    }

    fn voices(&self) -> Voices {
        (**self).voices()
    }

    fn mission_dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).mission_dude(id)
    }

    fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
        (**self).first_stellars()
    }
}

/// A shared catalog is a catalog, so the sim and the views can read the
/// same game data.
impl<T: PilotCatalog + ?Sized> PilotCatalog for Rc<T> {
    fn first_character(&self) -> Result<CharacterStart, StartError> {
        (**self).first_character()
    }

    fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
        (**self).ship_fields(id)
    }

    fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
        (**self).default_outfits(id)
    }

    fn outfits(&self) -> Vec<OutfitRecord> {
        (**self).outfits()
    }

    fn ships(&self) -> Vec<ShipRecord> {
        (**self).ships()
    }

    fn system_exists(&self, id: SystemId) -> bool {
        (**self).system_exists(id)
    }

    fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
        (**self).landing_sites(system)
    }

    fn star_map(&self) -> Vec<StarSystem> {
        (**self).star_map()
    }

    fn commodity_strings(&self) -> CommodityStrings {
        (**self).commodity_strings()
    }

    fn string_patch(&self, id: i16) -> StringPatch {
        (**self).string_patch(id)
    }

    fn buoy_strings(&self) -> Vec<String> {
        (**self).buoy_strings()
    }

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
    }

    fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
        (**self).stellar_flags()
    }

    fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
        (**self).stellar_weapons()
    }

    fn date_affixes(&self) -> DateAffixes {
        (**self).date_affixes()
    }

    fn gate_sites(&self) -> Vec<GateSite> {
        (**self).gate_sites()
    }

    fn stellar_lives(&self) -> Vec<StellarLife> {
        (**self).stellar_lives()
    }

    fn stellar_scripts(&self) -> Vec<StellarScripts> {
        (**self).stellar_scripts()
    }

    fn fallback_frame_size(&self) -> Option<(u32, u32)> {
        (**self).fallback_frame_size()
    }

    fn fallback_counts_frames(&self) -> bool {
        (**self).fallback_counts_frames()
    }

    fn missions(&self) -> Vec<MissionRecord> {
        (**self).missions()
    }

    fn mission_texts(&self) -> Vec<MissionTexts> {
        (**self).mission_texts()
    }

    fn listed_stellars(&self) -> Vec<ListedStellar> {
        (**self).listed_stellars()
    }

    fn system_visibility(&self) -> Vec<SystemVisibility> {
        (**self).system_visibility()
    }

    fn place_names(&self) -> PlaceNames {
        (**self).place_names()
    }

    fn stellar_marks(&self) -> Vec<StellarMark> {
        (**self).stellar_marks()
    }

    fn crons(&self) -> Vec<CronRecord> {
        (**self).crons()
    }

    fn ranks(&self) -> Vec<RankRecord> {
        (**self).ranks()
    }

    fn nebulae(&self) -> Vec<NebulaRecord> {
        (**self).nebulae()
    }

    fn warp_sound(&self) -> Option<SoundLength> {
        (**self).warp_sound()
    }

    fn warp_sound_double_time(&self) -> Option<SoundLength> {
        (**self).warp_sound_double_time()
    }

    fn warp_out_sound(&self) -> Option<SoundLength> {
        (**self).warp_out_sound()
    }

    fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
        (**self).boom_sounds()
    }

    fn voices(&self) -> Voices {
        (**self).voices()
    }

    fn mission_dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).mission_dude(id)
    }

    fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
        (**self).first_stellars()
    }
}

/// `items`, each an outfit with a count, as how many of each: repeats add
/// up, saturating, and none of an item is not listed.
pub fn tally(items: impl IntoIterator<Item = (OutfitId, u16)>) -> BTreeMap<OutfitId, u16> {
    let mut outfits = BTreeMap::new();
    for (id, count) in items {
        let owned: &mut u16 = outfits.entry(id).or_default();
        *owned = owned.saturating_add(count);
    }
    outfits.retain(|_, count| *count > 0);
    outfits
}

/// Every mod of the outfits `owned`, each record's four `ModType`s and
/// `ModVal`s with how many are owned. An outfit with no record has none.
#[must_use]
pub fn outfit_mods(owned: &BTreeMap<OutfitId, u16>, records: &[OutfitRecord]) -> Vec<OutfitMod> {
    records
        .iter()
        .filter_map(|record| Some((record, *owned.get(&record.id)?)))
        .flat_map(|(record, count)| {
            record.mods.map(|(mod_type, mod_val)| OutfitMod {
                mod_type,
                mod_val,
                count,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tally_adds_up_each_outfits_repeats() {
        let tallied = tally([(OutfitId(130), 2), (OutfitId(128), 1), (OutfitId(130), 3)]);
        assert_eq!(
            tallied,
            BTreeMap::from([(OutfitId(128), 1), (OutfitId(130), 5)])
        );
    }

    #[test]
    fn a_tally_saturates() {
        let tallied = tally([(OutfitId(128), u16::MAX - 1), (OutfitId(128), 2)]);
        assert_eq!(tallied, BTreeMap::from([(OutfitId(128), u16::MAX)]));
    }

    #[test]
    fn a_tally_lists_no_outfit_of_none() {
        let tallied = tally([(OutfitId(128), 0), (OutfitId(129), 1), (OutfitId(128), 0)]);
        assert_eq!(tallied, BTreeMap::from([(OutfitId(129), 1)]));
    }

    #[test]
    fn the_owned_outfits_mods_are_each_records_four_with_its_count() {
        let records = [
            crate::fixture::outfit(128, &[(1, 100), (2, 50)]),
            crate::fixture::outfit(129, &[(3, 10)]),
            crate::fixture::outfit(130, &[(4, -5), (5, 6), (6, 7), (7, 8)]),
        ];
        let owned = BTreeMap::from([(OutfitId(128), 2), (OutfitId(130), 1)]);
        let at = |mod_type, mod_val, count| OutfitMod {
            mod_type,
            mod_val,
            count,
        };
        assert_eq!(
            outfit_mods(&owned, &records),
            [
                at(1, 100, 2),
                at(2, 50, 2),
                at(0, 0, 2),
                at(0, 0, 2),
                at(4, -5, 1),
                at(5, 6, 1),
                at(6, 7, 1),
                at(7, 8, 1),
            ]
        );
    }

    #[test]
    fn no_outfit_owned_or_none_with_a_record_gives_no_mods() {
        let records = [crate::fixture::outfit(128, &[(1, 100)])];
        assert_eq!(outfit_mods(&BTreeMap::new(), &records), []);
        let unlisted = BTreeMap::from([(OutfitId(999), 3)]);
        assert_eq!(outfit_mods(&unlisted, &records), []);
    }

    // `_LoadPluginString` (@0x71a8e-0x71af4) with its `GetIndString`
    // fallback.

    #[test]
    fn a_str_patch_shows_its_text_or_an_empty_line_and_absent_shows_nothing() {
        assert_eq!(StringPatch::Text("patched".into()).shown(), Some("patched"));
        assert_eq!(StringPatch::Empty.shown(), Some(""));
        assert_eq!(StringPatch::Unreadable.shown(), Some(""));
        assert_eq!(StringPatch::Absent.shown(), None);
    }

    #[test]
    fn a_patched_string_is_the_patch_else_the_lists_line_from_one() {
        let lines = ["one", "two", "three"].map(str::to_owned);
        let string = |index, patch| patched_string(index, patch, &lines);
        assert_eq!(string(2, StringPatch::Text("p".into())), Some("p".into()));
        assert_eq!(string(9, StringPatch::Text("p".into())), Some("p".into()));
        assert_eq!(string(2, StringPatch::Empty), Some(String::new()));
        assert_eq!(string(2, StringPatch::Unreadable), Some(String::new()));
        assert_eq!(string(1, StringPatch::Absent), Some("one".into()));
        assert_eq!(string(3, StringPatch::Absent), Some("three".into()));
        for index in [0, -1, 4, i16::MIN, i16::MAX] {
            assert_eq!(string(index, StringPatch::Absent), None, "{index}");
        }
        assert_eq!(patched_string(1, StringPatch::Absent, &[]), None);
    }

    #[test]
    fn a_patch_id_is_the_index_past_its_offset_and_none_past_the_word() {
        assert_eq!(patch_id(1, 999), Some(1000));
        assert_eq!(patch_id(-1000, 999), Some(-1));
        assert_eq!(patch_id(i16::MAX - 4999, 4999), Some(i16::MAX));
        assert_eq!(patch_id(i16::MAX - 4998, 4999), None);
        assert_eq!(patch_id(i16::MIN, -1), None);
    }

    #[test]
    fn a_desc_with_no_movie_name_has_no_movie() {
        assert_eq!(DescMovie::from_desc("", 0x0007), None);
    }

    #[test]
    fn each_movie_flag_sets_its_own_field_alone() {
        let movie = |flags| DescMovie::from_desc("Race 1.mov", flags);
        let plain = DescMovie {
            file: "Race 1.mov".to_owned(),
            after: false,
            double_size: false,
            cinematic: false,
        };
        assert_eq!(movie(0), Some(plain.clone()));
        assert_eq!(
            movie(0x0001),
            Some(DescMovie {
                after: true,
                ..plain.clone()
            })
        );
        assert_eq!(
            movie(0x0002),
            Some(DescMovie {
                double_size: true,
                ..plain.clone()
            })
        );
        assert_eq!(
            movie(0x0004),
            Some(DescMovie {
                cinematic: true,
                ..plain.clone()
            })
        );
        assert_eq!(movie(0xfff8), Some(plain), "the other bits mean nothing");
    }

    #[test]
    fn a_movie_name_of_one_control_character_is_a_name() {
        assert_eq!(
            DescMovie::from_desc("\u{1}", 1).map(|movie| movie.file),
            Some("\u{1}".to_owned())
        );
    }

    #[test]
    fn the_loader_rebases_a_low_weapon_or_government_mod_val_to_an_id() {
        for (mod_type, raw, loaded) in [
            (1, 0, 128),
            (1, 5, 133),
            (1, 127, 255),
            (3, 5, 133),
            (21, 0, 128),
            (21, 127, 255),
            (1, 128, 128),
            (3, 128, 128),
            (21, 128, 128),
            (1, 383, 383),
            (1, 400, 400),
            (1, -1, -1),
            (3, -1, -1),
            (21, -1, -1),
            (16, 5, 5),
            (2, 5, 5),
            (0, 0, 0),
            (4, 127, 127),
            (20, 0, 0),
            (22, 0, 0),
        ] {
            assert_eq!(
                loaded_mod_val(mod_type, raw),
                loaded,
                "ModType {mod_type}, ModVal {raw}"
            );
        }
    }

    /// Ship 128 is average; system 130 alone exists.
    #[test]
    fn a_catalog_that_does_not_say_counts_graphic_type_0s_frames() {
        assert!(One.fallback_counts_frames(), "every stock spïn counts");
    }

    struct One;

    impl PilotCatalog for One {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            Ok(CharacterStart {
                ship: Some(ShipId(128)),
                systems: [Some(SystemId(130)), None, None, None],
                start: StartDate {
                    day: 23,
                    month: 6,
                    year: 1177,
                },
                ..CharacterStart::default()
            })
        }

        fn ship_fields(&self, id: ShipId) -> Result<ShipFields, String> {
            (id == ShipId(128))
                .then_some(ShipFields {
                    speed: 300,
                    accel: 300,
                    maneuver: 10,
                    ..ShipFields::default()
                })
                .ok_or_else(|| format!("no shïp {}", id.0))
        }

        /// Ship 128 carries two of outfit 200.
        fn default_outfits(&self, id: ShipId) -> Vec<(OutfitId, u16)> {
            if id != ShipId(128) {
                return Vec::new();
            }
            vec![(OutfitId(200), 2)]
        }

        /// Outfit 200, a fuel scoop.
        fn outfits(&self) -> Vec<OutfitRecord> {
            vec![OutfitRecord {
                id: OutfitId(200),
                name: "Scoop".to_owned(),
                short_name: "Scoop".to_owned(),
                disp_weight: 0,
                mass: 1,
                tech_level: 1,
                max: 1,
                flags: 0,
                cost: 100,
                mods: [(18, 8), (0, 0), (0, 0), (0, 0)],
                contribute: 0,
                require: 0,
                require_govt: -1,
                buy_random: 100,
                availability: Test::default(),
                item_class: 0,
                lc_name: "scoop".to_owned(),
                lc_plural: "scoops".to_owned(),
                scan_mask: 0,
                on_purchase: crate::control::Script::default(),
                on_sell: crate::control::Script::default(),
            }]
        }

        /// Ship 128, the Shuttle.
        fn ships(&self) -> Vec<ShipRecord> {
            vec![ShipRecord {
                name: "Shuttle".to_owned(),
                comm_name: "shuttle".to_owned(),
                inherent_govt: Some(GovtId(129)),
                ..crate::fixture::ship(128, ShipFields::default())
            }]
        }

        fn system_exists(&self, id: SystemId) -> bool {
            id == SystemId(130)
        }

        /// System 130 holds stellar 128 at (1, 2); no other system has
        /// any.
        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            if system != SystemId(130) {
                return Vec::new();
            }
            vec![LandingSite {
                id: StellarId(128),
                position: Vec2::new(1.0, 2.0),
                frame_size: Some((10, 20)),
                flags: 0x01,
                min_status: 0,
                tech_level: 3,
                special_tech: [0; 8],
                govt: None,
                flags2: 0,
                destroyed: false,
                graphic: 4,
                dead_graphic: Some(5),
                dead_frame_size: Some((60, 60)),
                type_counts_frames: true,
                dead_counts_frames: true,
                exit_angle: -1,
                tribute: 0,
                gravity: 0,
                fee: 0,
                animation: crate::catalog::SiteAnimation::default(),
                defense_dude: None,
                def_count: 0,
            }]
        }

        /// Hypergate 1400, linked to 1405.
        fn gate_sites(&self) -> Vec<GateSite> {
            let mut links = [None; 8];
            links[0] = Some(StellarId(1405));
            vec![GateSite {
                id: StellarId(1400),
                position: Vec2::new(-70.0, 250.0),
                flags2: 0x1200,
                links,
                exit_angle: 120,
            }]
        }

        /// System 130 at (5, -6), linked to 131.
        fn star_map(&self) -> Vec<StarSystem> {
            vec![StarSystem {
                id: SystemId(130),
                position: Vec2::new(5.0, -6.0),
                links: vec![SystemId(131)],
                govt: None,
                stellars: Vec::new(),
                interference: 0,
                murk: 0,
                message: -1,
                background: 0,
            }]
        }

        /// Food at 75.
        fn commodity_strings(&self) -> CommodityStrings {
            CommodityStrings {
                names: vec!["Food".to_owned()],
                name_patches: Default::default(),
                base_prices: vec!["75".to_owned()],
                price_patches: Default::default(),
            }
        }

        fn string_patch(&self, id: i16) -> StringPatch {
            if id == 1000 {
                StringPatch::Text("Patched buoy".into())
            } else {
                StringPatch::Absent
            }
        }

        fn buoy_strings(&self) -> Vec<String> {
            vec!["First buoy".into(), "Second buoy".into()]
        }

        /// Opals, sold at stellar 128.
        fn junk(&self) -> Vec<JunkRecord> {
            vec![JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
                abbrev: String::new(),
                base_price: 1200,
                sold_at: vec![StellarId(128)],
                bought_at: Vec::new(),
                buy_on: Test::default(),
                sell_on: Test::default(),
                flags: 0,
                scan_mask: 0,
                lc_name: String::new(),
            }]
        }

        /// Dates read "Year ... NC".
        fn date_affixes(&self) -> DateAffixes {
            DateAffixes {
                prefix: "Year ".to_owned(),
                suffix: " NC".to_owned(),
            }
        }

        /// A food surplus at stellar 128.
        fn disasters(&self) -> Vec<DisasterRecord> {
            vec![DisasterRecord {
                id: DisasterId(128),
                name: "An enormous food surplus".to_owned(),
                stellar: 128,
                ..DisasterRecord::default()
            }]
        }

        /// Stellar 128, uninhabited.
        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            vec![(StellarId(128), 0x20)]
        }

        /// Stellar 128, armed with 196.
        fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
            vec![(StellarId(128), WeaponId(196))]
        }

        /// Stellar 128, back after 7 days.
        fn stellar_lives(&self) -> Vec<StellarLife> {
            vec![StellarLife {
                id: StellarId(128),
                strength: 500,
                dead_time: 7,
                flags2: 0x0040,
                explosion: None,
                on_destroy: Script::default(),
                on_regen: Script::default(),
            }]
        }

        /// Stellar 128 sets bit 100 as it is dominated and clears it as it
        /// is released.
        fn stellar_scripts(&self) -> Vec<StellarScripts> {
            vec![StellarScripts {
                id: StellarId(128),
                on_dominate: Script::parse("b100"),
                on_release: Script::parse("!b100"),
            }]
        }

        /// `spïn` 1000's frames are 7 by 9.
        fn fallback_frame_size(&self) -> Option<(u32, u32)> {
            Some((7, 9))
        }

        /// Mission 130, offered at the mission computer.
        fn missions(&self) -> Vec<MissionRecord> {
            vec![crate::fixture::mission(130)]
        }

        /// Mission 130's texts: "Run", and nothing more.
        fn mission_texts(&self) -> Vec<MissionTexts> {
            vec![MissionTexts::named(MissionId(130), "Run")]
        }

        /// Stellar 128, listed by system 130.
        fn listed_stellars(&self) -> Vec<ListedStellar> {
            vec![ListedStellar {
                id: StellarId(128),
                systems: vec![SystemId(130)],
                flags: 0x01,
                flags2: 0,
                govt: None,
                min_status: 0,
            }]
        }

        /// System 130 visible by bit 5.
        fn system_visibility(&self) -> Vec<SystemVisibility> {
            vec![SystemVisibility {
                id: SystemId(130),
                visibility: Test::parse("b5"),
            }]
        }

        /// Stellar 128 Earth, in system 130 Sol.
        fn place_names(&self) -> PlaceNames {
            PlaceNames {
                stellars: BTreeMap::from([(StellarId(128), "Earth".to_owned())]),
                systems: BTreeMap::from([(SystemId(130), "Sol".to_owned())]),
            }
        }

        /// Stellar 128 Earth at 30,-40.
        fn stellar_marks(&self) -> Vec<StellarMark> {
            vec![StellarMark {
                id: StellarId(128),
                place: (30, -40),
                name: "Earth".to_owned(),
            }]
        }

        /// Cron 156, running 105 days.
        fn crons(&self) -> Vec<CronRecord> {
            vec![CronRecord {
                duration: 105,
                ..crate::fixture::cron(156)
            }]
        }

        /// Rank 140, paying 200.
        fn ranks(&self) -> Vec<RankRecord> {
            vec![RankRecord {
                salary: 200,
                ..crate::fixture::rank(140)
            }]
        }

        /// Nebula 141, 300 wide.
        fn nebulae(&self) -> Vec<NebulaRecord> {
            vec![crate::fixture::nebula(141, (0, 0, 300, 10), "", "")]
        }

        /// A second at 22050 Hz.
        fn warp_sound(&self) -> Option<SoundLength> {
            Some(SoundLength {
                frames: 22_050,
                rate: 22_050,
            })
        }

        /// System 130's first stellar is 129, unlike its landing sites'
        /// 128, so a forward that falls to the default shows.
        fn first_stellars(&self) -> Vec<(SystemId, StellarId)> {
            vec![(SystemId(130), StellarId(129))]
        }

        /// Half a second at 44100 Hz.
        fn warp_sound_double_time(&self) -> Option<SoundLength> {
            Some(SoundLength {
                frames: 22_050,
                rate: 44_100,
            })
        }

        /// Two seconds at 11025 Hz.
        fn warp_out_sound(&self) -> Option<SoundLength> {
            Some(SoundLength {
                frames: 22_050,
                rate: 11_025,
            })
        }

        fn boom_sounds(&self) -> Vec<(BoomId, SoundId)> {
            vec![(BoomId(130), SoundId(302))]
        }

        /// `gövt` 128 speaks set 3, forced odd, with one acknowledgement.
        fn voices(&self) -> Voices {
            Voices::read(
                &[(
                    SoundId(1300),
                    Some(SoundLength {
                        frames: 22_050,
                        rate: 22_050,
                    }),
                )],
                &[(GovtId(128), 1003)],
            )
        }

        /// Düde 150 flies ship 128.
        fn mission_dude(&self, id: DudeId) -> Option<DudeRecord> {
            (id == DudeId(150)).then(|| DudeRecord {
                ai_type: 2,
                govt: None,
                ships: vec![(ShipId(128), 40)],
                booty: 0,
                info_types: 0,
            })
        }
    }

    /// Everything `catalog` says about ships 128 and 129 and their
    /// outfits, systems 130 and 131, the star map, the goods and the
    /// outfits.
    fn reads(catalog: impl PilotCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.first_character()),
            format!("{:?}", catalog.ship_fields(ShipId(128))),
            format!("{:?}", catalog.ship_fields(ShipId(129))),
            format!("{}", catalog.system_exists(SystemId(130))),
            format!("{}", catalog.system_exists(SystemId(131))),
            format!("{:?}", catalog.landing_sites(SystemId(130))),
            format!("{:?}", catalog.landing_sites(SystemId(131))),
            format!("{:?}", catalog.star_map()),
            format!("{:?}", catalog.default_outfits(ShipId(128))),
            format!("{:?}", catalog.default_outfits(ShipId(129))),
            format!("{:?}", catalog.commodity_strings()),
            format!("{:?}", catalog.junk()),
            format!("{:?}", catalog.disasters()),
            format!("{:?}", catalog.outfits()),
            format!("{:?}", catalog.ships()),
            format!("{:?}", catalog.date_affixes()),
            format!("{:?}", catalog.gate_sites()),
            format!("{:?}", catalog.stellar_flags()),
            format!("{:?}", catalog.stellar_lives()),
            format!("{:?}", catalog.fallback_frame_size()),
            format!("{:?}", catalog.missions()),
            format!("{:?}", catalog.mission_texts()),
            format!("{:?}", catalog.listed_stellars()),
            format!("{:?}", catalog.place_names()),
            format!("{:?}", catalog.stellar_marks()),
            format!("{:?}", catalog.crons()),
            format!("{:?}", catalog.ranks()),
            format!("{:?}", catalog.system_visibility()),
            format!("{:?}", catalog.nebulae()),
            format!("{:?}", catalog.string_patch(1000)),
            format!("{:?}", catalog.buoy_strings()),
            format!("{:?}", catalog.warp_sound()),
            format!("{:?}", catalog.warp_sound_double_time()),
            format!("{:?}", catalog.warp_out_sound()),
            format!("{:?}", catalog.mission_dude(DudeId(150))),
            format!("{:?}", catalog.first_stellars()),
            format!("{:?}", catalog.boom_sounds()),
            format!("{:?}", catalog.voices()),
            format!("{:?}", catalog.stellar_weapons()),
            format!("{:?}", catalog.stellar_scripts()),
        ]
    }

    /// A star map of 130, which holds stellars 128 and 129, 131, which
    /// exists with none, and 132, which doesn't exist; nothing else, and
    /// [`PilotCatalog::first_stellars`] left to its default.
    struct Mapped;

    impl PilotCatalog for Mapped {
        fn first_character(&self) -> Result<CharacterStart, StartError> {
            Err(StartError::NoCharacter)
        }

        fn ship_fields(&self, _id: ShipId) -> Result<ShipFields, String> {
            Err(String::new())
        }

        fn default_outfits(&self, _id: ShipId) -> Vec<(OutfitId, u16)> {
            Vec::new()
        }

        fn outfits(&self) -> Vec<OutfitRecord> {
            Vec::new()
        }

        fn ships(&self) -> Vec<ShipRecord> {
            Vec::new()
        }

        fn system_exists(&self, id: SystemId) -> bool {
            matches!(id.0, 130 | 131)
        }

        /// Asked only of a system that exists.
        fn landing_sites(&self, system: SystemId) -> Vec<LandingSite> {
            match system.0 {
                130 => vec![
                    crate::fixture::planet(128, 0.0, 0.0),
                    crate::fixture::planet(129, 9.0, 0.0),
                ],
                131 => Vec::new(),
                other => panic!("asked for sÿst {other}'s sites"),
            }
        }

        fn star_map(&self) -> Vec<StarSystem> {
            [130, 131, 132]
                .into_iter()
                .map(|id| crate::fixture::star(id, (0.0, 0.0), &[]))
                .collect()
        }

        fn commodity_strings(&self) -> CommodityStrings {
            CommodityStrings::default()
        }

        fn string_patch(&self, _id: i16) -> StringPatch {
            StringPatch::Absent
        }

        fn buoy_strings(&self) -> Vec<String> {
            Vec::new()
        }

        fn junk(&self) -> Vec<JunkRecord> {
            Vec::new()
        }

        fn disasters(&self) -> Vec<DisasterRecord> {
            Vec::new()
        }

        fn stellar_flags(&self) -> Vec<(StellarId, u32)> {
            Vec::new()
        }

        fn stellar_weapons(&self) -> Vec<(StellarId, WeaponId)> {
            Vec::new()
        }

        fn date_affixes(&self) -> DateAffixes {
            DateAffixes::default()
        }

        fn gate_sites(&self) -> Vec<GateSite> {
            Vec::new()
        }

        fn stellar_lives(&self) -> Vec<StellarLife> {
            Vec::new()
        }

        fn stellar_scripts(&self) -> Vec<StellarScripts> {
            Vec::new()
        }

        fn fallback_frame_size(&self) -> Option<(u32, u32)> {
            None
        }

        fn missions(&self) -> Vec<MissionRecord> {
            Vec::new()
        }

        fn mission_texts(&self) -> Vec<MissionTexts> {
            Vec::new()
        }

        fn listed_stellars(&self) -> Vec<ListedStellar> {
            Vec::new()
        }

        fn system_visibility(&self) -> Vec<SystemVisibility> {
            Vec::new()
        }

        fn place_names(&self) -> PlaceNames {
            PlaceNames::default()
        }

        fn stellar_marks(&self) -> Vec<StellarMark> {
            Vec::new()
        }

        fn crons(&self) -> Vec<CronRecord> {
            Vec::new()
        }

        fn ranks(&self) -> Vec<RankRecord> {
            Vec::new()
        }

        fn nebulae(&self) -> Vec<NebulaRecord> {
            Vec::new()
        }
    }

    #[test]
    fn by_default_each_existing_systems_first_stellar_is_its_first_landing_site() {
        assert_eq!(
            Mapped.first_stellars(),
            [(SystemId(130), StellarId(128))],
            "131 has none; 132 doesn't exist and isn't asked"
        );
    }

    #[test]
    fn borrowed_and_shared_catalogs_are_catalogs() {
        let direct = reads(One);
        assert!(direct[1].contains("speed: 300"), "{direct:?}");
        assert_eq!(direct[2], r#"Err("no shïp 129")"#);
        assert_eq!(direct[3..5], ["true", "false"]);
        assert!(direct[5].contains("StellarId(128)"), "{direct:?}");
        assert_eq!(direct[6], "[]");
        assert!(direct[0].contains("year: 1177"), "{direct:?}");
        assert!(direct[7].contains("SystemId(131)"), "{direct:?}");
        assert_eq!(direct[8], "[(OutfitId(200), 2)]");
        assert_eq!(direct[9], "[]");
        assert!(direct[10].contains("\"75\""), "{direct:?}");
        assert!(direct[11].contains("Opals"), "{direct:?}");
        assert!(direct[12].contains("food surplus"), "{direct:?}");
        assert!(direct[13].contains("Scoop"), "{direct:?}");
        assert!(direct[14].contains("Shuttle"), "{direct:?}");
        assert!(direct[14].contains("\"shuttle\""), "{direct:?}");
        assert!(direct[14].contains("Some(GovtId(129))"), "{direct:?}");
        assert_eq!(
            direct[15],
            r#"DateAffixes { prefix: "Year ", suffix: " NC" }"#
        );
        assert!(direct[16].contains("StellarId(1405)"), "{direct:?}");
        assert_eq!(direct[17], "[(StellarId(128), 32)]");
        assert!(direct[18].contains("dead_time: 7"), "{direct:?}");
        assert!(
            direct[5].contains("graphic: 4, dead_graphic: Some(5)"),
            "{direct:?}"
        );
        assert_eq!(direct[19], "Some((7, 9))", "forwarded, not a default");
        assert!(direct[20].contains("MissionId(130)"), "{direct:?}");
        assert!(direct[21].contains("\"Run\""), "{direct:?}");
        assert!(
            direct[22].contains("systems: [SystemId(130)]"),
            "{direct:?}"
        );
        assert!(direct[23].contains("\"Earth\""), "{direct:?}");
        assert!(direct[23].contains("\"Sol\""), "{direct:?}");
        assert!(direct[24].contains("place: (30, -40)"), "{direct:?}");
        assert!(direct[25].contains("CronId(156)"), "{direct:?}");
        assert!(direct[25].contains("duration: 105"), "{direct:?}");
        assert!(direct[26].contains("RankId(140)"), "{direct:?}");
        assert!(direct[26].contains("salary: 200"), "{direct:?}");
        assert!(direct[27].contains("SystemId(130)"), "{direct:?}");
        assert!(direct[27].contains("Bit(5)"), "{direct:?}");
        assert!(direct[28].contains("NebulaId(141)"), "{direct:?}");
        assert!(direct[28].contains("width: 300"), "{direct:?}");
        assert_eq!(direct[29], r#"Text("Patched buoy")"#);
        assert_eq!(direct[30], r#"["First buoy", "Second buoy"]"#);
        assert_eq!(
            direct[31], "Some(SoundLength { frames: 22050, rate: 22050 })",
            "forwarded, not a default"
        );
        assert_eq!(
            direct[32], "Some(SoundLength { frames: 22050, rate: 44100 })",
            "forwarded, not a default"
        );
        assert_eq!(
            direct[33], "Some(SoundLength { frames: 22050, rate: 11025 })",
            "forwarded, not a default"
        );
        assert!(direct[34].contains("ShipId(128), 40"), "{direct:?}");
        assert_eq!(
            direct[35], "[(SystemId(130), StellarId(129))]",
            "forwarded, not the default's 128"
        );
        assert_eq!(
            direct[36], "[(BoomId(130), SoundId(302))]",
            "forwarded, not a default"
        );
        assert!(
            direct[37].contains("GovtId(128): GovtVoice { set: Some(3)"),
            "forwarded, not a default: {direct:?}"
        );
        assert_eq!(direct[38], "[(StellarId(128), WeaponId(196))]");
        assert!(
            direct[39].contains("StellarScripts { id: StellarId(128)"),
            "forwarded, not a default: {direct:?}"
        );
        assert_eq!(reads(&One), direct);
        assert_eq!(reads(Rc::new(One)), direct);
    }

    /// System 130 has düde 128 at 60 % and flët 129 at 20 %, five ships
    /// on average, and flët 129 for its reinforcements; düde 128 flies
    /// ship 128; flët 129 is led by ship 128.
    impl TrafficCatalog for One {
        fn system_reinforcements(&self, id: SystemId) -> Option<SystemReinforcements> {
            (id == SystemId(130)).then_some(SystemReinforcements {
                fleet: FleetId(129),
                time: 900,
                interval: 2,
            })
        }

        fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
            (id == SystemId(130)).then_some(SystemTraffic {
                dude_types: [
                    (128, 60),
                    (-129, 20),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                    (-1, 0),
                ],
                avg_ships: 5,
                persons: [
                    (Some(PersonId(510)), 50),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                    (None, 0),
                ],
            })
        }

        fn dude(&self, id: DudeId) -> Option<DudeRecord> {
            (id == DudeId(128)).then(|| DudeRecord {
                ai_type: 1,
                govt: Some(GovtId(128)),
                ships: vec![(ShipId(128), 100)],
                booty: 0,
                info_types: 0x4005,
            })
        }

        fn fleets(&self) -> Vec<FleetRecord> {
            vec![FleetRecord {
                id: FleetId(129),
                lead: Some(ShipId(128)),
                escorts: vec![EscortRecord {
                    ship: ShipId(128),
                    min: 1,
                    max: 2,
                }],
                govt: None,
                link_syst: -1,
                appear_on: Test::default(),
                quote: None,
                flags: 0,
            }]
        }

        fn persons(&self) -> Vec<PersonRecord> {
            vec![PersonRecord {
                name: "Ace".to_owned(),
                ..crate::fixture::person(510, 128)
            }]
        }
    }

    /// Everything `catalog` says about system 130's and 131's traffic,
    /// düdes 128 and 129, the fleets and the persons, and system 130's
    /// reinforcements.
    fn traffic(catalog: impl TrafficCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.system_traffic(SystemId(130))),
            format!("{:?}", catalog.system_traffic(SystemId(131))),
            format!("{:?}", catalog.dude(DudeId(128))),
            format!("{:?}", catalog.dude(DudeId(129))),
            format!("{:?}", catalog.fleets()),
            format!("{:?}", catalog.persons()),
            format!("{:?}", catalog.system_reinforcements(SystemId(130))),
        ]
    }

    #[test]
    fn a_traffic_catalog_keeps_no_reinforcements_unless_it_says_so() {
        struct Bare;
        impl TrafficCatalog for Bare {
            fn system_traffic(&self, _id: SystemId) -> Option<SystemTraffic> {
                None
            }
            fn dude(&self, _id: DudeId) -> Option<DudeRecord> {
                None
            }
            fn fleets(&self) -> Vec<FleetRecord> {
                Vec::new()
            }
            fn persons(&self) -> Vec<PersonRecord> {
                Vec::new()
            }
        }
        assert_eq!(Bare.system_reinforcements(SystemId(130)), None);
    }

    #[test]
    fn borrowed_and_shared_traffic_catalogs_are_catalogs() {
        let direct = traffic(One);
        assert!(direct[6].contains("FleetId(129)"), "{direct:?}");
        assert!(direct[0].contains("avg_ships: 5"), "{direct:?}");
        assert_eq!(direct[1], "None");
        assert!(direct[2].contains("ShipId(128), 100"), "{direct:?}");
        assert!(direct[2].contains("info_types: 16389"), "{direct:?}");
        assert_eq!(direct[3], "None");
        assert!(direct[4].contains("FleetId(129)"), "{direct:?}");
        assert!(direct[0].contains("PersonId(510)), 50"), "{direct:?}");
        assert!(direct[5].contains("\"Ace\""), "{direct:?}");
        assert_eq!(traffic(&One), direct);
        assert_eq!(traffic(Rc::new(One)), direct);
    }

    /// Weapon 128 is a blaster; ship 128 carries two of it and has a
    /// 24-pixel `shän`.
    impl CombatCatalog for One {
        fn weapons(&self) -> Vec<WeaponRecord> {
            vec![WeaponRecord {
                reload: 10,
                count: 13,
                speed: 1500,
                ..crate::fixture::weapon(128)
            }]
        }

        fn hulls(&self) -> Vec<HullRecord> {
            vec![HullRecord {
                weapons: vec![StockWeapon {
                    weapon: WeaponId(128),
                    count: 2,
                    ammo: 0,
                }],
                size: Some(24),
                strength: 250,
                ..crate::fixture::hull(128)
            }]
        }

        fn governments(&self) -> Vec<GovtRecord> {
            vec![GovtRecord {
                crime_tol: 6,
                comm_name: "Federation".to_owned(),
                ..crate::fixture::govt(128)
            }]
        }
    }

    /// Everything `catalog` says about weapons, hulls and governments.
    fn combat(catalog: impl CombatCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.weapons()),
            format!("{:?}", catalog.hulls()),
            format!("{:?}", catalog.governments()),
        ]
    }

    #[test]
    fn borrowed_and_shared_combat_catalogs_are_catalogs() {
        let direct = combat(One);
        assert!(direct[0].contains("speed: 1500"), "{direct:?}");
        assert!(direct[1].contains("size: Some(24)"), "{direct:?}");
        assert!(direct[1].contains("WeaponId(128), count: 2"), "{direct:?}");
        assert!(direct[1].contains("strength: 250"), "{direct:?}");
        assert!(direct[2].contains("crime_tol: 6"), "{direct:?}");
        assert!(direct[2].contains("\"Federation\""), "{direct:?}");
        assert_eq!(combat(&One), direct);
        assert_eq!(combat(Rc::new(One)), direct);
    }

    /// Asteroid type 4, `röid` 132, and system 130's field.
    impl AsteroidCatalog for One {
        fn asteroid_types(&self) -> Vec<AsteroidTypeRecord> {
            vec![AsteroidTypeRecord {
                kind: 4,
                strength: 30,
                spin_rate: 120,
                frames: 36,
                frame_size: Some((48, 40)),
                ..AsteroidTypeRecord::default()
            }]
        }

        fn asteroid_fields(&self) -> Vec<AsteroidFieldRecord> {
            vec![AsteroidFieldRecord {
                system: SystemId(130),
                count: 6,
                types: 0x0011,
            }]
        }

        fn box_frame_sizes(&self) -> [Option<(u32, u32)>; BOX_GRAPHICS] {
            [None, None, Some((22, 20)), None, None]
        }
    }

    type Asteroids = (
        Vec<AsteroidTypeRecord>,
        Vec<AsteroidFieldRecord>,
        [Option<(u32, u32)>; BOX_GRAPHICS],
    );

    fn asteroids(catalog: impl AsteroidCatalog) -> Asteroids {
        (
            catalog.asteroid_types(),
            catalog.asteroid_fields(),
            catalog.box_frame_sizes(),
        )
    }

    #[test]
    fn borrowed_and_shared_asteroid_catalogs_are_catalogs() {
        let direct = asteroids(One);
        assert_eq!(direct.0[0].kind, 4);
        assert_eq!(direct.1[0].types, 0x0011);
        assert_eq!(direct.2[2], Some((22, 20)));
        assert_eq!(asteroids(&One), direct);
        assert_eq!(asteroids(Rc::new(One)), direct);
    }

    /// `STR#` 3000 holds two replies; no other list exists. A plug-in
    /// patches `STR ` 5000 alone.
    impl CommCatalog for One {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 3000 {
                vec!["Channel open.".to_owned(), "Hello.".to_owned()]
            } else {
                Vec::new()
            }
        }

        fn plugin_string(&self, id: i16) -> StringPatch {
            if id == 5000 {
                StringPatch::Text("Patched.".into())
            } else {
                StringPatch::Absent
            }
        }
    }

    /// `STR#` 3000 and 3001, and the plug-in's `STR ` 5000, as `catalog`
    /// gives them.
    fn comm(catalog: impl CommCatalog) -> (Vec<Vec<String>>, StringPatch) {
        (
            vec![catalog.string_list(3000), catalog.string_list(3001)],
            catalog.plugin_string(5000),
        )
    }

    #[test]
    fn borrowed_and_shared_comm_catalogs_are_catalogs() {
        // `plugin_string` defaults to `Absent`, so a forwarding impl that
        // dropped it would still compile: the patch shows it forwards.
        let direct = comm(One);
        assert_eq!(direct.0[0], ["Channel open.", "Hello."]);
        assert!(direct.0[1].is_empty(), "{direct:?}");
        assert_eq!(direct.1, StringPatch::Text("Patched.".into()));
        assert_eq!(comm(&One), direct);
        assert_eq!(comm(Rc::new(One)), direct);
    }

    /// Data holding an `l33t` resource.
    struct Leet;

    impl CommCatalog for Leet {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }

        fn leet_mode(&self) -> bool {
            true
        }
    }

    /// Whether `catalog` turns on leet mode.
    fn leet(catalog: impl CommCatalog) -> bool {
        catalog.leet_mode()
    }

    #[test]
    fn a_comm_catalog_is_not_leet_unless_it_says_so_through_any_handle() {
        assert!(!leet(One), "none by default");
        assert!(!leet(&One));
        assert!(!leet(Rc::new(One)));
        assert!(leet(Leet));
        assert!(leet(&Leet));
        assert!(leet(Rc::new(Leet)));
    }

    #[test]
    fn by_default_no_missions_dude_can_be_read() {
        assert_eq!(Mapped.mission_dude(DudeId(150)), None);
    }

    #[test]
    fn an_explosion_type_of_0_to_63_is_its_boom_and_1000_more_adds_the_extras() {
        let boom = |id, extra| {
            Some(Explosion {
                boom: BoomId(id),
                extra,
            })
        };
        assert_eq!(Explosion::decode(0), boom(128, false));
        assert_eq!(Explosion::decode(5), boom(133, false));
        assert_eq!(Explosion::decode(63), boom(191, false));
        assert_eq!(Explosion::decode(1000), boom(128, true));
        assert_eq!(Explosion::decode(1005), boom(133, true));
        assert_eq!(Explosion::decode(1063), boom(191, true));
        for none in [-1, 64, 999, 1064, i16::MIN, i16::MAX] {
            assert_eq!(Explosion::decode(none), None, "{none}");
        }
        assert_eq!((FIRST_BOOM, BOOM_TYPES, EXTRA_EXPLOSIONS), (128, 64, 1000));
    }

    #[test]
    fn an_explosion_sounds_snd_300_on_for_a_sound_index_of_0_to_63_only() {
        assert_eq!(explosion_sound(0), Some(SoundId(300)));
        assert_eq!(explosion_sound(5), Some(SoundId(305)));
        assert_eq!(explosion_sound(63), Some(SoundId(363)));
        for none in [-1, 64, i16::MIN, i16::MAX] {
            assert_eq!(explosion_sound(none), None, "{none}");
        }
        assert_eq!(Mapped.boom_sounds(), [], "none by default");
    }

    #[test]
    fn each_escort_class_has_its_menu_label_and_message_form() {
        assert_eq!(
            EscortClass::ALL.map(EscortClass::menu_label),
            ["Fighters", "Medium Ships", "Warships", "Freighters"]
        );
        assert_eq!(
            EscortClass::ALL.map(EscortClass::message_form),
            ["Fighters", "Medium ships", "Warships", "Freighters"]
        );
    }

    #[test]
    fn no_special_ships_have_no_goal_and_no_behaviour() {
        let none = MissionShip::NONE;
        assert_eq!((none.count, none.goal, none.behav), (0, -1, -1));
        assert_eq!((none.syst, none.start), (0, 0));
        assert_eq!((none.dude, none.name_id, none.subtitle), (None, None, None));
    }

    #[test]
    fn each_error_reads_as_a_sentence() {
        assert_eq!(StartError::NoCharacter.to_string(), "no chär to start from");
        assert_eq!(
            StartError::Character("chär 128: too short".to_owned()).to_string(),
            "chär 128: too short"
        );
        assert_eq!(StartError::NoShip.to_string(), "the first chär has no ship");
        assert_eq!(
            StartError::Ship(ShipId(128), "no shïp 128".to_owned()).to_string(),
            "no shïp 128"
        );
        assert_eq!(
            StartError::NoStartingSystem([None, Some(SystemId(999)), None, Some(SystemId(5))])
                .to_string(),
            "none of the first chär's starting systems (none, 999, none, 5) exists"
        );
        assert_eq!(
            StartError::NoSystem(SystemId(130)).to_string(),
            "the pilot's system, sÿst 130, does not exist"
        );
    }
}
