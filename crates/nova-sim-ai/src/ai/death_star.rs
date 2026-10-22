//! [`DeathStar`]: the stellar-attacking AI, `_DeathStarAI`
//! (@0x8c125-0x8c2d1 in the `EV Nova` executable), which `_AIDispatch`
//! runs for a mission's special ship of `ShipBehav` 2 in place of its
//! type's think (@0x8ff3a-0x8ff43; see [`NovaAi`](crate::ai::NovaAi)).
//!
//! Its think ([`DeathStar::decision`]), in the original's order:
//!
//! 1. **Guards** (@0x8c12e-0x8c159): a ship disabled or dying, or in a
//!    count wait (goal 0x16), keeps everything, and no warship think runs.
//! 2. **The stellar** ([`enemy_stellar`], @0x8c15f-0x8c1de): the first of
//!    the system's stellars, in its `sÿst` order, still whole, of
//!    `Strength` above 0 ([`Surroundings::destroyable`]), whose government
//!    `_GovtEnemies` finds its enemy. Its travel stellar (+0x6c) is set to
//!    it at once (@0x8c286).
//! 3. **The weapon** ([`planet_weapon`], @0x8c1e0-0x8c253): the lowest
//!    weapon index that is planet-type (`Flags2` 0x0400), that its
//!    **class** carries in stock (class +0x5e8, not the ship's own count),
//!    that has its ammunition (`_WeaponHasAmmo`) and whose guidance is not
//!    0 or 3 and at most 7 ([`planet_guidance`]).
//! 4. **Both found** (@0x8c25a-0x8c280): goal 0x12,
//!    [`Goal::AttackStellar`], its target cleared and its travel stellar
//!    the stellar; no warship think runs.
//! 5. **Otherwise** (@0x8c293-0x8c2c5): a ship in goal 0x12 drops it (goal
//!    and sub-goal 0, target and travel stellar -1), and then the
//!    warship's think runs, whatever the ship's own type. A landing or a
//!    gate entry the warship keeps flies to the travel stellar the scan
//!    left, as the high level reads +0x6c for goals 1 and 0x14 (@0x8d8c9,
//!    @0x8daea, @0x8db70): a landing on an enemy gate is entered, by the
//!    hand-off (@0x8d8b8-0x8d8fe), and a gate entry goes to the stellar
//!    whatever it is.
//!
//! It draws nothing and plays no sound.

use std::rc::Rc;

use crate::ai::travel::TravelWrite;
use crate::ai::{Behaviour, Decision, Goal, Surroundings, fire};
use crate::catalog::{LandingSite, WeaponId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::armament::{Arsenal, Mount, Trigger};
use crate::combat::weapon::{Guidance, WeaponSpec};
use crate::flight::ShipState;
use crate::gate::GateKind;
use crate::geometry::Vec2;
use crate::traffic::inertialess;
use crate::traffic::npc::Npc;
use crate::trig;

/// The stellar-attacking AI (see the module docs), the warship's think
/// running in its place when it has no stellar to attack.
#[derive(Clone, Debug)]
pub struct DeathStar {
    /// The think `_DeathStarAI` tail-calls (`_WarshipAI`).
    pub warship: Rc<dyn Behaviour>,
}

impl Behaviour for DeathStar {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// `_DeathStarAI`'s think (see the module docs).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        if npc.condition.disabled() || npc.goal == Goal::CountWait {
            return Decision {
                goal: npc.goal,
                ..Decision::default()
            };
        }
        let enemy = enemy_stellar(npc, around);
        if let Some(stellar) = enemy
            && planet_weapon(npc, around.weapons).is_some()
        {
            return Decision {
                goal: Goal::AttackStellar(stellar.id),
                clears_target: true,
                travel: TravelWrite::Set,
                ..Decision::default()
            };
        }
        let reset = matches!(npc.goal, Goal::AttackStellar(_));
        let asked = if reset {
            Npc {
                goal: Goal::Idle,
                target: None,
                travel_set: false,
                ..npc.clone()
            }
        } else {
            npc.clone()
        };
        let decision = self.warship.decision(&asked, around, chance);
        // +0x6c after the scan's write and the drop's clear.
        let travel_to = enemy.filter(|_| !reset);
        let travel = match (decision.travel, reset, travel_to) {
            (TravelWrite::Keep, true, _) => TravelWrite::Clear,
            (TravelWrite::Keep, false, Some(_)) => TravelWrite::Set,
            (written, _, _) => written,
        };
        let kept = decision.goal == asked.goal && decision.travel == TravelWrite::Keep;
        let goal = match (decision.goal, travel_to) {
            (Goal::Land(_), Some(stellar)) if kept => {
                if GateKind::of(stellar.flags2).is_some() {
                    Goal::EnterGate(stellar.id)
                } else {
                    Goal::Land(stellar.id)
                }
            }
            (Goal::EnterGate(_), Some(stellar)) if kept => Goal::EnterGate(stellar.id),
            (goal, _) => goal,
        };
        Decision {
            goal,
            travel,
            clears_target: decision.clears_target || (reset && goal.quarry().is_none()),
            ..decision
        }
    }

    /// No ship while it attacks a stellar (its +0x70 holds the stellar's
    /// index, which no reader of a ship target is fed here); the
    /// warship's otherwise.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        if matches!(npc.goal, Goal::AttackStellar(_)) {
            None
        } else {
            self.warship.target(npc, around)
        }
    }
}

