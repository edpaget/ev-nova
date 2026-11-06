//! The table-driven checks over every screen with a pad focus
//! (rdm:phase/key-bindings/phase-8-gamepad-focus): each screen's row
//! builds a fresh [`PadProbe`], and every test runs over every row.
//!
//! - From pad inputs alone, the d-pad reaches each of a screen's controls
//!   (its highlight surrounds the control's rect) and South presses it as
//!   a click does, with a click's sounds.
//! - East does what Escape does.
//! - Return still presses the default after the pad moved the focus.
//! - The highlight shows only while the pad was the device used last, and
//!   after the pad the keyboard's ring draws as it always did.
//!
//! The table also checks touch (rdm:phase/key-bindings/phase-14-touch-input),
//! driving each screen with fingers through the touch router
//! ([`crate::touch`]) as the app does:
//!
//! - A tap presses each control as a mouse click there does (a move, a
//!   press and a release), with a click's sounds.
//! - A cancelled touch on a control does what a mouse press there and
//!   [`Screen::cancel_pointer`] do: it fires nothing.
//! - A second finger presses nothing while the first holds a control, and
//!   after the first lifts, a finger left down presses nothing.
//!
//! And it keeps the list of screens a pointer can't leave
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps) empty: wherever
//! Escape does something, a tap on one of the screen's controls does the
//! same ([`escape_s_way_out_is_also_a_tap`]), so a screen added with no
//! pointer exit fails it.
//!
//! A new screen with a pad focus adds its row to [`rows`], from a
//! `pad_probe` fixture its own module publishes for tests.

#![allow(clippy::float_cmp)]

use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

use crate::draw::{DrawCommand, DrawList};
use crate::geometry::{Bounds, Point};
use crate::input::{ControllerFamily, Device, Input, Key, MouseButton, PadButton, Surroundings};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::fixture::MonoMetrics;
use crate::touch::testkit::Fingers;
use crate::ui::dialog::{
    Dialog, DialogEvent, DialogTemplate, FocusRing, ItemSpec, ItemTemplate, Placement,
};

/// The pad, as the app tells it.
pub(crate) const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

/// A screen under test, built fresh for each run.
pub(crate) trait PadProbe {
    /// The screen.
    fn screen(&mut self) -> &mut dyn Screen;

    /// Each control the pad can reach, by name, with the rect its
    /// highlight surrounds.
    fn controls(&self) -> Vec<(&'static str, Bounds)>;

    /// What the screen has done so far, for comparing two runs: "item 2",
    /// "closed", "music off", "recording Flight.Accelerate".
    fn outcome(&mut self) -> String;

    /// The control Return presses, if any.
    fn default_control(&self) -> Option<&'static str> {
        None
    }

    /// Sets the keyboard's focus ring, for a screen that has one; `false`
    /// for one that has none.
    fn set_ring(&mut self, _ring: FocusRing) -> bool {
        false
    }

    /// The inputs a click on control `name` at `rect` is: a left press and
    /// release at its centre, unless the screen says otherwise (the keys
    /// screen's list compares with Space).
    fn click(&self, _name: &'static str, rect: Bounds) -> Vec<Input> {
        let at = rect.center();
        vec![pointer(true, at), pointer(false, at)]
    }

    /// The control a tap on which does what Escape does, where
    /// [`escape_s_way_out_is_also_a_tap`]'s search of [`Self::controls`]
    /// can't see it; `None` to search them.
    fn exit_control(&self) -> Option<(&'static str, Bounds)> {
        None
    }

    /// What the screen's touch aids (`ui::touch_aids`) draw now, which a
    /// safe-area change legitimately moves and touch legitimately shows:
    /// the tests that compare drawings across those leave them out.
    fn touch_aids_drawn(&mut self) -> Vec<DrawCommand> {
        Vec::new()
    }
}

