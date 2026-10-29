//! The session's side of domination (see `nova_sim::domination`, the one
//! rule its readers go through):
//!
//! - **The writer** ([`Chart::set_dominated`]): the planet comm dialog's
//!   Demand Tribute (`_DoPlanetCommDialog` @0x978fa) and Release
//!   (@0x9740f) write the galaxy overlay through it, which stamps the
//!   stellars shown, listed and waiting for the take-off afresh, so every
//!   reader of a site sees the change at once.
//!
//! Nothing here plays a sound.

use super::{Chart, Outbox, SessionEvent};
use crate::catalog::StellarId;
use crate::pilot::Pilot;

impl Chart {
    /// Sets whether the player dominates `stellar`, writing `pilot`'s
    /// overlay and the galaxy's: a change stamps the stellars shown, those
    /// the system lists and those waiting for the take-off afresh, tells
    /// [`SessionEvent::GalaxyChanged`] and makes a save due. Whether it
    /// changed anything.
    pub(super) fn set_dominated(
        &mut self,
        pilot: &mut Pilot,
        out: &mut Outbox,
        stellar: StellarId,
        dominated: bool,
    ) -> bool {
        let changed = self
            .write_galaxy(pilot, out, |overlay| {
                overlay.set_dominated(stellar, dominated);
            })
            .is_some();
        if changed {
            self.stamp_sites();
            out.emit(SessionEvent::GalaxyChanged);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domination;
    use crate::session::Session;
    use crate::testkit::catalog;

    const EARTH: StellarId = StellarId(128);

    /// Whether `session` shows and lists Earth dominated.
    fn earth_dominated(session: &Session) -> (bool, bool) {
        let shown = session.chart.sites.iter().find(|site| site.id == EARTH);
        let listed = session.chart.listed.iter().find(|site| site.id == EARTH);
        (
            shown.is_some_and(domination::dominated),
            listed.is_some_and(domination::dominated),
        )
    }

    #[test]
    fn dominating_stamps_the_sites_shown_and_listed_and_tells_the_change() {
        let catalog = catalog();
        let mut session = Session::start(&catalog).expect("starts");
        session.take_events();
        assert_eq!(earth_dominated(&session), (false, false));
        assert!(
            session
                .chart
                .set_dominated(&mut session.pilot, &mut session.out, EARTH, true)
        );
        assert_eq!(earth_dominated(&session), (true, true));
        assert!(session.pilot.galaxy.stellar(EARTH).dominated, "the pilot's");
        assert!(session.out.save_due);
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
        assert!(
            !session
                .chart
                .set_dominated(&mut session.pilot, &mut session.out, EARTH, true),
            "no change"
        );
        assert_eq!(session.take_events(), []);
        assert!(
            session
                .chart
                .set_dominated(&mut session.pilot, &mut session.out, EARTH, false)
        );
        assert_eq!(earth_dominated(&session), (false, false), "released");
        assert_eq!(session.take_events(), [SessionEvent::GalaxyChanged]);
    }
}
