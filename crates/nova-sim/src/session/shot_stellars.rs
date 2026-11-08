//! The stellars the proximity pass walks, as `_CheckShotProximities`'
//! stellar branch walks them (`EV Nova.app`, i386, @0x3722e-0x37527).
//!
//! **The walk.** The branch counts the sixteen `NavDefs` slots of the
//! player's system in order (@0x3722e-0x37527), skipping an empty one, a
//! stellar that can't be destroyed (+0x40 ≤ 0, @0x3726d) and one
//! destroyed (`_StellarIsDestroyed` @0x3727b). It reads neither the shown
//! flag nor the owner, so a hidden stellar is walked as a shown one is,
//! in slot order among them. The session walks `chart.listed`, that list
//! in slot order.
//!
//! **The sprite tested** (@0x37288-0x372a8) is spöb +0, the slot sprite
//! `_HandleStellarSystemVisibility` last wrote (@0x32fc8), which nothing
//! clears: the sprite the stellar holds ([`Session::held_sprite`], see the
//! `stale_sprites` module), by [`RuleKey::StaleStellarSprite`](crate::rulebook::RuleKey::StaleStellarSprite):
//!
//! - **shown:** its own, as it is drawn;
//! - **hidden, its last slot owned by a stellar shown now:** that
//!   stellar's sprite, at its place, on the frame and sheet it draws;
//! - **hidden, its last slot owned by none:** graphic type 0's frame 0,
//!   fixed on screen at the slot's last place from the player
//!   ([`SpriteAt::Screen`]), as `_HandleStellarSprites` moves only owned
//!   slots (@0x2eeab-0x2ef1a); with no graphic type 0 frame, nothing;
//! - **hidden and never shown:** a null sprite, which fails
//!   `_SWRLECollision`'s assert (@0xabfd8 → @0xac160, `_SWAssertFail`)
//!   and quits the game. A crash is no reading: the engine strikes
//!   nothing there (record only).
//!
//! By the other reading a hidden stellar holds nothing and no shot
//! strikes it.
//!
//! **The hit lands on the walked stellar.** Its damage comes off the
//! stellar walked (@0x372b5-0x372eb), not its sprite's owner, and the
//! walk ends, so with a hidden stellar listed before its slot's owner,
//! a shot on the owner's sprite strikes the hidden one. So each target
//! ([`Session::shot_stellar_targets`]) and each sprite lent
//! ([`Session::shot_stellar_sprites`]) is the walked stellar's, under its
//! ID, with its strength.
//!
//! **Shot down** (@0x37409-0x3751f), a hidden stellar is destroyed as a
//! shown one is ([`Session::shoot_down`]): its death explosion at its own
//! place (@0x3742e), its `OnDestroy`, and the player convicted with its
//! own government.
//!
//! The beams take only the stellars shown ([`Session::stellar_targets`]):
//! `_HandleBeams` reads no stellar, and only the Bible's reading of
//! [`RuleKey::BeamStellars`](crate::rulebook::RuleKey::BeamStellars) hits one, by what the player sees.

use std::rc::Rc;

use super::Session;
use super::deadly_masks::drawn_mask_sheet;
use crate::catalog::{LandingSite, MaskSheet, StellarId};
use crate::combat::contact::{SpriteAt, StellarSprite};
use crate::combat::stellar::StellarTarget;
use crate::geometry::Vec2;
use crate::landing::frame_radius;
use crate::story::sprite_pool::SlotSprite;
use crate::story::wreck;

/// The sprite a walked stellar holds (see the module docs).
#[derive(Clone, Copy, Debug)]
enum Held<'a> {
    /// A stellar shown's, drawn as that stellar is: its own, or its last
    /// slot's owner's.
    Drawn(&'a LandingSite),
    /// An unowned slot's graphic type 0 frame, its top left this far from
    /// the player.
    Ghost { top_left: (i32, i32) },
}

impl Session {
    /// The walk (see the module docs): each listed stellar, in slot order,
    /// whole and destroyable, holding a sprite, with the sprite it holds.
    fn shot_walk(&self) -> Vec<(&LandingSite, Held<'_>)> {
        self.chart
            .listed
            .iter()
            .filter(|site| !site.destroyed)
            .filter(|site| {
                self.statics
                    .lives
                    .get(&site.id)
                    .is_some_and(wreck::destroyable)
            })
            .filter_map(|walked| {
                let held = match self.held_sprite(walked) {
                    SlotSprite::Own => Held::Drawn(self.chart.shown_site(walked)?),
                    SlotSprite::Owner(owner) => {
                        Held::Drawn(self.chart.sites.iter().find(|site| site.id == owner)?)
                    }
                    SlotSprite::Ghost { top_left } => Held::Ghost { top_left },
                    SlotSprite::None => return None,
                };
                Some((walked, held))
            })
            .collect()
    }

