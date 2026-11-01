//! The frame a shot shows, which its sprite is drawn and touched by: the
//! index `_SpawnShot` starts it on and `_HandleShot` steps (`EV Nova.app`,
//! i386).
//!
//! - **The start** ([`spin_start`]). `_SpawnShot` sets shot +0x34 to 0
//!   for a weapon of `Flags` 0x0004 and otherwise to `Rand(36)`, 0 to 35,
//!   whatever its sheet holds (@0x3e96e-0x3e9b4); `_HandleStellarWeapons`
//!   builds its shots' the same way (@0x2f23e-0x2f259). Only a spinning
//!   shot's (`Flags` 0x0001) start is ever shown, so only a spinning one
//!   draws it here, on the fight's chance, and the others start at 0
//!   ([`Shot::spin_start`](super::projectile::Shot::spin_start)).
//! - **The frame** ([`shot_frame`], [`Shot::frame`]). A shot that does not
//!   spin shows its heading's frame, `trunc(n × heading / 360)` clamped to
//!   the sheet (@0x3593a-0x3596c). A spinning one shows the index
//!   `_HandleShot` holds before its pass's step (@0x35894), clamped to the
//!   sheet (@0x35956-0x3596c): the shot drawn at age `k` is on its `k`-th
//!   pass, so it has stepped on each pass before at which its spin timer
//!   reached `BeamWidth` ([`spin_timer`](super::spin_timer)). A step past
//!   the last frame wraps to 0, or holds the last under `Flags2` 0x0002
//!   (@0x358cb-0x358f6); under `Flags2` 0x0001 it shows frame 0 while its
//!   age is below `ProxSafety`, the index running on beneath, put back to
//!   0 on each of those passes with 0x0002 too (@0x35914-0x35932).
//!
//! The view draws the frame this gives it, and the contact passes touch
//! by it (see [`contact`](super::contact)). None of this plays a sound.

use std::num::NonZeroU16;

use super::projectile::Shot;
use super::spin_timer;
pub use super::weapon::SPINS;
use super::weapon::WeaponSpec;
use crate::chance::Chance;
use crate::sprite_mask::RleSheet;

/// `Flags`: the shot's frames start on the first (@0x3e976-0x3e985).
pub const FIRST_FRAME_START: u16 = 0x0004;
/// `Flags2`: the shot shows frame 0 until its age reaches `ProxSafety`.
pub const STILL_UNTIL_SAFE: u16 = 0x0001;
/// `Flags2`: a spinning shot holds its last frame instead of wrapping.
pub const HOLDS_LAST_FRAME: u16 = 0x0002;
/// How many indexes a start is drawn from: `_Rand(0x24)` (@0x3e996).
pub const SPIN_STARTS: u32 = 36;

