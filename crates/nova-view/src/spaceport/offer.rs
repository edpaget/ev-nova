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
//! and 7 (the map, player info and mission info) lie outside it. The
//! scroll arrows, items 9 and 10, are drawn and tracked as the text
//! dialog's are, and the text scrolls by pixels, a partial line cut at the
//! box ([`offer_arrows`](super::offer_arrows)).
//!
//! The dialog shrinks 1016 to its text as the original does, after
//! `_CenterWindow` has placed it at its stock size (@0xa23fd,
//! @0xa2548-0xa2816): the rule [`text_shrink`](crate::ui::text_shrink)
//! ports for "Desc Dialog" (`d = boxH − max(h, 48) − 16`, `SizeWindow`
//! @0xa25d1), measured as the text box sets the text. Items 1, 2 and 6 and
//! the text box's bottom come up by `d`, and the arrows of items 9 and 10
//! stay where they are, unlike "Desc Dialog"'s, so a dialog shrunk far
//! enough leaves them outside it, and one shrunk less never draws them, its
//! text always fitting its box. The window's top then becomes
//! `trunc(top + 0.3 × d)` (`MoveWindow` @0xa2640), the sum truncated and
//! not the product. All of it happens before the dialog shows
//! (`ShowWindowNoFlash` @0xa2c81), so the player sees the final place only.
//!
//! Then, in 1016 and 1020 alike, the buttons widen to the wider label + 40,
//! at least 100 ([`offer_buttons`](super::offer_buttons), @0xa2a29-0xa2beb).
//!
//! An offer whose `dësc` `Graphic` is above 127 shows in the picture
//! variant, `DLOG` 1020 ([`OfferTemplates::layout`], @0xa2344-0xa2373),
//! chosen on the ID alone: 651 x 244, centred, the text in item 3, the
//! buttons in items 1, 2 and 6 as in 1016, over the single frame `PICT`
//! 8528, with the graphic stretched into item 8. The original never
//! shrinks 1020 (`jg` past the shrink @0xa255b-0xa2564, the same `Graphic`
//! read at both places, as nothing writes `*0x3b52a0` between them), so
//! its template height is its own.
//!
//! An offer whose `dësc` names a movie ([`OfferPrompt::movie`]) plays it
//! while movies are on ([`Movies`]): before the dialog
//! (`_OfferOneMission` @0xa2273-0xa22ab), which is neither drawn nor
//! given input until the movie ends ([`OfferDialog::with_movies`]). The
//! movie begins on the dialog's first tick or input, which its holder
//! gives only once no mission dialog covers it, as `_PlayQTMovie` is
//! modal: an offer opened under a mission text waits for the text and its
//! movie. One
//! with `Flags` 0x0001 set plays after the answer has been acted on, its
//! `BriefText` or `RefuseText` shown (@0xa3580-0xa35b8, after the accept
//! @0xa2e06 and the refuse text @0xa2f04): the dialog plays none of it and
//! takes the choice at once, and the session queues that movie behind the
//! texts the answer shows
//! ([`MissionShown::Movie`](nova_sim::MissionShown::Movie)).
//!
//! Keys pressed while a press is held on an arrow, or a click tracked on a
//! button or an arrow, wait for the release (`_SingleMissionFilter`'s held
//! loops and `_TrackSingleMissionButtons` @0x28188 read no events), the
//! map, player info and mission info keys, the labels' letters and the page
//! keys among them. The release runs them in order through the dialog's
//! own input, within the same input, so a key that answers answers by the
//! input that let go, and a press its holder cancels runs them on the next
//! tick. What they queued after the answering key is left for what shows
//! next ([`OfferDialog::take_unclaimed`]), as the answer's `FlushEvents`
//! flush only mouse events; after M, P or I it goes with that screen's
//! flush. M and P always open theirs where an offer can hold keys (the
//! course map opens unasked, and the player info screen is refused only in
//! a jump or while the ship breaks up, under no offer); I opens nothing,
//! and so flushes nothing, with no mission listed
//! ([`OfferDialog::set_missions_listed`]). See the `held_keys` module.
//!
//! Keys (`_SingleMissionFilter` @0x98d9a): Return and Enter accept (the
//! characters 0x0D and 0x03, @0x98e62-0x98e6e). The labels' letters are
//! read from the character typed ([`Input::Text`]), as the filter folds the
//! event's character with `tolower` (@0x98e4a-0x98eea), not from the key
//! that typed it, and only from a press's, never a repeat's, as it takes
//! them on `keyDown` alone (@0x98e56): the accept label's first letter
//! accepts and, when the mission can be refused, the refuse label's
//! refuses, unless both start with the same letter. The character follows
//! its key's press, which the dialog notes; a press the map, player info
//! or mission info key took types nothing it reads. Escape does nothing. Up and Down scroll the text 10 px, repeats
//! included, and Home, End, Page Up and Page Down as the text dialog's do
//! (`ui::desc`'s `page_key`), on the press alone by the user's
//! standing decision, though the filter reads auto-key too
//! (@0x98efd-0x98fdf). The choice is taken once
//! ([`OfferDialog::take_choice`]), and the dialog is then answered. P
//! ([`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY), a press) asks for the player info screen
//! ([`OfferDialog::take_player_info_request`], @0xa3215), before the
//! labels' letters are read; the dialog stays open under it. I
//! ([`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY), a
//! press) asks for the mission info screen
//! ([`OfferDialog::take_info_request`], @0x98e0c-0x98e2a, opened by
//! `_OfferOneMission` @0xa3344), also before the letters; the dialog
//! stays open under it. With no active mission that isn't hidden it
//! asks for nothing and does nothing else (@0xa32c1-0xa32ee), as its
//! holder tells it ([`OfferDialog::set_missions_listed`]). The original then reloads the offer's `dësc`
//! (`_LoadDescription` @0xa335c), which redraws what the info screen
//! covered and leaves its movie the last loaded again: the spaceport asks
//! for that reload as the screen closes
//! ([`SpaceportView::take_offer_reload`](super::SpaceportView::take_offer_reload)).
//! M ([`MAP_KEY`](crate::flight::view::MAP_KEY), a press, never a
//! repeat), the filter's first check (@0x98dab-0x98dd3), asks for the
//! course map before anything else ([`OfferDialog::take_map_request`]),
//! marked with the offer's destination; the dialog stays open under it,
//! and nothing sounds.
//!
//! The answer hides the dialog only for an accept from the dialog with a
//! refuse button (@0xa2df3) or a refusal that showed a `RefuseText`
//! (@0xa2e8e). Otherwise it stays drawn, answered and inert, under the
//! mission dialogs the answer opened, its after-movie among them, until
//! they have all closed (`DisposeDialogNoFlash` @0xa361e). Every answer
//! then waits while I is down (@0xa35fc-0xa3614), the dialog drawn (or
//! hidden) and inert, the venue under it taking nothing; on I's release
//! comes the rest (@0xa361e-0xa3670): the disposal, the fade back in, and
//! a `FlushEvents(7, 0)` that keeps the keys typed meanwhile for the
//! venue. See the `close` module. The dialog
//! makes no sound (`_TrackSingleMissionButtons` @0x28188 plays nothing,
//! and its keys are silent), so none waits for the next input.
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

mod close;

