//! The view vocabulary of EV Nova's simulation: the view models the
//! session fills for the flight view and the value types its calls take
//! and give, as plain data and the pure rules that decide them, with
//! nothing of the session itself. It builds on the
//! foundation, `nova-sim-base`, the catalog, `nova-sim-catalog`, and the
//! state layer, `nova-sim-state`, whose `shown` module holds the view
//! types the layers above build. `nova-sim` re-exports each module at its
//! old path (`nova_sim::map_panel`, `nova_sim::glow`, …), or each item
//! of a session module at its old path (`nova_sim::session::SavedNav`,
//! `nova_sim::SavedNav`), so the rest of the workspace reaches them
//! through it, and keeps the session code that gathers their facts.
//!
//! The view models:
//!
//! - [`flight_frame`]: the values the flight view's port carries: the
//!   frame, the commands and their answers, the events, a frame's steps
//!   and what a screen asks
//!   ([`FlightFrame`](flight_frame::FlightFrame),
//!   [`FlightCommand`](flight_frame::FlightCommand),
//!   [`ScreenAsk`](flight_frame::ScreenAsk)).
//! - [`galaxy_map`]: what the galaxy map draws
//!   ([`MapDrawing`](galaxy_map::MapDrawing)), the hypergate arrows'
//!   heads, what its model shows of the galaxy
//!   ([`GalaxyShown`](galaxy_map::GalaxyShown)) and what a map marks
//!   beside the mission systems ([`MapMarks`]).
//! - [`glow`]: how bright a ship's engine glow is: its base level, which
//!   thrust ramps up and coasting down ([`ramp_glow`](glow::ramp_glow)),
//!   and the level drawn at a tick, flickering on the blink chance
//!   ([`glow_level`]).
//! - [`key_fade`]: what fades the flight cursor: which keys set
//!   `_keyPressedThisFrame`, past which gates
//!   ([`KeyFades`](key_fade::KeyFades)).
//! - [`map_borders`]: the galaxy map's government borders, as
//!   `_AddSystemInfluenceToMap` and `_RecalcMapInfluence` work them out: a
//!   grid of screen cells of four slots each ([`map_borders::grid`]), and
//!   the coarse and fine passes over it ([`map_borders::passes`]).
//! - [`map_lanes`]: how a drawn link looks by the new game's lane style
//!   ([`LaneLook`](map_lanes::LaneLook)).
//! - [`map_panel`]: what the galaxy map's two info panels say about the
//!   system selected, as `_MapDialogUpdate` decides it: the ports, the
//!   hazards, the title, and the explored level's goods and services
//!   ([`map_panel::map_panel`]).
//! - [`map_search`]: the galaxy map's Find, as `_DoSystemSearch` matches a
//!   typed name against the systems' names ([`map_search::search`]).
//! - [`pilot_panel`]: what the main screen's pilot panel says about the
//!   pilot loaded ([`PilotPanel`](pilot_panel::PilotPanel)).
//! - [`red_alert`]: the Red Alert: when a threat to the player sounds it
//!   and the icon's blink ([`AlertIcon`]).
//! - [`status_cargo`]: what the status bar's cargo area lists
//!   ([`StatusCargo`]).
//! - [`target_panel`]: what the status bar's target panel says of the
//!   target at its bottom: its
//!   [`TargetStatus`] and its
//!   [`TargetTag`].
//! - [`viewer_lanes`]: the links the developer galaxy viewer draws, picked
//!   by the rulebook ([`viewer_links`](viewer_lanes::viewer_links)).
//!
//! The session's value types, each module named for the `nova_sim::session`
//! module that builds them:
//!
//! - [`edit`]: the pilot as the developer tools' desk reads it.
//! - [`events`]: what the session tells a view about the scene it draws.
//! - [`map_click`]: a press on an in-flight map and what the click did.
//! - [`mission_screens`]: the mission computer, the mission texts and the
//!   mission info screen.
//! - [`missions`]: what a mission's offer and abort refused or left shown.
//! - [`nav`]: the nav a mission dialog saves and puts back.
//! - [`person_offers`]: what hailing the player's target opened.
//! - [`persons`]: a person's hail quote said.
//! - [`script_effects`]: what settling a script and a take-off did.
//! - [`ship_change`]: a ship change's interface load.
//! - [`slot_visibility`]: where a ship's hover and click box is.
//! - [`stellar_pick`]: which stellar a navigation key selects and what the
//!   land key did, from `nova_sim::session` itself.
//!
//! The facade: everything else of nova-sim that nova-view's prod code
//! names, at the path nova-sim gives it, but the session's adapter's
//! four names: `nova_sim::Session`, `nova_sim::SessionSetup`,
//! `nova_sim::SimCatalog` and `nova_sim::Pilot`. nova-view depends on
//! this crate directly and never on nova-sim; the adapter that names
//! those four lives in nova-flight, above both
//! (rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim). (The galaxy viewer's
//! gate lanes come from `nova_sim::viewer_lanes::gate_links`, a builder
//! that reads a fresh story, so it stays above the layers; the app calls
//! it and hands nova-view the [`GateLinks`](viewer_lanes::GateLinks) it
//! builds.) Each name is the
//! same item, re-exported here from the crate below the layers it now
//! lives in. A root name (`nova_sim::Vec2`, `nova_sim::NpcId`) is
//! re-exported at this crate's root, a lower crate's module the view
//! reaches whole (`nova_sim::trig`, `nova_sim::catalog`) is re-exported
//! whole, and a layer module (`nova_sim::combat`, `nova_sim::hail`'s
//! siblings such as [`brackets`], [`escort_dialog`]) is a module here
//! holding the names of it the view reaches. So the view's `nova_sim::`
//! paths become `nova_sim_frame::` paths unchanged.
//!
//! A doc here links only to items of this crate, `nova-sim-base`,
//! `nova-sim-catalog` and `nova-sim-state`. An item of a crate above it is
//! named by its facade path as inline code, such as `nova_sim::Session`.