/// What the frame rule reads of a shot's weapon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameRule {
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Flags2`.
    pub flags2: u16,
    /// Its `ProxSafety`, none when that is below none.
    pub prox_safety: u32,
    /// Its `BeamWidth`, the spin timer's step.
    pub beam_width: i16,
}

impl FrameRule {
    /// What the rule reads of `weapon`.
    #[must_use]
    pub fn of(weapon: &WeaponSpec) -> Self {
        Self {
            flags: weapon.flags,
            flags2: weapon.flags2,
            prox_safety: weapon.prox_safety,
            beam_width: weapon.beam_width,
        }
    }

    /// Whether its shots spin with their age.
    #[must_use]
    pub fn spins(self) -> bool {
        self.flags & SPINS != 0
    }
}

/// The index a shot of `rule`'s weapon starts on (see the module docs):
/// drawn below [`SPIN_STARTS`] on `chance` for a spinning one without
/// [`FIRST_FRAME_START`], else 0 with nothing drawn.
pub fn spin_start(rule: FrameRule, chance: &mut dyn Chance) -> u8 {
    if rule.spins() && rule.flags & FIRST_FRAME_START == 0 {
        u8::try_from(chance.below(SPIN_STARTS)).unwrap_or(0)
    } else {
        0
    }
}

/// The frame of a shot heading `heading` degrees, `age` ticks old, on a
/// sheet of `frames` frames, by `rule`, whose spin started on index
/// `start` and timer `timer` (see the module docs).
#[must_use]
pub fn shot_frame(
    heading: f32,
    age: u32,
    frames: NonZeroU16,
    rule: FrameRule,
    start: u8,
    timer: u32,
) -> u16 {
    let count = u32::from(frames.get());
    let last = count - 1;
    if !rule.spins() {
        // `(frames × heading)` in `f32`, over 360.0 in doubles, truncated
        // toward 0 and clamped to the sheet (`_HandleShot`
        // @0x3593a-0x3596c), so a bomb's unwrapped heading below 0 or past
        // 360 shows an end frame.
        let turned = f64::from(f32::from(frames.get()) * heading) / 360.0;
        // Clamped into the sheet's frames, which fit a `u16`.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        return (turned as i64).clamp(0, i64::from(last)) as u16;
    }
    let holds = rule.flags2 & HOLDS_LAST_FRAME != 0;
    let safety = if rule.flags2 & STILL_UNTIL_SAFE == 0 {
        0
    } else {
        rule.prox_safety
    };
    if age < safety {
        return 0;
    }
    // The steps made on passes `from` to `to`.
    let steps = |from: u32, to: u32| spin_timer::steps(timer, rule.beam_width, from, to);
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

impl Shot {
    /// The frame it shows now on a sheet of `frames` frames ([`shot_frame`]).
    #[must_use]
    pub fn frame(&self, frames: NonZeroU16) -> u16 {
        shot_frame(
            self.heading,
            self.age,
            frames,
            FrameRule::of(&self.weapon),
            self.spin_start,
            self.spin_timer,
        )
    }

    /// The frame it shows now on `sheet`, by the frames its tokens hold
    /// ([`Shot::frame`]): the one the contact passes touch it by and the
    /// view draws. None on a sheet of no frames.
    #[must_use]
    pub fn frame_on(&self, sheet: &RleSheet) -> Option<u16> {
        let frames = NonZeroU16::new(u16::try_from(sheet.starts.len()).unwrap_or(u16::MAX))?;
        Some(self.frame(frames))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_sim_base::fixture::Draws;

    fn n(frames: u16) -> NonZeroU16 {
        NonZeroU16::new(frames).expect("non-zero")
    }

    #[test]
    fn a_spinning_shot_draws_its_start_below_36_unless_it_starts_on_the_first() {
        let spinning = FrameRule {
            flags: SPINS,
            ..FrameRule::default()
        };
        let mut chance = Draws::of(&[20]);
        assert_eq!(spin_start(spinning, &mut chance), 20);
        assert_eq!(chance.times_asked(SPIN_STARTS), 1);
        let first = FrameRule {
            flags: SPINS | FIRST_FRAME_START,
            ..spinning
        };
        let mut none = Draws::of(&[]);
        assert_eq!(spin_start(first, &mut none), 0);
        assert_eq!(spin_start(FrameRule::default(), &mut none), 0, "no spin");
        assert_eq!(none.times_asked(SPIN_STARTS), 0, "neither draws");
    }

    #[test]
    fn a_shot_shows_the_frame_its_heading_truncates_to() {
        let plain = FrameRule::default();
        let frame = |heading, frames| shot_frame(heading, 1, n(frames), plain, 0, 0);
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
            shot_frame(185.0, 9, n(36), plain, 20, 3),
            18,
            "a shot that does not spin ignores its start and its timer"
        );
    }

    fn spinning(beam_width: i16, flags2: u16, prox_safety: u32) -> FrameRule {
        FrameRule {
            flags: SPINS,
            flags2,
            beam_width,
            prox_safety,
        }
    }

    /// The frames shown at `ages` of a shot of `rule` on a sheet of
    /// `frames`, started on index `start`. A shot is drawn at age `k` on
    /// its `k`-th pass, from age 1.
    fn ages_from(rule: FrameRule, frames: u16, start: u8, ages: std::ops::Range<u32>) -> Vec<u16> {
        timed(rule, frames, (start, 0), ages)
    }

    /// [`ages_from`] for a shot that started on spin timer `timer`.
    fn timed(
        rule: FrameRule,
        frames: u16,
        (start, timer): (u8, u32),
        ages: std::ops::Range<u32>,
    ) -> Vec<u16> {
        ages.map(|age| shot_frame(90.0, age, n(frames), rule, start, timer))
            .collect()
    }

    fn ages(rule: FrameRule, frames: u16, ages: std::ops::Range<u32>) -> Vec<u16> {
        ages_from(rule, frames, 0, ages)
    }

    #[test]
    fn a_spinning_shot_turns_a_frame_every_beam_width_ticks_and_wraps() {
        assert_eq!(ages(spinning(2, 0, 0), 3, 1..9), [0, 0, 1, 1, 2, 2, 0, 0]);
        assert_eq!(ages(spinning(0, 0, 0), 3, 1..5), [0, 1, 2, 0], "every tick");
        assert_eq!(ages(spinning(-1, 0, 0), 3, 1..5), [0, 1, 2, 0]);
        assert_eq!(ages(spinning(1, 0, 0), 3, 1..5), [0, 1, 2, 0]);
        assert_eq!(
            ages(spinning(1, 0, 0), 3, 0..2),
            [0, 0],
            "its first pass shows its start, before any step"
        );
    }

    #[test]
    fn a_spinning_shot_may_hold_its_last_frame() {
        assert_eq!(
            ages(spinning(1, HOLDS_LAST_FRAME, 0), 3, 1..7),
            [0, 1, 2, 2, 2, 2]
        );
    }

    #[test]
    fn a_spinning_shot_runs_on_from_its_start() {
        // Start 3 on 8 frames, a step every 2 passes.
        assert_eq!(
            ages_from(spinning(2, 0, 0), 8, 3, 1..13),
            [3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 0, 0]
        );
        assert_eq!(
            ages_from(spinning(2, HOLDS_LAST_FRAME, 0), 8, 3, 1..13),
            [3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 7, 7]
        );
    }

    #[test]
    fn a_start_past_the_sheet_shows_the_last_frame_until_the_first_step() {
        // Start 20 on 8 frames: clamped to 7 until the first step, which
        // wraps it to 0 (@0x358cb-0x358f6).
        assert_eq!(
            ages_from(spinning(2, 0, 0), 8, 20, 1..21),
            [7, 7, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 0, 0]
        );
        assert_eq!(
            ages_from(spinning(2, HOLDS_LAST_FRAME, 0), 8, 20, 1..7),
            [7, 7, 7, 7, 7, 7],
            "or holds the last"
        );
        assert_eq!(
            ages_from(spinning(1, 0, 0), 8, 8, 1..4),
            [7, 0, 1],
            "a start of the frame count is past the sheet"
        );
    }

    #[test]
    fn a_spinning_shot_steps_first_when_the_timer_it_started_on_runs_out() {
        // A timer of 2 at `BeamWidth` 4 steps on pass 2, shown from age 3,
        // then every 4; from 0, on pass 4, shown from age 5.
        let rule = spinning(4, 0, 0);
        assert_eq!(timed(rule, 8, (0, 2), 1..8), [0, 0, 1, 1, 1, 1, 2]);
        assert_eq!(timed(rule, 8, (0, 0), 1..8), [0, 0, 0, 0, 1, 1, 1]);
        assert_eq!(
            timed(spinning(3, 0, 0), 8, (0, 5), 1..6),
            [0, 1, 1, 1, 2],
            "a timer past BeamWidth steps on the first pass"
        );
        assert_eq!(
            timed(spinning(4, 0, 0), 8, (6, 3), 1..4),
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
                spinning(2, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 4),
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
            ages(spinning(1, STILL_UNTIL_SAFE, 3), 4, 1..9),
            [0, 0, 2, 3, 0, 1, 2, 3]
        );
        assert_eq!(
            ages_from(spinning(1, STILL_UNTIL_SAFE, 3), 4, 1, 1..6),
            [0, 0, 3, 0, 1],
            "from its start"
        );
        assert_eq!(
            ages(spinning(1, 0, 3), 4, 1..4),
            [0, 1, 2],
            "only with the flag"
        );
        // With 0x0002 too, the index is put back to 0 on the passes below
        // the safety, so it runs from 0 at it.
        assert_eq!(
            ages(spinning(1, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 3), 3, 1..8),
            [0, 0, 0, 1, 2, 2, 2]
        );
        assert_eq!(
            ages_from(
                spinning(2, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 4),
                8,
                20,
                1..10
            ),
            [0, 0, 0, 0, 1, 1, 2, 2, 3],
            "its start forgotten; the timer runs on: steps on passes 4, 6 and 8"
        );
        assert_eq!(
            ages_from(
                spinning(1, STILL_UNTIL_SAFE | HOLDS_LAST_FRAME, 1),
                4,
                2,
                1..4
            ),
            [2, 3, 3],
            "a safety of 1 puts nothing back"
        );
    }

    #[test]
    fn a_shots_frame_reads_its_heading_age_weapon_start_and_timer() {
        let spec = WeaponSpec::new(&nova_sim_catalog::fixture::weapon(128));
        let (ship, from) = (
            super::super::ShipRef::Player,
            crate::flight::ShipState::default(),
        );
        let mut shot = Shot::launch(spec, ship, ship, &from, 0.0);
        shot.weapon.flags = SPINS;
        shot.weapon.beam_width = 2;
        shot.age = 5;
        shot.spin_start = 3;
        shot.spin_timer = 1;
        assert_eq!(
            shot.frame(n(8)),
            shot_frame(0.0, 5, n(8), spinning(2, 0, 0), 3, 1)
        );
        assert_eq!(shot.frame(n(8)), 5, "steps on passes 1 and 3");
        shot.weapon.flags = 0;
        shot.heading = 100.0;
        assert_eq!(shot.frame(n(36)), 10);
    }

    #[test]
    fn a_shot_shows_its_frame_on_a_sheets_frame_count_and_none_on_an_empty_one() {
        use nova_sim_base::fixture::rle::sheet;
        let spec = WeaponSpec::new(&nova_sim_catalog::fixture::weapon(128));
        let (ship, from) = (
            super::super::ShipRef::Player,
            crate::flight::ShipState::default(),
        );
        let mut shot = Shot::launch(spec, ship, ship, &from, 0.0);
        shot.weapon.flags = 0;
        shot.heading = 200.0;
        let frame = ["#"];
        let four = sheet(&[&frame, &frame, &frame, &frame]);
        assert_eq!(shot.frame_on(&four), Some(shot.frame(n(4))));
        assert_eq!(shot.frame_on(&four), Some(2));
        let empty = RleSheet {
            starts: Vec::new(),
            ..four
        };
        assert_eq!(shot.frame_on(&empty), None);
    }
}
