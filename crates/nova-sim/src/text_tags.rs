//! The text tags: the one substitution path for every text the game
//! fills in (see the [`story`](crate::story) module's "Text
//! substitution"), as the original's `_MungeBriefing(offer, index)`
//! (@0x9c11d) is.
//!
//! **The original.** `_MungeBriefing` starts every value as `"[Error]"`
//! ([`UNSET`], 0xdb190) and replaces the tags of its table (0xdb198)
//! one at a time, in order, each with `_MungeForSymbol` (@0x984a3), which
//! calls `_Munger` from the start until no match is left: every
//! occurrence, case-sensitively, so a value naming a later tag has it
//! filled too. Its mission tags read an offer's values
//! (`_briefMissionInfo`, 0x35d800) with `offer` set, else active slot
//! `index`'s (`_currentMissions`, 0x184688); a free slot's, or index -1's,
//! stay `"[Error]"`.
//!
//! | Tag | What it reads |
//! |---|---|
//! | `<DST>`, `<DSY>` | the destination stellar's name and its system's; each `"[Error]"` takes `<RST>`'s or `<RSY>`'s on its own (@0x9c53a-0x9c5fc, @0x9caa4-0x9cb66) |
//! | `<RST>`, `<RSY>` | the return stellar's name and its system's |
//! | `<CT>` | for a cargo type of 0-255, `STR#` 4001's string type + 1, a leading `*` dropped (@0x9c65f-0x9c6bf) |
//! | `<CQ>` | the tons in plain digits, with a cargo type of 0-255 (@0x9c6c1-0x9c710) |
//! | `<SN>` | a slot's ship name (+0x70, @0x9ccf1); an offer has none |
//! | `<DL>` | the deadline, written long ([`long_date_text`]); one of today is `"[Error]"` (@0x9c715-0x9c78e) |
//! | `<PN>`, `<PNN>` | the pilot's name; its nickname, or the name with none (@0x9cf2d-0x9cfb6) |
//! | `<PSN>`, `<PST>` | the player's ship's name and its class's (@0x9cfbb-0x9d034) |
//! | `<OSN>` | the name of the person offering (@0x9cebe-0x9cf28) |
//! | `<PRK>`, `<SRK>`, `<RRK>` | the ranks' names, "captain" with none (@0x9d039-0x9d3b3) |
//! | `<PAY>` | the amount [`pay_shown`] gives, written by [`comma_number`]; below 0 `"[Error]"` (@0x9c7a0-0x9c835, @0x9cd2a-0x9cdb6) |
//! | `<REG>` | the registration's licence name (`_RT3_GetDisplayName` @0xd1517) |
//! | `<PRKnnn>`, `<SRKnnn>` | government nnn's highest active rank's `ShortName` (@0x9d1aa-0x9d35a) |
//!
//! **Here.** [`expand`] fills a text from a [`TagContext`], its names
//! read through the [`TagNames`] port. A context's mission is an offer's
//! ([`MissionValues::offered`]) or a slot's ([`MissionValues::active`]),
//! resolved, never the `mïsn`'s encodings; a free slot's is the default,
//! every mission tag [`UNSET`]. With no mission at all, as in a person's
//! hail quote ([`expand_tags`](crate::person::expand_tags)), the mission
//! tags are left as written. Where the engine reads otherwise:
//!
//! - `<DSY>` and `<RSY>` read the system the stop resolved to
//!   ([`Stop::system`]), the lowest visible one listing it, rather than
//!   the original's co-located search.
//! - There is no nickname, so `<PNN>` is the pilot's name; and no
//!   licence, so `<REG>` is empty.
//! - `<SN>` reads a slot's ship name, which stays empty until
//!   rdm:phase/mission-ships/phase-1-spawn-mission-ships draws one.
//! - The rank tags read "captain" ([`NO_RANK`]) until
//!   rdm:roadmap/crons-news-ranks: `<PRKnnn>` and `<SRKnnn>` for nnn
//!   128-383, any other nnn left as written.
//! - `<PAY>` of a million or more follows [`RuleKey::PayMillions`](crate::RuleKey::PayMillions).
//!
//! The original's quirks not reproduced here (a slot's `<PAY>` reading
//! its byte offset, the crossed `<PRKnnn>`/`<SRKnnn>` governments, a stale
//! `<OSN>`, the `l33t` mode, `'STR '` 9100+ cargo name patches, and
//! `"[Error]"` for the mission tags with no mission) are
//! rdm:phase/missions/phase-13-text-substitution-quirks.

