//! The screen query's values: what a screen of the flight view asks the
//! session when it opens or when its input changes ([`ScreenAsk`]), and
//! the plain-data snapshot it draws from ([`ScreenAnswer`]), which
//! [`Session::screen`](crate::Session::screen) answers. Nothing here is
//! asked per frame: a per-frame read is a frame field
//! ([`FlightFrame`](super::FlightFrame)).
//!
//! Each ask has its answer of the same kind
//! ([`ScreenAnswer::answers`]), and [`ScreenAnswer::Nothing`] answers any
//! of them: a flight that failed to start answers every ask so, and a
//! session never shared a catalog the asks that read one.
//!
//! The asks one screen always makes together are merged into one answer:
//! the course map's opening ([`ScreenAsk::MapOpening`], [`MapOpen`]), its
//! redraw ([`ScreenAsk::Map`], [`MapScreen`]) and the floating map's show
//! ([`ScreenAsk::FloatingMap`], [`FloatingMap`]).

use crate::MissionId;
use crate::catalog::{ShipId, SystemId};
use crate::galaxy_map::{GalaxyShown, MapDrawing, MapMarks};
use crate::map_borders::BorderSources;
use crate::map_panel::MapPanel;
use crate::map_search::SearchOutcome;
use crate::session::SavedNav;

/// What a screen asks the session as it opens or as its input changes
/// ([`Session::screen`](crate::Session::screen)), answered by the
/// [`ScreenAnswer`] of its kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenAsk {
    /// What the course map's model shows of the galaxy
    /// ([`GalaxyShown::of`]), asked as the view opens and at each
    /// `SessionEvent::GalaxyChanged`: [`ScreenAnswer::Galaxy`].
    Galaxy,
    /// Everything the course map reads as it opens, its marks taken as
    /// `marks` says: [`ScreenAnswer::MapOpening`].
    MapOpening {
        /// Where the map's marks come from.
        marks: MarksFrom,
    },
    /// What the course map draws at a redraw, with the panels for
    /// `selected`: on the hypergate map when `hypergate` holds the systems
    /// its gate offers, `missions` the mission systems taken as it opened
    /// and `marks` its opener's ([`ScreenAnswer::Map`]). Answered nothing
    /// without a catalog, as the panels read one.
    Map {
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
        /// The mission systems taken as the map opened.
        missions: Vec<SystemId>,
        /// The opener's marks.
        marks: MapMarks,
        /// The system the map selects.
        selected: Option<SystemId>,
    },
    /// The course map's panels alone, for `selected` after a selection
    /// change, on the hypergate map with its offers
    /// ([`ScreenAnswer::MapPanel`]). Answered nothing without a catalog.
    MapPanel {
        /// The system the map selects.
        selected: Option<SystemId>,
        /// The hypergate map's offers, or `None` on the course map.
        hypergate: Option<Vec<SystemId>>,
    },
    /// The system the course map selects after its Tab / `\` cycle
    /// ([`ScreenAnswer::MapPreselection`]).
    MapPreselection,
    /// The destinations the hypergate map's Tab / `\` cycles through
    /// ([`ScreenAnswer::HypergateCycle`]). Answered nothing without a
    /// catalog, as the gate sites are the catalog's.
    HypergateCycle,
    /// What the map's Find finds for its query ([`ScreenAnswer::Found`]).
    Find(String),
    /// What the floating map draws at a show ([`ScreenAnswer::FloatingMap`]).
    ///
    /// Asked at each show, and a show comes on every frame while H or `\`
    /// is held, as `_ShowFloatingMap` draws its frame image at each show
    /// (`_DrawMapInFloatingFrame`). Its drawing can change between two
    /// held frames with no command from the view: the flight steps
    /// meanwhile, so a mission ship destroyed takes its system's red
    /// arrow off (`Session::map_mission_systems`
    /// reads the ships left), a jump arrived at moves the current system
    /// and its links, and a step's script can change the mode-3 selection
    /// the preselection reads. None of those is in the frame's
    /// [`MapKey`](super::MapKey), so the view can't tell when to ask
    /// again; it asks at each show.
    FloatingMap,
    /// The ship types the system's next traffic is drawn from, asked once
    /// as an entry warms the view's sheets up
    /// ([`ScreenAnswer::ComingTraffic`]). Answered nothing without a
    /// catalog, as the spawn table it resolves reads one.
    ComingTraffic,
    /// The text with its tags filled for the pilot flying, asked as a
    /// screen's text is filled ([`ScreenAnswer::PlainText`]).
    PlainText(String),
}

