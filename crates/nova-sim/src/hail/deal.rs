//! A hail's [`Conversation`]: what it rolls once when the ship is hailed,
//! and haggling over a price.
//!
//! **Opening** ([`Conversation::open`], `_DoCommDialog` @0x956d5 in the
//! `EV Nova` executable). A hail rolls these once, in this order, and
//! nothing is rolled again while the channel stays open:
//!
//! 1. **The variant** r, a draw of [`VARIANTS`] (`responseRandom`
//!    @0x95880): every ship comm string of the conversation is said in
//!    it, and so is Greetings' advice.
//! 2. **The mood** (`bribeFlux`, an `f32`): (a draw of [`MOOD_SPREAD`] +
//!    [`MOOD_BASE`]) x [`MOOD_SCALE`] (@0xdd098), then less
//!    [`MOOD_SHIFT`] (@0xdd694) when a draw of [`MOOD_SHIFT_ODDS`] is 0,
//!    or else more when a second such draw is 0. It reads as [`Mood`]: a
//!    good one below [`GOOD_MOOD_BELOW`] (@0xdd6f8) and a bad one above
//!    [`BAD_MOOD_ABOVE`] (@0xddad0), compared as an `f64`, so a mood of
//!    1.20 (1.2000000477 as an `f32`) is a bad one.
//! 3. **The price** (`initialHaggleVal`), from the player's cash C: n =
//!    C x [`PRICE_STEPS_PER_CREDIT`] (@0xddac8), at least 1, then
//!    (a draw of n x [`PRICE_STEP`] + [`PRICE_BASE`]) x the mood. A
//!    greedy government's ships (`Flags` 0x8000) then draw again, of
//!    C x [`GREEDY_STEPS_PER_CREDIT`] (@0xdda58), at least 1, on
//!    [`GREEDY_PRICE_BASE`]. The price is at most C x [`CASH_SHARE`]
//!    (@0xdd0c8), then whole thousands, then at most [`MAX_PRICE`] and at
//!    least [`MIN_PRICE`]. Each draw's n is at most [`MAX_STEPS`], as the
//!    original keeps it in 16 bits.
//! 4. **The advice** (`_LoadAdvice` @0x9133c, called once): which of the
//!    ship's `düde` `InfoTypes` bits ([`ADVICE_BITS`]) Greetings' advice
//!    follows. The original draws 4 until it hits a bit that is set; here
//!    one draw picks among those set, in that order, and only when there
//!    are two or more. With none, there is no advice.
//!
//! **Haggling** (`_DoShipCommPayment` @0x920f3, `_DoHaggleDialog`
//! @0x91e67). Each time the ship asks a price ([`Conversation::ask`]),
//! the haggle roll is drawn: it is won when a draw of [`HAGGLE_ROLL`] is
//! at most [`HAGGLE_WINS_AT_MOST`]. Then ([`Conversation::haggle`]):
//!
//! - Lower Price with the roll won and not yet used lowers the price
//!   asked to trunc(trunc(price x [`HAGGLE_SHARE`]) / 100) x 100 (0.75
//!   @0xdd1f8), and the haggling goes on.
//! - Any other choice ends it: short when the player's cash is below the
//!   price asked; otherwise Accept pays it and Lower Price declines, which
//!   raises the conversation's price by [`DECLINE_RAISE`], so asking
//!   again costs more.

use crate::chance::Chance;

