//! A fleet's leader issuing its escorts' orders: `_AIIssueEscortOrders`
//! (@0x8986d-0x89b8d), read whole. The leader works out a table of four
//! order words, one per escort class (fighter, medium, warship,
//! freighter, [`EscortClass`]), and writes each ship it leads the word of
//! its class ([`Npc::order_word`]), which
//! [`FleetEscortAi`](crate::ai::FleetEscortAi) then flies. The traffic's
//! pass runs it (`traffic::escort_orders`).
//!
//! **When** (`_AIDispatch` @0x8fb52, its only caller, @0x8fde9). A leader
//! issues on the frames where the frame counter (`_frameCounter`,
//! `nova_sim`'s pass frame) % [`ISSUE_PERIOD`] equals its slot >> 3, by
//! C's truncating remainder (@0x8fdbc-0x8fde4, [`issues_on`]): slots 0-7 on
//! multiples of 8, slots 8-15 a frame later, and so on, and on the
//! negative frames after a take-off only slots 0-7, on frame -8. It
//! issues while some NPC it leads is in the system (+0xc0, set by
//! `_DoPlayGameWork` @0x44064-0x44129 on the parent of every ship of AI
//! type above 4; [`has_escorts`](crate::ai::candidates::has_escorts)). The
//! call comes after the dispatch's gates ([`issues`]): not an escape pod
//! (@0x8fb62), not gliding in or bursting out of a gate past its first
//! tick (+0x50 below -900.0, @0x8fcf5), and not in its own jump or
//! following its leader's (sub 4 or 0xd, @0x8fd83-0x8fd9a). It comes
//! before the AI timer, the disabled reset and goal 0x15's skip, so a
//! disabled leader and one waiting in its gate still issue, and before
//! its own think, so it reads the goal and target its last think left.
//!
//! **The table** ([`orders`]), set to -1 first and then wholly written by
//! every path, so it never issues -1:
//!
//! - **A trader leader**, the original's AI type 2 or less (@0x89888,
//!   [`Npc::original_ai_type`](crate::traffic::npc::Npc::original_ai_type)):
//!   fighters Attack while its shield is at least [`HIGH_SHIELD`] of
//!   its capacity (`setbe` @0x898bb), else Defend; medium ships and
//!   warships Defend; freighters formation (0). It reads no target.
//! - **Any other leader** reads its target t (+0x70, @0x898c8):
//!   - dis: whether t is disabled (`_IsDisabled` @0x8995d), read whether
//!     or not t is still in its slot, as nothing tests it (a gone NPC by
//!     its record, [`Surroundings::held_gone`]);
//!   - in and out: whether t reaches the leader and the leader reaches t
//!     (`_AIInGunRange(t, leader, 1)` @0x8991c and
//!     `_AIInGunRange(leader, t, 1)` @0x89945), worked out only while t
//!     attacks the leader (its target the leader, its goal 4,
//!     @0x898e5-0x898ff), otherwise both false. The third argument names
//!     weapon index 1, `wëap` 129 ([`ORDER_WEAPON`], the stock Medium
//!     Blaster, reaching 382), for both calls, and the distance is
//!     symmetric, so in and out are always equal by the engine:
//!     "in and not out" never holds. With no `wëap` 129 (plug-in data) no
//!     ship is within reach, where the original reads whatever the weapon
//!     table's slot 1 holds (not traced). By the other reading of
//!     [`RuleKey::EscortOrderReach`](crate::RuleKey::EscortOrderReach)
//!     each ship's own stock reach counts ([`fire::in_reach`]). The player
//!     never attacks a leader in this sense, as its goal word stays 0
//!     (`_ResetPlayer` @0x1d50f).
//!   - A disabled target while plundering (goal 0xd, @0x89966-0x8997e):
//!     all Return, freighters formation.
//!   - Shield below [`LOW_SHIELD`] of capacity, strictly (`ja`
//!     @0x899ab-0x899b8): dis, or in and not out, gives fighters and
//!     medium ships Attack and warships Defend; otherwise all three
//!     Defend.
//!   - Below [`HIGH_SHIELD`] (@0x899de-0x89a2a): fighters Attack when dis,
//!     or in and not out, else Defend; medium ships Attack; warships
//!     Defend.
//!   - At least [`HIGH_SHIELD`] (@0x89a3c-0x89a81): fighters Attack when
//!     dis, else Defend while out and Attack while not; medium ships
//!     Attack; warships Attack while the leader's odds (its cached +0xc914,
//!     [`cached_odds`]) are in [`ODDS_BAND`], else formation, a NaN or a
//!     stale -1 formation too.
//!   - Freighters formation (@0x89a87).
//! - **The goal overrides**, after either branch (@0x89a8d-0x89b0a): a
//!   leader neither attacking (goal 4, [`Goal::Attack`] or
//!   [`Goal::Snipe`]), plundering (0xd), fleeing (3) nor regrouping (0x13)
//!   orders all four Return, freighters too; one fleeing while braking for
//!   its jump (sub 1, [`Leg::Brake`]) all four Return too; any other flee
//!   (running out, sub 3, or fleeing close within
//!   [`FLEE_REACH`](crate::ai::pursuit::FLEE_REACH) of its attacker, sub
//!   5) gives fighters and medium ships Defend. A leader waiting in its
//!   gate ([`Mode::Emerging`], or bursting out on its first tick) is goal
//!   0x15: all Return. Goal 0x13 is `_AICheckParentExists`'s regroup
//!   pause ([`Goal::Regroup`]), so a led carrier whose fleet changed lead
//!   gives its fighters orders by the bands while it pauses.
//!
//! The shield's tests are in doubles, as the original's (0.66 @0xdd5b8,
//! 0.33 @0xdd568); the odds band in floats (0.0 @0xdd054, 0.5 @0xdd694).
//! Nothing here plays a sound or draws a random number.

