//! The catalog layer of EV Nova's simulation: the sim's typed view of the
//! game data, which every layer above reads, and the small modules that
//! view needs. It builds only on the foundation, `nova-sim-base`.
//! `nova-sim` re-exports each module at its old path (`nova_sim::catalog`,
//! `nova_sim::PilotCatalog`, …), so the rest of the workspace reaches
//! them through it.
//!
//! - [`buoy`]: a system's message buoy, the text a hyperspace arrival
//!   shows in place of the arrival line.
//! - [`catalog`]: the catalog ports, in the simulation's own terms: the
//!   [`PilotCatalog`] port, what a flight session starts from; the
//!   [`TrafficCatalog`] port, what NPC traffic is spawned from; the
//!   [`CombatCatalog`] port, what ships fight with; the
//!   [`AsteroidCatalog`] port, what asteroid fields hold; and the
//!   [`CommCatalog`] port, the game's string lists.
//! - [`control`]: the control bits missions and storylines script with,
//!   held as a [`ControlBitSet`] indexed by [`Bit`]; a record's parsed
//!   control-bit [`Test`], and evaluating one against the [`PilotFacts`]
//!   it reads ([`control::holds`]), through the [`ControlBits`] port with
//!   Nova's [`NovaBits`]; and running a set expression
//!   ([`control::execute`]).
//! - [`data`]: the catalog ports' adapters over `nova_data`'s `GameData`.
//! - `fixture` (with the `fixture` feature, and in this crate's tests):
//!   builders for the catalog's records and mocks of the [`ControlBits`]
//!   port, for the tests of this crate and the nova-sim crates above.
//! - [`sound`]: the [`SimSound`] events a session emits as it thrusts,
//!   lands, takes off and jumps; the audio side decides what they play.
//! - [`strings`]: the string lists built once at start-up, each entry
//!   from a plug-in's `'STR '` patch when one exists, else from its
//!   `STR#`.
//! - [`voice`]: the escorts' voices: the clips of each voice set and
//!   message, each government's voice, and the one voice channel the
//!   session plays them through ([`voice::VoiceChannel`]).
//! - [`warp`]: the player's warp clock: the warp factor, the warp sound's
//!   duration, and when the warp's lead-in ends and scans are held.
//!
//! A doc here links only to items of this crate and of `nova-sim-base`.
//! An item of a crate above it is named by its facade path as inline
//! code, such as `nova_sim::Session`.

// The foundation modules and items the moved code and its docs name by
// their old `crate::` paths, which now resolve to `nova-sim-base`.
#[allow(unused_imports)]
use nova_sim_base::{Chance, RuleKey, blink, chance, clock, geometry, handling, rulebook};

pub mod buoy;
pub mod catalog;
pub mod control;
pub mod data;
#[cfg(any(test, feature = "fixture"))]
pub mod fixture;
#[cfg(test)]
mod pict_sheet_sites;
pub mod sound;
pub mod strings;
pub mod voice;
pub mod warp;

pub use catalog::{
    AsteroidCatalog, AsteroidFieldRecord, AsteroidTypeRecord, BOX_GRAPHICS, BoomId, CharacterStart,
    CombatCatalog, CommCatalog, CommodityStrings, CronId, CronRecord, DateAffixes, DescMovie,
    DisasterId, DisasterRecord, DudeId, DudeRecord, EscortRecord, FleetId, FleetRecord, FrameSize,
    GateSite, GovtId, GovtRecord, HullRecord, JunkId, JunkRecord, LandingSite, ListedStellar,
    MissionId, MissionRecord, MissionShip, MissionTexts, NebulaId, NebulaRecord, OutfitId,
    OutfitRecord, Penalties, PersonId, PersonRecord, PersonWeapon, PilotCatalog, PlaceNames,
    RankId, RankRecord, ShipId, ShipRecord, SoundId, StarSystem, StartDate, StartError, StellarId,
    StellarLife, StockWeapon, StringPatch, SystemId, SystemTraffic, TextGraphics, TextMovies,
    TrafficCatalog, WeaponId, WeaponRecord,
};
pub use control::{
    Bit, BitStore, BitWrite, ControlBitSet, ControlBits, NovaBits, PilotFacts, Script, ScriptNote,
    SetExpr, SetOp, SetOpKind, Test, TestExpr,
};
pub use sound::{PLAYER_EXPLOSION_HALTS, SimSound};
