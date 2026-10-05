//! Drawing shots and beams, as the original does (`_HandleShot`
//! @0x3586a-0x359af, `_BeamDrawCallback` @0x37952 and `_DrawBeam`
//! @0xc3b3c in the `EV Nova` executable).
//!
//! - A shot is a frame of its weapon's sheet, `spïn` 3000 + `Graphic`
//!   ([`shot_frame`]): the frame for its heading, `trunc(n x heading /
//!   360)`, or, for a spinning weapon (`Flags` 0x0001), one frame every
//!   `BeamWidth` ticks of its age, wrapping, or holding the last
//!   (`Flags2` 0x0002), and from frame 0 only once its age reaches
//!   `ProxSafety` (`Flags2` 0x0001). Every shot starts on the first frame
//!   (`Flags` 0x0004 always holds: there is no random start).
//! - A translucent shot (`Flags3` 0x0002) is blended additively in the
//!   original; the renderer cannot, so it is drawn at
//!   [`TRANSLUCENT_ALPHA`] instead, a placeholder.
//! - A beam is drawn from its start to its end as lines in `BeamColor`
//!   ([`beam_strokes`]): its core at offsets 0, ±1, up to one short of
//!   ±`BeamWidth` along the minor axis, at alpha (32 - 16|i|)/32, and with a
//!   `Falloff`, its corona in `CoronaColor` at offsets ±(`BeamWidth` + j),
//!   from alpha 16/32 down by `Falloff`/32 a line while above 2/32. A
//!   lightning beam (`LiDensity`) is drawn as its plain core, a
//!   placeholder for the original's random zig-zag.
//! - A beam whose weapon sets `Flags2` 0x2000 is drawn under the ships,
//!   and every other beam over them.

use std::num::NonZeroU16;

use super::catalog::{Looks, WeaponId, WeaponLook};
use crate::draw::crossed_box;
use crate::system::scene::PLACEHOLDER;
use crate::{Color, DrawList, ImageKey, Point};

/// `Flags`: the shot's frames spin with its age, not its heading.
pub const SPINS: u16 = 0x0001;
/// `Flags`: the firing sound loops while the beam lasts.
pub const LOOPED_SOUND: u16 = 0x0010;
/// `Flags2`: the shot shows frame 0 until its age reaches `ProxSafety`.
pub const STILL_UNTIL_SAFE: u16 = 0x0001;
/// `Flags2`: a spinning shot holds its last frame instead of wrapping.
pub const HOLDS_LAST_FRAME: u16 = 0x0002;
/// `Flags2`: the secondary weapon's line hides its ammunition count.
pub const HIDES_AMMO: u16 = 0x0040;
/// `Flags2`: the beam is drawn under the ships.
pub const BEAM_UNDER_SHIPS: u16 = 0x2000;
/// `Flags3`: the shot is translucent.
pub const TRANSLUCENT: u16 = 0x0002;
/// The alpha a translucent shot, and every explosion, is drawn at: a
/// placeholder for the original's additive blend.
pub const TRANSLUCENT_ALPHA: f32 = 0.75;
/// The side of the crossed box drawn for a shot whose look cannot be
/// read, in the placeholder colour.
pub const SHOT_PLACEHOLDER_SIZE: f32 = 8.0;
/// A beam's alpha is in 32nds.
const ALPHA_STEPS: i32 = 32;
/// A beam core's alpha falls by this many 32nds a line out from its
/// centre.
const CORE_FALLOFF: i32 = 16;
/// A beam corona's first alpha, in 32nds.
const CORONA_START: i32 = 16;
/// A beam corona's lines stop once their alpha is this many 32nds or
/// less.
const CORONA_END: i32 = 2;

