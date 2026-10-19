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
//!   nothing to fight.
//! - Idle with nothing to fight, it inspects (@0x8ce34): it flies up to a
//!   ship picked at random among the others in the system (the player
//!   first, then the NPCs in order), but not the one it last inspected
//!   nor another interceptor (AI type 4, @0x8cd4e, @0x8cdfe: a ship with
//!   no AI, [`Npc::no_ai`], reads as none, whatever its class); the
//!   original retries `Rand(64)` slots, here one draw among them. Within [`INSPECT_REACH`] of it on both axes
//!   (@0x8f11e, [`within_reach`]) it has looked it over, and decides
//!   again; the traffic remembers the ship ([`Npc::inspected`]). Having
//!   looked the player over, it has scanned the player
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
//!   (@0x8cc6d-0x8cc7d). Only provocation (@0x8cbf6-0x8cc28) or its
//!   government's word lifts the wait. Short of ammunition, a waiting
//!   interceptor goes idle as a stopped one does (state 6 to 3 to 0,
//!   @0x8d391 and @0x8e228; see [`out_of_ammo`]).
//! - Inspecting a ship it no longer sees (`_ShipVisibleToShip`, see
//!   [`sight`](super::sight)), it drops it: target -1, state 0
//!   (@0x8d39a-0x8d3ee, [`lost_sight::after_inspection`]). An inspection
//!   the out-of-ammo retreat turned into a flight is left alone.
//! - Inspecting (state 7) it runs no search of its own (the block needs
//!   state 1 or below, or 0x14, @0x8cc6d-0x8cc7d; [`hunt`] answers only
//!   its provoker) while the inspection holds ([`inspection_holds`]): one
//!   that ends this think, within reach or gone, is let go as state 0
//!   (@0x8f391) and the hunt runs its full search, attacking any pick
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
//!   player scans nothing ([`Decision::re_picked`]).

use crate::ai::fire::FireRules;
use crate::ai::odds::state4_attack;
use crate::ai::target::{TargetRules, dropped, re_pick_target};
use crate::ai::warship::{Retreat, hunt};
use crate::ai::{
    Behaviour, CommCall, Decision, Goal, Reaction, Surroundings, fire, idle, idle_block,
    lost_sight, out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::govt::INTERCEPTORS_RETREAT;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Npc};
use crate::traffic::provocation::attack_breaking_up;

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
/// one live among `around` and out of its reach ([`within_reach`]). Within
/// reach, `_HighLevelAIHandler`'s state-7 block lets it go (@0x8f11e,
/// @0x8f391: state 0, +0x70 -1), and so it does a ship gone; either way
/// the think goes on as from state 0, with the hunt's full search
/// (`_InterceptorAI` @0x8cc6d, `_SelectWarshipTarget` @0x8cce6).
#[must_use]
pub fn inspection_holds(npc: &Npc, around: &Surroundings) -> bool {
    let Goal::Inspect(ship) = npc.goal else {
        return false;
    };
    around
        .state_of(ship)
        .filter(|_| around.live(ship))
        .is_some_and(|state| !within_reach(state.position - npc.state.position))
}

