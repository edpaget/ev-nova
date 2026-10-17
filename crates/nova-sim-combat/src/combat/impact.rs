//! Knockback and tractor beams: how a hit, a blast, a beam tick or a
//! shockwave pushes the ship it lands on by its weapon's `Impact`, and how
//! a beam of negative `Impact` holds its target or pulls its firer.
//!
//! **The impact block** ([`push`], `_DamageShip` @0x3a9b8-0x3ab84). It is
//! reached only past `_DamageShip`'s early-out (@0x3a86b-0x3a88f), so a
//! ship of the escape pod's class or later
//! ([`immune`](super::hull::immune)) and a destroyed ship are never
//! pushed; nothing tests `_IsDying` or `_IsDisabled` first, so a disabled
//! or breaking-up ship is.
//! - A negative `Impact` on a direct hit (arg +0x20 set) from a ship in
//!   the table (arg +0x1c in 0-0x3f) pushes nothing while that ship's
//!   centre is within [`NEAR`] of the ship hit on **either** axis
//!   (@0x3a9b8-0x3aa68).
//! - An `Impact` of none pushes nothing and clamps nothing (@0x3aa6e); nor
//!   does any on a ship whose jump is under way (+0x50 above none,
//!   @0x3aa7d), or on a massless (`Mass` at or below none) or planet-type
//!   (`Flags` 0x0400) class (@0x3aaaa-0x3aad2).
//! - Otherwise the ship gains `Impact` / `Mass` along the whole-degree
//!   bearing from the point to it (`_CalcInterceptAngle` @0x3aa9e,
//!   [`intercept_angle`]) through `_AdjustedAccel` (@0x3ab14,
//!   [`launch_velocity`]) against its class's top speed (class +0x40), and
//!   `_SpeedLimit` ([`speed_limit`], @0x2af91) clamps each axis to its top
//!   speed (`_ShipMaxSpeed`, @0x3ab6a-0x3ab7f). A negative `Impact` pulls
//!   it toward the point.
//!
//! The player's top speed is 1.8 times its own there while its
//! afterburner is on and no gravity source is present (@0x3ab19-0x3ab68);
//! that is left to `ship-handling-fidelity/phase-6-afterburner`, and the
//! plain top speed is used until then.
//!
//! **The point and the `Impact`** each caller passes: the weapon's raw
//! `Impact` ([`WeaponSpec::impact`](super::weapon::WeaponSpec::impact)),
//! never decayed, reduced by blast falloff or doubled in double time
//! (only the damages are, @0x31191). A shot's direct hit pushes from where
//! it met the ship (`_HandleShipHit` @0x36c7a); a blast, on a hit
//! (@0x36ec0) or as its shot expires (`_HandleShot` @0x36106), from its
//! centre, never the ship hit directly, which it skips (@0x36da4); a beam,
//! on every tick it hits, from its **start** (@0x316a1, its firer's exit),
//! not where it hit. A missile lock (@0x35855) has an `Impact` of none;
//! the death shockwave's is [`IMPACT`](super::shockwave::IMPACT) from the
//! wreck (@0x2da89).
//!
//! **A tractor beam** ([`tractor`], `_HandleBeams` @0x311d0-0x31450) is a
//! beam of negative `Impact` with a firer, which the branch reads before
//! the hit's `_DamageShip`:
//! - On a massless or planet-type target (@0x311ec-0x31217) it does
//!   nothing, and its `Impact` is spent ([`Tractor::Slips`]).
//! - A firer of a `Mass` at least [`HOLD_SHARE`] of the target's, in
//!   doubles (@0x3121d-0x31257), holds the target: the target's lock (its
//!   +0xc8dc) names the firer and is touched (+0xb4 = `TickCount`), and
//!   the `Impact` passes on, so the hit's push pulls the target toward the
//!   beam's start, past the either-axis gate from the firer's centre
//!   ([`Tractor::Holds`]).
//! - A lighter firer holds itself, unless something holds it already
//!   (@0x3127d-0x31287), and its lock is touched either way (@0x312cf)
//!   ([`TractorLock::touch_self`]). It pulls itself toward the target by
//!   `Impact` / its own `Mass` (`_AdjustedAccel` toward the bearing from
//!   the target to the beam's start @0x313a9-0x31410, against its class's
//!   top speed), clamped to its top speed (`_SpeedLimit` @0x3144b), unless
//!   the beam's start is within [`NEAR`] of the target on **both** axes
//!   (@0x312dd-0x31341) or it is itself massless or planet-type
//!   (@0x31347-0x3138c). There is no jump test and no 1.8. Its `Impact` is
//!   spent, so the target is not pushed ([`Tractor::Pulls`]).
//!
//! What a lock does to the ship it holds (the pull to its holder's
//! velocity, the lapse half a second after its last touch, the third of
//! its speed, turn and thrust, the refused jump) is
//! `ship-handling-fidelity/phase-12-tractor-hold`'s.
//!
//! Neither block makes a sound.

