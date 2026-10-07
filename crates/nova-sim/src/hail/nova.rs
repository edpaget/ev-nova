//! Nova's hail options, the defaults: [`Greetings`],
//! [`RequestAssistance`], [`BegForMercy`], [`Release`] and [`JoinFleet`].
//! The first three are for any ship but the player's escort, Release for
//! the escort alone, and Use As Escort for a person who may join.
//!
//! The comm dialog's buttons stand in a column (`_DrawCommDialogButtons`
//! @0x28c43 in the `EV Nova` executable): Greetings, then the middle
//! button, then Close Channel. The middle button reads "Beg For Mercy"
//! for a hostile ship and "Request Assistance" otherwise, and is hidden
//! for an untalkative one; so each option's `applies` lets only one of
//! the two show. The labels are `STR#` 150's #22, #23 and #25; "Offer
//! Bribe" (#24) is never shown for a ship, as Beg For Mercy is the bribe.
//! Each option follows the rulebook's
//! [`RuleKey::QuietHails`](crate::RuleKey::QuietHails) entry.
//!
//! **Greetings** (item 3 @0x9666a, key G), always listed. In order:
//!
//! - An untalkative ship, and by the Bible a quiet one: "No response."
//! - A hostile or unfriendly ship: "Stop wasting my time."
//! - A friendly ship gives its advice ([`Reply::Advice`]), as the
//!   conversation picked it (see [`deal`](super::deal)):
//!   - its government's hail (`InfoTypes` 0x8000, @0x91a03-0x91aa4):
//!     `STR#` 7000 + (govt - 128), string r + 1 for a trader and r + 6
//!     otherwise, r the variant. An independent ship reads `STR#` 6999,
//!     which does not exist, so its line is blank;
//!   - specific advice (0x4000): `STR#` 7500 + (`InfoTypes` & 0x0FFF),
//!     string #1, or "Greetings." when that is blank. The original picks
//!     the string with `Rand((adviceRandom mod count) + 1) + 1`, and
//!     `adviceRandom` is 0 until a planet is hailed, so for ships it is
//!     always #1 and nothing is drawn;
//!   - good prices and disasters (0x1000 and 0x2000), placeholders, and
//!     no advice at all: "Greetings.".
//!
//!   The line is then checked as [`reply`](super::reply) says, by the
//!   rulebook's [`RuleKey::LongAdvice`](crate::RuleKey::LongAdvice) entry.
//! - A friendly person with a comm quote ([`Hail::quote`], by the
//!   engine's reading of [`RuleKey::CommQuote`](crate::RuleKey::CommQuote))
//!   says it instead, `STR#` 7100 string `CommQuote`, as written with no
//!   check (`_LoadAdvice` @0x91ae7-0x91b55).
//!
//! **Request Assistance** (item 2, @0x9625d-0x96643, key R), listed for a
//! talkative ship that is not hostile. In order:
//!
//! 1. A quiet ship, by the engine: nothing happens, and the reply stays.
//! 2. An unfriendly ship, a plunderer or a xenophobe: "In your dreams,
//!    pal."
//! 3. A busy one (`_AIIsShipBusy` @0x82fac): "Okay, I'm on my way." when
//!    it is already assisting the player, else "I'm busy.".
//! 4. While any NPC threatens the player, the original asks for battle
//!    help; a placeholder here: "I'd rather not.".
//! 5. A player who needs nothing: "You're not in any trouble.".
//! 6. Free help ("Roadside Assistance"): "Okay, I'm on my way.", and it
//!    helps at once. Otherwise it says its mood ("You're lucky - I'm in
//!    a good mood today.", "I'm in a bad mood today, so it's going to
//!    cost you." or "I'll help you out if you pay me.") and asks its
//!    price: paid, "Okay, I'm on my way." and it helps; declined, "Ha ha
//!    ha. What a comedian."; short, "Yeah, come back when you actually
//!    have some money.".
//!
//! **Release** (`STR#` 150 #32, key R, the escort dialog's), listed for
//! the player's escort alone (`_DoCommDialog` @0x95721): it says "Goodbye,
//! captain." (group 38) and releases the escort ([`Deed::Release`]) once
//! the channel closes (`_releaseCommEscort` @0x96848). The escort's hail
//! opens "What can I do for you?" (group 4). The original gives a hired
//! or captured escort its own dialog (`DLOG` 1022), with Upgrade and
//! Sell; that waits for hiring. A carried fighter out of the player's bay
//! ([`Hail::carried`]) is hailed as an escort is, but never released
//! (Release is for AI type 6 alone, @0x9604c): it lists no option.
//!
//! **Beg For Mercy** (item 2, @0x96107-0x96258, key R), the engine's
//! bribe, listed for a talkative hostile ship. In order:
//!
//! 1. A quiet ship, by the engine: nothing happens.
//! 2. A ship that takes no bribes: "In your dreams, pal.", and it keeps
//!    attacking.
//! 3. It says its mood (as above, or "You'll have to pay me first.") and
//!    asks its price: paid, "A pleasure doing business with you." and it
//!    spares the player; declined, "What? How dare you! Prepare to die!"
//!    and it attacks; short, "Yeah, come back when you actually have some
//!    money." and nothing changes.
//!
//! **Use As Escort** (`STR#` 150 #46, key U), by
//! [`RuleKey::PersonJoin`]'s reading. By the engine it is never listed:
//! the original brings a person to fly with the player only through its
//! `LinkMission`, offered in place of the comm dialog, which waits for
//! missions. By the other reading it is listed for a person who may join
//! ([`Hail::joins`]: its record allows it and the fleet has room), not
//! hostile, and neither the player's escort nor a carried fighter: it
//! says "Okay, I'm on my way." (group 29) and joins the fleet as itself
//! at once ([`Deed::Join`]), at no cost.

