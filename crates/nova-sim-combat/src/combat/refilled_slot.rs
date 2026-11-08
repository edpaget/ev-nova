//! A shot follows its ship's slot once a new ship refills it
//! (rdm:phase/combat-and-flight/phase-5-refilled-slot-shot-not-lost).
//!
//! # The original
//!
//! A shot's ship (+0x2a) is a ship slot, not a ship. A ship destroyed in
//! a tick is cleared from its slot by `_HandleShipDisplay` (+0xb8
//! @0x2dee0, also @0x2c078), and a spawn takes the lowest free slot
//! (`_GenericRandomShipSpawn` @0x3c8b1-0x3c8e9, +0xb8 = 1 @0x3c90b): an
//! escape ship launched later in the same ship loop (`_LaunchFighter`
//! @0x354ea), reinforcements (`_HandleReinforcements` @0x44504), the
//! player's launch and eject (`_HandlePlayer` @0x43e19), arrivals
//! (`_EnterMoreShips` @0x44237) and the AI's launches (`_AIDispatch`
//! @0x4429d) all can, before the shot's next `_HandleShot`. Every reader
//! of +0x2a then reads the newcomer:
//!
//! - `_ShotCanHitShip`'s homing test (@0x44828-0x4482c) and its
//!   no-ship-source branch (@0x44a71) compare the slot, so the shot can
//!   hit the newcomer;
//! - `_CalculateTargetedDamage` (@0x3a1b7) counts a clear shot's damage
//!   against it;
//! - `_HandleShot`'s gone-ship rule (@0x355e0-0x35621) tests only the
//!   slot's active byte, so the shot is not lost: point defence is aimed
//!   at no slot every visit, a shot whose +0x2a is above 63 is skipped
//!   (@0x355fb-0x355ff), a jammed one (+0x30 ≠ 0) is skipped and keeps its
//!   slot (@0x35601-0x35606), and any other is lost (998, −1) only when the
//!   slot is inactive (@0x3561b-0x35621);
//! - `_HandleShotGuidance` (@0x32134-0x3215c) steers a homing shot at the
//!   newcomer: its +0x2e test compares systems (+0x74, Findings point 3),
//!   which a slot in flight always passes;
//! - its end-of-life release gives the newcomer to its sub-munitions
//!   (@0x35f3e-0x35f49).
//!
//! An interfered (jammed) shot names the newcomer as well. A shot locked
//! on an asteroid holds the asteroid's slot instead
//! ([`locked_slot`](super::locked_slot)), and is left to it. A
//! sub-munition given a locked parent's empty slot k holds k, so a refill
//! of k before its first visit saves it, as it would the original's.
//! Nothing here plays a sound.
//!
//! # Here
//!
//! The fight keeps [`ShipRef`](super::ShipRef) identities, and each shot remembers the
//! slot its ship held ([`Shot::aimed_slot`]). At the head of the
//! collision passes and of `_HandleShot`'s loop
//! ([`Combat::follow_refilled_slots`]) a shot whose ship is gone, and
//! whose slot an active ship now holds, is aimed at that ship
//! ([`follow`]); a shot aimed at no ship that holds a slot counts as one
//! whose ship is gone. The collision passes catch the refills of the
//! ship loop, the reinforcements and the player's launch and eject, and
//! `_HandleShot`'s those of the arrivals and the AI's launches, so
//! nothing is followed later than a frame the player sees. A gone ship's
//! slot that no ship holds is left for the gone-ship rule, which makes
//! the shot lost.
//!
//! The rulebook's
//! [`RuleKey::ReusedSlotShotTarget`](crate::RuleKey::ReusedSlotShotTarget)
//! chooses ([`RefilledSlot`]): the original's reading, or the port's
//! earlier one, under which a shot whose ship is gone is lost, whoever
//! takes its slot.
//!
//! # Left out
//!
//! - `_HandleBeams`' own aim (beam +0x18) is another routine, not read
//!   here.
//! - A child aimed at an empty slot's leftover position is
//!   rdm:phase/combat-and-flight-2/phase-1-stale-aim-at-an-empty-ship-slot.

use super::locked_slot::{holder, slot_number};
use super::projectile::{Jam, Shot, Target, active_among};
use super::weapon::Guidance;
use super::{Combat, Fighter};
use crate::rulebook::RuleSource;

