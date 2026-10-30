//! The main screen's rollover, the picture over the middle of the screen
//! that names the button under the pointer: `_MainScreenRolloverIdle`
//! (@0x16462) moves it, `_MainScreenRolloverDraw` (@0x17ad4) draws it.
//!
//! **The idle**, at most once a Mac tick (`TickCount` past the last step
//! @0x164b6-0x164c3, the last step then set to now): the target is the
//! first of the six buttons hit at the pointer, per pixel
//! (@0x16500-0x16528), or the last frame, the none frame, over none. A
//! target other than the frame shown fades the rollover out by 4 a tick
//! while it shows (@0x16558), and once it is clear the target is shown
//! (@0x16561); the frame shown as its target fades in by 4 a tick
//! (@0x16575). The alpha is kept within 0 to 32 (@0x1657b-0x1658c).
//! `_DoMainScreen` starts it clear on the none frame (@0x1af4a-0x1af6a),
//! and `_LoadMainScreenAnim` sets its last step to 0 (@0x17a73), so its
//! first idle steps.
//!
//! The engine steps once for each whole Mac tick that passes, carrying
//! the rest, as the original's idle, polled far more often than a tick,
//! steps each tick.
//!
//! **The draw** sets the sprite's destination factor (+0xa8) to 32 and its
//! red, green and blue factors (+0xaa to +0xae) to the alpha, with no
//! bias (+0xb0 = 0) (@0x17b65-0x17b9f): the lights' layout, which
//! `_BlitPixieRLETranslucentDrawProc` draws as `((src * alpha) >> 5) |
//! dst` per channel, `src | dst` at 32 and nothing at 0
//! ([`lights_tint`]). So it is drawn as the ship
//! lights are, with [`Blend::Or`](crate::draw::Blend::Or).
//!
//! Left out: at sprite-world depth 32 the original switches frames at
//! once, without the fade out (@0x1654e), and draws the rollover opaque
//! (@0x17be5). The engine draws at depth 16, the depth stock asks for
//! ("Thousands of colors", `STR#` 140 #18), and the roadmap leaves depth
//! 32 out.
//!
//! **In the background** (`_inBackground`, read @0x16468 before the
//! slides' hold and the timer) it shows the none frame at 32 every idle
//! (@0x16593-0x165bc): [`Rollover::switched_out`].
//!
//! **The slides' hold.** While any of the main screen's slides runs, the
//! idle sets the alpha to 0 and returns before its timer check
//! (@0x1647f-0x164b1), keeping the frame shown: [`Rollover::clear`] each
//! tick, so the first step after the last slide finishes comes at once.

use std::time::Duration;

use super::art::RolloverArt;
use crate::draw::{DrawList, lights_tint};
use crate::geometry::Point;
use crate::image::ImageKey;
use crate::time::{MAC_TICK_NANOS, sixtieths};

/// How far the alpha moves a tick.
pub const STEP: u8 = 4;
/// The alpha at which the rollover shows in full.
pub const FULL: u8 = 32;
/// More steps than any change takes to settle: a fade out (8 steps), the
/// switch and a fade in (8). Steps past it change nothing, so a long
/// frame takes no more.
const SETTLED: u128 = 17;

/// The rollover's frame and alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rollover {
    shown: u16,
    alpha: u8,
    frames: u16,
    since: Duration,
}

impl Rollover {
    /// A rollover of `frames` frames, clear on the none frame, its first
    /// step due.
    #[must_use]
    pub fn new(frames: u16) -> Self {
        Self {
            shown: frames.saturating_sub(1),
            alpha: 0,
            frames,
            since: Duration::from_nanos(MAC_TICK_NANOS as u64),
        }
    }

    /// The frame shown.
    #[must_use]
    pub fn shown(&self) -> u16 {
        self.shown
    }

    /// Its alpha, from 0 (clear) to [`FULL`].
    #[must_use]
    pub fn alpha(&self) -> u8 {
        self.alpha
    }

    /// Clears it, keeping the frame shown, with one step due at once: as
    /// `_LoadMainScreenAnim` sets the alpha to 0 and the last step to 0
    /// (@0x179d8-0x179dd, @0x17a73), and as the idle holds it while the
    /// slides run (@0x1647f-0x164b1).
    pub fn clear(&mut self) {
        self.alpha = 0;
        self.since = Duration::from_nanos(MAC_TICK_NANOS as u64);
    }

