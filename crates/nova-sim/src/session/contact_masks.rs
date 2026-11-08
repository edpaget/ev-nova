//! The masks the contact passes and the scoop touch by, laid out from the
//! session's shared sheets (see the `sprite_sheets` module).
//!
//! **The contact passes** ([`Session::contact_sprites`]). Each ship's
//! sprite whose sheet was read, at the frame its last display's set and
//! its heading now give, each weapon `Graphic`'s and asteroid kind's sheet
//! read, the pass's speed multiplier (the frame's gain, `_gSpeedMult`,
//! one on both passes of double time; see the `time_adjustment` module)
//! and the readings of
//! [`RuleKey::RleCollisionWalk`] and [`RuleKey::ShotContact`]. Combat
//! lays each rect out where the walk meets the sprite (see
//! `nova_sim::combat::contact`). The player's frame set is the one its
//! display stepped this pass, as `_HandleShipDisplay` runs for it
//! (@0x43e29) before the collision passes (@0x43e70-0x44015); each NPC's is
//! the one its last display drew, as the NPCs display in the ship loop
//! after them (@0x444ad), which is the original's order too (see the
//! `ship_display` module).
//!
//! **The stellars** ([`Session::contact_sprites`]'s `stellars`). Each
//! destroyable listed stellar holding a sprite whose sheet was read, as
//! its slot last showed it ([`Session::shot_stellar_sprites`], see the
//! `shot_stellars` module): the sheet it draws (`Type`, `DeadType` or
//! graphic type 0's, as the deadly stellars map it), the frame its slot
//! shows, its place and the sizes its slot is placed by, for the
//! proximity pass's `_SWRLECollision(shot, stellar)` (@0x372a8, see
//! `nova_sim::combat::stellar`). A hidden stellar is lent its last slot's
//! sprite under its own ID: its owner's now, or the ghost fixed on
//! screen. The collision passes come before this pass's sprite tick, so
//! the frame is the one the last pass's left.
//!
//! **The scoop** ([`Session::scoop_masks`]). `_ShipCollisionHandler`'s
//! scoop branch (@0x370ab-0x371ac) runs in the collision passes, after
//! `_HandlePlayer` turned the player and its display drew it (@0x43e29),
//! with every rect where the last frame placed it, relative to the player
//! as it stood then. So the scoop runs after the player's move and its
//! display step ([`Session::scoop_after_display`]), the player's mask its
//! frame after this pass's turn, on the set its display this pass stepped,
//! but placed at the screen's centre, where it stood before the move, and
//! every NPC's and box's rect laid out from that same place, at the places
//! the move didn't change, each NPC on the set its last display drew. A box is laid out by the box
//! rule at its frame (`nova_sim::scoop::Scooping`).

use super::Session;
use crate::combat::ShipRef;
use crate::combat::contact::{ContactSprites, ShipSprite, ShotContact};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleKey;
use crate::sprite_mask::{MaskWalk, PlacedMask};

/// The masks a scoop tests by, from the player standing at a place: its
/// own, and each NPC's in NPC order, none where a sheet wasn't read.
pub(super) struct ScoopMasks {
    /// The player's.
    pub(super) player: Option<PlacedMask>,
    /// Each NPC's, in NPC order.
    pub(super) npcs: Vec<Option<PlacedMask>>,
}

impl Session {
    /// How the masks are walked, by [`RuleKey::RleCollisionWalk`].
    pub(super) fn mask_walk(&self) -> MaskWalk {
        MaskWalk::from(self.setup.rules.source_for(RuleKey::RleCollisionWalk))
    }

    /// Each ship's sprite whose sheet was read, the player's first, then
    /// each NPC's in turn (see the module docs).
    fn ship_sprites(&self) -> Vec<(ShipRef, ShipSprite)> {
        let arsenal = &self.statics.arsenal;
        let masks = &self.space.sprite_masks;
        let own = masks
            .ship_sprite(
                arsenal.sprite_base(self.pilot.ship),
                &self.ship.fit.hull.frames,
                self.ship.ship_display.drawn.set,
                self.ship.player.heading,
            )
            .map(|sprite| (ShipRef::Player, sprite));
        let npcs = self.npcs().iter().filter_map(|npc| {
            let sprite = masks.ship_sprite(
                arsenal.sprite_base(npc.ship),
                &npc.hull.frames,
                npc.display.drawn.set,
                npc.state.heading,
            )?;
            Some((ShipRef::Npc(npc.id), sprite))
        });
        own.into_iter().chain(npcs).collect()
    }

