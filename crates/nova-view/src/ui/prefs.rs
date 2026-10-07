//! The Preferences dialog, "new prefs dialog" (`DLOG` 4003): sound effects
//! and music on or off, each one's volume, and Hyperspace Effects.
//!
//! The stock dialog has a column of check boxes for options this game
//! does not have yet, one volume (item 4's "Sound Volume:", its value in
//! item 5 and its arrows in items 7 and 6) and OK and Key Settings
//! buttons. It is used as it stands, with these changes:
//!
//! - Check box 8 ("Intro Music") is the music toggle, labelled "Music",
//!   and check box 20 ("Ambient Sounds") the sound effects toggle,
//!   labelled "Sound".
//! - Check box 21 ("Hyperspace Effects") works as it stands: on, a jump
//!   fades to white and back; off, it does not.
//! - The stock volume is the sound effects' volume. The music's is the
//!   same four parts moved [`MUSIC_VOLUME_OFFSET`] down, into item 3's
//!   empty area above the buttons, labelled "Music Volume:".
//! - Every other check box is drawn greyed and unchecked, and Key
//!   Settings is a greyed button: those options do not work yet.
//! - Every other item is hidden.
//!
//! The dialog has no stock frame: it is drawn over a dark backdrop with a
//! 1-unit outline.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::preferences::Prefs;
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, SoundPrefs};
use crate::text::TextMetrics;

use super::button::{ButtonSkin, ButtonStyle};
use super::dialog::{Dialog, DialogEvent, DialogTemplate, ItemSpec, Role, outline};
use super::toggle::Toggle;
use super::volume::{VolumeControl, VolumeRects};

/// The dialog's `DLOG` (and `DITL`) ID.
pub const PREFS_DIALOG: i16 = 4003;

/// The OK button's item: it closes the dialog.
pub const OK_ITEM: usize = 1;

/// The Key Settings button's item, greyed.
pub const KEY_SETTINGS_ITEM: usize = 16;

/// The music toggle's item ("Intro Music" in the stock dialog).
pub const MUSIC_ITEM: usize = 8;

/// The sound effects toggle's item ("Ambient Sounds" in the stock dialog).
pub const SOUND_ITEM: usize = 20;

/// The Hyperspace Effects toggle's item.
pub const HYPERSPACE_EFFECTS_ITEM: usize = 21;

/// The sound effects volume's label item.
pub const VOLUME_LABEL_ITEM: usize = 4;

/// The sound effects volume's value item.
pub const VOLUME_VALUE_ITEM: usize = 5;

/// The sound effects volume's up arrow item (`PICT` 134).
pub const VOLUME_UP_ITEM: usize = 7;

/// The sound effects volume's down arrow item (`PICT` 135).
pub const VOLUME_DOWN_ITEM: usize = 6;

/// How far below the sound effects volume the music volume is.
pub const MUSIC_VOLUME_OFFSET: f32 = 40.0;

/// The music toggle's label.
pub const MUSIC_LABEL: &str = "Music";

/// The sound effects toggle's label.
pub const SOUND_LABEL: &str = "Sound";

/// The Hyperspace Effects toggle's label, the stock check box's title.
pub const HYPERSPACE_EFFECTS_LABEL: &str = "Hyperspace Effects";

/// The sound effects volume's label.
pub const EFFECTS_VOLUME_LABEL: &str = "Sound Volume:";

/// The music volume's label.
pub const MUSIC_VOLUME_LABEL: &str = "Music Volume:";

/// The backdrop the dialog is drawn over.
pub const BACKDROP: Color = Color::rgba(16, 16, 24, 255);

/// The backdrop's outline.
pub const BORDER: Color = Color::DIM;

/// Whether an item at `bounds` (dialog-local) is at least partly inside a
/// dialog of `size`.
fn inside(bounds: Bounds, size: (f32, f32)) -> bool {
    bounds.max.x > 0.0 && bounds.min.x < size.0 && bounds.max.y > 0.0 && bounds.min.y < size.1
}

/// What has the keyboard focus. Tab moves it in this order, round from
/// OK back to the music toggle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The music toggle.
    Music,
    /// The sound effects toggle.
    Sound,
    /// The Hyperspace Effects toggle.
    HyperspaceEffects,
    /// The sound effects volume.
    EffectsVolume,
    /// The music volume.
    MusicVolume,
    /// The OK button.
    Ok,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::Music => Self::Sound,
            Self::Sound => Self::HyperspaceEffects,
            Self::HyperspaceEffects => Self::EffectsVolume,
            Self::EffectsVolume => Self::MusicVolume,
            Self::MusicVolume => Self::Ok,
            Self::Ok => Self::Music,
        }
    }
}

/// Where a pointer press went, so the rest of its click follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Music,
    Sound,
    HyperspaceEffects,
    EffectsVolume,
    MusicVolume,
    Dialog,
}

/// "new prefs dialog": the preferences, changed with the mouse or
/// the keyboard, and closed by OK, Return or Escape.
#[derive(Clone)]
pub struct PrefsDialog {
    dialog: Dialog,
    music: Toggle,
    sound: Toggle,
    effects_volume: VolumeControl,
    music_volume: VolumeControl,
    hyperspace: Toggle,
    /// The check boxes for options that do not work yet, greyed.
    inert: Vec<Toggle>,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    focus: Option<Focus>,
    target: Option<Target>,
    changed: bool,
    closed: bool,
}

