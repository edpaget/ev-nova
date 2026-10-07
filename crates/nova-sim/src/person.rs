//! Persons (`përs`): named characters who take an ordinary ship's place,
//! with their own ship, loadout, government, AI, comm and hail quotes.
//!
//! The rules are the original's (`_LoadObjectData`, `_SpawnPerson`
//! @0x408d5, `_SetupShipsInSystem`, `_HyperShipSpawn`, `_WarshipAI`,
//! `_SelectWarshipTarget`, `_DamageShip`, `_HandleShip`,
//! `_HandleShipDisplay`, `_DoShipCapture`, `_SetPlunderValues`,
//! `_LoadAdvice` and `_ShowPersonHailQuote` in the `EV Nova`
//! executable). Each disputed one follows its rulebook key; the engine's
//! reading is the default.
//!
//! **Where a person may appear** (`_SpawnPerson` @0x40990-0x40ba4), in a
//! system S, all of these holding:
//!
//! 1. it is alive: not gone for good ([`Pilot::gone`](crate::Pilot::gone));
//! 2. its `AIType` is above 0;
//! 3. its `ActiveOn` holds, through the [`ControlBits`] port, which lets
//!    every expression hold until control bits exist;
//! 4. its `LinkSyst` allows S ([`PersonLink`]): -1 any system; 128-9998
//!    that system; 9999-14999 a system governed by government
//!    `LinkSyst` - 10000 + 128, so 9999 means the independent systems
//!    (a fleet's government range starts at 10000); 15000-19999 a
//!    governed system allied with `LinkSyst` - 15000 + 128, a government
//!    counting as its own ally; 20000-24999 a governed system not of
//!    `LinkSyst` - 20000 + 128; 25000-29999 a governed system at war with
//!    `LinkSyst` - 25000 + 128; anything else none. The engine also
//!    compares the raw value with S's index (ID - 128, @0x409db), so a
//!    `LinkSyst` of 0-2047 also allows the system of ID `LinkSyst` + 128:
//!    the [`RuleKey::LinkSystSlip`] entry chooses;
//! 5. arriving by hyperspace, its government is not derelict (`gövt`
//!    `Flags` 0x0800): drifting derelicts never jump in;
//! 6. no person of its name is in the system already, in any condition
//!    (@0x40baa-0x40c48; the original skips mission ships, of which there
//!    are none yet).
//!
//! A person whose ship type has no record never appears (the original
//! flies ship 128 for a `ShipType` out of range).
//!
//! **How often** ([`PersonRules::roll`], [`RuleKey::PersonOdds`]): each
//! setup pass and each hyperspace arrival first rolls for a person
//! ([`PersonRoll`]). By the engine, `Rand(7)` not 0 is none; 0 with no
//! person who may appear is empty, with no further draw; otherwise
//! `Rand(1022)` = r and the person of ID 128 + r appears when it may, or
//! else the roll is empty. So each person has a fixed 1-in-1022 share, and
//! persons 1150 and 1151 are never drawn at random. By the Bible ("a 5%
//! chance that a specific AI-person will also be created"), `Rand(100)`
//! below 5 fires and one of those who may appear, in ascending ID, is
//! picked by `Rand(k)` (drawn only when k is 2 or more); with none, the
//! roll is none. A person takes the ship's place, keeping `AvgShips`.
//!
//! After setup's passes each of the system's eight Person slots is tried
//! in turn (`_SetupShipsInSystem` @0x43310-0x4344b): a slot naming a
//! person alive, whose ship has a record and whose `ActiveOn` holds is
//! listed by [`PersonRules::listed`] ([`RuleKey::SystemPersons`]: by the
//! engine `Rand(100) + 1` at most the slot's chance, drawn even at 0 %;
//! by the Bible always), and then appears unless one of its name is
//! there. Its `LinkSyst` and `AIType` are not tested.
//!
//! **The ship it flies** ([`fit`], `_SpawnPerson` @0x40cd4-0x41108): its
//! `Govt`, its `AIType`, and `Aggress` 0 or below as 1, 1 and 2 as they
//! are, and 3 or above as 4 ([`aggression_of`]); no `düde`, so no booty
//! and no `InfoTypes`. Each weapon slot naming a weapon with a record
//! adds its `WeapCount` to the ship's count of it (a later slot naming
//! the same weapon replaces the earlier), a count of none or less
//! removing it, and its `AmmoLoad` to the rounds of its ammunition, never
//! below none. A `ShieldMod` above 0 scales the shield and armour
//! capacity by `ShieldMod` / 100 ([`RuleKey::ShieldMod`]: the Bible
//! scales the shield only); below 0 the person is invincible, its shield
//! and armour refilled every fight tick. `Flags2` 0x0001 starts it with
//! no fuel. A person of a derelict government starts disabled: shield 0
//! and armour its ship type's times 0.33 (0.1 for a tough hull) less 1.
//! Placed at setup it is placed as any ship, and arriving it jumps in as
//! any ship, but it draws no aggression.
//!
//! The values are defaults, not a contract.

