//! Rules where the Nova Bible and the original `EV Nova` engine disagree:
//! each such rule can follow either, by the [`Rulebook`].
//!
//! The project's strategy is that wherever the Bible says one thing and
//! the engine does another, the rule is written both ways and the player
//! chooses. A [`Rulebook`] holds a default [`RuleSource`] for every such
//! rule (the engine's, unless chosen otherwise) and an override for any
//! single rule, named by its [`RuleKey`]. A rule asks the rulebook
//! [`Rulebook::source_for`] its key, and the rule's implementation makes
//! the decision. The settings file names the default `"rules"` and the
//! overrides `"rule_overrides"`, by each key's [`RuleKey::key`].
//!
//! The rules here so far:
//!
//! | [`RuleKey`] | settings key | engine | Bible |
//! |---|---|---|---|
//! | [`CrimeGains`](RuleKey::CrimeGains) | `crime_gains` | a crime pleases every government not allied with the victim's ([`NovaLaw`](crate::NovaLaw)) | only the victim's enemies |
//! | [`EmptyBooty`](RuleKey::EmptyBooty) | `empty_booty` | boarding a ship of `Booty` 0 opens the plunder dialog anyway ([`NovaBoarding`](crate::NovaBoarding)) | the player is repelled |
//! | [`CrewlessCapture`](RuleKey::CrewlessCapture) | `crewless_capture` | a player of no crew still has odds of 1 or more ([`NovaBoarding`](crate::NovaBoarding)) | no odds: it cannot capture |
//! | [`PiracyPolice`](RuleKey::PiracyPolice) | `piracy_police` | warships and interceptors answer the player's attack or boarding ([`NovaAi`](crate::NovaAi)) | interceptors only |
//! | [`QuietHails`](RuleKey::QuietHails) | `quiet_hails` | a quiet government's ships (`Flags2` 0x0008) answer Greetings, and Request Assistance and Beg For Mercy do nothing ([`hail::nova`](crate::hail::nova)) | they answer Greetings "No response.", and the middle button works |
//! | [`LongAdvice`](RuleKey::LongAdvice) | `long_advice` | an advice line exactly 42 characters long reads "Nice to meet you." ([`hail::reply`](crate::hail::reply)) | it is shown as written\* |
//! | [`EscortAi`](RuleKey::EscortAi) | `escort_ai` | an escort with no standing command keeps formation and fires its turrets at a threat to the player ([`EscortAi`](crate::EscortAi)) | it flies as its `InherentAI`: a warship or interceptor attacks the threat, a trader keeps formation |
//! | [`EscortOrders`](RuleKey::EscortOrders) | `escort_orders` | entering a system resets every escort's standing order to formation ([`Session`](crate::Session)) | the orders are kept\* |
//! | [`FighterLaunch`](RuleKey::FighterLaunch) | `fighter_launch` | a fighter launched takes its class's standing order and no target, and attacks the player's target on command ([`Session`](crate::Session)) | it attacks the player's target at once\* |
//! | [`FighterRecall`](RuleKey::FighterRecall) | `fighter_recall` | fighters out follow a jump when they hold a jump's fuel, the rest are abandoned, and they stay out while the player is landed ([`Session`](crate::Session)) | every fighter out goes back into its bay on arrival and on landing\* |
//! | [`HireRequire`](RuleKey::HireRequire) | `hire_require` | hiring ignores a ship's `Require`, which hides it only with `Flags3` 0x0200 ([`Session`](crate::Session)) | an unmet `Require` makes a listed ship greyed and refused\* |
//! | [`TakeOffPay`](RuleKey::TakeOffPay) | `take_off_pay` | each take-off pays every hired escort a day's wage ([`Session`](crate::Session)) | wages are paid only for the days that pass, a jump's\* |
//! | [`HireFee`](RuleKey::HireFee) | `hire_fee` | a hire takes trunc(cash − 0.1 × price) off the cash, a credit more than the fee shown when the tenth has a fraction ([`NovaHire`](crate::NovaHire)) | exactly the fee shown\* |
//! | [`EscortWage`](RuleKey::EscortWage) | `escort_wage` | a hired escort is paid the wage its ship type's `Cost` gives now ([`Session`](crate::Session)) | the wage kept on its record when hired\* |
//! | [`PersonOdds`](RuleKey::PersonOdds) | `person_odds` | a person comes on `Rand(7)` 0, then `Rand(1022)` landing on one who may appear ([`NovaPersons`](crate::NovaPersons)) | 5 %, one of those who may appear picked evenly |
//! | [`SystemPersons`](RuleKey::SystemPersons) | `system_persons` | a `sÿst` Person slot brings its person on `Rand(100) + 1` at most its chance ([`NovaPersons`](crate::NovaPersons)) | always |
//! | [`LinkSystSlip`](RuleKey::LinkSystSlip) | `link_syst_slip` | a person's system `LinkSyst` also allows the system of ID `LinkSyst` + 128 ([`NovaPersons`](crate::NovaPersons)) | that system only |
//! | [`ShieldMod`](RuleKey::ShieldMod) | `shield_mod` | a person's `ShieldMod` scales its shield and armour ([`NovaPersons`](crate::NovaPersons)) | its shield only |
//! | [`PersonCoward`](RuleKey::PersonCoward) | `person_coward` | only a warship person of a government with `Flags` 0x0010 runs at `Coward` % ([`NovaAi`](crate::NovaAi)) | any warship or interceptor person |
//! | [`PersonCredits`](RuleKey::PersonCredits) | `person_credits` | a person carries half its `Credits` in thousands, kept in 16 bits, and a draw of as many more ([`NovaBoarding`](crate::NovaBoarding)) | its `Credits`, ±25 % |
//! | [`CommQuote`](RuleKey::CommQuote) | `comm_quote` | a person's comm quote is a friendly person's answer to Greetings ([`Session`](crate::Session)) | it opens the hail, in place of the opening line\* |
//! | [`GrantCount`](RuleKey::GrantCount) | `grant_count` | boarding a person grants trunc((50 + `Rand(51)`) x `GrantCount` / 100) outfits, at least 1 ([`NovaBoarding`](crate::NovaBoarding)) | 1 + `Rand(GrantCount)`\* |
//! | [`GrantMax`](RuleKey::GrantMax) | `grant_max` | a grant may take the player past the outfit's `Max` ([`NovaBoarding`](crate::NovaBoarding)) | it is held to `Max` less the owned\* |
//! | [`PersonJoin`](RuleKey::PersonJoin) | `person_join` | no person offers to join: the original offers its `LinkMission`, which waits for missions ([`JoinFleet`](crate::hail::nova::JoinFleet)) | a person whose record allows it lists Use As Escort and joins the fleet as itself, for good\* |
//! | [`BuyRandom`](RuleKey::BuyRandom) | `buy_random` | an outfit's `BuyRandom` below 1 is never for sale and a ship's below 0 always (`_LoadObjectData` @0x78b8d, @0x7a340; [`Session`](crate::Session)) | an outfit's below 1 or above 100 always, and a ship's 0 or below never† |
//! | [`JunkFlags`](RuleKey::JunkFlags) | `junk_flags` | the free space, measured once, lets tribbles goods overfill the hold; perishable goods decay only while there is space; a good with both flags decays only beside a perishable-only good (`_HandlePlayer` @0x70827-0x7093b; [`Session`](crate::Session)) | tribbles goods grow only into free space, and perishable goods always decay‡ |
//! | [`LauncherSale`](RuleKey::LauncherSale) | `launcher_sale` | a launcher cannot be sold while the rounds held overfill the remaining launchers' `MaxAmmo`, and only when its weapon's `MaxAmmo` is above 0 (`_DoOutfitDialog` @0x5ca75-0x5cbe0; [`Session`](crate::Session)) | a launcher cannot be sold while any of its ammunition is held§ |
//! | [`RaisedMax`](RuleKey::RaisedMax) | `raised_max` | a `ModType` 27 outfit multiplies its target's `Max` by the outfits owned times their mods naming it, at least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), and one of n cannot be sold while the target owned exceeds its raw `Max` x (n - 1) (`_DoOutfitDialog` @0x5c7be-0x5ca6f; [`Session`](crate::Session)) | the multiplier is the items owned with such a mod, at least 1, and a sale is refused only while the target owned exceeds that maximum after it¶ |
//! | [`EventPrice`](RuleKey::EventPrice) | `event_price` | an active event prices its commodity at `BasePrice` + `PriceDelta`, whatever the level, the highest ID winning, and lists a commodity not traded there (`_DoTradeDialog` @0x5dcf8-0x5dd58; [`Session`](crate::Session)) | the level price plus every event's `PriceDelta`\*\* |
//! | [`PurchaseCargo`](RuleKey::PurchaseCargo) | `purchase_cargo` | a ship purchase keeps each commodity's share A/B (the new ship's cargo space over that plus the trader escorts' `Holds`, uncapped) and loses that share of each `jünk`, then trims the commodities to the fleet's holds (`_DestroyPartialFleetCargo` @0xcd32-0xcff6, `_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c; [`Session`](crate::Session)) | it keeps what fits the new ship's own cargo space, goods in order†† |
//! | [`JunkPrice`](RuleKey::JunkPrice) | `junk_price` | a `jünk` price is signed, with no floor: a negative one is listed, bought at 0 tons and sold at a loss that can leave the cash below 0, and a row priced 0 is not listed (`_DoTradeDialog` @0x5dddb, @0x5de2f, @0x5e27d, @0x5e543-0x5e546; `_TradeDialogUpdate` @0x4d2a2-0x4d2ac; [`Session`](crate::Session)) | a `jünk` price is never below 0, and a row priced 0 is listed and bought free, limited by space‡‡ |
//! | [`JunkTrade`](RuleKey::JunkTrade) | `junk_trade` | every listed `jünk` row is bought and sold at its own price, the order naming the row (`_CanBuyGoods` @0xccec, `_CanSellGoods` @0x4a94, `_DrawTradeButtons` @0x2938f/0x2939d, `_TrackTradeButtons` @0x2960d/0x2963e; [`Session`](crate::Session)) | the `SoldAt` row is bought only and the `BoughtAt` row sold only, as the Bible's "`SoldAt` … where the commodity is sold" and "`BoughtAt` … where the commodity is purchased" say |
//! | [`OutfitLimit`](RuleKey::OutfitLimit) | `outfit_limit` | a map (`ModType` 16) or clean-record (`ModType` 21) outfit is refused while the last outfit bought since the outfitter opened was of its kind; any other buy, or opening the outfitter again, lifts it (`_CanBuyOutfitItem` @0x4e8bc-0x4e9c5, `_GrantOutfitItem` @0x44d67-0x44f8b, `_DoOutfitDialog` @0x5bb37-0x5bb44; [`Session`](crate::Session)) | no limit§§ |
//! | [`TradeLot`](RuleKey::TradeLot) | `trade_lot` | a plain Buy at the exchange moves min(trunc(cash / price), 10, free) tons and a plain Sell min(held, 10); Option (Alt here) asks for the most (`_DoTradeDialog` @0x5e268-0x5e278, @0x5e48d-0x5e4fa; [`Session`](crate::Session)) | a plain trade moves 1 ton¶¶ |
//!
//! \* The Bible says nothing of `long_advice`, `escort_orders`,
//! `fighter_launch`, `fighter_recall`, `hire_require`, `take_off_pay`,
//! `hire_fee`, `escort_wage`, `grant_max`, `person_join` or `comm_quote`
//! (only that the quote is "displayed in the communications dialog"), and
//! agrees with the engine on `grant_count`: for them, the reading other
//! than the engine's (`"bible"` in the settings) is the intended
//! behaviour, not the engine's bug or quirk, and not anything the Bible
//! says.
//!
//! † The Bible gives an outfit's `BuyRandom` below 1 or above 100 as 100,
//! and a ship's 0 as never, but is silent on a ship's negative value:
//! its reading here takes that as never too.
//!
//! ‡ The Bible says only that a tribbles good "multiplies like tribbles"
//! and a perishable one "gradually decays away", and gives no rates: both
//! readings take the engine's ton every due frame in flight, and only
//! the conditions differ.
//!
//! § The Bible says nothing of selling a launcher before its ammunition.
//! The other reading is the phase's first wording of the rule (refuse
//! while any of its ammunition is owned), not anything the Bible says.
//!
//! ¶ The Bible gives the multiplier as the number of items, and says the
//! `Max` is unchanged with none. It says nothing of selling. Its reading
//! here refuses a sale only past the maximum it gives after the sale.
//!
//! \*\* The Bible gives `PriceDelta` only as "how much to raise or lower
//! the price", read here as raising the stellar's own price. Under both
//! readings a commodity price of 4 or less is 5, as in the engine
//! (@0x5dcc8-0x5dcce), and the Bible's reading leaves an event on a
//! commodity not traded there unlisted: the Bible is silent on both, so
//! these are the engine's floor and this port's choice.
//!
//! †† The Bible says nothing of cargo on a ship purchase. The other
//! reading is the phase's first wording of the rule (keep the cargo that
//! fits the new ship), not anything the Bible says.
//!
//! ‡‡ The Bible says nothing of a `jünk` of negative or zero price. The
//! other reading is the port's own floor of 0, which it had before the
//! engine's reading was found, not anything the Bible says.
//!
//! §§ The Bible says nothing of a limit on buying a map or clean-record
//! outfit. The other reading is the port's earlier behaviour, with no
//! limit, not anything the Bible says.
//!
//! ¶¶ The Bible says nothing of how many tons a click at the exchange
//! trades. The other reading is the port's earlier behaviour, one ton a
//! click, not anything the Bible says.
//!
//! # Adding a rule
//!
//! 1. Add a variant, its doc comment and its settings key to the
//!    `rule_keys!` table below (and a row to the table above). That is the
//!    only list: [`RuleKey::ALL`], [`RuleKey::key`] and
//!    [`RuleKey::from_key`] come from it, and the settings file reads the
//!    key with no further change.
//! 2. Give the rule's implementation its [`RuleSource`], taken from the
//!    rulebook with [`Rulebook::source_for`] where the rule is built (as
//!    [`NovaLaw::from_rulebook`](crate::NovaLaw::from_rulebook) does), and
//!    make the decision there.
//! 3. Test both readings, [`RuleSource::Engine`] and [`RuleSource::Bible`],
//!    and that the rule follows its rulebook entry; extend this module's
//!    key test to the new key.
//!
//! At the edge of the program, every rule is built from the one rulebook
//! the settings give (see `nova::rulebook`).

