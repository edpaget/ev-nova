//! A lightning beam's zig-zag, as `_DrawLightningBeam(port, port2,
//! start, end, BeamColor, BeamWidth, LiDensity, LiAmplitude, 32 − fade)`
//! draws it (@0xc3f9d-0xc48b0), called by `_BeamDrawCallback` for a beam
//! whose loaded `LiDensity` is above none (@0x37c75-0x37d62).
//!
//! - It has n = trunc(`LiDensity` × max(|dx|, |dy|) × 0.01) steps, over
//!   whole screen pixels, or 1 when that is none or less
//!   (@0xc40c4-0xc40ef, @0xc4833-0xc4866); a step is (end − start)/n in
//!   `f32`.
//! - Its points are the start, then for k = 1 to n − 2 the point
//!   start + k × step, truncated and jittered on both axes, each held
//!   on the minor axis within the start-to-end box grown by `BeamWidth` +
//!   a (@0xc41b8-0xc41e8, @0xc42e4-0xc430e), a being the loaded
//!   `LiAmplitude`; and the end. So it has n − 1 segments, the last two
//!   steps long, and none with n of 1 or less (@0xc420a-0xc4220).
//! - The jitter is `Random() % (2a + 1) − a` (@0xc47e4-0xc482e): the
//!   Toolbox's `Random()` runs −32767 to 32767, and the remainder keeps
//!   the dividend's sign (`idivl`), so the jitter runs −3a to a, leaning
//!   to negative x and y.
//! - Each segment is `BeamWidth` lines, the j-th (from 0) offset ±j
//!   across the minor axis (once for j = 0), at level
//!   max((32 − fade) − j × (32 / (`BeamWidth` + 1)), 1) 32nds, the
//!   division whole, in `BeamColor` (`_DrawTranslucentLine`
//!   @0xc4350-0xc44f5).
//!
//! The rulebook's [`RuleKey::LightningZigZag`](nova_sim::RuleKey::LightningZigZag)
//! chooses between the engine's reading, above, and the mended one: the
//! jitter −a to a, and a beam of one step or none drawn as one straight
//! segment. The Child's graviton beam (168, `LiDensity` 2 as loaded) draws
//! nothing under 100 pixels by the engine.
//!
//! The original draws the jitter anew every frame of its 30 a second; the
//! view draws it anew every step and holds it for the frames drawn in
//! that step ([`StepRolls`]).
//!
//! Left out: the clip test, which draws nothing when both ends lie beyond
//! the same edge of the port grown by 32 (@0xc4000-0xc406e), and the last
//! segment's flag to `_DrawTranslucentLine` (@0xc438e-0xc43a3): neither
//! changes what is seen.

use nova_sim::{Chance, RuleSource};

use super::catalog::WeaponLook;
use crate::{Color, Point};

/// What `LiDensity` × the beam's run is multiplied by for its steps (the
/// double 0.01).
pub const DENSITY_SCALE: f64 = 0.01;

/// The Toolbox's `Random()`'s reach either way.
pub const RANDOM_REACH: i32 = 32767;

/// A line's level is in 32nds.
const LEVELS: i32 = 32;

/// The jitter of a beam point of amplitude `amplitude`, drawn on
/// `chance`, by `rule` (see the module docs): −3a to a by the engine, −a
/// to a by the mended reading.
pub fn jitter(amplitude: i32, rule: RuleSource, chance: &mut dyn Chance) -> i32 {
    let a = amplitude.max(1);
    let sides = u32::try_from(2 * a + 1).unwrap_or(u32::MAX);
    match rule {
        RuleSource::Engine => {
            let reach = u32::try_from(2 * RANDOM_REACH + 1).unwrap_or(u32::MAX);
            let random = i64::from(chance.below(reach)) - i64::from(RANDOM_REACH);
            // The remainder keeps the dividend's sign, as `idivl` does.
            let rem = random % i64::from(2 * a + 1);
            i32::try_from(rem).unwrap_or(0) - a
        }
        RuleSource::Bible => i32::try_from(chance.below(sides)).unwrap_or(0) - a,
    }
}

