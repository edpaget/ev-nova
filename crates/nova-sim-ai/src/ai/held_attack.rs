//! A hunter's goal 4 (`_InterceptorAI` @0x8c896 and `_WarshipAI` @0x8b72a
//! in the `EV Nova` executable, then `_HighLevelAIHandler`'s goal-4 block
//! @0x8e246-0x8e731): an attack it holds, or a target its provoked branch
//! or its kept +0x70 turns into goal 4, taken through the think's tail
//! and the high level's drops in the original's order, with no search in
//! that think: the interceptor's ([`interceptor_hunt`]) and the warship's
//! ([`warship_hunt`]).
//!
//! # The interceptor
//!
//! **The provoked branch** (@0x8cbf6-0x8cc31): with +0x96 above 0, +0x70
//! set and +0x50 at or below 0, every goal but 3, 7, 9 and 0xf becomes 4
//! on +0x70. The gate then closes for goal 4 (@0x8cc6d-0x8cc7d), so no
//! `_SelectWarshipTarget` runs, and no more does it for an unprovoked
//! goal 4. Behind the gate the think's tail runs, in this order:
//!
//! 1. the retreats (@0x8d14d-0x8d2e4), which need a government (@0x8d157)
//!    and goal 4 (@0x8d161), the odds retreat also its government's
//!    `Flags` 0x0100 (@0x8d180): the flight is goal 3 on +0x70;
//! 2. the disabled drop (@0x8d2fa-0x8d33a): a disabled +0x70 with no
//!    destroying weapon, goal 0 and +0x70 -1;
//! 3. the out-of-ammo retreat (@0x8d343-0x8d391): goal 3 on +0x70, which
//!    the disabled drop has just made -1, so the high level stops it
//!    (@0x8dff6 → @0x8e228: goal 0, +0x96 0).
//!
//! Then the high level's slot test makes a gone +0x70 -1 (@0x8d4e4-0x8d50d),
//! and goal 4's block drops the target ([`drop_of`]), each drop with its
//! own writes:
//!
//! | Drop | Address | Goal | +0x70 | +0x96 |
//! |---|---|---|---|---|
//! | Disabled, no destroying weapon (think) | @0x8d2fa-0x8d33a | 0 | -1 | kept |
//! | Gone (the slot test made +0x70 -1) | @0x8e263 → @0x8e731 | 0 | -1 | kept |
//! | Ally (`_GovtAllies`, neither ship led by the player) | @0x8e277-0x8e2db → @0x8fb47 | 0, sub 0 | **kept** | kept |
//! | Its fleet | @0x8e2e9-0x8e399 → @0x8e381 | 0, sub 0 | -1 | kept |
//! | Breaking up (`_IsDying`) | @0x8e3ef → @0x8e714-0x8e726 | 0 | -1 | 0 |
//!
//! The inactive-slot branch (@0x8e3df → @0x8e714) can't be reached, the
//! slot test having come first, nor can goal 4's +0x4c test (@0x8e26d),
//! behind the handler's head gate (@0x8d4b9-0x8d4d2). Undropped, the
//! block flies the attack ([`state4_attack`]).
//!
//! The engine's own drops stand beside these: a player paid off
//! ([`Npc::spared`]; in the original a bribe makes the ship of AI type 1,
//! no interceptor at all) and, by the Bible's reading of
//! [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks),
//! a blessed player ([`releases_blessed`](super::warship::releases_blessed)),
//! each clearing the target as the fleet drop does.
//!
//! **The kept target.** The ally drop keeps +0x70, and provocation never
//! decays ([`provocation`](crate::traffic::provocation)): only a drop
//! zeroes it, and the ally drop doesn't. So each think writes goal 4 on the
//! ally again and the ally drop idles it again, with no search, for as
//! long as the ally stays in its slot; once it leaves, the slot let-go
//! clears +0x70 and the next think searches. An unprovoked interceptor
//! idle with +0x70 set searches, inspects and patrols nothing either
//! (@0x8cc93-0x8ccc9 → @0x8d13f, [`searches_nothing`]): its out-of-ammo
//! retreat alone still runs, fleeing +0x70
//! ([`out_of_ammo::after_patrol`]). The provoked write inside that gate
//! (@0x8ccbe-0x8ccd8) can't be reached: the provoked branch above has
//! already closed the gate.
//!
//! The answer to the question whether a goal-4 pick of
//! [`pick_target`](super::target::pick_target) differs from the gate's
//! skip: its step 1 keeps a live ship held, so they differ only where a
//! drop takes the target, where the engine re-picked in the same think and
//! the original idles that frame.
//!
//! # The warship
//!
//! `_WarshipAI`'s target section, with +0x70 set and no word, in order
//! ([`warship_hunt`]):
//!
//! - **The goal-0 block** (@0x8b835-0x8b881): with +0x50 at or below 0 it
//!   writes goal 4 on +0x70 (@0x8b843-0x8b860), with no
//!   `_SelectWarshipTarget` (@0x8b875) and no idle block (@0x8b88f). With
//!   +0x50 above 0 it keeps goal 0, which the high level's hold makes all
//!   but unreachable: it zeroes +0x50 every frame for goals 0, 1 and 0x14
//!   (@0x8d513-0x8d56b).
//! - **The provoked branch** (@0x8bb0c-0x8bb44): +0x96 above 0, +0x70 set
//!   and +0x50 at or below 0 make any goal but 7 or 3 goal 4, with no ally
//!   or fleet test. Its +0x50 gate is
//!   rdm:phase/escort-fleet-fidelity-followups-followups/phase-3-hunt-warp-and-wait-gates's.
//! - **Goals 1, 0x14 and 2** (@0x8bb56-0x8bba2): +0x70 set gives goal 4
//!   (@0x8bba2), -1 the search (@0x8bb96), each with +0x50 at or below 0;
//!   with it above, neither (@0x8bb75-0x8bb7c, @0x8bb8a-0x8bb91), and so
//!   for goal 0's search (@0x8b865-0x8b86c), which a warship leaving by
//!   hyperspace meets. **The wait** (@0x8bbab-0x8bc37) sets +0x70 -1 first
//!   (@0x8bbb9) and then searches, whatever +0x50. No other goal searches:
//!   with +0x70 -1 the think jumps to @0x8bf8c (@0x8bc48), its goal kept.
//! - **The own drop**, goal 4 only (@0x8bc4e-0x8bcaf): a target inactive
//!   (+0xb8 @0x8bc67), breaking up (@0x8bc73), or disabled while it has no
//!   destroying weapon (@0x8bc8b-0x8bc9e) gives goal 0 and +0x70 -1
//!   (@0x8bca0-0x8bca9), +0x96 kept, and jumps to @0x8bf8c (@0x8bcb4), past
//!   the retreats and the idle block.
//! - **The retreats**, which see the rewritten goal 4: the odds retreat
//!   (@0x8bcba-0x8bdf5: a government @0x8bcc5, goal 4 @0x8bccb, `Flags`
//!   0x0010 @0x8bcea); the late goal-4 write for any goal but 7 and 3 with
//!   +0x50 at or below 0 (@0x8be09-0x8be2d), which reaches every goal with
//!   +0x70 set that the blocks above left alone; the shield retreat
//!   (@0x8be36-0x8bf35: no fleet lead @0x8beea, `Flags` 0x0010 @0x8bf12,
//!   a goal not 2, 3 or 0xb @0x8bf19-0x8bf33); the out-of-ammo retreat
//!   (@0x8bf3e-0x8bf83). Each gives goal 3 on +0x70.
//! - **Fully dry** (@0x8c0e7-0x8c117), goal 4: goal 0, sub 0, +0x70 -1
//!   (@0x8c111) and +0x6c -1 (@0x8c117), before the high level, so no goal-4
//!   drop follows it ([`Warship`](super::Warship)'s decision makes it).
//! - **Goal 4's high level** is the table above less its first row, the
//!   interceptor's think's own disabled drop: the ally drop keeps +0x70 and
//!   +0x96, the fleet drops clear +0x70, and the breaking-up drop clears
//!   +0x70 and +0x96. A warship that came through its own drop meets no
//!   breaking-up target there; one that reached goal 4 by the late write
//!   (holding or drifting with +0x70 set, unprovoked) skipped the own drop
//!   and the odds retreat, which need goal 4, and meets it there.
//!
//! So a warship whose target is an ally takes goal 4 on it at every think,
//! provoked or not, and the ally drop idles it again with no search, for
//! as long as the ally stays in its slot: [`Warship`](super::Warship)
//! targets it idle, as [`Interceptor`](super::Interceptor) does. A warship
//! whose retreat fires first flees the ally instead, and within 250 goal
//! 3's block then stops it, clearing +0x70 (@0x8e81d-0x8e838,
//! [`flee_drop`](super::flee_drop)). The late write also reaches a hold or
//! a drift holding +0x70 unprovoked, as a hit of no nominal damage leaves
//! it ([`react`](super::react)): it turns on that ship, after its shield
//! and out-of-ammo retreats.
//!
//! A fresh pick that the own drop takes jumps past the idle block too
//! (@0x8bcb4 → @0x8bf8c); here it still runs the idle block, its target
//! cleared: rdm:phase/escort-fleet-fidelity-followups/phase-22-spared-ships-reach-the-block
//! built that on main. `_WarshipAI`'s sub-4 rewrite to goal 2
//! (@0x8b80f-0x8b82c) can't be reached: sub 4 or 0xd takes `_AIDispatch`'s
//! jump path (@0x8fd83-0x8fd9a → @0x900d5), which skips the think.
//!
//! Nothing here plays a sound or draws a random number.

