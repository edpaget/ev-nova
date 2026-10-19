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
//! - **The passes**, each body of its frame's in turn
//!   ([`pursuit::Frame`]), each gate outside its call, so a pass whose
//!   gate fails doesn't run, and each facing test the original's
//!   ([`faces`]: `_AngularDifference` of the whole-degree heading and the
//!   heading the body wants, under the turns, strictly): approaching, the
//!   gun and then the turret pass while facing within [`GUN_FACING`]
//!   turns, then the missile pass within [`MISSILE_FACING`]; dogfighting,
//!   the turret pass, then the gun pass within [`GUN_FACING`]; sniping at
//!   rest, the gun and missile passes while it faces the plain bearing of
//!   its target within [`GUN_FACING`] turns, strictly
//!   ([`snipe::facing`]), then the turret pass; sniping while moving 0.35
//!   or more on an axis ([`snipe::moving`]), none at all, and no point
//!   defence: it brakes and only launches its fighters (@0x87329, see
//!   [`snipe`]); fleeing close, the turret pass alone; breaking off, the
//!   turret pass alone inside [`pursuit::FIRE_BOX`] on both axes; rushing,
//!   inside it the turret pass then the gun pass within [`GUN_FACING`],
//!   outside it the missile pass within [`GUN_FACING`]
//!   (`_LowLevelAIHandler` substates 7, 6, 0xe, 5, 0x10 and 0x11). A ship
//!   running out from its attacker runs none.
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
//!   reloading after its launch. A plunderer (goal 0xd) launches only as
//!   it snipes, after the passes (@0x87329-0x8732f): the bay overrides
//!   their find, and fires if a pass raised the flag. A goal-4 sniper
//!   launches there a second time, which does the same when its first
//!   launch found no ship slot ([`Surroundings::fighter_room`]) and so left
//!   the bay ready (@0x81421-0x81423). The high level's launch is
//!   [`Trigger::bays`], the snipe's [`Trigger::bays_after`].
//! - **The wait gate** ([`gated`], `_HandleShip` @0x34a54-0x34a72): while
//!   its AI wait runs ([`Npc::wait_running`]) the fight fires nothing of
//!   its trigger but its fighter bays and its point defence, and its pick
//!   is neither fired nor cleared. A waiting escort's turret find still
//!   becomes its pick ([`escort::fire_unprovoked`](crate::ai::escort::fire_unprovoked)).
//! - **After firing** ([`after_firing`], `_HandleShip`): a pick fired of a
//!   weapon without `Flags` 0x0002 is cleared, flag and pick alike; a
//!   secondary's stays, so a gun pass that finds nothing fires it again.
//!   No pass clears the pick; a disabled NPC makes no decisions and fires
//!   nothing, so it keeps its pick until it recovers (`_LowLevelAIHandler`
//!   clears the flag every frame, @0x851f6, and its passes test that the
//!   ship is not disabled, @0x85240). A captured ship's pick is cleared
//!   (`_DoShipCapture` @0x41207).
//!
//! - **Its manoeuvres** ([`Manoeuvre`], [`pursuit`]): attacking, it
//!   approaches outside [`DOGFIGHT_BOX`] on either axis and dogfights
//!   within it, breaking off a joust and rushing on its afterburner as
//!   the pursuit says; it snipes; or it flees, close within the pursuit's
//!   reach and running out beyond. A stand-off ship (`shïp` `Flags2`
//!   0x0002) attacking or sniping keeps its band instead
//!   ([`stand_off::keep`](crate::ai::stand_off::keep), `_HighLevelAIHandler` @0x8e48e-0x8e5ee and
//!   @0x8e6b7-0x8e6c4): it approaches beyond it, snipes within it, and
//!   backs away (fleeing close) inside the box, its band halved against a
//!   disabled target.
//! - **The heading it wants**, worked out once as its frame starts from
//!   the pick it holds before its passes ([`lead_pick`]), so the passes'
//!   facing tests and the flight agree: dogfighting
//!   ([`dogfight_lead`]), the whole-degree lead (`_CalcLeadAngle`) for a
//!   pick of guidance -1 or 6, the table bearing for any other, and with
//!   no pick the lead for its last gun or the bearing; approaching
//!   ([`approach_lead`]), with a last gun the lead for its pick (or, by
//!   [`RuleKey::ApproachLead`](crate::RuleKey::ApproachLead)'s other
//!   reading, its last gun), else the bearing; rushing, the bearing;
//!   breaking off, its break heading; fleeing close, the bearing from its
//!   attacker to itself.
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
//! - A goal-4 sniper's second launch from a bay of `Reload` 0, which the
//!   high level's success leaves ready, so the snipe's call launches
//!   another fighter (@0x87329-0x8732f): no stock bay has one, and the
//!   engine's fight launches one sortie per bay a tick
//!   (rdm:phase/weapons-fidelity/phase-41-reload-zero-bay-second-snipe-launch).
//! - The escape pod's (class 0x2ff) exemption from the wait gate
//!   (@0x34a69-0x34a72): a pod NPC
//!   ([`pod_flight`](crate::traffic::pod_flight)) counts its wait in its
//!   mode, which [`Npc::wait_running`] doesn't read, and the gate's own
//!   test of its class is
//!   rdm:phase/ship-handling-fidelity/phase-20-npc-pod-sound-priority-and-fire-gate.
//!   A pirate's count wait (goal 22) is exempt ([`gated`]).
//! - `_ShipTurnRate`'s own factors but its floor, which
//!   [`suitable_missile`] reads through the target's turn rate: the
//!   tractor's hold (rdm:phase/ship-handling-fidelity/phase-12-tractor-hold),
//!   ionization (rdm:phase/ship-handling-fidelity/phase-11-ionization-handling)
//!   and an NPC's class rate without its outfits
//!   (rdm:phase/ship-handling-fidelity/phase-15-npc-skill-speed-and-reserves).

#[cfg(doc)]
use crate::ai::PlayerSide;
use crate::ai::{Surroundings, pursuit};
use crate::catalog::WeaponId;
use crate::combat::ShipRef;
use crate::combat::aim::{FRONT_ARC, blind_spot, quadrant_off};
#[cfg(doc)]
use crate::combat::armament::Armament;
use crate::combat::armament::{Arsenal, GUN_RANGE_MARGIN, Mount, Shooter, Trigger};
use crate::combat::hull::Condition;
use crate::combat::weapon::{AVOIDS_FAST_SHIPS, Ammo, Guidance, WeaponSpec};
use crate::flight::ShipState;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;
use crate::traffic::snipe;
use crate::trig;

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

/// How an NPC flies its fight: `_LowLevelAIHandler`'s substates (see
/// [`pursuit`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Manoeuvre {
    /// Closing on its target from afar (7).
    Approach,
    /// Turning with its target, close in (6).
    Dogfight,
    /// Standing off and firing (0xe).
    Snipe,
    /// Running from its attacker, close (5).
    Flee,
    /// Breaking off a joust along its break heading (0x10).
    BreakOff,
    /// Rushing its target on its afterburner (0x11).
    Rush,
}

