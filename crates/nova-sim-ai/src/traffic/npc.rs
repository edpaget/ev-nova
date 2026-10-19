//! An NPC ship: what it is, how it performs, where it is, what it is
//! doing, and what it fights with.

use crate::ai::Goal;
use crate::ai::lost_sight::SearchTimer;
use crate::ai::plunder_contents::ClassMarines;
use crate::ai::pursuit::Pursuit;
use crate::bay::Carrier;
use crate::catalog::{GovtId, PersonId, PersonRecord, ShipId, StellarId, WeaponId};
use crate::cloak::{Cloak, CloakGear};
use crate::combat::ShipRef;
use crate::combat::armament::{Armament, LauncherRounds, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::combat::impact::TractorLock;
use crate::combat::ion::Ion;
use crate::escort::{EscortClass, EscortDuty, EscortOrder, FormationSlot, KEEP_FORMATION};
use crate::flight::{Flyer, ShipState};
use crate::geometry::Vec2;
use crate::inertialess::{InertialDampener, is_inertialess};
use crate::market::Good;
use crate::reserves::Reserves;
use crate::self_repair::FormerEscort;
use crate::stats::ShipStats;
use crate::traffic::ShipBehav;
use crate::traffic::plunder_dock::Docking;
use crate::traffic::ship_display::NpcDisplay;
use crate::traffic::warp_out::Warp;

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
    /// (@0x972d) sees it and it is drawn. See [`Npc::arrived`]. A ship
    /// boarded on a mission's assist path waits so too
    /// ([`Npc::wait_on_course`]); one not intact drifts instead, its wait
    /// counting down all the same.
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
    /// An escape pod (class 0x2ff, `ESCAPE_POD_SHIP`) adrift, its AI wait
    /// +0x4c this many ticks yet: `_AIDispatch` flies it at full thrust
    /// along its heading and counts the wait down, and frees its slot once
    /// it is below 0 (@0x8fb62-0x8fbba; see
    /// [`pod_flight`](crate::traffic::pod_flight)). A pod in any other
    /// mode has a wait of none, as every spawner leaves +0x4c at 0.
    Adrift {
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
    /// Whether it starts with zero fuel (`përs` `Flags2`
    /// [`ZERO_FUEL`](crate::traffic::persons::ZERO_FUEL), person +0x16 & 1):
    /// the one ship that needs a jump's fuel now to jump out
    /// ([`can_jump`](crate::ai::leave::can_jump)).
    pub zero_fuel: bool,
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
            zero_fuel: record.flags2 & crate::traffic::persons::ZERO_FUEL != 0,
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
    /// Its speed along its heading, the original's +0x48, which an
    /// inertialess ship flies by (see [`Npc::flies_inertialess`]): none as
    /// it spawns, as every spawn zeroes it with the velocity
    /// (`_GenericRandomShipSpawn` @0x3c954, `_SpawnPerson` @0x41037,
    /// `_HyperShipSpawn` @0x42ab1, `_SetupShipsInSystem` @0x42f81, and
    /// the rest), and a fighter's its carrier's (`_LaunchFighter`
    /// @0x3d6da-0x3d6e0).
    pub speed: f32,
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
    /// AI's passes ([`Trigger::keep`]) or, waiting, by its turret find
    /// ([`GateWait`](crate::ai::GateWait)), kept from tick to tick, held
    /// through its wait ([`fire::gated`](crate::ai::fire::gated)), and
    /// cleared once a pick whose weapon record has no `Flags` 0x0002 fires
    /// ([`after_firing`](crate::ai::fire::after_firing)) or the ship is
    /// captured. A new ship starts with the pick its berth's last ship
    /// left, as no spawn writes +0x72
    /// ([`berth_picks`](crate::traffic::berth_picks)).
    pub pick: Option<WeaponId>,
    /// The ship it targets, as it last decided.
    pub target: Option<ShipRef>,
    /// Its provocation (the original's +0x96): the nominal damage of the
    /// hits it answered while it fights, each mass and energy arg above
    /// none (see [`react::victim`](crate::ai::react::victim)), not the
    /// damage it took; or 1 once its government's
    /// 0x0004 word turns it on the player (@0x8b7e5, @0x8cb9d,
    /// [`Decision::provokes`](crate::ai::Decision::provokes)); none once it
    /// goes back to an idle goal or is disabled.
    pub provoked: f32,
    /// Its stray provocation, the original's +0xc910: 1.75 × the weight of
    /// each stray hit of the player's on it, easing by 0.5 a tick, and
    /// none once it answers a hit; the player's stray hit is answered only
    /// from 50 (see [`answer_flag`](crate::ai::answer_flag)). It starts at
    /// none at every spawn and launch, and is never saved.
    pub stray: f32,
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
    /// The stellar of this system it has visited, the original's +0x92,
    /// which the idle block reads to send a ship that has visited away
    /// rather than to another stellar
    /// ([`idle_block::visited_here`](crate::ai::idle_block::visited_here)).
    /// `None` stands for both the original's -1 and -2: every spawn starts
    /// it at -2 (e.g. @0x1c3d6, @0x41ed4, @0x7f3fa), and the interceptor's
    /// patrol writes -2 (@0x8d01e). Its writers here: the gate emerge
    /// ([`Npc::emerge_from_gate`], @0x8955e-0x89574) and a miner's shuttle
    /// as it sets off (@0x8b3ae, [`Decision::visits`](crate::ai::Decision::visits),
    /// see [`miner`](crate::ai::miner)), which also reads it to leave that
    /// stellar out of its pick (@0x8b373). The landing's end
    /// (`_HighLevelAIHandler` @0x8db00) is
    /// rdm:phase/stellar-fidelity/phase-5-npc-landing-reach's.
    pub visited: Option<StellarId>,
    /// Whether its travel stellar, the original's +0x6c, is set: what a
    /// pirate's idle block reads to skip its pick (@0x8c472-0x8c477, read
    /// through [`travel::is_set`](crate::ai::travel::is_set), which adds the
    /// stellar goals and the emerge's gate). Its writers are the decisions'
    /// ([`Decision::travel`](crate::ai::Decision::travel), taken by
    /// [`travel::take`](crate::ai::travel::take)) and the drops that write
    /// -1 (see [`travel`](crate::ai::travel)). Every spawn clears it
    /// (@0x1c3c1, @0x3c444, @0x3c9e2).
    pub travel_set: bool,
    /// Whether its ship class carries a repair system among its default
    /// items, so that, disabled, it can repair itself
    /// ([`ShipKind::self_repair`](crate::traffic::table::ShipKind)).
    pub self_repair: bool,
    /// Whether its ship class is immobile, a `Speed` and `Accel` of 0
    /// ([`ShipKind::immobile`](crate::traffic::table::ShipKind)): each
    /// tick `_HandleShip` sets its velocity to none and skips its move
    /// (@0x34209-0x3423b), so a push lasts only until the next tick and
    /// it never moves (see the [`autopilot`](crate::traffic::autopilot)).
    /// No stock class is; the player's ship never is (`_HandlePlayer` has
    /// no such test).
    pub immobile: bool,
    /// Its skill (ship +0x40), rolled as it spawns
    /// ([`random_skill_level`](crate::skill::random_skill_level)) and kept
    /// until it leaves the system: its top speed and thrust take it (see
    /// the [`skill`](crate::traffic::skill) module). An escort kept across
    /// a system entry, and a ship enlisted in place, keep theirs. It is
    /// not saved: a loaded pilot's escorts roll afresh, as
    /// `_LoadPilotData` spawns them anew.
    pub skill: f32,
    /// Its government's `SkillMult` factor
    /// ([`Governments::skill_mult`](crate::govt::Governments::skill_mult)),
    /// kept in step with its [`govt`](Npc::govt) by every write of it
    /// (see the [`skill`](crate::traffic::skill) module).
    pub skill_mult: f32,
    /// Whether its ship class is `shïp` `Flags` 0x0400
    /// ([`STILL_HULL`](crate::skill::STILL_HULL)): it has no top speed and
    /// no thrust, its turn untouched.
    pub still: bool,
    /// Its ship type's miner bits, `shïp` `Flags3` & 0x0003
    /// ([`ShipKind::miner`](crate::traffic::table::ShipKind)): a leaderless
    /// ship with either goes to the miner's AI (see
    /// [`miner`](crate::ai::miner)).
    pub miner: u16,
    /// Whether its ship type carries an asteroid scoop among its default
    /// items, the original's +0xc3 (`_ShipCanScoop` @0x75c7, see
    /// [`ShipKind::can_scoop`](crate::traffic::table::ShipKind)), which
    /// only the miner's AI reads.
    pub can_scoop: bool,
    /// Its ship type's `Holds` in tons (`shïp` +0x0,
    /// [`ShipKind::holds`](crate::traffic::table::ShipKind)): a scooping
    /// miner leaves once its own hold ([`NpcHold::total`]) reaches it.
    pub holds: u16,
    /// Its class's `shïp` `Fuel`, raw ([`ShipKind::class_fuel`](crate::traffic::table::ShipKind::class_fuel)):
    /// above 99, it can jump out whatever fuel it holds, but for a person
    /// that starts with zero fuel ([`can_jump`](crate::ai::leave::can_jump)).
    pub class_fuel: i16,
    /// What it was, while it is a former escort of the player's: disabled
    /// or breaking up as one, it left the fleet with this record (the
    /// original's +0xc8de). It forgets once intact, boarded or taken into
    /// the fleet again (see [`self_repair`](crate::self_repair)).
    pub former: Option<FormerEscort>,
    /// Its tractor lock, if a tractor beam holds it or it holds itself
    /// along its own (the original's +0xc8dc and +0xb4, see
    /// [`impact`](crate::combat::impact)): none as it spawns or launches,
    /// as the original's spawns and `_LaunchFighter` set it (@0x3c5c8,
    /// @0x3cb1c, @0x3d71d). The fight sets and touches it; its hold pulls
    /// it, releases it and lets it lapse, and slows it while another ship
    /// holds it (see the [`tractor_hold`](crate::traffic::tractor_hold)
    /// module).
    pub tractor: Option<TractorLock>,
    /// Its ionization (the original's +0x5c and +0xb0, see
    /// [`ion`](crate::combat::ion)): none as it spawns, launches or is
    /// left as the player's hulk, as `_RandomShipSpawn`,
    /// `_GenericRandomShipSpawn`, `_LaunchFighter` and the eject set it
    /// (@0x3c457, @0x3c9f1, @0x3d8aa, @0x6f372). The fight raises it;
    /// [`ionization`](crate::traffic::ionization) sheds it and slows the
    /// ship by it. Never saved.
    pub ion: Ion,
    /// The crew pods it has left to eject as it dies (the original's
    /// +0xc908): its class's `PodCount` as it spawns, launches or is left
    /// as the player's hulk, as `_RandomShipSpawn`,
    /// `_GenericRandomShipSpawn`, `_MissionDudeSpawn` and `_LaunchFighter`
    /// set it (@0x3c4b0, @0x3ca48, @0x3cff7, @0x3d8fe), one fewer for each
    /// pod it ejects (see `nova_sim::npc_escape`). Written raw
    /// ([`HullSpec::signed_pod_count`]), so a class of negative
    /// `PodCount` leaves it negative, which a stray beam of `ExitType` 5
    /// reads when it steps the word (see `nova_sim::stray_words`). Never
    /// saved.
    pub pods_left: i16,
    /// The odds against it as its slot caches them (the original's
    /// +0xc914): -1 ([`odds_cache::STALE`](crate::traffic::odds_cache::STALE))
    /// from its spawn until the frame's odds pass reaches its berth, then
    /// the odds then, until the next round (see
    /// [`odds_cache`](crate::traffic::odds_cache)). What its odds retreat
    /// and its calls for reinforcements read by the engine
    /// ([`cached_odds`](crate::ai::odds::cached_odds)). Never saved.
    pub odds: f32,
    /// Whether it has no AI: the original's AI type 0 or below (+0x88),
    /// which only the player's eject hulk has, -1 (`_HandlePlayer`
    /// @0x6f31f). Its [`ai_type`](Npc::ai_type) stays its class's
    /// `InherentAI`, as the engine's [`AiType`] has no "none". A ship
    /// with no AI never thinks (`_AIDispatch`'s gate, `_DoPlayGameWork`
    /// @0x4427e), reads odds 0 (the odds pass @0x44307-0x4431f), answers
    /// no hit (`_DamageShip` @0x3b459), and reads as no AI type to the
    /// readers that test another ship's (`_InterceptorAI` @0x8cd4e,
    /// `_BraveTraderAI` @0x8b707, `_AIFindShipToPlunder` @0x7fdad).
    /// Never saved.
    pub no_ai: bool,
    /// Whether it has an afterburner (the original's +0xbd), rolled as it
    /// spawns or launches (see [`afterburner`](crate::ai::afterburner)):
    /// with one it rushes its target ([`pursuit`](crate::ai::pursuit)).
    pub afterburner: bool,
    /// The pursuit it keeps from tick to tick (substate 0x10 or 0x11) and
    /// this tick's frame (see [`pursuit`](crate::ai::pursuit)): none as it
    /// spawns. Never saved.
    pub pursuit: Pursuit,
    /// Its AI wait (the original's +0x4c) for the waits it takes while it
    /// flies, outside its [`Mode`]: a pirate's dock with its victim
    /// (`Rand(80)` + 100), its hold after a plunder (100), a ship a pirate
    /// took into its fleet (150) (see [`plunder`](crate::traffic::plunder))
    /// and a pirate's count wait (120, see
    /// [`board_wait`](crate::traffic::board_wait)). While it runs it counts
    /// in [`Npc::wait_running`], the traffic counts it down as the ship
    /// flies, its high level is skipped but for a drift, a hold or a count
    /// wait, and the ship moves on at its velocity with no turn or thrust
    /// but in its count wait, where it brakes. A hit's answer cuts it to
    /// 20. None as it spawns. Never saved.
    pub ai_wait: u32,
    /// The docking it flies with a disabled ship it plunders, its sub-goal
    /// and nav kept from tick to tick (see
    /// [`plunder_dock`](crate::traffic::plunder_dock)): none as it spawns,
    /// once its plunder runs, and with any goal but an attack or a plunder.
    /// Never saved.
    pub docking: Option<Docking>,
    /// Its search timer for a target it lost sight of (the original's
    /// +0xc90c): written only by `_HighLevelAIHandler`'s lost-sight block
    /// (see [`lost_sight`](crate::ai::lost_sight)). No spawn or launch
    /// writes it, so a ship brought into a berth takes the timer the
    /// berth's last ship left, a never-used berth's 0, as
    /// [`RuleKey::ReusedSlotSearch`](crate::RuleKey::ReusedSlotSearch)
    /// reads it (see [`berth_search`](crate::traffic::berth_search)).
    /// Never saved.
    pub lost_sight: SearchTimer,
    /// The berth ([`Npc::berth`]) of its swarm leader, the original's
    /// +0xc906, `None` for -1: the lowest berth of a swarming class fighting
    /// its target, kept while it stays so (see
    /// [`swarm`](crate::ai::swarm)). A berth, not an [`NpcId`], as the
    /// original holds a slot index: a name for a berth re-used before the
    /// next pass reads its new ship. None at every spawn and launch
    /// (`_AISetup` @0x7f415). Never saved.
    pub swarm_leader: Option<u32>,
    /// Its class's `Strength` and the marines its default items give its
    /// plunder (see [`plunder_contents`](crate::ai::plunder_contents)), as
    /// the spawn table sets them ([`ShipKind`](crate::traffic::table::ShipKind)).
    pub marines: ClassMarines,
    /// Its warp stage, +0x50 and +0xa4, and the sub-goal that drives it
    /// (see [`warp_out`](crate::traffic::warp_out)): none as it spawns,
    /// as every spawn clears +0x50. Never saved.
    pub warp: Warp,
    /// Its drawn state: its bank, engine glow, display timers and sets,
    /// and how it was flown this tick (see
    /// [`ship_display`](crate::traffic::ship_display)). Never saved.
    pub display: NpcDisplay,
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

    /// The hold of `words`, the tons of commodities 0-5 in order.
    #[must_use]
    pub fn from_words(words: [u32; NPC_COMMODITIES]) -> Self {
        Self { tons: words }
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

    /// The tons it holds in all, `_TotalCargo` of an NPC (@0x4a0b-0x4a26):
    /// its six commodity words summed.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.tons.iter().map(|&tons| u64::from(tons)).sum()
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
    /// before (see the session's `mission_ships` module). A `ShipBehav` 1
    /// ship joins on its own; one of any `ShipBehav` captured for an escort
    /// joins as it is captured, keeping its mark (`_DoPlunderDialog`
    /// @0x9459b-0x947cf, see `nova_sim::Session::assign`). It is never an
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
    /// ([`Npc::own_handling`], a third while another ship holds it in a
    /// tractor beam) × `led_speed`, the factor for the player and the
    /// ships it leads (see `nova_sim::top_speed`), while it
    /// [escorts the player](Npc::escorts_player), whose leader (+0x9a) is
    /// then the player; otherwise its own.
    #[must_use]
    pub fn top_speed(&self, led_speed: f32) -> f32 {
        let own = self.own_handling().max_speed;
        if self.escorts_player() {
            own * led_speed
        } else {
            own
        }
    }

    /// Whether it is a mission escort: a mission's special ship that has
    /// joined the player's formation ([`MissionMark::formation`]), a
    /// `ShipBehav` 1 ship or one captured for an escort.
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

    /// Whether it keeps close formation with its leader, at `leader`:
    /// it keeps formation ([`Goal::Formation`]) on a slot
    /// ([`Npc::formation`]) beside a leader that is there, within
    /// [`KEEP_FORMATION`] of the slot on each axis, where the autopilot
    /// moves it with the leader. That is the engine's counterpart of the
    /// original's substate 0xc, which `_IsShipInCloseForm` tests
    /// (@0x82f54-0x82f69).
    #[must_use]
    pub fn keeps_close_formation(&self, leader: Option<&ShipState>) -> bool {
        let (Goal::Formation { .. }, Some(leader), Some(place)) =
            (self.goal, leader, self.formation())
        else {
            return false;
        };
        let off = place.position(leader) - self.state.position;
        off.x.abs() <= KEEP_FORMATION && off.y.abs() <= KEEP_FORMATION
    }

    /// Whether it flies inertialess, its leader at `leader` (see
    /// [`is_inertialess`]): by its hull, or by an inertial dampener as
    /// `rule` counts an NPC's; never while it
    /// [keeps close formation](Npc::keeps_close_formation).
    #[must_use]
    pub fn flies_inertialess(&self, leader: Option<&ShipState>, rule: InertialDampener) -> bool {
        is_inertialess(
            self.stats.inertialess_hull,
            self.stats.dampener,
            Flyer::Npc,
            self.keeps_close_formation(leader),
            rule,
        )
    }

    /// The ship the threat tests read as the one it fights: its goal's
    /// quarry ([`Goal::quarry`]), or stopped ([`Goal::Wait`], state 6) the
    /// target it holds. The original's tests read +0x70 and spare none of
    /// state 6 (`_IsThreatToPlayer` @0x7f536-0x7f56b,
    /// `_ExtendedIsThreatToShip` @0x81a27), and a trader's stop keeps it
    /// (@0x8d803-0x8d80f; see
    /// [`trader_target`](crate::ai::lost_sight::trader_target)). In the
    /// other goals that leave the quarry none, +0x70 is -1 or the state is
    /// one the tests spare.
    #[must_use]
    pub fn fought(&self) -> Option<ShipRef> {
        match self.goal {
            Goal::Wait => self.target,
            goal => goal.quarry(),
        }
    }

    /// Whether it fights `ship` directly: it is intact, flying, and
    /// attacking, sniping at or fleeing from it, or stopped holding it as
    /// its target ([`Npc::fought`]).
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
            && self.fought() == Some(ship)
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
    /// the active and disabled tests and [`Npc::fought`] (the quarry of an
    /// attack, snipe or flight, or a stop's kept target) for the AI states
    /// and the target. Unlike `_IsThreatToPlayer`, there is
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
            && match self.fought() {
                Some(ShipRef::Npc(id)) => {
                    id == ship || npcs.iter().any(|n| n.id == id && n.leader == Some(ship))
                }
                Some(ShipRef::Player) | None => false,
            }
    }

    /// Regenerates its fuel a tick at its [`ShipStats::fuel_regen`], then
    /// holds it between none and its capacity, every tick whatever its
    /// condition, disabled and breaking up included (`_HandleShip`
    /// @0x35305-0x35374, `_ShipFuelGenRate` @0x2ffb for an NPC).
    pub fn regenerate_fuel(&mut self) {
        crate::fuel::regenerate(&mut self.reserves.fuel, self.stats.fuel_regen);
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
    /// [`threats_to_player`](crate::hail::Hail::threats_to_player), which
    /// reads `_AIFightEnemyThreatToPlayer`'s inline test (+0x70 == 0, AI
    /// state 3 or 4, @0x82cda-0x82cef), not `_IsThreatToPlayer`.
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
    /// [`Mode::Flying`] with no [`Npc::ai_wait`] for the wait (a ship
    /// coming out of a gate or jumping in has one, and so does a flying
    /// pirate docked or holding after a plunder, or a ship it took into
    /// its fleet), and
    /// [`Npc::fought`] (the quarry of an attack, snipe or flight, or a
    /// stop's kept target) for the AI states and the target.
    ///
    /// `_FindNearestThreatToPlayer` (@0x9529-0x956b) also takes a ship
    /// that fails this test but targets an active ship the player leads
    /// and is fighting it (`_IsShipFightingMe` @0x827fa: its target, in
    /// AI state 4, attack). That adds only a state-4 ship whose AI wait
    /// is still running: a pirate docked with a ship the player leads,
    /// which a disabled escort no longer is (`_DamageShip` @0x3afdd), so
    /// this one test stands for both branches.
    #[must_use]
    pub fn threatens_player_or_led(&self, led: impl Fn(NpcId) -> bool) -> bool {
        self.condition == Condition::Intact
            && self.mode == Mode::Flying
            && self.ai_wait == 0
            && match self.fought() {
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
    /// It serves the callers of `_IsThreatToPlayer` only: every modelled
    /// one asks it (`_IsPlayerThreatened` @0x8245c among them, the hail's
    /// [`player_threatened`](crate::hail::Hail::player_threatened)). The
    /// readers of `_ExtendedIsThreatToPlayer` @0x81b70 don't; they ask
    /// [`Npc::threatens_player_extended`], which lists them.
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
    ///   player leads](Npc::threatens_player_or_led) by [`Npc::fought`],
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
    /// It is the predicate for every reader of `_ExtendedIsThreatToPlayer`:
    ///
    /// - `_AIMakeShipAttackPlayer`'s quote gate (@0x89c53,
    ///   [`Traffic::turn_with_quote`](crate::traffic::Traffic::turn_with_quote));
    /// - point defence (`_HandleShipPointDefense` @0x395fe), which
    ///   `nova_sim::Session`'s fight terms feed with the player's sight of
    ///   the ship;
    /// - the threatened-by-enemy tests (`_IsPlayerThreatenedByMyEnemy`
    ///   @0x824b8, `_IsShipThreatenedByMyEnemy` @0x82525,
    ///   [`threatened`](crate::ai::threatened)), and through the first rank
    ///   battle help ([`Hail::threatened_by_enemy`](crate::hail::Hail::threatened_by_enemy));
    /// - the comm dialog's attitude (`_AIDoesShipLikePlayer` @0x8218c,
    ///   `_CommDialogUpdate` @0x90625, `_DrawCommDialogButtons` @0x28d5a,
    ///   `_DoCommDialog` @0x95ce1, @0x960fa, @0x966da;
    ///   [`hail::like`](crate::hail::like));
    /// - the player's escorts' threat picks (`_AIFightThreatToParent`
    ///   @0x81e5c-0x81f7d, `_IsShipANearbyThreatToParent` @0x83744;
    ///   [`escort::threats`](crate::escort::threats),
    ///   [`escort::threat_score`](crate::escort::threat_score)).
    ///
    /// The stellar-defender clause above is planet-domination's.
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
    /// out and flying, it has none. While it runs, `_HandleShip` skips the
    /// ship's thrust and fire (@0x34a54-0x34a72): the fight fires only its
    /// fighter bays and point defence, its pick kept
    /// ([`fire::gated`](crate::ai::fire::gated)). So does its AI wait
    /// while it flies ([`Npc::ai_wait`]): a pirate's dock and hold after a
    /// plunder, a ship a pirate took into its fleet, and the wait of a
    /// ship following its leader's warp out while the leader still turns
    /// (`_LowLevelAIHandler` @0x85bc3, see
    /// [`warp_out`](crate::traffic::warp_out)). The waits the engine
    /// doesn't fly yet (a landed ship's stay, a launched fighter's) will
    /// count here once built (rdm weapons-fidelity phase 17's findings).
    ///
    /// A pirate's count wait ([`Goal::CountWait`]) is this AI wait too, as
    /// the original keeps it in +0x4c, read so by
    /// `_ExtendedIsThreatToPlayer` (@0x81bed). `_HandleShip` exempts goal
    /// 0x16 from its gate ([`Npc::count_wait`]), and so does
    /// [`fire::gated`](crate::ai::fire::gated); a pirate turned out of it
    /// onto another goal keeps the wait, gate and all.
    #[must_use]
    pub fn wait_running(&self) -> bool {
        matches!(
            self.mode,
            Mode::Emerging { .. }
                | Mode::HeldAtGate { .. }
                | Mode::Coasting { .. }
                | Mode::EnteringGate { .. }
        ) || self.ai_wait > 0
    }

    /// Whether it is in its count wait, goal 0x16 ([`Goal::CountWait`]):
    /// `_AIIsShipDoingCountWait` (@0x89857), which `_HandleShip` reads to
    /// let it turn, thrust and fire while its wait runs (@0x3443f-0x34454,
    /// @0x34a54-0x34a72).
    #[must_use]
    pub fn count_wait(&self) -> bool {
        self.goal == Goal::CountWait
    }

    /// Sets its AI wait (+0x4c) to `ticks_left`, as boarding a mission's
    /// ship on the assist path does (`_HandlePlayerBoardAttempt` @0x65746):
    /// flying, or already holding its course, it holds its course for that
    /// long ([`Mode::Coasting`]), with no steering, thrust or fire, its
    /// wait counting down even while it is disabled. A ship gliding in,
    /// bursting out, or in or at a gate keeps its own glide or wait, which
    /// the original's write would overwrite; a disabled ship is boarded
    /// in flight, so none is. A wait of none leaves it as it is.
    pub fn wait_on_course(&mut self, ticks_left: u32) {
        if ticks_left > 0 && matches!(self.mode, Mode::Flying | Mode::Coasting { .. }) {
            self.mode = Mode::Coasting { ticks_left };
        }
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
    /// (+0x70 = -1, @0x89544), and its trigger released. A ship with a
    /// fuel tank has visited the gate ([`Npc::visited`], [`Npc::gate_visit`]).
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
        self.visited = Self::gate_visit(gate, self.reserves.fuel.max);
        self.target = None;
        self.trigger = Trigger::default();
    }

    /// The stellar a ship whose tank holds `fuel_capacity` has visited as it
    /// comes out of `gate`: the gate when `_ShipFuelCapacity` is above 0,
    /// else none (-2, @0x8955e-0x89574). The original makes the write under
    /// its gate-index test of at most 0x7ff (@0x8952d, @0x8955c), which
    /// every stellar passes.
    #[must_use]
    pub fn gate_visit(gate: StellarId, fuel_capacity: f32) -> Option<StellarId> {
        (fuel_capacity > 0.0).then_some(gate)
    }

    /// Whether it is in hyperspace: its jump state +0x50 outside
    /// (-900, 0], the gate of its government's word (`_WarshipAI`
    /// @0x8b77d-0x8b798, `_InterceptorAI` @0x8cb21-0x8cb40). Gliding in
    /// ([`Mode::JumpingIn`]) it is -999.0: `_AIDispatch` sends a ship below
    /// -900 to goal 8 (@0x8fcf5), whose handler sets -999.0 (@0x8f3d2).
    /// Bursting out of its gate ([`Mode::LeavingGate`]) it has goal 8
    /// from its first moving tick (@0x8dd8a), so -999.0 too; on tick 0 its
    /// goal is still 0x15. Waiting in its gate or held there
    /// ([`Mode::Emerging`], [`Mode::HeldAtGate`]) it is -1.0 (@0x89555);
    /// entering a gate, coasting or flying, 0; a ship whose warp out is
    /// under way, above 0 ([`Warp::under_way`], see
    /// [`warp_out`](crate::traffic::warp_out)), is in hyperspace too.
    #[must_use]
    pub fn in_hyperspace(&self) -> bool {
        match self.mode {
            Mode::JumpingIn { .. } => true,
            Mode::LeavingGate { tick } => tick > 0,
            _ => self.warp.under_way(),
        }
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

    /// The heading its low level last wanted (+0x68): the last body's
    /// heading of its pursuit frame while that frame is for its goal
    /// ([`Pursuit::frame`]); none without such a frame.
    #[must_use]
    pub fn wanted_heading(&self) -> Option<i16> {
        self.pursuit
            .frame
            .filter(|frame| frame.goal == self.goal)
            .and_then(|frame| frame.last())
            .map(|body| body.heading)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// An NPC regenerating `rate` fuel a tick, holding `now` of its 300.
    fn fuelled(rate: f32, now: f32) -> Npc {
        let stats = ShipStats {
            fuel_regen: rate,
            fuel: 300.0,
            ..ShipStats::default()
        };
        let mut npc = crate::traffic::fixture::npc(1, stats);
        npc.reserves.fuel.now = now;
        npc
    }

    #[test]
    fn the_heading_wanted_is_the_last_body_of_a_frame_for_its_goal() {
        use crate::ai::fire::Manoeuvre;
        use crate::ai::pursuit::{Body, Frame};
        let body = |heading| {
            Some(Body {
                manoeuvre: Manoeuvre::Dogfight,
                heading,
            })
        };
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::default());
        assert_eq!(npc.wanted_heading(), None, "no frame");
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.pursuit.frame = Some(Frame {
            goal: npc.goal,
            bodies: [body(10), body(20)],
            speed_cap: None,
        });
        assert_eq!(npc.wanted_heading(), Some(20), "the last body");
        npc.goal = Goal::Idle;
        assert_eq!(npc.wanted_heading(), None, "a frame for another goal");
    }

    #[test]
    fn an_npc_regenerates_fuel_at_its_rate_up_to_its_capacity() {
        let mut npc = fuelled(0.5, 298.0);
        npc.regenerate_fuel();
        assert_eq!(npc.reserves.fuel.now, 298.5);
        for _ in 0..10 {
            npc.regenerate_fuel();
        }
        assert_eq!(npc.reserves.fuel.now, 300.0, "up to its capacity");
    }

    #[test]
    fn an_npcs_fuel_is_held_to_its_capacity_and_above_none_every_tick() {
        let mut npc = fuelled(0.0, 310.0);
        npc.regenerate_fuel();
        assert_eq!(npc.reserves.fuel.now, 300.0, "clamped with no rate");
        let mut npc = fuelled(-0.5, 0.7);
        npc.regenerate_fuel();
        assert!((npc.reserves.fuel.now - 0.2).abs() < 1e-6);
        npc.regenerate_fuel();
        assert_eq!(npc.reserves.fuel.now, 0.0, "a drain stops at none");
    }

    #[test]
    fn a_disabled_or_dying_npc_still_regenerates_fuel() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        ] {
            let mut npc = fuelled(0.5, 100.0);
            npc.condition = condition;
            npc.regenerate_fuel();
            assert_eq!(npc.reserves.fuel.now, 100.5, "{condition:?}");
        }
    }

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
    fn a_stopped_ship_threatens_by_the_target_it_kept() {
        // `_IsThreatToPlayer` (@0x7f536-0x7f56b) and `_ExtendedIsThreatToShip`
        // spare none of state 6, and read +0x70, which a trader's stop
        // keeps (@0x8d803-0x8d80f).
        let asker = NpcId(1);
        let escort = NpcId(9);
        let led = |id: NpcId| id == escort;
        let stopped = |target| Npc {
            goal: Goal::Wait,
            target,
            ..crate::traffic::fixture::npc(4, ShipStats::default())
        };
        let on_player = stopped(Some(ShipRef::Player));
        assert!(on_player.threatens_player_or_led(led));
        assert!(on_player.threatens_player());
        assert!(on_player.threatens_player_extended(&[], true));
        assert!(stopped(Some(ShipRef::Npc(escort))).threatens_player_or_led(led));
        assert!(!stopped(None).threatens_player_or_led(led), "no target");
        assert!(!stopped(None).threatens_player());
        let npcs = [crate::traffic::fixture::npc(1, ShipStats::default())];
        assert!(stopped(Some(ShipRef::Npc(asker))).threatens_npc_among(asker, &npcs));
        assert!(!stopped(None).threatens_npc_among(asker, &npcs));
        let idle = Npc {
            goal: Goal::Idle,
            ..on_player.clone()
        };
        assert!(
            !idle.threatens_player_or_led(led),
            "an idle ship by its quarry alone"
        );
        let inspecting = Npc {
            goal: Goal::Inspect(ShipRef::Npc(asker)),
            target: Some(ShipRef::Npc(asker)),
            ..on_player
        };
        assert!(
            !inspecting.threatens_npc_among(asker, &npcs),
            "state 7 spared"
        );
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
    fn a_ship_with_a_fuel_tank_that_comes_out_of_a_gate_has_visited_it() {
        // `_AIMakeShipEmergeFromHyperGate` writes +0x92 the gate when
        // `_ShipFuelCapacity` > 0, else -2 (@0x8955e-0x89574).
        let mut npc = crate::traffic::fixture::npc(1, ShipStats::default());
        npc.reserves.fuel.max = 300.0;
        npc.reserves.fuel.now = 0.0;
        npc.emerge_from_gate(StellarId(310), 40);
        assert_eq!(
            npc.visited,
            Some(StellarId(310)),
            "by its tank, not its fuel"
        );
        npc.reserves.fuel.max = 0.0;
        npc.visited = Some(StellarId(128));
        npc.emerge_from_gate(StellarId(310), 40);
        assert_eq!(npc.visited, None, "no tank: -2, never visited");
        assert_eq!(Npc::gate_visit(StellarId(311), 1.0), Some(StellarId(311)));
        assert_eq!(Npc::gate_visit(StellarId(311), 0.0), None);
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

    #[test]
    fn a_ship_is_in_hyperspace_gliding_in_or_bursting_out_of_its_gate() {
        let gate = StellarId(129);
        for (mode, jumping) in [
            (Mode::JumpingIn { tick: 0 }, true),
            (Mode::JumpingIn { tick: 7 }, true),
            (Mode::LeavingGate { tick: 1 }, true),
            (Mode::LeavingGate { tick: 9 }, true),
            (Mode::LeavingGate { tick: 0 }, false),
            (Mode::Emerging { ticks_left: 5 }, false),
            (Mode::HeldAtGate { ticks_left: 5 }, false),
            (
                Mode::EnteringGate {
                    gate,
                    ticks_left: 5,
                },
                false,
            ),
            (Mode::Coasting { ticks_left: 5 }, false),
            (Mode::Flying, false),
        ] {
            let npc = Npc { mode, ..plain(1) };
            assert_eq!(npc.in_hyperspace(), jumping, "{mode:?}");
        }
    }

    #[test]
    fn a_ship_whose_warp_is_under_way_is_in_hyperspace() {
        let mut npc = plain(1);
        npc.warp.stage = 2.0;
        assert!(npc.in_hyperspace(), "warping out");
        npc.warp.stage = 0.0;
        assert!(!npc.in_hyperspace(), "its warp over");
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
    fn a_pirate_plundering_the_player_threatens_it_and_one_in_its_count_wait_does_not() {
        let plunder = Npc {
            goal: Goal::Plunder {
                ship: ShipRef::Player,
                hopeless: false,
            },
            ..plain(1)
        };
        assert!(
            plunder.threatens_player_among(&[]),
            "0xd is no excluded state"
        );
        assert!(plunder.threatens_player_extended(&[], true));
        // A hit sets its target to the player (`_DamageShip` @0x3be5d), but
        // its wait (+0x4c) runs, and the next frame clears the target.
        let waiting = Npc {
            goal: Goal::CountWait,
            ai_wait: 20,
            target: Some(ShipRef::Player),
            ..plain(1)
        };
        assert!(!waiting.threatens_player_extended(&[], true));
        let done = Npc {
            ai_wait: 0,
            ..waiting
        };
        assert!(done.threatens_player_extended(&[], true), "its wait is out");
    }

    #[test]
    fn a_count_wait_runs_its_wait_while_ticks_are_left_and_is_always_a_count_wait() {
        for (ticks_left, running) in [(0, false), (1, true), (120, true)] {
            let waiting = Npc {
                goal: Goal::CountWait,
                ai_wait: ticks_left,
                ..plain(1)
            };
            assert_eq!(waiting.wait_running(), running, "{ticks_left}");
            assert!(waiting.count_wait(), "{ticks_left}");
        }
        assert!(!plain(1).count_wait());
        assert!(!plain(1).wait_running());
        let coasting = Npc {
            mode: Mode::Coasting { ticks_left: 3 },
            ..plain(1)
        };
        assert!(coasting.wait_running() && !coasting.count_wait());
    }

    #[test]
    fn a_flying_ships_ai_wait_runs_its_wait_while_any_is_left() {
        for (ai_wait, running) in [(0, false), (1, true), (150, true)] {
            let waiting = Npc {
                ai_wait,
                ..plain(1)
            };
            assert_eq!(waiting.wait_running(), running, "{ai_wait}");
            assert!(!waiting.count_wait());
        }
    }

    #[test]
    fn a_ship_whose_ai_wait_runs_is_no_threat_by_its_quarry() {
        let attacking = Npc {
            goal: Goal::Attack(ShipRef::Player),
            ..plain(1)
        };
        assert!(attacking.threatens_player_or_led(|_| false));
        let waiting = Npc {
            ai_wait: 1,
            ..attacking
        };
        assert!(!waiting.threatens_player_or_led(|_| false), "@0x7f518");
        assert!(!waiting.threatens_player_among(&[]));
        assert!(
            !waiting.threatens_player_extended(&[], true),
            "nor by its target while its wait runs"
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
    fn a_holds_total_sums_its_six_commodities() {
        let mut held = NpcHold::of(Some((Good::Commodity(0), 7)));
        held.add(5, 3);
        held.add(2, u32::MAX);
        assert_eq!(held.total(), 10 + u64::from(u32::MAX), "wider than a word");
        assert_eq!(NpcHold::default().total(), 0);
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
    fn an_npc_person_starts_with_zero_fuel_by_its_flags2_0x0001_alone() {
        let record = |flags2| PersonRecord {
            flags2,
            ..nova_sim_catalog::fixture::person(151, 128)
        };
        assert!(NpcPerson::of(&record(0x0001), false).zero_fuel);
        assert!(NpcPerson::of(&record(0xFFFF), false).zero_fuel);
        assert!(!NpcPerson::of(&record(0xFFFE), false).zero_fuel);
        assert!(!NpcPerson::of(&record(0), false).zero_fuel);
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
                zero_fuel: false,
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

    #[test]
    fn a_wait_on_course_holds_a_flying_or_coasting_ship_and_leaves_any_other_mode() {
        for mode in [Mode::Flying, Mode::Coasting { ticks_left: 7 }] {
            let mut ship = Npc {
                mode,
                ..crate::traffic::fixture::npc(1, ShipStats::default())
            };
            ship.wait_on_course(100);
            assert_eq!(ship.mode, Mode::Coasting { ticks_left: 100 }, "{mode:?}");
            assert!(ship.wait_running(), "{mode:?}");
        }
        for mode in [
            Mode::JumpingIn { tick: 3 },
            Mode::LeavingGate { tick: 3 },
            Mode::Emerging { ticks_left: 7 },
            Mode::HeldAtGate { ticks_left: 7 },
            Mode::EnteringGate {
                gate: StellarId(300),
                ticks_left: 7,
            },
        ] {
            let mut ship = Npc {
                mode,
                ..crate::traffic::fixture::npc(1, ShipStats::default())
            };
            ship.wait_on_course(100);
            assert_eq!(ship.mode, mode, "its own glide or wait stands");
        }
        let mut idle = crate::traffic::fixture::npc(1, ShipStats::default());
        idle.wait_on_course(0);
        assert_eq!(idle.mode, Mode::Flying, "no wait");
    }
}
