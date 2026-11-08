//! A shot locked on an asteroid reads the asteroid's slot as a ship slot
//! (rdm:phase/combat-and-flight/phase-1-locked-shot-reads-asteroid-slot-as-ship-slot).
//!
//! # The original
//!
//! A homing `Seeker` 0x0002 shot that locks (jam 1) has its ship slot
//! (+0x2a) overwritten with the asteroid's slot k (`_HandleShotGuidance`
//! @0x32919-0x3291f; see [`asteroid_seek`](super::asteroid_seek)). The code
//! outside guidance still reads +0x2a as a ship slot:
//!
//! - `_ShotCanHitShip` rejects only jam 998 (@0x447f7), so a locked shot
//!   goes on to the homing test (@0x44812-0x44836), which compares k with
//!   the ship's slot (+0x86): it hits only the ship in ship slot k, unless
//!   its weapon has `Flags2` 0x0008. The no-ship-source branch
//!   (@0x44a71-0x44a88) passes a ship whose slot is k or whose stellar's
//!   index is k. Once the locked asteroid is gone, +0x2a is -1 (@0x327b5):
//!   the homing test matches no ship, and the no-ship-source branch passes
//!   every ship (0xffff is above 63).
//! - At its end of life its sub-munitions are given k as their target
//!   (`_HandleShot` @0x35f3e-0x35f49): the ship in ship slot k. A seeking
//!   child (`Flags2` 0x0010) seeks first and falls back to k
//!   (@0x3f3e2-0x3f3f0). A child aimed at a slot no active ship holds,
//!   seated jam 0 and not point defence, is made lost (998) by the
//!   gone-ship rule on its first handling (@0x355f7-0x3561b), which tests
//!   the child's own +0x2a: so a seeking child that found a live ship
//!   keeps it and is not lost.
//!
//! Ship slot k is the player for k = 0 and the NPC of berth k otherwise
//! ([`ship_slot`]). No call here plays a sound.
//!
//! Here the rulebook's
//! [`RuleKey::LockedShotShipSlot`](crate::RuleKey::LockedShotShipSlot)
//! chooses ([`LockedSlot`]): the original's reading, or the asteroids
//! plan's, under which a locked shot hits no ship and its sub-munitions
//! are aimed at none.
//!
//! # Left out
//!
//! - A child aimed at an empty slot k heads, in the original, at
//!   `_CalcInterceptAngle` of `_shipData[k]`'s leftover position, with no
//!   active test (@0x3f402-0x3f42b): the slot's last ship's place, or
//!   zeroed memory. The fight keeps no position for an empty ship slot, so
//!   here such a child keeps its parent's heading, as with no target
//!   (rdm:phase/combat-and-flight-2/phase-1-stale-aim-at-an-empty-ship-slot).

use super::ShipRef;
use super::projectile::{Jam, Shot, Target};
use super::weapon::Guidance;
use crate::rulebook::RuleSource;
use crate::ship_layer::Slot;

/// How a locked shot reads the asteroid slot it holds, as the rulebook's
/// [`RuleKey::LockedShotShipSlot`](crate::RuleKey::LockedShotShipSlot)
/// says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LockedSlot {
    /// The original's: as a ship slot (see the module docs).
    #[default]
    ShipSlot,
    /// The asteroids plan's: a locked shot hits no ship, and its
    /// sub-munitions are aimed at none.
    NoShip,
}

impl From<RuleSource> for LockedSlot {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::ShipSlot,
            RuleSource::Bible => Self::NoShip,
        }
    }
}

impl LockedSlot {
    /// Whether the hit rules turn a shot of `jam` away from every ship
    /// before any other test: a lost one (998, @0x447f7), and by
    /// [`LockedSlot::NoShip`] a locked one too.
    #[must_use]
    pub fn bars(self, jam: Jam) -> bool {
        match jam {
            Jam::Lost => true,
            Jam::Asteroid(_) => self == Self::NoShip,
            Jam::Clear | Jam::Interfered => false,
        }
    }