/// A row: the screen's name and how to build it.
pub(crate) type Row = (&'static str, fn() -> Box<dyn PadProbe>);

/// Every screen with a pad focus.
fn rows() -> Vec<Row> {
    vec![
        ("dialog", dialog_probe),
        ("note", crate::ui::note::pad_probe),
        ("yes/no", crate::ui::yes_no::pad_probe),
        ("desc", crate::ui::desc::pad_probe),
        ("desc, overflowing", crate::ui::desc::pad_probe_overflowing),
        ("comm", crate::ui::comm::comm_pad_probe),
        ("haggle", crate::ui::comm::haggle_pad_probe),
        ("plunder", crate::ui::plunder::plunder_pad_probe),
        ("assignment", crate::ui::plunder::assignment_pad_probe),
        ("planet comm", crate::ui::planet_comm::pad_probe),
        ("escort", crate::ui::escort_dialog::pad_probe),
        ("player info", crate::ui::player_info::pad_probe),
        (
            "player info's question",
            crate::ui::player_info::confirm_pad_probe,
        ),
        ("mission texts", crate::ui::mission_dialogs::pad_probe),
        ("preferences", crate::ui::prefs::pad_probe),
        ("keys screen", crate::ui::keys::pad_probe),
        ("trade's quantity", crate::ui::quantity::pad_probe),
        ("race's bet", crate::ui::quantity::bet_pad_probe),
        ("shipyard's naming", crate::ui::text_input::pad_probe),
        ("offer", crate::spaceport::offer::pad::pad_probe),
        (
            "offer, overflowing",
            crate::spaceport::offer::pad::pad_probe_overflowing,
        ),
        ("service", crate::spaceport::service::pad_probe),
        ("news", crate::spaceport::news::pad_probe),
        ("race", crate::spaceport::race::pad_probe),
        ("bar", crate::spaceport::bar::pad::pad_probe),
        ("trade", crate::spaceport::trade::pad::tests::pad_probe),
        (
            "mission computer",
            crate::spaceport::missions::pad::tests::computer_pad_probe,
        ),
        (
            "mission info",
            crate::spaceport::missions::pad::tests::info_pad_probe,
        ),
        (
            "outfitter",
            crate::spaceport::outfitter::pad::tests::pad_probe,
        ),
        (
            "shipyard",
            crate::spaceport::shipyard::pad::tests::pad_probe,
        ),
        (
            "shipyard's info",
            crate::spaceport::shipyard::pad::tests::info_pad_probe,
        ),
        ("hire", crate::spaceport::hire::pad::tests::pad_probe),
        (
            "hire's info",
            crate::spaceport::hire::pad::tests::info_pad_probe,
        ),
        ("spaceport", crate::spaceport::view::pad_probe),
        ("galaxy map", crate::galaxy::pad::tests::pad_probe),
        ("map's find", crate::galaxy::find::tests::pad_probe),
        ("main menu", crate::menu::main_menu::pad::tests::pad_probe),
        (
            "main menu, stand-in",
            crate::menu::main_menu::pad::tests::stand_in_pad_probe,
        ),
        ("pilot list", crate::menu::pilot_list::pad::tests::pad_probe),
        (
            "pilot list, empty",
            crate::menu::pilot_list::pad::tests::empty_pad_probe,
        ),
        ("new pilot", crate::ui::new_pilot::pad::tests::pad_probe),
        (
            "new pilot, built-in",
            crate::ui::new_pilot::pad::tests::fallback_pad_probe,
        ),
    ]
}

pub(crate) fn pointer(pressed: bool, at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    }
}

pub(crate) fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

pub(crate) fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings::of(device))
}

fn key(key: Key, pressed: bool) -> Input {
    Input::Key {
        key,
        pressed,
        repeat: false,
    }
}

/// A run of inputs over one probe: what the screen did, as its outcome
/// and every action it asked for.
struct Run {
    probe: Box<dyn PadProbe>,
    actions: Vec<ScreenAction>,
}

impl Run {
    fn new(build: fn() -> Box<dyn PadProbe>) -> Self {
        Self {
            probe: build(),
            actions: Vec::new(),
        }
    }

    fn send(&mut self, input: &Input) {
        let action = self.probe.screen().input(input);
        if action != ScreenAction::None {
            self.actions.push(action);
        }
    }

