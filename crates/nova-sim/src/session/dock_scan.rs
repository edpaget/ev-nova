//! The new game's attention from the law: traffic control scans the ship
//! as the player asks to land (see [`dock_scan`](crate::dock_scan) for the
//! rule), and the player's exit points follow the kits it carries.
//!
//! **The scan.** The land press stops at traffic control's request
//! ([`dock_request`](crate::landing::dock_request) gives
//! [`DockStage::Asks`](crate::landing::DockStage::Asks)), and
//! [`Session::answer_dock_ask`] answers it: when the press carries a
//! [`LegalCode`] to convict by, [`Session::dock_scan`] runs before the
//! clearance, which then reads the record the scan leaves. It runs only
//! for a ship neither cloaked nor disabled (the original refuses those
//! first: `_ShipIsCloaked` @0x66a8e, `_IsDisabled` @0x66aee), read once
//! for the scan and the refusal both (`PlayerShip::landing_bar`), at a
//! stellar that is no hypergate or wormhole, governed by
//! a government that keeps the law, for a ship whose kits draw any
//! attention. A request already granted for the stellar never reaches the
//! scan, since the press lands or fails on it before traffic control is
//! asked. One granted at another stellar spares no scan: the find is
//! applied, though the clearance that follows still grants on it, as the
//! original's does whichever stellar the request is for
//! (`Standing::granted`, @0x67934-0x6793d). A find is applied as an
//! interceptor's is ([`Session::apply_find`]: convicted of smuggling
//! against the stellar's government, ranks lost, fined,
//! [`SimSound::Smuggling`](crate::sound::SimSound::Smuggling), the latch
//! cleared) but sets no scanner on the player, and is said as
//! [`SimMessage::DockScanned`]. The latch, armed again only by the
//! precalced reset (a landing, a jump, a load), keeps a second press from
//! fining again.
//!
//! `FlightCommand::Land` presses with its collaborators' law
//! ([`Session::press_land_key`]). The bare [`Session::land`] carries none
//! and scans nothing.
//!
//! **The exits.** The player's refit lays its class's own hardpoints and
//! those of the kits it carries over its class's exit points
//! ([`Statics::fitted_hull`], `nova_sim::combat::spots`), so a kit bought
//! or granted adds its mount at once, and a load restores it. Stock data
//! has no fittings, so its exits are its `shän`s'.
//!
//! The original has neither: both are the new game's.

use std::collections::BTreeMap;

use super::{Session, Statics};
use crate::catalog::{LandingSite, OutfitId, ShipId};
use crate::combat::hull::HullSpec;
use crate::landing::{DockAsk, DockPress, LandingRefusal};
use crate::legal::LegalCode;
use crate::message::{DockFind, SimMessage};
use crate::rulebook::RuleKey;
use crate::scan::{self, Contraband};
use nova_sim_catalog::fittings::Spot;

impl Statics {
    /// The hardpoints a ship of class `ship` carrying `carried` fires
    /// from: its class's own, then each kit's it carries, by ascending
    /// `oütf`.
    pub(super) fn spots(&self, ship: ShipId, carried: &BTreeMap<OutfitId, u16>) -> Vec<Spot> {
        let class = self.fittings.hulls.get(&ship).into_iter().flatten();
        let kits = carried
            .iter()
            .filter(|&(_, &count)| count > 0)
            .filter_map(|(outfit, _)| self.fittings.kits.get(outfit))
            .flat_map(|kit| &kit.spots);
        class.chain(kits).copied().collect()
    }

    /// `hull`, of class `ship`, flying from the hardpoints of its class
    /// and the kits in `carried` ([`HullSpec::with_spots`]).
    pub(super) fn fitted_hull(
        &self,
        hull: &HullSpec,
        ship: ShipId,
        carried: &BTreeMap<OutfitId, u16>,
    ) -> HullSpec {
        let raw = self.arsenal.class_exits(ship);
        hull.with_spots(raw.as_ref(), &self.spots(ship, carried))
    }
}

