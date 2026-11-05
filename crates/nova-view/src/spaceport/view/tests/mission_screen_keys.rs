//! Wiring: keys held back by a press on the mission computer or the
//! mission info screen reach the spaceport's routing after the release,
//! and the rest after a key that closes either, or after an accept, goes
//! where the `mission_keys` module says.

use super::*;
use crate::spaceport::missions::MissionComputerScreen;
use crate::spaceport::port_map::PortMap;
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

/// The computer's first row: item 2 is (10, 30) to (110, 150).
const ROW_0: Point = Point::new(50.0, 35.0);

/// The spaceport's computer open on 20 offers.
fn computer() -> SpaceportView {
    let mut view = informed(missioned());
    click_item(&mut view, 10);
    view.open_mission_computer(Ok(many_offers(20)));
    let _ = view.take_computer_text_request();
    view
}

fn selected(view: &SpaceportView) -> Option<usize> {
    view.mission_computer()
        .and_then(MissionComputerScreen::rows)
        .and_then(RowList::selected)
}

/// A press on the computer's first row, `keys` while it is held, then
/// its release.
fn held_on_row_0(view: &mut SpaceportView, keys: &[Input]) {
    view.input(&button(true, ROW_0));
    for input in keys {
        view.input(input);
    }
    view.input(&button(false, ROW_0));
}

#[test]
fn a_key_held_during_a_computer_list_press_acts_after_the_release() {
    let mut view = computer();
    view.input(&button(true, ROW_0));
    view.input(&key(Key::Down));
    assert_eq!(selected(&view), Some(0), "it waits");
    view.input(&button(false, ROW_0));
    assert_eq!(selected(&view), Some(1));
    assert_eq!(view.take_computer_text_request(), Some(MissionId(131)));
}

#[test]
fn the_computers_rest_after_a_held_leave_reaches_the_spaceport() {
    let mut view = computer();
    held_on_row_0(&mut view, &[key(Key::Escape), key(Key::Char('m'))]);
    assert!(view.mission_computer().is_none(), "left");
    assert_eq!(
        view.take_map_request(),
        Some(PortMap::Screen(PortScreen::Spaceport)),
        "_PortFilter read the M (@0xa40c8, @0x60035-0x6004e)"
    );
}

#[test]
fn the_info_screens_rest_after_a_held_close_reaches_the_computer_beneath() {
    let mut view = computer();
    view.open_mission_info(actives(2));
    let at = info_row_0(&view);
    view.input(&button(true, at));
    view.input(&key(Key::Escape));
    view.input(&key(Key::Down));
    assert!(view.mission_info().is_some(), "it waits");
    view.input(&button(false, at));
    assert!(view.mission_info().is_none(), "closed");
    assert_eq!(selected(&view), Some(1), "the Down reached the computer");
}

#[test]
fn keys_held_through_an_accept_are_dropped_when_it_is_taken_and_run_when_the_slots_are_full() {
    for (taken, after) in [(true, Some(0)), (false, Some(1))] {
        let mut view = computer();
        held_on_row_0(&mut view, &[key(Key::Char('a')), key(Key::Down)]);
        assert_eq!(view.take_mission_accept(), Some(MissionId(130)));
        assert_eq!(selected(&view), Some(0), "taken {taken}: it waits");
        assert!(view.holds_back_keys(), "taken {taken}: for the answer");
        view.set_mission_computer(Ok(many_offers(20)), taken);
        assert_eq!(selected(&view), after, "taken {taken}");
        assert!(!view.holds_back_keys(), "taken {taken}");
    }
    let mut texted = computer();
    held_on_row_0(&mut texted, &[key(Key::Char('a')), key(Key::Down)]);
    texted.show_mission_texts(vec![MissionText {
        kind: MissionTextKind::Brief,
        mission: MissionId(130),
        text: "Too little space.".to_owned(),
        graphic: None,
        movie: None,
        map: None,
    }]);
    texted.set_mission_computer(Ok(many_offers(20)), false);
    done(&mut texted);
    assert_eq!(selected(&texted), Some(0), "the text flushed it (@0xa1bb2)");
}

