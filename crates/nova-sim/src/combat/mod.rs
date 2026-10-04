//! Combat: ships firing weapons, shots and beams flying and hitting, and
//! the damage that disables and destroys ships.
//!
//! - [`weapon`]: a weapon's [`WeaponSpec`](weapon::WeaponSpec), its `wëap`
//!   in the simulation's units.
//! - [`armament`]: a ship's [`Armament`](armament::Armament), firing its
//!   weapons on a [`Trigger`](armament::Trigger), and the
//!   [`Arsenal`](armament::Arsenal) armaments are built from.
//! - [`beam`]: a [`Beam`](beam::Beam), held on its firer and hitting the
//!   nearest ship along it.
//! - [`damage`]: what a [`Hit`](damage::Hit) does to a ship's reserves,
//!   and which ships a [`Blast`](damage::Blast) reaches.
//! - [`flags`]: what the simulation does with each documented weapon flag,
//!   and which of a weapon's are not done yet.
//! - [`hull`]: a ship type's [`HullSpec`](hull::HullSpec), its
//!   [`Condition`](hull::Condition), and the [`DisableRule`](hull::DisableRule)
//!   port with Nova's [`NovaDisable`](hull::NovaDisable).
//! - [`projectile`]: a [`Shot`](projectile::Shot) in flight, and the
//!   [`Target`](projectile::Target)s it may hit.
//! - [`report`]: the [`SimDiagnostic`](report::SimDiagnostic)s a session
//!   reports once each about game data it does not handle yet.
//!
//! [`Combat`] holds the shots and beams in flight. Each tick
//! ([`Combat::tick`]) runs over every ship in the fight, each a
//! [`Fighter`], in this order:
//!
//! 1. Each ship fires the weapons its trigger holds that are ready.
//! 2. Every reload timer counts down a tick.
//! 3. Shots fly a tick, and beams follow their firers; a beam whose firer
//!    is gone, or no longer intact, goes with it.
//! 4. Hits, blasts and expiries are resolved: a shot that hits a ship
//!    damages it, explodes and blasts the ships around it, and is gone; a
//!    shot at the end of its life detonates (blasting the same way) or
//!    vanishes; a beam damages the nearest ship along it, every tick, until
//!    its life is over.
//! 5. Conditions follow the damage: a ship with no armour left (of the
//!    armour it holds) starts breaking up, and any other is disabled, or
//!    not, as the [`DisableRule`](hull::DisableRule) says.
//! 6. A ship breaking up counts down its `DeathDelay`, then is destroyed.
//! 7. Shields regenerate on an intact or disabled ship, armour only on an
//!    intact one, each up to what it holds.
//!
//! Each step is reported as a [`CombatEvent`] for the view to show, and
//! each unimplemented weapon feature once as a
//! [`SimDiagnostic`](report::SimDiagnostic).

pub mod armament;
pub mod beam;
pub mod damage;
pub mod flags;
pub mod hull;
pub mod projectile;
pub mod report;
pub mod weapon;

use armament::{Armament, Rounds, Trigger};
use beam::Beam;
use damage::{Blast, Hit};
use hull::{Condition, DisableRule, HullSpec};
use projectile::{Shot, Target};
use report::{Reports, SimDiagnostic};
use weapon::{Explosion, Guidance};

use crate::catalog::{ShipId, WeaponId};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::reserves::{Gauge, Reserves};
use crate::traffic::npc::NpcId;

/// A ship in a fight: the player's, or an NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShipRef {
    /// The player's ship.
    Player,
    /// This NPC.
    Npc(NpcId),
}

/// Something that happened in a fight, for the view to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CombatEvent {
    /// A ship fired a weapon.
    Fired {
        /// The ship.
        ship: ShipRef,
        /// The weapon.
        weapon: WeaponId,
    },
    /// A shot or beam exploded where it hit, or a shot detonated.
    Exploded {
        /// Where.
        at: Vec2,
        /// Its explosion.
        explosion: Explosion,
    },
    /// A ship was disabled.
    Disabled {
        /// The ship.
        ship: ShipRef,
    },
    /// A ship's armour is gone, and it is breaking up.
    BreakingUp {
        /// The ship.
        ship: ShipRef,
        /// Where it is.
        at: Vec2,
        /// Its `Explode1`, if any.
        explosion: Option<Explosion>,
    },
    /// A ship was destroyed: the view draws its explosion and debris.
    Destroyed {
        /// The ship.
        ship: ShipRef,
        /// Its ship type.
        ship_type: ShipId,
        /// Where it was.
        at: Vec2,
        /// How it was moving.
        velocity: Vec2,
        /// Its `Explode2`, if any.
        explosion: Option<Explosion>,
        /// Whether the explosion is the huge one.
        huge: bool,
    },
}