use super::offer_arrows::arrow_roles;
use super::offer_buttons::widen_to_labels;
use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::flight::view::is_map_press;
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::movie::{MovieStage, Movies};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::button::{ButtonSkin, ButtonStyle};
use crate::ui::desc::{BODY_COLOR, BODY_FONT, BODY_SIZE, page_key, shown_graphic};
use crate::ui::dialog::{Dialog, DialogEvent, DialogFrame, DialogTemplate, FocusRing, Role};
use crate::ui::mission_info::is_mission_info_press;
use crate::ui::picture_variant::VariantTemplates;
use crate::ui::player_info::is_player_info_press;
use crate::ui::text_shrink::{shrink_after_placing, wrapped_height};

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

/// The items that come up with `DLOG` 1016's bottom edge
/// (@0xa2645-0xa27b1). Items 9 and 10, the scroll arrows, are not among
/// them: `_OfferOneMission` never moves them, unlike `_ShowTextDialog`'s.
const MOVED_ITEMS: [usize; 3] = [ACCEPT_ITEM, REFUSE_ITEM, SINGLE_ITEM];

/// `template` (`DLOG` 1016) shrunk to `text`, measured as its text box
/// sets it, by `metrics`, and moved down: see the module docs.
fn shrunk_to_text(
    template: &DialogTemplate,
    text: &str,
    metrics: &dyn TextMetrics,
) -> DialogTemplate {
    let height = template.items.get(TEXT_ITEM - 1).map_or(0.0, |item| {
        wrapped_height(metrics, BODY_FONT, BODY_SIZE, text, item.bounds.width())
    });
    shrink_after_placing(template, TEXT_ITEM, &MOVED_ITEMS, height)
}

/// What the player chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OfferChoice {
    /// Accept the mission.
    Accept,
    /// Refuse it.
    Refuse,
}

/// What the offer dialog's keys asked for, each until it is taken.
#[derive(Clone, Copy, Debug, Default)]
struct OfferAsks {
    /// [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY): the
    /// player info screen.
    player_info: bool,
    /// [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY):
    /// the mission info screen.
    mission_info: bool,
    /// [`MAP_KEY`](crate::flight::view::MAP_KEY): the map.
    map: bool,
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
    /// What its keys asked for, until each is taken.
    asks: OfferAsks,
    /// The picture variant's graphic, drawn in item 8 over
    /// [`PICTURE_FRAME`]; none for the plain dialog over [`FRAME`].
    picture: Option<i16>,
    /// The movie played before the dialog, until it ends.
    before: Option<MovieStage>,
    /// What a replay queued after the key that answered it, until taken
    /// ([`OfferDialog::take_unclaimed`]).
    unclaimed: Vec<Input>,
    /// Whether the mission info screen lists a mission, so that
    /// [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// opens it ([`OfferDialog::set_missions_listed`]).
    missions_listed: bool,
    /// Whether the last key input was a press, not a repeat, that reached
    /// the labels' letters, so that the character it types
    /// ([`Input::Text`], which follows it) is read as a `keyDown`'s.
    letter_pressed: bool,
    /// Its close: hidden, waiting or disposed of (the `close` module).
    close: close::Close,
}

