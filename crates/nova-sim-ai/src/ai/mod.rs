//! NPC decisions: what each NPC sets out to do, on a timer kept apart from
//! the physics step (see [`traffic`](crate::traffic)).
//!
//! A [`Behaviour`] decides an NPC's [`Goal`] from what is around it
//! ([`Surroundings`]), and the fire command it holds and the ship it
//! targets, and answers the hits ships take ([`Behaviour::react`]). The
//! [`traffic`](crate::traffic) autopilot then flies each goal, every tick.
//!
//! - [`Peaceful`]: traffic that never fights: it lands, enters a gate, or
//!   leaves, and escorts follow their lead. Its decisions are the idle
//!   block the four AI types share ([`idle`]).
//! - [`odds`]: how strong a ship and its friends are, the odds against it,
//!   and whether a chase is hopeless.
//! - [`target`]: whom a warship or interceptor attacks: the player, by its
//!   government's relations and the player's legal record, or an NPC;
//!   [`candidates`], the candidate pass its steps share; and
//!   [`threatened`], whether a friend is threatened by a ship's enemy.
//! - [`flee_drop`]: a fleeing ship giving up fleeing an ally it spares.
//! - [`fire`]: which weapon an NPC fires, as it attacks, snipes or flees.
//! - [`stand_off`]: how far a stand-off ship keeps from its target, by its
//!   weapons' longest reach, and how it flies there.
//! - [`answer_flag`]: whether a ship hit answers the hit at all, and the
//!   player's stray provocation.
//! - [`react`]: how a ship hit answers, and how others come to its help.
//! - [`call_for_help`]: the calls a think makes on the message line
//!   ([`CommCall`]): a fleeing trader's distress call and an
//!   interceptor's pre-scan hail, and the distress call's gates.
//! - [`reinforcements`]: a ship's call for its system's reinforcement
//!   fleet, the clock that brings it, and the daily countdown before the
//!   next.
//! - [`wimpy`], [`brave`], [`warship`] and [`interceptor`]: Nova's four AI
//!   types, each its own [`Behaviour`], and [`NovaAi`], which routes each
//!   NPC to the one for its type.
//! - [`escort`]: [`EscortAi`], how the player's escorts fly and fight by
//!   their standing orders; [`NovaAi`] routes every escort to it.
//! - [`carried`]: [`CarriedAi`], how an NPC carrier's fighters fly and
//!   fight with their carrier; [`NovaAi`] routes every one to it.
//! - [`miner`]: [`MinerAi`], how a leaderless miner stops, mines, or
//!   shuttles to its nearest stellar; [`NovaAi`] routes every one to it.

pub mod afterburner;
pub mod answer_flag;
pub mod brave;
pub mod call_for_help;
pub mod candidates;
pub mod carried;
pub mod dispatch_gate;
pub mod escort;
pub mod escort_orders;
pub mod fire;
#[cfg(test)]
pub(crate) mod fixture;
pub mod flee_drop;
pub mod fleet_escort;
pub mod idle_block;
pub mod interceptor;
pub mod leave;
pub mod lost_sight;
pub mod miner;
pub mod nova;
pub mod odds;
pub mod out_of_ammo;
pub mod pirate;
pub mod plunder_contents;
pub mod plunder_target;
pub mod pursuit;
pub mod react;
pub mod regroup;
pub mod reinforcements;
pub mod sight;
pub mod stand_off;
pub mod swarm;
pub mod target;
pub mod threatened;
pub mod travel;
pub mod wait;
pub mod warship;
pub mod wimpy;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use crate::catalog::{EscortClass, GovtId, LandingSite, RankId, RankRecord, ShipId, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::Strike;
use crate::combat::armament::{Arsenal, Trigger};
use crate::combat::hull::{Condition, HullSpec};
use crate::flight::ShipState;
use crate::gate::GateKind;
use crate::govt::Governments;
use crate::handling::Handling;
use crate::landing::is_landable as landable;
use crate::reserves::Reserves;
use crate::rulebook::RuleSource;
use crate::traffic::destination;
use crate::traffic::gone::GoneShip;
use crate::traffic::npc::{Npc, NpcId};
use nova_sim_combat::asteroid::Asteroid;

pub use crate::traffic::destination::Admit;

pub use brave::BraveTrader;
pub use call_for_help::CommCall;
pub use carried::CarriedAi;
pub use escort::EscortAi;
pub use fleet_escort::FleetEscortAi;
pub use interceptor::Interceptor;
pub use miner::MinerAi;
pub use nova::NovaAi;
pub use warship::Warship;
pub use wimpy::WimpyTrader;

/// `Flags` 0x0400: ships the rank reaches fight a threat to the player
/// when hailed, for nothing (`_DoCommDialog` @0x95ddb).
pub const HELPS_IN_BATTLE: u16 = 0x0400;
/// `Flags` 0x0800: ships the rank reaches refuel or repair the player
/// for nothing (`_DoCommDialog` @0x95de1).
pub const HELPS_FREE: u16 = 0x0800;

/// What the pilot's ranks grant a hailed ship's help (see
/// `nova_sim::rank`'s module docs' **Hails**).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Favour {
    /// Some rank that reaches the ship has [`HELPS_IN_BATTLE`].
    pub battle_help: bool,
    /// Some rank that reaches the ship has [`HELPS_FREE`].
    pub free_help: bool,
}

/// What the ranks of `records` in `active` grant a ship of `govt` (see
/// `nova_sim::rank`'s module docs' **Hails**): each one that reaches `govt` by `reach`'s
/// reading of [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach)
/// adds its [`HELPS_IN_BATTLE`] and [`HELPS_FREE`].
#[must_use]
pub fn favour(
    records: &[RankRecord],
    active: &BTreeSet<RankId>,
    govt: Option<GovtId>,
    govts: &Governments,
    reach: RuleSource,
) -> Favour {
    let flags = records
        .iter()
        .filter(|rank| active.contains(&rank.id) && reaches(rank, govt, govts, reach))
        .fold(0, |flags, rank| flags | rank.flags);
    Favour {
        battle_help: flags & HELPS_IN_BATTLE != 0,
        free_help: flags & HELPS_FREE != 0,
    }
}

/// Whether `rank` reaches `govt` by `reach`'s reading of
/// [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach): by the
/// engine, when `_GovtAllies` finds them allied; by the other reading,
/// only a rank of `govt` itself.
#[must_use]
pub fn reaches(
    rank: &RankRecord,
    govt: Option<GovtId>,
    govts: &Governments,
    reach: RuleSource,
) -> bool {
    match reach {
        RuleSource::Engine => govts.allies(govt, rank.affil_govt),
        RuleSource::Bible => rank.affil_govt.is_some() && rank.affil_govt == govt,
    }
}

/// Which of the two blessings a rank's `Flags` give (see
/// `nova_sim::rank`'s module docs' **Blessing**): `_PlayerBlessedByGovt`'s `which`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blessed {
    /// `nova_sim::rank::BLESSES_PEACE`, `which` 0: the government's ships won't attack
    /// the player.
    Peace,
    /// `nova_sim::rank::BLESSES_LANDING`, `which` 1: the government's stellars clear the
    /// player to land.
    Landing,
}

/// The lowest government ID the blessing table holds: the original's
/// slot 0.
const FIRST_BLESSED: i32 = 128;

/// How many governments the blessing table holds, as the original's 256
/// slots.
const BLESSED_SLOTS: usize = 256;

/// The blessing table `_ResetPlayerPrecalcedValues` builds (see
/// `nova_sim::rank`'s module docs' **Blessing**): for each government 128-383, whether it
/// blesses [`Blessed::Peace`] and whether [`Blessed::Landing`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Blessing {
    /// Bit n: government 128 + n blesses peace.
    peace: [u64; 4],
    /// Bit n: government 128 + n blesses landing.
    landing: [u64; 4],
}

impl Blessing {
    /// No government blesses anything.
    pub const NONE: Self = Self {
        peace: [0; 4],
        landing: [0; 4],
    };

    /// Whether `govt` blesses `which` (`_PlayerBlessedByGovt`
    /// @0x2732-0x276b): never for an independent, or an ID outside
    /// 128-383, as the original's `> 0xff` test gives 0.
    #[must_use]
    pub fn blesses(&self, govt: Option<GovtId>, which: Blessed) -> bool {
        govt.and_then(blessed_slot)
            .is_some_and(|slot| self.set(which)[slot / 64] & (1 << (slot % 64)) != 0)
    }

    /// The set of `which`.
    fn set(&self, which: Blessed) -> &[u64; 4] {
        match which {
            Blessed::Peace => &self.peace,
            Blessed::Landing => &self.landing,
        }
    }

    /// Makes `govt` bless `which`; nothing for an ID outside 128-383.
    pub fn bless(&mut self, govt: GovtId, which: Blessed) {
        if let Some(slot) = blessed_slot(govt) {
            let set = match which {
                Blessed::Peace => &mut self.peace,
                Blessed::Landing => &mut self.landing,
            };
            set[slot / 64] |= 1 << (slot % 64);
        }
    }
}

/// `govt`'s slot in the blessing table, if it has one.
fn blessed_slot(govt: GovtId) -> Option<usize> {
    usize::try_from(i32::from(govt.0) - FIRST_BLESSED)
        .ok()
        .filter(|&slot| slot < BLESSED_SLOTS)
}

