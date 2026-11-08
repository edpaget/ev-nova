//! A shot's contact with a destroyable stellar in the proximity pass, by
//! the two sprites' masks, as `_CheckShotProximities`' stellar branch
//! tests it (`EV Nova.app`, i386, @0x371bc-0x37775).
//!
//! **The test.** `_SWRLECollision(shot, stellar)` (@0x37288-0x372af): the
//! shot's sprite first, the stellar's second, so under the walk's quirk 2
//! ([`RuleKey::RleCollisionWalk`](crate::RuleKey::RleCollisionWalk)) the
//! shot's last drawn pixel at x − 1 touches the stellar's first at x, and
//! not the other way about. There is no rect pre-test in the branch: the
//! only bound is `_SWRLECollision`'s own strict overlap of the two rects
//! (@0xac020-0xac088), which [`PlacedMask::touches`] makes. It reads no
//! visibility flag, so a hidden shot (point defence's, at life 0) is
//! tested at its rect too.
//!
//! **The rects.** The pass runs before `_HandleShot` and
//! `_HandleStellarSprites` (@0x44015, @0x44385, @0x444d2), so both sprites
//! are where the last pass placed them, relative to the player as it then
//! stood: the shot by the shot rule at its frame then, as the contact
//! passes lay it out ([`contact`](super::contact)), carried back on a
//! wrap's pass; the stellar by its slot (`_HandleStellarSprites`
//! @0x2eeab-0x2ef1a), the standard rule at the size it is placed by
//! ([`StellarSprite::placed_as`]) around the stellar's place, at the frame
//! drawn's size, so a frozen slot's stale frame sits at the place less 16.
//! The wrap never carries a stellar, so on its pass the stellar's sprite
//! is met where it stood, as an asteroid's is (`Combat::rock_sprite`, see
//! [`wrap_carry`](super::wrap_carry)); the disc, the fallback, reads world
//! places.
//! The session lends the frame its last sprite tick left
//! ([`StellarSprite`]).
//!
//! **The hit's place.** `_CreateExplosion` takes the shot's position
//! (@0x3731d-0x3732a), so a hit by the masks lands where the shot is. The
//! masks test only where the shot is: a shot faster than the stellar's
//! extent can pass it between passes.
//!
//! **The fallback.** Where either mask can't be read (a stellar or a
//! weapon `Graphic` whose sheet the session didn't lend, or a sheet of no
//! frames), the original's `_SWAssertFail` (@0xac120-0xac1a6) is no
//! reading, so the engine keeps the swept disc ([`stellar::entered`]). By
//! the other reading of [`RuleKey::ShotContact`](crate::RuleKey::ShotContact)
//! ([`ShotContact::Swept`]) every stellar is that disc. The branch plays
//! no sound.

use super::Combat;
use super::contact::{ContactSprites, ShotContact, StellarSprite};
use super::projectile::Shot;
use super::stellar::{self, StellarTarget};
use crate::catalog::StellarId;
use crate::geometry::Vec2;
use crate::sprite_mask::{MaskRect, PlacedMask};
use crate::sprite_place::Placement;

impl ContactSprites {
    /// The mask of the stellar `id` lent, laid out about `offset` of its
    /// place, its sprite's offset from the player (see the module docs);
    /// none when it isn't lent.
    pub(super) fn stellar(
        &self,
        id: StellarId,
        offset: impl FnOnce(Vec2) -> Vec2,
    ) -> Option<PlacedMask> {
        let (_, sprite) = self.stellars.iter().find(|(lent, _)| *lent == id)?;
        let StellarSprite {
            sheet,
            frame,
            at,
            size,
            placed_as,
        } = sprite;
        let top_left = Placement::Standard.top_left(offset(*at), *placed_as);
        Some(PlacedMask {
            sheet: sheet.clone(),
            frame: *frame,
            rect: MaskRect::placed(top_left, *size),
        })
    }
}

impl Combat {
    /// The stellar among `stellars`, by its index, that `shot`, its last
    /// move from `was`, meets, and where it strikes it, as `sprites` lend
    /// the masks and readings and the player was seen at `camera` (see
    /// the module docs): the first still standing in the order given,
    /// which is the system's list (@0x37230-0x37527), by the masks, the
    /// shot's first, at the shot's position; else, by the other reading
    /// or where a mask can't be read, where its swept move entered the
    /// disc. None for a shot that can't hit one ([`stellar::can_hit`]).
    pub(super) fn stellar_met(
        &self,
        (shot, was): (&Shot, Vec2),
        stellars: &[StellarTarget],
        (sprites, camera): (&ContactSprites, Vec2),
    ) -> Option<(usize, Vec2)> {
        if !stellar::can_hit(shot) {
            return None;
        }
        let laid = if sprites.reading == ShotContact::Placed {
            let offset = shot.position + self.left_back(shot) - camera;
            sprites.shot(shot, offset).and_then(|laid| laid.mask)
        } else {
            None
        };
        stellars
            .iter()
            .enumerate()
            .filter(|(_, target)| !stellar::shot_down(target.strength))
            .find_map(|(index, target)| {
                let mask = sprites.stellar(target.stellar, |at| self.rock_sprite(at) - camera);
                let masks = laid.as_ref().zip(mask);
                let touch = masks.and_then(|(laid, mask)| laid.touches(&mask, sprites.walk));
                match touch {
                    Some(touch) => touch.then_some((index, shot.position)),
                    None => stellar::entered(shot, was, target).map(|at| (index, at)),
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::sprite_mask::RleSheet;
    use nova_sim_base::fixture::rle::sheet;

    /// A sheet of two one-pixel frames.
    fn two() -> Rc<RleSheet> {
        Rc::new(sheet(&[&["#"], &["#"]]))
    }

    /// Stellar 128 at (100, 50) of `sheet` showing frame 1, its frame
    /// `size`, placed as `placed_as`.
    fn lent(sheet: &Rc<RleSheet>, size: (u32, u32), placed_as: (u32, u32)) -> ContactSprites {
        let sprite = StellarSprite {
            sheet: Rc::clone(sheet),
            frame: 1,
            at: Vec2::new(100.0, 50.0),
            size,
            placed_as,
        };
        ContactSprites {
            stellars: vec![(StellarId(128), sprite)],
            ..ContactSprites::default()
        }
    }

    #[test]
    fn a_stellar_is_laid_out_about_its_place_from_the_camera_at_its_frames_size() {
        let drawn = two();
        let sprites = lent(&drawn, (40, 40), (40, 40));
        let mask = sprites
            .stellar(StellarId(128), |at| at - Vec2::new(10.0, 10.0))
            .expect("lent");
        assert_eq!(mask.rect, MaskRect::placed((70, 20), (40, 40)));
        assert!(Rc::ptr_eq(&mask.sheet, &drawn));
        assert_eq!(mask.frame, 1);
    }

    #[test]
    fn a_frozen_slots_stale_frame_sits_at_its_place_less_16() {
        let sprites = lent(&two(), (40, 30), (32, 32));
        let mask = sprites
            .stellar(StellarId(128), |at| at - Vec2::new(10.0, 10.0))
            .expect("lent");
        assert_eq!(mask.rect, MaskRect::placed((74, 24), (40, 30)));
    }

    #[test]
    fn a_stellar_not_lent_has_no_mask() {
        let sprites = lent(&two(), (40, 40), (40, 40));
        assert_eq!(sprites.stellar(StellarId(129), |at| at), None);
        assert_eq!(
            ContactSprites::default().stellar(StellarId(128), |at| at),
            None
        );
    }
}
