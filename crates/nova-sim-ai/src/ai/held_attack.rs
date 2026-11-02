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
//! Then `_HighLevelAIHandler` takes goal 4 as it takes every type's: the
//! slot test makes a gone +0x70 -1 (@0x8d4e4-0x8d50d,
//! [`handler_head::slot_test`](super::handler_head::slot_test)), and the
//! state-4 block drops an ally (keeping +0x70 and +0x96), a fleet-mate
//! (+0x70 -1) or a ship breaking up (+0x70 -1, +0x96 0), and snipes a
//! hopeless chase ([`attack_drop`](super::attack_drop)). The think's own
//! drop, the disabled one, keeps +0x96. The inactive-slot branch
//! (@0x8e3df → @0x8e714) can't be reached, the slot test having come
//! first, nor can goal 4's +0x4c test (@0x8e26d), behind the handler's
//! head gate (@0x8d4b9-0x8d4d2).
//!
//! The engine's own drop stands beside these: a player paid off
//! ([`Npc::spared`]; in the original a bribe makes the ship of AI type 1,
//! no interceptor at all) and, by the Bible's reading of
//! [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks),
//! a blessed player ([`releases_blessed`](super::warship::releases_blessed)),
//! each clearing the target as the think's own drop does.
//!
//! **The kept target.** The ally drop keeps +0x70
//! ([`Decision::holds_target`](super::Decision::holds_target)), and
//! provocation never decays ([`provocation`](crate::traffic::provocation)):
//! only a drop zeroes it, and the ally drop doesn't. So each think writes
//! goal 4 on the ally again and the ally drop idles it again, with no
//! search, for as long as the ally stays in its slot; once it leaves, the
//! slot let-go clears +0x70 and the next think searches. An unprovoked
//! interceptor idle with +0x70 set searches, inspects and patrols nothing
//! either (@0x8cc93-0x8ccc9 → @0x8d13f, [`searches_nothing`]), holding its
//! target, or losing it with its slot gone (@0x8cca7-0x8ccb8): its
//! out-of-ammo retreat alone still runs, fleeing +0x70
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
//! - **Goal 4's high level** is the state-4 block's
//!   ([`attack_drop`](super::attack_drop)): the ally drop keeps +0x70 and
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
//! A fresh pick, a word's player or a held target that the own drop takes
//! jumps past the idle block too (@0x8bcb4 → @0x8bf8c,
//! [`Hunt::dropped`](super::warship::Hunt::dropped)). `_WarshipAI`'s sub-4 rewrite to goal 2
//! (@0x8b80f-0x8b82c) can't be reached: sub 4 or 0xd takes `_AIDispatch`'s
//! jump path (@0x8fd83-0x8fd9a → @0x900d5), which skips the think.
//!
//! Nothing here plays a sound or draws a random number.

use crate::ai::interceptor::{gate_closed, searches_nothing};
use crate::ai::warship::{Hunt, HuntTarget, Retreat, keeps, releases_blessed, retreats};
use crate::ai::{Goal, Surroundings, fire, out_of_ammo};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// Whether `npc`'s think lets `ship`, the player, go by the engine's own
/// drop (see the module docs): paid off, or blessed by `always_attacks`.
fn paid_off(npc: &Npc, ship: ShipRef, around: &Surroundings, always_attacks: RuleSource) -> bool {
    (ship == ShipRef::Player && npc.spared) || releases_blessed(npc, ship, around, always_attacks)
}

/// What an interceptor `npc` does with goal 4 on `ship` among `around`,
/// retreating as `retreat` says, a blessed player let go as
/// `always_attacks` says: the think's tail (see the module docs), the
/// high level's drops left to the state-4 block. Judged as attacking
/// `ship` whatever goal it held, as the provoked branch writes goal 4
/// first.
#[must_use]
pub fn interceptor_attack(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    retreat: Retreat,
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
            forgets_provocation: dry,
            ..Hunt::dropped(false)
        };
    }
    if dry {
        return Hunt::of(Goal::Flee(ship));
    }
    if paid_off(npc, ship, around, always_attacks) {
        return Hunt::dropped(false);
    }
    Hunt::of(Goal::Attack(ship))
}

