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
//! built from the active ranks as the last refresh took them, never
//! saved, at the original's calls of `_ResetPlayerPrecalcedValues`:
//!
//! - game entry (`_EnterGameFromMainScreen` @0x19ffb): [`Session::fly`],
//!   for a load or a new pilot, and [`Session::begin`] after the first
//!   `chär`'s `OnStart`, which the original runs before it (@0x1925e);
//! - leaving a stellar (`_HandlePlayerDockRequest` @0x6709b): a take-off
//!   after the stay's days and before the setup, and a gate exit before
//!   the exit's setup;
//! - a jump's arrival, after the days, the enter-system hook and the
//!   setup (`_HandlePlayer` @0x6c51f);
//! - a jettison (`_JettisonCargo` @0x3e3b5, and the jettison key's
//!   @0x6fdaf after it);
//! - the plunder dialog closing (`_DoPlunderDialog` @0x94a0b): an abort,
//!   a failed capture, a self-destruct, a capture into the fleet and an
//!   assignment, but not a boarding the system's change or setup ends;
//! - a mission cargo pickup that fits (`_DoMissionCargoPickup` @0x98316),
//!   whether a boarding or a landing makes it.
//!
//! *Ordering against the original.* The engine runs a take-off's, a gate
//! exit's and an arrival's story subscriptions (the days', the
//! explorations' and the take-off's) when the script next settles, after
//! the refresh, where the original ran them before it. So each of those
//! three also makes the refresh due again, and the first idle settling
//! after it refreshes once more, before any frame is drawn. The hooks the
//! original runs after its refresh never `K`: the enter-system hook,
//! `_MissionHandlePlayerEnteredNewSystem`, only redraws mission-ship
//! timers (@0x99f9a, @0x99fe0).
//!
//! The landing cargo refreshes (`_DoMissionCargoPickup` reached from a
//! landing @0x9ee32, and `_MissionLandCargoCheck` @0x9eff9 after a
//! `DumpCargoText`) need nothing of their own: while docked nothing reads
//! the blessing, and the take-off refreshes before any reader. The
//! pickup's refresh runs from a landing too, as boarding shares it. The
//! onboard bomb's refresh (@0x69a05) is for whoever builds the bomb, and
//! the debug keys' (@0x6dc6f, @0x6f08a) are left out with the keys. By
//! the other reading the table is built from the active ranks as they
//! are when it is asked.
//!
//! The pilot keeps its active ranks by ID. A pilot flown from a save
//! naming a rank the data no longer has forgets it, so the next save
//! leaves it out, as the original's `_LoadPilotData` drops an active flag
//! whose `ränk` is gone (@0x75e47-0x75eb6).

