//! The hyperspace jump's effect: the stars streak, the screen fades to
//! white, and the new system fades in from white.
//!
//! The effect runs on display time, in three phases:
//!
//! 1. [`JumpPhase::Streak`], for [`STREAK_FOR`]: the stars stretch into
//!    lines trailing away from the jump's direction, growing to
//!    [`STREAK_LENGTH`] (times each star layer's parallax factor).
//! 2. [`JumpPhase::FadeOut`], for [`FADE_OUT_FOR`]: the streaked scene
//!    fades to white ([`FADE_COLOR`]).
//! 3. [`JumpPhase::FadeIn`], for [`FADE_IN_FOR`]: the new system fades in
//!    from white.
//!
//! The ship arrives between the fade-out and the fade-in, the instant
//! [`JumpEffect::advance`] reports.
//!
//! # Without the fades
//!
//! When the Hyperspace Effects preference is off ([`JumpEffect::with_fades`]
//! false), the stars still streak for [`STREAK_FOR`], the ship arrives as
//! the streak ends, and the screen shows solid white for one frame,
//! [`JumpPhase::Flash`] for [`ARRIVAL_FLASH_FOR`]; there is no fade-out or
//! fade-in.
//!
//! # The original's effect
//!
//! Checked against the original Mac executable (`EV Nova.app`, i386, read
//! with its symbols):
//!
//! - `_HandlePlayer` @0x6d29c-0x6d2bf: once the jump begins the ship
//!   accelerates away along its bearing, and `_hyperGamma` rises from its
//!   warp speed past a threshold (`(speed - 55) * 5`). The stars rush past
//!   because the ship itself moves; `_HandleStars` and `_ScrollStarfield`
//!   draw no streak lines. The ship is frozen while our effect plays, so
//!   the streak stands in for that motion.
//! - `_HandlePlayer` @0x683a4-0x68409: once `_hyperGamma` is positive it
//!   calls `_FadeWhiteIn` @0x546f, a `CGDisplayFade` of the whole display
//!   to white (rgb 1,1,1) over 1.5 s. It is a display fade, so the HUD
//!   whites out too; the flight view draws the fade over everything.
//! - @0x6be44-0x6bf3a: the arrival frame is painted solid white.
//! - `_HandlePlayer` @0x6840b-0x6841f: on the next frame `_FadeWhiteOut`
//!   @0x54d1 fades the display back from white over 1.5 s, which
//!   [`FADE_IN_FOR`] matches.
//!
//! The preference: `Keys.nib` binds the Hyperspace Effects check box to the
//! `HyperspaceEffects` default through `NSNegateBoolean`, so a set default
//! (settings+0x14) means "skip the effects", and a fresh install has them
//! on. All three engine tests of it skip only the white display fades:
//! `_HandlePlayer` @0x6d2a1 (`_hyperGamma` is not raised, though the ship
//! still accelerates away, so our streak stays), `_HandlePlayer` @0x683af
//! (no `_FadeWhiteIn`) and `_PlayerEnterHypergate` @0x63c30 (no
//! `_FadeWhiteOut`). The arrival frame @0x6be44 is painted white without
//! testing it, which [`JumpPhase::Flash`] stands for. The phase text's
//! other reading, no streak and arrival at once, is recorded in task
//! `bible-vs-engine-settings`.
//!
//! The black fades (`_FadeScreenOut`, `_FadeScreenIn`) serve death, the
//! intro and dialogs, never the jump. The streak and fade-out durations
//! are ours: the original's fade-out starts from a warp speed we do not
//! simulate, and the roadmap does not aim to match its timing. The Help
//! Book and the Bible do not describe the effect.

use std::time::Duration;

use nova_sim::Vec2;

use crate::draw::fill_rect;
use crate::geometry::Bounds;
use crate::system::camera::VIEW_SIZE;
use crate::{Color, DrawList, Point};

/// How long the stars streak.
pub const STREAK_FOR: Duration = Duration::from_secs(1);
/// How long the screen takes to fade out.
pub const FADE_OUT_FOR: Duration = Duration::from_millis(500);
/// How long the new system takes to fade in: the original's 1.5 s, as its
/// `_FadeWhiteOut` fades the display back from white.
pub const FADE_IN_FOR: Duration = Duration::from_millis(1500);
/// How long a streak is at the end of the streak phase, for a star layer
/// that moves with the camera; each layer's is this times its factor, so
/// near stars streak longer.
pub const STREAK_LENGTH: f32 = 512.0;
/// How long the arrival frame shows solid white when the jump plays
/// without its fades: one frame of the original's 30 a second, ours, as
/// its arrival frame is painted white whatever the preference.
pub const ARRIVAL_FLASH_FOR: Duration = Duration::from_millis(33);
/// The colour the screen fades to, at full alpha: white, as the original's
/// `_FadeWhiteIn` and `_FadeWhiteOut` fade the display.
pub const FADE_COLOR: Color = Color::WHITE;