use super::ShipRef;
use crate::bay::launch_velocity;
use crate::geometry::Vec2;
pub use crate::speed_cap::speed_limit;
use crate::trig::intercept_angle;

/// How near, in pixels on an axis, the gates of the impact block and the
/// tractor's self-pull reach (50.0 @0xdd064).
pub const NEAR: f32 = 50.0;

/// The share of the target's `Mass` a tractor's firer must reach to hold
/// it (0.75 @0xdd1f8).
pub const HOLD_SHARE: f64 = 0.75;

/// A ship as a push reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Body {
    /// Where it is.
    pub position: Vec2,
    /// How it moves.
    pub velocity: Vec2,
    /// Its class's `Mass`.
    pub mass: f32,
    /// Whether its class is planet-type (`Flags` 0x0400).
    pub planet_type: bool,
    /// Its class's top speed, its `Speed` alone (class +0x40), which a
    /// push is soft-capped against.
    pub class_speed: f32,
    /// Its top speed (`_ShipMaxSpeed`), which a push is clamped to on each
    /// axis.
    pub top_speed: f32,
    /// Whether its jump is under way (+0x50 above none).
    pub jumping: bool,
}

/// A ship's tractor lock: the ship holding it in a tractor beam, which
/// may be itself, and how long since a beam last touched it (the
/// original's +0xc8dc and +0xb4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TractorLock {
    /// The ship holding it.
    pub holder: ShipRef,
    /// The engine ticks since a tractor beam last touched it. The fight
    /// sets it to none on each touch, as the original sets +0xb4 to the
    /// `TickCount`; counting it up, and dropping the lock as the original
    /// does once `TickCount` reaches +0xb4 + 30 (half a second, 15 ticks),
    /// are `ship-handling-fidelity/phase-12-tractor-hold`'s.
    pub ticks_since_touch: u32,
}

impl TractorLock {
    /// Held by `holder`, touched now.
    #[must_use]
    pub fn by(holder: ShipRef) -> Self {
        Self {
            holder,
            ticks_since_touch: 0,
        }
    }

    /// Touches `lock`, the lock of `me`, the firer of a tractor too light
    /// to hold its target: held by `me` when nothing held it, its holder
    /// kept otherwise.
    pub fn touch_self(lock: &mut Option<Self>, me: ShipRef) {
        let holder = lock.map_or(me, |lock| lock.holder);
        *lock = Some(Self::by(holder));
    }
}

/// What a tractor beam's touch does (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tractor {
    /// Nothing, on a massless or planet-type target: its `Impact` is spent.
    Slips,
    /// The firer holds the target, and the `Impact` passes on to the hit.
    Holds,
    /// The firer holds itself, or keeps its holder, and pulls itself to
    /// this velocity, if at all; its `Impact` is spent.
    Pulls(Option<Vec2>),
}

/// The velocity `impact` from `from` leaves `body` with, as `_DamageShip`'s
/// impact block gives it (see the module docs); none when it is not
/// pushed. `source` is the centre of the ship whose direct hit it is,
/// which a negative `impact` is gated by; none for any other hit, or when
/// that ship is no longer in the fight.
#[must_use]
pub fn push(body: &Body, from: Vec2, impact: f32, source: Option<Vec2>) -> Option<Vec2> {
    let gated = impact < 0.0 && source.is_some_and(|source| near_either(source, body.position));
    if gated || impact == 0.0 || body.jumping || !pushable(body) {
        return None;
    }
    let bearing = f32::from(intercept_angle(from, body.position));
    Some(pulled(body, bearing, impact))
}

