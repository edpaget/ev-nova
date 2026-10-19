//! The original's pool of stellar sprites, as `_HandleStellarSystemVisibility`
//! (HSSV, @0x32aa5) hands it out at each rebuild point, and the sprite a
//! stellar still points at once it is hidden (rdm
//! phase/ship-handling-fidelity/phase-24-left-out-of-phase-14, item 2).
//!
//! - **The pool.** Stellar sprites come from a pool of
//!   [`STELLAR_SPRITES`] (`*0x3b53d0`). Each HSSV hides all of them,
//!   nulls a cloned one's frames and sets its owner to -1 (@0x32e62-0x32eab),
//!   then walks the spöbs in index (ID) order: each one shown in the
//!   player's system takes the next slot, at most sixteen
//!   (@0x33205-0x33214). The stellar's sprite pointer, spöb +0, is set to
//!   its slot (@0x32fc8) and the slot's owner to the stellar (@0x32fda).
//!   Any slot left with no frames, unowned ones included, gets graphic
//!   type 0's frame 0 (@0x33237-0x33315).
//! - **No clear.** Nothing else writes spöb +0: not the loaders, a new
//!   pilot or a pilot loaded. So a stellar once shown keeps pointing at the
//!   slot it last took for the rest of the process, across hides, jumps
//!   and pilots; one never shown points at none.
//! - **Placement.** `_HandleStellarSprites` (@0x2e718-0x2e73e) moves only
//!   the owned slots each pass, to the owner's place on screen: its left
//!   is trunc(owner x − player x) + the screen's centre − half its frame's
//!   width (@0x2eeab-0x2ef1a), its top likewise. An unowned slot stays
//!   where it was last put on screen, so in world terms it follows the
//!   player at the offset of its last placement ([`offset`]).
//!
//! [`StellarSpritePool::sprite_of`] says which sprite a stellar holds now:
//! its own, the slot's new owner's, or the unowned slot's ghost.

use std::collections::BTreeMap;

use crate::catalog::StellarId;
use crate::geometry::Vec2;

/// How many stellar sprites the original's pool holds (`*0x3b53d0`).
pub const STELLAR_SPRITES: usize = 16;

/// The sprite a stellar holds now ([`StellarSpritePool::sprite_of`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotSprite {
    /// Its slot is its own: it is shown.
    Own,
    /// Its last slot is owned by another stellar, shown now.
    Owner(StellarId),
    /// Its last slot is owned by none: an invisible sprite of graphic type
    /// 0's frame, its top left this far from the player.
    Ghost {
        /// The top left's offset from the player, in whole pixels.
        top_left: (i32, i32),
    },
    /// It has never taken a slot, or its slot was never placed.
    None,
}

/// The stellar sprite pool (see the module docs): each stellar's last slot,
/// each slot's owner since the last rebuild point, and where each slot was
/// last placed. Never saved: it lives as long as the original's process.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StellarSpritePool {
    last: BTreeMap<StellarId, usize>,
    owners: [Option<StellarId>; STELLAR_SPRITES],
    placed: [Option<(i32, i32)>; STELLAR_SPRITES],
}

/// Where a sprite of `frame` (width, height) whose owner is at `owner` is
/// placed while the player is at `player`: its top left's offset from
/// the player, each axis truncated toward zero less half the frame's side,
/// halved as an integer.
#[must_use]
pub fn offset(owner: Vec2, player: Vec2, frame: (u32, u32)) -> (i32, i32) {
    // A screen offset is far within an `i32`, and the original truncates.
    #[allow(clippy::cast_possible_truncation)]
    let trunc = |delta: f32| delta.trunc() as i32;
    let half = |side: u32| i32::try_from(side / 2).unwrap_or(i32::MAX);
    (
        trunc(owner.x - player.x) - half(frame.0),
        trunc(owner.y - player.y) - half(frame.1),
    )
}

impl StellarSpritePool {
    /// A rebuild point: the stellars `shown` take the slots from 0 in ID
    /// order, at most [`STELLAR_SPRITES`]; every other slot is left with no
    /// owner, and every stellar remembers the slot it last took.
    pub fn rebuild(&mut self, shown: impl IntoIterator<Item = StellarId>) {
        let mut shown: Vec<StellarId> = shown.into_iter().collect();
        shown.sort_unstable();
        shown.dedup();
        self.owners = [None; STELLAR_SPRITES];
        for (slot, stellar) in shown.into_iter().take(STELLAR_SPRITES).enumerate() {
            self.owners[slot] = Some(stellar);
            self.last.insert(stellar, slot);
        }
    }

    /// Places `stellar`'s slot at `top_left` from the player, when it owns
    /// one; a stellar owning none moves nothing.
    pub fn place(&mut self, stellar: StellarId, top_left: (i32, i32)) {
        if let Some(slot) = self.owned(stellar) {
            self.placed[slot] = Some(top_left);
        }
    }

    /// The slot `stellar` owns now, if any.
    fn owned(&self, stellar: StellarId) -> Option<usize> {
        self.last
            .get(&stellar)
            .copied()
            .filter(|&slot| self.owners[slot] == Some(stellar))
    }

