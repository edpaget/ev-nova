//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use crate::ai::Goal;
use crate::bay::Carrier;
use crate::catalog::{GovtId, PersonId, ShipId};
use crate::cloak::{Cloak, CloakGear};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, LauncherRounds, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::escort::{EscortClass, EscortDuty, EscortOrder, FormationSlot};
use crate::flight::ShipState;
use crate::mission::ships::ShipBehav;
use crate::reserves::Reserves;
use crate::self_repair::FormerEscort;
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

/// Whether an NPC is still coming out of hyperspace, or of a hypergate or
/// wormhole (see [`emerge`](crate::traffic::emerge)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Gliding in from hyperspace, for this many more ticks: no steering
    /// and no decisions.
    JumpingIn {
        /// The ticks of the glide left.
        ticks_left: u32,
    },
    /// Waiting at the centre of the gate it comes out of, for this many
    /// more ticks: hidden, still, seen by no ship, and no decisions. An
    /// arrival out of a gate (see [`emerge`](crate::traffic::emerge)) and
    /// a mission ship following the player through one
    /// (`_AIMakeShipEmergeFromHyperGate`, goal 21; see the session's
    /// `gate_follow` module) both wait so, untargetable and not drawn.
    Emerging {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
    /// Bursting out of the gate along its heading, on this tick of the
    /// burst (from 0), at [`exit_speed`](crate::traffic::emerge::exit_speed):
    /// no steering and no decisions.
    LeavingGate {
        /// The tick of the burst.
        tick: u32,
    },
    /// Flying under its own controls.
    Flying,
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
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    /// Its number.
    pub id: NpcId,
    /// Its ship-table slot (the original's ship index +0x86): the first
    /// slot free counting up from 1 as it is added
    /// (`_GenericRandomShipSpawn` @0x3c8b1-0x3c90b), or the slot it is
    /// placed at. The ships the player leads take formation slots in this
    /// order ([`Traffic::player_led`](crate::traffic::Traffic::player_led)).
    /// Unlike its [`id`](Npc::id), a berth is reused once its ship leaves.
    pub berth: u32,
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
    /// The rounds it holds, on each launcher (see
    /// [`armament`](crate::combat::armament)).
    pub rounds: LauncherRounds,
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
    /// Its `düde`'s `Booty` flags, what boarding it yields, and whose
    /// 0x0100 bit the hit rules read ([`sides`](crate::combat::sides));
    /// `None` when it has no `düde` (the original's +0x78 is -1): a
    /// fleet's ship, a person's, a hired escort, a fighter, or a captured
    /// ship, which loses its `düde` (`_DoShipCapture` @0x411aa). Boarding
    /// reads `None` as `Booty` 0.
    pub booty: Option<u16>,
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
    /// The slot of the mission whose auxiliary ship it is (the original's
    /// ship +0x8a): ordinary traffic, given back to its slot as the
    /// system's ships are dropped (see the [`mission::aux_ships`](crate::mission::aux_ships)
    /// module); none for any other ship.
    pub aux: Option<u8>,
    /// Its cloak: how far it has faded, and which way (see
    /// [`cloak`](crate::cloak)). A mission's special ship set up cloaked
    /// (`ShipStart` 2) starts fading in.
    pub cloak: Cloak,
    /// What its ship type gives its cloak.
    pub cloak_gear: CloakGear,
    /// Made to come out of a gate while the player led it, so its burst
    /// starts at [`LED_EXIT_SPEED`](crate::traffic::emerge::LED_EXIT_SPEED)
    /// (`_AIMakeShipEmergeFromHyperGate` @0x895aa-0x895bd). Read only as
    /// its wait runs out: decided at the emerge, as the original decides
    /// by the leader then.
    pub led_out: bool,
    /// Whether its ship class carries a repair system among its default
    /// items, so that, disabled, it can repair itself
    /// ([`ShipKind::self_repair`](crate::traffic::table::ShipKind)).
    pub self_repair: bool,
    /// What it was, while it is a former escort of the player's: disabled
    /// or breaking up as one, it left the fleet with this record (the
    /// original's +0xc8de). It forgets once intact, boarded or taken into
    /// the fleet again (see [`self_repair`](crate::self_repair)).
    pub former: Option<FormerEscort>,
}

