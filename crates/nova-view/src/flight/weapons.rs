//! Drawing shots and beams, as the original does (`_HandleShot`
//! @0x3586a-0x359af, `_BeamDrawCallback` @0x37952 and `_DrawBeam`
//! @0xc3b3c in the `EV Nova` executable).
//!
//! - A shot is a frame of its weapon's sheet, `spïn` 3000 + `Graphic`
//!   ([`shot_frame`]): the frame for its heading, `trunc(n x heading /
//!   360)`, or, for a spinning weapon (`Flags` 0x0001), one frame every
//!   `BeamWidth` ticks of its age from the index its spin started on, the
//!   first coming early on the spin timer its shot slot's last spinning
//!   shot left (`nova_sim::combat::spin_timer`), wrapping, or holding the
//!   last (`Flags2` 0x0002), and frame 0 while
//!   its age is below `ProxSafety` (`Flags2` 0x0001). A shot's spin starts
//!   on a random index, `Rand(36)`, unless its weapon has `Flags` 0x0004
//!   ([`spin`](super::spin)).
//! - A shot fades in and out by its `Falloff`, crossfaded by OR with what
//!   is under it, and a translucent one (`Flags3` 0x0002) is added to it
//!   ([`shot_fade`](super::shot_fade)); a shot far from the player's ship
//!   in a murky system is darkened ([`murk`](super::murk)).
//! - A beam is drawn from its start to its end as lines in `BeamColor`
//!   ([`beam_strokes`]): its core at offsets 0, ±1, up to one short of
//!   ±`BeamWidth` along the minor axis, at alpha (32 - 16|i|)/32, and with a
//!   `Falloff` as the original loads it (16 for none) plus the ticks it has
//!   faded, its corona in `CoronaColor` at offsets ±(`BeamWidth` + j),
//!   from alpha 16/32 down by that falloff/32 a line while above 2/32. A
//!   lightning beam (`LiDensity`) is drawn as its zig-zag, fading by its
//!   fade over 32 ticks ([`lightning`]).
//! - A beam whose weapon sets `Flags2` 0x2000 is drawn under the ships,
//!   and every other beam over them.

use std::num::NonZeroU16;

use super::catalog::{Looks, WeaponId, WeaponLook};
use super::lightning::{self, StepRolls};
use super::murk::murk_tint;
use super::shot_fade::{ShotShade, shot_shade};
use crate::draw::{crossed_box, lights_tint};
use crate::system::scene::PLACEHOLDER;
use crate::{Color, DrawList, ImageKey, Point};
use nova_sim::combat::projectile::ShotId;
use nova_sim::combat::spin_timer;
use nova_sim::sprite_layer::SpriteLayer;
use nova_sim::{Chance, RuleSource};