use std::collections::BTreeSet;
use std::fmt::Debug;

use crate::catalog::{GovtId, PersonId, PersonRecord, SystemId, WeaponId};
use crate::chance::Chance;
use crate::combat::armament::{Armament, Arsenal};
use crate::combat::hull::Condition;
use crate::combat::weapon::{Ammo, WeaponSpec};
use crate::govt::Governments;
use crate::hire::{ControlBits, NoControlBits};
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};
use crate::traffic::table::ShipKind;

/// `Flags`: it holds a grudge once the player hits it.
pub const GRUDGE: u16 = 0x0001;
/// `Flags`: it uses an escape pod, so it survives its ship (@0x2dc9b).
pub const ESCAPE_POD: u16 = 0x0002;
/// `Flags`: its hail quote only while it holds a grudge.
pub const QUOTE_IF_GRUDGE: u16 = 0x0004;
/// `Flags`: its hail quote only while it likes the player.
pub const QUOTE_IF_LIKED: u16 = 0x0008;
/// `Flags`: its hail quote when it begins to attack the player.
pub const QUOTE_ON_ATTACK: u16 = 0x0010;
/// `Flags`: its hail quote only while it is disabled.
pub const QUOTE_IF_DISABLED: u16 = 0x0020;
/// `Flags`: its hail quote only once a stay.
pub const QUOTE_ONCE: u16 = 0x0080;
/// `Flags`: its hail quote only while its mission is available.
pub const QUOTE_IF_MISSION: u16 = 0x0400;
/// `Flags`: no hail quote while it is leaving.
pub const QUOTE_UNLESS_LEAVING: u16 = 0x0800;
/// `Flags`: no hail quote to a player whose ship's `InherentAI` is 1.
pub const NOT_TO_WIMPY: u16 = 0x1000;
/// `Flags`: no hail quote to a player whose ship's `InherentAI` is 2.
pub const NOT_TO_BRAVE: u16 = 0x2000;
/// `Flags`: no hail quote to a player whose ship's `InherentAI` is 3 or
/// more.
pub const NOT_TO_WARSHIP: u16 = 0x4000;
/// `Flags2`: it starts with no fuel (@0x40f1f-0x40f55).
pub const ZERO_FUEL: u16 = 0x0001;

/// `Rand(1022)` (0x3fe @0x40c8e): the person a roll lands on.
pub const PERSON_DRAW: u32 = 1022;
/// The ID of the first person.
pub const FIRST_PERSON: i16 = 128;
/// The Bible's 5 % person roll.
pub const BIBLE_PERSON_PERCENT: u32 = 5;
/// `Rand(100)`: a percentage roll.
pub const PERCENT: u32 = 100;
/// What a `ShieldMod` is divided by (100.0 @0xdd060).
pub const SHIELD_MOD_PERCENT: f32 = 100.0;
/// A derelict person's armour, as a share of its ship type's (0.33
/// @0xdd568).
pub const DERELICT_ARMOR_SHARE: f32 = 0.33;
/// A derelict person's armour on a tough hull (0.1 @0xdd110).
pub const TOUGH_DERELICT_ARMOR_SHARE: f32 = 0.1;
/// What a derelict person's armour is less (1.0 @0xdd0c0).
pub const DERELICT_ARMOR_LESS: f32 = 1.0;

/// The highest `LinkSyst` the engine's slip compares with a system's
/// index: the last system's.
pub const LAST_SLIP: i16 = 2047;

/// Which systems a person's `LinkSyst` allows, as the Bible reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linked {
    /// -1: any system.
    Any,
    /// 128-9998: this system.
    System(SystemId),
    /// 9999-14999: a system governed by this government; none means the
    /// independent systems.
    Govt(Option<GovtId>),
    /// 15000-19999: a governed system allied with this government.
    AlliesOf(GovtId),
    /// 20000-24999: a governed system not of this government.
    NotGovt(GovtId),
    /// 25000-29999: a governed system at war with this government.
    EnemiesOf(GovtId),
    /// Anything else: none.
    Never,
}

/// A person's `LinkSyst`, decoded (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonLink {
    /// What it allows by its range.
    pub linked: Linked,
    /// The system the engine's slip also allows, for a `LinkSyst` of
    /// 0-2047: the one of ID `LinkSyst` + 128.
    pub slip: Option<SystemId>,
}

