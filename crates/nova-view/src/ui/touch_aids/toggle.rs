//! [`ModifierToggle`]: the on-screen toggle that gives a touch player one
//! of the original's modifier clicks ([`CLICK_MODIFIERS`]).
//!
//! - **Shown** only while touch was the device used last and its host says
//!   it is live (no modal of its own open over it); hidden, it passes every
//!   input through untouched, so a keyboard, mouse or pad player's clicks
//!   are exactly what they were.
//! - **One-shot.** A tap on it arms it, acting on the release inside it; a
//!   second tap disarms it. Armed, the next tap anywhere else is wrapped
//!   in its command's press and release as the touch device's
//!   ([`TouchControl::Button`]): the command's press, the tap's press, its
//!   moves and release, then the command's release. The host runs each
//!   through its own input, so its reader holds the command across the tap
//!   exactly as it holds Option or Shift across a click. It disarms after
//!   that tap, as an Option-click is one click; a sticky toggle would turn
//!   a later plain click into one the player didn't ask for.
//! - **Cancels.** The host's `cancel_pointer` calls [`ModifierToggle::cancel`],
//!   which forgets a press on the toggle and, mid-tap, gives back the
//!   command's release for the host to run after its own cancel. Told any
//!   device but touch, it hides and disarms, giving back the release if a
//!   tap is wrapped.
//! - **Silent**, as phase 15's overlay is: the wrapped tap's sounds are the
//!   click's.
//! - **Looks** like a square control of flight's overlay
//!   ([`look::draw_square`]), a menu cell in size ([`CELL_PT`]), placed by
//!   [`place`] from the bottom-left corner on, highlighted while armed.

use super::look;
use super::place::{BOTTOM_FIRST, CELL_PT, place};
use super::press::{Press, Step};
use crate::draw::DrawList;
use crate::flight::touch_controls::draw::label;
use crate::geometry::Bounds;
use crate::input::{Device, Input, MouseButton, Surroundings, TouchControl};
#[cfg(doc)]
use crate::keymap::CLICK_MODIFIERS;
use crate::keymap::Command;

/// A modifier click's toggle: see the module docs.
#[derive(Clone, Debug, PartialEq)]
pub struct ModifierToggle {
    /// The command it holds across a tap.
    command: Command,
    /// The host's canvas.
    canvas: Bounds,
    /// The host's dialog, which the toggle keeps clear of when it can.
    occupied: Bounds,
    /// The surroundings the host was told last.
    told: Surroundings,
    /// Whether the host lets it show.
    live: bool,
    /// Whether the next tap elsewhere is wrapped.
    armed: bool,
    /// A press on the toggle itself.
    press: Press,
    /// Whether a tap is wrapped now: its command is held.
    wrapping: bool,
}

impl ModifierToggle {
    /// The toggle for `command`, over a host drawing in `canvas` with its
    /// dialog at `occupied`; live, and hidden until touch is told.
    #[must_use]
    pub fn new(command: Command, canvas: Bounds, occupied: Bounds) -> Self {
        Self {
            command,
            canvas,
            occupied,
            told: Surroundings::default(),
            live: true,
            armed: false,
            press: Press::default(),
            wrapping: false,
        }
    }

    /// The command it holds across a tap.
    #[must_use]
    pub const fn command(&self) -> Command {
        self.command
    }

