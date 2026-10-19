//! What an ion charge does to a ship's handling, as the `EV Nova`
//! executable (i386) works it out: the ionization factor, the slowdown it
//! makes of the turn and the thrust, the charge shed each frame, and the
//! pull back toward a lowered top speed.
//!
//! - **The factor** ([`factor`], `_ShipIonizationFactor` @0x3225-0x32f6):
//!   the charge over the capacity, none at a capacity of none or less
//!   (@0x32d3-0x32e3). The capacity is the class's `IonizeMax`, and for
//!   the player its Ion Absorbers' too (`nova_sim::stats::IonStats`).
//! - **The slowdown** ([`slowdown`]): the factor, but at most
//!   [`MAX_SLOWDOWN`] (the double 0.7 @0xdd0d0 compared, the float 0.7
//!   @0xdd070 written).
//! - **The turn and the thrust** ([`ionized`]): while the charge is above
//!   none, `_ShipTurnRate` and `_ShipAccelRate` end by multiplying the
//!   rate by 1 − the slowdown, in doubles, and flooring it at none
//!   (@0x3462-0x34cd, @0x3683-0x36db). The turn is spared while the ship's
//!   jump is under way (+0x50 above 0, `ucomiss`/`jb` @0x3478); the thrust
//!   is not.
//! - **The frame's block** ([`deionize`], [`pull`]), `_HandleShip`'s
//!   (@0x342b2-0x34421) and `_HandlePlayer`'s (@0x6d7fd-0x6d96d), after
//!   the move: a charge above none falls by the deionization rate ×
//!   `_gSpeedMult` (1 a tick here), with **no floor** that frame; then,
//!   on the charge left, each axis of the velocity past the top speed ×
//!   (1 − the slowdown) moves back [`ION_PULL`] (@0xdd608, @0xdd610). A
//!   charge at or below none is cleared to none (@0x34418, @0x6d966) and
//!   pulls nothing.
//!
//! A charge left below none for a frame gives a factor below none, and a
//! cap above the top speed; the turn, the thrust, the fire lockout and the
//! tint all ask for a charge or factor above none, so none of them reads
//! it. None of this plays a sound.

use crate::geometry::Vec2;
use crate::handling::Handling;

/// The most the ionization factor slows a ship's turn, thrust and top
/// speed: 0.7 (the double @0xdd0d0 the factor is compared with, the float
/// @0xdd070 written in its place).
pub const MAX_SLOWDOWN: f32 = 0.7;

/// How far a frame's pull moves an axis of the velocity back toward the
/// lowered top speed, in pixels a tick (the doubles -0.025 @0xdd608 and
/// 0.025 @0xdd610, × `_gSpeedMult`).
pub const ION_PULL: f64 = 0.025;

/// A ship's ionization factor, `_ShipIonizationFactor`: its `charge` over
/// its `capacity`, none when the capacity is none or less.
#[must_use]
pub fn factor(charge: f32, capacity: f32) -> f32 {
    if capacity > 0.0 {
        charge / capacity
    } else {
        0.0
    }
}

/// How much a `factor` slows a ship: the factor, but [`MAX_SLOWDOWN`]
/// once it is above 0.7 as a double. A factor of 0.7 as a float (just
/// under the double) is kept.
#[must_use]
pub fn slowdown(factor: f32) -> f32 {
    if f64::from(factor) > 0.7 {
        MAX_SLOWDOWN
    } else {
        factor
    }
}

/// `rate` slowed by `slowdown`: rate × (1 − slowdown) in doubles, as a
/// float, none below none.
fn slowed(rate: f32, slowdown: f32) -> f32 {
    let rate = (f64::from(rate) * (1.0 - f64::from(slowdown))) as f32;
    rate.max(0.0)
}

/// `handling` as a ship of ion `charge` and ionization `factor` turns and
/// thrusts (see the module docs), its jump under way when `jumping`: while
/// the charge is above none, its turn (unless jumping) and its thrust ×
/// (1 − [`slowdown`]), none below none. Its top speed is left as it is: the
/// frame's [`pull`] holds the ship to the lowered one.
#[must_use]
pub fn ionized(handling: Handling, charge: f32, factor: f32, jumping: bool) -> Handling {
    if charge <= 0.0 {
        return handling;
    }
    let slowdown = slowdown(factor);
    Handling {
        turn_rate: if jumping {
            handling.turn_rate
        } else {
            slowed(handling.turn_rate, slowdown)
        },
        accel: slowed(handling.accel, slowdown),
        ..handling
    }
}