    /// What `shot`'s end-of-life sub-munitions are given as their target
    /// among `targets` (`_HandleShot` @0x35f3e-0x35f49): the ship its
    /// damage is aimed at, or for a locked shot by
    /// [`LockedSlot::ShipSlot`] the ship holding its slot ([`holder`]),
    /// or the empty slot when none does. Any other locked shot gives
    /// none: the lock cleared the ship it was aimed at
    /// ([`asteroid_seek`](super::asteroid_seek)).
    #[must_use]
    pub fn released_at(self, shot: &Shot, targets: &[Target]) -> Given {
        match (shot.jam, self) {
            (Jam::Asteroid(Some(k)), Self::ShipSlot) => {
                holder(k, targets).map_or(Given::EmptySlot, |ship| Given::Ship(Some(ship)))
            }
            _ => Given::Ship(shot.aimed_at),
        }
    }
}

/// The target a release gives its sub-munitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Given {
    /// A ship, or none.
    Ship(Option<ShipRef>),
    /// A locked parent's ship slot that no active ship holds.
    EmptySlot,
}

impl Given {
    /// The ship given, if any.
    #[must_use]
    pub fn ship(self) -> Option<ShipRef> {
        match self {
            Self::Ship(ship) => ship,
            Self::EmptySlot => None,
        }
    }

    /// Sub-munition `child`, aimed ([`aim`](super::submunition::aim)), as
    /// it leaves the release: one given the empty slot that is still
    /// aimed at it (its seek found no ship), seated clear and not point
    /// defence, is lost ([`Jam::Lost`]), as the gone-ship rule makes it on
    /// its first handling (@0x355f7-0x3561b). One that found a ship keeps
    /// it; an interfered one stays interfered.
    #[must_use]
    pub fn settle(self, child: Shot) -> Shot {
        let lost = self == Self::EmptySlot
            && child.target.is_none()
            && child.jam == Jam::Clear
            && child.weapon.guidance != Guidance::PointDefence;
        if lost {
            Shot {
                jam: Jam::Lost,
                ..child
            }
        } else {
            child
        }
    }
}

/// Ship slot `k`, as the original reads an asteroid's slot in +0x2a: the
/// player's for 0, else berth `k`.
#[must_use]
pub fn ship_slot(k: usize) -> Slot {
    match u32::try_from(k) {
        Ok(0) => Slot::Player,
        Ok(berth) => Slot::Berth(berth),
        Err(_) => Slot::Berth(u32::MAX),
    }
}

/// The number of ship slot `slot`: 0 for the player's, else its berth.
#[must_use]
pub fn slot_number(slot: Slot) -> i64 {
    match slot {
        Slot::Player => 0,
        Slot::Berth(berth) => i64::from(berth),
    }
}

