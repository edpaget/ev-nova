//! Boarding a disabled ship: whether the player can, what is on board,
//! the plunder dialog's takes, the self-destruct, capturing the ship and
//! what becomes of a ship captured.
//!
//! The rules are the original's (`_HandlePlayerBoardAttempt` @0x65000,
//! `_SetPlunderValues` @0x92219, `_DoPlunderDialog` @0x9302b,
//! `_DoCapturedShipAssignmentDialog` @0x5b508 and `_DoShipCapture`
//! @0x41120 in the `EV Nova` executable); the session applies them
//! ([`Session::board`](crate::Session::board),
//! [`Session::plunder`](crate::Session::plunder) and
//! [`Session::assign`](crate::Session::assign)).
//!
//! **Boarding** ([`check_board`]): with no target nothing happens. The
//! first refusal that applies, in this order, is said: the player cannot
//! board a ship that is not disabled, has been boarded before, or has no
//! `Crew`, nor when its own ship is not intact (`STR#` 2002 #130); not
//! moving more than [`MAX_RELATIVE_SPEED`] against it on either axis
//! (#132), then not off its centre by more than its board reach on either
//! axis (#131); and, silently, the player must face within
//! [`FACING_LIMIT`] of its heading or its reverse.
//!
//! **What is on board** ([`Plunder::roll`]), rolled once a boarding, from
//! its `düde`'s `Booty` (none for a fleet's ship) and its `shïp`: credits
//! with [`MONEY`], about 2.5-5 % of its `Cost` and at least
//! [`MIN_CREDITS`]; a commodity its booty names, half its holds and a
//! draw of as many more (the original draws `Rand(7)` until it hits a
//! named commodity, and would loop for ever on a booty naming none; here
//! a booty with no commodity bit gives no cargo); all the rounds of one
//! of its ammunitions the player can fire, other than a fighter bay's;
//! and a draw of a tenth of its `Fuel`, in tens. A person (see
//! [`person`](crate::person)) has no `düde`, so no booty and no cargo;
//! the credits on board are its `Credits` as the
//! [`BoardingRule::person_credits`] says ([`RuleKey::PersonCredits`]):
//! by the engine (`_SetPlunderValues` @0x922f2-0x9238f), k = its
//! `Credits` in whole thousands kept in 16 bits (so 32,768,000 or more
//! wraps), x = k x [`PERSON_CREDITS_SHARE`], above
//! [`CREDITS_DRAW_ABOVE`] a draw of `Rand(trunc(x))` more, and trunc(x x
//! 1000) credits, none at or below none and with no floor; by the Bible
//! ("This many credits, +/- 25%"), trunc(`Credits` x (75 + `Rand(51)`) /
//! 100), none for `Credits` of none or less.
//!
//! **Capture odds** ([`NovaBoarding::capture_odds`]): ten times the ratio
//! of the player's crew (its ship's `Crew`, a tenth of each warship or
//! interceptor escort's, and the marines outfits' positive `ModVal`s) to
//! the target's, truncated; a negative marines `ModVal` adds its size in
//! points, and [`STRENGTH_BONUS`] more when the player's strength (with a
//! tenth of the escorts') is above [`STRENGTH_RATIO`] times the target's;
//! then [`JITTER`] either way, held within [`MIN_ODDS`] and [`MAX_ODDS`];
//! and none for a derelict government or a full fleet. The original also
//! zeroes the odds for an unregistered shareware copy, which is not
//! modelled.
//!
//! **The plunder dialog**: each item can be taken once ([`Take`],
//! [`Taken`]). The self-destruct threshold starts at [`THRESHOLD_BASE`]
//! and a draw of [`THRESHOLD_SPREAD`], and each take multiplies it
//! ([`CARGO_GROWTH`] and the others), truncated, whether it stored
//! anything or not. The press after a take, before it is handled, blows
//! the ship up when a draw of [`SELF_DESTRUCT_ROLL`] is at or below the
//! threshold; Abort never rolls. A self-destruct zeroes the ship's shield
//! and armour, so the fight breaks it up, and is no crime.
//!
//! **The capture roll** ([`NovaBoarding::captures`], `_DoPlunderDialog`
//! @0x942ad-0x942d8): a draw of [`CAPTURE_ROLL`] at or below the odds
//! succeeds, but odds of none never do. The original draws `Rand(100)`
//! and goes on when it is at or below the odds (or when cheats are on,
//! `_cheatsActive`), and then fails any odds not above 0; it sets the odds
//! to -1 for a derelict government just before the roll. So the button,
//! greyed at odds of none, and the roll agree. A capture that succeeds
//! still blows the ship up 1 time in [`CAPTURE_TRIP`]; with the fleet
//! full ([`MAX_ESCORTS`]) nothing changes and the dialog stays open.
//! Capturing is no crime.
//!
//! **A ship captured** ([`Assignment`]) joins the fleet with its armour at
//! [`ESCORT_ARMOR_SHARE`] of its most; or, "Use As My Ship", the player
//! takes it over, while a ship slot is free in the system
//! ([`MAX_SHIPS_IN_SYSTEM`]) for the old ship, which joins the fleet with
//! its class's stock loadout, full; with no slot free the whole swap is
//! dropped. A player of no crew captures straight into the fleet.
//!
//! **A person's grant** ([`BoardingRule::grant`]): boarding a person may
//! also give the player outfits of its `GrantClass`, as
//! [`grant`](crate::grant) says, drawn after the plunder and the odds.
//!
//! **Bible versus engine** (the [`Rulebook`]'s entries):
//! [`RuleKey::EmptyBooty`], whether a ship of `Booty` 0 repels boarders
//! (the Bible) or opens the dialog anyway (the engine), and
//! [`RuleKey::CrewlessCapture`], whether a player of no crew has no odds
//! (the Bible) or odds held at 1 or more (the engine); and for a grant,
//! [`RuleKey::GrantCount`] and [`RuleKey::GrantMax`] (see
//! [`grant`](crate::grant)).

use std::fmt::Debug;

use crate::catalog::{OutfitId, ShipId, WeaponId};
use crate::chance::Chance;
use crate::combat::aim::angle_off;
use crate::combat::hull::Condition;
use crate::flight::ShipState;
use crate::grant::{GrantStock, Granted, PersonGrant};
use crate::market::Good;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::traffic::npc::NpcId;

/// Why the player cannot board its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardRefusal {
    /// There is no target.
    NoTarget,
    /// "You can't board this ship." (`STR#` 2002 #130).
    CantBoard,
    /// "You're moving too fast to board this ship." (#132).
    TooFast,
    /// "You're not close enough to board this ship." (#131).
    TooFar,
    /// The player does not face the target's way, nor its reverse.
    Misaligned,
}

/// The ship the player tries to board, as the boarding checks see it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardTarget {
    /// Where it is and how it moves.
    pub state: ShipState,
    /// How it is holding up.
    pub condition: Condition,
    /// Whether it has been boarded before.
    pub boarded: bool,
    /// Its `shïp` `Crew`.
    pub crew: i16,
    /// How far off its centre, on either axis, the player may be.
    pub reach: f32,
}

/// How fast, in pixels a tick on either axis, the player may move against
/// the ship it boards (the float @0xdd694).
pub const MAX_RELATIVE_SPEED: f32 = 0.5;
/// How far off the target's heading, or its reverse, in degrees, the
/// player may face to board it (@0x6523b, @0x65295).
pub const FACING_LIMIT: f32 = 30.0;

impl BoardRefusal {
    /// Whether the player is told nothing: no target, or not facing it.
    #[must_use]
    pub fn silent(self) -> bool {
        matches!(self, Self::NoTarget | Self::Misaligned)
    }
}

