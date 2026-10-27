//! The stellar sprite pool in flight, and the sprite a hidden deadly
//! stellar still kills by (see
//! [`sprite_pool`](crate::story::sprite_pool)).
//!
//! **The pool.** Each rebuild point (see the `stellars` module) hands the
//! stellars shown the pool's slots in ID order
//! ([`StellarSpritePool::rebuild`]), as `_HandleStellarSystemVisibility`
//! does, and places each owned slot where its stellar is drawn, from the
//! player's place then. Each flight frame's sprite tick places them again
//! ([`Session::see_stellars`]), as `_HandleStellarSprites` (@0x444d2)
//! moves the owned slots each pass. A slot is placed at its stellar's
//! place less half the frame it is drawn with
//! ([`Space::drawn_frame_size`]). The pool outlives the session: the
//! app hands it to the next pilot's session in the same run
//! ([`Session::stellar_sprite_pool`],
//! [`Session::carry_stellar_sprite_pool`]), as the original never clears
//! a stellar's sprite pointer.
//!
//! **Which sprite a deadly stellar kills by** ([`Session::deadly_discs`]):
//! `_HandleDeadlyStellars` walks the system's listed stellars in slot
//! order and tests each deadly one holding a sprite (spöb +0 not null,
//! @0x39d1d), shown or not:
//!
//! - **shown:** its own, the frame it is drawn with
//!   ([`Space::drawn_frame_size`]): its `DeadType` sheet while
//!   destroyed, graphic type 0's while destroyed and dead since the last
//!   rebuild point with no `DeadType` sheet, its `Type` sheet otherwise;
//! - **hidden, its last slot owned by a stellar shown now:** that
//!   stellar's, at its place with the frame it is drawn with;
//! - **hidden, its last slot owned by none:** graphic type 0's frame,
//!   which the rebuild gave every slot left with no frames
//!   (@0x33237-0x33315), at the slot's last screen place, which follows
//!   the player: its top left is the player's place now plus the slot's
//!   last offset;
//! - **hidden and never shown, or graphic type 0 unreadable for the
//!   ghost:** none. The original would read a null frame in
//!   `_SWRLECollision` there and crash; the engine kills nothing (record
//!   only).
//!
//! Those are [`RuleKey::StaleStellarSprite`]'s engine reading. By the
//! other reading a hidden stellar kills nothing. Each sprite is a disc,
//! half the larger side of its frame across, centred on its rect, where
//! the original compares masks: see rdm
//! phase/ship-handling-fidelity-followups/phase-1-deadly-stellars-pixel-masks.

use super::{Chart, PlayerShip, Session, Setup, Space, Statics};
use crate::catalog::LandingSite;
use crate::combat::stellar_hazards::DeadlyDisc;
use crate::deadly_stellar::{is_deadly, sprite_radius};
use crate::geometry::Vec2;
use crate::rulebook::{RuleKey, RuleSource};
use crate::story::sprite_pool::{SlotSprite, StellarSpritePool, offset};

impl Session {
    /// A rebuild point's sprite pool: the stellars shown take its slots,
    /// each placed where it is drawn (see the module docs).
    pub(super) fn rebuild_sprite_pool(&mut self) {
        self.space
            .stellar_sprites
            .rebuild(self.chart.sites.iter().map(|site| site.id));
        self.place_stellar_sprites();
    }

    /// Places each stellar shown that owns a slot, from the player's place
    /// now (see the module docs).
    pub(super) fn place_stellar_sprites(&mut self) {
        let placed: Vec<_> = self
            .chart
            .sites
            .iter()
            .map(|site| {
                let frame = self
                    .space
                    .drawn_frame_size(&self.statics, &self.setup, site)
                    .unwrap_or((0, 0));
                (
                    site.id,
                    offset(site.position, self.ship.player.position, frame),
                )
            })
            .collect();
        for (stellar, top_left) in placed {
            self.space.stellar_sprites.place(stellar, top_left);
        }
    }

