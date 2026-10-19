//! What the galaxy map's info panels ask of the session
//! ([`Session::map_panel`], the session's side of
//! [`map_panel::map_panel`]; see that module for the original's rules).
//!
//! The inputs, for the system the map selects:
//!
//! - its explored level, its own under both readings of
//!   [`RuleKey::ExploredByPlace`] (`_selSystem`'s +0x90, @0x1035d);
//! - its name and its government's (the overlay's government), as the
//!   catalog names them;
//! - its stellars, `catalog`'s landing sites in `NavDefs` order stamped by
//!   the overlay, each named by the catalog, with its `Gravity`, and
//!   active while a visible system holds it
//!   ([`OverlaidGalaxy::active_holder`](crate::story::OverlaidGalaxy::active_holder),
//!   +0x44);
//! - whether it is inhabited, [`exploration::inhabited`] as
//!   [`RuleKey::MapExplore`] reads it;
//! - its `Asteroids`, the count its asteroid field spawns from, its
//!   `Interference`, raw, and its `Murk` as loaded ([`murk::loaded_murk`]
//!   as [`RuleKey::LowMurk`] reads it: the runtime +0xbc the panel reads,
//!   which `_LoadObjectData` sets @0x77eb7-0x77f08);
//! - the legal status string in it (`_DrawLegalStatusString(_selSystem)`
//!   @0x10eb0; `Session::legal_status_in`), its record read as
//!   [`RuleKey::LegalRecords`] says;
//! - the standard commodities' names, as the trade screen names them
//!   (`_cargoName`, [`market::commodities`]);
//! - today's date ([`Session::med_date_text`]).
//!
//! With nothing selected the panel reads as for an unexplored system.

use super::{Session, overlaid_sites};
use crate::catalog::{PilotCatalog, SystemId};
use crate::exploration;
use crate::map_panel::{self, MapPanel, PanelInput, PanelStellar, PanelWords};
use crate::market;
use crate::murk;
use crate::news::MISC_STRINGS;
use crate::pilot::ExploredLevel;
use crate::rulebook::RuleKey;

impl Session {
    /// The galaxy map's two info panels (see the module docs), for the
    /// system `selected` on the map, `catalog` the stellars, names and
    /// commodities are read from; `hypergate`, on the hypergate map, the
    /// systems its gate offers.
    #[must_use]
    pub fn map_panel(
        &self,
        catalog: &impl PilotCatalog,
        selected: Option<SystemId>,
        hypergate: Option<&[SystemId]>,
    ) -> MapPanel {
        let words = PanelWords::from_list(&self.strings().string_list(MISC_STRINGS));
        let commodities: Vec<String> = market::commodities(&catalog.commodity_strings())
            .into_iter()
            .map(|(_, commodity)| commodity.name)
            .collect();
        let date = self.med_date_text();
        // An unexplored system's panel reads nothing more of it.
        let facts = selected
            .filter(|&system| self.pilot.explored_level(system) > ExploredLevel::Unexplored)
            .map(|system| self.panel_facts(catalog, system));
        let facts = facts.as_ref();
        let input = PanelInput {
            words: &words,
            selected,
            current: self.pilot.system,
            nav: self.nav_mode(),
            hypergate,
            level: facts.map_or(ExploredLevel::Unexplored, |facts| facts.level),
            name: facts.map_or("", |facts| &facts.name),
            government: facts.and_then(|facts| facts.government.as_deref()),
            asteroids: facts.map_or(0, |facts| facts.asteroids),
            interference: facts.map_or(0, |facts| facts.interference),
            murk: facts.map_or(0, |facts| facts.murk),
            stellars: facts.map_or(&[], |facts| &facts.stellars),
            inhabited: facts.is_some_and(|facts| facts.inhabited),
            legal: facts.map_or("", |facts| &facts.legal),
            commodities: &commodities,
            date: &date,
        };
        map_panel::map_panel(&input)
    }