impl PersonLink {
    /// The link a raw `LinkSyst` gives.
    #[must_use]
    pub fn decode(raw: i16) -> Self {
        let govt = |base: i16| GovtId(raw - base + FIRST_PERSON);
        let linked = match raw {
            -1 => Linked::Any,
            128..=9998 => Linked::System(SystemId(raw)),
            9999 => Linked::Govt(None),
            10_000..=14_999 => Linked::Govt(Some(govt(10_000))),
            15_000..=19_999 => Linked::AlliesOf(govt(15_000)),
            20_000..=24_999 => Linked::NotGovt(govt(20_000)),
            25_000..=29_999 => Linked::EnemiesOf(govt(25_000)),
            _ => Linked::Never,
        };
        let slip = (0..=LAST_SLIP)
            .contains(&raw)
            .then(|| SystemId(raw + FIRST_PERSON));
        Self { linked, slip }
    }

    /// Whether system `system`, governed by `system_govt`, is allowed,
    /// with the relations in `govts`, and the engine's slip as `slip`
    /// says.
    #[must_use]
    pub fn matches(
        self,
        system: SystemId,
        system_govt: Option<GovtId>,
        govts: &Governments,
        slip: RuleSource,
    ) -> bool {
        let governed = system_govt.is_some();
        let linked = match self.linked {
            Linked::Any => true,
            Linked::System(id) => id == system,
            Linked::Govt(govt) => system_govt == govt,
            Linked::AlliesOf(govt) => governed && govts.allies(Some(govt), system_govt),
            Linked::NotGovt(govt) => system_govt.is_some_and(|own| own != govt),
            Linked::EnemiesOf(govt) => governed && govts.enemies(Some(govt), system_govt),
            Linked::Never => false,
        };
        linked || (slip == RuleSource::Engine && self.slip == Some(system))
    }
}

/// A person's aggression from its `Aggress` (@0x40daf-0x40dd1): 0 or
/// below is 1, 1 and 2 as they are, 3 or above 4.
#[must_use]
pub fn aggression_of(aggress: i16) -> u8 {
    match aggress {
        i16::MIN..=1 => 1,
        2 => 2,
        _ => 4,
    }
}

/// A ship of `kind` as `person` flies it (see the module docs): its
/// weapons and rounds with the person's slots, read from `arsenal`; its
/// shield, and armour by the engine, scaled as `shield_mod` says; its
/// starting reserves; and its condition, disabled when `derelict`.
#[must_use]
pub fn fit(
    kind: &ShipKind,
    person: &PersonRecord,
    arsenal: &Arsenal,
    shield_mod: RuleSource,
    derelict: bool,
) -> (ShipKind, Reserves, Condition) {
    let mut slots: Vec<(WeaponId, i16, i16)> = Vec::new();
    for slot in &person.weapons {
        let entry = (slot.weapon, slot.count, slot.ammo);
        match slots.iter_mut().find(|(weapon, ..)| *weapon == slot.weapon) {
            Some(earlier) => *earlier = entry,
            None => slots.push(entry),
        }
    }
    let mut counts: Vec<(WeaponSpec, i32)> = kind
        .armament
        .mounts()
        .iter()
        .map(|mount| (mount.spec, i32::try_from(mount.count).unwrap_or(i32::MAX)))
        .collect();
    let mut rounds = kind.rounds.clone();
    for (weapon, count, ammo) in slots {
        let Some(spec) = arsenal.weapon(weapon) else {
            continue;
        };
        match counts.iter_mut().find(|(carried, _)| carried.id == weapon) {
            Some((_, carried)) => *carried += i32::from(count),
            None => counts.push((*spec, i32::from(count))),
        }
        if let Ammo::Rounds(of) = spec.ammo {
            let held = i64::from(rounds.get(&of).copied().unwrap_or(0)) + i64::from(ammo);
            match u32::try_from(held) {
                Ok(held) if held > 0 => {
                    rounds.insert(of, held);
                }
                _ => {
                    rounds.remove(&of);
                }
            }
        }
    }
    let armament = Armament::new(
        counts
            .into_iter()
            .filter_map(|(spec, count)| Some((spec, u32::try_from(count).ok()?))),
    );
    let mut stats = kind.stats;
    if person.shield_mod > 0 {
        let scale = f32::from(person.shield_mod) / SHIELD_MOD_PERCENT;
        stats.shield *= scale;
        if shield_mod == RuleSource::Engine {
            stats.armor *= scale;
        }
    }
    let mut reserves = stats.full();
    if person.flags2 & ZERO_FUEL != 0 {
        reserves.fuel.now = 0.0;
    }
    let condition = if derelict {
        let share = if kind.hull.tough {
            TOUGH_DERELICT_ARMOR_SHARE
        } else {
            DERELICT_ARMOR_SHARE
        };
        reserves.shield.now = 0.0;
        reserves.armor.now = kind.stats.armor.mul_add(share, -DERELICT_ARMOR_LESS);
        Condition::Disabled
    } else {
        Condition::Intact
    };
    let fitted = ShipKind {
        stats,
        armament,
        rounds,
        ..kind.clone()
    };
    (fitted, reserves, condition)
}