/// An interceptor's retreat: when outnumbered, by `Flags` 0x0100, and
/// out of ammunition, by its hull's `Flags2` 0x0080.
pub const INTERCEPTOR_RETREAT: Retreat = Retreat {
    odds_flag: INTERCEPTORS_RETREAT,
    shields: false,
    person_coward: RuleSource::Engine,
    out_of_ammo: true,
    searches_waiting: false,
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
    /// ([`out_of_ammo::stops`]) and an attack on a ship breaking up
    /// ([`attack_breaking_up`]: `_InterceptorAI` has no drop of its own,
    /// so goal 4 reaches @0x8e714).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
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
        let decision = hunt.or(|| {
            let (goal, call) = self.patrol(npc, around, chance);
            comm_call = call;
            stops = out_of_ammo::stops(npc, goal);
            out_of_ammo::after_patrol(npc, goal)
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
        Decision {
            forgets_provocation: stops || attack_breaking_up(npc, decision.goal, around),
            ..decision
        }
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around, self.fire)
    }

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
        if matches!(npc.goal, Goal::Hold | Goal::Wait) {
            // The patrol and inspect block needs goal 1 or below, or
            // 0x14 (`_InterceptorAI` @0x8cc6d-0x8cc7d): a held ship
            // holds, and a waiting one (state 6) waits.
            return (idle(npc, around, chance), None);
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
/// pick that writes state 4 ([`Pick::attacks`](crate::ai::target::Pick::attacks)) attacks, as
/// `_HighLevelAIHandler`'s state-4 block takes it that frame: dropped at
/// once when the drop would take it (@0x8e3ef, [`dropped`]), sniped at in
/// a hopeless chase unless the hull stands off (@0x8e48e), the call on the
/// player kept. Any other pick is inspected instead of the ship it was
/// inspecting. The retreats came before (@0x8d161-0x8d2e4) and need state
/// 4, so a fresh attack isn't judged for one in the same think.
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
    let turns_on_player = decision.turns_on_player || pick.turns_on_player;
    let (goal, clears_target) = if !pick.attacks {
        (Goal::Inspect(pick.ship), false)
    } else if dropped(npc, pick.ship, around, rules) {
        (Goal::Idle, true)
    } else {
        (state4_attack(npc, pick.ship, around), false)
    };
    Decision {
        goal,
        turns_on_player,
        clears_target,
        re_picked: true,
        ..decision
    }
}

/// A ship for `npc` to inspect among `around`, picked on `chance`: the
/// player or an NPC, not itself, not another interceptor, not the one it
/// last inspected nor the one it has just looked over.
fn inspect(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Option<Goal> {
    let done = [npc.inspected, npc.goal.inspecting()];
    let player = around.live(ShipRef::Player).then_some(ShipRef::Player);
    let npcs = around
        .npcs
        .iter()
        .filter(|other| other.id != npc.id && (other.no_ai || other.ai_type != AiType::Interceptor))
        .map(|other| ShipRef::Npc(other.id))
        .filter(|&ship| around.live(ship));
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
        NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, short_of_ammo, sites,
    };
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::govt::{Governments, WARSHIPS_RETREAT};
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

    #[test]
    fn a_provoked_waiting_interceptor_attacks() {
        let mut waiting = police();
        waiting.goal = Goal::Wait;
        waiting.provoked = 1.0;
        waiting.target = Some(n(2));
        let neutral = ship(2, NEUTRAL, AiType::Warship, 0.0, 300.0);
        assert_eq!(decided(&[waiting, neutral], 0, &[]), Goal::Attack(n(2)));
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
                keeps_pursuit: false,
                plunders: None,
                lost_sight: None,
                travel: crate::ai::travel::TravelWrite::Keep,
                visits: None,
                re_picked: false,
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
        assert_eq!(
            decision_by(ALWAYS, &[busy, pirate], Condition::Disabled, 0, &[0]),
            Decision {
                comm_call: Some(CommCall::PreScan),
                ..turned(Goal::Inspect(P))
            },
            "the word lets a disabled player go: on to an inspection"
        );
        let mut held = police();
        held.goal = Goal::Hold;
        assert_eq!(
            decision_by(ALWAYS, &[held], Condition::Disabled, 0, &[]),
            turned(Goal::Hold),
            "held, the word lets a disabled player go"
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

    /// `npc` fully cloaked.
    fn cloaked(mut npc: Npc) -> Npc {
        npc.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        npc
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

    #[test]
    fn a_re_picked_attack_on_a_ship_it_drops_leaves_it_idle() {
        // `_HighLevelAIHandler`'s state-4 drop takes it that frame
        // (@0x8e3ef).
        let (cop, trader) = trader_cop_inspecting();
        let mut police_cop = cop;
        police_cop.govt = Some(POLICE);
        let mut fleeing = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 200.0);
        fleeing.goal = Goal::Flee(n(4));
        let mut wreck = ship(4, PIRATES, AiType::Warship, 0.0, 300.0);
        wreck.condition = Condition::Disabled;
        let (decision, _) = re_picked(police_cop, &[trader, fleeing, wreck], 0, None, &[]);
        assert_eq!(decision.goal, Goal::Idle);
        assert!(decision.clears_target);
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
            Interceptor::default()
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
