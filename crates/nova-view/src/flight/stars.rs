//! The flight's starfield as the original builds it: 20 star sprites
//! scattered over the play area about the ship by `_RandomizeStars`
//! (@0x0b145-0x0b324), wrapped and placed every pass by `_HandleStars`
//! (@0x2f87d-0x2fcae), and moved by their parallax once a frame by
//! `_ScrollStarfield` (@0x8171-0x81d5). The flight view drives it
//! (`flight::view`'s `stars` module); this model is pure, its inputs plain
//! values and a [`Chance`].
//!
//! - **The slots.** `_starData` is [`STAR_SLOTS`] slots of 0x14 bytes
//!   (`_InitArrays` @0x1bb95, `NewPtr(0x190)`): the slot's sprite (+0), shown
//!   (+4), frame (+6), parallax factor (+8, a float) and x and y in world
//!   units (+0xc, +0x10, floats). `_InitObjects` (@0x1ce0d-0x1ce39) leaves
//!   every slot hidden at frame 0 with factor 1.0 ([`Starfield::new`]).
//! - **Scattering** ([`Starfield::randomize`]). In a system whose `Murk`
//!   as loaded is negative, every slot is hidden. Otherwise min(trunc(H /
//!   600 × 20), 20) slots ([`star_count`]) are each shown at a random
//!   frame, at the ship's position plus `Rand(W)` − trunc(W / 2) and
//!   `Rand(H)` − trunc(H / 2), (W, H) the play area's size, with a factor
//!   of `Rand(35)` × 0.01 when the Parallax Starfield preference is on and
//!   0 when off. The slots past the count are shown as they were.
//! - **Handling** ([`Starfield::handle`]). The stars are dimmed by the
//!   system's murk ([`murk_level`]). A shown star whose last-placed centre
//!   is more than twice its size outside the play area is moved W (or H)
//!   plus twice its size to the other side, its other axis drawn afresh
//!   about the ship; then it is placed at trunc(its position − the ship's)
//!   plus the anchor (`_screenCenter`) less its whole size.
//! - **Scrolling** ([`Starfield::scroll`]). Each shown star of factor above
//!   0 moves back by its factor of the ship's move.
//! - **Drawing** ([`Starfield::draw`]). Each shown star is its frame at the
//!   placement taken after the frame's first pass ([`Starfield::snapshot`]),
//!   OR-ed onto what is behind it at the murk's level, as
//!   `_BlitPixieRLEAddOverClipped` (@0xc25e7, `dest |= src` at full level,
//!   @0xc2790-0xc27c8) and `_BlitPixieRLETranslucentClipped` (@0xc18c0,
//!   `(src × level) >> 5 | dest` below it, through
//!   `_BlitPixieTranslucentCopy` @0xc11cc-0xc1275) draw it at 16 bits:
//!   [`Blend::Or`](crate::Blend::Or) at [`lights_tint`]`(level)`.
//!
//! The draws are made on the flight's effects chance, never the
//! simulation's: the stars are view-only. Their ranges and conditions are
//! the original's; their order is not matched.
//!
//! Left out:
//!
//! - **Other depths** (`_HandleStars` @0x2fa08-0x2fa70, the 8-bit
//!   translucency): the engine draws at 16 bits only.
//! - **Slots past the count on their first use** hold uninitialised x and
//!   y (`NewPtr`, never written), which can't be replicated; they start at
//!   (0, 0), frame 0, factor 1.0, as `_InitObjects` leaves the rest. Only
//!   a play area under 600 high reaches them.
//! - **The 16-bit positions and `Rand` range**: the engine uses wide
//!   numbers.

use nova_sim::Vec2;
use nova_sim::chance::Chance;

use crate::draw::lights_tint;
use crate::flight::catalog::StarSheet;
use crate::{DrawList, ImageKey, Point};

/// How many star slots there are (`_InitArrays` @0x1bb95, 0x190 bytes of
/// 0x14-byte slots).
pub const STAR_SLOTS: usize = 20;

/// The stars a [`PER_HEIGHT`]-high play area holds (the double @0xdd148).
pub const STARS_PER: f64 = 20.0;

/// The play area's height that holds [`STARS_PER`] stars (the double
/// @0xdd140).
pub const PER_HEIGHT: f64 = 600.0;

/// The sides of a star's parallax roll (`Rand(35)` @0x0b2be-0x0b2fa).
pub const PARALLAX_SIDES: u16 = 35;

/// What a parallax roll is multiplied by (the double @0xdd098).
pub const PARALLAX_UNIT: f64 = 0.01;

