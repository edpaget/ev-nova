//! [`SessionFlight`]: the [`FlightSim`] adapter over a real
//! `nova_sim::Session`, with the collaborators the session's calls are
//! handed: the chance, the NPCs' behaviour and the fight's, the law's and
//! the boardings' rules, and the comm dialog's options.
//!
//! It forwards: [`FlightSim::frame`] to `Session::fill_frame`,
//! [`FlightSim::hover`] to `Session::hover`, [`FlightSim::coming_traffic`]
//! to `Session::coming_traffic_ships`, [`FlightSim::death_descs`] to
//! `Session::sabotage_death_descs`,
//! [`FlightSim::step`] to `Session::step`, with its collaborators and
//! the catalog the view shared,
//! [`FlightSim::apply`] to `Session::apply`, whose match over the
//! commands lives on the session's side, with the catalog the view
//! shared, the chance, the law, the boarding rule and the hail options,
//! [`FlightSim::drain`] to `Session::drain_events`, [`CommSource`]'s to
//! `Session::hailing` and `Session::boarding`, and [`PortSource`]'s,
//! [`NameSource`]'s and [`PilotSource`]'s to the session's method of each
//! name. A session that failed to start
//! fills the default frame, runs no step, refuses every command, answers
//! nothing open and drains nothing, and says why
//! ([`FlightSim::failure`]).
//!
//! **The edge's API**, which the port leaves out, is the adapter's own:
//! the constructors that build the session ([`SessionFlight::start`],
//! [`SessionFlight::fly`], [`SessionFlight::load`]), its setup and the
//! run's history ([`SessionFlight::set_up`], [`SessionFlight::carry_over`],
//! one `Session` call each), entering play ([`SessionFlight::begin`],
//! [`SessionFlight::enter_game`], [`SessionFlight::resume`],
//! [`SessionFlight::open_on_main_screen`], [`SessionFlight::open_at_launch`]),
//! saving ([`SessionFlight::may_save`], [`SessionFlight::is_dying`],
//! [`SessionFlight::deletes_pilot_on_ending_play`],
//! [`SessionFlight::load_repaired`], [`SessionFlight::loaded_after_movie`])
//! and the pilot the app saves and names ([`SessionFlight::pilot`]). The
//! app reaches them through `FlightView::sim`. `SessionFlight::transact`,
//! which takes a closure no command can carry, is for tests. The adapter
//! is the developer tools' desk too ([`DevDesk`]).
//!
//! **For the strangler's length**, the adapter's fields are visible to
//! `crate::flight`: the fixture's view over a session it built
//! (`FlightView::from_session`) and the test conveniences read the
//! chance and share the catalog through them, and the view's tests put
//! the session and the collaborators into the state they need. Other
//! code reads the session through [`SessionFlight::session`]; tests change
//! it through `SessionFlight::session_mut`.

use std::marker::PhantomData;
use std::rc::Rc;

use nova_sim::flight_frame::{
    Answer, DrawnBoxes, FlightCommand, FlightEvent, FlightFrame, FloatingLinks, StepInput,
    StepReport,
};
use nova_sim::galaxy_map::MapDrawing;
use nova_sim::map_borders::BorderSources;
use nova_sim::map_panel::MapPanel;
use nova_sim::map_search::SearchOutcome;
use nova_sim::pilot_panel::PilotPanel;
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{
    Allegiance, ApplyCollaborators, Behaviour, BoardingRule, DescMovie, DisableRule, Good,
    HailOptions, HailView, HireList, Hover, JunkId, LegalCode, MapMarks, Market, MissionId,
    MissionInfo, News, NovaAi, NovaBoarding, NovaDisable, NovaLaw, OutfitId, Outfitter, Pilot,
    PlayerCommand, PlayerInfo, PlunderView, PointDefenceRule, Rules, SavedNav, Session, ShipId,
    ShipNaming, ShipRefusal, Shipyard, SimCatalog, StellarId, StepCollaborators, SystemId,
    TagNames, Vec2,
};

use nova_sim::{
    AsteroidCatalog, CarryOver, CombatCatalog, DeskReading, RelocateRefusal, Rulebook,
    SessionSetup, StartError,
};

use super::SharedChance;
use super::sim_port::{CommSource, FlightSim, MapSource, NameSource, PilotSource, PortSource};
use crate::devtools::{DevDesk, EditRefusal, PilotEdit};

/// The flight of a real `nova_sim::Session`, or why it could not start,
/// with the collaborators its calls are handed (see the module docs).
/// `C` is the catalog the view reads; it names the flight the view's
/// default `FlightView<C>` shows. The view shares its handle on it with
/// the flight as it is read ([`SessionFlight::share_catalog`]). A clone
/// shares the catalog and the collaborators, and copies the session.
#[derive(Debug)]
pub struct SessionFlight<C> {
    /// The flight, or why it could not start.
    pub(in crate::flight) session: Result<Session, String>,
    /// What each day's events and the traffic are rolled on.
    pub(in crate::flight) chance: SharedChance,
    /// How the NPCs decide.
    pub(in crate::flight) behaviour: Rc<dyn Behaviour>,
    /// When a ship in the fight is disabled.
    pub(in crate::flight) disable_rule: Rc<dyn DisableRule>,
    /// Which missiles the ships' point defence engages.
    pub(in crate::flight) defence_rule: Rc<dyn PointDefenceRule>,
    /// What the player's crimes do to its legal record.
    pub(in crate::flight) law: Rc<dyn LegalCode>,
    /// Who repels boarders, the capture odds and the capture roll.
    pub(in crate::flight) boarding_rule: Rc<dyn BoardingRule>,
    /// The comm dialog's options, which the hail and its answers list.
    pub(in crate::flight) hail_options: HailOptions,
    /// The catalog the view reads, the same handle as the view's, once
    /// the view has shared it ([`SessionFlight::share_catalog`]): the
    /// map's answers that read the catalog read it. `None` before.
    catalog: Option<SharedCatalog>,
    /// `C` names the view's catalog: the flight the view's default
    /// `FlightView<C>` shows.
    view_catalog: PhantomData<C>,
}

