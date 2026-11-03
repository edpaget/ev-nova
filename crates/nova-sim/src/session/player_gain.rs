//! The frame's gain in the player's pass, where `Session::move_player`
//! and [`Session::field`](super::Session) read it in `session.rs`
//! (`_HandlePlayer`'s `_gSpeedMult`, see `nova_sim_base::speed_mult`).
//!
//! `Session::move_player` reads `Session::speed_mult` once a pass and
//! hands it to each of the player's readers: the turn and its bank
//! (`ship_display`), the thrust and the move ([`player_step`], or the
//! `inertialess` module's flight), the ion block (`ionization`), the
//! afterburner's drain (`afterburner`), the warp stage and the pre-jump
//! (`warp_stage`, [`pre_jump`](crate::pre_jump)), and the fuel's
//! regeneration ([`regenerate`]), out of a jump and through its
//! hyperspace alike. The fight reads it from the pass's [`Field`]
//! (`Session::field`): the player's shield and armour regeneration and
//! its reloads; the gravity from the pass's stellar hazards (`gravity`).
//! The pod, the repair inhibit, the sabotage clock, the self-destruct
//! count and the cloak's upkeep read it in their own modules.
//!
//! Both of double time's passes run at one gain, as `adjust_time` runs
//! once a step before them.
//!
//! These tests pin the wiring only: each expected value is the upstream
//! function at the same gain.
//!
//! [`player_step`]: crate::flight::player_step
//! [`regenerate`]: crate::fuel::regenerate
//! [`Field`]: crate::combat::Field

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::SpeedMult;
    use crate::flight::{Controls, FlightRules, Turn, player_step};
    use crate::session::{Jump, Mode, Session};
    use crate::testkit::{catalog, edge_lander, work};

    const HALF: SpeedMult = SpeedMult::new(0.5);

    const THRUST: Controls = Controls {
        thrust: true,
        turn: Turn::None,
        reverse: false,
        face: crate::flight::Face::Off,
        afterburner: false,
    };

    #[test]
    fn at_half_gain_the_players_thrust_and_move_take_half_a_frame() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        work(&mut session, &catalog, Controls::default());
        session.at_gain(HALF);
        let mut expected = *session.player();
        let caps = session.player_caps();
        let handling = session.handling();
        work(&mut session, &catalog, THRUST);
        player_step(
            &mut expected,
            &handling,
            THRUST,
            None,
            caps,
            FlightRules::default(),
            HALF,
        );
        assert_eq!(*session.player(), expected);
    }

    #[test]
    fn at_half_gain_the_fuel_regenerates_half_its_rate_in_flight_and_in_hyperspace() {
        let catalog = edge_lander();
        for hyperspace in [false, true] {
            let mut session = Session::start(&catalog).expect("starts");
            session.pilot.reserves.fuel.now = 100.0;
            if hyperspace {
                session.mode = Mode::Jumping(Jump::Hyperspace {
                    to: crate::catalog::SystemId(132),
                    warp: 3,
                });
            }
            session.at_gain(HALF);
            let mut expected = session.pilot.reserves.fuel;
            let regen = session.ship.fit.stats.fuel_regen;
            work(&mut session, &catalog, Controls::default());
            crate::fuel::regenerate(&mut expected, regen, HALF);
            assert_eq!(
                session.reserves().fuel.now,
                expected.now,
                "hyperspace {hyperspace}"
            );
            assert_ne!(expected.now, 100.0 + regen, "a half, not a frame");
        }
    }

    #[test]
    fn the_fights_field_carries_the_sessions_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.field().speed_mult, SpeedMult::FULL);
        session.at_gain(HALF);
        assert_eq!(session.field().speed_mult, HALF);
    }

    #[test]
    fn at_half_gain_a_turning_pass_turns_the_player_by_the_turn_at_that_gain() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.at_gain(HALF);
        let before = session.player().heading;
        let rate = session.handling().turn_rate;
        let rule = session.setup.flight_rules().player_turn;
        let right = Controls {
            turn: Turn::Right,
            ..Controls::default()
        };
        work(&mut session, &catalog, right);
        let (expected, _) = crate::player_turn::turn(before, rate, None, Turn::Right, rule, HALF);
        let (full, _) =
            crate::player_turn::turn(before, rate, None, Turn::Right, rule, SpeedMult::FULL);
        assert_ne!(expected, full);
        assert_eq!(session.player().heading, expected);
    }
}