/// What a murky system's murk is multiplied by for the stars' level (the
/// double @0xdd5a8).
pub const MURK_LEVEL_SCALE: f64 = 0.9;

/// The highest level a murky system's stars are drawn at, out of 32
/// (@0x2f8cc-0x2f913).
pub const MURK_LEVEL_MAX: u8 = 29;

/// The level a murky system's stars are drawn at when the scaled murk is
/// at most 1 (@0x2f8cc-0x2f913).
pub const MURK_LEVEL_MIN: u8 = 2;

/// The level of a system without murk: the stars as they are.
pub const FULL_LEVEL: u8 = 32;

/// The size `_GetSpriteXSize` and `_GetSpriteYSize` give a sprite without
/// a frame (@0x26d4-0x2701).
pub const NO_FRAME_SIZE: i32 = 32;

/// How many stars a play area `height` pixels high holds: trunc(height /
/// 600 × 20) (`_RandomizeStars` @0x0b1c0-0x0b1de), at most the
/// [`STAR_SLOTS`] there are.
#[must_use]
pub fn star_count(height: i32) -> usize {
    let count = (f64::from(height) / PER_HEIGHT * STARS_PER).trunc();
    // Clamped to 0..=20 before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = count.clamp(0.0, STAR_SLOTS as f64) as usize;
    count
}

/// The level, out of 32, a system of `Murk` `murk` as loaded draws its
/// stars at (`_HandleStars` @0x2f8cc-0x2f913): [`FULL_LEVEL`] without
/// murk; otherwise trunc(murk × 0.9), [`MURK_LEVEL_MAX`] above it and
/// [`MURK_LEVEL_MIN`] at 1 or below.
#[must_use]
pub fn murk_level(murk: i16) -> u8 {
    if murk == 0 {
        return FULL_LEVEL;
    }
    let level = (f64::from(murk) * MURK_LEVEL_SCALE).trunc();
    if level > f64::from(MURK_LEVEL_MAX) {
        MURK_LEVEL_MAX
    } else if level <= 1.0 {
        MURK_LEVEL_MIN
    } else {
        // Between 2 and 29 here.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let level = level as u8;
        level
    }
}

/// What the stars are drawn with: the master sprite's frame count and each
/// frame's size in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarLook {
    /// How many frames a star can show.
    pub frames: u16,
    /// Each frame's width and height.
    pub size: (i32, i32),
}

impl StarLook {
    /// The look of `sheet`, or, without one, no frames at
    /// [`NO_FRAME_SIZE`], as the original's sizes read a sprite without a
    /// frame.
    #[must_use]
    pub fn of(sheet: Option<&StarSheet>) -> Self {
        sheet.map_or(
            Self {
                frames: 0,
                size: (NO_FRAME_SIZE, NO_FRAME_SIZE),
            },
            |sheet| {
                let side = |pixels: u32| i32::try_from(pixels).unwrap_or(i32::MAX);
                Self {
                    frames: sheet.frames.get(),
                    size: (side(sheet.size.0), side(sheet.size.1)),
                }
            },
        )
    }
}

/// Where the stars are handled: the play area, the ship, the anchor, the
/// system's murk and the stars' look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surroundings {
    /// The play area's width and height, (W, H): the sprite world,
    /// (0, 0)-(W − 194, H) of the view (`_SetupSpriteWorld`
    /// @0x21b54-0x21ba4).
    pub area: (i32, i32),
    /// The player's ship's position in world units (`_shipData`
    /// +0x18/+0x1c).
    pub ship: Vec2,
    /// The screen point the ship is drawn at (`_screenCenter`), in whole
    /// pixels.
    pub anchor: (i32, i32),
    /// The player's system's `Murk` as loaded (its runtime +0xbc).
    pub murk: i16,
    /// The stars' look.
    pub look: StarLook,
}

/// One star slot's state (`_starData`'s 0x14 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    /// Whether it is shown (+4).
    pub shown: bool,
    /// Its frame (+6).
    pub frame: u16,
    /// Its parallax factor (+8): how far it moves back for each unit the
    /// ship moves.
    pub factor: f32,
    /// Its x in world units (+0xc).
    pub x: f32,
    /// Its y in world units (+0x10).
    pub y: f32,
}

impl Default for Star {
    /// Hidden, at frame 0 with factor 1.0, as `_InitObjects` leaves it
    /// (@0x1ce0d-0x1ce39); at (0, 0), which it never writes.
    fn default() -> Self {
        Self {
            shown: false,
            frame: 0,
            factor: 1.0,
            x: 0.0,
            y: 0.0,
        }
    }
}

