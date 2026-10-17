//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use crate::ai::Goal;
use crate::bay::Carrier;
use crate::catalog::{GovtId, PersonId, PersonRecord, ShipId, StellarId, WeaponId};
use crate::cloak::{Cloak, CloakGear};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, LauncherRounds, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::impact::TractorLock;
use crate::combat::ion::Ion;
use crate::escort::{EscortClass, EscortDuty, EscortOrder, FormationSlot};
use crate::flight::ShipState;
use crate::geometry::Vec2;
use crate::market::Good;
use crate::reserves::Reserves;
use crate::self_repair::FormerEscort;
use crate::stats::ShipStats;
use crate::traffic::ShipBehav;

pub use crate::combat::NpcId;

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
    /// it fires nothing here or in its burst, but for the point defence of
    /// an escort or fighter given a target (see the traffic's tick).
    ///
    /// It answers a hit as a flying ship does: its hyperspace state +0x50,
    /// which `_DamageShip` reads to skip a reaction (@0x3b4b0-0x3b4c2),
    /// is -1.0 here (@0x89555), never above 0. It takes the attacker as
    /// its target and the damage as provocation, and its wait is cut to
    /// 20 (@0x3be6e-0x3be7c); it stays here, hidden, and keeps both
    /// through its burst. A hit by the player holds it here instead
    /// ([`Mode::HeldAtGate`]).
    Emerging {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
    /// Held where it waited in its gate, for this many more ticks, once
    /// the player's hit (or a Good Samaritan's answer to the player) made
    /// it attack the player there: `_AIMakeShipAttackPlayer` (@0x89d3e)
    /// sets goal 4 whatever the goal, so it is no longer goal 0x15 and
    /// `_ShipVisibleToShip` (@0x972d) sees it, it is targeted and drawn
    /// unfaded, it lets go of the gate's sprite (@0x2e8bd-0x2e8d7), and
    /// `_HighLevelAIHandler` never bursts it out (@0x8dd61). Its wait
    /// +0x4c still runs: `_HandleShip` skips its steering, thrust and fire
    /// (@0x34a54-0x34a72), so it holds still, and once the wait is out it
    /// flies its goal from rest ([`Npc::hold_at_gate`]), with no burst:
    /// its attack rewrites the burst word +0x34 the step it first thrusts
    /// (see [`autopilot`](crate::traffic::autopilot)). Its decisions are
    /// due through the hold, as goal 0x15 no longer skips the per-type AIs
    /// (@0x8ff18-0x8ff20 → @0x8ff26-0x90016), but it fires only the point
    /// defence of a waiting escort or fighter (see the traffic's tick).
    HeldAtGate {
        /// The ticks of the wait left.
        ticks_left: u32,
    },
    /// Bursting out of the gate along its heading, on this tick of the
    /// burst (from 0), at [`exit_speed`](crate::traffic::emerge::exit_speed)
    /// from 30, or from 15 when [`led_out`](Npc::led_out): no steering and
    /// no decisions. Tick 0 is the frame its wait runs out, its goal still
    /// 0x15, so it still holds the gate's sprite open; from its first
    /// moving tick it has goal 8 and is out of its gate (+0x6c = -1,
    /// @0x8dd8a-0x8dd93), and lets go ([`Npc::holds_gate_open`]).
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
/// `nova_sim::person`).
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

impl NpcPerson {
    /// What an NPC flown by `record`'s person carries of it, holding a
    /// grudge when `grudge`, having said no hail quote yet.
    #[must_use]
    pub fn of(record: &PersonRecord, grudge: bool) -> Self {
        Self {
            id: record.id,
            flags: record.flags,
            coward: record.coward,
            comm_quote: record.comm_quote,
            hail_quote: record.hail_quote,
            mission: record.link_mission.is_some(),
            portrait: record.hail_pict,
            invincible: record.shield_mod < 0,
            grudge,
            quoted: false,
            quoted_at: None,
        }
    }
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
    /// The weapon it holds as its pick (the original's +0x72): set by its
    /// AI's passes ([`Trigger::keep`]), kept from tick to tick and cleared
    /// once a pick without `Flags` 0x0002 fires
    /// ([`after_firing`](crate::ai::fire::after_firing)); none at first.
    pub pick: Option<WeaponId>,
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
    /// The commodities in its own hold and their tons: the original's
    /// six commodity words at ship +0x7a, all cleared as it spawns
    /// (`_GenericRandomShipSpawn` @0x3ccce-0x3cce5), of which a `flët`
    /// with `Flags` 0x0001 fills one for its trader ships
    /// ([`spawn::fleet`](crate::traffic::spawn::fleet)). The plunder never
    /// offers it: boarding reads the `düde`'s `Booty` alone
    /// (`_SetPlunderValues` @0x92222-0x92256). It holds all six, so a ton
    /// scooped of another commodity is kept beside the first
    /// ([`NpcHold`]).
    pub cargo: NpcHold,
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
    /// system's ships are dropped (see the `nova_sim::mission::aux_ships`
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
    /// (`_AIMakeShipEmergeFromHyperGate` @0x895aa-0x895bd). Read through
    /// its burst: decided at the emerge, as the original decides by the
    /// leader then and fixes the burst's start speed (+0x34) there.
    pub led_out: bool,
    /// The gate it is coming out of, from its emerge until its burst
    /// starts: the original's travel stellar (+0x6c), which
    /// `_AIMakeShipEmergeFromHyperGate` sets to the gate (@0x8954a) and
    /// `_HighLevelAIHandler` sets to -1 in the pass that starts the burst
    /// (@0x8dd93), or `_AIMakeShipAttackPlayer` as the ship is held at its
    /// gate (@0x89d47). The gate's sprite opens while it waits
    /// ([`Npc::holds_gate_open`]).
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
    /// Its tractor lock, if a tractor beam holds it or it holds itself
    /// along its own (the original's +0xc8dc and +0xb4, see
    /// [`impact`](crate::combat::impact)): none as it spawns or launches,
    /// as the original's spawns and `_LaunchFighter` set it (@0x3c5c8,
    /// @0x3cb1c, @0x3d71d). The fight sets and touches it.
    pub tractor: Option<TractorLock>,
    /// Its ionization (the original's +0x5c and +0xb0, see
    /// [`ion`](crate::combat::ion)): none as it spawns, launches or is
    /// left as the player's hulk, as `_RandomShipSpawn`,
    /// `_GenericRandomShipSpawn`, `_LaunchFighter` and the eject set it
    /// (@0x3c457, @0x3c9f1, @0x3d8aa, @0x6f372). The fight raises it;
    /// ship-handling-fidelity phase 11 lowers it. Never saved.
    pub ion: Ion,
}

/// The commodities 0-5 the original's ship records count.
pub const NPC_COMMODITIES: usize = 6;

/// An NPC's own hold: the tons of each commodity 0-5 it carries, the
/// original's six commodity words at ship +0x7a. Its spawn fills one at
/// most ([`NpcHold::of`]); a scoop adds a ton of any
/// ([`scoop::npc_scoop`](crate::scoop::npc_scoop)). It holds no `jünk`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NpcHold {
    tons: [u32; NPC_COMMODITIES],
}

