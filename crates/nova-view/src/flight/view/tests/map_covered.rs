//! The course map's Plot Course toggle under what covers flight
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): flight passes
//! its holder's word on to its map, so the toggle hides and disarms while
//! a dialog of the app's is in front of the map.

use super::*;
use crate::input::{Device, Surroundings};
use crate::touch::testkit::Fingers;

#[test]
fn covered_flight_hides_and_disarms_its_maps_toggle() {
    let mut view = faked_sol(in_sol());
    view.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
    view.open_map();
    assert!(view.course_map().touch_toggle().shown(), "shown on the map");
    let at = view.course_map().touch_toggle().rect().center();
    {
        let mut fingers = Fingers::on(&mut view);
        fingers.down(1, at);
        fingers.up(1, at);
    }
    assert!(view.course_map().touch_toggle().armed(), "armed");
    view.set_covered(true);
    assert!(!view.course_map().touch_toggle().shown(), "hidden");
    assert!(!view.course_map().touch_toggle().armed(), "disarmed");
    view.set_covered(false);
    assert!(view.course_map().touch_toggle().shown(), "back");
}