use crate::catalog::{DateAffixes, StellarId, SystemId};
use crate::date::{GameDate, long_date_text};
use crate::mission::{ActiveMission, Offer, Stop};
use crate::person::NO_RANK;
use crate::rulebook::RuleSource;

/// What a tag reads that has no value: every value's start
/// (0xdb190, @0x9c1a7-0x9c37b).
pub const UNSET: &str = "[Error]";

/// `PayVal` from which a pay fines a percent of the cash: -40001 to
/// -40099, the percent `-pay - 40000`.
const FINES: std::ops::RangeInclusive<i64> = -40_099..=-40_001;
/// `PayVal` below which a pay is cash taken at accept: `-50000 - pay`.
const CHARGES_BELOW: i64 = -50_000;

/// The amount `<PAY>` shows for a pay of `pay` with `cash` held
/// (@0x9c7a0-0x9c835, @0x9cd2a-0x9cdb6): above 0, the pay; -40099 to
/// -40001, its percent of the cash, worked in doubles and truncated as
/// the original does (trunc(cash × (percent × 0.01)), 0.01 @0xdd098);
/// below -50000, the cash taken at accept; any other pay, 0. `None` for
/// an amount below 0, which the tag shows as [`UNSET`].
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn pay_shown(pay: i64, cash: i64) -> Option<i64> {
    let amount = match pay {
        1.. => pay,
        pay if FINES.contains(&pay) => {
            let percent = (-pay - 40_000) as f64;
            (cash as f64 * (percent * 0.01)).trunc() as i64
        }
        pay if pay < CHARGES_BELOW => CHARGES_BELOW - pay,
        _ => 0,
    };
    (amount >= 0).then_some(amount)
}

/// `n` grouped by threes with commas: "1,234,567".
fn with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() * 4 / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// `n` as the original's `_StrcatCommaNumber` (@0x8820) writes it: plain
/// below 1000, and "1,234" below a million. From a million,
/// [`RuleKey::PayMillions`](crate::RuleKey::PayMillions) chooses: by the
/// engine, in millions to two places, truncated, "1.23M"
/// (@0x8990-0x8aa3); by the other reading, with its commas, "1,234,567".
#[must_use]
pub fn comma_number(n: i64, millions: RuleSource) -> String {
    match n {
        ..1000 => n.to_string(),
        1_000_000.. if millions == RuleSource::Engine => {
            format!("{}.{:02}M", n / 1_000_000, n % 1_000_000 / 10_000)
        }
        _ => with_commas(n.unsigned_abs()),
    }
}

/// The names the tags read, in the core's terms.
pub trait TagNames {
    /// Stellar `stellar`'s name, if there is one.
    fn stellar_name(&self, stellar: StellarId) -> Option<String>;
    /// System `system`'s name, if there is one.
    fn system_name(&self, system: SystemId) -> Option<String>;
    /// Cargo type `cargo`'s name, raw: `STR#` 4001's string `cargo` + 1
    /// (`_lcCargoName`, 0x1b8de0), `None` past the list's end.
    fn cargo_name(&self, cargo: u8) -> Option<String>;
}

/// No names at all: for a text with no mission, whose tags read none.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoNames;

impl TagNames for NoNames {
    fn stellar_name(&self, _stellar: StellarId) -> Option<String> {
        None
    }

    fn system_name(&self, _system: SystemId) -> Option<String> {
        None
    }

    fn cargo_name(&self, _cargo: u8) -> Option<String> {
        None
    }
}