/// The manoeuvre `npc`'s high level starts its frame with at a ship at
/// `at`, disabled as `at_disabled` says ([`pursuit::high_level`]): none
/// for a goal that is no fight, or for a ship running out from its
/// attacker beyond [`pursuit::FLEE_REACH`].
#[must_use]
pub fn manoeuvre(npc: &Npc, at: &ShipState, at_disabled: bool) -> Option<Manoeuvre> {
    match pursuit::high_level(npc, at, at_disabled)? {
        pursuit::Start::Sub(manoeuvre) => Some(manoeuvre),
        pursuit::Start::BreakOff { .. } => Some(Manoeuvre::BreakOff),
        pursuit::Start::RunningOut => None,
    }
}

/// Whether `spec` fires forward: guidance -1, 0 or 6.
fn forward(spec: &WeaponSpec) -> bool {
    matches!(
        spec.guidance,
        Guidance::Unguided | Guidance::Beam | Guidance::Rocket
    )
}

/// The spec of `weapon` among `npc`'s own mounts, if it carries it.
fn carried(npc: &Npc, weapon: WeaponId) -> Option<&WeaponSpec> {
    npc.armament
        .mounts()
        .iter()
        .find(|mount| mount.spec.id == weapon)
        .map(|mount| &mount.spec)
}

/// The whole-degree heading a dogfighter wants at a ship at `at`, holding
/// `pick` as its pick (+0x72) before its passes (@0x85e3c-0x85f0d): the
/// lead for a pick of guidance -1 or 6, the table bearing for any other,
/// and with no pick the lead for its last gun
/// ([`Armament::last_gun`], +0xc8da), or the bearing with none. A weapon
/// it does not carry leads with nothing here (see the module docs).
#[must_use]
pub fn dogfight_lead(npc: &Npc, at: &ShipState, pick: Option<WeaponId>) -> i16 {
    let weapon = match pick {
        Some(pick) => carried(npc, pick)
            .filter(|spec| matches!(spec.guidance, Guidance::Unguided | Guidance::Rocket)),
        None => npc.armament.last_gun().and_then(|gun| carried(npc, gun)),
    };
    lead(npc, at, weapon)
}

/// The whole-degree heading an approaching attacker wants at a ship at
/// `at`, holding `pick` as its pick before its passes, as `reading` of
/// [`RuleKey::ApproachLead`](crate::RuleKey::ApproachLead) says: with a
/// last gun ([`Armament::last_gun`]), the lead for its pick by the engine
/// (`_CalcLeadAngle` gets +0x72, @0x86822-0x86858, the bearing with no
/// pick, as `_CalcLeadAngle` returns at once for -1, @0x2b05b), or for
/// that last gun by the other reading; with none, the table bearing
/// (@0x8685f-0x86883).
#[must_use]
pub fn approach_lead(
    npc: &Npc,
    at: &ShipState,
    pick: Option<WeaponId>,
    reading: RuleSource,
) -> i16 {
    let Some(last) = npc.armament.last_gun() else {
        return lead(npc, at, None);
    };
    let weapon = match reading {
        RuleSource::Engine => pick,
        RuleSource::Bible => Some(last),
    };
    lead(npc, at, weapon.and_then(|weapon| carried(npc, weapon)))
}

/// The pick `npc` holds as its low level starts among `around`: the bay
/// `_AILaunchFighter` launches, when it attacks or snipes at a ship live
/// there (the high level's launch comes first, @0x8e6fa, @0x81411), else
/// the pick it holds.
#[must_use]
pub fn lead_pick(npc: &Npc, around: &Surroundings) -> Option<WeaponId> {
    if launches(npc, around)
        && let Some(bay) = npc.armament.bay_to_launch(&npc.rounds)
    {
        return Some(bay);
    }
    npc.pick
}

/// How far, in whole degrees, a ship heading `heading` is off `wanted`:
/// `_AngularDifference(trunc(heading), wanted)`, as every facing test of
/// `_LowLevelAIHandler`'s substates 5, 6, 7, 0x10 and 0x11 reads it.
#[must_use]
pub fn off_by(heading: f32, wanted: i16) -> f32 {
    f32::from(trig::angular_difference(trig::whole(heading), wanted))
}

/// Whether `npc` faces `wanted` within `turns` of its turn rate,
/// strictly, as the substates' passes test it (`ucomiss`, `jbe` past the
/// call).
#[must_use]
pub fn faces(npc: &Npc, wanted: i16, turns: f32) -> bool {
    off_by(npc.state.heading, wanted) < turns * npc.stats.handling.turn_rate
}

/// The whole-degree lead from `npc`'s centre to a ship at `at` for
/// `weapon` ([`lead_with`]).
pub(crate) fn lead(npc: &Npc, at: &ShipState, weapon: Option<&WeaponSpec>) -> i16 {
    trig::lead_angle(
        npc.state.position,
        npc.state.velocity,
        (at.position, at.velocity),
        weapon.and_then(WeaponSpec::lead_shot),
    )
}

