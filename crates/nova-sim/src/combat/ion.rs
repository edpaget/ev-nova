//! Ionization: the charge and colour a ship gathers from the shots,
//! blasts and beams of ionizing weapons, as `_IonizeShip`
//! (@0x8409-0x84d1) adds them.
//!
//! **A dose** ([`Ion::take`]) is a weapon's `Ionization`
//! ([`WeaponSpec::ionization`]) and its `IonizeColor`
//! ([`WeaponSpec::ionize_colour`]), with a blast's centre and radius when
//! it is a blast's:
//! - A direct hit or a beam tick (no point, @0x8421-0x8427) adds the
//!   whole `Ionization`.
//! - A blast measures the squared distance from its centre to the ship
//!   (`_XYFloatSquaredDist` @0x8433, in f32) against its radius squared,
//!   an integer squared and made a float (@0x843b-0x8447). Beyond it the
//!   ship takes nothing, no colour either (@0x844e): a **circle**, inside
//!   the blast passes' square. Within it, off the centre (@0x8450), the
//!   amount is `Ionization` × (1 − d²/R²), the ratio taken in f32 and the
//!   rest in f64, truncated toward none (@0x8459-0x8478). At the centre
//!   it is whole; only a blast of no radius could make that 0/0, and
//!   none reaches a ship ([`Blast::reaches`](super::damage::Blast::reaches)).
//! - An amount at or below none adds nothing and colours nothing
//!   (@0x847c-0x847f): a negative `Ionization`, a share truncated to none
//!   and a ship on the circle.
//! - Otherwise the charge rises by the amount (@0x8481-0x8492), with **no
//!   clamp** to the class's `IonizeMax` or any other cap: the handling
//!   side's factor divides by `IonizeMax` instead. The colour ORs in the
//!   weapon's (@0x84b5-0x84c1).
//!
//! The original takes the weapon's colour only when the ship has none at
//! an 8-bit display depth (@0x8499-0x84b3); the engine has no such depth,
//! so it always ORs. The amount is kept whole as the record's i16 widened,
//! where the original tests its low 16 bits.
//!
//! **The callers** (all five): `_HandleBeams` on every tick a beam hits a
//! ship (@0x310ce, no point), `_HandleShipHit`'s direct hit (@0x36b27, no
//! point; the weapon read raw, so a shot's decay never lessens it) and its
//! blast (@0x36ef2, the blast's centre), `_HandleShot`'s expiry blast
//! (@0x3612d, the shot's position) and `_HandleAsteroidHit`'s ship blast
//! (@0x368b7, asteroids phase 2's). A missile lock (@0x35855), the death
//! shockwave (@0x2da89), sabotage and `_HandlePlayer`'s own `_DamageShip`
//! calls never ionize. `_IonizeShip` plays no sound.
//!
//! What the charge does (deionization, the slowdown, the fire lockout and
//! the tint) is ship-handling-fidelity phase 11's. Nothing here is saved.

use super::weapon::WeaponSpec;
use crate::geometry::Vec2;

/// A ship's ionization (the original's +0x5c and +0xb0): none as it
/// spawns.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ion {
    /// Its ion charge, which a dose only ever raises, uncapped.
    pub charge: f32,
    /// Its ion colour, `00RRGGBB`: every colour it has been ionized with,
    /// OR-ed together.
    pub colour: u32,
}

/// What a hit that landed ionizes its ship by: `_IonizeShip`'s weapon and
/// point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dose {
    /// Its weapon's `Ionization`, raw ([`WeaponSpec::ionization`]).
    pub ionization: i32,
    /// Its weapon's `IonizeColor` ([`WeaponSpec::ionize_colour`]).
    pub colour: u32,
    /// The blast it is part of, if it is a blast's; none for a direct hit
    /// or a beam tick.
    pub blast: Option<Burst>,
}

/// A blast's centre and reach, for a [`Dose`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Burst {
    /// Its centre.
    pub centre: Vec2,
    /// Its `BlastRadius`, in whole pixels.
    pub radius: f32,
}

impl Dose {
    /// `weapon`'s dose, of the blast `blast` if it is a blast's.
    #[must_use]
    pub fn of(weapon: &WeaponSpec, blast: Option<Burst>) -> Self {
        Self {
            ionization: weapon.ionization,
            colour: weapon.ionize_colour,
            blast,
        }
    }
}

