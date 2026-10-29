//! The session's side of domination (see `nova_sim::domination`, the one
//! rule its readers go through):
//!
//! - **The writer** ([`Chart::set_dominated`]): the planet comm dialog's
//!   Demand Tribute (`_DoPlanetCommDialog` @0x978fa) and Release
//!   (@0x9740f) write the galaxy overlay through it, which stamps the
//!   stellars shown, listed and waiting for the take-off afresh, so every
//!   reader of a site sees the change at once.
//! - **The tribute** ([`Session::tribute_day`], the daily step's fourth
//!   participant; `_DoStellarTimePassage` @0x40867-0x408d4, called
//!   @0xb5b8): each stellar some system lists that is active (+0x44,
//!   @0x4087a: its holder visible) and dominated (+0x46, @0x40880) adds
//!   its tribute to the cash (@0x408ab-0x408b3,
//!   [`tribute_of`](crate::player_info::tribute_of)), in ID order. The
//!   player info's tribute row reads the same sum ([`Chart::daily_tribute`];
//!   its holder tests @0x53560-0x53570 are the active test). Left out, as
//!   nothing reads them: the stellar's +0x2a, which the loop increments
//!   behind a test of the nav stellar's `Flags2` (@0x40886-0x408a3), and
//!   `_statusRedraw[2]` (@0x408be).
//!
//! Nothing here plays a sound.

use super::{Chart, Outbox, Session, SessionEvent, Statics};
use crate::catalog::StellarId;
use crate::pilot::Pilot;
use crate::player_info;

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

    /// The day's tribute (`_DoStellarTimePassage` @0x40867-0x408d4): the
    /// tribute of every stellar some system lists that is active and
    /// dominated, in ID order.
    pub(super) fn daily_tribute(&self, pilot: &Pilot, statics: &Statics) -> i64 {
        statics
            .listed_stellars
            .iter()
            .filter(|listed| {
                self.galaxy.dominated(listed.id, listed.flags2)
                    && self
                        .galaxy
                        .active_holder(listed.id, &listed.systems, pilot.system)
                        .is_some()
            })
            .map(player_info::tribute_of)
            .sum()
    }
}

impl Session {
    /// The daily step's fourth participant (`_DoStellarTimePassage`,
    /// called @0xb5b8): the day's tribute ([`Chart::daily_tribute`]) added
    /// to the cash. It plays no sound and draws nothing.
    pub(super) fn tribute_day(&mut self) {
        self.pilot.cash += self.chart.daily_tribute(&self.pilot, &self.statics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chance::NeverFires;
    use crate::domination;
    use crate::testkit::{catalog, land_now, mission};

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

    /// The catalog with Earth paying `tribute` at `TechLevel` 4, and
    /// system 131's 140 paying 1000.
    fn paying(tribute: i16) -> crate::testkit::FakePilotCatalog {
        let mut catalog = catalog();
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut() {
                (site.tribute, site.tech_level) = match site.id.0 {
                    128 => (tribute, 4),
                    140 => (1000, 0),
                    _ => (0, 0),
                };
            }
        }
        catalog
    }

    /// What `stellar` of `session`'s statics pays a day, by the port's
    /// rule.
    fn tribute(session: &Session, stellar: StellarId) -> i64 {
        crate::player_info::tribute_of(
            session
                .statics
                .listed_stellars
                .iter()
                .find(|listed| listed.id == stellar)
                .expect("listed"),
        )
    }

    /// The cash `days` days add in `session`.
    fn paid_over(session: &mut Session, days: u32) -> i64 {
        let before = session.pilot.cash;
        session.pass_days(days, &mut NeverFires);
        session.pilot.cash - before
    }

    #[test]
    fn each_day_pays_the_tribute_of_every_active_dominated_stellar() {
        for raw in [250, 0, -3] {
            let catalog = paying(raw);
            let mut session = Session::start(&catalog).expect("starts");
            assert_eq!(paid_over(&mut session, 1), 0, "none dominated");
            dominate(&mut session, EARTH);
            let earth = tribute(&session, EARTH);
            assert_ne!(earth, 0);
            assert_eq!(paid_over(&mut session, 3), 3 * earth, "{raw}");
            dominate(&mut session, StellarId(140));
            assert_eq!(
                paid_over(&mut session, 1),
                earth + tribute(&session, StellarId(140)),
                "another system's too (@0x40875-0x408ce)"
            );
        }
    }

    #[test]
    fn an_always_dominated_stellar_pays_and_an_inactive_one_does_not() {
        let mut catalog = paying(250);
        catalog.sites[0].1[0].flags2 = crate::market::DOMINATED;
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(paid_over(&mut session, 1), 250, "Flags2 0x0020");
        dominate(&mut session, StellarId(140));
        let mut overlay = session.chart.galaxy.overlay().clone();
        overlay.set_visible(crate::catalog::SystemId(131), Some(false));
        session.chart.galaxy.set_overlay(overlay);
        assert_eq!(
            paid_over(&mut session, 1),
            250,
            "140's holder hidden: not active (+0x44 @0x4087a)"
        );
    }

    #[test]
    fn the_player_infos_tribute_row_reads_the_same_sum() {
        let catalog = paying(250);
        let mut session = Session::start(&catalog).expect("starts");
        dominate(&mut session, EARTH);
        dominate(&mut session, StellarId(140));
        assert_eq!(
            session
                .chart
                .daily_tribute(&session.pilot, &session.statics),
            paid_over(&mut session, 1),
            "@0x53547"
        );
    }

    /// Dominates `stellar` in `session` at run time.
    fn dominate(session: &mut Session, stellar: StellarId) {
        session
            .chart
            .set_dominated(&mut session.pilot, &mut session.out, stellar, true);
    }

    #[test]
    fn missions_needing_a_domination_read_the_ones_made_at_run_time() {
        use crate::catalog::{MissionId, MissionRecord};
        let mut catalog = catalog();
        catalog.missions = vec![
            MissionRecord {
                avail_record: -32000,
                ..mission(140)
            },
            MissionRecord {
                avail_record: -32001,
                ..mission(141)
            },
        ];
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let offered = |session: &Session, id| {
            session
                .offer_refusals(MissionId(id))
                .expect("docked")
                .is_empty()
        };
        assert!(!offered(&session, 140), "none dominated");
        assert!(!offered(&session, 141), "none dominated");
        dominate(&mut session, StellarId(140));
        assert!(!offered(&session, 140), "not the stellar docked at");
        assert!(offered(&session, 141), "system 131's 140 (@0x9b578)");
        dominate(&mut session, EARTH);
        assert!(offered(&session, 140), "the stellar docked at (@0x9b559)");
    }

    #[test]
    fn a_stellar_dominated_at_run_time_marks_its_prices_up_by_1_5() {
        use crate::market::Good;
        let catalog = crate::session::tests::exchange();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        let prices = |session: &Session| {
            let market = session.market().expect("an exchange");
            [Good::Commodity(0), Good::Commodity(4)]
                .map(|good| market.row(good).expect("traded").price)
        };
        let standard = prices(&session);
        dominate(&mut session, EARTH);
        assert_ne!(prices(&session), standard, "1.5 (@0x5dc3a)");
    }
}
