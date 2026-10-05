//! Combat: ships firing weapons, shots and beams flying and hitting, and
//! the damage that disables and destroys ships.
//!
//! - [`weapon`]: a weapon's [`WeaponSpec`](weapon::WeaponSpec), its `wëap`
//!   in the simulation's units.
//! - [`aim`]: which way each guidance fires, and at what: the bearing, the
//!   lead angle, the turrets' arcs and blind spots.
//! - [`armament`]: a ship's [`Armament`](armament::Armament), firing its
//!   weapons on a [`Trigger`](armament::Trigger) and its point defence,
//!   and the [`Arsenal`](armament::Arsenal) armaments are built from.
//! - [`beam`]: a [`Beam`](beam::Beam), held on its firer and hitting the
//!   nearest ship along it.
//! - [`damage`]: what a [`Hit`](damage::Hit) does to a ship's reserves,
//!   and which ships a [`Blast`](damage::Blast) reaches.
//! - [`defence`]: point defence, which missiles it engages, as the
//!   [`PointDefenceRule`](defence::PointDefenceRule) port says, and what
//!   its hits do to them.
//! - [`flags`]: what the simulation does with each documented weapon flag,
//!   and which of a weapon's are not done yet.
//! - [`hull`]: a ship type's [`HullSpec`](hull::HullSpec), its
//!   [`Condition`](hull::Condition), and the [`DisableRule`](hull::DisableRule)
//!   port with Nova's [`NovaDisable`](hull::NovaDisable).
//! - [`projectile`]: a [`Shot`](projectile::Shot) in flight, homing ones
//!   steering at their target, and the [`Target`](projectile::Target)s it
//!   may hit.
//! - [`report`]: the [`SimDiagnostic`](report::SimDiagnostic)s a session
//!   reports once each about game data it does not handle yet.
//! - [`submunition`]: the shots a shot releases on a hit and at the end
//!   of its life.
//!
//! [`Combat`] holds the shots and beams in flight. Each tick
//! ([`Combat::tick`]) runs over every ship in the fight, each a
//! [`Fighter`] with the ship it targets, by the fight's [`Rules`], in this
//! order:
//!
//! 1. Each ship fires the weapons its trigger holds that are ready, each
//!    as its guidance aims it at the ship's target ([`aim`]); a turret
//!    that cannot fire spends nothing.
//! 2. Each ship's point defence fires at the missile it picks, with no
//!    target needed ([`defence`]).
//! 3. Every reload timer counts down a tick.
//! 4. Shots fly a tick, homing ones steering at their target, and beams
//!    follow their firers, each held ahead, on its target or on its
//!    missile; a beam whose firer is gone or no longer intact, or whose
//!    target or missile is gone, goes with it.
//! 5. A point-defence shot that passed within
//!    [`INTERCEPT_RADIUS`](defence::INTERCEPT_RADIUS) of a missile the
//!    rules call hostile is spent on it; a missile hit after its
//!    durability is used up is shot down.
//! 6. A point-defence beam hits the missile it is held on the same way.
//! 7. Hits, blasts and expiries are resolved: a shot that hits a ship
//!    damages it, explodes and blasts the ships around it, releases its
//!    sub-munitions at that ship, and is gone; a shot at the end of its
//!    life detonates (blasting the same way) or vanishes, and releases its
//!    sub-munitions at its own target; a beam damages the nearest ship
//!    along it, every tick, until its life is over.
//! 8. Conditions follow the damage: a ship with no armour left (of the
//!    armour it holds) starts breaking up, and any other that holds armour
//!    is disabled, or not, as the [`DisableRule`](hull::DisableRule) says;
//!    a ship that holds none is never disabled.
//! 9. A ship breaking up counts down its `DeathDelay`, then is destroyed.
//! 10. Shields regenerate on an intact or disabled ship, armour only on an
//!     intact one, each up to what it holds.
//!
//! Each hit on a ship (a shot's, a beam's, a blast's) is kept as a
//! [`Strike`] by the ship whose it was, with the shield and armour it
//! took; the last on a ship disabled or breaking up that tick says so
//! ([`Downed`]). The session takes them ([`Combat::take_strikes`]) for
//! the player's crimes and the NPCs' answers.
//!
//! Each step is reported as a [`CombatEvent`] for the view to show, and
//! each unimplemented weapon feature once as a
//! [`SimDiagnostic`](report::SimDiagnostic). Nothing here is saved: the
//! shots, their targets, durability and generations live only in flight.

pub mod aim;
pub mod armament;
pub mod beam;
pub mod damage;
pub mod defence;
pub mod flags;
pub mod hull;
pub mod projectile;
pub mod report;
pub mod submunition;
pub mod weapon;

use armament::{Armament, Arsenal, Launch, Rounds, Trigger};
use beam::{Aiming, Beam};
use damage::{Blast, Hit};
use defence::{INTERCEPT_RADIUS, PointDefenceRule, Side};
use hull::{Condition, DisableRule, HullSpec};
use projectile::{Shot, ShotId, Target, contact};
use report::{Reports, SimDiagnostic};
use weapon::{Explosion, Guidance};

use crate::catalog::{GovtId, ShipId, WeaponId};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::legal::{CrimeGains, LegalCode, NovaLaw};
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
    /// A ship fired a weapon, or a shot of its released sub-munitions.
    Fired {
        /// The ship.
        ship: ShipRef,
        /// The weapon.
        weapon: WeaponId,
        /// Where it was fired from: the ship, or the shot releasing them.
        at: Vec2,
    },
    /// Point defence destroyed a missile.
    ShotDown {
        /// Where.
        at: Vec2,
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
        /// The explosion's size ([`HullSpec::death_size`]): how far its
        /// extra explosions scatter, and how many there are.
        size: f32,
    },
}

/// How a strike left the ship it hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Downed {
    /// It disabled the ship.
    Disabled,
    /// It took the last of the ship's armour: the ship is breaking up.
    BreakingUp,
}

/// A hit on a ship: who made it, what it took, and whether it downed
/// the ship.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    /// The ship hit.
    pub ship: ShipRef,
    /// The ship whose shot, beam or blast it was.
    pub by: ShipRef,
    /// The shield and armour it took.
    pub damage: f32,
    /// How it left the ship, for the last hit on a ship whose condition
    /// changed that tick; none otherwise.
    pub downed: Option<Downed>,
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
    /// Its government, or `None` for an independent ship or the player.
    pub govt: Option<GovtId>,
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
    /// The ship it targets, if any.
    pub target: Option<ShipRef>,
    /// Its shield, armour and fuel.
    pub reserves: &'a mut Reserves,
    /// How it is holding up.
    pub condition: &'a mut Condition,
    /// Its weapons.
    pub armament: &'a mut Armament,
    /// Its rounds of ammunition.
    pub rounds: &'a mut dyn Rounds,
}

