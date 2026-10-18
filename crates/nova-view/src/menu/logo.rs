//! The main screen's logo flicker, as `_MainScreenAnimIdle` picks its
//! frame (@0x18853-0x1888f).
//!
//! Once `TickCount` is at least 5 past the last pick it picks again,
//! setting the last pick to now; the pick is `Rand(frame count)` drawn
//! again until it differs from the frame shown. `_DoMainScreen` starts it
//! on frame 0 (@0x1af31), and `_LoadMainScreenAnim` starts the 5 ticks
//! as the screen appears (@0x17a66).
//!
//! The engine makes the same decision in one draw: a frame among the
//! others, uniformly, with no loop that could run long. Its order of
//! draws is not the original's, which the engine's generator could not
//! reproduce anyway. With a single frame there is nothing else to pick,
//! and the logo stays on it.

use std::time::Duration;

use nova_sim::Chance;

use crate::time::{MAC_TICK_NANOS, sixtieths};

/// The Mac ticks between picks.
pub const PICK_TICKS: u128 = 5;

/// The logo's frame, and the time since it was picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Logo {
    frame: u16,
    frames: u16,
    since: Duration,
}

impl Logo {
    /// A logo of `frames` frames, on frame 0, just picked.
    #[must_use]
    pub fn new(frames: u16) -> Self {
        Self {
            frame: 0,
            frames,
            since: Duration::ZERO,
        }
    }

    /// The frame shown.
    #[must_use]
    pub fn frame(&self) -> u16 {
        self.frame
    }

    /// `dt` passes: once 5 Mac ticks have since the last pick, a new
    /// frame is picked on `chance`, and the last pick is now, keeping only
    /// the part of a tick already under way, as `TickCount` counts whole
    /// ticks.
    pub fn tick(&mut self, dt: Duration, chance: &mut dyn Chance) {
        self.wait(dt);
        if sixtieths(self.since) < PICK_TICKS {
            return;
        }
        self.since = Duration::from_nanos((self.since.as_nanos() % MAC_TICK_NANOS) as u64);
        if self.frames > 1 {
            let other = chance.roll(self.frames - 1);
            self.frame = if other >= self.frame {
                other + 1
            } else {
                other
            };
        }
    }

    /// Starts the 5 ticks again, keeping the frame shown, as
    /// `_LoadMainScreenAnim` sets the last pick to now (@0x17a66) each
    /// time the screen appears.
    pub fn restart(&mut self) {
        self.since = Duration::ZERO;
    }

    /// `dt` passes with nothing picked: the time counts towards the next
    /// pick, as `TickCount` runs on while the original's idle is not
    /// called.
    pub fn wait(&mut self, dt: Duration) {
        self.since = self.since.saturating_add(dt);
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::screens::mac_ticks;

    use super::*;

    /// Rolls the scripted outcomes in turn, recording each roll's sides.
    struct Scripted {
        outcomes: Vec<u16>,
        sides: Vec<u16>,
    }

    impl Scripted {
        fn new(outcomes: &[u16]) -> Self {
            Self {
                outcomes: outcomes.iter().rev().copied().collect(),
                sides: Vec::new(),
            }
        }
    }

    impl Chance for Scripted {
        fn fires(&mut self, _percent: u8) -> bool {
            unreachable!("the logo only rolls")
        }

        fn below(&mut self, _n: u32) -> u32 {
            unreachable!("the logo only rolls")
        }

        fn roll(&mut self, sides: u16) -> u16 {
            self.sides.push(sides);
            self.outcomes.pop().expect("scripted")
        }
    }

    #[test]
    fn it_starts_on_frame_0() {
        assert_eq!(Logo::new(7).frame(), 0);
    }

    #[test]
    fn nothing_changes_before_the_fifth_tick() {
        let mut logo = Logo::new(7);
        let mut chance = Scripted::new(&[]);
        logo.tick(mac_ticks(4), &mut chance);
        assert_eq!(logo.frame(), 0);
        assert_eq!(chance.sides, [0_u16; 0], "no roll");
    }

    #[test]
    fn the_fifth_tick_picks_among_the_other_frames() {
        let mut logo = Logo::new(7);
        let mut chance = Scripted::new(&[3]);
        logo.tick(mac_ticks(5), &mut chance);
        assert_eq!(chance.sides, [6], "one of the six others");
        assert_eq!(logo.frame(), 4, "3 is past frame 0");
    }

    #[test]
    fn no_outcome_gives_the_frame_before() {
        for shown in 0..7 {
            for outcome in 0..6 {
                let mut logo = Logo {
                    frame: shown,
                    frames: 7,
                    since: Duration::ZERO,
                };
                logo.tick(mac_ticks(5), &mut Scripted::new(&[outcome]));
                assert_ne!(logo.frame(), shown, "{shown} {outcome}");
                assert!(logo.frame() < 7);
            }
        }
        let mut logo = Logo {
            frame: 3,
            frames: 7,
            since: Duration::ZERO,
        };
        logo.tick(mac_ticks(5), &mut Scripted::new(&[2]));
        assert_eq!(logo.frame(), 2, "below the frame shown");
    }

    #[test]
    fn time_carries_across_ticks_and_a_pick_keeps_only_a_tick_under_way() {
        let mut logo = Logo::new(7);
        let mut chance = Scripted::new(&[0, 0, 0]);
        for _ in 0..4 {
            logo.tick(mac_ticks(1), &mut chance);
        }
        assert_eq!(logo.frame(), 0);
        logo.tick(mac_ticks(1) + Duration::from_millis(5), &mut chance);
        assert_eq!(logo.frame(), 1, "the fifth tick picks");
        // 5 ms of a tick carried: 4 ticks and 12 ms more make the next.
        logo.tick(mac_ticks(4) + Duration::from_millis(11), &mut chance);
        assert_eq!(logo.frame(), 1);
        logo.tick(Duration::from_millis(1), &mut chance);
        assert_eq!(logo.frame(), 0);
        // A long gap picks once, then waits 5 ticks again.
        logo.tick(mac_ticks(40), &mut chance);
        assert_eq!(logo.frame(), 1);
        logo.tick(mac_ticks(4), &mut chance);
        assert_eq!(chance.sides.len(), 3, "no pick 4 ticks on");
    }

    #[test]
    fn waiting_counts_toward_the_next_pick_without_picking() {
        let mut logo = Logo::new(7);
        let mut chance = Scripted::new(&[0]);
        logo.wait(mac_ticks(9));
        assert_eq!(logo.frame(), 0);
        logo.tick(Duration::ZERO, &mut chance);
        assert_eq!(logo.frame(), 1);
    }

    #[test]
    fn a_restart_keeps_the_frame_and_waits_5_ticks_again() {
        let mut logo = Logo::new(7);
        let mut chance = Scripted::new(&[2, 0]);
        logo.tick(mac_ticks(5), &mut chance);
        assert_eq!(logo.frame(), 3);
        logo.wait(mac_ticks(4));
        logo.restart();
        assert_eq!(logo.frame(), 3, "kept");
        logo.tick(mac_ticks(4), &mut chance);
        assert_eq!(logo.frame(), 3, "the wait starts again");
        logo.tick(mac_ticks(1), &mut chance);
        assert_eq!(logo.frame(), 0);
    }

    #[test]
    fn one_frame_stays_and_rolls_nothing() {
        let mut logo = Logo::new(1);
        let mut chance = Scripted::new(&[]);
        logo.tick(mac_ticks(50), &mut chance);
        assert_eq!(logo.frame(), 0);
        assert!(chance.sides.is_empty());
    }
}
