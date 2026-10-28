//! The [`FlightSim`] port: what the flight view needs of the flight it
//! shows, in the view's own terms, in five methods. A frame to draw and
//! read from ([`FlightSim::frame`]), a frame's fixed steps
//! ([`FlightSim::step`]), the player's commands and the view's asks
//! ([`FlightSim::apply`]), what the flight raised for the view to act on
//! once ([`FlightSim::drain`]), and what a screen draws from, asked as it
//! opens or as its input changes ([`FlightSim::screen`]). The values it
//! carries are `nova_sim`'s ([`flight_frame`](nova_sim::flight_frame)), so
//! the session can build them.
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
//! - **A read made at every draw or every frame is a frame field**: the
//!   floating map's places, which its draw centres on
//!   ([`FlightFrame::floating`](nova_sim::flight_frame::FlightFrame::floating)),
//!   the readouts, the HUD.
//! - **A question a screen asks as it opens or as its input changes is a
//!   screen's ask** ([`ScreenAsk`], answered by the [`ScreenAnswer`] of its
//!   kind through [`FlightSim::screen`]): the course map's galaxy, its
//!   opening, each redraw, its panels after a selection change, its cycle
//!   and Find; the floating map's show; the entry's coming traffic; a
//!   text's fill. The asks one screen always makes together are one
//!   answer (the course map's opening, [`MapOpen`](nova_sim::flight_frame::MapOpen);
//!   its redraw, [`MapScreen`](nova_sim::flight_frame::MapScreen)). None
//!   changes a frame, which nova-sim pins (`session/screen.rs`) and
//!   `fixture::check_contract` checks, so no fill follows one; and none
//!   is asked per frame, save the floating map's show, which comes on
//!   every frame a key holds it up, as the original's does, and asks each
//!   time, as the flight steps between (`ScreenAsk::FloatingMap`'s docs).
//!   A flight that failed to start answers each [`ScreenAnswer::Nothing`],
//!   and the screen shows what it would with nothing to show.
//! - **An ask on `apply` is a per-frame question with arguments the view
//!   computes, or a load that writes run state** ([`FlightCommand::is_ask`]:
//!   the cursor's hover, with the boxes the view draws, after each tick's
//!   steps; a description's load, which draws on the flight's chance and
//!   writes the rank governments and the last movie). It changes nothing
//!   a frame shows either, so no fill follows one; it is a
//!   [`FlightCommand`] variant answered in nova-sim, with an [`Answer`]
//!   variant of its own.
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
//! The supertraits, [`CommSource`], [`PortSource`] and [`PilotSource`],
//! hold the comm's, the spaceport's and the pilot's screens' questions,
//! each method defaulting to "nothing open", so a sim that has none of
//! them open needs no code: [`CommSource`] the hail's and the boarding's,
//! [`PortSource`] the spaceport's screens' and the mission screens', and
//! [`PilotSource`] the player info screen's and the pilot panel's. The
//! maps' questions are screens' asks.
//!
//! The `fixture` feature (and this crate's tests) exposes `fixture`:
//! `FakeFlight`, a scripted sim for the view's component tests, and
//! `check_contract`, what every sim must honour. `FakeFlight` answers
//! the screens' asks from one queue, as it does the commands, and every
//! other ask from what a test tells it, apart from its answer queues and
//! its log of commands applied: a new ask on `apply` there is one field
//! and setter for its answer, and one arm in its ask's match; a new
//! screen's ask is a variant pair in nova-sim, sampled in
//! `fixture::sample_screen_asks`.

use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, ScreenAnswer, ScreenAsk, StepInput, StepReport,
};
use nova_sim::mission::screens::OFFER_GAP_TICKS;
use nova_sim::pilot_panel::PilotPanel;
use nova_sim::{
    HailView, HireList, Market, MissionId, MissionInfo, News, Outfitter, PlayerInfo, PlunderView,
    ShipId, ShipNaming, ShipRefusal, Shipyard,
};

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// The flight the view shows, as the view needs it, in five methods: the
/// frame, the steps, the commands (the asks among them), the drain and the
/// screens' asks (see the module docs). The supertraits hold the comm's,
/// the spaceport's and the pilot's screens' questions.
pub trait FlightSim: CommSource + PortSource + PilotSource {
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
