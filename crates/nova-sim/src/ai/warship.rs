//! Nova's warship (AI type 3, `_WarshipAI` @0x8b72a in the `EV Nova`
//! executable), and the hunt it shares with the interceptor ([`hunt`]).
//!
//! Each decision, in order:
//!
//! - **Fleeing**, it keeps fleeing while its attacker is live; then it
//!   decides again.
//! - **Its target**: provoked, the attacker it took on being hit;
//!   otherwise the ship [`select_target`] gives, keeping the one it
//!   attacks. A target is dropped as [`dropped`] says.
//! - **Retreat**, only when its government says so (a warship's `Flags`
//!   0x0010; the Bible: "retreat when shields drop below 25%"):
//!   - the odds retreat (@0x8bcea): while attacking, with its shields
//!     below [`ODDS_RETREAT_SHIELDS`] of their most and the odds against
//!     it above its government's `MaxOdds` (the original doubles them
//!     while allied reinforcements are coming; there are none here);
//!   - the shield retreat (@0x8be36), for a warship with no fleet lead:
//!     shields below [`AGGRESSION_1_SHIELDS`] of their most at aggression
//!     1, or [`AGGRESSION_2_SHIELDS`] at aggression 2. Any other
//!     aggression never retreats.
//!   - A person's shield retreat (@0x8be36-0x8bf35) is instead below
//!     trunc(its shields' most x `Coward` x [`COWARD_SHARE`]), whatever
//!     its aggression; a `Coward` of 0 or less never retreats. By the
//!     engine only a warship person retreats so, as any warship, with no
//!     fleet lead and by its government's `Flags` 0x0010 (an
//!     independent never); an interceptor person never does. By the
//!     Bible ([`RuleKey::PersonCoward`](crate::RuleKey::PersonCoward)),
//!     any warship or interceptor person with no fleet lead does,
//!     whatever its government.
//! - **A hopeless chase** ([`hopeless_chase`]) stands off and snipes;
//!   anything else is attacked.
//! - With no target it goes about its business ([`idle`](super::idle)).

use crate::ai::odds::{hopeless_chase, odds_against};
use crate::ai::target::{dropped, select_target};
use crate::ai::{Behaviour, Goal, Reaction, Surroundings, fire, idle, react};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::govt::WARSHIPS_RETREAT;
use crate::rulebook::RuleSource;
use crate::traffic::npc::Npc;

/// The share of its shields below which a ship outnumbered retreats
/// (0.5 @0xdd128).
pub const ODDS_RETREAT_SHIELDS: f32 = 0.5;
/// The share of its shields below which a warship of aggression 1
/// retreats (0.3 @0xdd8d0).
pub const AGGRESSION_1_SHIELDS: f32 = 0.3;
/// The share of its shields below which a warship of aggression 2
/// retreats (0.15 @0xdda88).
pub const AGGRESSION_2_SHIELDS: f32 = 0.15;

/// A person's shield retreat's share of its shields per point of
/// `Coward` (0.01 @0xdd098).
pub const COWARD_SHARE: f64 = 0.01;

/// How a hunter retreats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retreat {
    /// The government `Flags` bit that lets it retreat when outnumbered.
    pub odds_flag: u16,
    /// Whether it retreats when its shields run low.
    pub shields: bool,
    /// Which persons retreat at their `Coward`
    /// ([`RuleKey::PersonCoward`](crate::RuleKey::PersonCoward)).
    pub person_coward: RuleSource,
}

/// What a hunter (a warship or interceptor) does, retreating as `retreat`
/// says: its goal, or none when it has nothing to fight (see the module
/// docs).
#[must_use]
pub fn hunt(npc: &Npc, around: &Surroundings, retreat: Retreat) -> Option<Goal> {
    if let Goal::Flee(attacker) = npc.goal
        && around.live(attacker)
    {
        return Some(Goal::Flee(attacker));
    }
    let keep = |ship: ShipRef| !dropped(npc, ship, around);
    let provoker = npc.target.filter(|&ship| npc.provoked > 0.0 && keep(ship));
    let current = npc.goal.attacking().filter(|&ship| keep(ship));
    let target = provoker.or_else(|| select_target(npc, current, around))?;
    if retreats(npc, around, retreat) {
        return Some(Goal::Flee(target));
    }
    Some(if hopeless_chase(npc, target, around) {
        Goal::Snipe(target)
    } else {
        Goal::Attack(target)
    })
}

