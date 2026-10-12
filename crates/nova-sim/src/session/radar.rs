//! What the radar asks of the session (see [`iff`](crate::iff)): the
//! player's radar outfits, each ship's and stellar's IFF colour code, and
//! the interference in the player's system.

use super::Session;
use crate::catalog::{LandingSite, StellarId};
use crate::iff::{self, RadarGear, ShipIff, Standing, StellarIff};
use crate::market;
use crate::traffic::npc::Npc;

impl Session {
    /// What the player's outfits give the radar: IFF, a density scanner
    /// and anti-interference ([`RadarGear::of`]).
    ///
    /// It is read afresh at every call. The original caches it and
    /// refreshes it only where `_ResetPlayerPrecalcedValues` (@0xc357)
    /// runs, never from the outfitter, so an outfit bought there shows on
    /// its radar only from the next refresh point:
    /// rdm:phase/hud-fidelity/phase-10-radar-outfit-cache-refresh.
    #[must_use]
    pub fn radar_gear(&self) -> RadarGear {
        RadarGear::of(&self.pilot.outfits, &self.outfits)
    }

    /// What IFF shows `npc` as ([`iff::ship_iff`]).
    #[must_use]
    pub fn ship_iff(&self, npc: &Npc) -> ShipIff {
        iff::ship_iff(npc)
    }

    /// What IFF shows `stellar` as, one of the stellars shown, as its
    /// destroyed state stands: by the player's legal record that its
    /// `MinStatus` is checked against (as the
    /// [`LegalRecords`](crate::RuleKey::LegalRecords) rule reads it, by the
    /// engine the record in the current system, `_playerRecord[stellar
    /// +0x14]`), and its [`Standing`]. `None` for a stellar not shown.
    #[must_use]
    pub fn stellar_iff(&self, stellar: StellarId) -> Option<StellarIff> {
        let site = self.sites.iter().find(|site| site.id == stellar)?;
        Some(iff::stellar_iff(
            site,
            self.landing_record(site),
            Self::standing(site),
        ))
    }

    /// What the player stands with `site` beyond its record.
    ///
    /// - **Dominated:** only the always-dominated bit
    ///   ([`DOMINATED`](market::DOMINATED)), as [`Session::markup`] reads
    ///   it. Domination by tribute (`_DoPlanetCommDialog` @0x978fa, cleared
    ///   @0x9740f) has no state yet: rdm:task/planet-domination feeds it in
    ///   here.
    /// - **Blessed:** never. The port has no ranks yet:
    ///   rdm:phase/crons-news-ranks/phase-12-rank-blessing feeds the
    ///   government's "may land" blessing (`gövt` +0x85) in here.
    fn standing(site: &LandingSite) -> Standing {
        Standing {
            dominated: site.flags2 & market::DOMINATED != 0,
            blessed: false,
        }
    }

