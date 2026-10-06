//! The catalog ports: what a flight session starts from
//! ([`PilotCatalog`]), what its NPC traffic is spawned from
//! ([`TrafficCatalog`]), what its ships fight with ([`CombatCatalog`]) and
//! the words a hailed ship answers with ([`CommCatalog`]), in the
//! simulation's own terms.

use std::rc::Rc;

pub use nova_data::{
    BoomId, DudeId, FleetId, GovtId, JunkId, OutfitId, ShipId, SoundId, StellarId, SystemId,
    WeaponId,
};

use crate::geometry::Vec2;
use crate::handling::ShipFields;

/// A new pilot's start, from the first `chär`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharacterStart {
    /// The starting `shïp`, if it names one.
    pub ship: Option<ShipId>,
    /// The starting `sÿst`s, in order; any may be unused.
    pub systems: [Option<SystemId>; 4],
    /// The starting date, raw.
    pub start: StartDate,
    /// The starting credits, raw: the [`pilot`](crate::pilot) rules
    /// decide what a negative amount means.
    pub cash: i32,
    /// The starting legal records, `Govt1-4` with `Status1-4`: each
    /// government and the record with it, or `None` for an unused slot.
    pub legal: [Option<(GovtId, i16)>; 4],
}

/// A new pilot's starting date, raw from the `chär`: the
/// [`date`](crate::date) rules decide what the values mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartDate {
    /// The day of the month.
    pub day: i16,
    /// The month, 1 (January) to 12.
    pub month: i16,
    /// The year.
    pub year: i16,
}

/// A star system on the map, raw from its `sÿst`: the
/// [`hyperspace`](crate::hyperspace) rules decide which links count.
#[derive(Clone, Debug, PartialEq)]
pub struct StarSystem {
    /// The `sÿst`'s ID.
    pub id: SystemId,
    /// Its map position, `xPos` and `yPos`.
    pub position: Vec2,
    /// Its hyperlinks, `Con1-Con16`, in record order: they may repeat,
    /// point at itself or at a system that does not exist.
    pub links: Vec<SystemId>,
    /// Its controlling government, `Govt`, or `None` when it is
    /// independent (-1).
    pub govt: Option<GovtId>,
}

/// A stellar the player might land on, raw from its `spöb`; the
/// [`landing`](crate::landing) rules decide what the values mean.
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
    /// The sound it plays when the player lands, from its `CustSndID`:
    /// stellar landing sounds are `snd ` 10000 and up, and any other value
    /// (-1 for none, 0, or the angle hypergates and wormholes keep there)
    /// is none.
    pub landing_sound: Option<SoundId>,
    /// Its `TechLevel`: the [`outfitter`](crate::outfitter) sells outfits
    /// up to it.
    pub tech_level: i16,
    /// Its `SpecialTech` 1-8, in order: the outfitter also sells outfits of
    /// exactly these tech levels.
    pub special_tech: [i16; 8],
    /// Its `Govt`, or `None` when it is independent.
    pub govt: Option<GovtId>,
}

/// An outfit, raw from its `oütf`: the [`outfitter`](crate::outfitter)
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
    /// Its `ModType` and `ModVal` pairs 1-4, in order.
    pub mods: [(i16, i16); 4],
    /// Its `Contribute` bits.
    pub contribute: u64,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `RequireGovt`, raw.
    pub require_govt: i16,
    /// Its `Availability` control-bit expression.
    pub availability: String,
}

/// A ship class, raw from its `shïp`: the [`shipyard`](crate::shipyard)
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
    /// Its `Cost`.
    pub cost: i32,
    /// Its `TechLevel`.
    pub tech_level: i16,
    /// Its `BuyRandom`: the percent chance a day it is for sale.
    pub buy_random: i16,
    /// Its `HireRandom`: the percent chance a day it is for hire in the
    /// bar (see [`hire`](crate::hire)).
    pub hire_random: i16,
    /// Its `Require` bits.
    pub require: u64,
    /// Its `Availability` control-bit expression.
    pub availability: String,
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
    /// Its `EscortType`, raw: the class it has as the player's escort
    /// (see [`EscortClass::of`](crate::escort::EscortClass::of)).
    pub escort_type: i16,
}

/// The standard commodities, raw from their string lists: `STR#` 4000
/// "All Cargo" names them and `STR#` 4004 "Base Prices" prices them, the
/// nth string for commodity n (from 0); the [`market`](crate::market)
/// rules decide which are traded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommodityStrings {
    /// Every string of `STR#` 4000, in order; none when it is missing.
    pub names: Vec<String>,
    /// Every string of `STR#` 4004, in order; none when it is missing.
    pub base_prices: Vec<String>,
}

