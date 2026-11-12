//! The mission screens as the player sees them: the mission computer
//! ([`MissionComputer`], [`ComputerRow`], [`ComputerRefusal`]), the
//! mission texts and offer movies the session queues ([`MissionShown`],
//! [`MissionText`], [`MissionTextKind`], [`TextBar`]) and the mission
//! info screen ([`MissionInfo`], [`InfoRow`], [`MissionInfoPress`]), from
//! `nova_sim::session::mission_screens`, which re-exports them and keeps
//! the session code that builds them.

use nova_sim_base::reserves::Reserves;
use nova_sim_catalog::catalog::{DescMovie, MissionId, MissionTexts};
use nova_sim_state::shown::mission::active::AcceptRefusal;
use nova_sim_state::shown::mission::screens::NO_MISSIONS_HERE;
use nova_sim_state::shown::navigation::NavMode;

use crate::flight_frame::StoryRow;
use crate::galaxy_map::MapMarks;

/// One offer the mission computer lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputerRow {
    /// The mission.
    pub mission: MissionId,
    /// Its name, its tags filled for the offer: the list's.
    pub name: String,
    /// Its name as the `mïsn` stores it, its tags unfilled: the offer's
    /// name above its text is cut to its box from this, and filled after
    /// (`_MissionDialogUpdate` @0x9f37d-0x9f40c), so the view fills the
    /// cut name through `nova_sim::Session::offer_text`.
    pub raw_name: String,
    /// Its offer text, its tags filled for the offer: empty with none.
    pub text: String,
}

/// The mission computer, as the player sees it (`DLOG` 1006).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionComputer {
    /// The offers, in the order they show.
    pub rows: Vec<ComputerRow>,
    /// Today, as the medium date.
    pub today: String,
}

/// Why the mission computer does not open (`_DoMissionDialog`
/// @0xa37e9-0xa3956).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComputerRefusal {
    /// Every mission slot is taken.
    SlotsFull,
    /// No offer it lists is available now, or the ship is not docked.
    NoneAvailable,
}

impl ComputerRefusal {
    /// What the original says: `STR#` 2002 #351/#352 with every slot
    /// taken, #353 with nothing available.
    #[must_use]
    pub fn message(self) -> String {
        match self {
            Self::SlotsFull => AcceptRefusal::SlotsFull.message(),
            Self::NoneAvailable => NO_MISSIONS_HERE.to_owned(),
        }
    }
}

/// Which of a mission's texts is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionTextKind {
    /// Its `BriefText`, as it is accepted.
    Brief,
    /// Its `CompText`, as it is completed.
    Comp,
    /// Its `FailText`, as it fails at its return.
    Fail,
    /// Its `RefuseText`, as its offer is refused.
    Refuse,
    /// Why it has no room for its cargo: `STR#` 2002 #355 or #356 as its
    /// accept is refused.
    NoRoom,
    /// Its `LoadCargText`, as its cargo is loaded.
    LoadCargo,
    /// Its `DumpCargoText`, as its cargo is unloaded.
    DumpCargo,
    /// Its `ShipDoneText`, as its ship goals are met.
    ShipDone,
}

impl MissionTextKind {
    /// This text of `texts`, as written, if it has one. `pub` for
    /// `nova_sim::session`'s mission texts, which read it.
    #[must_use]
    pub fn of(self, texts: &MissionTexts) -> Option<&str> {
        match self {
            Self::Brief => texts.brief.as_deref(),
            Self::Comp => texts.comp.as_deref(),
            Self::Fail => texts.fail.as_deref(),
            Self::Refuse => texts.refuse.as_deref(),
            Self::LoadCargo => texts.load_cargo.as_deref(),
            Self::DumpCargo => texts.dump_cargo.as_deref(),
            Self::ShipDone => texts.ship_done.as_deref(),
            Self::NoRoom => None,
        }
    }