/// What marks an NPC as an active mission's special ship (see the
/// [`traffic::spawn`](crate::traffic::spawn) module's "Mission ships").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissionMark {
    /// The slot of the mission.
    pub slot: u8,
    /// The mission's `ShipBehav`.
    pub behav: ShipBehav,
    /// Its place in the player's formation once it has joined it as a
    /// mission escort (the original's AI type 6 led by the player): none
    /// before (see the session's `mission_ships` module). It is never an
    /// [`EscortDuty`], so every hired-escort path leaves it out.
    pub formation: Option<FormationSlot>,
    /// Its standing order while a mission escort, given by the player's
    /// escort commands (`_IssueNewEscortCommand` @0x6614a-0x66176), the
    /// counterpart of [`EscortDuty::order`]. Never saved: mission escorts
    /// are set up again in each system.
    pub order: Option<EscortOrder>,
}

impl Npc {
    /// Whether it is the player's: an escort hired or captured, or a
    /// fighter the player launched. In the original its parent (+0x9a)
    /// is the player (0), and `_EnterMoreShips` (@0x43b14) leaves it out
    /// of the count it holds to `AvgShips`. An NPC carrier's fighters and
    /// orphaned fighters are not the player's, and neither is a mission
    /// escort ([`Npc::mission_escort`]), which carries no
    /// [`EscortDuty`].
    #[must_use]
    pub fn is_players(&self) -> bool {
        self.escort.is_some()
    }

    /// The ship it follows, the original's +0x9a: the player while it
    /// [escorts the player](Npc::escorts_player), otherwise its
    /// [`leader`](Npc::leader). The hit rules read it
    /// ([`sides`](crate::combat::sides)).
    #[must_use]
    pub fn leader_ref(&self) -> Option<ShipRef> {
        if self.escorts_player() {
            Some(ShipRef::Player)
        } else {
            self.leader.map(ShipRef::Npc)
        }
    }

    /// Its fleet: the player, for the player's escort, hired or a mission
    /// escort; otherwise the lead it escorts, or itself. Ships of one fleet
    /// count each other's missiles friendly.
    #[must_use]
    pub fn fleet(&self) -> ShipRef {
        if self.escorts_player() {
            ShipRef::Player
        } else {
            ShipRef::Npc(self.leader.unwrap_or(self.id))
        }
    }

    /// Whether it escorts the player: an escort hired or captured, a
    /// fighter the player launched (both [`Npc::is_players`]), or a mission
    /// escort. Not a fighter one of those launched, which an escort leads
    /// with no [`EscortDuty`] of its own (the IFF radar counts it through
    /// its leader: `iff::ship_iff`).
    #[must_use]
    pub fn escorts_player(&self) -> bool {
        self.escort.is_some() || self.mission_escort()
    }

    /// Its top speed in flight, its `_ShipMaxSpeed`: its own handling's
    /// × `led_speed`, the factor for the player and the ships it leads
    /// (see [`top_speed`](crate::top_speed)), while it
    /// [escorts the player](Npc::escorts_player), whose leader (+0x9a) is
    /// then the player; otherwise its own.
    #[must_use]
    pub fn top_speed(&self, led_speed: f32) -> f32 {
        let own = self.stats.handling.max_speed;
        if self.escorts_player() {
            own * led_speed
        } else {
            own
        }
    }

    /// Whether it is a mission escort: a mission's special ship that has
    /// joined the player's formation ([`MissionMark::formation`]).
    #[must_use]
    pub fn mission_escort(&self) -> bool {
        self.mission.is_some_and(|mark| mark.formation.is_some())
    }

    /// Its standing order as one of the player's escorts: a hired
    /// escort's ([`EscortDuty::order`]), else a mission escort's
    /// ([`MissionMark::order`]); none for any other ship.
    #[must_use]
    pub fn escort_order(&self) -> Option<EscortOrder> {
        match (self.escort, self.mission) {
            (Some(duty), _) => duty.order,
            (None, Some(mark)) if mark.formation.is_some() => mark.order,
            _ => None,
        }
    }

    /// Its place in the player's formation: a hired escort's
    /// ([`EscortDuty`]) or a mission escort's; none for any other ship.
    #[must_use]
    pub fn formation(&self) -> Option<FormationSlot> {
        self.escort
            .map(|duty| duty.formation)
            .or_else(|| self.mission.and_then(|mark| mark.formation))
    }

