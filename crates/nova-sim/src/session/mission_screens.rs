//! The session's side of the mission screens (see the
//! [`mission`](crate::mission) module's "Screens"), each a plain value the
//! view draws, as [`HireList`](crate::HireList) is the bar's hire screen:
//!
//! - what each mission's screens show of it, read as the session starts
//!   ([`Session::mission_texts`]);
//! - the mission computer's view of the visit's offers
//!   ([`Session::mission_computer`], `_DoMissionDialog` @0xa37cf), or why
//!   it does not open;
//! - a venue's next offer outside the computer, at the bar, spaceport,
//!   trade center, shipyard or outfitter ([`Session::next_offer`],
//!   `_OfferMissionFromPort` @0xa369e), which enters the venue, with the
//!   offers suppressed after a refused accept until another venue is
//!   entered ([`Session::enter_offer_venue`]);
//! - the mission info screen's view of the active missions
//!   ([`Session::mission_info`], `_DoMissionInfoDialog` @0x9e19b);
//! - the queue of mission texts to show ([`Session::take_mission_texts`]):
//!   a mission's `BriefText`, `CompText`, `FailText` and `RefuseText`,
//!   queued at the lifecycle's moments (see the `missions` module), its
//!   `ShipDoneText` as its ship goals are met (see the `mission_ships`
//!   module), and an accept refused for want of room for its cargo; and,
//!   in their places among them, an offer's movies ([`MissionShown`]).
//!   The original keeps the last `dësc` loaded's movie in globals that
//!   every movie check reads ([`RuleKey::DescMovieGlobals`]): the session
//!   keeps it as each load is made ([`Session::load_description`]), so a
//!   `STR#` text plays that `dësc`'s after-movie, and the offer dialog's
//!   after-check reads what its answer left.
//!   Each text notes the status bar as it is queued ([`TextBar`],
//!   [`Session::text_bars`]), and each docked `C`, `E` or `H` how many
//!   texts came before it ([`InterfaceLoad::texts`](super::InterfaceLoad::texts)),
//!   so the view can redraw the bar as each text's caller does as the
//!   text closes.
//!
//! None of it is saved. The next outside offer the original drops after
//! an accept is rdm:phase/missions/phase-15-accept-drops-next-offer.

#[cfg(test)]
use super::missions::TextSource;
use super::{Chart, MissionRun, Outbox, Session, Setup, Statics};
use crate::catalog::{DescMovie, MissionId, MissionTexts};
use crate::chance::{Chance, NeverFires};
use crate::galaxy_map::MapMarks;
use crate::mission::screens::{HIDDEN, NO_MISSIONS_HERE, SPACEPORT, UNREFUSABLE};
use crate::mission::{
    AcceptRefusal, AvailFacts, Offer, OfferPrompt, Venue, VenueOffer, availability, offer_labels,
};
use crate::navigation::NavMode;
use crate::pilot::Pilot;
use crate::player_fate::PlayerCommand;
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource};
use crate::sound::SimSound;

/// One offer the mission computer lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComputerRow {
    /// The mission.
    pub mission: MissionId,
    /// Its name, its tags filled for the offer.
    pub name: String,
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
    /// This text of `texts`, as written, if it has one.
    fn of(self, texts: &MissionTexts) -> Option<&str> {
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
    /// @0x982ff), so it has none.
    fn graphic_of(self, texts: &MissionTexts) -> Option<i16> {
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
    /// ([`Session::loaded_after_movie`]).
    fn movie_of(self, texts: &MissionTexts) -> Option<DescMovie> {
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
    /// after-movie instead, or none ([`Session::loaded_after_movie`]).
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
/// each text, not for an offer's movie ([`Session::text_bars`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextBar {
    /// Its place among the texts: [`Session::texts_queued`] once it was
    /// queued. A [`InterfaceLoad`](super::InterfaceLoad) whose
    /// [`texts`](super::InterfaceLoad::texts) is below it ran before the
    /// text was queued.
    pub texts: u64,
    /// The nav mode and selection as it was queued.
    pub nav: NavMode,
    /// The shield, armour and fuel as it was queued.
    pub reserves: Reserves,
}

/// One thing the session queues to show, in order
/// ([`Session::take_mission_texts`]): a mission text, or an offer's movie,
/// which `_OfferOneMission` plays outside `_ShowTextDialog`, so in its own
/// place among the texts: the before-movie ahead of the `BriefText` of an
/// offer accepted at once (@0xa2273-0xa22ab), and the after-movie behind
/// the texts the offer dialog's answer shows (@0xa3580-0xa35b8), which is
/// whatever the last `dësc` loaded names
/// ([`RuleKey::DescMovieGlobals`](crate::RuleKey::DescMovieGlobals)).
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

/// The mission info screen, as the player sees it (`DLOG` 1012).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionInfo {
    /// The active missions shown, in slot order.
    pub rows: Vec<InfoRow>,
    /// Today, as the medium date.
    pub today: String,
}

/// What a press of the in-flight mission info key does
/// ([`Session::press_mission_info`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissionInfoPress {
    /// Refused, in silence: nothing opens and nothing is said.
    Refused,
    /// Taken, with no mission to show: the refusal beep has sounded, and
    /// the view says "You have no active missions." (`STR#` 2002 #354).
    NoActiveMissions,
    /// Taken: the mission info screen opens on this.
    Opens(MissionInfo),
}

impl Session {
    /// The mission info key, `missionInfoKey` (slot 0x50), is pressed in
    /// flight (`_HandlePlayer` @0x70405-0x705b4). It is refused, in
    /// silence (each refusal only clears the key's latch, @0x705af):
    ///
    /// 1. during a jump, the pre-jump included: +0x50 above 0
    ///    (@0x70424-0x70428);
    /// 2. while the pod countdown +0xc908 is above 0 (@0x7042e-0x70436),
    ///    which can't be reached, as the pod's frames return (@0x68ddd)
    ///    before any key handler: the
    ///    [`PlayerFate::Ejected`](crate::PlayerFate::Ejected) row refuses
    ///    it;
    /// 3. while `_IsDying` (-0x2ca, @0x70439-0x70445) and
    /// 4. while the death countdown +0x3c is above 0 (@0x7044b-0x7044f):
    ///    both the [`PlayerFate::BreakingUp`](crate::PlayerFate::BreakingUp)
    ///    row;
    /// 5. for [`TAKEOFF_GRACE_TICKS`](crate::TAKEOFF_GRACE_TICKS) after a
    ///    take-off or a gate's exit, `_frameCounter` below 0 (`js`
    ///    @0x7045e; see the `takeoff_grace` module).
    ///
    /// An exploded player's key is refused too: the dead branch
    /// (0x6870d-0x6888c) runs no key handler. A disabled ship is taken
    /// (none of the tests reads `_IsDisabled`). Taken, it counts the
    /// active missions the screen shows (@0x7047d-0x704ba, those without
    /// `Flags` 0x0400): with one, the screen opens
    /// ([`Session::mission_info`]); with none, it beeps `_beepSnd[3]`
    /// ([`SimSound::Refused`], `PlaySound` @0x70557) before "No active
    /// missions.", `STR#` 2002 #354
    /// ([`SimMessage::NoActiveMissions`](crate::SimMessage::NoActiveMissions),
    /// @0x7055c-0x705a8).
    pub fn press_mission_info(&mut self) -> MissionInfoPress {
        if self.mode.jump().is_some() || !self.takes(PlayerCommand::MissionInfo) {
            return MissionInfoPress::Refused;
        }
        let info = self.mission_info();
        if info.rows.is_empty() {
            self.out.sounds.push(SimSound::Refused);
            self.space
                .say(&mut self.out, crate::SimMessage::NoActiveMissions);
            return MissionInfoPress::NoActiveMissions;
        }
        MissionInfoPress::Opens(info)
    }

    /// What the screens show of mission `id`, as the session read it as
    /// it started: `None` for a `mïsn` it has no texts of.
    #[must_use]
    pub fn mission_texts(&self, id: MissionId) -> Option<&MissionTexts> {
        self.statics.mission_texts(id)
    }

    /// The mission texts and offer movies to show since they were last
    /// taken, in the order they came. Taking them lets go of the bars
    /// noted with them ([`Session::text_bars`]).
    pub fn take_mission_texts(&mut self) -> Vec<MissionShown> {
        self.out.take_mission_texts()
    }

    /// Queues `mission`'s offer movie to show next, when its `dësc` names
    /// one played `after` the dialog or not, as asked.
    fn show_offer_movie(&mut self, mission: MissionId, after: bool) {
        if let Some(movie) = self
            .statics
            .offer_movie(mission)
            .filter(|movie| movie.after == after)
        {
            self.out.shown_texts.push(MissionShown::Movie(movie));
        }
    }

    /// Loads `mission`'s offer `dësc` again, drawing on `chance`, as
    /// `_OfferOneMission` does once the mission info screen opened over the
    /// offer dialog closes (@0xa335c): the movie and the governments it
    /// leaves are the offer's again. The text is drawn already, so it is
    /// not filled.
    pub fn reload_offer_desc(&mut self, mission: MissionId, chance: &mut dyn Chance) {
        let texts = self.mission_texts(mission);
        let raw = texts.and_then(|texts| texts.offer.clone());
        let movie = texts.and_then(|texts| texts.movies.offer.clone());
        let _ = self.load_description(raw.as_deref(), movie.as_ref(), chance);
    }

    /// Whether `mission`'s offer on the visit under way is answered in the
    /// offer dialog with a refuse button: an outside offer that can be
    /// refused (`Flags` 0x0004 clear; `_OfferOneMission`'s `0xdef8c`,
    /// @0xa21e2-0xa21f7). That dialog hides and redraws the whole bar
    /// before its accept (@0xa2df8) and before a refusal's `RefuseText`
    /// (@0xa2e93). The one-button dialog redraws before neither
    /// (@0xa2f52-0xa2fba), and the mission computer has no such redraw
    /// (@0xa3b37-0xa3bb4). False for a mission not offered, and in flight,
    /// where a ship's offer is answered over flight.
    #[must_use]
    pub fn refusable_offer_dialog(&self, mission: MissionId) -> bool {
        self.mode.landed().is_some()
            && self
                .run
                .offer_of(self.mode.landed(), mission)
                .is_some_and(|offer| offer.venue != Venue::Computer)
            && self
                .statics
                .mission_record(mission)
                .is_some_and(|record| record.flags & UNREFUSABLE == 0)
    }

    /// How many mission texts have been queued since the session started,
    /// never reset: a view that notes it at each of its redraws can tell
    /// whether the last text was queued after its last redraw.
    #[must_use]
    pub fn texts_queued(&self) -> u64 {
        self.out.texts_queued()
    }

    /// The status bar as each mission text since they were last taken
    /// was queued, in order, one for each text ([`TextBar`]): what each
    /// text's caller redraws the whole bar with as it closes, where a
    /// script run after it, such as an `OnSuccess` after the `CompText`
    /// (@0xa048c, @0xa0491, then @0xa04f8), has written the bar since.
    /// Taking the texts lets go of them.
    #[must_use]
    pub fn text_bars(&self) -> &[TextBar] {
        self.out.text_bars()
    }

    /// The nav mode ([`Session::nav_mode`]) as the last text since they
    /// were last taken was queued, `None` with none: the last of
    /// [`Session::text_bars`]'s.
    #[must_use]
    pub fn text_nav(&self) -> Option<NavMode> {
        self.out.text_nav()
    }

    /// Queues `mission`'s `kind` text to show, when it has one, taken in
    /// as a `dësc` ([`TextSource::Desc`](super::missions::TextSource::Desc), drawing on `chance`) and filled for `slot`
    /// ([`Session::mission_text`]), or with no mission
    /// ([`Session::plain_text`]): whether it had one. It opens no map
    /// ([`MissionText::map`]).
    pub(super) fn show_mission_text(
        &mut self,
        kind: MissionTextKind,
        mission: MissionId,
        slot: Option<u8>,
        chance: &mut dyn Chance,
    ) -> bool {
        self.show_mission_text_with_map(kind, mission, slot, None, chance)
    }

    /// [`Session::show_mission_text`], the text opening a map marked
    /// `map` on the map key, or none.
    pub(super) fn show_mission_text_with_map(
        &mut self,
        kind: MissionTextKind,
        mission: MissionId,
        slot: Option<u8>,
        map: Option<MapMarks>,
        chance: &mut dyn Chance,
    ) -> bool {
        let Some((raw, graphic, movie)) =
            self.statics.mission_texts.get(&mission).and_then(|texts| {
                Some((
                    kind.of(texts)?.to_owned(),
                    kind.graphic_of(texts),
                    kind.movie_of(texts),
                ))
            })
        else {
            return false;
        };
        let raw = self.load_description(Some(&raw), movie.as_ref(), chance);
        let text = match slot {
            Some(slot) => self.mission_text(slot, &raw),
            None => self.plain_text(&raw),
        };
        self.chart.queue_text(
            &self.pilot,
            &mut self.out,
            MissionText {
                kind,
                mission,
                text,
                graphic,
                movie,
                map,
            },
        );
        true
    }

    /// Queues `text`, `mission`'s `kind` text, a `STR#` text, to show as
    /// it is, with no picture, and the last `dësc` loaded's after-movie
    /// ([`Session::loaded_after_movie`]). The original reads the globals
    /// as it shows the text, not as it is queued; the texts are modal and
    /// shown in order, so no load comes between the two.
    pub(super) fn show_text(&mut self, kind: MissionTextKind, mission: MissionId, text: String) {
        let movie = self.loaded_after_movie();
        self.chart.queue_text(
            &self.pilot,
            &mut self.out,
            MissionText {
                kind,
                mission,
                text,
                graphic: None,
                movie,
                map: None,
            },
        );
    }