/// The stellar `npc` attacks among `around` (@0x8c15f-0x8c1de): the first
/// of the system's stellars, in its `sÿst` order, not destroyed, that shots
/// can shoot down ([`Surroundings::destroyable`]) and whose government is
/// an enemy of `npc`'s (`_GovtEnemies(ship +0x98, stellar +0x0a)`).
#[must_use]
pub fn enemy_stellar<'a>(npc: &Npc, around: &Surroundings<'a>) -> Option<&'a LandingSite> {
    around.sites.iter().find(|site| {
        !site.destroyed
            && around.destroyable.contains(&site.id)
            && around.govts.enemies(npc.govt, site.govt)
    })
}

/// Whether a weapon of `guidance` may be a planet gun, by
/// `_DeathStarAI`'s and `_AIFirePlanetGun`'s test (@0x8c230-0x8c244,
/// @0x80536-0x805c9): its raw guidance not 0 or 3, and at most 7. So -1,
/// 1, 2, 4, 5, 6 and 7 pass, and 8, 9, 10 and 99 do not.
#[must_use]
pub fn planet_guidance(guidance: Guidance) -> bool {
    match guidance {
        Guidance::Unguided
        | Guidance::Homing
        | Guidance::Turret
        | Guidance::FreefallBomb
        | Guidance::Rocket
        | Guidance::FrontTurret => true,
        Guidance::Other(raw) => raw <= 7,
        Guidance::Beam
        | Guidance::TurretBeam
        | Guidance::RearTurret
        | Guidance::PointDefence
        | Guidance::PointDefenceBeam
        | Guidance::FighterBay => false,
    }
}

/// The planet gun `npc` attacks with, read from `weapons`, the weapon
/// records and ship classes ([`Surroundings::weapons`]): `_DeathStarAI`'s
/// weapon test (@0x8c1e0-0x8c253), the lowest-index weapon (by ascending
/// `wëap` ID) that is planet-type, that its class carries in stock
/// (`Arsenal::stock_slots`, class `+0x5e8[i]` above 0, @0x8c20e), that it
/// has the ammunition for ([`fire::has_ammo`], @0x8c21f) and whose
/// guidance passes ([`planet_guidance`]). Its reload is not read.
#[must_use]
pub fn planet_weapon(npc: &Npc, weapons: &Arsenal) -> Option<WeaponSpec> {
    weapons
        .stock_slots(npc.ship)
        .into_iter()
        .filter(|&(_, (count, _))| count > 0)
        .filter_map(|(id, _)| weapons.weapon(id).copied())
        .find(|spec| {
            spec.planet_type()
                && planet_guidance(spec.guidance)
                && fire::has_ammo(npc, &own_mount(npc, spec))
        })
}

/// `_HighLevelAIHandler`'s goal-0x12 band (@0x8f906-0x8faeb): the
/// substate a stellar attacker flies by its squared distance d² from its
/// stellar against its planet weapon's squared range r².
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// Sub 2: beyond 0.75 r, it closes in.
    Approach,
    /// Sub 3: inside 0.25 r, it runs out.
    RunOut,
    /// Sub 1: in band and moving, it brakes.
    Brake,
    /// Sub 0x16: in band and at rest, it aims and fires.
    Aim,
}