/// The rules a fight is fought by: when a ship is disabled, which
/// missiles point defence engages, and what the player's crimes do to
/// its legal record.
#[derive(Clone, Copy, Debug)]
pub struct Rules<'a> {
    /// When a ship is disabled.
    pub disable: &'a dyn DisableRule,
    /// Which missiles point defence engages.
    pub defence: &'a dyn PointDefenceRule,
    /// What the player's crimes do to its legal record.
    pub law: &'a dyn LegalCode,
}

impl Default for Rules<'static> {
    /// Nova's: [`NovaDisable`](hull::NovaDisable), point defence by
    /// [`Allegiance`](defence::Allegiance), and [`NovaLaw`].
    fn default() -> Self {
        Self {
            disable: &hull::NovaDisable,
            defence: &defence::Allegiance,
            law: &NovaLaw {
                gains: CrimeGains::Engine,
            },
        }
    }
}

/// The shots and beams in flight, and what the fight has reported.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    shots: Vec<Shot>,
    beams: Vec<Beam>,
    events: Vec<CombatEvent>,
    strikes: Vec<Strike>,
    reports: Reports,
    /// The next shot's number.
    next_shot: u32,
}

impl Combat {
    /// Advances the fight among `fighters` a tick (see the module docs),
    /// by `rules`, with the relations between governments in `govts`,
    /// each sub-munition's weapon read from `arsenal` and each shot's
    /// inaccuracy and spread drawn on `chance`.
    pub fn tick(
        &mut self,
        fighters: &mut [Fighter],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
        chance: &mut (impl Chance + ?Sized),
    ) {
        // A `dyn` source over the caller's, which may be unsized.
        let mut source = &mut *chance;
        let chance: &mut dyn Chance = &mut source;
        let targets: Vec<Target> = fighters.iter().map(Fighter::as_target).collect();
        self.fire(fighters, &targets, chance);
        self.defend(fighters, rules.defence, govts, chance);
        for fighter in fighters.iter_mut() {
            fighter.armament.reload();
        }
        let was: Vec<Vec2> = self
            .shots
            .iter_mut()
            .map(|shot| shot.step(&targets))
            .collect();
        let shots = &self.shots;
        self.beams.retain_mut(|beam| {
            let firer = fighters.iter().find(|fighter| {
                fighter.ship == beam.firer && *fighter.condition == Condition::Intact
            });
            firer.is_some_and(|firer| beam.follow(&firer.state, &targets, shots))
        });
        let mut gone = vec![false; self.shots.len()];
        self.intercept(&was, &mut gone, rules.defence, govts);
        self.hold_on_missiles(&mut gone);
        let first_strike = self.strikes.len();
        self.resolve(fighters, &targets, &was, &gone, arsenal, chance);
        for fighter in fighters.iter_mut() {
            let downed = self.update_condition(fighter, rules.disable);
            let last = self.strikes[first_strike..]
                .iter_mut()
                .rev()
                .find(|strike| strike.ship == fighter.ship);
            if let (Some(downed), Some(last)) = (downed, last) {
                last.downed = Some(downed);
            }
        }
        for fighter in fighters.iter_mut() {
            regenerate(fighter);
        }
    }

    /// The next shot's number.
    fn next_id(&mut self) -> ShotId {
        let id = ShotId(self.next_shot);
        self.next_shot += 1;
        id
    }

    /// Puts `launch` in flight from `side`, at `from`: a beam held on the
    /// missile `quarry`, on its target, or ahead, or a shot of its
    /// government.
    fn launch(&mut self, side: Side, from: &ShipState, launch: Launch, quarry: Option<ShotId>) {
        let Side { ship, fleet, govt } = side;
        self.events.push(CombatEvent::Fired {
            ship,
            weapon: launch.weapon.id,
            at: from.position,
        });
        if launch.weapon.is_beam() {
            let aiming = match (quarry, launch.target) {
                (Some(missile), _) => Aiming::Shot(missile),
                (None, Some(target)) => Aiming::Ship(target),
                (None, None) => Aiming::Ahead,
            };
            let beam = Beam::launch(launch.weapon, ship, fleet, from, launch.heading, aiming);
            self.beams.push(beam);
        } else {
            let shot = Shot {
                id: self.next_id(),
                target: launch.target,
                govt,
                ..Shot::launch(launch.weapon, ship, fleet, from, launch.heading)
            };
            self.shots.push(shot);
        }
    }

    /// Step 1: each ship fires what its trigger holds, aimed at its target
    /// among `targets` while that can be hit.
    fn fire(&mut self, fighters: &mut [Fighter], targets: &[Target], chance: &mut dyn Chance) {
        for fighter in fighters.iter_mut() {
            let (state, hull) = (fighter.state, fighter.hull);
            let target = fighter.target.and_then(|ship| {
                targets
                    .iter()
                    .find(|target| target.ship == ship && target.condition.hittable())
            });
            let launches = fighter.armament.fire(
                fighter.trigger,
                *fighter.condition,
                fighter.rounds,
                &mut fighter.reserves.fuel,
                chance,
                &mut self.reports,
                &mut |spec| aim::aim(spec, &state, &hull, target),
            );
            for launch in launches {
                self.launch(fighter.side(), &state, launch, None);
            }
        }
    }

    /// Step 2: each ship's point defence fires at the missile it picks,
    /// as `rule` says which are hostile with the relations in `govts`.
    fn defend(
        &mut self,
        fighters: &mut [Fighter],
        rule: &dyn PointDefenceRule,
        govts: &Governments,
        chance: &mut dyn Chance,
    ) {
        for fighter in fighters.iter_mut() {
            let (state, hull) = (fighter.state, fighter.hull);
            let side = fighter.side();
            let shots = &self.shots;
            let fired = fighter.armament.fire_point_defence(
                *fighter.condition,
                fighter.rounds,
                &mut fighter.reserves.fuel,
                chance,
                &mut self.reports,
                &mut |spec| {
                    let missile = defence::choose(side, &state, &hull, spec, shots, rule, govts)?;
                    Some((aim::bearing(state.position, missile.position), missile.id))
                },
            );
            if let Some((launch, missile)) = fired {
                self.launch(side, &state, launch, Some(missile));
            }
        }
    }