use crate::ai::interceptor::{gate_closed, searches_nothing};
use crate::ai::odds::state4_attack;
use crate::ai::target::{TargetRules, spares_ally};
use crate::ai::warship::{Hunt, Retreat, releases_blessed, retreats};
use crate::ai::{Goal, Surroundings, fire, out_of_ammo};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// A drop of goal 4's target (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    /// Disabled while the ship has no destroying weapon: the think's
    /// drop (@0x8d2fa-0x8d33a).
    Disabled,
    /// Gone from its slot, or not yet arrived (@0x8e263 → @0x8e731).
    Gone,
    /// An ally it spares (@0x8e277-0x8e2db).
    Ally,
    /// A ship of its own fleet (@0x8e2e9-0x8e399).
    Fleet,
    /// The player paid off, or blessed by the Bible's reading: the
    /// engine's own.
    PaidOff,
    /// Breaking up (@0x8e3ef → @0x8e714-0x8e726).
    Dying,
}

impl Drop {
    /// Whether the drop makes the target -1: all but the ally drop.
    #[must_use]
    pub fn clears_target(self) -> bool {
        self != Self::Ally
    }

    /// Whether the drop zeroes the provocation: the breaking-up drop
    /// alone (@0x8e714-0x8e726).
    #[must_use]
    pub fn forgets_provocation(self) -> bool {
        self == Self::Dying
    }
}

