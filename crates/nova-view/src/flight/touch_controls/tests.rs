//! The overlay's roles and queue, with a desktop window told by touch.

#![allow(clippy::float_cmp)]

use super::*;
use crate::geometry::Bounds;
use crate::keymap::command::PlayCommand;

const VIEW: (f32, f32) = (1024.0, 768.0);

/// An overlay showing over live flight in a desktop window.
fn showing() -> TouchControls {
    let mut controls = TouchControls::default();
    controls.set_view(VIEW);
    controls.tell(Surroundings::of(Device::Touch));
    controls.set_live(true);
    controls
}

fn finger(n: usize) -> FingerId {
    FingerId::new(n)
}

fn touch(controls: &mut TouchControls, n: usize, phase: TouchPhase) -> Vec<Input> {
    controls.touch(Touch {
        finger: finger(n),
        phase,
    });
    controls.take_queued()
}

fn rect_of(controls: &TouchControls, id: ControlId) -> Bounds {
    controls.layout().get(id).expect("placed").rect
}

fn centre(controls: &TouchControls, id: ControlId) -> Point {
    rect_of(controls, id).center()
}

const FIRE: Command = Command::Flight(FlightCommand::FirePrimary);
const REVERSE: Command = Command::Flight(FlightCommand::ReverseCourse);

fn fire_at(controls: &TouchControls) -> Point {
    centre(controls, ControlId::Command(FIRE))
}

#[test]
fn a_touch_in_the_stick_zone_claims_and_starts_a_stick_where_it_lands() {
    let mut controls = showing();
    let at = centre(&controls, ControlId::Stick);
    assert!(controls.claims(at));
    assert_eq!(touch(&mut controls, 1, TouchPhase::Started(at)), []);
    assert_eq!(controls.stick(), Some((at, at)));
}

#[test]
fn moving_the_stick_queues_its_offset() {
    let mut controls = showing();
    let at = centre(&controls, ControlId::Stick);
    touch(&mut controls, 1, TouchPhase::Started(at));
    let radius = controls.stick_radius();
    let to = Point::new(at.x + radius / 2.0, at.y);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Moved(to)),
        [stick_input(0.5, 0.0)]
    );
    assert_eq!(controls.stick(), Some((at, to)));
}

#[test]
fn lifting_the_stick_centres_it_and_lets_go_of_reverse() {
    let mut controls = showing();
    let at = centre(&controls, ControlId::Stick);
    touch(&mut controls, 1, TouchPhase::Started(at));
    let radius = controls.stick_radius();
    let back = Point::new(at.x, at.y + radius);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Moved(back)),
        [stick_input(0.0, -1.0), button(REVERSE, true)]
    );
    assert!(controls.holds(REVERSE));
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Ended(back)),
        [stick_input(0.0, 0.0), button(REVERSE, false)]
    );
    assert_eq!(controls.stick(), None);
    assert!(!controls.holds(REVERSE));
}

#[test]
fn a_touch_on_a_button_presses_its_command_and_its_lift_releases_it() {
    let mut controls = showing();
    let at = fire_at(&controls);
    assert!(controls.claims(at));
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Started(at)),
        [button(FIRE, true)]
    );
    assert!(controls.holds(FIRE));
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Ended(at)),
        [button(FIRE, false)]
    );
}

#[test]
fn a_finger_sliding_off_its_control_keeps_it_until_it_lifts() {
    let mut controls = showing();
    let at = fire_at(&controls);
    touch(&mut controls, 1, TouchPhase::Started(at));
    let off = Point::new(10.0, 10.0);
    assert_eq!(touch(&mut controls, 1, TouchPhase::Moved(off)), []);
    assert!(controls.holds(FIRE), "still held off the button");
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Ended(off)),
        [button(FIRE, false)]
    );
    let zone = centre(&controls, ControlId::Stick);
    touch(&mut controls, 2, TouchPhase::Started(zone));
    let far = Point::new(800.0, 100.0);
    let moved = touch(&mut controls, 2, TouchPhase::Moved(far));
    let Some(&Input::TouchControl(TouchControl::Stick { x, y })) = moved.first() else {
        panic!("{moved:?}");
    };
    assert!((x.hypot(y) - 1.0).abs() < 1e-6, "clamped to the ring");
}

