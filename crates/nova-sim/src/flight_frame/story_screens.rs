//! What the new game's own landed screens read of its storylines: the
//! refusal of an accept for want of room ([`RoomRefusal`], raised as
//! [`FlightEvent::RoomRefused`](super::FlightEvent::RoomRefused)). Each is
//! plain data the session builds from its own reads, borrowing nothing of
//! the session's. A session whose data holds no storylines (the stock
//! game's) never raises one.

/// An offer's accept refused because its cargo doesn't fit the hold
/// (`nova_sim_story::storylines::lifecycle::Decision::NoRoom`). The offer
/// stands, decided for the visit, and comes back at a later landing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomRefusal {
    /// The mission's id.
    pub mission: String,
    /// The mission's display title.
    pub title: String,
    /// What its cargo is.
    pub cargo: String,
    /// The tons it would load.
    pub tons: u32,
    /// The tons free in the hold as it was refused, never below 0.
    pub room: i64,
}
