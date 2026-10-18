//! Which weapon an NPC fires: the one each tick it proposes, as it
//! attacks, snipes at or flees from the ship its goal is about.
//!
//! The rules are the original's (`_AIFireGun` @0x7feb6, `_AIFireTurret`
//! @0x80ad3, `_AIFireMissile` @0x8115d, `_AILaunchFighter` @0x81372,
//! `_HandleShip` @0x34fcb-0x3501f and `_AIHasDestroyingWeapons` @0x7fc7c
//! in the `EV Nova` executable). An NPC holds a pick (the original's
//! +0x72, [`Npc::pick`]) from tick to tick. Each tick its AI runs the
//! passes its manoeuvre runs, which may set the pick and raise the fire
//! flag (+0xba); [`trigger`] gives the pick it fires, if the flag is up
//! ([`Trigger::only`]), and the pick it holds after the passes
//! ([`Trigger::keep`]). The fight then fires it as `_FireAIShipWeapon`
//! does ([`ai_fire`](crate::combat::ai_fire)). While it attacks or snipes
//! at a ship that is live in the system, it also launches its fighters
//! ([`Trigger::bays`], `_AILaunchFighter` from the attack state, see
//! [`bay`](crate::bay)).
//!
//! - **The passes**, each gate outside its call, so a pass whose gate
//!   fails doesn't run: approaching, the gun and then the turret pass
//!   while facing within [`GUN_FACING`] turns of the heading it wants,
//!   then the missile pass within [`MISSILE_FACING`]; dogfighting, the
//!   turret pass, then the gun pass within [`GUN_FACING`]; sniping, the
//!   gun and missile passes within [`GUN_FACING`], then the turret pass;
//!   fleeing, the turret pass alone (`_LowLevelAIHandler` substates 7, 6,
//!   0xe and 5).
//! - **The gun pass** (`_AIFireGun`), past its preamble (a target live
//!   in the system and seen), makes the gun it finds the pick; then any
//!   pick, found or kept, raises the flag (@0x80213-0x8021d), its
//!   ammunition and reload not tested again. With no pick and no armed
//!   gun it tries again with homing allowed (@0x80226-0x80243).
//! - **The turret and missile passes** make what they find the pick and
//!   raise the flag; with nothing found they leave both as they were.
//! - **A launching bay**: attacking or sniping, the bay `_AILaunchFighter`
//!   launches ([`Armament::bay_to_launch`]) becomes the pick, with no
//!   flag, before the passes (@0x8e6fa, @0x81411): a later find overrides
//!   it, and a gun pass finding nothing fires it, which then finds it
//!   reloading after its launch.
//! - **After firing** ([`after_firing`], `_HandleShip`): a pick fired of a
//!   weapon without `Flags` 0x0002 is cleared, flag and pick alike; a
//!   secondary's stays, so a gun pass that finds nothing fires it again.
//!   No pass clears the pick; a disabled NPC makes no decisions and fires
//!   nothing, so it keeps its pick until it recovers (`_LowLevelAIHandler`
//!   clears the flag every frame, @0x851f6, and its passes test that the
//!   ship is not disabled, @0x85240). A captured ship's pick is cleared
//!   (`_DoShipCapture` @0x41207).
//!
//! - **Its manoeuvre** ([`Manoeuvre`]): attacking, it approaches outside
//!   [`DOGFIGHT_BOX`] on either axis and dogfights within it; it snipes;
//!   or it flees.
//! - **The heading it wants** ([`wanted_heading`]): the lead angle for its
//!   first forward gun's speed, or the plain bearing with none.
//! - **A gun** (guidance -1, 0 and 6; a homing weapon only while no other
//!   gun of the target's planet-type has its ammunition, @0x7ffaa-0x8001c,
//!   @0x80243): none at a target the shooter does not see
//!   (`_ShipVisibleToShip` @0x7ff1e-0x7ff2c, [`cloak::visible`](crate::cloak::visible)); a
//!   planet-type weapon (`Flags2` 0x0400) exactly when the target's ship
//!   type is planet-type (`Flags` 0x0400, @0x7ffaa-0x7ffdf); reloaded and
//!   affordable; in range, the distance at most its range plus
//!   [`RANGE_MARGIN`] (a beam's range is its length); a rocket with a
//!   `ProxRadius` only while the target is [`PROX_CLEARANCE`] times it
//!   away on both axes. Of those, the most `EnergyDmg` while the target's
//!   shield is at or above none (@0x801e5-0x801fb), else the most
//!   `MassDmg`, a value of none or less counting as 1, a tie to the
//!   earlier mount. Guidance 5 is never chosen.
//! - **A turret** (3, 4, 7 and 8): none at a target whose ship type is
//!   untargetable (`shïp` `Flags2` 0x0004,
//!   [`UNTARGETABLE`](crate::cloak::UNTARGETABLE), `_AIFireTurret`
//!   @0x80b51), whatever the shooter's scanner, nor at one the shooter
//!   does not see (@0x80b5f-0x80b70); of the target's planet-type
//!   (@0x80be6-0x80c23), reloaded, affordable, in range, not in a blind
//!   spot, and 7 and 8 only with the target within [`FRONT_ARC`] of the
//!   nose and of the tail, measured as the rulebook's
//!   [`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)
//!   entry says ([`FireRules`]; by the engine, unfolded across the seam,
//!   @0x80c3d-0x80cf3). Chosen by the same shield rule (@0x80e12-0x80e27).
//! - **Point defence** ([`Trigger::point_defence`]): asked for whenever
//!   the manoeuvre reaches a turret, before any check of the target
//!   (`_AIFireTurret` calls `_HandleShipPointDefense` @0x80b0d):
//!   dogfighting, sniping, fleeing, and approaching while facing within
//!   [`GUN_FACING`].
//! - **A missile** (1, `_AIFireMissile` @0x8115d): none at a target whose
//!   ship type is untargetable unless the shooter's ship type carries a
//!   scanner with `ModVal` bit 0x0004 (@0x811a7-0x811cd,
//!   [`CloakGear::scanner`](crate::cloak::CloakGear::scanner)), nor at a
//!   cloaked target unless it carries one with bit 0x0008
//!   (`_ShipCanTargetCloakedShips` @0x811e7-0x811fd); the player counts
//!   its own ship type's bit ([`PlayerSide::untargetable`]) and never
//!   cloaks. None either while the damage the shots in flight aim at the
//!   target ([`Surroundings::targeted`],
//!   [`Combat::targeted_damage`](crate::combat::Combat::targeted_damage))
//!   is at least [`SUFFICIENT_DAMAGE`] times its shield and armour
//!   (`_SufficentTargetedDamage` @0x81203-0x8121c, @0x3fd22-0x3fd53); a
//!   gun may still fire. Then, of the homing weapons it carries, the one of lowest
//!   weapon ID (the walk by weapon index, @0x81267-0x8133a) that suits
//!   the target ([`suitable_missile`], @0x812ab), is a planet-type weapon
//!   (`Flags2` 0x0400) exactly when the target's ship type is planet-type
//!   (`Flags` 0x0400, @0x812b4-0x812fb), has its ammunition ([`has_ammo`],
//!   @0x81307) and whose range squared is at least [`MISSILE_RANGE`] of
//!   the distance squared (@0x81310-0x8132b), fired only if it has
//!   reloaded (@0x81349-0x8135d): one reloading does not give way to a
//!   later one.
//! - **Destroying weapons** ([`destroys`]): a weapon that does mass damage,
//!   is not `Flags2` 0x1000, is guided -1, 0, 3, 4, 6, 7 or 8, and has its
//!   ammunition. A ship with none leaves a disabled target alone.
//! - **Ammunition**: every weapon an NPC picks, and every destroying
//!   weapon it counts, has its ammunition as `_WeaponHasAmmo` says
//!   ([`has_ammo`]): an NPC never picks a weapon of `Flags2` 0x0100, a
//!   cloaked one only a weapon of `Flags2` 0x4000, and a weapon of
//!   `Flags2` 0x0080 needs its class's `KeyCarried` aboard. Its rounds are
//!   its launcher's own (see [`armament`](crate::combat::armament)).
//! - There is no friendly-fire check, as in the original. Guns
//!   (`_AIFireGun`) never read the untargetable bit.
//!
//! # Not modelled
//!
//! - The difficulty reload multiplier against the player.
//! - A full turret's (3 and 4) blind-spot angle, which `_AIFireTurret`
//!   reads from a local only an earlier 7 or 8 slot sets (@0x80c29-0x80c37,
//!   @0x80c5b), else uninitialised stack: here the bearing.
//! - The whole-degree truncation in the arc test (@0x80c7f, @0x80c95).
//! - Sniping while moving 0.35 or more on an axis, `_LowLevelAIHandler`
//!   brakes and runs only `_AILaunchFighter` (@0x870db-0x87137), and a
//!   new ship in a reused slot keeps the last one's pick, as no spawn
//!   writes +0x72: rdm phase weapons-fidelity/phase-17.
//! - `_LowLevelAIHandler`'s second `_AILaunchFighter` in the snipe, after
//!   its passes (@0x8732f), which writes the bay over the frame's pick
//!   only when the attack block's launch failed or under goal 0xd: the
//!   engine's sorties never fail and it has no goal 0xd.
//! - `_ShipTurnRate`'s own factors, which [`suitable_missile`] reads
//!   through the target's turn rate: rdm
//!   system-traffic-fidelity/phase-17-npc-skill-and-speed-factors.

