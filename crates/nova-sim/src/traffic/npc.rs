//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use std::collections::BTreeMap;

use crate::ai::Goal;
use crate::bay::Carrier;
use crate::catalog::{GovtId, PersonId, ShipId, StellarId, WeaponId};
use crate::cloak::{Cloak, CloakGear};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::escort::{EscortClass, EscortDuty, EscortOrder, FormationSlot};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::market::Good;
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
/// wormhole (see [`emerge`](crate::traffic::emerge)), or entering one to
/// leave (see [`gate_entry`](crate::traffic::gate_entry)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Gliding in from hyperspace, on this tick of the glide (from 0), at
    /// [`glide_speed`](crate::traffic::spawn::glide_speed): no steering
    /// and no decisions.
    JumpingIn {
        /// The tick of the glide.
        tick: u32,
    },
    /// Waiting at the centre of the gate it comes out of, for this many
    /// more ticks: hidden, still, seen by no ship, and no decisions. An
    /// arrival out of a gate (see [`emerge`](crate::traffic::emerge)) and
    /// a mission ship following the player through one
    /// (`_AIMakeShipEmergeFromHyperGate`, goal 21; see the session's
    /// `gate_follow` module) both wait so, untargetable and not drawn,
    /// with no target and no trigger held ([`Npc::emerge_from_gate`]):
    /// it fires nothing here or in its burst.
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
    /// Holding its course after its glide in or its burst out of a gate,
    /// for this many more ticks: a ship with no lead
    /// (`_HandleShip` @0x34d5c-0x34d8b, its AI wait +0x4c). It moves on at
    /// the velocity the glide or burst left it, with no steering, thrust,
    /// slowing, fire or decisions, but is seen, targeted and fought as a
    /// flying ship is: its goal is 0, not 21, so `_ShipVisibleToShip`
    /// (@0x972d) sees it and it is drawn. See [`Npc::arrived`].
    Coasting {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
    /// Entering `gate`, the hypergate or wormhole it leaves the system
    /// through, for this many more ticks: the original's sub-goal 0x17,
    /// its wait +0x4c counting down (see
    /// [`gate_entry`](crate::traffic::gate_entry)). It holds still and
    /// fades out, and decides nothing, but is seen, targeted, fought and
    /// hailed as a flying ship is ([`Npc::arrived`]).
    EnteringGate {
        /// The gate it enters.
        gate: StellarId,
        /// The ticks of the wait left.
        ticks_left: u32,
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
    /// The commodity in its own hold and its tons, if any: the original's
    /// six commodity words at ship +0x7a, all cleared as it spawns
    /// (`_GenericRandomShipSpawn` @0x3ccce-0x3cce5), of which a `flët`
    /// with `Flags` 0x0001 fills one for its trader ships
    /// ([`spawn::fleet`](crate::traffic::spawn::fleet)). The plunder never
    /// offers it: boarding reads the `düde`'s `Booty` alone
    /// (`_SetPlunderValues` @0x92222-0x92256).
    pub cargo: Option<(Good, u32)>,
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
    /// The gate it is coming out of, from its emerge until its burst is
    /// over: the original's travel stellar (+0x6c), which
    /// `_AIMakeShipEmergeFromHyperGate` sets to the gate (@0x8954a) and
    /// `_AIMakeShipStopJumping` sets to -1 (@0x82e9f). The gate's sprite
    /// opens while it waits and bursts ([`Npc::holds_gate_open`]).
    pub emerges_from: Option<StellarId>,
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

    /// Makes it come out of `gate`, waiting there `ticks_left` ticks
    /// (`_AIMakeShipEmergeFromHyperGate` @0x89518): goal 0x15 and
    /// sub-goal 0 ([`Mode::Emerging`], @0x89532-0x8953b), its travel
    /// stellar the gate ([`Npc::emerges_from`], @0x8954a), no target
    /// (+0x70 = -1, @0x89544), and its trigger released.
    ///
    /// The original holds no trigger from one frame to the next: each
    /// frame `_LowLevelAIHandler` clears the fire flag +0xba (@0x851f6)
    /// for an `_AIFire*` call to set again. A waiting ship's fire step is
    /// skipped (`_HandleShip` @0x34a54-0x34a72), and its burst runs under
    /// goal 8, sub-goal 0xa, which fires nothing (@0x8dd61-0x8dd93,
    /// @0x8f3c8-0x8f3d9). No decision is due until it flies, so the
    /// released trigger holds through both.
    ///
    /// The caller places it, gives it its heading and decides
    /// [`Npc::led_out`].
    pub fn emerge_from_gate(&mut self, gate: StellarId, ticks_left: u32) {
        self.mode = Mode::Emerging { ticks_left };
        self.emerges_from = Some(gate);
        self.target = None;
        self.trigger = Trigger::default();
    }

    /// Whether it holds `gate`'s sprite open (`_HandleStellarSprites`
    /// @0x2e884-0x2e8d7): coming out of it, waiting or bursting
    /// (`_AIShipIsEmergingFromHypergate` @0x7e2e8, goal 0x15, with its
    /// travel stellar +0x6c the gate), and neither disabled nor breaking
    /// up (`_IsDisabled` @0x2e8a3). Holding its course after the burst its
    /// goal is 0, so it no longer does.
    #[must_use]
    pub fn holds_gate_open(&self, gate: StellarId) -> bool {
        self.emerges_from == Some(gate)
            && matches!(self.mode, Mode::Emerging { .. } | Mode::LeavingGate { .. })
            && self.condition == Condition::Intact
    }

    /// Whether it is entering a gate to leave ([`Mode::EnteringGate`]):
    /// `_AIShipIsEnteringHypergate` (@0x89502), which tests the sub-goal
    /// 0x17 alone.
    #[must_use]
    pub fn entering_gate(&self) -> bool {
        matches!(self.mode, Mode::EnteringGate { .. })
    }

    /// Whether it heads in to `gate`, at `at`, strictly inside `reach` of
    /// it on both axes, holding its sprite open (`_HandleStellarSprites`
    /// @0x2e8dd-0x2e9a2): intact (`_IsDisabled` @0x2e8a3), bound to land
    /// on it or enter it (`_AIShipIsShipGoingToStellar` @0x895d6, goal 1
    /// or 0x14 with its travel stellar the gate), whatever its mode.
    #[must_use]
    pub fn approaches_gate(&self, gate: StellarId, at: Vec2, reach: f32) -> bool {
        let bound = matches!(self.goal, Goal::Land(to) | Goal::EnterGate(to) if to == gate);
        let off = self.state.position - at;
        bound && self.condition == Condition::Intact && off.x.abs() < reach && off.y.abs() < reach
    }

    /// Whether it is done coming into the system, there to be fought and
    /// hailed: [`Mode::Flying`], holding its course after its glide or
    /// burst ([`Mode::Coasting`]), which the original's +0x4c wait marks
    /// and its hyperspace state (+0x50, zeroed by `_AIMakeShipStopJumping`
    /// @0x82e77) and goal 21 do not, or entering a gate
    /// ([`Mode::EnteringGate`]): `_ShipVisibleToShip` tests goal 0x15
    /// alone, and state 20 zeroes +0x50 (@0x8db8f).
    #[must_use]
    pub fn arrived(&self) -> bool {
        matches!(
            self.mode,
            Mode::Flying | Mode::Coasting { .. } | Mode::EnteringGate { .. }
        )
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
                Condition::Dying { ticks_left: 1 },
                Condition::Destroyed,
            ] {
                let downed = Npc {
                    condition,
                    ..npc.clone()
                };
                assert!(!downed.threatens_player(), "{goal:?} {condition:?}");
            }
            for mode in [
                Mode::JumpingIn { tick: 0 },
                Mode::Coasting { ticks_left: 30 },
            ] {
                let arriving = Npc {
                    mode,
                    ..npc.clone()
                };
                assert!(!arriving.threatens_player(), "{goal:?} {mode:?}");
            }
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
            Condition::Dying { ticks_left: 1 },
            Condition::Destroyed,
        ] {
            let downed = Npc {
                condition,
                ..hunter.clone()
            };
            assert!(!downed.threatens_player_or_led(led), "{condition:?}");
        }
        for mode in [
            Mode::JumpingIn { tick: 0 },
            Mode::Emerging { ticks_left: 2 },
            Mode::Coasting { ticks_left: 30 },
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
                Condition::Dying { ticks_left: 1 },
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
                Condition::Dying { ticks_left: 1 },
                Condition::Destroyed,
            ] {
                let downed = Npc {
                    condition,
                    ..with(Goal::Attack(quarry))
                };
                assert!(!downed.threatens_npc_among(asker, &npcs), "{condition:?}");
            }
            for mode in [
                Mode::JumpingIn { tick: 0 },
                Mode::Emerging { ticks_left: 2 },
                Mode::Coasting { ticks_left: 30 },
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
        npc.mode = Mode::JumpingIn { tick: 0 };
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
    fn a_ship_holds_the_gate_it_comes_out_of_open_while_it_waits_and_bursts_intact() {
        let gate = StellarId(128);
        let ship = |mode, condition| Npc {
            mode,
            condition,
            emerges_from: Some(gate),
            ..crate::testkit::npc(1, ShipStats::default())
        };
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::LeavingGate { tick: 2 },
        ] {
            assert!(
                ship(mode, Condition::Intact).holds_gate_open(gate),
                "{mode:?}"
            );
            assert!(
                !ship(mode, Condition::Intact).holds_gate_open(StellarId(129)),
                "{mode:?}"
            );
            for condition in [
                Condition::Disabled,
                Condition::Dying { ticks_left: 3 },
                Condition::Destroyed,
            ] {
                assert!(
                    !ship(mode, condition).holds_gate_open(gate),
                    "{mode:?} {condition:?}"
                );
            }
        }
        for mode in [
            Mode::Coasting { ticks_left: 30 },
            Mode::Flying,
            Mode::JumpingIn { tick: 0 },
        ] {
            assert!(
                !ship(mode, Condition::Intact).holds_gate_open(gate),
                "{mode:?}"
            );
        }
        let unmarked = Npc {
            emerges_from: None,
            ..ship(Mode::Emerging { ticks_left: 3 }, Condition::Intact)
        };
        assert!(!unmarked.holds_gate_open(gate));
    }

    #[test]
    fn a_ship_made_to_emerge_from_a_gate_waits_there_with_no_target_and_its_trigger_released() {
        let state = ShipState {
            position: Vec2::new(10.0, -20.0),
            velocity: Vec2::new(1.0, 2.0),
            heading: 45.0,
        };
        let mut npc = Npc {
            mode: Mode::Flying,
            state,
            trigger: Trigger {
                primary: true,
                ..Trigger::default()
            },
            target: Some(ShipRef::Player),
            led_out: true,
            ..crate::testkit::npc(1, ShipStats::default())
        };
        npc.emerge_from_gate(StellarId(310), 40);
        assert_eq!(npc.mode, Mode::Emerging { ticks_left: 40 });
        assert_eq!(npc.emerges_from, Some(StellarId(310)));
        assert_eq!(npc.target, None);
        assert_eq!(npc.trigger, Trigger::default());
        assert_eq!(npc.state, state, "the caller places it");
        assert!(npc.led_out, "the caller decides it");
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }
}