    /// The app is in the background: the none frame shows in full, with
    /// a step due at once. `_MainScreenRolloverIdle` checks
    /// `_inBackground` before the slides' hold and its timer, and sets
    /// the none frame (`_menuRolloverSprite`'s frames less one, or 0 with
    /// none) at alpha 0x20 every idle (@0x16468, @0x16593-0x165bc), never
    /// touching its last step, so the first idle after steps.
    pub fn switched_out(&mut self) {
        self.shown = self.none_frame();
        self.alpha = FULL;
        self.since = Duration::from_nanos(MAC_TICK_NANOS as u64);
    }

    /// The frame shown over no button: the last.
    #[must_use]
    pub fn none_frame(&self) -> u16 {
        self.frames.saturating_sub(1)
    }

    /// `dt` passes with the button `target` under the pointer, if any: a
    /// step for each whole Mac tick.
    pub fn tick(&mut self, dt: Duration, target: Option<usize>) {
        self.since = self.since.saturating_add(dt);
        let steps = sixtieths(self.since);
        self.since = Duration::from_nanos((self.since.as_nanos() % MAC_TICK_NANOS) as u64);
        let target = target.map_or(self.none_frame(), |button| {
            u16::try_from(button).unwrap_or(u16::MAX)
        });
        for _ in 0..steps.min(SETTLED) {
            self.step(target);
        }
    }

    /// One tick's step toward showing `target`.
    fn step(&mut self, target: u16) {
        if target == self.shown {
            self.alpha = (self.alpha + STEP).min(FULL);
        } else if self.alpha > 0 {
            self.alpha = self.alpha.saturating_sub(STEP);
        } else {
            self.shown = target;
        }
    }

