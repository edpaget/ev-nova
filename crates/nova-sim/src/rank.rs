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
//! **Text** ([`RankNames`], `_MungeBriefing` @0x9d039-0x9d3b3). `<PRK>`
//! and `<SRK>` are the `ConvName` and `ShortName` of the active rank of
//! highest `Weight` with that name non-empty, in ID order, a rank
//! replacing the best only with a strictly greater `Weight`, so a tie
//! goes to the lowest ID (@0x9d039-0x9d1a5). `<PRKnnn>` and `<SRKnnn>`
//! both read the `ShortName` of government nnn's such rank (+0xde
//! @0x9d1ef, @0x9d2d0); an unaffiliated rank is no government's. `<RRK>`
//! is the recent rank's full name, with no active, record or emptiness
//! test (@0x9d35f-0x9d391), so it reads empty for a recent rank with no
//! record or no name. Each falls back to `STR#` 2002 #341, "captain"
//! (@0x9d0d6), only when no rank qualifies, or there is no recent rank.
//!
//! **Pilot info** ([`pilot_info`], `_SetupPlayerInfoText`
//! @0x54bf9-0x54f29). The ranks line lists the active ranks with a
//! non-empty full name, by `Weight` descending: passes of the highest
//! `Weight` left, each placing those of it in index order, so a tie goes
//! by ID (@0x54dbc-0x54f29). Let N be those ranks plus the Ranks-section
//! items that follow them (oütf `Flags` 0x2000, counted @0x54c82-0x54d4b
//! into the same count). After placing the k-th rank, counting from 1:
//! below N-1 it appends ", " (@0x54e47, @0x54ead); at N-1 it appends ",
//! ", `STR#` 2002 #392 ("and") and " " (@0x54e47, @0x54e7b-0x54ead); at
//! N, nothing (@0x54e5f-0x54e65). So "and" goes before the last item of
//! the whole list: "A, and B" and "A, B, and C" alone, but "A, B, and "
//! with one item following and "A, B, " with two. The heading, the line
//! breaks and the items are phase 17's; [`pilot_info`] takes how many
//! items follow. The daily income adds each active rank's salary under
//! the same cap test against today's cash (`_PlayerInfoDialogUpdate`
//! @0x53585-0x535c1), with no floor.
//!
//! **Hails** ([`favour`], `_DoCommDialog` @0x95d80-0x95e46). As the
//! comm dialog opens, it loops over the 128 rank slots (stride 0x120,
//! @0x95d9c-0x95e06). A slot that is active (+0), has a record (+1) and
//! whose `AffilGovt` (+6) `_GovtAllies` finds allied with the hailed
//! ship's government (+0x98, @0x95dbf) adds its `Flags` (+0x1c): 0x0400
//! ([`HELPS_IN_BATTLE`]) gives battle help (@0x95ddb), 0x0800
//! ([`HELPS_FREE`]) free help (@0x95de1). `_GovtAllies` holds at once for
//! equal IDs (@0x4e4e), -1 and -1 included, so by the engine an
//! unaffiliated rank reaches an independent ship. The
//! [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) rule's
//! other reading reaches only ships of the rank's own government. The
//! loop plays no sound. What the help does is the hail's (see
//! [`hail`](crate::hail)).
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
//!
//! **Prices** ([`price_mults`], `_DoPortDialog` @0x5f970-0x5fa04). As
//! the spaceport opens, both of `_priceFluxMult`'s slots (0x3166d4) are
//! set to 1.0. Unless the stellar is independent (its government -1,
//! @0x5f98b-0x5f990), each of the 128 rank slots in ID order that is
//! active (+0), has a record (+1) and whose `AffilGovt` (+6) `_GovtAllies`
//! finds allied with the stellar's government (@0x5f9bb) multiplies both
//! by its `PriceMod` (+8) as a fraction: `m = (float)((double)m *
//! ((double)PriceMod * 0.01))` (@0x5f9c4-0x5f9f6), so ranks stack, each
//! product worked in double precision and rounded to single (two ranks of
//! 90 give exactly 0.81f, where a single-precision product gives
//! 0.80999994). The two slots always hold the same factor ([`PriceMults`]).
//! An unaffiliated rank counts nowhere: at an independent stellar nothing
//! counts, and at a governed one `_GovtAllies` never finds -1 allied. The
//! [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) rule's
//! other reading counts only a rank of the stellar's own government. The
//! block plays no sound. Only the shipyard reads the slots, through
//! `_ApplyPriceAndTechnologyFlux` ([`price_flux`](crate::hire::price_flux)):
//! a ship's price and the hire fee with `[1]`, the trade-in with `[0]`
//! then `[1]` (see [`shipyard`](crate::shipyard)).
//!
//! **Crimes** ([`lose_to_crime`], `_SlapWithPenalty` @0x9c47-0x9caf).
//! After the penalty walk (see [`legal`](crate::legal)), a crime the
//! player commits against a ship of government V takes ranks, but only
//! when V is not derelict (byte +0x21 bit 0x08, @0x9bd5-0x9bf7; the
//! test is skipped for an independent V) and the crime's mission slot is
//! -1 (@0x9c10), so never for a mission ship. The pass goes through the
//! 128 slots in ID order (stride 0x120). A slot that is active (+0) at
//! its turn, has a record (+1), lacks [`SHIELDED`] (+0x1c) and whose
//! `AffilGovt` (+6) `_GovtAllies` finds allied with V (@0x9c70) goes
//! through `_DeactivateRank` (@0x9c9e), so its 0x0002 and 0x0020 sweeps
//! run as for `L`, when:
//!
//! - it has [`LOST_TO_CRIME`] (0x0040), on any crime; or
//! - it has [`LOST_TO_VIOLENCE`] (0x0004), on crime type 1 or 3
//!   (@0x9c89-0x9c97).
//!
//! The original's crime types map onto [`Crime`]: 0 (contraband a scan
//! finds, `_ScanPlayer` @0x7ebcc, @0x7f01f) is [`Crime::Smuggle`], so it
//! takes 0x0040 alone; 1 (`_DamageShip`
//! @0x3addf) is [`Crime::Disable`], 2 (`_HandlePlayerBoardAttempt`
//! @0x6582b) [`Crime::Board`], 3 (`_DamageShip` @0x3ae34, and a stellar
//! shot down, `_CheckShotProximities` @0x37518) [`Crime::Kill`]. No
//! caller slaps for a hit, so [`Crime::Shoot`] takes nothing. A rank an
//! earlier rank's sweep cleared is skipped at its turn, so its own flags
//! do nothing. `_GovtAllies` holds at once for equal IDs (@0x4e4e), so a
//! crime against an independent ship takes an unaffiliated rank; no
//! stock rank with these flags is unaffiliated. The stellar kill slaps
//! ten times, but after the first pass no matching rank is still active,
//! so the session runs the pass once. The
//! [`RankCrimeReach`](crate::rulebook::RuleKey::RankCrimeReach) rule's
//! other reading, the Bible's, lets 0x0040 reach only the rank's own
//! government, 0x0004 its own or its allies, and an unaffiliated rank
//! none. Neither function plays a sound.
//!
//! Left out: Demand Tribute's type-3 slaps (`_DoPlanetCommDialog` @0x975d0,
//! @0x9773d, @0x9785f), with tribute (rdm:task/planet-domination); and
//! the debug "destroy target" key (`_HandlePlayer` @0x6ecfe), which the
//! engine does not model.
//!
//! **Blessing** ([`blessing`], [`Blessing`]). `_PlayerBlessedByGovt(g,
//! which)` (@0x2732-0x276b) returns the byte `govt[g]+0x84+which` (stride
//! 0x88, base 0x3b5164), and 0 for `g` above 0xff unsigned (so -1) or
//! `which` above 1. `which` 0 is [`BLESSES_PEACE`] (0x0100, "won't
//! attack"), 1 is [`BLESSES_LANDING`] (0x0200, "may land").
//! `_ResetPlayerPrecalcedValues` (@0xc8be-0xc981) builds the table: it
//! clears both bytes of all 256 governments, then for each of the 128
//! rank slots (stride 0x120) that is active (+0), has a record (+1), has
//! `Flags` (+0x1c) & 0x0300 and an `AffilGovt` (+6) other than -1
//! (@0xc90a), it tests every `g` 0-255 with `_GovtAllies(g, AffilGovt)`
//! (@0xc925): 0x0100 sets `[g][0]` (@0xc936-0xc945), 0x0200 `[g][1]`
//! (@0xc94f-0xc95e). `_GovtAllies` holds at once for equal IDs (@0x4e4e)
//! and never for a derelict government otherwise (@0x4e91, @0x4ea9). The
//! [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) rule's
//! other reading blesses only the rank's own government. Only the
//! governments the data holds are tested, and the rank's own: a reader
//! asks of a ship's or a stellar's government, which the data holds. The
//! rest of the function resets the player's other cached values (derived
//! stats, cargo totals and one `Rand`), which the engine works out live.
//! It plays no sound, and the table is not saved. When the session takes
//! it is the session's (see its `ranks` module); landing reads `[1]` (see
//! [`landing`](crate::landing)), and the AI's readers of `[0]` are
//! rdm:phase/crons-news-ranks/phase-18-rank-blessing-in-the-ai.