    /// The mission computer of the stellar docked at (`_DoMissionDialog`
    /// @0xa37cf): every offer at the computer in the order it shows, with
    /// no availability check (`_SetupMissionList` @0xa08c4-0xa09d6), each
    /// name and text filled for its offer, and today's medium date. The
    /// names read the remembered rank governments, as the original's list
    /// munges them before any text is loaded (@0xa0947); each text reads
    /// its own, as if loaded ([`Session::peek_desc`]: a missing one the
    /// remembered), changing nothing: the screen loads the selected one
    /// ([`Session::show_computer_row`]).
    ///
    /// # Errors
    ///
    /// [`ComputerRefusal::SlotsFull`] with every slot taken
    /// (@0xa37e9-0xa38b8); then [`ComputerRefusal::NoneAvailable`] in
    /// flight, or when none of the offers is available now
    /// (`_IsMissionAvailable(i, 1)` @0xa38bd-0xa3956), checked as an
    /// accept leaves them, drawing nothing.
    pub fn mission_computer(&self) -> Result<MissionComputer, ComputerRefusal> {
        if self
            .ship
            .accept_refusal(&self.pilot, &self.statics, &self.setup, 0, &mut NeverFires)
            == Err(AcceptRefusal::SlotsFull)
        {
            return Err(ComputerRefusal::SlotsFull);
        }
        let offers = self.mission_offers(Venue::Computer);
        if !offers.iter().any(|offer| self.still_available(offer)) {
            return Err(ComputerRefusal::NoneAvailable);
        }
        let rows = offers
            .iter()
            .map(|offer| {
                let texts = self.mission_texts(offer.mission);
                let name = texts.map_or("", |texts| texts.name.as_str());
                let (text, loaded) = self.peek_desc(texts.and_then(|texts| texts.offer.as_deref()));
                ComputerRow {
                    mission: offer.mission,
                    name: self.offer_text(offer.mission, name).unwrap_or_default(),
                    text: self
                        .offer_text_reading(offer.mission, &text, loaded)
                        .unwrap_or_default(),
                }
            })
            .collect();
        Ok(MissionComputer {
            rows,
            today: self.med_date_text(),
        })
    }

    /// The mission info screen (`_DoMissionInfoDialog` @0x9e19b): each
    /// active slot whose mission lacks `Flags` 0x0400, in slot order
    /// (`_SetupMissionInfoList` @0x9de7a), its name and `QuickBrief`
    /// filled for the slot (@0x9e2e1-0x9e309), and today's medium date. A
    /// slot whose `mïsn` is gone reads with no text, and abortable. The
    /// names read the remembered rank governments, as the original's list
    /// munges them before any text is loaded (@0x9e108); each text reads
    /// its own, as if loaded ([`Session::peek_desc`]: a missing one the
    /// remembered), changing nothing: the screen loads the selected one
    /// ([`Session::show_info_row`]).
    #[must_use]
    pub fn mission_info(&self) -> MissionInfo {
        let rows = self
            .pilot
            .missions
            .iter()
            .filter_map(|(slot, active)| {
                let record = self.statics.lifecycle_record(active.mission);
                if record.flags & HIDDEN != 0 {
                    return None;
                }
                let texts = self.mission_texts(active.mission);
                let name = texts.map_or("", |texts| texts.name.as_str());
                let (text, loaded) =
                    self.peek_desc(texts.and_then(|texts| texts.quick_brief.as_deref()));
                Some(InfoRow {
                    slot,
                    name: self.mission_text(slot, name),
                    failed: active.failed.is_some(),
                    text: self.mission_text_reading(slot, &text, loaded),
                    abortable: record.can_abort != 0,
                })
            })
            .collect();
        MissionInfo {
            rows,
            today: self.med_date_text(),
        }
    }

    /// Shows the mission computer's row of `mission`, selected: its offer
    /// text loaded as a `dësc` ([`Session::load_description`]: a missing one
    /// empty, keeping the governments) and filled for its offer, as
    /// `_MissionFilter` loads a selected row's (@0x9f9bd-0x9f9eb), and
    /// then its name filled, as `_MissionDialogUpdate` munges the
    /// selected name after (@0x9f40c), drawing on `chance`. `None` when
    /// it is not offered.
    pub fn show_computer_row(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> Option<ComputerRow> {
        self.run.offer_of(self.mode.landed(), mission)?;
        let texts = self.mission_texts(mission);
        let name = texts.map_or("", |texts| texts.name.as_str()).to_owned();
        let raw = texts.and_then(|texts| texts.offer.clone());
        let movie = texts.and_then(|texts| texts.movies.offer.clone());
        let raw = self.load_description(raw.as_deref(), movie.as_ref(), chance);
        Some(ComputerRow {
            mission,
            text: self.offer_text(mission, &raw)?,
            name: self.offer_text(mission, &name)?,
        })
    }

    /// Shows the mission info screen's row of `slot`, selected: its
    /// `QuickBrief` loaded as a `dësc` ([`Session::load_description`]: a missing
    /// one empty, keeping the governments) and filled for the slot, as
    /// `_MissionInfoFilter` loads a selected row's (@0x9d89f-0x9d8bc),
    /// drawing on `chance`. `None` for a free slot.
    pub fn show_info_row(&mut self, slot: u8, chance: &mut dyn Chance) -> Option<String> {
        let active = self.pilot.missions.get(slot)?;
        let texts = self.mission_texts(active.mission);
        let raw = texts.and_then(|texts| texts.quick_brief.clone());
        let movie = texts.and_then(|texts| texts.movies.quick_brief.clone());
        let raw = self.load_description(raw.as_deref(), movie.as_ref(), chance);
        Some(self.mission_text(slot, &raw))
    }

    /// The next offer at venue `loc` (an `AvailLoc` other than the mission
    /// computer's: [`BAR`](crate::mission::screens::BAR),
    /// [`SPACEPORT`], [`TRADE_CENTER`](crate::mission::screens::TRADE_CENTER),
    /// [`SHIPYARD`](crate::mission::screens::SHIPYARD) or
    /// [`OUTFITTER`](crate::mission::screens::OUTFITTER)), drawing on
    /// `chance`, as `_OfferMissionFromPort` (@0xa369e) offers one. It
    /// enters the venue first ([`Session::enter_offer_venue`]), as that
    /// routine does (@0xa36a7-0xa36d7), then offers the first outside
    /// offer, in the order they show, whose `AvailLoc` is `loc`, that is
    /// not suppressed, and that is still available, checked as an accept
    /// leaves them, drawing nothing (`_IsMissionAvailable(i, 1)`,
    /// @0xa36da-0xa3742). One that cannot be refused, with no offer text,
    /// is accepted at once. `None` with nothing to offer, or in flight.
    /// Answering the offer shifts the outside list from where it was
    /// found ([`RuleKey::OfferShift`](crate::RuleKey::OfferShift)), so an
    /// accept from the dialog drops the next outside offer too.
    pub fn next_offer(&mut self, loc: i16, chance: &mut dyn Chance) -> Option<VenueOffer> {
        // A new run of docked scripts (see [`Session::take_interface_loads`]).
        self.out.interface_loads.clear();
        self.enter_offer_venue(loc);
        let candidates: Vec<Offer> = self
            .mission_offers(Venue::Outside)
            .iter()
            .filter(|offer| {
                self.statics
                    .mission_record(offer.mission)
                    .is_some_and(|record| record.avail_loc == loc)
                    && !self.run.suppressed.contains(&offer.mission)
            })
            .cloned()
            .collect();
        for offer in candidates {
            // Each one checked writes its cached `AvailBits` (@0xa372f).
            self.check_avail_bits(offer.mission);
            if self.still_available(&offer) {
                return Some(self.offer_dialog(offer.mission, chance));
            }
        }
        None
    }

    /// Opens the mission computer as `_DoMissionDialog` does
    /// (@0xa37cf): [`Session::mission_computer`], but each offer it
    /// lists checked first, writing its cached `AvailBits`
    /// (`_IsMissionAvailable(i, 1)` @0xa38bd-0xa38ee; see
    /// [`RuleKey::CachedAvailBits`](crate::RuleKey::CachedAvailBits)),
    /// unless every slot is taken.
    ///
    /// # Errors
    ///
    /// As [`Session::mission_computer`].
    pub fn open_mission_computer(&mut self) -> Result<MissionComputer, ComputerRefusal> {
        if self
            .ship
            .accept_refusal(&self.pilot, &self.statics, &self.setup, 0, &mut NeverFires)
            != Err(AcceptRefusal::SlotsFull)
        {
            let listed: Vec<MissionId> = self
                .mission_offers(Venue::Computer)
                .iter()
                .map(|offer| offer.mission)
                .collect();
            for mission in listed {
                self.check_avail_bits(mission);
            }
        }
        self.mission_computer()
    }

    /// The offer dialog for `mission`, offered now (`_OfferOneMission`
    /// @0xa21ac), drawing on `chance`: its offer text loaded
    /// ([`Session::load_description`]: a missing one empty, keeping the
    /// governments) and filled for the offer, and its labels; or, for one
    /// that cannot be refused with no offer text, its accept at once
    /// (@0xa22ec-0xa231c), made before the dialog is, so with no offer
    /// dialog open ([`Session::accept_offered`], which then shifts the
    /// outside list as `_OfferMissionFromPort` does, @0xa377d-0xa37a1).
    pub(super) fn offer_dialog(
        &mut self,
        mission: MissionId,
        chance: &mut dyn Chance,
    ) -> VenueOffer {
        let can_refuse = self
            .statics
            .mission_record(mission)
            .is_some_and(|record| record.flags & UNREFUSABLE == 0);
        let texts = self.mission_texts(mission);
        let raw = texts.and_then(|texts| texts.offer.clone());
        let movie = texts.and_then(|texts| texts.movies.offer.clone());
        if !can_refuse && raw.as_deref().is_none_or(str::is_empty) {
            // The offer `dësc` is loaded before the empty-text check
            // (@0xa226e), so a no-room text reads its movie. The accept at
            // once returns 1 once it succeeds (@0xa2311, @0xa368e), so
            // `_OfferMissionFromPort` shifts the list (@0xa377d-0xa37a1),
            // which `accept_offered` does (`Session::drop_offered`). Its
            // before-movie plays first, as it comes before the empty-text
            // check (@0xa2273-0xa22ec); the accept at once returns before
            // the after-movie (@0xa230a-0xa231c).
            let _ = self.load_description(raw.as_deref(), movie.as_ref(), chance);
            self.show_offer_movie(mission, false);
            let accepted = self.accept_offered(mission, chance);
            return VenueOffer::Accepted { mission, accepted };
        }
        let labels = offer_labels(
            texts.map_or("", |texts| texts.accept_button.as_str()),
            texts.map_or("", |texts| texts.refuse_button.as_str()),
            can_refuse,
        );
        let graphic = texts.and_then(|texts| texts.graphics.offer);
        let raw = self.load_description(raw.as_deref(), movie.as_ref(), chance);
        VenueOffer::Prompt(OfferPrompt {
            mission,
            text: self.offer_text(mission, &raw).unwrap_or_default(),
            labels,
            graphic,
            movie,
        })
    }

    /// Enters venue `loc` for its offers (`_OfferMissionFromPort`
    /// @0xa36a7-0xa36d7): entering one other than the last forgets the
    /// suppressed offers, unless it is the spaceport ([`SPACEPORT`]).
    pub fn enter_offer_venue(&mut self, loc: i16) {
        self.run.enter_offer_venue(loc);
    }

    /// Whether `offer`, on the visit under way, is still available at its
    /// venue, the visit's rolls read and nothing drawn, as
    /// `_IsMissionAvailable(i, 1)` checks it: never in flight, nor for a
    /// mission the session has no record of.
    fn still_available(&self, offer: &Offer) -> bool {
        let (Some(stellar), Some(record)) = (
            self.mode.landed(),
            self.statics.mission_record(offer.mission),
        ) else {
            return false;
        };
        self.at_visit(stellar, |visit| {
            let facts = AvailFacts {
                visit,
                mission: record,
                venue: offer.venue,
                rolls: self.run.offers.table(),
                cached_avail_bits: None,
            };
            availability()
                .first_refusal(&facts, &mut NeverFires)
                .is_ok()
        })
        .unwrap_or(false)
    }
}

impl Outbox {
    /// The mission texts and offer movies to show since they were last
    /// taken, in the order they came. Taking them lets go of the bars
    /// noted with them ([`Session::text_bars`]).
    fn take_mission_texts(&mut self) -> Vec<MissionShown> {
        self.text_bars.clear();
        std::mem::take(&mut self.shown_texts)
    }

    /// The mission texts to show since they were last taken, for a test
    /// that queues no offer movie: it panics on one.
    #[cfg(test)]
    pub(super) fn take_texts_only(&mut self) -> Vec<MissionText> {
        self.take_mission_texts()
            .into_iter()
            .map(|shown| match shown {
                MissionShown::Text(text) => text,
                MissionShown::Movie(movie) => panic!("an offer movie: {movie:?}"),
            })
            .collect()
    }

