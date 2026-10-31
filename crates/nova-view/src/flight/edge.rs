//! The edge that builds a flight: [`open_flight`] builds the session by the
//! setup's rulebook, wraps it in a [`SessionFlight`] with its
//! collaborators, hands it to the view ([`FlightView::over`]), and then
//! sets the session up ([`SessionFlight::set_up`]) and shows the view what
//! that changed ([`FlightView::show_set_up`]). Entering play pairs the
//! view's half with the adapter's: [`begin_game`] for a new pilot,
//! [`enter_game`] from the main screen.
//!
//! **The opening order.** The session is built by the setup's rulebook
//! whichever way the flight starts, so the view's opening read is already
//! the chosen reading: the course map shows the rulebook's links from the
//! first frame, with no `GalaxyChanged` to wait for. The rest of the setup
//! comes after the read: the disable rule, hire terms, strings, person
//! rules, control bits and the run's history. Handing the rulebook in again
//! there changes nothing.
//!
//! **One rulebook and one chance, by construction.** The setup's rulebook
//! is the session's rules, a load's fallbacks and the view's presentation
//! rules alike, and the collaborators' chance is the flight's and the
//! view's wording's.
//!
//! The `fixture` feature (and this crate's tests) exposes `test_flight`:
//! the conveniences that build a bare flight with no setup, as the view's
//! old constructors did.

use std::rc::Rc;

use nova_sim::{AsteroidCatalog, CombatCatalog, Pilot, Rulebook, SessionSetup, SimCatalog};

use super::catalog::{CombatLooks, ShipSprites, StatusBars};
use super::escorts::EscortMenuLooks;
use super::sim_adapter::{Collaborators, SessionFlight};
use super::view::FlightView;
use crate::galaxy::GalaxyCatalog;
use crate::system::catalog::SystemCatalog;

/// How a flight starts: the pilot it flies, and whether it is loaded.
#[derive(Clone, Debug)]
pub enum FlightStart {
    /// A new, unnamed pilot (`nova_sim::Session::start_ruled`): the
    /// developer's flight.
    New,
    /// `Pilot`, just made: built as a load is, by the setup's rulebook
    /// (`nova_sim::Session::load`), with no fallback to take.
    Pilot(Pilot),
    /// `Pilot`, loaded from a save, by the setup's rulebook, its load's
    /// fallbacks among what reads it (`nova_sim::Session::load`).
    Loaded(Pilot),
}

/// What the edge's catalog must be: the sim's catalogs, which the flight
/// reads, and the view's own.
pub trait EdgeCatalog:
    SimCatalog
    + CombatCatalog
    + AsteroidCatalog
    + CombatLooks
    + SystemCatalog
    + ShipSprites
    + StatusBars
    + GalaxyCatalog
    + EscortMenuLooks
{
}

impl<
    C: SimCatalog
        + CombatCatalog
        + AsteroidCatalog
        + CombatLooks
        + SystemCatalog
        + ShipSprites
        + StatusBars
        + GalaxyCatalog
        + EscortMenuLooks,
> EdgeCatalog for C
{
}

/// The flight `start` says, over `catalog`, set up with `setup`, its calls
/// handed `collaborators` (see the module docs): the session is built by
/// the setup's rulebook, the view reads it as it opens, by that rulebook
/// and on the collaborators' chance, and the rest of the setup comes after.
pub fn open_flight<C: EdgeCatalog + 'static>(
    catalog: C,
    start: FlightStart,
    setup: SessionSetup,
    collaborators: Collaborators,
) -> FlightView<C> {
    let slots_carried = setup.carry.slot_sprites.is_some();
    let mut view = opened(Rc::new(catalog), start, &setup.rules, collaborators);
    view.change_sim(|flight| flight.set_up(setup));
    view.show_set_up(slots_carried);
    view
}

/// Begins a new pilot's game on `flight`: the adapter's half
/// ([`SessionFlight::begin`], the `chär`'s `OnStart` on the flight's
/// chance) paired with the view's ([`FlightView::begin`]), which draws the
/// ship where the beginning put it and scatters the stars afresh. A flight
/// that begins nothing changes nothing.
pub fn begin_game<C: SystemCatalog + ShipSprites + CombatLooks>(flight: &mut FlightView<C>) {
    flight.begin(SessionFlight::begin);
}

/// Enters play from the main screen on `flight`: the adapter's half
/// ([`SessionFlight::enter_game`]) paired with the view's
/// ([`FlightView::enter_game`]), which scatters the stars afresh. A flight
/// that enters nothing changes nothing.
pub fn enter_game<C: SystemCatalog + ShipSprites + CombatLooks>(flight: &mut FlightView<C>) {
    flight.enter_game(SessionFlight::enter_game);
}

/// The bare opening, which [`open_flight`] sets up after and the test
/// flights leave bare: the [`SessionFlight`] `start` says over `catalog`,
/// built by `rules`, its calls handed `collaborators`, read by the view
/// ([`FlightView::over`]) by `rules`, its wording on the collaborators'
/// chance.
fn opened<C: EdgeCatalog + 'static>(
    catalog: Rc<C>,
    start: FlightStart,
    rules: &Rulebook,
    collaborators: Collaborators,
) -> FlightView<C> {
    let chance = collaborators.chance.clone();
    let sim = match start {
        FlightStart::New => SessionFlight::start_ruled(&catalog, rules),
        FlightStart::Pilot(pilot) | FlightStart::Loaded(pilot) => {
            SessionFlight::load(&catalog, pilot, rules)
        }
    }
    .with_collaborators(collaborators);
    let opening = sim.opening();
    FlightView::over(catalog, sim, opening, rules, chance)
}

/// Conveniences for tests: a bare flight, built as the view's constructors
/// once built it, with no setup applied and Nova's collaborators unless
/// given; each the view of a [`SessionFlight`] read by
/// [`FlightView::over`].
#[cfg(any(test, feature = "fixture"))]
pub mod test_flight {
    use std::rc::Rc;

    use nova_sim::Rulebook;

    use super::{Collaborators, EdgeCatalog, FlightStart, FlightView};

    /// A new, unnamed pilot's flight over `catalog`, shown by the default
    /// rulebook, with Nova's collaborators.
    #[must_use]
    pub fn new<C: EdgeCatalog + 'static>(catalog: C) -> FlightView<C> {
        flown(catalog, FlightStart::New, Collaborators::default())
    }

    /// The flight `start` says over `catalog`, its calls handed
    /// `collaborators`, the view's wording on their chance, shown by the
    /// default rulebook, with no setup. The session is built by the default
    /// rulebook, a loaded pilot's fallbacks among what reads it.
    #[must_use]
    pub fn flown<C: EdgeCatalog + 'static>(
        catalog: C,
        start: FlightStart,
        collaborators: Collaborators,
    ) -> FlightView<C> {
        super::opened(Rc::new(catalog), start, &Rulebook::default(), collaborators)
    }
}