use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::catalog::{GovtId, RankId, RankRecord};
use crate::govt::Governments;
use crate::legal::Crime;
use crate::rulebook::RuleSource;

pub use crate::ai::{Blessed, Blessing, Favour, HELPS_FREE, HELPS_IN_BATTLE, favour};

use crate::ai::reaches;

/// The index (1-based) in `STR#` 2002 of what a rank tag reads with no
/// rank to name: stock "captain" (0x155 @0x9d0d6).
pub const NO_RANK_STRING: usize = 341;

/// The index (1-based) in `STR#` 2002 of the word before the pilot
/// info's last rank: stock "and" (0x188 @0x54e81).
pub const AND_STRING: usize = 392;

/// `Flags` 0x0001: activating the rank clears the other active ranks of
/// its government.
pub const SWEEP_GOVT_ON_ACTIVATE: u16 = 0x0001;
/// `Flags` 0x0002: deactivating the rank clears the active ranks of its
/// government.
pub const SWEEP_GOVT_ON_DEACTIVATE: u16 = 0x0002;
/// `Flags` 0x0004: disabling or destroying a ship the rank reaches
/// takes it (`_SlapWithPenalty` @0x9c89-0x9c97).
pub const LOST_TO_VIOLENCE: u16 = 0x0004;
/// `Flags` 0x0008: no sweep clears the rank, and no crime takes it.
pub const SHIELDED: u16 = 0x0008;
/// `Flags` 0x0010: activating the rank clears the active ranks of its
/// government of strictly lower `Weight`.
pub const SWEEP_LOWER_ON_ACTIVATE: u16 = 0x0010;
/// `Flags` 0x0020: deactivating the rank clears the active ranks of its
/// government of strictly lower `Weight`.
pub const SWEEP_LOWER_ON_DEACTIVATE: u16 = 0x0020;
/// `Flags` 0x0040: any crime against a ship the rank reaches takes it
/// (`_SlapWithPenalty` @0x9c47-0x9caf).
pub const LOST_TO_CRIME: u16 = 0x0040;
/// `Flags` 0x0100: ships of the governments the rank reaches won't attack
/// the player (`_PlayerBlessedByGovt` which 0; see [`Blessed::Peace`]).
pub const BLESSES_PEACE: u16 = 0x0100;
/// `Flags` 0x0200: stellars of the governments the rank reaches clear the
/// player to land (which 1; see [`Blessed::Landing`]).
pub const BLESSES_LANDING: u16 = 0x0200;

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

/// `_SlapWithPenalty`'s rank pass (see the module docs' **Crimes**) for
/// `crime` against a ship of `victim`: unless `victim`'s government is
/// derelict, each rank of `records` in ID order that is active at its
/// turn, not [`SHIELDED`], whose flags `crime` takes and that reaches
/// `victim` by `reach`'s reading of
/// [`RankCrimeReach`](crate::rulebook::RuleKey::RankCrimeReach) goes
/// through [`deactivate`], sweeps included, unaffiliated ranks one
/// government or none by `unaffiliated`.
pub fn lose_to_crime(
    records: &[RankRecord],
    Held { active, recent }: Held,
    crime: Crime,
    victim: Option<GovtId>,
    govts: &Governments,
    reach: RuleSource,
    unaffiliated: RuleSource,
) {
    if govts.derelict(victim) {
        return;
    }
    for rank in records {
        if active.contains(&rank.id)
            && rank.flags & SHIELDED == 0
            && crime_reaches(rank, crime, victim, govts, reach)
        {
            deactivate(
                records,
                Held {
                    active: &mut *active,
                    recent: &mut *recent,
                },
                rank.id,
                unaffiliated,
            );
        }
    }
}

/// The crime flags of `flags` that `crime` takes: 0x0040 on any crime
/// the original slaps, 0x0004 on disabling and destroying (types 1 and 3,
/// @0x9c89-0x9c97). The original never slaps for a hit.
fn flags_taken(flags: u16, crime: Crime) -> u16 {
    match crime {
        Crime::Disable | Crime::Kill => flags & (LOST_TO_CRIME | LOST_TO_VIOLENCE),
        Crime::Board | Crime::Smuggle => flags & LOST_TO_CRIME,
        Crime::Shoot => 0,
    }
}

