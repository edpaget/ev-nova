//! The session's part of the stellars' defence fleets (see
//! `nova_sim_ai::traffic::stellar_defence`, which the traffic owns, and
//! `nova_sim_ai::ai::defence_ship`, which flies them):
//!
//! - **The +0x47 pass** ([`Session::mark_defended_stellars`]): each frame,
//!   after the ship loop and before the stellar guns, every nav stellar
//!   of the player's system (the chart's listing) is marked under attack
//!   exactly while one of its defence ships is in the system
//!   (`_DoPlayGameWork` @0x443d4-0x4444f), which the guns' `defended`
//!   reads (see the `stellar_guns` module).
//! - **The clear** (`Session::rebuild_quietly`): every rebuild point
//!   clears every stellar's mark, docked or not, as
//!   `_HandleStellarSystemVisibility` does (@0x32d5d), so a gate exit's
//!   extra arrivals pass sends no defender up.
//! - **The refund** ([`Session::refund_defenders`]): the defence ships
//!   dropped with the system's ships give their stellars one back each,
//!   beside the aux ships' give-back (`_DeleteAllComputerShips`
//!   @0x3c02a-0x3c06f): at a populate, a dropping move and a traffic
//!   Reset, which hands the counts on to the next traffic.
//! - **The spawn** ([`Session::spawn_stellar_defender`]): one defence
//!   ship sent up at once, the planet comm dialog's Demand Tribute's
//!   (`_DoPlanetCommDialog` @0x97814,
//!   rdm:phase/play-feel-4/phase-2-planet-comm-dialog).
//!
//! A defender captured stops defending (`Session::join_fleet`, @0x9470d).
//! The counts are never saved here (phase 3 saves them). Nothing here
//! plays a sound.

use std::collections::BTreeSet;

use super::Session;
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::traffic::npc::NpcId;

impl Session {
    /// The frame's +0x47 pass (see the module docs).
    pub(super) fn mark_defended_stellars(&mut self) {
        self.space
            .traffic
            .recompute_defence_alarms(&self.chart.listed);
    }

    /// The defence ships among the system's ships but those in `kept`
    /// give their stellars one back each, as the ships are dropped (see the
    /// module docs).
    pub(super) fn refund_defenders(&mut self, kept: &BTreeSet<NpcId>) {
        self.space.traffic.refund_defenders(kept);
    }

    /// Sends up one of `stellar`'s defence ships at once, drawing on
    /// `chance`, as `_SpawnStellarDefenseShip` does (see
    /// `nova_sim_ai::traffic::Traffic::spawn_defender`): its number, or
    /// none when the player's system lists no such stellar, it has no
    /// `DefenseDude` or no slot is free.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the planet comm dialog's Demand Tribute calls it \
                      (rdm:phase/play-feel-4/phase-2-planet-comm-dialog)"
        )
    )]
    pub(crate) fn spawn_stellar_defender(
        &mut self,
        stellar: StellarId,
        chance: &mut dyn Chance,
    ) -> Option<NpcId> {
        let site = *self.chart.listed.iter().find(|site| site.id == stellar)?;
        self.step_traffic(|traffic, world| traffic.spawn_defender(&site, &world, chance))
    }
}

/// The fixture's reach into the defences (`crate::fixture::alarm_stellar`,
/// `crate::fixture::defence_ships_left`).
#[cfg(any(test, feature = "fixture"))]
impl Session {
    /// Marks `stellar` under attack.
    pub(crate) fn alarm_stellar(&mut self, stellar: StellarId) {
        self.space.traffic.defences_mut().alarm(stellar);
    }

    /// The defence ships `stellar`, one the system lists, has left.
    pub(crate) fn defence_ships_left(&self, stellar: StellarId) -> Option<i16> {
        self.chart
            .listed
            .iter()
            .find(|site| site.id == stellar)
            .map(|site| self.space.traffic.defences().left(site))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{DudeId, DudeRecord, LandingSite, ShipId};
    use crate::chance::NeverFires;
    use crate::flight::Controls;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, jump, land_now, ship, work};

    const EARTH: StellarId = StellarId(128);