use super::reply::{
    self, BAD_MOOD, BUSY, COMEDIAN, GOOD_MOOD, HELP_FOR_PAY, HOW_DARE_YOU, IN_YOUR_DREAMS,
    NO_MONEY, NO_RESPONSE, NOT_IN_TROUBLE, ON_MY_WAY, PAY_ME_FIRST, PLEASURE, RATHER_NOT, RELEASED,
    WASTING_TIME,
};
use super::{Answer, Ask, Attitude, Deed, Hail, HailOption, Mood, Reply};
use crate::catalog::GovtId;
use crate::chance::Chance;
use crate::person::COMM_QUOTES;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

/// Greetings' label, `STR#` 150 #22.
pub const GREETINGS: &str = "Greetings";
/// Request Assistance's label, `STR#` 150 #23.
pub const REQUEST_ASSISTANCE: &str = "Request Assistance";
/// Beg For Mercy's label, `STR#` 150 #25.
pub const BEG_FOR_MERCY: &str = "Beg For Mercy";
/// Greetings' hotkey.
pub const GREETINGS_KEY: char = 'G';
/// The middle button's hotkey.
pub const MIDDLE_KEY: char = 'R';

/// `InfoTypes`: the government's hail.
pub const GOVT_HAIL: u16 = 0x8000;
/// `InfoTypes`: specific advice, its list in the low bits.
pub const SPECIFIC_ADVICE: u16 = 0x4000;
/// The low bits of `InfoTypes` that name the specific advice's list.
pub const ADVICE_LIST_BITS: u16 = 0x0FFF;
/// The governments' hail lists start at `STR#` 7000, for government 128.
pub const GOVT_HAILS: i16 = 7000;
/// The first government's ID.
pub const FIRST_GOVT: i16 = 128;
/// A trader's government hail: string r + 1.
pub const TRADER_HAIL: u16 = 1;
/// Any other ship's government hail: string r + 6.
pub const OTHER_HAIL: u16 = 6;
/// The specific advice lists start at `STR#` 7500.
pub const ADVICE_LISTS: i16 = 7500;

/// The group a ship of `mood` says before its price: a good or a bad
/// mood's, else `neutral`.
fn mood_reply(mood: f32, neutral: u8) -> u8 {
    match Mood::of(mood) {
        Mood::Good => GOOD_MOOD,
        Mood::Bad => BAD_MOOD,
        Mood::Neutral => neutral,
    }
}

/// Whether `hail`'s ship ignores the middle button, as a quiet ship does
/// by the engine.
fn ignores_middle(hail: &Hail, quiet_hails: RuleSource) -> bool {
    quiet_hails == RuleSource::Engine && hail.dispositions.quiet
}

/// Nova's Greetings (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Greetings {
    /// What a quiet ship answers.
    pub quiet_hails: RuleSource,
    /// Whether an advice line of 42 characters is passed over.
    pub long_advice: RuleSource,
}

impl Greetings {
    /// Greetings as `rulebook` chooses: its
    /// [`RuleKey::QuietHails`] and [`RuleKey::LongAdvice`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            quiet_hails: rulebook.source_for(RuleKey::QuietHails),
            long_advice: rulebook.source_for(RuleKey::LongAdvice),
        }
    }

    /// A friendly ship's advice (see the module docs).
    fn advice(self, hail: &Hail) -> Reply {
        let variant = u16::from(hail.variant);
        let npc = hail.npc;
        let (list, index, greet_if_blank) = match hail.advice {
            Some(GOVT_HAIL) => {
                let first = if npc.ai_type.trades() {
                    TRADER_HAIL
                } else {
                    OTHER_HAIL
                };
                (govt_hails(npc.govt), variant + first, false)
            }
            Some(SPECIFIC_ADVICE) => {
                let offset = (npc.info_types & ADVICE_LIST_BITS) as i16;
                (ADVICE_LISTS + offset, 1, true)
            }
            _ => (reply::GREETINGS.0, reply::GREETINGS.1, false),
        };
        Reply::Advice {
            list,
            index,
            greet_if_blank,
            long_advice: self.long_advice,
        }
    }
}

