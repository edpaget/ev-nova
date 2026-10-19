//! A ship's running lights as a state, as `_HandleShipDisplay`
//! (@0x2c2ea-0x2c5a6) steps them each frame in the `EV Nova` executable.
//!
//! The lights are a state machine over three of the ship's words: the
//! intensity +0x60 (f32), +0xc8d6, the square wave's blink count or the
//! triangle's direction, and the blink timer +0xc8ec (f32) ([`ShipLights`]).
//! Each frame, for a class with a lights sprite (`LightImageID` above 0,
//! [`FrameSpec::lights`], @0x2b681-0x2b6a2, gate @0x2c2dd), its `shän`'s
//! `BlinkMode` steps them by `_gSpeedMult` ([`SPEED_MULT`],
//! [`ShipLights::step`]):
//!
//! - **1, square** (@0x2c31a-0x2c40f): the timer is first held to at most
//!   max(B, D). While the count is below C, a timer below 0 turns a lit
//!   light (intensity above 0) dark, the count up by one and the timer A,
//!   or a dark one lit, the intensity 32 and the timer B; a timer of 0 or
//!   more falls. At a count of C or more, a timer of 0 or more falls, and
//!   one below 0 starts the gap: the count 0, the intensity 0, the timer D.
//! - **2, triangle** (@0x2c411-0x2c4bb): rising (+0xc8d6 0), at or above
//!   the top C it is set to C and turns; else it rises B hundredths.
//!   Falling, at or below the bottom A it is set to A and turns; else it
//!   falls D hundredths. Each step is added in double precision and stored
//!   as f32.
//! - **3, random** (@0x2c4c0-0x2c563): a timer at 0 or below rolls the
//!   intensity A + `_Rand(B − A + 1)` (@0x2c4f3) and loads C; else the timer
//!   falls and is held to at most C. A top below the bottom is the bottom,
//!   unrolled, as `nova_sim::blink` reads it.
//! - **any other**: the intensity 32 every frame (@0x2c565).
//!
//! C (the triangle's top) and B (random's top) are held to at most 31 as
//! the original loads them. Then **lights out** ([`lights_out`]): a ship
//! of `shän` `Flags` 0x0040 that is disabled has its intensity written 0
//! (@0x2c56c-0x2c593), and the layer is drawn only while the intensity is
//! above 1.0, at its whole part ([`level`], @0x2c59a-0x2c5a6).
//!
//! From rest (intensity 0, rising, timer 0) the state draws what
//! `nova_sim::lights_level` solves per tick. After a lights out it does
//! not: by [`RuleKey::LightsOutZeroesBlink`](crate::RuleKey::LightsOutZeroesBlink)'s
//! engine reading the intensity written 0 is the state's, so a repaired
//! square wave waits out its timer with its count held, a triangle climbs
//! back from 0, a random light waits for its next roll and a steady one
//! is back at once ([`ShipFrames::light`]). By the other reading the
//! layer is only hidden and the state steps on as if lit.
//!
//! Random mode rolls on the display's chance; no sound plays in any of it.

use nova_sim_base::blink::{Blink, FULL, level};

use super::ship_frames::{FrameSpec, LIGHTS_OUT_WHILE_DISABLED, Moment, SPEED_MULT, ShipFrames};
use crate::rulebook::RuleSource;

/// The most a triangle's top C or a random top B is loaded as (the
/// original's load-time clamp).
pub const TOP: i16 = 31;

/// A triangle's rise per B (0.01 @0xdd098).
const RISE_STEP: f64 = 0.01;

/// A triangle's fall per D (-0.01 @0xdd580).
const FALL_STEP: f64 = -0.01;

/// A ship's lights state, each field the original's (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipLights {
    /// The intensity, +0x60.
    pub intensity: f32,
    /// +0xc8d6: a square wave's blink count, a triangle's direction (0
    /// rising, else falling).
    pub count: i16,
    /// The blink timer, +0xc8ec, which no routine but the lights block
    /// writes.
    pub timer: f32,
}

impl ShipLights {
    /// One frame of the lights block for `blink`, random mode rolling on
    /// `roll` (sides in, outcome out), before lights out (see the module
    /// docs).
    pub fn step(&mut self, blink: &Blink, roll: impl FnOnce(u16) -> u16) {
        match blink.mode {
            1 => self.square(blink),
            2 => self.triangle(blink),
            3 => self.random(blink, roll),
            _ => self.intensity = f32::from(FULL),
        }
    }

