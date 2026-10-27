//! The [`FlightSim`] port: what the flight view needs of the flight it
//! shows, in the view's own terms. A frame to draw from
//! ([`FlightSim::frame`]), a frame's fixed steps ([`FlightSim::step`]), the
//! player's commands ([`FlightSim::apply`]) and what the flight raised
//! for the view to act on once ([`FlightSim::drain`]). The values it
//! carries are `nova_sim`'s ([`flight_frame`](nova_sim::flight_frame)),
//! so the session can build them.
//!
//! [`SessionFlight`](super::sim_adapter::SessionFlight) is the adapter
//! over a real `nova_sim::Session`. The port is the strangler's seam:
//! the view still reaches the session itself in the
//! `FlightView<C, SessionFlight<C>>` blocks, and later work moves that
//! code across the port a slice at a time, growing the port and its
//! values as it goes.
//!
//! The three supertraits, [`MapSource`], [`CommSource`] and
//! [`PortSource`], hold the course map's, the comm's and the spaceport's
//! questions, each method defaulting to "nothing open", so a sim that has
//! none of them open needs no code. [`MapSource`] has the course map's,
//! the floating map's and the nav area's; the other two are empty for
//! now.
//!
//! The `fixture` feature (and this crate's tests) exposes `fixture`:
//! `FakeFlight`, a scripted sim for the view's component tests, and
//! `check_contract`, what every sim must honour.

use nova_sim::flight_frame::{
    Answer, DrawnBoxes, FlightCommand, FlightEvent, FlightFrame, FloatingLinks, StepInput,
    StepReport,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{
    DescMovie, Hover, MapMarks, MissionId, PlayerCommand, SavedNav, ShipId, StellarId, SystemId,
    Vec2,
};

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// The flight the view shows, as the view needs it (see the module
/// docs).
pub trait FlightSim: MapSource + CommSource + PortSource {
    /// Fills `into` with what the view draws now, writing every field, so
    /// a frame kept from an earlier fill shows nothing stale.
    fn frame(&self, into: &mut FlightFrame);

    /// Runs up to `steps` of the flight's fixed steps for a frame, given
    /// `input`, and fills `into` afresh with what each step reported
    /// ([`StepReport`]): the steps run, each pass's records, and whether
    /// the stepping stopped short. The view calls it once a frame, its
    /// steps or none, as the flight holds the input then.
    fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport);

    /// Runs the player's `command` and answers it:
    /// [`Answer::Refused`] when it did nothing, and then no frame changes.
    fn apply(&mut self, command: FlightCommand) -> Answer;

    /// Moves what the flight raised since the last drain onto the end of
    /// `into`, leaving none to drain again.
    fn drain(&mut self, into: &mut Vec<FlightEvent>);

    /// Whether the flight takes the player's command `gate` now
    /// (`nova_sim::Session::takes`): the view asks before it acts on a
    /// key, mid-input, after the commands it has sent, and does nothing
    /// when it is refused. A flight that failed to start takes every one,
    /// as it has nothing to refuse them with.
    fn takes(&self, gate: PlayerCommand) -> bool;

    /// What the pointer at `at`, in system coordinates, is over among the
    /// things the player can target, the ships boxed as `drawn` says they
    /// are drawn (`nova_sim::Session::hover`): the cursor asks it each
    /// step it shows. `None` with no box and no stellar under it.
    fn hover(&self, at: Vec2, drawn: &DrawnBoxes) -> Option<Hover>;

    /// The ship types the system's traffic will bring
    /// (`nova_sim::Session::coming_traffic_ships`), whose sheets the view
    /// reads as play is entered. None by default: a sim with no traffic.
    fn coming_traffic(&self) -> Vec<ShipId> {
        Vec::new()
    }

    /// Loads `raw`, a `dësc` the view read that may be missing, naming
    /// `movie`, as `_LoadDescription` loads one
    /// (`nova_sim::Session::load_description`): taken in for the pilot,
    /// its `l33t` letters drawn on the flight's chance, its movie the last
    /// loaded. A port method rather than a command, as it draws on the
    /// flight's chance. By default only its conditional text is resolved,
    /// for a new pilot ([`fresh_description`]), as a flight that failed
    /// to start does; a missing one is empty.
    fn load_description(&mut self, raw: Option<&str>, movie: Option<&DescMovie>) -> String {
        let _ = movie;
        fresh_description(raw)
    }
}