    /// The `Graphic` of this text's `dësc` in `texts`, raw. Every mission
    /// `dësc` text allows its picture (`_ShowTextDialog`'s third argument
    /// 1, as `_DoMissionAccept` @0xa1f0e and `_DoMissionSuccess` @0xa048c
    /// pass it); the no-room text is `STR#` 2002's, passed 0 (@0xa1bb2,
    /// @0x982ff), so it has none. `pub` for `nova_sim::session`'s mission
    /// texts, which read it.
    #[must_use]
    pub fn graphic_of(self, texts: &MissionTexts) -> Option<i16> {
        let graphics = &texts.graphics;
        match self {
            Self::Brief => graphics.brief,
            Self::Comp => graphics.comp,
            Self::Fail => graphics.fail,
            Self::Refuse => graphics.refuse,
            Self::LoadCargo => graphics.load_cargo,
            Self::DumpCargo => graphics.dump_cargo,
            Self::ShipDone => graphics.ship_done,
            Self::NoRoom => None,
        }
    }

    /// The movie of this text's `dësc` in `texts`. The no-room text is
    /// `STR#` 2002's and loads no `dësc` (@0xa1b83-0xa1bb2), so it has none
    /// of its own: it plays the last one loaded's after-movie instead
    /// (`nova_sim::Session::loaded_after_movie`). `pub` for `nova_sim::session`'s
    /// mission texts, which read it.
    #[must_use]
    pub fn movie_of(self, texts: &MissionTexts) -> Option<DescMovie> {
        let movies = &texts.movies;
        match self {
            Self::Brief => movies.brief.clone(),
            Self::Comp => movies.comp.clone(),
            Self::Fail => movies.fail.clone(),
            Self::Refuse => movies.refuse.clone(),
            Self::LoadCargo => movies.load_cargo.clone(),
            Self::DumpCargo => movies.dump_cargo.clone(),
            Self::ShipDone => movies.ship_done.clone(),
            Self::NoRoom => None,
        }
    }
}

/// A mission text to show in the "Desc Dialog" (`_ShowTextDialog`
/// @0x5a752), its tags filled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionText {
    /// Which text it is.
    pub kind: MissionTextKind,
    /// The mission.
    pub mission: MissionId,
    /// The text.
    pub text: String,
    /// Its `dësc`'s `Graphic`, raw: none for -1, or for a text that is no
    /// `dësc`'s. The view shows the picture variant of the dialog
    /// (`DLOG` 3004) for one above 127 (`_ShowTextDialog` @0x5a89a).
    pub graphic: Option<i16>,
    /// The movie its `dësc` names, played before or after its dialog
    /// (`_ShowTextDialog` @0x5a76e-0x5a7ab, @0x5b40b-0x5b44f). A text that
    /// is no `dësc`'s, a `STR#` text, carries the last `dësc` loaded's
    /// after-movie instead, or none (`nova_sim::Session::loaded_after_movie`).
    pub movie: Option<DescMovie>,
    /// The marks of the map the map key opens from it, or `None` when it
    /// opens none. Only the accept's `BriefText` opens one
    /// (`_ShowTextDialog(1, 0, 1)` @0xa1f0e, whose first argument allows
    /// item 4, @0x5b135-0x5b146), marked as `_DoMissionAccept` marks it
    /// just before (@0xa1e8d-0xa1eda; see the `mission_map` module);
    /// `_ShowTextDialog`'s other 25 calls pass 0.
    pub map: Option<MapMarks>,
}

/// The status bar as a mission text noted it as it was queued: what the
/// whole-bar redraw its caller makes as it closes draws (`_RefreshSAT`
/// right after `_ShowTextDialog`, e.g. @0xa1f13, @0xa0491, @0xa03d2,
/// @0xa2f09), as the text is modal and nothing runs under it. Noted for
/// each text, not for an offer's movie (`nova_sim::Session::text_bars`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBar {
    /// Its place among the texts: `nova_sim::Session::texts_queued` once it was
    /// queued. A [`InterfaceLoad`](crate::ship_change::InterfaceLoad) whose
    /// [`texts`](crate::ship_change::InterfaceLoad::texts) is below it ran before the
    /// text was queued.
    pub texts: u64,
    /// The nav mode and selection as it was queued.
    pub nav: NavMode,
    /// The shield, armour and fuel as it was queued.
    pub reserves: Reserves,
}