/// What an NPC sets out to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Goal {
    /// Nothing yet: the original's state 0, which a type's think reads to
    /// run its idle block (see [`idle`]). It comes to a stop where it is.
    #[default]
    Idle,
    /// Stop and wait: the original's state 6, which the idle blocks write
    /// when a ship that would leave can't jump (see [`idle_block`]), the
    /// interceptor's patrol when its pick is destroyed, the warship's while
    /// its fighters are out (see [`wait`]), a trader that loses sight of
    /// its target and can't jump (see [`lost_sight`]), a miner that can't
    /// mine or jump (`_MinerAI` @0x8b2d1, @0x8b452, see [`miner`]), and an
    /// escort holding position (`_EscortAI` @0x83e6b, see [`escort`]).
    /// `_HighLevelAIHandler` flies it as a stop (+0x50 = 0, sub 1,
    /// @0x8f101-0x8f112), and a hail finds it busy (`_AIIsShipBusy`
    /// @0x82fac-0x82fe4; see [`hail`](crate::hail)). It is sticky for the
    /// traders and the interceptor: their idle blocks need state 0, so only
    /// provocation, or a hunter's government's word, lifts it. It fights
    /// nothing and scoops nothing. A trader's stop keeps its target
    /// ([`lost_sight::trader_target`]), and the threat tests spare none of
    /// state 6 (`_IsThreatToPlayer`, `_ExtendedIsThreatToShip`,
    /// `_ExtendedIsThreatToPlayer`): a stopped ship threatens the target it
    /// holds ([`Npc::fought`]). `_AIMakeShipWait` (@0x82c63) and
    /// `_IsShipWaiting` (@0x82f96) are never called.
    Wait,
    /// Land on this stellar, and leave the system that way. A stellar
    /// that is a hypergate or wormhole is entered instead, as
    /// [`Goal::EnterGate`] (`_HighLevelAIHandler` state 1, @0x8d8b8-0x8d8fe).
    Land(StellarId),
    /// Enter this hypergate or wormhole, and leave the system through it:
    /// the original's goal 0x14 (`_HighLevelAIHandler` state 20, see
    /// [`gate_entry`](crate::traffic::gate_entry)).
    EnterGate(StellarId),
    /// Leave by hyperspace (the original's goal 2): fly out past 1000 of
    /// the centre, brake unless it expedites, and jump out (see
    /// [`leave`]).
    JumpOut,
    /// Keep with `lead`, its fleet's lead: the original's goal 0xa under
    /// an NPC parent. By the engine's reading of
    /// [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation) it keeps
    /// its slot in the lead's formation
    /// ([`Npc::fleet_slot`](crate::traffic::npc::Npc::fleet_slot)) by goal
    /// 0xa's formation flight, as [`Goal::Formation`] keeps the player's
    /// (see [`formation_flight`](crate::traffic::formation_flight)); by
    /// the other, it follows within
    /// [`FOLLOW_DISTANCE`](crate::traffic::autopilot::FOLLOW_DISTANCE) of
    /// the lead. A fleet escort flying `_EscortAI`'s formation (see
    /// [`fleet_escort`]) holds the ship it guards in `guard`, its +0x70,
    /// at which it fires its turret pick and toward which it closes in.
    /// It fights nothing: its quarry is none.
    Follow {
        /// The lead it keeps with.
        lead: NpcId,
        /// The ship its turrets fire at, if any.
        guard: Option<ShipRef>,
    },
    /// Attack this ship.
    Attack(ShipRef),
    /// Stand off and fire at this ship, which it cannot catch.
    Snipe(ShipRef),
    /// Run from this ship, its attacker.
    Flee(ShipRef),
    /// Fly up to this ship to look it over.
    Inspect(ShipRef),
    /// Fly over to the player and give it this help, once asked (see
    /// [`hail`](crate::hail)): it fights nothing, and decides nothing
    /// more until it is done or provoked.
    Assist(Help),
    /// Keep the escort's slot in the player's formation (see
    /// [`escort`](crate::escort)), firing its turrets at `guard`, a ship
    /// threatening the player, if any. It fights nothing: its quarry is
    /// none.
    Formation {
        /// The ship its turrets fire at, if any.
        guard: Option<ShipRef>,
    },
    /// Fly back to this ship, its carrier, and dock in its fighter bay
    /// (see [`bay`](crate::bay)). It fights nothing: its quarry is none.
    Dock(ShipRef),
    /// Hold where it is: the original's goal 10 with no leader, which a
    /// ship swapped in for a person by a `ShipGoal` 3 mission gets
    /// (`_AIMakeEscortFlyInForm` @0x89bbd-0x89bcd) and which no goal
    /// block of `_HighLevelAIHandler` flies without a leader
    /// (@0x8ed18-0x8ed31), and which a fleet's new lead takes over from a
    /// lost lead keeping formation
    /// ([`succession`](crate::traffic::succession), @0x893b0-0x893e1). It
    /// coasts with no thrust and no fire, and
    /// keeps the hold until provoked or its type's think lifts it (see
    /// [`idle`]). It fights nothing: its quarry is none.
    Hold,
    /// Mine asteroid slot 0: the original's state 16, which a miner takes
    /// (see [`miner`]). Its quarry is none, it doesn't scoop, and
    /// `_ExtendedIsThreatToPlayer` doesn't spare it (16 is none of its
    /// states). `_HighLevelAIHandler` flies it as order 1, a stop, while
    /// slot 0 is empty (@0x8f879); the traffic brakes it so whether or not
    /// slot 0 is live, until rdm:phase/asteroids/phase-5-ai-mining adds the
    /// chase and kill (orders 0x13 and 0x14, @0x8f832-0x8f903) and the
    /// shots that hit no ship.
    Mine,
    /// Disable this ship to board it: a pirate's goal 0xd (see
    /// [`pirate`]), which it takes against a trader, a former escort of the
    /// player's or the player, whose class has a crew. Against the ship
    /// intact it flies as an attack (`_HighLevelAIHandler`
    /// @0x8ea49-0x8eb3a, see [`pursuit`]), sniping (sub-goal 0xe) when
    /// `hopeless`, but it is no attack to the readers of goal 4
    /// ([`Goal::attacking`]); its shots disable and never destroy (see
    /// `nova_sim_combat::combat::plunder`). Against the ship disabled it
    /// docks with it, firing nothing (see
    /// [`plunder_dock`](crate::traffic::plunder_dock)), never hopeless.
    Plunder {
        /// The ship it disables.
        ship: ShipRef,
        /// Whether the chase is hopeless, outside the dogfight box
        /// (`_AIEvalHopelessChase` @0x8eace): it snipes.
        hopeless: bool,
    },
    /// Wait while another ship boards the ship it would plunder: a
    /// pirate's goal 0x16, its ticks the AI wait [`Npc::ai_wait`] (the
    /// original's +0x4c, apart from the goal, so a goal that replaces it
    /// keeps the wait; see [`board_wait`](crate::traffic::board_wait)). It
    /// brakes, targets and fires nothing, and fights nothing: its quarry is
    /// none.
    CountWait,
    /// Drift off from the ship it plundered: a pirate's goal 0xe
    /// (`_HighLevelAIHandler` @0x8eb6b-0x8ebd4), its ticks the AI wait
    /// [`Npc::ai_wait`] its plunder set (see
    /// [`plunder`](crate::traffic::plunder)). It targets and fights
    /// nothing, fires nothing, and moves on at its velocity 0.7 pixels a
    /// tick further along its heading, until its wait reads none.
    Drift,
    /// The regroup pause, the original's goal 0x13: a ship of a fleet
    /// whose lead was lost waits to take up its new lead's orders, deciding
    /// nothing and running no high level, flying and firing the substate
    /// it kept, until a 1-in-100 draw each tick releases it (see
    /// [`regroup`]). Every reader of the goal but the escort orders leaves
    /// it to its default: no attack, no fight, not in form, not spared.
    Regroup {
        /// The substate it keeps flying.
        kept: regroup::Kept,
    },
}

impl Goal {
    /// The ship it fights, attacking, sniping at, fleeing from or
    /// plundering it; none for any other goal.
    #[must_use]
    pub fn quarry(self) -> Option<ShipRef> {
        match self {
            Self::Attack(ship)
            | Self::Snipe(ship)
            | Self::Flee(ship)
            | Self::Plunder { ship, .. } => Some(ship),
            _ => None,
        }
    }

    /// The ship it plunders, `_AIShipIsPlundering` (@0x89841, goal 0xd),
    /// if any.
    #[must_use]
    pub fn plundering(self) -> Option<ShipRef> {
        match self {
            Self::Plunder { ship, .. } => Some(ship),
            _ => None,
        }
    }

    /// The ship it attacks or snipes at, if any: the original's goal 4,
    /// which `_IsShipAttacking` (@0x82f6a) and `_IsShipFightingMe`
    /// (@0x827fa) test. A plunder (goal 0xd) is none.
    #[must_use]
    pub fn attacking(self) -> Option<ShipRef> {
        match self {
            Self::Attack(ship) | Self::Snipe(ship) => Some(ship),
            _ => None,
        }
    }

    /// The ship it inspects, if any.
    #[must_use]
    pub fn inspecting(self) -> Option<ShipRef> {
        match self {
            Self::Inspect(ship) => Some(ship),
            _ => None,
        }
    }

    /// Whether it fights: attacking, sniping or fleeing. Any other goal is
    /// idle.
    #[must_use]
    pub fn fights(self) -> bool {
        self.quarry().is_some()
    }

    /// Whether it is scooping up asteroid boxes: `_AIShipIsScooping`
    /// (@0x88727), the original's goal 0x11, which the scoop branch of
    /// `_ShipCollisionHandler` asks of an NPC (see
    /// [`scoop`](crate::scoop)). No goal answers yes until
    /// rdm:phase/asteroids/phase-5-ai-mining adds the miner's.
    #[must_use]
    pub fn scooping(self) -> bool {
        match self {
            Self::Idle
            | Self::Wait
            | Self::Land(_)
            | Self::EnterGate(_)
            | Self::JumpOut
            | Self::Follow { .. }
            | Self::Attack(_)
            | Self::Snipe(_)
            | Self::Flee(_)
            | Self::Inspect(_)
            | Self::Assist(_)
            | Self::Formation { .. }
            | Self::Dock(_)
            | Self::Hold
            | Self::Mine
            | Self::Plunder { .. }
            | Self::CountWait
            | Self::Drift
            | Self::Regroup { .. } => false,
        }
    }

    /// Whether it is the original's goal 0xa, an escort flying in form
    /// (see [`Goal::spares_player_threat`]'s table): [`Goal::Follow`],
    /// [`Goal::Formation`] or [`Goal::Hold`].
    #[must_use]
    pub fn flies_in_form(self) -> bool {
        matches!(
            self,
            Self::Follow { .. } | Self::Formation { .. } | Self::Hold
        )
    }

    /// Whether a ship in this goal is no threat to the player by its
    /// target, as `_ExtendedIsThreatToPlayer` leaves out the original's
    /// goals 7, 9, 0xf, 0xa, 0xb, 5, 0xc and 0x12 (@0x81bf3-0x81c28, again
    /// @0x81c55-0x81c8a; see
    /// [`Npc::threatens_player_extended`]). The engine's goals for them:
    ///
    /// | Original goal | Set at | Engine goal |
    /// |---|---|---|
    /// | 7 (inspect) | `_InterceptorAI` @0x8ca2f, @0x8ce42, @0x8d431 | [`Goal::Inspect`] |
    /// | 9 (refuel) | `_AIMakeShipRefuelPlayer` @0x82dd7 | [`Goal::Assist`] ([`Help::Refuel`]) |
    /// | 0xf (repair) | `_AIMakeShipRepairPlayer` @0x82e0e | [`Goal::Assist`] ([`Help::Repair`]) |
    /// | 5 (fighter return) | `_AIMakeFighterReturn` @0x82e3c, `_EscortAI` @0x83e89, `_HighLevelAIHandler` @0x8d608 | [`Goal::Dock`] |
    /// | 0xa (escort fly in form) | `_AIMakeEscortFlyInForm` @0x89bb2, `_EscortAI` @0x83da3, @0x83e40, @0x83f00 | [`Goal::Follow`], [`Goal::Formation`], [`Goal::Hold`] |
    /// | 0xb (jump with parent) | `_AIMakeShipJumpWithParent` @0x82bc7 | none: an escort follows its lead out |
    /// | 0xc (a `ShipBehav` 1 mission ship with no target) | @0x89fe0, `_LowLevelAIHandler` @0x886f9 | none |
    /// | 0x12 | `_DeathStarAI` @0x8c26f | none |
    ///
    /// A plunder (goal 0xd), a drift (0xe) and a count wait (0x16) are
    /// none of them.
    #[must_use]
    pub fn spares_player_threat(self) -> bool {
        match self {
            Self::Inspect(_)
            | Self::Assist(_)
            | Self::Dock(_)
            | Self::Follow { .. }
            | Self::Formation { .. }
            | Self::Hold => true,
            Self::Idle
            | Self::Wait
            | Self::Land(_)
            | Self::EnterGate(_)
            | Self::JumpOut
            | Self::Attack(_)
            | Self::Snipe(_)
            | Self::Flee(_)
            | Self::Mine
            | Self::Plunder { .. }
            | Self::CountWait
            | Self::Drift
            | Self::Regroup { .. } => false,
        }
    }
}