    /// The sprite `stellar` holds now, `shown` or not: its own while
    /// shown; else its last slot's owner's, the slot's ghost where it was
    /// last placed when no stellar owns it, or none when it never took a
    /// slot.
    #[must_use]
    pub fn sprite_of(&self, stellar: StellarId, shown: bool) -> SlotSprite {
        if shown {
            return SlotSprite::Own;
        }
        let Some(&slot) = self.last.get(&stellar) else {
            return SlotSprite::None;
        };
        match (self.owners[slot], self.placed[slot]) {
            (Some(owner), _) if owner == stellar => SlotSprite::Own,
            (Some(owner), _) => SlotSprite::Owner(owner),
            (None, Some(top_left)) => SlotSprite::Ghost { top_left },
            (None, None) => SlotSprite::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(ids: &[i16]) -> Vec<StellarId> {
        ids.iter().map(|&id| StellarId(id)).collect()
    }

    fn hidden(pool: &StellarSpritePool, id: i16) -> SlotSprite {
        pool.sprite_of(StellarId(id), false)
    }

    #[test]
    fn slots_go_by_id_not_by_listing_order() {
        let mut pool = StellarSpritePool::default();
        pool.rebuild(ids(&[131, 129, 130]));
        // Hide 129 (slot 0) and 131 (slot 2): 130 alone takes slot 0.
        pool.rebuild(ids(&[130]));
        assert_eq!(hidden(&pool, 129), SlotSprite::Owner(StellarId(130)));
        assert_eq!(hidden(&pool, 131), SlotSprite::None, "never placed");
        pool.place(StellarId(130), (1, 2));
        pool.rebuild(ids(&[]));
        assert_eq!(
            hidden(&pool, 129),
            SlotSprite::Ghost { top_left: (1, 2) },
            "slot 0, last placed by 130"
        );
    }

    #[test]
    fn at_most_sixteen_take_a_slot() {
        let mut pool = StellarSpritePool::default();
        let many: Vec<StellarId> = (128..145).map(StellarId).collect();
        pool.rebuild(many);
        // 144, the seventeenth, took none.
        pool.rebuild(ids(&[]));
        assert_eq!(hidden(&pool, 144), SlotSprite::None);
        assert_eq!(STELLAR_SPRITES, 16);
        pool.rebuild(ids(&[143]));
        assert_eq!(hidden(&pool, 128), SlotSprite::Owner(StellarId(143)));
        assert_eq!(hidden(&pool, 143), SlotSprite::Own, "its own slot");
    }

    #[test]
    fn a_stellar_hidden_at_a_rebuild_keeps_its_last_slot() {
        let mut pool = StellarSpritePool::default();
        pool.rebuild(ids(&[128, 129]));
        pool.place(StellarId(129), (-40, 7));
        pool.rebuild(ids(&[128]));
        assert_eq!(hidden(&pool, 129), SlotSprite::Ghost { top_left: (-40, 7) });
        // And through later rebuilds that leave slot 1 unowned.
        pool.rebuild(ids(&[]));
        pool.rebuild(ids(&[128]));
        assert_eq!(hidden(&pool, 129), SlotSprite::Ghost { top_left: (-40, 7) });
    }

    #[test]
    fn a_stellar_never_shown_holds_no_sprite() {
        let mut pool = StellarSpritePool::default();
        pool.rebuild(ids(&[128]));
        pool.place(StellarId(128), (5, 5));
        assert_eq!(hidden(&pool, 140), SlotSprite::None);
    }

    #[test]
    fn a_shown_stellar_holds_its_own() {
        let pool = StellarSpritePool::default();
        assert_eq!(pool.sprite_of(StellarId(140), true), SlotSprite::Own);
    }

    #[test]
    fn a_reused_slot_resolves_to_its_new_owner() {
        let mut pool = StellarSpritePool::default();
        pool.rebuild(ids(&[140]));
        pool.rebuild(ids(&[141]));
        assert_eq!(hidden(&pool, 140), SlotSprite::Owner(StellarId(141)));
        pool.rebuild(ids(&[140]));
        assert_eq!(hidden(&pool, 141), SlotSprite::Owner(StellarId(140)));
        assert_eq!(hidden(&pool, 140), SlotSprite::Own, "back in its slot");
    }

    #[test]
    fn an_unowned_slot_keeps_its_last_placement_whatever_other_slots_do() {
        let mut pool = StellarSpritePool::default();
        pool.rebuild(ids(&[128, 129]));
        pool.place(StellarId(128), (10, 20));
        pool.place(StellarId(129), (30, 40));
        pool.rebuild(ids(&[128]));
        pool.place(StellarId(128), (50, 60));
        pool.place(StellarId(129), (70, 80));
        assert_eq!(
            hidden(&pool, 129),
            SlotSprite::Ghost { top_left: (30, 40) },
            "slot 1 is unowned: neither 128's placement nor 129's moves it"
        );
    }

    #[test]
    fn an_offset_truncates_toward_zero_and_halves_the_frame_as_integers() {
        let player = Vec2::new(10.0, 10.0);
        assert_eq!(offset(Vec2::new(110.7, 9.5), player, (31, 20)), (85, -10));
        assert_eq!(offset(Vec2::new(9.5, 10.0), player, (1, 3)), (0, -1));
        assert_eq!(offset(Vec2::new(-90.9, 210.9), player, (0, 0)), (-100, 200));
    }
}