/// The `STR#` of `govt`'s hail lines: 6999 for an independent.
fn govt_hails(govt: Option<GovtId>) -> i16 {
    GOVT_HAILS.saturating_add(govt.map_or(-1, |govt| govt.0.saturating_sub(FIRST_GOVT)))
}

impl HailOption for Greetings {
    fn label(&self) -> String {
        GREETINGS.to_owned()
    }

    fn key(&self) -> Option<char> {
        Some(GREETINGS_KEY)
    }

    fn applies(&self, hail: &Hail) -> bool {
        !hail.escort()
    }

    fn press(&self, hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        let dispositions = hail.dispositions;
        let reply = if dispositions.untalkative
            || (self.quiet_hails == RuleSource::Bible && dispositions.quiet)
        {
            Reply::Comm(NO_RESPONSE)
        } else if hail.attitude != Attitude::Friendly {
            Reply::Comm(WASTING_TIME)
        } else if let Some(index) = hail.quote {
            Reply::Line {
                list: COMM_QUOTES,
                index,
            }
        } else {
            self.advice(hail)
        };
        Answer::say(reply)
    }
}

/// Nova's Request Assistance (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RequestAssistance {
    /// What a quiet ship does.
    pub quiet_hails: RuleSource,
}

impl RequestAssistance {
    /// Request Assistance as `rulebook` chooses: its
    /// [`RuleKey::QuietHails`] entry.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            quiet_hails: rulebook.source_for(RuleKey::QuietHails),
        }
    }
}

impl HailOption for RequestAssistance {
    fn label(&self) -> String {
        REQUEST_ASSISTANCE.to_owned()
    }

    fn key(&self) -> Option<char> {
        Some(MIDDLE_KEY)
    }

    fn applies(&self, hail: &Hail) -> bool {
        hail.attitude != Attitude::Hostile && !hail.dispositions.untalkative && !hail.escort()
    }

    fn press(&self, hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        let dispositions = hail.dispositions;
        if ignores_middle(hail, self.quiet_hails) {
            return Answer::default();
        }
        if hail.attitude != Attitude::Friendly || dispositions.plunderer || dispositions.xenophobe {
            return Answer::say(Reply::Comm(IN_YOUR_DREAMS));
        }
        if hail.busy {
            let group = if hail.assisting_player {
                ON_MY_WAY
            } else {
                BUSY
            };
            return Answer::say(Reply::Comm(group));
        }
        if hail.player_threatened {
            return Answer::say(Reply::Comm(RATHER_NOT));
        }
        let Some(need) = hail.need else {
            return Answer::say(Reply::Comm(NOT_IN_TROUBLE));
        };
        let help = Some(Deed::Help(need));
        if dispositions.free_help {
            return Answer {
                deed: help,
                ..Answer::say(Reply::Comm(ON_MY_WAY))
            };
        }
        Answer {
            ask: Some(Ask {
                paid: (Reply::Comm(ON_MY_WAY), help),
                declined: (Reply::Comm(COMEDIAN), None),
                short: Reply::Comm(NO_MONEY),
            }),
            ..Answer::say(Reply::Comm(mood_reply(hail.mood, HELP_FOR_PAY)))
        }
    }
}

/// Nova's Beg For Mercy, the bribe (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BegForMercy {
    /// What a quiet ship does.
    pub quiet_hails: RuleSource,
}

impl BegForMercy {
    /// Beg For Mercy as `rulebook` chooses: its [`RuleKey::QuietHails`]
    /// entry.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            quiet_hails: rulebook.source_for(RuleKey::QuietHails),
        }
    }
}

impl HailOption for BegForMercy {
    fn label(&self) -> String {
        BEG_FOR_MERCY.to_owned()
    }

    fn key(&self) -> Option<char> {
        Some(MIDDLE_KEY)
    }

    fn applies(&self, hail: &Hail) -> bool {
        hail.attitude == Attitude::Hostile && !hail.dispositions.untalkative && !hail.escort()
    }

