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
//! | `<CT>` | for a cargo type of 0-255, `STR#` 4001's string type + 1, as a plug-in's `'STR '` 9100 + type patches it ([`strings`](crate::strings)), a leading `*` dropped (@0x9c65f-0x9c6bf) |
//! | `<CQ>` | the tons in plain digits, with a cargo type of 0-255 (@0x9c6c1-0x9c710) |
//! | `<SN>` | a slot's ship name (+0x70, @0x9ccf1); an offer has none |
//! | `<DL>` | the deadline, written long ([`long_date_text`]); one of today is `"[Error]"` (@0x9c715-0x9c78e) |
//! | `<PN>`, `<PNN>` | the pilot's name; its nickname, or the name with none (@0x9cf2d-0x9cfb6) |
//! | `<PSN>`, `<PST>` | the player's ship's name and its class's (@0x9cfbb-0x9d034) |
//! | `<OSN>` | the name of the person of the ship last named offering (@0x9cebe-0x9cf28; see [`RuleKey::OfferingShipName`](crate::RuleKey::OfferingShipName)) |
//! | `<PRK>`, `<SRK>`, `<RRK>` | the ranks' names, `STR#` 2002 #341 ("captain") with none (@0x9d039-0x9d3b3; see [`rank`](crate::rank)) |
//! | `<PAY>` | the amount [`pay_shown`] gives, written by [`comma_number`]; below 0 `"[Error]"` (@0x9c7a0-0x9c835, @0x9cd2a-0x9cdb6) |
//! | `<REG>` | the registration's licence name (`_RT3_GetDisplayName` @0xd1517) |
//! | `<PRKnnn>`, `<SRKnnn>` | government nnn's highest active rank's `ShortName` (@0x9d1aa-0x9d35a) |
//!
//! **Here.** [`expand`] fills a text from a [`TagContext`], its names
//! read through the [`TagNames`] port. A context's mission is an offer's
//! ([`MissionValues::offered`]) or a slot's ([`MissionValues::active`]),
//! resolved, never the `mïsn`'s encodings; a free slot's is the default,
//! every mission tag [`UNSET`]. With no mission at all, as in a person's
//! hail quote ([`expand_tags`](crate::person::expand_tags)) or a
//! `RefuseText`, the mission tags
//! follow [`RuleKey::MissionlessTags`](crate::RuleKey::MissionlessTags):
//! by the engine [`UNSET`], as `_MungeBriefing` with index -1 leaves them
//! (@0x9c83f-0x9c843 → 0x9ce14); by the other reading left as written.
//! Where the engine reads otherwise:
//!
//! - `<DSY>` and `<RSY>` read the stop's stellar's system live, through
//!   the port ([`TagNames::stellar_system`]), not the system the offer
//!   stored ([`Stop::system`]): the stellar's holder's stand-in
//!   (`_FindActiveCoLocatedSystem`), else the holder's chain head
//!   (`_FindFirstCoLocatedSystem`), else, with no holder, the system
//!   `_FindSystemFromStellar` gives, unresolved by place (@0x9c8de-0x9c937,
//!   @0x9c9f9-0x9ca5c). So the name follows a galaxy change after the
//!   offer, as the original's does.
//! - There is no nickname, so `<PNN>` is the pilot's name; and no
//!   licence, so `<REG>` is empty.
//! - `<SN>` reads a slot's ship name, which stays empty until
//!   rdm:phase/mission-ships/phase-1-spawn-mission-ships draws one.
//! - The rank tags read the context's [`RankNames`]. `<PRKnnn>` and
//!   `<SRKnnn>` follow [`RuleKey::CrossedRankTags`](crate::RuleKey::CrossedRankTags):
//!   by the engine, as the original crosses them, only the nnn of each
//!   kind the context's [`RankTagGovts`] remember is filled, `<PRKnnn>`
//!   with the `<SRKnnn>` government's rank and `<SRKnnn>` with the
//!   `<PRKnnn>` one's; by the Bible, each nnn of
//!   128-383 with government nnn's own, any other nnn left as written.
//!   A rank is named by its `ShortName`; by the Bible's reading of
//!   [`RuleKey::PrkNnnName`](crate::RuleKey::PrkNnnName), `<PRKnnn>`
//!   reads its `ConvName`, "same as `<PRK>`". A context with no ranks
//!   wired ([`RankNames::none`]) reads "captain"
//!   ([`NO_RANK`](crate::person::NO_RANK)) for each.
//! - `<PAY>` of a million or more follows [`RuleKey::PayMillions`](crate::RuleKey::PayMillions),
//!   and a slot's `<PAY>` of a pay that is not cash, a fine or a charge
//!   follows [`RuleKey::PaySlotOffset`](crate::RuleKey::PaySlotOffset):
//!   by the engine the slot's byte offset, as the original's register
//!   leaves it ([`MissionTags::other_pay`]).
//!
//! - A text is filled after its conditional text is resolved
//!   ([`text_parse`](crate::text_parse), `_ParseTextDescription`), which
//!   the session does as it takes a text in: `expand` sees no `{G …}`.
//!
//! - With the data's `l33t` mode on ([`TagContext::leet`]), the filled
//!   text is [`munge_leet`]ed last, as `_MungeBriefing` does
//!   (@0x9d5aa-0x9d667); a `dësc` the session shows has its letters drawn
//!   first ([`load_leet`], `_LoadDescription` @0x7194f-0x71a5c), which
//!   can break a tag (`<PAY>` to `<P4Y>`) that is then left as written.
//!
//! - `<OSN>` reads the context's person, which the session gives as
//!   [`RuleKey::OfferingShipName`](crate::RuleKey::OfferingShipName)
//!   says: by the engine, the original's `_offeringShipIndex`, which a
//!   hail leaves set, so a later text names the person last hailed.

use crate::catalog::{DateAffixes, GovtId, PilotCatalog, ShipId, StellarId, SystemId};
use crate::chance::Chance;
use crate::date::{GameDate, long_date_text};
use crate::mission::{ActiveMission, Offer, Stop};
use crate::rank::RankNames;
use crate::rulebook::RuleSource;

