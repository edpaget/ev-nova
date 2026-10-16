//! How an NPC fires its pick: the decisions of `_FireAIShipWeapon`
//! (@0x8873d-0x89269 in the `EV Nova` executable), which `_HandleShip`
//! calls once the AI has raised the ship's fire flag (@0x34fcb-0x3501f).
//!
//! [`Armament::fire_pick`](super::armament::Armament::fire_pick) runs the
//! call, and [`Combat`](super::Combat) wires it into the fight; this
//! module holds its decisions as plain functions:
//!
//! - how many copies fire in a call ([`copies`], `_WeaponMaxSimultShots`);
//! - whether each copy spawns, and which way ([`spawn`]): turrets only
//!   inside the box of their reach, the full turrets clear of their blind
//!   spots and the quadrant turrets within their arc, a front-quadrant
//!   turret with no target ahead, homing only with a target, guns,
//!   rockets and beams always;
//! - the reload it sets ([`reload`]);
//! - and its recoil ([`recoil`], `_AdjustedAccel`).
//!
//! A copy counts as spawned whatever `_SpawnShot` or `_SpawnBeam`
//! returned (@0x889e2, @0x88cd0): under the rulebook's
//! [`RuleKey::CombatCaps`](crate::RuleKey::CombatCaps) one finding no free
//! slot still spends, sounds, recoils and reloads, and nothing flies
//! ([`Combat`](super::Combat)'s `launch`).
//!
//! Every copy that has its ammunition spends, spawned or not, by the
//! engine (the rulebook's
//! [`RuleKey::AiSpendWithoutShot`](crate::RuleKey::AiSpendWithoutShot)).
//! Nothing spawned, nothing else happens: no sound, reload, burst,
//! recoil, exclusive hold or last gun. The weapon's sound plays once a
//! call, at priority 4 ([`AI_FIRE_PRIORITY`](super::AI_FIRE_PRIORITY)).
//!
//! # Not modelled
//!
//! - Fold before fire: a ship of shän `Flags` 0x0002 and 0x0080 not at its
//!   firing pose unfolds and fires nothing (@0x88757-0x88794), and every
//!   call stamps the time it fired (+0xa8, @0x8874c) for the 45-tick
//!   refold: rdm phase ship-handling-fidelity/phase-10-ship-drawing, which
//!   models folding. Its hook is the top of `Armament::fire_pick`.
//! - The frozen game (@0x88799), set only for the priming frame on
//!   entering the game, which the engine doesn't have; and the nag ship
//!   (@0x88d40), which the engine doesn't have either.
//! - The rating stretch of the reload (@0x88fb3-0x8901b): rdm phase
//!   cloaking-fidelity/phase-6-rating-reload-stretch, between the reload
//!   and the burst in `Armament::fire_pick`.
//! - The weapon glow (`Flags2` 0x0200, +0xc8e8 = 32.0, @0x88ddd): rdm
//!   phase weapons-fidelity/phase-14, from the `Fired` it raises.
//! - Playing the sound by its priority, and a looped weapon (`Flags`
//!   0x0010) skipped while its sound plays anywhere (@0x88e00): rdm phase
//!   audio-fidelity/phase-15-weapon-sounds-by-priority.
//! - `_SpawnShot`'s exit points and its turret miss and launch speed:
//!   weapons-fidelity phases 6 and 10.
//! - The whole degrees and pixels: the box's reach (@0x888bf, @0x88a24),
//!   the heading (@0x88af7, @0x88b13, @0x88f46) and `_CalcInterceptAngle`'s
//!   table, as `ai::fire` leaves them; and the 16-bit cap on the copies.

use super::aim::{
    Aim, FRONT_ARC, bearing, blind, led, on_bearing, quadrant_off, quadrant_reference,
};
use super::armament::{GUN_RANGE_MARGIN, Rounds};
use super::hull::HullSpec;
use super::projectile::Target;
use super::weapon::{Ammo, FUEL_AMMO, Guidance, SIMULTANEOUS, WeaponSpec};
use crate::bay::launch_velocity;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;

