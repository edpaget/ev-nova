//! Deadly stellars: a stellar whose `spöb` `Flags2` holds
//! [`DEADLY_STELLAR`] (0x0100) destroys outright any ship that touches
//! it, as `_HandleDeadlyStellars` (@0x39ccd-0x39e47) does in the original
//! (`EV Nova.app`, i386).
//!
//! - **Which stellars.** Each of the system's sixteen slots whose stellar
//!   has a sprite (spöb +0, written @0x32fc8 by
//!   `_HandleStellarSystemVisibility` for each stellar it shows, a
//!   destroyed one with its dead graphic) and is deadly (byte +0x35
//!   bit 0, [`is_deadly`]). Nothing clears spöb +0, so a stellar hidden
//!   since it was last shown still holds the pool slot it last took: it
//!   touches by that slot's sprite, the stellar that owns it now or, with
//!   none, an invisible graphic type 0 frame at the slot's last screen
//!   place, as [`RuleKey::StaleStellarSprite`] reads (see
//!   `nova_sim::story::sprite_pool` and the session's `stale_sprites`
//!   module). A stellar never shown holds none.
//! - **Which ships.** Every active ship, the player included, that is not
//!   breaking up (`_IsDying`) and does not resist ([`resists_deadly`],
//!   `_ShipResistsDeadlyStellars` @0x7bbe-0x7c59): a class of `Flags3`
//!   [`STELLAR_IMMUNE_HULL`] (0x0020), or the player carrying an `oütf`
//!   of `ModType` [`DEADLY_RESISTANCE`] (42) in any mod slot, whatever its
//!   `ModVal` (cached in `_playerResistsDeadlyStellars` 0x231764, reset
//!   @0xc430). Unlike gravity, inertialess flight does not resist. A
//!   disabled, cloaked or jumping ship is tested.
//! - **Touching** ([`touches`]): the original compares the two sprites'
//!   pixel masks (`_SWRLECollision` @0x39d97). The engine has no masks, so
//!   it tests discs, as a shot at a stellar or an asteroid does: a ship is
//!   its hull's hit radius, and a stellar's sprite half the larger side of
//!   the frame it holds ([`sprite_radius`]), centred on its rect. Which
//!   frame that is is the session's: the one the stellar is drawn with, or
//!   graphic type 0's for an unowned slot (`nova_sim::Session`'s
//!   deadly-stellar discs). A sprite with no frame touches nothing.
//!
//! What a kill does to the ship is the session's (`nova_sim::Session`'s
//! deadly-stellar step), under [`RuleKey::DeadlyStellarKill`]
//! ([`DeadlyStellarKill`]).

use crate::flight::Flyer;
use crate::geometry::Vec2;
#[cfg(any(doc, test))]
use crate::gravity::STELLAR_IMMUNE_HULL;
#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;

/// The `spöb` `Flags2` bit of a deadly stellar (`testb $0x1` on byte
/// +0x35, `_HandleDeadlyStellars` @0x39d1b).
pub const DEADLY_STELLAR: u16 = 0x0100;

/// The `oütf` `ModType` of the deadly-stellar resistance: carried by the
/// player at all, whatever its `ModVal`, it keeps a deadly stellar from
/// killing it (`cmpw $0x2a`, `_ShipResistsDeadlyStellars`).
pub const DEADLY_RESISTANCE: i16 = 42;

/// The armour a ship a deadly stellar kills is left with (the immediate
/// float -1000.0 written to +0x58 @0x39db6).
pub const KILLED_ARMOR: f32 = -1000.0;

/// Whether a stellar of `spöb` `Flags2` `flags2` is deadly.
#[must_use]
pub fn is_deadly(flags2: u16) -> bool {
    flags2 & DEADLY_STELLAR != 0
}

/// Whether a deadly stellar leaves a ship alone
/// (`_ShipResistsDeadlyStellars`, see the module docs): its class is
/// [`STELLAR_IMMUNE_HULL`] (`immune_hull`), or it is the player (`flyer`)
/// carrying a [`DEADLY_RESISTANCE`] (`outfit`). An NPC's outfit never
/// counts.
#[must_use]
pub fn resists_deadly(immune_hull: bool, outfit: bool, flyer: Flyer) -> bool {
    immune_hull || (outfit && flyer == Flyer::Player)
}

