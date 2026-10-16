//! The offer dialog (`DLOG`/`DITL` 1016, `_OfferOneMission` @0xa21ac in
//! the `EV Nova` executable): a mission offered outside the mission
//! computer, such as at the bar, its text and its buttons.
//!
//! The stock dialog is 441 x 317, centred, over the frame of `PICT`s
//! 8521-8523: the offer text in item 3, and below it the accept (item 1)
//! and refuse (item 2) buttons, labelled as the session says
//! ([`OfferPrompt`]); a mission that cannot be refused shows the one
//! button of item 6 in their place, labelled to accept. The stock items
//! are all disabled, so the dialog enables its buttons itself. Items 4, 5
//! and 7 (the map, player info and mission info) lie outside it, and the
//! arrows of items 9 and 10 are not drawn: Up and Down scroll the text.
//!
//! The original shrinks `DLOG` 1016 to its text's height, at least 48,
//! moving the buttons up (@0xa2548-0xa2816), and widens the buttons to the
//! wider label + 40, at least 100 (@0xa2a29-0xa2b46); this dialog keeps
//! the template's height and widths.
//!
//! An offer whose `dësc` `Graphic` is above 127 shows in the picture
//! variant, `DLOG` 1020 ([`OfferTemplates::layout`], @0xa2344-0xa2373),
//! chosen on the ID alone: 651 x 244, centred, the text in item 3, the
//! buttons in items 1, 2 and 6 as in 1016, over the single frame `PICT`
//! 8528, with the graphic stretched into item 8. The original never
//! shrinks 1020 (`jg` past the shrink @0xa255b-0xa2564), so its template
//! height is its own.
//!
//! An offer whose `dësc` names a movie ([`OfferPrompt::movie`]) plays it
//! while movies are on ([`Movies`]): before the dialog
//! (`_OfferOneMission` @0xa2273-0xa22ab), which is neither drawn nor
//! given input until the movie ends ([`OfferDialog::with_movies`]). One
//! with `Flags` 0x0001 set plays after the answer has been acted on, its
//! `BriefText` or `RefuseText` shown (@0xa3580-0xa35b8, after the accept
//! @0xa2e06 and the refuse text @0xa2f04): the dialog plays none of it and
//! takes the choice at once, and the session queues that movie behind the
//! texts the answer shows
//! ([`MissionShown::Movie`](nova_sim::MissionShown::Movie)).
//!
//! Keys (`_SingleMissionFilter` @0x98d9a): Return and Enter accept; the
//! accept label's first letter accepts and, when the mission can be
//! refused, the refuse label's refuses, unless both start with the same
//! letter. Escape does nothing. The choice is taken once
//! ([`OfferDialog::take_choice`]), and the dialog is then answered. P
//! ([`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY), a press) asks for the player info screen
//! ([`OfferDialog::take_player_info_request`], @0xa3215), before the
//! labels' letters are read; the dialog stays open under it. I
//! ([`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), a
//! press) asks for the mission info screen
//! ([`OfferDialog::take_info_request`], @0x98e0c-0x98e2a, opened by
//! `_OfferOneMission` @0xa3344), also before the letters; the dialog
//! stays open under it. The original then reloads the offer's text
//! (`_LoadDescription` @0xa335c), which redraws what the info screen
//! covered and changes nothing shown.
//!
//! A venue that offers missions (the bar, the spaceport, the trade
//! center, the shipyard and the outfitter) holds its offers in a
//! `VenueOffers`: when it next asks for an offer, the offer dialog open
//! over it, and the choice made, until it is taken.
//!
//! The venue's offer timer runs as [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock)
//! reads it. By the engine, as the original's one deadline
//! (`_missionOfferTime`) does: the next offer is due its gap after the
//! last ask (`_BarFilter` @0x47be0-0x47c11, `_TradeFilter`
//! @0x4e550-0x4e581, `_OutfitFilter` @0x589ce-0x58a01, `_ShipyardFilter`
//! @0x60de5-0x60e21 set it as they ask; nothing resets it after the
//! answer), on wall time through the offer and through whatever is open
//! over the venue, whose own modal loop never calls the venue's filter
//! (`_MyModalDialog` @0x59fee). An offer that comes due while something
//! covers the venue is asked for on the first tick nothing does. The
//! original asks on the first event after the dialog over it closes,
//! after redrawing the venue once; both come before any offer is drawn,
//! so the player sees no difference. By the other reading, the gap runs
//! from the answer, and the wait stops while anything covers the venue.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::{MissionId, OfferPrompt, RuleSource};

use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::{MovieStage, Movies};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::desc::{BODY_COLOR, BODY_SIZE, shown_graphic};
use crate::ui::dialog::{Dialog, DialogEvent, DialogFrame, DialogTemplate, Role};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::picture_variant::VariantTemplates;
use crate::ui::player_info::is_player_info_press;

/// The offer dialog's `DLOG` (and `DITL`) ID.
pub const OFFER_DIALOG: i16 = 1016;

/// The accept button's item.
pub const ACCEPT_ITEM: usize = 1;
/// The refuse button's item.
pub const REFUSE_ITEM: usize = 2;
/// The text's item.
pub const TEXT_ITEM: usize = 3;
/// The one button's item, for a mission that cannot be refused.
pub const SINGLE_ITEM: usize = 6;

/// The picture variant's `DLOG` (and `DITL`) ID ("Single mission + pict",
/// `GetNewDialog` 0x3fc @0xa235a).
pub const PICTURE_OFFER_DIALOG: i16 = 1020;

/// The picture variant's picture item (`GetDialogItem` 8 @0x98c03).
pub const OFFER_PICTURE_ITEM: usize = 8;

/// The picture variant's frame, one picture over the whole dialog: `PICT`
/// 8528, "Single mission + pict" (0x2150 @0xa239d).
pub const PICTURE_FRAME: ImageKey = ImageKey::picture(8528);

/// Both offer dialog templates, or why each is missing: `DLOG` 1016's and
/// `DLOG` 1020's.
#[derive(Clone, Debug)]
pub struct OfferTemplates {
    /// The offer dialog ([`OFFER_DIALOG`]).
    pub plain: Result<DialogTemplate, String>,
    /// The offer dialog with a picture ([`PICTURE_OFFER_DIALOG`]).
    pub picture: Result<DialogTemplate, String>,
}

impl VariantTemplates for OfferTemplates {
    fn plain_template(&self) -> &Result<DialogTemplate, String> {
        &self.plain
    }

    fn picture_template(&self) -> &Result<DialogTemplate, String> {
        &self.picture
    }
}

