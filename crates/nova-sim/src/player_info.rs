//! What the player info screen shows, as `_DoPlayerInfoDialog`
//! (0x5a00a in the `EV Nova` executable) and its helpers decide it. The
//! session gathers the facts ([`Session::player_info`](crate::Session::player_info));
//! `nova_view::ui::player_info` lays the dialog out and draws it.
//!
//! **The General tab** (`_PlayerInfoDialogUpdate` @0x527b0-0x53731).
//!
//! - **Shield and armour** ([`reserve_shown`], @0x52fcc-0x530b3,
//!   @0x5311b-0x53212): with a capacity above 0, a gauge below 0 shows 0%,
//!   one at or under the capacity `trunc(gauge / capacity x 100)`%, and
//!   one over it 100%; with none, "N/A".
//! - **Energy** ([`energy_shown`], @0x5327a-0x53451): with a fuel
//!   capacity above 0, `trunc(fuel / capacity x 100)`%, then the jumps
//!   `trunc(fuel / 100)` and whether any maneuvering fuel is spare
//!   (`(int)fuel % 100 > 0`), or "maneuvering energy only" when no jump
//!   is left but fuel is; with none, "None".
//! - **Turn, accel and speed** ([`turn_shown`], [`accel_shown`],
//!   [`speed_shown`]): the original's `trunc(_ShipTurnRate x 30)`,
//!   `trunc(_ShipAccelRate x 2500)` and its Max Speed come to
//!   `trunc(maneuver x 3)`, `(Accel + outfits) / 2` and `Speed + outfits`
//!   on the ship's [`HandlingTotals`], each floored at 0 as the engine
//!   floors its handling. These equal the original's float path for whole
//!   totals; it is not replicated beyond that, in the spirit of not
//!   copying word widths. Its 1.0°/frame turn floor and its 1.5 speed
//!   bonus outside Strict Play are crons-news-ranks-fidelity phases 30
//!   and 31.
//! - **Legal Status** ([`legal_status_index`], `_DrawLegalStatusString`
//!   @0xd43f-0xd66d). With r the player's record in the system and t
//!   the `CrimeTol` of its government (government slot 0's when it is
//!   independent, @0xd47a-0xd4a7), the index is 2 when r < 0, then 3-9
//!   as r < -t, -4t, -16t, -64t, -256t, -1024t, -4096t; 1 when r = 0; and
//!   10-15 as r > 0, 4t, 16t, 64t, 256t, 1024t, each test over the one
//!   before (@0xd4ab-0xd596). The system's first three `NavDefs` are then
//!   scanned (@0xd599-0xd5fd): a stellar counts when it is active, not
//!   uninhabited, landable and no gate; any counting one dominated makes
//!   the index 16, all of them 17 ([`Domination`], @0xd5ff-0xd611). A
//!   xenophobic government shows "N/A" (@0xd614-0xd64a); otherwise the
//!   index picks `_legalStatusStrings[index]`, which `_LoadStrings`
//!   fills from `STR#` 134 #1-#18 (entry n is #n+1, @0x71c29-0x71c5b),
//!   [`LEGAL_STATUS_STRINGS`]. The row shows it only when
//!   `_SystemIsInhabited` (0x4c4a, [`system_is_inhabited`]): one of the
//!   system's first **four** `NavDefs` is neither uninhabited nor a gate;
//!   otherwise "N/A". The original's index 0, also "N/A", is never
//!   reached.
//! - **Daily income** ([`daily_income`], [`income_line`],
//!   @0x53456-0x53731). Each hired escort adds its class's `Cost x 0.01`
//!   to the expenses, the running total truncated after each
//!   (@0x534f2-0x5351c). The income is the tribute of the dominated
//!   stellars whose owner is visible (@0x5352c-0x53583,
//!   [`tribute_of`]) plus the ranks' salaries (@0x53585-0x535c1); one
//!   below 0 moves to the expenses (@0x535c3-0x535cd). The line shows
//!   only when either is above 0: "Expenses:" when the income is the
//!   less, else "Net Income:" with expenses, else "Income:".
//!
//! **The Cargo text** ([`cargo_text`], `_SetupPlayerInfoText`
//! @0x53cde-0x545e4). A 256-slot table, each slot from -1: a commodity
//! held above 0 and a mission's typed cargo aboard (tons 0 or more) add
//! their tons to their type's slot from `max(slot, 0)`, so mission cargo
//! merges with the commodity of its type and 0 tons still lists it. The
//! items are the listed slots, by type, then the `jünk` held, by ID; N
//! counts them. The heading is "Current cargo in your ship:" (or "in
//! your fleet" when the fleet holds more than the ship), then, with
//! N > 0, the items: a type whose `STR#` 4001 name begins with '*' bare,
//! without its '*'; a slot of tons above 0, and every `jünk`, as "five
//! tons of food" ([`ordinal_number`], capitalised for the first item); a
//! 0-ton slot as its name. Between items ", ", and before the last ", and
//! " (", " after a starred one, and no comma with two items). The list's
//! first character is capitalised, then ".\r\r", then the free space:
//! "Free cargo space: 3 tons", or, with a fleet, "Free cargo space in
//! your fleet:  12 tons", followed by "  (5 tons free in your ship)" when
//! the room for mission cargo differs from it and is above 0, "(none free
//! in your ship)" when it is 0, and nothing when they are equal. With
//! N = 0 the text is the heading alone (@0x53e5b). A type above 255,
//! which writes past the original's table, is left out.
//!
//! **The Extras text** ([`extras_text`], @0x545e4-0x54bf3). Outfits are
//! grouped under a leader: the first lower-numbered outfit with an
//! identical, non-empty `LCName` ([`outfit_leaders`],
//! `_LoadObjectData` @0x7907e-0x791c0). Each outfit owned with an
//! `LCName` and without [`OutfitFlags::HONOR`] adds its count to its
//! leader's group, and the highest `DispWeight` among the groups' leaders
//! starts the passes. A pass places, by ID, each group whose leader
//! (with an `LCName`, without `HONOR`) has that weight; the next weight is
//! the highest below it among the rest, taken over from its start of 0
//! by the first such group only when nothing came before it in the pass
//! (@0x54aa0-0x54ac8), so groups of negative weight are dropped whenever
//! one of weight 0 or more precedes them. The passes stop after one that
//! places nothing. Each item is its count ("a" or "an" by the `LCName`'s
//! first letter, "two", "three", or digits), the first item's first
//! letter capitalised when its count is 3 or less, then its `LCPlural`
//! above 1 or its `LCName`; separated by "," when N > 2, and " " or " and
//! " before the last. Then ".\r\r" and, with the ship's trade-in above 0,
//! "Ship trade-in value: 12,500 credits\r" (`_StrcatCommaNumber`, as
//! [`RuleKey::PayMillions`](crate::RuleKey::PayMillions) reads it).
//!
//! **The Honors text** ([`honors_text`], @0x54bf9-0x551ef). The groups
//! are formed again from the outfits owned with an `LCName` and `HONOR`
//! ([`special_groups`]); those whose leader has an `LCName` and `HONOR`
//! follow the ranks. With N, the ranks listed plus those groups, at 0
//! nothing is written. Otherwise "Your ranks and honors:\r\r", phase 3's
//! ranks line ([`rank::pilot_info`](crate::rank::pilot_info)) with the
//! groups as its following items, then each group as in Extras, its first
//! letter capitalised only when no rank was listed, separated by ", ",
//! with no "and" among them, then "\r".
//!
//! **Whether a tab has text** ([`has_any_cargo`], [`extras_shown`],
//! [`honors_shown`]). Cargo: a commodity or `jünk` above 0, or a
//! mission's typed cargo aboard (`_PlayerHasAnyCargo` 0x5d77). Extras: an
//! outfit owned with an `LCName` and without `HONOR` (@0x53836-0x5385e).
//! Honors: an active rank with a record, or an outfit owned with an
//! `LCName` and `HONOR` (@0x538f7-0x53963). Jettison is offered when
//! there is a commodity or `jünk` above 0, or a mission's typed cargo
//! aboard that can be aborted ([`has_jettisonable_cargo`],
//! `_PlayerHasJettisonableCargo` 0x825a).
//!
//! The strings the texts read, and the General tab's labels, words and
//! messages, are `STR#` 2002's ([`Words`]), carried in the
//! [`PlayerInfo`] so the screen draws the same entries; a missing one
//! reads empty, as `_StrcatIndCString` appends nothing. The original
//! keeps its counts and tons in 16 bits; they are kept wider here. Nothing
//! here plays a sound.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::catalog::{LandingSite, OutfitId, OutfitRecord};
use crate::outfitter::OutfitFlags;
use crate::rank::PilotInfoRanks;
use crate::rulebook::RuleSource;
use crate::stats::HandlingTotals;
use crate::text_tags::comma_number;

/// `STR#` 137 #29-#38 of `Nova-DF.rsrc`: the numbers one to ten in
/// words, which `_StrcatOrdinalNumber` @0x8ab4 writes counts in.
pub const NUMBER_WORDS: [&str; 10] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

/// `STR#` 134, the legal statuses `_legalStatusStrings` holds: entry n
/// is #n+1 (`_LoadStrings` @0x71c29-0x71c5b).
pub const LEGAL_STATUS_STRINGS: i16 = 134;

