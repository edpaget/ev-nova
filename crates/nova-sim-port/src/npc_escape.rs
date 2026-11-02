//! What a dying NPC leaves behind: its crew's escape pods, its person's
//! pod, and its one roll for an escape ship (`_HandleShip`'s dying block
//! @0x35080-0x3557b). The rules alone, in whole ticks of the death timer
//! (the original's +0x3c, the engine's `Condition::Dying::ticks_left`),
//! each computed in `i32` rather than the original's 16-bit words; the
//! session applies them (`nova_sim::Session`'s `npc_escape` module).
//!
//! **When.** The person's pod and the crew pods are asked only while the
//! timer is above 0 before the tick's decrement (`jbe 0x35305`
//! @0x3508b), so never on the breakup's first tick, whose timer is still
//! the spawn's 0; each asks of the timer after the decrement. The escape
//! ship is asked outside that guard, of a dying ship (`_IsDying` @0x253d:
//! the timer above 0, or the armour gone), so its first chance is the
//! breakup's first tick, at a timer of 0.
//!
//! **Half the `DeathDelay`.** Each test compares the timer with
//! `DeathDelay x 0.5` (0.5 at 0xdd128) in doubles: here `2 x timer` with
//! the `DeathDelay`, so that half an odd one never equals a whole timer.
//!
//! - **The person's pod** ([`person_pod_due`], @0x35141-0x351fa): on the
//!   tick the timer is exactly half the `DeathDelay`, for a person of
//!   `Flags` [`ESCAPE_POD`] whose government is none or lacks
//!   [`INTERCEPTORS_RETREAT`] (in-memory +0x21 bit 0, `Flags` 0x0100).
//! - **The crew pods** ([`crew_pod_due`], @0x351ff-0x35300): while pods
//!   are left and the timer is at or below half, the first at once and
//!   the rest on each frame a multiple of [`pod_interval`], which the
//!   frame's gain divides when it is above 0.0 (`_gSpeedMult` @0x3527a,
//!   `ucomisd` 0xdd118, `cvttsd2si` @0x35298), so a slow frame spaces the
//!   pods wider.
//! - **The escape ship** ([`escape_roll_due`], @0x35464-0x354b9): a dying
//!   ship with an escape ship aboard, once the timer is at or below half,
//!   rolls `Rand(`[`ESCAPE_ROLL_ODDS`]`) == 0`.
//!
//! Left out: the interval's 16-bit width, as the original takes it into
//! its `idivl` of `_frameCounter` (@0x352a4): an integer width, not
//! replicated.

use nova_sim_base::speed_mult::SpeedMult;
use nova_sim_state::govt::INTERCEPTORS_RETREAT;

use crate::person::ESCAPE_POD;

/// The least crew-pod interval, in frames (`cmpw $9; cmovle` with 10
/// @0x35262-0x35277).
pub const POD_INTERVAL_FLOOR: i32 = 10;
/// The share of `DeathDelay / PodCount` the interval takes (the double
/// @0xdd1e8).
pub const POD_INTERVAL_SHARE: f64 = 0.4;
/// The escape ship's odds: it launches on `Rand(3) == 0` (@0x354bf).
pub const ESCAPE_ROLL_ODDS: u32 = 3;

/// The frames between a dying ship's crew pods, for a `DeathDelay` of
/// `death_delay` and a class `PodCount` of `class_pods` (above 0), at the
/// frame's `gain`: `max(10, trunc(trunc(death_delay / class_pods) x
/// 0.4))`, the division an integer one (`idivl` @0x3525c), truncating
/// toward 0; then, when the gain is above 0.0, `trunc(interval / g)` in
/// doubles (@0x3527a-0x35298). An `i32`, wider than the original's
/// short (see the module docs).
#[must_use]
pub fn pod_interval(death_delay: i32, class_pods: i32, gain: SpeedMult) -> i32 {
    let share = f64::from(death_delay / class_pods.max(1)) * POD_INTERVAL_SHARE;
    let interval = (share as i32).max(POD_INTERVAL_FLOOR);
    let g = gain.get();
    if g > 0.0 {
        #[allow(clippy::cast_possible_truncation)]
        let divided = (f64::from(interval) / g) as i32;
        divided
    } else {
        interval
    }
}

