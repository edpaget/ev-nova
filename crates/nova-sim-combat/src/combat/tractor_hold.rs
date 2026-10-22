//! What a tractor lock ([`TractorLock`], the original's +0xc8dc and +0xb4)
//! does to the ship it holds: the pull toward its holder's velocity, the
//! release and the lapse, and the third of an NPC's speed, turn and
//! thrust. The lock itself is set and touched by the fight (see the
//! [`impact`](super::impact) module); the refused jump is the session's
//! (`nova_sim::Session::begin_jump`).
//!
//! **The pull** ([`pull`], `_HandleShip` @0x33fc0-0x3413d, `_HandlePlayer`
//! @0x684c5-0x68608). Each frame, before the ship moves, each velocity
//! axis moves toward the target's by a step of [`STEP_SHARE`] (0.25f
//! @0xdd684) × the **player's** `_ShipAccelRate` (slot 0, ionized), for
//! every held ship: first down by the step while above the target by more
//! than it, then up by the step while below by more than it, then snapped
//! to the target while strictly within the step. It is not scaled by the
//! frame. The target is the holder's velocity as its slot holds it, or
//! none when the lock names the ship itself (a light tractor's firer,
//! `_HandleBeams` @0x31287), which so brakes toward rest. No gate on the
//! holder or the held ship comes first: a disabled, dying or warping ship
//! is pulled too.
//!
//! **The release and the lapse** ([`Holder`], `_HandleShip`
//! @0x353ba-0x35449, `_HandlePlayer` @0x68436-0x684b7 and
//! @0x6860e-0x68623). A lock not naming the ship itself is dropped when
//! its holder is inactive (+0xb8 none), disabled or dying. Then, whoever
//! holds it, it is dropped once `TickCount` reaches its touch (+0xb4)
//! plus 30: 30 sixtieths of a second, [`LAPSE_TICKS`] engine ticks. The
//! fight sets [`TractorLock::ticks_since_touch`] to none on each touch;
//! [`TractorLock::age`] counts it up on each advance pass alone, as
//! double time's second pass doesn't move `TickCount`.
//!
//! An NPC's release comes after its pull, so a ship whose holder is
//! disabled or dying is pulled once more first; one whose holder is gone
//! is not pulled, as the original would read a slot no ship holds
//! ([`npc_pass`]). The player's release comes first, and it is not pulled
//! on the frame it is released ([`player_pass`]). The original pulls it
//! anyway, toward "slot −1", memory before the ship array (@0x684b7 →
//! 0x684c5): an original bug with nothing meaningful to replicate.
//!
//! **The held handling** ([`held_handling`]): `_ShipMaxSpeed`
//! (@0x37f7-0x3815), `_ShipTurnRate` (@0x33f7-0x3415) and
//! `_ShipAccelRate` (@0x3643-0x366e) take × [`HELD_FACTOR`] (0.333 as a
//! double, @0xdd0c8) on a ship another ship holds ([`held_by_other`]).
//! Only their NPC branch reads the lock: slot 0's never does, so the
//! player is never slowed. The turn takes it before the every-ship floor
//! (@0x3441-0x3473), so an NPC whose class turns 1° a tick or more still
//! turns at least that; the thrust takes it in place of the × 2, which the
//! engine's thrust already holds, so a held NPC thrusts a sixth of its
//! engine thrust. A ship that holds itself is neither slowed nor refused a
//! jump.
//!
//! None of this plays a sound.

use super::ShipRef;
use super::impact::TractorLock;
use crate::flight::Flyer;
use crate::geometry::Vec2;
use crate::handling::{Handling, floored_turn};

/// The share of the player's `_ShipAccelRate` a pull moves each axis by
/// a frame (0.25f @0xdd684).
pub const STEP_SHARE: f32 = 0.25;

/// The factor on a held NPC's top speed, turn and thrust (0.333 as a
/// double, @0xdd0c8).
pub const HELD_FACTOR: f64 = 0.333;

/// The engine ticks after its last touch a lock lapses at: the
/// original's 30 sixtieths of a second (`TickCount` ≥ +0xb4 + 30), at 30
/// ticks a second.
pub const LAPSE_TICKS: u32 = 15;

/// A pull's step, from the player's `_ShipAccelRate` `player_accel`, its
/// thrust as it flies (ionized, under the flight's divisor).
#[must_use]
pub fn step(player_accel: f32) -> f32 {
    player_accel * STEP_SHARE
}