/// A mission's values that its tags read: an offer's or an active
/// slot's, resolved. Each one missing reads [`UNSET`], so the default
/// is a free slot's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MissionValues {
    /// Where it goes: `<DST>`, `<DSY>`.
    pub destination: Option<Stop>,
    /// Where it is completed: `<RST>`, `<RSY>`.
    pub return_to: Option<Stop>,
    /// The cargo type, raw: `<CT>`.
    pub cargo_type: Option<i16>,
    /// The tons it carries: `<CQ>`.
    pub cargo_tons: i64,
    /// Its pay, `PayVal` raw: `<PAY>`.
    pub pay: Option<i64>,
    /// The last day it may be completed: `<DL>`.
    pub deadline: Option<GameDate>,
    /// Its ship's name: `<SN>`.
    pub ship_name: Option<String>,
}

impl MissionValues {
    /// `offer`'s values (`_briefMissionInfo`, 0x35d800), with no ship
    /// name: an offer's `<SN>` reads [`UNSET`].
    #[must_use]
    pub fn offered(offer: &Offer) -> Self {
        Self {
            destination: offer.destination,
            return_to: offer.return_to,
            cargo_type: offer.cargo_type,
            cargo_tons: offer.cargo_tons,
            pay: Some(offer.pay),
            deadline: offer.deadline,
            ship_name: None,
        }
    }

    /// `active`'s values (`_currentMissions`, 0x184688). Its ship name
    /// (slot +0x70) is empty until mission ships draw one at accept
    /// (rdm:phase/mission-ships/phase-1-spawn-mission-ships).
    #[must_use]
    pub fn active(active: &ActiveMission) -> Self {
        Self {
            destination: active.destination,
            return_to: active.return_to,
            cargo_type: active.cargo_type,
            cargo_tons: active.cargo_tons,
            pay: Some(active.pay),
            deadline: active.deadline,
            ship_name: Some(String::new()),
        }
    }
}

/// What a text's mission tags read: the mission's values, and what
/// `<DL>` and `<PAY>` are written against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionTags<'a> {
    /// The mission's values.
    pub values: MissionValues,
    /// The player's cash, which a fining pay is a percent of.
    pub cash: i64,
    /// Today: a deadline of today reads [`UNSET`].
    pub today: GameDate,
    /// What the deadline is wrapped in.
    pub date_affixes: &'a DateAffixes,
    /// How a pay of a million or more is written
    /// ([`RuleKey::PayMillions`](crate::RuleKey::PayMillions)).
    pub pay_millions: RuleSource,
}

/// What a text's tags read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagContext<'a> {
    /// The pilot's name: `<PN>`, and `<PNN>`, as there is no nickname.
    pub pilot: &'a str,
    /// The player's ship class's name: `<PST>`.
    pub ship_type: &'a str,
    /// The player's ship's name: `<PSN>`.
    pub ship_name: &'a str,
    /// The person speaking, if any: `<OSN>`, [`UNSET`] with none.
    pub person: Option<&'a str>,
    /// The mission, if the text has one; with none its tags are left as
    /// written.
    pub mission: Option<MissionTags<'a>>,
}

/// `stop`'s stellar's and system's names, [`UNSET`] for each missing.
fn stop_names(stop: Option<Stop>, names: &(impl TagNames + ?Sized)) -> (String, String) {
    let stellar = stop.and_then(|stop| names.stellar_name(stop.stellar));
    let system = stop
        .and_then(|stop| stop.system)
        .and_then(|system| names.system_name(system));
    let unset = |name: Option<String>| name.unwrap_or_else(|| UNSET.to_owned());
    (unset(stellar), unset(system))
}

/// `value`, or `fallback` when `value` is [`UNSET`].
fn or_else(value: String, fallback: &str) -> String {
    if value == UNSET {
        fallback.to_owned()
    } else {
        value
    }
}