/// Which of the two a disputed rule follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuleSource {
    /// What the original `EV Nova` executable does: the default.
    #[default]
    Engine,
    /// What the Nova Bible says.
    Bible,
}

impl RuleSource {
    /// Both, the engine's first.
    pub const ALL: [Self; 2] = [Self::Engine, Self::Bible];

    /// The source's name in the settings: `engine` or `bible`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Bible => "bible",
        }
    }

    /// The source named `name` (see [`RuleSource::name`]), if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|source| source.name() == name)
    }
}

/// Declares [`RuleKey`]: each variant with its doc comment and its
/// settings key, the one list of the disputed rules.
macro_rules! rule_keys {
    ($($(#[$doc:meta])* $variant:ident => $key:literal,)+) => {
        /// A rule where the Bible and the engine disagree (see the module
        /// docs).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum RuleKey {
            $($(#[$doc])* $variant,)+
        }

        impl RuleKey {
            /// Every disputed rule, in the order declared.
            pub const ALL: [Self; [$(Self::$variant),+].len()] = [$(Self::$variant),+];

            /// The rule's key in the settings' `rule_overrides`.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }
        }
    };
}

rule_keys! {
    /// Which governments a crime against a ship improves the player's
    /// record with: by the engine, every government not allied with the
    /// victim's; by the Bible, only its enemies (see
    /// [`legal`](crate::legal)).
    CrimeGains => "crime_gains",
    /// Boarding a ship whose `düde` `Booty` is 0: by the engine, the
    /// plunder dialog opens anyway; by the Bible, the player is repelled
    /// (see [`board`](crate::board)).
    EmptyBooty => "empty_booty",
    /// Capturing with a ship of no crew: by the engine, the odds are held
    /// at 1 or more; by the Bible, it cannot capture (see
    /// [`board`](crate::board)).
    CrewlessCapture => "crewless_capture",
    /// Which NPCs come to the help of a ship the player fires on or
    /// boards: by the engine, warships and interceptors; by the Bible,
    /// interceptors only, the "piracy police" (see
    /// [`react`](crate::ai::react)).
    PiracyPolice => "piracy_police",
    /// What a quiet government's ships (`gövt` `Flags2` 0x0008, or
    /// their ship type's inherent government's) answer when hailed: by
    /// the engine, Greetings as usual, while Request Assistance and Beg
    /// For Mercy do nothing; by the Bible, "No response." to Greetings,
    /// and the middle button as usual (see [`hail::nova`](crate::hail::nova)).
    QuietHails => "quiet_hails",
    /// An advice line exactly 42 characters long: by the engine, it reads
    /// "Nice to meet you.", as the original mistakes its length for a
    /// `*`; otherwise it is shown as written. The Bible is silent here, so
    /// the other reading is the intended behaviour, not the engine's bug
    /// (see [`hail::reply`](crate::hail::reply)).
    LongAdvice => "long_advice",
    /// What the player's escort with no standing command does: by the
    /// engine, it keeps formation and fires its turrets at a ship
    /// threatening the player; by the Bible (`shïp` `InherentAI`), it
    /// flies as its AI type would, a warship or interceptor attacking
    /// such a ship with every weapon (see [`ai::escort`](crate::ai::escort)).
    EscortAi => "escort_ai",
    /// What becomes of the escorts' standing orders when the player
    /// enters a system (a jump's arrival, a take-off, a pilot loaded): by
    /// the engine, every one is reset to formation; otherwise they are
    /// kept. The Bible is silent here, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::with_escort_orders`](crate::Session::with_escort_orders)).
    EscortOrders => "escort_orders",
    /// What a fighter the player launches from a bay does first: by the
    /// engine, it takes the standing order of the first ship of its class
    /// already in the fleet (Return to Hangar excepted) and no target, so
    /// it attacks the player's target once commanded; otherwise it
    /// attacks the player's target at once, as the help page has it
    /// ("choose a target and then launch your fighters"). The Bible is
    /// silent here, so the other reading is the intended behaviour, not
    /// anything the Bible says (see
    /// [`Session::with_fighter_launch`](crate::Session::with_fighter_launch)).
    FighterLaunch => "fighter_launch",
    /// What becomes of the player's fighters out when it leaves the
    /// system: by the engine, those whose ship type holds a jump's fuel
    /// follow it through a jump and the rest are abandoned, and landed
    /// they stay out; otherwise every one goes back into its bay at once
    /// on each arrival and landing, as the original's uncalled
    /// `_InstantFighterRecall` does. The Bible is silent here, so the
    /// other reading is the intended behaviour, not anything the Bible
    /// says (see
    /// [`Session::with_fighter_recall`](crate::Session::with_fighter_recall)).
    FighterRecall => "fighter_recall",
    /// Whether a ship's `Require` gates hiring it in the bar: by the
    /// engine, it does not (`_CalcShipCanBuy`'s hire branch never checks
    /// it), and an unmet `Require` only hides the ship when its `Flags3`
    /// has 0x0200; otherwise an unmet `Require` makes a listed ship
    /// greyed and refused, as it does in the shipyard. The Bible speaks
    /// of `Require` only for purchase, so the other reading is the
    /// intended behaviour, not anything the Bible says (see
    /// [`Session::with_hire_require`](crate::Session::with_hire_require)).
    HireRequire => "hire_require",
    /// Whether leaving the spaceport costs the hired escorts' wages for a
    /// day: by the engine, each take-off pays every hired escort a day's
    /// wage, an escort the player cannot pay defecting
    /// (`_DoEscortLand` @0x40853); otherwise wages are paid only for the
    /// days that pass, which here means a jump's. The Bible is silent
    /// here, so the other reading is the intended behaviour, not anything
    /// the Bible says (see
    /// [`Session::with_take_off_pay`](crate::Session::with_take_off_pay)).
    TakeOffPay => "take_off_pay",
    /// What hiring an escort takes from the cash: by the engine, a tenth
    /// of the hire price worked out in floating point and the cash cut
    /// down to whole credits, which can take one credit more than the
    /// fee shown (`_DoShipyardDialog` @0x5f168-0x5f1d1); otherwise
    /// exactly the fee shown. The Bible gives no fee, so the other
    /// reading is the intended behaviour, not anything the Bible says
    /// (see [`NovaHire`](crate::NovaHire)).
    HireFee => "hire_fee",
    /// Which wage a hired escort is paid each day, and shows when hailed:
    /// by the engine, the wage its ship type's record gives now, worked
    /// out from its `Cost` on every pay day as `_DoEscortPayment` does;
    /// otherwise the wage kept on its record when it was hired, as the
    /// phase has it. The Bible is silent here, so the other reading is
    /// the intended behaviour, not anything the Bible says (see
    /// [`Session::with_escort_wage`](crate::Session::with_escort_wage)).
    EscortWage => "escort_wage",
    /// How often a person (`përs`) takes an ordinary ship's place, and
    /// which: by the engine, a draw of `Rand(7)` of 0, then `Rand(1022)`
    /// landing on a person who may appear (`_SpawnPerson` @0x40c95), so
    /// each person has a fixed 1-in-1022 share; by the Bible ("a 5% chance
    /// that a specific AI-person will also be created"), `Rand(100)`
    /// below 5, then one of the persons who may appear, picked evenly,
    /// taking the ship's place (see [`person`](crate::person)).
    PersonOdds => "person_odds",
    /// Whether a `sÿst` Person slot brings its person: by the engine,
    /// when `Rand(100) + 1` is at most the slot's chance
    /// (`_SetupShipsInSystem` @0x43363-0x43386); by the Bible ("Want to
    /// make a ‘pers’ type ship always appear? Put its ID into one of the
    /// Person fields"), always (see [`person`](crate::person)).
    SystemPersons => "system_persons",
    /// Which systems a system-linked person's `LinkSyst` allows: by the
    /// engine, that system and, by its slip comparing the raw value with
    /// the system's index (@0x409db), the one whose ID is `LinkSyst` +
    /// 128 too; by the Bible, that system only (see
    /// [`person`](crate::person)).
    LinkSystSlip => "link_syst_slip",
    /// What a person's `ShieldMod` scales: by the engine, its shield and
    /// its armour (`_SpawnPerson` @0x40f68-0x40f9e); by the Bible ("How
    /// much to increase/decrease the person's shield capacity"), its
    /// shield only (see [`person`](crate::person)).
    ShieldMod => "shield_mod",
    /// Which persons run below `Coward`% of their shields: by the engine,
    /// only a warship person of a government with `Flags` 0x0010 and no
    /// fleet lead (`_WarshipAI` @0x8be36); by the Bible ("At what percent
    /// of total shield capacity will the person run away from a fight?"),
    /// any warship or interceptor person with no fleet lead (see
    /// [`ai::warship`](crate::ai::warship)).
    PersonCoward => "person_coward",
    /// The credits a boarded person carries: by the engine, half its
    /// `Credits` in thousands (kept in 16 bits), with a draw of as many
    /// more thousands above 2 (`_SetPlunderValues` @0x922f2-0x9238f); by
    /// the Bible ("This many credits, +/- 25%"), its `Credits` times 75 to
    /// 125 % (see [`board`](crate::board)).
    PersonCredits => "person_credits",
    /// When a person's comm quote (`STR#` 7100) is said: by the engine,
    /// as a friendly person's answer to Greetings, in place of its advice
    /// (`_LoadAdvice` @0x91ae7-0x91b55); otherwise, as the phase has it,
    /// in place of the hail's opening line, whatever the person's
    /// attitude. The Bible says only that it is "displayed in the
    /// communications dialog", so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`Session::with_comm_quote`](crate::Session::with_comm_quote)).
    CommQuote => "comm_quote",
    /// How many outfits boarding a person grants: by the engine,
    /// trunc((50 + `Rand(51)`) x `GrantCount` / 100), at least 1
    /// (`_DoPlunderDialog` @0x9319b-0x931d6), between `GrantCount` / 2 and
    /// `GrantCount`, as the Bible says too; otherwise 1 + `Rand(GrantCount)`,
    /// between 1 and `GrantCount` evenly, as the phase has it. The Bible
    /// agrees with the engine, so the other reading is the intended
    /// behaviour, not anything the Bible says (see
    /// [`grant`](crate::grant)).
    GrantCount => "grant_count",
    /// Whether a grant may take the player past the outfit's `Max`: by the
    /// engine, it may, as only an outfit's owned count is tested before
    /// the pick (`_DoPlunderDialog` @0x93119); otherwise the count is held
    /// to `Max` less the owned. The Bible is silent here, so the other
    /// reading is the obvious fix, not anything the Bible says (see
    /// [`grant`](crate::grant)).
    GrantMax => "grant_max",
    /// Whether a person offers to join the player when hailed: by the
    /// engine, none does, as the original brings a person to fly with the
    /// player only through its `LinkMission`, offered in place of the comm
    /// dialog and swapping the person's ship for a mission ship
    /// (`_HandlePlayerCommunication` @0x621a9-0x62509), which waits for
    /// missions; otherwise, as the phase has it, a person whose record
    /// allows it ([`offers_to_join`](crate::person::offers_to_join)) lists
    /// Use As Escort and joins the fleet as itself, for good. The Bible is
    /// silent here, so the other reading is the intended behaviour, not
    /// anything the Bible says (see
    /// [`JoinFleet`](crate::hail::nova::JoinFleet)).
    PersonJoin => "person_join",
    /// How a `BuyRandom` outside 1-100 reads: by the engine, an outfit's
    /// below 1 is never for sale and a ship's below 0 always, as the
    /// original's loader clamps them (`_LoadObjectData` @0x78b8d,
    /// @0x7a340); by the Bible, an outfit's below 1 or above 100 is
    /// always for sale and a ship's 0 or below never. The Bible is silent
    /// on a ship's negative value, so for it "0 means never" is taken to
    /// cover it (see [`outfitter`](crate::outfitter) and
    /// [`shipyard`](crate::shipyard)).
    BuyRandom => "buy_random",
    /// When held `jünk` with the Tribbles (0x0001) and Perishable
    /// (0x0002) `Flags` grow and decay, a ton each every due frame in
    /// flight: by the engine, on the free space measured once, so
    /// tribbles goods can fill the hold past full, perishable goods decay
    /// only while there is space, and a good with both flags decays only
    /// beside a perishable-only good (`_HandlePlayer` @0x70827-0x7093b);
    /// by the Bible, which says only that tribbles goods multiply and
    /// perishable goods "gradually decay away", tribbles goods grow only
    /// into free space and perishable goods always decay. The Bible gives
    /// no rates, so both readings take the engine's ton every due frame
    /// (see [`market`](crate::market)).
    JunkFlags => "junk_flags",
    /// When a launcher outfit (one whose first `ModType` 1 names a
    /// `wëap`) cannot be sold for the ammunition held: by the engine,
    /// only when its weapon's `MaxAmmo` is above 0 and selling it would
    /// leave more rounds than the remaining launchers' `MaxAmmo` holds
    /// (`_DoOutfitDialog` @0x5ca75-0x5cbe0), so in stock data only fighter
    /// bays are ever blocked; by the other reading, whenever any of its
    /// ammunition is held, whatever its `MaxAmmo`. The Bible says nothing
    /// of it, so the other reading is the phase's first wording of the
    /// rule, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)).
    LauncherSale => "launcher_sale",
    /// How an `oütf` of `ModType` 27 raises the `Max` of the outfit its
    /// `ModVal` names, and when one can be sold. By the engine, the
    /// target's `Max` is multiplied by the sum, over the outfits owned, of
    /// the count owned times that outfit's `ModType` 27 mods naming it, at
    /// least 1 (`_HasMaxOfItem` @0x45e9-0x46c3), so an outfit with two
    /// such mods counts twice; selling one of n raisers is refused while
    /// the target owned exceeds its raw `Max` x (n - 1), other raisers
    /// ignored and with no floor (`_DoOutfitDialog` @0x5c7be-0x5ca6f). By
    /// the Bible, the multiplier is the number of owned items with any
    /// such mod, at least 1, and a sale is refused while the target owned
    /// exceeds that maximum after the sale (see
    /// [`outfitter`](crate::outfitter)).
    RaisedMax => "raised_max",
    /// How an active `öops` event prices its commodity at its stellar. By
    /// the engine, the price is the commodity's `BasePrice` plus the
    /// event's `PriceDelta`, whatever the stellar's level; of several
    /// events on one commodity the highest ID wins, and an event lists a
    /// commodity the stellar does not otherwise trade (`_DoTradeDialog`
    /// @0x5dcf8-0x5dd58). By the Bible, every active event's `PriceDelta`
    /// is added to the stellar's own (level) price, several add up, and an
    /// event on a commodity not traded there moves nothing. Under both, a
    /// standard commodity's price of 4 or less is 5 (see
    /// [`market`](crate::market)).
    EventPrice => "event_price",
    /// What cargo a ship purchase keeps. By the engine, as
    /// `_DoShipyardDialog` calls `_DestroyPartialFleetCargo(0)`
    /// (@0x5ef9f, @0xcd32-0xcff6) with the new ship in place: with A the
    /// new ship's cargo space and B that plus the `Holds` of the trader
    /// escorts, uncapped, each commodity becomes trunc(held x A/B) and
    /// each `jünk` loses trunc(held x A/B), so without such escorts every
    /// `jünk` goes; then, when the fleet's holds are below everything
    /// held, each commodity is cut by their ratio in single floats
    /// (`_ResetPlayerPrecalcedValues` @0xc7d8-0xc82c). By the other
    /// reading, the cargo that fits the new ship's own cargo space is
    /// kept, goods in order. The Bible says nothing of it, so the other
    /// reading is the phase's first wording of the rule, not anything the
    /// Bible says (see [`shipyard`](crate::shipyard)).
    PurchaseCargo => "purchase_cargo",
    /// How a `jünk` of negative or zero price is traded. By the engine,
    /// `_DoTradeDialog` stores a `jünk`'s high price (@0x5dddb) and low
    /// price (@0x5de2f) in 16 bits with no floor, and every reader takes
    /// it signed: a negative price is listed at that price
    /// (`_TradeDialogUpdate` @0x4d545-0x4d54b), a buy of it gets 0 tons
    /// (the `jle` @0x5e27d), a sale of it takes tons x price from the
    /// cash, which can go below 0 (@0x5e543-0x5e546), and a row priced 0
    /// is not listed (`_TradeDialogUpdate` @0x4d2a2-0x4d2ac, `_TradeFilter`
    /// @0x4e262), the row each way on its own. By the other reading, the
    /// price is never below 0, and a row priced 0 is listed and bought
    /// free, limited by space. The Bible says nothing of it, so the other
    /// reading is the port's own floor, not anything the Bible says (see
    /// [`market`](crate::market)).
    JunkPrice => "junk_price",
    /// Which ways a listed `jünk` row trades. By the engine, every row
    /// trades both ways at its own price: the trade buttons ask only
    /// `_CanBuyGoods` (@0xccec, cash for a ton and free space) and
    /// `_CanSellGoods` (@0x4a94, tons held), never which list the row came
    /// from (`_DrawTradeButtons` @0x2938f/0x2939d, `_TrackTradeButtons`
    /// @0x2960d/0x2963e), and a buy or sale on the selected row is priced
    /// at that row's price (@0x5e2c4-0x5e2e2, @0x5e528-0x5e546). By the
    /// Bible, whose `SoldAt` is where "the commodity is sold" and
    /// `BoughtAt` where it "is purchased", the `SoldAt` row is bought only
    /// and the `BoughtAt` row sold only (see [`market`](crate::market)).
    JunkTrade => "junk_trade",
    /// Whether the outfitter sells a map or clean-record outfit only once
    /// an opening. By the engine, an outfit with a `ModType` 16 mod, or
    /// failing that a `ModType` 21 mod, is refused while the last outfit
    /// bought since the outfitter opened was of the same kind
    /// (`_CanBuyOutfitItem` @0x4e8bc-0x4e9c5 reads `_boughtMap` and
    /// `_boughtID`, which `_GrantOutfitItem` sets afresh on every buy,
    /// @0x44d67-0x44f8b, and `_DoOutfitDialog` clears on opening,
    /// @0x5bb37-0x5bb44), unless its first mod is `ModType` 3 or it has a
    /// `ModType` 2 mod. By the other reading, there is no limit. The Bible
    /// says nothing of it, so the other reading is the port's behaviour
    /// before the engine's was found, not anything the Bible says (see
    /// [`outfitter`](crate::outfitter)).
    OutfitLimit => "outfit_limit",
    /// How many tons a plain Buy or Sell at the exchange moves. By the
    /// engine, a plain buy moves min(trunc(cash / price), 10, free) tons
    /// (`_DoTradeDialog` @0x5e268-0x5e278) and a plain sale min(held, 10)
    /// (@0x5e48d-0x5e4fa); only Option opens the quantity dialog. By the
    /// other reading, a plain trade moves 1 ton. The Bible says nothing of
    /// it, so the other reading is the port's earlier behaviour, not
    /// anything the Bible says (see [`market`](crate::market)).
    TradeLot => "trade_lot",
}

impl RuleKey {
    /// The rule whose settings key is `key` (see [`RuleKey::key`]), if
    /// any.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.key() == key)
    }

    /// The rule's place in [`RuleKey::ALL`], which lists the variants in
    /// the order declared.
    const fn index(self) -> usize {
        self as usize
    }
}

/// Which source each disputed rule follows: a default for all, and an
/// override for any one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rulebook {
    default: RuleSource,
    overrides: [Option<RuleSource>; RuleKey::ALL.len()],
}

/// Every rule following the engine, with no override. Written out, as
/// the standard library derives `Default` for arrays of at most 32.
impl Default for Rulebook {
    fn default() -> Self {
        Self::new(RuleSource::default())
    }
}

impl Rulebook {
    /// Every rule following `default`.
    #[must_use]
    pub const fn new(default: RuleSource) -> Self {
        Self {
            default,
            overrides: [None; RuleKey::ALL.len()],
        }
    }

    /// This rulebook, with `rule` following `source` whatever the
    /// default.
    #[must_use]
    pub fn with_override(mut self, rule: RuleKey, source: RuleSource) -> Self {
        self.overrides[rule.index()] = Some(source);
        self
    }

    /// The source every rule without an override follows.
    #[must_use]
    pub fn default_source(&self) -> RuleSource {
        self.default
    }

    /// The source `rule` follows: its override, else the default.
    #[must_use]
    pub fn source_for(&self, rule: RuleKey) -> RuleSource {
        self.overrides[rule.index()].unwrap_or(self.default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_follows_the_engine_by_default() {
        assert_eq!(RuleSource::default(), RuleSource::Engine);
        let rulebook = Rulebook::default();
        assert_eq!(rulebook, Rulebook::new(RuleSource::Engine));
        assert_eq!(rulebook.default_source(), RuleSource::Engine);
        for rule in RuleKey::ALL {
            assert_eq!(rulebook.source_for(rule), RuleSource::Engine, "{rule:?}");
        }
    }

    #[test]
    fn the_default_source_sets_every_rule() {
        let rulebook = Rulebook::new(RuleSource::Bible);
        assert_eq!(rulebook.default_source(), RuleSource::Bible);
        for rule in RuleKey::ALL {
            assert_eq!(rulebook.source_for(rule), RuleSource::Bible, "{rule:?}");
        }
    }

    #[test]
    fn an_override_beats_the_default_both_ways() {
        for (default, other) in [
            (RuleSource::Engine, RuleSource::Bible),
            (RuleSource::Bible, RuleSource::Engine),
        ] {
            let rulebook = Rulebook::new(default).with_override(RuleKey::CrimeGains, other);
            assert_eq!(rulebook.source_for(RuleKey::CrimeGains), other);
            assert_eq!(rulebook.default_source(), default, "unchanged");
            let again = rulebook.with_override(RuleKey::CrimeGains, default);
            assert_eq!(
                again.source_for(RuleKey::CrimeGains),
                default,
                "the last wins"
            );
        }
    }

    #[test]
    fn an_override_on_one_rule_leaves_the_others_on_the_default() {
        for rule in RuleKey::ALL {
            let rulebook = Rulebook::default().with_override(rule, RuleSource::Bible);
            for other in RuleKey::ALL {
                let expected = if other == rule {
                    RuleSource::Bible
                } else {
                    RuleSource::Engine
                };
                assert_eq!(rulebook.source_for(other), expected, "{rule:?}: {other:?}");
            }
        }
    }

    #[test]
    fn each_rule_has_a_settings_key_found_by_name() {
        assert_eq!(
            RuleKey::ALL,
            [
                RuleKey::CrimeGains,
                RuleKey::EmptyBooty,
                RuleKey::CrewlessCapture,
                RuleKey::PiracyPolice,
                RuleKey::QuietHails,
                RuleKey::LongAdvice,
                RuleKey::EscortAi,
                RuleKey::EscortOrders,
                RuleKey::FighterLaunch,
                RuleKey::FighterRecall,
                RuleKey::HireRequire,
                RuleKey::TakeOffPay,
                RuleKey::HireFee,
                RuleKey::EscortWage,
                RuleKey::PersonOdds,
                RuleKey::SystemPersons,
                RuleKey::LinkSystSlip,
                RuleKey::ShieldMod,
                RuleKey::PersonCoward,
                RuleKey::PersonCredits,
                RuleKey::CommQuote,
                RuleKey::GrantCount,
                RuleKey::GrantMax,
                RuleKey::PersonJoin,
                RuleKey::BuyRandom,
                RuleKey::JunkFlags,
                RuleKey::LauncherSale,
                RuleKey::RaisedMax,
                RuleKey::EventPrice,
                RuleKey::PurchaseCargo,
                RuleKey::JunkPrice,
                RuleKey::JunkTrade,
                RuleKey::OutfitLimit,
                RuleKey::TradeLot
            ]
        );
        assert_eq!(RuleKey::TradeLot.key(), "trade_lot");
        assert_eq!(RuleKey::from_key("trade_lot"), Some(RuleKey::TradeLot));
        assert_eq!(RuleKey::OutfitLimit.key(), "outfit_limit");
        assert_eq!(
            RuleKey::from_key("outfit_limit"),
            Some(RuleKey::OutfitLimit)
        );
        assert_eq!(RuleKey::JunkTrade.key(), "junk_trade");
        assert_eq!(RuleKey::from_key("junk_trade"), Some(RuleKey::JunkTrade));
        assert_eq!(RuleKey::JunkPrice.key(), "junk_price");
        assert_eq!(RuleKey::from_key("junk_price"), Some(RuleKey::JunkPrice));
        assert_eq!(RuleKey::PurchaseCargo.key(), "purchase_cargo");
        assert_eq!(
            RuleKey::from_key("purchase_cargo"),
            Some(RuleKey::PurchaseCargo)
        );
        assert_eq!(RuleKey::EventPrice.key(), "event_price");
        assert_eq!(RuleKey::from_key("event_price"), Some(RuleKey::EventPrice));
        assert_eq!(RuleKey::RaisedMax.key(), "raised_max");
        assert_eq!(RuleKey::from_key("raised_max"), Some(RuleKey::RaisedMax));
        assert_eq!(RuleKey::LauncherSale.key(), "launcher_sale");
        assert_eq!(
            RuleKey::from_key("launcher_sale"),
            Some(RuleKey::LauncherSale)
        );
        assert_eq!(RuleKey::JunkFlags.key(), "junk_flags");
        assert_eq!(RuleKey::from_key("junk_flags"), Some(RuleKey::JunkFlags));
        assert_eq!(RuleKey::BuyRandom.key(), "buy_random");
        assert_eq!(RuleKey::from_key("buy_random"), Some(RuleKey::BuyRandom));
        assert_eq!(RuleKey::PersonJoin.key(), "person_join");
        assert_eq!(RuleKey::GrantCount.key(), "grant_count");
        assert_eq!(RuleKey::GrantMax.key(), "grant_max");
        assert_eq!(RuleKey::PersonOdds.key(), "person_odds");
        assert_eq!(RuleKey::SystemPersons.key(), "system_persons");
        assert_eq!(RuleKey::LinkSystSlip.key(), "link_syst_slip");
        assert_eq!(RuleKey::ShieldMod.key(), "shield_mod");
        assert_eq!(RuleKey::PersonCoward.key(), "person_coward");
        assert_eq!(RuleKey::PersonCredits.key(), "person_credits");
        assert_eq!(RuleKey::CommQuote.key(), "comm_quote");
        assert_eq!(RuleKey::HireRequire.key(), "hire_require");
        assert_eq!(RuleKey::TakeOffPay.key(), "take_off_pay");
        assert_eq!(RuleKey::HireFee.key(), "hire_fee");
        assert_eq!(RuleKey::EscortWage.key(), "escort_wage");
        assert_eq!(RuleKey::FighterLaunch.key(), "fighter_launch");
        assert_eq!(RuleKey::FighterRecall.key(), "fighter_recall");
        assert_eq!(RuleKey::CrimeGains.key(), "crime_gains");
        assert_eq!(RuleKey::EmptyBooty.key(), "empty_booty");
        assert_eq!(RuleKey::CrewlessCapture.key(), "crewless_capture");
        assert_eq!(RuleKey::PiracyPolice.key(), "piracy_police");
        assert_eq!(RuleKey::QuietHails.key(), "quiet_hails");
        assert_eq!(RuleKey::LongAdvice.key(), "long_advice");
        assert_eq!(RuleKey::EscortAi.key(), "escort_ai");
        assert_eq!(RuleKey::EscortOrders.key(), "escort_orders");
        for rule in RuleKey::ALL {
            assert_eq!(RuleKey::from_key(rule.key()), Some(rule));
        }
        assert_eq!(RuleKey::from_key("Crime_Gains"), None);
        assert_eq!(RuleKey::from_key(""), None);
    }

    #[test]
    fn each_source_has_a_name_found_by_name() {
        assert_eq!(RuleSource::ALL, [RuleSource::Engine, RuleSource::Bible]);
        assert_eq!(RuleSource::Engine.name(), "engine");
        assert_eq!(RuleSource::Bible.name(), "bible");
        for source in RuleSource::ALL {
            assert_eq!(RuleSource::from_name(source.name()), Some(source));
        }
        assert_eq!(RuleSource::from_name("Bible"), None);
        assert_eq!(RuleSource::from_name("both"), None);
    }
}