/// How many standard commodities the cargo table counts.
pub const COMMODITIES: usize = 6;

/// `n` as `_StrcatOrdinalNumber` (0x8ab4) writes it: 1 to 10 in words
/// ([`NUMBER_WORDS`]), anything else in digits, its first character
/// capitalised when `capitalise`.
#[must_use]
pub fn ordinal_number(n: i64, capitalise: bool) -> String {
    let word = usize::try_from(n)
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|index| NUMBER_WORDS.get(index));
    let mut text = word.map_or_else(|| n.to_string(), |word| (*word).to_owned());
    if capitalise {
        capitalise_at(&mut text, 0);
    }
    text
}

/// Upper-cases the ASCII letter at byte `at` of `text`, as `toupper`
/// does; anything else is left as it is.
fn capitalise_at(text: &mut String, at: usize) {
    if let Some(range) = text.get(at..).and_then(|rest| {
        let c = rest.chars().next()?;
        Some(at..at + c.len_utf8())
    }) {
        let upper = text[range.clone()].to_ascii_uppercase();
        text.replace_range(range, &upper);
    }
}

/// The `STR#` 2002 strings the player info screen shows: the texts'
/// words and the General tab's labels, words and messages, read once so
/// that every place on the screen shows the same entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words {
    /// #1 "ton".
    pub ton: Cow<'static, str>,
    /// #2 "tons".
    pub tons: Cow<'static, str>,
    /// #8 "Energy Status:", `_fuelString[5]`, the Energy row's label.
    pub energy_label: Cow<'static, str>,
    /// #9 "maneuvering energy".
    pub maneuvering: Cow<'static, str>,
    /// #12 "Shield Status:", `_shieldString[1]`, the Shield row's label.
    pub shield_label: Cow<'static, str>,
    /// #17 "Armor Status:", `_armorString[1]`, the Armor row's label.
    pub armor_label: Cow<'static, str>,
    /// #32 "credit".
    pub credit: Cow<'static, str>,
    /// #33 "credits", `_creditsString`.
    pub credits: Cow<'static, str>,
    /// #239 "jump".
    pub jump: Cow<'static, str>,
    /// #240 "jumps".
    pub jumps: Cow<'static, str>,
    /// #251 "Pilot Name:".
    pub pilot_name_label: Cow<'static, str>,
    /// #252 "Current Date:".
    pub date_label: Cow<'static, str>,
    /// #253 "System:".
    pub system_label: Cow<'static, str>,
    /// #254 "Combat Rating:".
    pub combat_rating_label: Cow<'static, str>,
    /// #255 "Ship Name:".
    pub ship_name_label: Cow<'static, str>,
    /// #256 "Ship Class:".
    pub ship_class_label: Cow<'static, str>,
    /// #257 "Turn Rate:".
    pub turn_rate_label: Cow<'static, str>,
    /// #258 "°/sec", after the turn rate.
    pub degrees_per_second: Cow<'static, str>,
    /// #259 "Accel Rate:".
    pub accel_label: Cow<'static, str>,
    /// #260 "Max Speed:".
    pub max_speed_label: Cow<'static, str>,
    /// #261 "Failed", the armour of a dying ship, which the engine never shows: it has no dying state (rdm:phase/player-death/phase-1-hide-destroyed-player-ship).
    pub failed: Cow<'static, str>,
    /// #262 "plus".
    pub plus: Cow<'static, str>,
    /// #263 "only".
    pub only: Cow<'static, str>,
    /// #264 "Expenses:".
    pub expenses: Cow<'static, str>,
    /// #265 "Income:".
    pub income: Cow<'static, str>,
    /// #266 "Net".
    pub net: Cow<'static, str>,
    /// #267 "per day".
    pub per_day: Cow<'static, str>,
    /// #268 "You don't have any cargo aboard your fleet's ships".
    pub no_fleet_cargo: Cow<'static, str>,
    /// #269 "You don't have any cargo aboard your ship".
    pub no_cargo: Cow<'static, str>,
    /// #270 "You don't have any extras on your ship".
    pub no_extras: Cow<'static, str>,
    /// #271 "You don't have any ranks or honors".
    pub no_honors: Cow<'static, str>,
    /// #272 "Current extras for your ship:".
    pub extras_heading: Cow<'static, str>,
    /// #273 "Ship trade-in value:".
    pub trade_in: Cow<'static, str>,
    /// #274 "Your ranks and honors:".
    pub honors_heading: Cow<'static, str>,
    /// #291 "Are you sure you want to jettison your cargo?", Jettison's question.
    pub jettison_question: Cow<'static, str>,
    /// #326 "Legal Status:".
    pub legal_status_label: Cow<'static, str>,
    /// #335 "None", the energy with no capacity.
    pub no_energy: Cow<'static, str>,
    /// #336 "none".
    pub none: Cow<'static, str>,
    /// #362 "Current cargo".
    pub current_cargo: Cow<'static, str>,
    /// #364 "Free cargo space".
    pub free_cargo_space: Cow<'static, str>,
    /// #365 "in your ship".
    pub in_your_ship: Cow<'static, str>,
    /// #366 "in your fleet".
    pub in_your_fleet: Cow<'static, str>,
    /// #369 "free in your ship".
    pub free_in_your_ship: Cow<'static, str>,
    /// #391 "of".
    pub of: Cow<'static, str>,
    /// #392 "and".
    pub and: Cow<'static, str>,
    /// #393 "a".
    pub a: Cow<'static, str>,
    /// #394 "an".
    pub an: Cow<'static, str>,
    /// #396 "N/A".
    pub not_available: Cow<'static, str>,
}

impl Words {
    /// The stock strings.
    pub const STOCK: Self = Self {
        ton: Cow::Borrowed("ton"),
        tons: Cow::Borrowed("tons"),
        energy_label: Cow::Borrowed("Energy Status:"),
        maneuvering: Cow::Borrowed("maneuvering energy"),
        shield_label: Cow::Borrowed("Shield Status:"),
        armor_label: Cow::Borrowed("Armor Status:"),
        credit: Cow::Borrowed("credit"),
        credits: Cow::Borrowed("credits"),
        jump: Cow::Borrowed("jump"),
        jumps: Cow::Borrowed("jumps"),
        pilot_name_label: Cow::Borrowed("Pilot Name:"),
        date_label: Cow::Borrowed("Current Date:"),
        system_label: Cow::Borrowed("System:"),
        combat_rating_label: Cow::Borrowed("Combat Rating:"),
        ship_name_label: Cow::Borrowed("Ship Name:"),
        ship_class_label: Cow::Borrowed("Ship Class:"),
        turn_rate_label: Cow::Borrowed("Turn Rate:"),
        degrees_per_second: Cow::Borrowed("°/sec"),
        accel_label: Cow::Borrowed("Accel Rate:"),
        max_speed_label: Cow::Borrowed("Max Speed:"),
        failed: Cow::Borrowed("Failed"),
        plus: Cow::Borrowed("plus"),
        only: Cow::Borrowed("only"),
        expenses: Cow::Borrowed("Expenses:"),
        income: Cow::Borrowed("Income:"),
        net: Cow::Borrowed("Net"),
        per_day: Cow::Borrowed("per day"),
        no_fleet_cargo: Cow::Borrowed("You don't have any cargo aboard your fleet's ships"),
        no_cargo: Cow::Borrowed("You don't have any cargo aboard your ship"),
        no_extras: Cow::Borrowed("You don't have any extras on your ship"),
        no_honors: Cow::Borrowed("You don't have any ranks or honors"),
        extras_heading: Cow::Borrowed("Current extras for your ship:"),
        trade_in: Cow::Borrowed("Ship trade-in value:"),
        honors_heading: Cow::Borrowed("Your ranks and honors:"),
        jettison_question: Cow::Borrowed("Are you sure you want to jettison your cargo?"),
        legal_status_label: Cow::Borrowed("Legal Status:"),
        no_energy: Cow::Borrowed("None"),
        none: Cow::Borrowed("none"),
        current_cargo: Cow::Borrowed("Current cargo"),
        free_cargo_space: Cow::Borrowed("Free cargo space"),
        in_your_ship: Cow::Borrowed("in your ship"),
        in_your_fleet: Cow::Borrowed("in your fleet"),
        free_in_your_ship: Cow::Borrowed("free in your ship"),
        of: Cow::Borrowed("of"),
        and: Cow::Borrowed("and"),
        a: Cow::Borrowed("a"),
        an: Cow::Borrowed("an"),
        not_available: Cow::Borrowed("N/A"),
    };