/// Whether the death timer `timer` is at or below half `death_delay`.
fn at_or_below_half(timer: i32, death_delay: i32) -> bool {
    2 * i64::from(timer) <= i64::from(death_delay)
}

/// Whether a crew pod comes out this frame, with the timer at `timer`
/// after its decrement, of a class of `DeathDelay` `death_delay` and
/// `PodCount` `class_pods`, `left` pods still aboard, on frame `frame`
/// (`_frameCounter`, signed): while pods are left, the timer is at or
/// below half the `DeathDelay` and the class carries pods, the first at
/// once (`left == class_pods`) and each later one on a frame that is a
/// multiple of [`pod_interval`] at the frame's `gain`.
#[must_use]
pub fn crew_pod_due(
    timer: i32,
    death_delay: i32,
    class_pods: i32,
    left: i32,
    frame: i32,
    gain: SpeedMult,
) -> bool {
    left > 0
        && at_or_below_half(timer, death_delay)
        && class_pods > 0
        && (frame % pod_interval(death_delay, class_pods, gain) == 0 || left == class_pods)
}

/// Whether a dying ship's person launches its pod this frame, with the
/// timer at `timer` after its decrement, of a class of `DeathDelay`
/// `death_delay`, the person of `Flags` `person_flags`, of a government
/// of `Flags` `govt_flags` (none for no government): exactly at half the
/// `DeathDelay`, for a person with [`ESCAPE_POD`], whose government is
/// none or lacks [`INTERCEPTORS_RETREAT`].
#[must_use]
pub fn person_pod_due(
    timer: i32,
    death_delay: i32,
    person_flags: u16,
    govt_flags: Option<u16>,
) -> bool {
    2 * i64::from(timer) == i64::from(death_delay)
        && person_flags & ESCAPE_POD != 0
        && govt_flags.is_none_or(|flags| flags & INTERCEPTORS_RETREAT == 0)
}