/// The high level's drop of `ship`, the target of `npc` in goal 4, among
/// `around`, by `rules` and `always_attacks`, in the original's order:
/// gone, an ally, its fleet, the engine's paid-off or blessed player,
/// breaking up. None when goal 4 flies on.
#[must_use]
pub fn drop_of(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Option<Drop> {
    let jumping_in = match ship {
        ShipRef::Player => false,
        ShipRef::Npc(id) => around.npc(id).is_some_and(|other| !other.arrived()),
    };
    if jumping_in || !around.condition_of(ship).is_some_and(Condition::active) {
        return Some(Drop::Gone);
    }
    if let ShipRef::Npc(id) = ship
        && let Some(other) = around.npc(id)
    {
        if spares_ally(npc, other, around, rules.led_ally) {
            return Some(Drop::Ally);
        }
        if other.fleet_among(around.npcs) == npc.fleet_among(around.npcs) {
            return Some(Drop::Fleet);
        }
    }
    if (ship == ShipRef::Player && npc.spared)
        || releases_blessed(npc, ship, around, always_attacks)
    {
        return Some(Drop::PaidOff);
    }
    matches!(around.condition_of(ship), Some(Condition::Dying { .. })).then_some(Drop::Dying)
}

/// What an interceptor `npc` does with goal 4 on `ship` among `around`,
/// retreating as `retreat` says and dropping by `rules` and
/// `always_attacks`: the think's tail, then goal 4's high level (see the
/// module docs). Judged as attacking `ship` whatever goal it held, as the
/// provoked branch writes goal 4 first.
#[must_use]
pub fn interceptor_attack(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Hunt {
    let judged = Npc {
        goal: Goal::Attack(ship),
        ..npc.clone()
    };
    if retreats(&judged, around, retreat) {
        return Hunt::of(Goal::Flee(ship));
    }
    let dry = retreat.out_of_ammo && out_of_ammo::retreats(npc);
    if around.condition_of(ship) == Some(Condition::Disabled) && !fire::destroys(npc) {
        // Goal 0, +0x70 -1; dry, goal 3 on -1, which the high level stops
        // (@0x8dff6 → @0x8e228: +0x96 0).
        return Hunt {
            goal: Some(Goal::Idle),
            clears_target: true,
            forgets_provocation: dry,
            ..Hunt::default()
        };
    }
    if dry {
        return Hunt::of(Goal::Flee(ship));
    }
    match drop_of(npc, ship, around, rules, always_attacks) {
        Some(drop) => Hunt {
            goal: Some(Goal::Idle),
            clears_target: drop.clears_target(),
            forgets_provocation: drop.forgets_provocation(),
            ..Hunt::default()
        },
        None => Hunt::of(state4_attack(npc, ship, around)),
    }
}

/// What an interceptor `npc`'s think does among `around` before its
/// search, by `retreat`, `rules` and `always_attacks`, when it holds no
/// word and flies no flight it keeps (see the module docs): provoked with
/// a target, any goal but a flight, an inspection or a help, goal 4 on it
/// ([`interceptor_attack`]); unprovoked in goal 4, the same on the ship
/// it attacks or snipes at; and nothing to fight, searching nothing, idle
/// with a target it kept ([`searches_nothing`]) or behind a closed gate
/// ([`gate_closed`]). None leaves it the hunt's own path: an inspection
/// this think ends.
#[must_use]
pub fn interceptor_hunt(
    npc: &Npc,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Option<Hunt> {
    let attack = |ship| interceptor_attack(npc, ship, around, retreat, rules, always_attacks);
    let provoked_branch = !matches!(
        npc.goal,
        Goal::Flee(_) | Goal::Inspect(_) | Goal::Assist(_) | Goal::CountWait
    );
    if let Some(ship) = npc.target.filter(|_| npc.provoked > 0.0 && provoked_branch) {
        return Some(attack(ship));
    }
    if let Some(ship) = npc.goal.attacking() {
        return Some(attack(ship));
    }
    (searches_nothing(npc) || gate_closed(npc, around)).then(Hunt::default)
}

/// Whether a warship `npc`'s own drop takes `ship`, among `around`: its
/// slot inactive (+0xb8 @0x8bc67), breaking up (`_IsDying` @0x8bc73), or
/// disabled while `npc` has no destroying weapon (@0x8bc8b-0x8bc9e).
fn warship_drops(npc: &Npc, ship: ShipRef, around: &Surroundings) -> bool {
    match around.condition_of(ship) {
        None | Some(Condition::Destroyed | Condition::Dying { .. }) => true,
        Some(Condition::Disabled) => !fire::destroys(npc),
        Some(Condition::Intact) => false,
    }
}

/// What a warship `npc` does with goal 4 on `ship` among `around` after
/// its retreats: its fully-dry drop, left to
/// [`Warship`](super::Warship)'s decision, which comes before the high
/// level (@0x8c0e7-0x8c117); then goal 4's drops ([`drop_of`] by `rules`
/// and `always_attacks`, each with its writes); otherwise the attack
/// ([`state4_attack`]).
fn warship_high_level(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Hunt {
    if out_of_ammo::drop_when_dry(npc, Goal::Attack(ship)) != Goal::Attack(ship) {
        return Hunt::of(Goal::Attack(ship));
    }
    match drop_of(npc, ship, around, rules, always_attacks) {
        Some(drop) => Hunt {
            goal: Some(Goal::Idle),
            clears_target: drop.clears_target(),
            forgets_provocation: drop.forgets_provocation(),
            ..Hunt::default()
        },
        None => Hunt::of(state4_attack(npc, ship, around)),
    }
}

/// What a warship `npc` does with goal 4 on `ship` among `around`,
/// retreating as `retreat` says and dropping by `rules` and
/// `always_attacks`, in `_WarshipAI`'s order (see the module docs): its
/// own drop (goal 0, +0x70 -1, past the retreats and the idle block,
/// @0x8bca0-0x8bcb4), then its retreats judged as goal 4 (@0x8bcba-0x8bf83),
/// then its fully-dry drop and goal 4's high level.
#[must_use]
pub fn warship_attack(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Hunt {
    if warship_drops(npc, ship, around) {
        return Hunt {
            goal: Some(Goal::Idle),
            clears_target: true,
            ..Hunt::default()
        };
    }
    let judged = Npc {
        goal: Goal::Attack(ship),
        ..npc.clone()
    };
    if retreats(&judged, around, retreat) || (retreat.out_of_ammo && out_of_ammo::retreats(npc)) {
        return Hunt::of(Goal::Flee(ship));
    }
    warship_high_level(npc, ship, around, rules, always_attacks)
}

/// What a warship `npc`'s think does among `around` before its search, by
/// `retreat`, `rules` and `always_attacks`, when it holds no word and
/// flies no flight (see the module docs), in `_WarshipAI`'s order:
///
/// 1. inspecting (goal 7), provoked or not, nothing (@0x8bb2d, @0x8be10);
/// 2. provoked with a target, goal 4 on it ([`warship_attack`],
///    @0x8bb0c-0x8bb44);
/// 3. unprovoked in goal 4, the same on the ship it attacks or snipes at;
/// 4. waiting, the wait's search, whatever its warp; following or in
///    formation (the other reading of
///    [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)), the search
///    as before;
/// 5. unprovoked with a target and its warp under way, no goal-4 write
///    (@0x8b84a-0x8b851, @0x8bb75-0x8bb7c, @0x8be24-0x8be2b): its shield
///    and out-of-ammo retreats judged as the goal it holds, which skip goal
///    2 (@0x8bf19-0x8bf33), or its goal kept;
/// 6. unprovoked idle, landing, entering a gate or leaving with a target,
///    goal 4 on it (@0x8b843-0x8b860, @0x8bb6e-0x8bba2);
/// 7. unprovoked in any other goal with a target, holding or drifting
///    among them, the late goal-4 write (@0x8be09-0x8be2d): past its own
///    drop and its odds retreat, which need goal 4, its shield and
///    out-of-ammo retreats judged as the goal it holds, then its fully-dry
///    drop and goal 4's high level;
/// 8. with no target, idle, landing, entering a gate or leaving, the
///    search, but with its warp under way (@0x8b865-0x8b86c,
///    @0x8bb8a-0x8bb91); in any other goal, holding among them, its goal
///    kept with no search and no idle block (@0x8bc48 → @0x8bf8c).
///
/// None leaves it the search.
#[must_use]
pub fn warship_hunt(
    npc: &Npc,
    around: &Surroundings,
    retreat: Retreat,
    rules: TargetRules,
    always_attacks: RuleSource,
) -> Option<Hunt> {
    let attack = |ship| warship_attack(npc, ship, around, retreat, rules, always_attacks);
    if let Goal::Inspect(_) = npc.goal {
        return Some(Hunt::default());
    }
    if let Some(ship) = npc.target.filter(|_| npc.provoked > 0.0) {
        return Some(attack(ship));
    }
    if let Some(ship) = npc.goal.attacking() {
        return Some(attack(ship));
    }
    let warping = npc.warp.stage > 0.0;
    // The shield and out-of-ammo retreats, judged as the goal it holds, so
    // with no odds retreat (it needs goal 4, @0x8bccb), and for no goal 2
    // (@0x8bf19-0x8bf33, @0x8bf67-0x8bf83).
    let worn = |ship| {
        (npc.goal != Goal::JumpOut
            && (retreats(npc, around, retreat)
                || (retreat.out_of_ammo && out_of_ammo::retreats(npc))))
        .then(|| Hunt::of(Goal::Flee(ship)))
    };
    let searches = matches!(
        npc.goal,
        Goal::Idle | Goal::Land(_) | Goal::EnterGate(_) | Goal::JumpOut
    );
    let waits = matches!(
        npc.goal,
        Goal::Wait | Goal::Follow { .. } | Goal::Formation { .. }
    );
    match npc.target {
        // The wait writes +0x70 -1 first and searches, whatever +0x50
        // (@0x8bbab-0x8bbc2); a follow or formation (the other reading of
        // `RuleKey::NpcEscortAi`) is `_EscortAI`'s.
        _ if waits => None,
        // With the warp under way no goal-4 write comes (@0x8b84a-0x8b851,
        // @0x8bb75-0x8bb7c, @0x8be24-0x8be2b).
        Some(ship) if warping => Some(worn(ship).unwrap_or_else(|| Hunt::of(npc.goal))),
        // Goal 4 at once (@0x8b843-0x8b860, @0x8bba2).
        Some(ship) if searches => Some(attack(ship)),
        // The late write (@0x8be09-0x8be2d), past the own drop and the
        // odds retreat.
        Some(ship) => Some(
            worn(ship)
                .unwrap_or_else(|| warship_high_level(npc, ship, around, rules, always_attacks)),
        ),
        // The search (@0x8b875, @0x8bb96), but with the warp under way
        // (@0x8b865-0x8b86c, @0x8bb8a-0x8bb91).
        None if searches && !warping => None,
        // No search and no idle block: the jump to @0x8bf8c (@0x8bc48).
        None => Some(Hunt::of(npc.goal)),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{
        NEUTRAL, PIRATES, POLICE, TRADERS, around, blessing_of, govts, n, ship, short_of_ammo,
        sites,
    };
    use crate::ai::interceptor::INTERCEPTOR_RETREAT;
    use crate::catalog::StellarId;
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::govt::{ALWAYS_ATTACKS_PLAYER, INTERCEPTORS_RETREAT};
    use crate::traffic::npc::{AiType, Mode, NpcId};

    const P: ShipRef = ShipRef::Player;

    /// A police interceptor (NPC 1) at the centre, attacking `ship`.
    fn police(ship_ref: ShipRef) -> Npc {
        let mut cop = ship(1, POLICE, AiType::Interceptor, 0.0, 0.0);
        cop.goal = Goal::Attack(ship_ref);
        cop.target = Some(ship_ref);
        cop
    }

    fn dropped_among(npcs: &[Npc], target: ShipRef, rules: TargetRules) -> Option<Drop> {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -1000.0), 0);
        drop_of(&npcs[0], target, &around, rules, RuleSource::Engine)
    }

    fn led_by_player(mut npc: Npc) -> Npc {
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: None,
        });
        npc
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    #[test]
    fn a_ship_gone_or_still_jumping_in_is_the_gone_drop() {
        let alone = [police(n(2))];
        assert_eq!(
            dropped_among(&alone, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
        let mut arriving = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        arriving.mode = Mode::JumpingIn { tick: 0 };
        let npcs = [police(n(2)), arriving];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
        let mut destroyed = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        destroyed.condition = Condition::Destroyed;
        let npcs = [police(n(2)), destroyed];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Gone)
        );
    }

    #[test]
    fn an_ally_is_dropped_by_either_reading_and_one_the_player_leads_only_by_the_bible() {
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            let rules = TargetRules {
                led_ally,
                ..TargetRules::default()
            };
            let npcs = [police(n(2)), ally.clone()];
            assert_eq!(dropped_among(&npcs, n(2), rules), Some(Drop::Ally));
        }
        let npcs = [police(n(2)), led_by_player(ally)];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            None,
            "the engine's ally test needs neither led by the player"
        );
        let bible = TargetRules {
            led_ally: RuleSource::Bible,
            ..TargetRules::default()
        };
        assert_eq!(dropped_among(&npcs, n(2), bible), Some(Drop::Ally));
    }

    #[test]
    fn a_ship_of_its_fleet_the_paid_off_player_and_a_ship_breaking_up_are_dropped() {
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(1));
        let npcs = [police(n(2)), escort];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Fleet)
        );
        let mut bribed = police(P);
        bribed.spared = true;
        assert_eq!(
            dropped_among(&[bribed], P, TargetRules::default()),
            Some(Drop::PaidOff)
        );
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        let npcs = [police(n(2)), dying];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Dying)
        );
        let npcs = [police(n(2)), ship(2, PIRATES, AiType::Warship, 0.0, 300.0)];
        assert_eq!(dropped_among(&npcs, n(2), TargetRules::default()), None);
    }

    #[test]
    fn by_the_bible_a_blessed_player_is_dropped_and_by_the_engine_it_is_not() {
        let govts = govts(0, ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        let npcs = [police(P)];
        let mut around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        if let Some(player) = around.player.as_mut() {
            player.blessing = blessing_of(POLICE);
        }
        let rules = TargetRules::default();
        assert_eq!(
            drop_of(&npcs[0], P, &around, rules, RuleSource::Bible),
            Some(Drop::PaidOff)
        );
        assert_eq!(
            drop_of(&npcs[0], P, &around, rules, RuleSource::Engine),
            None
        );
    }

    #[test]
    fn the_ally_drop_comes_before_the_breaking_up_one_so_a_dying_ally_keeps_its_target() {
        let mut ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        ally.condition = DYING;
        let npcs = [police(n(2)), ally];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Ally)
        );
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(1));
        escort.condition = DYING;
        let npcs = [police(n(2)), escort];
        assert_eq!(
            dropped_among(&npcs, n(2), TargetRules::default()),
            Some(Drop::Fleet),
            "the fleet drop comes before it too"
        );
    }

    #[test]
    fn each_drop_writes_its_own_target_and_provocation() {
        let writes = |drop: Drop| (drop.clears_target(), drop.forgets_provocation());
        assert_eq!(writes(Drop::Disabled), (true, false));
        assert_eq!(writes(Drop::Gone), (true, false));
        assert_eq!(writes(Drop::Ally), (false, false));
        assert_eq!(writes(Drop::Fleet), (true, false));
        assert_eq!(writes(Drop::PaidOff), (true, false));
        assert_eq!(writes(Drop::Dying), (true, true));
    }

    /// The interceptor `cop`'s goal 4 on NPC 2, `quarry`, by the police
    /// flags `police_flags`.
    fn attacked(cop: &Npc, quarry: &Npc, police_flags: u16) -> Hunt {
        let npcs = [cop.clone(), quarry.clone()];
        let govts = govts(0, police_flags);
        let sites = sites();
        let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        interceptor_attack(
            &npcs[0],
            n(2),
            &around,
            INTERCEPTOR_RETREAT,
            TargetRules::default(),
            RuleSource::Engine,
        )
    }

    fn disabled_pirate() -> Npc {
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.condition = Condition::Disabled;
        pirate
    }

    #[test]
    fn an_outnumbered_interceptor_retreats_before_its_drops() {
        let mut cop = police(n(2));
        cop.goal = Goal::Wait;
        cop.reserves.shield.now = 1.0;
        cop.odds = 5.0;
        let hunt = attacked(&cop, &disabled_pirate(), INTERCEPTORS_RETREAT);
        assert_eq!(hunt, Hunt::of(Goal::Flee(n(2))), "judged as goal 4");
    }

    #[test]
    fn the_disabled_drop_idles_clearing_the_target_and_dry_it_forgets_its_provocation() {
        let hunt = attacked(&police(n(2)), &disabled_pirate(), 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, false)
        );
        let dry = short_of_ammo(police(n(2)), AmmoLeft::Dry, true);
        let hunt = attacked(&dry, &disabled_pirate(), 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, true),
            "goal 3 on -1, stopped (@0x8e228)"
        );
    }

    #[test]
    fn dry_with_no_disabled_drop_it_flees_before_the_high_levels_drops() {
        let dry = short_of_ammo(police(n(2)), AmmoLeft::Dry, true);
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        assert_eq!(attacked(&dry, &dying, 0), Hunt::of(Goal::Flee(n(2))));
    }

    #[test]
    fn the_high_levels_drops_idle_it_with_their_writes_and_otherwise_it_attacks() {
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        let hunt = attacked(&police(n(2)), &dying, 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), true, true)
        );
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        let hunt = attacked(&police(n(2)), &ally, 0);
        assert_eq!(
            (hunt.goal, hunt.clears_target, hunt.forgets_provocation),
            (Some(Goal::Idle), false, false)
        );
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(
            attacked(&police(n(2)), &pirate, 0),
            Hunt::of(Goal::Attack(n(2)))
        );
    }

    // A warship's goal 4 (`_WarshipAI` @0x8b835-0x8c117, then goal 4's
    // high level), through its whole decision.

    use crate::ai::travel::TravelWrite;
    use crate::ai::{Behaviour, Decision, Warship};
    use crate::govt::WARSHIPS_RETREAT;
    use nova_sim_base::fixture::Draws;

    /// A police warship (NPC 1) at the centre holding `goal`, its target
    /// `target`, provoked by `provoked`.
    fn warship(goal: Goal, target: Option<ShipRef>, provoked: f32) -> Npc {
        let mut cop = ship(1, POLICE, AiType::Warship, 0.0, 0.0);
        cop.goal = goal;
        cop.target = target;
        cop.provoked = provoked;
        cop
    }

    /// An allied trader (NPC 2), 300 above.
    fn ally() -> Npc {
        ship(2, TRADERS, AiType::WimpyTrader, 0.0, 300.0)
    }

    /// The warship's decision among `npcs` (it first), its police of
    /// `police_flags`, by `led_ally`, with a wanted player 300 below that
    /// its search would take; and the draws asked.
    fn warship_decided(
        npcs: &[Npc],
        police_flags: u16,
        led_ally: RuleSource,
    ) -> (Decision, Vec<u32>) {
        let govts = govts(0, police_flags);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -300.0), -10);
        let warship = Warship {
            target: TargetRules {
                led_ally,
                ..TargetRules::default()
            },
            ..Warship::default()
        };
        let mut chance = Draws::of(&[0; 4]);
        let decision = warship.decision(&npcs[0], &around, &mut chance);
        (decision, chance.asked)
    }

    fn by_engine(npcs: &[Npc]) -> Decision {
        warship_decided(npcs, 0, RuleSource::Engine).0
    }

    #[test]
    fn a_warship_provoked_by_an_ally_idles_keeping_it_with_no_search() {
        // The provoked branch writes goal 4 with no ally test
        // (@0x8bb0c-0x8bb44), and goal 4's ally drop idles it, keeping
        // +0x70 and +0x96 (@0x8e277-0x8e2db → @0x8fb47).
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            let npcs = [warship(Goal::Idle, Some(n(2)), 5.0), ally()];
            let (decision, asked) = warship_decided(&npcs, 0, led_ally);
            assert_eq!(decision.goal, Goal::Idle, "{led_ally:?}");
            assert!(!decision.clears_target, "{led_ally:?}");
            assert!(!decision.forgets_provocation, "{led_ally:?}");
            assert_eq!(decision.travel, TravelWrite::Keep, "{led_ally:?}");
            assert!(asked.is_empty(), "{led_ally:?}: no search");
        }
    }

    #[test]
    fn an_unprovoked_idle_warship_with_a_kept_target_takes_goal_4_on_it_with_no_search() {
        // The goal-0 block writes goal 4 on a +0x70 that is set
        // (@0x8b843-0x8b860), with no `_SelectWarshipTarget` (@0x8b875).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let npcs = [warship(Goal::Idle, Some(n(2)), 0.0), neutral];
        let (decision, asked) = warship_decided(&npcs, 0, RuleSource::Engine);
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        assert!(asked.is_empty(), "no search");
        let npcs = [warship(Goal::Idle, Some(n(2)), 0.0), ally()];
        let (decision, asked) = warship_decided(&npcs, 0, RuleSource::Engine);
        assert_eq!(
            (decision.goal, decision.clears_target),
            (Goal::Idle, false),
            "an ally it spares is idled on again"
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn a_warship_with_a_kept_target_landing_entering_a_gate_or_leaving_takes_goal_4_on_it() {
        // Goals 1, 0x14 and 2 with +0x70 set write goal 4 (@0x8bba2); with
        // the warp under way the goal stands (@0x8bb75-0x8bb7c).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        for goal in [
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::JumpOut,
        ] {
            let npcs = [warship(goal, Some(n(2)), 0.0), neutral.clone()];
            assert_eq!(by_engine(&npcs).goal, Goal::Attack(n(2)), "{goal:?}");
            let mut warping = npcs.clone();
            warping[0].warp.stage = 10.0;
            let (decision, asked) = warship_decided(&warping, 0, RuleSource::Engine);
            assert_eq!(decision.goal, goal, "{goal:?} warping");
            assert!(asked.is_empty(), "{goal:?} warping: no search");
        }
        let mut idle = [warship(Goal::Idle, Some(n(2)), 0.0), neutral];
        idle[0].warp.stage = 10.0;
        let (decision, asked) = warship_decided(&idle, 0, RuleSource::Engine);
        assert_eq!(decision.goal, Goal::Idle, "goal 0 kept (@0x8b84a-0x8b851)");
        assert_eq!(decision.travel, TravelWrite::Keep, "no idle block");
        assert!(asked.is_empty());
    }

    const DYING_LONG: Condition = Condition::Dying {
        ticks_left: 30,
        starting: false,
    };

    #[test]
    fn a_warship_attacking_a_ship_breaking_up_or_disabled_drops_it_past_the_idle_block() {
        // The own drop: goal 0, +0x70 -1 (@0x8bca0-0x8bca9), then the jump
        // past the retreats and the idle block (@0x8bcb4 → @0x8bf8c),
        // +0x96 kept.
        let mut dying = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING_LONG;
        let mut disabled = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        disabled.condition = Condition::Disabled;
        for quarry in [dying, disabled] {
            for provoked in [0.0, 5.0] {
                let npcs = [
                    warship(Goal::Attack(n(2)), Some(n(2)), provoked),
                    quarry.clone(),
                ];
                let (decision, asked) = warship_decided(&npcs, 0, RuleSource::Engine);
                let why = format!("{:?} provoked {provoked}", quarry.condition);
                assert_eq!(decision.goal, Goal::Idle, "{why}");
                assert!(decision.clears_target, "{why}");
                assert!(!decision.forgets_provocation, "{why}");
                assert_eq!(decision.travel, TravelWrite::Keep, "{why}: no pick");
                assert!(asked.is_empty(), "{why}: no search");
            }
        }
    }

    #[test]
    fn a_warship_attacking_one_of_its_fleet_idles_clearing_its_target() {
        // Goal 4's fleet drop: goal 0, sub 0, +0x70 -1 (@0x8e2e9-0x8e399).
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(1));
        let npcs = [warship(Goal::Attack(n(2)), Some(n(2)), 0.0), escort];
        let (decision, asked) = warship_decided(&npcs, 0, RuleSource::Engine);
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
    fn a_warship_attacking_a_disabled_ally_drops_it_by_its_own_drop_first() {
        // The own drop (@0x8bc8b-0x8bca9) comes before goal 4's ally drop.
        let mut downed = ally();
        downed.condition = Condition::Disabled;
        let npcs = [warship(Goal::Attack(n(2)), Some(n(2)), 5.0), downed];
        let decision = by_engine(&npcs);
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
    }

    /// `npc` with its shields below half, outnumbered by its cached odds.
    fn outnumbered(mut npc: Npc) -> Npc {
        npc.reserves.shield.now = 14.9;
        npc.aggression = 3;
        npc.odds = 5.0;
        npc
    }

    #[test]
    fn an_outnumbered_warship_provoked_by_an_ally_flees_it() {
        // The retreats see the rewritten goal 4 (@0x8bccb), before goal
        // 4's drops: goal 3 on +0x70.
        let npcs = [outnumbered(warship(Goal::Idle, Some(n(2)), 5.0)), ally()];
        let (decision, asked) = warship_decided(&npcs, WARSHIPS_RETREAT, RuleSource::Engine);
        assert_eq!(decision.goal, Goal::Flee(n(2)));
        assert!(asked.is_empty());
    }

    #[test]
    fn an_outnumbered_warship_attacking_a_disabled_ship_drops_it_rather_than_flee() {
        // The own drop jumps past the retreats (@0x8bcb4 → @0x8bf8c).
        let mut wreck = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        wreck.condition = Condition::Disabled;
        let npcs = [
            outnumbered(warship(Goal::Attack(n(2)), Some(n(2)), 0.0)),
            wreck,
        ];
        let (decision, _) = warship_decided(&npcs, WARSHIPS_RETREAT, RuleSource::Engine);
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
    }

    #[test]
    fn a_fully_dry_warship_on_its_ally_drops_it_and_its_travel() {
        // The fully-dry drop (@0x8c0e7-0x8c117: goal 0, sub 0, +0x70 and
        // +0x6c -1) comes before the high level, so no ally drop follows.
        let dry = short_of_ammo(warship(Goal::Idle, Some(n(2)), 5.0), AmmoLeft::Dry, false);
        let decision = by_engine(&[dry, ally()]);
        assert_eq!(
            (decision.goal, decision.travel, decision.clears_target),
            (Goal::Idle, TravelWrite::Clear, true)
        );
    }

    #[test]
    fn a_held_warship_holding_a_target_unprovoked_turns_on_it() {
        // Goal 0xa with +0x70 set reaches the late goal-4 write
        // (@0x8be09-0x8be2d) past the own drop, which needs goal 4.
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        for goal in [Goal::Hold, Goal::Drift] {
            let npcs = [warship(goal, Some(n(2)), 0.0), neutral.clone()];
            let (decision, asked) = warship_decided(&npcs, 0, RuleSource::Engine);
            assert_eq!(decision.goal, Goal::Attack(n(2)), "{goal:?}");
            assert!(asked.is_empty(), "{goal:?}: no search");
            let mut warping = npcs.clone();
            warping[0].warp.stage = 10.0;
            let (decision, asked) = warship_decided(&warping, 0, RuleSource::Engine);
            assert_eq!(decision.goal, goal, "{goal:?} warping: no write");
            assert!(asked.is_empty(), "{goal:?} warping: no search");
        }
        let npcs = [
            warship(Goal::Hold, None, 0.0),
            ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0),
        ];
        assert_eq!(by_engine(&npcs).goal, Goal::Hold, "no target: it holds");
    }

    #[test]
    fn a_held_warship_holding_a_target_unprovoked_retreats_for_its_shields_but_not_its_odds() {
        // The odds retreat needs goal 4 (@0x8bccb), before the late write;
        // the shield retreat comes after it (@0x8be36-0x8bf35).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let odds = [
            outnumbered(warship(Goal::Hold, Some(n(2)), 0.0)),
            neutral.clone(),
        ];
        let (decision, _) = warship_decided(&odds, WARSHIPS_RETREAT, RuleSource::Engine);
        assert_eq!(
            decision.goal,
            Goal::Attack(n(2)),
            "the odds retreat skipped"
        );
        let mut worn = warship(Goal::Hold, Some(n(2)), 0.0);
        worn.reserves.shield.now = 4.4;
        let (decision, _) = warship_decided(&[worn, neutral], WARSHIPS_RETREAT, RuleSource::Engine);
        assert_eq!(decision.goal, Goal::Flee(n(2)), "the shield retreat");
    }

    #[test]
    fn a_warship_warping_out_with_no_target_searches_nothing() {
        // With +0x70 -1 and +0x50 above 0 the goal-0 block skips
        // `_SelectWarshipTarget` and the idle block (@0x8b865-0x8b86c), and
        // so do goals 1, 0x14 and 2 (@0x8bb8a-0x8bb91).
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::JumpOut,
        ] {
            let mut warping = warship(goal, None, 0.0);
            warping.warp.stage = 10.0;
            let (decision, asked) = warship_decided(&[warping], 0, RuleSource::Engine);
            assert_eq!(decision.goal, goal, "{goal:?}");
            assert_eq!(
                decision.travel,
                TravelWrite::Keep,
                "{goal:?}: no idle block"
            );
            assert!(asked.is_empty(), "{goal:?}: no search");
        }
        let mut waiting = warship(Goal::Wait, Some(n(2)), 0.0);
        waiting.warp.stage = 10.0;
        let (decision, _) = warship_decided(&[waiting], 0, RuleSource::Engine);
        assert_eq!(
            decision.goal,
            Goal::Attack(P),
            "the wait searches whatever +0x50 (@0x8bbab-0x8bbc2)"
        );
    }

    #[test]
    fn a_warship_searches_only_from_goals_0_1_0x14_2_and_6() {
        // `_SelectWarshipTarget` runs for goals 0 (@0x8b875), 1, 0x14 and 2
        // (@0x8bb96) and 6 (@0x8bbc2) alone: in any other goal with +0x70
        // -1 the think jumps to @0x8bf8c (@0x8bc48), keeping it.
        for goal in [Goal::Drift, Goal::Mine, Goal::Scoop] {
            let (decision, asked) =
                warship_decided(&[warship(goal, None, 0.0)], 0, RuleSource::Engine);
            assert_eq!(decision.goal, goal, "{goal:?}");
            assert!(asked.is_empty(), "{goal:?}: no search");
        }
    }

    #[test]
    fn a_warship_in_any_other_goal_with_a_target_takes_the_late_goal_4_write() {
        // Any goal but 7 and 3 with +0x70 set and +0x50 at or below 0
        // (@0x8be09-0x8be2d).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        for goal in [Goal::Mine, Goal::Scoop, Goal::CountWait] {
            let npcs = [warship(goal, Some(n(2)), 0.0), neutral.clone()];
            assert_eq!(by_engine(&npcs).goal, Goal::Attack(n(2)), "{goal:?}");
        }
    }

    #[test]
    fn a_warship_warping_with_a_target_retreats_for_its_shields_but_leaving_does_not() {
        // With +0x50 above 0 no goal-4 write comes (@0x8be24-0x8be2b), but
        // the shield retreat runs for any goal but 2, 3 and 0xb
        // (@0x8bf19-0x8bf33).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        for (goal, flees) in [
            (Goal::Idle, true),
            (Goal::Hold, true),
            (Goal::Land(StellarId(128)), true),
            (Goal::JumpOut, false),
        ] {
            let mut worn = warship(goal, Some(n(2)), 0.0);
            worn.reserves.shield.now = 4.4;
            worn.warp.stage = 10.0;
            let (decision, _) = warship_decided(
                &[worn, neutral.clone()],
                WARSHIPS_RETREAT,
                RuleSource::Engine,
            );
            let expected = if flees { Goal::Flee(n(2)) } else { goal };
            assert_eq!(decision.goal, expected, "{goal:?}");
        }
    }

    #[test]
    fn a_held_warship_whose_kept_target_breaks_up_drops_it() {
        // Past the own drop, the late write reaches goal 4's breaking-up
        // drop, which clears +0x70 and +0x96 (@0x8e3ef → @0x8e714-0x8e726).
        let mut dying = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING_LONG;
        let decision = by_engine(&[warship(Goal::Hold, Some(n(2)), 0.0), dying]);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, true)
        );
    }
}