/// Where a course map's marks come from ([`ScreenAsk::MapOpening`]): the
/// mission dialog that opened it, or marks its opener gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarksFrom {
    /// Marks the opener gives itself: [`MapMarks::NONE`] for the plain
    /// map, the bar's and the port screens', or the briefing's own.
    Given(MapMarks),
    /// An offer of the mission
    /// ([`Session::offer_map_marks`](crate::Session::offer_map_marks)).
    Offer(MissionId),
    /// The mission computer, with its selected row's offer
    /// ([`Session::computer_map_marks`](crate::Session::computer_map_marks)).
    Computer(Option<MissionId>),
    /// Mission info, with its selected mission's slot
    /// (`Session::mission_info_map_marks`).
    MissionInfo(Option<u8>),
}

impl MarksFrom {
    /// The marks a map opened by a flight with nothing to answer shows:
    /// those given, otherwise none ([`MapMarks::NONE`]), as a failed
    /// flight's dialogs gave none.
    #[must_use]
    pub fn unanswered(self) -> MapMarks {
        match self {
            Self::Given(marks) => marks,
            Self::Offer(_) | Self::Computer(_) | Self::MissionInfo(_) => MapMarks::NONE,
        }
    }
}

/// What the course map reads as it opens ([`ScreenAsk::MapOpening`]), all
/// asked with no command between them, as `_DoSystemMap` reads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapOpen {
    /// The nav as it stands, which the map's close and its opener put
    /// back (`Session::map_nav`).
    pub nav: SavedNav,
    /// The map's marks, as [`MarksFrom`] said.
    pub marks: MapMarks,
    /// What the borders are painted from
    /// ([`Session::map_border_sources`](crate::Session::map_border_sources)).
    pub borders: BorderSources,
    /// The system the map selects as it opens
    /// (`Session::map_preselection`).
    pub preselection: Option<SystemId>,
    /// The mission systems its red arrows mark
    /// (`Session::map_mission_systems`).
    pub missions: Vec<SystemId>,
}

/// What the course map shows at a redraw ([`ScreenAsk::Map`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapScreen {
    /// What it draws
    /// ([`Session::map_drawing`](crate::Session::map_drawing)).
    pub drawing: MapDrawing,
    /// The systems drawing it makes known, for the view to mark known
    /// (`FlightCommand::MarkKnown`) right after it shows the drawing, as
    /// `_DrawMap` marks each hypergate arrow's system known as it draws
    /// it (@0xf457-0xf45f): on the hypergate map the drawing's gate
    /// arrows, otherwise none. The session decides them; the view only
    /// forwards them, and sends no mark when there are none.
    pub marks_known: Vec<SystemId>,
    /// What its borders are painted from: the known set counting
    /// [`MapScreen::marks_known`] as known, as the borders are shown after
    /// the mark.
    pub borders: BorderSources,
    /// Its info panels for the system selected
    /// ([`Session::map_panel`](crate::Session::map_panel)).
    pub panel: MapPanel,
}

/// What the floating map draws at a show ([`ScreenAsk::FloatingMap`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FloatingMap {
    /// The drawing off the hypergate map, with the mission systems taken
    /// now and no marks ([`MapMarks::NONE`]).
    pub drawing: MapDrawing,
    /// The system it selects
    /// (`Session::floating_preselection`).
    pub preselection: Option<SystemId>,
}

/// What the session answered a [`ScreenAsk`], each of the ask's own kind
/// ([`ScreenAnswer::answers`]), or [`ScreenAnswer::Nothing`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenAnswer {
    /// Nothing to answer: a flight that failed to start, or an ask that
    /// reads a catalog the session was never shared.
    Nothing,
    /// [`ScreenAsk::Galaxy`]'s.
    Galaxy(GalaxyShown),
    /// [`ScreenAsk::MapOpening`]'s.
    MapOpening(MapOpen),
    /// [`ScreenAsk::Map`]'s.
    Map(Box<MapScreen>),
    /// [`ScreenAsk::MapPanel`]'s.
    MapPanel(Box<MapPanel>),
    /// [`ScreenAsk::MapPreselection`]'s.
    MapPreselection(Option<SystemId>),
    /// [`ScreenAsk::HypergateCycle`]'s.
    HypergateCycle(Vec<Option<SystemId>>),
    /// [`ScreenAsk::Find`]'s.
    Found(SearchOutcome),
    /// [`ScreenAsk::FloatingMap`]'s.
    FloatingMap(Box<FloatingMap>),
    /// [`ScreenAsk::ComingTraffic`]'s.
    ComingTraffic(Vec<ShipId>),
    /// [`ScreenAsk::PlainText`]'s.
    PlainText(String),
}

