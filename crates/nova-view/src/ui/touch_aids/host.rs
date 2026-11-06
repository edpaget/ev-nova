//! How a screen hosts a [`ModifierToggle`]: the toggle sees each input
//! first and gives back the inputs the host runs through its own input,
//! in order; the host's pointer cancel ends a wrapped tap; and the toggle
//! shows only while the host says nothing of its own is open over it.
//!
//! A host implements [`ToggleHost`] in a sibling module of its own, and
//! its [`Screen`] methods call [`intercept`] (at the head of `input`),
//! [`cancel`] (in `cancel_pointer`) and [`draw`] (at the end of `draw`):
//! one line each in the host's file. The inputs the toggle gives back run
//! through the host's [`Screen::input`] again, past the toggle.

use super::ModifierToggle;
use crate::draw::DrawList;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};

/// A screen with a modifier toggle.
pub trait ToggleHost: Screen {
    /// Its toggle.
    fn toggle(&self) -> &ModifierToggle;

    /// Its toggle, to change.
    fn toggle_mut(&mut self) -> &mut ModifierToggle;

    /// Whether the toggle may show: nothing of the host's own (a quantity
    /// dialog, an offer, Find) is open over it.
    fn toggle_live(&self) -> bool;
}

/// Runs `input` through `host`'s toggle at the head of the host's
/// [`Screen::input`]: `None` when the host takes `input` as it is, or
/// else the action of the inputs the toggle gave back, each run through
/// the host's input past the toggle (the last that isn't
/// [`ScreenAction::None`]).
pub fn intercept(host: &mut impl ToggleHost, input: &Input) -> Option<ScreenAction> {
    if host.toggle().replaying {
        return None;
    }
    settle_live(host);
    let given = host.toggle_mut().input(input);
    if given == [*input] {
        return None;
    }
    Some(replay(host, &given))
}

/// In the host's pointer cancel: a press on the toggle forgotten, and a
/// wrapped tap's command released through the host's input. A modifier
/// is read at a press or a release, which the cancel abandons, so it may
/// run before or after the host's own cancel.
pub fn cancel(host: &mut impl ToggleHost) {
    let given = host.toggle_mut().cancel();
    replay(host, &given);
}

/// Draws `host`'s toggle over it while the host lets it show now, so one
/// opened by a call rather than an input (an offer) hides it at once.
pub fn draw(host: &impl ToggleHost, list: &mut DrawList) {
    if host.toggle_live() {
        host.toggle().draw(list);
    }
}

/// Runs `inputs` through the host's input past the toggle; the last
/// action that isn't [`ScreenAction::None`].
fn replay(host: &mut impl ToggleHost, inputs: &[Input]) -> ScreenAction {
    host.toggle_mut().replaying = true;
    let mut action = ScreenAction::None;
    for each in inputs {
        let got = host.input(each);
        if got != ScreenAction::None {
            action = got;
        }
    }
    host.toggle_mut().replaying = false;
    settle_live(host);
    action
}

/// Tells the toggle whether the host lets it show.
fn settle_live(host: &mut impl ToggleHost) {
    let live = host.toggle_live();
    host.toggle_mut().set_live(live);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Device, MouseButton, Surroundings, TouchControl};
    use crate::keymap::{Command, TradeCommand};

    const COMMAND: Command = Command::Trade(TradeCommand::MaxLot);

    /// A host that records what reaches it, and opens its own dialog on
    /// a release at [`OPENS`].
    struct Recorder {
        toggle: ModifierToggle,
        got: Vec<Input>,
        open: bool,
    }

    const OPENS: Point = Point::new(500.0, 300.0);

    impl ToggleHost for Recorder {
        fn toggle(&self) -> &ModifierToggle {
            &self.toggle
        }

        fn toggle_mut(&mut self) -> &mut ModifierToggle {
            &mut self.toggle
        }

        fn toggle_live(&self) -> bool {
            !self.open
        }
    }

    impl Screen for Recorder {
        fn tick(&mut self, _dt: std::time::Duration) {}

        fn draw(&self, list: &mut DrawList) {
            draw(self, list);
        }

        fn cancel_pointer(&mut self) {
            cancel(self);
        }

        fn input(&mut self, input: &Input) -> ScreenAction {
            if let Some(action) = intercept(self, input) {
                return action;
            }
            self.got.push(*input);
            if matches!(*input, Input::PointerButton { pressed: false, at, .. } if at == OPENS) {
                self.open = true;
            }
            if let Input::TouchControl(_) = input {
                return ScreenAction::Quit;
            }
            ScreenAction::None
        }
    }

    fn host() -> Recorder {
        let canvas = Bounds::at(Point::new(0.0, 0.0), 1024.0, 768.0);
        let occupied = Bounds::at(Point::new(212.0, 184.0), 600.0, 400.0);
        let mut host = Recorder {
            toggle: ModifierToggle::new(COMMAND, canvas, occupied),
            got: Vec::new(),
            open: false,
        };
        host.input(&Input::Surroundings(Surroundings::of(Device::Touch)));
        host
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

    fn arm(host: &mut Recorder) {
        let at = host.toggle.rect().center();
        host.input(&button(true, at));
        host.input(&button(false, at));
    }

    #[test]
    fn each_input_the_toggle_gives_back_runs_through_the_host_in_order() {
        let mut host = host();
        arm(&mut host);
        host.got.clear();
        let at = Point::new(400.0, 300.0);
        assert_eq!(host.input(&button(true, at)), ScreenAction::Quit);
        host.input(&button(false, at));
        assert_eq!(
            host.got,
            [
                command(true),
                button(true, at),
                button(false, at),
                command(false)
            ]
        );
    }

    #[test]
    fn a_host_dialog_opened_by_the_wrapped_tap_hides_the_toggle_after_it() {
        let mut host = host();
        arm(&mut host);
        host.input(&button(true, OPENS));
        host.input(&button(false, OPENS));
        assert!(!host.toggle.shown(), "hidden under the host's dialog");
        assert_eq!(host.got.last(), Some(&command(false)));
        host.open = false;
        host.input(&Input::PointerMoved(OPENS));
        assert!(host.toggle.shown(), "back once it closes");
    }

    #[test]
    fn it_is_drawn_only_while_the_host_lets_it_show() {
        let mut host = host();
        let drawn = |host: &Recorder| {
            let mut list = DrawList::new();
            host.draw(&mut list);
            list.len()
        };
        assert!(drawn(&host) > 0);
        host.open = true;
        assert_eq!(drawn(&host), 0, "before any input settles it");
    }

    #[test]
    fn the_hosts_cancel_releases_a_wrapped_command_through_its_input() {
        let mut host = host();
        arm(&mut host);
        host.input(&button(true, Point::new(400.0, 300.0)));
        host.got.clear();
        host.cancel_pointer();
        assert_eq!(host.got, [command(false)]);
        host.cancel_pointer();
        assert_eq!(host.got, [command(false)], "once");
    }
}
