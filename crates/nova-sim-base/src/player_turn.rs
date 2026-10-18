//! The player's turn: `_HandlePlayer`'s turn block (@0x6c899-0x6cad4 in
//! `EV Nova.app/Contents/MacOS/EV Nova`, i386), which turns the player's
//! ship each tick, and the aims its auto-turn turns to.
//!
//! **The rate.** The block takes `_ShipTurnRate` truncated to whole
//! degrees (`cvttss2si` @0x6c8b1), and each turn moves that times
//! `_gSpeedMult`, 1 at the engine's tick: `Maneuver` 25 turns 2° a tick,
//! not 2.5 ([`step_of`]). NPCs turn the full rate (`_HandleShip`), so
//! [`flight::step`](crate::flight::step) keeps theirs.
//!
//! **The auto-turn.** A local of `_HandlePlayer`, −0x2cd, says the ship
//! turns itself this tick toward its desired heading, +0x68 (the
//! session's aim). The face-target key sets it (@0x6af69), and so do
//! reverse (@0x6c892, aiming as [`reverse_aim`] does), the jump's turn to
//! the map bearing (@0x6b0d3) and the jump's brake against the velocity
//! (@0x6b861, aiming as [`brake_aim`] does). With it set ([`turn`]):
//!
//! - when |`_AngularDifference`(+0x68, the heading's whole degree)| is
//!   more than one turn (@0x6c8c2-0x6c8ff) and the ship isn't disabled
//!   (@0x6c905), it turns one turn: left when +0x68 less the heading,
//!   brought into [0, 360), is over 180 (@0x6c912-0x6c974), otherwise
//!   right (@0x6c979-0x6c995);
//! - within one turn it neither turns nor snaps onto the aim, so it stops
//!   up to a turn short ([`settled`]).
//!
//! **The turn keys** (+0x26 left, +0x28 right) work only out of a jump,
//! not disabled and with −0x2cd clear (@0x6c99a-0x6c9c0): while reverse
//! or the face-target key aims, they do nothing, even once the ship has
//! stopped within a turn of the aim. Left or right alone turns one turn;
//! both together do nothing (@0x6c9f8-0x6ca95).
//!
//! The heading is then wrapped once into [0, 360) (@0x6ca9e-0x6cacf);
//! ours brings it in whatever the turn
//! ([`normalized`](crate::flight::normalized)), which differs only for a
//! turn of more than 360° a tick. The way it turned, −1, 0 or +1 (ebx),
//! feeds the ship's bank (@0x6cd59, @0x6ce6b), which the engine doesn't
//! draw yet (rdm:phase/ship-handling-fidelity/phase-10-ship-drawing):
//! [`turn`] gives it as a [`Turn`].
//!
//! [`RuleKey::PlayerTurn`]'s other reading ([`PlayerTurn::Full`]) turns
//! the full rate and lands an auto-turn exactly on its aim, as the port
//! did before.
//!
//! None of the turn, reverse or face-target blocks plays a sound.

use crate::flight::{Turn, normalized, shortest_turn, turn_toward};
use crate::geometry::Vec2;
#[cfg(any(doc, test))]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;
use crate::speed_cap::whole_degree;
use crate::trig::{angular_difference, intercept_angle};

/// How the player turns, by [`RuleKey::PlayerTurn`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerTurn {
    /// The original's: by the whole degrees of its turn rate, an
    /// auto-turn stopping within a turn of its aim.
    #[default]
    WholeDegrees,
    /// The port's earlier reading: by the full rate, an auto-turn landing
    /// exactly on its aim.
    Full,
}

impl From<RuleSource> for PlayerTurn {
    /// The engine's reading is the original's whole degrees; the other is
    /// the full rate.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::WholeDegrees,
            RuleSource::Bible => Self::Full,
        }
    }
}

/// What the player's velocity is scaled by before reverse and the jump's
/// brake aim against it (100.0, f32 @0xdd060).
pub const AIM_SCALE: f32 = 100.0;

