//! [`SessionFlight`]: the [`FlightSim`] adapter over a real
//! `nova_sim::Session`, with the collaborators the session's calls are
//! handed: the chance, the NPCs' behaviour and the fight's, the law's and
//! the boardings' rules.
//!
//! It forwards: [`FlightSim::frame`] to `Session::fill_frame`,
//! [`FlightSim::hover`] to `Session::hover`, [`FlightSim::coming_traffic`]
//! to `Session::coming_traffic_ships`, [`FlightSim::death_descs`] to
//! `Session::sabotage_death_descs`,
//! [`FlightSim::step`] to `Session::step`, with its collaborators and
//! the catalog the view shared,
//! [`FlightSim::apply`] to `Session::apply`, whose match over the
//! commands lives on the session's side, with the catalog the view shared
//! and the chance, and [`FlightSim::drain`] to
//! `Session::drain_events`. A session that failed to start fills the
//! default frame, runs no step, refuses every command and drains nothing.
//!
//! **For the strangler's length**, the view's
//! `FlightView<C, SessionFlight<C>>` blocks reach the session and the
//! collaborators through this adapter's fields, which are visible to
//! `crate::flight` only: a call such as `session.board(&*self.sim.law,
//! .., &mut self.sim.chance)` borrows the fields apart, where a
//! [`SessionFlight::session_mut`] beside `&mut self.sim.chance` would
//! borrow all of `self.sim` twice. Code outside the concrete blocks uses
//! [`SessionFlight::session`] and [`SessionFlight::session_mut`].

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
use nova_sim::story::OverlaidGalaxy;
use nova_sim::{
    Allegiance, ApplyCollaborators, Behaviour, BoardingRule, DescMovie, DisableRule, HailOptions,
    Hover, LegalCode, MapMarks, MissionId, NovaAi, NovaBoarding, NovaDisable, NovaLaw,
    PlayerCommand, PointDefenceRule, Rules, SavedNav, Session, ShipId, SimCatalog, StellarId,
    StepCollaborators, SystemId, TagNames, Vec2,
};

use super::SharedChance;
use super::sim_port::{CommSource, FlightSim, MapSource, PortSource, fresh_description};

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
    hail_options: HailOptions,
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

/// The view's catalog, as the flight reads it: as a `SimCatalog` only,
/// so the adapter's `FlightSim` asks nothing of `C`.
#[derive(Clone)]
struct SharedCatalog(Rc<dyn SimCatalog>);

impl std::fmt::Debug for SharedCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedCatalog")
    }
}

impl<C> SessionFlight<C> {
    /// The flight of `session`, or why it could not start, with Nova's
    /// collaborators: the never-firing chance, Nova's AI, disable rule,
    /// law and boardings, and point defence by allegiance.
    #[must_use]
    pub fn new(session: Result<Session, String>) -> Self {
        Self {
            session,
            chance: SharedChance::default(),
            behaviour: Rc::new(NovaAi::default()),
            disable_rule: Rc::new(NovaDisable),
            defence_rule: Rc::new(Allegiance),
            law: Rc::new(NovaLaw::default()),
            boarding_rule: Rc::new(NovaBoarding::default()),
            hail_options: HailOptions::default(),
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

    /// The flight to change, or why it could not start.
    pub fn session_mut(&mut self) -> Result<&mut Session, &str> {
        self.session.as_mut().map_err(|reason| reason.as_str())
    }

    /// The flight with each day's events and the traffic rolled on
    /// `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The flight with its NPCs deciding as `behaviour` says.
    #[must_use]
    pub fn with_behaviour(self, behaviour: Rc<dyn Behaviour>) -> Self {
        Self { behaviour, ..self }
    }

    /// The flight with its ships disabled as `rule` says: in the fight,
    /// and in its session's changes of ship
    /// (`nova_sim::Session::with_disable_rule`).
    #[must_use]
    pub fn with_disable_rule(self, disable_rule: Rc<dyn DisableRule>) -> Self {
        Self {
            session: self
                .session
                .map(|session| session.with_disable_rule(Rc::clone(&disable_rule))),
            disable_rule,
            ..self
        }
    }

    /// The flight with its ships' point defence engaging the missiles
    /// `rule` calls hostile.
    #[must_use]
    pub fn with_point_defence_rule(self, defence_rule: Rc<dyn PointDefenceRule>) -> Self {
        Self {
            defence_rule,
            ..self
        }
    }

    /// The flight with the player's crimes judged by `law`.
    #[must_use]
    pub fn with_law(self, law: Rc<dyn LegalCode>) -> Self {
        Self { law, ..self }
    }

    /// The flight with its boardings by `rule`: who repels boarders, the
    /// capture odds and the capture roll.
    #[must_use]
    pub fn with_boarding_rule(self, boarding_rule: Rc<dyn BoardingRule>) -> Self {
        Self {
            boarding_rule,
            ..self
        }
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
    /// new pilot ([`fresh_description`]).
    fn load_description(&mut self, raw: Option<&str>, movie: Option<&DescMovie>) -> String {
        match &mut self.session {
            Ok(session) => session.load_description(raw, movie, &mut self.chance),
            Err(_) => fresh_description(raw),
        }
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

impl<C> CommSource for SessionFlight<C> {}

impl<C> PortSource for SessionFlight<C> {}

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
        assert_eq!(flight.load_description(Some("plain"), None), "plain");
        assert_eq!(flight.load_description(None, None), "");
    }

    #[test]
    fn a_failed_flight_drains_nothing() {
        let mut into = vec![FlightEvent::SaveDue];
        failed().drain(&mut into);
        assert_eq!(into, [FlightEvent::SaveDue]);
    }

    #[test]
    fn a_failed_flight_gives_why_it_could_not_start() {
        let mut flight = failed();
        assert_eq!(flight.session().err(), Some("no chär"));
        assert_eq!(flight.session_mut().err(), Some("no chär"));
    }
}