#[cfg(doc)]
use crate::ai::PlayerSide;
use crate::ai::{Goal, Surroundings};
use crate::catalog::WeaponId;
use crate::combat::ShipRef;
use crate::combat::aim::{
    FRONT_ARC, angle_off, bearing, blind, lead, quadrant_off, quadrant_reference,
};
#[cfg(doc)]
use crate::combat::armament::Armament;
use crate::combat::armament::{GUN_RANGE_MARGIN, Mount, Shooter, Trigger};
use crate::combat::projectile::Target;
use crate::combat::weapon::{AVOIDS_FAST_SHIPS, Ammo, Guidance, WeaponSpec};
use crate::flight::ShipState;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// How near, in pixels on either axis, an attacker dogfights rather than
/// approaches (165.0 @0xdda24).
pub const DOGFIGHT_BOX: f32 = 165.0;
/// What a gun reaches beyond its range, in pixels (32.0 @0xdd654).
pub const RANGE_MARGIN: f32 = GUN_RANGE_MARGIN;
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
/// The whole degrees a tick a target must turn more than for a slow or
/// flagged missile to be unsuitable for it (the immediate 3 @0x3512 and
/// @0x3526).
pub const FAST_TARGET_TURN: f32 = 3.0;
/// The guided turn, in degrees a tick, at or below which a missile is
/// unsuitable for a fast target (2.0 @0xdd074).
pub const SLOW_MISSILE_TURN: f32 = 2.0;
/// How many times its shield and armour the damage aimed at a target
/// must be for it to suffice (1.05 @0xdd498, `_SufficentTargetedDamage`).
pub const SUFFICIENT_DAMAGE: f64 = 1.05;

/// The rulebook's entries an NPC's weapon pick follows, held by the
/// behaviours that fire ([`NovaAi::from_rulebook`](crate::ai::nova::NovaAi::from_rulebook)
/// reads them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireRules {
    /// How a quadrant turret measures its arc
    /// ([`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)).
    pub quadrant_arc: RuleSource,
}

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
    // The lead reads only where it is and how it moves.
    let target = Target {
        ship: ShipRef::Player,
        ship_type: crate::catalog::ShipId(0),
        fleet: ShipRef::Player,
        ties: crate::combat::sides::Ties::default(),
        position: at.position,
        velocity: at.velocity,
        radius: 0.0,
        sprite_x: 0.0,
        sheet: true,
        condition: crate::combat::hull::Condition::Intact,
        miss_class: crate::combat::hull::HEAVY_MISS_CLASS,
    };
    lead(&npc.state, &target, speed)
}

/// Whether a stock weapon of `npc`'s class reaches a ship at `at`: within
/// its class's stock reach ([`HullSpec::stock_reach`]), as `_AIInGunRange`
/// walks the class's stock weapons of guidance 8 or less with no weapon
/// named (@0x7fa2d-0x7faa8). Its person's weapons, its point defence and
/// its bays never count. The original truncates each axis to whole pixels
/// before squaring them (@0x7f9e8-0x7fa25), which is arithmetic width and
/// is not done here.
///
/// [`HullSpec::stock_reach`]: crate::combat::hull::HullSpec::stock_reach
#[must_use]
pub fn in_reach(npc: &Npc, at: &ShipState) -> bool {
    let off = at.position - npc.state.position;
    let squared = off.x * off.x + off.y * off.y;
    npc.hull
        .stock_reach
        .is_some_and(|reach| squared <= reach * reach)
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

/// Whether a missile of `spec` suits a target turning `target_turn_rate`
/// degrees a tick (`_SuitableMissileType` @0x34e1-0x353a): not when the
/// target turns more than [`FAST_TARGET_TURN`] whole degrees a tick and the
/// missile either won't fire at fast ships
/// ([`AVOIDS_FAST_SHIPS`](crate::combat::weapon::AVOIDS_FAST_SHIPS)) or
/// turns [`SLOW_MISSILE_TURN`] or less.
///
/// The original asks `_ShipTurnRate` of the target, whose factors (the
/// tractor's, the nag ship's, the floor and ionization) are rdm
/// system-traffic-fidelity/phase-17-npc-skill-and-speed-factors's; here
/// the target's [`Handling::turn_rate`](crate::handling::Handling) stands
/// in for it.
#[must_use]
pub fn suitable_missile(spec: &WeaponSpec, target_turn_rate: f32) -> bool {
    let fast = target_turn_rate.trunc() > FAST_TARGET_TURN;
    let slow = spec.flags & AVOIDS_FAST_SHIPS != 0 || spec.turn <= SLOW_MISSILE_TURN;
    !(fast && slow)
}

/// Whether `npc` holds a destroying weapon (see the module docs).
#[must_use]
pub fn destroys(npc: &Npc) -> bool {
    npc.armament
        .mounts()
        .iter()
        .any(|mount| destroying(&mount.spec) && has_ammo(npc, mount))
}

/// How many shots of fuel a fuel-burning weapon needs left not to be out:
/// `_AIShipIsOutOfAmmo` compares the fuel with |`AmmoType`| − 1000
/// (@0x7f77a-0x7f7a2), ten times the shot's cost of (|`AmmoType`| − 1000)
/// × 0.1.
pub const DRY_FUEL_SHOTS: f32 = 10.0;

/// How much of its weapons a ship can still fire (`_AIShipIsOutOfAmmo`
/// @0x7f71f, which returns 0, 1 or 2 for these in order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmmoLeft {
    /// Every weapon it holds can fire, or some ammunition- or fuel-using
    /// one still can.
    Armed,
    /// Every ammunition- or fuel-using weapon is out, but a weapon of
    /// unlimited ammunition remains (the original's `AmmoType`
    /// -1000..-1, @0x7f750-0x7f75f; here [`Ammo::Unlimited`],
    /// [`Ammo::Other`] below none and [`Ammo::Fuel`] of no cost).
    OnlyUnlimited,
    /// It holds no weapon, or every weapon it holds is out of rounds
    /// (a fighter bay's rounds are its fighters) or of fuel (less than
    /// [`DRY_FUEL_SHOTS`] of its shots left, @0x7f797).
    Dry,
}

/// How much of its weapons `npc` can still fire (see [`AmmoLeft`]), as
/// `_AIShipIsOutOfAmmo` (@0x7f71f-0x7f7ec) classes each weapon it holds by
/// its `AmmoType` alone, a fighter bay's too:
///
/// - -1000 to -1 is unlimited and skipped (@0x7f750-0x7f75f):
///   [`Ammo::Unlimited`], [`Ammo::Other`] below none, and
///   [`Ammo::Fuel`] of no cost;
/// - -1001 and below burns fuel, and is out while the fuel is below
///   [`DRY_FUEL_SHOTS`] of its shots (@0x7f77a-0x7f7a2), though it may
///   afford one;
/// - 0 and above, 256 and above too, is out while its own slot of rounds
///   is empty (@0x7f765-0x7f774): a launcher whose ammunition another
///   holds is still empty.
///
/// It holds no weapon, or every weapon is out: [`AmmoLeft::Dry`]; every
/// weapon that spends is out beside an unlimited one:
/// [`AmmoLeft::OnlyUnlimited`] (@0x7f7b6-0x7f7e0).
///
/// [`select_target`](crate::ai::target::select_target) reads
/// [`AmmoLeft::Dry`] to refuse a new target; the warship's and
/// interceptor's out-of-ammo retreat reads anything but
/// [`AmmoLeft::Armed`], and the warship's fully-dry drop
/// [`AmmoLeft::Dry`] (see [`out_of_ammo`](crate::ai::out_of_ammo)). The
/// escort's fallback that reads them (`_EscortAI` @0x83bb5) stays with
/// rdm:task/escorts-follow-up.
#[must_use]
pub fn ammo_left(npc: &Npc) -> AmmoLeft {
    let mut held = 0_usize;
    let mut spending = 0_usize;
    let mut out = 0_usize;
    for mount in npc.armament.mounts() {
        held += 1;
        let dry = match mount.spec.ammo {
            Ammo::Unlimited => continue,
            Ammo::Other(raw) if raw < 0 => continue,
            Ammo::Fuel(cost) if cost <= 0.0 => continue,
            Ammo::Fuel(cost) => npc.reserves.fuel.now < (cost * DRY_FUEL_SHOTS).round(),
            Ammo::Rounds(_) | Ammo::Other(_) => npc.rounds.slot(mount.spec.id) <= 0,
        };
        spending += 1;
        out += usize::from(dry);
    }
    if held == 0 || (out > 0 && out == held) {
        AmmoLeft::Dry
    } else if out > 0 && out == spending {
        AmmoLeft::OnlyUnlimited
    } else {
        AmmoLeft::Armed
    }
}

