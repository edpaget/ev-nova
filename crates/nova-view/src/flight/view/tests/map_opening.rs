//! The map's opening sends [`FlightCommand::OpenSystemMap`], the tidy of
//! `_DoSystemMap`'s opening (@0x12338-0x12394), before its ask
//! ([`ScreenAsk::MapOpening`]), whichever opener opens it, so Clear Route
//! reads the course the tidy left (@0x12396-0x123c3).

use super::*;
use crate::flight::fake_catalog::catalog;
use crate::flight::view::dialog_map::MapOpener;
use nova_sim::flight_frame::{FlightEvent, StepInput, StepReport};

/// A flight that notes, in one list, each command applied and each
/// screen asked, in order, answering every command done and every screen
/// nothing; the tidy leaves no course plotted, as it does a start with
/// one hop and no start.
struct Tidies {
    frame: FlightFrame,
    log: Vec<String>,
}

impl FlightSim for Tidies {
    fn frame(&self, into: &mut FlightFrame) {
        into.clone_from(&self.frame);
    }

    fn step(&mut self, _input: &StepInput, _steps: u32, into: &mut StepReport) {
        into.clear();
    }

    fn apply(&mut self, command: FlightCommand) -> Answer {
        if command == FlightCommand::OpenSystemMap {
            self.frame.course_plotted = false;
        }
        self.log.push(format!("{command:?}"));
        Answer::Done
    }

    fn drain(&mut self, _into: &mut Vec<FlightEvent>) {}

    fn screen(&mut self, ask: ScreenAsk) -> ScreenAnswer {
        if matches!(ask, ScreenAsk::MapOpening { .. }) {
            self.log.push("MapOpening".to_owned());
        }
        ScreenAnswer::Nothing
    }
}

/// A view over [`Tidies`], in Sol with a course plotted.
fn tidied_view() -> FlightView<FakeCatalog, Tidies> {
    let sim = Tidies {
        frame: FlightFrame {
            course_plotted: true,
            ..in_sol()
        },
        log: Vec::new(),
    };
    FlightView::scripted(catalog(), sim, Names::default(), &Rulebook::default())
}

/// Checks `view`'s map opened with the tidy sent first, and Clear Route
/// read after it.
fn opened_tidied(view: &FlightView<FakeCatalog, Tidies>, opener: &str) {
    assert!(view.map_open, "{opener}");
    assert_eq!(
        view.sim.log[..2],
        ["OpenSystemMap".to_owned(), "MapOpening".to_owned()],
        "{opener}"
    );
    assert!(
        !view.map.clear_route_enabled(),
        "{opener}: Clear Route reads the course the tidy left"
    );
}

#[test]
fn the_course_maps_opening_tidies_the_course_before_its_ask() {
    let mut view = tidied_view();
    view.open_map();
    opened_tidied(&view, "flight");
}

#[test]
fn a_dialogs_map_opening_tidies_the_course_before_its_ask() {
    let mut view = tidied_view();
    view.open_dialog_map(MapOpener::Offer(MissionId(130)));
    opened_tidied(&view, "an offer");
}

#[test]
fn the_bars_map_opening_tidies_the_course_before_its_ask() {
    let mut view = tidied_view();
    view.open_bar_map();
    opened_tidied(&view, "the bar");
}

#[test]
fn a_faked_maps_opening_sends_the_tidy_once() {
    let mut view = faked(in_sol());
    view.open_map();
    assert_eq!(view.sim.applied(), [FlightCommand::OpenSystemMap]);
}
