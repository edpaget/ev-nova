//! Point defence: a ship's point-defence weapons (guidance 9 and 10)
//! shooting down the missiles fired at it, with no target needed.
//!
//! The values are the original's (`_HandleShipPointDefense`,
//! `_PointDefenseCollisionHandler` and `_HandleBeams` in the `EV Nova`
//! executable):
//!
//! - Every tick, each intact ship fires its first ready point-defence
//!   weapon it can pay for, by weapon ID, at the nearest candidate missile
//!   ([`choose`]), and nothing when there is none.
//! - A candidate is a live homing shot that has not lost its target, that
//!   point defence can target (no `Flags` 0x0080), that is fired at this
//!   ship or at its fleet's lead (an escort defends its lead), that is
//!   within the weapon's [`engagement_range`] and not in a blind spot (see
//!   [`aim`](super::aim)), and whose firer the [`PointDefenceRule`] says
//!   is hostile. The nearest wins; of two as near, the one fired first.
//!   A point-defence shot meets only a missile that point defence could
//!   have chosen but for its target, range and arc ([`engageable`]), so it
//!   flies through one that has lost its target.
//! - A point-defence turret (9) fires a shot at the missile's bearing, no
//!   lead, plus its inaccuracy; a point-defence beam (10) is held on the
//!   missile, with no inaccuracy.
//! - Each hit takes [`pd_damage`], `MassDmg` plus half the `EnergyDmg`
//!   (truncated), off the missile's durability while that is above none;
//!   the hit after that destroys it ([`Shot::take_pd_hit`]). A point
//!   defence shot is spent on the missile it meets; a beam hits the
//!   missile it is held on every tick.
//! - The original has no hostility check: "fired at me or my lead" is the
//!   whole rule. The default [`Allegiance`] adds governments' relations: a
//!   missile is hostile unless its firer is in the defender's fleet, or
//!   both have governments that are allies. The player has none, so its
//!   point defence engages any NPC's missile at it, and an NPC's the
//!   player's, while police leave an allied trader's stray missile be.
//! - The original tests a point-defence shot against a missile's sprite;
//!   the simulation has no sprites, so a shot meets a missile within
//!   [`INTERCEPT_RADIUS`] of it, a placeholder.

use std::fmt::Debug;

use super::ShipRef;
use super::aim::{angle_off, bearing, blind};
use super::hull::HullSpec;
use super::projectile::Shot;
use super::weapon::{Guidance, WeaponSpec};
use crate::catalog::GovtId;
use crate::flight::ShipState;
use crate::govt::Governments;

/// How near, in pixels, a point-defence shot must pass a missile to meet
/// it: a placeholder for the original's sprite test.
pub const INTERCEPT_RADIUS: f32 = 10.0;
/// A point-defence turret's engagement range, per pixel of its range.
pub const PD_RANGE_FACTOR: f32 = 1.5;

/// A ship, its fleet (the lead it escorts, or itself) and its government.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Side {
    /// The ship.
    pub ship: ShipRef,
    /// Its fleet.
    pub fleet: ShipRef,
    /// Its government, or `None` for an independent ship or the player.
    pub govt: Option<GovtId>,
}

impl Side {
    /// The side that fired `shot`.
    #[must_use]
    pub fn of(shot: &Shot) -> Self {
        Self {
            ship: shot.firer,
            fleet: shot.fleet,
            govt: shot.govt,
        }
    }
}

/// Which missiles point defence engages, by who fired them.
pub trait PointDefenceRule: Debug {
    /// Whether a missile fired by `firer` is hostile to `defender`, with
    /// the relations between governments in `govts`.
    fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool;
}

/// Nova's rule (see the module docs): a missile is hostile unless its
/// firer is in the defender's fleet, or both have governments that are
/// allies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Allegiance;

impl PointDefenceRule for Allegiance {
    fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
        let allied = defender.govt.is_some()
            && firer.govt.is_some()
            && govts.allies(defender.govt, firer.govt);
        firer.fleet != defender.fleet && !allied
    }
}