/// A star as a frame draws it: its frame and the top-left it moves from
/// and to over the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drawn {
    frame: u16,
    from: (i32, i32),
    to: (i32, i32),
}

/// One slot: its star, where its sprite was last placed and whether it
/// jumped since the last snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Slot {
    star: Star,
    /// The sprite's top-left as last placed, in play-area pixels. The
    /// clone starts with its master's rect (`SWCloneSprite` @0xa9419,
    /// `_InitSprites` @0x1fd86), which is never placed: (0, 0).
    placed: (i32, i32),
    /// Whether it was scattered or wrapped since the last snapshot, so the
    /// next draws it where it is, not on its way.
    jumped: bool,
    /// What the frame draws of it, if anything.
    drawn: Option<Drawn>,
}

/// The 20 star slots and what the frame draws of them (see the module
/// docs).
#[derive(Clone, Debug, PartialEq)]
pub struct Starfield {
    slots: [Slot; STAR_SLOTS],
    /// The level the stars were last handled at, out of 32.
    level: u8,
    /// The level the frame draws them at.
    drawn_level: u8,
}

impl Default for Starfield {
    fn default() -> Self {
        Self::new()
    }
}

impl Starfield {
    /// Every slot hidden at frame 0 with factor 1.0, as `_InitObjects`
    /// leaves them, nothing drawn.
    #[must_use]
    pub fn new() -> Self {
        Self {
            slots: [Slot::default(); STAR_SLOTS],
            level: FULL_LEVEL,
            drawn_level: FULL_LEVEL,
        }
    }

    /// Each slot's star.
    #[must_use]
    pub fn stars(&self) -> [Star; STAR_SLOTS] {
        self.slots.map(|slot| slot.star)
    }

    /// Each slot's sprite's top-left as last placed, in play-area pixels.
    #[must_use]
    pub fn placements(&self) -> [(i32, i32); STAR_SLOTS] {
        self.slots.map(|slot| slot.placed)
    }

    /// Scatters the stars about the ship, as `_RandomizeStars` does
    /// (@0x0b145-0x0b324; see the module docs), `parallax` the Parallax
    /// Starfield preference (`_settings` +0xb), drawing on `chance`. A
    /// negative murk hides every slot and draws nothing (@0x0b168-0x0b18d).
    pub fn randomize(&mut self, at: &Surroundings, parallax: bool, chance: &mut dyn Chance) {
        if at.murk < 0 {
            self.clear();
            return;
        }
        let (width, height) = at.area;
        let count = star_count(height);
        for (index, slot) in self.slots.iter_mut().enumerate() {
            slot.star.shown = true;
            if index >= count {
                // Shown as it was (@0x0b304-0x0b30b).
                continue;
            }
            slot.star.frame = chance.roll(at.look.frames);
            slot.star.x = scattered(at.ship.x, width, chance);
            slot.star.y = scattered(at.ship.y, height, chance);
            slot.star.factor = if parallax {
                // At most 0.34: exact enough in an f32.
                #[allow(clippy::cast_possible_truncation)]
                let factor = (f64::from(chance.roll(PARALLAX_SIDES)) * PARALLAX_UNIT) as f32;
                factor
            } else {
                0.0
            };
            slot.jumped = true;
        }
    }

    /// Marks every slot as moved since the last snapshot, so the next
    /// draws it where it is: the player's wrap moves the ship the stars are
    /// placed from, though no star moves, and the original draws no path
    /// between placements.
    pub fn mark_jumped(&mut self) {
        for slot in &mut self.slots {
            slot.jumped = true;
        }
    }