/// The weapon `npc` fires now among `around`, if any (see the module
/// docs), its fighter bays while it attacks or snipes at a ship that is
/// live there, and its point defence while its manoeuvre reaches a turret
/// (see the module docs).
///
/// A target with no state here (gone from the system) asks for no point
/// defence, where the original's handler may still reach
/// `_AIFireTurret` on the frame before its next think drops the target: a
/// one-tick lag the player cannot see.
#[must_use]
pub fn trigger(npc: &Npc, around: &Surroundings, rules: FireRules) -> Trigger {
    let bays = launches(npc, around);
    let (only, keep) = pick(npc, around, rules, bays);
    Trigger {
        only,
        keep,
        bays,
        point_defence: asks_turret(npc, around),
        ..Trigger::default()
    }
}

/// [`trigger`] for a ship whose attack launches no fighters: the player's
/// escort, whose launch the engine doesn't fly yet (rdm
/// escort-fleet-fidelity/phase-4-carriers-and-fighters), so its bay is
/// never its pick either.
#[must_use]
pub fn trigger_holding_fighters(npc: &Npc, around: &Surroundings, rules: FireRules) -> Trigger {
    let (only, keep) = pick(npc, around, rules, false);
    Trigger {
        only,
        keep,
        point_defence: asks_turret(npc, around),
        ..Trigger::default()
    }
}

/// Whether `npc` launches its fighters: attacking or sniping at a ship
/// live in the system (`_AILaunchFighter` from `_HighLevelAIHandler`'s
/// attack block, @0x8e6fa).
fn launches(npc: &Npc, around: &Surroundings) -> bool {
    npc.goal.attacking().is_some_and(|ship| around.live(ship))
}

/// What `_HandleShip` does with `npc`'s pick once the fight has fired it
/// (@0x34fcb-0x3501f): a pick fired ([`Trigger::only`]) of a weapon
/// without `Flags` 0x0002 is cleared, flag and pick alike, whether or not
/// anything spawned; a secondary's stays, so it fires again on the next
/// tick its AI sets no flag of its own (its gun pass keeps it). A pick it
/// doesn't carry is cleared too.
pub fn after_firing(npc: &mut Npc) {
    let Some(fired) = npc.trigger.only else {
        return;
    };
    let secondary = npc
        .armament
        .mounts()
        .iter()
        .any(|mount| mount.spec.id == fired && mount.spec.secondary());
    if !secondary {
        npc.trigger.only = None;
        npc.pick = None;
    }
}

/// Whether `npc`'s fight reaches `_AIFireTurret` this tick, which runs its
/// point defence before any check of the target (@0x80b0d): dogfighting
/// (`_LowLevelAIHandler` substate 6, @0x85f17), sniping (0xe,
/// @0x8730e-0x87324), fleeing (5, @0x85d5c-0x85d72), and approaching only
/// while facing within [`GUN_FACING`] turns of the heading it wants (7,
/// @0x868d3-0x868f6).
fn asks_turret(npc: &Npc, around: &Surroundings) -> bool {
    let Some(at) = npc.goal.quarry().and_then(|ship| around.state_of(ship)) else {
        return false;
    };
    match manoeuvre(npc.goal, &npc.state, &at) {
        None => false,
        Some(Manoeuvre::Approach) => facing(npc, &at, GUN_FACING),
        Some(_) => true,
    }
}

/// Whether `npc` faces within `turns` of its turn rate of the heading it
/// wants to fire at a ship at `at` ([`wanted_heading`]).
fn facing(npc: &Npc, at: &ShipState, turns: f32) -> bool {
    angle_off(npc.state.heading, wanted_heading(npc, at)) <= turns * npc.stats.handling.turn_rate
}

/// The weapon [`trigger`] fires, if any, and the pick `npc` holds after
/// its passes, by `rules`, its bay written as its pick first when it
/// `launches` (see the module docs).
fn pick(
    npc: &Npc,
    around: &Surroundings,
    rules: FireRules,
    launches: bool,
) -> (Option<WeaponId>, Option<WeaponId>) {
    let mut passes = Passes {
        pick: npc.pick,
        flag: false,
    };
    // `_HighLevelAIHandler`'s `_AILaunchFighter` comes before the passes
    // (@0x8e6fa): the bay it launches is the pick, with no flag
    // (@0x81411).
    if launches && let Some(bay) = npc.armament.bay_to_launch(&npc.rounds) {
        passes.pick = Some(bay);
    }
    if let Some(sight) = sight(npc, around, rules) {
        passes.run(&sight);
    }
    (passes.pick.filter(|_| passes.flag), passes.pick)
}

/// The pick (+0x72) and the fire flag (+0xba) as the passes leave them.
struct Passes {
    pick: Option<WeaponId>,
    flag: bool,
}

impl Passes {
    /// The passes `sight`'s manoeuvre runs, in the original's order, each
    /// gate outside its call (see the module docs).
    fn run(&mut self, sight: &Sight) {
        let facing = |turns: f32| facing(sight.npc, &sight.at, turns);
        match sight.manoeuvre {
            Manoeuvre::Approach => {
                if facing(GUN_FACING) {
                    self.gun(sight);
                    self.turret(sight);
                }
                if facing(MISSILE_FACING) {
                    self.missile(sight);
                }
            }
            Manoeuvre::Dogfight => {
                self.turret(sight);
                if facing(GUN_FACING) {
                    self.gun(sight);
                }
            }
            Manoeuvre::Snipe => {
                if facing(GUN_FACING) {
                    self.gun(sight);
                    self.missile(sight);
                }
                self.turret(sight);
            }
            Manoeuvre::Flee => self.turret(sight),
        }
    }

    /// `_AIFireGun`: past its preamble, a find is the pick; then any pick,
    /// found or kept, raises the flag (@0x8020f-0x8021d); with none and
    /// no armed gun, it tries again with homing allowed (@0x80226-0x80243).
    fn gun(&mut self, sight: &Sight) {
        if !sight.seen {
            return;
        }
        if let Some(found) = sight.gun(false) {
            self.pick = Some(found);
        }
        if self.pick.is_some() {
            self.flag = true;
        } else if !sight.armed()
            && let Some(found) = sight.gun(true)
        {
            self.pick = Some(found);
            self.flag = true;
        }
    }

    /// `_AIFireTurret`: a find is the pick and raises the flag; none leaves
    /// both (@0x80e33-0x80e40).
    fn turret(&mut self, sight: &Sight) {
        if let Some(found) = sight.turret() {
            self.pick = Some(found);
            self.flag = true;
        }
    }

    /// `_AIFireMissile`: a find that has reloaded is the pick and raises the
    /// flag; otherwise nothing (@0x81349-0x81363).
    fn missile(&mut self, sight: &Sight) {
        if let Some(found) = sight.missile() {
            self.pick = Some(found);
            self.flag = true;
        }
    }
}

/// How `npc` sizes up the ship its goal is about, by `rules`; none for a
/// goal that is no fight, or a ship not live or gone from the system,
/// where every pass stops at its preamble.
fn sight<'a>(npc: &'a Npc, around: &Surroundings, rules: FireRules) -> Option<Sight<'a>> {
    let ship = npc.goal.quarry()?;
    if !around.live(ship) {
        return None;
    }
    let at = around.state_of(ship)?;
    let reserves = match ship {
        ShipRef::Player => around.player.map(|player| player.reserves),
        ShipRef::Npc(id) => around.npc(id).map(|other| other.reserves),
    };
    // At or above none, as `_AIFireTurret` (@0x80e12-0x80e27) and
    // `_AIFireGun` (@0x801e5-0x801fb, 0.0 at 0xdd054) test it: a shield of
    // exactly none still picks by energy.
    let shields_up = reserves.is_some_and(|reserves| reserves.shield.now >= 0.0);
    let aimed_enough = reserves.is_some_and(|reserves| {
        let standing = reserves.shield.now + reserves.armor.now;
        f64::from(around.aimed_at(ship)) >= SUFFICIENT_DAMAGE * f64::from(standing)
    });
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
    // The player never cloaks, so is always seen. An NPC is seen as
    // `_ShipVisibleToShip(target, shooter)` says (@0x971c, called
    // @0x7ff1e-0x7ff2c and @0x80b5f-0x80b70): the shooter escorts it
    // (shooter +0x9a is the target's +0x86) and has a cloak, @0x9788-0x97a2.
    let seen = match ship {
        ShipRef::Player => true,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| {
            crate::cloak::visible(
                other.cloak,
                other.state.position,
                other.emerging(),
                crate::cloak::Observer {
                    at: npc.state.position,
                    scanner: npc.cloak_gear.scanner.on_screen,
                    cloaked_escort: npc.leader == Some(other.id) && npc.cloak_gear.device,
                },
            )
        }),
    };
    let (turn_rate, planet_type) = match ship {
        ShipRef::Player => around
            .player
            .map(|player| (player.handling.turn_rate, player.hull.planet_type)),
        ShipRef::Npc(id) => around
            .npc(id)
            .map(|other| (other.stats.handling.turn_rate, other.hull.planet_type)),
    }
    .unwrap_or_default();
    let manoeuvre = manoeuvre(npc.goal, &npc.state, &at)?;
    Some(Sight {
        npc,
        at,
        manoeuvre,
        shields_up,
        untargetable,
        cloaked,
        seen,
        aimed_enough,
        turn_rate,
        planet_type,
        quadrant_arc: rules.quadrant_arc,
    })
}

