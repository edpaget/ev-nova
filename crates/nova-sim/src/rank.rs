//! Ranks: the titles the pilot holds, as the original's `ränk` table
//! keeps them.
//!
//! **The table.** The original loads 128 slots, one for each `ränk`
//! 128-255 (`_LoadObjectData` @0x7da37-0x7dc4d). Each slot has an
//! "active" byte (+0) and an "exists" byte (+1), set when the record is
//! found (@0x7da98). Every reader tests both. `K` sets the active byte
//! without checking the other (@0x45332), so a rank with no record can be
//! active: it sweeps nothing (its slot's `Flags` are 0), nothing sweeps
//! it, and it pays, contributes and names nothing, but it still becomes
//! the recent rank. The engine models it as a rank in the pilot's active
//! set that is missing from the records.
//!
//! **`K`** ([`activate`], `_ActivateRank` @0x452f0). A rank already
//! active sweeps nothing, but becomes the recent rank. Otherwise it
//! becomes active (@0x45332) and then:
//!
//! - `Flags` 0x0001 ([`SWEEP_GOVT_ON_ACTIVATE`], @0x45335-0x45393) clears
//!   every other active rank of its government that lacks 0x0008.
//! - `Flags` 0x0010 ([`SWEEP_LOWER_ON_ACTIVATE`], @0x4539c-0x45404) does
//!   the same for the ranks of strictly lower `Weight` only, compared as
//!   signed 16-bit numbers (`jge` skips, @0x453de).
//!
//! The sweeps skip the rank itself by its index (@0x45356-0x4535d) and
//! clear the active bytes directly, never through `_DeactivateRank`, so a
//! swept rank's own flags do nothing: sweeps never cascade. A swept rank
//! that is the recent rank leaves none (@0x45372-0x45381,
//! @0x453e3-0x453f2), though `K` then makes its rank the recent one
//! (@0x45406-0x45413).
//!
//! **`L`** ([`deactivate`], `_DeactivateRank` @0x4541e). Only an active
//! rank sweeps (@0x4542d). It clears itself first (@0x45460), so it needs
//! no self test, then:
//!
//! - `Flags` 0x0002 ([`SWEEP_GOVT_ON_DEACTIVATE`], @0x45463-0x454bb)
//!   clears every active rank of its government that lacks 0x0008.
//! - `Flags` 0x0020 ([`SWEEP_LOWER_ON_DEACTIVATE`], @0x454c4-0x45526) does
//!   the same for those of strictly lower `Weight` (@0x45500).
//!
//! A swept rank that is the recent rank leaves none (@0x4549a-0x454a9,
//! @0x45505-0x45514), and so does `L` on the recent rank itself, active
//! or not (@0x45528-0x4553a).
//!
//! `Flags` 0x0008 ([`SHIELDED`]) shields a rank from every sweep, but
//! `L` still removes it.
//!
//! **Governments.** The sweeps compare `AffilGovt` raw (@0x4535f-0x45367,
//! @0x45487-0x4548f), so by the engine every unaffiliated rank (`AffilGovt`
//! -1, or 127 and below, which loads as -1) is one government. The
//! [`RankUnaffiliatedGroup`](crate::rulebook::RuleKey::RankUnaffiliatedGroup)
//! rule's other reading gives an unaffiliated rank no government.
//!
//! Neither function plays a sound or shows a message: their only calls
//! are crash-log lines (`_crashlog_print` @0x45305-0x45326,
//! @0x45433-0x45454), which the engine leaves out.
//!
//! **Contribute** ([`contributed`]). Every active rank with a record
//! adds its 64 `Contribute` bits to the player's, whatever its
//! government (`_GetPlayerContributeBits` @0x76de-0x7701).
//!
//! **Salary** ([`pay_salaries`], `_IncrementGameTime` @0xb75b-0xb7c2,
//! the day's eighth participant). In ID order, each active rank with a
//! record and a nonzero `Salary` adds it to the cash unless the cash is
//! at or above a `SalaryCap` above 0 (@0xb779-0xb794), then floors the
//! cash at 0 (@0xb79a-0xb7ac). So the cap is tested before paying and one
//! payment can overshoot it; a cap of 0 or less is no cap; a negative
//! salary is a fee, and the floor runs for every salaried rank, capped or
//! not. The [`SalaryCap`](crate::rulebook::RuleKey::SalaryCap) rule's
//! other reading pays only up to the cap ([`salary_paid`]). The loop plays
//! no sound.