/// The help an NPC flies over to give the player, once asked (see
/// [`hail`](crate::hail)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Help {
    /// It tops up the player's fuel.
    Refuel,
    /// It repairs the player's disabled ship.
    Repair,
}

/// The player's ship as an NPC sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSide {
    /// Where it is and how it moves.
    pub state: ShipState,
    /// How it is holding up.
    pub condition: Condition,
    /// Its shield, armour and fuel.
    pub reserves: Reserves,
    /// Its hull.
    pub hull: HullSpec,
    /// How it handles.
    pub handling: Handling,
    /// The pilot's combat rating (see `nova_sim::rating`).
    pub rating: i64,
    /// The raw `Strength` of `shïp` 128, the yardstick the odds measure
    /// the rating by ([`odds`](crate::ai::odds)).
    pub yardstick: i16,
    /// Whether its ship type is untargetable (`shïp` `Flags2` 0x0004,
    /// [`UNTARGETABLE`](crate::cloak::UNTARGETABLE)): an NPC's missile
    /// flies at it only with the scanner bit, and its turrets never (see
    /// [`fire`]).
    pub untargetable: bool,
    /// The governments that bless the player, as the session's last
    /// refresh took them (`nova_sim::Session::blessing`):
    /// [`Surroundings::blessed`] reads it.
    pub blessing: Blessing,
    /// Its ship type: the escape pod ([`ESCAPE_POD_SHIP`](crate::combat::hull::ESCAPE_POD_SHIP), the original's
    /// class 0x2ff) is no xenophobe's candidate (see [`target`]).
    pub ship: ShipId,
    /// The government its ship type fights as, its class's +0x9f4
    /// ([`ShipRecord::combat_govt`](crate::catalog::ShipRecord::combat_govt)),
    /// if any: the ships at war with it may turn on the player at the
    /// 1-in-50 chance (see [`target`]), and the player's friend strength
    /// counts the ships allied with it (see [`odds`]).
    pub combat_govt: Option<GovtId>,
    /// The ship it targets, the original's player +0x70, if any: the
    /// answer flag reads it ([`answer_flag`]).
    pub target: Option<ShipRef>,
    /// Its ship type's class as an escort, the original's ship class +0xe
    /// ([`EscortClass::of`]): the reaction's entry test reads whether it is
    /// a fighter ([`answer_flag`]).
    pub class: EscortClass,
    /// Whether a pirate has docked with it (+0xb9): set as a pirate starts
    /// its dock (`_LowLevelAIHandler` @0x86fc7), cleared at landing and on
    /// each frame it is not disabled (`_HandlePlayer` @0x68b02). A pirate
    /// passes a boarded player over ([`plunder_target`]) and waits or
    /// attacks one ([`pirate`]).
    pub boarded: bool,
    /// Its speed along its heading while it flies inertialess, the
    /// original's player +0x48 (`nova_sim::Session::player_speed`): an
    /// inertialess dogfighter matches it ([`pursuit`]).
    pub speed: f32,
}

/// No governments at all.
static NO_GOVERNMENTS: Governments = Governments::empty();

/// No damage aimed at any ship.
pub static NO_AIMED_DAMAGE: BTreeMap<ShipRef, i32> = BTreeMap::new();

/// What an NPC deciding can see.
#[derive(Clone, Copy, Debug)]
pub struct Surroundings<'a> {
    /// The system's stellars.
    pub sites: &'a [LandingSite],
    /// Every NPC in the system, the one deciding among them.
    pub npcs: &'a [Npc],
    /// The player's ship, while it flies in the system.
    pub player: Option<PlayerSide>,
    /// Every government and their relations.
    pub govts: &'a Governments,
    /// The system's government, or `None` when it is independent.
    pub system_govt: Option<GovtId>,
    /// The player's legal record in the system: its record with the
    /// system's government, none in an independent system.
    pub record: i16,
    /// The damage the shots in flight aim at each ship
    /// ([`Combat::targeted_damage`](crate::combat::Combat::targeted_damage)):
    /// an NPC holds its missile while it suffices (see [`fire`]).
    pub targeted: &'a BTreeMap<ShipRef, i32>,
    /// The asteroid in slot 0 of the system's field, if any: the one every
    /// miner works (`*0x3b540c`, whose +0x20 says it is live, read by
    /// `_MinerAI` @0x8b2ad-0x8b2b8; see [`miner`]).
    pub mining_rock: Option<Asteroid>,
    /// The system's reinforcement fleet while one called is on its way
    /// (its timer above 0), which doubles an allied ship's `MaxOdds` in
    /// its odds retreat ([`reinforcements::odds_factor`]).
    pub reinforcements: Option<reinforcements::Coming>,
    /// Whether the player scans for cloaked ships on screen, as
    /// `_ShipVisibleToShip` reads it for the player's sight of a ship
    /// ([`World::cloak_scanner`](crate::traffic::World::cloak_scanner)):
    /// whether the player sees a threat to it ([`Surroundings::seen_by_player`]).
    pub cloak_scanner: bool,
    /// The governments an IFF scrambler (`oütf` `ModType` 48) has marked,
    /// their byte +0x83 (see `nova_sim`'s `iff_scramblers` module): a
    /// warship of one takes no player candidate from its hunt (see
    /// [`target`]).
    pub iff_marked: &'a BTreeSet<GovtId>,
    /// Whether a fighter launched now would find a ship slot: a free berth
    /// below the spawn reserve, as `_LaunchFighter`'s
    /// `_GenericRandomShipSpawn(govt, 8)` asks (@0x3d648-0x3d662), read
    /// slot by slot: the traffic sets it before each NPC's trigger, the
    /// launches of lower berths that tick having taken theirs (see
    /// `traffic::launch_room`). With none, a goal-4 sniper's launch fails
    /// and its bay stays ready, so the snipe's own `_AILaunchFighter` makes
    /// the bay its pick after its passes (see [`fire`]).
    pub fighter_room: bool,
    /// Which hit lets a ship keep its own target
    /// ([`react::keeps_target`],
    /// [`RuleKey::HitFocus`](crate::RuleKey::HitFocus)): the original's
    /// test of the ship struck by the engine, the engine's test before
    /// weapons-fidelity phase 37 by the other reading.
    pub hit_focus: RuleSource,
    /// The heading each NPC wanted as the pass of strikes being answered
    /// began (+0x68, see `traffic::in_turn`), while one is: the focus test
    /// reads it ([`react::keeps_target`]), as +0x68 is the low level's
    /// alone and no reaction moves it. An NPC not in it wanted none, and
    /// the bearing to its target stands in. With none, the test reads
    /// each NPC's frame as it is now.
    pub wanted: Option<&'a BTreeMap<NpcId, i16>>,
    /// Who needs a jump's fuel now to jump out ([`leave::can_jump`]), by
    /// [`RuleKey::NpcJumpFuel`](crate::RuleKey::NpcJumpFuel).
    pub jump_fuel: RuleSource,
    /// Which odds the odds' readers read ([`odds::cached_odds`]): each
    /// ship's cached odds ([`Npc::odds`]) by the engine, worked out fresh
    /// by the other reading, by
    /// [`RuleKey::OddsCache`](crate::RuleKey::OddsCache).
    pub odds_cache: RuleSource,
    /// The factor on the top speed of the player and of every ship it
    /// leads ([`World::led_speed`](crate::traffic::World::led_speed)), as
    /// a swarmer's `_ShipMaxSpeed` reads it ([`swarm::lead`]).
    pub led_speed: f32,
    /// Every weapon's record, the original's global table `*0x3b517c`,
    /// which a fighter's lead reads by its pick or last gun whether or not
    /// it carries the weapon (@0x85e4c-0x85e59, @0x2b06a-0x2b075; see
    /// [`fire::dogfight_lead`]).
    pub weapons: &'a Arsenal,
    /// The ships gone from the system since the last decision pass, their
    /// records as they left, whose slots a ship's target may still name
    /// ([`gone`](crate::traffic::gone)): what a brave trader reads of a
    /// gone attacker ([`Surroundings::held_gone`]).
    pub gone: &'a [GoneShip],
    /// The player's warp stage, +0x50: above 0 from the brake of its jump
    /// on, when every ship it leads takes its jump (`_EscortAI`
    /// @0x83a41-0x83a5f, [`EscortAi`]).
    pub player_warp_stage: f32,
}

/// No government marked by an IFF scrambler.
pub static NO_IFF_MARKS: BTreeSet<GovtId> = BTreeSet::new();

/// No weapon records: a placeholder for surroundings in which nothing
/// pursues, as every lead then takes the table bearing.
pub static NO_WEAPONS: Arsenal = Arsenal::EMPTY;

impl<'a> Surroundings<'a> {
    /// `npcs` among `sites`, with no player, no governments, no record, no
    /// damage aimed, no asteroid in slot 0, no reinforcements coming, no
    /// cloak scanner, no government marked by an IFF scrambler, room for
    /// a fighter, the engine's [`hit_focus`](Surroundings::hit_focus), no
    /// pass's [`wanted`](Surroundings::wanted) headings, the engine's jump
    /// fuel and cached odds, a led speed of 1 and no weapon records
    /// ([`NO_WEAPONS`]).
    #[must_use]
    pub fn new(sites: &'a [LandingSite], npcs: &'a [Npc]) -> Self {
        Self {
            sites,
            npcs,
            player: None,
            govts: &NO_GOVERNMENTS,
            system_govt: None,
            record: 0,
            targeted: &NO_AIMED_DAMAGE,
            mining_rock: None,
            reinforcements: None,
            cloak_scanner: false,
            iff_marked: &NO_IFF_MARKS,
            fighter_room: true,
            hit_focus: RuleSource::Engine,
            wanted: None,
            jump_fuel: RuleSource::Engine,
            odds_cache: RuleSource::Engine,
            led_speed: 1.0,
            weapons: &NO_WEAPONS,
            gone: &[],
            player_warp_stage: 0.0,
        }
    }

