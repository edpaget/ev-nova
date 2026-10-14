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
//! - [`shockwave`]: what the shockwave of a heavy ship's final explosion
//!   does to each ship it reaches, and how it pushes it.
//! - [`stellar`]: which shots hit a destroyable stellar, and what they
//!   take off it.
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
//!    that cannot fire spends nothing. A fighter bay's launch is no shot
//!    but a [`Sortie`], which the session takes
//!    ([`Combat::take_sorties`]) and puts in the system as a ship.
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
//!
//!    7b. Each planet-type shot still in flight, past its `ProxSafety`,
//!    that met a stellar still standing among those the session hands
//!    the tick ([`StellarTarget`]) is spent on it: it explodes where it
//!    hit, with no blast and no sub-munitions, and takes its mass and
//!    energy damage off the stellar's strength. A stellar below 0 is
//!    skipped by the later shots. This comes after the ship hits and the
//!    expiries, as the original's stellar check (`_CheckShotProximities`
//!    @0x44015) comes after its ship collision passes; beams never hit a
//!    stellar. The session takes each hit as a [`StellarStrike`]
//!    ([`Combat::take_stellar_strikes`]) and shoots down those it
//!    destroyed.
//! 8. Conditions follow the damage: a ship with no armour left (of the
//!    armour it holds) starts breaking up, and any other that holds armour
//!    is disabled, or not, as the [`DisableRule`](hull::DisableRule) says;
//!    a ship that holds none is never disabled. A ship held disabled
//!    ([`Fighter::held_disabled`], a rescue mission's ship not yet
//!    boarded, as `_IsDisabled` reads it @0x2d24-0x2d69) is kept disabled
//!    at any armour, and is never newly disabled, so it reports no
//!    [`CombatEvent::Disabled`]; once it is no longer held, its armour
//!    decides again, with no event as it comes back intact.
//! 9. A ship breaking up counts down the original's death timer, its
//!    `DeathDelay` (three times it for the player), set on the breakup's
//!    first tick, and is destroyed once it is down to 2; a timer that
//!    never rises above 0 goes as the fight's [`Breakup`](hull::Breakup)
//!    says (see `update_condition`, `_HandleShipDisplay`
//!    @0x2d80d-0x2d85a).
//!
//!    9b. Each ship destroyed this tick that is heavy enough sends out a
//!    shockwave ([`shockwave`]): every ship within its explosion's size,
//!    breaking up or not, takes its damage, never destroyed or newly
//!    disabled by it, and is pushed away from the wreck. The session takes
//!    each push as a [`Shove`] ([`Combat::take_shoves`]) and sets the
//!    ship's velocity. Each intact or disabled ship hit then settles its
//!    condition as in step 8.
//! 10. Shields regenerate on an intact or disabled ship, armour only on an
//!     intact one, each up to what it holds.
//!
//! Each hit on a ship (a shot's, a beam's, a blast's, a shockwave's) is
//! kept as a [`Strike`] by the ship whose it was, with the shield and
//! armour it took; the last on a ship disabled or breaking up that tick
//! says so ([`Downed`]), and a shockwave's provokes nothing
//! ([`Strike::provokes`]). The session takes them ([`Combat::take_strikes`]) for
//! the player's crimes and the NPCs' answers.
//!
//! Each step is reported as a [`CombatEvent`] for the view to show, and
//! each unimplemented weapon feature once as a
//! [`SimDiagnostic`](report::SimDiagnostic). Nothing here is saved: the
//! shots, their targets, durability and generations live only in flight,
//! and a stellar's strength is the session's, which saves none either.

pub mod aim;
pub mod armament;
pub mod beam;
pub mod damage;
pub mod defence;
pub mod flags;
pub mod hull;
pub mod projectile;
pub mod report;
pub mod shockwave;
pub mod stellar;
pub mod submunition;
pub mod weapon;

use std::collections::BTreeMap;

use armament::{Armament, Arsenal, Launch, Rounds, Shooter, Trigger};
use beam::{Aiming, Beam};
use damage::{Blast, Hit};
use defence::{INTERCEPT_RADIUS, PointDefenceRule, Side};
use hull::{
    Breakup, Condition, DisableRule, EXPLOSION_BAND, HullSpec, PLAYER_BREAKUP_FACTOR, immune,
    never_breaks_up,
};
use projectile::{Shot, ShotId, Target, contact};
use report::{Reports, SimDiagnostic};
use stellar::{StellarStrike, StellarTarget};
use weapon::{Explosion, Guidance};

use crate::bay::FIGHTER_BAY;
use crate::catalog::{GovtId, ShipId, WeaponId};
use crate::chance::Chance;
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::govt::Governments;
use crate::legal::{LegalCode, NovaLaw};
use crate::reserves::{Gauge, Reserves};
use crate::rulebook::RuleSource;
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
    /// The ship whose shot, beam, blast or shockwave it was, or that
    /// boarded it.
    pub by: ShipRef,
    /// The shield and armour it took.
    pub damage: f32,
    /// How it left the ship, for the last hit on a ship whose condition
    /// changed that tick; none otherwise.
    pub downed: Option<Downed>,
    /// Whether it was an attack of `by`'s, which the ship hit may answer:
    /// the original's `_DamageShip` arg +0x20. A shot, beam, blast or
    /// boarding is; a death shockwave is not ([`shockwave`]), so it raises
    /// no grudge, uncloaks no ship and provokes no AI (the block @0x3b44d
    /// skips), though the player is still convicted of what it does.
    pub provokes: bool,
}

/// A fighter launched from a bay this tick, for the session to put in
/// the system (see [`bay`](crate::bay)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sortie {
    /// The ship that launched it.
    pub carrier: ShipRef,
    /// The bay it left from.
    pub bay: WeaponId,
    /// Its ship type.
    pub ship: ShipId,
    /// Where the carrier was and how it moved.
    pub from: ShipState,
    /// The heading it leaves along, the bay's inaccuracy included, in
    /// degrees.
    pub heading: f32,
    /// The carrier's target, if any.
    pub target: Option<ShipRef>,
}

/// A ship a shockwave pushed this tick, for the session to set its
/// velocity (see [`shockwave::shove`]): a [`Fighter`]'s state is a copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shove {
    /// The ship pushed.
    pub ship: ShipRef,
    /// Its velocity after the push.
    pub velocity: Vec2,
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
    /// Whether it is held disabled whatever its armour: a rescue
    /// mission's ship not yet boarded (see the session's `mission_ships`
    /// module). It is kept disabled until its armour is gone, never
    /// newly disabled, and the [`DisableRule`] is not asked of it.
    pub held_disabled: bool,
    /// Whether it is cloaked: it fires only its weapons that fire while
    /// cloaked ([`Armament::has_ammo`]). The player never is.
    pub cloaked: bool,
    /// Its class's top speed, in pixels a tick: its `shïp` `Speed` alone,
    /// none of its outfits' (the original's class +0x40), which a
    /// shockwave pushes it against ([`shockwave::shove`]).
    pub class_speed: f32,
    /// Its top speed, in pixels a tick, outfits and lead included (the
    /// original's `_ShipMaxSpeed`): a shockwave's push is clamped to it
    /// on each axis.
    pub top_speed: f32,
    /// Whether its jump is under way, the pre-jump stage included (the
    /// original's +0x50 above 0): no shockwave pushes it. An NPC's jump
    /// out has no stage in the simulation, so only the player's is.
    pub jumping: bool,
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
                crime_gains: RuleSource::Engine,
            },
        }
    }
}

/// The share of a shot's mass and energy damage that counts as aimed at
/// its ship (0.5 @0xdd128, `_CalculateTargetedDamage`).
pub const AIMED_DAMAGE_SHARE: f64 = 0.5;

/// The shots and beams in flight, and what the fight has reported.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    shots: Vec<Shot>,
    beams: Vec<Beam>,
    events: Vec<CombatEvent>,
    strikes: Vec<Strike>,
    stellar_strikes: Vec<StellarStrike>,
    sorties: Vec<Sortie>,
    shoves: Vec<Shove>,
    /// The ships destroyed this tick, in order, whose shockwaves are yet
    /// to go off (step 9b).
    wrecks: Vec<ShipRef>,
    reports: Reports,
    /// The next shot's number.
    next_shot: u32,
    /// What becomes of a breakup that never reaches the explosion band.
    breakup: Breakup,
}

