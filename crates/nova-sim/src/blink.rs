//! The running lights' blinking: how bright a ship's `shän` lights layer
//! is at a given tick, from its `BlinkMode` and `BlinkValA`–`D`.
//!
//! [`lights_level`] is a pure function of the [`Blink`] fields, the
//! elapsed time in 1/30 s ticks (the original's frame and the
//! simulation's [`TICK`](crate::TICK)) and a [`BlinkChance`] that random
//! mode rolls on. It follows the original's per-tick update in
//! `_HandleShipDisplay` (see `nova_data::records::ship_anim::ShipAnim`'s
//! blink fields for the semantics and their sources), but answers for any
//! tick without replaying from tick 0, so its cost never grows with the
//! elapsed time and the same tick always gives the same answer at any
//! frame rate.

/// The lights' full intensity: drawn as they are, added at full strength.
pub const FULL: u8 = 32;

/// A `shän`'s raw blink fields: `BlinkMode` and `BlinkValA`–`D`. The
/// load-time clamps the original applies are applied by [`lights_level`],
/// not stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Blink {
    /// `BlinkMode`: 1 square wave, 2 triangle wave, 3 random; anything
    /// else steady.
    pub mode: i16,
    /// `BlinkValA`.
    pub a: i16,
    /// `BlinkValB`.
    pub b: i16,
    /// `BlinkValC`.
    pub c: i16,
    /// `BlinkValD`.
    pub d: i16,
}

impl Blink {
    /// Lights that never blink: steady at [`FULL`].
    pub const STEADY: Self = Self {
        mode: 0,
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    };
}

/// Random mode's source of variation, keyed by which change it is rather
/// than drawn from a stream, so an intensity stays a pure function of the
/// tick.
pub trait BlinkChance {
    /// Which of `sides` equal outcomes the `change`-th intensity change
    /// (counting from 0) takes: 0 to `sides - 1`. The same question always
    /// gets the same answer.
    fn roll(&self, change: u64, sides: u16) -> u16;
}

/// A borrowed source is a source.
impl<T: BlinkChance + ?Sized> BlinkChance for &T {
    fn roll(&self, change: u64, sides: u16) -> u16 {
        (**self).roll(change, sides)
    }
}

/// `SplitMix64`'s increment, the golden ratio's fraction in 64 bits.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// The deterministic [`BlinkChance`]: each change's roll is a hash of the
/// seed and the change, `SplitMix64`'s finaliser over `seed + (change + 1)
/// x GOLDEN`, so change `n` from seed 0 is `SplitMix64`'s `n + 1`-th draw
/// from seed 0. The original's `_Rand` is uniform over the same outcomes
/// from another generator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HashedRolls {
    seed: u64,
}

impl HashedRolls {
    /// The rolls from `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { seed }
    }
}

impl BlinkChance for HashedRolls {
    fn roll(&self, change: u64, sides: u16) -> u16 {
        if sides == 0 {
            return 0;
        }
        let mut z = self
            .seed
            .wrapping_add(change.wrapping_add(1).wrapping_mul(GOLDEN));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z % u64::from(sides)) as u16
    }
}

/// The lights' level at `tick` (ticks since the ship appeared, the first
/// being tick 0): `Some(1..=32)` out of [`FULL`] when they are drawn,
/// `None` when they are not.
#[must_use]
pub fn lights_level(blink: &Blink, tick: u64, chance: &(impl BlinkChance + ?Sized)) -> Option<u8> {
    match blink.mode {
        1 => square_lit(blink, tick).then_some(FULL),
        2 => level(triangle(blink, tick)),
        3 => level(random(blink, tick, chance)),
        _ => Some(FULL),
    }
}

/// How an intensity is drawn: not at all unless it is above 1, else at
/// its whole part, at most [`FULL`] (only data outside the Bible's ranges
/// goes higher; the original's lights path has no cap).
fn level(intensity: f32) -> Option<u8> {
    // `as` truncates towards zero, like the original's `cvttss2si`.
    (intensity > 1.0).then(|| intensity.min(f32::from(FULL)) as u8)
}