/// The `Booty` bit for credits.
pub const MONEY: u16 = 0x0040;
/// The `Booty` bits for the six standard commodities, food (0x0001) to
/// equipment (0x0020), each commodity n's bit `1 << n`.
pub const COMMODITY_BITS: u16 = 0x003f;
/// The credits on board per thousand of the ship's `Cost` (the double
/// @0xdd610), in thousands.
pub const CREDITS_PER_THOUSAND: f64 = 0.025;
/// Above this many thousand credits, a draw of as many more thousand is
/// added (the double @0xdd1c8).
pub const CREDITS_DRAW_ABOVE: f64 = 2.0;
/// The fewest credits on board, when there are any.
pub const MIN_CREDITS: i64 = 1000;
/// A person's credits on board per thousand of its `Credits`, in
/// thousands, by the engine (0.5 @0xdd128).
pub const PERSON_CREDITS_SHARE: f64 = 0.5;
/// The Bible's least share of a person's `Credits`, in percent.
pub const BIBLE_CREDITS_LEAST: i64 = 75;
/// The spread of the Bible's share: `Rand(51)` more percent.
pub const BIBLE_CREDITS_SPREAD: u32 = 51;
/// The self-destruct threshold before any take: this and a draw of
/// [`THRESHOLD_SPREAD`] (@0x93037).
pub const THRESHOLD_BASE: u32 = 15;
/// The spread of the self-destruct threshold's draw.
pub const THRESHOLD_SPREAD: u32 = 26;
/// The energy on board is a draw of a tenth of the `Fuel`, in tens.
pub const ENERGY_STEP: u32 = 10;

/// The rounds of one ammunition a boarded ship holds, as the plunder
/// sees them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldRounds {
    /// The weapon they are the rounds of.
    pub ammo: WeaponId,
    /// How many it holds.
    pub rounds: u32,
    /// Whether a fighter bay (guidance 99) fires them: its fighters,
    /// which cannot be plundered.
    pub bay: bool,
    /// The ammunition outfit the player can take them as: one whose
    /// rounds a weapon the player carries fires. `None` when the player
    /// cannot use them.
    pub outfit: Option<OutfitId>,
}

/// What a boarded ship has on board to roll its plunder from: its
/// `düde`'s `Booty`, its `shïp`'s `Cost`, `Holds` and `Fuel`, and the
/// rounds it holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Prize {
    /// Its `düde`'s `Booty` flags; none for a fleet's ship.
    pub booty: u16,
    /// Its `shïp`'s `Cost`.
    pub cost: i32,
    /// Its `shïp`'s `Holds`.
    pub holds: i16,
    /// Its `shïp`'s `Fuel`.
    pub fuel: i16,
    /// The rounds of each ammunition it holds.
    pub rounds: Vec<HeldRounds>,
    /// Its person's `Credits`, when a person flies it: they, not its
    /// `Cost`, give the credits on board.
    pub person_credits: Option<i32>,
}

/// What can be taken from a boarded ship, rolled once a boarding, and the
/// self-destruct threshold. Nothing here is saved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Plunder {
    /// The credits on board; none to take when 0.
    pub credits: i64,
    /// The cargo on board: a commodity and its tons.
    pub cargo: Option<(Good, u32)>,
    /// The ammunition on board: the outfit the player takes it as, and
    /// how many rounds.
    pub ammo: Option<(OutfitId, u32)>,
    /// The energy on board, in fuel units; none to take when 0.
    pub fuel: u32,
    /// The capture odds, in percent; Capture is offered only above 0.
    pub odds: u8,
    /// The self-destruct threshold: each press after a take blows the ship
    /// up when a draw of 100 is at or below it.
    pub threshold: u32,
    /// Whether the last press took something, so the next is rolled.
    pub armed: bool,
}

impl Plunder {
    /// The plunder on board `prize`, rolled on `chance`, in the original's
    /// order: the threshold, the credits (a person's as `rule` says), the
    /// cargo, the ammunition and the energy (see the module docs). The
    /// odds are left at none, for the [`BoardingRule`] to set.
    pub fn roll(prize: &Prize, rule: &dyn BoardingRule, chance: &mut dyn Chance) -> Self {
        let threshold = THRESHOLD_BASE + chance.below(THRESHOLD_SPREAD);
        let credits = if let Some(credits) = prize.person_credits {
            rule.person_credits(credits, chance)
        } else if prize.booty & MONEY == 0 {
            0
        } else {
            let mut thousands = f64::from(prize.cost / 1000) * CREDITS_PER_THOUSAND;
            if thousands > CREDITS_DRAW_ABOVE {
                thousands += f64::from(chance.below(thousands as u32));
            }
            ((thousands * 1000.0) as i64).max(MIN_CREDITS)
        };
        let cargo = cargo(prize, chance);
        let candidates: Vec<(OutfitId, u32)> = prize
            .rounds
            .iter()
            .filter(|held| held.rounds > 0 && !held.bay)
            .filter_map(|held| Some((held.outfit?, held.rounds)))
            .collect();
        let ammo = if candidates.is_empty() {
            None
        } else {
            let index = chance.below(candidates.len() as u32) as usize;
            candidates.get(index).copied()
        };
        let tenths = u32::try_from(prize.fuel).unwrap_or(0) / ENERGY_STEP;
        let fuel = if tenths > 0 {
            chance.below(tenths) * ENERGY_STEP
        } else {
            0
        };
        Self {
            credits,
            cargo,
            ammo,
            fuel,
            odds: 0,
            threshold,
            armed: false,
        }
    }
}

/// The cargo on board `prize`, rolled on `chance`: one of the commodities
/// its booty names, then half its holds and a draw of as many more; none
/// without a commodity bit, and none when half the holds is none.
fn cargo(prize: &Prize, chance: &mut dyn Chance) -> Option<(Good, u32)> {
    let named: Vec<u8> = (0..6u8)
        .filter(|&n| prize.booty & COMMODITY_BITS & (1 << n) != 0)
        .collect();
    if named.is_empty() {
        return None;
    }
    let commodity = named[chance.below(named.len() as u32) as usize];
    let half = u32::try_from(prize.holds / 2).unwrap_or(0);
    if half == 0 {
        return None;
    }
    let tons = chance.below(half) + half;
    Some((Good::Commodity(commodity), tons))
}

/// The most escorts the player's fleet holds (`_CanHireEscorts`, counting
/// those that are not fighters).
pub const MAX_ESCORTS: usize = 6;
/// The most ships in a system, the player's among them (the Bible's "Max
/// Ships In System"): the old ship is kept as an escort after "Use As My
/// Ship" only while a slot is free.
pub const MAX_SHIPS_IN_SYSTEM: usize = 64;
/// How much each take multiplies the self-destruct threshold by, per
/// item: cargo and ammo double it, credits (the double @0xdd5c8) and
/// energy (@0xdd640) less.
pub const CARGO_GROWTH: f64 = 2.0;
/// See [`CARGO_GROWTH`].
pub const AMMO_GROWTH: f64 = 2.0;
/// See [`CARGO_GROWTH`].
pub const CREDITS_GROWTH: f64 = 1.25;
/// See [`CARGO_GROWTH`].
pub const ENERGY_GROWTH: f64 = 1.5;
/// The draw each press after a take rolls: the ship blows up at or below
/// the threshold (@0x935f7).
pub const SELF_DESTRUCT_ROLL: u32 = 100;
/// The guidance of a fighter bay: its rounds are fighters, never
/// plundered.
pub use crate::bay::FIGHTER_BAY;
/// The draw a successful capture rolls: the ship blows up on 0, a flat 1
/// in 10 (@0x942de).
pub const CAPTURE_TRIP: u32 = 10;
/// The share of its most armour a ship captured as an escort keeps (the
/// float @0xdd694).
pub const ESCORT_ARMOR_SHARE: f32 = 0.5;
/// The share of its `Armor` a ship taken over with "Use As My Ship" holds,
/// before [`TAKEOVER_ARMOR_BASE`] (@0x417f4-0x4183a).
pub const TAKEOVER_ARMOR_SHARE: f64 = 0.3333;
/// [`TAKEOVER_ARMOR_SHARE`] for a tough ship type (`shïp` `Flags`
/// 0x0010).
pub const TOUGH_TAKEOVER_ARMOR_SHARE: f64 = 0.1;
/// The armour a ship taken over holds besides its share.
pub const TAKEOVER_ARMOR_BASE: f64 = 2.0;