/// How many steps a lightning beam of loaded `LiDensity` `density` takes
/// over `run`, its longer axis in whole pixels: at least 1.
#[must_use]
pub fn steps(density: i16, run: i32) -> i32 {
    let n = (f64::from(density) * f64::from(run) * DENSITY_SCALE).trunc();
    // A density of an `i16` over a screen's pixels: within an `i32`.
    #[allow(clippy::cast_possible_truncation)]
    let n = n as i32;
    n.max(1)
}

/// The lines of a lightning beam from `start` to `end`, of a weapon that
/// looks as `look` says, `fade` ticks into its fade, its jitter drawn on
/// `chance` by `rule` (see the module docs): each line's ends and colour.
pub fn zig_zag(
    start: Point,
    end: Point,
    look: &WeaponLook,
    fade: u32,
    rule: RuleSource,
    chance: &mut dyn Chance,
) -> Vec<(Point, Point, Color)> {
    // Whole screen pixels.
    #[allow(clippy::cast_possible_truncation)]
    let whole = |at: Point| (at.x as i32, at.y as i32);
    let (from, to) = (whole(start), whole(end));
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let count = steps(look.li_density, dx.abs().max(dy.abs()));
    let amplitude = i32::from(look.li_amplitude.max(1));
    let width = i32::from(look.beam_width);
    // Each segment is `BeamWidth` lines: none at all for a width of none
    // or less, whose shrunken box would hold no point.
    if width <= 0 {
        return Vec::new();
    }
    // A mostly vertical beam's minor axis is x, any other's y.
    let minor_x = dx.abs() < dy.abs();
    let mut points = vec![from];
    if count <= 1 {
        if rule == RuleSource::Engine {
            return Vec::new();
        }
    } else {
        let step = (dx as f32 / count as f32, dy as f32 / count as f32);
        let grown = width + amplitude;
        let (low, high) = if minor_x {
            (from.0.min(to.0) - grown, from.0.max(to.0) + grown)
        } else {
            (from.1.min(to.1) - grown, from.1.max(to.1) + grown)
        };
        for k in 1..count - 1 {
            #[allow(clippy::cast_possible_truncation)]
            let along = |from: i32, by: f32| (from as f32 + k as f32 * by) as i32;
            let mut x = along(from.0, step.0) + jitter(amplitude, rule, chance);
            let mut y = along(from.1, step.1) + jitter(amplitude, rule, chance);
            if minor_x {
                x = x.clamp(low, high);
            } else {
                y = y.clamp(low, high);
            }
            points.push((x, y));
        }
    }
    points.push(to);
    let base = LEVELS - i32::try_from(fade).unwrap_or(i32::MAX);
    let fall = LEVELS / (width + 1).max(1);
    let mut lines = Vec::new();
    for pair in points.windows(2) {
        let (head, tail) = (pair[0], pair[1]);
        for j in 0..width {
            let level = (base - j * fall).max(1);
            let offsets: &[i32] = if j == 0 { &[0] } else { &[j, -j] };
            for &off in offsets {
                let (ox, oy) = if minor_x { (off, 0) } else { (0, off) };
                #[allow(clippy::cast_precision_loss)]
                let point = |(x, y): (i32, i32)| Point::new((x + ox) as f32, (y + oy) as f32);
                lines.push((point(head), point(tail), shade(look.beam_color, level)));
            }
        }
    }
    lines
}

/// `00RRGGBB` `raw` at `level` 32nds opaque.
fn shade(raw: u32, level: i32) -> Color {
    let color = Color::from_rgb24(raw);
    let level = level.clamp(0, LEVELS);
    let alpha = u8::try_from(255 * level / LEVELS).unwrap_or(u8::MAX);
    Color::rgba(color.r, color.g, color.b, alpha)
}

/// The rolls a step's lightning is drawn on: a `SplitMix64` from a seed
/// the view draws once a step, so every frame drawn in a step draws the
/// same zig-zag, and the next step a new one. Each beam takes its own
/// stream ([`StepRolls::for_beam`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StepRolls {
    state: u64,
}

/// `SplitMix64`'s increment, the golden gamma.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

