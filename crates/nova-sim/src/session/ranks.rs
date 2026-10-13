//! The ranks on the session (see [`rank`](crate::rank)).
//!
//! `K` and `L` apply at once, inside the script that runs them, as the
//! original's `_EvalSetExp` calls `_ActivateRank` and `_DeactivateRank`
//! as it meets them (@0x159e1, @0x15a17); neither runs a script. The
//! sweeps read the session's
//! [`RankUnaffiliatedGroup`](crate::rulebook::RuleKey::RankUnaffiliatedGroup)
//! rule. A change to the active ranks makes a save due, as any change a
//! set expression makes to the pilot does. The recent rank `<RRK>` reads
//! is the session's, never saved, and none on every
//! [`Session::fly`](crate::Session::fly), as the original's `ResetPlayer`
//! (@0x1db44) and `ResetPlayerRecord` (@0x1dbf1) reset it.
//!
//! [`Session::pilot_info_ranks`] gives the pilot info's ranks line and
//! the salaries' share of the daily income, for the player info screen
//! to show.
//!
//! The daily step's eighth participant pays the active ranks' salaries
//! ([`rank::pay_salaries`]), reading the session's
//! [`SalaryCap`](crate::rulebook::RuleKey::SalaryCap) rule; a change to
//! the cash makes a save due.
//!
//! **Prices.** As the spaceport opens (a landing, a relocation that
//! docks, and a load that resumes docked, which opens the port too), the
//! session takes the active ranks and the docked stellar's government,
//! never saved, as `_DoPortDialog` takes `_priceFluxMult`
//! (@0x5f970-0x5fa04). It does so before the landing's mission land check
//! (@0x5fbeb) and its rebuild point (@0x5fbf0), so a rank a mission
//! completed on this landing grants, and a change of the stellar's
//! government at the rebuild, count only from the next landing.
//! [`Session::price_mults`] works the multipliers out from them by the
//! engine's reading of
//! [`RankPriceTiming`](crate::rulebook::RuleKey::RankPriceTiming); by the
//! other reading, from the ranks and the government as they are when a
//! price is asked. Either way the ranks reach the stellar's government as
//! [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) says, read
//! when the price is asked, so the session's rules always apply. The
//! shipyard's prices and trade-in and the bar's hire fee read them (see
//! [`rank`]'s **Prices**).
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::catalog::{GovtId, RankId};
use crate::news::MISC_STRINGS;
use crate::person::NO_RANK;
use crate::rank::{self, AND_STRING, Held, NO_RANK_STRING, PilotInfoRanks, PriceMults, RankNames};
use crate::rulebook::{RuleKey, RuleSource};
use crate::session::Session;

/// What `_DoPortDialog` takes the price multipliers from as the
/// spaceport opens: the active ranks and the stellar's government.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct PortRanks {
    /// The ranks active as the port opened.
    active: BTreeSet<RankId>,
    /// The docked stellar's government as the port opened.
    govt: Option<GovtId>,
}

impl Session {
    /// The docked stellar's government now: none in flight or at an
    /// independent stellar.
    fn docked_govt(&self) -> Option<GovtId> {
        let landed = self.landed?;
        self.sites.iter().find(|site| site.id == landed)?.govt
    }

    /// Takes what the price multipliers read as the spaceport opens
    /// (`_DoPortDialog` @0x5f970-0x5fa04): the active ranks and the docked
    /// stellar's government.
    pub(super) fn open_port_prices(&mut self) {
        self.port_ranks = PortRanks {
            active: self.pilot.ranks.clone(),
            govt: self.docked_govt(),
        };
    }

    /// The ranks' price multipliers, `_priceFluxMult` (see
    /// [`rank::price_mults`]), the ranks reaching the stellar's government
    /// as the session's `rank_govt_reach` rule reads. By the engine's
    /// reading of `rank_price_timing` they are those the spaceport opened
    /// with; by the other, those of the ranks and the docked stellar's
    /// government now.
    #[must_use]
    pub fn price_mults(&self) -> PriceMults {
        let reach = self.rules.source_for(RuleKey::RankGovtReach);
        let (active, govt) = match self.rules.source_for(RuleKey::RankPriceTiming) {
            RuleSource::Engine => (&self.port_ranks.active, self.port_ranks.govt),
            RuleSource::Bible => (&self.pilot.ranks, self.docked_govt()),
        };
        rank::price_mults(&self.ranks, active, govt, &self.govts, reach)
    }