    /// The interference in the player's system ([`iff::interference`]):
    /// its `sÿst` `Interference` less the player's anti-interference,
    /// 0..=100.
    #[must_use]
    pub fn interference(&self) -> u8 {
        let system = self.galaxy.map().interference(self.pilot.system);
        iff::interference(system, self.radar_gear().anti_interference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, StarSystem, SystemId};
    use crate::iff::{MOD_ANTI_INTERFERENCE, MOD_DENSITY_SCANNER, MOD_IFF};
    use crate::landing::StellarFlags;
    use crate::pilot::Pilot;
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::stats::ShipStats;
    use crate::testkit::{FakePilotCatalog, catalog, npc, outfit, planet};

    /// IFF, a scanner, anti-interference 30 and −20, as outfits 300-303;
    /// system 130's `Interference` 60. Of system 130's stellars, 128 has
    /// `MinStatus` 10, and 129 is always dominated with `MinStatus` 10.
    fn radar_catalog() -> FakePilotCatalog {
        let base = catalog();
        FakePilotCatalog {
            outfits: vec![
                outfit(300, &[(MOD_IFF, 0)]),
                outfit(301, &[(MOD_DENSITY_SCANNER, 0)]),
                outfit(302, &[(MOD_ANTI_INTERFERENCE, 30)]),
                outfit(303, &[(MOD_ANTI_INTERFERENCE, -20)]),
            ],
            star_map: base
                .star_map
                .iter()
                .map(|system| StarSystem {
                    interference: if system.id == SystemId(130) { 60 } else { 0 },
                    ..system.clone()
                })
                .collect(),
            sites: vec![(
                SystemId(130),
                vec![
                    LandingSite {
                        min_status: 10,
                        ..planet(128, 30.0, -40.0)
                    },
                    LandingSite {
                        min_status: 10,
                        flags2: market::DOMINATED,
                        ..planet(129, 2000.0, 0.0)
                    },
                ],
            )],
            ..base
        }
    }

    fn session(catalog: &FakePilotCatalog) -> Session {
        Session::fly(catalog, Pilot::new(catalog, "Ada").expect("starts")).expect("flies")
    }

    fn own(session: &mut Session, outfit: i16, count: u16) {
        session.pilot.outfits.insert(OutfitId(outfit), count);
    }

    #[test]
    fn the_radar_gear_is_the_players_outfits() {
        let mut session = session(&radar_catalog());
        assert_eq!(session.radar_gear(), RadarGear::default());
        own(&mut session, 300, 1);
        own(&mut session, 301, 1);
        own(&mut session, 302, 2);
        assert_eq!(
            session.radar_gear(),
            RadarGear {
                iff: true,
                density_scanner: true,
                anti_interference: 60,
            }
        );
    }

    #[test]
    fn a_ships_iff_is_its_colour_code() {
        let session = session(&radar_catalog());
        assert_eq!(
            session.ship_iff(&npc(1, ShipStats::default())),
            ShipIff::Other
        );
    }

    #[test]
    fn a_stellar_without_the_dominated_bit_falls_through_to_its_legal_colour() {
        let mut session = session(&radar_catalog());
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Denied)
        );
        session.pilot.set_system_record(SystemId(130), 10);
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Cleared)
        );
        session.pilot.set_system_record(SystemId(130), -1);
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Hostile)
        );
    }

    #[test]
    fn an_always_dominated_stellar_is_dominated() {
        let mut session = session(&radar_catalog());
        session.pilot.set_system_record(SystemId(130), -1);
        assert_eq!(
            session.stellar_iff(StellarId(129)),
            Some(StellarIff::Dominated)
        );
    }

    #[test]
    fn a_stellar_not_shown_has_no_colour() {
        let session = session(&radar_catalog());
        assert_eq!(session.stellar_iff(StellarId(140)), None);
    }

    #[test]
    fn the_record_follows_the_legal_records_rule() {
        // By the Bible, the record with the stellar's government: 128 is
        // independent in an independent system, so its record is 0.
        let rules = Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible);
        let mut session = session(&radar_catalog()).with_rules(rules);
        session.pilot.set_system_record(SystemId(130), 50);
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Denied)
        );
    }

    #[test]
    fn a_destroyed_stellar_is_unlandable() {
        let mut session = session(&radar_catalog());
        session.pilot.set_system_record(SystemId(130), 50);
        session.sites[0].destroyed = true;
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Unlandable)
        );
        session.sites[0].destroyed = false;
        session.sites[0].flags |= StellarFlags::UNINHABITED;
        assert_eq!(
            session.stellar_iff(StellarId(128)),
            Some(StellarIff::Unlandable)
        );
    }

    #[test]
    fn interference_is_the_systems_less_the_anti_interference() {
        let mut session = session(&radar_catalog());
        assert_eq!(session.interference(), 60);
        own(&mut session, 302, 1);
        assert_eq!(session.interference(), 30);
        own(&mut session, 302, 3);
        assert_eq!(session.interference(), 0, "never below 0");
        own(&mut session, 302, 0);
        own(&mut session, 303, 1);
        assert_eq!(session.interference(), 80, "raised by a negative sum");
    }

    #[test]
    fn interference_is_the_players_systems() {
        let mut catalog = radar_catalog();
        catalog.star_map[0].interference = 0;
        let session = session(&catalog);
        assert_eq!(session.interference(), 0);
    }
}