/// How many variants a conversation draws from.
pub const VARIANTS: u32 = 5;
/// The mood's spread, drawn.
pub const MOOD_SPREAD: u32 = 41;
/// What the mood's draw is added to.
pub const MOOD_BASE: u32 = 80;
/// What the mood's sum is scaled by.
pub const MOOD_SCALE: f64 = 0.01;
/// The odds, one in so many, of each shift to the mood.
pub const MOOD_SHIFT_ODDS: u32 = 5;
/// How far a shift moves the mood.
pub const MOOD_SHIFT: f32 = 0.5;
/// A mood below this is a good one.
pub const GOOD_MOOD_BELOW: f64 = 0.8;
/// A mood above this is a bad one.
pub const BAD_MOOD_ABOVE: f64 = 1.2;
/// The price's draws per credit the player has.
pub const PRICE_STEPS_PER_CREDIT: f64 = 5e-7;
/// A greedy government's price's draws per credit.
pub const GREEDY_STEPS_PER_CREDIT: f64 = 0.0001;
/// The most a draw of the price is of.
pub const MAX_STEPS: u32 = 32_767;
/// What each step drawn adds to the price.
pub const PRICE_STEP: f64 = 1000.0;
/// The price before the steps.
pub const PRICE_BASE: f64 = 3000.0;
/// A greedy government's price before the steps.
pub const GREEDY_PRICE_BASE: f64 = 10_000.0;
/// The most of the player's cash a price may be.
pub const CASH_SHARE: f64 = 0.333;
/// What a price is rounded down to a multiple of.
pub const PRICE_ROUNDING: i64 = 1000;
/// The highest price.
pub const MAX_PRICE: i64 = 20_000;
/// The lowest price.
pub const MIN_PRICE: i64 = 1000;
/// The `InfoTypes` bits Greetings' advice can follow, in the order a
/// draw picks among them: good prices, disasters, specific advice and
/// the government's hail.
pub const ADVICE_BITS: [u16; 4] = [0x1000, 0x2000, 0x4000, 0x8000];
/// The haggle roll's draw.
pub const HAGGLE_ROLL: u32 = 100;
/// The haggle roll is won at or below this.
pub const HAGGLE_WINS_AT_MOST: u32 = 35;
/// What a won haggle keeps of the price asked.
pub const HAGGLE_SHARE: f64 = 0.75;
/// What a haggled price is rounded down to a multiple of.
pub const HAGGLE_ROUNDING: i64 = 100;
/// What declining raises the conversation's price by.
pub const DECLINE_RAISE: i64 = 1000;

/// How a ship's mood reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mood {
    /// Below [`GOOD_MOOD_BELOW`].
    Good,
    /// Neither.
    Neutral,
    /// Above [`BAD_MOOD_ABOVE`].
    Bad,
}

impl Mood {
    /// How `mood` reads, compared as an `f64` (see the module docs).
    #[must_use]
    pub fn of(mood: f32) -> Self {
        let mood = f64::from(mood);
        if mood < GOOD_MOOD_BELOW {
            Self::Good
        } else if mood > BAD_MOOD_ABOVE {
            Self::Bad
        } else {
            Self::Neutral
        }
    }
}

/// A choice in the haggle dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Haggle {
    /// "Accept Price".
    Accept,
    /// "Lower Price".
    LowerPrice,
}

/// What a choice in the haggle dialog came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settled {
    /// The price asked was lowered, and the haggling goes on.
    Lowered,
    /// The player paid this.
    Paid(i64),
    /// The player declined the price.
    Declined,
    /// The player's cash is short of the price.
    Short,
}

/// What a hail rolled when it opened, and the haggling under way (see
/// the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conversation {
    /// The variant every reply is said in, below [`VARIANTS`].
    pub variant: u8,
    /// The ship's mood.
    pub mood: f32,
    /// The price it asks, raised each time the player declines.
    pub price: i64,
    /// The `InfoTypes` bit Greetings' advice follows, if any.
    pub advice: Option<u16>,
    /// The price asked while haggling, if the ship has asked one.
    asking: Option<i64>,
    /// The haggle roll drawn when the price was asked.
    roll: u32,
    /// Whether the haggle roll has lowered the price already.
    used: bool,
}

impl Conversation {
    /// A hail's conversation, rolled on `chance` in the original's order
    /// from the player's `cash`, whether the ship's government is
    /// `greedy`, and its `info_types` (see the module docs).
    pub fn open(cash: i64, greedy: bool, info_types: u16, chance: &mut dyn Chance) -> Self {
        let variant = chance.below(VARIANTS) as u8;
        let mood = mood(chance);
        let price = price(cash, greedy, mood, chance);
        let advice = advice(info_types, chance);
        Self {
            variant,
            mood,
            price,
            advice,
            asking: None,
            roll: 0,
            used: false,
        }
    }

    /// The ship asks its price: the haggle roll is drawn on `chance`, once,
    /// and the haggling starts afresh at the conversation's price.
    pub fn ask(&mut self, chance: &mut dyn Chance) {
        self.roll = chance.below(HAGGLE_ROLL);
        self.used = false;
        self.asking = Some(self.price);
    }

    /// The price asked while haggling, if any.
    #[must_use]
    pub fn asking(&self) -> Option<i64> {
        self.asking
    }