    fn press(&self, hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        if ignores_middle(hail, self.quiet_hails) {
            return Answer::default();
        }
        if !hail.dispositions.bribable {
            return Answer::say(Reply::Comm(IN_YOUR_DREAMS));
        }
        Answer {
            ask: Some(Ask {
                paid: (Reply::Comm(PLEASURE), Some(Deed::Spare)),
                declined: (Reply::Comm(HOW_DARE_YOU), Some(Deed::Attack)),
                short: Reply::Comm(NO_MONEY),
            }),
            ..Answer::say(Reply::Comm(mood_reply(hail.mood, PAY_ME_FIRST)))
        }
    }
}

/// Release's label, `STR#` 150 #32.
pub const RELEASE: &str = "Release";
/// Release's hotkey, the escort dialog's.
pub const RELEASE_KEY: char = 'R';

/// Release (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Release;

impl HailOption for Release {
    fn label(&self) -> String {
        RELEASE.to_owned()
    }

    fn key(&self) -> Option<char> {
        Some(RELEASE_KEY)
    }

    fn applies(&self, hail: &Hail) -> bool {
        hail.escort() && !hail.carried()
    }

    fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        Answer {
            deed: Some(Deed::Release),
            ..Answer::say(Reply::Comm(RELEASED))
        }
    }
}

/// Use As Escort's label, `STR#` 150 #46 (the captured-ship dialog's
/// button).
pub const USE_AS_ESCORT: &str = "Use As Escort";
/// Use As Escort's hotkey.
pub const USE_AS_ESCORT_KEY: char = 'U';

/// Use As Escort (see the module docs): a person who may join, under
/// [`RuleKey::PersonJoin`]'s other reading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JoinFleet {
    /// Whether any person offers to join: by the engine, none does.
    pub rule: RuleSource,
}

impl JoinFleet {
    /// Use As Escort as `rulebook` chooses: its [`RuleKey::PersonJoin`]
    /// entry.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            rule: rulebook.source_for(RuleKey::PersonJoin),
        }
    }
}

impl HailOption for JoinFleet {
    fn label(&self) -> String {
        USE_AS_ESCORT.to_owned()
    }

    fn key(&self) -> Option<char> {
        Some(USE_AS_ESCORT_KEY)
    }

    fn applies(&self, hail: &Hail) -> bool {
        self.rule == RuleSource::Bible
            && hail.joins
            && !hail.escort()
            && !hail.carried()
            && hail.attitude != Attitude::Hostile
    }

