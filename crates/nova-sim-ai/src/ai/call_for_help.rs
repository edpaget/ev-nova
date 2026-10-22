//! The calls a think makes on the message line ([`CommCall`]): a trader's
//! or miner's distress call (`_AICallForHelp` @0x82ffb-0x83257 in the
//! `EV Nova` executable) and an interceptor's pre-scan hail
//! (`_InterceptorAI` @0x8ce4b-0x8cff5).
//!
//! - **Who calls for help.** `_AICallForHelp` is a tail call of
//!   `_WimpyTraderAI` (@0x8b1f5) and `_MinerAI` (@0x8b483) when the think
//!   ends in state 3 ([`Goal::Flee`], [`when_fleeing`]), and of
//!   `_BraveTraderAI` in state 3 (@0x8b6cc) and in state 4 against a ship
//!   of AI type above 2, a warship or interceptor (@0x8b6e3-0x8b71c,
//!   [`brave_call`]). Each think in that state calls, not only the first.
//! - **Whether it is made.** The session gathers each call's facts
//!   ([`DistressFacts`]) after the tick's decisions and asks
//!   [`calls_for_help`], which keeps `_AICallForHelp`'s gates in their
//!   order. Its threat gate is `_ExtendedIsThreatToShip(ship, player)`
//!   (@0x830b1 → @0x81a27), read as [`threatens_player`].
//! - **The pre-scan hail** is raised by the interceptor's fresh pick of
//!   the player ([`CommCall::PreScan`]; see
//!   [`interceptor`](super::interceptor)); the session tests its
//!   government, the mission cargo and the line.

use crate::ai::{Goal, Surroundings};
use crate::combat::ShipRef;
use crate::traffic::npc::{AiType, Npc};

/// A call a think makes on the message line, which the session answers
/// after the tick's decisions (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommCall {
    /// `_AICallForHelp`: a distress call, made when [`calls_for_help`].
    Distress,
    /// `_InterceptorAI`'s pre-scan hail: the interceptor has just picked
    /// the player to inspect (@0x8ce34-0x8ce4b).
    PreScan,
    /// `_AICallForReinforcements(ship, 0)`: a call for the system's
    /// reinforcement fleet ([`Decision::calls_reinforcements`](crate::ai::Decision::calls_reinforcements),
    /// see [`reinforcements`](crate::ai::reinforcements)), after the
    /// think's own call.
    Reinforcements,
}

/// The call of a wimpy trader's or a miner's think ending in `goal`: a
/// distress call in state 3, fleeing (`_WimpyTraderAI` @0x8b1e1-0x8b1f5,
/// `_MinerAI` @0x8b46c-0x8b483).
#[must_use]
pub fn when_fleeing(goal: Goal) -> Option<CommCall> {
    matches!(goal, Goal::Flee(_)).then_some(CommCall::Distress)
}

/// The call of a brave trader's think ending in `goal` among `around`: a
/// distress call fleeing (state 3, @0x8b6bc-0x8b6cc), or attacking an NPC
/// of AI type above 2, a warship or an interceptor (state 4, a target
/// neither -1 nor the player, @0x8b6e3-0x8b71c); a ship with no AI
/// ([`Npc::no_ai`](crate::traffic::npc::Npc::no_ai), -1) is neither. A
/// ship gone from its slot is read in its record as it left
/// ([`Surroundings::gone_npc`]), which the freed slot still holds.
#[must_use]
pub fn brave_call(goal: Goal, around: &Surroundings) -> Option<CommCall> {
    match goal {
        Goal::Flee(_) => Some(CommCall::Distress),
        Goal::Attack(ShipRef::Npc(id)) => around
            .npc(id)
            .or_else(|| around.gone_npc(id))
            .filter(|target| {
                !target.no_ai && matches!(target.ai_type, AiType::Warship | AiType::Interceptor)
            })
            .map(|_| CommCall::Distress),
        _ => None,
    }
}

/// What `_AICallForHelp` asks of a ship before it calls, each in its
/// order (see [`calls_for_help`]).
// Each is its own gate of the original's, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DistressFacts {
    /// It is a person (+0xc8d0 ≠ -1, @0x83010).
    pub person: bool,
    /// Its government's `Flags2` has 0x0008 (@0x8301e-0x83041).
    pub quiet: bool,
    /// Its class's inherent government's `Flags2` has 0x0008 (`shïp`
    /// +0x9f6, @0x83047-0x8307d).
    pub quiet_inherent: bool,
    /// The message line is free: `_commCount` ≤ 0 (@0x83083-0x8308c).
    pub line_free: bool,
    /// It likes the player (`_AIDoesShipLikePlayer` @0x83095).
    pub liked: bool,
    /// It threatens the player ([`threatens_player`], @0x830b1).
    pub threatens: bool,
    /// The player is dying (`_IsDying` @0x830c3).
    pub player_dying: bool,
    /// It is cloaked (`_ShipIsCloaked` @0x830d3).
    pub cloaked: bool,
}

/// Whether a ship of `facts` sends its distress call: not a person, of
/// no quiet government nor quiet inherent government, the line free,
/// liking the player and no threat to it, the player not dying, and the
/// ship not cloaked (`_AICallForHelp` @0x83010-0x830da).
#[must_use]
pub fn calls_for_help(facts: &DistressFacts) -> bool {
    !facts.person
        && !facts.quiet
        && !facts.quiet_inherent
        && facts.line_free
        && facts.liked
        && !facts.threatens
        && !facts.player_dying
        && !facts.cloaked
}