/// A special commodity, raw from its `jünk`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JunkRecord {
    /// The `jünk`'s ID.
    pub id: JunkId,
    /// Its name: the resource's name, or its `LCName` when the resource
    /// has none.
    pub name: String,
    /// Its `BasePrice`.
    pub base_price: i16,
    /// Its `SoldAt` stellars, the unused (-1) slots left out.
    pub sold_at: Vec<StellarId>,
    /// Its `BoughtAt` stellars, the unused (-1) slots left out.
    pub bought_at: Vec<StellarId>,
    /// Its `BuyOn` control-bit expression.
    pub buy_on: String,
    /// Its `SellOn` control-bit expression.
    pub sell_on: String,
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
    /// Its resource name, which the commodity exchange shows while it is
    /// active.
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
    /// Its `ActivateOn` control-bit expression.
    pub activate_on: String,
}

/// A system's traffic, raw from its `sÿst`: the
/// [`traffic`](crate::traffic) rules decide what the values mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemTraffic {
    /// Its `DudeTypes`, each with its `% Prob`, in record order: a `düde`
    /// ID, a negated `flët` ID, or an unused slot.
    pub dude_types: [(i16, i16); 8],
    /// Its `AvgShips`.
    pub avg_ships: i16,
}

/// A `düde`, raw: the [`traffic`](crate::traffic) rules decide what the
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
    /// [`board`](crate::board)).
    pub booty: u16,
    /// Its `InfoTypes` flags: what its ships say when hailed (see
    /// [`hail`](crate::hail)).
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

/// A fleet, raw from its `flët`: the [`traffic`](crate::traffic) rules
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
    /// Its `AppearOn` control-bit expression.
    pub appear_on: String,
}

/// The game data a system's NPC traffic is spawned from: its `sÿst`'s
/// traffic, the `düde`s it names and the `flët`s.
pub trait TrafficCatalog {
    /// System `id`'s traffic, or `None` when it cannot be read.
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic>;
    /// `düde` `id`, or `None` when it cannot be read.
    fn dude(&self, id: DudeId) -> Option<DudeRecord>;
    /// Every `flët` that can be read, by ascending ID.
    fn fleets(&self) -> Vec<FleetRecord>;
}

/// A borrowed catalog is a catalog.
impl<T: TrafficCatalog + ?Sized> TrafficCatalog for &T {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        (**self).system_traffic(id)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).dude(id)
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        (**self).fleets()
    }
}

/// A shared catalog is a catalog.
impl<T: TrafficCatalog + ?Sized> TrafficCatalog for Rc<T> {
    fn system_traffic(&self, id: SystemId) -> Option<SystemTraffic> {
        (**self).system_traffic(id)
    }

    fn dude(&self, id: DudeId) -> Option<DudeRecord> {
        (**self).dude(id)
    }

    fn fleets(&self) -> Vec<FleetRecord> {
        (**self).fleets()
    }
}

/// A weapon, raw from its `wëap`: the [`combat`](crate::combat) rules
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

/// What a ship class fights with and how it dies, raw from its `shïp` and
/// its `shän`: the [`combat`](crate::combat) rules decide what the values
/// mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HullRecord {
    /// The `shïp`'s ID.
    pub id: ShipId,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `DeathDelay`, in ticks.
    pub death_delay: i16,
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
    /// Its `Strength`: how strong it counts in a fight.
    pub strength: i16,
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

/// A government, raw from its `gövt`: the [`govt`](crate::govt) and
/// [`legal`](crate::legal) rules decide what the values mean.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovtRecord {
    /// The `gövt`'s ID.
    pub id: GovtId,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `CrimeTol`.
    pub crime_tol: i16,
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

/// The game data a hailed ship's words come from: string lists, read
/// whole.
pub trait CommCatalog {
    /// Every string of `STR#` `id`, in order; none when it is missing or
    /// cannot be read.
    fn string_list(&self, id: i16) -> Vec<String>;
}

/// A borrowed catalog is a catalog.
impl<T: CommCatalog + ?Sized> CommCatalog for &T {
    fn string_list(&self, id: i16) -> Vec<String> {
        (**self).string_list(id)
    }
}

