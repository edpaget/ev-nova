//! Sub-munitions: the shots a shot releases when it hits a ship or comes
//! to the end of its life.
//!
//! The values are the original's (`_SpawnShotSubmunitions`, called from
//! `_HandleShot` and `_HandleShipHit` in the `EV Nova` executable):
//!
//! - A shot releases its weapon's [`Submunitions`] on a hit, aimed at the
//!   ship it hit, and at the end of its life, aimed at its own target,
//!   unless its weapon releases none then (`Flags2` 0x0020). It releases
//!   them only while its generation is below the `SubLimit`, when there is
//!   one.
//! - Each of the `SubCount` shots starts where its parent is, a
//!   generation on. A point-defence (9) parent's take its own target. With
//!   `Flags2` 0x0010 they take the nearest ship they can hit, or else the
//!   given target, and head at its bearing; otherwise they take the given
//!   target and the parent's heading.
//! - A positive `SubTheta` turns each by `θ - below(2θ + 1)`, within θ
//!   either way; a negative one spreads them in a starburst |θ| apart,
//!   centred on the heading.
//! - Each flies as a shot launched from rest along its heading would; a
//!   rocket's (6) starts with its parent's velocity instead.
//!
//! [`Submunitions`]: super::weapon::Submunitions

use super::ShipRef;
use super::aim::bearing;
use super::projectile::{Shot, Target};
use super::weapon::{Guidance, WeaponSpec};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;