/// The frame of a shot heading `heading` degrees, `age` ticks old, on a
/// sheet of `frames` frames, of a weapon that looks as `look` says.
#[must_use]
pub fn shot_frame(heading: f32, age: u32, frames: NonZeroU16, look: &WeaponLook) -> u16 {
    let count = frames.get();
    let last = count - 1;
    if look.flags & SPINS == 0 {
        let turned = f32::from(count) * heading.rem_euclid(360.0) / 360.0;
        // Just short of a full turn can round to a full turn.
        return (turned as u16).min(last);
    }
    let safety = if look.flags2 & STILL_UNTIL_SAFE == 0 {
        0
    } else {
        u32::try_from(look.prox_safety).unwrap_or(0)
    };
    let Some(spinning) = age.checked_sub(safety) else {
        return 0;
    };
    let ticks_a_frame = u32::try_from(look.beam_width).unwrap_or(0).max(1);
    let turns = spinning / ticks_a_frame;
    let frame = if look.flags2 & HOLDS_LAST_FRAME == 0 {
        turns % u32::from(count)
    } else {
        turns.min(u32::from(last))
    };
    u16::try_from(frame).unwrap_or(last)
}

/// The lines that draw a beam from `start` to `end` of a weapon that
/// looks as `look` says, each with its colour, in drawing order: the
/// corona, then the core out to in.
#[must_use]
pub fn beam_strokes(start: Point, end: Point, look: &WeaponLook) -> Vec<(Point, Point, Color)> {
    // A mostly vertical beam widens along x, any other along y.
    let along_x = (end.x - start.x).abs() < (end.y - start.y).abs();
    let line = |offset: i32, color: u32, alpha: i32| {
        let by = offset as f32;
        let (dx, dy) = if along_x { (by, 0.0) } else { (0.0, by) };
        (
            Point::new(start.x + dx, start.y + dy),
            Point::new(end.x + dx, end.y + dy),
            faded(color, alpha),
        )
    };
    let width = i32::from(look.beam_width.max(1));
    let mut strokes = Vec::new();
    if look.falloff > 0 {
        // From the start down by the falloff a line, while above the end.
        let fading = (CORONA_END + 1..=CORONA_START)
            .rev()
            .step_by(usize::from(look.falloff.unsigned_abs()));
        for (out, alpha) in (width..).zip(fading) {
            strokes.push(line(-out, look.corona_color, alpha));
            strokes.push(line(out, look.corona_color, alpha));
        }
    }
    for out in (1..width).rev() {
        let alpha = ALPHA_STEPS - CORE_FALLOFF * out;
        if alpha > 0 {
            strokes.push(line(-out, look.beam_color, alpha));
            strokes.push(line(out, look.beam_color, alpha));
        }
    }
    strokes.push(line(0, look.beam_color, ALPHA_STEPS));
    strokes
}

/// The colour `00RRGGBB` `raw` at `alpha` 32nds opaque.
fn faded(raw: u32, alpha: i32) -> Color {
    let color = Color::from_rgb24(raw);
    let a = u8::try_from(255 * alpha / ALPHA_STEPS).unwrap_or(u8::MAX);
    Color::rgba(color.r, color.g, color.b, a)
}

/// A shot to draw: where on screen, which way it heads, how old it is and
/// its weapon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotShown {
    /// Where its centre is drawn, on screen.
    pub at: Point,
    /// Which way it heads, in degrees.
    pub heading: f32,
    /// The ticks it has flown.
    pub age: u32,
    /// Its weapon.
    pub weapon: WeaponId,
}

/// Draws each of `shots` as its weapon's frame ([`shot_frame`]), a
/// translucent one at [`TRANSLUCENT_ALPHA`]; nothing for a weapon whose
/// shots have no graphic, and a crossed box for one whose look was not
/// read.
pub fn draw_shots(list: &mut DrawList, shots: &[ShotShown], looks: &Looks) {
    for shot in shots {
        let Some(look) = looks.weapon(shot.weapon) else {
            crossed_box(list, shot.at, SHOT_PLACEHOLDER_SIZE, PLACEHOLDER);
            continue;
        };
        if let Some(sheet) = look.sheet {
            let frame = shot_frame(shot.heading, shot.age, sheet.frames, look);
            let tint = if look.flags3 & TRANSLUCENT == 0 {
                Color::WHITE
            } else {
                translucent()
            };
            list.sprite(ImageKey::sprite(sheet.image_id, frame), shot.at, tint);
        }
    }
}