impl Session {
    /// Traffic control's answer to `ask`, the land press's request: the
    /// dock scan first ([`Session::dock_scan`]), convicting by `law`,
    /// when the press carries one and nothing bars the ship itself from
    /// landing (`barred`, `PlayerShip::landing_bar`: the original refuses
    /// a cloaked or disabled ship before it asks); then the clearance,
    /// which reads the record the scan leaves.
    pub(super) fn answer_dock_ask(
        &mut self,
        ask: DockAsk,
        law: Option<&dyn LegalCode>,
        barred: Option<LandingRefusal>,
    ) -> DockPress {
        if let Some(law) = law
            && barred.is_none()
        {
            self.dock_scan(ask.site(), law);
        }
        let blessing = self.blessing();
        let reading = self.setup.rules.source_for(RuleKey::BlessingLanding);
        let site = ask.site();
        ask.answer(crate::landing::cleared(
            site,
            self.standing(site, &blessing),
            reading,
        ))
    }

    /// Traffic control's scan of the player as it asks `site` for
    /// clearance, convicting by `law` (see the module docs): nothing at a
    /// hypergate or wormhole, a stellar of no government or one that keeps
    /// no law, or for a ship drawing no attention; else the contraband
    /// scan, a find applied and said.
    ///
    /// It needs no check for a request already granted: traffic control
    /// is asked only when the player holds no request for `site`
    /// ([`dock_request`](crate::landing::dock_request) ends a press at a
    /// stellar with its own request before it asks), and a request held
    /// for another stellar spares no scan.
    pub(super) fn dock_scan(&mut self, site: &LandingSite, law: &dyn LegalCode) {
        if self.gate_kind(site.id).is_some() {
            return;
        }
        let Some(govt_id) = site.govt else {
            return;
        };
        let Some(govt) = self.statics.govts.get(govt_id).cloned() else {
            return;
        };
        let outfits = self.ship.counted_outfits(&self.pilot);
        let attention = crate::dock_scan::attention(&outfits, &self.statics.fittings.kits);
        if !crate::dock_scan::scans(&govt, attention) {
            return;
        }
        let carried = scan::Carried {
            cargo: &self.pilot.cargo,
            junk: self.statics.goods.junk(),
            outfits: &outfits,
            outfit_records: &self.statics.outfits,
        };
        let rules = scan::ScanRules {
            outfits_past_junk: self.setup.rules.source_for(RuleKey::ScanOutfitsPastJunk),
            outfit_fine: self.setup.rules.source_for(RuleKey::OutfitScanFine),
        };
        let found = scan::contraband(
            &govt,
            &carried,
            self.pilot.cash,
            self.ship.latches.scan_latches,
            rules,
        );
        drop(outfits);
        let Some(found) = found else {
            return;
        };
        self.apply_find(govt_id, &found, law);
        let (found, fine) = match found {
            Contraband::Junk { id, fine } => (DockFind::Junk(id), fine),
            Contraband::Outfit { id, one, fine } => (DockFind::Outfit { id, one }, fine),
        };
        self.space.say(
            &mut self.out,
            SimMessage::DockScanned {
                stellar: site.id,
                found,
                fine,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{GovtId, GovtRecord, OutfitRecord, Penalties, StellarId};
    use crate::clearance::{CLEARED, DockRequest};
    use crate::combat::hull::Condition;
    use crate::flight_frame::{Answer, FlightCommand};
    use crate::landing::{LandOutcome, LandingRefusal, StellarFlags};
    use crate::session::LandPress;
    use crate::session::apply::ApplyCollaborators;
    use crate::sound::SimSound;
    use crate::testkit::{FakePilotCatalog, catalog, govt, outfit};
    use nova_sim_catalog::fittings::{Fittings, Kit, SpotKind};

    const LAW: GovtId = GovtId(128);
    const PLANET: StellarId = StellarId(128);
    /// A legal kit: attention 1, no `ScanMask`.
    const PLATING: OutfitId = OutfitId(130);
    /// A lawless kit: attention 2, the law's `ScanMask`.
    const GUN_MOUNT: OutfitId = OutfitId(131);
    /// A lawless outfit that is no kit: the law's `ScanMask`, no attention.
    const LOOSE: OutfitId = OutfitId(132);
    const MASK: u16 = 0x0001;
    const FINE: i16 = 500;
    const PENALTY: i16 = 10;

    /// The test catalog with planet 128, under the ship, governed by a
    /// law-keeping government whose systems these are, and the two kits
    /// and the loose outfit.
    fn policed() -> FakePilotCatalog {
        let mut catalog = catalog();
        catalog.govts = vec![GovtRecord {
            penalties: Penalties {
                smuggle: PENALTY,
                ..Penalties::default()
            },
            scan_mask: MASK,
            scan_fine: FINE,
            news_pic: -1,
            ..govt(128)
        }];
        catalog.sites[0].1[0].govt = Some(LAW);
        for system in &mut catalog.star_map {
            system.govt = Some(LAW);
        }
        catalog.outfits.extend([
            outfit(130, &[]),
            OutfitRecord {
                scan_mask: MASK,
                ..outfit(131, &[])
            },
            OutfitRecord {
                scan_mask: MASK,
                ..outfit(132, &[])
            },
        ]);
        let kit = |attention, spots| Kit { attention, spots };
        catalog.fittings = Fittings {
            hulls: BTreeMap::new(),
            kits: BTreeMap::from([
                (PLATING, kit(1, Vec::new())),
                (
                    GUN_MOUNT,
                    kit(
                        2,
                        vec![Spot {
                            kind: SpotKind::Turret,
                            at: (0, -2),
                        }],
                    ),
                ),
            ]),
        };
        catalog
    }

    /// A session of `catalog` carrying `outfits`, with 2000 credits, its
    /// sounds, messages and save taken.
    fn carrying(catalog: &FakePilotCatalog, outfits: &[OutfitId]) -> Session {
        let mut session = Session::start(catalog).expect("starts");
        for &outfit in outfits {
            session.pilot.outfits.insert(outfit, 1);
        }
        session.pilot.cash = 2000;
        session.take_sounds();
        session.take_messages();
        session.take_save_due();
        session
    }

    /// The land key pressed as `FlightCommand::Land` does, with Nova's
    /// law.
    fn press(session: &mut Session) -> Answer {
        session.apply(
            FlightCommand::Land,
            &mut ApplyCollaborators {
                catalog: None,
                chance: &mut crate::chance::NeverFires,
                law: &crate::legal::NovaLaw::default(),
                boarding_rule: &crate::board::NovaBoarding::default(),
                hail_options: &crate::hail::HailOptions::default(),
            },
        )
    }

    fn requested() -> Answer {
        Answer::LandPress(LandPress::from(LandOutcome::Requested {
            stellar: PLANET,
            station: false,
        }))
    }

    fn denied(min_status: i16) -> Answer {
        Answer::LandRefused {
            refusal: LandingRefusal::Denied {
                stellar: PLANET,
                station: false,
                min_status,
            },
            gate: None,
            requested: true,
        }
    }

    fn scanned(fine: i64) -> SimMessage {
        SimMessage::DockScanned {
            stellar: PLANET,
            found: DockFind::Outfit {
                id: GUN_MOUNT,
                one: true,
            },
            fine,
        }
    }

    #[test]
    fn a_lawless_kit_at_a_law_keeping_stellar_is_found_fined_convicted_and_denied() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        let record = session.pilot().legal_record(LAW);
        assert_eq!(press(&mut session), denied(0));
        assert_eq!(session.pilot().cash(), 2000 - i64::from(FINE));
        assert_eq!(
            session.pilot().legal_record(LAW),
            record - PENALTY,
            "convicted of smuggling"
        );
        assert!(session.pilot().system_record(session.system()) < 0);
        let sounds = session.take_sounds();
        assert!(sounds.contains(&SimSound::Smuggling), "{sounds:?}");
        assert_eq!(session.take_messages(), [scanned(i64::from(FINE))]);
        assert!(session.take_save_due());
        assert_eq!(session.dock_request(), None);
    }

    #[test]
    fn a_second_press_fines_nothing_more_until_a_reset() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        press(&mut session);
        session.take_messages();
        let cash = session.pilot().cash();
        let record = session.pilot().legal_record(LAW);
        assert_eq!(press(&mut session), denied(0), "the record still bars it");
        assert_eq!(session.pilot().cash(), cash);
        assert_eq!(session.pilot().legal_record(LAW), record);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn a_legal_kit_is_scanned_found_clean_and_cleared() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[PLATING]);
        assert_eq!(press(&mut session), requested());
        assert_eq!(session.pilot().cash(), 2000);
        assert!(!session.take_sounds().contains(&SimSound::Smuggling));
    }

