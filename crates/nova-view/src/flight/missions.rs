//! The mission dialogs over flight: the mission texts the session queues
//! in flight, each in "Desc Dialog" as `_ShowTextDialog` (@0x5a752)
//! shows it, and the mission info screen that I opens in flight
//! (`_HandlePlayer` @0x704bc-0x704e3). Flight pauses under them; the
//! router routes to them and answers the info screen's Abort.
//!
//! The same queue shows the `dësc`s a sabotaged player sees as it dies
//! (`_HandlePlayer` @0x68739-0x68832), plain text with no mission behind
//! it ([`FlightMissionDialogs::show_descs`]). Those are text only: the
//! `DLOG` 3004 variant with the `dësc`'s graphic, and its movie, are left
//! to `missions-fidelity/phase-4-offer-pictures-and-movies`.

use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::{MissionInfo, MissionText};

use crate::draw::DrawList;
use crate::input::Input;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::spaceport::missions::MissionInfoScreen;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::desc::DescDialog;
use crate::ui::dialog::DialogTemplate;

/// The mission texts shown over flight, one after another, and the
/// mission info screen, under them.
#[derive(Clone)]
pub struct FlightMissionDialogs {
    /// "Desc Dialog"'s template, or why there is none.
    text: Result<DialogTemplate, String>,
    /// The mission info screen's template, or why there is none.
    info_template: Result<DialogTemplate, String>,
    metrics: Rc<dyn TextMetrics>,
    style: ButtonStyle,
    /// The texts to show, the front one shown.
    texts: VecDeque<DescDialog>,
    /// The mission info screen, while it is open.
    info: Option<MissionInfoScreen>,
    /// The sounds since they were last taken.
    sounds: Vec<Sound>,
    /// Whether the mission info screen closed laid out since this was
    /// last taken.
    info_closed: bool,
}

impl std::fmt::Debug for FlightMissionDialogs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlightMissionDialogs")
            .field("texts", &self.texts)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl FlightMissionDialogs {
    /// The dialogs laid out from `text` ("Desc Dialog", `DLOG` 3003) and
    /// `info` (the mission info screen, `DLOG` 1012), or why each cannot
    /// be, their text measured by `metrics`, their buttons labelled in
    /// `style`. Nothing is open.
    #[must_use]
    pub fn new(
        text: Result<DialogTemplate, String>,
        info: Result<DialogTemplate, String>,
        metrics: Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) -> Self {
        Self {
            text,
            info_template: info,
            metrics,
            style,
            texts: VecDeque::new(),
            info: None,
            sounds: Vec::new(),
            info_closed: false,
        }
    }

    /// Queues each of `texts` to show in "Desc Dialog", one after another
    /// as Done closes each. Without the dialog's template they are
    /// dropped, as the spaceport drops them.
    pub fn show_texts(&mut self, texts: Vec<MissionText>) {
        self.show_descs(texts.into_iter().map(|text| text.text).collect());
    }

    /// Queues each of `texts`, plain text with no mission behind it,
    /// to show as [`show_texts`](Self::show_texts) shows mission texts.
    pub fn show_descs(&mut self, texts: Vec<String>) {
        let Ok(template) = &self.text else {
            return;
        };
        for text in texts {
            self.texts.push_back(DescDialog::new(
                template,
                &text,
                self.style,
                Rc::clone(&self.metrics),
            ));
        }
    }

    /// Opens the mission info screen on `info`, laid out by its template,
    /// or showing why it cannot be; with nothing to list, it does not
    /// open.
    pub fn open_info(&mut self, info: MissionInfo) {
        if info.rows.is_empty() {
            return;
        }
        let layout = self
            .info_template
            .clone()
            .map(|template| (template, Rc::clone(&self.metrics)));
        let mut screen = MissionInfoScreen::new(layout, info, self.style);
        self.sounds.extend(screen.take_sounds());
        self.info = Some(screen);
    }

    /// After an abort, the mission info screen shows `info`, closing once
    /// nothing is left.
    pub fn set_info(&mut self, info: MissionInfo) {
        if let Some(screen) = &mut self.info {
            screen.set_info(info);
            self.sounds.extend(screen.take_sounds());
            if screen.closed() {
                self.close_info();
            }
        }
    }

    /// Whether the mission info screen has closed since this was last
    /// asked, once each time it closes laid out: whoever flies the ship
    /// redraws the whole status bar, as `_DoMissionInfoDialog` ends in
    /// `_RefreshSAT` (@0x9e769) in flight as in port. Without its dialog
    /// the original never shows it and redraws nothing (@0x9e20b).
    pub fn take_info_closed(&mut self) -> bool {
        std::mem::take(&mut self.info_closed)
    }

    /// Closes the mission info screen, noting its close when it was laid
    /// out ([`FlightMissionDialogs::take_info_closed`]).
    fn close_info(&mut self) {
        if self
            .info
            .take()
            .is_some_and(|info| info.problem().is_none())
        {
            self.info_closed = true;
        }
    }

    /// The slot the mission info screen asked to abort since this was
    /// last taken, once.
    pub fn take_abort(&mut self) -> Option<u8> {
        self.info.as_mut()?.take_abort()
    }

    /// The mission text shown, if any.
    #[must_use]
    pub fn text(&self) -> Option<&DescDialog> {
        self.texts.front()
    }

    /// The mission info screen, while it is open.
    #[must_use]
    pub fn info(&self) -> Option<&MissionInfoScreen> {
        self.info.as_ref()
    }

    /// Whether a text or the mission info screen is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        !self.texts.is_empty() || self.info.is_some()
    }
}