impl NpcHold {
    /// The hold a spawn gives: the one commodity its ship comes with, if
    /// any. A `jünk`, or a commodity past 5, is dropped; the spawn never
    /// gives either.
    #[must_use]
    pub fn of(cargo: Option<(Good, u32)>) -> Self {
        let mut hold = Self::default();
        if let Some((Good::Commodity(commodity), tons)) = cargo {
            hold.add(commodity, tons);
        }
        hold
    }

    /// The tons of `commodity` it holds: none past commodity 5.
    #[must_use]
    pub fn tons(&self, commodity: u8) -> u32 {
        self.tons.get(usize::from(commodity)).copied().unwrap_or(0)
    }

    /// Adds `tons` of `commodity`, saturating: nothing past commodity 5.
    pub fn add(&mut self, commodity: u8, tons: u32) {
        if let Some(held) = self.tons.get_mut(usize::from(commodity)) {
            *held = held.saturating_add(tons);
        }
    }

    /// The commodities it holds tons of, in commodity order, with the
    /// tons.
    pub fn held(&self) -> impl Iterator<Item = (Good, u32)> + '_ {
        (0..NPC_COMMODITIES)
            .zip(self.tons)
            .filter(|&(_, tons)| tons > 0)
            .map(|(commodity, tons)| {
                // In 0..6.
                #[allow(clippy::cast_possible_truncation)]
                let commodity = commodity as u8;
                (Good::Commodity(commodity), tons)
            })
    }
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

    /// Whether it is scooping up asteroid boxes, `_AIShipIsScooping`
    /// (@0x88727): by its goal ([`Goal::scooping`]), so never until
    /// rdm:phase/asteroids/phase-5-ai-mining.
    #[must_use]
    pub fn is_scooping(&self) -> bool {
        self.goal.scooping()
    }

    /// Its top speed in flight, its `_ShipMaxSpeed`: its own handling's
    /// × `led_speed`, the factor for the player and the ships it leads
    /// (see `nova_sim::top_speed`), while it
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
    /// a ship attacking one the player leads; every other caller, the
    /// radar IFF and the Red Alert among them, asks
    /// [`Npc::threatens_player_among`]. It is kept only for the hail's
    /// [`threatened_by_enemy`](crate::hail::Hail::threatened_by_enemy)
    /// and [`threats_to_player`](crate::hail::Hail::threats_to_player),
    /// which still read it.
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
    /// test has the same led clause (@0x81c2e-0x81c4a), but for the attack
    /// quote's gate, which asks the whole of it
    /// ([`Npc::threatens_player_extended`]).
    #[must_use]
    pub fn threatens_player_among(&self, npcs: &[Npc]) -> bool {
        self.threatens_player_or_led(|id| npcs.iter().any(|n| n.id == id && n.escorts_player()))
    }

    /// Whether it threatens the player as `_ExtendedIsThreatToPlayer`
    /// (@0x81b70-0x81de5) asks it, reading which ships the player and it
    /// lead from `npcs`, and `seen` saying whether the player sees it
    /// (`_ShipVisibleToShip(ship, player)` @0x81bc6, which only the
    /// session or its [`World`](crate::traffic::World) knows: a cloaked
    /// ship is seen only by a player scanning for it in range, and a ship
    /// waiting in its gate never, see [`cloak::visible`](crate::cloak::visible)).
    ///
    /// It is no threat when it is not intact (inactive, @0x81b82, or
    /// `_IsDisabled`, @0x81b92), when the player leads it (+0x9a == 0,
    /// @0x81b9f), when it has no target (+0x70 == -1, @0x81bad), or when it
    /// is unseen. Otherwise it is a threat for any of:
    ///
    /// - **A stellar defender** (+0x8c != -1, @0x81bd3): not modelled, as
    ///   the engine has no defence ships; rdm:task/planet-domination owns
    ///   them.
    /// - **The target clause**: it [threatens the player or a ship the
    ///   player leads](Npc::threatens_player_or_led) by its goal's quarry,
    ///   or its target ([`Npc::target`], the hit's, which
    ///   `_DamageShip` sets before `_AIMakeShipAttackPlayer`, @0x3bcf6) is
    ///   the player (@0x81be1) or a ship the player leads (@0x81c2e-0x81c4a)
    ///   while its wait is not running ([`Npc::wait_running`], +0x4c,
    ///   @0x81bed, @0x81c4f) and its goal is none the original leaves out
    ///   ([`Goal::spares_player_threat`], @0x81bf3-0x81c28,
    ///   @0x81c55-0x81c8a). @0x81c90-0x81cd7 repeats the led-target test,
    ///   which changes nothing.
    /// - **The escort clause** (@0x81cdd-0x81dcc): some ship in `npcs` it
    ///   leads (+0x9a is its +0x86), whose wait is not running, intact,
    ///   targets (or fights) the player (@0x81d33) or a ship the player
    ///   leads (@0x81dae-0x81dbe). The goal read there is this ship's
    ///   (`%esi` @0x81d38, @0x81d77), not the escort's, and this ship's own
    ///   wait is not read.
    ///
    /// A ship gliding in ([`Mode::JumpingIn`]) or bursting out of its gate
    /// ([`Mode::LeavingGate`]) runs no wait (`_HyperShipSpawn` sets only
    /// +0x50, @0x42b16), so it is a threat once it targets the player. At
    /// the burst's tick 0 the original's ship still has goal 0x15 and is
    /// unseen; the engine's [`Npc::emerging`] treats that tick as out of
    /// the gate, as every reader of the player's sight does.
    ///
    /// This is the predicate for `_AIMakeShipAttackPlayer`'s quote gate
    /// ([`Traffic::turn_with_quote`](crate::traffic::Traffic::turn_with_quote));
    /// the hail's and the ranks' readers still ask
    /// [`Npc::threatens_player_among`].
    #[must_use]
    pub fn threatens_player_extended(&self, npcs: &[Npc], seen: bool) -> bool {
        let led = |id: NpcId| npcs.iter().any(|n| n.id == id && n.escorts_player());
        let at_player = |ship: Option<ShipRef>| match ship {
            Some(ShipRef::Player) => true,
            Some(ShipRef::Npc(id)) => led(id),
            None => false,
        };
        let has_target = self.target.is_some() || self.goal.quarry().is_some();
        if !seen || self.condition != Condition::Intact || self.escorts_player() || !has_target {
            return false;
        }
        let in_threat_goal = !self.goal.spares_player_threat();
        let by_target = self.threatens_player_or_led(led)
            || (!self.wait_running() && in_threat_goal && at_player(self.target));
        let by_escort = in_threat_goal
            && npcs.iter().any(|escort| {
                escort.leader == Some(self.id)
                    && escort.condition == Condition::Intact
                    && !escort.wait_running()
                    && (at_player(escort.target) || at_player(escort.goal.quarry()))
            });
        by_target || by_escort
    }

    /// Whether its AI wait (+0x4c) is running: waiting in its gate or
    /// held at it, holding its course after its glide or burst, or
    /// entering a gate to leave (see each [`Mode`]). Gliding in, bursting
    /// out and flying, it has none.
    #[must_use]
    pub fn wait_running(&self) -> bool {
        matches!(
            self.mode,
            Mode::Emerging { .. }
                | Mode::HeldAtGate { .. }
                | Mode::Coasting { .. }
                | Mode::EnteringGate { .. }
        )
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
    /// @0x2e884-0x2e8d7): coming out of it, waiting or its wait just run
    /// out ([`Mode::LeavingGate`] at tick 0)
    /// (`_AIShipIsEmergingFromHypergate` @0x7e2e8, goal 0x15, with its
    /// travel stellar +0x6c the gate), and neither disabled nor breaking
    /// up (`_IsDisabled` @0x2e8a3). From its first burst tick it has goal 8
    /// and +0x6c = -1 (@0x8dd8a-0x8dd93), so it lets go.
    #[must_use]
    pub fn holds_gate_open(&self, gate: StellarId) -> bool {
        self.emerges_from == Some(gate)
            && matches!(
                self.mode,
                Mode::Emerging { .. } | Mode::LeavingGate { tick: 0 }
            )
            && self.condition == Condition::Intact
    }

    /// Holds it where it waits in its gate, attacking the player
    /// ([`Mode::HeldAtGate`], its wait kept): `_AIMakeShipAttackPlayer`
    /// sets goal 4, target the player and travel stellar +0x6c = -1
    /// (@0x89d3e-0x89d4d), keeping sub-goal 0. A ship not waiting in a
    /// gate is left as it is.
    pub fn hold_at_gate(&mut self) {
        if let Mode::Emerging { ticks_left } = self.mode {
            self.mode = Mode::HeldAtGate { ticks_left };
            self.emerges_from = None;
            self.goal = Goal::Attack(ShipRef::Player);
        }
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

    /// Whether it is done coming into the system, there to be seen,
    /// fought and hailed: [`Mode::Flying`], holding its course after its
    /// glide or burst ([`Mode::Coasting`]), which the original's +0x4c
    /// wait marks and goal 21 does not, held at its gate attacking the
    /// player ([`Mode::HeldAtGate`], goal 4), or entering a gate
    /// ([`Mode::EnteringGate`]): `_ShipVisibleToShip` tests goal 0x15
    /// alone (@0x972d). A ship gliding in, waiting in its gate or
    /// bursting out of it is not, though it answers hits all the same
    /// (see [`Mode::Emerging`]).
    #[must_use]
    pub fn arrived(&self) -> bool {
        matches!(
            self.mode,
            Mode::Flying
                | Mode::Coasting { .. }
                | Mode::HeldAtGate { .. }
                | Mode::EnteringGate { .. }
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
        let lead = crate::traffic::fixture::npc(3, ShipStats::default());
        assert_eq!(lead.fleet(), ShipRef::Npc(NpcId(3)));
        let escort = Npc {
            leader: Some(NpcId(3)),
            ..crate::traffic::fixture::npc(5, ShipStats::default())
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
            ..crate::traffic::fixture::npc(6, ShipStats::default())
        };
        assert_eq!(players.fleet(), ShipRef::Player);
    }

    #[test]
    fn a_ship_the_player_leads_has_its_top_speed_times_the_led_speed_and_any_other_its_own() {
        let fields = crate::handling::ShipFields {
            speed: 300,
            ..nova_sim_base::fixture::FAST
        };
        let ship = |id| crate::traffic::fixture::npc(id, ShipStats::new(fields, &[]));
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
            ..crate::traffic::fixture::npc(7, ShipStats::default())
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
            ..crate::traffic::fixture::npc(7, ShipStats::default())
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
            ..crate::traffic::fixture::npc(6, ShipStats::default())
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
                ..crate::traffic::fixture::npc(1, ShipStats::default())
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
                ..crate::traffic::fixture::npc(1, ShipStats::default())
            };
            assert!(!npc.threatens_player(), "{goal:?}");
        }
        let hunter = Npc {
            goal: Goal::Attack(other),
            ..crate::traffic::fixture::npc(1, ShipStats::default())
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
            ..crate::traffic::fixture::npc(1, ShipStats::default())
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
        let ship = |id| crate::traffic::fixture::npc(id, ShipStats::default());
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
        let ship = |id| crate::traffic::fixture::npc(id, ShipStats::default());
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
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::default());
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
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::default());
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
        let traffic = crate::traffic::fixture::npc(3, ShipStats::default());
        assert!(!traffic.is_players());
        let carrier = crate::bay::Carrier {
            ship: ShipRef::Npc(NpcId(3)),
            window: 100.0,
            reach: 30.0,
        };
        let npc_fighter = Npc {
            leader: Some(NpcId(3)),
            carrier: Some(carrier),
            ..crate::traffic::fixture::npc(4, ShipStats::default())
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
            ..crate::traffic::fixture::npc(5, ShipStats::default())
        };
        assert!(players.is_players());
    }

    #[test]
    fn a_ship_holds_the_gate_it_comes_out_of_open_while_it_waits_intact_and_lets_go_as_its_burst_starts()
     {
        let gate = StellarId(128);
        let ship = |mode, condition| Npc {
            mode,
            condition,
            emerges_from: Some(gate),
            ..crate::traffic::fixture::npc(1, ShipStats::default())
        };
        // Waiting, or its wait just run out (goal still 0x15).
        for mode in [
            Mode::Emerging { ticks_left: 3 },
            Mode::LeavingGate { tick: 0 },
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
                Condition::Dying {
                    ticks_left: 3,
                    starting: false,
                },
                Condition::Destroyed,
            ] {
                assert!(
                    !ship(mode, condition).holds_gate_open(gate),
                    "{mode:?} {condition:?}"
                );
            }
        }
        // Bursting it has goal 8 and +0x6c = -1 (@0x8dd8a-0x8dd93), so
        // `_AIShipIsEmergingFromHypergate` (@0x7e2e8) no longer sees it,
        // even with the gate still marked.
        for mode in [
            Mode::LeavingGate { tick: 1 },
            Mode::LeavingGate { tick: 2 },
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
            ..crate::traffic::fixture::npc(1, ShipStats::default())
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
    fn a_ship_held_at_its_gate_is_arrived_and_seen_but_no_longer_emerging_or_holding_it_open() {
        // `_AIMakeShipAttackPlayer` takes it out of goal 0x15 (@0x89d3e),
        // which `_ShipVisibleToShip` (@0x972d) and the gate sprite
        // (@0x2e8bd-0x2e8d7) test, but its wait +0x4c still runs, which
        // `_IsThreatToPlayer` tests (@0x7f518).
        let gate = StellarId(128);
        let held = Npc {
            mode: Mode::HeldAtGate { ticks_left: 12 },
            goal: Goal::Attack(ShipRef::Player),
            emerges_from: Some(gate),
            ..crate::traffic::fixture::npc(1, ShipStats::default())
        };
        assert!(held.arrived());
        assert!(!held.emerging());
        assert!(!held.holds_gate_open(gate));
        assert!(!held.threatens_player_or_led(|_| true));
        assert!(!held.threatens_player());
    }

    #[test]
    fn a_ship_held_at_its_gate_keeps_its_wait_leaves_its_gate_and_attacks_the_player() {
        let mut npc = Npc {
            mode: Mode::Emerging { ticks_left: 17 },
            emerges_from: Some(StellarId(310)),
            target: Some(ShipRef::Player),
            provoked: 4.0,
            ..crate::traffic::fixture::npc(1, ShipStats::default())
        };
        npc.hold_at_gate();
        assert_eq!(npc.mode, Mode::HeldAtGate { ticks_left: 17 });
        assert_eq!(npc.emerges_from, None, "+0x6c = -1 (@0x89d47)");
        assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(npc.target, Some(ShipRef::Player));
        assert_eq!(npc.provoked, 4.0);
        let mut flying = Npc {
            mode: Mode::LeavingGate { tick: 3 },
            emerges_from: Some(StellarId(310)),
            ..crate::traffic::fixture::npc(2, ShipStats::default())
        };
        let before = flying.clone();
        flying.hold_at_gate();
        assert_eq!(flying, before, "only a ship waiting in its gate is held");
    }

    #[test]
    fn only_the_traders_trade() {
        assert!(AiType::WimpyTrader.trades());
        assert!(AiType::BraveTrader.trades());
        assert!(!AiType::Warship.trades());
        assert!(!AiType::Interceptor.trades());
    }

    // `_ExtendedIsThreatToPlayer` (@0x81b70-0x81de5).

    /// NPC `id` at rest, flying, intact and idle.
    fn plain(id: u32) -> Npc {
        crate::traffic::fixture::npc(id, ShipStats::default())
    }

    /// The player's hired escort, NPC `id`.
    fn hired(id: u32) -> Npc {
        Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..plain(id)
        }
    }

    #[test]
    fn a_flying_ship_targeting_the_player_threatens_player_extended_but_one_held_or_coasting_never_does()
     {
        let attacking = Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..plain(1)
        };
        assert!(
            attacking.threatens_player_extended(&[], true),
            "goal 4, target 0"
        );
        let targeting = Npc {
            target: Some(ShipRef::Player),
            ..plain(1)
        };
        assert!(
            targeting.threatens_player_extended(&[], true),
            "the hit's target, set before `_AIMakeShipAttackPlayer`"
        );
        for mode in [
            Mode::HeldAtGate { ticks_left: 20 },
            Mode::Coasting { ticks_left: 30 },
            Mode::Emerging { ticks_left: 60 },
        ] {
            for npc in [&attacking, &targeting] {
                let waiting = Npc {
                    mode,
                    ..npc.clone()
                };
                assert!(!waiting.threatens_player_extended(&[], true), "{mode:?}");
            }
        }
        for mode in [Mode::JumpingIn { tick: 3 }, Mode::LeavingGate { tick: 3 }] {
            let coming = Npc {
                mode,
                ..targeting.clone()
            };
            assert!(
                coming.threatens_player_extended(&[], true),
                "{mode:?}: no wait"
            );
        }
        let disabled = Npc {
            condition: Condition::Disabled,
            ..targeting.clone()
        };
        assert!(!disabled.threatens_player_extended(&[], true));
        assert!(!plain(1).threatens_player_extended(&[], true));
        let elsewhere = Npc {
            target: Some(ShipRef::Npc(NpcId(5))),
            ..plain(1)
        };
        assert!(!elsewhere.threatens_player_extended(&[], true));
    }

    #[test]
    fn threatens_player_extended_reads_sight_the_wait_and_the_goal() {
        let p = ShipRef::Player;
        let targeting = Npc {
            target: Some(p),
            ..plain(1)
        };
        assert!(targeting.threatens_player_extended(&[], true));
        assert!(
            !targeting.threatens_player_extended(&[], false),
            "unseen (@0x81bc6)"
        );
        let players_own = Npc {
            target: Some(p),
            ..hired(1)
        };
        assert!(
            !players_own.threatens_player_extended(&[], true),
            "led by the player (@0x81b9f)"
        );
        for mode in [
            Mode::Emerging { ticks_left: 60 },
            Mode::HeldAtGate { ticks_left: 20 },
            Mode::Coasting { ticks_left: 30 },
            Mode::EnteringGate {
                gate: StellarId(300),
                ticks_left: 4,
            },
        ] {
            let waiting = Npc {
                mode,
                ..targeting.clone()
            };
            assert!(
                !waiting.threatens_player_extended(&[], true),
                "{mode:?}: its wait runs (@0x81bed)"
            );
        }
        for goal in [
            Goal::Inspect(p),
            Goal::Assist(crate::ai::Help::Refuel),
            Goal::Assist(crate::ai::Help::Repair),
            Goal::Dock(ShipRef::Npc(NpcId(5))),
            Goal::Follow(NpcId(5)),
            Goal::Formation { guard: None },
            Goal::Hold,
        ] {
            let spared = Npc {
                goal,
                ..targeting.clone()
            };
            assert!(
                !spared.threatens_player_extended(&[], true),
                "{goal:?} (@0x81bf3-0x81c28)"
            );
        }
        let escort = hired(7);
        let at_escort = Npc {
            target: Some(ShipRef::Npc(NpcId(7))),
            ..plain(1)
        };
        assert!(
            at_escort.threatens_player_extended(&[at_escort.clone(), escort], true),
            "a ship the player leads (@0x81c2e-0x81c4a)"
        );
        assert!(
            !at_escort.threatens_player_extended(std::slice::from_ref(&at_escort), true),
            "a ship nobody leads"
        );
    }

    #[test]
    fn threatens_player_extended_reads_only_the_target_as_led_by_the_player() {
        let at_seven = Npc {
            target: Some(ShipRef::Npc(NpcId(7))),
            ..plain(1)
        };
        assert!(
            !at_seven.threatens_player_extended(&[at_seven.clone(), plain(7), hired(8)], true),
            "another ship the player leads is not its target"
        );
    }

    #[test]
    fn threatens_player_extended_counts_a_lead_whose_escort_fights_the_player() {
        // The escort clause (@0x81cdd-0x81dcc): the lead targets an NPC.
        let p = ShipRef::Player;
        let lead = Npc {
            target: Some(ShipRef::Npc(NpcId(9))),
            ..plain(1)
        };
        let wing = Npc {
            leader: Some(NpcId(1)),
            target: Some(p),
            ..plain(2)
        };
        let threat = |lead: &Npc, wing: &Npc| {
            lead.threatens_player_extended(&[lead.clone(), wing.clone()], true)
        };
        assert!(threat(&lead, &wing), "its escort targets the player");
        assert!(
            threat(
                &lead,
                &Npc {
                    target: None,
                    goal: Goal::Attack(p),
                    ..wing.clone()
                }
            ),
            "its escort attacks the player"
        );
        let at_led = Npc {
            target: Some(ShipRef::Npc(NpcId(7))),
            ..wing.clone()
        };
        assert!(
            lead.threatens_player_extended(&[lead.clone(), at_led.clone(), hired(7)], true),
            "its escort targets a ship the player leads"
        );
        assert!(
            !threat(&lead, &at_led),
            "its escort targets a ship nobody leads"
        );
        assert!(
            threat(
                &Npc {
                    mode: Mode::Coasting { ticks_left: 30 },
                    ..lead.clone()
                },
                &wing
            ),
            "the lead's own wait is not read"
        );
        let waiting_wing = Npc {
            mode: Mode::Coasting { ticks_left: 30 },
            ..wing.clone()
        };
        assert!(!threat(&lead, &waiting_wing), "its escort's wait runs");
        let downed_wing = Npc {
            condition: Condition::Disabled,
            ..wing.clone()
        };
        assert!(!threat(&lead, &downed_wing), "its escort is disabled");
        let inspecting = Npc {
            goal: Goal::Inspect(ShipRef::Npc(NpcId(9))),
            ..lead.clone()
        };
        assert!(
            !threat(&inspecting, &wing),
            "the lead's goal is read (@0x81d38)"
        );
        assert!(!threat(&plain(1), &wing), "the lead has no target");
        let stranger = Npc {
            leader: Some(NpcId(3)),
            ..wing
        };
        assert!(!threat(&lead, &stranger), "another's escort");
    }

    fn hold(held: &NpcHold) -> Vec<(Good, u32)> {
        held.held().collect()
    }

    #[test]
    fn a_hold_starts_with_the_one_commodity_its_spawn_gives() {
        let spawned = NpcHold::of(Some((Good::Commodity(3), 7)));
        assert_eq!(spawned.tons(3), 7);
        assert_eq!(hold(&spawned), [(Good::Commodity(3), 7)]);
        assert_eq!(hold(&NpcHold::of(None)), []);
        assert_eq!(NpcHold::of(None), NpcHold::default());
    }

    #[test]
    fn a_hold_drops_a_junk_or_a_commodity_past_5_at_spawn() {
        let junk = NpcHold::of(Some((Good::Junk(crate::catalog::JunkId(130)), 4)));
        assert_eq!(junk, NpcHold::default());
        let seventh = NpcHold::of(Some((Good::Commodity(6), 4)));
        assert_eq!(seventh, NpcHold::default());
        assert_eq!(seventh.tons(6), 0);
        let sixth = NpcHold::of(Some((Good::Commodity(5), 4)));
        assert_eq!(hold(&sixth), [(Good::Commodity(5), 4)]);
    }

    #[test]
    fn a_ton_added_keeps_the_commodity_already_held() {
        let mut held = NpcHold::of(Some((Good::Commodity(3), 7)));
        held.add(1, 1);
        assert_eq!((held.tons(3), held.tons(1)), (7, 1));
        held.add(3, 2);
        assert_eq!(
            hold(&held),
            [(Good::Commodity(1), 1), (Good::Commodity(3), 9)],
            "in commodity order"
        );
    }

    #[test]
    fn a_ton_added_saturates_and_none_goes_past_commodity_5() {
        let mut held = NpcHold::of(Some((Good::Commodity(0), u32::MAX - 1)));
        held.add(0, 5);
        assert_eq!(held.tons(0), u32::MAX);
        let before = held;
        held.add(6, 1);
        held.add(u8::MAX, 1);
        assert_eq!(held, before);
        held.add(5, 1);
        assert_eq!(held.tons(5), 1);
    }

    #[test]
    fn the_held_commodities_skip_the_empty_ones() {
        let mut held = NpcHold::default();
        held.add(4, 2);
        held.add(2, 0);
        held.add(0, 3);
        assert_eq!(
            hold(&held),
            [(Good::Commodity(0), 3), (Good::Commodity(4), 2)]
        );
    }

    #[test]
    fn an_npc_person_carries_its_records_fields_and_is_invincible_below_zero() {
        let record = PersonRecord {
            flags: 0x0030,
            coward: 25,
            comm_quote: 3,
            hail_quote: 8,
            link_mission: Some(140),
            hail_pict: Some(5000),
            shield_mod: -1,
            ..nova_sim_catalog::fixture::person(151, 128)
        };
        assert_eq!(
            NpcPerson::of(&record, true),
            NpcPerson {
                id: PersonId(151),
                flags: 0x0030,
                coward: 25,
                comm_quote: 3,
                hail_quote: 8,
                mission: true,
                portrait: Some(5000),
                invincible: true,
                grudge: true,
                quoted: false,
                quoted_at: None,
            }
        );
        for shield_mod in [0, 100] {
            let record = PersonRecord {
                shield_mod,
                ..record.clone()
            };
            assert!(!NpcPerson::of(&record, false).invincible, "{shield_mod}");
        }
    }

    #[test]
    fn an_npc_follows_the_player_while_it_escorts_it_and_else_its_leader() {
        let npc = |leader: Option<u32>, escort: bool| Npc {
            leader: leader.map(NpcId),
            escort: escort.then(|| EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..crate::traffic::fixture::npc(
                1,
                crate::stats::ShipStats::new(nova_sim_base::fixture::FAST, &[]),
            )
        };
        assert_eq!(npc(None, false).leader_ref(), None);
        assert_eq!(
            npc(Some(3), false).leader_ref(),
            Some(ShipRef::Npc(NpcId(3)))
        );
        assert_eq!(npc(Some(3), true).leader_ref(), Some(ShipRef::Player));
    }

    #[test]
    fn an_npc_approaches_a_gate_it_is_bound_for_strictly_inside_its_reach_on_both_axes() {
        let gate = StellarId(300);
        let at = Vec2::new(1000.0, -500.0);
        let bound = |goal: Goal, x: f32, y: f32| {
            let mut npc = Npc { goal, ..plain(1) };
            npc.state.position = Vec2::new(x, y);
            npc
        };
        let landing = Goal::Land(gate);
        assert!(bound(landing, 1000.0, -500.0).approaches_gate(gate, at, 50.0));
        assert!(bound(Goal::EnterGate(gate), 1049.0, -451.0).approaches_gate(gate, at, 50.0));
        assert!(bound(landing, 951.0, -549.0).approaches_gate(gate, at, 50.0));
        for (x, y, why) in [
            (1050.0, -500.0, "x at the reach"),
            (950.0, -500.0, "x at the reach, the other side"),
            (1000.0, -450.0, "y at the reach"),
            (1000.0, -550.0, "y at the reach, the other side"),
            (0.0, 0.0, "far"),
        ] {
            assert!(
                !bound(landing, x, y).approaches_gate(gate, at, 50.0),
                "{why}"
            );
        }
        for goal in [
            Goal::Land(StellarId(301)),
            Goal::EnterGate(StellarId(301)),
            Goal::Idle,
            Goal::JumpOut,
        ] {
            assert!(
                !bound(goal, 1000.0, -500.0).approaches_gate(gate, at, 50.0),
                "{goal:?}"
            );
        }
        let disabled = Npc {
            condition: Condition::Disabled,
            ..bound(landing, 1000.0, -500.0)
        };
        assert!(!disabled.approaches_gate(gate, at, 50.0), "disabled");
    }
}
