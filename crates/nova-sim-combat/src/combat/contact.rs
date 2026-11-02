//! A shot's contact with a ship or an asteroid in the contact passes, by
//! the two sprites' placed rects, as `_ShipCollisionHandler` and
//! `_AsteroidCollisionHandler` test it (`EV Nova.app`, i386; see
//! [`collision`](super::collision) for the walk).
//!
//! **The rects.** `_SWCollideSpriteLayer` (@0xacc30-0xacc81) calls a
//! proc only for two sprites whose rects overlap, strictly on both axes
//! ([`MaskRect::overlaps`]). Each rect is where the last pass placed it,
//! relative to the player as it then stood (the walk's `seen` places, see
//! [`Lineup`](super::lineup::Lineup)): a ship by the standard rule at its
//! drawn frame's size (`_HandleShipDisplay`, [`Placement::Standard`]), the
//! sheet's when it was read, else its `shän`'s `BaseXSize` and
//! `BaseYSize`, which every stock sheet's frame equals; a shot by the
//! shot rule, half its frame's width off both axes (`_HandleShot`
//! @0x35c66-0x35cc0, [`Placement::Shot`]), carried back on a wrap's pass
//! ([`wrap_carry`](super::wrap_carry)); an asteroid by the standard rule
//! at its sheet's size. The collision insets (+0x56, @0xacc87-0xacca1)
//! are never set (`_SWSetFrameCollisionInset` @0xadf5d has no caller), so
//! they are not modelled.
//!
//! **A ship** (`_ShipCollisionHandler` @0x3700f-0x3704a, the ship first,
//! the shot second). It takes `_SWRadiusCollision` (@0xabc88,
//! [`radius_touch`]) when the pass's speed multiplier is
//! [`RADIUS_SPEED_MULT`] or more (`ucomisd`/`jae` against the double
//! @0xdd1c8) or the ship's rect is [`RADIUS_WIDTH`] wide or less
//! (`_GetSpriteXSize`, `cmpw $0x20`/`jg`), and `_SWRLECollision`
//! (@0x37045, [`PlacedMask::touches`]) otherwise ([`shape`]). The session
//! lends the frame's gain ([`ContactSprites::speed_mult`]), which at the
//! engine's fixed step stays within 0 to 1: each step is a frame of 1.0 of
//! new time, so the ramp never rises past it, and the original reaches
//! 2.0 only on frames of about 67 ms or more, which the engine runs as
//! catch-up steps. So the speed's branch is built but not reached.
//! `_GetSpriteXSize`'s 32 for a sprite with no frames is never reached.
//!
//! **An asteroid** (`_AsteroidCollisionHandler` @0x369c8-0x36a44, the
//! asteroid first): `Seeker` 0x0001 misses it, then `_SWRLECollision`
//! (@0x36a0e) over its frame ([`Asteroid::shown_frame`]) and the shot's.
//!
//! **The masks** are each sheet's run-length tokens, which the session
//! lends ([`ContactSprites`]): a ship's sprite base's sheet at the frame
//! its last display drew, a shot's weapon sheet at the frame
//! [`Shot::frame_on`] gives, an asteroid kind's at its shown frame. They are
//! walked as [`RuleKey::RleCollisionWalk`](crate::RuleKey::RleCollisionWalk)
//! says, quirk 2 included: the ship's or asteroid's last drawn pixel at
//! x − 1 touches the shot's first at x.
//!
//! **The hit's place.** `_HandleShipHit` and `_HandleAsteroidHit` explode
//! where the shot is (@0x36aa9, @0x3667d), so a contact lands at the
//! shot's position after its move.
//!
//! **The fallback.** Where the shape test can't be laid out (no frame size
//! for a rect, or no mask for `_SWRLECollision`: a sheet the catalog
//! couldn't read, which no stock sheet is), the contact keeps the
//! engine's swept test as it stood ([`Shot::touches`],
//! [`asteroid_hit::touches`]).
//!
//! By the other reading of [`RuleKey::ShotContact`](crate::RuleKey::ShotContact)
//! ([`ShotContact::Swept`]) every contact is that swept test, so no shot
//! passes through a ship between ticks; by the engine's a shot faster than
//! its target's extent can. None of this plays a sound. The 16-bit rects,
//! halves and centres of the original are whole numbers here, as wide as
//! they need.
//!
//! [`Asteroid::shown_frame`]: crate::asteroid::Asteroid::shown_frame