/// One thing the session queues to show, in order
/// (`nova_sim::Session::take_mission_texts`): a mission text, or an offer's movie,
/// which `_OfferOneMission` plays outside `_ShowTextDialog`, so in its own
/// place among the texts: the before-movie ahead of the `BriefText` of an
/// offer accepted at once (@0xa2273-0xa22ab), and the after-movie behind
/// the texts the offer dialog's answer shows (@0xa3580-0xa35b8), which is
/// whatever the last `dësc` loaded names
/// ([`RuleKey::DescMovieGlobals`](nova_sim_base::RuleKey::DescMovieGlobals)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionShown {
    /// A text, with its own `dësc`'s movie, or a `STR#` text's stale one.
    Text(MissionText),
    /// An offer's movie, played whatever its
    /// [`after`](DescMovie::after) flag says: the session has placed it.
    Movie(DescMovie),
}

impl From<MissionText> for MissionShown {
    fn from(text: MissionText) -> Self {
        Self::Text(text)
    }
}

/// One active mission the mission info screen lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoRow {
    /// Its slot.
    pub slot: u8,
    /// Its name, its tags filled for the slot.
    pub name: String,
    /// Whether it has failed: the screen marks it.
    pub failed: bool,
    /// Its `QuickBrief`, its tags filled for the slot: empty with none.
    pub text: String,
    /// Whether the player can abort it: its `CanAbort` is not 0.
    pub abortable: bool,
}

/// The mission info screen, as the player sees it (`DLOG` 1012). The
/// default lists nothing and names no day: what a flight with nothing to
/// list shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MissionInfo {
    /// The active missions shown, in slot order.
    pub rows: Vec<InfoRow>,
    /// Today, as the medium date.
    pub today: String,
    /// What each row's place picks, when it isn't `rows` itself: after an
    /// abort under the engine reading of `mission_info_rebuild`
    /// (`nova_sim::Session::mission_info_after_abort`), every active slot in slot
    /// order, hidden ones included, so row *i* picks the *i*th of these
    /// and a step can pick past the rows. `None` picks by `rows`.
    pub table: Option<Vec<InfoRow>>,
    /// The new game's active storyline missions, by catalog id
    /// (`Statics::story_rows`): the router shows them in its own missions
    /// log rather than on `DLOG` 1012. Always empty in stock, whose data
    /// holds no storylines.
    pub story: Vec<StoryRow>,
}

impl MissionInfo {
    /// The screen listing `rows` on `today`, each row picking its own
    /// mission.
    #[must_use]
    pub fn shown(rows: Vec<InfoRow>, today: impl Into<String>) -> Self {
        Self {
            rows,
            today: today.into(),
            table: None,
            story: Vec::new(),
        }
    }

    /// This screen listing the storyline missions `story` too
    /// ([`MissionInfo::story`]).
    #[must_use]
    pub fn with_story(self, story: Vec<StoryRow>) -> Self {
        Self { story, ..self }
    }

    /// Whether it lists nothing: no stock row and no storyline row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.story.is_empty()
    }

    /// This screen with each row's place picking from `table` instead
    /// ([`MissionInfo::table`]).
    #[must_use]
    pub fn with_table(self, table: Vec<InfoRow>) -> Self {
        Self {
            table: Some(table),
            ..self
        }
    }

    /// What each place picks: the table, if any, otherwise the rows.
    #[must_use]
    pub fn picks(&self) -> &[InfoRow] {
        self.table.as_deref().unwrap_or(&self.rows)
    }
}

