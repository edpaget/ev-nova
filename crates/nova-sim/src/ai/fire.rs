//! Which weapon an NPC fires: the one each tick it proposes, as it
//! attacks, snipes at or flees from the ship its goal is about.
//!
//! The rules are the original's (`_AIFireGun` @0x7feb6, `_AIFireTurret`
//! @0x80ad3, `_AIFireMissile` @0x8115d, `_FireAIShipWeapon` @0x8873d and
//! `_AIHasDestroyingWeapons` @0x7fc7c in the `EV Nova` executable). Each
//! tick an NPC proposes one weapon ([`trigger`], a [`Trigger::only`]);
//! the combat core's firing table then aims it. While it attacks or
//! snipes at a ship that is live in the system, it also launches its
//! fighters ([`Trigger::bays`], `_AILaunchFighter` @0x81372 from the
//! attack state, see [`bay`](crate::bay)).
//!
//! - **Its manoeuvre** ([`Manoeuvre`]): attacking, it approaches outside
//!   [`DOGFIGHT_BOX`] on either axis and dogfights within it; it snipes;
//!   or it flees.
//! - **The heading it wants** ([`wanted_heading`]): the lead angle for its
//!   first forward gun's speed, or the plain bearing with none.
//! - **A gun** (guidance -1, 0 and 6; a homing weapon only while no other
//!   gun has its ammunition, @0x80243): reloaded and affordable; in range, the distance at
//!   most its range plus [`RANGE_MARGIN`] (a beam's range is its length);
//!   a rocket with a `ProxRadius` only while the target is
//!   [`PROX_CLEARANCE`] times it away on both axes. Of those, the most
//!   `EnergyDmg` while the target's shields are up, else the most
//!   `MassDmg`, a value of none or less counting as 1, a tie to the
//!   earlier mount. Guidance 5 is never chosen.
//! - **A turret** (3, 4, 7 and 8): reloaded, affordable, in range, not in
//!   a blind spot, and 7 and 8 only with the target within [`FRONT_ARC`]
//!   of the nose and of the tail. Chosen by the same shield rule. None
//!   at a target whose ship type is untargetable (`shïp` `Flags2` 0x0004,
//!   [`UNTARGETABLE`](crate::cloak::UNTARGETABLE), `_AIFireTurret`
//!   @0x80b51), whatever the shooter's scanner.
//! - **A missile** (1, `_AIFireMissile` @0x8115d): none at a target whose
//!   ship type is untargetable unless the shooter's ship type carries a
//!   scanner with `ModVal` bit 0x0004 (@0x811a7-0x811cd,
//!   [`CloakGear::scanner`](crate::cloak::CloakGear::scanner)), nor at a
//!   cloaked target unless it carries one with bit 0x0008
//!   (`_ShipCanTargetCloakedShips` @0x811e7-0x811fd); the player counts
//!   its own ship type's bit ([`PlayerSide::untargetable`]) and never
//!   cloaks. Then the first homing weapon that has its ammunition and
//!   whose range squared is at least [`MISSILE_RANGE`] of the distance
//!   squared (@0x8126b-0x8132b), fired only if it has reloaded
//!   (@0x81349-0x8135d): one reloading does not give way to a later one.
//! - **By manoeuvre**, a later pick overriding an earlier: approaching,
//!   a gun, then a turret (each while facing within [`GUN_FACING`] times
//!   the turn rate of the heading it wants), then a missile (within
//!   [`MISSILE_FACING`] times); dogfighting, a turret, then a gun (within
//!   [`GUN_FACING`]); sniping, a gun and a missile (within
//!   [`GUN_FACING`]), then a turret; fleeing, a turret alone, at the
//!   attacker.
//! - **Destroying weapons** ([`destroys`]): a weapon that does mass damage,
//!   is not `Flags2` 0x1000, is guided -1, 0, 3, 4, 6, 7 or 8, and has its
//!   ammunition. A ship with none leaves a disabled target alone.
//! - There is no friendly-fire check, as in the original. Guns
//!   (`_AIFireGun`) never read the untargetable bit.
//!
//! # Not modelled
//!
//! The difficulty reload multiplier against the player. In
//! `_AIFireMissile`: `_SufficentTargetedDamage` (@0x81215), the
//! weapon's `_SuitableMissileType` (@0x812ab), the match of the weapon's
//! flag 0x0400 (wëap +0x22) against the target's ship type's `Flags`
//! 0x0400 (@0x812b4-0x812fb), the rest of `_WeaponHasAmmo` beyond
//! ammunition and fuel (@0x81307), and its walk in weapon-table order
//! rather than mount order. In `_AIFireTurret`'s preamble,
//! `_ShipVisibleToShip` (@0x80b69), which keeps a turret off a target its
//! shooter does not see, and the call of `_HandleShipPointDefense`
//! (@0x80b0d) from there rather than every tick (point defence itself is
//! [`defence`](crate::combat::defence); a disabled shooter, `_IsDisabled`
//! @0x80afa, fires nothing in [`Armament::fire`]). rdm phases
//! player-led-ships/phase-23-ai-fire-missile-rest and
//! player-led-ships/phase-24-ai-fire-turret-sight take them.

#[cfg(doc)]
use crate::ai::PlayerSide;
use crate::ai::{Goal, Surroundings};
use crate::catalog::WeaponId;
use crate::combat::ShipRef;
use crate::combat::aim::{FRONT_ARC, angle_off, bearing, blind, lead};
#[cfg(doc)]
use crate::combat::armament::Armament;
use crate::combat::armament::{Mount, Shooter, Trigger};
use crate::combat::projectile::Target;
use crate::combat::weapon::{Ammo, Guidance, WeaponSpec};
use crate::flight::ShipState;
use crate::traffic::npc::Npc;

