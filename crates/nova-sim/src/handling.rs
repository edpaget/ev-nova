//! A ship's handling: its `shïp` record's speed, acceleration and turn rate,
//! in the simulation's units.
//!
//! The simulation measures space in pixels, as Nova does (the Bible:
//! "Jump Distance 1000 pixels"; a `spöb`'s position is in pixels from the
//! system's centre), and time in ticks of 1/30 s (see [`crate::clock`]).
//! Every conversion from the record's numbers is a named constant here.
//! They are chosen so flight feels like Nova, not to match the original
//! number for number.

/// A `shïp`'s `Speed`, `Accel`, `Maneuver`, `Shield`, `Armor` and `Fuel`,
/// raw from the record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipFields {
    /// `Speed`: top speed; 300 is average.
    pub speed: i16,
    /// `Accel`: acceleration; 300 is average.
    pub accel: i16,
    /// `Maneuver`: turn rate; 10 is about 30°/s.
    pub maneuver: i16,
    /// `Shield`: shield strength; negative means 5x the absolute value.
    pub shield: i16,
    /// `Armor`: armour strength.
    pub armor: i16,
    /// `Fuel`: fuel capacity; 100 is one jump.
    pub fuel: i16,
}

/// `Speed` per pixel a tick. The Bible gives a weapon's speed in "pixels
/// per frame x100"; a ship's `Speed` is read on the same scale, so an
/// average ship (300) tops out at 3 pixels a tick, 90 a second.
pub const SPEED_PER_PIXEL_PER_TICK: f32 = 100.0;

/// `Accel` per pixel a tick, per tick: the speed scale over a second's
/// ticks, so a ship whose `Accel` equals its `Speed` reaches top speed from
/// rest in about a second.
pub const ACCEL_PER_PIXEL_PER_TICK_SQUARED: f32 = SPEED_PER_PIXEL_PER_TICK * 30.0;

/// `Maneuver` per degree a tick. The Bible: "10 is about 30°/s", which is
/// 1° a tick at 30 ticks a second.
pub const MANEUVER_PER_DEGREE_PER_TICK: f32 = 10.0;

/// How a ship flies, in pixels and ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Handling {
    /// Top speed, in pixels a tick.
    pub max_speed: f32,
    /// Thrust, in pixels a tick added to the speed each tick.
    pub accel: f32,
    /// Turn rate, in degrees a tick.
    pub turn_rate: f32,
}

impl Handling {
    /// The handling of a ship with these fields. A field of 0 or below
    /// gives 0: the ship cannot move that way.
    #[must_use]
    pub fn from_fields(fields: ShipFields) -> Self {
        let positive = |field: i16| f32::from(field.max(0));
        Self {
            max_speed: positive(fields.speed) / SPEED_PER_PIXEL_PER_TICK,
            accel: positive(fields.accel) / ACCEL_PER_PIXEL_PER_TICK_SQUARED,
            turn_rate: positive(fields.maneuver) / MANEUVER_PER_DEGREE_PER_TICK,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn handling(speed: i16, accel: i16, maneuver: i16) -> Handling {
        Handling::from_fields(ShipFields {
            speed,
            accel,
            maneuver,
            ..ShipFields::default()
        })
    }

    #[test]
    fn an_average_ship_tops_out_at_3_pixels_a_tick_in_a_second() {
        let average = handling(300, 300, 10);
        assert_eq!(average.max_speed, 3.0);
        assert_eq!(average.accel, 0.1);
        assert_eq!(average.turn_rate, 1.0);
        assert!((average.max_speed / average.accel - 30.0).abs() < 1e-4);
    }

    #[test]
    fn each_field_converts_on_its_own_scale() {
        let fast = handling(1000, 600, 30);
        assert_eq!(fast.max_speed, 10.0);
        assert_eq!(fast.accel, 0.2);
        assert_eq!(fast.turn_rate, 3.0);
        assert_eq!(
            (
                SPEED_PER_PIXEL_PER_TICK,
                ACCEL_PER_PIXEL_PER_TICK_SQUARED,
                MANEUVER_PER_DEGREE_PER_TICK
            ),
            (100.0, 3000.0, 10.0)
        );
    }

    #[test]
    fn zero_or_negative_fields_cannot_move_the_ship() {
        assert_eq!(handling(0, 0, 0), Handling::default());
        assert_eq!(handling(-300, -1, i16::MIN), Handling::default());
        let stuck = handling(300, -5, 10);
        assert_eq!(
            (stuck.max_speed, stuck.accel, stuck.turn_rate),
            (3.0, 0.0, 1.0)
        );
    }
}