    /// How many mission texts have been queued since the session started,
    /// never reset: a view that notes it at each of its redraws can tell
    /// whether the last text was queued after its last redraw.
    #[must_use]
    fn texts_queued(&self) -> u64 {
        self.texts_queued
    }

    /// The status bar as each mission text since they were last taken
    /// was queued, in order, one for each text ([`TextBar`]): what each
    /// text's caller redraws the whole bar with as it closes, where a
    /// script run after it, such as an `OnSuccess` after the `CompText`
    /// (@0xa048c, @0xa0491, then @0xa04f8), has written the bar since.
    /// Taking the texts lets go of them.
    #[must_use]
    fn text_bars(&self) -> &[TextBar] {
        &self.text_bars
    }

    /// The nav mode ([`Session::nav_mode`]) as the last text since they
    /// were last taken was queued, `None` with none: the last of
    /// [`Session::text_bars`]'s.
    #[must_use]
    fn text_nav(&self) -> Option<NavMode> {
        self.text_bars.last().map(|bar| bar.nav)
    }
}

impl Statics {
    /// What the screens show of mission `id`, as the session read it as
    /// it started: `None` for a `mïsn` it has no texts of.
    #[must_use]
    fn mission_texts(&self, id: MissionId) -> Option<&MissionTexts> {
        self.mission_texts.get(&id)
    }

    /// The movie `mission`'s offer `dësc` names, if any.
    fn offer_movie(&self, mission: MissionId) -> Option<DescMovie> {
        self.mission_texts(mission)
            .and_then(|texts| texts.movies.offer.clone())
    }
}

impl Chart {
    /// Queues `text` to show, counting it ([`Session::texts_queued`]) and
    /// noting the bar as it is now ([`Session::text_bars`]).
    fn queue_text(&mut self, pilot: &Pilot, out: &mut Outbox, text: MissionText) {
        out.texts_queued += 1;
        out.text_bars.push(TextBar {
            texts: out.texts_queued,
            nav: self.nav_mode(),
            reserves: super::reserves(pilot),
        });
        out.shown_texts.push(MissionShown::Text(text));
    }
}

impl MissionRun {
    /// Queues the after-movie the offer dialog of `mission` plays once its
    /// answer has queued its texts (`_OfferOneMission` @0xa3580-0xa35b8):
    /// only for an offer the dialog showed, at a venue or from a ship; the
    /// mission computer (`_DoMissionDialog`) plays none. The check reads
    /// the globals as the answer left them ([`RuleKey::DescMovieGlobals`]):
    /// by the engine, the last `dësc` loaded's after-movie, which after an
    /// accept is the offer's again (reloaded @0xa2e29), and after a
    /// `RefuseText` that text's, as nothing reloads the offer after it
    /// (@0xa2eb5-0xa2f4d); by the other reading the offer's own.
    pub(super) fn show_offer_after_movie(
        &mut self,
        statics: &Statics,
        setup: &Setup,
        out: &mut Outbox,
        mission: MissionId,
        venue: Venue,
    ) {
        if venue == Venue::Computer {
            return;
        }
        let movie = match setup.rules.source_for(RuleKey::DescMovieGlobals) {
            RuleSource::Engine => self.loaded_movie.clone(),
            RuleSource::Bible => statics.offer_movie(mission),
        };
        if let Some(movie) = movie.filter(|movie| movie.after) {
            out.shown_texts.push(MissionShown::Movie(movie));
        }
    }

    /// Enters venue `loc` for its offers (`_OfferMissionFromPort`
    /// @0xa36a7-0xa36d7): entering one other than the last forgets the
    /// suppressed offers, unless it is the spaceport ([`SPACEPORT`]).
    fn enter_offer_venue(&mut self, loc: i16) {
        if self.offer_venue != Some(loc) {
            if loc != SPACEPORT {
                self.suppressed.clear();
            }
            self.offer_venue = Some(loc);
        }
    }