/// One ship in a fight, for a tick: what it is, where, what it holds and
/// how it is holding up, borrowed from wherever the ship is kept.
pub struct Fighter<'a> {
    /// Which ship.
    pub ship: ShipRef,
    /// Its ship type.
    pub ship_type: ShipId,
    /// Its fleet: the lead it escorts, or itself.
    pub fleet: ShipRef,
    /// Where it is and how it moves.
    pub state: ShipState,
    /// Its hull.
    pub hull: HullSpec,
    /// The shield points it regenerates a tick.
    pub shield_regen: f32,
    /// The armour points it regenerates a tick.
    pub armor_regen: f32,
    /// The fire command it holds.
    pub trigger: Trigger,
    /// Its shield, armour and fuel.
    pub reserves: &'a mut Reserves,
    /// How it is holding up.
    pub condition: &'a mut Condition,
    /// Its weapons.
    pub armament: &'a mut Armament,
    /// Its rounds of ammunition.
    pub rounds: &'a mut dyn Rounds,
}

/// The shots and beams in flight, and what the fight has reported.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    shots: Vec<Shot>,
    beams: Vec<Beam>,
    events: Vec<CombatEvent>,
    reports: Reports,
}

impl Combat {
    /// Advances the fight among `fighters` a tick (see the module docs),
    /// disabling ships as `rule` says and drawing each shot's inaccuracy
    /// on `chance`.
    pub fn tick(
        &mut self,
        fighters: &mut [Fighter],
        rule: &(impl DisableRule + ?Sized),
        chance: &mut (impl Chance + ?Sized),
    ) {
        self.fire(fighters, &mut &mut *chance);
        for fighter in fighters.iter_mut() {
            fighter.armament.reload();
        }
        let was: Vec<Vec2> = self.shots.iter_mut().map(Shot::step).collect();
        self.beams.retain_mut(|beam| {
            let firer = fighters.iter().find(|fighter| {
                fighter.ship == beam.firer && *fighter.condition == Condition::Intact
            });
            firer.inspect(|firer| beam.follow(&firer.state)).is_some()
        });
        self.resolve(fighters, &was);
        for fighter in fighters.iter_mut() {
            self.update_condition(fighter, rule);
        }
        for fighter in fighters.iter_mut() {
            regenerate(fighter);
        }
    }

    /// Step 1: each ship fires what its trigger holds.
    fn fire(&mut self, fighters: &mut [Fighter], chance: &mut dyn Chance) {
        for fighter in fighters.iter_mut() {
            let launches = fighter.armament.fire(
                fighter.trigger,
                *fighter.condition,
                fighter.rounds,
                &mut fighter.reserves.fuel,
                chance,
                &mut self.reports,
            );
            for launch in launches {
                self.events.push(CombatEvent::Fired {
                    ship: fighter.ship,
                    weapon: launch.weapon.id,
                });
                let (ship, fleet, state) = (fighter.ship, fighter.fleet, &fighter.state);
                if launch.weapon.guidance == Guidance::Beam {
                    self.beams.push(Beam::launch(
                        launch.weapon,
                        ship,
                        fleet,
                        state,
                        launch.offset,
                    ));
                } else {
                    self.shots.push(Shot::launch(
                        launch.weapon,
                        ship,
                        fleet,
                        state,
                        launch.offset,
                    ));
                }
            }
        }
    }

