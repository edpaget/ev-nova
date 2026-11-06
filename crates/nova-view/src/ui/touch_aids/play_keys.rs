//! [`PlayKeys`]: Map, Pilot and Missions on touch, off flight
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps).
//!
//! The spaceport and its screens read the galaxy map, player info and
//! mission info keys ([`PLAY_KEYS`]) from the keyboard alone, as the
//! original does: `_PortFilter` reads the map key first (@0x4f165), then
//! mission info and player info, and the trade center's, outfitter's,
//! shipyard's, bar's, computer's and offers' filters read theirs; none
//! has a button for them. The pad reaches them through Start and the
//! stick (rdm:phase/key-bindings/phase-9-gamepad-spaceport); this is
//! touch's way.
//!
//! - **Three square controls** in a row, labelled as flight's overlay
//!   labels these commands ("Map", "Pilot", "Missions"), each a menu cell
//!   ([`CELL_PT`]) with a [`GAP_PT`] between, placed as one strip by
//!   [`place`] from the top-right corner on, clear of the spaceport's
//!   dialog, so a modifier toggle in the bottom-left corner never meets
//!   them.
//! - **A tap on one** gives the host the touch device's press and release
//!   of its command ([`TouchControl::Button`]), as the key's press and
//!   release: the screen beneath reads it through its own reader, so each
//!   opens what its key opens there, and does nothing where its key does
//!   nothing (the outfitter ignores player info, as `_OutfitFilter` never
//!   reads it).
//! - **Shown** only while touch was used last and the host is live (the
//!   mission texts, which take every input, hide it); hidden, it passes
//!   every input through.

use super::host::TouchAid;
use super::look::{self, label};
use super::place::{CELL_PT, Corner, place};
use super::press::{Press, Step};
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::input::{Device, Input, Surroundings, TouchControl};
use crate::keymap::{Command, PlayCommand};
use crate::view_shape::CLASSIC_SIZE;

/// The play keys, left to right.
pub const PLAY_KEYS: [Command; 3] = [
    Command::Play(PlayCommand::GalaxyMap),
    Command::Play(PlayCommand::PlayerInfo),
    Command::Play(PlayCommand::MissionInfo),
];

/// The gap between two of its controls, in points, as flight's overlay
/// spaces its own.
pub const GAP_PT: f32 = 8.0;

/// The top corners first, right before left: clear of a modifier
/// toggle, which takes the bottom ones first.
pub const TOP_FIRST: [Corner; 4] = [
    Corner::TopRight,
    Corner::TopLeft,
    Corner::BottomRight,
    Corner::BottomLeft,
];

/// The classic canvas's size, in logical units.
#[allow(clippy::cast_precision_loss)]
const CLASSIC: (f32, f32) = (CLASSIC_SIZE.0 as f32, CLASSIC_SIZE.1 as f32);

/// The play keys' strip: see the module docs.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayKeys {
    /// The host's canvas.
    canvas: Bounds,
    /// The host's dialog, which the strip keeps clear of when it can.
    occupied: Bounds,
    /// The surroundings the host was told last.
    told: Surroundings,
    /// Whether the host lets it show.
    live: bool,
    /// A press on one of its controls.
    press: Press,
    /// Whether its host is running the inputs it gave back.
    replaying: bool,
}

impl PlayKeys {
    /// The strip over a host in the classic 1024 × 768 canvas, with its
    /// dialog at `dialog` if it has one; live, hidden until touch is told.
    #[must_use]
    pub fn classic(dialog: Option<Bounds>) -> Self {
        let none = Bounds::at(Point::new(0.0, 0.0), 0.0, 0.0);
        Self {
            canvas: Bounds::at(Point::new(0.0, 0.0), CLASSIC.0, CLASSIC.1),
            occupied: dialog.unwrap_or(none),
            told: Surroundings::default(),
            live: true,
            press: Press::default(),
            replaying: false,
        }
    }