/// How near, in pixels on either axis, an attacker dogfights rather than
/// approaches (165.0 @0xdda24).
pub const DOGFIGHT_BOX: f32 = 165.0;
/// What a gun reaches beyond its range, in pixels (32.0 @0xdd654).
pub const RANGE_MARGIN: f32 = 32.0;
/// How many of its `ProxRadius` a rocket's target must be away on both
/// axes (2.5 @0xdda60).
pub const PROX_CLEARANCE: f32 = 2.5;
/// How many turns' worth a gun or turret may be off the heading wanted
/// (3.0 @0xdd664).
pub const GUN_FACING: f32 = 3.0;
/// How many turns' worth a missile may be off it while approaching (4.0
/// @0xdd680).
pub const MISSILE_FACING: f32 = 4.0;
/// The share of the distance squared a missile's range squared must
/// reach (0.95 @0xdd618).
pub const MISSILE_RANGE: f32 = 0.95;
/// `Flags2`: a weapon that does not destroy.
pub const NOT_DESTROYING: u16 = 0x1000;

/// How an NPC flies its fight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manoeuvre {
    /// Closing on its target from afar.
    Approach,
    /// Turning with its target, close in.
    Dogfight,
    /// Standing off and firing.
    Snipe,
    /// Running from its attacker.
    Flee,
}

/// How a ship at `from` flies `goal` at a ship at `at`: none for a goal
/// that is no fight.
#[must_use]
pub fn manoeuvre(goal: Goal, from: &ShipState, at: &ShipState) -> Option<Manoeuvre> {
    match goal {
        Goal::Attack(_) => {
            let off = at.position - from.position;
            Some(
                if off.x.abs() > DOGFIGHT_BOX || off.y.abs() > DOGFIGHT_BOX {
                    Manoeuvre::Approach
                } else {
                    Manoeuvre::Dogfight
                },
            )
        }
        Goal::Snipe(_) => Some(Manoeuvre::Snipe),
        Goal::Flee(_) => Some(Manoeuvre::Flee),
        _ => None,
    }
}

/// Whether `spec` fires forward: guidance -1, 0 or 6.
fn forward(spec: &WeaponSpec) -> bool {
    matches!(
        spec.guidance,
        Guidance::Unguided | Guidance::Beam | Guidance::Rocket
    )
}

/// The heading `npc` wants to face to fire at a ship at `at` (see the
/// module docs).
#[must_use]
pub fn wanted_heading(npc: &Npc, at: &ShipState) -> f32 {
    let speed = npc
        .armament
        .mounts()
        .iter()
        .find(|mount| forward(&mount.spec))
        .map_or(0.0, |mount| mount.spec.speed);
    let target = Target {
        ship: ShipRef::Player,
        fleet: ShipRef::Player,
        position: at.position,
        velocity: at.velocity,
        radius: 0.0,
        condition: crate::combat::hull::Condition::Intact,
    };
    lead(&npc.state, &target, speed)
}

/// Whether some weapon `npc` carries reaches a ship at `at`: within its
/// range and [`RANGE_MARGIN`] (`_AIInGunRange` @0x7f9d5 with no weapon
/// named).
#[must_use]
pub fn in_reach(npc: &Npc, at: &ShipState) -> bool {
    let off = at.position - npc.state.position;
    let squared = off.x * off.x + off.y * off.y;
    npc.armament.mounts().iter().any(|mount| {
        let reach = mount.spec.range() + RANGE_MARGIN;
        squared <= reach * reach
    })
}

/// Whether `spec` destroys (see the module docs).
fn destroying(spec: &WeaponSpec) -> bool {
    spec.mass_damage > 0.0
        && spec.flags2 & NOT_DESTROYING == 0
        && matches!(
            spec.guidance,
            Guidance::Unguided
                | Guidance::Beam
                | Guidance::TurretBeam
                | Guidance::Turret
                | Guidance::Rocket
                | Guidance::FrontTurret
                | Guidance::RearTurret
        )
}

/// Whether `npc` has the ammunition for `mount`, one of its own, as
/// `_WeaponHasAmmo` says ([`Armament::has_ammo`]): an NPC, cloaked or
/// not, of its class's `KeyCarried`, with its rounds and fuel.
#[must_use]
pub fn has_ammo(npc: &Npc, mount: &Mount) -> bool {
    let shooter = Shooter {
        npc: true,
        cloaked: npc.cloak.is_cloaked(),
        key_carried: npc.hull.key_carried,
    };
    npc.armament
        .has_ammo(mount, &npc.rounds, npc.reserves.fuel, shooter)
}

/// Whether `npc` holds a destroying weapon (see the module docs).
#[must_use]
pub fn destroys(npc: &Npc) -> bool {
    npc.armament
        .mounts()
        .iter()
        .any(|mount| destroying(&mount.spec) && has_ammo(npc, mount))
}

/// How much of its weapons a ship can still fire (`_AIShipIsOutOfAmmo`
/// @0x7f71f, which returns 0, 1 or 2 for these in order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmmoLeft {
    /// Every weapon it holds can fire, or some ammunition- or fuel-using
    /// one still can.
    Armed,
    /// Every ammunition- or fuel-using weapon is out, but a weapon of
    /// unlimited ammunition remains (the original's `AmmoType`
    /// -1000..-1, @0x7f750-0x7f75f; here [`Ammo::Unlimited`] and
    /// [`Ammo::Other`]).
    OnlyUnlimited,
    /// It holds no weapon, or every weapon it holds is out of rounds
    /// (a fighter bay's rounds are its fighters) or of fuel (a shot
    /// costing more than the fuel left, @0x7f797).
    Dry,
}

/// How much of its weapons `npc` can still fire (see [`AmmoLeft`]).
///
/// Only [`AmmoLeft::Dry`] is read today, by
/// [`select_target`](crate::ai::target::select_target); the escort's
/// fallback that reads the others (`_EscortAI` @0x83bb5) stays with
/// rdm:task/escorts-follow-up.
#[must_use]
pub fn ammo_left(npc: &Npc) -> AmmoLeft {
    let mut unlimited = false;
    let mut spending = false;
    for mount in npc.armament.mounts() {
        if matches!(mount.spec.ammo, Ammo::Unlimited | Ammo::Other(_)) {
            unlimited = true;
        } else if mount.affords(&npc.rounds, npc.reserves.fuel) {
            return AmmoLeft::Armed;
        } else {
            spending = true;
        }
    }
    match (spending, unlimited) {
        (false, true) => AmmoLeft::Armed,
        (true, true) => AmmoLeft::OnlyUnlimited,
        (_, false) => AmmoLeft::Dry,
    }
}