/// d² above this times r² approaches (the double 0.5625 @0xddaa0).
pub const APPROACH_BAND: f32 = 0.5625;
/// d² below this times r² runs out (the double 0.0625 @0xddaa8).
pub const RUN_OUT_BAND: f32 = 0.0625;
/// The sub 0x16 fires within its turn rate and this many degrees of the
/// stellar's bearing, strictly (15.0 @0xdda30).
pub const AIM_FACING: f32 = 15.0;
/// r² with no planet weapon: the float 1.0 (@0xdd068), not a range
/// squared.
pub const NO_GUN_RANGE_SQ: f32 = 1.0;

/// The band a ship at `at` flies attacking a stellar at `stellar` with a
/// planet weapon reaching `range`, if any (see [`Band`]): beyond 0.75 r
/// it approaches, inside 0.25 r it runs out, each strictly; in between it
/// brakes while either axis of its velocity is at least the double 0.35
/// (@0xdda68, [`inertialess::moving`]), and aims otherwise.
#[must_use]
pub fn band(at: &ShipState, stellar: Vec2, range: Option<f32>) -> Band {
    let reach = range.map_or(NO_GUN_RANGE_SQ, |range| range * range);
    let off = stellar - at.position;
    let squared = off.x * off.x + off.y * off.y;
    if squared > APPROACH_BAND * reach {
        Band::Approach
    } else if squared < RUN_OUT_BAND * reach {
        Band::RunOut
    } else if inertialess::moving(at.velocity) {
        Band::Brake
    } else {
        Band::Aim
    }
}

/// The band `npc` flies attacking its stellar among `sites`, and the
/// stellar, while it is in [`Goal::AttackStellar`]: by the range of the
/// planet weapon its think takes from `weapons` ([`planet_weapon`],
/// [`WeaponSpec::range`], the weapon's +0x5c), or with none by
/// [`NO_GUN_RANGE_SQ`].
#[must_use]
pub fn attack_band<'a>(
    npc: &Npc,
    sites: &'a [LandingSite],
    weapons: &Arsenal,
) -> Option<(Band, &'a LandingSite)> {
    let Goal::AttackStellar(stellar) = npc.goal else {
        return None;
    };
    let site = sites.iter().find(|site| site.id == stellar)?;
    let range = planet_weapon(npc, weapons).map(|spec| spec.range());
    Some((band(&npc.state, site.position, range), site))
}

/// The weapon `_AIFirePlanetGun` (@0x80536-0x805c9) picks for `npc`: the
/// **highest** index among its own mounts it carries, of a guidance
/// [`planet_guidance`] passes, with its ammunition ([`fire::has_ammo`])
/// and its reload at or below none. It does not test the planet-type bit,
/// so a ready ordinary gun above the planet gun wins. None when nothing
/// is ready.
#[must_use]
pub fn planet_gun_pick(npc: &Npc) -> Option<WeaponId> {
    npc.armament
        .mounts()
        .iter()
        .filter(|mount| {
            mount.count > 0
                && planet_guidance(mount.spec.guidance)
                && fire::has_ammo(npc, mount)
                && mount.reload <= 0.0
        })
        .map(|mount| mount.spec.id)
        .max()
}

/// What a stellar attacker fires among `around`: its planet gun pick
/// ([`planet_gun_pick`]) as its pick and its flag (+0x72, +0xba), only in
/// the [`Band::Aim`] substate (0x16, @0x88645-0x886f4) and while it faces
/// its stellar's bearing (`_CalcInterceptAngle`) within its turn rate and
/// [`AIM_FACING`] degrees, strictly. Otherwise it fires nothing and keeps
/// the pick it had, as the low level's preamble clears only the flag
/// (@0x851e6-0x851f6). It never asks for its point defence or its bays.
/// While its AI wait runs the fight holds it ([`fire::gated`]).
#[must_use]
pub fn trigger(npc: &Npc, around: &Surroundings) -> Trigger {
    let Some((Band::Aim, site)) = attack_band(npc, around.sites, around.weapons) else {
        return Trigger::default();
    };
    let bearing = trig::intercept_angle(npc.state.position, site.position);
    let facing = fire::off_by(npc.state.heading, bearing) < npc.turn_rate() + AIM_FACING;
    let pick = planet_gun_pick(npc).filter(|_| facing);
    Trigger {
        only: pick,
        keep: pick,
        ..Trigger::default()
    }
}