// By hand: the derive would ask `C: Clone`, which only names the view's
// catalog.
impl<C> Clone for SessionFlight<C> {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            chance: self.chance.clone(),
            behaviour: Rc::clone(&self.behaviour),
            disable_rule: Rc::clone(&self.disable_rule),
            defence_rule: Rc::clone(&self.defence_rule),
            law: Rc::clone(&self.law),
            boarding_rule: Rc::clone(&self.boarding_rule),
            hail_options: self.hail_options.clone(),
            catalog: self.catalog.clone(),
            view_catalog: PhantomData,
        }
    }
}

/// What a [`SessionFlight`]'s calls are handed besides the catalog, as
/// the edge gives them ([`SessionFlight::with_collaborators`]): what each
/// day's events and the traffic are rolled on, how the NPCs decide, which
/// missiles point defence engages, the law, the boardings' rules and the
/// comm dialog's options. The fight's disable rule is the session's setup's
/// (`nova_sim::SessionSetup::disable_rule`, [`SessionFlight::set_up`]),
/// so the two read one. The default is Nova's: the never-firing chance,
/// Nova's AI, law and boardings, point defence by allegiance, and the
/// default hail options.
#[derive(Clone, Debug)]
pub struct Collaborators {
    /// What each day's events and the traffic are rolled on, and the
    /// view's wording with them.
    pub chance: SharedChance,
    /// How the NPCs decide.
    pub behaviour: Rc<dyn Behaviour>,
    /// Which missiles the ships' point defence engages.
    pub defence_rule: Rc<dyn PointDefenceRule>,
    /// What the player's crimes do to its legal record.
    pub law: Rc<dyn LegalCode>,
    /// Who repels boarders, the capture odds and the capture roll.
    pub boarding_rule: Rc<dyn BoardingRule>,
    /// The comm dialog's options.
    pub hail_options: HailOptions,
}

impl Default for Collaborators {
    fn default() -> Self {
        Self {
            chance: SharedChance::default(),
            behaviour: Rc::new(NovaAi::default()),
            defence_rule: Rc::new(Allegiance),
            law: Rc::new(NovaLaw::default()),
            boarding_rule: Rc::new(NovaBoarding::default()),
            hail_options: HailOptions::default(),
        }
    }
}

/// The view's catalog, as the flight reads it: as a `SimCatalog` only,
/// so the adapter's `FlightSim` asks nothing of `C`.
#[derive(Clone)]
struct SharedCatalog(Rc<dyn SimCatalog>);

impl std::fmt::Debug for SharedCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedCatalog")
    }
}

/// The edge's constructors: each builds the session from `catalog` and
/// shares the catalog with the flight at once, the handle the view will
/// hold ([`SessionFlight::share_catalog`]), so the flight's answers that
/// read the catalog read the one the view draws from. The flight has
/// Nova's collaborators ([`SessionFlight::new`]), which the edge replaces
/// with the `with_*` builders, and the session is set up afterwards
/// ([`SessionFlight::set_up`]), once the view has read it as it opens.
impl<C: SimCatalog + CombatCatalog + AsteroidCatalog + 'static> SessionFlight<C> {
    /// A new, unnamed pilot's flight (`nova_sim::Session::start`), or why
    /// it could not start.
    #[must_use]
    pub fn start(catalog: &Rc<C>) -> Self {
        Self::opened(catalog, Session::start(&**catalog))
    }

    /// `pilot`'s flight, as `nova_sim::Session::fly` builds it: for a
    /// pilot just made, or one opened with no load's fallbacks.
    #[must_use]
    pub fn fly(catalog: &Rc<C>, pilot: Pilot) -> Self {
        Self::opened(catalog, Session::fly(&**catalog, pilot))
    }

    /// `pilot`'s flight as `nova_sim::Session::load` builds it, its load's
    /// fallbacks reading `rules`: for a pilot opened from a save, whose
    /// repair [`SessionFlight::load_repaired`] reports.
    #[must_use]
    pub fn load(catalog: &Rc<C>, pilot: Pilot, rules: &Rulebook) -> Self {
        Self::opened(catalog, Session::load(&**catalog, pilot, rules))
    }

    /// The flight of `session` over `catalog`, shared.
    fn opened(catalog: &Rc<C>, session: Result<Session, StartError>) -> Self {
        let mut flight = Self::new(session.map_err(|err| err.to_string()));
        flight.share_catalog(Rc::clone(catalog) as Rc<dyn SimCatalog>);
        flight
    }
}

impl<C> SessionFlight<C> {
    /// The flight of `session`, or why it could not start, with Nova's
    /// collaborators: the never-firing chance, Nova's AI, disable rule,
    /// law and boardings, and point defence by allegiance.
    #[must_use]
    pub fn new(session: Result<Session, String>) -> Self {
        let Collaborators {
            chance,
            behaviour,
            defence_rule,
            law,
            boarding_rule,
            hail_options,
        } = Collaborators::default();
        Self {
            session,
            chance,
            behaviour,
            disable_rule: Rc::new(NovaDisable),
            defence_rule,
            law,
            boarding_rule,
            hail_options,
            catalog: None,
            view_catalog: PhantomData,
        }
    }

    /// Shares the view's catalog with the flight: the view's own handle,
    /// so the flight's answers read the catalog the view draws from, and
    /// a test that records a read on it sees the flight's too.
    pub(in crate::flight) fn share_catalog(&mut self, catalog: Rc<dyn SimCatalog>) {
        self.catalog = Some(SharedCatalog(catalog));
    }

    /// The catalog the view shared, if it has
    /// ([`SessionFlight::share_catalog`]).
    #[cfg(test)]
    pub(in crate::flight) fn shared_catalog(&self) -> Option<&Rc<dyn SimCatalog>> {
        self.catalog.as_ref().map(|shared| &shared.0)
    }

    /// The flight, or why it could not start.
    pub fn session(&self) -> Result<&Session, &str> {
        self.session.as_ref().map_err(String::as_str)
    }

    /// The flight to change, or why it could not start: for tests, which
    /// put the session into a state behind the port.
    #[cfg(any(test, feature = "fixture"))]
    pub fn session_mut(&mut self) -> Result<&mut Session, &str> {
        self.session.as_mut().map_err(|reason| reason.as_str())
    }

    /// The flight with `collaborators` handed to its session's calls in
    /// place of the ones it has: the edge's.
    #[must_use]
    pub fn with_collaborators(self, collaborators: Collaborators) -> Self {
        let Collaborators {
            chance,
            behaviour,
            defence_rule,
            law,
            boarding_rule,
            hail_options,
        } = collaborators;
        Self {
            chance,
            behaviour,
            defence_rule,
            law,
            boarding_rule,
            hail_options,
            ..self
        }
    }