impl Combat {
    /// Advances the fight among `fighters` and `stellars` a tick (see the
    /// module docs), by `rules`, with the relations between governments in
    /// `govts`, each sub-munition's weapon read from `arsenal` and each
    /// shot's inaccuracy and spread drawn on `chance`. Each stellar's
    /// strength is left as the shots left it.
    pub fn tick(
        &mut self,
        fighters: &mut [Fighter],
        arsenal: &Arsenal,
        govts: &Governments,
        rules: Rules,
        stellars: &mut [StellarTarget],
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
        // Where each shot started the tick, by its number: `resolve`
        // rebuilds the shots.
        let from: Vec<(ShotId, Vec2)> = self
            .shots
            .iter()
            .map(|shot| shot.id)
            .zip(was.iter().copied())
            .collect();
        let mut gone = vec![false; self.shots.len()];
        self.intercept(&was, &mut gone, rules.defence, govts);
        self.hold_on_missiles(&mut gone);
        let first_strike = self.strikes.len();
        self.resolve(fighters, &targets, &was, &gone, arsenal, chance);
        self.strike_stellars(stellars, &from);
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
        self.shock(fighters, rules.disable);
        for fighter in fighters.iter_mut() {
            regenerate(fighter);
        }
    }

    /// Step 9b: the shockwave of each ship destroyed this tick, in the
    /// order they were destroyed ([`shockwave`]). It reaches every ship
    /// not destroyed, breaking up or not, within the wreck's
    /// [`HullSpec::death_size`] on each axis, the wreck spared; each takes
    /// [`shockwave::hit`] as a [`Strike`] by the wreck that provokes
    /// nothing, and is pushed as [`shockwave::shove`] says, reported as a
    /// [`Shove`]. Then each intact or disabled ship hit settles its
    /// condition as step 8 does, its strike marked when it is newly
    /// disabled; a breaking-up one is not counted down again.
    ///
    /// The original runs the shockwave inside `_HandleShipDisplay` as each
    /// ship's final explosion goes off (0x2d8ef-0x2da9c), so a ship later
    /// in the table has already had its own condition and movement that
    /// frame; here it comes after every ship's condition update, and its
    /// push takes effect on the next movement. The difference stays within
    /// the tick and crosses no boundary the player sees.
    fn shock(&mut self, fighters: &mut [Fighter], rule: &dyn DisableRule) {
        for wreck in std::mem::take(&mut self.wrecks) {
            let Some((at, hull)) = fighters
                .iter()
                .find(|fighter| fighter.ship == wreck)
                .map(|fighter| (fighter.state.position, fighter.hull))
            else {
                continue;
            };
            let blast = Blast {
                at,
                radius: hull.death_size(),
                direct: None,
                firer: wreck,
                spares_player: true,
            };
            let reached = blast.reaches(
                fighters
                    .iter()
                    .filter(|fighter| *fighter.condition != Condition::Destroyed)
                    .map(|fighter| (fighter.ship, fighter.state.position)),
            );
            let damage = shockwave::damage(&hull);
            for ship in reached {
                let Some(fighter) = fighters.iter_mut().find(|fighter| fighter.ship == ship) else {
                    continue;
                };
                let condition = *fighter.condition;
                let before = taken(fighter.reserves);
                let hurt = !immune(fighter.ship_type);
                if hurt {
                    shockwave::hit(fighter.reserves, condition, &fighter.hull, rule, damage);
                }
                let pushed = shockwave::shove(
                    &fighter.state,
                    at,
                    &fighter.hull,
                    fighter.jumping,
                    fighter.class_speed,
                    fighter.top_speed,
                );
                if let Some(velocity) = pushed {
                    fighter.state.velocity = velocity;
                    self.shoves.push(Shove { ship, velocity });
                }
                if !hurt {
                    continue;
                }
                let downed = if condition.hittable() {
                    self.update_condition(fighter, rule)
                } else {
                    None
                };
                self.strikes.push(Strike {
                    ship,
                    by: wreck,
                    damage: before - taken(fighter.reserves),
                    downed,
                    provokes: false,
                });
            }
        }
    }

    /// Has a breakup that never reaches the explosion band go as
    /// `breakup` says: by default, [`Breakup::Explodes`].
    pub fn set_breakup(&mut self, breakup: Breakup) {
        self.breakup = breakup;
    }

    /// The next shot's number.
    fn next_id(&mut self) -> ShotId {
        let id = ShotId(self.next_shot);
        self.next_shot += 1;
        id
    }

    /// The damage aimed at each ship by the shots in flight
    /// (`_CalculateTargetedDamage` @0x3a153-0x3a232): for each shot not
    /// lost and aimed at a ship ([`Shot::aimed_at`]), its mass and energy
    /// damage times [`AIMED_DAMAGE_SHARE`], added to that ship's total
    /// and truncated after each add. Beams are not shots and count for
    /// nothing. The original keeps each total in 16 bits; here it is not
    /// wrapped.
    ///
    /// The original builds the totals once a frame, before the AI runs
    /// (`_DoPlayGameWork` @0x44223, before `_AIDispatch` @0x4429d); the
    /// session builds them from the shots in flight as its traffic
    /// decides, which is the same point in the frame.
    #[must_use]
    pub fn targeted_damage(&self) -> BTreeMap<ShipRef, i32> {
        let mut aimed = BTreeMap::new();
        for shot in self.shots.iter().filter(|shot| !shot.lost) {
            let Some(ship) = shot.aimed_at else {
                continue;
            };
            let damage = f64::from(shot.weapon.mass_damage) + f64::from(shot.weapon.energy_damage);
            let total: &mut i32 = aimed.entry(ship).or_default();
            // A sum of `i16` halves stays far within an `i32`.
            #[allow(clippy::cast_possible_truncation)]
            {
                *total = damage
                    .mul_add(AIMED_DAMAGE_SHARE, f64::from(*total))
                    .trunc() as i32;
            }
        }
        aimed
    }