/// Whether `npc` retreats as `retreat` says (see the module docs).
fn retreats(npc: &Npc, around: &Surroundings, retreat: Retreat) -> bool {
    let govts = around.govts;
    let shield = npc.reserves.shield;
    let below = |share: f32| shield.now < share * shield.max;
    let outnumbered = govts.flag(npc.govt, retreat.odds_flag)
        && npc.goal.attacking().is_some()
        && below(ODDS_RETREAT_SHIELDS)
        && odds_against(npc, around) > govts.max_odds(npc.govt);
    let by_government = retreat.shields && govts.flag(npc.govt, WARSHIPS_RETREAT);
    let worn = npc.leader.is_none()
        && match npc.person {
            Some(person) => {
                let threshold =
                    (f64::from(shield.max) * f64::from(person.coward) * COWARD_SHARE).trunc();
                (by_government || retreat.person_coward == RuleSource::Bible)
                    && person.coward > 0
                    && f64::from(shield.now) < threshold
            }
            None => {
                by_government
                    && match npc.aggression {
                        1 => below(AGGRESSION_1_SHIELDS),
                        2 => below(AGGRESSION_2_SHIELDS),
                        _ => false,
                    }
            }
        };
    outnumbered || worn
}

/// Nova's warship (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Warship {
    /// Who comes to a ship's help ([`react::answer`]).
    pub piracy_police: RuleSource,
    /// Which persons retreat at their `Coward`.
    pub person_coward: RuleSource,
}

/// A warship's retreat: when outnumbered or its shields run low, both by
/// `Flags` 0x0010, a person at its `Coward` by the engine.
pub const WARSHIP_RETREAT: Retreat = Retreat {
    odds_flag: WARSHIPS_RETREAT,
    shields: true,
    person_coward: RuleSource::Engine,
};