/// The weapon `npc` fires now among `around`, if any (see the module
/// docs), and its fighter bays while it attacks or snipes at a ship that
/// is live there.
#[must_use]
pub fn trigger(npc: &Npc, around: &Surroundings) -> Trigger {
    let only = pick(npc, around);
    Trigger {
        only,
        bays: npc.goal.attacking().is_some_and(|ship| around.live(ship)),
        ..Trigger::default()
    }
}

/// The weapon [`trigger`] picks, if any.
fn pick(npc: &Npc, around: &Surroundings) -> Option<WeaponId> {
    let ship = npc.goal.quarry()?;
    if !around.live(ship) {
        return None;
    }
    let at = around.state_of(ship)?;
    let shields_up = match ship {
        ShipRef::Player => around.player.map(|player| player.reserves),
        ShipRef::Npc(id) => around.npc(id).map(|other| other.reserves),
    }
    .is_some_and(|reserves| reserves.shield.now > 0.0);
    // The player has no cloak.
    let (untargetable, cloaked) = match ship {
        ShipRef::Player => (
            around.player.is_some_and(|player| player.untargetable),
            false,
        ),
        ShipRef::Npc(id) => around.npc(id).map_or((false, false), |other| {
            (other.cloak_gear.untargetable(), other.cloak.is_cloaked())
        }),
    };
    let manoeuvre = manoeuvre(npc.goal, &npc.state, &at)?;
    let off = angle_off(npc.state.heading, wanted_heading(npc, &at));
    let facing = |turns: f32| off <= turns * npc.stats.handling.turn_rate;
    let sight = Sight {
        npc,
        at: &at,
        shields_up,
        untargetable,
        cloaked,
    };
    let gun = || facing(GUN_FACING).then(|| sight.gun()).flatten();
    let turret = || sight.turret();
    let missile = |turns: f32| facing(turns).then(|| sight.missile()).flatten();
    match manoeuvre {
        Manoeuvre::Approach => missile(MISSILE_FACING)
            .or_else(|| facing(GUN_FACING).then(turret).flatten())
            .or_else(gun),
        Manoeuvre::Dogfight => gun().or_else(turret),
        Manoeuvre::Snipe => turret().or_else(|| missile(GUN_FACING)).or_else(gun),
        Manoeuvre::Flee => turret(),
    }
}

/// An NPC sizing up its target at `at`.
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
struct Sight<'a> {
    npc: &'a Npc,
    at: &'a ShipState,
    shields_up: bool,
    /// Whether the target's ship type is untargetable.
    untargetable: bool,
    /// Whether the target is cloaked.
    cloaked: bool,
}

