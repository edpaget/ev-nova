//! The player's analog amounts: how far a stick holds the turn and the
//! thrust ([`Amounts`]), which each frame's [`StepInput`] carries beside
//! the on/off controls. The session holds them on the player's ship for
//! the frame's steps, double time's second pass included, and hands them
//! to the player's step with the controls ([`PlayerControls`]): its
//! inertial or inertialess flight scales the turn keys' step and the
//! thrust by them (`nova_sim_base`'s `player_step` and
//! `inertialess_player_step` own how). Everything else that reads the
//! controls (the glow, the thrust sound, the afterburner, reverse's aim,
//! the jump's stages) reads them on or off, so a stick held at all
//! thrusts there as the key does.
//!
//! With keys only the amounts are [`Amounts::FULL`], the default, and the
//! step is the controls' own.
//!
//! [`StepInput`]: crate::flight_frame::StepInput

use crate::flight::{Amounts, Controls, PlayerControls};

use super::player_ship::PlayerShip;

impl PlayerShip {
    /// Holds `amounts` for the frame's steps.
    pub(super) fn hold_amounts(&mut self, amounts: Amounts) {
        self.amounts = amounts;
    }

    /// `controls`, held at the amounts held.
    pub(super) fn held_at(&self, controls: Controls) -> PlayerControls {
        PlayerControls {
            controls,
            amounts: self.amounts,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::Session;
    use crate::catalog::ShipId;
    use crate::chance::NeverFires;
    use crate::combat::Rules;
    use crate::flight::{Amounts, Controls, Turn};
    use crate::flight_frame::{StepInput, StepReport};
    use crate::handling::ShipFields;
    use crate::session::StepCollaborators;
    use crate::stats::INERTIALESS_HULL;
    use crate::testkit::{FAST, FakePilotCatalog, catalog};

    /// `session` after one frame of `count` steps under `input`, and the
    /// frame's report.
    fn stepped(catalog: &FakePilotCatalog, input: &StepInput, count: u32) -> (Session, StepReport) {
        let mut session = Session::start(catalog).expect("starts");
        let behaviour = crate::ai::NovaAi::default();
        let mut chance = NeverFires;
        let mut collab = StepCollaborators {
            catalog,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        let mut report = StepReport::default();
        session.step(input, count, &mut collab, &mut report);
        (session, report)
    }

    fn turning_at(turn: f32, double: bool) -> StepInput {
        StepInput {
            controls: Controls {
                turn: Turn::Right,
                ..Controls::default()
            },
            amounts: Amounts { turn, thrust: 1.0 },
            double_time_key: double,
            window_width: 1024,
            ..StepInput::default()
        }
    }

    fn thrusting_at(thrust: f32) -> StepInput {
        StepInput {
            controls: Controls {
                thrust: true,
                ..Controls::default()
            },
            amounts: Amounts { turn: 1.0, thrust },
            window_width: 1024,
            ..StepInput::default()
        }
    }

    /// How far the player's ship turned from its start over `steps`.
    fn turned(input: &StepInput, steps: u32) -> f32 {
        let catalog = catalog();
        let start = Session::start(&catalog).expect("starts").player().heading;
        let (session, _) = stepped(&catalog, input, steps);
        (session.player().heading - start).rem_euclid(360.0)
    }

    #[test]
    fn the_input_holds_full_amounts_by_default() {
        assert_eq!(StepInput::default().amounts, Amounts::FULL);
    }

    #[test]
    fn a_step_with_a_half_turn_amount_turns_the_player_part_of_its_full_turn() {
        let none = turned(&turning_at(0.0, false), 4);
        let half = turned(&turning_at(0.5, false), 4);
        let full = turned(&turning_at(1.0, false), 4);
        assert!(none < half && half < full, "{none} {half} {full}");
        assert_eq!(none, 0.0);
    }

    #[test]
    fn a_step_with_half_throttle_thrusts_part_of_its_full_thrust() {
        let speed = |thrust| {
            let (session, _) = stepped(&catalog(), &thrusting_at(thrust), 4);
            session.player().velocity.length()
        };
        let (none, half, full) = (speed(0.0), speed(0.5), speed(1.0));
        assert!(none < half && half < full, "{none} {half} {full}");
        assert_eq!(none, 0.0);
    }

    #[test]
    fn an_inertialess_step_with_half_throttle_thrusts_part_of_its_full_thrust() {
        let catalog = FakePilotCatalog {
            ships: vec![(
                ShipId(128),
                Ok(ShipFields {
                    flags2: INERTIALESS_HULL,
                    ..FAST
                }),
            )],
            ..catalog()
        };
        let speed = |thrust| {
            let (session, _) = stepped(&catalog, &thrusting_at(thrust), 4);
            session.player_speed()
        };
        let (none, half, full) = (speed(0.0), speed(0.5), speed(1.0));
        assert!(none < half && half < full, "{none} {half} {full}");
        assert_eq!(none, 0.0);
    }

    #[test]
    fn a_double_time_second_pass_keeps_the_held_amounts() {
        // The heading as the first pass left it, and as the second did.
        let passes = |turn| {
            let catalog = catalog();
            let (session, report) = stepped(&catalog, &turning_at(turn, true), 1);
            let between = report.steps[0]
                .between_passes
                .as_ref()
                .expect("a double-time step")
                .ships[0]
                .heading;
            (between, session.player().heading)
        };
        let (half_first, half_second) = passes(0.5);
        let (full_first, full_second) = passes(1.0);
        let half_turn = (half_second - half_first).rem_euclid(360.0);
        let full_turn = (full_second - full_first).rem_euclid(360.0);
        assert!(half_first < full_first, "{half_first} {full_first}");
        assert!(
            0.0 < half_turn && half_turn < full_turn,
            "the second pass turns part of its full turn too: {half_turn} {full_turn}"
        );
    }
}