    /// Lights out: the intensity written 0 (@0x2c593).
    pub fn put_out(&mut self) {
        self.intensity = 0.0;
    }

    /// The level the intensity is drawn at, if drawn ([`level`]).
    #[must_use]
    pub fn level(&self) -> Option<u8> {
        level(self.intensity)
    }

    /// The timer a frame lower, in double precision (@0x2c3fe).
    fn fall(&mut self) {
        self.timer = (f64::from(self.timer) - f64::from(SPEED_MULT)) as f32;
    }

    /// Mode 1 (@0x2c31a-0x2c40f).
    fn square(&mut self, blink: &Blink) {
        self.timer = self.timer.min(f32::from(blink.b.max(blink.d)));
        if self.count < blink.c {
            if self.timer < 0.0 {
                if self.intensity > 0.0 {
                    self.count += 1;
                    self.intensity = 0.0;
                    self.timer = f32::from(blink.a);
                } else {
                    self.intensity = f32::from(FULL);
                    self.timer = f32::from(blink.b);
                }
            } else {
                self.fall();
            }
        } else if self.timer < 0.0 {
            self.count = 0;
            self.intensity = 0.0;
            self.timer = f32::from(blink.d);
        } else {
            self.fall();
        }
    }

    /// Mode 2 (@0x2c411-0x2c4bb).
    fn triangle(&mut self, blink: &Blink) {
        let step = if self.count == 0 {
            let top = f32::from(blink.c.min(TOP));
            if self.intensity >= top {
                self.intensity = top;
                self.count = 1;
                return;
            }
            f64::from(blink.b) * RISE_STEP
        } else {
            let bottom = f32::from(blink.a);
            if bottom >= self.intensity {
                self.intensity = bottom;
                self.count = 0;
                return;
            }
            f64::from(blink.d) * FALL_STEP
        };
        // The step is B or D hundredths times `_gSpeedMult`, which is 1.0
        // ([`SPEED_MULT`]).
        self.intensity = (f64::from(self.intensity) + step) as f32;
    }

    /// Mode 3 (@0x2c4c0-0x2c563).
    fn random(&mut self, blink: &Blink, roll: impl FnOnce(u16) -> u16) {
        let hold = f32::from(blink.c);
        if self.timer > 0.0 {
            self.fall();
            self.timer = self.timer.min(hold);
            return;
        }
        let bottom = i32::from(blink.a);
        let sides = i32::from(blink.b.min(TOP)) - bottom + 1;
        let rolled = match u16::try_from(sides) {
            Ok(sides) if sides > 0 => i32::from(roll(sides).min(sides - 1)),
            _ => 0,
        };
        // Within an `i16` and 31 more: exact in an `f32`.
        #[allow(clippy::cast_precision_loss)]
        let intensity = (bottom + rolled) as f32;
        self.intensity = intensity;
        self.timer = hold;
    }
}

/// Whether a ship of `shän` `Flags` `flags` shows no lights, `disabled` or
/// not: of 0x0040 and disabled (`_IsDisabled`, @0x2c56c-0x2c591).
#[must_use]
pub fn lights_out(flags: u16, disabled: bool) -> bool {
    flags & LIGHTS_OUT_WHILE_DISABLED != 0 && disabled
}