#[test]
fn the_rest_after_an_accept_that_leaves_nothing_reaches_the_spaceport() {
    let mut view = computer();
    held_on_row_0(&mut view, &[key(Key::Char('a')), key(Key::Char('m'))]);
    assert_eq!(view.take_mission_accept(), Some(MissionId(130)));
    view.set_mission_computer(Err(ComputerRefusal::NoneAvailable), true);
    assert!(
        view.mission_computer().is_none(),
        "closed (@0xa380a-0xa3817)"
    );
    assert_eq!(
        view.take_map_request(),
        Some(PortMap::Screen(PortScreen::Spaceport))
    );
}

#[test]
fn a_cancelled_computer_press_whose_tick_leaves_closes_the_computer() {
    let mut view = computer();
    view.input(&button(true, ROW_0));
    view.input(&key(Key::Escape));
    view.input(&key(Key::Char('m')));
    view.cancel_pointer();
    assert!(view.mission_computer().is_some(), "until the tick");
    view.tick(Duration::ZERO);
    assert!(view.mission_computer().is_none(), "left");
    assert_eq!(
        view.take_port_dialog_closed(),
        Some(PortDialog::MissionComputer)
    );
    assert_eq!(
        view.take_map_request(),
        Some(PortMap::Screen(PortScreen::Spaceport))
    );
}

#[test]
fn the_computer_is_told_whether_missions_are_listed() {
    let mut view = computer();
    view.set_computer_missions_listed(false);
    held_on_row_0(&mut view, &[key(MISSION_INFO_KEY), key(Key::Down)]);
    assert!(!view.take_mission_info_request(), "nothing listed");
    assert_eq!(selected(&view), Some(1), "the Down ran on");
    view.set_computer_missions_listed(true);
    held_on_row_0(&mut view, &[key(MISSION_INFO_KEY), key(Key::Down)]);
    assert!(view.take_mission_info_request());
    assert_eq!(
        selected(&view),
        Some(0),
        "row 0 read back, the Down flushed"
    );
}

#[test]
fn an_abort_tells_the_computer_beneath_whether_missions_are_still_listed() {
    let mut view = computer();
    view.open_mission_info(actives(1));
    view.set_mission_info(actives(0));
    assert!(view.mission_info().is_none(), "nothing left");
    held_on_row_0(&mut view, &[key(MISSION_INFO_KEY), key(Key::Down)]);
    assert!(!view.take_mission_info_request());
    assert_eq!(selected(&view), Some(1), "the Down ran on");
}

/// The centre of the info screen's row 0.
fn info_row_0(view: &SpaceportView) -> Point {
    view.mission_info()
        .and_then(MissionInfoScreen::rows)
        .and_then(|rows| rows.row_bounds(0))
        .expect("the info's row 0")
        .center()
}

#[test]
fn the_rest_after_an_abort_reaches_the_computer_beneath_as_the_info_is_set() {
    let mut view = computer();
    view.open_mission_info(actives(2));
    let abort = view
        .mission_info()
        .and_then(MissionInfoScreen::dialog)
        .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
        .expect("Abort")
        .center();
    view.input(&button(true, abort));
    view.input(&key(Key::Escape));
    view.input(&key(Key::Down));
    view.input(&button(false, abort));
    assert_eq!(view.take_mission_abort(), Some(0));
    assert!(view.mission_info().is_some(), "the Escape waits");
    view.set_mission_info(actives(1));
    assert!(view.mission_info().is_none(), "the Escape closed it");
    assert_eq!(
        selected(&view),
        Some(1),
        "the Down reached the computer at once"
    );
}

#[test]
fn an_info_screen_a_cancelled_press_closes_on_its_tick_redraws_the_status_bar_then() {
    let mut view = computer();
    view.open_mission_info(actives(2));
    view.take_status_refresh();
    let at = info_row_0(&view);
    view.input(&button(true, at));
    view.input(&key(Key::Escape));
    view.cancel_pointer();
    assert!(view.mission_info().is_some(), "until the tick");
    view.tick(Duration::ZERO);
    assert!(view.mission_info().is_none(), "closed");
    assert!(view.take_status_refresh(), "on the same tick");
}
