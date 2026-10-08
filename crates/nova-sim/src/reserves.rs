//! A ship's reserves: its shield, armour and fuel, each a gauge that
//! starts full at what the ship can hold ([`crate::stats`]).
//!
//! The Bible's rules for the record's numbers: a negative `Shield` means
//! five times its absolute value, and a negative `Armor` or `Fuel` means
//! none.

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

/// One of a ship's [`Reserves`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reserve {
    /// Shield points.
    Shield,
    /// Armour points.
    Armor,
    /// Fuel.
    Fuel,
}

/// How many shield points a negative `Shield` is per unit of its absolute
/// value (the Bible).
pub const NEGATIVE_SHIELD_FACTOR: i64 = 5;

/// The shield points a `shïp`'s `Shield` gives: a negative one is
/// [`NEGATIVE_SHIELD_FACTOR`] times its absolute value.
#[must_use]
pub(crate) fn shield_points(shield: i16) -> i64 {
    let shield = i64::from(shield);
    if shield.is_negative() {
        -shield * NEGATIVE_SHIELD_FACTOR
    } else {
        shield
    }
}

impl Reserves {
    /// Reserves holding `shield`, `armor` and `fuel`, full.
    /// [`ShipStats`](crate::stats::ShipStats) gives a ship's.
    #[must_use]
    pub fn full(shield: f32, armor: f32, fuel: f32) -> Self {
        Self {
            shield: Gauge::full(shield),
            armor: Gauge::full(armor),
            fuel: Gauge::full(fuel),
        }
    }

    /// The gauge of `reserve`.
    #[must_use]
    pub fn get(&self, reserve: Reserve) -> Gauge {
        match reserve {
            Reserve::Shield => self.shield,
            Reserve::Armor => self.armor,
            Reserve::Fuel => self.fuel,
        }
    }

    /// The gauge of `reserve`, to change.
    pub(crate) fn get_mut(&mut self, reserve: Reserve) -> &mut Gauge {
        match reserve {
            Reserve::Shield => &mut self.shield,
            Reserve::Armor => &mut self.armor,
            Reserve::Fuel => &mut self.fuel,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn full_reserves_hold_all_they_can() {
        assert_eq!(
            Reserves::full(30.0, 45.0, 300.0),
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
        assert_eq!(shield_points(-20), 100);
        assert_eq!(shield_points(-1), 5);
        assert_eq!(shield_points(i16::MIN), 163_840);
        assert_eq!(shield_points(0), 0);
        assert_eq!(shield_points(30), 30);
        assert_eq!(NEGATIVE_SHIELD_FACTOR, 5);
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

    #[test]
    fn each_reserve_names_its_own_gauge() {
        let mut reserves = Reserves::full(1.0, 2.0, 3.0);
        assert_eq!(reserves.get(Reserve::Shield), Gauge::full(1.0));
        assert_eq!(reserves.get(Reserve::Armor), Gauge::full(2.0));
        assert_eq!(reserves.get(Reserve::Fuel), Gauge::full(3.0));
        reserves.get_mut(Reserve::Shield).now = 0.5;
        reserves.get_mut(Reserve::Armor).now = 1.5;
        reserves.get_mut(Reserve::Fuel).now = 2.5;
        assert_eq!(
            [reserves.shield.now, reserves.armor.now, reserves.fuel.now],
            [0.5, 1.5, 2.5]
        );
    }
}