    /// Whether the player sees `npc` (`_ShipVisibleToShip(npc, player)`):
    /// [`cloak::visible`](crate::cloak::visible) from where the player is
    /// (the centre with no player), with its
    /// [`cloak_scanner`](Surroundings::cloak_scanner).
    #[must_use]
    pub fn seen_by_player(&self, npc: &Npc) -> bool {
        let observer = crate::cloak::Observer {
            at: self
                .player
                .map_or(crate::geometry::Vec2::ZERO, |player| player.state.position),
            scanner: self.cloak_scanner,
            cloaked_escort: false,
        };
        crate::cloak::visible(npc.cloak, npc.state.position, npc.emerging(), observer)
    }

    /// Whether `npc` threatens the player among these ships, as
    /// `_ExtendedIsThreatToPlayer` @0x81b70 asks it: read whole as
    /// [`Npc::threatens_player_extended`] over [`npcs`](Surroundings::npcs),
    /// with the player's sight of it ([`Surroundings::seen_by_player`],
    /// @0x81bc6).
    #[must_use]
    pub fn threatens_player(&self, npc: &Npc) -> bool {
        npc.threatens_player_extended(self.npcs, self.seen_by_player(npc))
    }

    /// Whether `govt` is marked by an IFF scrambler: never an independent.
    #[must_use]
    pub fn iff_marked(&self, govt: Option<GovtId>) -> bool {
        govt.is_some_and(|govt| self.iff_marked.contains(&govt))
    }

    /// Whether `govt` blesses the player with peace, rank `Flags` 0x0100
    /// (`_PlayerBlessedByGovt(govt, 0)`, see `nova_sim::rank`): never
    /// for an independent, and never with no player in the system. Its
    /// readers are the AI's: liking in hails, the odds, whom a warship
    /// targets, the government's word and the Good Samaritan.
    #[must_use]
    pub fn blessed(&self, govt: Option<GovtId>) -> bool {
        self.player
            .is_some_and(|player| player.blessing.blesses(govt, Blessed::Peace))
    }

    /// The damage the shots in flight aim at `ship`; none when none is.
    #[must_use]
    pub fn aimed_at(&self, ship: ShipRef) -> i32 {
        self.targeted.get(&ship).copied().unwrap_or(0)
    }

    /// The NPCs in the original's slot order, by ship-table slot
    /// ([`Npc::berth`]), those at one slot in the order they appeared: the
    /// order the original's loops over slots 1-63 meet them, as a tie in
    /// a nearest pick and a friend strength's carried shield factor read
    /// it.
    #[must_use]
    pub fn by_slot(&self) -> Vec<&'a Npc> {
        let mut npcs: Vec<&Npc> = self.npcs.iter().collect();
        npcs.sort_by_key(|npc| npc.berth);
        npcs
    }

    /// Whether `npc` is allied with the player, as `_AlliedWithPlayer`
    /// (@0x6985-0x69d0) has it: a ship the player leads
    /// ([`Npc::leader_ref`]), or one led by such a ship here, such as a
    /// fighter an escort launched. The answer flag's blessing and the
    /// session's escort Attack read it. Combat keeps its own reading
    /// (`Sides::allied_with_player` in `nova_sim_combat`): that crate sits
    /// below this one and walks the fight's ties, not its NPCs.
    #[must_use]
    pub fn allied_with_player(&self, npc: &Npc) -> bool {
        let players = |npc: &Npc| npc.leader_ref() == Some(ShipRef::Player);
        players(npc)
            || matches!(npc.leader_ref(), Some(ShipRef::Npc(lead))
                if self.npc(lead).is_some_and(players))
    }

    /// NPC `id`, if it is in the system.
    #[must_use]
    pub fn npc(&self, id: NpcId) -> Option<&'a Npc> {
        self.npcs.iter().find(|npc| npc.id == id)
    }

    /// NPC `id`'s record as it left, if it is gone from the system since
    /// the last decision pass ([`Surroundings::gone`]).
    #[must_use]
    pub fn gone_npc(&self, id: NpcId) -> Option<&'a Npc> {
        self.gone
            .iter()
            .find(|gone| gone.ship.id == id)
            .map(|gone| &gone.ship)
    }

    /// The record of the gone ship `npc`'s target still names, no slot
    /// test having cleared it ([`GoneShip::held_by`]): what `npc`'s think
    /// reads of its target, though the ship has left.
    #[must_use]
    pub fn held_gone(&self, npc: &Npc) -> Option<&'a Npc> {
        self.gone
            .iter()
            .find(|gone| gone.held_by(npc))
            .map(|gone| &gone.ship)
    }

    /// Where `ship` is and how it moves, if it is in the system.
    #[must_use]
    pub fn state_of(&self, ship: ShipRef) -> Option<ShipState> {
        match ship {
            ShipRef::Player => self.player.map(|player| player.state),
            ShipRef::Npc(id) => self.npc(id).map(|npc| npc.state),
        }
    }

    /// How `ship` is holding up, if it is in the system.
    #[must_use]
    pub fn condition_of(&self, ship: ShipRef) -> Option<Condition> {
        match ship {
            ShipRef::Player => self.player.map(|player| player.condition),
            ShipRef::Npc(id) => self.npc(id).map(|npc| npc.condition),
        }
    }

    /// `ship`'s government: none for the player or an independent.
    #[must_use]
    pub fn govt_of(&self, ship: ShipRef) -> Option<GovtId> {
        match ship {
            ShipRef::Player => None,
            ShipRef::Npc(id) => self.npc(id).and_then(|npc| npc.govt),
        }
    }

    /// Whether `ship` is here to be fought: in the system, intact or
    /// disabled, and not still jumping in or coming out of a gate (seen by
    /// no ship while it waits there, `_ShipVisibleToShip` @0x972d, and
    /// unfought while it bursts out, as during the glide). A ship holding
    /// its course after its glide or burst ([`Mode::Coasting`](crate::traffic::npc::Mode::Coasting)) is here:
    /// its goal is 0, which `_ShipVisibleToShip` sees ([`Npc::arrived`]).
    /// So is one held at its gate ([`Mode::HeldAtGate`](crate::traffic::npc::Mode::HeldAtGate)),
    /// its goal 4.
    #[must_use]
    pub fn live(&self, ship: ShipRef) -> bool {
        let jumping_in = match ship {
            ShipRef::Player => false,
            ShipRef::Npc(id) => self.npc(id).is_some_and(|npc| !npc.arrived()),
        };
        !jumping_in && self.condition_of(ship).is_some_and(Condition::standing)
    }
}

/// How a ship answers a strike.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Reaction {
    /// The ship it now targets, if it changes.
    pub target: Option<ShipRef>,
    /// The nominal damage it adds to its provocation: each of the strike's
    /// nominal mass and energy damage above none
    /// ([`Strike::nominal`], see [`react::victim`]).
    pub provoked: f32,
    /// The goal it takes at once, if any.
    pub goal: Option<Goal>,
}

/// What a ship waiting in its gate, or held at it, holds on one tick
/// (see [`Behaviour::gate_wait`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateWait {
    /// The fire command it holds.
    pub trigger: Trigger,
    /// Whether it drops its target.
    pub drops_target: bool,
}