use crate::ai::leave::{self, Leg};
use crate::ai::odds::cached_odds;
use crate::ai::pursuit::{self, Start};
use crate::ai::{Goal, Surroundings, dispatch_gate, fire};
use crate::catalog::{EscortClass, WeaponId};
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::fleet::OrderWord;
use crate::flight::ShipState;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{Mode, Npc};

/// How many frames apart a leader issues its orders (`% 8` @0x8fdd4).
pub const ISSUE_PERIOD: i32 = 8;

/// The fraction of its shield capacity at and above which a leader's
/// shield is high (0.66 @0xdd5b8).
pub const HIGH_SHIELD: f64 = 0.66;

/// The fraction of its shield capacity below which a leader's shield is
/// low (0.33 @0xdd568).
pub const LOW_SHIELD: f64 = 0.33;

/// The odds a high-shielded leader's warships attack within: from 0.0
/// (@0xdd054) up to 0.5 (@0xdd694).
pub const ODDS_BAND: std::ops::Range<f32> = 0.0..0.5;

/// The weapon whose reach the gun-range inputs read by the engine: weapon
/// index 1 of the weapon table, `wëap` 129.
pub const ORDER_WEAPON: WeaponId = WeaponId(129);

/// 1: Defend.
const DEFEND: i16 = 1;
/// 2: Attack.
const ATTACK: i16 = 2;
/// 3: Return.
const RETURN: i16 = 3;
/// 0: formation.
const FORM: i16 = 0;

/// Whether a leader in ship-table slot `berth` issues on `frame` (see the
/// module docs).
#[must_use]
pub fn issues_on(frame: i16, berth: u32) -> bool {
    // `%` truncates toward zero, as C's does.
    i64::from(frame) % i64::from(ISSUE_PERIOD) == i64::from(berth >> 3)
}

/// Whether `leader` reaches the call past `_AIDispatch`'s gates (see the
/// module docs, and [`dispatch_gate`]).
#[must_use]
pub fn issues(leader: &Npc) -> bool {
    dispatch_gate::reaches_think(leader)
}

/// The order words `leader` issues among `around`, one per
/// [`EscortClass`] in its order, the gun-range inputs read by `reach`'s
/// reading of [`RuleKey::EscortOrderReach`](crate::RuleKey::EscortOrderReach)
/// (see the module docs).
#[must_use]
pub fn orders(leader: &Npc, around: &Surroundings, reach: RuleSource) -> [OrderWord; 4] {
    let banded = if trader_leader(leader) {
        by_trader(leader)
    } else {
        by_target(leader, around, reach)
    };
    overridden(leader, around, banded).map(OrderWord)
}

/// The word of `table` for a ship of `class`.
#[must_use]
pub fn for_class(table: [OrderWord; 4], class: EscortClass) -> OrderWord {
    let index = match class {
        EscortClass::Fighter => 0,
        EscortClass::Medium => 1,
        EscortClass::Warship => 2,
        EscortClass::Freighter => 3,
    };
    table[index]
}

/// Whether `leader` is the original's AI type 2 or less (@0x89888,
/// [`Npc::original_ai_type`]): a trader that neither follows a leader nor
/// escorts the player (AI types 5 and 6), or one the player has paid off.
fn trader_leader(leader: &Npc) -> bool {
    leader.original_ai_type() <= 2
}

/// Whether `leader`'s shield is below `fraction` of its capacity, in
/// doubles.
fn shield_below(leader: &Npc, fraction: f64) -> bool {
    let shield = leader.reserves.shield;
    f64::from(shield.now) < fraction * f64::from(shield.max)
}