    /// Each control's rect, in [`PLAY_KEYS`]' order.
    #[must_use]
    pub fn rects(&self) -> [Bounds; 3] {
        let width = CELL_PT.0 * 3.0 + GAP_PT * 2.0;
        let strip = place(
            (width, CELL_PT.1),
            &self.told,
            self.canvas,
            self.occupied,
            &TOP_FIRST,
        );
        let ppp = self.told.points_per_pixel;
        let (cell, gap) = (CELL_PT.0 / ppp, GAP_PT / ppp);
        [0.0, 1.0, 2.0].map(|n| {
            Bounds::at(
                Point::new(strip.min.x + n * (cell + gap), strip.min.y),
                cell,
                strip.height(),
            )
        })
    }

    /// The rect of `command`'s control, if it is one of [`PLAY_KEYS`].
    #[must_use]
    pub fn rect(&self, command: Command) -> Option<Bounds> {
        let index = PLAY_KEYS.iter().position(|&each| each == command)?;
        Some(self.rects()[index])
    }

    /// Whether it shows: touch used last, and the host live.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.live && self.told.device == Device::Touch
    }

    /// The surroundings it was told last.
    #[must_use]
    pub const fn told(&self) -> Surroundings {
        self.told
    }

    /// The touch device's press or release of `command`.
    const fn button(command: Command, pressed: bool) -> Input {
        Input::TouchControl(TouchControl::Button { command, pressed })
    }
}

impl TouchAid for PlayKeys {
    fn input(&mut self, input: &Input) -> Vec<Input> {
        if let Input::Surroundings(told) = *input {
            self.told = told;
            if told.device != Device::Touch {
                self.press.cancel();
            }
            return vec![*input];
        }
        let rects = if self.shown() {
            self.rects().to_vec()
        } else {
            Vec::new()
        };
        match self.press.input(input, &rects) {
            Step::Acted(index) => {
                let command = PLAY_KEYS[index];
                vec![Self::button(command, true), Self::button(command, false)]
            }
            Step::Taken => Vec::new(),
            Step::Passed => vec![*input],
        }
    }

    fn cancel(&mut self) -> Vec<Input> {
        self.press.cancel();
        Vec::new()
    }

    fn set_live(&mut self, live: bool) {
        self.live = live;
        if !live {
            self.press.cancel();
        }
    }

    fn draw(&self, list: &mut DrawList) {
        if !self.shown() {
            return;
        }
        let held = self.press.held();
        for (index, (rect, command)) in self.rects().into_iter().zip(PLAY_KEYS).enumerate() {
            let fill = if held == Some(index) {
                look::HIGHLIGHT
            } else {
                look::FILL
            };
            look::draw_square(list, rect, label(command), fill, self.told.points_per_pixel);
        }
    }

    fn replaying(&self) -> bool {
        self.replaying
    }

