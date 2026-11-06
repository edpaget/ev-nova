//! How a screen hosts a [`ModifierToggle`]: the toggle sees each input
//! first and gives back the inputs the host runs through its own input,
//! in order; the host's pointer cancel ends a wrapped tap; and the toggle
//! is live only while the host says nothing of its own is open over it.
//!
//! A host implements [`ToggleHost`] in a sibling module of its own and
//! its [`Screen`](crate::Screen) methods call [`input`], [`cancel`] and
//! [`draw`]: one line each in the host's file.

use super::ModifierToggle;
use crate::draw::DrawList;
use crate::input::Input;
use crate::screen::ScreenAction;

/// A screen with a modifier toggle.
pub trait ToggleHost {
    /// Its toggle.
    fn toggle(&self) -> &ModifierToggle;

    /// Its toggle, to change.
    fn toggle_mut(&mut self) -> &mut ModifierToggle;

    /// Whether the toggle may show: nothing of the host's own (a quantity
    /// dialog, an offer, Find) is open over it.
    fn toggle_live(&self) -> bool;

    /// The host's own input, past the toggle.
    fn own_input(&mut self, input: &Input) -> ScreenAction;
}

/// `input` through `host`'s toggle, then each input it gives back through
/// the host's own; the last action that isn't [`ScreenAction::None`].
pub fn input(host: &mut impl ToggleHost, input: &Input) -> ScreenAction {
    settle_live(host);
    let mut action = ScreenAction::None;
    for each in host.toggle_mut().input(input) {
        let got = host.own_input(&each);
        if got != ScreenAction::None {
            action = got;
        }
    }
    settle_live(host);
    action
}

/// After the host's own pointer cancel: a press on the toggle forgotten,
/// and a wrapped tap's command released through the host's input.
pub fn cancel(host: &mut impl ToggleHost) {
    for each in host.toggle_mut().cancel() {
        host.own_input(&each);
    }
    settle_live(host);
}

/// Draws `host`'s toggle over it while the host lets it show now, so one
/// opened by a call rather than an input (an offer) hides it at once.
pub fn draw(host: &impl ToggleHost, list: &mut DrawList) {
    if host.toggle_live() {
        host.toggle().draw(list);
    }
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

        fn own_input(&mut self, input: &Input) -> ScreenAction {
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
        input(
            &mut host,
            &Input::Surroundings(Surroundings::of(Device::Touch)),
        );
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
        input(host, &button(true, at));
        input(host, &button(false, at));
    }

    #[test]
    fn each_input_the_toggle_gives_back_runs_through_the_host_in_order() {
        let mut host = host();
        arm(&mut host);
        host.got.clear();
        let at = Point::new(400.0, 300.0);
        assert_eq!(input(&mut host, &button(true, at)), ScreenAction::Quit);
        input(&mut host, &button(false, at));
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
        input(&mut host, &button(true, OPENS));
        input(&mut host, &button(false, OPENS));
        assert!(!host.toggle.shown(), "hidden under the host's dialog");
        assert_eq!(host.got.last(), Some(&command(false)));
        host.open = false;
        input(&mut host, &Input::PointerMoved(OPENS));
        assert!(host.toggle.shown(), "back once it closes");
    }

    #[test]
    fn it_is_drawn_only_while_the_host_lets_it_show() {
        let mut host = host();
        let drawn = |host: &Recorder| {
            let mut list = DrawList::new();
            draw(host, &mut list);
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
        input(&mut host, &button(true, Point::new(400.0, 300.0)));
        host.got.clear();
        cancel(&mut host);
        assert_eq!(host.got, [command(false)]);
        cancel(&mut host);
        assert_eq!(host.got, [command(false)], "once");
    }
}