impl OfferTemplates {
    /// The template of the variant an offer whose `dësc` `Graphic` is
    /// `graphic` shows in, or why it is missing: 1020 above
    /// [`LAST_UNSHOWN_GRAPHIC`](crate::ui::desc::LAST_UNSHOWN_GRAPHIC),
    /// on the ID alone (@0xa2344), else 1016.
    ///
    /// # Errors
    ///
    /// Why that variant's template is missing.
    pub fn layout(&self, graphic: Option<i16>) -> Result<DialogTemplate, String> {
        self.pick(graphic).1.clone()
    }
}

/// The game's mission offer frame (`PICT`s 8521 to 8523, 441 wide), which
/// the offer dialog is drawn over.
pub const FRAME: DialogFrame = DialogFrame {
    top: ImageKey::picture(8521),
    middle: ImageKey::picture(8522),
    bottom: ImageKey::picture(8523),
    top_height: 9.0,
    bottom_height: 40.0,
};

/// What the player chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferChoice {
    /// Accept the mission.
    Accept,
    /// Refuse it.
    Refuse,
}

/// The offer dialog over the screen that offers it.
#[derive(Clone, Debug)]
pub struct OfferDialog {
    /// The dialog, or why it cannot be shown.
    laid: Result<Dialog, String>,
    prompt: OfferPrompt,
    /// The choice made, until it is taken.
    choice: Option<OfferChoice>,
    answered: bool,
    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen.
    player_info: bool,
    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen.
    info_requested: bool,
    /// The picture variant's graphic, drawn in item 8 over
    /// [`PICTURE_FRAME`]; none for the plain dialog over [`FRAME`].
    picture: Option<i16>,
    sounds: Vec<Sound>,
    /// The movie played before the dialog, until it ends.
    before: Option<MovieStage>,
}

impl OfferDialog {
    /// `prompt`, laid out by `layout`: the template of the variant its
    /// graphic picks ([`OfferTemplates::layout`]) and the metrics its text
    /// is measured by, or why there are none; its buttons drawn in
    /// `style`. A graphic above 127 draws the picture variant: the frame
    /// [`PICTURE_FRAME`] and the graphic in item 8.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        prompt: OfferPrompt,
        style: ButtonStyle,
    ) -> Self {
        let picture = shown_graphic(prompt.graphic);
        let laid = layout.map(|(mut template, metrics)| {
            let single = prompt.labels.refuse.is_none();
            let buttons: &[usize] = if single {
                &[SINGLE_ITEM]
            } else {
                &[ACCEPT_ITEM, REFUSE_ITEM]
            };
            for &item in buttons {
                if let Some(found) = template.items.get_mut(item - 1) {
                    found.enabled = true;
                }
            }
            let accept = Role::Button(prompt.labels.accept.clone());
            let mut roles = vec![(
                TEXT_ITEM,
                Role::ScrollText {
                    text: prompt.text.clone(),
                    font: Font::Geneva,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                },
            )];
            match &prompt.labels.refuse {
                Some(refuse) => roles.extend([
                    (ACCEPT_ITEM, accept),
                    (REFUSE_ITEM, Role::Button(refuse.clone())),
                    (SINGLE_ITEM, Role::Hidden),
                ]),
                None => roles.extend([
                    (SINGLE_ITEM, accept),
                    (ACCEPT_ITEM, Role::Hidden),
                    (REFUSE_ITEM, Role::Hidden),
                ]),
            }
            let dialog = Dialog::new(&template, &roles, metrics)
                .with_buttons(ButtonSkin::NOVA, style)
                .with_default(None)
                .with_cancel(None);
            if picture.is_some() {
                dialog
            } else {
                dialog.with_frame(FRAME)
            }
        });
        Self {
            laid,
            prompt,
            choice: None,
            answered: false,
            player_info: false,
            info_requested: false,
            picture,
            sounds: Vec::new(),
            before: None,
        }
    }

    /// This dialog with its offer's movie, as `movies` plays it: one that
    /// plays before the dialog begins at once, and the dialog waits for
    /// it. One that plays after the answer is the session's to queue. With
    /// movies off, none.
    #[must_use]
    pub fn with_movies(self, movies: &Movies) -> Self {
        let Some(movie) = self.prompt.movie.clone().filter(|movie| !movie.after) else {
            return self;
        };
        let mut dialog = self;
        if let Some(mut stage) = movies.stage(&movie) {
            stage.begin();
            dialog.before = Some(stage);
            dialog.settle_movie();
        }
        dialog
    }

    /// Whether its movie plays before the dialog shows.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.before.is_some()
    }

    /// Drops the movie before the dialog once it has ended, handing what
    /// was pressed under it to the dialog.
    fn settle_movie(&mut self) {
        if let Some(stage) = &mut self.before
            && stage.ended()
        {
            let replay = stage.take_replay();
            self.before = None;
            for input in &replay {
                self.dialog_input(input);
            }
        }
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen since
    /// this was last taken, once.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.player_info)
    }

    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen since this was last taken, once.
    pub fn take_info_request(&mut self) -> bool {
        std::mem::take(&mut self.info_requested)
    }

    /// The dialog, if it could be laid out.
    #[must_use]
    pub fn dialog(&self) -> Option<&Dialog> {
        self.laid.as_ref().ok()
    }

    /// Why the offer cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// The offer shown.
    #[must_use]
    pub fn prompt(&self) -> &OfferPrompt {
        &self.prompt
    }

    /// Whether a choice has been made.
    #[must_use]
    pub fn answered(&self) -> bool {
        self.answered
    }

    /// The choice made since this was last taken, once.
    pub fn take_choice(&mut self) -> Option<OfferChoice> {
        self.choice.take()
    }

    /// Chooses `choice`, once.
    fn choose(&mut self, choice: OfferChoice) {
        if !self.answered {
            self.answered = true;
            self.choice = Some(choice);
        }
    }

    /// What key `key` chooses: Return accepts, and each label's first
    /// letter, either case, chooses it, unless both start with the same
    /// letter.
    fn key_choice(&self, key: Key) -> Option<OfferChoice> {
        let first = |label: &str| label.chars().next().map(|c| c.to_ascii_lowercase());
        let accept = first(&self.prompt.labels.accept);
        let refuse = self.prompt.labels.refuse.as_deref().and_then(first);
        let letter = match key {
            Key::Enter => return Some(OfferChoice::Accept),
            Key::Char(c) if accept != refuse => Some(c.to_ascii_lowercase()),
            _ => return None,
        };
        if letter == accept {
            Some(OfferChoice::Accept)
        } else if letter == refuse {
            Some(OfferChoice::Refuse)
        } else {
            None
        }
    }
}