/// Whether a square wave (mode 1) is lit at `tick`.
///
/// The original's timer is clamped to at most M = max(B, D) each tick,
/// then counts down a tick at a time, and the light switches on the tick
/// after it goes below 0, without counting down. So a phase loaded with
/// `v` lasts `max(min(v, M), -1) + 2` ticks. Lit loads B, dark between
/// blinks loads A, and after the C-th blink's dark the gap loads D. The
/// timer starts at 0, so the first blink waits a phase loaded with 0,
/// less a tick.
fn square_lit(blink: &Blink, tick: u64) -> bool {
    let longest = i64::from(blink.b.max(blink.d));
    let phase = |load: i16| -> u64 { (i64::from(load).min(longest).max(-1) + 2) as u64 };
    let (on, off, gap) = (phase(blink.b), phase(blink.a), phase(blink.d));
    let first = phase(0) - 1;
    let blinks = u64::try_from(blink.c).unwrap_or(0);
    let group = blinks * (on + off);
    let Some(since) = tick.checked_sub(first) else {
        return false;
    };
    let at = since % (group + gap);
    at < group && at % (on + off) < on
}

/// The most ticks a triangle wave's first rise is replayed for; a rise
/// that has not turned by then holds where it got to. Every ramp within
/// the Bible's ranges (bounds 1 to 32, steps of at least 0.01) turns within
/// 3,101 ticks, so only data outside them is ever held: a ramp that never
/// reaches its bound (B or D of 0 or less), where holding draws what the
/// original draws, or one that runs from far outside the drawn range,
/// whose return to it the original would show hours later.
const RISE_LIMIT: u64 = 4096;

/// The most ticks one period of a triangle wave (a fall, a turn, a rise
/// and a turn) is replayed for, likewise: two [`RISE_LIMIT`] ramps and
/// their turns.
const PERIOD_LIMIT: u64 = 8194;

/// A triangle wave's bounds and steps, as the original holds them.
struct Ramp {
    /// A, the bottom.
    bottom: f32,
    /// C, clamped to at most 31 at load, the top.
    top: f32,
    /// B hundredths, added each rising tick.
    rise: f64,
    /// D hundredths, taken away each falling tick.
    fall: f64,
}

/// A triangle wave's state between ticks: the original's per-ship
/// intensity and direction.
#[derive(Clone, Copy)]
struct Wave {
    intensity: f32,
    rising: bool,
}

impl Wave {
    /// The original's start: intensity 0, rising.
    const START: Self = Self {
        intensity: 0.0,
        rising: true,
    };

    /// One tick, as `_HandleShipDisplay` 0x2c41b–0x2c4b6 runs it: each
    /// bound is checked before the step, and each step is added in double
    /// precision and stored as f32. Whether it turned at a bound.
    fn step(&mut self, ramp: &Ramp) -> bool {
        let (reached, bound, step) = if self.rising {
            (self.intensity >= ramp.top, ramp.top, ramp.rise)
        } else {
            (ramp.bottom >= self.intensity, ramp.bottom, ramp.fall)
        };
        if reached {
            self.intensity = bound;
            self.rising = !self.rising;
        } else {
            self.intensity = (f64::from(self.intensity) + step) as f32;
        }
        reached
    }
}

/// A triangle wave's (mode 2) intensity after `tick`'s update.
///
/// The original's f32 arithmetic is replayed, but only so far: the first
/// rise to its turn at the top, then one period from there, back to a
/// turn at the top. Each turn sets the intensity to the bound exactly, so
/// the wave repeats exactly from its first turn, and a later tick is that
/// many ticks into the period. A replay that runs past [`RISE_LIMIT`] or
/// [`PERIOD_LIMIT`] without its turn holds where it got to from then on.
fn triangle(blink: &Blink, tick: u64) -> f32 {
    let ramp = Ramp {
        bottom: f32::from(blink.a),
        top: f32::from(blink.c.min(31)),
        rise: f64::from(blink.b) * 0.01,
        fall: f64::from(blink.d) * -0.01,
    };
    let mut wave = Wave::START;
    let mut turn = None;
    for now in 0..RISE_LIMIT {
        let turned = wave.step(&ramp);
        if now == tick {
            return wave.intensity;
        }
        if turned {
            turn = Some(now);
            break;
        }
    }
    let Some(turned_at) = turn else {
        return wave.intensity;
    };
    let top = wave;
    let mut period = None;
    for since in 1..=PERIOD_LIMIT {
        let turned = wave.step(&ramp);
        if turned_at + since == tick {
            return wave.intensity;
        }
        if turned && wave.rising == top.rising {
            period = Some(since);
            break;
        }
    }
    let Some(period) = period else {
        return wave.intensity;
    };
    let mut wave = top;
    for _ in 0..(tick - turned_at) % period {
        wave.step(&ramp);
    }
    wave.intensity
}