    #[test]
    fn a_ship_drawing_no_attention_is_never_scanned() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[LOOSE]);
        assert_eq!(press(&mut session), requested(), "contraband, unnoticed");
        assert_eq!(session.pilot().cash(), 2000);
    }

    #[test]
    fn a_stellar_that_keeps_no_law_or_has_no_government_never_scans() {
        let mut lawless = policed();
        lawless.govts[0].penalties.smuggle = 0;
        let mut session = carrying(&lawless, &[GUN_MOUNT]);
        assert_eq!(press(&mut session), requested());
        assert_eq!(session.pilot().cash(), 2000);
        let mut ungoverned = policed();
        ungoverned.sites[0].1[0].govt = None;
        let mut session = carrying(&ungoverned, &[GUN_MOUNT]);
        assert_eq!(press(&mut session), requested());
        assert_eq!(session.pilot().cash(), 2000);
    }

    #[test]
    fn a_hypergate_or_an_uninhabited_stellar_never_scans() {
        let mut gated = policed();
        gated.sites[0].1[0].flags2 = crate::landing::HYPERGATE;
        let mut session = carrying(&gated, &[GUN_MOUNT]);
        press(&mut session);
        assert_eq!(session.pilot().cash(), 2000, "a hypergate");
        let mut empty = policed();
        empty.sites[0].1[0].flags |= StellarFlags::UNINHABITED;
        let mut session = carrying(&empty, &[GUN_MOUNT]);
        press(&mut session);
        assert_eq!(session.pilot().cash(), 2000, "uninhabited");
    }

    #[test]
    fn a_request_granted_at_another_stellar_does_not_spare_the_scan() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        session.port.dock = Some(DockRequest {
            stellar: StellarId(129),
            permission: CLEARED,
        });
        let record = session.pilot().legal_record(LAW);
        assert_eq!(
            press(&mut session),
            requested(),
            "the original's clearance grants on any stellar's cleared request"
        );
        assert_eq!(session.pilot().cash(), 2000 - i64::from(FINE));
        assert_eq!(session.pilot().legal_record(LAW), record - PENALTY);
        assert_eq!(session.take_messages(), [scanned(i64::from(FINE))]);
    }

    #[test]
    fn a_request_granted_here_lands_without_a_second_scan() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        session.port.dock = Some(DockRequest {
            stellar: PLANET,
            permission: CLEARED,
        });
        assert_eq!(
            press(&mut session),
            Answer::LandPress(LandPress::from(LandOutcome::Landed(PLANET)))
        );
        assert_eq!(session.pilot().cash(), 2000);
        assert_eq!(session.take_messages(), []);
    }

    #[test]
    fn the_bare_land_press_never_scans() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        assert!(session.land().is_ok());
        assert_eq!(session.pilot().cash(), 2000);
    }

    #[test]
    fn a_cloaked_or_disabled_ship_is_refused_before_any_scan() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        session.ship.cloak.cloak = crate::cloak::Cloak {
            fade: crate::cloak::FADE_FULL,
            direction: 0,
        };
        assert!(matches!(
            press(&mut session),
            Answer::LandRefused {
                refusal: LandingRefusal::Cloaked,
                ..
            }
        ));
        assert_eq!(session.pilot().cash(), 2000, "cloaked");
        let mut session = carrying(&catalog, &[GUN_MOUNT]);
        session.ship.condition = Condition::Disabled;
        assert!(matches!(
            press(&mut session),
            Answer::LandRefused {
                refusal: LandingRefusal::Disabled,
                ..
            }
        ));
        assert_eq!(session.pilot().cash(), 2000, "disabled");
    }

    #[test]
    fn a_kit_granted_moves_the_turret_exits_and_taken_back_restores_them() {
        let catalog = policed();
        let mut session = carrying(&catalog, &[]);
        let class = session.ship.fit.hull;
        session.run_set(
            &crate::SetExpr::parse("G131").expect("parses"),
            &mut crate::chance::NeverFires,
        );
        let spot = Spot {
            kind: SpotKind::Turret,
            at: (0, -2),
        };
        let raw = session.statics.arsenal.class_exits(ShipId(128));
        assert_eq!(
            session.ship.fit.hull,
            class.with_spots(raw.as_ref(), &[spot]),
            "the gun bed's"
        );
        assert_ne!(session.ship.fit.hull.exits, class.exits);
        session.run_set(
            &crate::SetExpr::parse("D131").expect("parses"),
            &mut crate::chance::NeverFires,
        );
        assert_eq!(session.ship.fit.hull, class, "the class's own");
    }

    #[test]
    fn a_turret_given_with_a_kit_fires_at_the_target_from_the_kit_s_bed() {
        use crate::catalog::{WeaponId, WeaponRecord};
        use crate::combat::ShipRef;
        use crate::combat::armament::MOD_WEAPON;
        use crate::flight::Controls;
        use crate::testkit::work;
        use nova_sim_catalog::fixture::weapon;

        let driver = WeaponRecord {
            reload: 15,
            count: 60,
            speed: 1000,
            mass_dmg: 12,
            energy_dmg: 12,
            guidance: 4,
            exit_type: 1,
            ..weapon(133)
        };
        let mut catalog = super::super::tests::arming(driver, super::super::tests::blaster(), 0);
        catalog.hulls[0].weapons.clear();
        catalog.outfits = vec![
            OutfitRecord {
                flags: 0x0002,
                ..outfit(129, &[(MOD_WEAPON, 133)])
            },
            outfit(131, &[]),
        ];
        catalog.fittings = policed().fittings;
        let mut session = super::super::tests::with_an_npc_at(&catalog, 750, 850, 0);
        session.run_set(
            &crate::SetExpr::parse("G129 G131").expect("parses"),
            &mut crate::chance::NeverFires,
        );
        session.select_target(crate::TargetPick::Nearest);
        session.hold_fire(true, false);
        let centre = session.player().position;
        work(&mut session, &catalog, Controls::default());
        let shot = session
            .shots()
            .iter()
            .find(|shot| shot.firer == ShipRef::Player && shot.weapon.id == WeaponId(133))
            .expect("the turret fired");
        let from = shot.moved_from.unwrap_or(shot.position) - centre;
        assert!((from.length() - 2.0).abs() < 1e-3, "the gun bed: {from:?}");
    }

    #[test]
    fn a_class_s_own_spots_come_before_its_kits() {
        let mut catalog = policed();
        let nose = Spot {
            kind: SpotKind::Gun,
            at: (0, 8),
        };
        catalog.fittings.hulls.insert(ShipId(128), vec![nose]);
        let session = carrying(&catalog, &[GUN_MOUNT, PLATING]);
        assert_eq!(
            session.statics.spots(ShipId(128), &session.pilot.outfits),
            [
                nose,
                Spot {
                    kind: SpotKind::Turret,
                    at: (0, -2),
                }
            ]
        );
        let sold = BTreeMap::from([(GUN_MOUNT, 0)]);
        assert_eq!(session.statics.spots(ShipId(128), &sold), [nose]);
    }
}
