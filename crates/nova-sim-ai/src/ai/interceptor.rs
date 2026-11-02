//! Nova's interceptor (AI type 4, `_InterceptorAI` @0x8c896 in the `EV
//! Nova` executable): police.
//!
//! - It hunts as a warship does ([`hunt`]), its government's word on the
//!   player first (`_InterceptorAI` @0x8cb0d-0x8cbf3, the same as
//!   `_WarshipAI`'s; see [`warship`](super::warship)), but with no
//!   shield retreat (a person's at its `Coward` only by the Bible's
//!   [`RuleKey::PersonCoward`](crate::RuleKey::PersonCoward)), and its
//!   odds retreat is gated on its government's `Flags` 0x0100
//!   rather than 0x0010 (@0x8d180, likely a slip in the original, kept as
//!   the default). Its word reaches a blessed player as
//!   [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks)
//!   says (`_InterceptorAI` @0x8cb5c reads no blessing). Its turn on the
//!   player through `_AIMakeShipAttackPlayer`, the word's (@0x8cb8f) or
//!   its pick's ([`Decision::turns_on_player`]), holds through the
//!   inspection, hold or patrol it goes on to when the hunt gives it
//!   nothing to fight. Unlike a warship's, its think doesn't let go of a
//!   ship breaking up ([`let_go`]): goal 4 goes on to the high level, which
//!   ends it ([`attack_drop`](crate::ai::attack_drop)), unless the wait
//!   runs or it can't see the ship.
//! - Idle or travelling ([`Goal::Land`], [`Goal::EnterGate`]), it searches
//!   for no target while it holds one (+0x70,
//!   [`held_attack`](crate::ai::held_attack)): provoked by an active one it
//!   attacks it; otherwise it keeps its goal holding the target, with no
//!   inspection and no patrol (@0x8cc93 → @0x8d13f → @0x8d14d), an ally or
//!   a fleet-mate as much as any. A target its think lets go ([`let_go`],
//!   @0x8d2fd-0x8d33a), held, provoker, the word's or a fresh pick, leaves
//!   it in [`Goal::Idle`] with no target and no patrol. Its out-of-ammo
//!   flight still runs on either.
//! - Idle with nothing to fight, it inspects (@0x8ce34): it flies up to a
//!   ship picked at random among the others in the system (the player
//!   first, then the NPCs in order), but not the one it last inspected
//!   nor another interceptor (the original's AI type 4, @0x8cd4e,
//!   @0x8cdfe, [`Npc::original_ai_type`]: a led one reads 6 and a ship
//!   with no AI -1, whatever its class, so either may be picked); the
//!   original retries `Rand(64)` slots, here one draw among them. It
//!   picks only a ship it sees (`_ShipVisibleToShip`, @0x8cd5f in the
//!   count, @0x8ce13 in the pick; [`sight`](super::sight)), the player
//!   too: a cloaked player it can't see is no pick, and so raises no
//!   pre-scan hail.
//!   Within [`INSPECT_REACH`] of it on both axes
//!   (@0x8f11e, [`within_reach`]) it has looked it over, and decides
//!   again; a pick already within reach ends in the think that made it,
//!   as goal 7's block runs after the think (@0x8f11e-0x8f394,
//!   [`Decision::looked_over`]). The traffic remembers the ship
//!   ([`Npc::inspected`]). Having looked the player over, it has scanned
//!   the player
//!   ([`Traffic::take_on_line`](crate::traffic::Traffic::take_on_line)): the
//!   session checks the player's mission cargo against the interceptor's
//!   government (`_ScanPlayer` @0x7e46e; see the
//!   `nova_sim::mission` module's "Cargo"), failing a mission
//!   flagged 0x0020 or fining the smuggler, then its illegal `jünk` and
//!   outfits (see [`scan`](crate::scan)), convicting the smuggler; any
//!   find sets the interceptor attacking the player. A fresh pick of the
//!   player raises the pre-scan hail ([`CommCall::PreScan`], @0x8ce4b),
//!   which the session says when the interceptor has a government, the
//!   player carries matching mission cargo and the line is idle (see
//!   [`call_for_help`](super::call_for_help)); the hail comes in the
//!   pick, so it stands when the out-of-ammo retreat then turns the
//!   inspection into a flight.
//! - Short of ammunition with a hull of `Flags2` 0x0080
//!   (@0x8d343-0x8d391), it flees the ship it attacks, snipes at or
//!   inspects, and with none it stops, unless it is leaving; it has no
//!   fully-dry drop (see [`out_of_ammo`]).
//! - With nothing to inspect it patrols ([`idle_block::patrol`],
//!   @0x8d00d-0x8d136): its `_SelectRandomStellarDest` call is
//!   `(ship, 0, 1)` (@0x8d027-0x8d03a), plain stellars only
//!   ([`Admit::PlainOnly`](crate::ai::Admit::PlainOnly)): never a gate,
//!   and an uninhabited stellar as
//!   [`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)
//!   says. It has no visited test, so it goes on patrolling stellar to
//!   stellar. A destroyed pick makes it stop and wait ([`Goal::Wait`],
//!   state 6, @0x8d124); with no pick it leaves, or waits when it can't
//!   jump.
//! - Waiting (state 6) it inspects nothing, looks for no target and
//!   patrols nowhere: those blocks need a goal of 1 or below, or 0x14
//!   (@0x8cc6d-0x8cc7d, [`gate_closed`]). So does one holding, leaving
//!   (goal 2), plundering (0xd), drifting (0xe), mining or scooping: it
//!   keeps the goal, a lost lead's that the succession copied included,
//!   with a hostile player in view though it is. Only provocation or its
//!   government's word lifts it.
//! - **Goal 4** ([`held_attack`](super::held_attack)): provoked with a
//!   target, any goal but 3, 7, 9 and 0xf becomes goal 4 on it
//!   (@0x8cbf6-0x8cc31), an idle or patrolling one's too, and goal 4 closes
//!   the gate, provoked or not. So it runs no search: its retreats, its
//!   disabled drop (@0x8d2fa-0x8d33a) and its out-of-ammo retreat, then
//!   the high level's slot test and state-4 block end the attack that
//!   think, each with its own writes: a target gone (@0x8e263), its
//!   fleet's (@0x8e2e9-0x8e399) or breaking up (@0x8e3ef) is cleared, and
//!   an ally it spares (@0x8e277-0x8e2db) is kept. Idle with a target it kept, unprovoked, it searches, inspects
//!   and patrols nothing ([`searches_nothing`], @0x8cc93-0x8ccc9 →
//!   @0x8d13f), and holds it; provoked, it takes goal 4 on it again
//!   each think, which the ally drop idles again, until the ally's slot
//!   goes.
//! - **Fleeing** (goal 3) behind its closed gate, it keeps the flight on
//!   its target whatever that ship's state, searching nothing, as the
//!   warship does ([`hunt`]): goal 3's block ends a flight from a ship
//!   gone that think (@0x8dff6 → @0x8e228, [`flee_drop`](super::flee_drop)),
//!   and the next think searches.
//! - Short of ammunition, a waiting interceptor goes idle as a stopped
//!   one does (state 6 to 3 to 0, @0x8d391 and @0x8e228; see
//!   [`out_of_ammo`]), and a plundering one flees its ship. In its count
//!   wait (goal 0x16) it returns at once (@0x8c8be).
//! - Inspecting a ship it no longer sees (`_ShipVisibleToShip`, see
//!   [`sight`](super::sight)), it drops it: target -1, state 0
//!   (@0x8d39a-0x8d3ee, [`lost_sight::after_inspection`]), and its
//!   travel stellar -1 (@0x8d3df). An inspection
//!   the out-of-ammo retreat turned into a flight is left alone.
//! - Inspecting (state 7) it runs no search of its own (the block needs
//!   state 1 or below, or 0x14, @0x8cc6d-0x8cc7d; [`hunt`] answers only
//!   its provoker) while the inspection holds ([`inspection_holds`]): one
//!   that ends this think, within reach or gone, is let go as state 0,
//!   its travel stellar -1 when it scanned the player (@0x8f369) and kept
//!   for any other ship (@0x8f391), and the hunt runs its full search, attacking any pick
//!   (@0x8cce6-0x8cd02). Holding, last in its think, with a government (+0x98,
//!   @0x8d404), it re-picks ([`re_pick`], `_SelectWarshipTarget` @0x8d41e):
//!   a pick that writes state 4 itself (a grudge, a mission hunter's, the
//!   help, or the player through `_AIMakeShipAttackPlayer`) it attacks;
//!   the hunt's or the xenophobe's NPC pick, which writes the target alone
//!   (@0x8ad11, @0x8a3f0), it inspects instead; with none it keeps its
//!   inspection (@0x8d42d-0x8d431). The re-pick runs in the think that
//!   picks a fresh inspection too, rolling the enemy class a second time,
//!   and never asks the threat step, which needs no target (@0x8ad3c). Its
//!   reach then reads the new target (@0x8f11e): a re-pick away from the
//!   player scans nothing ([`Decision::re_picked`]), and a re-pick already
//!   within reach is looked over at once ([`Decision::looked_over`]), a
//!   scan if it is the player.

use crate::ai::fire::FireRules;
#[cfg(doc)]
use crate::ai::target::let_go;
use crate::ai::target::{TargetRules, re_pick_target};
use crate::ai::warship::{BreakingUp, HuntTarget, Retreat, hunt};
use crate::ai::{
    Behaviour, CommCall, Decision, Goal, Reaction, Surroundings, fire, idle_block, lost_sight,
    out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::govt::INTERCEPTORS_RETREAT;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// How near, in pixels on either axis, an interceptor comes to the ship
/// it inspects (100.0 @0xdd060).
pub const INSPECT_REACH: f32 = 100.0;

/// Whether a ship `off` from an interceptor is within its reach: within
/// [`INSPECT_REACH`] on both axes (@0x8f11e).
#[must_use]
pub fn within_reach(off: Vec2) -> bool {
    off.x.abs() <= INSPECT_REACH && off.y.abs() <= INSPECT_REACH
}

/// Whether `npc` inspects a ship (state 7) that its think doesn't end:
/// one whose slot is active among `around` (+0xb8 @0x8f158, breaking up or
/// not) and out of its reach ([`within_reach`]). Within
/// reach, `_HighLevelAIHandler`'s state-7 block lets it go (@0x8f11e,
/// @0x8f391: state 0, +0x70 -1), and so it does a ship gone; either way
/// the think goes on as from state 0, with the hunt's full search
/// (`_InterceptorAI` @0x8cc6d, `_SelectWarshipTarget` @0x8cce6).
#[must_use]
pub fn inspection_holds(npc: &Npc, around: &Surroundings) -> bool {
    let Goal::Inspect(ship) = npc.goal else {
        return false;
    };
    reach_of(npc, ship, around) == Some(false)
}

/// Whether `ship` is within `npc`'s reach ([`within_reach`]), when its slot
/// is active among `around` (+0xb8 @0x8f158).
fn reach_of(npc: &Npc, ship: ShipRef, around: &Surroundings) -> Option<bool> {
    around
        .state_of(ship)
        .filter(|_| around.active(ship))
        .map(|state| within_reach(state.position - npc.state.position))
}

/// Whether `npc`'s goal is behind `_InterceptorAI`'s gate among `around`:
/// its search, inspection and patrol run for a goal of 1 or below, or
/// 0x14, read from the goal in hand (@0x8cc6d-0x8cc7d), so not for a wait
/// (6), a hold (0xa), a jump out (2), a plunder (0xd), a drift (0xe), a
/// count wait (0x16), a mine, scoop or stellar attack (0x10-0x12), a
/// fighter's return (5), a help (9, 0xf), the regroup pause (0x13), or an
/// inspection that holds ([`inspection_holds`]). An attack or snipe is
/// goal 4, which closes the gate too, but is flown before it
/// ([`held_attack`](super::held_attack)). A flight (3) closes it too, and
/// [`hunt`] keeps it on its target whatever that ship's state, searching
/// nothing: one from a ship gone is ended by goal 3's block that frame
/// (@0x8dff6 → @0x8e228, [`flee_drop`](super::flee_drop)), and the search
/// waits for the next think. An inspection this think ends, and a follow or
/// formation (the other reading of
/// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)'s led ship), stay
/// open too.
///
/// Inside the gate the original skips the search and patrol for a ship
/// mid-jump (+0x50 above 0, @0x8cc9d, @0x8ccf8). The high level's hold
/// zeroes +0x50 every frame for goals 0, 1 and 0x14 (@0x8d513-0x8d56b),
/// so only a goal 0 written between the hold and the think meets it: a
/// pirate's announced capture dropping a fleeing interceptor mid-jump in a
/// higher slot (@0x8484a-0x848b1). The engine searches in that think, and
/// the original in the next: a one-think lag, not filed.
#[must_use]
pub fn gate_closed(npc: &Npc, around: &Surroundings) -> bool {
    match npc.goal {
        Goal::Inspect(_) => inspection_holds(npc, around),
        Goal::Wait
        | Goal::Hold
        | Goal::JumpOut
        | Goal::Plunder { .. }
        | Goal::Drift
        | Goal::CountWait
        | Goal::Mine
        | Goal::Scoop
        | Goal::AttackStellar(_)
        | Goal::Dock(_)
        | Goal::Assist(_)
        | Goal::Regroup { .. }
        | Goal::Flee(_) => true,
        Goal::Idle
        | Goal::Land(_)
        | Goal::EnterGate(_)
        | Goal::Attack(_)
        | Goal::Snipe(_)
        | Goal::Follow { .. }
        | Goal::Formation { .. } => false,
    }
}

/// Whether `npc`, behind `_InterceptorAI`'s open gate, searches, inspects
/// and patrols nothing for the target it kept: idle, landing or entering a
/// gate (the engine's goal 0 and 1) with +0x70 set goes to @0x8d13f
/// unprovoked or with the slot inactive (@0x8cc93-0x8ccc9), as an ally
/// goal 4 spared leaves it ([`held_attack`](super::held_attack)). A
/// provoked one has taken goal 4 on it already.
#[must_use]
pub fn searches_nothing(npc: &Npc) -> bool {
    matches!(npc.goal, Goal::Idle | Goal::Land(_) | Goal::EnterGate(_)) && npc.target.is_some()
}

/// An interceptor's retreat: when outnumbered, by `Flags` 0x0100, and
/// out of ammunition, by its hull's `Flags2` 0x0080.
pub const INTERCEPTOR_RETREAT: Retreat = Retreat {
    odds_flag: INTERCEPTORS_RETREAT,
    shields: false,
    person_coward: RuleSource::Engine,
    out_of_ammo: true,
    search_gate: true,
    breaking_up: BreakingUp::LeavesToHighLevel,
};

/// Nova's interceptor (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interceptor {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// Which persons retreat at their `Coward`.
    pub person_coward: RuleSource,
    /// Whether its government's 0x0004 word reaches a blessed player
    /// ([`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks)).
    pub always_attacks: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
    /// Whether its patrol can pick an uninhabited stellar
    /// ([`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)).
    pub patrol_uninhabited: RuleSource,
    /// The rulebook's entries its target pick follows.
    pub target: TargetRules,
}