/// How near, in pixels, a missile must be for point defence `spec` to
/// engage it: a beam's length, or a turret's range, truncated, times
/// [`PD_RANGE_FACTOR`], truncated again.
#[must_use]
pub fn engagement_range(spec: &WeaponSpec) -> f32 {
    if spec.guidance == Guidance::PointDefenceBeam {
        spec.beam_length
    } else {
        (spec.range().trunc() * PD_RANGE_FACTOR).trunc()
    }
}

/// Whether point defence on `defender`, whether a mount choosing a
/// missile or a shot passing one, can engage `missile` at all: a homing
/// shot that has not lost its target, that point defence can target (no
/// `Flags` 0x0080), and whose firer `rule` calls hostile, with the
/// relations in `govts`. A lost missile is neither chosen nor met.
#[must_use]
pub fn engageable(
    defender: Side,
    missile: &Shot,
    rule: &dyn PointDefenceRule,
    govts: &Governments,
) -> bool {
    missile.weapon.guidance == Guidance::Homing
        && !missile.lost
        && !missile.weapon.pd_immune()
        && rule.hostile(defender, Side::of(missile), govts)
}

/// The missile among `shots` that point defence `spec` on `defender`, at
/// `at` with `hull`, engages, if any, as `rule` says with the relations
/// in `govts` (see the module docs).
#[must_use]
pub fn choose<'a>(
    defender: Side,
    at: &ShipState,
    hull: &HullSpec,
    spec: &WeaponSpec,
    shots: &'a [Shot],
    rule: &dyn PointDefenceRule,
    govts: &Governments,
) -> Option<&'a Shot> {
    let range = engagement_range(spec);
    let blind_spots = spec.flags | hull.blind_spots;
    let mut best: Option<(&Shot, f32)> = None;
    for shot in shots {
        let off = shot.position - at.position;
        let distance = off.x * off.x + off.y * off.y;
        let candidate = (shot.target == Some(defender.ship) || shot.target == Some(defender.fleet))
            && distance <= range * range
            && !blind(
                blind_spots,
                angle_off(at.heading, bearing(at.position, shot.position)),
            )
            && best.is_none_or(|(_, nearest)| distance < nearest)
            && engageable(defender, shot, rule, govts);
        if candidate {
            best = Some((shot, distance));
        }
    }
    best.map(|(shot, _)| shot)
}

/// What a hit of point defence `spec` takes off a missile's durability:
/// its mass damage and half its energy damage, truncated.
#[must_use]
pub fn pd_damage(spec: &WeaponSpec) -> f32 {
    spec.mass_damage + (spec.energy_damage / 2.0).trunc()
}

impl Shot {
    /// Takes a point-defence hit of `damage`: off its durability while
    /// that is above none, and otherwise it is destroyed. Whether it was.
    pub fn take_pd_hit(&mut self, damage: f32) -> bool {
        if self.durability > 0.0 {
            self.durability -= damage;
            false
        } else {
            true
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::catalog::{GovtId, WeaponRecord};
    use crate::combat::aim::BLIND_REAR;
    use crate::combat::projectile::ShotId;
    use crate::geometry::Vec2;
    use crate::testkit::weapon;
    use crate::traffic::npc::NpcId;

    const DEFENDER: ShipRef = ShipRef::Npc(NpcId(1));
    const LEAD: ShipRef = ShipRef::Npc(NpcId(9));
    const ENEMY: ShipRef = ShipRef::Npc(NpcId(5));

    /// The Quad Light Blaster Turret: 20 pixels a tick for 12 ticks.
    fn quad(flags: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 2000,
            count: 12,
            mass_dmg: 1,
            energy_dmg: 4,
            flags,
            ..weapon(133)
        })
    }

