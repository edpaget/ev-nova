//! The player's inertialess flight (see
//! [`inertialess`](crate::inertialess)): a ship whose hull's `Flags2`
//! holds 0x0040, or that carries an `oütf` of `ModType` 38, holds a speed
//! along its heading, the original's ship +0x48, which thrust and reverse
//! move and its velocity follows (`_HandlePlayer` in `EV Nova.app`, i386).
//!
//! **Out of a jump** ([`Session::fly_inertialess`]):
//!
//! 1. Reverse (@0x6c770-0x6c7d3), held by a player not disabled, takes a
//!    tick's acceleration off the speed, down to none. It gives no aim and
//!    writes no +0x68, so the turn keys still turn (see the `steering`
//!    module), and it still counts as a flight key.
//! 2. The turn block turns the ship as any ship's (`player_turn`).
//! 3. Thrust (@0x6cbb2-0x6cc3e) adds a tick's acceleration to the speed,
//!    up to `_ShipMaxSpeed`, and ramps the glow as any ship's
//!    (@0x6cd24-0x6cd52).
//! 4. The speed is capped at the box, `_playerMaxVel` (@0x6d356-0x6d369;
//!    see the `afterburner` module), the top speed but while the
//!    afterburner widens it and as it bleeds back after. A disabled player's
//!    speed then falls by
//!    [`PLAYER_DISABLED_DECAY`](crate::inertialess::PLAYER_DISABLED_DECAY)
//!    and its glow by 1 more (@0x6d36e-0x6d3a3), after its velocity's
//!    [`DISABLED_DRAG`](crate::flight::DISABLED_DRAG) (@0x68ad1); in
//!    flight the glow follows the speed (@0x6d3af-0x6d42d; see the
//!    [`glow`](crate::glow) module).
//! 5. The velocity is pulled toward heading x speed (@0x6d43a), clamped
//!    to the box (@0x6d43f-0x6d4aa) and the ship moves.
//!
//! **In the pre-jump** (see [`pre_jump`](crate::pre_jump)) the brake takes
//! a tick's acceleration off the speed instead of turning against the
//! motion and thrusting, and the glow follows the speed in both stages,
//! as both reach @0x6d343.
//!
//! **The speed's resets.** It starts at none (`_ResetPlayer` @0x1d43c), and
//! landing sets it to none (`_PlayerLandOnStellar` @0x63409). An arrival
//! takes its velocity's length ([`Session::arrive_at_speed`]): a gate's
//! exit sets it to half the top speed along the exit heading
//! (`_PlayerEnterHypergate` @0x63b17, `_PlayerEnterWormhole` @0x643cd),
//! the speed the exit's velocity is; a jump's arrival is at rest in the
//! engine, so none. The original's arrival sets +0x48 to the top speed
//! (@0x6bce7-0x6bcf5), placing the ship 1350 out as it does every ship
//! (@0x6bc8c-0x6bccd): that whole block is
//! rdm:phase/ship-handling-fidelity/phase-18-arrival-at-top-speed's,
//! since it reverses the engine's at-rest arrival. Nothing else writes the
//! player's +0x48: not taking off (the ship is landed, at none), changing
//! ship or capturing one.
//!
//! None of these blocks plays a sound.

use super::Session;
use crate::flight::{Controls, Flyer, drag, inertialess_player_step};
use crate::glow::{Follow, ramp_inertialess_glow};
use crate::inertialess::{PLAYER_DISABLED_DECAY, is_inertialess};

impl Session {
    /// Whether the player's ship flies inertialess
    /// ([`is_inertialess`]), by its stats and the rules: the player is
    /// never in an NPC's close formation.
    pub(crate) fn player_inertialess(&self) -> bool {
        is_inertialess(
            self.stats.inertialess_hull,
            self.stats.dampener,
            Flyer::Player,
            false,
            self.flight_rules().inertial_dampener,
        )
    }

    /// The player's speed along its heading, the original's +0x48: what
    /// an inertialess ship's velocity follows (see the module docs). An
    /// inertial ship's is left as it was last set.
    #[must_use]
    pub fn player_speed(&self) -> f32 {
        self.player_speed
    }