impl std::fmt::Debug for PrefsDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrefsDialog")
            .field("prefs", &self.prefs())
            .field("focus", &self.focus)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl PrefsDialog {
    /// The dialog `template` (stock `DLOG` 4003) showing `prefs`, its
    /// buttons labelled in `style`, its text measured by `metrics`.
    ///
    /// # Errors
    ///
    /// When the template lacks one of the items the dialog is built from.
    pub fn new(
        template: &DialogTemplate,
        prefs: Prefs,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Result<Self, String> {
        let roles: Vec<(usize, Role)> = (1..=template.items.len())
            .filter(|&item| item != OK_ITEM)
            .map(|item| {
                let role = if item == KEY_SETTINGS_ITEM {
                    Role::Greyed
                } else {
                    Role::Hidden
                };
                (item, role)
            })
            .collect();
        let dialog = Dialog::new(template, &roles, Rc::clone(&metrics))
            .with_buttons(ButtonSkin::NOVA, style)
            .with_default(Some(OK_ITEM))
            .with_cancel(Some(OK_ITEM));
        let place = |item: usize| {
            dialog
                .item_bounds(item)
                .ok_or_else(|| format!("DITL {PREFS_DIALOG} has no item {item}"))
        };
        let effects = VolumeRects {
            label: place(VOLUME_LABEL_ITEM)?,
            value: place(VOLUME_VALUE_ITEM)?,
            up: place(VOLUME_UP_ITEM)?,
            down: place(VOLUME_DOWN_ITEM)?,
        };
        let music_rects = effects.offset(Point::new(0.0, MUSIC_VOLUME_OFFSET));
        let hyperspace = Toggle::new(
            place(HYPERSPACE_EFFECTS_ITEM)?,
            HYPERSPACE_EFFECTS_LABEL,
            prefs.hyperspace_effects,
        );
        let prefs = prefs.sound;
        let music = Toggle::new(place(MUSIC_ITEM)?, MUSIC_LABEL, prefs.music);
        let sound = Toggle::new(place(SOUND_ITEM)?, SOUND_LABEL, prefs.sound);
        let size = (template.bounds.width(), template.bounds.height());
        let inert = template
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match &item.kind {
                ItemSpec::CheckBox(title)
                    if ![MUSIC_ITEM, SOUND_ITEM, HYPERSPACE_EFFECTS_ITEM]
                        .contains(&(index + 1))
                        && inside(item.bounds, size) =>
                {
                    Some(Toggle::greyed(
                        dialog.item_bounds(index + 1)?,
                        title.as_str(),
                    ))
                }
                _ => None,
            })
            .collect();
        Ok(Self {
            music,
            sound,
            effects_volume: VolumeControl::new(effects, EFFECTS_VOLUME_LABEL, prefs.effects_level),
            music_volume: VolumeControl::new(music_rects, MUSIC_VOLUME_LABEL, prefs.music_level),
            hyperspace,
            inert,
            dialog,
            style,
            metrics,
            focus: None,
            target: None,
            changed: false,
            closed: false,
        })
    }

    /// The preferences as the dialog shows them.
    #[must_use]
    pub fn prefs(&self) -> Prefs {
        Prefs {
            sound: SoundPrefs {
                sound: self.sound.on(),
                music: self.music.on(),
                effects_level: self.effects_volume.level(),
                music_level: self.music_volume.level(),
            },
            hyperspace_effects: self.hyperspace.on(),
        }
    }

    /// The preferences, once after each change: `None` when nothing has
    /// changed since they were last taken.
    pub fn take_change(&mut self) -> Option<Prefs> {
        std::mem::take(&mut self.changed).then(|| self.prefs())
    }

    fn key(&mut self, key: Key, repeat: bool, input: &Input) {
        match (key, self.focus) {
            (Key::Tab, _) => {
                if !repeat {
                    self.focus = Some(self.focus.map_or(Focus::Music, Focus::next));
                }
            }
            (Key::Enter | Key::Escape, _) => self.dialog_input(input),
            (Key::Space, Some(Focus::Ok)) => self.closed |= !repeat,
            (_, Some(Focus::Music)) => self.changed |= self.music.input(input),
            (_, Some(Focus::Sound)) => self.changed |= self.sound.input(input),
            (_, Some(Focus::HyperspaceEffects)) => self.changed |= self.hyperspace.input(input),
            (_, Some(Focus::EffectsVolume)) => self.changed |= self.effects_volume.input(input),
            (_, Some(Focus::MusicVolume)) => self.changed |= self.music_volume.input(input),
            _ => {}
        }
    }

    /// Sends a press to the part under it, and the rest of its click (the
    /// moves and the release) to the same part.
    fn pointer(&mut self, input: &Input) {
        if let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        {
            self.cancel_pointer();
            self.target = Some(if self.music.contains(at) {
                Target::Music
            } else if self.sound.contains(at) {
                Target::Sound
            } else if self.hyperspace.contains(at) {
                Target::HyperspaceEffects
            } else if self.effects_volume.arrow_at(at).is_some() {
                Target::EffectsVolume
            } else if self.music_volume.arrow_at(at).is_some() {
                Target::MusicVolume
            } else {
                Target::Dialog
            });
        }
        match self.target {
            Some(Target::Music) => self.changed |= self.music.input(input),
            Some(Target::Sound) => self.changed |= self.sound.input(input),
            Some(Target::HyperspaceEffects) => self.changed |= self.hyperspace.input(input),
            Some(Target::EffectsVolume) => self.changed |= self.effects_volume.input(input),
            Some(Target::MusicVolume) => self.changed |= self.music_volume.input(input),
            Some(Target::Dialog) => self.dialog_input(input),
            None => {}
        }
    }

    fn dialog_input(&mut self, input: &Input) {
        if self.dialog.input(input) == Some(DialogEvent::Item(OK_ITEM)) {
            self.closed = true;
        }
    }

    /// Whether OK has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// What has the keyboard focus, if anything.
    #[must_use]
    pub fn focus(&self) -> Option<Focus> {
        self.focus
    }

    /// The dialog itself, for its layout and its buttons.
    #[must_use]
    pub fn dialog(&self) -> &Dialog {
        &self.dialog
    }

    /// The music toggle.
    #[must_use]
    pub fn music(&self) -> &Toggle {
        &self.music
    }

    /// The sound effects toggle.
    #[must_use]
    pub fn sound(&self) -> &Toggle {
        &self.sound
    }

    /// The Hyperspace Effects toggle.
    #[must_use]
    pub fn hyperspace_effects(&self) -> &Toggle {
        &self.hyperspace
    }

    /// The sound effects volume.
    #[must_use]
    pub fn effects_volume(&self) -> &VolumeControl {
        &self.effects_volume
    }

    /// The music volume.
    #[must_use]
    pub fn music_volume(&self) -> &VolumeControl {
        &self.music_volume
    }

    /// The greyed check boxes, in item order.
    #[must_use]
    pub fn inert(&self) -> &[Toggle] {
        &self.inert
    }
}