impl StepRolls {
    /// The rolls of a step whose seed is `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Draws a step's seed on `chance`.
    pub fn draw(chance: &mut dyn Chance) -> Self {
        let high = u64::from(chance.below(u32::MAX));
        let low = u64::from(chance.below(u32::MAX));
        Self::new(high << 32 | low)
    }

    /// The rolls of the step's `beam`-th beam.
    #[must_use]
    pub fn for_beam(self, beam: usize) -> Self {
        let beam = u64::try_from(beam).unwrap_or(u64::MAX);
        Self::new(mix(self.state ^ beam.wrapping_add(1).wrapping_mul(GOLDEN)))
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN);
        mix(self.state)
    }
}

/// `SplitMix64`'s finaliser.
const fn mix(z: u64) -> u64 {
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Chance for StepRolls {
    fn fires(&mut self, percent: u8) -> bool {
        self.below(100) < u32::from(percent)
    }

    fn below(&mut self, n: u32) -> u32 {
        u32::try_from(self.next() % u64::from(n.max(1))).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// Gives its draws in turn, then the middle of each range.
    struct Draws {
        draws: VecDeque<u32>,
        asked: Vec<u32>,
    }

    impl Draws {
        fn of(draws: &[u32]) -> Self {
            Self {
                draws: draws.iter().copied().collect(),
                asked: Vec::new(),
            }
        }
    }

    impl Chance for Draws {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.asked.push(n);
            self.draws.pop_front().unwrap_or(n / 2)
        }
    }

    const ENGINE: RuleSource = RuleSource::Engine;
    const MENDED: RuleSource = RuleSource::Bible;

    fn lightning(density: i16, amplitude: i16, width: i16) -> WeaponLook {
        WeaponLook {
            li_density: density,
            li_amplitude: amplitude,
            beam_width: width,
            beam_color: 0x00FF_8000,
            ..WeaponLook::default()
        }
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// The segments' ends among `lines`, j = 0's alone, for a beam one
    /// line wide.
    fn ends(lines: &[(Point, Point, Color)]) -> Vec<(Point, Point)> {
        lines.iter().map(|&(from, to, _)| (from, to)).collect()
    }

    #[test]
    fn a_beams_steps_are_its_density_times_its_run_over_100() {
        assert_eq!(steps(2, 150), 3);
        assert_eq!(steps(2, 99), 1, "1.98");
        assert_eq!(steps(2, 49), 1, "0.98: at least 1");
        assert_eq!(steps(10, 300), 30);
        assert_eq!(steps(0, 300), 1);
    }

    #[test]
    fn a_beam_of_one_step_draws_nothing_by_the_engine_and_a_straight_line_mended() {
        let graviton = lightning(2, 4, 1);
        let (from, to) = (at(0.0, 0.0), at(99.0, 0.0));
        assert_eq!(
            zig_zag(from, to, &graviton, 0, ENGINE, &mut Draws::of(&[])),
            []
        );
        let mended = zig_zag(from, to, &graviton, 0, MENDED, &mut Draws::of(&[]));
        assert_eq!(
            mended,
            [(from, to, Color::rgba(255, 128, 0, 255))],
            "one straight segment"
        );
    }

    #[test]
    fn a_beam_of_two_steps_is_one_segment_and_three_has_one_point_between() {
        let look = lightning(2, 4, 1);
        let mut none = Draws::of(&[]);
        let two = zig_zag(at(0.0, 0.0), at(100.0, 0.0), &look, 0, ENGINE, &mut none);
        assert_eq!(ends(&two), [(at(0.0, 0.0), at(100.0, 0.0))]);
        assert!(none.asked.is_empty(), "no jitter");
        // 150 pixels at 2: 3 steps of 50, a point at 50 jittered by the
        // middle draws: (32767 − 32767) % 9 − 4 = −4 on each axis.
        let mut middle = Draws::of(&[32767, 32767]);
        let three = zig_zag(at(0.0, 0.0), at(150.0, 0.0), &look, 0, ENGINE, &mut middle);
        assert_eq!(
            ends(&three),
            [
                (at(0.0, 0.0), at(46.0, -4.0)),
                (at(46.0, -4.0), at(150.0, 0.0))
            ],
            "the last segment two steps long"
        );
        assert_eq!(middle.asked, [65535, 65535]);
    }

    #[test]
    fn the_engines_jitter_runs_from_minus_3a_to_a_and_the_mended_from_minus_a() {
        let draw = |raw: u32, rule| jitter(4, rule, &mut Draws::of(&[raw]));
        // −32767 % 9 = −7, the dividend's sign kept: −7 − 4 = −11. The
        // lowest a remainder of 9 reaches is −8: −12 = −3a.
        assert_eq!(draw(0, ENGINE), -32767 % 9 - 4);
        let lowest = (0..65535).map(|raw| draw(raw, ENGINE)).min();
        let highest = (0..65535).map(|raw| draw(raw, ENGINE)).max();
        assert_eq!((lowest, highest), (Some(-12), Some(4)), "−3a to a");
        assert_eq!(draw(0, MENDED), -4, "−a");
        assert_eq!(draw(8, MENDED), 4, "a");
        let mut asked = Draws::of(&[0]);
        jitter(4, MENDED, &mut asked);
        assert_eq!(asked.asked, [9]);
        assert_eq!(
            jitter(0, MENDED, &mut Draws::of(&[0])),
            -1,
            "an amplitude of at least 1"
        );
    }

    #[test]
    fn a_point_is_held_on_the_minor_axis_within_the_grown_box() {
        // Mostly horizontal, 400 pixels at 1: 4 steps of 100, points at
        // 100 and 200; amplitude 2, width 1: the box grown by 3 on y,
        // from −3 to 3.
        let look = lightning(1, 2, 1);
        let (from, to) = (at(0.0, 0.0), at(400.0, 0.0));
        // The lowest jitter, −6 (−3a), on both axes: x is free, y held
        // at −3.
        let low = |raw| {
            let mut draws = Draws::of(&[raw, raw, raw, raw]);
            ends(&zig_zag(from, to, &look, 0, ENGINE, &mut draws))
        };
        let lowest = (0..65535)
            .min_by_key(|&raw| jitter(2, ENGINE, &mut Draws::of(&[raw])))
            .expect("a draw");
        assert_eq!(jitter(2, ENGINE, &mut Draws::of(&[lowest])), -6);
        let held = low(lowest);
        assert_eq!(held[0].1, at(94.0, -3.0), "y held at the box, x not");
        assert_eq!(held[1].1, at(194.0, -3.0));
        // A mostly vertical beam is held on x.
        let up = zig_zag(
            at(0.0, 0.0),
            at(0.0, 400.0),
            &look,
            0,
            ENGINE,
            &mut Draws::of(&[lowest; 4]),
        );
        assert_eq!(ends(&up)[0].1, at(-3.0, 94.0));
    }

    #[test]
    fn each_segment_is_its_width_in_lines_at_falling_levels() {
        let look = lightning(2, 1, 3);
        let levels = |fade| -> Vec<(f32, u8)> {
            zig_zag(
                at(0.0, 0.0),
                at(100.0, 0.0),
                &look,
                fade,
                ENGINE,
                &mut Draws::of(&[]),
            )
            .iter()
            .map(|&(from, _, color)| (from.y, color.a))
            .collect()
        };
        // 32 / 4 = 8 a line: 32, 24, 16 32nds.
        assert_eq!(
            levels(0),
            [(0.0, 255), (1.0, 191), (-1.0, 191), (2.0, 127), (-2.0, 127)]
        );
        // 2, then held at 1.
        assert_eq!(
            levels(30),
            [(0.0, 15), (1.0, 7), (-1.0, 7), (2.0, 7), (-2.0, 7)]
        );
        assert_eq!(levels(40)[0].1, 7, "at least 1");
        for width in [0, -5] {
            let none = lightning(10, 1, width);
            for rule in [ENGINE, MENDED] {
                let mut draws = Draws::of(&[]);
                let lines = zig_zag(at(0.0, 0.0), at(300.0, 0.0), &none, 0, rule, &mut draws);
                assert_eq!(lines, [], "no width, no lines: {width}");
            }
        }
    }

    #[test]
    fn a_point_jittered_up_the_minor_axis_stays_inside_the_box() {
        // The highest jitter, a = 2, on both axes, inside the box grown
        // by 3 either way.
        let look = lightning(1, 2, 1);
        let highest = (0..65535)
            .max_by_key(|&raw| jitter(2, ENGINE, &mut Draws::of(&[raw])))
            .expect("a draw");
        let mut draws = Draws::of(&[highest; 4]);
        let across = ends(&zig_zag(
            at(0.0, 0.0),
            at(400.0, 0.0),
            &look,
            0,
            ENGINE,
            &mut draws,
        ));
        assert_eq!(across[0].1, at(102.0, 2.0));
        let mut draws = Draws::of(&[highest; 4]);
        let up = ends(&zig_zag(
            at(0.0, 0.0),
            at(0.0, 400.0),
            &look,
            0,
            ENGINE,
            &mut draws,
        ));
        assert_eq!(up[0].1, at(2.0, 102.0));
    }

    #[test]
    fn a_diagonal_beam_offsets_its_lines_in_y() {
        let look = lightning(2, 1, 2);
        let mut draws = Draws::of(&[]);
        let lines = zig_zag(at(0.0, 0.0), at(100.0, 100.0), &look, 0, ENGINE, &mut draws);
        let starts: Vec<Point> = lines.iter().map(|&(from, ..)| from).collect();
        assert_eq!(
            starts,
            [at(0.0, 0.0), at(0.0, 1.0), at(0.0, -1.0)],
            "a tie is not mostly vertical"
        );
    }

    #[test]
    fn a_steps_rolls_are_splitmix64s() {
        // SplitMix64 from 0: its first output is 0xE220A8397B1DCDAF.
        assert_eq!(StepRolls::new(0).next(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(
            StepRolls::new(0).for_beam(0),
            StepRolls::new(0xE220_A839_7B1D_CDAF)
        );
        assert_eq!(
            StepRolls::new(5).for_beam(2),
            StepRolls::new(0xE1EB_9BB6_6446_0C41)
        );
        let first = StepRolls::new(9).below(100);
        let percent = |n: u32| u8::try_from(n).expect("at most 100");
        assert!(!StepRolls::new(9).fires(percent(first)), "below its draw");
        assert!(StepRolls::new(9).fires(percent(first + 1)));
    }

    #[test]
    fn a_mostly_vertical_beam_offsets_its_lines_in_x() {
        let look = lightning(2, 1, 2);
        let lines = zig_zag(
            at(10.0, 0.0),
            at(20.0, 100.0),
            &look,
            0,
            ENGINE,
            &mut Draws::of(&[]),
        );
        let starts: Vec<Point> = lines.iter().map(|&(from, ..)| from).collect();
        assert_eq!(starts, [at(10.0, 0.0), at(11.0, 0.0), at(9.0, 0.0)]);
    }

    #[test]
    fn a_steps_rolls_draw_the_same_zig_zag_twice_and_the_next_steps_anew() {
        let look = lightning(10, 5, 1);
        let (from, to) = (at(0.0, 0.0), at(400.0, 30.0));
        let draw = |mut rolls: StepRolls| zig_zag(from, to, &look, 0, ENGINE, &mut rolls);
        let step = StepRolls::draw(&mut Draws::of(&[1, 2]));
        assert_eq!(step, StepRolls::new(1 << 32 | 2));
        assert_eq!(
            draw(step.for_beam(0)),
            draw(step.for_beam(0)),
            "the same step"
        );
        assert_ne!(
            draw(step.for_beam(0)),
            draw(step.for_beam(1)),
            "another beam"
        );
        let next = StepRolls::draw(&mut Draws::of(&[3, 4]));
        assert_ne!(
            draw(step.for_beam(0)),
            draw(next.for_beam(0)),
            "the next step"
        );
        let mut rolls = StepRolls::new(9);
        assert!((0..100).all(|_| rolls.below(7) < 7));
        assert!(!StepRolls::new(9).fires(0));
        assert!(StepRolls::new(9).fires(100));
    }
}