/// What a press of the in-flight mission info key does
/// (`nova_sim::Session::press_mission_info`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionInfoPress {
    /// Refused, in silence: nothing opens and nothing is said.
    Refused,
    /// Taken, with no mission to show: the refusal beep has sounded, and
    /// the view says "You have no active missions." (`STR#` 2002 #354).
    NoActiveMissions,
    /// Taken: the mission info screen opens on this.
    Opens(MissionInfo),
    /// Taken, with a mission to show, and refused in silence as the
    /// screen is faded (`nova_sim::Session::press_faded_mission_info`).
    Faded,
}

#[cfg(test)]
mod tests {
    use nova_sim_catalog::catalog::{TextGraphics, TextMovies};

    use super::*;

    fn film(file: &str, flags: u16) -> Option<DescMovie> {
        DescMovie::from_desc(file, flags)
    }

    #[test]
    fn ship_done_picks_the_ship_done_text() {
        let texts = MissionTexts {
            ship_done: Some("Done.".to_owned()),
            comp: Some("Comp.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        };
        assert_eq!(MissionTextKind::ShipDone.of(&texts), Some("Done."));
        assert_eq!(
            MissionTextKind::ShipDone.of(&MissionTexts::named(MissionId(130), "Run")),
            None
        );
    }

    #[test]
    fn each_kind_reads_its_own_texts_graphic_and_no_room_none() {
        let texts = MissionTexts {
            graphics: TextGraphics {
                offer: Some(1),
                brief: Some(2),
                comp: Some(3),
                fail: Some(4),
                refuse: Some(5),
                load_cargo: Some(6),
                dump_cargo: Some(7),
                ship_done: Some(8),
            },
            ..MissionTexts::named(MissionId(130), "Run")
        };
        let graphics = [
            MissionTextKind::Brief,
            MissionTextKind::Comp,
            MissionTextKind::Fail,
            MissionTextKind::Refuse,
            MissionTextKind::LoadCargo,
            MissionTextKind::DumpCargo,
            MissionTextKind::ShipDone,
            MissionTextKind::NoRoom,
        ]
        .map(|kind| kind.graphic_of(&texts));
        assert_eq!(
            graphics,
            [
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8),
                None
            ],
            "the STR# 2002 no-room text shows no picture (_ShowTextDialog's third argument 0, @0xa1bb2)"
        );
    }

    #[test]
    fn each_kind_reads_its_own_texts_movie_and_no_room_none() {
        let texts = MissionTexts {
            movies: TextMovies {
                offer: film("0", 0),
                brief: film("1", 0),
                comp: film("2", 0),
                fail: film("3", 0),
                refuse: film("4", 0),
                load_cargo: film("5", 0),
                dump_cargo: film("6", 0),
                ship_done: film("7", 0),
                quick_brief: film("8", 0),
            },
            ..MissionTexts::named(MissionId(130), "Run")
        };
        let movies = [
            MissionTextKind::Brief,
            MissionTextKind::Comp,
            MissionTextKind::Fail,
            MissionTextKind::Refuse,
            MissionTextKind::LoadCargo,
            MissionTextKind::DumpCargo,
            MissionTextKind::ShipDone,
            MissionTextKind::NoRoom,
        ]
        .map(|kind| kind.movie_of(&texts).map(|movie| movie.file));
        assert_eq!(
            movies,
            [
                Some("1"),
                Some("2"),
                Some("3"),
                Some("4"),
                Some("5"),
                Some("6"),
                Some("7"),
                None
            ]
            .map(|file| file.map(str::to_owned)),
            "the STR# 2002 no-room text loads no dësc"
        );
    }

    #[test]
    fn each_refusal_says_what_the_original_says() {
        assert_eq!(
            ComputerRefusal::SlotsFull.message(),
            AcceptRefusal::SlotsFull.message()
        );
        assert_eq!(ComputerRefusal::NoneAvailable.message(), NO_MISSIONS_HERE);
    }
}
