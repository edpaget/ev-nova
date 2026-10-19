//! [`AppScreen`]'s launch: the main menu opens holding the last pilot, as
//! `_InitEVNova` runs `_OpenLastPilotFile` (@0x473ef) before
//! `_DoMainScreen`.
//!
//! The pilot store keeps a last-pilot record, the original's "Last Pilot"
//! alias (`STR#` 130 #4), which every save and every open rewrite
//! ([`nova_sim::PilotKeeper::open_last`]). At launch, when it names a pilot
//! that opens and whose load needed no fallback, the menu starts holding
//! it, opened as Open Pilot opens a pilot but with the launch's own setup:
//!
//! - its explored systems' nebulae swept as the session is built
//!   ([`nova_sim::Session::load`]), as `_OpenLastPilotFile` sweeps
//!   (@0x765a2-0x765ce). The original sweeps after the load; both settle
//!   before the menu's first frame;
//! - its ship facing a drawn heading and the offers' rolls drawn
//!   ([`FlightView::resume`](nova_view::flight::FlightView::resume));
//! - every active mission's auxiliary ships restarted in the unguarded
//!   form the take-off uses, and the player's system explored, as
//!   `_OpenLastPilotFile`'s loop (@0x765d4-0x76637) and `_DoMainScreen`'s
//!   once-a-launch setup (@0x1affe) do ([`FlightView::open_at_launch`](nova_view::flight::FlightView::open_at_launch)).
//!
//! What its hooks then make due is settled at once, and any save they make
//! due is dropped: the original saves nothing at launch.
//!
//! Otherwise the menu starts holding nothing, with no note and no warning,
//! as `_OpenLastPilotFile` returns silently on any failure (@0x764da,
//! @0x7658a): no store, no record, a record naming a pilot no longer saved
//! (a Strict Play pilot deleted on its death leaves the record, as
//! `_DeletePilotFile` never touches the alias, @0x72e6b-0x72ef6), a save
//! that can't be read or is from a newer version, a pilot whose session
//! can't start on this game data, and a load that had to
//! fall back ([`FlightView::load_repaired`](nova_view::flight::FlightView::load_repaired)), which `_LoadPilotData`
//! reports as -46 (@0x7648b-0x76498) and `_OpenLastPilotFile` treats as a
//! failure; Open Pilot's note `STR#` 140 #52 is `_MainScreenChoice`'s, not
//! the launch's. The original's player globals then hold the repaired or
//! partly read pilot with no game in progress, where the engine holds
//! nothing (rdm:phase/main-screen-fidelity/phase-4-launch-left-outs).
//!
//! Left out of `_OpenLastPilotFile` and `_DoMainScreen`'s setup, with
//! nothing for the engine to do: the resets before the load (a fresh
//! session is built per load), `_MissionPostLoadValidCheck` (crash logging
//! only, @0x9b8f2) and the port clears (the engine redraws every frame).

use super::AppScreen;

impl AppScreen {
    /// Opens the last pilot at launch and holds it (see the module docs),
    /// or holds nothing, silently. Call it last when building the router:
    /// the load reads the router's rules and draws on its chance, so those
    /// must be given first. It shows no note or dialog, and runs before the
    /// first frame.
    #[must_use]
    pub fn open_last_pilot(mut self) -> Self {
        let Some(pilot) = self.keeper().and_then(|keeper| keeper.open_last()) else {
            return self;
        };
        let mut flight = self.pilot_flight(pilot, true);
        if flight.session().is_err() || flight.load_repaired() {
            return self;
        }
        flight.resume();
        flight.open_at_launch();
        self.hold_game(flight);
        let _ = self.settle_game();
        // Before the first frame, which no input or tick precedes.
        self.show_game_on_menu();
        self
    }
}