    /// The words `STR#` 2002 `list` holds, each read by its string
    /// number (1-based), empty when the list lacks it.
    #[must_use]
    pub fn from_list(list: &[String]) -> Self {
        let at = |n: usize| Cow::Owned(list.get(n - 1).cloned().unwrap_or_default());
        Self {
            ton: at(1),
            tons: at(2),
            energy_label: at(8),
            maneuvering: at(9),
            shield_label: at(12),
            armor_label: at(17),
            credit: at(32),
            credits: at(33),
            jump: at(239),
            jumps: at(240),
            pilot_name_label: at(251),
            date_label: at(252),
            system_label: at(253),
            combat_rating_label: at(254),
            ship_name_label: at(255),
            ship_class_label: at(256),
            turn_rate_label: at(257),
            degrees_per_second: at(258),
            accel_label: at(259),
            max_speed_label: at(260),
            failed: at(261),
            plus: at(262),
            only: at(263),
            expenses: at(264),
            income: at(265),
            net: at(266),
            per_day: at(267),
            no_fleet_cargo: at(268),
            no_cargo: at(269),
            no_extras: at(270),
            no_honors: at(271),
            extras_heading: at(272),
            trade_in: at(273),
            honors_heading: at(274),
            jettison_question: at(291),
            legal_status_label: at(326),
            no_energy: at(335),
            none: at(336),
            current_cargo: at(362),
            free_cargo_space: at(364),
            in_your_ship: at(365),
            in_your_fleet: at(366),
            free_in_your_ship: at(369),
            of: at(391),
            and: at(392),
            a: at(393),
            an: at(394),
            not_available: at(396),
        }
    }

    /// The Credits row's label: #33 (`_creditsString`) with its first
    /// letter capitalised, then ":".
    #[must_use]
    pub fn credits_label(&self) -> String {
        let mut label = self.credits.to_string();
        capitalise_at(&mut label, 0);
        label.push(':');
        label
    }

    /// "ton" for 1, else "tons".
    fn tons_for(&self, n: i64) -> &str {
        if n == 1 { &self.ton } else { &self.tons }
    }
}

/// A mission's cargo, as the Cargo text reads it: only a typed cargo
/// aboard of tons 0 or more is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissionLoad {
    /// Its cargo type, raw.
    pub cargo_type: i16,
    /// Its tons.
    pub tons: i64,
    /// Whether its mission can be aborted (`CanAbort`), which makes it
    /// jettisonable.
    pub can_abort: bool,
}

/// What the Cargo text and the cargo tests are decided from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldCargo<'a> {
    /// The tons held of each standard commodity, by number.
    pub commodities: [i64; COMMODITIES],
    /// The missions' cargo aboard, in slot order.
    pub missions: Vec<MissionLoad>,
    /// Each `jünk` held above 0, by ID: its `LCName` and the count.
    pub junk: Vec<(&'a str, i64)>,
    /// The cargo types' names, `STR#` 4001 in order.
    pub cargo_names: &'a [String],
    /// Whether the fleet holds more than the ship (`_TotalFleetHolds` >
    /// `_ShipTotalHolds`).
    pub fleet: bool,
    /// The fleet's holds less the cargo held.
    pub free: i64,
    /// The room left for mission cargo (`_TotalMissionCargoSpace`).
    pub mission_room: i64,
}

/// One item of the cargo list.
enum CargoItem<'a> {
    /// A slot of the table: its type's name and tons.
    Slot(&'a str, i64),
    /// A `jünk`: its `LCName` and count.
    Junk(&'a str, i64),
}

/// The listed slots of the table, by type: each type held, its tons.
fn cargo_slots(cargo: &HeldCargo) -> BTreeMap<u8, i64> {
    let mut slots: BTreeMap<u8, i64> = BTreeMap::new();
    let mut add = |slot: u8, tons: i64| {
        let held = slots.entry(slot).or_insert(0);
        *held += tons;
    };
    for (n, &tons) in (0_u8..).zip(&cargo.commodities) {
        if tons > 0 {
            add(n, tons);
        }
    }
    for load in &cargo.missions {
        if let Ok(slot) = u8::try_from(load.cargo_type) {
            add(slot, load.tons);
        }
    }
    slots
}

/// The Cargo text (see the module docs).
#[must_use]
pub fn cargo_text(cargo: &HeldCargo, words: &Words) -> String {
    let mut text = format!(
        "{} {}:\r\r",
        words.current_cargo,
        if cargo.fleet {
            &words.in_your_fleet
        } else {
            &words.in_your_ship
        }
    );
    let slots = cargo_slots(cargo);
    let items: Vec<CargoItem> = slots
        .iter()
        .map(|(&slot, &tons)| {
            let name = cargo
                .cargo_names
                .get(usize::from(slot))
                .map_or("", String::as_str);
            CargoItem::Slot(name, tons)
        })
        .chain(
            cargo
                .junk
                .iter()
                .filter(|(_, count)| *count > 0)
                .map(|&(name, count)| CargoItem::Junk(name, count)),
        )
        .collect();
    let n = items.len();
    if n == 0 {
        return text;
    }
    let start = text.len();
    let quantity = |text: &mut String, tons: i64, first: bool, name: &str| {
        text.push_str(&ordinal_number(tons, first));
        text.push(' ');
        text.push_str(words.tons_for(tons));
        text.push(' ');
        text.push_str(&words.of);
        text.push(' ');
        text.push_str(name);
    };
    for (k, item) in items.iter().enumerate() {
        let starred = match *item {
            CargoItem::Slot(name, tons) => match name.strip_prefix('*') {
                Some(bare) => {
                    text.push_str(bare);
                    true
                }
                None if tons > 0 => {
                    quantity(&mut text, tons, k == 0, name);
                    false
                }
                None => {
                    text.push_str(name);
                    false
                }
            },
            CargoItem::Junk(name, count) => {
                quantity(&mut text, count, k == 0, name);
                false
            }
        };
        if k + 2 < n || (k + 2 == n && starred) {
            text.push_str(", ");
        } else if k + 2 == n {
            if n > 2 {
                text.push(',');
            }
            text.push(' ');
            text.push_str(&words.and);
            text.push(' ');
        }
    }
    capitalise_at(&mut text, start);
    text.push_str(".\r\r");
    text.push_str(&free_space(cargo, words));
    text
}

/// The Cargo text's free space line, nothing when none is free (see the
/// module docs).
fn free_space(cargo: &HeldCargo, words: &Words) -> String {
    let free = cargo.free;
    if free <= 0 {
        return String::new();
    }
    if !cargo.fleet {
        return format!(
            "{}: {free} {}",
            words.free_cargo_space,
            words.tons_for(free)
        );
    }
    let room = cargo.mission_room.max(0);
    let in_ship = if room > 0 && room != free {
        format!(
            "  ({room} {} {})",
            words.tons_for(room),
            words.free_in_your_ship
        )
    } else if room == 0 {
        format!("({} {})", words.none, words.free_in_your_ship)
    } else {
        String::new()
    };
    format!(
        "{} {}:  {free} {}{in_ship}",
        words.free_cargo_space,
        words.in_your_fleet,
        words.tons_for(free)
    )
}

/// Whether there is any cargo to list (`_PlayerHasAnyCargo` 0x5d77): a
/// commodity or `jünk` above 0, or any mission's cargo aboard.
#[must_use]
pub fn has_any_cargo(cargo: &HeldCargo) -> bool {
    cargo.commodities.iter().any(|&tons| tons > 0)
        || !cargo.missions.is_empty()
        || cargo.junk.iter().any(|(_, count)| *count > 0)
}

/// Whether there is cargo Jettison can drop
/// (`_PlayerHasJettisonableCargo` 0x825a): a commodity or `jünk` above
/// 0, or a mission's cargo aboard whose mission can be aborted.
#[must_use]
pub fn has_jettisonable_cargo(cargo: &HeldCargo) -> bool {
    cargo.commodities.iter().any(|&tons| tons > 0)
        || cargo.junk.iter().any(|(_, count)| *count > 0)
        || cargo.missions.iter().any(|load| load.can_abort)
}

/// The lowest and highest outfit IDs the original loads (its 512 slots).
const OUTFIT_IDS: std::ops::RangeInclusive<i16> = 128..=639;

/// Each loaded outfit's group leader, as `_LoadObjectData` sets it
/// (@0x7907e-0x791c0): the first lower-numbered outfit of `records`
/// with an identical, non-empty `LCName`, owned or not, else the outfit
/// itself. Only IDs 128-639, the original's slots, are grouped.
#[must_use]
pub fn outfit_leaders(records: &[OutfitRecord]) -> BTreeMap<OutfitId, OutfitId> {
    let mut loaded: Vec<&OutfitRecord> = records
        .iter()
        .filter(|record| OUTFIT_IDS.contains(&record.id.0))
        .collect();
    loaded.sort_by_key(|record| record.id);
    loaded
        .iter()
        .enumerate()
        .map(|(at, record)| {
            let leader = loaded[..at]
                .iter()
                .find(|other| !record.lc_name.is_empty() && other.lc_name == record.lc_name)
                .map_or(record.id, |other| other.id);
            (record.id, leader)
        })
        .collect()
}

/// The record of outfit `id` among `records`, if any.
fn record_of(records: &[OutfitRecord], id: OutfitId) -> Option<&OutfitRecord> {
    records.iter().find(|record| record.id == id)
}

/// Whether `record` has its `HONOR` flag set as `honor` says.
fn honor_is(record: &OutfitRecord, honor: bool) -> bool {
    (record.flags & OutfitFlags::HONOR != 0) == honor
}

/// The groups of the outfits `owned` with an `LCName` and with `HONOR` as
/// `honor` says, by leader in ID order: each leader with the counts of
/// its group summed.
fn groups(
    records: &[OutfitRecord],
    owned: &BTreeMap<OutfitId, u16>,
    honor: bool,
) -> BTreeMap<OutfitId, i64> {
    let leaders = outfit_leaders(records);
    let mut groups = BTreeMap::new();
    for (&id, &count) in owned {
        let Some(record) = record_of(records, id) else {
            continue;
        };
        if count == 0 || record.lc_name.is_empty() || !honor_is(record, honor) {
            continue;
        }
        let leader = leaders.get(&id).copied().unwrap_or(id);
        *groups.entry(leader).or_insert(0) += i64::from(count);
    }
    groups
}

/// The count word of an item of `count` named `lc_name`: "a" or "an" by
/// its first letter for 1, "two" and "three", digits above.
fn count_word(count: i64, lc_name: &str, words: &Words) -> String {
    match count {
        1 => {
            let vowel = lc_name
                .chars()
                .next()
                .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'));
            (if vowel { &words.an } else { &words.a }).to_string()
        }
        2 => NUMBER_WORDS[1].to_owned(),
        3 => NUMBER_WORDS[2].to_owned(),
        _ => count.to_string(),
    }
}

/// Appends an item of `count` of `record` to `text`: its count word and a
/// space, the count word's first letter capitalised when `capitalise`
/// and the count is 3 or less, then its `LCPlural` above 1 or its
/// `LCName`.
fn push_item(
    text: &mut String,
    count: i64,
    record: &OutfitRecord,
    capitalise: bool,
    words: &Words,
) {
    let start = text.len();
    text.push_str(&count_word(count, &record.lc_name, words));
    text.push(' ');
    if capitalise && count <= 3 {
        capitalise_at(text, start);
    }
    text.push_str(if count > 1 {
        &record.lc_plural
    } else {
        &record.lc_name
    });
}

/// The Extras text of the outfits `owned` (see the module docs), with the
/// ship's `trade_in` written as `millions` says.
#[must_use]
pub fn extras_text(
    records: &[OutfitRecord],
    owned: &BTreeMap<OutfitId, u16>,
    words: &Words,
    trade_in: i64,
    millions: RuleSource,
) -> String {
    let mut text = format!("{}\r\r", words.extras_heading);
    let mut left: Vec<(&OutfitRecord, i64)> = groups(records, owned, false)
        .into_iter()
        .filter_map(|(leader, count)| Some((record_of(records, leader)?, count)))
        .collect();
    let n = left.len();
    let mut weight = left.iter().map(|(record, _)| record.disp_weight).max();
    let mut placed = 0;
    while let Some(w) = weight {
        let mut placed_now = 0;
        // Whether any group, placed or not, came before in this pass.
        let mut seen = false;
        let mut next = 0;
        for (record, count) in &mut left {
            // The original also skips a leader with no `LCName`
            // (@0x547d6), which none is: a leader's name is its group's.
            if *count <= 0 || !honor_is(record, false) {
                continue;
            }
            let dw = record.disp_weight;
            if dw == w {
                push_item(&mut text, *count, record, placed == 0, words);
                placed += 1;
                placed_now += 1;
                if placed < n && n > 2 {
                    text.push(',');
                }
                if placed + 1 < n {
                    text.push(' ');
                } else if placed + 1 == n {
                    text.push(' ');
                    text.push_str(&words.and);
                    text.push(' ');
                }
                *count = 0;
            } else {
                // The next weight is the highest below `w`, from its start
                // of 0 unless this is the first group of the pass. The
                // original also asks `w > dw` (@0x54a9e), which always
                // holds here: every group left is at or below `w`, as the
                // passes go down by the highest weight left.
                next = if seen { next.max(dw) } else { dw };
            }
            seen = true;
        }
        weight = (placed_now > 0).then_some(next);
    }
    text.push_str(".\r\r");
    if trade_in > 0 {
        let credits = if trade_in == 1 {
            &words.credit
        } else {
            &words.credits
        };
        let _ = write!(
            text,
            "{} {} {credits}\r",
            words.trade_in,
            comma_number(trade_in, millions)
        );
    }
    text
}

/// Whether the Extras tab has text: an outfit owned with an `LCName` and
/// without `HONOR` (@0x53836-0x5385e).
#[must_use]
pub fn extras_shown(records: &[OutfitRecord], owned: &BTreeMap<OutfitId, u16>) -> bool {
    !groups(records, owned, false).is_empty()
}

/// The Honors tab's groups: those of the outfits owned with an `LCName`
/// and `HONOR`, by leader in ID order, whose leader has an `LCName` and
/// `HONOR` itself (@0x54c82-0x54d4b), each with its count.
#[must_use]
pub fn special_groups(
    records: &[OutfitRecord],
    owned: &BTreeMap<OutfitId, u16>,
) -> Vec<(OutfitId, i64)> {
    groups(records, owned, true)
        .into_iter()
        .filter(|&(leader, _)| {
            record_of(records, leader)
                .is_some_and(|record| !record.lc_name.is_empty() && honor_is(record, true))
        })
        .collect()
}

/// The Honors text (see the module docs): `ranks`, the ranks line built
/// with `specials` following, then `specials`; `None` when there is
/// nothing to list.
#[must_use]
pub fn honors_text(
    ranks: &PilotInfoRanks,
    specials: &[(OutfitId, i64)],
    records: &[OutfitRecord],
    words: &Words,
) -> Option<String> {
    let n = ranks.listed + specials.len();
    if n == 0 {
        return None;
    }
    let mut text = format!("{}\r\r{}", words.honors_heading, ranks.line);
    let mut k = ranks.listed;
    for &(leader, count) in specials {
        let Some(record) = record_of(records, leader) else {
            continue;
        };
        push_item(&mut text, count, record, k == 0, words);
        k += 1;
        if k < n {
            text.push_str(", ");
        }
    }
    text.push('\r');
    Some(text)
}

/// Whether the Honors tab has text: `any_rank` (an active rank with a
/// record, named or not), or an outfit owned with an `LCName` and
/// `HONOR` (@0x538f7-0x53963).
#[must_use]
pub fn honors_shown(
    any_rank: bool,
    records: &[OutfitRecord],
    owned: &BTreeMap<OutfitId, u16>,
) -> bool {
    any_rank || !groups(records, owned, true).is_empty()
}

/// The day's income and expenses, before the line is chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DailyIncome {
    /// The tribute and salaries, 0 or more.
    pub income: i64,
    /// The escorts' wages, and any income below 0.
    pub expenses: i64,
}