/// Whether `crime` against a ship of `victim` takes `rank`, by `reach`'s
/// reading of [`RankCrimeReach`](crate::rulebook::RuleKey::RankCrimeReach):
/// by the engine, when `crime` takes one of its flags and `_GovtAllies`
/// finds the governments allied, -1 and -1 included (@0x9c70); by the
/// other reading, when it takes 0x0040 of a rank of `victim`'s own
/// government, or 0x0004 of one of that government or its allies, and
/// never an unaffiliated rank.
fn crime_reaches(
    rank: &RankRecord,
    crime: Crime,
    victim: Option<GovtId>,
    govts: &Governments,
    reach: RuleSource,
) -> bool {
    let taken = flags_taken(rank.flags, crime);
    match reach {
        RuleSource::Engine => taken != 0 && govts.allies(victim, rank.affil_govt),
        RuleSource::Bible => {
            let own = rank.affil_govt.is_some() && rank.affil_govt == victim;
            let allied = rank.affil_govt.is_some() && govts.allies(victim, rank.affil_govt);
            (taken & LOST_TO_CRIME != 0 && own) || (taken & LOST_TO_VIOLENCE != 0 && allied)
        }
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
    } else if cap == RuleSource::Bible && capped {
        // Below the cap, so a fee is less than the room left.
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

/// The two price multipliers the shipyard reads, `_priceFluxMult[0]` and
/// `[1]` (see the module docs' **Prices**).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceMults {
    /// `_priceFluxMult[0]`: the trade-in's first pass.
    pub first: f32,
    /// `_priceFluxMult[1]`: a ship's price, the hire fee and the
    /// trade-in's second pass.
    pub second: f32,
}

impl PriceMults {
    /// No rank's factor: both 1.0.
    pub const NONE: Self = Self {
        first: 1.0,
        second: 1.0,
    };
}

impl Default for PriceMults {
    fn default() -> Self {
        Self::NONE
    }
}

/// The price multipliers the ranks of `records` in `active` give at a
/// stellar of `stellar_govt` (see the module docs' **Prices**): none at
/// an independent stellar; otherwise each rank, in the records' order (by
/// ID, as the catalog gives them), that reaches
/// the stellar's government by `reach`'s reading of
/// [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) multiplies
/// both by its `PriceMod` %, in double precision rounded to single.
#[must_use]
pub fn price_mults(
    records: &[RankRecord],
    active: &BTreeSet<RankId>,
    stellar_govt: Option<GovtId>,
    govts: &Governments,
    reach: RuleSource,
) -> PriceMults {
    if stellar_govt.is_none() {
        return PriceMults::NONE;
    }
    let mult = records
        .iter()
        .filter(|rank| active.contains(&rank.id) && reaches(rank, stellar_govt, govts, reach))
        .fold(1.0_f32, |mult, rank| {
            (f64::from(mult) * (f64::from(rank.price_mod) * 0.01)) as f32
        });
    PriceMults {
        first: mult,
        second: mult,
    }
}

/// `price` scaled by the ranks' factor `mult` alone, as the outfitter's
/// buy price takes it by the other reading of
/// [`RankPriceScope`](crate::rulebook::RuleKey::RankPriceScope): none for
/// a price of none or less; otherwise the product in single precision, as
/// the shipyard's flux takes it, cut down to whole credits and never
/// below 1. The original's outfitter never reads the multipliers, so it
/// has no tech discount or rounding to take with them; this is the port's
/// choice.
#[must_use]
pub fn scaled(price: i64, mult: f32) -> i64 {
    if price <= 0 {
        return 0;
    }
    ((price as f32 * mult) as i64).max(1)
}

/// `_ResetPlayerPrecalcedValues`' blessing pass (see the module docs'
/// **Blessing**): each rank of `records` in `active` that has
/// [`BLESSES_PEACE`] or [`BLESSES_LANDING`] and a government blesses
/// every government it reaches by `reach`'s reading of
/// [`RankGovtReach`](crate::rulebook::RuleKey::RankGovtReach) with its
/// flags. An unaffiliated rank blesses none.
#[must_use]
pub fn blessing(
    records: &[RankRecord],
    active: &BTreeSet<RankId>,
    govts: &Governments,
    reach: RuleSource,
) -> Blessing {
    let mut blessed = Blessing::NONE;
    for rank in records {
        let Some(own) = rank.affil_govt else {
            continue;
        };
        // A rank with neither flag goes through the loop blessing nothing,
        // as the original skips it (@0xc90a).
        if !active.contains(&rank.id) {
            continue;
        }
        // `_GovtAllies` holds at once for equal IDs, so the rank's own
        // government is blessed even where the table lacks it.
        for govt in govts.ids().chain([own]) {
            if !reaches(rank, Some(govt), govts, reach) {
                continue;
            }
            if rank.flags & BLESSES_PEACE != 0 {
                blessed.bless(govt, Blessed::Peace);
            }
            if rank.flags & BLESSES_LANDING != 0 {
                blessed.bless(govt, Blessed::Landing);
            }
        }
    }
    blessed
}

/// The names the rank tags read (see the module docs): the records, the
/// active ranks, the recent rank and the fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankNames<'a> {
    /// Every `ränk`.
    pub records: &'a [RankRecord],
    /// The active ranks.
    pub active: &'a BTreeSet<RankId>,
    /// The recent rank, if any.
    pub recent: Option<RankId>,
    /// What a tag reads with no rank to name: `STR#` 2002 #341.
    pub fallback: Cow<'a, str>,
}

/// No rank held, for a text read with no ranks wired.
static NONE_HELD: BTreeSet<RankId> = BTreeSet::new();

impl RankNames<'_> {
    /// No rank held, and the fallback [`NO_RANK`](crate::person::NO_RANK).
    #[must_use]
    pub fn none() -> RankNames<'static> {
        RankNames {
            records: &[],
            active: &NONE_HELD,
            recent: None,
            fallback: Cow::Borrowed(crate::person::NO_RANK),
        }
    }

    /// `<PRK>`: the highest active rank's `ConvName`.
    #[must_use]
    pub fn conversation(&self) -> &str {
        self.highest(|rank| &rank.conv_name, None)
    }

    /// `<SRK>`: the highest active rank's `ShortName`.
    #[must_use]
    pub fn short(&self) -> &str {
        self.highest(|rank| &rank.short_name, None)
    }

    /// `<PRKnnn>` and `<SRKnnn>`: the `ShortName` of government `govt`'s
    /// highest active rank.
    #[must_use]
    pub fn short_of(&self, govt: GovtId) -> &str {
        self.highest(|rank| &rank.short_name, Some(govt))
    }

    /// `<PRKnnn>` as the Bible reads it, "same as `<PRK>`": the `ConvName`
    /// of government `govt`'s highest active rank.
    #[must_use]
    pub fn conversation_of(&self, govt: GovtId) -> &str {
        self.highest(|rank| &rank.conv_name, Some(govt))
    }

    /// `<RRK>`: the recent rank's full name, empty when it has no record.
    #[must_use]
    pub fn recent(&self) -> &str {
        match self.recent {
            Some(id) => record(self.records, id).map_or("", |rank| &rank.name),
            None => &self.fallback,
        }
    }

    /// `name_of` the active rank of highest `Weight`, of government
    /// `govt` when given, whose `name_of` is not empty, a tie going to the
    /// lowest ID; the fallback with none.
    fn highest(&self, name_of: fn(&RankRecord) -> &String, govt: Option<GovtId>) -> &str {
        let mut best: Option<&RankRecord> = None;
        for rank in self.records {
            if self.active.contains(&rank.id)
                && !name_of(rank).is_empty()
                && govt.is_none_or(|govt| rank.affil_govt == Some(govt))
                && best.is_none_or(|best| rank.weight > best.weight)
            {
                best = Some(rank);
            }
        }
        best.map_or(&self.fallback, |rank| name_of(rank))
    }
}

/// The pilot info's part for the ranks: the ranks line and the
/// salaries' share of the daily income.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PilotInfoRanks {
    /// The active ranks with a full name, joined (see the module docs);
    /// empty with none.
    pub line: String,
    /// Today's salaries, each tested against today's cash, with no floor.
    pub salary: i64,
    /// How many ranks the line lists.
    pub listed: usize,
}

