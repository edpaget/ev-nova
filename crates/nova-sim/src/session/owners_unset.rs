//! The owner words left unset between a refresh, or a landed move, and the
//! next rebuild point ([`RuleKey::RescueUntilRefresh`]'s engine reading).
//! All addresses are in `EV Nova.app`'s `EV Nova` (i386).
//!
//! `_PropagateMissionBitEffects` writes 0xffff to every stellar's owner
//! word (+0x14) and sets it again only where a visible system claims the
//! stellar (@0x9968a-0x997bd); `_HandleStellarSystemVisibility` (HSSV)
//! rescues the rest that the player's system lists (@0x32c58-0x32ca1).
//! Between the two, a stellar neither owned nor rescued has no owner, and
//! the owner-first readers fall back to `_FindSystemFromStellar` (0x7ec1):
//! `_RandomizeOneMission` (@0xa15a8, @0xa1653), `_MungeBriefing`
//! (@0x9c8de, @0x9c9f9) and `_MapDialogUpdate` (@0x10c03). The readers with
//! no fallback (the green map system @0xa1ed5, @0x9e434; the tribute
//! @0x5353f-0x5355d) find no system. Two windows reach those readers:
//!
//! | Window | Opens | Closes | Readers inside |
//! |---|---|---|---|
//! | A jump | the refresh @0x6bf3f | the HSSV @0x6c203 | the days' crons and `OnRegen`s (@0x6c144), the setup's persons (`_SetupShipsInSystem` @0x6c1f9) |
//! | A landed `M` or `N` | the write of +0x74 (@0x1572b, @0x158f2) | the take-off's HSSV (@0x674c7) | a later docked script's `S`, mission info, the map, the tribute, the take-off's persons (@0x674c2) |
//!
//! **Here.** [`Chart::leave_owners_unset`] makes the galaxy await its
//! rebuild ([`OverlaidGalaxy::await_rebuild`](crate::story::OverlaidGalaxy::await_rebuild)):
//! the holder then derives no rescue for the player's system, and the
//! next rescue pass (`Chart::keep_rescues`, at every rebuild point and
//! after every refresh) ends the wait. `Session::arrive` writes the jump's
//! refresh, forgets the rescues and leaves the owners unset before the
//! days, so the wait lasts through the entry's population and ends at its
//! rebuild. A landed `M` or `N` (`Session::move_to`) leaves them unset
//! until the take-off's rebuild, which follows its population, or until a
//! landing's refresh: a land check's `M` settles before the `REOFFER`
//! refresh under `story_reentry = engine`, so it leaves no window, as in
//! the original (@0xa1a92, @0xa1af2). Under `story_reentry = bible` it
//! settles after that refresh and keeps the wait to the take-off. An
//! `N` out of range moves nowhere and writes no system
//! (`Session::stay`), so it leaves nothing unset; nor do the developer
//! tools' moves, which are not the original's.
//!
//! The stellars shown keep deriving the player's rescue
//! ([`OverlaidGalaxy::shows`](crate::story::OverlaidGalaxy::shows)), as
//! they are what each rebuild derives. During the jump's days they stay
//! the departure's until the entry; nothing is drawn between the
//! original's refresh and its HSSV, so that crosses no boundary. Docked
//! after a landed move they stay the docked system's until the take-off's
//! rebuild, before its first frame.
//!
//! `_SelectMissionStellar`'s draws test the active flag (+0x44), which
//! only HSSV writes, and then the owner's visible byte: for a stellar
//! with a stale active flag and no owner it reads system -1's, outside the
//! system table. That can't be replicated; the engine skips such a
//! stellar (`nova_sim_story::mission::select`).
//!
//! By the other reading nothing is left unset: the holder keeps deriving
//! the player's system. The wait plays no sound and draws nothing.

use super::{Chart, Setup};
use crate::rulebook::{RuleKey, RuleSource};