    /// Suppresses the offer of `mission`, whose accept was refused, until
    /// another venue is entered (`_OfferMissionFromPort` @0xa3765).
    pub(super) fn suppress_offer(&mut self, mission: MissionId) {
        self.suppressed.insert(mission);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{
        DateAffixes, DescMovie, MissionRecord, PlaceNames, StellarId, SystemId, TextGraphics,
        TextMovies,
    };
    use crate::combat::hull::Condition;
    use crate::control::{Bit, Script, SetExpr, Test};
    use crate::geometry::Vec2;
    use crate::mission::AbortRefusal;
    use crate::mission::screens::{BAR, NO_LABEL, OKAY_LABEL, OUTFITTER, SHIPYARD, TRADE_CENTER};
    use crate::mission::{AcceptError, ActiveMission, OfferLabels, SLOTS};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::{AbortError, RefuseShown};
    use crate::sound::SimSound;
    use crate::testkit::{Draws, FakePilotCatalog, catalog, land_now, mission};

    /// Stellars 128 Earth, 129 Luna and 140 Mars; systems 130 Sol and 131
    /// Alpha.
    fn names() -> PlaceNames {
        PlaceNames {
            stellars: [(128, "Earth"), (129, "Luna"), (140, "Mars")]
                .into_iter()
                .map(|(id, name)| (StellarId(id), name.to_owned()))
                .collect(),
            systems: [(130, "Sol"), (131, "Alpha")]
                .into_iter()
                .map(|(id, name)| (SystemId(id), name.to_owned()))
                .collect(),
        }
    }

    /// [`catalog`], named, with 131 linked back to 130, `missions` and
    /// `texts`.
    fn screens(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.place_names = names();
        catalog.missions = missions;
        catalog.mission_texts = texts;
        catalog
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// A session of `catalog` landed on 128 and settled, every roll 1.
    fn docked(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, catalog);
        session
    }

    /// Mission `id`, offered at the mission computer, to Mars for 900.
    fn to_mars(id: i16) -> MissionRecord {
        MissionRecord {
            travel_stel: 140,
            pay_val: 900,
            ..mission(id)
        }
    }

    /// Mission `id`'s texts: named `name`, offering `offer`.
    fn texts(id: i16, name: &str, offer: Option<&str>) -> MissionTexts {
        MissionTexts {
            offer: offer.map(str::to_owned),
            ..MissionTexts::named(MissionId(id), name)
        }
    }

    fn bit(n: u16) -> Bit {
        Bit::new(n).expect("in range")
    }

    /// [`screens`] of `missions` and `texts`, with ranks 200 (government
    /// 128, "Cdr") and 201 (129, "Ad"), docked and both held.
    fn ranked(missions: Vec<MissionRecord>, texts: Vec<MissionTexts>, rules: Rulebook) -> Session {
        let mut catalog = screens(missions, texts);
        let rank = |id, govt, short: &str| crate::catalog::RankRecord {
            short_name: short.to_owned(),
            affil_govt: Some(crate::catalog::GovtId(govt)),
            ..crate::testkit::rank(id)
        };
        catalog.ranks = vec![rank(200, 128, "Cdr"), rank(201, 129, "Ad")];
        let mut session = docked(&catalog).with_rules(rules);
        session.run_set(
            &SetExpr::parse("K200 K201").expect("parses"),
            &mut NeverFires,
        );
        session
    }

    #[test]
    fn by_the_engine_the_computers_names_read_the_governments_left_and_each_text_its_own() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run <PRK128>", Some("Go, <PRK128>."))],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            (
                computer.rows[0].name.as_str(),
                computer.rows[0].text.as_str()
            ),
            ("Run Ad", "Go, captain."),
            "the list's names are munged before any text loads (@0xa0947); the \
             text as if loaded"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "the computer's view changes nothing"
        );
        let shown = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(
            (shown.name.as_str(), shown.text.as_str()),
            ("Run captain", "Go, captain."),
            "selected: its text loaded (@0x9f9bd), then its name munged again (@0x9f40c)"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "captain",
            "left by the load"
        );
        assert_eq!(
            session.mission_computer().expect("opens").rows[0].name,
            "Run captain",
            "opened again, the names read what the last load left"
        );
        assert_eq!(
            session.show_computer_row(MissionId(131), &mut NeverFires),
            None,
            "not offered"
        );
    }

    #[test]
    fn by_the_bible_the_computers_rank_nnn_tags_read_their_own_government() {
        let rules = Rulebook::default().with_override(RuleKey::CrossedRankTags, RuleSource::Bible);
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run <PRK128>", Some("Go, <PRK128>."))],
            rules,
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            (
                computer.rows[0].name.as_str(),
                computer.rows[0].text.as_str()
            ),
            ("Run Cdr", "Go, Cdr.")
        );
    }

    #[test]
    fn by_the_engine_a_shown_text_resets_the_governments_before_its_own() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![MissionTexts {
                brief: Some("Off you go, <PRK128>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Quote,
            "Earlier: <SRK129> said.",
            &mut NeverFires,
        );
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.out.take_texts_only(),
            [text(MissionTextKind::Brief, 130, "Off you go, captain.")],
            "the earlier <SRK129> is reset (@0x71926-0x71930)"
        );
    }

    #[test]
    fn the_mission_infos_names_read_the_governments_left_and_a_selected_row_loads_its_text() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![MissionTexts {
                quick_brief: Some("To <PRK128>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run <PRK128>")
            }],
            Rulebook::default(),
        );
        let slot = session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let info = session.mission_info();
        assert_eq!(
            (info.rows[0].name.as_str(), info.rows[0].text.as_str()),
            ("Run Ad", "To captain.")
        );
        assert_eq!(
            session.show_info_row(slot, &mut NeverFires).as_deref(),
            Some("To captain.")
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "captain",
            "left by the load"
        );
        assert_eq!(
            session.show_info_row(15, &mut NeverFires),
            None,
            "a free slot"
        );
    }

    #[test]
    fn a_selected_info_row_with_no_quick_brief_keeps_the_governments_left() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![MissionTexts::named(MissionId(130), "Run")],
            Rulebook::default(),
        );
        let slot = session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        assert_eq!(session.mission_info().rows[0].text, "");
        assert_eq!(
            session.show_info_row(slot, &mut NeverFires).as_deref(),
            Some("")
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "a missing dësc only empties the text (@0x71825, @0x71a5e-0x71a81): \
             no reset (@0x71926-0x71930), no parse (@0x71936)"
        );
    }

    #[test]
    fn a_selected_computer_row_with_no_offer_text_keeps_the_governments_left() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run", None)],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        assert_eq!(session.mission_computer().expect("opens").rows[0].text, "");
        let shown = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(shown.text, "");
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "a missing dësc is not loaded (@0x71825)"
        );
    }

    #[test]
    fn an_offer_with_no_offer_text_keeps_the_governments_left() {
        let mut session = ranked(
            vec![at_the_bar(130)],
            vec![texts(130, "Run", None)],
            Rulebook::default(),
        );
        let _ = session.take_in(
            TextSource::Desc,
            "Earlier: <PRK128> <SRK129>.",
            &mut NeverFires,
        );
        let Some(VenueOffer::Prompt(prompt)) = session.next_offer(BAR, &mut NeverFires) else {
            panic!("not a prompt");
        };
        assert_eq!(prompt.text, "");
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "a missing dësc is not loaded (@0x71825)"
        );
    }

    /// String lists of none, from data holding a `l33t` resource.
    struct Leet;

    impl crate::catalog::CommCatalog for Leet {
        fn string_list(&self, _id: i16) -> Vec<String> {
            Vec::new()
        }

        fn leet_mode(&self) -> bool {
            true
        }
    }

    /// [`docked`] in `catalog`, its strings [`Leet`]'s.
    fn leet(catalog: &FakePilotCatalog) -> Session {
        docked(catalog).with_strings(std::rc::Rc::new(Leet))
    }

    /// Mission 130 to Mars, "Ride" offering "You go to <DST> alone."
    fn leet_catalog() -> FakePilotCatalog {
        screens(
            vec![to_mars(130)],
            vec![texts(130, "Ride", Some("You go to <DST> alone."))],
        )
    }

    // The conditional text (`_ParseTextDescription` @0x14bdd-0x14f2f).

    /// A session of `texts`' mission 130, docked, its pilot of `gender`.
    fn conditioned(texts_of_130: MissionTexts, gender: crate::pilot::Gender) -> Session {
        let catalog = screens(vec![to_mars(130)], vec![texts_of_130]);
        let mut session = docked(&catalog);
        session.pilot.gender = gender;
        session
    }

    #[test]
    fn a_shown_offer_reads_its_conditional_text_by_the_pilots_gender() {
        use crate::pilot::Gender;
        let offer = texts(130, "Ride", Some("Go, {G \"sir\" \"ma'am\"}."));
        for (gender, said) in [(Gender::Male, "Go, sir."), (Gender::Female, "Go, ma'am.")] {
            let mut session = conditioned(offer.clone(), gender);
            let shown = session
                .show_computer_row(MissionId(130), &mut NeverFires)
                .expect("offered");
            assert_eq!(shown.text, said);
            assert_eq!(
                session.mission_computer().expect("opens").rows[0].text,
                said,
                "the computer's view too, as if loaded"
            );
        }
    }

    #[test]
    fn a_shown_texts_bit_test_reads_the_bit_as_of_its_load() {
        let offer = texts(130, "Ride", Some("{b7 \"Again\" \"First\"}"));
        let mut session = conditioned(offer, crate::pilot::Gender::Male);
        let first = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(first.text, "First");
        session.run_set(&SetExpr::parse("b7").expect("parses"), &mut NeverFires);
        let again = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(again.text, "Again");
    }

    #[test]
    fn a_computer_rows_view_reads_the_governments_of_its_chosen_string() {
        let mut session = ranked(
            vec![to_mars(130)],
            vec![texts(130, "Run", Some("{b7 \"<SRK128>\" \"\"}<PRK129>."))],
            Rulebook::default(),
        );
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            computer.rows[0].text, "captain.",
            "crossed, <PRK129> reads the <SRK> government, which the string \
             not chosen never named"
        );
        let shown = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(shown.text, "captain.", "loaded alike");
        session.run_set(&SetExpr::parse("b7").expect("parses"), &mut NeverFires);
        let shown = session
            .show_computer_row(MissionId(130), &mut NeverFires)
            .expect("offered");
        assert_eq!(
            shown.text, "AdCdr.",
            "chosen, <SRK128> names 128 for <PRK129>"
        );
    }

    #[test]
    fn with_leet_mode_the_letters_are_drawn_on_the_conditioned_text() {
        let catalog = screens(
            vec![to_mars(130)],
            vec![texts(130, "Ride", Some("{G \"Hy\" \"Ha\"}"))],
        );
        let mut session = leet(&catalog);
        let mut chance = Draws::of(&[0]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(
            chance.asked,
            Vec::<u32>::new(),
            "the a of the string not chosen is never drawn on"
        );
        assert_eq!(shown.text, "Hy");
        let catalog = screens(
            vec![to_mars(130)],
            vec![texts(130, "Ride", Some("{G \"Ho\" \"Hy\"}"))],
        );
        let mut session = leet(&catalog);
        let mut chance = Draws::of(&[0]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(chance.asked, [3], "only the o of Ho");
        assert_eq!(shown.text, "H0");
    }

    #[test]
    fn without_leet_mode_texts_are_unchanged() {
        let catalog = leet_catalog();
        let mut session = docked(&catalog);
        let mut chance = Draws::of(&[]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(shown.text, "You go to Mars alone.");
        assert_eq!(chance.asked, Vec::<u32>::new(), "nothing drawn");
    }

    #[test]
    fn with_leet_mode_a_shown_text_draws_its_letters_then_is_munged() {
        let catalog = leet_catalog();
        let mut session = leet(&catalog);
        // "You go to <DST> alone.": o (go) kept, o (to) replaced, a and o
        // (alone) replaced, e kept; Y's "ou" turns to j00 with no draw.
        // The munge then turns the e to 3 and the l to 7.
        let mut chance = Draws::of(&[2, 0, 1, 0, 2]);
        let shown = session
            .show_computer_row(MissionId(130), &mut chance)
            .expect("offered");
        assert_eq!(chance.asked, [3; 5]);
        assert_eq!(
            shown.text, "j00 go t0 Mars 470n3.",
            "letters drawn on the dësc (@0x7194f), then the filled text \
             munged (@0x9d5aa): the stellar's name too"
        );
        assert_eq!(shown.name, "R1d3", "a name is munged, never drawn on");
    }

    #[test]
    fn with_leet_mode_a_query_gives_the_same_text_each_time() {
        let catalog = leet_catalog();
        let session = leet(&catalog);
        let first = session.mission_computer().expect("opens");
        assert_eq!(first, session.mission_computer().expect("opens"));
        assert_eq!(
            first.rows[0].text, "j00 go to Mars a7on3.",
            "munged only, nothing drawn"
        );
        assert_eq!(
            session.offer_text(MissionId(130), "Hello"),
            session.offer_text(MissionId(130), "Hello")
        );
        assert_eq!(session.plain_text("Hello").as_str(), "H377o");
    }

    // The cached `AvailBits` (`mïsn`+0x16) the checks write.

    /// The cached verdicts of `missions`.
    fn cached(session: &Session, missions: &[i16]) -> Vec<Option<bool>> {
        missions
            .iter()
            .map(|&id| session.run.avail_cache.get(&MissionId(id)).copied())
            .collect()
    }

    /// Every verdict cached `false`, as if `AvailBits` had failed at the
    /// last refresh.
    fn all_cached_false(session: &mut Session) {
        for verdict in session.run.avail_cache.values_mut() {
            *verdict = false;
        }
    }

    #[test]
    fn a_landing_caches_every_missions_avail_bits_verdict() {
        let catalog = screens(
            vec![
                MissionRecord {
                    avail_bits: Test::parse("b7"),
                    ..to_mars(130)
                },
                to_mars(131),
            ],
            vec![],
        );
        let session = docked(&catalog);
        assert_eq!(cached(&session, &[130, 131]), [Some(false), Some(true)]);
    }

    #[test]
    fn opening_the_computer_writes_each_offers_verdict_and_its_view_writes_none() {
        let catalog = screens(vec![to_mars(130), to_mars(131), at_the_bar(150)], vec![]);
        let mut session = docked(&catalog);
        all_cached_false(&mut session);
        let _ = session.mission_computer();
        assert_eq!(cached(&session, &[130, 131]), [Some(false); 2], "a view");
        let _ = session.open_mission_computer();
        assert_eq!(
            cached(&session, &[130, 131]),
            [Some(true); 2],
            "@0xa38bd-0xa38ee"
        );
        all_cached_false(&mut session);
        fill_slots(&mut session);
        assert_eq!(
            session.open_mission_computer(),
            Err(ComputerRefusal::SlotsFull)
        );
        assert_eq!(
            cached(&session, &[130, 131]),
            [Some(false); 2],
            "refused first"
        );
    }

    #[test]
    fn a_port_offer_writes_each_verdict_it_checks_until_one_is_offered() {
        let catalog = screens(
            vec![at_the_bar(141), at_the_bar(142), at_the_bar(143)],
            vec![],
        );
        let mut session = docked(&catalog);
        session.enter_offer_venue(BAR);
        session.run.suppressed.insert(MissionId(141));
        all_cached_false(&mut session);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(142)
        );
        assert_eq!(
            cached(&session, &[141, 142, 143]),
            [Some(false), Some(true), Some(false)],
            "141 suppressed, 143 never reached (@0xa372f)"
        );
    }

    #[test]
    fn an_accept_with_the_computer_open_writes_each_remaining_offers_verdict() {
        let catalog = screens(vec![to_mars(130), to_mars(131), to_mars(132)], vec![]);
        let mut session = docked(&catalog);
        all_cached_false(&mut session);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            cached(&session, &[131, 132]),
            [Some(true); 2],
            "the refilter's _IsMissionAvailable(j, 1) (@0xa207a)"
        );
    }

    fn ids(computer: &MissionComputer) -> Vec<i16> {
        computer.rows.iter().map(|row| row.mission.0).collect()
    }

    #[test]
    fn today_reads_as_the_medium_date_with_the_chärs_affixes() {
        let mut catalog = catalog();
        catalog.date_affixes = DateAffixes {
            prefix: String::new(),
            suffix: " NC".to_owned(),
        };
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.med_date_text(),
            crate::date::med_date_text(session.date(), &catalog.date_affixes)
        );
        assert!(
            session.med_date_text().ends_with(" NC"),
            "{}",
            session.med_date_text()
        );
    }

    #[test]
    fn the_session_reads_each_missions_texts_as_it_starts() {
        let mut catalog = catalog();
        let run = MissionTexts {
            offer: Some("Go.".to_owned()),
            ..MissionTexts::named(MissionId(130), "Run")
        };
        catalog.mission_texts = vec![run.clone(), MissionTexts::named(MissionId(131), "Walk")];
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(session.mission_texts(MissionId(130)), Some(&run));
        assert_eq!(
            session
                .mission_texts(MissionId(131))
                .map(|texts| texts.name.as_str()),
            Some("Walk")
        );
        assert_eq!(session.mission_texts(MissionId(132)), None);
    }

    #[test]
    fn the_computer_lists_its_offers_in_display_order_filled_for_each_offer() {
        let catalog = screens(
            vec![
                MissionRecord {
                    disp_weight: -1,
                    ..to_mars(130)
                },
                to_mars(131),
                MissionRecord {
                    avail_loc: 1,
                    ..to_mars(132)
                },
            ],
            vec![
                texts(130, "Run to <DST>", Some("Take this to <DST> for <PAY>.")),
                texts(131, "Walk", None),
            ],
        );
        let session = docked(&catalog);
        let computer = session.mission_computer().expect("opens");
        assert_eq!(
            computer,
            MissionComputer {
                rows: vec![
                    ComputerRow {
                        mission: MissionId(131),
                        name: "Walk".to_owned(),
                        text: String::new(),
                    },
                    ComputerRow {
                        mission: MissionId(130),
                        name: "Run to Mars".to_owned(),
                        text: "Take this to Mars for 900.".to_owned(),
                    },
                ],
                today: session.med_date_text(),
            },
            "131 weighs 0, 130 below it; 132 is the bar's"
        );
    }

    #[test]
    fn with_every_slot_taken_the_computer_does_not_open() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let mut session = docked(&catalog);
        let offer = session.mission_offers(Venue::Computer)[0].clone();
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(141), &offer, session.date());
            session.pilot.missions.fill(active);
        }
        assert_eq!(session.mission_computer(), Err(ComputerRefusal::SlotsFull));
    }

    #[test]
    fn with_no_offer_available_again_the_computer_does_not_open() {
        let catalog = screens(
            vec![MissionRecord {
                avail_bits: Test::parse("!b5"),
                ..to_mars(130)
            }],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        assert!(session.mission_computer().is_ok());
        session.set_control_bit(bit(5), true);
        assert_eq!(
            session.mission_offers(Venue::Computer).len(),
            1,
            "still offered"
        );
        assert_eq!(
            session.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
    }

    #[test]
    fn the_computer_lists_an_offer_no_longer_available_while_another_is() {
        let catalog = screens(
            vec![
                MissionRecord {
                    avail_bits: Test::parse("!b5"),
                    ..to_mars(130)
                },
                to_mars(131),
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        session.set_control_bit(bit(5), true);
        let computer = session.mission_computer().expect("opens");
        assert_eq!(ids(&computer), [130, 131]);
    }

    #[test]
    fn in_flight_the_computer_has_nothing() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            session.mission_computer(),
            Err(ComputerRefusal::NoneAvailable)
        );
    }

    /// Takes `session` off, settles, puts it at rest over `stellar` (one
    /// of 130's), lands there and settles.
    fn land_on(session: &mut Session, catalog: &FakePilotCatalog, stellar: i16) {
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        settle(session, catalog);
        let site = *session
            .chart
            .sites
            .iter()
            .find(|site| site.id == StellarId(stellar))
            .expect("shown");
        session.ship.player.position = site.position;
        session.ship.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(session), Ok(StellarId(stellar)));
        settle(session, catalog);
    }

    /// Mission `id`, offered at Earth, to Luna and back there.
    fn to_luna(id: i16) -> MissionRecord {
        MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            ..mission(id)
        }
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

    /// `mission`'s `kind` text: a briefing opens the map, marking none of
    /// these missions, which lack `Flags` 0x0100.
    fn text(kind: MissionTextKind, mission: i16, text: &str) -> MissionText {
        MissionText {
            kind,
            mission: MissionId(mission),
            text: text.to_owned(),
            graphic: None,
            movie: None,
            map: (kind == MissionTextKind::Brief).then_some(MapMarks::NONE),
        }
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
    fn a_shown_text_carries_its_dëscs_graphic() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("S131"),
                    ..to_mars(130)
                },
                MissionRecord {
                    avail_loc: -1,
                    ..to_mars(131)
                },
            ],
            vec![
                MissionTexts {
                    refuse: Some("No.".to_owned()),
                    graphics: TextGraphics {
                        refuse: Some(5008),
                        brief: Some(5009),
                        ..TextGraphics::default()
                    },
                    ..MissionTexts::named(MissionId(130), "Run")
                },
                MissionTexts {
                    brief: Some("Off.".to_owned()),
                    graphics: TextGraphics {
                        brief: Some(5010),
                        ..TextGraphics::default()
                    },
                    ..MissionTexts::named(MissionId(131), "Walk")
                },
            ],
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(130), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.out.take_texts_only(),
            [
                MissionText {
                    graphic: Some(5008),
                    ..text(MissionTextKind::Refuse, 130, "No.")
                },
                MissionText {
                    graphic: Some(5010),
                    ..text(MissionTextKind::Brief, 131, "Off.")
                },
            ]
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
    fn a_shown_text_carries_its_dëscs_movie() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("S131"),
                    ..to_mars(130)
                },
                MissionRecord {
                    avail_loc: -1,
                    ..to_mars(131)
                },
            ],
            vec![
                MissionTexts {
                    refuse: Some("No.".to_owned()),
                    movies: TextMovies {
                        refuse: film("No.mov", 0x0001),
                        brief: film("Other.mov", 0),
                        ..TextMovies::default()
                    },
                    ..MissionTexts::named(MissionId(130), "Run")
                },
                MissionTexts {
                    brief: Some("Off.".to_owned()),
                    ..MissionTexts::named(MissionId(131), "Walk")
                },
            ],
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(130), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.out.take_texts_only(),
            [
                MissionText {
                    movie: film("No.mov", 0x0001),
                    ..text(MissionTextKind::Refuse, 130, "No.")
                },
                text(MissionTextKind::Brief, 131, "Off."),
            ]
        );
    }

    fn run(session: &mut Session, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
    }

    #[test]
    fn a_mission_completed_shows_its_comp_text_filled_before_its_slot_is_freed() {
        let catalog = screens(
            vec![to_luna(130)],
            vec![MissionTexts {
                comp: Some("Delivered to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.out.take_texts_only(), []);
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(
            session.out.take_texts_only(),
            [text(MissionTextKind::Comp, 130, "Delivered to Luna.")]
        );
        assert_eq!(session.out.take_texts_only(), [], "taken");
    }

    // The nav each text notes as it is queued.

    fn stellar(id: i16) -> NavMode {
        NavMode::Stellar(Some(StellarId(id)))
    }

    #[test]
    fn a_comp_text_notes_the_nav_from_before_on_success_moves() {
        let catalog = screens(
            vec![MissionRecord {
                on_success: Script::parse("M131"),
                ..to_luna(130)
            }],
            vec![MissionTexts {
                comp: Some("Done.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        land_on(&mut session, &catalog, 129);
        assert_eq!(session.text_nav(), Some(stellar(129)), "the landed stellar");
        assert_eq!(session.nav_mode(), stellar(140), "Alpha's first, by M131");
    }

    #[test]
    fn a_brief_text_notes_what_on_accept_wrote() {
        let catalog = screens(
            vec![MissionRecord {
                on_accept: Script::parse("M131"),
                ..to_mars(130)
            }],
            vec![MissionTexts {
                brief: Some("Off.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.text_nav(), Some(stellar(140)));
        assert_eq!(session.nav_mode(), stellar(140));
    }

    #[test]
    fn a_refuse_text_notes_the_nav_from_before_on_refuse() {
        let catalog = screens(
            vec![MissionRecord {
                on_refuse: Script::parse("M131"),
                ..to_mars(130)
            }],
            vec![MissionTexts {
                refuse: Some("No.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(130), &mut NeverFires)
            .expect("refused");
        assert_eq!(session.text_nav(), Some(stellar(128)), "the landed stellar");
        assert_eq!(session.nav_mode(), stellar(140), "Alpha's first, by M131");
    }

    #[test]
    fn the_texts_are_counted_for_good_and_the_nav_noted_until_they_are_taken() {
        let catalog = screens(
            vec![to_mars(130), to_mars(131)],
            vec![
                MissionTexts {
                    brief: Some("Off.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                },
                MissionTexts {
                    brief: Some("Away.".to_owned()),
                    ..MissionTexts::named(MissionId(131), "Walk")
                },
            ],
        );
        let mut session = docked(&catalog);
        assert_eq!((session.texts_queued(), session.text_nav()), (0, None));
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.texts_queued(), 1);
        session.chart.nav = NavMode::Off;
        session
            .accept_mission(MissionId(131), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.texts_queued(), 2);
        assert_eq!(session.text_nav(), Some(NavMode::Off), "the last text's");
        assert_eq!(session.take_mission_texts().len(), 2);
        assert_eq!(session.texts_queued(), 2, "never reset");
        assert_eq!(session.text_nav(), None, "taken");
    }

    #[test]
    fn a_mission_failed_at_its_return_shows_its_fail_text_filled_after_its_slot_is_freed() {
        let catalog = screens(
            vec![to_luna(130)],
            vec![MissionTexts {
                fail: Some("Failed at <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        run(&mut session, "F130");
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().get(0).is_none(), "failed");
        assert_eq!(
            session.out.take_texts_only(),
            [text(MissionTextKind::Fail, 130, "Failed at [Error].")]
        );
    }

    #[test]
    fn a_refusal_shows_its_refuse_text_with_no_mission_before_on_refuse_runs() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("S131"),
                    ..to_mars(130)
                },
                MissionRecord {
                    avail_loc: -1,
                    ..to_mars(131)
                },
            ],
            vec![
                MissionTexts {
                    refuse: Some("No trip to <DST>.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run")
                },
                MissionTexts {
                    brief: Some("Off to <DST>.".to_owned()),
                    ..MissionTexts::named(MissionId(131), "Walk")
                },
            ],
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(130), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.out.take_texts_only(),
            [
                text(MissionTextKind::Refuse, 130, "No trip to [Error]."),
                text(MissionTextKind::Brief, 131, "Off to Mars."),
            ],
            "_OfferOneMission munges it with index -1 (@0xa2eba-0xa2ec9)"
        );
    }

    #[test]
    fn an_accept_shows_its_brief_text_filled_for_its_slot_after_on_accept() {
        let catalog = screens(
            vec![MissionRecord {
                on_accept: Script::parse("A130"),
                ..to_mars(130)
            }],
            vec![MissionTexts {
                brief: Some("Off to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.out.take_texts_only(),
            [text(MissionTextKind::Brief, 130, "Off to [Error].")],
            "its OnAccept aborted it, freeing the slot"
        );
        let plain = screens(
            vec![to_mars(130)],
            vec![MissionTexts {
                brief: Some("Off to <DST>.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&plain);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            session.out.take_texts_only(),
            [text(MissionTextKind::Brief, 130, "Off to Mars.")]
        );
    }

    #[test]
    fn a_mission_without_the_text_shows_nothing() {
        let catalog = screens(
            vec![
                to_luna(130),
                to_luna(131),
                to_luna(132),
                MissionRecord {
                    avail_loc: -1,
                    ..to_luna(133)
                },
            ],
            vec![MissionTexts::named(MissionId(130), "Run")],
        );
        let mut session = docked(&catalog);
        for id in [130, 131] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        session
            .refuse_mission(MissionId(132), &mut NeverFires)
            .expect("refused");
        run(&mut session, "F131");
        land_on(&mut session, &catalog, 129);
        assert!(session.pilot().missions().iter().next().is_none());
        assert_eq!(session.out.take_texts_only(), []);
    }

    /// Mission `id`, offered at the bar, to Mars for 900.
    fn at_the_bar(id: i16) -> MissionRecord {
        MissionRecord {
            avail_loc: BAR,
            ..to_mars(id)
        }
    }

    /// Mission `id`'s texts: named "Mission `id`", offering `offer`, with
    /// these raw labels.
    fn offering(id: i16, offer: &str, accept: &str, refuse: &str) -> MissionTexts {
        MissionTexts {
            offer: Some(offer.to_owned()),
            accept_button: accept.to_owned(),
            refuse_button: refuse.to_owned(),
            ..MissionTexts::named(MissionId(id), &format!("Mission {id}"))
        }
    }

    #[test]
    fn an_offer_carries_its_offer_dëscs_graphic() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                graphics: TextGraphics {
                    offer: Some(5008),
                    refuse: Some(5009),
                    ..TextGraphics::default()
                },
                ..offering(142, "Ferry.", "", "")
            }],
        );
        let mut session = docked(&catalog);
        assert!(
            matches!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Prompt(OfferPrompt {
                    graphic: Some(5008),
                    ..
                }))
            ),
            "the offer's own, not another text's"
        );
    }

    fn film(file: &str, flags: u16) -> Option<DescMovie> {
        DescMovie::from_desc(file, flags)
    }

    #[test]
    fn an_offer_carries_its_offer_dëscs_movie() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                movies: TextMovies {
                    offer: film("Offer.mov", 0x0001),
                    refuse: film("Refuse.mov", 0),
                    ..TextMovies::default()
                },
                ..offering(142, "Ferry.", "", "")
            }],
        );
        let mut session = docked(&catalog);
        let Some(VenueOffer::Prompt(prompt)) = session.next_offer(BAR, &mut NeverFires) else {
            panic!("a prompt");
        };
        assert_eq!(prompt.movie, film("Offer.mov", 0x0001), "the offer's own");
    }

    #[test]
    fn an_offer_accepted_at_once_queues_its_before_movie_and_never_its_after_movie() {
        let unrefusable = |id| MissionRecord {
            flags: UNREFUSABLE,
            ..at_the_bar(id)
        };
        let silent = |id, movie| MissionTexts {
            movies: TextMovies {
                offer: movie,
                ..TextMovies::default()
            },
            ..offering(id, "", "", "")
        };
        let catalog = screens(
            vec![unrefusable(141), unrefusable(142)],
            vec![
                silent(141, film("Before.mov", 0x0004)),
                silent(142, film("After.mov", 0x0001)),
            ],
        );
        let mut session = docked(&catalog);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(141),
                accepted: Ok(0),
            }),
        );
        assert_eq!(
            queued(&mut session),
            [MissionShown::Movie(
                film("Before.mov", 0x0004).expect("named")
            )],
            "it plays before the empty-text check (@0xa2273-0xa22ec)"
        );
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(142),
                accepted: Ok(1),
            }),
        );
        assert_eq!(
            queued(&mut session),
            [],
            "the accept at once returns before the after-movie (@0xa230a-0xa231c)"
        );
    }

    /// Everything the session queued to show since it was last taken.
    fn queued(session: &mut Session) -> Vec<MissionShown> {
        session.take_mission_texts()
    }

    #[test]
    fn an_offers_movies_are_queued_in_order_with_the_texts_its_answer_shows() {
        let unrefusable = MissionRecord {
            flags: UNREFUSABLE,
            ..at_the_bar(141)
        };
        let filmed = |id, offer: &str, movie| MissionTexts {
            brief: Some(format!("Brief {id}.")),
            refuse: Some(format!("Refuse {id}.")),
            movies: TextMovies {
                offer: movie,
                ..TextMovies::default()
            },
            ..offering(id, offer, "", "")
        };
        let catalog = screens(
            vec![unrefusable, at_the_bar(142), to_mars(144)],
            vec![
                filmed(141, "", film("Before.mov", 0)),
                filmed(142, "Ferry.", film("After.mov", 0x0001)),
                filmed(144, "Ferry.", film("After.mov", 0x0001)),
            ],
        );
        let shown = |kind, id| MissionShown::Text(text(kind, id, &format!("{kind:?} {id}.")));
        let after = || MissionShown::Movie(film("After.mov", 0x0001).expect("named"));
        let mut session = docked(&catalog);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted { .. })
        ));
        assert_eq!(
            queued(&mut session),
            [
                MissionShown::Movie(film("Before.mov", 0).expect("named")),
                shown(MissionTextKind::Brief, 141),
            ],
            "accepted at once: the before-movie, then its BriefText (@0xa2273, @0xa230a)"
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(142), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            queued(&mut session),
            [shown(MissionTextKind::Brief, 142), after()],
            "accepted from the dialog: its texts, then the after-movie (@0xa3580)"
        );
        let mut session = docked(&catalog);
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            queued(&mut session),
            [shown(MissionTextKind::Refuse, 142)],
            "refused: its RefuseText, whose dësc names no movie, so the after-check \
             (@0xa3580) finds none: nothing reloads the offer after it (@0xa2eb5-0xa2f4d)"
        );
        session
            .accept_mission(MissionId(144), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            queued(&mut session),
            [shown(MissionTextKind::Brief, 144)],
            "the mission computer plays no offer movie (`_DoMissionDialog` has none)"
        );
    }

    #[test]
    fn an_offers_after_movie_plays_after_an_accept_refused() {
        let heavy = MissionRecord {
            cargo_type: 0,
            cargo_qty: 10_000,
            ..at_the_bar(142)
        };
        let filmed = |id| MissionTexts {
            brief: Some(format!("Brief {id}.")),
            movies: TextMovies {
                offer: film("After.mov", 0x0001),
                ..TextMovies::default()
            },
            ..offering(id, "Ferry.", "", "")
        };
        let catalog = screens(vec![heavy], vec![filmed(142)]);
        let after = || MissionShown::Movie(film("After.mov", 0x0001).expect("named"));
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(
            session.accept_mission(MissionId(142), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::NoHolds))
        );
        assert_eq!(
            queued(&mut session),
            [
                MissionShown::Text(MissionText {
                    movie: film("After.mov", 0x0001),
                    ..text(
                        MissionTextKind::NoRoom,
                        142,
                        &AcceptRefusal::NoHolds.message()
                    )
                }),
                after(),
            ],
            "the no-room text (`_DoMissionAccept` @0xa1bb2), with the offer's after-movie, \
             the last dësc loaded (@0x5b40b), then the after-movie again (`_OfferOneMission` \
             sets -0x139 whatever the accept returns @0xa2e5e, @0xa2ff7, and @0xa3580 plays it)"
        );
        let catalog = screens(vec![at_the_bar(142)], vec![filmed(142)]);
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(
            session.accept_mission(MissionId(142), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
        );
        assert_eq!(
            queued(&mut session),
            [after()],
            "every slot taken shows nothing (@0xa1b32-0xa1b42), then the after-movie (@0xa3580)"
        );
    }

    #[test]
    fn a_prompts_before_movie_is_left_to_the_dialog() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                brief: Some("Off.".to_owned()),
                movies: TextMovies {
                    offer: film("Before.mov", 0),
                    ..TextMovies::default()
                },
                ..offering(142, "Ferry.", "", "")
            }],
        );
        let mut session = docked(&catalog);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Prompt(_))
        ));
        assert_eq!(queued(&mut session), [], "the dialog plays it");
        session
            .accept_mission(MissionId(142), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            queued(&mut session),
            [MissionShown::Text(text(
                MissionTextKind::Brief,
                142,
                "Off."
            ))],
            "played before the dialog, never again after it"
        );
    }

    fn prompted(offer: Option<VenueOffer>) -> MissionId {
        match offer {
            Some(VenueOffer::Prompt(prompt)) => prompt.mission,
            other => panic!("not a prompt: {other:?}"),
        }
    }

    /// Takes every mission slot with mission 150.
    fn fill_slots(session: &mut Session) {
        let offer = Offer {
            mission: MissionId(150),
            ..session.mission_offers(Venue::Outside)[0].clone()
        };
        for _ in 0..SLOTS {
            let active = ActiveMission::accepted(&mission(150), &offer, session.date());
            session.pilot.missions.fill(active);
        }
    }

    #[test]
    fn the_bar_offers_its_first_offer_in_display_order_filled_with_its_labels() {
        let catalog = screens(
            vec![
                MissionRecord {
                    avail_loc: SPACEPORT,
                    ..to_mars(140)
                },
                MissionRecord {
                    disp_weight: -1,
                    ..at_the_bar(141)
                },
                at_the_bar(142),
            ],
            vec![offering(142, "Ferry this to <DST>.", "I'm in", "")],
        );
        let mut session = docked(&catalog);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Prompt(OfferPrompt {
                mission: MissionId(142),
                text: "Ferry this to Mars.".to_owned(),
                labels: OfferLabels {
                    accept: "I'm in".to_owned(),
                    refuse: Some(NO_LABEL.to_owned()),
                },
                graphic: None,
                movie: None,
            })),
            "142 weighs 0, 141 below it; 140 is the spaceport's"
        );
        assert!(
            matches!(session.next_offer(BAR, &mut NeverFires), Some(VenueOffer::Prompt(prompt)) if prompt.can_refuse()),
            "the same again, until it is answered"
        );
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .refuse_mission(MissionId(141), &mut NeverFires)
            .expect("refused");
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None, "none left");
        assert_eq!(
            prompted(session.next_offer(SPACEPORT, &mut NeverFires)),
            MissionId(140)
        );
    }

    #[test]
    fn in_flight_nothing_is_offered() {
        let catalog = screens(vec![at_the_bar(141)], Vec::new());
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
    }

    #[test]
    fn an_offer_that_cannot_be_refused_has_one_button_and_with_no_text_is_accepted_at_once() {
        let unrefusable = |id| MissionRecord {
            flags: UNREFUSABLE,
            ..at_the_bar(id)
        };
        let catalog = screens(
            vec![unrefusable(141), unrefusable(142)],
            vec![
                offering(141, "", "", ""),
                offering(142, "Come along.", "", "No"),
            ],
        );
        let mut session = docked(&catalog);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(141),
                accepted: Ok(0),
            })
        );
        assert_eq!(session.pilot().missions().missions(), [MissionId(141)]);
        let next = session.next_offer(BAR, &mut NeverFires);
        let Some(VenueOffer::Prompt(prompt)) = next else {
            panic!("a prompt: {next:?}");
        };
        assert_eq!(prompt.mission, MissionId(142));
        assert_eq!(
            prompt.labels,
            OfferLabels {
                accept: OKAY_LABEL.to_owned(),
                refuse: None,
            }
        );
        assert!(!prompt.can_refuse());
    }

    #[test]
    fn only_an_outside_offer_that_can_be_refused_is_answered_in_the_two_button_dialog() {
        let catalog = screens(
            vec![
                at_the_bar(141),
                MissionRecord {
                    flags: UNREFUSABLE,
                    ..at_the_bar(142)
                },
                to_mars(143),
            ],
            vec![
                offering(141, "Come along.", "", ""),
                offering(142, "Come along.", "", ""),
            ],
        );
        let mut session = docked(&catalog);
        let answered = |session: &Session| {
            [141, 142, 143, 144].map(|id| session.refusable_offer_dialog(MissionId(id)))
        };
        assert_eq!(
            answered(&session),
            [true, false, false, false],
            "at the bar, then unrefusable, at the computer and not offered"
        );
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(answered(&session), [false; 4], "in flight");
    }

    #[test]
    fn the_accept_at_once_drops_its_offer_but_reaches_nothing_and_refilters_nothing() {
        let catalog = screens(
            vec![
                MissionRecord {
                    flags: UNREFUSABLE,
                    avail_stel: 128,
                    travel_stel: 128,
                    on_accept: crate::control::Script::parse("b5"),
                    ..at_the_bar(141)
                },
                MissionRecord {
                    avail_bits: crate::control::Test::parse("!b5"),
                    ..at_the_bar(143)
                },
            ],
            vec![offering(141, "", "", "")],
        );
        let mut session = docked(&catalog);
        let ids = |session: &Session| -> Vec<MissionId> {
            session
                .mission_offers(Venue::Outside)
                .iter()
                .map(|offer| offer.mission)
                .collect()
        };
        assert_eq!(ids(&session), [MissionId(141), MissionId(143)]);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Ok(0),
                ..
            })
        ));
        let active = session.pilot().missions().get(0).expect("held");
        assert!(!active.reached, "the dialog never opened (@0xa230a)");
        assert_eq!(
            ids(&session),
            [MissionId(143)],
            "the caller drops the accepted offer (@0xa3765-0xa37a1); the \
             others stay unfiltered"
        );
    }

    /// The outside offers, in the order they show.
    fn outside(session: &Session) -> Vec<i16> {
        session
            .mission_offers(Venue::Outside)
            .iter()
            .map(|offer| offer.mission.0)
            .collect()
    }

    /// The rulebook with `offer_shift` read by `source`.
    fn shifting(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::OfferShift, source)
    }

    #[test]
    fn an_accept_from_the_offer_dialog_drops_the_next_outside_offer() {
        let catalog = screens(
            vec![at_the_bar(141), at_the_bar(142), at_the_bar(143)],
            Vec::new(),
        );
        for (source, left, next) in [
            (RuleSource::Engine, vec![143], 143),
            (RuleSource::Bible, vec![142, 143], 142),
        ] {
            let mut session = docked(&catalog).with_rules(shifting(source));
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(141)
            );
            session
                .accept_mission(MissionId(141), &mut NeverFires)
                .expect("accepted");
            assert_eq!(outside(&session), left, "{source:?}");
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(next),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_dropped_offer_is_the_next_listed_whatever_its_venue() {
        let catalog = screens(
            vec![at_the_bar(141), at_venue(142, SHIPYARD), at_the_bar(143)],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        assert_eq!(session.next_offer(SHIPYARD, &mut NeverFires), None);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(143)
        );

        let mut session = docked(&catalog).with_rules(shifting(RuleSource::Bible));
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        assert_eq!(
            prompted(session.next_offer(SHIPYARD, &mut NeverFires)),
            MissionId(142)
        );
    }

    #[test]
    fn the_drop_reads_the_list_the_accepts_refilter_left() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_accept: Script::parse("b5"),
                    ..at_the_bar(141)
                },
                MissionRecord {
                    avail_bits: Test::parse("!b5"),
                    ..at_the_bar(142)
                },
                at_the_bar(143),
                at_the_bar(144),
            ],
            Vec::new(),
        );
        for (source, left) in [
            (RuleSource::Engine, vec![144]),
            (RuleSource::Bible, vec![143, 144]),
        ] {
            let mut session = docked(&catalog).with_rules(shifting(source));
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(141)
            );
            session
                .accept_mission(MissionId(141), &mut NeverFires)
                .expect("accepted");
            assert_eq!(
                outside(&session),
                left,
                "{source:?}: the refilter drops 142, so the shift drops 143"
            );
        }
    }

    #[test]
    fn accepting_the_last_listed_offer_drops_no_other() {
        let catalog = screens(vec![at_venue(141, SHIPYARD), at_the_bar(142)], Vec::new());
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = docked(&catalog).with_rules(shifting(source));
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(142)
            );
            session
                .accept_mission(MissionId(142), &mut NeverFires)
                .expect("accepted");
            assert_eq!(outside(&session), [141], "{source:?}");
        }
    }

    #[test]
    fn a_refusal_whose_on_refuse_refilters_leaves_the_refused_offer_listed() {
        let catalog = screens(
            vec![
                at_venue(141, SHIPYARD),
                MissionRecord {
                    on_refuse: Script::parse("S141"),
                    ..at_the_bar(142)
                },
                at_the_bar(143),
            ],
            Vec::new(),
        );
        for (source, left, next) in [
            (RuleSource::Engine, vec![142], 142),
            (RuleSource::Bible, vec![143], 143),
        ] {
            let mut session = docked(&catalog).with_rules(shifting(source));
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(142)
            );
            session
                .refuse_mission(MissionId(142), &mut NeverFires)
                .expect("refused");
            assert_eq!(
                session.pilot().missions().missions(),
                [MissionId(141)],
                "{source:?}"
            );
            assert_eq!(outside(&session), left, "{source:?}");
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(next),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_accept_at_once_drops_its_own_offer_unless_it_refilters() {
        let catalog = screens(
            vec![unrefusable_at(141, BAR), at_the_bar(142), at_the_bar(143)],
            vec![offering(141, "", "", "")],
        );
        let rules =
            |dialog, shift| shifting(shift).with_override(RuleKey::StartOutsideDialog, dialog);
        for (dialog, shift, left) in [
            (RuleSource::Engine, RuleSource::Engine, vec![142, 143]),
            (RuleSource::Engine, RuleSource::Bible, vec![142, 143]),
            (RuleSource::Bible, RuleSource::Bible, vec![142, 143]),
            // The keys compose literally: the accept drops its own offer
            // as if from the dialog, and the shift then drops the next.
            (RuleSource::Bible, RuleSource::Engine, vec![143]),
        ] {
            let mut session = docked(&catalog).with_rules(rules(dialog, shift));
            assert_eq!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    accepted: Ok(0),
                })
            );
            assert_eq!(outside(&session), left, "{dialog:?} {shift:?}");
        }
    }

    #[test]
    fn an_outside_accept_refused_for_full_slots_shows_nothing_and_drops_nothing() {
        let catalog = screens(
            vec![at_the_bar(141), at_the_bar(142), at_the_bar(143)],
            vec![offering(141, "Come along.", "", "")],
        );
        for source in [RuleSource::Engine, RuleSource::Bible] {
            let mut session = docked(&catalog).with_rules(shifting(source));
            fill_slots(&mut session);
            let _ = session.take_sounds();
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(141),
                "offered with every slot taken"
            );
            assert_eq!(
                session.accept_mission(MissionId(141), &mut NeverFires),
                Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
            );
            assert_eq!(session.out.take_texts_only(), [], "{source:?}");
            assert_eq!(session.take_sounds(), [], "{source:?}");
            assert_eq!(outside(&session), [141, 142, 143], "{source:?}");
            assert_eq!(
                prompted(session.next_offer(BAR, &mut NeverFires)),
                MissionId(142),
                "{source:?}: 141 suppressed"
            );
        }
    }

    #[test]
    fn a_mission_computer_accept_leaves_the_outside_offers_whole() {
        let catalog = screens(
            vec![to_mars(140), at_the_bar(141), at_the_bar(142)],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(140), &mut NeverFires)
            .expect("accepted");
        assert!(session.mission_offers(Venue::Computer).is_empty());
        assert_eq!(outside(&session), [141, 142]);
    }

    #[test]
    fn an_outside_offer_whose_accept_is_refused_is_suppressed() {
        let catalog = screens(
            vec![
                MissionRecord {
                    flags: UNREFUSABLE,
                    ..at_the_bar(141)
                },
                at_the_bar(142),
                at_the_bar(143),
            ],
            vec![offering(141, "", "", "")],
        );
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                mission: MissionId(141),
                accepted: Err(AcceptError::Refused(AcceptRefusal::SlotsFull)),
            })
        );
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(142)
        );
        assert_eq!(
            session.accept_mission(MissionId(142), &mut NeverFires),
            Err(AcceptError::Refused(AcceptRefusal::SlotsFull))
        );
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(143),
            "141 and 142 suppressed"
        );
    }

    #[test]
    fn an_outside_offer_accepted_is_not_suppressed() {
        let catalog = screens(vec![at_the_bar(141)], Vec::new());
        let mut session = docked(&catalog);
        let slot = session
            .accept_mission(MissionId(141), &mut NeverFires)
            .expect("accepted");
        session
            .abort_mission(slot, &mut NeverFires)
            .expect("aborted");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        settle(&mut session, &catalog);
        session.ship.player.velocity = Vec2::ZERO;
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        settle(&mut session, &catalog);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141),
            "offered again on the next landing"
        );
    }

    #[test]
    fn suppression_lasts_until_a_venue_other_than_the_last_but_the_spaceport_is_entered() {
        let catalog = screens(vec![at_the_bar(142), at_the_bar(143)], Vec::new());
        let mut session = docked(&catalog);
        session.enter_offer_venue(BAR);
        let mut full = docked(&catalog);
        full.enter_offer_venue(BAR);
        fill_slots(&mut full);
        assert!(
            full.accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        let first = |session: &mut Session| prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(first(&mut full), MissionId(143));
        full.enter_offer_venue(BAR);
        assert_eq!(first(&mut full), MissionId(143), "the same visit");
        full.enter_offer_venue(SPACEPORT);
        assert!(
            full.run.suppressed.contains(&MissionId(142)),
            "the spaceport keeps it"
        );
        full.enter_offer_venue(BAR);
        assert_eq!(
            first(&mut full),
            MissionId(142),
            "back in the bar, forgotten"
        );
        assert!(
            full.accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        let flown = Session::fly(&catalog, full.pilot().clone()).expect("flies");
        let mut flown = flown;
        assert_eq!(land_now(&mut flown), Ok(StellarId(128)));
        settle(&mut flown, &catalog);
        assert_eq!(first(&mut flown), MissionId(142), "never saved");
        assert_eq!(first(&mut session), MissionId(142));
    }

    /// Mission `id`, offered at venue `loc`, to Mars for 900.
    fn at_venue(id: i16, loc: i16) -> MissionRecord {
        MissionRecord {
            avail_loc: loc,
            ..to_mars(id)
        }
    }

    /// Mission `id`, offered at venue `loc`, that cannot be refused and
    /// has no offer text, so its offer is an accept at once.
    fn unrefusable_at(id: i16, loc: i16) -> MissionRecord {
        MissionRecord {
            flags: UNREFUSABLE,
            ..at_venue(id, loc)
        }
    }

    const VENUES: [i16; 5] = [BAR, SPACEPORT, TRADE_CENTER, SHIPYARD, OUTFITTER];

    #[test]
    fn next_offer_at_the_spaceport_offers_an_availloc_3_mission() {
        let catalog = screens(vec![at_venue(141, SPACEPORT)], Vec::new());
        for loc in VENUES.into_iter().filter(|&loc| loc != SPACEPORT) {
            let mut session = docked(&catalog);
            assert_eq!(session.next_offer(loc, &mut NeverFires), None, "{loc}");
        }
        let mut session = docked(&catalog);
        assert!(matches!(
            session.next_offer(SPACEPORT, &mut NeverFires),
            Some(VenueOffer::Prompt(prompt)) if prompt.mission == MissionId(141) && prompt.can_refuse()
        ));
    }

    #[test]
    fn next_offer_at_each_shop_offers_only_its_own_missions() {
        let catalog = screens(
            vec![
                at_venue(141, TRADE_CENTER),
                at_venue(142, SHIPYARD),
                at_venue(143, OUTFITTER),
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        for (loc, id) in [(TRADE_CENTER, 141), (SHIPYARD, 142), (OUTFITTER, 143)] {
            assert_eq!(
                prompted(session.next_offer(loc, &mut NeverFires)),
                MissionId(id),
                "{loc}"
            );
        }
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
    }

    #[test]
    fn the_next_offer_enters_its_venue_forgetting_the_suppressed_offers() {
        let catalog = screens(vec![unrefusable_at(141, BAR)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Err(_),
                ..
            })
        ));
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None, "suppressed");
        assert_eq!(session.next_offer(OUTFITTER, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the outfitter entered forgot it"
        );
    }

    #[test]
    fn a_spaceport_offer_refused_stays_suppressed_until_another_venue_is_offered_at() {
        let catalog = screens(vec![unrefusable_at(141, SPACEPORT)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(matches!(
            session.next_offer(SPACEPORT, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Err(_),
                ..
            })
        ));
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(SPACEPORT, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the bar entered forgot it"
        );
    }

    /// A bar offer refused for want of a slot stays suppressed while no
    /// other venue is offered at, and offering at the spaceport in between
    /// forgets it: so the router must not ask at the spaceport again as
    /// the player comes back to it from the bar.
    #[test]
    fn a_bar_offer_stays_suppressed_while_nothing_is_offered_at_in_between() {
        let catalog = screens(vec![unrefusable_at(141, BAR)], Vec::new());
        let mut session = docked(&catalog);
        fill_slots(&mut session);
        assert!(session.next_offer(BAR, &mut NeverFires).is_some());
        assert_eq!(
            session.next_offer(BAR, &mut NeverFires),
            None,
            "nothing offered at in between"
        );
        assert_eq!(session.next_offer(SPACEPORT, &mut NeverFires), None);
        assert!(
            matches!(
                session.next_offer(BAR, &mut NeverFires),
                Some(VenueOffer::Accepted {
                    mission: MissionId(141),
                    ..
                })
            ),
            "the spaceport offered at forgot it"
        );
    }

    #[test]
    fn a_refusal_that_makes_a_later_offer_unavailable_stops_it_being_offered() {
        let catalog = screens(
            vec![
                MissionRecord {
                    on_refuse: Script::parse("b5"),
                    ..at_the_bar(141)
                },
                MissionRecord {
                    avail_bits: Test::parse("!b5"),
                    ..at_the_bar(142)
                },
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        assert_eq!(
            prompted(session.next_offer(BAR, &mut NeverFires)),
            MissionId(141)
        );
        session
            .refuse_mission(MissionId(141), &mut NeverFires)
            .expect("refused");
        assert_eq!(
            session.mission_offers(Venue::Outside).len(),
            1,
            "142 is still listed"
        );
        assert_eq!(session.next_offer(BAR, &mut NeverFires), None);
    }

    fn info_row(slot: u8, name: &str, failed: bool, text: &str, abortable: bool) -> InfoRow {
        InfoRow {
            slot,
            name: name.to_owned(),
            failed,
            text: text.to_owned(),
            abortable,
        }
    }

    #[test]
    fn mission_info_lists_the_shown_active_missions_filled_for_their_slots() {
        let catalog = screens(
            vec![
                MissionRecord {
                    time_limit: 7,
                    ..to_mars(130)
                },
                MissionRecord {
                    flags: HIDDEN,
                    ..to_mars(131)
                },
                MissionRecord {
                    can_abort: 0,
                    ..to_mars(132)
                },
            ],
            vec![
                MissionTexts {
                    quick_brief: Some("Deliver to <DST> by <DL>.".to_owned()),
                    ..MissionTexts::named(MissionId(130), "Run to <DST>")
                },
                MissionTexts::named(MissionId(131), "Hidden"),
                MissionTexts::named(MissionId(132), "Stay"),
            ],
        );
        let mut session = docked(&catalog);
        for id in [130, 131, 132] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        run(&mut session, "F132");
        assert_eq!(
            session.mission_info(),
            MissionInfo {
                rows: vec![
                    info_row(
                        0,
                        "Run to Mars",
                        false,
                        "Deliver to Mars by June 30th, 1177 NC.",
                        true
                    ),
                    info_row(2, "Stay", true, "", false),
                ],
                today: session.med_date_text(),
            },
            "131 has Flags 0x0400; 132 failed by F, and cannot be aborted"
        );
    }

    #[test]
    fn a_slot_whose_mission_is_gone_reads_with_no_text() {
        let catalog = screens(
            vec![to_mars(130)],
            vec![MissionTexts {
                quick_brief: Some("Go.".to_owned()),
                ..MissionTexts::named(MissionId(130), "Run")
            }],
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let gone = screens(Vec::new(), Vec::new());
        let flown = Session::fly(&gone, session.pilot().clone()).expect("flies");
        assert_eq!(
            flown.mission_info().rows,
            [info_row(0, "", false, "", true)]
        );
        assert_eq!(
            Session::start(&gone).expect("starts").mission_info().rows,
            []
        );
    }

    #[test]
    fn pressing_mission_info_with_no_missions_beeps_and_says_so() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let mut session = docked(&catalog);
        let _ = session.take_sounds();
        let _ = session.take_messages();
        assert_eq!(
            session.press_mission_info(),
            MissionInfoPress::NoActiveMissions
        );
        assert_eq!(session.take_sounds(), [crate::SimSound::Refused]);
        assert_eq!(
            session.take_messages(),
            [crate::SimMessage::NoActiveMissions]
        );
    }

    #[test]
    fn pressing_mission_info_with_only_a_hidden_mission_beeps_and_says_so() {
        let catalog = screens(
            vec![MissionRecord {
                flags: HIDDEN,
                ..to_mars(130)
            }],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let _ = session.take_sounds();
        let _ = session.take_messages();
        assert_eq!(
            session.press_mission_info(),
            MissionInfoPress::NoActiveMissions
        );
        assert_eq!(session.take_sounds(), [crate::SimSound::Refused]);
        assert_eq!(
            session.take_messages(),
            [crate::SimMessage::NoActiveMissions]
        );
    }

    #[test]
    fn pressing_mission_info_with_a_failed_shown_mission_opens_it_silently() {
        let catalog = screens(vec![to_mars(130)], Vec::new());
        let mut session = docked(&catalog);
        session
            .accept_mission(MissionId(130), &mut NeverFires)
            .expect("accepted");
        let slot = session.mission_info().rows[0].slot;
        session
            .slot_holding(slot, MissionId(130))
            .expect("held")
            .failed = Some(crate::mission::FailedBy::Quick);
        let _ = session.take_sounds();
        let _ = session.take_messages();
        assert_eq!(
            session.press_mission_info(),
            MissionInfoPress::Opens(session.mission_info())
        );
        assert_eq!(session.take_sounds(), []);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn mission_info_aborts_only_where_its_row_says() {
        let catalog = screens(
            vec![
                to_mars(130),
                MissionRecord {
                    can_abort: 0,
                    ..to_mars(131)
                },
            ],
            Vec::new(),
        );
        let mut session = docked(&catalog);
        for id in [130, 131] {
            session
                .accept_mission(MissionId(id), &mut NeverFires)
                .expect("accepted");
        }
        let info = session.mission_info();
        let refused = info.rows.iter().find(|row| !row.abortable).expect("131");
        assert_eq!(
            session.abort_mission(refused.slot, &mut NeverFires),
            Err(AbortError::Refused(AbortRefusal::CannotAbort))
        );
        let allowed = info.rows.iter().find(|row| row.abortable).expect("130");
        assert_eq!(session.abort_mission(allowed.slot, &mut NeverFires), Ok(()));
        let left: Vec<_> = session
            .mission_info()
            .rows
            .iter()
            .map(|row| row.slot)
            .collect();
        assert_eq!(left, [refused.slot]);
    }

    #[test]
    fn each_refusal_says_what_the_original_says() {
        assert_eq!(
            ComputerRefusal::SlotsFull.message(),
            AcceptRefusal::SlotsFull.message()
        );
        assert_eq!(ComputerRefusal::NoneAvailable.message(), NO_MISSIONS_HERE);
    }

    // The flight key, `_HandlePlayer` @0x70405-0x705b4.

    /// A session of [`screens`] landed on 128 with mission 130 to Mars
    /// accepted, and one with `flags` 0x0400 too when `hidden`, its
    /// sounds taken.
    fn with_missions(shown: bool, hidden: bool) -> Session {
        let catalog = screens(
            vec![
                to_mars(130),
                MissionRecord {
                    flags: HIDDEN,
                    ..to_mars(131)
                },
            ],
            vec![
                MissionTexts::named(MissionId(130), "Run"),
                MissionTexts::named(MissionId(131), "Hidden"),
            ],
        );
        let mut session = docked(&catalog);
        for (id, take) in [(130, shown), (131, hidden)] {
            if take {
                session
                    .accept_mission(MissionId(id), &mut NeverFires)
                    .expect("accepted");
            }
        }
        let _ = session.take_sounds();
        session
    }

    #[test]
    fn the_mission_info_key_with_no_shown_mission_beeps_and_opens_nothing() {
        for (why, hidden) in [("none", false), ("only a 0x0400 one", true)] {
            let mut session = with_missions(false, hidden);
            assert_eq!(
                session.press_mission_info(),
                MissionInfoPress::NoActiveMissions,
                "{why}"
            );
            assert_eq!(
                session.take_sounds(),
                [SimSound::Refused],
                "{why} (@0x70557)"
            );
        }
    }

    #[test]
    fn the_mission_info_key_with_a_shown_mission_opens_the_screen_in_silence() {
        let mut session = with_missions(true, true);
        let info = session.mission_info();
        assert_eq!(info.rows.len(), 1);
        assert_eq!(session.press_mission_info(), MissionInfoPress::Opens(info));
        assert_eq!(session.take_sounds(), []);
    }

    #[test]
    fn the_mission_info_key_is_refused_in_silence_in_a_jump_the_pod_a_breakup_or_the_grace() {
        type Refusal = (&'static str, fn(&mut Session));
        let refusals: [Refusal; 7] = [
            ("the pre-jump, +0x50 > 0 (@0x70424)", |session| {
                session.mode = crate::session::Mode::Jumping(super::super::Jump::PreJump {
                    to: SystemId(131),
                    bearing: None,
                    warp: None,
                });
            }),
            ("hyperspace, +0x50 > 0 (@0x70424)", |session| {
                session.mode = crate::session::Mode::Jumping(super::super::Jump::Hyperspace {
                    to: SystemId(131),
                    warp: 0,
                });
            }),
            ("the pod, +0xc908 > 0 (@0x7042e)", |session| {
                session.pod = Some(10);
            }),
            (
                "breaking up, -0x2ca and +0x3c (@0x70439, @0x7044b)",
                |session| {
                    session.ship.condition = Condition::Dying {
                        ticks_left: 3,
                        starting: false,
                    };
                },
            ),
            ("exploded, the dead branch (@0x6870d)", |session| {
                session.ship.condition = Condition::Destroyed;
                session.ship.fate.player_exploded_this_tick = true;
            }),
            ("holding after the explosion", |session| {
                session.ship.condition = Condition::Destroyed;
            }),
            (
                "the take-off grace, _frameCounter < 0 (@0x7045e)",
                |session| {
                    session.start_takeoff_grace();
                },
            ),
        ];
        for (why, refuse) in refusals {
            for shown in [false, true] {
                let mut session = with_missions(shown, false);
                refuse(&mut session);
                let before = session.clone();
                assert_eq!(
                    session.press_mission_info(),
                    MissionInfoPress::Refused,
                    "{why}, shown {shown}"
                );
                assert_eq!(session.take_sounds(), [], "{why}, shown {shown}");
                assert_eq!(session.pilot(), before.pilot(), "{why}, shown {shown}");
            }
        }
    }

    #[test]
    fn a_disabled_ship_takes_the_mission_info_key() {
        let mut session = with_missions(true, false);
        session.ship.condition = Condition::Disabled;
        let info = session.mission_info();
        assert_eq!(session.press_mission_info(), MissionInfoPress::Opens(info));
        let mut none = with_missions(false, false);
        none.ship.condition = Condition::Disabled;
        assert_eq!(
            none.press_mission_info(),
            MissionInfoPress::NoActiveMissions
        );
        assert_eq!(none.take_sounds(), [SimSound::Refused]);
    }

    /// The rulebook with `desc_movie_globals` read by `source`.
    fn globals(source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(RuleKey::DescMovieGlobals, source)
    }

    /// Mission `id`, offered at the bar, with cargo no hold can take.
    fn heavy(id: i16) -> MissionRecord {
        MissionRecord {
            cargo_type: 0,
            cargo_qty: 10_000,
            ..at_the_bar(id)
        }
    }

    /// Mission `id`'s texts, offering "Ferry." with the offer movie
    /// `offer` and a `BriefText`.
    fn offer_filmed(id: i16, offer: Option<DescMovie>) -> MissionTexts {
        MissionTexts {
            brief: Some(format!("Brief {id}.")),
            movies: TextMovies {
                offer,
                ..TextMovies::default()
            },
            ..offering(id, "Ferry.", "", "")
        }
    }

    /// The no-room text of mission `id`, for want of holds, with `movie`.
    fn no_holds(id: i16, movie: Option<DescMovie>) -> MissionShown {
        MissionShown::Text(MissionText {
            movie,
            ..text(
                MissionTextKind::NoRoom,
                id,
                &AcceptRefusal::NoHolds.message(),
            )
        })
    }

    fn movie_shown(file: &str, flags: u16) -> MissionShown {
        MissionShown::Movie(film(file, flags).expect("named"))
    }

    #[test]
    fn a_loaded_desc_leaves_its_movie_and_a_missing_one_empties_it() {
        let mut session = ranked(Vec::new(), Vec::new(), Rulebook::default());
        let movie = film("Loaded.mov", 0x0001);
        assert_eq!(
            session.run.loaded_movie, None,
            "a new session has loaded none"
        );
        let _ = session.load_description(
            Some("Earlier: <PRK128> <SRK129>."),
            movie.as_ref(),
            &mut NeverFires,
        );
        assert_eq!(session.run.loaded_movie, movie);
        let _ = session.load_description(None, film("Other.mov", 0).as_ref(), &mut NeverFires);
        assert_eq!(
            session.run.loaded_movie, None,
            "a missing dësc empties the MovieFile (@0x71a67-0x71a77)"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "Ad",
            "and keeps the governments (@0x71a5e skips @0x71926)"
        );
        let _ = session.load_description(Some("Plain."), None, &mut NeverFires);
        assert_eq!(session.run.loaded_movie, None, "a dësc that names none");
        let _ = session.load_description(Some("Again."), movie.as_ref(), &mut NeverFires);
        let _ = session.peek_desc(Some("Peeked."));
        assert_eq!(session.run.loaded_movie, movie, "peeking loads nothing");
    }

    #[test]
    fn a_no_room_text_in_an_offer_replays_the_offers_after_movie() {
        let catalog = screens(
            vec![heavy(142)],
            vec![offer_filmed(142, film("After.mov", 0x0001))],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert!(
            session
                .accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        assert_eq!(
            queued(&mut session),
            [
                no_holds(142, film("After.mov", 0x0001)),
                movie_shown("After.mov", 0x0001),
            ],
            "the STR# text's after-check (@0x5b40b) reads the offer's (@0xa226e), \
             and the offer's after-check plays it again (@0xa3580)"
        );
    }

    #[test]
    fn by_the_other_reading_the_no_room_text_plays_no_movie() {
        let catalog = screens(
            vec![heavy(142)],
            vec![offer_filmed(142, film("After.mov", 0x0001))],
        );
        let mut session = docked(&catalog).with_rules(globals(RuleSource::Bible));
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert!(
            session
                .accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        assert_eq!(
            queued(&mut session),
            [no_holds(142, None), movie_shown("After.mov", 0x0001)]
        );
    }

    #[test]
    fn a_before_movie_never_follows_a_no_room_text() {
        let catalog = screens(
            vec![heavy(142)],
            vec![offer_filmed(142, film("Before.mov", 0))],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert!(
            session
                .accept_mission(MissionId(142), &mut NeverFires)
                .is_err()
        );
        assert_eq!(
            queued(&mut session),
            [no_holds(142, None)],
            "a STR# text's before-check is gated off (@0x5a76c), and Flags 0 has no after-movie"
        );
    }

    #[test]
    fn an_offer_accepted_at_once_with_no_room_carries_its_after_movie() {
        let unrefusable = MissionRecord {
            flags: UNREFUSABLE,
            ..heavy(142)
        };
        let catalog = screens(
            vec![unrefusable],
            vec![MissionTexts {
                offer: Some(String::new()),
                ..offer_filmed(142, film("After.mov", 0x0001))
            }],
        );
        let mut session = docked(&catalog);
        assert!(matches!(
            session.next_offer(BAR, &mut NeverFires),
            Some(VenueOffer::Accepted {
                accepted: Err(_),
                ..
            })
        ));
        assert_eq!(
            queued(&mut session),
            [no_holds(142, film("After.mov", 0x0001))],
            "the offer dësc is loaded before the empty-text check (@0xa226e), and the accept \
             at once returns before the offer's after-check (@0xa230a-0xa231c)"
        );
    }

    /// Mission 142's texts: offer movie `offer`, and a `RefuseText` whose
    /// `dësc` names `refuse`.
    fn refusing(offer: Option<DescMovie>, refuse: Option<DescMovie>) -> MissionTexts {
        MissionTexts {
            refuse: Some("No.".to_owned()),
            refuse_named: true,
            movies: TextMovies {
                offer,
                refuse,
                ..TextMovies::default()
            },
            ..offering(142, "Ferry.", "", "")
        }
    }

    fn refuse_shown(movie: Option<DescMovie>) -> MissionShown {
        MissionShown::Text(MissionText {
            movie,
            ..text(MissionTextKind::Refuse, 142, "No.")
        })
    }

    #[test]
    fn a_refuse_texts_after_movie_stands_in_for_the_offers() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![refusing(film("A.mov", 0x0001), film("R.mov", 0x0001))],
        );
        for (source, last) in [
            (RuleSource::Engine, movie_shown("R.mov", 0x0001)),
            (RuleSource::Bible, movie_shown("A.mov", 0x0001)),
        ] {
            let mut session = docked(&catalog).with_rules(globals(source));
            prompted(session.next_offer(BAR, &mut NeverFires));
            session
                .refuse_mission(MissionId(142), &mut NeverFires)
                .expect("refused");
            assert_eq!(
                queued(&mut session),
                [refuse_shown(film("R.mov", 0x0001)), last],
                "{source:?}: nothing reloads the offer after the refuse text (@0xa2eb5-0xa2f4d)"
            );
        }
    }

    #[test]
    fn a_refuse_text_with_a_before_movie_leaves_the_offer_none() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![refusing(film("A.mov", 0x0001), film("R.mov", 0))],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("refused");
        assert_eq!(queued(&mut session), [refuse_shown(film("R.mov", 0))]);
    }

    #[test]
    fn a_refuse_text_naming_a_missing_desc_empties_the_movie() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                refuse: None,
                ..refusing(film("A.mov", 0x0001), None)
            }],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(
            session.refuse_mission(MissionId(142), &mut NeverFires),
            Ok(RefuseShown { offer_hidden: true }),
            "the test is RefuseText != -1 (@0xa2e70)"
        );
        assert_eq!(
            queued(&mut session),
            [],
            "the empty text shows nothing (@0x5a7b0), and the missing dësc emptied the \
             MovieFile (@0xa2eb5, @0x71a67)"
        );
    }

    #[test]
    fn a_refuse_without_a_refuse_text_keeps_the_offer_shown() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![offer_filmed(142, film("A.mov", 0x0001))],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(
            session.refuse_mission(MissionId(142), &mut NeverFires),
            Ok(RefuseShown {
                offer_hidden: false
            }),
            "hidden only for a refuse text (@0xa2e8e)"
        );
        assert_eq!(queued(&mut session), [movie_shown("A.mov", 0x0001)]);
    }

    #[test]
    fn a_refuse_with_a_refuse_text_hides_the_offer() {
        let catalog = screens(vec![at_the_bar(142)], vec![refusing(None, None)]);
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        assert_eq!(
            session.refuse_mission(MissionId(142), &mut NeverFires),
            Ok(RefuseShown { offer_hidden: true })
        );
    }

    #[test]
    fn an_accept_reloads_the_offer_desc() {
        let mut session = ranked(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                brief: Some("Earlier: <PRK128> <SRK129>.".to_owned()),
                movies: TextMovies {
                    offer: film("A.mov", 0x0001),
                    brief: film("B.mov", 0x0001),
                    ..TextMovies::default()
                },
                ..offering(142, "Ferry.", "", "")
            }],
            Rulebook::default(),
        );
        prompted(session.next_offer(BAR, &mut NeverFires));
        session
            .accept_mission(MissionId(142), &mut NeverFires)
            .expect("accepted");
        let shown = queued(&mut session);
        assert!(
            matches!(
                &shown[..],
                [
                    MissionShown::Text(MissionText {
                        kind: MissionTextKind::Brief,
                        movie: Some(brief),
                        ..
                    }),
                    MissionShown::Movie(offer),
                ] if brief.file == "B.mov" && offer.file == "A.mov"
            ),
            "the offer dësc is reloaded after the accept (@0xa2e29), before the after-check: \
             {shown:?}"
        );
        assert_eq!(
            session.plain_text("<PRK128>"),
            "<PRK128>",
            "the reload leaves the offer's governments, none, where the BriefText left 129's"
        );
    }

    #[test]
    fn the_computers_no_room_text_reads_the_selected_rows_offer() {
        let catalog = screens(
            vec![MissionRecord {
                cargo_type: 0,
                cargo_qty: 10_000,
                ..to_mars(144)
            }],
            vec![MissionTexts {
                offer: Some("Ferry.".to_owned()),
                movies: TextMovies {
                    offer: film("M.mov", 0x0001),
                    ..TextMovies::default()
                },
                ..MissionTexts::named(MissionId(144), "Run")
            }],
        );
        let mut session = docked(&catalog);
        let _ = session.show_computer_row(MissionId(144), &mut NeverFires);
        assert!(
            session
                .accept_mission(MissionId(144), &mut NeverFires)
                .is_err()
        );
        assert_eq!(
            queued(&mut session),
            [no_holds(144, film("M.mov", 0x0001))],
            "the selected row's offer (@0x9f9bd); the computer has no after-check"
        );
    }

    #[test]
    fn the_info_row_loads_its_quick_brief_movie() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![MissionTexts {
                quick_brief: Some("Quick.".to_owned()),
                movies: TextMovies {
                    quick_brief: film("Q.mov", 0x0001),
                    ..TextMovies::default()
                },
                ..offering(142, "Ferry.", "", "")
            }],
        );
        let mut session = docked(&catalog);
        prompted(session.next_offer(BAR, &mut NeverFires));
        session
            .accept_mission(MissionId(142), &mut NeverFires)
            .expect("accepted");
        let _ = session.show_info_row(0, &mut NeverFires);
        assert_eq!(session.run.loaded_movie, film("Q.mov", 0x0001));
    }

    #[test]
    fn reloading_the_offer_desc_restores_its_movie() {
        let catalog = screens(
            vec![at_the_bar(142)],
            vec![
                MissionTexts {
                    quick_brief: Some("Quick.".to_owned()),
                    movies: TextMovies {
                        quick_brief: film("Q.mov", 0x0001),
                        ..TextMovies::default()
                    },
                    ..MissionTexts::named(MissionId(150), "Held")
                },
                offer_filmed(142, film("A.mov", 0x0001)),
            ],
        );
        let mut session = docked(&catalog);
        let offer = Offer {
            mission: MissionId(150),
            ..session.mission_offers(Venue::Outside)[0].clone()
        };
        let active = ActiveMission::accepted(&mission(150), &offer, session.date());
        session.pilot.missions.fill(active);
        prompted(session.next_offer(BAR, &mut NeverFires));
        let _ = session.show_info_row(0, &mut NeverFires);
        assert_eq!(
            session.run.loaded_movie,
            film("Q.mov", 0x0001),
            "the QuickBrief's"
        );
        session.reload_offer_desc(MissionId(142), &mut NeverFires);
        assert_eq!(
            session.run.loaded_movie,
            film("A.mov", 0x0001),
            "the info screen's close reloads the offer (@0xa335c)"
        );
        session
            .refuse_mission(MissionId(142), &mut NeverFires)
            .expect("refused");
        assert_eq!(queued(&mut session), [movie_shown("A.mov", 0x0001)]);
    }

    #[test]
    fn loaded_after_movie_by_each_reading() {
        for (source, flags, after) in [
            (RuleSource::Engine, 0x0001, film("L.mov", 0x0001)),
            (RuleSource::Engine, 0, None),
            (RuleSource::Bible, 0x0001, None),
        ] {
            let mut session = docked(&screens(Vec::new(), Vec::new())).with_rules(globals(source));
            let _ = session.load_description(
                Some("Loaded."),
                film("L.mov", flags).as_ref(),
                &mut NeverFires,
            );
            assert_eq!(session.loaded_after_movie(), after, "{source:?} {flags}");
        }
    }
}
