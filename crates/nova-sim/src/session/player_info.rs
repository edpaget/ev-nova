//! The session's side of the player info screen (see
//! [`player_info`](crate::player_info)): the facts it is decided from,
//! gathered from the pilot, the ship's stats, the records, the galaxy and
//! the string lists.
//!
//! A stellar counts as active, and its tribute as paid, when its owner
//! exists and is visible (`_HandleStellarSystemVisibility`
//! @0x32cbe-0x32cdf, the tribute's test @0x53560). Here that is any
//! stellar with a holder
//! ([`OverlaidGalaxy::holder`](crate::story::OverlaidGalaxy::holder)): an
//! owner is visible by definition, and the rescue's holder is the
//! player's system, which the galaxy refresh always keeps visible (see
//! [`story`](crate::story)), so the original's tests always hold. The
//! player's system's own stellars all have one.
//!
//! The legal status scans the player's system's `NavDefs` as the
//! catalog gives them, through the galaxy overlay; a `NavDefs` slot the
//! catalog cannot read is skipped there, so it does not take up one of
//! the first three or four places the original's scans count. The
//! tribute counts each stellar some system lists, held as
//! [`OverlaidGalaxy::holder`](crate::story::OverlaidGalaxy::holder) gives
//! it, when the stellar is dominated ([`DOMINATED`]); the tribute itself
//! is read from the holder's stellars in `catalog`.

use super::{Session, overlaid_sites};
use crate::catalog::{GovtId, LandingSite, PilotCatalog};
use crate::landing::{StellarFlags, is_landable};
use crate::market::{DOMINATED, Good};
use crate::news::{CARGO_NAMES, MISC_STRINGS};
use crate::outfitter::outfit_mods;
use crate::player_info::{
    self, COMMODITIES, Domination, HeldCargo, LEGAL_STATUS_STRINGS, MissionLoad, PlayerInfo, Words,
};
use crate::rulebook::RuleKey;
use crate::shipyard;
use crate::stats::HandlingTotals;

/// Government slot 0, whose `CrimeTol` an independent system's legal
/// status reads (`_DrawLegalStatusString` @0xd49f-0xd4a7).
const FIRST_GOVT: GovtId = GovtId(128);

/// The `Flags2` bits of a hypergate (0x1000) and a wormhole (0x2000),
/// either of which makes a stellar a gate.
const GATES: u16 = 0x3000;

/// Whether `site` is uninhabited.
fn uninhabited(site: &LandingSite) -> bool {
    site.flags & StellarFlags::UNINHABITED != 0
}

/// Whether `site` is a hypergate or wormhole.
fn gate(site: &LandingSite) -> bool {
    site.flags2 & GATES != 0
}