    /// What the panel reads of `system` (see the module docs).
    fn panel_facts(&self, catalog: &impl PilotCatalog, system: SystemId) -> PanelFacts {
        let map = self.galaxy.map();
        let stellars = overlaid_sites(&self.galaxy, catalog, system)
            .into_iter()
            .map(|site| PanelStellar {
                name: self
                    .statics
                    .place_names
                    .stellars
                    .get(&site.id)
                    .cloned()
                    .unwrap_or_default(),
                flags: site.flags,
                flags2: site.flags2,
                gravity: site.gravity,
                active: self
                    .galaxy
                    .active_holder(self.listers(site.id), self.pilot.system)
                    .is_some(),
                destroyed: site.destroyed,
            })
            .collect();
        PanelFacts {
            level: self.pilot.explored_level(system),
            name: self
                .statics
                .place_names
                .systems
                .get(&system)
                .cloned()
                .unwrap_or_default(),
            government: self.galaxy.govt(system).map(|govt| {
                self.statics
                    .govts
                    .get(govt)
                    .map(|record| record.name.clone())
                    .unwrap_or_default()
            }),
            asteroids: self
                .statics
                .asteroid_fields
                .get(&system)
                .map_or(0, |field| field.count),
            interference: map.interference(system),
            murk: murk::loaded_murk(map.murk(system), self.rules.source_for(RuleKey::LowMurk)),
            stellars,
            inhabited: exploration::inhabited(
                map.stellars(system),
                self.rules.source_for(RuleKey::MapExplore),
            ),
            legal: self.legal_status_in(catalog, system, &self.player_info_words()),
        }
    }
}

/// What the panel reads of the selected system.
struct PanelFacts {
    level: ExploredLevel,
    name: String,
    government: Option<String>,
    asteroids: i16,
    interference: i16,
    murk: i16,
    stellars: Vec<PanelStellar>,
    inhabited: bool,
    legal: String,
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::catalog::{
        AsteroidFieldRecord, CommCatalog, CommodityStrings, GovtId, GovtRecord, LandingSite,
        StellarId,
    };
    use crate::map_panel::{ColumnBody, Listing, Ports, Services, Strip};
    use crate::market::DOMINATED;
    use crate::player_info::LEGAL_STATUS_STRINGS;
    use crate::rulebook::{RuleSource, Rulebook};
    use crate::session::tests::{alternates, hide};
    use crate::testkit::{FakePilotCatalog, govt, planet, starting};