    /// One tick of the player's inertialess ship out of a jump under
    /// `controls`, turning toward `aim` when there is one (see the module
    /// docs), its glow ramped and the thrust sounded as any ship's.
    pub(super) fn fly_inertialess(&mut self, controls: Controls, aim: Option<i16>) {
        let handling = self.flying_handling();
        let caps = self.player_caps();
        let disabled = self.player_fate().row().drag;
        if disabled {
            drag(&mut self.player);
            self.player_speed = self.player_speed.min(caps.max_vel) * PLAYER_DISABLED_DECAY;
        }
        let flight = self.flight_rules();
        inertialess_player_step(
            &mut self.player,
            &mut self.player_speed,
            &handling,
            controls,
            aim,
            caps,
            flight,
        );
        let follow = if disabled {
            Follow::Disabled
        } else {
            Follow::Speed {
                speed: self.player_speed,
                max_speed: handling.max_speed,
            }
        };
        self.ramp_inertialess(controls.thrust, follow);
    }

    /// Sounds the thrust starting or stopping as `thrust` says, and ramps
    /// an inertialess ship's glow, following `follow`, with the
    /// afterburner's step while it is on ([`ramp_inertialess_glow`]).
    pub(super) fn ramp_inertialess(&mut self, thrust: bool, follow: Follow) {
        self.sound_thrust(thrust);
        let burning = self.afterburner_on;
        self.engine_glow = ramp_inertialess_glow(self.engine_glow, thrust, follow, burning);
    }

    /// Ramps the glow after a pre-jump tick in which the ship thrust or
    /// not, following its speed if it is inertialess.
    pub(super) fn ramp_pre_jump(&mut self, thrust: bool) {
        if self.player_inertialess() {
            let follow = Follow::Speed {
                speed: self.player_speed,
                max_speed: self.flying_handling().max_speed,
            };
            self.ramp_inertialess(thrust, follow);
        } else {
            self.thrust(thrust);
        }
    }

    /// Sets the player's speed as it arrives moving at its velocity: its
    /// length (see the module docs).
    pub(super) fn arrive_at_speed(&mut self) {
        self.player_speed = self.player.velocity.length();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, ShipId, SystemId};
    use crate::combat::hull::Condition;
    use crate::flight::{Face, Turn, facing};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::pre_jump::slow_enough;
    use crate::stats::{INERTIAL_DAMPENER, INERTIALESS_HULL};
    use crate::testkit::{FAST, FakePilotCatalog, catalog, outfit};
    use crate::{RuleKey, RuleSource, Rulebook};

