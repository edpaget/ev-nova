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
//! - [`trade`]: the Trade Center's [`TradeScreen`], the stellar's
//!   commodity exchange.

pub mod catalog;
pub mod data;
pub mod layout;
pub mod service;
pub mod trade;
pub mod view;

pub use catalog::{PortRecord, SpaceportCatalog, StellarId};
pub use service::ServiceScreen;
pub use trade::TradeScreen;
pub use view::SpaceportView;