    fn sends(mut self, inputs: &[Input]) -> Self {
        for input in inputs {
            self.send(input);
        }
        self
    }

    fn tap(&mut self, button: PadButton) {
        self.send(&pad(button, true));
        self.send(&pad(button, false));
    }

    fn drawn(&mut self) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        self.probe.screen().draw(&mut list);
        list.iter().cloned().collect()
    }

    /// What it draws but its touch aids ([`PadProbe::touch_aids_drawn`]).
    fn drawn_without_aids(&mut self) -> Vec<DrawCommand> {
        let mut drawn = self.drawn();
        for aid in self.probe.touch_aids_drawn() {
            if let Some(at) = drawn.iter().position(|command| *command == aid) {
                drawn.remove(at);
            }
        }
        drawn
    }

    fn result(&mut self) -> (String, Vec<ScreenAction>) {
        (self.probe.outcome(), self.actions.clone())
    }

    fn sounds(&mut self) -> Vec<Sound> {
        self.probe.screen().take_sounds()
    }

    /// The control whose highlight is drawn now, if any.
    fn highlighted(&mut self) -> Option<&'static str> {
        let drawn = self.drawn();
        self.probe
            .controls()
            .into_iter()
            .find(|&(_, rect)| highlights(&drawn, rect))
            .map(|(name, _)| name)
    }
}

/// Whether `drawn` holds the pad's highlight round `rect`: the top edges
/// of its two rings, in any colour.
pub(crate) fn highlights(drawn: &[DrawCommand], rect: Bounds) -> bool {
    let top = |ring: Bounds| {
        drawn.iter().any(|command| {
            matches!(*command, DrawCommand::Line { from, to, width, .. }
                if from == ring.min && to == Point::new(ring.max.x, ring.min.y) && width == 1.0)
        })
    };
    top(rect.grown(2.0)) && top(rect.grown(3.0))
}

const DPAD: [PadButton; 4] = [
    PadButton::DPadUp,
    PadButton::DPadDown,
    PadButton::DPadLeft,
    PadButton::DPadRight,
];

/// The shortest run of d-pad taps after `Surroundings(Gamepad)` that puts
/// the highlight on control `name`, found breadth first, each path
/// replayed on a fresh build.
fn path_to(build: fn() -> Box<dyn PadProbe>, name: &str) -> Option<Vec<PadButton>> {
    let replay = |path: &[PadButton]| {
        let mut run = Run::new(build).sends(&[told(PAD)]);
        for &button in path {
            run.tap(button);
        }
        run
    };
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([Vec::new()]);
    while let Some(path) = queue.pop_front() {
        let mut run = replay(&path);
        let at = run.highlighted();
        if at == Some(name) {
            return Some(path);
        }
        if !seen.insert(format!("{:?}", run.drawn())) {
            continue;
        }
        for button in DPAD {
            let mut next = path.clone();
            next.push(button);
            queue.push_back(next);
        }
    }
    None
}

/// A run that reaches control `name` with the d-pad.
fn reached(build: fn() -> Box<dyn PadProbe>, screen: &str, name: &str) -> Run {
    let path =
        path_to(build, name).unwrap_or_else(|| panic!("{screen}: the d-pad never reaches {name}"));
    let mut run = Run::new(build).sends(&[told(PAD)]);
    for button in path {
        run.tap(button);
    }
    run
}

#[test]
fn every_listed_screen_reaches_and_presses_each_control_with_the_dpad_and_south() {
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let mut padded = reached(build, screen, name);
            padded.tap(PadButton::South);
            let clicks = build().click(name, rect);
            let mut clicked = Run::new(build).sends(&clicks);
            assert_eq!(padded.result(), clicked.result(), "{screen}: {name}");
        }
    }
}

#[test]
fn a_pad_press_sounds_as_a_click_does() {
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let mut padded = reached(build, screen, name);
            padded.sounds();
            padded.tap(PadButton::South);
            let mut clicked = Run::new(build);
            for input in build().click(name, rect) {
                clicked.send(&input);
            }
            assert_eq!(padded.sounds(), clicked.sounds(), "{screen}: {name}");
        }
    }
}