    /// Draws the frame shown at `art`'s place from `corner`, at its alpha;
    /// nothing while it is clear. A frame past the last draws the last,
    /// as the draw clamps it (@0x17af2-0x17b0d).
    pub fn draw(&self, list: &mut DrawList, art: &RolloverArt, corner: Point) {
        if self.alpha == 0 {
            return;
        }
        let frame = self.shown.min(art.frames.saturating_sub(1));
        let center = Point::new(
            corner.x + art.position.x + art.frame_size.0 / 2.0,
            corner.y + art.position.y + art.frame_size.1 / 2.0,
        );
        list.or_sprite(
            ImageKey::sprite(art.sprite, frame),
            center,
            lights_tint(self.alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::screens::mac_ticks;

    use super::*;
    use crate::draw::{Blend, DrawCommand};

    fn ticked(rollover: &mut Rollover, target: Option<usize>) -> (u16, u8) {
        rollover.tick(mac_ticks(1), target);
        (rollover.shown(), rollover.alpha())
    }

    #[test]
    fn it_starts_clear_on_the_none_frame() {
        let rollover = Rollover::new(7);
        assert_eq!((rollover.shown(), rollover.alpha()), (6, 0));
        assert_eq!(rollover.none_frame(), 6);
    }

    #[test]
    fn clearing_it_keeps_its_frame_and_leaves_one_step_due() {
        let mut rollover = Rollover::new(7);
        rollover.tick(mac_ticks(20), Some(2));
        assert_eq!((rollover.shown(), rollover.alpha()), (2, 32));
        rollover.tick(Duration::from_millis(10), Some(2));
        rollover.clear();
        assert_eq!((rollover.shown(), rollover.alpha()), (2, 0));
        rollover.tick(Duration::ZERO, Some(2));
        assert_eq!(rollover.alpha(), 4, "one step at once");
        rollover.tick(Duration::from_millis(10), Some(2));
        assert_eq!(rollover.alpha(), 4, "and only one");
    }

    #[test]
    fn switched_out_it_shows_the_none_frame_in_full_with_a_step_due() {
        let mut rollover = Rollover {
            shown: 2,
            alpha: 12,
            frames: 7,
            since: Duration::ZERO,
        };
        rollover.switched_out();
        assert_eq!((rollover.shown(), rollover.alpha()), (6, FULL));
        rollover.tick(Duration::ZERO, Some(2));
        assert_eq!(
            (rollover.shown(), rollover.alpha()),
            (6, 28),
            "the step due comes at once"
        );
    }

    #[test]
    fn switched_out_with_no_frames_it_shows_frame_0() {
        let mut rollover = Rollover::new(0);
        rollover.switched_out();
        assert_eq!((rollover.shown(), rollover.alpha()), (0, FULL));
    }

    #[test]
    fn its_first_step_comes_at_once() {
        let mut rollover = Rollover::new(7);
        rollover.tick(Duration::ZERO, None);
        assert_eq!(rollover.alpha(), 4, "fading in on the none frame");
    }

    #[test]
    fn over_a_button_it_switches_then_fades_in_by_4_a_tick_to_32() {
        let mut rollover = Rollover::new(7);
        rollover.tick(Duration::ZERO, Some(2));
        assert_eq!(
            (rollover.shown(), rollover.alpha()),
            (2, 0),
            "clear: switches"
        );
        let alphas: Vec<u8> = (0..9).map(|_| ticked(&mut rollover, Some(2)).1).collect();
        assert_eq!(alphas, [4, 8, 12, 16, 20, 24, 28, 32, 32]);
    }

    #[test]
    fn it_fades_out_before_switching_to_another_buttons_frame() {
        let mut rollover = Rollover::new(7);
        for _ in 0..9 {
            rollover.tick(mac_ticks(1), Some(1));
        }
        assert_eq!((rollover.shown(), rollover.alpha()), (1, 32));
        let mut steps = Vec::new();
        for _ in 0..10 {
            steps.push(ticked(&mut rollover, Some(4)));
        }
        assert_eq!(
            steps,
            [
                (1, 28),
                (1, 24),
                (1, 20),
                (1, 16),
                (1, 12),
                (1, 8),
                (1, 4),
                (1, 0),
                (4, 0),
                (4, 4)
            ]
        );
    }

    #[test]
    fn over_no_button_it_shows_the_none_frame() {
        let mut rollover = Rollover::new(7);
        // The step due at once switches, the tick's fades in.
        rollover.tick(mac_ticks(1), Some(0));
        assert_eq!((rollover.shown(), rollover.alpha()), (0, 4));
        assert_eq!(ticked(&mut rollover, None), (0, 0));
        assert_eq!(ticked(&mut rollover, None), (6, 0));
        assert_eq!(ticked(&mut rollover, None), (6, 4));
    }

    #[test]
    fn an_alpha_off_the_step_is_kept_within_0_and_32() {
        let mut rollover = Rollover {
            shown: 3,
            alpha: 30,
            frames: 7,
            since: Duration::ZERO,
        };
        assert_eq!(ticked(&mut rollover, Some(3)), (3, 32));
        rollover.alpha = 2;
        assert_eq!(ticked(&mut rollover, Some(5)), (3, 0));
        rollover.alpha = 31;
        assert_eq!(ticked(&mut rollover, Some(3)), (3, 32));
    }

    #[test]
    fn it_steps_once_a_whole_tick_carrying_the_rest() {
        let mut rollover = Rollover::new(7);
        rollover.tick(Duration::ZERO, Some(6));
        assert_eq!(rollover.alpha(), 4);
        rollover.tick(Duration::from_millis(10), Some(6));
        assert_eq!(rollover.alpha(), 4, "not a whole tick");
        rollover.tick(Duration::from_millis(7), Some(6));
        assert_eq!(rollover.alpha(), 8, "17 ms make a tick");
        rollover.tick(mac_ticks(3), Some(6));
        assert_eq!(rollover.alpha(), 20, "three steps");
    }

    #[test]
    fn a_long_frame_settles_as_any_number_of_steps_would() {
        let mut rollover = Rollover::new(7);
        rollover.tick(mac_ticks(1000), Some(3));
        assert_eq!((rollover.shown(), rollover.alpha()), (3, 32));
        rollover.tick(mac_ticks(17), Some(5));
        assert_eq!((rollover.shown(), rollover.alpha()), (5, 32));
    }

    fn art() -> RolloverArt {
        RolloverArt {
            sprite: 8020,
            frame_size: (136.0, 98.0),
            frames: 7,
            position: Point::new(444.0, 465.0),
        }
    }

    #[test]
    fn it_draws_its_frame_ored_at_its_alpha_from_the_corner() {
        let rollover = Rollover {
            shown: 2,
            alpha: 12,
            frames: 7,
            since: Duration::ZERO,
        };
        let mut list = DrawList::new();
        rollover.draw(&mut list, &art(), Point::new(10.0, 20.0));
        let commands: Vec<&DrawCommand> = list.iter().collect();
        assert_eq!(
            commands,
            [&DrawCommand::Sprite {
                image: ImageKey::sprite(8020, 2),
                center: Point::new(10.0 + 444.0 + 68.0, 20.0 + 465.0 + 49.0),
                tint: lights_tint(12),
                blend: Blend::Or,
            }]
        );
    }

    #[test]
    fn it_draws_nothing_while_clear_and_clamps_a_frame_past_the_last() {
        let mut rollover = Rollover::new(7);
        let mut list = DrawList::new();
        rollover.draw(&mut list, &art(), Point::new(0.0, 0.0));
        assert!(list.is_empty());
        rollover.shown = 9;
        rollover.alpha = 32;
        rollover.draw(&mut list, &art(), Point::new(0.0, 0.0));
        let frames: Vec<u16> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { image, .. } => Some(image.frame),
                _ => None,
            })
            .collect();
        assert_eq!(frames, [6]);
    }
}