use std::collections::BTreeMap;
use std::rc::Rc;

use super::asteroid_hit::{self, Rock};
use super::hull::HullSpec;
use super::projectile::{Shot, Target};
use super::{Combat, ShipRef};
use crate::asteroid::KINDS;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::speed_mult::SpeedMult;
use crate::sprite_mask::{MaskRect, MaskWalk, PlacedMask, RleSheet};
use crate::sprite_place::{Placement, radius_touch};

/// The speed multiplier at and past which a ship is tested by radius (the
/// double @0xdd1c8).
pub const RADIUS_SPEED_MULT: f64 = 2.0;

/// The widest ship tested by radius at any speed (`cmpw $0x20` @0x3702a).
pub const RADIUS_WIDTH: i64 = 32;

/// How a shot touches a ship or an asteroid, by
/// [`RuleKey::ShotContact`](crate::RuleKey::ShotContact).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShotContact {
    /// The original's placed rects (see the module docs), the default.
    #[default]
    Placed,
    /// The engine's swept hit radius, the other reading.
    Swept,
}

impl From<RuleSource> for ShotContact {
    /// The engine's reading is the placed rects; the other the swept test.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Placed,
            RuleSource::Bible => Self::Swept,
        }
    }
}

/// Which test `_ShipCollisionHandler` puts a ship to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// `_SWRadiusCollision`: the rects' centres and half widths.
    Radius,
    /// `_SWRLECollision`: the pixel masks.
    Mask,
}

/// The test a ship whose rect is `width` wide is put to at a speed
/// multiplier of `speed_mult` (see the module docs).
#[must_use]
pub fn shape(speed_mult: f64, width: i64) -> Shape {
    if speed_mult >= RADIUS_SPEED_MULT || width <= RADIUS_WIDTH {
        Shape::Radius
    } else {
        Shape::Mask
    }
}

/// A ship's sprite as its last display drew it: the sheet its sprite base
/// draws and the frame it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipSprite {
    /// Its sheet.
    pub sheet: Rc<RleSheet>,
    /// The frame it shows.
    pub frame: u16,
}

/// What the session lends the contact passes (see the module docs): each
/// ship's sprite and each shot's and asteroid's sheet it holds, the pass's
/// speed multiplier and the readings of the two rules. A ship, weapon
/// graphic or asteroid kind not here has no mask.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactSprites {
    /// Each ship's sprite whose sheet was read.
    pub ships: Vec<(ShipRef, ShipSprite)>,
    /// Each weapon `Graphic`'s sheet that was read.
    pub shots: BTreeMap<i16, Rc<RleSheet>>,
    /// Each asteroid kind's sheet, where it was read.
    pub asteroids: [Option<Rc<RleSheet>>; KINDS],
    /// The pass's speed multiplier, `_gSpeedMult`.
    pub speed_mult: f64,
    /// How the masks are walked, by
    /// [`RuleKey::RleCollisionWalk`](crate::RuleKey::RleCollisionWalk).
    pub walk: MaskWalk,
    /// How a shot touches, by [`RuleKey::ShotContact`](crate::RuleKey::ShotContact).
    pub reading: ShotContact,
}

impl Default for ContactSprites {
    /// No masks at the engine's speed, by the default readings.
    fn default() -> Self {
        Self {
            ships: Vec::new(),
            shots: BTreeMap::new(),
            asteroids: std::array::from_fn(|_| None),
            speed_mult: SpeedMult::FULL.get(),
            walk: MaskWalk::default(),
            reading: ShotContact::default(),
        }
    }
}

/// One sprite as the contact lays it out: its rect, and its mask where its
/// sheet was read.
#[derive(Clone, Debug, PartialEq)]
struct Laid {
    rect: MaskRect,
    mask: Option<PlacedMask>,
}