/// A press in the plunder dialog: a button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Take {
    /// "Cargo" (item 2).
    Cargo,
    /// "Credits" (item 3).
    Credits,
    /// "Ammo" (item 4).
    Ammo,
    /// "Energy" (item 6).
    Energy,
    /// "Capture Ship" (item 7).
    Capture,
    /// "Abort" (item 1), and Return, Enter and Escape.
    Abort,
}

/// What a press in the plunder dialog did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    /// The press did nothing: no boarding is under way, or what it takes
    /// is not on board (its button is greyed).
    Nothing,
    /// The cargo was taken: `stored` tons of `good`, as much as the hold
    /// had room for.
    Cargo {
        /// The good.
        good: Good,
        /// The tons stored; none when the hold was full.
        stored: u32,
    },
    /// These credits were taken.
    Credits(i64),
    /// `count` of the ammunition outfit `outfit` were taken, as many as
    /// the free mass, the outfit's `Max` and the rounds on board allow.
    Ammo {
        /// The ammunition outfit.
        outfit: OutfitId,
        /// How many were taken; none when there was no room.
        count: u32,
    },
    /// The energy was taken: `stored` of the `offered` fuel, as much as
    /// the tank had room for.
    Energy {
        /// The fuel on board.
        offered: u32,
        /// The fuel stored.
        stored: u32,
        /// Whether the tank is full after it, which the original's
        /// message tells apart from all of it stored.
        full: bool,
    },
    /// The self-destruct went off: the ship is breaking up, and the
    /// boarding is over.
    Tripped,
    /// The capture succeeded; [`Session::assign`](crate::Session::assign)
    /// says what becomes of the ship.
    Captured,
    /// The capture succeeded, and with no crew to trade places with, the
    /// ship joined the fleet at once.
    Escorted,
    /// The capture failed, and the boarding is over.
    CaptureFailed,
    /// The capture succeeded, but the fleet is full: nothing changes, and
    /// the boarding goes on.
    FleetFull,
    /// The boarding is over.
    Aborted,
}

/// What boarding a ship did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boarding {
    /// The plunder dialog opens on what is on board.
    Opened(PlunderView),
    /// The crew repelled the boarders: nothing can be taken.
    Repelled,
}

/// What is on board the ship being boarded, as the plunder dialog shows
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlunderView {
    /// The ship boarded.
    pub npc: NpcId,
    /// Its ship class.
    pub ship: ShipId,
    /// The credits on board; none to take when 0.
    pub credits: i64,
    /// The cargo on board.
    pub cargo: Option<(Good, u32)>,
    /// The ammunition on board: the outfit, and how many rounds.
    pub ammo: Option<(OutfitId, u32)>,
    /// The energy on board, in fuel units.
    pub fuel: u32,
    /// The capture odds, in percent.
    pub odds: u8,
}

impl PlunderView {
    /// Whether `take`'s button is enabled: its value is on board. Abort
    /// always is.
    #[must_use]
    pub fn offers(&self, take: Take) -> bool {
        match take {
            Take::Cargo => self.cargo.is_some(),
            Take::Credits => self.credits > 0,
            Take::Ammo => self.ammo.is_some(),
            Take::Energy => self.fuel > 0,
            Take::Capture => self.odds > 0,
            Take::Abort => true,
        }
    }
}

/// What the player makes of a ship captured: the assignment dialog's
/// buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assignment {
    /// "Use As Escort" (item 2).
    Escort,
    /// "Use As My Ship" (item 1).
    MyShip,
}

/// What an assignment did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assigned {
    /// The ship joined the fleet (`STR#` 2002 #123).
    Escort,
    /// The player took the ship over, and the old one joined the fleet
    /// (#304).
    MyShip,
    /// No ship slot was free for the old ship, so the swap was dropped and
    /// the prize lost (#305).
    Abandoned,
}

/// The share of each warship or interceptor escort's crew and strength
/// that counts toward the player's (the double @0xdd110).
pub const ESCORT_SHARE: f64 = 0.1;
/// The fewest `InherentAI` above which an escort's crew counts: warships
/// (3) and interceptors (4).
pub const ESCORT_AI_ABOVE: i16 = 2;
/// The `ModType` of marines: a positive `ModVal` adds crew, a negative one
/// adds its size in points to the odds.
pub const MARINES: i16 = 25;
/// The odds per crew ratio: crew / (target crew x 10) x 100 (the doubles
/// @0xdd0b0 and @0xdd0e0).
pub const CREW_RATIO_DIVISOR: f64 = 10.0;
/// The points the odds gain when the player's strength is above
/// [`STRENGTH_RATIO`] times the target's.
pub const STRENGTH_BONUS: i32 = 10;
/// How many times the target's strength the player's must be above for
/// the [`STRENGTH_BONUS`].
pub const STRENGTH_RATIO: i32 = 5;
/// How far the odds jitter either way: + 5 - a draw of 11.
pub const JITTER: i32 = 5;
/// The odds' least, before the zeroing.
pub const MIN_ODDS: i32 = 1;
/// The odds' most.
pub const MAX_ODDS: i32 = 75;
/// The draw the capture roll makes: it succeeds at or below the odds.
pub const CAPTURE_ROLL: u32 = 100;

/// One of the player's escorts, as the capture odds count it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortCrew {
    /// Its `shïp` `Crew`.
    pub crew: i32,
    /// Its `shïp` `Strength`.
    pub strength: i32,
    /// Its `shïp` `InherentAI`.
    pub inherent_ai: i16,
}

/// Everything the capture odds are worked out from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CaptureCrew {
    /// The player's ship's `Crew`.
    pub crew: i32,
    /// The player's ship's `Strength`.
    pub strength: i32,
    /// The player's escorts.
    pub escorts: Vec<EscortCrew>,
    /// Each marines outfit the player owns (`ModType` [`MARINES`]): its
    /// `ModVal`, and how many.
    pub marines: Vec<(i16, u16)>,
    /// The target's `Crew`.
    pub target_crew: i32,
    /// The target's `Strength`.
    pub target_strength: i32,
    /// Whether the target's government is derelict.
    pub derelict: bool,
    /// Whether the fleet has room for one more escort.
    pub fleet_room: bool,
}

/// Boarding's rules that a mod might replace: who repels boarders, the
/// capture odds and the capture roll. Nova's is [`NovaBoarding`].
pub trait BoardingRule: Debug {
    /// Whether a ship of `booty` repels the boarders: the board still
    /// happens, but nothing can be taken.
    fn repels(&self, booty: u16) -> bool;
    /// The capture odds, in percent, for `crew`, jittered on `chance`.
    fn capture_odds(&self, crew: &CaptureCrew, chance: &mut dyn Chance) -> u8;
    /// Whether a capture at `odds` succeeds, rolled on `chance`.
    fn captures(&self, odds: u8, chance: &mut dyn Chance) -> bool;
    /// The credits a person of `Credits` `credits` carries, drawn on
    /// `chance`.
    fn person_credits(&self, credits: i32, chance: &mut dyn Chance) -> i64;
    /// What boarding a person of `grant` gives the player from `stock`,
    /// every outfit as the grant sees it, with `free_mass` free, drawn on
    /// `chance`; none when nothing is granted (see [`grant`](crate::grant)).
    fn grant(
        &self,
        grant: &PersonGrant,
        stock: &[GrantStock],
        free_mass: i64,
        chance: &mut dyn Chance,
    ) -> Option<Granted>;
}

