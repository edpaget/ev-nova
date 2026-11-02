//! A spinning shot's spin timer: when its frames step (rdm
//! weapons-fidelity phase 35).
//!
//! `_HandleShot` steps a shot's spin only for a weapon of `Flags` 0x0001
//! (`testb $0x1, 0x20(weapon)` @0x35879-0x3587e): on each live visit the
//! timer (shot +0x1c, a float) gains the frame's gain, `_gSpeedMult`
//! (@0x35884), added in doubles and narrowed, and when it reaches the
//! weapon's `BeamWidth` (+0x72), or the `BeamWidth` is 0 or less, the
//! index steps and the timer goes back to 0 (@0x35884-0x358c4).
//!
//! `_SpawnShot` never writes +0x1c (no store to it @0x3e550-0x3f143), and
//! neither does anything else but the step, so a new shot starts on the
//! timer its slot's last spinning shot left
//! ([`ShotSlots::spin`](super::shot_slots::ShotSlots::spin)).
//!
//! [`pass`] is one visit, which the shot's spin
//! ([`Shot::spin_visit`](super::projectile::Shot::spin_visit)) and its
//! slot's timer ([`ShotSlots::keep_spin`](super::shot_slots::ShotSlots::keep_spin))
//! run on.

use crate::speed_mult::SpeedMult;

/// One live visit of a spinning shot whose timer is `timer`, of a weapon
/// of `BeamWidth` `beam_width`, at the frame's `gain` (@0x35884-0x358c4):
/// the timer after it, and whether the index stepped. The timer gains the
/// gain; at or past the `BeamWidth`, as a float, or with a `BeamWidth` of
/// 0 or less, the index steps and the timer is 0.
#[must_use]
pub fn pass(timer: f32, beam_width: i16, gain: SpeedMult) -> (f32, bool) {
    let timer = gain.add_times(timer, 1.0);
    if beam_width <= 0 || timer >= f32::from(beam_width) {
        (0.0, true)
    } else {
        (timer, false)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const FULL: SpeedMult = SpeedMult::FULL;

    /// How many of visits `from` to `to` (counted from 1, the shot's
    /// first) step the index of a shot that started on timer `timer`, of
    /// a weapon of `BeamWidth` `beam_width`: [`pass`] run visit by visit
    /// at full gain.
    fn steps(timer: u32, beam_width: i16, from: u32, to: u32) -> u32 {
        let mut now = f32::from(u16::try_from(timer).expect("a small timer"));
        let mut stepped = 0;
        for n in 1..=to {
            let (next, step) = pass(now, beam_width, FULL);
            now = next;
            if step && n >= from {
                stepped += 1;
            }
        }
        stepped
    }

    #[test]
    fn a_pass_steps_once_the_timer_reaches_the_beam_width_and_resets_it() {
        assert_eq!(pass(0.0, 4, FULL), (1.0, false));
        assert_eq!(pass(2.0, 4, FULL), (3.0, false));
        assert_eq!(pass(3.0, 4, FULL), (0.0, true), "timer + 1 reaches 4");
        assert_eq!(pass(9.0, 4, FULL), (0.0, true), "past it");
        assert_eq!(pass(0.0, 1, FULL), (0.0, true), "every pass at 1");
        assert_eq!(pass(5.0, 0, FULL), (0.0, true), "every pass at 0");
        assert_eq!(pass(5.0, -3, FULL), (0.0, true), "and below");
    }

    #[test]
    fn steps_agrees_with_folding_pass_over_the_passes() {
        // The steps, in closed form: every pass for a `BeamWidth` of 1 or
        // less; otherwise the first on pass `BeamWidth - timer` (pass 1
        // for a timer at or past `BeamWidth - 1`), then every `BeamWidth`.
        let closed = |timer: u32, beam_width: i16, from: u32, to: u32| {
            let width = u32::try_from(beam_width).unwrap_or(0);
            let first = if width <= 1 {
                1
            } else {
                width.saturating_sub(timer).max(1)
            };
            let every = width.max(1);
            let through = |n: u32| {
                if n < first {
                    0
                } else {
                    (n - first) / every + 1
                }
            };
            through(to).saturating_sub(through(from.saturating_sub(1)))
        };
        for timer in 0..=5 {
            for beam_width in -1..=5 {
                for from in 1..=12_u32 {
                    for to in 0..=12_u32 {
                        assert_eq!(
                            steps(timer, beam_width, from, to),
                            closed(timer, beam_width, from, to),
                            "timer {timer}, BeamWidth {beam_width}, passes {from}..={to}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_first_step_waits_out_what_the_timer_has_left() {
        let first = |timer, beam_width| (1..=20).find(|&n| steps(timer, beam_width, n, n) == 1);
        assert_eq!(
            first(0, 4),
            Some(4),
            "a timer of 0: every BeamWidth-th pass"
        );
        assert_eq!(steps(0, 4, 1, 12), 3, "passes 4, 8 and 12");
        assert_eq!(first(2, 4), Some(2), "a timer of 2 at 4: pass 2");
        assert_eq!(steps(2, 4, 1, 10), 3, "passes 2, 6 and 10");
        assert_eq!(first(5, 3), Some(1), "a timer past BeamWidth: pass 1");
        assert_eq!(steps(0, 4, 1, 0), 0, "no passes");
    }

    #[test]
    fn at_half_gain_a_beam_width_of_2_steps_every_4_visits() {
        let mut timer = 0.0;
        let stepped: Vec<u32> = (1..=12)
            .filter(|_| {
                let (next, step) = pass(timer, 2, SpeedMult::new(0.5));
                timer = next;
                step
            })
            .collect();
        assert_eq!(stepped, [4, 8, 12]);
        assert_eq!(pass(0.5, 2, SpeedMult::new(0.5)), (1.0, false));
    }

    #[test]
    fn at_no_gain_a_beam_width_of_0_still_steps_every_visit_and_another_never() {
        assert_eq!(pass(0.0, 0, SpeedMult::ZERO), (0.0, true));
        assert_eq!(pass(1.5, 2, SpeedMult::ZERO), (1.5, false));
    }
}