/// The mission tags' values, in the table's order (@0x9c38a-0x9cdb6).
fn mission_values(
    mission: &MissionTags,
    names: &(impl TagNames + ?Sized),
) -> [(&'static str, String); 9] {
    let values = &mission.values;
    let unset = || UNSET.to_owned();
    let (rst, rsy) = stop_names(values.return_to, names);
    let (dst, dsy) = stop_names(values.destination, names);
    let cargo = values.cargo_type.and_then(|cargo| u8::try_from(cargo).ok());
    let ct = cargo.map_or_else(unset, |cargo| {
        let name = names.cargo_name(cargo).unwrap_or_default();
        name.strip_prefix('*').map_or(name.clone(), str::to_owned)
    });
    let cq = cargo.map_or_else(unset, |_| values.cargo_tons.to_string());
    let dl = values
        .deadline
        .filter(|&deadline| deadline != mission.today)
        .map_or_else(unset, |deadline| {
            long_date_text(deadline, mission.date_affixes)
        });
    let pay = values
        .pay
        .and_then(|pay| pay_shown(pay, mission.cash))
        .map_or_else(unset, |amount| comma_number(amount, mission.pay_millions));
    [
        ("<DST>", or_else(dst, &rst)),
        ("<DSY>", or_else(dsy, &rsy)),
        ("<RST>", rst),
        ("<RSY>", rsy),
        ("<CT>", ct),
        ("<CQ>", cq),
        ("<SN>", values.ship_name.clone().unwrap_or_else(unset)),
        ("<DL>", dl),
        ("<PAY>", pay),
    ]
}

/// The lowest and highest government a `<PRKnnn>` or `<SRKnnn>` names:
/// nnn - 128 is 0-255 (`_ParseTextDescription` @0x14f4b-0x15039).
const RANKED_GOVTS: std::ops::RangeInclusive<u32> = 128..=383;

/// `text` with each `{prefix}nnn>` of a government in [`RANKED_GOVTS`]
/// read as `rank`; any other left as written.
fn fill_ranked(text: &str, prefix: &str, rank: &str) -> String {
    let mut filled = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        filled.push_str(&rest[..at]);
        let after = &rest[at + prefix.len()..];
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let govt = after[..digits].parse::<u32>().ok();
        match govt.filter(|govt| RANKED_GOVTS.contains(govt)) {
            Some(_) if after[digits..].starts_with('>') => {
                filled.push_str(rank);
                rest = &after[digits + 1..];
            }
            _ => {
                filled.push_str(prefix);
                rest = after;
            }
        }
    }
    filled.push_str(rest);
    filled
}