    /// Sets the session up with `setup` (`nova_sim::Session::set_up`),
    /// after the view has read it as it opens: its rules, its strategy
    /// ports and what the run carried over. The fight's disable rule is the
    /// setup's too, so the session's changes of ship and the fight read
    /// one. A failed session sets nothing up.
    pub fn set_up(&mut self, setup: SessionSetup) {
        self.disable_rule = Rc::clone(&setup.disable_rule);
        let session = std::mem::replace(&mut self.session, Err(String::new()));
        self.session = session.map(|session| session.set_up(setup));
    }

    /// What the flight hands on to the next flight of the run
    /// (`nova_sim::Session::carry_over`), or `None` when its session never
    /// started.
    #[must_use]
    pub fn carry_over(&self) -> Option<CarryOver> {
        self.session().ok().map(Session::carry_over)
    }

    /// Begins a new pilot's game (`nova_sim::Session::begin`) over the
    /// catalog shared, on the flight's chance, and says whether it did: a
    /// failed session, or one never shared a catalog, begins nothing.
    pub fn begin(&mut self) -> bool {
        let (Ok(session), Some(catalog)) = (&mut self.session, &self.catalog) else {
            return false;
        };
        session.begin(&&*catalog.0, &mut self.chance);
        true
    }

    /// Enters play from the main screen (`nova_sim::Session::enter_game`),
    /// and says whether it did: a failed session enters nothing.
    pub fn enter_game(&mut self) -> bool {
        let Ok(session) = &mut self.session else {
            return false;
        };
        session.enter_game();
        true
    }

    /// Resumes a pilot opened from a save (`nova_sim::Session::resume`)
    /// on the flight's chance; a failed session resumes nothing.
    pub fn resume(&mut self) {
        if let Ok(session) = &mut self.session {
            session.resume(&mut self.chance);
        }
    }

    /// Opens a pilot just loaded on the main screen: every active
    /// mission's auxiliary ships restart
    /// (`nova_sim::Session::restart_aux_from_main_screen`) on the flight's
    /// chance. A failed session opens nothing.
    pub fn open_on_main_screen(&mut self) {
        if let Ok(session) = &mut self.session {
            session.restart_aux_from_main_screen(&mut self.chance);
        }
    }

    /// Opens the last pilot at launch (`nova_sim::Session::open_at_launch`)
    /// on the flight's chance. A failed session opens nothing.
    pub fn open_at_launch(&mut self) {
        if let Ok(session) = &mut self.session {
            session.open_at_launch(&mut self.chance);
        }
    }

    /// Whether the pilot may be saved now (`nova_sim::Session::may_save`):
    /// not while the player is dying or dead, nor without a session.
    #[must_use]
    pub fn may_save(&self) -> bool {
        self.session().is_ok_and(Session::may_save)
    }

    /// Whether the pilot is dying, as `_IsDying` (@0x253d) says: its ship
    /// breaking up or destroyed, which [`SessionFlight::may_save`]
    /// answers. Never without a session.
    #[must_use]
    pub fn is_dying(&self) -> bool {
        self.session().is_ok() && !self.may_save()
    }

    /// Whether ending play now deletes the pilot
    /// (`nova_sim::Session::deletes_pilot_on_ending_play`): a strict-play
    /// pilot dying outside the escape pod. Never without a session.
    #[must_use]
    pub fn deletes_pilot_on_ending_play(&self) -> bool {
        self.session()
            .is_ok_and(Session::deletes_pilot_on_ending_play)
    }

    /// Whether the pilot's load had to fall back for data the save
    /// references and the catalog lacks (`nova_sim::Session::load_repaired`):
    /// false for a session that failed.
    #[must_use]
    pub fn load_repaired(&self) -> bool {
        self.session().is_ok_and(Session::load_repaired)
    }

    /// The after-movie a `STR#` text shown now plays
    /// (`nova_sim::Session::loaded_after_movie`); none for a session that
    /// failed.
    #[must_use]
    pub fn loaded_after_movie(&self) -> Option<nova_sim::DescMovie> {
        self.session().ok().and_then(Session::loaded_after_movie)
    }

    /// The pilot flying, if the flight started: the adapter's own API, not
    /// the port's, for the app's save and the name it shows.
    #[must_use]
    pub fn pilot(&self) -> Option<&Pilot> {
        self.session().ok().map(Session::pilot)
    }

    /// Changes the pilot with `change` while landed, as
    /// `nova_sim::Session::transact` does, and says whether it did; a
    /// failed session changes nothing. The adapter's own API, for tests:
    /// a closure over the pilot is no command a sim could carry.
    #[cfg(any(test, feature = "fixture"))]
    pub fn transact(&mut self, change: impl FnOnce(&mut Pilot)) -> bool {
        self.session
            .as_mut()
            .is_ok_and(|session| session.transact(change))
    }
}

impl<C> From<Session> for SessionFlight<C> {
    /// The flight of `session`, with Nova's collaborators
    /// ([`SessionFlight::new`]).
    fn from(session: Session) -> Self {
        Self::new(Ok(session))
    }
}

impl<C> FlightSim for SessionFlight<C> {
    fn frame(&self, into: &mut FlightFrame) {
        match &self.session {
            Ok(session) => session.fill_frame(into),
            Err(_) => *into = FlightFrame::default(),
        }
    }

    /// Forwards to `Session::step`, handing it the chance, the behaviour,
    /// the fight's rules and the catalog the view shared
    /// ([`SessionFlight::share_catalog`]). A failed session, or one whose
    /// catalog was never shared, runs no step and holds nothing.
    fn step(&mut self, input: &StepInput, steps: u32, into: &mut StepReport) {
        let (Ok(session), Some(catalog)) = (&mut self.session, &self.catalog) else {
            into.clear();
            return;
        };
        let rules = Rules {
            disable: &*self.disable_rule,
            defence: &*self.defence_rule,
            law: &*self.law,
        };
        let mut collab = StepCollaborators {
            catalog: &*catalog.0,
            behaviour: &*self.behaviour,
            rules,
            chance: &mut self.chance,
        };
        session.step(input, steps, &mut collab, into);
    }