impl Chart {
    /// Leaves the owner words unset until the next rebuild point (see the
    /// module docs), by the engine reading of
    /// [`RuleKey::RescueUntilRefresh`]; by the other reading, nothing.
    pub(super) fn leave_owners_unset(&mut self, setup: &Setup) {
        if setup.rules.source_for(RuleKey::RescueUntilRefresh) == RuleSource::Engine {
            self.galaxy.await_rebuild();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::crons::tests::scripted;
    use super::super::kept_rescues::tests::{SHARED, by, rescued_in_130, shared};
    use crate::catalog::{
        LandingSite, MissionId, MissionRecord, PersonId, PersonRecord, StellarId, SystemId,
        SystemTraffic,
    };
    use crate::chance::NeverFires;
    use crate::control::{Script, SetExpr};
    use crate::galaxy_map::MARKS_DESTINATION;
    use crate::market::DOMINATED;
    use crate::pilot::Pilot;
    use crate::player_info::IncomeLine;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::session::Session;
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, catalog, fly_out, hideable, hiding_bit, land_now,
        mission, person, planet, ship, star,
    };

    /// The mission whose travel and return stellar is [`SHARED`].
    const TO_SHARED: MissionId = MissionId(140);

    /// The mission [`TO_SHARED`]'s person offers.
    const OFFERED: MissionId = MissionId(160);

    /// A mission going to and returning at `stellar`, marking its system.
    fn going_to(id: MissionId, stellar: i16) -> MissionRecord {
        MissionRecord {
            travel_stel: stellar,
            return_stel: stellar,
            flags: MARKS_DESTINATION,
            ..mission(id.0)
        }
    }

    /// `catalog` with `system`'s traffic one person, "Ace" (`përs` 600,
    /// ship 129) at 100 %, linked to [`OFFERED`], which goes to 150.
    fn with_person_in(mut catalog: FakePilotCatalog, system: i16) -> FakePilotCatalog {
        let mut slots: [(Option<PersonId>, i16); 8] = Default::default();
        slots[0] = (Some(PersonId(600)), 100);
        catalog.traffic.push((
            SystemId(system),
            SystemTraffic {
                dude_types: [(-1, 0); 8],
                avg_ships: 0,
                persons: slots,
            },
        ));
        catalog.persons.push(PersonRecord {
            name: "Ace".to_owned(),
            link_mission: Some(OFFERED.0),
            ..person(600, 129)
        });
        catalog.ship_records.push(ship(129, FAST));
        catalog.missions.push(going_to(OFFERED, 150));
        catalog
    }

    /// The return system of [`OFFERED`]'s offer from a ship.
    fn offered_return(session: &Session) -> Option<SystemId> {
        session
            .run
            .ship_offer(OFFERED)
            .expect("offered")
            .return_to
            .and_then(|stop| stop.system)
    }

    // Window (a): the jump.

    /// [`shared`] with 128, shadowed by 127 at (-600, 600), listing 150
    /// too: the lowest visible lister, which `_FindSystemFromStellar`
    /// gives while no system owns or rescues 150. Mission [`TO_SHARED`]
    /// goes to 150.
    fn found_in_128() -> FakePilotCatalog {
        let mut catalog = shared();
        catalog.star_map.push(star(127, (-600.0, 600.0), &[]));
        catalog.star_map.push(star(128, (-600.0, 600.0), &[]));
        catalog.systems.push(SystemId(128));
        catalog
            .sites
            .push((SystemId(128), vec![planet(150, 400.0, 300.0)]));
        hideable(&mut catalog);
        catalog.missions.push(going_to(TO_SHARED, 150));
        catalog
    }

    /// `session` jumped from 130 to 131.
    fn jump_to_131(session: &mut Session, catalog: &FakePilotCatalog) {
        session.plot_course(SystemId(131)).expect("a route");
        fly_out(session);
        assert_eq!(begin_jump_now(session), Ok(SystemId(131)));
        assert_eq!(
            session.arrive(catalog, &mut NeverFires),
            Some(SystemId(131))
        );
    }

    /// The (destination, return) systems of the mission held in slot 0.
    fn held_systems(session: &Session) -> (Option<SystemId>, Option<SystemId>) {
        let held = session.pilot().missions().get(0).expect("held");
        (
            held.destination.and_then(|stop| stop.system),
            held.return_to.and_then(|stop| stop.system),
        )
    }

    #[test]
    fn a_crons_s_on_a_jumps_day_takes_the_found_system_for_an_unrescued_stellar() {
        let mut catalog = found_in_128();
        catalog.crons.push(scripted(128, 1, "S140", ""));
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            assert!(session.pilot().missions().get(0).is_none(), "{source:?}");
            jump_to_131(&mut session, &catalog);
            // By the other reading the holder derives the departure 130,
            // the player's system during the days (phase 40 moves it to
            // the arrival); 129 stands in for it at its place.
            assert_eq!(
                held_systems(&session),
                by(
                    source,
                    (Some(SystemId(128)), Some(SystemId(128))),
                    (Some(SystemId(129)), Some(SystemId(130)))
                ),
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_arrivals_persons_take_the_found_system_for_an_unrescued_stellar() {
        let catalog = with_person_in(found_in_128(), 131);
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            jump_to_131(&mut session, &catalog);
            assert_eq!(
                offered_return(&session),
                by(source, Some(SystemId(128)), Some(SystemId(131))),
                "{source:?}"
            );
            assert_eq!(
                session.galaxy().holder(
                    SHARED,
                    session.statics.listers(SHARED),
                    session.pilot().system()
                ),
                Some(SystemId(131)),
                "{source:?}: the entry's rebuild ended the wait"
            );
            assert!(!session.galaxy().awaits_rebuild(), "{source:?}");
        }
    }

    #[test]
    fn the_days_read_the_ownership_the_jumps_refresh_wrote() {
        // 133 owns 160 until the jump's refresh hides it; 128, shadowed,
        // lists it too.
        let mut catalog = found_in_128();
        catalog.systems.push(SystemId(133));
        catalog
            .sites
            .push((SystemId(133), vec![planet(160, 0.0, 0.0)]));
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(128) {
                sites.push(planet(160, 0.0, 0.0));
            }
        }
        catalog.missions.push(going_to(MissionId(141), 160));
        catalog.crons.push(scripted(128, 1, "S141", ""));
        for source in RuleSource::ALL {
            let mut session = rescued_in_130(&catalog, source);
            let listers = session.statics.listers(StellarId(160)).to_vec();
            assert_eq!(
                session
                    .galaxy()
                    .holder(StellarId(160), &listers, SystemId(130)),
                Some(SystemId(133)),
                "{source:?}"
            );
            session.pilot.bits_mut().set(hiding_bit(133));
            jump_to_131(&mut session, &catalog);
            assert_eq!(
                held_systems(&session),
                (Some(SystemId(128)), Some(SystemId(128))),
                "{source:?}: 133 hidden by the refresh before the day"
            );
        }
    }

    // Window (b): a landed move.

    /// [`catalog`] with 125, shadowed by 124 at (-600, 600), and 131,
    /// shadowed by 126 at its place, listing planet 150, dominated and
    /// paying 250: no system owns it, and `_FindSystemFromStellar` gives
    /// 125. Mission [`TO_SHARED`] goes to 150.
    fn listed_by_125_and_131() -> FakePilotCatalog {
        let mut catalog = catalog();
        let tributary = LandingSite {
            flags2: DOMINATED,
            tribute: 250,
            ..planet(150, 400.0, 300.0)
        };
        catalog.star_map.extend([
            star(124, (-600.0, 600.0), &[]),
            star(125, (-600.0, 600.0), &[]),
            star(126, (600.0, 0.0), &[]),
        ]);
        catalog.systems.push(SystemId(125));
        catalog.sites.push((SystemId(125), vec![tributary]));
        for (system, sites) in &mut catalog.sites {
            if *system == SystemId(131) {
                sites.push(tributary);
            }
        }
        catalog.missions.push(going_to(TO_SHARED, 150));
        catalog
    }

    /// `catalog`'s new pilot flown, reading `rescue_until_refresh` as
    /// `source` and `story_reentry` as `reentry`.
    fn flown(catalog: &FakePilotCatalog, source: RuleSource, reentry: RuleSource) -> Session {
        let pilot = Pilot::new(catalog, "").expect("starts");
        Session::fly(catalog, pilot).expect("flies").with_rules(
            Rulebook::default()
                .with_override(RuleKey::RescueUntilRefresh, source)
                .with_override(RuleKey::StoryReentry, reentry),
        )
    }

    /// `catalog`'s new pilot landed on 128 in 130, which does not list
    /// 150, then moved to 131 by a landed `M131`.
    fn moved_while_landed(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let mut session = flown(catalog, source, RuleSource::Engine);
        assert_eq!(land_now(&mut session), Ok(StellarId(128)), "{source:?}");
        let _ = session.settle_script(catalog, &mut NeverFires);
        run(&mut session, catalog, "M131");
        assert_eq!(session.system(), SystemId(131), "{source:?}");
        assert!(session.landed().is_some(), "{source:?}");
        session
    }

    /// Runs `script` and settles it.
    fn run(session: &mut Session, catalog: &FakePilotCatalog, script: &str) {
        session.run_set(&SetExpr::parse(script).expect("parses"), &mut NeverFires);
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// The system holding 150.
    fn holding_150(session: &Session) -> Option<SystemId> {
        session.galaxy().holder(
            StellarId(150),
            session.statics.listers(StellarId(150)),
            session.pilot().system(),
        )
    }

    #[test]
    fn a_landed_move_leaves_the_new_systems_stellar_unowned_until_the_take_off() {
        let catalog = listed_by_125_and_131();
        for source in RuleSource::ALL {
            let mut session = moved_while_landed(&catalog, source);
            assert_eq!(
                holding_150(&session),
                by(source, None, Some(SystemId(131))),
                "{source:?}"
            );
            run(&mut session, &catalog, "S140");
            assert_eq!(
                held_systems(&session),
                by(
                    source,
                    (Some(SystemId(125)), Some(SystemId(125))),
                    (Some(SystemId(126)), Some(SystemId(131)))
                ),
                "{source:?}: 126 stands in for 131"
            );
            assert_eq!(
                session.mission_info_map_marks(Some(0)).green,
                by(source, None, Some(SystemId(131))),
                "{source:?}: the raw owner (@0x9e434)"
            );
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(holding_150(&session), Some(SystemId(131)), "{source:?}");
            assert!(
                session.stellars_shown().any(|id| id == StellarId(150)),
                "{source:?}"
            );
        }
    }

    #[test]
    fn a_take_off_after_a_landed_move_resolves_its_persons_through_the_fallback() {
        let catalog = with_person_in(listed_by_125_and_131(), 131);
        for source in RuleSource::ALL {
            let mut session = moved_while_landed(&catalog, source);
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(
                offered_return(&session),
                by(source, Some(SystemId(125)), Some(SystemId(131))),
                "{source:?}: the population before the rebuild (@0x674c2, @0x674c7)"
            );
        }
    }

    #[test]
    fn the_tribute_skips_an_unrescued_stellar_after_a_landed_move() {
        let catalog = listed_by_125_and_131();
        for source in RuleSource::ALL {
            let session = moved_while_landed(&catalog, source);
            assert_eq!(
                session.player_info(&catalog).income,
                by(source, None, Some(IncomeLine::Income(250))),
                "{source:?}: inactive and unowned (@0x5353f-0x5355d)"
            );
        }
    }

    #[test]
    fn a_land_check_move_leaves_no_wait_unless_it_settles_after_the_refresh() {
        let mut catalog = listed_by_125_and_131();
        catalog.missions.push(MissionRecord {
            on_success: Script::parse("M131"),
            ..going_to(MissionId(141), 128)
        });
        for reentry in RuleSource::ALL {
            let mut session = flown(&catalog, RuleSource::Engine, reentry);
            run(&mut session, &catalog, "S141");
            assert_eq!(land_now(&mut session), Ok(StellarId(128)), "{reentry:?}");
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert_eq!(session.system(), SystemId(131), "{reentry:?}: moved");
            assert!(session.pilot().missions().get(0).is_none(), "{reentry:?}");
            // By the engine the move settles before the `REOFFER` refresh,
            // whose rescue keeps 150 in 131 (@0xa1a92, @0xa1af2); by the
            // other reading of `story_reentry` it settles after, so the
            // owners wait for the take-off.
            assert_eq!(
                holding_150(&session),
                by(reentry, Some(SystemId(131)), None),
                "{reentry:?}"
            );
            session.take_off(&catalog, &mut NeverFires);
            assert_eq!(holding_150(&session), Some(SystemId(131)), "{reentry:?}");
        }
    }

    #[test]
    fn the_wait_is_never_set_by_the_other_reading_and_is_ended_by_reading_it() {
        let catalog = listed_by_125_and_131();
        let mut session = moved_while_landed(&catalog, RuleSource::Engine);
        assert!(session.galaxy().awaits_rebuild());
        session = session.with_rules(
            Rulebook::default().with_override(RuleKey::RescueUntilRefresh, RuleSource::Bible),
        );
        assert!(!session.galaxy().awaits_rebuild());
        assert_eq!(holding_150(&session), Some(SystemId(131)));
    }
}
