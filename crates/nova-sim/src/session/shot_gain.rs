//! The frame's gain in the shots' flight, where the session hands it to
//! the fight (`_HandleShot`'s, `_HandleShotGuidance`'s and `_SpawnShot`'s
//! `_gSpeedMult`, see `nova_sim_base::speed_mult`).
//!
//! `Session::field` fills the pass's `Field::speed_mult` with
//! `Session::speed_mult`, and the fight hands it to each shot's visit
//! (its life, move, homing, lock, spin, fade and decay) and to the
//! interference roll at launch (`nova_sim_combat`'s
//! `combat::projectile`). No sub-state method reads it here. The frame
//! carries each shot's life and the fade its last visit read
//! ([`FrameShot`](crate::flight_frame::FrameShot)), which the view shades
//! it by.
//!
//! These tests pin the wiring only: each expected value is the upstream
//! function at the same gain.

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::tests::{FIRE, armed};
    use crate::SpeedMult;
    use crate::Trigger;
    use crate::flight::{Controls, moved};
    use crate::session::Session;
    use crate::testkit::work;

    const HALF: SpeedMult = SpeedMult::new(0.5);

    #[test]
    fn at_half_gain_a_bare_pass_moves_a_players_shot_half_its_velocity_and_counts_its_life_down_half()
     {
        let catalog = armed();
        let mut session = Session::start(&catalog).expect("starts");
        session.hold_trigger(FIRE);
        work(&mut session, &catalog, Controls::default());
        session.hold_trigger(Trigger::default());
        let fired = session.shots()[0];
        session.at_gain(HALF);
        work(&mut session, &catalog, Controls::default());
        let flown = session.shots()[0];
        assert_eq!(flown.id, fired.id);
        assert_eq!(flown.position, moved(fired.position, fired.velocity, HALF));
        assert_eq!(flown.life, HALF.add_times(fired.life, -1.0));
    }
}