    /// The test catalog with stellar 128 defended by düde 140 (ship 129),
    /// `DefCount` `def_count`.
    fn defended(def_count: i16) -> FakePilotCatalog {
        let mut base = catalog();
        for (_, sites) in &mut base.sites {
            for site in sites.iter_mut().filter(|site| site.id == EARTH) {
                *site = LandingSite {
                    defense_dude: Some(DudeId(140)),
                    def_count,
                    ..*site
                };
            }
        }
        base.dudes.push((
            DudeId(140),
            DudeRecord {
                ai_type: 1,
                govt: None,
                ships: vec![(ShipId(129), 1)],
                booty: 0,
                info_types: 0,
            },
        ));
        base.ship_records.push(ship(129, FAST));
        base
    }

    /// `catalog`'s session in flight, its system populated.
    fn flying(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        work(&mut session, catalog, Controls::default());
        session
    }

    /// The ships of `session` defending `stellar`.
    fn up(session: &Session, stellar: StellarId) -> usize {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.defends == Some(stellar))
            .count()
    }

    /// Earth, as `session` lists it.
    fn earth(session: &Session) -> LandingSite {
        *session
            .chart
            .listed
            .iter()
            .find(|site| site.id == EARTH)
            .expect("listed")
    }

    #[test]
    fn a_stellar_listed_sends_up_a_defender_at_once_and_one_unlisted_none() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let id = session.spawn_stellar_defender(EARTH, &mut NeverFires);
        assert!(
            id.is_some(),
            "its DefenseDude drawn from the system's table"
        );
        assert_eq!(up(&session, EARTH), 1);
        assert!(session.space.traffic.defences().under_attack(EARTH));
        assert_eq!(
            session.spawn_stellar_defender(StellarId(140), &mut NeverFires),
            None,
            "system 131's"
        );
    }

    #[test]
    fn the_frame_pass_keeps_a_stellar_marked_while_its_defender_is_up() {
        // A wave of 1: the respawn never tops it up past the one.
        let catalog = defended(1);
        let mut session = flying(&catalog);
        let id = session
            .spawn_stellar_defender(EARTH, &mut NeverFires)
            .expect("up");
        work(&mut session, &catalog, Controls::default());
        assert!(session.space.traffic.defences().under_attack(EARTH));
        assert_eq!(up(&session, EARTH), 1);
        // Destroyed in the ship loop: the frame's pass clears the mark, so
        // the next frame's respawn sends none up.
        session.space.traffic.remove(id);
        session.mark_defended_stellars();
        assert!(!session.space.traffic.defences().under_attack(EARTH));
        work(&mut session, &catalog, Controls::default());
        assert_eq!(up(&session, EARTH), 0);
    }

    #[test]
    fn a_jump_or_a_landing_clears_every_stellars_mark() {
        let catalog = defended(0);
        let mut session = flying(&catalog);
        session.space.traffic.defences_mut().alarm(EARTH);
        session.space.traffic.defences_mut().alarm(StellarId(400));
        jump(&mut session, &catalog, 131);
        let defences = session.space.traffic.defences();
        assert!(!defences.under_attack(EARTH));
        assert!(!defences.under_attack(StellarId(400)));
        let mut session = flying(&catalog);
        session.ship.player.position = crate::geometry::Vec2::new(30.0, -40.0);
        session.ship.player.velocity = crate::geometry::Vec2::ZERO;
        session.space.traffic.defences_mut().alarm(EARTH);
        assert_eq!(land_now(&mut session), Ok(EARTH));
        assert!(!session.space.traffic.defences().under_attack(EARTH));
    }

    #[test]
    fn a_jump_gives_back_the_defenders_left_in_space_capped_at_the_def_count() {
        let catalog = defended(2);
        let mut session = flying(&catalog);
        for _ in 0..2 {
            session.spawn_stellar_defender(EARTH, &mut NeverFires);
        }
        let site = earth(&session);
        session.space.traffic.defences_mut().set_left(&site, 1);
        jump(&mut session, &catalog, 131);
        assert_eq!(up(&session, EARTH), 0);
        assert_eq!(
            session.space.traffic.defences().left(&site),
            2,
            "1 + 2, capped at DefCount 2"
        );
    }

    #[test]
    fn a_captured_defender_defends_no_stellar() {
        let catalog = defended(3);
        let mut session = flying(&catalog);
        let id = session
            .spawn_stellar_defender(EARTH, &mut NeverFires)
            .expect("up");
        session.join_fleet(id, &mut NeverFires);
        assert_eq!(session.space.npc(id).map(|npc| npc.defends), Some(None));
    }
}