    /// Hides every slot, as `_ClearStarfield` does (@0x8152-0x8170).
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.star.shown = false;
            slot.jumped = true;
        }
    }

    /// Wraps and places each shown star, as `_HandleStars` does
    /// (@0x2f87d-0x2fcae; see the module docs), drawing a wrapped star's
    /// other axis on `chance`.
    pub fn handle(&mut self, at: &Surroundings, chance: &mut dyn Chance) {
        self.level = murk_level(at.murk);
        let (width, height) = at.area;
        let (sx, sy) = at.look.size;
        for slot in self.slots.iter_mut().filter(|slot| slot.star.shown) {
            // The centre as last placed, read before either wrap moves it
            // (@0x2faed-0x2fc29).
            let cx = slot.placed.0 + sx / 2;
            let cy = slot.placed.1 + sy / 2;
            let star = &mut slot.star;
            if let Some(shift) = wrap_shift(cx, width, sx) {
                star.x += shift;
                star.y = scattered(at.ship.y, height, chance);
                slot.jumped = true;
            }
            if let Some(shift) = wrap_shift(cy, height, sy) {
                star.y += shift;
                star.x = scattered(at.ship.x, width, chance);
                slot.jumped = true;
            }
            // `cvttss2si` of the offset, then the anchor less the whole
            // size (@0x2fc2e-0x2fc7f).
            #[allow(clippy::cast_possible_truncation)]
            let offset = |star: f32, ship: f32| (star - ship).trunc() as i32;
            slot.placed = (
                offset(star.x, at.ship.x) + at.anchor.0 - sx,
                offset(star.y, at.ship.y) + at.anchor.1 - sy,
            );
        }
    }

    /// Moves each shown star of factor above 0 back by its factor of
    /// (`dx`, `dy`), as `_ScrollStarfield(dx, dy)` does (@0x8171-0x81d5).
    pub fn scroll(&mut self, dx: f32, dy: f32) {
        for slot in &mut self.slots {
            let star = &mut slot.star;
            if star.shown && star.factor > 0.0 {
                star.x -= star.factor * dx;
                star.y -= star.factor * dy;
            }
        }
    }

    /// Takes what the frame draws: each shown star at its placement now,
    /// moving there from the last snapshot's unless it was scattered or
    /// wrapped since, and the level the stars were last handled at.
    ///
    /// The movement between snapshots is an engine smoothing, as the
    /// ships' positions are drawn between ticks; the original places its
    /// sprites once a frame. It crosses no boundary the player can keep.
    pub fn snapshot(&mut self) {
        for slot in &mut self.slots {
            slot.drawn = slot.star.shown.then(|| {
                let from = match slot.drawn {
                    Some(drawn) if !slot.jumped => drawn.to,
                    _ => slot.placed,
                };
                Drawn {
                    frame: slot.star.frame,
                    from,
                    to: slot.placed,
                }
            });
            slot.jumped = false;
        }
        self.drawn_level = self.level;
    }

    /// Draws each star of the last snapshot `alpha` of the way from where
    /// it was to where it is, its frame of `sheet` centred on its sprite's
    /// rect, OR-ed onto what is behind it at the murk's level (see the
    /// module docs).
    pub fn draw(&self, list: &mut DrawList, sheet: &StarSheet, alpha: f32) {
        let tint = lights_tint(self.drawn_level);
        #[allow(clippy::cast_precision_loss)]
        let (half_w, half_h) = (sheet.size.0 as f32 / 2.0, sheet.size.1 as f32 / 2.0);
        for drawn in self.slots.iter().filter_map(|slot| slot.drawn) {
            #[allow(clippy::cast_precision_loss)]
            let between = |from: i32, to: i32| (to - from) as f32 * alpha + from as f32;
            let left = between(drawn.from.0, drawn.to.0);
            let top = between(drawn.from.1, drawn.to.1);
            list.or_sprite(
                ImageKey::sprite(sheet.image_id, drawn.frame),
                Point::new(left + half_w, top + half_h),
                tint,
            );
        }
    }
}

/// A position `ship` plus `Rand(extent)` − trunc(extent / 2): a star
/// scattered over an axis of the play area `extent` pixels long about the
/// ship (@0x0b210-0x0b317).
fn scattered(ship: f32, extent: i32, chance: &mut dyn Chance) -> f32 {
    let roll = u32::try_from(extent)
        .ok()
        .filter(|&sides| sides > 0)
        .map_or(0, |sides| chance.below(sides));
    // Both within a play area's pixels: exact in an f32.
    #[allow(clippy::cast_precision_loss)]
    let offset = roll as f32 - (extent / 2) as f32;
    ship + offset
}

