//! The pilot flying as the developer tools' desk reads it
//! ([`DeskReading`]), from `nova_sim::session::edit`, which re-exports it
//! and keeps the pilot's plain edits.

use nova_sim_base::reserves::Reserves;
use nova_sim_catalog::catalog::{StellarId, SystemId};
use nova_sim_catalog::control::Bit;
use nova_sim_state::date::GameDate;

/// The pilot flying as the developer tools' desk reads it
/// (`nova_sim::Session::desk_reading`): plain values, which the desk names.
#[derive(Clone, Debug, PartialEq)]
pub struct DeskReading {
    /// The pilot's name.
    pub name: String,
    /// Its cash.
    pub credits: i64,
    /// The ship's shield, armour and fuel.
    pub reserves: Reserves,
    /// The date.
    pub date: GameDate,
    /// The date as the game writes it (`nova_sim::Session::date_text`).
    pub date_text: String,
    /// The system the pilot is in.
    pub system: SystemId,
    /// The stellar it is landed on; none in flight.
    pub landed: Option<StellarId>,
    /// The control bits set, ascending.
    pub bits: Vec<Bit>,
}
