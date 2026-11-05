//! The gate autopilot's pilot: how it steers the player's ship to a gate
//! and when it presses L there, by
//! [`RuleKey::GateAutopilot`](crate::RuleKey::GateAutopilot)'s other
//! reading, the new game's.
//!
//! The original has no autopilot that flies to a stellar: the face key
//! only turns the ship (`_loadKeys`' `autopilot`, @0x6aeba-0x6aff3), J's
//! pre-jump brakes and turns for a hyperspace jump only (@0x6b6a2-0x6b868),
//! and a gate is entered by the land key's dock request
//! (`_HandlePlayerDockRequest` @0x66691-0x67ef9, its hypergate branch
//! @0x66ed1). So this is a new reading, and it drives only routines the
//! engine has already ported: nothing here flies the ship. [`steer`] gives
//! the controls the player would hold this tick, its turn keys and thrust
//! (and reverse for an inertialess hull), and the session flies the ship
//! by them through the player's own step, so every flight rule applies
//! to it as it does to a player.
//!
//! **The control law.** Distances are in pixels and speeds in pixels a
//! tick; the frame's gain scales a tick's turn and thrust alike, so the
//! law's speeds and distances don't depend on it.
//!
//! - An inertial hull corrects its velocity. The velocity it wants points
//!   at the gate's centre at `min(top speed, sqrt(2 × accel × max(0, d −
//!   v × T)))`, where `d` is the distance, `v` the speed and `T` the
//!   ticks a half turn takes at the turn keys' step
//!   ([`step_of`]): the margin pays for the
//!   turn it makes before it brakes. It turns by the turn keys toward the
//!   heading of (wanted − current), and holds still within half a turn
//!   step of it. It thrusts only when its heading is within one turn step
//!   of that aim and the difference is more than half a thrust (accel ×
//!   gain / 2), so it settles within a thrust of still.
//! - An inertialess hull holds a speed along its heading, so it turns
//!   toward the gate's bearing instead. It wants the same speed with no
//!   margin, none while it faces more than a quarter turn off the bearing.
//!   It thrusts while its speed is short of that by more than half a
//!   thrust and its heading is within one turn step of the bearing, and
//!   holds reverse while its speed is over it by more than half a thrust.
//!
//! A hull whose turn keys turn nothing (a turn rate below 1° by the
//! original's whole degrees) never turns, and a hull with no thrust never
//! moves; neither arrives, and the session's per-step disengage list is
//! what lets go of it.
//!
//! **When it presses L** ([`Steer::press_land`]). Only for a press the
//! dock request takes without a beep (`_beepSnd[3]`, `snd ` 153;
//! [`LandingRefusal::beeps`](crate::landing::LandingRefusal::beeps)), so
//! the autopilot never spams the refusal while the ship settles:
//!
//! - **To request clearance:** the gate is inhabited, the dock request is
//!   not the gate's, and the ship is within
//!   [`NEAR`](clearance::NEAR) of it on both axes. The press is
//!   silent and makes the request at
//!   [`REQUESTED_NEAR`](clearance::REQUESTED_NEAR), which the
//!   approach clears on its next pass.
//! - **To enter:** [`check_landing`] passes for the gate with the
//!   request's counter, or with [`CLEARED`] at an uninhabited gate, whose
//!   first press starts its request there ("No response.",
//!   @0x66b51-0x66bcd).
//!
//! It never presses for a gate the press would refuse with a beep before
//! those: one that can't be landed on, or that the player's system no
//! longer holds ([`Gate::held`]; `NotLandable`, @0x66977-0x669ae), or
//! while the ship is cloaked ([`Gate::cloaked`]; `Cloaked`,
//! `_ShipIsCloaked` @0x66a8e). Cloaked, it keeps steering to the gate and
//! holds over it, and presses once the cloak is off: the player can
//! decloak and the flight goes on. The refusals that don't beep, the
//! traffic control's denial (`Denied`, which plays `_beepSnd[1]`) and a
//! fee the pilot can't pay (`CannotPayFee`, silent), can't be foreseen
//! here, so the press is made and the session lets go of the autopilot on
//! them.
//!
//! The autopilot makes no sound of its own: the dock request's refusals
//! and greeting, and the gate's exit, raise theirs as they do for a
//! player's L.