/// `raw`, a `dësc` that may be missing, with its conditional text
/// resolved for a new pilot
/// ([`Gate::FRESH`](nova_sim::control::Gate::FRESH)): what a flight that
/// failed to start loads ([`FlightSim::load_description`]'s default).
/// A missing one is empty.
#[must_use]
pub fn fresh_description(raw: Option<&str>) -> String {
    raw.map(|raw| {
        nova_sim::text_parse::conditional_text(
            raw,
            nova_sim::control::Gate::FRESH.pilot,
            nova_sim::text_parse::ParseRules::default(),
        )
    })
    .unwrap_or_default()
}

/// The course map's questions of the flight, and the floating map's and
/// the nav area's. Each defaults to "nothing open": `None` where the view
/// does nothing without an answer, otherwise nothing found, marked or
/// listed.
pub trait MapSource {
    /// What the course map draws now (`nova_sim::Session::map_drawing`):
    /// on the hypergate map when `hypergate` holds the systems its gate
    /// offers, with `missions` the mission systems taken as it opened and
    /// `marks` its opener's.
    fn map_drawing(
        &self,
        hypergate: Option<&[SystemId]>,
        missions: &[SystemId],
        marks: MapMarks,
    ) -> Option<MapDrawing> {
        let _ = (hypergate, missions, marks);
        None
    }

    /// The course map's info panels for the system `selected` on it
    /// (`nova_sim::Session::map_panel`), on the hypergate map with the
    /// systems its gate offers.
    fn map_panel(
        &self,
        selected: Option<SystemId>,
        hypergate: Option<&[SystemId]>,
    ) -> Option<MapPanel> {
        let _ = (selected, hypergate);
        None
    }

    /// What the course map's borders are painted from
    /// (`nova_sim::Session::map_border_sources`).
    fn map_border_sources(&self) -> Option<BorderSources> {
        None
    }

    /// What the map's Find finds for `query`
    /// (`nova_sim::Session::find_map_system`).
    fn find_map_system(&self, query: &str) -> Option<SearchOutcome> {
        let _ = query;
        None
    }

    /// The mission systems the course map arrows as it opens
    /// (`nova_sim::Session::map_mission_systems`).
    fn map_mission_systems(&self) -> Vec<SystemId> {
        Vec::new()
    }

    /// The system the course map selects as it opens, and after its
    /// cycle (`nova_sim::Session::map_preselection`).
    fn map_preselection(&self) -> Option<SystemId> {
        None
    }

    /// The system the floating map selects as it shows
    /// (`nova_sim::Session::floating_preselection`).
    fn floating_preselection(&self) -> Option<SystemId> {
        None
    }

    /// The destinations the hypergate map's Tab cycles through
    /// (`nova_sim::Session::hypergate_cycle`): none with no hypergate
    /// entry pending.
    fn hypergate_cycle(&self) -> Vec<Option<SystemId>> {
        Vec::new()
    }

    /// The marks an offer of `mission` gives its map
    /// (`nova_sim::Session::offer_map_marks`).
    fn offer_map_marks(&self, mission: MissionId) -> MapMarks {
        let _ = mission;
        MapMarks::NONE
    }

    /// The marks the mission computer gives its map for its row
    /// `selected` (`nova_sim::Session::computer_map_marks`).
    fn computer_map_marks(&self, selected: Option<MissionId>) -> MapMarks {
        let _ = selected;
        MapMarks::NONE
    }

    /// The marks mission info gives its map for its slot `slot`
    /// (`nova_sim::Session::mission_info_map_marks`).
    fn mission_info_map_marks(&self, slot: Option<u8>) -> MapMarks {
        let _ = slot;
        MapMarks::NONE
    }