/// `text` with its tags filled as `context` and `names` say, as
/// `_MungeBriefing` (@0x9c11d) fills them: one tag at a time in the
/// table's order (0xdb198), every occurrence, case-sensitively, so a
/// value naming a later tag has it filled too; any other tag is left as
/// written. With no mission, the mission tags are left as written too.
#[must_use]
pub fn expand(text: &str, context: &TagContext, names: &(impl TagNames + ?Sized)) -> String {
    let mission = context
        .mission
        .as_ref()
        .map(|mission| mission_values(mission, names));
    let (early, pay) = match &mission {
        Some([early @ .., pay]) => (early.as_slice(), Some(pay)),
        None => (&[][..], None),
    };
    let player = [
        ("<PN>", context.pilot),
        ("<PNN>", context.pilot),
        ("<PSN>", context.ship_name),
        ("<PST>", context.ship_type),
        ("<OSN>", context.person.unwrap_or(UNSET)),
        ("<PRK>", NO_RANK),
        ("<SRK>", NO_RANK),
        ("<RRK>", NO_RANK),
    ];
    let tags = early
        .iter()
        .map(|(tag, value)| (*tag, value.as_str()))
        .chain(player)
        .chain(pay.map(|(tag, value)| (*tag, value.as_str())))
        .chain([("<REG>", "")]);
    let filled = tags.fold(text.to_owned(), |text, (tag, value)| {
        text.replace(tag, value)
    });
    let filled = fill_ranked(&filled, "<PRK", NO_RANK);
    fill_ranked(&filled, "<SRK", NO_RANK)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pay shown.

    #[test]
    fn a_pay_above_0_shows_itself() {
        assert_eq!(pay_shown(1, 500), Some(1));
        assert_eq!(pay_shown(25_000, 500), Some(25_000));
    }

    #[test]
    fn a_fining_pay_shows_its_percent_of_the_cash_truncated() {
        assert_eq!(pay_shown(-40_001, 12_345), Some(123));
        assert_eq!(pay_shown(-40_010, 12_345), Some(1234));
        assert_eq!(pay_shown(-40_099, 12_345), Some(12_221));
        assert_eq!(pay_shown(-40_050, 0), Some(0));
    }

    #[test]
    fn a_fine_of_a_negative_cash_shows_nothing() {
        assert_eq!(pay_shown(-40_010, -500), None);
    }

    #[test]
    fn a_charging_pay_shows_what_it_takes() {
        assert_eq!(pay_shown(-50_001, 500), Some(1));
        assert_eq!(pay_shown(-60_000, 500), Some(10_000));
    }

    #[test]
    fn any_other_pay_shows_0() {
        for pay in [0, -1, -10_128, -40_000, -40_100, -50_000] {
            assert_eq!(pay_shown(pay, 12_345), Some(0), "{pay}");
        }
    }

    // Numbers with commas.

    #[test]
    fn a_number_below_1000_is_plain() {
        assert_eq!(comma_number(0, RuleSource::Engine), "0");
        assert_eq!(comma_number(999, RuleSource::Engine), "999");
        assert_eq!(comma_number(-5, RuleSource::Engine), "-5");
        assert_eq!(comma_number(-1234, RuleSource::Engine), "-1234");
    }

    #[test]
    fn a_number_in_the_thousands_takes_a_comma() {
        assert_eq!(comma_number(1000, RuleSource::Engine), "1,000");
        assert_eq!(comma_number(1005, RuleSource::Engine), "1,005");
        assert_eq!(comma_number(999_999, RuleSource::Engine), "999,999");
    }

    #[test]
    fn by_the_engine_a_million_or_more_reads_in_millions_to_two_places_truncated() {
        assert_eq!(comma_number(1_000_000, RuleSource::Engine), "1.00M");
        assert_eq!(comma_number(1_050_000, RuleSource::Engine), "1.05M");
        assert_eq!(comma_number(1_234_567, RuleSource::Engine), "1.23M");
        assert_eq!(comma_number(1_239_999, RuleSource::Engine), "1.23M");
        assert_eq!(
            comma_number(12_345_678_901, RuleSource::Engine),
            "12345.67M"
        );
    }

    #[test]
    fn by_the_other_reading_a_million_or_more_takes_its_commas() {
        assert_eq!(comma_number(1_000_000, RuleSource::Bible), "1,000,000");
        assert_eq!(comma_number(1_234_567, RuleSource::Bible), "1,234,567");
        assert_eq!(
            comma_number(12_345_678_901, RuleSource::Bible),
            "12,345,678,901"
        );
        assert_eq!(comma_number(999_999, RuleSource::Bible), "999,999");
        assert_eq!(comma_number(999, RuleSource::Bible), "999");
    }

    // The tags.

    use std::collections::BTreeMap;

    use crate::catalog::MissionId;

    /// Names by ID, and the cargo names in a list.
    #[derive(Default)]
    struct Names {
        stellars: BTreeMap<i16, &'static str>,
        systems: BTreeMap<i16, &'static str>,
        cargo: Vec<&'static str>,
    }

    impl TagNames for Names {
        fn stellar_name(&self, stellar: StellarId) -> Option<String> {
            self.stellars.get(&stellar.0).map(|&name| name.to_owned())
        }

        fn system_name(&self, system: SystemId) -> Option<String> {
            self.systems.get(&system.0).map(|&name| name.to_owned())
        }

        fn cargo_name(&self, cargo: u8) -> Option<String> {
            self.cargo
                .get(usize::from(cargo))
                .map(|&name| name.to_owned())
        }
    }

    /// Stellars 128 Earth and 129 Luna and 140 Mars, systems 130 Sol and
    /// 131 Alpha; cargo food, "*passengers", industrial.
    fn names() -> Names {
        Names {
            stellars: BTreeMap::from([(128, "Earth"), (129, "Luna"), (140, "Mars")]),
            systems: BTreeMap::from([(130, "Sol"), (131, "Alpha")]),
            cargo: vec!["food", "*passengers", "industrial"],
        }
    }

    fn stop(stellar: i16, system: Option<i16>) -> Stop {
        Stop {
            stellar: StellarId(stellar),
            system: system.map(SystemId),
        }
    }

    fn today() -> GameDate {
        GameDate::new(1177, 6, 23).expect("exists")
    }

    /// A mission to Mars in Alpha and back to Earth in Sol with 12 tons
    /// of industrial for 25,000 credits by July 1st, aboard "Rover".
    fn values() -> MissionValues {
        MissionValues {
            destination: Some(stop(140, Some(131))),
            return_to: Some(stop(128, Some(130))),
            cargo_type: Some(2),
            cargo_tons: 12,
            pay: Some(25_000),
            deadline: GameDate::new(1177, 7, 1),
            ship_name: Some("Rover".to_owned()),
        }
    }

    static AFFIXES: DateAffixes = DateAffixes {
        prefix: String::new(),
        suffix: String::new(),
    };

    /// Stock the pilot, flying the Kestrel, a Shuttle, hailed by the
    /// Bounty Hunter, with `mission`, 12,345 credits and the engine's
    /// rules.
    fn context(mission: Option<MissionValues>) -> TagContext<'static> {
        TagContext {
            pilot: "Stock",
            ship_type: "Shuttle",
            ship_name: "Kestrel",
            person: Some("Bounty Hunter"),
            mission: mission.map(|values| MissionTags {
                values,
                cash: 12_345,
                today: today(),
                date_affixes: &AFFIXES,
                pay_millions: RuleSource::Engine,
            }),
        }
    }

    /// `text` with `values`' tags filled.
    fn with(values: MissionValues, text: &str) -> String {
        expand(text, &context(Some(values)), &names())
    }

    #[test]
    fn dst_and_dsy_name_the_destination_and_its_system() {
        assert_eq!(with(values(), "<DST> in <DSY>"), "Mars in Alpha");
    }

    #[test]
    fn rst_and_rsy_name_the_return_and_its_system() {
        assert_eq!(with(values(), "<RST> in <RSY>"), "Earth in Sol");
    }

    #[test]
    fn a_stop_with_no_system_or_no_name_reads_unset() {
        let values = MissionValues {
            destination: Some(stop(140, None)),
            return_to: Some(stop(999, Some(999))),
            ..values()
        };
        assert_eq!(
            with(values, "<DST> <DSY> <RST> <RSY>"),
            "Mars [Error] [Error] [Error]"
        );
    }

    #[test]
    fn with_no_destination_dst_and_dsy_read_the_return() {
        let values = MissionValues {
            destination: None,
            ..values()
        };
        assert_eq!(with(values, "<DST> in <DSY>"), "Earth in Sol");
    }

    #[test]
    fn dsy_reads_rsy_on_its_own_when_only_it_is_unset() {
        let values = MissionValues {
            destination: Some(stop(140, None)),
            ..values()
        };
        assert_eq!(with(values, "<DST> in <DSY>"), "Mars in Sol");
    }

    #[test]
    fn with_no_stop_at_all_all_four_read_unset() {
        let values = MissionValues {
            destination: None,
            return_to: None,
            ..values()
        };
        assert_eq!(
            with(values, "<DST> <DSY> <RST> <RSY>"),
            "[Error] [Error] [Error] [Error]"
        );
    }

    #[test]
    fn ct_names_the_cargo_and_cq_its_tons_in_plain_digits() {
        let values = MissionValues {
            cargo_tons: 1500,
            ..values()
        };
        assert_eq!(with(values, "<CQ> tons of <CT>"), "1500 tons of industrial");
    }

    #[test]
    fn ct_drops_a_leading_star() {
        let values = MissionValues {
            cargo_type: Some(1),
            ..values()
        };
        assert_eq!(with(values, "<CT>"), "passengers");
    }

    #[test]
    fn a_cargo_past_the_lists_end_reads_empty() {
        let values = MissionValues {
            cargo_type: Some(255),
            ..values()
        };
        assert_eq!(with(values, "[<CT>] <CQ>"), "[] 12");
    }

    #[test]
    fn a_cargo_outside_0_to_255_or_none_reads_unset_with_its_tons() {
        for cargo in [None, Some(-1), Some(256), Some(1000)] {
            let values = MissionValues {
                cargo_type: cargo,
                ..values()
            };
            assert_eq!(with(values, "<CQ> <CT>"), "[Error] [Error]", "{cargo:?}");
        }
        let first = MissionValues {
            cargo_type: Some(0),
            ..values()
        };
        assert_eq!(with(first, "<CQ> <CT>"), "12 food");
    }

    #[test]
    fn sn_is_the_missions_ship_name_and_unset_with_none() {
        assert_eq!(with(values(), "<SN>"), "Rover");
        let offered = MissionValues {
            ship_name: None,
            ..values()
        };
        assert_eq!(with(offered, "<SN>"), "[Error]");
    }

    #[test]
    fn dl_is_the_deadline_written_long() {
        assert_eq!(with(values(), "by <DL>"), "by July 1st, 1177");
        let tags = TagContext {
            mission: Some(MissionTags {
                date_affixes: &DateAffixes {
                    prefix: "Year ".to_owned(),
                    suffix: " NC".to_owned(),
                },
                ..context(Some(values())).mission.expect("a mission")
            }),
            ..context(None)
        };
        assert_eq!(expand("<DL>", &tags, &names()), "Year July 1st, 1177 NC");
    }

    #[test]
    fn dl_with_no_deadline_or_one_of_today_reads_unset() {
        for deadline in [None, Some(today())] {
            let values = MissionValues {
                deadline,
                ..values()
            };
            assert_eq!(with(values, "<DL>"), "[Error]", "{deadline:?}");
        }
        let tomorrow = MissionValues {
            deadline: Some(today().next_day()),
            ..values()
        };
        assert_eq!(with(tomorrow, "<DL>"), "June 24th, 1177");
    }

    #[test]
    fn the_player_tags_name_the_pilot_and_the_ship() {
        assert_eq!(
            with(values(), "<PN>, <PNN>, <PSN>, <PST>"),
            "Stock, Stock, Kestrel, Shuttle"
        );
    }

    #[test]
    fn osn_names_the_person_and_reads_unset_with_none() {
        assert_eq!(with(values(), "<OSN>"), "Bounty Hunter");
        let alone = TagContext {
            person: None,
            ..context(Some(values()))
        };
        assert_eq!(expand("<OSN>", &alone, &names()), "[Error]");
    }

    #[test]
    fn pay_shows_the_amount_with_its_commas() {
        assert_eq!(with(values(), "<PAY>"), "25,000");
        let millions = MissionValues {
            pay: Some(1_234_567),
            ..values()
        };
        assert_eq!(with(millions.clone(), "<PAY>"), "1.23M");
        let mut full = context(Some(millions));
        if let Some(mission) = &mut full.mission {
            mission.pay_millions = RuleSource::Bible;
        }
        assert_eq!(expand("<PAY>", &full, &names()), "1,234,567");
        let fine = MissionValues {
            pay: Some(-40_010),
            ..values()
        };
        assert_eq!(with(fine, "<PAY>"), "1,234", "10 % of 12,345");
    }

    #[test]
    fn pay_with_no_amount_reads_unset() {
        let none = MissionValues {
            pay: None,
            ..values()
        };
        assert_eq!(with(none, "<PAY>"), "[Error]");
        let mut broke = context(Some(MissionValues {
            pay: Some(-40_010),
            ..values()
        }));
        if let Some(mission) = &mut broke.mission {
            mission.cash = -500;
        }
        assert_eq!(expand("<PAY>", &broke, &names()), "[Error]");
    }

    #[test]
    fn reg_is_empty() {
        assert_eq!(with(values(), "[<REG>]"), "[]");
        assert_eq!(expand("[<REG>]", &context(None), &names()), "[]");
    }

    #[test]
    fn the_rank_tags_read_captain() {
        assert_eq!(
            with(values(), "<PRK> <SRK> <RRK> <PRK128> <SRK383> <PRK200>"),
            "captain captain captain captain captain captain"
        );
    }

    #[test]
    fn a_rank_tag_of_a_government_outside_128_to_383_is_left_as_written() {
        assert_eq!(
            with(
                values(),
                "<PRK999> <SRK127> <PRK384> <PRK> <PRKx> <PRK12a> <PRK130x> <SRK130"
            ),
            "<PRK999> <SRK127> <PRK384> captain <PRKx> <PRK12a> <PRK130x> <SRK130"
        );
    }

    #[test]
    fn every_occurrence_is_filled_case_sensitively() {
        assert_eq!(
            with(values(), "<DST>, <DST>; <dst> <Dst> <DST"),
            "Mars, Mars; <dst> <Dst> <DST"
        );
    }

    #[test]
    fn unknown_tags_are_left_as_written() {
        assert_eq!(
            with(values(), "<good> <DSZ> <XYZ> <> no tags"),
            "<good> <DSZ> <XYZ> <> no tags"
        );
    }

    #[test]
    fn a_value_naming_a_later_tag_has_it_filled_too() {
        let mut named = names();
        named.stellars.insert(140, "<PN>'s Rest");
        assert_eq!(
            expand("<DST>", &context(Some(values())), &named),
            "Stock's Rest"
        );
        let early = TagContext {
            pilot: "<DST>",
            ..context(Some(values()))
        };
        assert_eq!(
            expand("<PN>", &early, &names()),
            "<DST>",
            "not an earlier one"
        );
    }

    #[test]
    fn a_freed_slots_mission_tags_read_unset_and_its_player_tags_fill() {
        assert_eq!(
            with(
                MissionValues::default(),
                "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> <PN> <OSN>"
            ),
            "[Error] [Error] [Error] [Error] [Error] [Error] [Error] [Error] [Error] \
             Stock Bounty Hunter"
        );
    }

    #[test]
    fn with_no_mission_the_mission_tags_are_left_as_written() {
        assert_eq!(
            expand(
                "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> <PN> <PRK>",
                &context(None),
                &names()
            ),
            "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> Stock captain"
        );
    }

    #[test]
    fn no_names_names_nothing() {
        assert_eq!(NoNames.stellar_name(StellarId(128)), None);
        assert_eq!(NoNames.system_name(SystemId(130)), None);
        assert_eq!(NoNames.cargo_name(0), None);
    }

    // The values.

    fn offer() -> Offer {
        Offer {
            mission: MissionId(130),
            venue: crate::mission::Venue::Computer,
            destination: Some(stop(140, Some(131))),
            return_to: Some(stop(128, Some(130))),
            cargo_type: Some(2),
            cargo_tons: 12,
            pay: -40_010,
            deadline: GameDate::new(1177, 7, 1),
        }
    }

    #[test]
    fn an_offers_values_are_its_own_with_no_ship_name() {
        assert_eq!(
            MissionValues::offered(&offer()),
            MissionValues {
                pay: Some(-40_010),
                ship_name: None,
                ..values()
            }
        );
    }

    #[test]
    fn an_active_missions_values_are_its_own_with_an_empty_ship_name() {
        let record = crate::testkit::mission(130);
        let mut active = ActiveMission::accepted(&record, &offer(), today());
        active.deadline = GameDate::new(1177, 7, 2);
        assert_eq!(
            MissionValues::active(&active),
            MissionValues {
                pay: Some(-40_010),
                deadline: GameDate::new(1177, 7, 2),
                ship_name: Some(String::new()),
                ..values()
            }
        );
    }
}