impl ShipFrames {
    /// One frame of `_HandleShipDisplay`'s lights block for a ship of class
    /// `spec` in the `moment`, random mode rolling on `roll`: the level its
    /// lights layer is drawn at, if drawn. A class with no lights sprite
    /// steps nothing. Lights out writes the intensity 0 into the state
    /// by the engine's reading of its rule, and only hides the layer by
    /// the other (see the module docs).
    pub fn light(
        &mut self,
        spec: &FrameSpec,
        moment: &Moment,
        roll: impl FnOnce(u16) -> u16,
    ) -> Option<u8> {
        if !spec.lights {
            return None;
        }
        self.lights.step(&spec.blink, roll);
        if lights_out(spec.flags, moment.disabled) {
            if moment.rules.lights_zeroed == RuleSource::Engine {
                self.lights.put_out();
            }
            return None;
        }
        self.lights.level()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use nova_sim_base::blink::{BlinkChance, lights_level};

    use super::*;
    use crate::combat::armament::Fold;
    use crate::combat::ship_frames::{DisplayRules, Drawn};

    fn blink(mode: i16, a: i16, b: i16, c: i16, d: i16) -> Blink {
        Blink { mode, a, b, c, d }
    }

    /// The Shuttle's square wave, `shän` 128.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    /// The 51 stock ships' triangle.
    const TRIANGLE: Blink = Blink {
        mode: 2,
        a: 10,
        b: 75,
        c: 32,
        d: 75,
    };

    /// Answers each change from a script (the last answer repeats past its
    /// end), recording what it is asked.
    struct ScriptedRolls {
        answers: Vec<u16>,
        asked: RefCell<Vec<(u64, u16)>>,
    }

    impl ScriptedRolls {
        fn new(answers: &[u16]) -> Self {
            Self {
                answers: answers.to_vec(),
                asked: RefCell::default(),
            }
        }

        fn answer(&self, change: u64) -> u16 {
            let at = usize::try_from(change).unwrap_or(usize::MAX);
            self.answers
                .get(at)
                .or(self.answers.last())
                .copied()
                .unwrap_or(0)
        }
    }

    impl BlinkChance for ScriptedRolls {
        fn roll(&self, change: u64, sides: u16) -> u16 {
            self.asked.borrow_mut().push((change, sides));
            self.answer(change)
        }
    }

    /// The levels `lights` draws over `n` frames of `blink`, its `k`-th
    /// roll answered as `rolls` answers change `k`, each roll's sides
    /// recorded in `sides`.
    fn stepped(
        lights: &mut ShipLights,
        blink: &Blink,
        n: usize,
        rolls: &ScriptedRolls,
        made: &mut u64,
        sides: &mut Vec<u16>,
    ) -> Vec<Option<u8>> {
        (0..n)
            .map(|_| {
                lights.step(blink, |s| {
                    sides.push(s);
                    let answer = rolls.answer(*made);
                    *made += 1;
                    answer
                });
                lights.level()
            })
            .collect()
    }

    /// Steps from rest `n` frames, giving the levels and the sides asked.
    fn from_rest(blink: &Blink, n: usize, rolls: &ScriptedRolls) -> (Vec<Option<u8>>, Vec<u16>) {
        let mut lights = ShipLights::default();
        let mut sides = Vec::new();
        let levels = stepped(&mut lights, blink, n, rolls, &mut 0, &mut sides);
        (levels, sides)
    }

    #[test]
    fn stepping_from_rest_matches_lights_level_until_put_out() {
        let blinks = [
            SHUTTLE,
            TRIANGLE,
            blink(2, 10, 160, 32, 85),
            blink(3, 5, 12, 3, 99),
            blink(3, 5, 12, 0, 0),
            blink(3, 5, 12, -1, 0),
            blink(3, 5, 12, i16::MIN, 0),
            blink(3, 5, 40, 0, 0),
            blink(3, 5, 4, 0, 0),
            blink(3, 0, 12, 0, 0),
            blink(3, -40, 12, 0, 0),
            blink(3, i16::MIN, 31, 0, 0),
            Blink::STEADY,
            blink(-1, 4, 1, 2, 20),
            blink(1, 2, 0, 3, 5),
            blink(1, 10, 1, 1, 3),
            blink(1, 1, -4, 2, 0),
            blink(1, -2, -3, 1, -5),
            blink(1, 4, 1, 0, 20),
            blink(1, 4, 1, -3, 20),
            blink(1, i16::MAX, i16::MAX, i16::MAX, i16::MAX),
            blink(2, 20, 75, 15, 75),
            blink(2, 10, 0, 32, 75),
            blink(2, 10, -75, 32, 75),
            blink(2, 10, 75, 32, 0),
            blink(2, 10, 75, 32, -75),
            blink(2, -5, 75, 0, 300),
            blink(2, 5, 300, -2, 300),
            blink(2, 10, 75, 1000, 75),
            blink(2, 1, 1, 31, 1),
        ];
        let answers = [3, 0, 7, 26, 1, 200, 32_799, 5, 2, 9];
        for blink in blinks {
            let rolls = ScriptedRolls::new(&answers);
            let (levels, _) = from_rest(&blink, 2000, &rolls);
            let solved: Vec<Option<u8>> = (0..2000)
                .map(|tick| lights_level(&blink, tick, &rolls))
                .collect();
            assert_eq!(levels, solved, "{blink:?}");
        }
    }

    #[test]
    fn random_mode_rolls_the_sides_the_solver_asks() {
        let random = blink(3, 5, 40, 3, 0);
        let rolls = ScriptedRolls::new(&[26]);
        let (_, sides) = from_rest(&random, 9, &rolls);
        assert_eq!(sides, [27, 27, 27], "B held to 31: 31 - 5 + 1");
        let _ = lights_level(&random, 0, &rolls);
        assert_eq!(rolls.asked.borrow()[0], (0, 27));
        let (_, sides) = from_rest(&blink(3, 5, 4, 0, 0), 3, &rolls);
        assert_eq!(sides, Vec::<u16>::new(), "a top below the bottom: unrolled");
    }

    /// The levels over `n` frames with the ship disabled and its lights
    /// put out after each.
    fn out(lights: &mut ShipLights, blink: &Blink, n: usize) -> Vec<Option<u8>> {
        (0..n)
            .map(|_| {
                lights.step(blink, |_| 0);
                lights.put_out();
                lights.level()
            })
            .collect()
    }

    fn lit(lights: &mut ShipLights, blink: &Blink, n: usize) -> Vec<Option<u8>> {
        let rolls = ScriptedRolls::new(&[0]);
        stepped(lights, blink, n, &rolls, &mut 0, &mut Vec::new())
    }

    #[test]
    fn a_put_out_square_wave_holds_its_count_and_relights_after_its_timer() {
        let mut lights = ShipLights::default();
        assert_eq!(lit(&mut lights, &SHUTTLE, 2), [None, Some(FULL)]);
        // Out from tick 2 to 10: every expiry finds it dark and relights it
        // for B, so its count never moves.
        assert_eq!(out(&mut lights, &SHUTTLE, 9), [None; 9]);
        assert_eq!(lights.count, 0, "held");
        assert_eq!(lights.timer, 1.0, "relit on tick 10");
        // Repaired: dark until its timer runs out, then the wave goes on
        // from the held count: two blinks, then the gap.
        let after = lit(&mut lights, &SHUTTLE, 13);
        assert_eq!(
            after,
            [
                None,
                None,
                Some(FULL),
                Some(FULL),
                Some(FULL),
                None,
                None,
                None,
                None,
                None,
                None,
                Some(FULL),
                Some(FULL)
            ]
        );
        assert_eq!(lights.count, 1);
        // Never out, it would have blinked on its own cycle instead.
        let rolls = ScriptedRolls::new(&[0]);
        let never: Vec<Option<u8>> = (11..24)
            .map(|tick| lights_level(&SHUTTLE, tick, &rolls))
            .collect();
        assert_ne!(after, never);
    }

    #[test]
    fn a_put_out_square_wave_in_its_gap_starts_its_next_group_after_its_timer() {
        let mut lights = ShipLights {
            count: 2,
            timer: -1.0,
            ..ShipLights::default()
        };
        assert_eq!(out(&mut lights, &SHUTTLE, 1), [None]);
        assert_eq!((lights.count, lights.timer), (0, 20.0), "the gap");
    }

    #[test]
    fn a_put_out_triangle_climbs_back_from_nothing() {
        let rolls = ScriptedRolls::new(&[0]);
        for falling_when_out in [false, true] {
            let mut lights = ShipLights::default();
            // At its top and turned (tick 42) or rising (tick 30).
            let before = if falling_when_out { 44 } else { 30 };
            lit(&mut lights, &TRIANGLE, before);
            assert_eq!(lights.count, i16::from(falling_when_out));
            assert_eq!(out(&mut lights, &TRIANGLE, 2), [None, None]);
            assert_eq!(lights.count, 0, "rising from 0");
            let after = lit(&mut lights, &TRIANGLE, 100);
            assert_eq!(after[0], None, "dark for a step");
            assert_eq!(after[1], Some(1));
            assert_eq!(after[41], Some(31), "at its top 41 steps later");
            let from_rest: Vec<Option<u8>> = (0..100)
                .map(|tick| lights_level(&TRIANGLE, tick, &rolls))
                .collect();
            assert_eq!(after, from_rest, "as from rest");
        }
    }

    #[test]
    fn a_put_out_random_light_stays_dark_until_its_next_roll() {
        let random = blink(3, 5, 12, 3, 0);
        let mut lights = ShipLights::default();
        let rolls = ScriptedRolls::new(&[3, 6]);
        let mut made = 0;
        let mut sides = Vec::new();
        let first = stepped(&mut lights, &random, 2, &rolls, &mut made, &mut sides);
        assert_eq!(first, [Some(8), Some(8)]);
        // Out for a tick: rolled nothing new, the timer falls on.
        assert_eq!(out(&mut lights, &random, 1), [None]);
        assert_eq!(lights.timer, 1.0);
        let after = stepped(&mut lights, &random, 3, &rolls, &mut made, &mut sides);
        assert_eq!(
            after,
            [None, Some(11), Some(11)],
            "dark until it rolls again"
        );
        assert_eq!(sides, [8, 8]);
    }

    #[test]
    fn a_random_light_on_a_longer_timer_falls_a_frame_then_holds_to_c() {
        // A slot's last ship left its timer at 20; this one rolls every 4.
        let random = blink(3, 5, 12, 3, 0);
        let mut lights = ShipLights {
            timer: 20.0,
            ..ShipLights::default()
        };
        lights.step(&random, |_| 0);
        assert_eq!(lights.timer, 3.0, "19, held to C");
        lights.step(&random, |_| 0);
        assert_eq!(lights.timer, 2.0);
        // A square wave's timer is held to max(B, D) before it falls.
        let mut square = ShipLights {
            timer: 30.0,
            ..ShipLights::default()
        };
        square.step(&SHUTTLE, |_| 0);
        assert_eq!(square.timer, 19.0, "20, then a frame");
    }

    #[test]
    fn steady_lights_come_back_at_once() {
        let mut lights = ShipLights::default();
        assert_eq!(out(&mut lights, &Blink::STEADY, 3), [None; 3]);
        assert_eq!(lit(&mut lights, &Blink::STEADY, 1), [Some(FULL)]);
    }

    /// A class of `flags` with a lights sprite blinking `blink`.
    fn lights_class(flags: u16, blink: Blink) -> FrameSpec {
        FrameSpec {
            flags,
            blink,
            lights: true,
            ..FrameSpec::NONE
        }
    }

    fn moment(disabled: bool, lights_zeroed: RuleSource) -> Moment {
        Moment {
            disabled,
            key_aboard: false,
            rules: DisplayRules {
                lights_zeroed,
                ..DisplayRules::ENGINE
            },
        }
    }

    #[test]
    fn a_class_without_a_lights_image_never_steps() {
        let spec = FrameSpec {
            lights: false,
            ..lights_class(0, TRIANGLE)
        };
        let mut frames = ShipFrames::default();
        let lit = moment(false, RuleSource::Engine);
        for _ in 0..5 {
            assert_eq!(frames.light(&spec, &lit, |_| 0), None);
        }
        assert_eq!(frames.lights, ShipLights::default());
    }

    #[test]
    fn lights_out_zeroes_the_state_by_the_engine_and_only_hides_it_by_the_other_reading() {
        let spec = lights_class(LIGHTS_OUT_WHILE_DISABLED, TRIANGLE);
        let rolls = ScriptedRolls::new(&[0]);
        let as_if_lit = lights_level(&TRIANGLE, 26, &rolls);
        assert_eq!(as_if_lit, Some(20));
        for (reading, resumed) in [
            (RuleSource::Engine, Some(1)),
            (RuleSource::Bible, as_if_lit),
        ] {
            let mut frames = ShipFrames::default();
            for _ in 0..20 {
                frames.light(&spec, &moment(false, reading), |_| 0);
            }
            for _ in 0..5 {
                assert_eq!(frames.light(&spec, &moment(true, reading), |_| 0), None);
            }
            frames.light(&spec, &moment(false, reading), |_| 0);
            let level = frames.light(&spec, &moment(false, reading), |_| 0);
            assert_eq!(level, resumed, "{reading:?}");
        }
        // A ship of no 0x0040 keeps its lights disabled.
        let plain = lights_class(0, Blink::STEADY);
        let mut frames = ShipFrames::default();
        let disabled = moment(true, RuleSource::Engine);
        assert_eq!(frames.light(&plain, &disabled, |_| 0), Some(FULL));
    }

    #[test]
    fn the_lights_go_out_on_a_disabled_ship_of_0x0040() {
        assert!(lights_out(0x0040, true));
        assert!(lights_out(0x0058, true));
        assert!(!lights_out(0x0040, false));
        assert!(!lights_out(0x0018, true));
    }

    #[test]
    fn a_frame_draws_no_lights_of_its_own() {
        assert_eq!(Drawn::default().lights, None);
        let mut frames = ShipFrames::default();
        let drawn = frames.step(
            &lights_class(0, Blink::STEADY),
            &mut Fold::default(),
            moment(false, RuleSource::Engine),
        );
        assert_eq!(drawn.lights, None, "the lights block fills it");
    }
}
