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
//!   fresh frame to read one fact. Drawing reads `FlightView::drawn`: the
//!   kept frame, or a double-time step's frame between its passes
//!   ([`StepRun::between_passes`](nova_sim::flight_frame::StepRun::between_passes)),
//!   which the view holds until the next step and no draw fills.
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
//!   the readouts, the HUD, and what fades the cursor
//!   ([`FlightFrame::key_fades`](nova_sim::flight_frame::FlightFrame::key_fades)).
//! - **A question a screen asks as it opens or as its input changes is a
//!   screen's ask** ([`ScreenAsk`], answered by the [`ScreenAnswer`] of its
//!   kind through [`FlightSim::screen`]): the course map's galaxy, its
//!   opening, each redraw, its panels after a selection change, its cycle
//!   and Find; the floating map's show; the entry's coming traffic; a
//!   text's fill; and the port's, the comm's and the pilot's screens: the
//!   spaceport's shops, the outfitter alone, the hire list, a ship's name
//!   prompt, the bar's news, what the bar's Gamble opens, the mission info, an offer's refuse button
//!   and its filled text, the hail, the boarding and the pilot panel. The
//!   asks one screen always makes together are one answer (the course
//!   map's opening, [`MapOpen`](nova_sim::flight_frame::MapOpen); its
//!   redraw, [`MapScreen`](nova_sim::flight_frame::MapScreen); the
//!   spaceport's shops, [`PortShops`](nova_sim::flight_frame::PortShops);
//!   the bar's news and its picture,
//!   [`BarNews`](nova_sim::flight_frame::BarNews)). None changes a frame,
//!   which nova-sim pins (`session/screen.rs`) and
//!   `fixture::check_contract` checks, so no fill follows one; and none
//!   is asked per frame, save the floating map's show, which comes on
//!   every frame a key holds it up, as the original's does, and asks each
//!   time, as the flight steps between (`ScreenAsk::FloatingMap`'s docs).
//!   A screen read on every menu frame keeps its answer until the flight
//!   changes (the pilot panel, `FlightView::pilot_panel`). A flight that
//!   failed to start answers each [`ScreenAnswer::Nothing`], and the
//!   screen shows what it would with nothing to show.
//! - **A screen's ask is a read**: asked twice in a row, it answers the
//!   same (`fixture::check_contract`'s "a screen ask is a read"). The
//!   shops' and the bar's asks settle the day's rolls not drawn yet on the
//!   flight's chance, which the flight keeps, as the original keeps each
//!   day's, so asked again they answer from the rolls kept. A ship's name
//!   prompt alone draws its default name afresh at each ask and keeps
//!   nothing ([`ScreenAsk::draws_afresh`]).
//! - **A fact the router reads between commands is a frame field**:
//!   whether Recharge is enabled
//!   ([`FlightFrame::recharge_enabled`](nova_sim::flight_frame::FlightFrame::recharge_enabled))
//!   and whether the pilot has seen the intro
//!   ([`FlightFrame::seen_intro`](nova_sim::flight_frame::FlightFrame::seen_intro)),
//!   which any command may change, and the refill after it keeps current.
//! - **A draw that reads no flight state is the router's**: the wait
//!   before a venue's next offer (`nova_sim::mission::offer_gap`) is drawn
//!   by the router on its own chance, the one the flight shares, not asked
//!   of the flight.
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
//! The port has no supertraits: every screen's question is a screen's
//! ask, and what the edge alone reads (the pilot to save, the saving
//! queries, the setup) is the adapter's own API.
//!
//! # Adding to the port
//!
//! The port grows with data, not methods: each addition is a field or a
//! variant of nova-sim's, answered and pinned on nova-sim's side, and
//! each adapter forward stays one `Session` call. nova-sim's `session`
//! module is private, so its paths here are inline code.
//!
//! - **A readout** is a [`FlightFrame`] field, filled in nova-sim's
//!   `session/frame.rs` (or a module under `session/frame/`) and pinned by
//!   a test there, one test per field, each filling over a stale frame.
//! - **An action** is a [`FlightCommand`] variant, answered with its
//!   [`Answer`] in `session/apply.rs` or a module under `session/apply/`.
//! - **A one-shot output** is a [`FlightEvent`] variant, drained in
//!   `session/flight_events.rs`.
//! - **A screen's question** is a [`ScreenAsk`] and [`ScreenAnswer`]
//!   pair, answered in `session/screen.rs` or a module under
//!   `session/screen/`, pinned there, and sampled in
//!   `fixture::sample_screen_asks`.
//! - **The step's order** lives in `session/step.rs`, and what a step
//!   reports for the view to draw from is a
//!   [`StepRun`](nova_sim::flight_frame::StepRun) or
//!   [`PassReport`](nova_sim::flight_frame::PassReport) field, such as the
//!   frame between double time's passes.
//!
//! New sim-side code for the port goes in those modules, not in
//! `session.rs`. What the edge alone needs goes on the adapter, not the
//! port.
//!
//! # Testing the view
//!
//! View code goes in `impl<C, S: FlightSim> FlightView<C, S>` blocks,
//! names no `Session` and adds no block over `SessionFlight<C>`. The
//! only concrete members are two fixture-only ones
//! (`cfg(any(test, feature = "fixture"))`): `FlightView::session`, which
//! reads a real session behind the port, and `FlightView::from_session`,
//! which wires a view over a session a test built.
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
//!
//! The view's component tests run on `FakeFlight`: they set a frame (the
//! builders in `flight/view/tests/frames.rs`), script its answers and
//! events, and assert what the view draws, plays and sends. A real
//! `Session` starts in nova-view only in:
//!
//! - `tests/it/flight_seam.rs`, one test per port surface or router
//!   handoff, as its module doc says;
//! - the adapter's checks, `sim_adapter`'s tests and
//!   `sim_adapter/fake_catalog_checks.rs`;
//! - the edge's checks, `flight/view/build/edge_checks.rs`;
//! - the `stock_*` tests.
//!
//! The developer tools reach the flight through
//! [`DevDesk`](crate::devtools::DevDesk), not this port.

use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, ScreenAnswer, ScreenAsk, StepInput, StepReport,
};

#[cfg(any(test, feature = "fixture"))]
pub mod fixture;

/// The flight the view shows, as the view needs it, in five methods: the
/// frame, the steps, the commands (the asks among them), the drain and the
/// screens' asks (see the module docs).
pub trait FlightSim {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A flight with nothing to it.
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

    #[test]
    fn a_flight_with_nothing_to_it_fills_the_default_frame_and_refuses_every_command() {
        let mut flight = Defaults;
        let mut frame = crate::flight::sim_port::fixture::unlike(&FlightFrame::default());
        flight.frame(&mut frame);
        assert_eq!(frame, FlightFrame::default());
        assert_eq!(flight.apply(FlightCommand::Jump), Answer::Refused);
    }
}