/// The point a click on control `name` presses: its click's own, or its
/// rect's centre for a control clicked by key (the keys screen's list).
fn click_point(build: fn() -> Box<dyn PadProbe>, name: &'static str, rect: Bounds) -> Point {
    build()
        .click(name, rect)
        .into_iter()
        .find_map(|input| match input {
            Input::PointerButton { at, .. } => Some(at),
            _ => None,
        })
        .unwrap_or_else(|| rect.center())
}

/// A mouse click at `at` as the platform sends it: a move there first.
fn mouse_click(at: Point) -> [Input; 3] {
    [
        Input::PointerMoved(at),
        pointer(true, at),
        pointer(false, at),
    ]
}

/// The fingers the touch tests use: none claimed.
type Free<'a> = Fingers<'a, fn(Point) -> bool, dyn Screen + 'a>;

/// A run whose screen `touch` drives with fingers.
fn touched(build: fn() -> Box<dyn PadProbe>, touch: impl FnOnce(&mut Free<'_>)) -> Run {
    let mut run = Run::new(build);
    let mut fingers = Fingers::on(run.probe.screen());
    touch(&mut fingers);
    let actions = fingers.take_actions();
    run.actions.extend(
        actions
            .into_iter()
            .filter(|&action| action != ScreenAction::None),
    );
    run
}

#[test]
fn a_tap_presses_each_control_as_a_click_does() {
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let at = click_point(build, name, rect);
            let mut tapped = touched(build, |fingers| {
                fingers.down(1, at);
                fingers.up(1, at);
            });
            let mut clicked = Run::new(build).sends(&mouse_click(at));
            assert_eq!(tapped.result(), clicked.result(), "{screen}: {name}");
        }
    }
}

#[test]
fn a_tap_sounds_as_a_click_does() {
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let at = click_point(build, name, rect);
            let mut tapped = touched(build, |fingers| {
                fingers.down(1, at);
                fingers.up(1, at);
            });
            let mut clicked = Run::new(build).sends(&mouse_click(at));
            assert_eq!(tapped.sounds(), clicked.sounds(), "{screen}: {name}");
        }
    }
}

#[test]
fn a_cancelled_touch_on_a_control_fires_nothing() {
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let at = click_point(build, name, rect);
            let mut cancelled = touched(build, |fingers| {
                fingers.down(1, at);
                fingers.cancel(1);
            });
            let mut pressed = Run::new(build).sends(&mouse_click(at)[..2]);
            pressed.probe.screen().cancel_pointer();
            assert_eq!(cancelled.result(), pressed.result(), "{screen}: {name}");
            assert_eq!(cancelled.sounds(), pressed.sounds(), "{screen}: {name}");
            assert_eq!(cancelled.drawn(), pressed.drawn(), "{screen}: {name}");
        }
    }
}

#[test]
fn a_second_finger_presses_nothing_while_the_first_holds_a_control() {
    for (screen, build) in rows() {
        let controls = build().controls();
        for &(name, rect) in &controls {
            let at = click_point(build, name, rect);
            let mut touched = touched(build, |fingers| {
                fingers.down(1, at);
                for &(other, other_rect) in &controls {
                    let elsewhere = click_point(build, other, other_rect);
                    fingers.down(2, elsewhere);
                    fingers.up(2, elsewhere);
                }
                fingers.up(1, at);
            });
            let mut clicked = Run::new(build).sends(&mouse_click(at));
            assert_eq!(touched.result(), clicked.result(), "{screen}: {name}");
        }
    }
}

