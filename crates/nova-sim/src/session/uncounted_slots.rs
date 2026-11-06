//! Where a shot and a deadly stellar meet a stellar whose sprite slot's
//! frames count none (rendering phase 3).
//!
//! `_GetSpriteXSize` (@0x26d4-0x2702, its Y twin @0x2703-0x2731) reads 32
//! (`movl $0x20` @0x26fc) for a slot whose frame index is not below its
//! count (@0x26ec-0x26ef), so a slot whose frames count none, a frame
//! frozen ([`Session::frame_frozen`]), is placed as a 32-pixel sprite: its
//! rect's top left at the stellar's centre less 16 on each axis
//! (`_HandleStellarSprites` @0x2ee3d-0x2ef1a), the rect keeping the stale
//! frame's own size (`_SWSetSpriteLocation` @0xab66d-0xab6a0 moves it
//! without resizing it).
//!
//! - **Shots.** `_CheckShotProximities` (@0x37230-0x372af) tests each
//!   listed stellar of strength above 0 (@0x3726d-0x37272), not destroyed
//!   (@0x3727b), by `_SWRLECollision(shot sprite, stellar sprite)`
//!   (@0x37288-0x372a8): the slot's frame at its rect.
//! - **Deadly stellars.** `_HandleDeadlyStellars` (@0x39cd0-0x39e33)
//!   tests each live ship against each listed stellar holding a sprite
//!   (@0x39d1d-0x39d21) with `Flags2` 0x0100 (@0x39d2b) by
//!   `_SWRLECollision(stellar sprite, ship sprite)` (@0x39d77-0x39d97):
//!   the same slot read.
//!
//! The engine's shots still meet a stellar by a disc
//! (rdm:phase/combat-and-flight/phase-11-shot-on-stellar-by-masks), and a
//! deadly stellar falls back to one where its mask can't be read (the
//! mask itself is placed by [`Space::slot_layout`]). Each disc is centred
//! where the slot's rect is ([`Space::slot_centre`]): for a frozen slot,
//! the stale frame's centre placed from the centre less 16. A slot showing
//! no frame (`+0x50` null) faults in both readers in the original; the
//! engine hits nothing and kills nothing there.
//!
//! Only [`RuleKey::UncountedSheet`](crate::rulebook::RuleKey)'s engine
//! reading freezes a frame, so by the other reading every disc stays on
//! the stellar's centre. None of this plays a sound.

use super::stellars::NO_SPRITE_SIDE;
use super::{Setup, Space, Statics};
use crate::catalog::LandingSite;
use crate::geometry::Vec2;

impl Space {
    /// The centre of the rect `site`'s sprite slot is placed at, one of the
    /// stellars shown (see the module docs): none when it draws no frame
    /// ([`Space::drawn_frame_size`]); for a frame frozen, the stale frame's
    /// centre with its top left at `site`'s centre less 16 on each axis;
    /// else `site`'s centre.
    pub(super) fn slot_centre(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> Option<Vec2> {
        let (width, height) = self.drawn_frame_size(statics, setup, site)?;
        if !self.frame_frozen(setup, site.id) {
            return Some(site.position);
        }
        // A sprite's side is far within a float's exact range.
        #[allow(clippy::cast_precision_loss)]
        let (half_w, half_h, half_slot) = (
            width as f32 / 2.0,
            height as f32 / 2.0,
            NO_SPRITE_SIDE as f32 / 2.0,
        );
        Some(site.position + Vec2::new(half_w - half_slot, half_h - half_slot))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_base::fixture::rle::sheet;

    use super::*;
    use crate::catalog::{MaskSheet, StellarId};
    use crate::deadly_stellar::DEADLY_STELLAR;
    use crate::rulebook::RuleSource;
    use crate::session::Session;
    use crate::session::stellars::tests::{
        frozen_on_dead, keeps_a_dead_frame, player_at, quiet, uncounted_sheet,
    };
    use crate::sprite_mask::{MaskRect, RleSheet};

    const STELLAR: StellarId = StellarId(128);

    /// A frame `side` square, every pixel drawn.
    fn solid(side: u32) -> RleSheet {
        let row = "#".repeat(side as usize);
        let rows: Vec<&str> = (0..side).map(|_| row.as_str()).collect();
        sheet(&[&rows])
    }

    /// Stellar 128's shot target's centre and radius.
    fn target(session: &Session) -> Option<(Vec2, f32)> {
        session
            .stellar_targets()
            .iter()
            .find(|target| target.stellar == STELLAR)
            .map(|target| (target.position, target.radius))
    }

    /// Stellar 128 made deadly as it is listed and shown now.
    fn make_deadly(session: &mut Session) {
        for site in session
            .chart
            .listed
            .iter_mut()
            .chain(session.chart.sites.iter_mut())
            .filter(|site| site.id == STELLAR)
        {
            site.flags2 |= DEADLY_STELLAR;
        }
    }

    #[test]
    fn a_shot_meets_a_frozen_slot_by_its_stale_frame_from_the_centre_less_16() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        keeps_a_dead_frame(&mut session);
        assert!(session.frame_frozen(STELLAR));
        let centre = session.chart.sites[0].position;
        // Its `DeadType` frame, 200 square, from the centre less 16: its
        // centre 84 from the stellar's on each axis.
        assert_eq!(
            target(&session),
            Some((centre + Vec2::new(84.0, 84.0), 100.0))
        );
        // By the other reading nothing is frozen: its `Type`, 40 square,
        // on its centre.
        let session = session.with_rules(uncounted_sheet(RuleSource::Bible));
        assert_eq!(target(&session), Some((centre, 20.0)));
    }

    #[test]
    fn a_deadly_frozen_slots_disc_and_mask_sit_at_the_rect_a_32_pixel_slot_gives_it() {
        let mut catalog = frozen_on_dead();
        // Its `DeadType`, graphic 11, solid 200 square.
        catalog.rle_sheets = vec![(MaskSheet::Stellar(11), solid(200))];
        let mut session = quiet(&catalog);
        keeps_a_dead_frame(&mut session);
        player_at(&mut session, 0.0, 0.0);
        make_deadly(&mut session);
        session.load_sprite_masks(&catalog);
        let centre = session.chart.sites[0].position;
        let sprites = session.deadly_sprites();
        let [sprite] = sprites.as_slice() else {
            panic!("one deadly sprite: {sprites:?}");
        };
        assert_eq!(sprite.disc.at, centre + Vec2::new(84.0, 84.0));
        assert_eq!(sprite.disc.radius, 100.0);
        #[allow(clippy::cast_possible_truncation)]
        let (x, y) = (centre.x.trunc() as i32, centre.y.trunc() as i32);
        assert_eq!(
            sprite.mask.as_ref().map(|mask| mask.rect),
            Some(MaskRect {
                left: x - 16,
                top: y - 16,
                right: x + 184,
                bottom: y + 184,
            })
        );
        // By the other reading: its `Type` disc, 40 square, on its centre.
        let session = session.with_rules(uncounted_sheet(RuleSource::Bible));
        let discs: Vec<_> = session
            .deadly_sprites()
            .iter()
            .map(|sprite| (sprite.disc.at, sprite.disc.radius))
            .collect();
        assert_eq!(discs, [(centre, 20.0)]);
    }

    #[test]
    fn a_frameless_slot_is_neither_hit_nor_deadly() {
        let catalog = frozen_on_dead();
        let mut session = quiet(&catalog);
        make_deadly(&mut session);
        assert_eq!(target(&session), None);
        assert_eq!(session.deadly_sprites(), []);
    }
}