/// What a tag reads that has no value: every value's start
/// (0xdb190, @0x9c1a7-0x9c37b).
pub const UNSET: &str = "[Error]";

/// `PayVal` from which a pay fines a percent of the cash: -40001 to
/// -40099, the percent `-pay - 40000`.
const FINES: std::ops::RangeInclusive<i64> = -40_099..=-40_001;
/// `PayVal` below which a pay is cash taken at accept: `-50000 - pay`.
const CHARGES_BELOW: i64 = -50_000;

/// The bytes of one active mission slot (`_currentMissions`, 0x8ec):
/// what a slot's `<PAY>` of any other pay reads by the engine, times the
/// slot (@0x9c874-0x9c87c).
pub const SLOT_BYTES: i64 = 0x8ec;

/// The amount `<PAY>` shows for a pay of `pay` with `cash` held
/// (@0x9c7a0-0x9c835, @0x9cd2a-0x9cdb6): above 0, the pay; -40099 to
/// -40001, its percent of the cash, worked in doubles and truncated as
/// the original does (trunc(cash × (percent × 0.01)), 0.01 @0xdd098);
/// below -50000, the cash taken at accept; any other pay, `other`
/// ([`MissionTags::other_pay`]). `None` for an amount below 0, which the
/// tag shows as [`UNSET`].
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub fn pay_shown(pay: i64, cash: i64, other: i64) -> Option<i64> {
    let amount = match pay {
        1.. => pay,
        pay if FINES.contains(&pay) => {
            let percent = (-pay - 40_000) as f64;
            (cash as f64 * (percent * 0.01)).trunc() as i64
        }
        pay if pay < CHARGES_BELOW => CHARGES_BELOW - pay,
        _ => other,
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
    /// The system a mission tag names stellar `stellar` in, as
    /// `_MungeBriefing` resolves it when the text is drawn
    /// ([`Whereabouts::named`](crate::story::Whereabouts::named)): `None`
    /// when no system lists it.
    fn stellar_system(&self, stellar: StellarId) -> Option<SystemId>;
    /// Cargo type `cargo`'s name, raw: `STR#` 4001's string `cargo` + 1
    /// as `_LoadStrings` patches it (`_lcCargoName`, 0x1b8de0; see
    /// [`strings`](crate::strings)), `None` past the list's end.
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

    fn stellar_system(&self, _stellar: StellarId) -> Option<SystemId> {
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

    /// `active`'s values (`_currentMissions`, 0x184688), its ship name
    /// the one drawn at accept (slot +0x70, `_LoadCurrentMissionData`
    /// @0xa1195-0xa1273): empty when it has none.
    #[must_use]
    pub fn active(active: &ActiveMission) -> Self {
        Self {
            destination: active.destination,
            return_to: active.return_to,
            cargo_type: active.cargo_type,
            cargo_tons: active.cargo_tons,
            pay: Some(active.pay),
            deadline: active.deadline,
            ship_name: Some(active.ships.name.clone()),
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
    /// The active slot the mission is in; `None` for an offer.
    pub slot: Option<u8>,
}

impl MissionTags<'_> {
    /// What `<PAY>` shows of a pay that is not cash, a fine or a charge,
    /// as `rule` ([`RuleKey::PaySlotOffset`](crate::RuleKey::PaySlotOffset))
    /// reads it: an offer's is 0 (@0x9c7e6-0x9c7f0); a slot's is, by the
    /// engine, the slot's byte offset, [`SLOT_BYTES`] × the slot, which
    /// the original's register still holds (@0x9cd2a-0x9cdb6), and by
    /// the other reading 0, as an offer's.
    #[must_use]
    pub fn other_pay(&self, rule: RuleSource) -> i64 {
        match (self.slot, rule) {
            (Some(slot), RuleSource::Engine) => SLOT_BYTES * i64::from(slot),
            _ => 0,
        }
    }
}

/// The disputed rules the tags follow, each a
/// [`RuleKey`](crate::RuleKey)'s reading: the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TagRules {
    /// What a slot's `<PAY>` of any other pay reads
    /// ([`RuleKey::PaySlotOffset`](crate::RuleKey::PaySlotOffset)).
    pub pay_slot_offset: RuleSource,
    /// What a text with no mission reads for the mission tags
    /// ([`RuleKey::MissionlessTags`](crate::RuleKey::MissionlessTags)).
    pub missionless: RuleSource,
    /// Which name of a rank `<PRKnnn>` reads
    /// ([`RuleKey::PrkNnnName`](crate::RuleKey::PrkNnnName)).
    pub prk_nnn_name: RuleSource,
    /// Whether `<PRKnnn>` and `<SRKnnn>` cross their remembered
    /// governments ([`RuleKey::CrossedRankTags`](crate::RuleKey::CrossedRankTags)).
    pub crossed_rank_tags: RuleSource,
}

impl TagRules {
    /// The readings `rules` gives.
    #[must_use]
    pub fn from_rulebook(rules: &crate::rulebook::Rulebook) -> Self {
        use crate::rulebook::RuleKey;
        Self {
            pay_slot_offset: rules.source_for(RuleKey::PaySlotOffset),
            missionless: rules.source_for(RuleKey::MissionlessTags),
            prk_nnn_name: rules.source_for(RuleKey::PrkNnnName),
            crossed_rank_tags: rules.source_for(RuleKey::CrossedRankTags),
        }
    }
}

/// The governments the last `<PRKnnn>` and `<SRKnnn>` a text named,
/// as the original remembers them between texts (`_descRankGovt`, +0
/// and +2): what the crossed rank tags read
/// ([`RuleKey::CrossedRankTags`](crate::RuleKey::CrossedRankTags)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RankTagGovts {
    /// The last `<PRKnnn>`'s government nnn.
    pub prk: Option<GovtId>,
    /// The last `<SRKnnn>`'s government nnn.
    pub srk: Option<GovtId>,
}

impl RankTagGovts {
    /// Neither: what `_LoadDescription` resets both to before each `dësc`
    /// (@0x71926-0x71930).
    pub const NONE: Self = Self {
        prk: None,
        srk: None,
    };

    /// These governments after `_ParseTextDescription`'s scan of `text`
    /// (@0x14f34-0x15055), counted in characters: a text of 5 or fewer
    /// is not scanned. At each place before the last 5 characters, a
    /// `<`, then `P` or `S`, then `RK` and anything but `>` is a tag:
    /// its digits (none reads 0) name government nnn when it is 128-383,
    /// remembered for its kind, and else that kind is forgotten; the scan
    /// goes on after the `<PRK`. So the last tag of each kind wins, and a
    /// kind with no tag in `text` is kept.
    #[must_use]
    pub fn parsed(self, text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut govts = self;
        let mut at = 0;
        while at + 5 < chars.len() {
            let kind = chars[at + 1];
            if chars[at] == '<'
                && matches!(kind, 'P' | 'S')
                && chars[at + 2..at + 4] == ['R', 'K']
                && chars[at + 4] != '>'
            {
                let nnn = chars[at + 4..]
                    .iter()
                    .map_while(|c| c.to_digit(10))
                    .fold(0_u64, |nnn, digit| {
                        nnn.saturating_mul(10).saturating_add(u64::from(digit))
                    });
                let govt = u32::try_from(nnn)
                    .ok()
                    .filter(|nnn| RANKED_GOVTS.contains(nnn))
                    .and_then(|nnn| i16::try_from(nnn).ok())
                    .map(GovtId);
                if kind == 'P' {
                    govts.prk = govt;
                } else {
                    govts.srk = govt;
                }
                at += 4;
            }
            at += 1;
        }
        govts
    }
}

/// `text` with `_MungeBriefing`'s `l33t` replacements made, after every
/// tag (@0x9d5aa-0x9d667, strings 0xdb20f-0xdb232): `you` and `You` to
/// `j00`, `ck` to `x0r`, then `i` and `I` to `1`, `e` and `E` to `3`, `l`
/// and `L` to `7`, each in that order, every occurrence.
#[must_use]
pub fn munge_leet(text: &str) -> String {
    const LEET: [(&str, &str); 9] = [
        ("you", "j00"),
        ("You", "j00"),
        ("ck", "x0r"),
        ("i", "1"),
        ("I", "1"),
        ("e", "3"),
        ("E", "3"),
        ("l", "7"),
        ("L", "7"),
    ];
    LEET.iter()
        .fold(text.to_owned(), |text, (from, to)| text.replace(from, to))
}

/// `text` with `_LoadDescription`'s `l33t` letters drawn on `chance`, as
/// a `dësc` is loaded before its tags are filled (@0x7194f-0x71a5c): each
/// character in turn, an `a` or `A` becomes `4`, an `e` or `E` `3`, an
/// `i` or `I` `1` and an `o` or `O` `0`, each on a draw of
/// `_Rand(3)` landing on 0 or 1, 2 in 3; then a `y` or `Y` followed by a
/// lower-case `ou`, more than 4 characters from the end, becomes `j00`.
/// Only what each draw decides, and its odds, are the original's, not
/// the order of the draws (`chance` is not the original's generator).
pub fn load_leet(text: &str, chance: &mut dyn Chance) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    for at in 0..len {
        for (letter, digit) in [('a', '4'), ('e', '3'), ('i', '1'), ('o', '0')] {
            if chars[at].eq_ignore_ascii_case(&letter) && chance.below(3) < 2 {
                chars[at] = digit;
            }
        }
        if at + 4 < len && matches!(chars[at], 'y' | 'Y') && chars[at + 1..at + 3] == ['o', 'u'] {
            chars[at..at + 3].copy_from_slice(&['j', '0', '0']);
        }
    }
    chars.into_iter().collect()
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
    /// The ranks: `<PRK>`, `<SRK>`, `<RRK>`, `<PRKnnn>` and `<SRKnnn>`.
    pub ranks: RankNames<'a>,
    /// The mission, if the text has one; with none its tags are left as
    /// written.
    pub mission: Option<MissionTags<'a>>,
    /// The governments the crossed rank tags read
    /// ([`RuleKey::CrossedRankTags`](crate::RuleKey::CrossedRankTags)'s
    /// engine reading): those the text's source leaves (see the
    /// [`story`](crate::story) module's "Text substitution").
    pub rank_govts: RankTagGovts,
    /// Whether the data holds a `l33t` resource (`_leetMode`,
    /// `_LoadObjectData` @0x7e28c-0x7e2a2): the filled text is then
    /// [`munge_leet`]ed.
    pub leet: bool,
    /// The disputed rules the tags follow.
    pub rules: TagRules,
}

