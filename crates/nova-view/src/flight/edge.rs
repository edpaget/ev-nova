//! The edge that builds a flight: [`open_flight`] builds the session, wraps
//! it in a [`SessionFlight`] with its collaborators, hands it to the view
//! ([`FlightView::over`]), and then sets the session up
//! ([`SessionFlight::set_up`]) and shows the view what that changed
//! ([`FlightView::show_set_up`]). Entering play pairs the view's half with
//! the adapter's: [`begin_game`] for a new pilot, [`enter_game`] from the
//! main screen.
//!
//! **The opening order.** The view reads the bare session as it opens, and
//! the setup comes after, as the app's `with_*` builders came after the
//! read: one setup rule changes what the read builds.
//! `RuleKey::ColocatedResolution` rebuilds the galaxy's links as it is
//! set (`OverlaidGalaxy::set_colocated`), after the course map has shown
//! the galaxy, and the map shows the links again only at the next
//! `GalaxyChanged`. Setting the session up before the read would show the
//! other reading's links at once, which the player would see.
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
    /// A new, unnamed pilot (`nova_sim::Session::start`): the developer's
    /// flight.
    New,
    /// `Pilot`, flown as it is (`nova_sim::Session::fly`): a pilot just
    /// made.
    Pilot(Pilot),
    /// `Pilot`, loaded from a save, its load's fallbacks reading the
    /// setup's rulebook (`nova_sim::Session::load`).
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
/// handed `collaborators` (see the module docs): the view reads the bare
/// session as it opens, by the setup's rulebook and on the collaborators'
/// chance, and the session is set up after.
pub fn open_flight<C: EdgeCatalog + 'static>(
    catalog: C,
    start: FlightStart,
    setup: SessionSetup,
    collaborators: Collaborators,
) -> FlightView<C> {
    let slots_carried = setup.carry.slot_sprites.is_some();
    let mut view = opened(Rc::new(catalog), start, &setup.rules, collaborators);
    view.sim_mut().set_up(setup);
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
/// a load's fallbacks reading `rules`, its calls handed `collaborators`,
/// read by the view ([`FlightView::over`]) by `rules`, its wording on the
/// collaborators' chance.
fn opened<C: EdgeCatalog + 'static>(
    catalog: Rc<C>,
    start: FlightStart,
    rules: &Rulebook,
    collaborators: Collaborators,
) -> FlightView<C> {
    let chance = collaborators.chance.clone();
    let sim = match start {
        FlightStart::New => SessionFlight::start(&catalog),
        FlightStart::Pilot(pilot) => SessionFlight::fly(&catalog, pilot),
        FlightStart::Loaded(pilot) => SessionFlight::load(&catalog, pilot, rules),
    }
    .with_collaborators(collaborators);
    FlightView::over(catalog, sim, rules, chance)
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
    /// default rulebook, with no setup. A loaded pilot's fallbacks read the
    /// default rulebook.
    #[must_use]
    pub fn flown<C: EdgeCatalog + 'static>(
        catalog: C,
        start: FlightStart,
        collaborators: Collaborators,
    ) -> FlightView<C> {
        super::opened(Rc::new(catalog), start, &Rulebook::default(), collaborators)
    }
}
