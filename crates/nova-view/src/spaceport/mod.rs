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
//! - [`service`]: the [`ServiceScreen`], a placeholder for each service
//!   until it is built.
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
//! - [`grid`]: the parts the Outfitter and the Shipyard share: the grid
//!   of cells, a `ShortName`'s lines, and the selected item's picture and
//!   description.

pub mod bar;
pub mod catalog;
pub mod data;
pub mod grid;
pub mod hire;
pub mod layout;
pub mod outfitter;
pub mod service;
pub mod shipyard;
pub mod trade;
pub mod view;

pub use bar::{BarScreen, Hiring};
pub use catalog::{PortRecord, SpaceportCatalog, StellarId};
pub use hire::HireScreen;
pub use outfitter::{OutfitterCatalog, OutfitterScreen};
pub use service::ServiceScreen;
pub use shipyard::{ShipBaseImages, ShipyardCatalog, ShipyardScreen};
pub use trade::TradeScreen;
pub use view::SpaceportView;
