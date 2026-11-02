//! A decaying shot's decay timer and steps, the original's +0x18 and
//! +0x32, as `_HandleShot` runs them each live visit at the frame's gain
//! (@0x35eb2-0x35f03).
//!
//! With the weapon's `Decay` (+0x1a) above 0, the timer gains the gain,
//! `_gSpeedMult` (@0x35ecd), added in doubles and narrowed; once it is
//! above `Decay` as a float the shot takes a step and the timer goes back
//! to 0. Each step takes a point off its direct hit's mass and energy
//! damage (`_HandleShipHit` @0x36add-0x36afd). `_SpawnShot` sets the steps
//! to 0 (@0x3e9ca) and the timer to 0 with a `Decay`, else -1.0, which
//! nothing reads (@0x3e9d9-0x3e9f3). By the Bible's reading of
//! [`RuleKey::ShotDecay`](crate::RuleKey::ShotDecay) it steps once the
//! timer reaches `Decay` ([`DecayStep`]).
//!
//! None of this plays a sound.

use super::projectile::DecayStep;
use crate::speed_mult::SpeedMult;

/// A decay timer `timer` and its steps `steps` after a visit at the
/// frame's `gain`, of a weapon of `Decay` `decay`, a step as `step` reads
/// it (see the module docs): unchanged with no `Decay`.
#[must_use]
pub fn pass(timer: f32, steps: u32, decay: u32, step: DecayStep, gain: SpeedMult) -> (f32, u32) {
    if decay == 0 {
        return (timer, steps);
    }
    let timer = gain.add_times(timer, 1.0);
    let line = super::projectile::whole_frames(decay);
    let stepped = match step {
        DecayStep::EveryDecayPlusOne => timer > line,
        DecayStep::EveryDecay => timer >= line,
    };
    if stepped {
        (0.0, steps + 1)
    } else {
        (timer, steps)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const HALF: SpeedMult = SpeedMult::new(0.5);

    /// The visits, of the first `visits`, on which a `Decay` `decay` shot
    /// steps at `gain` by `step`.
    fn steps_on(decay: u32, step: DecayStep, gain: SpeedMult, visits: u32) -> Vec<u32> {
        let (mut timer, mut steps) = (0.0, 0);
        (1..=visits)
            .filter(|_| {
                let before = steps;
                (timer, steps) = pass(timer, steps, decay, step, gain);
                steps > before
            })
            .collect()
    }

    #[test]
    fn by_the_original_a_decay_of_1_steps_every_3_visits_at_half_gain_and_every_2_at_full() {
        let original = DecayStep::EveryDecayPlusOne;
        assert_eq!(steps_on(1, original, HALF, 9), [3, 6, 9]);
        assert_eq!(steps_on(1, original, SpeedMult::FULL, 6), [2, 4, 6]);
        assert_eq!(
            pass(0.5, 0, 1, original, HALF),
            (1.0, 0),
            "1.0 is not above 1"
        );
    }

    #[test]
    fn by_the_bible_a_decay_of_1_steps_every_2_visits_at_half_gain_and_every_one_at_full() {
        let bible = DecayStep::EveryDecay;
        assert_eq!(steps_on(1, bible, HALF, 6), [2, 4, 6]);
        assert_eq!(steps_on(1, bible, SpeedMult::FULL, 3), [1, 2, 3]);
    }

    #[test]
    fn no_decay_never_steps_and_no_gain_moves_nothing() {
        assert_eq!(pass(0.0, 2, 0, DecayStep::EveryDecay, HALF), (0.0, 2));
        assert_eq!(
            pass(0.5, 1, 3, DecayStep::EveryDecayPlusOne, SpeedMult::ZERO),
            (0.5, 1)
        );
    }
}
