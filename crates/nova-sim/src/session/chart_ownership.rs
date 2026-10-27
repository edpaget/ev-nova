//! The owner test `_GetStellarSpriteXSize` and `_HandlePlayerDockRequest`
//! run on a stellar: whether its owner word (+0x14) is the player's
//! system (the first ship's +0x74 through 0x3b5138; @0xacb2-0xacc1 and
//! @0x6699a-0x669a4). See the `story` module's "Checked for
//! stellar-fidelity phase 7".
//!
//! The session reads it in four places, each through
//! [`Chart::held_here`]:
//!
//! - the player's landing reach (`Space::landing_reach`), which gives a
//!   stellar not held here the unloaded `Type` width, 150, as
//!   `_GetStellarSpriteXSize`'s owner test does ahead of its `Type`
//!   range and master tests (@0xacb9 before @0xacc3);
//! - the land press, which refuses such a stellar as not landable before
//!   any reach (`_HandlePlayerDockRequest` @0x6699a-0x669a4, then the
//!   not-landable line and `snd ` 153 @0x67cda-0x67e8f);
//! - F5, which drops a nearest pick another system owns, writing nothing
//!   and making no sound (`_HandlePlayer` @0x69e73-0x69e94; see the
//!   `stellar_picks` module);
//! - the NPCs' gate entry box (`Session::gate_type_widths`), which takes
//!   a quarter of the same 150 (`_HighLevelAIHandler` state 20
//!   @0x8dbdf).
//!
//! A stellar the player's system shows is always held there
//! ([`OverlaidGalaxy::shows`](crate::story::OverlaidGalaxy::shows)), so
//! the test bites on a stellar the system lists but another visible
//! system owns, which the land key and F5 pick among
//! ([`Chart::pick_candidates`]), and on a stellar still among the sites
//! after the player's system has changed without a rebuild point.

use super::{Chart, Statics};
use crate::catalog::StellarId;
use crate::pilot::Pilot;

impl Chart {
    /// Whether the player's system holds `stellar`: the stellar's +0x14 as
    /// the session holds it ([`Chart::holder_of`], its owner, else the
    /// rescue to the player's system) is the player's system. The one
    /// place the session reads the original's owner test (see the module
    /// docs). The engine keeps no rescue past the next refresh beyond
    /// what [`holder`](crate::story::OverlaidGalaxy::holder) derives, so
    /// this reads that derivation, as the aux and mission ship readers do.
    pub(super) fn held_here(&self, pilot: &Pilot, statics: &Statics, stellar: StellarId) -> bool {
        self.holder_of(pilot, statics, stellar) == Some(pilot.system)
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{StellarId, SystemId};
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog, hideable, planet};

    /// The testkit catalog with stellar 200 listed by every system, and
    /// every system hideable: 130, the lowest visible lister, owns it.
    fn twice_listed() -> FakePilotCatalog {
        let mut catalog = catalog();
        for (_, sites) in &mut catalog.sites {
            sites.push(planet(200, 500.0, 500.0));
        }
        hideable(&mut catalog);
        catalog
    }

    fn held(session: &Session, stellar: i16) -> bool {
        session
            .chart
            .held_here(&session.pilot, &session.statics, StellarId(stellar))
    }

    #[test]
    fn the_owner_holds_a_stellar_two_systems_list_and_the_other_does_not() {
        let catalog = twice_listed();
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.system(), SystemId(130));
        assert!(held(&session, 200), "130 owns 200");
        assert!(held(&session, 128), "listed by 130 alone");
        session.pilot.system = SystemId(131);
        assert!(!held(&session, 200), "130 owns it, not 131");
        assert!(!held(&session, 128), "131 does not list it");
        assert!(held(&session, 140), "listed by 131 alone");
    }

    #[test]
    fn a_galaxy_refresh_that_hides_the_owner_hands_the_stellar_on() {
        let catalog = twice_listed();
        let mut session = Session::start(&catalog).expect("starts");
        crate::testkit::jump(&mut session, &catalog, 131).expect("arrives");
        assert!(!held(&session, 200), "130 owns 200");
        crate::session::tests::hide(&mut session, &catalog, &[130]);
        assert!(held(&session, 200), "131 owns 200 now");
        session.pilot.system = SystemId(130);
        assert!(!held(&session, 200), "130 is hidden: it holds nothing");
    }
}