/// `Flags`: the shot's frames spin with its age, not its heading.
pub const SPINS: u16 = 0x0001;
/// `Flags`: the firing sound loops while the beam lasts.
pub const LOOPED_SOUND: u16 = 0x0010;
/// `Flags2`: the shot shows frame 0 until its age reaches `ProxSafety`.
pub const STILL_UNTIL_SAFE: u16 = 0x0001;
/// `Flags2`: a spinning shot holds its last frame instead of wrapping.
pub const HOLDS_LAST_FRAME: u16 = 0x0002;
/// `Flags2`: the beam is drawn under the ships.
pub const BEAM_UNDER_SHIPS: u16 = 0x2000;
/// `Flags3`: the shot is translucent, added to what is under it (see
/// [`shot_fade`](super::shot_fade)).
pub const TRANSLUCENT: u16 = 0x0002;
/// The alpha every explosion is drawn at ([`effects`](super::effects)): a
/// placeholder for the original's additive blend of a translucent
/// explosion.
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
/// sheet of `frames` frames, of a weapon that looks as `look` says, whose
/// spin started on index `start` ([`SpinStarts`](super::spin::SpinStarts))
/// and timer `timer`
/// ([`Shot::spin_timer`](nova_sim::combat::projectile::Shot::spin_timer)).
///
/// A shot that does not spin shows its heading's frame. A spinning one
/// (`Flags` 0x0001) shows the index `_HandleShot` holds before its
/// tick's step (@0x35894), clamped to the sheet (@0x35956-0x3596c): the
/// shot drawn at `age` is on its `age`-th pass (the session moves it,
/// and ages it, once a pass), so it has stepped on each pass before at
/// which its spin timer, from `timer`, reached `BeamWidth`, every pass
/// for a `BeamWidth` of 1 or less (@0x35884-0x358c4;
/// [`spin_timer::steps`](nova_sim::combat::spin_timer::steps)): its first
/// step comes early on a timer its slot's last spinning shot left. A step
/// past the last frame wraps to 0, or holds
/// the last under `Flags2` 0x0002 (@0x358cb-0x358f6). So a start below
/// the frame count runs from the start; one at or past it shows the last
/// frame until the first step, then runs from 0 (or holds the last).
/// Under `Flags2` 0x0001 the shot shows frame 0 while its age is below
/// `ProxSafety`, the index running on beneath; with 0x0002 too, the index
/// is put back to 0 on each of those passes (@0x35914-0x35932).
#[must_use]
pub fn shot_frame(
    heading: f32,
    age: u32,
    frames: NonZeroU16,
    look: &WeaponLook,
    start: u8,
    timer: u32,
) -> u16 {
    let count = u32::from(frames.get());
    let last = count - 1;
    if look.flags & SPINS == 0 {
        // `(frames × heading)` in `f32`, over 360.0 in doubles, truncated
        // toward 0 and clamped to the sheet (`_HandleShot`
        // @0x3593a-0x3596c), so a bomb's unwrapped heading below 0 or past
        // 360 shows an end frame.
        let turned = f64::from(f32::from(frames.get()) * heading) / 360.0;
        return (turned as i64).clamp(0, i64::from(last)) as u16;
    }
    let holds = look.flags2 & HOLDS_LAST_FRAME != 0;
    let safety = if look.flags2 & STILL_UNTIL_SAFE == 0 {
        0
    } else {
        u32::try_from(look.prox_safety).unwrap_or(0)
    };
    if age < safety {
        return 0;
    }
    // The steps made on passes `from` to `to`.
    let steps = |from: u32, to: u32| spin_timer::steps(timer, look.beam_width, from, to);
    let before = age.saturating_sub(1);
    let index = if holds && safety > 1 {
        // Put back to 0 on every pass below the safety; the timer runs on.
        steps(safety, before).min(last)
    } else {
        let stepped = steps(1, before);
        let start = u32::from(start);
        match (start < count, holds) {
            (true, false) => (start + stepped) % count,
            (true, true) => (start + stepped).min(last),
            (false, _) if stepped == 0 => last,
            (false, false) => (stepped - 1) % count,
            (false, true) => last,
        }
    };
    u16::try_from(index).unwrap_or(u16::MAX)
}