impl ScreenAnswer {
    /// Whether it answers `ask`: [`ScreenAnswer::Nothing`] answers any,
    /// otherwise only the answer of the ask's own kind.
    #[must_use]
    pub fn answers(&self, ask: &ScreenAsk) -> bool {
        if *self == Self::Nothing {
            return true;
        }
        match ask {
            ScreenAsk::Galaxy => matches!(self, Self::Galaxy(_)),
            ScreenAsk::MapOpening { .. } => matches!(self, Self::MapOpening(_)),
            ScreenAsk::Map { .. } => matches!(self, Self::Map(_)),
            ScreenAsk::MapPanel { .. } => matches!(self, Self::MapPanel(_)),
            ScreenAsk::MapPreselection => matches!(self, Self::MapPreselection(_)),
            ScreenAsk::HypergateCycle => matches!(self, Self::HypergateCycle(_)),
            ScreenAsk::Find(_) => matches!(self, Self::Found(_)),
            ScreenAsk::FloatingMap => matches!(self, Self::FloatingMap(_)),
            ScreenAsk::ComingTraffic => matches!(self, Self::ComingTraffic(_)),
            ScreenAsk::PlainText(_) => matches!(self, Self::PlainText(_)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One ask of each kind.
    fn asks() -> Vec<ScreenAsk> {
        vec![
            ScreenAsk::Galaxy,
            ScreenAsk::MapOpening {
                marks: MarksFrom::Given(MapMarks::NONE),
            },
            ScreenAsk::Map {
                hypergate: None,
                missions: Vec::new(),
                marks: MapMarks::NONE,
                selected: None,
            },
            ScreenAsk::MapPanel {
                selected: None,
                hypergate: None,
            },
            ScreenAsk::MapPreselection,
            ScreenAsk::HypergateCycle,
            ScreenAsk::Find("Sol".to_owned()),
            ScreenAsk::FloatingMap,
            ScreenAsk::ComingTraffic,
            ScreenAsk::PlainText("Hi.".to_owned()),
        ]
    }

    /// A drawing of nothing, in 130.
    fn drawing() -> MapDrawing {
        MapDrawing {
            hypergate: false,
            course: Vec::new(),
            current_links: Vec::new(),
            links: Vec::new(),
            mode3_link: None,
            systems: Vec::new(),
            current: SystemId(130),
            gate_arrows: Vec::new(),
            named: Vec::new(),
        }
    }

    /// The answer of each kind, in [`asks`]' order.
    fn answers() -> Vec<ScreenAnswer> {
        let panel = MapPanel {
            strip: crate::map_panel::Strip::HypergatePrompt {
                prompt: String::new(),
                date: String::new(),
            },
            column: crate::map_panel::Column {
                title: String::new(),
                body: crate::map_panel::ColumnBody::Unknown,
            },
            words: crate::map_panel::PanelWords::default(),
        };
        vec![
            ScreenAnswer::Galaxy(GalaxyShown::default()),
            ScreenAnswer::MapOpening(MapOpen {
                nav: SavedNav::of(crate::navigation::NavMode::Off),
                marks: MapMarks::NONE,
                borders: BorderSources::default(),
                preselection: None,
                missions: Vec::new(),
            }),
            ScreenAnswer::Map(Box::new(MapScreen {
                drawing: drawing(),
                marks_known: Vec::new(),
                borders: BorderSources::default(),
                panel: panel.clone(),
            })),
            ScreenAnswer::MapPanel(Box::new(panel)),
            ScreenAnswer::MapPreselection(None),
            ScreenAnswer::HypergateCycle(Vec::new()),
            ScreenAnswer::Found(SearchOutcome::Found(SystemId(130))),
            ScreenAnswer::FloatingMap(Box::new(FloatingMap {
                drawing: drawing(),
                preselection: None,
            })),
            ScreenAnswer::ComingTraffic(Vec::new()),
            ScreenAnswer::PlainText(String::new()),
        ]
    }

    #[test]
    fn an_answer_answers_only_an_ask_of_its_own_kind_and_nothing_answers_any() {
        let (asks, answers) = (asks(), answers());
        for (at, answer) in answers.iter().enumerate() {
            for (to, ask) in asks.iter().enumerate() {
                assert_eq!(answer.answers(ask), at == to, "{answer:?} to {ask:?}");
            }
        }
        for ask in &asks {
            assert!(ScreenAnswer::Nothing.answers(ask), "{ask:?}");
        }
    }

    #[test]
    fn a_map_unanswered_keeps_only_the_marks_given() {
        let green = MapMarks {
            green: Some(SystemId(131)),
            multi_arrows: false,
        };
        assert_eq!(MarksFrom::Given(green).unanswered(), green);
        for from in [
            MarksFrom::Offer(MissionId(128)),
            MarksFrom::Computer(Some(MissionId(128))),
            MarksFrom::MissionInfo(Some(0)),
        ] {
            assert_eq!(from.unanswered(), MapMarks::NONE, "{from:?}");
        }
    }
}
