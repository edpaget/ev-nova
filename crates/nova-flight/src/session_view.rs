//! [`SessionView`], the flight view of a real session, and
//! [`SessionViewExt`], the reads of that session the wiring tests make.
//!
//! The two reads were concrete members of nova-view's `FlightView` over
//! a [`SessionFlight`] until nova-view stopped depending on nova-sim
//! (rdm:phase/faster-builds/phase-6-nova-view-off-nova-sim). With the
//! trait in scope (`use nova_flight::SessionViewExt as _;`), a call reads
//! as it did: `view.session()`, `FlightView::from_session(..)`.

use std::rc::Rc;

use nova_sim::{Rulebook, Session, SimCatalog};
use nova_view::flight::FlightView;
use nova_view::flight::catalog::{CombatLooks, ShipSprites, StatusBars};
use nova_view::flight::escorts::EscortMenuLooks;
use nova_view::galaxy::GalaxyCatalog;
use nova_view::system::catalog::SystemCatalog;

use crate::sim_adapter::SessionFlight;

/// The flight view of a real session: the view the app shows.
pub type SessionView<C> = FlightView<C, SessionFlight<C>>;

/// The reads of a [`SessionView`]'s session. Wiring-only: for the tests
/// that wire a real session behind the port (this crate's, and the app's
/// in `crates/nova/tests/it` until
/// rdm:phase/crate-decoupling/phase-10-app-tests-wiring-only retires
/// them). View code and component tests never call them; they open the
/// view over `FakeFlight`.
pub trait SessionViewExt<C>: Sized {
    /// The flight's session, or why it could not start.
    fn session(&self) -> Result<&Session, &str>;

    /// The view of `sim` over `catalog`, read as the edge reads a flight
    /// ([`FlightView::over`]) and shown by `rules`, the rulebook its
    /// session was built with (`nova_sim::fixture::SessionBuilder`), so
    /// the view and the session read one: for a test whose session a
    /// builder put into the state it needs, passed as
    /// `builder.build().into()` for Nova's collaborators
    /// ([`SessionFlight::new`]). The catalog is shared with the flight
    /// first, and the wording picks on the flight's chance.
    fn from_session(catalog: C, sim: SessionFlight<C>, rules: &Rulebook) -> Self
    where
        C: SimCatalog
            + CombatLooks
            + SystemCatalog
            + ShipSprites
            + StatusBars
            + GalaxyCatalog
            + EscortMenuLooks
            + 'static;
}

impl<C> SessionViewExt<C> for SessionView<C> {
    fn session(&self) -> Result<&Session, &str> {
        self.sim().session()
    }

    fn from_session(catalog: C, mut sim: SessionFlight<C>, rules: &Rulebook) -> Self
    where
        C: SimCatalog
            + CombatLooks
            + SystemCatalog
            + ShipSprites
            + StatusBars
            + GalaxyCatalog
            + EscortMenuLooks
            + 'static,
    {
        let catalog = Rc::new(catalog);
        sim.share_catalog(Rc::clone(&catalog) as Rc<dyn SimCatalog>);
        let chance = sim.chance.clone();
        let opening = sim.opening();
        FlightView::over(catalog, sim, opening, rules, chance)
    }
}