/// Velocity axis `v` pulled toward `target` by `step` (see the module
/// docs): down by the step while above `target` + `step`, then up by it
/// while below `target` − `step`, then snapped to `target` while strictly
/// within `step` of it.
#[must_use]
pub fn pull_axis(v: f32, target: f32, step: f32) -> f32 {
    let mut v = v;
    if v > target + step {
        v -= step;
    }
    if target - step > v {
        v += step;
    }
    if (v - target).abs() < step {
        v = target;
    }
    v
}

/// `velocity` pulled toward `target` by `step` on each axis
/// ([`pull_axis`]).
#[must_use]
pub fn pull(velocity: Vec2, target: Vec2, step: f32) -> Vec2 {
    Vec2::new(
        pull_axis(velocity.x, target.x, step),
        pull_axis(velocity.y, target.y, step),
    )
}

/// Whether `lock`, the lock of `me`, names a holder other than `me`: the
/// test every × 0.333 site and the jump refusal make.
#[must_use]
pub fn held_by_other(lock: Option<TractorLock>, me: ShipRef) -> bool {
    lock.is_some_and(|lock| lock.holder != me)
}

impl TractorLock {
    /// Counts an advance pass since its last touch.
    pub fn age(&mut self) {
        self.ticks_since_touch = self.ticks_since_touch.saturating_add(1);
    }

    /// Whether it has lapsed: [`LAPSE_TICKS`] or more since its last
    /// touch.
    #[must_use]
    pub fn lapsed(&self) -> bool {
        self.ticks_since_touch >= LAPSE_TICKS
    }
}

/// What a hold reads of the ship holding another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Holder {
    /// Gone from the system: its slot inactive.
    Gone,
    /// In the system, moving at `velocity`, `intact` unless disabled or
    /// dying.
    There {
        /// How it moves.
        velocity: Vec2,
        /// Whether it is neither disabled nor dying.
        intact: bool,
    },
}

/// One pass of an NPC's hold (`_HandleShip`, see the module docs): `me`,
/// moving at `velocity`, held by `lock`, its holder as `holder_of` gives it,
/// the pull's [`step`] `step`, aging on an `advance` pass alone. Pulled
/// toward its holder's velocity, or rest when it holds itself, unless the
/// holder is gone; then released when the holder is gone, disabled or
/// dying; then aged, and lapsed.
pub fn npc_pass(
    lock: &mut Option<TractorLock>,
    me: ShipRef,
    velocity: &mut Vec2,
    holder_of: impl FnOnce(ShipRef) -> Holder,
    step: f32,
    advance: bool,
) {
    let Some(held) = *lock else {
        return;
    };
    if held.holder == me {
        *velocity = pull(*velocity, Vec2::ZERO, step);
    } else {
        match holder_of(held.holder) {
            Holder::Gone => *lock = None,
            Holder::There {
                velocity: target,
                intact,
            } => {
                *velocity = pull(*velocity, target, step);
                if !intact {
                    *lock = None;
                }
            }
        }
    }
    lapse(lock, advance);
}

/// One pass of the player's hold (`_HandlePlayer`, see the module docs):
/// as [`npc_pass`], but released first, and not pulled on the pass it is
/// released.
pub fn player_pass(
    lock: &mut Option<TractorLock>,
    velocity: &mut Vec2,
    holder_of: impl FnOnce(ShipRef) -> Holder,
    step: f32,
    advance: bool,
) {
    let Some(held) = *lock else {
        return;
    };
    let target = if held.holder == ShipRef::Player {
        Vec2::ZERO
    } else {
        match holder_of(held.holder) {
            Holder::There {
                velocity,
                intact: true,
            } => velocity,
            Holder::Gone | Holder::There { intact: false, .. } => {
                *lock = None;
                return;
            }
        }
    };
    *velocity = pull(*velocity, target, step);
    lapse(lock, advance);
}

/// Ages `lock` on an `advance` pass, and drops it once it has lapsed.
fn lapse(lock: &mut Option<TractorLock>, advance: bool) {
    if !advance {
        return;
    }
    if let Some(held) = lock {
        held.age();
        if held.lapsed() {
            *lock = None;
        }
    }
}

