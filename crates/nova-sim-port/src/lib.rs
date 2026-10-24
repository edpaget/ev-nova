//! The port layer of EV Nova's simulation: what the player does on a
//! stellar and the screens that come with it (the outfitter, the
//! shipyard, the bar's hires, recharging, the player info screen),
//! boarding a disabled ship and its grants, the escape pod, the radar's
//! IFF and the hypergate sprite. It builds on the foundation,
//! `nova-sim-base`, the catalog, `nova-sim-catalog`, the state,
//! `nova-sim-state`, combat, `nova-sim-combat`, the ai and traffic layer,
//! `nova-sim-ai`, and the story, `nova-sim-story`, since every shop
//! settles on the story's [`Pilot`](nova_sim_story::pilot::Pilot).
//! `nova-sim` re-exports each module at its old path
//! (`nova_sim::shipyard`, `nova_sim::Shipyard`, …), so the rest of the
//! workspace reaches them through it.
//!
//! - [`board`]: boarding a disabled ship: whether the player can
//!   ([`BoardRefusal`](board::BoardRefusal)), the
//!   [`Plunder`](board::Plunder) rolled for it, the plunder dialog's
//!   [`Take`](board::Take)s and what each [`Taken`](board::Taken) did,
//!   capturing it, and assigning a ship captured
//!   ([`Assignment`](board::Assignment)); the
//!   [`BoardingRule`](board::BoardingRule) port with Nova's
//!   [`NovaBoarding`](board::NovaBoarding): who repels boarders, the
//!   capture odds and the capture roll, a person's credits and its grant.
//! - [`christening`]: christening a new pilot's ship: the prompt and
//!   default `_DoNewPilot` asks with
//!   ([`Christening`](christening::Christening)) and the name kept
//!   ([`christen`](christening::christen)).
//! - [`new_pilot`]: the New Pilot dialog's name and nickname: the
//!   defaults `_DoNewPilot` opens it with
//!   ([`name_defaults`](new_pilot::name_defaults)) and the names kept
//!   ([`kept_name`](new_pilot::kept_name)).
//! - [`escape`]: the player's escape: the escape pod and auto-eject
//!   outfits, the escape ship a fighter bay offers, when auto-eject arms,
//!   and where the escape pod is taken.
//! - [`escort_dialog`]: the escort dialog a fleet escort hailed opens:
//!   what it shows ([`EscortDialogView`](escort_dialog::EscortDialogView)),
//!   when an escort can be upgraded or sold, its Upgrade and Sell marks
//!   ([`toggle`](escort_dialog::toggle)), and what leaving a shipyard
//!   does with them ([`settle_marks`](escort_dialog::settle_marks),
//!   [`escort_land_text`](escort_dialog::escort_land_text)).
//! - [`npc_escape`]: what a dying NPC leaves behind: when its crew's
//!   escape pods and its person's pod come out, and when it rolls for an
//!   escape ship.
//! - [`gate_sprite`]: which frame a hypergate's sprite shows, opening
//!   while a ship comes out of it and closing once none does.
//! - [`grant`]: boarding grants: what a person's record grants
//!   ([`PersonGrant`](grant::PersonGrant)), the outfits a grant sees
//!   ([`GrantStock`](grant::GrantStock)), and what it gave
//!   ([`Granted`](grant::Granted)).
//! - [`hire`]: hiring escorts in the bar: which ships are for hire today
//!   ([`HireList`](hire::HireList), [`HireRow`](hire::HireRow),
//!   [`HireRefusal`](hire::HireRefusal)), the
//!   [`HireTerms`](hire::HireTerms) port with Nova's
//!   [`NovaHire`](hire::NovaHire) fee and daily wage,
//!   [`price_flux`](hire::price_flux) (the original's price flux under
//!   the ranks' multiplier, which the shipyard shares), the
//!   [`ControlBits`](nova_sim_catalog::control::ControlBits) port a
//!   ship's `Availability` goes through, what a hire did
//!   ([`Hired`](hire::Hired)), and the escorts who defect unpaid
//!   ([`PayNote`](hire::PayNote)).
//! - [`iff`]: the radar's outfits and colour codes: the radar gear the
//!   player's outfits give ([`RadarGear`](iff::RadarGear)) and the colour
//!   a ship or a stellar shows on the radar ([`ShipIff`](iff::ShipIff),
//!   [`StellarIff`](iff::StellarIff)).
//! - [`murk`]: a system's murk: its `Murk` as loaded
//!   ([`loaded_murk`](murk::loaded_murk)) and the player's effective murk
//!   with its outfits ([`effective_murk`](murk::effective_murk)).
//! - [`order_loop`]: an order of a count run one pass at a time: each
//!   pass handed its [`order_loop::Pass`], the buy loop
//!   ([`order_loop::run`]) stopping at the first refusal and the sell
//!   loop ([`order_loop::run_on`]) running on past it, with the
//!   [`order_loop::OrderRun`] keeping the first refusal and its pass.
//! - [`outfitter`]: the outfitter: which outfits a stellar lists and
//!   sells ([`Outfitter`](outfitter::Outfitter)), their price and mass,
//!   the ship's free mass, and buying and selling one at a time
//!   ([`OutfitOrder`](outfitter::OutfitOrder)).
//! - [`player_info`]: what the player info screen shows
//!   ([`PlayerInfo`](player_info::PlayerInfo)).
//! - [`recharge`]: recharging in the spaceport: which stellars sell
//!   fuel ([`sells_fuel`](recharge::sells_fuel)), when the Recharge
//!   button is enabled ([`enabled`](recharge::enabled)), the refill
//!   ([`fill`](recharge::fill)), and the
//!   [`RechargeRefusal`](recharge::RechargeRefusal).
//! - [`ship_change`]: changing the player's ship outside the shipyard,
//!   as the `C`, `E` and `H` set operators do: which outfits carry over.
//! - [`shipyard`]: the shipyard: which ships a stellar lists and sells
//!   ([`Shipyard`](shipyard::Shipyard)), their price, what the ship flown
//!   trades in for, and buying a new one
//!   ([`ShipPurchase`](shipyard::ShipPurchase)): which outfits carry
//!   over, the cargo kept and the default items fitted.
//! - [`wares`]: the rules the outfitter and the shipyard share: tech
//!   levels, `Require` and `Contribute`, the hiding flags, the hide-higher
//!   sweep and the rows' order.
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog`, `nova-sim-state`, `nova-sim-combat`, `nova-sim-ai`
//! and `nova-sim-story`. An item of a crate above it is named by its
//! facade path as inline code, such as `nova_sim::Session`.

// The foundation, catalog, state, combat, ai and story modules the moved
// code and its docs name by their old `crate::` paths (and `RuleKey`,
// which the docs link by `crate::RuleKey`), which now resolve to
// `nova-sim-base`, `nova-sim-catalog`, `nova-sim-state`,
// `nova-sim-combat`, `nova-sim-ai` and `nova-sim-story`.
#[allow(unused_imports)]
use nova_sim_ai::{ai, escort, gate, hail, traffic};
#[allow(unused_imports)]
use nova_sim_base::{
    RuleKey, chance, flight, fuel, geometry, handling, ionization, reserves, rulebook, rules,
};
#[allow(unused_imports)]
use nova_sim_catalog::{catalog, control};
#[allow(unused_imports)]
use nova_sim_combat::{bay, combat, fleet};
#[allow(unused_imports)]
use nova_sim_state::{clearance, cron, landing, market, outfit_effects, stats};
#[allow(unused_imports)]
use nova_sim_story::{exploration, mission, person, pilot, rank, story, text_tags};

pub mod board;
pub mod christening;
pub mod escape;
pub mod escort_dialog;
pub mod gate_sprite;
pub mod grant;
pub mod hire;
pub mod iff;
pub mod murk;
pub mod new_pilot;
pub mod npc_escape;
pub mod order_loop;
pub mod outfitter;
pub mod player_info;
pub mod recharge;
pub mod ship_change;
pub mod shipyard;
pub(crate) mod string_entry;
pub mod wares;