    /// The nav as it stands, for a map's close or its opener to put back
    /// (`nova_sim::Session::map_nav`).
    fn map_nav(&self) -> Option<SavedNav> {
        None
    }

    /// The floating map's places: the current system's and each link's
    /// its clicks test (`nova_sim::Session::floating_map_places`).
    fn floating_map_places(&self) -> FloatingLinks {
        FloatingLinks::default()
    }

    /// Stellar `stellar`'s name as the session read it, for the nav area
    /// to name one its scene doesn't hold (`nova_sim::TagNames`).
    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        let _ = stellar;
        None
    }

    /// Whether the pilot has explored `system`, which the nav area names
    /// only then (`nova_sim::Pilot::has_explored`).
    fn has_explored(&self, system: SystemId) -> bool {
        let _ = system;
        false
    }

    /// The galaxy the course map is built from: the catalog's star map
    /// with the overlay applied (`nova_sim::Session::galaxy`), read as a
    /// `GalaxyChanged` event is caught up with.
    fn galaxy(&self) -> Option<&OverlaidGalaxy> {
        None
    }
}

/// The comm's questions of the flight: hails and their answers. None yet:
/// each method will default to "nothing open".
pub trait CommSource {}

/// The spaceport's questions of the flight. None yet: each method will
/// default to "nothing open".
pub trait PortSource {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source with nothing open: every answer its default.
    struct NothingOpen;

    impl MapSource for NothingOpen {}

    /// A flight with nothing to it, answering every port method it may
    /// leave to its default by that default.
    struct Defaults;

    impl FlightSim for Defaults {
        fn frame(&self, into: &mut FlightFrame) {
            *into = FlightFrame::default();
        }

        fn step(&mut self, _input: &StepInput, _steps: u32, into: &mut StepReport) {
            into.clear();
        }

        fn apply(&mut self, _command: FlightCommand) -> Answer {
            Answer::Refused
        }

        fn drain(&mut self, _into: &mut Vec<FlightEvent>) {}

        fn takes(&self, _gate: PlayerCommand) -> bool {
            true
        }

        fn hover(&self, _at: Vec2, _drawn: &DrawnBoxes) -> Option<Hover> {
            None
        }
    }

    impl MapSource for Defaults {}

    impl CommSource for Defaults {}

    impl PortSource for Defaults {}

    #[test]
    fn by_default_a_description_is_resolved_for_a_new_pilot_and_a_missing_one_is_empty() {
        let mut flight = Defaults;
        assert_eq!(
            flight.load_description(Some("{b5 \"set\" \"clear\"}, sir."), None),
            "clear, sir."
        );
        assert_eq!(flight.load_description(None, None), "");
        assert_eq!(flight.coming_traffic(), []);
    }

    #[test]
    fn a_source_with_nothing_open_answers_nothing() {
        let source = NothingOpen;
        let none = SystemId(130);
        assert_eq!(source.map_drawing(None, &[], MapMarks::NONE), None);
        assert_eq!(source.map_panel(Some(none), None), None);
        assert_eq!(source.map_border_sources(), None);
        assert_eq!(source.find_map_system("sol"), None);
        assert_eq!(source.map_mission_systems(), []);
        assert_eq!(source.map_preselection(), None);
        assert_eq!(source.floating_preselection(), None);
        assert_eq!(source.hypergate_cycle(), []);
        assert_eq!(source.offer_map_marks(MissionId(128)), MapMarks::NONE);
        assert_eq!(source.computer_map_marks(None), MapMarks::NONE);
        assert_eq!(source.mission_info_map_marks(Some(0)), MapMarks::NONE);
        assert_eq!(source.map_nav(), None);
        assert_eq!(source.floating_map_places(), FloatingLinks::default());
        assert_eq!(source.stellar_name(StellarId(128)), None);
        assert!(!source.has_explored(none));
        assert_eq!(source.galaxy(), None);
    }
}
