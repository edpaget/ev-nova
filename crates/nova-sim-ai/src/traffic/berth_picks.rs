//! The pick (+0x72) each ship-table slot keeps after its ship is gone.
//!
//! No spawn writes the original's +0x72 (`_RandomShipSpawn`,
//! `_GenericRandomShipSpawn`, `_MissionDudeSpawn`, `_SpecificDudeSpawn`,
//! `_RespawnEscort`, `_LaunchFighter`, `_SpawnEscort`, `_HyperSpawnFleet`,
//! `_SpawnPerson`, `_AISetup`), and nothing that frees a slot does: only
//! `_HandleShip` clears it once a pick without `Flags` 0x0002 fires
//! (@0x35012), `_DoShipCapture` clears it (@0x41207), and `_InitObjects`
//! clears it as the game starts (@0x1c385). So a new ship brought into a
//! slot (a [`Npc::berth`](crate::traffic::npc::Npc::berth)) holds the
//! pick the slot's last ship left, and its first gun pass that finds
//! nothing fires it, if it carries that weapon (`_FireAIShipWeapon`'s
//! preamble returns for a count of 0, @0x887c3).
//!
//! [`BerthPicks`], a [`BerthKept`], keeps that pick for each berth, written as each ship's
//! pick changes, and gives a new ship its berth's by the reading of
//! [`RuleKey::ReusedSlotPick`](crate::RuleKey::ReusedSlotPick): the
//! last ship's by the engine, none by the other reading.
//!
//! It lives as long as the [`Traffic`](crate::traffic::Traffic) holding
//! it, as its berths' persons do: across systems, but not across a new
//! traffic. The original keeps a slot's pick through a pilot load or a new
//! game until the app quits; that is recorded only (rdm phase
//! weapons-fidelity/phase-17-fire-gates-left-out-of-phase-5).

use crate::catalog::WeaponId;

use super::berth_kept::BerthKept;

/// The pick each berth's last ship left, and the reading a new ship takes
/// it by (see the module docs).
pub type BerthPicks = BerthKept<WeaponId>;