/// What an interceptor `npc`'s think does among `around` before its
/// search, by `retreat` and `always_attacks`, when it holds no word and
/// flies no flight it keeps (see the module docs): provoked with a
/// target, any goal but a flight, an inspection or a help, goal 4 on it
/// ([`interceptor_attack`]); unprovoked in goal 4, the same on the ship
/// it attacks or snipes at; and nothing to fight, searching nothing, idle
/// with a target it kept ([`searches_nothing`]), holding it while its
/// slot is active and losing it otherwise (@0x8cca7-0x8ccb8), or behind a
/// closed gate ([`gate_closed`]). None leaves it the hunt's own path: an
/// inspection this think ends.
#[must_use]
pub fn interceptor_hunt(
    npc: &Npc,
    around: &Surroundings,
    retreat: Retreat,
    always_attacks: RuleSource,
) -> Option<Hunt> {
    let attack = |ship| interceptor_attack(npc, ship, around, retreat, always_attacks);
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
    if searches_nothing(npc)
        && let Some(ship) = npc.target
    {
        let target = if around.active(ship) {
            HuntTarget::Holds(ship)
        } else {
            HuntTarget::Cleared
        };
        return Some(Hunt {
            target,
            ..Hunt::default()
        });
    }
    gate_closed(npc, around).then(Hunt::default)
}

/// What a warship `npc` does with goal 4 on `ship` among `around`,
/// retreating as `retreat` says, a blessed player let go as
/// `always_attacks` says, in `_WarshipAI`'s order (see the module docs):
/// its own drop (goal 0, +0x70 -1, past the retreats and the idle block,
/// @0x8bca0-0x8bcb4, [`keeps`]), then its retreats judged as goal 4
/// (@0x8bcba-0x8bf83), then the attack, which its fully-dry drop
/// ([`Warship`](super::Warship)'s decision) and the high level's state-4
/// block take next.
#[must_use]
pub fn warship_attack(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    retreat: Retreat,
    always_attacks: RuleSource,
) -> Hunt {
    if !keeps(npc, ship, around, retreat) || paid_off(npc, ship, around, always_attacks) {
        return Hunt::dropped(false);
    }
    let judged = Npc {
        goal: Goal::Attack(ship),
        ..npc.clone()
    };
    if retreats(&judged, around, retreat) || (retreat.out_of_ammo && out_of_ammo::retreats(npc)) {
        return Hunt::of(Goal::Flee(ship));
    }
    Hunt::of(Goal::Attack(ship))
}