/// The speed, on either axis, from which reverse aims against the
/// velocity (0.05, f64 @0xdd990, @0x6c7f4 and @0x6c81a).
pub const REVERSE_AIM_SPEED: f64 = 0.05;

/// How far, in degrees, a ship turning `rate` degrees a tick turns in one
/// tick under `rule`: the rate's whole degrees (`cvttss2si` @0x6c8b1), or
/// the rate itself.
#[must_use]
pub fn step_of(rate: f32, rule: PlayerTurn) -> f32 {
    match rule {
        PlayerTurn::WholeDegrees => rate.trunc(),
        PlayerTurn::Full => rate,
    }
}

/// Whether a ship facing `heading` and turning `rate` degrees a tick has
/// done turning toward `aim` under `rule`: by the original, within one
/// turn of it, as its auto-turn stops there (@0x6c8c2-0x6c8ff); by the
/// other reading, on it.
#[must_use]
pub fn settled(heading: f32, rate: f32, aim: i16, rule: PlayerTurn) -> bool {
    match rule {
        PlayerTurn::WholeDegrees => f32::from(off_aim(heading, aim)) <= step_of(rate, rule),
        PlayerTurn::Full => shortest_turn(heading, f32::from(aim)) == 0.0,
    }
}

/// How far, in whole degrees, `heading` is off `aim`: |`_AngularDifference`|
/// of the aim and the heading's whole degree (@0x6c8c2-0x6c8ea).
#[allow(clippy::cast_possible_truncation)]
fn off_aim(heading: f32, aim: i16) -> i16 {
    angular_difference(aim, whole_degree(heading) as i16).abs()
}

/// `heading` turned one tick by a ship turning `rate` degrees a tick under
/// `rule`, and the way it turned (see the module docs): toward `aim` when
/// the auto-turn aims (`Some`), the turn keys `keys` held off; otherwise
/// by `keys`.
#[must_use]
pub fn turn(
    heading: f32,
    rate: f32,
    aim: Option<i16>,
    keys: Turn,
    rule: PlayerTurn,
) -> (f32, Turn) {
    let step = step_of(rate, rule);
    let way = match aim {
        Some(aim) if rule == PlayerTurn::Full => {
            let turned = turn_toward(heading, f32::from(aim), step);
            let way = match shortest_turn(heading, turned) {
                off if off < 0.0 => Turn::Left,
                off if off > 0.0 => Turn::Right,
                _ => Turn::None,
            };
            return (turned, way);
        }
        Some(aim) if !settled(heading, rate, aim, rule) => {
            let mut ahead = f32::from(aim) - heading;
            while ahead >= 360.0 {
                ahead -= 360.0;
            }
            while ahead < 0.0 {
                ahead += 360.0;
            }
            if ahead > 180.0 {
                Turn::Left
            } else {
                Turn::Right
            }
        }
        Some(_) => Turn::None,
        None => keys,
    };
    let heading = match way {
        Turn::Left => normalized(heading - step),
        Turn::Right => normalized(heading + step),
        Turn::None => heading,
    };
    (heading, way)
}

/// The heading reverse aims a ship moving at `velocity` at, as
/// `_HandlePlayer` does (@0x6c7d8-0x6c892): against its velocity,
/// (`_CalcInterceptAngle`(0, v × [`AIM_SCALE`]) + 180) mod 360, when it
/// moves at [`REVERSE_AIM_SPEED`] or more on either axis; otherwise none,
/// and reverse does nothing.
#[must_use]
pub fn reverse_aim(velocity: Vec2) -> Option<i16> {
    let fast = |speed: f32| f64::from(speed.abs()) >= REVERSE_AIM_SPEED;
    (fast(velocity.x) || fast(velocity.y)).then(|| {
        let toward = intercept_angle(Vec2::ZERO, velocity * AIM_SCALE);
        (toward + 180) % 360
    })
}

