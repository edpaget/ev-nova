//! The ai and traffic layer of EV Nova's simulation: the NPCs a system
//! spawns and how they decide, fly, hail, escort, scan and scoop, and the
//! player's own picks among them, such as the nav and ship targets, the
//! hover test and the jump route. It builds on the foundation,
//! `nova-sim-base`, the catalog, `nova-sim-catalog`, the state,
//! `nova-sim-state`, and combat, `nova-sim-combat`. `nova-sim` re-exports
//! each module at its old path (`nova_sim::traffic`, `nova_sim::Traffic`,
//! …), so the rest of the workspace reaches them through it.
//!
//! - [`ai`]: NPC decisions: the [`Behaviour`](ai::Behaviour) port that
//!   sets each NPC's [`Goal`](ai::Goal) from its
//!   [`Surroundings`](ai::Surroundings) and answers hits;
//!   [`Peaceful`](ai::Peaceful), traffic that never fights; and
//!   [`NovaAi`](ai::NovaAi), Nova's combat AI, routing each NPC by its AI
//!   type to [`WimpyTrader`](ai::WimpyTrader),
//!   [`BraveTrader`](ai::BraveTrader), [`Warship`](ai::Warship) or
//!   [`Interceptor`](ai::Interceptor): whom they attack by governments'
//!   relations and the player's legal record, when they flee, how they
//!   come to each other's help, and which weapon they fire; the player's
//!   escorts to [`EscortAi`](ai::EscortAi), which flies them by their
//!   standing orders; an NPC carrier's fighters to
//!   [`CarriedAi`](ai::CarriedAi); and a leaderless miner to
//!   [`MinerAi`](ai::MinerAi).
//! - [`brackets`]: the corner brackets round the nav target and the ship
//!   target: their [`FlyIn`](brackets::FlyIn) from a selection and a ship
//!   target's [`Standing`](brackets::Standing).
//! - [`cargo_box`]: the cargo boxes a jettison drops behind the player
//!   and its trader escorts: how many ([`box_count`](cargo_box::box_count)),
//!   where each starts, and how it drifts, spins and fades
//!   ([`CargoBoxes`](cargo_box::CargoBoxes)).
//! - [`escort`]: the player's escorts: their
//!   [`EscortClass`](escort::EscortClass), the
//!   [`EscortOrder`](escort::EscortOrder) each follows, the
//!   [`EscortCommand`](escort::EscortCommand)s the player gives an
//!   [`EscortGroup`](escort::EscortGroup) and what one changed
//!   ([`Commanded`](escort::Commanded)), the escort menu's
//!   [`ClassRow`](escort::ClassRow)s, an escort NPC's
//!   [`EscortDuty`](escort::EscortDuty), the formation it keeps beside the
//!   player, and how it scores the threats to the player.
//! - [`gate`]: hypergates and wormholes: which a stellar is
//!   ([`GateKind`](gate::GateKind)), where each leads, and how a ship
//!   comes out of one, under the [`GateArrivalRule`](gate::GateArrivalRule)
//!   and [`WormholeRule`](gate::WormholeRule).
//! - [`hail`]: hailing a ship: whether it likes the player
//!   ([`Attitude`](hail::Attitude)), the
//!   [`Conversation`](hail::Conversation) a hail rolls (its variant, mood,
//!   price and advice) and haggling over a price
//!   ([`Haggle`](hail::Haggle)), the [`Reply`](hail::Reply) it says, and
//!   the [`HailOption`](hail::HailOption) port the comm dialog lists
//!   ([`HailOptions`](hail::HailOptions)), with Nova's Greetings, Request
//!   Assistance and Beg For Mercy, [`Release`](hail::Release) for the
//!   player's escort, and Use As Escort ([`JoinFleet`](hail::JoinFleet))
//!   for a person who may join, which answer each press with an
//!   [`Answer`](hail::Answer): a reply, a [`Deed`](hail::Deed) and a price
//!   asked ([`Ask`](hail::Ask)); the [`Help`](hail::Help) a ship gives the
//!   player once asked; and what the comm dialog shows of a ship the
//!   player leads ([`EscortStatus`](hail::EscortStatus)), a hired escort's
//!   daily pay among it.
//! - [`hover`]: the hover test: whether the mouse pointer is over a ship
//!   the player can target ([`Hover`](hover::Hover)), as the original's
//!   `_HandleMouse` walks the ship slots, read off the boxes the view drew
//!   ([`DrawnShips`](hover::DrawnShips)).
//! - [`hyperspace`]: the [`StarMap`](hyperspace::StarMap) of hyperlinks
//!   and the routes along it, whether the ship can jump
//!   ([`check_jump`](hyperspace::check_jump)), and where it arrives.
//! - [`navigation`]: how the player selects the stellar navigation
//!   target: by its `NavDef` slot, the nearest, or a click
//!   ([`stellar_under`](navigation::stellar_under)), as the original does.
//! - [`scan`]: a warship's or an interceptor's scan of the player: whether
//!   it goes on ([`scan::goes_on`]) and the illegal `jünk` or outfit it
//!   finds after the mission cargo ([`scan::contraband`]).
//! - [`scoop`]: scooping asteroid boxes into the hold: which ships scoop
//!   ([`player_scoops`](scoop::player_scoops),
//!   [`npc_scoop`](scoop::npc_scoop)), which boxes they take and what each
//!   box adds ([`Haul`](scoop::Haul)).
//! - [`targeting`]: which NPC the player's target command picks
//!   ([`TargetPick`](targeting::TargetPick)): the nearest, the nearest
//!   threat, the next in turn, or the next of the player's escorts.
//! - [`traffic`]: NPC [`Traffic`](traffic::Traffic): the ships spawned
//!   from a system's `düde`s and fleets on arrival and over time, each
//!   [`Npc`](traffic::npc::Npc) flown by an autopilot with the player's
//!   flight physics and stats, and the persons (`përs`) among them
//!   ([`traffic::persons`]).
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog`, `nova-sim-state` and `nova-sim-combat`. An item of a
//! crate above it is named by its facade path as inline code, such as
//! `nova_sim::Session`.

// The foundation, catalog, state and combat modules and items the moved
// code and its docs name by their old `crate::` paths, which now resolve
// to `nova-sim-base`, `nova-sim-catalog`, `nova-sim-state` and
// `nova-sim-combat`.
#[allow(unused_imports)]
use nova_sim_base::{
    NeverFires, RuleKey, Rulebook, chance, flight, fuel, geometry, handling, reserves, rulebook,
    trig,
};
#[allow(unused_imports)]
use nova_sim_catalog::{ControlBits, SimSound, buoy, catalog, control, warp};
#[allow(unused_imports)]
use nova_sim_combat::message::SimMessage;
#[allow(unused_imports)]
use nova_sim_combat::{bay, cloak, combat, fleet, self_repair, ship_layer};
#[allow(unused_imports)]
use nova_sim_state::{govt, landing, legal, market, stats};

pub mod ai;
pub mod brackets;
pub mod cargo_box;
pub mod escort;
pub mod gate;
pub mod hail;
pub mod hover;
pub mod hyperspace;
pub mod navigation;
pub mod scan;
pub mod scoop;
pub mod targeting;
pub mod traffic;
