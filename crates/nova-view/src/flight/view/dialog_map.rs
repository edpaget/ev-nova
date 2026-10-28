//! [`FlightView`]'s map opened by a mission dialog: an offer, the mission
//! computer, mission info or the accept's briefing text, each on the map
//! key ([`FlightView::open_dialog_map`]), and who opened the map shown
//! ([`MapOpening`]), the port screens' maps among them
//! ([`FlightView::open_port_screen_map`]). All addresses are in
//! `EV Nova.app`'s `EV Nova` (i386).
//!
//! It is flight's course map, over the spaceport docked and over flight
//! undocked, and everything the course map does applies unchanged: Find,
//! Space, B, Tab, zoom and scroll, clicks and Clear Route, and the close
//! by Done, Return, Enter, Escape or M ([`FlightView::port_map_input`]).
//! What sets it apart:
//!
//! - **Its marks**: the green mission system and the arrow flag its
//!   opener gives it (`nova_sim::Session::offer_map_marks` and its kin;
//!   the `mission_map` module of the session), drawn and taken by the
//!   click as a mission system is (@0x12f82-0x12f8a). They come with the
//!   map's opening ([`ScreenAsk::MapOpening`], its [`MarksFrom`] the
//!   opener's, [`MapOpener::marks_from`]); a flight that answers the
//!   opening nothing (one that failed to start) marks only what the
//!   briefing gives itself, and an offer's, the computer's and mission
//!   info's maps none, as such a flight's marks were none
//!   ([`MarksFrom::unanswered`]). Every other map opens with none
//!   ([`MapMarks::NONE`]), as `_InitMissions` and `_ResetPlayer` leave
//!   them, so the marks never outlive their map.
//! - **No sound**: neither the opening nor the close beeps. The
//!   [`UiSound::MapOpened`](crate::UiSound::MapOpened) and
//!   [`UiSound::MapClosed`](crate::UiSound::MapClosed) beeps are
//!   `_HandlePlayer`'s, around M in flight (@0x6a809, @0x6a85a), and no
//!   dialog calls `PlaySound` around its map.
//! - **The opener's restore**, after the map's own close
//!   ([`FlightView::close_map`]): the offer (@0xa3108-0xa3110), the
//!   computer (@0xa3d86-0xa3d9e) and the text dialog (@0x5b169-0x5b17d)
//!   write back the nav saved before the map ([`Session::restore_map_nav`](nova_sim::Session::restore_map_nav)),
//!   over a plain click's mode 3 in flight or the close's cue; the course
//!   stays. The computer then redraws the nav area (`_DrawStatus`,
//!   @0xa3da2-0xa3daa). Mission info's restore is dead (@0x9e45f-0x9e476),
//!   so its map's nav writes stand.
//!
//! Left out, record only: the `FlushEvents` before the offer's and the
//! computer's map (@0xa3009-0xa3054, @0xa3c85-0xa3cd0), as ours queues no
//! events past each screen's handling; the callers' invalidations and
//! port redraws after the map, as ours draws every frame whole; and the
//! `_mapDialog` guards (@0x98dc0, @0x9f80a), which nothing can trip.

#[cfg(doc)]
use nova_sim::flight_frame::ScreenAsk;
use nova_sim::flight_frame::{FlightCommand, MarksFrom};
use nova_sim::{MapMarks, MissionId, SavedNav};

use super::port_screen_map::{self, PortScreen};
use super::{FlightView, MAP_KEY};
use crate::Input;
use crate::flight::sim_port::FlightSim;

/// Whether `input` is a press of [`MAP_KEY`], never a repeat: how each
/// mission dialog asks for the map, as the bar does (`_QuickKeyCheck` of
/// `_keyPrefs` +0x12).
#[must_use]
pub(crate) fn is_map_press(input: &Input) -> bool {
    *input
        == Input::Key {
            key: MAP_KEY,
            pressed: true,
            repeat: false,
        }
}

/// The mission dialog that asks for the map, and what it reads for the
/// map's marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapOpener {
    /// An offer of the mission (`_OfferOneMission` item 4, @0x121a3).
    Offer(MissionId),
    /// The mission computer, with its selected row's offer, if any
    /// (`_DoMissionDialog` item 6).
    Computer(Option<MissionId>),
    /// Mission info, with the slot of its selected mission, if any
    /// (`_DoMissionInfoDialog` item 6, @0x9e45a).
    MissionInfo(Option<u8>),
    /// The accept's briefing text, with the marks the session set on it as
    /// it queued it ([`MissionText::map`](nova_sim::MissionText::map),
    /// `_ShowTextDialog` item 4, @0x5b164).
    Briefing(MapMarks),
}

