//! The black screen fades: the death's fade out and the main screen's fade
//! back in.
//!
//! The original fades the whole display with `CGDisplayFade`: to solid
//! black (rgb 0,0,0) in `_FadeScreenOut` (@0x530f, over 1.0 s, 0x3f800000
//! @0x5399), and back from it in `_FadeScreenIn` (@0x53b2, 1.0 s,
//! @0x542d-0x543f). Both are synchronous, so nothing else moves while one
//! plays. A [`ScreenFade`] draws the same over the whole view, flight's
//! widened one included ([`whole_view`]): a black
//! quad whose alpha rises or falls with the fade, drawn last so it covers
//! everything. The original skips its fades in windowed mode; the engine
//! draws them always, as it does the jump's white fades.

use std::time::Duration;

use crate::draw::fill_rect;
use crate::geometry::Bounds;
use crate::{Color, DrawList, Point};

/// How long the death's fade to black and the main screen's fade back
/// from it take: the 1.0 s of `_FadeScreenOut` (@0x5399) and
/// `_FadeScreenIn` (@0x542d).
pub const DEATH_FADE_FOR: Duration = Duration::from_secs(1);

/// The whole of a view `view` logical units in size, which a fade covers.
#[must_use]
pub fn whole_view(view: (f32, f32)) -> Bounds {
    Bounds {
        min: Point::new(0.0, 0.0),
        max: Point::new(view.0, view.1),
    }
}

/// A fade of the whole screen to black or back from it, on display time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenFade {
    /// Whether it ends black (to black) or clear (from black).
    to_black: bool,
    /// How long it takes.
    duration: Duration,
    /// How far into it the display is.
    elapsed: Duration,
}

impl ScreenFade {
    /// A fade from clear to solid black over `duration`.
    #[must_use]
    pub fn to_black(duration: Duration) -> Self {
        Self {
            to_black: true,
            duration,
            elapsed: Duration::ZERO,
        }
    }

    /// A fade from solid black to clear over `duration`.
    #[must_use]
    pub fn from_black(duration: Duration) -> Self {
        Self {
            to_black: false,
            ..Self::to_black(duration)
        }
    }

    /// Moves the fade on by `dt`.
    pub fn advance(&mut self, dt: Duration) {
        self.elapsed = self.elapsed.saturating_add(dt);
    }

    /// Whether the fade has run its whole duration.
    #[must_use]
    pub fn done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// How opaque the black is now, 0 (clear) to 255 (solid).
    #[must_use]
    pub fn alpha(&self) -> u8 {
        let progress = if self.done() {
            1.0
        } else {
            self.elapsed.as_secs_f32() / self.duration.as_secs_f32()
        };
        let black = if self.to_black {
            progress
        } else {
            1.0 - progress
        };
        // In 0..=1, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let alpha = (black * 255.0).round() as u8;
        alpha
    }

    /// Covers the whole of a view `view` in size in black at the fade's
    /// alpha, if it shows at all: flight's, widened or not, or Nova's own
    /// 1024 x 768 over the main menu.
    pub fn draw(&self, list: &mut DrawList, view: (f32, f32)) {
        let alpha = self.alpha();
        if alpha > 0 {
            fill_rect(
                list,
                whole_view(view),
                Color {
                    a: alpha,
                    ..Color::BLACK
                },
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::{Color, DrawCommand, DrawList};

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn after(mut fade: ScreenFade, dt: Duration) -> ScreenFade {
        fade.advance(dt);
        fade
    }

    #[test]
    fn a_fade_to_black_rises_from_clear_to_opaque_over_its_duration() {
        let fade = ScreenFade::to_black(ms(1000));
        assert_eq!(fade.alpha(), 0);
        assert_eq!(after(fade, ms(500)).alpha(), 128);
        assert_eq!(after(fade, ms(999)).alpha(), 255);
        assert_eq!(after(fade, ms(1000)).alpha(), 255);
        assert_eq!(after(fade, ms(5000)).alpha(), 255, "and stays");
    }

    #[test]
    fn a_fade_from_black_falls_from_opaque_to_clear_over_its_duration() {
        let fade = ScreenFade::from_black(ms(1000));
        assert_eq!(fade.alpha(), 255);
        assert_eq!(after(fade, ms(250)).alpha(), 191);
        assert_eq!(after(fade, ms(500)).alpha(), 128);
        assert_eq!(after(fade, ms(1000)).alpha(), 0);
        assert_eq!(after(fade, ms(5000)).alpha(), 0, "and stays");
    }

    #[test]
    fn a_fade_runs_at_the_pace_of_its_duration() {
        let to = ScreenFade::to_black(ms(2000));
        assert_eq!(after(to, ms(500)).alpha(), 64, "a quarter of the way");
        assert_eq!(after(to, ms(1500)).alpha(), 191);
        let from = ScreenFade::from_black(ms(400));
        assert_eq!(after(from, ms(100)).alpha(), 191);
    }

    #[test]
    fn a_fade_is_done_once_its_duration_has_passed() {
        for fade in [
            ScreenFade::to_black(ms(1000)),
            ScreenFade::from_black(ms(1000)),
        ] {
            assert!(!fade.done());
            assert!(!after(fade, ms(999)).done());
            assert!(after(fade, ms(1000)).done());
            let mut fade = fade;
            fade.advance(ms(600));
            fade.advance(ms(400));
            assert!(fade.done(), "advances add up");
        }
    }

    #[test]
    fn a_fade_over_no_time_is_done_at_once_at_its_end() {
        assert_eq!(ScreenFade::to_black(Duration::ZERO).alpha(), 255);
        assert!(ScreenFade::to_black(Duration::ZERO).done());
        assert_eq!(ScreenFade::from_black(Duration::ZERO).alpha(), 0);
    }

    #[test]
    fn it_covers_the_whole_screen_in_black_at_its_alpha() {
        let mut list = DrawList::new();
        after(ScreenFade::to_black(ms(1000)), ms(500)).draw(&mut list, (1024.0, 768.0));
        after(ScreenFade::to_black(ms(1000)), ms(500)).draw(&mut list, (1365.0, 768.0));
        let commands: Vec<_> = list.iter().cloned().collect();
        let line = |width| DrawCommand::Line {
            from: crate::Point::new(0.0, 384.0),
            to: crate::Point::new(width, 384.0),
            width: 768.0,
            color: Color::rgba(0, 0, 0, 128),
        };
        assert_eq!(commands, [line(1024.0), line(1365.0)]);
    }

    #[test]
    fn a_clear_fade_draws_nothing() {
        let mut list = DrawList::new();
        ScreenFade::to_black(ms(1000)).draw(&mut list, (1024.0, 768.0));
        after(ScreenFade::from_black(ms(1000)), ms(1000)).draw(&mut list, (1024.0, 768.0));
        assert!(list.is_empty(), "{list:?}");
    }

    #[test]
    fn the_death_fade_takes_a_second() {
        assert_eq!(DEATH_FADE_FOR, ms(1000));
    }
}