pub mod asteroid;
pub mod bay;
pub mod brackets;
pub mod capture_naming;
pub mod cloak;
pub mod combat;
pub mod edit;
pub mod escape_pod;
pub mod escort;
pub mod escort_dialog;
pub mod events;
pub mod fleet;
pub mod flight_frame;
pub mod galaxy_map;
pub mod gamble;
pub mod gate;
pub mod glow;
pub mod hire;
pub mod hyperspace;
pub mod iff;
pub mod key_fade;
pub mod map_borders;
pub mod map_click;
pub mod map_lanes;
pub mod map_panel;
pub mod map_search;
pub mod mission;
pub mod mission_screens;
pub mod missions;
pub mod nav;
pub mod navigation;
pub mod news;
pub mod order_loop;
pub mod person_offers;
pub mod persons;
pub mod pilot_panel;
pub mod planet_comm;
pub mod player_info;
pub mod red_alert;
pub mod script_effects;
pub mod ship_change;
pub mod ship_layer;
pub mod shipyard;
pub mod slot_pool;
pub mod slot_visibility;
pub mod sprite_layer;
pub mod status_cargo;
pub mod stellar_frames;
pub mod stellar_pick;
pub mod storylines;
pub mod target_panel;
pub mod text_tags;
pub mod traffic;
pub mod viewer_lanes;

