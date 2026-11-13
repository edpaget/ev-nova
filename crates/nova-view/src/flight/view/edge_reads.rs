//! The reads of the view's own state that the edge's checks over a real
//! flight make (`edge_checks`, which sit with the adapter above
//! nova-view: rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim).
//! Fixture-only: view code never calls them. Each one is the view's
//! private state, or the private step, the check reached while it sat in
//! this crate.

use nova_sim::flight_frame::FlightFrame;
use nova_sim::{ShipId, SimMessage, SystemId};

use super::{FlightSim, FlightView, SharedChance};

impl<C, S: FlightSim> FlightView<C, S> {
    /// The words of each of `messages`, in order, with its ticks, as the
    /// view words the flight's messages (`FlightView::word`): the edge's
    /// checks read the rulebook and the chance the edge handed the
    /// wording.
    pub fn worded(&mut self, messages: Vec<SimMessage>) -> Vec<(String, u16)> {
        self.word(messages)
    }

    /// The chance the view words its picks on, which the edge hands it
    /// with the flight's own.
    #[must_use]
    pub fn words_chance(&self) -> &SharedChance {
        &self.words_chance
    }

    /// Whether the stars are to be scattered afresh at the next draw.
    #[must_use]
    pub fn stars_due(&self) -> bool {
        self.stars_due
    }

    /// Lets go of a scatter of the stars that is due, so a check sees
    /// whether the edge's beginning or entering play asks for one.
    pub fn clear_stars_due(&mut self) {
        self.stars_due = false;
    }

    /// The frame the view keeps of the flight, as it last filled it.
    #[must_use]
    pub fn kept_frame(&self) -> &FlightFrame {
        &self.frame
    }

    /// Whether the view has read the sheet of NPC ship type `ship`.
    #[must_use]
    pub fn reads_npc_sheet(&self, ship: ShipId) -> bool {
        self.npc_sheets.contains_key(&ship)
    }

    /// Opens the hypergate map offering `offered`, as entering a
    /// hypergate opens it.
    pub fn open_gate_map(&mut self, offered: Vec<SystemId>) {
        self.map.offer_gates(offered);
        self.open_map();
    }
}