    /// Step 5: each point-defence shot, having flown from `was`, meets
    /// the first missile it can engage ([`defence::engageable`], as `rule`
    /// says which are hostile with the relations in `govts`) that it
    /// passes within [`INTERCEPT_RADIUS`] of, and is spent on it; a
    /// missile destroyed is shot down. What is spent or destroyed is
    /// marked `gone`.
    fn intercept(
        &mut self,
        was: &[Vec2],
        gone: &mut [bool],
        rule: &dyn PointDefenceRule,
        govts: &Governments,
    ) {
        for pd in 0..self.shots.len() {
            if gone[pd] || self.shots[pd].weapon.guidance != Guidance::PointDefence {
                continue;
            }
            let defender = &self.shots[pd];
            let side = Side::of(defender);
            let met = self
                .shots
                .iter()
                .enumerate()
                .filter(|&(missile, shot)| {
                    !gone[missile] && defence::engageable(side, shot, rule, govts)
                })
                .filter_map(|(missile, shot)| {
                    let from = was[pd] - was[missile];
                    let to = defender.position - shot.position;
                    Some((contact(from, to, Vec2::ZERO, INTERCEPT_RADIUS)?, missile))
                })
                .min_by(|(a, _), (b, _)| a.total_cmp(b));
            if let Some((t, missile)) = met {
                gone[pd] = true;
                let damage = defence::pd_damage(&self.shots[pd].weapon);
                if self.shots[missile].take_pd_hit(damage) {
                    gone[missile] = true;
                    let flown = self.shots[missile].position - was[missile];
                    self.events.push(CombatEvent::ShotDown {
                        at: was[missile] + flown * t,
                    });
                }
            }
        }
    }

    /// Step 6: each point-defence beam hits the missile it is held on; a
    /// missile destroyed is shot down and marked `gone`, and the beam goes
    /// with it.
    fn hold_on_missiles(&mut self, gone: &mut [bool]) {
        for beam in &self.beams {
            let Aiming::Shot(id) = beam.aiming else {
                continue;
            };
            let Some(missile) = self.shots.iter().position(|shot| shot.id == id) else {
                continue;
            };
            if !gone[missile] && self.shots[missile].take_pd_hit(defence::pd_damage(&beam.weapon)) {
                gone[missile] = true;
                self.events.push(CombatEvent::ShotDown {
                    at: self.shots[missile].position,
                });
            }
        }
        let shots = &self.shots;
        self.beams.retain(|beam| match beam.aiming {
            Aiming::Shot(id) => shots
                .iter()
                .zip(gone.iter())
                .any(|(shot, gone)| shot.id == id && !gone),
            _ => true,
        });
    }

    /// Step 7: hits, blasts, expiries and sub-munitions, the shots having
    /// flown from `was`, those `gone` taken out first; each sub-munition's
    /// weapon is read from `arsenal`, its spread drawn on `chance`.
    fn resolve(
        &mut self,
        fighters: &mut [Fighter],
        targets: &[Target],
        was: &[Vec2],
        gone: &[bool],
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) {
        // Each ship hit, by whom, and the hit.
        let mut hits: Vec<(ShipRef, ShipRef, Hit)> = Vec::new();
        let mut kept = Vec::with_capacity(self.shots.len());
        let mut released: Vec<(Shot, Option<ShipRef>)> = Vec::new();
        let flown = self.shots.drain(..).zip(was).zip(gone);
        for ((mut shot, &from), &gone) in flown {
            if gone {
                continue;
            }
            let (blast, subs) = if let Some((ship, at)) = shot.hit(from, targets) {
                hits.push((ship, shot.firer, Hit::of(&shot.weapon)));
                shot.position = at;
                (Some(shot.blast(Some(ship))), Some(Some(ship)))
            } else if shot.expired() {
                let blast = shot.weapon.detonates().then(|| shot.blast(None));
                (blast, shot.weapon.subs_on_expiry().then_some(shot.target))
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
                let reached = reached(&blast, targets);
                hits.extend(
                    reached
                        .into_iter()
                        .map(|ship| (ship, shot.firer, Hit::of(&shot.weapon))),
                );
            }
            if let Some(target) = subs {
                released.push((shot, target));
            }
        }
        self.shots = kept;
        for (parent, target) in released {
            self.release(&parent, target, targets, arsenal, chance);
        }
        for beam in &mut self.beams {
            if let Some((ship, at)) = beam.hit(targets) {
                hits.push((ship, beam.firer, Hit::of(&beam.weapon)));
                if let Some(explosion) = beam.weapon.explosion {
                    self.events.push(CombatEvent::Exploded { at, explosion });
                }
            }
            beam.age();
        }
        self.beams.retain(|beam| !beam.expired());
        for (ship, by, hit) in hits {
            if let Some(fighter) = fighters.iter_mut().find(|fighter| fighter.ship == ship) {
                let before = taken(fighter.reserves);
                damage::apply(fighter.reserves, hit);
                self.strikes.push(Strike {
                    ship,
                    by,
                    damage: before - taken(fighter.reserves),
                    downed: None,
                });
            }
        }
    }

    /// Puts in flight the sub-munitions `parent` releases at `target`
    /// among `targets`, their weapon read from `arsenal` (none when it
    /// cannot be read), heard from where the parent is. Released, they
    /// report their weapon's unimplemented flags as a weapon fired does;
    /// of a guidance not flown yet, they report it and none fly.
    fn release(
        &mut self,
        parent: &Shot,
        target: Option<ShipRef>,
        targets: &[Target],
        arsenal: &Arsenal,
        chance: &mut dyn Chance,
    ) {
        let Some(sub) = parent
            .weapon
            .submunitions
            .and_then(|subs| arsenal.weapon(subs.weapon))
        else {
            return;
        };
        let shots = submunition::release(parent, sub, target, targets, chance);
        if shots.is_empty() {
            return;
        }
        if let Guidance::Other(guidance) = sub.guidance {
            self.reports.report(SimDiagnostic::UnimplementedGuidance {
                weapon: sub.id,
                guidance,
            });
            return;
        }
        self.reports.fired(sub);
        self.events.push(CombatEvent::Fired {
            ship: parent.firer,
            weapon: sub.id,
            at: parent.position,
        });
        for shot in shots {
            let id = self.next_id();
            self.shots.push(Shot { id, ..shot });
        }
    }

