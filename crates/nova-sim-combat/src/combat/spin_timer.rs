//! A spinning shot's spin timer: when its frames step (rdm
//! weapons-fidelity phase 35).
//!
//! `_HandleShot` steps a shot's spin only for a weapon of `Flags` 0x0001
//! (`testb $0x1, 0x20(weapon)` @0x35879-0x3587e): on each live visit the
//! timer (shot +0x1c, a float) gains `_gSpeedMult`, and when it reaches
//! the weapon's `BeamWidth` (+0x72), or the `BeamWidth` is 0 or less, the
//! index steps and the timer goes back to 0 (@0x35884-0x358c4). With
//! `_gSpeedMult` at 1 a pass the timer is a whole number of passes, kept
//! here as a `u32`.
//!
//! `_SpawnShot` never writes +0x1c (no store to it @0x3e550-0x3f143), and
//! neither does anything else but the step, so a new shot starts on the
//! timer its slot's last spinning shot left
//! ([`ShotSlots::spin`](super::shot_slots::ShotSlots::spin)).
//!
//! [`pass`] is one visit; [`steps`] counts the steps over a run of
//! visits from a starting timer, which the view reads to show a shot's
//! frame from the timer it started on.

/// One live visit of a spinning shot whose timer is `timer`, of a weapon
/// of `BeamWidth` `beam_width` (@0x35884-0x358c4): the timer after it,
/// and whether the index stepped. The timer gains 1; at or past the
/// `BeamWidth`, or with a `BeamWidth` of 0 or less, the index steps and
/// the timer is 0.
#[must_use]
pub fn pass(timer: u32, beam_width: i16) -> (u32, bool) {
    let timer = timer.saturating_add(1);
    match u32::try_from(beam_width) {
        // A `BeamWidth` of none steps too: no timer is below it.
        Ok(width) if timer < width => (timer, false),
        _ => (0, true),
    }
}

/// How many of passes `from` to `to` (counted from 1, the shot's first
/// visit) step the index of a shot that started on timer `timer`, of a
/// weapon of `BeamWidth` `beam_width`: [`pass`] folded over them, in
/// closed form. Every pass steps for a `BeamWidth` of 1 or less;
/// otherwise the first step is on pass `BeamWidth − timer` (pass 1 for a
/// timer at or past `BeamWidth − 1`), then every `BeamWidth` passes.
#[must_use]
pub fn steps(timer: u32, beam_width: i16, from: u32, to: u32) -> u32 {
    let width = u32::try_from(beam_width).unwrap_or(0);
    let first = if width <= 1 {
        1
    } else {
        width.saturating_sub(timer).max(1)
    };
    let every = width.max(1);
    // The steps on passes 1 to `n`.
    let through = |n: u32| {
        if n < first {
            0
        } else {
            (n - first) / every + 1
        }
    };
    through(to).saturating_sub(through(from.saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_steps_once_the_timer_reaches_the_beam_width_and_resets_it() {
        assert_eq!(pass(0, 4), (1, false));
        assert_eq!(pass(2, 4), (3, false));
        assert_eq!(pass(3, 4), (0, true), "timer + 1 reaches 4");
        assert_eq!(pass(9, 4), (0, true), "past it");
        assert_eq!(pass(0, 1), (0, true), "every pass at 1");
        assert_eq!(pass(5, 0), (0, true), "every pass at 0");
        assert_eq!(pass(5, -3), (0, true), "and below");
    }

    #[test]
    fn steps_agrees_with_folding_pass_over_the_passes() {
        for timer in 0..=5 {
            for beam_width in -1..=5 {
                // Pass n's step, folding from `timer`.
                let mut stepped = Vec::new();
                let mut now = timer;
                for _ in 1..=12 {
                    let (next, step) = pass(now, beam_width);
                    stepped.push(step);
                    now = next;
                }
                for from in 1..=12_u32 {
                    for to in 0..=12_u32 {
                        let folded =
                            (from..=to).filter(|&n| stepped[n as usize - 1]).count() as u32;
                        assert_eq!(
                            steps(timer, beam_width, from, to),
                            folded,
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
}