/// The daily income (see the module docs) of hired escorts of class
/// `costs`, `tribute` and `salaries`.
#[must_use]
pub fn daily_income(
    costs: impl IntoIterator<Item = i32>,
    tribute: i64,
    salaries: i64,
) -> DailyIncome {
    let mut expenses = 0_i64;
    for cost in costs {
        expenses = (expenses as f64 + f64::from(cost) * 0.01).trunc() as i64;
    }
    let income = tribute + salaries;
    // Income below 0 moves to the expenses.
    let moved = income.min(0);
    DailyIncome {
        income: income - moved,
        expenses: expenses - moved,
    }
}

/// The daily income line, with its amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncomeLine {
    /// "Expenses:", the expenses less the income.
    Expenses(i64),
    /// "Net Income:", the income less the expenses.
    NetIncome(i64),
    /// "Income:", the income.
    Income(i64),
}

/// The line `day` shows, if any (@0x535cf-0x53731).
#[must_use]
pub fn income_line(day: DailyIncome) -> Option<IncomeLine> {
    let DailyIncome { income, expenses } = day;
    if expenses <= 0 && income <= 0 {
        None
    } else if income < expenses {
        Some(IncomeLine::Expenses(expenses - income))
    } else if expenses != 0 {
        Some(IncomeLine::NetIncome(income - expenses))
    } else {
        Some(IncomeLine::Income(income))
    }
}

/// What a dominated `site` pays a day (`_LoadObjectData`
/// @0x775a3-0x775ce): its `Tribute` when above 0, else its `TechLevel` x
/// 1000.
#[must_use]
pub fn tribute_of(site: &LandingSite) -> i64 {
    if site.tribute > 0 {
        i64::from(site.tribute)
    } else {
        i64::from(site.tech_level) * 1000
    }
}

/// A shield or armour gauge as shown, `current` of `capacity`: `None`
/// ("N/A") with no capacity (see the module docs).
#[must_use]
pub fn reserve_shown(current: f32, capacity: f32) -> Option<i64> {
    // Below 0 is 0% and over the capacity 100%, as the original's
    // branches have it; in between, truncated.
    (capacity > 0.0).then(|| ((current / capacity * 100.0) as i64).clamp(0, 100))
}

/// How far the energy left reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reach {
    /// This many jumps, and whether maneuvering energy is spare.
    Jumps {
        /// The jumps left.
        jumps: i64,
        /// Whether some maneuvering energy is left over.
        spare: bool,
    },
    /// No jump, but maneuvering energy.
    ManeuveringOnly,
    /// Nothing.
    Nothing,
}

/// The energy row's values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyShown {
    /// The fuel as a percentage of the capacity.
    pub percent: i64,
    /// What it reaches.
    pub reach: Reach,
}

