//! What the new game's own landed screens read of its storylines: the
//! active missions the mission info key lists beside the stock ones
//! ([`StoryRow`], on [`MissionInfo::story`](crate::MissionInfo::story)),
//! and the refusal of an accept for want of room ([`RoomRefusal`], raised
//! as [`FlightEvent::RoomRefused`](super::FlightEvent::RoomRefused)). Each is
//! plain data the session builds from its own reads, borrowing nothing of
//! the session's. A session whose data holds no storylines (the stock
//! game's) never raises one.

use crate::catalog::StellarId;

/// One active storyline mission, as the new game's missions log lists it
/// (`Statics::story_rows`): by catalog id, with no slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryRow {
    /// The mission's id.
    pub mission: String,
    /// Its display title.
    pub title: String,
    /// What the player must do.
    pub objective: StoryObjective,
    /// The tons of its cargo aboard.
    pub cargo: u32,
    /// Whether its objective is met.
    pub met: bool,
    /// Where it completes: its completion's stellar, else a delivery's.
    /// `None` when neither names one.
    pub complete_at: Option<StellarId>,
}

/// A storyline mission's objective, its names resolved for the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoryObjective {
    /// Carry `tons` of `cargo` to `to`.
    Deliver {
        /// What the cargo is.
        cargo: String,
        /// How many tons.
        tons: u32,
        /// The stellar to deliver to.
        to: StellarId,
    },
    /// Take a jump point from the faction holding it.
    TakePoint {
        /// The point's hypergate, `None` when the data names no such
        /// point.
        gate: Option<StellarId>,
        /// The holder's government name, or its faction id when no
        /// government stands for it.
        holder: String,
    },
}

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
