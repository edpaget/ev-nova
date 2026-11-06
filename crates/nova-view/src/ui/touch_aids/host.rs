//! How a screen hosts a touch aid ([`TouchAid`]: a [`ModifierToggle`],
//! the [`PlayKeys`](super::PlayKeys)): the aid sees each input first and
//! gives back the inputs the host runs through its own input, in order;
//! the host's pointer cancel ends what the aid holds; and the aid shows
//! only while the host says nothing of its own is open over it.
//!
//! A host implements [`AidHost`] in a sibling module of its own, and its
//! [`Screen`] methods call [`intercept`] (at the head of `input`),
//! [`cancel`] (in `cancel_pointer`) and [`draw`] (at the end of `draw`):
//! one line each in the host's file. The inputs the aid gives back run
//! through the host's [`Screen::input`] again, past the aid.

use crate::draw::DrawList;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};

/// What an aid does for its host: see [`ModifierToggle`] and
/// [`PlayKeys`](super::PlayKeys).
pub trait TouchAid {
    /// Follows `input` ahead of the host: the inputs the host should run
    /// instead, in order (none for an input the aid took).
    fn input(&mut self, input: &Input) -> Vec<Input>;

    /// The host's pointer was let go of: the inputs the host should run
    /// to end what the aid holds.
    fn cancel(&mut self) -> Vec<Input>;

    /// Whether the host lets it show.
    fn set_live(&mut self, live: bool);

    /// Draws it while it shows.
    fn draw(&self, list: &mut DrawList);

    /// Whether its host is running the inputs it gave back, which pass it
    /// by.
    fn replaying(&self) -> bool;

    /// Notes that its host is, or no longer is, running them.
    fn set_replaying(&mut self, replaying: bool);
}

/// A screen with a touch aid.
pub trait AidHost: Screen {
    /// The aid it hosts.
    type Aid: TouchAid;

    /// Its aid.
    fn aid(&self) -> &Self::Aid;

    /// Its aid, to change.
    fn aid_mut(&mut self) -> &mut Self::Aid;

    /// Whether the aid may show: nothing of the host's own (a quantity
    /// dialog, an offer, Find, the mission texts) is open over it.
    fn aid_live(&self) -> bool;
}

/// Runs `input` through `host`'s aid at the head of the host's
/// [`Screen::input`]: `None` when the host takes `input` as it is, or
/// else the action of the inputs the aid gave back, each run through the
/// host's input past the aid (the last that isn't [`ScreenAction::None`]).
pub fn intercept(host: &mut impl AidHost, input: &Input) -> Option<ScreenAction> {
    if host.aid().replaying() {
        return None;
    }
    settle_live(host);
    let given = host.aid_mut().input(input);
    if given == [*input] {
        return None;
    }
    Some(replay(host, &given))
}

/// In the host's pointer cancel: a press on the aid forgotten, and what
/// it holds let go of through the host's input. A modifier is read at a
/// press or a release, which the cancel abandons, so it may run before
/// or after the host's own cancel.
pub fn cancel(host: &mut impl AidHost) {
    let given = host.aid_mut().cancel();
    replay(host, &given);
}

/// Draws `host`'s aid over it while the host lets it show now, so one
/// opened by a call rather than an input (an offer) hides it at once.
pub fn draw(host: &impl AidHost, list: &mut DrawList) {
    if host.aid_live() {
        host.aid().draw(list);
    }
}

/// Runs `inputs` through the host's input past the aid; the last action
/// that isn't [`ScreenAction::None`].
fn replay(host: &mut impl AidHost, inputs: &[Input]) -> ScreenAction {
    host.aid_mut().set_replaying(true);
    let mut action = ScreenAction::None;
    for each in inputs {
        let got = host.input(each);
        if got != ScreenAction::None {
            action = got;
        }
    }
    host.aid_mut().set_replaying(false);
    settle_live(host);
    action
}

/// Tells the aid whether the host lets it show.
fn settle_live(host: &mut impl AidHost) {
    let live = host.aid_live();
    host.aid_mut().set_live(live);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Device, MouseButton, Surroundings, TouchControl};
    use crate::keymap::{Command, TradeCommand};
    use crate::ui::touch_aids::ModifierToggle;

    const COMMAND: Command = Command::Trade(TradeCommand::MaxLot);

    /// A host that records what reaches it, and opens its own dialog on
    /// a release at [`OPENS`].
    struct Recorder {
        toggle: ModifierToggle,
        got: Vec<Input>,
        open: bool,
    }

    const OPENS: Point = Point::new(500.0, 300.0);

    impl AidHost for Recorder {
        type Aid = ModifierToggle;

        fn aid(&self) -> &ModifierToggle {
            &self.toggle
        }

        fn aid_mut(&mut self) -> &mut ModifierToggle {
            &mut self.toggle
        }

        fn aid_live(&self) -> bool {
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