impl Screen for FlightMissionDialogs {
    /// The text shown takes every input until Done closes it; otherwise
    /// the mission info screen does. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(text) = self.texts.front_mut() {
            text.input(input);
            self.sounds.extend(text.take_sounds());
            if text.closed() {
                self.texts.pop_front();
            }
        } else if let Some(info) = &mut self.info {
            info.input(input);
            self.sounds.extend(info.take_sounds());
            if info.closed() {
                self.close_info();
            }
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The mission info screen, then the text shown over it.
    fn draw(&self, list: &mut DrawList) {
        if let Some(info) = &self.info {
            info.draw(list);
        }
        if let Some(text) = self.texts.front() {
            text.draw(list);
        }
    }

    fn cancel_pointer(&mut self) {
        if let Some(text) = self.texts.front_mut() {
            text.cancel_pointer();
        } else if let Some(info) = &mut self.info {
            info.cancel_pointer();
        }
    }

    /// The sounds, in order, of the dialogs open and closed. Their
    /// buttons are silent under the pointer, as `_TrackTextButtons`
    /// @0x26bb0 and `_TrackMissionInfoButtons` @0x2850e play nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::{InfoRow, MissionId, MissionTextKind};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::sound::UiSound;
    use crate::spaceport::missions::ABORT_ITEM;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::desc::DONE_ITEM;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    fn user_items(items: &[(f32, f32, f32, f32, bool)]) -> Vec<ItemTemplate> {
        items
            .iter()
            .map(|&(x, y, w, h, enabled)| ItemTemplate {
                bounds: rect(x, y, w, h),
                enabled,
                kind: ItemSpec::User,
            })
            .collect()
    }

    /// "Desc Dialog"'s shape, smaller, as `ui::desc`'s tests have it:
    /// Done (1), a parked picture (2) and the text box (3).
    fn text_template() -> DialogTemplate {
        let mut items = user_items(&[(100.0, 90.0, 99.0, 25.0, true)]);
        items.push(ItemTemplate {
            bounds: rect(10.0, 150.0, 20.0, 20.0),
            enabled: false,
            kind: ItemSpec::Picture(1431),
        });
        items.extend(user_items(&[(10.0, 10.0, 60.0, 36.0, false)]));
        DialogTemplate {
            bounds: rect(0.0, 0.0, 200.0, 120.0),
            placement: Placement::Fixed,
            items,
        }
    }

    /// Stock `DLOG` 1012's shape, as `spaceport::missions`' tests have it.
    fn info_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(40.0, 40.0, 471.0, 155.0),
            placement: Placement::Center,
            items: user_items(&[
                (290.0, 125.0, 99.0, 25.0, true),
                (9.0, 24.0, 195.0, 84.0, true),
                (13.0, 1.0, 193.0, 12.0, true),
                (218.0, 26.0, 242.0, 91.0, false),
                (57.0, 125.0, 99.0, 25.0, true),
                (177.0, 222.0, 55.0, 37.0, true),
                (343.0, 4.0, 122.0, 12.0, false),
            ]),
        }
    }

    fn dialogs() -> FlightMissionDialogs {
        FlightMissionDialogs::new(
            Ok(text_template()),
            Ok(info_template()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        )
    }

    fn brief(mission: i16, text: &str) -> MissionText {
        MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(mission),
            text: text.to_owned(),
        }
    }

    /// Slot 2's run, abortable.
    fn info() -> MissionInfo {
        MissionInfo {
            rows: vec![InfoRow {
                slot: 2,
                name: "Run".to_owned(),
                failed: false,
                text: "Go.".to_owned(),
                abortable: true,
            }],
            today: "June 23rd, 1177 NC".to_owned(),
        }
    }

    fn none() -> MissionInfo {
        MissionInfo {
            rows: Vec::new(),
            today: String::new(),
        }
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click(dialogs: &mut FlightMissionDialogs, at: Point) {
        for pressed in [true, false] {
            dialogs.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn shown(dialogs: &FlightMissionDialogs) -> Option<String> {
        dialogs.text().map(|text| text.lines().join(" "))
    }

    fn drawn(dialogs: &FlightMissionDialogs) -> Vec<String> {
        let mut list = DrawList::new();
        dialogs.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn texts_show_one_after_another_and_close_on_done() {
        let mut dialogs = dialogs();
        assert!(!dialogs.is_open());
        dialogs.show_texts(vec![brief(128, "First."), brief(129, "Second.")]);
        assert!(dialogs.is_open());
        assert_eq!(shown(&dialogs).as_deref(), Some("First."));
        assert!(drawn(&dialogs).contains(&"First.".to_owned()));
        let done = dialogs
            .text()
            .and_then(|text| text.dialog().item_bounds(DONE_ITEM))
            .expect("Done")
            .center();
        click(&mut dialogs, done);
        assert_eq!(dialogs.take_sounds(), []);
        assert_eq!(shown(&dialogs).as_deref(), Some("Second."));
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs), None);
        assert!(!dialogs.is_open());
    }

    #[test]
    fn plain_texts_show_one_after_another_and_close_on_done() {
        let mut dialogs = dialogs();
        dialogs.show_descs(vec!["Sabotaged.".to_owned(), "Again.".to_owned()]);
        assert!(dialogs.is_open());
        assert_eq!(shown(&dialogs).as_deref(), Some("Sabotaged."));
        assert!(drawn(&dialogs).contains(&"Sabotaged.".to_owned()));
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs).as_deref(), Some("Again."));
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs), None);
        assert!(!dialogs.is_open());
    }

    #[test]
    fn the_info_screen_opens_only_with_rows_and_closes_once_empty() {
        let mut dialogs = dialogs();
        dialogs.open_info(none());
        assert!(dialogs.info().is_none());
        assert!(!dialogs.is_open());
        dialogs.open_info(info());
        assert!(dialogs.is_open());
        let abort = dialogs
            .info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center();
        click(&mut dialogs, abort);
        assert_eq!(
            dialogs.take_sounds(),
            [Sound::Ui(UiSound::MissionInfoOpened)]
        );
        assert_eq!(dialogs.take_abort(), Some(2));
        assert_eq!(dialogs.take_abort(), None, "once");
        dialogs.set_info(info());
        assert!(dialogs.is_open(), "still listed");
        dialogs.set_info(none());
        assert!(dialogs.info().is_none());
        assert!(!dialogs.is_open());
    }

    #[test]
    fn the_info_screens_beeps_come_out_as_it_opens_and_as_an_abort_closes_it() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        assert_eq!(
            dialogs.take_sounds(),
            [Sound::Ui(UiSound::MissionInfoOpened)]
        );
        dialogs.set_info(none());
        assert!(dialogs.info().is_none());
        assert_eq!(
            dialogs.take_sounds(),
            [Sound::Ui(UiSound::MissionInfoClosed)]
        );
    }

    #[test]
    fn the_info_screens_close_is_told_once_when_it_was_laid_out() {
        let mut dialogs = dialogs();
        assert!(!dialogs.take_info_closed(), "nothing open");
        dialogs.open_info(info());
        assert!(!dialogs.take_info_closed(), "opening");
        dialogs.input(&key(Key::Down));
        assert!(!dialogs.take_info_closed(), "still open");
        dialogs.input(&key(Key::Enter));
        assert!(dialogs.take_info_closed(), "Done (@0x9e769)");
        assert!(!dialogs.take_info_closed(), "once");
        dialogs.open_info(info());
        dialogs.set_info(none());
        assert!(dialogs.take_info_closed(), "an abort leaving nothing");
        let mut bare = FlightMissionDialogs::new(
            Ok(text_template()),
            Err("no DLOG 1012".to_owned()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        bare.open_info(info());
        bare.input(&key(Key::Escape));
        assert!(bare.info().is_none());
        assert!(!bare.take_info_closed(), "without its dialog (@0x9e20b)");
        bare.open_info(info());
        bare.set_info(none());
        assert!(!bare.take_info_closed(), "without its dialog, by an abort");
    }

    #[test]
    fn done_closes_the_info_screen() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        dialogs.input(&key(Key::Enter));
        assert!(dialogs.info().is_none());
        assert!(!dialogs.is_open());
    }

    #[test]
    fn without_its_template_the_info_screen_says_why_and_closes_on_return() {
        let mut dialogs = FlightMissionDialogs::new(
            Ok(text_template()),
            Err("no DLOG 1012".to_owned()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        dialogs.open_info(info());
        assert_eq!(
            dialogs.info().and_then(MissionInfoScreen::problem),
            Some("no DLOG 1012")
        );
        dialogs.input(&key(Key::Escape));
        assert!(!dialogs.is_open());
    }

    #[test]
    fn a_text_shown_over_the_info_screen_takes_the_input_first() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        dialogs.show_texts(vec![brief(128, "Over.")]);
        let words = drawn(&dialogs);
        let run = words.iter().position(|text| text == "Run").expect("Run");
        let over = words
            .iter()
            .position(|text| text == "Over.")
            .expect("Over.");
        assert!(run < over, "{words:?}");
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs), None);
        assert!(dialogs.info().is_some(), "the info screen stays open");
        assert!(dialogs.is_open());
    }

    #[test]
    fn a_pointer_cancelled_goes_to_the_text_shown_over_the_info_screen() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        dialogs.show_texts(vec![brief(128, "Over.")]);
        let done = dialogs
            .text()
            .and_then(|text| text.dialog().item_bounds(DONE_ITEM))
            .expect("Done")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: done,
        };
        dialogs.input(&button(true));
        dialogs.cancel_pointer();
        dialogs.input(&button(false));
        assert_eq!(shown(&dialogs).as_deref(), Some("Over."), "abandoned");
    }

    #[test]
    fn a_pointer_cancelled_goes_to_the_info_screen_with_no_text() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        let abort = dialogs
            .info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: abort,
        };
        dialogs.input(&button(true));
        dialogs.cancel_pointer();
        dialogs.input(&button(false));
        assert_eq!(dialogs.take_abort(), None, "abandoned");
    }

    #[test]
    fn it_debugs_as_its_texts_and_info_screen() {
        let mut dialogs = dialogs();
        dialogs.show_texts(vec![brief(128, "Shown.")]);
        let debugged = format!("{dialogs:?}");
        assert!(
            debugged.starts_with("FlightMissionDialogs { texts: ["),
            "{debugged}"
        );
        assert!(debugged.contains("info: None"), "{debugged}");
    }

    #[test]
    fn without_a_text_template_texts_are_dropped() {
        let mut dialogs = FlightMissionDialogs::new(
            Err("no DLOG 3003".to_owned()),
            Ok(info_template()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        dialogs.show_texts(vec![brief(128, "Lost.")]);
        assert!(dialogs.text().is_none());
        assert!(!dialogs.is_open());
    }
}