/// The whole screen, which the fade covers.
const SCREEN: Bounds = Bounds {
    min: Point::new(0.0, 0.0),
    max: Point::new(VIEW_SIZE.0, VIEW_SIZE.1),
};

/// Where the effect is, each phase with how far through it, in `[0, 1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JumpPhase {
    /// The stars streak.
    Streak(f32),
    /// The old system fades out.
    FadeOut(f32),
    /// The new system fades in.
    FadeIn(f32),
    /// The arrival frame shows solid white, when the jump plays without
    /// its fades.
    Flash(f32),
    /// It is over.
    Done,
}

/// A hyperspace jump's effect, as it plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpEffect {
    /// Which way the ship jumps, on screen: a unit vector.
    direction: Point,
    /// How long it has played.
    elapsed: Duration,
    /// Whether it fades out to white and back in, as the Hyperspace Effects
    /// preference asks; without, the ship arrives as the streak ends.
    fades: bool,
}

impl JumpEffect {
    /// The effect of a jump from the system at map position `from` to the
    /// one at `to`: along the bearing between them, or up when they share
    /// a position.
    #[must_use]
    pub fn toward(from: Vec2, to: Vec2) -> Self {
        let bearing = to - from;
        let length = bearing.length();
        let direction = if length > 0.0 {
            Point::new(bearing.x / length, bearing.y / length)
        } else {
            Point::new(0.0, -1.0)
        };
        Self {
            direction,
            elapsed: Duration::ZERO,
            fades: true,
        }
    }

    /// This effect, with its white fades (the default) or without them, as
    /// the Hyperspace Effects preference is on or off.
    #[must_use]
    pub fn with_fades(self, fades: bool) -> Self {
        Self { fades, ..self }
    }

    /// Which way the ship jumps, on screen: a unit vector.
    #[must_use]
    pub fn direction(&self) -> Point {
        self.direction
    }

    /// Plays `dt` more of the effect, and says whether that crossed the
    /// instant the ship arrives, the end of the fade-out (or of the streak,
    /// without fades): true exactly once, however long `dt` is.
    pub fn advance(&mut self, dt: Duration) -> bool {
        let arrival = if self.fades {
            STREAK_FOR + FADE_OUT_FOR
        } else {
            STREAK_FOR
        };
        let before = self.elapsed;
        self.elapsed = self.elapsed.saturating_add(dt);
        before < arrival && self.elapsed >= arrival
    }

    /// Where the effect is.
    #[must_use]
    pub fn phase(&self) -> JumpPhase {
        let fraction = |into: Duration, of: Duration| into.as_secs_f32() / of.as_secs_f32();
        let mut at = self.elapsed;
        if at < STREAK_FOR {
            return JumpPhase::Streak(fraction(at, STREAK_FOR));
        }
        at -= STREAK_FOR;
        if !self.fades {
            if at < ARRIVAL_FLASH_FOR {
                return JumpPhase::Flash(fraction(at, ARRIVAL_FLASH_FOR));
            }
            return JumpPhase::Done;
        }
        if at < FADE_OUT_FOR {
            return JumpPhase::FadeOut(fraction(at, FADE_OUT_FOR));
        }
        at -= FADE_OUT_FOR;
        if at < FADE_IN_FOR {
            return JumpPhase::FadeIn(fraction(at, FADE_IN_FOR));
        }
        JumpPhase::Done
    }

    /// Whether the effect is over.
    #[must_use]
    pub fn done(&self) -> bool {
        self.phase() == JumpPhase::Done
    }

    /// How long the near stars' streaks are: growing through the streak
    /// phase, full through the fade-out, and none after.
    #[must_use]
    pub fn streak_length(&self) -> f32 {
        match self.phase() {
            JumpPhase::Streak(p) => STREAK_LENGTH * p,
            JumpPhase::FadeOut(_) => STREAK_LENGTH,
            JumpPhase::FadeIn(_) | JumpPhase::Flash(_) | JumpPhase::Done => 0.0,
        }
    }