    /// The stellars the proximity pass walks, as shots can hit them (see
    /// the module docs): each walked stellar's disc, the fallback where no
    /// mask is read, under its ID with its strength. A stellar drawn is
    /// the disc of the stellar shown that draws it
    /// ([`Session::target_drawn_as`]). A ghost is the disc of graphic type
    /// 0's frame at its top left from the player's place now, which only
    /// the fallback reads; with no such frame, it is none.
    pub(super) fn shot_stellar_targets(&self) -> Vec<StellarTarget> {
        self.shot_walk()
            .into_iter()
            .filter_map(|(walked, held)| match held {
                Held::Drawn(drawn) => self.target_drawn_as(walked, drawn),
                Held::Ghost { top_left } => {
                    let frame = self.statics.fallback_frame?;
                    // A screen offset and a sprite's side are far within a
                    // float's exact range.
                    #[allow(clippy::cast_precision_loss)]
                    let (corner, size) = (
                        Vec2::new(top_left.0 as f32, top_left.1 as f32),
                        Vec2::new(frame.0 as f32, frame.1 as f32),
                    );
                    let centre = self.ship.player.position + corner + size * 0.5;
                    self.stellar_target(walked.id, centre, frame_radius(frame))
                }
            })
            .collect()
    }

    /// The sprite each walked stellar holds whose sheet was read, under
    /// its ID (see the module docs): a stellar drawn by the sheet its slot
    /// draws ([`Space::slot_layout`](super::Space::slot_layout)), the
    /// frame its slot shows ([`Session::stellar_frame`]), around its
    /// place, at the sizes its slot is laid out by; a ghost by graphic
    /// type 0's frame 0, fixed on screen at its top left. The collision
    /// passes run before this pass's sprite tick, so the frame is the one
    /// the last pass's left, as the original's sprite still shows it at
    /// `_CheckShotProximities` (@0x44015, `_HandleStellarSprites` @0x444d2
    /// after it).
    pub(super) fn shot_stellar_sprites(&self) -> Vec<(StellarId, StellarSprite)> {
        let (statics, setup) = (&self.statics, &self.setup);
        let masks = &self.space.sprite_masks;
        self.shot_walk()
            .into_iter()
            .filter_map(|(walked, held)| {
                let sprite = match held {
                    Held::Drawn(drawn) => {
                        let layout = self.space.slot_layout(statics, setup, drawn, Vec2::ZERO)?;
                        StellarSprite {
                            sheet: Rc::clone(masks.sheet(drawn_mask_sheet(drawn, layout.sheet)?)?),
                            frame: self.stellar_frame(drawn.id),
                            at: SpriteAt::Place(drawn.position),
                            size: layout.size,
                            placed_as: layout.placed_as,
                        }
                    }
                    Held::Ghost { top_left } => {
                        let frame = statics.fallback_frame?;
                        StellarSprite {
                            sheet: Rc::clone(masks.sheet(MaskSheet::Stellar(0))?),
                            frame: 0,
                            at: SpriteAt::Screen { top_left },
                            size: frame,
                            placed_as: frame,
                        }
                    }
                };
                Some((walked.id, sprite))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, StellarLife};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::weapon::Explosion;
    use crate::control::Script;
    use crate::flight::Controls;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::contact_masks::tests::{left_quartered, planet_typed, shooting, square};
    use crate::session::stale_sprites::tests::shown_then;
    use crate::session::stellars::tests::{Fine, bit, booms, destroyed};
    use crate::testkit::{FakePilotCatalog, Passive, life, planet, work, work_with};

    /// The hidden stellar here.
    const H: StellarId = StellarId(140);

    /// The stellar shown in its last slot.
    const S: StellarId = StellarId(141);

    /// Stellar `id` of graphic type 3, 40 square, at (`x`, `y`).
    fn typed(id: StellarId, x: f32, y: f32) -> LandingSite {
        LandingSite {
            graphic: 3,
            frame_size: Some((40, 40)),
            ..planet(id.0, x, y)
        }
    }

    /// [`left_quartered`]'s catalog, its weapons planet-type, `lives`
    /// living and graphic type 3's sheet drawn only on its left quarter
    /// when `masked`.
    fn catalog(masked: bool, lives: &[StellarLife]) -> FakePilotCatalog {
        let mut catalog = planet_typed(left_quartered()).living(lives.iter().cloned());
        if masked {
            catalog
                .rle_sheets
                .push((MaskSheet::Stellar(3), square(40, |x, _| x < 10)));
        }
        catalog
    }

    /// `catalog`'s session by `rules`, the player shooting straight up from
    /// the centre, the system listing `listed`, `first` shown at one
    /// rebuild point and `then` at the next, graphic type 0's frame 100
    /// square.
    fn listing(
        catalog: &FakePilotCatalog,
        rules: Rulebook,
        listed: &[LandingSite],
        (first, then): (&[LandingSite], &[LandingSite]),
    ) -> Session {
        let mut session = shooting(catalog, 900.0, rules);
        session.chart.listed = listed.to_vec();
        session.statics_mut().fallback_frame = Some((100, 100));
        shown_then(&mut session, first, then);
        session
    }

    /// H shown at the first rebuild point, S, of no `Strength`, taking
    /// its slot at the second; H at `h` and S at `s`.
    fn h_then_s(catalog: &FakePilotCatalog, rules: Rulebook, h: Vec2, s: Vec2) -> Session {
        let (hidden, owner) = (typed(H, h.x, h.y), typed(S, s.x, s.y));
        listing(catalog, rules, &[hidden, owner], (&[hidden], &[owner]))
    }

    /// Whether `stellar`'s strength fell below full in 12 passes.
    fn struck(session: &mut Session, catalog: &FakePilotCatalog, stellar: StellarId) -> bool {
        for _ in 0..12 {
            work(session, catalog, Controls::default());
        }
        session
            .space
            .strength
            .get(&stellar)
            .is_some_and(|&left| left < 100)
    }

    fn other_reading() -> Rulebook {
        Rulebook::default().with_override(RuleKey::StaleStellarSprite, RuleSource::Bible)
    }

    const FAR: Vec2 = Vec2::new(3000.0, 3000.0);

    #[test]
    fn a_hidden_stellar_shown_before_is_struck_by_its_slots_new_owners_mask() {
        let catalog = catalog(true, &[life(H.0, 2)]);
        // S's drawn quarter in the shot's path, H far off.
        let mut owned = h_then_s(&catalog, Rulebook::default(), FAR, Vec2::new(15.0, -100.0));
        assert!(struck(&mut owned, &catalog, H), "H, by S's sprite");
        assert_eq!(owned.space.strength.get(&S), None, "not S");
        // The shot through S's clear middle: the mask decides.
        let mut clear = h_then_s(&catalog, Rulebook::default(), FAR, Vec2::new(0.0, -100.0));
        assert!(!struck(&mut clear, &catalog, H), "through S's clear middle");
        // H in the shot's path, S far off: H's own place is harmless.
        let mut own = h_then_s(&catalog, Rulebook::default(), Vec2::new(15.0, -100.0), FAR);
        assert!(!struck(&mut own, &catalog, H), "H's own place");
    }

    /// H, of graphic type 0, shown at (`x`, -150) with the player at the
    /// centre and then hidden with nothing shown, the ghost drawn only on
    /// its left quarter, and the player moved to (2000, 0).
    fn ghost_of(x: f32, rules: Rulebook) -> (Session, FakePilotCatalog) {
        let mut catalog = catalog(false, &[life(H.0, 2)]);
        catalog
            .rle_sheets
            .push((MaskSheet::Stellar(0), square(100, |x, _| x < 25)));
        let hidden = planet(H.0, x, -150.0);
        let mut session = listing(&catalog, rules, &[hidden], (&[hidden], &[]));
        session.ship.player.position = Vec2::new(2000.0, 0.0);
        (session, catalog)
    }

    #[test]
    fn a_hidden_stellar_whose_slot_is_unowned_is_struck_at_the_ghost_on_screen() {
        // Its top left (-20, -200) from the player: the shot straight up
        // crosses its drawn quarter, 2000 from H's own place.
        let (mut session, catalog) = ghost_of(30.0, Rulebook::default());
        assert!(struck(&mut session, &catalog, H), "at the ghost");
        // Its top left (-50, -200): the shot crosses its clear middle,
        // within the disc.
        let (mut session, catalog) = ghost_of(0.0, Rulebook::default());
        assert!(!struck(&mut session, &catalog, H), "the mask decides");
    }

    #[test]
    fn a_hidden_stellars_sprite_falls_back_to_its_owners_disc_where_no_mask_is_read() {
        let catalog = catalog(false, &[life(H.0, 2)]);
        let mut session = h_then_s(&catalog, Rulebook::default(), FAR, Vec2::new(0.0, -100.0));
        assert!(struck(&mut session, &catalog, H), "within S's disc");
    }

    #[test]
    fn the_first_listed_holding_the_touched_sprite_takes_the_hit() {
        let catalog = catalog(true, &[life(H.0, 2), life(S.0, 2)]);
        let s = Vec2::new(15.0, -100.0);
        let mut engine = h_then_s(&catalog, Rulebook::default(), FAR, s);
        assert!(struck(&mut engine, &catalog, H), "H, listed first");
        assert_eq!(engine.space.strength.get(&S), None, "S keeps full strength");
        let mut other = h_then_s(&catalog, other_reading(), FAR, s);
        assert!(struck(&mut other, &catalog, S), "by the other reading, S");
        assert_eq!(other.space.strength.get(&H), None);
    }

    #[test]
    fn a_hidden_stellar_shot_down_is_destroyed_at_its_own_place() {
        let weak = StellarLife {
            strength: 1,
            explosion: Explosion::decode(4),
            on_destroy: Script::parse("b5"),
            ..life(H.0, 2)
        };
        let catalog = catalog(true, &[weak]);
        let hidden = LandingSite {
            govt: Some(GovtId(141)),
            ..typed(H, FAR.x, FAR.y)
        };
        let owner = typed(S, 15.0, -100.0);
        let mut session = listing(
            &catalog,
            Rulebook::default(),
            &[hidden, owner],
            (&[hidden], &[owner]),
        );
        let fine = Fine::default();
        let mut events = Vec::new();
        for _ in 0..12 {
            let rules = Rules {
                law: &fine,
                ..Rules::default()
            };
            let controls = Controls::default();
            work_with(
                &mut session,
                &catalog,
                controls,
                &Passive,
                rules,
                &mut NeverFires,
            );
            events.extend(session.take_combat_events());
        }
        assert!(
            destroyed(&session, H.0).is_some(),
            "destroyed in the galaxy"
        );
        assert_eq!(booms(&events, 4), [FAR], "at H's own place, not S's");
        assert!(session.control_bit(bit(5)), "its OnDestroy");
        let seen = fine.seen.take();
        assert!(!seen.is_empty(), "the player is convicted");
        assert!(
            seen.iter()
                .all(|&seen| seen == (crate::legal::Crime::Kill, Some(GovtId(141)))),
            "a killing of H's government: {seen:?}"
        );
    }

    #[test]
    fn a_hidden_stellar_never_shown_is_never_struck() {
        let catalog = catalog(true, &[life(H.0, 2)]);
        let hidden = typed(H, 15.0, -100.0);
        let shown = typed(S, FAR.x, FAR.y);
        let mut session = listing(
            &catalog,
            Rulebook::default(),
            &[shown, hidden],
            (&[shown], &[shown]),
        );
        assert!(session.shot_stellar_targets().is_empty());
        assert!(session.shot_stellar_sprites().is_empty());
        assert!(!struck(&mut session, &catalog, H));
        assert_eq!(session.space.strength.get(&H), None);
    }

    #[test]
    fn by_the_other_reading_no_hidden_stellar_is_struck() {
        let catalog = catalog(true, &[life(H.0, 2)]);
        let mut owned = h_then_s(&catalog, other_reading(), FAR, Vec2::new(15.0, -100.0));
        assert!(!struck(&mut owned, &catalog, H), "by its slot's owner");
        let (mut ghost, ghost_catalog) = ghost_of(30.0, other_reading());
        assert!(!struck(&mut ghost, &ghost_catalog, H), "at the ghost");
        // A shown one is struck by either reading.
        let shown = typed(H, 15.0, -100.0);
        let mut session = listing(&catalog, other_reading(), &[shown], (&[shown], &[shown]));
        assert!(struck(&mut session, &catalog, H));
    }

    #[test]
    fn the_beams_still_take_only_the_stellars_shown() {
        let catalog = catalog(true, &[life(H.0, 2)]);
        let session = h_then_s(&catalog, Rulebook::default(), FAR, Vec2::new(15.0, -100.0));
        let ids = |targets: Vec<StellarTarget>| -> Vec<StellarId> {
            targets.iter().map(|target| target.stellar).collect()
        };
        assert_eq!(ids(session.stellar_targets()), []);
        assert_eq!(ids(session.shot_stellar_targets()), [H]);
    }
}
