//! [`EscortAi`]: how the player's escorts fight, by the standing order
//! each follows (`_EscortAI` @0x838d2 in the `EV Nova` executable). The
//! values are in [`escort`](crate::escort).
//!
//! - **Hold Position**: it stops and waits where it is ([`Goal::Wait`],
//!   goal 6, which a hail finds busy: `_AIIsShipBusy` @0x82fac-0x82fe4),
//!   targets nothing and fires nothing; its point defence runs on each of
//!   its decisions, as `_EscortAI`'s hold tail-calls `_AIFireTurret`
//!   (@0x83e59-0x83e74, @0x83f4e, [`hold_trigger`]), and held at its gate
//!   on every tick of the hold, beside the gate wait's
//!   ([`fire_unprovoked`]). Hold keeps its AI wait, so it stays held.
//! - **Defend**: once its think's validity step has dropped a gone or
//!   disabled target ([`valid_target`]), it keeps its target while within
//!   [`DEFEND_KEEP_SQ`] (distance squared) of the player
//!   ([`kept_by_order`]), arrived or not; otherwise it takes the best
//!   threat to the player within [`DEFEND_RADIUS`] of the player
//!   ([`best_threat`]), and attacks it with every weapon. With none, it
//!   keeps formation.
//! - **Attack**: it keeps any target the validity step leaves, disabled
//!   or still arriving ([`kept_by_order`]); with none, it takes the best
//!   threat to the player at any distance, and attacks it with every
//!   weapon. With none, it keeps formation.
//!
//!   Defend and Attack keep the player too: `_EscortAI` keeps no record
//!   of where its target came from, so an escort the player hits
//!   (`_AIMakeShipAttackPlayer`, goal 4, +0x70 = 0) attacks the player at
//!   every think after, Defend measuring the player from itself, 0. Only
//!   formation drops it (@0x83f0d-0x83f1a). No source says an escort
//!   spares the player, so there is no rulebook key.
//! - **No standing command** ("Formation"), as the rulebook's
//!   [`RuleKey::EscortAi`](crate::RuleKey::EscortAi) entry says:
//!   - by the engine, it keeps formation ([`Goal::Formation`]) and guards
//!     against a ship: the one it guards still while some weapon it
//!     carries reaches it (`_AIInGunRange` @0x83edb), once its think's
//!     validity step has dropped a gone or disabled one ([`valid_target`],
//!     @0x83a9b-0x83ad2), else one drawn at random in the same think among
//!     the ships threatening the player (`_AIFightThreatToParent`
//!     @0x81de6), at any distance. Every think its low level fires
//!     `_AIFireTurret`'s one pick at its guard and runs its point defence,
//!     at any distance and with no box (`_AIEscortFireUnprovoked` from
//!     substates 1, 9, 0xb and 0xc, [`unprovoked`]); a guard disabled or
//!     gone between thinks it drops there;
//!   - by the Bible (`shïp` `InherentAI`: "What AI the ship uses when
//!     it's escorting the player"), it flies as its AI type would: a
//!     warship or interceptor attacks the best threat to the player at
//!     any distance with every weapon, keeping formation while there is
//!     none, and a trader keeps formation and fires nothing.
//! - **Return to Hangar**: a carried fighter (one with a
//!   [`Carrier`](crate::bay::Carrier)) drops its target and docks with
//!   the player ([`Goal::Dock`], `_EscortAI` @0x83e79), firing nothing
//!   and running no point defence;
//!   any other escort, holding it from an edited save, acts as with no
//!   standing command.
//!
//! A mission escort (a `ShipBehav` 1 mission ship in the player's
//! formation, [`Npc::mission_escort`]) flies here too, by the standing
//! order on its mark ([`Npc::escort_order`]): the original runs it
//! through `_EscortAI` as AI type 6, and the player's commands reach it
//! by class (`_IssueNewEscortCommand` @0x6614a-0x66176).
//!
//! From the player's brake on (its +0x50 above 0), every ship the player
//! leads takes its jump instead (`_EscortAI` @0x83a41-0x83a5f, goal 0xb
//! @0x83be3): it drops its target and keeps formation, following the
//! jump ([`Decision::follows_jump`]), whatever its order. Defend, Attack
//! and formation write +0x50 = -1.0 with their +0x4c
//! ([`Decision::ends_wait`]), as under an NPC parent.
//!
//! While the player breaks up (`_IsDying`), `_EscortAI`'s lost parent
//! frees every ship it leads at that ship's think (@0x83917-0x8397b): the
//! session frees its hired and captured escorts and launched fighters
//! first (`nova_sim`'s `dying_player_escorts` module), so only its mission
//! escorts are flown here then, and each idles with its target kept, its
//! mission's still, as the dispatch leads it again each frame
//! (@0x8fbcd-0x8fc22).
//!
//! **Out of ammunition** (`_EscortAI` @0x83b88-0x83bda,
//! [`out_of_ammo::escort_order`]), on Defend or Attack with `Flags2`
//! 0x0080: a fighter short of any ammunition returns to its hangar, its
//! wait kept, and any other escort fully dry keeps formation as with no
//! standing command (order 0), whichever
//! [`RuleKey::EscortAi`](crate::RuleKey::EscortAi) reading, its wait
//! ended. It runs again at each think while the ship is short, as the
//! word of a ship the player leads is rewritten from its class's order at
//! every think (@0x83ae4-0x83b0f).
//!
//! An escort hit takes its attacker as its target, as any ship does
//! ([`react::victim`]), and keeps it by its order as any target, but
//! never comes to another ship's help. Attacking,
//! it fires as any ship does ([`fire::trigger`]), launching the fighters
//! it carries ([`Trigger::bays`]: the high level's `_AILaunchFighter` is
//! every type's, @0x8e6fa). Each flies `_EscortAI` under it
//! ([`FleetEscortAi`](crate::ai::FleetEscortAi)), taking its Defend or
//! Attack, or Return while it keeps formation
//! ([`order_of`](crate::ai::fleet_escort::order_of)).

use crate::ai::fire::FireRules;
use crate::ai::{
    Behaviour, Decision, GateWait, Goal, Reaction, Surroundings, fire, out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Strike};
use crate::escort::{DEFEND_KEEP_SQ, DEFEND_RADIUS, EscortOrder, best_threat, threats};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Mode, Npc, NpcId};

/// The player's escorts' AI (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscortAi {
    /// What an escort with no standing command does.
    pub escort_ai: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
}

impl EscortAi {
    /// The goal of `npc`'s order block under `order` among `around`,
    /// drawing any guard on `chance` (see the module docs).
    fn goal_by(
        self,
        npc: &Npc,
        order: Option<EscortOrder>,
        around: &Surroundings,
        chance: &mut dyn Chance,
    ) -> Goal {
        if order == Some(EscortOrder::Hold) {
            // Target and +0x6c -1, goal 6 (@0x83e59-0x83e74).
            return Goal::Wait;
        }
        if order == Some(EscortOrder::Dock) && npc.carrier.is_some() {
            return Goal::Dock(ShipRef::Player);
        }
        let Some(player) = around.player.map(|player| player.state) else {
            return Goal::Formation { guard: None };
        };
        match order {
            Some(fighting @ (EscortOrder::Defend | EscortOrder::Attack)) => {
                let target = valid_target(npc, order, around);
                let radius = (fighting == EscortOrder::Defend).then_some(DEFEND_RADIUS);
                kept_by_order(order, target, player.position, around).map_or_else(
                    || attack_or_form(best_threat(npc, around, &player, radius)),
                    Goal::Attack,
                )
            }
            _ => self.no_command(npc, around, &player, chance),
        }
    }

    /// What `npc`, with no standing command, does among `around`, the
    /// player at `player`, drawing any guard on `chance`.
    fn no_command(
        self,
        npc: &Npc,
        around: &Surroundings,
        player: &ShipState,
        chance: &mut dyn Chance,
    ) -> Goal {
        match self.escort_ai {
            RuleSource::Engine => Goal::Formation {
                guard: guard(npc, around, chance),
            },
            RuleSource::Bible if npc.ai_type.trades() => Goal::Formation { guard: None },
            RuleSource::Bible => attack_or_form(best_threat(npc, around, player, None)),
        }
    }
}

