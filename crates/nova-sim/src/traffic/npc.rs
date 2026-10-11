//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use std::collections::BTreeMap;

use crate::ai::Goal;
use crate::bay::Carrier;
use crate::catalog::{GovtId, PersonId, ShipId, WeaponId};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::escort::{EscortClass, EscortDuty, FormationSlot};
use crate::flight::ShipState;
use crate::mission::ships::ShipBehav;
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
    /// Coming out of a hypergate or wormhole after the player, for this
    /// many more ticks: at rest, seen by no one, untargetable and not
    /// drawn, with no decisions (`_AIMakeShipEmergeFromHyperGate`, goal
    /// 21; see the session's `gate_follow` module).
    Emerging {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
}

/// What an NPC flown by a person (`përs`) carries of it (see
/// [`person`](crate::person)).
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NpcPerson {
    /// The person.
    pub id: PersonId,
    /// Its `Flags`.
    pub flags: u16,
    /// Its `Coward`: the percent of its shields below which it runs.
    pub coward: i16,
    /// Its `CommQuote`, an entry in `STR#` 7100.
    pub comm_quote: i16,
    /// Its `HailQuote`, an entry in `STR#` 7101.
    pub hail_quote: i16,
    /// Whether it has a `LinkMission`.
    pub mission: bool,
    /// Its `HailPict`, which the comm dialog shows in place of its ship's
    /// picture.
    pub portrait: Option<i16>,
    /// Whether it is invincible (`ShieldMod` below 0): its shield and
    /// armour are refilled every fight tick.
    pub invincible: bool,
    /// Whether it holds a grudge against the player.
    pub grudge: bool,
    /// Whether it has said its hail quote this stay.
    pub quoted: bool,
    /// The quote tick it last said its hail quote, if it has.
    pub quoted_at: Option<u64>,
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
    /// Its duty as the player's escort, if it is one: none for traffic.
    pub escort: Option<EscortDuty>,
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
    /// How aggressive it is: 0, 2 or 3 for traffic; 1, 2 or 4 for a
    /// person.
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
    /// The ship that launched it from a fighter bay, which it docks with,
    /// while it has one (see [`bay`](crate::bay)); none for any other
    /// ship.
    pub carrier: Option<Carrier>,
    /// The person flying it, if any.
    pub person: Option<NpcPerson>,
    /// The mission whose special ship it is, while it is one (the
    /// original's ship +0xc8d2): none for any other ship.
    pub mission: Option<MissionMark>,
}

/// What marks an NPC as an active mission's special ship (see the
/// [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissionMark {
    /// The slot of the mission.
    pub slot: u8,
    /// The mission's `ShipBehav`.
    pub behav: ShipBehav,
    /// Whether it spawned cloaked (`ShipStart` 2): only recorded, as the
    /// engine has no cloak (rdm:phase/mission-ships/phase-16-cloaked-mission-ships).
    pub cloaked: bool,
    /// Its place in the player's formation once it has joined it as a
    /// mission escort (the original's AI type 6 led by the player): none
    /// before (see the session's `mission_ships` module). It is never an
    /// [`EscortDuty`], so every hired-escort path leaves it out.
    pub formation: Option<FormationSlot>,
}

impl Npc {
    /// Its fleet: the player, for the player's escort, hired or a mission
    /// escort; otherwise the lead it escorts, or itself. Ships of one fleet
    /// never hit each other (`_ShipsShareParent` @0x7926, which follows the
    /// leader +0x9a), and count each other's missiles friendly.
    #[must_use]
    pub fn fleet(&self) -> ShipRef {
        if self.escorts_player() {
            ShipRef::Player
        } else {
            ShipRef::Npc(self.leader.unwrap_or(self.id))
        }
    }

    /// Whether it escorts the player: a hired escort (its fighters too) or
    /// a mission escort.
    #[must_use]
    pub fn escorts_player(&self) -> bool {
        self.escort.is_some() || self.mission_escort()
    }

