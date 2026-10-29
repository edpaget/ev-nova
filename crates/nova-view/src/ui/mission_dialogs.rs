//! The mission dialogs, over the spaceport or over flight: the mission
//! texts the session queues, each in "Desc Dialog" as `_ShowTextDialog`
//! (@0x5a752) shows it, and, under them, the mission info screen that I
//! opens (in flight, `_HandlePlayer` @0x704bc-0x704e3). Whatever holds
//! them (the spaceport, or the router over flight) passes its input,
//! drawing and pointer through, and answers the info screen's Abort and
//! the text it asks for. It learns when the info screen closes
//! ([`MissionDialogs::take_info_closed`]), as the spaceport does to let
//! go of the keys of the screen beneath.
//!
//! In flight the same queue shows the `dësc`s a sabotaged player sees as
//! it dies (`_HandlePlayer` @0x68739-0x68832) and the rescue's, with no
//! mission behind them ([`MissionDialogs::show_descs`]); in the
//! spaceport, a refused mission computer's reason.
//!
//! Each shows in the variant its `dësc`'s `Graphic` picks
//! ([`DescTemplates::open`]): above 127, `DLOG` 3004 with the graphic, as
//! every one of these callers allows a picture (`_ShowTextDialog`'s third
//! argument 1). An empty text shows no dialog (@0x5a7b0-0x5a7c7).
//!
//! A text whose `dësc` names a movie ([`DescMovie`]) plays it, while
//! movies are on ([`Movies`]): before its dialog (@0x5a76e-0x5a7ab, even
//! for an empty text), or, `Flags` 0x0001 set, after it closes
//! (@0x5b40b-0x5b44f; never for an empty text, which returns first). Each
//! plays as [`MovieStage`] plays it, over the info screen, taking every
//! input. A movie that ends on its own hands the keys pressed under it to
//! what comes next. An offer's movie the session queued among the texts
//! ([`MissionShown::Movie`]) plays in its place.

use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use nova_sim::{DescMovie, MapMarks, MissionInfo, MissionShown};

use crate::draw::DrawList;
#[cfg(doc)]
use crate::flight::FlightView;
use crate::flight::MapOpener;
use crate::input::Input;
use crate::movie::{MovieStage, Movies};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::spaceport::missions::MissionInfoScreen;
use crate::text::TextMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::desc::{DescDialog, DescTemplates};
use crate::ui::dialog::{DialogTemplate, FocusRing};

/// A `dësc`'s text shown with no mission behind it: its text, its
/// `Graphic` and its movie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DescShown {
    /// The text.
    pub text: String,
    /// Its `Graphic`, raw.
    pub graphic: Option<i16>,
    /// The movie it names.
    pub movie: Option<DescMovie>,
}

/// One of the things shown one after another: a movie or a text, or
/// the mark that a mission text is done.
#[derive(Clone, Debug)]
enum Stage {
    /// A `dësc`'s movie.
    Movie(MovieStage),
    /// A text in "Desc Dialog".
    Text(DescDialog),
    /// A mission text and its movies are done, shown or dropped
    /// ([`MissionDialogs::take_texts_closed`]).
    Closed,
}

/// The mission texts shown one after another, and the mission info
/// screen, under them.
#[derive(Clone)]
pub struct MissionDialogs {
    /// "Desc Dialog"'s templates, both variants, or why each is missing.
    text: DescTemplates,
    /// The mission info screen's template, or why there is none.
    info_template: Result<DialogTemplate, String>,
    metrics: Rc<dyn TextMetrics>,
    style: ButtonStyle,
    /// Whether the texts' and the info screen's buttons take the keyboard
    /// focus.
    focus_ring: FocusRing,
    /// The texts and movies to show, the front one shown.
    texts: VecDeque<Stage>,
    /// The movies, and whether they play.
    movies: Movies,
    /// The mission info screen, while it is open.
    info: Option<MissionInfoScreen>,
    /// The sounds since they were last taken.
    sounds: Vec<Sound>,
    /// Whether the mission info screen has closed since this was last
    /// taken.
    info_closed: bool,
    /// Whether the mission info screen closed laid out since this was
    /// last taken.
    info_closed_laid_out: bool,
    /// What was pressed under a movie that ended with nothing after it,
    /// for what is beneath, until it is taken.
    unclaimed: Vec<Input>,
    /// How many mission texts have closed since this was last taken.
    texts_closed: usize,
}

impl std::fmt::Debug for MissionDialogs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MissionDialogs")
            .field("texts", &self.texts)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

