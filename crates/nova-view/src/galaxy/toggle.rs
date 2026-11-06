//! Plot Course on touch (rdm:phase/key-bindings/phase-16-touch-pointer-gaps):
//! an in-flight map hosts a [`ModifierToggle`] for
//! [`MapCommand::PlotCourse`], so a tap on it then a tap on a system is
//! the press a Shift-click makes (`_DoSystemMap` polls the left Shift,
//! `_KeyCheck(0x38)` @0x13071 and @0x1325e), handed to the owner with its
//! Shift held. It shows while touch was used last on the course and
//! hypergate maps (the dialog look, wherever the map shows: over flight,
//! over the spaceport, from a mission dialog) and Find isn't open. The
//! developer's galaxy viewer plots no course, so it never shows there.
//!
//! The map is drawn in its own 1024 × 768 frame, which flight moves
//! across a widened view; flight tells the map its surroundings through
//! that move (`Input::translated`), so the toggle is placed in the map's
//! frame clear of the dialog ([`dialog_area`]).

use super::dialog::dialog_area;
use super::map::{GalaxyMap, MapMode};
use crate::keymap::{Command, MapCommand};
use crate::ui::touch_aids::ModifierToggle;
use crate::ui::touch_aids::host::AidHost;

/// An in-flight map's toggle, clear of the map's dialog.
pub(super) fn toggle() -> ModifierToggle {
    ModifierToggle::classic(Command::Map(MapCommand::PlotCourse), Some(dialog_area()))
}

impl AidHost for GalaxyMap {
    type Aid = ModifierToggle;

    fn aid(&self) -> &ModifierToggle {
        &self.toggle
    }

    fn aid_mut(&mut self) -> &mut ModifierToggle {
        &mut self.toggle
    }

    fn aid_live(&self) -> bool {
        self.mode() != MapMode::Viewer && !self.finding()
    }
}

impl GalaxyMap {
    /// Plot Course's touch toggle: where it is, whether it shows, and the
    /// surroundings it was told.
    #[must_use]
    pub fn touch_toggle(&self) -> &ModifierToggle {
        &self.toggle
    }
}

/// An in-flight map as the toggles' table drives it.
#[cfg(test)]
struct Probe(GalaxyMap);

#[cfg(test)]
impl crate::ui::touch_aids::tests::ToggleProbe for Probe {
    fn screen(&mut self) -> &mut dyn crate::screen::Screen {
        &mut self.0
    }

    fn toggle(&self) -> &ModifierToggle {
        &self.0.toggle
    }

    fn target(&self) -> crate::geometry::Point {
        tests::dot(&self.0, 129)
    }

    fn holds(&mut self) -> bool {
        self.0.take_press().is_some_and(|press| press.shift)
    }
}