impl OfferDialog {
    /// `prompt`, laid out by `layout`: the template of the variant its
    /// graphic picks ([`OfferTemplates::layout`]) and the metrics its text
    /// is measured by, or why there are none; its buttons drawn in
    /// `style`. A graphic above 127 draws the picture variant: the frame
    /// [`PICTURE_FRAME`] and the graphic in item 8. The plain variant is
    /// shrunk to the text first, and either way the buttons are widened to
    /// their labels, as the module docs say.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        prompt: OfferPrompt,
        style: ButtonStyle,
    ) -> Self {
        let picture = shown_graphic(prompt.graphic);
        let laid = layout.map(|(template, metrics)| {
            let mut template = if picture.is_none() {
                shrunk_to_text(&template, &prompt.text, &*metrics)
            } else {
                template
            };
            widen_to_labels(&mut template, &prompt.labels, &*metrics);
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
                    font: BODY_FONT,
                    size: BODY_SIZE,
                    color: BODY_COLOR,
                    backing: None,
                },
            )];
            roles.extend(arrow_roles());
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
                .with_cancel(None)
                .with_pixel_scroll()
                .with_held_keys();
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
            asks: OfferAsks::default(),
            picture,
            before: None,
            unclaimed: Vec::new(),
            close: close::Close::default(),
            missions_listed: true,
            letter_pressed: false,
        }
    }

    /// This dialog told whether the mission info screen lists a mission
    /// ([`OfferDialog::set_missions_listed`]).
    #[must_use]
    pub fn with_missions_listed(mut self, listed: bool) -> Self {
        self.set_missions_listed(listed);
        self
    }

    /// Tells the dialog whether the mission info screen lists a mission:
    /// an active one that isn't hidden. With none, `_OfferOneMission`'s
    /// arm for [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// does nothing (the count @0xa32c1-0xa32ee), so the key asks for
    /// nothing, flushes nothing, and the offer takes the next event, a key
    /// held back by a press among them. Its holder says so as the dialog
    /// opens and after an abort from the mission info screen over it, the
    /// only change to the missions while the offer is open. Listed until
    /// told otherwise.
    pub fn set_missions_listed(&mut self, listed: bool) {
        self.missions_listed = listed;
    }

    /// This dialog with its buttons taking the keyboard focus as `ring`
    /// says (the `dialog_focus_ring` rule).
    #[must_use]
    pub fn with_focus_ring(mut self, ring: FocusRing) -> Self {
        if let Ok(dialog) = &mut self.laid {
            dialog.set_focus_ring(ring);
        }
        self
    }

    /// This dialog with its offer's movie, as `movies` plays it: one that
    /// plays before the dialog is staged, and begins on the dialog's first
    /// tick or input, which its holder gives only once nothing covers it,
    /// so it never shares the player with a mission text's movie
    /// (`_PlayQTMovie` is modal); the dialog waits for it, drawing
    /// nothing. One that plays after the answer is the session's to queue.
    /// With movies off, none.
    #[must_use]
    pub fn with_movies(self, movies: &Movies) -> Self {
        let Some(movie) = self.prompt.movie.clone().filter(|movie| !movie.after) else {
            return self;
        };
        let mut dialog = self;
        dialog.before = movies.stage(&movie);
        dialog
    }

    /// Begins the movie before the dialog if it has not begun, dropping
    /// it at once if it cannot start.
    fn begin_movie(&mut self) {
        if let Some(stage) = &mut self.before {
            stage.begin();
        }
        self.settle_movie();
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
            self.run_replay();
        }
    }

    /// What a replay queued after the key that answered the offer, once:
    /// its holder gives it to what shows next, as the answer's
    /// `FlushEvents` calls flush only mouse events (event codes passed as
    /// masks, @0xa2da2-0xa2dde, @0xa2f70-0xa2fac; the refusal flushes
    /// nothing). A text the answer shows flushes it as it opens (@0x5ae67).
    pub fn take_unclaimed(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.unclaimed)
    }

    /// An accept's `FlushEvents(3, 0)`, `(5, 0)`, `(1, 0)` and `(2, 0)`
    /// flush mouse-downs and mouse-ups, their codes read as masks
    /// (@0xa2da2-0xa2dde, @0xa2f70-0xa2fac), so a press left after it goes;
    /// a refusal flushes nothing (@0xa2e65-0xa2f09).
    fn flush_answers_mouse(&mut self) {
        if self.choice == Some(OfferChoice::Accept) {
            self.unclaimed
                .retain(|input| !matches!(input, Input::PointerButton { .. }));
        }
    }

    /// Whether a key has asked for the map, player info or mission info,
    /// and not been taken.
    fn asked(&self) -> bool {
        let asks = self.asks;
        asks.map || asks.player_info || asks.mission_info
    }

    /// Runs what the dialog held back through [`Self::dialog_input`], in
    /// order, until a key answers the offer, whose rest is left for what
    /// shows next ([`Self::take_unclaimed`]), or asks for the map, player
    /// info or mission info, whose rest that screen's flush drops
    /// (`_DoSystemMap` @0x1257b, `_DoPlayerInfoDialog` @0x5a544,
    /// `_DoMissionInfoDialog` @0x9e287). Each asks only for a screen that
    /// opens, so the flush always follows: an I with no mission listed
    /// asks nothing, and the replay runs on past it.
    fn run_replay(&mut self) {
        let asked = self.asked();
        let Ok(dialog) = &mut self.laid else {
            return;
        };
        let mut replay = dialog.take_replay().into_iter();
        while let Some(input) = replay.next() {
            // A click's release can answer before the keys it held back.
            if self.answered {
                self.unclaimed.push(input);
                self.unclaimed.extend(replay);
                self.flush_answers_mouse();
                return;
            }
            self.dialog_input(&input);
            if self.asked() && !asked {
                return;
            }
        }
    }

    /// Whether [`PLAYER_INFO_KEY`](crate::ui::player_info::PLAYER_INFO_KEY) asked for the player info screen since
    /// this was last taken, once.
    pub fn take_player_info_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.player_info)
    }

    /// Whether [`MISSION_INFO_KEY`](crate::ui::mission_info::MISSION_INFO_KEY)
    /// asked for the mission info screen since this was last taken, once.
    pub fn take_info_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.mission_info)
    }

    /// Whether [`MAP_KEY`](crate::flight::view::MAP_KEY) asked for the
    /// map since this was last taken, once: `_SingleMissionFilter`'s first
    /// check gives item 4 (@0x98dab-0x98dd3), and `_OfferOneMission` opens
    /// the course map over the dialog (@0xa2ffe-0xa3119; see
    /// [`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)).
    pub fn take_map_request(&mut self) -> bool {
        std::mem::take(&mut self.asks.map)
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

    /// What the character `typed` chooses: each label's first letter,
    /// either case, chooses it, unless both start with the same letter.
    /// `_SingleMissionFilter` folds the event's character and each label's
    /// first with `tolower` (@0x98e4a-0x98eea), which folds ASCII alone.
    fn letter_choice(&self, typed: char) -> Option<OfferChoice> {
        let first = |label: &str| label.chars().next().map(|c| c.to_ascii_lowercase());
        let accept = first(&self.prompt.labels.accept);
        let refuse = self.prompt.labels.refuse.as_deref().and_then(first);
        if accept == refuse {
            return None;
        }
        let letter = Some(typed.to_ascii_lowercase());
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
    /// The dialog's input, its replay aside: Return, Enter and the labels'
    /// letters choose; everything else goes to the dialog. While the dialog
    /// holds its keys, everything goes straight to it. Once answered it
    /// takes nothing.
    fn dialog_input(&mut self, input: &Input) {
        if self.answered {
            return;
        }
        if let Ok(dialog) = &mut self.laid
            && dialog.holds()
        {
            let event = dialog.input(input);
            self.choose_from(event);
            return;
        }
        let letter_pressed = std::mem::take(&mut self.letter_pressed);
        if is_map_press(input) {
            self.asks.map = true;
            return;
        }
        if is_player_info_press(input) {
            self.asks.player_info = true;
            return;
        }
        if is_mission_info_press(input) {
            self.asks.mission_info |= self.missions_listed;
            return;
        }
        match *input {
            Input::Key {
                key: Key::Enter | Key::NumpadEnter,
                pressed: true,
                repeat: false,
            } => {
                self.choose(OfferChoice::Accept);
                return;
            }
            Input::Key {
                pressed: true,
                repeat: false,
                ..
            } => self.letter_pressed = true,
            Input::Text(typed) if letter_pressed => {
                if let Some(choice) = self.letter_choice(typed) {
                    self.choose(choice);
                }
                return;
            }
            _ => {}
        }
        let Ok(dialog) = &mut self.laid else {
            return;
        };
        if dialog
            .scroll_text_mut()
            .is_some_and(|text| page_key(text, input))
        {
            return;
        }
        let event = dialog.input(input);
        self.choose_from(event);
    }

    /// Chooses what the dialog's `event` activated, if anything.
    fn choose_from(&mut self, event: Option<DialogEvent>) {
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
    ///
    /// While a press is held on an arrow or a click tracked on a button or
    /// an arrow, keys wait for the release, as `_SingleMissionFilter`'s held
    /// loops and `_TrackSingleMissionButtons` read no events
    /// (@0x99060-0x991e6, @0x28188): the map, player info and mission info
    /// keys, the labels' letters and the page keys too. The release runs
    /// them in order, within the same input, so a key that answers answers
    /// by the input that let go; what they queued after it is left for what
    /// shows next ([`OfferDialog::take_unclaimed`]). See the `held_keys`
    /// module.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.close_input(input) {
            return ScreenAction::None;
        }
        self.begin_movie();
        if let Some(stage) = &mut self.before {
            stage.input(input);
            self.settle_movie();
        } else {
            self.dialog_input(input);
            self.run_replay();
        }
        ScreenAction::None
    }

    /// The movie before the dialog begins, if it has not, and moves on.
    /// Once no movie plays, the dialog ticks: a press held on an arrow
    /// scrolls a pixel a Mac tick ([`Dialog::tick`]).
    fn tick(&mut self, dt: Duration) {
        self.begin_movie();
        if let Some(stage) = &mut self.before {
            stage.tick(dt);
            self.settle_movie();
        } else if let Ok(dialog) = &mut self.laid {
            dialog.tick(dt);
            self.run_replay();
        }
    }

    /// While a cinematic movie hides the cursor.
    fn hides_pointer(&self) -> bool {
        self.before.as_ref().is_some_and(MovieStage::hides_cursor)
    }

    /// The laid-out dialog's readout ([`Dialog::holds_back_keys`]): false
    /// with no layout.
    fn holds_back_keys(&self) -> bool {
        self.laid.as_ref().is_ok_and(Dialog::holds_back_keys)
    }

    /// The plain dialog draws its frame under its items. The picture
    /// variant draws [`PICTURE_FRAME`] stretched over the dialog, as
    /// `_SingleMissionDialogUpdate` fills the port with it, then the
    /// items, then the graphic stretched into item 8 (@0x98bdd-0x98c53);
    /// a graphic whose `PICT` is missing leaves item 8 empty, as
    /// `GetPicture` finds nothing to draw.
    fn draw(&self, list: &mut DrawList) {
        if self.close.hidden() {
            return;
        }
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

    /// Abandons a press, whose held keys run on the next tick.
    fn cancel_pointer(&mut self) {
        if self.before.is_some() {
            return;
        }
        if let Ok(dialog) = &mut self.laid {
            dialog.cancel_pointer();
        }
    }

    /// Letting go of the keys while a press holds them abandons the press,
    /// as [`Screen::cancel_pointer`] does; otherwise nothing is held here.
    fn release_keys(&mut self) {
        self.release_close_wait();
        if let Ok(dialog) = &mut self.laid
            && dialog.holds()
        {
            dialog.cancel_pointer();
        }
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
    /// The offer dialog answered, drawn (unless hidden) and inert under
    /// the mission dialogs its answer shows until they close and its
    /// holder releases it ([`VenueOffers::release_answered`]), then until
    /// I's release if it is down (the `close` module).
    answered: Option<OfferDialog>,
    /// The offer answered and its choice, until it is taken.
    choice: Option<(MissionId, OfferChoice)>,
    /// The movies the offers play, and whether they play.
    movies: Movies,
    /// The offer dialog's sounds not yet handed on.
    sounds: Vec<Sound>,
    /// How the timer runs ([`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock)).
    clock: RuleSource,
    /// Whether the offer dialog's buttons take the keyboard focus.
    focus_ring: FocusRing,
    /// The time since the venue last asked (or, before it first asks,
    /// since it opened), which the engine's gap runs from.
    since_ask: Duration,
    /// Whether the timer restarted since this was last taken, and wants
    /// a new gap.
    restarted: bool,
    /// What the answered dialog's replay queued after its answering key
    /// ([`OfferDialog::take_unclaimed`]), until taken.
    unclaimed: Vec<Input>,
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
            answered: None,
            choice: None,
            movies: Movies::none(),
            sounds: Vec::new(),
            clock: RuleSource::Engine,
            focus_ring: FocusRing::default(),
            since_ask: Duration::ZERO,
            restarted: false,
            unclaimed: Vec::new(),
        }
    }

    /// These offers with their timer running as `clock` reads
    /// [`RuleKey::OfferClock`](nova_sim::RuleKey::OfferClock).
    #[must_use]
    pub(crate) fn with_clock(self, clock: RuleSource) -> Self {
        Self { clock, ..self }
    }

    /// These offers' dialogs with their buttons taking the keyboard focus
    /// as `ring` says, for each opened after it.
    #[must_use]
    pub(crate) fn with_focus_ring(self, ring: FocusRing) -> Self {
        Self {
            focus_ring: ring,
            ..self
        }
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

    /// Whether an offer is asked for and not yet taken, or open: the
    /// venue's offer stands between it and the player.
    pub(crate) fn pending(&self) -> bool {
        self.requested || self.dialog.is_some()
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
        self.dialog = Some(
            OfferDialog::new(layout, prompt, style)
                .with_focus_ring(self.focus_ring)
                .with_movies(&self.movies),
        );
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

    /// Tells the offer dialog open whether the mission info screen lists
    /// a mission ([`OfferDialog::set_missions_listed`]).
    pub(crate) fn set_missions_listed(&mut self, listed: bool) {
        if let Some(dialog) = &mut self.dialog {
            dialog.set_missions_listed(listed);
        }
    }

    /// Whether the offer dialog open asked for the mission info screen
    /// since this was last taken, once.
    pub(crate) fn take_info_request(&mut self) -> bool {
        self.dialog
            .as_mut()
            .is_some_and(OfferDialog::take_info_request)
    }

    /// The mission of the offer dialog open, when it asked for the map
    /// since this was last taken, once.
    pub(crate) fn take_map_request(&mut self) -> Option<MissionId> {
        let dialog = self.dialog.as_mut()?;
        dialog.take_map_request().then_some(dialog.prompt().mission)
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
        if self.answer_input(input) {
            return true;
        }
        let Some(dialog) = &mut self.dialog else {
            return false;
        };
        dialog.input(input);
        self.settle_dialog();
        sounds.append(&mut self.sounds);
        true
    }

    /// Keeps the offer dialog's sounds; answered, it closes, its choice
    /// kept until taken. An accept from the dialog with a refuse button
    /// hides it (`_OfferOneMission` @0xa2df3). The one-button dialog's
    /// accept does not (@0xa2f52-0xa2ff7), nor does a refusal, until its
    /// holder knows it showed a `RefuseText` (@0xa2e8e): those stay drawn
    /// under what the answer shows ([`VenueOffers::release_answered`]),
    /// until `DisposeDialogNoFlash` (@0xa361e).
    fn settle_dialog(&mut self) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        self.sounds.extend(dialog.take_sounds());
        if let Some(choice) = dialog.take_choice() {
            self.choice = Some((dialog.prompt().mission, choice));
            self.unclaimed = dialog.take_unclaimed();
            if choice == OfferChoice::Accept && dialog.prompt().labels.refuse.is_some() {
                dialog.hide();
            }
            self.answered = self.dialog.take();
        }
    }

    /// What the answered dialog's replay queued after the key that answered
    /// it, once its choice has been taken, and so acted on: the venue runs
    /// it through its own input as its next tick begins, the answer's own
    /// `FlushEvents` flushing only mouse events (@0xa2da2-0xa2dde). A tick
    /// covered before then drops it ([`VenueOffers::tick`]). Nothing while
    /// the choice waits to be taken.
    pub(crate) fn take_unclaimed(&mut self) -> Vec<Input> {
        if self.choice.is_some() || self.answer_waits() {
            return Vec::new();
        }
        std::mem::take(&mut self.unclaimed)
    }

    /// The offer dialog answered and left drawn under what its answer
    /// shows, if any.
    #[cfg(test)]
    pub(crate) fn answered(&self) -> Option<&OfferDialog> {
        self.answered.as_ref()
    }

    /// The wait for the next offer runs down by `dt`; once it is out, and
    /// nothing is `covered` over the venue nor is the offer open, the venue
    /// asks. By the engine it runs down whatever covers the venue; by the
    /// other reading only while nothing does.
    /// The offer dialog open ticks too, its movie moving on, unless
    /// something covers it ([`VenueOffers::tick_dialog`]).
    ///
    /// Covered, it drops what the answer left once its choice has been
    /// taken ([`VenueOffers::take_unclaimed`]): what covers the venue then
    /// is a text the answer shows, or an info screen, and each flushes the
    /// queue as it opens (`_ShowTextDialog` @0x5ae67, `_DoMissionInfoDialog`
    /// @0x9e287, `_DoPlayerInfoDialog` @0x5a544).
    pub(crate) fn tick(&mut self, dt: Duration, covered: bool) {
        if covered {
            drop(self.take_unclaimed());
        } else {
            self.tick_dialog(dt);
        }
        // An answer waiting for I's release covers the venue too: the
        // filter's deadline check can't run while `_OfferOneMission` spins
        // in its wait (@0xa35fc-0xa3614), though the deadline, a tick
        // count, runs on (the `close` module).
        let covered = covered || self.dialog.is_some() || self.answer_waits();
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

    /// Ticks the offer dialog, if it is open: its movie begins or moves
    /// on, and the keys pressed under a movie that ends can answer it, on
    /// that tick. [`VenueOffers::tick`] ticks it only while nothing covers
    /// it.
    fn tick_dialog(&mut self, dt: Duration) {
        if let Some(dialog) = &mut self.dialog {
            dialog.tick(dt);
            self.settle_dialog();
        }
    }

    /// Draws the offer dialog answered and left drawn, then the one open.
    pub(crate) fn draw(&self, list: &mut DrawList) {
        if let Some(dialog) = &self.answered {
            dialog.draw(list);
        }
        if let Some(dialog) = &self.dialog {
            dialog.draw(list);
        }
    }

    /// Whether keys typed now would come after keys held back here: the
    /// open dialog's ([`OfferDialog`]'s [`Screen::holds_back_keys`]), or
    /// what its answer left until its holder runs it
    /// ([`VenueOffers::take_unclaimed`]) while it holds an event the
    /// original queues ([`queues_an_event`]). Never the answered dialog's,
    /// which takes no input.
    pub(crate) fn holds_back_keys(&self) -> bool {
        self.dialog.as_ref().is_some_and(Screen::holds_back_keys)
            || self.unclaimed.iter().any(queues_an_event)
    }

    /// Cancels the offer dialog's pointer, if it is open: whether it was.
    pub(crate) fn cancel_pointer(&mut self) -> bool {
        self.dialog
            .as_mut()
            .map(OfferDialog::cancel_pointer)
            .is_some()
    }
}

/// Whether the original's event queue holds an event for `input`: not
/// for a key's release, as the system event mask leaves out key-ups by
/// default and the game never sets it (no `SetEventMask` in `EV Nova`),
/// nor for a modifier going down, which the classic event model has no
/// event for.
fn queues_an_event(input: &Input) -> bool {
    match *input {
        Input::Key { key, pressed, .. } => {
            pressed
                && !matches!(
                    key,
                    Key::AltLeft
                        | Key::AltRight
                        | Key::ControlLeft
                        | Key::ControlRight
                        | Key::ShiftLeft
                        | Key::ShiftRight
                        | Key::MetaLeft
                        | Key::MetaRight
                        | Key::CapsLock
                )
        }
        _ => true,
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

    /// Stock `DLOG` 1016, 441 x 317 and centred, with its ten disabled
    /// user items as `DITL` 1016 has them, 4, 5 and 7 parked outside it.
    pub(crate) fn stock_1016() -> DialogTemplate {
        stock(
            (441.0, 317.0),
            [
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
            ],
        )
    }

    /// Stock `DLOG` 1020's shape, 651 x 244 and centred: its buttons, text,
    /// picture and arrows where `DITL` 1020 has them, 4, 5 and 7 parked
    /// outside it.
    pub(crate) fn stock_1020() -> DialogTemplate {
        stock(
            (651.0, 244.0),
            [
                (221.0, 213.0, 99.0, 25.0),
                (115.0, 213.0, 99.0, 25.0),
                (13.0, 7.0, 413.0, 200.0),
                (150.0, 300.0, 32.0, 32.0),
                (197.0, 300.0, 32.0, 32.0),
                (170.0, 213.0, 99.0, 25.0),
                (172.0, 340.0, 32.0, 32.0),
                (441.0, 7.0, 200.0, 200.0),
                (340.0, 214.0, 23.0, 23.0),
                (373.0, 214.0, 23.0, 23.0),
            ],
        )
    }

    /// A centred dialog `size` big with disabled user items at `items`.
    fn stock((width, height): (f32, f32), items: [(f32, f32, f32, f32); 10]) -> DialogTemplate {
        let rect = |x, y, w, h| Bounds::at(Point::new(x, y), w, h);
        DialogTemplate {
            bounds: rect(40.0, 40.0, width, height),
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

    /// A click on `dialog`'s accept button (item 1) with `held` pressed
    /// while the click is held: the press, `held` and the release, which
    /// accepts and leaves `held` for what shows next.
    pub(crate) fn accept_click_holding(
        dialog: &super::OfferDialog,
        held: crate::input::Input,
    ) -> [crate::input::Input; 3] {
        let at = dialog
            .dialog()
            .and_then(|dialog| dialog.item_bounds(super::ACCEPT_ITEM))
            .expect("the accept button")
            .center();
        let left = |pressed| crate::input::Input::PointerButton {
            button: crate::input::MouseButton::Left,
            pressed,
            at,
        };
        [left(true), held, left(false)]
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim::mission::OfferLabels;
    use nova_sim::{DescMovie, RuleSource};

    use super::*;
    use crate::draw::DrawCommand;
    use crate::geometry::{Bounds, Point};
    use crate::input::MouseButton;
    use crate::movie::fixture::{Call, MockMoviePlayer};
    use crate::movie::{MOVIE_FADE_FOR, MoviePlayer};
    use crate::spaceport::offer_buttons::{LABEL_FONT, LABEL_PADDING, LABEL_SIZE};
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::{ItemSpec, ItemTemplate, Placement};

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Stock `DLOG` 1016 ([`fixture::stock_1016`]).
    fn template() -> DialogTemplate {
        fixture::stock_1016()
    }

    /// Where the dialog goes: (1024 - 441) / 2 and (768 - 317) / 2,
    /// floored.
    const ORIGIN: Point = Point::new(291.0, 225.0);

    /// Where a short text's dialog goes: [`ORIGIN`], then 203 × 0.3 down.
    const SHRUNK_ORIGIN: Point = Point::new(291.0, 285.0);

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

    fn press_item(dialog: &mut OfferDialog, number: usize, pressed: bool) {
        let at = dialog
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center();
        dialog.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }

    #[test]
    fn a_click_on_accept_holds_back_keys_until_its_release_answers() {
        let mut dialog = offer();
        assert!(!dialog.holds_back_keys(), "before the press");
        press_item(&mut dialog, ACCEPT_ITEM, true);
        assert!(dialog.holds_back_keys(), "during it");
        dialog.input(&key(Key::KeyZ));
        press_item(&mut dialog, ACCEPT_ITEM, false);
        assert!(dialog.answered());
        assert!(!dialog.holds_back_keys(), "answered");
    }

    #[test]
    fn an_offer_with_no_layout_holds_back_no_keys() {
        let dialog = OfferDialog::new(
            Err("no DLOG 1016".to_owned()),
            prompt("I'm in", None),
            ButtonStyle::STOCK,
        );
        assert!(!dialog.holds_back_keys());
    }

    #[test]
    fn venue_offers_hold_back_keys_for_their_open_dialog_and_what_its_answer_left() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert!(!offers.holds_back_keys(), "nothing open");
        opened(&mut offers);
        let at = offers
            .dialog()
            .and_then(OfferDialog::dialog)
            .and_then(|dialog| dialog.item_bounds(ACCEPT_ITEM))
            .expect("Accept")
            .center();
        let left = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        };
        offers.input(&left(true), &mut Vec::new());
        assert!(offers.holds_back_keys(), "during the press");
        offers.input(&key(Key::KeyZ), &mut Vec::new());
        offers.input(&left(false), &mut Vec::new());
        assert!(offers.dialog().is_none(), "answered");
        assert!(offers.holds_back_keys(), "Z waits for its holder");
        offers.take_choice();
        assert!(offers.holds_back_keys(), "until its holder runs it");
        assert_eq!(offers.take_unclaimed(), [key(Key::KeyZ)]);
        assert!(!offers.holds_back_keys(), "once it has");
    }

    #[test]
    fn modifiers_and_releases_left_by_an_answer_hold_back_nothing() {
        let released = Input::Key {
            key: Key::KeyZ,
            pressed: false,
            repeat: false,
        };
        let modifiers = [
            Key::AltLeft,
            Key::AltRight,
            Key::ControlLeft,
            Key::ControlRight,
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::MetaLeft,
            Key::MetaRight,
            Key::CapsLock,
        ];
        for left in modifiers.map(key).into_iter().chain([released]) {
            let mut offers = VenueOffers::asking(plain_only(Ok(template())));
            opened(&mut offers);
            let click = fixture::accept_click_holding(offers.dialog().expect("open"), left);
            for input in &click {
                offers.input(input, &mut Vec::new());
            }
            assert!(offers.dialog().is_none(), "{left:?}: answered");
            assert!(!offers.holds_back_keys(), "{left:?}");
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
        assert_eq!(laid.bounds().min, SHRUNK_ORIGIN);
        assert_eq!(
            laid.text(TEXT_ITEM).map(|text| text.lines().join(" ")),
            Some("Ferry this to Mars.".to_owned())
        );
        let commands = drawn(&dialog);
        assert!(matches!(
            commands[0],
            DrawCommand::StretchedPicture { image, top_left, .. }
                if image == FRAME.top && top_left == SHRUNK_ORIGIN
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
        let mut dialog = offer();
        dialog.input(&key(Key::NumpadEnter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    /// Types `typed` as a Mac sends it: `key`'s press, then the character
    /// it types.
    fn typed(dialog: &mut OfferDialog, key: Key, typed: char) {
        dialog.input(&Input::Key {
            key,
            pressed: true,
            repeat: false,
        });
        dialog.input(&Input::Text(typed));
    }

    /// `_SingleMissionFilter` compares the event's character, case folded,
    /// with each label's first (@0x98e4a-0x98eea).
    #[test]
    fn each_labels_first_letter_typed_either_case_chooses_it() {
        for c in ['n', 'N'] {
            let mut dialog = offer();
            typed(&mut dialog, Key::KeyN, c);
            assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse), "{c}");
        }
        let mut dialog = offer();
        typed(&mut dialog, Key::KeyX, 'x');
        assert_eq!(dialog.take_choice(), None);
        let mut cannot = dialog_of(prompt("Okay", None));
        typed(&mut cannot, Key::KeyN, 'n');
        assert_eq!(cannot.take_choice(), None, "no refuse label");
        typed(&mut cannot, Key::KeyO, 'O');
        assert_eq!(cannot.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn the_character_typed_chooses_wherever_the_layout_puts_it() {
        let mut dialog = dialog_of(prompt("Accept", Some("Refuse")));
        typed(&mut dialog, Key::KeyQ, 'a');
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Accept),
            "AZERTY's A, where a US keyboard has Q"
        );
        let mut dialog = dialog_of(prompt("Accept", Some("Refuse")));
        typed(&mut dialog, Key::KeyA, 'q');
        assert_eq!(dialog.take_choice(), None, "the US A typing Q");
    }

    #[test]
    fn a_letter_keys_press_alone_chooses_nothing() {
        let mut dialog = offer();
        dialog.input(&key(Key::KeyN));
        assert_eq!(dialog.take_choice(), None, "no character typed");
        dialog.input(&Input::Key {
            key: Key::KeyN,
            pressed: false,
            repeat: false,
        });
        dialog.input(&Input::Text('n'));
        assert_eq!(dialog.take_choice(), None, "not typed by a press");
    }

    /// The filter takes a letter on `keyDown` alone, never `autoKey`
    /// (@0x98e56).
    #[test]
    fn a_character_a_repeat_types_chooses_nothing() {
        let mut dialog = offer();
        dialog.input(&Input::Key {
            key: Key::KeyN,
            pressed: true,
            repeat: true,
        });
        dialog.input(&Input::Text('n'));
        assert_eq!(dialog.take_choice(), None);
        typed(&mut dialog, Key::KeyN, 'n');
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse), "a press");
    }

    #[test]
    fn labels_starting_with_the_same_letter_take_no_letter() {
        let mut dialog = dialog_of(prompt("Sure", Some("Scram")));
        typed(&mut dialog, Key::KeyS, 's');
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
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(scrolled(&dialog), Some(crate::ui::desc::ARROW_UNITS));
        assert!(!dialog.answered());
    }

    /// Where item `number` is inside `dialog`.
    fn local(dialog: &OfferDialog, number: usize) -> Point {
        let laid = dialog.dialog().expect("laid out");
        let at = laid.item_bounds(number).expect("an item").min;
        Point::new(at.x - laid.bounds().min.x, at.y - laid.bounds().min.y)
    }

    fn scrolled(dialog: &OfferDialog) -> Option<f32> {
        dialog
            .dialog()
            .and_then(|laid| laid.text(TEXT_ITEM))
            .map(crate::ui::ScrollText::offset)
    }

    fn long(accept: &str, refuse: Option<&str>) -> OfferPrompt {
        OfferPrompt {
            text: "line\r".repeat(40),
            ..prompt(accept, refuse)
        }
    }

    #[test]
    fn a_short_text_shrinks_1016_and_moves_it_down() {
        let mut dialog = offer();
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds(), Bounds::at(SHRUNK_ORIGIN, 441.0, 114.0));
        assert_eq!(
            laid.item_bounds(TEXT_ITEM),
            Some(rect(303.0, 294.0, 415.0, 64.0))
        );
        assert_eq!(local(&dialog, ACCEPT_ITEM), Point::new(225.0, 82.0));
        assert_eq!(local(&dialog, REFUSE_ITEM).y, 82.0);
        assert_eq!(
            local(&dialog, 9),
            Point::new(340.0, 287.0),
            "item 9 stays (@0xa2645-0xa2816)"
        );
        assert_eq!(local(&dialog, 10), Point::new(373.0, 287.0), "and item 10");
        click_item(&mut dialog, REFUSE_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Refuse));
        let mut dialog = offer();
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_shrunk_offer_that_cannot_be_refused_moves_its_one_button() {
        let mut dialog = dialog_of(prompt("Okay", None));
        assert_eq!(local(&dialog, SINGLE_ITEM).y, 82.0);
        click_item(&mut dialog, SINGLE_ITEM);
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_text_that_fills_the_box_keeps_441_by_317_and_scrolls() {
        let mut dialog = dialog_of(long("Yes", Some("No")));
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds(), Bounds::at(ORIGIN, 441.0, 317.0));
        assert_eq!(local(&dialog, ACCEPT_ITEM), Point::new(225.0, 285.0));
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(scrolled(&dialog), Some(crate::ui::desc::ARROW_UNITS));
    }

    #[test]
    fn a_shrunk_text_has_nothing_to_scroll() {
        let mut dialog = offer();
        dialog.input(&key(Key::ArrowDown));
        assert_eq!(scrolled(&dialog), Some(0.0));
    }

    #[test]
    fn the_shrink_measures_the_text_in_the_body_font_wrapped_to_its_box() {
        // 83 characters fill the 415 wide box at 5 a character.
        let line_height = MonoMetrics.line_height(BODY_FONT, BODY_SIZE);
        let wide = "x".repeat(83);
        let lines = 5;
        let text = vec![wide.as_str(); lines].join(" ");
        let dialog = dialog_of(OfferPrompt {
            text,
            ..prompt("Yes", Some("No"))
        });
        let high = lines as f32 * line_height;
        assert!(high > 48.0, "past the floor: {high}");
        let laid = dialog.dialog().expect("laid out");
        assert_eq!(laid.bounds().height(), 317.0 - (267.0 - high - 16.0));
    }

    /// A button's width for a label wider than the floor, by
    /// [`MonoMetrics`].
    fn button_width(label: &str) -> f32 {
        MonoMetrics.width(LABEL_FONT, LABEL_SIZE, label) + LABEL_PADDING
    }

    /// `prompt` in `template`.
    fn laid_in(template: DialogTemplate, prompt: OfferPrompt) -> OfferDialog {
        OfferDialog::new(
            Ok((template, Rc::new(MonoMetrics))),
            prompt,
            ButtonStyle::STOCK,
        )
    }

    /// The widths of `dialog`'s item `number`.
    fn width_of(dialog: &OfferDialog, number: usize) -> Option<f32> {
        dialog
            .dialog()
            .and_then(|laid| laid.item_bounds(number))
            .map(|item| item.width())
    }

    #[test]
    fn both_variants_widen_their_buttons_to_the_wider_label() {
        let wide = button_width("I need more time");
        let answer = |text: &str| OfferPrompt {
            text: text.to_owned(),
            ..prompt("I'm in", Some("I need more time"))
        };
        for dialog in [
            laid_in(template(), answer("Short.")),
            laid_in(template(), answer(&"line\r".repeat(40))),
            laid_in(
                picture_template(),
                OfferPrompt {
                    graphic: Some(5008),
                    ..answer("Short.")
                },
            ),
        ] {
            assert_eq!(width_of(&dialog, ACCEPT_ITEM), Some(wide));
            assert_eq!(width_of(&dialog, REFUSE_ITEM), Some(wide));
        }
    }

    #[test]
    fn a_shrunk_dialog_keeps_its_widened_buttons_edges() {
        let dialog = laid_in(template(), prompt("Yes", Some("No")));
        assert_eq!(local(&dialog, ACCEPT_ITEM), Point::new(225.0, 82.0));
        assert_eq!(width_of(&dialog, ACCEPT_ITEM), Some(100.0));
        assert_eq!(local(&dialog, REFUSE_ITEM), Point::new(116.0, 82.0));
        assert_eq!(width_of(&dialog, REFUSE_ITEM), Some(100.0));
    }

    #[test]
    fn a_one_button_offer_widens_its_button_about_its_centre() {
        let dialog = laid_in(template(), prompt("Okay", None));
        assert_eq!(local(&dialog, SINGLE_ITEM), Point::new(172.0, 82.0));
        assert_eq!(width_of(&dialog, SINGLE_ITEM), Some(100.0));
    }

    #[test]
    fn a_click_just_inside_the_widened_edge_chooses() {
        let mut dialog = laid_in(template(), prompt("Yes", Some("No")));
        let accept = dialog
            .dialog()
            .and_then(|laid| laid.item_bounds(ACCEPT_ITEM))
            .expect("an item");
        click_at(
            &mut dialog,
            Point::new(accept.min.x + 99.5, accept.center().y),
        );
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_picture_offer_never_shrinks() {
        for graphic in [5008, 128] {
            let dialog = OfferDialog::new(
                Ok((picture_template(), Rc::new(MonoMetrics))),
                pictured(graphic, Some("No")),
                ButtonStyle::STOCK,
            );
            let laid = dialog.dialog().expect("laid out");
            assert_eq!(laid.bounds(), Bounds::at(PICTURE_ORIGIN, 651.0, 244.0));
            assert_eq!(
                laid.item_bounds(TEXT_ITEM).map(|text| text.height()),
                Some(200.0),
                "{graphic}"
            );
            assert_eq!(local(&dialog, ACCEPT_ITEM), Point::new(221.0, 213.0));
        }
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

    /// `_TrackSingleMissionButtons` @0x28188 plays nothing under the pointer.
    #[test]
    fn pointer_click_is_silent_as_the_original_and_a_click_abandoned_does_nothing() {
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
        assert_eq!(dialog.take_sounds(), []);
        click_item(&mut dialog, ACCEPT_ITEM);
        assert_eq!(dialog.take_sounds(), []);
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
        assert!(offers.input(&key(Key::KeyX), &mut sounds));
        assert!(offers.input(&Input::Text('x'), &mut sounds));
        assert!(offers.dialog().is_some(), "x chooses nothing");
        assert!(offers.input(&key(Key::KeyN), &mut sounds));
        assert!(offers.input(&Input::Text('n'), &mut sounds));
        assert!(offers.dialog().is_none(), "answered, it closes");
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert_eq!(offers.take_choice(), None, "once");
        assert!(!offers.input(&key(Key::Enter), &mut sounds));
    }

    #[test]
    fn an_open_offers_pointer_and_drawing_go_through_them() {
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
        assert_eq!(sounds, []);
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
            Some(at(221.0, 213.0, button_width("No thanks"), 25.0))
        );
        assert_eq!(
            laid.item_bounds(REFUSE_ITEM),
            Some(at(
                214.0 - button_width("No thanks"),
                213.0,
                button_width("No thanks"),
                25.0
            ))
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
        assert_eq!(shown.bounds().min, SHRUNK_ORIGIN);
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
        typed(&mut dialog, i, 'i');
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

    /// `_OfferOneMission`'s I arm counts the active missions that aren't
    /// hidden and, with none, does nothing (@0xa32c1-0xa32ee): I is still
    /// read before the labels' letters, so it doesn't choose I'm in.
    #[test]
    fn with_no_mission_listed_the_mission_info_key_asks_and_chooses_nothing() {
        let i = crate::ui::mission_info::MISSION_INFO_KEY;
        let mut dialog = offer().with_missions_listed(false);
        typed(&mut dialog, i, 'i');
        assert!(!dialog.take_info_request());
        assert_eq!(dialog.take_choice(), None);
        dialog.set_missions_listed(true);
        dialog.input(&key(i));
        assert!(dialog.take_info_request(), "listed again after an abort");
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        opened(&mut offers);
        offers.set_missions_listed(false);
        offers.input(&key(i), &mut Vec::new());
        assert!(!offers.take_info_request(), "told through the venue");
        assert_eq!(offers.take_choice(), None);
    }

    #[test]
    fn the_map_key_asks_for_the_map_before_the_labels_letters_on_a_press() {
        let m = crate::flight::view::MAP_KEY;
        let mut dialog = dialog_of(prompt("Move it", Some("No thanks")));
        for (input, what) in [
            (
                Input::Key {
                    key: m,
                    pressed: true,
                    repeat: true,
                },
                "a repeat",
            ),
            (
                Input::Key {
                    key: m,
                    pressed: false,
                    repeat: false,
                },
                "a release",
            ),
        ] {
            dialog.input(&input);
            assert!(!dialog.take_map_request(), "{what}");
        }
        typed(&mut dialog, m, 'm');
        assert!(dialog.take_map_request());
        assert!(!dialog.take_map_request(), "once");
        assert_eq!(dialog.take_choice(), None, "not Move it's letter");
        assert!(!dialog.answered());
        assert_eq!(dialog.take_sounds(), [], "silent");
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        assert_eq!(offers.take_map_request(), None, "nothing open");
        opened(&mut offers);
        let mut sounds = Vec::new();
        offers.input(&key(m), &mut sounds);
        assert_eq!(offers.take_map_request(), Some(MissionId(141)));
        assert_eq!(offers.take_map_request(), None, "once");
        assert!(offers.dialog().is_some(), "still open under the map");
        assert_eq!(sounds, []);
    }

    #[test]
    fn an_answered_offer_asks_for_no_map() {
        let mut dialog = offer();
        dialog.input(&key(Key::Enter));
        assert!(dialog.answered());
        dialog.input(&key(crate::flight::view::MAP_KEY));
        assert!(!dialog.take_map_request());
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
        dialog.tick(Duration::ZERO);
        assert_eq!(player.calls()[0], Call::Start("Offer.mov".to_owned()));
        assert!(
            texts(&dialog).is_empty(),
            "no dialog drawn: {:?}",
            texts(&dialog)
        );
        assert_eq!(player.drawn().len(), 1, "the movie is");
        dialog.input(&key(Key::KeyN));
        dialog.input(&Input::Text('n'));
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
        dialog.input(&key(Key::KeyN));
        dialog.input(&Input::Text('n'));
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
        dialog.tick(Duration::ZERO);
        assert!(
            texts(&dialog).contains(&"Ferry this to Mars.".to_owned()),
            "on its first tick, as the movie begins"
        );
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
        let mut dialog = offer();
        dialog.input(&key(Key::NumpadEnter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_cinematic_offer_movie_hides_the_pointer_and_requests_nothing() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0x0004)).with_movies(&movies(&player));
        dialog.tick(MOVIE_FADE_FOR);
        assert!(dialog.hides_pointer());
        dialog.input(&key(Key::KeyP));
        dialog.input(&key(Key::KeyI));
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
        assert!(player.calls().is_empty(), "staged");
        offers.tick(TICK, false);
        assert_eq!(player.calls()[0], Call::Start("Offer.mov".to_owned()));
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
        offers.input(&key(Key::KeyN), &mut sounds);
        offers.input(&Input::Text('n'), &mut sounds);
        assert_eq!(offers.take_choice(), None, "not while the movie plays");
        offers.tick(TICK, false);
        assert_eq!(
            offers.take_choice(),
            Some((MissionId(141), OfferChoice::Refuse)),
            "on the tick the movie ends"
        );
        assert!(offers.dialog().is_none(), "and the dialog closes");
    }

    #[test]
    fn a_before_movie_begins_on_the_dialogs_first_tick_or_input() {
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&player));
        assert!(player.calls().is_empty(), "staged, not begun");
        assert!(dialog.playing(), "the dialog waits for it");
        assert!(drawn(&dialog).is_empty(), "drawing nothing");
        dialog.tick(Duration::ZERO);
        assert_eq!(player.calls()[0], Call::Start("Offer.mov".to_owned()));
        let player = MockMoviePlayer::playing((160, 120), 2);
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&player));
        dialog.input(&key(Key::KeyN));
        dialog.input(&Input::Text('n'));
        assert_eq!(player.calls(), [Call::Start("Offer.mov".to_owned())]);
        dialog.tick(TICK);
        dialog.tick(TICK);
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Refuse),
            "the input that began it is kept for the dialog"
        );
    }

    #[test]
    fn a_before_movie_that_cannot_start_gives_its_first_input_to_the_dialog() {
        let mut dialog = dialog_of(filmed(0)).with_movies(&movies(&MockMoviePlayer::refusing()));
        dialog.input(&key(Key::Enter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
        let mut dialog = offer();
        dialog.input(&key(Key::NumpadEnter));
        assert_eq!(dialog.take_choice(), Some(OfferChoice::Accept));
    }

    #[test]
    fn a_covered_venue_offer_neither_ticks_nor_begins_its_movie() {
        let player = MockMoviePlayer::playing((160, 120), 1);
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        offers.set_movies(movies(&player));
        offers.open(filmed(0), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        offers.tick(TICK, true);
        assert!(
            player.calls().is_empty(),
            "a mission text over it plays alone: `_PlayQTMovie` is modal"
        );
        offers.tick(TICK, false);
        assert_eq!(player.calls()[0], Call::Start("Offer.mov".to_owned()));
    }

    /// Offers whose dialog was accepted by a click on its accept button,
    /// an Escape pressed while the click was held, the choice not taken.
    fn accepted_with_an_escape_left() -> VenueOffers {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        opened(&mut offers);
        let click = fixture::accept_click_holding(offers.dialog().expect("open"), key(Key::Escape));
        for input in &click {
            offers.input(input, &mut Vec::new());
        }
        assert!(offers.dialog().is_none(), "accepted");
        offers
    }

    /// The answer's `FlushEvents` flush only mouse events (@0xa2da2-0xa2dde),
    /// so the Escape goes to the venue once the answer has been acted on.
    #[test]
    fn what_the_answer_left_goes_to_the_venue_once_its_choice_is_taken() {
        let mut offers = accepted_with_an_escape_left();
        assert_eq!(
            offers.take_unclaimed(),
            [],
            "not before the choice is taken"
        );
        offers.tick(TICK, true);
        assert!(offers.take_choice().is_some());
        assert_eq!(
            offers.take_unclaimed(),
            [key(Key::Escape)],
            "a cover before the answer was acted on drops nothing"
        );
        assert_eq!(offers.take_unclaimed(), [], "once");
    }

    /// A text the answer shows flushes the queue as it opens (@0x5ae67).
    #[test]
    fn a_tick_covered_once_the_choice_is_taken_drops_what_the_answer_left() {
        let mut offers = accepted_with_an_escape_left();
        assert!(offers.take_choice().is_some());
        offers.tick(TICK, true);
        assert_eq!(offers.take_unclaimed(), []);
        let mut offers = accepted_with_an_escape_left();
        assert!(offers.take_choice().is_some());
        offers.tick(TICK, false);
        assert_eq!(
            offers.take_unclaimed(),
            [key(Key::Escape)],
            "an uncovered tick keeps it"
        );
    }

    /// Answers `offers`' open dialog with `key`, taking the choice.
    fn answer(offers: &mut VenueOffers, key_pressed: Key) -> Option<(MissionId, OfferChoice)> {
        offers.input(&key(key_pressed), &mut Vec::new());
        if let Some(typed) = crate::input::key_letter(key_pressed) {
            offers.input(&Input::Text(typed), &mut Vec::new());
        }
        offers.take_choice()
    }

    fn drawn_offers(offers: &VenueOffers) -> Vec<String> {
        let mut list = DrawList::new();
        offers.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_refused_offer_stays_drawn_and_inert_until_released() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        opened(&mut offers);
        assert_eq!(
            answer(&mut offers, Key::KeyN),
            Some((MissionId(141), OfferChoice::Refuse))
        );
        assert!(offers.dialog().is_none(), "answered");
        assert!(
            offers.answered().is_some(),
            "but not hidden (@0xa2e70-0xa2e78)"
        );
        assert!(drawn_offers(&offers).contains(&"Ferry this to Mars.".to_owned()));
        offers.input(&key(Key::Enter), &mut Vec::new());
        offers.tick(TICK, false);
        assert_eq!(offers.take_choice(), None, "inert");
        offers.release_answered(false);
        assert!(drawn_offers(&offers).is_empty());
    }

    #[test]
    fn an_accept_hides_the_two_button_dialog_and_not_the_one_button_one() {
        let mut offers = VenueOffers::asking(plain_only(Ok(template())));
        opened(&mut offers);
        assert!(answer(&mut offers, Key::Enter).is_some());
        assert!(drawn_offers(&offers).is_empty(), "hidden (@0xa2df3)");
        offers.open(prompt("OK", None), Rc::new(MonoMetrics), ButtonStyle::STOCK);
        assert!(answer(&mut offers, Key::Enter).is_some());
        assert!(
            offers.answered().is_some(),
            "the one-button dialog's accept hides nothing (@0xa2f52-0xa2ff7)"
        );
    }

    /// The original's `_SingleMissionFilter` (@0x98d9a) maps Return, Escape
    /// and the labels' first letters to items without a click sound.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [
            Key::Enter,
            Key::Escape,
            crate::ui::player_info::PLAYER_INFO_KEY,
        ]
        .into_iter()
        .chain(('a'..='z').filter_map(crate::input::letter_key))
        {
            let mut dialog = offer();
            for pressed in [true, false] {
                dialog.input(&Input::Key {
                    key: k,
                    pressed,
                    repeat: false,
                });
            }
            assert_eq!(dialog.take_sounds(), [], "{k:?}");
        }
        for c in ['n', 'N', 'i'] {
            let mut dialog = offer();
            typed(&mut dialog, Key::KeyN, c);
            assert_eq!(dialog.take_sounds(), [], "{c}");
        }
    }

    #[test]
    fn an_offer_takes_tab_then_space_only_with_the_focus_ring() {
        let mut ringed = VenueOffers::asking(plain_only(Ok(template())));
        opened(&mut ringed);
        ringed.input(&key(Key::Tab), &mut Vec::new());
        ringed.input(&key(Key::Space), &mut Vec::new());
        assert!(ringed.take_choice().is_some(), "by default");
        let mut off =
            VenueOffers::asking(plain_only(Ok(template()))).with_focus_ring(FocusRing::Off);
        opened(&mut off);
        off.input(&key(Key::Tab), &mut Vec::new());
        off.input(&key(Key::Space), &mut Vec::new());
        assert_eq!(off.take_choice(), None);
        assert!(off.dialog().is_some(), "still open");
    }
}
