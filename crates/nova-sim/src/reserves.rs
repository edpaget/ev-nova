//! A ship's reserves: its shield, armour and fuel, each a gauge that
//! starts full at its `shïp` record's value.
//!
//! The Bible's rules for the record's numbers: a negative `Shield` means
//! five times its absolute value, and a negative `Armor` or `Fuel` means
//! none.

use crate::handling::ShipFields;

/// How much of something a ship has, out of how much it can hold.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gauge {
    /// How much it has now.
    pub now: f32,
    /// How much it can hold.
    pub max: f32,
}

impl Gauge {
    /// A gauge holding all `max` of its capacity.
    #[must_use]
    pub const fn full(max: f32) -> Self {
        Self { now: max, max }
    }

    /// How full it is, in `[0, 1]`; 0 when it can hold nothing.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.max > 0.0 {
            (self.now / self.max).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// A ship's shield, armour and fuel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reserves {
    /// Shield points.
    pub shield: Gauge,
    /// Armour points.
    pub armor: Gauge,
    /// Fuel; 100 is one jump.
    pub fuel: Gauge,
}

/// How many shield points a negative `Shield` is per unit of its absolute
/// value (the Bible).
pub const NEGATIVE_SHIELD_FACTOR: f32 = 5.0;

impl Reserves {
    /// A new ship's reserves, full, from its record's fields.
    #[must_use]
    pub fn from_fields(fields: ShipFields) -> Self {
        let shield = if fields.shield < 0 {
            f32::from(fields.shield).abs() * NEGATIVE_SHIELD_FACTOR
        } else {
            f32::from(fields.shield)
        };
        let positive = |field: i16| f32::from(field.max(0));
        Self {
            shield: Gauge::full(shield),
            armor: Gauge::full(positive(fields.armor)),
            fuel: Gauge::full(positive(fields.fuel)),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn fields(shield: i16, armor: i16, fuel: i16) -> ShipFields {
        ShipFields {
            shield,
            armor,
            fuel,
            ..ShipFields::default()
        }
    }

    #[test]
    fn a_new_ships_reserves_start_full_at_its_records_values() {
        assert_eq!(
            Reserves::from_fields(fields(30, 45, 300)),
            Reserves {
                shield: Gauge {
                    now: 30.0,
                    max: 30.0
                },
                armor: Gauge {
                    now: 45.0,
                    max: 45.0
                },
                fuel: Gauge {
                    now: 300.0,
                    max: 300.0
                },
            }
        );
    }

    #[test]
    fn a_negative_shield_is_five_times_its_absolute_value() {
        assert_eq!(
            Reserves::from_fields(fields(-20, 1, 1)).shield,
            Gauge::full(100.0)
        );
        assert_eq!(
            Reserves::from_fields(fields(i16::MIN, 1, 1)).shield,
            Gauge::full(163_840.0)
        );
        assert_eq!(
            Reserves::from_fields(fields(0, 1, 1)).shield,
            Gauge::full(0.0)
        );
    }

    #[test]
    fn negative_armour_or_fuel_is_none() {
        let reserves = Reserves::from_fields(fields(10, -5, -100));
        assert_eq!(reserves.armor, Gauge::full(0.0));
        assert_eq!(reserves.fuel, Gauge::full(0.0));
        assert_eq!(reserves.shield, Gauge::full(10.0));
    }

    #[test]
    fn a_gauges_fraction_is_how_full_it_is_clamped() {
        let at = |now, max| Gauge { now, max }.fraction();
        assert_eq!(at(0.0, 300.0), 0.0);
        assert_eq!(at(150.0, 300.0), 0.5);
        assert_eq!(at(300.0, 300.0), 1.0);
        assert_eq!(at(450.0, 300.0), 1.0, "over-full");
        assert_eq!(at(-10.0, 300.0), 0.0, "below empty");
        assert_eq!(at(5.0, 0.0), 0.0, "holds nothing");
        assert_eq!(at(5.0, -1.0), 0.0, "holds less than nothing");
        assert_eq!(Gauge::full(7.0), Gauge { now: 7.0, max: 7.0 });
    }
}