/// The course map's row in the toggles' table.
#[cfg(test)]
pub(crate) fn toggle_probe() -> Box<dyn crate::ui::touch_aids::tests::ToggleProbe> {
    Box::new(Probe(tests::map()))
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::draw::DrawList;
    use crate::galaxy::buttons::{MapButton, MapButtons};
    use crate::galaxy::catalog::SystemId;
    use crate::galaxy::dialog::DIALOG_ORIGIN;
    use crate::galaxy::testkit::{FakeCatalog, galaxy};
    use crate::geometry::Point;
    use crate::input::{Device, Input, Key, MouseButton, Surroundings};
    use crate::screen::Screen;
    use crate::text::fixture::MonoMetrics;
    use crate::touch::testkit::Fingers;
    use crate::ui::button::ButtonStyle;
    use crate::ui::touch_aids::testkit::{OTHER_DEVICES, drawn, toggle_then_taps};

    fn told(device: Device) -> Input {
        Input::Surroundings(Surroundings::of(device))
    }

    /// The course map in Alpha (128) with a course to Beta (129), looking
    /// at Gamma, with Find.
    pub(super) fn map() -> GalaxyMap {
        let mut map = GalaxyMap::course(&FakeCatalog::new(galaxy()))
            .with_metrics(Rc::new(MonoMetrics))
            .with_find(None, ButtonStyle::STOCK);
        map.show_course(SystemId(128), &[SystemId(129)]);
        map.opened();
        map.centre_on_system(SystemId(129));
        map
    }

    pub(super) fn dot(map: &GalaxyMap, id: i16) -> Point {
        let system = map.model().system(SystemId(id)).expect("a system");
        map.transform().world_to_screen(system.position())
    }

    fn toggle_at(map: &GalaxyMap) -> Point {
        map.touch_toggle().rect().center()
    }

    #[test]
    fn the_toggle_then_a_tap_on_a_system_is_pressed_as_a_shift_click_is() {
        let mut toggled = map();
        let beta = dot(&toggled, 129);
        toggle_then_taps(&mut toggled, toggle_at, &[]);
        let mut fingers = Fingers::on(&mut toggled);
        fingers.down(1, beta);
        let by_touch = fingers.screen().take_press();
        fingers.up(1, beta);
        let mut keyed = map();
        keyed.input(&Input::Key {
            key: Key::ShiftLeft,
            pressed: true,
            repeat: false,
        });
        keyed.input(&Input::PointerMoved(beta));
        keyed.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: beta,
        });
        let by_key = keyed.take_press();
        assert_eq!(by_touch, by_key);
        assert!(by_touch.expect("held for the owner").shift, "Shift held");
        assert!(!toggled.touch_toggle().armed(), "one tap");
    }

    #[test]
    fn the_toggle_hides_while_find_is_open() {
        let mut map = map();
        map.input(&told(Device::Touch));
        assert!(map.touch_toggle().shown());
        let find = MapButtons::new(DIALOG_ORIGIN)
            .rect(MapButton::Find)
            .center();
        for pressed in [true, false] {
            map.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: find,
            });
        }
        assert!(map.finding());
        let mut alone = DrawList::new();
        map.touch_toggle().draw(&mut alone);
        assert!(!alone.is_empty());
        let drawn = drawn(&map);
        assert!(alone.iter().all(|command| !drawn.contains(command)));
        map.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert!(!map.touch_toggle().shown(), "and hidden at the next input");
    }

    #[test]
    fn the_viewer_has_no_toggle() {
        let mut viewer = GalaxyMap::new(&FakeCatalog::new(galaxy()));
        let before = drawn(&viewer);
        viewer.input(&told(Device::Touch));
        assert!(!viewer.touch_toggle().shown());
        assert_eq!(drawn(&viewer), before);
    }

    #[test]
    fn the_toggle_is_placed_clear_of_the_maps_dialog() {
        let mut map = map();
        map.input(&told(Device::Touch));
        let rect = map.touch_toggle().rect();
        assert!(crate::geometry::intersection(rect, dialog_area()).is_none());
    }

    #[test]
    fn the_toggle_is_drawn_only_while_touch_was_used_last() {
        let mut touched = map();
        touched.input(&told(Device::Touch));
        let mut alone = DrawList::new();
        touched.touch_toggle().draw(&mut alone);
        assert!(!alone.is_empty());
        let all = drawn(&touched);
        assert_eq!(
            all[all.len() - alone.len()..],
            alone.iter().cloned().collect::<Vec<_>>()[..]
        );
        for device in OTHER_DEVICES {
            let mut map = map();
            map.input(&told(Device::Touch));
            map.input(&told(device));
            let mut bare = self::map();
            bare.input(&told(device));
            assert_eq!(drawn(&map), drawn(&bare), "{device:?}");
        }
    }

    #[test]
    fn a_click_where_the_toggle_would_be_does_what_it_did() {
        let mut touched = map();
        touched.input(&told(Device::Touch));
        let at = toggle_at(&touched);
        let click = |map: &mut GalaxyMap| {
            map.input(&told(Device::Mouse));
            for pressed in [true, false] {
                map.input(&Input::PointerButton {
                    button: MouseButton::Left,
                    pressed,
                    at,
                });
            }
            (map.take_press(), map.selected(), drawn(map))
        };
        let mut never = map();
        assert_eq!(click(&mut touched), click(&mut never));
    }
}