/// The radius a stellar's sprite touches ships within: half the larger
/// side of `frame` (width, height), the frame it holds.
#[must_use]
pub fn sprite_radius(frame: (u32, u32)) -> f32 {
    // A sprite's side is far within a float's exact range.
    #[allow(clippy::cast_precision_loss)]
    let side = frame.0.max(frame.1) as f32;
    side / 2.0
}

/// Whether a ship at `ship_at` of hit radius `ship_radius` touches a
/// stellar at `stellar_at` of `stellar_radius` ([`sprite_radius`]): their
/// discs overlap. A stellar of no radius touches nothing.
#[must_use]
pub fn touches(
    ship_at: Vec2,
    ship_radius: f32,
    stellar_at: Vec2,
    stellar_radius: Option<f32>,
) -> bool {
    stellar_radius.is_some_and(|radius| (ship_at - stellar_at).length() < ship_radius + radius)
}

/// What a deadly stellar's kill does to an NPC, by
/// [`RuleKey::DeadlyStellarKill`]. In both readings the ship is gone at
/// once with its `Explode2`, and there is no breakup, shockwave, debris,
/// crime, kill credit or escape pod.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeadlyStellarKill {
    /// The original's: the ship vanishes. Its slot goes inactive and it
    /// never reaches `_HandleShipDisplay`'s death branch, so it raises no
    /// destroyed fate, is not counted as a lost mission ship and does not
    /// fail its mission, and its person lives on (@0x2daa2-0x2dca8).
    #[default]
    Vanishes,
    /// What a plug-in author would expect: it counts as destroyed, its
    /// fate raised, a mission ship counted lost and a person without an
    /// escape pod gone, as a ship destroyed in a fight.
    Destroyed,
}

impl From<RuleSource> for DeadlyStellarKill {
    /// The engine's reading is the original's; the other counts the ship
    /// destroyed.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Vanishes,
            RuleSource::Bible => Self::Destroyed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stellar_is_deadly_by_its_flags2_0x0100_alone() {
        assert_eq!(DEADLY_STELLAR, 0x0100);
        assert!(is_deadly(0x0100));
        assert!(is_deadly(0xFFFF));
        assert!(!is_deadly(0xFEFF), "every other bit");
    }

    #[test]
    fn a_stellar_immune_hull_resists_and_the_outfit_only_the_players() {
        for flyer in [Flyer::Player, Flyer::Npc] {
            assert!(resists_deadly(true, false, flyer));
            assert!(!resists_deadly(false, false, flyer));
        }
        assert!(resists_deadly(false, true, Flyer::Player));
        assert!(!resists_deadly(false, true, Flyer::Npc));
        assert_eq!((STELLAR_IMMUNE_HULL, DEADLY_RESISTANCE), (0x0020, 42));
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_killed_ship_is_left_at_minus_1000_armour() {
        assert_eq!(KILLED_ARMOR, -1000.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn a_sprite_touches_within_half_the_larger_side_of_its_frame() {
        assert_eq!(sprite_radius((100, 60)), 50.0);
        assert_eq!(sprite_radius((30, 51)), 25.5);
        assert_eq!(sprite_radius((0, 0)), 0.0);
    }

    #[test]
    fn a_ship_touches_a_stellar_its_disc_overlaps() {
        let at = Vec2::new(100.0, 100.0);
        assert!(touches(Vec2::new(160.0, 100.0), 10.0, at, Some(55.0)));
        assert!(touches(at, 10.0, at, Some(0.0)), "on its centre");
        assert!(
            !touches(Vec2::new(165.0, 100.0), 10.0, at, Some(55.0)),
            "just apart"
        );
        assert!(
            touches(Vec2::new(164.9, 100.0), 10.0, at, Some(55.0)),
            "just within"
        );
        assert!(!touches(Vec2::new(100.0, 166.0), 10.0, at, Some(55.0)));
        assert!(!touches(at, 10.0, at, None), "a stellar with no frame");
    }

    #[test]
    fn the_rules_reading_follows_its_source() {
        assert_eq!(
            DeadlyStellarKill::from(RuleSource::Engine),
            DeadlyStellarKill::Vanishes
        );
        assert_eq!(
            DeadlyStellarKill::from(RuleSource::Bible),
            DeadlyStellarKill::Destroyed
        );
        assert_eq!(DeadlyStellarKill::default(), DeadlyStellarKill::Vanishes);
    }
}