impl Sight<'_> {
    /// How far the target is, squared.
    fn distance_squared(&self) -> f32 {
        let off = self.at.position - self.npc.state.position;
        off.x * off.x + off.y * off.y
    }

    /// Whether `mount` is ready: reloaded and paid for.
    fn ready(&self, mount: &Mount) -> bool {
        mount.reload <= 0.0 && has_ammo(self.npc, mount)
    }

    /// Whether `spec` reaches the target: within its range and the margin.
    fn reaches(&self, spec: &WeaponSpec) -> bool {
        let reach = spec.range() + RANGE_MARGIN;
        self.distance_squared() <= reach * reach
    }

    /// The damage `spec` counts for: its energy while the target's
    /// shields are up, else its mass, none or less counting as 1.
    fn worth(&self, spec: &WeaponSpec) -> f32 {
        let damage = if self.shields_up {
            spec.energy_damage
        } else {
            spec.mass_damage
        };
        if damage <= 0.0 { 1.0 } else { damage }
    }

    /// The most damaging of the mounts that pass `fits`, a tie to the
    /// earlier.
    fn best(&self, fits: impl Fn(&Mount) -> bool) -> Option<WeaponId> {
        let mut best: Option<(&Mount, f32)> = None;
        for mount in self
            .npc
            .armament
            .mounts()
            .iter()
            .filter(|mount| fits(mount))
        {
            let worth = self.worth(&mount.spec);
            if best.is_none_or(|(_, most)| worth > most) {
                best = Some((mount, worth));
            }
        }
        best.map(|(mount, _)| mount.spec.id)
    }

    /// Whether a rocket's proximity fuse leaves it room: the target its
    /// `ProxRadius` times [`PROX_CLEARANCE`] away on both axes.
    fn clear_of(&self, spec: &WeaponSpec) -> bool {
        let off = self.at.position - self.npc.state.position;
        let room = spec.prox_radius * PROX_CLEARANCE;
        spec.guidance != Guidance::Rocket
            || spec.prox_radius <= 0.0
            || (off.x.abs() >= room && off.y.abs() >= room)
    }

    /// The gun it fires, if any (see the module docs).
    fn gun(&self) -> Option<WeaponId> {
        let usable = |mount: &Mount| {
            self.ready(mount) && self.reaches(&mount.spec) && self.clear_of(&mount.spec)
        };
        let armed = self
            .npc
            .armament
            .mounts()
            .iter()
            .any(|mount| forward(&mount.spec) && has_ammo(self.npc, mount));
        let gun =
            |spec: &WeaponSpec| forward(spec) || (!armed && spec.guidance == Guidance::Homing);
        self.best(|mount| gun(&mount.spec) && usable(mount))
    }

    /// The turret it fires, if any (see the module docs).
    fn turret(&self) -> Option<WeaponId> {
        if self.untargetable {
            return None;
        }
        let bearing_off = angle_off(
            self.npc.state.heading,
            bearing(self.npc.state.position, self.at.position),
        );
        self.best(|mount| {
            let spec = &mount.spec;
            let in_arc = match spec.guidance {
                Guidance::TurretBeam | Guidance::Turret => true,
                Guidance::FrontTurret => bearing_off <= FRONT_ARC,
                Guidance::RearTurret => bearing_off >= 180.0 - FRONT_ARC,
                _ => false,
            };
            in_arc
                && !blind(spec.flags | self.npc.hull.blind_spots, bearing_off)
                && self.ready(mount)
                && self.reaches(spec)
        })
    }

    /// The missile it fires, if any (see the module docs).
    fn missile(&self) -> Option<WeaponId> {
        let scanner = self.npc.cloak_gear.scanner;
        if (self.untargetable && !scanner.untargetable) || (self.cloaked && !scanner.targets) {
            return None;
        }
        let distance = self.distance_squared();
        self.npc
            .armament
            .mounts()
            .iter()
            .find(|mount| {
                let range = mount.spec.range();
                mount.spec.guidance == Guidance::Homing
                    && has_ammo(self.npc, mount)
                    && range * range >= MISSILE_RANGE * distance
            })
            .filter(|mount| mount.reload <= 0.0)
            .map(|mount| mount.spec.id)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::ai::PlayerSide;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::{Condition, HullSpec};
    use crate::geometry::Vec2;
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, weapon};
    use crate::traffic::npc::NpcId;

    /// A gun of `id`, 10 pixels a tick for 20 ticks (200 pixels), doing
    /// `energy` and `mass` damage.
    fn gun(id: i16, energy: i16, mass: i16) -> WeaponRecord {
        WeaponRecord {
            speed: 1000,
            count: 20,
            energy_dmg: energy,
            mass_dmg: mass,
            ..weapon(id)
        }
    }

    /// A weapon of `guidance` like [`gun`].
    fn guided(id: i16, guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            ..gun(id, 5, 5)
        }
    }

    /// A homing missile of `id` reaching 300 pixels.
    fn missile(id: i16) -> WeaponRecord {
        WeaponRecord {
            guidance: 1,
            count: 30,
            ..gun(id, 5, 5)
        }
    }

    /// Where the shooter is: off the centre, so every offset is measured
    /// from it.
    const SHOOTER: Vec2 = Vec2::new(300.0, 200.0);

    /// An NPC at [`SHOOTER`] facing up (3 degrees a tick), carrying one of
    /// each of `weapons`, with `goal`.
    fn shooter(weapons: &[WeaponRecord], goal: Goal) -> Npc {
        let mut npc = Npc {
            armament: Armament::new(weapons.iter().map(|record| (WeaponSpec::new(record), 1))),
            goal,
            ..crate::testkit::npc(1, ShipStats::new(FAST, &[]))
        };
        npc.state.position = SHOOTER;
        npc
    }

    /// The player at (`x`, `y`), at rest, its shields at `shield`.
    fn player_at(x: f32, y: f32, shield: f32) -> PlayerSide {
        PlayerSide {
            state: ShipState {
                position: Vec2::new(x, y),
                ..ShipState::default()
            },
            condition: Condition::Intact,
            reserves: Reserves {
                shield: crate::reserves::Gauge {
                    now: shield,
                    max: 100.0,
                },
                ..Reserves::full(100.0, 100.0, 100.0)
            },
            hull: HullSpec::default(),
            handling: ShipStats::new(FAST, &[]).handling,
            rating: 0,
            yardstick: 0,
            untargetable: false,
        }
    }

    const ATTACK: Goal = Goal::Attack(ShipRef::Player);

    /// What `npc` fires at the player (`x`, `y`) from [`SHOOTER`] with
    /// `shield`.
    fn fired(npc: &Npc, x: f32, y: f32, shield: f32) -> Option<WeaponId> {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x + x, SHOOTER.y + y, shield)),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around).only
    }

    /// What `npc` fires at the player (`x`, `y`) from [`SHOOTER`], its
    /// shields up and its ship type untargetable.
    fn fired_at_untargetable(npc: &Npc, x: f32, y: f32) -> Option<WeaponId> {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(PlayerSide {
                untargetable: true,
                ..player_at(SHOOTER.x + x, SHOOTER.y + y, 50.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around).only
    }

    /// What `npc`, attacking NPC 2, fires at it as `target` makes it,
    /// (`x`, `y`) from [`SHOOTER`].
    fn fired_at_npc(npc: &Npc, x: f32, y: f32, target: impl Fn(&mut Npc)) -> Option<WeaponId> {
        let mut npc = npc.clone();
        npc.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
        let mut other = crate::testkit::npc(2, ShipStats::new(FAST, &[]));
        other.state.position = SHOOTER + Vec2::new(x, y);
        target(&mut other);
        let npcs = [npc.clone(), other];
        trigger(&npc, &Surroundings::new(&[], &npcs)).only
    }

    /// `npc` with a scanner of these bits among its ship type's items.
    fn scanning(mut npc: Npc, scanner: crate::cloak::Scanner) -> Npc {
        npc.cloak_gear.scanner = scanner;
        npc
    }

    const SEES_UNTARGETABLE: crate::cloak::Scanner = crate::cloak::Scanner {
        untargetable: true,
        ..crate::cloak::Scanner::NONE
    };

    const SEES_CLOAKED: crate::cloak::Scanner = crate::cloak::Scanner {
        targets: true,
        ..crate::cloak::Scanner::NONE
    };

    #[test]
    fn a_missile_flies_at_an_untargetable_player_only_with_the_shooters_scanner() {
        // A short gun keeps the missile from being a gun.
        let npc = shooter(&[short(128), missile(131)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(131));
        assert_eq!(fired_at_untargetable(&npc, 0.0, -300.0), None);
        let cloak_only = scanning(npc.clone(), SEES_CLOAKED);
        assert_eq!(fired_at_untargetable(&cloak_only, 0.0, -300.0), None);
        let sees = scanning(npc, SEES_UNTARGETABLE);
        assert_eq!(fired_at_untargetable(&sees, 0.0, -300.0), id(131));
    }

    #[test]
    fn a_missile_flies_at_an_untargetable_npc_only_with_the_shooters_scanner() {
        let npc = shooter(&[short(128), missile(131)], ATTACK);
        let plain = |_: &mut Npc| {};
        let untargetable = |other: &mut Npc| other.cloak_gear.flags2 = crate::cloak::UNTARGETABLE;
        assert_eq!(fired_at_npc(&npc, 0.0, -300.0, plain), id(131));
        assert_eq!(fired_at_npc(&npc, 0.0, -300.0, untargetable), None);
        let sees = scanning(npc, SEES_UNTARGETABLE);
        assert_eq!(fired_at_npc(&sees, 0.0, -300.0, untargetable), id(131));
    }

    #[test]
    fn a_missile_flies_at_a_cloaked_npc_only_with_the_shooters_cloak_bit() {
        let npc = shooter(&[short(128), missile(131)], ATTACK);
        let cloaked = |other: &mut Npc| {
            other.cloak = crate::cloak::Cloak {
                fade: crate::cloak::FADE_FULL,
                direction: 0,
            };
        };
        assert_eq!(fired_at_npc(&npc, 0.0, -300.0, cloaked), None);
        let untargetable_bit = scanning(npc.clone(), SEES_UNTARGETABLE);
        assert_eq!(fired_at_npc(&untargetable_bit, 0.0, -300.0, cloaked), None);
        let sees = scanning(npc, SEES_CLOAKED);
        assert_eq!(fired_at_npc(&sees, 0.0, -300.0, cloaked), id(131));
    }

    #[test]
    fn a_turret_never_fires_at_an_untargetable_ship_and_a_gun_still_does() {
        let turret = shooter(&[guided(130, 4)], Goal::Flee(ShipRef::Player));
        assert_eq!(fired(&turret, 0.0, 200.0, 50.0), id(130));
        assert_eq!(fired_at_untargetable(&turret, 0.0, 200.0), None);
        let sees = scanning(turret, SEES_UNTARGETABLE);
        assert_eq!(
            fired_at_untargetable(&sees, 0.0, 200.0),
            None,
            "whatever the scanner"
        );
        let mut at_npc = shooter(&[guided(130, 4)], ATTACK);
        at_npc.state.heading = 180.0;
        let untargetable = |other: &mut Npc| other.cloak_gear.flags2 = crate::cloak::UNTARGETABLE;
        assert_eq!(fired_at_npc(&at_npc, 0.0, 100.0, |_| {}), id(130));
        assert_eq!(fired_at_npc(&at_npc, 0.0, 100.0, untargetable), None);
        let gun = shooter(&[gun(128, 1, 1)], ATTACK);
        assert_eq!(fired_at_untargetable(&gun, 0.0, -100.0), id(128));
    }

    #[test]
    fn a_missile_out_of_ammunition_gives_way_to_a_later_one_in_range() {
        let empty = WeaponRecord {
            ammo_type: 10,
            ..missile(131)
        };
        let npc = shooter(&[short(128), empty, missile(132)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(132));
        let slow = WeaponRecord {
            reload: 10,
            ..missile(131)
        };
        let mut reloading = shooter(&[short(128), slow, missile(132)], ATTACK);
        let mut fuel = reloading.reserves.fuel;
        reloading.armament.fire(
            Trigger {
                only: Some(WeaponId(131)),
                ..Trigger::default()
            },
            Condition::Intact,
            crate::combat::armament::Shooter::PLAYER,
            &mut BTreeMap::new(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            &mut |_| {
                Some(crate::combat::aim::Aim {
                    heading: 0.0,
                    target: None,
                })
            },
        );
        assert_eq!(
            fired(&reloading, 0.0, -300.0, 50.0),
            None,
            "a reload does not give way"
        );
    }

    #[allow(clippy::unnecessary_wraps)]
    fn id(id: i16) -> Option<WeaponId> {
        Some(WeaponId(id))
    }

    #[test]
    fn some_weapon_reaches_within_its_range_and_the_margin() {
        // A turret reaching 300 pixels, and 332 with the margin.
        let mut npc = crate::testkit::npc(1, crate::stats::ShipStats::default());
        npc.state.position = crate::geometry::Vec2::new(100.0, 50.0);
        let at = |x: f32, y: f32| ShipState {
            position: crate::geometry::Vec2::new(x, y),
            ..ShipState::default()
        };
        assert!(!in_reach(&npc, &at(101.0, 50.0)), "unarmed");
        npc.armament = crate::combat::armament::Armament::new([(
            WeaponSpec::new(&crate::catalog::WeaponRecord {
                guidance: 4,
                speed: 1000,
                count: 30,
                ..crate::testkit::weapon(140)
            }),
            1,
        )]);
        // 300² + 100² = 100000 is within 332² = 110224; 320² + 100² is not.
        assert!(in_reach(&npc, &at(400.0, 150.0)));
        assert!(in_reach(&npc, &at(-200.0, -50.0)));
        assert!(!in_reach(&npc, &at(420.0, 150.0)));
        assert!(!in_reach(&npc, &at(100.0, 383.0)));
    }

    #[test]
    fn the_named_values() {
        assert_eq!(
            [
                DOGFIGHT_BOX,
                RANGE_MARGIN,
                PROX_CLEARANCE,
                GUN_FACING,
                MISSILE_FACING,
                MISSILE_RANGE
            ],
            [165.0, 32.0, 2.5, 3.0, 4.0, 0.95]
        );
        assert_eq!(NOT_DESTROYING, 0x1000);
    }

    #[test]
    fn an_attack_approaches_outside_the_box_and_dogfights_within_it() {
        let from = ShipState {
            position: SHOOTER,
            ..ShipState::default()
        };
        let at = |x: f32, y: f32| ShipState {
            position: SHOOTER + Vec2::new(x, y),
            ..ShipState::default()
        };
        let target = ShipRef::Npc(NpcId(2));
        let of = |goal, x, y| manoeuvre(goal, &from, &at(x, y));
        assert_eq!(
            of(Goal::Attack(target), 165.0, -165.0),
            Some(Manoeuvre::Dogfight)
        );
        assert_eq!(
            of(Goal::Attack(target), 165.1, 0.0),
            Some(Manoeuvre::Approach)
        );
        assert_eq!(
            of(Goal::Attack(target), 0.0, 165.1),
            Some(Manoeuvre::Approach)
        );
        assert_eq!(of(Goal::Snipe(target), 10.0, 0.0), Some(Manoeuvre::Snipe));
        assert_eq!(of(Goal::Flee(target), 10.0, 0.0), Some(Manoeuvre::Flee));
        assert_eq!(of(Goal::Inspect(target), 10.0, 0.0), None);
        assert_eq!(of(Goal::Idle, 10.0, 0.0), None);
    }

    #[test]
    fn the_heading_wanted_leads_with_the_first_forward_gun_or_bears_without() {
        let mut npc = shooter(&[missile(131), gun(128, 1, 1)], ATTACK);
        npc.state.velocity = Vec2::new(-1.0, 0.0);
        let crossing = ShipState {
            position: SHOOTER + Vec2::new(0.0, -100.0),
            velocity: Vec2::new(4.0, 0.0),
            heading: 0.0,
        };
        // 100 pixels at 10 a tick: 10 ticks, 50 to the right of it,
        // relative to the shooter's own motion.
        let expected = bearing(Vec2::ZERO, Vec2::new(50.0, -100.0));
        assert!((wanted_heading(&npc, &crossing) - expected).abs() < 1e-3);
        npc.armament = Armament::new([(WeaponSpec::new(&missile(131)), 1)]);
        assert_eq!(wanted_heading(&npc, &crossing), 0.0, "the bearing");
    }

    #[test]
    fn a_gun_does_the_most_energy_damage_to_shields_and_mass_to_armour() {
        let npc = shooter(&[gun(128, 10, 1), gun(129, 2, 8)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(128), "shields up");
        assert_eq!(fired(&npc, 0.0, -200.0, 0.0), id(129), "shields down");
    }

    #[test]
    fn a_damage_of_none_or_less_counts_as_1_and_a_tie_goes_to_the_earlier() {
        let npc = shooter(&[gun(128, -5, 0), gun(129, 0, 0), gun(130, 1, 1)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(128));
        let npc = shooter(&[gun(128, 1, 1), gun(129, 2, 0)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 0.0), id(128), "mass 0 counts as 1");
    }

    #[test]
    fn a_gun_reaches_32_beyond_its_range() {
        let npc = shooter(&[gun(128, 1, 1)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -232.0, 50.0), id(128));
        assert_eq!(fired(&npc, 0.0, -232.1, 50.0), None);
        let beam = WeaponRecord {
            guidance: 0,
            beam_length: 100,
            ..gun(129, 1, 1)
        };
        let npc = shooter(&[beam], ATTACK);
        assert_eq!(fired(&npc, 0.0, -132.0, 50.0), id(129), "its length");
        assert_eq!(fired(&npc, 0.0, -132.1, 50.0), None);
    }

    #[test]
    fn a_rocket_with_a_proximity_fuse_needs_room_on_both_axes() {
        let rocket = WeaponRecord {
            guidance: 6,
            prox_radius: 10,
            ..gun(128, 1, 1)
        };
        let npc = shooter(&[rocket], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None, "none across");
        let mut leaning = npc.clone();
        leaning.state.heading = 7.0;
        assert_eq!(fired(&leaning, 25.0, -200.0, 50.0), id(128));
        assert_eq!(fired(&leaning, 24.9, -200.0, 50.0), None);
        let unfused = shooter(
            &[WeaponRecord {
                guidance: 6,
                ..gun(128, 1, 1)
            }],
            ATTACK,
        );
        assert_eq!(fired(&unfused, 0.0, -200.0, 50.0), id(128));
        let fused_gun = shooter(
            &[WeaponRecord {
                prox_radius: 10,
                ..gun(128, 1, 1)
            }],
            ATTACK,
        );
        assert_eq!(
            fired(&fused_gun, 0.0, -200.0, 50.0),
            id(128),
            "only a rocket's fuse needs room"
        );
    }

    #[test]
    fn a_freefall_bomb_is_never_chosen() {
        let npc = shooter(&[guided(128, 5)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None);
    }

    #[test]
    fn a_gun_fires_only_facing_within_three_turns_and_a_missile_four() {
        // 3 degrees a tick: 9 for a gun, 12 for a missile.
        let mut npc = shooter(&[gun(128, 1, 1)], ATTACK);
        npc.state.heading = 9.0;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(128));
        npc.state.heading = 9.1;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None);
        let mut npc = shooter(&[missile(131)], ATTACK);
        npc.state.heading = 348.0;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(131));
        npc.state.heading = 347.9;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None);
    }

    #[test]
    fn a_turret_fires_out_of_its_blind_spots_and_a_quadrant_turret_in_its_arc() {
        let npc = shooter(&[guided(130, 4)], Goal::Flee(ShipRef::Player));
        assert_eq!(fired(&npc, 0.0, 200.0, 50.0), id(130), "astern");
        let blind_aft = shooter(
            &[WeaponRecord {
                flags: 0x4000,
                ..guided(130, 4)
            }],
            Goal::Flee(ShipRef::Player),
        );
        assert_eq!(fired(&blind_aft, 0.0, 200.0, 50.0), None);
        let mut hull_blind = npc.clone();
        hull_blind.hull.blind_spots = 0x4000;
        assert_eq!(fired(&hull_blind, 0.0, 200.0, 50.0), None);
        let mut both_blind = blind_aft.clone();
        both_blind.hull.blind_spots = 0x4000;
        assert_eq!(
            fired(&both_blind, 0.0, 200.0, 50.0),
            None,
            "blind both ways"
        );
        let front = shooter(&[guided(130, 7)], Goal::Flee(ShipRef::Player));
        assert_eq!(
            fired(&front, 100.0, -100.0, 50.0),
            id(130),
            "45 off the nose"
        );
        assert_eq!(fired(&front, 100.1, -100.0, 50.0), None);
        let rear = shooter(&[guided(130, 8)], Goal::Flee(ShipRef::Player));
        assert_eq!(fired(&rear, 100.0, 100.0, 50.0), id(130), "45 off the tail");
        assert_eq!(fired(&rear, 100.1, 100.0, 50.0), None);
        assert_eq!(fired(&npc, 0.0, 232.1, 50.0), None, "out of range");
    }

    /// A gun of `id` reaching 50 pixels.
    fn short(id: i16) -> WeaponRecord {
        WeaponRecord {
            count: 5,
            ..gun(id, 1, 1)
        }
    }

    #[test]
    fn a_missile_fires_with_its_range_squared_at_least_0_95_of_the_distance_squared() {
        // 300 pixels: up to 307.79.
        let npc = shooter(&[short(128), missile(131)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -307.79, 50.0), id(131));
        assert_eq!(fired(&npc, 0.0, -307.8, 50.0), None);
        // Off to the side, both ways counting: 40000 + 52900 is in reach,
        // 40000 + 55225 not.
        let mut leaning = npc;
        leaning.state.heading = 41.0;
        assert_eq!(fired(&leaning, 200.0, -230.0, 50.0), id(131));
        assert_eq!(fired(&leaning, 200.0, -235.0, 50.0), None);
    }

    #[test]
    fn a_homing_weapon_is_a_gun_only_while_no_other_gun_has_its_ammunition() {
        // Dogfighting 100 above, the gun pick alone.
        let npc = shooter(&[missile(131), gun(128, 1, 1)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(128), "the gun");
        let npc = shooter(&[missile(131), short(128)], ATTACK);
        assert_eq!(
            fired(&npc, 0.0, -100.0, 50.0),
            None,
            "out of range, not of ammo"
        );
        let empty = WeaponRecord {
            ammo_type: 10,
            ..short(128)
        };
        let npc = shooter(&[missile(131), empty], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(131), "out of ammo");
        let only_homing = shooter(&[missile(131)], ATTACK);
        assert_eq!(
            fired(&only_homing, 0.0, -100.0, 50.0),
            id(131),
            "a dogfight's gun when no other gun can fire"
        );
    }

    #[test]
    fn a_later_pick_overrides_an_earlier_by_manoeuvre() {
        let all = [gun(128, 1, 1), guided(130, 4), missile(131)];
        // Approaching (200 above): gun, turret, then missile.
        let npc = shooter(&all, ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(131));
        let npc = shooter(&all[..2], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(130));
        // Dogfighting (100 above): turret, then gun.
        let npc = shooter(&all[..2], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(128));
        // Sniping: gun and missile, then turret.
        let snipe = Goal::Snipe(ShipRef::Player);
        assert_eq!(fired(&shooter(&all, snipe), 0.0, -100.0, 50.0), id(130));
        let npc = shooter(&[all[0], all[2]], snipe);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(131));
        // Fleeing: turrets alone.
        let flee = Goal::Flee(ShipRef::Player);
        assert_eq!(fired(&shooter(&all, flee), 0.0, -100.0, 50.0), id(130));
        let npc = shooter(&[all[0], all[2]], flee);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None);
    }

    #[test]
    fn nothing_fires_without_a_fight_or_a_target_there() {
        let all = [gun(128, 1, 1), guided(130, 4), missile(131)];
        for goal in [Goal::Idle, Goal::Inspect(ShipRef::Player)] {
            assert_eq!(
                fired(&shooter(&all, goal), 0.0, -100.0, 50.0),
                None,
                "{goal:?}"
            );
        }
        let npc = shooter(&all, Goal::Attack(ShipRef::Npc(NpcId(9))));
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None, "not there");
        let npcs = [npc.clone()];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(trigger(&shooter(&all, ATTACK), &around), Trigger::default());
    }

    #[test]
    fn a_weapon_reloading_or_unpaid_for_is_not_chosen() {
        let rocket = WeaponRecord {
            ammo_type: 10,
            ..gun(128, 1, 1)
        };
        let mut npc = shooter(&[rocket], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None, "no rounds");
        npc.rounds = BTreeMap::from([(WeaponId(138), 1)]);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(128));
        let mut slow = shooter(
            &[WeaponRecord {
                reload: 10,
                ..gun(128, 1, 1)
            }],
            ATTACK,
        );
        assert_eq!(fired(&slow, 0.0, -100.0, 50.0), id(128));
        let mut fuel = slow.reserves.fuel;
        let mut rounds = BTreeMap::new();
        slow.armament.fire(
            Trigger {
                only: Some(WeaponId(128)),
                ..Trigger::default()
            },
            Condition::Intact,
            crate::combat::armament::Shooter::PLAYER,
            &mut rounds,
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            &mut |_| {
                Some(crate::combat::aim::Aim {
                    heading: 0.0,
                    target: None,
                })
            },
        );
        assert_eq!(fired(&slow, 0.0, -100.0, 50.0), None, "reloading");
    }

    #[test]
    fn destroying_weapons_do_mass_damage_unflagged_with_their_ammunition() {
        assert!(destroys(&shooter(&[gun(128, 0, 1)], ATTACK)));
        assert!(
            !destroys(&shooter(&[gun(128, 10, 0)], ATTACK)),
            "no mass damage"
        );
        assert!(!destroys(&shooter(&[], ATTACK)));
        let flagged = WeaponRecord {
            flags2: NOT_DESTROYING,
            ..gun(128, 0, 1)
        };
        assert!(!destroys(&shooter(&[flagged], ATTACK)));
        for guidance in [-1, 0, 3, 4, 6, 7, 8] {
            assert!(
                destroys(&shooter(&[guided(128, guidance)], ATTACK)),
                "{guidance}"
            );
        }
        for guidance in [1, 5, 9, 10, 99] {
            assert!(
                !destroys(&shooter(&[guided(128, guidance)], ATTACK)),
                "{guidance}"
            );
        }
        let mut rocket = shooter(
            &[WeaponRecord {
                ammo_type: 10,
                ..gun(128, 0, 1)
            }],
            ATTACK,
        );
        assert!(!destroys(&rocket), "no rounds");
        rocket.rounds = BTreeMap::from([(WeaponId(138), 1)]);
        assert!(destroys(&rocket));
    }

    /// Whether `npc`'s trigger launches its fighters, among `npcs` and the
    /// player 100 above it.
    fn bays(npc: &Npc, npcs: &[Npc]) -> bool {
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0)),
            ..Surroundings::new(&[], npcs)
        };
        trigger(npc, &around).bays
    }

    #[test]
    fn a_ship_attacking_or_sniping_at_a_ship_there_launches_its_fighters() {
        let all = [gun(128, 1, 1)];
        let mut quarry = crate::testkit::npc(7, ShipStats::new(FAST, &[]));
        quarry.state.position = SHOOTER + Vec2::new(5000.0, 0.0);
        let at = ShipRef::Npc(NpcId(7));
        for goal in [ATTACK, Goal::Snipe(ShipRef::Player), Goal::Attack(at)] {
            assert!(bays(&shooter(&all, goal), &[quarry.clone()]), "{goal:?}");
        }
        assert!(
            bays(&shooter(&[], ATTACK), &[]),
            "whatever it carries, the armament picks"
        );
        let mut disabled = quarry.clone();
        disabled.condition = Condition::Disabled;
        assert!(bays(&shooter(&all, Goal::Attack(at)), &[disabled]));
    }

    #[test]
    fn no_fighters_launch_fleeing_idle_or_at_a_ship_gone_or_jumping_in() {
        let all = [gun(128, 1, 1)];
        for goal in [
            Goal::Flee(ShipRef::Player),
            Goal::Idle,
            Goal::Inspect(ShipRef::Player),
            Goal::Formation { guard: None },
            Goal::Dock(ShipRef::Player),
        ] {
            assert!(!bays(&shooter(&all, goal), &[]), "{goal:?}");
        }
        let at = ShipRef::Npc(NpcId(7));
        assert!(!bays(&shooter(&all, Goal::Attack(at)), &[]), "gone");
        let mut arriving = crate::testkit::npc(7, ShipStats::new(FAST, &[]));
        arriving.mode = crate::traffic::npc::Mode::JumpingIn { ticks_left: 3 };
        assert!(!bays(&shooter(&all, Goal::Attack(at)), &[arriving]));
        let mut wreck = crate::testkit::npc(7, ShipStats::new(FAST, &[]));
        wreck.condition = Condition::Dying { ticks_left: 3 };
        assert!(!bays(&shooter(&all, Goal::Attack(at)), &[wreck]));
    }

    /// A launcher of `id` firing rounds of weapon `id + 10`.
    fn launcher(id: i16) -> WeaponRecord {
        WeaponRecord {
            ammo_type: 10,
            ..weapon(id)
        }
    }

    /// A weapon of `id` spending `cost` fuel a shot.
    fn fuel_weapon(id: i16, cost: f32) -> WeaponRecord {
        #[allow(clippy::cast_possible_truncation)]
        let raw = -1000 - (cost * 10.0) as i16;
        WeaponRecord {
            ammo_type: raw,
            ..weapon(id)
        }
    }

    #[test]
    fn an_unarmed_ship_is_dry() {
        assert_eq!(ammo_left(&shooter(&[], ATTACK)), AmmoLeft::Dry);
    }

    #[test]
    fn a_gun_alone_is_armed() {
        assert_eq!(ammo_left(&shooter(&[weapon(128)], ATTACK)), AmmoLeft::Armed);
        let other = WeaponRecord {
            ammo_type: -2,
            ..weapon(128)
        };
        assert_eq!(ammo_left(&shooter(&[other], ATTACK)), AmmoLeft::Armed);
    }

    #[test]
    fn a_launcher_is_dry_without_rounds_and_armed_with_them() {
        let mut ship = shooter(&[launcher(128)], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::Dry);
        ship.rounds = BTreeMap::from([(WeaponId(138), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    #[test]
    fn a_gun_beside_an_empty_launcher_is_only_unlimited() {
        let ship = shooter(&[weapon(128), launcher(129)], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::OnlyUnlimited);
        let ship = shooter(&[launcher(129), weapon(128)], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::OnlyUnlimited);
    }

    #[test]
    fn one_launcher_with_rounds_arms_a_ship_whose_others_are_empty() {
        let other = WeaponRecord {
            ammo_type: 12,
            ..weapon(130)
        };
        let mut ship = shooter(&[weapon(127), launcher(128), other], ATTACK);
        ship.rounds = BTreeMap::from([(WeaponId(140), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    #[test]
    fn a_fuel_weapon_is_armed_while_fuel_covers_its_cost() {
        let mut ship = shooter(&[fuel_weapon(128, 5.0)], ATTACK);
        ship.reserves.fuel.now = 4.0;
        assert_eq!(ammo_left(&ship), AmmoLeft::Dry);
        ship.reserves.fuel.now = 5.0;
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    /// `record` with `flags2`.
    fn with_flags2(record: WeaponRecord, flags2: u16) -> WeaponRecord {
        WeaponRecord { flags2, ..record }
    }

    #[test]
    fn an_npc_never_picks_a_weapon_ai_ships_wont_use() {
        use crate::combat::weapon::AI_WONT_USE;
        let npc = shooter(&[with_flags2(gun(128, 1, 1), AI_WONT_USE)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None, "the gun");
        let npc = shooter(
            &[with_flags2(gun(128, 9, 9), AI_WONT_USE), gun(129, 1, 1)],
            ATTACK,
        );
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(129), "the lesser gun");
        let npc = shooter(
            &[
                short(128),
                with_flags2(missile(131), AI_WONT_USE),
                missile(132),
            ],
            ATTACK,
        );
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(132), "the missile");
        let turret = shooter(
            &[with_flags2(guided(130, 4), AI_WONT_USE)],
            Goal::Flee(ShipRef::Player),
        );
        assert_eq!(fired(&turret, 0.0, 200.0, 50.0), None, "the turret");
        assert!(!destroys(&shooter(
            &[with_flags2(gun(128, 0, 1), AI_WONT_USE)],
            ATTACK
        )));
    }

    #[test]
    fn a_cloaked_npc_picks_only_a_weapon_that_fires_cloaked() {
        use crate::combat::weapon::FIRES_CLOAKED;
        let cloak = |mut npc: Npc| {
            npc.cloak = crate::cloak::Cloak {
                fade: crate::cloak::FADE_FULL,
                direction: 0,
            };
            npc
        };
        let npc = cloak(shooter(
            &[gun(128, 9, 9), with_flags2(gun(129, 1, 1), FIRES_CLOAKED)],
            ATTACK,
        ));
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(129));
        let npc = cloak(shooter(&[short(128), missile(131)], ATTACK));
        assert_eq!(
            fired(&npc, 0.0, -300.0, 50.0),
            None,
            "nothing fires cloaked"
        );
        let npc = cloak(shooter(
            &[short(128), with_flags2(missile(131), FIRES_CLOAKED)],
            ATTACK,
        ));
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(131));
        assert!(!destroys(&cloak(shooter(&[gun(128, 0, 1)], ATTACK))));
    }

    #[test]
    fn a_fighter_bay_counts_its_fighters_as_rounds() {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 128,
            ..weapon(150)
        };
        let mut ship = shooter(&[bay], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::Dry);
        ship.rounds = BTreeMap::from([(WeaponId(150), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }
}