/// The heading the jump's brake aims a ship moving at `velocity` at:
/// `_CalcInterceptAngle`(v × [`AIM_SCALE`], 0) (@0x6b761), against its
/// velocity, however slow.
#[must_use]
pub fn brake_aim(velocity: Vec2) -> i16 {
    intercept_angle(velocity * AIM_SCALE, Vec2::ZERO)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::rulebook::Rulebook;

    const WHOLE: PlayerTurn = PlayerTurn::WholeDegrees;
    const FULL: PlayerTurn = PlayerTurn::Full;

    #[test]
    fn the_rule_reads_the_rulebook() {
        assert_eq!(PlayerTurn::default(), WHOLE);
        assert_eq!(
            PlayerTurn::from(Rulebook::default().source_for(RuleKey::PlayerTurn)),
            WHOLE
        );
        assert_eq!(PlayerTurn::from(RuleSource::Bible), FULL);
    }

    #[test]
    fn the_named_values_are_the_originals() {
        assert_eq!(AIM_SCALE, 100.0);
        assert_eq!(REVERSE_AIM_SPEED, 0.05);
    }

    #[test]
    fn a_turn_goes_the_rates_whole_degrees_by_the_engine_and_the_full_rate_by_the_other() {
        assert_eq!(step_of(2.5, WHOLE), 2.0);
        assert_eq!(step_of(2.5, FULL), 2.5);
        assert_eq!(step_of(0.9, WHOLE), 0.0);
        assert_eq!(step_of(3.0, WHOLE), 3.0);
        assert_eq!(
            turn(90.0, 2.5, None, Turn::Right, WHOLE),
            (92.0, Turn::Right)
        );
        assert_eq!(turn(90.0, 2.5, None, Turn::Left, WHOLE), (88.0, Turn::Left));
        assert_eq!(
            turn(90.0, 2.5, None, Turn::Right, FULL),
            (92.5, Turn::Right)
        );
        assert_eq!(turn(90.0, 2.5, None, Turn::Left, FULL), (87.5, Turn::Left));
        assert_eq!(turn(90.0, 2.5, None, Turn::None, WHOLE), (90.0, Turn::None));
    }

    #[test]
    fn the_keys_wrap_the_heading_into_0_to_360() {
        assert_eq!(turn(359.0, 3.0, None, Turn::Right, WHOLE).0, 2.0);
        assert_eq!(turn(1.0, 3.0, None, Turn::Left, WHOLE).0, 358.0);
        assert_eq!(turn(357.0, 3.0, None, Turn::Right, WHOLE).0, 0.0);
        let tiny = turn(0.0, 1e-6, None, Turn::Left, FULL).0;
        assert!((0.0..360.0).contains(&tiny), "{tiny}");
    }

    #[test]
    fn the_auto_turn_turns_while_more_than_a_turn_off_and_stops_without_snapping() {
        // From 0 toward 9 at 2 a tick: 6 is 3 off, so it turns on to 8,
        // 1 off, and stops there.
        let mut heading = 0.0;
        let mut seen = vec![];
        for _ in 0..6 {
            let (next, _) = turn(heading, 2.5, Some(9), Turn::None, WHOLE);
            heading = next;
            seen.push(heading);
        }
        assert_eq!(seen, [2.0, 4.0, 6.0, 8.0, 8.0, 8.0], "stops 1 short");
        assert!(settled(8.0, 2.5, 9, WHOLE));
        assert!(settled(7.0, 2.5, 9, WHOLE), "exactly a turn off");
        assert!(!settled(6.0, 2.5, 9, WHOLE));
        // By the other reading it lands exactly.
        let mut heading = 0.0;
        for _ in 0..4 {
            heading = turn(heading, 2.5, Some(9), Turn::None, FULL).0;
        }
        assert_eq!(heading, 9.0);
        assert!(settled(9.0, 2.5, 9, FULL));
        assert!(!settled(8.0, 2.5, 9, FULL));
    }

    #[test]
    fn the_auto_turn_reads_the_headings_whole_degree() {
        // 5.9 is whole degree 5, 3 off 8: more than 2, so it turns.
        assert_eq!(turn(5.9, 2.0, Some(8), Turn::None, WHOLE).0, 7.9);
        // 6.9 is 6, 2 off: within a turn.
        assert_eq!(turn(6.9, 2.0, Some(8), Turn::None, WHOLE).0, 6.9);
        // A heading a rounding step short of a whole degree is on it.
        assert!(settled(5.999_9, 2.0, 8, WHOLE));
    }

    #[test]
    fn the_auto_turn_goes_left_past_180_ahead_and_right_at_180_or_less() {
        assert_eq!(
            turn(0.0, 3.0, Some(180), Turn::None, WHOLE),
            (3.0, Turn::Right)
        );
        assert_eq!(
            turn(0.0, 3.0, Some(181), Turn::None, WHOLE),
            (357.0, Turn::Left)
        );
        assert_eq!(
            turn(0.0, 3.0, Some(179), Turn::None, WHOLE),
            (3.0, Turn::Right)
        );
        assert_eq!(
            turn(350.0, 3.0, Some(10), Turn::None, WHOLE),
            (353.0, Turn::Right)
        );
        assert_eq!(
            turn(10.0, 3.0, Some(350), Turn::None, WHOLE),
            (7.0, Turn::Left)
        );
        // Ahead is measured from the float heading: 0.5 to 180.0 is 179.5.
        assert_eq!(turn(0.5, 3.0, Some(180), Turn::None, WHOLE).1, Turn::Right);
        assert_eq!(turn(359.5, 3.0, Some(180), Turn::None, WHOLE).1, Turn::Left);
    }

    #[test]
    fn while_the_auto_turn_aims_the_turn_keys_do_nothing() {
        for keys in [Turn::Left, Turn::Right] {
            assert_eq!(turn(0.0, 3.0, Some(90), keys, WHOLE), (3.0, Turn::Right));
            assert_eq!(
                turn(89.0, 3.0, Some(90), keys, WHOLE),
                (89.0, Turn::None),
                "settled, and still held off"
            );
            assert_eq!(turn(90.0, 3.0, Some(90), keys, FULL), (90.0, Turn::None));
        }
    }

    #[test]
    fn a_ship_that_cannot_turn_a_whole_degree_never_turns() {
        assert_eq!(
            turn(0.0, 0.9, Some(90), Turn::None, WHOLE),
            (0.0, Turn::Right),
            "it turns by none"
        );
        assert_eq!(turn(0.0, 0.9, None, Turn::Right, WHOLE), (0.0, Turn::Right));
        assert!(!settled(0.0, 0.9, 90, WHOLE));
    }

    #[test]
    fn reverse_aims_against_the_velocity_from_0_05_on_either_axis() {
        assert_eq!(reverse_aim(Vec2::ZERO), None);
        assert_eq!(reverse_aim(Vec2::new(0.049, -0.049)), None);
        assert_eq!(reverse_aim(Vec2::new(0.0, -0.05)), Some(180), "moving up");
        assert_eq!(reverse_aim(Vec2::new(0.05, 0.0)), Some(270), "moving right");
        assert_eq!(reverse_aim(Vec2::new(-3.0, 0.0)), Some(90));
        assert_eq!(reverse_aim(Vec2::new(0.0, 3.0)), Some(0));
        // Down and to the left, 1 in 6: the table's 9° off straight left,
        // behind it up and to the right.
        assert_eq!(reverse_aim(Vec2::new(-0.06, 0.01)), Some(81));
    }

    #[test]
    fn the_brake_aims_against_the_velocity_however_slow() {
        assert_eq!(brake_aim(Vec2::new(0.0, -3.0)), 180);
        assert_eq!(brake_aim(Vec2::new(0.01, 0.0)), 270);
        assert_eq!(brake_aim(Vec2::ZERO), 0);
    }
}
