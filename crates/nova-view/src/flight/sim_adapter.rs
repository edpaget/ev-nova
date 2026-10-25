//! [`SessionFlight`]: the [`FlightSim`] adapter over a real
//! `nova_sim::Session`, with the collaborators the session's calls are
//! handed: the chance, the NPCs' behaviour and the fight's, the law's and
//! the boardings' rules.
//!
//! It forwards: [`FlightSim::frame`] to `Session::fill_frame` and
//! [`FlightSim::apply`] to `Session::apply`, whose match over the
//! commands lives on the session's side. A session that failed to start
//! fills the default frame and refuses every command.
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
    Answer, FlightCommand, FlightEvent, FlightFrame, StepInput, StepReport,
};
use nova_sim::{
    Allegiance, Behaviour, BoardingRule, DisableRule, LegalCode, NovaAi, NovaBoarding, NovaDisable,
    NovaLaw, PilotCatalog, PointDefenceRule, Session,
};

use super::SharedChance;
use super::sim_port::{CommSource, FlightSim, MapSource, PortSource};

/// The flight of a real `nova_sim::Session`, or why it could not start,
/// with the collaborators its calls are handed (see the module docs).
/// `C` is the catalog the view reads; it names the flight the view's
/// default `FlightView<C>` shows. The view shares its handle on it with
/// the flight as it is read ([`SessionFlight::share_catalog`]).
#[derive(Clone, Debug)]
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
    /// The catalog the view reads, the same handle as the view's, once
    /// the view has shared it ([`SessionFlight::share_catalog`]): the
    /// map's answers that read the catalog read it. `None` before.
    catalog: Option<SharedCatalog>,
    /// `C` names the view's catalog: the flight the view's default
    /// `FlightView<C>` shows.
    view_catalog: PhantomData<C>,
}

/// The view's catalog, as the flight reads it: as a `PilotCatalog` only,
/// so the adapter's `FlightSim` asks nothing of `C`.
#[derive(Clone)]
#[allow(dead_code)] // read by the map's answers, which come next
struct SharedCatalog(Rc<dyn PilotCatalog>);

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
            catalog: None,
            view_catalog: PhantomData,
        }
    }

    /// Shares the view's catalog with the flight: the view's own handle,
    /// so the flight's answers read the catalog the view draws from, and
    /// a test that records a read on it sees the flight's too.
    pub(in crate::flight) fn share_catalog(&mut self, catalog: Rc<dyn PilotCatalog>) {
        self.catalog = Some(SharedCatalog(catalog));
    }

    /// The catalog the view shared, if it has
    /// ([`SessionFlight::share_catalog`]).
    #[cfg(test)]
    pub(in crate::flight) fn shared_catalog(&self) -> Option<&Rc<dyn PilotCatalog>> {
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

    /// Nothing yet: the view still runs the fixed step's passes itself,
    /// until they move into the session's own step.
    fn step(&mut self, _input: &StepInput) -> StepReport {
        StepReport::default()
    }

    fn apply(&mut self, command: FlightCommand) -> Answer {
        match &mut self.session {
            Ok(session) => session.apply(command),
            Err(_) => Answer::Refused,
        }
    }

    /// Nothing yet: the view still takes the session's one-shot outputs
    /// itself, until they are drained as one stream of events.
    fn drain(&mut self, _into: &mut Vec<FlightEvent>) {}
}

impl<C> MapSource for SessionFlight<C> {}

impl<C> CommSource for SessionFlight<C> {}

impl<C> PortSource for SessionFlight<C> {}

#[cfg(test)]
mod tests {
    use nova_sim::navigation::NavMode;
    use nova_sim::{StellarId, StellarPick};

    use super::*;

    fn failed() -> SessionFlight<()> {
        SessionFlight::new(Err("no chär".to_owned()))
    }

    #[test]
    fn a_failed_flight_fills_the_default_frame() {
        let mut frame = FlightFrame {
            nav_mode: NavMode::Stellar(Some(StellarId(128))),
        };
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
    fn a_failed_flight_gives_why_it_could_not_start() {
        let mut flight = failed();
        assert_eq!(flight.session().err(), Some("no chär"));
        assert_eq!(flight.session_mut().err(), Some("no chär"));
    }
}