/// What a tractor beam of `impact` (below none) from `start`, fired by
/// `firer`, does as it touches `target` (see the module docs).
#[must_use]
pub fn tractor(target: &Body, firer: &Body, start: Vec2, impact: f32) -> Tractor {
    if !pushable(target) {
        return Tractor::Slips;
    }
    if f64::from(firer.mass) >= HOLD_SHARE * f64::from(target.mass) {
        return Tractor::Holds;
    }
    let off = start - target.position;
    let close = off.x.abs() < NEAR && off.y.abs() < NEAR;
    if close || !pushable(firer) {
        return Tractor::Pulls(None);
    }
    let bearing = f32::from(intercept_angle(target.position, start));
    Tractor::Pulls(Some(pulled(firer, bearing, impact)))
}

/// Whether a push can move `body`: a class of some `Mass`, not
/// planet-type.
fn pushable(body: &Body) -> bool {
    body.mass > 0.0 && !body.planet_type
}

/// Whether `a` and `b` are within [`NEAR`] of each other on either axis.
fn near_either(a: Vec2, b: Vec2) -> bool {
    (a.x - b.x).abs() < NEAR || (a.y - b.y).abs() < NEAR
}

/// `body`'s velocity pushed `impact` / its mass along `bearing`, soft-capped
/// at its class speed and clamped to its top speed.
fn pulled(body: &Body, bearing: f32, impact: f32) -> Vec2 {
    let pushed = launch_velocity(body.velocity, bearing, impact / body.mass, body.class_speed);
    speed_limit(pushed, body.top_speed)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::combat::NpcId;
    use crate::flight::facing;

    const X: ShipRef = ShipRef::Npc(NpcId(7));

    /// A ship of `mass` at (`x`, `y`), at rest, with a class and top speed
    /// of 100.
    fn body(mass: f32, x: f32, y: f32) -> Body {
        Body {
            position: Vec2::new(x, y),
            mass,
            class_speed: 100.0,
            top_speed: 100.0,
            ..Body::default()
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn a_ship_at_rest_gains_impact_over_mass_along_the_whole_degree_bearing() {
        let ship = body(10.0, 100.0, 100.0);
        let pushed = push(&ship, Vec2::ZERO, 50.0, None).expect("pushed");
        // The table makes the diagonal 134, a degree short of 135.
        assert!(near(pushed, facing(134.0) * 5.0), "{pushed:?}");
        assert!(!near(pushed, facing(135.0) * 5.0));
        let right = push(&body(200.0, 40.0, 0.0), Vec2::ZERO, 750.0, None);
        assert!(near(right.expect("pushed"), Vec2::new(3.75, 0.0)));
    }

    #[test]
    fn a_negative_impact_pulls_the_ship_toward_the_point() {
        let pulled = push(&body(10.0, 40.0, 0.0), Vec2::ZERO, -50.0, None);
        assert!(near(pulled.expect("pulled"), Vec2::new(-5.0, 0.0)));
    }

    #[test]
    fn the_push_is_soft_capped_per_axis_at_the_class_speed() {
        let ship = Body {
            velocity: Vec2::new(4.0, 0.0),
            class_speed: 5.0,
            ..body(10.0, 100.0, 100.0)
        };
        // The share of the class speed on x is 5 x sin 134 = 3.6: x is
        // past it and gains nothing; y is short of its share and gains.
        let pushed = push(&ship, Vec2::ZERO, 50.0, None).expect("pushed");
        assert_eq!(pushed.x, 4.0);
        assert!(
            (pushed.y - facing(134.0).y * 5.0).abs() < 1e-4,
            "{pushed:?}"
        );
    }

    #[test]
    fn the_push_is_clamped_on_each_axis_to_the_top_speed() {
        let ship = Body {
            top_speed: 3.0,
            ..body(1.0, 100.0, 100.0)
        };
        let pushed = push(&ship, Vec2::ZERO, 10.0, None).expect("pushed");
        assert_eq!(pushed, Vec2::new(3.0, 3.0), "longer than 3 on the diagonal");
        let back = Body {
            velocity: Vec2::new(-20.0, 9.0),
            class_speed: 20.0,
            top_speed: 8.0,
            ..body(100.0, 40.0, 0.0)
        };
        let clamped = push(&back, Vec2::ZERO, 750.0, None).expect("pushed");
        assert!(near(clamped, Vec2::new(-8.0, 8.0)), "{clamped:?}");
    }

    #[test]
    fn an_impact_of_none_neither_pushes_nor_clamps() {
        let fast = Body {
            velocity: Vec2::new(20.0, 0.0),
            top_speed: 10.0,
            ..body(10.0, 40.0, 0.0)
        };
        assert_eq!(push(&fast, Vec2::ZERO, 0.0, None), None);
    }

    #[test]
    fn a_massless_planet_type_or_jumping_ship_is_not_pushed() {
        let ship = body(10.0, 40.0, 0.0);
        assert!(push(&ship, Vec2::ZERO, 50.0, None).is_some());
        let massless = Body { mass: 0.0, ..ship };
        assert_eq!(push(&massless, Vec2::ZERO, 50.0, None), None);
        let negative = Body { mass: -3.0, ..ship };
        assert_eq!(push(&negative, Vec2::ZERO, 50.0, None), None);
        let planet = Body {
            planet_type: true,
            ..ship
        };
        assert_eq!(push(&planet, Vec2::ZERO, 50.0, None), None);
        let jumping = Body {
            jumping: true,
            ..ship
        };
        assert_eq!(push(&jumping, Vec2::ZERO, 50.0, None), None);
    }

    #[test]
    fn a_negative_direct_hit_pushes_nothing_from_a_source_near_on_either_axis() {
        let ship = body(10.0, 300.0, 300.0);
        let point = Vec2::ZERO;
        let source = |dx: f32, dy: f32| Some(ship.position + Vec2::new(dx, dy));
        assert_eq!(push(&ship, point, -50.0, source(30.0, 200.0)), None);
        assert_eq!(push(&ship, point, -50.0, source(200.0, -30.0)), None);
        assert_eq!(push(&ship, point, -50.0, source(-49.9, 200.0)), None);
        let pulled = push(&ship, point, -50.0, source(200.0, 200.0)).expect("pulled");
        assert!(
            near(pulled, facing(134.0) * -5.0),
            "toward the point: {pulled:?}"
        );
        assert!(push(&ship, point, -50.0, source(50.0, -50.0)).is_some());
        assert!(push(&ship, point, -50.0, None).is_some(), "no source");
        assert!(
            push(&ship, point, 50.0, source(0.0, 0.0)).is_some(),
            "a positive impact is never gated"
        );
    }

    #[test]
    fn a_tractor_slips_off_a_massless_or_planet_type_target() {
        let firer = body(200.0, 0.0, 0.0);
        let target = body(100.0, 300.0, 300.0);
        assert_ne!(tractor(&target, &firer, Vec2::ZERO, -5.0), Tractor::Slips);
        let massless = Body {
            mass: 0.0,
            ..target
        };
        assert_eq!(tractor(&massless, &firer, Vec2::ZERO, -5.0), Tractor::Slips);
        let planet = Body {
            planet_type: true,
            ..target
        };
        assert_eq!(tractor(&planet, &firer, Vec2::ZERO, -5.0), Tractor::Slips);
    }

    #[test]
    fn a_firer_of_three_quarters_the_targets_mass_holds_it() {
        let target = body(100.0, 300.0, 300.0);
        let heavy = body(75.0, 0.0, 0.0);
        assert_eq!(tractor(&target, &heavy, Vec2::ZERO, -5.0), Tractor::Holds);
        let light = body(74.0, 0.0, 0.0);
        assert!(matches!(
            tractor(&target, &light, Vec2::ZERO, -5.0),
            Tractor::Pulls(_)
        ));
    }

    #[test]
    fn a_lighter_firer_pulls_itself_toward_the_target() {
        let target = body(15.0, 300.0, 0.0);
        let firer = Body {
            jumping: true,
            ..body(6.0, -10.0, 0.0)
        };
        let Tractor::Pulls(Some(pulled)) = tractor(&target, &firer, Vec2::ZERO, -5.0) else {
            panic!("pulled");
        };
        assert!(near(pulled, Vec2::new(5.0 / 6.0, 0.0)), "{pulled:?}");
        // The bearing runs from the target to the start: (100, 100) off
        // the start, it is 314, and the pull runs back along it.
        let diagonal = body(15.0, 100.0, 100.0);
        let Tractor::Pulls(Some(pulled)) = tractor(&diagonal, &firer, Vec2::ZERO, -6.0) else {
            panic!("pulled");
        };
        assert!(near(pulled, facing(314.0) * -1.0), "{pulled:?}");
    }

    #[test]
    fn a_self_pull_gains_past_the_class_speed_and_is_clamped_to_the_top_speed() {
        let target = body(15.0, 300.0, 0.0);
        let capped = Body {
            velocity: Vec2::new(2.0, 0.0),
            class_speed: 2.0,
            ..body(6.0, -10.0, 0.0)
        };
        let Tractor::Pulls(Some(past)) = tractor(&target, &capped, Vec2::ZERO, -5.0) else {
            panic!("pulled");
        };
        assert!(
            near(past, Vec2::new(2.0 + 5.0 / 6.0, 0.0)),
            "a negative push gains past the class speed: {past:?}"
        );
        let slow = Body {
            top_speed: 0.5,
            ..body(1.0, -10.0, 0.0)
        };
        let Tractor::Pulls(Some(clamped)) = tractor(&target, &slow, Vec2::ZERO, -5.0) else {
            panic!("pulled");
        };
        assert!(near(clamped, Vec2::new(0.5, 0.0)), "{clamped:?}");
    }

    #[test]
    fn a_firer_is_not_pulled_while_the_start_is_near_the_target_on_both_axes() {
        let firer = body(6.0, -10.0, 0.0);
        let at = |x: f32, y: f32| body(15.0, x, y);
        let start = Vec2::ZERO;
        assert_eq!(
            tractor(&at(30.0, -49.9), &firer, start, -5.0),
            Tractor::Pulls(None)
        );
        assert!(matches!(
            tractor(&at(30.0, 200.0), &firer, start, -5.0),
            Tractor::Pulls(Some(_))
        ));
        assert!(matches!(
            tractor(&at(-200.0, 30.0), &firer, start, -5.0),
            Tractor::Pulls(Some(_))
        ));
        assert!(matches!(
            tractor(&at(50.0, 0.0), &firer, start, -5.0),
            Tractor::Pulls(Some(_))
        ));
    }

    #[test]
    fn the_near_gate_is_measured_from_the_start_wherever_it_is_and_is_strict_on_each_axis() {
        let firer = body(6.0, 990.0, 1000.0);
        let start = Vec2::new(1000.0, 1000.0);
        let at = |x: f32, y: f32| tractor(&body(15.0, x, y), &firer, start, -5.0);
        assert_eq!(at(1030.0, 1049.9), Tractor::Pulls(None));
        assert!(
            matches!(at(1030.0, 1050.0), Tractor::Pulls(Some(_))),
            "50 down"
        );
        assert!(
            matches!(at(1049.9, 950.0), Tractor::Pulls(Some(_))),
            "50 up"
        );
        assert!(
            matches!(at(30.0, 49.9), Tractor::Pulls(Some(_))),
            "near the origin, far from the start"
        );
    }

    #[test]
    fn a_massless_or_planet_type_firer_holds_itself_but_is_not_pulled() {
        let target = body(15.0, 300.0, 0.0);
        let massless = body(0.0, -10.0, 0.0);
        assert_eq!(
            tractor(&target, &massless, Vec2::ZERO, -5.0),
            Tractor::Pulls(None)
        );
        let planet = Body {
            planet_type: true,
            ..body(6.0, -10.0, 0.0)
        };
        assert_eq!(
            tractor(&target, &planet, Vec2::ZERO, -5.0),
            Tractor::Pulls(None)
        );
    }

    #[test]
    fn a_light_tractors_firer_holds_itself_unless_held_and_is_touched_either_way() {
        let mut free = None;
        TractorLock::touch_self(&mut free, ShipRef::Player);
        assert_eq!(free, Some(TractorLock::by(ShipRef::Player)));
        let mut held = Some(TractorLock {
            holder: X,
            ticks_since_touch: 9,
        });
        TractorLock::touch_self(&mut held, ShipRef::Player);
        assert_eq!(
            held,
            Some(TractorLock {
                holder: X,
                ticks_since_touch: 0
            })
        );
        assert_eq!((NEAR, HOLD_SHARE), (50.0, 0.75));
    }
}