/// `npc`'s mount of `spec`, or one of none when it carries none, which
/// `_WeaponHasAmmo` reads all the same.
fn own_mount(npc: &Npc, spec: &WeaponSpec) -> Mount {
    npc.armament
        .mounts()
        .iter()
        .find(|mount| mount.spec.id == spec.id)
        .copied()
        .unwrap_or(Mount {
            spec: *spec,
            count: 0,
            reload: 0.0,
            burst: 0,
        })
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, govts, ship};
    use crate::catalog::{HullRecord, ShipId, StellarId, StockWeapon, WeaponId, WeaponRecord};
    use crate::combat::armament::{Armament, Arsenal, LauncherRounds};
    use crate::combat::hull::Condition;
    use crate::combat::weapon::PLANET_TYPE;
    use crate::govt::Governments;
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::{hull, planet, weapon};

    /// The warship's think, answering `goal` with `travel`, recording the
    /// goal, target and travel flag of each ship it is asked of.
    #[derive(Debug)]
    struct Recording {
        goal: Goal,
        travel: TravelWrite,
        asked: RefCell<Vec<(Goal, Option<ShipRef>, bool)>>,
    }

    impl Recording {
        fn answering(goal: Goal) -> Rc<Self> {
            Self::writing(goal, TravelWrite::Keep)
        }

        fn writing(goal: Goal, travel: TravelWrite) -> Rc<Self> {
            Rc::new(Self {
                goal,
                travel,
                asked: RefCell::default(),
            })
        }
    }

    impl Behaviour for Recording {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            self.asked
                .borrow_mut()
                .push((npc.goal, npc.target, npc.travel_set));
            self.goal
        }

        fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
            Decision {
                goal: self.decide(npc, around, chance),
                travel: self.travel,
                ..Decision::default()
            }
        }

        fn target(&self, _npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            Some(ShipRef::Npc(crate::traffic::npc::NpcId(77)))
        }
    }

    /// A planet gun, `wëap` `id`, of `guidance`.
    fn planet_gun(id: i16, guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            flags2: PLANET_TYPE,
            ..weapon(id)
        }
    }

    /// The pirates' stellar `id` at (`x`, 0).
    fn pirate_stellar(id: i16, x: f32) -> LandingSite {
        LandingSite {
            govt: Some(PIRATES),
            ..planet(id, x, 0.0)
        }
    }

    /// The system's stellars: 128 the traders' own, 129 a pirates' one
    /// destroyed, 130 a pirates' one of `Strength` 0, and 131 and 132 the
    /// pirates', whole and of `Strength` above 0.
    fn sites() -> Vec<LandingSite> {
        let destroyed = LandingSite {
            destroyed: true,
            ..pirate_stellar(129, 200.0)
        };
        vec![
            LandingSite {
                govt: Some(TRADERS),
                ..planet(128, 100.0, 0.0)
            },
            destroyed,
            pirate_stellar(130, 300.0),
            pirate_stellar(131, 400.0),
            pirate_stellar(132, 500.0),
        ]
    }

    /// Those shots can shoot down: all but 130.
    fn destroyable() -> BTreeSet<StellarId> {
        [128, 129, 131, 132].map(StellarId).into()
    }

    /// Class 128 carrying `stock` in stock, and the weapons `records`.
    fn arsenal(records: &[WeaponRecord], stock: &[(i16, i16)]) -> Arsenal {
        let hull = HullRecord {
            weapons: stock
                .iter()
                .map(|&(weapon, count)| StockWeapon {
                    weapon: WeaponId(weapon),
                    count,
                    ammo: 0,
                })
                .collect(),
            ..hull(128)
        };
        Arsenal::new(records, vec![hull])
    }

    /// A trader of class 128 carrying `records`, one of each.
    fn attacker(records: &[WeaponRecord]) -> Npc {
        let mut npc = ship(1, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        npc.ship = ShipId(128);
        npc.armament = Armament::new(records.iter().map(|record| (WeaponSpec::new(record), 1)));
        npc
    }

    /// What `npc` decides by [`DeathStar`] around `sites`, armed from
    /// `arsenal`, its warship's think `warship`.
    fn decided(
        npc: &Npc,
        sites: &[LandingSite],
        arsenal: &Arsenal,
        warship: Rc<Recording>,
    ) -> Decision {
        let govts: Governments = govts(0, 0);
        let destroyable = destroyable();
        let npcs = [npc.clone()];
        let around = Surroundings {
            govts: &govts,
            weapons: arsenal,
            destroyable: &destroyable,
            ..Surroundings::new(sites, &npcs)
        };
        DeathStar { warship }.decision(npc, &around, &mut Draws::of(&[]))
    }

    #[test]
    fn a_marked_ship_with_a_planet_gun_attacks_the_first_enemy_stellar_it_can_shoot_down() {
        let gun = planet_gun(130, -1);
        let warship = Recording::answering(Goal::JumpOut);
        let decision = decided(
            &attacker(&[gun]),
            &sites(),
            &arsenal(&[gun], &[(130, 1)]),
            warship.clone(),
        );
        assert_eq!(decision.goal, Goal::AttackStellar(StellarId(131)));
        assert!(decision.clears_target);
        assert_eq!(decision.travel, TravelWrite::Set);
        assert_eq!(*warship.asked.borrow(), [], "no warship think");
    }

    #[test]
    fn only_the_first_enemy_in_the_systems_order_is_taken() {
        let gun = planet_gun(130, -1);
        let mut sites = sites();
        sites.swap(3, 4);
        let decision = decided(
            &attacker(&[gun]),
            &sites,
            &arsenal(&[gun], &[(130, 1)]),
            Recording::answering(Goal::JumpOut),
        );
        assert_eq!(decision.goal, Goal::AttackStellar(StellarId(132)));
    }

    #[test]
    fn the_weapon_test_takes_the_lowest_planet_type_weapon_of_the_classs_stock() {
        let ordinary = WeaponRecord {
            flags2: 0,
            ..planet_gun(129, -1)
        };
        let gun = planet_gun(131, -1);
        let npc = attacker(&[ordinary, gun]);
        let armed = arsenal(&[ordinary, gun], &[(129, 1), (131, 1)]);
        assert_eq!(
            planet_weapon(&npc, &armed).map(|spec| spec.id),
            Some(WeaponId(131))
        );
        let lower = planet_gun(130, 4);
        let npc = attacker(&[lower, gun]);
        let armed = arsenal(&[lower, gun], &[(130, 1), (131, 1)]);
        assert_eq!(
            planet_weapon(&npc, &armed).map(|spec| spec.id),
            Some(WeaponId(130)),
            "the lowest index"
        );
    }

    #[test]
    fn the_weapon_test_reads_the_guidance_the_ammunition_and_the_classs_stock() {
        let found = |record: WeaponRecord, stock: i16, rounds: i32| {
            let mut npc = attacker(&[record]);
            npc.rounds = LauncherRounds::from([(record.id, rounds)]);
            let armed = arsenal(&[record], &[(record.id.0, stock)]);
            planet_weapon(&npc, &armed).is_some()
        };
        for guidance in [-1, 1, 2, 4, 5, 6, 7] {
            assert!(found(planet_gun(130, guidance), 1, 0), "{guidance}");
        }
        for guidance in [0, 3, 8, 9, 10, 99] {
            assert!(!found(planet_gun(130, guidance), 1, 0), "{guidance}");
        }
        let launcher = WeaponRecord {
            ammo_type: 10,
            ..planet_gun(130, -1)
        };
        assert!(!found(launcher, 1, 0), "no rounds");
        assert!(found(launcher, 1, 1), "a round");
        assert!(!found(planet_gun(130, -1), 0, 0), "not the class's");
        let slow = WeaponRecord {
            reload: 50,
            ..planet_gun(130, -1)
        };
        let mut npc = attacker(&[slow]);
        npc.armament.hold(slow.id);
        assert!(npc.armament.mounts()[0].reload > 0.0);
        let armed = arsenal(&[slow], &[(130, 1)]);
        assert!(
            planet_weapon(&npc, &armed).is_some(),
            "its reload is not read"
        );
    }

    #[test]
    fn the_class_stock_counts_though_the_ship_carries_none() {
        let gun = planet_gun(130, -1);
        let npc = attacker(&[]);
        let armed = arsenal(&[gun], &[(130, 1)]);
        assert!(planet_weapon(&npc, &armed).is_some());
    }

    #[test]
    fn with_an_enemy_stellar_and_no_planet_gun_the_warship_decides_and_a_kept_landing_goes_there() {
        let gun = WeaponRecord {
            flags2: 0,
            ..planet_gun(130, -1)
        };
        let armed = arsenal(&[gun], &[(130, 1)]);
        let mut landing = attacker(&[gun]);
        landing.goal = Goal::Land(StellarId(128));
        let warship = Recording::answering(Goal::Land(StellarId(128)));
        let decision = decided(&landing, &sites(), &armed, warship.clone());
        assert_eq!(decision.goal, Goal::Land(StellarId(131)), "@0x8d8c9");
        assert_eq!(decision.travel, TravelWrite::Set);
        assert_eq!(warship.asked.borrow().len(), 1);
        let mut gate_sites = sites();
        gate_sites[3].flags2 = 0x1000;
        let decision = decided(
            &landing,
            &gate_sites,
            &armed,
            Recording::answering(Goal::Land(StellarId(128))),
        );
        assert_eq!(
            decision.goal,
            Goal::EnterGate(StellarId(131)),
            "the hand-off"
        );
        let mut entering = attacker(&[gun]);
        entering.goal = Goal::EnterGate(StellarId(128));
        let decision = decided(
            &entering,
            &sites(),
            &armed,
            Recording::answering(Goal::EnterGate(StellarId(128))),
        );
        assert_eq!(decision.goal, Goal::EnterGate(StellarId(131)), "@0x8db70");
        let set = decided(
            &attacker(&[gun]),
            &sites(),
            &armed,
            Recording::answering(Goal::Land(StellarId(128))),
        );
        assert_eq!(
            set.goal,
            Goal::Land(StellarId(128)),
            "a landing set this think stands"
        );
        let written = decided(
            &landing,
            &sites(),
            &armed,
            Recording::writing(Goal::Land(StellarId(128)), TravelWrite::Clear),
        );
        assert_eq!(
            written.goal,
            Goal::Land(StellarId(128)),
            "the warship's own write"
        );
        assert_eq!(written.travel, TravelWrite::Clear);
        let no_enemy: Vec<LandingSite> = sites()[..3].to_vec();
        let decision = decided(
            &landing,
            &no_enemy,
            &armed,
            Recording::answering(Goal::Land(StellarId(128))),
        );
        assert_eq!(decision.goal, Goal::Land(StellarId(128)));
        assert_eq!(decision.travel, TravelWrite::Keep);
    }

    #[test]
    fn a_stellar_attacker_whose_stellar_or_gun_is_gone_drops_the_goal_before_the_warship_thinks() {
        let gun = planet_gun(130, -1);
        let armed = arsenal(&[gun], &[(130, 1)]);
        let mut npc = attacker(&[gun]);
        npc.goal = Goal::AttackStellar(StellarId(131));
        npc.target = Some(ShipRef::Player);
        npc.travel_set = true;
        let no_enemy: Vec<LandingSite> = sites()[..3].to_vec();
        let warship = Recording::answering(Goal::Wait);
        let decision = decided(&npc, &no_enemy, &armed, warship.clone());
        assert_eq!(*warship.asked.borrow(), [(Goal::Idle, None, false)]);
        assert_eq!(decision.goal, Goal::Wait);
        assert_eq!(decision.travel, TravelWrite::Clear);
        assert!(decision.clears_target);
        let dry = arsenal(&[gun], &[(130, 0)]);
        let warship = Recording::writing(Goal::Land(StellarId(128)), TravelWrite::Set);
        let decision = decided(&npc, &sites(), &dry, warship.clone());
        assert_eq!(*warship.asked.borrow(), [(Goal::Idle, None, false)]);
        assert_eq!(
            decision.goal,
            Goal::Land(StellarId(128)),
            "no redirect: +0x6c was cleared"
        );
        assert_eq!(decision.travel, TravelWrite::Set, "the warship's write");
        let fighting = Recording::answering(Goal::Attack(ShipRef::Player));
        let decision = decided(&npc, &no_enemy, &armed, fighting);
        assert!(
            !decision.clears_target,
            "a fight keeps the target the warship takes"
        );
    }

    #[test]
    fn a_ship_waiting_disabled_or_dying_keeps_everything() {
        let gun = planet_gun(130, -1);
        let armed = arsenal(&[gun], &[(130, 1)]);
        for (goal, condition) in [
            (Goal::CountWait, Condition::Intact),
            (Goal::Land(StellarId(128)), Condition::Disabled),
            (
                Goal::Land(StellarId(128)),
                Condition::Dying {
                    ticks_left: 3,
                    starting: false,
                },
            ),
        ] {
            let mut npc = attacker(&[gun]);
            npc.goal = goal;
            npc.condition = condition;
            let warship = Recording::answering(Goal::JumpOut);
            let decision = decided(&npc, &sites(), &armed, warship.clone());
            assert_eq!(
                decision,
                Decision {
                    goal,
                    ..Decision::default()
                },
                "{goal:?}"
            );
            assert_eq!(*warship.asked.borrow(), [], "{goal:?}");
        }
    }

    #[test]
    fn a_stellar_attacker_the_player_turned_on_goes_back_to_its_stellar() {
        let gun = planet_gun(130, -1);
        let mut npc = attacker(&[gun]);
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        let decision = decided(
            &npc,
            &sites(),
            &arsenal(&[gun], &[(130, 1)]),
            Recording::answering(Goal::JumpOut),
        );
        assert_eq!(decision.goal, Goal::AttackStellar(StellarId(131)));
    }

    #[test]
    fn a_stellar_attacker_targets_no_ship_and_otherwise_as_the_warship() {
        let npc = attacker(&[]);
        let around = Surroundings::new(&[], &[]);
        let death_star = DeathStar {
            warship: Recording::answering(Goal::Idle),
        };
        let attacking = Npc {
            goal: Goal::AttackStellar(StellarId(131)),
            ..npc.clone()
        };
        assert_eq!(death_star.target(&attacking, &around), None);
        assert_eq!(
            death_star.target(&npc, &around),
            Some(ShipRef::Npc(crate::traffic::npc::NpcId(77)))
        );
    }

    // The high level's band and the planet gun.

    /// A ship at (`x`, 0) moving (`vx`, `vy`).
    fn at(x: f32, (vx, vy): (f32, f32)) -> ShipState {
        ShipState {
            position: Vec2::new(x, 0.0),
            velocity: Vec2::new(vx, vy),
            heading: 0.0,
        }
    }

    #[test]
    fn the_band_approaches_beyond_three_quarters_runs_out_inside_a_quarter_and_brakes_or_aims_between()
     {
        let band_at = |x: f32, v: (f32, f32)| band(&at(x, v), Vec2::ZERO, Some(100.0));
        assert_eq!(band_at(75.01, (0.0, 0.0)), Band::Approach);
        assert_eq!(
            band_at(75.0, (0.0, 0.0)),
            Band::Aim,
            "exactly 0.75 r is in band"
        );
        assert_eq!(
            band_at(25.0, (0.0, 0.0)),
            Band::Aim,
            "exactly 0.25 r is in band"
        );
        assert_eq!(band_at(24.99, (0.0, 0.0)), Band::RunOut);
        // The float against the double 0.35: a float 0.35 is just below.
        assert_eq!(band_at(50.0, (0.350_001, 0.0)), Band::Brake);
        assert_eq!(band_at(50.0, (0.0, -0.350_001)), Band::Brake);
        assert_eq!(band_at(50.0, (0.35, -0.35)), Band::Aim);
        assert_eq!(
            band_at(80.0, (5.0, 0.0)),
            Band::Approach,
            "speed counts only in band"
        );
        assert_eq!(band_at(10.0, (5.0, 0.0)), Band::RunOut);
        let no_gun = |x: f32| band(&at(x, (0.0, 0.0)), Vec2::ZERO, None);
        assert_eq!(no_gun(0.75), Band::Aim, "r² 1.0, not squared");
        assert_eq!(no_gun(0.76), Band::Approach);
        assert_eq!(no_gun(0.24), Band::RunOut);
        assert_eq!(
            (APPROACH_BAND, RUN_OUT_BAND, AIM_FACING, NO_GUN_RANGE_SQ),
            (0.5625, 0.0625, 15.0, 1.0)
        );
    }

    #[test]
    fn the_band_reads_the_range_of_the_planet_weapon_the_think_takes() {
        let gun = WeaponRecord {
            count: 50,
            speed: 400,
            ..planet_gun(131, -1)
        };
        let armed = arsenal(&[gun], &[(131, 1)]);
        let range = armed.weapon(WeaponId(131)).map_or(0.0, WeaponSpec::range);
        // 100 off 131 at (400, 0): in band at this range, beyond r² 1.0.
        assert!((134.0..400.0).contains(&range), "{range}");
        let mut npc = attacker(&[gun]);
        npc.goal = Goal::AttackStellar(StellarId(131));
        npc.state.position = Vec2::new(300.0, 0.0);
        let sites = sites();
        let (band, site) = attack_band(&npc, &sites, &armed).expect("attacking 131");
        assert_eq!((site.id, band), (StellarId(131), Band::Aim));
        let unarmed = arsenal(&[gun], &[]);
        let (band, _) = attack_band(&npc, &sites, &unarmed).expect("attacking 131");
        assert_eq!(band, Band::Approach, "no planet weapon");
        let idle = Npc {
            goal: Goal::Idle,
            ..npc.clone()
        };
        assert_eq!(attack_band(&idle, &sites, &armed), None);
        let gone = Npc {
            goal: Goal::AttackStellar(StellarId(140)),
            ..npc
        };
        assert_eq!(attack_band(&gone, &sites, &armed), None);
    }

    /// A gun of `guidance` and `flags2`, `wëap` `id`, reloading 30.
    fn gun(id: i16, guidance: i16, flags2: u16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            flags2,
            reload: 30,
            ..weapon(id)
        }
    }

    #[test]
    fn the_planet_gun_pick_takes_the_highest_ready_weapon_planet_type_or_not() {
        let planet = gun(130, -1, PLANET_TYPE);
        let ordinary = gun(132, 4, 0);
        let npc = attacker(&[planet, ordinary]);
        assert_eq!(planet_gun_pick(&npc), Some(WeaponId(132)), "the highest");
        let mut reloading = npc.clone();
        reloading.armament.hold(WeaponId(132));
        assert_eq!(planet_gun_pick(&reloading), Some(WeaponId(130)));
        for guidance in [0, 3, 8, 9, 10] {
            let npc = attacker(&[planet, gun(132, guidance, 0)]);
            assert_eq!(planet_gun_pick(&npc), Some(WeaponId(130)), "{guidance}");
        }
        let launcher = WeaponRecord {
            ammo_type: 10,
            ..gun(132, -1, 0)
        };
        let dry = attacker(&[planet, launcher]);
        assert_eq!(planet_gun_pick(&dry), Some(WeaponId(130)), "no rounds");
        let mut all_held = npc;
        all_held.armament.hold(WeaponId(130));
        all_held.armament.hold(WeaponId(132));
        assert_eq!(planet_gun_pick(&all_held), None);
        assert_eq!(planet_gun_pick(&attacker(&[])), None);
    }

    /// The trigger of a ship at the centre facing `off` degrees off its
    /// bearing to 131, at (60, 0), moving `velocity`, attacking it with a
    /// planet gun of range 100.
    fn aimed(off: i16, velocity: (f32, f32), goal: Goal) -> Trigger {
        let planet_gun = WeaponRecord {
            count: 25,
            speed: 400,
            ..gun(130, -1, PLANET_TYPE)
        };
        let armed = arsenal(&[planet_gun], &[(130, 1)]);
        let mut npc = attacker(&[planet_gun]);
        npc.goal = goal;
        let site = LandingSite {
            govt: Some(PIRATES),
            ..planet(131, 60.0, 0.0)
        };
        let bearing = crate::trig::intercept_angle(npc.state.position, site.position);
        npc.state.heading = f32::from(bearing + off);
        npc.state.velocity = Vec2::new(velocity.0, velocity.1);
        let sites = [site];
        let npcs = [npc.clone()];
        let around = Surroundings {
            weapons: &armed,
            ..Surroundings::new(&sites, &npcs)
        };
        let range = armed.weapon(WeaponId(130)).map_or(0.0, WeaponSpec::range);
        assert!(
            range * 0.75 > 60.0 && range * 0.25 < 60.0,
            "in band: {range}"
        );
        trigger(&npc, &around)
    }

    #[test]
    fn the_trigger_fires_the_pick_aiming_within_its_turn_and_15_degrees() {
        let npc = attacker(&[]);
        let limit = npc.turn_rate() + AIM_FACING;
        let inside = limit.ceil() as i16 - 1;
        let attack = Goal::AttackStellar(StellarId(131));
        let fired = Trigger {
            only: Some(WeaponId(130)),
            keep: Some(WeaponId(130)),
            ..Trigger::default()
        };
        assert_eq!(aimed(0, (0.0, 0.0), attack), fired);
        assert_eq!(aimed(inside, (0.0, 0.0), attack), fired);
        assert_eq!(aimed(-inside, (0.0, 0.0), attack), fired);
        assert_eq!(
            aimed(limit.ceil() as i16, (0.0, 0.0), attack),
            Trigger::default(),
            "strictly"
        );
        assert_eq!(aimed(0, (1.0, 0.0), attack), Trigger::default(), "braking");
        assert_eq!(aimed(0, (0.0, 0.0), Goal::Idle), Trigger::default());
    }
}