impl Ion {
    /// Takes `dose` on a ship at `at` (see the module docs).
    pub fn take(&mut self, dose: Dose, at: Vec2) {
        let amount = match dose.blast {
            None => dose.ionization,
            Some(Burst { centre, radius }) => {
                let off = at - centre;
                let distance = off.x * off.x + off.y * off.y;
                let reach = radius * radius;
                if distance > reach {
                    return;
                }
                if distance > 0.0 {
                    let share = 1.0 - f64::from(distance / reach);
                    // Truncated toward none, as `cvttsd2si` does.
                    #[allow(clippy::cast_possible_truncation)]
                    let amount = (f64::from(dose.ionization) * share) as i32;
                    amount
                } else {
                    dose.ionization
                }
            }
        };
        if amount <= 0 {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let amount = amount as f32;
        self.charge += amount;
        self.colour |= dose.colour;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const AT: Vec2 = Vec2::new(300.0, -200.0);

    fn direct(ionization: i32, colour: u32) -> Dose {
        Dose {
            ionization,
            colour,
            blast: None,
        }
    }

    /// A dose of `ionization` and `colour` from a blast of `radius` whose
    /// centre is `off` from [`AT`].
    fn blast(ionization: i32, colour: u32, radius: f32, off: Vec2) -> Dose {
        Dose {
            ionization,
            colour,
            blast: Some(Burst {
                centre: AT - off,
                radius,
            }),
        }
    }

    fn dosed(doses: &[Dose]) -> Ion {
        let mut ion = Ion::default();
        for &dose in doses {
            ion.take(dose, AT);
        }
        ion
    }

    #[test]
    fn a_direct_dose_adds_the_whole_ionization_and_doses_add_up_uncapped() {
        assert_eq!(dosed(&[direct(100, 0)]).charge, 100.0);
        assert_eq!(
            dosed(&[direct(100, 0), direct(100, 0), direct(100, 0)]).charge,
            300.0
        );
    }

    #[test]
    fn a_blast_at_half_its_radius_adds_three_quarters() {
        let ion = dosed(&[blast(100, 0, 100.0, Vec2::new(50.0, 0.0))]);
        assert_eq!(ion.charge, 75.0);
    }

    #[test]
    fn a_blasts_share_is_truncated_not_rounded() {
        // d² 242 of R² 400: 20 × 0.395 = 7.9.
        let ion = dosed(&[blast(20, 0, 20.0, Vec2::new(11.0, 11.0))]);
        assert_eq!(ion.charge, 7.0);
    }

    #[test]
    fn a_blasts_square_corner_outside_its_circle_ionizes_nothing() {
        let corner = Vec2::new(80.0, 80.0);
        let ion = dosed(&[blast(100, 0x00ff_0000, 100.0, corner)]);
        assert_eq!(ion, Ion::default());
        // Beyond the circle the share is negative, which would make a
        // negative `Ionization` add: the circle's return comes first.
        let ion = dosed(&[blast(-100, 0x00ff_0000, 100.0, corner)]);
        assert_eq!(ion, Ion::default());
    }

    #[test]
    fn a_ship_on_the_blasts_circle_takes_nothing() {
        let ion = dosed(&[blast(100, 0x00ff_0000, 100.0, Vec2::new(60.0, 80.0))]);
        assert_eq!(ion, Ion::default());
    }

    #[test]
    fn no_ionization_or_a_negative_one_adds_nothing_and_colours_nothing() {
        assert_eq!(dosed(&[direct(-50, 0x00ff_0000)]), Ion::default());
        assert_eq!(dosed(&[direct(0, 0x00ff_0000)]), Ion::default());
    }

    #[test]
    fn a_blast_centred_on_the_ship_adds_the_whole_ionization_at_any_radius() {
        assert_eq!(dosed(&[blast(40, 0, 0.0, Vec2::ZERO)]).charge, 40.0);
        assert_eq!(dosed(&[blast(40, 0, 100.0, Vec2::ZERO)]).charge, 40.0);
    }

    #[test]
    fn the_colours_or_together_and_a_colourless_weapon_keeps_the_colour() {
        let ion = dosed(&[direct(1, 0x0000_ff00), direct(1, 0x0000_00ff)]);
        assert_eq!(ion.colour, 0x0000_ffff);
        let ion = dosed(&[direct(1, 0x0000_ff00), direct(5, 0)]);
        assert_eq!(
            ion,
            Ion {
                charge: 6.0,
                colour: 0x0000_ff00
            }
        );
    }

    #[test]
    fn a_dose_is_its_weapons_ionization_and_colour() {
        let weapon = WeaponSpec::new(&crate::catalog::WeaponRecord {
            ionization: -7,
            ionize_color: 0x00d4_ffff,
            ..crate::testkit::weapon(128)
        });
        let burst = Burst {
            centre: AT,
            radius: 30.0,
        };
        assert_eq!(
            Dose::of(&weapon, Some(burst)),
            Dose {
                ionization: -7,
                colour: 0x00d4_ffff,
                blast: Some(burst),
            }
        );
    }
}
