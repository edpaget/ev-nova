//! What the main screen's pilot panel says about the pilot loaded, as
//! `_MainScreenUpdate` (@0x17dcc-0x184ea) reads it from the player
//! globals: the session gathers it (`nova_sim::Session::pilot_panel`), and
//! the view lays it out with the labels, the ship's subtitle and its
//! picture, which come from the game data.
//!
//! - **The pilot's name** (`_playerName` 0x230540) and **the ship's name**
//!   (`_shipName` 0x293d80), as the pilot holds them; a ship with no name
//!   reads empty.
//! - **The ship's class** (`shipClass[type]`+0x6c), the name the player
//!   info's Ship Class row shows (`nova_sim::Session::ship_name`), and the
//!   type flown, which the view reads the subtitle (+0xac, the `shïp`'s
//!   Subtitle, which `_LoadObjectData` copies from the resource's 0x6E6
//!   @0x7b0e7-0x7b10a) and the picture of.
//! - **The legal status in the current system**: "N/A" (`STR#` 2002 #396)
//!   when `_SystemIsInhabited` says the system is not (@0x180fb-0x1811a),
//!   otherwise `_DrawLegalStatusString` of it (@0x18113): exactly the
//!   player info's Legal Status row.
//! - **The combat rating's words**, `_DrawCombatRatingString` @0x57e5
//!   ([`rating::string_index`](crate::rating::string_index) into `STR#`
//!   138); empty when the list lacks the string.
//! - **The date**, `_DateToMedString(_gameTime)`
//!   (`nova_sim::Session::med_date_text`).

use crate::catalog::ShipId;

/// What the pilot panel says about the pilot loaded (see the module
/// docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PilotPanel {
    /// The pilot's name.
    pub pilot_name: String,
    /// The ship's name; empty when it has none.
    pub ship_name: String,
    /// The ship's class name, as the player info's Ship Class row shows
    /// it.
    pub ship_class: String,
    /// The ship type flown, whose subtitle and picture the panel shows.
    pub ship: ShipId,
    /// The legal status in the current system, "N/A" when it is
    /// uninhabited.
    pub legal_status: String,
    /// The combat rating's words.
    pub combat_rating: String,
    /// Today's date.
    pub date: String,
}