use crate::catalog::LandingSite;
use crate::flight::{Controls, ShipState, Turn, heading_of, shortest_turn};
use crate::geometry::Vec2;
use crate::handling::Handling;
use crate::landing::{LandingReach, StellarFlags, check_landing, is_landable};
use crate::rulebook::RuleSource;
use crate::speed_mult::SpeedMult;
use nova_sim_base::player_turn::{PlayerTurn, step_of};
use nova_sim_state::clearance::{self, CLEARED, DockRequest};

/// The player's ship as the autopilot flies it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Craft {
    /// Where it is, how it moves and which way it faces.
    pub state: ShipState,
    /// The speed it holds along its heading when it is inertialess, `None`
    /// for an inertial ship.
    pub inertialess: Option<f32>,
    /// How it flies.
    pub handling: Handling,
    /// How the player's turn keys turn it, by
    /// [`RuleKey::PlayerTurn`](crate::RuleKey::PlayerTurn).
    pub turn: PlayerTurn,
    /// The frame's gain, which its turn and thrust are scaled by.
    pub gain: SpeedMult,
}

/// The gate the autopilot flies to, and what the dock request reads of a
/// press there.
#[derive(Clone, Copy, Debug)]
pub struct Gate<'a> {
    /// The gate.
    pub site: &'a LandingSite,
    /// How near its centre the ship must be to enter
    /// ([`landing_reach`](crate::landing::landing_reach)).
    pub reach: LandingReach,
    /// Whether the player's system still holds it, owned and shown: the
    /// dock request refuses one it doesn't as not landable, with a beep
    /// (@0x6699a-0x669aa).
    pub held: bool,
    /// The dock request standing, whichever stellar it is for.
    pub dock: Option<DockRequest>,
    /// The landing speed's reading,
    /// [`RuleKey::LandingSpeed`](crate::RuleKey::LandingSpeed).
    pub speed: RuleSource,
    /// Whether the ship is cloaked: the dock request refuses it with a
    /// beep (`_ShipIsCloaked` @0x66a8e), so the autopilot keeps flying and
    /// doesn't press.
    pub cloaked: bool,
}

/// What the autopilot does this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Steer {
    /// The controls the player's ship flies by this tick.
    pub controls: Controls,
    /// Whether it asks for L: a press the dock request takes without a
    /// beep (see the module docs).
    pub press_land: bool,
}

/// What the autopilot flying `craft` to `gate` does this tick: see the
/// module docs.
#[must_use]
pub fn steer(craft: &Craft, gate: &Gate<'_>) -> Steer {
    let controls = match craft.inertialess {
        None => inertial(craft, gate.site.position),
        Some(speed) => inertialess(craft, speed, gate.site.position),
    };
    Steer {
        controls,
        press_land: presses(craft, gate),
    }
}

/// Half a turn, in degrees.
const HALF_TURN: f32 = 180.0;

/// A quarter turn, in degrees: an inertialess hull facing further off the
/// gate's bearing wants to stop.
const QUARTER_TURN: f32 = 90.0;

/// How far `craft`'s turn keys turn it this tick, in degrees: the turn
/// step × the gain.
fn turn_step(craft: &Craft) -> f32 {
    craft
        .gain
        .times(step_of(craft.handling.turn_rate, craft.turn))
}

/// Half of one tick's thrust: the velocity's slack the law leaves alone.
fn half_thrust(craft: &Craft) -> f32 {
    craft.gain.times(craft.handling.accel) / 2.0
}

/// The speed a hull of `accel` wants `room` pixels short of where it
/// stops, at most `top`: the speed it brakes from to none in that room.
fn wanted_speed(accel: f32, room: f32, top: f32) -> f32 {
    (2.0 * accel * room.max(0.0)).sqrt().min(top)
}

/// The turn keys that bring `heading` toward `aim` at `step` a tick:
/// none within half a step of it.
fn turn_toward(heading: f32, aim: f32, step: f32) -> Turn {
    let off = shortest_turn(heading, aim);
    if off.abs() <= step / 2.0 {
        Turn::None
    } else if off < 0.0 {
        Turn::Left
    } else {
        Turn::Right
    }
}