/// What a person roll gave (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonRoll {
    /// No person: the pass or arrival goes on to its fleet and `düde`
    /// rolls.
    None,
    /// A person was due but none came: the pass or arrival brings
    /// nothing.
    Empty,
    /// This person appears.
    Person(PersonId),
}

/// The persons' rules that a mod might replace: how often a person
/// appears, whether a Person slot brings its person, and which readings
/// of the `LinkSyst` slip and `ShieldMod` apply. Nova's is
/// [`NovaPersons`].
pub trait PersonRules: Debug {
    /// The person roll among the persons who may appear, `eligible` in
    /// ascending ID, rolled on `chance`.
    fn roll(&self, eligible: &[PersonId], chance: &mut dyn Chance) -> PersonRoll;
    /// Whether a Person slot of chance `prob` brings its person, rolled on
    /// `chance`.
    fn listed(&self, prob: i16, chance: &mut dyn Chance) -> bool;
    /// Which reading of a system `LinkSyst` applies
    /// ([`RuleKey::LinkSystSlip`]).
    fn link_slip(&self) -> RuleSource;
    /// What `ShieldMod` scales ([`RuleKey::ShieldMod`]).
    fn shield_mod(&self) -> RuleSource;
}

/// Nova's persons' rules (see the module docs), the engine's by default.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NovaPersons {
    /// How often a person appears ([`RuleKey::PersonOdds`]).
    pub odds: RuleSource,
    /// Whether a Person slot rolls ([`RuleKey::SystemPersons`]).
    pub slots: RuleSource,
    /// The `LinkSyst` slip ([`RuleKey::LinkSystSlip`]).
    pub link: RuleSource,
    /// What `ShieldMod` scales ([`RuleKey::ShieldMod`]).
    pub shield_mod: RuleSource,
}

impl NovaPersons {
    /// The engine's rules, for a place that needs them as a constant.
    pub const ENGINE: Self = Self {
        odds: RuleSource::Engine,
        slots: RuleSource::Engine,
        link: RuleSource::Engine,
        shield_mod: RuleSource::Engine,
    };

    /// The rules `rulebook` chooses: its [`RuleKey::PersonOdds`],
    /// [`RuleKey::SystemPersons`], [`RuleKey::LinkSystSlip`] and
    /// [`RuleKey::ShieldMod`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            odds: rulebook.source_for(RuleKey::PersonOdds),
            slots: rulebook.source_for(RuleKey::SystemPersons),
            link: rulebook.source_for(RuleKey::LinkSystSlip),
            shield_mod: rulebook.source_for(RuleKey::ShieldMod),
        }
    }
}

impl PersonRules for NovaPersons {
    fn roll(&self, eligible: &[PersonId], chance: &mut dyn Chance) -> PersonRoll {
        match self.odds {
            RuleSource::Engine => {
                if chance.below(crate::traffic::spawn::PERSON_ODDS) != 0 {
                    return PersonRoll::None;
                }
                if eligible.is_empty() {
                    return PersonRoll::Empty;
                }
                let drawn = chance.below(PERSON_DRAW) as i16;
                let id = PersonId(FIRST_PERSON + drawn);
                if eligible.contains(&id) {
                    PersonRoll::Person(id)
                } else {
                    PersonRoll::Empty
                }
            }
            RuleSource::Bible => {
                if chance.below(PERCENT) >= BIBLE_PERSON_PERCENT {
                    return PersonRoll::None;
                }
                let picked = match eligible {
                    [] => return PersonRoll::None,
                    [only] => *only,
                    many => many[chance.below(many.len() as u32) as usize],
                };
                PersonRoll::Person(picked)
            }
        }
    }

    fn listed(&self, prob: i16, chance: &mut dyn Chance) -> bool {
        match self.slots {
            RuleSource::Engine => chance.below(PERCENT) < u32::try_from(prob).unwrap_or(0),
            RuleSource::Bible => true,
        }
    }

    fn link_slip(&self) -> RuleSource {
        self.link
    }

    fn shield_mod(&self) -> RuleSource {
        self.shield_mod
    }
}

/// No person gone, and no grudge.
static NO_PERSONS: BTreeSet<PersonId> = BTreeSet::new();