impl Behaviour for Warship {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        let retreat = Retreat {
            person_coward: self.person_coward,
            ..WARSHIP_RETREAT
        };
        hunt(npc, around, retreat).unwrap_or_else(|| idle(npc, around, chance))
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        fire::trigger(npc, around)
    }

    fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
        npc.goal.quarry()
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        react::answer(npc, strike, around, self.piracy_police)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, sites};
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::govt::Governments;
    use crate::rulebook::RuleSource;
    use crate::testkit::{Draws, weapon};
    use crate::traffic::npc::{AiType, NpcId};

    const P: ShipRef = ShipRef::Player;

    /// A police warship (NPC 1) at the centre.
    fn police() -> Npc {
        ship(1, POLICE, AiType::Warship, 0.0, 0.0)
    }

    /// The warship's decision among `npcs` (it first) with the player 100
    /// above, of record `record` in the traders' system, by `govts`.
    fn decided_by(govts: &Governments, npcs: &[Npc], record: i16) -> Goal {
        let sites = sites();
        let around = around(&sites, npcs, govts, (0.0, -100.0), record);
        Warship::default().decide(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    fn decided(npcs: &[Npc], record: i16) -> Goal {
        decided_by(&govts(0, 0), npcs, record)
    }

    #[test]
    fn a_warship_attacks_a_wanted_player_a_pirate_or_whoever_provoked_it() {
        // The traders' allied police tolerate -9 at most.
        assert_eq!(decided(&[police()], -10), Goal::Attack(P));
        assert_eq!(decided(&[police()], -9), Goal::Land(StellarId(128)));
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let mut trader_cop = police();
        trader_cop.govt = Some(TRADERS);
        assert_eq!(
            decided(&[trader_cop, pirate.clone()], 0),
            Goal::Attack(n(2))
        );
        let mut provoked = police();
        provoked.provoked = 5.0;
        provoked.target = Some(n(3));
        let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        assert_eq!(decided(&[provoked, neutral], 0), Goal::Attack(n(3)));
    }

    #[test]
    fn only_a_provoked_warship_turns_on_its_attacker_and_never_on_an_ally() {
        let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        let mut calm = police();
        calm.target = Some(n(3));
        assert_eq!(
            decided(&[calm.clone(), neutral], 0),
            Goal::Land(StellarId(128)),
            "not provoked"
        );
        let trader = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 500.0);
        calm.provoked = 5.0;
        assert_eq!(
            decided(&[calm, trader], 0),
            Goal::Land(StellarId(128)),
            "provoked by an ally"
        );
    }

    #[test]
    fn a_warship_keeps_its_target_until_it_drops_it() {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(3));
        let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        let npcs = [hunter.clone(), neutral.clone()];
        assert_eq!(decided(&npcs, 0), Goal::Attack(n(3)), "kept");
        let mut dying = neutral.clone();
        dying.condition = Condition::Dying { ticks_left: 2 };
        assert_eq!(
            decided(&[hunter.clone(), dying], 0),
            Goal::Land(StellarId(128))
        );
        let mut wreck = neutral.clone();
        wreck.condition = Condition::Disabled;
        let npcs = [hunter.clone(), wreck];
        assert_eq!(decided(&npcs, 0), Goal::Land(StellarId(128)), "harmless");
        let mut armed = npcs.clone();
        armed[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(decided(&armed, 0), Goal::Attack(n(3)), "it finishes it off");
    }

    #[test]
    fn a_warship_drops_an_allied_target_or_one_of_its_fleet() {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(3));
        let trader = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 500.0);
        assert_eq!(
            decided(&[hunter.clone(), trader], 0),
            Goal::Land(StellarId(128))
        );
        let mut escort = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        escort.leader = Some(NpcId(1));
        assert_eq!(decided(&[hunter, escort], 0), Goal::Land(StellarId(128)));
    }

    #[test]
    fn a_warship_never_attacks_its_own_kind_or_an_ally_for_an_ally_or_as_a_threat() {
        // A stray shot from police NPC 2 provoked the allied trader NPC 3.
        let kin = ship(2, POLICE, AiType::Warship, 0.0, 300.0);
        let mut trader = ship(3, TRADERS, AiType::WimpyTrader, 0.0, 500.0);
        trader.goal = Goal::Flee(n(2));
        let npcs = [police(), kin, trader];
        assert_eq!(decided(&npcs, 0), Goal::Land(StellarId(128)), "fled");
        let mut fought = npcs.clone();
        fought[2].goal = Goal::Attack(n(2));
        assert_eq!(decided(&fought, 0), Goal::Land(StellarId(128)), "fought");
        let mut at_it = npcs.clone();
        at_it[2].goal = Goal::Attack(n(1));
        assert_eq!(decided(&at_it, 0), Goal::Land(StellarId(128)), "threat");
    }

    /// The police warship (with `police_flags`) fighting a pirate of
    /// strength `foe` 300 below, at `shield` of its 30.
    fn outgunned(police_flags: u16, shield: f32, foe: f32, aggression: u8) -> Goal {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield.now = shield;
        hunter.aggression = aggression;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.hull.strength = foe;
        pirate.goal = Goal::Attack(n(1));
        decided_by(&govts(0, police_flags), &[hunter, pirate], 0)
    }

    #[test]
    fn a_warship_retreats_outnumbered_with_its_shields_below_half_only_by_0x0010() {
        // Strength 100 against 101 (odds 1.01 over MaxOdds 1), aggression 3
        // so no shield retreat.
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 14.9, 101.0, 3),
            Goal::Flee(n(2))
        );
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 15.0, 101.0, 3),
            Goal::Attack(n(2))
        );
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 14.9, 100.0, 3),
            Goal::Attack(n(2)),
            "even"
        );
        assert_eq!(
            outgunned(0, 14.9, 101.0, 3),
            Goal::Attack(n(2)),
            "no 0x0010"
        );
        assert_eq!(ODDS_RETREAT_SHIELDS, 0.5);
    }

    #[test]
    fn a_warship_retreats_with_its_shields_low_by_its_aggression() {
        // Even odds: only the shields count.
        assert_eq!(outgunned(WARSHIPS_RETREAT, 4.4, 50.0, 2), Goal::Flee(n(2)));
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 4.5, 50.0, 2),
            Goal::Attack(n(2))
        );
        assert_eq!(outgunned(WARSHIPS_RETREAT, 8.9, 50.0, 1), Goal::Flee(n(2)));
        assert_eq!(
            outgunned(WARSHIPS_RETREAT, 9.0, 50.0, 1),
            Goal::Attack(n(2))
        );
        for aggression in [0, 3] {
            assert_eq!(
                outgunned(WARSHIPS_RETREAT, 0.0, 50.0, aggression),
                Goal::Attack(n(2)),
                "{aggression}"
            );
        }
        assert_eq!(outgunned(0, 0.0, 50.0, 2), Goal::Attack(n(2)), "no 0x0010");
        assert_eq!((AGGRESSION_1_SHIELDS, AGGRESSION_2_SHIELDS), (0.3, 0.15));
    }

    #[test]
    fn a_warship_with_a_fleet_lead_never_retreats_for_its_shields() {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield.now = 0.0;
        hunter.leader = Some(NpcId(3));
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let lead = ship(3, POLICE, AiType::Warship, 50.0, 0.0);
        let govts = govts(0, WARSHIPS_RETREAT);
        assert_eq!(
            decided_by(&govts, &[hunter, pirate, lead], 0),
            Goal::Attack(n(2))
        );
    }

    #[test]
    fn a_fleeing_warship_flees_until_its_attacker_is_gone() {
        let mut runner = police();
        runner.goal = Goal::Flee(n(2));
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(decided(&[runner.clone(), pirate], 0), Goal::Flee(n(2)));
        assert_eq!(decided(&[runner], 0), Goal::Land(StellarId(128)));
    }

    #[test]
    fn a_hopeless_chase_stands_off_and_snipes() {
        let mut hunter = police();
        hunter.hull.mass = 100.0;
        let mut quarry = ship(2, PIRATES, AiType::WimpyTrader, 0.0, -300.0);
        quarry.goal = Goal::Flee(n(1));
        quarry.state.velocity = Vec2::new(0.0, -1.0);
        let mut trader_cop = hunter;
        trader_cop.govt = Some(TRADERS);
        assert_eq!(decided(&[trader_cop, quarry], 0), Goal::Snipe(n(2)));
    }

    #[test]
    fn a_warship_targets_and_fires_at_whom_it_fights() {
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
        let npcs = [hunter];
        let seen = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(Warship::default().target(&npcs[0], &seen), Some(P));
        assert_eq!(
            Warship::default().trigger(&npcs[0], &seen).only,
            Some(WeaponId(128))
        );
        let hit = Strike {
            ship: n(9),
            by: P,
            damage: 1.0,
            downed: None,
        };
        let victim = ship(9, TRADERS, AiType::WimpyTrader, 900.0, 0.0);
        let npcs = [ship(1, POLICE, AiType::Warship, 0.0, 0.0), victim];
        let seen = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            Warship::default().react(&npcs[0], &hit, &seen).goal,
            Some(Goal::Attack(P)),
            "a Good Samaritan"
        );
    }

    // Persons.

    /// A person (`Coward` `coward`) of `ai_type` flying for the police
    /// (with `police_flags`), at `shield` of its 100 and aggression
    /// `aggression`, fighting the pirate NPC 2, by `rule`; led by NPC 3
    /// when `led`.
    fn coward(
        police_flags: u16,
        ai_type: AiType,
        coward: i16,
        (shield, aggression): (f32, u8),
        led: bool,
        rule: RuleSource,
    ) -> Goal {
        let mut hunter = ship(1, POLICE, ai_type, 0.0, 0.0);
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield = crate::reserves::Gauge {
            now: shield,
            max: 100.0,
        };
        hunter.aggression = aggression;
        hunter.leader = led.then_some(NpcId(3));
        hunter.person = Some(crate::traffic::npc::NpcPerson { coward, ..person() });
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let leader = ship(3, POLICE, AiType::Warship, 50.0, 0.0);
        let govts = govts(0, police_flags);
        let sites = sites();
        let npcs = [hunter, pirate, leader];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let behaviour: Box<dyn Behaviour> = match ai_type {
            AiType::Interceptor => Box::new(crate::ai::Interceptor {
                person_coward: rule,
                ..crate::ai::Interceptor::default()
            }),
            _ => Box::new(Warship {
                person_coward: rule,
                ..Warship::default()
            }),
        };
        behaviour.decide(&npcs[0], &around, &mut Draws::of(&[0]))
    }

    /// Person 600's traits, none of note.
    fn person() -> crate::traffic::npc::NpcPerson {
        crate::traffic::npc::NpcPerson {
            id: crate::catalog::PersonId(600),
            flags: 0,
            coward: 0,
            comm_quote: -1,
            hail_quote: -1,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: false,
            quoted: false,
            quoted_at: None,
        }
    }

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;
    const FLEE: Goal = Goal::Flee(ShipRef::Npc(NpcId(2)));
    const FIGHT: Goal = Goal::Attack(ShipRef::Npc(NpcId(2)));

    #[test]
    fn by_the_engine_a_warship_person_of_0x0010_runs_below_its_coward_share() {
        let w = AiType::Warship;
        for aggression in [1, 2, 4] {
            assert_eq!(
                coward(WARSHIPS_RETREAT, w, 25, (24.0, aggression), false, ENGINE),
                FLEE,
                "{aggression}"
            );
            assert_eq!(
                coward(WARSHIPS_RETREAT, w, 25, (26.0, aggression), false, ENGINE),
                FIGHT,
                "{aggression}"
            );
        }
        assert_eq!(
            coward(WARSHIPS_RETREAT, w, 25, (25.0, 4), false, ENGINE),
            FIGHT,
            "at its share it stays"
        );
        assert_eq!(
            coward(0, w, 25, (24.0, 4), false, ENGINE),
            FIGHT,
            "no 0x0010"
        );
        assert_eq!(
            coward(WARSHIPS_RETREAT, w, 0, (0.0, 1), false, ENGINE),
            FIGHT,
            "Coward 0 never runs"
        );
        assert_eq!(
            coward(WARSHIPS_RETREAT, w, -5, (0.0, 1), false, ENGINE),
            FIGHT
        );
        assert_eq!(
            coward(WARSHIPS_RETREAT, w, 25, (24.0, 4), true, ENGINE),
            FIGHT,
            "led"
        );
        assert_eq!(
            coward(
                WARSHIPS_RETREAT,
                AiType::Interceptor,
                25,
                (1.0, 4),
                false,
                ENGINE
            ),
            FIGHT,
            "an interceptor person never runs for its shields"
        );
        assert_eq!(COWARD_SHARE, 0.01);
    }

    #[test]
    fn a_coward_share_is_truncated() {
        // Shields of 30 at Coward 25: trunc(7.5) = 7.
        let mut hunter = ship(1, POLICE, AiType::Warship, 0.0, 0.0);
        hunter.reserves.shield.now = 7.0;
        hunter.goal = Goal::Attack(n(2));
        hunter.person = Some(crate::traffic::npc::NpcPerson {
            coward: 25,
            ..person()
        });
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let govts = govts(0, WARSHIPS_RETREAT);
        let npcs = [hunter, pirate];
        assert_eq!(decided_by(&govts, &npcs, 0), FIGHT, "7 is not below 7");
        let mut lower = npcs.clone();
        lower[0].reserves.shield.now = 6.9;
        assert_eq!(decided_by(&govts, &lower, 0), FLEE);
    }

    #[test]
    fn an_independent_warship_person_never_runs_by_the_engine() {
        let mut hunter = ship(1, POLICE, AiType::Warship, 0.0, 0.0);
        hunter.govt = None;
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield.now = 0.5;
        hunter.person = Some(crate::traffic::npc::NpcPerson {
            coward: 50,
            ..person()
        });
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        let govts = govts(0, WARSHIPS_RETREAT);
        assert_eq!(
            decided_by(&govts, &[hunter.clone(), pirate.clone()], 0),
            FIGHT
        );
        let sites = sites();
        let npcs = [hunter, pirate];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let bible = Warship {
            person_coward: BIBLE,
            ..Warship::default()
        };
        assert_eq!(
            bible.decide(&npcs[0], &around, &mut Draws::of(&[0])),
            FLEE,
            "by the Bible, independents too"
        );
    }

    #[test]
    fn by_the_bible_any_warship_or_interceptor_person_unled_runs_at_its_coward_share() {
        for ai_type in [AiType::Warship, AiType::Interceptor] {
            assert_eq!(
                coward(0, ai_type, 25, (24.0, 4), false, BIBLE),
                FLEE,
                "{ai_type:?}"
            );
            assert_eq!(
                coward(0, ai_type, 25, (26.0, 4), false, BIBLE),
                FIGHT,
                "{ai_type:?}"
            );
            assert_eq!(
                coward(0, ai_type, 25, (24.0, 4), true, BIBLE),
                FIGHT,
                "{ai_type:?}"
            );
            assert_eq!(
                coward(0, ai_type, 0, (0.0, 4), false, BIBLE),
                FIGHT,
                "{ai_type:?}"
            );
        }
    }
}