    /// The pilot's ranks and the recent rank, as `K` and `L` change them.
    fn held_ranks(&mut self) -> Held<'_> {
        Held {
            active: &mut self.pilot.ranks,
            recent: &mut self.recent_rank,
        }
    }

    /// `K`: activates rank `rank` (see [`rank::activate`]), unaffiliated
    /// ranks grouped by the session's `rank_unaffiliated_group` rule.
    pub(super) fn activate_rank(&mut self, rank: RankId) {
        let unaffiliated = self.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.ranks);
        rank::activate(&records, self.held_ranks(), rank, unaffiliated);
    }

    /// `L`: deactivates rank `rank` (see [`rank::deactivate`]),
    /// unaffiliated ranks grouped by the session's
    /// `rank_unaffiliated_group` rule.
    pub(super) fn deactivate_rank(&mut self, rank: RankId) {
        let unaffiliated = self.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.ranks);
        rank::deactivate(&records, self.held_ranks(), rank, unaffiliated);
    }

    /// What the rank tags read: the pilot's active ranks, the recent
    /// rank, and `STR#` 2002 #341 from the session's string lists as the
    /// fallback, [`NO_RANK`] when the lists lack it (none are wired by
    /// default).
    pub(super) fn rank_names(&self) -> RankNames<'_> {
        RankNames {
            records: &self.ranks,
            active: &self.pilot.ranks,
            recent: self.recent_rank,
            fallback: self
                .nth_string(MISC_STRINGS, NO_RANK_STRING)
                .map_or(Cow::Borrowed(NO_RANK), Cow::Owned),
        }
    }

    /// The pilot info's ranks line and the salaries' share of the daily
    /// income (see [`rank::pilot_info`]), against the pilot's cash and the
    /// session's `salary_cap` rule, joined with `STR#` 2002 #392 (nothing
    /// when the lists lack it, as `_StrcatIndCString` appends nothing).
    /// No Ranks-section item follows the ranks until the player info
    /// screen lists them (crons-news-ranks phase 17).
    #[must_use]
    pub fn pilot_info_ranks(&self) -> PilotInfoRanks {
        let and = self
            .nth_string(MISC_STRINGS, AND_STRING)
            .unwrap_or_default();
        rank::pilot_info(
            &self.ranks,
            &self.pilot.ranks,
            self.pilot.cash,
            self.rules.source_for(RuleKey::SalaryCap),
            &and,
            0,
        )
    }

    /// The day's eighth participant: the active ranks' salaries (see
    /// [`rank::pay_salaries`]), capped by the session's `salary_cap`
    /// rule. A change to the cash makes a save due.
    pub(super) fn ranks_day(&mut self) {
        let cap = self.rules.source_for(RuleKey::SalaryCap);
        let before = self.pilot.cash;
        rank::pay_salaries(&self.ranks, &self.pilot.ranks, &mut self.pilot.cash, cap);
        if self.pilot.cash != before {
            self.save_due = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::catalog::{OutfitId, OutfitRecord, RankId, RankRecord, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::outfitter::OutfitFlags;
    use crate::pilot::Pilot;
    use crate::rank::{SWEEP_GOVT_ON_ACTIVATE, SWEEP_GOVT_ON_DEACTIVATE};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::save;
    use crate::session::Session;
    use crate::shipyard::ShipFlags3;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, land_now, outfit, rank, ship};

    fn set(text: &str) -> SetExpr {
        SetExpr::parse(text).expect("parses")
    }

    /// A session over `ranks`.
    fn ranked(ranks: Vec<RankRecord>) -> Session {
        let catalog = FakePilotCatalog { ranks, ..catalog() };
        Session::start(&catalog).expect("starts")
    }

    /// The session's active ranks.
    fn active(session: &Session) -> Vec<i16> {
        session.pilot().ranks().iter().map(|id| id.0).collect()
    }

    #[test]
    fn flying_a_pilot_forgets_a_rank_the_data_no_longer_has() {
        let catalog = FakePilotCatalog {
            ranks: vec![rank(128)],
            ..catalog()
        };
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.extend([RankId(128), RankId(200)]);
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert_eq!(pilot.ranks().len(), 2, "the save keeps it");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(
            session.pilot().ranks().iter().collect::<Vec<_>>(),
            [&RankId(128)]
        );
        let saved = save::decode(&save::encode(session.pilot())).expect("loads");
        assert_eq!(saved.ranks().iter().collect::<Vec<_>>(), [&RankId(128)]);
    }

    #[test]
    fn k_and_l_apply_at_once_inside_a_script() {
        let mut session = ranked(vec![rank(200)]);
        let mut seen = Vec::new();
        session.run_ops(
            &set("K200 L200 K201"),
            &mut Scripted::default(),
            &mut |session, _| seen.push((active(session), session.recent_rank)),
        );
        assert_eq!(
            seen,
            [
                (vec![200], Some(RankId(200))),
                (vec![], None),
                (vec![201], Some(RankId(201))),
            ],
            "each applied before the next operator; 201 has no record"
        );
        assert_eq!(session.take_script_notes(), []);
    }

    #[test]
    fn a_rank_change_makes_a_save_due() {
        let mut session = ranked(vec![rank(200)]);
        session.run_set(&set("K200"), &mut Scripted::default());
        assert!(session.take_save_due());
        session.run_set(&set("K200"), &mut Scripted::default());
        assert!(!session.take_save_due(), "already active");
        session.run_set(&set("L200"), &mut Scripted::default());
        assert!(session.take_save_due());
    }

    #[test]
    fn the_sweeps_read_the_sessions_rank_unaffiliated_group() {
        let ranks = vec![
            RankRecord {
                flags: SWEEP_GOVT_ON_ACTIVATE,
                ..rank(200)
            },
            RankRecord {
                flags: SWEEP_GOVT_ON_DEACTIVATE,
                ..rank(201)
            },
            rank(202),
        ];
        for (source, after_k, after_l) in [
            (RuleSource::Engine, vec![200], vec![]),
            (RuleSource::Bible, vec![200, 201, 202], vec![200, 202]),
        ] {
            let mut session = ranked(ranks.clone()).with_rules(
                Rulebook::default().with_override(RuleKey::RankUnaffiliatedGroup, source),
            );
            session.run_set(&set("K201 K202 K200"), &mut Scripted::default());
            assert_eq!(active(&session), after_k, "{source:?}");
            session.run_set(&set("K201 K202 L201"), &mut Scripted::default());
            assert_eq!(active(&session), after_l, "{source:?}");
        }
    }

    #[test]
    fn letting_a_day_pass_pays_each_active_ranks_salary() {
        let mut session = ranked(vec![
            RankRecord {
                salary: 100,
                ..rank(200)
            },
            RankRecord {
                salary: 50,
                salary_cap: 700,
                ..rank(201)
            },
            RankRecord {
                salary: 1000,
                ..rank(202)
            },
        ]);
        session.run_set(&set("K200 K201"), &mut Scripted::default());
        session.pilot.cash = 500;
        let _ = session.take_save_due();
        session.let_days_pass(1, &mut NeverFires);
        assert_eq!(session.pilot().cash(), 650);
        assert!(session.take_save_due());
        session.let_days_pass(2, &mut NeverFires);
        assert_eq!(session.pilot().cash(), 850, "201 stops at its cap");
    }

    #[test]
    fn the_day_reads_the_sessions_salary_cap() {
        let ranks = vec![RankRecord {
            salary: 200,
            salary_cap: 1100,
            ..rank(200)
        }];
        for (source, cash) in [(RuleSource::Engine, 1200), (RuleSource::Bible, 1100)] {
            let mut session = ranked(ranks.clone())
                .with_rules(Rulebook::default().with_override(RuleKey::SalaryCap, source));
            session.run_set(&set("K200"), &mut Scripted::default());
            session.pilot.cash = 1000;
            session.let_days_pass(1, &mut NeverFires);
            assert_eq!(session.pilot().cash(), cash, "{source:?}");
        }
    }

    #[test]
    fn granting_a_rank_adds_its_contribute_bits_to_the_players() {
        let mut session = ranked(vec![RankRecord {
            contribute: 0x0100_0000_0004,
            ..rank(200)
        }]);
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0);
        session.run_set(&set("K200"), &mut Scripted::default());
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0x0100_0000_0004);
        session.run_set(&set("L200"), &mut Scripted::default());
        assert_eq!(session.contributed() & 0x0100_0000_0004, 0);
    }

    #[test]
    fn an_active_ranks_contribute_lists_a_required_ship_and_outfit() {
        let mut catalog = crate::session::tests::shipbuying();
        catalog.ranks = vec![RankRecord {
            contribute: 0x4,
            ..rank(200)
        }];
        catalog.ship_records.push(ShipRecord {
            require: 0x4,
            flags3: ShipFlags3::HIDE_UNLESS_REQUIRED,
            ..ship(130, catalog.ship_records[0].fields)
        });
        catalog.outfits.push(OutfitRecord {
            flags: OutfitFlags::HIDE_UNLESS_REQUIRED,
            require: 0x4,
            require_govt: -1,
            tech_level: 1,
            ..outfit(310, &[])
        });
        let mut session = crate::session::tests::outfitted(&catalog);
        let listed = |session: &mut Session| {
            let yard = session.shipyard(&mut NeverFires).expect("a shipyard");
            let shop = session.outfitter(&mut NeverFires).expect("an outfitter");
            (
                yard.row(ShipId(130)).is_some(),
                shop.row(OutfitId(310)).is_some(),
            )
        };
        assert_eq!(listed(&mut session), (false, false), "not held");
        session.run_set(&set("K200"), &mut Scripted::default());
        assert_eq!(listed(&mut session), (true, true), "held");
    }

    /// Rank 128, "Commander" of government 128, "Cmdr" in conversation.
    fn commander() -> RankRecord {
        RankRecord {
            name: "Commander".to_owned(),
            conv_name: "Cmdr".to_owned(),
            short_name: "Cdr".to_owned(),
            affil_govt: Some(crate::catalog::GovtId(128)),
            ..rank(128)
        }
    }

    #[test]
    fn ranks_survive_save_and_reload_and_rrk_reads_captain_after_a_load() {
        let catalog = FakePilotCatalog {
            ranks: vec![commander()],
            ..catalog()
        };
        let mut session = Session::start(&catalog).expect("starts");
        assert_eq!(session.plain_text("<PRK> <RRK>"), "captain captain");
        session.run_set(&set("K128"), &mut Scripted::default());
        assert_eq!(
            session.plain_text("<PRK> <SRK> <RRK> <PRK128>"),
            "Cmdr Cdr Commander Cdr"
        );
        let pilot = save::decode(&save::encode(session.pilot())).expect("loads");
        let session = Session::fly(&catalog, pilot).expect("flies");
        assert_eq!(active(&session), [128]);
        assert_eq!(
            session.plain_text("<PRK> <RRK>"),
            "Cmdr captain",
            "the recent rank is not saved"
        );
    }

    /// `STR#` 2002 with #341 `fallback`.
    struct Misc(&'static str);

    impl crate::catalog::CommCatalog for Misc {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == crate::news::MISC_STRINGS {
                let mut strings = vec![String::new(); 400];
                strings[340] = self.0.to_owned();
                strings
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn the_rank_fallback_reads_str_2002_341_when_the_lists_have_it() {
        let session = ranked(vec![commander()]).with_strings(std::rc::Rc::new(Misc("skipper")));
        assert_eq!(
            session.plain_text("<PRK> <SRK> <RRK>"),
            "skipper skipper skipper"
        );
        assert_eq!(session.plain_text("<PRK128>"), "skipper");
    }

    /// `STR#` 2002 with #392 `and`, as stock has it.
    struct And(&'static str);

    impl crate::catalog::CommCatalog for And {
        fn string_list(&self, id: i16) -> Vec<String> {
            if id == crate::news::MISC_STRINGS {
                let mut strings = vec![String::new(); 400];
                strings[391] = self.0.to_owned();
                strings
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn the_pilot_info_reads_str_2002_392_and_the_cash() {
        let ranks = vec![
            RankRecord {
                salary: 200,
                ..commander()
            },
            RankRecord {
                name: "Admiral".to_owned(),
                weight: 3,
                salary: 100,
                salary_cap: 1000,
                ..rank(129)
            },
        ];
        let mut session = ranked(ranks.clone()).with_strings(std::rc::Rc::new(And("und")));
        assert_eq!(session.pilot_info_ranks().line, "");
        session.run_set(&set("K128 K129"), &mut Scripted::default());
        session.pilot.cash = 999;
        let info = session.pilot_info_ranks();
        assert_eq!(info.line, "Admiral, und Commander");
        assert_eq!(info.salary, 300);
        session.pilot.cash = 1000;
        assert_eq!(
            session.pilot_info_ranks().salary,
            200,
            "against today's cash"
        );
        let mut bare = ranked(ranks)
            .with_rules(Rulebook::default().with_override(RuleKey::SalaryCap, RuleSource::Bible));
        bare.run_set(&set("K128 K129"), &mut Scripted::default());
        bare.pilot.cash = 950;
        let info = bare.pilot_info_ranks();
        assert_eq!(info.line, "Admiral,  Commander", "no lists: no \"and\"");
        assert_eq!(info.salary, 250, "the session's salary_cap");
    }

    // Prices.

    /// Government 128 (class 1, allied with class 2), 129 (class 2) and
    /// 130 (class 3, allied with nobody).
    fn price_govts() -> Vec<crate::catalog::GovtRecord> {
        vec![
            crate::catalog::GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..crate::testkit::govt(128)
            },
            crate::catalog::GovtRecord {
                classes: [2, -1, -1, -1],
                ..crate::testkit::govt(129)
            },
            crate::catalog::GovtRecord {
                classes: [3, -1, -1, -1],
                ..crate::testkit::govt(130)
            },
        ]
    }

    /// Rank `id` of government `govt` and `PriceMod` 90.
    fn ninety(id: i16, govt: i16) -> RankRecord {
        RankRecord {
            affil_govt: Some(crate::catalog::GovtId(govt)),
            price_mod: 90,
            ..rank(id)
        }
    }

    /// [`shipbuying`](crate::session::tests::shipbuying)'s planet 128 of
    /// government 128 with a bar too, where ship 129 (17,500 credits, tech
    /// 5 as the planet) is always for hire; ranks 200 and 201 of 128 and
    /// 202 of its ally 129, each of `PriceMod` 90; and `missions`.
    ///
    /// Ship 129 is priced 17,500 with no rank, 15,700 under one (15,750,
    /// rounded down to a hundred) and 14,100 under two (14,175); its hire
    /// fee is a tenth.
    fn port(missions: Vec<crate::catalog::MissionRecord>) -> FakePilotCatalog {
        let mut catalog = crate::session::tests::shipbuying();
        for (_, sites) in &mut catalog.sites {
            for site in sites {
                site.flags |= crate::landing::StellarFlags::BAR;
            }
        }
        catalog.ship_records[1].hire_random = 100;
        catalog.govts = price_govts();
        catalog.ranks = vec![ninety(200, 128), ninety(201, 128), ninety(202, 129)];
        catalog.missions = missions;
        catalog
    }

    /// Ship 129's price in the shipyard and its hire fee in the bar.
    fn prices(session: &mut Session) -> (i64, i64) {
        let price = session
            .shipyard(&mut NeverFires)
            .expect("a shipyard")
            .row(ShipId(129))
            .expect("listed")
            .price;
        let fee = session
            .escorts_for_hire(&mut NeverFires)
            .expect("a bar")
            .row(ShipId(129))
            .expect("for hire")
            .fee;
        (price, fee)
    }

    /// A session of `catalog` under `rules`, holding `ranks`, landed at
    /// planet 128 and settled.
    fn landed_holding(catalog: &FakePilotCatalog, rules: Rulebook, ranks: &[i16]) -> Session {
        let mut session = Session::start(catalog).expect("starts").with_rules(rules);
        session
            .pilot
            .ranks
            .extend(ranks.iter().copied().map(RankId));
        land_now(&mut session).expect("lands");
        let _ = session.settle_script(catalog, &mut NeverFires);
        session
    }

    /// Takes `session` off, settles, and lands it again at planet 128.
    fn land_again(session: &mut Session, catalog: &FakePilotCatalog) {
        session
            .take_off(catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(catalog, &mut NeverFires);
        let site = session.sites[0];
        session.player.position = site.position;
        session.player.velocity = crate::geometry::Vec2::ZERO;
        land_now(session).expect("lands again");
        let _ = session.settle_script(catalog, &mut NeverFires);
    }

    /// The default rule set with `key` read by `source`.
    fn reading(key: RuleKey, source: RuleSource) -> Rulebook {
        Rulebook::default().with_override(key, source)
    }

    const UNRANKED: (i64, i64) = (17_500, 1750);
    const ONE_RANK: (i64, i64) = (15_700, 1570);
    const TWO_RANKS: (i64, i64) = (14_100, 1410);

    #[test]
    fn a_landing_takes_the_ranks_into_the_shipyards_prices() {
        let catalog = port(Vec::new());
        let rules = Rulebook::default();
        assert_eq!(prices(&mut landed_holding(&catalog, rules, &[])), UNRANKED);
        let mut one = landed_holding(&catalog, rules, &[200]);
        assert_eq!(one.price_mults().second.to_bits(), 0.9_f32.to_bits());
        assert_eq!(prices(&mut one), ONE_RANK);
        assert_eq!(
            prices(&mut landed_holding(&catalog, rules, &[200, 201])),
            TWO_RANKS
        );
    }

    #[test]
    fn by_the_engine_a_rank_gained_while_landed_changes_no_price_until_the_next_landing() {
        let catalog = port(Vec::new());
        let mut session = landed_holding(&catalog, Rulebook::default(), &[200]);
        session.run_set(&set("K201"), &mut Scripted::default());
        assert_eq!(active(&session), [200, 201]);
        assert_eq!(prices(&mut session), ONE_RANK, "taken as the port opened");
        land_again(&mut session, &catalog);
        assert_eq!(prices(&mut session), TWO_RANKS);
        session.run_set(&set("L201 L200"), &mut Scripted::default());
        assert_eq!(prices(&mut session), TWO_RANKS, "a rank lost, the same");
    }

    #[test]
    fn by_the_other_reading_a_rank_gained_while_landed_changes_prices_at_once() {
        let catalog = port(Vec::new());
        let rules = reading(RuleKey::RankPriceTiming, RuleSource::Bible);
        let mut session = landed_holding(&catalog, rules, &[200]);
        assert_eq!(prices(&mut session), ONE_RANK);
        session.run_set(&set("K201"), &mut Scripted::default());
        assert_eq!(prices(&mut session), TWO_RANKS);
        session.run_set(&set("L201"), &mut Scripted::default());
        assert_eq!(prices(&mut session), ONE_RANK);
    }

    #[test]
    fn a_rank_a_landing_mission_grants_counts_from_the_next_landing() {
        let catalog = port(vec![crate::catalog::MissionRecord {
            avail_stel: 128,
            return_stel: 128,
            on_success: crate::control::Script::parse("K201"),
            ..crate::testkit::mission(140)
        }]);
        for (timing, after_landing) in [
            (RuleSource::Engine, ONE_RANK),
            (RuleSource::Bible, TWO_RANKS),
        ] {
            let rules = reading(RuleKey::RankPriceTiming, timing);
            let mut session = landed_holding(&catalog, rules, &[200]);
            session
                .accept_mission(crate::catalog::MissionId(140), &mut NeverFires)
                .expect("accepted");
            land_again(&mut session, &catalog);
            assert_eq!(active(&session), [200, 201], "completed: {timing:?}");
            assert_eq!(prices(&mut session), after_landing, "{timing:?}");
            land_again(&mut session, &catalog);
            assert_eq!(prices(&mut session), TWO_RANKS, "{timing:?}");
        }
    }

    #[test]
    fn a_pilot_resumed_docked_has_its_ranks_in_the_prices() {
        let catalog = port(Vec::new());
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(202));
        pilot.stellar = Some(crate::catalog::StellarId(128));
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        let mut session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert!(session.landed().is_some(), "resumed docked");
        assert_eq!(
            prices(&mut session),
            ONE_RANK,
            "202's government is an ally"
        );
        let mut own_only = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(reading(RuleKey::RankGovtReach, RuleSource::Bible));
        assert_eq!(
            prices(&mut own_only),
            UNRANKED,
            "the rules given after the load still choose the reach"
        );
    }

    #[test]
    fn a_relocation_opens_the_port_it_docks_at() {
        let mut catalog = port(Vec::new());
        let mut other = catalog.sites[0].1[0];
        other.id = crate::catalog::StellarId(129);
        other.govt = Some(crate::catalog::GovtId(130));
        catalog.sites[0].1.push(other);
        let mut session = landed_holding(&catalog, Rulebook::default(), &[200]);
        assert_eq!(prices(&mut session), ONE_RANK);
        session
            .relocate(
                &catalog,
                crate::catalog::SystemId(130),
                crate::catalog::StellarId(129),
            )
            .expect("relocates");
        assert_eq!(prices(&mut session), UNRANKED, "government 130 is no ally");
    }

    #[test]
    fn rank_govt_reach_reaches_the_shipyard() {
        let catalog = port(Vec::new());
        for timing in [RuleSource::Engine, RuleSource::Bible] {
            let engine = reading(RuleKey::RankPriceTiming, timing);
            let bible = engine.with_override(RuleKey::RankGovtReach, RuleSource::Bible);
            let mut allied = landed_holding(&catalog, engine, &[202]);
            assert_eq!(prices(&mut allied), ONE_RANK, "{timing:?}");
            let mut own = landed_holding(&catalog, bible, &[202]);
            assert_eq!(prices(&mut own), UNRANKED, "{timing:?}");
            let mut both = landed_holding(&catalog, bible, &[200, 202]);
            assert_eq!(prices(&mut both), ONE_RANK, "{timing:?}");
        }
    }

    #[test]
    fn a_hire_charges_the_ranked_fee() {
        let catalog = port(Vec::new());
        let mut session = landed_holding(&catalog, Rulebook::default(), &[200]);
        let cash = session.pilot().cash();
        let hired = session.hire(ShipId(129), &mut NeverFires).expect("hired");
        assert_eq!(hired.fee, 1570);
        assert_eq!(session.pilot().cash(), cash - 1570);
    }

    #[test]
    fn a_purchase_pays_the_ranked_price_and_takes_the_ranked_trade_in() {
        // Ship 128's hull of 10,000 trades in raw for 2500: 2250, then
        // 2025, rounded down to 2020 by the engine; 2250 by the other
        // reading of rank_trade_in.
        let catalog = port(Vec::new());
        for (trade_in, cash) in [
            (RuleSource::Engine, 25_000 + 2020 - 15_700),
            (RuleSource::Bible, 25_000 + 2250 - 15_700),
        ] {
            let rules = reading(RuleKey::RankTradeIn, trade_in);
            let mut session = landed_holding(&catalog, rules, &[200]);
            session.pilot.cash = 25_000;
            let bought = session
                .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
                .expect("bought");
            assert_eq!(bought.price, 15_700, "{trade_in:?}");
            assert_eq!(session.pilot().cash(), cash, "{trade_in:?}");
        }
    }
}
