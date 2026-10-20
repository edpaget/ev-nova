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
//! **Prices.** As the spaceport opens (a landing, and a relocation that
//! docks; a load opens none, resuming in flight), the
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
//! **Crimes.** A crime the player commits takes the ranks it reaches
//! ([`rank::lose_to_crime`], see [`rank`]'s **Crimes**), in the same call
//! as its conviction and right after it, as `_SlapWithPenalty` walks the
//! penalties first: disabling and breaking up an NPC
//! ([`Session::punish`]), boarding one ([`Session::board`]) and shooting
//! a stellar down ([`Session::shoot_down`], one pass for the ten
//! convictions, as the nine after the first find no matching rank still
//! active). A mission's special ship is no victim. The pass reads the
//! session's [`RankCrimeReach`](crate::rulebook::RuleKey::RankCrimeReach)
//! and [`RankUnaffiliatedGroup`](crate::rulebook::RuleKey::RankUnaffiliatedGroup)
//! rules, makes no save due and plays no sound.
//!
//! **Blessing.** [`Session::blessing`] gives the ranks' blessing table
//! ([`rank::blessing`], see [`rank`]'s **Blessing**), the ranks reaching
//! governments as [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach)
//! says, read when the table is asked. By the engine's reading of
//! [`BlessingRefresh`](crate::rulebook::RuleKey::BlessingRefresh) it is
//! built from the active ranks as the last reset took them, never saved,
//! at each of the original's calls of `_ResetPlayerPrecalcedValues`
//! (see the `precalced` module for every site): game entry, leaving a
//! stellar, a jump's arrival, a landing, the end of every set expression
//! that is not empty, an outfit bought or sold, a ship bought or taken
//! as the player's, a
//! jettison, the plunder dialog closing, a mission cargo pickup or
//! drop-off, an escort's share of the cargo destroyed, the player's
//! eject (@0x6f8bb), the onboard bombs (@0x69a05) and the escape pod's
//! rescue. The same reset caches the sabotage flag and restarts its
//! clock (see [`sabotage`](crate::sabotage)), so every site reaches it
//! too. So a `K` or
//! `L` blesses, or stops blessing, as its set
//! expression ends, and a crime's rank loss (`Session::lose_ranks_to`,
//! `_SlapWithPenalty`'s pass, which reaches no reset) only at the next
//! reset. By the other reading the table is built
//! from the active ranks as they are when it is asked.
//!
//! *Ordering against the original.* The engine runs a take-off's, a gate
//! exit's and an arrival's story subscriptions (the days', the
//! explorations' and the take-off's) when the script next settles, after
//! the reset, where the original ran them before it. Each script they run
//! resets as it ends, in both, so the settling needs no reset of its own,
//! and no frame is drawn between them.
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