impl Session {
    /// What the player info screen shows (see
    /// [`player_info`](crate::player_info)), the stellars the legal
    /// status and the tribute read taken from `catalog`.
    #[must_use]
    pub fn player_info(&self, catalog: &impl PilotCatalog) -> PlayerInfo {
        let words = self.player_info_words();
        let cargo_names = self.strings().string_list(CARGO_NAMES);
        let cargo = self.held_cargo(&cargo_names);
        let reserves = self.reserves();
        let totals = HandlingTotals::new(
            self.fields,
            &outfit_mods(&self.pilot.outfits, &self.outfits),
        );
        let records = &self.outfits[..];
        let owned = &self.pilot.outfits;
        let ranks = self.pilot_info_ranks();
        let any_rank = self
            .pilot
            .ranks
            .iter()
            .any(|id| self.ranks.iter().any(|rank| rank.id == *id));
        let honors = player_info::honors_shown(any_rank, records, owned).then(|| {
            let specials = player_info::special_groups(records, owned);
            player_info::honors_text(&ranks, &specials, records, &words).unwrap_or_default()
        });
        let extras = player_info::extras_shown(records, owned).then(|| {
            player_info::extras_text(
                records,
                owned,
                &words,
                self.trade_in_shown(),
                self.rules.source_for(RuleKey::PayMillions),
            )
        });
        let costs: Vec<i32> = self
            .pilot
            .escorts
            .iter()
            .filter(|escort| escort.hired())
            .filter_map(|escort| self.ship_record(escort.ship).map(|record| record.cost))
            .collect();
        let day = player_info::daily_income(costs, self.tribute(catalog), ranks.salary);
        PlayerInfo {
            pilot_name: self.pilot.name().to_owned(),
            date: self.med_date_text(),
            system: self
                .place_names
                .systems
                .get(&self.pilot.system)
                .cloned()
                .unwrap_or_default(),
            legal_status: self.legal_status(catalog, &words),
            ship_name: self.pilot.ship_name().unwrap_or_default().to_owned(),
            ship_class: self
                .ship_name(self.pilot.ship)
                .unwrap_or_default()
                .to_owned(),
            turn_rate: player_info::turn_shown(&totals),
            accel: player_info::accel_shown(&totals),
            max_speed: player_info::speed_shown(&totals),
            credits: self.pilot.cash,
            shield: player_info::reserve_shown(reserves.shield.now, self.stats.shield),
            armor: player_info::reserve_shown(reserves.armor.now, self.stats.armor),
            energy: player_info::energy_shown(reserves.fuel.now, self.stats.fuel as i64),
            income: player_info::income_line(day),
            fleet: cargo.fleet,
            cargo: player_info::has_any_cargo(&cargo)
                .then(|| player_info::cargo_text(&cargo, &words)),
            extras,
            honors,
            jettisonable: player_info::has_jettisonable_cargo(&cargo),
            words,
        }
    }

    /// The `STR#` 2002 strings the player info shows, from the session's
    /// string lists; a missing one reads empty.
    fn player_info_words(&self) -> Words {
        Words::from_list(&self.strings().string_list(MISC_STRINGS))
    }

    /// The cargo the Cargo text lists, its types named by `cargo_names`.
    fn held_cargo<'a>(&'a self, cargo_names: &'a [String]) -> HeldCargo<'a> {
        let mut commodities = [0; COMMODITIES];
        let mut junk = Vec::new();
        for (good, tons) in self.pilot.cargo() {
            match good {
                Good::Commodity(n) => {
                    if let Some(held) = commodities.get_mut(usize::from(n)) {
                        *held = i64::from(tons);
                    }
                }
                Good::Junk(id) => {
                    junk.push((
                        id,
                        self.junk_lc_name(id).unwrap_or_default(),
                        i64::from(tons),
                    ));
                }
            }
        }
        junk.sort_by_key(|&(id, _, _)| id);
        let missions = self
            .pilot
            .missions()
            .iter()
            .filter_map(|(_, active)| {
                let cargo_type = active.cargo_type.filter(|_| active.aboard)?;
                (active.cargo_tons >= 0).then(|| MissionLoad {
                    cargo_type,
                    tons: active.cargo_tons,
                    can_abort: self
                        .mission_record(active.mission)
                        .is_some_and(|record| record.can_abort != 0),
                })
            })
            .collect();
        let holds = i64::from(self.capacity());
        HeldCargo {
            commodities,
            missions,
            junk: junk
                .into_iter()
                .map(|(_, name, count)| (name, count))
                .collect(),
            cargo_names,
            fleet: holds > i64::from(self.stats.capacity),
            free: holds - self.pilot.total_cargo(),
            mission_room: self.mission_cargo_space(),
        }
    }

    /// What the ship flown trades in for, unfluxed, as
    /// `_PlayerShipTradeInPrice` (@0xb079-0xb144) values it for the
    /// Extras text ([`shipyard::trade_in`]).
    fn trade_in_shown(&self) -> i64 {
        shipyard::trade_in(
            self.ship_record(self.pilot.ship)
                .map_or(0, |record| record.cost),
            self.fields.mass,
            &self.pilot.outfits,
            &self.outfits,
            self.rules.source_for(RuleKey::TradeInOutfits),
            &self.rules,
        )
    }

