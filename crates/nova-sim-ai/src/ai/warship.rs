//! Nova's warship (AI type 3, `_WarshipAI` @0x8b72a in the `EV Nova`
//! executable), and the hunt it shares with the interceptor ([`hunt`]).
//!
//! Each decision, in order:
//!
//! - **The government's word on the player** (`_WarshipAI`
//!   @0x8b76c-0x8b80f, `_InterceptorAI` @0x8cb0d-0x8cbf3, identical),
//!   before anything else, for a ship with a government that is not in
//!   hyperspace, while the player is in the system, by its government's
//!   `Flags`, 0x0004 first. Its gate is the jump state +0x50 in
//!   (-900, 0] (0xdda4c, 0xdd054; [`Npc::in_hyperspace`]), not
//!   provocation (+0x96), so a provoked ship takes it too. A leaderless
//!   warship of a plunderer's government (`Flags` 0x1000) never thinks
//!   here: `_AIDispatch` sends it to `_PirateWarshipAI` (@0x8ffc8-0x8fff0,
//!   [`pirate`](super::pirate)), which has no word.
//!   - **0x0004**: unless it already threatens the player or a ship the
//!     player leads ([`Npc::threatens_player_among`], the original's
//!     `_IsThreatToPlayer` @0x8b7b7, @0x8cb66), it turns on the player
//!     (`_AIMakeShipAttackPlayer`) and its provocation is set to 1
//!     (@0x8b7e5, @0x8cb9d; [`Hunt::provokes`]). When [`dropped`] would let the player
//!     go (disabled while the ship has no destroying weapons, or breaking
//!     up: `_WarshipAI` drops such a target later in the same think,
//!     @0x8bc58-0x8bca9), it has no target instead. The retreat and the
//!     hopeless chase below still apply.
//!   - **0x0040**: when it threatens the player or a ship the player
//!     leads (@0x8b7f7, @0x8cbb2), it drops its target (-1, state 0) and
//!     picks afresh ([`select_target`] with none kept, as
//!     `_SelectWarshipTarget` does in the same think, @0x8b835-0x8b875).
//!
//!   The word reads no blessing (neither function calls
//!   `_PlayerBlessedByGovt`), so by the engine a 0x0004 ship turns on a
//!   player its government blesses with peace (rank `Flags` 0x0100,
//!   [`Surroundings::blessed`]), as Bureau 164's warships do to a
//!   Federation Commander. By the other reading of
//!   [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks),
//!   the Bible's, a blessed ship gives no word, and [`select_target`]
//!   gives it no player either; nor does it hold on to a player it already
//!   attacks, kept or as its provoker (crons-news-ranks-fidelity 33).
//!
//!   So a 0x0004 ship attacking a ship the player leads goes on fighting
//!   it, and a 0x0040 ship attacking one drops it. The provocation of 1
//!   keeps a ship with a target in state 4 (@0x8bb0c-0x8bb44); here a
//!   hunt always attacks the target it holds. The -900 bound of the gate
//!   marks a ship still jumping in (@0x8fcf5), the 0 bound one warping
//!   out, which the engine has no stage for. The player is always visible
//!   to NPCs here (its cloak is cloaking-fidelity's).
//!
//!   The 0x0004 word turns the ship on the player through
//!   `_AIMakeShipAttackPlayer` (@0x8b7da, @0x8cb8f), whose gate says a
//!   person's attack quote at once, with `_ShowPersonHailQuote`'s beep
//!   ([`Hunt::turns_on_player`], asked by the traffic's
//!   [`Traffic::turn_with_quote`](crate::traffic::Traffic::turn_with_quote)).
//!   It is called before the rest of the think, so it holds whatever goal
//!   the think ends in, a retreat, a hopeless chase or none. The 0x0040
//!   word makes no call itself, but drops the target and goal first
//!   ([`Hunt::dropped_target`]), so a person whose grudge then takes the
//!   player again is no threat to the gate, and says its quote again
//!   each think. The word plays no sound of its own.
//! - **Fleeing**, it keeps fleeing while its attacker is live; then it
//!   decides again.
//! - **Its target**: provoked, the attacker it took on being hit;
//!   otherwise the ship [`select_target`] gives, keeping the one it
//!   attacks. A target is dropped as [`dropped`] says, a fresh pick
//!   included: `_WarshipAI` sets state 4 for the target
//!   `_SelectWarshipTarget` gives and at once drops one inactive, breaking
//!   up, or disabled while it has no destroying weapons (@0x8bc4e-0x8bca9),
//!   and `_HighLevelAIHandler` drops a breaking-up one too (@0x8e3ef). So
//!   such a pick (a breaking-up NPC a ship with destroying weapons hunts,
//!   or a downed ally's quarry it helps against) leaves it with no target
//!   that think, and no other candidate is tried. The original's step 1
//!   keeps a target that starts breaking up and drops it the same way,
//!   so the ship idles that think, where the engine's keep drops it first
//!   and picks afresh at once: a one-think lag, not filed.
//! - **Retreat**, only when its government says so (a warship's `Flags`
//!   0x0010; the Bible: "retreat when shields drop below 25%"):
//!   - the odds retreat (@0x8bcea): while attacking, with its shields
//!     below [`ODDS_RETREAT_SHIELDS`] of their most and the odds against
//!     it above its government's `MaxOdds`, twice that while a
//!     reinforcement fleet allied with it is coming to the system
//!     (@0x8bd36-0x8bdc8, `_InterceptorAI` @0x8d1de-0x8d27e;
//!     [`odds_factor`]);
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
//! - **The out-of-ammo retreat** (@0x8bf3e-0x8bf83), after those, by its
//!   hull's `Flags2` 0x0080 rather than its government: short of
//!   ammunition, it flees its target (see [`out_of_ammo`]).
//! - **A hopeless chase** ([`hopeless_chase`]) stands off and snipes;
//!   anything else is attacked. A stand-off ship (`shïp` `Flags2`
//!   0x0002) never asks: `_HighLevelAIHandler` tests the bit first and
//!   jumps past `_AIEvalHopelessChase` (@0x8e48e), keeping its band
//!   instead (see [`stand_off`](super::stand_off)).
//! - **Fully dry** (@0x8c0e7-0x8c117), an attack or snipe is dropped: it
//!   stops ([`Goal::Idle`], [`out_of_ammo::drop_when_dry`]).
//! - With no target it goes about its business ([`idle`](super::idle)),
//!   short of ammunition or not: the out-of-ammo retreat needs a target.
//!
//! A held ship ([`Goal::Hold`], the original's goal 10 with no leader)
//! still hears its government's word and answers its attacker, but looks
//! for no target: `_SelectWarshipTarget` runs only for goals 0, 1, 2,
//! 6 and 0x14 (@0x8b835-0x8b875, @0x8bb56-0x8bba2, @0x8bbab-0x8bbc2), so
//! with no word and no provocation it holds ([`idle`](super::idle)).
//!
//! A fleet's escort of a government with `Flags` 0x1000 thinks here,
//! word and retreats included, as every fleet escort does until
//! rdm:phase/escort-fleet-fidelity/phase-3-npc-led-escorts gives them
//! `_EscortAI`; a leaderless one is a pirate ([`pirate`](super::pirate)).