    /// Forwards to `Session::apply`, handing it the catalog the view
    /// shared ([`SessionFlight::share_catalog`]), if it has, the flight's
    /// chance, its law, its boarding rule and its hail options; a failed
    /// session refuses every command.
    fn apply(&mut self, command: FlightCommand) -> Answer {
        let Ok(session) = &mut self.session else {
            return Answer::Refused;
        };
        let mut collab = ApplyCollaborators {
            catalog: self.catalog.as_ref().map(|shared| &*shared.0),
            chance: &mut self.chance,
            law: &*self.law,
            boarding_rule: &*self.boarding_rule,
            hail_options: &self.hail_options,
        };
        session.apply(command, &mut collab)
    }

    /// Forwards to `Session::drain_events`; a failed session drains
    /// nothing.
    fn drain(&mut self, into: &mut Vec<FlightEvent>) {
        if let Ok(session) = &mut self.session {
            session.drain_events(into);
        }
    }

    fn takes(&self, gate: PlayerCommand) -> bool {
        self.session().map_or(true, |session| session.takes(gate))
    }

    /// Forwards to `Session::hover`; a failed session hovers nothing.
    fn hover(&self, at: Vec2, drawn: &DrawnBoxes) -> Option<Hover> {
        self.session().ok()?.hover(at, drawn)
    }

    /// Forwards to `Session::sabotage_death_descs`; a failed session has
    /// none.
    fn death_descs(&self) -> Vec<i16> {
        self.session()
            .map_or_else(|_| Vec::new(), Session::sabotage_death_descs)
    }

    /// Forwards to `Session::coming_traffic_ships`, reading the catalog the
    /// view shared ([`SessionFlight::share_catalog`]); a failed session, or
    /// one never shared a catalog, brings none.
    fn coming_traffic(&self) -> Vec<ShipId> {
        match (self.session(), &self.catalog) {
            (Ok(session), Some(catalog)) => session.coming_traffic_ships(&*catalog.0),
            _ => Vec::new(),
        }
    }

    /// Forwards to `Session::load_description`, drawing on the flight's
    /// chance; a failed session resolves only the conditional text, for a
    /// new pilot (`Session::fresh_description`).
    fn load_description(&mut self, raw: Option<&str>, movie: Option<&DescMovie>) -> String {
        match &mut self.session {
            Ok(session) => session.load_description(raw, movie, &mut self.chance),
            Err(_) => Session::fresh_description(raw),
        }
    }

    /// Why the session could not start, if it could not.
    fn failure(&self) -> Option<&str> {
        self.session().err()
    }
}

/// Each forwards to the session's method of its name; a failed session
/// answers nothing. The panel and the hypergate cycle read the catalog the
/// view shared ([`SessionFlight::share_catalog`]), and answer nothing
/// before it is shared.
impl<C> MapSource for SessionFlight<C> {
    fn map_drawing(
        &self,
        hypergate: Option<&[SystemId]>,
        missions: &[SystemId],
        marks: MapMarks,
    ) -> Option<MapDrawing> {
        let session = self.session().ok()?;
        Some(session.map_drawing(hypergate, missions, marks))
    }

    fn map_panel(
        &self,
        selected: Option<SystemId>,
        hypergate: Option<&[SystemId]>,
    ) -> Option<MapPanel> {
        let session = self.session().ok()?;
        let catalog = &self.catalog.as_ref()?.0;
        Some(session.map_panel(catalog, selected, hypergate))
    }

    fn map_border_sources(&self) -> Option<BorderSources> {
        Some(self.session().ok()?.map_border_sources())
    }

    fn find_map_system(&self, query: &str) -> Option<SearchOutcome> {
        Some(self.session().ok()?.find_map_system(query))
    }

    fn map_mission_systems(&self) -> Vec<SystemId> {
        self.session()
            .map_or_else(|_| Vec::new(), Session::map_mission_systems)
    }

    fn map_preselection(&self) -> Option<SystemId> {
        self.session().ok()?.map_preselection()
    }

    fn floating_preselection(&self) -> Option<SystemId> {
        self.session().ok()?.floating_preselection()
    }

    fn hypergate_cycle(&self) -> Vec<Option<SystemId>> {
        match (self.session(), &self.catalog) {
            (Ok(session), Some(catalog)) => session.hypergate_cycle(&catalog.0),
            _ => Vec::new(),
        }
    }

    fn offer_map_marks(&self, mission: MissionId) -> MapMarks {
        self.session()
            .map_or(MapMarks::NONE, |session| session.offer_map_marks(mission))
    }

    fn computer_map_marks(&self, selected: Option<MissionId>) -> MapMarks {
        self.session().map_or(MapMarks::NONE, |session| {
            session.computer_map_marks(selected)
        })
    }

    fn mission_info_map_marks(&self, slot: Option<u8>) -> MapMarks {
        self.session().map_or(MapMarks::NONE, |session| {
            session.mission_info_map_marks(slot)
        })
    }

    fn map_nav(&self) -> Option<SavedNav> {
        self.session().ok().map(Session::map_nav)
    }

    fn floating_map_places(&self) -> FloatingLinks {
        self.session()
            .map_or_else(|_| FloatingLinks::default(), Session::floating_map_places)
    }

    fn stellar_name(&self, stellar: StellarId) -> Option<String> {
        TagNames::stellar_name(self.session().ok()?, stellar)
    }

    fn has_explored(&self, system: SystemId) -> bool {
        self.session()
            .is_ok_and(|session| session.pilot().has_explored(system))
    }

    fn galaxy(&self) -> Option<&OverlaidGalaxy> {
        self.session().ok().map(Session::galaxy)
    }
}

/// Each forwards to the session's method of its name; a failed session
/// answers nothing. The hail reads the catalog the view shared
/// ([`SessionFlight::share_catalog`]) and lists the flight's hail options,
/// and answers nothing before the catalog is shared.
impl<C> CommSource for SessionFlight<C> {
    fn hailing(&self) -> Option<HailView> {
        let session = self.session().ok()?;
        let catalog = &self.catalog.as_ref()?.0;
        session.hailing(&**catalog, &self.hail_options)
    }

    fn boarding(&self) -> Option<PlunderView> {
        self.session().ok()?.boarding()
    }
}