    /// `STR#` 2002 with each of #308-#340 "#n"; `STR#` 134, the legal
    /// statuses, each "status n" for its entry n.
    #[derive(Debug)]
    struct Lists;

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            match id {
                MISC_STRINGS => (1..=400).map(|n| format!("#{n}")).collect(),
                LEGAL_STATUS_STRINGS => (1..=18).map(|n| format!("status {n}")).collect(),
                _ => Vec::new(),
            }
        }
    }

    /// A planet with every service and the first commodity at a low
    /// price.
    fn port(id: i16) -> LandingSite {
        LandingSite {
            flags: 0x1000_0000 | 0x0001 | 0x0002 | 0x0004 | 0x0008,
            ..planet(id, 0.0, 0.0)
        }
    }

    /// [`alternates`] starting in 131, with 130 and 131 under government
    /// 128 ("Federation", `CrimeTol` 10), 140 ("Auroran", `CrimeTol` 10)
    /// besides; 130 holds port 128 ("Earth") and planet 129 ("Moon"), and
    /// is named "Sol"; the first commodity is "Food".
    fn panelled() -> FakePilotCatalog {
        let mut catalog = FakePilotCatalog {
            character: starting([Some(131), None, None, None]).character,
            ..alternates()
        };
        for star in &mut catalog.star_map {
            star.govt = matches!(star.id.0, 130 | 131).then_some(GovtId(128));
        }
        catalog.govts = vec![
            GovtRecord {
                name: "Federation".to_owned(),
                crime_tol: 10,
                ..govt(128)
            },
            GovtRecord {
                name: "Auroran".to_owned(),
                crime_tol: 10,
                ..govt(140)
            },
        ];
        catalog.sites[0] = (SystemId(130), vec![port(128), planet(129, 0.0, 0.0)]);
        catalog
            .place_names
            .systems
            .insert(SystemId(130), "Sol".to_owned());
        for (id, name) in [(128, "Earth"), (129, "Moon")] {
            catalog
                .place_names
                .stellars
                .insert(StellarId(id), name.to_owned());
        }
        catalog.commodities = CommodityStrings {
            names: (1..=6).map(|n| format!("good {n}")).collect(),
            ..CommodityStrings::default()
        };
        synced(catalog)
    }

    /// `catalog` with each system's star-map stellars its landing sites'
    /// `Flags` and `Flags2`, as the catalog adapter reads both from the
    /// same `NavDefs`.
    fn synced(mut catalog: FakePilotCatalog) -> FakePilotCatalog {
        for (system, sites) in &catalog.sites {
            if let Some(star) = catalog.star_map.iter_mut().find(|star| star.id == *system) {
                star.stellars = sites
                    .iter()
                    .map(|site| Some((site.flags, site.flags2)))
                    .collect();
            }
        }
        catalog
    }

    fn session(catalog: &FakePilotCatalog, rules: Rulebook) -> Session {
        Session::start(catalog)
            .expect("starts")
            .with_strings(Rc::new(Lists))
            .with_rules(rules)
    }

    fn panel(session: &Session, catalog: &FakePilotCatalog, system: i16) -> MapPanel {
        session.map_panel(catalog, Some(SystemId(system)), None)
    }

    fn body(session: &Session, catalog: &FakePilotCatalog, system: i16) -> ColumnBody {
        panel(session, catalog, system).column.body
    }

    fn legal(body: &ColumnBody) -> &str {
        match body {
            ColumnBody::Inhabited { legal, .. } => legal,
            body => panic!("{body:?}"),
        }
    }

    #[test]
    fn the_panel_reads_the_selected_systems_name_ports_government_and_level() {
        let catalog = panelled();
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.chart(SystemId(130));
        let panel = panel(&session, &catalog, 130);
        assert_eq!(panel.column.title, "#338");
        let Strip::Info { ports, date, .. } = &panel.strip else {
            panic!("the info strip");
        };
        assert_eq!(*ports, Ports::Listed(vec!["Earth".into(), "Moon".into()]));
        assert_eq!(*date, session.med_date_text());
        let ColumnBody::Inhabited {
            name,
            government,
            goods,
            services,
            ..
        } = panel.column.body
        else {
            panic!("inhabited");
        };
        assert_eq!((name.as_str(), government.as_str()), ("Sol", "Federation"));
        assert_eq!(goods, Listing::Items(vec!["good 1".into()]));
        assert_eq!(
            services,
            Services::Items(vec!["#329".into(), "#330".into(), "#331".into()])
        );
        assert_eq!(panel.words.ports, "#309");
    }

    #[test]
    fn a_level_1_and_a_level_2_system_show_different_goods_and_services() {
        let catalog = panelled();
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(130));
        let ColumnBody::Inhabited {
            goods, services, ..
        } = body(&session, &catalog, 130)
        else {
            panic!("inhabited");
        };
        assert_eq!(
            (goods, services),
            (Listing::Unknown, Services::Unknown { any: true })
        );
        session.pilot.chart(SystemId(130));
        let ColumnBody::Inhabited { goods, .. } = body(&session, &catalog, 130) else {
            panic!("inhabited");
        };
        assert_eq!(goods, Listing::Items(vec!["good 1".into()]));
        assert_eq!(
            body(&session, &catalog, 132),
            ColumnBody::Unknown,
            "unexplored"
        );
    }

    #[test]
    fn the_legal_status_reads_the_selected_systems_record_as_the_rule_says() {
        let catalog = panelled();
        // By the engine, the record in each system.
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(130));
        session.pilot.set_system_record(SystemId(130), 50);
        session.pilot.set_system_record(SystemId(131), 0);
        assert_eq!(legal(&body(&session, &catalog, 130)), "status 12");
        assert_eq!(legal(&body(&session, &catalog, 131)), "status 2");
        // By the Bible, the record with each system's government.
        let rules = Rulebook::default().with_override(RuleKey::LegalRecords, RuleSource::Bible);
        let mut session = self::session(&catalog, rules);
        session.pilot.explore(SystemId(130));
        session.pilot.set_legal_record(GovtId(128), 50);
        assert!(session.change_galaxy(&catalog, |galaxy| {
            galaxy.set_govt(SystemId(131), Some(Some(GovtId(140))));
        }));
        assert_eq!(legal(&body(&session, &catalog, 130)), "status 12");
        assert_eq!(legal(&body(&session, &catalog, 131)), "status 2");
    }

    #[test]
    fn uninhabited_follows_the_map_explore_rule() {
        // 130's only inhabited stellar is in its fifth slot.
        let mut catalog = panelled();
        let uninhabited = Some((0x0021, 0));
        catalog.star_map[0].stellars = vec![
            uninhabited,
            uninhabited,
            uninhabited,
            uninhabited,
            Some((0x0001, 0)),
        ];
        for source in RuleSource::ALL {
            let rules = Rulebook::default().with_override(RuleKey::MapExplore, source);
            let mut session = session(&catalog, rules);
            session.pilot.explore(SystemId(130));
            let body = body(&session, &catalog, 130);
            match source {
                RuleSource::Engine => assert_eq!(
                    body,
                    ColumnBody::Uninhabited {
                        name: "Sol".to_owned()
                    }
                ),
                RuleSource::Bible => {
                    assert!(matches!(body, ColumnBody::Inhabited { .. }), "{body:?}");
                }
            }
        }
    }

    fn government(body: &ColumnBody) -> &str {
        match body {
            ColumnBody::Inhabited { government, .. } => government,
            body => panic!("{body:?}"),
        }
    }

    #[test]
    fn the_government_follows_the_overlay() {
        let catalog = panelled();
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(130));
        assert!(session.change_galaxy(&catalog, |galaxy| {
            galaxy.set_govt(SystemId(130), Some(Some(GovtId(140))));
        }));
        assert_eq!(government(&body(&session, &catalog, 130)), "Auroran");
        assert!(session.change_galaxy(&catalog, |galaxy| {
            galaxy.set_govt(SystemId(130), Some(None));
        }));
        assert_eq!(government(&body(&session, &catalog, 130)), "#333");
        assert!(session.change_galaxy(&catalog, |galaxy| {
            galaxy.set_govt(SystemId(130), Some(Some(GovtId(150))));
        }));
        assert_eq!(
            government(&body(&session, &catalog, 130)),
            "",
            "a government with no record has no name"
        );
    }

    #[test]
    fn the_hazards_read_the_systems_field_interference_murk_and_gravity() {
        let mut catalog = panelled();
        catalog.star_map[0].interference = 40;
        catalog.star_map[0].murk = 61;
        catalog.asteroid_fields = vec![AsteroidFieldRecord {
            system: SystemId(130),
            count: 2,
            types: 1,
        }];
        catalog.sites[0].1[1].gravity = 5;
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(130));
        let Strip::Info { hazards, .. } = panel(&session, &catalog, 130).strip else {
            panic!("the info strip");
        };
        assert_eq!(hazards, "#312, #313 #316, #318 #320, #323 #324");
    }

    #[test]
    fn the_visibility_hazard_reads_the_murk_as_loaded() {
        // `_MapDialogUpdate` reads the runtime +0xbc (@0x1086c-0x1090c),
        // which `_LoadObjectData` zeroes for a `Murk` of 1 to 7
        // (@0x77ee6-0x77ef8) and caps at 100 (@0x77ed5-0x77edb).
        let hazards = |murk, rulebook| {
            let mut catalog = panelled();
            catalog.star_map[0].murk = murk;
            let mut session = session(&catalog, rulebook);
            session.pilot.explore(SystemId(130));
            let Strip::Info { hazards, .. } = panel(&session, &catalog, 130).strip else {
                panic!("the info strip");
            };
            hazards
        };
        let bible = Rulebook::default().with_override(RuleKey::LowMurk, RuleSource::Bible);
        assert_eq!(hazards(5, Rulebook::default()), "#335", "loaded as none");
        assert_eq!(hazards(5, bible), "#321 #324");
        assert_eq!(hazards(8, Rulebook::default()), "#321 #324");
        assert_eq!(hazards(-4, Rulebook::default()), "#335");
    }

    #[test]
    fn a_stellar_no_visible_system_holds_serves_nothing() {
        // 134, at 131's place, alone lists port 141: while 131 stands for
        // the place, no visible system holds it, so it is inactive.
        let mut catalog = panelled();
        catalog.sites.retain(|(system, _)| system.0 != 134);
        catalog.sites.push((SystemId(134), vec![port(141)]));
        let catalog = synced(catalog);
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.chart(SystemId(134));
        let services = |session: &Session| match body(session, &catalog, 134) {
            ColumnBody::Inhabited { services, .. } => services,
            body => panic!("{body:?}"),
        };
        assert_eq!(services(&session), Services::NoneAvailable);
        let Strip::Info { ports, .. } = panel(&session, &catalog, 134).strip else {
            panic!("the info strip");
        };
        assert_eq!(
            ports,
            Ports::Listed(vec![String::new()]),
            "listed all the same"
        );
        assert!(hide(&mut session, &catalog, &[131]));
        assert!(
            matches!(services(&session), Services::Items(_)),
            "134 holds it"
        );
    }

    #[test]
    fn a_dominated_stellar_no_visible_system_holds_leaves_the_legal_status_undominated() {
        // 134, at 131's place, alone lists dominated port 141: while 131
        // stands for the place, it is inactive and does not count.
        let mut catalog = panelled();
        catalog.sites.retain(|(system, _)| system.0 != 134);
        catalog.sites.push((
            SystemId(134),
            vec![LandingSite {
                flags2: DOMINATED,
                ..port(141)
            }],
        ));
        let catalog = synced(catalog);
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.explore(SystemId(134));
        assert_eq!(
            legal(&body(&session, &catalog, 134)),
            "status 2",
            "no counting stellar: the record's status"
        );
        assert!(hide(&mut session, &catalog, &[131]));
        assert_eq!(
            legal(&body(&session, &catalog, 134)),
            "status 18",
            "134 holds it: every counting stellar dominated"
        );
    }

    #[test]
    fn an_unexplored_systems_panel_reads_none_of_its_stellars() {
        let catalog = panelled();
        let mut session = session(&catalog, Rulebook::default());
        catalog.sites_asked.borrow_mut().clear();
        assert_eq!(body(&session, &catalog, 130), ColumnBody::Unknown);
        assert_eq!(*catalog.sites_asked.borrow(), [], "unexplored");
        session.pilot.explore(SystemId(130));
        body(&session, &catalog, 130);
        assert!(catalog.sites_asked.borrow().contains(&SystemId(130)));
    }

    #[test]
    fn no_selection_reads_unexplored_and_the_gate_titles_its_offers() {
        let catalog = panelled();
        let mut session = session(&catalog, Rulebook::default());
        session.pilot.chart(SystemId(130));
        let none = session.map_panel(&catalog, None, None);
        assert_eq!(none.column.body, ColumnBody::Unknown);
        assert_eq!(none.column.title, "#338");
        let offers = [SystemId(130)];
        let offered = session.map_panel(&catalog, Some(SystemId(130)), Some(&offers));
        assert_eq!(offered.column.title, "#340");
        let here = session.map_panel(&catalog, Some(SystemId(131)), Some(&offers));
        assert!(matches!(here.strip, Strip::HypergatePrompt { .. }));
    }
}