use crate::ai::fire::FireRules;
use crate::ai::odds::{hopeless_chase, odds_against};
use crate::ai::reinforcements::odds_factor;
#[cfg(doc)]
use crate::ai::target::select_target;
use crate::ai::target::{Pick, TargetRules, dropped, pick_target};
use crate::ai::{
    Behaviour, Decision, Goal, Reaction, Surroundings, fire, idle, out_of_ammo, react,
};
use crate::chance::Chance;
use crate::combat::armament::Trigger;
use crate::combat::{ShipRef, Strike};
use crate::govt::{ALWAYS_ATTACKS_PLAYER, NEVER_ATTACKS_PLAYER, WARSHIPS_RETREAT};
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
    /// Whether it retreats when its weapons run out of ammunition, by its
    /// hull's `Flags2` 0x0080 ([`out_of_ammo::retreats`]).
    pub out_of_ammo: bool,
}

/// What a hunter (a warship or interceptor) does, retreating as `retreat`
/// says and taking its government's word on the player, which reaches a
/// blessed player as `always_attacks` says
/// ([`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks)):
/// its goal, or none when it has nothing to fight (see the module docs).
///
/// After a word, the retreat judges the ship by the goal the word has
/// just given it, an attack on its new target: the original sets the
/// target and state 4 at once (@0x8b7da-0x8b7e5; @0x8b800 then
/// @0x8b875-0x8b881), so its odds later in the same think
/// (`_ExtendedIsThreatToShip` asking `_IsThreatToPlayer`) count the player
/// against a ship the word turns on it, and not against one the word
/// turns off a ship the player leads.
///
/// Without a word, the retreat judges the ship by the goal it held, so
/// the odds retreat waits for a ship that was not attacking to attack.
/// The original's gate is state 4, which a ship has as soon as it takes
/// a target (@0x8b857, @0x8b881), so a ship outnumbered as it picks a
/// target retreats one think later here than in the original: a
/// one-think lag the player can't see, noted here and not filed.
///
/// The hunt also says whether the think turns the ship on the player
/// through `_AIMakeShipAttackPlayer` ([`Hunt::turns_on_player`]): the
/// 0x0004 word does (@0x8b7da, @0x8cb8f), even when the player is let go
/// at once, and so does a pick that calls it ([`pick_target`]), the
/// 0x0040 word's fresh pick included. A ship fleeing on, or turning on
/// its provoker, makes no call.
#[must_use]
pub fn hunt(
    npc: &Npc,
    around: &Surroundings,
    retreat: Retreat,
    always_attacks: RuleSource,
    rules: TargetRules,
    chance: &mut dyn Chance,
) -> Hunt {
    let word = government_word(npc, around, always_attacks);
    let dropped_target = word == Some(Word::DropTarget);
    let word_provokes = word == Some(Word::TakePlayer);
    let nothing = |turns_on_player| Hunt {
        goal: None,
        turns_on_player,
        dropped_target,
        provokes: word_provokes,
    };
    let keep = |ship: ShipRef| !dropped(npc, ship, around, rules);
    // A fresh pick the think drops at once (@0x8bc4e-0x8bca9, @0x8e3ef)
    // leaves the ship with no target: no other candidate is tried.
    let picked = |pick: Option<Pick>| match pick {
        Some(pick) if keep(pick.ship) => Ok((pick.ship, pick.turns_on_player)),
        Some(pick) => Err(pick.turns_on_player),
        None => Err(false),
    };
    let (target, turns_on_player) = match word {
        Some(Word::TakePlayer) => {
            if dropped(npc, ShipRef::Player, around, rules) {
                return nothing(true);
            }
            (ShipRef::Player, true)
        }
        Some(Word::DropTarget) => match picked(pick_target(npc, None, around, rules, chance)) {
            Ok(pick) => pick,
            Err(turns_on_player) => return nothing(turns_on_player),
        },
        None => {
            if let Goal::Flee(attacker) = npc.goal
                && around.live(attacker)
            {
                return Hunt::of(Goal::Flee(attacker));
            }
            let lets_go = |ship: ShipRef| releases_blessed(npc, ship, around, always_attacks);
            let held = |ship: ShipRef| keep(ship) && !lets_go(ship);
            let provoker = npc.target.filter(|&ship| npc.provoked > 0.0 && held(ship));
            if npc.goal == Goal::Hold {
                return Hunt {
                    goal: provoker.map(|ship| hunted(npc, ship, around, retreat)),
                    ..nothing(false)
                };
            }
            if let Some(ship) = provoker {
                return Hunt::of(hunted(npc, ship, around, retreat));
            }
            let current = npc.goal.attacking().filter(|&ship| held(ship));
            match picked(pick_target(npc, current, around, rules, chance)) {
                Ok(pick) => pick,
                Err(turns_on_player) => return nothing(turns_on_player),
            }
        }
    };
    let goal = if word.is_some() {
        let judged = Npc {
            goal: Goal::Attack(target),
            ..npc.clone()
        };
        hunted(&judged, target, around, retreat)
    } else {
        hunted(npc, target, around, retreat)
    };
    Hunt {
        goal: Some(goal),
        turns_on_player,
        dropped_target,
        provokes: word_provokes,
    }
}

/// Whether `npc` lets `ship` go as a target it holds, by the Bible's
/// reading of [`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks)
/// in `always_attacks`: `ship` is the player, `npc`'s government has
/// `Flags` 0x0004 and blesses the player (crons-news-ranks-fidelity 33).
/// The word itself is held back for a blessed player by that reading
/// ([`government_word`]); this lets go of one the word turned the ship on
/// before the blessing came, as the kept target or as the provoker (the
/// word's provocation of 1 can't be told from a hit's), so the ship fights
/// a blessed player only on the hit's own reaction. Never by the engine.
fn releases_blessed(
    npc: &Npc,
    ship: ShipRef,
    around: &Surroundings,
    always_attacks: RuleSource,
) -> bool {
    ship == ShipRef::Player
        && always_attacks == RuleSource::Bible
        && around.govts.flag(npc.govt, ALWAYS_ATTACKS_PLAYER)
        && around.blessed(npc.govt)
}