/// A whole size of `side` pixels, none below none.
fn whole(side: f32) -> u32 {
    // A sprite's side, far within a `u32`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = side.max(0.0) as u32;
    whole
}

impl ContactSprites {
    /// `ship` of `hull` laid out `offset` from the player (see the module
    /// docs): at its sheet's size with its mask, else at its `shän`
    /// sizes with none; none without either.
    fn ship(&self, ship: ShipRef, hull: &HullSpec, offset: Vec2) -> Option<Laid> {
        if let Some((_, sprite)) = self.ships.iter().find(|(id, _)| *id == ship) {
            let rect = Placement::Standard.rect(offset, (sprite.sheet.width, sprite.sheet.height));
            let mask = PlacedMask {
                sheet: Rc::clone(&sprite.sheet),
                frame: sprite.frame,
                rect,
            };
            return Some(Laid {
                rect,
                mask: Some(mask),
            });
        }
        let size = (whole(hull.sprite_size?), whole(hull.sprite_height?));
        Some(Laid {
            rect: Placement::Standard.rect(offset, size),
            mask: None,
        })
    }

    /// `shot` laid out `offset` from the player by the shot rule: at its
    /// sheet's size with its mask at [`Shot::frame_on`], else at its weapon's
    /// frame size with none; none without either.
    fn shot(&self, shot: &Shot, offset: Vec2) -> Option<Laid> {
        let read = self
            .shots
            .get(&shot.weapon.graphic)
            .and_then(|sheet| Some((sheet, shot.frame_on(sheet)?)));
        if let Some((sheet, frame)) = read {
            let rect = Placement::Shot.rect(offset, (sheet.width, sheet.height));
            let mask = PlacedMask {
                sheet: Rc::clone(sheet),
                frame,
                rect,
            };
            return Some(Laid {
                rect,
                mask: Some(mask),
            });
        }
        let frame = shot.weapon.frame?;
        Some(Laid {
            rect: Placement::Shot.rect(offset, (frame.width, frame.height)),
            mask: None,
        })
    }

    /// An asteroid of `kind` showing `frame`, `offset` from the player,
    /// by the standard rule at its sheet's size; none when its sheet
    /// wasn't read.
    fn asteroid(&self, kind: u8, frame: u16, offset: Vec2) -> Option<PlacedMask> {
        let sheet = self.asteroids.get(usize::from(kind))?.as_ref()?;
        Some(PlacedMask {
            sheet: Rc::clone(sheet),
            frame,
            rect: Placement::Standard.rect(offset, (sheet.width, sheet.height)),
        })
    }
}

/// Whether the ship laid out as `ship` touches the shot laid out as
/// `shot` (see the module docs), at `speed_mult`, walked by `walk`; none
/// where the shape test needs a mask either lacks.
fn ship_meets(ship: &Laid, shot: &Laid, speed_mult: f64, walk: MaskWalk) -> Option<bool> {
    let shape = shape(speed_mult, ship.rect.width());
    let masks = ship.mask.as_ref().zip(shot.mask.as_ref());
    if shape == Shape::Mask && masks.is_none() {
        return None;
    }
    if !ship.rect.overlaps(shot.rect) {
        return Some(false);
    }
    match (shape, masks) {
        (Shape::Radius, _) => Some(radius_touch(ship.rect, shot.rect)),
        (Shape::Mask, Some((ship, shot))) => ship.touches(shot, walk),
        (Shape::Mask, None) => None,
    }
}