/// An NPC's decision ([`Behaviour::decision`]): its goal, and whether the
/// think turned it on the player.
// Each flag is its own write or call of the original's think.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    /// Its goal now.
    pub goal: Goal,
    /// Whether the original calls `_AIMakeShipAttackPlayer` in this think,
    /// whose gate says a person's attack quote at once (@0x89c50-0x89c9b,
    /// see [`Traffic::turn_with_quote`](crate::traffic::Traffic::turn_with_quote)).
    /// The per-type AIs' callers are `_SelectWarshipTarget`'s grudge and
    /// `ShipBehav` 0 tail call (@0x89f20), its xenophobe (@0x8a40e) and
    /// its hunt (@0x8ad31) ([`target::Pick`]), the government's 0x0004
    /// word in `_WarshipAI` (@0x8b7da) and `_InterceptorAI` (@0x8cb8f)
    /// ([`warship::hunt`]), and `_HighLevelAIHandler`'s mission hunt
    /// (@0x8d6b7, [`NovaAi`]). The call comes before the rest of the
    /// think, so it is made whatever goal the think ends in: an attack, a
    /// retreat, a hopeless chase, or none at all.
    pub turns_on_player: bool,
    /// Whether the think dropped its target and goal (-1, goal 0) before
    /// it turned on the player: the government's 0x0040 word
    /// (`_WarshipAI` @0x8b800, `_InterceptorAI` @0x8cbc0). The quote's gate
    /// then sees the ship with neither, and finds it no threat.
    pub dropped_target: bool,
    /// Whether the think sets its provocation to 1: the government's
    /// 0x0004 word writes +0x96 = 1 as it turns the ship on the player
    /// (`_WarshipAI` @0x8b7e5, `_InterceptorAI` @0x8cb9d). The traffic sets
    /// [`Npc::provoked`] so as it takes the goal.
    pub provokes: bool,
    /// Whether `_HighLevelAIHandler` ends the fight the think left it
    /// with, zeroing its provocation (+0x96), in the same dispatch: a
    /// flight with no target (@0x8dff6 → @0x8e228-0x8e23d), or an attack
    /// on a ship gone or breaking up (@0x8e3e6 → @0x8e714-0x8e726). The
    /// traffic zeroes [`Npc::provoked`] so as it takes the goal, before
    /// [`provokes`](Self::provokes) (see
    /// [`provocation`](crate::traffic::provocation)).
    pub forgets_provocation: bool,
    /// The call the think makes on the message line, if any: a distress
    /// call (`_AICallForHelp`) or a pre-scan hail (see
    /// [`call_for_help`]). The session answers it in slot order with the
    /// frame's other work on the line, after `_HandlePlayer`'s part of the
    /// frame (`Session::settle_line_queue`; see [`crate::traffic::on_line`]).
    pub comm_call: Option<CommCall>,
    /// Whether the think calls for its system's reinforcement fleet
    /// (`_AICallForReinforcements(ship, 0)`, see [`reinforcements`]): a
    /// fleeing brave trader's (@0x8b6dc), and `_HighLevelAIHandler`'s
    /// in state 3 and 4 ([`reinforcements::high_level_calls`]). The
    /// traffic asks it after the think's own [`comm_call`](Self::comm_call)
    /// ([`CommCall::Reinforcements`]).
    pub calls_reinforcements: bool,
    /// The ship it would plunder, when the think found another ship had
    /// boarded it: `_PirateWarshipAI`'s wait loop (@0x8c68c-0x8c730), which
    /// the traffic runs in slot order
    /// ([`board_wait::resolve`](crate::traffic::board_wait::resolve)),
    /// waiting ([`Goal::CountWait`]) when a ship boards it; otherwise the
    /// think's [`goal`](Self::goal) stands.
    pub board_wait: Option<ShipRef>,
    /// Whether the think left the ship with no target (+0x70 = -1) though
    /// its goal is no fight: `_PirateWarshipAI`'s search (@0x8c35d) finding
    /// nothing. The traffic clears [`Npc::target`] as it takes the goal; a
    /// goal it keeps with a target keeps the target.
    pub clears_target: bool,
    /// Whether the think dropped its fight leaving its sub-goal as it was:
    /// `_PirateWarshipAI`'s disabled-warship drop writes the goal and the
    /// target alone (@0x8c839-0x8c842), so a kept rush or break-off
    /// ([`Npc::pursuit`]) carries over to its next fight.
    pub keeps_pursuit: bool,
    /// The ship it plunders now: `_PirateWarshipAI`'s plunder
    /// (@0x8c74c-0x8c7ae), a pirate docked (goal 4, sub 0xf) with its wait
    /// out, its [`goal`](Self::goal) the drift that follows
    /// ([`Goal::Drift`]). The traffic runs it after the goals, in berth
    /// order ([`plunder::resolve`](crate::traffic::plunder::resolve)), and
    /// the wait loop drops it when it makes the pirate wait instead.
    pub plunders: Option<ShipRef>,
    /// `_HighLevelAIHandler`'s lost-sight block's write of the search
    /// timer, when it ran ([`lost_sight::after_think`]); none otherwise,
    /// which leaves [`Npc::lost_sight`] as it was.
    pub lost_sight: Option<lost_sight::LostSight>,
    /// How the think writes its travel stellar, the original's +0x6c (see
    /// [`travel`]): the traffic [`take`](travel::take)s it into
    /// [`Npc::travel_set`] as it takes the goal.
    pub travel: travel::TravelWrite,
    /// The stellar the think records as visited, its +0x92, if it writes
    /// one: a miner's shuttle as it sets off (`_MinerAI` @0x8b3ae, see
    /// [`miner`]). The traffic sets [`Npc::visited`] so as it takes the
    /// goal; none leaves it as it was.
    pub visits: Option<StellarId>,
    /// Whether an inspecting interceptor's re-pick gave its goal
    /// (`_InterceptorAI` @0x8d41e, see [`interceptor`]): the state-7
    /// block's reach then reads the new target (@0x8f11e), and a state-4
    /// pick runs no reach at all, so the ship it was inspecting is not
    /// looked over and the player is not scanned.
    pub re_picked: bool,
    /// Whether the think ends its AI wait (+0x4c = -1.0): `_EscortAI`'s
    /// Defend, Attack and formation blocks write it (@0x83c6b-0x83c73,
    /// @0x83d62-0x83d6a, @0x83ea8-0x83eb0) whoever the parent, an NPC
    /// ([`fleet_escort`]) or the player ([`EscortAi`]), so a ship a pirate
    /// took into its fleet ends its
    /// [`CAPTURED_WAIT`](plunder_contents::CAPTURED_WAIT) at its first
    /// think. The traffic zeroes [`Npc::ai_wait`] as it takes the goal, and
    /// gives up its warp stage, as the blocks write +0x50 = -1.0 with it
    /// (see [`jump_with_leader`](crate::traffic::jump_with_leader)); the
    /// high level's hold zeroes the stage again in the same tick.
    pub ends_wait: bool,
    /// The order word the think writes to the ship, if any: a fighter of
    /// the player's escort writes the order it takes from its parent
    /// (`_EscortAI` @0x83b76, @0x83b7f; [`fleet_escort`]), which its next
    /// target check, and a stray beam, then read. The traffic sets
    /// [`Npc::order_word`] as it takes the goal; none leaves it as it was.
    pub order_word: Option<crate::fleet::OrderWord>,
    /// Whether the think takes the ship out of its fleet: `_EscortAI`'s
    /// lost parent (@0x83917-0x8397b, its AI type @0x83c4a), for a ship
    /// whose NPC lead is gone or not intact ([`fleet_escort`]). The traffic
    /// runs [`Npc::leave_fleet`] as it takes the goal, and gives up its
    /// warp stage, as the branch writes +0x50 = -1.0 (@0x83974).
    pub leaves_fleet: bool,
    /// Whether the think puts the ship on its leader's jump: `_EscortAI`'s
    /// entry into goal 0xb (@0x839fd-0x83a92, @0x83be3-0x83c51), for an
    /// NPC's escort whose lead draws it ([`fleet_escort`]) and for every
    /// ship the player leads from the player's brake on ([`EscortAi`]).
    /// The traffic resolves it right after taking the goal: the ship
    /// follows the jump, or an inertialess NPC's escort detaches
    /// ([`jump_with_leader`](crate::traffic::jump_with_leader)).
    pub follows_jump: bool,
}

/// How NPCs decide what to do.
pub trait Behaviour: Debug {
    /// `npc`'s goal now, among `around`, rolling any choice on `chance`.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal;

    /// `npc`'s whole decision now ([`Decision`]), among `around`, rolling
    /// any choice on `chance`: by default its [`decide`](Self::decide)
    /// goal, turning on nobody.
    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        Decision {
            goal: self.decide(npc, around, chance),
            ..Decision::default()
        }
    }

    /// The fire command `npc` holds now, among `around`: none by default.
    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        let _ = (npc, around);
        Trigger::default()
    }

    /// The ship `npc` targets now, among `around`: none by default.
    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        let _ = (npc, around);
        None
    }

    /// What `npc` holds on a tick it waits in its gate or is held at it,
    /// among `around`, in place of its [`trigger`](Self::trigger):
    /// `_LowLevelAIHandler` at sub-goal 0 calls `_AIEscortFireUnprovoked`
    /// every frame (@0x8521e-0x85236), which does nothing for a ship of AI
    /// type 4 or below (@0x80e59). By default it fires nothing and keeps
    /// its target; the escorts' and carried fighters' AIs run
    /// [`escort::fire_unprovoked`].
    fn gate_wait(&self, npc: &Npc, around: &Surroundings) -> GateWait {
        let _ = (npc, around);
        GateWait::default()
    }

    /// How `npc` answers `strike`, a hit on itself or another ship, among
    /// `around`: not at all by default.
    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        let _ = (npc, strike, around);
        Reaction::default()
    }
}

/// Nova's peaceful traffic (see [`idle`]): it never fires, targets
/// nothing and ignores hits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Peaceful;

impl Behaviour for Peaceful {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        idle(npc, around, chance)
    }
}

/// The idle block the AI types share (e.g. `_WarshipAI` @0x8b06c in the
/// `EV Nova` executable; see [`idle_block`]), picking as their (ship, 0, 0)
/// call does ([`Admit::All`], see
/// [`destination`](crate::traffic::destination)):
///
/// - An escort follows its lead while the lead is in the system; once it
///   is gone, the escort decides as below.
/// - A goal it can still fly is kept: a stellar that is still there and
///   landable, a gate it enters that is still there, a jump out, a hold,
///   or a wait. A leaving ship (state 2) keeps leaving whether or not it
///   can jump: no type's idle block tests it again (`_WimpyTraderAI`
///   @0x8b06c, `_BraveTraderAI` @0x8b4d6 and `_InterceptorAI` @0x8d010
///   pick only in state 0); the warship's and the pirate's own re-tests
///   are theirs ([`warship`], [`pirate`]). A waiting ship ([`Goal::Wait`],
///   state 6) keeps waiting, as those idle blocks need state 0; the
///   warship's state 6 is its own ([`wait`]).
/// - A ship that has visited a stellar of the system, or is in a system
///   with none ([`idle_block::visited_here`], @0x8b0a3-0x8b0c8), skips the
///   pick. One that hasn't heads for a stellar picked at random among
///   `_SelectRandomStellarDest`'s candidates (@0x805de): landable, within
///   999 of the centre on each axis, not uninhabited unless a gate, not
///   of a government at war with its own, the gates chosen by its
///   government's `Flags2`. The pick is one `below(candidates)` draw in
///   the system's stellar order: what the original's `Rand(16)` loop
///   decides, a uniform pick among them. A hypergate or wormhole picked
///   is entered ([`Goal::EnterGate`]), the ship leaving the system through
///   it. A ship visits a stellar coming out of its gate
///   ([`Npc::gate_visit`]), so it leaves on its first idle think; landing
///   writes the visit too (`_HighLevelAIHandler` @0x8db00), which is
///   rdm:phase/stellar-fidelity/phase-5-npc-landing-reach's, and until
///   then a ship that lands leaves the system.
/// - A held ship ([`Goal::Hold`], the original's goal 10 with no
///   leader) keeps its hold and picks nothing: every type's destination
///   pick needs goal 0 (`_WimpyTraderAI` @0x8b06c, `_BraveTraderAI`
///   @0x8b4d6), and `_AICheckParentExists` (@0x8926a-0x89281) never
///   clears the goal of a ship with no leader. Only provocation, or a
///   hunter's government's word, lifts it.
/// - With no pick, or once visited, it jumps out if it can
///   ([`leave::can_jump`]: its class's `Fuel` above 99, held in no other
///   ship's tractor beam, and a jump's fuel for a person that starts with
///   zero fuel), and otherwise stops and waits ([`Goal::Wait`]), as the
///   idle blocks' inlined `_AIShipHasFuelForJump` sends it to state 6
///   (`_WimpyTraderAI` @0x8b11d-0x8b1a3): [`idle_block::leave_or_wait`].
///   Any other ship jumps with an empty tank.
pub fn idle(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    if let Some(kept) = kept(npc, around) {
        return kept;
    }
    if idle_block::visited_here(npc, around) {
        return idle_block::leave_or_wait(npc, around);
    }
    random_stellar(npc, around, chance, Admit::All)
        .unwrap_or_else(|| idle_block::leave_or_wait(npc, around))
}

/// A stellar picked at random for `npc` among `around`'s, admitting
/// `admit`, as `_SelectRandomStellarDest` picks (@0x805de; see
/// [`idle`]): one `below(candidates)` draw, a hypergate or
/// wormhole entered ([`Goal::EnterGate`]), any other landed on. `None`,
/// drawing nothing, when there is no candidate.
pub(crate) fn random_stellar(
    npc: &Npc,
    around: &Surroundings,
    chance: &mut dyn Chance,
    admit: Admit,
) -> Option<Goal> {
    let candidates = destination::candidates(npc.govt, around.sites, around.govts, admit);
    if candidates.is_empty() {
        return None;
    }
    let len = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let pick = chance.below(len) as usize;
    candidates.get(pick).map(|site| {
        if GateKind::of(site.flags2).is_some() {
            Goal::EnterGate(site.id)
        } else {
            Goal::Land(site.id)
        }
    })
}

/// What `npc` keeps to among `around` before its idle block runs, if
/// anything: an escort follows its lead while the lead is in the system,
/// guarding nothing, and a goal it can still fly is kept. A led ship
/// reaches its type's idle block only by the other reading of
/// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi), the port's
/// earlier routing; by the engine's it flies [`FleetEscortAi`] instead.
pub(crate) fn kept(npc: &Npc, around: &Surroundings) -> Option<Goal> {
    if let Some(lead) = npc.leader
        && around.npc(lead).is_some()
    {
        return Some(Goal::Follow { lead, guard: None });
    }
    can_still_fly(npc, around).then_some(npc.goal)
}