/// What the persons' spawning sees of the game: the rules, the persons
/// gone for good and those holding a grudge, and the control bits a
/// person's `ActiveOn` is tested through.
#[derive(Clone, Copy, Debug)]
pub struct PersonWorld<'a> {
    /// The persons' rules.
    pub rules: &'a dyn PersonRules,
    /// The persons gone for good.
    pub gone: &'a BTreeSet<PersonId>,
    /// The persons holding a grudge against the player.
    pub grudges: &'a BTreeSet<PersonId>,
    /// The control-bit test of a person's `ActiveOn`.
    pub control_bits: &'a dyn ControlBits,
}

impl PersonWorld<'static> {
    /// Nova's rules by the engine, no person gone, no grudge, and every
    /// `ActiveOn` holding.
    pub const NONE: Self = Self {
        rules: &NovaPersons::ENGINE,
        gone: &NO_PERSONS,
        grudges: &NO_PERSONS,
        control_bits: &NoControlBits,
    };
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::catalog::{GovtRecord, PersonWeapon, WeaponRecord};
    use crate::combat::hull::HullSpec;
    use crate::stats::ShipStats;
    use crate::testkit::{Draws, FAST, govt, person, weapon};

    /// Governments 128 and 129 are allies (class 1 and its ally); 130 is
    /// at war with 128 (class 3, listing class 1).
    fn relations() -> Governments {
        Governments::new([
            GovtRecord {
                classes: [1, -1, -1, -1],
                ..govt(128)
            },
            GovtRecord {
                allies: [1, -1, -1, -1],
                ..govt(129)
            },
            GovtRecord {
                classes: [3, -1, -1, -1],
                enemies: [1, -1, -1, -1],
                ..govt(130)
            },
        ])
    }

    /// The systems of the fake star, 128-260, that `raw` allows by
    /// `slip`, each governed by `govt_of`.
    fn allowed(raw: i16, slip: RuleSource, govt_of: impl Fn(i16) -> Option<i16>) -> Vec<i16> {
        let govts = relations();
        (128..=260)
            .filter(|&id| {
                PersonLink::decode(raw).matches(SystemId(id), govt_of(id).map(GovtId), &govts, slip)
            })
            .collect()
    }

    /// Systems 128-130 are governed by 128-130, 131 by none, the rest by
    /// 128.
    fn governed(id: i16) -> Option<i16> {
        match id {
            128..=130 => Some(id),
            131 => None,
            _ => Some(128),
        }
    }

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;

    #[test]
    fn any_link_allows_every_system() {
        assert_eq!(allowed(-1, ENGINE, governed).len(), 133);
        assert_eq!(allowed(-1, BIBLE, governed).len(), 133);
    }

    #[test]
    fn a_system_link_allows_its_system_and_by_the_engines_slip_another() {
        assert_eq!(allowed(132, ENGINE, governed), [132, 260]);
        assert_eq!(allowed(132, BIBLE, governed), [132]);
        assert_eq!(allowed(50, ENGINE, governed), [178]);
        assert_eq!(allowed(50, BIBLE, governed), Vec::<i16>::new());
        assert_eq!(allowed(0, ENGINE, governed), [128]);
        assert_eq!(PersonLink::decode(2047).slip, Some(SystemId(2175)));
        assert_eq!(PersonLink::decode(2048).slip, None);
        assert_eq!(PersonLink::decode(-1).slip, None);
        assert_eq!(LAST_SLIP, 2047);
    }

    #[test]
    fn a_government_link_allows_its_systems_and_9999_the_independent_ones() {
        let one = |id: i16| (id == 131).then_some(129).or(Some(128));
        let independent = |id: i16| (id != 131).then_some(128);
        assert_eq!(allowed(9999, ENGINE, independent), [131]);
        assert_eq!(allowed(10_000, ENGINE, governed), {
            let mut ids = vec![128];
            ids.extend(132..=260);
            ids
        });
        assert_eq!(allowed(10_001, ENGINE, one), [131]);
        assert_eq!(allowed(10_000, ENGINE, |_| None), Vec::<i16>::new());
    }

    /// Systems 128-131 only: 128 to 130 governed by their own number, 131
    /// independent.
    fn four(raw: i16) -> Vec<i16> {
        allowed(raw, BIBLE, governed)
            .into_iter()
            .filter(|&id| id <= 131)
            .collect()
    }

    #[test]
    fn the_relation_links_allow_their_governed_systems_only() {
        assert_eq!(four(15_000), [128, 129], "its own and its ally's");
        assert_eq!(four(20_000), [129, 130], "never an independent one");
        assert_eq!(four(25_000), [130], "its enemy's");
        assert_eq!(four(30_000), Vec::<i16>::new());
        assert_eq!(four(-2), Vec::<i16>::new());
        assert_eq!(four(127), Vec::<i16>::new());
    }

    #[test]
    fn link_syst_decodes_at_each_ranges_edges() {
        for (raw, linked) in [
            (-1, Linked::Any),
            (128, Linked::System(SystemId(128))),
            (9998, Linked::System(SystemId(9998))),
            (9999, Linked::Govt(None)),
            (10_000, Linked::Govt(Some(GovtId(128)))),
            (14_999, Linked::Govt(Some(GovtId(5127)))),
            (15_000, Linked::AlliesOf(GovtId(128))),
            (19_999, Linked::AlliesOf(GovtId(5127))),
            (20_000, Linked::NotGovt(GovtId(128))),
            (24_999, Linked::NotGovt(GovtId(5127))),
            (25_000, Linked::EnemiesOf(GovtId(128))),
            (29_999, Linked::EnemiesOf(GovtId(5127))),
            (30_000, Linked::Never),
            (127, Linked::Never),
            (-2, Linked::Never),
        ] {
            assert_eq!(PersonLink::decode(raw).linked, linked, "{raw}");
        }
    }

    // The roll.

    fn roll(rules: NovaPersons, eligible: &[i16], draws: &[u32]) -> (PersonRoll, Vec<u32>) {
        let eligible: Vec<PersonId> = eligible.iter().copied().map(PersonId).collect();
        let mut chance = Draws::of(draws);
        let rolled = rules.roll(&eligible, &mut chance);
        (rolled, chance.asked)
    }

    fn bible() -> NovaPersons {
        NovaPersons::from_rulebook(&Rulebook::new(RuleSource::Bible))
    }

    #[test]
    fn by_the_engine_a_person_comes_on_0_in_7_then_its_draw_of_1022() {
        let engine = NovaPersons::default();
        assert_eq!(roll(engine, &[510], &[1]), (PersonRoll::None, vec![7]));
        assert_eq!(roll(engine, &[], &[0]), (PersonRoll::Empty, vec![7]));
        assert_eq!(
            roll(engine, &[510], &[0, 382]),
            (PersonRoll::Person(PersonId(510)), vec![7, 1022])
        );
        assert_eq!(
            roll(engine, &[510], &[0, 381]),
            (PersonRoll::Empty, vec![7, 1022])
        );
        assert_eq!(
            roll(engine, &[128, 510], &[0, 0]).0,
            PersonRoll::Person(PersonId(128))
        );
        assert_eq!([PERSON_DRAW, FIRST_PERSON as u32], [1022, 128]);
    }

    #[test]
    fn by_the_bible_a_person_comes_5_times_in_100_picked_evenly() {
        assert_eq!(roll(bible(), &[510], &[5]), (PersonRoll::None, vec![100]));
        assert_eq!(
            roll(bible(), &[510], &[4]),
            (PersonRoll::Person(PersonId(510)), vec![100])
        );
        assert_eq!(
            roll(bible(), &[131, 510], &[4, 1]),
            (PersonRoll::Person(PersonId(510)), vec![100, 2])
        );
        assert_eq!(
            roll(bible(), &[131, 510], &[0, 0]).0,
            PersonRoll::Person(PersonId(131))
        );
        assert_eq!(roll(bible(), &[], &[4]), (PersonRoll::None, vec![100]));
        assert_eq!(BIBLE_PERSON_PERCENT, 5);
    }

    fn listed(rules: NovaPersons, prob: i16, draws: &[u32]) -> (bool, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let listed = rules.listed(prob, &mut chance);
        (listed, chance.asked)
    }

    #[test]
    fn by_the_engine_a_slot_lists_its_person_on_a_percentage_roll_and_by_the_bible_always() {
        let engine = NovaPersons::default();
        assert_eq!(listed(engine, 50, &[49]), (true, vec![100]));
        assert_eq!(listed(engine, 50, &[50]), (false, vec![100]));
        assert_eq!(listed(engine, 0, &[0]), (false, vec![100]), "drawn at 0 %");
        assert_eq!(listed(engine, 100, &[99]), (true, vec![100]));
        assert_eq!(listed(bible(), 1, &[]), (true, vec![]), "no draw");
        assert_eq!(listed(bible(), 0, &[]), (true, vec![]));
    }

    #[test]
    fn the_rules_follow_their_four_rulebook_keys() {
        assert_eq!(NovaPersons::default(), NovaPersons::ENGINE);
        assert_eq!(
            NovaPersons::from_rulebook(&Rulebook::default()),
            NovaPersons::ENGINE
        );
        for (key, expected) in [
            (
                RuleKey::PersonOdds,
                NovaPersons {
                    odds: BIBLE,
                    ..NovaPersons::ENGINE
                },
            ),
            (
                RuleKey::SystemPersons,
                NovaPersons {
                    slots: BIBLE,
                    ..NovaPersons::ENGINE
                },
            ),
            (
                RuleKey::LinkSystSlip,
                NovaPersons {
                    link: BIBLE,
                    ..NovaPersons::ENGINE
                },
            ),
            (
                RuleKey::ShieldMod,
                NovaPersons {
                    shield_mod: BIBLE,
                    ..NovaPersons::ENGINE
                },
            ),
        ] {
            let rulebook = Rulebook::default().with_override(key, BIBLE);
            assert_eq!(NovaPersons::from_rulebook(&rulebook), expected, "{key:?}");
        }
        let bible = bible();
        assert_eq!((bible.link_slip(), bible.shield_mod()), (BIBLE, BIBLE));
        let engine = NovaPersons::ENGINE;
        assert_eq!((engine.link_slip(), engine.shield_mod()), (ENGINE, ENGINE));
    }

    #[test]
    fn the_world_without_persons_follows_the_engine_and_lets_every_person_be() {
        let none = PersonWorld::NONE;
        assert!(none.gone.is_empty() && none.grudges.is_empty());
        assert!(none.control_bits.allows("b0 & !b8"));
        let mut chance = Draws::of(&[0]);
        assert_eq!(none.rules.roll(&[], &mut chance), PersonRoll::Empty);
        assert_eq!(none.rules.link_slip(), ENGINE);
    }

    // The ship.

    #[test]
    fn a_persons_aggression_is_1_2_or_4() {
        for (aggress, aggression) in [
            (i16::MIN, 1),
            (-1, 1),
            (0, 1),
            (1, 1),
            (2, 2),
            (3, 4),
            (4, 4),
            (9, 4),
        ] {
            assert_eq!(aggression_of(aggress), aggression, "{aggress}");
        }
    }

    /// Weapon A (128); launcher L (129) firing rounds of R (130); C (131).
    fn arsenal() -> Arsenal {
        Arsenal::new(
            &[
                weapon(128),
                WeaponRecord {
                    ammo_type: 2,
                    ..weapon(129)
                },
                weapon(130),
                weapon(131),
            ],
            Vec::new(),
        )
    }

    /// A ship carrying weapon A twice and launcher L once, with 10 rounds
    /// of R.
    fn armed() -> ShipKind {
        let arsenal = arsenal();
        let spec = |id: i16| *arsenal.weapon(WeaponId(id)).expect("armed");
        ShipKind {
            stats: ShipStats::new(FAST, &[]),
            armament: Armament::new([(spec(128), 2), (spec(129), 1)]),
            rounds: BTreeMap::from([(WeaponId(130), 10)]),
            ..ShipKind::default()
        }
    }

    fn slot(weapon: i16, count: i16, ammo: i16) -> PersonWeapon {
        PersonWeapon {
            weapon: WeaponId(weapon),
            count,
            ammo,
        }
    }

    /// `kind` fitted for a person with `slots`, by the engine.
    fn fitted(kind: &ShipKind, slots: &[PersonWeapon]) -> ShipKind {
        let person = PersonRecord {
            weapons: slots.to_vec(),
            ..person(128, 128)
        };
        fit(kind, &person, &arsenal(), ENGINE, false).0
    }

    fn counts(kind: &ShipKind) -> Vec<(i16, u32)> {
        kind.armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn a_persons_weapon_slots_add_to_the_ships_counts() {
        let kind = armed();
        assert_eq!(counts(&fitted(&kind, &[])), [(128, 2), (129, 1)]);
        assert_eq!(
            counts(&fitted(&kind, &[slot(128, 2, 0)])),
            [(128, 4), (129, 1)]
        );
        assert_eq!(
            counts(&fitted(&kind, &[slot(128, -3, 0)])),
            [(129, 1)],
            "none left"
        );
        assert_eq!(counts(&fitted(&kind, &[slot(128, -2, 0)])), [(129, 1)]);
        assert_eq!(
            counts(&fitted(&kind, &[slot(131, 1, 0)])),
            [(128, 2), (129, 1), (131, 1)],
            "one not carried is added"
        );
        assert_eq!(
            counts(&fitted(&kind, &[slot(999, 3, 3)])),
            [(128, 2), (129, 1)],
            "no record: ignored"
        );
        assert_eq!(
            counts(&fitted(&kind, &[slot(128, 5, 0), slot(128, 1, 0)])),
            [(128, 3), (129, 1)],
            "a later slot for the same weapon replaces the earlier"
        );
    }

    #[test]
    fn a_persons_ammunition_adds_to_the_rounds_never_below_none() {
        let kind = armed();
        let rounds = |slots: &[PersonWeapon]| fitted(&kind, slots).rounds;
        assert_eq!(
            rounds(&[slot(129, 1, 50)]),
            BTreeMap::from([(WeaponId(130), 60)])
        );
        assert_eq!(rounds(&[slot(129, 0, -10)]), BTreeMap::new());
        let bare = ShipKind {
            rounds: BTreeMap::new(),
            ..armed()
        };
        let person = PersonRecord {
            weapons: vec![slot(129, 0, -1)],
            ..person(128, 128)
        };
        let (kind, ..) = fit(&bare, &person, &arsenal(), ENGINE, false);
        assert_eq!(kind.rounds, BTreeMap::new(), "none, not negative");
        assert_eq!(
            rounds(&[slot(128, 1, 5)]),
            BTreeMap::from([(WeaponId(130), 10)]),
            "a weapon firing no rounds holds none"
        );
    }

    fn shielded(shield_mod: i16, rule: RuleSource) -> (ShipKind, Reserves) {
        let person = PersonRecord {
            shield_mod,
            ..person(128, 128)
        };
        let (kind, reserves, condition) = fit(&armed(), &person, &arsenal(), rule, false);
        assert_eq!(condition, Condition::Intact);
        (kind, reserves)
    }

    #[test]
    fn shield_mod_scales_the_shield_and_by_the_engine_the_armour() {
        let (kind, reserves) = shielded(250, ENGINE);
        assert_eq!((kind.stats.shield, kind.stats.armor), (75.0, 112.5));
        assert_eq!(reserves, kind.stats.full(), "full");
        let (kind, reserves) = shielded(250, BIBLE);
        assert_eq!((kind.stats.shield, kind.stats.armor), (75.0, 45.0));
        assert_eq!(reserves, kind.stats.full());
        for unscaled in [0, -1] {
            let (kind, _) = shielded(unscaled, ENGINE);
            assert_eq!(kind.stats, ShipStats::new(FAST, &[]), "{unscaled}");
        }
        assert_eq!(shielded(50, ENGINE).0.stats.shield, 15.0);
    }

    #[test]
    fn a_person_of_zero_fuel_starts_with_none_of_a_full_tanks_capacity() {
        let empty = PersonRecord {
            flags2: ZERO_FUEL,
            ..person(128, 128)
        };
        let (kind, reserves, _) = fit(&armed(), &empty, &arsenal(), ENGINE, false);
        assert_eq!(reserves.fuel.now, 0.0);
        assert_eq!(reserves.fuel.max, kind.stats.fuel);
        assert_eq!(reserves.shield, kind.stats.full().shield);
        let (_, reserves, _) = fit(&armed(), &person(128, 128), &arsenal(), ENGINE, false);
        assert_eq!(reserves.fuel.now, 300.0, "without the flag, full");
        assert_eq!(ZERO_FUEL, 0x0001);
    }

    #[test]
    fn a_derelict_person_starts_disabled_with_no_shield_and_a_third_of_its_armour() {
        let (kind, reserves, condition) =
            fit(&armed(), &person(128, 128), &arsenal(), ENGINE, true);
        assert_eq!(condition, Condition::Disabled);
        assert_eq!(reserves.shield.now, 0.0);
        assert!((reserves.armor.now - (45.0 * 0.33 - 1.0)).abs() < 1e-4);
        assert_eq!(reserves.armor.max, kind.stats.armor);
        let tough = ShipKind {
            hull: HullSpec {
                tough: true,
                ..HullSpec::default()
            },
            ..armed()
        };
        let (_, reserves, _) = fit(&tough, &person(128, 128), &arsenal(), ENGINE, true);
        assert!((reserves.armor.now - (45.0 * 0.1 - 1.0)).abs() < 1e-4);
        assert_eq!(
            [
                DERELICT_ARMOR_SHARE,
                TOUGH_DERELICT_ARMOR_SHARE,
                DERELICT_ARMOR_LESS
            ],
            [0.33, 0.1, 1.0]
        );
    }

    #[test]
    fn the_flag_bits_are_the_bibles() {
        assert_eq!(
            [
                GRUDGE,
                ESCAPE_POD,
                QUOTE_IF_GRUDGE,
                QUOTE_IF_LIKED,
                QUOTE_ON_ATTACK,
                QUOTE_IF_DISABLED,
                QUOTE_ONCE,
                QUOTE_IF_MISSION,
                QUOTE_UNLESS_LEAVING,
                NOT_TO_WIMPY,
                NOT_TO_BRAVE,
                NOT_TO_WARSHIP
            ],
            [
                0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0080, 0x0400, 0x0800, 0x1000,
                0x2000, 0x4000
            ]
        );
    }
}