impl MissionDialogs {
    /// The dialogs laid out from `text` ("Desc Dialog", `DLOG` 3003, and
    /// its picture variant, 3004) and `info` (the mission info screen,
    /// `DLOG` 1012), or why each cannot be, their text measured by
    /// `metrics`, their buttons labelled in `style`. Nothing is open.
    #[must_use]
    pub fn new(
        text: DescTemplates,
        info: Result<DialogTemplate, String>,
        metrics: Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) -> Self {
        Self {
            text,
            info_template: info,
            metrics,
            style,
            focus_ring: FocusRing::default(),
            texts: VecDeque::new(),
            movies: Movies::none(),
            info: None,
            sounds: Vec::new(),
            info_closed: false,
            info_closed_laid_out: false,
            unclaimed: Vec::new(),
            texts_closed: 0,
        }
    }

    /// These dialogs with their buttons taking the keyboard focus as
    /// `ring` says (the `dialog_focus_ring` rule), for each opened after it.
    #[must_use]
    pub fn with_focus_ring(self, ring: FocusRing) -> Self {
        Self {
            focus_ring: ring,
            ..self
        }
    }

    /// These dialogs playing the texts' movies as `movies` says.
    #[must_use]
    pub fn with_movies(self, movies: Movies) -> Self {
        Self { movies, ..self }
    }

    /// Queues each of `texts` to show in "Desc Dialog", in the variant
    /// its graphic picks, one after another as Done closes each, with its
    /// movie before or after it, and each offer movie among them in the
    /// place the session gave it. Without that variant's template the
    /// dialog is dropped, as the spaceport drops it, and its movies still
    /// play. Each mission text is told closed once it and its movies are
    /// done ([`MissionDialogs::take_texts_closed`]).
    pub fn show_texts(&mut self, texts: impl IntoIterator<Item = impl Into<MissionShown>>) {
        for shown in texts {
            match shown.into() {
                MissionShown::Text(text) => {
                    let map = text.map;
                    self.queue_desc_with_map(
                        &DescShown {
                            text: text.text,
                            graphic: text.graphic,
                            movie: text.movie,
                        },
                        map,
                    );
                    self.texts.push_back(Stage::Closed);
                }
                MissionShown::Movie(movie) => self.queue_movie(&movie),
            }
        }
        self.settle();
    }

    /// How many of the mission texts shown ([`MissionDialogs::show_texts`])
    /// have closed since this was last taken, in order, each once: closed
    /// by its Done and its after-movie ended, or dropped, as an empty text
    /// or one with no template is. `_ShowTextDialog` returns there
    /// (@0x5b40b-0x5b44f), and every caller of a mission text redraws the
    /// whole bar as it returns (`_RefreshSAT`, e.g. @0xa1f13), which
    /// whoever flies the ship then does
    /// ([`FlightView::close_mission_text`](crate::flight::FlightView::close_mission_text)).
    /// A `dësc` with no mission ([`MissionDialogs::show_descs`]) is not
    /// counted.
    pub fn take_texts_closed(&mut self) -> usize {
        std::mem::take(&mut self.texts_closed)
    }

    /// Lets go of the marks of the mission texts done that are in front,
    /// counting each closed.
    fn pop_closed(&mut self) {
        while matches!(self.texts.front(), Some(Stage::Closed)) {
            self.texts.pop_front();
            self.texts_closed += 1;
        }
    }

    /// Queues each of `texts`, a `dësc` with no mission behind it, to
    /// show as [`show_texts`](Self::show_texts) shows mission texts.
    pub fn show_descs(&mut self, texts: Vec<DescShown>) {
        for shown in texts {
            self.queue_desc(&shown);
        }
        self.settle();
    }

    /// Queues `shown`: its movie before it, its dialog (none for an empty
    /// text, which plays no movie after), and its movie after it. It
    /// opens no map.
    fn queue_desc(&mut self, shown: &DescShown) {
        self.queue_desc_with_map(shown, None);
    }

    /// [`MissionDialogs::queue_desc`], its dialog opening the map marked
    /// `map` on the map key, or none.
    fn queue_desc_with_map(&mut self, shown: &DescShown, map: Option<MapMarks>) {
        let movie = |after: bool| shown.movie.as_ref().filter(|movie| movie.after == after);
        if let Some(before) = movie(false) {
            self.queue_movie(before);
        }
        if shown.text.is_empty() {
            return;
        }
        if let Ok(dialog) = self.text.open(
            &shown.text,
            shown.graphic,
            self.style,
            Rc::clone(&self.metrics),
        ) {
            self.texts.push_back(Stage::Text(
                dialog.with_map(map).with_focus_ring(self.focus_ring),
            ));
        }
        if let Some(after) = movie(true) {
            self.queue_movie(after);
        }
    }

    /// Queues `movie` after whatever is queued, while movies are on.
    fn queue_movie(&mut self, movie: &DescMovie) {
        if let Some(stage) = self.movies.stage(movie) {
            self.texts.push_back(Stage::Movie(stage));
        }
    }

