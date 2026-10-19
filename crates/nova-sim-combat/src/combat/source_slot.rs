//! What a sub-munition reads of its parent's source slot as it spawns
//! (rdm weapons-fidelity phase 34).
//!
//! `_SpawnShotSubmunitions` spawns each sub-munition with its parent's
//! source slot (+0x2c, `_SpawnShot(parent +0x2c, target, sub, 1)`
//! @0x3f307-0x3f328), and the new shot keeps it (@0x3e7c4-0x3e7c8), so a
//! sub-munition's own sub-munitions read the same slot. With a ship slot
//! (0x3f or less, @0x3e5e1-0x3e5e6) `_SpawnShot` reads whatever ship that
//! slot holds now, which may have taken the slot since the parent was
//! fired:
//!
//! - its layer: a turret (4) goes over the ships by the slot's class
//!   (`ship[src] +0x76`, class +0x9ec bit 0x40, @0x3f0d6-0x3f109);
//! - its disable-only: whether that ship plunders the target
//!   (`_AIShipIsPlunderingShip` @0x3e826-0x3e899) or attempts a disable
//!   (`_AIIsShipAttemptingDisable` @0x3e9fb-0x3ea37).
//!
//! Freeing a ship slot clears only its active flag (+0xb8) and its leader
//! (+0x9a) (`_HandleShipDisplay` @0x2c078-0x2c07f, @0x2dee0-0x2dee7;
//! `_HandleShip` @0x3359a, @0x335c1): its class, goal and target stay, so
//! a sub-munition whose source slot is empty reads the slot's last ship.
//! The fight keeps that record by [`Slot`](crate::ship_layer::Slot) ([`SlotShip`]), written from
//! the fighters at the start of each phase ([`Combat::note_slots`]).
//!
//! [`RuleKey::SubmunitionSourceSlot`](crate::RuleKey::SubmunitionSourceSlot)
//! chooses: by the engine, the slot's ship as above; by the other reading,
//! the firer itself while it is in the fight, else the bit its parent
//! carried ([`Shot::turret_over`]) and no plunder.
//!
//! Left out: the sub-munition's system (+0x2e, `ship[src] +0x74`,
//! @0x3ea4b-0x3ea70), always the player's, as a parent lives only in its
//! firer's system; and the broadside's read of the slot's heading
//! (@0x3ee10-0x3ee7a), which `_SpawnShotSubmunitions` overwrites
//! (@0x3f50f-0x3f5a5).

use super::armament::Arsenal;
use super::plunder::Plunderer;
use super::projectile::{Shot, Source};
use super::{Combat, Fighter};
use crate::catalog::ShipId;
use crate::rulebook::RuleSource;

/// What a ship slot's record keeps of the last ship in it, which a
/// sub-munition reads (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotShip {
    /// Its class (+0x76).
    pub class: ShipId,
    /// Its plunder: its goal and its target (+0x70).
    pub plunderer: Plunderer,
}

impl Combat {
    /// Writes each of `fighters` into its slot's record, as the original's
    /// ship table holds it: a slot no fighter holds keeps its last ship's.
    pub(super) fn note_slots(&mut self, fighters: &[Fighter]) {
        for fighter in fighters {
            let ship = SlotShip {
                class: fighter.ship_type,
                plunderer: Plunderer::of(fighter),
            };
            self.slot_ships.insert(fighter.slot, ship);
        }
    }

    /// What a sub-munition of `parent` reads of its source as it spawns
    /// (see the module docs): whether a turret goes over the ships, and
    /// its owner's plunder. By
    /// [`RuleKey::SubmunitionSourceSlot`](crate::RuleKey::SubmunitionSourceSlot)'s
    /// engine reading, the ship its parent's source slot holds, or its
    /// last, its class read from `arsenal`; by the other, the firer among
    /// `fighters`. With neither, the bit the parent carried and no plunder.
    /// A shot no ship fired reads no ship.
    pub(super) fn source_slot_reads(
        &self,
        parent: &Shot,
        fighters: &[Fighter],
        arsenal: &Arsenal,
    ) -> (bool, Plunderer) {
        let fallback = (parent.turret_over, Plunderer::default());
        if !matches!(parent.source, Source::Ship(_)) {
            return fallback;
        }
        let ship = match self.source_slot {
            RuleSource::Engine => parent
                .firer_slot
                .and_then(|slot| self.slot_ships.get(&slot).copied()),
            RuleSource::Bible => fighters
                .iter()
                .find(|fighter| fighter.ship == parent.firer)
                .map(|firer| SlotShip {
                    class: firer.ship_type,
                    plunderer: Plunderer::of(firer),
                }),
        };
        ship.map_or(fallback, |ship| {
            (arsenal.hull(ship.class).turret_shots_over, ship.plunderer)
        })
    }

    /// Has a sub-munition read its parent's source as `source` says
    /// ([`RuleKey::SubmunitionSourceSlot`](crate::RuleKey::SubmunitionSourceSlot)):
    /// by default, the engine's, the ship its source slot holds.
    pub fn set_submunition_source_slot(&mut self, source: RuleSource) {
        self.source_slot = source;
    }
}