/// The whole-degree lead angle from `npc`'s centre to a ship at `at` for
/// a shot of `weapon`, as `_LowLevelAIHandler` sets the heading it wants
/// with `_CalcLeadAngle(self, target, weap, &self.pos)` (@0x85e99,
/// @0x85ed9, @0x86858, @0x871a4; [`trig::lead_angle`]): the table bearing
/// with no weapon, or one outside the lead's guidance gate
/// ([`WeaponSpec::lead_shot`]), so the steering, the facing tests and the
/// shots share one lead.
#[must_use]
pub fn lead_with(npc: &Npc, at: &ShipState, weapon: Option<&WeaponSpec>) -> f32 {
    f32::from(lead(npc, at, weapon))
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
/// The original asks `_ShipTurnRate` of the target; here the target's
/// [`Handling::turn_rate`](crate::handling::Handling) stands in for it,
/// floored as `_ShipTurnRate` floors it
/// ([`floored_turn`](crate::handling::floored_turn)). Its other factors
/// are left out: the tractor's hold
/// (rdm:phase/ship-handling-fidelity/phase-12-tractor-hold), ionization
/// (rdm:phase/ship-handling-fidelity/phase-11-ionization-handling), an
/// NPC's class rate without its outfits
/// (rdm:phase/ship-handling-fidelity/phase-15-npc-skill-speed-and-reserves),
/// and the nag ship's +1, which no ship of ours is.
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
            Ammo::Unlimited | Ammo::Other(..0) => continue,
            Ammo::Fuel(cost) if cost <= 0.0 => continue,
            Ammo::Fuel(cost) => npc.reserves.fuel.now < (cost * DRY_FUEL_SHOTS).round(),
            Ammo::Rounds(_) | Ammo::Other(_) => npc.rounds.slot(mount.spec.id) <= 0,
        };
        spending += 1;
        out += usize::from(dry);
    }
    // Every weapon out is every weapon held, or none held.
    if out == held {
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
    let after = launches_after_passes(npc, around);
    let (only, keep) = pick(npc, around, rules, (bays, after));
    Trigger {
        only,
        keep,
        bays,
        bays_after: after,
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
    let (only, keep) = pick(npc, around, rules, (false, false));
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

/// Whether `npc` launches its fighters after its passes, as the snipe
/// substate ends every frame with `_AILaunchFighter` (@0x87329-0x8732f),
/// while it snipes at a ship live in the system:
///
/// - Plundering it (goal 0xd), always: the only launch, its high level
///   making none.
/// - Attacking or sniping at it (goal 4, [`launches`]), only while a
///   fighter would find no ship slot ([`Surroundings::fighter_room`]). The
///   high level's launch came first (@0x8e6fa): with room it succeeded and
///   the bay now reloads, so the second call writes nothing (every stock
///   bay has a `Reload`). With none it wrote the bay (@0x81411) and failed
///   (`_LaunchFighter`'s slot search, @0x3d648-0x3d662; @0x81421-0x81423),
///   setting no reload, so the second call writes the bay again over the
///   passes' pick and fails the same way.
fn launches_after_passes(npc: &Npc, around: &Surroundings) -> bool {
    let after = npc
        .goal
        .plundering()
        .or_else(|| npc.goal.attacking().filter(|_| !around.fighter_room));
    let Some(ship) = after.filter(|&ship| around.live(ship)) else {
        return false;
    };
    around.state_of(ship).is_some_and(|at| {
        pursuit::frame_of(npc, &at, disabled_not_dying(around, ship))
            .is_some_and(|frame| frame.iter().any(|body| body.manoeuvre == Manoeuvre::Snipe))
    })
}

/// The part of `npc`'s trigger the fight fires: all of it, but while its
/// AI wait runs ([`Npc::wait_running`]) only its fighter bays and its
/// point defence, its pick kept; a pirate's count wait is exempt
/// ([`Npc::count_wait`], @0x34a5d-0x34a67). `_HandleShip` jumps past its thrust and
/// its whole fire block while +0x4c is above 0 (@0x34a54-0x34a72 →
/// @0x3501f), so the pick is neither fired nor cleared (@0x35012); the
/// point defence (`_AIFireTurret` → `_HandleShipPointDefense` @0x80b0d)
/// and `_AILaunchFighter` are the AI's, which the gate does not stop.
#[must_use]
pub fn gated(npc: &Npc) -> Trigger {
    let trigger = npc.trigger;
    if !npc.wait_running() || npc.count_wait() {
        return trigger;
    }
    Trigger {
        bays: trigger.bays,
        bays_after: trigger.bays_after,
        point_defence: trigger.point_defence,
        keep: trigger.keep,
        ..Trigger::default()
    }
}

/// What `_HandleShip` does with `npc`'s pick once the fight has fired it
/// (@0x34fcb-0x3501f): a pick fired ([`Trigger::only`]) whose weapon
/// record in `arsenal` has no `Flags` 0x0002 is cleared, flag and pick
/// alike, whether or not anything spawned; a secondary's stays, so it
/// fires again on the next tick its AI sets no flag of its own (its gun
/// pass keeps it). The original reads the global weapon record by the
/// pick's index (`0x3b517c[+0x72 × 0xc8] + 0x20`, @0x34fe1-0x34ff5), not
/// the ship's own mount, so a secondary the ship doesn't carry (a reused
/// slot's, see [`berth_picks`](crate::traffic::berth_picks)) is kept
/// forever, firing nothing, and a weapon with no record is cleared.
/// While its wait runs nothing was fired ([`gated`]), so its pick stays.
pub fn after_firing(npc: &mut Npc, arsenal: &Arsenal) {
    let Some(fired) = gated(npc).only else {
        return;
    };
    if !arsenal.weapon(fired).is_some_and(WeaponSpec::secondary) {
        npc.trigger.only = None;
        npc.pick = None;
    }
}

/// Whether `npc`'s fight reaches `_AIFireTurret` this tick, which runs its
/// point defence before any check of the target (@0x80b0d): dogfighting
/// (`_LowLevelAIHandler` substate 6, @0x85f17), sniping at rest (0xe,
/// @0x8730e-0x87324; moving, it brakes and runs none, @0x87329), fleeing (5, @0x85d5c-0x85d72), and approaching only
/// while facing within [`GUN_FACING`] turns of the heading it wants (7,
/// @0x868d3-0x868f6).
fn asks_turret(npc: &Npc, around: &Surroundings) -> bool {
    let Some(ship) = npc.goal.quarry() else {
        return false;
    };
    let Some(at) = around.state_of(ship) else {
        return false;
    };
    let Some(frame) = pursuit::frame_of(npc, &at, disabled_not_dying(around, ship)) else {
        return false;
    };
    frame.iter().any(|body| match body.manoeuvre {
        Manoeuvre::Approach => faces(npc, body.heading, GUN_FACING),
        Manoeuvre::Snipe => !snipe::moving(&npc.state),
        Manoeuvre::BreakOff | Manoeuvre::Rush => in_fire_box(npc, &at),
        Manoeuvre::Dogfight | Manoeuvre::Flee => true,
    })
}

/// Whether a ship at `at` is within [`pursuit::FIRE_BOX`] of `npc` on
/// both axes, each truncated (`trunc(d) + 164` unsigned ≤ 328), as a
/// break-off's and a rush's turret pass asks.
fn in_fire_box(npc: &Npc, at: &ShipState) -> bool {
    let off = npc.state.position - at.position;
    let near = |offset: f32| {
        #[allow(clippy::cast_possible_truncation)] // Offsets fit an i32.
        let whole = offset as i32;
        whole.abs() <= pursuit::FIRE_BOX
    };
    near(off.x) && near(off.y)
}

/// Whether `ship` is disabled and not breaking up, as the stand-off
/// ship's target is read for `_IsDisabled` (@0x8e4ca). Narrower than
/// [`Condition::disabled`], which counts a ship breaking up too; the two
/// agree on every ship live in the system, as the sight and the snipe's
/// launch read it.
fn disabled_not_dying(around: &Surroundings, ship: ShipRef) -> bool {
    around.condition_of(ship) == Some(Condition::Disabled)
}

/// The weapon [`trigger`] fires, if any, and the pick `npc` holds after
/// its passes, by `rules`, its bay written as its pick before them when it
/// launches first, and after them when it launches last (see the module
/// docs).
fn pick(
    npc: &Npc,
    around: &Surroundings,
    rules: FireRules,
    (launches, after): (bool, bool),
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
    if let Some((sight, frame)) = sight(npc, around, rules) {
        for body in frame.iter() {
            passes.run(&sight, body);
        }
    }
    // The snipe's `_AILaunchFighter` after its passes (@0x8732f), under
    // goal 0xd or after a goal-4 launch that found no slot: the bay is the
    // pick, the flag as the passes left it.
    if after && let Some(bay) = npc.armament.bay_to_launch(&npc.rounds) {
        passes.pick = Some(bay);
    }
    (passes.pick.filter(|_| passes.flag), passes.pick)
}

/// The pick (+0x72) and the fire flag (+0xba) as the passes leave them.
struct Passes {
    pick: Option<WeaponId>,
    flag: bool,
}

impl Passes {
    /// The passes `body` runs on `sight`, in the original's order, each
    /// gate outside its call (see the module docs).
    fn run(&mut self, sight: &Sight, body: pursuit::Body) {
        let facing = |turns: f32| faces(sight.npc, body.heading, turns);
        match body.manoeuvre {
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
            // Moving, it brakes and runs no pass (@0x870bf-0x87109 →
            // @0x87329); at rest it fires facing the plain bearing
            // (@0x872aa-0x872fe).
            Manoeuvre::Snipe if snipe::moving(&sight.npc.state) => {}
            Manoeuvre::Snipe => {
                if snipe::facing(sight.npc, &sight.at) {
                    self.gun(sight);
                    self.missile(sight);
                }
                self.turret(sight);
            }
            Manoeuvre::Flee => self.turret(sight),
            Manoeuvre::BreakOff => {
                if in_fire_box(sight.npc, &sight.at) {
                    self.turret(sight);
                }
            }
            Manoeuvre::Rush => {
                if in_fire_box(sight.npc, &sight.at) {
                    self.turret(sight);
                    if facing(GUN_FACING) {
                        self.gun(sight);
                    }
                } else if facing(GUN_FACING) {
                    self.missile(sight);
                }
            }
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

/// How `npc` sizes up the ship its goal is about, by `rules`, and the
/// frame it flies its fight by ([`pursuit::frame_of`]); none for a goal
/// that is no fight, or a ship not live or gone from the system, where
/// every pass stops at its preamble.
fn sight<'a>(
    npc: &'a Npc,
    around: &Surroundings,
    rules: FireRules,
) -> Option<(Sight<'a>, pursuit::Frame)> {
    let ship = npc.goal.quarry()?;
    let sight = sight_of(npc, ship, around, rules)?;
    let frame = pursuit::frame_of(npc, &sight.at, disabled_not_dying(around, ship))?;
    Some((sight, frame))
}

/// The turret `npc`, intact, finds to fire at `ship` among `around` by
/// `rules`, if any: `_AIFireTurret`'s search on its target +0x70
/// (@0x80b15-0x80e3c), whatever its goal. A disabled shooter searches
/// nothing (@0x80afa-0x80b01).
#[must_use]
pub fn turret_find(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    rules: FireRules,
) -> Option<WeaponId> {
    if npc.condition != Condition::Intact {
        return None;
    }
    sight_of(npc, ship, around, rules)?.turret()
}

/// How `npc` sizes up `ship`, by `rules`; none for a ship not live or
/// gone from the system.
fn sight_of<'a>(
    npc: &'a Npc,
    ship: ShipRef,
    around: &Surroundings,
    rules: FireRules,
) -> Option<Sight<'a>> {
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
    Some(Sight {
        npc,
        at,
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
        // The table's angle (@0x80c4d), against the truncated heading for
        // the arcs and `_TurretBlindSpot` (@0x80d0b).
        let angle = trig::intercept_angle(self.npc.state.position, self.at.position);
        self.best(|mount| {
            let spec = &mount.spec;
            let in_arc = match spec.guidance {
                Guidance::TurretBeam | Guidance::Turret => true,
                Guidance::FrontTurret | Guidance::RearTurret => {
                    quadrant_off(spec.guidance, heading, angle, self.quadrant_arc) <= FRONT_ARC
                }
                _ => false,
            };
            in_arc
                && spec.planet_type() == self.planet_type
                && !blind_spot(spec.flags | self.npc.hull.blind_spots, heading, angle)
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
    use crate::ai::{Goal, PlayerSide};
    use crate::catalog::WeaponRecord;
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
            ship: crate::catalog::ShipId(128),
            combat_govt: None,
            target: None,
            class: crate::catalog::EscortClass::Freighter,
            boarded: false,
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
        // (3 degrees a tick) of the heading it wants, strictly, in whole
        // degrees (@0x868d3-0x868de).
        let mut approaching = shooter(&[], ATTACK);
        approaching.state.heading = 8.9;
        assert!(asks_at_player(&approaching, 0.0, -300.0, plain));
        approaching.state.heading = 9.0;
        assert!(!asks_at_player(&approaching, 0.0, -300.0, plain));
        approaching.state.heading = 351.9;
        assert!(!asks_at_player(&approaching, 0.0, -300.0, plain));
        approaching.state.heading = 352.0;
        assert!(asks_at_player(&approaching, 0.0, -300.0, plain));
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
        // Fleeing beyond 250 (whole pixels) it runs out, as substates 1, 3
        // and 4, which call no `_AIFireTurret`: no point defence.
        let fleeing = shooter(&[], Goal::Flee(ShipRef::Player));
        assert!(asks_at_player(&fleeing, 0.0, 250.9, plain));
        assert!(!asks_at_player(&fleeing, 0.0, 251.0, plain));
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
        let of = |goal, x, y| {
            let npc = Npc {
                state: from,
                ..shooter(&[], goal)
            };
            manoeuvre(&npc, &at(x, y), false)
        };
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

    /// A ship at [`SHOOTER`] with `goal`, its hull standing off as
    /// `stands_off` says, carrying a beam 1000 long: a band of 850, and of
    /// 425 against a disabled target.
    fn standing_off(goal: Goal, stands_off: bool) -> Npc {
        let mut npc = shooter(
            &[WeaponRecord {
                guidance: 0,
                beam_length: 1000,
                ..weapon(128)
            }],
            goal,
        );
        npc.hull.stands_off = stands_off;
        npc
    }

    /// How `npc` flies at a ship (`x`, `y`) from [`SHOOTER`], disabled as
    /// `disabled` says.
    fn flown(npc: &Npc, x: f32, y: f32, disabled: bool) -> Option<Manoeuvre> {
        let at = ShipState {
            position: SHOOTER + Vec2::new(x, y),
            ..ShipState::default()
        };
        manoeuvre(npc, &at, disabled)
    }

    #[test]
    fn a_stand_off_attacker_approaches_beyond_its_band_and_snipes_within_it() {
        let npc = standing_off(ATTACK, true);
        assert_eq!(flown(&npc, 850.1, 0.0, false), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 0.0, -850.1, false), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 850.0, -850.0, false), Some(Manoeuvre::Snipe));
        assert_eq!(flown(&npc, 165.1, 0.0, false), Some(Manoeuvre::Snipe));
    }

    #[test]
    fn a_stand_off_attacker_backs_away_inside_the_box() {
        let npc = standing_off(ATTACK, true);
        assert_eq!(flown(&npc, 165.0, -165.0, false), Some(Manoeuvre::Flee));
        assert_eq!(flown(&npc, 10.0, 0.0, true), Some(Manoeuvre::Flee));
    }

    #[test]
    fn a_stand_off_attackers_band_halves_against_a_disabled_target() {
        let npc = standing_off(ATTACK, true);
        assert_eq!(flown(&npc, 600.0, 0.0, false), Some(Manoeuvre::Snipe));
        assert_eq!(flown(&npc, 600.0, 0.0, true), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 425.0, 0.0, true), Some(Manoeuvre::Snipe));
        assert_eq!(flown(&npc, 425.1, 0.0, true), Some(Manoeuvre::Approach));
    }

    #[test]
    fn a_stand_off_sniper_keeps_its_band_too() {
        let npc = standing_off(Goal::Snipe(ShipRef::Player), true);
        assert_eq!(flown(&npc, 900.0, 0.0, false), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 600.0, 0.0, false), Some(Manoeuvre::Snipe));
        assert_eq!(flown(&npc, 100.0, 0.0, false), Some(Manoeuvre::Flee));
    }

    #[test]
    fn a_stand_off_ship_flees_or_ignores_as_any_other() {
        let flee = standing_off(Goal::Flee(ShipRef::Player), true);
        assert_eq!(flown(&flee, 200.0, 0.0, false), Some(Manoeuvre::Flee));
        assert_eq!(flown(&flee, 900.0, 0.0, false), None, "running out");
        let idle = standing_off(Goal::Idle, true);
        assert_eq!(flown(&idle, 600.0, 0.0, false), None);
    }

    #[test]
    fn a_ship_that_does_not_stand_off_ignores_the_band() {
        let npc = standing_off(ATTACK, false);
        assert_eq!(flown(&npc, 600.0, 0.0, false), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 600.0, 0.0, true), Some(Manoeuvre::Approach));
        assert_eq!(flown(&npc, 100.0, 0.0, false), Some(Manoeuvre::Dogfight));
        let sniper = standing_off(Goal::Snipe(ShipRef::Player), false);
        assert_eq!(flown(&sniper, 900.0, 0.0, false), Some(Manoeuvre::Snipe));
        assert_eq!(flown(&sniper, 100.0, 0.0, false), Some(Manoeuvre::Snipe));
    }

    #[test]
    fn a_stand_off_ship_backing_away_fires_its_turrets_but_not_its_guns() {
        let mut turret = shooter(&[guided(128, 4)], ATTACK);
        turret.hull.stands_off = true;
        assert_eq!(fired(&turret, 0.0, -100.0, 50.0), id(128));
        let mut gunner = shooter(&[gun(128, 1, 1)], ATTACK);
        assert_eq!(fired(&gunner, 0.0, -100.0, 50.0), id(128), "dogfighting");
        gunner.hull.stands_off = true;
        assert_eq!(fired(&gunner, 0.0, -100.0, 50.0), None, "backing away");
    }

    #[test]
    fn a_stand_off_ship_reads_its_targets_disablement() {
        // A turret reaching 1000 gives a band of 850, 425 against a
        // disabled target. 600 off to the side, it snipes at an intact
        // target, running its turret pass; it approaches a disabled one,
        // facing away, so runs none.
        let mut npc = shooter(
            &[WeaponRecord {
                count: 100,
                ..guided(128, 4)
            }],
            ATTACK,
        );
        npc.hull.stands_off = true;
        let at =
            |condition| trigger_at_npc(&npc, 600.0, 0.0, |other| other.condition = condition).only;
        assert_eq!(at(Condition::Intact), id(128));
        assert_eq!(at(Condition::Disabled), None);
    }

    /// A target 100 above [`SHOOTER`] crossing right at 4 a tick, the
    /// shooter moving left at 1: by the table, a lead of 26 for a gun of 10
    /// a tick (10 ticks, 50 to the right relative to the shooter: 26, not
    /// 26.57, `_CalcLeadAngle` @0x85e99), and a bearing of 0.
    fn crossing() -> ShipState {
        ShipState {
            position: SHOOTER + Vec2::new(0.0, -100.0),
            velocity: Vec2::new(4.0, 0.0),
            heading: 0.0,
        }
    }

    /// A shooter at [`SHOOTER`] moving left at 1, carrying a gun (128, 10
    /// a tick), a rocket (129), a turret (130, 20 a tick), a missile (131)
    /// and a beam (132), with `goal`.
    fn leading(goal: Goal) -> Npc {
        let mut npc = shooter(
            &[
                gun(128, 1, 1),
                guided(129, 6),
                WeaponRecord {
                    speed: 2000,
                    ..guided(130, 4)
                },
                missile(131),
                guided(132, 0),
            ],
            goal,
        );
        npc.state.velocity = Vec2::new(-1.0, 0.0);
        npc
    }

    /// `npc` having fired `weapon` once: its last gun when that is a gun
    /// or a rocket.
    fn having_fired(mut npc: Npc, weapon: i16) -> Npc {
        let mut fuel = npc.reserves.fuel;
        npc.armament.fire_pick(
            WeaponId(weapon),
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut fuel,
            &mut crate::chance::NeverFires,
            &mut crate::combat::report::Reports::default(),
            RuleSource::Engine,
            &mut |_| Some(crate::combat::aim::Aim::along(0.0, None)),
        );
        npc
    }

    /// The lead of `npc` at [`crossing`] for its weapon `id`.
    fn lead_for(npc: &Npc, id: i16) -> i16 {
        let spec = npc
            .armament
            .mounts()
            .iter()
            .find(|mount| mount.spec.id.0 == id)
            .map(|mount| mount.spec);
        lead(npc, &crossing(), spec.as_ref())
    }

    #[test]
    fn a_dogfighter_leads_with_a_gun_or_rocket_pick() {
        let npc = leading(ATTACK);
        assert_eq!(trig::table_arctan(-50.0, 100.0), 26);
        assert_eq!(dogfight_lead(&npc, &crossing(), id(128)), 26);
        let rocket = lead_for(&npc, 129);
        assert_ne!(rocket, 0);
        assert_eq!(dogfight_lead(&npc, &crossing(), id(129)), rocket);
    }

    #[test]
    fn a_dogfighter_with_no_pick_leads_with_its_last_gun_or_takes_the_bearing() {
        let npc = leading(ATTACK);
        assert_eq!(dogfight_lead(&npc, &crossing(), None), 0, "no last gun");
        let fired = having_fired(npc, 128);
        assert_eq!(fired.armament.last_gun(), id(128));
        assert_eq!(dogfight_lead(&fired, &crossing(), None), 26);
    }

    #[test]
    fn a_dogfighter_with_a_turret_missile_or_beam_pick_takes_the_plain_bearing() {
        // Its last gun is not read while it holds a pick (@0x85e43-0x85e6e).
        let npc = having_fired(leading(ATTACK), 128);
        for pick in [130, 131, 132] {
            assert_eq!(dogfight_lead(&npc, &crossing(), id(pick)), 0, "{pick}");
        }
        assert_ne!(lead_for(&npc, 130), 0, "though a turret leads");
    }

    #[test]
    fn an_approacher_with_a_last_gun_leads_with_its_pick_by_the_engine_and_its_last_gun_by_the_other_reading()
     {
        let npc = leading(ATTACK);
        let at = crossing();
        for reading in RuleSource::ALL {
            assert_eq!(approach_lead(&npc, &at, id(128), reading), 0, "no last gun");
        }
        let fired = having_fired(npc, 128);
        let turret = lead_for(&fired, 130);
        assert_ne!(turret, 26);
        assert_eq!(
            approach_lead(&fired, &at, id(130), RuleSource::Engine),
            turret
        );
        assert_eq!(approach_lead(&fired, &at, id(130), RuleSource::Bible), 26);
        // `_CalcLeadAngle` takes the bearing for a pick of -1 (@0x2b05b) or
        // one outside its gate.
        assert_eq!(approach_lead(&fired, &at, None, RuleSource::Engine), 0);
        assert_eq!(approach_lead(&fired, &at, id(131), RuleSource::Engine), 0);
        assert_eq!(approach_lead(&fired, &at, None, RuleSource::Bible), 26);
    }

    #[test]
    fn the_lead_is_fixed_from_the_pick_held_before_the_passes() {
        // A dogfighter holding its gun, facing its lead at a crossing
        // target: its turret pass picks the turret first, and its gun pass
        // still faces the lead the gun gave, so the gun fires. Led from the
        // turret pick, it would want the bearing, 26 off, and fire the
        // turret.
        let mut npc = holding(leading(ATTACK), 128);
        npc.armament = Armament::new([
            (WeaponSpec::new(&gun(128, 1, 1)), 1),
            (
                WeaponSpec::new(&WeaponRecord {
                    count: 30,
                    ..guided(130, 4)
                }),
                1,
            ),
        ]);
        npc.state.heading = 26.0;
        let mut player = player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0);
        player.state.velocity = Vec2::new(4.0, 0.0);
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player),
            ..Surroundings::new(&[], &npcs)
        };
        let mut stepped = npc.clone();
        stepped.pursuit = pursuit::step(
            &npc,
            &around,
            pursuit::PursuitRules::default(),
            &mut nova_sim_base::fixture::Ranged::of(&[]),
        );
        let trigger = trigger(&stepped, &around, FireRules::default());
        assert_eq!(trigger.only, id(128));
        let frame = stepped.pursuit.frame.expect("a frame");
        assert_eq!(frame.last().map(|body| body.heading), Some(26));
    }

    #[test]
    fn a_dogfighters_gun_faces_by_whole_degrees() {
        // The bearing straight up is 0; 3 turns of 3 is 9, strictly. A
        // heading of 351.5 is 8.5 off as a float but 351, 9 off, in whole
        // degrees: it holds.
        let at = |heading: f32| {
            let mut npc = shooter(&[gun(128, 1, 1)], ATTACK);
            npc.state.heading = heading;
            fired(&npc, 0.0, -100.0, 50.0)
        };
        assert_eq!(at(351.5), None);
        assert_eq!(at(352.0), id(128));
        assert_eq!(at(8.9), id(128));
        assert_eq!(at(9.0), None);
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
        // 3 degrees a tick: under 9 for a gun, under 12 for a missile,
        // strictly, in whole degrees (@0x868db, @0x86947).
        let mut npc = shooter(&[gun(128, 1, 1)], ATTACK);
        npc.state.heading = 8.9;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(128));
        npc.state.heading = 9.0;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None);
        let mut npc = shooter(&[missile(131)], ATTACK);
        npc.state.heading = 349.0;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), id(131));
        npc.state.heading = 348.9;
        assert_eq!(fired(&npc, 0.0, -200.0, 50.0), None);
    }

    #[test]
    fn a_ship_fleeing_beyond_250_fires_no_turret() {
        // A turret reaching 300: within 250 the flee's turret pass fires
        // it; beyond, running out, no pass runs.
        let npc = shooter(
            &[WeaponRecord {
                count: 30,
                ..guided(130, 4)
            }],
            Goal::Flee(ShipRef::Player),
        );
        assert_eq!(fired(&npc, 0.0, 250.0, 50.0), id(130));
        assert_eq!(fired(&npc, 0.0, 251.0, 50.0), None);
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
        // The table's angles: (99, -100) reads 44 off the nose and the
        // diagonal 46, a degree past its 45; (99, 100) reads 136, 44 off
        // the tail, and its diagonal 134.
        let front = shooter(&[guided(130, 7)], Goal::Flee(ShipRef::Player));
        assert_eq!(
            fired(&front, 99.0, -100.0, 50.0),
            id(130),
            "44 off the nose"
        );
        assert_eq!(fired(&front, 100.0, -100.0, 50.0), None, "46");
        let rear = shooter(&[guided(130, 8)], Goal::Flee(ShipRef::Player));
        assert_eq!(fired(&rear, 99.0, 100.0, 50.0), id(130), "44 off the tail");
        assert_eq!(fired(&rear, 100.0, 100.0, 50.0), None, "46");
        assert_eq!(fired(&npc, 0.0, 232.1, 50.0), None, "out of range");
    }

    #[test]
    fn a_turrets_blind_spot_is_read_by_the_tables_bearing_and_the_truncated_heading() {
        let blind = |guidance, flags| {
            shooter(
                &[WeaponRecord {
                    flags,
                    beam_length: 100,
                    ..guided(130, guidance)
                }],
                Goal::Flee(ShipRef::Player),
            )
        };
        for guidance in [3, 4] {
            // The diagonal ahead and right is 45 off by float, the front,
            // but the table reads it 46, the side.
            let front_blind = blind(guidance, 0x1000);
            let side_blind = blind(guidance, 0x2000);
            assert_eq!(
                fired(&front_blind, 50.0, -50.0, 50.0),
                id(130),
                "{guidance}: 46"
            );
            assert_eq!(
                fired(&side_blind, 50.0, -50.0, 50.0),
                None,
                "{guidance}: the side"
            );
            // Dead ahead of a ship facing 45.9 is 45.9 off by float, the
            // side, but 45 off its truncated heading, the front.
            let mut turned = side_blind.clone();
            turned.state.heading = 45.9;
            assert_eq!(fired(&turned, 0.0, -50.0, 50.0), id(130), "{guidance}: 45");
            let mut turned = front_blind.clone();
            turned.state.heading = 45.9;
            assert_eq!(
                fired(&turned, 0.0, -50.0, 50.0),
                None,
                "{guidance}: the front"
            );
        }
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
    fn a_hopeless_plunderer_launches_its_fighters_after_its_snipes_passes_and_never_otherwise() {
        // Goal 0xd's high level makes no launch (goal 4's is @0x8e6fa);
        // the snipe's own after its passes is the only one
        // (@0x87329-0x8732f), the bay written over the passes' pick with
        // no flag (@0x81411).
        let carrier = |gun: WeaponRecord, hopeless: bool| {
            let goal = Goal::Plunder {
                ship: ShipRef::Player,
                hopeless,
            };
            let mut npc = shooter(&[gun, bay(150)], goal);
            npc.rounds = LauncherRounds::from([(WeaponId(150), 2)]);
            npc
        };
        let found = trigger_at_player(&carrier(gun(128, 1, 1), true), 0.0, -200.0);
        assert_eq!(
            (found.only, found.keep, found.bays, found.bays_after),
            (id(150), id(150), false, true),
            "the gun's flag fires the bay"
        );
        let out = trigger_at_player(&carrier(short(128), true), 0.0, -200.0);
        assert_eq!(
            (out.only, out.keep, out.bays, out.bays_after),
            (None, id(150), false, true)
        );
        let mut moving = carrier(gun(128, 1, 1), true);
        moving.state.velocity = Vec2::new(0.36, 0.0);
        let braking = trigger_at_player(&moving, 0.0, -200.0);
        assert_eq!(
            (braking.only, braking.keep, braking.bays, braking.bays_after),
            (None, id(150), false, true)
        );
        let mut reloading = carrier(gun(128, 1, 1), true);
        reloading.armament.hold(WeaponId(150));
        let held = trigger_at_player(&reloading, 0.0, -200.0);
        assert_eq!(
            (held.only, held.keep),
            (id(128), id(128)),
            "no launch, its gun"
        );
        for (y, what) in [(-200.0, "approaching"), (-100.0, "dogfighting")] {
            let plain = trigger_at_player(&carrier(gun(128, 1, 1), false), 0.0, y);
            assert_eq!(
                (plain.only, plain.bays, plain.bays_after),
                (id(128), false, false),
                "{what}"
            );
        }
    }

    #[test]
    fn a_plunderer_docking_with_a_disabled_victim_fires_nothing_and_launches_nothing() {
        let mut npc = shooter(
            &[gun(128, 1, 1), bay(150)],
            Goal::Plunder {
                ship: ShipRef::Player,
                hopeless: true,
            },
        );
        npc.rounds = LauncherRounds::from([(WeaponId(150), 2)]);
        let npcs = [npc.clone()];
        let mut player = player_at(SHOOTER.x, SHOOTER.y - 200.0, 50.0);
        player.condition = Condition::Disabled;
        let around = Surroundings {
            player: Some(player),
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            trigger(&npc, &around, FireRules::default()),
            Trigger::default()
        );
    }

    #[test]
    fn a_dogfighter_launching_a_bay_leads_from_the_bay_and_wants_the_plain_bearing() {
        // Holding its gun at a crossing target in the box, a carrier whose
        // bay is ready launches it before its low level (@0x8e6fa,
        // @0x81411), so it leads from the bay, no gun or rocket: the
        // bearing, 0, not the gun's lead of 26.
        let carrier = |rounds: LauncherRounds| {
            let mut npc = holding(leading(ATTACK), 128);
            npc.armament = Armament::new([
                (WeaponSpec::new(&gun(128, 1, 1)), 1),
                (WeaponSpec::new(&bay(150)), 1),
            ]);
            npc.rounds = rounds;
            npc.state.heading = 26.0;
            npc
        };
        let mut player = player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0);
        player.state.velocity = Vec2::new(4.0, 0.0);
        let wanted = |npc: &Npc| {
            let npcs = [npc.clone()];
            let around = Surroundings {
                player: Some(player),
                ..Surroundings::new(&[], &npcs)
            };
            let kept = pursuit::step(
                npc,
                &around,
                pursuit::PursuitRules::default(),
                &mut nova_sim_base::fixture::Ranged::of(&[]),
            );
            let heading = kept
                .frame
                .and_then(|frame| frame.last())
                .map(|body| body.heading);
            (lead_pick(npc, &around), heading)
        };
        let launching = carrier(LauncherRounds::from([(WeaponId(150), 2)]));
        assert_eq!(wanted(&launching), (id(150), Some(0)));
        let empty = carrier(LauncherRounds::default());
        assert_eq!(wanted(&empty), (id(128), Some(26)), "no bay to launch");
        // Attacking no ship live there, it launches nothing.
        let npcs = [launching.clone()];
        assert_eq!(
            lead_pick(&launching, &Surroundings::new(&[], &npcs)),
            id(128)
        );
    }

    /// The arsenal of a gun of 128 and a secondary missile of 131.
    fn gun_and_secondary() -> Arsenal {
        Arsenal::new(&[gun(128, 1, 1), secondary_missile(131)], Vec::new())
    }

    #[test]
    fn after_firing_a_pick_without_0x0002_is_cleared_and_a_secondary_is_kept() {
        let fired = |npc: Npc, pick: i16| {
            let mut npc = holding(npc, pick);
            npc.trigger.only = id(pick);
            after_firing(&mut npc, &gun_and_secondary());
            (npc.trigger.only, npc.pick)
        };
        let npc = shooter(&[gun(128, 1, 1), secondary_missile(131)], ATTACK);
        assert_eq!(fired(npc.clone(), 128), (None, None));
        assert_eq!(fired(npc.clone(), 131), (id(131), id(131)), "Flags 0x0002");
        // No flag: nothing fired, and the pick stays.
        let mut idle = holding(npc, 128);
        after_firing(&mut idle, &gun_and_secondary());
        assert_eq!((idle.trigger.only, idle.pick), (None, id(128)));
    }

    #[test]
    fn after_firing_keeps_an_uncarried_kept_secondary_by_its_weapon_record_and_clears_an_uncarried_primary()
     {
        // `_HandleShip` tests `Flags` 0x0002 of the global weapon record
        // by the pick's index (`0x3b517c[+0x72 x 0xc8] + 0x20`,
        // @0x34fe1-0x34ff5), not the ship's own mount.
        let fired = |pick: i16| {
            let mut npc = holding(shooter(&[], ATTACK), pick);
            npc.trigger.only = id(pick);
            after_firing(&mut npc, &gun_and_secondary());
            (npc.trigger.only, npc.pick)
        };
        assert_eq!(fired(131), (id(131), id(131)), "a secondary's record");
        assert_eq!(fired(128), (None, None), "a primary's record");
        assert_eq!(fired(140), (None, None), "no record");
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

    // The wait gate.

    /// A trigger with every part set.
    const FULL: Trigger = Trigger {
        primary: true,
        secondary: Some(WeaponId(131)),
        only: Some(WeaponId(128)),
        turrets_only: true,
        bays: true,
        bays_after: true,
        point_defence: true,
        keep: Some(WeaponId(128)),
    };

    #[test]
    fn a_waiting_npc_fires_nothing_but_its_point_defence_and_bays_and_keeps_its_pick() {
        use crate::catalog::StellarId;
        use crate::traffic::npc::Mode;
        let mut npc = shooter(&[gun(128, 1, 1)], ATTACK);
        npc.trigger = FULL;
        let waits = [
            Mode::Emerging { ticks_left: 5 },
            Mode::HeldAtGate { ticks_left: 5 },
            Mode::Coasting { ticks_left: 5 },
            Mode::EnteringGate {
                gate: StellarId(128),
                ticks_left: 5,
            },
        ];
        for mode in waits {
            npc.mode = mode;
            assert_eq!(
                gated(&npc),
                Trigger {
                    bays: true,
                    bays_after: true,
                    point_defence: true,
                    keep: Some(WeaponId(128)),
                    ..Trigger::default()
                },
                "{mode:?}"
            );
        }
        for mode in [
            Mode::Flying,
            Mode::LeavingGate { tick: 1 },
            Mode::JumpingIn { tick: 1 },
        ] {
            npc.mode = mode;
            assert_eq!(gated(&npc), FULL, "{mode:?}");
        }
    }

    #[test]
    fn a_pirates_count_wait_is_exempt_from_the_wait_gate() {
        // `_AIIsShipDoingCountWait` lets its trigger through (@0x34a5d-0x34a67).
        let mut npc = shooter(&[gun(128, 1, 1)], Goal::CountWait { ticks_left: 5 });
        npc.trigger = FULL;
        assert!(npc.wait_running());
        assert_eq!(gated(&npc), FULL);
        npc.mode = crate::traffic::npc::Mode::Coasting { ticks_left: 5 };
        assert_eq!(gated(&npc), FULL, "its wait is one, its goal exempts it");
    }

    #[test]
    fn a_flying_ships_ai_wait_holds_all_but_its_bays_and_point_defence() {
        // A pirate docked (goal 4, sub 0xf) or holding after a plunder:
        // unlike goal 0x16, no exemption (@0x34a54-0x34a72).
        let mut npc = shooter(&[gun(128, 1, 1)], Goal::Attack(ShipRef::Player));
        npc.trigger = FULL;
        npc.ai_wait = 1;
        assert_eq!(
            gated(&npc),
            Trigger {
                bays: true,
                bays_after: true,
                point_defence: true,
                keep: Some(WeaponId(128)),
                ..Trigger::default()
            }
        );
        npc.ai_wait = 0;
        assert_eq!(gated(&npc), FULL);
    }

    // The snipe.

    const SNIPE: Goal = Goal::Snipe(ShipRef::Player);

    #[test]
    fn a_moving_sniper_runs_no_pass_and_asks_no_point_defence_but_launches_its_fighters() {
        // `_LowLevelAIHandler`'s sub-goal 0xe, moving the double 0.35 or
        // more on an axis (see `snipe::moving`), goes straight to
        // `_AILaunchFighter` (@0x87329): no gun, missile or turret pass,
        // so no point defence (@0x80b0d). With room its high level's
        // launch (@0x8e6fa) is the one that launches: the bay then
        // reloads, and the snipe's own call (@0x8732f) launches nothing.
        let sniper = |vx: f32, vy: f32| {
            let mut npc = shooter(&[gun(128, 1, 1), guided(130, 4), bay(150)], SNIPE);
            npc.rounds = LauncherRounds::from([(WeaponId(150), 2)]);
            npc.state.velocity = Vec2::new(vx, vy);
            npc
        };
        for (vx, vy) in [(0.36, 0.0), (0.0, -0.36)] {
            let moving = trigger_at_player(&sniper(vx, vy), 0.0, -100.0);
            assert_eq!(
                (moving.only, moving.keep, moving.point_defence),
                (None, id(150), false),
                "{vx}, {vy}"
            );
            assert_eq!((moving.bays, moving.bays_after), (true, false));
        }
        let resting = trigger_at_player(&sniper(0.3, 0.3), 0.0, -100.0);
        assert_eq!(resting.only, id(130), "its turret, after its gun");
        assert!(resting.point_defence && resting.bays);
    }

    /// What `npc` fires at the player 100 above [`SHOOTER`], crossing right
    /// at 5 a tick: its gun (10 a tick) leads it at 26.57 degrees.
    fn fired_at_crossing(npc: &Npc) -> Option<WeaponId> {
        let npcs = [npc.clone()];
        let mut player = player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0);
        player.state.velocity = Vec2::new(5.0, 0.0);
        let around = Surroundings {
            player: Some(player),
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default()).only
    }

    #[test]
    fn a_sniper_at_rest_fires_facing_the_plain_bearing_not_the_lead_and_holds_at_exactly_three_turns()
     {
        // Within 3.0 x its turn rate (3) of the bearing, 0, strictly, its
        // heading truncated to whole degrees (@0x872aa-0x872fe).
        let sniper = |heading: f32| {
            let mut npc = shooter(&[gun(128, 1, 1)], SNIPE);
            npc.state.heading = heading;
            npc
        };
        let still = sniper(0.0);
        assert_eq!(
            lead_with(
                &still,
                &ShipState {
                    position: SHOOTER + Vec2::new(0.0, -100.0),
                    velocity: Vec2::new(5.0, 0.0),
                    heading: 0.0,
                },
                Some(&still.armament.mounts()[0].spec),
            ),
            26.0,
            "the table's lead"
        );
        assert_eq!(fired_at_crossing(&sniper(26.57)), None, "facing the lead");
        assert_eq!(fired_at_crossing(&sniper(8.99)), id(128));
        assert_eq!(fired_at_crossing(&sniper(9.0)), None, "strictly");
        // 351.01 and 351.99 are 351, 9 off: no gun pass, where their
        // fractional 8.99 and 8.01 would fire; 352.5 is 352, 8 off.
        assert_eq!(fired_at_crossing(&sniper(351.01)), None);
        assert_eq!(fired_at_crossing(&sniper(351.99)), None);
        assert_eq!(fired_at_crossing(&sniper(352.5)), id(128));
        assert_eq!(fired_at_crossing(&sniper(9.5)), None, "9, not less than 9");
    }

    /// A goal-4 carrier sniping at rest at the player 100 above, facing
    /// it, with a gun its pass finds and a ready bay with rounds.
    fn sniping_carrier() -> Npc {
        let mut npc = shooter(&[gun(128, 1, 1), bay(150)], SNIPE);
        npc.rounds = LauncherRounds::from([(WeaponId(150), 2)]);
        npc
    }

    /// The trigger `npc` holds at the player 100 above [`SHOOTER`], a
    /// fighter finding a ship slot or not by `room`.
    fn trigger_with_room(npc: &Npc, room: bool) -> Trigger {
        let npcs = [npc.clone()];
        let around = Surroundings {
            player: Some(player_at(SHOOTER.x, SHOOTER.y - 100.0, 50.0)),
            fighter_room: room,
            ..Surroundings::new(&[], &npcs)
        };
        trigger(npc, &around, FireRules::default())
    }

    #[test]
    fn a_sniper_whose_launch_finds_no_room_takes_its_bay_as_its_pick_after_its_passes() {
        // With no ship slot the high level's launch fails (@0x81421-0x81423,
        // @0x3d648-0x3d662), setting no reload, so the snipe's own
        // `_AILaunchFighter` (@0x8732f) writes the bay over the gun pass's
        // pick (@0x81411), the flag as the gun pass left it.
        // Both calls are asked for, and both fail: the traffic's room
        // ledger clears both (`LaunchRoom::settle`).
        let npc = sniping_carrier();
        let none = trigger_with_room(&npc, false);
        assert_eq!((none.only, none.keep), (id(150), id(150)));
        assert_eq!((none.bays, none.bays_after), (true, true));
        // With room the high level's launch (@0x8e6fa) succeeds, blind to
        // the room as it asks; the bay then reloads and the second call
        // writes nothing: the gun pass's find stands.
        let room = trigger_with_room(&npc, true);
        assert_eq!((room.only, room.keep), (id(128), id(128)));
        assert_eq!((room.bays, room.bays_after), (true, false));
    }

    #[test]
    fn a_moving_sniper_with_no_room_keeps_its_bay_as_its_pick_unflagged() {
        // A guard: it passes before and after the second call is built, as
        // the high level's bay pick comes before the passes and a moving
        // sniper runs none.
        let mut npc = sniping_carrier();
        npc.state.velocity = Vec2::new(0.0, 2.0);
        let moving = trigger_with_room(&npc, false);
        assert_eq!((moving.only, moving.keep), (None, id(150)));
        assert_eq!((moving.bays, moving.bays_after), (true, true));
    }

    #[test]
    fn a_sniper_whose_bay_reloads_takes_nothing_after_its_passes_whatever_the_room() {
        // A guard: it passes before and after the second call is built. A
        // bay reloading returns at once (@0x81403-0x8140b), writing nothing.
        let mut npc = sniping_carrier();
        npc.armament.hold(WeaponId(150));
        for room in [false, true] {
            let held = trigger_with_room(&npc, room);
            assert_eq!((held.only, held.keep), (id(128), id(128)), "{room}");
        }
    }

    #[test]
    fn after_firing_leaves_a_waiting_npcs_pick() {
        let mut npc = holding(shooter(&[gun(128, 1, 1)], ATTACK), 128);
        npc.trigger.only = id(128);
        npc.mode = crate::traffic::npc::Mode::Coasting { ticks_left: 3 };
        after_firing(&mut npc, &gun_and_secondary());
        assert_eq!(npc.pick, id(128), "nothing fired");
    }
}