    /// Steps 8 and 9: `fighter`'s condition follows its damage, as `rule`
    /// says, and a ship breaking up counts down to its destruction. How
    /// it was downed, if it was this tick.
    fn update_condition(
        &mut self,
        fighter: &mut Fighter,
        rule: &dyn DisableRule,
    ) -> Option<Downed> {
        let ship = fighter.ship;
        let at = fighter.state.position;
        let mut downed = None;
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
                downed = Some(Downed::BreakingUp);
            }
            Condition::Intact | Condition::Disabled => {
                let armor = fighter.reserves.armor;
                let disabled = holds_armour(armor) && rule.disabled(armor, &fighter.hull);
                if disabled && *fighter.condition == Condition::Intact {
                    self.events.push(CombatEvent::Disabled { ship });
                    downed = Some(Downed::Disabled);
                }
                *fighter.condition = if disabled {
                    Condition::Disabled
                } else {
                    Condition::Intact
                };
                return downed;
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
                    size: fighter.hull.death_size(),
                });
            }
            Condition::Dying { ticks_left } => {
                *fighter.condition = Condition::Dying {
                    ticks_left: ticks_left - 1,
                };
            }
            _ => {}
        }
        downed
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

    /// The strikes made since they were last taken, in order; taking them
    /// empties the list.
    pub fn take_strikes(&mut self) -> Vec<Strike> {
        std::mem::take(&mut self.strikes)
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

impl Fighter<'_> {
    /// The ship's side: itself, its fleet and its government.
    fn side(&self) -> Side {
        Side {
            ship: self.ship,
            fleet: self.fleet,
            govt: self.govt,
        }
    }

    /// The ship as shots and beams see it.
    fn as_target(&self) -> Target {
        Target {
            ship: self.ship,
            fleet: self.fleet,
            position: self.state.position,
            velocity: self.state.velocity,
            radius: self.hull.hit_radius,
            condition: *self.condition,
        }
    }
}

/// The shield and armour `reserves` hold, together: what a hit takes is
/// how much less this is after it.
fn taken(reserves: &Reserves) -> f32 {
    reserves.shield.now + reserves.armor.now
}

/// Whether `armor` is gone: at or below none, on a ship that holds any.
fn armour_gone(armor: Gauge) -> bool {
    armor.now <= 0.0 && holds_armour(armor)
}