/// The pilot info's ranks (see the module docs): the line of the ranks
/// of `records` in `active` with a full name, by `Weight` and then ID,
/// joined with `and` before the last of them and the `following`
/// Ranks-section items after them; and the salaries [`salary_paid`] gives
/// each on `cash` by `cap`, summed.
#[must_use]
pub fn pilot_info(
    records: &[RankRecord],
    active: &BTreeSet<RankId>,
    cash: i64,
    cap: RuleSource,
    and: &str,
    following: usize,
) -> PilotInfoRanks {
    let held = || records.iter().filter(|rank| active.contains(&rank.id));
    let mut named: Vec<&RankRecord> = held().filter(|rank| !rank.name.is_empty()).collect();
    named.sort_by_key(|rank| std::cmp::Reverse(rank.weight));
    let last = (named.len() + following).saturating_sub(1);
    let mut line = String::new();
    for (placed, rank) in (1..).zip(&named) {
        line.push_str(&rank.name);
        if placed < last {
            line.push_str(", ");
        } else if placed == last {
            line.push_str(", ");
            line.push_str(and);
            line.push(' ');
        }
    }
    let salary = held().map(|rank| salary_paid(rank, cash, cap)).sum();
    PilotInfoRanks {
        line,
        salary,
        listed: named.len(),
    }
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
    use crate::catalog::GovtRecord;
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
        let records = [
            rank(200, Some(128), 3, SWEEP_GOVT_ON_ACTIVATE),
            rank(201, Some(128), 2, 0),
        ];
        let mut state = State::holding(&[200, 201]);
        state.l(&records, 200);
        assert_eq!(state.active(), [201], "not without 0x20");
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

    /// Rank `id` of government `govt`, `weight`, full name `name`,
    /// `ConvName` `conv` and `ShortName` `short`.
    fn named(
        id: i16,
        govt: Option<i16>,
        weight: i16,
        [name, conv, short]: [&str; 3],
    ) -> RankRecord {
        RankRecord {
            name: name.to_owned(),
            conv_name: conv.to_owned(),
            short_name: short.to_owned(),
            ..rank(id, govt, weight, 0)
        }
    }

    /// The names `records` give with `active` held and `recent` the
    /// recent rank, falling back to "captain".
    fn names<'a>(
        records: &'a [RankRecord],
        active: &'a BTreeSet<RankId>,
        recent: Option<i16>,
    ) -> RankNames<'a> {
        RankNames {
            records,
            active,
            recent: recent.map(RankId),
            fallback: "captain".into(),
        }
    }

    fn ids(ids: &[i16]) -> BTreeSet<RankId> {
        ids.iter().copied().map(RankId).collect()
    }

    #[test]
    fn prk_is_the_highest_weight_active_ranks_conv_name() {
        let records = [
            named(200, Some(128), 1, ["Ensign", "Ensign", "Ens"]),
            named(201, Some(129), 7, ["Admiral", "Admiral Sir", "Adm"]),
            named(202, Some(128), 9, ["Lord", "Lord High", "Ld"]),
            named(203, None, -2, ["Low", "Lowly", "Lo"]),
        ];
        let active = ids(&[200, 201, 203]);
        let held = names(&records, &active, None);
        assert_eq!(held.conversation(), "Admiral Sir", "202 is not active");
        assert_eq!(held.short(), "Adm");
        let low = ids(&[203]);
        assert_eq!(names(&records, &low, None).short(), "Lo", "below 0");
    }

    #[test]
    fn a_weight_tie_goes_to_the_lowest_id() {
        let records = [
            named(200, Some(128), 1, ["", "Low", "L"]),
            named(201, Some(128), 5, ["", "First", "F"]),
            named(202, Some(129), 5, ["", "Second", "S"]),
        ];
        let active = ids(&[200, 201, 202]);
        let names = names(&records, &active, None);
        assert_eq!(names.conversation(), "First");
        assert_eq!(names.short(), "F");
    }

    #[test]
    fn a_rank_with_an_empty_conv_name_is_skipped_for_prk_but_counts_for_srk() {
        let records = [
            named(200, Some(128), 1, ["", "Ensign", "Ens"]),
            named(201, Some(128), 9, ["", "", "Adm"]),
            named(202, Some(128), 8, ["", "Captain", ""]),
        ];
        let active = ids(&[200, 201, 202]);
        let names = names(&records, &active, None);
        assert_eq!(names.conversation(), "Captain");
        assert_eq!(names.short(), "Adm");
    }

    #[test]
    fn with_no_named_active_rank_each_tag_reads_the_fallback() {
        let records = [
            named(200, Some(128), 1, ["", "", ""]),
            named(201, Some(128), 9, ["Lord", "Lord", "Ld"]),
        ];
        for active in [ids(&[]), ids(&[200]), ids(&[250])] {
            let names = names(&records, &active, None);
            assert_eq!(names.conversation(), "captain", "{active:?}");
            assert_eq!(names.short(), "captain", "{active:?}");
            assert_eq!(names.recent(), "captain", "{active:?}");
            assert_eq!(names.short_of(GovtId(128)), "captain", "{active:?}");
        }
        let none = RankNames::none();
        assert_eq!(none.conversation(), crate::person::NO_RANK);
        assert_eq!(none.short(), "captain");
        assert_eq!(none.recent(), "captain");
        assert_eq!(none.short_of(GovtId(128)), "captain");
    }

    #[test]
    fn rrk_is_the_recent_ranks_full_name_even_when_empty_and_the_fallback_with_none() {
        let records = [
            named(200, Some(128), 1, ["Commander", "Cmdr", "C"]),
            named(201, Some(128), 9, ["", "Lord", "Ld"]),
        ];
        let active = ids(&[200, 201]);
        assert_eq!(names(&records, &active, Some(200)).recent(), "Commander");
        assert_eq!(names(&records, &active, Some(201)).recent(), "", "empty");
        assert_eq!(
            names(&records, &ids(&[]), Some(200)).recent(),
            "Commander",
            "no active test (@0x9d35f-0x9d391)"
        );
        assert_eq!(
            names(&records, &active, Some(250)).recent(),
            "",
            "no record"
        );
        assert_eq!(names(&records, &active, None).recent(), "captain");
    }

    #[test]
    fn rrk_reads_the_fallback_after_l_clears_it() {
        let records = [named(200, Some(128), 1, ["Commander", "Cmdr", "C"])];
        let mut state = State::default();
        state.k(&records, 200);
        assert_eq!(
            names(&records, &state.active, state.recent.map(|id| id.0)).recent(),
            "Commander"
        );
        state.l(&records, 200);
        assert_eq!(
            names(&records, &state.active, state.recent.map(|id| id.0)).recent(),
            "captain"
        );
    }

    #[test]
    fn short_of_a_government_reads_its_highest_short_name_rank_and_never_an_unaffiliated_one() {
        let records = [
            named(200, Some(128), 1, ["", "Envoy", "Env"]),
            named(201, Some(128), 4, ["", "Ambassador Sir", "Ambassador"]),
            named(202, Some(128), 9, ["", "Lord", ""]),
            named(203, Some(129), 99, ["", "Admiral", "Adm"]),
            named(204, None, 99, ["", "Rogue", "Rg"]),
        ];
        let active = ids(&[200, 201, 202, 203, 204]);
        let names = names(&records, &active, None);
        assert_eq!(names.short_of(GovtId(128)), "Ambassador", "ShortName");
        assert_eq!(names.short_of(GovtId(129)), "Adm");
        assert_eq!(names.short_of(GovtId(130)), "captain");
        assert_eq!(names.short_of(GovtId(127)), "captain", "never -1's");
    }

    #[test]
    fn conversation_of_a_government_reads_its_highest_conv_name_rank() {
        let records = [
            named(200, Some(128), 1, ["", "Envoy", "Env"]),
            named(201, Some(128), 4, ["", "Ambassador Sir", "Ambassador"]),
            named(202, Some(128), 9, ["", "", "Lord"]),
            named(203, Some(129), 99, ["", "Admiral", "Adm"]),
            named(204, None, 99, ["", "Rogue", "Rg"]),
        ];
        let active = ids(&[200, 201, 202, 203, 204]);
        let names = names(&records, &active, None);
        assert_eq!(
            names.conversation_of(GovtId(128)),
            "Ambassador Sir",
            "ConvName, an empty one passed over"
        );
        assert_eq!(names.conversation_of(GovtId(129)), "Admiral");
        assert_eq!(names.conversation_of(GovtId(130)), "captain");
        assert_eq!(names.conversation_of(GovtId(127)), "captain", "never -1's");
    }

    /// Rank `id` of `weight` and full name `name`, unaffiliated.
    fn titled(id: i16, weight: i16, name: &str) -> RankRecord {
        RankRecord {
            name: name.to_owned(),
            ..rank(id, None, weight, 0)
        }
    }

    /// The ranks line of `records` with `active` held and `following`
    /// Ranks-section items after it.
    fn line(records: &[RankRecord], active: &[i16], following: usize) -> String {
        pilot_info(
            records,
            &ids(active),
            0,
            RuleSource::Engine,
            "and",
            following,
        )
        .line
    }

    #[test]
    fn the_ranks_line_lists_named_active_ranks_by_weight_then_id() {
        let records = [
            titled(200, 1, "Low"),
            titled(201, 9, "High"),
            titled(202, 5, "Mid A"),
            titled(203, 5, "Mid B"),
            titled(204, 99, "Inactive"),
            titled(205, -3, "Below"),
        ];
        assert_eq!(
            line(&records, &[200, 201, 202, 203, 205], 0),
            "High, Mid A, Mid B, Low, and Below"
        );
        assert_eq!(line(&records, &[], 0), "", "none held");
    }

    #[test]
    fn a_rank_with_an_empty_full_name_is_left_out() {
        let records = [
            titled(200, 1, "Low"),
            titled(201, 9, ""),
            titled(202, 5, "Mid"),
        ];
        assert_eq!(line(&records, &[200, 201, 202], 0), "Mid, and Low");
        assert_eq!(line(&records, &[201], 0), "");
        assert_eq!(line(&records, &[250], 0), "", "no record");
        let listed =
            |active| pilot_info(&records, &ids(active), 0, RuleSource::Engine, "", 0).listed;
        assert_eq!(listed(&[200, 201, 202]), 2, "the ranks placed");
        assert_eq!(listed(&[201, 250]), 0);
    }

    #[test]
    fn one_rank_stands_alone() {
        assert_eq!(line(&[titled(200, 1, "Commander")], &[200], 0), "Commander");
    }

    #[test]
    fn two_ranks_join_with_comma_and() {
        let records = [titled(200, 1, "A"), titled(201, 1, "B")];
        assert_eq!(line(&records, &[200, 201], 0), "A, and B");
    }

    #[test]
    fn three_or_more_join_with_commas_and_a_final_comma_and() {
        let records = [
            titled(200, 1, "A"),
            titled(201, 1, "B"),
            titled(202, 1, "C"),
            titled(203, 1, "D"),
        ];
        assert_eq!(line(&records, &[200, 201, 202], 0), "A, B, and C");
        assert_eq!(line(&records, &[200, 201, 202, 203], 0), "A, B, C, and D");
        let missing = pilot_info(&records, &ids(&[200, 201]), 0, RuleSource::Engine, "", 0);
        assert_eq!(missing.line, "A,  B", "a missing \"and\" appends nothing");
    }

    #[test]
    fn the_and_goes_before_the_last_item_counting_those_that_follow() {
        let records = [titled(200, 1, "A"), titled(201, 1, "B")];
        assert_eq!(line(&records, &[200], 1), "A, and ", "@0x54e5f-0x54ead");
        assert_eq!(line(&records, &[200, 201], 1), "A, B, and ");
        assert_eq!(line(&records, &[200], 2), "A, ");
        assert_eq!(line(&records, &[200, 201], 2), "A, B, ");
        assert_eq!(line(&records, &[200, 201], 5), "A, B, ");
        assert_eq!(line(&records, &[], 3), "", "no rank, no line");
    }

    #[test]
    fn the_salary_part_sums_the_active_salaries_under_the_cap_test_against_todays_cash() {
        let records = [
            paying(200, 200, 0),
            paying(201, 300, 1100),
            paying(202, -50, 0),
            paying(203, 400, 900),
            paying(204, 1000, 0),
        ];
        let active = ids(&[200, 201, 202, 203, 250]);
        let salary = |cash, source| pilot_info(&records, &active, cash, source, "and", 0).salary;
        assert_eq!(
            salary(1000, RuleSource::Engine),
            450,
            "each against 1000; 203 at its cap; 204 inactive"
        );
        assert_eq!(
            salary(1000, RuleSource::Bible),
            250,
            "201 pays up to its cap"
        );
        assert_eq!(salary(0, RuleSource::Engine), 850);
        let fee = pilot_info(
            &[paying(200, -50, 0)],
            &ids(&[200]),
            10,
            RuleSource::Engine,
            "and",
            0,
        );
        assert_eq!(fee.salary, -50, "no floor");
        assert_eq!(fee.line, "", "no name");
    }

    /// Govt 140 (G, class 1, allied with class 2), 141 (A, class 2) and
    /// 142 (U, class 3, allied with nobody).
    fn favour_govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..testkit::govt(140)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..testkit::govt(141)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..testkit::govt(142)
            },
        ])
    }

    /// The favour of active ranks `active` among `records` to a ship of
    /// `govt`, by `reach`.
    fn favour_of(
        records: &[RankRecord],
        active: &[i16],
        govt: Option<i16>,
        reach: RuleSource,
    ) -> Favour {
        favour(
            records,
            &ids(active),
            govt.map(GovtId),
            &favour_govts(),
            reach,
        )
    }

    const BOTH: Favour = Favour {
        battle_help: true,
        free_help: true,
    };

    #[test]
    fn the_hail_flags_are_0x0400_and_0x0800() {
        assert_eq!(HELPS_IN_BATTLE, 0x0400);
        assert_eq!(HELPS_FREE, 0x0800);
    }

    #[test]
    fn a_rank_reaches_its_own_and_allied_governments_by_the_engine() {
        let records = [
            rank(130, Some(140), 0, HELPS_IN_BATTLE),
            rank(131, Some(141), 0, HELPS_FREE),
            rank(132, Some(142), 0, HELPS_IN_BATTLE | HELPS_FREE),
        ];
        let engine = RuleSource::Engine;
        assert_eq!(favour_of(&records, &[130, 131], Some(140), engine), BOTH);
        assert_eq!(
            favour_of(&records, &[130], Some(140), engine),
            Favour {
                battle_help: true,
                free_help: false
            }
        );
        assert_eq!(
            favour_of(&records, &[131], Some(140), engine),
            Favour {
                battle_help: false,
                free_help: true
            }
        );
        assert_eq!(
            favour_of(&records, &[132], Some(140), engine),
            Favour::default()
        );
    }

    #[test]
    fn a_rank_reaches_only_its_own_government_by_the_bible() {
        let records = [
            rank(130, Some(140), 0, HELPS_IN_BATTLE | HELPS_FREE),
            rank(131, Some(141), 0, HELPS_IN_BATTLE | HELPS_FREE),
            rank(132, None, 0, HELPS_IN_BATTLE | HELPS_FREE),
        ];
        let bible = RuleSource::Bible;
        assert_eq!(favour_of(&records, &[130], Some(140), bible), BOTH);
        assert_eq!(
            favour_of(&records, &[131], Some(140), bible),
            Favour::default()
        );
        assert_eq!(favour_of(&records, &[132], None, RuleSource::Engine), BOTH);
        assert_eq!(favour_of(&records, &[132], None, bible), Favour::default());
        assert_eq!(
            favour_of(&records, &[132], Some(140), RuleSource::Engine),
            Favour::default()
        );
    }

    #[test]
    fn only_active_ranks_with_a_record_favour() {
        let records = [rank(130, Some(140), 0, HELPS_IN_BATTLE | HELPS_FREE)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                favour_of(&records, &[], Some(140), reach),
                Favour::default()
            );
            assert_eq!(
                favour_of(&records, &[131], Some(140), reach),
                Favour::default()
            );
            assert_eq!(favour_of(&records, &[130], Some(140), reach), BOTH);
        }
    }

    #[test]
    fn two_ranks_carrying_the_same_flag_still_give_it() {
        let both = HELPS_IN_BATTLE | HELPS_FREE;
        let records = [rank(130, Some(140), 0, both), rank(131, Some(141), 0, both)];
        assert_eq!(
            favour_of(&records, &[130, 131], Some(140), RuleSource::Engine),
            BOTH
        );
    }

    #[test]
    fn battle_help_and_free_help_are_separate_flags() {
        let records = [
            rank(130, Some(140), 0, HELPS_IN_BATTLE),
            rank(131, Some(140), 0, HELPS_FREE),
            rank(132, Some(140), 0, 0x0001 | 0x0008 | 0x0100),
        ];
        let engine = RuleSource::Engine;
        assert_eq!(
            favour_of(&records, &[130], Some(140), engine),
            Favour {
                battle_help: true,
                free_help: false
            }
        );
        assert_eq!(
            favour_of(&records, &[131], Some(140), engine),
            Favour {
                battle_help: false,
                free_help: true
            }
        );
        assert_eq!(
            favour_of(&records, &[132], Some(140), engine),
            Favour::default()
        );
    }

    #[test]
    fn bits_two_ranks_give_are_held_once() {
        let records = [
            RankRecord {
                contribute: 0x5,
                ..testkit::rank(200)
            },
            RankRecord {
                contribute: 0x6,
                ..testkit::rank(201)
            },
        ];
        assert_eq!(contributed(&records, &ids(&[200, 201])), 0x7);
        assert_eq!(contributed(&records, &ids(&[201])), 0x6);
    }

    /// Rank `id` of government `govt` (`None` unaffiliated) and
    /// `PriceMod` `price_mod`.
    fn pricing(id: i16, govt: Option<i16>, price_mod: i16) -> RankRecord {
        RankRecord {
            price_mod,
            ..rank(id, govt, 0, 0)
        }
    }

    /// The multipliers of active ranks `active` among `records` at a
    /// stellar of `govt`, by `reach`, among [`favour_govts`].
    fn mults_of(
        records: &[RankRecord],
        active: &[i16],
        govt: Option<i16>,
        reach: RuleSource,
    ) -> PriceMults {
        price_mults(
            records,
            &ids(active),
            govt.map(GovtId),
            &favour_govts(),
            reach,
        )
    }

    /// Both multipliers at `mult`.
    fn both(mult: f32) -> PriceMults {
        PriceMults {
            first: mult,
            second: mult,
        }
    }

    #[test]
    fn no_rank_leaves_both_prices_at_1() {
        assert_eq!(PriceMults::NONE, both(1.0));
        assert_eq!(PriceMults::default(), PriceMults::NONE);
        let records = [pricing(130, Some(140), 85)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(mults_of(&records, &[], Some(140), reach), PriceMults::NONE);
        }
    }

    #[test]
    fn a_rank_of_the_stellars_government_multiplies_both_by_its_price_mod() {
        let records = [pricing(130, Some(140), 85)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(mults_of(&records, &[130], Some(140), reach), both(0.85));
        }
        let dearer = [pricing(130, Some(140), 150)];
        assert_eq!(
            mults_of(&dearer, &[130], Some(140), RuleSource::Engine),
            both(1.5)
        );
    }

    #[test]
    fn ranks_stack_as_the_original_multiplies_in_double_and_rounds_to_single() {
        let records = [
            pricing(130, Some(140), 90),
            pricing(131, Some(140), 90),
            pricing(132, Some(140), 60),
            pricing(133, Some(140), 85),
        ];
        let engine = RuleSource::Engine;
        let two_nineties = mults_of(&records, &[130, 131], Some(140), engine);
        assert_eq!(two_nineties, both(0.81), "@0x5f9c4-0x5f9f6");
        assert_ne!(
            (0.9_f32 * 0.9_f32).to_bits(),
            0.81_f32.to_bits(),
            "not in single precision"
        );
        let mixed = mults_of(&records, &[132, 133], Some(140), engine);
        assert_eq!(mixed, both(0.51));
        assert_ne!(
            (0.6_f32 * 0.85_f32).to_bits(),
            0.51_f32.to_bits(),
            "not in single precision"
        );
    }

    #[test]
    fn only_active_ranks_with_a_record_count() {
        let records = [pricing(130, Some(140), 90), pricing(131, Some(140), 80)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(mults_of(&records, &[131], Some(140), reach), both(0.8));
            assert_eq!(
                mults_of(&records, &[150], Some(140), reach),
                PriceMults::NONE,
                "no record"
            );
        }
    }

    #[test]
    fn by_the_engine_a_rank_of_an_allied_government_counts_and_by_the_other_reading_only_its_own() {
        let records = [pricing(130, Some(141), 90), pricing(131, Some(142), 80)];
        assert_eq!(
            mults_of(&records, &[130, 131], Some(140), RuleSource::Engine),
            both(0.9),
            "141 is allied with 140; 142 is not"
        );
        assert_eq!(
            mults_of(&records, &[130, 131], Some(140), RuleSource::Bible),
            PriceMults::NONE
        );
        assert_eq!(
            mults_of(&records, &[130, 131], Some(141), RuleSource::Bible),
            both(0.9)
        );
    }

    #[test]
    fn an_independent_stellar_takes_no_rank() {
        let records = [pricing(130, None, 90), pricing(131, Some(140), 80)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                mults_of(&records, &[130, 131], None, reach),
                PriceMults::NONE,
                "@0x5f98b-0x5f990"
            );
        }
        assert!(
            favour_of(
                &[rank(130, None, 0, HELPS_FREE)],
                &[130],
                None,
                RuleSource::Engine
            )
            .free_help,
            "where the hail's favour reaches"
        );
    }

    #[test]
    fn a_price_scaled_by_the_rank_factor_is_none_for_none_and_at_least_1() {
        assert_eq!(scaled(0, 0.9), 0);
        assert_eq!(scaled(-5, 0.9), 0);
        assert_eq!(scaled(1, 0.5), 1);
        assert_eq!(scaled(10_000, 0.9), 9000, "in single precision");
        assert_eq!(scaled(1500, 0.85), 1275);
        assert_eq!(scaled(1500, 1.0), 1500);
    }

    #[test]
    fn an_unaffiliated_rank_does_not_count_at_a_governed_stellar() {
        let records = [pricing(130, None, 90)];
        for reach in [RuleSource::Engine, RuleSource::Bible] {
            assert_eq!(
                mults_of(&records, &[130], Some(140), reach),
                PriceMults::NONE
            );
        }
    }

    /// [`favour_govts`] and govt 143, derelict.
    fn crime_govts() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [2, -1, -1, -1],
                ..testkit::govt(140)
            },
            GovtRecord {
                classes: [2, -1, -1, -1],
                ..testkit::govt(141)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                ..testkit::govt(142)
            },
            GovtRecord {
                flags: crate::govt::DERELICT,
                ..testkit::govt(143)
            },
        ])
    }

    impl State {
        /// The ranks `crime` against a ship of `victim` takes, by `reach`
        /// and the engine's grouping, among [`crime_govts`].
        fn lose(
            &mut self,
            records: &[RankRecord],
            crime: Crime,
            victim: Option<i16>,
            reach: RuleSource,
        ) -> &mut Self {
            lose_to_crime(
                records,
                self.held(),
                crime,
                victim.map(GovtId),
                &crime_govts(),
                reach,
                RuleSource::Engine,
            );
            self
        }
    }

    /// Whether a lone active rank `record` survives `crime` against a ship
    /// of `victim` by `reach`.
    fn kept(record: RankRecord, crime: Crime, victim: Option<i16>, reach: RuleSource) -> bool {
        let id = record.id.0;
        let mut state = State::holding(&[id]);
        state.lose(&[record], crime, victim, reach);
        state.active() == [id]
    }

    const ENGINE: RuleSource = RuleSource::Engine;

    #[test]
    fn the_crime_flags_are_0x04_and_0x40() {
        assert_eq!(LOST_TO_VIOLENCE, 0x0004);
        assert_eq!(LOST_TO_CRIME, 0x0040);
    }

    #[test]
    fn flag_0x40_takes_a_rank_on_disabling_boarding_and_destroying() {
        for crime in [Crime::Disable, Crime::Board, Crime::Kill] {
            let mut state = State::holding(&[130]);
            state.recent = Some(RankId(130));
            state.lose(
                &[rank(130, Some(140), 0, LOST_TO_CRIME)],
                crime,
                Some(140),
                ENGINE,
            );
            assert_eq!(state, State::default(), "{crime:?}");
        }
    }

    /// Contraband a scan finds is type 0 (`_ScanPlayer` @0x7ebcc,
    /// @0x7f01f): 0x0040 takes on any type (@0x9c81), 0x0004 only on
    /// types 1 and 3 (@0x9c89-0x9c97).
    #[test]
    fn smuggling_takes_a_0x40_rank_and_leaves_a_0x04_one_by_either_reach() {
        for reach in [ENGINE, RuleSource::Bible] {
            assert!(
                !kept(
                    rank(130, Some(140), 0, LOST_TO_CRIME),
                    Crime::Smuggle,
                    Some(140),
                    reach
                ),
                "{reach:?}"
            );
            assert!(
                kept(
                    rank(130, Some(140), 0, LOST_TO_VIOLENCE),
                    Crime::Smuggle,
                    Some(140),
                    reach
                ),
                "{reach:?}"
            );
            assert!(
                kept(
                    rank(130, Some(143), 0, LOST_TO_CRIME),
                    Crime::Smuggle,
                    Some(143),
                    reach
                ),
                "a derelict scanner: {reach:?}"
            );
        }
    }

    #[test]
    fn shooting_takes_no_rank() {
        let flags = LOST_TO_CRIME | LOST_TO_VIOLENCE;
        assert!(kept(
            rank(130, Some(140), 0, flags),
            Crime::Shoot,
            Some(140),
            ENGINE
        ));
        assert!(kept(
            rank(130, Some(140), 0, flags),
            Crime::Shoot,
            Some(140),
            RuleSource::Bible
        ));
    }

    #[test]
    fn flag_0x04_takes_a_rank_only_on_disabling_and_destroying() {
        let violence = rank(130, Some(140), 0, LOST_TO_VIOLENCE);
        assert!(kept(violence.clone(), Crime::Board, Some(140), ENGINE));
        assert!(!kept(violence.clone(), Crime::Disable, Some(140), ENGINE));
        assert!(!kept(violence, Crime::Kill, Some(140), ENGINE));
    }

    #[test]
    fn a_rank_with_neither_flag_is_not_lost_to_a_crime() {
        let others = !(LOST_TO_CRIME | LOST_TO_VIOLENCE);
        for crime in [Crime::Disable, Crime::Board, Crime::Kill] {
            assert!(kept(
                rank(130, Some(140), 0, others),
                crime,
                Some(140),
                ENGINE
            ));
        }
    }

    #[test]
    fn flag_0x08_shields_a_rank_from_crimes() {
        let flags = LOST_TO_CRIME | LOST_TO_VIOLENCE | SHIELDED;
        for reach in [ENGINE, RuleSource::Bible] {
            assert!(kept(
                rank(130, Some(140), 0, flags),
                Crime::Kill,
                Some(140),
                reach
            ));
        }
    }

    #[test]
    fn by_the_engine_a_crime_reaches_ranks_allied_with_the_victims_government() {
        for flags in [LOST_TO_CRIME, LOST_TO_VIOLENCE] {
            assert!(!kept(
                rank(130, Some(140), 0, flags),
                Crime::Kill,
                Some(141),
                ENGINE
            ));
            assert!(!kept(
                rank(131, Some(141), 0, flags),
                Crime::Kill,
                Some(140),
                ENGINE
            ));
            assert!(kept(
                rank(132, Some(142), 0, flags),
                Crime::Kill,
                Some(140),
                ENGINE
            ));
            assert!(kept(
                rank(130, Some(140), 0, flags),
                Crime::Kill,
                Some(142),
                ENGINE
            ));
        }
    }

    #[test]
    fn an_independent_victim_takes_only_an_unaffiliated_rank_by_the_engine() {
        for flags in [LOST_TO_CRIME, LOST_TO_VIOLENCE] {
            assert!(kept(
                rank(130, Some(140), 0, flags),
                Crime::Kill,
                None,
                ENGINE
            ));
            assert!(
                !kept(rank(130, None, 0, flags), Crime::Kill, None, ENGINE),
                "@0x4e4e"
            );
            assert!(kept(
                rank(130, None, 0, flags),
                Crime::Kill,
                Some(140),
                ENGINE
            ));
        }
    }

    #[test]
    fn a_derelict_victim_takes_no_rank() {
        for reach in [ENGINE, RuleSource::Bible] {
            assert!(
                kept(
                    rank(130, Some(143), 0, LOST_TO_CRIME),
                    Crime::Kill,
                    Some(143),
                    reach
                ),
                "@0x9bf2"
            );
        }
    }

    #[test]
    fn a_lost_rank_sweeps_as_l_does() {
        let records = [
            rank(130, Some(140), 0, LOST_TO_CRIME | SWEEP_GOVT_ON_DEACTIVATE),
            rank(131, Some(140), 9, 0),
            rank(132, Some(140), 9, SHIELDED),
            rank(133, Some(142), 9, 0),
        ];
        let mut state = State::holding(&[130, 131, 132, 133]);
        state.recent = Some(RankId(131));
        state.lose(&records, Crime::Board, Some(140), ENGINE);
        assert_eq!(state.active(), [132, 133]);
        assert_eq!(state.recent, None);
    }

    #[test]
    fn ranks_are_lost_in_id_order_and_a_swept_rank_sweeps_nothing() {
        let records = [
            rank(130, Some(140), 5, LOST_TO_CRIME | SWEEP_LOWER_ON_DEACTIVATE),
            rank(140, Some(140), 3, LOST_TO_CRIME | SWEEP_GOVT_ON_DEACTIVATE),
            rank(145, Some(140), 9, 0),
        ];
        let mut state = State::holding(&[130, 140, 145]);
        state.lose(&records, Crime::Kill, Some(140), ENGINE);
        assert_eq!(state.active(), [145]);
    }

    #[test]
    fn an_active_rank_with_no_record_is_untouched() {
        let mut state = State::holding(&[130]);
        state.recent = Some(RankId(130));
        state.lose(
            &[rank(131, None, 0, LOST_TO_CRIME)],
            Crime::Kill,
            None,
            ENGINE,
        );
        assert_eq!(state.active(), [130]);
        assert_eq!(state.recent, Some(RankId(130)));
    }

    #[test]
    fn an_inactive_rank_is_not_lost_and_leaves_the_recent_rank() {
        let mut state = State::holding(&[131]);
        state.recent = Some(RankId(130));
        state.lose(
            &[rank(130, Some(140), 0, LOST_TO_CRIME)],
            Crime::Kill,
            Some(140),
            ENGINE,
        );
        assert_eq!(state.active(), [131]);
        assert_eq!(state.recent, Some(RankId(130)), "@0x9c56");
    }

    #[test]
    fn by_the_other_reading_0x40_takes_only_its_own_government_and_0x04_its_allies_too() {
        let bible = RuleSource::Bible;
        let crime = rank(130, Some(140), 0, LOST_TO_CRIME);
        assert!(!kept(crime.clone(), Crime::Board, Some(140), bible));
        assert!(kept(crime.clone(), Crime::Kill, Some(141), bible));
        let violence = rank(130, Some(140), 0, LOST_TO_VIOLENCE);
        assert!(!kept(violence.clone(), Crime::Kill, Some(140), bible));
        assert!(!kept(violence.clone(), Crime::Disable, Some(141), bible));
        assert!(kept(violence, Crime::Kill, Some(142), bible));
        let both = rank(130, Some(140), 0, LOST_TO_CRIME | LOST_TO_VIOLENCE);
        assert!(!kept(both.clone(), Crime::Disable, Some(141), bible));
        assert!(kept(both.clone(), Crime::Board, Some(141), bible));
        assert!(!kept(both, Crime::Board, Some(140), bible));
        for flags in [LOST_TO_CRIME, LOST_TO_VIOLENCE] {
            assert!(kept(rank(130, None, 0, flags), Crime::Kill, None, bible));
        }
    }

    // Blessing.

    /// The blessing of active ranks `active` among `records`, by `reach`,
    /// among [`crime_govts`].
    fn blessing_of(records: &[RankRecord], active: &[i16], reach: RuleSource) -> Blessing {
        blessing(records, &ids(active), &crime_govts(), reach)
    }

    /// Which of 140-143 `blessed` blesses `which`.
    fn blessed_of(blessed: Blessing, which: Blessed) -> Vec<i16> {
        (140..=143)
            .filter(|&id| blessed.blesses(Some(GovtId(id)), which))
            .collect()
    }

    #[test]
    fn the_blessing_flags_are_0x0100_and_0x0200() {
        assert_eq!(BLESSES_PEACE, 0x0100);
        assert_eq!(BLESSES_LANDING, 0x0200);
    }

    #[test]
    fn a_rank_of_flags_0x0200_blesses_landing_at_its_government_and_its_allies() {
        let records = [rank(130, Some(140), 0, BLESSES_LANDING)];
        let blessed = blessing_of(&records, &[130], ENGINE);
        assert_eq!(blessed_of(blessed, Blessed::Landing), [140, 141], "@0xc94f");
        assert_eq!(blessed_of(blessed, Blessed::Peace), [] as [i16; 0]);
        let ally = [rank(130, Some(141), 0, BLESSES_LANDING)];
        assert_eq!(
            blessed_of(blessing_of(&ally, &[130], ENGINE), Blessed::Landing),
            [140, 141],
            "either way round"
        );
    }

    #[test]
    fn a_rank_of_flags_0x0100_blesses_peace_only() {
        let records = [rank(130, Some(140), 0, BLESSES_PEACE)];
        let blessed = blessing_of(&records, &[130], ENGINE);
        assert_eq!(blessed_of(blessed, Blessed::Peace), [140, 141], "@0xc936");
        assert_eq!(blessed_of(blessed, Blessed::Landing), [] as [i16; 0]);
        let both = [rank(130, Some(142), 0, BLESSES_PEACE | BLESSES_LANDING)];
        let blessed = blessing_of(&both, &[130], ENGINE);
        assert_eq!(blessed_of(blessed, Blessed::Peace), [142]);
        assert_eq!(blessed_of(blessed, Blessed::Landing), [142]);
    }

    #[test]
    fn ranks_add_their_blessings_together() {
        let records = [
            rank(130, Some(140), 0, BLESSES_PEACE),
            rank(131, Some(142), 0, BLESSES_LANDING),
            rank(132, Some(142), 0, HELPS_FREE | SHIELDED),
        ];
        let blessed = blessing_of(&records, &[130, 131, 132], ENGINE);
        assert_eq!(blessed_of(blessed, Blessed::Peace), [140, 141]);
        assert_eq!(blessed_of(blessed, Blessed::Landing), [142]);
    }

    #[test]
    fn an_unaffiliated_rank_blesses_no_government() {
        let records = [rank(130, None, 0, BLESSES_PEACE | BLESSES_LANDING)];
        for reach in [ENGINE, RuleSource::Bible] {
            let blessed = blessing_of(&records, &[130], reach);
            assert_eq!(blessed, Blessing::NONE, "@0xc90a {reach:?}");
            for which in [Blessed::Peace, Blessed::Landing] {
                assert!(!blessed.blesses(None, which), "{reach:?}");
            }
        }
    }

    #[test]
    fn an_inactive_rank_or_one_without_a_record_blesses_nothing() {
        let records = [rank(130, Some(140), 0, BLESSES_PEACE | BLESSES_LANDING)];
        assert_eq!(blessing_of(&records, &[], ENGINE), Blessing::NONE);
        assert_eq!(blessing_of(&records, &[131], ENGINE), Blessing::NONE);
        assert_eq!(Blessing::default(), Blessing::NONE);
    }

    #[test]
    fn a_derelict_government_is_blessed_only_by_its_own_rank() {
        let records = [rank(130, Some(143), 0, BLESSES_LANDING)];
        assert_eq!(
            blessed_of(blessing_of(&records, &[130], ENGINE), Blessed::Landing),
            [143],
            "@0x4e4e"
        );
        let govts = Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                allies: [5, -1, -1, -1],
                ..testkit::govt(140)
            },
            GovtRecord {
                flags: crate::govt::DERELICT,
                classes: [5, -1, -1, -1],
                ..testkit::govt(143)
            },
        ]);
        let of_140 = [rank(130, Some(140), 0, BLESSES_LANDING)];
        let blessed = blessing(&of_140, &ids(&[130]), &govts, ENGINE);
        assert!(blessed.blesses(Some(GovtId(140)), Blessed::Landing));
        assert!(
            !blessed.blesses(Some(GovtId(143)), Blessed::Landing),
            "@0x4e91"
        );
    }

    #[test]
    fn a_rank_blesses_its_own_government_even_where_the_table_lacks_it() {
        let records = [rank(130, Some(200), 0, BLESSES_PEACE)];
        let blessed = blessing_of(&records, &[130], ENGINE);
        assert!(blessed.blesses(Some(GovtId(200)), Blessed::Peace));
        assert!(!blessed.blesses(Some(GovtId(201)), Blessed::Peace));
    }

    #[test]
    fn by_the_other_reading_of_rank_govt_reach_only_the_ranks_own_government_is_blessed() {
        let records = [rank(130, Some(140), 0, BLESSES_PEACE | BLESSES_LANDING)];
        let blessed = blessing_of(&records, &[130], RuleSource::Bible);
        assert_eq!(blessed_of(blessed, Blessed::Landing), [140]);
        assert_eq!(blessed_of(blessed, Blessed::Peace), [140]);
    }

    #[test]
    fn a_government_outside_128_to_383_is_never_blessed() {
        let govts = Governments::new([127, 128, 383, 384].map(testkit::govt));
        let records: Vec<RankRecord> = [127, 128, 383, 384]
            .into_iter()
            .map(|govt| rank(govt, Some(govt), 0, BLESSES_PEACE | BLESSES_LANDING))
            .collect();
        let blessed = blessing(&records, &ids(&[127, 128, 383, 384]), &govts, ENGINE);
        for which in [Blessed::Peace, Blessed::Landing] {
            let held: Vec<i16> = [127, 128, 255, 256, 383, 384, -1, i16::MAX, i16::MIN]
                .into_iter()
                .filter(|&id| blessed.blesses(Some(GovtId(id)), which))
                .collect();
            assert_eq!(held, [128, 383], "{which:?}");
        }
    }
}