/// White at [`TRANSLUCENT_ALPHA`].
#[must_use]
pub fn translucent() -> Color {
    Color::rgba(255, 255, 255, (TRANSLUCENT_ALPHA * 255.0) as u8)
}

/// A beam to draw: from where to where on screen, and its weapon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamShown {
    /// Where it starts, on screen.
    pub start: Point,
    /// Where it ends, on screen.
    pub end: Point,
    /// Its weapon.
    pub weapon: WeaponId,
}

/// Draws those of `beams` drawn `under` the ships (`Flags2` 0x2000), or
/// the others, as [`beam_strokes`]; a beam whose look was not read is a
/// plain line in [`Color::ERROR`], over the ships.
pub fn draw_beams(list: &mut DrawList, beams: &[BeamShown], looks: &Looks, under: bool) {
    for beam in beams {
        match looks.weapon(beam.weapon) {
            Some(look) if (look.flags2 & BEAM_UNDER_SHIPS != 0) == under => {
                for (from, to, color) in beam_strokes(beam.start, beam.end, look) {
                    list.line(from, to, 1.0, color);
                }
            }
            None if !under => {
                list.line(beam.start, beam.end, 1.0, Color::ERROR);
            }
            Some(_) | None => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::DrawCommand;
    use crate::flight::catalog::EffectSheet;

    fn n(frames: u16) -> NonZeroU16 {
        NonZeroU16::new(frames).expect("non-zero")
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn a_shot_shows_the_frame_its_heading_truncates_to() {
        let plain = WeaponLook::default();
        let frames: Vec<u16> = [0.0, 9.9, 10.0, 185.0, 359.9]
            .into_iter()
            .map(|heading| shot_frame(heading, 0, n(36), &plain))
            .collect();
        assert_eq!(frames, [0, 0, 1, 18, 35]);
        assert_eq!(shot_frame(360.0, 0, n(36), &plain), 0, "a full turn");
        assert_eq!(shot_frame(-10.0, 0, n(36), &plain), 35);
        // Just short of 0 is a whole turn in f32.
        assert_eq!(shot_frame(-0.000_001, 0, n(36), &plain), 35, "clamped");
        assert_eq!(shot_frame(200.0, 0, n(1), &plain), 0);
        assert_eq!(shot_frame(100.0, 0, n(2), &plain), 0);
        assert_eq!(shot_frame(190.0, 0, n(2), &plain), 1);
    }

    fn spinning(beam_width: i16, flags2: u16, prox_safety: i16) -> WeaponLook {
        WeaponLook {
            flags: SPINS,
            flags2,
            beam_width,
            prox_safety,
            ..WeaponLook::default()
        }
    }

    fn ages(look: &WeaponLook, frames: u16, ages: std::ops::Range<u32>) -> Vec<u16> {
        ages.map(|age| shot_frame(90.0, age, n(frames), look))
            .collect()
    }

    #[test]
    fn a_spinning_shot_turns_a_frame_every_beam_width_ticks_and_wraps() {
        assert_eq!(ages(&spinning(2, 0, 0), 3, 0..8), [0, 0, 1, 1, 2, 2, 0, 0]);
        assert_eq!(
            ages(&spinning(0, 0, 0), 3, 0..4),
            [0, 1, 2, 0],
            "every tick"
        );
        assert_eq!(ages(&spinning(-1, 0, 0), 3, 0..4), [0, 1, 2, 0]);
        assert_eq!(ages(&spinning(1, 0, 0), 3, 0..4), [0, 1, 2, 0]);
    }

    #[test]
    fn a_spinning_shot_may_hold_its_last_frame() {
        assert_eq!(
            ages(&spinning(1, HOLDS_LAST_FRAME, 0), 3, 0..6),
            [0, 1, 2, 2, 2, 2]
        );
    }

    #[test]
    fn a_spinning_shot_may_wait_for_its_prox_safety() {
        assert_eq!(
            ages(&spinning(1, STILL_UNTIL_SAFE, 3), 4, 0..9),
            [0, 0, 0, 0, 1, 2, 3, 0, 1]
        );
        assert_eq!(
            ages(&spinning(1, 0, 3), 4, 0..3),
            [0, 1, 2],
            "only with the flag"
        );
        assert_eq!(
            ages(
                &spinning(1, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 2),
                3,
                0..7
            ),
            [0, 0, 0, 1, 2, 2, 2]
        );
    }

    fn beam(beam_width: i16, falloff: i16) -> WeaponLook {
        WeaponLook {
            beam_width,
            falloff,
            beam_color: 0x00FF_8000,
            corona_color: 0x0000_80FF,
            ..WeaponLook::default()
        }
    }

    const BEAM: Color = Color::rgba(255, 128, 0, 255);
    const HALF: Color = Color::rgba(255, 128, 0, 127);

    fn corona(alpha: u8) -> Color {
        Color::rgba(0, 128, 255, alpha)
    }

    #[test]
    fn a_beam_one_wide_is_one_opaque_line_in_its_colour() {
        let from = at(10.0, 20.0);
        let to = at(110.0, 30.0);
        assert_eq!(beam_strokes(from, to, &beam(1, 0)), [(from, to, BEAM)]);
        assert_eq!(beam_strokes(from, to, &beam(0, 0)), [(from, to, BEAM)]);
    }

    #[test]
    fn a_beam_two_wide_adds_half_alpha_lines_along_its_minor_axis() {
        // Mostly horizontal: offsets go down y.
        let strokes = beam_strokes(at(0.0, 0.0), at(100.0, 10.0), &beam(2, 0));
        assert_eq!(
            strokes,
            [
                (at(0.0, -1.0), at(100.0, 9.0), HALF),
                (at(0.0, 1.0), at(100.0, 11.0), HALF),
                (at(0.0, 0.0), at(100.0, 10.0), BEAM),
            ]
        );
        // Mostly vertical: offsets go along x.
        let strokes = beam_strokes(at(0.0, 0.0), at(10.0, -100.0), &beam(2, 0));
        assert_eq!(
            strokes,
            [
                (at(-1.0, 0.0), at(9.0, -100.0), HALF),
                (at(1.0, 0.0), at(11.0, -100.0), HALF),
                (at(0.0, 0.0), at(10.0, -100.0), BEAM),
            ]
        );
    }

    /// Which axis `beam` widens along: x or y.
    fn widens(from: Point, to: Point) -> char {
        let strokes = beam_strokes(from, to, &beam(2, 0));
        if strokes[0].0.x == from.x { 'y' } else { 'x' }
    }

    #[test]
    fn the_minor_axis_follows_the_beams_run_whichever_way_it_goes() {
        assert_eq!(widens(at(100.0, 0.0), at(-90.0, 30.0)), 'y', "leftwards");
        assert_eq!(widens(at(0.0, 100.0), at(30.0, -90.0)), 'x', "upwards");
        assert_eq!(widens(at(0.0, 0.0), at(50.0, 50.0)), 'y', "a tie");
    }

    #[test]
    fn a_corona_line_at_two_32nds_is_left_out() {
        // 16, 9, then 2: not above 2.
        let strokes = beam_strokes(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 7));
        let alphas: Vec<u8> = strokes.iter().map(|(_, _, color)| color.a).collect();
        assert_eq!(alphas, [127, 127, 71, 71, 255]);
    }

    #[test]
    fn a_wide_beams_outer_core_lines_vanish() {
        let strokes = beam_strokes(at(0.0, 0.0), at(100.0, 0.0), &beam(4, 0));
        assert_eq!(strokes.len(), 3, "±2 and ±3 are clear: {strokes:?}");
    }

    #[test]
    fn a_falloff_adds_a_corona_fading_by_it_a_line() {
        let strokes = beam_strokes(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 8));
        assert_eq!(
            strokes,
            [
                (at(0.0, -1.0), at(100.0, -1.0), corona(127)),
                (at(0.0, 1.0), at(100.0, 1.0), corona(127)),
                (at(0.0, -2.0), at(100.0, -2.0), corona(63)),
                (at(0.0, 2.0), at(100.0, 2.0), corona(63)),
                (at(0.0, 0.0), at(100.0, 0.0), BEAM),
            ]
        );
        let slow = beam_strokes(at(0.0, 0.0), at(100.0, 0.0), &beam(2, 4));
        let alphas: Vec<u8> = slow.iter().map(|(_, _, color)| color.a).collect();
        // Corona at ±2 to ±5 (16, 12, 8 and 4 32nds: 4 is above 2), then
        // the core.
        assert_eq!(alphas, [127, 127, 95, 95, 63, 63, 31, 31, 127, 127, 255]);
        assert_eq!(slow[0].0, at(0.0, -2.0));
        assert_eq!(slow[6].0, at(0.0, -5.0));
        let none = beam_strokes(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 0));
        assert_eq!(none.len(), 1, "no falloff, no corona");
    }

    fn looks(weapons: &[(i16, WeaponLook)]) -> Looks {
        Looks {
            weapons: weapons
                .iter()
                .map(|(id, look)| (WeaponId(*id), Ok(look.clone())))
                .collect(),
            booms: BTreeMap::new(),
        }
    }

    fn sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            image_id,
            frames: n(frames),
        }
    }

    #[test]
    fn shots_are_drawn_as_their_frames_translucent_ones_see_through() {
        let looks = looks(&[
            (
                128,
                WeaponLook {
                    sheet: Some(sheet(3500, 36)),
                    ..WeaponLook::default()
                },
            ),
            (
                129,
                WeaponLook {
                    sheet: Some(sheet(3501, 36)),
                    flags3: TRANSLUCENT,
                    ..WeaponLook::default()
                },
            ),
            (130, WeaponLook::default()),
        ]);
        let shot = |x, weapon| ShotShown {
            at: at(x, 5.0),
            heading: 90.0,
            age: 3,
            weapon: WeaponId(weapon),
        };
        let mut list = DrawList::new();
        draw_shots(
            &mut list,
            &[
                shot(1.0, 128),
                shot(2.0, 129),
                shot(3.0, 130),
                shot(4.0, 131),
            ],
            &looks,
        );
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(3500, 9), at(1.0, 5.0), Color::WHITE);
        expected.sprite(
            ImageKey::sprite(3501, 9),
            at(2.0, 5.0),
            Color::rgba(255, 255, 255, 191),
        );
        crossed_box(
            &mut expected,
            at(4.0, 5.0),
            SHOT_PLACEHOLDER_SIZE,
            PLACEHOLDER,
        );
        assert_eq!(list, expected);
        assert_eq!((TRANSLUCENT_ALPHA, SHOT_PLACEHOLDER_SIZE), (0.75, 8.0));
    }

    #[test]
    fn beams_are_drawn_under_the_ships_or_over_them_as_their_weapon_says() {
        let looks = looks(&[
            (146, beam(1, 0)),
            (
                147,
                WeaponLook {
                    flags2: BEAM_UNDER_SHIPS,
                    ..beam(1, 0)
                },
            ),
        ]);
        let shown = |weapon| BeamShown {
            start: at(0.0, 0.0),
            end: at(50.0, 0.0),
            weapon: WeaponId(weapon),
        };
        let beams = [shown(146), shown(147), shown(148)];
        let lines = |under| {
            let mut list = DrawList::new();
            draw_beams(&mut list, &beams, &looks, under);
            list.iter()
                .map(|command| match *command {
                    DrawCommand::Line { color, width, .. } => (color, width),
                    _ => panic!("{command:?}"),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(lines(true), [(BEAM, 1.0)]);
        assert_eq!(lines(false), [(BEAM, 1.0), (Color::ERROR, 1.0)]);
    }
}