/// The sub-munitions of `sub` that `parent` releases, aimed at `target`
/// (the ship it hit, or its own target) among `targets`, their spread
/// drawn on `chance` (see the module docs).
pub fn release(
    parent: &Shot,
    sub: &WeaponSpec,
    target: Option<ShipRef>,
    targets: &[Target],
    chance: &mut dyn Chance,
) -> Vec<Shot> {
    let Some(subs) = parent
        .weapon
        .submunitions
        .filter(|subs| subs.releases_at(parent.generation))
    else {
        return Vec::new();
    };
    let launcher = ShipState {
        position: parent.position,
        velocity: if sub.guidance == Guidance::Rocket {
            parent.velocity
        } else {
            Vec2::ZERO
        },
        heading: parent.heading,
    };
    let unaimed = Shot {
        generation: parent.generation + 1,
        ..Shot::launch(*sub, parent.firer, parent.fleet, &launcher, parent.heading)
    };
    let (target, heading) = if parent.weapon.guidance == Guidance::PointDefence {
        (parent.target, parent.heading)
    } else if parent.weapon.seeks_with_subs() {
        let nearest = targets
            .iter()
            .filter(|ship| unaimed.can_hit(ship))
            .min_by(|a, b| {
                let away = |ship: &Target| (ship.position - parent.position).length();
                away(a).total_cmp(&away(b))
            })
            .map(|ship| ship.ship);
        let target = nearest.or(target);
        let heading = target
            .and_then(|target| targets.iter().find(|ship| ship.ship == target))
            .map_or(parent.heading, |ship| {
                bearing(parent.position, ship.position)
            });
        (target, heading)
    } else {
        (target, parent.heading)
    };
    let theta = f32::from(subs.theta);
    let spread = theta.abs();
    (0..subs.count)
        .map(|n| {
            let offset = if subs.theta > 0 {
                let width = u32::from(subs.theta.unsigned_abs()) * 2 + 1;
                theta - chance.below(width) as f32
            } else {
                spread * n as f32 - spread * (subs.count - 1) as f32 / 2.0
            };
            Shot {
                target,
                generation: unaimed.generation,
                govt: parent.govt,
                ..Shot::launch(
                    *sub,
                    parent.firer,
                    parent.fleet,
                    &launcher,
                    heading + offset,
                )
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::hull::Condition;
    use crate::flight::{ShipState, facing};
    use crate::geometry::Vec2;
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::NpcId;

    const FIRER: ShipRef = ShipRef::Npc(NpcId(1));
    const HIT: ShipRef = ShipRef::Npc(NpcId(2));
    const NEAR: ShipRef = ShipRef::Npc(NpcId(3));

    /// A parent of `guidance` releasing `count` of weapon 148 at `theta`,
    /// up to `limit` generations, with `flags2`, at (100, 50) heading
    /// right at 5 pixels a tick down, at `target`.
    fn parent(guidance: i16, count: i16, theta: i16, limit: i16, flags2: u16) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 500,
            sub_count: count,
            sub_type: Some(WeaponId(148)),
            sub_theta: theta,
            sub_limit: limit,
            flags2,
            ..weapon(182)
        });
        Shot {
            target: Some(HIT),
            ..Shot::launch(
                spec,
                FIRER,
                FIRER,
                &ShipState {
                    position: Vec2::new(100.0, 50.0),
                    velocity: Vec2::new(0.0, 5.0),
                    heading: 90.0,
                },
                90.0,
            )
        }
    }

    /// Sub-munition 148 of `guidance`, 13 pixels a tick.
    fn sub(guidance: i16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            speed: 1300,
            count: 250,
            durability: 2,
            ..weapon(148)
        })
    }

    fn ship(ship: ShipRef, x: f32, y: f32) -> Target {
        Target {
            ship,
            fleet: ship,
            position: Vec2::new(x, y),
            velocity: Vec2::ZERO,
            radius: 10.0,
            condition: Condition::Intact,
        }
    }

    fn headings(subs: &[Shot]) -> Vec<f32> {
        subs.iter().map(|shot| shot.heading).collect()
    }

    #[test]
    fn each_sub_munition_starts_where_its_parent_is_a_generation_on() {
        let parent = Shot {
            generation: 2,
            govt: Some(crate::catalog::GovtId(140)),
            ..parent(-1, 3, 0, 0, 0)
        };
        let mut chance = Draws::of(&[]);
        let subs = release(&parent, &sub(-1), Some(HIT), &[], &mut chance);
        assert!(chance.asked.is_empty(), "no spread, nothing drawn");
        assert_eq!(subs.len(), 3);
        for shot in &subs {
            assert_eq!(shot.position, Vec2::new(100.0, 50.0));
            assert_eq!(shot.generation, 3);
            assert_eq!((shot.firer, shot.fleet), (FIRER, FIRER));
            assert_eq!(shot.govt, parent.govt, "its firer's government");
            assert_eq!(shot.weapon.id, WeaponId(148));
            assert_eq!(shot.target, Some(HIT));
            assert_eq!(shot.heading, 90.0, "the parent's heading");
            assert_eq!(shot.durability, 2.0, "its own weapon's");
            assert_eq!(shot.age, 0);
        }
    }

    #[test]
    fn a_positive_theta_turns_each_within_it_either_way() {
        let mut chance = Draws::of(&[0, 45, 90]);
        let subs = release(&parent(-1, 3, 45, 0, 0), &sub(-1), None, &[], &mut chance);
        assert_eq!(headings(&subs), [135.0, 90.0, 45.0]);
        assert_eq!(chance.asked, [91, 91, 91]);
    }

    #[test]
    fn a_negative_theta_spreads_them_in_a_starburst() {
        let mut chance = Draws::of(&[]);
        let subs = release(&parent(-1, 5, -10, 0, 0), &sub(-1), None, &[], &mut chance);
        assert_eq!(headings(&subs), [70.0, 80.0, 90.0, 100.0, 110.0]);
        let pair = release(&parent(-1, 2, -10, 0, 0), &sub(-1), None, &[], &mut chance);
        assert_eq!(headings(&pair), [85.0, 95.0]);
        assert!(chance.asked.is_empty(), "nothing drawn");
    }

    #[test]
    fn seeking_the_nearest_takes_the_nearest_ship_they_can_hit_and_heads_at_it() {
        let targets = [
            ship(HIT, 100.0, 450.0),
            ship(NEAR, 100.0, -50.0),
            ship(FIRER, 100.0, 60.0),
            Target {
                condition: Condition::Dying { ticks_left: 3 },
                ..ship(ShipRef::Npc(NpcId(4)), 110.0, 50.0)
            },
        ];
        let seeking = parent(-1, 2, 0, 0, 0x0010);
        let subs = release(&seeking, &sub(-1), Some(HIT), &targets, &mut Draws::of(&[]));
        assert_eq!(
            subs.iter().map(|s| s.target).collect::<Vec<_>>(),
            [Some(NEAR); 2]
        );
        assert_eq!(headings(&subs), [0.0, 0.0], "at its bearing");
        let alone = release(
            &seeking,
            &sub(-1),
            Some(HIT),
            &targets[..1],
            &mut Draws::of(&[]),
        );
        assert_eq!(alone[0].target, Some(HIT), "the given target, the only one");
        assert_eq!(alone[0].heading, 180.0, "at its bearing");
        // Nearer the parent at (100, 50), though nearer the centre is the
        // other.
        let measured = [ship(NEAR, 100.0, -50.0), ship(HIT, -100.0, -40.0)];
        let subs = release(&seeking, &sub(-1), None, &measured, &mut Draws::of(&[]));
        assert_eq!(subs[0].target, Some(NEAR), "from the parent");
        let nothing = release(&seeking, &sub(-1), Some(HIT), &[], &mut Draws::of(&[]));
        assert_eq!(nothing[0].target, Some(HIT), "the given target");
        assert_eq!(nothing[0].heading, 90.0, "nowhere to head");
        let unseeking = release(
            &parent(-1, 1, 0, 0, 0),
            &sub(-1),
            Some(HIT),
            &targets,
            &mut Draws::of(&[]),
        );
        assert_eq!(unseeking[0].target, Some(HIT));
        assert_eq!(unseeking[0].heading, 90.0, "the parent's heading");
    }

    #[test]
    fn a_guided_sub_munition_with_no_target_can_hit_nothing_and_takes_the_given_target() {
        let targets = [ship(NEAR, 100.0, -50.0), ship(HIT, 100.0, 450.0)];
        let seeking = parent(4, 1, 0, 0, 0x0010);
        let subs = release(&seeking, &sub(1), Some(HIT), &targets, &mut Draws::of(&[]));
        assert_eq!(subs[0].target, Some(HIT));
        assert_eq!(subs[0].heading, 180.0);
    }

    #[test]
    fn a_point_defence_parent_passes_on_its_own_target() {
        let defence = Shot {
            target: Some(NEAR),
            ..parent(9, 1, 0, 0, 0)
        };
        let subs = release(&defence, &sub(-1), Some(HIT), &[], &mut Draws::of(&[]));
        assert_eq!(subs[0].target, Some(NEAR));
    }

    #[test]
    fn rockets_keep_their_parents_velocity_and_others_start_from_rest() {
        let parent = parent(-1, 1, 0, 0, 0);
        assert_eq!(parent.velocity, Vec2::new(5.0, 5.0));
        let subs = release(&parent, &sub(6), None, &[], &mut Draws::of(&[]));
        assert_eq!(subs[0].velocity, parent.velocity);
        let unguided = release(&parent, &sub(-1), None, &[], &mut Draws::of(&[]));
        assert!((unguided[0].velocity - facing(90.0) * 13.0).length() < 1e-4);
    }

    #[test]
    fn sub_munitions_are_released_only_below_the_limit_and_only_with_a_count() {
        for (generation, limit, released) in [(0, 0, 1), (5, 0, 1), (0, 2, 1), (1, 2, 1), (2, 2, 0)]
        {
            let parent = Shot {
                generation,
                ..parent(-1, 1, 0, limit, 0)
            };
            let subs = release(&parent, &sub(-1), None, &[], &mut Draws::of(&[]));
            assert_eq!(subs.len(), released, "{generation} of {limit}");
        }
        let none = release(
            &parent(-1, 0, 0, 0, 0),
            &sub(-1),
            None,
            &[],
            &mut Draws::of(&[]),
        );
        assert_eq!(none, []);
    }
}