/// The energy row of `fuel` of `capacity`: `None` ("None") with no
/// capacity (see the module docs).
#[must_use]
pub fn energy_shown(fuel: f32, capacity: i64) -> Option<EnergyShown> {
    if capacity <= 0 {
        return None;
    }
    let jumps = (fuel / 100.0) as i64;
    let reach = if jumps > 0 {
        Reach::Jumps {
            jumps,
            spare: (fuel as i64) % 100 > 0,
        }
    } else if fuel > 0.0 {
        Reach::ManeuveringOnly
    } else {
        Reach::Nothing
    };
    Some(EnergyShown {
        percent: (fuel / capacity as f32 * 100.0) as i64,
        reach,
    })
}

/// The Max Speed row: `Speed` plus the outfits, floored at 0.
#[must_use]
pub fn speed_shown(totals: &HandlingTotals) -> i64 {
    totals.speed.max(0)
}

/// The Accel Rate row: `(Accel + outfits) / 2`, truncated, floored at 0.
#[must_use]
pub fn accel_shown(totals: &HandlingTotals) -> i64 {
    (totals.accel / 2).max(0)
}

/// The Turn Rate row, in degrees a second: `trunc(maneuver x 3)`,
/// floored at 0.
#[must_use]
pub fn turn_shown(totals: &HandlingTotals) -> i64 {
    ((totals.maneuver * 3.0) as i64).max(0)
}

/// How many of the counting stellars are dominated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domination {
    /// None of them (or there are none).
    None,
    /// Some, not all.
    Some,
    /// Every one.
    All,
}

impl Domination {
    /// The domination of the counting stellars, each dominated or not.
    #[must_use]
    pub fn of(counting: impl IntoIterator<Item = bool>) -> Self {
        let (mut dominated, mut free) = (false, false);
        for is_dominated in counting {
            if is_dominated {
                dominated = true;
            } else {
                free = true;
            }
        }
        match (dominated, free) {
            (false, _) => Self::None,
            (true, false) => Self::All,
            (true, true) => Self::Some,
        }
    }
}

/// The `_legalStatusStrings` index of `record` under a government of
/// `crime_tol`, xenophobic or not, with the counting stellars
/// `dominated` (see the module docs): `None` ("N/A") for a xenophobic
/// government.
#[must_use]
pub fn legal_status_index(
    record: i64,
    crime_tol: i64,
    xenophobic: bool,
    dominated: Domination,
) -> Option<usize> {
    let (r, t) = (record, crime_tol);
    // 2 for a record below 0; a record of 0 or more is always given 1 or
    // 10 below, so the original's start of 0 for it never shows.
    let mut index = 2;
    for (at, times) in [
        (3, 1),
        (4, 4),
        (5, 16),
        (6, 64),
        (7, 256),
        (8, 1024),
        (9, 4096),
    ] {
        if r < -t * times {
            index = at;
        }
    }
    if r == 0 {
        index = 1;
    }
    for (at, times) in [(10, 0), (11, 4), (12, 16), (13, 64), (14, 256), (15, 1024)] {
        if r > t * times {
            index = at;
        }
    }
    match dominated {
        Domination::None => {}
        Domination::Some => index = 16,
        Domination::All => index = 17,
    }
    (!xenophobic).then_some(index)
}

/// Whether a system is inhabited (`_SystemIsInhabited` 0x4c4a): one of
/// its first four `NavDefs`, each given as whether it is uninhabited and
/// whether it is a gate, is neither.
#[must_use]
pub fn system_is_inhabited(stellars: impl IntoIterator<Item = (bool, bool)>) -> bool {
    stellars
        .into_iter()
        .take(4)
        .any(|(uninhabited, gate)| !uninhabited && !gate)
}