/// Whether `npc` threatens the player among `around`, as
/// `_ExtendedIsThreatToShip(ship, player)` (@0x81a27) asks it for
/// `_AICallForHelp`: never an inactive or disabled player
/// (`player_intact` false, @0x81a33-0x81a4a), nor one whose government
/// blesses the player (`_PlayerBlessedByGovt` @0x81a89), and otherwise
/// `_IsThreatToPlayer` (@0x81a9c, [`Npc::threatens_player_among`]). Its
/// test of the player led by the ship (@0x81a5a) never holds: the
/// player follows no NPC.
#[must_use]
pub fn threatens_player(npc: &Npc, around: &Surroundings, player_intact: bool) -> bool {
    player_intact && !around.blessed(npc.govt) && npc.threatens_player_among(around.npcs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::fixture::{PIRATES, TRADERS, around, blessing_of, govts, n, ship, sites};
    use crate::traffic::npc::NpcId;

    /// The facts of a ship that calls.
    fn calling() -> DistressFacts {
        DistressFacts {
            line_free: true,
            liked: true,
            ..DistressFacts::default()
        }
    }

    #[test]
    fn a_ship_that_likes_the_player_calls_while_the_line_is_free() {
        assert!(calls_for_help(&calling()));
    }

    #[test]
    fn each_gate_blocks_the_call() {
        type Flip = fn(&mut DistressFacts);
        let gates: [(&str, Flip); 8] = [
            ("a person", |facts| facts.person = true),
            ("quiet", |facts| facts.quiet = true),
            ("quiet inherent", |facts| facts.quiet_inherent = true),
            ("line busy", |facts| facts.line_free = false),
            ("disliked", |facts| facts.liked = false),
            ("a threat", |facts| facts.threatens = true),
            ("player dying", |facts| facts.player_dying = true),
            ("cloaked", |facts| facts.cloaked = true),
        ];
        for (gate, flip) in gates {
            let mut facts = calling();
            flip(&mut facts);
            assert!(!calls_for_help(&facts), "{gate}");
        }
    }

    #[test]
    fn a_wimpy_trader_or_miner_calls_only_fleeing() {
        assert_eq!(when_fleeing(Goal::Flee(n(2))), Some(CommCall::Distress));
        for goal in [Goal::Idle, Goal::Attack(n(2)), Goal::JumpOut] {
            assert_eq!(when_fleeing(goal), None, "{goal:?}");
        }
    }

    #[test]
    fn a_brave_trader_calls_fleeing_or_attacking_a_warship_or_interceptor() {
        let govts = govts(0, 0);
        let sites = sites();
        let npcs = [
            ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0),
            ship(2, PIRATES, AiType::Warship, 0.0, 100.0),
            ship(3, PIRATES, AiType::Interceptor, 0.0, 100.0),
            ship(4, PIRATES, AiType::BraveTrader, 0.0, 100.0),
            ship(5, PIRATES, AiType::WimpyTrader, 0.0, 100.0),
        ];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        let call = |goal| brave_call(goal, &around);
        assert_eq!(call(Goal::Flee(n(4))), Some(CommCall::Distress));
        assert_eq!(call(Goal::Attack(n(2))), Some(CommCall::Distress));
        assert_eq!(call(Goal::Attack(n(3))), Some(CommCall::Distress));
        assert_eq!(call(Goal::Attack(n(4))), None, "a brave trader");
        assert_eq!(call(Goal::Attack(n(5))), None, "a wimpy trader");
        assert_eq!(call(Goal::Attack(ShipRef::Player)), None, "the player");
        assert_eq!(call(Goal::Attack(ShipRef::Npc(NpcId(9)))), None, "gone");
        assert_eq!(call(Goal::Snipe(n(2))), None, "sniping");
        assert_eq!(call(Goal::Idle), None);
    }

    #[test]
    fn a_brave_trader_attacking_a_ship_with_no_ai_calls_for_none_whatever_its_class() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut hulk = ship(2, PIRATES, AiType::Warship, 0.0, 100.0);
        hulk.no_ai = true;
        let npcs = [ship(1, TRADERS, AiType::BraveTrader, 0.0, 0.0), hulk];
        let around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert_eq!(
            brave_call(Goal::Attack(n(2)), &around),
            None,
            "AI type -1 is not above 2 (@0x8b707)"
        );
    }

    #[test]
    fn a_ship_attacking_the_player_threatens_it_unless_blessed_or_the_player_is_downed() {
        let govts = govts(0, 0);
        let sites = sites();
        let mut pirate = ship(1, PIRATES, AiType::Warship, 0.0, 0.0);
        pirate.goal = Goal::Attack(ShipRef::Player);
        let npcs = [pirate];
        let mut around = around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert!(threatens_player(&npcs[0], &around, true));
        assert!(!threatens_player(&npcs[0], &around, false), "downed");
        if let Some(player) = around.player.as_mut() {
            player.blessing = blessing_of(PIRATES);
        }
        assert!(!threatens_player(&npcs[0], &around, true), "blessed");
        let mut idle = npcs[0].clone();
        idle.goal = Goal::Idle;
        let around = crate::ai::fixture::around(&sites, &npcs, &govts, (0.0, -100.0), 0);
        assert!(!threatens_player(&idle, &around, true), "idle");
    }
}