/// The lines that draw a beam from `start` to `end` of a weapon that
/// looks as `look` says, `fade` ticks into its fade
/// ([`Beam::fade`](nova_sim::combat::beam::Beam::fade)), each with its
/// colour, in drawing order: the corona, then the core out to in. The
/// corona falls off by `Falloff` + `fade` and is drawn when that is above
/// none (`_DrawBeam` @0xc3e00), so a fading beam grows one. A lightning
/// beam (`LiDensity` above none) is its zig-zag instead
/// ([`lightning::zig_zag`]), drawn on the `zig_zag` rolls by its rule's
/// reading of [`RuleKey::LightningZigZag`](nova_sim::RuleKey::LightningZigZag),
/// at (32 − `fade`)/32 (`_BeamDrawCallback` @0x37c75-0x37d62).
#[must_use]
pub fn beam_strokes(
    start: Point,
    end: Point,
    look: &WeaponLook,
    fade: u32,
    zig_zag: (RuleSource, &mut dyn Chance),
) -> Vec<(Point, Point, Color)> {
    if look.li_density > 0 {
        let (rule, rolls) = zig_zag;
        return lightning::zig_zag(start, end, look, fade, rule, rolls);
    }
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
    let fade = i32::try_from(fade).unwrap_or(i32::MAX);
    let mut strokes = Vec::new();
    let falloff = i32::from(look.falloff).saturating_add(fade);
    if falloff > 0 {
        // From the start down by the falloff a line, while above the end.
        let step = usize::try_from(falloff).unwrap_or(usize::MAX);
        let fading = (CORONA_END + 1..=CORONA_START).rev().step_by(step);
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

/// A shot to draw: which it is, where on screen, which way it heads, how
/// old it is, its weapon, the sprite layer it is drawn in, the index its
/// spin started on and how murky it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotShown {
    /// Its number in the fight.
    pub id: ShotId,
    /// Where its centre is drawn, on screen.
    pub at: Point,
    /// Which way it heads, in degrees.
    pub heading: f32,
    /// The ticks it has flown.
    pub age: u32,
    /// Its weapon.
    pub weapon: WeaponId,
    /// The layer it is drawn in among the ships
    /// ([`nova_sim::sprite_layer`]).
    pub layer: SpriteLayer,
    /// The index its spin started on ([`SpinStarts`](super::spin::SpinStarts)).
    pub spin_start: u8,
    /// The spin timer it started on, its slot's leftover
    /// ([`Shot::spin_timer`](nova_sim::combat::projectile::Shot::spin_timer)).
    pub spin_timer: u32,
    /// How murky it is, out of 32 ([`sprite_murk`](super::murk::sprite_murk)).
    pub murk: u8,
}

/// Draws each of `shots` in turn ([`draw_shot`]), by `fade`'s reading
/// of [`RuleKey::TranslucentShotFade`](nova_sim::RuleKey::TranslucentShotFade).
pub fn draw_shots(list: &mut DrawList, shots: &[ShotShown], looks: &Looks, fade: RuleSource) {
    for shot in shots {
        draw_shot(list, shot, looks, fade);
    }
}

/// Draws `shot` as its weapon's frame ([`shot_frame`]), shaded as it
/// fades ([`shot_shade`], by `fade`'s reading of
/// [`RuleKey::TranslucentShotFade`](nova_sim::RuleKey::TranslucentShotFade)):
/// opaque; translucent by t, crossfaded by OR,
/// [`Blend::OrCrossfade`](crate::draw::Blend::OrCrossfade) at alpha
/// (32 − t)/32 ([`see_through`]); or additive,
/// [`Blend::Or`](crate::draw::Blend::Or) at [`lights_tint`]`(n)`; each
/// darkened by its murk ([`murk_tint`]). Nothing for a weapon whose shots
/// have no graphic, and a crossed box for one whose look or sheet was not
/// read.
pub fn draw_shot(list: &mut DrawList, shot: &ShotShown, looks: &Looks, fade: RuleSource) {
    let Some(look) = looks.weapon(shot.weapon) else {
        crossed_box(list, shot.at, SHOT_PLACEHOLDER_SIZE, PLACEHOLDER);
        return;
    };
    match &look.sheet {
        None => {}
        Some(Err(_)) => crossed_box(list, shot.at, SHOT_PLACEHOLDER_SIZE, PLACEHOLDER),
        Some(Ok(sheet)) => {
            let spin = (shot.spin_start, shot.spin_timer);
            let frame = shot_frame(shot.heading, shot.age, sheet.frames, look, spin.0, spin.1);
            let image = ImageKey::sprite(sheet.image_id, frame);
            match shot_shade(look, shot.age, fade) {
                ShotShade::Opaque => {
                    list.sprite(image, shot.at, murk_tint(Color::WHITE, shot.murk));
                }
                ShotShade::Translucent(t) => {
                    let tint = see_through(t);
                    list.crossfade_sprite(image, shot.at, murk_tint(tint, shot.murk));
                }
                ShotShade::Additive(n) => {
                    list.or_sprite(image, shot.at, murk_tint(lights_tint(n), shot.murk));
                }
            }
        }
    }
}

/// White at alpha (32 − `t`)/32, rounded: a shot translucent by `t`
/// 32nds.
#[must_use]
pub fn see_through(t: u8) -> Color {
    let shown = u16::from(FADE_STEPS) - u16::from(t).min(u16::from(FADE_STEPS));
    let alpha = (shown * 255 + u16::from(FADE_STEPS) / 2) / u16::from(FADE_STEPS);
    Color::rgba(255, 255, 255, u8::try_from(alpha).unwrap_or(u8::MAX))
}

/// The steps of a shot's translucency.
const FADE_STEPS: u8 = 32;

/// White at [`TRANSLUCENT_ALPHA`].
#[must_use]
pub fn translucent() -> Color {
    Color::rgba(255, 255, 255, (TRANSLUCENT_ALPHA * 255.0) as u8)
}

/// A beam to draw: from where to where on screen, its weapon, and how
/// far it has faded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamShown {
    /// Where it starts, on screen.
    pub start: Point,
    /// Where it ends, on screen.
    pub end: Point,
    /// Its weapon.
    pub weapon: WeaponId,
    /// The ticks it has faded at the end of its life, as the session
    /// counts them ([`Beam::fade`](nova_sim::combat::beam::Beam::fade)).
    pub fade: u32,
}