/// Nova's boarding rules (see the module docs), the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaBoarding {
    /// Whether a ship of `Booty` 0 repels boarders: by the engine, never;
    /// by the Bible, always ([`RuleKey::EmptyBooty`]).
    pub empty_booty: RuleSource,
    /// Whether a player of no crew can capture: by the engine, its odds
    /// are held at 1 or more; by the Bible, they are none
    /// ([`RuleKey::CrewlessCapture`]).
    pub crewless_capture: RuleSource,
    /// The credits a person carries: by the engine, half its `Credits`
    /// and a draw more; by the Bible, its `Credits` +/- 25 %
    /// ([`RuleKey::PersonCredits`]).
    pub person_credits: RuleSource,
    /// How many outfits a person grants: by the engine, half its
    /// `GrantCount` to all of it; otherwise 1 to all of it evenly
    /// ([`RuleKey::GrantCount`]).
    pub grant_count: RuleSource,
    /// Whether a grant may pass the outfit's `Max`: by the engine, it
    /// may; otherwise it is held to it ([`RuleKey::GrantMax`]).
    pub grant_max: RuleSource,
}

impl NovaBoarding {
    /// The rules `rulebook` chooses: its [`RuleKey::EmptyBooty`],
    /// [`RuleKey::CrewlessCapture`], [`RuleKey::PersonCredits`],
    /// [`RuleKey::GrantCount`] and [`RuleKey::GrantMax`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            empty_booty: rulebook.source_for(RuleKey::EmptyBooty),
            crewless_capture: rulebook.source_for(RuleKey::CrewlessCapture),
            person_credits: rulebook.source_for(RuleKey::PersonCredits),
            grant_count: rulebook.source_for(RuleKey::GrantCount),
            grant_max: rulebook.source_for(RuleKey::GrantMax),
        }
    }
}

impl BoardingRule for NovaBoarding {
    fn repels(&self, booty: u16) -> bool {
        self.empty_booty == RuleSource::Bible && booty == 0
    }

    fn capture_odds(&self, crew: &CaptureCrew, chance: &mut dyn Chance) -> u8 {
        let share =
            |total: i32, part: i32| (f64::from(total) + ESCORT_SHARE * f64::from(part)) as i32;
        let mut men = crew.crew;
        let mut strength = crew.strength;
        for escort in crew
            .escorts
            .iter()
            .filter(|escort| escort.inherent_ai > ESCORT_AI_ABOVE)
        {
            men = share(men, escort.crew);
            strength = share(strength, escort.strength);
        }
        // A positive `ModVal` is so many more crew, a negative one so many
        // more points; none is nothing either way.
        let marines = |sign: i32| {
            crew.marines
                .iter()
                .map(|&(value, count)| (sign * i32::from(value)).max(0) * i32::from(count))
                .sum::<i32>()
        };
        men += marines(1);
        let ratio = f64::from(men) / (f64::from(crew.target_crew) * CREW_RATIO_DIVISOR) * 100.0;
        let mut odds = ratio as i32 + marines(-1);
        if STRENGTH_RATIO * crew.target_strength < strength {
            odds += STRENGTH_BONUS;
        }
        odds = odds + JITTER - chance.below(2 * JITTER as u32 + 1) as i32;
        let odds = odds.clamp(MIN_ODDS, MAX_ODDS);
        let crewless = self.crewless_capture == RuleSource::Bible && crew.crew <= 0;
        if crew.derelict || !crew.fleet_room || crewless {
            0
        } else {
            u8::try_from(odds).unwrap_or(0)
        }
    }

    fn captures(&self, odds: u8, chance: &mut dyn Chance) -> bool {
        let draw = chance.below(CAPTURE_ROLL);
        odds > 0 && draw <= u32::from(odds)
    }

    fn person_credits(&self, credits: i32, chance: &mut dyn Chance) -> i64 {
        match self.person_credits {
            RuleSource::Engine => {
                // `movswl` @0x92326: the thousands kept in 16 bits.
                let thousands = (credits / 1000) as i16;
                let mut x = f64::from(thousands) * PERSON_CREDITS_SHARE;
                if x > CREDITS_DRAW_ABOVE {
                    x += f64::from(chance.below(x as u32));
                }
                ((x * 1000.0) as i64).max(0)
            }
            RuleSource::Bible => {
                if credits <= 0 {
                    return 0;
                }
                let percent = BIBLE_CREDITS_LEAST + i64::from(chance.below(BIBLE_CREDITS_SPREAD));
                i64::from(credits) * percent / 100
            }
        }
    }

    fn grant(
        &self,
        grant: &PersonGrant,
        stock: &[GrantStock],
        free_mass: i64,
        chance: &mut dyn Chance,
    ) -> Option<Granted> {
        crate::grant::roll(
            grant,
            stock,
            free_mass,
            (self.grant_count, self.grant_max),
            chance,
        )
    }
}

/// Whether the player's ship, at `player` and in `condition`, can board
/// `target`; the first refusal that applies, in the original's order,
/// otherwise (see the module docs).
///
/// # Errors
///
/// The [`BoardRefusal`] that applies.
pub fn check_board(
    player: &ShipState,
    condition: Condition,
    target: Option<BoardTarget>,
) -> Result<(), BoardRefusal> {
    let target = target.ok_or(BoardRefusal::NoTarget)?;
    if condition != Condition::Intact
        || target.condition != Condition::Disabled
        || target.boarded
        || target.crew <= 0
    {
        return Err(BoardRefusal::CantBoard);
    }
    let beyond = |offset: crate::geometry::Vec2, limit: f32| {
        offset.x.abs() > limit || offset.y.abs() > limit
    };
    if beyond(player.velocity - target.state.velocity, MAX_RELATIVE_SPEED) {
        return Err(BoardRefusal::TooFast);
    }
    if beyond(player.position - target.state.position, target.reach) {
        return Err(BoardRefusal::TooFar);
    }
    // Off the target's heading by 0 to 180 degrees: within the limit of
    // it, or of its reverse, is facing it.
    let off = angle_off(player.heading, target.state.heading);
    if off > FACING_LIMIT && off < 180.0 - FACING_LIMIT {
        return Err(BoardRefusal::Misaligned);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::trivially_copy_pass_by_ref)]
mod tests {
    use super::*;
    use crate::geometry::Vec2;
    use crate::grant::{COUNT_FLOOR_PERCENT, COUNT_PERCENT, COUNT_SPREAD, GRANT_ROLL};
    use crate::testkit::Draws;

    /// A disabled ship of crew 3 at the centre, at rest, facing up, never
    /// boarded, with a reach of 20.
    fn quarry() -> BoardTarget {
        BoardTarget {
            state: ShipState::default(),
            condition: Condition::Disabled,
            boarded: false,
            crew: 3,
            reach: 20.0,
        }
    }