    #[test]
    fn a_turrets_engagement_range_is_half_again_its_range_and_a_beams_its_length() {
        assert_eq!(engagement_range(&quad(0)), 360.0);
        let storm = WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 1200,
            count: 15,
            ..weapon(161)
        });
        assert_eq!(engagement_range(&storm), 270.0);
        let odd = WeaponSpec::new(&WeaponRecord {
            guidance: 9,
            speed: 1050,
            count: 3,
            ..weapon(161)
        });
        assert_eq!(engagement_range(&odd), 46.0, "31.5 to 31, then 46.5 to 46");
        let beam = WeaponSpec::new(&WeaponRecord {
            guidance: 10,
            speed: 2000,
            count: 12,
            beam_length: 150,
            ..weapon(150)
        });
        assert_eq!(engagement_range(&beam), 150.0);
        assert_eq!(PD_RANGE_FACTOR, 1.5);
    }

    /// A shot numbered `id` of a homing missile fired by `ENEMY` of
    /// government 140 at `target`, at (`x`, `y`).
    fn missile(id: u32, x: f32, y: f32, target: Option<ShipRef>) -> Shot {
        let spec = WeaponSpec::new(&WeaponRecord {
            guidance: 1,
            durability: 4,
            ..weapon(134)
        });
        Shot {
            id: ShotId(id),
            position: Vec2::new(x, y),
            target,
            govt: Some(GovtId(140)),
            ..Shot::launch(spec, ENEMY, ENEMY, &ShipState::default(), 0.0)
        }
    }

    /// Says every firer is `hostile`, recording what it was asked and
    /// the governments it was asked with.
    #[derive(Debug, Default)]
    struct Recording {
        hostile: bool,
        asked: RefCell<Vec<(Side, Side)>>,
        govts: RefCell<Vec<Governments>>,
    }

    impl PointDefenceRule for Recording {
        fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
            self.asked.borrow_mut().push((defender, firer));
            self.govts.borrow_mut().push(govts.clone());
            self.hostile
        }
    }

    const ESCORT: Side = Side {
        ship: DEFENDER,
        fleet: LEAD,
        govt: None,
    };

    /// The missile the defender, an escort of `LEAD` at the centre facing
    /// up, picks among `shots` with `quad(flags)` and a hull with
    /// `blind_spots`, every firer hostile.
    fn picked(shots: &[Shot], flags: u16, blind_spots: u16) -> Option<ShotId> {
        let hull = HullSpec {
            blind_spots,
            ..HullSpec::default()
        };
        let rule = Recording {
            hostile: true,
            ..Recording::default()
        };
        choose(
            ESCORT,
            &ShipState::default(),
            &hull,
            &quad(flags),
            shots,
            &rule,
            &Governments::default(),
        )
        .map(|shot| shot.id)
    }

    #[test]
    fn a_missile_at_the_defender_or_its_lead_is_a_candidate_and_one_at_another_ship_is_not() {
        let at = |target| [missile(1, 0.0, -100.0, target)];
        assert_eq!(picked(&at(Some(DEFENDER)), 0, 0), Some(ShotId(1)));
        assert_eq!(picked(&at(Some(LEAD)), 0, 0), Some(ShotId(1)), "its lead");
        assert_eq!(picked(&at(Some(ShipRef::Player)), 0, 0), None);
        assert_eq!(picked(&at(None), 0, 0), None, "at nothing");
        assert_eq!(picked(&[], 0, 0), None);
    }

    #[test]
    fn a_lost_untargetable_unguided_far_or_unseen_missile_is_not_a_candidate() {
        let lost = Shot {
            lost: true,
            ..missile(1, 0.0, -100.0, Some(DEFENDER))
        };
        assert_eq!(picked(&[lost], 0, 0), None, "lost");
        let mut immune = missile(1, 0.0, -100.0, Some(DEFENDER));
        immune.weapon.flags = 0x0080;
        assert_eq!(picked(&[immune], 0, 0), None, "0x0080");
        let mut unguided = missile(1, 0.0, -100.0, Some(DEFENDER));
        unguided.weapon.guidance = crate::combat::weapon::Guidance::Turret;
        assert_eq!(picked(&[unguided], 0, 0), None, "not guided");
        let at_range = missile(1, 0.0, -360.0, Some(DEFENDER));
        assert_eq!(picked(&[at_range], 0, 0), Some(ShotId(1)), "at its range");
        let beyond = missile(1, 0.0, 360.1, Some(DEFENDER));
        assert_eq!(picked(&[beyond], 0, 0), None, "beyond its range");
        let astern = [missile(1, 0.0, 100.0, Some(DEFENDER))];
        assert_eq!(
            picked(&astern, BLIND_REAR, 0),
            None,
            "the weapon's blind spot"
        );
        assert_eq!(
            picked(&astern, 0, BLIND_REAR),
            None,
            "the hull's blind spot"
        );
        assert_eq!(picked(&astern, 0, 0), Some(ShotId(1)));
        assert_eq!(
            picked(&astern, BLIND_REAR, BLIND_REAR),
            None,
            "blind both ways"
        );
    }

    #[test]
    fn a_missile_is_measured_from_where_the_defender_is() {
        let at = ShipState {
            position: Vec2::new(1000.0, 1000.0),
            ..ShipState::default()
        };
        let rule = Recording {
            hostile: true,
            ..Recording::default()
        };
        let near = [missile(1, 1000.0, 700.0, Some(DEFENDER))];
        let none = Governments::default();
        let chosen = choose(
            ESCORT,
            &at,
            &HullSpec::default(),
            &quad(0),
            &near,
            &rule,
            &none,
        );
        assert_eq!(
            chosen.map(|shot| shot.id),
            Some(ShotId(1)),
            "300 pixels off"
        );
        let far = [missile(1, 0.0, 0.0, Some(DEFENDER))];
        let chosen = choose(
            ESCORT,
            &at,
            &HullSpec::default(),
            &quad(0),
            &far,
            &rule,
            &none,
        );
        assert_eq!(chosen, None, "1414 pixels off");
    }

    #[test]
    fn the_nearest_candidate_wins_and_a_tie_goes_to_the_one_fired_first() {
        let shots = [
            missile(1, 0.0, -200.0, Some(DEFENDER)),
            missile(2, 0.0, -50.0, Some(ShipRef::Player)),
            missile(3, 100.0, 0.0, Some(LEAD)),
            missile(4, -100.0, 0.0, Some(DEFENDER)),
        ];
        assert_eq!(picked(&shots, 0, 0), Some(ShotId(3)));
        let mut reversed = shots;
        reversed.reverse();
        assert_eq!(picked(&reversed, 0, 0), Some(ShotId(4)));
    }

    #[test]
    fn the_rule_is_asked_of_each_candidate_and_one_not_hostile_is_skipped() {
        let shots = [
            missile(1, 0.0, -50.0, Some(DEFENDER)),
            Shot {
                firer: ShipRef::Player,
                fleet: ShipRef::Player,
                govt: None,
                ..missile(2, 0.0, -80.0, Some(DEFENDER))
            },
            missile(3, 0.0, -1000.0, Some(DEFENDER)),
        ];
        let peaceful = Recording::default();
        let govts = Governments::new([crate::testkit::govt(128)]);
        let chosen = choose(
            ESCORT,
            &ShipState::default(),
            &HullSpec::default(),
            &quad(0),
            &shots,
            &peaceful,
            &govts,
        );
        assert_eq!(chosen, None);
        let enemy = Side {
            ship: ENEMY,
            fleet: ENEMY,
            govt: Some(GovtId(140)),
        };
        let player = Side {
            ship: ShipRef::Player,
            fleet: ShipRef::Player,
            govt: None,
        };
        assert_eq!(
            peaceful.asked.take(),
            [(ESCORT, enemy), (ESCORT, player)],
            "not of the one out of range"
        );
        assert_eq!(
            peaceful.govts.take(),
            [govts.clone(), govts],
            "asked with them"
        );
    }

    /// Governments 128 and 129 are allies, 130 is at war with 128, and
    /// 131 is neutral.
    fn relations() -> Governments {
        use crate::catalog::GovtRecord;
        use crate::testkit::govt;
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..govt(130)
            },
            govt(131),
        ])
    }

    fn side(ship: ShipRef, fleet: ShipRef, govt: Option<i16>) -> Side {
        Side {
            ship,
            fleet,
            govt: govt.map(GovtId),
        }
    }

    #[test]
    fn allegiance_spares_the_defenders_own_fleet_whatever_the_governments() {
        let govts = relations();
        let defender = side(DEFENDER, LEAD, Some(128));
        for govt in [Some(128), Some(130), None] {
            assert!(
                !Allegiance.hostile(defender, side(LEAD, LEAD, govt), &govts),
                "its lead"
            );
            assert!(
                !Allegiance.hostile(defender, side(ENEMY, LEAD, govt), &govts),
                "another escort"
            );
        }
        assert!(!Allegiance.hostile(defender, defender, &govts), "itself");
    }

    #[test]
    fn allegiance_spares_an_allied_government_and_its_own() {
        let govts = relations();
        let defender = side(DEFENDER, DEFENDER, Some(128));
        assert!(!Allegiance.hostile(defender, side(ENEMY, ENEMY, Some(129)), &govts));
        assert!(!Allegiance.hostile(defender, side(ENEMY, ENEMY, Some(128)), &govts));
        let ally = side(DEFENDER, DEFENDER, Some(129));
        assert!(!Allegiance.hostile(ally, side(ENEMY, ENEMY, Some(128)), &govts));
    }

    #[test]
    fn allegiance_calls_hostile_an_enemy_a_neutral_an_independent_and_the_player() {
        let govts = relations();
        let defender = side(DEFENDER, DEFENDER, Some(128));
        for govt in [Some(130), Some(131), None] {
            assert!(
                Allegiance.hostile(defender, side(ENEMY, ENEMY, govt), &govts),
                "{govt:?}"
            );
        }
        let player = side(ShipRef::Player, ShipRef::Player, None);
        assert!(Allegiance.hostile(player, side(ENEMY, ENEMY, Some(128)), &govts));
        let independent = side(DEFENDER, DEFENDER, None);
        assert!(
            Allegiance.hostile(independent, player, &govts),
            "two without a government are not allies"
        );
        assert!(Allegiance.hostile(independent, side(ENEMY, ENEMY, None), &govts));
    }

    #[test]
    fn a_hit_does_its_mass_and_half_its_energy_damage_truncated() {
        assert_eq!(pd_damage(&quad(0)), 3.0);
        let odd = WeaponSpec::new(&WeaponRecord {
            mass_dmg: 3,
            energy_dmg: 2 * 2 + 1,
            ..weapon(161)
        });
        assert_eq!(pd_damage(&odd), 5.0);
    }

    /// The hits of `damage` that destroy a missile of `durability`.
    fn hits_to_destroy(durability: f32, damage: f32) -> u32 {
        let mut shot = Shot {
            durability,
            ..missile(1, 0.0, 0.0, None)
        };
        let mut hits = 1;
        while !shot.take_pd_hit(damage) {
            hits += 1;
            assert!(hits < 100, "never destroyed");
        }
        hits
    }

    #[test]
    fn a_missile_is_destroyed_by_the_hit_after_its_durability_is_used_up() {
        assert_eq!(hits_to_destroy(0.0, 3.0), 1);
        assert_eq!(hits_to_destroy(4.0, 3.0), 3, "4, 1, -2, then gone");
        assert_eq!(hits_to_destroy(3.0, 3.0), 2, "3, 0, then gone");
        assert_eq!(hits_to_destroy(4.0, 1.0), 5);
        let mut shot = Shot {
            durability: 4.0,
            ..missile(1, 0.0, 0.0, None)
        };
        assert!(!shot.take_pd_hit(3.0));
        assert_eq!(shot.durability, 1.0);
    }
}