    /// Makes `choice` in the haggle dialog, with the player's `cash`, and
    /// gives what it came to (see the module docs); `None`, and nothing
    /// changes, when no price is asked.
    pub fn haggle(&mut self, choice: Haggle, cash: i64) -> Option<Settled> {
        let asking = self.asking?;
        if choice == Haggle::LowerPrice && self.roll <= HAGGLE_WINS_AT_MOST && !self.used {
            self.used = true;
            self.asking = Some(lowered(asking));
            return Some(Settled::Lowered);
        }
        self.asking = None;
        if cash < asking {
            return Some(Settled::Short);
        }
        Some(match choice {
            Haggle::Accept => Settled::Paid(asking),
            Haggle::LowerPrice => {
                self.price += DECLINE_RAISE;
                Settled::Declined
            }
        })
    }
}

/// The mood, drawn on `chance` (see the module docs).
fn mood(chance: &mut dyn Chance) -> f32 {
    let base = (f64::from(chance.below(MOOD_SPREAD) + MOOD_BASE) * MOOD_SCALE) as f32;
    let lower = chance.below(MOOD_SHIFT_ODDS) == 0;
    if lower {
        return base - MOOD_SHIFT;
    }
    let raise = chance.below(MOOD_SHIFT_ODDS) == 0;
    if raise { base + MOOD_SHIFT } else { base }
}

/// How many steps a price with `cash` at `per_credit` draws from: at
/// least 1, at most [`MAX_STEPS`].
fn steps(cash: i64, per_credit: f64) -> u32 {
    ((cash as f64 * per_credit) as u32).clamp(1, MAX_STEPS)
}

/// The price, drawn on `chance` (see the module docs).
fn price(cash: i64, greedy: bool, mood: f32, chance: &mut dyn Chance) -> i64 {
    let mood = f64::from(mood);
    let drawn = |base: f64, per_credit: f64, chance: &mut dyn Chance| {
        let step = f64::from(chance.below(steps(cash, per_credit)));
        step.mul_add(PRICE_STEP, base) * mood
    };
    let mut price = drawn(PRICE_BASE, PRICE_STEPS_PER_CREDIT, chance) as i64;
    if greedy {
        price = drawn(GREEDY_PRICE_BASE, GREEDY_STEPS_PER_CREDIT, chance) as i64;
    }
    let price = price.min((cash as f64 * CASH_SHARE) as i64);
    let price = price / PRICE_ROUNDING * PRICE_ROUNDING;
    price.clamp(MIN_PRICE, MAX_PRICE)
}

/// The `InfoTypes` bit Greetings' advice follows, drawn on `chance`
/// when there are two or more (see the module docs).
fn advice(info_types: u16, chance: &mut dyn Chance) -> Option<u16> {
    let set: Vec<u16> = ADVICE_BITS
        .into_iter()
        .filter(|&bit| info_types & bit != 0)
        .collect();
    match set.len() {
        0 => None,
        1 => set.first().copied(),
        count => set.get(chance.below(count as u32) as usize).copied(),
    }
}