/// Each forwards to the session's method of its name, the rolled reads
/// on the flight's chance; a failed session answers nothing open. The
/// name prompt reads the catalog the view shared
/// ([`SessionFlight::share_catalog`]), and has no shipyard before it is
/// shared. The offer gap is drawn on the flight's chance whatever the
/// session (`nova_sim::mission::offer_gap`).
impl<C> PortSource for SessionFlight<C> {
    fn market(&self) -> Option<Market> {
        self.session().ok()?.market()
    }

    fn recharge_enabled(&self) -> bool {
        self.session().is_ok_and(Session::recharge_enabled)
    }

    fn news(&self) -> Option<&News> {
        self.session().ok()?.news()
    }

    fn news_picture(&self) -> Option<i16> {
        self.session().ok()?.news_picture()
    }

    fn mission_info(&self) -> MissionInfo {
        self.session()
            .map_or_else(|_| MissionInfo::default(), Session::mission_info)
    }

    fn seen_intro(&self) -> bool {
        self.session().is_ok_and(Session::seen_intro)
    }

    fn refusable_offer_dialog(&self, mission: MissionId) -> bool {
        self.session()
            .is_ok_and(|session| session.refusable_offer_dialog(mission))
    }

    fn offer_text(&self, mission: MissionId, raw: &str) -> Option<String> {
        self.session().ok()?.offer_text(mission, raw)
    }

    fn outfitter(&mut self) -> Option<Outfitter> {
        self.session.as_mut().ok()?.outfitter(&mut self.chance)
    }

    fn shipyard(&mut self) -> Option<Shipyard> {
        self.session.as_mut().ok()?.shipyard(&mut self.chance)
    }

    fn escorts_for_hire(&mut self) -> Option<HireList> {
        self.session
            .as_mut()
            .ok()?
            .escorts_for_hire(&mut self.chance)
    }

    fn ship_naming(&mut self, ship: ShipId) -> Result<ShipNaming, ShipRefusal> {
        let (Ok(session), Some(catalog)) = (&mut self.session, &self.catalog) else {
            return Err(ShipRefusal::NoShipyard);
        };
        session.ship_naming(ship, &*catalog.0, &mut self.chance)
    }

    fn offer_gap(&mut self) -> u32 {
        nova_sim::mission::offer_gap(&mut self.chance)
    }

    fn can_hire_escorts(&self) -> bool {
        self.session().is_ok_and(Session::can_hire_escorts)
    }
}

/// Each forwards to the session's method of its name, the player's ship
/// name to its pilot's and the cargo's to `TagNames::cargo_name`; a
/// failed session names nothing.
impl<C> NameSource for SessionFlight<C> {
    fn ship_name(&self, ship: ShipId) -> Option<&str> {
        self.session().ok()?.ship_name(ship)
    }

    fn ship_long_name(&self, ship: ShipId) -> Option<&str> {
        self.session().ok()?.ship_long_name(ship)
    }

    fn outfit_name(&self, outfit: OutfitId) -> Option<&str> {
        self.session().ok()?.outfit_name(outfit)
    }

    fn outfit_names(&self, outfit: OutfitId) -> Option<(&str, &str)> {
        self.session().ok()?.outfit_names(outfit)
    }

    fn good_name(&self, good: Good) -> Option<&str> {
        self.session().ok()?.good_name(good)
    }

    fn junk_lc_name(&self, junk: JunkId) -> Option<&str> {
        self.session().ok()?.junk_lc_name(junk)
    }

    fn cargo_name(&self, cargo: u8) -> Option<String> {
        TagNames::cargo_name(self.session().ok()?, cargo)
    }

    fn player_ship_name(&self) -> Option<&str> {
        self.session().ok()?.pilot().ship_name()
    }

    fn long_date(&self) -> String {
        self.session()
            .map_or_else(|_| String::new(), Session::long_date_text)
    }

    /// A failed session fills nothing: the text comes back unchanged.
    fn plain_text(&self, text: &str) -> String {
        self.session()
            .map_or_else(|_| text.to_owned(), |session| session.plain_text(text))
    }
}

/// Each forwards to the session's method of its name, over the catalog
/// the view shared ([`SessionFlight::share_catalog`]); a failed session,
/// or one never shared a catalog, shows none.
impl<C> PilotSource for SessionFlight<C> {
    fn player_info(&self) -> Option<PlayerInfo> {
        let session = self.session().ok()?;
        let catalog = &self.catalog.as_ref()?.0;
        Some(session.player_info(&**catalog))
    }

    fn pilot_panel(&self) -> Option<PilotPanel> {
        let session = self.session().ok()?;
        let catalog = &self.catalog.as_ref()?.0;
        Some(session.pilot_panel(&**catalog))
    }
}

/// The developer tools' desk: the reading is
/// `nova_sim::Session::desk_reading`, and each [`PilotEdit`] goes to the
/// one session method that makes it (`set_credits`, `set_reserve`,
/// `set_date`, `relocate` and `set_control_bit`); the session decides, and
/// the adapter only maps a move's refusal. A move reads the catalog the
/// view shared ([`SessionFlight::share_catalog`]), and is refused as
/// [`EditRefusal::NoSystem`] before it is shared, as no system can be
/// found. A failed session has no reading and refuses every edit as a
/// move with no catalog is.
impl<C> DevDesk for SessionFlight<C> {
    fn reading(&self) -> Option<DeskReading> {
        Some(self.session().ok()?.desk_reading())
    }

    fn edit(&mut self, edit: PilotEdit) -> Result<(), EditRefusal> {
        let Ok(session) = &mut self.session else {
            return Err(EditRefusal::NoSystem);
        };
        match edit {
            PilotEdit::Credits(credits) => session.set_credits(credits),
            PilotEdit::Reserve(reserve, amount) => {
                session.set_reserve(reserve, amount);
            }
            PilotEdit::Date(date) => session.set_date(date),
            PilotEdit::MoveTo { system, stellar } => {
                let Some(catalog) = &self.catalog else {
                    return Err(EditRefusal::NoSystem);
                };
                session
                    .relocate(&*catalog.0, system, stellar)
                    .map_err(refusal)?;
            }
            PilotEdit::Bit(bit, on) => session.set_control_bit(bit, on),
        }
        Ok(())
    }
}