    /// The fake catalog with ship 128's hull inertialess.
    fn hull() -> FakePilotCatalog {
        FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: INERTIALESS_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        }
    }

    /// The fake catalog with ship 128 a plain hull carrying an inertial
    /// dampener, of `ModVal` 0, as a default item.
    fn dampened() -> FakePilotCatalog {
        FakePilotCatalog {
            defaults: vec![(ShipId(128), vec![(OutfitId(330), 1)])],
            outfits: vec![outfit(330, &[(INERTIAL_DAMPENER, 0)])],
            ..catalog()
        }
    }

    /// A session of `catalog` at rest at the centre facing `heading`.
    fn flying(catalog: &FakePilotCatalog, heading: f32) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        session.settle_box();
        session.player.position = Vec2::ZERO;
        session.player.velocity = Vec2::ZERO;
        session.player.heading = heading;
        session
    }

    fn held(thrust: bool, turn: Turn, reverse: bool) -> Controls {
        Controls {
            thrust,
            turn,
            reverse,
            face: Face::Off,
            afterburner: false,
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    /// Thrusts east to full speed, turns a quarter left to north and
    /// coasts until the velocity has followed: what the ship is then.
    fn thrust_turn_and_coast(session: &mut Session) -> (Vec2, f32) {
        for _ in 0..60 {
            session.tick(held(true, Turn::None, false));
        }
        let top = session.top_speed();
        assert!((session.player_speed() - top).abs() < 1e-4, "at full speed");
        assert!(near(session.player.velocity, facing(90.0) * top));
        for _ in 0..30 {
            session.tick(held(false, Turn::Left, false));
        }
        assert_eq!(session.player.heading, 0.0);
        for _ in 0..30 {
            session.tick(Controls::default());
        }
        (session.player.velocity, session.player_speed())
    }

    #[test]
    fn an_inertialess_hull_and_a_dampener_each_make_the_player_inertialess() {
        assert!(!flying(&catalog(), 0.0).player_inertialess());
        assert!(flying(&hull(), 0.0).player_inertialess());
        assert!(flying(&dampened(), 0.0).player_inertialess());
        let bible = flying(&dampened(), 0.0).with_rules(
            Rulebook::default().with_override(RuleKey::InertialDampener, RuleSource::Bible),
        );
        assert!(bible.player_inertialess(), "the player's counts either way");
    }

    #[test]
    fn an_inertialess_player_flies_along_its_heading_with_no_drift_hull_or_dampener_alike() {
        let mut by_hull = flying(&hull(), 90.0);
        let mut by_outfit = flying(&dampened(), 90.0);
        let (velocity, speed) = thrust_turn_and_coast(&mut by_hull);
        let top = by_hull.top_speed();
        assert!(near(velocity, facing(0.0) * top), "{velocity:?}");
        assert!((speed - top).abs() < 1e-4);
        let at = by_hull.player.position;
        by_hull.tick(Controls::default());
        assert!(
            near(by_hull.player.position - at, facing(0.0) * top),
            "no drift"
        );
        assert_eq!(thrust_turn_and_coast(&mut by_outfit), (velocity, speed));
        by_outfit.tick(Controls::default());
        assert_eq!(by_outfit.player, by_hull.player, "the same flight");
    }

    #[test]
    fn an_inertial_player_still_drifts_after_turning() {
        let mut session = flying(&catalog(), 90.0);
        for _ in 0..60 {
            session.tick(held(true, Turn::None, false));
        }
        for _ in 0..30 {
            session.tick(held(false, Turn::Left, false));
        }
        assert!(session.player.velocity.x > 5.0, "{:?}", session.player);
    }

    #[test]
    fn reverse_brakes_the_speed_by_a_ticks_acceleration_to_none_and_gives_no_aim() {
        let mut session = flying(&hull(), 90.0);
        session.player_speed = 1.0;
        session.player.velocity = Vec2::new(1.0, 0.0);
        session.aim = 45;
        let accel = session.flying_handling().accel;
        session.tick(held(false, Turn::None, true));
        assert!((session.player_speed() - (1.0 - accel)).abs() < 1e-6);
        assert_eq!(session.player.heading, 90.0, "reverse turns nothing");
        assert_eq!(session.aim, 45, "and writes no aim");
        session.tick(held(false, Turn::Right, true));
        assert_eq!(session.player.heading, 93.0, "the turn keys still turn");
        for _ in 0..20 {
            session.tick(held(false, Turn::None, true));
        }
        assert_eq!(session.player_speed(), 0.0, "floored at none");
        assert_eq!(session.player.velocity, Vec2::ZERO);
    }

    #[test]
    fn a_disabled_inertialess_players_speed_falls_by_0_985_a_tick_and_its_glow_by_two() {
        let mut session = flying(&hull(), 90.0);
        session.player_speed = 4.0;
        session.player.velocity = Vec2::new(4.0, 0.0);
        session.engine_glow = 10;
        session.condition = Condition::Disabled;
        session.tick(held(true, Turn::Left, true));
        assert!((session.player_speed() - 4.0 * 0.985).abs() < 1e-5);
        assert_eq!(session.engine_glow(), 8, "coasting and disabled");
        assert_eq!(session.player.heading, 90.0, "no keys");
        session.tick(Controls::default());
        assert!((session.player_speed() - 4.0 * 0.985 * 0.985).abs() < 1e-5);
        assert_eq!(session.engine_glow(), 6);
    }

    #[test]
    fn coasting_at_full_speed_holds_the_glow_and_thrusting_there_settles_it_at_23() {
        let mut session = flying(&hull(), 0.0);
        session.player_speed = session.top_speed();
        session.player.velocity = facing(0.0) * session.top_speed();
        session.engine_glow = 15;
        for _ in 0..10 {
            session.tick(Controls::default());
        }
        assert_eq!(session.engine_glow(), 15, "coast -1, follow +1");
        for _ in 0..10 {
            session.tick(held(true, Turn::None, false));
        }
        assert_eq!(session.engine_glow(), 23);
    }

    #[test]
    fn coasting_at_half_speed_from_cruise_the_glow_falls_to_12() {
        let mut session = flying(&hull(), 0.0);
        let half = session.top_speed() / 2.0;
        session.player_speed = half;
        session.player.velocity = facing(0.0) * half;
        session.engine_glow = 24;
        for _ in 0..30 {
            session.tick(Controls::default());
        }
        assert_eq!(session.engine_glow(), 12);
    }

    #[test]
    fn the_inertialess_pre_jump_brakes_the_speed_without_turning_then_turns_to_the_bearing() {
        let mut session = super::super::tests::bound_for_131(
            &hull(),
            super::super::tests::OUT,
            Vec2::new(0.0, -6.0),
            0.0,
        );
        session.player_speed = 6.0;
        session.aim = 7;
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        let accel = session.flying_handling().accel;
        session.tick(Controls::default());
        assert!((session.player_speed() - (6.0 - accel)).abs() < 1e-5);
        assert_eq!(session.player.heading, 0.0, "the brake does not turn");
        assert_eq!(session.aim, 7, "nor aim");
        let mut braking = 1;
        while !slow_enough(session.player.velocity) {
            session.tick(Controls::default());
            assert_eq!(session.player.heading, 0.0);
            braking += 1;
            assert!(braking < 100, "{:?}", session.player);
        }
        assert!(session.player_speed() > 0.0, "slow enough before it stops");
        for _ in 0..200 {
            if session.jumping().is_some() {
                break;
            }
            session.tick(Controls::default());
        }
        assert_eq!(session.jumping(), Some(SystemId(131)));
        assert!(
            (87.0..=90.0).contains(&session.player.heading),
            "turned east: {:?}",
            session.player
        );
    }

    /// Braking at 6 of a top speed of 9, by 0.18 a tick, the speed's
    /// glow target is 16 before the tick and 15 after it: from 16 the
    /// glow follows the braked speed down to 14, where the inertial ramp
    /// would hold 15 and the unbraked speed raise 16. From 0 it rises to
    /// 1, where the inertial ramp holds 0 and no speed would lower it.
    #[test]
    fn the_inertialess_pre_jump_brakes_the_glow_toward_the_braked_speed() {
        let mut session = super::super::tests::bound_for_131(
            &hull(),
            super::super::tests::OUT,
            Vec2::new(0.0, -6.0),
            0.0,
        );
        session.player_speed = 6.0;
        assert_eq!(session.begin_jump(), Ok(SystemId(131)));
        let max_speed = session.flying_handling().max_speed;
        for (base, glow) in [(16_i16, 14_i16), (0, 1)] {
            session.engine_glow = base;
            session.tick(Controls::default());
            assert!(!slow_enough(session.player.velocity), "still braking");
            let follow = Follow::Speed {
                speed: session.player_speed(),
                max_speed,
            };
            assert_eq!(
                session.engine_glow,
                ramp_inertialess_glow(base, false, follow, false),
                "from {base}"
            );
            assert_eq!(session.engine_glow, glow, "from {base}");
            assert_ne!(glow, crate::glow::ramp_glow(base, false), "from {base}");
        }
    }

    #[test]
    fn a_new_pilot_starts_at_none_and_landing_and_a_jumps_arrival_set_none() {
        let catalog = hull();
        let mut session = flying(&catalog, 0.0);
        assert_eq!(session.player_speed(), 0.0, "a new pilot");
        session.player_speed = 3.0;
        session.dock(crate::catalog::StellarId(128));
        assert_eq!(session.player_speed(), 0.0, "landed");
        let mut session = flying(&catalog, 0.0);
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert_eq!(
            session.player.velocity,
            Vec2::ZERO,
            "the engine's at-rest arrival"
        );
        assert_eq!(session.player_speed(), 0.0);
    }

    #[test]
    fn a_gate_exit_sets_the_speed_to_the_exits_half_top_speed() {
        let gates = FakePilotCatalog {
            ships: hull().ships,
            ..super::super::tests::gated()
        };
        let mut session = super::super::tests::at_gate(&gates);
        session
            .enter_hypergate(Some(SystemId(131)), &gates, &mut crate::chance::NeverFires)
            .expect("enters");
        let half = session.top_speed() * crate::gate::EXIT_SPEED_FRACTION;
        assert!(
            (session.player_speed() - half).abs() < 1e-4,
            "{}",
            session.player_speed()
        );
        assert!(near(
            session.player.velocity,
            facing(session.player.heading) * half
        ));
    }
}