use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::catalog::{GovtId, RankId};
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

    /// The ranks' factor the outfitter's buy price takes, as the
    /// session's `rank_price_scope` rule reads: none (1.0) by the engine,
    /// as the original's outfitter never reads `_priceFluxMult`; by the
    /// other reading, `[1]`, as the shipyard's ship price takes it.
    pub(super) fn outfit_price_mult(&self) -> f32 {
        match self.rules.source_for(RuleKey::RankPriceScope) {
            RuleSource::Engine => 1.0,
            RuleSource::Bible => self.price_mults().second,
        }
    }

    /// Takes the active ranks the blessing table is built from, as
    /// `_ResetPlayerPrecalcedValues` builds it (see the module docs'
    /// **Blessing**).
    pub(super) fn refresh_blessing(&mut self) {
        self.blessing_ranks.clone_from(&self.pilot.ranks);
    }

    /// The refresh of a take-off, a gate exit or a jump's arrival: now,
    /// and again at the next idle settling (see the module docs'
    /// **Blessing**).
    pub(super) fn refresh_blessing_at_entry(&mut self) {
        self.refresh_blessing();
        self.blessing_due = true;
    }

    /// The second refresh of a take-off, a gate exit or a jump's arrival,
    /// at the first idle settling after it (see the module docs'
    /// **Blessing**).
    pub(super) fn settle_blessing(&mut self) {
        if std::mem::take(&mut self.blessing_due) {
            self.refresh_blessing();
        }
    }

    /// The ranks' blessing table (see [`rank::blessing`]), the ranks
    /// reaching governments as the session's `rank_govt_reach` rule reads.
    /// By the engine's reading of `blessing_refresh` it is built from the
    /// ranks the last refresh took; by the other, from the active ranks
    /// now.
    #[must_use]
    pub fn blessing(&self) -> Blessing {
        let reach = self.rules.source_for(RuleKey::RankGovtReach);
        let active = match self.rules.source_for(RuleKey::BlessingRefresh) {
            RuleSource::Engine => &self.blessing_ranks,
            RuleSource::Bible => &self.pilot.ranks,
        };
        rank::blessing(&self.ranks, active, &self.govts, reach)
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

    /// `_SlapWithPenalty`'s rank pass (see [`rank::lose_to_crime`]) for
    /// `crime` the player commits against a ship of `victim`, reaching
    /// the ranks by the session's `rank_crime_reach` rule and sweeping
    /// them by its `rank_unaffiliated_group` rule. The callers skip a
    /// mission's special ship, as the original passes its mark
    /// (@0x9c10). No save is due, as a conviction makes none in flight,
    /// and no sound plays.
    pub(super) fn lose_ranks_to(&mut self, crime: Crime, victim: Option<GovtId>) {
        let reach = self.rules.source_for(RuleKey::RankCrimeReach);
        let unaffiliated = self.rules.source_for(RuleKey::RankUnaffiliatedGroup);
        rank::lose_to_crime(
            &self.ranks,
            Held {
                active: &mut self.pilot.ranks,
                recent: &mut self.recent_rank,
            },
            crime,
            victim,
            &self.govts,
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
            session.sites[0].govt = Some(crate::catalog::GovtId(130));
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

    #[test]
    fn by_the_engine_a_rank_granted_in_space_blesses_only_from_the_next_refresh() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        assert!(!blessed(&session));
        run(&mut session, "K200");
        assert!(!blessed(&session), "taken at the last refresh");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(!blessed(&session), "a settling is no refresh");
        session.jettison_cargo(false, &mut NeverFires);
        assert!(blessed(&session), "@0x3e3b5");
    }

    #[test]
    fn by_the_other_reading_a_rank_granted_in_space_blesses_at_once() {
        let mut session = Session::start(&blessing_catalog())
            .expect("starts")
            .with_rules(live());
        run(&mut session, "K200");
        assert!(blessed(&session));
        run(&mut session, "L200");
        assert!(!blessed(&session));
    }

    #[test]
    fn by_the_engine_a_rank_lost_in_space_still_blesses_until_the_next_refresh() {
        let catalog = blessing_catalog();
        let mut pilot = Pilot::new(&catalog, "Ada").expect("starts");
        pilot.ranks.insert(RankId(200));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(blessed(&session), "flying refreshes (@0x19ffb)");
        run(&mut session, "L200");
        assert!(blessed(&session));
        session.jettison_cargo(true, &mut NeverFires);
        assert!(!blessed(&session));
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
        run(&mut session, "K200");
        assert!(!blessed(&session), "landing is no refresh");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(blessed(&session), "@0x6709b");
    }

    #[test]
    fn a_rank_granted_before_the_settling_after_a_take_off_counts() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        run(&mut session, "K200");
        assert!(!blessed(&session));
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(
            blessed(&session),
            "as the take-off's subscriptions ran first"
        );
        run(&mut session, "L200");
        let _ = session.settle_script(&catalog, &mut NeverFires);
        assert!(blessed(&session), "only the first settling refreshes");
    }

    #[test]
    fn a_jump_arrival_refreshes_the_blessing() {
        let catalog = blessing_catalog();
        let mut session = Session::start(&catalog).expect("starts");
        run(&mut session, "K200");
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
            run(&mut session, "K200");
            assert!(!blessed(&session), "{kind:?}");
            exit_gate(&mut session, &catalog, kind, &mut NeverFires);
            assert!(blessed(&session), "{kind:?}");
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
        run(&mut session, "K200");
        assert!(!blessed(&session));
        assert_eq!(
            crate::session::tests::take(&mut session, Take::Abort, &[]).0,
            Taken::Aborted
        );
        assert!(blessed(&session), "@0x94a0b");
    }
}
