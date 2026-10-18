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
//!   government's relations and the player's legal record, or an NPC.
//! - [`fire`]: which weapon an NPC fires, as it attacks, snipes or flees.
//! - [`react`]: how a ship hit answers, and how others come to its help.
//! - [`wimpy`], [`brave`], [`warship`] and [`interceptor`]: Nova's four AI
//!   types, each its own [`Behaviour`], and [`NovaAi`], which routes each
//!   NPC to the one for its type.
//! - [`escort`]: [`EscortAi`], how the player's escorts fly and fight by
//!   their standing orders; [`NovaAi`] routes every escort to it.
//! - [`carried`]: [`CarriedAi`], how an NPC carrier's fighters fly and
//!   fight with their carrier; [`NovaAi`] routes every one to it.

pub mod brave;
pub mod carried;
pub mod escort;
pub mod fire;
#[cfg(test)]
pub(crate) mod fixture;
pub mod interceptor;
pub mod nova;
pub mod odds;
pub mod react;
pub mod target;
pub mod warship;
pub mod wimpy;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

use crate::catalog::{GovtId, LandingSite, RankId, RankRecord, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::Strike;
use crate::combat::armament::Trigger;
use crate::combat::hull::{Condition, HullSpec};
use crate::flight::ShipState;
use crate::gate::GateKind;
use crate::govt::Governments;
use crate::handling::Handling;
use crate::hyperspace::JUMP_FUEL;
use crate::landing::is_landable as landable;
use crate::reserves::Reserves;
use crate::rulebook::RuleSource;
use crate::traffic::destination;
use crate::traffic::npc::{Npc, NpcId};

pub use crate::traffic::destination::Admit;

pub use brave::BraveTrader;
pub use carried::CarriedAi;
pub use escort::EscortAi;
pub use interceptor::Interceptor;
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
    /// Nothing: it comes to a stop where it is.
    #[default]
    Idle,
    /// Land on this stellar, and leave the system that way. A stellar
    /// that is a hypergate or wormhole is entered instead, as
    /// [`Goal::EnterGate`] (`_HighLevelAIHandler` state 1, @0x8d8b8-0x8d8fe).
    Land(StellarId),
    /// Enter this hypergate or wormhole, and leave the system through it:
    /// the original's goal 0x14 (`_HighLevelAIHandler` state 20, see
    /// [`gate_entry`](crate::traffic::gate_entry)).
    EnterGate(StellarId),
    /// Fly out past its jump distance and jump out.
    JumpOut,
    /// Keep with this NPC, its fleet's lead.
    Follow(NpcId),
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
    /// (@0x8ed18-0x8ed31). It coasts with no thrust and no fire, and
    /// keeps the hold until provoked or its type's think lifts it (see
    /// [`idle`]). It fights nothing: its quarry is none.
    Hold,
}

impl Goal {
    /// The ship it fights, attacking, sniping at or fleeing from it; none
    /// for any other goal.
    #[must_use]
    pub fn quarry(self) -> Option<ShipRef> {
        match self {
            Self::Attack(ship) | Self::Snipe(ship) | Self::Flee(ship) => Some(ship),
            _ => None,
        }
    }

    /// The ship it attacks or snipes at, if any.
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
            | Self::Land(_)
            | Self::EnterGate(_)
            | Self::JumpOut
            | Self::Follow(_)
            | Self::Attack(_)
            | Self::Snipe(_)
            | Self::Flee(_)
            | Self::Inspect(_)
            | Self::Assist(_)
            | Self::Formation { .. }
            | Self::Dock(_)
            | Self::Hold => false,
        }
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
    #[must_use]
    pub fn spares_player_threat(self) -> bool {
        matches!(
            self,
            Self::Inspect(_)
                | Self::Assist(_)
                | Self::Dock(_)
                | Self::Follow(_)
                | Self::Formation { .. }
                | Self::Hold
        )
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
}

impl<'a> Surroundings<'a> {
    /// `npcs` among `sites`, with no player, no governments, no record and
    /// no damage aimed.
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
        }
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

    /// NPC `id`, if it is in the system.
    #[must_use]
    pub fn npc(&self, id: NpcId) -> Option<&'a Npc> {
        self.npcs.iter().find(|npc| npc.id == id)
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
    /// The damage it adds to its provocation.
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

/// The idle block the four AI types share (e.g. `_WarshipAI` @0x8b06c in
/// the `EV Nova` executable), picking as their (ship, 0, 0) call does
/// ([`idle_admitting`] with [`Admit::All`]).
pub fn idle(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
    idle_admitting(npc, around, chance, Admit::All)
}