/// Whether a ship rolls for its escape ship this frame: `dying`, with an
/// escape ship aboard (`has_escape_ship`), and the timer at `timer`, at
/// or below half its `DeathDelay` `death_delay`.
#[must_use]
pub fn escape_roll_due(dying: bool, has_escape_ship: bool, timer: i32, death_delay: i32) -> bool {
    dying && has_escape_ship && at_or_below_half(timer, death_delay)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn the_interval_is_a_whole_share_of_the_delay_floored_at_10() {
        assert_eq!(
            pod_interval(100, 10, SpeedMult::FULL),
            10,
            "4 floored at 10"
        );
        assert_eq!(pod_interval(165, 3, SpeedMult::FULL), 22, "trunc(55 x 0.4)");
        assert_eq!(pod_interval(1000, 4, SpeedMult::FULL), 100);
        assert_eq!(
            pod_interval(129, 2, SpeedMult::FULL),
            25,
            "64 x 0.4 = 25.6, not 129 / 2 x 0.4 = 25.8"
        );
        assert_eq!(
            pod_interval(139, 2, SpeedMult::FULL),
            27,
            "69 x 0.4 = 27.6, not 27.8"
        );
        assert_eq!(
            pod_interval(149, 2, SpeedMult::FULL),
            29,
            "74 x 0.4 = 29.6, not 29.8"
        );
        assert_eq!(pod_interval(175, 7, SpeedMult::FULL), 10, "25 x 0.4 = 10");
        assert_eq!(pod_interval(200, 7, SpeedMult::FULL), 11, "28 x 0.4 = 11.2");
        assert_eq!(
            pod_interval(-500, 2, SpeedMult::FULL),
            10,
            "a negative delay is floored"
        );
        assert_eq!(pod_interval(250, 1, SpeedMult::FULL), 100);
    }

    #[test]
    fn the_frames_gain_divides_the_interval_when_above_none() {
        assert_eq!(pod_interval(1000, 4, SpeedMult::new(0.25)), 400);
        assert_eq!(pod_interval(1000, 4, SpeedMult::new(0.5)), 200);
        assert_eq!(pod_interval(165, 3, SpeedMult::new(0.5)), 44);
        assert_eq!(
            pod_interval(165, 3, SpeedMult::new(0.3)),
            73,
            "trunc(73.33)"
        );
        assert_eq!(pod_interval(1000, 4, SpeedMult::FULL), 100);
        assert_eq!(
            pod_interval(1000, 4, SpeedMult::ZERO),
            100,
            "none: no division"
        );
        assert_eq!(
            pod_interval(100, 10, SpeedMult::new(0.5)),
            20,
            "after the floor"
        );
    }

    #[test]
    fn at_half_gain_a_later_crew_pod_comes_on_twice_the_interval() {
        let half = SpeedMult::new(0.5);
        let due = |frame| crew_pod_due(49, 100, 10, 9, frame, half);
        assert!(due(20), "a multiple of 20");
        assert!(!due(10), "not of 10 alone");
        assert!(
            crew_pod_due(50, 100, 10, 10, 7, half),
            "the first still at once"
        );
    }

    #[test]
    fn the_first_crew_pod_comes_at_once_and_the_rest_on_the_interval() {
        let due = |timer, left, frame| crew_pod_due(timer, 100, 10, left, frame, SpeedMult::FULL);
        assert!(due(50, 10, 7), "the first at once, any frame");
        assert!(!due(51, 10, 0), "not above half");
        assert!(!due(50, 9, 7), "a later one off the interval");
        assert!(due(49, 9, 20), "on a multiple of 10");
        assert!(due(0, 1, 0), "the last");
        assert!(!due(40, 0, 0), "none left");
        assert!(due(40, 3, -20), "a negative multiple");
        assert!(!due(40, 3, -15), "-15 % 10 is not 0");
        assert!(
            !crew_pod_due(40, 100, 0, 3, 0, SpeedMult::FULL),
            "a class of no pods"
        );
        assert!(
            !crew_pod_due(40, 100, -1, -1, 0, SpeedMult::FULL),
            "nor of fewer"
        );
        assert!(
            crew_pod_due(50, 101, 10, 9, 10, SpeedMult::FULL),
            "50 is below 50.5"
        );
        assert!(
            !crew_pod_due(51, 101, 10, 10, 10, SpeedMult::FULL),
            "51 above it"
        );
    }

    #[test]
    fn a_persons_pod_comes_at_exactly_half_the_delay() {
        let due = |timer, delay| person_pod_due(timer, delay, ESCAPE_POD, None);
        assert!(due(50, 100));
        assert!(!due(49, 100) && !due(51, 100));
        for timer in 0..200 {
            assert!(!due(timer, 101), "never for an odd delay: {timer}");
        }
        assert!(due(0, 0), "half of none");
    }

    #[test]
    fn a_persons_pod_needs_its_flag_and_a_government_that_lets_it() {
        let due = |flags, govt| person_pod_due(50, 100, flags, govt);
        assert!(due(ESCAPE_POD, None), "no government");
        assert!(
            due(ESCAPE_POD | 0x0001, Some(0x00ff)),
            "a govt without 0x0100"
        );
        assert!(due(ESCAPE_POD, Some(0xfeff)));
        assert!(!due(ESCAPE_POD, Some(0x0100)), "a govt with 0x0100");
        assert!(!due(0xfffd, None), "no 0x0002");
    }

    #[test]
    fn the_escape_roll_needs_a_dying_ship_an_escape_ship_and_half_the_delay() {
        assert!(escape_roll_due(true, true, 0, 100), "on the breakup tick");
        assert!(escape_roll_due(true, true, 50, 100));
        assert!(!escape_roll_due(true, true, 51, 100));
        assert!(!escape_roll_due(false, true, 0, 100), "not dying");
        assert!(!escape_roll_due(true, false, 0, 100), "no escape ship");
        assert!(
            !escape_roll_due(true, true, 0, -2),
            "a negative delay never"
        );
        assert!(escape_roll_due(true, true, 0, 0));
    }

    #[test]
    fn the_constants_are_the_originals() {
        assert_eq!((POD_INTERVAL_FLOOR, ESCAPE_ROLL_ODDS), (10, 3));
        assert_eq!(POD_INTERVAL_SHARE, 0.4);
        assert_eq!((ESCAPE_POD, INTERCEPTORS_RETREAT), (0x0002, 0x0100));
    }
}
