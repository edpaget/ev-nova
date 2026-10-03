//! A stellar's animation: which frame of its sheet shows when.
//!
//! A stellar animates when its sheet has more than one frame, its
//! `AnimDelay` is above 0, and it is not marked to animate only when
//! destroyed (`Flags2` 0x0080; nothing is destroyed in a viewer). Each
//! frame then shows for `AnimDelay` ticks of 1/30 s, except frame 0, which
//! shows `Frame0Bias` times as long when the bias is above 1. Frames run
//! 0, 1, ..., n - 1 and wrap round. Otherwise frame 0 shows throughout.
//!
//! In ticks, with delay d, bias b = max(`Frame0Bias`, 1) and n frames, the
//! cycle is d x (b + n - 1) ticks long; at t ticks into a cycle, the frame
//! is 0 while t < d x b, and 1 + (t - d x b) / d after.

use std::num::NonZeroU16;

use super::catalog::AnimationData;

/// When each frame of a stellar's sheet shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Animation {
    /// The cycle, or `None` for a stellar that stays on frame 0.
    cycle: Option<Cycle>,
}

/// An animated stellar's timing, in ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cycle {
    /// How long each frame after frame 0 shows; at least 1.
    delay: u128,
    /// How long frame 0 shows; at least `delay`.
    first: u128,
    /// How many frames there are; at least 2.
    frames: u128,
}

impl Animation {
    /// The animation of a sheet of `frames` frames with these fields.
    #[must_use]
    pub fn new(frames: NonZeroU16, data: AnimationData) -> Self {
        let animated = frames.get() > 1 && data.delay > 0 && !data.only_when_destroyed;
        let cycle = animated.then(|| {
            let delay = u128::from(data.delay.unsigned_abs());
            let bias = u128::from(data.frame0_bias.max(1).unsigned_abs());
            Cycle {
                delay,
                first: delay * bias,
                frames: u128::from(frames.get()),
            }
        });
        Self { cycle }
    }

    /// Whether the frame ever changes.
    #[must_use]
    pub fn is_animated(&self) -> bool {
        self.cycle.is_some()
    }

    /// The frame showing `ticks` ticks of 1/30 s after the clock started.
    #[must_use]
    pub fn frame(&self, ticks: u128) -> u16 {
        let Some(cycle) = self.cycle else {
            return 0;
        };
        let length = cycle.first + cycle.delay * (cycle.frames - 1);
        let t = ticks % length;
        if t < cycle.first {
            0
        } else {
            // Below `frames`, which is a u16.
            (1 + (t - cycle.first) / cycle.delay) as u16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animation(frames: u16, delay: i16, frame0_bias: i16) -> Animation {
        Animation::new(
            NonZeroU16::new(frames).expect("non-zero"),
            AnimationData {
                delay,
                frame0_bias,
                only_when_destroyed: false,
            },
        )
    }

    fn frames(animation: Animation, ticks: impl IntoIterator<Item = u128>) -> Vec<u16> {
        ticks.into_iter().map(|t| animation.frame(t)).collect()
    }

    #[test]
    fn a_wormhole_shows_each_of_its_32_frames_for_2_ticks_and_wraps() {
        let wormhole = animation(32, 2, -1);
        assert!(wormhole.is_animated());
        assert_eq!(frames(wormhole, [0, 1, 2, 3, 4]), [0, 0, 1, 1, 2]);
        assert_eq!(frames(wormhole, [62, 63, 64, 65, 66]), [31, 31, 0, 0, 1]);
        assert_eq!(wormhole.frame(64 * 1000 + 5), 2);
    }

    #[test]
    fn a_bias_above_1_holds_frame_0_that_many_times_as_long() {
        let biased = animation(4, 2, 3);
        assert_eq!(
            frames(biased, 0..=13),
            [0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 0, 0]
        );
        // A bias of 1, 0 or below shows frame 0 for one delay.
        for bias in [1, 0, -1] {
            let plain = animation(4, 2, bias);
            assert_eq!(frames(plain, 0..=8), [0, 0, 1, 1, 2, 2, 3, 3, 0], "{bias}");
        }
    }

    #[test]
    fn a_delay_of_1_steps_every_tick() {
        assert_eq!(frames(animation(3, 1, 0), 0..=6), [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(frames(animation(2, 1, 2), 0..=6), [0, 0, 1, 0, 0, 1, 0]);
    }

    fn stays_on_frame_0(animation: Animation) {
        assert!(!animation.is_animated(), "{animation:?}");
        assert!((0..100).all(|t| animation.frame(t) == 0), "{animation:?}");
    }

    #[test]
    fn one_frame_no_delay_or_only_when_destroyed_stays_on_frame_0() {
        stays_on_frame_0(animation(1, 2, 0));
        // A Hypergate: 42 frames, but no delay.
        stays_on_frame_0(animation(42, 0, 0));
        stays_on_frame_0(animation(42, -1, 0));
        stays_on_frame_0(Animation::new(
            NonZeroU16::new(32).expect("non-zero"),
            AnimationData {
                delay: 2,
                frame0_bias: -1,
                only_when_destroyed: true,
            },
        ));
    }

    #[test]
    fn the_largest_fields_do_not_overflow() {
        let slow = animation(u16::MAX, i16::MAX, i16::MAX);
        assert!(slow.is_animated());
        let first = u128::from(i16::MAX.unsigned_abs()).pow(2);
        assert_eq!(slow.frame(first - 1), 0);
        assert_eq!(slow.frame(first), 1);
        assert_eq!(slow.frame(u128::MAX), slow.frame(u128::MAX));
    }
}