    /// The player at `position` moving at `velocity`, facing `heading`.
    fn player(position: (f32, f32), velocity: (f32, f32), heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(position.0, position.1),
            velocity: Vec2::new(velocity.0, velocity.1),
            heading,
        }
    }

    fn board(player: ShipState, target: BoardTarget) -> Result<(), BoardRefusal> {
        check_board(&player, Condition::Intact, Some(target))
    }

    fn over() -> ShipState {
        player((0.0, 0.0), (0.0, 0.0), 0.0)
    }

    #[test]
    fn an_intact_player_over_a_disabled_ship_at_its_speed_and_heading_boards_it() {
        assert_eq!(board(over(), quarry()), Ok(()));
    }

    #[test]
    fn with_no_target_nothing_happens() {
        assert_eq!(
            check_board(&over(), Condition::Intact, None),
            Err(BoardRefusal::NoTarget)
        );
        assert_eq!(
            check_board(&over(), Condition::Disabled, None),
            Err(BoardRefusal::NoTarget),
            "before the player's own state"
        );
    }

    #[test]
    fn an_intact_dying_or_gone_ship_cannot_be_boarded() {
        for condition in [
            Condition::Intact,
            Condition::Dying { ticks_left: 4 },
            Condition::Destroyed,
        ] {
            let target = BoardTarget {
                condition,
                ..quarry()
            };
            assert_eq!(
                board(over(), target),
                Err(BoardRefusal::CantBoard),
                "{condition:?}"
            );
        }
    }

    #[test]
    fn a_ship_boarded_before_cannot_be_boarded_again() {
        let target = BoardTarget {
            boarded: true,
            ..quarry()
        };
        assert_eq!(board(over(), target), Err(BoardRefusal::CantBoard));
    }

    #[test]
    fn a_ship_with_no_crew_cannot_be_boarded() {
        for crew in [0, -1] {
            let target = BoardTarget { crew, ..quarry() };
            assert_eq!(
                board(over(), target),
                Err(BoardRefusal::CantBoard),
                "{crew}"
            );
        }
        let target = BoardTarget {
            crew: 1,
            ..quarry()
        };
        assert_eq!(board(over(), target), Ok(()));
    }

    #[test]
    fn a_player_not_intact_cannot_board() {
        for condition in [Condition::Disabled, Condition::Dying { ticks_left: 1 }] {
            assert_eq!(
                check_board(&over(), condition, Some(quarry())),
                Err(BoardRefusal::CantBoard),
                "{condition:?}"
            );
        }
    }

    #[test]
    fn half_a_pixel_a_tick_either_way_on_either_axis_is_slow_enough() {
        let drifting = BoardTarget {
            state: ShipState {
                velocity: Vec2::new(1.0, -2.0),
                ..ShipState::default()
            },
            ..quarry()
        };
        for (vx, vy) in [(1.5, -2.0), (0.5, -2.0), (1.0, -1.5), (1.0, -2.5)] {
            assert_eq!(board(player((0.0, 0.0), (vx, vy), 0.0), drifting), Ok(()));
        }
        for (vx, vy) in [(1.51, -2.0), (0.49, -2.0), (1.0, -1.49), (1.0, -2.51)] {
            assert_eq!(
                board(player((0.0, 0.0), (vx, vy), 0.0), drifting),
                Err(BoardRefusal::TooFast),
                "({vx}, {vy})"
            );
        }
        assert_eq!(MAX_RELATIVE_SPEED, 0.5);
    }

    #[test]
    fn within_the_reach_on_either_axis_is_close_enough() {
        for (x, y) in [(20.0, 0.0), (-20.0, 20.0), (0.0, -20.0), (20.0, 20.0)] {
            assert_eq!(board(player((x, y), (0.0, 0.0), 0.0), quarry()), Ok(()));
        }
        for (x, y) in [(20.01, 0.0), (-20.01, 0.0), (0.0, 20.01), (0.0, -20.01)] {
            assert_eq!(
                board(player((x, y), (0.0, 0.0), 0.0), quarry()),
                Err(BoardRefusal::TooFar),
                "({x}, {y})"
            );
        }
        // Measured from wherever the target is.
        let away = BoardTarget {
            state: ShipState {
                position: Vec2::new(100.0, 50.0),
                ..ShipState::default()
            },
            ..quarry()
        };
        assert_eq!(board(player((115.0, 35.0), (0.0, 0.0), 0.0), away), Ok(()));
        assert_eq!(
            board(player((0.0, 0.0), (0.0, 0.0), 0.0), away),
            Err(BoardRefusal::TooFar)
        );
    }

    #[test]
    fn too_fast_is_said_before_too_far() {
        assert_eq!(
            board(player((50.0, 0.0), (3.0, 0.0), 0.0), quarry()),
            Err(BoardRefusal::TooFast)
        );
    }

    #[test]
    fn within_30_degrees_of_its_heading_or_the_reverse_faces_it() {
        let facing = |heading| BoardTarget {
            state: ShipState {
                heading,
                ..ShipState::default()
            },
            ..quarry()
        };
        for (mine, theirs) in [
            (30.0, 0.0),
            (330.0, 0.0),
            (0.0, 30.0),
            (180.0, 0.0),
            (210.0, 0.0),
            (150.0, 0.0),
            (100.0, 280.0),
        ] {
            assert_eq!(
                board(player((0.0, 0.0), (0.0, 0.0), mine), facing(theirs)),
                Ok(()),
                "{mine} on {theirs}"
            );
        }
        for (mine, theirs) in [
            (31.0, 0.0),
            (329.0, 0.0),
            (0.0, 31.0),
            (211.0, 0.0),
            (149.0, 0.0),
            (90.0, 0.0),
        ] {
            assert_eq!(
                board(player((0.0, 0.0), (0.0, 0.0), mine), facing(theirs)),
                Err(BoardRefusal::Misaligned),
                "{mine} on {theirs}"
            );
        }
        assert_eq!(FACING_LIMIT, 30.0);
    }

    // What is on board.

    /// A ship of `Cost` 10,000, 15 holds and 300 fuel with `booty`,
    /// holding no rounds.
    fn prize(booty: u16) -> Prize {
        Prize {
            booty,
            cost: 10_000,
            holds: 15,
            fuel: 300,
            rounds: Vec::new(),
            person_credits: None,
        }
    }

    fn roll(prize: &Prize, draws: &[u32]) -> (Plunder, Vec<u32>) {
        roll_by(&NovaBoarding::default(), prize, draws)
    }

    fn roll_by(rule: &dyn BoardingRule, prize: &Prize, draws: &[u32]) -> (Plunder, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let plunder = Plunder::roll(prize, rule, &mut chance);
        (plunder, chance.asked)
    }

    /// A person's credits by `rule` for `Credits` `credits`, drawn from
    /// `draws`, and the bounds asked.
    fn person_credits(rule: RuleSource, credits: i32, draws: &[u32]) -> (i64, Vec<u32>) {
        let rule = NovaBoarding {
            person_credits: rule,
            ..NovaBoarding::default()
        };
        let mut chance = Draws::of(draws);
        let credits = rule.person_credits(credits, &mut chance);
        (credits, chance.asked)
    }

    #[test]
    fn by_the_engine_a_person_carries_half_its_credits_in_thousands_and_a_draw_more() {
        let engine = RuleSource::Engine;
        assert_eq!(person_credits(engine, 8000, &[3]), (7000, vec![4]));
        assert_eq!(person_credits(engine, 75_000, &[0]), (37_500, vec![37]));
        assert_eq!(
            person_credits(engine, 1000, &[]),
            (500, vec![]),
            "no draw at 0.5"
        );
        assert_eq!(
            person_credits(engine, 4000, &[]),
            (2000, vec![]),
            "none at 2"
        );
        assert_eq!(
            person_credits(engine, 4999, &[]),
            (2000, vec![]),
            "thousands truncated"
        );
        assert_eq!(person_credits(engine, 999, &[]), (0, vec![]));
        assert_eq!(
            person_credits(engine, 42_000_000, &[]),
            (0, vec![]),
            "42,000 thousands wrap in 16 bits below none"
        );
        assert_eq!(
            person_credits(engine, 65_536_000, &[]),
            (0, vec![]),
            "65,536 thousands wrap to none"
        );
        assert_eq!(
            person_credits(engine, 65_540_000, &[0]),
            (2000, vec![]),
            "and 65,540 to 4"
        );
        assert_eq!(person_credits(engine, 0, &[]), (0, vec![]));
        assert_eq!(person_credits(engine, -8000, &[]), (0, vec![]));
        assert_eq!(PERSON_CREDITS_SHARE, 0.5);
    }

    #[test]
    fn by_the_bible_a_person_carries_its_credits_give_or_take_a_quarter() {
        let bible = RuleSource::Bible;
        assert_eq!(person_credits(bible, 8000, &[0]), (6000, vec![51]));
        assert_eq!(person_credits(bible, 8000, &[50]), (10_000, vec![51]));
        assert_eq!(person_credits(bible, 8000, &[25]), (8000, vec![51]));
        assert_eq!(person_credits(bible, 7, &[0]), (5, vec![51]), "truncated");
        assert_eq!(person_credits(bible, 0, &[]), (0, vec![]));
        assert_eq!(person_credits(bible, -100, &[]), (0, vec![]));
        assert_eq!(
            person_credits(bible, 42_000_000, &[50]),
            (52_500_000, vec![51]),
            "no wrap"
        );
    }

    #[test]
    fn a_persons_plunder_rolls_the_threshold_then_its_credits_and_no_cargo() {
        let person = Prize {
            booty: 0,
            person_credits: Some(8000),
            ..prize(0x003f)
        };
        let (plunder, asked) = roll(&person, &[0, 3, 0]);
        assert_eq!(asked, [26, 4, 30], "threshold, credits, then the energy");
        assert_eq!(plunder.credits, 7000);
        assert_eq!(plunder.cargo, None);
        let bible = NovaBoarding {
            person_credits: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        let (plunder, asked) = roll_by(&bible, &person, &[0, 50, 0]);
        assert_eq!(asked, [26, 51, 30]);
        assert_eq!(plunder.credits, 10_000);
        let dude = Prize {
            cost: 150_000,
            ..prize(MONEY)
        };
        assert_eq!(
            roll_by(&bible, &dude, &[0, 2]).0.credits,
            5750,
            "a düde ship's credits are as before"
        );
    }

    #[test]
    fn the_self_destruct_threshold_is_15_and_a_draw_of_26() {
        let (plunder, asked) = roll(&prize(0), &[7]);
        assert_eq!(plunder.threshold, 22);
        assert_eq!(asked[0], 26, "drawn first");
        assert!(!plunder.armed, "nothing taken yet");
        assert_eq!((THRESHOLD_BASE, THRESHOLD_SPREAD), (15, 26));
    }

    #[test]
    fn credits_are_a_fortieth_of_the_cost_in_thousands_and_at_least_1000() {
        let credits = |cost, draws: &[u32]| {
            let money = Prize {
                cost,
                ..prize(MONEY)
            };
            roll(&money, draws)
        };
        let (cheap, asked) = credits(10_000, &[0]);
        assert_eq!(cheap.credits, 1000, "250, the floor");
        assert_eq!(asked, [26, 30], "no credit draw at 0.25, then the fuel");
        let (dear, asked) = credits(150_000, &[0, 2]);
        assert_eq!(dear.credits, 5750, "3.75 + 2");
        assert_eq!(asked[..2], [26, 3], "the draw below trunc(3.75)");
        let (even, asked) = credits(80_000, &[0]);
        assert_eq!(even.credits, 2000, "2.0 draws nothing");
        assert_eq!(asked, [26, 30]);
        let (dear, _) = credits(150_999, &[0, 0]);
        assert_eq!(dear.credits, 3750, "the cost truncated to thousands");
        let (none, _) = roll(
            &Prize {
                cost: 150_000,
                ..prize(0x003f)
            },
            &[0, 0, 0],
        );
        assert_eq!(none.credits, 0, "only with 0x0040");
        assert_eq!(MIN_CREDITS, 1000);
    }

    #[test]
    fn the_cargo_is_a_commodity_its_booty_names_and_half_the_holds_and_a_draw() {
        // Food (0x01), metal (0x10) and equipment (0x20): the draw 1 is
        // metal, commodity 4.
        let (plunder, asked) = roll(&prize(0x0031), &[0, 1, 3]);
        assert_eq!(asked[..3], [26, 3, 7]);
        assert_eq!(plunder.cargo, Some((Good::Commodity(4), 10)), "3 + 7");
        let (plunder, _) = roll(&prize(0x0031), &[0, 2, 0]);
        assert_eq!(plunder.cargo, Some((Good::Commodity(5), 7)));
        let (plunder, _) = roll(&prize(0x0031), &[0, 0, 6]);
        assert_eq!(plunder.cargo, Some((Good::Commodity(0), 13)));
        let (plunder, _) = roll(&prize(0x0042), &[0, 0, 0]);
        assert_eq!(plunder.cargo, Some((Good::Commodity(1), 7)), "money aside");
    }

    #[test]
    fn no_cargo_without_a_commodity_bit_or_holds() {
        let (plunder, asked) = roll(&prize(0x0100), &[0]);
        assert_eq!(plunder.cargo, None);
        assert_eq!(asked, [26, 30], "nothing drawn for cargo");
        let (plunder, asked) = roll(&prize(0x0040), &[0]);
        assert_eq!(plunder.cargo, None);
        assert_eq!(asked, [26, 30]);
        for holds in [0, 1, -4] {
            let (plunder, asked) = roll(
                &Prize {
                    holds,
                    ..prize(0x0001)
                },
                &[0, 0],
            );
            assert_eq!(plunder.cargo, None, "{holds}");
            assert_eq!(asked, [26, 1, 30], "{holds}: only the good drawn");
        }
    }

    /// Rounds of weapon `ammo`: `rounds` of them, usable as outfit
    /// `outfit`, from a fighter bay or not.
    fn held(ammo: i16, rounds: u32, bay: bool, outfit: Option<i16>) -> HeldRounds {
        HeldRounds {
            ammo: WeaponId(ammo),
            rounds,
            bay,
            outfit: outfit.map(OutfitId),
        }
    }

    #[test]
    fn the_ammo_is_all_of_one_usable_weapons_rounds_drawn_among_them() {
        let armed = Prize {
            rounds: vec![
                held(130, 4, false, Some(200)),
                held(131, 0, false, Some(201)),
                held(132, 9, true, Some(202)),
                held(133, 5, false, None),
                held(134, 12, false, Some(204)),
            ],
            ..prize(0)
        };
        let (plunder, asked) = roll(&armed, &[0, 1, 0]);
        assert_eq!(asked[..2], [26, 2], "two candidates: 130 and 134");
        assert_eq!(plunder.ammo, Some((OutfitId(204), 12)));
        let (plunder, _) = roll(&armed, &[0, 0, 0]);
        assert_eq!(plunder.ammo, Some((OutfitId(200), 4)));
        let unusable = Prize {
            rounds: vec![
                held(131, 0, false, Some(201)),
                held(132, 9, true, Some(202)),
                held(133, 5, false, None),
            ],
            ..prize(0)
        };
        let (plunder, asked) = roll(&unusable, &[0, 0]);
        assert_eq!(plunder.ammo, None);
        assert_eq!(asked, [26, 30], "nothing drawn for ammo");
    }

    #[test]
    fn the_energy_is_a_draw_of_a_tenth_of_the_fuel_in_tens() {
        let (plunder, asked) = roll(&prize(0), &[0, 17]);
        assert_eq!(asked, [26, 30]);
        assert_eq!(plunder.fuel, 170);
        for fuel in [9, 0, -1] {
            let (plunder, asked) = roll(&Prize { fuel, ..prize(0) }, &[0]);
            assert_eq!(plunder.fuel, 0, "{fuel}");
            assert_eq!(asked, [26], "{fuel}: nothing drawn");
        }
    }

    // Capture.

    /// A player of `crew` and strength 0, alone, without marines, against
    /// a ship of `target` crew and strength 1000, of no derelict
    /// government, with room in the fleet.
    fn crews(crew: i32, target: i32) -> CaptureCrew {
        CaptureCrew {
            crew,
            strength: 0,
            escorts: Vec::new(),
            marines: Vec::new(),
            target_crew: target,
            target_strength: 1000,
            derelict: false,
            fleet_room: true,
        }
    }

    /// The odds `rule` gives `crew` with the jitter's draw `jitter` (5 is
    /// none), and the draws it asked for.
    fn odds_with(rule: &NovaBoarding, crew: &CaptureCrew, jitter: u32) -> (u8, Vec<u32>) {
        let mut chance = Draws::of(&[jitter]);
        let odds = rule.capture_odds(crew, &mut chance);
        (odds, chance.asked)
    }

    fn odds(crew: &CaptureCrew) -> u8 {
        odds_with(&NovaBoarding::default(), crew, 5).0
    }

    #[test]
    fn the_odds_are_ten_times_the_crews_ratio_truncated() {
        let (ten_to_one, asked) = odds_with(&NovaBoarding::default(), &crews(10, 1), 5);
        assert_eq!(ten_to_one, 75, "100, capped");
        assert_eq!(asked, [11], "the jitter's one draw");
        assert_eq!(odds(&crews(2, 3)), 6, "6.67");
        assert_eq!(odds(&crews(3, 1)), 30);
        assert_eq!(odds(&crews(50, 7)), 71, "71.4");
    }

    #[test]
    fn each_warship_or_interceptor_escort_adds_a_tenth_of_its_crew_and_strength() {
        let escort = |crew, strength, inherent_ai| EscortCrew {
            crew,
            strength,
            inherent_ai,
        };
        let with = |escorts: Vec<EscortCrew>| CaptureCrew {
            escorts,
            ..crews(10, 10)
        };
        assert_eq!(odds(&with(Vec::new())), 10);
        assert_eq!(
            odds(&with(vec![escort(5, 0, 3), escort(5, 0, 4)])),
            10,
            "half a man each, truncated each time"
        );
        assert_eq!(odds(&with(vec![escort(15, 0, 3), escort(15, 0, 4)])), 12);
        assert_eq!(
            odds(&with(vec![escort(150, 0, 2), escort(150, 0, 1)])),
            10,
            "traders do not count"
        );
        // Their strength counts toward the bonus: 5 x 2 < 0 + 11.
        let strong = CaptureCrew {
            escorts: vec![escort(0, 110, 3)],
            target_strength: 2,
            ..crews(10, 10)
        };
        assert_eq!(odds(&strong), 20);
        let weak = CaptureCrew {
            escorts: vec![escort(0, 109, 3)],
            ..strong.clone()
        };
        assert_eq!(odds(&weak), 10, "10.9, truncated");
        assert_eq!(ESCORT_SHARE, 0.1);
    }

    #[test]
    fn marines_add_crew_and_negative_ones_add_points() {
        let marines = |marines: Vec<(i16, u16)>| CaptureCrew {
            marines,
            ..crews(10, 10)
        };
        assert_eq!(odds(&marines(vec![(3, 2)])), 16, "6 more crew");
        assert_eq!(odds(&marines(vec![(-4, 2)])), 18, "8 more points");
        assert_eq!(odds(&marines(vec![(3, 2), (-4, 1), (0, 9)])), 20);
    }

    #[test]
    fn more_than_five_times_the_targets_strength_adds_10() {
        let strength = |strength| CaptureCrew {
            strength,
            target_strength: 10,
            ..crews(10, 10)
        };
        assert_eq!(odds(&strength(50)), 10, "equal: no");
        assert_eq!(odds(&strength(51)), 20, "one more: +10");
        assert_eq!((STRENGTH_RATIO, STRENGTH_BONUS), (5, 10));
    }

    #[test]
    fn the_odds_jitter_by_5_either_way_and_hold_within_1_and_75() {
        let rule = NovaBoarding::default();
        assert_eq!(odds_with(&rule, &crews(10, 10), 0).0, 15);
        assert_eq!(odds_with(&rule, &crews(10, 10), 10).0, 5);
        assert_eq!(odds_with(&rule, &crews(1, 100), 10).0, 1, "-5, held at 1");
        assert_eq!(odds_with(&rule, &crews(1, 100), 4).0, 1, "1 + 0, at 1");
        assert_eq!(odds_with(&rule, &crews(1, 100), 0).0, 5);
        assert_eq!(odds_with(&rule, &crews(7, 1), 0).0, 75, "75, held");
        assert_eq!(odds_with(&rule, &crews(7, 1), 10).0, 65);
        assert_eq!((MIN_ODDS, MAX_ODDS, JITTER), (1, 75, 5));
    }

    #[test]
    fn a_derelict_ship_or_a_full_fleet_gives_no_odds() {
        let derelict = CaptureCrew {
            derelict: true,
            ..crews(10, 1)
        };
        let (none, asked) = odds_with(&NovaBoarding::default(), &derelict, 5);
        assert_eq!(none, 0);
        assert_eq!(asked, [11], "the jitter is still drawn");
        let full = CaptureCrew {
            fleet_room: false,
            ..crews(10, 1)
        };
        assert_eq!(odds(&full), 0);
    }

    #[test]
    fn a_crewless_player_captures_by_the_engine_and_not_by_the_bible() {
        let engine = NovaBoarding::default();
        assert_eq!(engine.crewless_capture, RuleSource::Engine);
        assert_eq!(odds_with(&engine, &crews(0, 3), 5).0, 1, "held at 1");
        let bible = NovaBoarding {
            crewless_capture: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        assert_eq!(odds_with(&bible, &crews(0, 3), 5).0, 0);
        assert_eq!(odds_with(&bible, &crews(-2, 3), 0).0, 0);
        assert_eq!(
            odds_with(&bible, &crews(2, 3), 5).0,
            6,
            "with a crew, as the engine"
        );
    }

    fn captures(odds: u8, draw: u32) -> (bool, Vec<u32>) {
        let mut chance = Draws::of(&[draw]);
        let captured = NovaBoarding::default().captures(odds, &mut chance);
        (captured, chance.asked)
    }

    #[test]
    fn a_capture_succeeds_on_a_draw_of_100_at_or_below_the_odds_and_never_at_none() {
        assert_eq!(captures(40, 40), (true, vec![100]));
        assert_eq!(captures(40, 0), (true, vec![100]));
        assert_eq!(captures(40, 41), (false, vec![100]));
        assert_eq!(captures(75, 75), (true, vec![100]));
        assert_eq!(captures(1, 1), (true, vec![100]));
        assert_eq!(captures(1, 2), (false, vec![100]));
        // `_DoPlunderDialog` @0x942d5: odds of none fail whatever the
        // draw, which is still made.
        assert_eq!(captures(0, 0), (false, vec![100]));
    }

    #[test]
    fn an_empty_booty_repels_by_the_bible_and_never_by_the_engine() {
        let engine = NovaBoarding::default();
        assert_eq!(engine.empty_booty, RuleSource::Engine);
        assert!(!engine.repels(0));
        assert!(!engine.repels(0x0041));
        let bible = NovaBoarding {
            empty_booty: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        assert!(bible.repels(0));
        assert!(!bible.repels(0x0040));
        assert!(!bible.repels(0x0100));
    }

    #[test]
    fn nova_boarding_follows_its_rulebook_entries() {
        assert_eq!(
            NovaBoarding::from_rulebook(&Rulebook::default()),
            NovaBoarding::default()
        );
        let bible = NovaBoarding::from_rulebook(&Rulebook::new(RuleSource::Bible));
        assert_eq!(
            (
                bible.empty_booty,
                bible.crewless_capture,
                bible.person_credits
            ),
            (RuleSource::Bible, RuleSource::Bible, RuleSource::Bible)
        );
        let credits = Rulebook::default().with_override(RuleKey::PersonCredits, RuleSource::Bible);
        assert_eq!(
            NovaBoarding::from_rulebook(&credits),
            NovaBoarding {
                person_credits: RuleSource::Bible,
                ..NovaBoarding::default()
            }
        );
        let one = Rulebook::default().with_override(RuleKey::EmptyBooty, RuleSource::Bible);
        let rule = NovaBoarding::from_rulebook(&one);
        assert_eq!(
            (rule.empty_booty, rule.crewless_capture),
            (RuleSource::Bible, RuleSource::Engine)
        );
        let other = Rulebook::default().with_override(RuleKey::CrewlessCapture, RuleSource::Bible);
        let rule = NovaBoarding::from_rulebook(&other);
        assert_eq!(
            (rule.empty_booty, rule.crewless_capture),
            (RuleSource::Engine, RuleSource::Bible)
        );
        assert_eq!(
            (bible.grant_count, bible.grant_max),
            (RuleSource::Bible, RuleSource::Bible)
        );
        for (key, expected) in [
            (
                RuleKey::GrantCount,
                NovaBoarding {
                    grant_count: RuleSource::Bible,
                    ..NovaBoarding::default()
                },
            ),
            (
                RuleKey::GrantMax,
                NovaBoarding {
                    grant_max: RuleSource::Bible,
                    ..NovaBoarding::default()
                },
            ),
        ] {
            let one = Rulebook::default().with_override(key, RuleSource::Bible);
            assert_eq!(NovaBoarding::from_rulebook(&one), expected, "{key:?}");
        }
    }

    /// A grant of `count` outfits of class 7 at odds `prob`.
    fn grant_of(prob: u8, count: u16) -> PersonGrant {
        PersonGrant {
            class: 7,
            prob,
            count,
        }
    }

    /// Outfit 200, of class 7 and `Mass` `mass`, `owned` of a `Max` of
    /// `max` owned.
    fn spare(mass: i16, owned: u16, max: i16) -> GrantStock {
        GrantStock {
            outfit: OutfitId(200),
            item_class: 7,
            mass,
            owned,
            max,
        }
    }

    /// What `rule` grants of `grant` from outfit `stock` alone with
    /// `free` mass free, drawn from `draws`, and the bounds asked.
    fn granted(
        rule: NovaBoarding,
        grant: PersonGrant,
        stock: GrantStock,
        free: i64,
        draws: &[u32],
    ) -> (Option<u16>, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let granted = rule.grant(&grant, &[stock], free, &mut chance);
        assert!(granted.is_none_or(|granted| granted.outfit == OutfitId(200)));
        (granted.map(|granted| granted.count), chance.asked)
    }

    /// The count the engine grants of `count`, its `Rand(51)` at `draw`.
    fn engine_count(count: u16, draw: u32) -> Option<u16> {
        granted(
            NovaBoarding::default(),
            grant_of(100, count),
            spare(0, 0, 999),
            100,
            &[0, draw],
        )
        .0
    }

    #[test]
    fn by_the_engine_a_grant_comes_at_its_odds() {
        let rule = NovaBoarding::default();
        assert_eq!(
            granted(rule, grant_of(40, 4), spare(0, 0, 999), 100, &[39, 0]),
            (Some(2), vec![100, 51]),
            "39 + 1 is at most 40; then the count"
        );
        assert_eq!(
            granted(rule, grant_of(40, 4), spare(0, 0, 999), 100, &[40]),
            (None, vec![100]),
            "41 is above 40, and nothing more is drawn"
        );
    }

    #[test]
    fn by_the_engine_a_grant_is_half_its_count_to_all_of_it() {
        assert_eq!(engine_count(4, 0), Some(2));
        assert_eq!(engine_count(4, 50), Some(4));
        assert_eq!(engine_count(4, 25), Some(3));
        assert_eq!(engine_count(1, 0), Some(1), "held at least 1");
        assert_eq!(engine_count(1, 50), Some(1));
        assert_eq!(engine_count(3, 0), Some(1));
        assert_eq!(engine_count(3, 50), Some(3));
        assert_eq!(engine_count(3, 17), Some(2), "trunc(67 x 3 / 100)");
        assert_eq!(
            (COUNT_SPREAD, COUNT_FLOOR_PERCENT, COUNT_PERCENT, GRANT_ROLL),
            (51, 50, 100.0, 100)
        );
    }

    #[test]
    fn by_the_phase_a_grant_is_one_to_its_count_evenly() {
        let rule = NovaBoarding {
            grant_count: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        assert_eq!(
            granted(rule, grant_of(100, 4), spare(0, 0, 999), 100, &[0, 0]),
            (Some(1), vec![100, 4])
        );
        assert_eq!(
            granted(rule, grant_of(100, 4), spare(0, 0, 999), 100, &[0, 3]),
            (Some(4), vec![100, 4])
        );
    }

    #[test]
    fn by_the_engine_a_grant_may_pass_the_outfits_max_and_otherwise_not() {
        let near = spare(0, 9, 10);
        assert_eq!(
            granted(
                NovaBoarding::default(),
                grant_of(100, 3),
                near,
                100,
                &[0, 50]
            )
            .0,
            Some(3)
        );
        let held = NovaBoarding {
            grant_max: RuleSource::Bible,
            ..NovaBoarding::default()
        };
        assert_eq!(
            granted(held, grant_of(100, 3), near, 100, &[0, 50]).0,
            Some(1)
        );
        assert_eq!(
            granted(held, grant_of(100, 3), spare(0, 7, 10), 100, &[0, 50]).0,
            Some(3),
            "room for all three"
        );
    }

    #[test]
    fn a_grant_is_cut_to_the_free_mass() {
        let rule = NovaBoarding::default();
        let all = |free| granted(rule, grant_of(100, 4), spare(5, 0, 999), free, &[0, 50]).0;
        assert_eq!(all(20), Some(4));
        assert_eq!(all(19), Some(3));
        assert_eq!(all(14), Some(2));
        assert_eq!(all(5), Some(1));
        assert_eq!(all(4), None, "none fits: nothing is granted, nor said");
        assert_eq!(
            granted(rule, grant_of(100, 4), spare(0, 0, 999), -30, &[0, 50]).0,
            Some(4),
            "a negative free mass is none, which a massless outfit fits"
        );
        assert_eq!(
            granted(rule, grant_of(100, 4), spare(-3, 0, 999), -30, &[0, 50]).0,
            Some(4),
            "a negative mass always fits"
        );
        assert_eq!(
            granted(rule, grant_of(100, 4), spare(1, 0, 999), -30, &[0, 50]).0,
            None
        );
    }

    #[test]
    fn a_button_is_enabled_while_its_value_is_on_board() {
        let empty = PlunderView {
            npc: NpcId(1),
            ship: ShipId(129),
            credits: 0,
            cargo: None,
            ammo: None,
            fuel: 0,
            odds: 0,
        };
        for take in [
            Take::Cargo,
            Take::Credits,
            Take::Ammo,
            Take::Energy,
            Take::Capture,
        ] {
            assert!(!empty.offers(take), "{take:?}");
        }
        assert!(empty.offers(Take::Abort));
        let full = PlunderView {
            credits: 1,
            cargo: Some((Good::Commodity(0), 1)),
            ammo: Some((OutfitId(200), 1)),
            fuel: 1,
            odds: 1,
            ..empty
        };
        for take in [
            Take::Cargo,
            Take::Credits,
            Take::Ammo,
            Take::Energy,
            Take::Capture,
            Take::Abort,
        ] {
            assert!(full.offers(take), "{take:?}");
        }
    }

    #[test]
    fn misaligned_and_no_target_are_silent() {
        assert!(BoardRefusal::NoTarget.silent());
        assert!(BoardRefusal::Misaligned.silent());
        for loud in [
            BoardRefusal::CantBoard,
            BoardRefusal::TooFast,
            BoardRefusal::TooFar,
        ] {
            assert!(!loud.silent(), "{loud:?}");
        }
    }
}