    /// Where it is drawn and taps it, in the host's canvas.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        place(
            CELL_PT,
            &self.told,
            self.canvas,
            self.occupied,
            &BOTTOM_FIRST,
        )
    }

    /// Whether it shows: touch used last, and the host live.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.live && self.told.device == Device::Touch
    }

    /// Whether the next tap elsewhere is wrapped.
    #[must_use]
    pub const fn armed(&self) -> bool {
        self.armed
    }

    /// The surroundings it was told last.
    #[must_use]
    pub const fn told(&self) -> Surroundings {
        self.told
    }

    /// Whether the host lets it show; not live, it disarms (a tap already
    /// wrapped still ends with its release).
    pub fn set_live(&mut self, live: bool) {
        self.live = live;
        if !live {
            self.armed = false;
            self.press.cancel();
        }
    }

    /// Follows `input` ahead of the host: the inputs the host should run
    /// instead, in order (none for an input the toggle took).
    pub fn input(&mut self, input: &Input) -> Vec<Input> {
        if let Input::Surroundings(told) = *input {
            self.told = told;
            let mut out = vec![*input];
            if told.device != Device::Touch {
                self.armed = false;
                self.press.cancel();
                out.extend(self.unwrap());
            }
            return out;
        }
        if self.wrapping {
            return match *input {
                Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: false,
                    ..
                } => {
                    let mut out = vec![*input];
                    out.extend(self.unwrap());
                    out
                }
                _ => vec![*input],
            };
        }
        let rects = if self.shown() {
            vec![self.rect()]
        } else {
            Vec::new()
        };
        match self.press.input(input, &rects) {
            Step::Acted(_) => {
                self.armed = !self.armed;
                return Vec::new();
            }
            Step::Taken => return Vec::new(),
            Step::Passed => {}
        }
        if self.armed
            && self.shown()
            && let Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                ..
            } = *input
        {
            self.wrapping = true;
            return vec![self.button(true), *input];
        }
        vec![*input]
    }

    /// The host's pointer was let go of: a press on the toggle is
    /// forgotten, and a wrapped tap ends, giving back its command's
    /// release for the host to run after its own cancel.
    pub fn cancel(&mut self) -> Vec<Input> {
        self.press.cancel();
        self.unwrap()
    }

    /// Ends a wrapped tap: disarmed, and the command's release, if one was
    /// wrapped.
    fn unwrap(&mut self) -> Vec<Input> {
        if !std::mem::take(&mut self.wrapping) {
            return Vec::new();
        }
        self.armed = false;
        vec![self.button(false)]
    }

    /// The touch device's press or release of the command.
    const fn button(&self, pressed: bool) -> Input {
        Input::TouchControl(TouchControl::Button {
            command: self.command,
            pressed,
        })
    }

    /// Draws it while it shows: highlighted while armed or held.
    pub fn draw(&self, list: &mut DrawList) {
        if !self.shown() {
            return;
        }
        let fill = if self.armed || self.press.held().is_some() {
            look::HIGHLIGHT
        } else {
            look::FILL
        };
        look::draw_square(
            list,
            self.rect(),
            label(self.command),
            fill,
            self.told.points_per_pixel,
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::input::ControllerFamily;
    use crate::keymap::TradeCommand;

    const COMMAND: Command = Command::Trade(TradeCommand::MaxLot);
    const ELSEWHERE: Point = Point::new(500.0, 300.0);

    fn canvas() -> Bounds {
        Bounds::at(Point::new(0.0, 0.0), 1024.0, 768.0)
    }

    fn dialog() -> Bounds {
        Bounds::at(Point::new(212.0, 184.0), 600.0, 400.0)
    }

    fn touch() -> Input {
        Input::Surroundings(Surroundings::of(Device::Touch))
    }

    fn toggle() -> ModifierToggle {
        let mut toggle = ModifierToggle::new(COMMAND, canvas(), dialog());
        toggle.input(&touch());
        toggle
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn command(pressed: bool) -> Input {
        Input::TouchControl(TouchControl::Button {
            command: COMMAND,
            pressed,
        })
    }

    /// A tap at `at` as the router sends it: a move, the press, the
    /// release and the leave; what the host runs.
    fn tap(toggle: &mut ModifierToggle, at: Point) -> Vec<Input> {
        [
            Input::PointerMoved(at),
            button(true, at),
            button(false, at),
            Input::PointerLeft,
        ]
        .iter()
        .flat_map(|input| toggle.input(input))
        .collect()
    }

    fn on(toggle: &ModifierToggle) -> Point {
        toggle.rect().center()
    }

    const OTHERS: [Device; 3] = [
        Device::Keyboard,
        Device::Mouse,
        Device::Gamepad(ControllerFamily::Generic),
    ];

    fn drawn(toggle: &ModifierToggle) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        toggle.draw(&mut list);
        list.iter().cloned().collect()
    }

    #[test]
    fn hidden_unless_touch_was_used_last_it_passes_every_input_through() {
        for device in OTHERS {
            let mut toggle = ModifierToggle::new(COMMAND, canvas(), dialog());
            toggle.input(&Input::Surroundings(Surroundings::of(device)));
            assert!(!toggle.shown(), "{device:?}");
            let at = on(&toggle);
            for input in [
                Input::PointerMoved(at),
                button(true, at),
                button(false, at),
                button(true, ELSEWHERE),
                button(false, ELSEWHERE),
            ] {
                assert_eq!(toggle.input(&input), [input], "{device:?}");
            }
            assert!(!toggle.armed());
        }
    }

    #[test]
    fn a_tap_on_it_arms_it_and_reaches_the_host_as_nothing() {
        let mut toggle = toggle();
        let at = on(&toggle);
        let ran = tap(&mut toggle, at);
        assert_eq!(
            ran,
            [Input::PointerMoved(at), Input::PointerLeft],
            "only the move before and the leave after"
        );
        assert!(toggle.armed());
    }

    #[test]
    fn a_press_on_it_that_slides_off_and_lifts_does_nothing() {
        let mut toggle = toggle();
        let at = on(&toggle);
        toggle.input(&button(true, at));
        toggle.input(&Input::PointerMoved(ELSEWHERE));
        assert_eq!(toggle.input(&button(false, ELSEWHERE)), []);
        assert!(!toggle.armed());
    }

    #[test]
    fn while_its_press_is_held_every_pointer_input_is_its_own() {
        let mut toggle = toggle();
        let at = on(&toggle);
        toggle.input(&button(true, at));
        for input in [
            Input::PointerMoved(ELSEWHERE),
            Input::PointerMovedOutside(Point::new(-5.0, 900.0)),
            Input::PointerLeft,
            Input::PointerMoved(at),
        ] {
            assert_eq!(toggle.input(&input), [], "{input:?}");
        }
    }

    #[test]
    fn armed_the_next_tap_elsewhere_is_wrapped_in_its_commands_press_and_release() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        let moved = Point::new(505.0, 302.0);
        let ran: Vec<Input> = [
            Input::PointerMoved(ELSEWHERE),
            button(true, ELSEWHERE),
            Input::PointerMoved(moved),
            button(false, moved),
            Input::PointerLeft,
        ]
        .iter()
        .flat_map(|input| toggle.input(input))
        .collect();
        assert_eq!(
            ran,
            [
                Input::PointerMoved(ELSEWHERE),
                command(true),
                button(true, ELSEWHERE),
                Input::PointerMoved(moved),
                button(false, moved),
                command(false),
                Input::PointerLeft,
            ]
        );
    }

    #[test]
    fn it_disarms_after_one_wrapped_tap() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        tap(&mut toggle, ELSEWHERE);
        assert!(!toggle.armed());
        assert_eq!(
            tap(&mut toggle, ELSEWHERE),
            [
                Input::PointerMoved(ELSEWHERE),
                button(true, ELSEWHERE),
                button(false, ELSEWHERE),
                Input::PointerLeft,
            ],
            "the next tap is plain"
        );
    }

    #[test]
    fn a_second_tap_on_it_disarms_it() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        assert!(!toggle.armed());
        assert_eq!(tap(&mut toggle, ELSEWHERE).len(), 4, "plain");
    }

    #[test]
    fn a_cancel_mid_tap_gives_the_commands_release_after_the_hosts_cancel() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        toggle.input(&button(true, ELSEWHERE));
        assert_eq!(toggle.cancel(), [command(false)]);
        assert!(!toggle.armed());
        assert_eq!(toggle.cancel(), [], "once");
        assert_eq!(
            toggle.input(&button(false, ELSEWHERE)),
            [button(false, ELSEWHERE)]
        );
        let mut pressing = self::toggle();
        pressing.input(&button(true, on(&pressing)));
        assert_eq!(pressing.cancel(), []);
        let at = on(&pressing);
        assert_eq!(pressing.input(&button(false, at)), [button(false, at)]);
        assert!(!pressing.armed(), "the press forgotten");
    }

    #[test]
    fn another_device_hides_it_disarms_it_and_releases_a_wrapped_command() {
        for device in OTHERS {
            let told = Input::Surroundings(Surroundings::of(device));
            let mut armed = toggle();
            {
                let at = on(&armed);
                tap(&mut armed, at)
            };
            assert_eq!(armed.input(&told), [told], "{device:?}");
            assert!(!armed.armed() && !armed.shown(), "{device:?}");
            let mut wrapping = toggle();
            {
                let at = on(&wrapping);
                tap(&mut wrapping, at)
            };
            wrapping.input(&button(true, ELSEWHERE));
            assert_eq!(wrapping.input(&told), [told, command(false)], "{device:?}");
            wrapping.input(&touch());
            assert!(!wrapping.armed(), "{device:?}: stays disarmed");
        }
    }

    #[test]
    fn not_live_it_is_hidden_and_disarmed() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        toggle.set_live(false);
        assert!(!toggle.shown() && !toggle.armed());
        let at = on(&toggle);
        assert_eq!(toggle.input(&button(true, at)), [button(true, at)]);
        assert_eq!(drawn(&toggle), []);
        toggle.set_live(true);
        assert!(toggle.shown());
    }

    #[test]
    fn a_wrapped_tap_whose_press_takes_the_host_off_live_still_ends_with_its_release() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        toggle.input(&button(true, ELSEWHERE));
        toggle.set_live(false);
        assert_eq!(
            toggle.input(&button(false, ELSEWHERE)),
            [button(false, ELSEWHERE), command(false)]
        );
    }

    #[test]
    fn it_draws_as_the_overlays_square_controls_do() {
        let toggle = toggle();
        let mut expected = DrawList::new();
        look::draw_square(&mut expected, toggle.rect(), "Trade Most", look::FILL, 1.0);
        assert_eq!(drawn(&toggle), expected.iter().cloned().collect::<Vec<_>>());
    }

    #[test]
    fn armed_it_draws_highlighted() {
        let mut toggle = toggle();
        let at = on(&toggle);
        toggle.input(&button(true, at));
        let mut expected = DrawList::new();
        look::draw_square(
            &mut expected,
            toggle.rect(),
            "Trade Most",
            look::HIGHLIGHT,
            1.0,
        );
        let expected: Vec<_> = expected.iter().cloned().collect();
        assert_eq!(drawn(&toggle), expected, "held");
        toggle.input(&button(false, at));
        assert_eq!(drawn(&toggle), expected, "armed");
    }

    #[test]
    fn nothing_is_drawn_unless_touch_was_used_last() {
        for device in OTHERS {
            let mut toggle = toggle();
            toggle.input(&Input::Surroundings(Surroundings::of(device)));
            assert_eq!(drawn(&toggle), [], "{device:?}");
        }
    }

    #[test]
    fn it_follows_a_new_safe_area() {
        let mut toggle = toggle();
        let before = toggle.rect();
        toggle.input(&Input::Surroundings(Surroundings {
            safe_area: Bounds::at(Point::new(40.0, 0.0), 984.0, 740.0),
            points_per_pixel: 0.5,
            ..Surroundings::of(Device::Touch)
        }));
        let after = toggle.rect();
        assert_ne!(after, before);
        assert_eq!(after.min.x, 56.0, "the new left, past 8 pt");
        assert_eq!(after.width(), 192.0, "96 pt at half a point a pixel");
        assert_eq!(after.max.y, 724.0);
    }

    #[test]
    fn keys_and_text_pass_whether_armed_or_not() {
        let mut toggle = toggle();
        {
            let at = on(&toggle);
            tap(&mut toggle, at)
        };
        let key = Input::Text('a');
        assert_eq!(toggle.input(&key), [key]);
        assert!(toggle.armed());
    }
}