/// The idle block, its pick admitting `admit` (see
/// [`destination`](crate::traffic::destination)):
///
/// - An escort follows its lead while the lead is in the system; once it
///   is gone, the escort decides as below.
/// - A goal it can still fly is kept: a stellar that is still there and
///   landable, a gate it enters that is still there, a jump it still
///   has the fuel for, or a hold.
/// - Otherwise it heads for a stellar picked at random among
///   `_SelectRandomStellarDest`'s candidates (@0x805de): landable, within
///   999 of the centre on each axis, not uninhabited unless a gate, not
///   of a government at war with its own, the gates chosen by its
///   government's `Flags2`; or it jumps out when there is none. The
///   original sends a ship that has not visited a stellar of the system
///   there, and one that has away; here a ship that lands leaves the
///   system, so none has, and every ship, of any AI type, heads for a
///   stellar (as `_HyperShipSpawn` makes every arrival do). The pick is
///   one `below(candidates)` draw in the system's stellar order: what the
///   original's `Rand(16)` loop decides, a uniform pick among them. A
///   hypergate or wormhole picked is entered ([`Goal::EnterGate`]), the
///   ship leaving the system through it.
/// - A held ship ([`Goal::Hold`], the original's goal 10 with no
///   leader) keeps its hold and picks nothing: every type's destination
///   pick needs goal 0 (`_WimpyTraderAI` @0x8b06c, `_BraveTraderAI`
///   @0x8b4d6), and `_AICheckParentExists` (@0x8926a-0x89281) never
///   clears the goal of a ship with no leader. Only provocation, or a
///   hunter's government's word, lifts it.
/// - A ship that would jump without a jump's fuel stays idle.
pub fn idle_admitting(
    npc: &Npc,
    around: &Surroundings,
    chance: &mut dyn Chance,
    admit: Admit,
) -> Goal {
    if let Some(lead) = npc.leader
        && around.npc(lead).is_some()
    {
        return Goal::Follow(lead);
    }
    if can_still_fly(npc, around) {
        return npc.goal;
    }
    let leave = if npc.reserves.fuel.now >= JUMP_FUEL {
        Goal::JumpOut
    } else {
        Goal::Idle
    };
    let candidates = destination::candidates(npc.govt, around.sites, around.govts, admit);
    if candidates.is_empty() {
        return leave;
    }
    let len = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let pick = chance.below(len) as usize;
    candidates.get(pick).map_or(leave, |site| {
        if GateKind::of(site.flags2).is_some() {
            Goal::EnterGate(site.id)
        } else {
            Goal::Land(site.id)
        }
    })
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
/// again), a jump it has the fuel for, or a hold. Idling, following
/// once the lead is gone, fighting and inspecting are no goals to keep.
fn can_still_fly(npc: &Npc, around: &Surroundings) -> bool {
    match npc.goal {
        Goal::Land(stellar) => around
            .sites
            .iter()
            .any(|site| site.id == stellar && landable(site)),
        Goal::EnterGate(gate) => around.sites.iter().any(|site| site.id == gate),
        Goal::JumpOut => npc.reserves.fuel.now >= JUMP_FUEL,
        Goal::Hold => true,
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
            Goal::Follow(NpcId(2)),
            Goal::Attack(ship),
            Goal::Snipe(ship),
            Goal::Flee(ship),
            Goal::Inspect(ship),
            Goal::Assist(Help::Refuel),
            Goal::Formation { guard: None },
            Goal::Dock(ShipRef::Player),
            Goal::Hold,
        ] {
            assert!(!goal.scooping(), "{goal:?}");
        }
        assert!(!npc(1, AiType::WimpyTrader).is_scooping());
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
        let plain = Admit::PlainOnly { uninhabited: true };
        let mut chance = Draws::of(&[0]);
        assert_eq!(
            idle_admitting(&npc(1, AiType::Interceptor), &around, &mut chance, plain),
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

    #[test]
    fn a_ship_without_a_jumps_fuel_that_would_jump_stays_idle() {
        for ai_type in [AiType::WimpyTrader, AiType::Warship] {
            let mut short = npc(1, ai_type);
            short.reserves.fuel.now = 99.9;
            assert_eq!(decide(&short, &[], &[], &mut Draws::of(&[])), Goal::Idle);
            short.reserves.fuel.now = 100.0;
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
                Goal::Follow(NpcId(1))
            );
            assert!(chance.asked.is_empty());
            escort.goal = Goal::Follow(NpcId(1));
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
            ship: ShipRef::Npc(NpcId(1)),
            by: Some(ShipRef::Player),
            damage: 30.0,
            downed: None,
            direct: true,
            intended: true,
            lock: false,
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
        let mut stranded = npc(1, AiType::Warship);
        stranded.goal = Goal::JumpOut;
        stranded.reserves.fuel.now = 50.0;
        assert_eq!(decide(&stranded, &[], &[], &mut Draws::of(&[])), Goal::Idle);
        for goal in [
            Goal::Follow(NpcId(7)),
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
            Goal::Follow(NpcId(1)),
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
}
