//! The port's own `dësc`, loaded where the original loads it.
//!
//! `_PlayerLandOnStellar` calls `_DoPortDialog` unconditionally
//! (@0x63381). Past its prologue, which writes the port globals and empties
//! the text buffer (@0x5f92c-0x5f95a), `_DoPortDialog` loads the spöb's own
//! `dësc`, `index + 128` (@0x5f95d-0x5f96b): its only `_LoadDescription`,
//! so a stay never reloads it. Nothing between it and `_MissionLandCheck`
//! (@0x5fbeb) loads another, so the port's text resolves its conditional
//! text before the land checks' scripts run, and the last loaded movie and
//! the remembered rank governments after the landing are the last landing
//! text's, else the port's.
//!
//! The session does the same. [`Session::dock`](super::Session::dock) marks the
//! load due ([`Port::due_own_desc`](super::port::Port::due_own_desc)), and
//! [`Session::load_own_desc`] runs it the first time either of these comes:
//!
//! - the spaceport asks for its text
//!   ([`FlightCommand::LoadPortDescription`], answered by
//!   [`Session::port_description`]), as it does as it opens with movies off,
//!   before the settle;
//! - the landing settles ([`Session::settle_script`]), before it dispatches
//!   the land checks.
//!
//! Either way it is loaded once, before the land checks, and the spaceport
//! is served the text it loaded. A missing `dësc` empties the movie and
//! keeps the governments, as [`Session::load_description`] does.
//!
//! Left out: `_DoPortDialog`'s return on a faded screen (@0x5f91a-0x5f926,
//! `*0x3b5198` set, to @0x6025e), which loads nothing. The engine never
//! lands with the screen faded: the press is refused while jumping, flight
//! holds under the jump's white, the pod can't land, and a hypergate's
//! `_FadeWhiteOut` (@0x63c40) clears the flag.
//!
//! [`FlightCommand::LoadPortDescription`]: crate::flight_frame::FlightCommand::LoadPortDescription

use super::Session;
use crate::PilotCatalog;
use crate::catalog::StellarId;
use crate::chance::Chance;