impl Screen for PrefsDialog {
    /// - A click on a toggle flips it, and a click on a volume's arrow
    ///   steps it. A click on OK closes the dialog; the greyed check boxes
    ///   and Key Settings do nothing.
    /// - Tab moves the focus (music, sound, Hyperspace Effects, the sound
    ///   volume, the music volume, OK, and round). Space flips a focused toggle or activates
    ///   OK; the arrow keys step a focused volume, repeats included.
    /// - Return and Escape close it.
    ///
    /// It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Key {
                key,
                pressed: true,
                repeat,
            } => self.key(key, repeat, input),
            Input::Key { .. } | Input::Text(_) => {}
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline, OK and Key Settings, the greyed check
    /// boxes, the three toggles, then the two volumes. The focused part is
    /// outlined.
    fn draw(&self, list: &mut DrawList) {
        let bounds = self.dialog.bounds();
        fill_rect(list, bounds, BACKDROP);
        outline(list, bounds, BORDER);
        self.dialog.draw(list);
        for toggle in &self.inert {
            toggle.draw(false, &self.metrics, list);
        }
        let focused = |part| self.focus == Some(part);
        self.music.draw(focused(Focus::Music), &self.metrics, list);
        self.sound.draw(focused(Focus::Sound), &self.metrics, list);
        self.hyperspace
            .draw(focused(Focus::HyperspaceEffects), &self.metrics, list);
        self.effects_volume
            .draw(focused(Focus::EffectsVolume), list);
        self.music_volume.draw(focused(Focus::MusicVolume), list);
        if let (true, Some(ok)) = (focused(Focus::Ok), self.dialog.item_bounds(OK_ITEM)) {
            outline(list, ok, self.style.up);
        }
    }

    fn cancel_pointer(&mut self) {
        self.dialog.cancel_pointer();
        self.music.cancel_pointer();
        self.sound.cancel_pointer();
        self.hyperspace.cancel_pointer();
        self.effects_volume.cancel_pointer();
        self.music_volume.cancel_pointer();
        self.target = None;
    }

    /// OK's sounds as it is clicked.
    fn take_sounds(&mut self) -> Vec<Sound> {
        self.dialog
            .take_sound()
            .map(Sound::Ui)
            .into_iter()
            .collect()
    }

    /// The preferences once after each change.
    fn take_prefs(&mut self) -> Option<Prefs> {
        self.take_change()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemTemplate, Placement};
    use crate::ui::volume::{Arrow, draw_arrow};

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    /// Bounds from (left, top) to (right, bottom).
    fn ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    /// The Mac "new prefs dialog" (`DITL` 4003), item by item: 336 x 278,
    /// centred.
    fn template() -> DialogTemplate {
        let item = |enabled, (l, t, r, b), kind| ItemTemplate {
            bounds: ltrb(l, t, r, b),
            enabled,
            kind,
        };
        let check =
            |(l, t, r, b), title: &str| item(true, (l, t, r, b), ItemSpec::CheckBox(title.into()));
        DialogTemplate {
            bounds: ltrb(37.0, 54.0, 373.0, 332.0),
            placement: Placement::Center,
            items: vec![
                item(
                    true,
                    (225.0, 245.0, 295.0, 265.0),
                    ItemSpec::Button("OK".into()),
                ),
                check((171.0, 55.0, 342.0, 73.0), "Share Processor Time"),
                item(false, (69.0, 213.0, 314.0, 230.0), ItemSpec::User),
                item(
                    false,
                    (171.0, 167.0, 277.0, 183.0),
                    ItemSpec::StaticText("Sound Volume:".into()),
                ),
                item(
                    false,
                    (189.0, 186.0, 311.0, 202.0),
                    ItemSpec::StaticText("Static Text".into()),
                ),
                item(true, (172.0, 194.0, 183.0, 203.0), ItemSpec::Picture(135)),
                item(true, (172.0, 185.0, 183.0, 194.0), ItemSpec::Picture(134)),
                check((171.0, 33.0, 270.0, 51.0), "Intro Music"),
                check((171.0, 99.0, 307.0, 117.0), "QuickTime Movies"),
                check((11.0, 121.0, 172.0, 139.0), "Smoke Trails"),
                check((171.0, 77.0, 302.0, 95.0), "Run in a window"),
                check((11.0, 33.0, 172.0, 51.0), "Ship Animations"),
                check((11.0, 55.0, 172.0, 73.0), "Engine Glows"),
                check((11.0, 77.0, 172.0, 95.0), "Running Lights"),
                check((11.0, 99.0, 172.0, 117.0), "Weapon Effects"),
                item(
                    true,
                    (49.0, 245.0, 184.0, 265.0),
                    ItemSpec::Button("Key Settings".into()),
                ),
                item(true, (186.0, 416.0, 306.0, 436.0), ItemSpec::User),
                check((11.0, 143.0, 156.0, 161.0), "Parallax Starfield"),
                item(false, (12.0, 5.0, 325.0, 28.0), ItemSpec::User),
                check((171.0, 121.0, 307.0, 139.0), "Ambient Sounds"),
                check((171.0, 143.0, 316.0, 161.0), "Hyperspace Effects"),
                check((11.0, 165.0, 156.0, 183.0), "Check For Updates"),
                // Parked outside, as the Windows build parks nothing here.
                check((400.0, 10.0, 420.0, 20.0), "Outside"),
            ],
        }
    }

    /// Where the dialog goes: ((1024 - 336) / 2, (768 - 278) / 2).
    const ORIGIN: Point = Point::new(344.0, 245.0);

    /// A dialog-local rectangle on the screen.
    fn placed(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        ltrb(left, top, right, bottom).offset(ORIGIN)
    }

    fn start() -> Prefs {
        Prefs {
            sound: SoundPrefs {
                sound: true,
                music: false,
                effects_level: 4,
                music_level: 2,
            },
            hyperspace_effects: true,
        }
    }

    fn prefs() -> PrefsDialog {
        PrefsDialog::new(
            &template(),
            start(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds")
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    fn click(dialog: &mut PrefsDialog, at: Point) {
        for pressed in [true, false] {
            assert_eq!(dialog.input(&button(pressed, at)), ScreenAction::None);
        }
    }

    fn drawn(dialog: &PrefsDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn ok(dialog: &PrefsDialog) -> Point {
        dialog.dialog().item_bounds(OK_ITEM).expect("OK").center()
    }

    #[test]
    fn it_starts_from_the_prefs_it_is_given() {
        let dialog = prefs();
        assert_eq!(dialog.prefs(), start());
        assert!(!dialog.music().on() && dialog.sound().on());
        assert_eq!(dialog.effects_volume().level(), 4);
        assert_eq!(dialog.music_volume().level(), 2);
        assert!(!dialog.closed());
        assert_eq!(dialog.focus(), None);
        assert_eq!(PREFS_DIALOG, 4003);
    }

    #[test]
    fn the_toggles_are_the_two_sound_check_boxes_relabelled() {
        let dialog = prefs();
        assert_eq!(dialog.music().rect(), placed(171.0, 33.0, 270.0, 51.0));
        assert_eq!(dialog.music().label(), MUSIC_LABEL);
        assert_eq!(dialog.sound().rect(), placed(171.0, 121.0, 307.0, 139.0));
        assert_eq!(dialog.sound().label(), SOUND_LABEL);
        assert_eq!((MUSIC_LABEL, SOUND_LABEL), ("Music", "Sound"));
        assert!(dialog.music().enabled() && dialog.sound().enabled());
    }

    #[test]
    fn the_effects_volume_is_the_stock_one_and_the_music_volume_sits_below() {
        let dialog = prefs();
        let effects = VolumeRects {
            label: placed(171.0, 167.0, 277.0, 183.0),
            value: placed(189.0, 186.0, 311.0, 202.0),
            up: placed(172.0, 185.0, 183.0, 194.0),
            down: placed(172.0, 194.0, 183.0, 203.0),
        };
        assert_eq!(dialog.effects_volume().rects(), effects);
        assert_eq!(dialog.effects_volume().label(), EFFECTS_VOLUME_LABEL);
        assert_eq!(
            dialog.music_volume().rects(),
            effects.offset(at(0.0, MUSIC_VOLUME_OFFSET))
        );
        assert_eq!(dialog.music_volume().label(), MUSIC_VOLUME_LABEL);
        assert_eq!(MUSIC_VOLUME_OFFSET, 40.0);
        assert_eq!(
            (EFFECTS_VOLUME_LABEL, MUSIC_VOLUME_LABEL),
            ("Sound Volume:", "Music Volume:")
        );
    }

    #[test]
    fn every_other_check_box_inside_is_greyed_with_its_own_title() {
        let dialog = prefs();
        let inert: Vec<(&str, bool, bool)> = dialog
            .inert()
            .iter()
            .map(|toggle| (toggle.label(), toggle.enabled(), toggle.on()))
            .collect();
        let titles: Vec<&str> = inert.iter().map(|(title, ..)| *title).collect();
        assert_eq!(
            titles,
            [
                "Share Processor Time",
                "QuickTime Movies",
                "Smoke Trails",
                "Run in a window",
                "Ship Animations",
                "Engine Glows",
                "Running Lights",
                "Weapon Effects",
                "Parallax Starfield",
                "Check For Updates",
            ]
        );
        assert!(inert.iter().all(|(_, enabled, on)| !enabled && !on));
        assert_eq!(dialog.inert()[2].rect(), placed(11.0, 121.0, 172.0, 139.0));
    }

    #[test]
    fn the_hyperspace_effects_box_is_enabled_and_shows_the_pref() {
        let dialog = prefs();
        let toggle = dialog.hyperspace_effects();
        assert_eq!(toggle.rect(), placed(171.0, 143.0, 316.0, 161.0));
        assert_eq!(toggle.label(), HYPERSPACE_EFFECTS_LABEL);
        assert_eq!(HYPERSPACE_EFFECTS_LABEL, "Hyperspace Effects");
        assert_eq!(HYPERSPACE_EFFECTS_ITEM, 21);
        assert!(toggle.enabled() && toggle.on());
        let off = PrefsDialog::new(
            &template(),
            Prefs {
                hyperspace_effects: false,
                ..start()
            },
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert!(!off.hyperspace_effects().on());
        assert!(!off.prefs().hyperspace_effects);
        let on = PrefsDialog::new(
            &template(),
            Prefs::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        assert!(on.hyperspace_effects().on(), "on by default");
    }

    #[test]
    fn a_click_on_hyperspace_effects_flips_it_and_reports_once() {
        let mut dialog = prefs();
        let point = dialog.hyperspace_effects().rect().center();
        click(&mut dialog, point);
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                hyperspace_effects: false,
                ..start()
            })
        );
        assert_eq!(dialog.take_change(), None, "once");
        click(&mut dialog, point);
        assert_eq!(dialog.take_change(), Some(start()), "back on");
        assert!(!dialog.closed());
    }

    #[test]
    fn space_flips_the_focused_hyperspace_effects_box_and_outlines_it() {
        let mut dialog = prefs();
        let unfocused = drawn(&dialog);
        for _ in 0..3 {
            dialog.input(&key(Key::Tab));
        }
        assert_eq!(dialog.focus(), Some(Focus::HyperspaceEffects));
        let mut outlined = DrawList::new();
        dialog
            .hyperspace_effects()
            .draw(true, &MonoMetrics, &mut outlined);
        let outlined: Vec<DrawCommand> = outlined.iter().cloned().collect();
        let commands = drawn(&dialog);
        assert_eq!(commands.len(), unfocused.len() + 4);
        assert!(commands.windows(outlined.len()).any(|w| w == outlined));
        dialog.input(&held(Key::Space));
        assert!(dialog.prefs().hyperspace_effects, "a repeat does nothing");
        dialog.input(&key(Key::Space));
        assert!(!dialog.prefs().hyperspace_effects);
        assert_eq!(
            dialog.take_change().map(|p| p.hyperspace_effects),
            Some(false)
        );
    }

    #[test]
    fn only_ok_and_key_settings_are_left_to_the_dialog() {
        let dialog = prefs();
        let shown: Vec<usize> = (1..=23)
            .filter(|&n| dialog.dialog().item_shown(n))
            .collect();
        assert_eq!(shown, [OK_ITEM, KEY_SETTINGS_ITEM]);
        assert_eq!(dialog.dialog().bounds(), Bounds::at(ORIGIN, 336.0, 278.0));
    }

    #[test]
    fn a_template_without_an_item_it_needs_is_an_error() {
        for missing in [
            MUSIC_ITEM,
            SOUND_ITEM,
            HYPERSPACE_EFFECTS_ITEM,
            VOLUME_LABEL_ITEM,
            VOLUME_DOWN_ITEM,
        ] {
            let mut template = template();
            template.items.truncate(missing - 1);
            let error =
                PrefsDialog::new(&template, start(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
                    .expect_err("an item is missing");
            assert!(
                error.contains("DITL 4003 has no item"),
                "{missing}: {error}"
            );
        }
    }

    #[test]
    fn clicking_the_toggles_changes_music_and_sound() {
        let mut dialog = prefs();
        let point = dialog.music().rect().center();
        click(&mut dialog, point);
        assert!(dialog.prefs().sound.music);
        let point = dialog.sound().rect().center();
        click(&mut dialog, point);
        assert!(!dialog.prefs().sound.sound);
        assert_eq!(
            dialog.take_change(),
            Some(Prefs {
                sound: SoundPrefs {
                    music: true,
                    sound: false,
                    ..start().sound
                },
                ..start()
            })
        );
        assert!(!dialog.closed());
    }

    #[test]
    fn typed_text_changes_nothing() {
        let mut dialog = prefs();
        for c in [' ', 'm', 's', '\r'] {
            assert_eq!(dialog.input(&Input::Text(c)), ScreenAction::None);
        }
        assert_eq!(dialog.prefs(), start());
        assert!(!dialog.closed());
        assert_eq!(dialog.take_change(), None);
    }

    #[test]
    fn the_arrows_step_the_volumes() {
        let mut dialog = prefs();
        let effects = dialog.effects_volume().rects();
        let music = dialog.music_volume().rects();
        click(&mut dialog, effects.up.center());
        click(&mut dialog, effects.up.center());
        click(&mut dialog, music.down.center());
        assert_eq!(dialog.prefs().sound.effects_level, 6);
        assert_eq!(dialog.prefs().sound.music_level, 1);
    }

    #[test]
    fn a_click_moved_off_and_let_go_elsewhere_changes_nothing() {
        let mut dialog = prefs();
        let music = dialog.music().rect().center();
        dialog.input(&button(true, music));
        dialog.input(&Input::PointerMoved(ok(&dialog)));
        dialog.input(&button(false, ok(&dialog)));
        assert_eq!(dialog.prefs(), start());
        assert!(!dialog.closed(), "the press began on the toggle");
        assert_eq!(dialog.take_change(), None);
    }

    #[test]
    fn a_cancelled_click_changes_nothing() {
        let mut dialog = prefs();
        let points = [
            dialog.music().rect().center(),
            dialog.sound().rect().center(),
            dialog.hyperspace_effects().rect().center(),
            dialog.effects_volume().rects().up.center(),
            dialog.music_volume().rects().up.center(),
            ok(&dialog),
        ];
        for point in points {
            dialog.input(&button(true, point));
            dialog.cancel_pointer();
            dialog.input(&button(false, point));
        }
        assert_eq!(dialog.prefs(), start());
        assert!(!dialog.closed());
    }

    #[test]
    fn the_greyed_check_boxes_and_key_settings_do_nothing() {
        let mut dialog = prefs();
        let points: Vec<Point> = dialog
            .inert()
            .iter()
            .map(|toggle| toggle.rect().center())
            .chain([dialog
                .dialog()
                .item_bounds(KEY_SETTINGS_ITEM)
                .expect("Key Settings")
                .center()])
            .collect();
        for point in points {
            click(&mut dialog, point);
            assert_eq!(dialog.take_sounds(), [], "{point:?}");
        }
        let point = dialog.effects_volume().rects().label.center();
        click(&mut dialog, point);
        let point = dialog.effects_volume().rects().value.center();
        click(&mut dialog, point);
        assert_eq!(dialog.prefs(), start());
        assert_eq!(dialog.take_change(), None);
        assert!(!dialog.closed());
        assert!(dialog.inert().iter().all(|toggle| !toggle.on()));
    }

    #[test]
    fn tab_moves_the_focus_round_the_ring() {
        let mut dialog = prefs();
        let mut order = Vec::new();
        for _ in 0..7 {
            dialog.input(&key(Key::Tab));
            order.push(dialog.focus().expect("focused"));
        }
        assert_eq!(
            order,
            [
                Focus::Music,
                Focus::Sound,
                Focus::HyperspaceEffects,
                Focus::EffectsVolume,
                Focus::MusicVolume,
                Focus::Ok,
                Focus::Music,
            ]
        );
        dialog.input(&held(Key::Tab));
        assert_eq!(dialog.focus(), Some(Focus::Music), "a held Tab moves once");
    }

    #[test]
    fn space_flips_the_focused_toggle() {
        let mut dialog = prefs();
        dialog.input(&key(Key::Space));
        assert_eq!(dialog.prefs(), start(), "nothing focused");
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Space));
        assert!(dialog.prefs().sound.music);
        dialog.input(&held(Key::Space));
        assert!(dialog.prefs().sound.music, "a repeat does nothing");
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Space));
        assert!(!dialog.prefs().sound.sound);
        dialog.input(&key(Key::Up));
        assert_eq!(dialog.prefs().sound.effects_level, 4, "not a volume");
    }

    #[test]
    fn the_arrow_keys_step_the_focused_volume() {
        let mut dialog = prefs();
        dialog.input(&key(Key::Up));
        assert_eq!(dialog.prefs(), start(), "nothing focused");
        for _ in 0..4 {
            dialog.input(&key(Key::Tab));
        }
        dialog.input(&key(Key::Up));
        dialog.input(&held(Key::Right));
        assert_eq!(dialog.prefs().sound.effects_level, 6);
        dialog.input(&key(Key::Space));
        assert_eq!(
            dialog.prefs(),
            Prefs {
                sound: SoundPrefs {
                    effects_level: 6,
                    ..start().sound
                },
                ..start()
            }
        );
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Down));
        dialog.input(&held(Key::Left));
        dialog.input(&held(Key::Left));
        assert_eq!(dialog.prefs().sound.music_level, 0);
        dialog.input(&key(Key::Tab));
        dialog.input(&key(Key::Down));
        assert_eq!(dialog.prefs().sound.music_level, 0, "OK is focused");
        assert!(!dialog.closed());
    }

    #[test]
    fn return_escape_space_on_ok_and_a_click_on_ok_close_it() {
        for input in [key(Key::Enter), key(Key::Escape)] {
            let mut dialog = prefs();
            dialog.input(&input);
            assert!(dialog.closed(), "{input:?}");
        }
        let mut dialog = prefs();
        for _ in 0..6 {
            dialog.input(&key(Key::Tab));
        }
        dialog.input(&held(Key::Space));
        assert!(!dialog.closed(), "a repeat");
        dialog.input(&key(Key::Space));
        assert!(dialog.closed());
        let mut dialog = prefs();
        let ok = ok(&dialog);
        click(&mut dialog, ok);
        assert!(dialog.closed());
        for input in [held(Key::Enter), held(Key::Escape)] {
            let mut dialog = prefs();
            dialog.input(&input);
            assert!(!dialog.closed(), "{input:?}");
        }
    }

    #[test]
    fn a_change_is_reported_once() {
        let mut dialog = prefs();
        assert_eq!(dialog.take_prefs(), None, "no change yet");
        let point = dialog.effects_volume().rects().down.center();
        click(&mut dialog, point);
        let changed = Prefs {
            sound: SoundPrefs {
                effects_level: 3,
                ..start().sound
            },
            ..start()
        };
        assert_eq!(dialog.take_prefs(), Some(changed));
        assert_eq!(dialog.take_prefs(), None, "once");
        // A step that changes nothing is not a change.
        for _ in 0..4 {
            dialog.input(&key(Key::Tab));
        }
        for _ in 0..3 {
            dialog.input(&key(Key::Down));
        }
        dialog.take_prefs();
        dialog.input(&key(Key::Down));
        assert_eq!(dialog.take_prefs(), None, "already silent");
    }

    #[test]
    fn every_way_of_changing_a_setting_reports_it() {
        let focused = |tabs: usize, input: Input| {
            let mut dialog = prefs();
            for _ in 0..tabs {
                dialog.input(&key(Key::Tab));
            }
            dialog.input(&input);
            dialog.take_change()
        };
        let music = focused(1, key(Key::Space)).expect("Space on Music");
        assert!(music.sound.music);
        let sound = focused(2, key(Key::Space)).expect("Space on Sound");
        assert!(!sound.sound.sound);
        let hyperspace = focused(3, key(Key::Space)).expect("Space on Hyperspace Effects");
        assert!(!hyperspace.hyperspace_effects);
        let effects = focused(4, key(Key::Up)).expect("Up on the sound volume");
        assert_eq!(effects.sound.effects_level, 5);
        let volume = focused(5, key(Key::Down)).expect("Down on the music volume");
        assert_eq!(volume.sound.music_level, 1);
        let clicked = |part: fn(&PrefsDialog) -> Point| {
            let mut dialog = prefs();
            let point = part(&dialog);
            click(&mut dialog, point);
            dialog.take_change()
        };
        assert!(
            clicked(|d| d.music().rect().center())
                .expect("Music")
                .sound
                .music
        );
        assert!(
            !clicked(|d| d.sound().rect().center())
                .expect("Sound")
                .sound
                .sound
        );
        let hyperspace = clicked(|d| d.hyperspace_effects().rect().center());
        assert!(!hyperspace.expect("Hyperspace Effects").hyperspace_effects);
        let up = clicked(|d| d.effects_volume().rects().up.center());
        assert_eq!(up.expect("the sound volume").sound.effects_level, 5);
        let up = clicked(|d| d.music_volume().rects().up.center());
        assert_eq!(up.expect("the music volume").sound.music_level, 3);
    }

    #[test]
    fn a_check_box_touching_an_edge_from_outside_is_not_drawn() {
        let size = (336.0, 278.0);
        let touching = [
            ltrb(-20.0, 10.0, 0.0, 20.0),
            ltrb(size.0, 10.0, size.0 + 20.0, 20.0),
            ltrb(10.0, -20.0, 30.0, 0.0),
            ltrb(10.0, size.1, 30.0, size.1 + 20.0),
        ];
        let overlapping = [
            ltrb(-20.0, 10.0, 1.0, 20.0),
            ltrb(size.0 - 1.0, 10.0, size.0 + 20.0, 20.0),
            ltrb(10.0, -20.0, 30.0, 1.0),
            ltrb(10.0, size.1 - 1.0, 30.0, size.1 + 20.0),
        ];
        // Only the edge check boxes are greyed: the others become users,
        // keeping every item's number.
        let mut template = template();
        for item in &mut template.items {
            if matches!(item.kind, ItemSpec::CheckBox(_))
                && item.bounds != ltrb(171.0, 33.0, 270.0, 51.0)
                && item.bounds != ltrb(171.0, 121.0, 307.0, 139.0)
            {
                item.kind = ItemSpec::User;
            }
        }
        for (n, bounds) in touching.iter().chain(&overlapping).enumerate() {
            template.items.push(ItemTemplate {
                bounds: *bounds,
                enabled: true,
                kind: ItemSpec::CheckBox(format!("edge {n}")),
            });
        }
        let dialog = PrefsDialog::new(&template, start(), ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        let labels: Vec<&str> = dialog.inert().iter().map(Toggle::label).collect();
        assert_eq!(labels, ["edge 4", "edge 5", "edge 6", "edge 7"]);
    }

    #[test]
    fn a_click_on_ok_sounds_it_and_the_rest_are_silent() {
        let mut dialog = prefs();
        let point = dialog.music().rect().center();
        click(&mut dialog, point);
        let point = dialog.effects_volume().rects().up.center();
        click(&mut dialog, point);
        assert_eq!(dialog.take_sounds(), []);
        let ok = ok(&dialog);
        dialog.input(&button(true, ok));
        assert_eq!(dialog.take_sounds(), [Sound::Ui(UiSound::ButtonDown)]);
        dialog.input(&button(false, ok));
        assert_eq!(dialog.take_sounds(), [Sound::Ui(UiSound::ButtonUp)]);
        let mut dialog = prefs();
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_sounds(), []);
    }

    #[test]
    fn it_never_quits_and_nothing_moves_on_its_own() {
        let mut dialog = prefs();
        let before = drawn(&dialog);
        dialog.tick(Duration::from_secs(1));
        assert_eq!(drawn(&dialog), before);
        assert_eq!(dialog.input(&key(Key::Escape)), ScreenAction::None);
        dialog.release_keys();
    }

    #[test]
    fn it_draws_the_backdrop_the_buttons_the_check_boxes_then_the_volumes() {
        let dialog = prefs();
        let commands = drawn(&dialog);
        let mut expected = DrawList::new();
        let bounds = dialog.dialog().bounds();
        fill_rect(&mut expected, bounds, BACKDROP);
        outline(&mut expected, bounds, BORDER);
        dialog.dialog().draw(&mut expected);
        for toggle in dialog.inert() {
            toggle.draw(false, &MonoMetrics, &mut expected);
        }
        dialog.music().draw(false, &MonoMetrics, &mut expected);
        dialog.sound().draw(false, &MonoMetrics, &mut expected);
        dialog
            .hyperspace_effects()
            .draw(false, &MonoMetrics, &mut expected);
        dialog.effects_volume().draw(false, &mut expected);
        dialog.music_volume().draw(false, &mut expected);
        assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
        // OK, and Key Settings greyed.
        let labels: Vec<(String, Color)> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text, color, font, ..
                } if *font == crate::font::Font::Charcoal => Some((text.clone(), *color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            labels,
            [
                ("OK".to_owned(), ButtonStyle::STOCK.up),
                ("Key Settings".to_owned(), ButtonStyle::STOCK.grey),
            ]
        );
        assert!(commands.iter().any(|command| matches!(
            command,
            DrawCommand::StretchedPicture { image, .. } if *image == ButtonSkin::NOVA.disabled.left
        )));
    }

    #[test]
    fn the_focused_part_is_outlined() {
        let mut dialog = prefs();
        let unfocused = drawn(&dialog);
        dialog.input(&key(Key::Tab));
        let mut music = DrawList::new();
        dialog.music().draw(true, &MonoMetrics, &mut music);
        let commands = drawn(&dialog);
        assert_eq!(commands.len(), unfocused.len() + 4);
        let music: Vec<DrawCommand> = music.iter().cloned().collect();
        assert!(commands.windows(music.len()).any(|window| window == music));
        for _ in 0..3 {
            dialog.input(&key(Key::Tab));
        }
        let mut effects = DrawList::new();
        dialog.effects_volume().draw(true, &mut effects);
        let effects: Vec<DrawCommand> = effects.iter().cloned().collect();
        assert!(
            drawn(&dialog)
                .windows(effects.len())
                .any(|window| window == effects)
        );
        dialog.input(&key(Key::Tab));
        let mut volume = DrawList::new();
        dialog.music_volume().draw(true, &mut volume);
        let volume: Vec<DrawCommand> = volume.iter().cloned().collect();
        assert!(
            drawn(&dialog)
                .windows(volume.len())
                .any(|window| window == volume)
        );
        dialog.input(&key(Key::Tab));
        let mut ring = DrawList::new();
        outline(
            &mut ring,
            dialog.dialog().item_bounds(OK_ITEM).expect("OK"),
            ButtonStyle::STOCK.up,
        );
        let commands = drawn(&dialog);
        assert_eq!(commands.len(), unfocused.len() + 4);
        let ring: Vec<DrawCommand> = ring.iter().cloned().collect();
        assert_eq!(commands[commands.len() - 4..], ring[..], "outlined last");
    }

    #[test]
    fn a_held_arrow_is_drawn_pressed() {
        let mut dialog = prefs();
        let up = dialog.music_volume().rects().up;
        dialog.input(&button(true, up.center()));
        let mut filled = DrawList::new();
        draw_arrow(&mut filled, up, Arrow::Up, true, crate::ui::volume::COLOR);
        let filled: Vec<DrawCommand> = filled.iter().cloned().collect();
        assert!(
            drawn(&dialog)
                .windows(filled.len())
                .any(|window| window == filled)
        );
    }

    #[test]
    fn debug_shows_the_prefs_and_the_focus() {
        let debug = format!("{:?}", prefs());
        assert!(debug.starts_with("PrefsDialog { prefs: Prefs"), "{debug}");
        assert!(debug.contains("focus: None"), "{debug}");
        assert!(debug.contains("closed: false"), "{debug}");
    }
}
