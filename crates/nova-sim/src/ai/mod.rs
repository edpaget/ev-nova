//! NPC decisions: what each NPC sets out to do, on a timer kept apart from
//! the physics step (see [`traffic`](crate::traffic)).
//!
//! A [`Behaviour`] decides an NPC's [`Goal`] from what is around it
//! ([`Surroundings`]), and the fire command it holds and the ship it
//! targets, and answers the hits ships take ([`Behaviour::react`]). The
//! [`traffic`](crate::traffic) autopilot then flies each goal, every tick.
//!
//! - [`Peaceful`]: traffic that never fights: it lands, or leaves, and
//!   escorts follow their lead. Its decisions are the idle block the four
//!   AI types share ([`idle`]).
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
mod fixture;
pub mod interceptor;
pub mod nova;
pub mod odds;
pub mod react;
pub mod target;
pub mod warship;
pub mod wimpy;

use std::fmt::Debug;

use crate::catalog::{GovtId, LandingSite, StellarId};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::Strike;
use crate::combat::armament::Trigger;
use crate::combat::hull::{Condition, HullSpec};
use crate::flight::ShipState;
use crate::govt::Governments;
use crate::handling::Handling;
use crate::hyperspace::JUMP_FUEL;
use crate::landing::landable;
use crate::reserves::Reserves;
use crate::traffic::npc::{Mode, Npc, NpcId};

pub use brave::BraveTrader;
pub use carried::CarriedAi;
pub use escort::EscortAi;
pub use interceptor::Interceptor;
pub use nova::NovaAi;
pub use warship::Warship;
pub use wimpy::WimpyTrader;

/// What an NPC sets out to do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Goal {
    /// Nothing: it comes to a stop where it is.
    #[default]
    Idle,
    /// Land on this stellar, and leave the system that way.
    Land(StellarId),
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
}

/// No governments at all.
static NO_GOVERNMENTS: Governments = Governments::empty();

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
}

impl<'a> Surroundings<'a> {
    /// `npcs` among `sites`, with no player, no governments and no record.
    #[must_use]
    pub fn new(sites: &'a [LandingSite], npcs: &'a [Npc]) -> Self {
        Self {
            sites,
            npcs,
            player: None,
            govts: &NO_GOVERNMENTS,
            system_govt: None,
            record: 0,
        }
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
    /// disabled, and not still jumping in.
    #[must_use]
    pub fn live(&self, ship: ShipRef) -> bool {
        let jumping_in = match ship {
            ShipRef::Player => false,
            ShipRef::Npc(id) => self.npc(id).is_some_and(|npc| npc.mode != Mode::Flying),
        };
        !jumping_in && self.condition_of(ship).is_some_and(Condition::hittable)
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

/// How NPCs decide what to do.
pub trait Behaviour: Debug {
    /// `npc`'s goal now, among `around`, rolling any choice on `chance`.
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal;

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
/// the `EV Nova` executable):
///
/// - An escort follows its lead while the lead is in the system; once it
///   is gone, the escort decides as below.
/// - A goal it can still fly is kept: a stellar that is still there and
///   landable, or a jump it still has the fuel for.
/// - Otherwise it lands on a landable stellar picked at random, one not
///   of a government at war with its own (`_SelectRandomStellarDest`
///   @0x805de, its enemy test @0x80807), or jumps out when there is none.
///   The original sends a ship that has not visited a stellar of the
///   system there, and one that has away; here a ship that lands leaves
///   the system, so none has, and every ship, of any AI type, heads for a
///   stellar (as `_HyperShipSpawn` makes every arrival do).
/// - A ship that would jump without a jump's fuel stays idle.
pub fn idle(npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
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
    let welcoming: Vec<_> = around
        .sites
        .iter()
        .filter(|site| landable(site) && !around.govts.enemies(npc.govt, site.govt))
        .collect();
    if welcoming.is_empty() {
        return leave;
    }
    let len = u32::try_from(welcoming.len()).unwrap_or(u32::MAX);
    let pick = chance.below(len) as usize;
    welcoming
        .get(pick)
        .map_or(leave, |site| Goal::Land(site.id))
}

/// The ship that provoked `npc`, its target since the hit, while it is
/// provoked and that ship is live among `around`.
#[must_use]
pub fn provoked_by(npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
    npc.target
        .filter(|&attacker| npc.provoked > 0.0 && around.live(attacker))
}

/// Whether `npc` can still fly its goal among `around`: a stellar still
/// there and landable, or a jump it has the fuel for. Idling, following
/// once the lead is gone, fighting and inspecting are no goals to keep.
fn can_still_fly(npc: &Npc, around: &Surroundings) -> bool {
    match npc.goal {
        Goal::Land(stellar) => around
            .sites
            .iter()
            .any(|site| site.id == stellar && landable(site)),
        Goal::JumpOut => npc.reserves.fuel.now >= JUMP_FUEL,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtRecord;
    use crate::landing::StellarFlags;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, planet};
    use crate::traffic::npc::AiType;

    fn npc(id: u32, ai_type: AiType) -> Npc {
        Npc {
            ai_type,
            ..crate::testkit::npc(id, ShipStats::new(FAST, &[]))
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
                ..crate::testkit::govt(140)
            },
            GovtRecord {
                enemies: [1, -1, -1, -1],
                ..crate::testkit::govt(141)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..crate::testkit::govt(142)
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
            by: ShipRef::Player,
            damage: 30.0,
            downed: None,
        };
        assert_eq!(
            Peaceful.react(&npcs[0], &strike, &around),
            Reaction::default()
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
        jumping.mode = Mode::JumpingIn { ticks_left: 3 };
        jumping.govt = Some(GovtId(140));
        let mut dying = npc(3, AiType::Warship);
        dying.condition = Condition::Dying { ticks_left: 3 };
        let mut disabled = npc(4, AiType::Warship);
        disabled.condition = Condition::Disabled;
        disabled.state.position = crate::geometry::Vec2::new(5.0, 6.0);
        let npcs = [npc(1, AiType::Warship), jumping, dying, disabled];
        let mut around = Surroundings::new(&[], &npcs);
        let n = |id| ShipRef::Npc(NpcId(id));
        assert!(around.live(n(1)));
        assert!(!around.live(n(2)), "jumping in");
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
        };
        around.player = Some(player);
        assert!(around.live(P));
        assert_eq!(around.govt_of(P), None, "the player has none");
        assert_eq!(around.state_of(P), Some(player.state));
        assert_eq!(around.condition_of(P), Some(Condition::Intact));
        around.player = Some(PlayerSide {
            condition: Condition::Dying { ticks_left: 1 },
            ..player
        });
        assert!(!around.live(P));
        assert_eq!(around.system_govt, None);
        assert_eq!(around.record, 0);
        assert_eq!(*around.govts, Governments::default());
    }
}
