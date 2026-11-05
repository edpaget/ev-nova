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
    Input::Surroundings(Surroundings { device })
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
fn highlights(drawn: &[DrawCommand], rect: Bounds) -> bool {
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

#[test]
fn east_does_what_escape_does() {
    for (screen, build) in rows() {
        let mut escaped = Run::new(build).sends(&[key(Key::Escape, true), key(Key::Escape, false)]);
        let mut padded = Run::new(build).sends(&[told(PAD)]);
        padded.tap(PadButton::East);
        assert_eq!(padded.result(), escaped.result(), "{screen}");
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
        for device in [Device::Keyboard, Device::Mouse] {
            let mut left = Run::new(build).sends(&[told(PAD), told(device)]);
            assert_eq!(left.highlighted(), None, "{screen}: {device:?}");
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
            let mut after = ringed().expect("a ring").sends(&[told(PAD)]);
            for button in DPAD {
                after.tap(button);
            }
            let mut after = after.sends(&[told(Device::Mouse)]).sends(&tab).sends(&tab);
            assert_eq!(after.drawn(), keyboard.drawn(), "{screen}: {ring:?}");
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