impl Behaviour for EscortAi {
    /// [`EscortAi::decision`]'s goal.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// `_EscortAI` for the player's escort, in its order:
    ///
    /// 1. While the player breaks up, the lost parent: goal 0, its target
    ///    kept, no +0x4c write (@0x83917-0x8397b).
    /// 2. The out-of-ammunition fallback (@0x83b88-0x83bda,
    ///    [`out_of_ammo::escort_order`]): the order the rest flies.
    /// 3. From the player's brake on, the player's jump
    ///    (`takes_players_jump`): formation, its target dropped,
    ///    following the jump ([`Decision::follows_jump`]), its wait kept,
    ///    as no order block runs. The engine keeps no word for a ship the
    ///    player leads, so the fallback has nothing to write here.
    /// 4. The order block (`EscortAi::goal_by`), ending its AI wait
    ///    ([`Decision::ends_wait`]) in the Defend, Attack and formation
    ///    blocks (+0x4c = -1.0, @0x83c6b, @0x83d62, @0x83ea8), but not
    ///    holding or returning to its hangar: a launched fighter told to
    ///    hold or return, or sent home by the fallback, coasts out its
    ///    glide (`_LaunchFighter` @0x3d843-0x3d85b).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        if player_breaking_up(around) {
            // Goal 0 ([`Goal::Idle`], the default), its target kept.
            return Decision::default();
        }
        let order = out_of_ammo::escort_order(npc, npc.escort_order());
        if takes_players_jump(around) {
            return Decision {
                goal: Goal::Formation { guard: None },
                clears_target: true,
                follows_jump: true,
                ..Decision::default()
            };
        }
        let keeps_wait = order == Some(EscortOrder::Hold)
            || (order == Some(EscortOrder::Dock) && npc.carrier.is_some());
        Decision {
            goal: self.goal_by(npc, order, around, chance),
            ends_wait: !keeps_wait,
            ..Decision::default()
        }
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        match npc.goal {
            Goal::Attack(_) => fire::trigger(npc, around, self.fire),
            // Keeping formation, `_LowLevelAIHandler`'s substates 1, 9,
            // 0xb and 0xc call `_AIEscortFireUnprovoked` on the guard the
            // think left in +0x70 (@0x85633, @0x878f4, @0x87692,
            // @0x87e01). This tick's `npc.target` is still last tick's.
            Goal::Formation { guard: Some(guard) } => {
                unprovoked(npc, Some(guard), around, self.fire).trigger
            }
            Goal::Wait => hold_trigger(npc),
            // Return to Hangar calls no turret (@0x83e79-0x83ea3).
            _ => Trigger::default(),
        }
    }

    /// Keeping formation, its guard, unless [`unprovoked`] drops it
    /// (+0x70 = -1, @0x80e9d), so its next think draws again; idle, which
    /// it decides only under a player breaking up, the target it has, as
    /// the lost parent writes none.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        match npc.goal {
            Goal::Idle => npc.target,
            Goal::Attack(target) => Some(target),
            Goal::Formation { guard } => {
                guard.filter(|_| !unprovoked(npc, guard, around, self.fire).drops_target)
            }
            _ => None,
        }
    }

    /// [`fire_unprovoked`], whatever its standing order.
    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        fire_unprovoked(npc, around, self.fire)
    }

    /// [`react::victim`]: `_DamageShip`'s reaction block reads no AI
    /// type, so an escort (AI 5 or 6) takes its attacker as its target, as
    /// any ship does. It is never a Good Samaritan (AI 3 and 4 alone,
    /// @0x8289f-0x828bb).
    ///
    /// Its next think keeps the hit's attacker as its order keeps any
    /// target ([`kept_by_order`], the player included), or sets another.
    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::victim(npc, strike, around)
    }
}

/// Whether every ship the player leads takes its jump, its +0x50 above 0
/// (`_EscortAI` @0x83a41-0x83a5f): from its brake on (+0x50 = 1.0,
/// `_HandlePlayer` @0x6b6aa), before `_MakeEscortsHyperWithParent` puts
/// them on its warp stage. Goal 0xb's block (@0x83be3) then returns the
/// player's escort with no order block, so it keeps formation and fights
/// no more. The entry runs after the lost parent.
fn takes_players_jump(around: &Surroundings) -> bool {
    around.player_warp_stage > 0.0
}

/// Whether the player in `around` is breaking up or destroyed, as
/// `_IsDying` reads it (@0x253d: its death timer set, or its armour at or
/// below none, [`Condition::dying_or_destroyed`]): `_EscortAI`'s lost
/// parent then frees every ship it leads at that ship's think
/// (@0x83917-0x8397b). The session frees the player's own (`nova_sim`'s
/// `dying_player_escorts` module); a mission escort, which the dispatch
/// leads again each frame (@0x8fbcd-0x8fc22), idles here, its mission's
/// still.
fn player_breaking_up(around: &Surroundings) -> bool {
    around
        .player
        .is_some_and(|player| player.condition.dying_or_destroyed(player.reserves.armor))
}

/// What `npc`, an escort or fighter (AI type above 4, @0x80e59), holds
/// waiting in its gate or held at it, among `around`: [`unprovoked`] on
/// its target, as `_LowLevelAIHandler`'s substate 0 calls it
/// (@0x8521e-0x85236). Its find's fire flag ([`Trigger::only`]) is never
/// read, as `_HandleShip` skips the fire step while the wait runs
/// (@0x34a54-0x34a72, [`fire::gated`]), so it fires nothing and the pick
/// stays.
///
/// Held at its gate on Hold Position ([`Mode::HeldAtGate`] with
/// [`Goal::Wait`]), its think's point defence ([`hold_trigger`]) holds
/// beside it on every tick of the hold, as `_AIFireTurret` runs it
/// whether or not the wait runs (@0x83f4e, @0x80afa-0x80b0d). A ship
/// waiting in its gate ([`Mode::Emerging`], goal 21) runs no think,
/// whatever its goal from before.
#[must_use]
pub fn fire_unprovoked(npc: &Npc, around: &Surroundings, rules: FireRules) -> GateWait {
    let mut wait = unprovoked(npc, npc.target, around, rules);
    if matches!(npc.mode, Mode::HeldAtGate { .. }) && npc.goal == Goal::Wait {
        wait.trigger.point_defence |= hold_trigger(npc).point_defence;
    }
    wait
}

/// What `npc`, an escort or fighter holding position ([`Goal::Wait`]),
/// fires: `_EscortAI`'s hold tail-calls `_AIFireTurret` with no target
/// (@0x83e59-0x83e74, @0x83f4e), which runs its point defence unless it
/// is disabled (@0x80afa-0x80b0d) and fires nothing.
#[must_use]
pub fn hold_trigger(npc: &Npc) -> Trigger {
    Trigger {
        point_defence: npc.condition == Condition::Intact,
        ..Trigger::default()
    }
}

/// What `npc`, an escort or fighter (AI type above 4, @0x80e59), fires
/// at `target` among `around`: `_AIEscortFireUnprovoked`
/// (@0x80e4f-0x80ea8), whole. With a target (@0x80e63) it tail-calls
/// `_AIFireTurret` while the target is active and intact (+0xb8,
/// `_IsDisabled`, @0x80e98), and drops the target otherwise (+0x70 = -1,
/// @0x80e9d). It tests no range nor box, and plays no sound. The active
/// flag +0xb8 is the slot's, set at the spawn (@0x3c90b) and cleared
/// only when the ship is gone (its gate entry @0x2c078, its jump out
/// @0x2dee0), so a ship gliding in, waiting in its gate or bursting out
/// of it is active: the engine's [`Condition::active`], whatever its
/// [`Mode`](crate::traffic::npc::Mode). Such a target is kept, and
/// `_AIFireTurret` runs on it; its search skips one it does not see
/// (`_ShipVisibleToShip` @0x80b69, which hides a ship waiting in its
/// gate, goal 0x15, @0x972d).
/// `_AIFireTurret` runs the point defence first unless the shooter is
/// disabled (@0x80afa-0x80b0d), then searches by `rules`
/// ([`fire::turret_find`]) for the single best turret: a find becomes
/// its pick and its flag ([`Trigger::keep`] and [`Trigger::only`], +0x72
/// and +0xba, @0x80e3c-0x80e40). With no target it fires nothing.
#[must_use]
pub fn unprovoked(
    npc: &Npc,
    target: Option<ShipRef>,
    around: &Surroundings,
    rules: FireRules,
) -> GateWait {
    let Some(target) = target else {
        return GateWait::default();
    };
    if around.condition_of(target) != Some(Condition::Intact) {
        return GateWait {
            trigger: Trigger::default(),
            drops_target: true,
        };
    }
    let pick = fire::turret_find(npc, target, around, rules);
    GateWait {
        trigger: Trigger {
            point_defence: npc.condition == Condition::Intact,
            only: pick,
            keep: pick,
            ..Trigger::default()
        },
        drops_target: false,
    }
}

/// The target `npc`, an escort or fighter following `order`, keeps
/// through its think's validity step among `around` (`_EscortAI`
/// @0x83a9b-0x83ad2, for every parent): none once the target's slot is
/// inactive (+0xb8: gone from the system, or destroyed; a ship still
/// arriving holds its slot, @0x3c90b) or, unless the order is Attack, once
/// it is `_IsDisabled` ([`Condition::disabled`], breaking up included).
/// Phase 1 of escort-fleet-fidelity left this step to the low level's
/// [`unprovoked`], so a disabled guard stood until the next think.
#[must_use]
pub fn valid_target(
    npc: &Npc,
    order: Option<EscortOrder>,
    around: &Surroundings,
) -> Option<ShipRef> {
    let target = npc.target?;
    let condition = around.condition_of(target)?;
    if !condition.active() || (condition.disabled() && order != Some(EscortOrder::Attack)) {
        return None;
    }
    Some(target)
}

/// The target an escort or fighter following `order` keeps after its
/// think's validity step ([`valid_target`]) left it `target`, its parent
/// at `parent_at`, among `around` (`_EscortAI`, for every parent): on
/// Defend while within [`DEFEND_KEEP_SQ`] (distance squared, inclusive)
/// of its parent (@0x83c67-0x83cd6, @0x83cbc-0x83cbe); on Attack whatever
/// it is (@0x83d5d-0x83d6d); under any other order none, as Hold, Return
/// and formation keep theirs their own way. It reads no arrival, sight
/// nor type: a ship bursting out of its gate is kept.
#[must_use]
pub fn kept_by_order(
    order: Option<EscortOrder>,
    target: Option<ShipRef>,
    parent_at: Vec2,
    around: &Surroundings,
) -> Option<ShipRef> {
    match order {
        Some(EscortOrder::Attack) => target,
        Some(EscortOrder::Defend) => target.filter(|&ship| {
            around.state_of(ship).is_some_and(|at| {
                let off = at.position - parent_at;
                off.x * off.x + off.y * off.y <= DEFEND_KEEP_SQ
            })
        }),
        _ => None,
    }
}

