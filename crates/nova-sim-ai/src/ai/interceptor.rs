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
//!   nor another interceptor (the original retries `Rand(64)` slots; here
//!   one draw among them). Within [`INSPECT_REACH`] of it on both axes
//!   (@0x8f11e, [`within_reach`]) it has looked it over, and decides
//!   again; the traffic remembers the ship ([`Npc::inspected`]). Having
//!   looked the player over, it has scanned the player
//!   ([`Traffic::take_scans`](crate::traffic::Traffic::take_scans)): the
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
//! - With nothing to inspect it patrols: the idle block, but its
//!   `_SelectRandomStellarDest` call is (ship, 0, 1) (@0x8d027-0x8d03a),
//!   plain stellars only ([`idle_admitting`] with [`Admit::PlainOnly`]):
//!   never a gate, and an uninhabited stellar as
//!   [`RuleKey::PatrolUninhabited`](crate::RuleKey::PatrolUninhabited)
//!   says.

use crate::ai::fire::FireRules;
use crate::ai::target::TargetRules;
use crate::ai::warship::{Retreat, hunt};
use crate::ai::{
    Admit, Behaviour, CommCall, Decision, Goal, Reaction, Surroundings, fire, idle, idle_admitting,
    out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::geometry::Vec2;
use crate::govt::INTERCEPTORS_RETREAT;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Npc};

/// How near, in pixels on either axis, an interceptor comes to the ship
/// it inspects (100.0 @0xdd060).
pub const INSPECT_REACH: f32 = 100.0;

/// Whether a ship `off` from an interceptor is within its reach: within
/// [`INSPECT_REACH`] on both axes (@0x8f11e).
#[must_use]
pub fn within_reach(off: Vec2) -> bool {
    off.x.abs() <= INSPECT_REACH && off.y.abs() <= INSPECT_REACH
}

/// An interceptor's retreat: when outnumbered, by `Flags` 0x0100, and
/// out of ammunition, by its hull's `Flags2` 0x0080.
pub const INTERCEPTOR_RETREAT: Retreat = Retreat {
    odds_flag: INTERCEPTORS_RETREAT,
    shields: false,
    person_coward: RuleSource::Engine,
    out_of_ammo: true,
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
    /// through each.
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
        let decision = hunt.or(|| {
            let (goal, call) = self.patrol(npc, around, chance);
            comm_call = call;
            out_of_ammo::after_patrol(npc, goal)
        });
        Decision {
            comm_call,
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
        if let Goal::Inspect(ship) = npc.goal
            && let Some(state) = around.state_of(ship).filter(|_| around.live(ship))
            && !within_reach(state.position - npc.state.position)
        {
            return (Goal::Inspect(ship), None);
        }
        if npc.goal == Goal::Hold {
            // The patrol and inspect block needs goal 1 or below, or
            // 0x14 (`_InterceptorAI` @0x8cc6d-0x8cc7d): a held ship
            // holds.
            return (idle(npc, around, chance), None);
        }
        let patrol = Admit::PlainOnly {
            uninhabited: self.patrol_uninhabited == RuleSource::Engine,
        };
        match inspect(npc, around, chance) {
            Some(goal) => {
                let call = (goal == Goal::Inspect(ShipRef::Player)).then_some(CommCall::PreScan);
                (goal, call)
            }
            None => (idle_admitting(npc, around, chance, patrol), None),
        }
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
            damage: 3.0,
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
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
                plunders: None,
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
        });
        assert_eq!(
            decision_by(NEVER, &[grudger], intact, 0, &[]),
            Decision {
                goal: Goal::Attack(P),
                turns_on_player: true,
                dropped_target: true,
                provokes: false,
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
                plunders: None,
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
}