    /// The day's tribute (@0x5352c-0x53583): each stellar some system
    /// lists that is dominated and active, its tribute read from its
    /// holder's stellars in `catalog`.
    fn tribute(&self, catalog: &impl PilotCatalog) -> i64 {
        self.listed_stellars
            .iter()
            .filter(|listed| listed.flags2 & DOMINATED != 0)
            .filter_map(|listed| {
                let holder = self.galaxy.holder(&listed.systems, self.pilot.system)?;
                let sites = catalog.landing_sites(holder);
                let site = sites.iter().find(|site| site.id == listed.id)?;
                Some(player_info::tribute_of(site))
            })
            .sum()
    }

    /// The Legal Status row (`_DrawLegalStatusString` @0xd43f,
    /// `_SystemIsInhabited` 0x4c4a): the player's system's stellars read
    /// from `catalog`; "N/A" in an uninhabited system or under a
    /// xenophobic government; a status missing from the string lists
    /// reads empty.
    fn legal_status(&self, catalog: &impl PilotCatalog, words: &Words) -> String {
        let system = self.pilot.system;
        let sites = overlaid_sites(&self.galaxy, catalog, system);
        let inhabited = player_info::system_is_inhabited(
            sites.iter().map(|site| (uninhabited(site), gate(site))),
        );
        if !inhabited {
            return words.not_available.to_string();
        }
        let govt = self.galaxy.govt(system);
        // Each of the player's system's stellars is active: the original
        // asks (+0x44 @0xd5ba), but its holder is always visible (see the
        // module docs).
        let counting = sites
            .iter()
            .take(3)
            .filter(|site| !uninhabited(site) && is_landable(site) && !gate(site));
        let index = player_info::legal_status_index(
            i64::from(self.system_record()),
            i64::from(self.govts.crime_tol(govt.or(Some(FIRST_GOVT)))),
            self.govts.xenophobic(govt),
            Domination::of(counting.map(|site| site.flags2 & DOMINATED != 0)),
        );
        match index {
            Some(index) => self
                .nth_string(LEGAL_STATUS_STRINGS, index + 1)
                .unwrap_or_default(),
            None => words.not_available.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::SystemId;
    use crate::catalog::{
        CommCatalog, GovtRecord, JunkId, JunkRecord, MissionRecord, OutfitId, OutfitRecord, RankId,
        RankRecord, ShipId, ShipRecord,
    };
    use crate::control::Test;
    use crate::govt::XENOPHOBIC;
    use crate::outfitter::OutfitFlags;
    use crate::pilot::Escort;
    use crate::player_info::{EnergyShown, IncomeLine, Reach};
    use crate::reserves::Reserves;
    use crate::session::tests::{alternates, hide, with_alternate};
    use crate::testkit::{
        FAST, FakePilotCatalog, carrying, catalog, govt, mission, outfit, planet, rank, ship,
        starting,
    };

    /// `STR#` 2002, every string empty but #392 "und" and #396 "N/A";
    /// `STR#` 134, the
    /// legal statuses, each "status n" for its entry n; `STR#` 4001,
    /// "food" and "*passengers".
    #[derive(Debug)]
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                MISC_STRINGS => {
                    let mut strings = vec![String::new(); 400];
                    strings[391] = "und".to_owned();
                    strings[395] = "N/A".to_owned();
                    strings
                }
                LEGAL_STATUS_STRINGS => (1..=18).map(|n| format!("status {n}")).collect(),
                CARGO_NAMES => vec!["food".to_owned(), "*passengers".to_owned()],
                _ => Vec::new(),
            }
        }
    }

    #[test]
    fn the_words_are_read_from_str_2002_and_a_missing_one_is_empty() {
        let session = Session::start(&catalog())
            .expect("starts")
            .with_strings(Rc::new(Lists));
        let words = session.player_info_words();
        assert_eq!(&*words.and, "und");
        assert_eq!(&*words.ton, "");
        assert_eq!(
            Session::start(&catalog())
                .expect("starts")
                .player_info_words(),
            Words::from_list(&[]),
            "no lists"
        );
    }

    /// Planet `id`, dominated (`Flags2` 0x0020), paying `tribute`.
    fn dominated(id: i16, tribute: i16) -> LandingSite {
        LandingSite {
            flags2: DOMINATED,
            tribute,
            tech_level: 5,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// [`alternates`] with 131's planet 140 dominated, paying 250.
    fn tributary() -> FakePilotCatalog {
        let mut catalog = alternates();
        catalog.sites[1] = (SystemId(131), vec![dominated(140, 250)]);
        catalog
    }

    fn income(session: &Session, catalog: &FakePilotCatalog) -> Option<IncomeLine> {
        session.player_info(catalog).income
    }

    #[test]
    fn a_dominated_stellar_in_a_visible_system_pays_its_tribute() {
        let catalog = tributary();
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(income(&session, &catalog), Some(IncomeLine::Income(250)));
        let mut catalog = tributary();
        catalog.sites[1] = (SystemId(131), vec![dominated(140, -1)]);
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(
            income(&session, &catalog),
            Some(IncomeLine::Income(5000)),
            "a thousand a tech level"
        );
        let mut catalog = tributary();
        catalog.sites[1] = (
            SystemId(131),
            vec![LandingSite {
                tech_level: 5,
                ..planet(140, 0.0, 0.0)
            }],
        );
        let session = Session::start(&catalog).expect("starts");
        assert_eq!(income(&session, &catalog), None, "not dominated");
    }

    #[test]
    fn a_dominated_stellar_in_a_hidden_system_pays_none() {
        let catalog = tributary();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131]));
        assert_eq!(income(&session, &catalog), None);
    }

    #[test]
    fn a_dominated_stellar_with_no_owner_pays_only_when_rescued_into_the_players_system() {
        let catalog = tributary();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131, 134]));
        assert_eq!(session.galaxy().owner(&[SystemId(131)]), None);
        assert_eq!(
            income(&session, &catalog),
            None,
            "no owner, not the player's"
        );
        let mut catalog = FakePilotCatalog {
            character: starting([Some(131), None, None, None]).character,
            ..crate::testkit::catalog()
        };
        with_alternate(&mut catalog);
        catalog.sites[1] = (SystemId(131), vec![dominated(140, 250)]);
        let mut session = Session::start(&catalog).expect("starts");
        assert!(hide(&mut session, &catalog, &[131, 134]));
        assert_eq!(session.system(), SystemId(131));
        assert_eq!(
            session.galaxy().owner(&[SystemId(131)]),
            None,
            "made visible"
        );
        assert_eq!(
            income(&session, &catalog),
            Some(IncomeLine::Income(250)),
            "the rescue holds it in the player's system"
        );
    }

    /// [`catalog`] with ship class 129 costing 50,000.
    fn with_escort_class() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![ShipRecord {
                cost: 50_000,
                ..ship(129, FAST)
            }],
            ..catalog()
        }
    }

    fn escort(wage: Option<i64>) -> Escort {
        Escort {
            ship: ShipId(129),
            reserves: Reserves::default(),
            order: None,
            carried: false,
            wage,
            person: None,
        }
    }

    #[test]
    fn a_hired_escort_costs_a_hundredth_of_its_class_and_a_captured_one_nothing() {
        let catalog = with_escort_class();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(!session.player_info(&catalog).fleet);
        session.pilot.escorts.push(escort(Some(0)));
        assert_eq!(income(&session, &catalog), Some(IncomeLine::Expenses(500)));
        assert!(
            session.player_info(&catalog).fleet,
            "the trader escort's holds"
        );
        session.pilot.escorts[0] = escort(None);
        assert_eq!(income(&session, &catalog), None);
        session.pilot.escorts[0] = Escort {
            ship: ShipId(200),
            ..escort(Some(0))
        };
        assert_eq!(income(&session, &catalog), None, "no record");
    }

    #[test]
    fn the_salaries_add_to_the_income() {
        let catalog = FakePilotCatalog {
            ranks: vec![RankRecord {
                salary: 300,
                ..rank(128)
            }],
            ..with_escort_class()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.ranks.insert(RankId(128));
        session.pilot.escorts.push(escort(Some(0)));
        assert_eq!(income(&session, &catalog), Some(IncomeLine::Expenses(200)));
    }

    /// [`catalog`] naming system 130 "Sol" and flying a recorded ship 128.
    fn named() -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            ship_records: vec![ship(128, FAST)],
            ..catalog()
        };
        catalog
            .place_names
            .systems
            .insert(SystemId(130), "Sol".to_owned());
        catalog
    }

    #[test]
    fn a_fresh_sessions_player_info() {
        let catalog = named();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.cash = 12_345;
        session.pilot.ship_name = Some("Bitsy".to_owned());
        let info = session.player_info(&catalog);
        assert_eq!(
            info,
            PlayerInfo {
                pilot_name: String::new(),
                date: session.med_date_text(),
                system: "Sol".to_owned(),
                legal_status: String::new(),
                ship_name: "Bitsy".to_owned(),
                ship_class: "Ship 128".to_owned(),
                turn_rate: 90,
                accel: 450,
                max_speed: 600,
                credits: 12_345,
                shield: Some(100),
                armor: Some(100),
                energy: Some(EnergyShown {
                    percent: 100,
                    reach: Reach::Jumps {
                        jumps: 3,
                        spare: false
                    }
                }),
                income: None,
                fleet: false,
                cargo: None,
                extras: None,
                honors: None,
                jettisonable: false,
                words: Words::from_list(&[]),
            }
        );
        assert!(!info.date.is_empty());
    }

    #[test]
    fn the_reserves_read_the_gauges_against_the_stats() {
        let catalog = named();
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.reserves.shield.now = 15.0;
        session.pilot.reserves.armor.now = 9.0;
        session.pilot.reserves.fuel.now = 40.0;
        let info = session.player_info(&catalog);
        assert_eq!((info.shield, info.armor), (Some(50), Some(20)));
        assert_eq!(
            info.energy,
            Some(EnergyShown {
                percent: 13,
                reach: Reach::ManeuveringOnly
            })
        );
    }

    #[test]
    fn the_stat_rows_count_the_outfits() {
        let mut catalog = named();
        catalog.outfits = vec![outfit(
            200,
            &[
                (crate::stats::MORE_SPEED, 100),
                (crate::stats::MORE_ACCEL, 100),
                (crate::stats::TURN_CHANGE, 100),
            ],
        )];
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.outfits.insert(OutfitId(200), 2);
        let info = session.player_info(&catalog);
        assert_eq!(
            (info.max_speed, info.accel, info.turn_rate),
            (800, 550, 150)
        );
    }

    /// [`catalog`] with mission 140 carrying `tons` of cargo type 1,
    /// abortable as `can_abort` says, and `jünk` 146 "opal".
    fn cargo_catalog(can_abort: i16) -> FakePilotCatalog {
        FakePilotCatalog {
            missions: vec![MissionRecord {
                can_abort,
                ..mission(140)
            }],
            junk: vec![JunkRecord {
                id: JunkId(146),
                name: "Opals".to_owned(),
                abbrev: "Opl".to_owned(),
                base_price: 100,
                sold_at: Vec::new(),
                bought_at: Vec::new(),
                buy_on: Test::default(),
                sell_on: Test::default(),
                flags: 0,
                scan_mask: 0,
                lc_name: "opal".to_owned(),
            }],
            ..catalog()
        }
    }

    #[test]
    fn the_cargo_text_lists_the_goods_held_and_the_mission_cargo_aboard() {
        let catalog = cargo_catalog(0);
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(session.player_info(&catalog).cargo, None);
        session.pilot.cargo.insert(Good::Commodity(0), 3);
        session.pilot.cargo.insert(Good::Junk(JunkId(146)), 2);
        let mut carried = carrying(140, 4);
        carried.cargo_type = Some(1);
        carried.aboard = true;
        session.pilot.missions.put(0, carried);
        let info = session.player_info(&catalog);
        assert_eq!(
            info.cargo.as_deref(),
            Some(" :\r\rThree   food, passengers, two   opal.\r\r: 11 "),
            "the mission's passengers merged into slot 1; 20 tons less 9 free"
        );
        assert!(info.jettisonable, "the commodity");
    }

    #[test]
    fn mission_cargo_alone_is_jettisonable_only_when_it_can_be_aborted() {
        for (can_abort, jettisonable) in [(0, false), (1, true)] {
            let catalog = cargo_catalog(can_abort);
            let mut session = Session::start(&catalog).expect("starts");
            let mut carried = carrying(140, 0);
            carried.cargo_type = Some(1);
            carried.aboard = true;
            session.pilot.missions.put(0, carried.clone());
            let info = session.player_info(&catalog);
            assert!(info.cargo.is_some());
            assert_eq!(info.jettisonable, jettisonable, "CanAbort {can_abort}");
            carried.aboard = false;
            session.pilot.missions.put(0, carried.clone());
            assert_eq!(session.player_info(&catalog).cargo, None, "not aboard");
            carried.aboard = true;
            carried.cargo_type = None;
            session.pilot.missions.put(0, carried.clone());
            assert_eq!(session.player_info(&catalog).cargo, None, "untyped");
            carried.cargo_type = Some(1);
            carried.cargo_tons = -1;
            session.pilot.missions.put(0, carried);
            assert_eq!(session.player_info(&catalog).cargo, None, "below 0 tons");
        }
    }

    #[test]
    fn the_extras_and_honors_read_the_outfits_owned_and_the_ranks() {
        let mut catalog = named();
        catalog.outfits = vec![
            OutfitRecord {
                lc_name: "pod".to_owned(),
                cost: 1000,
                ..outfit(200, &[])
            },
            OutfitRecord {
                lc_name: "badge".to_owned(),
                flags: OutfitFlags::HONOR,
                ..outfit(201, &[])
            },
        ];
        catalog.ranks = vec![RankRecord {
            name: "Commander".to_owned(),
            ..rank(128)
        }];
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.outfits.insert(OutfitId(200), 1);
        let info = session.player_info(&catalog);
        assert_eq!(
            info.extras.as_deref(),
            Some("\r\r pod.\r\r 3,000 \r"),
            "no lists: empty words; the hull's quarter and half the pod"
        );
        assert_eq!(info.honors, None);
        session.pilot.outfits.insert(OutfitId(201), 1);
        session.pilot.ranks.insert(RankId(128));
        let honors = session.player_info(&catalog).honors;
        assert_eq!(
            honors.as_deref(),
            Some("\r\rCommander,   badge\r"),
            "the badge follows the ranks, its \"and\" and \"a\" empty"
        );
    }

    #[test]
    fn an_active_rank_with_an_empty_name_shows_an_empty_honors_text() {
        let catalog = FakePilotCatalog {
            ranks: vec![rank(128)],
            ..named()
        };
        let mut session = Session::start(&catalog).expect("starts");
        session.pilot.ranks.insert(RankId(128));
        assert_eq!(session.player_info(&catalog).honors.as_deref(), Some(""));
    }

    /// [`alternates`] with 130 and 131 under government 128 (`CrimeTol`
    /// 10) and 134, at 131's place, under xenophobic 129; the ship
    /// starting in 131 with a record of 50 there.
    fn governed() -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            character: starting([Some(131), None, None, None]).character,
            ..alternates()
        };
        for star in &mut catalog.star_map {
            star.govt = match star.id.0 {
                130 | 131 => Some(GovtId(128)),
                134 => Some(GovtId(129)),
                _ => None,
            };
        }
        catalog.govts = vec![
            GovtRecord {
                crime_tol: 10,
                ..govt(128)
            },
            GovtRecord {
                flags: XENOPHOBIC,
                ..govt(129)
            },
        ];
        catalog
    }

    fn legal(session: &Session, catalog: &FakePilotCatalog) -> String {
        session.player_info(catalog).legal_status
    }

    #[test]
    fn the_legal_status_reads_the_record_and_the_systems_government() {
        let catalog = governed();
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(legal(&session, &catalog), "status 2", "no record: index 1");
        session.pilot.set_system_record(SystemId(131), 50);
        assert_eq!(legal(&session, &catalog), "status 12", "above 4t: index 11");
        assert!(hide(&mut session, &catalog, &[131]));
        assert_eq!(session.system(), SystemId(134), "swapped for 134");
        assert_eq!(
            legal(&session, &catalog),
            "N/A",
            "134's government is xenophobic"
        );
    }

    #[test]
    fn an_independent_system_reads_government_slot_zeros_tolerance() {
        let mut catalog = governed();
        for star in &mut catalog.star_map {
            star.govt = None;
        }
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        session.pilot.set_system_record(SystemId(131), 50);
        assert_eq!(legal(&session, &catalog), "status 12", "128's CrimeTol 10");
        catalog.govts[0].flags = XENOPHOBIC;
        let session2 = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(
            legal(&session2, &catalog),
            "status 2",
            "slot 0's xenophobia is not read"
        );
    }

    #[test]
    fn dominated_counting_stellars_make_the_status_military() {
        let mut catalog = governed();
        catalog.sites[1] = (
            SystemId(131),
            vec![
                dominated(140, 0),
                planet(142, 0.0, 0.0),
                LandingSite {
                    flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
                    ..dominated(143, 0)
                },
                dominated(144, 0),
            ],
        );
        let session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(legal(&session, &catalog), "status 17", "some: index 16");
        catalog.sites[1].1[1] = LandingSite {
            flags2: 0x1000,
            ..planet(142, 0.0, 0.0)
        };
        let session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(
            legal(&session, &catalog),
            "status 18",
            "all the counting ones: index 17; the gate, the uninhabited one and \
             the fourth do not count"
        );
        catalog.sites[1].1[1] = LandingSite {
            flags: 0,
            ..planet(142, 0.0, 0.0)
        };
        let session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(legal(&session, &catalog), "status 18", "unlandable");
    }

    #[test]
    fn an_uninhabited_system_shows_n_a_by_its_first_four_stellars() {
        let uninhabited_site = |id| LandingSite {
            flags: StellarFlags::CAN_LAND | StellarFlags::UNINHABITED,
            ..planet(id, 0.0, 0.0)
        };
        let mut catalog = governed();
        catalog.sites[1] = (
            SystemId(131),
            vec![
                uninhabited_site(140),
                LandingSite {
                    flags2: 0x2000,
                    ..planet(142, 0.0, 0.0)
                },
                uninhabited_site(143),
                uninhabited_site(144),
                planet(145, 0.0, 0.0),
            ],
        );
        let session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        assert_eq!(legal(&session, &catalog), "N/A", "the fifth does not count");
        let info = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(NotAvailable))
            .player_info(&catalog);
        assert_eq!(info.legal_status, "n/a", "#396 from the lists");
        assert_eq!(
            &*info.words.not_available, "n/a",
            "the same #396 for the screen's other rows"
        );
    }

    /// `STR#` 2002 with #396 "n/a".
    #[derive(Debug)]
    struct NotAvailable;

    impl CommCatalog for NotAvailable {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == MISC_STRINGS {
                let mut strings = vec![String::new(); 400];
                strings[395] = "n/a".to_owned();
                strings
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn the_ranks_line_counts_the_special_outfits_following_it() {
        let mut catalog = named();
        catalog.outfits = vec![OutfitRecord {
            lc_name: "badge".to_owned(),
            flags: OutfitFlags::HONOR,
            ..outfit(201, &[])
        }];
        catalog.ranks = vec![RankRecord {
            name: "Commander".to_owned(),
            ..rank(128)
        }];
        let mut session = Session::start(&catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists));
        session.pilot.ranks.insert(RankId(128));
        assert_eq!(session.pilot_info_ranks().line, "Commander");
        session.pilot.outfits.insert(OutfitId(201), 1);
        assert_eq!(session.pilot_info_ranks().line, "Commander, und ");
        assert_eq!(session.pilot_info_ranks().listed, 1);
    }

    #[test]
    fn a_missing_legal_status_string_reads_empty() {
        let session = Session::start(&governed()).expect("starts");
        assert_eq!(legal(&session, &governed()), "");
    }
}