use super::{Chart, PlayerShip, Setup, Statics};
use crate::pilot::Pilot;
use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::catalog::{GovtId, RankId, StellarId};
use crate::legal::Crime;
use crate::news::MISC_STRINGS;
use crate::person::NO_RANK;
use crate::rank::{
    self, AND_STRING, Blessing, Held, NO_RANK_STRING, PilotInfoRanks, PriceMults, RankNames,
};
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
    /// Takes what the price multipliers read as the spaceport opens
    /// (`_DoPortDialog` @0x5f970-0x5fa04): the active ranks and the docked
    /// stellar's government.
    pub(super) fn open_port_prices(&mut self) {
        self.port_ranks = PortRanks {
            active: self.pilot.ranks.clone(),
            govt: self.chart.docked_govt(self.landed),
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
        let reach = self.setup.rules.source_for(RuleKey::RankGovtReach);
        let (active, govt) = match self.setup.rules.source_for(RuleKey::RankPriceTiming) {
            RuleSource::Engine => (&self.port_ranks.active, self.port_ranks.govt),
            RuleSource::Bible => (&self.pilot.ranks, self.chart.docked_govt(self.landed)),
        };
        rank::price_mults(
            &self.statics.ranks,
            active,
            govt,
            &self.statics.govts,
            reach,
        )
    }

    /// The ranks' factor the outfitter's buy price takes, as the
    /// session's `rank_price_scope` rule reads: none (1.0) by the engine,
    /// as the original's outfitter never reads `_priceFluxMult`; by the
    /// other reading, `[1]`, as the shipyard's ship price takes it.
    pub(super) fn outfit_price_mult(&self) -> f32 {
        match self.setup.rules.source_for(RuleKey::RankPriceScope) {
            RuleSource::Engine => 1.0,
            RuleSource::Bible => self.price_mults().second,
        }
    }

    /// The ranks' blessing table (see [`rank::blessing`]), the ranks
    /// reaching governments as the session's `rank_govt_reach` rule reads.
    /// By the engine's reading of `blessing_refresh` it is built from the
    /// ranks the last reset took; by the other, from the active ranks
    /// now.
    #[must_use]
    pub fn blessing(&self) -> Blessing {
        self.ship.blessing(&self.pilot, &self.statics, &self.setup)
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
        let unaffiliated = self.setup.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.statics.ranks);
        rank::activate(&records, self.held_ranks(), rank, unaffiliated);
    }

    /// `L`: deactivates rank `rank` (see [`rank::deactivate`]),
    /// unaffiliated ranks grouped by the session's
    /// `rank_unaffiliated_group` rule.
    pub(super) fn deactivate_rank(&mut self, rank: RankId) {
        let unaffiliated = self.setup.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        let records = std::rc::Rc::clone(&self.statics.ranks);
        rank::deactivate(&records, self.held_ranks(), rank, unaffiliated);
    }

    /// `_SlapWithPenalty`'s rank pass (see [`rank::lose_to_crime`]) for
    /// `crime` the player commits against a ship of `victim`, reaching
    /// the ranks by the session's `rank_crime_reach` rule and sweeping
    /// them by its `rank_unaffiliated_group` rule. The callers skip a
    /// mission's special ship, as the original passes its mark
    /// (@0x9c10). No save is due, as a conviction makes none in flight,
    /// and no sound plays.
    pub(super) fn lose_ranks_to(&mut self, crime: Crime, victim: Option<GovtId>) {
        let reach = self.setup.rules.source_for(RuleKey::RankCrimeReach);
        let unaffiliated = self.setup.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        rank::lose_to_crime(
            &self.statics.ranks,
            Held {
                active: &mut self.pilot.ranks,
                recent: &mut self.recent_rank,
            },
            crime,
            victim,
            &self.statics.govts,
            reach,
            unaffiliated,
        );
    }

    /// What the rank tags read: the pilot's active ranks, the recent
    /// rank, and `STR#` 2002 #341 from the session's string lists as the
    /// fallback, [`NO_RANK`] when the lists lack it (none are wired by
    /// default).
    pub(super) fn rank_names(&self) -> RankNames<'_> {
        RankNames {
            records: &self.statics.ranks,
            active: &self.pilot.ranks,
            recent: self.recent_rank,
            fallback: self
                .setup
                .nth_string(MISC_STRINGS, NO_RANK_STRING)
                .map_or(Cow::Borrowed(NO_RANK), Cow::Owned),
        }
    }

    /// The pilot info's ranks line and the salaries' share of the daily
    /// income (see [`rank::pilot_info`]), against the pilot's cash and the
    /// session's `salary_cap` rule, joined with `STR#` 2002 #392 (nothing
    /// when the lists lack it, as `_StrcatIndCString` appends nothing),
    /// the player info's special outfits following the ranks
    /// ([`player_info::special_groups`](crate::player_info::special_groups)).
    #[must_use]
    pub fn pilot_info_ranks(&self) -> PilotInfoRanks {
        let and = self
            .setup
            .nth_string(MISC_STRINGS, AND_STRING)
            .unwrap_or_default();
        rank::pilot_info(
            &self.statics.ranks,
            &self.pilot.ranks,
            self.pilot.cash,
            self.setup.rules.source_for(RuleKey::SalaryCap),
            &and,
            crate::player_info::special_groups(&self.statics.outfits, &self.pilot.outfits).len(),
        )
    }

    /// The day's eighth participant: the active ranks' salaries (see
    /// [`rank::pay_salaries`]), capped by the session's `salary_cap`
    /// rule. A change to the cash makes a save due.
    pub(super) fn ranks_day(&mut self) {
        let cap = self.setup.rules.source_for(RuleKey::SalaryCap);
        let before = self.pilot.cash;
        rank::pay_salaries(
            &self.statics.ranks,
            &self.pilot.ranks,
            &mut self.pilot.cash,
            cap,
        );
        if self.pilot.cash != before {
            self.out.save_due = true;
        }
    }
}