/// `stop`'s stellar's and system's names, [`UNSET`] for each missing.
fn stop_names(stop: Option<Stop>, names: &(impl TagNames + ?Sized)) -> (String, String) {
    let stellar = stop.and_then(|stop| names.stellar_name(stop.stellar));
    let system = stop
        .and_then(|stop| names.stellar_system(stop.stellar))
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

/// The mission tags, in the table's order (0xdb198), as
/// [`mission_values`] gives them.
const MISSION_TAGS: [&str; 9] = [
    "<DST>", "<DSY>", "<RST>", "<RSY>", "<CT>", "<CQ>", "<SN>", "<DL>", "<PAY>",
];

/// The mission tags' values, in the table's order (@0x9c38a-0x9cdb6).
fn mission_values(
    mission: &MissionTags,
    rules: TagRules,
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
        .and_then(|pay| pay_shown(pay, mission.cash, mission.other_pay(rules.pay_slot_offset)))
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
/// read as `rank` gives for government nnn; any other left as written.
fn fill_ranked<'a>(text: &str, prefix: &str, rank: impl Fn(GovtId) -> &'a str) -> String {
    let mut filled = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(prefix) {
        filled.push_str(&rest[..at]);
        let after = &rest[at + prefix.len()..];
        let digits = after.len() - after.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let govt = after[..digits].parse::<u32>().ok();
        let govt = govt
            .filter(|govt| RANKED_GOVTS.contains(govt))
            .and_then(|govt| i16::try_from(govt).ok());
        match govt {
            Some(govt) if after[digits..].starts_with('>') => {
                filled.push_str(rank(GovtId(govt)));
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

/// `text` with its rank-nnn tags crossed as `_MungeBriefing` fills them
/// after its table (@0x9d53d-0x9d5a5): with a `<PRKnnn>` government
/// remembered, every `<PRKnnn>` of it reads the remembered `<SRKnnn>`
/// government's rank as `prk_name` names it; with an `<SRKnnn>`
/// government remembered, every `<SRKnnn>` of it reads the `<PRKnnn>`
/// government's `ShortName`. A government not remembered, or one with no
/// rank, reads the fallback (@0x9d25f, @0x9d33a). Any other rank-nnn tag
/// is left as written.
fn fill_crossed<'a>(
    text: &str,
    govts: RankTagGovts,
    ranks: &'a RankNames,
    prk_name: impl Fn(GovtId) -> &'a str,
) -> String {
    let fallback: &str = &ranks.fallback;
    let mut filled = text.to_owned();
    if let Some(prk) = govts.prk {
        let name = govts.srk.map_or(fallback, &prk_name);
        filled = filled.replace(&format!("<PRK{}>", prk.0), name);
    }
    if let Some(srk) = govts.srk {
        let name = govts.prk.map_or(fallback, |govt| ranks.short_of(govt));
        filled = filled.replace(&format!("<SRK{}>", srk.0), name);
    }
    filled
}

/// `text` with its tags filled as `context` and `names` say, as
/// `_MungeBriefing` (@0x9c11d) fills them: one tag at a time in the
/// table's order (0xdb198), every occurrence, case-sensitively, so a
/// value naming a later tag has it filled too; any other tag is left as
/// written. With no mission, the mission tags read as
/// [`TagRules::missionless`] says.
#[must_use]
pub fn expand(text: &str, context: &TagContext, names: &(impl TagNames + ?Sized)) -> String {
    let mission = match (&context.mission, context.rules.missionless) {
        (Some(mission), _) => Some(mission_values(mission, context.rules, names)),
        (None, RuleSource::Engine) => Some(MISSION_TAGS.map(|tag| (tag, UNSET.to_owned()))),
        (None, RuleSource::Bible) => None,
    };
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
        ("<PRK>", context.ranks.conversation()),
        ("<SRK>", context.ranks.short()),
        ("<RRK>", context.ranks.recent()),
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
    let ranks = &context.ranks;
    let prk_name = |govt| match context.rules.prk_nnn_name {
        RuleSource::Engine => ranks.short_of(govt),
        RuleSource::Bible => ranks.conversation_of(govt),
    };
    let filled = match context.rules.crossed_rank_tags {
        RuleSource::Engine => fill_crossed(&filled, context.rank_govts, ranks, prk_name),
        RuleSource::Bible => {
            let filled = fill_ranked(&filled, "<PRK", prk_name);
            fill_ranked(&filled, "<SRK", |govt| ranks.short_of(govt))
        }
    };
    if context.leet {
        munge_leet(&filled)
    } else {
        filled
    }
}

/// `text` with its tags filled as `_MungeBriefing(0, -1)` fills them
/// before any pilot is loaded, as the main screen's X does for `dësc`
/// 32766 (@0x1aca2-0x1acb1): no offer and slot -1, so every mission tag
/// is [`UNSET`].
///
/// The player tags read the original's globals as launch leaves them:
/// `<PN>`, `<PNN>` and `<PSN>` empty (`_playerName`, `_playerNickname`,
/// `_shipName`, zero-fill statics), the ranks the fallback ("captain",
/// `_rankData` zero-filled), and `<REG>` empty. `<PST>` reads ship class 0
/// (`_shipData` slot 0's +0x76, which `_InitObjects` @0x1c327 and
/// `_ResetPlayer(1)` @0x1d446 both set to 0, `_InitEVNova` calling them
/// @0x47252 and @0x4739a before `_DoMainScreen`): shïp 128's name in
/// `ships`, or empty when `ships` has no shïp 128 that can be read (the
/// engine's choice: the original is not read with it missing). `<OSN>` is
/// [`UNSET`]: `_ResetPlayer` sets slot 0's +0xc8d0 to -1 (@0x1d4fb),
/// which fails its `<= 0x3ff` check (@0x9cef5), so the tag keeps its
/// start, `"[Error]"` (@0x9c295).
#[must_use]
pub fn expand_before_any_pilot(text: &str, ships: &(impl PilotCatalog + ?Sized)) -> String {
    static NO_AFFIXES: DateAffixes = DateAffixes {
        prefix: String::new(),
        suffix: String::new(),
    };
    // With no deadline, `today` decides nothing.
    let mission = GameDate::new(1177, 1, 1).map(|today| MissionTags {
        values: MissionValues::default(),
        cash: 0,
        today,
        date_affixes: &NO_AFFIXES,
        pay_millions: RuleSource::Engine,
        slot: None,
    });
    // Ship class 0 is shïp 128.
    let ship_type = ships
        .ships()
        .into_iter()
        .find(|ship| ship.id == ShipId(128))
        .map(|ship| ship.name)
        .unwrap_or_default();
    let context = TagContext {
        pilot: "",
        ship_type: &ship_type,
        ship_name: "",
        person: None,
        ranks: RankNames::none(),
        mission,
        // The `dësc` loads as any does, its rank-nnn governments reset and
        // parsed from it; the tags read the engine's rules, and the
        // `l33t` mode is not read here.
        rank_govts: RankTagGovts::NONE.parsed(text),
        leet: false,
        rules: TagRules::default(),
    };
    expand(text, &context, &NoNames)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pay shown.

    #[test]
    fn a_pay_above_0_shows_itself() {
        assert_eq!(pay_shown(1, 500, 0), Some(1));
        assert_eq!(pay_shown(25_000, 500, 0), Some(25_000));
    }

    #[test]
    fn a_fining_pay_shows_its_percent_of_the_cash_truncated() {
        assert_eq!(pay_shown(-40_001, 12_345, 0), Some(123));
        assert_eq!(pay_shown(-40_010, 12_345, 0), Some(1234));
        assert_eq!(pay_shown(-40_099, 12_345, 0), Some(12_221));
        assert_eq!(pay_shown(-40_050, 0, 0), Some(0));
    }

    #[test]
    fn a_fine_of_a_negative_cash_shows_nothing() {
        assert_eq!(pay_shown(-40_010, -500, 0), None);
    }

    #[test]
    fn a_charging_pay_shows_what_it_takes() {
        assert_eq!(pay_shown(-50_001, 500, 0), Some(1));
        assert_eq!(pay_shown(-60_000, 500, 0), Some(10_000));
    }

    #[test]
    fn any_other_pay_shows_the_other_amount() {
        for pay in [0, -1, -10_128, -40_000, -40_100, -50_000] {
            assert_eq!(pay_shown(pay, 12_345, 0), Some(0), "{pay}");
            assert_eq!(pay_shown(pay, 12_345, 4568), Some(4568), "{pay}");
        }
    }

    #[test]
    fn the_other_amount_changes_no_cash_fine_or_charge() {
        assert_eq!(pay_shown(25_000, 12_345, 4568), Some(25_000));
        assert_eq!(pay_shown(-40_010, 12_345, 4568), Some(1234));
        assert_eq!(pay_shown(-60_000, 12_345, 4568), Some(10_000));
        assert_eq!(pay_shown(-40_010, -500, 4568), None);
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

    use std::collections::{BTreeMap, BTreeSet};

    use crate::catalog::{GovtId, MissionId, RankId, RankRecord};

    /// Names by ID, and the cargo names in a list.
    #[derive(Default)]
    struct Names {
        stellars: BTreeMap<i16, &'static str>,
        systems: BTreeMap<i16, &'static str>,
        /// The system each stellar is named in.
        placed: BTreeMap<i16, i16>,
        cargo: Vec<&'static str>,
    }

    impl TagNames for Names {
        fn stellar_name(&self, stellar: StellarId) -> Option<String> {
            self.stellars.get(&stellar.0).map(|&name| name.to_owned())
        }

        fn system_name(&self, system: SystemId) -> Option<String> {
            self.systems.get(&system.0).map(|&name| name.to_owned())
        }

        fn stellar_system(&self, stellar: StellarId) -> Option<SystemId> {
            self.placed.get(&stellar.0).copied().map(SystemId)
        }

        fn cargo_name(&self, cargo: u8) -> Option<String> {
            self.cargo
                .get(usize::from(cargo))
                .map(|&name| name.to_owned())
        }
    }

    /// Stellars 128 Earth in 130 Sol, 129 Luna placed nowhere and 140
    /// Mars in 131 Alpha; cargo food, "*passengers", industrial.
    fn names() -> Names {
        Names {
            stellars: BTreeMap::from([(128, "Earth"), (129, "Luna"), (140, "Mars")]),
            systems: BTreeMap::from([(130, "Sol"), (131, "Alpha")]),
            placed: BTreeMap::from([(128, 130), (140, 131)]),
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
            ranks: RankNames::none(),
            mission: mission.map(|values| MissionTags {
                values,
                cash: 12_345,
                today: today(),
                date_affixes: &AFFIXES,
                pay_millions: RuleSource::Engine,
                slot: None,
            }),
            rank_govts: RankTagGovts::NONE,
            leet: false,
            rules: TagRules::default(),
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
            destination: Some(stop(129, Some(130))),
            return_to: Some(stop(999, Some(999))),
            ..values()
        };
        assert_eq!(
            with(values, "<DST> <DSY> <RST> <RSY>"),
            "Luna [Error] [Error] [Error]"
        );
    }

    #[test]
    fn dsy_and_rsy_name_the_system_the_port_gives_the_stellar_not_the_stops() {
        let values = MissionValues {
            destination: Some(stop(140, Some(130))),
            return_to: Some(stop(128, None)),
            ..values()
        };
        assert_eq!(
            with(values, "<DST> in <DSY>, <RST> in <RSY>"),
            "Mars in Alpha, Earth in Sol",
            "_MungeBriefing reads the stellar's owner when the text is drawn (@0x9c8de, @0x9c9f9)"
        );
    }

    #[test]
    fn a_stellar_the_port_places_nowhere_reads_unset() {
        let values = MissionValues {
            return_to: Some(stop(129, Some(130))),
            ..values()
        };
        assert_eq!(
            with(values, "<RST> in <RSY>"),
            "Luna in [Error]",
            "@0x9ca4b: _FindSystemFromStellar gives -1"
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
            destination: Some(stop(129, None)),
            ..values()
        };
        assert_eq!(with(values, "<DST> in <DSY>"), "Luna in Sol");
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

    /// `values` filled in slot `slot` (none for an offer) under the
    /// `pay_slot_offset` reading `rule`.
    fn in_slot(values: MissionValues, slot: Option<u8>, rule: RuleSource, text: &str) -> String {
        let mut tags = context(Some(values));
        if let Some(mission) = &mut tags.mission {
            mission.slot = slot;
        }
        tags.rules.pay_slot_offset = rule;
        expand(text, &tags, &names())
    }

    #[test]
    fn by_the_engine_a_slots_other_pay_reads_its_byte_offset() {
        for (slot, shown) in [(0, "0"), (1, "2,284"), (3, "6,852"), (15, "34,260")] {
            for pay in [0, -45_000] {
                let values = MissionValues {
                    pay: Some(pay),
                    ..values()
                };
                assert_eq!(
                    in_slot(values, Some(slot), RuleSource::Engine, "<PAY>"),
                    shown,
                    "slot {slot}, pay {pay}: slot x 0x8ec (@0x9cd2a-0x9cdb6)"
                );
            }
        }
    }

    #[test]
    fn by_the_other_reading_a_slots_other_pay_reads_0() {
        let values = MissionValues {
            pay: Some(0),
            ..values()
        };
        assert_eq!(in_slot(values, Some(3), RuleSource::Bible, "<PAY>"), "0");
    }

    #[test]
    fn an_offers_other_pay_reads_0_by_either_reading() {
        for rule in RuleSource::ALL {
            let values = MissionValues {
                pay: Some(-10),
                ..values()
            };
            assert_eq!(in_slot(values, None, rule, "<PAY>"), "0", "{rule:?}");
        }
    }

    #[test]
    fn a_slots_fine_and_charge_are_unchanged_by_the_offset() {
        for (pay, shown) in [(-40_010, "1,234"), (-60_000, "10,000"), (25_000, "25,000")] {
            let values = MissionValues {
                pay: Some(pay),
                ..values()
            };
            assert_eq!(
                in_slot(values, Some(3), RuleSource::Engine, "<PAY>"),
                shown,
                "{pay}"
            );
        }
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
        let text = "<PRK> <SRK> <RRK> <PRK128> <SRK383> <PRK200>";
        let mut loaded = context(Some(values()));
        loaded.rank_govts = RankTagGovts::NONE.parsed(text);
        assert_eq!(
            expand(text, &loaded, &names()),
            "captain captain captain <PRK128> captain captain",
            "by the engine only the last <PRKnnn> parsed is filled"
        );
        loaded.rules.crossed_rank_tags = RuleSource::Bible;
        assert_eq!(
            expand(text, &loaded, &names()),
            "captain captain captain captain captain captain"
        );
    }

    /// Rank 200 of government 128, "Commander" ("Cmdr", "Cdr"), and
    /// rank 201 of government 129, "Admiral" ("Adm", "Ad"), of greater
    /// `Weight`.
    fn ranks() -> Vec<RankRecord> {
        let rank = |id, govt, weight, [name, conv, short]: [&str; 3]| RankRecord {
            name: name.to_owned(),
            conv_name: conv.to_owned(),
            short_name: short.to_owned(),
            affil_govt: Some(GovtId(govt)),
            weight,
            ..crate::testkit::rank(id)
        };
        vec![
            rank(200, 128, 1, ["Commander", "Cmdr", "Cdr"]),
            rank(201, 129, 5, ["Admiral", "Adm", "Ad"]),
        ]
    }

    #[test]
    fn the_rank_tags_read_the_contexts_ranks() {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        let tags = TagContext {
            ranks: RankNames {
                records: &records,
                active: &active,
                recent: Some(RankId(200)),
                fallback: "sir".into(),
            },
            ..context(Some(values()))
        };
        assert_eq!(
            expand("<PRK> <SRK> <RRK>", &tags, &names()),
            "Adm Ad Commander"
        );
        let held = BTreeSet::new();
        let none = TagContext {
            ranks: RankNames {
                active: &held,
                recent: None,
                ..tags.ranks.clone()
            },
            ..tags.clone()
        };
        assert_eq!(expand("<PRK> <SRK> <RRK>", &none, &names()), "sir sir sir");
    }

    /// [`context`] with [`ranks`] all held, `<PRKnnn>` named as
    /// `prk_nnn_name` reads it and the tags crossed as `crossed` reads
    /// them, the governments remembered `govts`.
    fn ranked<'a>(
        records: &'a [RankRecord],
        active: &'a BTreeSet<RankId>,
        prk_nnn_name: RuleSource,
        crossed: RuleSource,
        govts: RankTagGovts,
    ) -> TagContext<'a> {
        let mut tags = TagContext {
            ranks: RankNames {
                records,
                active,
                recent: None,
                fallback: "sir".into(),
            },
            rank_govts: govts,
            ..context(Some(values()))
        };
        tags.rules.prk_nnn_name = prk_nnn_name;
        tags.rules.crossed_rank_tags = crossed;
        tags
    }

    /// The governments `prk` and `srk` remembered.
    fn govts(prk: Option<i16>, srk: Option<i16>) -> RankTagGovts {
        RankTagGovts {
            prk: prk.map(GovtId),
            srk: srk.map(GovtId),
        }
    }

    /// `text` filled with [`ranks`] all held, by the engine's names, the
    /// tags crossed as `crossed` reads them, the governments remembered
    /// `remembered`.
    fn crossing(crossed: RuleSource, remembered: RankTagGovts, text: &str) -> String {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        let tags = ranked(&records, &active, RuleSource::Engine, crossed, remembered);
        expand(text, &tags, &names())
    }

    #[test]
    fn by_the_bible_each_rank_nnn_tag_reads_its_own_government() {
        assert_eq!(
            crossing(
                RuleSource::Bible,
                RankTagGovts::NONE,
                "<PRK128> <SRK128> <PRK129> <SRK129> <PRK130> <SRK383>"
            ),
            "Cdr Cdr Ad Ad sir sir",
            "uncrossed, each its own government's, nothing remembered needed"
        );
    }

    #[test]
    fn by_the_engine_two_governments_swap() {
        assert_eq!(
            crossing(
                RuleSource::Engine,
                govts(Some(128), Some(129)),
                "<PRK128> <SRK129>"
            ),
            "Ad Cdr",
            "<PRKnnn> takes the SRK government's, <SRKnnn> the PRK's (@0x9d53d-0x9d5a5)"
        );
    }

    #[test]
    fn by_the_engine_a_lone_tag_reads_the_fallback() {
        assert_eq!(
            crossing(RuleSource::Engine, govts(Some(128), None), "<PRK128>"),
            "sir"
        );
        assert_eq!(
            crossing(RuleSource::Engine, govts(None, Some(129)), "<SRK129>"),
            "sir"
        );
    }

    #[test]
    fn by_the_engine_only_the_remembered_nnn_is_filled() {
        assert_eq!(
            crossing(
                RuleSource::Engine,
                govts(Some(129), Some(129)),
                "<PRK128> <PRK129> <SRK128> <SRK129>"
            ),
            "<PRK128> Ad <SRK128> Ad"
        );
    }

    #[test]
    fn by_the_engine_a_tag_of_an_unset_government_is_left_as_written() {
        assert_eq!(
            crossing(RuleSource::Engine, RankTagGovts::NONE, "<PRK128> <SRK129>"),
            "<PRK128> <SRK129>"
        );
        assert_eq!(
            crossing(
                RuleSource::Engine,
                govts(None, Some(128)),
                "<PRK128> <SRK128>"
            ),
            "<PRK128> sir"
        );
    }

    #[test]
    fn by_the_engine_prk_nnn_reads_short_name() {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        for crossed in RuleSource::ALL {
            let tags = ranked(
                &records,
                &active,
                RuleSource::Engine,
                crossed,
                govts(Some(128), Some(128)),
            );
            assert_eq!(
                expand("<PRK128>", &tags, &names()),
                "Cdr",
                "+0xde @0x9d1ef, @0x9d2d0; {crossed:?}"
            );
        }
    }

    #[test]
    fn by_the_bible_prk_nnn_reads_conv_name() {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        let tags = ranked(
            &records,
            &active,
            RuleSource::Bible,
            RuleSource::Bible,
            RankTagGovts::NONE,
        );
        assert_eq!(
            expand("<PRK128> <PRK129> <PRK130>", &tags, &names()),
            "Cmdr Adm sir",
            "same as <PRK>"
        );
    }

    #[test]
    fn srk_nnn_reads_short_name_by_either_reading() {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        for rule in RuleSource::ALL {
            let tags = ranked(
                &records,
                &active,
                rule,
                RuleSource::Bible,
                RankTagGovts::NONE,
            );
            assert_eq!(
                expand("<SRK128> <SRK129>", &tags, &names()),
                "Cdr Ad",
                "{rule:?}"
            );
        }
    }

    #[test]
    fn prk_nnn_name_by_each_reading_with_the_crossing() {
        let records = ranks();
        let active = BTreeSet::from([RankId(200), RankId(201)]);
        let remembered = govts(Some(128), Some(129));
        let cases = [
            (RuleSource::Engine, RuleSource::Engine, "Ad Cdr"),
            (RuleSource::Bible, RuleSource::Engine, "Adm Cdr"),
            (RuleSource::Engine, RuleSource::Bible, "Cdr Ad"),
            (RuleSource::Bible, RuleSource::Bible, "Cmdr Ad"),
        ];
        for (name, crossed, filled) in cases {
            let tags = ranked(&records, &active, name, crossed, remembered);
            assert_eq!(
                expand("<PRK128> <SRK129>", &tags, &names()),
                filled,
                "prk_nnn_name {name:?}, crossed_rank_tags {crossed:?}"
            );
        }
    }

    #[test]
    fn a_rank_tag_of_a_government_outside_128_to_383_is_left_as_written() {
        let text = "<PRK999> <SRK127> <PRK384> <PRK> <PRKx> <PRK12a> <PRK130x> <SRK130";
        let filled = "<PRK999> <SRK127> <PRK384> captain <PRKx> <PRK12a> <PRK130x> <SRK130";
        let mut tags = context(Some(values()));
        tags.rules.crossed_rank_tags = RuleSource::Bible;
        assert_eq!(expand(text, &tags, &names()), filled);
        tags.rules.crossed_rank_tags = RuleSource::Engine;
        tags.rank_govts = RankTagGovts::NONE.parsed(text);
        assert_eq!(expand(text, &tags, &names()), filled);
    }

    // `_ParseTextDescription`'s scan (@0x14f34-0x15055).

    #[test]
    fn the_scan_remembers_the_last_prk_and_srk_government() {
        assert_eq!(
            RankTagGovts::NONE.parsed("<PRK128> and <SRK130> then <PRK129>."),
            govts(Some(129), Some(130))
        );
    }

    #[test]
    fn the_scan_keeps_a_kind_it_finds_no_tag_of() {
        assert_eq!(
            govts(Some(128), Some(129)).parsed("Only <SRK140> here."),
            govts(Some(128), Some(140))
        );
        assert_eq!(
            govts(Some(128), Some(129)).parsed("<PRK> <SRK> <prk130> <PRk130>"),
            govts(Some(128), Some(129)),
            "<PRK> with no number is not read, nor another case"
        );
    }

    #[test]
    fn a_tag_with_no_number_or_one_outside_128_to_383_forgets_its_kind() {
        for text in ["<PRKx> padding", "<PRK127>!!", "<PRK384>!!", "<PRK99999>!"] {
            assert_eq!(
                govts(Some(128), Some(129)).parsed(text),
                govts(None, Some(129)),
                "{text}"
            );
        }
        assert_eq!(
            govts(Some(128), Some(129)).parsed("<PRK383x> <SRK128a"),
            govts(Some(383), Some(128)),
            "the digits end at the first other character"
        );
    }

    #[test]
    fn the_scan_skips_a_text_of_5_characters_or_fewer_and_the_last_5() {
        assert_eq!(
            RankTagGovts::NONE.parsed("<PRK1"),
            RankTagGovts::NONE,
            "5 characters"
        );
        assert_eq!(
            govts(Some(130), None).parsed("ab<PRK1"),
            govts(Some(130), None),
            "a '<' among the last 5 is not looked at"
        );
        assert_eq!(
            govts(Some(130), None).parsed("<PRK1x"),
            RankTagGovts::NONE,
            "6 characters: read, and 1 is out of range"
        );
        assert_eq!(
            RankTagGovts::NONE.parsed("é<PRK128>"),
            govts(Some(128), None),
            "counted in characters"
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
    fn by_the_engine_a_missionless_texts_mission_tags_read_unset() {
        assert_eq!(
            expand(
                "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> <PN> <PRK>",
                &context(None),
                &names()
            ),
            "[Error] [Error] [Error] [Error] [Error] [Error] [Error] [Error] [Error] Stock \
             captain",
            "index -1 (@0x9c83f-0x9c843 -> 0x9ce14)"
        );
    }

    #[test]
    fn by_the_other_reading_a_missionless_texts_mission_tags_are_left_as_written() {
        let mut tags = context(None);
        tags.rules.missionless = RuleSource::Bible;
        assert_eq!(
            expand(
                "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> <PN> <PRK>",
                &tags,
                &names()
            ),
            "<DST> <DSY> <RST> <RSY> <CT> <CQ> <SN> <DL> <PAY> Stock captain"
        );
    }

    // `l33t`.

    #[test]
    fn munge_leet_replaces_in_the_originals_order() {
        assert_eq!(munge_leet("You lick"), "j00 71x0r");
        assert_eq!(munge_leet("yours"), "j00rs", "you before i");
        assert_eq!(munge_leet("Elite"), "371t3");
        assert_eq!(munge_leet("LAST pick"), "7AST p1x0r");
        assert_eq!(munge_leet("1337"), "1337");
    }

    #[test]
    fn with_leet_mode_the_filled_text_is_munged_last() {
        let mut tags = TagContext {
            pilot: "Ellie",
            ..context(Some(values()))
        };
        assert_eq!(expand("Hi <PN>, <DST>", &tags, &names()), "Hi Ellie, Mars");
        tags.leet = true;
        assert_eq!(
            expand("Hi <PN>, <DST>", &tags, &names()),
            "H1 37713, Mars",
            "the pilot's name filled in is munged too (@0x9d5aa-0x9d667)"
        );
    }

    #[test]
    fn load_leet_replaces_a_letter_on_2_draws_in_3() {
        let mut chance = crate::testkit::Draws::of(&[0, 2, 1, 2, 1, 0, 2, 2]);
        assert_eq!(
            load_leet("aeioAEIO", &mut chance),
            "4e1o43IO",
            "_Rand(3) of 0 or 1 replaces (@0x7198a-0x71993)"
        );
        assert_eq!(chance.asked, [3; 8], "one draw a vowel");
    }

    #[test]
    fn load_leet_draws_nothing_for_other_letters() {
        let mut chance = crate::testkit::Draws::of(&[]);
        assert_eq!(load_leet("thrysm ", &mut chance), "thrysm ");
        assert_eq!(chance.asked, Vec::<u32>::new());
    }

    #[test]
    fn load_leet_turns_you_into_j00() {
        let mut chance = crate::testkit::Draws::of(&[]);
        assert_eq!(load_leet("Your hymn", &mut chance), "j00r hymn");
        assert_eq!(load_leet("you!!", &mut chance), "j00!!");
        assert_eq!(
            chance.asked,
            Vec::<u32>::new(),
            "the o of you is not drawn for"
        );
        assert_eq!(
            load_leet("YOU!!", &mut chance),
            "YOU!!",
            "only a lower-case ou (@0x71a35-0x71a3e)"
        );
    }

    #[test]
    fn load_leet_leaves_a_you_too_near_the_end() {
        let mut chance = crate::testkit::Draws::of(&[]);
        assert_eq!(load_leet("you!", &mut chance), "you!", "i < strlen - 4");
        assert_eq!(load_leet("Thank you", &mut chance), "Thank you");
        assert_eq!(chance.asked, [3, 3, 3], "its o drawn for, kept");
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
    fn an_active_missions_ship_name_is_the_one_drawn_at_accept() {
        let record = crate::testkit::mission(130);
        let mut active = ActiveMission::accepted(&record, &offer(), today());
        active.ships.name = "Rover".to_owned();
        assert_eq!(
            MissionValues::active(&active).ship_name.as_deref(),
            Some("Rover")
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

    // Before any pilot.

    /// A catalog of `ships`, each named as given.
    fn ships_named(ships: &[(i16, &str)]) -> crate::testkit::FakePilotCatalog {
        let mut catalog = crate::testkit::catalog();
        catalog.ship_records = ships
            .iter()
            .map(|&(id, name)| crate::catalog::ShipRecord {
                name: name.to_owned(),
                ..crate::testkit::ship(id, crate::testkit::FAST)
            })
            .collect();
        catalog
    }

    #[test]
    fn before_any_pilot_the_player_tags_read_empty_and_the_mission_tags_unset() {
        let text = "<PN>|<PNN>|<PSN>|<PST>|<OSN>|<PRK>|<SRK>|<RRK>|<PRK128>|<REG>|\
                    <DST>|<DSY>|<RST>|<RSY>|<CT>|<CQ>|<SN>|<DL>|<PAY>|<XYZ>";
        let ships = ships_named(&[(128, "Shuttle")]);
        assert_eq!(
            expand_before_any_pilot(text, &ships),
            "|||Shuttle|[Error]|captain|captain|captain|captain||\
             [Error]|[Error]|[Error]|[Error]|[Error]|[Error]|[Error]|[Error]|[Error]|<XYZ>"
        );
    }

    #[test]
    fn before_any_pilot_the_ship_type_is_ship_class_0s_name() {
        let ships = ships_named(&[(129, "Shuttle"), (128, "Pod"), (130, "Kestrel")]);
        assert_eq!(expand_before_any_pilot("a <PST>.", &ships), "a Pod.");
    }

    #[test]
    fn before_any_pilot_with_no_ship_128_the_ship_type_reads_empty() {
        let ships = ships_named(&[(129, "Shuttle"), (130, "Kestrel")]);
        assert_eq!(expand_before_any_pilot("a <PST>.", &ships), "a .");
        assert_eq!(
            expand_before_any_pilot("a <PST>.", &ships_named(&[])),
            "a ."
        );
    }

    #[test]
    fn before_any_pilot_a_text_with_no_tags_is_as_written() {
        let text = "Thanks for playing EV Nova!";
        let ships = ships_named(&[(128, "Shuttle")]);
        assert_eq!(expand_before_any_pilot(text, &ships), text);
    }
}