/// What becomes of a shot whose ship's slot a new ship takes, as the
/// rulebook's
/// [`RuleKey::ReusedSlotShotTarget`](crate::RuleKey::ReusedSlotShotTarget)
/// says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefilledSlot {
    /// The original's: it follows the slot (see the module docs).
    #[default]
    Follows,
    /// The port's earlier reading: it is lost, whoever takes the slot.
    Lost,
}

impl From<RuleSource> for RefilledSlot {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Follows,
            RuleSource::Bible => Self::Lost,
        }
    }
}

/// Aims `shot` at the ship that took its ship's slot among `targets`, as
/// the original's +0x2a reads the slot (see the module docs). A shot is
/// left as it is when it holds no slot, is lost, is locked on an
/// asteroid, is point defence, or its ship is still active. A shot aimed
/// at no ship that holds a slot counts as one whose ship is gone. When an
/// active ship holds the slot ([`holder`]), it becomes the ship the
/// shot's damage is aimed at, and its target too when that was the ship
/// it had. With no holder it is left for the gone-ship rule.
pub fn follow(shot: &mut Shot, targets: &[Target]) {
    let Some(held) = shot.aimed_slot else {
        return;
    };
    if matches!(shot.jam, Jam::Lost | Jam::Asteroid(_))
        || shot.weapon.guidance == Guidance::PointDefence
    {
        return;
    }
    let gone = shot
        .aimed_at
        .is_none_or(|ship| active_among(targets, ship).is_none());
    if !gone {
        return;
    }
    let k = usize::try_from(slot_number(held)).unwrap_or(usize::MAX);
    let Some(newcomer) = holder(k, targets) else {
        return;
    };
    if shot.target == shot.aimed_at {
        shot.target = Some(newcomer);
    }
    shot.aimed_at = Some(newcomer);
}

impl Combat {
    /// Has a shot whose ship's slot a new ship takes do as `source` says
    /// ([`RuleKey::ReusedSlotShotTarget`](crate::RuleKey::ReusedSlotShotTarget),
    /// see [`refilled_slot`](super::refilled_slot)): by default, the
    /// engine's, it follows the slot; by the other reading, it is lost.
    pub fn set_reused_slot_shot_target(&mut self, source: RuleSource) {
        self.refilled_slot = RefilledSlot::from(source);
    }