    /// The fade's alpha: none while the stars streak, rising to opaque
    /// through the fade-out, and falling back to none through the fade-in;
    /// opaque through the arrival flash.
    #[must_use]
    pub fn fade_alpha(&self) -> u8 {
        let alpha = |p: f32| (p * 255.0).round() as u8;
        match self.phase() {
            JumpPhase::Streak(_) | JumpPhase::Done => 0,
            JumpPhase::FadeOut(p) => alpha(p),
            JumpPhase::FadeIn(p) => alpha(1.0 - p),
            JumpPhase::Flash(_) => u8::MAX,
        }
    }

    /// Covers the whole screen in [`FADE_COLOR`] at the fade's alpha, if it
    /// shows at all.
    pub fn draw_fade(&self, list: &mut DrawList) {
        let alpha = self.fade_alpha();
        if alpha > 0 {
            fill_rect(
                list,
                SCREEN,
                Color {
                    a: alpha,
                    ..FADE_COLOR
                },
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::DrawCommand;

    fn effect() -> JumpEffect {
        JumpEffect::toward(Vec2::new(0.0, 0.0), Vec2::new(600.0, 0.0))
    }

    fn after(dt: Duration) -> JumpEffect {
        let mut effect = effect();
        effect.advance(dt);
        effect
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn the_direction_is_the_map_bearing_between_the_systems_or_up() {
        assert_eq!(effect().direction(), Point::new(1.0, 0.0));
        let diagonal = JumpEffect::toward(Vec2::new(100.0, 100.0), Vec2::new(-200.0, 500.0));
        assert_eq!(diagonal.direction(), Point::new(-0.6, 0.8));
        let same = JumpEffect::toward(Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0));
        assert_eq!(same.direction(), Point::new(0.0, -1.0));
    }

    #[test]
    fn the_phases_follow_each_other_at_their_boundaries() {
        assert_eq!(effect().phase(), JumpPhase::Streak(0.0));
        assert_eq!(after(ms(500)).phase(), JumpPhase::Streak(0.5));
        assert_eq!(after(ms(1000)).phase(), JumpPhase::FadeOut(0.0));
        assert_eq!(after(ms(1250)).phase(), JumpPhase::FadeOut(0.5));
        assert_eq!(after(ms(1500)).phase(), JumpPhase::FadeIn(0.0));
        assert_eq!(after(ms(2250)).phase(), JumpPhase::FadeIn(0.5));
        assert_eq!(after(ms(3000)).phase(), JumpPhase::Done);
        assert_eq!(after(Duration::MAX).phase(), JumpPhase::Done);
        assert!(!after(ms(2999)).done());
        assert!(after(ms(3000)).done());
        assert_eq!(
            (STREAK_FOR, FADE_OUT_FOR, FADE_IN_FOR),
            (ms(1000), ms(500), ms(1500))
        );
    }

    #[test]
    fn advancing_reports_the_arrival_exactly_once() {
        let mut effect = effect();
        let mut arrivals = Vec::new();
        for frame in 1..=150 {
            if effect.advance(Duration::from_millis(16)) {
                arrivals.push(frame);
            }
        }
        // 1.5 s is crossed in the 94th frame of 16 ms.
        assert_eq!(arrivals, [94]);

        let mut exact = effect_at(ms(1499));
        assert!(!exact.advance(Duration::ZERO));
        assert!(exact.advance(ms(1)), "reaching it exactly");
        assert!(!exact.advance(ms(1)));

        let mut long = self::effect();
        assert!(long.advance(ms(5000)), "across the whole effect at once");
        assert!(!long.advance(ms(5000)));
        assert!(!long.advance(Duration::MAX), "nor past the end of time");
    }

    fn effect_at(at: Duration) -> JumpEffect {
        after(at)
    }

    #[test]
    fn the_streaks_grow_then_hold_then_vanish() {
        assert_eq!(effect().streak_length(), 0.0);
        assert_eq!(after(ms(250)).streak_length(), 128.0);
        assert!((after(ms(999)).streak_length() - 511.488).abs() < 1e-3);
        assert_eq!(after(ms(1200)).streak_length(), STREAK_LENGTH);
        assert_eq!(after(ms(1600)).streak_length(), 0.0);
        assert_eq!(after(ms(3000)).streak_length(), 0.0);
        assert_eq!(STREAK_LENGTH, 512.0);
    }

    #[test]
    fn the_fade_rises_from_nothing_to_opaque_and_falls_back() {
        let alphas: Vec<u8> = [0, 999, 1000, 1250, 1499, 1500, 2250, 2994, 3000]
            .into_iter()
            .map(|at| after(ms(at)).fade_alpha())
            .collect();
        assert_eq!(alphas, [0, 0, 0, 128, 254, 255, 128, 1, 0]);
    }

    #[test]
    fn the_fade_covers_the_screen_in_white() {
        let mut list = DrawList::new();
        after(ms(1250)).draw_fade(&mut list);
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::Line {
                from: Point::new(0.0, 384.0),
                to: Point::new(1024.0, 384.0),
                width: 768.0,
                color: Color::rgba(255, 255, 255, 128),
            }]
        );
        assert_eq!(FADE_COLOR, Color::WHITE);
        for at in [500, 3000] {
            let mut none = DrawList::new();
            after(ms(at)).draw_fade(&mut none);
            assert!(none.is_empty(), "{at}");
        }
    }

    fn plain(at: Duration) -> JumpEffect {
        let mut effect = effect().with_fades(false);
        effect.advance(at);
        effect
    }

    #[test]
    fn with_fades_true_is_the_default_effect() {
        assert_eq!(effect().with_fades(true), effect());
        assert_ne!(effect().with_fades(false), effect());
    }

    #[test]
    fn without_fades_the_ship_arrives_as_the_streak_ends() {
        let mut stepped = plain(ms(999));
        assert!(!stepped.advance(Duration::ZERO));
        assert!(stepped.advance(ms(1)), "reaching the streak's end");
        assert!(!stepped.advance(ms(1)));
        assert!(!stepped.advance(ms(5000)));

        let mut long = effect().with_fades(false);
        assert!(long.advance(ms(5000)), "across the whole effect at once");
        assert!(!long.advance(Duration::MAX));

        let mut frames = effect().with_fades(false);
        let arrivals: Vec<u32> = (1..=100).filter(|_| frames.advance(ms(16))).collect();
        // 1 s is crossed in the 63rd frame of 16 ms.
        assert_eq!(arrivals, [63]);
    }

    #[test]
    fn without_fades_the_phases_are_streak_flash_done() {
        assert_eq!(ARRIVAL_FLASH_FOR, ms(33));
        assert_eq!(plain(ms(500)).phase(), JumpPhase::Streak(0.5));
        assert_eq!(plain(STREAK_FOR).phase(), JumpPhase::Flash(0.0));
        let half = plain(STREAK_FOR + ARRIVAL_FLASH_FOR / 2).phase();
        assert!(
            matches!(half, JumpPhase::Flash(p) if (p - 0.5).abs() < 0.02),
            "{half:?}"
        );
        assert_eq!(
            plain(STREAK_FOR + ARRIVAL_FLASH_FOR).phase(),
            JumpPhase::Done
        );
        assert!(!plain(ms(1032)).done());
        assert!(plain(STREAK_FOR + ARRIVAL_FLASH_FOR).done());
        for at in (0..2000).step_by(10) {
            let phase = plain(ms(at)).phase();
            assert!(
                !matches!(phase, JumpPhase::FadeOut(_) | JumpPhase::FadeIn(_)),
                "{at}: {phase:?}"
            );
        }
    }

    #[test]
    fn without_fades_only_the_arrival_flash_is_white() {
        for at in [0, 500, 999] {
            let streak = plain(ms(at));
            assert_eq!(streak.fade_alpha(), 0, "{at}");
            let mut none = DrawList::new();
            streak.draw_fade(&mut none);
            assert!(none.is_empty(), "{at}");
        }
        assert_eq!(
            plain(ms(999)).streak_length(),
            after(ms(999)).streak_length()
        );
        for at in [1000, 1020, 1032] {
            let flash = plain(ms(at));
            assert_eq!(flash.fade_alpha(), 255, "{at}");
            assert_eq!(flash.streak_length(), 0.0, "{at}");
            let mut list = DrawList::new();
            flash.draw_fade(&mut list);
            assert_eq!(
                list.iter().cloned().collect::<Vec<_>>(),
                [DrawCommand::Line {
                    from: Point::new(0.0, 384.0),
                    to: Point::new(1024.0, 384.0),
                    width: 768.0,
                    color: FADE_COLOR,
                }],
                "{at}"
            );
        }
        assert_eq!(plain(ms(1033)).fade_alpha(), 0);
    }
}
