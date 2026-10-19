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
//! **What the charge does.** The ship sheds it each frame and flies, turns
//! and thrusts slower while it lasts (`nova_sim_base::ionization`, run by
//! the session for the player and by the traffic for each NPC). A
//! `Seeker` 0x0020 weapon is held while it is ionized ([`IonLockout`]).
//! The ship glows with it ([`ion_tint`](super::ion_tint)), and the
//! display, which runs for every ship in the system each frame, clears the
//! colour once the charge is gone (`_HandleShipDisplay` @0x2bf8a), so a
//! colour outlives its charge by a frame at most. Nothing here is saved.

use super::weapon::{Guidance, WeaponSpec};
use crate::flight::Flyer;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

/// The `wëap` `Seeker` bit of a weapon that can't fire while its ship is
/// ionized (in-memory +0x26 bit 0x20, `shrl $5` @0x338e3).
pub const CANT_FIRE_IONIZED: u16 = 0x0020;

/// Which weapons an ionized ship can't fire, as
/// [`RuleKey::IonFireLockout`](crate::RuleKey::IonFireLockout) reads: the
/// reload loops' hold (`_HandlePlayer` @0x6ae73-0x6ae9c, `_HandleShip`
/// @0x338d6-0x338f0), which sets a held weapon's reload to 1.0 after its
/// countdown ([`Armament::reload_holding`](super::armament::Armament::reload_holding)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IonLockout(RuleSource);

/// How long a held weapon's reload is kept, in ticks (the float 1.0
/// written @0x338f0 and @0x6ae9c).
pub const HELD_RELOAD: f32 = 1.0;

impl From<RuleSource> for IonLockout {
    fn from(source: RuleSource) -> Self {
        Self(source)
    }
}

impl IonLockout {
    /// Whether `spec`, carried by a ship flown by `flyer` of ionization
    /// factor `factor`, is held this tick. By the engine, any weapon of
    /// [`CANT_FIRE_IONIZED`], whatever its guidance: the player's while
    /// the factor is above 0, an NPC's while trunc(factor) ≥ 1, which is a
    /// factor of 1 or more. By the other reading, only a guided one, and
    /// only at a factor of 1 or more, either flyer.
    #[must_use]
    pub fn holds(self, spec: &WeaponSpec, flyer: Flyer, factor: f32) -> bool {
        if spec.seeker & CANT_FIRE_IONIZED == 0 {
            return false;
        }
        match self.0 {
            RuleSource::Engine => match flyer {
                Flyer::Player => factor > 0.0,
                Flyer::Npc => factor >= 1.0,
            },
            RuleSource::Bible => spec.guidance == Guidance::Homing && factor >= 1.0,
        }
    }
}

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

    /// A weapon of `guidance` and `Seeker` `seeker`.
    fn seeking(guidance: i16, seeker: u16) -> WeaponSpec {
        WeaponSpec::new(&crate::catalog::WeaponRecord {
            guidance,
            seeker,
            ..nova_sim_catalog::fixture::weapon(128)
        })
    }

    const ENGINE: IonLockout = IonLockout(RuleSource::Engine);
    const BIBLE: IonLockout = IonLockout(RuleSource::Bible);

    #[test]
    fn by_the_engine_the_player_is_held_at_any_ionization() {
        let gun = seeking(-1, CANT_FIRE_IONIZED);
        assert!(ENGINE.holds(&gun, Flyer::Player, 0.01));
        assert!(ENGINE.holds(&gun, Flyer::Player, 3.0));
        assert!(!ENGINE.holds(&gun, Flyer::Player, 0.0));
        assert!(!ENGINE.holds(&gun, Flyer::Player, -0.1));
    }

    #[test]
    fn by_the_engine_an_npc_is_held_from_a_factor_of_1() {
        let gun = seeking(-1, CANT_FIRE_IONIZED);
        assert!(!ENGINE.holds(&gun, Flyer::Npc, 0.99));
        assert!(ENGINE.holds(&gun, Flyer::Npc, 1.0));
        assert!(ENGINE.holds(&gun, Flyer::Npc, 2.5));
    }

    #[test]
    fn by_the_engine_any_guidance_is_held() {
        for guidance in [-1, 0, 1, 3, 4, 5, 6, 7, 8, 9, 10] {
            let weapon = seeking(guidance, CANT_FIRE_IONIZED | 0x0001);
            assert!(ENGINE.holds(&weapon, Flyer::Npc, 1.0), "{guidance}");
        }
    }

    #[test]
    fn by_the_bible_only_a_guided_weapon_is_held_and_only_from_a_factor_of_1() {
        let missile = seeking(1, CANT_FIRE_IONIZED);
        let gun = seeking(-1, CANT_FIRE_IONIZED);
        for flyer in [Flyer::Player, Flyer::Npc] {
            assert!(BIBLE.holds(&missile, flyer, 1.0), "{flyer:?}");
            assert!(!BIBLE.holds(&missile, flyer, 0.99), "{flyer:?}");
            assert!(!BIBLE.holds(&gun, flyer, 5.0), "{flyer:?}");
        }
    }

    #[test]
    fn a_weapon_without_the_bit_is_never_held() {
        let missile = seeking(1, !CANT_FIRE_IONIZED);
        for lockout in [ENGINE, BIBLE] {
            for flyer in [Flyer::Player, Flyer::Npc] {
                assert!(!lockout.holds(&missile, flyer, 5.0));
            }
        }
        assert_eq!(CANT_FIRE_IONIZED, 0x0020);
    }

    #[test]
    fn a_held_mount_sits_at_1_after_its_countdown_and_the_others_count_down() {
        use super::super::armament::Armament;
        let record = |id, seeker| crate::catalog::WeaponRecord {
            reload: 5,
            seeker,
            ..nova_sim_catalog::fixture::weapon(id)
        };
        let held = WeaponSpec::new(&record(128, CANT_FIRE_IONIZED));
        let free = WeaponSpec::new(&record(129, 0));
        let mut armament = Armament::new([(held, 1), (free, 1)]);
        armament.hold(held.id);
        armament.hold(free.id);
        let reloads = |armament: &Armament| -> Vec<f32> {
            armament.mounts().iter().map(|mount| mount.reload).collect()
        };
        armament.reload_holding(|spec| ENGINE.holds(spec, Flyer::Npc, 1.0));
        assert_eq!(reloads(&armament), [HELD_RELOAD, 4.0]);
        armament.reload_holding(|spec| ENGINE.holds(spec, Flyer::Npc, 1.0));
        assert_eq!(reloads(&armament), [1.0, 3.0]);
        // Released: the countdown runs it to none, ready to fire.
        armament.reload_holding(|spec| ENGINE.holds(spec, Flyer::Npc, 0.99));
        assert_eq!(reloads(&armament), [0.0, 2.0]);
        armament.reload();
        assert_eq!(reloads(&armament), [0.0, 1.0]);
        assert_eq!(HELD_RELOAD, 1.0);
    }

    #[test]
    fn the_lockout_reads_its_source() {
        assert_eq!(IonLockout::from(RuleSource::Bible), BIBLE);
        assert_eq!(IonLockout::default(), ENGINE);
    }

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
            ..nova_sim_catalog::fixture::weapon(128)
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