/// Draws those of `beams` drawn `under` the ships (`Flags2` 0x2000), or
/// the others, as [`beam_strokes`], the `n`-th's zig-zag, if it is
/// lightning, on the step's rolls for its `n`-th beam
/// ([`StepRolls::for_beam`]) by `rule`'s reading of
/// [`RuleKey::LightningZigZag`](nova_sim::RuleKey::LightningZigZag); a
/// beam whose look was not read is a plain line in [`Color::ERROR`], over
/// the ships.
pub fn draw_beams(
    list: &mut DrawList,
    beams: &[BeamShown],
    looks: &Looks,
    under: bool,
    (rule, rolls): (RuleSource, StepRolls),
) {
    for (n, beam) in beams.iter().enumerate() {
        match looks.weapon(beam.weapon) {
            Some(look) if (look.flags2 & BEAM_UNDER_SHIPS != 0) == under => {
                let mut rolls = rolls.for_beam(n);
                let zig_zag = (rule, &mut rolls as &mut dyn Chance);
                for (from, to, color) in
                    beam_strokes(beam.start, beam.end, look, beam.fade, zig_zag)
                {
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
    use crate::draw::Blend;
    use crate::flight::catalog::EffectSheet;

    fn n(frames: u16) -> NonZeroU16 {
        NonZeroU16::new(frames).expect("non-zero")
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// [`beam_strokes`] of a beam that is not lightning.
    fn plain(start: Point, end: Point, look: &WeaponLook, fade: u32) -> Vec<(Point, Point, Color)> {
        beam_strokes(
            start,
            end,
            look,
            fade,
            (RuleSource::Engine, &mut StepRolls::default()),
        )
    }

    #[test]
    fn a_shot_shows_the_frame_its_heading_truncates_to() {
        let plain = WeaponLook::default();
        let frame = |heading, frames| shot_frame(heading, 1, n(frames), &plain, 0, 0);
        let frames: Vec<u16> = [0.0, 9.9, 10.0, 185.0, 359.9]
            .into_iter()
            .map(|heading| frame(heading, 36))
            .collect();
        assert_eq!(frames, [0, 0, 1, 18, 35]);
        // `_HandleShot` truncates toward 0 and clamps to the sheet
        // (@0x3593a-0x3596c): an unwrapped bomb past 360 or below 0 shows
        // an end frame.
        assert_eq!(frame(360.0, 36), 35, "clamped");
        assert_eq!(frame(360.5, 36), 35);
        assert_eq!(frame(-10.0, 36), 0, "clamped");
        assert_eq!(frame(-1.5, 36), 0);
        assert_eq!(frame(-0.000_001, 36), 0);
        assert_eq!(frame(200.0, 1), 0);
        assert_eq!(frame(100.0, 2), 0);
        assert_eq!(frame(190.0, 2), 1);
        assert_eq!(
            shot_frame(185.0, 9, n(36), &plain, 20, 3),
            18,
            "a shot that does not spin ignores its start and its timer"
        );
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

    /// The frames shown at `ages` of a shot of `look` on a sheet of
    /// `frames`, started on index `start`. A shot is drawn at age `k` on
    /// its `k`-th pass, from age 1.
    fn ages_from(
        look: &WeaponLook,
        frames: u16,
        start: u8,
        ages: std::ops::Range<u32>,
    ) -> Vec<u16> {
        timed(look, frames, (start, 0), ages)
    }

    /// [`ages_from`] for a shot that started on spin timer `timer`.
    fn timed(
        look: &WeaponLook,
        frames: u16,
        (start, timer): (u8, u32),
        ages: std::ops::Range<u32>,
    ) -> Vec<u16> {
        ages.map(|age| shot_frame(90.0, age, n(frames), look, start, timer))
            .collect()
    }

    fn ages(look: &WeaponLook, frames: u16, ages: std::ops::Range<u32>) -> Vec<u16> {
        ages_from(look, frames, 0, ages)
    }

    #[test]
    fn a_spinning_shot_turns_a_frame_every_beam_width_ticks_and_wraps() {
        assert_eq!(ages(&spinning(2, 0, 0), 3, 1..9), [0, 0, 1, 1, 2, 2, 0, 0]);
        assert_eq!(
            ages(&spinning(0, 0, 0), 3, 1..5),
            [0, 1, 2, 0],
            "every tick"
        );
        assert_eq!(ages(&spinning(-1, 0, 0), 3, 1..5), [0, 1, 2, 0]);
        assert_eq!(ages(&spinning(1, 0, 0), 3, 1..5), [0, 1, 2, 0]);
        assert_eq!(
            ages(&spinning(1, 0, 0), 3, 0..2),
            [0, 0],
            "its first pass shows its start, before any step"
        );
    }

    #[test]
    fn a_spinning_shot_may_hold_its_last_frame() {
        assert_eq!(
            ages(&spinning(1, HOLDS_LAST_FRAME, 0), 3, 1..7),
            [0, 1, 2, 2, 2, 2]
        );
    }

    #[test]
    fn a_spinning_shot_runs_on_from_its_start() {
        // Start 3 on 8 frames, a step every 2 passes.
        assert_eq!(
            ages_from(&spinning(2, 0, 0), 8, 3, 1..13),
            [3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 0, 0]
        );
        assert_eq!(
            ages_from(&spinning(2, HOLDS_LAST_FRAME, 0), 8, 3, 1..13),
            [3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7]
        );
    }

    #[test]
    fn a_start_past_the_sheet_shows_the_last_frame_until_the_first_step() {
        // Start 20 on 8 frames: clamped to 7 until the first step, which
        // wraps it to 0 (@0x358cb-0x358f6).
        assert_eq!(
            ages_from(&spinning(2, 0, 0), 8, 20, 1..21),
            [7, 7, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 0, 0]
        );
        assert_eq!(
            ages_from(&spinning(2, HOLDS_LAST_FRAME, 0), 8, 20, 1..7),
            [7, 7, 7, 7, 7, 7],
            "or holds the last"
        );
        assert_eq!(
            ages_from(&spinning(1, 0, 0), 8, 8, 1..4),
            [7, 0, 1],
            "a start of the frame count is past the sheet"
        );
    }

    #[test]
    fn a_spinning_shot_steps_first_when_the_timer_it_started_on_runs_out() {
        // A timer of 2 at `BeamWidth` 4 steps on pass 2, shown from age 3,
        // then every 4; from 0, on pass 4, shown from age 5.
        let look = spinning(4, 0, 0);
        assert_eq!(timed(&look, 8, (0, 2), 1..8), [0, 0, 1, 1, 1, 1, 2]);
        assert_eq!(timed(&look, 8, (0, 0), 1..8), [0, 0, 0, 0, 1, 1, 1]);
        assert_eq!(
            timed(&spinning(3, 0, 0), 8, (0, 5), 1..6),
            [0, 1, 1, 1, 2],
            "a timer past BeamWidth steps on the first pass"
        );
        assert_eq!(
            timed(&spinning(4, 0, 0), 8, (6, 3), 1..4),
            [6, 7, 7],
            "from its start"
        );
    }

    #[test]
    fn put_back_below_its_prox_safety_a_shot_counts_steps_from_its_timer() {
        // `BeamWidth` 2 and a timer of 1: steps on passes 1, 3, 5 and 7,
        // the index put back to 0 below the safety of 4.
        assert_eq!(
            timed(
                &spinning(2, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 4),
                8,
                (20, 1),
                1..10
            ),
            [0, 0, 0, 0, 0, 1, 1, 2, 2]
        );
    }

    #[test]
    fn a_spinning_shot_may_wait_for_its_prox_safety() {
        // Frame 0 below the safety, its index running on beneath it.
        assert_eq!(
            ages(&spinning(1, STILL_UNTIL_SAFE, 3), 4, 1..9),
            [0, 0, 2, 3, 0, 1, 2, 3]
        );
        assert_eq!(
            ages_from(&spinning(1, STILL_UNTIL_SAFE, 3), 4, 1, 1..6),
            [0, 0, 3, 0, 1],
            "from its start"
        );
        assert_eq!(
            ages(&spinning(1, 0, 3), 4, 1..4),
            [0, 1, 2],
            "only with the flag"
        );
        // With 0x0002 too, the index is put back to 0 on the passes below
        // the safety, so it runs from 0 at it.
        assert_eq!(
            ages(
                &spinning(1, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 3),
                3,
                1..8
            ),
            [0, 0, 0, 1, 2, 2, 2]
        );
        assert_eq!(
            ages_from(
                &spinning(2, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 4),
                8,
                20,
                1..10
            ),
            [0, 0, 0, 0, 1, 1, 2, 2, 3],
            "its start forgotten; the timer runs on: steps on passes 4, 6 and 8"
        );
        assert_eq!(
            ages_from(
                &spinning(1, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 1),
                4,
                2,
                1..4
            ),
            [2, 3, 3],
            "a safety of 1 puts nothing back"
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
        assert_eq!(plain(from, to, &beam(1, 0), 0), [(from, to, BEAM)]);
        assert_eq!(plain(from, to, &beam(0, 0), 0), [(from, to, BEAM)]);
    }

    #[test]
    fn a_beam_two_wide_adds_half_alpha_lines_along_its_minor_axis() {
        // Mostly horizontal: offsets go down y.
        let strokes = plain(at(0.0, 0.0), at(100.0, 10.0), &beam(2, 0), 0);
        assert_eq!(
            strokes,
            [
                (at(0.0, -1.0), at(100.0, 9.0), HALF),
                (at(0.0, 1.0), at(100.0, 11.0), HALF),
                (at(0.0, 0.0), at(100.0, 10.0), BEAM),
            ]
        );
        // Mostly vertical: offsets go along x.
        let strokes = plain(at(0.0, 0.0), at(10.0, -100.0), &beam(2, 0), 0);
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
        let strokes = plain(from, to, &beam(2, 0), 0);
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
        let strokes = plain(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 7), 0);
        let alphas: Vec<u8> = strokes.iter().map(|(_, _, color)| color.a).collect();
        assert_eq!(alphas, [127, 127, 71, 71, 255]);
    }

    #[test]
    fn a_wide_beams_outer_core_lines_vanish() {
        let strokes = plain(at(0.0, 0.0), at(100.0, 0.0), &beam(4, 0), 0);
        assert_eq!(strokes.len(), 3, "±2 and ±3 are clear: {strokes:?}");
    }

    #[test]
    fn a_falloff_adds_a_corona_fading_by_it_a_line() {
        let strokes = plain(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 8), 0);
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
        let slow = plain(at(0.0, 0.0), at(100.0, 0.0), &beam(2, 4), 0);
        let alphas: Vec<u8> = slow.iter().map(|(_, _, color)| color.a).collect();
        // Corona at ±2 to ±5 (16, 12, 8 and 4 32nds: 4 is above 2), then
        // the core.
        assert_eq!(alphas, [127, 127, 95, 95, 63, 63, 31, 31, 127, 127, 255]);
        assert_eq!(slow[0].0, at(0.0, -2.0));
        assert_eq!(slow[6].0, at(0.0, -5.0));
        let none = plain(at(0.0, 0.0), at(100.0, 0.0), &beam(1, 0), 0);
        assert_eq!(none.len(), 1, "no falloff, no corona");
    }

    #[test]
    fn a_fading_beam_draws_its_corona_at_falloff_plus_its_fade() {
        let alphas = |look: &WeaponLook, fade| -> Vec<u8> {
            plain(at(0.0, 0.0), at(100.0, 0.0), look, fade)
                .iter()
                .map(|(_, _, color)| color.a)
                .collect()
        };
        assert_eq!(alphas(&beam(1, 8), 0), [127, 127, 63, 63, 255]);
        // Falloff 12: 16, then 4 32nds.
        assert_eq!(alphas(&beam(1, 8), 4), [127, 127, 31, 31, 255]);
        assert_eq!(alphas(&beam(1, 8), 9), [127, 127, 255], "16 only");
        // None of its own: 16, 13, 10, 7 and 4 32nds by its fade of 3.
        assert_eq!(
            alphas(&beam(1, 0), 3),
            [127, 127, 103, 103, 79, 79, 55, 55, 31, 31, 255]
        );
        assert_eq!(alphas(&beam(1, 0), 0), [255], "nor any fade");
    }

    #[test]
    fn a_lightning_beam_is_its_zig_zag_on_the_rolls_given_and_has_no_corona() {
        let lightning = WeaponLook {
            li_density: 3,
            li_amplitude: 5,
            falloff: 8,
            ..beam(1, 8)
        };
        let (from, to) = (at(0.0, 0.0), at(300.0, 20.0));
        let step = StepRolls::new(7);
        for rule in [RuleSource::Engine, RuleSource::Bible] {
            let mut rolls = step;
            let mut again = step;
            assert_eq!(
                beam_strokes(from, to, &lightning, 16, (rule, &mut rolls)),
                lightning::zig_zag(from, to, &lightning, 16, rule, &mut again),
                "{rule:?}"
            );
        }
        let mut rolls = step;
        let lines = beam_strokes(from, to, &lightning, 0, (RuleSource::Engine, &mut rolls));
        assert!(
            lines.iter().all(|&(.., color)| color == BEAM),
            "no corona: {lines:?}"
        );
        assert_eq!(lines.len(), 8, "9 steps: 8 segments, one line each");
    }

    #[test]
    fn each_beams_zig_zag_takes_its_own_rolls_of_the_step() {
        let lightning = WeaponLook {
            li_density: 3,
            li_amplitude: 5,
            ..beam(1, 0)
        };
        let looks = looks(&[(165, lightning.clone())]);
        let shown = |y| BeamShown {
            start: at(0.0, y),
            end: at(300.0, y),
            weapon: WeaponId(165),
            fade: 0,
        };
        let step = StepRolls::new(11);
        let mut list = DrawList::new();
        draw_beams(
            &mut list,
            &[shown(0.0), shown(50.0)],
            &looks,
            false,
            (RuleSource::Engine, step),
        );
        let mut expected = DrawList::new();
        for (n, y) in [0.0, 50.0].into_iter().enumerate() {
            let mut rolls = step.for_beam(n);
            let beam = shown(y);
            for (from, to, color) in lightning::zig_zag(
                beam.start,
                beam.end,
                &lightning,
                0,
                RuleSource::Engine,
                &mut rolls,
            ) {
                expected.line(from, to, 1.0, color);
            }
        }
        assert_eq!(list, expected);
    }

    fn looks(weapons: &[(i16, WeaponLook)]) -> Looks {
        Looks {
            smokes: BTreeMap::default(),
            weapons: weapons
                .iter()
                .map(|(id, look)| (WeaponId(*id), Ok(look.clone())))
                .collect(),
            booms: BTreeMap::new(),
            boxes: BTreeMap::new(),
            asteroids: BTreeMap::new(),
            cursor: None,
            pod: None,
            stars: None,
        }
    }

    fn sheet(image_id: i16, frames: u16) -> EffectSheet {
        EffectSheet {
            frame_width: 0,
            image_id,
            frames: n(frames),
        }
    }

    #[test]
    fn shots_are_drawn_as_their_frames_translucent_ones_added_to_what_is_under() {
        let looks = looks(&[
            (
                128,
                WeaponLook {
                    sheet: Some(Ok(sheet(3500, 36))),
                    ..WeaponLook::default()
                },
            ),
            (
                129,
                WeaponLook {
                    sheet: Some(Ok(sheet(3501, 36))),
                    flags3: TRANSLUCENT,
                    ..WeaponLook::default()
                },
            ),
            (130, WeaponLook::default()),
            (
                132,
                WeaponLook {
                    sheet: Some(Err("no spïn 3005".to_owned())),
                    ..WeaponLook::default()
                },
            ),
        ]);
        let shot = |x, weapon| ShotShown {
            id: ShotId(1),
            spin_start: 0,
            spin_timer: 0,
            murk: 0,
            at: at(x, 5.0),
            heading: 90.0,
            age: 3,
            weapon: WeaponId(weapon),
            layer: SpriteLayer::Shots,
        };
        let mut list = DrawList::new();
        draw_shots(
            &mut list,
            &[
                shot(1.0, 128),
                shot(2.0, 129),
                shot(3.0, 130),
                shot(4.0, 131),
                shot(5.0, 132),
            ],
            &looks,
            RuleSource::Engine,
        );
        let mut expected = DrawList::new();
        expected.sprite(ImageKey::sprite(3500, 9), at(1.0, 5.0), Color::WHITE);
        expected.or_sprite(ImageKey::sprite(3501, 9), at(2.0, 5.0), lights_tint(32));
        for x in [4.0, 5.0] {
            crossed_box(
                &mut expected,
                at(x, 5.0),
                SHOT_PLACEHOLDER_SIZE,
                PLACEHOLDER,
            );
        }
        assert_eq!(list, expected);
        assert_eq!(SHOT_PLACEHOLDER_SIZE, 8.0);
    }

    /// How a shot of `look` is drawn at `age`, `murk` 32nds murky, by
    /// `rule`: its tint and its blend.
    fn shaded(look: WeaponLook, age: u32, murk: u8, rule: RuleSource) -> (Color, Blend) {
        let looks = looks(&[(
            128,
            WeaponLook {
                sheet: Some(Ok(sheet(3500, 36))),
                ..look
            },
        )]);
        let shot = ShotShown {
            id: ShotId(1),
            at: at(0.0, 0.0),
            heading: 0.0,
            age,
            weapon: WeaponId(128),
            layer: SpriteLayer::Shots,
            spin_start: 0,
            spin_timer: 0,
            murk,
        };
        let mut list = DrawList::new();
        draw_shot(&mut list, &shot, &looks, rule);
        match list.iter().next() {
            Some(&DrawCommand::Sprite { tint, blend, .. }) => (tint, blend),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_shot_is_drawn_as_it_fades() {
        let fading = |flags3| WeaponLook {
            count: 40,
            shot_falloff: 2,
            flags3,
            ..WeaponLook::default()
        };
        let engine = RuleSource::Engine;
        assert_eq!(
            shaded(fading(0), 8, 0, engine),
            (Color::WHITE, Blend::Normal)
        );
        assert_eq!(
            shaded(fading(0), 25, 0, engine),
            (Color::rgba(255, 255, 255, 8), Blend::OrCrossfade),
            "16 ticks into its window: 2 × 16, translucent by 31 at most"
        );
        assert_eq!(
            shaded(fading(0), 12, 0, engine),
            (see_through(6), Blend::OrCrossfade),
            "translucent by 6, crossfaded by OR"
        );
        assert_eq!(see_through(6), Color::rgba(255, 255, 255, 207));
        assert_eq!(see_through(0), Color::WHITE);
        assert_eq!(
            shaded(fading(TRANSLUCENT), 10, 0, engine),
            (lights_tint(30), Blend::Or),
            "added at 30"
        );
        assert_eq!(
            shaded(fading(TRANSLUCENT), 5, 0, engine),
            (Color::WHITE, Blend::Normal),
            "opaque before its window, by the engine"
        );
        assert_eq!(
            shaded(fading(TRANSLUCENT), 5, 0, RuleSource::Bible),
            (lights_tint(32), Blend::Or),
            "added at 32, by the other reading"
        );
    }

    #[test]
    fn a_murky_shot_is_darkened_by_its_murk() {
        let plain = WeaponLook::default();
        let engine = RuleSource::Engine;
        assert_eq!(
            shaded(plain.clone(), 1, 16, engine),
            (Color::rgba(127, 127, 127, 255), Blend::Normal)
        );
        let added = WeaponLook {
            flags3: TRANSLUCENT,
            ..plain
        };
        assert_eq!(
            shaded(added, 1, 31, engine),
            (murk_tint(lights_tint(32), 31), Blend::Or)
        );
        assert_eq!(murk_tint(lights_tint(32), 31), Color::rgba(7, 7, 7, 255));
    }

    #[test]
    fn a_murky_shot_fading_plainly_is_crossfaded_at_its_darkened_tint() {
        let fading = WeaponLook {
            count: 40,
            shot_falloff: 2,
            ..WeaponLook::default()
        };
        assert_eq!(
            shaded(fading, 12, 16, RuleSource::Engine),
            (murk_tint(see_through(6), 16), Blend::OrCrossfade)
        );
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
            fade: 0,
        };
        let beams = [shown(146), shown(147), shown(148)];
        let lines = |under| {
            let mut list = DrawList::new();
            draw_beams(
                &mut list,
                &beams,
                &looks,
                under,
                (RuleSource::Engine, StepRolls::default()),
            );
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