    /// What the contact passes touch by (see the module docs).
    pub(super) fn contact_sprites(&self) -> ContactSprites {
        let masks = &self.space.sprite_masks;
        ContactSprites {
            ships: self.ship_sprites(),
            shots: masks.shot_sheets(),
            asteroids: masks.asteroid_sheets(),
            stellars: self.shot_stellar_sprites(),
            speed_mult: self.time.gain().get(),
            walk: self.mask_walk(),
            reading: ShotContact::from(self.setup.rules.source_for(RuleKey::ShotContact)),
        }
    }

    /// The masks a scoop tests by, the player having stood at `stood` (see
    /// the module docs): the player's frame now at the centre, and each
    /// NPC's where it is from there.
    pub(super) fn scoop_masks(&self, stood: Vec2) -> ScoopMasks {
        let arsenal = &self.statics.arsenal;
        let masks = &self.space.sprite_masks;
        let centred = ShipState {
            position: stood,
            ..self.ship.player
        };
        let player = masks.ship(
            arsenal.sprite_base(self.pilot.ship),
            &self.ship.fit.hull.frames,
            self.ship.ship_display.drawn.set,
            &centred,
            stood,
        );
        let npcs = self
            .npcs()
            .iter()
            .map(|npc| {
                masks.ship(
                    arsenal.sprite_base(npc.ship),
                    &npc.hull.frames,
                    npc.display.drawn.set,
                    &npc.state,
                    stood,
                )
            })
            .collect();
        ScoopMasks { player, npcs }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use nova_sim_base::fixture::rle::sheet;

    use std::rc::Rc;

    use super::*;
    use crate::catalog::{MaskSheet, ShipId, StellarId};
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::combat::armament::Trigger;
    use crate::combat::contact::{SpriteAt, StellarSprite};
    use crate::flight::{Controls, Turn};
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{armed, facing_an_npc};
    use crate::sprite_mask::RleSheet;
    use crate::testkit::{FakePilotCatalog, Passive, work};

    /// The rows of a frame `side` square, drawn where `drawn` says.
    pub(in crate::session) fn square(side: u32, drawn: impl Fn(u32, u32) -> bool) -> RleSheet {
        let rows: Vec<String> = (0..side)
            .map(|y| {
                (0..side)
                    .map(|x| if drawn(x, y) { '#' } else { '.' })
                    .collect()
            })
            .collect();
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        sheet(&[&rows])
    }

    /// The NPC's class, 129, by its sprite base.
    const NPC_SHIP: MaskSheet = MaskSheet::Ship(ShipId(129));

    /// The blaster's `Graphic`, 0.
    const BLASTER_SHOT: MaskSheet = MaskSheet::Shot(0);

    /// [`armed`]'s catalog, the NPC 40 square, drawn only in its left
    /// quarter, and the blaster's shot 2 square.
    pub(in crate::session) fn left_quartered() -> FakePilotCatalog {
        let mut catalog = armed();
        catalog.rle_sheets = vec![
            (NPC_SHIP, square(40, |x, _| x < 10)),
            (BLASTER_SHOT, square(2, |_, _| true)),
        ];
        catalog
    }

    /// `catalog`'s session with the player at rest at the centre, facing
    /// up at its NPC 100 above at `npc_x`, its trigger held, by `rules`.
    pub(in crate::session) fn shooting(
        catalog: &FakePilotCatalog,
        npc_x: f32,
        rules: Rulebook,
    ) -> Session {
        let mut session = facing_an_npc(catalog, 180).with_rules(rules);
        session.ship.player.position = Vec2::ZERO;
        session.ship.player.velocity = Vec2::ZERO;
        session.space.traffic.npcs_mut()[0].state.position = Vec2::new(npc_x, -100.0);
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session
    }

    /// Whether the NPC's shield or armour fell in `passes` passes.
    fn struck(session: &mut Session, catalog: &FakePilotCatalog, passes: usize) -> bool {
        let before = session.npcs()[0].reserves;
        for _ in 0..passes {
            work(session, catalog, Controls::default());
        }
        let after = session.npcs()[0].reserves;
        after.shield.now < before.shield.now || after.armor.now < before.armor.now
    }

    #[test]
    fn contact_sprites_lend_the_frames_gain() {
        let catalog = crate::testkit::catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.time.zero();
        crate::session::time_adjustment::tests::frame_gains(&mut session, 2);
        assert!((session.contact_sprites().speed_mult - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn an_npc_is_struck_by_its_mask_and_missed_where_only_the_disc_meets_it() {
        let catalog = left_quartered();
        let mut clear = shooting(&catalog, 0.0, Rulebook::default());
        assert!(
            !struck(&mut clear, &catalog, 12),
            "the shot passes through its clear middle"
        );
        let mut drawn = shooting(&catalog, 15.0, Rulebook::default());
        assert!(struck(&mut drawn, &catalog, 12), "over its drawn quarter");
        // With no masks, the disc meets it in the middle.
        let discs = armed();
        let mut fallback = shooting(&discs, 0.0, Rulebook::default());
        assert!(struck(&mut fallback, &discs, 12));
        // By the other reading of `shot_contact`, the disc too.
        let swept = Rulebook::default().with_override(RuleKey::ShotContact, RuleSource::Bible);
        let mut other = shooting(&catalog, 0.0, swept);
        assert!(struck(&mut other, &catalog, 12));
    }

    #[test]
    fn the_sheets_are_asked_with_no_deadly_stellar_for_every_contact_each_once() {
        let mut catalog = left_quartered();
        catalog.asteroid_types = vec![crate::catalog::AsteroidTypeRecord {
            kind: 3,
            ..crate::catalog::AsteroidTypeRecord::default()
        }];
        let mut session = shooting(&catalog, 0.0, Rulebook::default());
        let rock = crate::asteroid::Asteroid {
            position: Vec2::new(900.0, 900.0),
            velocity: Vec2::ZERO,
            frame: 0.0,
            spin: 0.0,
            strength: 10,
            kind: 3,
        };
        session.space.asteroids.place(rock);
        session
            .space
            .boxes
            .spawn_scoopable(Vec2::new(-900.0, 0.0), Some(1), 2, &mut NeverFires);
        for _ in 0..3 {
            work(&mut session, &catalog, Controls::default());
        }
        let asked = catalog.rle_sheets_asked.borrow().clone();
        let mut once = asked.clone();
        once.sort_unstable();
        once.dedup();
        assert_eq!(asked.len(), once.len(), "each once: {asked:?}");
        for sheet in [
            MaskSheet::Ship(ShipId(128)),
            NPC_SHIP,
            BLASTER_SHOT,
            MaskSheet::Asteroid(3),
            MaskSheet::CargoBox(2),
        ] {
            assert!(asked.contains(&sheet), "{sheet:?} in {asked:?}");
        }
        assert!(
            !asked
                .iter()
                .any(|sheet| matches!(sheet, MaskSheet::Stellar(_))),
            "{asked:?}"
        );
    }

    #[test]
    fn a_shot_fired_on_double_times_second_pass_meets_by_its_mask_on_the_next() {
        let catalog = left_quartered();
        let mut session = shooting(&catalog, 0.0, Rulebook::default());
        session.hold_trigger(Trigger::default());
        work(&mut session, &catalog, Controls::default());
        session.hold_trigger(Trigger {
            primary: true,
            ..Trigger::default()
        });
        session.tick_again(
            Controls::default(),
            &Passive,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!session.shots().is_empty(), "fired on the second pass");
        session.hold_trigger(Trigger::default());
        assert!(
            !struck(&mut session, &catalog, 12),
            "through the clear middle by the masks, not the disc"
        );
    }

    #[test]
    fn the_players_contact_frame_is_its_frame_after_the_passs_turn() {
        let mut catalog = armed();
        catalog.rle_sheets = vec![(MaskSheet::Ship(ShipId(128)), square(10, |_, _| true))];
        let mut session = facing_an_npc(&catalog, 180);
        work(&mut session, &catalog, Controls::default());
        let before = session.ship.player.heading;
        let turning = Controls {
            turn: Turn::Left,
            ..Controls::default()
        };
        work(&mut session, &catalog, turning);
        let after = session.ship.player.heading;
        let frames = session.ship.fit.hull.frames;
        assert_ne!(
            frames.index(0, before),
            frames.index(0, after),
            "turned a frame"
        );
        let sprites = session.contact_sprites();
        let (_, own) = sprites
            .ships
            .iter()
            .find(|(ship, _)| *ship == ShipRef::Player)
            .expect("the player's");
        assert_eq!(
            own.frame,
            frames.index(session.ship.ship_display.drawn.set, after)
        );
    }

    /// A sheet of two sets of 36 frames, each 40 square: set 0 drawn in
    /// its left eighth alone, set 1 solid.
    fn strip_then_solid() -> RleSheet {
        let frame = |set: u32| -> Vec<String> {
            (0..40)
                .map(|_| {
                    (0..40)
                        .map(|x| if set == 1 || x < 5 { '#' } else { '.' })
                        .collect()
                })
                .collect()
        };
        let frames: Vec<Vec<String>> = (0..72).map(|frame_index| frame(frame_index / 36)).collect();
        let rows: Vec<Vec<&str>> = frames
            .iter()
            .map(|rows| rows.iter().map(String::as_str).collect())
            .collect();
        let frames: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
        sheet(&frames)
    }

    /// Gives `catalog`'s class `id` an animated `shän` of two sets,
    /// `AnimDelay` 0, so its set steps at every display, 36 frames a turn.
    fn alternating(catalog: &mut FakePilotCatalog, id: i16) {
        let hull = catalog
            .hulls
            .iter_mut()
            .find(|hull| hull.id == ShipId(id))
            .expect("the class's hull");
        hull.display = Some(crate::catalog::ShipDisplay {
            flags: crate::combat::ship_frames::ANIMATED,
            base_sets: 2,
            anim_delay: 0,
            ..crate::catalog::ShipDisplay::default()
        });
        hull.exits = Some(crate::catalog::ShipExits {
            frames: 36,
            ..crate::catalog::ShipExits::default()
        });
    }

    #[test]
    fn an_npcs_shot_meets_the_player_by_the_set_its_display_this_pass_steps() {
        use crate::session::tests::Firing;
        use crate::testkit::work_with;
        // `_HandleShipDisplay` steps the player (@0x43e29) before the
        // collision passes' `_ShipCollisionHandler` tests a shot against
        // its sprite (@0x37045). The player's set alternates each display;
        // the NPC's shots, down the player's middle, meet only set 1.
        let mut catalog = armed();
        alternating(&mut catalog, 128);
        catalog.rle_sheets = vec![
            (MaskSheet::Ship(ShipId(128)), strip_then_solid()),
            (BLASTER_SHOT, square(2, |_, _| true)),
        ];
        for start in [0, 1] {
            let mut session = facing_an_npc(&catalog, 180);
            session.ship.ship_display.frames.anim_set = start;
            session.ship.ship_display.drawn.set = start.cast_unsigned();
            let mut strikes = 0;
            for _ in 0..30 {
                let before = session.reserves();
                work_with(
                    &mut session,
                    &catalog,
                    Controls::default(),
                    &Firing,
                    Rules::default(),
                    &mut NeverFires,
                );
                let after = session.reserves();
                if after.shield.now < before.shield.now || after.armor.now < before.armor.now {
                    strikes += 1;
                    assert_eq!(session.player_drawn().set, 1, "from set {start}");
                }
            }
            assert!(strikes > 0, "from set {start}: struck");
        }
    }

    #[test]
    fn the_players_shot_meets_an_npc_by_the_set_its_last_display_drew() {
        // The NPCs display in the ship loop (@0x444ad), after the
        // collision passes (@0x43e70-0x44015): a shot meets an NPC by the
        // set its display drew on the pass before. Its set alternates each
        // display, set 0 drawn in its left quarter alone; the player's
        // shots, 15 right of its centre, meet only set 1.
        let mut catalog = armed();
        alternating(&mut catalog, 129);
        let left_quarter_then_solid = {
            let frame = |set: u32| -> Vec<String> {
                (0..40)
                    .map(|_| {
                        (0..40)
                            .map(|x| if set == 1 || x < 10 { '#' } else { '.' })
                            .collect()
                    })
                    .collect()
            };
            let frames: Vec<Vec<String>> = (0..72).map(|index| frame(index / 36)).collect();
            let rows: Vec<Vec<&str>> = frames
                .iter()
                .map(|rows| rows.iter().map(String::as_str).collect())
                .collect();
            let frames: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
            sheet(&frames)
        };
        catalog.rle_sheets = vec![
            (NPC_SHIP, left_quarter_then_solid),
            (BLASTER_SHOT, square(2, |_, _| true)),
        ];
        for start in [0, 1] {
            let mut session = shooting(&catalog, -15.0, Rulebook::default());
            session.space.traffic.npcs_mut()[0].display.frames.anim_set = start;
            session.space.traffic.npcs_mut()[0].display.drawn.set = start.cast_unsigned();
            let mut strikes = 0;
            for _ in 0..30 {
                let last_drawn = session.npcs()[0].display.drawn.set;
                if struck(&mut session, &catalog, 1) {
                    strikes += 1;
                    assert_eq!(last_drawn, 1, "from set {start}");
                }
                if strikes == 3 {
                    break;
                }
            }
            assert!(strikes > 0, "from set {start}: struck");
        }
    }

    #[test]
    fn a_weapon_of_no_graphic_asks_for_no_sheet() {
        let mut catalog = left_quartered();
        catalog.weapons.push(crate::catalog::WeaponRecord {
            graphic: -1,
            ..nova_sim_catalog::fixture::weapon(129)
        });
        let mut session = shooting(&catalog, 0.0, Rulebook::default());
        work(&mut session, &catalog, Controls::default());
        let asked = catalog.rle_sheets_asked.borrow().clone();
        assert!(asked.contains(&BLASTER_SHOT), "{asked:?}");
        assert!(!asked.contains(&MaskSheet::Shot(-1)), "{asked:?}");
    }

    #[test]
    fn the_contact_is_walked_by_the_rle_collision_walk() {
        let mut catalog = armed();
        let runs: Vec<String> = (0..40).map(|_| "r".repeat(40)).collect();
        let rows: Vec<&str> = runs.iter().map(String::as_str).collect();
        catalog.rle_sheets = vec![
            (NPC_SHIP, sheet(&[&rows])),
            (BLASTER_SHOT, square(2, |_, _| true)),
        ];
        let drawn = Rulebook::default().with_override(RuleKey::RleCollisionWalk, RuleSource::Bible);
        let mut original = shooting(&catalog, 0.0, Rulebook::default());
        assert!(!struck(&mut original, &catalog, 12), "a run never touches");
        let mut other = shooting(&catalog, 0.0, drawn);
        assert!(struck(&mut other, &catalog, 12));
    }

    // Stellars.

    use crate::catalog::{LandingSite, SiteAnimation};
    use crate::combat::weapon::PLANET_TYPE;
    use crate::session::deadly_masks::drawn_mask_sheet;
    use crate::story::DrawnSheet;
    use crate::testkit::{life, planet};

    /// Stellar 128's graphic type, 3.
    const TYPE_3: MaskSheet = MaskSheet::Stellar(3);

    /// Stellar 128 of graphic type 3, 40 square, at (`x`, -100).
    fn stellar_at(x: f32) -> LandingSite {
        LandingSite {
            graphic: 3,
            frame_size: Some((40, 40)),
            ..planet(128, x, -100.0)
        }
    }

    /// `catalog` with 128 of `Strength` 100 and the player's (and the
    /// NPC's) blaster planet-type.
    pub(in crate::session) fn planet_typed(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
        for weapon in &mut catalog.weapons {
            weapon.flags2 |= PLANET_TYPE;
        }
        catalog.living([life(128, 2)])
    }

    /// `session` showing and listing only `site`.
    fn showing(session: &mut Session, site: LandingSite) {
        session.chart.listed = vec![site];
        session.chart.sites = vec![site];
    }

    /// What the contact passes are lent of stellar 128.
    fn lent_128(session: &Session) -> Option<StellarSprite> {
        session
            .contact_sprites()
            .stellars
            .into_iter()
            .find(|(id, _)| *id == StellarId(128))
            .map(|(_, sprite)| sprite)
    }

    #[test]
    fn a_shown_destroyable_stellar_is_lent_at_its_place_on_the_frame_its_slot_shows() {
        let two_frames = sheet(&[&["#"], &["#"]]);
        let mut catalog = planet_typed(armed());
        catalog.rle_sheets = vec![(TYPE_3, two_frames)];
        let mut session = facing_an_npc(&catalog, 180);
        let site = LandingSite {
            animation: SiteAnimation {
                delay: 1,
                type_frames: 2,
                type_sheet_frames: 2,
                ..SiteAnimation::default()
            },
            ..stellar_at(70.0)
        };
        showing(&mut session, site);
        for _ in 0..12 {
            if session.stellar_frame(site.id) != 0 {
                break;
            }
            work(&mut session, &catalog, Controls::default());
        }
        let frame = session.stellar_frame(site.id);
        assert_ne!(frame, 0, "its animation stepped");
        let sprite = lent_128(&session).expect("lent");
        let read = session.space.sprite_masks.sheet(TYPE_3).expect("read");
        assert!(Rc::ptr_eq(&sprite.sheet, read));
        assert_eq!(sprite.frame, frame);
        assert_eq!(sprite.at, SpriteAt::Place(site.position));
        assert_eq!((sprite.size, sprite.placed_as), ((40, 40), (40, 40)));
    }

    #[test]
    fn a_frozen_slot_is_lent_its_stale_frames_sheet_placed_as_32_square() {
        use crate::rulebook::RuleSource;
        // Its `Type` counts no frames; its `DeadType`, graphic 5, 40 by
        // 30, is the sheet its slot kept showing as it froze.
        let mut catalog = planet_typed(armed());
        catalog.rle_sheets = vec![(TYPE_3, solid_square(40)), (DEAD_5, solid_square(30))];
        let mut session = facing_an_npc(&catalog, 180);
        let site = LandingSite {
            type_counts_frames: false,
            dead_graphic: Some(5),
            dead_frame_size: Some((40, 30)),
            ..stellar_at(70.0)
        };
        showing(&mut session, site);
        let sites = session.chart.sites.clone();
        let rebuilt = &mut session.space.rebuilt;
        rebuilt.frame(&sites, |id| id == site.id, RuleSource::Engine);
        rebuilt.frame(&sites, |_| false, RuleSource::Engine);
        assert!(session.frame_frozen(site.id));
        session.load_sprite_masks(&catalog);
        let sprite = lent_128(&session).expect("lent");
        let dead = session.space.sprite_masks.sheet(DEAD_5).expect("read");
        assert!(Rc::ptr_eq(&sprite.sheet, dead), "its `DeadType`'s");
        assert_eq!((sprite.size, sprite.placed_as), ((40, 30), (32, 32)));
    }

    /// Stellar 128's `DeadType`, graphic type 5.
    const DEAD_5: MaskSheet = MaskSheet::Stellar(5);

    /// A sheet of one frame `side` square, every pixel drawn.
    fn solid_square(side: u32) -> RleSheet {
        square(side, |_, _| true)
    }

    #[test]
    fn a_stellar_whose_sheet_was_not_read_or_that_cannot_be_destroyed_is_not_lent() {
        let mut catalog = planet_typed(armed());
        catalog.rle_sheets = vec![(TYPE_3, solid_square(40))];
        let mut session = facing_an_npc(&catalog, 180);
        showing(&mut session, stellar_at(70.0));
        assert_eq!(lent_128(&session), None, "not asked yet");
        session.load_sprite_masks(&catalog);
        assert!(lent_128(&session).is_some());
        let unread = LandingSite {
            graphic: 4,
            ..stellar_at(70.0)
        };
        showing(&mut session, unread);
        session.load_sprite_masks(&catalog);
        assert_eq!(lent_128(&session), None, "graphic type 4 isn't read");
        let destroyed = LandingSite {
            destroyed: true,
            dead_graphic: Some(3),
            dead_frame_size: Some((40, 40)),
            ..stellar_at(70.0)
        };
        showing(&mut session, destroyed);
        assert_eq!(lent_128(&session), None, "destroyed");
        let mut lifeless = armed();
        lifeless.rle_sheets = vec![(TYPE_3, solid_square(40))];
        let mut session = facing_an_npc(&lifeless, 180);
        showing(&mut session, stellar_at(70.0));
        session.space.sprite_masks.load(&lifeless, TYPE_3);
        assert_eq!(lent_128(&session), None, "of no `Strength`");
    }

    #[test]
    fn the_sheet_drawn_maps_to_its_graphic_type() {
        let site = LandingSite {
            dead_graphic: Some(5),
            ..stellar_at(0.0)
        };
        assert_eq!(drawn_mask_sheet(&site, DrawnSheet::Type), Some(TYPE_3));
        assert_eq!(drawn_mask_sheet(&site, DrawnSheet::Dead), Some(DEAD_5));
        assert_eq!(
            drawn_mask_sheet(&site, DrawnSheet::TypeZero),
            Some(MaskSheet::Stellar(0))
        );
        assert_eq!(drawn_mask_sheet(&site, DrawnSheet::NoFrame), None);
        assert_eq!(drawn_mask_sheet(&stellar_at(0.0), DrawnSheet::Dead), None);
    }

    /// Whether `catalog`'s session asks for any stellar's sheet in a pass,
    /// showing and listing `site`: the stellar sheets it asked.
    fn stellar_sheets_asked(catalog: &FakePilotCatalog, site: LandingSite) -> Vec<MaskSheet> {
        let mut session = facing_an_npc(catalog, 180);
        showing(&mut session, site);
        work(&mut session, catalog, Controls::default());
        catalog
            .rle_sheets_asked
            .borrow()
            .iter()
            .copied()
            .filter(|sheet| matches!(sheet, MaskSheet::Stellar(_)))
            .collect()
    }

    #[test]
    fn the_stellars_sheets_are_asked_while_the_arsenal_holds_a_planet_type_weapon() {
        let site = LandingSite {
            dead_graphic: Some(5),
            ..stellar_at(70.0)
        };
        let mut asked = stellar_sheets_asked(&planet_typed(armed()), site);
        asked.sort_unstable();
        assert_eq!(asked, [MaskSheet::Stellar(0), TYPE_3, DEAD_5]);
        let ordinary = armed().living([life(128, 2)]);
        assert_eq!(stellar_sheets_asked(&ordinary, site), []);
    }

    /// `catalog`'s session shooting up at 128, at (`x`, -100), its NPC
    /// out of the way: whether 128's strength fell in 12 passes.
    fn stellar_struck(catalog: &FakePilotCatalog, x: f32) -> bool {
        let mut session = shooting(catalog, 900.0, Rulebook::default());
        showing(&mut session, stellar_at(x));
        for _ in 0..12 {
            work(&mut session, catalog, Controls::default());
        }
        session
            .space
            .strength
            .get(&StellarId(128))
            .is_some_and(|&left| left < 100)
    }

    #[test]
    fn a_planet_type_shot_strikes_a_stellar_by_its_mask_and_misses_where_only_the_disc_meets_it() {
        let mut catalog = planet_typed(left_quartered());
        catalog.rle_sheets.push((TYPE_3, square(40, |x, _| x < 10)));
        assert!(!stellar_struck(&catalog, 0.0), "through its clear middle");
        assert!(stellar_struck(&catalog, 15.0), "over its drawn quarter");
        // With no stellar sheet, the disc meets it in the middle.
        let discs = planet_typed(left_quartered());
        assert!(stellar_struck(&discs, 0.0));
    }
}