    /// Whether it fights `ship` directly: it is intact, flying, and
    /// attacking, sniping at or fleeing from it.
    ///
    /// This is only the direct-target part of the original's threat
    /// tests: `_ExtendedIsThreatToShip` @0x81a27 also counts a ship
    /// attacking one the asker leads ([`Npc::threatens_npc_among`]), and
    /// `_IsThreatToPlayer` one the player leads
    /// ([`Npc::threatens_player_among`]).
    #[must_use]
    pub fn threatens(&self, ship: ShipRef) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && self.goal.quarry() == Some(ship)
    }

    /// Whether it threatens the NPC `ship`, as `_ExtendedIsThreatToShip`
    /// @0x81a27 asks it of a ship other than the player, reading which
    /// ships `ship` leads from `npcs`. A ship whose quarry isn't in
    /// `npcs` is led by nobody.
    ///
    /// The original's test, `ship` its arg 0 and this ship its arg 1:
    /// this ship is active and not disabled (@0x81a33-0x81a4a); its
    /// leader (+0x9a) is not `ship` (@0x81a5a), so a ship's own escort
    /// never threatens it; its AI state is none of 5, 7, 9, 10, 11, 12,
    /// 15, 18; and its target (+0x70) is `ship` (@0x81aa9-0x81b03) or an
    /// active ship in slots 1-63, other than itself, whose leader is
    /// `ship` (@0x81ab2-0x81b59). Here [`Condition::Intact`] stands for
    /// the active and disabled tests and [`Goal::quarry`]'s attack, snipe
    /// and flee for the AI states. Unlike `_IsThreatToPlayer`, there is
    /// no AI-wait (+0x4c) test; [`Mode::Flying`] is kept as
    /// [`Npc::threatens`] keeps it, since its one modelled reader
    /// (`_SelectWarshipTarget` @0x8adb6, [`select_target`](crate::ai::target::select_target))
    /// sees only flying ships. The "other than itself" test can't
    /// change the answer: a ship led by `ship` is excluded by the leader
    /// test first.
    ///
    /// When the other ship is the player, `_ExtendedIsThreatToShip` asks
    /// `_IsThreatToPlayer` of `ship` instead
    /// ([`Npc::threatens_player_among`]).
    #[must_use]
    pub fn threatens_npc_among(&self, ship: NpcId, npcs: &[Npc]) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && self.leader != Some(ship)
            && match self.goal.quarry() {
                Some(ShipRef::Npc(id)) => {
                    id == ship || npcs.iter().any(|n| n.id == id && n.leader == Some(ship))
                }
                Some(ShipRef::Player) | None => false,
            }
    }

    /// Whether it can cloak now (`_ShipCanCloak` @0x553c): its
    /// [`CloakGear::can_cloak`], knowing whether it is disabled and how
    /// much fuel it has left.
    #[must_use]
    pub fn can_cloak(&self) -> bool {
        self.cloak_gear.can_cloak(
            self.condition == Condition::Disabled,
            self.reserves.fuel.now,
        )
    }

    /// Whether it threatens the player itself: intact and attacking,
    /// sniping at or fleeing from the player. There is no legal or
    /// government test; inspecting is no threat.
    ///
    /// This is only part of `_IsThreatToPlayer` @0x7f501, which also counts
    /// a ship attacking one the player leads; every caller roadmap
    /// player-led-ships ported asks [`Npc::threatens_player_among`]. It is
    /// kept for roadmap hud-fidelity's radar IFF
    /// ([`iff::ship_iff`](crate::iff::ship_iff)) and Red Alert, which build
    /// on it, and that roadmap's phase 14 moves them and retires it. The
    /// hail's [`threatened_by_enemy`](crate::hail::Hail::threatened_by_enemy)
    /// and [`threats_to_player`](crate::hail::Hail::threats_to_player)
    /// read it too.
    #[must_use]
    pub fn threatens_player(&self) -> bool {
        self.threatens(ShipRef::Player)
    }

    /// Whether it threatens the player or a ship the player leads, as
    /// `_IsThreatToPlayer` @0x7f501 asks it whole; `led` says whether an
    /// NPC is led by the player ([`Traffic::player_led`](super::Traffic::player_led)).
    ///
    /// The original's test: the ship is active, its AI wait (+0x4c) is
    /// not running (@0x7f518), it is not disabled, it has a target, its
    /// AI state is none of 5, 7, 9, 10, 11, 12, 15, 18 (@0x7f536-0x7f56b),
    /// and its target is the player (@0x7f56d) or a ship in slots 1-63
    /// whose leader (+0x9a) is the player (@0x7f572-0x7f590). Here
    /// [`Condition::Intact`] stands for the active and disabled tests,
    /// [`Mode::Flying`] for the wait (a flying ship keeps none; a ship
    /// coming out of a gate or jumping in has one), and
    /// [`Goal::quarry`]'s attack, snipe and flee for the AI states.
    ///
    /// `_FindNearestThreatToPlayer` (@0x9529-0x956b) also takes a ship
    /// that fails this test but targets an active ship the player leads
    /// and is fighting it (`_IsShipFightingMe` @0x827fa: its target, in
    /// AI state 4, attack). That adds only a state-4 ship whose AI wait
    /// is still running, which the engine never has on a flying ship, so
    /// this one test stands for both branches.
    #[must_use]
    pub fn threatens_player_or_led(&self, led: impl Fn(NpcId) -> bool) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && match self.goal.quarry() {
                Some(ShipRef::Player) => true,
                Some(ShipRef::Npc(id)) => led(id),
                None => false,
            }
    }

    /// Whether it threatens the player, as `_IsThreatToPlayer` @0x7f501
    /// asks it whole ([`Npc::threatens_player_or_led`]), reading which
    /// ships the player leads from `npcs`: those that
    /// [escort the player](Npc::escorts_player), as
    /// [`Traffic::player_led`](super::Traffic::player_led) does. A ship
    /// whose quarry isn't in `npcs` is led by nobody.
    ///
    /// Every modelled caller of `_IsThreatToPlayer` asks this, and so do
    /// the readers of `_ExtendedIsThreatToPlayer` @0x81b70, whose target
    /// test has the same led clause (@0x81c2e-0x81c4a).
    #[must_use]
    pub fn threatens_player_among(&self, npcs: &[Npc]) -> bool {
        self.threatens_player_or_led(|id| npcs.iter().any(|n| n.id == id && n.escorts_player()))
    }

    /// Whether it is waiting at the gate it comes out of
    /// ([`Mode::Emerging`]): `_AIShipIsEmergingFromHypergate` (@0x7e2e8),
    /// which holds it invisible to every ship (`_ShipVisibleToShip`
    /// @0x972d). Bursting out of the gate, it is no longer.
    #[must_use]
    pub fn emerging(&self) -> bool {
        matches!(self.mode, Mode::Emerging { .. })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
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

    #[test]
    fn a_ship_the_player_leads_has_its_top_speed_times_the_led_speed_and_any_other_its_own() {
        let fields = crate::handling::ShipFields {
            speed: 300,
            ..crate::testkit::FAST
        };
        let ship = |id| crate::testkit::npc(id, ShipStats::new(fields, &[]));
        assert_eq!(ship(1).stats.handling.max_speed, 3.0);
        let hired = Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..ship(1)
        };
        let mission = Npc {
            mission: Some(mark(
                ShipBehav::EscortPlayer,
                Some(FormationSlot::default()),
            )),
            ..ship(2)
        };
        let not_joined = Npc {
            mission: Some(mark(ShipBehav::EscortPlayer, None)),
            ..ship(3)
        };
        let fleet_escort = Npc {
            leader: Some(NpcId(9)),
            ..ship(4)
        };
        assert_eq!(hired.top_speed(1.5), 4.5, "a hired escort");
        assert_eq!(mission.top_speed(1.5), 4.5, "a mission escort");
        assert_eq!(not_joined.top_speed(1.5), 3.0, "not yet joined");
        assert_eq!(fleet_escort.top_speed(1.5), 3.0, "a fleet escort");
        assert_eq!(ship(5).top_speed(1.5), 3.0, "a lone NPC");
        assert_eq!(hired.top_speed(1.0), 3.0, "Strict Play");
    }

    /// A mark of slot 1 of `behav`, holding `formation`.
    fn mark(behav: ShipBehav, formation: Option<FormationSlot>) -> MissionMark {
        MissionMark {
            slot: 1,
            behav,
            formation,
            order: None,
        }
    }

    #[test]
    fn a_ships_escort_order_is_its_dutys_else_its_marks_while_a_mission_escort() {
        let slot = FormationSlot::default();
        let ordered = |formation| MissionMark {
            order: Some(EscortOrder::Hold),
            ..mark(ShipBehav::EscortPlayer, formation)
        };
        let joined = Npc {
            mission: Some(ordered(Some(slot))),
            ..crate::testkit::npc(7, ShipStats::default())
        };
        assert_eq!(joined.escort_order(), Some(EscortOrder::Hold));
        let unjoined = Npc {
            mission: Some(ordered(None)),
            ..joined.clone()
        };
        assert_eq!(unjoined.escort_order(), None);
        let hired = Npc {
            escort: Some(EscortDuty {
                formation: slot,
                order: Some(EscortOrder::Attack),
            }),
            ..joined.clone()
        };
        assert_eq!(hired.escort_order(), Some(EscortOrder::Attack));
        let plain = Npc {
            mission: None,
            ..joined
        };
        assert_eq!(plain.escort_order(), None);
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
                Condition::Dying {
                    ticks_left: 1,
                    starting: false,
                },
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
    fn an_npc_attacking_a_ship_the_player_leads_threatens_the_player_whole() {
        let led = |id: NpcId| id == NpcId(9);
        let escort = ShipRef::Npc(NpcId(9));
        let stranger = ShipRef::Npc(NpcId(8));
        let with = |goal| Npc {
            goal,
            ..crate::testkit::npc(1, ShipStats::default())
        };
        for quarry in [escort, ShipRef::Player] {
            for goal in [
                Goal::Attack(quarry),
                Goal::Snipe(quarry),
                Goal::Flee(quarry),
            ] {
                assert!(with(goal).threatens_player_or_led(led), "{goal:?}");
            }
        }
        for goal in [
            Goal::Attack(stranger),
            Goal::Snipe(stranger),
            Goal::Flee(stranger),
            Goal::Inspect(escort),
            Goal::Idle,
        ] {
            assert!(!with(goal).threatens_player_or_led(led), "{goal:?}");
        }
        let hunter = with(Goal::Attack(escort));
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 1,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            let downed = Npc {
                condition,
                ..hunter.clone()
            };
            assert!(!downed.threatens_player_or_led(led), "{condition:?}");
        }
        for mode in [
            Mode::JumpingIn { ticks_left: 2 },
            Mode::Emerging { ticks_left: 2 },
        ] {
            let arriving = Npc {
                mode,
                ..hunter.clone()
            };
            assert!(!arriving.threatens_player_or_led(led), "{mode:?}");
        }
    }

    #[test]
    fn threatens_player_among_counts_the_player_and_any_ship_the_player_leads() {
        let ship = |id| crate::testkit::npc(id, ShipStats::default());
        let hired = Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..ship(1)
        };
        let mission = Npc {
            mission: Some(mark(
                ShipBehav::EscortPlayer,
                Some(FormationSlot::default()),
            )),
            ..ship(2)
        };
        let unjoined = Npc {
            mission: Some(mark(ShipBehav::EscortPlayer, None)),
            ..ship(3)
        };
        let stranger = ship(4);
        let npcs = [hired, mission, unjoined, stranger];
        let attacking = |quarry| Npc {
            goal: Goal::Attack(quarry),
            ..ship(9)
        };
        for quarry in [
            ShipRef::Player,
            ShipRef::Npc(NpcId(1)),
            ShipRef::Npc(NpcId(2)),
        ] {
            assert!(
                attacking(quarry).threatens_player_among(&npcs),
                "{quarry:?}"
            );
            for condition in [
                Condition::Disabled,
                Condition::Dying {
                    ticks_left: 1,
                    starting: false,
                },
                Condition::Destroyed,
            ] {
                let downed = Npc {
                    condition,
                    ..attacking(quarry)
                };
                assert!(!downed.threatens_player_among(&npcs), "{condition:?}");
            }
            let emerging = Npc {
                mode: Mode::Emerging { ticks_left: 2 },
                ..attacking(quarry)
            };
            assert!(!emerging.threatens_player_among(&npcs), "{quarry:?}");
        }
        for quarry in [3, 4, 5].map(|id| ShipRef::Npc(NpcId(id))) {
            assert!(
                !attacking(quarry).threatens_player_among(&npcs),
                "{quarry:?}"
            );
        }
    }

    #[test]
    fn threatens_npc_among_counts_a_ship_attacking_the_asker_or_one_it_leads() {
        let asker = NpcId(1);
        let ship = |id| crate::testkit::npc(id, ShipStats::default());
        let led = Npc {
            leader: Some(asker),
            ..ship(2)
        };
        let other_led = Npc {
            leader: Some(NpcId(5)),
            ..ship(3)
        };
        let stranger = ship(4);
        let npcs = [ship(1), led, other_led, stranger];
        let with = |goal| Npc { goal, ..ship(9) };
        for quarry in [asker, NpcId(2)].map(ShipRef::Npc) {
            for goal in [
                Goal::Attack(quarry),
                Goal::Snipe(quarry),
                Goal::Flee(quarry),
            ] {
                assert!(with(goal).threatens_npc_among(asker, &npcs), "{goal:?}");
            }
            assert!(
                !with(Goal::Inspect(quarry)).threatens_npc_among(asker, &npcs),
                "inspecting {quarry:?}"
            );
            for condition in [
                Condition::Disabled,
                Condition::Dying {
                    ticks_left: 1,
                    starting: false,
                },
                Condition::Destroyed,
            ] {
                let downed = Npc {
                    condition,
                    ..with(Goal::Attack(quarry))
                };
                assert!(!downed.threatens_npc_among(asker, &npcs), "{condition:?}");
            }
            for mode in [
                Mode::JumpingIn { ticks_left: 2 },
                Mode::Emerging { ticks_left: 2 },
            ] {
                let arriving = Npc {
                    mode,
                    ..with(Goal::Attack(quarry))
                };
                assert!(!arriving.threatens_npc_among(asker, &npcs), "{mode:?}");
            }
            let own_escort = Npc {
                leader: Some(asker),
                ..with(Goal::Attack(quarry))
            };
            assert!(
                !own_escort.threatens_npc_among(asker, &npcs),
                "its own escort, at {quarry:?}"
            );
            let led_elsewhere = Npc {
                leader: Some(NpcId(5)),
                ..with(Goal::Attack(quarry))
            };
            assert!(
                led_elsewhere.threatens_npc_among(asker, &npcs),
                "another's escort, at {quarry:?}"
            );
        }
        for quarry in [
            ShipRef::Player,
            ShipRef::Npc(NpcId(3)),
            ShipRef::Npc(NpcId(4)),
            ShipRef::Npc(NpcId(6)),
        ] {
            assert!(
                !with(Goal::Attack(quarry)).threatens_npc_among(asker, &npcs),
                "{quarry:?}"
            );
        }
        assert!(!with(Goal::Idle).threatens_npc_among(asker, &npcs));
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
    fn an_npc_can_cloak_with_a_cloak_and_fuel_while_not_disabled() {
        let mut npc = crate::testkit::npc(1, ShipStats::default());
        npc.cloak_gear.device = true;
        npc.reserves.fuel.now = 1.0;
        assert!(npc.can_cloak());
        npc.condition = Condition::Disabled;
        assert!(!npc.can_cloak(), "disabled");
        npc.condition = Condition::Intact;
        npc.reserves.fuel.now = 0.0;
        assert!(!npc.can_cloak(), "no fuel");
        npc.reserves.fuel.now = 1.0;
        npc.cloak_gear.device = false;
        assert!(!npc.can_cloak(), "no cloak");
    }

    #[test]
    fn only_the_players_escorts_and_fighters_are_the_players() {
        let traffic = crate::testkit::npc(3, ShipStats::default());
        assert!(!traffic.is_players());
        let carrier = crate::bay::Carrier {
            ship: ShipRef::Npc(NpcId(3)),
            window: 100.0,
            reach: 30.0,
        };
        let npc_fighter = Npc {
            leader: Some(NpcId(3)),
            carrier: Some(carrier),
            ..crate::testkit::npc(4, ShipStats::default())
        };
        assert!(!npc_fighter.is_players());
        let players = Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot {
                    slot: 2,
                    ships: 2,
                    spacing: 30.0,
                },
                order: None,
            }),
            carrier: Some(crate::bay::Carrier {
                ship: ShipRef::Player,
                ..carrier
            }),
            ..crate::testkit::npc(5, ShipStats::default())
        };
        assert!(players.is_players());
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }
}