/// A trader leader's table (@0x89888-0x898bf).
fn by_trader(leader: &Npc) -> [i16; 4] {
    let fighter = if shield_below(leader, HIGH_SHIELD) {
        DEFEND
    } else {
        ATTACK
    };
    [fighter, DEFEND, DEFEND, FORM]
}

/// Any other leader's table, by its target (@0x898c8-0x89a87).
fn by_target(leader: &Npc, around: &Surroundings, reach: RuleSource) -> [i16; 4] {
    let target = Target::of(leader, around);
    let (reaches_leader, leader_reaches) = target.attacker.map_or((false, false), |attacker| {
        in_and_out(leader, attacker, around, reach)
    });
    let dis = target.disabled;
    if dis && leader.goal.plundering().is_some() {
        return [RETURN, RETURN, RETURN, FORM];
    }
    let pressed = dis || (reaches_leader && !leader_reaches);
    if shield_below(leader, LOW_SHIELD) {
        if pressed {
            [ATTACK, ATTACK, DEFEND, FORM]
        } else {
            [DEFEND, DEFEND, DEFEND, FORM]
        }
    } else if shield_below(leader, HIGH_SHIELD) {
        let fighter = if pressed { ATTACK } else { DEFEND };
        [fighter, ATTACK, DEFEND, FORM]
    } else {
        let fighter = if dis || !leader_reaches {
            ATTACK
        } else {
            DEFEND
        };
        let warship = if ODDS_BAND.contains(&cached_odds(leader, around)) {
            ATTACK
        } else {
            FORM
        };
        [fighter, ATTACK, warship, FORM]
    }
}

/// What a leader reads of its target (+0x70).
struct Target<'a> {
    /// Whether it is disabled (`_IsDisabled`).
    disabled: bool,
    /// The target while it attacks the leader: its target the leader and
    /// its goal 4.
    attacker: Option<&'a Npc>,
}

impl<'a> Target<'a> {
    /// What `leader` reads of its target among `around`: an NPC in the
    /// system, or by its record once gone; the player for its condition
    /// alone, as it never attacks in this sense.
    fn of(leader: &Npc, around: &Surroundings<'a>) -> Self {
        match leader.target {
            Some(ShipRef::Npc(id)) => {
                let ship = around.npc(id).or_else(|| around.held_gone(leader));
                Self {
                    disabled: ship.is_some_and(|ship| ship.condition.disabled()),
                    attacker: ship.filter(|ship| {
                        ship.target == Some(ShipRef::Npc(leader.id))
                            && ship.goal.attacking().is_some()
                    }),
                }
            }
            Some(ShipRef::Player) => Self {
                disabled: around
                    .condition_of(ShipRef::Player)
                    .is_some_and(Condition::disabled),
                attacker: None,
            },
            None => Self {
                disabled: false,
                attacker: None,
            },
        }
    }
}

/// Whether `attacker` reaches `leader`, and `leader` reaches `attacker`,
/// by `reach` (see the module docs): by the engine both by
/// [`ORDER_WEAPON`]'s reach, read from `around`'s weapon table; by the
/// other reading each by its own stock reach.
fn in_and_out(
    leader: &Npc,
    attacker: &Npc,
    around: &Surroundings,
    reach: RuleSource,
) -> (bool, bool) {
    match reach {
        RuleSource::Engine => {
            let within = around
                .weapons
                .weapon(ORDER_WEAPON)
                .is_some_and(|spec| within(&leader.state, &attacker.state, spec.gun_reach()));
            (within, within)
        }
        RuleSource::Bible => (
            fire::in_reach(attacker, &leader.state),
            fire::in_reach(leader, &attacker.state),
        ),
    }
}

/// Whether `a` and `b` are within `reach` of each other, as
/// [`fire::in_reach`] measures.
fn within(a: &ShipState, b: &ShipState, reach: f32) -> bool {
    let off = b.position - a.position;
    off.x * off.x + off.y * off.y <= reach * reach
}

/// `banded` after `leader`'s goal overrides among `around`
/// (@0x89a8d-0x89b0a).
fn overridden(leader: &Npc, around: &Surroundings, banded: [i16; 4]) -> [i16; 4] {
    // Waiting in its gate, or on its burst's first tick, it is goal 0x15.
    if matches!(
        leader.mode,
        Mode::Emerging { .. } | Mode::LeavingGate { tick: 0 }
    ) {
        return [RETURN; 4];
    }
    match leader.goal {
        Goal::Attack(_) | Goal::Snipe(_) | Goal::Plunder { .. } | Goal::Regroup { .. } => banded,
        Goal::Flee(attacker) if braking(leader, attacker, around) => [RETURN; 4],
        Goal::Flee(_) => [DEFEND, DEFEND, banded[2], banded[3]],
        _ => [RETURN; 4],
    }
}