    /// Whether it is a mission escort: a mission's special ship that has
    /// joined the player's formation ([`MissionMark::formation`]).
    #[must_use]
    pub fn mission_escort(&self) -> bool {
        self.mission.is_some_and(|mark| mark.formation.is_some())
    }

    /// Its place in the player's formation: a hired escort's
    /// ([`EscortDuty`]) or a mission escort's; none for any other ship.
    #[must_use]
    pub fn formation(&self) -> Option<FormationSlot> {
        self.escort
            .map(|duty| duty.formation)
            .or_else(|| self.mission.and_then(|mark| mark.formation))
    }

    /// Whether it threatens `ship` (`_ExtendedIsThreatToShip`): it is
    /// intact, flying, and attacking, sniping at or fleeing from it.
    #[must_use]
    pub fn threatens(&self, ship: ShipRef) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && self.goal.quarry() == Some(ship)
    }

    /// Whether it is emerging from a hypergate or wormhole
    /// ([`Mode::Emerging`]).
    #[must_use]
    pub fn emerging(&self) -> bool {
        matches!(self.mode, Mode::Emerging { .. })
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
    fn an_npcs_fleet_is_its_lead_or_itself_and_the_players_escorts_the_players() {
        let lead = crate::testkit::npc(3, ShipStats::default());
        assert_eq!(lead.fleet(), ShipRef::Npc(NpcId(3)));
        let escort = Npc {
            leader: Some(NpcId(3)),
            ..crate::testkit::npc(5, ShipStats::default())
        };
        assert_eq!(escort.fleet(), ShipRef::Npc(NpcId(3)));
        let players = Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: 30.0,
                },
                order: None,
            }),
            ..crate::testkit::npc(6, ShipStats::default())
        };
        assert_eq!(players.fleet(), ShipRef::Player);
    }

    /// A mark of slot 1 of `behav`, holding `formation`.
    fn mark(behav: ShipBehav, formation: Option<FormationSlot>) -> MissionMark {
        MissionMark {
            slot: 1,
            behav,
            cloaked: false,
            formation,
        }
    }

    #[test]
    fn a_mission_escort_is_of_the_players_fleet_and_a_marked_ship_not_yet_joined_its_own() {
        let slot = FormationSlot {
            slot: 3,
            ships: 3,
            spacing: 40.0,
        };
        let joined = Npc {
            mission: Some(mark(ShipBehav::EscortPlayer, Some(slot))),
            ..crate::testkit::npc(7, ShipStats::default())
        };
        assert!(joined.mission_escort());
        assert_eq!(joined.fleet(), ShipRef::Player);
        assert_eq!(joined.formation(), Some(slot));
        assert_eq!(joined.escort, None);
        let unjoined = Npc {
            mission: Some(mark(ShipBehav::EscortPlayer, None)),
            ..joined.clone()
        };
        assert!(!unjoined.mission_escort());
        assert_eq!(unjoined.fleet(), ShipRef::Npc(NpcId(7)));
        assert_eq!(unjoined.formation(), None);
        let unmarked = Npc {
            mission: None,
            ..joined
        };
        assert!(!unmarked.mission_escort());
        assert_eq!(unmarked.formation(), None);
    }

    #[test]
    fn a_hired_escorts_formation_is_its_dutys() {
        let hired = Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot {
                    slot: 2,
                    ships: 4,
                    spacing: 35.0,
                },
                order: None,
            }),
            ..crate::testkit::npc(6, ShipStats::default())
        };
        assert_eq!(
            hired.formation(),
            Some(FormationSlot {
                slot: 2,
                ships: 4,
                spacing: 35.0
            })
        );
        assert!(!hired.mission_escort());
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
    fn only_a_ship_coming_out_of_a_gate_is_emerging() {
        let mut npc = crate::testkit::npc(1, ShipStats::default());
        npc.goal = Goal::Attack(ShipRef::Player);
        assert!(!npc.emerging());
        npc.mode = Mode::JumpingIn { ticks_left: 2 };
        assert!(!npc.emerging());
        npc.mode = Mode::Emerging { ticks_left: 2 };
        assert!(npc.emerging());
        assert!(!npc.threatens_player(), "while it emerges");
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }
}