    /// The disc each listed deadly stellar holding a sprite kills within,
    /// in slot order, by [`RuleKey::StaleStellarSprite`] (see the module
    /// docs).
    pub(super) fn deadly_discs(&self) -> Vec<DeadlyDisc> {
        let stale = self.setup.rules.source_for(RuleKey::StaleStellarSprite) == RuleSource::Engine;
        self.chart
            .listed
            .iter()
            .filter(|site| is_deadly(site.flags2))
            .filter_map(|listed| {
                let shown = self.chart.shown_site(listed);
                let sprite = match shown {
                    Some(_) => SlotSprite::Own,
                    None if stale => self.space.stellar_sprites.sprite_of(listed.id, false),
                    None => SlotSprite::None,
                };
                match sprite {
                    SlotSprite::Own => self.space.drawn_disc(&self.statics, &self.setup, shown?),
                    SlotSprite::Owner(owner) => {
                        let owner = self.chart.sites.iter().find(|site| site.id == owner)?;
                        self.space.drawn_disc(&self.statics, &self.setup, owner)
                    }
                    SlotSprite::Ghost { top_left } => self.ship.ghost_disc(&self.statics, top_left),
                    SlotSprite::None => None,
                }
            })
            .collect()
    }

    /// The stellar sprite pool, to hand to the next pilot's session in the
    /// same run (see the module docs).
    #[must_use]
    pub fn stellar_sprite_pool(&self) -> &StellarSpritePool {
        self.space.stellar_sprite_pool()
    }

    /// Takes on `pool`, the last session's in the same run, and hands it
    /// out again to the stellars shown now, as this session's first
    /// rebuild point would have, had the pool been its own then (see the
    /// module docs).
    pub fn carry_stellar_sprite_pool(&mut self, pool: StellarSpritePool) {
        self.space.stellar_sprites = pool;
        self.rebuild_sprite_pool();
    }
}

impl PlayerShip {
    /// The disc of an unowned slot placed at `top_left` from the player:
    /// graphic type 0's frame there, none when it cannot be read.
    fn ghost_disc(&self, statics: &Statics, top_left: (i32, i32)) -> Option<DeadlyDisc> {
        let frame = statics.fallback_frame?;
        // A screen offset and a sprite's side are far within a float's
        // exact range.
        #[allow(clippy::cast_precision_loss)]
        let (corner, size) = (
            Vec2::new(top_left.0 as f32, top_left.1 as f32),
            Vec2::new(frame.0 as f32, frame.1 as f32),
        );
        Some(DeadlyDisc {
            at: self.player.position + corner + size * 0.5,
            radius: sprite_radius(frame),
        })
    }
}

impl Space {
    /// The disc of `site`, shown, as it is drawn: none when nothing is.
    fn drawn_disc(
        &self,
        statics: &Statics,
        setup: &Setup,
        site: &LandingSite,
    ) -> Option<DeadlyDisc> {
        Some(DeadlyDisc {
            at: site.position,
            radius: sprite_radius(self.drawn_frame_size(statics, setup, site)?),
        })
    }

    /// The stellar sprite pool, to hand to the next pilot's session in the
    /// same run (see the module docs).
    #[must_use]
    fn stellar_sprite_pool(&self) -> &StellarSpritePool {
        &self.stellar_sprites
    }
}