#[test]
fn a_cancelled_touch_on_fire_releases_fire() {
    let mut controls = showing();
    let at = fire_at(&controls);
    touch(&mut controls, 1, TouchPhase::Started(at));
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Cancelled),
        [button(FIRE, false)]
    );
    assert!(!controls.holds(FIRE));
}

#[test]
fn two_fingers_on_one_button_hold_it_until_both_lift() {
    let mut controls = showing();
    let at = fire_at(&controls);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Started(at)),
        [button(FIRE, true)]
    );
    assert_eq!(touch(&mut controls, 2, TouchPhase::Started(at)), []);
    assert_eq!(touch(&mut controls, 1, TouchPhase::Ended(at)), []);
    assert!(controls.holds(FIRE));
    assert_eq!(
        touch(&mut controls, 2, TouchPhase::Ended(at)),
        [button(FIRE, false)]
    );
}

#[test]
fn the_stick_and_fire_work_at_once() {
    let mut controls = showing();
    let zone = centre(&controls, ControlId::Stick);
    let fire = fire_at(&controls);
    touch(&mut controls, 1, TouchPhase::Started(zone));
    assert_eq!(
        touch(&mut controls, 2, TouchPhase::Started(fire)),
        [button(FIRE, true)]
    );
    let radius = controls.stick_radius();
    let right = Point::new(zone.x + radius, zone.y);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Moved(right)),
        [stick_input(1.0, 0.0)]
    );
    assert!(controls.holds(FIRE));
    let second = touch(&mut controls, 3, TouchPhase::Started(zone));
    assert_eq!(second, [], "a second finger on the zone is no stick");
    assert_eq!(touch(&mut controls, 3, TouchPhase::Moved(right)), []);
}

#[test]
fn more_opens_the_menu_and_a_menu_button_presses_then_releases_and_closes_on_lift() {
    let mut controls = showing();
    let more = centre(&controls, ControlId::More);
    assert_eq!(touch(&mut controls, 1, TouchPhase::Started(more)), []);
    assert!(!controls.menu_open(), "opens on the lift");
    assert_eq!(touch(&mut controls, 1, TouchPhase::Ended(more)), []);
    assert!(controls.menu_open());
    let info = ControlId::Command(PlayCommand::PlayerInfo.into());
    let cell = centre(&controls, info);
    assert_eq!(
        touch(&mut controls, 2, TouchPhase::Started(cell)),
        [button(PlayCommand::PlayerInfo.into(), true)]
    );
    assert!(controls.menu_open(), "open while held");
    assert_eq!(
        touch(&mut controls, 2, TouchPhase::Ended(cell)),
        [button(PlayCommand::PlayerInfo.into(), false)]
    );
    assert!(!controls.menu_open());
    touch(&mut controls, 3, TouchPhase::Started(more));
    touch(&mut controls, 3, TouchPhase::Ended(more));
    touch(&mut controls, 3, TouchPhase::Started(more));
    touch(&mut controls, 3, TouchPhase::Ended(more));
    assert!(!controls.menu_open(), "More again closes it");
}

#[test]
fn a_cancel_on_more_or_a_page_arrow_does_nothing() {
    let mut controls = showing();
    let more = centre(&controls, ControlId::More);
    touch(&mut controls, 1, TouchPhase::Started(more));
    assert_eq!(touch(&mut controls, 1, TouchPhase::Cancelled), []);
    assert!(!controls.menu_open());
    let mut small = small_window();
    open_menu(&mut small);
    let next = centre(&small, ControlId::PageNext);
    let before = small.layout();
    touch(&mut small, 2, TouchPhase::Started(next));
    touch(&mut small, 2, TouchPhase::Cancelled);
    assert_eq!(small.layout(), before, "the same page");
}

/// An overlay showing in a window small enough that the menu pages.
fn small_window() -> TouchControls {
    let mut controls = showing();
    controls.tell(Surroundings {
        points_per_pixel: 0.3125,
        ..Surroundings::of(Device::Touch)
    });
    assert!(controls.layout().pages > 1);
    controls
}

fn open_menu(controls: &mut TouchControls) {
    let more = centre(controls, ControlId::More);
    touch(controls, 9, TouchPhase::Started(more));
    touch(controls, 9, TouchPhase::Ended(more));
    assert!(controls.menu_open());
}