impl Behaviour for Interceptor {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// Its [`hunt`], or with nothing to fight its inspection, hold or
    /// patrol, after its out-of-ammo retreat
    /// ([`out_of_ammo::after_patrol`]), the hunt's call on the player kept
    /// through each; then an inspection of a ship it no longer sees is
    /// dropped ([`lost_sight::after_inspection`], @0x8d39a-0x8d3ee). The
    /// high level ends with its provocation a retreat with no target
    /// ([`out_of_ammo::stops`]), clearing its target; an attack on a ship
    /// breaking up meets the high level's state-4 block, which `NovaAi`
    /// runs: `_InterceptorAI` has no drop of its own ([`let_go`]), so goal
    /// 4 reaches @0x8e714 ([`attack_drop`](crate::ai::attack_drop)).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        if npc.goal == Goal::CountWait {
            // It returns for goal 0x16 at once (@0x8c8be).
            return Decision {
                goal: npc.goal,
                ..Decision::default()
            };
        }
        let retreat = Retreat {
            person_coward: self.person_coward,
            ..INTERCEPTOR_RETREAT
        };
        let hunt = hunt(
            npc,
            around,
            retreat,
            self.always_attacks,
            self.target,
            chance,
        );
        let mut comm_call = None;
        let mut stops = false;
        // The 0x0040 word wrote +0x70 -1 and goal 0 (@0x8cbc0): its gate
        // runs from there.
        let dropped;
        let patroller = if hunt.dropped_target {
            dropped = Npc {
                goal: Goal::Idle,
                target: None,
                ..npc.clone()
            };
            &dropped
        } else {
            npc
        };
        // A target its think dropped leaves no fight of its own: its
        // out-of-ammo flight still runs on the goal, with no target, and
        // stops (@0x8d343-0x8d391 → @0x8dff6 → @0x8e228).
        if let (HuntTarget::Cleared, Some(goal)) = (hunt.target, hunt.goal) {
            let let_go = Npc {
                target: None,
                ..npc.clone()
            };
            stops = out_of_ammo::stops(&let_go, goal);
        }
        let decision = hunt.or(|| {
            let (goal, call) = self.patrol(patroller, around, chance);
            comm_call = call;
            stops = out_of_ammo::stops(patroller, goal);
            out_of_ammo::after_patrol(patroller, goal)
        });
        let decision = lost_sight::after_inspection(
            npc,
            Decision {
                comm_call,
                ..decision
            },
            around,
        );
        let decision = re_pick(npc, decision, around, self.target, chance);
        let decision = reached(npc, decision, around);
        Decision {
            forgets_provocation: decision.forgets_provocation || stops,
            // A stop leaves no target: goal 3 on -1 (@0x8dff6).
            clears_target: decision.clears_target || stops,
            holds_target: decision.holds_target.filter(|_| !stops),
            ..decision
        }
    }

    /// Its weapon pick, but nothing while it docks or drifts
    /// ([`fire::quiet`]).
    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        if fire::quiet(npc) {
            return Trigger::default();
        }
        fire::trigger(npc, around, self.fire)
    }

    /// Its goal's quarry: a target it holds on a goal that names none is
    /// its think's ([`Decision::holds_target`], [`searches_nothing`]), or
    /// the high level's ([`attack_drop`](crate::ai::attack_drop)).
    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around, self.piracy_police)
    }
}

impl Interceptor {
    /// Its goal with nothing to fight: its inspection, hold or patrol;
    /// and the pre-scan hail when its pick is the player, fresh
    /// (`_InterceptorAI` @0x8ce34-0x8ce4b: the block runs only as the
    /// pick sets slot 0, never for an inspection kept).
    fn patrol(
        self,
        npc: &Npc,
        around: &Surroundings,
        chance: &mut dyn Chance,
    ) -> (Goal, Option<CommCall>) {
        if inspection_holds(npc, around) {
            return (npc.goal, None);
        }
        if gate_closed(npc, around) || searches_nothing(npc) {
            // The patrol and inspect block needs goal 1 or below, or
            // 0x14 (`_InterceptorAI` @0x8cc6d-0x8cc7d): a held ship
            // holds, a waiting one (state 6) waits, and a leaving,
            // plundering or drifting one keeps at it. Inside the gate a
            // kept +0x70 skips it too (@0x8cc93-0x8ccc9 → @0x8d13f).
            return (super::kept(npc, around).unwrap_or(npc.goal), None);
        }
        match inspect(npc, around, chance) {
            Some(goal) => {
                let call = (goal == Goal::Inspect(ShipRef::Player)).then_some(CommCall::PreScan);
                (goal, call)
            }
            None => (
                idle_block::patrol(
                    npc,
                    around,
                    chance,
                    self.patrol_uninhabited == RuleSource::Engine,
                ),
                None,
            ),
        }
    }
}