    /// Puts `launch` in flight from `side`, at `from`, its damage aimed at
    /// `aimed_at`: a fighter's [`Sortie`] from a bay, a beam held on the
    /// missile `quarry`, on its target, or ahead, or a shot of its
    /// government.
    fn launch(
        &mut self,
        side: Side,
        from: &ShipState,
        launch: Launch,
        quarry: Option<ShotId>,
        aimed_at: Option<ShipRef>,
    ) {
        let Side { ship, fleet, govt } = side;
        self.events.push(CombatEvent::Fired {
            ship,
            weapon: launch.weapon.id,
            at: from.position,
        });
        if let Some(carried) = launch.weapon.carried {
            self.sorties.push(Sortie {
                carrier: ship,
                bay: launch.weapon.id,
                ship: carried,
                from: *from,
                heading: launch.heading,
                target: launch.target,
            });
        } else if launch.weapon.is_beam() {
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
                aimed_at,
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
                fighter.shooter(),
                fighter.rounds,
                &mut fighter.reserves.fuel,
                chance,
                &mut self.reports,
                &mut |spec| aim::aim(spec, &state, &hull, target),
            );
            for launch in launches {
                // A front-quadrant turret firing ahead aims at nothing
                // (@0x88bf0); every other shot at the ship's target.
                let ahead =
                    launch.weapon.guidance == Guidance::FrontTurret && launch.target.is_none();
                let aimed_at = if ahead { None } else { fighter.target };
                self.launch(fighter.side(), &state, launch, None, aimed_at);
            }
        }
    }

    /// Step 2: each ship's point defence its trigger asks for
    /// ([`Trigger::point_defence`](armament::Trigger::point_defence))
    /// fires at the missile it picks, as `rule` says which are hostile
    /// with the relations in `govts`.
    fn defend(
        &mut self,
        fighters: &mut [Fighter],
        rule: &dyn PointDefenceRule,
        govts: &Governments,
        chance: &mut dyn Chance,
    ) {
        for fighter in fighters
            .iter_mut()
            .filter(|fighter| fighter.trigger.point_defence)
        {
            let (state, hull) = (fighter.state, fighter.hull);
            let side = fighter.side();
            let shots = &self.shots;
            let fired = fighter.armament.fire_point_defence(
                *fighter.condition,
                fighter.shooter(),
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
                self.launch(side, &state, launch, Some(missile), None);
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
        // A shot that hits the escape pod is spent on it all the same:
        // `_HandleShipHit` turns away only a class above 0x2ff
        // (@0x36a58), and `_DamageShip` then does nothing to it ([`immune`]).
        for (ship, by, hit) in hits {
            let fighter = fighters
                .iter_mut()
                .find(|fighter| fighter.ship == ship && !immune(fighter.ship_type));
            if let Some(fighter) = fighter {
                let before = taken(fighter.reserves);
                damage::apply(fighter.reserves, hit);
                self.strikes.push(Strike {
                    ship,
                    by,
                    damage: before - taken(fighter.reserves),
                    downed: None,
                    provokes: true,
                });
            }
        }
    }

    /// Step 7b: each planet-type shot left in flight that met a stellar
    /// still standing among `stellars` on its way from where `from` says
    /// it started the tick is spent on it ([`stellar`]): it explodes where
    /// it hit, with no blast and no sub-munitions, and takes its damage
    /// off the stellar's strength. A sub-munition released this tick
    /// started it nowhere, so it is first checked the next.
    fn strike_stellars(&mut self, stellars: &mut [StellarTarget], from: &[(ShotId, Vec2)]) {
        let mut kept = Vec::with_capacity(self.shots.len());
        for shot in std::mem::take(&mut self.shots) {
            let met = from
                .iter()
                .find(|(id, _)| *id == shot.id)
                .and_then(|&(_, was)| stellar::met(&shot, was, stellars));
            let Some((index, at)) = met else {
                kept.push(shot);
                continue;
            };
            if let Some(explosion) = shot.weapon.explosion {
                self.events.push(CombatEvent::Exploded { at, explosion });
            }
            let target = &mut stellars[index];
            let damage = stellar::damage(&shot.weapon);
            target.strength -= damage;
            self.stellar_strikes.push(StellarStrike {
                stellar: target.stellar,
                by: shot.firer,
                damage,
                destroyed: stellar::shot_down(target.strength),
            });
        }
        self.shots = kept;
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
        let unflown = match sub.guidance {
            Guidance::Other(guidance) => Some(guidance),
            Guidance::FighterBay => Some(FIGHTER_BAY),
            _ => None,
        };
        if let Some(guidance) = unflown {
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
    ///
    /// The countdown is the original's death timer (field 0x3c), as
    /// `_HandleShipDisplay` reads it (@0x2d80d-0x2d85a):
    ///
    /// - On the breakup's first tick the timer is set to the ship's
    ///   `DeathDelay`, times [`PLAYER_BREAKUP_FACTOR`] for the player
    ///   (@0x2d834-0x2d850), and nothing else happens (both paths jump to
    ///   0x2def0): the ship is `Dying` with `starting` set, and no tick is
    ///   taken off.
    /// - Each later tick takes 1 off a timer above 0 (the player's
    ///   `_HandlePlayer` @0x6d7e6-0x6d7f8, an NPC's `_HandleShip`
    ///   @0x35083-0x350a3, each before the display check), and the ship
    ///   is destroyed once it is at [`EXPLOSION_BAND`] or below
    ///   (@0x2d85a). So the player explodes 3d - 2 ticks after its
    ///   breakup tick, and an NPC d - 2 ticks after, or 1 tick after for
    ///   d = 2.
    /// - A timer that never rises above 0 by the check (d <= 0, or d = 1
    ///   for an NPC) goes as the fight's [`Breakup`] says: destroyed on
    ///   the breakup tick (d <= 0) or the tick after (d = 1) by default;
    ///   set again, `starting` again, every tick under
    ///   [`Breakup::Stalls`], the original's reset (@0x2d80d-0x2d83d).
    ///
    /// Class 0x2ff, the escape pod, skips the whole dying block
    /// (`_HandleShipDisplay` @0x2d803, [`never_breaks_up`]): with its
    /// armour gone its condition stays as it is, with no breakup, no
    /// countdown and no explosion.
    ///
    /// Left out: `_HandleShip` skips the NPC decrement while
    /// `_gameFrozen` is set (@0x35091-0x35099), and the engine has no
    /// frozen game.
    fn update_condition(
        &mut self,
        fighter: &mut Fighter,
        rule: &dyn DisableRule,
    ) -> Option<Downed> {
        if never_breaks_up(fighter.ship_type) && armour_gone(fighter.reserves.armor) {
            return None;
        }
        let ship = fighter.ship;
        let at = fighter.state.position;
        let timer = if ship == ShipRef::Player {
            fighter.hull.death_delay * PLAYER_BREAKUP_FACTOR
        } else {
            fighter.hull.death_delay
        };
        let starting = Condition::Dying {
            ticks_left: timer,
            starting: true,
        };
        match *fighter.condition {
            Condition::Intact | Condition::Disabled if armour_gone(fighter.reserves.armor) => {
                self.events.push(CombatEvent::BreakingUp {
                    ship,
                    at,
                    explosion: fighter.hull.breakup,
                });
                if timer == 0 && self.breakup == Breakup::Explodes {
                    self.destroy(fighter);
                } else {
                    *fighter.condition = starting;
                }
                Some(Downed::BreakingUp)
            }
            Condition::Intact | Condition::Disabled if fighter.held_disabled => {
                *fighter.condition = Condition::Disabled;
                None
            }
            Condition::Intact | Condition::Disabled => {
                let armor = fighter.reserves.armor;
                let disabled = holds_armour(armor) && rule.disabled(armor, &fighter.hull);
                let downed = (disabled && *fighter.condition == Condition::Intact).then(|| {
                    self.events.push(CombatEvent::Disabled { ship });
                    Downed::Disabled
                });
                *fighter.condition = if disabled {
                    Condition::Disabled
                } else {
                    Condition::Intact
                };
                downed
            }
            Condition::Dying { ticks_left, .. } => {
                let left = ticks_left.saturating_sub(1);
                if left == 0 && self.breakup == Breakup::Stalls {
                    *fighter.condition = starting;
                } else if left <= EXPLOSION_BAND {
                    self.destroy(fighter);
                } else {
                    *fighter.condition = Condition::Dying {
                        ticks_left: left,
                        starting: false,
                    };
                }
                None
            }
            Condition::Destroyed => None,
        }
    }

    /// Destroys `fighter`, raising its [`CombatEvent::Destroyed`].
    fn destroy(&mut self, fighter: &mut Fighter) {
        *fighter.condition = Condition::Destroyed;
        self.wrecks.push(fighter.ship);
        self.events.push(CombatEvent::Destroyed {
            ship: fighter.ship,
            ship_type: fighter.ship_type,
            at: fighter.state.position,
            velocity: fighter.state.velocity,
            explosion: fighter.hull.explosion,
            size: fighter.hull.death_size(),
        });
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

    /// The hits on stellars made since they were last taken, in order;
    /// taking them empties the list.
    pub fn take_stellar_strikes(&mut self) -> Vec<StellarStrike> {
        std::mem::take(&mut self.stellar_strikes)
    }

    /// Reports an explosion at `at` that is no weapon's: a stellar's as it
    /// is destroyed. It is of size 0, as `_CreateExplosion(.., 0, 1)`
    /// makes it, so no extra explosions scatter from it.
    pub(crate) fn explode(&mut self, at: Vec2, explosion: Explosion) {
        self.events.push(CombatEvent::Exploded { at, explosion });
    }

    /// The fighters launched since they were last taken, in order; taking
    /// them empties the list.
    pub fn take_sorties(&mut self) -> Vec<Sortie> {
        std::mem::take(&mut self.sorties)
    }

    /// The ships shockwaves pushed since they were last taken, in order,
    /// each with its velocity after the push; taking them empties the
    /// list.
    pub fn take_shoves(&mut self) -> Vec<Shove> {
        std::mem::take(&mut self.shoves)
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
    /// The ship as `_WeaponHasAmmo` asks of it: an NPC or the player,
    /// cloaked or not, and its class's `KeyCarried`.
    fn shooter(&self) -> Shooter {
        Shooter {
            npc: matches!(self.ship, ShipRef::Npc(_)),
            cloaked: self.cloaked,
            key_carried: self.hull.key_carried,
        }
    }

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
    use crate::combat::hull::{ESCAPE_POD_SHIP, NovaDisable};
    use crate::combat::weapon::WeaponSpec;
    use crate::testkit::weapon;

    /// A ship in a test fight, owning what a fighter borrows.
    struct Ship {
        id: ShipRef,
        class: ShipId,
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
        held_disabled: bool,
        cloaked: bool,
        class_speed: f32,
        top_speed: f32,
        jumping: bool,
        armament: Armament,
        rounds: BTreeMap<WeaponId, u32>,
    }

    /// A trigger holding no weapon, its point defence running, as a test
    /// ship's always did before an NPC's ran only when its AI asks.
    const QUIET: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: false,
        point_defence: true,
    };

    impl Ship {
        /// NPC `id` at (`x`, `y`), at rest, facing right, unarmed, with
        /// 10 shield and 30 armour, a default hull and a class and top
        /// speed of 10.
        fn at(id: u32, x: f32, y: f32) -> Self {
            Self {
                id: ShipRef::Npc(NpcId(id)),
                class: ShipId(128),
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
                trigger: QUIET,
                reserves: Reserves::full(10.0, 30.0, 100.0),
                condition: Condition::Intact,
                held_disabled: false,
                cloaked: false,
                class_speed: 10.0,
                top_speed: 10.0,
                jumping: false,
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
                    turrets_only: false,
                    bays: false,
                    point_defence: true,
                },
                ..self
            }
        }

        fn fighter(&mut self) -> Fighter<'_> {
            Fighter {
                ship: self.id,
                ship_type: self.class,
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
                held_disabled: self.held_disabled,
                cloaked: self.cloaked,
                class_speed: self.class_speed,
                top_speed: self.top_speed,
                jumping: self.jumping,
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
        combat.tick(
            &mut fighters,
            arsenal,
            govts,
            rules,
            &mut [],
            &mut NeverFires,
        );
    }

    /// A tick among `ships` and `stellars`, with `arsenal`.
    fn tick_by(
        combat: &mut Combat,
        ships: &mut [Ship],
        arsenal: &Arsenal,
        stellars: &mut [StellarTarget],
    ) {
        let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
        combat.tick(
            &mut fighters,
            arsenal,
            &Governments::default(),
            Rules::default(),
            stellars,
            &mut NeverFires,
        );
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

    /// The shots in flight after `ship`, firing `record`, fires a tick
    /// among `others`.
    fn launched_by(ship: Ship, record: WeaponRecord, others: Vec<Ship>) -> Combat {
        let mut combat = Combat::default();
        let mut ships = vec![ship.armed(record)];
        ships.extend(others);
        tick(&mut combat, &mut ships, &NovaDisable);
        combat
    }

    #[test]
    fn every_shot_a_ship_fires_is_aimed_at_its_target() {
        let homing = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            ..blaster()
        };
        for record in [blaster(), homing] {
            let combat = launched_by(
                Ship::at(1, 0.0, 0.0).targeting(B),
                record,
                vec![Ship::at(2, 0.0, -300.0)],
            );
            assert_eq!(combat.shots().len(), 1);
            assert_eq!(combat.shots()[0].aimed_at, Some(B), "{:?}", record.guidance);
        }
        let combat = launched_by(Ship::at(1, 0.0, 0.0), blaster(), Vec::new());
        assert_eq!(combat.shots()[0].aimed_at, None, "no target");
        let gone = launched_by(Ship::at(1, 0.0, 0.0).targeting(C), blaster(), Vec::new());
        assert_eq!(
            gone.shots()[0].aimed_at,
            Some(C),
            "its target, there or not"
        );
    }

    #[test]
    fn a_front_turret_firing_ahead_is_aimed_at_nothing() {
        let front = WeaponRecord {
            guidance: 7,
            ..blaster()
        };
        // Facing right: B ahead, C behind.
        let ahead = launched_by(
            Ship::at(1, 0.0, 0.0).targeting(B),
            front,
            vec![Ship::at(2, 300.0, 0.0)],
        );
        assert_eq!(ahead.shots()[0].aimed_at, Some(B));
        let astern = launched_by(
            Ship::at(1, 0.0, 0.0).targeting(C),
            front,
            vec![Ship::at(3, -300.0, 0.0)],
        );
        assert_eq!(astern.shots()[0].aimed_at, None, "fired straight ahead");
    }

    /// A shot of `record` by A, aimed at B.
    fn aimed(record: WeaponRecord) -> Shot {
        Shot {
            aimed_at: Some(B),
            ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 0.0)
        }
    }

    #[test]
    fn the_damage_aimed_at_a_ship_is_half_its_shots_mass_and_energy_truncated_after_each() {
        let five = WeaponRecord {
            mass_dmg: 2,
            energy_dmg: 3,
            ..blaster()
        };
        let mut combat = Combat {
            shots: vec![aimed(five), aimed(five), aimed(five)],
            ..Combat::default()
        };
        assert_eq!(
            combat.targeted_damage(),
            BTreeMap::from([(B, 6)]),
            "2.5 three times, truncated each time: not 7"
        );
        let missile = WeaponRecord {
            guidance: 1,
            mass_dmg: 30,
            energy_dmg: 11,
            ..blaster()
        };
        combat.shots = vec![
            aimed(missile),
            aimed(blaster()),
            Shot {
                aimed_at: Some(C),
                ..aimed(five)
            },
            Shot {
                aimed_at: None,
                ..aimed(missile)
            },
            Shot {
                lost: true,
                ..aimed(missile)
            },
        ];
        // 20.5 truncated to 20, then 20 + 7 = 27.
        assert_eq!(combat.targeted_damage(), BTreeMap::from([(B, 27), (C, 2)]));
        let negative = WeaponRecord {
            mass_dmg: -3,
            energy_dmg: 0,
            ..blaster()
        };
        combat.shots = vec![aimed(negative), aimed(five)];
        assert_eq!(
            combat.targeted_damage(),
            BTreeMap::from([(B, 1)]),
            "-1.5 truncates to -1, then 1.5 to 1"
        );
        assert_eq!(Combat::default().targeted_damage(), BTreeMap::new());
        assert_eq!(AIMED_DAMAGE_SHARE, 0.5);
    }

    #[test]
    fn point_defence_is_aimed_at_no_ship() {
        let quad = WeaponRecord {
            guidance: 9,
            ..blaster()
        };
        let mut combat = Combat::default();
        let missile = WeaponRecord {
            guidance: 1,
            guided_turn: 70,
            count: 100,
            speed: 100,
            ..weapon(134)
        };
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(quad).targeting(B),
            Ship::at(2, 150.0, 0.0).armed(missile).targeting(A),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        let defence: Vec<&Shot> = combat
            .shots()
            .iter()
            .filter(|shot| shot.weapon.guidance == Guidance::PointDefence)
            .collect();
        assert_eq!(defence.len(), 1, "{:?}", combat.shots());
        assert_eq!(defence[0].aimed_at, None);
    }

    /// How many shots `ship`, firing `record`, fires in a tick.
    fn shots_of(ship: Ship, record: WeaponRecord) -> usize {
        let mut combat = Combat::default();
        let id = ship.id;
        let mut ships = [ship.armed(record)];
        tick(&mut combat, &mut ships, &NovaDisable);
        fired(&combat.take_events(), id)
    }

    #[test]
    fn a_fighter_fires_as_weapon_has_ammo_says_of_its_ship() {
        use crate::combat::weapon::{AI_WONT_USE, FIRES_CLOAKED, NEEDS_KEY_SHIP};
        let wont = WeaponRecord {
            flags2: AI_WONT_USE,
            ..blaster()
        };
        assert_eq!(shots_of(Ship::at(1, 0.0, 0.0), wont), 0, "an NPC");
        let player = Ship {
            id: ShipRef::Player,
            fleet: ShipRef::Player,
            ..Ship::at(1, 0.0, 0.0)
        };
        assert_eq!(shots_of(player, wont), 1, "the player");
        let cloaked = || Ship {
            cloaked: true,
            ..Ship::at(1, 0.0, 0.0)
        };
        assert_eq!(shots_of(cloaked(), blaster()), 0, "cloaked");
        let fires_cloaked = WeaponRecord {
            flags2: FIRES_CLOAKED,
            ..blaster()
        };
        assert_eq!(shots_of(cloaked(), fires_cloaked), 1);
        // A key ship needed, its class's `KeyCarried` read from the hull.
        let keyed = WeaponRecord {
            flags2: NEEDS_KEY_SHIP,
            ..blaster()
        };
        let carrier = |key| {
            let mut ship = Ship::at(1, 0.0, 0.0).armed(keyed);
            ship.hull.key_carried = key;
            ship.armament = Armament::new([
                (WeaponSpec::new(&keyed), 1),
                (
                    WeaponSpec::new(&WeaponRecord {
                        guidance: 99,
                        ammo_type: 200,
                        ..weapon(150)
                    }),
                    1,
                ),
            ]);
            ship.rounds = BTreeMap::from([(WeaponId(150), 1)]);
            let mut combat = Combat::default();
            let mut ships = [ship];
            tick(&mut combat, &mut ships, &NovaDisable);
            fired(&combat.take_events(), A)
        };
        assert_eq!(carrier(Some(ShipId(200))), 1);
        assert_eq!(carrier(None), 0);
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
        assert_eq!(
            destroyed_on - breaking_on,
            1,
            "its DeathDelay less 2: an NPC's timer of 3 takes 1 off to 2"
        );
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

    /// A ship whose armour is gone, breaking up for `death_delay`: the
    /// player's, or NPC 2's.
    fn breaking(player: bool, death_delay: u32) -> Ship {
        let mut ship = Ship::at(2, 0.0, 0.0);
        if player {
            ship.id = ShipRef::Player;
            ship.fleet = ShipRef::Player;
        }
        ship.reserves.armor.now = 0.0;
        ship.hull.death_delay = death_delay;
        ship
    }

    /// A fight whose breakups below the explosion band go as `breakup`
    /// says.
    fn breaking_up_by(breakup: Breakup) -> Combat {
        let mut combat = Combat::default();
        combat.set_breakup(breakup);
        combat
    }

    /// How many ticks after the one it breaks up on `ship` is destroyed,
    /// under `breakup`: none within 400.
    fn breakup_ticks(ship: Ship, breakup: Breakup) -> Option<u32> {
        let mut combat = breaking_up_by(breakup);
        let mut ships = [ship];
        for tick_no in 0..400 {
            tick(&mut combat, &mut ships, &NovaDisable);
            let events = combat.take_events();
            if tick_no == 0 {
                assert!(
                    matches!(events.first(), Some(CombatEvent::BreakingUp { .. })),
                    "{events:?}"
                );
            }
            if events
                .iter()
                .any(|event| matches!(event, CombatEvent::Destroyed { .. }))
            {
                assert_eq!(ships[0].condition, Condition::Destroyed);
                return Some(tick_no);
            }
        }
        None
    }

    #[test]
    fn a_players_breakup_lasts_three_times_its_death_delay_less_two() {
        for breakup in [Breakup::Explodes, Breakup::Stalls] {
            for (death_delay, after) in [(1, 1), (2, 4), (60, 178)] {
                assert_eq!(
                    breakup_ticks(breaking(true, death_delay), breakup),
                    Some(after),
                    "{breakup:?}: {death_delay}"
                );
            }
        }
    }

    #[test]
    fn an_npcs_breakup_lasts_its_death_delay_less_two_and_one_tick_at_two() {
        for breakup in [Breakup::Explodes, Breakup::Stalls] {
            for (death_delay, after) in [(2, 1), (3, 1), (4, 2), (60, 58)] {
                assert_eq!(
                    breakup_ticks(breaking(false, death_delay), breakup),
                    Some(after),
                    "{breakup:?}: {death_delay}"
                );
            }
        }
    }

    #[test]
    fn the_breakup_tick_sets_the_timer_and_marks_it_starting() {
        for (player, timer) in [(true, 15), (false, 5)] {
            let mut combat = Combat::default();
            let mut ships = [breaking(player, 5)];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: timer,
                    starting: true
                },
                "player {player}"
            );
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: timer - 1,
                    starting: false
                },
                "player {player}"
            );
        }
    }

    #[test]
    fn an_npc_with_a_death_delay_of_one_explodes_the_tick_after_by_default() {
        let mut combat = Combat::default();
        let mut ships = [breaking(false, 1)];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(
            ships[0].condition,
            Condition::Dying {
                ticks_left: 1,
                starting: true
            }
        );
        assert_eq!(
            breakup_ticks(breaking(false, 1), Breakup::default()),
            Some(1)
        );
    }

    #[test]
    fn a_player_without_a_death_delay_is_destroyed_as_it_breaks_up_by_default() {
        assert_eq!(
            breakup_ticks(breaking(true, 0), Breakup::default()),
            Some(0)
        );
    }

    #[test]
    fn by_the_originals_reading_a_breakup_below_the_band_never_ends() {
        for (player, death_delay) in [(true, 0), (false, 0), (false, 1)] {
            let mut combat = breaking_up_by(Breakup::Stalls);
            let mut ships = [breaking(player, death_delay)];
            let mut events = Vec::new();
            for _ in 0..300 {
                tick(&mut combat, &mut ships, &NovaDisable);
                events.extend(combat.take_events());
            }
            assert_eq!(
                ships[0].condition,
                Condition::Dying {
                    ticks_left: death_delay,
                    starting: true
                },
                "player {player}, {death_delay}: set again every tick"
            );
            assert!(
                matches!(events[..], [CombatEvent::BreakingUp { .. }]),
                "player {player}, {death_delay}: {events:?}"
            );
        }
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

    /// A ship of `mass` tons at (`x`, `y`) whose armour is gone, with no
    /// death delay, so it breaks up and explodes on the next tick: `id`,
    /// an NPC or the player.
    fn wreck(id: ShipRef, mass: f32, x: f32, y: f32) -> Ship {
        let mut ship = Ship::at(9, x, y);
        ship.id = id;
        ship.fleet = id;
        ship.reserves.armor.now = 0.0;
        ship.hull.mass = mass;
        ship
    }

    /// NPC `id` at (`x`, `y`), at rest, with `shield` shields and `armor`
    /// of `armor_max` armour.
    fn holding(id: u32, x: f32, y: f32, shield: f32, armor: f32, armor_max: f32) -> Ship {
        let mut ship = Ship::at(id, x, y);
        ship.reserves = Reserves::full(shield, armor_max, 100.0);
        ship.reserves.armor.now = armor;
        ship
    }

    /// The shields and armour `ship` holds now.
    fn left(ship: &Ship) -> (f32, f32) {
        (ship.reserves.shield.now, ship.reserves.armor.now)
    }

    const WRECK: ShipRef = ShipRef::Npc(NpcId(9));

    #[test]
    fn a_heavy_ships_final_explosion_hits_a_ship_65_pixels_off_and_not_66() {
        let mut combat = Combat::default();
        let mut ships = [
            wreck(WRECK, 200.0, 0.0, 0.0),
            holding(1, 65.0, -65.0, 100.0, 300.0, 300.0),
            holding(2, 66.0, 0.0, 100.0, 300.0, 300.0),
            holding(3, -30.0, 66.0, 100.0, 300.0, 300.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Destroyed);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        assert_eq!(left(&ships[2]), (100.0, 300.0), "66 off on x");
        assert_eq!(left(&ships[3]), (100.0, 300.0), "66 off on y");
        let strikes = combat.take_strikes();
        assert_eq!(
            strikes,
            [Strike {
                ship: A,
                by: WRECK,
                damage: 32.0,
                downed: None,
                provokes: false,
            }]
        );
    }

    #[test]
    fn a_planet_type_or_light_ship_explodes_without_a_shockwave() {
        let mut planet = wreck(WRECK, 400.0, 0.0, 0.0);
        planet.hull.planet_type = true;
        for wrecked in [planet, wreck(WRECK, 99.0, 0.0, 0.0)] {
            let mut combat = Combat::default();
            let mut ships = [wrecked, holding(1, 10.0, 0.0, 100.0, 300.0, 300.0)];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(ships[0].condition, Condition::Destroyed);
            assert_eq!(left(&ships[1]), (100.0, 300.0));
            assert_eq!(combat.take_strikes(), []);
            assert_eq!(combat.take_shoves(), []);
            let events = combat.take_events();
            assert!(
                events.iter().any(
                    |event| matches!(event, CombatEvent::Destroyed { size, .. } if *size == 0.0)
                ),
                "{events:?}"
            );
        }
    }

    #[test]
    fn the_players_final_explosion_hits_npcs_and_not_itself() {
        let mut combat = Combat::default();
        let mut npc = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        npc.hull.mass = 200.0;
        let mut ships = [wreck(ShipRef::Player, 200.0, 0.0, 0.0), npc];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Destroyed);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].ship, strikes[0].by), (A, ShipRef::Player));
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, A);
    }

    #[test]
    fn an_npcs_final_explosion_hits_the_player() {
        let mut combat = Combat::default();
        let mut player = holding(1, 0.0, 40.0, 100.0, 300.0, 300.0);
        player.id = ShipRef::Player;
        player.fleet = ShipRef::Player;
        player.hull.mass = 200.0;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), player];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].ship, strikes[0].by), (ShipRef::Player, WRECK));
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, ShipRef::Player);
        assert!(
            (shoves[0].velocity - Vec2::new(0.0, 3.75)).length() < 1e-4,
            "away from the wreck: {shoves:?}"
        );
    }

    #[test]
    fn a_shockwave_strikes_by_the_wreck_and_says_when_it_disabled() {
        let mut combat = Combat::default();
        let mut ships = [
            wreck(WRECK, 200.0, 0.0, 0.0),
            holding(1, 20.0, 0.0, 0.0, 300.0, 300.0),
            holding(2, -20.0, 0.0, 0.0, 13_350.0, 40_000.0),
        ];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (0.0, 268.0));
        assert_eq!(ships[1].condition, Condition::Intact);
        assert_eq!(left(&ships[2]), (0.0, 13_333.0), "floored, still disabled");
        assert_eq!(ships[2].condition, Condition::Disabled);
        assert_eq!(
            combat.take_strikes(),
            [
                Strike {
                    ship: A,
                    by: WRECK,
                    damage: 32.0,
                    downed: None,
                    provokes: false,
                },
                Strike {
                    ship: B,
                    by: WRECK,
                    damage: 17.0,
                    downed: Some(Downed::Disabled),
                    provokes: false,
                },
            ]
        );
        let events = combat.take_events();
        let disabled: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, CombatEvent::Disabled { .. }))
            .collect();
        assert_eq!(disabled, [&CombatEvent::Disabled { ship: B }]);
    }

    #[test]
    fn a_shockwave_shoves_a_breaking_up_ship_too_and_counts_it_down_once() {
        let mut combat = Combat::default();
        let mut dying = holding(1, 40.0, 0.0, 0.0, -1.0, 300.0);
        dying.hull.mass = 200.0;
        dying.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), dying];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (0.0, -33.0));
        assert_eq!(
            ships[1].condition,
            Condition::Dying {
                ticks_left: 8,
                starting: false
            }
        );
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        assert_eq!(shoves[0].ship, A);
        assert!(
            (shoves[0].velocity - Vec2::new(3.75, 0.0)).length() < 1e-4,
            "{shoves:?}"
        );
        let strikes = combat.take_strikes();
        assert_eq!(strikes.len(), 1, "{strikes:?}");
        assert_eq!((strikes[0].damage, strikes[0].downed), (32.0, None));
    }

    #[test]
    fn a_ship_whose_jump_is_under_way_is_hit_but_not_shoved() {
        let mut combat = Combat::default();
        let mut jumping = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        jumping.hull.mass = 200.0;
        jumping.jumping = true;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), jumping];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(left(&ships[1]), (68.0, 300.0));
        assert_eq!(combat.take_shoves(), []);
    }

    #[test]
    fn a_shove_is_capped_at_the_ships_class_speed_and_clamped_to_its_top_speed() {
        let mut combat = Combat::default();
        let mut slow = holding(1, 40.0, 0.0, 100.0, 300.0, 300.0);
        slow.hull.mass = 100.0;
        slow.state.velocity = Vec2::new(-9.0, 7.0);
        slow.class_speed = 4.0;
        slow.top_speed = 6.0;
        let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), slow];
        tick(&mut combat, &mut ships, &NovaDisable);
        let shoves = combat.take_shoves();
        assert_eq!(shoves.len(), 1, "{shoves:?}");
        // -9 + 7.5 is -1.5, short of the class speed; 7 is clamped to 6.
        assert!(
            (shoves[0].velocity - Vec2::new(-1.5, 6.0)).length() < 1e-4,
            "{shoves:?}"
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
        dying.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
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
    fn a_held_ship_at_full_armour_is_stored_disabled_with_no_event_and_no_armour_back() {
        let mut combat = Combat::default();
        let mut held = Ship::at(1, 0.0, 0.0);
        held.held_disabled = true;
        held.reserves.armor.now = 20.0;
        held.armor_regen = 1.0;
        held.shield_regen = 1.0;
        held.reserves.shield.now = 0.0;
        let mut ships = [held];
        let well = Recording::default();
        for _ in 0..3 {
            tick(&mut combat, &mut ships, &well);
            assert_eq!(ships[0].condition, Condition::Disabled);
        }
        assert_eq!(combat.take_events(), [], "never newly disabled");
        assert_eq!(well.asked.take(), [], "the rule is not asked");
        assert_eq!(ships[0].reserves.armor.now, 20.0, "no armour back");
        assert_eq!(ships[0].reserves.shield.now, 3.0, "shields still do");
        ships[0].reserves.armor.now = 30.0;
        tick(&mut combat, &mut ships, &well);
        assert_eq!(ships[0].condition, Condition::Disabled, "at full armour");
    }

    #[test]
    fn a_held_ship_hit_strikes_no_downing() {
        let mut combat = Combat::default();
        let mut held = Ship::at(2, 100.0, 0.0);
        held.held_disabled = true;
        let mut ships = [Ship::at(1, 0.0, 0.0).armed(blaster()), held];
        let mut strikes = Vec::new();
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            strikes.extend(combat.take_strikes());
            if ships[1].reserves.armor.now <= 0.0 {
                break;
            }
        }
        let on_b: Vec<Option<Downed>> = strikes
            .iter()
            .filter(|strike| strike.ship == B)
            .map(|strike| strike.downed)
            .collect();
        assert!(!on_b.is_empty());
        assert!(
            on_b.iter().all(|downed| *downed != Some(Downed::Disabled)),
            "{on_b:?}"
        );
    }

    #[test]
    fn a_held_ship_whose_armour_runs_out_breaks_up_and_is_destroyed() {
        let mut combat = Combat::default();
        let mut held = Ship::at(2, 0.0, 0.0);
        held.held_disabled = true;
        held.condition = Condition::Disabled;
        held.reserves.armor.now = 0.0;
        let mut ships = [held];
        tick(&mut combat, &mut ships, &NovaDisable);
        let events = combat.take_events();
        assert!(matches!(events[0], CombatEvent::BreakingUp { ship: B, .. }));
        assert!(matches!(events[1], CombatEvent::Destroyed { ship: B, .. }));
        assert_eq!(ships[0].condition, Condition::Destroyed);
    }

    #[test]
    fn a_ship_no_longer_held_reads_its_armour_again_with_no_event() {
        let mut combat = Combat::default();
        let mut released = Ship::at(1, 0.0, 0.0);
        released.condition = Condition::Disabled;
        let mut ships = [released];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Intact, "full armour");
        assert_eq!(combat.take_events(), []);
        ships[0].condition = Condition::Disabled;
        ships[0].reserves.armor.now = 5.0;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(ships[0].condition, Condition::Disabled, "below a third");
        assert_eq!(combat.take_events(), [], "not newly disabled");
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
        ships[0].condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
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
            ships[0].trigger = QUIET;
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
        dying.condition = Condition::Dying {
            ticks_left: 50,
            starting: false,
        };
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
            ships[0].trigger = QUIET;
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
            ships[0].trigger = QUIET;
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
        ships[0].trigger = QUIET;
        ships[1].state.position = Vec2::new(100.0, 0.0);
        tick(&mut combat, &mut ships, &NovaDisable);
        let end = combat.beams()[0].end;
        assert!((end - Vec2::new(300.0, 0.0)).length() < 1e-3, "{end:?}");
        assert_eq!(ships[1].reserves.shield.now, 8.0, "hit both ticks");
        ships[1].condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
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
            ships[0].trigger = QUIET;
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
            ships[0].trigger = QUIET;
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
        defender.trigger = QUIET;
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

    #[test]
    fn a_ship_whose_trigger_does_not_ask_runs_no_point_defence() {
        let rule = Hostility {
            hostile: true,
            ..Hostility::default()
        };
        let rules = Rules {
            defence: &rule,
            ..Rules::default()
        };
        let defended = |point_defence: bool| {
            let mut combat = Combat::default();
            let mut ships = missile_attack(quad());
            ships[0].trigger = Trigger {
                point_defence,
                ..Trigger::default()
            };
            tick_among(
                &mut combat,
                &mut ships,
                &Arsenal::default(),
                &Governments::default(),
                rules,
            );
            combat
                .take_events()
                .iter()
                .filter(|e| matches!(e, CombatEvent::Fired { ship: A, .. }))
                .count()
        };
        assert_eq!(defended(false), 0, "not asked");
        assert_eq!(defended(true), 1, "asked");
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
            ships[1].trigger = QUIET;
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
            turrets_only: false,
            bays: false,
            point_defence: true,
        };
        let mut defender = Ship::at(1, 0.0, 0.0).armed(quad()).facing(0.0);
        defender.trigger = QUIET;
        let mut ships = [defender, attacker];
        let mut combat = Combat::default();
        let mut events = Vec::new();
        for _ in 0..10 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
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
            turrets_only: false,
            bays: false,
            point_defence: true,
        };
        for _ in 0..12 {
            tick(&mut combat, &mut ships, &NovaDisable);
            ships[1].trigger = QUIET;
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
            ships[1].trigger = QUIET;
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
            ships[1].trigger = QUIET;
            ships[2].trigger = QUIET;
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
            ships[1].trigger = QUIET;
            ships[2].trigger = QUIET;
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
            ships[1].trigger = QUIET;
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
            assert_eq!(sub.aimed_at, Some(C), "aimed at its own target");
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
        assert_eq!(subs[0].aimed_at, Some(B));
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
        // Guidance 2, and a fighter bay, which only a ship launches from.
        for (guidance, ammo_type) in [(2, -1), (99, 144)] {
            let (shell, arsenal) = cluster_of(WeaponRecord {
                guidance,
                ammo_type,
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
            assert_eq!(combat.take_sorties(), [], "and no fighter");
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
                    guidance,
                }]
            );
        }
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
                downed: None,
                provokes: true,
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
                downed: None,
                provokes: true,
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
            provokes: true,
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
            provokes: true,
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
                downed: None,
                provokes: true,
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
                    downed: None,
                    provokes: true,
                },
                Strike {
                    ship: B,
                    by: C,
                    damage: 4.0,
                    downed: Some(Downed::Disabled),
                    provokes: true,
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

    // Fighter bays.

    /// Ship A at (10, 20), moving (1, 2) and facing right, its secondary
    /// trigger on a fighter bay, 149, of ship 144, 10 degrees inaccurate,
    /// holding 2 fighters, and targeting B, which is at (300, 0).
    fn carrier() -> [Ship; 2] {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            inaccuracy: 10,
            reload: 60,
            speed: 400,
            ..weapon(149)
        };
        let mut carrier = Ship::at(1, 10.0, 20.0).armed(bay);
        carrier.state.velocity = Vec2::new(1.0, 2.0);
        carrier.trigger = Trigger {
            secondary: Some(WeaponId(149)),
            ..Trigger::default()
        };
        carrier.rounds = BTreeMap::from([(WeaponId(149), 2)]);
        carrier.target = Some(B);
        [carrier, Ship::at(2, 300.0, 0.0)]
    }

    #[test]
    fn a_bays_launch_is_a_sortie_not_a_shot() {
        let mut combat = Combat::default();
        let mut ships = carrier();
        let from = ships[0].state;
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(combat.shots(), [], "no shot");
        assert_eq!(combat.beams(), []);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Fired {
                ship: A,
                weapon: WeaponId(149),
                at: Vec2::new(10.0, 20.0)
            }],
            "heard at the carrier"
        );
        assert_eq!(
            combat.take_sorties(),
            [Sortie {
                carrier: A,
                bay: WeaponId(149),
                ship: ShipId(144),
                from,
                heading: 99.0,
                target: Some(B),
            }],
            "heading 90, and the inaccuracy drawn: 19 of 20, less 10"
        );
        assert_eq!(combat.take_sorties(), [], "taken");
        assert_eq!(ships[0].rounds[&WeaponId(149)], 1, "a round spent");
    }

    #[test]
    fn a_sortie_without_a_target_has_none() {
        let mut combat = Combat::default();
        let [mut carrier, _] = carrier();
        carrier.target = None;
        tick(&mut combat, &mut [carrier], &NovaDisable);
        let sorties = combat.take_sorties();
        assert_eq!(sorties.len(), 1);
        assert_eq!(sorties[0].target, None);
    }

    // Stellars.

    use crate::catalog::StellarId;
    use crate::combat::weapon::PLANET_TYPE;

    /// [`blaster`], planet-type, exploding in `bööm` 130.
    fn planet_gun() -> WeaponRecord {
        WeaponRecord {
            flags2: PLANET_TYPE,
            explod_type: 2,
            ..blaster()
        }
    }

    /// Stellar 128 at (`x`, `y`), of `radius` and `strength`.
    fn stellar_at(x: f32, y: f32, radius: f32, strength: i64) -> StellarTarget {
        StellarTarget {
            stellar: StellarId(128),
            position: Vec2::new(x, y),
            radius,
            strength,
        }
    }

    /// Ship A at the centre fires `weapon` once to the right, among
    /// `others`; then the fight goes on for `ticks` ticks in all, among
    /// `stellars`, with `arsenal`. The events, and the strikes on
    /// stellars, in order.
    fn shoot_once(
        weapon: WeaponRecord,
        others: Vec<Ship>,
        stellars: &mut [StellarTarget],
        arsenal: &Arsenal,
        ticks: u32,
    ) -> (Combat, Vec<Ship>, Vec<CombatEvent>, Vec<StellarStrike>) {
        let mut combat = Combat::default();
        let mut ships = vec![Ship::at(1, 0.0, 0.0).armed(weapon)];
        ships.extend(others);
        let (mut events, mut strikes) = (Vec::new(), Vec::new());
        for _ in 0..ticks {
            tick_by(&mut combat, &mut ships, arsenal, stellars);
            ships[0] = Ship::at(1, 0.0, 0.0);
            events.extend(combat.take_events());
            strikes.extend(combat.take_stellar_strikes());
        }
        (combat, ships, events, strikes)
    }

    fn strike(damage: i64, destroyed: bool) -> StellarStrike {
        StellarStrike {
            stellar: StellarId(128),
            by: A,
            damage,
            destroyed,
        }
    }

    fn explosions(events: &[CombatEvent]) -> Vec<(Vec2, Explosion)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Exploded { at, explosion } => Some((at, explosion)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_planet_type_shot_entering_a_stellar_damages_it_by_its_mass_and_energy_and_is_gone() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (combat, _, events, strikes) = shoot_once(
            planet_gun(),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            7,
        );
        assert_eq!(strikes, [strike(14, false)], "4 mass and 10 energy");
        assert_eq!(stellars[0].strength, 86);
        assert_eq!(combat.shots(), [], "gone");
        let seen = explosions(&events);
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert!(
            (seen[0].0 - Vec2::new(80.0, 0.0)).length() < 1e-3,
            "{seen:?}"
        );
        assert_eq!(Some(seen[0].1), Explosion::decode(2));
    }

    #[test]
    fn a_shot_of_another_weapon_passes_through_a_stellar() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (combat, _, events, strikes) =
            shoot_once(blaster(), Vec::new(), &mut stellars, &Arsenal::default(), 8);
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
        assert_eq!(combat.shots().len(), 1, "flying on");
        assert_eq!(explosions(&events), []);
    }

    #[test]
    fn a_ship_in_the_shots_path_is_hit_before_the_stellar() {
        let mut stellars = [stellar_at(100.0, 0.0, 20.0, 100)];
        let (_, ships, _, strikes) = shoot_once(
            planet_gun(),
            vec![Ship::at(2, 100.0, 0.0)],
            &mut stellars,
            &Arsenal::default(),
            8,
        );
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
        assert!(ships[1].reserves.shield.now < 10.0, "the ship took it");
    }

    /// A shot 100 pixels a tick from the centre to the right, among
    /// `stellars`: 129 at (100, 0), edge at 80, and 128 at (50, 0), edge
    /// at 40, both on its first tick's path. Its strikes, and where it
    /// exploded.
    fn through_two(stellars: &mut [StellarTarget]) -> (Vec<StellarStrike>, Vec<Vec2>) {
        let fast = WeaponRecord {
            speed: 10_000,
            ..planet_gun()
        };
        let (_, _, events, strikes) =
            shoot_once(fast, Vec::new(), stellars, &Arsenal::default(), 1);
        let at = explosions(&events).iter().map(|(at, _)| *at).collect();
        (strikes, at)
    }

    fn far(strength: i64) -> StellarTarget {
        StellarTarget {
            stellar: StellarId(129),
            ..stellar_at(100.0, 0.0, 20.0, strength)
        }
    }

    fn near(strength: i64) -> StellarTarget {
        stellar_at(50.0, 0.0, 10.0, strength)
    }

    fn close(seen: &[Vec2], x: f32) -> bool {
        seen.len() == 1 && (seen[0] - Vec2::new(x, 0.0)).length() < 1e-3
    }

    #[test]
    fn a_shot_strikes_the_nearest_stellar_along_its_path_whatever_their_order() {
        let mut stellars = [far(100), near(100)];
        let (strikes, seen) = through_two(&mut stellars);
        assert_eq!(strikes, [strike(14, false)], "128, the nearer");
        assert_eq!((stellars[0].strength, stellars[1].strength), (100, 86));
        assert!(close(&seen, 40.0), "on the nearer's edge: {seen:?}");
    }

    #[test]
    fn a_shot_passes_a_stellar_shot_down_for_the_next_along_its_path() {
        let mut stellars = [near(-1), far(100)];
        let (strikes, seen) = through_two(&mut stellars);
        assert_eq!(
            strikes,
            [StellarStrike {
                stellar: StellarId(129),
                ..strike(14, false)
            }]
        );
        assert_eq!((stellars[0].strength, stellars[1].strength), (-1, 86));
        assert!(close(&seen, 80.0), "on the farther's edge: {seen:?}");
    }

    #[test]
    fn a_fast_shot_hits_a_stellar_it_passes_between_ticks() {
        let fast = WeaponRecord {
            speed: 10_000,
            ..planet_gun()
        };
        let mut stellars = [stellar_at(150.0, 0.0, 10.0, 100)];
        let (_, _, events, strikes) =
            shoot_once(fast, Vec::new(), &mut stellars, &Arsenal::default(), 3);
        assert_eq!(strikes, [strike(14, false)]);
        let seen = explosions(&events);
        assert!(
            (seen[0].0 - Vec2::new(140.0, 0.0)).length() < 1e-3,
            "{seen:?}"
        );
    }

    #[test]
    fn a_shot_hitting_a_stellar_blasts_no_ship_and_releases_no_sub_munitions() {
        let (shell, arsenal) = cluster(PLANET_TYPE);
        let shell = WeaponRecord {
            blast_radius: 100,
            flags: crate::combat::weapon::DETONATES,
            ..shell
        };
        let mut stellars = [stellar_at(15.0, 0.0, 10.0, 100)];
        let (combat, ships, _, strikes) = shoot_once(
            shell,
            vec![Ship::at(2, 15.0, 40.0)],
            &mut stellars,
            &arsenal,
            5,
        );
        assert_eq!(strikes.len(), 1);
        assert_eq!(
            ships[1].reserves,
            Reserves::full(10.0, 30.0, 100.0),
            "no blast"
        );
        assert_eq!(combat.shots(), [], "no sub-munitions");
    }

    #[test]
    fn a_shot_hits_a_stellar_only_once_it_has_flown_past_its_proximity_safety() {
        let safe = WeaponRecord {
            prox_safety: 3,
            ..planet_gun()
        };
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        for ticks in 1..=3 {
            let (_, _, _, strikes) =
                shoot_once(safe, Vec::new(), &mut stellars, &Arsenal::default(), ticks);
            assert_eq!(strikes, [], "{ticks}");
        }
        let (_, _, _, strikes) =
            shoot_once(safe, Vec::new(), &mut stellars, &Arsenal::default(), 4);
        assert_eq!(strikes, [strike(14, false)], "on its fourth tick");
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        let (_, _, _, strikes) = shoot_once(
            planet_gun(),
            Vec::new(),
            &mut stellars,
            &Arsenal::default(),
            1,
        );
        assert_eq!(strikes, [strike(14, false)], "none: at once");
    }

    /// Ships A, at the centre, and B, 50 pixels below, each fire a
    /// [`planet_gun`] once, to the right, inside stellar 128 of
    /// `strength`. Its strikes, and the
    /// shots left.
    fn two_shots_at(strength: i64) -> (Vec<StellarStrike>, usize, i64) {
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, strength)];
        let mut combat = Combat::default();
        let mut ships = [
            Ship::at(1, 0.0, 0.0).armed(planet_gun()),
            Ship::at(2, 0.0, 50.0).armed(planet_gun()),
        ];
        tick_by(&mut combat, &mut ships, &Arsenal::default(), &mut stellars);
        (
            combat.take_stellar_strikes(),
            combat.shots().len(),
            stellars[0].strength,
        )
    }

    #[test]
    fn a_stellar_shot_below_none_is_destroyed_once_and_later_shots_pass_through() {
        let (strikes, left, strength) = two_shots_at(10);
        assert_eq!(strikes, [strike(14, true)]);
        assert_eq!((left, strength), (1, -4), "the second flies on");
        let (strikes, left, strength) = two_shots_at(14);
        assert_eq!(
            strikes,
            [
                strike(14, false),
                StellarStrike {
                    by: B,
                    ..strike(14, true)
                }
            ],
            "none left is still standing"
        );
        assert_eq!((left, strength), (0, -14));
    }

    #[test]
    fn a_beam_never_hits_a_stellar() {
        let beam = WeaponRecord {
            flags2: PLANET_TYPE,
            ..laser()
        };
        let mut stellars = [stellar_at(0.0, 0.0, 1000.0, 100)];
        let (_, _, _, strikes) =
            shoot_once(beam, Vec::new(), &mut stellars, &Arsenal::default(), 3);
        assert_eq!(strikes, []);
        assert_eq!(stellars[0].strength, 100);
    }

    #[test]
    fn combat_reports_an_explosion_it_is_told_of() {
        let mut combat = Combat::default();
        let boom = Explosion::decode(7).expect("a bööm");
        combat.explode(Vec2::new(3.0, 4.0), boom);
        assert_eq!(
            combat.take_events(),
            [CombatEvent::Exploded {
                at: Vec2::new(3.0, 4.0),
                explosion: boom
            }]
        );
    }
    /// `ship` as the escape pod, or a class after it.
    fn pod(ship: Ship, class: ShipId) -> Ship {
        Ship { class, ..ship }
    }

    #[test]
    fn a_shot_beam_blast_or_shockwave_does_no_damage_to_the_escape_pod() {
        let shell = WeaponRecord {
            explod_type: 0,
            blast_radius: 30,
            ..blaster()
        };
        let after = ShipId(ESCAPE_POD_SHIP.0 + 1);
        for (ship_type, immune) in [
            (ESCAPE_POD_SHIP, true),
            (after, true),
            (ShipId(ESCAPE_POD_SHIP.0 - 1), false),
        ] {
            // A shot, then a blast beside it: A fires at B, C beside B.
            for record in [blaster(), shell, laser()] {
                let mut combat = Combat::default();
                let mut ships = [
                    Ship::at(1, 0.0, 0.0).armed(record),
                    pod(Ship::at(2, 100.0, 0.0), ship_type),
                    pod(Ship::at(3, 100.0, 25.0), ship_type),
                ];
                for _ in 0..12 {
                    tick(&mut combat, &mut ships, &NovaDisable);
                    if record.beam_length == 0 {
                        ships[0].trigger = QUIET;
                    }
                }
                let hurt = |ship: &Ship| left(ship) != (10.0, 30.0);
                assert_eq!(hurt(&ships[1]), !immune, "{ship_type:?} {record:?}");
                if record.blast_radius > 0 {
                    assert_eq!(hurt(&ships[2]), !immune, "blasted, {ship_type:?}");
                }
                let struck = combat.take_strikes().iter().any(|strike| strike.ship != A);
                assert_eq!(struck, !immune, "{ship_type:?} {record:?}");
                if record.beam_length == 0 {
                    assert_eq!(combat.shots(), [], "the shot is spent on it");
                }
            }
            // A shockwave: the pod is still pushed.
            let mut combat = Combat::default();
            let mut pushed = pod(holding(1, 40.0, 0.0, 100.0, 300.0, 300.0), ship_type);
            pushed.hull.mass = 200.0;
            let mut ships = [wreck(WRECK, 200.0, 0.0, 0.0), pushed];
            tick(&mut combat, &mut ships, &NovaDisable);
            assert_eq!(
                left(&ships[1]) == (100.0, 300.0),
                immune,
                "shockwave, {ship_type:?}"
            );
            assert_eq!(combat.take_strikes().is_empty(), immune);
            assert_eq!(combat.take_shoves().len(), 1, "pushed all the same");
        }
    }

    #[test]
    fn an_escape_pod_at_zero_armour_neither_breaks_up_nor_explodes() {
        for id in [ShipRef::Player, A] {
            let mut combat = Combat::default();
            let mut ship = pod(Ship::at(1, 0.0, 0.0), ESCAPE_POD_SHIP);
            ship.id = id;
            ship.fleet = id;
            ship.reserves.armor.now = 0.0;
            let mut ships = [ship];
            for _ in 0..20 {
                tick(&mut combat, &mut ships, &NovaDisable);
            }
            assert_eq!(ships[0].condition, Condition::Intact, "{id:?}");
            assert_eq!(about(&combat.take_events(), id), [], "{id:?}");
        }
        // Not so the class before it.
        let mut combat = Combat::default();
        let mut ship = pod(Ship::at(1, 0.0, 0.0), ShipId(ESCAPE_POD_SHIP.0 - 1));
        ship.reserves.armor.now = 0.0;
        let mut ships = [ship];
        tick(&mut combat, &mut ships, &NovaDisable);
        assert_ne!(ships[0].condition, Condition::Intact);
    }
}