/// The commands on the open menu's page.
fn page(controls: &TouchControls) -> Vec<Command> {
    let layout = controls.layout();
    let panel = layout.panel.expect("open");
    layout
        .placed
        .iter()
        .filter(|placed| panel.contains(placed.rect.center()))
        .filter_map(|placed| match placed.id {
            ControlId::Command(command) => Some(command),
            _ => None,
        })
        .collect()
}

#[test]
fn the_menus_background_closes_it_on_lift() {
    let mut controls = showing();
    open_menu(&mut controls);
    let panel = controls.layout().panel.expect("open");
    let empty = Point::new(panel.max.x - 2.0, panel.max.y - 2.0);
    assert!(controls.claims(empty));
    assert_eq!(controls.layout().hit(empty), None);
    touch(&mut controls, 1, TouchPhase::Started(empty));
    assert!(controls.menu_open());
    touch(&mut controls, 1, TouchPhase::Ended(empty));
    assert!(!controls.menu_open());
}

#[test]
fn the_page_arrows_turn_the_page() {
    let mut controls = small_window();
    open_menu(&mut controls);
    let first = page(&controls);
    let next = centre(&controls, ControlId::PageNext);
    touch(&mut controls, 1, TouchPhase::Started(next));
    touch(&mut controls, 1, TouchPhase::Ended(next));
    let second = page(&controls);
    assert_ne!(second, first);
    let prev = centre(&controls, ControlId::PagePrev);
    touch(&mut controls, 1, TouchPhase::Started(prev));
    touch(&mut controls, 1, TouchPhase::Ended(prev));
    assert_eq!(page(&controls), first);
    touch(&mut controls, 1, TouchPhase::Started(prev));
    touch(&mut controls, 1, TouchPhase::Ended(prev));
    let last = controls.layout();
    let menu = &last.menu;
    assert_eq!(
        page(&controls).last(),
        menu.last(),
        "back from the first wraps to the last"
    );
}

#[test]
fn while_hidden_it_claims_nothing() {
    let mut controls = showing();
    let at = fire_at(&controls);
    controls.tell(Surroundings::of(Device::Keyboard));
    assert!(!controls.claims(at));
    assert_eq!(touch(&mut controls, 1, TouchPhase::Started(at)), []);
    let mut dead = showing();
    dead.set_live(false);
    assert!(!dead.claims(at));
    let mut unset = TouchControls::default();
    unset.set_view(VIEW);
    unset.tell(Surroundings::of(Device::Touch));
    assert!(!unset.claims(at), "not live until told");
}

#[test]
fn hiding_releases_everything_held() {
    for hide in [
        (|c: &mut TouchControls| c.tell(Surroundings::of(Device::Mouse))) as fn(&mut TouchControls),
        |c| c.set_live(false),
    ] {
        let mut controls = showing();
        let at = fire_at(&controls);
        touch(&mut controls, 1, TouchPhase::Started(at));
        hide(&mut controls);
        assert_eq!(controls.take_queued(), [button(FIRE, false)]);
        assert!(!controls.holds(FIRE));
    }
}

#[test]
fn let_go_releases_everything_and_forgets_its_fingers() {
    let mut controls = showing();
    let zone = centre(&controls, ControlId::Stick);
    let fire = fire_at(&controls);
    touch(&mut controls, 1, TouchPhase::Started(zone));
    let radius = controls.stick_radius();
    touch(
        &mut controls,
        1,
        TouchPhase::Moved(Point::new(zone.x, zone.y + radius)),
    );
    touch(&mut controls, 2, TouchPhase::Started(fire));
    open_menu(&mut controls);
    controls.let_go();
    let mut queued = controls.take_queued();
    queued.sort_by_key(|input| format!("{input:?}"));
    let mut expected = vec![
        stick_input(0.0, 0.0),
        button(FIRE, false),
        button(REVERSE, false),
    ];
    expected.sort_by_key(|input| format!("{input:?}"));
    assert_eq!(queued, expected);
    assert!(!controls.menu_open());
    assert_eq!(controls.stick(), None);
    assert_eq!(touch(&mut controls, 1, TouchPhase::Ended(zone)), []);
    assert_eq!(touch(&mut controls, 2, TouchPhase::Ended(fire)), []);
    controls.let_go();
    assert_eq!(controls.take_queued(), [], "nothing left to let go of");
}