#[test]
fn after_the_primary_lifts_a_finger_left_down_presses_nothing() {
    let empty = Point::new(1.0, 1.0);
    for (screen, build) in rows() {
        for (name, rect) in build().controls() {
            let at = click_point(build, name, rect);
            let left_down = |fingers: &mut Free<'_>| {
                fingers.down(1, empty);
                fingers.down(2, at);
                fingers.up(1, empty);
                fingers.up(2, at);
            };
            let mut touched = touched(build, left_down);
            let mut clicked = Run::new(build).sends(&mouse_click(empty));
            assert_eq!(touched.result(), clicked.result(), "{screen}: {name}");
            let mut tapped_after = self::touched(build, |fingers| {
                left_down(fingers);
                fingers.down(3, at);
                fingers.up(3, at);
            });
            let mut clicked_after = Run::new(build)
                .sends(&mouse_click(empty))
                .sends(&mouse_click(at));
            assert_eq!(
                tapped_after.result(),
                clicked_after.result(),
                "{screen}: {name}, a fresh tap"
            );
        }
    }
}

#[test]
fn east_does_what_escape_does() {
    for (screen, build) in rows() {
        let mut escaped = Run::new(build).sends(&[key(Key::Escape, true), key(Key::Escape, false)]);
        let mut padded = Run::new(build).sends(&[told(PAD)]);
        padded.tap(PadButton::East);
        assert_eq!(padded.result(), escaped.result(), "{screen}");
    }
}

/// A run of one finger tapping `at`.
fn tapped_at(build: fn() -> Box<dyn PadProbe>, at: Point) -> Run {
    touched(build, |fingers| {
        fingers.down(1, at);
        fingers.up(1, at);
    })
}

/// Every screen Escape leaves (or acts on) can be left by a tap too: when
/// Escape changes a fresh build's result, a tap on one of its controls (or
/// the row's [`PadProbe::exit_control`]) gives that same result. A row
/// with no such control is a screen a pointer can't leave, and needs a
/// touch way out (rdm:phase/key-bindings/phase-16-touch-pointer-gaps).
#[test]
fn escape_s_way_out_is_also_a_tap() {
    for (screen, build) in rows() {
        let mut escaped = Run::new(build).sends(&[key(Key::Escape, true), key(Key::Escape, false)]);
        let escaped = escaped.result();
        if escaped == Run::new(build).result() {
            continue;
        }
        let probe = build();
        let candidates = probe
            .exit_control()
            .map_or_else(|| probe.controls(), |exit| vec![exit]);
        let tapped = candidates.iter().find(|&&(name, rect)| {
            tapped_at(build, click_point(build, name, rect)).result() == escaped
        });
        assert!(
            tapped.is_some(),
            "{screen}: no tap does what Escape does ({escaped:?})"
        );
    }
}

#[test]
fn return_presses_the_default_after_the_pad_moved() {
    for (screen, build) in rows() {
        let Some(default) = build().default_control() else {
            continue;
        };
        let mut returned = Run::new(build).sends(&[key(Key::Enter, true), key(Key::Enter, false)]);
        let other = build()
            .controls()
            .into_iter()
            .map(|(name, _)| name)
            .find(|&name| name != default && path_to(build, name).is_some());
        let mut moved = match other {
            Some(other) => reached(build, screen, other),
            None => Run::new(build).sends(&[told(PAD)]),
        };
        moved.send(&key(Key::Enter, true));
        moved.send(&key(Key::Enter, false));
        moved.send(&told(Device::Keyboard));
        assert_eq!(moved.result(), returned.result(), "{screen}");
    }
}

#[test]
fn the_highlight_shows_only_while_the_pad_was_used_last() {
    for (screen, build) in rows() {
        let mut padded = Run::new(build).sends(&[told(PAD)]);
        assert!(padded.highlighted().is_some(), "{screen}: shown");
        for device in [Device::Keyboard, Device::Mouse, Device::Touch] {
            let mut left = Run::new(build).sends(&[told(PAD), told(device)]);
            assert_eq!(left.highlighted(), None, "{screen}: {device:?}");
        }
    }
}

/// Surroundings with `device` and the rest changed, as a resize or a
/// safe-area change tells them.
fn resized(device: Device) -> Input {
    let told = Surroundings::of(device);
    Input::Surroundings(Surroundings {
        safe_area: told.safe_area.inset(40.0, 12.0),
        points_per_pixel: 0.75,
        ..told
    })
}

