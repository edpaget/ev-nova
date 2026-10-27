//! Where the player's escorts arrive with it: the arrival push of
//! `_SetupShipsInSystem` (@0x42b61), the engine reading of
//! [`RuleKey::EscortArrival`](crate::RuleKey::EscortArrival). The session
//! applies it (`nova_sim::Session`'s `escort_arrival` module).
//!
//! **Callers.** `_SetupShipsInSystem(sys, restock)` is called by
//! `_DoNewPilot` @0x1937a (restock 1), `_MainScreenChoice` @0x1a8c5 and
//! `_DoMainScreen` @0x1afeb (a load, 1), `_HandlePlayerDockRequest`
//! @0x672e0 (a gate exit, 0) and @0x674c2 (a take-off, 1), `_HandlePlayer`
//! @0x69114 (1), and the jump's arrival in `_HandlePlayer` @0x6c1f9 (0).
//! Only the jump's arrival sets the player's jump state +0x50 to -999.0
//! before the call (@0x6c1e3) and back to 0 after it (@0x6c20b); no other
//! caller writes it, so it is 0 there.
//!
//! **Order inside it.**
//! 1. `_DeleteAllComputerShips(0)` @0x42b92, then the respawn loop
//!    @0x42b97-0x42c88: each kept ship the player leads is re-placed by
//!    `_RespawnEscort` with the player's position, heading and velocity.
//! 2. The provisional snap @0x42c8e-0x42ca1:
//!    `_AICalcFormationPositions(player, 1)`. The mission escorts were
//!    deleted in step 1 and come back only in step 4, so the slots and the
//!    spacing count the kept hired escorts and fighters alone. `maintain`
//!    1 moves each onto its slot, by position (`_AIMaintainFormation`
//!    @0x84fc2-0x84fce).
//! 3. The arrival push @0x42ca6-0x42d9e, only while the player's +0x50
//!    isn't 0.0 (`ucomiss`/`jp`/`je` @0x42cab-0x42cb1), so only on a
//!    jump's arrival. Its distance ([`push_distance`], @0x42cb7-0x42cec)
//!    sums an f32 speed over [`PUSH_TICKS`] ticks, starting at
//!    [`PUSH_START`] (f32 @0xdd728) and dropping [`PUSH_SLOWING`] (f64
//!    @0xdd0f0) a tick, rounded to f32 each time, the sum taken before
//!    each drop: 891.7344 (f32 bits 0x445eef01), where the closed form
//!    45 × 39 − 1.165 × 741 gives 891.735. Its loop @0x42cf9-0x42d9e takes
//!    every active ship (+0xb8) whose leader (+0x9a) is the player, the
//!    kept escorts alone at this point, and for each ([`pushed`]):
//!    `_Accel((trunc(+0x44) + 180) mod 360, 891.7344, &+0x18)`
//!    @0x42d14-0x42d55 moves it back along its whole-degree heading, the
//!    player's; +0x50 = -999.0 @0x42d5e starts its hyperspace glide; +0xa4
//!    = 0 @0x42d68 clears its warp-out stamp; and
//!    `_Accel(trunc(+0x44), 50.0, &+0x20)` @0x42d75-0x42d8d adds 50 to the
//!    velocity it has, the player's, along its heading. It plays no sound.
//! 4. The mission ships @0x42da4-0x43194 (`_MissionDudeSpawn` @0x42ebb):
//!    a `ShipBehav` 1 ship becomes a mission escort (@0x4309f-0x430da),
//!    taking the player's heading only when `restock` is 1
//!    (@0x430e4-0x430ef).
//! 5. The re-snap @0x4319a-0x431b2, only when `restock` is 1:
//!    `_AICalcFormationPositions(player, 1)` again, counting the mission
//!    escorts now, each ship moved onto its slot by position, a mission
//!    escort keeping its spawn velocity and speed.
//! 6. Persons, fleets and random ships from @0x431b7: system traffic.
//!
//! **The glide.** +0x50 below -900 makes `_AIDispatch` set goal 8 and
//! sub-goal 0xa (@0x8fcf5-0x8fd18), whose handler holds +0x50 at -999.0
//! (@0x8f3c8-0x8f3d9); low mode 10 sets +0x34 = -50.0 and +0x30 = -1.165
//! (@0x88114-0x88136) and `_HandleShip` runs the negative-speed path
//! (@0x34bc0-0x34d32). It is the glide of a ship arriving by hyperspace
//! (`_HyperShipSpawn` sets the same -999.0 @0x4257f), which the traffic
//! models as [`Mode::JumpingIn`](crate::traffic::npc::Mode::JumpingIn)
//! (the [`autopilot`](crate::traffic::autopilot)'s glide,
//! [`glide_speed`](crate::traffic::spawn::glide_speed)): 50, then 1.165
//! slower a tick, to the lower of its own top speed and its lead's, the
//! player's. The escort then goes back to formation. No escort-specific
//! animation exists, and nothing in steps 2, 3 or 5 plays a sound.
//!
//! The NaN branch of the +0x50 test (@0x42caf) is left out: the player's
//! +0x50 is never NaN.