/// The ship that provoked `npc`, its target since the hit, while it is
/// provoked and that ship is live among `around`.
#[must_use]
pub fn provoked_by(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    npc.target
        .filter(|&attacker| npc.provoked > 0.0 && around.live(attacker))
}

/// Whether `npc` can still fly its goal among `around`: a stellar still
/// there and landable, a gate still there (state 20 tests no landability
/// again), a jump out (state 2 tests no fuel), a hold, or a drift (goal
/// 0xe, which its high level ends: the trader AIs' idle blocks run only on
/// goal 0, `_WimpyTraderAI` @0x8b06c, `_BraveTraderAI` @0x8b4d6). Idling,
/// following once the lead is gone, fighting and inspecting are no goals
/// to keep.
fn can_still_fly(npc: &Npc, around: &Surroundings) -> bool {
    match npc.goal {
        Goal::Land(stellar) => around
            .sites
            .iter()
            .any(|site| site.id == stellar && landable(site)),
        Goal::EnterGate(gate) => around.sites.iter().any(|site| site.id == gate),
        Goal::JumpOut | Goal::Hold | Goal::Wait | Goal::Drift => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::landing::StellarFlags;
    use crate::stats::ShipStats;
    use crate::traffic::npc::{AiType, Mode};
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::planet;

    fn npc(id: u32, ai_type: AiType) -> Npc {
        Npc {
            ai_type,
            ..crate::traffic::fixture::npc(id, ShipStats::new(FAST, &[]))
        }
    }

    /// Planets 128 and 130 can be landed on; 129 cannot, and 131 only once
    /// destroyed.
    fn sites() -> Vec<LandingSite> {
        let mut sites: Vec<_> = (128..=131).map(|id| planet(id, 0.0, 0.0)).collect();
        sites[1].flags = 0;
        sites[3].flags |= StellarFlags::ONLY_WHEN_DESTROYED;
        sites
    }

    fn decide(npc: &Npc, sites: &[LandingSite], npcs: &[Npc], chance: &mut Draws) -> Goal {
        Peaceful.decide(npc, &Surroundings::new(sites, npcs), chance)
    }

    const TRADERS: [AiType; 2] = [AiType::WimpyTrader, AiType::BraveTrader];
    const FIGHTERS: [AiType; 2] = [AiType::Warship, AiType::Interceptor];

    #[test]
    fn no_goal_scoops_until_the_miners_goal() {
        let ship = ShipRef::Npc(NpcId(2));
        for goal in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(129)),
            Goal::JumpOut,
            Goal::Follow {
                lead: NpcId(2),
                guard: None,
            },
            Goal::Attack(ship),
            Goal::Snipe(ship),
            Goal::Flee(ship),
            Goal::Inspect(ship),
            Goal::Assist(Help::Refuel),
            Goal::Formation { guard: None },
            Goal::Dock(ShipRef::Player),
            Goal::Hold,
            Goal::Mine,
        ] {
            assert!(!goal.scooping(), "{goal:?}");
        }
        assert!(!npc(1, AiType::WimpyTrader).is_scooping());
    }

    #[test]
    fn threatens_player_asks_the_extended_test_among_around_with_the_players_sight() {
        let mut escort = npc(1, AiType::Warship);
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot::default(),
            order: None,
        });
        let mut at_escort = npc(2, AiType::Warship);
        at_escort.goal = Goal::Attack(ShipRef::Npc(NpcId(1)));
        let led = [escort, at_escort.clone()];
        assert!(Surroundings::new(&[], &led).threatens_player(&at_escort));
        assert!(
            !Surroundings::new(&[], std::slice::from_ref(&at_escort)).threatens_player(&at_escort),
            "the ship it attacks is not among around"
        );
        at_escort.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        let unseen = [led[0].clone(), at_escort.clone()];
        assert!(
            !Surroundings::new(&[], &unseen).threatens_player(&at_escort),
            "the player can't see it"
        );
        let scanned = Surroundings {
            cloak_scanner: true,
            ..Surroundings::new(&[], &unseen)
        };
        assert!(scanned.threatens_player(&at_escort));
    }

    #[test]
    fn a_miner_mining_fights_nothing_and_is_not_spared_as_a_threat() {
        assert_eq!(Goal::Mine.quarry(), None);
        assert!(!Goal::Mine.fights());
        assert!(!Goal::Mine.spares_player_threat(), "16 isn't in the list");
    }

    #[test]
    fn every_ai_type_lands_on_a_landable_stellar_picked_at_random() {
        for ai_type in TRADERS.into_iter().chain(FIGHTERS) {
            for (draw, stellar) in [(0, 128), (1, 130)] {
                let mut chance = Draws::of(&[draw]);
                let goal = decide(&npc(1, ai_type), &sites(), &[], &mut chance);
                assert_eq!(goal, Goal::Land(StellarId(stellar)), "{ai_type:?}");
                assert_eq!(chance.asked, [2], "among the two landable");
            }
        }
    }

    #[test]
    fn a_ship_never_lands_on_a_destroyed_stellar_unless_landable_only_once_destroyed() {
        let mut sites = sites();
        sites[0].destroyed = true;
        sites[3].destroyed = true;
        let mut chance = Draws::of(&[1]);
        assert_eq!(
            decide(&npc(1, AiType::WimpyTrader), &sites, &[], &mut chance),
            Goal::Land(StellarId(131))
        );
        assert_eq!(chance.asked, [2], "130 and 131");
    }

    #[test]
    fn a_ship_bound_for_a_stellar_destroyed_since_decides_again() {
        let mut bound = npc(1, AiType::WimpyTrader);
        bound.goal = Goal::Land(StellarId(128));
        let mut sites = sites();
        let mut chance = Draws::of(&[]);
        assert_eq!(
            decide(&bound, &sites, &[], &mut chance),
            Goal::Land(StellarId(128)),
            "kept"
        );
        sites[0].destroyed = true;
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            decide(&bound, &sites, &[], &mut chance),
            Goal::Land(StellarId(130))
        );
        assert_eq!(chance.asked, [1], "only 130 is left");
    }

    #[test]
    fn a_ship_with_nowhere_to_land_jumps_out() {
        let unlandable = &sites()[1..2];
        for ai_type in TRADERS.into_iter().chain(FIGHTERS) {
            let mut chance = Draws::of(&[]);
            assert_eq!(
                decide(&npc(1, ai_type), &[], &[], &mut chance),
                Goal::JumpOut
            );
            assert_eq!(
                decide(&npc(1, ai_type), unlandable, &[], &mut chance),
                Goal::JumpOut
            );
            assert!(chance.asked.is_empty());
        }
    }

    #[test]
    fn a_ship_never_lands_on_a_stellar_of_an_enemy_government() {
        // 128 is the enemy's, 130 an ally's, and the ship is of govt 140.
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(140)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(141)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(142)
            },
        ]);
        let mut sites = sites();
        sites[0].govt = Some(GovtId(141));
        sites[2].govt = Some(GovtId(142));
        let mut ship = npc(1, AiType::Warship);
        ship.govt = Some(GovtId(140));
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&sites, &[])
        };
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            Peaceful.decide(&ship, &around, &mut chance),
            Goal::Land(StellarId(130))
        );
        assert_eq!(chance.asked, [1], "only one welcomes it");
        sites[2].govt = Some(GovtId(141));
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&sites, &[])
        };
        let mut chance = Draws::of(&[]);
        assert_eq!(Peaceful.decide(&ship, &around, &mut chance), Goal::JumpOut);
    }

    #[test]
    fn an_idle_pick_skips_a_far_an_uninhabited_and_an_enemys_stellar() {
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(140)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..nova_sim_catalog::fixture::govt(141)
            },
        ]);
        let far = planet(128, 1000.0, 0.0);
        let empty = LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..planet(129, 0.0, 0.0)
        };
        let hostile = LandingSite {
            govt: Some(GovtId(141)),
            ..planet(130, 0.0, 0.0)
        };
        let sites = [far, empty, hostile, planet(131, -2000.0, 0.0)];
        let mut ship = npc(1, AiType::WimpyTrader);
        ship.govt = Some(GovtId(140));
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&sites, &[])
        };
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle(&ship, &around, &mut chance),
            Goal::Land(StellarId(131))
        );
        assert_eq!(chance.asked, [1], "only the ok one");
    }

    #[test]
    fn an_independent_ships_idle_pick_counts_a_stellar_of_any_government() {
        let govts = Governments::new([GovtRecord {
            flags: crate::govt::XENOPHOBIC,
            ..nova_sim_catalog::fixture::govt(141)
        }]);
        let sites = [LandingSite {
            govt: Some(GovtId(141)),
            ..planet(128, 0.0, 0.0)
        }];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&sites, &[])
        };
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle(&npc(1, AiType::Warship), &around, &mut chance),
            Goal::Land(StellarId(128)),
            "a xenophobe's stellar is no enemy's to a ship of none"
        );
        assert_eq!(chance.asked, [1]);
    }

    #[test]
    fn a_plain_only_pick_never_lands_on_a_gate() {
        let gate = LandingSite {
            flags2: crate::gate::HYPERGATE,
            ..planet(129, 0.0, 0.0)
        };
        let sites = [gate, planet(128, 0.0, 0.0)];
        let around = Surroundings::new(&sites, &[]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle_block::patrol(&npc(1, AiType::Interceptor), &around, &mut chance, true),
            Goal::Land(StellarId(128))
        );
        assert_eq!(chance.asked, [1]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle(&npc(1, AiType::Interceptor), &around, &mut chance),
            Goal::EnterGate(StellarId(129)),
            "the idle blocks admit the gate"
        );
        assert_eq!(chance.asked, [2]);
    }

    #[test]
    fn an_idle_pick_of_a_gate_enters_it_and_keeps_to_it_while_it_is_shown() {
        let wormhole = LandingSite {
            flags2: crate::gate::WORMHOLE,
            ..planet(130, 0.0, 0.0)
        };
        let govts = Governments::new([GovtRecord {
            flags2: crate::govt::PREFERS_WORMHOLES,
            ..nova_sim_catalog::fixture::govt(141)
        }]);
        let sites = [planet(128, 0.0, 0.0), wormhole];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&sites, &[])
        };
        let mut ship = npc(1, AiType::WimpyTrader);
        ship.govt = Some(GovtId(141));
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle(&ship, &around, &mut chance),
            Goal::EnterGate(StellarId(130))
        );
        assert_eq!(chance.asked, [1], "the wormhole alone");
        ship.goal = Goal::EnterGate(StellarId(130));
        let mut chance = Draws::of(&[]);
        assert_eq!(idle(&ship, &around, &mut chance), ship.goal, "kept");
        assert!(chance.asked.is_empty());
        let unlandable = LandingSite {
            flags: 0,
            ..sites[1]
        };
        let still_there = [sites[0], unlandable];
        let around = Surroundings {
            govts: &govts,
            ..Surroundings::new(&still_there, &[])
        };
        assert_eq!(
            idle(&ship, &around, &mut Draws::of(&[])),
            ship.goal,
            "state 20 tests no landability again"
        );
        let gone = [sites[0]];
        let around = Surroundings::new(&gone, &[]);
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle(&ship, &around, &mut chance),
            Goal::Land(StellarId(128)),
            "decided again"
        );
    }

    /// `npc` flown by a person that starts with zero fuel.
    fn zero_fuelled(mut npc: Npc) -> Npc {
        npc.person = Some(crate::traffic::npc::NpcPerson {
            zero_fuel: true,
            ..crate::traffic::npc::NpcPerson::of(
                &nova_sim_catalog::fixture::person(151, 128),
                false,
            )
        });
        npc
    }

    #[test]
    fn a_ship_with_nowhere_to_land_jumps_with_an_empty_tank_by_its_classs_fuel() {
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            let mut dry = npc(1, ai_type);
            dry.class_fuel = 300;
            dry.reserves.fuel.now = 0.0;
            assert_eq!(decide(&dry, &[], &[], &mut Draws::of(&[])), Goal::JumpOut);
            dry.class_fuel = 99;
            dry.reserves.fuel.now = 300.0;
            assert_eq!(decide(&dry, &[], &[], &mut Draws::of(&[])), Goal::Wait);
        }
    }

    #[test]
    fn a_person_that_starts_with_zero_fuel_jumps_with_a_jumps_fuel_and_waits_without() {
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            let mut short = zero_fuelled(npc(1, ai_type));
            short.reserves.fuel.now = 99.9;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::Wait);
            short.reserves.fuel.now = 100.0;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::JumpOut);
        }
    }

    #[test]
    fn by_the_other_reading_a_ship_without_a_jumps_fuel_that_would_jump_waits() {
        let mut short = npc(1, AiType::WimpyTrader);
        short.reserves.fuel.now = 99.9;
        let npcs = [short.clone()];
        let around = Surroundings {
            jump_fuel: RuleSource::Bible,
            ..Surroundings::new(&[], &npcs)
        };
        assert_eq!(idle(&short, &around, &mut Draws::of(&[])), Goal::Wait);
        short.reserves.fuel.now = 100.0;
        assert_eq!(idle(&short, &around, &mut Draws::of(&[])), Goal::JumpOut);
    }

    #[test]
    fn a_ship_without_a_jumps_fuel_that_would_jump_waits() {
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            let mut short = npc(1, ai_type);
            short.class_fuel = 99;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::Wait);
            short.class_fuel = 100;
            short.reserves.fuel.now = 0.0;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::JumpOut);
        }
        // A trader with somewhere to land needs no fuel.
        let mut dry = npc(1, AiType::BraveTrader);
        dry.reserves.fuel.now = 0.0;
        assert_eq!(
            decide(&dry, &sites(), &[], &mut Draws::of(&[0])),
            Goal::Land(StellarId(128))
        );
    }

    #[test]
    fn an_escort_follows_its_lead_and_once_it_is_gone_decides_by_its_own_ai_type() {
        let lead = npc(1, AiType::WimpyTrader);
        for ai_type in [AiType::Warship, AiType::Interceptor, AiType::BraveTrader] {
            let alone = Goal::Land(StellarId(128));
            let mut escort = npc(2, ai_type);
            escort.leader = Some(NpcId(1));
            let together = [lead.clone(), escort.clone()];
            let mut chance = Draws::of(&[0]);
            assert_eq!(
                decide(&escort, &sites(), &together, &mut chance),
                Goal::Follow {
                    lead: NpcId(1),
                    guard: None
                }
            );
            assert!(chance.asked.is_empty());
            escort.goal = Goal::Follow {
                lead: NpcId(1),
                guard: None,
            };
            assert_eq!(
                decide(&escort, &sites(), &[escort.clone()], &mut chance),
                alone,
                "{ai_type:?} without its lead"
            );
        }
    }

    #[test]
    fn peaceful_traffic_targets_nothing() {
        let npcs = [npc(1, AiType::Warship), npc(2, AiType::Interceptor)];
        let sites = sites();
        let around = Surroundings::new(&sites, &npcs);
        for npc in &npcs {
            assert_eq!(Peaceful.target(npc, &around), None);
        }
    }

    #[test]
    fn peaceful_traffic_ignores_hits() {
        let npcs = [npc(1, AiType::Warship)];
        let around = Surroundings::new(&[], &npcs);
        let strike = Strike {
            damage: 30.0,
            ..crate::combat::fixture::strike(ShipRef::Npc(NpcId(1)), ShipRef::Player)
        };
        assert_eq!(
            Peaceful.react(&npcs[0], &strike, &around),
            Reaction::default()
        );
    }

    #[test]
    fn peaceful_traffic_waiting_in_its_gate_fires_nothing_and_keeps_its_target() {
        let mut waiting = npc(1, AiType::Warship);
        waiting.mode = Mode::Emerging { ticks_left: 30 };
        waiting.target = Some(ShipRef::Player);
        let around = Surroundings::new(&[], &[]);
        assert_eq!(Peaceful.gate_wait(&waiting, &around), GateWait::default());
        assert_eq!(
            GateWait::default(),
            GateWait {
                trigger: Trigger::default(),
                drops_target: false,
            }
        );
    }

    #[test]
    fn peaceful_traffic_never_fires() {
        let sites = sites();
        let around = Surroundings::new(&sites, &[]);
        for ai_type in TRADERS.into_iter().chain(FIGHTERS) {
            assert_eq!(
                Peaceful.trigger(&npc(1, ai_type), &around),
                Trigger::default()
            );
        }
    }

    #[test]
    fn a_goal_it_can_still_fly_is_kept() {
        let mut lander = npc(1, AiType::WimpyTrader);
        lander.goal = Goal::Land(StellarId(130));
        let mut chance = Draws::of(&[]);
        assert_eq!(decide(&lander, &sites(), &[], &mut chance), lander.goal);
        assert!(chance.asked.is_empty(), "no new pick");
        let mut jumper = npc(1, AiType::Warship);
        jumper.goal = Goal::JumpOut;
        assert_eq!(decide(&jumper, &sites(), &[], &mut chance), Goal::JumpOut);
        // A trader that settled on jumping keeps to it.
        let mut leaving = npc(1, AiType::BraveTrader);
        leaving.goal = Goal::JumpOut;
        assert_eq!(decide(&leaving, &sites(), &[], &mut chance), Goal::JumpOut);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn a_leaving_ship_that_cannot_jump_keeps_leaving_as_no_idle_block_tests_state_2() {
        // `_WimpyTraderAI`, `_BraveTraderAI` and `_InterceptorAI` pick
        // only in state 0 (@0x8b06c, @0x8b4d6, @0x8d010).
        let deciders: [(AiType, &dyn Behaviour); 3] = [
            (AiType::WimpyTrader, &WimpyTrader::default()),
            (AiType::BraveTrader, &BraveTrader::default()),
            (AiType::Interceptor, &Interceptor::default()),
        ];
        for (ai_type, decider) in deciders {
            let mut stranded = zero_fuelled(npc(1, ai_type));
            stranded.goal = Goal::JumpOut;
            stranded.reserves.fuel.now = 0.0;
            let mut chance = Draws::of(&[]);
            for sites in [sites(), Vec::new()] {
                let npcs = [stranded.clone()];
                let around = Surroundings::new(&sites, &npcs);
                assert_eq!(
                    decider.decide(&stranded, &around, &mut chance),
                    Goal::JumpOut,
                    "{decider:?}"
                );
            }
            assert!(chance.asked.is_empty(), "no new pick");
        }
    }

    #[test]
    fn a_held_ship_of_every_ai_type_keeps_its_hold_drawing_nothing() {
        let deciders: [(AiType, &dyn Behaviour); 5] = [
            (AiType::WimpyTrader, &Peaceful),
            (AiType::WimpyTrader, &WimpyTrader::default()),
            (AiType::BraveTrader, &BraveTrader::default()),
            (AiType::Warship, &Warship::default()),
            (AiType::Interceptor, &Interceptor::default()),
        ];
        let sites = sites();
        for (ai_type, decider) in deciders {
            let mut held = npc(1, ai_type);
            held.goal = Goal::Hold;
            let npcs = [held.clone()];
            let mut chance = Draws::of(&[]);
            let goal = decider.decide(&held, &Surroundings::new(&sites, &npcs), &mut chance);
            assert_eq!(goal, Goal::Hold, "{decider:?}");
            assert!(chance.asked.is_empty(), "{decider:?} drew");
        }
    }

    #[test]
    fn a_waiting_ship_of_every_idle_type_keeps_waiting_drawing_nothing() {
        // Their idle blocks need state 0 (@0x8b06c, @0x8b4d6).
        let deciders: [(AiType, &dyn Behaviour); 3] = [
            (AiType::WimpyTrader, &Peaceful),
            (AiType::WimpyTrader, &WimpyTrader::default()),
            (AiType::BraveTrader, &BraveTrader::default()),
        ];
        let sites = sites();
        for (ai_type, decider) in deciders {
            let mut waiting = npc(1, ai_type);
            waiting.goal = Goal::Wait;
            let npcs = [waiting.clone()];
            let mut chance = Draws::of(&[]);
            let goal = decider.decide(&waiting, &Surroundings::new(&sites, &npcs), &mut chance);
            assert_eq!(goal, Goal::Wait, "{decider:?}");
            assert!(chance.asked.is_empty(), "{decider:?} drew");
        }
    }

    #[test]
    fn a_drifting_trader_keeps_its_drift_drawing_nothing() {
        // A repairer's drift after its repair (goal 0xe); their idle
        // blocks need state 0 (@0x8b06c, @0x8b4d6).
        let deciders: [(AiType, &dyn Behaviour); 3] = [
            (AiType::WimpyTrader, &Peaceful),
            (AiType::WimpyTrader, &WimpyTrader::default()),
            (AiType::BraveTrader, &BraveTrader::default()),
        ];
        let sites = sites();
        for (ai_type, decider) in deciders {
            let mut drifting = npc(1, ai_type);
            drifting.goal = Goal::Drift;
            drifting.ai_wait = 50;
            let npcs = [drifting.clone()];
            let mut chance = Draws::of(&[]);
            let goal = decider.decide(&drifting, &Surroundings::new(&sites, &npcs), &mut chance);
            assert_eq!(goal, Goal::Drift, "{decider:?}");
            assert!(chance.asked.is_empty(), "{decider:?} drew");
        }
    }

    #[test]
    fn a_goal_it_cannot_fly_is_decided_again() {
        let mut lander = npc(1, AiType::WimpyTrader);
        for gone in [129, 131, 140] {
            lander.goal = Goal::Land(StellarId(gone));
            let mut chance = Draws::of(&[1]);
            assert_eq!(
                decide(&lander, &sites(), &[], &mut chance),
                Goal::Land(StellarId(130)),
                "{gone}"
            );
        }
        for goal in [
            Goal::Follow {
                lead: NpcId(7),
                guard: None,
            },
            Goal::Idle,
            Goal::Attack(ShipRef::Player),
            Goal::Snipe(ShipRef::Player),
            Goal::Flee(ShipRef::Player),
            Goal::Inspect(ShipRef::Player),
        ] {
            let mut ship = npc(1, AiType::Warship);
            ship.goal = goal;
            assert_eq!(
                decide(&ship, &[], &[], &mut Draws::of(&[])),
                Goal::JumpOut,
                "{goal:?}"
            );
        }
    }

    const P: ShipRef = ShipRef::Player;

    #[test]
    fn a_ship_is_provoked_by_its_live_target_while_it_holds_any_provocation() {
        let attacker = npc(2, AiType::Warship);
        let mut victim = npc(1, AiType::WimpyTrader);
        victim.target = Some(ShipRef::Npc(NpcId(2)));
        let npcs = [victim.clone(), attacker];
        let around = Surroundings::new(&[], &npcs);
        assert_eq!(provoked_by(&victim, &around), None, "unprovoked");
        victim.provoked = 0.5;
        assert_eq!(provoked_by(&victim, &around), Some(ShipRef::Npc(NpcId(2))));
        victim.target = Some(ShipRef::Npc(NpcId(9)));
        assert_eq!(provoked_by(&victim, &around), None, "gone");
        victim.target = None;
        assert_eq!(provoked_by(&victim, &around), None);
    }

    #[test]
    fn a_goal_fights_a_ship_attacking_sniping_or_fleeing() {
        let n = ShipRef::Npc(NpcId(3));
        assert_eq!(Goal::Attack(n).quarry(), Some(n));
        assert_eq!(Goal::Snipe(P).quarry(), Some(P));
        assert_eq!(Goal::Flee(n).quarry(), Some(n));
        assert_eq!(Goal::Attack(n).attacking(), Some(n));
        assert_eq!(Goal::Snipe(P).attacking(), Some(P));
        assert_eq!(Goal::Flee(n).attacking(), None);
        assert_eq!(Goal::Inspect(n).inspecting(), Some(n));
        assert_eq!(Goal::Attack(n).inspecting(), None);
        for idle in [
            Goal::Idle,
            Goal::Land(StellarId(128)),
            Goal::EnterGate(StellarId(128)),
            Goal::JumpOut,
            Goal::Follow {
                lead: NpcId(1),
                guard: None,
            },
            Goal::Inspect(n),
            Goal::Assist(Help::Refuel),
            Goal::Assist(Help::Repair),
            Goal::Formation { guard: Some(n) },
            Goal::Dock(P),
            Goal::Dock(n),
        ] {
            assert_eq!((idle.quarry(), idle.attacking()), (None, None), "{idle:?}");
            assert!(!idle.fights());
        }
        assert!(Goal::Flee(n).fights());
    }

    #[test]
    fn a_plunder_fights_its_victim_without_attacking_it() {
        let n = ShipRef::Npc(NpcId(3));
        for hopeless in [false, true] {
            let plunder = Goal::Plunder { ship: n, hopeless };
            assert_eq!(plunder.quarry(), Some(n), "a target to fight");
            assert!(plunder.fights());
            assert_eq!(plunder.attacking(), None, "goal 0xd is not goal 4");
            assert_eq!(plunder.plundering(), Some(n));
            assert!(!plunder.scooping());
            assert!(!plunder.spares_player_threat(), "0xd isn't in the list");
        }
        for other in [Goal::Attack(n), Goal::Snipe(n), Goal::Flee(n), Goal::Idle] {
            assert_eq!(other.plundering(), None, "{other:?}");
        }
    }

    #[test]
    fn a_count_wait_fights_nothing_and_is_not_spared_as_a_threat() {
        let wait = Goal::CountWait;
        assert_eq!((wait.quarry(), wait.attacking()), (None, None));
        assert!(!wait.fights());
        assert_eq!(wait.plundering(), None);
        assert!(!wait.scooping());
        assert!(!wait.spares_player_threat(), "0x16 isn't in the list");
    }

    #[test]
    fn a_wait_fights_nothing_scoops_nothing_and_is_not_spared_as_a_threat() {
        let wait = Goal::Wait;
        assert_eq!((wait.quarry(), wait.attacking()), (None, None));
        assert!(!wait.fights());
        assert_eq!((wait.plundering(), wait.inspecting()), (None, None));
        assert!(!wait.scooping());
        assert!(!wait.spares_player_threat(), "6 isn't in the list");
    }

    #[test]
    fn a_drift_fights_nothing_and_is_not_spared_as_a_threat() {
        let drift = Goal::Drift;
        assert_eq!((drift.quarry(), drift.attacking()), (None, None));
        assert!(!drift.fights());
        assert_eq!((drift.plundering(), drift.inspecting()), (None, None));
        assert!(!drift.scooping());
        assert!(!drift.spares_player_threat(), "0xe isn't in the list");
    }

    #[test]
    fn surroundings_find_each_ship_and_the_player() {
        let mut jumping = npc(2, AiType::Warship);
        jumping.mode = Mode::JumpingIn { tick: 0 };
        jumping.govt = Some(GovtId(140));
        let mut dying = npc(3, AiType::Warship);
        dying.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let mut disabled = npc(4, AiType::Warship);
        disabled.condition = Condition::Disabled;
        disabled.state.position = crate::geometry::Vec2::new(5.0, 6.0);
        let mut emerging = npc(5, AiType::Warship);
        emerging.mode = Mode::Emerging { ticks_left: 3 };
        let mut coasting = npc(6, AiType::Warship);
        coasting.mode = Mode::Coasting { ticks_left: 30 };
        let npcs = [
            npc(1, AiType::Warship),
            jumping,
            dying,
            disabled,
            emerging,
            coasting,
        ];
        let mut around = Surroundings::new(&[], &npcs);
        let n = |id| ShipRef::Npc(NpcId(id));
        assert!(around.live(n(1)));
        assert!(around.live(n(6)), "holding its course after its burst");
        assert!(!around.live(n(2)), "jumping in");
        assert!(!around.live(n(5)), "emerging from a gate: seen by no one");
        assert!(!around.live(n(3)), "dying");
        assert!(around.live(n(4)), "disabled");
        assert!(!around.live(n(9)), "not here");
        assert!(!around.live(P), "no player");
        assert_eq!(around.govt_of(n(2)), Some(GovtId(140)));
        assert_eq!(around.govt_of(n(9)), None);
        assert_eq!(around.condition_of(n(4)), Some(Condition::Disabled));
        assert_eq!(
            around.state_of(n(4)).map(|state| state.position),
            Some(crate::geometry::Vec2::new(5.0, 6.0))
        );
        assert_eq!(around.state_of(P), None);
        let player = PlayerSide {
            state: ShipState {
                position: crate::geometry::Vec2::new(1.0, 2.0),
                ..ShipState::default()
            },
            condition: Condition::Intact,
            reserves: Reserves::full(1.0, 1.0, 1.0),
            hull: HullSpec::default(),
            handling: Handling::default(),
            rating: 0,
            yardstick: 0,
            untargetable: false,
            blessing: Blessing::NONE,
            ship: crate::catalog::ShipId(128),
            combat_govt: None,
            target: None,
            class: crate::catalog::EscortClass::Freighter,
            boarded: false,
            speed: 0.0,
        };
        around.player = Some(player);
        assert!(around.live(P));
        assert_eq!(around.govt_of(P), None, "the player has none");
        assert_eq!(around.state_of(P), Some(player.state));
        assert_eq!(around.condition_of(P), Some(Condition::Intact));
        around.player = Some(PlayerSide {
            condition: Condition::Dying {
                ticks_left: 1,
                starting: false,
            },
            ..player
        });
        assert!(!around.live(P));
        assert_eq!(around.system_govt, None);
        assert_eq!(around.record, 0);
        assert_eq!(*around.govts, Governments::default());
    }

    #[test]
    fn a_government_blesses_only_through_a_player_in_the_system() {
        let blessed = GovtId(140);
        let mut around = Surroundings::new(&[], &[]);
        around.player = Some(PlayerSide {
            blessing: fixture::blessing_of(blessed),
            ..fixture::player(0.0, 0.0)
        });
        assert!(around.blessed(Some(blessed)));
        assert!(!around.blessed(Some(GovtId(141))), "unblessed");
        assert!(!around.blessed(None), "an independent never blesses");
        around.player = None;
        assert!(!around.blessed(Some(blessed)), "no player in the system");
    }

    #[test]
    fn two_ranks_of_the_same_help_still_grant_it() {
        use nova_sim_catalog::fixture::rank;
        let ranked = |id: i16, flags: u16| RankRecord {
            affil_govt: Some(GovtId(140)),
            flags,
            ..rank(id)
        };
        let records = [
            ranked(130, HELPS_IN_BATTLE),
            ranked(131, HELPS_IN_BATTLE | HELPS_FREE),
        ];
        let active = BTreeSet::from([RankId(130), RankId(131)]);
        let govts = Governments::new([crate::catalog::GovtRecord {
            classes: [1, -1, -1, -1],
            ..nova_sim_catalog::fixture::govt(140)
        }]);
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                favour(&records, &active, Some(GovtId(140)), &govts, reach),
                Favour {
                    battle_help: true,
                    free_help: true,
                },
                "{reach:?}"
            );
        }
    }

    #[test]
    fn the_blessing_table_ends_at_government_383() {
        let mut blessing = Blessing::NONE;
        blessing.bless(GovtId(384), Blessed::Peace);
        blessing.bless(GovtId(127), Blessed::Landing);
        assert_eq!(blessing, Blessing::NONE, "no slot for either");
        assert!(!blessing.blesses(Some(GovtId(384)), Blessed::Peace));
        blessing.bless(GovtId(383), Blessed::Peace);
        assert!(blessing.blesses(Some(GovtId(383)), Blessed::Peace));
        assert!(!blessing.blesses(Some(GovtId(383)), Blessed::Landing));
    }

    #[test]
    fn a_ship_the_player_leads_or_one_led_by_such_a_ship_is_allied_with_the_player() {
        let escorting = |id| Npc {
            escort: Some(crate::escort::EscortDuty {
                formation: crate::escort::FormationSlot::default(),
                order: None,
            }),
            ..npc(id, AiType::Warship)
        };
        let led = |id, lead| Npc {
            leader: Some(NpcId(lead)),
            ..npc(id, AiType::Warship)
        };
        let npcs = [
            escorting(1),
            led(2, 1),
            led(3, 2),
            npc(4, AiType::Warship),
            led(5, 4),
            led(6, 9),
        ];
        let around = Surroundings::new(&[], &npcs);
        let allied: Vec<bool> = npcs
            .iter()
            .map(|npc| around.allied_with_player(npc))
            .collect();
        assert_eq!(
            allied,
            [true, true, false, false, false, false],
            "its escort, the escort's fighter, that fighter's own, a stranger, \
             its fighter, and one whose leader has gone"
        );
    }
}
