//! A press held on the mission computer's list ends as mission info or
//! player info opens over it: the overlay takes the release, so the list
//! is let go of before it is covered, as the course map's opening lets go
//! (`open_port_map` in `nova::app`). Keys the original holds back during
//! the press are left out (rdm:phase/play-feel-8-fidelity/phase-8-what-play-feel-8-phase-10-left-out).

use super::*;
use crate::spaceport::missions::MissionComputerScreen;
use crate::ui::row_list::RowList;

/// `count` offers, 130 on.
fn many_offers(count: i16) -> MissionComputer {
    MissionComputer {
        rows: (0..count)
            .map(|n| ComputerRow {
                mission: MissionId(130 + n),
                name: format!("Offer {n}"),
                raw_name: format!("Offer {n}"),
                text: format!("Text {n}."),
            })
            .collect(),
        today: String::new(),
    }
}

fn button(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

/// The spaceport's computer open on 20 offers, a press on its first row
/// dragged below its list (item 2, (10, 30) to (110, 150), ten rows), so
/// it autoscrolls.
fn dragging_below() -> SpaceportView {
    let mut view = informed(missioned());
    click_item(&mut view, 10);
    view.open_mission_computer(Ok(many_offers(20)));
    view.input(&button(true, Point::new(50.0, 35.0)));
    view.input(&Input::PointerMoved(Point::new(50.0, 170.0)));
    assert_eq!(rows(&view).map(RowList::first), Some(1), "autoscrolled");
    assert_eq!(rows(&view).map(RowList::tracking), Some(true));
    view
}

fn rows(view: &SpaceportView) -> Option<&RowList> {
    view.mission_computer()
        .and_then(MissionComputerScreen::rows)
}

/// Ticks `view` a second.
fn a_second(view: &mut SpaceportView) {
    for _ in 0..60 {
        view.tick(Duration::from_secs(1) / 60);
    }
}

#[test]
fn mission_info_opened_over_a_dragged_computer_list_lets_go_of_it() {
    let mut view = dragging_below();
    view.open_mission_info(actives(2));
    assert_eq!(rows(&view).map(RowList::tracking), Some(false), "let go");
    view.input(&button(false, Point::new(50.0, 170.0)));
    view.input(&key(Key::Escape));
    assert!(view.mission_info().is_none(), "closed");
    a_second(&mut view);
    assert_eq!(rows(&view).map(RowList::first), Some(1), "no scroll after");
}

#[test]
fn player_info_opened_over_a_dragged_computer_list_lets_go_of_it() {
    let mut view = dragging_below();
    view.open_player_info(pilot_info());
    assert_eq!(rows(&view).map(RowList::tracking), Some(false), "let go");
    view.input(&button(false, Point::new(50.0, 170.0)));
    a_second(&mut view);
    assert_eq!(rows(&view).map(RowList::first), Some(1), "no scroll after");
}

#[test]
fn player_info_opened_over_mission_info_lets_go_of_its_list() {
    let mut view = informed(missioned());
    let rows = (0..10)
        .map(|n| InfoRow {
            slot: n,
            name: format!("Mission {n}"),
            failed: false,
            text: String::new(),
            abortable: true,
        })
        .collect();
    view.open_mission_info(MissionInfo::shown(rows, ""));
    let list = |view: &SpaceportView| {
        view.mission_info()
            .and_then(MissionInfoScreen::rows)
            .map(|rows| (rows.first(), rows.tracking()))
    };
    let at = view
        .mission_info()
        .and_then(MissionInfoScreen::rows)
        .and_then(RowList::bar)
        .map(|bar| Point::new(bar.center().x, bar.max.y - 4.0))
        .expect("a bar");
    view.input(&button(true, at));
    assert_eq!(list(&view), Some((1, true)), "the down arrow held");
    view.open_player_info(pilot_info());
    assert_eq!(list(&view), Some((1, false)), "let go");
}