impl OfferDialog {
    /// The dialog's input: Return, Enter and the labels' letters choose;
    /// everything else goes to the dialog. Once answered it takes nothing.
    fn dialog_input(&mut self, input: &Input) {
        if self.answered {
            return;
        }
        if is_player_info_press(input) {
            self.player_info = true;
            return;
        }
        if is_mission_info_press(input) {
            self.info_requested = true;
            return;
        }
        if let Input::Key {
            key,
            pressed: true,
            repeat: false,
        } = *input
            && let Some(choice) = self.key_choice(key)
        {
            self.choose(choice);
            return;
        }
        let Ok(dialog) = &mut self.laid else {
            return;
        };
        let event = dialog.input(input);
        self.sounds.extend(dialog.take_sound().map(Sound::Ui));
        match event {
            Some(DialogEvent::Item(ACCEPT_ITEM | SINGLE_ITEM)) => self.choose(OfferChoice::Accept),
            Some(DialogEvent::Item(REFUSE_ITEM)) => self.choose(OfferChoice::Refuse),
            _ => {}
        }
    }
}

impl Screen for OfferDialog {
    /// The movie before the dialog takes every input until it ends.
    /// Then Return, Enter and the labels' letters choose; everything else
    /// goes to the dialog. Once answered it takes nothing. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(stage) = &mut self.before {
            stage.input(input);
            self.settle_movie();
        } else {
            self.dialog_input(input);
        }
        ScreenAction::None
    }

    /// The movie before the dialog moves on.
    fn tick(&mut self, dt: Duration) {
        if let Some(stage) = &mut self.before {
            stage.tick(dt);
            self.settle_movie();
        }
    }

    /// While a cinematic movie hides the cursor.
    fn hides_pointer(&self) -> bool {
        self.before.as_ref().is_some_and(MovieStage::hides_cursor)
    }

    /// The plain dialog draws its frame under its items. The picture
    /// variant draws [`PICTURE_FRAME`] stretched over the dialog, as
    /// `_SingleMissionDialogUpdate` fills the port with it, then the
    /// items, then the graphic stretched into item 8 (@0x98bdd-0x98c53);
    /// a graphic whose `PICT` is missing leaves item 8 empty, as
    /// `GetPicture` finds nothing to draw.
    fn draw(&self, list: &mut DrawList) {
        if let Some(stage) = &self.before {
            stage.draw(list);
            return;
        }
        match &self.laid {
            Ok(dialog) => {
                let Some(graphic) = self.picture else {
                    dialog.draw(list);
                    return;
                };
                dialog.draw_stretched_over(list, PICTURE_FRAME);
                dialog.draw(list);
                dialog.draw_picture_in(list, OFFER_PICTURE_ITEM, graphic);
            }
            Err(reason) => {
                list.text(
                    format!("Cannot show the mission offer: {reason}. Press Return to accept it."),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    fn cancel_pointer(&mut self) {
        if self.before.is_some() {
            return;
        }
        if let Ok(dialog) = &mut self.laid {
            dialog.cancel_pointer();
        }
    }

    /// The buttons' sounds, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

/// A venue's mission offers: the offer dialog's templates (or why there
/// are none), the wait before it next asks, and the offer open over it.
#[derive(Clone, Debug)]
pub(crate) struct VenueOffers {
    templates: OfferTemplates,
    /// How long until the venue asks for an offer: none while it waits to
    /// be rearmed.
    wait: Option<Duration>,
    /// Whether the venue has asked since this was last taken.
    requested: bool,
    /// The offer dialog, while it is open.
    dialog: Option<OfferDialog>,
    /// The offer answered and its choice, until it is taken.
    choice: Option<(MissionId, OfferChoice)>,
    /// The movies the offers play, and whether they play.
    movies: Movies,
    /// The offer dialog's sounds not yet handed on.
    sounds: Vec<Sound>,
    /// How the timer runs ([`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock)).
    clock: RuleSource,
    /// The time since the venue last asked (or, before it first asks,
    /// since it opened), which the engine's gap runs from.
    since_ask: Duration,
    /// Whether the timer restarted since this was last taken, and wants
    /// a new gap.
    restarted: bool,
}

impl VenueOffers {
    /// Offers that ask for the first after `first`, as the bar does
    /// (`_DoBarDialog` @0x48f51-0x48f5f).
    pub(crate) fn waiting(templates: OfferTemplates, first: Duration) -> Self {
        Self {
            templates,
            wait: Some(first),
            requested: false,
            dialog: None,
            choice: None,
            movies: Movies::none(),
            sounds: Vec::new(),
            clock: RuleSource::Engine,
            since_ask: Duration::ZERO,
            restarted: false,
        }
    }

    /// These offers with their timer running as `clock` reads
    /// [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock).
    #[must_use]
    pub(crate) fn with_clock(self, clock: RuleSource) -> Self {
        Self { clock, ..self }
    }

    /// These offers playing their movies as `movies` says.
    pub(crate) fn set_movies(&mut self, movies: Movies) {
        self.movies = movies;
    }

    /// Offers already asking for the first, as the spaceport and the shops
    /// ask as they open (`_DoPortDialog` @0x5fc37, `_DoTradeDialog`
    /// @0x5df7f, `_DoShipyardDialog` @0x5e8eb, `_DoOutfitDialog`
    /// @0x5bd92).
    pub(crate) fn asking(templates: OfferTemplates) -> Self {
        Self {
            requested: true,
            wait: None,
            ..Self::waiting(templates, Duration::ZERO)
        }
    }

    /// Whether the venue has asked for an offer since this was last taken.
    pub(crate) fn take_request(&mut self) -> bool {
        std::mem::take(&mut self.requested)
    }

    /// Opens the offer dialog on `prompt`, in the variant its graphic
    /// picks, its text measured by `metrics` and its buttons drawn in
    /// `style`.
    pub(crate) fn open(
        &mut self,
        prompt: OfferPrompt,
        metrics: Rc<dyn TextMetrics>,
        style: ButtonStyle,
    ) {
        let layout = self
            .templates
            .layout(prompt.graphic)
            .map(|template| (template, metrics));
        self.dialog = Some(OfferDialog::new(layout, prompt, style).with_movies(&self.movies));
    }

    /// The offer dialog, while it is open.
    pub(crate) fn dialog(&self) -> Option<&OfferDialog> {
        self.dialog.as_ref()
    }

    /// Whether the offer dialog open asked for the player info screen
    /// since this was last taken, once.
    pub(crate) fn take_player_info_request(&mut self) -> bool {
        self.dialog
            .as_mut()
            .is_some_and(OfferDialog::take_player_info_request)
    }

    /// Whether the offer dialog open asked for the mission info screen
    /// since this was last taken, once.
    pub(crate) fn take_info_request(&mut self) -> bool {
        self.dialog
            .as_mut()
            .is_some_and(OfferDialog::take_info_request)
    }

    /// The offer answered since this was last taken, and the choice made,
    /// once.
    pub(crate) fn take_choice(&mut self) -> Option<(MissionId, OfferChoice)> {
        self.choice.take()
    }

    /// Asks for the next offer after `gap`: by the engine, `gap` after the
    /// last ask (or restart), so a gap already past asks on the next tick
    /// nothing covers the venue; by the other reading, `gap` from now.
    pub(crate) fn rearm(&mut self, gap: Duration) {
        self.wait = Some(match self.clock {
            RuleSource::Engine => gap.saturating_sub(self.since_ask),
            RuleSource::Bible => gap,
        });
    }

    /// By the engine, restarts the timer, as the bar's hire screen does
    /// opening (`_DoShipyardDialog` sets the one deadline @0x5e74c-0x5e775
    /// in hire mode too): no ask is pending, and the next is due a new gap
    /// after now, given by [`VenueOffers::rearm`] once
    /// [`VenueOffers::take_restart`] asks for it. By the other reading it
    /// does nothing.
    pub(crate) fn restart(&mut self) {
        if self.clock == RuleSource::Engine {
            self.wait = None;
            self.requested = false;
            self.since_ask = Duration::ZERO;
            self.restarted = true;
        }
    }

    /// Whether the timer restarted since this was last taken: whoever
    /// holds the session rearms it with a new gap.
    pub(crate) fn take_restart(&mut self) -> bool {
        std::mem::take(&mut self.restarted)
    }

    /// Gives `input` to the offer dialog, if it is open, adding its sounds,
    /// and any it made as it ticked, to `sounds`; answered, it closes.
    /// Whether it took the input.
    pub(crate) fn input(&mut self, input: &Input, sounds: &mut Vec<Sound>) -> bool {
        sounds.append(&mut self.sounds);
        let Some(dialog) = &mut self.dialog else {
            return false;
        };
        dialog.input(input);
        self.settle_dialog();
        sounds.append(&mut self.sounds);
        true
    }

    /// Keeps the offer dialog's sounds; answered, it closes, its choice
    /// kept until taken.
    fn settle_dialog(&mut self) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        self.sounds.extend(dialog.take_sounds());
        if let Some(choice) = dialog.take_choice() {
            self.choice = Some((dialog.prompt().mission, choice));
            self.dialog = None;
        }
    }

    /// The wait for the next offer runs down by `dt`; once it is out, and
    /// nothing is `covered` over the venue nor is the offer open, the venue
    /// asks. By the engine it runs down whatever covers the venue; by the
    /// other reading only while nothing does.
    /// The offer dialog open ticks too, its movie moving on
    /// ([`VenueOffers::tick_dialog`]).
    pub(crate) fn tick(&mut self, dt: Duration, covered: bool) {
        self.tick_dialog(dt);
        let covered = covered || self.dialog.is_some();
        if covered && self.clock == RuleSource::Bible {
            return;
        }
        self.since_ask = self.since_ask.saturating_add(dt);
        if let Some(wait) = &mut self.wait {
            *wait = wait.saturating_sub(dt);
            if wait.is_zero() && !covered {
                self.wait = None;
                self.requested = true;
                self.since_ask = Duration::ZERO;
            }
        }
    }

    /// Ticks the offer dialog, if it is open: its movie moves on, and the
    /// keys pressed under a movie that ends can answer it, on that tick.
    pub(crate) fn tick_dialog(&mut self, dt: Duration) {
        if let Some(dialog) = &mut self.dialog {
            dialog.tick(dt);
            self.settle_dialog();
        }
    }

    /// Draws the offer dialog, if it is open.
    pub(crate) fn draw(&self, list: &mut DrawList) {
        if let Some(dialog) = &self.dialog {
            dialog.draw(list);
        }
    }

    /// Cancels the offer dialog's pointer, if it is open: whether it was.
    pub(crate) fn cancel_pointer(&mut self) -> bool {
        self.dialog
            .as_mut()
            .map(OfferDialog::cancel_pointer)
            .is_some()
    }
}

/// An offer dialog's template and prompt, for the venues' tests.
#[cfg(test)]
pub(crate) mod fixture {
    use nova_sim::mission::OfferLabels;
    use nova_sim::{MissionId, OfferPrompt};

    use crate::geometry::{Bounds, Point};
    use crate::ui::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};

    /// The offer dialog's shape, smaller: 300 x 200, fixed at (0, 0), with
    /// accept (1), refuse (2), the text (3) and the one button (6).
    pub(crate) fn template() -> DialogTemplate {
        let rect = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
        let mut items: Vec<ItemTemplate> = (0..6)
            .map(|_| ItemTemplate {
                bounds: rect(0.0, 300.0, 1.0, 1.0),
                enabled: false,
                kind: ItemSpec::User,
            })
            .collect();
        items[0].bounds = rect(200.0, 170.0, 90.0, 25.0);
        items[1].bounds = rect(100.0, 170.0, 90.0, 25.0);
        items[2].bounds = rect(10.0, 10.0, 280.0, 150.0);
        items[5].bounds = rect(150.0, 170.0, 90.0, 25.0);
        DialogTemplate {
            bounds: rect(0.0, 0.0, 300.0, 200.0),
            placement: Placement::Fixed,
            items,
        }
    }

    /// [`template`] as the offer dialog, with no picture variant.
    pub(crate) fn templates() -> super::OfferTemplates {
        super::OfferTemplates {
            plain: Ok(template()),
            picture: Err("no DLOG 1020 here".to_owned()),
        }
    }

    /// Mission 141's offer, to Mars, "I'm in" or "No".
    pub(crate) fn prompt() -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this to Mars.".to_owned(),
            graphic: None,
            labels: OfferLabels {
                accept: "I'm in".to_owned(),
                refuse: Some("No".to_owned()),
            },
            movie: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::mission::OfferLabels;
    use nova_sim::{DescMovie, RuleSource};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::movie::fixture::{Call, MockMoviePlayer};
    use crate::movie::{MOVIE_FADE_FOR, MoviePlayer};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DLOG` 1016, 441 x 317 and centred, with its ten disabled
    /// user items as `DITL` 1016 has them, 4, 5 and 7 parked outside it.
    fn template() -> DialogTemplate {
        let items = [
            (225.0, 285.0, 99.0, 25.0),
            (117.0, 285.0, 99.0, 25.0),
            (12.0, 9.0, 415.0, 267.0),
            (150.0, 323.0, 32.0, 32.0),
            (197.0, 326.0, 32.0, 32.0),
            (173.0, 285.0, 99.0, 25.0),
            (172.0, 337.0, 32.0, 32.0),
            (272.0, 345.0, 23.0, 23.0),
            (340.0, 287.0, 23.0, 23.0),
            (373.0, 287.0, 23.0, 23.0),
        ];
        DialogTemplate {
            bounds: rect(40.0, 40.0, 441.0, 317.0),
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled: false,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where the dialog goes: (1024 - 441) / 2 and (768 - 317) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(291.0, 225.0);

    fn prompt(accept: &str, refuse: Option<&str>) -> OfferPrompt {
        OfferPrompt {
            mission: MissionId(141),
            text: "Ferry this to Mars.".to_owned(),
            graphic: None,
            labels: OfferLabels {
                accept: accept.to_owned(),
                refuse: refuse.map(str::to_owned),
            },
            movie: None,
        }
    }

    fn dialog_of(prompt: OfferPrompt) -> OfferDialog {
        OfferDialog::new(
            Ok((template(), Rc::new(MonoMetrics))),
            prompt,
            ButtonStyle::STOCK,
        )
    }

    fn offer() -> OfferDialog {
        dialog_of(prompt("I'm in", Some("No thanks")))
    }

    fn drawn(dialog: &OfferDialog) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn texts(dialog: &OfferDialog) -> Vec<String> {
        drawn(dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn click_at(dialog: &mut OfferDialog, at: Point) {
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    fn click_item(dialog: &mut OfferDialog, number: usize) {
        let at = dialog
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(OFFER_DIALOG, 1016);
        assert_eq!(
            [ACCEPT_ITEM, REFUSE_ITEM, TEXT_ITEM, SINGLE_ITEM],
            [1, 2, 3, 6]
        );
        assert_eq!(
            [FRAME.top, FRAME.middle, FRAME.bottom],
            [8521, 8522, 8523].map(ImageKey::picture)
        );
    }

    #[test]
    fn it_shows_the_text_and_both_labels_over_its_frame() {
        let dialog = offer();
        assert_eq!(dialog.problem(), None);
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds().min, ORIGIN);
        assert_eq!(
            laid.text(TEXT_ITEM).map(|text| text.lines().join(" ")),
            Some("Ferry this to Mars.".to_owned())
        );
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[0],
            DrawCommand::StretchedPicture { image, top_left, .. }
                if image == FRAME.top && top_left == ORIGIN
        ));
        let shown = texts(&dialog);
        for label in ["I'm in", "No thanks", "Ferry this to Mars."] {
            assert!(shown.contains(&label.to_owned()), "{label}: {shown:?}");
        }
        assert!(laid.item_shown(ACCEPT_ITEM) && laid.item_shown(REFUSE_ITEM));
        assert!(!laid.item_shown(SINGLE_ITEM));
        assert!(!dialog.answered());
    }

    #[test]
    fn an_offer_that_cannot_be_refused_shows_one_button() {
        let mut dialog = dialog_of(prompt("Okay", None));
        let laid = dialog.dialog().expect("laid out");
        assert!(laid.item_shown(SINGLE_ITEM));
        assert!(!laid.item_shown(ACCEPT_ITEM) && !laid.item_shown(REFUSE_ITEM));
        assert_eq!(
            texts(&dialog).iter().filter(|text| *text == "Okay").count(),
            1
        );
        click_item(&mut dialog, SINGLE_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn the_buttons_choose_and_the_choice_is_taken_once() {
        let mut dialog = offer();
        assert_eq!(dialog.take_choice(), None);
        click_item(&mut dialog, REFUSE_ITEM);
        assert!(dialog.answered());
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse));
        assert_eq!(dialog.take_choice(), None, "once");
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_choice(), None, "answered already");
        let mut dialog = offer();
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn return_and_enter_accept_and_escape_does_nothing() {
        let mut dialog = offer();
        dialog.input(&key(Key::Escape));
        assert!(!dialog.answered());
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: true,
        });
        assert_eq!(dialog.take_choice(), None, "a repeat does nothing");
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn each_labels_first_letter_chooses_it() {
        let mut dialog = offer();
        dialog.input(&key(Key::Char('n')));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse));
        let mut dialog = offer();
        dialog.input(&key(Key::Char('I')));
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Accept),
            "either case"
        );
        let mut dialog = offer();
        dialog.input(&key(Key::Char('x')));
        assert_eq!(dialog.take_choice(), None);
        let mut cannot = dialog_of(prompt("Okay", None));
        cannot.input(&key(Key::Char('n')));
        assert_eq!(cannot.take_choice(), None, "no refuse label");
        cannot.input(&key(Key::Char('o')));
        assert_eq!(cannot.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn labels_starting_with_the_same_letter_take_no_letter() {
        let mut dialog = dialog_of(prompt("Sure", Some("Scram")));
        dialog.input(&key(Key::Char('s')));
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn up_and_down_scroll_the_text() {
        let mut dialog = dialog_of(OfferPrompt {
            text: "line\r".repeat(40),
            ..prompt("Yes", Some("No"))
        });
        dialog.input(&key(Key::Down));
        assert_eq!(
            dialog
                .dialog()
                .and_then(|laid| laid.text(TEXT_ITEM))
                .map(crate::ui::ScrollText::first),
            Some(1)
        );
        assert!(!dialog.answered());
    }

    #[test]
    fn without_its_dialog_it_says_why_and_return_accepts() {
        let mut dialog = OfferDialog::new(
            Err("no DLOG 1016".to_owned()),
            prompt("Yes", Some("No")),
            ButtonStyle::STOCK,
        );
        assert_eq!(dialog.problem(), Some("no DLOG 1016"));
        assert!(drawn(&dialog).iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, .. } if text.contains("no DLOG 1016") && *color == Color::ERROR
        )));
        dialog.input(&key(Key::Escape));
        assert_eq!(dialog.take_choice(), None);
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
        assert_eq!(dialog.prompt().mission, MissionId(141));
    }

    const DOWN: Sound = Sound::Ui(crate::sound::UiSound::ButtonDown);
    const UP: Sound = Sound::Ui(crate::sound::UiSound::ButtonUp);

    #[test]
    fn its_buttons_sound_and_a_click_abandoned_does_nothing() {
        let mut dialog = offer();
        let at = dialog
            .dialog()
            .and_then(|laid| laid.item_bounds(ACCEPT_ITEM))
            .expect("accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        dialog.input(&button(true));
        dialog.cancel_pointer();
        dialog.input(&button(false));
        assert_eq!(dialog.take_choice(), None, "abandoned");
        assert_eq!(dialog.take_sounds(), [DOWN]);
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_sounds(), [DOWN, UP]);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    // A venue's offers.

    /// `plain` for the offer dialog, with no picture variant.
    fn plain_only(plain: Result<DialogTemplate, String>) -> OfferTemplates {
        OfferTemplates {
            plain,
            picture: Err("no DLOG 1020".to_owned()),
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn opened(offers: &mut VenueOffers) {
        offers.open(
            prompt("I'm in", Some("No thanks")),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
    }

    #[test]
    fn waiting_offers_ask_once_after_their_first_wait() {
        let mut offers = VenueOffers::waiting(plain_only(Ok(template())), ms(250));
        assert!(!offers.take_request(), "not before any tick");
        offers.tick(ms(249), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request());
        assert!(!offers.take_request(), "once");
        offers.tick(ms(10_000), false);
        assert!(!offers.take_request(), "not again until rearmed");
    }

    #[test]
    fn asking_offers_ask_before_any_tick_and_only_once() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert!(offers.take_request());
        assert!(!offers.take_request());
        offers.tick(ms(10_000), false);
        assert!(!offers.take_request());
    }

    #[test]
    fn by_the_other_reading_the_wait_does_not_run_down_while_an_offer_is_open() {
        let mut offers = on_the(
            RuleSource::Bible,
            VenueOffers::waiting(plain_only(Ok(template())), ms(100)),
        );
        opened(&mut offers);
        assert!(offers.dialog().is_some());
        offers.tick(ms(500), false);
        assert!(!offers.take_request(), "paused under the offer");
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.tick(ms(99), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request());
    }

    #[test]
    fn after_a_rearm_they_ask_again_after_the_gap() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert!(offers.take_request());
        offers.rearm(ms(600));
        offers.tick(ms(599), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request());
    }

    fn on_the(clock: RuleSource, offers: VenueOffers) -> VenueOffers {
        offers.with_clock(clock)
    }

    /// Offers on `clock` that have asked once and were answered with
    /// nothing, rearmed for 600 ms.
    fn rearmed(clock: RuleSource) -> VenueOffers {
        let mut offers = on_the(clock, VenueOffers::asking(plain_only(Ok(template()))));
        assert!(offers.take_request());
        offers.rearm(ms(600));
        offers
    }

    #[test]
    fn they_run_on_the_engines_clock_unless_told_otherwise() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert!(offers.take_request());
        offers.rearm(ms(600));
        offers.tick(ms(10_000), true);
        offers.tick(ms(0), false);
        assert!(offers.take_request(), "the engine's: ran on while covered");
    }

    #[test]
    fn by_the_engine_the_wait_runs_out_while_covered_and_asks_once_uncovered() {
        let mut offers = rearmed(RuleSource::Engine);
        offers.tick(ms(10_000), true);
        assert!(!offers.take_request(), "nothing is asked while covered");
        offers.tick(ms(0), false);
        assert!(offers.take_request(), "asked on the first uncovered tick");
        offers.tick(ms(10_000), false);
        assert!(!offers.take_request(), "once, until rearmed");
    }

    #[test]
    fn by_the_engine_a_wait_not_yet_out_runs_on_from_where_cover_left_it() {
        let mut offers = rearmed(RuleSource::Engine);
        offers.tick(ms(400), true);
        offers.tick(ms(199), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request());
    }

    #[test]
    fn by_the_other_reading_the_wait_stops_while_covered() {
        let mut offers = rearmed(RuleSource::Bible);
        offers.tick(ms(10_000), true);
        assert!(!offers.take_request());
        offers.tick(ms(599), false);
        assert!(!offers.take_request(), "the whole gap after the cover");
        offers.tick(ms(1), false);
        assert!(offers.take_request());
    }

    #[test]
    fn by_the_engine_the_gap_runs_from_the_ask_through_the_offer() {
        let mut offers = on_the(
            RuleSource::Engine,
            VenueOffers::asking(plain_only(Ok(template()))),
        );
        assert!(offers.take_request());
        opened(&mut offers);
        offers.tick(ms(400), false);
        assert!(!offers.take_request(), "not while the offer is open");
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.rearm(ms(600));
        offers.tick(ms(199), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request(), "600 ms after the ask");
    }

    #[test]
    fn by_the_engine_a_gap_already_past_asks_on_the_next_tick() {
        let mut offers = on_the(
            RuleSource::Engine,
            VenueOffers::waiting(plain_only(Ok(template())), ms(0)),
        );
        offers.tick(ms(0), false);
        assert!(offers.take_request());
        opened(&mut offers);
        offers.tick(ms(900), false);
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.rearm(ms(600));
        assert!(!offers.take_request(), "not before a tick");
        offers.tick(ms(0), false);
        assert!(offers.take_request());
    }

    #[test]
    fn by_the_other_reading_the_gap_runs_from_the_answer() {
        let mut offers = on_the(
            RuleSource::Bible,
            VenueOffers::asking(plain_only(Ok(template()))),
        );
        assert!(offers.take_request());
        opened(&mut offers);
        offers.tick(ms(400), false);
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.rearm(ms(600));
        offers.tick(ms(599), false);
        assert!(!offers.take_request());
        offers.tick(ms(1), false);
        assert!(offers.take_request());
    }

    #[test]
    fn by_the_engine_a_restart_waits_a_new_gap_from_the_restart() {
        let mut offers = rearmed(RuleSource::Engine);
        offers.tick(ms(500), false);
        assert!(!offers.take_restart(), "not restarted yet");
        offers.restart();
        offers.tick(ms(10_000), false);
        assert!(!offers.take_request(), "no wait until it is given a gap");
        assert!(offers.take_restart(), "asks for a new gap");
        assert!(!offers.take_restart(), "once");
        let mut offers = rearmed(RuleSource::Engine);
        offers.tick(ms(500), false);
        offers.restart();
        offers.tick(ms(100), true);
        offers.rearm(ms(600));
        offers.tick(ms(399), true);
        offers.tick(ms(0), false);
        assert!(!offers.take_request(), "the gap runs from the restart");
        offers.tick(ms(101), true);
        offers.tick(ms(0), false);
        assert!(offers.take_request());
    }

    #[test]
    fn a_restart_drops_an_ask_not_yet_taken() {
        let mut offers = rearmed(RuleSource::Engine);
        offers.tick(ms(600), false);
        offers.restart();
        assert!(!offers.take_request());
    }

    #[test]
    fn by_the_other_reading_a_restart_does_nothing() {
        let mut offers = rearmed(RuleSource::Bible);
        offers.tick(ms(500), false);
        offers.restart();
        assert!(!offers.take_restart());
        offers.tick(ms(100), false);
        assert!(offers.take_request());
    }

    #[test]
    fn an_open_offer_takes_the_input_and_its_choice_is_taken_once() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        let mut sounds = Vec::new();
        assert!(
            !offers.input(&key(Key::Enter), &mut sounds),
            "nothing open takes nothing"
        );
        opened(&mut offers);
        assert!(offers.input(&key(Key::Char('x')), &mut sounds));
        assert!(offers.dialog().is_some(), "x chooses nothing");
        assert!(offers.input(&key(Key::Char('n')), &mut sounds));
        assert!(offers.dialog().is_none(), "answered, it closes");
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert_eq!(offers.take_choice(), None, "once");
        assert!(!offers.input(&key(Key::Enter), &mut sounds));
    }

    #[test]
    fn an_open_offers_sounds_pointer_and_drawing_go_through_them() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        let mut list = DrawList::new();
        offers.draw(&mut list);
        assert!(list.iter().next().is_none(), "nothing open draws nothing");
        assert!(!offers.cancel_pointer());
        opened(&mut offers);
        let mut list = DrawList::new();
        offers.draw(&mut list);
        let commands: Vec<DrawCommand> = list.iter().cloned().collect();
        assert_eq!(commands, drawn(offers.dialog().expect("open")));
        let at = offers
            .dialog()
            .and_then(OfferDialog::dialog)
            .and_then(|laid| laid.item_bounds(ACCEPT_ITEM))
            .expect("accept")
            .center();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        let mut sounds = Vec::new();
        offers.input(&button(true), &mut sounds);
        assert!(offers.cancel_pointer());
        offers.input(&button(false), &mut sounds);
        assert_eq!(offers.take_choice(), None, "abandoned");
        assert_eq!(sounds, [DOWN]);
    }

    #[test]
    fn without_a_template_the_offer_says_why() {
        let mut offers = VenueOffers::asking(plain_only(Err("no DLOG 1016".to_owned())));
        opened(&mut offers);
        assert_eq!(
            offers.dialog().and_then(OfferDialog::problem),
            Some("no DLOG 1016")
        );
    }

    // The picture variant, `DLOG` 1020.

    /// Stock `DLOG` 1020, 651 x 244 and centred, with its ten disabled
    /// user items as `DITL` 1020 has them, 4, 5 and 7 parked outside it.
    fn picture_template() -> DialogTemplate {
        let items = [
            (221.0, 213.0, 99.0, 25.0),
            (115.0, 213.0, 99.0, 25.0),
            (13.0, 7.0, 413.0, 200.0),
            (147.0, 319.0, 32.0, 32.0),
            (194.0, 322.0, 32.0, 32.0),
            (170.0, 213.0, 99.0, 25.0),
            (169.0, 333.0, 32.0, 32.0),
            (441.0, 7.0, 200.0, 200.0),
            (340.0, 214.0, 23.0, 23.0),
            (373.0, 214.0, 23.0, 23.0),
        ];
        DialogTemplate {
            bounds: rect(-20.0, 103.0, 651.0, 244.0),
            placement: Placement::Center,
            items: items
                .into_iter()
                .map(|(x, y, w, h)| ItemTemplate {
                    bounds: rect(x, y, w, h),
                    enabled: false,
                    kind: ItemSpec::User,
                })
                .collect(),
        }
    }

    /// Where 1020 goes: (1024 - 651) / 2 and (768 - 244) / 2, floored.
    const PICTURE_ORIGIN: Point = Point::new(186.0, 262.0);

    fn pictured(graphic: i16, refuse: Option<&str>) -> OfferPrompt {
        OfferPrompt {
            graphic: Some(graphic),
            ..prompt("I'm in", refuse)
        }
    }

    fn templates() -> OfferTemplates {
        OfferTemplates {
            plain: Ok(template()),
            picture: Ok(picture_template()),
        }
    }

    #[test]
    fn the_picture_variants_named_values() {
        assert_eq!(
            (PICTURE_OFFER_DIALOG, OFFER_PICTURE_ITEM, PICTURE_FRAME),
            (1020, 8, ImageKey::picture(8528))
        );
    }

    #[test]
    fn a_graphic_above_127_takes_1020_on_the_id_alone() {
        let templates = OfferTemplates {
            plain: Err("no DLOG 1016".to_owned()),
            picture: Err("no DLOG 1020".to_owned()),
        };
        for graphic in [None, Some(-5), Some(0), Some(127)] {
            assert_eq!(
                templates.layout(graphic),
                Err("no DLOG 1016".to_owned()),
                "{graphic:?}"
            );
        }
        for graphic in [128, 5008] {
            assert_eq!(
                templates.layout(Some(graphic)),
                Err("no DLOG 1020".to_owned())
            );
        }
        assert_eq!(self::templates().layout(Some(128)), Ok(picture_template()));
        assert_eq!(self::templates().layout(Some(127)), Ok(template()));
    }

    #[test]
    fn the_picture_variant_lays_out_1020_and_draws_8528_then_the_items_then_the_graphic() {
        let dialog = OfferDialog::new(
            Ok((picture_template(), Rc::new(MonoMetrics))),
            pictured(5008, Some("No thanks")),
            ButtonStyle::STOCK,
        );
        assert_eq!(dialog.problem(), None);
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds(), Bounds::at(PICTURE_ORIGIN, 651.0, 244.0));
        let at =
            |x, y, w, h| Bounds::at(Point::new(PICTURE_ORIGIN.x + x, PICTURE_ORIGIN.y + y), w, h);
        assert_eq!(
            laid.item_bounds(TEXT_ITEM),
            Some(at(13.0, 7.0, 413.0, 200.0))
        );
        assert_eq!(
            laid.text(TEXT_ITEM).map(|text| text.lines().join(" ")),
            Some("Ferry this to Mars.".to_owned())
        );
        assert_eq!(
            laid.item_bounds(ACCEPT_ITEM),
            Some(at(221.0, 213.0, 99.0, 25.0))
        );
        assert_eq!(
            laid.item_bounds(REFUSE_ITEM),
            Some(at(115.0, 213.0, 99.0, 25.0))
        );
        assert!(laid.item_shown(ACCEPT_ITEM) && laid.item_shown(REFUSE_ITEM));
        let commands = drawn(&dialog);
        assert_eq!(
            commands[0],
            DrawCommand::StretchedPicture {
                image: PICTURE_FRAME,
                top_left: PICTURE_ORIGIN,
                width: 651.0,
                height: 244.0,
            },
            "one frame over the port (@0x98b9b-0x98bc3)"
        );
        let mut alone = DrawList::new();
        laid.draw(&mut alone);
        let alone: Vec<DrawCommand> = alone.iter().cloned().collect();
        assert_eq!(commands[1..=alone.len()], alone[..]);
        let item = at(441.0, 7.0, 200.0, 200.0);
        assert_eq!(
            commands[alone.len() + 1..],
            [DrawCommand::StretchedPicture {
                image: ImageKey::picture(5008),
                top_left: item.min,
                width: 200.0,
                height: 200.0,
            }],
            "the graphic stretched into item 8 (@0x98bdd-0x98c53)"
        );
        let plain = [FRAME.top, FRAME.middle, FRAME.bottom];
        assert!(
            !commands.iter().any(|command| matches!(
                command,
                DrawCommand::Picture { image, .. }
                    | DrawCommand::StretchedPicture { image, .. }
                    | DrawCommand::TiledPicture { image, .. }
                    if plain.contains(image)
            )),
            "no plain three-slice frame under the picture variant: {commands:?}"
        );
    }

    #[test]
    fn the_picture_variant_of_an_unrefusable_offer_shows_its_one_button_and_chooses() {
        let mut dialog = OfferDialog::new(
            Ok((picture_template(), Rc::new(MonoMetrics))),
            pictured(200, None),
            ButtonStyle::STOCK,
        );
        let laid = dialog.dialog().expect("laid out");
        assert!(laid.item_shown(SINGLE_ITEM));
        assert!(!laid.item_shown(ACCEPT_ITEM) && !laid.item_shown(REFUSE_ITEM));
        click_item(&mut dialog, SINGLE_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_venue_shows_a_pictured_offer_in_1020_and_a_plain_one_in_1016() {
        let mut offers = VenueOffers::asking(templates());
        offers.open(
            pictured(5008, Some("No")),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        let shown = offers
            .dialog()
            .and_then(OfferDialog::dialog)
            .expect("laid out");
        assert_eq!(shown.bounds().min, PICTURE_ORIGIN);
        offers.input(&key(Key::Enter), &mut Vec::new());
        opened(&mut offers);
        let shown = offers
            .dialog()
            .and_then(OfferDialog::dialog)
            .expect("laid out");
        assert_eq!(shown.bounds().min, ORIGIN);
        assert!(matches!(
            drawn(offers.dialog().expect("open"))[0],
            DrawCommand::StretchedPicture { image, .. } if image == FRAME.top
        ));
    }

    #[test]
    fn without_the_picture_template_a_pictured_offer_says_why() {
        let mut offers = VenueOffers::asking(OfferTemplates {
            plain: Ok(template()),
            picture: Err("no DLOG 1020".to_owned()),
        });
        offers.open(
            pictured(5008, Some("No")),
            Rc::new(MonoMetrics),
            ButtonStyle::STOCK,
        );
        assert_eq!(
            offers.dialog().and_then(OfferDialog::problem),
            Some("no DLOG 1020")
        );
    }

    #[test]
    fn the_mission_info_key_asks_for_mission_info_before_the_labels_letters() {
        let i = crate::ui::mission_info::MISSION_INFO_KEY;
        let mut dialog = offer();
        assert!(!dialog.take_info_request());
        dialog.input(&Input::Key {
            key: i,
            pressed: true,
            repeat: true,
        });
        assert!(!dialog.take_info_request(), "not a repeat");
        dialog.input(&key(i));
        assert!(dialog.take_info_request());
        assert!(!dialog.take_info_request(), "once");
        assert_eq!(dialog.take_choice(), None, "not I'm in's letter");
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert!(!offers.take_info_request(), "nothing open");
        opened(&mut offers);
        offers.input(&key(i), &mut Vec::new());
        assert!(offers.take_info_request());
        assert!(!offers.take_info_request(), "once");
        assert!(offers.dialog().is_some(), "still open");
    }

    fn film(flags: u16) -> Option<DescMovie> {
        DescMovie::from_desc("Offer.mov", flags)
    }

    fn filmed(flags: u16) -> OfferPrompt {
        OfferPrompt {
            movie: film(flags),
            ..prompt("I'm in", Some("No thanks"))
        }
    }

    fn movies(player: &Rc<MockMoviePlayer>) -> Movies {
        Movies::new(Rc::clone(player) as Rc<dyn MoviePlayer>, true)
    }

    const TICK: Duration = Duration::from_millis(100);

    #[test]
    fn an_offer_movie_with_flag_clear_plays_before_the_dialog() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&player));
        assert!(dialog.playing());
        assert!(!offer().playing());
        assert_eq!(player.calls(), [Call::Start("Offer.mov".to_owned())]);
        assert!(
            texts(&dialog).is_empty(),
            "no dialog drawn: {:?}",
            texts(&dialog)
        );
        assert_eq!(player.drawn().len(), 1, "the movie is");
        dialog.input(&key(Key::Char('n')));
        assert!(!dialog.answered(), "the dialog takes no input yet");
        dialog.tick(TICK);
        dialog.tick(TICK);
        assert!(!dialog.playing());
        assert!(texts(&dialog).contains(&"Ferry this to Mars.".to_owned()));
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Refuse),
            "the N pressed under the movie reaches the dialog as it opens"
        );
    }

    #[test]
    fn a_skipped_offer_movie_opens_the_dialog_with_nothing_queued() {
        let player = MockMoviePlayer::playing((160, 120), 20);
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&player));
        dialog.input(&key(Key::Char('n')));
        click_at(&mut dialog, Point::new(500.0, 520.0));
        assert!(texts(&dialog).contains(&"Ferry this to Mars.".to_owned()));
        assert!(
            !dialog.answered(),
            "the N flushed, and the click's release went to no button"
        );
    }

    #[test]
    fn an_offer_movie_with_flag_set_is_left_to_the_session() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0x0001)).with_movies(&movies(&player));
        assert!(player.calls().is_empty(), "not before the dialog");
        assert!(texts(&dialog).contains(&"Ferry this to Mars.".to_owned()));
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Accept),
            "the choice comes at once: its texts play before the movie"
        );
        assert!(
            player.calls().is_empty(),
            "the session queues it after them"
        );
    }

    #[test]
    fn with_movies_off_or_a_movie_that_cannot_start_the_dialog_opens_at_once() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let off = Movies::new(Rc::clone(&player) as Rc<dyn MoviePlayer>, false);
        let dialog = dialog_of(filmed(0)).with_movies(&off);
        assert!(texts(&dialog).contains(&"Ferry this to Mars.".to_owned()));
        assert!(player.calls().is_empty());
        let refusing = MockMoviePlayer::refusing();
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&refusing));
        assert!(texts(&dialog).contains(&"Ferry this to Mars.".to_owned()));
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_cinematic_offer_movie_hides_the_pointer_and_requests_nothing() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0x0004)).with_movies(&movies(&player));
        dialog.tick(MOVIE_FADE_FOR);
        assert!(dialog.hides_pointer());
        dialog.input(&key(Key::Char('p')));
        dialog.input(&key(Key::Char('i')));
        assert!(!dialog.take_player_info_request());
        assert!(!dialog.take_info_request());
        dialog.cancel_pointer();
        assert!(!offer().hides_pointer());
    }

    #[test]
    fn the_venues_offer_plays_its_before_movie_and_hands_out_the_choice() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        offers.set_movies(movies(&player));
        offers.open(filmed(0), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        assert_eq!(player.calls(), [Call::Start("Offer.mov".to_owned())]);
        offers.tick(TICK, false);
        assert!(
            offers
                .dialog()
                .is_some_and(|dialog| texts(dialog).contains(&"Ferry this to Mars.".to_owned())),
            "ticked with the venue"
        );
        let mut sounds = Vec::new();
        offers.input(&key(Key::Enter), &mut sounds);
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Accept))
        );
    }

    #[test]
    fn a_choice_pressed_under_the_venues_movie_settles_on_the_tick_it_ends() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        offers.set_movies(movies(&player));
        offers.open(filmed(0x0000), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        let mut sounds = Vec::new();
        offers.input(&key(Key::Char('n')), &mut sounds);
        assert_eq!(offers.take_choice(), None, "not while the movie plays");
        offers.tick(TICK, false);
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Refuse)),
            "on the tick the movie ends"
        );
        assert!(offers.dialog().is_none(), "and the dialog closes");
    }
}