/// A shared catalog is a catalog.
impl<T: CommCatalog + ?Sized> CommCatalog for Rc<T> {
    fn string_list(&self, id: i16) -> Vec<String> {
        (**self).string_list(id)
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
    /// Every `jünk` that can be read, by ascending ID.
    fn junk(&self) -> Vec<JunkRecord>;
    /// Every `öops` that can be read, by ascending ID.
    fn disasters(&self) -> Vec<DisasterRecord>;
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

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
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

    fn junk(&self) -> Vec<JunkRecord> {
        (**self).junk()
    }

    fn disasters(&self) -> Vec<DisasterRecord> {
        (**self).disasters()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ship 128 is average; system 130 alone exists.
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
                availability: String::new(),
            }]
        }

        /// Ship 128, the Shuttle.
        fn ships(&self) -> Vec<ShipRecord> {
            vec![ShipRecord {
                name: "Shuttle".to_owned(),
                comm_name: "shuttle".to_owned(),
                inherent_govt: Some(GovtId(129)),
                ..crate::testkit::ship(128, ShipFields::default())
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
                landing_sound: None,
                tech_level: 3,
                special_tech: [0; 8],
                govt: None,
            }]
        }

        /// System 130 at (5, -6), linked to 131.
        fn star_map(&self) -> Vec<StarSystem> {
            vec![StarSystem {
                id: SystemId(130),
                position: Vec2::new(5.0, -6.0),
                links: vec![SystemId(131)],
                govt: None,
            }]
        }

        /// Food at 75.
        fn commodity_strings(&self) -> CommodityStrings {
            CommodityStrings {
                names: vec!["Food".to_owned()],
                base_prices: vec!["75".to_owned()],
            }
        }

        /// Opals, sold at stellar 128.
        fn junk(&self) -> Vec<JunkRecord> {
            vec![JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
                base_price: 1200,
                sold_at: vec![StellarId(128)],
                bought_at: Vec::new(),
                buy_on: String::new(),
                sell_on: String::new(),
            }]
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
        ]
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
        assert_eq!(reads(&One), direct);
        assert_eq!(reads(Rc::new(One)), direct);
    }

    /// System 130 has düde 128 at 60 % and flët 129 at 20 %, five ships
    /// on average; düde 128 flies ship 128; flët 129 is led by ship 128.
    impl TrafficCatalog for One {
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
                appear_on: String::new(),
            }]
        }
    }

    /// Everything `catalog` says about system 130's and 131's traffic,
    /// düdes 128 and 129 and the fleets.
    fn traffic(catalog: impl TrafficCatalog) -> Vec<String> {
        vec![
            format!("{:?}", catalog.system_traffic(SystemId(130))),
            format!("{:?}", catalog.system_traffic(SystemId(131))),
            format!("{:?}", catalog.dude(DudeId(128))),
            format!("{:?}", catalog.dude(DudeId(129))),
            format!("{:?}", catalog.fleets()),
        ]
    }

    #[test]
    fn borrowed_and_shared_traffic_catalogs_are_catalogs() {
        let direct = traffic(One);
        assert!(direct[0].contains("avg_ships: 5"), "{direct:?}");
        assert_eq!(direct[1], "None");
        assert!(direct[2].contains("ShipId(128), 100"), "{direct:?}");
        assert!(direct[2].contains("info_types: 16389"), "{direct:?}");
        assert_eq!(direct[3], "None");
        assert!(direct[4].contains("FleetId(129)"), "{direct:?}");
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
                ..crate::testkit::weapon(128)
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
                ..crate::testkit::hull(128)
            }]
        }

        fn governments(&self) -> Vec<GovtRecord> {
            vec![GovtRecord {
                crime_tol: 6,
                comm_name: "Federation".to_owned(),
                ..crate::testkit::govt(128)
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

    /// `STR#` 3000 holds two replies; no other list exists.
    impl CommCatalog for One {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == 3000 {
                vec!["Channel open.".to_owned(), "Hello.".to_owned()]
            } else {
                Vec::new()
            }
        }
    }

    /// `STR#` 3000 and 3001, as `catalog` gives them.
    fn comm(catalog: impl CommCatalog) -> Vec<Vec<String>> {
        vec![catalog.string_list(3000), catalog.string_list(3001)]
    }

    #[test]
    fn borrowed_and_shared_comm_catalogs_are_catalogs() {
        let direct = comm(One);
        assert_eq!(direct[0], ["Channel open.", "Hello."]);
        assert!(direct[1].is_empty(), "{direct:?}");
        assert_eq!(comm(&One), direct);
        assert_eq!(comm(Rc::new(One)), direct);
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