/// How many copies of `spec` an NPC carrying `count` of it fires in one
/// call (`_WeaponMaxSimultShots` @0x8350-0x8408): one, unless its copies
/// fire together ([`SIMULTANEOUS`]); then its `count`, and for a weapon
/// that spends only at the end of a burst, at most the rounds of its own
/// slot (guidance 99) or of its `AmmoType`'s weapon (0-255, the
/// player-style slot even for an NPC, @0x8397-0x83ab), or the shots its
/// fuel pays for (`AmmoType` -1001 and below, none with no fuel,
/// @0x83c2-0x8401). `AmmoType` -1000 and the rest leave the count.
#[must_use]
pub fn copies(spec: &WeaponSpec, count: u32, rounds: &dyn Rounds, fuel: Gauge) -> u32 {
    if spec.flags & SIMULTANEOUS == 0 {
        return 1;
    }
    if !spec.ammo_at_burst_end() {
        return count;
    }
    if spec.guidance_99() {
        return count.min(rounds.held(spec.id));
    }
    match spec.ammo {
        Ammo::Rounds(ammo) => count.min(rounds.held(ammo)),
        Ammo::Fuel(cost) if spec.ammo_type < FUEL_AMMO => {
            if fuel.now > 0.0 {
                // A whole number of shots, at most `count`.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let paid = (f64::from(fuel.now) / f64::from(cost)).trunc() as u32;
                count.min(paid)
            } else {
                0
            }
        }
        _ => count,
    }
}

/// What a copy reaches on each axis, in pixels, for the box test: a
/// turreted beam its `BeamLength`, any other its range
/// ([`WeaponSpec::range`]), each with [`GUN_RANGE_MARGIN`].
fn box_reach(spec: &WeaponSpec) -> f32 {
    let reach = if spec.guidance == Guidance::TurretBeam {
        spec.beam_length
    } else {
        spec.range()
    };
    reach + GUN_RANGE_MARGIN
}

/// Whether `target` is inside the box of `spec`'s reach from `firer`:
/// nearer than it on both axes, strictly (@0x888cf-0x88943,
/// @0x88a2c-0x88a9f).
fn in_box(spec: &WeaponSpec, firer: &ShipState, target: &Target) -> bool {
    let off = target.position - firer.position;
    let reach = box_reach(spec);
    off.x.abs() < reach && off.y.abs() < reach
}

/// Which way a copy of `spec` fires from `firer`, of `hull`, at its
/// `target`, if it spawns at all (`_FireAIShipWeapon` @0x88820-0x88d3e),
/// each turret's arc measured as `arc` says
/// ([`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)):
///
/// - 3 and 4, only with a target not in a blind spot of the weapon's or
///   the ship's `Flags` by the bearing (`_TurretBlindSpot` @0x88884) and
///   inside the box of its reach: 3 along the bearing, 4 at the lead angle
///   (`_SpawnShot` @0x3e6c4, @0x3ecdc).
/// - 7 and 8 with a target, inside the box and within [`FRONT_ARC`] of
///   the heading (7) or of `(heading + 180) mod 360` (8), at the lead
///   angle; no blind spot is tested.
/// - 7 with no target, along the heading at nothing (@0x88bc7-0x88c07).
/// - 1, only with a target: along the heading, at it.
/// - -1, 0 and 6, with or without a target and with no range test: along
///   the heading, at nothing (`Combat` aims the damage at the firer's
///   target).
/// - 5, 9, 10, 99 and anything else: nothing.
#[must_use]
pub fn spawn(
    spec: &WeaponSpec,
    firer: &ShipState,
    hull: &HullSpec,
    target: Option<&Target>,
    arc: RuleSource,
) -> Option<Aim> {
    let ahead = Aim::along(firer.heading, None);
    match (spec.guidance, target) {
        (Guidance::TurretBeam | Guidance::Turret, Some(target)) => {
            let bearing = bearing(firer.position, target.position);
            let off = super::aim::angle_off(firer.heading, bearing);
            if blind(spec.flags | hull.blind_spots, off) || !in_box(spec, firer, target) {
                return None;
            }
            Some(if spec.guidance == Guidance::TurretBeam {
                on_bearing(target)
            } else {
                led(spec, target)
            })
        }
        (Guidance::FrontTurret | Guidance::RearTurret, Some(target)) => {
            let bearing = bearing(firer.position, target.position);
            let reference = quadrant_reference(spec.guidance, firer.heading);
            (in_box(spec, firer, target) && quadrant_off(reference, bearing, arc) <= FRONT_ARC)
                .then(|| led(spec, target))
        }
        (Guidance::Homing, Some(target)) => Some(Aim {
            target: Some(target.ship),
            ..ahead
        }),
        (Guidance::FrontTurret, None)
        | (Guidance::Unguided | Guidance::Beam | Guidance::Rocket, _) => Some(ahead),
        _ => None,
    }
}