/// What the player info screen shows.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerInfo {
    /// The pilot's name.
    pub pilot_name: String,
    /// Today's medium date.
    pub date: String,
    /// The system's name.
    pub system: String,
    /// The legal status string, or "N/A".
    pub legal_status: String,
    /// The ship's name.
    pub ship_name: String,
    /// The ship class's name.
    pub ship_class: String,
    /// The Turn Rate row, in degrees a second.
    pub turn_rate: i64,
    /// The Accel Rate row.
    pub accel: i64,
    /// The Max Speed row.
    pub max_speed: i64,
    /// The player's cash.
    pub credits: i64,
    /// The shield's percentage, `None` for "N/A".
    pub shield: Option<i64>,
    /// The armour's percentage, `None` for "N/A".
    pub armor: Option<i64>,
    /// The energy row, `None` for "None".
    pub energy: Option<EnergyShown>,
    /// The daily income line, if it shows.
    pub income: Option<IncomeLine>,
    /// Whether the fleet holds more than the ship.
    pub fleet: bool,
    /// The Cargo text, `None` when there is no cargo.
    pub cargo: Option<String>,
    /// The Extras text, `None` when there are no extras.
    pub extras: Option<String>,
    /// The Honors text, `None` when there are no ranks or honors.
    pub honors: Option<String>,
    /// Whether Jettison is offered on the Cargo tab.
    pub jettisonable: bool,
    /// The `STR#` 2002 strings the screen shows, the same the texts and
    /// the legal status were written with.
    pub words: Words,
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::testkit::{outfit, planet};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|&name| name.to_owned()).collect()
    }

    /// Stock `STR#` 4001's first entries.
    fn cargo_names() -> Vec<String> {
        names(&[
            "food",
            "industrial goods",
            "medical supplies",
            "luxury goods",
            "metal",
            "equipment",
            "*passengers",
        ])
    }

    fn held(names: &[String]) -> HeldCargo<'_> {
        HeldCargo {
            cargo_names: names,
            ..HeldCargo::default()
        }
    }

    fn load(cargo_type: i16, tons: i64) -> MissionLoad {
        MissionLoad {
            cargo_type,
            tons,
            can_abort: false,
        }
    }

    fn text(cargo: &HeldCargo) -> String {
        cargo_text(cargo, &Words::STOCK)
    }

    /// The list part of `cargo`'s text: after the heading, before ".".
    fn list(cargo: &HeldCargo) -> String {
        let text = text(cargo);
        let rest = text.split_once(":\r\r").expect("a heading").1;
        rest.split_once(".\r\r").expect("a tail").0.to_owned()
    }

    #[test]
    fn ordinal_numbers_are_words_from_one_to_ten() {
        assert_eq!(ordinal_number(1, false), "one");
        assert_eq!(ordinal_number(10, false), "ten");
        assert_eq!(ordinal_number(11, false), "11");
        assert_eq!(ordinal_number(0, false), "0");
        assert_eq!(ordinal_number(-3, false), "-3");
        assert_eq!(ordinal_number(2, true), "Two");
        assert_eq!(ordinal_number(12, true), "12");
    }

    #[test]
    fn the_words_are_read_by_number_and_a_missing_one_is_empty() {
        let list: Vec<String> = (1..=400).map(|n| format!("#{n}")).collect();
        let words = Words::from_list(&list);
        let read = [
            (&words.ton, 1),
            (&words.tons, 2),
            (&words.energy_label, 8),
            (&words.maneuvering, 9),
            (&words.shield_label, 12),
            (&words.armor_label, 17),
            (&words.credit, 32),
            (&words.credits, 33),
            (&words.jump, 239),
            (&words.jumps, 240),
            (&words.pilot_name_label, 251),
            (&words.date_label, 252),
            (&words.system_label, 253),
            (&words.combat_rating_label, 254),
            (&words.ship_name_label, 255),
            (&words.ship_class_label, 256),
            (&words.turn_rate_label, 257),
            (&words.degrees_per_second, 258),
            (&words.accel_label, 259),
            (&words.max_speed_label, 260),
            (&words.failed, 261),
            (&words.plus, 262),
            (&words.only, 263),
            (&words.expenses, 264),
            (&words.income, 265),
            (&words.net, 266),
            (&words.per_day, 267),
            (&words.no_fleet_cargo, 268),
            (&words.no_cargo, 269),
            (&words.no_extras, 270),
            (&words.no_honors, 271),
            (&words.extras_heading, 272),
            (&words.trade_in, 273),
            (&words.honors_heading, 274),
            (&words.jettison_question, 291),
            (&words.legal_status_label, 326),
            (&words.no_energy, 335),
            (&words.none, 336),
            (&words.current_cargo, 362),
            (&words.free_cargo_space, 364),
            (&words.in_your_ship, 365),
            (&words.in_your_fleet, 366),
            (&words.free_in_your_ship, 369),
            (&words.of, 391),
            (&words.and, 392),
            (&words.a, 393),
            (&words.an, 394),
            (&words.not_available, 396),
        ];
        for (word, n) in read {
            assert_eq!(**word, format!("#{n}"), "STR# 2002 #{n}");
        }
        let short = Words::from_list(&names(&["ton"]));
        assert_eq!(&*short.ton, "ton");
        assert_eq!(&*short.tons, "", "past the list's end");
        assert_eq!(&*Words::from_list(&[]).ton, "");
        assert_eq!(&*Words::from_list(&[]).not_available, "");
    }

    #[test]
    fn the_credits_label_is_33_capitalised_with_a_colon() {
        assert_eq!(Words::STOCK.credits_label(), "Credits:");
        let words = Words {
            credits: Cow::Borrowed("crédits"),
            ..Words::STOCK
        };
        assert_eq!(words.credits_label(), "Crédits:");
        let words = Words {
            credits: Cow::Borrowed(""),
            ..Words::STOCK
        };
        assert_eq!(words.credits_label(), ":");
    }

    #[test]
    fn the_cargo_heading_says_ship_or_fleet() {
        let names = cargo_names();
        assert_eq!(text(&held(&names)), "Current cargo in your ship:\r\r");
        let fleet = HeldCargo {
            fleet: true,
            ..held(&names)
        };
        assert_eq!(text(&fleet), "Current cargo in your fleet:\r\r");
    }

    #[test]
    fn one_commodity_is_written_with_its_quantity_in_words() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.commodities[0] = 5;
        assert_eq!(
            text(&cargo),
            "Current cargo in your ship:\r\rFive tons of food.\r\r"
        );
        cargo.commodities[0] = 1;
        assert_eq!(list(&cargo), "One ton of food");
        cargo.commodities[0] = 12;
        assert_eq!(list(&cargo), "12 tons of food");
    }

    #[test]
    fn two_items_are_joined_by_and_and_three_by_commas_too() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.commodities[0] = 5;
        cargo.commodities[4] = 2;
        assert_eq!(list(&cargo), "Five tons of food and two tons of metal");
        cargo.commodities[1] = 3;
        assert_eq!(
            list(&cargo),
            "Five tons of food, three tons of industrial goods, and two tons of metal"
        );
    }

    #[test]
    fn a_starred_cargo_is_bare_and_takes_no_and_after_it() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.missions = vec![load(6, 4)];
        assert_eq!(list(&cargo), "Passengers");
        cargo.junk = vec![("opal", 2)];
        assert_eq!(list(&cargo), "Passengers, two tons of opal");
        cargo.commodities[0] = 1;
        assert_eq!(
            list(&cargo),
            "One ton of food, passengers, two tons of opal",
            "second-to-last: \", \" and no \"and\""
        );
    }

    #[test]
    fn a_zero_ton_mission_cargo_is_its_bare_name() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.missions = vec![load(4, 0)];
        assert_eq!(list(&cargo), "Metal");
    }

    #[test]
    fn mission_cargo_merges_with_the_commodity_of_its_type() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.commodities[1] = 3;
        cargo.missions = vec![load(1, 4)];
        assert_eq!(list(&cargo), "Seven tons of industrial goods");
    }

    #[test]
    fn the_junk_follows_the_commodities_with_its_quantities() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.commodities[0] = 2;
        cargo.junk = vec![("opal", 1), ("gem", 0), ("ore", 11)];
        assert_eq!(
            list(&cargo),
            "Two tons of food, one ton of opal, and 11 tons of ore"
        );
        cargo.commodities[0] = 0;
        cargo.junk = vec![("opal", 1)];
        assert_eq!(list(&cargo), "One ton of opal", "the first capitalised");
    }

    #[test]
    fn a_type_above_255_is_left_out_and_one_past_the_names_is_empty() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.missions = vec![load(256, 3)];
        assert_eq!(text(&cargo), "Current cargo in your ship:\r\r");
        cargo.missions = vec![load(20, 3), load(0, 1)];
        assert_eq!(list(&cargo), "One ton of food and three tons of ");
        cargo.missions = vec![load(-2, 3)];
        assert_eq!(text(&cargo), "Current cargo in your ship:\r\r");
    }

    #[test]
    fn the_free_space_follows_the_list() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.commodities[0] = 1;
        cargo.free = 3;
        assert!(text(&cargo).ends_with(".\r\rFree cargo space: 3 tons"));
        cargo.free = 1;
        assert!(text(&cargo).ends_with(".\r\rFree cargo space: 1 ton"));
        cargo.fleet = true;
        cargo.free = 12;
        cargo.mission_room = 5;
        assert!(text(&cargo).ends_with(
            ".\r\rFree cargo space in your fleet:  12 tons  (5 tons free in your ship)"
        ));
        cargo.mission_room = 1;
        assert!(text(&cargo).ends_with("12 tons  (1 ton free in your ship)"));
        cargo.mission_room = 0;
        assert!(text(&cargo).ends_with("12 tons(none free in your ship)"));
        cargo.mission_room = -4;
        assert!(text(&cargo).ends_with("12 tons(none free in your ship)"));
        cargo.mission_room = 12;
        assert!(text(&cargo).ends_with(":  12 tons"));
        cargo.free = 1;
        cargo.mission_room = 1;
        assert!(text(&cargo).ends_with(":  1 ton"));
        cargo.free = 0;
        assert!(text(&cargo).ends_with("food.\r\r"), "none free: no line");
        cargo.free = -3;
        assert!(text(&cargo).ends_with("food.\r\r"));
    }

    #[test]
    fn with_nothing_listed_the_text_is_the_heading_alone() {
        let names = cargo_names();
        let mut cargo = held(&names);
        cargo.free = 10;
        assert_eq!(text(&cargo), "Current cargo in your ship:\r\r");
    }

    #[test]
    fn any_cargo_counts_a_missions_and_jettisonable_only_an_abortable_one() {
        let names = cargo_names();
        let mut cargo = held(&names);
        assert!(!has_any_cargo(&cargo));
        assert!(!has_jettisonable_cargo(&cargo));
        cargo.missions = vec![load(3, 0)];
        assert!(has_any_cargo(&cargo));
        assert!(!has_jettisonable_cargo(&cargo));
        cargo.missions[0].can_abort = true;
        assert!(has_jettisonable_cargo(&cargo));
        let mut cargo = held(&names);
        cargo.commodities[5] = 1;
        assert!(has_any_cargo(&cargo) && has_jettisonable_cargo(&cargo));
        let mut cargo = held(&names);
        cargo.junk = vec![("opal", 0)];
        assert!(!has_any_cargo(&cargo) && !has_jettisonable_cargo(&cargo));
        cargo.junk = vec![("opal", 1)];
        assert!(has_any_cargo(&cargo) && has_jettisonable_cargo(&cargo));
    }

    /// Outfit `id` named `name`/`plural`, of `DispWeight` `weight`.
    fn named(id: i16, name: &str, plural: &str, weight: i16) -> OutfitRecord {
        OutfitRecord {
            lc_name: name.to_owned(),
            lc_plural: plural.to_owned(),
            disp_weight: weight,
            ..outfit(id, &[])
        }
    }

    fn honor(record: OutfitRecord) -> OutfitRecord {
        OutfitRecord {
            flags: OutfitFlags::HONOR,
            ..record
        }
    }

    fn owned(list: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        list.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    fn extras(records: &[OutfitRecord], list: &[(i16, u16)]) -> String {
        extras_text(records, &owned(list), &Words::STOCK, 0, RuleSource::Engine)
    }

    #[test]
    fn a_leader_is_the_first_lower_outfit_of_the_same_name() {
        let records = [
            named(128, "pod", "pods", 0),
            named(129, "", "", 0),
            named(130, "pod", "pods", 0),
            named(131, "", "", 0),
            named(133, "pod", "pods", 0),
            named(132, "tank", "tanks", 0),
            named(127, "pod", "pods", 0),
            named(640, "pod", "pods", 0),
        ];
        let leaders = outfit_leaders(&records);
        let of = |id| leaders.get(&OutfitId(id)).map(|leader| leader.0);
        assert_eq!(of(128), Some(128));
        assert_eq!(of(129), Some(129));
        assert_eq!(of(130), Some(128));
        assert_eq!(of(131), Some(131), "an empty name leads only itself");
        assert_eq!(of(132), Some(132));
        assert_eq!(of(133), Some(128));
        assert_eq!(of(127), None, "below the slots");
        assert_eq!(of(640), None, "above the slots");
        let unsorted =
            outfit_leaders(&[named(130, "pod", "pods", 0), named(128, "pod", "pods", 0)]);
        assert_eq!(unsorted.get(&OutfitId(130)), Some(&OutfitId(128)), "by ID");
    }

    #[test]
    fn an_extra_is_counted_with_a_or_an_by_its_first_vowel() {
        let records = [
            named(128, "afterburner", "afterburners", 0),
            named(129, "cargo pod", "cargo pods", 0),
            named(130, "Ion cannon", "Ion cannons", 0),
            named(131, "émetteur", "émetteurs", 0),
        ];
        assert_eq!(
            extras(&records, &[(128, 1)]),
            "Current extras for your ship:\r\rAn afterburner.\r\r"
        );
        assert_eq!(
            extras(&records, &[(129, 1)]),
            "Current extras for your ship:\r\rA cargo pod.\r\r"
        );
        assert!(extras(&records, &[(130, 1)]).contains("An Ion cannon."));
        assert!(extras(&records, &[(131, 1)]).contains("A émetteur."));
        for record in &records[..] {
            let vowel = record.lc_name.starts_with(['a', 'e', 'i', 'o', 'u', 'I']);
            assert_eq!(
                count_word(1, &record.lc_name, &Words::STOCK),
                if vowel { "an" } else { "a" },
                "{}",
                record.lc_name
            );
        }
        for (name, article) in [("egg", "an"), ("orb", "an"), ("urn", "an"), ("box", "a")] {
            assert_eq!(count_word(1, name, &Words::STOCK), article, "{name}");
        }
    }

    #[test]
    fn counts_above_one_are_words_to_three_then_digits_with_the_plural() {
        let records = [named(128, "fuel tank", "fuel tanks", 0)];
        assert!(extras(&records, &[(128, 2)]).ends_with("\r\rTwo fuel tanks.\r\r"));
        assert!(extras(&records, &[(128, 3)]).ends_with("\r\rThree fuel tanks.\r\r"));
        assert!(
            extras(&records, &[(128, 4)]).ends_with("\r\r4 fuel tanks.\r\r"),
            "not capitalised"
        );
    }

    #[test]
    fn extras_go_by_weight_then_id_merged_under_their_leader() {
        let records = [
            named(128, "pod", "pods", 1),
            named(129, "gun", "guns", 5),
            named(130, "pod", "pods", 9),
            named(131, "scoop", "scoops", 5),
            honor(named(132, "badge", "badges", 7)),
        ];
        assert_eq!(
            extras(
                &records,
                &[(128, 1), (129, 1), (130, 2), (131, 1), (132, 1)]
            ),
            "Current extras for your ship:\r\rA gun, a scoop, and three pods.\r\r",
            "130 merges under 128 and takes its weight; the honor is left out"
        );
        assert_eq!(
            extras(&records, &[(129, 1), (131, 2)]),
            "Current extras for your ship:\r\rA gun and two scoops.\r\r"
        );
    }

    #[test]
    fn a_negative_weight_is_dropped_once_a_weight_of_zero_or_more_precedes_it() {
        let records = [
            named(128, "gun", "guns", 5),
            named(129, "low", "lows", -2),
            named(130, "zero", "zeros", 0),
        ];
        assert_eq!(
            extras(&records, &[(128, 1), (129, 1)]),
            "Current extras for your ship:\r\rA gun and .\r\r",
            "the placed gun came first: the low one is never reached"
        );
        assert_eq!(
            extras(&records, &[(129, 1), (130, 1)]),
            "Current extras for your ship:\r\rA zero and a low.\r\r",
            "the low one came first in the zero's pass, so the next weight is its"
        );
        let low_first = [named(128, "low", "lows", -2), named(129, "gun", "guns", 5)];
        assert_eq!(
            extras(&low_first, &[(128, 1), (129, 1)]),
            "Current extras for your ship:\r\rA gun and a low.\r\r",
            "the low one is seen first in the gun's pass"
        );
        let all_low = [
            named(128, "low", "lows", -2),
            named(129, "lower", "lowers", -5),
        ];
        assert_eq!(
            extras(&all_low, &[(128, 1), (129, 1)]),
            "Current extras for your ship:\r\rA low and .\r\r",
            "the placed low one came first: the next weight stays 0"
        );
        let lower_first = [
            named(128, "lower", "lowers", -5),
            named(129, "low", "lows", -2),
        ];
        assert_eq!(
            extras(&lower_first, &[(128, 1), (129, 1)]),
            "Current extras for your ship:\r\rA low and a lower.\r\r"
        );
    }

    #[test]
    fn the_next_weight_is_the_highest_below_among_the_groups_left() {
        let records = [
            named(128, "high", "highs", 9),
            named(129, "low", "lows", 1),
            named(130, "mid", "mids", 3),
        ];
        assert_eq!(
            extras(&records, &[(128, 1), (129, 1), (130, 1)]),
            "Current extras for your ship:\r\rA high, a mid, and a low.\r\r",
            "after the placed high one, 3 over 1"
        );
    }

    #[test]
    fn a_group_placed_in_an_earlier_pass_does_not_count_as_coming_before() {
        // Pass 1 places 128 alone; in pass 2 the negative 129 is the first
        // group not yet placed, so the next weight starts at its -2.
        let records = [
            named(128, "high", "highs", 5),
            named(129, "low", "lows", -2),
            named(130, "mid", "mids", 1),
        ];
        assert_eq!(
            extras(&records, &[(128, 1), (129, 1), (130, 1)]),
            "Current extras for your ship:\r\rA high, a mid, and a low.\r\r"
        );
    }

    #[test]
    fn a_group_whose_leader_is_an_honor_is_left_out_of_the_extras() {
        let records = [
            honor(named(128, "badge", "badges", 0)),
            named(129, "badge", "badges", 0),
        ];
        assert!(
            extras_shown(&records, &owned(&[(129, 1)])),
            "129 is no honor"
        );
        assert_eq!(
            extras(&records, &[(129, 1)]),
            "Current extras for your ship:\r\r.\r\r",
            "its leader 128 is"
        );
    }

    #[test]
    fn the_trade_in_line_follows_the_extras() {
        let records = [named(128, "pod", "pods", 0)];
        let with = |trade_in, millions| {
            extras_text(
                &records,
                &owned(&[(128, 1)]),
                &Words::STOCK,
                trade_in,
                millions,
            )
        };
        assert_eq!(
            with(12_500, RuleSource::Engine),
            "Current extras for your ship:\r\rA pod.\r\rShip trade-in value: 12,500 credits\r"
        );
        assert!(with(1, RuleSource::Engine).ends_with("value: 1 credit\r"));
        assert!(with(2_340_000, RuleSource::Engine).ends_with("value: 2.34M credits\r"));
        assert!(with(2_340_000, RuleSource::Bible).ends_with("value: 2,340,000 credits\r"));
        assert!(with(0, RuleSource::Engine).ends_with("A pod.\r\r"));
        assert!(with(-5, RuleSource::Engine).ends_with("A pod.\r\r"));
    }

    #[test]
    fn extras_show_only_with_an_owned_named_outfit_that_is_no_honor() {
        let records = [
            honor(named(128, "badge", "badges", 0)),
            named(129, "", "", 0),
            named(130, "pod", "pods", 0),
        ];
        assert!(!extras_shown(&records, &owned(&[(128, 1), (129, 1)])));
        assert!(!extras_shown(&records, &owned(&[(130, 0)])));
        assert!(!extras_shown(&records, &owned(&[(131, 1)])), "no record");
        assert!(extras_shown(&records, &owned(&[(130, 1)])));
    }

    fn ranks(line: &str, listed: usize) -> PilotInfoRanks {
        PilotInfoRanks {
            line: line.to_owned(),
            salary: 0,
            listed,
        }
    }

    fn honors(
        ranks: &PilotInfoRanks,
        records: &[OutfitRecord],
        list: &[(i16, u16)],
    ) -> Option<String> {
        let specials = special_groups(records, &owned(list));
        honors_text(ranks, &specials, records, &Words::STOCK)
    }

    #[test]
    fn honors_list_the_ranks_then_the_special_outfits() {
        let records = [
            honor(named(128, "cloaking device", "cloaking devices", 0)),
            honor(named(129, "fuel scoop", "fuel scoops", 0)),
            honor(named(130, "fuel scoop", "fuel scoops", 0)),
            named(131, "pod", "pods", 0),
        ];
        assert_eq!(honors(&ranks("", 0), &records, &[]), None);
        assert_eq!(
            honors(&ranks("Rank", 1), &records, &[]),
            Some("Your ranks and honors:\r\rRank\r".to_owned())
        );
        assert_eq!(
            honors(&ranks("Rank, and ", 1), &records, &[(128, 1)]),
            Some("Your ranks and honors:\r\rRank, and a cloaking device\r".to_owned()),
            "the special not capitalised after a rank"
        );
        assert_eq!(
            honors(
                &ranks("", 0),
                &records,
                &[(128, 1), (129, 1), (130, 1), (131, 1)]
            ),
            Some("Your ranks and honors:\r\rA cloaking device, two fuel scoops\r".to_owned()),
            "merged under 129; the pod is no honor"
        );
        assert_eq!(
            honors(&ranks("", 0), &records, &[(129, 4)]),
            Some("Your ranks and honors:\r\r4 fuel scoops\r".to_owned())
        );
    }

    #[test]
    fn a_special_group_whose_leader_is_no_honor_is_neither_listed_nor_counted() {
        let records = [
            named(128, "badge", "badges", 0),
            honor(named(129, "badge", "badges", 0)),
            honor(named(130, "", "", 0)),
        ];
        let specials = special_groups(&records, &owned(&[(129, 1), (130, 1)]));
        assert_eq!(
            specials,
            [],
            "129's leader 128 lacks 0x2000; 130 has no name"
        );
        assert_eq!(honors(&ranks("", 0), &records, &[(129, 1)]), None);
        assert_eq!(
            honors(&ranks("Rank", 1), &records, &[(129, 1)]),
            Some("Your ranks and honors:\r\rRank\r".to_owned())
        );
    }

    #[test]
    fn honors_show_with_any_active_rank_or_an_owned_named_honor() {
        let records = [
            named(128, "badge", "badges", 0),
            honor(named(129, "badge", "badges", 0)),
            honor(named(130, "", "", 0)),
        ];
        assert!(!honors_shown(false, &records, &owned(&[])));
        assert!(
            honors_shown(true, &records, &owned(&[])),
            "an empty name too"
        );
        assert!(!honors_shown(
            false,
            &records,
            &owned(&[(128, 1), (130, 1)])
        ));
        assert!(!honors_shown(false, &records, &owned(&[(129, 0)])));
        assert!(
            honors_shown(false, &records, &owned(&[(129, 1)])),
            "its own flag, though its leader lacks it"
        );
    }

    #[test]
    fn wages_are_a_hundredth_of_each_cost_truncated_after_each() {
        assert_eq!(daily_income([50_000], 0, 0).expenses, 500);
        assert_eq!(
            daily_income([150, 150], 0, 0).expenses,
            2,
            "1.5 to 1, then 2.5 to 2"
        );
        assert_eq!(daily_income([-150], 0, 0).expenses, -1);
        assert_eq!(daily_income([500, -150], 0, 0).expenses, 3, "5 - 1.5 = 3.5");
        assert_eq!(daily_income(Vec::new(), 0, 0), DailyIncome::default());
    }

    #[test]
    fn income_is_tribute_and_salaries_and_below_zero_it_moves_to_the_expenses() {
        assert_eq!(
            daily_income([10_000], 250, 50),
            DailyIncome {
                income: 300,
                expenses: 100
            }
        );
        assert_eq!(
            daily_income([10_000], 0, -40),
            DailyIncome {
                income: 0,
                expenses: 140
            }
        );
    }

    #[test]
    fn the_income_line_is_expenses_net_income_income_or_none() {
        let line = |income, expenses| income_line(DailyIncome { income, expenses });
        assert_eq!(line(0, 0), None);
        assert_eq!(line(0, -5), None);
        assert_eq!(line(100, 300), Some(IncomeLine::Expenses(200)));
        assert_eq!(line(300, 100), Some(IncomeLine::NetIncome(200)));
        assert_eq!(line(100, 100), Some(IncomeLine::NetIncome(0)));
        assert_eq!(line(300, 0), Some(IncomeLine::Income(300)));
        assert_eq!(line(0, 40), Some(IncomeLine::Expenses(40)));
        assert_eq!(line(10, -5), Some(IncomeLine::NetIncome(15)));
    }

    #[test]
    fn tribute_is_the_stellars_or_a_thousand_a_tech_level() {
        let site = |tribute, tech_level| LandingSite {
            tech_level,
            tribute,
            ..planet(128, 0.0, 0.0)
        };
        assert_eq!(tribute_of(&site(250, 5)), 250);
        assert_eq!(tribute_of(&site(1, 5)), 1);
        assert_eq!(tribute_of(&site(-1, 5)), 5000);
        assert_eq!(tribute_of(&site(0, 5)), 5000);
    }

    #[test]
    fn a_reserve_is_its_share_of_the_capacity() {
        assert_eq!(reserve_shown(50.0, 0.0), None);
        assert_eq!(reserve_shown(50.0, -1.0), None);
        assert_eq!(reserve_shown(-1.0, 200.0), Some(0));
        assert_eq!(reserve_shown(-10.0, 200.0), Some(0));
        assert_eq!(reserve_shown(0.0, 200.0), Some(0));
        assert_eq!(reserve_shown(50.0, 200.0), Some(25));
        assert_eq!(reserve_shown(199.0, 200.0), Some(99));
        assert_eq!(reserve_shown(200.0, 200.0), Some(100));
        assert_eq!(reserve_shown(300.0, 200.0), Some(100));
    }

    #[test]
    fn energy_is_its_share_and_the_jumps_it_reaches() {
        assert_eq!(energy_shown(100.0, 0), None);
        assert_eq!(
            energy_shown(250.0, 300),
            Some(EnergyShown {
                percent: 83,
                reach: Reach::Jumps {
                    jumps: 2,
                    spare: true
                }
            })
        );
        assert_eq!(
            energy_shown(100.0, 300),
            Some(EnergyShown {
                percent: 33,
                reach: Reach::Jumps {
                    jumps: 1,
                    spare: false
                }
            })
        );
        assert_eq!(
            energy_shown(40.0, 300).map(|shown| shown.reach),
            Some(Reach::ManeuveringOnly)
        );
        assert_eq!(
            energy_shown(0.0, 300),
            Some(EnergyShown {
                percent: 0,
                reach: Reach::Nothing
            })
        );
        assert_eq!(
            energy_shown(300.0, 300).map(|shown| shown.percent),
            Some(100)
        );
    }

    fn totals(speed: i64, accel: i64, maneuver: f32) -> HandlingTotals {
        HandlingTotals {
            speed,
            accel,
            maneuver,
        }
    }

    #[test]
    fn the_stat_rows_read_the_handling_totals() {
        assert_eq!(speed_shown(&totals(300, 0, 0.0)), 300);
        assert_eq!(speed_shown(&totals(-5, 0, 0.0)), 0);
        assert_eq!(accel_shown(&totals(0, 300, 0.0)), 150);
        assert_eq!(accel_shown(&totals(0, 301, 0.0)), 150);
        assert_eq!(accel_shown(&totals(0, -300, 0.0)), 0);
        assert_eq!(turn_shown(&totals(0, 0, 10.0)), 30);
        assert_eq!(
            turn_shown(&totals(0, 0, 20.0)),
            60,
            "a TURN_CHANGE of 100 adds 30"
        );
        assert_eq!(turn_shown(&totals(0, 0, 10.5)), 31);
        assert_eq!(turn_shown(&totals(0, 0, -4.0)), 0);
    }

    #[test]
    fn the_handling_totals_add_the_outfits() {
        use crate::fuel::OutfitMod;
        use crate::stats::TURN_CHANGE;
        let fields = crate::handling::ShipFields {
            maneuver: 10,
            ..crate::handling::ShipFields::default()
        };
        let plus = HandlingTotals::new(
            fields,
            &[OutfitMod {
                mod_type: TURN_CHANGE,
                mod_val: 100,
                count: 1,
            }],
        );
        assert_eq!(turn_shown(&plus), 60);
    }

    #[test]
    fn domination_counts_the_counting_stellars() {
        assert_eq!(Domination::of([]), Domination::None);
        assert_eq!(Domination::of([false, false]), Domination::None);
        assert_eq!(Domination::of([true, false]), Domination::Some);
        assert_eq!(Domination::of([false, true]), Domination::Some);
        assert_eq!(Domination::of([true, true]), Domination::All);
        assert_eq!(Domination::of([true]), Domination::All);
    }

    #[test]
    fn the_legal_status_index_steps_at_each_multiple_of_the_tolerance() {
        let index = |record| legal_status_index(record, 10, false, Domination::None);
        assert_eq!(index(0), Some(1));
        assert_eq!(index(-1), Some(2));
        assert_eq!(index(-10), Some(2));
        assert_eq!(index(-11), Some(3));
        assert_eq!(index(-40), Some(3));
        assert_eq!(index(-41), Some(4));
        assert_eq!(index(-160), Some(4));
        assert_eq!(index(-161), Some(5));
        assert_eq!(index(-640), Some(5));
        assert_eq!(index(-641), Some(6));
        assert_eq!(index(-2560), Some(6));
        assert_eq!(index(-2561), Some(7));
        assert_eq!(index(-10_240), Some(7));
        assert_eq!(index(-10_241), Some(8));
        assert_eq!(index(-40_960), Some(8));
        assert_eq!(index(-40_961), Some(9));
        assert_eq!(index(1), Some(10));
        assert_eq!(index(40), Some(10));
        assert_eq!(index(41), Some(11));
        assert_eq!(index(160), Some(11));
        assert_eq!(index(161), Some(12));
        assert_eq!(index(640), Some(12));
        assert_eq!(index(641), Some(13));
        assert_eq!(index(2560), Some(13));
        assert_eq!(index(2561), Some(14));
        assert_eq!(index(10_240), Some(14));
        assert_eq!(index(10_241), Some(15));
    }

    #[test]
    fn a_negative_tolerance_lets_the_later_tests_win() {
        // t = -1: r = -5 passes every negative test (9), then r > 0 and
        // r > -4 fail but r > -16, -64, -256 and -1024 hold (15), as the
        // original's cmovs fall; r = -4 holds from r > -16 too.
        assert_eq!(
            legal_status_index(-5, -1, false, Domination::None),
            Some(15)
        );
        assert_eq!(
            legal_status_index(-3, -1, false, Domination::None),
            Some(15)
        );
        assert_eq!(legal_status_index(5, 0, false, Domination::None), Some(15));
    }

    #[test]
    fn domination_overrides_the_record_and_xenophobia_shows_nothing() {
        assert_eq!(
            legal_status_index(-50, 10, false, Domination::Some),
            Some(16)
        );
        assert_eq!(legal_status_index(50, 10, false, Domination::All), Some(17));
        assert_eq!(legal_status_index(50, 10, true, Domination::None), None);
        assert_eq!(legal_status_index(50, 10, true, Domination::All), None);
    }

    #[test]
    fn a_system_is_inhabited_by_its_first_four_stellars_only() {
        assert!(!system_is_inhabited([]));
        assert!(system_is_inhabited([(false, false)]));
        assert!(!system_is_inhabited([(true, false), (false, true)]));
        assert!(system_is_inhabited([
            (true, false),
            (true, false),
            (true, false),
            (false, false)
        ]));
        assert!(!system_is_inhabited([
            (true, false),
            (true, false),
            (true, false),
            (false, true),
            (false, false)
        ]));
    }
}