impl Chart {
    /// `listed` among the stellars shown, as they hold it now.
    fn shown_site(&self, listed: &LandingSite) -> Option<&LandingSite> {
        self.sites.iter().find(|site| site.id == listed.id)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    use crate::deadly_stellar::DEADLY_STELLAR;
    use crate::flight::Controls;
    use crate::rulebook::Rulebook;
    use crate::session::tests::{armed, facing_an_npc};
    use crate::testkit::work;
    use crate::testkit::{FakePilotCatalog, planet};

    /// A deadly stellar `id` at (`x`, `y`), 100 x 100.
    fn deadly_at(id: i16, x: f32, y: f32) -> LandingSite {
        LandingSite {
            flags2: DEADLY_STELLAR,
            ..planet(id, x, y)
        }
    }

    /// `catalog`'s session under `rules`, the player at rest at the
    /// centre and its NPC at rest 100 above, the system listing `listed`,
    /// graphic type 0's frame 100 x 100.
    fn among(catalog: &FakePilotCatalog, listed: &[LandingSite], rules: Rulebook) -> Session {
        let mut session = facing_an_npc(catalog, 180).with_rules(rules);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.chart.listed = listed.to_vec();
        session.chart.sites = Vec::new();
        session.statics_mut().fallback_frame = Some((100, 100));
        session
    }

    /// Two rebuild points: `first` shown at the first, `then` at the
    /// second, from the player's place.
    fn shown_then(session: &mut Session, first: &[LandingSite], then: &[LandingSite]) {
        session.chart.sites = first.to_vec();
        session.rebuild_sprite_pool();
        session.chart.sites = then.to_vec();
        session.rebuild_sprite_pool();
    }

    fn fight(session: &mut Session) {
        work(session, &crate::testkit::catalog(), Controls::default());
    }

    fn other_reading() -> Rulebook {
        Rulebook::default().with_override(RuleKey::StaleStellarSprite, RuleSource::Bible)
    }

    /// H, deadly, hidden after the first rebuild point, and S, harmless,
    /// shown at the second in H's old slot; H at `h` and S at `s`.
    fn h_then_s(h: (f32, f32), s: (f32, f32), rules: Rulebook) -> Session {
        let catalog = armed();
        let hidden = deadly_at(140, h.0, h.1);
        let owner = planet(141, s.0, s.1);
        let mut session = among(&catalog, &[hidden, owner], rules);
        shown_then(&mut session, &[hidden], &[owner]);
        session
    }

    #[test]
    fn a_hidden_deadly_stellar_kills_by_its_slots_new_owner() {
        // S takes in the NPC, 100 above the centre; H is far off.
        let mut session = h_then_s((3000.0, 3000.0), (0.0, -150.0), Rulebook::default());
        fight(&mut session);
        assert_eq!(session.npcs(), [], "killed by S's sprite");
        // H takes in the NPC, S is far off: H's own place is harmless.
        let mut session = h_then_s((0.0, -150.0), (3000.0, 3000.0), Rulebook::default());
        fight(&mut session);
        assert_eq!(session.npcs().len(), 1, "H's own place");
    }

    #[test]
    fn a_hidden_deadly_stellar_whose_slot_is_unowned_kills_at_the_ghost_that_follows_the_player() {
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let ghost_on = |player: Vec2, npc: Vec2| {
            let mut session = among(&catalog, &[hidden], Rulebook::default());
            shown_then(&mut session, &[hidden], &[]);
            session.ship.player.position = player;
            session.space.traffic.npcs_mut()[0].state.position = npc;
            fight(&mut session);
            session.npcs().len()
        };
        // Placed with the player at the centre: the ghost's centre is
        // H's place, 150 above the player.
        assert_eq!(
            ghost_on(Vec2::ZERO, Vec2::new(0.0, -100.0)),
            0,
            "at the ghost"
        );
        let away = Vec2::new(2000.0, 0.0);
        assert_eq!(
            ghost_on(away, Vec2::new(2000.0, -100.0)),
            0,
            "the ghost moved with the player"
        );
        assert_eq!(
            ghost_on(away, Vec2::new(0.0, -100.0)),
            1,
            "H's own place is harmless"
        );
    }

    #[test]
    fn the_ghost_is_graphic_type_0s_frame_from_the_slots_top_left() {
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let mut session = among(&catalog, &[hidden], Rulebook::default());
        // Graphic type 0 a 300 x 40 sheet: its top left (-50, -200) from
        // the player, its centre (100, -180), half its larger side 150.
        session.statics_mut().fallback_frame = Some((300, 40));
        shown_then(&mut session, &[hidden], &[]);
        assert_eq!(
            session.deadly_discs(),
            [DeadlyDisc {
                at: Vec2::new(100.0, -180.0),
                radius: 150.0,
            }]
        );
    }

    #[test]
    fn with_no_graphic_type_0_sheet_the_ghost_kills_nothing() {
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let mut session = among(&catalog, &[hidden], Rulebook::default());
        shown_then(&mut session, &[hidden], &[]);
        session.statics_mut().fallback_frame = None;
        assert_eq!(session.deadly_discs(), []);
        fight(&mut session);
        assert_eq!(session.npcs().len(), 1);
    }

    #[test]
    fn a_hidden_deadly_stellar_never_shown_kills_nothing() {
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let shown = planet(141, 3000.0, 0.0);
        let mut session = among(&catalog, &[shown, hidden], Rulebook::default());
        shown_then(&mut session, &[shown], &[shown]);
        assert_eq!(session.deadly_discs(), []);
        fight(&mut session);
        assert_eq!(session.npcs().len(), 1);
    }

    #[test]
    fn by_the_other_reading_a_hidden_stellar_kills_nothing() {
        let mut owned = h_then_s((3000.0, 3000.0), (0.0, -150.0), other_reading());
        fight(&mut owned);
        assert_eq!(owned.npcs().len(), 1, "its slot's owner");
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let mut ghost = among(&catalog, &[hidden], other_reading());
        shown_then(&mut ghost, &[hidden], &[]);
        assert_eq!(ghost.deadly_discs(), []);
        fight(&mut ghost);
        assert_eq!(ghost.npcs().len(), 1, "the ghost");
        // A shown one still kills by either reading.
        let mut shown = among(&catalog, &[hidden], other_reading());
        shown_then(&mut shown, &[hidden], &[hidden]);
        fight(&mut shown);
        assert_eq!(shown.npcs(), []);
    }

    #[test]
    fn a_shown_destroyed_deadly_stellar_dead_since_the_rebuild_with_no_dead_type_kills_by_graphic_type_0()
     {
        let catalog = armed();
        // Its `Type` sheet, 10 across, does not reach the NPC 50 off;
        // graphic type 0's, 100 across, does.
        let dead = LandingSite {
            destroyed: true,
            frame_size: Some((10, 10)),
            ..deadly_at(140, 0.0, -150.0)
        };
        let mut session = among(&catalog, &[dead], Rulebook::default());
        session.chart.sites = vec![dead];
        session.space.rebuilt.rebuild(&[dead], |_| true, true);
        assert!(session.dead_since_rebuild(dead.id));
        fight(&mut session);
        assert_eq!(session.npcs(), [], "graphic type 0's frame reaches");
        // Not dead since the rebuild: its slot keeps its `Type` frames.
        let mut whole_then = among(&catalog, &[dead], Rulebook::default());
        whole_then.chart.sites = vec![dead];
        whole_then.space.rebuilt.rebuild(&[dead], |_| false, true);
        fight(&mut whole_then);
        assert_eq!(whole_then.npcs().len(), 1, "its Type frame");
    }

    #[test]
    fn a_hidden_stellar_still_pulls_by_either_reading() {
        let catalog = armed();
        let hidden = LandingSite {
            gravity: 40,
            ..planet(140, 2000.0, 0.0)
        };
        for rules in [Rulebook::default(), other_reading()] {
            let mut session = among(&catalog, &[hidden], rules);
            shown_then(&mut session, &[hidden], &[]);
            fight(&mut session);
            assert!(
                session.ship.player.velocity.x > 0.09,
                "{:?}",
                session.ship.player
            );
        }
    }

    #[test]
    fn a_rebuild_point_hands_out_the_pool_and_a_frame_places_it() {
        let catalog = armed();
        let mut session = facing_an_npc(&catalog, 180);
        session.rebuild_quietly();
        let site = *session.chart.sites.first().expect("a stellar shown");
        let pool = session.stellar_sprite_pool().clone();
        assert_eq!(pool.sprite_of(site.id, false), SlotSprite::Own);
        // A frame with the player moved places it again.
        let player = Vec2::new(30.0, -7.0);
        session.ship.player.position = player;
        session.see_stellars();
        let mut pool = session.stellar_sprite_pool().clone();
        pool.rebuild([]);
        let frame = session
            .space
            .drawn_frame_size(&session.statics, &session.setup, &site)
            .expect("drawn");
        assert_eq!(
            pool.sprite_of(site.id, false),
            SlotSprite::Ghost {
                top_left: offset(site.position, player, frame)
            }
        );
    }

    #[test]
    fn the_pool_carries_into_another_session() {
        let catalog = armed();
        let hidden = deadly_at(140, 0.0, -150.0);
        let mut first = among(&catalog, &[hidden], Rulebook::default());
        shown_then(&mut first, &[hidden], &[]);
        let mut next = among(&catalog, &[hidden], Rulebook::default());
        assert_eq!(next.deadly_discs(), [], "a fresh pool");
        next.carry_stellar_sprite_pool(first.stellar_sprite_pool().clone());
        assert_eq!(next.deadly_discs().len(), 1, "the carried ghost");
        // The stellars shown as it is carried take their slots again.
        let shown = planet(141, 3000.0, 0.0);
        let mut again = among(&catalog, &[hidden, shown], Rulebook::default());
        again.chart.sites = vec![shown];
        again.carry_stellar_sprite_pool(first.stellar_sprite_pool().clone());
        assert_eq!(
            again.stellar_sprite_pool().sprite_of(hidden.id, false),
            SlotSprite::Owner(shown.id)
        );
    }
}