impl Session {
    /// Loads the port's own `dësc` if a landing has made it due and the
    /// ship is still landed there (`_DoPortDialog` @0x5f96b), read from
    /// `catalog`, its `l33t` letters drawn on `chance`. Run at the top of
    /// [`Session::settle_script`], so it comes before the land checks
    /// (`_MissionLandCheck` @0x5fbeb), as it does in the original.
    pub(super) fn load_own_desc(
        &mut self,
        catalog: &(impl PilotCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) {
        let Some(stellar) = self.port.own_desc_due() else {
            return;
        };
        if self.mode.landed() == Some(stellar) {
            self.load_own_desc_at(stellar, catalog, chance);
        }
    }

    /// The port's own `dësc` as the spaceport shows it, landed: loaded now
    /// if it is not loaded for the stellar landed on yet (it is due, or the
    /// session started or loaded landed), else the text it loaded, never
    /// loaded again. Empty for a missing one; `None` in flight.
    pub fn port_description(
        &mut self,
        catalog: &(impl PilotCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) -> Option<String> {
        let stellar = self.mode.landed()?;
        if self.port.own_desc_text(stellar).is_none() {
            self.load_own_desc_at(stellar, catalog, chance);
        }
        self.port.own_desc_text(stellar).map(str::to_owned)
    }

    /// Loads `stellar`'s own `dësc`, `dësc` `stellar`, read from
    /// `catalog`, and notes it loaded.
    fn load_own_desc_at(
        &mut self,
        stellar: StellarId,
        catalog: &(impl PilotCatalog + ?Sized),
        chance: &mut dyn Chance,
    ) {
        let desc = catalog.desc_text(stellar.0);
        let text = desc.as_ref().map(|desc| desc.text.as_str());
        let movie = desc.as_ref().and_then(|desc| desc.movie.as_ref());
        let loaded = self.load_description(text, movie, chance);
        self.port.own_desc_loaded(stellar, loaded);
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{
        DescMovie, DescText, MissionId, MissionRecord, MissionTexts, StellarId, SystemId,
        TextMovies,
    };
    use crate::chance::NeverFires;
    use crate::control::{Bit, Script};
    use crate::geometry::Vec2;
    use crate::session::Session;
    use crate::testkit::{FakePilotCatalog, catalog, land_now, mission};

    /// A `dësc` of `text` naming `movie`, played after its dialog.
    fn desc(text: &str, movie: &str) -> DescText {
        DescText {
            text: text.to_owned(),
            movie: DescMovie::from_desc(movie, 0x0001),
        }
    }

    /// [`catalog`] with 131 linked back to 130, stellar 129's `dësc`
    /// `port`, and mission 130, offered at 128 and completed at 129 with
    /// a `CompText` naming `Comp.mov` and an `OnSuccess` of `on_success`.
    fn port_catalog(port: DescText, on_success: &str) -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.star_map[1].links.push(SystemId(130));
        catalog.descs = vec![(129, port)];
        catalog.missions = vec![MissionRecord {
            avail_stel: 128,
            travel_stel: 129,
            on_success: Script::parse(on_success),
            ..mission(130)
        }];
        catalog.mission_texts = vec![MissionTexts {
            comp: Some("Done.".to_owned()),
            movies: TextMovies {
                comp: DescMovie::from_desc("Comp.mov", 0x0001),
                ..TextMovies::default()
            },
            ..MissionTexts::named(MissionId(130), "Run")
        }];
        catalog
    }

    /// A session of `catalog` landed on 128 and settled, mission 130
    /// accepted if `accept`, then taken off and at rest over 129, its
    /// reads and texts so far forgotten.
    fn over_129(catalog: &FakePilotCatalog, accept: bool) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        assert_eq!(land_now(&mut session), Ok(StellarId(128)));
        let _ = session.settle_script(catalog, &mut NeverFires);
        if accept {
            session
                .accept_mission(MissionId(130), &mut NeverFires)
                .expect("accepted");
        }
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(catalog, &mut NeverFires);
        let site = *session
            .chart
            .sites
            .iter()
            .find(|site| site.id == StellarId(129))
            .expect("shown");
        session.ship.player.position = site.position;
        session.ship.player.velocity = Vec2::ZERO;
        let _ = session.out.take_texts_only();
        catalog.desc_reads.borrow_mut().clear();
        session
    }

    fn settle(session: &mut Session, catalog: &FakePilotCatalog) {
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    fn ask(session: &mut Session, catalog: &FakePilotCatalog) -> Option<String> {
        session.port_description(catalog, &mut NeverFires)
    }

    #[test]
    fn a_landing_loads_the_ports_desc_before_the_land_checks() {
        let catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        let mut session = over_129(&catalog, true);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        assert!(
            catalog.desc_reads.borrow().is_empty(),
            "the dock only marks it due"
        );
        settle(&mut session, &catalog);
        assert!(session.pilot().missions().get(0).is_none(), "completed");
        assert_eq!(
            session.run.loaded_movie,
            DescMovie::from_desc("Comp.mov", 0x0001),
            "the port's (@0x5f96b), then the CompText's (@0x5fbeb)"
        );
        assert_eq!(*catalog.desc_reads.borrow(), [129]);

        let mut session = over_129(&catalog, false);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        settle(&mut session, &catalog);
        assert_eq!(
            session.run.loaded_movie,
            DescMovie::from_desc("Port.mov", 0x0001),
            "no landing text: the port's is the last loaded"
        );
    }

    #[test]
    fn the_ports_text_reads_its_conditional_text_before_the_land_checks_scripts() {
        let catalog = port_catalog(desc("{b5 \"set\" \"clear\"}", ""), "b5");
        let mut session = over_129(&catalog, true);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        settle(&mut session, &catalog);
        assert!(
            session.control_bit(Bit::new(5).expect("a bit")),
            "the OnSuccess ran"
        );
        assert_eq!(ask(&mut session, &catalog), Some("clear".to_owned()));
    }

    #[test]
    fn asking_for_the_ports_text_after_the_settle_loads_nothing() {
        let catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        let mut session = over_129(&catalog, true);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        settle(&mut session, &catalog);
        let govts = session.run.rank_govts;
        assert_eq!(ask(&mut session, &catalog), Some("Docked.".to_owned()));
        assert_eq!(ask(&mut session, &catalog), Some("Docked.".to_owned()));
        assert_eq!(
            session.run.loaded_movie,
            DescMovie::from_desc("Comp.mov", 0x0001),
            "as the land checks left it"
        );
        assert_eq!(session.run.rank_govts, govts);
        assert_eq!(*catalog.desc_reads.borrow(), [129], "read once");
    }

    #[test]
    fn asking_before_the_settle_loads_it_then_and_the_settle_does_not_reload() {
        let catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        let mut session = over_129(&catalog, true);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        assert_eq!(ask(&mut session, &catalog), Some("Docked.".to_owned()));
        assert_eq!(
            session.run.loaded_movie,
            DescMovie::from_desc("Port.mov", 0x0001)
        );
        settle(&mut session, &catalog);
        assert_eq!(
            session.run.loaded_movie,
            DescMovie::from_desc("Comp.mov", 0x0001),
            "the land checks still come after it"
        );
        assert_eq!(*catalog.desc_reads.borrow(), [129], "read once");
    }

    #[test]
    fn a_missing_port_desc_empties_the_movie_and_keeps_the_governments() {
        let mut catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        catalog.descs.clear();
        let mut session = over_129(&catalog, false);
        let _ = session.load_description(
            Some("<PRK130> leads."),
            DescMovie::from_desc("Earlier.mov", 0x0001).as_ref(),
            &mut NeverFires,
        );
        let govts = session.run.rank_govts;
        assert!(govts.prk.is_some(), "the earlier text's");
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        settle(&mut session, &catalog);
        assert_eq!(session.run.loaded_movie, None, "@0x71a5e");
        assert_eq!(session.run.rank_govts, govts, "never reset or parsed");
        assert_eq!(ask(&mut session, &catalog), Some(String::new()));
        assert_eq!(*catalog.desc_reads.borrow(), [129], "read once");
    }

    #[test]
    fn the_remembered_governments_after_a_landing_are_the_last_landing_texts() {
        let mut catalog = port_catalog(desc("<PRK131> rules here.", ""), "");
        catalog.mission_texts[0].comp = Some("<PRK132> thanks you.".to_owned());
        let mut session = over_129(&catalog, true);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        let _ = ask(&mut session, &catalog);
        settle(&mut session, &catalog);
        assert_eq!(
            session.run.rank_govts.prk.map(|govt| govt.0),
            Some(132),
            "the CompText's, loaded after the port's"
        );

        let mut session = over_129(&catalog, false);
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        settle(&mut session, &catalog);
        assert_eq!(session.run.rank_govts.prk.map(|govt| govt.0), Some(131));
    }

    #[test]
    fn the_port_is_asked_for_only_landed_and_loads_for_a_session_started_landed() {
        let catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        let mut session = over_129(&catalog, false);
        assert_eq!(ask(&mut session, &catalog), None, "in flight");
        assert!(catalog.desc_reads.borrow().is_empty(), "nothing read");
        assert_eq!(land_now(&mut session), Ok(StellarId(129)));
        session.port.own_desc = super::super::port::OwnDesc::None;
        assert_eq!(
            ask(&mut session, &catalog),
            Some("Docked.".to_owned()),
            "landed with nothing due, as a session loaded landed"
        );
    }

    #[test]
    fn a_due_load_the_ship_took_off_from_is_not_run() {
        let catalog = port_catalog(desc("Docked.", "Port.mov"), "");
        let mut session = over_129(&catalog, false);
        session.port.due_own_desc(StellarId(129));
        settle(&mut session, &catalog);
        assert!(catalog.desc_reads.borrow().is_empty(), "not landed there");
    }
}