    /// Begins the movie in front, if one is; drops it once it has ended,
    /// handing what was pressed under it to what comes next, until a
    /// text, or a movie still playing, is in front.
    fn settle(&mut self) {
        self.pop_closed();
        while let Some(Stage::Movie(movie)) = self.texts.front_mut() {
            movie.begin();
            if !movie.ended() {
                return;
            }
            let replay = movie.take_replay();
            self.texts.pop_front();
            self.pop_closed();
            for input in replay {
                if self.is_open() {
                    self.front_input(&input);
                } else {
                    self.unclaimed.push(input);
                }
            }
        }
    }

    /// What was pressed under a movie that ended with nothing left open
    /// here, once: it goes to what is beneath, as the original's queued
    /// events reach the next event loop.
    pub fn take_unclaimed(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.unclaimed)
    }

    /// Gives `input` to what is in front: the movie or text shown, or else
    /// the mission info screen.
    fn front_input(&mut self, input: &Input) {
        self.pop_closed();
        match self.texts.front_mut() {
            Some(Stage::Movie(movie)) => movie.input(input),
            Some(Stage::Text(text)) => {
                text.input(input);
                self.sounds.extend(text.take_sounds());
                if text.closed() {
                    self.texts.pop_front();
                    self.pop_closed();
                }
            }
            Some(Stage::Closed) => {}
            None => {
                if let Some(info) = &mut self.info {
                    info.input(input);
                    self.sounds.extend(info.take_sounds());
                    if info.closed() {
                        self.close_info();
                    }
                }
            }
        }
    }

    /// Opens the mission info screen on `info`, laid out by its template,
    /// or showing why it cannot be; with nothing to list, it does not
    /// open. Its map key opens the map
    /// ([`MissionDialogs::open_info_with_map`]).
    pub fn open_info(&mut self, info: MissionInfo) {
        self.open_info_with_map(info, true);
    }

    /// [`MissionDialogs::open_info`], its map key opening the map when
    /// `map_allowed`: not when it opens over an offer or the mission
    /// computer, as `_missionDialog` is set then
    /// ([`MissionInfoScreen::with_map_allowed`]).
    pub fn open_info_with_map(&mut self, info: MissionInfo, map_allowed: bool) {
        if info.rows.is_empty() {
            return;
        }
        let layout = self
            .info_template
            .clone()
            .map(|template| (template, Rc::clone(&self.metrics)));
        let mut screen = MissionInfoScreen::new(layout, info, self.style)
            .with_map_allowed(map_allowed)
            .with_focus_ring(self.focus_ring);
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

    /// The slot the mission info screen asked to abort since this was
    /// last taken, once.
    pub fn take_abort(&mut self) -> Option<u8> {
        self.info.as_mut()?.take_abort()
    }

    /// The slot the mission info screen has shown selected since this
    /// was last taken, once, whose `QuickBrief` is to be loaded: whoever
    /// flies the ship answers with
    /// [`MissionDialogs::show_info_text`].
    pub fn take_info_text_request(&mut self) -> Option<u8> {
        self.info.as_mut()?.take_text_request()
    }

    /// Shows `text`, slot `slot`'s loaded, in the mission info screen
    /// while that slot is selected.
    pub fn show_info_text(&mut self, slot: u8, text: &str) {
        if let Some(screen) = &mut self.info {
            screen.show_text(slot, text);
        }
    }

    /// The map the text shown or the mission info screen asked for since
    /// this was last taken, once: the accept's briefing's, marked as the
    /// session queued it, or mission info's, for its selected slot. A
    /// mission dialog's map ([`FlightView::open_dialog_map`]) opens over
    /// them, and they stay open under it.
    pub fn take_map_request(&mut self) -> Option<MapOpener> {
        let text = self.texts.iter_mut().find_map(|stage| match stage {
            Stage::Text(text) => text.take_map_request(),
            Stage::Movie(_) | Stage::Closed => None,
        });
        let info = self
            .info
            .as_mut()
            .and_then(MissionInfoScreen::take_map_request);
        text.map(MapOpener::Briefing).or(info)
    }

    /// The mission text shown, if any: none while a movie plays.
    #[must_use]
    pub fn text(&self) -> Option<&DescDialog> {
        match self.texts.front() {
            Some(Stage::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// Whether a movie is in front, playing or about to.
    #[must_use]
    pub fn playing(&self) -> bool {
        matches!(self.texts.front(), Some(Stage::Movie(_)))
    }

    /// The mission info screen, while it is open.
    #[must_use]
    pub fn info(&self) -> Option<&MissionInfoScreen> {
        self.info.as_ref()
    }

    /// Whether the mission info screen has closed since this was last
    /// taken, once: by its own input, or by [`set_info`](Self::set_info)
    /// leaving nothing; never by a text closing over it.
    pub fn take_info_closed(&mut self) -> bool {
        std::mem::take(&mut self.info_closed)
    }

    /// Whether the mission info screen has closed laid out since this was
    /// last asked, once each time: whoever flies the ship redraws the
    /// whole status bar, as `_DoMissionInfoDialog` ends in `_RefreshSAT`
    /// (@0x9e769) in flight as in port. Without its dialog the original
    /// never shows it and redraws nothing (@0x9e20b).
    pub fn take_info_closed_laid_out(&mut self) -> bool {
        std::mem::take(&mut self.info_closed_laid_out)
    }

    /// Closes the mission info screen, noting its close
    /// ([`MissionDialogs::take_info_closed`]), and, when it was laid out,
    /// that too ([`MissionDialogs::take_info_closed_laid_out`]).
    fn close_info(&mut self) {
        if let Some(info) = self.info.take() {
            self.info_closed = true;
            self.info_closed_laid_out |= info.problem().is_none();
        }
    }

    /// Whether a text or a movie is queued, the first shown.
    #[must_use]
    pub fn texts_queued(&self) -> bool {
        !self.texts.is_empty()
    }

    /// Whether a text, a movie or the mission info screen is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        !self.texts.is_empty() || self.info.is_some()
    }
}

impl Screen for MissionDialogs {
    /// The movie or text shown takes every input until it ends or Done
    /// closes it; otherwise the mission info screen does. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        self.front_input(input);
        self.settle();
        ScreenAction::None
    }

    /// The movie shown moves on; the rest wait on input.
    fn tick(&mut self, dt: Duration) {
        if let Some(Stage::Movie(movie)) = self.texts.front_mut() {
            movie.tick(dt);
            self.settle();
        }
    }

    /// The mission info screen, then the text or movie shown over it.
    fn draw(&self, list: &mut DrawList) {
        if let Some(info) = &self.info {
            info.draw(list);
        }
        match self.texts.front() {
            Some(Stage::Text(text)) => text.draw(list),
            Some(Stage::Movie(movie)) => movie.draw(list),
            Some(Stage::Closed) | None => {}
        }
    }

    fn cancel_pointer(&mut self) {
        match self.texts.front_mut() {
            Some(Stage::Text(text)) => text.cancel_pointer(),
            Some(Stage::Movie(_) | Stage::Closed) => {}
            None => {
                if let Some(info) = &mut self.info {
                    info.cancel_pointer();
                }
            }
        }
    }

    /// The sounds, in order, of the dialogs open and closed. Their
    /// buttons are silent under the pointer, as `_TrackTextButtons`
    /// @0x26bb0 and `_TrackMissionInfoButtons` @0x2850e play nothing.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// While a cinematic movie hides the cursor.
    fn hides_pointer(&self) -> bool {
        matches!(self.texts.front(), Some(Stage::Movie(movie)) if movie.hides_cursor())
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::{InfoRow, MapMarks, MissionId, MissionText, MissionTextKind};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::flight::MapOpener;
    use crate::geometry::{Bounds, Point};
    use crate::input::{Key, MouseButton};
    use crate::movie::fixture::{Call, MockMoviePlayer};
    use crate::movie::{MOVIE_FADE_FOR, MoviePlayer};
    use crate::sound::UiSound;
    use crate::spaceport::missions::ABORT_ITEM;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::desc::DONE_ITEM;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};
    use nova_sim::DescMovie;

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

    /// The picture variant's shape: [`text_template`], 300 wide.
    fn picture_template() -> DialogTemplate {
        DialogTemplate {
            bounds: rect(0.0, 0.0, 300.0, 120.0),
            ..text_template()
        }
    }

    fn texts(plain: Result<DialogTemplate, String>) -> DescTemplates {
        DescTemplates {
            plain,
            picture: Ok(picture_template()),
        }
    }

    fn dialogs() -> MissionDialogs {
        MissionDialogs::new(
            texts(Ok(text_template())),
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
            graphic: None,
            movie: None,
            map: None,
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

    fn click(dialogs: &mut MissionDialogs, at: Point) {
        for pressed in [true, false] {
            dialogs.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn shown(dialogs: &MissionDialogs) -> Option<String> {
        dialogs.text().map(|text| text.lines().join(" "))
    }

    fn drawn(dialogs: &MissionDialogs) -> Vec<String> {
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
    fn the_briefings_map_key_asks_for_its_marked_map_and_another_texts_none() {
        let m = crate::flight::view::MAP_KEY;
        let marks = MapMarks {
            green: Some(nova_sim::SystemId(131)),
            multi_arrows: true,
        };
        let mut dialogs = dialogs();
        dialogs.show_texts(vec![
            MissionText {
                map: Some(marks),
                ..brief(128, "Off you go.")
            },
            brief(129, "Pity."),
        ]);
        dialogs.input(&key(m));
        assert_eq!(dialogs.take_map_request(), Some(MapOpener::Briefing(marks)));
        assert_eq!(dialogs.take_map_request(), None, "once");
        assert_eq!(
            shown(&dialogs).as_deref(),
            Some("Off you go."),
            "open under it"
        );
        dialogs.input(&key(Key::Enter));
        dialogs.input(&key(m));
        assert_eq!(dialogs.take_map_request(), None, "no map from this one");
        assert_eq!(dialogs.take_sounds(), [], "silent");
    }

    #[test]
    fn mission_infos_map_key_asks_for_the_map_of_its_selected_slot_unless_refused() {
        let m = crate::flight::view::MAP_KEY;
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        dialogs.input(&key(m));
        assert_eq!(
            dialogs.take_map_request(),
            Some(MapOpener::MissionInfo(Some(2)))
        );
        let mut dialogs = dialogs_with_info_over_a_mission_dialog();
        dialogs.input(&key(m));
        assert_eq!(dialogs.take_map_request(), None);
        assert!(dialogs.info().is_some(), "still open");
    }

    /// [`dialogs`] with the mission info screen open over an offer or the
    /// computer, where it opens no map.
    fn dialogs_with_info_over_a_mission_dialog() -> MissionDialogs {
        let mut dialogs = dialogs();
        dialogs.open_info_with_map(info(), false);
        dialogs
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
        dialogs.show_descs(vec![
            shown_desc("Sabotaged.", None),
            shown_desc("Again.", None),
        ]);
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
        dialogs.take_sounds();
        let abort = dialogs
            .info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.item_bounds(ABORT_ITEM))
            .expect("Abort")
            .center();
        click(&mut dialogs, abort);
        assert_eq!(dialogs.take_sounds(), []);
        assert_eq!(dialogs.take_abort(), Some(2));
        assert_eq!(dialogs.take_abort(), None, "once");
        dialogs.set_info(info());
        assert!(dialogs.is_open(), "still listed");
        dialogs.set_info(none());
        assert!(dialogs.info().is_none());
        assert!(!dialogs.is_open());
    }

    #[test]
    fn the_info_screen_beeps_as_it_opens_and_as_its_last_abort_closes_it() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        assert_eq!(
            dialogs.take_sounds(),
            [Sound::Ui(UiSound::MissionInfoOpened)]
        );
        dialogs.set_info(none());
        assert_eq!(
            dialogs.take_sounds(),
            [Sound::Ui(UiSound::MissionInfoClosed)]
        );
    }

    #[test]
    fn the_info_screens_selected_text_is_asked_for_and_shown() {
        let mut dialogs = dialogs();
        assert_eq!(dialogs.take_info_text_request(), None, "nothing open");
        dialogs.show_info_text(2, "Nowhere to show.");
        dialogs.open_info(info());
        assert_eq!(dialogs.take_info_text_request(), Some(2));
        assert_eq!(dialogs.take_info_text_request(), None, "once");
        dialogs.show_info_text(2, "Loaded.");
        let text = dialogs
            .info()
            .and_then(MissionInfoScreen::dialog)
            .and_then(|dialog| dialog.text(crate::spaceport::missions::INFO_TEXT_ITEM))
            .expect("the text")
            .lines()
            .join(" ");
        assert_eq!(text, "Loaded.");
    }

    #[test]
    fn the_info_screens_close_is_told_once_when_it_was_laid_out() {
        let mut dialogs = dialogs();
        assert!(!dialogs.take_info_closed_laid_out(), "nothing open");
        dialogs.open_info(info());
        assert!(!dialogs.take_info_closed_laid_out(), "opening");
        dialogs.input(&key(Key::Down));
        assert!(!dialogs.take_info_closed_laid_out(), "still open");
        dialogs.input(&key(Key::Enter));
        assert!(dialogs.take_info_closed_laid_out(), "Done (@0x9e769)");
        assert!(!dialogs.take_info_closed_laid_out(), "once");
        dialogs.open_info(info());
        dialogs.set_info(none());
        assert!(
            dialogs.take_info_closed_laid_out(),
            "an abort leaving nothing"
        );
        let mut bare = MissionDialogs::new(
            texts(Ok(text_template())),
            Err("no DLOG 1012".to_owned()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        bare.open_info(info());
        bare.input(&key(Key::Escape));
        assert!(bare.info().is_none());
        assert!(bare.take_info_closed(), "closed all the same");
        assert!(
            !bare.take_info_closed_laid_out(),
            "without its dialog (@0x9e20b)"
        );
        bare.open_info(info());
        bare.set_info(none());
        assert!(bare.take_info_closed());
        assert!(
            !bare.take_info_closed_laid_out(),
            "without its dialog, by an abort"
        );
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
        let mut dialogs = MissionDialogs::new(
            texts(Ok(text_template())),
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
            debugged.starts_with("MissionDialogs { texts: ["),
            "{debugged}"
        );
        assert!(debugged.contains("info: None"), "{debugged}");
    }

    #[test]
    fn without_a_text_template_texts_are_dropped() {
        let mut dialogs = MissionDialogs::new(
            texts(Err("no DLOG 3003".to_owned())),
            Ok(info_template()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        dialogs.show_texts(vec![brief(128, "Lost.")]);
        assert!(dialogs.text().is_none());
        assert!(!dialogs.is_open());
    }

    #[test]
    fn each_mission_text_close_is_reported_once() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![
            brief(128, "First."),
            brief(129, ""),
            filmed("Last.", 0x0001),
        ]);
        dialogs.show_descs(vec![shown_desc("No mission.", None)]);
        assert_eq!(dialogs.take_texts_closed(), 0, "the first shows");
        dialogs.input(&key(Key::Enter));
        assert_eq!(
            dialogs.take_texts_closed(),
            2,
            "the first, then the empty one"
        );
        assert_eq!(dialogs.take_texts_closed(), 0, "once");
        dialogs.input(&key(Key::Enter));
        assert_eq!(dialogs.take_texts_closed(), 0, "its after-movie plays");
        dialogs.tick(TICK);
        assert_eq!(dialogs.take_texts_closed(), 1, "after its after-movie");
        assert_eq!(shown(&dialogs).as_deref(), Some("No mission."));
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open());
        assert_eq!(dialogs.take_texts_closed(), 0, "a `dësc` with no mission");
        let mut dropped = MissionDialogs::new(
            texts(Err("no DLOG 3003".to_owned())),
            Ok(info_template()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        dropped.show_texts(vec![brief(128, "Lost."), brief(129, "Gone.")]);
        assert_eq!(dropped.take_texts_closed(), 2, "dropped, so closed at once");
    }

    fn widths(dialogs: &mut MissionDialogs) -> Vec<(f32, Option<i16>)> {
        let mut shown = Vec::new();
        while let Some(text) = dialogs.text() {
            shown.push((text.dialog().bounds().width(), text.picture()));
            dialogs.input(&key(Key::Enter));
        }
        shown
    }

    #[test]
    fn a_mission_text_with_a_graphic_shows_in_3004() {
        let mut dialogs = dialogs();
        dialogs.show_texts(vec![
            MissionText {
                graphic: Some(5008),
                ..brief(128, "Pictured.")
            },
            brief(129, "Plain."),
        ]);
        assert_eq!(widths(&mut dialogs), [(300.0, Some(5008)), (200.0, None)]);
    }

    #[test]
    fn a_desc_with_a_graphic_shows_in_3004() {
        let mut dialogs = dialogs();
        dialogs.show_descs(vec![
            shown_desc("Sabotaged.", Some(200)),
            shown_desc("Rescued.", Some(127)),
        ]);
        assert_eq!(widths(&mut dialogs), [(300.0, Some(200)), (200.0, None)]);
    }

    #[test]
    fn the_info_screen_closing_on_input_is_reported_once() {
        for close in [Key::Escape, Key::Enter] {
            let mut dialogs = dialogs();
            dialogs.open_info(info());
            assert!(!dialogs.take_info_closed(), "{close:?}: just opened");
            dialogs.input(&key(close));
            assert!(dialogs.info().is_none(), "{close:?}");
            assert!(dialogs.take_info_closed(), "{close:?}");
            assert!(!dialogs.take_info_closed(), "{close:?}: once");
        }
    }

    #[test]
    fn a_text_closing_over_the_info_screen_is_not_an_info_close() {
        let mut dialogs = dialogs();
        dialogs.open_info(info());
        dialogs.show_texts(vec![brief(128, "Over.")]);
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs), None);
        assert!(!dialogs.take_info_closed());
        let mut dialogs = dialogs_with_text_only();
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open());
        assert!(!dialogs.take_info_closed(), "a text alone");
    }

    fn dialogs_with_text_only() -> MissionDialogs {
        let mut dialogs = dialogs();
        dialogs.show_texts(vec![brief(128, "Alone.")]);
        dialogs
    }

    #[test]
    fn the_info_screen_closing_on_set_info_is_reported_once() {
        let mut dialogs = dialogs();
        dialogs.set_info(none());
        assert!(!dialogs.take_info_closed(), "nothing was open");
        dialogs.open_info(info());
        dialogs.set_info(info());
        assert!(!dialogs.take_info_closed(), "rows remain");
        dialogs.set_info(none());
        assert!(dialogs.take_info_closed());
        assert!(!dialogs.take_info_closed(), "once");
    }

    fn shown_desc(text: &str, graphic: Option<i16>) -> DescShown {
        DescShown {
            text: text.to_owned(),
            graphic,
            movie: None,
        }
    }

    fn film(flags: u16) -> Option<DescMovie> {
        DescMovie::from_desc("Brief.mov", flags)
    }

    fn filmed(text: &str, flags: u16) -> MissionText {
        MissionText {
            movie: film(flags),
            ..brief(128, text)
        }
    }

    /// The dialogs, playing movies through `player`.
    fn playing(player: &Rc<MockMoviePlayer>) -> MissionDialogs {
        dialogs().with_movies(Movies::new(Rc::clone(player) as Rc<dyn MoviePlayer>, true))
    }

    const TICK: Duration = Duration::from_millis(100);

    #[test]
    fn a_movie_desc_with_flag_clear_plays_before_its_text() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("Briefed.", 0)]);
        assert_eq!(player.calls(), [Call::Start("Brief.mov".to_owned())]);
        assert!(dialogs.is_open());
        assert_eq!(shown(&dialogs), None, "no text while the movie plays");
        assert!(drawn(&dialogs).is_empty(), "the text is not drawn");
        assert_eq!(player.drawn().len(), 1, "the movie is");
        dialogs.input(&key(Key::Char('q')));
        assert_eq!(shown(&dialogs), None, "the movie swallows input");
        dialogs.tick(TICK);
        dialogs.tick(TICK);
        assert_eq!(shown(&dialogs).as_deref(), Some("Briefed."));
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open());
        assert_eq!(
            player.calls(),
            [
                Call::Start("Brief.mov".to_owned()),
                Call::Advance,
                Call::Advance,
                Call::Stop
            ]
        );
    }

    #[test]
    fn it_says_whether_a_movie_is_in_front() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        assert!(!dialogs.playing());
        dialogs.show_texts(vec![filmed("After.", 0x0001)]);
        assert!(!dialogs.playing(), "the text first");
        dialogs.input(&key(Key::Enter));
        assert!(dialogs.playing());
        dialogs.tick(TICK);
        assert!(!dialogs.playing());
    }

    #[test]
    fn a_movie_desc_with_flag_set_plays_after_its_text() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("Briefed.", 0x0001)]);
        assert_eq!(shown(&dialogs).as_deref(), Some("Briefed."));
        assert!(player.calls().is_empty(), "not before the text");
        dialogs.input(&key(Key::Enter));
        assert_eq!(shown(&dialogs), None);
        assert_eq!(player.calls(), [Call::Start("Brief.mov".to_owned())]);
        assert!(dialogs.is_open(), "while the movie plays");
        dialogs.tick(TICK);
        assert!(dialogs.is_open());
        dialogs.tick(TICK);
        assert!(!dialogs.is_open(), "closed on the tick the movie ends");
    }

    #[test]
    fn with_movies_off_no_movie_plays() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let movies = Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, false);
        let mut dialogs = dialogs().with_movies(movies.clone());
        dialogs.show_texts(vec![filmed("Before.", 0), filmed("After.", 0x0001)]);
        assert_eq!(shown(&dialogs).as_deref(), Some("Before."));
        dialogs.input(&key(Key::Enter));
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open());
        assert!(player.calls().is_empty());
        movies.set_on(true);
        dialogs.show_texts(vec![filmed("Before.", 0)]);
        assert_eq!(player.calls().len(), 1, "on again, as the preference says");
    }

    #[test]
    fn without_a_player_no_movie_plays() {
        let mut dialogs = dialogs();
        dialogs.show_texts(vec![filmed("Before.", 0)]);
        assert_eq!(shown(&dialogs).as_deref(), Some("Before."));
    }

    #[test]
    fn a_movie_that_cannot_start_shows_the_text_at_once() {
        let player = MockMoviePlayer::refusing();
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("Briefed.", 0)]);
        assert_eq!(shown(&dialogs).as_deref(), Some("Briefed."));
        dialogs.input(&key(Key::Enter));
        dialogs.show_texts(vec![filmed("After.", 0x0001)]);
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open(), "the after-movie ends as it begins");
        assert_eq!(player.calls().len(), 2);
    }

    #[test]
    fn an_empty_text_shows_no_dialog_and_plays_its_before_movie_only() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("", 0x0001), brief(129, "")]);
        assert!(
            !dialogs.is_open(),
            "no dialog, no after-movie (@0x5a7b0-0x5a7c7)"
        );
        assert!(player.calls().is_empty());
        dialogs.show_texts(vec![filmed("", 0)]);
        assert_eq!(player.calls(), [Call::Start("Brief.mov".to_owned())]);
        dialogs.tick(TICK);
        assert!(!dialogs.is_open(), "the before-movie alone");
    }

    #[test]
    fn a_desc_shown_over_flight_plays_its_movie() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.show_descs(vec![DescShown {
            movie: film(0),
            ..shown_desc("Sabotaged.", None)
        }]);
        assert_eq!(shown(&dialogs), None);
        dialogs.tick(TICK);
        assert_eq!(shown(&dialogs).as_deref(), Some("Sabotaged."));
    }

    #[test]
    fn a_text_that_cannot_be_laid_out_still_plays_its_movies() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = MissionDialogs::new(
            texts(Err("no DLOG 3003".to_owned())),
            Ok(info_template()),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        )
        .with_movies(Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, true));
        dialogs.show_texts(vec![filmed("Lost.", 0x0001)]);
        assert_eq!(player.calls(), [Call::Start("Brief.mov".to_owned())]);
    }

    #[test]
    fn an_offer_movie_queued_among_the_texts_plays_in_its_place_whatever_its_flag() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        let offer =
            |flags| MissionShown::Movie(DescMovie::from_desc("Offer.mov", flags).expect("named"));
        dialogs.show_texts(vec![
            MissionShown::Text(brief(128, "First.")),
            offer(0x0001),
            MissionShown::Text(brief(129, "Second.")),
        ]);
        assert!(player.calls().is_empty(), "behind the first text");
        dialogs.input(&key(Key::Enter));
        assert_eq!(player.calls(), [Call::Start("Offer.mov".to_owned())]);
        assert_eq!(shown(&dialogs), None, "while it plays");
        dialogs.tick(TICK);
        assert_eq!(shown(&dialogs).as_deref(), Some("Second."), "then the next");
        dialogs.input(&key(Key::Enter));
        assert!(!dialogs.is_open());
        dialogs.show_texts([offer(0)]);
        assert!(dialogs.is_open(), "alone, at once");
    }

    #[test]
    fn keys_pressed_during_a_movie_that_ends_on_its_own_reach_the_text_after_it() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("First.", 0), brief(129, "Second.")]);
        dialogs.input(&key(Key::Escape));
        dialogs.tick(TICK);
        assert_eq!(
            shown(&dialogs).as_deref(),
            Some("Second."),
            "the Escape queued under the movie closed the first text"
        );
    }

    #[test]
    fn keys_pressed_under_the_last_movie_are_left_for_what_is_beneath() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("Gone.", 0x0001)]);
        dialogs.input(&key(Key::Enter));
        dialogs.input(&key(Key::Escape));
        assert!(dialogs.take_unclaimed().is_empty(), "not while it plays");
        dialogs.tick(TICK);
        assert!(!dialogs.is_open());
        assert_eq!(dialogs.take_unclaimed(), [key(Key::Escape)]);
        assert!(dialogs.take_unclaimed().is_empty(), "once");
    }

    #[test]
    fn a_skipped_movie_flushes_what_was_pressed() {
        let player = MockMoviePlayer::playing((160, 120), 10);
        let mut dialogs = playing(&player);
        dialogs.show_texts(vec![filmed("First.", 0)]);
        dialogs.input(&key(Key::Escape));
        dialogs.input(&key(Key::Space));
        assert_eq!(shown(&dialogs).as_deref(), Some("First."));
    }

    #[test]
    fn a_movie_plays_over_the_info_screen() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        dialogs.open_info(info());
        dialogs.show_texts(vec![filmed("Over.", 0)]);
        let mut list = DrawList::new();
        dialogs.draw(&mut list);
        let commands: Vec<_> = list.iter().collect();
        let run = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Run"))
            .expect("the info screen");
        let frame = commands
            .iter()
            .position(|command| matches!(command, DrawCommand::MovieFrame { .. }))
            .expect("the frame");
        assert!(run < frame);
        dialogs.cancel_pointer();
        dialogs.tick(TICK);
        assert_eq!(shown(&dialogs).as_deref(), Some("Over."));
    }

    #[test]
    fn a_cinematic_movie_hides_the_pointer() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut dialogs = playing(&player);
        assert!(!dialogs.hides_pointer());
        dialogs.show_texts(vec![filmed("Over.", 0x0004)]);
        assert!(!dialogs.hides_pointer(), "not during the fade out");
        dialogs.tick(MOVIE_FADE_FOR);
        assert!(dialogs.hides_pointer());
    }

    /// A mission text is `_ShowTextDialog`'s, whose `_TextDialogFilter`
    /// @0x51f77 maps no Tab or Space.
    #[test]
    fn a_mission_text_takes_tab_then_space_only_with_the_focus_ring() {
        let mut ringed = dialogs();
        ringed.show_texts(vec![brief(128, "First.")]);
        ringed.input(&key(Key::Tab));
        ringed.input(&key(Key::Space));
        assert!(!ringed.is_open(), "by default");
        let mut off = dialogs().with_focus_ring(FocusRing::Off);
        off.show_texts(vec![brief(128, "First.")]);
        off.input(&key(Key::Tab));
        off.input(&key(Key::Space));
        assert_eq!(shown(&off).as_deref(), Some("First."));
        assert_eq!(off.text().and_then(|text| text.dialog().focus()), None);
    }
}