/// What a warship `npc`'s think does among `around` before its search, by
/// `retreat` and `always_attacks`, when it holds no word and flies no
/// flight (see the module docs), in `_WarshipAI`'s order:
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
///    2 (@0x8bf19-0x8bf33), or its goal kept, holding the target;
/// 6. unprovoked idle, landing, entering a gate or leaving with a target,
///    goal 4 on it (@0x8b843-0x8b860, @0x8bb6e-0x8bba2);
/// 7. unprovoked in any other goal with a target, holding or drifting
///    among them, the late goal-4 write (@0x8be09-0x8be2d): past its own
///    drop and its odds retreat, which need goal 4, its shield and
///    out-of-ammo retreats judged as the goal it holds, then the attack,
///    which its fully-dry drop and the state-4 block take next;
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
    always_attacks: RuleSource,
) -> Option<Hunt> {
    let attack = |ship| warship_attack(npc, ship, around, retreat, always_attacks);
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
        // @0x8bb75-0x8bb7c, @0x8be24-0x8be2b), and +0x70 stands.
        Some(ship) if warping => Some(worn(ship).unwrap_or(Hunt {
            target: HuntTarget::Holds(ship),
            ..Hunt::of(npc.goal)
        })),
        // Goal 4 at once (@0x8b843-0x8b860, @0x8bba2).
        Some(ship) if searches => Some(attack(ship)),
        // The late write (@0x8be09-0x8be2d), past the own drop and the
        // odds retreat.
        Some(ship) => Some(worn(ship).unwrap_or(Hunt::of(Goal::Attack(ship)))),
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
    use crate::govt::{ALWAYS_ATTACKS_PLAYER, INTERCEPTORS_RETREAT};
    use crate::traffic::npc::{AiType, NpcId};

    const P: ShipRef = ShipRef::Player;

    /// A police interceptor (NPC 1) at the centre, attacking `ship`.
    fn police(ship_ref: ShipRef) -> Npc {
        let mut cop = ship(1, POLICE, AiType::Interceptor, 0.0, 0.0);
        cop.goal = Goal::Attack(ship_ref);
        cop.target = Some(ship_ref);
        cop
    }

    const DYING: Condition = Condition::Dying {
        ticks_left: 3,
        starting: false,
    };

    #[test]
    fn by_the_bible_a_blessed_player_is_let_go_and_by_the_engine_it_is_not() {
        // The engine's own drop, as the paid-off player's (see the module
        // docs).
        let govts = govts(0, ALWAYS_ATTACKS_PLAYER);
        let sites = sites();
        let npcs = [police(P)];
        let mut around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        if let Some(player) = around.player.as_mut() {
            player.blessing = blessing_of(POLICE);
        }
        let attack = |always_attacks| {
            interceptor_attack(&npcs[0], P, &around, INTERCEPTOR_RETREAT, always_attacks)
        };
        assert_eq!(attack(RuleSource::Bible), Hunt::dropped(false));
        assert_eq!(attack(RuleSource::Engine), Hunt::of(Goal::Attack(P)));
        let mut bribed = police(P);
        bribed.spared = true;
        let npcs = [bribed];
        let govts = crate::ai::fixture::govts(0, 0);
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
        assert_eq!(
            interceptor_attack(
                &npcs[0],
                P,
                &around,
                INTERCEPTOR_RETREAT,
                RuleSource::Engine
            ),
            Hunt::dropped(false),
            "paid off"
        );
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
            (hunt.goal, hunt.target, hunt.forgets_provocation),
            (Some(Goal::Idle), HuntTarget::Cleared, false)
        );
        let dry = short_of_ammo(police(n(2)), AmmoLeft::Dry, true);
        let hunt = attacked(&dry, &disabled_pirate(), 0);
        assert_eq!(
            (hunt.goal, hunt.target, hunt.forgets_provocation),
            (Some(Goal::Idle), HuntTarget::Cleared, true),
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
    fn the_think_leaves_goal_4_to_the_high_level_which_drops_or_flies_it() {
        // The think's tail has no ally, fleet or breaking-up drop: the
        // state-4 block's (@0x8e277-0x8e2db, @0x8e3ef → @0x8e714-0x8e726).
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING;
        let ally = ship(2, TRADERS, AiType::Warship, 0.0, 300.0);
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        for quarry in [&dying, &ally, &pirate] {
            assert_eq!(
                attacked(&police(n(2)), quarry, 0),
                Hunt::of(Goal::Attack(n(2))),
                "{:?}",
                quarry.govt
            );
        }
        let (govts, sites) = (govts(0, 0), sites());
        let flown = |quarry: &Npc| {
            let npcs = [police(n(2)), quarry.clone()];
            let around = around(&sites, &npcs, &govts, (0.0, -1000.0), 0);
            NovaAi::default().decision(&npcs[0], &around, &mut Draws::of(&[0; 4]))
        };
        let dropped = flown(&dying);
        assert_eq!(
            (
                dropped.goal,
                dropped.clears_target,
                dropped.forgets_provocation
            ),
            (Goal::Idle, true, true)
        );
        let spared = flown(&ally);
        assert_eq!(
            (spared.goal, spared.holds_target, spared.forgets_provocation),
            (Goal::Idle, Some(n(2)), false)
        );
        assert_eq!(flown(&pirate).goal, Goal::Attack(n(2)));
    }

    // A warship's goal 4 (`_WarshipAI` @0x8b835-0x8c117, then goal 4's
    // high level), through its whole decision.

    use crate::ai::travel::TravelWrite;
    use crate::ai::{Behaviour, Decision, NovaAi, Warship, attack_drop};
    use crate::govt::WARSHIPS_RETREAT;
    use crate::rulebook::{RuleKey, Rulebook};
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
    /// `police_flags`, with a wanted player 300 below that its search would
    /// take; and the draws asked.
    fn warship_decided(npcs: &[Npc], police_flags: u16) -> (Decision, Vec<u32>) {
        let govts = govts(0, police_flags);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -300.0), -10);
        let warship = Warship::default();
        let mut chance = Draws::of(&[0; 4]);
        let decision = warship.decision(&npcs[0], &around, &mut chance);
        (decision, chance.asked)
    }

    fn by_engine(npcs: &[Npc]) -> Decision {
        warship_decided(npcs, 0).0
    }

    /// [`warship_decided`]'s decision through the state-4 block, by
    /// `led_ally` ([`attack_drop::attack_block`]).
    fn warship_blocked(
        npcs: &[Npc],
        police_flags: u16,
        led_ally: RuleSource,
    ) -> (Decision, Vec<u32>) {
        let (decision, asked) = warship_decided(npcs, police_flags);
        let govts = govts(0, police_flags);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -300.0), -10);
        (
            attack_drop::attack_block(&npcs[0], decision, &around, led_ally),
            asked,
        )
    }

    /// The decision of `npcs[0]` through Nova's AI, by `led_ally`, as
    /// [`warship_decided`] places them.
    fn nova_decided(npcs: &[Npc], led_ally: RuleSource) -> (Decision, Vec<u32>) {
        let govts = govts(0, 0);
        let sites = sites();
        let around = around(&sites, npcs, &govts, (0.0, -300.0), -10);
        let ai = NovaAi::from_rulebook(
            &Rulebook::default().with_override(RuleKey::LedAllyAttack, led_ally),
        );
        let mut chance = Draws::of(&[0; 4]);
        let decision = ai.decision(&npcs[0], &around, &mut chance);
        (decision, chance.asked)
    }

    #[test]
    fn a_warship_provoked_by_an_ally_idles_keeping_it_with_no_search() {
        // The provoked branch writes goal 4 with no ally test
        // (@0x8bb0c-0x8bb44), and the state-4 block's ally drop idles it,
        // keeping +0x70 and +0x96 (@0x8e277-0x8e2db → @0x8fb47).
        for led_ally in [RuleSource::Engine, RuleSource::Bible] {
            let npcs = [warship(Goal::Idle, Some(n(2)), 5.0), ally()];
            let (decision, asked) = nova_decided(&npcs, led_ally);
            assert_eq!(decision.goal, Goal::Idle, "{led_ally:?}");
            assert_eq!(decision.holds_target, Some(n(2)), "{led_ally:?}");
            assert!(!decision.forgets_provocation, "{led_ally:?}");
            assert_eq!(decision.travel, TravelWrite::Keep, "{led_ally:?}");
            assert!(asked.is_empty(), "{led_ally:?}: no search");
        }
    }

    #[test]
    fn a_provoked_warship_turns_on_its_attacker_from_a_wait_or_a_hold() {
        // The provoked branch writes goal 4 for any goal but 7 and 3
        // (@0x8bb26-0x8bb44), before the wait's search (@0x8bbb9) and the
        // odds retreat that needs goal 4 (@0x8bccb).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let npcs = [warship(Goal::Wait, Some(n(2)), 5.0), neutral.clone()];
        let (decision, asked) = warship_decided(&npcs, 0);
        assert_eq!(decision.goal, Goal::Attack(n(2)), "not the wait's search");
        assert!(asked.is_empty());
        let npcs = [outnumbered(warship(Goal::Hold, Some(n(2)), 5.0)), neutral];
        let (decision, _) = warship_decided(&npcs, WARSHIPS_RETREAT);
        assert_eq!(decision.goal, Goal::Flee(n(2)), "judged as goal 4");
    }

    #[test]
    fn an_unprovoked_idle_warship_with_a_kept_target_takes_goal_4_on_it_with_no_search() {
        // The goal-0 block writes goal 4 on a +0x70 that is set
        // (@0x8b843-0x8b860), with no `_SelectWarshipTarget` (@0x8b875).
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let npcs = [warship(Goal::Idle, Some(n(2)), 0.0), neutral];
        let (decision, asked) = warship_decided(&npcs, 0);
        assert_eq!(decision.goal, Goal::Attack(n(2)));
        assert!(asked.is_empty(), "no search");
        let npcs = [warship(Goal::Idle, Some(n(2)), 0.0), ally()];
        let (decision, asked) = warship_blocked(&npcs, 0, RuleSource::Engine);
        assert_eq!(
            (decision.goal, decision.holds_target),
            (Goal::Idle, Some(n(2))),
            "an ally it spares is idled on again"
        );
        assert!(asked.is_empty());
    }

    #[test]
    fn an_idle_warship_with_a_kept_target_takes_it_through_its_own_drop_and_odds_retreat() {
        // The goal-0 block's goal 4 comes before the own drop (@0x8bc4e) and
        // the odds retreat (@0x8bccb), which the late write skips.
        let mut wreck = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        wreck.condition = Condition::Disabled;
        let decision = by_engine(&[warship(Goal::Idle, Some(n(2)), 0.0), wreck]);
        assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        let npcs = [outnumbered(warship(Goal::Idle, Some(n(2)), 0.0)), neutral];
        let (decision, _) = warship_decided(&npcs, WARSHIPS_RETREAT);
        assert_eq!(decision.goal, Goal::Flee(n(2)));
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
            let (decision, asked) = warship_decided(&warping, 0);
            assert_eq!(decision.goal, goal, "{goal:?} warping");
            assert!(asked.is_empty(), "{goal:?} warping: no search");
        }
        let mut idle = [warship(Goal::Idle, Some(n(2)), 0.0), neutral];
        idle[0].warp.stage = 10.0;
        let (decision, asked) = warship_decided(&idle, 0);
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
                let (decision, asked) = warship_decided(&npcs, 0);
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
        // The state-4 block's fleet drop: goal 0, sub 0, +0x70 -1
        // (@0x8e2e9-0x8e399), here a fellow follower of NPC 3 (its parent
        // is the target's, @0x8e322).
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        escort.leader = Some(NpcId(3));
        let mut hunter = warship(Goal::Attack(n(2)), Some(n(2)), 0.0);
        hunter.leader = Some(NpcId(3));
        let npcs = [hunter, escort];
        let (decision, asked) = warship_blocked(&npcs, 0, RuleSource::Engine);
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
        let (decision, asked) = warship_decided(&npcs, WARSHIPS_RETREAT);
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
        let (decision, _) = warship_decided(&npcs, WARSHIPS_RETREAT);
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
            let (decision, asked) = warship_decided(&npcs, 0);
            assert_eq!(decision.goal, Goal::Attack(n(2)), "{goal:?}");
            assert!(asked.is_empty(), "{goal:?}: no search");
            let mut warping = npcs.clone();
            warping[0].warp.stage = 10.0;
            let (decision, asked) = warship_decided(&warping, 0);
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
        let (decision, _) = warship_decided(&odds, WARSHIPS_RETREAT);
        assert_eq!(
            decision.goal,
            Goal::Attack(n(2)),
            "the odds retreat skipped"
        );
        let mut worn = warship(Goal::Hold, Some(n(2)), 0.0);
        worn.reserves.shield.now = 4.4;
        let (decision, _) = warship_decided(&[worn, neutral], WARSHIPS_RETREAT);
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
            let (decision, asked) = warship_decided(&[warping], 0);
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
        let (decision, _) = warship_decided(&[waiting], 0);
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
            let (decision, asked) = warship_decided(&[warship(goal, None, 0.0)], 0);
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
            let (decision, _) = warship_decided(&[worn, neutral.clone()], WARSHIPS_RETREAT);
            let expected = if flees { Goal::Flee(n(2)) } else { goal };
            assert_eq!(decision.goal, expected, "{goal:?}");
        }
    }

    #[test]
    fn a_held_warship_whose_kept_target_breaks_up_drops_it() {
        // Past the own drop, the late write reaches the state-4 block's
        // breaking-up drop, which clears +0x70 and +0x96 (@0x8e3ef →
        // @0x8e714-0x8e726).
        let mut dying = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        dying.condition = DYING_LONG;
        let npcs = [warship(Goal::Hold, Some(n(2)), 0.0), dying];
        assert_eq!(by_engine(&npcs).goal, Goal::Attack(n(2)), "the think's");
        let (decision, _) = warship_blocked(&npcs, 0, RuleSource::Engine);
        assert_eq!(
            (
                decision.goal,
                decision.clears_target,
                decision.forgets_provocation
            ),
            (Goal::Idle, true, true)
        );
    }

    /// The target a warship or an interceptor holds as it thinks (+0x70):
    /// how each think takes it up again after the state-4 block's ally drop,
    /// and what it does with one it drops (`_WarshipAI` @0x8b729,
    /// `_InterceptorAI` @0x8c896).
    mod held {
        use crate::ai::fixture::{NEUTRAL, POLICE, TRADERS, around, govts, n, ship, sites};
        use crate::ai::interceptor::Interceptor;
        use crate::ai::warship::Warship;
        use crate::ai::{Behaviour, Decision, Goal, NovaAi};
        use crate::catalog::StellarId;
        use crate::combat::hull::Condition;
        use crate::traffic::npc::{AiType, Npc};
        use nova_sim_base::fixture::Draws;

        const DYING: Condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };

        /// A police ship of `ai_type` (NPC 1) at the centre, holding `target`
        /// in `goal`, unprovoked.
        fn holding(ai_type: AiType, goal: Goal, target: u32) -> Npc {
            Npc {
                goal,
                target: Some(n(target)),
                ..ship(1, POLICE, ai_type, 0.0, 0.0)
            }
        }

        /// An allied trader (NPC 3) 500 below.
        fn trader() -> Npc {
            ship(3, TRADERS, AiType::WimpyTrader, 0.0, 500.0)
        }

        /// `behaviour`'s decision for `npcs[0]` among `npcs`, the player far
        /// off and clean, and the draws it asked.
        fn decided_by(behaviour: &dyn Behaviour, npcs: &[Npc]) -> (Decision, Vec<u32>) {
            let (sites, govts) = (sites(), govts(0, 0));
            let around = around(&sites, npcs, &govts, (0.0, -5000.0), 0);
            let mut chance = Draws::of(&[]);
            let decision = behaviour.decision(&npcs[0], &around, &mut chance);
            (decision, chance.asked)
        }

        fn warship(npcs: &[Npc]) -> Decision {
            decided_by(&Warship::default(), npcs).0
        }

        #[test]
        fn an_unprovoked_warship_holding_an_allied_npc_attacks_it() {
            // State 0 with +0x70 not -1 takes state 4 at once (@0x8b843-0x8b860).
            let npcs = [holding(AiType::Warship, Goal::Idle, 3), trader()];
            assert_eq!(warship(&npcs).goal, Goal::Attack(n(3)));
        }

        #[test]
        fn a_travelling_warship_holding_a_target_attacks_it() {
            // States 1, 0x14 and 2 (@0x8bb6e-0x8bba2).
            let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
            for goal in [
                Goal::Land(StellarId(128)),
                Goal::EnterGate(StellarId(128)),
                Goal::JumpOut,
            ] {
                let npcs = [holding(AiType::Warship, goal, 3), neutral.clone()];
                assert_eq!(warship(&npcs).goal, Goal::Attack(n(3)), "{goal:?}");
            }
        }

        #[test]
        fn a_warship_whose_warp_is_under_way_never_attacks_the_target_it_holds() {
            // +0x50 above 0 (@0x8b843-0x8b860).
            let mut leaving = holding(AiType::Warship, Goal::JumpOut, 3);
            leaving.warp.stage = 1.0;
            let npcs = [leaving, ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0)];
            assert_eq!(warship(&npcs).goal, Goal::JumpOut);
        }

        #[test]
        fn a_warship_holding_a_target_it_drops_ends_idle_with_no_target_and_no_landing() {
            // The drop (@0x8bc4e-0x8bcaf) jumps past the idle block (@0x8bcb4
            // → @0x8bf8c).
            let dropped = Decision {
                goal: Goal::Idle,
                clears_target: true,
                ..Decision::default()
            };
            let gone = [holding(AiType::Warship, Goal::Idle, 9)];
            assert_eq!(warship(&gone), dropped, "gone");
            let dying = [
                holding(AiType::Warship, Goal::Idle, 3),
                Npc {
                    condition: DYING,
                    ..trader()
                },
            ];
            assert_eq!(warship(&dying), dropped, "breaking up");
            let harmless = [
                holding(AiType::Warship, Goal::Land(StellarId(128)), 3),
                Npc {
                    condition: Condition::Disabled,
                    ..trader()
                },
            ];
            assert_eq!(
                warship(&harmless),
                dropped,
                "disabled, no destroying weapons"
            );
        }

        #[test]
        fn a_warship_provoked_by_a_ship_it_drops_ends_idle_with_no_target() {
            // The provoked branch (@0x8bb0c-0x8bb44) takes state 4, and the drop
            // lets it go.
            let mut provoked = holding(AiType::Warship, Goal::Idle, 3);
            provoked.provoked = 5.0;
            let npcs = [
                provoked,
                Npc {
                    condition: Condition::Disabled,
                    ..ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0)
                },
            ];
            let decision = warship(&npcs);
            assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
        }

        fn interceptor(npcs: &[Npc]) -> (Decision, Vec<u32>) {
            decided_by(&Interceptor::default(), npcs)
        }

        #[test]
        fn an_unprovoked_interceptor_whose_held_target_is_gone_keeps_its_goal_with_no_target() {
            // The slot test clears a freed slot's +0x70 after the think
            // (@0x8d4e4-0x8d50d).
            let goal = Goal::Land(StellarId(128));
            let (decision, asked) = interceptor(&[holding(AiType::Interceptor, goal, 9)]);
            assert_eq!(
                (decision.goal, decision.clears_target, decision.holds_target),
                (goal, true, None)
            );
            assert_eq!(asked, Vec::<u32>::new(), "no inspection, no patrol");
        }

        #[test]
        fn an_interceptor_out_of_ammunition_holding_a_target_flees_it_and_with_none_stops() {
            // The out-of-ammo flight (@0x8d343-0x8d391) writes goal 3 on the
            // goal it keeps, +0x70 as it stands: it flees the target it holds,
            // and with none it stops, state 3 to 0, its provocation zeroed
            // (@0x8dff6 → @0x8e228-0x8e23d).
            let dry = crate::ai::fixture::short_of_ammo(
                holding(AiType::Interceptor, Goal::Land(StellarId(128)), 3),
                crate::ai::fire::AmmoLeft::OnlyUnlimited,
                true,
            );
            let (decision, _) = interceptor(&[dry.clone(), trader()]);
            assert_eq!(
                (
                    decision.goal,
                    decision.clears_target,
                    decision.forgets_provocation
                ),
                (Goal::Flee(n(3)), false, false)
            );
            // With none, behind the closed gate of a hold (no inspection to
            // name one, @0x8cc6d-0x8cc7d), it stops.
            let lone = Npc {
                goal: Goal::Hold,
                target: None,
                ..dry
            };
            let (decision, _) = interceptor(&[lone]);
            assert_eq!(
                (
                    decision.goal,
                    decision.clears_target,
                    decision.holds_target,
                    decision.forgets_provocation
                ),
                (Goal::Idle, true, None, true)
            );
        }

        #[test]
        fn an_unprovoked_interceptor_holding_an_ally_keeps_it_and_inspects_nothing() {
            // +0x70 not -1 searches nothing and jumps past the inspection and
            // the patrol (@0x8cc93 → @0x8d13f → @0x8d14d).
            for goal in [
                Goal::Idle,
                Goal::Land(StellarId(128)),
                Goal::EnterGate(StellarId(128)),
            ] {
                let npcs = [holding(AiType::Interceptor, goal, 3), trader()];
                let (decision, asked) = interceptor(&npcs);
                assert_eq!(
                    (decision.goal, decision.holds_target),
                    (goal, Some(n(3))),
                    "{goal:?}"
                );
                assert_eq!(asked, Vec::<u32>::new(), "no inspection pick, {goal:?}");
            }
        }

        #[test]
        fn a_provoked_interceptor_attacks_the_ally_it_holds() {
            // Active and provoked, state 4 (@0x8ccb0-0x8cccf).
            let mut provoked = holding(AiType::Interceptor, Goal::Idle, 3);
            provoked.provoked = 5.0;
            let (decision, _) = interceptor(&[provoked, trader()]);
            assert_eq!(decision.goal, Goal::Attack(n(3)));
        }

        #[test]
        fn an_interceptor_provoked_by_a_ship_it_lets_go_ends_idle_with_no_target_or_patrol() {
            // The drop (@0x8d2fd-0x8d33a) jumps past the patrol.
            let mut provoked = holding(AiType::Interceptor, Goal::Idle, 3);
            provoked.provoked = 5.0;
            let downed = Npc {
                condition: Condition::Disabled,
                ..trader()
            };
            let (decision, asked) = interceptor(&[provoked, downed]);
            assert_eq!((decision.goal, decision.clears_target), (Goal::Idle, true));
            assert_eq!(asked, Vec::<u32>::new(), "no inspection, no patrol");
        }

        /// `npcs[0]`'s decision through Nova's AI.
        fn flown(npcs: &[Npc]) -> Decision {
            decided_by(&NovaAi::default(), npcs).0
        }

        /// `npc` as the frame `decision` ends leaves it: its goal, and its
        /// target held or cleared.
        fn after(npc: &Npc, decision: &Decision) -> Npc {
            Npc {
                goal: decision.goal,
                target: decision.last_target().unwrap_or(decision.goal.quarry()),
                ..npc.clone()
            }
        }

        #[test]
        fn a_hunter_provoked_by_an_ally_stays_idle_holding_it_think_after_think_until_it_goes() {
            for ai_type in [AiType::Warship, AiType::Interceptor] {
                let mut provoked = holding(ai_type, Goal::Idle, 3);
                provoked.provoked = 5.0;
                let mut npcs = [provoked, trader()];
                for think in 0..2 {
                    let decision = flown(&npcs);
                    assert_eq!(
                        (decision.goal, decision.holds_target),
                        (Goal::Idle, Some(n(3))),
                        "{ai_type:?}, think {think}: the block's ally drop (@0x8e2d2)"
                    );
                    assert!(!decision.forgets_provocation, "{ai_type:?}");
                    npcs[0] = after(&npcs[0], &decision);
                }
                let gone = [npcs[0].clone()];
                let decision = flown(&gone);
                assert_eq!(
                    (decision.goal, decision.clears_target),
                    (Goal::Idle, true),
                    "{ai_type:?}: gone"
                );
                let searching = [after(&gone[0], &decision)];
                assert_eq!(searching[0].target, None);
                assert_ne!(flown(&searching).goal, Goal::Idle, "{ai_type:?} searches");
            }
        }
    }
}