/// An NPC sizing up its target at `at`.
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
struct Sight<'a> {
    npc: &'a Npc,
    at: ShipState,
    /// How it flies its fight.
    manoeuvre: Manoeuvre,
    shields_up: bool,
    /// Whether the target's ship type is untargetable.
    untargetable: bool,
    /// Whether the target is cloaked.
    cloaked: bool,
    /// Whether the shooter sees the target (`_ShipVisibleToShip`
    /// @0x971c): a gun or turret holds fire at one it does not see.
    seen: bool,
    /// Whether the damage aimed at the target suffices
    /// (`_SufficentTargetedDamage` @0x3fd22-0x3fd53).
    aimed_enough: bool,
    /// The target's turn rate, in degrees a tick.
    turn_rate: f32,
    /// Whether the target's ship type is planet-type.
    planet_type: bool,
    /// How a quadrant turret measures its arc
    /// ([`RuleKey::QuadrantTurretArc`](crate::RuleKey::QuadrantTurretArc)).
    quadrant_arc: RuleSource,
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

    /// Whether a gun suits the target's planet-type: a planet-type weapon
    /// exactly when its ship type is.
    fn suits(&self, spec: &WeaponSpec) -> bool {
        spec.planet_type() == self.planet_type
    }

    /// Whether the ship holds an armed gun: a forward gun of the target's
    /// planet-type with its ammunition, reloaded or not (@0x8000e-0x8001c).
    fn armed(&self) -> bool {
        self.npc.armament.mounts().iter().any(|mount| {
            forward(&mount.spec) && self.suits(&mount.spec) && has_ammo(self.npc, mount)
        })
    }

    /// The gun it fires, if any (see the module docs): a forward one, or
    /// with `homing` a homing one too, of the target's planet-type, ready,
    /// in reach and clear of its fuse.
    fn gun(&self, homing: bool) -> Option<WeaponId> {
        let usable = |mount: &Mount| {
            self.ready(mount) && self.reaches(&mount.spec) && self.clear_of(&mount.spec)
        };
        let gun =
            |spec: &WeaponSpec| forward(spec) || (homing && spec.guidance == Guidance::Homing);
        self.best(|mount| gun(&mount.spec) && self.suits(&mount.spec) && usable(mount))
    }

    /// The turret it fires, if any (see the module docs).
    fn turret(&self) -> Option<WeaponId> {
        if self.untargetable || !self.seen {
            return None;
        }
        let heading = self.npc.state.heading;
        let bearing = bearing(self.npc.state.position, self.at.position);
        // Blind spots take the folded angle, as `_TurretBlindSpot` folds
        // (@0xb35a-0xb387).
        let bearing_off = angle_off(heading, bearing);
        self.best(|mount| {
            let spec = &mount.spec;
            let in_arc = match spec.guidance {
                Guidance::TurretBeam | Guidance::Turret => true,
                Guidance::FrontTurret | Guidance::RearTurret => {
                    let reference = quadrant_reference(spec.guidance, heading);
                    quadrant_off(reference, bearing, self.quadrant_arc) <= FRONT_ARC
                }
                _ => false,
            };
            in_arc
                && spec.planet_type() == self.planet_type
                && !blind(spec.flags | self.npc.hull.blind_spots, bearing_off)
                && self.ready(mount)
                && self.reaches(spec)
        })
    }

    /// The missile it fires, if any (see the module docs).
    fn missile(&self) -> Option<WeaponId> {
        let scanner = self.npc.cloak_gear.scanner;
        if (self.untargetable && !scanner.untargetable)
            || (self.cloaked && !scanner.targets)
            || self.aimed_enough
        {
            return None;
        }
        let distance = self.distance_squared();
        self.npc
            .armament
            .mounts()
            .iter()
            .filter(|mount| {
                let spec = &mount.spec;
                let range = spec.range();
                spec.guidance == Guidance::Homing
                    && suitable_missile(spec, self.turn_rate)
                    && spec.planet_type() == self.planet_type
                    && has_ammo(self.npc, mount)
                    && range * range >= MISSILE_RANGE * distance
            })
            .min_by_key(|mount| mount.spec.id)
            .filter(|mount| mount.reload <= 0.0)
            .map(|mount| mount.spec.id)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use crate::combat::armament::LauncherRounds;
    use std::collections::BTreeMap;

    use super::*;
    use crate::ai::PlayerSide;
    use crate::catalog::{WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::{Condition, HullSpec};
    use crate::combat::weapon::AVOIDS_FAST_SHIPS;
    use crate::geometry::Vec2;
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::traffic::npc::NpcId;
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::weapon;

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
            ..crate::traffic::fixture::npc(1, ShipStats::new(FAST, &[]))
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
            blessing: crate::ai::Blessing::NONE,
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
        trigger(npc, &around, FireRules::default()).only
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
        trigger(npc, &around, FireRules::default()).only
    }

    /// What `npc`, attacking NPC 2, fires at it as `target` makes it,
    /// (`x`, `y`) from [`SHOOTER`].
    fn fired_at_npc(npc: &Npc, x: f32, y: f32, target: impl Fn(&mut Npc)) -> Option<WeaponId> {
        trigger_at_npc(npc, x, y, target).only
    }

    /// The trigger `npc`, attacking NPC 2, holds as `target` makes it,
    /// (`x`, `y`) from [`SHOOTER`].
    fn trigger_at_npc(npc: &Npc, x: f32, y: f32, target: impl Fn(&mut Npc)) -> Trigger {
        let mut npc = npc.clone();
        npc.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
        let mut other = crate::traffic::fixture::npc(2, ShipStats::new(FAST, &[]));
        other.state.position = SHOOTER + Vec2::new(x, y);
        target(&mut other);
        let npcs = [npc.clone(), other];
        trigger(&npc, &Surroundings::new(&[], &npcs), FireRules::default())
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

    const SCANS_SCREEN: crate::cloak::Scanner = crate::cloak::Scanner {
        on_screen: true,
        ..crate::cloak::Scanner::NONE
    };

    /// A target fully cloaked.
    fn cloaked(other: &mut Npc) {
        other.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
    }

    /// Whether `npc`, carrying weapon 130 reaching 300 pixels, fires it at
    /// a cloaked NPC only while it sees it (`_ShipVisibleToShip`
    /// @0x971c): with a scanner on screen within [`SIGHT_RANGE`] on both
    /// axes, or as its cloaked escort.
    ///
    /// [`SIGHT_RANGE`]: crate::cloak::SIGHT_RANGE
    fn holds_fire_while_unseen(npc: &Npc) {
        assert_eq!(fired_at_npc(npc, 0.0, -150.0, cloaked), None, "no scanner");
        let scans = scanning(npc.clone(), SCANS_SCREEN);
        assert_eq!(fired_at_npc(&scans, 0.0, -150.0, cloaked), id(130));
        assert_eq!(
            fired_at_npc(&scans, 0.0, -200.0, cloaked),
            id(130),
            "200 on the axis"
        );
        assert_eq!(fired_at_npc(&scans, 0.0, -250.0, cloaked), None, "250 off");
        assert_eq!(fired_at_npc(npc, 0.0, -250.0, |_| {}), id(130), "uncloaked");
        let mut escort = npc.clone();
        escort.leader = Some(NpcId(2));
        assert_eq!(
            fired_at_npc(&escort, 0.0, -250.0, cloaked),
            None,
            "escorting it without a cloak"
        );
        escort.cloak_gear.device = true;
        assert_eq!(
            fired_at_npc(&escort, 0.0, -250.0, cloaked),
            id(130),
            "its cloaked escort"
        );
        let mut cloaked_stranger = npc.clone();
        cloaked_stranger.cloak_gear.device = true;
        assert_eq!(
            fired_at_npc(&cloaked_stranger, 0.0, -250.0, cloaked),
            None,
            "a cloak without escorting it"
        );
    }

    #[test]
    fn a_turret_holds_fire_at_a_cloaked_ship_it_does_not_see_and_fires_once_it_sees_it() {
        let turret = WeaponRecord {
            count: 30,
            ..guided(130, 4)
        };
        holds_fire_while_unseen(&shooter(&[turret], Goal::Flee(ShipRef::Player)));
    }

    #[test]
    fn a_gun_holds_fire_at_a_cloaked_ship_it_does_not_see_and_fires_once_it_sees_it() {
        let gun = WeaponRecord {
            count: 30,
            ..gun(130, 1, 1)
        };
        holds_fire_while_unseen(&shooter(&[gun], ATTACK));
    }

    /// Whether `npc` asks for its point defence with the player (`x`,
    /// `y`) from [`SHOOTER`], its shields up, as `player` makes it.
    fn asks_at_player(npc: &Npc, x: f32, y: f32, player: impl Fn(&mut PlayerSide)) -> bool {
        let npcs = [npc.clone()];
        let mut side = player_at(SHOOTER.x + x, SHOOTER.y + y, 50.0);
        player(&mut side);
        let around = Surroundings {
            player: Some(side),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default()).point_defence
    }

    #[test]
    fn point_defence_is_asked_for_whenever_the_manoeuvre_reaches_a_turret() {
        let plain = |_: &mut PlayerSide| {};
        // Unarmed: point defence is asked for whatever it carries.
        for goal in [
            ATTACK,
            Goal::Snipe(ShipRef::Player),
            Goal::Flee(ShipRef::Player),
        ] {
            assert!(
                asks_at_player(&shooter(&[], goal), 0.0, -100.0, plain),
                "{goal:?}"
            );
        }
        // Approaching (300 above), only while facing within three turns
        // (3 degrees a tick) of the heading it wants (@0x868d3-0x868de).
        let mut approaching = shooter(&[], ATTACK);
        approaching.state.heading = 9.0;
        assert!(asks_at_player(&approaching, 0.0, -300.0, plain));
        approaching.state.heading = 9.1;
        assert!(!asks_at_player(&approaching, 0.0, -300.0, plain));
        approaching.state.heading = 350.9;
        assert!(!asks_at_player(&approaching, 0.0, -300.0, plain));
        assert!(
            asks_at_player(&approaching, 0.0, -100.0, plain),
            "dogfighting, any heading"
        );
        for goal in [
            Goal::Idle,
            Goal::Inspect(ShipRef::Player),
            Goal::Formation { guard: None },
        ] {
            assert!(
                !asks_at_player(&shooter(&[], goal), 0.0, -100.0, plain),
                "{goal:?}"
            );
        }
        let gone = shooter(&[], Goal::Attack(ShipRef::Npc(NpcId(9))));
        assert!(
            !asks_at_player(&gone, 0.0, -100.0, plain),
            "no target state"
        );
        // Before the target checks (@0x80b0d): untargetable, unseen or not
        // live.
        let npc = shooter(&[guided(130, 4)], ATTACK);
        let untargetable = |player: &mut PlayerSide| player.untargetable = true;
        assert!(asks_at_player(&npc, 0.0, -100.0, untargetable));
        assert!(trigger_at_npc(&npc, 0.0, -100.0, cloaked).point_defence);
        let dying = |other: &mut Npc| {
            other.condition = Condition::Dying {
                ticks_left: 3,
                starting: false,
            }
        };
        assert!(trigger_at_npc(&npc, 0.0, -100.0, dying).point_defence);
        let arriving = |other: &mut Npc| {
            other.mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        };
        assert!(trigger_at_npc(&npc, 0.0, -100.0, arriving).point_defence);
        assert_eq!(trigger_at_npc(&npc, 0.0, -100.0, arriving).only, None);
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
            &mut LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
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

    /// An NPC at [`SHOOTER`] facing up, carrying `id` of `records` as an
    /// arsenal reads them, so its range runs down its sub-munition chain.
    fn arsenal_shooter(records: &[WeaponRecord], id: i16) -> Npc {
        let arsenal = crate::combat::armament::Arsenal::new(records, Vec::new());
        let mut npc = shooter(&[], ATTACK);
        npc.armament = Armament::new([(*arsenal.weapon(WeaponId(id)).expect("read"), 1)]);
        npc
    }

    #[test]
    fn a_turret_picks_a_sub_munition_weapon_at_its_chain_reach() {
        let nanites = WeaponRecord {
            sub_count: 1,
            sub_type: Some(WeaponId(229)),
            ..guided(163, 4)
        };
        let swarm = WeaponRecord {
            count: 100,
            ..gun(229, 5, 5)
        };
        let npc = arsenal_shooter(&[nanites, swarm], 163);
        assert_eq!(
            npc.armament.mounts()[0].spec.range(),
            1200.0,
            "200 and 1000"
        );
        assert_eq!(
            fired(&npc, 0.0, -1000.0, 50.0),
            Some(WeaponId(163)),
            "beyond its own 232 pixels"
        );
        assert_eq!(fired(&npc, 0.0, -1232.0, 50.0), Some(WeaponId(163)));
        assert_eq!(fired(&npc, 0.0, -1233.0, 50.0), None, "beyond the chain");
    }

    #[test]
    fn a_missile_fires_at_its_chain_reach() {
        let torpedo = WeaponRecord {
            count: 26,
            sub_count: 5,
            sub_type: Some(WeaponId(148)),
            ..missile(182)
        };
        let homer = missile(148);
        let mut npc = arsenal_shooter(&[torpedo, homer], 182);
        assert_eq!(npc.armament.mounts()[0].spec.range(), 560.0, "260 and 300");
        // A gun of its own, so the missile is not fired as a gun.
        npc.armament = Armament::new(
            [
                npc.armament.mounts()[0].spec,
                WeaponSpec::new(&gun(128, 5, 5)),
            ]
            .map(|spec| (spec, 1)),
        );
        // 560² >= 0.95 d² up to d = 574.5.
        assert_eq!(fired(&npc, 0.0, -574.0, 50.0), Some(WeaponId(182)));
        assert_eq!(fired(&npc, 0.0, -575.0, 50.0), None);
    }

    #[test]
    fn in_reach_counts_only_the_classs_stock_weapons_of_guidance_8_or_less() {
        // The class's stock turret reaching 300 pixels, and 332 with the
        // margin.
        let mut npc = crate::traffic::fixture::npc(1, crate::stats::ShipStats::default());
        npc.state.position = crate::geometry::Vec2::new(100.0, 50.0);
        let at = |x: f32, y: f32| ShipState {
            position: crate::geometry::Vec2::new(x, y),
            ..ShipState::default()
        };
        assert!(!in_reach(&npc, &at(101.0, 50.0)), "no stock reach");
        npc.armament = crate::combat::armament::Armament::new([(
            WeaponSpec::new(&crate::catalog::WeaponRecord {
                guidance: 4,
                speed: 1000,
                count: 300,
                ..nova_sim_catalog::fixture::weapon(140)
            }),
            1,
        )]);
        assert!(
            !in_reach(&npc, &at(101.0, 50.0)),
            "a long turret among its outfits or a person's counts for nothing"
        );
        npc.hull.stock_reach = Some(332.0);
        // 300² + 100² = 100000 is within 332² = 110224; 320² + 100² is not.
        assert!(in_reach(&npc, &at(400.0, 150.0)));
        assert!(in_reach(&npc, &at(-200.0, -50.0)));
        assert!(!in_reach(&npc, &at(420.0, 150.0)));
        assert!(!in_reach(&npc, &at(100.0, 383.0)));
        assert!(in_reach(&npc, &at(100.0, 382.0)), "332 exactly");
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
        // Below none: a shield of exactly none still counts as up.
        assert_eq!(fired(&npc, 0.0, -200.0, -1.0), id(129), "shields down");
    }

    #[test]
    fn a_gun_and_a_turret_pick_by_energy_at_a_shield_of_none_and_by_mass_below_it() {
        // `ucomiss` against 0, `jb` to the mass best (@0x80e12-0x80e27,
        // @0x801e5-0x801fb).
        let guns = shooter(&[gun(128, 10, 1), gun(129, 2, 8)], ATTACK);
        assert_eq!(fired(&guns, 0.0, -200.0, 0.0), id(128));
        assert_eq!(fired(&guns, 0.0, -200.0, -0.1), id(129));
        let turret = |id: i16, energy: i16, mass: i16| WeaponRecord {
            guidance: 4,
            ..gun(id, energy, mass)
        };
        let turrets = shooter(
            &[turret(130, 10, 1), turret(131, 2, 8)],
            Goal::Flee(ShipRef::Player),
        );
        assert_eq!(fired(&turrets, 0.0, 200.0, 0.0), id(130));
        assert_eq!(fired(&turrets, 0.0, 200.0, -0.1), id(131));
    }

    #[test]
    fn a_damage_of_none_or_less_counts_as_1_and_a_tie_goes_to_the_earlier() {
        let npc = shooter(&[gun(128, -5, 0), gun(129, 0, 0), gun(130, 1, 1)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(128));
        let npc = shooter(&[gun(128, 1, 1), gun(129, 2, 0)], ATTACK);
        // Below none, so by mass: a shield of exactly none counts as up.
        assert_eq!(
            fired(&npc, 0.0, -200.0, -1.0),
            id(128),
            "mass 0 counts as 1"
        );
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

    /// What `npc` fires at the player 150 away bearing `bearing` from
    /// [`SHOOTER`], its shields up, by `rules`.
    fn fired_bearing(npc: &Npc, bearing: f32, rules: FireRules) -> Option<WeaponId> {
        let (sin, cos) = bearing.to_radians().sin_cos();
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player_at(
                SHOOTER.x + 150.0 * sin,
                SHOOTER.y - 150.0 * cos,
                50.0,
            )),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, rules).only
    }

    #[test]
    fn a_quadrant_turret_across_the_seam_holds_fire_by_the_engine_and_fires_by_the_bible() {
        use crate::rulebook::RuleSource;
        let engine = FireRules {
            quadrant_arc: RuleSource::Engine,
        };
        let bible = FireRules {
            quadrant_arc: RuleSource::Bible,
        };
        assert_eq!(FireRules::default(), engine);
        let turret = |guidance: i16, heading: f32| {
            let mut npc = shooter(&[guided(130, guidance)], Goal::Flee(ShipRef::Player));
            npc.state.heading = heading;
            npc
        };
        // |10 - 350| = 340 unfolded, 20 folded.
        let front = turret(7, 350.0);
        assert_eq!(fired_bearing(&front, 10.0, engine), None);
        assert_eq!(fired_bearing(&front, 10.0, bible), id(130));
        // |350 - 10| = 340 unfolded, 20 folded.
        let front_east = turret(7, 10.0);
        assert_eq!(fired_bearing(&front_east, 350.0, engine), None);
        assert_eq!(fired_bearing(&front_east, 350.0, bible), id(130));
        // The rear's reference: (170 + 180) mod 360 = 350.
        let rear = turret(8, 170.0);
        assert_eq!(fired_bearing(&rear, 10.0, engine), None);
        assert_eq!(fired_bearing(&rear, 10.0, bible), id(130));
        assert_eq!(fired_bearing(&rear, 340.0, engine), id(130), "10 off");
        // (270 + 180) mod 360 = 90: the reference wraps past 360.
        let rear_west = turret(8, 270.0);
        assert_eq!(fired_bearing(&rear_west, 100.0, engine), id(130));
        assert_eq!(fired_bearing(&rear_west, 100.0, bible), id(130));
        // Off the seam, both fire, up to 45 either side.
        let east = turret(7, 90.0);
        for rules in [engine, bible] {
            assert_eq!(fired_bearing(&east, 100.0, rules), id(130));
            assert_eq!(fired_bearing(&east, 134.9, rules), id(130));
            assert_eq!(fired_bearing(&east, 45.1, rules), id(130));
            assert_eq!(fired_bearing(&east, 135.1, rules), None);
            assert_eq!(fired_bearing(&east, 44.9, rules), None);
        }
        // A full turret has no arc.
        let full = turret(4, 350.0);
        assert_eq!(fired_bearing(&full, 170.0, engine), id(130));
    }

    /// A gun of `id` reaching 50 pixels.
    fn short(id: i16) -> WeaponRecord {
        WeaponRecord {
            count: 5,
            ..gun(id, 1, 1)
        }
    }

    // The kept pick.

    /// The trigger `npc` holds at the player (`x`, `y`) from [`SHOOTER`],
    /// its shields up.
    fn trigger_at_player(npc: &Npc, x: f32, y: f32) -> Trigger {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x + x, SHOOTER.y + y, 50.0)),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default())
    }

    /// `npc` holding `pick` as its pick.
    fn holding(mut npc: Npc, pick: i16) -> Npc {
        npc.pick = id(pick);
        npc
    }

    /// A secondary homing missile of `id` reaching 300 pixels.
    fn secondary_missile(id: i16) -> WeaponRecord {
        WeaponRecord {
            flags: crate::combat::weapon::SECONDARY,
            ..missile(id)
        }
    }

    #[test]
    fn an_npc_refires_a_kept_secondary_on_a_tick_its_gun_pass_picks_nothing() {
        // Dogfighting 100 above, its gun reaching 82: no gun, and a
        // dogfight runs no missile pass.
        let npc = shooter(&[short(128), secondary_missile(131)], ATTACK);
        let fresh = trigger_at_player(&npc, 0.0, -100.0);
        assert_eq!((fresh.only, fresh.keep), (None, None));
        let kept = trigger_at_player(&holding(npc, 131), 0.0, -100.0);
        assert_eq!((kept.only, kept.keep), (id(131), id(131)));
        // Its own gun, in reach, takes over.
        let near = shooter(&[gun(128, 1, 1), secondary_missile(131)], ATTACK);
        let found = trigger_at_player(&holding(near, 131), 0.0, -100.0);
        assert_eq!((found.only, found.keep), (id(128), id(128)));
    }

    #[test]
    fn the_turret_pass_leaves_an_earlier_pick_and_sets_no_flag_when_it_finds_none() {
        let fleeing = holding(shooter(&[guided(130, 4)], Goal::Flee(ShipRef::Player)), 131);
        let none = trigger_at_player(&fleeing, 0.0, 300.0);
        assert_eq!((none.only, none.keep), (None, id(131)), "out of its reach");
        let found = trigger_at_player(&fleeing, 0.0, 100.0);
        assert_eq!((found.only, found.keep), (id(130), id(130)));
    }

    #[test]
    fn a_gun_pass_stopped_at_its_preamble_fires_no_kept_pick() {
        let npc = holding(shooter(&[short(128), missile(131)], ATTACK), 131);
        let unseen = trigger_at_npc(&npc, 0.0, -100.0, cloaked);
        assert_eq!((unseen.only, unseen.keep), (None, id(131)));
        let seen = trigger_at_npc(&npc, 0.0, -100.0, |_| {});
        assert_eq!(seen.only, id(131));
        let gone = trigger_at_npc(&npc, 0.0, -100.0, |other| {
            other.condition = Condition::Dying {
                ticks_left: 3,
                starting: false,
            };
        });
        assert_eq!((gone.only, gone.keep), (None, id(131)), "not live");
    }

    #[test]
    fn a_kept_pick_fires_in_place_of_the_homing_gun_retry() {
        // No forward gun: the gun pass retries with homing allowed, unless
        // a pick is kept.
        let npc = shooter(&[missile(131), missile(132)], ATTACK);
        assert_eq!(
            trigger_at_player(&npc, 0.0, -100.0).only,
            id(131),
            "the retry"
        );
        let kept = trigger_at_player(&holding(npc, 132), 0.0, -100.0);
        assert_eq!((kept.only, kept.keep), (id(132), id(132)));
    }

    /// A fighter bay of `id`, launching `shïp` 200 every 60 ticks.
    fn bay(id: i16) -> WeaponRecord {
        WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            reload: 60,
            flags: crate::combat::weapon::SECONDARY,
            ..weapon(id)
        }
    }

    #[test]
    fn a_launching_bay_is_the_pick_before_the_passes_and_a_later_find_overrides_it() {
        for goal in [ATTACK, Goal::Snipe(ShipRef::Player)] {
            let carrier = |gun: WeaponRecord| {
                let mut npc = shooter(&[gun, bay(150)], goal);
                npc.rounds = LauncherRounds::from([(WeaponId(150), 2)]);
                npc
            };
            // Its gun out of reach: the gun pass fires the bay it kept.
            let out = trigger_at_player(&carrier(short(128)), 0.0, -100.0);
            assert_eq!(
                (out.only, out.keep, out.bays),
                (id(150), id(150), true),
                "{goal:?}"
            );
            let found = trigger_at_player(&carrier(gun(128, 1, 1)), 0.0, -100.0);
            assert_eq!((found.only, found.keep), (id(128), id(128)), "{goal:?}");
            // A bay reloading is no pick.
            let mut reloading = carrier(short(128));
            reloading.armament.hold(WeaponId(150));
            let held = trigger_at_player(&reloading, 0.0, -100.0);
            assert_eq!((held.only, held.keep), (None, None), "{goal:?}");
            // Nor is an empty one.
            let mut empty = carrier(short(128));
            empty.rounds = LauncherRounds::default();
            assert_eq!(trigger_at_player(&empty, 0.0, -100.0).keep, None);
        }
    }

    #[test]
    fn after_firing_a_pick_without_0x0002_is_cleared_and_a_secondary_is_kept() {
        let fired = |npc: Npc, pick: i16| {
            let mut npc = holding(npc, pick);
            npc.trigger.only = id(pick);
            after_firing(&mut npc);
            (npc.trigger.only, npc.pick)
        };
        let npc = shooter(&[gun(128, 1, 1), secondary_missile(131)], ATTACK);
        assert_eq!(fired(npc.clone(), 128), (None, None));
        assert_eq!(fired(npc.clone(), 131), (id(131), id(131)), "Flags 0x0002");
        // No flag: nothing fired, and the pick stays.
        let mut idle = holding(npc, 128);
        after_firing(&mut idle);
        assert_eq!((idle.trigger.only, idle.pick), (None, id(128)));
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
        assert_eq!(
            trigger(&shooter(&all, ATTACK), &around, FireRules::default()),
            Trigger::default()
        );
    }

    #[test]
    fn a_weapon_reloading_or_unpaid_for_is_not_chosen() {
        let rocket = WeaponRecord {
            ammo_type: 10,
            ..gun(128, 1, 1)
        };
        let mut npc = shooter(&[rocket], ATTACK);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None, "no rounds");
        npc.rounds = LauncherRounds::from([(WeaponId(138), 1)]);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), None, "its ammunition's");
        npc.rounds = LauncherRounds::from([(WeaponId(128), 1)]);
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(128), "its own");
        let mut slow = shooter(
            &[WeaponRecord {
                reload: 10,
                ..gun(128, 1, 1)
            }],
            ATTACK,
        );
        assert_eq!(fired(&slow, 0.0, -100.0, 50.0), id(128));
        let mut fuel = slow.reserves.fuel;
        let mut rounds = LauncherRounds::default();
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
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
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
        rocket.rounds = LauncherRounds::from([(WeaponId(128), 1)]);
        assert!(destroys(&rocket));
    }

    /// Whether `npc`'s trigger launches its fighters, among `npcs` and the
    /// player 100 above it.
    fn bays(npc: &Npc, npcs: &[Npc]) -> bool {
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0)),
            ..Surroundings::new(&[], npcs)
        };
        trigger(npc, &around, FireRules::default()).bays
    }

    #[test]
    fn a_ship_attacking_or_sniping_at_a_ship_there_launches_its_fighters() {
        let all = [gun(128, 1, 1)];
        let mut quarry = crate::traffic::fixture::npc(7, ShipStats::new(FAST, &[]));
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
        let mut arriving = crate::traffic::fixture::npc(7, ShipStats::new(FAST, &[]));
        arriving.mode = crate::traffic::npc::Mode::JumpingIn { tick: 0 };
        assert!(!bays(&shooter(&all, Goal::Attack(at)), &[arriving]));
        let mut wreck = crate::traffic::fixture::npc(7, ShipStats::new(FAST, &[]));
        wreck.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
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
        ship.rounds = LauncherRounds::from([(WeaponId(128), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    #[test]
    fn ammo_left_reads_each_launchers_own_rounds() {
        let storm = WeaponRecord {
            guidance: 9,
            ammo_type: 27,
            ..weapon(161)
        };
        let mut ship = shooter(&[storm], ATTACK);
        ship.rounds = LauncherRounds::from([(WeaponId(155), 40), (WeaponId(161), 0)]);
        assert_eq!(
            ammo_left(&ship),
            AmmoLeft::Dry,
            "155's rounds are not 161's (`_AIShipIsOutOfAmmo` @0x7f76a)"
        );
        ship.rounds = LauncherRounds::from([(WeaponId(161), 1)]);
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
        ship.rounds = LauncherRounds::from([(WeaponId(130), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    #[test]
    fn a_fuel_weapon_is_dry_below_ten_shots_of_fuel() {
        // -1010: a unit a shot, dry below 10 (@0x7f77a-0x7f7a2).
        let mut ship = shooter(&[fuel_weapon(128, 1.0)], ATTACK);
        ship.reserves.fuel.now = 10.0;
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
        ship.reserves.fuel.now = 9.9;
        assert_eq!(ammo_left(&ship), AmmoLeft::Dry, "though it affords a shot");
        let mut beside = shooter(&[fuel_weapon(128, 1.0), weapon(127)], ATTACK);
        beside.reserves.fuel.now = 9.9;
        assert_eq!(ammo_left(&beside), AmmoLeft::OnlyUnlimited);
        // -1007: 0.7 a shot, armed at exactly 7.
        let mut fine = shooter(&[fuel_weapon(128, 0.7)], ATTACK);
        fine.reserves.fuel.now = 7.0;
        assert_eq!(ammo_left(&fine), AmmoLeft::Armed);
        fine.reserves.fuel.now = 6.99;
        assert_eq!(ammo_left(&fine), AmmoLeft::Dry);
        assert_eq!(DRY_FUEL_SHOTS, 10.0);
    }

    #[test]
    fn an_ammo_type_from_minus_1000_to_minus_1_is_unlimited() {
        for raw in [-1, -2, -500, -999, -1000] {
            let gun = WeaponRecord {
                ammo_type: raw,
                ..weapon(127)
            };
            let mut alone = shooter(std::slice::from_ref(&gun), ATTACK);
            alone.reserves.fuel.now = 0.0;
            assert_eq!(ammo_left(&alone), AmmoLeft::Armed, "{raw}");
            let beside = shooter(&[gun, launcher(129)], ATTACK);
            assert_eq!(
                ammo_left(&beside),
                AmmoLeft::OnlyUnlimited,
                "{raw}: unlimited, not an ammunition user (@0x7f750-0x7f75f)"
            );
        }
    }

    #[test]
    fn an_ammo_type_past_255_reads_its_own_slot() {
        let odd = WeaponRecord {
            ammo_type: 300,
            ..weapon(140)
        };
        let mut ship = shooter(&[odd], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::Dry, "`testw; js` @0x7f765");
        ship.rounds = LauncherRounds::from([(WeaponId(140), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    #[test]
    fn a_fighter_bay_of_an_unlimited_ammo_type_is_unlimited() {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: -5,
            ..weapon(150)
        };
        let ship = shooter(&[bay, launcher(129)], ATTACK);
        assert_eq!(ammo_left(&ship), AmmoLeft::OnlyUnlimited);
    }

    #[test]
    fn a_dry_launcher_beside_a_live_one_and_a_gun_is_armed() {
        let mut ship = shooter(&[launcher(128), launcher(129), weapon(127)], ATTACK);
        ship.rounds = LauncherRounds::from([(WeaponId(129), 3)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }

    /// `record` with `flags2`.
    fn with_flags2(record: WeaponRecord, flags2: u16) -> WeaponRecord {
        WeaponRecord { flags2, ..record }
    }

    /// A homing missile of `id` like [`missile`], turning `guided_turn`
    /// tenths of a degree a tick, with `flags`.
    fn turning_missile(id: i16, guided_turn: i16, flags: u16) -> WeaponRecord {
        WeaponRecord {
            flags,
            guided_turn,
            ..missile(id)
        }
    }

    /// What `npc` fires at the player (`x`, `y`) from [`SHOOTER`], its
    /// shields up, as `player` makes it.
    fn fired_at_player(
        npc: &Npc,
        x: f32,
        y: f32,
        player: impl Fn(&mut PlayerSide),
    ) -> Option<WeaponId> {
        let npcs = [npc.clone()];
        let mut side = player_at(SHOOTER.x + x, SHOOTER.y + y, 50.0);
        player(&mut side);
        let around = Surroundings {
            player: Some(side),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default()).only
    }

    #[test]
    fn a_missile_is_unsuitable_for_a_target_turning_over_3_if_it_turns_2_or_less_or_is_flagged() {
        let spec = |guided_turn, flags| WeaponSpec::new(&turning_missile(131, guided_turn, flags));
        let table = [
            // (GuidedTurn, Flags, the target's turn, suitable)
            (20, 0, 4.0, false),
            (20, 0, 3.99, true),
            (20, 0, 3.0, true),
            (21, 0, 4.0, true),
            (21, 0, 40.0, true),
            (0, 0, 4.0, false),
            (70, AVOIDS_FAST_SHIPS, 4.0, false),
            (70, AVOIDS_FAST_SHIPS, 3.99, true),
            (70, 0, 4.0, true),
            (70, !AVOIDS_FAST_SHIPS, 4.0, true),
        ];
        for (guided_turn, flags, turn, suitable) in table {
            assert_eq!(
                suitable_missile(&spec(guided_turn, flags), turn),
                suitable,
                "{guided_turn} {flags:#x} at {turn}"
            );
        }
        assert_eq!((FAST_TARGET_TURN, SLOW_MISSILE_TURN), (3.0, 2.0));
        assert_eq!(AVOIDS_FAST_SHIPS, 0x0008);
    }

    #[test]
    fn a_slow_or_flagged_missile_is_not_fired_at_an_agile_target() {
        let turning = |rate: f32| move |player: &mut PlayerSide| player.handling.turn_rate = rate;
        let emp = shooter(&[short(128), turning_missile(131, 20, 0)], ATTACK);
        assert_eq!(fired_at_player(&emp, 0.0, -300.0, turning(4.0)), None);
        assert_eq!(fired_at_player(&emp, 0.0, -300.0, turning(3.0)), id(131));
        let flagged = shooter(
            &[short(128), turning_missile(131, 70, AVOIDS_FAST_SHIPS)],
            ATTACK,
        );
        assert_eq!(fired_at_player(&flagged, 0.0, -300.0, turning(4.0)), None);
        assert_eq!(
            fired_at_player(&flagged, 0.0, -300.0, turning(3.0)),
            id(131)
        );
        let both = shooter(
            &[
                short(128),
                turning_missile(131, 20, 0),
                turning_missile(132, 70, 0),
            ],
            ATTACK,
        );
        assert_eq!(
            fired_at_player(&both, 0.0, -300.0, turning(4.0)),
            id(132),
            "the suitable one"
        );
        let agile = |other: &mut Npc| other.stats.handling.turn_rate = 4.0;
        assert_eq!(fired_at_npc(&emp, 0.0, -300.0, agile), None, "an NPC");
        assert_eq!(fired_at_npc(&emp, 0.0, -300.0, |_| {}), id(131));
    }

    #[test]
    fn a_planet_type_missile_flies_only_at_a_planet_type_ship_and_another_only_at_another() {
        use crate::combat::weapon::PLANET_TYPE;
        let planet = |player: &mut PlayerSide| player.hull.planet_type = true;
        let ordinary = shooter(&[short(128), missile(131)], ATTACK);
        assert_eq!(fired_at_player(&ordinary, 0.0, -300.0, |_| {}), id(131));
        assert_eq!(fired_at_player(&ordinary, 0.0, -300.0, planet), None);
        let planetary = shooter(
            &[short(128), with_flags2(missile(131), PLANET_TYPE)],
            ATTACK,
        );
        assert_eq!(fired_at_player(&planetary, 0.0, -300.0, planet), id(131));
        assert_eq!(fired_at_player(&planetary, 0.0, -300.0, |_| {}), None);
        let planet_npc = |other: &mut Npc| other.hull.planet_type = true;
        assert_eq!(fired_at_npc(&planetary, 0.0, -300.0, planet_npc), id(131));
        assert_eq!(fired_at_npc(&ordinary, 0.0, -300.0, planet_npc), None);
    }

    #[test]
    fn a_turret_fires_a_planet_type_weapon_only_at_a_planet_type_ship() {
        use crate::combat::weapon::PLANET_TYPE;
        let planet = |player: &mut PlayerSide| player.hull.planet_type = true;
        let planet_npc = |other: &mut Npc| other.hull.planet_type = true;
        let flee = Goal::Flee(ShipRef::Player);
        let ordinary = shooter(&[guided(130, 4)], flee);
        assert_eq!(fired_at_player(&ordinary, 0.0, 200.0, |_| {}), id(130));
        assert_eq!(fired_at_player(&ordinary, 0.0, 200.0, planet), None);
        assert_eq!(fired_at_npc(&ordinary, 0.0, 100.0, planet_npc), None);
        let planetary = shooter(&[with_flags2(guided(130, 4), PLANET_TYPE)], flee);
        assert_eq!(fired_at_player(&planetary, 0.0, 200.0, planet), id(130));
        assert_eq!(fired_at_player(&planetary, 0.0, 200.0, |_| {}), None);
        assert_eq!(fired_at_npc(&planetary, 0.0, 100.0, planet_npc), id(130));
        assert_eq!(fired_at_npc(&planetary, 0.0, 100.0, |_| {}), None);
    }

    #[test]
    fn a_gun_fires_a_planet_type_weapon_only_at_a_planet_type_ship() {
        use crate::combat::weapon::PLANET_TYPE;
        let planet = |player: &mut PlayerSide| player.hull.planet_type = true;
        let planet_npc = |other: &mut Npc| other.hull.planet_type = true;
        let ordinary = shooter(&[gun(128, 1, 1)], ATTACK);
        assert_eq!(fired_at_player(&ordinary, 0.0, -100.0, |_| {}), id(128));
        assert_eq!(fired_at_player(&ordinary, 0.0, -100.0, planet), None);
        assert_eq!(fired_at_npc(&ordinary, 0.0, -100.0, planet_npc), None);
        let planetary = shooter(&[with_flags2(gun(128, 1, 1), PLANET_TYPE)], ATTACK);
        assert_eq!(fired_at_player(&planetary, 0.0, -100.0, planet), id(128));
        assert_eq!(fired_at_player(&planetary, 0.0, -100.0, |_| {}), None);
        assert_eq!(fired_at_npc(&planetary, 0.0, -100.0, planet_npc), id(128));
        assert_eq!(fired_at_npc(&planetary, 0.0, -100.0, |_| {}), None);
    }

    #[test]
    fn a_homing_gun_is_chosen_when_the_only_armed_gun_is_of_the_other_planet_type() {
        // Dogfighting 100 above, the gun pick alone (@0x7ffaa-0x8001c).
        use crate::combat::weapon::PLANET_TYPE;
        let npc = shooter(
            &[missile(131), with_flags2(gun(128, 1, 1), PLANET_TYPE)],
            ATTACK,
        );
        assert_eq!(fired(&npc, 0.0, -100.0, 50.0), id(131));
        let planet = |player: &mut PlayerSide| player.hull.planet_type = true;
        assert_eq!(
            fired_at_player(&npc, 0.0, -100.0, planet),
            id(128),
            "armed against a planet-type ship"
        );
    }

    /// What `npc` fires at the player 300 above it, its shields at 50 of
    /// 100 and its armour full at 100, with `aimed` damage already aimed
    /// at it and 9 at NPC 2.
    fn fired_with_aimed(npc: &Npc, aimed: i32) -> Option<WeaponId> {
        let npcs = [npc.clone()];
        let targeted = BTreeMap::from([(ShipRef::Player, aimed), (ShipRef::Npc(NpcId(2)), 9)]);
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x, SHOOTER.y - 300.0, 50.0)),
            targeted: &targeted,
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default()).only
    }

    #[test]
    fn no_missile_flies_while_the_damage_aimed_at_the_target_suffices() {
        // 1.05 x (50 + 100) = 157.5.
        let npc = shooter(&[short(128), missile(131)], ATTACK);
        assert_eq!(fired_with_aimed(&npc, 158), None);
        assert_eq!(fired_with_aimed(&npc, 157), id(131));
        assert_eq!(fired_with_aimed(&npc, 0), id(131));
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(131), "none aimed");
        let other = |target: &mut Npc| {
            target.reserves = Reserves::full(4.0, 4.0, 100.0);
        };
        // 1.05 x 8 = 8.4: the 9 aimed at NPC 2 suffices.
        let npcs = {
            let mut shooter = npc.clone();
            shooter.goal = Goal::Attack(ShipRef::Npc(NpcId(2)));
            let mut target = crate::traffic::fixture::npc(2, ShipStats::new(FAST, &[]));
            target.state.position = SHOOTER + Vec2::new(0.0, -300.0);
            other(&mut target);
            [shooter, target]
        };
        let targeted = BTreeMap::from([(ShipRef::Npc(NpcId(2)), 9)]);
        let around = Surroundings {
            targeted: &targeted,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(trigger(&npcs[0], &around, FireRules::default()).only, None);
        let targeted = BTreeMap::from([(ShipRef::Npc(NpcId(2)), 8)]);
        let around = Surroundings {
            targeted: &targeted,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            trigger(&npcs[0], &around, FireRules::default()).only,
            id(131)
        );
        assert_eq!(SUFFICIENT_DAMAGE, 1.05);
    }

    #[test]
    fn sufficient_aimed_damage_holds_the_missile_alone() {
        let npc = shooter(&[gun(128, 1, 1)], ATTACK);
        assert_eq!(fired_with_aimed(&npc, 1000), None, "out of the gun's reach");
        let close = shooter(&[gun(128, 1, 1)], Goal::Snipe(ShipRef::Player));
        let npcs = [close.clone()];
        let targeted = BTreeMap::from([(ShipRef::Player, 1000)]);
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0)),
            targeted: &targeted,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            trigger(&close, &around, FireRules::default()).only,
            id(128),
            "a gun still fires"
        );
    }

    #[test]
    fn the_missile_of_lowest_weapon_id_is_taken_and_its_reload_does_not_give_way() {
        let npc = shooter(&[short(128), missile(132), missile(131)], ATTACK);
        assert_eq!(fired(&npc, 0.0, -300.0, 50.0), id(131));
        let slow = WeaponRecord {
            reload: 10,
            ..missile(131)
        };
        let mut reloading = shooter(&[short(128), missile(132), slow], ATTACK);
        let mut fuel = reloading.reserves.fuel;
        reloading.armament.fire(
            Trigger {
                only: Some(WeaponId(131)),
                ..Trigger::default()
            },
            Condition::Intact,
            crate::combat::armament::Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
        );
        assert_eq!(fired(&reloading, 0.0, -300.0, 50.0), None);
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
        ship.rounds = LauncherRounds::from([(WeaponId(150), 1)]);
        assert_eq!(ammo_left(&ship), AmmoLeft::Armed);
    }
}