/// What a hunter's think gives ([`hunt`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hunt {
    /// Its goal, or none when it has nothing to fight.
    pub goal: Option<Goal>,
    /// Whether the think calls `_AIMakeShipAttackPlayer`
    /// ([`Decision::turns_on_player`](crate::ai::Decision::turns_on_player)).
    pub turns_on_player: bool,
    /// Whether the government's 0x0040 word dropped its target and goal
    /// first ([`Decision::dropped_target`](crate::ai::Decision::dropped_target)).
    pub dropped_target: bool,
    /// Whether the government's 0x0004 word set its provocation to 1
    /// ([`Decision::provokes`](crate::ai::Decision::provokes)).
    pub provokes: bool,
}

impl Hunt {
    /// A hunt for `goal` that makes no call and drops nothing.
    fn of(goal: Goal) -> Self {
        Self {
            goal: Some(goal),
            ..Self::default()
        }
    }

    /// The decision this hunt makes, its goal or else `otherwise`.
    #[must_use]
    pub fn or(self, otherwise: impl FnOnce() -> Goal) -> Decision {
        Decision {
            goal: self.goal.unwrap_or_else(otherwise),
            turns_on_player: self.turns_on_player,
            dropped_target: self.dropped_target,
            provokes: self.provokes,
            comm_call: None,
            calls_reinforcements: false,
            board_wait: None,
            clears_target: false,
            keeps_pursuit: false,
        }
    }
}

/// What a hunter `npc`, judged by the goal it holds, does about
/// `target`: it retreats from it as `retreat` says, the odds and shield
/// retreats then the out-of-ammo one (@0x8bcd9-0x8bf35, then
/// @0x8bf3e-0x8bf83), or snipes at it in a hopeless chase (which a
/// stand-off ship never asks, @0x8e48e), or attacks it.
fn hunted(npc: &Npc, target: ShipRef, around: &Surroundings, retreat: Retreat) -> Goal {
    if retreats(npc, around, retreat) || (retreat.out_of_ammo && out_of_ammo::retreats(npc)) {
        Goal::Flee(target)
    } else if !npc.hull.stands_off && hopeless_chase(npc, target, around) {
        Goal::Snipe(target)
    } else {
        Goal::Attack(target)
    }
}

/// What a hunter's government says of the player before anything else
/// in its think (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Word {
    /// `Flags` 0x0004: turn on the player.
    TakePlayer,
    /// `Flags` 0x0040: drop a target that makes it a threat to the player.
    DropTarget,
}

/// The word `npc`'s government has on the player among `around`, if any
/// (`_WarshipAI` @0x8b76c-0x8b80f, `_InterceptorAI` @0x8cb0d-0x8cbf3; see
/// the module docs): none without a government, in hyperspace
/// ([`Npc::in_hyperspace`]), or with no player in the system; and by the
/// Bible's `always_attacks`, no 0x0004 word for a blessed player.
fn government_word(npc: &Npc, around: &Surroundings, always_attacks: RuleSource) -> Option<Word> {
    if npc.govt.is_none() || npc.in_hyperspace() || around.player.is_none() {
        return None;
    }
    let govts = around.govts;
    let threat = npc.threatens_player_among(around.npcs);
    if govts.flag(npc.govt, ALWAYS_ATTACKS_PLAYER) {
        let held = always_attacks == RuleSource::Bible && around.blessed(npc.govt);
        (!threat && !held).then_some(Word::TakePlayer)
    } else if govts.flag(npc.govt, NEVER_ATTACKS_PLAYER) {
        threat.then_some(Word::DropTarget)
    } else {
        None
    }
}

/// Whether `npc` retreats as `retreat` says (see the module docs).
fn retreats(npc: &Npc, around: &Surroundings, retreat: Retreat) -> bool {
    let govts = around.govts;
    let shield = npc.reserves.shield;
    let below = |share: f32| shield.now < share * shield.max;
    let outnumbered = govts.flag(npc.govt, retreat.odds_flag)
        && npc.goal.attacking().is_some()
        && below(ODDS_RETREAT_SHIELDS)
        && odds_against(npc, around) > govts.max_odds(npc.govt) * odds_factor(npc, around);
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
    /// Whether its government's 0x0004 word reaches a blessed player
    /// ([`RuleKey::BlessingAlwaysAttacks`](crate::RuleKey::BlessingAlwaysAttacks)).
    pub always_attacks: RuleSource,
    /// The rulebook's entries its weapon pick follows.
    pub fire: FireRules,
    /// The rulebook's entries its target pick follows.
    pub target: TargetRules,
}

/// A warship's retreat: when outnumbered or its shields run low, both by
/// `Flags` 0x0010, a person at its `Coward` by the engine, and out of
/// ammunition by its hull's `Flags2` 0x0080.
pub const WARSHIP_RETREAT: Retreat = Retreat {
    odds_flag: WARSHIPS_RETREAT,
    shields: true,
    person_coward: RuleSource::Engine,
    out_of_ammo: true,
};