/// `decision`, an interceptor's think, through `_InterceptorAI`'s re-pick
/// while inspecting (@0x8d404-0x8d43a), last in its think: an inspection
/// (state 7, kept or fresh) by an interceptor with a government (+0x98,
/// @0x8d404) runs `_SelectWarshipTarget` again
/// ([`re_pick_target`](crate::ai::target::re_pick_target), by `rules`,
/// drawing on `chance`). With none the inspection stands (@0x8d42d). A
/// pick that writes state 4 ([`Pick::attacks`](crate::ai::target::Pick::attacks)) attacks, the
/// call on the player kept; `_HighLevelAIHandler`'s state-4 block then
/// takes it that frame ([`attack_drop`](crate::ai::attack_drop)),
/// dropping an ally, a fleet-mate or a ship breaking up and sniping at a
/// hopeless chase. The think's disabled test (@0x8d2fd-0x8d33a) needs
/// goal 4 and came before (@0x8d41e), and the block has no disabled drop,
/// so a disabled pick stands this think and the next one drops it. Any
/// other pick is inspected instead of the ship it was inspecting. The
/// retreats came before too (@0x8d161-0x8d2e4) and need state 4, so a
/// fresh attack isn't judged for one in the same think.
fn re_pick(
    npc: &Npc,
    decision: Decision,
    around: &Surroundings,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Decision {
    let Goal::Inspect(_) = decision.goal else {
        return decision;
    };
    if npc.govt.is_none() {
        return decision;
    }
    let Some(pick) = re_pick_target(npc, around, rules, chance) else {
        return decision;
    };
    Decision {
        goal: if pick.attacks {
            Goal::Attack(pick.ship)
        } else {
            Goal::Inspect(pick.ship)
        },
        turns_on_player: decision.turns_on_player || pick.turns_on_player,
        re_picked: true,
        ..decision
    }
}

/// `decision`, an interceptor's think, through goal 7's block, which runs
/// after the think in the same dispatch (@0x8f11e-0x8f394): an inspection
/// the think leaves it with, of a ship already within reach and active
/// among `around`, ends there, goal 0 with no target (@0x8f391), the ship
/// [`looked_over`](Decision::looked_over). Any hail it raised stands, as
/// the pick's hail comes before the block's scan.
fn reached(npc: &Npc, decision: Decision, around: &Surroundings) -> Decision {
    match decision.goal {
        Goal::Inspect(ship) if reach_of(npc, ship, around) == Some(true) => Decision {
            goal: Goal::Idle,
            clears_target: true,
            looked_over: Some(ship),
            ..decision
        },
        _ => decision,
    }
}

/// A ship for `npc` to inspect among `around`, picked on `chance`: the
/// player or an NPC, not itself, not another unled interceptor (see the
/// module docs), not the one it last inspected nor the one it has just
/// looked over.
fn inspect(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Option<Goal> {
    let done = [npc.inspected, npc.goal.inspecting()];
    let player = (around.active(ShipRef::Player) && around.sees(npc, ShipRef::Player))
        .then_some(ShipRef::Player);
    let npcs = around
        .npcs
        .iter()
        .filter(|other| other.id != npc.id && other.original_ai_type() != 4)
        .map(|other| ShipRef::Npc(other.id))
        .filter(|&ship| around.active(ship) && around.sees(npc, ship));
    let candidates: Vec<ShipRef> = player
        .into_iter()
        .chain(npcs)
        .filter(|ship| !done.contains(&Some(*ship)))
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let len = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    candidates
        .get(chance.below(len) as usize)
        .map(|&ship| Goal::Inspect(ship))
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::CommCall;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{
        NEUTRAL, PIRATES, POLICE, TRADERS, around, cloaked, govts, n, scanning, ship,
        short_of_ammo, sites,
    };
    use crate::ai::lost_sight::{LostSight, SEARCH_TICKS};
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::govt::{ALWAYS_ATTACKS_PLAYER, Governments, WARSHIPS_RETREAT};
    use crate::traffic::npc::{AiType, Mode};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

    const P: ShipRef = ShipRef::Player;

    /// A police interceptor (NPC 1) at the centre.
    fn police() -> Npc {
        ship(1, POLICE, AiType::Interceptor, 0.0, 0.0)
    }

    /// The interceptor's decision among `npcs` (it first), the player at
    /// `player` of record `record`, drawing `draws`; and the draws asked.
    fn decided_by(
        govts: &Governments,
        npcs: &[Npc],
        player: (f32, f32),
        record: i16,
        draws: &[u32],
    ) -> (Goal, Vec<u32>) {
        let sites = sites();
        let around = around(&sites, npcs, govts, player, record);
        let mut chance = Draws::of(draws);
        let goal = Interceptor::default().decide(&npcs[0], &around, &mut chance);
        (goal, chance.asked)
    }

    fn decided(npcs: &[Npc], record: i16, draws: &[u32]) -> Goal {
        decided_by(&govts(0, 0), npcs, (0.0, -1000.0), record, draws).0
    }

    #[test]
    fn an_interceptor_hunts_as_a_warship_does() {
        assert_eq!(decided(&[police()], -10, &[]), Goal::Attack(P));
        let mut trader_cop = police();
        trader_cop.govt = Some(TRADERS);
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(decided(&[trader_cop, pirate], 0, &[]), Goal::Attack(n(2)));
        let mut provoked = police();
        provoked.provoked = 1.0;
        provoked.target = Some(n(2));
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        assert_eq!(decided(&[provoked, neutral], 0, &[]), Goal::Attack(n(2)));
    }

    /// The interceptor (its government's `flags`) attacking a pirate of
    /// strength `foe` attacking it, at `shield` of its 30.
    fn outgunned(police_flags: u16, shield: f32, foe: f32) -> Goal {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield.now = shield;
        // The odds the scene gives, as the pass cached them.
        hunter.odds = foe / 100.0;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.hull.strength = foe;
        pirate.goal = Goal::Attack(n(1));
        decided_by(
            &govts(0, police_flags),
            &[hunter, pirate],
            (0.0, -1000.0),
            0,
            &[],
        )
        .0
    }

    #[test]
    fn its_odds_retreat_needs_0x0100_and_it_has_no_shield_retreat() {
        assert_eq!(
            outgunned(INTERCEPTORS_RETREAT, 14.9, 101.0),
            Goal::Flee(n(2))
        );
        assert_eq!(
            outgunned(INTERCEPTORS_RETREAT, 15.0, 101.0),
            Goal::Attack(n(2))
        );
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 0.0, 101.0),
            Goal::Attack(n(2)),
            "not 0x0010"
        );
        assert_eq!(
            outgunned(WARSHIPS_RETREAT | INTERCEPTORS_RETREAT, 0.0, 50.0),
            Goal::Attack(n(2)),
            "no shield retreat, though a warship would at aggression 2"
        );
    }

    #[test]
    fn a_held_interceptor_inspects_nothing() {
        let mut held = police();
        held.goal = Goal::Hold;
        let npcs = [held, ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0)];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[]);
        assert_eq!(goal, Goal::Hold);
        assert!(asked.is_empty(), "no pick");
    }

    #[test]
    fn idle_it_inspects_a_ship_picked_at_random_among_the_others() {
        let mut other_cop = ship(3, POLICE, AiType::Interceptor, 10.0, 0.0);
        other_cop.goal = Goal::Idle;
        let npcs = [
            police(),
            ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0),
            other_cop,
            ship(4, NEUTRAL, AiType::Warship, 900.0, 0.0),
        ];
        let govts = govts(0, 0);
        for (draw, inspected) in [(0, P), (1, n(2)), (2, n(4))] {
            let (goal, asked) = decided_by(&govts, &npcs, (0.0, -1000.0), 0, &[draw]);
            assert_eq!(goal, Goal::Inspect(inspected), "{draw}");
            assert_eq!(
                asked,
                [3],
                "the player and two NPCs; not another interceptor"
            );
        }
    }

    /// What [`police`] decides with `other` alone in the system, its draw
    /// falling on the first candidate.
    fn inspects_alone(other: Npc) -> Goal {
        let npcs = [police(), other];
        let (sites, govts) = (sites(), govts(0, 0));
        let mut around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        around.player = None;
        Interceptor::default().decide(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    #[test]
    fn it_may_inspect_a_ship_gliding_in_or_breaking_up_but_not_one_in_its_gate() {
        // The walk reads the slot and `_ShipVisibleToShip` (@0x8cd3c-0x8cd5f,
        // @0x8cde1-0x8ce13), and calls no `_IsDying`.
        let trader = || ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
        let with = |mode| Npc { mode, ..trader() };
        for mode in [Mode::JumpingIn { tick: 1 }, Mode::LeavingGate { tick: 1 }] {
            assert_eq!(inspects_alone(with(mode)), Goal::Inspect(n(2)), "{mode:?}");
        }
        let dying = Npc {
            condition: Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            ..trader()
        };
        assert_eq!(inspects_alone(dying), Goal::Inspect(n(2)));
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::LeavingGate { tick: 0 },
        ] {
            assert_ne!(inspects_alone(with(mode)), Goal::Inspect(n(2)), "{mode:?}");
        }
    }

    #[test]
    fn it_may_inspect_a_player_breaking_up() {
        let npcs = [police()];
        let (sites, govts) = (sites(), govts(0, 0));
        let mut around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let player = around.player.expect("in the system");
        around.player = Some(crate::ai::PlayerSide {
            condition: Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
            ..player
        });
        let goal = Interceptor::default().decide(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(goal, Goal::Inspect(P));
    }

    #[test]
    fn its_inspection_holds_on_a_ship_gliding_in_or_breaking_up_and_ends_on_one_destroyed() {
        // State 7 reads the slot alone (@0x8f158).
        let at = |target: &dyn Fn(&mut Npc)| {
            let cop = Npc {
                goal: Goal::Inspect(n(2)),
                ..police()
            };
            let mut trader = ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
            target(&mut trader);
            let npcs = [cop, trader];
            let (sites, govts) = (sites(), govts(0, 0));
            let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            inspection_holds(&npcs[0], &around)
        };
        assert!(at(&|_| {}), "flying");
        assert!(at(&|other| other.mode = Mode::JumpingIn { tick: 1 }));
        assert!(at(&|other| {
            other.condition = Condition::Dying {
                ticks_left: 3,
                starting: false,
            };
        }));
        assert!(!at(&|other| other.condition = Condition::Destroyed));
    }

    /// An interceptor provoked into attacking NPC 2, as `npc` changes it.
    fn on_the_wreck(npc: impl Fn(&mut Npc)) -> Npc {
        let mut cop = police();
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        cop.provoked = 5.0;
        npc(&mut cop);
        cop
    }

    /// NPC 2, a pirate 100 off, breaking up.
    fn wreck() -> Npc {
        let mut wreck = ship(2, PIRATES, AiType::Warship, 100.0, 0.0);
        wreck.condition = Condition::Dying {
            ticks_left: 10,
            starting: false,
        };
        wreck
    }

    /// `npcs[0]`'s decision by Nova's AI, the high level after its think,
    /// the player far off and as `condition` says, the police's `flags`
    /// as given.
    fn high_level(flags: u16, npcs: &[Npc], condition: Option<Condition>) -> Decision {
        let (sites, govts) = (sites(), govts(0, flags));
        let mut around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
        if let (Some(player), Some(condition)) = (around.player.as_mut(), condition) {
            player.condition = condition;
        }
        crate::ai::NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0; 16]))
    }

    #[test]
    fn its_think_has_no_drop_of_a_ship_breaking_up_and_the_block_ends_it() {
        // `_InterceptorAI` calls `_IsDying` on the player's slot alone
        // (@0x8c906), so goal 4 reaches the block (@0x8e3ef → @0x8e714).
        let npcs = [on_the_wreck(|_| {}), wreck()];
        let (sites, govts) = (sites(), govts(0, 0));
        let around = around(&sites, &npcs, &govts, (0.0, -5000.0), 0);
        let mut chance = Draws::of(&[0; 16]);
        assert_eq!(
            Interceptor::default().decision(&npcs[0], &around, &mut chance),
            Decision {
                goal: Goal::Attack(n(2)),
                ..Decision::default()
            },
            "the think keeps goal 4"
        );
        assert_eq!(
            high_level(0, &npcs, None),
            Decision {
                goal: Goal::Idle,
                clears_target: true,
                forgets_provocation: true,
                lost_sight: Some(LostSight::Seen),
                ..Decision::default()
            },
            "@0x8e717-0x8e726, then sub 0 at the tail (@0x8fb13)"
        );
    }

    #[test]
    fn its_fresh_pick_of_a_ship_breaking_up_ends_in_the_block_with_no_inspection() {
        // The pick writes goal 4 (@0x8cd02) and the block drops it the same
        // frame, so no inspection is picked. Only a ship with destroying
        // weapons has it for a candidate.
        let trader_cop = Npc {
            govt: Some(TRADERS),
            armament: Armament::new([(
                WeaponSpec::new(&WeaponRecord {
                    mass_dmg: 1,
                    ..weapon(128)
                }),
                1,
            )]),
            ..police()
        };
        let decision = high_level(0, &[trader_cop, wreck()], None);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target, "@0x8e717");
    }

    #[test]
    fn an_interceptor_whose_wait_runs_keeps_its_attack_on_a_ship_breaking_up() {
        // The handler jumps out at its head while +0x4c runs (@0x8d4b9
        // -0x8d4d2 → 0x8fb47), the think having run first (@0x901e5).
        let waiting = on_the_wreck(|cop| cop.ai_wait = 3);
        let held = on_the_wreck(|cop| cop.mode = Mode::HeldAtGate { ticks_left: 20 });
        for npc in [waiting, held] {
            let decision = high_level(0, &[npc, wreck()], None);
            assert_eq!(decision.goal, Goal::Attack(n(2)), "goal 4");
            assert!(!decision.forgets_provocation, "+0x96 stands");
            assert!(!decision.clears_target, "+0x70 stands");
        }
    }

    #[test]
    fn an_interceptor_that_cannot_see_a_ship_breaking_up_keeps_its_provocation_and_searches() {
        // The lost-sight block ends the handler first (@0x8d85a, @0x8d8a6),
        // so the state-4 block never runs.
        let decision = high_level(0, &[on_the_wreck(|_| {}), cloaked(wreck())], None);
        assert_eq!(decision.goal, Goal::Attack(n(2)), "goal 4");
        assert_eq!(
            decision.lost_sight,
            Some(LostSight::Searching(SEARCH_TICKS)),
            "@0x8d83b-0x8d852"
        );
        assert!(!decision.forgets_provocation, "+0x96 stands");
        assert!(!decision.clears_target, "+0x70 stands");
    }

    #[test]
    fn its_flight_from_a_ship_breaking_up_keeps_its_provocation() {
        // State 3 isn't the block's: only @0x8e3ef's state 4 ends a fight
        // with +0x96.
        let flees = on_the_wreck(|cop| cop.goal = Goal::Flee(n(2)));
        let decision = high_level(0, &[flees, wreck()], None);
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert!(!decision.forgets_provocation);
    }

    #[test]
    fn its_word_on_a_player_breaking_up_ends_in_the_block_with_no_provocation() {
        // The word writes target 0 and +0x96 1 (@0x8cb97, @0x8cb9d), and the
        // block then writes its 0 over them (@0x8e71d): it has no drop of
        // its own, as `_WarshipAI` has (@0x8bc58-0x8bca9).
        let dying = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let decision = high_level(ALWAYS_ATTACKS_PLAYER, &[police()], Some(dying));
        assert_eq!(decision.goal, Goal::Idle, "@0x8e726");
        assert!(decision.turns_on_player, "the call came first (@0x8cb8f)");
        assert!(decision.clears_target, "@0x8e717");
        assert!(decision.forgets_provocation, "@0x8e71d");
        assert!(!decision.provokes, "the block's 0 came after the word's 1");
    }

    #[test]
    fn its_re_pick_of_a_ship_breaking_up_ends_in_the_block() {
        // The re-pick is last in the think (@0x8d41e), and the block takes
        // its goal 4 that frame (@0x8e3ef). The help pick writes state 4.
        let (mut cop, trader) = trader_cop_inspecting();
        cop.govt = Some(POLICE);
        let mut fleeing = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 200.0);
        fleeing.goal = Goal::Flee(n(4));
        let wrecked = Npc {
            id: crate::traffic::npc::NpcId(4),
            ..wreck()
        };
        let others = [trader, fleeing, wrecked];
        let (think, _) = re_picked(cop.clone(), &others, 0, None, &[]);
        assert_eq!(think.goal, Goal::Attack(n(4)), "goal 4 for the block");
        assert!(think.re_picked);
        let npcs: Vec<Npc> = std::iter::once(cop).chain(others).collect();
        let dropped = high_level(0, &npcs, None);
        assert_eq!(dropped.goal, Goal::Idle, "@0x8e726");
        assert!(dropped.forgets_provocation, "@0x8e71d");
    }

    #[test]
    fn idle_it_never_picks_a_cloaked_ship_it_cant_see_to_inspect() {
        let npcs = [
            police(),
            cloaked(ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0)),
            ship(3, TRADERS, AiType::WimpyTrader, 900.0, 0.0),
        ];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[1]);
        assert_eq!(
            (goal, asked),
            (Goal::Inspect(n(3)), vec![2]),
            "the player and the trader it sees"
        );
        let sites = sites();
        let govts = govts(0, 0);
        let alone = &npcs[..2];
        let around = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(&sites, alone)
        };
        assert_eq!(
            Interceptor::default().decide(&alone[0], &around, &mut Draws::of(&[0])),
            Goal::Land(StellarId(128)),
            "nothing it sees: it patrols"
        );
    }

    #[test]
    fn it_inspects_a_cloaked_ship_its_scanner_sees() {
        let npcs = [
            scanning(police()),
            cloaked(ship(2, TRADERS, AiType::WimpyTrader, 150.0, 0.0)),
        ];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[1]);
        assert_eq!((goal, asked), (Goal::Inspect(n(2)), vec![2]));
    }

    #[test]
    fn it_may_inspect_a_ship_with_no_ai_whatever_its_class() {
        let mut hulk = ship(2, POLICE, AiType::Interceptor, 500.0, 0.0);
        hulk.no_ai = true;
        hulk.govt = None;
        hulk.condition = crate::combat::hull::Condition::Disabled;
        let npcs = [police(), hulk];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[1]);
        assert_eq!(
            (goal, asked),
            (Goal::Inspect(n(2)), vec![2]),
            "AI type -1 is not 4 (@0x8cd4e)"
        );
    }

    #[test]
    fn it_may_inspect_a_led_or_marked_interceptor_but_not_an_unled_one() {
        // A led ship is the original's AI type 6, not 4 (@0x8cd4e).
        let mut led = ship(2, POLICE, AiType::Interceptor, 500.0, 0.0);
        led.leader = Some(crate::traffic::npc::NpcId(9));
        let mut marked = ship(2, POLICE, AiType::Interceptor, 500.0, 0.0);
        marked.kept_escort_ai = true;
        marked.condition = crate::combat::hull::Condition::Disabled;
        for other in [led, marked] {
            let npcs = [police(), other];
            let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[1]);
            assert_eq!(
                (goal, asked),
                (Goal::Inspect(n(2)), vec![2]),
                "{:?}",
                npcs[1]
            );
        }
        let npcs = [police(), ship(2, POLICE, AiType::Interceptor, 500.0, 0.0)];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[0]);
        assert_eq!((goal, asked), (Goal::Inspect(P), vec![1]), "unled");
    }

    #[test]
    fn it_never_inspects_the_ship_it_last_inspected() {
        let mut cop = police();
        cop.inspected = Some(n(2));
        let npcs = [cop, ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0)];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[0]);
        assert_eq!((goal, asked), (Goal::Inspect(P), vec![1]));
    }

    #[test]
    fn it_closes_in_until_within_100_on_both_axes_and_then_decides_again() {
        let mut cop = police();
        cop.goal = Goal::Inspect(n(2));
        let far = ship(2, TRADERS, AiType::WimpyTrader, 100.0, 100.1);
        let npcs = [cop.clone(), far];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[]);
        assert_eq!((goal, asked), (Goal::Inspect(n(2)), vec![]), "kept");
        for (x, y) in [(-100.0, 100.0), (10.0, -50.0), (-50.0, 10.0)] {
            let near = ship(2, TRADERS, AiType::WimpyTrader, x, y);
            let npcs = [cop.clone(), near];
            let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[0]);
            assert_eq!(
                (goal, asked),
                (Goal::Inspect(P), vec![1]),
                "looked it over at ({x}, {y}): on to another, not it again"
            );
        }
        let mut aside = cop.clone();
        aside.state.position = Vec2::new(500.0, 500.0);
        let beside = ship(2, TRADERS, AiType::WimpyTrader, 450.0, 450.0);
        let (goal, _) = decided_by(
            &govts(0, 0),
            &[aside.clone(), beside],
            (0.0, -1000.0),
            0,
            &[0],
        );
        assert_eq!(goal, Goal::Inspect(P), "measured from where it is");
        let off = ship(2, TRADERS, AiType::WimpyTrader, 0.0, 0.0);
        let (goal, _) = decided_by(&govts(0, 0), &[aside, off], (0.0, -1000.0), 0, &[0]);
        assert_eq!(goal, Goal::Inspect(n(2)), "500 off: kept");
        let (goal, _) = decided_by(&govts(0, 0), &[cop], (0.0, -1000.0), 0, &[0]);
        assert_eq!(goal, Goal::Inspect(P), "it has gone");
        assert_eq!(INSPECT_REACH, 100.0);
    }

    #[test]
    fn with_nothing_to_inspect_it_parks_at_a_stellar() {
        let sites = sites();
        let govts = govts(0, 0);
        let npcs = [police()];
        let around = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(&sites, &npcs)
        };
        assert_eq!(
            Interceptor::default().decide(&npcs[0], &around, &mut Draws::of(&[0])),
            Goal::Land(StellarId(128))
        );
    }

    /// A lone interceptor's idle pick among `sites`, by `patrol_uninhabited`,
    /// drawing `draw`; and the draws asked.
    fn patrols(
        sites: &[crate::catalog::LandingSite],
        patrol_uninhabited: RuleSource,
        draw: u32,
    ) -> (Goal, Vec<u32>) {
        let govts = govts(0, 0);
        let npcs = [police()];
        let around = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(sites, &npcs)
        };
        let interceptor = Interceptor {
            patrol_uninhabited,
            ..Interceptor::default()
        };
        let mut chance = Draws::of(&[draw]);
        let goal = interceptor.decide(&npcs[0], &around, &mut chance);
        (goal, chance.asked)
    }

    #[test]
    fn its_patrol_never_picks_a_gate() {
        let gate = crate::catalog::LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..nova_sim_catalog::fixture::planet(129, 0.0, 0.0)
        };
        let sites = [gate, nova_sim_catalog::fixture::planet(128, 0.0, 0.0)];
        for rule in RuleSource::ALL {
            assert_eq!(
                patrols(&sites, rule, 0),
                (Goal::Land(StellarId(128)), vec![1]),
                "{rule:?}"
            );
        }
    }

    #[test]
    fn its_patrol_picks_an_uninhabited_stellar_as_its_rule_says() {
        let empty = crate::catalog::LandingSite {
            flags: crate::landing::StellarFlags::CAN_LAND
                | crate::landing::StellarFlags::UNINHABITED,
            ..nova_sim_catalog::fixture::planet(129, 0.0, 0.0)
        };
        let sites = [empty, nova_sim_catalog::fixture::planet(128, 0.0, 0.0)];
        assert_eq!(
            Interceptor::default().patrol_uninhabited,
            RuleSource::Engine
        );
        assert_eq!(
            patrols(&sites, RuleSource::Engine, 0),
            (Goal::Land(StellarId(129)), vec![2])
        );
        assert_eq!(
            patrols(&sites, RuleSource::Bible, 0),
            (Goal::Land(StellarId(128)), vec![1])
        );
    }

    /// A lone interceptor, as `npc`, deciding among `sites`; and the draws
    /// asked.
    fn lone(npc: Npc, sites: &[crate::catalog::LandingSite], draws: &[u32]) -> (Goal, Vec<u32>) {
        let govts = govts(0, 0);
        let npcs = [npc];
        let around = crate::ai::Surroundings {
            govts: &govts,
            ..crate::ai::Surroundings::new(sites, &npcs)
        };
        let mut chance = Draws::of(draws);
        let goal = Interceptor::default().decide(&npcs[0], &around, &mut chance);
        (goal, chance.asked)
    }

    #[test]
    fn an_interceptor_patrols_even_when_it_has_visited() {
        let gate = crate::catalog::LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..nova_sim_catalog::fixture::planet(129, 0.0, 0.0)
        };
        let sites = [gate, nova_sim_catalog::fixture::planet(128, 0.0, 0.0)];
        for visited in [128, 129] {
            let cop = Npc {
                visited: Some(StellarId(visited)),
                ..police()
            };
            assert_eq!(
                lone(cop, &sites, &[0]),
                (Goal::Land(StellarId(128)), vec![1]),
                "no visited test (@0x8d00d): {visited}"
            );
        }
    }

    #[test]
    fn a_destroyed_patrol_pick_makes_it_wait() {
        let ruin = crate::catalog::LandingSite {
            flags: crate::landing::StellarFlags::CAN_LAND
                | crate::landing::StellarFlags::ONLY_WHEN_DESTROYED,
            destroyed: true,
            ..nova_sim_catalog::fixture::planet(128, 0.0, 0.0)
        };
        assert_eq!(lone(police(), &[ruin], &[0]), (Goal::Wait, vec![1]));
    }

    #[test]
    fn no_patrol_pick_leaves_or_waits() {
        assert_eq!(lone(police(), &[], &[]), (Goal::JumpOut, vec![]));
        let stranded = Npc {
            class_fuel: 99,
            ..police()
        };
        assert_eq!(lone(stranded, &[], &[]), (Goal::Wait, vec![]));
    }

    #[test]
    fn a_waiting_interceptor_inspects_and_searches_nothing() {
        let mut waiting = police();
        waiting.goal = Goal::Wait;
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 300.0);
        let mut trader_cop = waiting.clone();
        trader_cop.govt = Some(TRADERS);
        for npcs in [
            vec![waiting, ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0)],
            vec![trader_cop, pirate],
        ] {
            let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), -10, &[0]);
            assert_eq!(goal, Goal::Wait, "{:?}", npcs[0].govt);
            assert!(asked.is_empty(), "no pick");
        }
    }

    /// The plunder of the trader, NPC 2.
    const PLUNDER: Goal = Goal::Plunder {
        ship: ShipRef::Npc(crate::traffic::npc::NpcId(2)),
        hopeless: false,
    };

    #[test]
    fn an_unprovoked_interceptor_holding_a_plunder_a_jump_out_or_a_drift_keeps_it_by_a_hostile_player()
     {
        // The search, inspection and patrol run for goal 1 or below, or
        // 0x14, alone (@0x8cc6d-0x8cc7d): a held goal 0xd, 2 or 0xe is
        // kept, the player in view of record -10 though it is.
        for goal in [PLUNDER, Goal::JumpOut, Goal::Drift] {
            let mut cop = police();
            cop.goal = goal;
            cop.target = goal.quarry();
            let npcs = [cop, ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0)];
            let (decided, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -300.0), -10, &[0]);
            assert_eq!(decided, goal, "{goal:?}");
            assert!(asked.is_empty(), "{goal:?}: no pick");
        }
    }

    #[test]
    fn the_gate_closes_on_every_goal_above_1_but_0x14_and_opens_on_the_rest() {
        // The search, inspection and patrol run for goal 1 or below, or
        // 0x14, alone (@0x8cc6d-0x8cc7d). The engine's follow and
        // formation stay open too; an inspection's hold has its own tests.
        let closed = [
            Goal::Wait,
            Goal::Hold,
            Goal::JumpOut,
            PLUNDER,
            Goal::Drift,
            Goal::CountWait,
            Goal::Mine,
            Goal::Scoop,
            Goal::AttackStellar(StellarId(128)),
            Goal::Dock(P),
            Goal::Assist(crate::ai::Help::Refuel),
            Goal::Regroup {
                kept: crate::ai::regroup::Kept::Coast,
            },
            Goal::Flee(P),
        ];
        let open = [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::Attack(P),
            Goal::Snipe(P),
            Goal::Follow {
                lead: crate::traffic::npc::NpcId(2),
                guard: None,
            },
            Goal::Formation { guard: None },
        ];
        let sites = sites();
        let govts = govts(0, 0);
        for (goals, shut) in [(&closed[..], true), (&open[..], false)] {
            for &goal in goals {
                let mut cop = police();
                cop.goal = goal;
                let npcs = [cop];
                let around = around(&sites, &npcs, &govts, (0.0, -300.0), -10);
                assert_eq!(gate_closed(&npcs[0], &around), shut, "{goal:?}");
            }
        }
    }

    #[test]
    fn a_fleeing_interceptor_searches_nothing_behind_its_gate() {
        // Its provoked branch skips goal 3 (@0x8cc07) and its gate closes
        // for it (@0x8cc6d-0x8cc7d → @0x8d13f): fleeing a ship breaking
        // up or gone, with a pirate in view the search would take, it
        // flees on, and goal 3's block ends a flight from a ship gone
        // (`flee_drop`).
        let mut runner = police();
        runner.goal = Goal::Flee(n(2));
        runner.target = Some(n(2));
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let other = ship(3, PIRATES, AiType::Warship, 0.0, 400.0);
        for npcs in [
            vec![runner.clone(), dying, other.clone()],
            vec![runner.clone(), other],
        ] {
            let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[0]);
            assert_eq!(goal, Goal::Flee(n(2)), "{} ships", npcs.len());
            assert!(asked.is_empty(), "no search");
        }
    }

    #[test]
    fn a_provoked_interceptor_holding_a_plunder_attacks_its_provoker_or_drops_one_it_cant_destroy()
    {
        // The provoked branch writes goal 4 (@0x8cbf3-0x8cc31), the gate
        // then skips the search (@0x8cc6d), and the disabled drop takes a
        // disabled target with no destroying weapon: goal 0, +0x70 -1
        // (@0x8d2fa-0x8d33a).
        let mut cop = police();
        cop.goal = PLUNDER;
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let sites = sites();
        let govts = govts(0, 0);
        let npcs = [cop.clone(), pirate.clone()];
        let around = around(&sites, &npcs, &govts, (0.0, -300.0), -10);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        let mut ally = npcs.clone();
        ally[1].govt = Some(TRADERS);
        let around = crate::ai::fixture::around(&sites, &ally, &govts, (0.0, -300.0), -10);
        let decision = Interceptor::default().decision(&ally[0], &around, &mut Draws::of(&[0]));
        assert_eq!(
            decision.goal,
            Goal::Attack(n(2)),
            "the think spares no ally"
        );
        let decision =
            crate::ai::NovaAi::default().decision(&ally[0], &around, &mut Draws::of(&[0]));
        assert_eq!(
            (decision.goal, decision.clears_target),
            (Goal::Idle, false),
            "an ally the state-4 block spares, +0x70 kept (@0x8e277-0x8e2db)"
        );
        pirate.condition = Condition::Disabled;
        let npcs = [cop, pirate];
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -300.0), -10);
        let mut chance = Draws::of(&[0]);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut chance);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
        assert!(chance.asked.is_empty(), "no search that think");
    }

    #[test]
    fn a_provoked_interceptor_holding_a_plunder_ends_its_attack_on_a_ship_breaking_up_forgetting_it()
     {
        // Goal 4 on a ship breaking up: goal 0, +0x70 -1, +0x96 0
        // (@0x8e3ef → @0x8e714-0x8e726), with no search that think.
        let mut cop = police();
        cop.goal = PLUNDER;
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let npcs = [cop, pirate];
        let sites = sites();
        let govts = govts(0, 0);
        let around = around(&sites, &npcs, &govts, (0.0, -300.0), -10);
        let mut chance = Draws::of(&[0]);
        let decision = crate::ai::NovaAi::default().decision(&npcs[0], &around, &mut chance);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
        assert!(decision.forgets_provocation);
        assert!(chance.asked.is_empty(), "no search that think");
    }

    #[test]
    fn a_provoked_interceptor_holding_a_plunder_is_judged_by_its_new_goal_4_for_the_odds_retreat() {
        // The provoked branch writes goal 4 before the retreats, which need
        // it (@0x8cbf3-0x8cc31, @0x8d14d-0x8d2e4): outnumbered, it flees.
        let mut cop = police();
        cop.goal = PLUNDER;
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        cop.reserves.shield.now = 1.0;
        cop.odds = 5.0;
        let npcs = [cop, ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        let sites = sites();
        let govts = govts(0, INTERCEPTORS_RETREAT);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
    }

    #[test]
    fn a_dry_interceptor_of_flags2_0x0080_holding_a_plunder_flees_its_ship() {
        // Any goal but 2, 3 and 0xb becomes goal 3 on +0x70 (@0x8d343-0x8d391).
        let mut cop = short_of_ammo(police(), AmmoLeft::Dry, true);
        cop.goal = PLUNDER;
        cop.target = Some(n(2));
        let npcs = [cop, ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0)];
        let sites = sites();
        let govts = govts(0, 0);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert!(!decision.forgets_provocation, "it flees a ship");
    }

    #[test]
    fn an_interceptor_in_its_count_wait_keeps_it_provoked_or_not() {
        // It returns for goal 0x16 at once (@0x8c8be).
        let mut cop = police();
        cop.goal = Goal::CountWait;
        let npcs = [cop.clone(), ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -300.0), -10, &[0]);
        assert_eq!(goal, Goal::CountWait);
        assert!(asked.is_empty());
        cop.provoked = 2.0;
        cop.target = Some(n(2));
        let npcs = [cop, ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        assert_eq!(decided(&npcs, -10, &[0]), Goal::CountWait, "provoked");
    }

    #[test]
    fn a_provoked_waiting_interceptor_attacks() {
        let mut waiting = police();
        waiting.goal = Goal::Wait;
        waiting.provoked = 1.0;
        waiting.target = Some(n(2));
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        assert_eq!(decided(&[waiting, neutral], 0, &[]), Goal::Attack(n(2)));
    }

    /// The decision of an interceptor provoked by NPC 2 while in `goal`,
    /// NPC 2 being `provoker`, the player in view of record -10, through the
    /// high level's slot test and state-4 block ([`attacking_led`]); and the
    /// draws asked.
    fn provoked_in(goal: Goal, provoker: Npc) -> (Decision, Vec<u32>) {
        let mut cop = police();
        cop.goal = goal;
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        attacking_with(Interceptor::default(), cop, Some(provoker))
    }

    #[test]
    fn a_provoked_waiting_or_held_interceptor_whose_provoker_it_cant_destroy_goes_idle() {
        // The provoked branch writes goal 4 over a wait (6) or a hold
        // (0xa), excluding only goals 3, 7, 9 and 0xf (@0x8cbf6-0x8cc31);
        // the gate then skips the search for goal 4 (@0x8cc6d-0x8cc7d), and
        // the disabled drop takes a disabled target with no destroying
        // weapon: goal 0, +0x70 -1 (@0x8d2fa-0x8d33a).
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Disabled;
        for goal in [Goal::Wait, Goal::Hold] {
            let (decision, asked) = provoked_in(goal, pirate.clone());
            assert_eq!(decision.goal, Goal::Idle, "{goal:?}");
            assert!(decision.clears_target, "{goal:?}");
            assert!(asked.is_empty(), "{goal:?}: no search that think");
        }
    }

    #[test]
    fn a_provoked_waiting_or_held_interceptor_whose_provoker_it_spares_goes_idle() {
        // Goal 4, written over the wait or hold (@0x8cbf6-0x8cc31), meets
        // the ally drop: goal 0 (@0x8e277-0x8e2d2), with no search that
        // think (@0x8cc6d-0x8cc7d).
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        for goal in [Goal::Wait, Goal::Hold] {
            let (decision, asked) = provoked_in(goal, ally.clone());
            assert_eq!(decision.goal, Goal::Idle, "{goal:?}");
            assert!(!decision.clears_target, "{goal:?}: +0x70 kept (@0x8e2d2)");
            assert!(asked.is_empty(), "{goal:?}: no search that think");
        }
    }

    /// The decision of `cop` (unprovoked unless set) attacking NPC 2,
    /// `quarry` (none for a ship gone), by `cop_ai`, a hostile player in
    /// view (record -10, 300 below), then the high level's slot test and
    /// state-4 block by `led_ally`
    /// ([`attack_drop`](crate::ai::attack_drop)); and the draws asked.
    fn attacking_led(
        cop_ai: Interceptor,
        cop: Npc,
        quarry: Option<Npc>,
        led_ally: RuleSource,
    ) -> (Decision, Vec<u32>) {
        let npcs: Vec<Npc> = std::iter::once(cop).chain(quarry).collect();
        let sites = sites();
        let govts = govts(0, 0);
        let around = around(&sites, &npcs, &govts, (0.0, -300.0), -10);
        let mut chance = Draws::of(&[0]);
        let decision = cop_ai.decision(&npcs[0], &around, &mut chance);
        let decision = crate::ai::handler_head::slot_test(decision, &around);
        let decision = crate::ai::attack_drop::attack_block(&npcs[0], decision, &around, led_ally);
        (decision, chance.asked)
    }

    /// [`attacking_led`] by the engine's reading.
    fn attacking_with(cop_ai: Interceptor, cop: Npc, quarry: Option<Npc>) -> (Decision, Vec<u32>) {
        attacking_led(cop_ai, cop, quarry, RuleSource::Engine)
    }

    fn attacking(quarry: Option<Npc>) -> (Decision, Vec<u32>) {
        let mut cop = police();
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        attacking_with(Interceptor::default(), cop, quarry)
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    #[test]
    fn an_unprovoked_interceptor_whose_attack_the_disabled_drop_ends_searches_nothing_that_think() {
        // Goal 4 closes the gate (@0x8cc6d-0x8cc7d); the disabled drop
        // (@0x8d2fa-0x8d33a) writes goal 0 and +0x70 -1.
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Disabled;
        let (decision, asked) = attacking(Some(pirate));
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
        assert!(asked.is_empty(), "no search that think");
    }

    #[test]
    fn an_unprovoked_interceptor_attacking_an_ally_it_spares_idles_keeping_it_as_its_target() {
        // @0x8e277-0x8e2db: goal 0, sub 0, +0x70 kept.
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            let mut cop = police();
            cop.goal = Goal::Attack(n(2));
            cop.target = Some(n(2));
            let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
            let (decision, asked) =
                attacking_led(Interceptor::default(), cop, Some(ally), led_ally);
            assert_eq!(
                (decision.goal, decision.clears_target, decision.holds_target),
                (Goal::Idle, false, Some(n(2))),
                "{led_ally:?}"
            );
            assert!(asked.is_empty(), "{led_ally:?}: no search");
        }
    }

    /// The decision of a provoked `cop` attacking NPC 2, `quarry`.
    fn provoked_attacking(quarry: Npc) -> (Decision, Vec<u32>) {
        let mut cop = police();
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        attacking_with(Interceptor::default(), cop, Some(quarry))
    }

    #[test]
    fn a_provoked_interceptor_attacking_a_dying_ally_idles_keeping_its_provocation() {
        // The ally drop (@0x8e277-0x8e2e4) comes before the breaking-up
        // test (@0x8e3ef) and returns with goal 0, +0x70 and +0x96 kept
        // (@0x8e2d2 → @0x8fb47).
        let mut ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        ally.condition = DYING;
        let (decision, asked) = provoked_attacking(ally);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, false, false)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn a_provoked_interceptor_attacking_a_dying_escort_of_its_fleet_idles_keeping_its_provocation()
    {
        // The fleet drop (@0x8e2e9-0x8e399) comes before the breaking-up
        // test (@0x8e3ef) and returns with goal 0, +0x70 -1 and +0x96 kept
        // (@0x8e381 → @0x8fb47): here a fellow follower of NPC 3 (its parent
        // is the target's, @0x8e322).
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(crate::traffic::npc::NpcId(3));
        escort.condition = DYING;
        let mut cop = police();
        cop.leader = Some(crate::traffic::npc::NpcId(3));
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        let (decision, asked) = attacking_with(Interceptor::default(), cop, Some(escort));
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, false)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn an_unprovoked_interceptor_attacking_a_ship_breaking_up_idles_forgetting_its_provocation() {
        // @0x8e3ef → @0x8e714-0x8e726.
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = DYING;
        let (decision, asked) = attacking(Some(pirate));
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, true)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn an_unprovoked_interceptor_attacking_one_of_its_fleet_idles_clearing_its_target() {
        // @0x8e2e9-0x8e399 → @0x8e381, a fellow follower of NPC 3.
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(crate::traffic::npc::NpcId(3));
        let mut cop = police();
        cop.leader = Some(crate::traffic::npc::NpcId(3));
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        let (decision, asked) = attacking_with(Interceptor::default(), cop, Some(escort));
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, false)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn an_unprovoked_interceptor_attacking_a_ship_gone_idles() {
        // The slot test makes +0x70 -1 (@0x8d4e4-0x8d50d), goal 4 then 0
        // (@0x8e263 → @0x8e731), +0x96 kept.
        let (decision, asked) = attacking(None);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, false)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn a_dry_interceptor_whose_disabled_drop_ends_its_attack_stops_forgetting_its_provocation() {
        // The disabled drop makes +0x70 -1, the out-of-ammo retreat then
        // goal 3 on it (@0x8d343-0x8d391), which the high level stops
        // (@0x8dff6 → @0x8e228: goal 0, +0x96 0).
        let mut cop = short_of_ammo(police(), AmmoLeft::Dry, true);
        cop.goal = Goal::Attack(n(2));
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Disabled;
        let (decision, asked) = attacking_with(Interceptor::default(), cop, Some(pirate));
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, true)
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn a_provoked_idle_interceptor_spared_its_provoker_idles_with_it_as_its_target_and_no_search() {
        // The provoked branch writes goal 4 on +0x70 (@0x8cbf6-0x8cc31)
        // and the ally drop idles it again, keeping +0x70 (@0x8e2d2).
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        let (decision, asked) = provoked_in(Goal::Idle, ally);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, false, false)
        );
        assert!(asked.is_empty(), "no search, inspection or patrol");
    }

    #[test]
    fn a_provoked_idle_interceptor_is_judged_by_its_new_goal_4_for_the_odds_retreat() {
        // The provoked branch writes goal 4 before the retreats, which need
        // it (@0x8cbf6-0x8cc31, @0x8d14d-0x8d2e4).
        let mut cop = police();
        cop.goal = Goal::Idle;
        cop.target = Some(n(2));
        cop.provoked = 3.0;
        cop.reserves.shield.now = 1.0;
        cop.odds = 5.0;
        let npcs = [cop, ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        let sites = sites();
        let govts = govts(0, INTERCEPTORS_RETREAT);
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
    }

    #[test]
    fn an_unprovoked_idle_interceptor_with_a_kept_target_searches_inspects_and_patrols_nothing() {
        // +0x70 set inside the open gate, unprovoked: @0x8cc93-0x8ccc9 →
        // @0x8d13f, past the search, the inspection and the patrol.
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
        ] {
            let mut cop = police();
            cop.goal = goal;
            cop.target = Some(n(2));
            let ally = ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
            let (decision, asked) = attacking_with(Interceptor::default(), cop, Some(ally));
            assert_eq!(
                (decision.goal, decision.clears_target),
                (goal, false),
                "{goal:?}"
            );
            assert_eq!(decision.comm_call, None, "{goal:?}: no pre-scan hail");
            assert!(asked.is_empty(), "{goal:?}: no search or pick");
        }
    }

    #[test]
    fn a_dry_unprovoked_idle_interceptor_with_a_kept_target_flees_it() {
        // The out-of-ammo retreat still runs, goal 3 on +0x70
        // (@0x8d343-0x8d391).
        let mut cop = short_of_ammo(police(), AmmoLeft::Dry, true);
        cop.goal = Goal::Idle;
        cop.target = Some(n(2));
        let ally = ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
        let (decision, asked) = attacking_with(Interceptor::default(), cop, Some(ally));
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert!(!decision.forgets_provocation, "it flees a ship");
        assert!(asked.is_empty());
    }

    #[test]
    fn an_idle_interceptors_frame_ends_with_the_target_it_kept() {
        // Its think holds it (`Decision::holds_target`); the target step is
        // its goal's quarry.
        let govts = govts(0, 0);
        let sites = sites();
        let mut cop = police();
        cop.goal = Goal::Idle;
        cop.target = Some(n(2));
        let npcs = [cop, ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0)];
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[0]));
        assert_eq!(decision.last_target(), Some(Some(n(2))));
        assert_eq!(Interceptor::default().target(&npcs[0], &around), None);
        let mut waiting = npcs[0].clone();
        waiting.goal = Goal::Wait;
        assert_eq!(
            Interceptor::default().target(&waiting, &around),
            None,
            "any other goal's quarry"
        );
    }

    #[test]
    fn a_waiting_interceptor_short_of_ammo_goes_idle() {
        // State 6 to 3 to 0 (@0x8d391, @0x8e228).
        let waiting = Npc {
            goal: Goal::Wait,
            ..short_of_ammo(police(), AmmoLeft::Dry, true)
        };
        assert_eq!(lone(waiting, &sites(), &[]).0, Goal::Idle);
    }

    #[test]
    fn an_interceptor_docking_with_a_plunder_or_drifting_fires_nothing() {
        // Subs 9 and 0xb end in `_AIEscortFireUnprovoked`, which returns
        // for AI type 4 or below (@0x80e59-0x80e61); sub 0xf and the
        // drift's sub 0 fire nothing.
        use crate::traffic::plunder_dock::{DockSub, Docking};
        let govts = govts(0, 0);
        let sites = sites();
        let mut hunter = police();
        hunter.goal = Goal::Attack(P);
        hunter.target = Some(P);
        hunter.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 20,
                ..weapon(128)
            }),
            1,
        )]);
        for sub in [DockSub::Approach, DockSub::Close, DockSub::Dock] {
            hunter.docking = Some(Docking { ship: P, sub });
            let npcs = [hunter.clone()];
            let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
            assert_eq!(
                Interceptor::default().trigger(&npcs[0], &around),
                Trigger::default(),
                "{sub:?}"
            );
        }
        hunter.docking = None;
        hunter.goal = Goal::Drift;
        let npcs = [hunter];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            Interceptor::default().trigger(&npcs[0], &around),
            Trigger::default()
        );
    }

    #[test]
    fn an_interceptor_targets_and_fires_at_whom_it_fights_and_not_whom_it_inspects() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut hunter = police();
        hunter.goal = Goal::Attack(P);
        hunter.target = Some(P);
        hunter.armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                speed: 1000,
                count: 20,
                ..weapon(128)
            }),
            1,
        )]);
        let npcs = [hunter.clone()];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(Interceptor::default().target(&npcs[0], &around), Some(P));
        assert_eq!(
            Interceptor::default().trigger(&npcs[0], &around).only,
            Some(WeaponId(128))
        );
        let inspecting = Npc {
            goal: Goal::Inspect(P),
            ..hunter
        };
        assert_eq!(Interceptor::default().target(&inspecting, &around), None);
        assert_eq!(
            Interceptor::default().trigger(&inspecting, &around),
            Trigger::default()
        );
        let hit = Strike {
            nominal: crate::combat::Nominal {
                mass: 3.0,
                energy: 0.0,
            },
            ..crate::combat::fixture::strike(n(1), P)
        };
        // Hit by the player it attacks, it keeps at it with no reaction
        // (`_DamageShip`'s focus test, @0x3bbf3-0x3bcac).
        assert_eq!(
            Interceptor::default().react(&npcs[0], &hit, &around),
            crate::ai::Reaction::default()
        );
        let bible = Surroundings {
            hit_focus: RuleSource::Bible,
            ..around
        };
        assert_eq!(
            Interceptor::default()
                .react(&npcs[0], &hit, &bible)
                .provoked,
            3.0,
            "by the other reading"
        );
    }

    #[test]
    fn an_interceptor_holds_back_a_0x0004_word_for_a_blessed_player_as_its_rule_says() {
        let govts = govts(0, crate::govt::ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        let npcs = [police()];
        let mut seen = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        if let Some(player) = seen.player.as_mut() {
            player.blessing = crate::ai::fixture::blessing_of(POLICE);
        }
        let decide = |always_attacks| {
            let interceptor = Interceptor {
                always_attacks,
                ..Interceptor::default()
            };
            interceptor.decide(&npcs[0], &seen, &mut Draws::of(&[0]))
        };
        assert_eq!(Interceptor::default().always_attacks, RuleSource::Engine);
        assert_eq!(decide(RuleSource::Engine), Goal::Attack(P));
        assert_eq!(
            decide(RuleSource::Bible),
            Goal::Inspect(P),
            "no word: it goes about inspecting"
        );
    }

    // The quote call (`_AIMakeShipAttackPlayer` @0x8cb8f, and
    // `_SelectWarshipTarget`'s).

    /// The interceptor's whole decision among `npcs` (it first), the police
    /// of `police_flags`, the player 1000 above in `condition` of record
    /// `record`, drawing `draws`.
    fn decision_by(
        police_flags: u16,
        npcs: &[Npc],
        condition: Condition,
        record: i16,
        draws: &[u32],
    ) -> Decision {
        let govts = govts(0, police_flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, (0.0, -1000.0), record);
        if let Some(player) = seen.player.as_mut() {
            player.condition = condition;
        }
        Interceptor::default().decision(&npcs[0], &seen, &mut Draws::of(draws))
    }

    #[test]
    fn an_interceptor_whose_word_drops_its_target_runs_its_gate_from_goal_0() {
        // The 0x0040 word writes +0x70 -1 and goal 0 (@0x8cbc0), so with
        // nothing picked the gate opens (@0x8cc6d) and nothing it held is
        // read again (@0x8cc93): a waiting interceptor threatening the
        // player inspects it.
        let mut threat = police();
        threat.goal = Goal::Wait;
        threat.target = Some(P);
        let decision = decision_by(
            crate::govt::NEVER_ATTACKS_PLAYER,
            &[threat],
            Condition::Intact,
            0,
            &[0; 4],
        );
        assert!(decision.dropped_target);
        assert!(decision.clears_target);
        assert_eq!(
            decision.goal,
            Goal::Inspect(P),
            "its inspection, not its wait"
        );
    }

    #[test]
    fn a_provoked_interceptor_takes_its_governments_word_and_is_provoked_by_it() {
        let mut provoked = police();
        provoked.goal = Goal::Attack(n(2));
        provoked.provoked = 5.0;
        provoked.target = Some(n(2));
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(
            decision_by(
                crate::govt::ALWAYS_ATTACKS_PLAYER,
                &[provoked, pirate],
                Condition::Intact,
                0,
                &[0]
            ),
            Decision {
                goal: Goal::Attack(P),
                turns_on_player: true,
                dropped_target: false,
                provokes: true,
                forgets_provocation: false,
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                holds_target: None,
                writes_sub: false,
                plunders: None,
                lost_sight: None,
                travel: crate::ai::travel::TravelWrite::Keep,
                visits: None,
                re_picked: false,
                looked_over: None,
                ends_wait: false,
                order_word: None,
                leaves_fleet: false,
                follows_jump: false,
                voice: None,
                hopeless_flee: None,
            },
            "@0x8cb8f-0x8cb9d"
        );
    }

    #[test]
    fn an_interceptors_word_and_hunt_turn_on_the_player_with_the_quote_call() {
        const ALWAYS: u16 = crate::govt::ALWAYS_ATTACKS_PLAYER;
        const NEVER: u16 = crate::govt::NEVER_ATTACKS_PLAYER;
        let intact = Condition::Intact;
        let turned = |goal| Decision {
            goal,
            turns_on_player: true,
            provokes: true,
            ..Decision::default()
        };
        let picked = |goal| Decision {
            goal,
            turns_on_player: true,
            ..Decision::default()
        };
        let not = |goal| Decision {
            goal,
            ..Decision::default()
        };
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let mut busy = police();
        busy.goal = Goal::Attack(n(2));
        assert_eq!(
            decision_by(ALWAYS, &[busy.clone(), pirate.clone()], intact, 0, &[0]),
            turned(Goal::Attack(P)),
            "its government's word (@0x8cb8f)"
        );
        // The word writes state 4, and the drop (@0x8d2fd-0x8d33a) lets a
        // disabled player go: state 0, no target, past the patrol.
        let dropped = Decision {
            clears_target: true,
            ..turned(Goal::Idle)
        };
        assert_eq!(
            decision_by(ALWAYS, &[busy, pirate], Condition::Disabled, 0, &[0]),
            dropped,
            "the word lets a disabled player go, +0x70 -1 (@0x8d33a): goal 4 closed the gate, no inspection"
        );
        let mut held = police();
        held.goal = Goal::Hold;
        assert_eq!(
            decision_by(ALWAYS, &[held], Condition::Disabled, 0, &[]),
            dropped,
            "held, the word lets a disabled player go, +0x70 -1 (@0x8d33a)"
        );
        assert_eq!(
            decision_by(0, &[police()], intact, -10, &[]),
            picked(Goal::Attack(P)),
            "the hunt's pick (@0x8ad31)"
        );
        let mut hunting = police();
        hunting.goal = Goal::Attack(P);
        assert_eq!(
            decision_by(0, &[hunting.clone()], intact, -10, &[]),
            not(Goal::Attack(P)),
            "the player kept"
        );
        assert_eq!(
            decision_by(0, &[police()], intact, 0, &[0]),
            Decision {
                comm_call: Some(CommCall::PreScan),
                ..not(Goal::Inspect(P))
            },
            "an inspection"
        );
        assert_eq!(
            decision_by(0, &[police()], intact, 0, &[]).goal,
            Goal::Inspect(P),
            "decide agrees"
        );
        let mut grudger = hunting;
        grudger.person = Some(crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(600),
            flags: crate::traffic::GRUDGE,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: true,
            quoted: false,
            quoted_at: None,
            zero_fuel: false,
        });
        assert_eq!(
            decision_by(NEVER, &[grudger], intact, 0, &[]),
            Decision {
                goal: Goal::Attack(P),
                turns_on_player: true,
                dropped_target: true,
                ..Decision::default()
            },
            "the 0x0040 word drops it (@0x8cbc0), the grudge takes it again"
        );
    }

    #[test]
    fn a_fresh_pick_of_the_player_raises_the_pre_scan_hail() {
        let intact = Condition::Intact;
        let call = |npcs: &[Npc], draws: &[u32]| decision_by(0, npcs, intact, 0, draws).comm_call;
        assert_eq!(call(&[police()], &[0]), Some(CommCall::PreScan), "@0x8ce4b");
        let trader = ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0);
        assert_eq!(
            decision_by(0, &[police(), trader.clone()], intact, 0, &[1]).goal,
            Goal::Inspect(n(2))
        );
        assert_eq!(
            call(&[police(), trader.clone()], &[1]),
            None,
            "an NPC picked"
        );
        let mut kept = police();
        kept.goal = Goal::Inspect(P);
        assert_eq!(
            decision_by(0, &[kept.clone()], intact, 0, &[]).goal,
            Goal::Inspect(P)
        );
        assert_eq!(call(&[kept], &[]), None, "an inspection kept");
        let mut dry = short_of_ammo(police(), AmmoLeft::Dry, true);
        dry.goal = Goal::Idle;
        let decision = decision_by(0, &[dry], intact, 0, &[0]);
        assert_eq!(
            (decision.goal, decision.comm_call),
            (Goal::Flee(P), Some(CommCall::PreScan)),
            "the hail comes in the pick, before the out-of-ammo retreat (@0x8d343)"
        );
    }

    // Out of ammunition (`_InterceptorAI` @0x8d343-0x8d391).

    /// The police interceptor with `goal`, `left` of its ammunition,
    /// retreating out of it as `retreats` says.
    fn short(goal: Goal, left: AmmoLeft, retreats: bool) -> Npc {
        short_of_ammo(Npc { goal, ..police() }, left, retreats)
    }

    #[test]
    fn an_interceptor_of_flags2_0x0080_retreats_from_its_target_out_of_ammunition() {
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let fight = Goal::Attack(n(2));
        for left in [AmmoLeft::Dry, AmmoLeft::OnlyUnlimited] {
            assert_eq!(
                decided(&[short(fight, left, true), pirate.clone()], 0, &[]),
                Goal::Flee(n(2)),
                "{left:?}"
            );
        }
        assert_eq!(
            decided(
                &[short(fight, AmmoLeft::Armed, true), pirate.clone()],
                0,
                &[]
            ),
            fight
        );
        assert_eq!(
            decided(&[short(fight, AmmoLeft::Dry, false), pirate], 0, &[]),
            fight,
            "without 0x0080 it fights on: the interceptor has no fully-dry drop"
        );
    }

    #[test]
    fn an_interceptor_of_flags2_0x0080_out_of_ammunition_flees_the_ship_it_inspects() {
        // An inspection holds its ship as the target (state 7, @0x8ce34-0x8ce42).
        let trader = ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
        let inspecting = short(Goal::Inspect(n(2)), AmmoLeft::Dry, true);
        assert_eq!(
            decided(&[inspecting, trader.clone()], 0, &[]),
            Goal::Flee(n(2)),
            "kept"
        );
        let idle = short(Goal::Idle, AmmoLeft::OnlyUnlimited, true);
        assert_eq!(
            decided(&[idle, trader], 0, &[1]),
            Goal::Flee(n(2)),
            "picked afresh"
        );
    }

    #[test]
    fn an_interceptor_of_flags2_0x0080_out_of_ammunition_with_no_target_stops() {
        // State 3 with target -1 is state 0 at once in `_HighLevelAIHandler`
        // (@0x8dff6 → @0x8e228-0x8e23d).
        let sites = sites();
        let govts = govts(0, 0);
        let decide = |npc: Npc, sites: &[crate::catalog::LandingSite]| {
            let npcs = [npc];
            let around = crate::ai::Surroundings {
                govts: &govts,
                ..crate::ai::Surroundings::new(sites, &npcs)
            };
            Interceptor::default().decide(&npcs[0], &around, &mut Draws::of(&[0]))
        };
        assert_eq!(
            decide(short(Goal::Idle, AmmoLeft::Dry, true), &sites),
            Goal::Idle,
            "its patrol"
        );
        assert_eq!(
            decide(short(Goal::Idle, AmmoLeft::Dry, false), &sites),
            Goal::Land(StellarId(128)),
            "without 0x0080 it patrols"
        );
        assert_eq!(
            decide(short(Goal::Hold, AmmoLeft::Dry, true), &sites),
            Goal::Idle,
            "its hold"
        );
        assert_eq!(
            decide(short(Goal::JumpOut, AmmoLeft::Dry, true), &[]),
            Goal::JumpOut,
            "leaving, state 2"
        );
    }

    /// The decision of `cop` inspecting NPC 2, a trader 500 off, cloaked
    /// as `hidden` says.
    fn inspecting(cop: Npc, hidden: bool) -> Decision {
        let trader = ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
        let npcs = [cop, if hidden { cloaked(trader) } else { trader }];
        let (sites, govts) = (sites(), govts(0, 0));
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        Interceptor::default().decision(&npcs[0], &around, &mut Draws::of(&[]))
    }

    #[test]
    fn an_interceptor_drops_an_inspection_it_loses_sight_of() {
        let cop = Npc {
            goal: Goal::Inspect(n(2)),
            ..police()
        };
        let decision = inspecting(cop, true);
        assert_eq!(decision.goal, Goal::Idle, "@0x8d39a-0x8d3ee");
        assert!(decision.clears_target);
    }

    // A cloaked player (`_ShipVisibleToShip` with the player the target).

    /// The interceptor's whole decision among `npcs` (it first), the police
    /// of `police_flags`, a cloaked player at `at`, drawing `draws`.
    fn by_cloaked_player(
        police_flags: u16,
        npcs: &[Npc],
        at: (f32, f32),
        draws: &[u32],
    ) -> Decision {
        let govts = govts(0, police_flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, at, 0);
        seen.player = seen.player.map(crate::ai::fixture::cloaked_player);
        Interceptor::default().decision(&npcs[0], &seen, &mut Draws::of(draws))
    }

    #[test]
    fn an_interceptors_0x0004_word_turns_it_on_no_cloaked_player_it_cannot_see() {
        const ALWAYS: u16 = crate::govt::ALWAYS_ATTACKS_PLAYER;
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let busy = Npc {
            goal: Goal::Attack(n(2)),
            ..police()
        };
        let unseen =
            by_cloaked_player(ALWAYS, &[busy.clone(), pirate.clone()], (0.0, -100.0), &[0]);
        assert_eq!(unseen.goal, Goal::Attack(n(2)), "it keeps its NPC fight");
        assert!(!unseen.turns_on_player && !unseen.provokes);
        let scanned = by_cloaked_player(ALWAYS, &[scanning(busy), pirate], (0.0, -100.0), &[0]);
        assert_eq!(
            (scanned.goal, scanned.turns_on_player, scanned.provokes),
            (Goal::Attack(P), true, true),
            "scanned within 200"
        );
    }

    #[test]
    fn an_interceptor_inspects_no_cloaked_player_it_cannot_see_and_raises_no_pre_scan_hail() {
        let unseen = by_cloaked_player(0, &[police()], (0.0, -1000.0), &[0]);
        assert_eq!(
            (unseen.goal, unseen.comm_call),
            (Goal::Land(StellarId(128)), None),
            "nothing it sees: it patrols"
        );
        let scanned = by_cloaked_player(0, &[scanning(police())], (0.0, -150.0), &[0]);
        assert_eq!(
            (scanned.goal, scanned.comm_call),
            (Goal::Inspect(P), Some(CommCall::PreScan)),
            "scanned within 200"
        );
    }

    #[test]
    fn an_interceptor_inspecting_the_player_drops_it_once_the_player_cloaks_out_of_sight() {
        let cop = Npc {
            goal: Goal::Inspect(P),
            ..police()
        };
        let decision = by_cloaked_player(0, std::slice::from_ref(&cop), (0.0, -1000.0), &[]);
        assert_eq!(decision.goal, Goal::Idle, "@0x8d3cd");
        assert!(decision.clears_target);
        let scanned = by_cloaked_player(0, &[scanning(cop)], (0.0, -150.0), &[]);
        assert_eq!(scanned.goal, Goal::Inspect(P), "scanned within 200");
        assert!(!scanned.clears_target);
    }

    #[test]
    fn one_whose_inspected_ship_is_seen_keeps_inspecting_it() {
        let cop = Npc {
            goal: Goal::Inspect(n(2)),
            ..police()
        };
        let decision = inspecting(cop, false);
        assert_eq!(decision.goal, Goal::Inspect(n(2)));
        assert!(!decision.clears_target);
    }

    #[test]
    fn an_inspection_the_out_of_ammo_retreat_turned_to_flight_is_left_alone() {
        let cop = short(Goal::Inspect(n(2)), AmmoLeft::Dry, true);
        let decision = inspecting(cop, true);
        assert_eq!(decision.goal, Goal::Flee(n(2)), "state 3, not 7");
        assert!(!decision.clears_target);
    }

    // The re-pick while inspecting (`_InterceptorAI` @0x8d404-0x8d43a).

    /// The decision of `cop` among `others` (it first), the player 1000
    /// above of record `record` and fighting as `combat_govt`, drawing
    /// `draws`; and the draws asked.
    fn re_picked(
        cop: Npc,
        others: &[Npc],
        record: i16,
        combat_govt: Option<crate::catalog::GovtId>,
        draws: &[u32],
    ) -> (Decision, Vec<u32>) {
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs: Vec<Npc> = std::iter::once(cop).chain(others.iter().cloned()).collect();
        let mut seen = around(&sites, &npcs, &govts, (0.0, -1000.0), record);
        if let Some(player) = seen.player.as_mut() {
            player.combat_govt = combat_govt;
        }
        let mut chance = Draws::of(draws);
        let decision = Interceptor::default().decision(&npcs[0], &seen, &mut chance);
        (decision, chance.asked)
    }

    /// A traders' interceptor at the centre inspecting NPC 2, a trader 500
    /// off.
    fn trader_cop_inspecting() -> (Npc, Npc) {
        let mut cop = ship(1, TRADERS, AiType::Interceptor, 0.0, 0.0);
        cop.goal = Goal::Inspect(n(2));
        (cop, ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0))
    }

    #[test]
    fn an_inspecting_interceptor_answers_only_a_provoker_it_holds() {
        // Its search needs state 1 or below, or 0x14 (@0x8cc6d-0x8cc7d):
        // while the inspection holds, a target a hit of no damage set is
        // kept unanswered, and so is a provoker it spares; one it holds is
        // answered (its provoked inspection is
        // rdm:phase/play-feel-9/phase-6-inspection-redirected-by-a-hit's).
        let (cop, trader) = trader_cop_inspecting();
        let hitter = |govt| ship(3, govt, AiType::Warship, 0.0, 300.0);
        let with = |provoked: f32| Npc {
            target: Some(n(3)),
            provoked,
            ..cop.clone()
        };
        let (decision, _) = re_picked(with(0.0), &[trader.clone(), hitter(NEUTRAL)], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(2)), "unprovoked");
        // An ally is answered too: the state-4 block spares it after the
        // think (`attack_drop`, @0x8e277-0x8e2db).
        let (decision, _) = re_picked(with(5.0), &[trader.clone(), hitter(POLICE)], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Attack(n(3)), "an ally");
        let (decision, _) = re_picked(with(5.0), &[trader, hitter(NEUTRAL)], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Attack(n(3)), "a provoker it holds");
    }

    #[test]
    fn an_inspecting_interceptor_does_not_hunt_at_its_first_search() {
        // `_SelectWarshipTarget` @0x8cce6 needs state 1 or below, or 0x14
        // (@0x8cc6d-0x8cc7d).
        let (sites, govts) = (sites(), govts(0, 0));
        let mut cop = police();
        cop.goal = Goal::Inspect(n(2));
        let npcs = [cop, ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0)];
        let wanted = around(&sites, &npcs, &govts, (0.0, -1000.0), -10);
        let mut chance = Draws::of(&[]);
        let first = hunt(
            &npcs[0],
            &wanted,
            INTERCEPTOR_RETREAT,
            RuleSource::Engine,
            TargetRules::default(),
            &mut chance,
        );
        assert_eq!(first.goal, None, "no search while inspecting");
        let mut idle = npcs.clone();
        idle[0].goal = Goal::Idle;
        let wanted = around(&sites, &idle, &govts, (0.0, -1000.0), -10);
        let first = hunt(
            &idle[0],
            &wanted,
            INTERCEPTOR_RETREAT,
            RuleSource::Engine,
            TargetRules::default(),
            &mut chance,
        );
        assert_eq!(first.goal, Some(Goal::Attack(P)), "idle, it searches");
        let mut provoked = npcs.clone();
        provoked[0].provoked = 3.0;
        provoked[0].target = Some(n(2));
        provoked[1].govt = Some(PIRATES);
        let seen = around(&sites, &provoked, &govts, (0.0, -1000.0), 0);
        let first = hunt(
            &provoked[0],
            &seen,
            INTERCEPTOR_RETREAT,
            RuleSource::Engine,
            TargetRules::default(),
            &mut chance,
        );
        assert_eq!(first.goal, Some(Goal::Attack(n(2))), "the provoker stands");
    }

    // An inspection that ends this think, within reach (the state-7 block
    // @0x8f391 lets it go: state 0, +0x70 -1) or gone, leaves the hunt its
    // full search (@0x8cc6d, `_SelectWarshipTarget` @0x8cce6), whose any
    // pick it attacks (state 4 @0x8cd02), threats asked.

    #[test]
    fn an_interceptor_within_reach_of_its_inspection_attacks_its_hunts_pick() {
        let (cop, _) = trader_cop_inspecting();
        let near = ship(2, TRADERS, AiType::WimpyTrader, 50.0, -50.0);
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 300.0);
        let (decision, _) = re_picked(cop.clone(), &[near.clone(), pirate], 0, None, &[0]);
        assert_eq!(decision.goal, Goal::Attack(n(3)), "not a fresh inspection");
        assert!(!decision.re_picked);
        let mut threat = ship(3, NEUTRAL, AiType::Warship, 0.0, 300.0);
        threat.goal = Goal::Attack(n(1));
        let (decision, _) = re_picked(cop, &[near, threat], 0, None, &[0]);
        assert_eq!(decision.goal, Goal::Attack(n(3)), "the threat step runs");
    }

    #[test]
    fn an_interceptor_that_has_scanned_the_player_attacks_its_hunts_pick() {
        let (mut cop, _) = trader_cop_inspecting();
        cop.goal = Goal::Inspect(P);
        let npcs = [cop, ship(3, PIRATES, AiType::Warship, 0.0, 300.0)];
        let (sites, govts) = (sites(), govts(0, 0));
        let seen = around(&sites, &npcs, &govts, (0.0, 50.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &seen, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Attack(n(3)));
    }

    #[test]
    fn an_interceptor_whose_inspected_ship_has_gone_attacks_its_hunts_pick() {
        let (mut cop, _) = trader_cop_inspecting();
        cop.goal = Goal::Inspect(n(9));
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 300.0);
        let (decision, _) = re_picked(cop, &[pirate], 0, None, &[0]);
        assert_eq!(decision.goal, Goal::Attack(n(3)));
    }

    #[test]
    fn an_inspecting_interceptor_inspects_its_hunts_pick_rather_than_attacking_it() {
        // The hunt's NPC pick writes the target alone (@0x8ad11).
        let (cop, trader) = trader_cop_inspecting();
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 300.0);
        let (decision, _) = re_picked(cop, &[trader, pirate], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(3)));
        assert!(decision.re_picked);
        assert!(!decision.turns_on_player);
    }

    #[test]
    fn an_inspecting_interceptor_attacks_a_pick_that_writes_state_4() {
        let (cop, trader) = trader_cop_inspecting();
        let (decision, _) = re_picked(cop.clone(), std::slice::from_ref(&trader), -10, None, &[]);
        assert_eq!(
            decision.goal,
            Goal::Attack(P),
            "the hunt's player (@0x8ad31)"
        );
        assert!(decision.turns_on_player);
        assert!(decision.re_picked);
        let mut police_cop = police();
        police_cop.goal = Goal::Inspect(n(2));
        let mut fleeing = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 200.0);
        fleeing.goal = Goal::Flee(n(4));
        let pirate = ship(4, PIRATES, AiType::Warship, 0.0, 300.0);
        let (decision, _) = re_picked(police_cop, &[trader, fleeing, pirate], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Attack(n(4)), "a help (@0x8a5c1)");
        assert!(!decision.turns_on_player, "the help makes no call");
    }

    #[test]
    fn an_inspecting_interceptor_with_nothing_to_pick_keeps_its_inspection() {
        let (cop, trader) = trader_cop_inspecting();
        let (decision, asked) = re_picked(cop, &[trader], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(2)), "@0x8d42d-0x8d431");
        assert!(!decision.re_picked);
        assert!(asked.is_empty());
    }

    #[test]
    fn an_inspecting_interceptor_re_picks_no_threat() {
        // Step 8 needs +0x70 -1 (@0x8ad3c), and the re-pick enters with
        // the inspected ship.
        let (cop, trader) = trader_cop_inspecting();
        let mut threat = ship(3, NEUTRAL, AiType::Warship, 0.0, 300.0);
        threat.goal = Goal::Attack(n(1));
        let (decision, _) = re_picked(cop, &[trader, threat], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(2)));
    }

    #[test]
    fn an_independent_inspecting_interceptor_re_picks_nothing() {
        // The re-pick needs a government (+0x98, @0x8d404).
        let (mut cop, trader) = trader_cop_inspecting();
        cop.govt = None;
        cop.person = Some(crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(600),
            flags: crate::traffic::GRUDGE,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: true,
            quoted: false,
            quoted_at: None,
            zero_fuel: false,
        });
        let (decision, _) = re_picked(cop.clone(), std::slice::from_ref(&trader), 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(2)), "it still inspects");
        cop.govt = Some(TRADERS);
        let (decision, _) = re_picked(cop, &[trader], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Attack(P), "with one, the grudge");
    }

    #[test]
    fn a_fresh_inspection_is_re_picked_in_the_same_think() {
        // The pick (@0x8ce34-0x8ce42) comes before the re-pick in one
        // think, which rolls the enemy class again (`Rand(50)` @0x8a993).
        let mut cop = ship(1, TRADERS, AiType::Interceptor, 0.0, 0.0);
        cop.goal = Goal::Idle;
        let trader = ship(2, TRADERS, AiType::WimpyTrader, 500.0, 0.0);
        let (decision, asked) = re_picked(
            cop.clone(),
            std::slice::from_ref(&trader),
            0,
            Some(PIRATES),
            &[1, 0, 0],
        );
        assert_eq!(
            asked,
            [50, 2, 50],
            "the hunt's roll, the pick, the re-pick's roll"
        );
        assert_eq!(decision.goal, Goal::Attack(P), "the second roll turns it");
        assert!(decision.turns_on_player);
        assert_eq!(
            decision.comm_call,
            Some(CommCall::PreScan),
            "the fresh pick's hail stands"
        );
        let (decision, _) = re_picked(cop, &[trader], 0, Some(PIRATES), &[1, 0, 1]);
        assert_eq!(decision.goal, Goal::Inspect(P));
    }

    #[test]
    fn a_lost_inspection_is_not_re_picked() {
        // The lost-sight tail leaves +0x70 -1 (@0x8d3d6-0x8d402).
        let (cop, trader) = trader_cop_inspecting();
        let (decision, asked) = re_picked(cop, &[cloaked(trader)], -10, None, &[]);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
        assert!(!decision.turns_on_player);
        assert!(asked.is_empty());
    }

    // A pick within reach ends in its own think: goal 7's block runs after
    // the think (@0x8f11e-0x8f394).

    #[test]
    fn a_pick_within_reach_is_looked_over_in_its_own_think() {
        // Picked by the draw, it being the only pick but the player.
        let picking = |x: f32, y: f32| {
            let trader = ship(2, TRADERS, AiType::WimpyTrader, x, y);
            re_picked(police(), &[trader], 0, None, &[1]).0
        };
        for (x, y) in [(50.0, -50.0), (100.0, 100.0), (-100.0, -100.0)] {
            let decision = picking(x, y);
            assert_eq!(decision.goal, Goal::Idle, "goal 0 (@0x8f391)");
            assert!(decision.clears_target, "+0x70 -1");
            assert_eq!(decision.looked_over, Some(n(2)), "({x}, {y})");
            assert_eq!(decision.comm_call, None);
        }
        for (x, y) in [(100.5, 0.0), (0.0, -100.5)] {
            let decision = picking(x, y);
            assert_eq!(decision.goal, Goal::Inspect(n(2)), "({x}, {y})");
            assert!(!decision.clears_target);
            assert_eq!(decision.looked_over, None);
        }
    }

    #[test]
    fn a_pick_of_the_player_within_reach_keeps_its_pre_scan_hail() {
        // The hail comes in the pick (@0x8ce4b), before the block's scan.
        let (sites, govts) = (sites(), govts(0, 0));
        let npcs = [police()];
        let seen = around(&sites, &npcs, &govts, (0.0, 50.0), 0);
        let decision = Interceptor::default().decision(&npcs[0], &seen, &mut Draws::of(&[0]));
        assert_eq!(decision.goal, Goal::Idle);
        assert_eq!(decision.looked_over, Some(P));
        assert_eq!(decision.comm_call, Some(CommCall::PreScan));
    }

    #[test]
    fn a_re_pick_within_reach_ends_the_inspection_of_the_new_ship() {
        // The reach reads the new target (@0x8f11e).
        let (cop, trader) = trader_cop_inspecting();
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 90.0);
        let (decision, _) = re_picked(cop.clone(), &[trader.clone(), pirate], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.re_picked);
        assert!(decision.clears_target);
        assert_eq!(decision.looked_over, Some(n(3)));
        // Beyond reach, it goes on to it.
        let pirate = ship(3, PIRATES, AiType::Warship, 0.0, 300.0);
        let (decision, _) = re_picked(cop, &[trader, pirate], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Inspect(n(3)));
        assert_eq!(decision.looked_over, None);
    }

    #[test]
    fn an_inspection_kept_beyond_reach_is_looked_over_by_nobody() {
        let (cop, trader) = trader_cop_inspecting();
        let (decision, _) = re_picked(cop, &[trader], 0, None, &[]);
        assert_eq!(decision.looked_over, None);
    }

    #[test]
    fn a_re_picked_attack_on_a_disabled_ship_stands_until_the_next_think() {
        // The disabled test (@0x8d2fd-0x8d33a) needs goal 4 and comes
        // before the re-pick (@0x8d41e), and the state-4 block has no
        // disabled drop: the pick stands that think.
        let (cop, trader) = trader_cop_inspecting();
        let mut police_cop = cop;
        police_cop.govt = Some(POLICE);
        let mut fleeing = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 200.0);
        fleeing.goal = Goal::Flee(n(4));
        let mut wreck = ship(4, PIRATES, AiType::Warship, 0.0, 300.0);
        wreck.condition = Condition::Disabled;
        let others = [trader.clone(), fleeing, wreck.clone()];
        let (decision, _) = re_picked(police_cop.clone(), &others, 0, None, &[]);
        assert_eq!(decision.goal, Goal::Attack(n(4)), "@0x8d41e");
        assert!(!decision.clears_target);
        // Its next think finds goal 4 on a disabled ship with no
        // destroying weapons and drops it (@0x8d319); with no call for help
        // to pick it again, it fights nothing.
        let attacking = Npc {
            goal: Goal::Attack(n(4)),
            target: Some(n(4)),
            ..police_cop
        };
        let (next, _) = re_picked(attacking, &[trader, wreck], 0, None, &[0; 4]);
        assert_eq!(next.goal.quarry(), None, "{next:?}");
    }

    #[test]
    fn a_re_picked_attack_in_a_hopeless_chase_snipes_unless_its_hull_stands_off() {
        // `_HighLevelAIHandler` asks `_AIEvalHopelessChase` of the state-4
        // pick that frame, past a stand-off hull (@0x8e48e).
        let (mut cop, trader) = trader_cop_inspecting();
        cop.hull.mass = 100.0;
        cop.hull.class_speed = 6.0;
        let decide = |cop: &Npc| {
            let (sites, govts) = (sites(), govts(0, 0));
            let npcs = [cop.clone(), trader.clone()];
            let mut seen = around(&sites, &npcs, &govts, (0.0, -1000.0), -10);
            if let Some(player) = seen.player.as_mut() {
                player.state.velocity = Vec2::new(0.0, -1.0);
                player.hull.class_speed = 6.0;
            }
            let think = Interceptor::default()
                .decision(&npcs[0], &seen, &mut Draws::of(&[]))
                .goal;
            assert_eq!(think, Goal::Attack(P), "the think's goal 4");
            crate::ai::NovaAi::default()
                .decision(&npcs[0], &seen, &mut Draws::of(&[]))
                .goal
        };
        assert_eq!(decide(&cop), Goal::Snipe(P), "the player opens the gap");
        cop.hull.stands_off = true;
        assert_eq!(
            decide(&cop),
            Goal::Attack(P),
            "a stand-off hull asks nothing"
        );
    }
}