/// What a ship's handling reads of its ionization: its charge, its
/// factor and whether its jump is under way. None by default.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ionization {
    /// Its ion charge.
    pub charge: f32,
    /// Its ionization factor ([`factor`]).
    pub factor: f32,
    /// Whether its jump is under way (+0x50 above 0), sparing its turn.
    pub jumping: bool,
}

impl Ionization {
    /// `handling` slowed by this ionization ([`ionized`]).
    #[must_use]
    pub fn slows(self, handling: Handling) -> Handling {
        ionized(handling, self.charge, self.factor, self.jumping)
    }
}

/// The ion `charge` left after a frame shedding `rate` (see the module
/// docs): a charge above none falls by the rate, in doubles, unfloored,
/// and the frame's [`pull`] follows; one at or below none is cleared to
/// none, and nothing is pulled (`None`).
#[must_use]
pub fn deionize(charge: f32, rate: f32) -> Option<f32> {
    (charge > 0.0).then(|| (f64::from(charge) - f64::from(rate)) as f32)
}

/// The top speed a ship of top speed `top_speed` and ionization `factor`
/// is pulled back toward: `top_speed` × (1 − [`slowdown`]), in floats
/// (@0x34339-0x34360). A factor below none gives a cap above the top
/// speed.
#[must_use]
pub fn lowered_top(top_speed: f32, factor: f32) -> f32 {
    top_speed * (1.0 - slowdown(factor))
}