/// The active ship among `targets` holding ship slot `k`, if any: an
/// inactive one counts as none, as the slot's +0xb8 is clear.
#[must_use]
pub fn holder(k: usize, targets: &[Target]) -> Option<ShipRef> {
    let slot = ship_slot(k);
    targets
        .iter()
        .find(|target| target.slot == slot && target.condition.active())
        .map(|target| target.ship)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, WeaponRecord};
    use crate::combat::NpcId;
    use crate::combat::hull::Condition;
    use crate::combat::sides::Ties;
    use crate::combat::weapon::WeaponSpec;
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use nova_sim_catalog::fixture::weapon;

    fn npc(id: u32) -> ShipRef {
        ShipRef::Npc(NpcId(id))
    }

    /// `ship` in ship slot `slot`, holding up as `condition`.
    fn at(ship: ShipRef, slot: Slot, condition: Condition) -> Target {
        Target {
            ship,
            ship_type: ShipId(128),
            fleet: ship,
            ties: Ties::default(),
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            radius: 5.0,
            sprite_x: 15.0,
            sheet: true,
            condition,
            disabled: false,
            miss_class: 100,
            slot,
        }
    }

    /// A shot of a weapon of `guidance`, NPC 1's, jammed as `jam`, aimed
    /// at `aimed`.
    fn shot(guidance: i16, jam: Jam, aimed: Option<ShipRef>) -> Shot {
        Shot {
            jam,
            target: aimed,
            aimed_at: aimed,
            ..Shot::launch(
                WeaponSpec::new(&WeaponRecord {
                    guidance,
                    ..weapon(128)
                }),
                npc(1),
                npc(1),
                &ShipState::default(),
                0.0,
            )
        }
    }

    #[test]
    fn asteroid_slot_zero_is_the_players_ship_slot_and_any_other_its_berth() {
        assert_eq!(ship_slot(0), Slot::Player);
        assert_eq!(ship_slot(1), Slot::Berth(1));
        assert_eq!(ship_slot(15), Slot::Berth(15));
        assert_eq!(slot_number(Slot::Player), 0);
        assert_eq!(slot_number(Slot::Berth(7)), 7);
    }

    #[test]
    fn the_holder_of_a_slot_is_the_active_ship_in_it() {
        let targets = [
            at(ShipRef::Player, Slot::Player, Condition::Intact),
            at(npc(3), Slot::Berth(3), Condition::Disabled),
            at(npc(4), Slot::Berth(4), Condition::Destroyed),
        ];
        assert_eq!(holder(0, &targets), Some(ShipRef::Player));
        assert_eq!(holder(3, &targets), Some(npc(3)), "disabled is active");
        assert_eq!(holder(4, &targets), None, "a destroyed one is none");
        assert_eq!(holder(5, &targets), None, "an empty slot");
    }

    #[test]
    fn the_engines_reading_is_the_ship_slot_and_the_others_no_ship() {
        assert_eq!(LockedSlot::from(RuleSource::Engine), LockedSlot::ShipSlot);
        assert_eq!(LockedSlot::from(RuleSource::Bible), LockedSlot::NoShip);
        assert_eq!(LockedSlot::default(), LockedSlot::ShipSlot);
    }

    #[test]
    fn a_lost_shot_is_always_barred_and_a_locked_one_only_by_the_other_reading() {
        for rule in [LockedSlot::ShipSlot, LockedSlot::NoShip] {
            assert!(rule.bars(Jam::Lost), "{rule:?}");
            assert!(!rule.bars(Jam::Clear), "{rule:?}");
            assert!(!rule.bars(Jam::Interfered), "{rule:?}");
        }
        for jam in [Jam::Asteroid(Some(3)), Jam::Asteroid(None)] {
            assert!(!LockedSlot::ShipSlot.bars(jam), "{jam:?}");
            assert!(LockedSlot::NoShip.bars(jam), "{jam:?}");
        }
    }

    #[test]
    fn a_locked_parents_children_are_given_the_ship_in_its_slot_or_the_empty_slot() {
        let targets = [
            at(ShipRef::Player, Slot::Player, Condition::Intact),
            at(npc(3), Slot::Berth(3), Condition::Intact),
        ];
        let locked = |k| shot(1, Jam::Asteroid(k), None);
        let by = LockedSlot::ShipSlot;
        assert_eq!(
            by.released_at(&locked(Some(3)), &targets),
            Given::Ship(Some(npc(3)))
        );
        assert_eq!(
            by.released_at(&locked(Some(0)), &targets),
            Given::Ship(Some(ShipRef::Player))
        );
        assert_eq!(by.released_at(&locked(Some(5)), &targets), Given::EmptySlot);
        assert_eq!(by.released_at(&locked(None), &targets), Given::Ship(None));
        for k in [Some(3), Some(5), None] {
            assert_eq!(
                LockedSlot::NoShip.released_at(&locked(k), &targets),
                Given::Ship(None),
                "{k:?}"
            );
        }
        // An unlocked shot gives the ship its damage is aimed at.
        let aimed = shot(1, Jam::Clear, Some(npc(9)));
        for rule in [LockedSlot::ShipSlot, LockedSlot::NoShip] {
            assert_eq!(
                rule.released_at(&aimed, &targets),
                Given::Ship(Some(npc(9)))
            );
        }
    }

    #[test]
    fn a_clear_child_left_aimed_at_the_empty_slot_is_lost() {
        let child = shot(1, Jam::Clear, None);
        assert_eq!(Given::EmptySlot.settle(child).jam, Jam::Lost);
        assert_eq!(Given::EmptySlot.ship(), None);
        // Given a ship, or none, it is left as it is.
        assert_eq!(Given::Ship(None).settle(child).jam, Jam::Clear);
        assert_eq!(Given::Ship(Some(npc(2))).ship(), Some(npc(2)));
    }

    #[test]
    fn a_child_that_found_a_ship_an_interfered_one_or_point_defence_is_not_lost() {
        let found = shot(1, Jam::Clear, Some(npc(2)));
        assert_eq!(Given::EmptySlot.settle(found), found);
        let interfered = shot(1, Jam::Interfered, None);
        assert_eq!(Given::EmptySlot.settle(interfered), interfered);
        let defence = shot(9, Jam::Clear, None);
        assert_eq!(Given::EmptySlot.settle(defence), defence);
    }
}