    /// Each shot in flight follows its ship's slot among `fighters`
    /// ([`follow`]), by [`RefilledSlot::Follows`]; by
    /// [`RefilledSlot::Lost`] nothing changes.
    pub(super) fn follow_refilled_slots(&mut self, fighters: &[Fighter]) {
        if self.refilled_slot == RefilledSlot::Lost {
            return;
        }
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        for shot in &mut self.shots {
            follow(shot, &targets);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, WeaponRecord};
    use crate::combat::hull::Condition;
    use crate::combat::sides::Ties;
    use crate::combat::weapon::WeaponSpec;
    use crate::combat::{NpcId, ShipRef};
    use crate::flight::ShipState;
    use crate::geometry::Vec2;
    use crate::ship_layer::Slot;
    use nova_sim_catalog::fixture::weapon;

    /// The ship that was in berth 3 and left the fight.
    const OLD: ShipRef = ShipRef::Npc(NpcId(3));
    /// The ship that took berth 3.
    const NEW: ShipRef = ShipRef::Npc(NpcId(7));

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

    /// A shot of a weapon of `guidance`, NPC 1's, jammed as `jam`, fired
    /// at OLD in berth 3.
    fn shot(guidance: i16, jam: Jam) -> Shot {
        Shot {
            jam,
            target: Some(OLD),
            aimed_at: Some(OLD),
            aimed_slot: Some(Slot::Berth(3)),
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

    /// The fight with OLD gone and NEW in berth 3 as `condition`.
    fn refilled(condition: Condition) -> [Target; 2] {
        [
            at(npc(1), Slot::Berth(1), Condition::Intact),
            at(NEW, Slot::Berth(3), condition),
        ]
    }

    #[test]
    fn the_engines_reading_follows_the_slot_and_the_others_loses_the_shot() {
        assert_eq!(
            RefilledSlot::from(RuleSource::Engine),
            RefilledSlot::Follows
        );
        assert_eq!(RefilledSlot::from(RuleSource::Bible), RefilledSlot::Lost);
        assert_eq!(RefilledSlot::default(), RefilledSlot::Follows);
    }

    #[test]
    fn a_clear_shot_whose_slot_holds_a_new_ship_is_aimed_at_it() {
        let mut followed = shot(1, Jam::Clear);
        follow(&mut followed, &refilled(Condition::Intact));
        assert_eq!(
            (followed.target, followed.aimed_at, followed.jam),
            (Some(NEW), Some(NEW), Jam::Clear)
        );
        assert_eq!(followed.aimed_slot, Some(Slot::Berth(3)));
    }

    #[test]
    fn a_shot_whose_target_is_another_ship_keeps_it() {
        // Only the ship its damage is aimed at was the slot's.
        let mut followed = Shot {
            target: Some(npc(9)),
            ..shot(1, Jam::Clear)
        };
        follow(&mut followed, &refilled(Condition::Intact));
        assert_eq!(
            (followed.target, followed.aimed_at),
            (Some(npc(9)), Some(NEW))
        );
    }

    #[test]
    fn a_shot_whose_slot_is_empty_is_left_for_the_gone_ship_rule() {
        let targets = [at(npc(1), Slot::Berth(1), Condition::Intact)];
        let mut left = shot(1, Jam::Clear);
        follow(&mut left, &targets);
        assert_eq!(left, shot(1, Jam::Clear));
        // A destroyed ship in it holds it no more.
        let mut left = shot(1, Jam::Clear);
        follow(&mut left, &refilled(Condition::Destroyed));
        assert_eq!(left, shot(1, Jam::Clear));
    }

    #[test]
    fn a_shot_whose_ship_is_still_in_the_fight_is_unchanged() {
        let targets = [
            at(OLD, Slot::Berth(3), Condition::Intact),
            at(NEW, Slot::Berth(4), Condition::Intact),
        ];
        let mut kept = shot(1, Jam::Clear);
        follow(&mut kept, &targets);
        assert_eq!(kept, shot(1, Jam::Clear));
        // Breaking up, it is still active.
        let dying = [
            at(
                OLD,
                Slot::Berth(3),
                Condition::Dying {
                    ticks_left: 9,
                    starting: false,
                },
            ),
            at(NEW, Slot::Berth(3), Condition::Intact),
        ];
        let mut kept = shot(1, Jam::Clear);
        follow(&mut kept, &dying);
        assert_eq!(kept, shot(1, Jam::Clear));
    }

    #[test]
    fn an_interfered_shot_follows_its_slot_too() {
        let mut followed = shot(1, Jam::Interfered);
        follow(&mut followed, &refilled(Condition::Intact));
        assert_eq!(
            (followed.target, followed.aimed_at, followed.jam),
            (Some(NEW), Some(NEW), Jam::Interfered)
        );
    }

    #[test]
    fn a_locked_or_lost_shot_point_defence_or_one_holding_no_slot_does_not_follow() {
        for unmoved in [
            shot(1, Jam::Asteroid(Some(3))),
            shot(1, Jam::Lost),
            shot(9, Jam::Clear),
            Shot {
                aimed_slot: None,
                ..shot(1, Jam::Clear)
            },
        ] {
            let mut left = unmoved;
            follow(&mut left, &refilled(Condition::Intact));
            assert_eq!(left, unmoved);
        }
    }

    #[test]
    fn a_shot_aimed_at_no_ship_that_holds_a_slot_takes_the_ship_in_it() {
        // A sub-munition given a locked parent's empty slot k holds k
        // (@0x35f3e-0x35f49): a refill of k is followed like any other.
        let unaimed = Shot {
            target: None,
            aimed_at: None,
            ..shot(1, Jam::Clear)
        };
        let mut followed = unaimed;
        follow(&mut followed, &refilled(Condition::Intact));
        assert_eq!((followed.target, followed.aimed_at), (Some(NEW), Some(NEW)));
        let mut left = unaimed;
        follow(&mut left, &[at(npc(1), Slot::Berth(1), Condition::Intact)]);
        assert_eq!(left, unaimed, "k is still empty");
    }
}
