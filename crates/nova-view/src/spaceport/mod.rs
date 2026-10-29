//! The spaceport: the screen the player lands on, showing the stellar's
//! landscape, name and description, and a button for each service its
//! `spöb` flags offer, laid out by the interface file's "Spaceport"
//! dialog.
//!
//! - [`catalog`]: the [`SpaceportCatalog`] port, the stellar's name, flags
//!   and pictures.
//! - [`data`]: the port's adapter over `nova_data`'s `GameData`.
//! - [`layout`]: the decisions: which item shows what, which button each
//!   service gets and what it says, and which picture is the landscape.
//! - [`view`]: the [`SpaceportView`] screen.
//! - [`ambient`]: the stellar's ambient sound, and when it plays while
//!   the spaceport is frontmost.
//! - [`service`]: the [`ServiceScreen`], a placeholder for each service
//!   the spaceport is not given.
//! - [`bar`]: the Bar's [`BarScreen`], the stellar's bar text and its
//!   Hire Escort button.
//! - [`trade`]: the Trade Center's [`TradeScreen`], the stellar's
//!   commodity exchange.
//! - [`outfitter`]: the Outfitter's [`OutfitterScreen`], the stellar's
//!   outfits for sale.
//! - [`shipyard`]: the Shipyard's [`ShipyardScreen`], the stellar's ships
//!   for sale, and its info panel.
//! - [`hire`]: the bar's [`HireScreen`], the ships for hire, built from
//!   the Shipyard's pieces.
//! - [`missions`]: the [`MissionComputerScreen`], the offers of Mission
//!   BBS, and the [`MissionInfoScreen`], the active missions, read and
//!   aborted.
//! - [`offer`]: the [`OfferDialog`], a mission offered at the bar.
//! - [`news`]: the bar's [`NewsScreen`], the visit's commercial line and
//!   story.
//! - [`grid`]: the parts the Outfitter and the Shipyard share: the grid
//!   of cells, a `ShortName`'s lines, and the selected item's picture and
//!   description.
//! - [`grid_clicks`]: the shipyard's and the hire screen's double-click
//!   on a cell, which opens Info.
//! - [`shipyard_grid`]: the shipyard's and the hire screen's grid
//!   selection, as `_ShipyardFilter` keeps it for both.
//! - [`modal`]: the [`ModalHost`](modal::ModalHost) the screens open
//!   their dialogs in, over themselves.
//! - [`port_map`]: the [`PortMap`] the spaceport asks for, the bar's or a
//!   mission dialog's.
//! - [`quantity_ask`]: the [`QuantityAsk`](quantity_ask::QuantityAsk),
//!   the quantity dialog as the exchange and the outfitter host it.

pub mod ambient;
pub mod bar;
pub mod buttons;
pub mod catalog;
pub mod credits;
pub mod data;
pub mod grid;
pub mod grid_arrows;
mod grid_clicks;
pub mod hire;
pub mod layout;
pub mod missions;
pub mod modal;
pub mod news;
pub mod offer;
pub mod outfitter;
pub mod port_map;
pub mod quantity_ask;
pub mod race;
pub mod service;
pub mod shipyard;
mod shipyard_grid;
pub mod trade;
pub mod view;

pub use bar::{BarScreen, Hiring};
pub use catalog::{PortRecord, SpaceportCatalog, StellarId};
pub use hire::HireScreen;
pub use layout::{PORT_KEYS, PortKey};
pub use missions::{MissionComputerScreen, MissionInfoScreen, OfferNames};
pub use news::NewsScreen;
pub use offer::{OfferChoice, OfferDialog};
pub use outfitter::{OutfitterCatalog, OutfitterScreen};
pub use port_map::PortMap;
pub use service::ServiceScreen;
pub use shipyard::{ShipBaseImages, ShipOrder, ShipyardCatalog, ShipyardScreen};
pub use trade::{ListColors, ListLooks, TradeScreen};
pub use view::{PortDialog, SpaceportView};