    fn press(&self, _hail: &Hail, _chance: &mut dyn Chance) -> Answer {
        Answer {
            deed: Some(Deed::Join),
            ..Answer::say(Reply::Comm(ON_MY_WAY))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{hail, ship};
    use super::super::{Dispositions, Help};
    use super::*;
    use crate::ai::Goal;
    use crate::combat::ShipRef;
    use crate::traffic::npc::{AiType, Npc};

    /// A chance that a test never expects drawn on.
    struct NoDraws;

    impl Chance for NoDraws {
        fn fires(&mut self, percent: u8) -> bool {
            panic!("asked whether {percent} % fires")
        }

        fn below(&mut self, n: u32) -> u32 {
            panic!("drew below {n}")
        }
    }

    fn press(option: &dyn HailOption, hail: &Hail) -> Answer {
        option.press(hail, &mut NoDraws)
    }

    fn engine() -> [Box<dyn HailOption>; 3] {
        [
            Box::new(Greetings::default()),
            Box::new(RequestAssistance::default()),
            Box::new(BegForMercy::default()),
        ]
    }

    fn says(group: u8) -> Answer {
        Answer::say(Reply::Comm(group))
    }

    fn advice(list: i16, index: u16, greet_if_blank: bool) -> Answer {
        Answer::say(Reply::Advice {
            list,
            index,
            greet_if_blank,
            long_advice: RuleSource::Engine,
        })
    }

    fn untalkative() -> Dispositions {
        Dispositions {
            untalkative: true,
            ..Dispositions::default()
        }
    }

    fn quiet() -> Dispositions {
        Dispositions {
            quiet: true,
            bribable: true,
            ..Dispositions::default()
        }
    }

    /// A ship of `ai_type` and `govt` with `info_types`.
    fn of(ai_type: AiType, govt: Option<i16>, info_types: u16) -> Npc {
        let mut npc = ship();
        npc.ai_type = ai_type;
        npc.govt = govt.map(GovtId);
        npc.info_types = info_types;
        npc
    }

    /// `npc` as the player's escort.
    fn escorting(mut npc: Npc) -> Npc {
        npc.escort = Some(crate::escort::EscortDuty {
            slot: 2,
            ships: 2,
            spacing: 30.0,
            order: None,
        });
        npc
    }

    #[test]
    fn release_applies_only_to_an_escort_and_the_others_never_to_one() {
        let stranger = ship();
        let escort = escorting(ship());
        let [greetings, request, beg] = engine();
        for attitude in [Attitude::Friendly, Attitude::Unfriendly, Attitude::Hostile] {
            for dispositions in [Dispositions::default(), untalkative()] {
                let of = |npc| Hail {
                    attitude,
                    dispositions,
                    ..hail(npc)
                };
                assert!(Release.applies(&of(&escort)), "{attitude:?}");
                assert!(!Release.applies(&of(&stranger)), "{attitude:?}");
                for option in [&greetings, &request, &beg] {
                    assert!(!option.applies(&of(&escort)), "{}", option.label());
                }
            }
        }
        assert!(hail(&escort).escort());
        assert!(!hail(&stranger).escort());
    }

    #[test]
    fn a_carried_fighter_is_hailed_as_an_escort_but_never_released() {
        let mut fighter = escorting(ship());
        fighter.carrier = Some(crate::bay::Carrier {
            ship: crate::combat::ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        let hailed = hail(&fighter);
        assert!(hailed.escort() && hailed.carried());
        assert!(!Release.applies(&hailed));
        for option in engine() {
            assert!(!option.applies(&hailed), "{}", option.label());
        }
        assert!(!hail(&escorting(ship())).carried());
        assert!(!hail(&ship()).carried());
    }

    #[test]
    fn release_says_goodbye_and_releases_the_escort() {
        let escort = escorting(ship());
        assert_eq!(Release.label(), "Release");
        assert_eq!(Release.key(), Some('R'));
        assert_eq!(
            press(&Release, &hail(&escort)),
            Answer {
                deed: Some(Deed::Release),
                ..says(RELEASED)
            }
        );
    }

    #[test]
    fn the_labels_and_keys_are_the_originals() {
        let [greetings, request, beg] = engine();
        assert_eq!(greetings.label(), "Greetings");
        assert_eq!(request.label(), "Request Assistance");
        assert_eq!(beg.label(), "Beg For Mercy");
        assert_eq!(greetings.key(), Some('G'));
        assert_eq!(request.key(), Some('R'));
        assert_eq!(beg.key(), Some('R'));
    }

    #[test]
    fn greetings_always_applies() {
        let npc = ship();
        for attitude in [Attitude::Friendly, Attitude::Unfriendly, Attitude::Hostile] {
            for dispositions in [Dispositions::default(), untalkative()] {
                let hail = Hail {
                    attitude,
                    dispositions,
                    ..hail(&npc)
                };
                assert!(Greetings::default().applies(&hail));
            }
        }
    }

    #[test]
    fn an_untalkative_ship_greets_with_no_response() {
        let npc = ship();
        for attitude in [Attitude::Friendly, Attitude::Hostile] {
            let hail = Hail {
                attitude,
                dispositions: untalkative(),
                ..hail(&npc)
            };
            assert_eq!(press(&Greetings::default(), &hail), says(NO_RESPONSE));
        }
    }

    #[test]
    fn a_hostile_or_unfriendly_ship_says_stop_wasting_my_time() {
        let npc = ship();
        for attitude in [Attitude::Unfriendly, Attitude::Hostile] {
            let hail = Hail {
                attitude,
                ..hail(&npc)
            };
            assert_eq!(press(&Greetings::default(), &hail), says(WASTING_TIME));
        }
    }

    /// A friendly hail of `npc` in `variant` with `advice`.
    fn advising(npc: &Npc, variant: u8, advice: Option<u16>) -> Hail<'_> {
        Hail {
            variant,
            advice,
            ..hail(npc)
        }
    }

    #[test]
    fn a_friendly_ship_greets_with_its_governments_hail_in_the_variant() {
        let trader = of(AiType::WimpyTrader, Some(130), 0x8000);
        let warship = of(AiType::Warship, Some(130), 0x8000);
        let brave = of(AiType::BraveTrader, Some(130), 0x8000);
        let interceptor = of(AiType::Interceptor, Some(130), 0x8000);
        let greet = |npc, variant| {
            press(
                &Greetings::default(),
                &advising(npc, variant, Some(GOVT_HAIL)),
            )
        };
        assert_eq!(greet(&trader, 2), advice(7002, 3, false));
        assert_eq!(greet(&brave, 2), advice(7002, 3, false));
        assert_eq!(greet(&warship, 2), advice(7002, 8, false));
        assert_eq!(greet(&interceptor, 2), advice(7002, 8, false));
        assert_eq!(greet(&trader, 0), advice(7002, 1, false));
        assert_eq!(greet(&warship, 4), advice(7002, 10, false));
        let fed = of(AiType::Warship, Some(128), 0x8000);
        assert_eq!(greet(&fed, 0), advice(7000, 6, false));
    }

    #[test]
    fn an_independent_ship_reads_the_list_before_the_first_governments() {
        let trader = of(AiType::WimpyTrader, None, 0x8000);
        let warship = of(AiType::Warship, None, 0x8000);
        for variant in 0..5 {
            let greet = |npc| {
                press(
                    &Greetings::default(),
                    &advising(npc, variant, Some(GOVT_HAIL)),
                )
            };
            let r = u16::from(variant);
            assert_eq!(greet(&trader), advice(6999, r + 1, false));
            assert_eq!(greet(&warship), advice(6999, r + 6, false));
        }
    }

    #[test]
    fn specific_advice_is_the_first_string_of_its_list_in_every_variant() {
        let npc = of(AiType::Warship, Some(130), 0x4005);
        for variant in 0..5 {
            let hail = advising(&npc, variant, Some(SPECIFIC_ADVICE));
            assert_eq!(press(&Greetings::default(), &hail), advice(7505, 1, true));
        }
        let far = of(AiType::Warship, Some(130), 0x4FFF);
        let hail = advising(&far, 0, Some(SPECIFIC_ADVICE));
        assert_eq!(press(&Greetings::default(), &hail), advice(11_595, 1, true));
    }

    #[test]
    fn no_advice_and_the_placeholder_kinds_greet_with_greetings() {
        let npc = of(AiType::Warship, Some(130), 0xF000);
        for kind in [None, Some(0x1000), Some(0x2000)] {
            let hail = advising(&npc, 3, kind);
            assert_eq!(
                press(&Greetings::default(), &hail),
                advice(2002, 175, false),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_friendly_person_greets_with_its_comm_quote_as_written() {
        let npc = ship();
        let quoted = Hail {
            quote: Some(24),
            ..advising(&npc, 3, Some(GOVT_HAIL))
        };
        assert_eq!(
            press(&Greetings::default(), &quoted),
            Answer::say(Reply::Line {
                list: 7100,
                index: 24,
            }),
            "in place of its advice"
        );
        for attitude in [Attitude::Unfriendly, Attitude::Hostile] {
            let hail = Hail { attitude, ..quoted };
            assert_eq!(press(&Greetings::default(), &hail), says(WASTING_TIME));
        }
        let hail = Hail {
            dispositions: untalkative(),
            ..quoted
        };
        assert_eq!(press(&Greetings::default(), &hail), says(NO_RESPONSE));
        assert_eq!(COMM_QUOTES, 7100);
    }

    #[test]
    fn greetings_carries_its_long_advice_reading() {
        let npc = of(AiType::Warship, Some(130), 0x8000);
        let greetings = Greetings {
            long_advice: RuleSource::Bible,
            ..Greetings::default()
        };
        assert_eq!(
            press(&greetings, &advising(&npc, 0, Some(GOVT_HAIL))),
            Answer::say(Reply::Advice {
                list: 7002,
                index: 6,
                greet_if_blank: false,
                long_advice: RuleSource::Bible,
            })
        );
    }

    #[test]
    fn a_quiet_ship_greets_as_usual_by_the_engine_and_with_no_response_by_the_bible() {
        let npc = ship();
        let hail = Hail {
            dispositions: quiet(),
            ..hail(&npc)
        };
        assert_eq!(
            press(&Greetings::default(), &hail),
            advice(2002, 175, false)
        );
        let bible = Greetings {
            quiet_hails: RuleSource::Bible,
            ..Greetings::default()
        };
        assert_eq!(press(&bible, &hail), says(NO_RESPONSE));
        let unfriendly = Hail {
            attitude: Attitude::Unfriendly,
            ..hail
        };
        assert_eq!(press(&bible, &unfriendly), says(NO_RESPONSE));
    }

    #[test]
    fn request_assistance_applies_only_to_a_talkative_ship_that_is_not_hostile() {
        let npc = ship();
        let applies = |attitude, dispositions| {
            RequestAssistance::default().applies(&Hail {
                attitude,
                dispositions,
                ..hail(&npc)
            })
        };
        assert!(applies(Attitude::Friendly, Dispositions::default()));
        assert!(applies(Attitude::Unfriendly, Dispositions::default()));
        assert!(!applies(Attitude::Hostile, Dispositions::default()));
        assert!(!applies(Attitude::Friendly, untalkative()));
        assert!(!applies(Attitude::Unfriendly, untalkative()));
    }

    /// A friendly hail of `npc`, the player needing fuel.
    fn needy(npc: &Npc) -> Hail<'_> {
        Hail {
            need: Some(Help::Refuel),
            ..hail(npc)
        }
    }

    #[test]
    fn an_unfriendly_plunderer_or_xenophobe_will_not_help() {
        let npc = ship();
        let request = RequestAssistance::default();
        let unfriendly = Hail {
            attitude: Attitude::Unfriendly,
            ..needy(&npc)
        };
        assert_eq!(press(&request, &unfriendly), says(IN_YOUR_DREAMS));
        for dispositions in [
            Dispositions {
                plunderer: true,
                ..Dispositions::default()
            },
            Dispositions {
                xenophobe: true,
                ..Dispositions::default()
            },
        ] {
            let hail = Hail {
                dispositions,
                ..needy(&npc)
            };
            assert_eq!(press(&request, &hail), says(IN_YOUR_DREAMS));
        }
    }

    #[test]
    fn a_busy_ship_is_busy_unless_it_is_already_on_its_way() {
        let npc = ship();
        let request = RequestAssistance::default();
        let busy = Hail {
            busy: true,
            player_threatened: true,
            ..needy(&npc)
        };
        assert_eq!(press(&request, &busy), says(BUSY));
        let assisting = Hail {
            assisting_player: true,
            ..busy
        };
        assert_eq!(press(&request, &assisting), says(ON_MY_WAY));
    }

    #[test]
    fn while_the_player_is_threatened_it_would_rather_not() {
        let npc = ship();
        let hail = Hail {
            player_threatened: true,
            ..needy(&npc)
        };
        assert_eq!(
            press(&RequestAssistance::default(), &hail),
            says(RATHER_NOT)
        );
    }

    #[test]
    fn a_player_who_needs_nothing_is_not_in_any_trouble() {
        let npc = ship();
        assert_eq!(
            press(&RequestAssistance::default(), &hail(&npc)),
            says(NOT_IN_TROUBLE)
        );
    }

    /// The price help asks, with its paid deed `need`.
    fn help_ask(need: Help) -> Ask {
        Ask {
            paid: (Reply::Comm(ON_MY_WAY), Some(Deed::Help(need))),
            declined: (Reply::Comm(COMEDIAN), None),
            short: Reply::Comm(NO_MONEY),
        }
    }

    #[test]
    fn help_is_offered_for_a_price_in_the_ships_mood() {
        let npc = ship();
        for (mood, group) in [
            (0.79, GOOD_MOOD),
            (0.8, HELP_FOR_PAY),
            (1.0, HELP_FOR_PAY),
            (1.2, BAD_MOOD),
        ] {
            for need in [Help::Refuel, Help::Repair] {
                let hail = Hail {
                    mood,
                    need: Some(need),
                    ..hail(&npc)
                };
                assert_eq!(
                    press(&RequestAssistance::default(), &hail),
                    Answer {
                        reply: Some(Reply::Comm(group)),
                        deed: None,
                        ask: Some(help_ask(need)),
                    },
                    "{mood} {need:?}"
                );
            }
        }
    }

    #[test]
    fn free_help_is_on_its_way_at_once_with_no_price() {
        let npc = ship();
        let hail = Hail {
            dispositions: Dispositions {
                free_help: true,
                ..Dispositions::default()
            },
            need: Some(Help::Repair),
            ..hail(&npc)
        };
        assert_eq!(
            press(&RequestAssistance::default(), &hail),
            Answer {
                reply: Some(Reply::Comm(ON_MY_WAY)),
                deed: Some(Deed::Help(Help::Repair)),
                ask: None,
            }
        );
    }

    #[test]
    fn a_quiet_ship_ignores_the_request_by_the_engine_only() {
        let npc = ship();
        let hail = Hail {
            dispositions: quiet(),
            ..needy(&npc)
        };
        assert_eq!(
            press(&RequestAssistance::default(), &hail),
            Answer::default()
        );
        let bible = RequestAssistance {
            quiet_hails: RuleSource::Bible,
        };
        assert_eq!(
            press(&bible, &hail).ask,
            Some(help_ask(Help::Refuel)),
            "as usual"
        );
    }

    #[test]
    fn beg_for_mercy_applies_only_to_a_talkative_hostile_ship() {
        let npc = ship();
        let applies = |attitude, dispositions| {
            BegForMercy::default().applies(&Hail {
                attitude,
                dispositions,
                ..hail(&npc)
            })
        };
        assert!(applies(Attitude::Hostile, Dispositions::default()));
        assert!(!applies(Attitude::Hostile, untalkative()));
        assert!(!applies(Attitude::Friendly, Dispositions::default()));
        assert!(!applies(Attitude::Unfriendly, Dispositions::default()));
    }

    /// A hostile hail of a ship that takes bribes or not.
    fn begging(npc: &Npc, bribable: bool) -> Hail<'_> {
        Hail {
            attitude: Attitude::Hostile,
            dispositions: Dispositions {
                bribable,
                ..Dispositions::default()
            },
            ..hail(npc)
        }
    }

    #[test]
    fn a_ship_that_takes_no_bribes_says_in_your_dreams() {
        let mut npc = ship();
        npc.goal = Goal::Attack(ShipRef::Player);
        assert_eq!(
            press(&BegForMercy::default(), &begging(&npc, false)),
            says(IN_YOUR_DREAMS)
        );
    }

    #[test]
    fn a_bribable_ship_names_its_price_in_its_mood() {
        let npc = ship();
        let ask = Ask {
            paid: (Reply::Comm(PLEASURE), Some(Deed::Spare)),
            declined: (Reply::Comm(HOW_DARE_YOU), Some(Deed::Attack)),
            short: Reply::Comm(NO_MONEY),
        };
        for (mood, group) in [
            (0.3, GOOD_MOOD),
            (0.79, GOOD_MOOD),
            (0.8, PAY_ME_FIRST),
            (1.19, PAY_ME_FIRST),
            (1.2, BAD_MOOD),
            (1.7, BAD_MOOD),
        ] {
            let hail = Hail {
                mood,
                ..begging(&npc, true)
            };
            assert_eq!(
                press(&BegForMercy::default(), &hail),
                Answer {
                    reply: Some(Reply::Comm(group)),
                    deed: None,
                    ask: Some(ask),
                },
                "{mood}"
            );
        }
    }

    #[test]
    fn a_quiet_ship_ignores_begging_by_the_engine_only() {
        let npc = ship();
        let hail = Hail {
            dispositions: quiet(),
            ..begging(&npc, true)
        };
        assert_eq!(press(&BegForMercy::default(), &hail), Answer::default());
        let bible = BegForMercy {
            quiet_hails: RuleSource::Bible,
        };
        assert!(press(&bible, &hail).ask.is_some(), "as usual");
    }

    // Joining the fleet.

    /// A friendly hail of `npc`, a person who may join.
    fn joining(npc: &Npc) -> Hail<'_> {
        Hail {
            joins: true,
            ..hail(npc)
        }
    }

    const BY_THE_PHASE: JoinFleet = JoinFleet {
        rule: RuleSource::Bible,
    };

    #[test]
    fn by_the_engine_no_one_offers_to_join() {
        let npc = ship();
        assert!(!JoinFleet::default().applies(&joining(&npc)));
        assert_eq!(JoinFleet::default().rule, RuleSource::Engine);
    }

    #[test]
    fn by_the_other_reading_a_person_who_may_join_offers_to() {
        let npc = ship();
        assert!(BY_THE_PHASE.applies(&joining(&npc)));
        for attitude in [Attitude::Friendly, Attitude::Unfriendly] {
            let hail = Hail {
                attitude,
                ..joining(&npc)
            };
            assert!(BY_THE_PHASE.applies(&hail), "{attitude:?}");
        }
        assert_eq!(BY_THE_PHASE.label(), "Use As Escort");
        assert_eq!(BY_THE_PHASE.key(), Some('U'));
    }

    #[test]
    fn no_escort_fighter_hostile_ship_or_ship_that_may_not_join_offers_to() {
        let stranger = ship();
        assert!(!BY_THE_PHASE.applies(&hail(&stranger)), "may not join");
        let hostile = Hail {
            attitude: Attitude::Hostile,
            ..joining(&stranger)
        };
        assert!(!BY_THE_PHASE.applies(&hostile));
        let escort = escorting(ship());
        assert!(!BY_THE_PHASE.applies(&joining(&escort)));
        let mut fighter = ship();
        fighter.carrier = Some(crate::bay::Carrier {
            ship: crate::combat::ShipRef::Player,
            window: 100.0,
            reach: 40.0,
        });
        assert!(!BY_THE_PHASE.applies(&joining(&fighter)));
    }

    #[test]
    fn use_as_escort_says_it_is_on_its_way_and_joins() {
        let npc = ship();
        assert_eq!(
            press(&BY_THE_PHASE, &joining(&npc)),
            Answer {
                deed: Some(Deed::Join),
                ..says(ON_MY_WAY)
            },
            "drawing nothing"
        );
    }

    #[test]
    fn join_fleet_follows_its_rulebook_entry() {
        assert_eq!(
            JoinFleet::from_rulebook(&Rulebook::default()),
            JoinFleet::default()
        );
        assert_eq!(
            JoinFleet::from_rulebook(
                &Rulebook::default().with_override(RuleKey::PersonJoin, RuleSource::Bible)
            ),
            BY_THE_PHASE
        );
    }
}