/// How far a star whose centre is at `centre` on an axis `extent` long,
/// its sprite `size` along it, moves to wrap: `extent` + 2 × `size` on
/// when the centre is more than 2 × `size` before the axis, as far back
/// when it is more than 2 × `size` past it, and none otherwise
/// (`_HandleStars` @0x2faed-0x2fc29).
fn wrap_shift(centre: i32, extent: i32, size: i32) -> Option<f32> {
    let margin = 2 * size;
    // A play area's pixels: exact in an f32.
    #[allow(clippy::cast_precision_loss)]
    let shift = (extent + margin) as f32;
    if centre < -margin {
        Some(shift)
    } else if centre > extent + margin {
        Some(-shift)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::VecDeque;
    use std::num::NonZeroU16;

    use super::*;
    use crate::{Blend, DrawCommand};

    /// What a draw asked for: a `Rand(sides)` roll or a wide draw below
    /// `n`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Asked {
        Roll(u16),
        Below(u32),
    }

    /// Gives its values in turn, then 0s, and records what each draw
    /// asked for.
    #[derive(Default)]
    struct Scripted {
        values: VecDeque<u32>,
        asked: Vec<Asked>,
    }

    impl Scripted {
        fn giving(values: &[u32]) -> Self {
            Self {
                values: values.iter().copied().collect(),
                asked: Vec::new(),
            }
        }

        fn next(&mut self) -> u32 {
            self.values.pop_front().unwrap_or(0)
        }
    }

    impl Chance for Scripted {
        fn fires(&mut self, _percent: u8) -> bool {
            panic!("the stars fire no chance")
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(Asked::Below(n));
            self.next()
        }

        fn roll(&mut self, sides: u16) -> u16 {
            self.asked.push(Asked::Roll(sides));
            u16::try_from(self.next()).expect("a roll")
        }
    }

    /// Stock's look: 16 frames of 5 × 5.
    const LOOK: StarLook = StarLook {
        frames: 16,
        size: (5, 5),
    };

    /// A 830 × 768 play area, the ship at (1000, -500), drawn at
    /// (415, 384), no murk.
    fn surroundings() -> Surroundings {
        Surroundings {
            area: (830, 768),
            ship: Vec2::new(1000.0, -500.0),
            anchor: (415, 384),
            murk: 0,
            look: LOOK,
        }
    }

    fn sheet() -> StarSheet {
        StarSheet {
            image_id: 700,
            frames: NonZeroU16::new(16).expect("non-zero"),
            size: (5, 5),
        }
    }

    /// A field whose slot 0 is shown at (`x`, `y`) with factor `factor`
    /// and its sprite last placed at `placed`; the others hidden.
    fn one_star(x: f32, y: f32, factor: f32, placed: (i32, i32)) -> Starfield {
        let mut field = Starfield::new();
        field.slots[0] = Slot {
            star: Star {
                shown: true,
                frame: 3,
                factor,
                x,
                y,
            },
            placed,
            ..Slot::default()
        };
        field
    }

    #[test]
    fn a_new_field_is_20_hidden_slots_at_frame_0_with_factor_1() {
        let field = Starfield::new();
        assert_eq!(field.stars().len(), 20);
        for star in field.stars() {
            assert_eq!(
                star,
                Star {
                    shown: false,
                    frame: 0,
                    factor: 1.0,
                    x: 0.0,
                    y: 0.0,
                }
            );
        }
        assert_eq!(Starfield::default(), field);
    }

    #[test]
    fn the_count_is_the_height_over_600_times_20_at_most_the_20_slots() {
        assert_eq!(star_count(768), 20, "25, held to the 20 slots");
        assert_eq!(star_count(600), 20);
        assert_eq!(star_count(599), 19, "19.97, truncated");
        assert_eq!(star_count(480), 16);
        assert_eq!(star_count(29), 0);
        assert_eq!(star_count(30), 1);
        assert_eq!(star_count(0), 0);
        assert_eq!(star_count(-5), 0);
    }

    #[test]
    fn a_negative_murk_hides_every_star_and_draws_nothing() {
        let mut field = one_star(1.0, 2.0, 0.1, (0, 0));
        let mut chance = Scripted::default();
        let at = Surroundings {
            murk: -1,
            ..surroundings()
        };
        field.randomize(&at, true, &mut chance);
        assert!(field.stars().iter().all(|star| !star.shown));
        assert_eq!(chance.asked, []);
    }

    #[test]
    fn each_star_rolls_its_frame_its_place_and_its_parallax() {
        let mut field = Starfield::new();
        let mut chance = Scripted::giving(&[7, 100, 700, 34]);
        field.randomize(&surroundings(), true, &mut chance);
        assert_eq!(
            chance.asked[..4],
            [
                Asked::Roll(16),
                Asked::Below(830),
                Asked::Below(768),
                Asked::Roll(35)
            ]
        );
        assert_eq!(chance.asked.len(), 4 * 20, "all 20 slots at 768");
        let first = field.stars()[0];
        assert_eq!(
            first,
            Star {
                shown: true,
                frame: 7,
                factor: 0.34,
                x: 1000.0 + 100.0 - 415.0,
                y: -500.0 + 700.0 - 384.0,
            }
        );
        assert!(field.stars().iter().all(|star| star.shown));
    }

    #[test]
    fn without_parallax_no_factor_is_rolled_and_each_is_0() {
        let mut field = Starfield::new();
        let mut chance = Scripted::giving(&[1, 2, 3, 4]);
        field.randomize(&surroundings(), false, &mut chance);
        assert_eq!(
            chance.asked[..4],
            [
                Asked::Roll(16),
                Asked::Below(830),
                Asked::Below(768),
                Asked::Roll(16)
            ]
        );
        assert!(field.stars().iter().all(|star| star.factor == 0.0));
    }

    #[test]
    fn the_half_area_is_truncated_toward_zero() {
        let mut field = Starfield::new();
        let mut chance = Scripted::default();
        let at = Surroundings {
            area: (831, 767),
            ship: Vec2::ZERO,
            ..surroundings()
        };
        field.randomize(&at, true, &mut chance);
        assert_eq!((field.stars()[0].x, field.stars()[0].y), (-415.0, -383.0));
    }

    #[test]
    fn below_600_high_the_slots_past_the_count_are_shown_as_they_were() {
        let mut field = Starfield::new();
        field.slots[17].star = Star {
            shown: false,
            frame: 9,
            factor: 0.2,
            x: 5.0,
            y: 6.0,
        };
        let mut chance = Scripted::default();
        let at = Surroundings {
            area: (830, 480),
            ..surroundings()
        };
        field.randomize(&at, true, &mut chance);
        assert_eq!(chance.asked.len(), 4 * 16, "16 stars at 480");
        assert_eq!(
            field.stars()[17],
            Star {
                shown: true,
                frame: 9,
                factor: 0.2,
                x: 5.0,
                y: 6.0,
            }
        );
        assert_eq!(
            field.stars()[19],
            Star {
                shown: true,
                ..Star::default()
            }
        );
    }

    #[test]
    fn the_murk_level_is_full_without_murk_and_scaled_and_held_with_it() {
        assert_eq!(murk_level(0), 32);
        assert_eq!(murk_level(3), 2, "2.7, truncated");
        assert_eq!(murk_level(2), 2, "1.8 → 1, at most 1");
        assert_eq!(murk_level(8), 7);
        assert_eq!(murk_level(32), 28);
        assert_eq!(murk_level(33), 29);
        assert_eq!(murk_level(50), 29);
        assert_eq!(murk_level(100), 29);
        assert_eq!(murk_level(-1), 2);
    }

    #[test]
    fn a_star_is_placed_at_its_offset_from_the_ship_plus_the_anchor_less_its_size() {
        let mut field = one_star(1010.7, -520.2, 0.0, (0, 0));
        field.handle(&surroundings(), &mut Scripted::default());
        assert_eq!(field.placements()[0], (10 + 415 - 5, -20 + 384 - 5));
        assert_eq!(field.placements()[1], (0, 0), "a hidden slot's unplaced");
    }

    #[test]
    fn a_star_more_than_twice_its_size_left_wraps_right_with_a_new_y() {
        let mut field = one_star(100.0, 200.0, 0.0, (-13, 50));
        let mut chance = Scripted::giving(&[300]);
        field.handle(&surroundings(), &mut chance);
        let star = field.stars()[0];
        assert_eq!(chance.asked, [Asked::Below(768)]);
        assert_eq!(star.x, 100.0 + 830.0 + 10.0, "W + 2 × 5");
        assert_eq!(star.y, -500.0 + 300.0 - 384.0);
    }

    #[test]
    fn a_star_exactly_twice_its_size_out_stays() {
        // The centre, -12 + 2, is -10: not below.
        let mut field = one_star(100.0, 200.0, 0.0, (-12, 50));
        let mut chance = Scripted::default();
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].x, 100.0);
        assert_eq!(chance.asked, []);
        // The centre, 838 + 2, is 840: not past.
        let mut field = one_star(100.0, 200.0, 0.0, (838, 50));
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].x, 100.0);
        let mut field = one_star(100.0, 200.0, 0.0, (50, 776));
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].y, 200.0);
        let mut field = one_star(100.0, 200.0, 0.0, (50, -12));
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].y, 200.0);
        assert_eq!(chance.asked, []);
    }

    #[test]
    fn a_star_more_than_twice_its_size_right_wraps_left_with_a_new_y() {
        let mut field = one_star(100.0, 200.0, 0.0, (839, 50));
        let mut chance = Scripted::giving(&[0]);
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].x, 100.0 - 840.0);
        assert_eq!(field.stars()[0].y, -500.0 - 384.0);
    }

    #[test]
    fn a_star_past_either_edge_on_y_wraps_with_a_new_x() {
        let mut field = one_star(100.0, 200.0, 0.0, (50, -13));
        let mut chance = Scripted::giving(&[30]);
        field.handle(&surroundings(), &mut chance);
        assert_eq!(chance.asked, [Asked::Below(830)]);
        assert_eq!(field.stars()[0].y, 200.0 + 778.0, "H + 2 × 5");
        assert_eq!(field.stars()[0].x, 1000.0 + 30.0 - 415.0);
        let mut field = one_star(100.0, 200.0, 0.0, (50, 777));
        field.handle(&surroundings(), &mut Scripted::default());
        assert_eq!(field.stars()[0].y, 200.0 - 778.0);
    }

    #[test]
    fn the_wrap_reads_the_last_placement_not_the_new_position() {
        // Far to the right of the ship, but last placed on screen: no wrap
        // this pass, then placed far off, and wrapped on the next.
        let mut field = one_star(5000.0, -500.0, 0.0, (100, 100));
        let mut chance = Scripted::default();
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].x, 5000.0);
        assert_eq!(field.placements()[0].0, 4000 + 415 - 5);
        field.handle(&surroundings(), &mut chance);
        assert_eq!(field.stars()[0].x, 5000.0 - 840.0);
    }

    #[test]
    fn both_wraps_read_the_centre_before_either_moves_it() {
        let mut field = one_star(100.0, 200.0, 0.0, (-13, -13));
        let mut chance = Scripted::giving(&[1, 2]);
        field.handle(&surroundings(), &mut chance);
        assert_eq!(chance.asked, [Asked::Below(768), Asked::Below(830)]);
        // y redrawn by the x wrap, then wrapped; x wrapped, then redrawn.
        assert_eq!(field.stars()[0].y, -500.0 + 1.0 - 384.0 + 778.0);
        assert_eq!(field.stars()[0].x, 1000.0 + 2.0 - 415.0);
    }

    #[test]
    fn a_scroll_moves_a_star_back_by_its_factor_of_the_move() {
        let mut field = one_star(100.0, 200.0, 0.25, (0, 0));
        field.scroll(8.0, -4.0);
        assert_eq!((field.stars()[0].x, field.stars()[0].y), (98.0, 201.0));
        let mut fixed = one_star(100.0, 200.0, 0.0, (0, 0));
        fixed.scroll(8.0, -4.0);
        assert_eq!((fixed.stars()[0].x, fixed.stars()[0].y), (100.0, 200.0));
        let mut hidden = one_star(100.0, 200.0, 0.25, (0, 0));
        hidden.slots[0].star.shown = false;
        hidden.scroll(8.0, -4.0);
        assert_eq!((hidden.stars()[0].x, hidden.stars()[0].y), (100.0, 200.0));
    }

    #[test]
    fn a_clear_hides_every_star() {
        let mut field = Starfield::new();
        field.randomize(&surroundings(), true, &mut Scripted::default());
        field.clear();
        assert!(field.stars().iter().all(|star| !star.shown));
        field.snapshot();
        let mut list = DrawList::new();
        field.draw(&mut list, &sheet(), 0.5);
        assert!(list.is_empty());
    }

    fn drawn(field: &Starfield, alpha: f32) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        field.draw(&mut list, &sheet(), alpha);
        list.iter().cloned().collect()
    }

    #[test]
    fn a_star_is_its_frame_ored_at_its_rects_centre() {
        let mut field = one_star(1010.0, -520.0, 0.0, (0, 0));
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        let mut expected = DrawList::new();
        expected.or_sprite(
            ImageKey::sprite(700, 3),
            Point::new(420.0 + 2.5, 359.0 + 2.5),
            lights_tint(32),
        );
        let expected: Vec<DrawCommand> = expected.iter().cloned().collect();
        assert_eq!(drawn(&field, 0.7), expected);
        assert!(matches!(
            expected[0],
            DrawCommand::Sprite {
                blend: Blend::Or,
                ..
            }
        ));
    }

    #[test]
    fn a_murky_systems_stars_are_drawn_at_its_level() {
        let mut field = one_star(1010.0, -520.0, 0.0, (0, 0));
        let murky = Surroundings {
            murk: 8,
            ..surroundings()
        };
        field.handle(&murky, &mut Scripted::default());
        assert_eq!(drawn(&field, 0.0), [], "nothing before a snapshot");
        field.snapshot();
        let DrawCommand::Sprite { tint, .. } = drawn(&field, 0.0)[0] else {
            panic!("a sprite")
        };
        assert_eq!(tint, lights_tint(7));
    }

    #[test]
    fn a_star_is_drawn_between_its_last_two_snapshots() {
        let mut field = one_star(1010.0, -520.0, 0.0, (0, 0));
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        let moved = Surroundings {
            ship: Vec2::new(1000.0 - 10.0, -500.0 + 4.0),
            ..surroundings()
        };
        field.handle(&moved, &mut Scripted::default());
        field.snapshot();
        let DrawCommand::Sprite { center, .. } = drawn(&field, 0.5)[0] else {
            panic!("a sprite")
        };
        assert_eq!(center, Point::new(425.0 + 2.5, 357.0 + 2.5));
    }

    #[test]
    fn a_wrapped_or_scattered_star_is_drawn_where_it_is() {
        let mut field = one_star(1010.0, -520.0, 0.0, (0, 0));
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        field.slots[0].placed = (-13, 50);
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        let to = field.placements()[0];
        let DrawCommand::Sprite { center, .. } = drawn(&field, 0.0)[0] else {
            panic!("a sprite")
        };
        #[allow(clippy::cast_precision_loss)]
        let expected = Point::new(to.0 as f32 + 2.5, to.1 as f32 + 2.5);
        assert_eq!(center, expected, "wrapped");
        field.randomize(&surroundings(), true, &mut Scripted::giving(&[0, 1]));
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        let to = field.placements()[0];
        let DrawCommand::Sprite { center, .. } = drawn(&field, 0.0)[0] else {
            panic!("a sprite")
        };
        #[allow(clippy::cast_precision_loss)]
        let expected = Point::new(to.0 as f32 + 2.5, to.1 as f32 + 2.5);
        assert_eq!(center, expected, "scattered");
    }

    #[test]
    fn a_star_marked_as_jumped_is_drawn_where_it_is_placed() {
        let mut field = one_star(1010.0, -520.0, 0.0, (0, 0));
        field.handle(&surroundings(), &mut Scripted::default());
        field.snapshot();
        let was = surroundings();
        let moved = Surroundings {
            ship: was.ship + Vec2::new(-25_000.0, 0.0),
            ..was
        };
        field.handle(&moved, &mut Scripted::default());
        field.mark_jumped();
        field.snapshot();
        let to = field.placements()[0];
        let DrawCommand::Sprite { center, .. } = drawn(&field, 0.0)[0] else {
            panic!("a sprite")
        };
        #[allow(clippy::cast_precision_loss)]
        let expected = Point::new(to.0 as f32 + 2.5, to.1 as f32 + 2.5);
        assert_eq!(center, expected);
    }

    #[test]
    fn a_star_a_wrap_left_behind_steps_back_a_pass_at_a_time_and_is_shown_again() {
        // `_HandleStars` (@0x2f87d-0x2fcae): each pass moves it W + 2 × size
        // by its last placement, its other axis scattered about the ship, so
        // the stars black out for 25,000 / (W + 2 × size) passes.
        let ship = surroundings();
        let mut field = one_star(ship.ship.x, ship.ship.y, 0.0, (0, 0));
        field.handle(&ship, &mut Scripted::default());
        let moved = Surroundings {
            ship: ship.ship + Vec2::new(-25_000.0, 0.0),
            ..ship
        };
        let (width, size) = (830_i32, 5);
        let on_screen = |field: &Starfield| (0..width).contains(&field.placements()[0].0);
        let mut passes = 0;
        field.handle(&moved, &mut Scripted::default());
        while !on_screen(&field) {
            field.handle(&moved, &mut Scripted::default());
            passes += 1;
            assert!(passes < 100, "it comes back");
        }
        let step = width + 2 * size;
        assert_eq!(passes, (25_000 + step - 1) / step, "rounded up");
    }

    #[test]
    fn the_look_of_a_sheet_is_its_frames_and_size_and_without_one_32_square() {
        assert_eq!(StarLook::of(Some(&sheet())), LOOK);
        assert_eq!(
            StarLook::of(None),
            StarLook {
                frames: 0,
                size: (32, 32),
            }
        );
    }

    #[test]
    fn without_a_sheet_the_frame_rolls_0_sides_and_an_empty_area_scatters_on_the_ship() {
        let mut field = Starfield::new();
        let mut chance = Scripted::giving(&[5, 5, 5, 5]);
        let at = Surroundings {
            area: (0, 768),
            look: StarLook::of(None),
            ..surroundings()
        };
        field.randomize(&at, false, &mut chance);
        assert_eq!(chance.asked[..2], [Asked::Roll(0), Asked::Below(768)]);
        assert_eq!(field.stars()[0].x, 1000.0);
    }
}