/// Whether a ship with `armor` holds any. A ship type with no armour at
/// all (stock `shïp` 895, plug-in or test data) has none to lose, so it
/// never breaks up, and the [`DisableRule`] is never asked of it, so it is
/// never disabled either.
fn holds_armour(armor: Gauge) -> bool {
    armor.max > 0.0
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

/// Step 10: `fighter`'s shields regenerate unless it is breaking up or
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
    use crate::combat::defence::Side;
    use crate::combat::hull::NovaDisable;
    use crate::combat::weapon::WeaponSpec;
    use crate::testkit::weapon;

    /// A ship in a test fight, owning what a fighter borrows.
    struct Ship {
        id: ShipRef,
        fleet: ShipRef,
        govt: Option<GovtId>,
        target: Option<ShipRef>,
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
                fleet: ShipRef::Npc(NpcId(id)),
                govt: None,
                target: None,
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
                    only: None,
                },
                ..self
            }
        }

        fn fighter(&mut self) -> Fighter<'_> {
            Fighter {
                ship: self.id,
                ship_type: ShipId(128),
                fleet: self.fleet,
                govt: self.govt,
                state: self.state,
                hull: self.hull,
                shield_regen: self.shield_regen,
                armor_regen: self.armor_regen,
                trigger: self.trigger,
                target: self.target,
                reserves: &mut self.reserves,
                condition: &mut self.condition,
                armament: &mut self.armament,
                rounds: &mut self.rounds,
            }
        }
    }

    fn tick(combat: &mut Combat, ships: &mut [Ship], rule: &dyn DisableRule) {
        let rules = Rules {
            disable: rule,
            ..Rules::default()
        };
        tick_with(combat, ships, &Arsenal::default(), rules);
    }

    fn tick_with(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal, rules: Rules) {
        tick_among(combat, ships, arsenal, &Governments::default(), rules);
    }

    fn tick_among(
        combat: &mut Combat,
        ships: &mut [Ship],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(&mut fighters, arsenal, govts, rules, &mut NeverFires);
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
        ships[0].govt = Some(GovtId(140));
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(128),
                at: Vec2::ZERO
            }]
        );
        assert_eq!(combat.take_events(), [], "taken");
        assert_eq!(combat.shots().len(), 1);
        assert_eq!(combat.shots()[0].govt, Some(GovtId(140)), "its firer's");
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
                size: 0.0
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
    fn a_ship_that_holds_no_armour_is_never_disabled_by_a_hit() {
        let mut combat = Combat::default();
        let mut armourless = Ship::at(2, 100.0, 0.0);
        armourless.reserves.shield.now = 0.0;
        armourless.reserves.armor = Gauge::full(0.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), armourless];
        let mut events = Vec::new();
        for _ in 0..20 {
            tick(&mut combat, &mut ships, &NovaDisable);
            events.extend(combat.take_events());
        }
        assert!(
            events
                .iter()
                .any(|event| matches!(event, CombatEvent::Fired { ship: A, .. })),
            "{events:?}"
        );
        assert!(
            ships[1].reserves.shield.now < 0.0,
            "hit with its shields down"
        );
        assert_eq!(ships[1].reserves.armor, Gauge::full(0.0), "none to take");
        assert_eq!(ships[1].condition, Condition::Intact);
        assert_eq!(about(&events, B), []);
        let disabling = Recording {
            disabled: true,
            ..Recording::default()
        };
        ships[1].reserves.armor.now = -1.0;
        tick(&mut combat, &mut ships, &disabling);
        assert_eq!(ships[1].condition, Condition::Intact, "whatever the rule");
        assert_eq!(disabling.asked.take().len(), 1, "only of the armoured");
        assert_eq!(about(&combat.take_events(), B), []);
    }

    #[test]
    fn a_heavy_ship_is_destroyed_in_an_explosion_its_mass_sizes() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = -1.0;
        target.hull.death_delay = 60;
        target.hull.mass = 400.0;
        let mut ships = [target];
        for _ in 0..=60 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let events = combat.take_events();
        assert!(
            matches!(
                events.last(),
                Some(CombatEvent::Destroyed { size, .. }) if *size == 80.0
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
    fn a_blast_hurts_a_disabled_ship_and_spares_a_dying_one() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let mut combat = Combat::default();
        let mut dying = Ship::at(3, 100.0, 25.0);
        dying.condition = Condition::Dying { ticks_left: 50 };
        let mut disabled = Ship::at(4, 100.0, -25.0);
        disabled.condition = Condition::Disabled;
        disabled.reserves.armor.now = 5.0;
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(shell),
            Ship::at(2, 100.0, 0.0),
            dying,
            disabled,
        ];
        for _ in 0..6 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = Trigger::default();
        }
        assert_eq!(ships[1].reserves.shield.now, 0.0, "hit directly");
        assert_eq!(
            ships[2].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "a dying ship is past hitting"
        );
        assert_eq!(
            (ships[3].reserves.shield.now, ships[3].reserves.armor.now),
            (0.0, 1.0),
            "a disabled ship is blasted"
        );
        assert_eq!(ships[3].condition, Condition::Disabled);
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

    // Guided weapons and turrets.

    impl Ship {
        /// The ship facing `heading`.
        fn facing(self, heading: f32) -> Self {
            Self {
                state: ShipState {
                    heading,
                    ..self.state
                },
                ..self
            }
        }

        /// The ship targeting `target`.
        fn targeting(self, target: ShipRef) -> Self {
            Self {
                target: Some(target),
                ..self
            }
        }
    }

    /// A turret of `guidance` firing every 5 ticks, 20 pixels a tick for
    /// 30, doing 5 mass and 10 energy damage.
    fn turret(guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            reload: 5,
            count: 30,
            speed: 2000,
            mass_dmg: 5,
            energy_dmg: 10,
            ..weapon(130)
        }
    }

    #[test]
    fn a_turret_destroys_a_target_behind_its_firer() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(turret(4)).targeting(B),
            Ship::at(2, -150.0, 0.0),
        ];
        for _ in 0..60 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(ships[1].condition, Condition::Destroyed);
        let mut idle = [
            Ship::at(1, 0.0, 0.0).armed(turret(4)),
            Ship::at(2, -150.0, 0.0),
        ];
        let mut combat = Combat::default();
        tick(&mut combat, &mut idle, &NovaDisable);
        assert_eq!(combat.take_events(), [], "no target, no fire");
        assert_eq!(combat.shots(), []);
    }

    #[test]
    fn a_front_quadrant_turret_without_a_target_fires_along_the_heading() {
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(turret(7)).facing(30.0)];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots().len(), 1);
        assert_eq!(combat.shots()[0].heading, 30.0);
        assert_eq!(combat.shots()[0].target, None);
    }

    #[test]
    fn a_turreted_beam_follows_its_target_and_ends_with_it() {
        let beam = WeaponRecord {
            guidance: 3,
            count: 10,
            beam_length: 300,
            energy_dmg: 1,
            ..weapon(142)
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(beam).targeting(B),
            Ship::at(2, 0.0, 100.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[0].trigger = Trigger::default();
        ships[1].state.position = Vec2::new(100.0, 0.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        let end = combat.beams()[0].end;
        assert!((end - Vec2::new(300.0, 0.0)).length() < 1e-3, "{end:?}");
        assert_eq!(ships[1].reserves.shield.now, 8.0, "hit both ticks");
        ships[1].condition = Condition::Dying { ticks_left: 9 };
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.beams(), [], "its target breaking up");
    }

    /// A missile 10 pixels a tick for 100 ticks, turning 7 degrees a tick,
    /// doing 20 mass and 10 energy damage, standing 4 of point defence.
    fn missile() -> WeaponRecord {
        WeaponRecord {
            guidance: 1,
            reload: 1000,
            count: 100,
            speed: 1000,
            guided_turn: 70,
            durability: 4,
            mass_dmg: 20,
            energy_dmg: 10,
            ..weapon(134)
        }
    }

    #[test]
    fn a_homing_missile_chases_and_hits_a_crossing_target() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(missile())
                .facing(0.0)
                .targeting(B),
            Ship::at(2, 0.0, -300.0),
        ];
        let mut hit = false;
        for _ in 0..100 {
            ships[1].state.velocity = Vec2::new(3.0, 0.0);
            ships[1].state.position = ships[1].state.position + ships[1].state.velocity;
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = Trigger::default();
            if ships[1].reserves.shield.now < 10.0 {
                hit = true;
                break;
            }
        }
        assert!(hit, "{:?}", combat.shots());
        assert_eq!(combat.shots(), [], "spent");
    }

    #[test]
    fn a_missile_whose_target_is_destroyed_flies_on_and_hits_nothing() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0)
                .armed(missile())
                .facing(90.0)
                .targeting(B),
            Ship::at(2, 0.0, 500.0),
            Ship::at(3, 400.0, 0.0),
        ];
        for _ in 0..10 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[0].trigger = Trigger::default();
        }
        assert_eq!(combat.shots()[0].target, Some(B), "fired at B");
        ships[1].condition = Condition::Destroyed;
        let mut flown = 0;
        while !combat.shots().is_empty() {
            tick(&mut combat, &mut ships, &NovaDisable);
            flown += 1;
        }
        assert_eq!(flown, 90, "flew its life");
        assert_eq!(
            ships[2].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "flown through"
        );
    }

    /// The Quad Light Blaster Turret: point defence firing every 5 ticks,
    /// 20 pixels a tick for 12, doing 1 mass and 4 energy damage (3 to a
    /// missile).
    fn quad() -> WeaponRecord {
        WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(133)
        }
    }

    /// Says every missile is `hostile`, recording what it was asked and
    /// the governments it was asked with.
    #[derive(Debug, Default)]
    struct Hostility {
        hostile: bool,
        asked: RefCell<Vec<(Side, Side)>>,
        govts: RefCell<Vec<Governments>>,
    }

    impl PointDefenceRule for Hostility {
        fn hostile(&self, defender: Side, firer: Side, govts: &Governments) -> bool {
            self.asked.borrow_mut().push((defender, firer));
            self.govts.borrow_mut().push(govts.clone());
            self.hostile
        }
    }

    /// The defender A at the centre with `defence`, no target and no
    /// trigger, and B 200 pixels above, facing it, firing a missile at
    /// it on the first tick.
    fn missile_attack(defence: WeaponRecord) -> [Ship; 2] {
        let mut defender = Ship::at(1, 0.0, 0.0).armed(defence).facing(0.0);
        defender.trigger = Trigger::default();
        [
            defender,
            Ship::at(2, 0.0, -200.0)
                .armed(WeaponRecord {
                    speed: 500,
                    ..missile()
                })
                .facing(180.0)
                .targeting(A),
        ]
    }

    /// The durability of the one missile in flight, if any.
    fn missile_durability(combat: &Combat) -> Option<f32> {
        let missiles: Vec<_> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(134))
            .collect();
        assert!(missiles.len() <= 1);
        missiles.first().map(|shot| shot.durability)
    }

    #[test]
    fn point_defence_shoots_down_a_missile_by_the_hit_after_its_durability_is_gone() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        ships[1].govt = Some(GovtId(140));
        let govts = Governments::new([crate::testkit::govt(140)]);
        let rule = Hostility {
            hostile: true,
            ..Hostility::default()
        };
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        let mut durabilities = Vec::new();
        let mut events = Vec::new();
        for _ in 0..30 {
            tick_among(&mut combat, &mut ships, &Arsenal::default(), &govts, rules);
            ships[1].trigger = Trigger::default();
            let new = combat.take_events();
            let defended = new
                .iter()
                .filter(|e| matches!(e, CombatEvent::Fired { ship: A, .. }))
                .count();
            let spent_before = combat
                .shots()
                .iter()
                .filter(|s| s.weapon.id == WeaponId(133))
                .count();
            events.extend(new);
            durabilities.push((missile_durability(&combat), defended, spent_before));
        }
        // 3 to a missile of 4: 4, 1, -2, then gone. Its shots meet it on
        // ticks 7, 11 and 15, each fired 5 ticks apart.
        let seen: Vec<Option<f32>> = durabilities.iter().map(|(d, _, _)| *d).collect();
        assert_eq!(&seen[..7], [Some(4.0); 7]);
        assert_eq!(&seen[7..11], [Some(1.0); 4]);
        assert_eq!(&seen[11..15], [Some(-2.0); 4]);
        assert_eq!(&seen[15..], [None; 15]);
        let downed: Vec<CombatEvent> = events
            .iter()
            .filter(|e| matches!(e, CombatEvent::ShotDown { .. }))
            .copied()
            .collect();
        assert_eq!(downed.len(), 1, "{events:?}");
        let CombatEvent::ShotDown { at } = downed[0] else {
            unreachable!()
        };
        assert!((at - Vec2::new(0.0, -120.0)).length() < 5.0, "{at:?}");
        let fired: Vec<usize> = durabilities.iter().map(|(_, f, _)| *f).collect();
        assert_eq!(
            &fired[..16],
            [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
        );
        assert_eq!(&fired[16..], [0; 14], "nothing left to shoot at");
        let pd_shots: Vec<usize> = durabilities.iter().map(|(_, _, s)| *s).collect();
        assert_eq!(pd_shots[6], 2, "two in flight");
        assert_eq!(pd_shots[7], 1, "the first spent on the missile");
        assert_eq!(
            ships[0].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "untouched"
        );
        let asked = rule.asked.take();
        let defender = Side {
            ship: A,
            fleet: A,
            govt: None,
        };
        let attacker = Side {
            ship: B,
            fleet: B,
            govt: Some(GovtId(140)),
        };
        assert!(asked.contains(&(defender, attacker)), "{asked:?}");
        assert!(rule.govts.take().iter().all(|asked| *asked == govts));
    }

    #[test]
    fn each_shot_in_flight_has_its_own_number() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 0.0, 500.0).armed(blaster()),
        ];
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        let mut ids: Vec<ShotId> = combat.shots().iter().map(|shot| shot.id).collect();
        assert_eq!(ids.len(), 6);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 6, "{:?}", combat.shots());
    }

    #[test]
    fn a_point_defence_shot_meets_only_a_hostile_homing_missile_it_can_target() {
        // B fires, together and along the same line at A: a shell, a
        // missile point defence cannot target, and one it can, in that
        // order.
        let shell = WeaponRecord {
            reload: 1000,
            count: 100,
            speed: 500,
            ..weapon(128)
        };
        let immune = WeaponRecord {
            flags: 0x0080,
            ..WeaponRecord {
                speed: 500,
                ..missile()
            }
        };
        let plain = WeaponRecord {
            guidance: 1,
            reload: 1000,
            count: 100,
            speed: 500,
            ..weapon(134)
        };
        let mut attacker = Ship::at(2, 0.0, -200.0).facing(180.0).targeting(A);
        attacker.armament = Armament::new(
            [
                WeaponRecord {
                    id: WeaponId(128),
                    ..shell
                },
                WeaponRecord {
                    id: WeaponId(135),
                    ..immune
                },
                WeaponRecord {
                    id: WeaponId(134),
                    ..plain
                },
            ]
            .iter()
            .map(|record| (WeaponSpec::new(record), 1)),
        );
        attacker.trigger = Trigger {
            primary: true,
            secondary: None,
            only: None,
        };
        let mut defender = Ship::at(1, 0.0, 0.0).armed(quad()).facing(0.0);
        defender.trigger = Trigger::default();
        let mut ships = [defender, attacker];
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..10 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = Trigger::default();
            events.extend(combat.take_events());
        }
        let flying: Vec<i16> = combat
            .shots()
            .iter()
            .map(|shot| shot.weapon.id.0)
            .filter(|&id| id != 133)
            .collect();
        assert_eq!(flying, [128, 135], "only 134 shot down: {events:?}");
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, CombatEvent::ShotDown { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn a_shot_of_no_point_defence_meets_no_missile() {
        let gun = WeaponRecord {
            guidance: -1,
            ..quad()
        };
        let mut combat = Combat::default();
        let mut ships = missile_attack(gun);
        ships[0].trigger = Trigger {
            primary: true,
            secondary: None,
            only: None,
        };
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = Trigger::default();
        }
        assert_eq!(missile_durability(&combat), Some(4.0), "flown through");
        assert!(
            !combat
                .take_events()
                .iter()
                .any(|event| matches!(event, CombatEvent::ShotDown { .. }))
        );
    }

    #[test]
    fn point_defence_spares_a_missile_the_rule_does_not_call_hostile() {
        let mut combat = Combat::default();
        let mut ships = missile_attack(quad());
        let rule = Hostility::default();
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        for _ in 0..3 {
            tick_with(&mut combat, &mut ships, &Arsenal::default(), rules);
            ships[1].trigger = Trigger::default();
        }
        assert_eq!(combat.shots().len(), 1, "only the missile");
        assert_eq!(rule.asked.take().len(), 3, "asked each tick");
    }

    /// Calls hostile only a missile `ship` fired.
    #[derive(Debug)]
    struct FiredBy(ShipRef);

    impl PointDefenceRule for FiredBy {
        fn hostile(&self, _defender: Side, firer: Side, _govts: &Governments) -> bool {
            firer.ship == self.0
        }
    }

    #[test]
    fn a_point_defence_shot_flies_through_a_missile_the_rule_does_not_call_hostile() {
        // B and C, each its own fleet, fire missiles down the same line at
        // A, C's nearer, and C moves off the line. Only B's missile is
        // hostile, so A's shot, fired at it, passes through C's on the way.
        let [defender, attacker] = missile_attack(quad());
        let ally = Ship::at(3, 0.0, -100.0)
            .armed(WeaponRecord {
                speed: 500,
                ..missile()
            })
            .facing(180.0)
            .targeting(A);
        let mut ships = [defender, attacker, ally];
        let rule = FiredBy(B);
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..8 {
            tick_with(&mut combat, &mut ships, &Arsenal::default(), rules);
            ships[1].trigger = Trigger::default();
            ships[2].trigger = Trigger::default();
            ships[2].state.position = Vec2::new(500.0, 0.0);
            events.extend(combat.take_events());
        }
        let durability = |firer| {
            combat
                .shots()
                .iter()
                .find(|shot| shot.firer == firer && shot.weapon.id == WeaponId(134))
                .map(|shot| shot.durability)
        };
        assert_eq!(durability(C), Some(4.0), "flown through: {events:?}");
        assert_eq!(durability(B), Some(1.0), "met: {events:?}");
    }

    #[test]
    fn a_point_defence_shot_flies_through_a_lost_missile() {
        // B and C, each its own fleet and both hostile, fire missiles down
        // the same line at A, C's nearer, and C moves off the line. C's
        // missile has lost its target, so A's shot, fired at B's, passes
        // through it on the way.
        let [defender, attacker] = missile_attack(quad());
        let other = Ship::at(3, 0.0, -100.0)
            .armed(WeaponRecord {
                speed: 500,
                ..missile()
            })
            .facing(180.0)
            .targeting(A);
        let mut ships = [defender, attacker, other];
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..8 {
            tick_with(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                Rules::default(),
            );
            ships[1].trigger = Trigger::default();
            ships[2].trigger = Trigger::default();
            ships[2].state.position = Vec2::new(500.0, 0.0);
            for shot in &mut combat.shots {
                if shot.firer == C {
                    shot.target = None;
                    shot.lost = true;
                }
            }
            events.extend(combat.take_events());
        }
        let durability = |firer| {
            combat
                .shots()
                .iter()
                .find(|shot| shot.firer == firer && shot.weapon.id == WeaponId(134))
                .map(|shot| shot.durability)
        };
        assert_eq!(durability(C), Some(4.0), "flown through: {events:?}");
        assert_eq!(durability(B), Some(1.0), "met: {events:?}");
    }

    #[test]
    fn a_point_defence_beam_shoots_down_a_missile_a_tick_at_a_time() {
        let beam = WeaponRecord {
            guidance: 10,
            reload: 30,
            count: 5,
            beam_length: 300,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(150)
        };
        let mut combat = Combat::default();
        let mut ships = missile_attack(beam);
        let mut seen = Vec::new();
        let mut events = Vec::new();
        for _ in 0..4 {
            tick_with(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                Rules::default(),
            );
            ships[1].trigger = Trigger::default();
            seen.push((missile_durability(&combat), combat.beams().len()));
            events.extend(combat.take_events());
        }
        assert_eq!(
            seen,
            [(Some(1.0), 1), (Some(-2.0), 1), (None, 0), (None, 0)]
        );
        let downed: Vec<Vec2> = events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::ShotDown { at } => Some(at),
                _ => None,
            })
            .collect();
        assert_eq!(downed.len(), 1, "{events:?}");
        assert!(
            (downed[0] - Vec2::new(0.0, -185.0)).length() < 1e-3,
            "{downed:?}"
        );
        assert_eq!(ships[0].reserves, Reserves::full(10.0, 30.0, 100.0));
    }

    /// A shell 10 pixels a tick for 3 ticks releasing 2 of weapon 148 with
    /// `flags2`, and that weapon.
    fn cluster(flags2: u16) -> (WeaponRecord, Arsenal) {
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            energy_dmg: 1,
            sub_count: 2,
            sub_type: Some(WeaponId(148)),
            sub_theta: -10,
            flags2,
            ..weapon(182)
        };
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            ..weapon(148)
        };
        (shell, Arsenal::new(&[shell, sub], Vec::new()))
    }

    fn sub_shots(combat: &Combat) -> Vec<&Shot> {
        combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.id == WeaponId(148))
            .collect()
    }

    #[test]
    fn a_shot_releases_its_sub_munitions_at_the_end_of_its_life() {
        let (shell, arsenal) = cluster(0);
        let turret_shell = WeaponRecord {
            guidance: 4,
            ..shell
        };
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(turret_shell).targeting(C),
            Ship::at(3, 500.0, 0.0),
        ];
        let mut events = Vec::new();
        for _ in 0..3 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            events.extend(combat.take_events());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2);
        let at = subs[0].position;
        assert!((at - Vec2::new(30.0, 0.0)).length() < 1e-3, "{at:?}");
        for sub in &subs {
            assert_eq!(sub.position, at);
            assert_eq!(sub.generation, 1);
            assert_eq!(sub.target, Some(C), "the shell's target");
        }
        assert_eq!((subs[0].heading, subs[1].heading), (85.0, 95.0));
        assert_ne!(subs[0].id, subs[1].id);
        assert_eq!(
            events.last(),
            Some(&CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(148),
                at
            })
        );
        let (shell, arsenal) = cluster(0x0020);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..3 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert_eq!(sub_shots(&combat).len(), 0, "none on expiry");
        let (shell, _) = cluster(0);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &NovaDisable);
        }
        assert_eq!(combat.shots(), [], "a sub-munition that cannot be read");
    }

    #[test]
    fn a_shot_releases_its_sub_munitions_where_it_hits_aimed_at_the_ship_hit() {
        let (shell, arsenal) = cluster(0x0020);
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell), Ship::at(2, 30.0, 0.0)];
        let mut events = Vec::new();
        for _ in 0..2 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            events.extend(combat.take_events());
        }
        let subs = sub_shots(&combat);
        assert_eq!(subs.len(), 2, "{events:?}");
        assert_eq!(subs[0].target, Some(B));
        let at = subs[0].position;
        assert!(
            (at - Vec2::new(14.0, 0.0)).length() < 1e-3,
            "where it met B: {at:?}"
        );
        assert!(events.contains(&CombatEvent::Fired {
            ship: A,
            weapon: WeaponId(148),
            at
        }));
    }

    /// A shell like [`cluster`]'s, fired every tick, releasing 2 of `sub`
    /// at the end of its life, and that sub-munition.
    fn cluster_of(sub: WeaponRecord) -> (WeaponRecord, Arsenal) {
        let (shell, _) = cluster(0);
        let shell = WeaponRecord { reload: 0, ..shell };
        let sub = WeaponRecord {
            id: WeaponId(148),
            ..sub
        };
        (shell, Arsenal::new(&[shell, sub], Vec::new()))
    }

    #[test]
    fn a_sub_munition_reports_its_unimplemented_flags_once_when_first_released() {
        let (shell, arsenal) = cluster_of(WeaponRecord {
            guidance: 1,
            count: 50,
            speed: 500,
            seeker: 0x0001,
            ..weapon(148)
        });
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        for _ in 0..2 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert_eq!(combat.take_diagnostics(), [], "not before it is released");
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert!(sub_shots(&combat).len() > 2, "released more than once");
        assert_eq!(
            combat.take_diagnostics(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(148),
                field: flags::FlagField::Seeker,
                bit: 0x0001,
            }]
        );
    }

    #[test]
    fn a_sub_munition_of_an_unimplemented_guidance_is_reported_and_not_released() {
        let (shell, arsenal) = cluster_of(WeaponRecord {
            guidance: 2,
            count: 50,
            speed: 500,
            ..weapon(148)
        });
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell)];
        let mut events = Vec::new();
        for _ in 0..2 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
        }
        assert_eq!(combat.take_diagnostics(), [], "not before it is released");
        for _ in 0..5 {
            tick_with(&mut combat, &mut ships, &arsenal, Rules::default());
            events.extend(combat.take_events());
        }
        assert_eq!(sub_shots(&combat).len(), 0, "none in flight");
        assert!(
            !events.iter().any(|event| matches!(
                event,
                CombatEvent::Fired {
                    weapon: WeaponId(148),
                    ..
                }
            )),
            "{events:?}"
        );
        assert_eq!(
            combat.take_diagnostics(),
            [SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(148),
                guidance: 2,
            }]
        );
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
    }

    // Strikes.

    /// Ticks until a strike is made, or 40 ticks pass, and gives the
    /// strikes of that tick.
    fn first_strikes(combat: &mut Combat, ships: &mut [Ship], arsenal: &Arsenal) -> Vec<Strike> {
        for _ in 0..40 {
            tick_with(combat, ships, arsenal, Rules::default());
            let strikes = combat.take_strikes();
            if !strikes.is_empty() {
                return strikes;
            }
        }
        Vec::new()
    }

    #[test]
    fn a_shot_that_hits_strikes_by_its_firer_with_the_shield_and_armour_it_took() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 100.0, 0.0),
        ];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes,
            [Strike {
                ship: B,
                by: A,
                damage: 14.0,
                downed: None
            }],
            "10 off the shield, then 4 off the armour"
        );
        assert_eq!(combat.take_strikes(), [], "taken");
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            combat.take_strikes(),
            [Strike {
                ship: B,
                by: A,
                damage: 5.0,
                downed: None
            }],
            "the shield down to its floor, a tenth below none, then 4 off the armour"
        );
    }

    #[test]
    fn a_beam_strikes_every_tick_it_touches() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield = Gauge::full(100.0);
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(laser()), target];
        let mut struck = Vec::new();
        for _ in 0..5 {
            tick(&mut combat, &mut ships, &NovaDisable);
            struck.push(combat.take_strikes());
        }
        let hit = vec![Strike {
            ship: B,
            by: A,
            damage: 5.0,
            downed: None,
        }];
        assert_eq!(struck, [hit.clone(), hit.clone(), hit, vec![], vec![]]);
    }

    #[test]
    fn a_blast_strikes_each_ship_it_reaches() {
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
        ships[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                reload: 1000,
                ..shell
            }),
            1,
        )]);
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        let of = |ship| Strike {
            ship,
            by: A,
            damage: 14.0,
            downed: None,
        };
        assert_eq!(strikes, [of(B), of(C)], "the hit, then the blast");
    }

    #[test]
    fn a_sub_munition_strikes_by_its_parents_firer() {
        let shell = WeaponRecord {
            reload: 1000,
            count: 3,
            speed: 1000,
            sub_count: 1,
            sub_type: Some(WeaponId(148)),
            ..weapon(182)
        };
        let sub = WeaponRecord {
            count: 50,
            speed: 500,
            energy_dmg: 3,
            ..weapon(148)
        };
        let arsenal = Arsenal::new(&[shell, sub], Vec::new());
        let mut combat = Combat::default();
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(shell), Ship::at(2, 80.0, 0.0)];
        let strikes = first_strikes(&mut combat, &mut ships, &arsenal);
        assert_eq!(
            strikes,
            [Strike {
                ship: B,
                by: A,
                damage: 3.0,
                downed: None
            }]
        );
    }

    #[test]
    fn the_hit_that_disables_a_ship_says_so_and_the_one_that_breaks_it_up_too() {
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            Ship::at(2, 100.0, 0.0),
        ];
        let mut struck = Vec::new();
        for _ in 0..30 {
            tick(&mut combat, &mut ships, &NovaDisable);
            struck.extend(combat.take_strikes());
        }
        let downed: Vec<Option<Downed>> = struck.iter().map(|strike| strike.downed).collect();
        // Six hits take the shield and 24 armour (6 left, below a third):
        // disabled; two more leave it at -2.
        assert_eq!(
            downed,
            [
                None,
                None,
                None,
                None,
                None,
                Some(Downed::Disabled),
                None,
                Some(Downed::BreakingUp)
            ],
            "{struck:?}"
        );
        assert!(
            struck
                .iter()
                .all(|strike| strike.ship == B && strike.by == A)
        );
    }

    #[test]
    fn a_ship_broken_up_at_once_is_only_breaking_up() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield.now = 0.0;
        target.reserves.armor.now = 3.0;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), target];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes
                .iter()
                .map(|strike| strike.downed)
                .collect::<Vec<_>>(),
            [Some(Downed::BreakingUp)]
        );
    }

    #[test]
    fn only_the_last_strike_on_a_ship_downed_that_tick_says_so() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 100.0, 0.0);
        target.reserves.shield.now = 0.0;
        target.reserves.armor.now = 12.0;
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(blaster()),
            target,
            Ship::at(3, 200.0, 0.0).armed(blaster()).facing(270.0),
        ];
        let strikes = first_strikes(&mut combat, &mut ships, &Arsenal::default());
        assert_eq!(
            strikes,
            [
                Strike {
                    ship: B,
                    by: A,
                    damage: 5.0,
                    downed: None
                },
                Strike {
                    ship: B,
                    by: C,
                    damage: 4.0,
                    downed: Some(Downed::Disabled)
                }
            ],
            "12 armour less 8 is below a third of 30"
        );
    }

    #[test]
    fn a_ship_downed_without_a_strike_makes_none() {
        let mut combat = Combat::default();
        let mut target = Ship::at(2, 0.0, 0.0);
        target.reserves.armor.now = 0.0;
        tick(&mut combat, &mut [target], &NovaDisable);
        assert_eq!(combat.take_strikes(), []);
    }
}
