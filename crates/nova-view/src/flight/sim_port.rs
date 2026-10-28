//! The [`FlightSim`] port: what the flight view needs of the flight it
//! shows, in the view's own terms, in four methods. A frame to draw and
//! read from ([`FlightSim::frame`]), a frame's fixed steps
//! ([`FlightSim::step`]), the player's commands and the view's asks
//! ([`FlightSim::apply`]) and what the flight raised for the view to act
//! on once ([`FlightSim::drain`]). The values it carries are `nova_sim`'s
//! ([`flight_frame`](nova_sim::flight_frame)), so the session can build
//! them.
//!
//! [`SessionFlight`](super::sim_adapter::SessionFlight) is the adapter
//! over a real `nova_sim::Session`, and the edge builds it
//! (`crate::flight::open_flight`).
//!
//! # How the view reads the flight
//!
//! - **The kept frame.** The view keeps one frame
//!   (`FlightView::frame`) and reads every fact of the flight from it,
//!   mid-input and mid-tick too: the gates a key is taken under
//!   ([`FlightFrame::gates`](nova_sim::flight_frame::FlightFrame::gates)),
//!   the nav mode, docked, the jump, the death `dësc`s. It never fills a
//!   fresh frame to read one fact.
//! - **Every change fills it afresh where it is made**: after each
//!   command (`FlightView::send`), each frame's steps, a catch-up's
//!   drain, and each change behind the port, which goes through
//!   `FlightView::change_sim` (the edge's setup, a pilot's resume and
//!   openings, a test's change). So the frame is never stale at a read: a
//!   quiet tick fills it twice (the catch-up and the steps), a key once
//!   for each command it sends, and a draw not at all.
//! - **An ask doesn't.** A command that only asks
//!   ([`FlightCommand::is_ask`]: the hover and a description's load)
//!   changes nothing a frame shows, which nova-sim
//!   pins (`session/apply/asks.rs`) and `fixture::check_contract`
//!   checks, so no fill follows one. A question the view asks of the
//!   flight between frames, with arguments it computes or drawing on the
//!   flight's chance, is an ask: a [`FlightCommand`] variant answered in
//!   nova-sim, with an [`Answer`] variant of its own.
//! - **What is fixed for the flight's life comes as it opens**: the
//!   weapons whose looks the view reads, the names it words the flight's
//!   messages with and why the flight could not start
//!   ([`FlightOpening`](nova_sim::flight_frame::FlightOpening)), handed to
//!   `FlightView::over` by the edge, which reads it off the adapter
//!   (`SessionFlight::opening`), not asked through the port.
//! - **A name fixed for the flight's life comes in the opening's
//!   [`Names`](nova_sim::flight_frame::Names)** (a ship's, an outfit's, a
//!   good's, a `jünk`'s, a stellar's); **a name the flight decides is a
//!   frame field** (the player's ship's,
//!   [`FlightFrame::player_ship_name`](nova_sim::flight_frame::FlightFrame::player_ship_name);
//!   the date, [`FlightFrame::long_date`](nova_sim::flight_frame::FlightFrame::long_date);
//!   whether a system is explored,
//!   [`FlightFrame::explored`](nova_sim::flight_frame::FlightFrame::explored)),
//!   **or rides the value that carries it** (a mission cargo's, on
//!   [`MissionBoardLine::Retrieved`](nova_sim::MissionBoardLine::Retrieved)).
//!   A text filled for the pilot is a screen's ask
//!   ([`ScreenAsk::PlainText`]).
//! - **A readout is a frame field, an action a command, a one-shot output
//!   an event** ([`FlightFrame`], [`FlightCommand`] with its [`Answer`],
//!   [`FlightEvent`]), each filled or answered on nova-sim's side and
//!   pinned there.
//!
//! The supertraits, [`MapSource`], [`CommSource`], [`PortSource`] and
//! [`PilotSource`], hold the course map's, the comm's, the spaceport's and
//! the pilot's screens' questions, each method defaulting to "nothing
//! open", so a sim that has none of them open needs no code. [`MapSource`]
//! has the course map's and the floating map's, [`CommSource`] the hail's
//! and the boarding's, [`PortSource`] the spaceport's screens' and the
//! mission screens', and [`PilotSource`] the player info screen's and the
//! pilot panel's.
//!
//! The `fixture` feature (and this crate's tests) exposes `fixture`:
//! `FakeFlight`, a scripted sim for the view's component tests, and
//! `check_contract`, what every sim must honour. `FakeFlight` answers
//! every ask from what a test tells it, apart from its answer queue and
//! its log of commands applied: a new ask there is one field and setter
//! for its answer, and one arm in its ask's match.