/// The reload a call of `spec` that spawned `spawned` copies of the
/// `count` carried sets, before any burst (@0x88f78-0x88faf): its whole
/// `Reload` when its copies fire together ([`SIMULTANEOUS`]), else
/// `spawned` times `Reload / count`.
#[must_use]
pub fn reload(spec: &WeaponSpec, spawned: u32, count: u32) -> f32 {
    if spec.flags & SIMULTANEOUS != 0 {
        spec.reload
    } else {
        spawned as f32 * (spec.reload / count.max(1) as f32)
    }
}

/// The velocity `spec`'s recoil leaves its firer in, as it moves in
/// `state`, of `hull` and class speed `class_speed` (@0x88ee2-0x88f71):
/// `Recoil` over its mass, pushed along `(heading + 180) mod 360` up to
/// its class speed ([`launch_velocity`], `_AdjustedAccel`); none when the
/// weapon does not recoil ([`WeaponSpec::recoils`]). A massless hull takes
/// none, where the original divides by its mass of 0 and leaves the
/// velocity infinite or not a number.
#[must_use]
pub fn recoil(
    state: &ShipState,
    hull: &HullSpec,
    class_speed: f32,
    spec: &WeaponSpec,
) -> Option<Vec2> {
    if !spec.recoils() || hull.mass <= 0.0 {
        return None;
    }
    let away = (state.heading + 180.0) % 360.0;
    Some(launch_velocity(
        state.velocity,
        away,
        spec.recoil / hull.mass,
        class_speed,
    ))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{ShipId, WeaponId, WeaponRecord};
    use crate::combat::ShipRef;
    use crate::combat::aim::{BLIND_REAR, lead};
    use crate::combat::armament::LauncherRounds;
    use crate::combat::hull::Condition;
    use crate::combat::sides::Ties;
    use crate::combat::weapon::AMMO_AT_BURST_END;
    use crate::flight::facing;
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const B: ShipRef = ShipRef::Npc(NpcId(2));

    /// A weapon of `guidance` reaching 200 pixels (10 a tick for 20), its
    /// beam 100 long.
    fn guided(guidance: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 1000,
            count: 20,
            beam_length: 100,
            ..weapon(130)
        })
    }

    /// A firer at the origin, at rest, facing up.
    fn firer() -> ShipState {
        ShipState::default()
    }

    /// Ship B at (`x`, `y`), at rest.
    fn at(x: f32, y: f32) -> Target {
        Target {
            ship: B,
            ship_type: ShipId(128),
            fleet: B,
            ties: Ties::default(),
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 0.0,
            sprite_x: 0.0,
            condition: Condition::Intact,
            miss_class: 100,
        }
    }

    fn fires(spec: &WeaponSpec, target: Option<&Target>) -> Option<Aim> {
        spawn(
            spec,
            &firer(),
            &HullSpec::default(),
            target,
            RuleSource::Engine,
        )
    }

    #[test]
    fn a_turret_fires_only_with_its_target_inside_the_box_of_its_reach() {
        // Reach 200 + 32: (231, -231) is outside the circle but inside the
        // box, strictly.
        let turret = guided(4);
        let corner = at(231.0, -231.0);
        let pointed =
            |aim: Option<Aim>| aim.map(|aim| (aim.pointing.heading_from(&firer()), aim.target));
        assert_eq!(fires(&turret, Some(&corner)), Some(led(&turret, &corner)));
        assert_eq!(
            pointed(fires(&turret, Some(&corner))),
            Some((lead(&firer(), &corner, turret.speed), Some(B)))
        );
        assert_eq!(fires(&turret, Some(&at(232.0, 0.0))), None, "on the edge");
        assert_eq!(fires(&turret, Some(&at(0.0, -232.0))), None);
        assert_eq!(
            pointed(fires(&turret, Some(&at(0.0, 231.9)))),
            Some((180.0, Some(B)))
        );
        // A turreted beam reaches its length and the margin, along the
        // bearing.
        let beam = guided(3);
        assert_eq!(
            pointed(fires(&beam, Some(&at(131.0, 0.0)))),
            Some((90.0, Some(B)))
        );
        assert_eq!(fires(&beam, Some(&at(132.0, 0.0))), None);
    }

    #[test]
    fn a_turret_holds_fire_at_a_target_in_its_blind_spot_by_the_bearing() {
        let rear_blind = WeaponSpec {
            flags: BLIND_REAR,
            ..guided(4)
        };
        assert_eq!(fires(&rear_blind, Some(&at(0.0, 100.0))), None, "astern");
        assert!(fires(&rear_blind, Some(&at(100.0, 0.0))).is_some(), "abeam");
        // The ship's own blind spots count too.
        let hull = HullSpec {
            blind_spots: BLIND_REAR,
            ..HullSpec::default()
        };
        let beam = guided(3);
        let behind = at(0.0, 100.0);
        assert_eq!(
            spawn(&beam, &firer(), &hull, Some(&behind), RuleSource::Engine),
            None
        );
        // A quadrant turret takes no blind spot here.
        let rear = WeaponSpec {
            flags: BLIND_REAR,
            ..guided(8)
        };
        assert!(fires(&rear, Some(&behind)).is_some());
    }

    #[test]
    fn a_quadrant_turret_spawns_only_within_45_of_its_reference() {
        let front = guided(7);
        assert!(fires(&front, Some(&at(100.0, -100.0))).is_some(), "45 off");
        assert_eq!(fires(&front, Some(&at(100.0, -99.0))), None, "past 45");
        assert_eq!(fires(&front, Some(&at(0.0, 100.0))), None, "astern");
        let rear = guided(8);
        assert!(fires(&rear, Some(&at(0.0, 100.0))).is_some());
        assert_eq!(fires(&rear, Some(&at(0.0, -100.0))), None);
        // Facing up, the front's reference is 0: a target 10 degrees to
        // its left bears 350, which the engine holds as 350 off.
        let left = at(-17.632_7, -100.0);
        assert_eq!(fires(&front, Some(&left)), None, "by the engine");
        let folded = spawn(
            &front,
            &firer(),
            &HullSpec::default(),
            Some(&left),
            RuleSource::Bible,
        );
        assert!(folded.is_some(), "folded");
        // Facing down, the rear's reference is 0 too.
        let down = ShipState {
            heading: 180.0,
            ..firer()
        };
        assert_eq!(
            spawn(
                &rear,
                &down,
                &HullSpec::default(),
                Some(&left),
                RuleSource::Engine
            ),
            None
        );
        assert!(
            spawn(
                &rear,
                &down,
                &HullSpec::default(),
                Some(&left),
                RuleSource::Bible
            )
            .is_some()
        );
        // And the box: 7 and 8 reach their range and the margin.
        assert_eq!(fires(&front, Some(&at(0.0, -232.0))), None);
        let led = fires(&front, Some(&at(0.0, -231.0))).expect("in the box");
        assert_eq!(led.target, Some(B));
    }

    #[test]
    fn with_no_target_only_a_front_quadrant_turret_fires_and_it_fires_ahead_at_nothing() {
        assert_eq!(fires(&guided(7), None), Some(Aim::along(0.0, None)));
        for guidance in [3, 4, 8, 1] {
            assert_eq!(fires(&guided(guidance), None), None, "{guidance}");
        }
    }

    #[test]
    fn guns_rockets_and_beams_fire_with_no_range_test() {
        let far = at(5000.0, 5000.0);
        for guidance in [-1, 0, 6] {
            let ahead = Aim::along(0.0, None);
            assert_eq!(
                fires(&guided(guidance), Some(&far)),
                Some(ahead),
                "{guidance}"
            );
            assert_eq!(fires(&guided(guidance), None), Some(ahead), "{guidance}");
        }
        assert_eq!(
            fires(&guided(1), Some(&far)),
            Some(Aim::along(0.0, Some(B))),
            "homing: along the heading at its target"
        );
    }

    #[test]
    fn bombs_bays_and_point_defence_spawn_nothing() {
        let bay = WeaponSpec::new(&WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            ..weapon(150)
        });
        let near = at(0.0, -50.0);
        for spec in [
            guided(5),
            guided(9),
            guided(10),
            guided(99),
            guided(11),
            bay,
        ] {
            assert_eq!(fires(&spec, Some(&near)), None, "{:?}", spec.guidance);
        }
    }

    #[test]
    fn simultaneous_copies_are_the_count_capped_by_the_ammunition_at_burst_end() {
        let fuel = |now: f32| Gauge { now, max: 100.0 };
        let rounds = LauncherRounds::from([(WeaponId(130), 2), (WeaponId(140), 5)]);
        let plain = guided(-1);
        assert_eq!(copies(&plain, 3, &rounds, fuel(100.0)), 1, "one at a time");
        let together = WeaponSpec {
            flags: SIMULTANEOUS,
            ..plain
        };
        assert_eq!(copies(&together, 3, &rounds, fuel(100.0)), 3, "the count");
        let at_end = |ammo_type: i16, guidance: i16| {
            WeaponSpec::new(&WeaponRecord {
                flags: SIMULTANEOUS,
                flags3: AMMO_AT_BURST_END,
                ammo_type,
                guidance,
                ..weapon(130)
            })
        };
        // Guidance 99: its own slot.
        assert_eq!(copies(&at_end(-1, 99), 3, &rounds, fuel(0.0)), 2);
        // `AmmoType` 0-255: the `AmmoType`'s slot, wëap 128 + 12.
        assert_eq!(copies(&at_end(12, -1), 9, &rounds, fuel(0.0)), 5);
        assert_eq!(copies(&at_end(13, -1), 9, &rounds, fuel(0.0)), 0);
        // Fuel at -1005, half a unit a shot: 1.2 units fire 2.
        assert_eq!(copies(&at_end(-1005, -1), 9, &rounds, fuel(1.2)), 2);
        assert_eq!(copies(&at_end(-1005, -1), 1, &rounds, fuel(1.2)), 1);
        assert_eq!(copies(&at_end(-1005, -1), 9, &rounds, fuel(0.0)), 0, "none");
        // -1000, and the other `AmmoType`s, leave the count.
        for ammo_type in [-1000, -1, -999, 300] {
            assert_eq!(copies(&at_end(ammo_type, -1), 4, &rounds, fuel(0.0)), 4);
        }
    }

    #[test]
    fn a_call_reloads_by_its_spawned_share_of_reload_or_the_whole_reload_for_simultaneous() {
        let spec = WeaponSpec {
            reload: 30.0,
            ..guided(-1)
        };
        assert_eq!(reload(&spec, 1, 3), 10.0);
        assert_eq!(reload(&spec, 2, 3), 20.0);
        let together = WeaponSpec {
            flags: SIMULTANEOUS,
            ..spec
        };
        assert_eq!(reload(&together, 2, 3), 30.0);
    }

    #[test]
    fn recoil_pushes_the_firer_away_from_its_heading_by_recoil_over_mass_up_to_its_class_speed() {
        let hull = HullSpec {
            mass: 50.0,
            ..HullSpec::default()
        };
        let kicks = |raw: i16| WeaponSpec {
            recoil: f32::from(raw),
            ..guided(-1)
        };
        // Facing up: pushed down, 100 / 50 = 2 a tick.
        let pushed = recoil(&firer(), &hull, 10.0, &kicks(100)).expect("recoils");
        assert!((pushed - facing(180.0) * 2.0).length() < 1e-5, "{pushed:?}");
        // At its class speed already, it gains no more that way.
        let fast = ShipState {
            velocity: Vec2::new(0.0, 10.0),
            ..firer()
        };
        let held = recoil(&fast, &hull, 10.0, &kicks(100)).expect("recoils");
        assert!((held - fast.velocity).length() < 1e-5, "{held:?}");
        // A negative recoil pushes it forward, whatever its speed.
        let forward = recoil(&fast, &hull, 10.0, &kicks(-100)).expect("recoils");
        assert!(
            (forward - Vec2::new(0.0, 8.0)).length() < 1e-5,
            "{forward:?}"
        );
        assert_eq!(recoil(&firer(), &hull, 10.0, &kicks(0)), None);
        assert_eq!(recoil(&firer(), &hull, 10.0, &kicks(-1)), None);
        let massless = HullSpec::default();
        assert_eq!(
            recoil(&firer(), &massless, 10.0, &kicks(100)),
            None,
            "mass 0"
        );
    }
}