use std::collections::BTreeSet;

use crate::catalog::{RankId, RankRecord};
use crate::rulebook::RuleSource;

/// `Flags` 0x0001: activating the rank clears the other active ranks of
/// its government.
pub const SWEEP_GOVT_ON_ACTIVATE: u16 = 0x0001;
/// `Flags` 0x0002: deactivating the rank clears the active ranks of its
/// government.
pub const SWEEP_GOVT_ON_DEACTIVATE: u16 = 0x0002;
/// `Flags` 0x0008: no sweep clears the rank.
pub const SHIELDED: u16 = 0x0008;
/// `Flags` 0x0010: activating the rank clears the active ranks of its
/// government of strictly lower `Weight`.
pub const SWEEP_LOWER_ON_ACTIVATE: u16 = 0x0010;
/// `Flags` 0x0020: deactivating the rank clears the active ranks of its
/// government of strictly lower `Weight`.
pub const SWEEP_LOWER_ON_DEACTIVATE: u16 = 0x0020;

/// The rank state `K` and `L` change: the active ranks, and the recent
/// rank `<RRK>` reads.
#[derive(Debug)]
pub struct Held<'a> {
    /// The active ranks.
    pub active: &'a mut BTreeSet<RankId>,
    /// The rank last activated, until `L` or a sweep clears it.
    pub recent: &'a mut Option<RankId>,
}

/// Rank `id`'s record among `records`, if it has one.
fn record(records: &[RankRecord], id: RankId) -> Option<&RankRecord> {
    records.iter().find(|record| record.id == id)
}

/// `K`: activates rank `id` as `_ActivateRank` does (see the module
/// docs), unaffiliated ranks one government or none by `unaffiliated`.
pub fn activate(records: &[RankRecord], mut held: Held, id: RankId, unaffiliated: RuleSource) {
    if held.active.insert(id)
        && let Some(by) = record(records, id)
    {
        if by.flags & SWEEP_GOVT_ON_ACTIVATE != 0 {
            sweep(records, &mut held, by, false, unaffiliated);
        }
        if by.flags & SWEEP_LOWER_ON_ACTIVATE != 0 {
            sweep(records, &mut held, by, true, unaffiliated);
        }
    }
    *held.recent = Some(id);
}

/// `L`: deactivates rank `id` as `_DeactivateRank` does (see the module
/// docs), unaffiliated ranks one government or none by `unaffiliated`.
pub fn deactivate(records: &[RankRecord], mut held: Held, id: RankId, unaffiliated: RuleSource) {
    if held.active.remove(&id)
        && let Some(by) = record(records, id)
    {
        if by.flags & SWEEP_GOVT_ON_DEACTIVATE != 0 {
            sweep(records, &mut held, by, false, unaffiliated);
        }
        if by.flags & SWEEP_LOWER_ON_DEACTIVATE != 0 {
            sweep(records, &mut held, by, true, unaffiliated);
        }
    }
    if *held.recent == Some(id) {
        *held.recent = None;
    }
}

/// What `rank` pays today on `cash`, by `cap`'s reading of its
/// `SalaryCap` (see the module docs): nothing at or above a cap above 0;
/// otherwise its `Salary`, or by the other reading no more than takes the
/// cash to the cap. A fee is never capped further.
#[must_use]
pub fn salary_paid(rank: &RankRecord, cash: i64, cap: RuleSource) -> i64 {
    let capped = rank.salary_cap > 0;
    if capped && cash >= rank.salary_cap {
        0
    } else if cap == RuleSource::Bible && capped && rank.salary > 0 {
        rank.salary.min(rank.salary_cap - cash)
    } else {
        rank.salary
    }
}

/// One day's salaries: each rank of `records` in `active` with a nonzero
/// `Salary`, in ID order, adds [`salary_paid`] to `cash`, then floors it
/// at 0 (`_IncrementGameTime` @0xb75b-0xb7c2).
pub fn pay_salaries(
    records: &[RankRecord],
    active: &BTreeSet<RankId>,
    cash: &mut i64,
    cap: RuleSource,
) {
    for rank in records {
        if active.contains(&rank.id) && rank.salary != 0 {
            *cash = cash.saturating_add(salary_paid(rank, *cash, cap));
            *cash = (*cash).max(0);
        }
    }
}