    fn set_replaying(&mut self, replaying: bool) {
        self.replaying = replaying;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::intersection;
    use crate::input::MouseButton;
    use crate::ui::touch_aids::testkit::{OTHER_DEVICES, WINDOWS};

    /// Stock "Spaceport", 618 × 517 and centred.
    fn port() -> Bounds {
        Bounds::at(Point::new(203.0, 125.0), 618.0, 517.0)
    }

    fn keys() -> PlayKeys {
        let mut keys = PlayKeys::classic(Some(port()));
        keys.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
        keys
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn tap(keys: &mut PlayKeys, at: Point) -> Vec<Input> {
        [
            Input::PointerMoved(at),
            button(true, at),
            button(false, at),
            Input::PointerLeft,
        ]
        .iter()
        .flat_map(|input| keys.input(input))
        .collect()
    }

    fn drawn(keys: &PlayKeys) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        keys.draw(&mut list);
        list.iter().cloned().collect()
    }

    #[test]
    fn a_tap_on_each_gives_its_commands_press_and_release() {
        for command in PLAY_KEYS {
            let mut keys = keys();
            let at = keys.rect(command).expect("one of them").center();
            assert_eq!(
                tap(&mut keys, at),
                [
                    Input::PointerMoved(at),
                    PlayKeys::button(command, true),
                    PlayKeys::button(command, false),
                    Input::PointerLeft,
                ],
                "{command:?}"
            );
        }
    }

    #[test]
    fn it_says_whether_its_host_is_replaying_what_it_gave_back() {
        let mut keys = keys();
        assert!(!keys.replaying());
        keys.set_replaying(true);
        assert!(keys.replaying());
        keys.set_replaying(false);
        assert!(!keys.replaying());
    }

    #[test]
    fn a_press_that_slides_off_and_lifts_does_nothing() {
        let mut keys = keys();
        let at = keys.rects()[0].center();
        keys.input(&button(true, at));
        let off = Point::new(500.0, 400.0);
        assert_eq!(keys.input(&Input::PointerMoved(off)), []);
        assert_eq!(keys.input(&button(false, off)), []);
    }

    #[test]
    fn a_tap_elsewhere_is_the_hosts() {
        let mut keys = keys();
        let at = Point::new(500.0, 400.0);
        assert_eq!(tap(&mut keys, at).len(), 4);
    }

    #[test]
    fn hidden_unless_touch_was_used_last_it_passes_every_input_through() {
        for device in OTHER_DEVICES {
            let mut keys = PlayKeys::classic(Some(port()));
            keys.input(&Input::Surroundings(Surroundings::of(device)));
            let at = keys.rects()[0].center();
            for input in [button(true, at), button(false, at)] {
                assert_eq!(keys.input(&input), [input], "{device:?}");
            }
            assert_eq!(drawn(&keys), [], "{device:?}");
        }
    }

    #[test]
    fn not_live_it_is_hidden_and_takes_nothing() {
        let mut keys = keys();
        keys.set_live(false);
        let at = keys.rects()[0].center();
        assert_eq!(keys.input(&button(true, at)), [button(true, at)]);
        assert_eq!(drawn(&keys), []);
    }

    #[test]
    fn a_cancel_or_another_device_forgets_a_press_on_it() {
        let mut keys = keys();
        let at = keys.rects()[1].center();
        keys.input(&button(true, at));
        assert_eq!(keys.cancel(), []);
        assert_eq!(keys.input(&button(false, at)), [button(false, at)]);
        let mut told = self::keys();
        told.input(&button(true, at));
        told.input(&Input::Surroundings(Surroundings::of(Device::Mouse)));
        assert_eq!(told.input(&button(false, at)), [button(false, at)]);
    }

    #[test]
    fn its_three_controls_are_cells_in_a_row_clear_of_the_spaceport_at_each_size() {
        for window in &WINDOWS {
            let mut keys = PlayKeys::classic(Some(port()));
            let told = window.classic_surroundings();
            keys.input(&Input::Surroundings(told));
            let rects = keys.rects();
            let ppp = told.points_per_pixel;
            for rect in rects {
                assert!(
                    (rect.width() * ppp - CELL_PT.0).abs() < 1e-3,
                    "{}",
                    window.name
                );
                assert!(
                    (rect.height() * ppp - CELL_PT.1).abs() < 1e-3,
                    "{}",
                    window.name
                );
            }
            assert!((rects[1].min.x - rects[0].max.x) * ppp - GAP_PT < 1e-3);
            assert_eq!(rects[0].min.y, rects[2].min.y, "{}: a row", window.name);
            if window.name == "desktop window" {
                assert_eq!(rects[2].max.x, 984.0, "the top-right corner, past the cuts");
                assert_eq!(rects[0].min.y, 32.0);
                for rect in rects {
                    assert!(intersection(rect, port()).is_none());
                }
            }
        }
    }

    #[test]
    fn it_draws_its_controls_as_the_overlays_square_controls_with_their_labels() {
        let mut keys = keys();
        let mut expected = DrawList::new();
        for (rect, text) in keys.rects().into_iter().zip(["Map", "Pilot", "Missions"]) {
            look::draw_square(&mut expected, rect, text, look::FILL, 1.0);
        }
        let expected: Vec<_> = expected.iter().cloned().collect();
        assert_eq!(drawn(&keys), expected);
        let at = keys.rects()[2].center();
        keys.input(&button(true, at));
        let mut held = DrawList::new();
        for (index, (rect, text)) in keys
            .rects()
            .into_iter()
            .zip(["Map", "Pilot", "Missions"])
            .enumerate()
        {
            let fill = if index == 2 {
                look::HIGHLIGHT
            } else {
                look::FILL
            };
            look::draw_square(&mut held, rect, text, fill, 1.0);
        }
        assert_eq!(drawn(&keys), held.iter().cloned().collect::<Vec<_>>());
    }
}