#[test]
fn new_surroundings_with_the_same_device_change_nothing_drawn() {
    for (screen, build) in rows() {
        for device in [PAD, Device::Keyboard, Device::Mouse, Device::Touch] {
            let moved = || {
                let mut run = Run::new(build).sends(&[told(device)]);
                run.tap(PadButton::DPadDown);
                run
            };
            let mut kept = moved();
            let mut retold = moved().sends(&[resized(device)]);
            assert_eq!(
                retold.drawn_without_aids(),
                kept.drawn_without_aids(),
                "{screen}: {device:?}"
            );
            assert_eq!(retold.result(), kept.result(), "{screen}: {device:?}");
        }
    }
}

#[test]
fn after_the_pad_the_keyboard_ring_draws_as_it_always_did() {
    let tab = [key(Key::Tab, true), key(Key::Tab, false)];
    for (screen, build) in rows() {
        for ring in [FocusRing::Tab, FocusRing::Off] {
            let ringed = || {
                let mut probe = build();
                probe.set_ring(ring).then_some(Run {
                    probe,
                    actions: Vec::new(),
                })
            };
            let Some(keyboard) = ringed() else {
                continue;
            };
            let mut keyboard = keyboard.sends(&tab).sends(&tab);
            for device in [Device::Mouse, Device::Touch] {
                let mut after = ringed().expect("a ring").sends(&[told(PAD)]);
                for button in DPAD {
                    after.tap(button);
                }
                let mut after = after.sends(&[told(device)]).sends(&tab).sends(&tab);
                assert_eq!(
                    after.drawn_without_aids(),
                    keyboard.drawn(),
                    "{screen}: {ring:?} {device:?}"
                );
            }
        }
    }
}

/// A [`Dialog`] in a host that records its events.
struct DialogHost {
    dialog: Dialog,
    events: Vec<DialogEvent>,
}

impl Screen for DialogHost {
    fn input(&mut self, input: &Input) -> ScreenAction {
        self.events.extend(self.dialog.input(input));
        ScreenAction::None
    }

    fn tick(&mut self, _dt: std::time::Duration) {}

    fn draw(&self, list: &mut DrawList) {
        self.dialog.draw(list);
    }
}

impl PadProbe for DialogHost {
    fn screen(&mut self) -> &mut dyn Screen {
        self
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        ["OK", "Cancel", "Other", "Below", "Area"]
            .into_iter()
            .zip(1..)
            .map(|(name, item)| (name, self.dialog.item_bounds(item).expect("item")))
            .collect()
    }

    fn outcome(&mut self) -> String {
        format!("{:?}", self.events)
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("OK")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.dialog.set_focus_ring(ring);
        true
    }
}

/// A generic dialog: three buttons in a row (OK the default, Cancel the
/// cancel item, Other), a fourth below Other, a role-less user item (a
/// click area) at the top right, and a static text.
fn dialog_probe() -> Box<dyn PadProbe> {
    let rect = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
    let item = |bounds, enabled, kind| ItemTemplate {
        bounds,
        enabled,
        kind,
    };
    let button =
        |x, y, title: &str| item(rect(x, y, 70.0, 20.0), true, ItemSpec::Button(title.into()));
    let template = DialogTemplate {
        bounds: rect(0.0, 0.0, 300.0, 140.0),
        placement: Placement::Fixed,
        items: vec![
            button(220.0, 70.0, "OK"),
            button(140.0, 70.0, "Cancel"),
            button(60.0, 70.0, "Other"),
            button(60.0, 110.0, "Below"),
            item(rect(250.0, 10.0, 30.0, 30.0), true, ItemSpec::User),
            item(
                rect(10.0, 10.0, 200.0, 40.0),
                false,
                ItemSpec::StaticText("Hi".into()),
            ),
        ],
    };
    Box::new(DialogHost {
        dialog: Dialog::new(&template, &[], Rc::new(MonoMetrics)).with_cancel(Some(2)),
        events: Vec::new(),
    })
}
