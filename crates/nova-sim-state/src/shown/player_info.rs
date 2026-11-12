//! What the player info screen shows ([`PlayerInfo`]): its words
//! ([`Words`], [`NUMBER_WORDS`]), the daily income line ([`IncomeLine`]),
//! the energy row ([`EnergyShown`], [`Reach`]) and cheat mode's numbers
//! ([`CheatIds`], [`cheat_suffix`]), from `nova_sim_port::player_info`,
//! which re-exports them and keeps the texts and the rows' readings.

use std::borrow::Cow;

/// `STR#` 137 #29-#38 of `Nova-DF.rsrc`: the numbers one to ten in
/// words, which `_StrcatOrdinalNumber` @0x8ab4 writes counts in.
pub const NUMBER_WORDS: [&str; 10] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

/// Upper-cases the ASCII letter at byte `at` of `text`, as `toupper`
/// does; anything else is left as it is. `pub` for
/// `nova_sim_port::player_info`'s texts and `ordinal_number`, which
/// capitalise as [`Words::credits_label`] does.
pub fn capitalise_at(text: &mut String, at: usize) {
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

    /// "ton" for 1, else "tons". `pub` for `nova_sim_port::player_info`'s
    /// cargo text, which writes its tons with it.
    #[must_use]
    pub fn tons_for(&self, n: i64) -> &str {
        if n == 1 { &self.ton } else { &self.tons }
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

/// The numbers cheat mode appends to four General rows
/// (`_PlayerInfoDialogUpdate` @0x52951, @0x52a16, @0x52ae3, @0x52c1d).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheatIds {
    /// The system's `sÿst` ID, after System.
    pub system: i16,
    /// The player's record the Legal Status was read from, after it;
    /// `None` in an uninhabited system, whose "N/A" gets none.
    pub record: Option<i16>,
    /// The kill count the combat rating reads, after Combat Rating.
    pub kills: i64,
    /// The ship's `shïp` ID, after Ship Class.
    pub class: i16,
}

/// The suffix cheat mode appends to a General value: `" ("`, the
/// number, then `")"` (`DrawString` of the Pascal strings @0xd97ab and
/// @0xdaddc around `NumToString`).
#[must_use]
pub fn cheat_suffix(n: impl std::fmt::Display) -> String {
    format!(" ({n})")
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
    /// The combat rating's words, `STR#` 138 by
    /// `nova_sim::rating::string_index` (`_DrawCombatRatingString` @0x57e5, drawn by
    /// `_PlayerInfoDialogUpdate` @0x52ade); empty when the list has no
    /// such entry.
    pub combat_rating: String,
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
    /// The numbers cheat mode appends, always filled.
    pub cheat_ids: CheatIds,
    /// Whether cheat mode is on, so the screen appends [`Self::cheat_ids`].
    /// The session leaves it `false`; flight sets it from its developer
    /// setting.
    pub cheats: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn a_cheat_suffix_is_the_number_in_brackets_after_a_space() {
        assert_eq!(cheat_suffix(128), " (128)");
        assert_eq!(cheat_suffix(-5), " (-5)");
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
}