/// A random wave's (mode 3) intensity at `tick`: A plus a roll of
/// `B - A + 1` sides (B clamped to at most 31 at load), rolled afresh
/// every `max(C, 0) + 1` ticks from tick 0. A top below the bottom is the
/// bottom, unrolled: the original's `_Rand` has no meaningful answer for
/// no sides.
fn random(blink: &Blink, tick: u64, chance: &(impl BlinkChance + ?Sized)) -> f32 {
    let bottom = i32::from(blink.a);
    let sides = i32::from(blink.b.min(31)) - bottom + 1;
    let Ok(sides) = u16::try_from(sides) else {
        return bottom as f32;
    };
    if sides == 0 {
        return bottom as f32;
    }
    let held = u64::try_from(blink.c).unwrap_or(0) + 1;
    let roll = chance.roll(tick / held, sides).min(sides - 1);
    (bottom + i32::from(roll)) as f32
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    /// Answers each change from a script (the last answer repeats past its
    /// end) and records every `(change, sides)` it is asked.
    #[derive(Default)]
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

        fn asked(&self) -> Vec<(u64, u16)> {
            self.asked.borrow().clone()
        }
    }

    impl BlinkChance for ScriptedRolls {
        fn roll(&self, change: u64, sides: u16) -> u16 {
            self.asked.borrow_mut().push((change, sides));
            let at = usize::try_from(change).unwrap_or(usize::MAX);
            self.answers
                .get(at)
                .or(self.answers.last())
                .copied()
                .unwrap_or(0)
        }
    }

    fn blink(mode: i16, a: i16, b: i16, c: i16, d: i16) -> Blink {
        Blink { mode, a, b, c, d }
    }

    /// The level at each tick in `ticks`, rolling on `chance`.
    fn levels(
        blink: Blink,
        ticks: impl IntoIterator<Item = u64>,
        chance: &impl BlinkChance,
    ) -> Vec<Option<u8>> {
        ticks
            .into_iter()
            .map(|tick| lights_level(&blink, tick, chance))
            .collect()
    }

    /// The ticks below `end` at which the lights are drawn at full.
    fn lit(blink: Blink, end: u64) -> Vec<u64> {
        let chance = ScriptedRolls::default();
        (0..end)
            .filter(|&tick| match lights_level(&blink, tick, &chance) {
                Some(FULL) => true,
                None => false,
                other => panic!("a square wave is on or off: {other:?} at {tick}"),
            })
            .collect()
    }

    // Steady.

    #[test]
    fn modes_other_than_one_to_three_hold_the_lights_steady_at_full() {
        let chance = ScriptedRolls::default();
        for mode in [0, -1, 4, 100, i16::MIN] {
            for blink in [blink(mode, 0, 0, 0, 0), blink(mode, 4, 1, 2, 20)] {
                assert_eq!(
                    levels(blink, [0, 1, 1_000_000, u64::MAX], &chance),
                    [Some(FULL); 4],
                    "{blink:?}"
                );
            }
        }
        assert_eq!(lights_level(&Blink::STEADY, 7, &chance), Some(FULL));
        assert_eq!(chance.asked(), []);
    }

    // Square wave.

    #[test]
    fn the_shuttle_double_flashes_every_40_ticks() {
        // Shuttle, `shän` 128: A=4 B=1 C=2 D=20.
        let shuttle = blink(1, 4, 1, 2, 20);
        assert_eq!(
            lit(shuttle, 53),
            [1, 2, 3, 10, 11, 12, 41, 42, 43, 50, 51, 52]
        );
        let chance = ScriptedRolls::default();
        let later = 40 * 1000;
        assert_eq!(lights_level(&shuttle, 1 + later, &chance), Some(FULL));
        assert_eq!(lights_level(&shuttle, 12 + later, &chance), Some(FULL));
        assert_eq!(lights_level(&shuttle, 13 + later, &chance), None);
        assert_eq!(lights_level(&shuttle, 40 + later, &chance), None);
        assert_eq!(chance.asked(), []);
    }

    #[test]
    fn b_is_the_on_time_a_the_off_time_c_the_blinks_and_d_the_gap() {
        // A zero on-time still lights for two ticks; three blinks a group.
        assert_eq!(
            lit(blink(1, 2, 0, 3, 5), 60),
            [1, 2, 7, 8, 13, 14, 26, 27, 32, 33, 38, 39, 51, 52, 57, 58]
        );
    }

    #[test]
    fn every_phase_is_clamped_to_the_longer_of_b_and_d() {
        // The off-time of 10 counts as 3.
        assert_eq!(
            lit(blink(1, 10, 1, 1, 3), 60),
            [1, 2, 3, 14, 15, 16, 27, 28, 29, 40, 41, 42, 53, 54, 55]
        );
    }

    #[test]
    fn a_negative_phase_lasts_a_tick() {
        // B = -4, D = 0: on for one tick, off (A = 1, clamped to 0) for
        // two, the gap (0) two more.
        assert_eq!(
            lit(blink(1, 1, -4, 2, 0), 30),
            [1, 4, 9, 12, 17, 20, 25, 28]
        );
        // Every phase negative: each lasts one tick, and the first blink
        // needs no wait.
        assert_eq!(lit(blink(1, -2, -3, 1, -5), 6), [0, 3]);
    }

    #[test]
    fn no_blinks_a_group_never_lights() {
        assert!(lit(blink(1, 4, 1, 0, 20), 60).is_empty());
        assert!(lit(blink(1, 4, 1, -3, 20), 60).is_empty());
    }

    #[test]
    fn the_longest_square_wave_does_not_overflow() {
        let longest = blink(1, i16::MAX, i16::MAX, i16::MAX, i16::MAX);
        let chance = ScriptedRolls::default();
        assert_eq!(lights_level(&longest, 0, &chance), None);
        assert_eq!(lights_level(&longest, 1, &chance), Some(FULL));
        assert_eq!(lights_level(&longest, 32_769, &chance), Some(FULL));
        assert_eq!(lights_level(&longest, 32_770, &chance), None);
        let _ = lights_level(&longest, u64::MAX, &chance);
    }

    // Triangle wave.

    /// The 51 stock ships' triangle: 10 to 32 (31 once loaded), up and
    /// down 0.75 a tick.
    const STOCK_TRIANGLE: Blink = Blink {
        mode: 2,
        a: 10,
        b: 75,
        c: 32,
        d: 75,
    };

    #[test]
    fn the_stock_triangle_pulses_between_10_and_31_every_58_ticks() {
        let chance = ScriptedRolls::default();
        let pinned = [
            (0, None),
            (1, Some(1)),
            (2, Some(2)),
            (41, Some(31)),
            (42, Some(31)),
            (43, Some(30)),
            (70, Some(10)),
            (71, Some(10)),
            (72, Some(10)),
            (73, Some(11)),
            (99, Some(31)),
            (100, Some(31)),
            (101, Some(30)),
        ];
        for (tick, level) in pinned {
            assert_eq!(
                lights_level(&STOCK_TRIANGLE, tick, &chance),
                level,
                "{tick}"
            );
        }
        for tick in 43..300 {
            let level = lights_level(&STOCK_TRIANGLE, tick, &chance);
            assert_eq!(
                lights_level(&STOCK_TRIANGLE, tick + 58, &chance),
                level,
                "{tick}"
            );
            assert!(level.is_some_and(|l| (10..=31).contains(&l)), "{tick}");
        }
        assert_eq!(chance.asked(), []);
    }

    #[test]
    fn the_krypt_pod_overshoots_and_keeps_the_originals_float_drift() {
        // Krypt Pod, `shän` 176: 10 to 32, up 1.6 and down 0.85 a tick.
        // The levels the original's f32 arithmetic gives, all drawn (the
        // first is 1.6).
        let pod = blink(2, 10, 160, 32, 85);
        let first: Vec<Option<u8>> = [
            1, 3, 4, 6, 8, 9, 11, 12, 14, 16, 17, 19, 20, 22, 24, 25, 27, 28, 30, 32, 31, 30, 29,
            28, 27, 26, 25, 25, 24, 23, 22, 21, 20, 19, 19, 18, 17, 16, 15, 14, 13, 13, 12, 11, 10,
            9, 10, 11, 13, 14, 16, 18, 19, 21, 22, 24, 26, 27, 29, 30, 32, 31,
        ]
        .into_iter()
        .map(Some)
        .collect();
        let chance = ScriptedRolls::default();
        assert_eq!(levels(pod, 0..62, &chance), first);
        // Exact hundredths would show 14 here; f32 drift shows 13.
        for tick in [40, 81, 122, 40 + 41 * 10_000] {
            assert_eq!(lights_level(&pod, tick, &chance), Some(13), "{tick}");
        }
        assert_eq!(lights_level(&pod, 19 + 41 * 10_000, &chance), Some(FULL));
    }

    #[test]
    fn a_rise_of_nothing_never_lights() {
        let flat = Blink {
            b: 0,
            ..STOCK_TRIANGLE
        };
        let falling = Blink {
            b: -75,
            ..STOCK_TRIANGLE
        };
        let chance = ScriptedRolls::default();
        for tick in [0, 1, 41, 5000, u64::MAX] {
            assert_eq!(lights_level(&flat, tick, &chance), None, "{tick}");
            assert_eq!(lights_level(&falling, tick, &chance), None, "{tick}");
        }
    }

    #[test]
    fn a_fall_of_nothing_holds_at_the_top() {
        let held = Blink {
            d: 0,
            ..STOCK_TRIANGLE
        };
        let chance = ScriptedRolls::default();
        assert_eq!(lights_level(&held, 40, &chance), Some(30));
        for tick in [41, 42, 100, 5000, u64::MAX] {
            assert_eq!(lights_level(&held, tick, &chance), Some(31), "{tick}");
        }
    }

    #[test]
    fn a_fall_that_rises_climbs_past_full_and_stays_there() {
        // D < 0 adds as it falls: the level climbs from 31 and is drawn at
        // full from 32 on.
        let climbing = Blink {
            d: -75,
            ..STOCK_TRIANGLE
        };
        let chance = ScriptedRolls::default();
        assert_eq!(lights_level(&climbing, 42, &chance), Some(31));
        assert_eq!(lights_level(&climbing, 43, &chance), Some(31));
        assert_eq!(lights_level(&climbing, 44, &chance), Some(FULL));
        for tick in [100, 5000, 10_000, u64::MAX] {
            assert_eq!(lights_level(&climbing, tick, &chance), Some(FULL), "{tick}");
        }
    }

    #[test]
    fn a_bottom_above_the_top_alternates_between_them() {
        let chance = ScriptedRolls::default();
        let inverted = blink(2, 20, 75, 15, 75);
        assert_eq!(
            levels(inverted, 18..30, &chance),
            [14, 15, 15, 20, 15, 20, 15, 20, 15, 20, 15, 20].map(Some)
        );
        assert_eq!(lights_level(&inverted, 1_000_001, &chance), Some(20));
        assert_eq!(lights_level(&inverted, 1_000_000, &chance), Some(15));
    }

    #[test]
    fn a_top_at_or_below_nothing_turns_at_once() {
        // C = 0 peaks on tick 0 and falls straight to A = -5.
        let chance = ScriptedRolls::default();
        let low = blink(2, -5, 75, 0, 300);
        assert_eq!(levels(low, 0..3, &chance), [None, None, None]);
        let high = blink(2, 5, 300, -2, 300);
        // Peaks at -2, then 5, then rises from 5 (5 >= -2 at once).
        assert_eq!(levels(high, 0..4, &chance), [None, Some(5), None, Some(5)]);
    }

    #[test]
    fn a_top_above_31_is_clamped_to_31() {
        let chance = ScriptedRolls::default();
        let tall = Blink {
            c: 1000,
            ..STOCK_TRIANGLE
        };
        assert_eq!(
            levels(tall, 0..200, &chance),
            levels(STOCK_TRIANGLE, 0..200, &chance)
        );
        let at = Blink {
            c: 31,
            ..STOCK_TRIANGLE
        };
        assert_eq!(
            levels(at, 0..200, &chance),
            levels(STOCK_TRIANGLE, 0..200, &chance)
        );
        let below = Blink {
            c: 30,
            ..STOCK_TRIANGLE
        };
        assert_eq!(
            levels(below, 39..42, &chance),
            [Some(30), Some(30), Some(29)]
        );
    }

    #[test]
    fn the_slowest_triangle_in_the_bibles_ranges_repeats_every_6002_ticks() {
        // 1 to 31 at 0.01 a tick: a 3,000-tick fall and a 3,000-tick rise.
        let slowest = blink(2, 1, 1, 31, 1);
        let chance = ScriptedRolls::default();
        let pinned = [
            (99, None),
            (100, Some(1)),
            (3099, Some(31)),
            (3101, Some(30)),
            (6101, None),
            (6102, Some(1)),
            (9000, Some(29)),
            (15_104, Some(31)),
        ];
        for (tick, level) in pinned {
            assert_eq!(lights_level(&slowest, tick, &chance), level, "{tick}");
        }
        for tick in [3200, 6000, 7000, 9000, 9104] {
            assert_eq!(
                lights_level(&slowest, tick + 6002 * 1000, &chance),
                lights_level(&slowest, tick, &chance),
                "{tick}"
            );
        }
    }

    #[test]
    fn an_endless_triangle_still_answers_at_any_tick() {
        let chance = ScriptedRolls::default();
        let slowest = blink(2, i16::MIN, 1, 31, 1);
        let _ = lights_level(&slowest, u64::MAX, &chance);
        assert_eq!(lights_level(&slowest, 0, &chance), None);
        assert_eq!(lights_level(&slowest, 3099, &chance), Some(31));
        let extreme = blink(2, i16::MAX, i16::MIN, i16::MIN, i16::MAX);
        for tick in [0, 1, 2, 1_000_000, u64::MAX] {
            let _ = lights_level(&extreme, tick, &chance);
        }
    }

    // Random.

    #[test]
    fn random_mode_holds_each_roll_for_c_plus_one_ticks() {
        // A=5 B=12 C=3: 8 outcomes, each held 4 ticks.
        let chance = ScriptedRolls::new(&[3, 0, 7]);
        let random = blink(3, 5, 12, 3, 99);
        assert_eq!(
            levels(random, 0..12, &chance),
            [8, 8, 8, 8, 5, 5, 5, 5, 12, 12, 12, 12].map(Some)
        );
        let asked = chance.asked();
        assert!(
            asked
                .iter()
                .all(|&(change, sides)| change < 3 && sides == 8),
            "{asked:?}"
        );
        assert_eq!(
            asked.iter().map(|&(change, _)| change).collect::<Vec<_>>(),
            [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]
        );
    }

    #[test]
    fn a_random_tick_asks_once_and_always_gets_the_same_answer() {
        let chance = ScriptedRolls::new(&[3, 0, 7]);
        let random = blink(3, 5, 12, 3, 0);
        assert_eq!(lights_level(&random, 5, &chance), Some(5));
        assert_eq!(chance.asked(), [(1, 8)]);
        assert_eq!(lights_level(&random, 5, &chance), Some(5));
        assert_eq!(chance.asked(), [(1, 8), (1, 8)]);
        assert_eq!(lights_level(&random, u64::MAX, &chance), Some(12));
        assert_eq!(chance.asked()[2], (u64::MAX / 4, 8));
    }

    #[test]
    fn a_random_top_above_31_is_clamped_to_31() {
        let chance = ScriptedRolls::new(&[26]);
        assert_eq!(lights_level(&blink(3, 5, 40, 0, 0), 0, &chance), Some(31));
        assert_eq!(chance.asked(), [(0, 27)]);
        let chance = ScriptedRolls::new(&[26]);
        assert_eq!(lights_level(&blink(3, 5, 31, 0, 0), 0, &chance), Some(31));
        assert_eq!(chance.asked(), [(0, 27)]);
        let chance = ScriptedRolls::new(&[0]);
        let _ = lights_level(&blink(3, 5, 30, 0, 0), 0, &chance);
        assert_eq!(chance.asked(), [(0, 26)]);
    }

    #[test]
    fn a_delay_of_nothing_rolls_every_tick() {
        for c in [0, -1, i16::MIN] {
            let chance = ScriptedRolls::new(&[0, 1, 2, 3]);
            let random = blink(3, 5, 12, c, 0);
            assert_eq!(levels(random, 0..4, &chance), [5, 6, 7, 8].map(Some), "{c}");
        }
    }

    #[test]
    fn the_longest_delay_does_not_overflow() {
        let chance = ScriptedRolls::new(&[0, 1]);
        let random = blink(3, 5, 12, i16::MAX, 0);
        assert_eq!(lights_level(&random, 32_767, &chance), Some(5));
        assert_eq!(lights_level(&random, 32_768, &chance), Some(6));
    }

    #[test]
    fn a_roll_past_the_last_outcome_is_the_last() {
        let chance = ScriptedRolls::new(&[8, 200]);
        let random = blink(3, 5, 12, 0, 0);
        assert_eq!(levels(random, 0..2, &chance), [Some(12), Some(12)]);
    }

    #[test]
    fn a_random_top_below_the_bottom_is_the_bottom_without_rolling() {
        let chance = ScriptedRolls::new(&[3]);
        for b in [4, -100, i16::MIN] {
            let random = blink(3, 5, b, 0, 0);
            assert_eq!(levels(random, [0, 9], &chance), [Some(5); 2], "{b}");
        }
        assert_eq!(chance.asked(), []);
        // B = A has a single outcome, which is still rolled.
        let _ = lights_level(&blink(3, 5, 5, 0, 0), 0, &chance);
        assert_eq!(chance.asked(), [(0, 1)]);
    }

    #[test]
    fn a_random_level_of_one_or_less_is_not_drawn() {
        let random = blink(3, 0, 12, 0, 0);
        let chance = ScriptedRolls::new(&[1, 2, 0]);
        assert_eq!(levels(random, 0..3, &chance), [None, Some(2), None]);
        let below = blink(3, -40, 12, 0, 0);
        let chance = ScriptedRolls::new(&[0]);
        assert_eq!(lights_level(&below, 0, &chance), None);
        assert_eq!(chance.asked(), [(0, 53)]);
    }

    #[test]
    fn the_widest_random_range_fits_its_sides() {
        let chance = ScriptedRolls::new(&[32_799]);
        let widest = blink(3, i16::MIN, 31, 0, 0);
        assert_eq!(lights_level(&widest, 0, &chance), Some(31));
        assert_eq!(chance.asked(), [(0, 32_800)]);
    }

    // Hashed rolls.

    #[test]
    fn hashed_rolls_are_splitmix64s_draws() {
        // `SplitMix64` from seed 0: 0xE220A8397B1DCDAF, then
        // 0x6E789E6AA1B965F4.
        let rolls = HashedRolls::new(0);
        assert_eq!(rolls.roll(0, 100), 35);
        assert_eq!(rolls.roll(1, 100), 0);
        assert_eq!(rolls.roll(0, 8), 7);
        assert_eq!(
            rolls.roll(0, u16::MAX),
            (0xE220_A839_7B1D_CDAF_u64 % 65_535) as u16
        );
        assert_eq!(HashedRolls::default(), rolls);
    }

    #[test]
    fn hashed_rolls_depend_on_the_seed_and_the_change_alone() {
        let draws = |seed| (0..20).map(move |change| HashedRolls::new(seed).roll(change, 1000));
        assert!(draws(42).eq(draws(42)));
        assert!(!draws(42).eq(draws(43)));
        let rolls = HashedRolls::new(9);
        assert_eq!(rolls.roll(5, 31), rolls.roll(5, 31));
        assert!(
            (0..10)
                .map(|c| rolls.roll(c, 1000))
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 5
        );
    }

    #[test]
    fn a_hashed_roll_is_one_of_its_sides() {
        let rolls = HashedRolls::new(7);
        for sides in [1, 2, 8, 27, 100] {
            assert!(
                (0..1000).all(|change| rolls.roll(change, sides) < sides),
                "{sides}"
            );
        }
        assert!((0..1000).all(|change| rolls.roll(change, 1) == 0));
        assert_eq!(rolls.roll(3, 0), 0, "no sides");
        let _ = rolls.roll(u64::MAX, 8);
    }

    #[test]
    fn a_borrowed_source_answers_as_the_source() {
        let chance = ScriptedRolls::new(&[4]);
        let borrowed: &dyn BlinkChance = &&chance;
        assert_eq!(borrowed.roll(2, 9), 4);
        assert_eq!(chance.asked(), [(2, 9)]);
        let random = blink(3, 5, 12, 0, 0);
        assert_eq!(lights_level(&random, 0, borrowed), Some(9));
    }
}