impl MapOpener {
    /// Where the map's marks come from, for its opening's ask: the
    /// briefing's own, or the session's for the offer, the computer's row
    /// or mission info's slot.
    #[must_use]
    pub fn marks_from(self) -> MarksFrom {
        match self {
            Self::Offer(mission) => MarksFrom::Offer(mission),
            Self::Computer(selected) => MarksFrom::Computer(selected),
            Self::MissionInfo(slot) => MarksFrom::MissionInfo(slot),
            Self::Briefing(marks) => MarksFrom::Given(marks),
        }
    }
}

/// Who opened the map shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MapOpening {
    /// M in flight, or the hypergate: no dialog's.
    Plain,
    /// The bar (`_DoBarDialog` item 8).
    Bar,
    /// A mission dialog.
    Dialog(MapOpener),
    /// A port screen ([`FlightView::open_port_screen_map`]).
    Port(PortScreen),
}

impl MapOpening {
    /// Where the map's marks come from: its dialog's opener, or none
    /// given ([`MapMarks::NONE`]) for every other map.
    pub(super) fn marks_from(self) -> MarksFrom {
        match self {
            Self::Dialog(opener) => opener.marks_from(),
            Self::Plain | Self::Bar | Self::Port(_) => MarksFrom::Given(MapMarks::NONE),
        }
    }
}

/// How the map shown was opened: who opened it, the nav as it opened,
/// which `_DoSystemMap` saves (@0x12349) and its opener too, and its
/// marks, each as its opening's answer gave them. Taken afresh as each
/// map opens and dropped as it closes, so neither the marks nor a restore
/// outlives its map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MapOpened {
    pub(super) by: MapOpening,
    pub(super) nav: Option<SavedNav>,
    pub(super) marks: MapMarks,
}

impl MapOpened {
    /// The marks of the map: those it opened with, or none with no map.
    pub(super) fn marks(opened: Option<&Self>) -> MapMarks {
        opened.map_or(MapMarks::NONE, |opened| opened.marks)
    }
}

/// The dialog maps, over the [`FlightSim`] port: their marks and the
/// openers' restores go through the flight.
impl<C, S: FlightSim> FlightView<C, S> {
    /// Whether the map shown is a mission dialog's
    /// ([`FlightView::open_dialog_map`]): its input is
    /// [`FlightView::port_map_input`]'s, in port and over flight alike,
    /// before anything open under it.
    #[must_use]
    pub fn dialog_map_open(&self) -> bool {
        self.map_open
            && self
                .map_opened
                .is_some_and(|opened| matches!(opened.by, MapOpening::Dialog(..)))
    }

    /// The restore of the map's opener, after its close (see the module
    /// docs): the bar's ([`Session::restore_bar_nav`](nova_sim::Session::restore_bar_nav)), a port screen's
    /// ([`PortScreen`]), or a mission dialog's.
    pub(super) fn restore_opener_nav(&mut self, opened: Option<MapOpened>) {
        let Some(opened) = opened else {
            return;
        };
        match opened.by {
            MapOpening::Plain | MapOpening::Dialog(MapOpener::MissionInfo(_)) => {}
            MapOpening::Bar => {
                self.send(FlightCommand::RestoreBarNav);
            }
            MapOpening::Port(screen) => {
                if let Some(command) = port_screen_map::restore_command(screen, &opened) {
                    self.send(command);
                }
            }
            MapOpening::Dialog(dialog) => {
                if let Some(nav) = opened.nav {
                    self.send(FlightCommand::RestoreMapNav(nav));
                }
                if matches!(dialog, MapOpener::Computer(_)) {
                    self.retake_nav();
                }
            }
        }
    }

    /// Opens the course map for `opener`, as each mission dialog calls
    /// `_DoSystemMap` on its map key: in
    /// [`MapMode::Course`](crate::galaxy::MapMode::Course), as each clears
    /// the hypergate flag first (@0xa3100, @0xa3d69, @0x9e457, @0x5b161),
    /// marked as the session says for it with its opening
    /// ([`MapOpener::marks_from`]), silently. Its close is
    /// [`FlightView::close_map`]'s, then the opener's restore (see the
    /// module docs).
    pub fn open_dialog_map(&mut self, opener: MapOpener) {
        self.open_map_as(MapOpening::Dialog(opener));
    }
}