/// `velocity` after a frame's pull toward `cap` on each axis (see the
/// module docs): an axis above the cap moves [`ION_PULL`] down, and then,
/// as it now is, one below −cap moves [`ION_PULL`] up. An axis within it
/// is left alone. A pull, not a clamp: a ship far past the cap takes many
/// frames to come back.
#[must_use]
pub fn pull(velocity: Vec2, cap: f32) -> Vec2 {
    let axis = |mut v: f32| {
        if v > cap {
            v = (f64::from(v) - ION_PULL) as f32;
        }
        if v < -cap {
            v = (f64::from(v) + ION_PULL) as f32;
        }
        v
    };
    Vec2::new(axis(velocity.x), axis(velocity.y))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const HANDLING: Handling = Handling {
        max_speed: 5.0,
        accel: 0.2,
        turn_rate: 3.0,
    };

    #[test]
    fn the_factor_is_the_charge_over_the_capacity_and_none_at_no_capacity() {
        assert_eq!(factor(100.0, 400.0), 0.25);
        assert_eq!(factor(800.0, 400.0), 2.0);
        assert_eq!(factor(-10.0, 400.0), -0.025);
        assert_eq!(factor(100.0, 0.0), 0.0);
        assert_eq!(factor(100.0, -50.0), 0.0);
        assert_eq!(factor(0.0, 400.0), 0.0);
    }

    #[test]
    fn the_slowdown_is_the_factor_capped_at_0_7() {
        assert_eq!(slowdown(0.25), 0.25);
        assert_eq!(slowdown(0.7), 0.7, "0.7 as a float is under the double");
        assert_eq!(slowdown(0.700_001), MAX_SLOWDOWN);
        assert_eq!(slowdown(3.0), 0.7);
        assert_eq!(slowdown(-0.5), -0.5);
        assert_eq!(MAX_SLOWDOWN, 0.7);
    }

    #[test]
    fn an_ionized_ship_turns_and_thrusts_by_1_less_the_slowdown() {
        let ionized = ionized(HANDLING, 100.0, 0.5, false);
        assert_eq!(ionized.turn_rate, 1.5);
        assert_eq!(ionized.accel, 0.1);
        assert_eq!(ionized.max_speed, 5.0, "the pull holds the top speed");
        let capped = super::ionized(HANDLING, 900.0, 3.0, false);
        assert_eq!(
            capped.turn_rate,
            (3.0_f64 * (1.0 - f64::from(MAX_SLOWDOWN))) as f32
        );
        assert_eq!(
            capped.accel,
            (f64::from(0.2_f32) * (1.0 - f64::from(MAX_SLOWDOWN))) as f32
        );
    }

    #[test]
    fn the_rates_are_worked_in_doubles() {
        let ionized = ionized(HANDLING, 1.0, 0.3, false);
        assert_eq!(
            ionized.accel,
            (f64::from(0.2_f32) * (1.0 - f64::from(0.3_f32))) as f32
        );
    }

    #[test]
    fn a_rate_is_floored_at_none() {
        let backwards = Handling {
            turn_rate: -2.0,
            accel: -1.0,
            ..HANDLING
        };
        let ionized = ionized(backwards, 10.0, 0.1, false);
        assert_eq!((ionized.turn_rate, ionized.accel), (0.0, 0.0));
    }

    #[test]
    fn a_jumping_ship_turns_at_its_full_rate_but_thrusts_slower() {
        let ionized = ionized(HANDLING, 100.0, 0.5, true);
        assert_eq!(ionized.turn_rate, 3.0);
        assert_eq!(ionized.accel, 0.1);
    }

    #[test]
    fn no_charge_slows_nothing() {
        assert_eq!(ionized(HANDLING, 0.0, 0.5, false), HANDLING);
        assert_eq!(ionized(HANDLING, -1.0, 0.5, false), HANDLING);
        // Nor does a negative factor's speed-up on no charge.
        assert_eq!(ionized(HANDLING, -1.0, -0.5, false), HANDLING);
    }

    #[test]
    fn an_ionization_slows_as_ionized_does() {
        let ion = Ionization {
            charge: 100.0,
            factor: 0.5,
            jumping: true,
        };
        assert_eq!(ion.slows(HANDLING), ionized(HANDLING, 100.0, 0.5, true));
        assert_eq!(Ionization::default().slows(HANDLING), HANDLING);
    }

    #[test]
    fn a_charge_sheds_its_rate_unfloored_and_none_clears_to_none() {
        assert_eq!(deionize(10.0, 1.0), Some(9.0));
        assert_eq!(deionize(0.5, 1.0), Some(-0.5), "no floor this frame");
        assert_eq!(deionize(0.0, 1.0), None);
        assert_eq!(deionize(-0.5, 1.0), None);
        let shed = deionize(1.0, 0.1).expect("charged");
        assert_eq!(shed, (1.0_f64 - f64::from(0.1_f32)) as f32, "in doubles");
    }

    #[test]
    fn the_lowered_top_is_the_top_speed_by_1_less_the_slowdown() {
        assert_eq!(lowered_top(4.0, 0.25), 3.0);
        assert_eq!(lowered_top(4.0, 2.0), 4.0 * (1.0 - 0.7));
        assert_eq!(lowered_top(4.0, -0.25), 5.0, "a negative factor's cap");
    }

    #[test]
    fn an_axis_past_the_cap_is_pulled_back_0_025() {
        let pulled = pull(Vec2::new(3.0, -1.0), 2.0);
        assert_eq!(pulled, Vec2::new((3.0_f64 - 0.025) as f32, -1.0));
        let pulled = pull(Vec2::new(1.0, -3.0), 2.0);
        assert_eq!(pulled, Vec2::new(1.0, (-3.0_f64 + 0.025) as f32));
        assert_eq!(ION_PULL, 0.025);
    }

    #[test]
    fn an_axis_within_the_cap_is_left_alone() {
        let within = Vec2::new(2.0, -2.0);
        assert_eq!(pull(within, 2.0), within);
        assert_eq!(pull(Vec2::new(0.5, 0.0), 2.0), Vec2::new(0.5, 0.0));
    }

    #[test]
    fn a_cap_of_none_pulls_both_signs_toward_rest() {
        let pulled = pull(Vec2::new(1.0, -1.0), 0.0);
        assert_eq!(
            pulled,
            Vec2::new((1.0_f64 - 0.025) as f32, (-1.0_f64 + 0.025) as f32)
        );
    }

    #[test]
    fn the_up_pull_reads_the_axis_as_the_down_pull_left_it() {
        // At a cap of none, a small speed pulled down past rest is below
        // −cap, so the up pull follows on the axis as updated.
        let pulled = pull(Vec2::new(0.01, 0.0), 0.0);
        let down = (f64::from(0.01_f32) - 0.025) as f32;
        let up = (f64::from(down) + 0.025) as f32;
        assert_eq!(pulled.x, up);
    }

    #[test]
    fn a_negative_factors_cap_above_the_top_speed_pulls_nothing_within_it() {
        let cap = lowered_top(4.0, -0.25);
        let fast = Vec2::new(4.5, -4.5);
        assert_eq!(pull(fast, cap), fast);
    }
}