/// `price` lowered by a won haggle, in the original's arithmetic.
fn lowered(price: i64) -> i64 {
    let kept = (price as f64 * HAGGLE_SHARE) as i64;
    (kept as f64 / HAGGLE_ROUNDING as f64) as i64 * HAGGLE_ROUNDING
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::testkit::Draws;

    /// The draws of a conversation's opening: variant 0, a mood of 1.0
    /// with no shift, and `price` (each of the price's draws), and no
    /// more.
    fn opening(price: &[u32]) -> Vec<u32> {
        let mut draws = vec![0, 20, 1, 1];
        draws.extend(price);
        draws
    }

    fn open(cash: i64, greedy: bool, info_types: u16, draws: &[u32]) -> (Conversation, Draws) {
        let mut chance = Draws::of(draws);
        (
            Conversation::open(cash, greedy, info_types, &mut chance),
            chance,
        )
    }

    #[test]
    fn the_variant_is_the_first_draw_of_five() {
        for variant in [0, 3, 4] {
            let (talk, chance) = open(100_000, false, 0, &[variant, 20, 1, 1, 0]);
            assert_eq!(talk.variant, variant as u8);
            assert_eq!(chance.asked[0], 5);
        }
    }

    #[test]
    fn the_mood_is_drawn_around_one_and_may_shift_by_a_half() {
        let (talk, chance) = open(100_000, false, 0, &[0, 20, 1, 1, 0]);
        assert_eq!(talk.mood, 1.0);
        assert_eq!(chance.asked, [5, 41, 5, 5, 1], "both shifts drawn");
        let (talk, chance) = open(100_000, false, 0, &[0, 20, 0, 0]);
        assert_eq!(talk.mood, 0.5, "the first shift lowers it");
        assert_eq!(chance.asked, [5, 41, 5, 1], "no second shift drawn");
        let (talk, chance) = open(100_000, false, 0, &[0, 20, 1, 0, 0]);
        assert_eq!(talk.mood, 1.5, "the second raises it");
        assert_eq!(chance.asked, [5, 41, 5, 5, 1]);
        let (talk, _) = open(100_000, false, 0, &[0, 0, 1, 1, 0]);
        assert_eq!(talk.mood, 0.8_f32);
        let (talk, _) = open(100_000, false, 0, &[0, 40, 1, 1, 0]);
        assert_eq!(talk.mood, 1.2_f32);
    }

    #[test]
    fn a_mood_reads_good_below_point_eight_and_bad_above_one_point_two() {
        assert_eq!(Mood::of(0.8), Mood::Neutral, "base 0, as an f32");
        assert_eq!(Mood::of(1.2), Mood::Bad, "base 40: 1.2000000477");
        assert_eq!(Mood::of(1.19), Mood::Neutral);
        assert_eq!(Mood::of(1.0), Mood::Neutral);
        assert_eq!(Mood::of(0.79), Mood::Good);
        assert_eq!(Mood::of(0.3), Mood::Good);
        assert_eq!(Mood::of(1.7), Mood::Bad);
    }

    #[test]
    fn the_price_is_three_thousand_at_an_even_mood_with_little_cash() {
        let (talk, chance) = open(100_000, false, 0, &opening(&[0]));
        assert_eq!(talk.price, 3000);
        assert_eq!(chance.asked, [5, 41, 5, 5, 1], "n is at least 1");
    }

    #[test]
    fn a_good_mood_lowers_the_price_to_no_less_than_a_thousand() {
        let (talk, _) = open(100_000, false, 0, &[0, 20, 0, 0]);
        assert_eq!(talk.mood, 0.5);
        assert_eq!(talk.price, 1000, "1500 to whole thousands");
        let (talk, _) = open(2000, false, 0, &opening(&[0]));
        assert_eq!(talk.price, 1000, "a third of 2000 is none, then the floor");
    }

    #[test]
    fn more_cash_draws_more_steps() {
        let (talk, chance) = open(10_000_000, false, 0, &opening(&[4]));
        assert_eq!(talk.price, 7000);
        assert_eq!(chance.asked, [5, 41, 5, 5, 5], "ten million x 5e-7");
        let (_, chance) = open(i64::from(i32::MAX) * 100, false, 0, &opening(&[0]));
        assert_eq!(chance.asked[4], 32_767, "16 bits at most");
    }

    #[test]
    fn a_greedy_government_draws_again_on_ten_thousand() {
        let (talk, chance) = open(100_000, true, 0, &opening(&[0, 9]));
        assert_eq!(talk.price, 19_000);
        assert_eq!(chance.asked, [5, 41, 5, 5, 1, 10], "the plain draw first");
        let (talk, _) = open(30_000, true, 0, &opening(&[0, 2]));
        assert_eq!(talk.price, 9000, "12000 down to 9990, to thousands");
        let (talk, _) = open(1_000_000, true, 0, &opening(&[0, 50]));
        assert_eq!(talk.price, 20_000, "the most");
    }

    #[test]
    fn a_price_is_scaled_by_the_mood_before_it_is_rounded() {
        let (talk, _) = open(100_000, true, 0, &[0, 20, 1, 0, 0, 3]);
        assert_eq!(talk.mood, 1.5);
        assert_eq!(talk.price, 19_000, "13000 x 1.5 = 19500");
    }

    #[test]
    fn no_advice_bits_is_no_advice_and_no_draw() {
        let (talk, chance) = open(100_000, false, 0x0005, &opening(&[0]));
        assert_eq!(talk.advice, None);
        assert_eq!(chance.asked.len(), 5);
    }

    #[test]
    fn one_advice_bit_is_the_advice_with_no_draw() {
        let (talk, chance) = open(100_000, false, 0x4005, &opening(&[0]));
        assert_eq!(talk.advice, Some(0x4000));
        assert_eq!(chance.asked.len(), 5);
    }

    #[test]
    fn two_or_more_advice_bits_are_drawn_among_last() {
        for (draw, advice) in [(0, 0x4000), (1, 0x8000)] {
            let (talk, chance) = open(100_000, false, 0xC005, &opening(&[0, draw]));
            assert_eq!(talk.advice, Some(advice));
            assert_eq!(chance.asked, [5, 41, 5, 5, 1, 2], "after the price");
        }
        for (draw, advice) in [(0, 0x1000), (1, 0x2000), (2, 0x4000), (3, 0x8000)] {
            let (talk, chance) = open(100_000, false, 0xF000, &opening(&[0, draw]));
            assert_eq!(talk.advice, Some(advice));
            assert_eq!(chance.asked.last(), Some(&4));
        }
    }

    /// A conversation asking 3000, its haggle roll `roll`.
    fn asking(roll: u32) -> (Conversation, Draws) {
        let (mut talk, _) = open(100_000, false, 0, &opening(&[0]));
        let mut chance = Draws::of(&[roll]);
        talk.ask(&mut chance);
        (talk, chance)
    }

    #[test]
    fn asking_the_price_draws_the_haggle_roll_once() {
        let (talk, chance) = asking(35);
        assert_eq!(chance.asked, [100], "one draw of 100");
        assert_eq!(talk.asking(), Some(3000));
        let (talk, _) = open(100_000, false, 0, &opening(&[0]));
        assert_eq!(talk.asking(), None, "nothing asked on opening");
    }

    #[test]
    fn a_won_roll_lowers_the_price_once_and_a_second_lower_declines() {
        let (mut talk, _) = asking(35);
        assert_eq!(
            talk.haggle(Haggle::LowerPrice, 100_000),
            Some(Settled::Lowered)
        );
        assert_eq!(talk.asking(), Some(2200), "2250 to hundreds");
        assert_eq!(
            talk.haggle(Haggle::LowerPrice, 100_000),
            Some(Settled::Declined)
        );
        assert_eq!(talk.asking(), None);
        assert_eq!(talk.price, 4000, "the price asked before haggling, raised");
    }

    #[test]
    fn a_lost_roll_declines() {
        let (mut talk, _) = asking(36);
        assert_eq!(
            talk.haggle(Haggle::LowerPrice, 100_000),
            Some(Settled::Declined)
        );
        assert_eq!(talk.price, 4000);
        assert_eq!(talk.asking(), None);
    }

    #[test]
    fn asking_again_rolls_afresh_at_the_raised_price() {
        let (mut talk, _) = asking(35);
        talk.haggle(Haggle::LowerPrice, 100_000);
        talk.haggle(Haggle::LowerPrice, 100_000);
        let mut chance = Draws::of(&[0]);
        talk.ask(&mut chance);
        assert_eq!(talk.asking(), Some(4000));
        assert_eq!(
            talk.haggle(Haggle::LowerPrice, 100_000),
            Some(Settled::Lowered),
            "the roll is fresh and unused"
        );
        assert_eq!(talk.asking(), Some(3000));
    }

    #[test]
    fn accepting_pays_the_price_asked_when_the_cash_covers_it() {
        let (mut talk, _) = asking(99);
        assert_eq!(talk.haggle(Haggle::Accept, 3000), Some(Settled::Paid(3000)));
        assert_eq!(talk.price, 3000, "unchanged");
        let (mut talk, _) = asking(99);
        assert_eq!(talk.haggle(Haggle::Accept, 2999), Some(Settled::Short));
        assert_eq!(talk.asking(), None);
        let (mut talk, _) = asking(0);
        talk.haggle(Haggle::LowerPrice, 3000);
        assert_eq!(talk.haggle(Haggle::Accept, 2200), Some(Settled::Paid(2200)));
    }

    #[test]
    fn declining_with_too_little_cash_is_short_and_raises_nothing() {
        let (mut talk, _) = asking(99);
        assert_eq!(talk.haggle(Haggle::LowerPrice, 500), Some(Settled::Short));
        assert_eq!(talk.price, 3000);
    }

    #[test]
    fn a_won_roll_lowers_whatever_the_cash() {
        let (mut talk, _) = asking(0);
        assert_eq!(talk.haggle(Haggle::LowerPrice, 0), Some(Settled::Lowered));
    }

    #[test]
    fn haggling_with_no_price_asked_does_nothing() {
        let (mut talk, _) = open(100_000, false, 0, &opening(&[0]));
        let before = talk;
        assert_eq!(talk.haggle(Haggle::Accept, 100_000), None);
        assert_eq!(talk, before);
    }
}