/// Whether `leader`, fleeing `attacker` among `around`, is braking for its
/// jump (sub 1): beyond its attacker's
/// [`FLEE_REACH`](crate::ai::pursuit::FLEE_REACH), past 1000 of the
/// centre, able to jump, and moving ([`leave::flee_leg`]). With its
/// attacker unseen, it flies no flee leg.
fn braking(leader: &Npc, attacker: ShipRef, around: &Surroundings) -> bool {
    around.state_of(attacker).is_some_and(|at| {
        pursuit::high_level(leader, &at, false, None) == Some(Start::RunningOut)
            && leave::flee_leg(
                &leader.state,
                leave::expedites(leader),
                leave::can_jump(leader, around.jump_fuel),
            ) == Leg::Brake
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, n, player, ship};
    use crate::catalog::WeaponRecord;
    use crate::combat::armament::Arsenal;
    use crate::combat::weapon::WeaponSpec;
    use crate::geometry::Vec2;
    use crate::traffic::gone::{Freed, GoneShip};
    use crate::traffic::npc::{AiType, NpcId};
    use crate::traffic::warp_out::WarpGoal;
    use nova_sim_catalog::fixture::weapon;

    /// The stock Medium Blaster, `wëap` 129: range 350, reach 382.
    fn medium_blaster() -> Arsenal {
        Arsenal::of_specs([WeaponSpec::new(&WeaponRecord {
            speed: 1400,
            count: 25,
            ..weapon(129)
        })])
    }

    /// The leader, NPC 1, a pirate warship at the centre with `goal`, its
    /// shield `shield` of 100, its odds cache stale.
    fn leader(goal: Goal, shield: f32) -> Npc {
        let mut npc = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        npc.odds = crate::traffic::odds_cache::STALE;
        npc.goal = goal;
        npc.reserves.shield.max = 100.0;
        npc.reserves.shield.now = shield;
        npc
    }

    /// `leader` targeting `target`.
    fn aiming(mut leader: Npc, target: ShipRef) -> Npc {
        leader.target = Some(target);
        leader
    }

    /// NPC `id`, a trader warship at (0, `y`) attacking the leader.
    fn attacker(id: u32, y: f32) -> Npc {
        let mut npc = ship(id, TRADERS, AiType::Warship, 0.0, y);
        npc.goal = Goal::Attack(n(1));
        npc.target = Some(n(1));
        npc
    }

    /// `npc` disabled.
    fn disabled(mut npc: Npc) -> Npc {
        npc.condition = Condition::Disabled;
        npc
    }

    /// The raw words `npcs[0]` issues among `npcs`, the player far off and
    /// the Medium Blaster in the weapon table, by `reach`.
    fn issued_by(npcs: &[Npc], reach: RuleSource) -> [i16; 4] {
        let weapons = medium_blaster();
        let around = Surroundings {
            player: Some(player(0.0, 5000.0)),
            weapons: &weapons,
            ..Surroundings::new(&[], npcs)
        };
        orders(&npcs[0], &around, reach).map(|word| word.0)
    }

    /// [`issued_by`] by the engine.
    fn issued(npcs: &[Npc]) -> [i16; 4] {
        issued_by(npcs, RuleSource::Engine)
    }

    #[test]
    fn a_trader_leaders_fighters_attack_at_two_thirds_shield_and_defend_below() {
        // @0x89888-0x898bf, @0x899d6, @0x89a34, @0x89a87.
        let trader = |shield: f32| {
            let mut npc = leader(Goal::Attack(n(2)), shield);
            npc.ai_type = AiType::BraveTrader;
            npc
        };
        assert_eq!(issued(&[trader(66.0)]), [2, 1, 1, 0]);
        assert_eq!(issued(&[trader(100.0)]), [2, 1, 1, 0]);
        assert_eq!(issued(&[trader(65.99)]), [1, 1, 1, 0]);
        let mut wimpy = trader(10.0);
        wimpy.ai_type = AiType::WimpyTrader;
        assert_eq!(issued(&[wimpy]), [1, 1, 1, 0]);
        // It reads no target: a disabled one changes nothing.
        let npcs = [aiming(trader(66.0), n(2)), disabled(attacker(2, 100.0))];
        assert_eq!(issued(&npcs), [2, 1, 1, 0]);
    }

    #[test]
    fn a_trader_that_follows_a_leader_or_escorts_the_player_reads_the_other_table() {
        // A led ship is the original's AI type 5 or 6, above 2.
        let mut led = leader(Goal::Attack(n(2)), 100.0);
        led.ai_type = AiType::WimpyTrader;
        led.leader = Some(NpcId(9));
        assert_eq!(issued(&[led.clone()]), [2, 2, 0, 0], "stale odds");
        let mut escort = led;
        escort.leader = None;
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        assert_eq!(issued(&[escort]), [2, 2, 0, 0]);
    }

    #[test]
    fn a_marked_former_escort_reads_the_other_table_and_a_spared_led_trader_the_traders() {
        // The disabled reset leaves 6; the bribe writes 1, parent kept.
        let mut marked = leader(Goal::Attack(n(2)), 100.0);
        marked.ai_type = AiType::WimpyTrader;
        marked.kept_escort_ai = true;
        assert_eq!(issued(&[marked.clone()]), [2, 2, 0, 0], "stale odds");
        let mut spared = marked;
        spared.kept_escort_ai = false;
        spared.leader = Some(NpcId(9));
        spared.spared = true;
        spared.ai_type = AiType::Warship;
        assert_eq!(issued(&[spared]), [2, 1, 1, 0]);
    }

    #[test]
    fn below_a_third_of_its_shield_a_leader_orders_attack_on_a_disabled_target_else_defend() {
        // @0x899ab-0x89a34.
        let low = |target| aiming(leader(Goal::Attack(n(2)), 32.99), target);
        let npcs = [low(n(2)), disabled(attacker(2, 100.0))];
        assert_eq!(issued(&npcs), [2, 2, 1, 0]);
        let npcs = [low(n(2)), attacker(2, 100.0)];
        assert_eq!(issued(&npcs), [1, 1, 1, 0], "in and out alike");
        let npcs = [low(n(2)), attacker(2, 3000.0)];
        assert_eq!(issued(&npcs), [1, 1, 1, 0]);
        assert_eq!(issued(&[leader(Goal::Attack(n(2)), 0.0)]), [1, 1, 1, 0]);
    }

    #[test]
    fn from_a_third_to_two_thirds_a_leaders_fighters_go_by_dis_alone() {
        // @0x899de-0x89a2a: 33.0 is not below a third.
        let middle = |shield| aiming(leader(Goal::Attack(n(2)), shield), n(2));
        for shield in [33.0, 65.99] {
            let npcs = [middle(shield), disabled(attacker(2, 100.0))];
            assert_eq!(issued(&npcs), [2, 2, 1, 0], "{shield}");
            let npcs = [middle(shield), attacker(2, 100.0)];
            assert_eq!(issued(&npcs), [1, 2, 1, 0], "{shield}");
        }
    }

    #[test]
    fn at_two_thirds_a_leaders_fighters_defend_while_an_attacker_is_within_the_blasters_reach() {
        // @0x89a3c-0x89a57: out is the Medium Blaster's 382.
        let high = || aiming(leader(Goal::Attack(n(2)), 66.0), n(2));
        let npcs = [high(), disabled(attacker(2, 100.0))];
        assert_eq!(issued(&npcs)[..2], [2, 2], "dis");
        let npcs = [high(), attacker(2, 382.0)];
        assert_eq!(issued(&npcs)[..2], [1, 2], "within");
        let npcs = [high(), attacker(2, 383.0)];
        assert_eq!(issued(&npcs)[..2], [2, 2], "beyond");
        // Not attacking the leader: its goal not 4, or its target another.
        let mut idle = attacker(2, 100.0);
        idle.goal = Goal::Idle;
        assert_eq!(issued(&[high(), idle])[..2], [2, 2], "goal not 4");
        let mut elsewhere = attacker(2, 100.0);
        elsewhere.target = Some(n(3));
        elsewhere.goal = Goal::Attack(n(3));
        assert_eq!(issued(&[high(), elsewhere])[..2], [2, 2], "another");
        let mut sniping = attacker(2, 100.0);
        sniping.goal = Goal::Snipe(n(1));
        assert_eq!(issued(&[high(), sniping])[..2], [1, 2], "a snipe is goal 4");
    }

    #[test]
    fn the_blasters_reach_is_measured_between_the_two_ships_wherever_they_are() {
        // Off the centre, and across: 382 within, 383 beyond.
        let at = |x: f32| {
            let mut high = aiming(leader(Goal::Attack(n(2)), 66.0), n(2));
            high.state.position = Vec2::new(1000.0, 1000.0);
            let mut across = attacker(2, 0.0);
            across.state.position = Vec2::new(1000.0 + x, 1000.0);
            issued(&[high, across])[0]
        };
        assert_eq!(at(382.0), 1, "within");
        assert_eq!(at(383.0), 2, "beyond");
        assert_eq!(at(-382.0), 1, "within, the other side");
    }

    #[test]
    fn with_no_weapon_129_by_the_engine_no_ship_is_within_reach() {
        let npcs = [
            aiming(leader(Goal::Attack(n(2)), 66.0), n(2)),
            attacker(2, 10.0),
        ];
        let around = Surroundings::new(&[], &npcs);
        let words = orders(&npcs[0], &around, RuleSource::Engine);
        assert_eq!(words[0], OrderWord(2));
    }

    #[test]
    fn a_high_shielded_leaders_warships_attack_while_its_odds_are_from_0_to_below_a_half() {
        // @0x89a5d-0x89a81, read from the cache by the engine.
        for (odds, word) in [
            (0.0, 2),
            (0.49, 2),
            (0.5, 0),
            (-0.01, 0),
            (-1.0, 0),
            (f32::NAN, 0),
            (3.0, 0),
        ] {
            let mut high = leader(Goal::Attack(n(2)), 100.0);
            high.odds = odds;
            assert_eq!(issued(&[high]), [2, 2, word, 0], "{odds}");
        }
    }

    #[test]
    fn by_the_other_odds_reading_a_leader_reads_its_odds_fresh() {
        // Its cache says stale; the scene holds no enemy, so odds 0.
        let mut high = leader(Goal::Attack(n(2)), 100.0);
        high.odds = -1.0;
        let npcs = [high];
        let around = Surroundings {
            odds_cache: RuleSource::Bible,
            ..Surroundings::new(&[], &npcs)
        };
        let words = orders(&npcs[0], &around, RuleSource::Engine);
        assert_eq!(words[2], OrderWord(2));
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(
            orders(&npcs[0], &around, RuleSource::Engine)[2],
            OrderWord(0)
        );
    }

    #[test]
    fn a_leader_plundering_a_disabled_ship_orders_return_and_its_freighters_formation() {
        // @0x89966-0x8997e, whatever its shield.
        let plunder = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        for shield in [0.0, 50.0, 100.0] {
            let npcs = [
                aiming(leader(plunder, shield), n(2)),
                disabled(attacker(2, 100.0)),
            ];
            assert_eq!(issued(&npcs), [3, 3, 3, 0], "{shield}");
        }
        let npcs = [aiming(leader(plunder, 100.0), n(2)), attacker(2, 3000.0)];
        assert_eq!(issued(&npcs), [2, 2, 0, 0], "intact: banded");
        let npcs = [
            aiming(leader(Goal::Attack(n(2)), 100.0), n(2)),
            disabled(attacker(2, 3000.0)),
        ];
        assert_eq!(issued(&npcs), [2, 2, 0, 0], "attacking: banded");
    }

    #[test]
    fn a_leader_neither_attacking_plundering_nor_fleeing_orders_all_return() {
        // @0x89a94-0x89ac1, and goal 2 @0x89af2-0x89b0a.
        for goal in [
            Goal::Idle,
            Goal::Wait,
            Goal::JumpOut,
            Goal::Formation { guard: None },
            Goal::Follow {
                lead: NpcId(9),
                guard: None,
            },
            Goal::Dock(n(9)),
            Goal::Inspect(n(2)),
            Goal::Drift,
        ] {
            assert_eq!(issued(&[leader(goal, 100.0)]), [3, 3, 3, 3], "{goal:?}");
        }
        let plunder = Goal::Plunder {
            ship: n(2),
            hopeless: false,
        };
        for goal in [Goal::Attack(n(2)), Goal::Snipe(n(2)), plunder] {
            assert_eq!(issued(&[leader(goal, 100.0)]), [2, 2, 0, 0], "{goal:?}");
        }
    }

    /// A leader at `at` fleeing NPC 2 at `from`, `moving` or still, able to
    /// jump, its shield full and its odds in band.
    fn fleeing(at: (f32, f32), moving: bool, from: (f32, f32)) -> [Npc; 2] {
        let mut runner = leader(Goal::Flee(n(2)), 100.0);
        runner.odds = 0.0;
        runner.class_fuel = 300;
        runner.state.position = Vec2::new(at.0, at.1);
        if moving {
            runner.state.velocity = Vec2::new(5.0, 0.0);
        }
        let mut chaser = attacker(2, 0.0);
        chaser.state.position = Vec2::new(from.0, from.1);
        [aiming(runner, n(2)), chaser]
    }

    #[test]
    fn a_leader_in_the_regroup_pause_issues_its_banded_table_not_all_return() {
        // Goal 0x13 is exempt from the override (@0x89aa9-0x89aad).
        use crate::ai::regroup::Kept;
        for kept in [Kept::Coast, Kept::Stop, Kept::Formation { guard: None }] {
            let paused = leader(Goal::Regroup { kept }, 100.0);
            for reach in RuleSource::ALL {
                assert_eq!(
                    issued_by(std::slice::from_ref(&paused), reach),
                    [2, 2, 0, 0],
                    "{kept:?} {reach:?}"
                );
            }
            let low = leader(Goal::Regroup { kept }, 20.0);
            assert_eq!(issued(&[low]), [1, 1, 1, 0], "{kept:?}");
        }
    }

    #[test]
    fn a_leader_fleeing_while_braking_for_its_jump_orders_all_return() {
        // Sub 1 (@0x8e1fe; @0x89acd-0x89ae2, @0x89af8): beyond 1000, able
        // to jump, moving, its attacker beyond 250.
        assert_eq!(
            issued(&fleeing((1100.0, 0.0), true, (0.0, 0.0))),
            [3, 3, 3, 3]
        );
    }

    #[test]
    fn a_leader_fleeing_otherwise_orders_its_fighters_and_medium_ships_to_defend() {
        // @0x89ae4-0x89aea: running out (sub 3), within 250 (sub 5), or
        // about to jump (sub 4, which issues nothing once under way).
        let runs_out = fleeing((500.0, 0.0), true, (0.0, 0.0));
        assert_eq!(issued(&runs_out), [1, 1, 2, 0]);
        // Warships and freighters keep their bands: a low shield's Defend,
        // a full shield's formation on stale odds.
        let mut low = fleeing((500.0, 0.0), true, (0.0, 0.0));
        low[0].reserves.shield.now = 32.99;
        assert_eq!(issued(&low), [1, 1, 1, 0], "low shield");
        let mut stale = fleeing((500.0, 0.0), true, (0.0, 0.0));
        stale[0].odds = -1.0;
        assert_eq!(issued(&stale), [1, 1, 0, 0], "stale odds");
        let close = fleeing((1100.0, 0.0), true, (1100.0, 250.0));
        assert_eq!(issued(&close), [1, 1, 2, 0]);
        let still = fleeing((1100.0, 0.0), false, (0.0, 0.0));
        assert_eq!(issued(&still), [1, 1, 2, 0]);
        let mut unable = fleeing((1100.0, 0.0), true, (0.0, 0.0));
        unable[0].class_fuel = 0;
        assert_eq!(issued(&unable), [1, 1, 2, 0], "it runs out");
        let mut lost = fleeing((1100.0, 0.0), true, (0.0, 0.0));
        lost[0].goal = Goal::Flee(n(9));
        assert_eq!(issued(&lost), [1, 1, 2, 0], "its attacker unseen");
    }

    #[test]
    fn a_leader_waiting_in_its_gate_is_goal_0x15_and_orders_all_return() {
        for mode in [
            Mode::Emerging { ticks_left: 5 },
            Mode::LeavingGate { tick: 0 },
        ] {
            let mut waiting = leader(Goal::Attack(n(2)), 100.0);
            waiting.mode = mode;
            assert_eq!(issued(&[waiting]), [3, 3, 3, 3], "{mode:?}");
        }
        let mut held = leader(Goal::Attack(n(2)), 100.0);
        held.mode = Mode::HeldAtGate { ticks_left: 5 };
        assert_eq!(issued(&[held]), [2, 2, 0, 0], "held attacking: banded");
    }

    #[test]
    fn by_the_other_reach_reading_a_target_outranging_the_leader_is_in_and_not_out() {
        // Weapon -1: the attacker's own reach covers the leader at 300,
        // the leader's own (100) doesn't cover it.
        let mut low = aiming(leader(Goal::Attack(n(2)), 10.0), n(2));
        low.hull.stock_reach = Some(100.0);
        let mut far = attacker(2, 300.0);
        far.hull.stock_reach = Some(400.0);
        let npcs = [low.clone(), far.clone()];
        assert_eq!(issued_by(&npcs, RuleSource::Bible), [2, 2, 1, 0]);
        assert_eq!(issued_by(&npcs, RuleSource::Engine), [1, 1, 1, 0]);
        // Both reach: in and out.
        low.hull.stock_reach = Some(400.0);
        assert_eq!(issued_by(&[low, far], RuleSource::Bible), [1, 1, 1, 0]);
        // In the middle band, the fighter alone.
        let mut middle = aiming(leader(Goal::Attack(n(2)), 50.0), n(2));
        middle.hull.stock_reach = Some(100.0);
        let mut far = attacker(2, 300.0);
        far.hull.stock_reach = Some(400.0);
        assert_eq!(issued_by(&[middle, far], RuleSource::Bible), [2, 2, 1, 0]);
        // At the top, out by its own reach.
        let mut high = aiming(leader(Goal::Attack(n(2)), 100.0), n(2));
        high.hull.stock_reach = Some(400.0);
        let mut near = attacker(2, 300.0);
        near.hull.stock_reach = Some(100.0);
        assert_eq!(issued_by(&[high, near], RuleSource::Bible)[0], 1);
    }

    #[test]
    fn a_target_gone_from_its_slot_is_read_by_its_record() {
        let npcs = [aiming(leader(Goal::Attack(n(2)), 10.0), n(2))];
        let mut record = disabled(attacker(2, 100.0));
        record.berth = 0;
        let mut reader = npcs[0].clone();
        reader.berth = 5;
        let npcs = [reader];
        let gone = [GoneShip {
            ship: record,
            freed: Freed::InDispatch,
        }];
        let around = Surroundings {
            gone: &gone,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(
            orders(&npcs[0], &around, RuleSource::Engine).map(|word| word.0),
            [2, 2, 1, 0]
        );
    }

    #[test]
    fn a_player_target_is_read_for_dis_alone() {
        let low = aiming(leader(Goal::Attack(ShipRef::Player), 10.0), ShipRef::Player);
        let npcs = [low];
        let around = |condition| Surroundings {
            player: Some(crate::ai::PlayerSide {
                condition,
                ..player(0.0, 100.0)
            }),
            ..Surroundings::new(&[], &npcs)
        };
        let words =
            |condition| orders(&npcs[0], &around(condition), RuleSource::Bible).map(|word| word.0);
        assert_eq!(words(Condition::Disabled), [2, 2, 1, 0]);
        assert_eq!(words(Condition::Intact), [1, 1, 1, 0], "never attacking");
    }

    #[test]
    fn each_class_takes_its_own_word() {
        let table = [OrderWord(1), OrderWord(2), OrderWord(3), OrderWord(0)];
        assert_eq!(for_class(table, EscortClass::Fighter), OrderWord(1));
        assert_eq!(for_class(table, EscortClass::Medium), OrderWord(2));
        assert_eq!(for_class(table, EscortClass::Warship), OrderWord(3));
        assert_eq!(for_class(table, EscortClass::Freighter), OrderWord(0));
    }

    #[test]
    fn a_leader_issues_when_the_frame_mod_8_is_its_slot_over_8() {
        // @0x8fdbc-0x8fde4, C's truncating remainder.
        for berth in 0..8 {
            assert!(issues_on(0, berth), "{berth}");
            assert!(issues_on(8, berth), "{berth}");
            assert!(issues_on(-8, berth), "{berth}");
            assert!(!issues_on(1, berth), "{berth}");
        }
        for berth in 8..16 {
            assert!(issues_on(1, berth), "{berth}");
            assert!(issues_on(1025, berth), "{berth}");
            assert!(!issues_on(0, berth), "{berth}");
            assert!(!issues_on(-8, berth), "{berth}");
        }
        assert!(issues_on(7, 63));
        assert!(!issues_on(6, 63));
        for berth in 0..64 {
            assert!(!issues_on(-3, berth), "{berth}");
            assert!(!issues_on(-7, berth), "{berth}");
        }
    }

    #[test]
    fn a_leader_gliding_in_bursting_out_or_jumping_issues_nothing() {
        let with = |mode| Npc {
            mode,
            ..leader(Goal::Idle, 100.0)
        };
        assert!(!issues(&with(Mode::JumpingIn { tick: 0 })));
        assert!(!issues(&with(Mode::LeavingGate { tick: 1 })));
        assert!(issues(&with(Mode::LeavingGate { tick: 0 })));
        assert!(issues(&with(Mode::Emerging { ticks_left: 3 })));
        assert!(issues(&with(Mode::HeldAtGate { ticks_left: 3 })));
        assert!(issues(&with(Mode::Coasting { ticks_left: 3 })));
        assert!(issues(&with(Mode::Flying)));
        let mut jumping = with(Mode::Flying);
        jumping.goal = Goal::JumpOut;
        jumping.warp.goal = Some(WarpGoal::Own);
        assert!(!issues(&jumping), "its own jump");
        let mut following = with(Mode::Flying);
        following.warp.goal = Some(WarpGoal::Follow(n(9)));
        assert!(!issues(&following), "its leader's");
        assert!(issues(&disabled(with(Mode::Flying))), "disabled");
        let mut pod = with(Mode::Flying);
        pod.ship = crate::combat::hull::ESCAPE_POD_SHIP;
        assert!(!issues(&pod));
        let mut hulk = with(Mode::Flying);
        hulk.no_ai = true;
        assert!(!issues(&hulk));
    }
}