/// The `Contribute` bits of the ranks of `records` in `active`, whatever
/// their government (`_GetPlayerContributeBits` @0x76de-0x7701).
#[must_use]
pub fn contributed(records: &[RankRecord], active: &BTreeSet<RankId>) -> u64 {
    records
        .iter()
        .filter(|rank| active.contains(&rank.id))
        .fold(0, |bits, rank| bits | rank.contribute)
}

/// Whether ranks of `a` and `b` share a government for the sweeps.
fn same_govt(a: &RankRecord, b: &RankRecord, unaffiliated: RuleSource) -> bool {
    match (a.affil_govt, b.affil_govt) {
        (None, None) => unaffiliated == RuleSource::Engine,
        (a, b) => a == b,
    }
}

/// Clears every active rank with a record, other than `by`, of `by`'s
/// government and not [`SHIELDED`], and only those of strictly lower
/// `Weight` when `lower_only`; a cleared recent rank leaves none.
fn sweep(
    records: &[RankRecord],
    held: &mut Held,
    by: &RankRecord,
    lower_only: bool,
    unaffiliated: RuleSource,
) {
    for swept in records {
        if swept.id == by.id
            || swept.flags & SHIELDED != 0
            || !same_govt(swept, by, unaffiliated)
            || (lower_only && swept.weight >= by.weight)
            || !held.active.remove(&swept.id)
        {
            continue;
        }
        if *held.recent == Some(swept.id) {
            *held.recent = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::GovtId;
    use crate::testkit;

    /// Rank `id` of government `govt` (`None` unaffiliated), `weight`
    /// and `flags`.
    fn rank(id: i16, govt: Option<i16>, weight: i16, flags: u16) -> RankRecord {
        RankRecord {
            affil_govt: govt.map(GovtId),
            weight,
            flags,
            ..testkit::rank(id)
        }
    }

    /// The active ranks and the recent rank.
    #[derive(Debug, Default, PartialEq, Eq)]
    struct State {
        active: BTreeSet<RankId>,
        recent: Option<RankId>,
    }

    impl State {
        /// Active ranks `ids`, with no recent rank.
        fn holding(ids: &[i16]) -> Self {
            Self {
                active: ids.iter().copied().map(RankId).collect(),
                recent: None,
            }
        }

        fn held(&mut self) -> Held<'_> {
            Held {
                active: &mut self.active,
                recent: &mut self.recent,
            }
        }

        /// `K` on rank `id`, by the engine.
        fn k(&mut self, records: &[RankRecord], id: i16) -> &mut Self {
            activate(records, self.held(), RankId(id), RuleSource::Engine);
            self
        }

        /// `L` on rank `id`, by the engine.
        fn l(&mut self, records: &[RankRecord], id: i16) -> &mut Self {
            deactivate(records, self.held(), RankId(id), RuleSource::Engine);
            self
        }

        fn active(&self) -> Vec<i16> {
            self.active.iter().map(|id| id.0).collect()
        }
    }

    #[test]
    fn activating_a_rank_makes_it_active_and_the_recent_rank() {
        let records = [rank(200, Some(128), 1, 0)];
        let mut state = State::default();
        state.k(&records, 200);
        assert_eq!(state.active(), [200]);
        assert_eq!(state.recent, Some(RankId(200)));
    }

    #[test]
    fn activating_an_active_rank_sweeps_nothing_but_makes_it_the_recent_rank() {
        let records = [
            rank(200, Some(128), 5, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, Some(128), 1, 0),
        ];
        let mut state = State::holding(&[200, 201]);
        state.recent = Some(RankId(201));
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 201], "@0x452ff");
        assert_eq!(state.recent, Some(RankId(200)));
    }

    #[test]
    fn flag_0x01_sweeps_every_other_active_rank_of_its_government() {
        let records = [
            rank(200, Some(128), 1, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, Some(128), 9, 0),
            rank(202, Some(128), -4, 0),
            rank(203, Some(129), 0, 0),
        ];
        let mut state = State::holding(&[201, 202, 203]);
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 203], "itself kept, a higher swept");
        assert_eq!(state.recent, Some(RankId(200)));
    }

    #[test]
    fn flag_0x10_sweeps_only_the_strictly_lower_weights_of_its_government() {
        let records = [
            rank(200, Some(128), 0, SWEEP_LOWER_ON_ACTIVATE),
            rank(201, Some(128), 0, 0),
            rank(202, Some(128), -1, 0),
            rank(203, Some(128), 1, 0),
            rank(204, Some(128), -32_768, 0),
            rank(205, Some(129), -5, 0),
        ];
        let mut state = State::holding(&[201, 202, 203, 204, 205]);
        state.k(&records, 200);
        assert_eq!(
            state.active(),
            [200, 201, 203, 205],
            "an equal Weight kept, negatives compared signed"
        );
    }

    #[test]
    fn flag_0x02_sweeps_its_government_as_it_is_deactivated() {
        let records = [
            rank(200, Some(128), 1, SWEEP_GOVT_ON_DEACTIVATE),
            rank(201, Some(128), 9, 0),
            rank(202, Some(129), 0, 0),
        ];
        let mut state = State::holding(&[200, 201, 202]);
        state.k(&records, 202).l(&records, 200);
        assert_eq!(state.active(), [202]);
        assert_eq!(state.recent, Some(RankId(202)), "another government's");
        let mut state = State::holding(&[200, 201]);
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 201], "not as it is activated");
    }

    #[test]
    fn flag_0x20_sweeps_the_lower_weights_as_it_is_deactivated() {
        let records = [
            rank(200, Some(128), 3, SWEEP_LOWER_ON_DEACTIVATE),
            rank(201, Some(128), 3, 0),
            rank(202, Some(128), 2, 0),
            rank(203, Some(128), 4, 0),
        ];
        let mut state = State::holding(&[200, 201, 202, 203]);
        state.l(&records, 200);
        assert_eq!(state.active(), [201, 203]);
        let mut state = State::holding(&[201, 202]);
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 201, 202], "not as it is activated");
    }

    #[test]
    fn a_shielded_rank_resists_every_sweep_but_l_removes_it() {
        let records = [
            rank(
                200,
                Some(128),
                9,
                SWEEP_GOVT_ON_ACTIVATE | SWEEP_LOWER_ON_ACTIVATE,
            ),
            rank(201, Some(128), 9, SWEEP_GOVT_ON_DEACTIVATE),
            rank(202, Some(128), 9, SWEEP_LOWER_ON_DEACTIVATE),
            rank(203, Some(128), 1, SHIELDED),
        ];
        let mut state = State::holding(&[203]);
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 203]);
        let mut state = State::holding(&[201, 202, 203]);
        state.l(&records, 201);
        assert_eq!(state.active(), [203]);
        state.k(&records, 202).l(&records, 202);
        assert_eq!(state.active(), [203]);
        state.l(&records, 203);
        assert_eq!(state.active(), [] as [i16; 0], "L removes it");
    }

    #[test]
    fn sweeps_do_not_cascade() {
        let records = [
            rank(200, Some(128), 5, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, Some(128), 1, SWEEP_GOVT_ON_DEACTIVATE),
            rank(202, Some(128), 1, SWEEP_LOWER_ON_DEACTIVATE),
            rank(203, Some(129), 0, 0),
        ];
        let mut state = State::holding(&[201, 202, 203]);
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 203]);
        let records = [
            rank(200, Some(128), 5, SWEEP_GOVT_ON_DEACTIVATE),
            rank(201, Some(128), 1, SWEEP_GOVT_ON_DEACTIVATE),
            rank(202, Some(128), 0, SHIELDED),
        ];
        let mut state = State::holding(&[200, 201, 202]);
        state.l(&records, 200);
        assert_eq!(state.active(), [202], "201's own sweep never ran");
    }

    #[test]
    fn deactivating_an_inactive_rank_sweeps_nothing_but_clears_it_as_the_recent_rank() {
        let records = [
            rank(200, Some(128), 5, SWEEP_GOVT_ON_DEACTIVATE),
            rank(201, Some(128), 1, 0),
        ];
        let mut state = State::holding(&[201]);
        state.recent = Some(RankId(200));
        state.l(&records, 200);
        assert_eq!(state.active(), [201], "@0x4542d");
        assert_eq!(state.recent, None, "@0x45528");
    }

    #[test]
    fn deactivating_the_recent_rank_leaves_none() {
        let records = [rank(200, Some(128), 1, 0), rank(201, Some(128), 1, 0)];
        let mut state = State::default();
        state.k(&records, 201).k(&records, 200).l(&records, 201);
        assert_eq!(state.recent, Some(RankId(200)), "not the recent rank");
        state.l(&records, 200);
        assert_eq!(state.recent, None);
    }

    #[test]
    fn a_sweep_that_clears_the_recent_rank_leaves_none() {
        let records = [
            rank(200, Some(128), 1, 0),
            rank(201, Some(128), 1, SWEEP_GOVT_ON_DEACTIVATE),
        ];
        let mut state = State::default();
        state.k(&records, 201).k(&records, 200);
        state.recent = Some(RankId(200));
        state.l(&records, 201);
        assert_eq!(state.active(), [] as [i16; 0]);
        assert_eq!(state.recent, None, "@0x4549a-0x454a9");
        let records = [
            rank(200, Some(128), 1, 0),
            rank(201, Some(128), 5, SWEEP_LOWER_ON_DEACTIVATE),
        ];
        let mut state = State::default();
        state.k(&records, 201).k(&records, 200).l(&records, 201);
        assert_eq!(state.recent, None, "@0x45505-0x45514");
    }

    #[test]
    fn a_rank_with_no_record_activates_sweeps_nothing_and_is_never_swept() {
        let records = [
            rank(
                200,
                Some(128),
                5,
                SWEEP_GOVT_ON_ACTIVATE | SWEEP_GOVT_ON_DEACTIVATE,
            ),
            rank(201, Some(128), 1, 0),
        ];
        let mut state = State::holding(&[201]);
        state.k(&records, 250);
        assert_eq!(state.active(), [201, 250], "@0x45332");
        assert_eq!(state.recent, Some(RankId(250)));
        state.k(&records, 200);
        assert_eq!(state.active(), [200, 250], "250 never swept");
        state.l(&records, 200);
        assert_eq!(state.active(), [250]);
        state.l(&records, 250);
        assert_eq!(state.active(), [] as [i16; 0]);
    }

    #[test]
    fn ranks_of_other_governments_are_never_swept() {
        let records = [
            rank(
                200,
                Some(128),
                9,
                SWEEP_GOVT_ON_ACTIVATE | SWEEP_LOWER_ON_ACTIVATE | SWEEP_GOVT_ON_DEACTIVATE,
            ),
            rank(201, Some(129), 1, 0),
            rank(202, None, 1, 0),
        ];
        let mut state = State::holding(&[201, 202]);
        state.k(&records, 200).l(&records, 200);
        assert_eq!(state.active(), [201, 202]);
    }

    #[test]
    fn unaffiliated_ranks_sweep_each_other_by_the_engine_and_never_by_the_other_reading() {
        let records = [
            rank(200, None, 5, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, None, 5, SWEEP_GOVT_ON_DEACTIVATE),
            rank(202, None, 1, 0),
            rank(203, Some(128), 1, 0),
        ];
        let run = |source: RuleSource, k: bool, id: i16| {
            let mut state = State::holding(&[201, 202, 203]);
            let held = state.held();
            if k {
                activate(&records, held, RankId(id), source);
            } else {
                deactivate(&records, held, RankId(id), source);
            }
            state.active()
        };
        assert_eq!(run(RuleSource::Engine, true, 200), [200, 203]);
        assert_eq!(run(RuleSource::Engine, false, 201), [203]);
        assert_eq!(run(RuleSource::Bible, true, 200), [200, 201, 202, 203]);
        assert_eq!(run(RuleSource::Bible, false, 201), [202, 203]);
        let records = [
            rank(200, Some(128), 5, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, None, 1, 0),
        ];
        let mut state = State::holding(&[201]);
        activate(&records, state.held(), RankId(200), RuleSource::Bible);
        assert_eq!(state.active(), [200, 201], "an affiliated one never");
    }

    /// Rank `id` paying `salary` a day up to `cap`.
    fn paying(id: i16, salary: i64, cap: i64) -> RankRecord {
        RankRecord {
            salary,
            salary_cap: cap,
            ..testkit::rank(id)
        }
    }

    /// The cash after one day's salaries from `records`, all active, on
    /// `cash`, by `source`.
    fn paid(records: &[RankRecord], cash: i64, source: RuleSource) -> i64 {
        let active = records.iter().map(|record| record.id).collect();
        let mut cash = cash;
        pay_salaries(records, &active, &mut cash, source);
        cash
    }

    #[test]
    fn a_salary_is_paid_each_day() {
        for source in RuleSource::ALL {
            assert_eq!(paid(&[paying(200, 200, 0)], 1000, source), 1200);
        }
    }

    #[test]
    fn a_cap_of_0_or_less_is_no_cap() {
        for source in RuleSource::ALL {
            for cap in [0, -1, -500] {
                assert_eq!(
                    paid(&[paying(200, 200, cap)], 1000, source),
                    1200,
                    "{cap} {source:?}"
                );
            }
        }
    }

    #[test]
    fn by_the_engine_the_cap_is_tested_before_paying_so_it_can_overshoot() {
        let records = [paying(200, 200, 1100)];
        assert_eq!(paid(&records, 1000, RuleSource::Engine), 1200);
        assert_eq!(paid(&records, 1099, RuleSource::Engine), 1299);
    }

    #[test]
    fn by_the_other_reading_the_cash_never_passes_the_cap() {
        let records = [paying(200, 200, 1100)];
        assert_eq!(paid(&records, 1000, RuleSource::Bible), 1100);
        assert_eq!(paid(&records, 1099, RuleSource::Bible), 1100);
        assert_eq!(paid(&records, 800, RuleSource::Bible), 1000, "room left");
    }

    #[test]
    fn at_or_above_the_cap_nothing_is_paid_by_either_reading() {
        for source in RuleSource::ALL {
            assert_eq!(paid(&[paying(200, 200, 1000)], 1000, source), 1000);
            assert_eq!(paid(&[paying(200, 200, 1000)], 1500, source), 1500);
            assert_eq!(
                paid(&[paying(200, -200, 1000)], 1500, source),
                1500,
                "a fee too"
            );
        }
    }

    #[test]
    fn a_fee_takes_cash_and_floors_it_at_0() {
        for source in RuleSource::ALL {
            assert_eq!(paid(&[paying(200, -200, 0)], 1000, source), 800);
            assert_eq!(paid(&[paying(200, -200, 0)], 150, source), 0);
            assert_eq!(
                paid(&[paying(200, -200, 5000)], 1000, source),
                800,
                "below the cap, a fee is not capped further"
            );
        }
    }

    #[test]
    fn any_salaried_rank_floors_a_negative_cash() {
        for source in RuleSource::ALL {
            assert_eq!(paid(&[paying(200, 10, 0)], -50, source), 0, "not -40");
        }
    }

    #[test]
    fn ranks_pay_in_id_order_each_tested_against_the_cash_so_far() {
        let records = [paying(200, 300, 0), paying(201, 100, 1200)];
        assert_eq!(
            paid(&records, 1000, RuleSource::Engine),
            1300,
            "200 took the cash to the cap of 201"
        );
        let records = [paying(200, 100, 1200), paying(201, 300, 0)];
        assert_eq!(paid(&records, 1000, RuleSource::Engine), 1400);
        let records = [paying(200, -100, 0), paying(201, 50, 0)];
        assert_eq!(
            paid(&records, 60, RuleSource::Engine),
            50,
            "the fee floored at 0 before the next pays"
        );
    }

    #[test]
    fn an_inactive_rank_or_one_with_no_record_or_no_salary_pays_nothing_and_floors_nothing() {
        let records = [paying(200, 100, 0), paying(201, 0, 0)];
        for source in RuleSource::ALL {
            let mut cash = -50;
            let active = BTreeSet::from([RankId(201), RankId(250)]);
            pay_salaries(&records, &active, &mut cash, source);
            assert_eq!(cash, -50, "{source:?}");
        }
    }

    #[test]
    fn a_salary_paid_is_the_salary_under_the_cap_test() {
        let rank = paying(200, 200, 1100);
        assert_eq!(salary_paid(&rank, 1000, RuleSource::Engine), 200);
        assert_eq!(salary_paid(&rank, 1000, RuleSource::Bible), 100);
        assert_eq!(salary_paid(&rank, 1100, RuleSource::Engine), 0);
        assert_eq!(salary_paid(&rank, 1100, RuleSource::Bible), 0);
        assert_eq!(salary_paid(&rank, -5000, RuleSource::Engine), 200);
        assert_eq!(
            salary_paid(&paying(200, -30, 0), 10, RuleSource::Bible),
            -30
        );
    }
}