    /// Step 4: hits, blasts and expiries, the shots having flown from
    /// `was`.
    fn resolve(&mut self, fighters: &mut [Fighter], was: &[Vec2]) {
        let targets: Vec<Target> = fighters
            .iter()
            .map(|fighter| Target {
                ship: fighter.ship,
                fleet: fighter.fleet,
                position: fighter.state.position,
                radius: fighter.hull.hit_radius,
                condition: *fighter.condition,
            })
            .collect();
        let mut hits: Vec<(ShipRef, Hit)> = Vec::new();
        let mut kept = Vec::with_capacity(self.shots.len());
        for (mut shot, &from) in self.shots.drain(..).zip(was) {
            let blast = if let Some((ship, at)) = shot.hit(from, &targets) {
                hits.push((ship, Hit::of(&shot.weapon)));
                shot.position = at;
                Some(shot.blast(Some(ship)))
            } else if shot.expired() {
                shot.weapon.detonates().then(|| shot.blast(None))
            } else {
                kept.push(shot);
                continue;
            };
            if let Some(blast) = blast {
                if let Some(explosion) = shot.weapon.explosion {
                    self.events.push(CombatEvent::Exploded {
                        at: blast.at,
                        explosion,
                    });
                }
                let reached = reached(&blast, &targets);
                hits.extend(
                    reached
                        .into_iter()
                        .map(|ship| (ship, Hit::of(&shot.weapon))),
                );
            }
        }
        self.shots = kept;
        for beam in &mut self.beams {
            if let Some((ship, at)) = beam.hit(&targets) {
                hits.push((ship, Hit::of(&beam.weapon)));
                if let Some(explosion) = beam.weapon.explosion {
                    self.events.push(CombatEvent::Exploded { at, explosion });
                }
            }
            beam.age();
        }
        self.beams.retain(|beam| !beam.expired());
        for (ship, hit) in hits {
            if let Some(fighter) = fighters.iter_mut().find(|fighter| fighter.ship == ship) {
                damage::apply(fighter.reserves, hit);
            }
        }
    }

    /// Steps 5 and 6: `fighter`'s condition follows its damage, as `rule`
    /// says, and a ship breaking up counts down to its destruction.
    fn update_condition(&mut self, fighter: &mut Fighter, rule: &(impl DisableRule + ?Sized)) {
        let ship = fighter.ship;
        let at = fighter.state.position;
        match *fighter.condition {
            Condition::Intact | Condition::Disabled if armour_gone(fighter.reserves.armor) => {
                *fighter.condition = Condition::Dying {
                    ticks_left: fighter.hull.death_delay,
                };
                self.events.push(CombatEvent::BreakingUp {
                    ship,
                    at,
                    explosion: fighter.hull.breakup,
                });
            }
            Condition::Intact | Condition::Disabled => {
                let disabled = rule.disabled(fighter.reserves.armor, &fighter.hull);
                if disabled && *fighter.condition == Condition::Intact {
                    self.events.push(CombatEvent::Disabled { ship });
                }
                *fighter.condition = if disabled {
                    Condition::Disabled
                } else {
                    Condition::Intact
                };
                return;
            }
            Condition::Dying { .. } | Condition::Destroyed => {}
        }
        match *fighter.condition {
            Condition::Dying { ticks_left: 0 } => {
                *fighter.condition = Condition::Destroyed;
                self.events.push(CombatEvent::Destroyed {
                    ship,
                    ship_type: fighter.ship_type,
                    at,
                    velocity: fighter.state.velocity,
                    explosion: fighter.hull.explosion,
                    huge: fighter.hull.huge(),
                });
            }
            Condition::Dying { ticks_left } => {
                *fighter.condition = Condition::Dying {
                    ticks_left: ticks_left - 1,
                };
            }
            _ => {}
        }
    }

    /// The shots in flight.
    #[must_use]
    pub fn shots(&self) -> &[Shot] {
        &self.shots
    }

    /// The beams being fired.
    #[must_use]
    pub fn beams(&self) -> &[Beam] {
        &self.beams
    }

    /// What has happened since this was last taken, in order; taking it
    /// empties the list.
    pub fn take_events(&mut self) -> Vec<CombatEvent> {
        std::mem::take(&mut self.events)
    }

    /// The diagnostics made since they were last taken, each once a
    /// fight; taking them empties the list.
    pub fn take_diagnostics(&mut self) -> Vec<SimDiagnostic> {
        self.reports.take()
    }

    /// Clears the shots and beams, as the ship leaves the system.
    pub fn clear(&mut self) {
        self.shots.clear();
        self.beams.clear();
    }
}

/// Whether `armor` is gone: at or below none, on a ship that holds any.
/// A ship type with no armour at all (stock `shïp` 895, plug-in or test
/// data) has none to lose, as it is never disabled either.
fn armour_gone(armor: Gauge) -> bool {
    armor.now <= 0.0 && armor.max > 0.0
}