/// An inertial hull's controls toward `to` (see the module docs).
fn inertial(craft: &Craft, to: Vec2) -> Controls {
    let ShipState {
        position,
        velocity,
        heading,
    } = craft.state;
    let step = turn_step(craft);
    let toward = to - position;
    let distance = toward.length();
    let speed = velocity.length();
    let full_step = step_of(craft.handling.turn_rate, craft.turn);
    let half_turn_ticks = if full_step > 0.0 {
        HALF_TURN / full_step
    } else {
        0.0
    };
    let room = distance - speed * half_turn_ticks;
    let wanted = if distance > 0.0 {
        toward * (wanted_speed(craft.handling.accel, room, craft.handling.max_speed) / distance)
    } else {
        Vec2::ZERO
    };
    let error = wanted - velocity;
    if error.length() <= half_thrust(craft) {
        return Controls::default();
    }
    let aim = heading_of(error);
    Controls {
        turn: turn_toward(heading, aim, step),
        thrust: shortest_turn(heading, aim).abs() <= step,
        ..Controls::default()
    }
}

/// An inertialess hull's controls toward `to`, holding `speed` along its
/// heading (see the module docs).
fn inertialess(craft: &Craft, speed: f32, to: Vec2) -> Controls {
    let ShipState {
        position, heading, ..
    } = craft.state;
    let step = turn_step(craft);
    let toward = to - position;
    let distance = toward.length();
    let (turn, off) = if distance > 0.0 {
        let bearing = heading_of(toward);
        (
            turn_toward(heading, bearing, step),
            shortest_turn(heading, bearing).abs(),
        )
    } else {
        (Turn::None, HALF_TURN)
    };
    let wanted = if off <= QUARTER_TURN {
        wanted_speed(craft.handling.accel, distance, craft.handling.max_speed)
    } else {
        0.0
    };
    let slack = half_thrust(craft);
    Controls {
        turn,
        thrust: speed < wanted - slack && off <= step,
        reverse: speed > wanted + slack,
        ..Controls::default()
    }
}