impl PlayerShip {
    /// The ranks' blessing table (see [`rank::blessing`]), the ranks
    /// reaching governments as the session's `rank_govt_reach` rule reads.
    /// By the engine's reading of `blessing_refresh` it is built from the
    /// ranks the last reset took; by the other, from the active ranks
    /// now.
    #[must_use]
    pub(super) fn blessing(&self, pilot: &Pilot, statics: &Statics, setup: &Setup) -> Blessing {
        let reach = setup.rules.source_for(RuleKey::RankGovtReach);
        let active = match setup.rules.source_for(RuleKey::BlessingRefresh) {
            RuleSource::Engine => &self.latches.blessing_ranks,
            RuleSource::Bible => &pilot.ranks,
        };
        rank::blessing(&statics.ranks, active, &statics.govts, reach)
    }
}

impl Chart {
    /// The docked stellar's government now: none in flight or at an
    /// independent stellar.
    fn docked_govt(&self, landed: Option<StellarId>) -> Option<GovtId> {
        let landed = landed?;
        self.sites.iter().find(|site| site.id == landed)?.govt
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::catalog::{GovtId, OutfitId, OutfitRecord, RankId, RankRecord, ShipId, ShipRecord};
    use crate::chance::NeverFires;
    use crate::control::SetExpr;
    use crate::outfitter::OutfitFlags;
    use crate::pilot::Pilot;
    use crate::rank::{BLESSES_LANDING, Blessed, SWEEP_GOVT_ON_ACTIVATE, SWEEP_GOVT_ON_DEACTIVATE};
    use crate::rulebook::{RuleKey, RuleSource, Rulebook};
    use crate::save;
    use crate::session::Session;
    use crate::session::missions::TextSource;
    use crate::shipyard::ShipFlags3;
    use crate::testkit::{FakePilotCatalog, Scripted, catalog, jump, land_now, outfit, rank, ship};

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
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x0100_0000_0004,
            0
        );
        session.run_set(&set("K200"), &mut Scripted::default());
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x0100_0000_0004,
            0x0100_0000_0004
        );
        session.run_set(&set("L200"), &mut Scripted::default());
        assert_eq!(
            session.ship.contributed(&session.pilot, &session.statics) & 0x0100_0000_0004,
            0
        );
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
            affil_govt: Some(GovtId(128)),
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
            "Cmdr Cdr Commander <PRK128>",
            "no <PRKnnn> government remembered yet"
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
        let mut session = session;
        let loaded = session.take_in(TextSource::Desc, "<PRK128>!", &mut NeverFires);
        assert_eq!(
            session.plain_text(&loaded),
            "skipper!",
            "the <SRKnnn> government, none, reads the fallback"
        );
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
        let site = session.chart.sites[0];
        session.ship.player.position = site.position;
        session.ship.player.velocity = crate::geometry::Vec2::ZERO;
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
    fn a_pilot_resumed_takes_its_ranks_into_the_prices_at_its_first_landing() {
        let catalog = port(Vec::new());
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(202));
        pilot.stellar = Some(crate::catalog::StellarId(128));
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        let mut session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert_eq!(
            session.port_ranks,
            super::PortRanks::default(),
            "no spaceport opens on resuming"
        );
        land_now(&mut session).expect("lands");
        assert_eq!(
            prices(&mut session),
            ONE_RANK,
            "202's government is an ally"
        );
        let mut own_only = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(reading(RuleKey::RankGovtReach, RuleSource::Bible));
        land_now(&mut own_only).expect("lands");
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

    #[test]
    fn an_outfit_bought_under_the_other_scope_pays_the_rank_price() {
        // Speed booster 300 costs 1000; 900 under one rank of 90.
        let catalog = port(Vec::new());
        let order = crate::OutfitOrder {
            outfit: OutfitId(300),
            direction: crate::Direction::Buy,
        };
        for (scope, paid) in [(RuleSource::Engine, 1000), (RuleSource::Bible, 900)] {
            let rules = reading(RuleKey::RankPriceScope, scope);
            let mut session = landed_holding(&catalog, rules, &[200]);
            let cash = session.pilot().cash();
            let shop = session.outfitter(&mut NeverFires).expect("an outfitter");
            assert_eq!(
                shop.row(OutfitId(300))
                    .map(|row| (row.price, row.buy_price)),
                Some((1000, paid)),
                "{scope:?}"
            );
            session.outfit(order, &mut NeverFires).expect("bought");
            assert_eq!(cash - session.pilot().cash(), paid, "{scope:?}");
        }
    }

    #[test]
    fn the_scope_key_changes_nothing_on_the_ship_side() {
        // Ship 128's hull trades in at 2020 under the default rank_trade_in.
        let catalog = port(Vec::new());
        for scope in [RuleSource::Engine, RuleSource::Bible] {
            let rules = reading(RuleKey::RankPriceScope, scope);
            let mut session = landed_holding(&catalog, rules, &[200]);
            assert_eq!(prices(&mut session), ONE_RANK, "{scope:?}");
            session.pilot.cash = 25_000;
            let bought = session
                .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
                .expect("bought");
            assert_eq!(bought.price, ONE_RANK.0, "{scope:?}");
            assert_eq!(
                session.pilot().cash(),
                25_000 + 2020 - ONE_RANK.0,
                "{scope:?}"
            );
            let mut hiring = landed_holding(&catalog, rules, &[200]);
            let cash = hiring.pilot().cash();
            let hired = hiring.hire(ShipId(129), &mut NeverFires).expect("hired");
            assert_eq!(hired.fee, ONE_RANK.1, "{scope:?}");
            assert_eq!(cash - hiring.pilot().cash(), ONE_RANK.1, "{scope:?}");
        }
    }

    #[test]
    fn a_government_change_while_landed_follows_the_timing_rule() {
        let catalog = port(Vec::new());
        for (timing, while_landed) in [
            (RuleSource::Engine, ONE_RANK),
            (RuleSource::Bible, UNRANKED),
        ] {
            let rules = reading(RuleKey::RankPriceTiming, timing);
            let mut session = landed_holding(&catalog, rules, &[200]);
            assert_eq!(prices(&mut session), ONE_RANK, "{timing:?}");
            // Government 130 is no ally of the rank's government 128.
            session.chart.sites[0].govt = Some(crate::catalog::GovtId(130));
            session.chart.listed[0].govt = Some(crate::catalog::GovtId(130));
            assert_eq!(prices(&mut session), while_landed, "{timing:?}");
            land_again(&mut session, &catalog);
            assert_eq!(prices(&mut session), UNRANKED, "{timing:?}");
        }
    }

    // Blessing.

    /// Rank 200 of government 140, blessing landing.
    pub(in crate::session) fn blessing_rank() -> RankRecord {
        RankRecord {
            affil_govt: Some(GovtId(140)),
            flags: BLESSES_LANDING,
            ..rank(200)
        }
    }

    /// Whether `session`'s blessing has government 140 bless landing.
    pub(in crate::session) fn blessed(session: &Session) -> bool {
        session
            .blessing()
            .blesses(Some(GovtId(140)), Blessed::Landing)
    }

    /// [`catalog`] with [`blessing_rank`].
    fn blessing_catalog() -> FakePilotCatalog {
        FakePilotCatalog {
            ranks: vec![blessing_rank()],
            ..catalog()
        }
    }

    /// Runs `text` as a set expression.
    fn run(session: &mut Session, text: &str) {
        session.run_set(&set(text), &mut Scripted::default());
    }

    /// The default rule set with `blessing_refresh` read live.
    fn live() -> Rulebook {
        reading(RuleKey::BlessingRefresh, RuleSource::Bible)
    }

    /// Makes rank 200 active outside any reset, as no set expression can,
    /// each resetting as it ends.
    fn activate_unreset(session: &mut Session) {
        session.pilot.ranks.insert(RankId(200));
    }

    /// Runs `text` through the nested runner, and gives whether rank 200
    /// blessed after each operator it handed on, inside the expression.
    fn blessings_inside(session: &mut Session, text: &str) -> Vec<bool> {
        let mut inside = Vec::new();
        session.run_ops(&set(text), &mut Scripted::default(), &mut |session, _| {
            inside.push(blessed(session));
        });
        inside
    }

    #[test]
    fn by_the_engine_a_k_blesses_as_its_set_expression_ends() {
        let mut session = Session::start(&blessing_catalog()).expect("starts");
        let inside = blessings_inside(&mut session, "K200 K201");
        assert_eq!(inside, [false, false], "not between the operators");
        assert!(blessed(&session), "@0x99a85");
    }

    #[test]
    fn by_the_other_reading_a_k_blesses_at_once() {
        let mut session = Session::start(&blessing_catalog())
            .expect("starts")
            .with_rules(live());
        let inside = blessings_inside(&mut session, "K200 K201");
        assert_eq!(inside, [true, true], "within the expression too");
        assert!(blessed(&session));
        run(&mut session, "L200");
        assert!(!blessed(&session));
    }

    #[test]
    fn by_the_engine_an_l_stops_blessing_as_its_set_expression_ends() {
        let catalog = blessing_catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(200));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(blessed(&session), "flying resets (@0x19ffb)");
        let inside = blessings_inside(&mut session, "L200 L201");
        assert_eq!(inside, [true, true]);
        assert!(!blessed(&session));
    }

    #[test]
    fn a_rank_lost_to_a_crime_still_blesses_until_the_next_reset_by_the_engine() {
        // `_SlapWithPenalty` reaches no reset, so the two readings differ
        // from the crime until the next one.
        use crate::rank::LOST_TO_CRIME;
        let catalog = FakePilotCatalog {
            ranks: vec![RankRecord {
                flags: BLESSES_LANDING | LOST_TO_CRIME,
                ..blessing_rank()
            }],
            ..catalog()
        };
        for (source, after_crime) in [(RuleSource::Engine, true), (RuleSource::Bible, false)] {
            let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
            pilot.ranks.insert(RankId(200));
            let mut session = Session::fly(&catalog, pilot)
                .expect("flies")
                .with_rules(reading(RuleKey::BlessingRefresh, source));
            session.lose_ranks_to(crate::legal::Crime::Board, Some(GovtId(140)));
            assert_eq!(active(&session), [] as [i16; 0], "{source:?}");
            assert_eq!(blessed(&session), after_crime, "{source:?}");
            session.jettison_cargo(false, &mut NeverFires);
            assert!(!blessed(&session), "{source:?}: @0x3e3b5");
        }
    }

    /// Whether the player as the session's NPCs see it carries government
    /// 140's blessing.
    fn npcs_see_blessed(session: &Session) -> bool {
        session
            .player_side()
            .blessing
            .blesses(Some(GovtId(140)), Blessed::Landing)
    }

    #[test]
    fn the_player_side_carries_the_blessing_the_last_reset_took() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        activate_unreset(&mut session);
        assert!(!npcs_see_blessed(&session), "taken at the last reset");
        session.reset_precalced();
        assert!(npcs_see_blessed(&session));
    }

    #[test]
    fn the_player_side_carries_the_live_blessing_by_the_other_reading() {
        let mut session = Session::start(&blessing_catalog())
            .expect("starts")
            .with_rules(live());
        run(&mut session, "K200");
        assert!(npcs_see_blessed(&session));
    }

    #[test]
    fn the_blessing_reads_the_sessions_rank_govt_reach() {
        let mut catalog = blessing_catalog();
        catalog.govts = price_govts()
            .into_iter()
            .map(|record| crate::catalog::GovtRecord {
                id: GovtId(record.id.0 + 12),
                ..record
            })
            .collect();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(200));
        let ally = |session: &Session| {
            session
                .blessing()
                .blesses(Some(GovtId(141)), Blessed::Landing)
        };
        let session = Session::fly(&catalog, pilot.clone()).expect("flies");
        assert!(ally(&session), "141 is 140's ally");
        let own = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(reading(RuleKey::RankGovtReach, RuleSource::Bible));
        assert!(!ally(&own));
        assert!(blessed(&own));
    }

    #[test]
    fn a_take_off_refreshes_the_blessing() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        activate_unreset(&mut session);
        assert!(!blessed(&session), "made active outside any reset");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(blessed(&session), "@0x6709b");
    }

    #[test]
    fn a_jump_arrival_refreshes_the_blessing() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        activate_unreset(&mut session);
        assert!(!blessed(&session));
        jump(&mut session, &catalog, 131).expect("arrives");
        assert!(blessed(&session), "@0x6c51f");
    }

    #[test]
    fn a_gate_exit_refreshes_the_blessing() {
        use crate::gate::GateKind;
        use crate::session::gate_follow::tests::{exit_gate, following, gated};
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let mut catalog = gated(following(0, 0), 0);
            catalog.ranks = vec![blessing_rank()];
            let mut session = Session::start(&catalog).expect("starts");
            activate_unreset(&mut session);
            assert!(!blessed(&session), "{kind:?}");
            exit_gate(&mut session, &catalog, kind, &mut NeverFires);
            assert!(blessed(&session), "{kind:?}");
        }
    }

    #[test]
    fn the_settling_after_an_entry_takes_no_second_blessing() {
        // Each script the entry's story work runs resets as it ends, so
        // the settling after it needs no reset of its own: a rank made
        // active outside any reset is not taken.
        use crate::gate::GateKind;
        use crate::session::gate_follow::tests::{following, gated, pass_gate};
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        session.pilot.ranks.insert(RankId(200));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(!blessed(&session), "after a take-off");
        let mut session = Session::start(&catalog).expect("starts");
        jump(&mut session, &catalog, 131).expect("arrives");
        session.pilot.ranks.insert(RankId(200));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(!blessed(&session), "after an arrival");
        for kind in [GateKind::Hypergate, GateKind::Wormhole] {
            let mut catalog = gated(following(0, 0), 0);
            catalog.ranks = vec![blessing_rank()];
            let mut session = Session::start(&catalog).expect("starts");
            pass_gate(&mut session, &catalog, kind, &mut NeverFires);
            session.pilot.ranks.insert(RankId(200));
            let _ = session.settle_script(&catalog, &mut NeverFires);
            assert!(!blessed(&session), "{kind:?}");
        }
    }

    #[test]
    fn flying_a_pilot_refreshes_the_blessing_and_begin_counts_on_starts_rank() {
        let catalog = blessing_catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(200));
        let pilot = save::decode(&save::encode(&pilot)).expect("loads");
        assert!(blessed(&Session::fly(&catalog, pilot).expect("flies")));
        let starting = FakePilotCatalog {
            character: Ok(crate::catalog::CharacterStart {
                on_start: crate::control::Script::parse("K200"),
                ..catalog.character.clone().expect("a chär")
            }),
            ..blessing_catalog()
        };
        let mut session = Session::start(&starting).expect("starts");
        assert!(!blessed(&session));
        session.begin(&starting, &mut Scripted::default());
        assert!(blessed(&session), "OnStart runs before @0x19ffb");
    }

    #[test]
    fn closing_the_plunder_dialog_refreshes_the_blessing() {
        use crate::board::{Take, Taken};
        let mut catalog = crate::session::tests::boardable();
        catalog.ranks = vec![blessing_rank()];
        let mut session = crate::session::tests::aboard(&catalog);
        activate_unreset(&mut session);
        assert!(!blessed(&session));
        assert_eq!(
            crate::session::tests::take(&mut session, Take::Abort, &[]).0,
            Taken::Aborted
        );
        assert!(blessed(&session), "@0x94a0b");
    }

    #[test]
    fn every_other_exit_of_the_plunder_dialog_refreshes_the_blessing() {
        use crate::board::{Assigned, Assignment, Take, Taken};
        use crate::session::tests::{aboard, boardable, take};
        use crate::testkit::Draws;
        /// An exit's name, and how the test takes it.
        type Exit = (&'static str, fn(&mut Session));
        let mut catalog = boardable();
        catalog.ranks = vec![blessing_rank()];
        let exits: [Exit; 4] = [
            ("a failed capture", |session| {
                assert_eq!(take(session, Take::Capture, &[44]).0, Taken::CaptureFailed);
            }),
            ("a tripped self-destruct", |session| {
                assert_eq!(take(session, Take::Capture, &[43, 0]).0, Taken::Tripped);
            }),
            ("a capture straight into the fleet", |session| {
                for record in &mut session.statics_mut().ships {
                    if record.id == ShipId(128) {
                        record.crew = 0;
                    }
                }
                assert_eq!(take(session, Take::Capture, &[43, 1]).0, Taken::Escorted);
            }),
            ("an assignment", |session| {
                assert_eq!(take(session, Take::Capture, &[43, 1]).0, Taken::Captured);
                assert!(!blessed(session), "a capture awaiting assignment");
                assert_eq!(
                    session.assign(Assignment::Escort, &mut Draws::of(&[])),
                    Some(Assigned::Escort)
                );
            }),
        ];
        for (exit, close) in exits {
            let mut session = aboard(&catalog);
            activate_unreset(&mut session);
            assert!(!blessed(&session), "{exit}");
            close(&mut session);
            assert_eq!(session.boarding(), None, "{exit}");
            assert!(blessed(&session), "{exit}: @0x94a0b");
        }
    }

    // Landing.

    use crate::landing::{LandOutcome, LandingRefusal};
    use crate::session::LandPress;

    /// [`blessing_catalog`] whose planet 128 is of government 140,
    /// requiring `require`, with `MinStatus` 32767 and these `Flags2`.
    fn guarded(require: u64, flags2: u16) -> FakePilotCatalog {
        let mut catalog = blessing_catalog();
        catalog.govts = vec![crate::catalog::GovtRecord {
            require,
            ..crate::testkit::govt(140)
        }];
        for (_, sites) in &mut catalog.sites {
            for site in sites.iter_mut().filter(|site| site.id.0 == 128) {
                site.govt = Some(GovtId(140));
                site.min_status = 32767;
                site.flags2 = flags2;
            }
        }
        catalog
    }

    /// A new pilot of `catalog` under `rules`, holding `ranks`, flown.
    fn holding(catalog: &FakePilotCatalog, rules: Rulebook, ranks: &[i16]) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.ranks.extend(ranks.iter().copied().map(RankId));
        Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(rules)
    }

    /// L's selection and request of planet 128, cleared.
    const REQUESTED: Result<LandPress, LandingRefusal> =
        Ok(LandPress::Outcome(LandOutcome::Requested {
            stellar: crate::catalog::StellarId(128),
            station: false,
        }));

    const LANDED: Result<LandPress, LandingRefusal> = Ok(LandPress::Outcome(LandOutcome::Landed(
        crate::catalog::StellarId(128),
    )));

    const DENIED: Result<LandPress, LandingRefusal> = Err(LandingRefusal::Denied {
        stellar: crate::catalog::StellarId(128),
        station: false,
        min_status: 32767,
    });

    /// L over planet 128, a tick for the dock approach, and L again.
    fn press_twice(session: &mut Session) -> [Result<LandPress, LandingRefusal>; 2] {
        let first = session.land();
        session.tick(crate::flight::Controls::default());
        [first, session.land()]
    }

    #[test]
    fn a_pilot_blessed_with_landing_lands_where_its_record_is_denied() {
        let catalog = guarded(0, 0);
        let mut unblessed = holding(&catalog, Rulebook::default(), &[]);
        assert_eq!(
            press_twice(&mut unblessed),
            [DENIED, DENIED],
            "MinStatus 32767"
        );
        for source in RuleSource::ALL {
            let rules = reading(RuleKey::BlessingLanding, source);
            let mut session = holding(&catalog, rules, &[200]);
            assert_eq!(press_twice(&mut session), [REQUESTED, LANDED], "{source:?}");
        }
    }

    #[test]
    fn the_blessing_clears_the_approach_with_no_require_test_by_either_reading() {
        let catalog = guarded(0x4, 0);
        for source in RuleSource::ALL {
            let rules = reading(RuleKey::BlessingLanding, source);
            for (ranks, greets) in [(&[][..], false), (&[200][..], true)] {
                let mut session = holding(&catalog, rules, ranks);
                session.dock = Some(crate::clearance::DockRequest {
                    stellar: crate::catalog::StellarId(128),
                    permission: crate::clearance::REQUESTED_NEAR,
                });
                session.take_messages();
                session.tick(crate::flight::Controls::default());
                assert_eq!(
                    session
                        .take_messages()
                        .contains(&crate::message::SimMessage::DockCleared {
                            stellar: crate::catalog::StellarId(128),
                            kind: crate::clearance::DockKind::Planet,
                        }),
                    greets,
                    "{source:?} {ranks:?}"
                );
            }
        }
    }

    #[test]
    fn by_the_engine_the_blessing_is_requested_greeted_and_lands_past_require() {
        let catalog = guarded(0x4, 0);
        let mut engine = holding(&catalog, Rulebook::default(), &[200]);
        assert_eq!(engine.land(), REQUESTED);
        engine.take_messages();
        engine.tick(crate::flight::Controls::default());
        assert_eq!(
            engine.take_messages(),
            [crate::message::SimMessage::DockCleared {
                stellar: crate::catalog::StellarId(128),
                kind: crate::clearance::DockKind::Planet,
            }]
        );
        assert_eq!(engine.land(), LANDED);
    }

    #[test]
    fn by_the_bible_the_blessing_does_not_get_past_require() {
        let catalog = guarded(0x4, 0);
        let mut engine = holding(&catalog, Rulebook::default(), &[200]);
        assert_eq!(press_twice(&mut engine), [REQUESTED, LANDED], "@0x679cd");
        let rules = reading(RuleKey::BlessingLanding, RuleSource::Bible);
        let mut bible = holding(&catalog, rules, &[200]);
        assert_eq!(press_twice(&mut bible), [DENIED, DENIED]);
    }

    #[test]
    fn a_contributed_require_lets_the_pilot_land() {
        let mut catalog = guarded(0x4, 0);
        catalog.ranks.push(RankRecord {
            contribute: 0x0100_0000_0004,
            ..rank(201)
        });
        for site in &mut catalog.sites[0].1 {
            site.min_status = 0;
        }
        let mut lacking = holding(&catalog, Rulebook::default(), &[]);
        assert_eq!(
            press_twice(&mut lacking),
            [Err(LandingRefusal::Denied {
                stellar: crate::catalog::StellarId(128),
                station: false,
                min_status: 0,
            }); 2],
            "the record passes, Require does not (@0x67940)"
        );
        let mut contributing = holding(&catalog, Rulebook::default(), &[201]);
        assert_eq!(press_twice(&mut contributing), [REQUESTED, LANDED]);
    }

    /// A new pilot under [`guarded`]`(0x4, 0)` who took mission 140, of
    /// these `TravelStel` and `ReturnStel`, at planet 129, its slot then
    /// failed as `failed` says, flown back over planet 128.
    fn on_mission(
        travel_stel: i16,
        return_stel: i16,
        failed: Option<crate::mission::FailedBy>,
    ) -> Session {
        let mut catalog = guarded(0x4, 0);
        catalog.missions = vec![crate::catalog::MissionRecord {
            avail_stel: 129,
            travel_stel,
            return_stel,
            ..crate::testkit::mission(140)
        }];
        let mut session = holding(&catalog, Rulebook::default(), &[]);
        let at = |session: &mut Session, stellar: usize| {
            let site = session.chart.sites[stellar];
            session.ship.player.position = site.position;
            session.ship.player.velocity = crate::geometry::Vec2::ZERO;
            session.chart.nav = crate::navigation::NavMode::Off;
        };
        at(&mut session, 1);
        land_now(&mut session).expect("lands at 129");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        session
            .accept_mission(crate::catalog::MissionId(140), &mut NeverFires)
            .expect("accepted");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        let missions = &mut session.pilot.missions;
        for slot in missions.holding(crate::catalog::MissionId(140)) {
            missions.get_mut(slot).expect("held").failed = failed;
        }
        at(&mut session, 0);
        session
    }

    /// The travel and return stellars of `session`'s missions.
    fn stops(session: &Session) -> Vec<[Option<i16>; 2]> {
        session
            .pilot()
            .missions()
            .iter()
            .map(|(_, active)| {
                [active.destination, active.return_to].map(|stop| stop.map(|stop| stop.stellar.0))
            })
            .collect()
    }

    #[test]
    fn a_pilot_is_cleared_at_its_missions_stellar_whatever_its_record() {
        for (travel, back, why) in [
            (128, 129, "travel stellar, @0x679aa"),
            (129, 128, "return stellar, @0x679b0"),
        ] {
            let mut session = on_mission(travel, back, None);
            assert_eq!(stops(&session), [[Some(travel), Some(back)]], "{why}");
            assert_eq!(press_twice(&mut session), [REQUESTED, LANDED], "{why}");
        }
    }

    #[test]
    fn a_failed_missions_stellar_still_clears_the_pilot() {
        use crate::mission::FailedBy;
        for failed in [FailedBy::Quick, FailedBy::Marked] {
            for (travel, back) in [(128, 129), (129, 128)] {
                let mut session = on_mission(travel, back, Some(failed));
                assert_eq!(stops(&session), [[Some(travel), Some(back)]]);
                assert_eq!(
                    press_twice(&mut session),
                    [REQUESTED, LANDED],
                    "{failed:?} ({travel}, {back}): no failure test in @0x67996-0x679c5"
                );
            }
        }
    }

    #[test]
    fn a_mission_elsewhere_does_not_clear_the_pilot() {
        let mut session = on_mission(129, 129, None);
        assert_eq!(stops(&session), [[Some(129), Some(129)]]);
        assert_eq!(press_twice(&mut session), [DENIED, DENIED]);
    }

    #[test]
    fn a_dominated_stellar_clears_the_pilot() {
        let catalog = guarded(0, crate::market::DOMINATED);
        let mut session = holding(&catalog, Rulebook::default(), &[]);
        assert_eq!(press_twice(&mut session), [REQUESTED, LANDED], "@0x6792d");
    }

    #[test]
    fn by_the_engine_a_rank_made_active_opens_its_stellars_only_from_the_next_reset() {
        let catalog = guarded(0, 0);
        let mut engine = holding(&catalog, Rulebook::default(), &[]);
        activate_unreset(&mut engine);
        assert_eq!(press_twice(&mut engine), [DENIED, DENIED]);
        engine.jettison_cargo(false, &mut NeverFires);
        assert_eq!(press_twice(&mut engine), [REQUESTED, LANDED]);
        let mut live = holding(&catalog, live(), &[]);
        activate_unreset(&mut live);
        assert_eq!(press_twice(&mut live), [REQUESTED, LANDED]);
        let mut scripted = holding(&catalog, Rulebook::default(), &[]);
        run(&mut scripted, "K200");
        assert_eq!(
            press_twice(&mut scripted),
            [REQUESTED, LANDED],
            "a K in space is taken as its expression ends"
        );
    }
}