/// The desk's refusal for the session's.
fn refusal(refusal: RelocateRefusal) -> EditRefusal {
    match refusal {
        RelocateRefusal::InFlight => EditRefusal::InFlight,
        RelocateRefusal::NoSystem => EditRefusal::NoSystem,
        RelocateRefusal::NoStellar => EditRefusal::NoStellar,
        RelocateRefusal::NotLandable => EditRefusal::NotLandable,
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::StellarPick;

    use super::*;
    use crate::flight::sim_port::fixture::unlike;

    fn failed() -> SessionFlight<()> {
        SessionFlight::new(Err("no chär".to_owned()))
    }

    #[test]
    fn a_failed_flight_fills_the_default_frame() {
        let mut frame = unlike(&FlightFrame::default());
        failed().frame(&mut frame);
        assert_eq!(frame, FlightFrame::default());
    }

    #[test]
    fn a_failed_flight_refuses_every_command() {
        assert_eq!(
            failed().apply(FlightCommand::SelectStellar(StellarPick::Slot(0))),
            Answer::Refused
        );
    }

    #[test]
    fn a_failed_flight_takes_every_gate_and_answers_nothing() {
        let flight = failed();
        assert!(flight.takes(PlayerCommand::Nav));
        assert!(flight.takes(PlayerCommand::Map));
        let none = SystemId(130);
        assert_eq!(flight.map_drawing(None, &[], MapMarks::NONE), None);
        assert_eq!(flight.map_panel(Some(none), None), None);
        assert_eq!(flight.map_border_sources(), None);
        assert_eq!(flight.find_map_system("sol"), None);
        assert_eq!(flight.map_mission_systems(), []);
        assert_eq!(flight.map_preselection(), None);
        assert_eq!(flight.floating_preselection(), None);
        assert_eq!(flight.hypergate_cycle(), []);
        assert_eq!(flight.offer_map_marks(MissionId(128)), MapMarks::NONE);
        assert_eq!(flight.computer_map_marks(None), MapMarks::NONE);
        assert_eq!(flight.mission_info_map_marks(None), MapMarks::NONE);
        assert_eq!(flight.map_nav(), None);
        assert_eq!(flight.floating_map_places(), FloatingLinks::default());
        assert_eq!(flight.stellar_name(StellarId(128)), None);
        assert!(!flight.has_explored(none));
        assert_eq!(flight.galaxy(), None);
        assert_eq!(flight.hover(Vec2::ZERO, &DrawnBoxes::default()), None);
        assert_eq!(flight.coming_traffic(), []);
        assert_eq!(flight.death_descs(), Vec::<i16>::new());
    }

    #[test]
    fn a_failed_flight_names_nothing() {
        let flight = failed();
        assert_eq!(flight.ship_name(ShipId(128)), None);
        assert_eq!(flight.ship_long_name(ShipId(128)), None);
        assert_eq!(flight.outfit_name(OutfitId(128)), None);
        assert_eq!(flight.outfit_names(OutfitId(128)), None);
        assert_eq!(flight.good_name(Good::Commodity(0)), None);
        assert_eq!(flight.junk_lc_name(JunkId(128)), None);
        assert_eq!(flight.cargo_name(0), None);
        assert_eq!(flight.player_ship_name(), None);
        assert_eq!(flight.long_date(), "");
        assert_eq!(flight.player_info(), None);
        assert_eq!(flight.pilot_panel(), None);
    }

    #[test]
    fn a_failed_flight_refuses_the_travel_commands() {
        let mut flight = failed();
        for command in [
            FlightCommand::Jump,
            FlightCommand::Land,
            FlightCommand::Arrive,
            FlightCommand::TakeOff,
            FlightCommand::LeaveIfDue,
            FlightCommand::LandEscorts,
            FlightCommand::SettleScript,
            FlightCommand::BeginFade,
            FlightCommand::EndPlay,
            FlightCommand::CommShown(240),
        ] {
            assert_eq!(
                flight.apply(command.clone()),
                Answer::Refused,
                "{command:?}"
            );
        }
    }

    #[test]
    fn a_failed_flight_refuses_the_comm_commands_and_answers_no_comm() {
        let mut flight = failed();
        assert_eq!(flight.hailing(), None);
        assert_eq!(flight.boarding(), None);
        for command in [
            FlightCommand::Hail { option: false },
            FlightCommand::AnswerHail(0),
            FlightCommand::Haggle(nova_sim::Haggle::Accept),
            FlightCommand::HangUp,
            FlightCommand::Board,
            FlightCommand::Plunder(nova_sim::Take::Cargo),
            FlightCommand::LetGoOfBoarding,
            FlightCommand::CommandEscorts {
                group: nova_sim::EscortGroup::All,
                command: nova_sim::EscortCommand::Hold,
            },
            FlightCommand::PressEscort(nova_sim::escort_dialog::EscortButton::Close),
            FlightCommand::CommAcknowledged,
            FlightCommand::CommTicked,
        ] {
            assert_eq!(
                flight.apply(command.clone()),
                Answer::Refused,
                "{command:?}"
            );
        }
    }

    #[test]
    fn a_failed_flight_refuses_the_port_commands_and_answers_nothing_open() {
        let mut flight = failed();
        assert_eq!(flight.market(), None);
        assert!(!flight.recharge_enabled());
        assert_eq!(flight.news(), None);
        assert_eq!(flight.news_picture(), None);
        assert_eq!(flight.mission_info(), MissionInfo::default());
        assert!(!flight.seen_intro());
        assert!(!flight.refusable_offer_dialog(MissionId(130)));
        assert_eq!(flight.offer_text(MissionId(130), "Run <PAY>"), None);
        assert_eq!(flight.outfitter(), None);
        assert_eq!(flight.shipyard(), None);
        assert_eq!(flight.escorts_for_hire(), None);
        assert_eq!(
            flight.ship_naming(ShipId(129)),
            Err(ShipRefusal::NoShipyard)
        );
        assert!(!flight.transact(|pilot| pilot.cash = 1), "changes nothing");
        let order = nova_sim::OutfitOrder {
            outfit: nova_sim::OutfitId(128),
            direction: nova_sim::Direction::Buy,
        };
        for command in [
            FlightCommand::Trade(nova_sim::Order {
                row: 0,
                good: nova_sim::Good::Commodity(0),
                direction: nova_sim::Direction::Buy,
                lot: nova_sim::Lot::Click,
            }),
            FlightCommand::Recharge,
            FlightCommand::OpenOutfitter,
            FlightCommand::CloseOutfitter,
            FlightCommand::Outfit { order, count: 1 },
            FlightCommand::OutfitCounted { order, count: 1 },
            FlightCommand::BuyShip {
                ship: ShipId(129),
                name: "Kestrel".to_owned(),
            },
            FlightCommand::DeclineShip(ShipId(129)),
            FlightCommand::OpenBar,
            FlightCommand::Hire(ShipId(129)),
            FlightCommand::CloseHire,
            FlightCommand::OpenMissionComputer,
            FlightCommand::ShowComputerRow(MissionId(130)),
            FlightCommand::ShowInfoRow(0),
            FlightCommand::AcceptMission(MissionId(130)),
            FlightCommand::RefuseMission(MissionId(130)),
            FlightCommand::AbortMission(0),
            FlightCommand::NextOffer(1),
            FlightCommand::EnterOfferVenue(1),
            FlightCommand::ReloadOfferDesc(MissionId(130)),
            FlightCommand::PressMissionInfo,
            FlightCommand::PressPlayerInfo,
            FlightCommand::OpenPlayerInfo,
            FlightCommand::Jettison { all: true },
            FlightCommand::Assign(nova_sim::Assignment::Escort),
            FlightCommand::MarkIntroSeen,
        ] {
            assert_eq!(
                flight.apply(command.clone()),
                Answer::Refused,
                "{command:?}"
            );
        }
    }

    /// Draws 7 every time, and notes each draw's range.
    #[derive(Debug, Default)]
    struct Sevens(Vec<u32>);

    impl nova_sim::Chance for Sevens {
        fn fires(&mut self, _percent: u8) -> bool {
            false
        }

        fn below(&mut self, n: u32) -> u32 {
            self.0.push(n);
            7
        }
    }

    #[test]
    fn the_offer_gap_is_drawn_on_the_flights_chance_whatever_the_session() {
        let sevens = Rc::new(std::cell::RefCell::new(Sevens::default()));
        let chance: Rc<std::cell::RefCell<dyn nova_sim::Chance>> = sevens.clone();
        let mut flight = failed().with_collaborators(Collaborators {
            chance: SharedChance::new(chance),
            ..Collaborators::default()
        });
        assert_eq!(flight.offer_gap(), 37, "30 and the draw of 7");
        assert_eq!(sevens.borrow().0, [30]);
    }

    #[test]
    fn a_failed_flight_runs_no_step() {
        let mut report = StepReport {
            steps: vec![nova_sim::flight_frame::StepRun::default()],
            stopped: true,
        };
        failed().step(&StepInput::default(), 3, &mut report);
        assert_eq!(report, StepReport::default());
    }

    #[test]
    fn a_failed_flight_loads_a_description_for_a_new_pilot() {
        let mut flight = failed();
        assert_eq!(
            flight.load_description(Some("{b5 \"set\" \"clear\"}, sir."), None),
            "clear, sir.",
            "resolved by nova-sim"
        );
        assert_eq!(flight.load_description(None, None), "");
    }

    #[test]
    fn a_failed_flight_drains_nothing() {
        let mut into = vec![FlightEvent::SaveDue];
        failed().drain(&mut into);
        assert_eq!(into, [FlightEvent::SaveDue]);
    }

    #[test]
    fn a_failed_flight_has_no_pilot() {
        assert!(failed().pilot().is_none());
    }

    #[test]
    fn a_failed_flight_gives_why_it_could_not_start() {
        let mut flight = failed();
        assert_eq!(flight.session().err(), Some("no chär"));
        assert_eq!(flight.session_mut().err(), Some("no chär"));
        assert_eq!(flight.failure(), Some("no chär"));
    }

    #[test]
    fn a_failed_flight_meets_the_contract() {
        crate::flight::sim_port::fixture::check_contract(&mut failed()).expect("meets it");
    }

    #[test]
    fn a_failed_flight_sets_up_begins_enters_and_saves_nothing() {
        let mut flight = failed();
        flight.set_up(SessionSetup::default());
        assert_eq!(flight.carry_over(), None);
        assert!(!flight.begin());
        assert!(!flight.enter_game());
        flight.resume();
        flight.open_on_main_screen();
        flight.open_at_launch();
        assert_eq!(flight.session().err(), Some("no chär"), "still failed");
        assert!(!flight.may_save());
        assert!(!flight.is_dying());
        assert!(!flight.deletes_pilot_on_ending_play());
        assert!(!flight.load_repaired());
        assert_eq!(flight.loaded_after_movie(), None);
        assert!(!flight.can_hire_escorts());
        assert_eq!(flight.plain_text("Hi <PSN>."), "Hi <PSN>.", "unchanged");
    }

    #[test]
    fn a_failed_flight_has_no_desk_reading_and_refuses_every_edit() {
        let mut flight = failed();
        assert_eq!(flight.reading(), None);
        assert_eq!(
            flight.edit(PilotEdit::Credits(77)),
            Err(EditRefusal::NoSystem)
        );
    }

    /// The catalog crate's canned catalog, shared as the edge shares it.
    fn shared() -> Rc<nova_sim::fixture::FakePilotCatalog> {
        Rc::new(nova_sim::fixture::pilot_catalog())
    }

    #[test]
    fn the_edge_shares_its_catalog_with_the_flight_it_builds() {
        let catalog = shared();
        let pilot = || Pilot::new(&*catalog, "Ada").expect("a pilot");
        let built: [fn(&Rc<_>, Pilot) -> SessionFlight<_>; 3] = [
            |catalog, _| SessionFlight::start(catalog),
            SessionFlight::fly,
            |catalog, pilot| SessionFlight::load(catalog, pilot, &Rulebook::default()),
        ];
        for build in built {
            let flight = build(&catalog, pilot());
            assert!(flight.session().is_ok(), "started");
            let held = flight.shared_catalog().expect("shared");
            assert!(std::ptr::addr_eq(Rc::as_ptr(held), Rc::as_ptr(&catalog)));
        }
    }

    #[test]
    fn the_edge_says_why_a_flight_could_not_start() {
        let catalog = Rc::new(nova_sim::fixture::FakePilotCatalog {
            character: Err(nova_sim::StartError::NoCharacter),
            ..nova_sim::fixture::pilot_catalog()
        });
        let flight = SessionFlight::start(&catalog);
        assert_eq!(
            flight.failure(),
            Some(nova_sim::StartError::NoCharacter.to_string().as_str())
        );
    }

    #[test]
    fn a_flight_never_shared_a_catalog_shows_no_player_info_or_pilot_panel() {
        let session = Session::start(&nova_sim::fixture::pilot_catalog()).expect("starts");
        let flight: SessionFlight<()> = session.into();
        assert_eq!(flight.player_info(), None);
        assert_eq!(flight.pilot_panel(), None);
    }

    #[test]
    fn a_flight_never_shared_a_catalog_begins_nothing() {
        let session = Session::start(&nova_sim::fixture::pilot_catalog()).expect("starts");
        let mut flight: SessionFlight<()> = session.clone().into();
        assert!(!flight.begin());
        assert_eq!(flight.session().ok(), Some(&session), "unchanged");
    }

    #[test]
    fn the_setup_reaches_the_session_and_the_fights_disable_rule() {
        let catalog = shared();
        let mut flight = SessionFlight::start(&catalog);
        let setup = SessionSetup {
            rules: Rulebook::default()
                .with_override(nova_sim::RuleKey::BreakupStall, nova_sim::RuleSource::Bible),
            disable_rule: Rc::new(nova_sim::NovaDisable),
            carry: CarryOver {
                last_refresh: Some(nova_sim::LastRefresh::hiding([SystemId(131)])),
                ..CarryOver::default()
            },
            ..SessionSetup::default()
        };
        let expected = flight
            .session()
            .expect("started")
            .clone()
            .set_up(setup.clone());
        flight.set_up(setup.clone());
        let session = flight.session().expect("started");
        assert_eq!(session.rules(), expected.rules());
        assert_eq!(session.carry_over(), expected.carry_over());
        assert!(Rc::ptr_eq(&flight.disable_rule, &setup.disable_rule));
        assert!(std::ptr::addr_eq(
            std::ptr::from_ref(session.disable_rule()),
            Rc::as_ptr(&setup.disable_rule)
        ));
        assert_eq!(flight.carry_over(), Some(session.carry_over()));
    }

    #[test]
    fn beginning_begins_the_session_on_the_catalog_shared() {
        let catalog = shared();
        let mut flight = SessionFlight::start(&catalog);
        let mut expected = flight.session().expect("started").clone();
        expected.begin(&*catalog, &mut nova_sim::NeverFires);
        assert!(flight.begin());
        assert_eq!(flight.session().expect("started").pilot(), expected.pilot());
        assert_eq!(
            flight.session().expect("started").player().position,
            Vec2::new(50.0, 50.0)
        );
        assert!(flight.enter_game());
    }

    #[test]
    fn the_saving_queries_are_the_sessions() {
        let catalog = shared();
        let flight = SessionFlight::start(&catalog);
        let session = flight.session().expect("started");
        assert!(flight.may_save());
        assert_eq!(flight.may_save(), session.may_save());
        assert!(!flight.is_dying());
        assert_eq!(
            flight.deletes_pilot_on_ending_play(),
            session.deletes_pilot_on_ending_play()
        );
        assert_eq!(flight.load_repaired(), session.load_repaired());
        assert_eq!(flight.loaded_after_movie(), session.loaded_after_movie());
        assert_eq!(flight.can_hire_escorts(), session.can_hire_escorts());
        assert_eq!(
            flight.plain_text("Hello, <PN>."),
            session.plain_text("Hello, <PN>.")
        );
    }

    #[test]
    fn a_dying_pilot_may_not_be_saved() {
        let catalog = nova_sim::fixture::pilot_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        nova_sim::fixture::set_player_condition(&mut session, nova_sim::Condition::Destroyed);
        let flight: SessionFlight<()> = session.into();
        assert!(!flight.may_save());
        assert!(flight.is_dying());
    }

    #[test]
    fn the_desks_reading_is_the_sessions() {
        let catalog = shared();
        let flight = SessionFlight::start(&catalog);
        assert_eq!(
            flight.reading(),
            Some(flight.session().expect("started").desk_reading())
        );
    }

    #[test]
    fn each_desk_edit_goes_to_the_session() {
        let catalog = shared();
        let mut flight = SessionFlight::start(&catalog);
        let date = nova_sim::GameDate::new(1180, 1, 2).expect("a date");
        let bit = nova_sim::Bit::new(9).expect("in range");
        for edit in [
            PilotEdit::Credits(77),
            PilotEdit::Reserve(nova_sim::Reserve::Armor, 5.0),
            PilotEdit::Date(date),
            PilotEdit::Bit(bit, true),
        ] {
            assert_eq!(flight.edit(edit), Ok(()), "{edit:?}");
        }
        let session = flight.session().expect("started");
        assert_eq!(session.pilot().cash(), 77);
        assert!((session.reserves().armor.now - 5.0).abs() < f32::EPSILON);
        assert_eq!(session.date(), date);
        assert!(session.control_bit(bit));
    }

    #[test]
    fn a_move_is_the_sessions_relocation_with_its_refusal_mapped() {
        let catalog = shared();
        let mut flight = SessionFlight::start(&catalog);
        let before = flight.session().expect("started").clone();
        assert_eq!(
            flight.edit(PilotEdit::MoveTo {
                system: SystemId(131),
                stellar: StellarId(140)
            }),
            Err(EditRefusal::InFlight),
            "in flight"
        );
        assert_eq!(flight.session().ok(), Some(&before), "unchanged");
    }

    #[test]
    fn a_move_without_a_catalog_is_refused_as_no_system() {
        let catalog = nova_sim::fixture::pilot_catalog();
        let session = Session::start(&catalog).expect("starts");
        let mut flight: SessionFlight<()> = session.into();
        assert_eq!(
            flight.edit(PilotEdit::MoveTo {
                system: SystemId(131),
                stellar: StellarId(140)
            }),
            Err(EditRefusal::NoSystem)
        );
    }

    #[test]
    fn each_refusal_maps_to_its_own() {
        for (from, to) in [
            (RelocateRefusal::InFlight, EditRefusal::InFlight),
            (RelocateRefusal::NoSystem, EditRefusal::NoSystem),
            (RelocateRefusal::NoStellar, EditRefusal::NoStellar),
            (RelocateRefusal::NotLandable, EditRefusal::NotLandable),
        ] {
            assert_eq!(refusal(from), to);
        }
    }
}