impl Behaviour for Warship {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.decision(npc, around, chance).goal
    }

    /// Its [`hunt`], or with nothing to fight its business ([`idle`]),
    /// the hunt's call on the player kept either way; an attack fully dry
    /// is dropped ([`out_of_ammo::drop_when_dry`]).
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        let retreat = Retreat {
            person_coward: self.person_coward,
            ..WARSHIP_RETREAT
        };
        let decision = hunt(
            npc,
            around,
            retreat,
            self.always_attacks,
            self.target,
            chance,
        )
        .or(|| idle(npc, around, chance));
        Decision {
            goal: out_of_ammo::drop_when_dry(npc, decision.goal),
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fire::AmmoLeft;
    use crate::ai::fixture::{
        NEUTRAL, PIRATES, POLICE, TRADERS, around, govts, n, ship, short_of_ammo, sites,
    };
    use crate::catalog::{StellarId, WeaponId, WeaponRecord};
    use crate::combat::armament::Armament;
    use crate::combat::hull::Condition;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::govt::Governments;
    use crate::rulebook::RuleSource;
    use crate::traffic::npc::{AiType, Mode, NpcId};
    use nova_sim_base::fixture::Draws;
    use nova_sim_catalog::fixture::weapon;

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
    fn a_held_warship_looks_for_no_target_but_answers_its_attacker_and_its_governments_word() {
        let mut held = police();
        held.govt = Some(TRADERS);
        held.goal = Goal::Hold;
        let pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        assert_eq!(
            decided(&[held.clone(), pirate.clone()], 0),
            Goal::Hold,
            "a pirate in view"
        );
        let mut provoked = held.clone();
        provoked.provoked = 1.0;
        provoked.target = Some(n(2));
        assert_eq!(decided(&[provoked, pirate.clone()], 0), Goal::Attack(n(2)));
        assert_eq!(
            decided_by(&govts(ALWAYS_ATTACKS_PLAYER, 0), &[held, pirate], 0),
            Goal::Attack(P),
            "its government's word"
        );
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
        dying.condition = Condition::Dying {
            ticks_left: 2,
            starting: false,
        };
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

    /// The police warship, its shields below half, fighting a pirate of
    /// strength `foe` against its 100 (its `MaxOdds` 1), while the fleet
    /// of `coming`, if any, is on its way: whether it retreats.
    fn retreats_with_coming(
        behaviour: &dyn Behaviour,
        retreat_flag: u16,
        foe: f32,
        coming: Option<crate::ai::reinforcements::Coming>,
    ) -> bool {
        let mut hunter = police();
        hunter.goal = Goal::Attack(n(2));
        hunter.reserves.shield.now = 14.9;
        hunter.aggression = 3;
        let mut pirate = ship(2, PIRATES, AiType::Warship, 0.0, 300.0);
        pirate.hull.strength = foe;
        pirate.goal = Goal::Attack(n(1));
        let npcs = [hunter, pirate];
        let govts = govts(0, retreat_flag);
        let sites = sites();
        let mut around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        around.reinforcements = coming;
        behaviour.decide(&npcs[0], &around, &mut Draws::of(&[0])) == Goal::Flee(n(2))
    }

    /// The fleet of `govt` coming.
    fn coming(govt: crate::catalog::GovtId) -> crate::ai::reinforcements::Coming {
        crate::ai::reinforcements::Coming { govt: Some(govt) }
    }

    #[test]
    fn a_warships_odds_retreat_waits_for_twice_max_odds_while_allied_reinforcements_come() {
        let warship = Warship::default();
        let retreats = |foe, coming| retreats_with_coming(&warship, WARSHIPS_RETREAT, foe, coming);
        assert!(retreats(150.0, None), "odds 1.5 over 1, nothing coming");
        assert!(
            !retreats(150.0, Some(coming(POLICE))),
            "its own fleet coming"
        );
        assert!(!retreats(150.0, Some(coming(TRADERS))), "an ally's");
        assert!(retreats(150.0, Some(coming(PIRATES))), "not an ally's");
        assert!(retreats(201.0, Some(coming(POLICE))), "beyond twice");
        assert!(!retreats(200.0, Some(coming(POLICE))), "at twice");
    }

    #[test]
    fn an_interceptors_odds_retreat_waits_for_twice_max_odds_too() {
        let interceptor = crate::ai::Interceptor::default();
        let retreats = |foe, coming| {
            retreats_with_coming(&interceptor, crate::govt::INTERCEPTORS_RETREAT, foe, coming)
        };
        assert!(retreats(150.0, None), "nothing coming");
        assert!(
            !retreats(150.0, Some(coming(POLICE))),
            "an ally's (@0x8d26d)"
        );
        assert!(retreats(150.0, Some(coming(PIRATES))), "not an ally's");
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
    fn a_fleet_leader_turns_on_the_ship_attacking_its_escort() {
        let mut escort = ship(2, NEUTRAL, AiType::Warship, 0.0, 100.0);
        escort.leader = Some(NpcId(1));
        let mut raider = ship(3, NEUTRAL, AiType::Warship, 0.0, 400.0);
        raider.goal = Goal::Attack(n(2));
        let npcs = [police(), escort, raider];
        assert_eq!(decided(&npcs, 0), Goal::Attack(n(3)));
        let mut unled = npcs.clone();
        unled[1].leader = None;
        assert_eq!(decided(&unled, 0), Goal::Land(StellarId(128)));
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
    fn a_stand_off_hunter_never_asks_whether_a_chase_is_hopeless() {
        let mut hunter = police();
        hunter.hull.mass = 100.0;
        hunter.hull.stands_off = true;
        hunter.govt = Some(TRADERS);
        let mut quarry = ship(2, PIRATES, AiType::WimpyTrader, 0.0, -300.0);
        quarry.goal = Goal::Flee(n(1));
        quarry.state.velocity = Vec2::new(0.0, -1.0);
        assert_eq!(decided(&[hunter, quarry], 0), Goal::Attack(n(2)));
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
            damage: 1.0,
            ..crate::combat::fixture::strike(n(9), P)
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
            coward(WARSHIPS_RETREAT, w, 0, (-1.0, 1), false, ENGINE),
            FIGHT,
            "Coward 0 never runs, its shields below none"
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
                coward(0, ai_type, 0, (-1.0, 4), false, BIBLE),
                FIGHT,
                "{ai_type:?}"
            );
        }
    }

    // The government's word on the player (`_WarshipAI` @0x8b76c-0x8b80f,
    // `_InterceptorAI` @0x8cb0d-0x8cbf3).

    /// The pirate warship NPC 2, 300 below.
    fn pirate() -> Npc {
        ship(2, PIRATES, AiType::Warship, 0.0, 300.0)
    }

    /// NPC 3, a neutral hired escort of the player's, 500 below.
    fn hired() -> Npc {
        let mut escort = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        escort
    }

    /// The police (of `Flags` `flags`) attacking `quarry`.
    fn attacking(quarry: ShipRef) -> Npc {
        let mut hunter = police();
        hunter.goal = Goal::Attack(quarry);
        hunter
    }

    /// The decision of `npcs[0]`, a warship of the police with `flags`, the
    /// player 100 above in `condition`, or not in the system without one.
    fn worded(flags: u16, npcs: &[Npc], condition: Option<Condition>) -> Goal {
        let govts = govts(0, flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, (0.0, -100.0), 0);
        match condition {
            Some(condition) => {
                if let Some(player) = seen.player.as_mut() {
                    player.condition = condition;
                }
            }
            None => seen.player = None,
        }
        Warship::default().decide(&npcs[0], &seen, &mut Draws::of(&[0]))
    }

    const ALWAYS: u16 = crate::govt::ALWAYS_ATTACKS_PLAYER;
    const NEVER: u16 = crate::govt::NEVER_ATTACKS_PLAYER;
    const INTACT: Option<Condition> = Some(Condition::Intact);
    const IDLE: Goal = Goal::Land(StellarId(128));

    #[test]
    fn a_government_that_always_attacks_the_player_turns_an_unprovoked_warship_on_it() {
        let npcs = [attacking(n(2)), pirate()];
        assert_eq!(worded(ALWAYS, &npcs, INTACT), Goal::Attack(P));
        assert_eq!(worded(0, &npcs, INTACT), Goal::Attack(n(2)), "no word");
        let govts = govts(0, ALWAYS);
        let sites = sites();
        let mut interceptor = npcs.clone();
        interceptor[0].ai_type = AiType::Interceptor;
        let seen = around(&sites, &interceptor, &govts, (0.0, -100.0), 0);
        assert_eq!(
            crate::ai::Interceptor::default().decide(&interceptor[0], &seen, &mut Draws::of(&[0])),
            Goal::Attack(P),
            "an interceptor too"
        );
    }

    #[test]
    fn a_government_that_always_attacks_the_player_turns_an_unprovoked_warship_on_it_unless_it_already_fights_a_ship_the_player_leads()
     {
        let npcs = [attacking(n(3)), hired()];
        assert_eq!(worded(ALWAYS, &npcs, INTACT), Goal::Attack(n(3)));
        let fleeing = [
            Npc {
                goal: Goal::Flee(n(2)),
                ..police()
            },
            pirate(),
        ];
        assert_eq!(
            worded(ALWAYS, &fleeing, INTACT),
            Goal::Attack(P),
            "the word comes before the flight"
        );
    }

    #[test]
    fn a_government_that_always_attacks_the_player_turns_a_provoked_warship_on_it_and_provokes_it()
    {
        // The word's gate is the jump state +0x50 (@0x8b77d-0x8b798), not
        // provocation; the word sets provocation +0x96 to 1 (@0x8b7e5).
        let mut provoked = attacking(n(2));
        provoked.provoked = 5.0;
        provoked.target = Some(n(2));
        assert_eq!(
            worded_decision(
                ALWAYS,
                &[provoked.clone(), pirate()],
                Condition::Intact,
                0.0
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
            }
        );
        assert!(
            !worded_decision(0, &[provoked, pirate()], Condition::Intact, 0.0).provokes,
            "no word"
        );
    }

    #[test]
    fn a_warship_in_hyperspace_takes_no_word() {
        for (mode, goal) in [
            (Mode::JumpingIn { tick: 2 }, Goal::Attack(n(2))),
            (Mode::LeavingGate { tick: 2 }, Goal::Attack(n(2))),
            (Mode::HeldAtGate { ticks_left: 5 }, Goal::Attack(P)),
            (Mode::Flying, Goal::Attack(P)),
        ] {
            let hunter = Npc {
                mode,
                ..attacking(n(2))
            };
            assert_eq!(
                worded(ALWAYS, &[hunter, pirate()], INTACT),
                goal,
                "{mode:?}"
            );
        }
        let hunting = Npc {
            mode: Mode::JumpingIn { tick: 2 },
            ..attacking(P)
        };
        assert_eq!(
            worded(NEVER, &[hunting, pirate()], INTACT),
            Goal::Attack(P),
            "nor a 0x0040 word"
        );
    }

    #[test]
    fn a_government_that_always_attacks_the_player_turns_an_unprovoked_warship_on_it_and_lets_a_disabled_player_go_without_destroying_weapons()
     {
        let npcs = [attacking(n(2)), pirate()];
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        ] {
            assert_eq!(
                worded(ALWAYS, &npcs, Some(condition)),
                IDLE,
                "{condition:?}"
            );
        }
        let mut armed = npcs.clone();
        armed[0].armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                mass_dmg: 1,
                ..weapon(128)
            }),
            1,
        )]);
        assert_eq!(
            worded(ALWAYS, &armed, Some(Condition::Disabled)),
            Goal::Attack(P),
            "it finishes the player off"
        );
        assert_eq!(
            worded(
                ALWAYS,
                &armed,
                Some(Condition::Dying {
                    ticks_left: 3,
                    starting: false
                })
            ),
            IDLE,
            "breaking up, the player is gone"
        );
    }

    #[test]
    fn a_government_that_always_attacks_the_player_turns_an_unprovoked_warship_on_it_with_no_player_here_nothing_changes()
     {
        let npcs = [attacking(n(2)), pirate()];
        assert_eq!(worded(ALWAYS, &npcs, None), Goal::Attack(n(2)));
    }

    #[test]
    fn a_government_that_never_attacks_the_player_makes_an_unprovoked_warship_drop_the_player_or_a_led_target()
     {
        // `_IsThreatToPlayer` holds at once of a target of 0, the player
        // (@0x7f56d-0x7f570), so @0x8b7f7-0x8b806 drops it (-1, state 0).
        let hunting = [attacking(P), pirate()];
        assert_eq!(
            worded(NEVER, &hunting, INTACT),
            IDLE,
            "nothing else to fight"
        );
        assert_eq!(worded(0, &hunting, INTACT), Goal::Attack(P), "no word");
        let raider = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        assert_eq!(
            worded(NEVER, &[attacking(P), raider], INTACT),
            Goal::Attack(n(2)),
            "it picks afresh"
        );
        // Fleeing is none of the states `_IsThreatToPlayer` excludes.
        let fleeing = [
            Npc {
                goal: Goal::Flee(n(3)),
                ..police()
            },
            hired(),
        ];
        assert_eq!(worded(NEVER, &fleeing, INTACT), IDLE, "it stops fleeing");
        assert_eq!(worded(0, &fleeing, INTACT), Goal::Flee(n(3)), "no word");
        let npcs = [attacking(n(3)), hired()];
        assert_eq!(worded(NEVER, &npcs, INTACT), IDLE);
        assert_eq!(worded(0, &npcs, INTACT), Goal::Attack(n(3)), "no word");
        let raider = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        let with_raider = [attacking(n(3)), hired(), raider];
        assert_eq!(
            worded(NEVER, &with_raider, INTACT),
            Goal::Attack(n(2)),
            "it picks afresh"
        );
        let mut provoked = attacking(n(3));
        provoked.provoked = 5.0;
        provoked.target = Some(n(3));
        assert_eq!(
            worded(NEVER, &[provoked, hired()], INTACT),
            IDLE,
            "provoked, it drops it too"
        );
        let unled = [
            attacking(n(3)),
            ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0),
        ];
        assert_eq!(
            worded(NEVER, &unled, INTACT),
            Goal::Attack(n(3)),
            "a ship nobody leads"
        );
        assert_eq!(
            worded(NEVER | ALWAYS, &npcs, INTACT),
            Goal::Attack(n(3)),
            "0x0004 comes first"
        );
    }

    // The word, then the retreat and the hopeless chase (`_WarshipAI`
    // @0x8bcd9-0x8bf35 sets state 3 on whatever target it holds;
    // `_HighLevelAIHandler` asks `_AIEvalHopelessChase` @0x8e5fe, @0x8eace
    // of it too).

    /// The decision of `npcs[0]`, a warship of the police with `flags`,
    /// the player 100 above moving up at `speed`.
    fn worded_moving(flags: u16, npcs: &[Npc], speed: f32) -> Goal {
        let govts = govts(0, flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, (0.0, -100.0), 0);
        if let Some(player) = seen.player.as_mut() {
            player.state.velocity = Vec2::new(0.0, -speed);
        }
        Warship::default().decide(&npcs[0], &seen, &mut Draws::of(&[0]))
    }

    /// A police warship at `shield` of its 30, of `aggression`, with
    /// `goal`.
    fn worn(shield: f32, aggression: u8, goal: Goal) -> Npc {
        let mut hunter = police();
        hunter.reserves.shield.now = shield;
        hunter.aggression = aggression;
        hunter.goal = goal;
        hunter
    }

    #[test]
    fn a_warship_its_government_turns_on_the_player_still_retreats_and_keeps_fleeing() {
        let flags = ALWAYS | WARSHIPS_RETREAT;
        // Shields low at aggression 2 (below 15 %).
        let npcs = [worn(4.4, 2, Goal::Attack(n(2))), pirate()];
        assert_eq!(worded(flags, &npcs, INTACT), Goal::Flee(P), "shields");
        // Outnumbered (101 against 100) below half its shields.
        let mut strong = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        strong.hull.strength = 101.0;
        let npcs = [worn(14.9, 3, Goal::Attack(n(2))), strong.clone()];
        assert_eq!(worded(flags, &npcs, INTACT), Goal::Flee(P), "odds");
        // Fleeing the player is a threat to it (state 3 is none
        // `_IsThreatToPlayer` excludes), so the word passes it by.
        let fled = [worn(14.9, 3, Goal::Flee(P)), strong];
        assert_eq!(
            worded(flags, &fled, INTACT),
            Goal::Flee(P),
            "it keeps fleeing"
        );
        let rested = [worn(30.0, 3, Goal::Flee(P)), pirate()];
        assert_eq!(
            worded(flags, &rested, INTACT),
            Goal::Flee(P),
            "while the player is live, whatever its shields"
        );
        let npcs = [worn(30.0, 2, Goal::Attack(n(2))), pirate()];
        assert_eq!(worded(flags, &npcs, INTACT), Goal::Attack(P), "fresh");
    }

    #[test]
    fn a_warship_its_government_turns_on_the_player_snipes_at_one_out_of_reach() {
        let mut heavy = attacking(n(2));
        heavy.hull.mass = 100.0;
        let npcs = [heavy, pirate()];
        assert_eq!(worded_moving(ALWAYS, &npcs, 1.0), Goal::Snipe(P));
        assert_eq!(
            worded_moving(ALWAYS, &npcs, 0.3),
            Goal::Attack(P),
            "the gap opens too slowly"
        );
    }

    #[test]
    fn a_warship_that_drops_a_led_target_flees_from_or_snipes_at_the_one_it_picks_afresh() {
        let mut strong = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        strong.hull.strength = 101.0;
        let npcs = [worn(14.9, 3, Goal::Attack(n(3))), hired(), strong];
        assert_eq!(
            worded(NEVER | WARSHIPS_RETREAT, &npcs, INTACT),
            Goal::Flee(n(2)),
            "outnumbered"
        );
        let mut heavy = attacking(n(3));
        heavy.hull.mass = 100.0;
        let mut runner = ship(2, PIRATES, AiType::WimpyTrader, 0.0, -300.0);
        runner.goal = Goal::Flee(n(1));
        runner.state.velocity = Vec2::new(0.0, -1.0);
        let npcs = [heavy, hired(), runner];
        assert_eq!(worded(NEVER, &npcs, INTACT), Goal::Snipe(n(2)), "hopeless");
    }

    // The retreat judges the goal the word has just chosen: the original
    // sets the target and state 4 at once (@0x8b7da-0x8b7e5, @0x8b800 and
    // @0x8b875-0x8b881), so the odds later in the same think
    // (`_ExtendedIsThreatToShip` asking `_IsThreatToPlayer`) see it.

    #[test]
    fn a_warship_that_drops_a_led_target_weighs_its_odds_without_the_player() {
        // A raider of 100 against it; the player (100) would make it 2.
        let raider = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        let npcs = [worn(14.9, 3, Goal::Attack(n(3))), hired(), raider];
        assert_eq!(
            worded(NEVER | WARSHIPS_RETREAT, &npcs, INTACT),
            Goal::Attack(n(2)),
            "even odds: it fights the ship it picked afresh"
        );
    }

    #[test]
    fn a_warship_its_government_turns_on_the_player_weighs_the_player_in_its_odds_at_once() {
        // A raider of 1 against it, and the player (100) it now attacks.
        let mut raider = Npc {
            goal: Goal::Attack(n(1)),
            ..pirate()
        };
        raider.hull.strength = 1.0;
        let npcs = [worn(14.9, 3, Goal::Attack(n(2))), raider];
        assert_eq!(
            worded(ALWAYS | WARSHIPS_RETREAT, &npcs, INTACT),
            Goal::Flee(P),
            "101 against 100"
        );
        let idle = [worn(14.9, 3, Goal::Idle), npcs[1].clone()];
        assert_eq!(
            worded(ALWAYS | WARSHIPS_RETREAT, &idle, INTACT),
            Goal::Flee(P),
            "state 4 at once, from no fight"
        );
    }

    // The blessing.

    /// The decision of `npcs[0]`, a warship of the police with `flags`
    /// following `always_attacks`, the player 100 above blessed by the
    /// police or not.
    fn blessed_word(flags: u16, npcs: &[Npc], blessed: bool, always_attacks: RuleSource) -> Goal {
        let govts = govts(0, flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, (0.0, -100.0), 0);
        if blessed && let Some(player) = seen.player.as_mut() {
            player.blessing = crate::ai::fixture::blessing_of(POLICE);
        }
        let warship = Warship {
            always_attacks,
            ..Warship::default()
        };
        warship.decide(&npcs[0], &seen, &mut Draws::of(&[0]))
    }

    #[test]
    fn a_0x0004_word_turns_on_a_blessed_player_by_the_engine() {
        assert_eq!(Warship::default().always_attacks, RuleSource::Engine);
        for npcs in [vec![police()], vec![attacking(n(2)), pirate()]] {
            assert_eq!(
                blessed_word(ALWAYS, &npcs, true, RuleSource::Engine),
                Goal::Attack(P)
            );
        }
    }

    #[test]
    fn by_the_bible_a_blessing_holds_a_0x0004_word_back() {
        let bible = RuleSource::Bible;
        assert_eq!(blessed_word(ALWAYS, &[police()], true, bible), IDLE);
        let fighting = [attacking(n(2)), pirate()];
        assert_eq!(
            blessed_word(ALWAYS, &fighting, true, bible),
            Goal::Attack(n(2)),
            "it keeps its own target"
        );
        assert_eq!(
            blessed_word(ALWAYS, &[police()], false, bible),
            Goal::Attack(P),
            "unblessed, the word holds"
        );
    }

    #[test]
    fn by_the_bible_a_0x0004_warship_lets_go_of_a_blessed_player_it_already_attacks() {
        // crons-news-ranks-fidelity 33: kept by step 1, or by the word's
        // provocation of 1.
        let mut hunting = police();
        hunting.goal = Goal::Attack(P);
        let mut worded = hunting.clone();
        worded.provoked = 1.0;
        worded.target = Some(P);
        for npc in [hunting, worded] {
            let bible = RuleSource::Bible;
            assert_eq!(
                blessed_word(ALWAYS, std::slice::from_ref(&npc), true, bible),
                IDLE
            );
            assert_eq!(
                blessed_word(ALWAYS, std::slice::from_ref(&npc), true, RuleSource::Engine),
                Goal::Attack(P),
                "the engine keeps it"
            );
            assert_eq!(
                blessed_word(ALWAYS, std::slice::from_ref(&npc), false, bible),
                Goal::Attack(P),
                "unblessed"
            );
            assert_eq!(
                blessed_word(0, &[npc], true, bible),
                Goal::Attack(P),
                "no 0x0004"
            );
        }
    }

    // A fresh pick dropped in the same think (@0x8bc4e-0x8bca9, @0x8e3ef).

    /// A traders' warship with destroying weapons, or not.
    fn trader_cop(destroying: bool) -> Npc {
        let mut cop = police();
        cop.govt = Some(TRADERS);
        if destroying {
            cop.armament = Armament::new([(
                WeaponSpec::new(&WeaponRecord {
                    mass_dmg: 1,
                    ..weapon(128)
                }),
                1,
            )]);
        }
        cop
    }

    #[test]
    fn a_breaking_up_pick_leaves_the_warship_with_no_target_that_think() {
        // The player 100 above is wanted (-7 at the traders' home); the
        // breaking-up pirate is nearer.
        let mut dying = ship(2, PIRATES, AiType::Warship, 0.0, 50.0);
        dying.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        assert_eq!(
            decided(&[trader_cop(true), dying.clone()], -7),
            IDLE,
            "the nearest is breaking up: no other candidate is tried"
        );
        assert_eq!(
            decided(&[trader_cop(false), dying], -7),
            Goal::Attack(P),
            "without destroying weapons it was never a candidate"
        );
        assert_eq!(decided(&[trader_cop(true)], -7), Goal::Attack(P));
    }

    #[test]
    fn a_downed_quarry_of_an_ally_leaves_the_warship_with_no_target_that_think() {
        let mut ally = ship(2, POLICE, AiType::Warship, 0.0, 300.0);
        ally.goal = Goal::Attack(n(3));
        let mut downed = ship(3, PIRATES, AiType::Warship, 0.0, 600.0);
        downed.condition = Condition::Disabled;
        let pirate = ship(4, PIRATES, AiType::Warship, 0.0, 900.0);
        let npcs = [trader_cop(false), ally, downed, pirate];
        assert_eq!(decided(&npcs, 0), IDLE);
        let mut armed = npcs.clone();
        armed[0] = trader_cop(true);
        assert_eq!(
            decided(&armed, 0),
            Goal::Attack(n(3)),
            "it can finish it off"
        );
    }

    // The quote call (`_AIMakeShipAttackPlayer` @0x8b7da, @0x8cb8f).

    /// The whole decision of `npcs[0]`, a warship of the police with
    /// `flags`, the player 100 above in `condition`, moving up at `speed`.
    fn worded_decision(flags: u16, npcs: &[Npc], condition: Condition, speed: f32) -> Decision {
        let govts = govts(0, flags);
        let sites = sites();
        let mut seen = around(&sites, npcs, &govts, (0.0, -100.0), 0);
        if let Some(player) = seen.player.as_mut() {
            player.condition = condition;
            player.state.velocity = Vec2::new(0.0, -speed);
        }
        Warship::default().decision(&npcs[0], &seen, &mut Draws::of(&[0]))
    }

    /// A decision of `goal` that turns on the player by the 0x0004 word,
    /// provoking the ship, or not, as `turns` says, dropping no target
    /// first.
    fn turning(goal: Goal, turns: bool) -> Decision {
        Decision {
            goal,
            turns_on_player: turns,
            dropped_target: false,
            provokes: turns,
            comm_call: None,
            calls_reinforcements: false,
            board_wait: None,
            clears_target: false,
            keeps_pursuit: false,
        }
    }

    #[test]
    fn the_governments_0x0004_word_turns_on_the_player_whatever_the_goal_it_ends_in() {
        let intact = Condition::Intact;
        let npcs = [attacking(n(2)), pirate()];
        assert_eq!(
            worded_decision(ALWAYS, &npcs, intact, 0.0),
            turning(Goal::Attack(P), true),
            "an attack"
        );
        let mut heavy = attacking(n(2));
        heavy.hull.mass = 100.0;
        assert_eq!(
            worded_decision(ALWAYS, &[heavy, pirate()], intact, 1.0),
            turning(Goal::Snipe(P), true),
            "a hopeless chase"
        );
        let tired = [worn(4.4, 2, Goal::Attack(n(2))), pirate()];
        assert_eq!(
            worded_decision(ALWAYS | WARSHIPS_RETREAT, &tired, intact, 0.0),
            turning(Goal::Flee(P), true),
            "a retreat"
        );
        assert_eq!(
            worded_decision(ALWAYS, &npcs, Condition::Disabled, 0.0),
            turning(IDLE, true),
            "a disabled player let go (@0x8bc58-0x8bca9)"
        );
        assert_eq!(
            worded_decision(ALWAYS, &[attacking(P), pirate()], intact, 0.0),
            turning(Goal::Attack(P), false),
            "already a threat"
        );
        let mut provoked = attacking(n(2));
        provoked.provoked = 5.0;
        provoked.target = Some(n(2));
        assert_eq!(
            worded_decision(ALWAYS, &[provoked, pirate()], intact, 0.0),
            turning(Goal::Attack(P), true),
            "provoked"
        );
        let mut stateless = attacking(n(2));
        stateless.govt = None;
        assert!(
            !worded_decision(ALWAYS, &[stateless, pirate()], intact, 0.0).turns_on_player,
            "no government"
        );
        assert_eq!(
            worded_decision(0, &npcs, intact, 0.0),
            turning(Goal::Attack(n(2)), false),
            "no word"
        );
    }

    #[test]
    fn a_0x0040_ship_holding_a_grudge_turns_on_the_player_again_after_its_word_drops_it() {
        let grudge = crate::traffic::GRUDGE;
        let mut grudger = attacking(P);
        grudger.person = Some(crate::traffic::npc::NpcPerson {
            flags: grudge,
            grudge: true,
            ..person()
        });
        // @0x8b800 clears the target and goal, then `_SelectWarshipTarget`'s
        // grudge calls `_AIMakeShipAttackPlayer` (@0x89f20).
        assert_eq!(
            worded_decision(NEVER, &[grudger.clone(), pirate()], Condition::Intact, 0.0),
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
            }
        );
        let mut calm = grudger;
        calm.person = Some(crate::traffic::npc::NpcPerson {
            flags: grudge,
            ..person()
        });
        assert_eq!(
            worded_decision(NEVER, &[calm, pirate()], Condition::Intact, 0.0),
            Decision {
                goal: IDLE,
                turns_on_player: false,
                dropped_target: true,
                provokes: false,
                comm_call: None,
                calls_reinforcements: false,
                board_wait: None,
                clears_target: false,
                keeps_pursuit: false,
            },
            "no grudge: dropped, and nothing picked"
        );
    }

    // Out of ammunition (`_WarshipAI` @0x8bf3e-0x8bf83, @0x8c0e7-0x8c117).

    const ARMED: AmmoLeft = AmmoLeft::Armed;
    const ONLY_UNLIMITED: AmmoLeft = AmmoLeft::OnlyUnlimited;
    const DRY: AmmoLeft = AmmoLeft::Dry;

    /// The police warship with `goal`, `left` of its ammunition, retreating
    /// out of it as `retreats` says.
    fn short(goal: Goal, left: AmmoLeft, retreats: bool) -> Npc {
        short_of_ammo(Npc { goal, ..police() }, left, retreats)
    }

    #[test]
    fn a_warship_of_flags2_0x0080_retreats_from_its_target_out_of_ammunition() {
        let fight = Goal::Attack(n(2));
        for left in [DRY, ONLY_UNLIMITED] {
            assert_eq!(
                worded(0, &[short(fight, left, true), pirate()], INTACT),
                Goal::Flee(n(2)),
                "{left:?}"
            );
        }
        assert_eq!(
            worded(0, &[short(fight, ARMED, true), pirate()], INTACT),
            fight
        );
        let mut provoked = short(Goal::Idle, DRY, true);
        provoked.provoked = 3.0;
        provoked.target = Some(n(3));
        let neutral = ship(3, NEUTRAL, AiType::Warship, 0.0, 500.0);
        assert_eq!(
            worded(0, &[provoked, neutral], INTACT),
            Goal::Flee(n(3)),
            "from its provoker"
        );
    }

    #[test]
    fn a_warship_without_flags2_0x0080_drops_its_target_only_fully_dry() {
        let fight = Goal::Attack(n(2));
        assert_eq!(
            worded(0, &[short(fight, ONLY_UNLIMITED, false), pirate()], INTACT),
            fight
        );
        assert_eq!(
            worded(0, &[short(fight, DRY, false), pirate()], INTACT),
            Goal::Idle,
            "state 0, target and nav -1 (@0x8c0ff-0x8c117)"
        );
        let mut heavy = short(fight, DRY, false);
        heavy.hull.mass = 100.0;
        assert_eq!(
            worded_moving(ALWAYS, &[heavy.clone(), pirate()], 1.0),
            Goal::Idle,
            "a hopeless chase too"
        );
        heavy.armament = police().armament;
        assert_eq!(
            worded_moving(ALWAYS, &[heavy, pirate()], 1.0),
            Goal::Snipe(P)
        );
    }

    #[test]
    fn a_warship_with_no_target_does_not_retreat_out_of_ammunition() {
        // The retreat needs a target: target -1 jumps past it (@0x8bc48).
        assert_eq!(worded(0, &[short(Goal::Idle, DRY, true)], INTACT), IDLE);
        assert_eq!(
            worded(0, &[short(Goal::Hold, DRY, true)], INTACT),
            Goal::Hold
        );
        let fleeing = short(Goal::Flee(n(2)), DRY, true);
        assert_eq!(
            worded(0, &[fleeing, pirate()], INTACT),
            Goal::Flee(n(2)),
            "state 3 already"
        );
    }

    #[test]
    fn a_warship_its_government_turns_on_the_player_retreats_from_it_out_of_ammunition() {
        let npcs = [short(Goal::Attack(n(2)), DRY, true), pirate()];
        assert_eq!(
            worded_decision(ALWAYS, &npcs, Condition::Intact, 0.0),
            turning(Goal::Flee(P), true)
        );
        let npcs = [short(Goal::Attack(n(2)), DRY, false), pirate()];
        assert_eq!(
            worded_decision(ALWAYS, &npcs, Condition::Intact, 0.0),
            turning(Goal::Idle, true),
            "fully dry without 0x0080, it drops the player at once"
        );
    }
}
