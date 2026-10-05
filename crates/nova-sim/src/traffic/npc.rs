//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use std::collections::BTreeMap;

use crate::ai::Goal;
use crate::catalog::{GovtId, ShipId, WeaponId};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::escort::EscortClass;
use crate::flight::ShipState;
use crate::reserves::Reserves;
use crate::stats::ShipStats;

/// An NPC's number in its system, unique while the player stays there:
/// the order it appeared in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NpcId(pub u32);

/// How an NPC behaves: the `düde` `AIType`, or a `shïp`'s `InherentAI`
/// (the Bible: 1 wimpy trader, 2 brave trader, 3 warship, 4 interceptor).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiType {
    /// 1: a trader that flees.
    WimpyTrader,
    /// 2: a trader that fights back.
    BraveTrader,
    /// 3: a warship.
    Warship,
    /// 4: an interceptor.
    Interceptor,
}

impl AiType {
    /// What a raw AI type other than 1-4 means: a wimpy trader, the most
    /// harmless, which lands rather than lingers. Stock ships all have an
    /// `InherentAI` of 1-4; this covers plug-in data that leaves it 0 or
    /// -1, where the original's behaviour has not been traced.
    pub const FALLBACK: Self = Self::WimpyTrader;

    /// The AI type `raw` means: 1-4 as the Bible says, anything else
    /// [`AiType::FALLBACK`].
    #[must_use]
    pub fn from_raw(raw: i16) -> Self {
        match raw {
            2 => Self::BraveTrader,
            3 => Self::Warship,
            4 => Self::Interceptor,
            _ => Self::FALLBACK,
        }
    }

    /// Whether it is a trader, wimpy or brave.
    #[must_use]
    pub fn trades(self) -> bool {
        matches!(self, Self::WimpyTrader | Self::BraveTrader)
    }
}

/// Whether an NPC is still coming out of hyperspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Gliding in from hyperspace, for this many more ticks: no steering
    /// and no decisions.
    JumpingIn {
        /// The ticks of the glide left.
        ticks_left: u32,
    },
    /// Flying under its own controls.
    Flying,
}

/// An NPC ship in the player's system.
#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    /// Its number.
    pub id: NpcId,
    /// Its ship type.
    pub ship: ShipId,
    /// Its government, or `None` for independent.
    pub govt: Option<GovtId>,
    /// How it behaves.
    pub ai_type: AiType,
    /// The fleet lead it escorts, if it is an escort.
    pub leader: Option<NpcId>,
    /// Its ship type's class as an escort, which the escorts' threat
    /// scores weigh (see [`escort`](crate::escort)).
    pub class: EscortClass,
    /// How it performs, its default items included.
    pub stats: ShipStats,
    /// Its shield, armour and fuel (100 is one jump).
    pub reserves: Reserves,
    /// Where it is and how it moves.
    pub state: ShipState,
    /// Whether it is still jumping in.
    pub mode: Mode,
    /// What it is doing.
    pub goal: Goal,
    /// How it is holding up.
    pub condition: Condition,
    /// Its ship type's hull.
    pub hull: HullSpec,
    /// Its weapons.
    pub armament: Armament,
    /// The rounds of each ammunition it holds.
    pub rounds: BTreeMap<WeaponId, u32>,
    /// The fire command it holds, as it last decided.
    pub trigger: Trigger,
    /// The ship it targets, as it last decided.
    pub target: Option<ShipRef>,
    /// The shield and armour damage it has taken while it fights: none
    /// once it goes back to an idle goal or is disabled.
    pub provoked: f32,
    /// How aggressive it is: 0, 2 or 3 for traffic.
    pub aggression: u8,
    /// The ship it last inspected, if any.
    pub inspected: Option<ShipRef>,
    /// Its `düde`'s `Booty` flags, what boarding it yields; none for a
    /// fleet's ship.
    pub booty: u16,
    /// Whether the player has boarded it: it cannot be boarded again.
    pub boarded: bool,
    /// Its `düde`'s `InfoTypes` flags, what it says when hailed (see
    /// [`hail`](crate::hail)); none for a fleet's ship.
    pub info_types: u16,
    /// Whether the player has paid it to spare it: it never targets the
    /// player again.
    pub spared: bool,
    /// The ticks it has spent within reach of the player, repairing it.
    pub assisting: u32,
}

impl Npc {
    /// Its fleet: the lead it escorts, or itself.
    #[must_use]
    pub fn fleet(&self) -> NpcId {
        self.leader.unwrap_or(self.id)
    }

    /// Whether it threatens `ship` (`_ExtendedIsThreatToShip`): it is
    /// intact, flying, and attacking, sniping at or fleeing from it.
    #[must_use]
    pub fn threatens(&self, ship: ShipRef) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && self.goal.quarry() == Some(ship)
    }

    /// Whether it threatens the player (`_IsThreatToPlayer` @0x7f501):
    /// intact and attacking, sniping at or fleeing from the player. There
    /// is no legal or government test; inspecting is no threat.
    #[must_use]
    pub fn threatens_player(&self) -> bool {
        self.threatens(ShipRef::Player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_ai_types_1_to_4_are_the_bibles_and_any_other_is_the_fallback() {
        assert_eq!(
            [1, 2, 3, 4].map(AiType::from_raw),
            [
                AiType::WimpyTrader,
                AiType::BraveTrader,
                AiType::Warship,
                AiType::Interceptor
            ]
        );
        for raw in [0, -1, 5, i16::MIN, i16::MAX] {
            assert_eq!(AiType::from_raw(raw), AiType::FALLBACK, "{raw}");
        }
        assert_eq!(AiType::FALLBACK, AiType::WimpyTrader);
    }

    #[test]
    fn an_npcs_fleet_is_its_lead_or_itself() {
        let lead = crate::testkit::npc(3, ShipStats::default());
        assert_eq!(lead.fleet(), NpcId(3));
        let escort = Npc {
            leader: Some(NpcId(3)),
            ..crate::testkit::npc(5, ShipStats::default())
        };
        assert_eq!(escort.fleet(), NpcId(3));
    }

    #[test]
    fn an_npc_threatens_the_ship_it_attacks_snipes_at_or_flees_from_while_intact() {
        let other = ShipRef::Npc(NpcId(9));
        for goal in [
            Goal::Attack(ShipRef::Player),
            Goal::Snipe(ShipRef::Player),
            Goal::Flee(ShipRef::Player),
        ] {
            let npc = Npc {
                goal,
                ..crate::testkit::npc(1, ShipStats::default())
            };
            assert!(npc.threatens_player(), "{goal:?}");
            assert!(!npc.threatens(other), "{goal:?}");
            for condition in [
                Condition::Disabled,
                Condition::Dying { ticks_left: 1 },
                Condition::Destroyed,
            ] {
                let downed = Npc {
                    condition,
                    ..npc.clone()
                };
                assert!(!downed.threatens_player(), "{goal:?} {condition:?}");
            }
            let arriving = Npc {
                mode: Mode::JumpingIn { ticks_left: 2 },
                ..npc.clone()
            };
            assert!(!arriving.threatens_player());
        }
        for goal in [
            Goal::Inspect(ShipRef::Player),
            Goal::Idle,
            Goal::Attack(other),
            Goal::Assist(crate::ai::Help::Refuel),
        ] {
            let npc = Npc {
                goal,
                ..crate::testkit::npc(1, ShipStats::default())
            };
            assert!(!npc.threatens_player(), "{goal:?}");
        }
        let hunter = Npc {
            goal: Goal::Attack(other),
            ..crate::testkit::npc(1, ShipStats::default())
        };
        assert!(hunter.threatens(other));
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }
}