/// The ships among `targets` that can still be hit that `blast` reaches.
fn reached(blast: &Blast, targets: &[Target]) -> Vec<ShipRef> {
    blast.reaches(
        targets
            .iter()
            .filter(|target| target.condition.hittable())
            .map(|target| (target.ship, target.position)),
    )
}

/// Step 7: `fighter`'s shields regenerate unless it is breaking up or
/// destroyed, and its armour only while it is intact.
fn regenerate(fighter: &mut Fighter) {
    let condition = *fighter.condition;
    if condition.hittable() {
        recharge(&mut fighter.reserves.shield, fighter.shield_regen);
    }
    if condition == Condition::Intact {
        recharge(&mut fighter.reserves.armor, fighter.armor_regen);
    }
}

/// Adds `rate` to `gauge` up to what it holds, never taking any away.
fn recharge(gauge: &mut Gauge, rate: f32) {
    gauge.now = gauge.now.max((gauge.now + rate).min(gauge.max));
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{BoomId, WeaponRecord};
    use crate::chance::NeverFires;
    use crate::combat::hull::NovaDisable;
    use crate::combat::weapon::WeaponSpec;
    use crate::testkit::weapon;

    /// A ship in a test fight, owning what a fighter borrows.
    struct Ship {
        id: ShipRef,
        state: ShipState,
        hull: HullSpec,
        shield_regen: f32,
        armor_regen: f32,
        trigger: Trigger,
        reserves: Reserves,
        condition: Condition,
        armament: Armament,
        rounds: BTreeMap<WeaponId, u32>,
    }

    impl Ship {
        /// NPC `id` at (`x`, `y`), at rest, facing right, unarmed, with
        /// 10 shield and 30 armour and a default hull.
        fn at(id: u32, x: f32, y: f32) -> Self {
            Self {
                id: ShipRef::Npc(NpcId(id)),
                state: ShipState {
                    position: Vec2::new(x, y),
                    velocity: Vec2::ZERO,
                    heading: 90.0,
                },
                hull: HullSpec::default(),
                shield_regen: 0.0,
                armor_regen: 0.0,
                trigger: Trigger::default(),
                reserves: Reserves::full(10.0, 30.0, 100.0),
                condition: Condition::Intact,
                armament: Armament::default(),
                rounds: BTreeMap::new(),
            }
        }

        /// The ship armed with `record`, its trigger held.
        fn armed(self, record: WeaponRecord) -> Self {
            Self {
                armament: Armament::new([(WeaponSpec::new(&record), 1)]),
                trigger: Trigger {
                    primary: true,
                    secondary: None,
                },
                ..self
            }
        }

        fn fighter(&mut self) -> Fighter<'_> {
            Fighter {
                ship: self.id,
                ship_type: ShipId(128),
                fleet: self.id,
                state: self.state,
                hull: self.hull,
                shield_regen: self.shield_regen,
                armor_regen: self.armor_regen,
                trigger: self.trigger,
                reserves: &mut self.reserves,
                condition: &mut self.condition,
                armament: &mut self.armament,
                rounds: &mut self.rounds,
            }
        }
    }

    fn tick(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(&mut fighters, rule, &mut NeverFires);
    }

    /// A blaster firing every tick, 15 pixels a tick for 13 ticks, doing
    /// 4 mass and 10 energy damage.
    fn blaster() -> WeaponRecord {
        WeaponRecord {
            count: 13,
            speed: 1500,
            mass_dmg: 4,
            energy_dmg: 10,
            ..weapon(128)
        }
    }

    const A: ShipRef = ShipRef::Npc(NpcId(1));
    const B: ShipRef = ShipRef::Npc(NpcId(2));
    const C: ShipRef = ShipRef::Npc(NpcId(3));

    /// The events about `ship`, other than its firing.
    fn about(events: &[CombatEvent], ship: ShipRef) -> Vec<CombatEvent> {
        events
            .iter()
            .filter(|event| match **event {
                CombatEvent::Disabled { ship: s }
                | CombatEvent::BreakingUp { ship: s, .. }
                | CombatEvent::Destroyed { ship: s, .. } => s == ship,
                _ => false,
            })
            .copied()
            .collect()
    }

    fn fired(events: &[CombatEvent], ship: ShipRef) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, CombatEvent::Fired { ship: s, .. } if *s == ship))
            .count()
    }

    #[test]
    fn a_shot_fired_this_tick_has_flown_a_tick() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster())];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(128)
            }]
        );
        assert_eq!(combat.take_events(), [], "taken");
        assert_eq!(combat.shots().len(), 1);
        assert!((combat.shots()[0].position.x - 15.0).abs() < 1e-4);
        assert_eq!(combat.beams(), []);
        for _ in 1..13 {
            tick(&mut combat, &mut [Ship::at(1, 0.0, 0.0)], &NovaDisable);
        }
        assert_eq!(combat.shots(), [], "gone after its count");
    }

    #[test]
    fn a_ship_is_disabled_then_breaks_up_then_is_destroyed_by_further_hits() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0).armed(blaster());
        target.hull.death_delay = 3;
        target.hull.explosion = Explosion::decode(5);
        target.hull.breakup = Explosion::decode(4);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), target];
        let mut events = Vec::new();
        let mut log = Vec::new();
        for tick_no in 0..40 {
            tick(&mut combat, &mut ships, &NovaDisable);
            let new = combat.take_events();
            log.push((tick_no, fired(&new, B), about(&new, B)));
            events.extend(new);
        }
        let seen: Vec<(u32, CombatEvent)> = log
            .iter()
            .flat_map(|(tick_no, _, about)| about.iter().map(|event| (*tick_no, *event)))
            .collect();
        assert_eq!(seen.len(), 3, "{seen:?}");
        let (disabled_on, disabled) = seen[0];
        let (breaking_on, breaking) = seen[1];
        let (destroyed_on, destroyed) = seen[2];
        assert_eq!(disabled, CombatEvent::Disabled { ship: B });
        assert_eq!(
            breaking,
            CombatEvent::BreakingUp {
                ship: B,
                at: Vec2::new(100.0, 0.0),
                explosion: Explosion::decode(4)
            }
        );
        assert_eq!(
            destroyed,
            CombatEvent::Destroyed {
                ship: B,
                ship_type: ShipId(128),
                at: Vec2::new(100.0, 0.0),
                velocity: Vec2::ZERO,
                explosion: Some(Explosion {
                    boom: BoomId(133),
                    extra: false
                }),
                huge: false
            }
        );
        // Six hits take the shield and 24 armour (6 left, below a third):
        // disabled; two more leave it at -2.
        assert_eq!(breaking_on - disabled_on, 2);
        assert_eq!(destroyed_on - breaking_on, 3, "its DeathDelay later");
        assert!(
            log.iter()
                .filter(|(tick_no, _, _)| *tick_no > disabled_on)
                .all(|(_, fired, _)| *fired == 0),
            "a disabled ship stops firing"
        );
        assert!(
            log.iter()
                .take(disabled_on as usize)
                .all(|(_, fired, _)| *fired == 1)
        );
        assert_eq!(ships[1].condition, Condition::Destroyed);
        assert!(ships[1].reserves.armor.now < 0.0);
        assert_eq!(ships[0].condition, Condition::Intact, "B's shots flew away");
    }

    #[test]
    fn a_ship_without_a_death_delay_is_destroyed_as_it_breaks_up() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = 0.0;
        let mut ships = [target];
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        assert!(matches!(events[0], CombatEvent::BreakingUp { ship: B, .. }));
        assert!(matches!(events[1], CombatEvent::Destroyed { ship: B, .. }));
        assert_eq!(ships[0].condition, Condition::Destroyed);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.take_events(), [], "destroyed once");
    }

    #[test]
    fn a_ship_that_holds_no_armour_never_breaks_up() {
        let mut combat = Combat::default();
        let mut armourless = Ship::at(2, 0.0, 0.0);
        armourless.reserves.armor = Gauge::full(0.0);
        let mut ships = [armourless];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact);
        assert_eq!(combat.take_events(), []);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact, "none to lose");
        ships[0].reserves.armor = Gauge { now: 0.0, max: 0.5 };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert!(matches!(ships[0].condition, Condition::Destroyed));
    }

    #[test]
    fn a_long_death_delay_ends_in_the_huge_explosion() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = -1.0;
        target.hull.death_delay = 60;
        let mut ships = [target];
        for _ in 0..=60 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let events = combat.take_events();
        assert!(
            matches!(
                events.last(),
                Some(CombatEvent::Destroyed { huge: true, .. })
            ),
            "{events:?}"
        );
    }

    /// Says every ship is `disabled`, recording what it was asked.
    #[derive(Debug, Default)]
    struct Recording {
        disabled: bool,
        asked: RefCell<Vec<(Gauge, HullSpec)>>,
    }

    impl DisableRule for Recording {
        fn disabled(&self, armor: Gauge, hull: &HullSpec) -> bool {
            self.asked.borrow_mut().push((armor, *hull));
            self.disabled
        }
    }

    #[test]
    fn the_disable_rule_given_decides_and_is_asked_of_every_ship_still_whole() {
        let mut combat = Combat::default();
        let mut tough = Ship::at(2, 50.0, 0.0);
        tough.hull.tough = true;
        tough.reserves.armor.now = 29.0;
        let mut dying = Ship::at(3, 90.0, 0.0);
        dying.condition = Condition::Dying { ticks_left: 5 };
        let mut ships = [Ship::at(1, 0.0, 0.0), tough, dying];
        let rule = Recording {
            disabled: true,
            ..Recording::default()
        };
        tick(&mut combat, &mut ships, &rule);
        let asked = rule.asked.take();
        assert_eq!(asked.len(), 2, "not of the dying");
        assert_eq!(
            asked[1].0,
            Gauge {
                now: 29.0,
                max: 30.0
            }
        );
        assert!(asked[1].1.tough);
        assert_eq!(
            about(&combat.take_events(), A),
            [CombatEvent::Disabled { ship: A }]
        );
        assert_eq!(ships[1].condition, Condition::Disabled);
        tick(&mut combat, &mut ships, &rule);
        assert_eq!(combat.take_events(), [], "disabled once");
        let well = Recording::default();
        tick(&mut combat, &mut ships, &well);
        assert_eq!(ships[1].condition, Condition::Intact, "as the rule says");
        assert_eq!(combat.take_events(), []);
    }

    #[test]
    fn shields_regenerate_to_their_most_and_stop() {
        let mut combat = Combat::default();
        let mut ship = Ship::at(1, 0.0, 0.0);
        ship.reserves.shield.now = 5.0;
        ship.reserves.armor.now = 20.0;
        ship.shield_regen = 2.0;
        let mut ships = [ship];
        let mut shields = Vec::new();
        for _ in 0..4 {
            tick(&mut combat, &mut ships, &NovaDisable);
            shields.push(ships[0].reserves.shield.now);
        }
        assert_eq!(shields, [7.0, 9.0, 10.0, 10.0]);
        ships[0].reserves.shield.now = 12.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.shield.now, 12.0, "over-full, kept");
        assert_eq!(ships[0].reserves.armor.now, 20.0, "no ArmorRech, no armour");
    }

    #[test]
    fn armour_regenerates_at_its_rate_but_not_while_disabled_and_shields_still_do() {
        let mut combat = Combat::default();
        let mut ship = Ship::at(1, 0.0, 0.0);
        ship.reserves.shield.now = -1.0;
        ship.reserves.armor.now = 28.5;
        ship.shield_regen = 0.5;
        ship.armor_regen = 1.0;
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            (ships[0].reserves.shield.now, ships[0].reserves.armor.now),
            (-0.5, 29.5)
        );
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].reserves.armor.now, 30.0, "up to its most");
        let disabling = Recording {
            disabled: true,
            ..Recording::default()
        };
        ships[0].reserves.armor.now = 5.0;
        tick(&mut combat, &mut ships, &disabling);
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(ships[0].reserves.armor.now, 5.0, "not while disabled");
        assert_eq!(ships[0].reserves.shield.now, 1.0, "shields still do");
        ships[0].condition = Condition::Dying { ticks_left: 9 };
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(ships[0].reserves.shield.now, 1.0, "nor while breaking up");
    }

    /// A 200-pixel beam lasting 3 ticks, firing every 10, doing 5 energy
    /// damage.
    fn laser() -> WeaponRecord {
        WeaponRecord {
            guidance: 0,
            count: 3,
            reload: 10,
            beam_length: 200,
            energy_dmg: 5,
            ..weapon(146)
        }
    }

    #[test]
    fn a_beam_damages_the_ship_it_touches_every_tick_it_lasts() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()), target];
        let mut shields = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            shields.push(ships[1].reserves.shield.now);
        }
        assert_eq!(shields, [95.0, 90.0, 85.0, 85.0, 85.0]);
        assert_eq!(combat.beams(), [], "over");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_beam_explodes_where_it_meets_the_ship_every_tick_it_touches_it() {
        let flamer = WeaponRecord {
            explod_type: 0,
            ..laser()
        };
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(flamer), target];
        let mut explosions = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            explosions.push(
                combat
                    .take_events()
                    .into_iter()
                    .filter_map(|event| match event {
                        CombatEvent::Exploded { at, explosion } => Some((at, explosion)),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let boom = Explosion {
            boom: BoomId(128),
            extra: false,
        };
        let edge = Vec2::new(84.0, 0.0);
        for (tick_no, exploded) in explosions.iter().enumerate() {
            if tick_no < 3 {
                assert_eq!(exploded.len(), 1, "tick {tick_no}: {exploded:?}");
                let (at, explosion) = exploded[0];
                assert!((at - edge).length() < 1e-3, "tick {tick_no}: {at:?}");
                assert_eq!(explosion, boom);
            } else {
                assert_eq!(exploded, &[], "tick {tick_no}: the beam is over");
            }
        }
    }

    #[test]
    fn a_beam_follows_its_firer_and_goes_with_it() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser())];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].state.position = Vec2::new(5.0, 6.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams()[0].start, Vec2::new(5.0, 6.0));
        ships[0].condition = Condition::Disabled;
        tick(
            &mut combat,
            &mut ships,
            &Recording {
                disabled: true,
                ..Recording::default()
            },
        );
        assert_eq!(combat.beams(), [], "its firer no longer intact");
        let mut combat = Combat::default();
        tick(
            &mut combat,
            &mut [Ship::at(1, 0.0, 0.0).armed(laser())],
            &NovaDisable,
        );
        tick(&mut combat, &mut [Ship::at(2, 0.0, 0.0)], &NovaDisable);
        assert_eq!(combat.beams(), [], "its firer gone");
    }

    #[test]
    fn a_hit_explodes_and_blasts_the_ships_around_it() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            Ship::at(3, 100.0, 25.0),
        ];
        let mut events = Vec::new();
        for _ in 0..6 {
            tick(&mut combat, &mut ships, &NovaDisable);
            events.extend(combat.take_events());
            ships[0].trigger = Trigger::default();
        }
        let explosions: Vec<_> = events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { at, explosion } => Some((at, explosion)),
                _ => None,
            })
            .collect();
        assert_eq!(explosions.len(), 1, "{events:?}");
        let (at, explosion) = explosions[0];
        assert!(
            (at.x - 84.0).abs() < 1e-3 && at.y.abs() < 1e-3,
            "at its edge: {at:?}"
        );
        assert_eq!(explosion.boom, BoomId(128));
        assert_eq!(ships[1].reserves.shield.now, 0.0, "hit directly");
        assert_eq!(ships[2].reserves.shield.now, 0.0, "blasted");
        assert_eq!(ships[2].reserves.armor.now, 26.0);
        assert_eq!(ships[0].reserves.shield.now, 10.0, "out of the blast");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_shot_detonates_at_the_end_of_its_life_if_its_weapon_says_so() {
        let flak = |flags| WeaponRecord {
            count: 3,
            speed: 1000,
            explod_type: 2,
            blast_radius: 40,
            flags,
            ..blaster()
        };
        for (flags, blasted) in [(0x8000, true), (0, false)] {
            let mut combat = Combat::default();
            let mut ships = [
                Ship::at(1, 0.0, 0.0).armed(flak(flags)),
                Ship::at(3, 30.0, 35.0),
            ];
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = Trigger::default();
            for _ in 0..2 {
                tick(&mut combat, &mut ships, &NovaDisable);
            }
            assert_eq!(combat.shots(), [], "{flags:#x}");
            let exploded = combat
                .take_events()
                .into_iter()
                .any(|event| matches!(event, CombatEvent::Exploded { .. }));
            assert_eq!(exploded, blasted, "{flags:#x}");
            assert_eq!(ships[1].reserves.shield.now < 10.0, blasted, "{flags:#x}");
        }
    }

    #[test]
    fn diagnostics_are_taken_once() {
        let mining = WeaponRecord {
            flags2: 0x8000,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(mining)];
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(combat.take_diagnostics().len(), 1);
        assert_eq!(combat.take_diagnostics(), []);
    }

    #[test]
    fn clearing_takes_the_shots_and_beams_away() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, 500.0).armed(laser()),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!((combat.shots().len(), combat.beams().len()), (1, 1));
        combat.clear();
        assert_eq!((combat.shots().len(), combat.beams().len()), (0, 0));
        let _ = C;
    }
}
