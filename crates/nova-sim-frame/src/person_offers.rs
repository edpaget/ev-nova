//! What hailing the player's target opened ([`Hailed`]), from
//! `nova_sim::session::person_offers`, which re-exports it and keeps the
//! hail and the persons' offers in flight.

use nova_sim_state::shown::escort_dialog::EscortDialogView;
use nova_sim_state::shown::hail::HailView;
use nova_sim_state::shown::mission::screens::VenueOffer;
use nova_sim_state::shown::planet_comm::PlanetCommView;

/// What hailing the player's target opened (`nova_sim::Session::open_hail`).
#[derive(Clone, Debug, PartialEq)]
pub enum Hailed {
    /// The comm dialog, as `nova_sim::Session::hail` opens it.
    Comm(HailView),
    /// The escort dialog, opened in place of the comm dialog for one of
    /// the fleet's escorts (see the `escort_dialog` module).
    Escort(EscortDialogView),
    /// The person's mission, offered in place of the comm dialog
    /// (`_OfferOneMission` @0x62293): the offer dialog, or the mission
    /// accepted at once.
    Offer(VenueOffer),
    /// The navigation stellar, which answers: the planet comm dialog
    /// (`_DoPlanetCommDialog` @0x6204b; see the `planet_comm` module).
    Stellar(PlanetCommView),
    /// The comm dialog, the escort dialog or the planet comm dialog would
    /// have opened, and its `_screenFaded` guard refused it in silence
    /// (`nova_sim::Session::hail_key_faded`).
    Faded,
}