impl Combat {
    /// Where `shot`, its last move from `from`, touches the ship `target`
    /// of `hull`, if it does, as `sprites` lend the masks and readings and
    /// the player was seen at `camera` (see the module docs): the shot's
    /// position by the placed rects, or where its swept move entered the
    /// ship's radius by the other reading or the fallback. None for a shot
    /// that touches no ship ([`Shot::touches_ships`]).
    pub(super) fn ship_contact(
        &self,
        (shot, from): (&Shot, Vec2),
        (target, hull): (&Target, &HullSpec),
        (sprites, camera): (&ContactSprites, Vec2),
    ) -> Option<Vec2> {
        if !shot.touches_ships() {
            return None;
        }
        let back = self.left_back(shot);
        if sprites.reading == ShotContact::Placed {
            let ship = sprites.ship(target.ship, hull, target.position - camera);
            let laid = ship.zip(sprites.shot(shot, shot.position + back - camera));
            let touch = laid.and_then(|(ship, laid)| {
                ship_meets(&ship, &laid, sprites.speed_mult, sprites.walk)
            });
            if let Some(touch) = touch {
                return touch.then_some(shot.position);
            }
        }
        let met = Target {
            position: target.position - back,
            ..*target
        };
        shot.touches(from, &met)
    }

    /// Where `shot`, its last move from `from`, touches `rock`, of `kind`
    /// showing `frame`, where the walk meets it, if it does, as `sprites`
    /// lend the masks and readings and the player was seen at `camera`
    /// (see the module docs): by the masks, the asteroid's first, at the
    /// shot's position; else, by the other reading or the fallback, where
    /// its swept move entered the asteroid's radius. None for a weapon
    /// that ignores asteroids.
    pub(super) fn asteroid_contact(
        &self,
        (shot, from): (&Shot, Vec2),
        (rock, kind, frame): (&Rock, u8, u16),
        (sprites, camera): (&ContactSprites, Vec2),
    ) -> Option<Vec2> {
        if shot.weapon.ignores_asteroids() {
            return None;
        }
        let back = self.left_back(shot);
        if sprites.reading == ShotContact::Placed {
            let asteroid = sprites.asteroid(kind, frame, rock.position - camera);
            let laid = sprites
                .shot(shot, shot.position + back - camera)
                .and_then(|laid| laid.mask);
            if let Some((asteroid, laid)) = asteroid.zip(laid) {
                let touch = if asteroid.rect.overlaps(laid.rect) {
                    asteroid.touches(&laid, sprites.walk)
                } else {
                    Some(false)
                };
                if let Some(touch) = touch {
                    return touch.then_some(shot.position);
                }
            }
        }
        let met = Rock {
            position: rock.position - back,
            ..*rock
        };
        asteroid_hit::touches(shot, from, &met)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::rle::sheet;

    #[test]
    fn a_ship_32_wide_or_less_or_at_twice_the_speed_goes_by_radius() {
        assert_eq!(shape(1.0, 32), Shape::Radius);
        assert_eq!(shape(1.0, 33), Shape::Mask);
        assert_eq!(shape(2.0, 33), Shape::Radius, "at 2.0, inclusive");
        assert_eq!(shape(1.99, 33), Shape::Mask);
        assert_eq!(shape(3.0, 400), Shape::Radius);
        assert_eq!(shape(1.0, -1), Shape::Radius);
    }

    #[test]
    fn the_readings_follow_the_rule_source() {
        assert_eq!(ShotContact::from(RuleSource::Engine), ShotContact::Placed);
        assert_eq!(ShotContact::from(RuleSource::Bible), ShotContact::Swept);
        assert_eq!(ShotContact::default(), ShotContact::Placed);
        let lent = ContactSprites::default();
        assert!((lent.speed_mult - SpeedMult::FULL.get()).abs() < f64::EPSILON);
        assert!(lent.asteroids.iter().all(Option::is_none));
    }

    /// A sheet of one frame, its rows `rows`.
    fn one(rows: &[&str]) -> Rc<RleSheet> {
        Rc::new(sheet(&[rows]))
    }

    /// `rows` as a laid-out sprite with its top left at `at`.
    fn laid(rows: &[&str], at: (i32, i32)) -> Laid {
        let sheet = one(rows);
        let rect = MaskRect::placed(at, (sheet.width, sheet.height));
        Laid {
            rect,
            mask: Some(PlacedMask {
                sheet,
                frame: 0,
                rect,
            }),
        }
    }

    /// A row `width` wide drawn only at the pixels `drawn` says.
    fn row(width: usize, drawn: impl Fn(usize) -> bool) -> String {
        (0..width)
            .map(|x| if drawn(x) { '#' } else { '.' })
            .collect()
    }

    #[test]
    fn the_ships_last_pixel_left_of_the_shots_first_touches_and_not_the_mirror() {
        let wide = row(40, |x| x == 9);
        let ship = laid(&[wide.as_str()], (0, 0));
        let shot = laid(&[".#", ".."], (9, 0));
        // The ship's pixel at 9, the shot's at 10.
        assert_eq!(
            ship_meets(&ship, &shot, 1.0, MaskWalk::Original),
            Some(true)
        );
        assert_eq!(ship_meets(&ship, &shot, 1.0, MaskWalk::Drawn), Some(false));
        let mirrored = row(40, |x| x == 10);
        let ship = laid(&[mirrored.as_str()], (0, 0));
        let shot = laid(&["#.", ".."], (9, 0));
        assert_eq!(
            ship_meets(&ship, &shot, 1.0, MaskWalk::Original),
            Some(false)
        );
    }

    #[test]
    fn a_shot_over_a_ships_run_misses_by_the_original_walk() {
        let run = row(40, |_| true).replace('#', "r");
        let ship = laid(&[run.as_str()], (0, 0));
        let shot = laid(&["#"], (20, 0));
        assert_eq!(
            ship_meets(&ship, &shot, 1.0, MaskWalk::Original),
            Some(false)
        );
        assert_eq!(ship_meets(&ship, &shot, 1.0, MaskWalk::Drawn), Some(true));
    }

    #[test]
    fn a_mask_test_without_a_mask_cannot_tell_and_a_radius_one_needs_none() {
        let wide = row(40, |_| true);
        let mut ship = laid(&[wide.as_str(); 40], (0, 0));
        let shot = laid(&["#"], (20, 20));
        ship.mask = None;
        assert_eq!(ship_meets(&ship, &shot, 1.0, MaskWalk::Original), None);
        assert_eq!(
            ship_meets(&ship, &shot, 2.0, MaskWalk::Original),
            Some(true),
            "by radius at 2.0"
        );
        let far = laid(&["#"], (90, 0));
        assert_eq!(ship_meets(&ship, &far, 1.0, MaskWalk::Original), None);
        assert_eq!(
            ship_meets(&ship, &far, 2.0, MaskWalk::Original),
            Some(false),
            "the rects apart"
        );
    }

    #[test]
    fn a_short_wide_ship_is_missed_outside_its_rect_though_within_its_radius() {
        // 40 by 10, centred by radius at (20, 20), below its own rect.
        let ship = Laid {
            rect: MaskRect::placed((0, 0), (40, 10)),
            mask: None,
        };
        let below = Laid {
            rect: MaskRect::placed((18, 12), (4, 4)),
            mask: None,
        };
        assert!(radius_touch(ship.rect, below.rect), "within the radius");
        assert_eq!(
            ship_meets(&ship, &below, 2.0, MaskWalk::Original),
            Some(false),
            "outside its rect"
        );
        let inside = Laid {
            rect: MaskRect::placed((18, 7), (4, 4)),
            mask: None,
        };
        assert_eq!(
            ship_meets(&ship, &inside, 2.0, MaskWalk::Original),
            Some(true)
        );
    }

    #[test]
    fn a_ship_with_a_mask_goes_by_radius_where_its_shape_says_against_its_pixels() {
        let clear = |side| laid(&vec![row(side, |_| false).as_str(); side], (0, 0));
        let solid = |side| laid(&vec![row(side, |_| true).as_str(); side], (0, 0));
        let dot = |at| laid(&["#"], at);
        let walk = MaskWalk::Original;
        // 32 wide: by radius at the centre over clear pixels, and not by
        // its drawn corner, 22.6 from its centre, past 16.
        assert_eq!(
            ship_meets(&clear(32), &dot((16, 16)), 1.0, walk),
            Some(true)
        );
        let (ship, corner) = (solid(32), dot((0, 0)));
        let masks = ship.mask.as_ref().zip(corner.mask.as_ref());
        assert_eq!(masks.and_then(|(a, b)| a.touches(b, walk)), Some(true));
        assert_eq!(ship_meets(&ship, &corner, 1.0, walk), Some(false));
        // 40 wide: by its pixels at 1.0, by radius at 2.0.
        assert_eq!(
            ship_meets(&clear(40), &dot((20, 20)), 1.0, walk),
            Some(false)
        );
        assert_eq!(
            ship_meets(&clear(40), &dot((20, 20)), 2.0, walk),
            Some(true)
        );
        assert_eq!(ship_meets(&solid(40), &dot((0, 0)), 1.0, walk), Some(true));
        assert_eq!(ship_meets(&solid(40), &dot((0, 0)), 2.0, walk), Some(false));
    }

    #[test]
    fn a_ship_is_laid_out_at_its_sheets_size_or_its_shan_sizes() {
        let sheet = one(&["###", "###"]);
        let mut sprites = ContactSprites::default();
        sprites.ships.push((
            ShipRef::Player,
            ShipSprite {
                sheet: Rc::clone(&sheet),
                frame: 0,
            },
        ));
        let hull = HullSpec {
            sprite_size: Some(10.0),
            sprite_height: Some(6.0),
            ..HullSpec::default()
        };
        let at = Vec2::new(10.5, -4.5);
        let own = sprites.ship(ShipRef::Player, &hull, at).expect("its sheet");
        assert_eq!(own.rect, MaskRect::placed((9, -5), (3, 2)));
        assert_eq!(own.mask.expect("a mask").rect, own.rect);
        let npc = ShipRef::Npc(super::super::NpcId(1));
        let shan = sprites.ship(npc, &hull, at).expect("its shän sizes");
        assert_eq!(shan.rect, MaskRect::placed((5, -7), (10, 6)));
        assert!(shan.mask.is_none());
        assert!(sprites.ship(npc, &HullSpec::default(), at).is_none());
    }

    #[test]
    fn a_shot_is_laid_out_by_the_shot_rule_at_its_sheets_frame_or_its_weapons_size() {
        use super::super::weapon::WeaponSpec;
        use crate::catalog::FrameSize;
        let mut spec = WeaponSpec::new(&nova_sim_catalog::fixture::weapon(128));
        spec.graphic = 7;
        spec.frame = Some(FrameSize {
            width: 4,
            height: 8,
        });
        let state = crate::flight::ShipState::default();
        let shot = Shot::launch(spec, ShipRef::Player, ShipRef::Player, &state, 180.0);
        let at = Vec2::new(10.5, 0.5);
        let mut sprites = ContactSprites::default();
        let sized = sprites.shot(&shot, at).expect("its weapon's size");
        assert_eq!(sized.rect, MaskRect::placed((8, -2), (4, 8)));
        assert!(sized.mask.is_none());
        let tall = Rc::new(sheet(&[&["#.", "#.", "#."], &[".#", ".#", ".#"]]));
        sprites.shots.insert(7, Rc::clone(&tall));
        let read = sprites.shot(&shot, at).expect("its sheet");
        assert_eq!(
            read.rect,
            MaskRect::placed((9, -1), (2, 3)),
            "half its width up"
        );
        assert_eq!(read.mask.expect("a mask").frame, 1, "heading 180's of 2");
        let no_frame = Shot {
            weapon: WeaponSpec {
                frame: None,
                ..spec
            },
            ..shot
        };
        sprites.shots.clear();
        assert!(sprites.shot(&no_frame, at).is_none());
    }

    #[test]
    fn an_asteroid_is_laid_out_by_the_standard_rule_at_its_sheet_and_frame() {
        let mut sprites = ContactSprites::default();
        assert!(sprites.asteroid(2, 0, Vec2::ZERO).is_none());
        sprites.asteroids[2] = Some(one(&["###", "###", "###"]));
        let mask = sprites.asteroid(2, 5, Vec2::new(-4.9, 7.2)).expect("read");
        assert_eq!(
            (mask.frame, mask.rect),
            (5, MaskRect::placed((-5, 6), (3, 3)))
        );
        assert!(sprites.asteroid(u8::MAX, 0, Vec2::ZERO).is_none());
    }
}