use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, FloatingLinks, ScreenAnswer, ScreenAsk,
    StepInput, StepReport,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::mission::screens::OFFER_GAP_TICKS;
use nova_sim::pilot_panel::PilotPanel;
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{
    HailView, HireList, MapMarks, Market, MissionId, MissionInfo, News, Outfitter, PlayerInfo,
    PlunderView, SavedNav, ShipId, ShipNaming, ShipRefusal, Shipyard, SystemId,
};

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// The flight the view shows, as the view needs it, in four methods: the
/// frame, the steps, the commands (the asks among them) and the drain (see
/// the module docs). The supertraits hold the screens' questions.
pub trait FlightSim: MapSource + CommSource + PortSource + PilotSource {
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

    /// Answers a screen's `ask`, made as the screen opens or as its input
    /// changes, never per frame, with the snapshot it draws from: the
    /// [`ScreenAnswer`] of the ask's kind
    /// ([`ScreenAnswer::answers`]), or [`ScreenAnswer::Nothing`] with
    /// nothing to answer. It changes nothing a frame shows, so no fill
    /// follows one.
    fn screen(&mut self, ask: ScreenAsk) -> ScreenAnswer;
}

/// The course map's questions of the flight, and the floating map's. Each
/// defaults to "nothing open": `None` where the view
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

    /// The galaxy the course map is built from: the catalog's star map
    /// with the overlay applied (`nova_sim::Session::galaxy`), read as a
    /// `GalaxyChanged` event is caught up with.
    fn galaxy(&self) -> Option<&OverlaidGalaxy> {
        None
    }
}

/// The comm's questions of the flight: the hail under way and the
/// boarding under way, each defaulting to "nothing open". The escort
/// dialog's contents come only with the commands' answers
/// ([`Answer::Hailed`], [`Answer::EscortDialog`]), as the view asks
/// nothing else of it.
pub trait CommSource {
    /// The comm dialog's contents while a hail is under way
    /// (`nova_sim::Session::hailing`): none with no hail under way.
    fn hailing(&self) -> Option<HailView> {
        None
    }

    /// What is on board the ship being boarded while the plunder dialog
    /// is open (`nova_sim::Session::boarding`): none with no boarding
    /// under way, or a capture awaiting its assignment.
    fn boarding(&self) -> Option<PlunderView> {
        None
    }
}

/// The spaceport's questions of the flight: what its screens show as they
/// open or after a command, each defaulting to "nothing open": no
/// exchange, outfitter, shipyard, hire list or news, nothing listed, no
/// fuel sold, no intro seen. Those that settle the day's rolls on the
/// flight's chance or draw on it take `&mut self`. What the player does
/// there is a command ([`FlightSim::apply`]), and so is a description's
/// load ([`FlightCommand::LoadDescription`]), an ask.
pub trait PortSource {
    /// The exchange of the stellar landed on
    /// (`nova_sim::Session::market`).
    fn market(&self) -> Option<Market> {
        None
    }

    /// Whether the spaceport's Recharge button is enabled
    /// (`nova_sim::Session::recharge_enabled`).
    fn recharge_enabled(&self) -> bool {
        false
    }

    /// The news of the bar visit under way (`nova_sim::Session::news`).
    fn news(&self) -> Option<&News> {
        None
    }

    /// The picture the bar's news is drawn over
    /// (`nova_sim::Session::news_picture`).
    fn news_picture(&self) -> Option<i16> {
        None
    }

    /// What the mission info screen lists
    /// (`nova_sim::Session::mission_info`): nothing by default
    /// ([`MissionInfo::default`]).
    fn mission_info(&self) -> MissionInfo {
        MissionInfo::default()
    }

    /// Whether the pilot flying has seen the intro screen
    /// (`nova_sim::Session::seen_intro`).
    fn seen_intro(&self) -> bool {
        false
    }

    /// Whether `mission`'s offer dialog has a refuse button
    /// (`nova_sim::Session::refusable_offer_dialog`).
    fn refusable_offer_dialog(&self, mission: MissionId) -> bool {
        let _ = mission;
        false
    }

    /// `raw` with its tags filled for `mission`'s offer on the visit under
    /// way, as the mission computer fills a cut offer's name
    /// (`nova_sim::Session::offer_text`): none when it is not offered, as
    /// in flight, where no offer is open.
    fn offer_text(&self, mission: MissionId, raw: &str) -> Option<String> {
        let _ = (mission, raw);
        None
    }

    /// The outfitter of the stellar landed on, the day's rolls drawn on
    /// the flight's chance (`nova_sim::Session::outfitter`).
    fn outfitter(&mut self) -> Option<Outfitter> {
        None
    }