/// An attack on `threat`, or formation with none.
fn attack_or_form(threat: Option<NpcId>) -> Goal {
    threat.map_or(Goal::Formation { guard: None }, |id| {
        Goal::Attack(ShipRef::Npc(id))
    })
}

/// The ship `npc`, keeping formation, guards against among `around`:
/// its target still, once the think's validity step has dropped a gone
/// or disabled one ([`valid_target`]), while some weapon `npc` carries
/// reaches it, arrived or not (`_AIInGunRange(ship, target, -1)`
/// @0x83eb3-0x83ee4 reads the two places, +0x18 and +0x1c, and the
/// weapons' reach alone, @0x7f9d5-0x7faf8: no slot, mode nor
/// condition). So a ship still gliding in, waiting in its gate or
/// bursting out of it ([`Npc::arrived`] false) is kept while in reach;
/// otherwise one drawn on `chance` among the ships
/// threatening the player ([`threats`], `_AIFightThreatToParent`
/// @0x83ef4), in the same think; none when there is none. The player is
/// never kept (@0x83f0d-0x83f1a).
fn guard(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Option<ShipRef> {
    let kept = match valid_target(npc, None, around) {
        Some(ShipRef::Npc(id)) => around
            .npc(id)
            .filter(|target| fire::in_reach(npc, &target.state)),
        _ => None,
    };
    if let Some(target) = kept {
        return Some(ShipRef::Npc(target.id));
    }
    let threats: Vec<NpcId> = threats(npc, around).map(|threat| threat.id).collect();
    if threats.is_empty() {
        return None;
    }
    let count = u32::try_from(threats.len()).unwrap_or(u32::MAX);
    let pick = chance.below(count) as usize;
    threats.get(pick).copied().map(ShipRef::Npc)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::PlayerSide;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{PIRATES, govts, n, player, ship};
    use crate::catalog::WeaponId;
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::escort::{EscortClass, EscortDuty};
    use crate::traffic::npc::AiType;
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    /// A turret reaching 300 pixels (332 with the margin), and a gun, as
    /// its class's stock weapons reach ([`HullSpec::stock_reach`]).
    ///
    /// [`HullSpec::stock_reach`]: crate::combat::hull::HullSpec::stock_reach
    const STOCK_REACH: f32 = 332.0;

    /// A turret reaching 300 pixels, and a gun.
    fn armed() -> Armament {
        Armament::new([
            (
                WeaponSpec::new(&crate::catalog::WeaponRecord {
                    guidance: 4,
                    speed: 1000,
                    count: 30,
                    energy_dmg: 5,
                    ..weapon(140)
                }),
                1,
            ),
            (
                WeaponSpec::new(&crate::catalog::WeaponRecord {
                    speed: 1000,
                    count: 30,
                    energy_dmg: 5,
                    ..weapon(141)
                }),
                1,
            ),
        ])
    }

    /// The escort, NPC 1, a medium warship at the centre following
    /// `order`, armed with [`armed`].
    fn escort(order: Option<EscortOrder>) -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.govt = None;
        npc.class = EscortClass::Medium;
        npc.armament = armed();
        npc.hull.stock_reach = Some(STOCK_REACH);
        npc.escort = Some(EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            },
            order,
        });
        npc
    }

    /// NPC `id` at (`x`, `y`) attacking the player.
    fn pirate(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, PIRATES, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc.goal = Goal::Attack(ShipRef::Player);
        npc
    }

    /// NPC `id` at (`x`, `y`), idle.
    fn bystander(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = ship(id, PIRATES, AiType::Warship, x, y);
        npc.class = EscortClass::Medium;
        npc
    }

    /// What `ai` decides for `npcs[0]` among `npcs`, the player at
    /// (`x`, `y`), drawing on `draws`; and the draws asked for.
    fn decide_with(
        ai: EscortAi,
        npcs: &[Npc],
        (x, y): (f32, f32),
        draws: &[u32],
    ) -> (Goal, Vec<u32>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(x, y)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut chance = Draws::of(draws);
        let goal = ai.decide(&npcs[0], &around, &mut chance);
        (goal, chance.asked)
    }

    const ENGINE: EscortAi = EscortAi {
        escort_ai: RuleSource::Engine,
        fire: FireRules {
            quadrant_arc: RuleSource::Engine,
        },
    };
    const BIBLE: EscortAi = EscortAi {
        escort_ai: RuleSource::Bible,
        fire: FireRules {
            quadrant_arc: RuleSource::Engine,
        },
    };

    fn decide(npcs: &[Npc], player_at: (f32, f32), draws: &[u32]) -> (Goal, Vec<u32>) {
        decide_with(ENGINE, npcs, player_at, draws)
    }

    /// `npc` having decided `goal`, and targeting what it decided.
    fn having(mut npc: Npc, goal: Goal, target: Option<ShipRef>) -> Npc {
        npc.goal = goal;
        npc.target = target;
        npc
    }

    /// What `ai` holds for `npcs[0]` once it decided: its trigger and
    /// target.
    fn command(ai: EscortAi, npcs: &[Npc]) -> (Trigger, Option<ShipRef>) {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        (ai.trigger(&npcs[0], &around), ai.target(&npcs[0], &around))
    }

    /// What a formation escort holds firing `weapon`, `_AIFireTurret`'s
    /// pick: its flag and its pick (+0xba, +0x72, @0x80e3c-0x80e40), with
    /// its point defence (@0x80b0d).
    fn turret_pick(weapon: i16) -> Trigger {
        Trigger {
            only: Some(WeaponId(weapon)),
            keep: Some(WeaponId(weapon)),
            point_defence: true,
            ..Trigger::default()
        }
    }

    /// Point defence alone: `_AIFireTurret` found no turret.
    const DEFENCE: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        bays: false,
        bays_after: false,
        point_defence: true,
        keep: None,
    };

    /// A turret `id` reaching 600 pixels (632 with the margin), of
    /// `energy` and `mass` damage.
    fn long_turret(id: i16, energy: i16, mass: i16) -> (WeaponSpec, u32) {
        (
            WeaponSpec::new(&crate::catalog::WeaponRecord {
                guidance: 4,
                speed: 1000,
                count: 60,
                energy_dmg: energy,
                mass_dmg: mass,
                ..weapon(id)
            }),
            1,
        )
    }

    /// The escort guarding NPC 2 in formation, armed with a gun and
    /// `turrets` reaching 632.
    fn long_guarding(turrets: &[(WeaponSpec, u32)]) -> Npc {
        let mut npc = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        let gun = armed().mounts()[1];
        npc.armament = Armament::new(
            turrets
                .iter()
                .copied()
                .chain([(gun.spec, gun.count)])
                .collect::<Vec<_>>(),
        );
        npc.hull.stock_reach = Some(632.0);
        npc
    }

    #[test]
    fn with_no_command_and_no_threat_it_keeps_formation_and_fires_nothing() {
        let npcs = [escort(None), bystander(2, 100.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]),
            (Goal::Formation { guard: None }, vec![])
        );
        let formed = having(escort(None), Goal::Formation { guard: None }, None);
        assert_eq!(command(ENGINE, &[formed]), (Trigger::default(), None));
    }

    #[test]
    fn with_no_command_it_guards_against_a_threat_drawn_at_random_at_any_distance() {
        let npcs = [
            escort(None),
            pirate(2, 100.0, 0.0),
            bystander(3, 50.0, 0.0),
            pirate(4, 5000.0, 0.0),
        ];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]),
            (Goal::Formation { guard: Some(n(2)) }, vec![2])
        );
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[1]),
            (Goal::Formation { guard: Some(n(4)) }, vec![2])
        );
    }

    /// The escort as a mission escort: marked by slot 0's `ShipBehav` 1
    /// mission and in the player's formation, with no [`EscortDuty`].
    fn mission_escort() -> Npc {
        let mut npc = escort(None);
        npc.escort = None;
        npc.mission = Some(crate::traffic::npc::MissionMark {
            order: None,
            slot: 0,
            behav: crate::traffic::ShipBehav::EscortPlayer,
            formation: Some(crate::escort::FormationSlot {
                slot: 2,
                ships: 2,
                spacing: 30.0,
            }),
        });
        npc
    }

    /// [`mission_escort`] with standing order `order`.
    fn ordered_mission_escort(order: EscortOrder) -> Npc {
        let mut npc = mission_escort();
        if let Some(mark) = &mut npc.mission {
            mark.order = Some(order);
        }
        npc
    }

    #[test]
    fn under_a_player_breaking_up_or_destroyed_it_idles_keeping_its_target() {
        // `_EscortAI`'s lost parent with `_IsDying(player)`
        // (@0x83917-0x8397b): goal 0, its target kept, no +0x4c write.
        let dying = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        let mut armourless = player(0.0, 0.0);
        armourless.reserves.armor.now = 0.0;
        for order in [
            None,
            Some(EscortOrder::Attack),
            Some(EscortOrder::Hold),
            Some(EscortOrder::Defend),
        ] {
            for side in [
                PlayerSide {
                    condition: dying,
                    ..player(0.0, 0.0)
                },
                PlayerSide {
                    condition: Condition::Destroyed,
                    ..player(0.0, 0.0)
                },
                armourless,
            ] {
                let npcs = [
                    having(
                        ordered_mission_escort_or(order),
                        Goal::Formation { guard: None },
                        Some(n(2)),
                    ),
                    pirate(2, 100.0, 0.0),
                ];
                let govts = govts(0, 0);
                let around = Surroundings {
                    player: Some(side),
                    govts: &govts,
                    ..Surroundings::new(&[], &npcs)
                };
                let decision = ENGINE.decision(&npcs[0], &around, &mut Draws::of(&[0]));
                assert_eq!(decision.goal, Goal::Idle, "{order:?} {:?}", side.condition);
                assert!(!decision.ends_wait, "{order:?}");
                let idle = having(npcs[0].clone(), Goal::Idle, Some(n(2)));
                assert_eq!(ENGINE.target(&idle, &around), Some(n(2)), "{order:?}");
            }
        }
        // Intact or disabled, it flies its order.
        for condition in [Condition::Intact, Condition::Disabled] {
            let npcs = [mission_escort(), pirate(2, 100.0, 0.0)];
            let govts = govts(0, 0);
            let around = Surroundings {
                player: Some(PlayerSide {
                    condition,
                    ..player(0.0, 0.0)
                }),
                govts: &govts,
                ..Surroundings::new(&[], &npcs)
            };
            let goal = ENGINE.decide(&npcs[0], &around, &mut Draws::of(&[0]));
            assert_ne!(goal, Goal::Idle, "{condition:?}");
        }
    }

    /// [`mission_escort`], with standing order `order` if any.
    fn ordered_mission_escort_or(order: Option<EscortOrder>) -> Npc {
        order.map_or_else(mission_escort, ordered_mission_escort)
    }

    #[test]
    fn a_mission_escort_takes_the_order_the_player_gave_it() {
        let npcs = [
            ordered_mission_escort(EscortOrder::Hold),
            pirate(2, 100.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[0]), (Goal::Wait, vec![]));
        let npcs = [
            ordered_mission_escort(EscortOrder::Attack),
            pirate(2, 100.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[0]).0, Goal::Attack(n(2)));
    }

    #[test]
    fn a_mission_escort_guards_against_a_pirate_by_the_engine_and_attacks_it_by_the_bible() {
        let npcs = [mission_escort(), pirate(2, 100.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]),
            (Goal::Formation { guard: Some(n(2)) }, vec![1])
        );
        assert_eq!(
            decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
            (Goal::Attack(n(2)), vec![])
        );
    }

    #[test]
    fn no_escort_takes_a_fellow_hired_or_mission_escort_for_a_threat() {
        let mut hired = escort(None);
        hired.id = NpcId(3);
        hired.goal = Goal::Attack(ShipRef::Player);
        let mut fellow = mission_escort();
        fellow.id = NpcId(4);
        fellow.goal = Goal::Attack(ShipRef::Player);
        for me in [escort(None), mission_escort()] {
            let npcs = [me, hired.clone(), fellow.clone()];
            assert_eq!(
                decide(&npcs, (0.0, 0.0), &[]),
                (Goal::Formation { guard: None }, vec![])
            );
            assert_eq!(
                decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                (Goal::Formation { guard: None }, vec![])
            );
        }
    }

    #[test]
    fn a_guard_within_some_weapons_reach_is_kept_and_one_beyond_it_replaced() {
        // The turret and gun reach 332.
        let kept = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        let npcs = [kept.clone(), pirate(2, 332.0, 0.0), pirate(3, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]),
            (Goal::Formation { guard: Some(n(2)) }, vec![]),
            "no draw"
        );
        let npcs = [kept.clone(), pirate(2, 0.0, 333.0), pirate(3, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[1]),
            (Goal::Formation { guard: Some(n(3)) }, vec![2]),
            "drawn among both"
        );
        let mut unarmed = kept;
        unarmed.hull.stock_reach = None;
        let npcs = [unarmed, pirate(2, 10.0, 0.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]).1,
            vec![1],
            "nothing reaches"
        );
    }

    #[test]
    fn with_no_command_it_fires_its_turrets_alone_at_its_guard_and_never_leaves_formation() {
        let guarding = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        assert_eq!(
            command(ENGINE, &[guarding, pirate(2, 100.0, 0.0)]),
            (turret_pick(140), Some(n(2))),
            "its turret's pick alone, never its gun"
        );
        for at in [(10.0, 0.0), (300.0, 300.0), (4000.0, 0.0)] {
            let npcs = [escort(None), pirate(2, at.0, at.1)];
            assert!(
                matches!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Formation { .. }),
                "{at:?}"
            );
        }
    }

    #[test]
    fn a_formation_escort_fires_its_turret_pick_at_its_guard_at_any_distance_in_reach() {
        // `_AIEscortFireUnprovoked` (@0x80e4f-0x80ea8) tests no range nor
        // box: the 164-pixel box is pursuit's (substates 0x10 and 0x11).
        let guarding = long_guarding(&[long_turret(140, 5, 1)]);
        for (x, y) in [
            (500.0, 0.0),
            (165.0, 0.0),
            (-165.0, 0.0),
            (0.0, 165.0),
            (0.0, -165.0),
            (164.9, 0.0),
        ] {
            let npcs = [guarding.clone(), pirate(2, x, y)];
            assert_eq!(
                command(ENGINE, &npcs),
                (turret_pick(140), Some(n(2))),
                "{x} {y}"
            );
        }
        // Measured from the escort, wherever it is.
        let mut away = guarding;
        away.state.position = crate::geometry::Vec2::new(1000.0, 1000.0);
        let npcs = [away, pirate(2, 1400.0, 700.0)];
        assert_eq!(command(ENGINE, &npcs).0, turret_pick(140));
    }

    #[test]
    fn a_formation_escort_fires_one_turret_type_the_best_for_its_guards_shield() {
        // `_AIFireTurret` picks the single best turret: by energy while
        // the target's shield is at or above none, else by mass
        // (@0x80e12-0x80e27).
        let guarding = long_guarding(&[long_turret(140, 5, 1), long_turret(142, 1, 9)]);
        let npcs = [guarding.clone(), pirate(2, 0.0, -100.0)];
        assert_eq!(command(ENGINE, &npcs).0, turret_pick(140), "shields up");
        let mut bare = pirate(2, 0.0, -100.0);
        bare.reserves.shield.now = -1.0;
        assert_eq!(
            command(ENGINE, &[guarding, bare]).0,
            turret_pick(142),
            "shields down"
        );
    }

    #[test]
    fn a_formation_escort_with_no_turret_find_runs_its_point_defence_alone() {
        let guarding = long_guarding(&[long_turret(140, 5, 1)]);
        let npcs = [guarding.clone(), pirate(2, 900.0, 0.0)];
        assert_eq!(
            command(ENGINE, &npcs),
            (DEFENCE, Some(n(2))),
            "beyond every turret"
        );
        let mut cloaked = pirate(2, 0.0, -100.0);
        cloaked.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        assert_eq!(
            command(ENGINE, &[guarding.clone(), cloaked]),
            (DEFENCE, Some(n(2))),
            "cloaked and unseen"
        );
        // A disabled shooter searches nothing and runs no point defence
        // (@0x80afa-0x80b01), its guard kept.
        let mut down = guarding;
        down.condition = Condition::Disabled;
        assert_eq!(
            command(ENGINE, &[down, pirate(2, 0.0, -100.0)]),
            (Trigger::default(), Some(n(2)))
        );
    }

    #[test]
    fn a_formation_escort_drops_a_disabled_or_gone_guard() {
        // `_AIEscortFireUnprovoked`: +0x70 = -1 (@0x80e9d).
        let guarding = long_guarding(&[long_turret(140, 5, 1)]);
        let mut down = pirate(2, 0.0, -100.0);
        down.condition = Condition::Disabled;
        assert_eq!(
            command(ENGINE, &[guarding.clone(), down]),
            (Trigger::default(), None),
            "disabled"
        );
        assert_eq!(
            command(ENGINE, std::slice::from_ref(&guarding)),
            (Trigger::default(), None),
            "gone"
        );
    }

    #[test]
    fn with_no_command_its_think_drops_a_disabled_guard_and_draws_another() {
        // `_EscortAI`'s step 5 drops a disabled target unless the order is
        // Attack (@0x83a9b-0x83ad2), before the formation block keeps a
        // target in reach (@0x83eb3-0x83ee4); the formation block then
        // draws again in the same think (@0x83ef4). Between thinks
        // `_AIEscortFireUnprovoked` drops it too.
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 2,
                starting: false,
            },
        ] {
            let mut down = pirate(2, 100.0, 0.0);
            down.condition = condition;
            let guarding = having(
                escort(None),
                Goal::Formation { guard: Some(n(2)) },
                Some(n(2)),
            );
            let npcs = [guarding, down, pirate(3, 200.0, 0.0)];
            assert_eq!(
                decide(&npcs, (0.0, 0.0), &[0]),
                (Goal::Formation { guard: Some(n(3)) }, vec![1]),
                "{condition:?}: drawn again among the threats in the same think"
            );
            assert_eq!(command(ENGINE, &npcs), (Trigger::default(), None));
        }
    }

    #[test]
    fn the_think_drops_a_gone_target_and_a_disabled_one_unless_the_order_is_attack() {
        let mut down = pirate(2, 100.0, 0.0);
        down.condition = Condition::Disabled;
        let npcs = [escort(None), down];
        let around = Surroundings::new(&[], &npcs);
        let targeting = |npc: Npc| having(npc, Goal::Idle, Some(n(2)));
        assert_eq!(valid_target(&targeting(escort(None)), None, &around), None);
        for order in [EscortOrder::Defend, EscortOrder::Hold, EscortOrder::Dock] {
            assert_eq!(
                valid_target(&targeting(escort(None)), Some(order), &around),
                None,
                "{order:?}"
            );
        }
        assert_eq!(
            valid_target(&targeting(escort(None)), Some(EscortOrder::Attack), &around),
            Some(n(2)),
            "attack keeps a disabled target"
        );
        let gone = having(escort(None), Goal::Idle, Some(n(9)));
        assert_eq!(
            valid_target(&gone, Some(EscortOrder::Attack), &around),
            None
        );
        let mut destroyed = pirate(2, 100.0, 0.0);
        destroyed.condition = Condition::Destroyed;
        let npcs = [escort(None), destroyed];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            valid_target(&targeting(escort(None)), Some(EscortOrder::Attack), &around),
            None,
            "its slot freed"
        );
        // The slot's active test (+0xb8) keeps a ship still arriving.
        let mut arriving = pirate(2, 100.0, 0.0);
        arriving.mode = Mode::Emerging { ticks_left: 5 };
        let npcs = [escort(None), arriving];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            valid_target(&targeting(escort(None)), None, &around),
            Some(n(2))
        );
        // The player: gone with no player in the system.
        let on_player = having(escort(None), Goal::Idle, Some(ShipRef::Player));
        assert_eq!(valid_target(&on_player, None, &around), None);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            ..around
        };
        assert_eq!(
            valid_target(&on_player, None, &around),
            Some(ShipRef::Player)
        );
    }

    #[test]
    fn with_no_command_it_keeps_and_fires_at_a_guard_in_reach_still_arriving() {
        // `_AIInGunRange` reads the two places alone (+0x18, +0x1c,
        // @0x7f9e8-0x7fa28), and `_AIEscortFireUnprovoked` the slot
        // (+0xb8, @0x80e7b) and `_IsDisabled` (@0x80e87): a ship gliding
        // in, waiting in its gate or bursting out holds its slot from its
        // spawn (+0xb8 = 1, @0x3c90b), so it is kept, not dropped, and
        // `_AIFireTurret` runs on it. Its search sees no ship waiting in
        // its gate (`_ShipVisibleToShip` @0x972d), so only the point
        // defence runs; what it sees of one gliding in or bursting out is
        // rdm:phase/escort-fleet-fidelity-followups/phase-1-turret-sight-of-arriving-ships's.
        for mode in [
            Mode::JumpingIn { tick: 1 },
            Mode::Emerging { ticks_left: 5 },
            Mode::LeavingGate { tick: 1 },
        ] {
            let mut arriving = pirate(2, 100.0, 0.0);
            arriving.mode = mode;
            assert!(!arriving.arrived(), "{mode:?}");
            let guarding = having(
                escort(None),
                Goal::Formation { guard: Some(n(2)) },
                Some(n(2)),
            );
            let npcs = [guarding, arriving, pirate(3, 200.0, 0.0)];
            assert_eq!(
                decide(&npcs, (0.0, 0.0), &[]),
                (Goal::Formation { guard: Some(n(2)) }, vec![]),
                "{mode:?}: kept, no draw"
            );
            let (trigger, target) = command(ENGINE, &npcs);
            assert_eq!(target, Some(n(2)), "{mode:?}: not dropped");
            assert!(trigger.point_defence, "{mode:?}: point defence");
            if matches!(mode, Mode::Emerging { .. }) {
                assert_eq!(trigger, DEFENCE, "{mode:?}: unseen, no turret");
            }
        }
    }

    #[test]
    fn a_disabled_target_is_dropped_under_defend() {
        let mut down = pirate(2, 100.0, 0.0);
        down.condition = Condition::Disabled;
        let defend = Some(EscortOrder::Defend);
        let npcs = [having(escort(defend), Goal::Attack(n(2)), Some(n(2))), down];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
    }

    #[test]
    fn defending_it_keeps_its_target_within_about_639_pixels_of_the_player() {
        let defend = Some(EscortOrder::Defend);
        let fighting = having(escort(defend), Goal::Attack(n(2)), Some(n(2)));
        // 408375 is between 639² (408321) and 640² (409600).
        let npcs = [fighting.clone(), bystander(2, 0.0, 639.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)), "kept");
        let npcs = [fighting.clone(), bystander(2, 0.0, 640.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None },
            "dropped"
        );
        let npcs = [fighting.clone(), bystander(2, 1000.0, 639.0)];
        assert_eq!(
            decide(&npcs, (1000.0, 0.0), &[]).0,
            Goal::Attack(n(2)),
            "measured from the player"
        );
        // 451² x 2 = 406802 is within, and 452² x 2 = 408608 is not.
        let npcs = [fighting.clone(), bystander(2, 451.0, 451.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)));
        let npcs = [fighting, bystander(2, 452.0, -452.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
    }

    #[test]
    fn defending_or_attacking_it_keeps_the_player_as_its_target_and_formation_drops_it() {
        // `_EscortAI` keeps no record of where +0x70 came from: Defend
        // measures the player from itself, 0 (@0x83c67-0x83cd6), and
        // Attack keeps it (@0x83d5d-0x83d6d); formation drops a target
        // that is its parent (@0x83f0d-0x83f1a).
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            let hit = having(
                escort(Some(order)),
                Goal::Attack(ShipRef::Player),
                Some(ShipRef::Player),
            );
            let npcs = [hit, pirate(2, 0.0, 100.0)];
            for ai in [ENGINE, BIBLE] {
                assert_eq!(
                    decide_with(ai, &npcs, (0.0, 200.0), &[]).0,
                    Goal::Attack(ShipRef::Player),
                    "{order:?} {:?}",
                    ai.escort_ai
                );
            }
        }
        let hit = having(
            escort(None),
            Goal::Attack(ShipRef::Player),
            Some(ShipRef::Player),
        );
        let npcs = [hit, pirate(2, 0.0, 100.0)];
        assert_eq!(
            decide(&npcs, (0.0, 200.0), &[0]),
            (Goal::Formation { guard: Some(n(2)) }, vec![1]),
            "formation draws a threat in its place"
        );
    }

    #[test]
    fn defending_or_attacking_it_keeps_an_active_target_bursting_out_of_its_gate() {
        // The validity step tests the slot (+0xb8) and `_IsDisabled` alone
        // (@0x83a9b-0x83ad2), and Defend the distance (@0x83c67-0x83cd6):
        // no arrival test, so a ship bursting out of its gate is kept.
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            let fighting = having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2)));
            let mut bursting = bystander(2, 0.0, 300.0);
            bursting.mode = Mode::LeavingGate { tick: 3 };
            assert!(!bursting.arrived());
            let npcs = [fighting, bursting, pirate(3, 0.0, 100.0)];
            assert_eq!(
                decide(&npcs, (0.0, 0.0), &[]).0,
                Goal::Attack(n(2)),
                "{order:?}"
            );
        }
    }

    #[test]
    fn an_escort_keeps_a_target_holding_its_course_after_its_burst() {
        let coasting = |npc: Npc| Npc {
            mode: Mode::Coasting { ticks_left: 30 },
            ..npc
        };
        let defend = Some(EscortOrder::Defend);
        let fighting = having(escort(defend), Goal::Attack(n(2)), Some(n(2)));
        let npcs = [fighting, coasting(bystander(2, 0.0, 100.0))];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)));
        let guarding = having(
            escort(None),
            Goal::Formation { guard: Some(n(2)) },
            Some(n(2)),
        );
        let npcs = [guarding, coasting(pirate(2, 10.0, 0.0))];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]),
            (Goal::Formation { guard: Some(n(2)) }, vec![])
        );
    }

    #[test]
    fn defending_it_takes_a_threat_within_550_pixels_of_the_player() {
        let defend = Some(EscortOrder::Defend);
        let npcs = [escort(defend), pirate(2, 0.0, 550.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)));
        let npcs = [escort(defend), pirate(2, 0.0, 551.0)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
        let npcs = [escort(defend), pirate(2, 0.0, 551.0), pirate(3, 400.0, 0.0)];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(3)));
        let npcs = [escort(defend)];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None }
        );
    }

    #[test]
    fn attacking_it_keeps_its_target_disabled_or_not_else_the_best_threat_anywhere() {
        let attack = Some(EscortOrder::Attack);
        let mut down = bystander(2, 5000.0, 0.0);
        down.condition = Condition::Disabled;
        let npcs = [
            having(escort(attack), Goal::Formation { guard: None }, Some(n(2))),
            down,
            pirate(3, 10.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]).0, Goal::Attack(n(2)), "kept");
        let npcs = [
            escort(attack),
            pirate(2, 5000.0, 0.0),
            pirate(3, 3000.0, 0.0),
        ];
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[]).0,
            Goal::Attack(n(3)),
            "the nearer"
        );
        let gone = having(escort(attack), Goal::Attack(n(9)), Some(n(9)));
        assert_eq!(
            decide(&[gone, bystander(2, 10.0, 0.0)], (0.0, 0.0), &[]).0,
            Goal::Formation { guard: None },
            "its target gone, and no threat"
        );
    }

    #[test]
    fn attacking_or_defending_it_fires_as_any_fighter_does() {
        for order in [EscortOrder::Attack, EscortOrder::Defend] {
            let fighting = having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2)));
            let npcs = [fighting, pirate(2, 0.0, -100.0)];
            let govts = govts(0, 0);
            let around = Surroundings {
                player: Some(player(0.0, 0.0)),
                govts: &govts,
                ..Surroundings::new(&[], &npcs)
            };
            let fires = fire::trigger(&npcs[0], &around, FireRules::default());
            assert_eq!(command(ENGINE, &npcs), (fires, Some(n(2))), "{order:?}");
            assert_eq!(
                fire::trigger(&npcs[0], &around, FireRules::default()).only,
                Some(WeaponId(141)),
                "the gun ahead"
            );
        }
    }

    #[test]
    fn defend_attack_and_formation_end_its_ai_wait_and_hold_and_return_keep_it() {
        // +0x4c = -1.0 in the Defend, Attack and formation blocks
        // (@0x83c6b, @0x83d62, @0x83ea8); Hold and Return leave it, so a
        // launched fighter told to hold or return coasts its glide.
        let mut fighter = escort(None);
        fighter.carrier = Some(crate::bay::Carrier {
            ship: ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        fighter.ai_wait = 30;
        let govts = govts(0, 0);
        for (order, threat, ends) in [
            (None, false, true),
            (Some(EscortOrder::Defend), true, true),
            (Some(EscortOrder::Defend), false, true),
            (Some(EscortOrder::Attack), true, true),
            (Some(EscortOrder::Attack), false, true),
            (Some(EscortOrder::Hold), true, false),
            (Some(EscortOrder::Dock), true, false),
        ] {
            let mut ordered = fighter.clone();
            if let Some(duty) = &mut ordered.escort {
                duty.order = order;
            }
            let npcs = if threat {
                vec![ordered, pirate(2, 0.0, 100.0)]
            } else {
                vec![ordered]
            };
            for ai in [ENGINE, BIBLE] {
                let around = Surroundings {
                    player: Some(player(0.0, 0.0)),
                    govts: &govts,
                    ..Surroundings::new(&[], &npcs)
                };
                let decision = ai.decision(&npcs[0], &around, &mut Draws::of(&[0]));
                assert_eq!(decision.ends_wait, ends, "{order:?} {threat}");
            }
        }
        let npcs = [escort(None)];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let decision = ENGINE.decision(&npcs[0], &around, &mut Draws::of(&[]));
        assert_eq!(
            (decision.goal, decision.ends_wait),
            (Goal::Formation { guard: None }, true),
            "with the player gone, formation"
        );
        // Return to Hangar with no carrier acts as no standing command:
        // formation, its wait ended.
        let mut uncarried = escort(Some(EscortOrder::Dock));
        uncarried.ai_wait = 30;
        let npcs = [uncarried];
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        for ai in [ENGINE, BIBLE] {
            let decision = ai.decision(&npcs[0], &around, &mut Draws::of(&[]));
            assert_eq!(
                (decision.goal, decision.ends_wait),
                (Goal::Formation { guard: None }, true),
                "{:?}: Return to Hangar with no carrier",
                ai.escort_ai
            );
        }
    }

    #[test]
    fn from_the_players_brake_on_every_escort_stops_fighting_and_takes_its_jump() {
        // `_EscortAI` @0x83a41-0x83a5f: the player's +0x50 above 0, its
        // brake's 1.0 included; target -1, goal 0xb, no order block.
        let govts = govts(0, 0);
        let decided = |order: Option<EscortOrder>, stage: f32| {
            let mut fighting = escort(order);
            fighting.target = Some(ShipRef::Npc(NpcId(2)));
            let npcs = [fighting, pirate(2, 0.0, 100.0)];
            let around = Surroundings {
                player: Some(player(0.0, 0.0)),
                govts: &govts,
                player_warp_stage: stage,
                ..Surroundings::new(&[], &npcs)
            };
            ENGINE.decision(&npcs[0], &around, &mut Draws::of(&[0]))
        };
        for order in [
            None,
            Some(EscortOrder::Defend),
            Some(EscortOrder::Attack),
            Some(EscortOrder::Hold),
            Some(EscortOrder::Dock),
        ] {
            let decision = decided(order, 1.0);
            assert_eq!(decision.goal, Goal::Formation { guard: None }, "{order:?}");
            assert!(decision.clears_target, "{order:?}");
            assert!(decision.follows_jump, "{order:?}");
            assert!(!decision.ends_wait, "{order:?}");
            let decision = decided(order, 0.0);
            assert!(!decision.follows_jump, "{order:?}: no jump");
            assert!(!decision.clears_target, "{order:?}: no jump");
        }
        assert_eq!(
            decided(Some(EscortOrder::Attack), 0.0).goal,
            Goal::Attack(ShipRef::Npc(NpcId(2)))
        );
    }

    #[test]
    fn a_player_breaking_up_in_its_jump_leaves_every_escort_idle_and_out_of_it() {
        // `_EscortAI`'s lost parent (@0x83917-0x8397b) runs before the
        // jump's entry (@0x83a41-0x83a5f): goal 0, its target kept, no
        // jump taken, whatever the player's +0x50.
        let govts = govts(0, 0);
        let dying = Condition::Dying {
            ticks_left: 0,
            starting: true,
        };
        let mut armourless = player(0.0, 0.0);
        armourless.reserves.armor.now = 0.0;
        for order in [
            None,
            Some(EscortOrder::Defend),
            Some(EscortOrder::Attack),
            Some(EscortOrder::Hold),
            Some(EscortOrder::Dock),
        ] {
            for side in [
                PlayerSide {
                    condition: dying,
                    ..player(0.0, 0.0)
                },
                PlayerSide {
                    condition: Condition::Destroyed,
                    ..player(0.0, 0.0)
                },
                armourless,
            ] {
                let mut fighting = escort(order);
                fighting.target = Some(ShipRef::Npc(NpcId(2)));
                let npcs = [fighting, pirate(2, 0.0, 100.0)];
                let around = Surroundings {
                    player: Some(side),
                    govts: &govts,
                    player_warp_stage: 1.0,
                    ..Surroundings::new(&[], &npcs)
                };
                let decision = ENGINE.decision(&npcs[0], &around, &mut Draws::of(&[0]));
                let at = format!("{order:?} {:?}", side.condition);
                assert_eq!(decision.goal, Goal::Idle, "{at}");
                assert!(!decision.follows_jump, "{at}");
                assert!(!decision.clears_target, "{at}");
                let mut idle = npcs[0].clone();
                idle.goal = decision.goal;
                assert_eq!(
                    ENGINE.target(&idle, &around),
                    Some(ShipRef::Npc(NpcId(2))),
                    "{at}"
                );
            }
        }
    }

    #[test]
    fn a_holding_escort_waits_and_runs_its_point_defence() {
        // `_EscortAI`'s hold writes target and +0x6c -1 and goal 6, then
        // tail-calls `_AIFireTurret` with no target (@0x83e59-0x83e74,
        // @0x83f4e): point defence, then nothing.
        let hold = Some(EscortOrder::Hold);
        let npcs = [
            having(escort(hold), Goal::Attack(n(2)), Some(n(2))),
            pirate(2, 10.0, 0.0),
        ];
        assert_eq!(decide(&npcs, (0.0, 0.0), &[]), (Goal::Wait, vec![]));
        // Hit, it holds its attacker as its target (`react::victim`) until
        // its next think, every frame at full speed (`_aiComplexity` 1,
        // `_HandleTimeAdjustment` @0x3343f-0x33491), which clears it: its
        // turret fires nothing at it.
        let holding = having(escort(hold), Goal::Wait, Some(n(2)));
        assert_eq!(
            decide(&[holding.clone(), pirate(2, 10.0, 0.0)], (0.0, 0.0), &[]),
            (Goal::Wait, vec![])
        );
        assert_eq!(
            command(ENGINE, &[holding, pirate(2, 10.0, 0.0)]),
            (
                Trigger {
                    point_defence: true,
                    ..Trigger::default()
                },
                None
            )
        );
        let idle = having(escort(None), Goal::Idle, None);
        assert_eq!(
            command(ENGINE, &[idle, pirate(2, 10.0, 0.0)]).0,
            Trigger::default(),
            "goal 0 calls no turret"
        );
    }

    #[test]
    fn attacking_and_defending_escorts_ask_for_point_defence_as_their_fight_does() {
        for order in [EscortOrder::Attack, EscortOrder::Defend] {
            let fighting = having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2)));
            let close = [fighting.clone(), pirate(2, 0.0, -100.0)];
            assert!(command(ENGINE, &close).0.point_defence, "{order:?}");
            // Approaching 300 to the right, facing up: off by 90.
            let far = [fighting, pirate(2, 300.0, 0.0)];
            assert!(!command(ENGINE, &far).0.point_defence, "{order:?}");
        }
        // Return to Hangar calls no turret (@0x83e79-0x83ea3).
        let docking = having(
            fighter(Some(EscortOrder::Dock)),
            Goal::Dock(ShipRef::Player),
            None,
        );
        assert_eq!(
            command(ENGINE, &[docking, pirate(2, 0.0, -100.0)]).0,
            Trigger::default()
        );
    }

    #[test]
    fn by_the_bible_a_warship_with_no_command_attacks_the_best_threat() {
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            let mut warship = escort(None);
            warship.ai_type = ai_type;
            let npcs = [
                warship.clone(),
                pirate(2, 5000.0, 0.0),
                pirate(3, 3000.0, 0.0),
            ];
            assert_eq!(
                decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                (Goal::Attack(n(3)), vec![]),
                "{ai_type:?}"
            );
            assert_eq!(
                decide_with(BIBLE, &[warship.clone()], (0.0, 0.0), &[]).0,
                Goal::Formation { guard: None }
            );
            let fighting = having(warship, Goal::Attack(n(3)), Some(n(3)));
            let npcs = [fighting, pirate(3, 0.0, -100.0)];
            assert_eq!(command(BIBLE, &npcs).1, Some(n(3)));
            assert_eq!(
                command(BIBLE, &npcs).0.only,
                Some(WeaponId(141)),
                "its gun ahead, as any fighter"
            );
        }
    }

    #[test]
    fn by_the_bible_a_trader_with_no_command_keeps_formation_and_fires_nothing() {
        for ai_type in [AiType::WimpyTrader, AiType::BraveTrader] {
            let mut trader = escort(None);
            trader.ai_type = ai_type;
            let npcs = [trader, pirate(2, 100.0, 0.0)];
            assert_eq!(
                decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                (Goal::Formation { guard: None }, vec![]),
                "{ai_type:?}"
            );
        }
    }

    #[test]
    fn by_the_bible_explicit_orders_are_unchanged() {
        for order in [EscortOrder::Attack, EscortOrder::Defend, EscortOrder::Hold] {
            for ai_type in [AiType::WimpyTrader, AiType::Warship] {
                let mut npc = escort(Some(order));
                npc.ai_type = ai_type;
                let npcs = [npc, pirate(2, 100.0, 0.0), pirate(3, 3000.0, 0.0)];
                assert_eq!(
                    decide_with(BIBLE, &npcs, (0.0, 0.0), &[]),
                    decide_with(ENGINE, &npcs, (0.0, 0.0), &[]),
                    "{order:?} {ai_type:?}"
                );
            }
        }
    }

    /// What `ai` holds for `npcs[0]` waiting in its gate among `npcs`.
    fn waiting(ai: &dyn Behaviour, npcs: &[Npc]) -> GateWait {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], npcs)
        };
        let mut npc = npcs[0].clone();
        npc.mode = Mode::Emerging { ticks_left: 30 };
        ai.gate_wait(&npc, &around)
    }

    const POINT_DEFENCE: GateWait = GateWait {
        trigger: Trigger {
            primary: false,
            secondary: None,
            only: None,
            bays: false,
            bays_after: false,
            point_defence: true,
            keep: None,
        },
        drops_target: false,
    };

    #[test]
    fn waiting_in_its_gate_with_an_intact_target_it_runs_its_point_defence() {
        // `_LowLevelAIHandler` at sub-goal 0 calls
        // `_AIEscortFireUnprovoked` (@0x8521e-0x85236), which for AI type
        // above 4 with an active, intact target tail-calls `_AIFireTurret`
        // (@0x80e98), whose point defence runs (@0x80b0d).
        for npc in [escort(None), fighter(None)] {
            let npcs = [having(npc, Goal::Idle, Some(n(2))), pirate(2, 900.0, 0.0)];
            assert_eq!(waiting(&ENGINE, &npcs), POINT_DEFENCE);
            assert_eq!(waiting(&BIBLE, &npcs), POINT_DEFENCE);
        }
        let mut on_player = having(escort(None), Goal::Idle, Some(ShipRef::Player));
        on_player.state.position = crate::geometry::Vec2::new(900.0, 0.0);
        assert_eq!(waiting(&ENGINE, &[on_player]), POINT_DEFENCE);
    }

    #[test]
    fn a_holding_escort_held_at_its_gate_runs_its_point_defence_through_the_wait() {
        // Hold tail-calls `_AIFireTurret` with no target (@0x83e5f,
        // @0x83f4e), point defence unless disabled (@0x80afa-0x80b0d),
        // beside the wait's `_AIEscortFireUnprovoked`, which does nothing
        // with no target (@0x8521e-0x85236). A ship waiting in its gate
        // (goal 21) runs no think, whatever its goal from before.
        let in_gate = |mode: Mode, goal: Goal, condition: Condition| {
            let mut npc = having(escort(Some(EscortOrder::Hold)), goal, None);
            npc.mode = mode;
            npc.condition = condition;
            let npcs = [npc, pirate(2, 10.0, 0.0)];
            let govts = govts(0, 0);
            let around = Surroundings {
                player: Some(player(0.0, 0.0)),
                govts: &govts,
                ..Surroundings::new(&[], &npcs)
            };
            ENGINE.gate_wait(&npcs[0], &around)
        };
        let held = Mode::HeldAtGate { ticks_left: 20 };
        let intact = Condition::Intact;
        assert_eq!(in_gate(held, Goal::Wait, intact), POINT_DEFENCE);
        assert_eq!(
            in_gate(Mode::Emerging { ticks_left: 20 }, Goal::Wait, intact),
            GateWait::default(),
            "waiting in its gate"
        );
        assert_eq!(
            in_gate(held, Goal::Attack(ShipRef::Player), intact),
            GateWait::default(),
            "held, attacking with no target"
        );
        assert_eq!(
            in_gate(held, Goal::Wait, Condition::Disabled),
            GateWait::default(),
            "disabled"
        );
    }

    #[test]
    fn a_waiting_escorts_turret_find_becomes_its_pick_and_fires_nothing() {
        // `_AIFireTurret`'s find writes +0x72 and raises +0xba
        // (@0x80e3c-0x80e40); `_HandleShip` skips the fire block while
        // the wait runs (@0x34a54-0x34a72), so it fires nothing and the
        // pick stays.
        let npcs = [
            having(escort(None), Goal::Idle, Some(n(2))),
            pirate(2, 0.0, -100.0),
        ];
        let found = GateWait {
            trigger: Trigger {
                only: Some(WeaponId(140)),
                keep: Some(WeaponId(140)),
                ..POINT_DEFENCE.trigger
            },
            drops_target: false,
        };
        assert_eq!(waiting(&ENGINE, &npcs), found, "its turret, in reach");
        let mut held = npcs[0].clone();
        held.mode = Mode::Emerging { ticks_left: 30 };
        held.trigger = found.trigger;
        assert_eq!(fire::gated(&held).only, None, "fires nothing");
        // Out of its turret's reach, it finds nothing.
        let far = [npcs[0].clone(), pirate(2, 0.0, -900.0)];
        assert_eq!(waiting(&ENGINE, &far), POINT_DEFENCE);
    }

    #[test]
    fn waiting_in_its_gate_with_its_target_down_or_gone_it_drops_it_and_runs_nothing() {
        // `_AIEscortFireUnprovoked` sets +0x70 to -1 (@0x80e9d).
        let dropped = GateWait {
            trigger: Trigger::default(),
            drops_target: true,
        };
        let mut down = pirate(2, 10.0, 0.0);
        down.condition = Condition::Disabled;
        let targeting = having(escort(None), Goal::Idle, Some(n(2)));
        assert_eq!(
            waiting(&ENGINE, &[targeting.clone(), down]),
            dropped,
            "down"
        );
        let mut destroyed = pirate(2, 10.0, 0.0);
        destroyed.condition = Condition::Destroyed;
        assert_eq!(
            waiting(&ENGINE, &[targeting.clone(), destroyed]),
            dropped,
            "destroyed"
        );
        assert_eq!(waiting(&ENGINE, &[targeting]), dropped, "gone");
    }

    #[test]
    fn waiting_in_its_gate_with_no_target_or_disabled_it_runs_nothing_and_keeps_its_target() {
        // With no target `_AIEscortFireUnprovoked` returns (@0x80e63); a
        // disabled shooter runs no point defence (@0x80afa).
        let idle = [having(escort(None), Goal::Idle, None), pirate(2, 10.0, 0.0)];
        assert_eq!(waiting(&ENGINE, &idle), GateWait::default());
        let mut disabled = having(escort(None), Goal::Idle, Some(n(2)));
        disabled.condition = Condition::Disabled;
        assert_eq!(
            waiting(&ENGINE, &[disabled, pirate(2, 10.0, 0.0)]),
            GateWait::default()
        );
    }

    /// How the escort, `npcs[0]`, answers `strike` among `npcs`, the
    /// player at the centre.
    fn reacts(npcs: &[Npc], strike: &crate::combat::Strike) -> crate::ai::Reaction {
        reacts_by(RuleSource::Engine, npcs, strike)
    }

    /// [`reacts`] with the focus test read by `focus`.
    fn reacts_by(
        focus: RuleSource,
        npcs: &[Npc],
        strike: &crate::combat::Strike,
    ) -> crate::ai::Reaction {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            hit_focus: focus,
            ..Surroundings::new(&[], npcs)
        };
        ENGINE.react(&npcs[0], strike, &around)
    }

    #[test]
    fn an_escort_hit_by_a_pirate_takes_it_as_its_target() {
        // `_DamageShip`'s reaction block reads no AI type (@0x3bb7a-0x3bed2):
        // an escort (AI 5 or 6) takes its attacker as its target.
        let npcs = [escort(None), pirate(2, 100.0, 0.0)];
        let strike = crate::combat::Strike {
            nominal: crate::combat::Nominal {
                mass: 4.0,
                energy: 6.0,
            },
            ..crate::combat::fixture::strike(n(1), n(2))
        };
        assert_eq!(
            reacts(&npcs, &strike),
            crate::ai::Reaction {
                target: Some(n(2)),
                provoked: 10.0,
                goal: None,
            }
        );
    }

    #[test]
    fn an_escort_is_no_good_samaritan() {
        // `_DoGoodSamaritan` takes AI 3 and 4 alone (@0x8289f-0x828bb).
        let mut trader = bystander(2, 3000.0, 0.0);
        trader.ai_type = AiType::WimpyTrader;
        let npcs = [escort(None), trader];
        let strike = crate::combat::fixture::strike(n(2), ShipRef::Player);
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        assert!(
            crate::ai::react::answer(&npcs[0], &strike, &around, RuleSource::Engine)
                .goal
                .is_some(),
            "a warship would answer"
        );
        assert_eq!(reacts(&npcs, &strike), crate::ai::Reaction::default());
    }

    #[test]
    fn an_escort_attacking_a_target_ahead_of_it_lets_a_hit_be_unless_the_attacker_is_near() {
        // The focus skip (@0x3bc25-0x3bcac) reads no AI type: an escort
        // keeps its target unless dist²(attacker) is at most 0.25 ×
        // dist²(target).
        let mut fighting = having(
            escort(Some(EscortOrder::Attack)),
            Goal::Attack(n(3)),
            Some(n(3)),
        );
        fighting.state.heading = 0.0;
        let npcs = [fighting, pirate(2, 100.0, 0.0), pirate(3, 0.0, -60.0)];
        let strike = crate::combat::fixture::strike(n(1), n(2));
        assert_eq!(reacts(&npcs, &strike), crate::ai::Reaction::default());
        let mut near = npcs.clone();
        near[1].state.position = crate::geometry::Vec2::new(29.0, 0.0);
        assert_eq!(
            reacts(&near, &strike).target,
            Some(n(2)),
            "its attacker within half its target's distance"
        );
    }

    #[test]
    fn by_the_other_reading_an_escort_attacking_a_nearer_target_ahead_of_it_lets_a_hit_be() {
        let mut fighting = having(
            escort(Some(EscortOrder::Attack)),
            Goal::Attack(n(3)),
            Some(n(3)),
        );
        fighting.state.heading = 0.0;
        let npcs = [fighting, pirate(2, 100.0, 0.0), pirate(3, 0.0, -10.0)];
        let strike = crate::combat::fixture::strike(n(1), n(2));
        let bible = |npcs: &[Npc]| reacts_by(RuleSource::Bible, npcs, &strike);
        assert_eq!(bible(&npcs), crate::ai::Reaction::default());
        let mut far = npcs.clone();
        far[2].state.position = crate::geometry::Vec2::new(0.0, -60.0);
        assert_eq!(bible(&far).target, Some(n(2)), "its target too far");
    }

    /// The escort as the player's fighter, out of its bay.
    fn fighter(order: Option<EscortOrder>) -> Npc {
        Npc {
            class: EscortClass::Fighter,
            carrier: Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            }),
            ..escort(order)
        }
    }

    /// `npc` short of ammunition as `left` says, its hull of `Flags2`
    /// 0x0080.
    fn short(npc: Npc, left: AmmoLeft) -> Npc {
        crate::ai::fixture::short_of_ammo(npc, left, true)
    }

    /// What `ai` decides for `npcs[0]` among `npcs`, the player at the
    /// centre with its warp stage at `stage`, drawing on `draws`.
    fn decision_of(ai: EscortAi, npcs: &[Npc], stage: f32, draws: &[u32]) -> Decision {
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            player_warp_stage: stage,
            ..Surroundings::new(&[], npcs)
        };
        ai.decision(&npcs[0], &around, &mut Draws::of(draws))
    }

    #[test]
    fn a_fighter_on_attack_out_of_ammunition_docks_with_the_player_keeping_its_wait() {
        // `_EscortAI`'s fallback writes word 3 for AI type 5 (@0x83bc9),
        // and the Return block docks it, +0x4c kept (@0x83e79-0x83ea3).
        for order in [EscortOrder::Attack, EscortOrder::Defend] {
            for left in [AmmoLeft::OnlyUnlimited, AmmoLeft::Dry] {
                let mut dry = short(
                    having(fighter(Some(order)), Goal::Attack(n(2)), Some(n(2))),
                    left,
                );
                dry.ai_wait = 30;
                let npcs = [dry, pirate(2, 0.0, 100.0)];
                for ai in [ENGINE, BIBLE] {
                    let decision = decision_of(ai, &npcs, 0.0, &[0]);
                    assert_eq!(
                        decision.goal,
                        Goal::Dock(ShipRef::Player),
                        "{order:?} {left:?}"
                    );
                    assert!(!decision.ends_wait, "{order:?} {left:?}");
                }
            }
        }
    }

    #[test]
    fn a_defending_escort_with_no_firepower_keeps_formation_ending_its_wait() {
        // Any other type takes word 0 fully dry (@0x83bda), and the
        // formation block ends its wait (+0x4c = -1.0, @0x83ea8-0x83eb0);
        // with unlimited weapons left it keeps its order.
        for order in [EscortOrder::Defend, EscortOrder::Attack] {
            let dry = short(
                having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2))),
                AmmoLeft::Dry,
            );
            let npcs = [dry.clone(), pirate(2, 0.0, 100.0)];
            let decision = decision_of(ENGINE, &npcs, 0.0, &[0]);
            assert_eq!(
                decision.goal,
                Goal::Formation { guard: Some(n(2)) },
                "{order:?}: it guards, by the engine"
            );
            assert!(decision.ends_wait, "{order:?}");
            // Order 0 is no standing command, whichever reading.
            let mut formed = dry.clone();
            if let Some(duty) = &mut formed.escort {
                duty.order = None;
            }
            let plain = [formed, pirate(2, 0.0, 100.0)];
            assert_eq!(
                decision_of(BIBLE, &npcs, 0.0, &[0]).goal,
                decision_of(BIBLE, &plain, 0.0, &[0]).goal,
                "{order:?}: by the Bible"
            );
            let unlimited = short(
                having(escort(Some(order)), Goal::Attack(n(2)), Some(n(2))),
                AmmoLeft::OnlyUnlimited,
            );
            let npcs = [unlimited, pirate(2, 0.0, 100.0)];
            assert_eq!(
                decision_of(ENGINE, &npcs, 0.0, &[0]).goal,
                Goal::Attack(n(2)),
                "{order:?}: unlimited weapons left"
            );
        }
    }

    #[test]
    fn on_the_players_jump_an_escort_out_of_ammunition_still_takes_the_jump() {
        // The fallback (@0x83b88-0x83bda) runs before goal 0xb's return
        // (@0x83be3), which then returns with no order block.
        for npc in [
            fighter(Some(EscortOrder::Attack)),
            escort(Some(EscortOrder::Defend)),
        ] {
            let npcs = [short(npc, AmmoLeft::Dry), pirate(2, 0.0, 100.0)];
            for ai in [ENGINE, BIBLE] {
                let decision = decision_of(ai, &npcs, 1.0, &[0]);
                assert_eq!(decision.goal, Goal::Formation { guard: None });
                assert!(decision.follows_jump);
                assert!(decision.clears_target);
                assert!(!decision.ends_wait);
            }
        }
    }

    #[test]
    fn a_fighter_returning_to_its_hangar_docks_with_the_player_firing_nothing() {
        let dock = Some(EscortOrder::Dock);
        let npcs = [
            having(fighter(dock), Goal::Attack(n(2)), Some(n(2))),
            pirate(2, 100.0, 0.0),
        ];
        for ai in [ENGINE, BIBLE] {
            assert_eq!(
                decide_with(ai, &npcs, (0.0, 0.0), &[]),
                (Goal::Dock(ShipRef::Player), vec![])
            );
            let docking = having(fighter(dock), Goal::Dock(ShipRef::Player), Some(n(2)));
            assert_eq!(
                command(ai, &[docking, pirate(2, 100.0, 0.0)]),
                (Trigger::default(), None)
            );
        }
    }

    #[test]
    fn an_escort_of_its_own_told_to_dock_acts_with_no_standing_command() {
        let npcs = [escort(Some(EscortOrder::Dock)), pirate(2, 100.0, 0.0)];
        let plain = [escort(None), pirate(2, 100.0, 0.0)];
        for ai in [ENGINE, BIBLE] {
            assert_eq!(
                decide_with(ai, &npcs, (0.0, 0.0), &[0]),
                decide_with(ai, &plain, (0.0, 0.0), &[0])
            );
        }
        assert_eq!(
            decide(&npcs, (0.0, 0.0), &[0]).0,
            Goal::Formation { guard: Some(n(2)) }
        );
    }

    /// The escort attacking NPC 2, carrying a bay too (weapon 150, two
    /// fighters aboard, ready), and the pirate it attacks 600 ahead, beyond
    /// its gun and turret.
    fn attacking_carrier() -> [Npc; 2] {
        let bay = WeaponSpec::new(&crate::catalog::WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            reload: 60,
            flags: crate::combat::weapon::SECONDARY,
            ..weapon(150)
        });
        let mut carrier = having(
            escort(Some(EscortOrder::Attack)),
            Goal::Attack(n(2)),
            Some(n(2)),
        );
        let mut mounts: Vec<(WeaponSpec, u32)> = carrier
            .armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec, mount.count))
            .collect();
        mounts.push((bay, 1));
        carrier.armament = Armament::new(mounts);
        carrier.rounds = crate::combat::armament::LauncherRounds::from([(WeaponId(150), 2)]);
        [carrier, pirate(2, 0.0, -600.0)]
    }

    #[test]
    fn an_escort_attacking_launches_the_fighters_it_carries_as_any_ship_does() {
        // The high level's `_AILaunchFighter` is every type's (@0x8e6fa),
        // the player's escorts' too, and its bay is its pick as any ship's.
        let npcs = attacking_carrier();
        let govts = govts(0, 0);
        let around = Surroundings {
            player: Some(player(0.0, 0.0)),
            govts: &govts,
            ..Surroundings::new(&[], &npcs)
        };
        let (trigger, target) = command(ENGINE, &npcs);
        assert_eq!(target, Some(n(2)));
        assert!(trigger.bays, "{trigger:?}");
        assert_eq!(
            trigger,
            fire::trigger(&npcs[0], &around, FireRules::default())
        );
    }
}