/// Whether the autopilot flying `craft` asks for L at `gate`: only for a
/// press the dock request takes without a beep (see the module docs).
fn presses(craft: &Craft, gate: &Gate<'_>) -> bool {
    let site = gate.site;
    if gate.cloaked || !gate.held || !is_landable(site) {
        return false;
    }
    let enters =
        |permission| check_landing(&craft.state, site, permission, gate.reach, gate.speed).is_ok();
    if site.flags & StellarFlags::UNINHABITED != 0 {
        return enters(CLEARED);
    }
    match gate.dock.filter(|request| request.stellar == site.id) {
        Some(request) => enters(request.permission),
        None => clearance::near(site.position - craft.state.position),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::StellarId;
    use crate::flight::{Caps, FlightRules, inertialess_player_step, player_step};
    use crate::landing::{LandOutcome, StellarShown, TypeWidth, land_or_select, landing_reach};
    use crate::rulebook::Rulebook;
    use nova_sim_state::clearance::{NEAR, REQUESTED_NEAR};

    const GATE: StellarId = StellarId(300);

    /// A 100-pixel gate at the system's centre, inhabited and landable.
    fn site() -> LandingSite {
        LandingSite {
            flags2: crate::landing::HYPERGATE,
            ..nova_sim_catalog::fixture::planet(GATE.0, 0.0, 0.0)
        }
    }

    /// The original's reach for [`site`]: 175 pixels each way.
    fn reach(site: &LandingSite) -> LandingReach {
        landing_reach(
            site,
            StellarShown {
                frame: true,
                type_width: TypeWidth::Frame,
            },
            RuleSource::Engine,
        )
    }

    /// A shuttle: 3 pixels a tick, 0.06 a tick's thrust, 3° a tick.
    fn shuttle() -> Handling {
        Handling::from_totals(300, 300, 30.0)
    }

    fn craft(position: (f32, f32), velocity: (f32, f32), heading: f32) -> Craft {
        Craft {
            state: ShipState {
                position: Vec2::new(position.0, position.1),
                velocity: Vec2::new(velocity.0, velocity.1),
                heading,
            },
            inertialess: None,
            handling: shuttle(),
            turn: PlayerTurn::WholeDegrees,
            gain: SpeedMult::FULL,
        }
    }

    fn gate(site: &LandingSite, dock: Option<DockRequest>) -> Gate<'_> {
        Gate {
            site,
            reach: reach(site),
            held: true,
            dock,
            speed: RuleSource::Engine,
            cloaked: false,
        }
    }

    /// The dock request for the gate at `permission`.
    fn request(permission: i16) -> DockRequest {
        DockRequest {
            stellar: GATE,
            permission,
        }
    }

    fn controls(turn: Turn, thrust: bool) -> Controls {
        Controls {
            turn,
            thrust,
            ..Controls::default()
        }
    }

    // ---- The inertial law ----

    #[test]
    fn far_and_at_rest_it_turns_toward_the_gate_then_thrusts_once_aligned() {
        let site = site();
        // The gate is straight up (heading 0) from (0, 1000).
        let facing_right = steer(&craft((0.0, 1000.0), (0.0, 0.0), 90.0), &gate(&site, None));
        assert_eq!(facing_right.controls, controls(Turn::Left, false));
        let facing_left = steer(&craft((0.0, 1000.0), (0.0, 0.0), 270.0), &gate(&site, None));
        assert_eq!(facing_left.controls, controls(Turn::Right, false));
        let aligned = steer(&craft((0.0, 1000.0), (0.0, 0.0), 0.0), &gate(&site, None));
        assert_eq!(aligned.controls, controls(Turn::None, true));
        // Within a turn step but beyond half of one, it turns and thrusts.
        let nearly = steer(&craft((0.0, 1000.0), (0.0, 0.0), 2.0), &gate(&site, None));
        assert_eq!(nearly.controls, controls(Turn::Left, true));
        // Beyond a turn step it doesn't thrust yet.
        let off = steer(&craft((0.0, 1000.0), (0.0, 0.0), 4.0), &gate(&site, None));
        assert_eq!(off.controls, controls(Turn::Left, false));
    }

    #[test]
    fn fast_and_near_it_turns_against_the_excess_velocity_and_brakes() {
        let site = site();
        // 150 out at 3 a tick toward the gate, less than the 180 it
        // covers in a half turn: it wants to stop.
        let facing_in = steer(&craft((0.0, 150.0), (0.0, -3.0), 0.0), &gate(&site, None));
        assert_ne!(facing_in.controls.turn, Turn::None);
        assert!(!facing_in.controls.thrust);
        let facing_back = steer(&craft((0.0, 150.0), (0.0, -3.0), 180.0), &gate(&site, None));
        assert_eq!(facing_back.controls, controls(Turn::None, true));
    }

    #[test]
    fn with_sideways_drift_its_aim_leans_against_the_drift() {
        let site = site();
        // Drifting right below the gate, facing it: it turns left of up.
        let right = steer(&craft((0.0, 1000.0), (2.0, 0.0), 0.0), &gate(&site, None));
        assert_eq!(right.controls.turn, Turn::Left);
        let left = steer(&craft((0.0, 1000.0), (-2.0, 0.0), 0.0), &gate(&site, None));
        assert_eq!(left.controls.turn, Turn::Right);
        // Facing the aim against a rightward drift (up and left), it thrusts.
        let wanted = (2.0 * shuttle().accel * (1000.0 - 2.0 * 60.0))
            .sqrt()
            .min(3.0);
        let aim = heading_of(Vec2::new(-2.0, -wanted));
        let leaning = steer(&craft((0.0, 1000.0), (2.0, 0.0), aim), &gate(&site, None));
        assert_eq!(leaning.controls, controls(Turn::None, true));
    }

    #[test]
    fn within_half_a_thrust_of_the_wanted_velocity_it_neither_thrusts_nor_turns() {
        let site = site();
        // At rest over the gate's centre it wants no velocity.
        let still = steer(&craft((0.0, 0.0), (0.0, 0.0), 90.0), &gate(&site, None));
        assert_eq!(still.controls, Controls::default());
        // A drift within half a thrust (0.03) of none, facing away: held.
        let drifting = steer(&craft((0.0, 0.0), (0.029, 0.0), 0.0), &gate(&site, None));
        assert_eq!(drifting.controls, Controls::default());
        // A drift past it is braked.
        let braking = steer(&craft((0.0, 0.0), (0.031, 0.0), 0.0), &gate(&site, None));
        assert_ne!(braking.controls, Controls::default());
        // Moving at the wanted velocity, it holds.
        let wanted = (2.0 * shuttle().accel * 1000.0).sqrt().min(3.0);
        let cruising = steer(
            &craft((0.0, 1000.0), (0.0, -wanted), 0.0),
            &gate(&site, None),
        );
        assert_eq!(cruising.controls.turn, Turn::None);
    }

    #[test]
    fn at_full_speed_it_wants_no_more_than_its_top_speed() {
        let site = site();
        // 1,500 out at top speed toward the gate, facing it: no thrust.
        let cruising = steer(&craft((0.0, 1500.0), (0.0, -3.0), 0.0), &gate(&site, None));
        assert_eq!(cruising.controls, Controls::default());
    }

    // ---- The presses ----

    #[test]
    fn near_an_inhabited_gate_with_no_request_it_presses_to_request() {
        let site = site();
        let near = steer(&craft((NEAR, -NEAR), (3.0, 3.0), 0.0), &gate(&site, None));
        assert!(near.press_land);
        // A request for another stellar is replaced by the press.
        let other = Some(DockRequest {
            stellar: StellarId(301),
            permission: CLEARED,
        });
        assert!(steer(&craft((0.0, 200.0), (0.0, 0.0), 0.0), &gate(&site, other)).press_land);
        for far in [(NEAR + 1.0, 0.0), (0.0, NEAR + 1.0), (-NEAR - 1.0, 0.0)] {
            let pressed = steer(&craft(far, (0.0, 0.0), 0.0), &gate(&site, None)).press_land;
            assert!(!pressed, "{far:?}");
        }
    }

    #[test]
    fn with_the_request_near_but_not_yet_cleared_it_does_not_press() {
        let site = site();
        let over = craft((0.0, 0.0), (0.0, 0.0), 0.0);
        assert!(!steer(&over, &gate(&site, Some(request(REQUESTED_NEAR)))).press_land);
        assert!(!steer(&over, &gate(&site, Some(request(0)))).press_land);
    }

    #[test]
    fn over_the_gate_cleared_and_slow_it_presses_and_not_too_fast_or_out_of_reach() {
        let site = site();
        for permission in [CLEARED, CLEARED + 100] {
            let dock = Some(request(permission));
            let slow = craft((174.0, -174.0), (0.75, -0.75), 0.0);
            assert!(steer(&slow, &gate(&site, dock)).press_land);
            let out_of_reach = craft((175.0, 0.0), (0.0, 0.0), 0.0);
            assert!(!steer(&out_of_reach, &gate(&site, dock)).press_land);
            let fast = craft((0.0, 0.0), (0.76, 0.0), 0.0);
            assert!(!steer(&fast, &gate(&site, dock)).press_land);
        }
    }

    #[test]
    fn by_the_other_landing_speed_it_presses_up_to_1_overall() {
        let site = site();
        let bible = |site| Gate {
            speed: RuleSource::Bible,
            ..gate(site, Some(request(CLEARED)))
        };
        // 0.9 on one axis: too fast by the engine, slow enough overall.
        let on_one_axis = craft((0.0, 0.0), (0.9, 0.0), 0.0);
        assert!(steer(&on_one_axis, &bible(&site)).press_land);
        assert!(!steer(&on_one_axis, &gate(&site, Some(request(CLEARED)))).press_land);
        // 0.75 on each axis: slow enough by the engine, too fast overall.
        let on_both = craft((0.0, 0.0), (0.75, 0.75), 0.0);
        assert!(!steer(&on_both, &bible(&site)).press_land);
        assert!(steer(&on_both, &gate(&site, Some(request(CLEARED)))).press_land);
    }

    #[test]
    fn an_uninhabited_gate_gets_one_press_over_it_and_slow_and_none_for_clearance() {
        let site = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..site()
        };
        let over = craft((100.0, 0.0), (0.5, 0.0), 0.0);
        assert!(steer(&over, &gate(&site, None)).press_land);
        let near_not_over = craft((200.0, 0.0), (0.0, 0.0), 0.0);
        assert!(!steer(&near_not_over, &gate(&site, None)).press_land);
        let over_fast = craft((0.0, 0.0), (1.0, 0.0), 0.0);
        assert!(!steer(&over_fast, &gate(&site, None)).press_land);
        // A request standing for it at the near counter changes nothing:
        // the press starts it at cleared again.
        assert!(steer(&over, &gate(&site, Some(request(REQUESTED_NEAR)))).press_land);
    }

    #[test]
    fn it_never_presses_while_cloaked_and_keeps_steering() {
        let site = site();
        let cloaked = |site, dock| Gate {
            cloaked: true,
            ..gate(site, dock)
        };
        let near = craft((0.0, 200.0), (0.0, 0.0), 0.0);
        let steered = steer(&near, &cloaked(&site, None));
        assert!(!steered.press_land);
        assert_eq!(steered.controls, steer(&near, &gate(&site, None)).controls);
        assert_ne!(steered.controls, Controls::default());
        let over = craft((0.0, 0.0), (0.0, 0.0), 0.0);
        assert!(!steer(&over, &cloaked(&site, Some(request(CLEARED)))).press_land);
        assert!(steer(&over, &gate(&site, Some(request(CLEARED)))).press_land);
    }

    #[test]
    fn it_never_presses_at_a_gate_that_cannot_be_landed_on_or_that_the_system_lost() {
        let over = craft((0.0, 0.0), (0.0, 0.0), 0.0);
        let unlandable = LandingSite { flags: 0, ..site() };
        assert!(!steer(&over, &gate(&unlandable, None)).press_land);
        assert!(!steer(&over, &gate(&unlandable, Some(request(CLEARED)))).press_land);
        let destroyed = LandingSite {
            destroyed: true,
            ..site()
        };
        assert!(!steer(&over, &gate(&destroyed, Some(request(CLEARED)))).press_land);
        let site = site();
        let lost = Gate {
            held: false,
            ..gate(&site, Some(request(CLEARED)))
        };
        assert!(!steer(&over, &lost).press_land);
        assert!(
            !steer(
                &over,
                &Gate {
                    held: false,
                    ..gate(&site, None)
                }
            )
            .press_land
        );
    }

    // ---- The inertialess law ----

    fn inertialess(position: (f32, f32), speed: f32, heading: f32) -> Craft {
        let facing = crate::flight::facing(heading) * speed;
        Craft {
            inertialess: Some(speed),
            ..craft(position, (facing.x, facing.y), heading)
        }
    }

    #[test]
    fn inertialess_it_turns_toward_the_gate_and_thrusts_only_aligned_below_the_wanted_speed() {
        let site = site();
        let turning = steer(&inertialess((0.0, 1000.0), 0.0, 90.0), &gate(&site, None));
        assert_eq!(turning.controls, controls(Turn::Left, false));
        let aligned = steer(&inertialess((0.0, 1000.0), 1.0, 0.0), &gate(&site, None));
        assert_eq!(aligned.controls, controls(Turn::None, true));
    }

    #[test]
    fn inertialess_it_holds_reverse_above_the_wanted_speed() {
        let site = site();
        // 50 out at 3 a tick: it wants sqrt(2 × 0.06 × 50), about 2.4.
        let fast = steer(&inertialess((0.0, 50.0), 3.0, 0.0), &gate(&site, None));
        assert_eq!(
            fast.controls,
            Controls {
                reverse: true,
                ..Controls::default()
            }
        );
        // Facing away from the gate it wants to stop.
        let away = steer(&inertialess((0.0, 1000.0), 1.0, 180.0), &gate(&site, None));
        assert!(away.controls.reverse);
        assert!(!away.controls.thrust);
        // At the wanted speed it holds.
        let wanted = (2.0 * shuttle().accel * 1000.0).sqrt().min(3.0);
        let cruising = steer(&inertialess((0.0, 1000.0), wanted, 0.0), &gate(&site, None));
        assert_eq!(cruising.controls, Controls::default());
    }

    /// A hull whose thrust is exactly 0.5 a tick, so the wanted speed
    /// `d` out is exactly `sqrt(d)`, and half a thrust 0.25, up to its top
    /// speed of 6.
    fn half_pixel_thrust(craft: Craft) -> Craft {
        Craft {
            handling: Handling::from_totals(600, 2500, 30.0),
            ..craft
        }
    }

    #[test]
    fn inertialess_it_wants_the_speed_it_brakes_from_in_the_room_left() {
        let site = site();
        let at = |speed| {
            steer(
                &half_pixel_thrust(inertialess((0.0, 4.0), speed, 0.0)),
                &gate(&site, None),
            )
            .controls
        };
        // 4 out it wants sqrt(2 × 0.5 × 4) = 2, with 0.25 of slack.
        assert_eq!(at(2.0), Controls::default());
        assert_eq!(at(1.75), Controls::default(), "short by the slack alone");
        assert_eq!(at(2.25), Controls::default(), "over by the slack alone");
        assert!(at(1.74).thrust);
        assert!(at(2.26).reverse);
        // 9 out it wants 3.
        let nine_out = steer(
            &half_pixel_thrust(inertialess((0.0, 9.0), 3.0, 0.0)),
            &gate(&site, None),
        );
        assert_eq!(nine_out.controls, Controls::default());
    }

    #[test]
    fn inertialess_at_the_gates_centre_it_neither_turns_nor_thrusts() {
        let site = site();
        let still = steer(&inertialess((0.0, 0.0), 0.0, 90.0), &gate(&site, None));
        assert_eq!(still.controls, Controls::default());
    }

    #[test]
    fn within_half_a_turn_step_of_its_aim_it_holds_its_heading() {
        let site = site();
        // A 3° step: 1.2° off is within half of it.
        let near_aim = steer(&craft((0.0, 1000.0), (0.0, 0.0), 1.2), &gate(&site, None));
        assert_eq!(near_aim.controls, controls(Turn::None, true));
        let past = steer(&craft((0.0, 1000.0), (0.0, 0.0), 1.6), &gate(&site, None));
        assert_eq!(past.controls, controls(Turn::Left, true));
    }

    #[test]
    fn a_hull_that_cannot_turn_still_thrusts_when_it_faces_the_gate() {
        let site = site();
        // Maneuver 5 turns 0.5° a tick: none by the original's whole
        // degrees, so it takes no margin for a turn it can't make.
        let stiff = Craft {
            handling: Handling::from_totals(300, 300, 5.0),
            ..craft((0.0, 1000.0), (0.0, -1.0), 0.0)
        };
        assert_eq!(
            steer(&stiff, &gate(&site, None)).controls,
            controls(Turn::None, true)
        );
    }

    #[test]
    fn it_measures_near_from_the_gate_wherever_the_gate_is() {
        let site = LandingSite {
            position: Vec2::new(1000.0, -500.0),
            ..site()
        };
        let near = craft((800.0, -300.0), (0.0, 0.0), 0.0);
        assert!(steer(&near, &gate(&site, None)).press_land);
        let far = craft((1000.0, 0.0), (0.0, 0.0), 0.0);
        assert!(!steer(&far, &gate(&site, None)).press_land);
    }

    // ---- Convergence ----

    /// A sluggish hull: 2 a tick, 0.02 a tick's thrust, 1.5° a tick (1° by
    /// the original's whole degrees).
    fn sluggish() -> Handling {
        Handling::from_totals(200, 100, 15.0)
    }

    /// A fast, light hull: 6 a tick, 0.18 a tick's thrust, 3° a tick.
    fn fast() -> Handling {
        Handling::from_totals(600, 900, 30.0)
    }

    /// What flying to the gate came to.
    #[derive(Debug)]
    struct Flown {
        /// The tick the clearance press was made, if it was.
        requested: Option<u32>,
        /// The tick the entering press was made, if it was.
        entered: Option<u32>,
    }

    /// Flies `handling` from `from`, drifting at `drift`, to [`site`] by
    /// the autopilot's controls through the player's own step, `passes`
    /// steps a decision (two for double time), for at most `cap` ticks:
    /// each press goes through the land key's dock request, which must take
    /// it without a refusal, and the approach is run by hand after each
    /// step.
    fn fly(
        handling: Handling,
        inertial: bool,
        turn: PlayerTurn,
        from: (f32, f32),
        drift: (f32, f32),
        passes: u32,
        cap: u32,
    ) -> Flown {
        let site = site();
        let rules = FlightRules {
            player_turn: turn,
            ..FlightRules::from_rulebook(&Rulebook::default())
        };
        let mut state = ShipState {
            position: Vec2::new(from.0, from.1),
            velocity: Vec2::new(drift.0, drift.1),
            heading: 135.0,
        };
        let mut speed = (!inertial).then_some(0.0_f32);
        let mut dock = None;
        let mut flown = Flown {
            requested: None,
            entered: None,
        };
        let mut tick = 0;
        while tick < cap {
            let craft = Craft {
                state,
                inertialess: speed,
                handling,
                turn,
                gain: SpeedMult::FULL,
            };
            let steered = steer(&craft, &gate(&site, dock));
            if steered.press_land {
                let press = land_or_select(
                    Some(&site),
                    dock,
                    &state,
                    0,
                    std::slice::from_ref(&site),
                    |_| true,
                    |_| true,
                    reach,
                    RuleSource::Engine,
                    RuleSource::Engine,
                );
                match press.outcome {
                    Ok(LandOutcome::Requested { .. }) => {
                        assert!(flown.requested.is_none(), "requested twice");
                        flown.requested = Some(tick);
                    }
                    Ok(LandOutcome::Landed(stellar)) => {
                        assert_eq!(stellar, GATE);
                        flown.entered = Some(tick);
                        return flown;
                    }
                    Err(refusal) => panic!("refused at tick {tick}: {refusal:?}"),
                }
                dock = press.dock;
            }
            for _ in 0..passes {
                let caps = Caps::top(handling.max_speed);
                match speed.as_mut() {
                    Some(speed) => inertialess_player_step(
                        &mut state,
                        speed,
                        &handling,
                        steered.controls,
                        None,
                        caps,
                        rules,
                        SpeedMult::FULL,
                    ),
                    None => player_step(
                        &mut state,
                        &handling,
                        steered.controls,
                        None,
                        caps,
                        rules,
                        SpeedMult::FULL,
                    ),
                }
                tick += 1;
                if let Some(request) = dock.as_mut() {
                    let _ =
                        clearance::approach(request, &site, site.position - state.position, true);
                }
            }
        }
        flown
    }

    fn arrives(handling: Handling, inertial: bool, from: (f32, f32), drift: (f32, f32), cap: u32) {
        for turn in [PlayerTurn::WholeDegrees, PlayerTurn::Full] {
            for passes in [1, 2] {
                let flown = fly(handling, inertial, turn, from, drift, passes, cap);
                assert!(
                    flown.requested.is_some() && flown.entered.is_some(),
                    "{handling:?} inertial {inertial} {turn:?} passes {passes} from {from:?} \
                     drift {drift:?}: {flown:?}"
                );
            }
        }
    }

    #[test]
    fn an_inertial_hull_flies_from_1500_out_at_rest_to_an_entering_press() {
        for handling in [shuttle(), fast(), sluggish()] {
            for from in [(0.0, 1500.0), (1500.0, 0.0), (-1060.0, -1060.0)] {
                arrives(handling, true, from, (0.0, 0.0), 2_000);
            }
        }
    }

    #[test]
    fn an_inertial_hull_flies_from_1500_out_with_drift_to_an_entering_press() {
        for handling in [shuttle(), fast(), sluggish()] {
            let top = handling.max_speed;
            for drift in [(top, 0.0), (0.0, top), (-top * 0.7, -top * 0.7)] {
                arrives(handling, true, (0.0, 1500.0), drift, 2_000);
            }
        }
    }

    #[test]
    fn an_inertialess_hull_flies_from_1500_out_to_an_entering_press() {
        for handling in [shuttle(), fast(), sluggish()] {
            for from in [(0.0, 1500.0), (1500.0, 0.0), (-1060.0, -1060.0)] {
                arrives(handling, false, from, (0.0, 0.0), 2_000);
            }
            let top = handling.max_speed;
            arrives(handling, false, (0.0, 1500.0), (top, 0.0), 2_000);
        }
    }
}