    /// The shipyard of the stellar landed on, the day's rolls drawn on the
    /// flight's chance (`nova_sim::Session::shipyard`).
    fn shipyard(&mut self) -> Option<Shipyard> {
        None
    }

    /// The ships for hire in the bar of the stellar landed on, the day's
    /// rolls drawn on the flight's chance
    /// (`nova_sim::Session::escorts_for_hire`).
    fn escorts_for_hire(&mut self) -> Option<HireList> {
        None
    }

    /// The prompt for naming a ship of class `ship` before it is bought,
    /// its words read from the catalog, on the flight's chance
    /// (`nova_sim::Session::ship_naming`): no shipyard by default.
    ///
    /// # Errors
    ///
    /// The [`ShipRefusal`] that applies.
    fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        let _ = ship;
        Err(ShipRefusal::NoShipyard)
    }

    /// The wait before a venue's next offer, in the Mac's ticks, drawn on
    /// the flight's chance (`nova_sim::mission::offer_gap`): by default
    /// the gap with no roll, [`OFFER_GAP_TICKS`].
    fn offer_gap(&mut self) -> u32 {
        OFFER_GAP_TICKS
    }

    /// Whether the bar's hire screen may hire escorts now
    /// (`nova_sim::Session::can_hire_escorts`, `_CanHireEscorts`): false
    /// by default, a flight with no fleet to add to.
    fn can_hire_escorts(&self) -> bool {
        false
    }
}

/// The pilot's screens' questions of the flight: what the player info
/// screen and the main screen's pilot panel show, each none by default, a
/// flight with no pilot to show. Opening the player info screen is a
/// command ([`FlightCommand::OpenPlayerInfo`]), as it resets the flight
/// first.
pub trait PilotSource {
    /// What the player info screen shows, changing nothing
    /// (`nova_sim::Session::player_info`).
    fn player_info(&self) -> Option<PlayerInfo> {
        None
    }

    /// What the main screen's pilot panel says about the pilot, changing
    /// nothing (`nova_sim::Session::pilot_panel`).
    fn pilot_panel(&self) -> Option<PilotPanel> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source with nothing open: every answer its default.
    struct NothingOpen;

    impl MapSource for NothingOpen {}

    impl CommSource for NothingOpen {}

    impl PortSource for NothingOpen {}

    impl PilotSource for NothingOpen {}

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

        fn screen(&mut self, _ask: ScreenAsk) -> ScreenAnswer {
            ScreenAnswer::Nothing
        }
    }

    impl MapSource for Defaults {}

    impl CommSource for Defaults {}

    impl PortSource for Defaults {}

    impl PilotSource for Defaults {}

    #[test]
    fn a_flight_with_nothing_to_it_fills_the_default_frame_and_refuses_every_command() {
        let mut flight = Defaults;
        let mut frame = crate::flight::sim_port::fixture::unlike(&FlightFrame::default());
        flight.frame(&mut frame);
        assert_eq!(frame, FlightFrame::default());
        assert_eq!(flight.apply(FlightCommand::Jump), Answer::Refused);
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
        assert_eq!(source.galaxy(), None);
    }

    #[test]
    fn a_pilot_source_with_no_pilot_shows_none() {
        let source = NothingOpen;
        assert_eq!(source.player_info(), None);
        assert_eq!(source.pilot_panel(), None);
    }

    #[test]
    fn a_comm_source_with_nothing_open_answers_nothing() {
        let source = NothingOpen;
        assert_eq!(source.hailing(), None);
        assert_eq!(source.boarding(), None);
    }

    #[test]
    fn a_port_source_with_nothing_open_answers_nothing() {
        let mut source = NothingOpen;
        assert_eq!(source.market(), None);
        assert!(!source.recharge_enabled());
        assert_eq!(source.news(), None);
        assert_eq!(source.news_picture(), None);
        assert_eq!(source.mission_info(), MissionInfo::default());
        assert!(!source.seen_intro());
        assert!(!source.refusable_offer_dialog(MissionId(130)));
        assert_eq!(source.outfitter(), None);
        assert_eq!(source.shipyard(), None);
        assert_eq!(source.escorts_for_hire(), None);
        assert_eq!(
            source.ship_naming(ShipId(129)),
            Err(ShipRefusal::NoShipyard)
        );
        assert_eq!(source.offer_gap(), OFFER_GAP_TICKS);
        assert_eq!(source.offer_text(MissionId(130), "Run <PAY>"), None);
        assert!(!source.can_hire_escorts());
    }
}
