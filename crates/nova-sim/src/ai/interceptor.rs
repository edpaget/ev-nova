//! Nova's interceptor (AI type 4, `_InterceptorAI` @0x8c896 in the `EV
//! Nova` executable): police.
//!
//! - It hunts as a warship does ([`hunt`]), but with no shield retreat,
//!   and its odds retreat is gated on its government's `Flags` 0x0100
//!   rather than 0x0010 (@0x8d180, likely a slip in the original, kept as
//!   the default).
//! - Idle with nothing to fight, it inspects (@0x8ce34): it flies up to a
//!   ship picked at random among the others in the system (the player
//!   first, then the NPCs in order), but not the one it last inspected
//!   nor another interceptor (the original retries `Rand(64)` slots; here
//!   one draw among them). Within [`INSPECT_REACH`] of it on both axes
//!   (@0x8f11e) it has looked it over, and decides again; the traffic
//!   remembers the ship ([`Npc::inspected`]). Scanning the player for
//!   illegal cargo waits for mission cargo.
//! - With nothing to inspect it goes about its business
//!   ([`idle`](super::idle)).

use crate::ai::warship::{Retreat, hunt};
use crate::ai::{Behaviour, Goal, Reaction, Surroundings, fire, idle, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::govt::INTERCEPTORS_RETREAT;
use crate::traffic::npc::{AiType, Npc};

/// How near, in pixels on either axis, an interceptor comes to the ship
/// it inspects (100.0 @0xdd060).
pub const INSPECT_REACH: f32 = 100.0;

/// An interceptor's retreat: only when outnumbered, by `Flags` 0x0100.
pub const INTERCEPTOR_RETREAT: Retreat = Retreat {
    odds_flag: INTERCEPTORS_RETREAT,
    shields: false,
};

/// Nova's interceptor (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interceptor;

impl Behaviour for Interceptor {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        if let Some(goal) = hunt(npc, around, INTERCEPTOR_RETREAT) {
            return goal;
        }
        if let Goal::Inspect(ship) = npc.goal
            && let Some(state) = around.state_of(ship).filter(|_| around.live(ship))
        {
            let off = state.position - npc.state.position;
            if off.x.abs() > INSPECT_REACH || off.y.abs() > INSPECT_REACH {
                return Goal::Inspect(ship);
            }
        }
        inspect(npc, around, chance).unwrap_or_else(|| idle(npc, around, chance))
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around)
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
        .filter(|other| other.id != npc.id && other.ai_type != AiType::Interceptor)
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
    use crate::ai::fixture::{NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::weapon::WeaponSpec;
    use crate::govt::{Governments, WARSHIPS_RETREAT};
    use crate::testkit::{Draws, weapon};

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
        let goal = Interceptor.decide(&npcs[0], &around, &mut chance);
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
        let near = ship(2, TRADERS, AiType::WimpyTrader, -100.0, 100.0);
        let npcs = [cop.clone(), near];
        let (goal, asked) = decided_by(&govts(0, 0), &npcs, (0.0, -1000.0), 0, &[0]);
        assert_eq!(
            (goal, asked),
            (Goal::Inspect(P), vec![1]),
            "looked it over: on to another, not it again"
        );
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
            Interceptor.decide(&npcs[0], &around, &mut Draws::of(&[0])),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn an_interceptor_targets_and_fires_at_whom_it_fights_and_not_whom_it_inspects() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut hunter = police();
        hunter.goal = Goal::Attack(P);
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
        assert_eq!(Interceptor.target(&npcs[0], &around), Some(P));
        assert_eq!(
            Interceptor.trigger(&npcs[0], &around).only,
            Some(WeaponId(128))
        );
        let inspecting = Npc {
            goal: Goal::Inspect(P),
            ..hunter
        };
        assert_eq!(Interceptor.target(&inspecting, &around), None);
        assert_eq!(
            Interceptor.trigger(&inspecting, &around),
            Trigger::default()
        );
        let hit = Strike {
            ship: n(1),
            by: P,
            damage: 3.0,
            downed: None,
        };
        assert_eq!(Interceptor.react(&npcs[0], &hit, &around).provoked, 3.0);
    }
}