#[test]
fn events_of_a_finger_it_does_not_know_queue_nothing() {
    let mut controls = showing();
    let at = fire_at(&controls);
    assert_eq!(touch(&mut controls, 5, TouchPhase::Moved(at)), []);
    assert_eq!(touch(&mut controls, 5, TouchPhase::Ended(at)), []);
    assert_eq!(touch(&mut controls, 5, TouchPhase::Cancelled), []);
    touch(&mut controls, 1, TouchPhase::Started(at));
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Started(at)),
        [],
        "already down"
    );
}

#[test]
fn a_stick_held_back_stays_on_reverse_as_it_moves_and_lets_go_once_forward() {
    let mut controls = showing();
    let at = centre(&controls, ControlId::Stick);
    touch(&mut controls, 1, TouchPhase::Started(at));
    let radius = controls.stick_radius();
    touch(
        &mut controls,
        1,
        TouchPhase::Moved(Point::new(at.x, at.y + radius)),
    );
    let still_back = Point::new(at.x + 5.0, at.y + 0.8 * radius);
    let (x, y) = stick::offset(at, still_back, radius);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Moved(still_back)),
        [stick_input(x, y)],
        "still back: nothing let go"
    );
    assert!(controls.holds(REVERSE));
    let forward = touch(
        &mut controls,
        1,
        TouchPhase::Moved(Point::new(at.x, at.y - radius)),
    );
    assert_eq!(forward, [stick_input(0.0, 1.0), button(REVERSE, false)]);
    let ahead = Point::new(at.x, at.y - radius / 2.0);
    let (x, y) = stick::offset(at, ahead, radius);
    assert_eq!(
        touch(&mut controls, 1, TouchPhase::Moved(ahead)),
        [stick_input(x, y)],
        "forward again: nothing"
    );
}

#[test]
fn the_stick_radius_is_its_points_in_logical_units() {
    let mut controls = showing();
    assert_eq!(controls.stick_radius(), layout::STICK_RADIUS_PT);
    controls.tell(Surroundings {
        points_per_pixel: 2.0,
        ..Surroundings::of(Device::Touch)
    });
    assert_eq!(controls.stick_radius(), layout::STICK_RADIUS_PT / 2.0);
}

#[test]
fn the_escort_groups_are_placed_while_flight_says_its_escort_menu_shows() {
    let mut controls = showing();
    let group = ControlId::Command(crate::keymap::EscortMenuCommand::Group1.into());
    assert!(controls.layout().get(group).is_none());
    controls.set_escort_menu(true);
    assert!(controls.layout().get(group).is_some());
    controls.set_escort_menu(false);
    assert!(controls.layout().get(group).is_none());
}

#[test]
fn a_cancel_on_the_back_arrow_or_the_menus_background_does_nothing() {
    let mut small = small_window();
    open_menu(&mut small);
    let next = centre(&small, ControlId::PageNext);
    touch(&mut small, 1, TouchPhase::Started(next));
    touch(&mut small, 1, TouchPhase::Ended(next));
    let second = page(&small);
    let prev = centre(&small, ControlId::PagePrev);
    touch(&mut small, 2, TouchPhase::Started(prev));
    touch(&mut small, 2, TouchPhase::Cancelled);
    assert_eq!(page(&small), second, "the same page");
    let panel = small.layout().panel.expect("open");
    let empty = Point::new(panel.max.x - 2.0, panel.max.y - 2.0);
    assert_eq!(small.layout().hit(empty), None);
    touch(&mut small, 3, TouchPhase::Started(empty));
    touch(&mut small, 3, TouchPhase::Cancelled);
    assert!(small.menu_open(), "still open");
}

#[test]
fn the_next_arrow_from_the_last_page_wraps_to_the_first() {
    let mut controls = small_window();
    open_menu(&mut controls);
    let first = page(&controls);
    let pages = controls.layout().pages;
    let next = centre(&controls, ControlId::PageNext);
    for _ in 0..pages {
        touch(&mut controls, 1, TouchPhase::Started(next));
        touch(&mut controls, 1, TouchPhase::Ended(next));
    }
    assert_eq!(page(&controls), first);
    touch(&mut controls, 1, TouchPhase::Started(next));
    touch(&mut controls, 1, TouchPhase::Ended(next));
    assert_ne!(page(&controls), first, "and on from there");
}