// The facade: every name nova-view reaches at nova-sim's root, and the
// lower crates' modules it reaches whole, at the same path here (the
// layer modules it reaches are the modules below).
pub use crate::edit::DeskReading;
pub use crate::events::SessionEvent;
pub use crate::galaxy_map::MapMarks;
pub use crate::glow::glow_level;
pub use crate::mission_screens::{
    ComputerRefusal, ComputerRow, InfoRow, MissionComputer, MissionInfo, MissionInfoPress,
    MissionShown, TextBar,
};
pub use crate::missions::{AbortError, NotOffered, RefuseShown};
pub use crate::nav::SavedNav;
pub use crate::person_offers::Hailed;
pub use crate::persons::PersonQuote;
pub use crate::red_alert::{ALERT_CICN, AlertIcon};
pub use crate::script_effects::{Departure, Settled};
pub use crate::ship_change::InterfaceLoad;
pub use crate::slot_visibility::HoverPlace;
pub use crate::status_cargo::{Special, StatusCargo};
pub use crate::stellar_pick::{LandPress, StellarPick};
pub use crate::target_panel::{TargetStatus, TargetTag};
pub use nova_data::expr::Bit;
pub use nova_data::wire::id::{
    BoomId, GovtId, MissionId, OutfitId, ShipId, SoundId, StellarId, SystemId, WeaponId,
};
pub use nova_sim_base::blink::{Blink, HashedRolls, lights_level};
pub use nova_sim_base::chance::{Chance, NeverFires};
pub use nova_sim_base::clock::{FixedStep, Steps, TICKS_PER_SECOND};
pub use nova_sim_base::flight::{Amounts, Controls, Face, Turn};
pub use nova_sim_base::geometry::Vec2;
pub use nova_sim_base::reserves::{Gauge, Reserve, Reserves};
pub use nova_sim_base::rulebook::{RuleKey, RuleSource, Rulebook};
pub use nova_sim_base::speed_mult::SpeedMult;
pub use nova_sim_base::{blink, chance, flight, player_wrap, speed_mult, sprite_place, trig};
pub use nova_sim_catalog::SoundCue;
pub use nova_sim_catalog::catalog::{
    AsteroidCatalog, CombatCatalog, DescMovie, EscortClass, StarSystem,
};
pub use nova_sim_catalog::control::Gender;
pub use nova_sim_catalog::sound::SimSound;
pub use nova_sim_catalog::{catalog, data, sound};
pub use nova_sim_state::clearance::DockKind;
pub use nova_sim_state::date::GameDate;
pub use nova_sim_state::govt::Governments;
pub use nova_sim_state::landing;
pub use nova_sim_state::landing::{GateKind, LandingRefusal, Service, services};
pub use nova_sim_state::market::{Direction, Good, Lot, Market, MarketRow, Order, TradeRefusal};
pub use nova_sim_state::outfit_effects::Rgb15;
pub use nova_sim_state::shown::ai::Help;
pub use nova_sim_state::shown::bay::FighterNote;
pub use nova_sim_state::shown::board::{
    Assigned, Assignment, BoardRefusal, Boarding, MissionBoardLine, PlunderView, Take, Taken,
};
pub use nova_sim_state::shown::combat::hull::{Condition, ESCAPE_POD_SHIP};
pub use nova_sim_state::shown::combat::report::SimDiagnostic;
pub use nova_sim_state::shown::combat::{CombatEvent, NpcId, ShipRef};
pub use nova_sim_state::shown::dialog::conversation::ConversationPage;
pub use nova_sim_state::shown::escort::{ClassRow, Commanded, EscortCommand, EscortGroup};
pub use nova_sim_state::shown::gate::GateRefusal;
pub use nova_sim_state::shown::grant::Granted;
pub use nova_sim_state::shown::hail::deal::Haggle;
pub use nova_sim_state::shown::hail::{CommNote, CreditUnit, HailRefusal, HailView, ShownWage};
pub use nova_sim_state::shown::hire::{HireList, HireRefusal, HireRow, Hired, PayNote};
pub use nova_sim_state::shown::hover::{DrawnBox, Hover};
pub use nova_sim_state::shown::hyperspace::{JumpReadiness, JumpRefusal};
pub use nova_sim_state::shown::iff::{RadarGear, ShipIff, StellarIff};
pub use nova_sim_state::shown::message::{DockFind, SelfDestructLine, SimMessage};
pub use nova_sim_state::shown::mission::active::AcceptError;
pub use nova_sim_state::shown::mission::screens::{OfferPrompt, VenueOffer};
pub use nova_sim_state::shown::navigation::NavMode;
pub use nova_sim_state::shown::news::News;
pub use nova_sim_state::shown::outfitter::{
    LcNames, OutfitOrder, OutfitRefusal, OutfitRow, OutfitRun, Outfitter, Reselection, SaleAlert,
};
pub use nova_sim_state::shown::player_fate::{Drawn, PlayerCommand, PlayerFate, Takes};
pub use nova_sim_state::shown::player_info::PlayerInfo;
pub use nova_sim_state::shown::recharge::{RechargeRefusal, sells_fuel};
pub use nova_sim_state::shown::ship_layer::ShipLayer;
pub use nova_sim_state::shown::shipyard::{
    Rating, ShipNaming, ShipPurchase, ShipRefusal, ShipRow, ShipSpecs, Shipyard, StockLine,
};
pub use nova_sim_state::shown::story::rebuild::DrawnSheet;
pub use nova_sim_state::shown::targeting::TargetPick;
pub use nova_sim_state::shown::tutorial::TutorialHint;
pub use nova_sim_state::shown::wares::{ShownTons, TonUnit};