use crate::flight::ShipState;
use crate::traffic::spawn::JUMP_IN_SPEED;
use crate::trig::{accel, whole};

/// How many ticks the push's distance is summed over: 0x27 (@0x42cb7).
pub const PUSH_TICKS: u32 = 39;
/// The speed the push's sum starts at: the f32 at @0xdd728.
pub const PUSH_START: f32 = 45.0;
/// How much the push's speed drops each tick: the f64 at @0xdd0f0.
pub const PUSH_SLOWING: f64 = 1.165;

/// How far back an arriving escort is pushed: [`PUSH_START`] summed over
/// [`PUSH_TICKS`] ticks, [`PUSH_SLOWING`] slower each, in the original's
/// f32 (`cvtss2sd`/`subsd`/`cvtsd2ss` @0x42cb7-0x42cec), 891.7344.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn push_distance() -> f32 {
    let mut speed = PUSH_START;
    let mut sum = 0.0_f32;
    for _ in 0..PUSH_TICKS {
        sum += speed;
        speed = (f64::from(speed) - PUSH_SLOWING) as f32;
    }
    sum
}

/// An escort's `state` pushed back for its glide in (@0x42d14-0x42d8d):
/// [`push_distance`] back along its whole-degree heading, its velocity
/// gaining [`JUMP_IN_SPEED`] along it, its heading kept.
#[must_use]
pub fn pushed(state: ShipState) -> ShipState {
    let heading = whole(state.heading);
    ShipState {
        position: state.position + accel((heading + 180) % 360, push_distance()),
        velocity: state.velocity + accel(heading, JUMP_IN_SPEED),
        heading: state.heading,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;

    #[test]
    fn the_push_is_the_originals_f32_sum() {
        assert_eq!(
            push_distance().to_bits(),
            0x445e_ef01,
            "{}",
            push_distance()
        );
        assert!((push_distance() - 891.735).abs() < 1e-3);
    }

    #[test]
    fn an_escort_is_pushed_back_along_its_whole_heading_and_gains_50_along_it() {
        let state = ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(3.0, 4.0),
            heading: 90.7,
        };
        let out = pushed(state);
        assert_eq!(
            out.position,
            Vec2::new(10.0, 20.0) + accel(270, push_distance())
        );
        assert!((out.position - Vec2::new(10.0 - 891.7344, 20.0)).length() < 1e-3);
        assert_eq!(out.velocity, Vec2::new(3.0, 4.0) + accel(90, 50.0));
        assert!((out.velocity - Vec2::new(53.0, 4.0)).length() < 1e-4);
        assert_eq!(out.heading, 90.7);
    }

    #[test]
    fn the_push_back_wraps_past_360() {
        let state = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            heading: 270.0,
        };
        let out = pushed(state);
        assert_eq!(out.position, accel(90, push_distance()));
        assert!((out.position - Vec2::new(891.7344, 0.0)).length() < 1e-3);
        assert_eq!(out.velocity, accel(270, 50.0));
    }
}