/// `own`, an NPC's handling, as it flies while another ship holds it
/// ([`held_by_other`]), its class turning `class_turn`: its top speed
/// × [`HELD_FACTOR`], its turn × that floored again as `_ShipTurnRate`
/// floors an NPC's ([`floored_turn`]), and its thrust × that in place of
/// the × 2 its engine thrust holds, all in doubles.
#[must_use]
pub fn held_handling(own: Handling, class_turn: f32) -> Handling {
    let held = |value: f64| (value * HELD_FACTOR) as f32;
    Handling {
        max_speed: held(f64::from(own.max_speed)),
        turn_rate: floored_turn(class_turn, held(f64::from(own.turn_rate)), Flyer::Npc),
        accel: held(f64::from(own.accel) / 2.0),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::NpcId;

    const ME: ShipRef = ShipRef::Npc(NpcId(1));
    const OTHER: ShipRef = ShipRef::Npc(NpcId(2));

    #[test]
    fn an_axis_steps_down_and_up_and_snaps_strictly_within_the_step() {
        assert_eq!(pull_axis(5.0, 1.0, 1.0), 4.0, "down");
        assert_eq!(pull_axis(-3.0, 1.0, 1.0), -2.0, "up");
        assert_eq!(pull_axis(1.5, 1.0, 1.0), 1.0, "snapped within");
        assert_eq!(pull_axis(0.5, 1.0, 1.0), 1.0, "snapped within below");
        assert_eq!(pull_axis(2.0, 1.0, 1.0), 2.0, "exactly the step is kept");
        assert_eq!(pull_axis(0.0, 1.0, 1.0), 0.0, "exactly the step below");
        // Stepped down to exactly within: then snapped.
        assert_eq!(pull_axis(2.5, 1.0, 1.0), 1.0);
        assert_eq!(pull_axis(-0.5, 1.0, 1.0), 1.0);
        assert_eq!(pull_axis(3.0, 3.0, 0.0), 3.0, "no step");
    }

    #[test]
    fn a_pull_moves_each_axis_on_its_own() {
        assert_eq!(
            pull(Vec2::new(5.0, -5.0), Vec2::new(1.0, 1.0), 1.0),
            Vec2::new(4.0, -4.0)
        );
        assert_eq!(
            pull(Vec2::new(1.5, -5.0), Vec2::ZERO, 2.0),
            Vec2::new(0.0, -3.0)
        );
    }

    #[test]
    fn the_step_is_a_quarter_of_the_players_thrust() {
        assert_eq!(step(0.2), 0.05);
        assert_eq!(STEP_SHARE.to_bits(), 0x3e80_0000);
        assert_eq!(HELD_FACTOR.to_bits(), 0x3fd5_4fdf_3b64_5a1d);
    }

    #[test]
    fn a_lock_lapses_at_15_ticks_since_its_touch() {
        let mut lock = TractorLock::by(OTHER);
        for _ in 0..14 {
            lock.age();
        }
        assert!(!lock.lapsed());
        lock.age();
        assert!(lock.lapsed());
        lock.ticks_since_touch = u32::MAX;
        lock.age();
        assert_eq!(lock.ticks_since_touch, u32::MAX);
    }

    #[test]
    fn only_a_holder_other_than_the_ship_holds_it_for_the_factors() {
        assert!(held_by_other(Some(TractorLock::by(OTHER)), ME));
        assert!(!held_by_other(Some(TractorLock::by(ME)), ME));
        assert!(!held_by_other(None, ME));
    }

    fn there(velocity: Vec2, intact: bool) -> impl FnOnce(ShipRef) -> Holder {
        move |_| Holder::There { velocity, intact }
    }

    #[test]
    fn an_npc_is_pulled_toward_its_holders_velocity_and_aged() {
        let mut lock = Some(TractorLock::by(OTHER));
        let mut velocity = Vec2::new(5.0, 0.0);
        let mut asked = None;
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            |ship| {
                asked = Some(ship);
                Holder::There {
                    velocity: Vec2::new(0.0, 3.0),
                    intact: true,
                }
            },
            1.0,
            true,
        );
        assert_eq!(asked, Some(OTHER));
        assert_eq!(velocity, Vec2::new(4.0, 1.0));
        assert_eq!(lock.map(|lock| lock.ticks_since_touch), Some(1));
    }

    #[test]
    fn a_self_held_npc_brakes_toward_rest_and_asks_for_no_holder() {
        let mut lock = Some(TractorLock::by(ME));
        let mut velocity = Vec2::new(5.0, -0.5);
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            |_| panic!("no holder to holder_of at"),
            1.0,
            true,
        );
        assert_eq!(velocity, Vec2::new(4.0, 0.0));
        assert!(lock.is_some());
    }

    #[test]
    fn an_npc_whose_holder_is_disabled_is_pulled_then_released() {
        let mut lock = Some(TractorLock::by(OTHER));
        let mut velocity = Vec2::new(5.0, 0.0);
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            there(Vec2::ZERO, false),
            1.0,
            false,
        );
        assert_eq!(velocity, Vec2::new(4.0, 0.0));
        assert_eq!(lock, None);
    }

    #[test]
    fn an_npc_whose_holder_is_gone_is_released_unpulled() {
        let mut lock = Some(TractorLock::by(OTHER));
        let mut velocity = Vec2::new(5.0, 0.0);
        npc_pass(&mut lock, ME, &mut velocity, |_| Holder::Gone, 1.0, true);
        assert_eq!(velocity, Vec2::new(5.0, 0.0));
        assert_eq!(lock, None);
    }

    #[test]
    fn a_hold_ages_on_the_advance_pass_alone_and_lapses_after_its_pull() {
        let mut lock = Some(TractorLock {
            holder: OTHER,
            ticks_since_touch: 13,
        });
        let mut velocity = Vec2::new(5.0, 0.0);
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            there(Vec2::ZERO, true),
            1.0,
            false,
        );
        assert_eq!(
            lock.map(|lock| lock.ticks_since_touch),
            Some(13),
            "not aged"
        );
        assert_eq!(velocity.x, 4.0, "pulled on the second pass");
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            there(Vec2::ZERO, true),
            1.0,
            true,
        );
        assert_eq!(lock.map(|lock| lock.ticks_since_touch), Some(14));
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            there(Vec2::ZERO, true),
            1.0,
            true,
        );
        assert_eq!(velocity.x, 2.0, "still pulled on the 15th");
        assert_eq!(lock, None, "lapsed");
        npc_pass(
            &mut lock,
            ME,
            &mut velocity,
            there(Vec2::ZERO, true),
            1.0,
            true,
        );
        assert_eq!(velocity.x, 2.0, "no lock, no pull");
    }

    #[test]
    fn the_player_is_released_before_its_pull_and_not_pulled_then() {
        for holder in [
            Holder::Gone,
            Holder::There {
                velocity: Vec2::ZERO,
                intact: false,
            },
        ] {
            let mut lock = Some(TractorLock::by(OTHER));
            let mut velocity = Vec2::new(5.0, 0.0);
            player_pass(&mut lock, &mut velocity, |_| holder, 1.0, true);
            assert_eq!(velocity, Vec2::new(5.0, 0.0), "{holder:?}");
            assert_eq!(lock, None, "{holder:?}");
        }
    }

    #[test]
    fn the_player_is_pulled_toward_its_holder_or_rest_and_lapses() {
        let mut lock = Some(TractorLock {
            holder: OTHER,
            ticks_since_touch: 14,
        });
        let mut velocity = Vec2::new(5.0, 0.0);
        player_pass(
            &mut lock,
            &mut velocity,
            there(Vec2::new(0.0, 3.0), true),
            1.0,
            false,
        );
        assert_eq!(velocity, Vec2::new(4.0, 1.0));
        assert!(lock.is_some(), "not aged on the second pass");
        player_pass(
            &mut lock,
            &mut velocity,
            there(Vec2::new(0.0, 3.0), true),
            1.0,
            true,
        );
        assert_eq!(velocity, Vec2::new(3.0, 2.0), "pulled on the 15th");
        assert_eq!(lock, None);
        let mut own = Some(TractorLock::by(ShipRef::Player));
        let mut velocity = Vec2::new(-5.0, 0.0);
        player_pass(&mut own, &mut velocity, |_| panic!("itself"), 1.0, true);
        assert_eq!(velocity, Vec2::new(-4.0, 0.0));
        assert_eq!(own.map(|lock| lock.ticks_since_touch), Some(1));
        let mut none = None;
        player_pass(&mut none, &mut velocity, |_| panic!("no lock"), 1.0, true);
        assert_eq!(velocity, Vec2::new(-4.0, 0.0));
    }

    #[test]
    fn a_held_npc_flies_at_a_third_turns_at_a_floored_third_and_thrusts_a_sixth() {
        let own = Handling {
            max_speed: 6.0,
            accel: 0.12,
            turn_rate: 9.0,
        };
        let held = held_handling(own, 9.0);
        assert_eq!(held.max_speed, (6.0 * 0.333_f64) as f32);
        assert_eq!(held.turn_rate, (9.0 * 0.333_f64) as f32);
        assert_eq!(held.accel, (f64::from(0.12_f32) * 0.1665) as f32);
        // A class turning 1° or more is floored at 1°; one under it isn't.
        let slow = Handling {
            turn_rate: 2.0,
            ..own
        };
        assert_eq!(held_handling(slow, 2.0).turn_rate, 1.0);
        let slower = Handling {
            turn_rate: 0.9,
            ..own
        };
        assert_eq!(
            held_handling(slower, 0.9).turn_rate,
            (f64::from(0.9_f32) * 0.333) as f32
        );
    }
}
