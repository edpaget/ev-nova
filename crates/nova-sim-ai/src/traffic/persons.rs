//! The persons' model the traffic spawns them by: where a person may
//! appear ([`PersonLink`], [`PersonWorld`]), how often ([`PersonRules`],
//! [`PersonRoll`]), the ship it flies ([`fit`], [`aggression_of`],
//! [`derelict_armor`]), and whether it says its hail quote as it turns on
//! the player ([`turns_with_quote`]). Nova's rules are [`NovaPersons`].
//!
//! The rules are written out in full in `nova_sim::person`'s module docs,
//! which "the module docs" below means; `nova_sim::person` re-exports
//! every item here.

use std::collections::BTreeSet;
use std::fmt::Debug;

use crate::catalog::{GovtId, PersonId, PersonRecord, SystemId, WeaponId};
use crate::chance::Chance;
use crate::combat::armament::{Armament, Arsenal};
use crate::combat::hull::Condition;
use crate::combat::weapon::WeaponSpec;
use crate::control::{ControlBits, FreshPilot, Gate, NovaBits, PilotFacts, Test};
use crate::fleet::Escort;
use crate::govt::Governments;
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource, Rulebook};

use super::QUOTE_ON_ATTACK;
use super::npc::Npc;
use super::table::ShipKind;

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

/// Whether `npc`, turning on the player, says its person's hail quote at
/// once, as `_AIMakeShipAttackPlayer` gates it (@0x89c50-0x89c9b): it was
/// not `threatened` already (`_ExtendedIsThreatToPlayer` @0x89c53, asked
/// before the goal changes; see [`Npc::threatens_player_extended`]), it flies a person
/// (+0xc8d0, @0x89c60) of `Flags` 0x0010 (@0x89c8a), it is no mission's
/// special ship (+0xc8d2, @0x89c71), it is not disabled (`_IsDisabled`
/// @0x89c94, which reads its armour, so a ship breaking up is disabled
/// too), and the player is not `player_dying` (`_IsDying` @0x89ca7). A
/// person escorting the player is not the person: the original's joined
/// ship is a mission ship.
///
/// It reads none of `nova_sim::person::quote_eligible`'s filters: not the `HailQuote`,
/// "once", the grudge, liking, mission or leaving flags, the player's
/// `InherentAI`, nor whether the person is gone for good; and it draws
/// nothing.
#[must_use]
pub fn turns_with_quote(npc: &Npc, threatened: bool, player_dying: bool) -> bool {
    !threatened
        && !player_dying
        && npc.condition == Condition::Intact
        && npc.mission.is_none()
        && npc.escort.is_none()
        && npc
            .person
            .is_some_and(|person| person.flags & QUOTE_ON_ATTACK != 0)
}

/// What `_AIMakeShipAttackPlayer`'s quote gate reads of the player: whether
/// it is dying (`_IsDying` @0x89ca7), and how it sees a ship
/// (`_ExtendedIsThreatToPlayer`'s `_ShipVisibleToShip(ship, player)`
/// @0x81bc6), from where it is and whether it scans for cloaked ships on
/// screen.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GateView {
    /// Whether the player is dying or destroyed.
    pub player_dying: bool,
    /// The player as it sees ships ([`cloak::visible`](crate::cloak::visible)).
    pub observer: crate::cloak::Observer,
}

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
        rounds.add(weapon, i32::from(ammo));
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
        reserves.shield.now = 0.0;
        reserves.armor.now = derelict_armor(kind.base_armor, kind.hull.tough);
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

/// A derelict's armour, of a ship type holding `armor`, its hull `tough`
/// or not: a third of it (a tenth for a tough hull), less one, just below
/// where it is disabled (@0xdd568, @0xdd110, @0xdd0c0). `armor` is the
/// `shïp`'s own `Armor` (+0x60, [`ShipKind::base_armor`]), without its
/// default outfits' armour: both derelict paths read it there, the rescue
/// setup (@0x42fae-0x42fdc) and `_SpawnPerson`'s derelict person
/// (@0x40ff6-0x41020), the tough hull being `Flags` 0x10 (+0x9e8).
#[must_use]
pub fn derelict_armor(armor: f32, tough: bool) -> f32 {
    let share = if tough {
        TOUGH_DERELICT_ARMOR_SHARE
    } else {
        DERELICT_ARMOR_SHARE
    };
    armor.mul_add(share, -DERELICT_ARMOR_LESS)
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
/// of the `LinkSyst` slip, `ShieldMod` and a derelict's setup speed
/// apply. Nova's is [`NovaPersons`].
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
    /// How fast a derelict person starts at system setup
    /// ([`RuleKey::DerelictSetupSpeed`]).
    fn derelict_setup_speed(&self) -> RuleSource;
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
    /// How fast a derelict person starts at setup
    /// ([`RuleKey::DerelictSetupSpeed`]).
    pub derelict_speed: RuleSource,
}

impl NovaPersons {
    /// The engine's rules, for a place that needs them as a constant.
    pub const ENGINE: Self = Self {
        odds: RuleSource::Engine,
        slots: RuleSource::Engine,
        link: RuleSource::Engine,
        shield_mod: RuleSource::Engine,
        derelict_speed: RuleSource::Engine,
    };

    /// The rules `rulebook` chooses: its [`RuleKey::PersonOdds`],
    /// [`RuleKey::SystemPersons`], [`RuleKey::LinkSystSlip`],
    /// [`RuleKey::ShieldMod`] and [`RuleKey::DerelictSetupSpeed`] entries.
    #[must_use]
    pub fn from_rulebook(rulebook: &Rulebook) -> Self {
        Self {
            odds: rulebook.source_for(RuleKey::PersonOdds),
            slots: rulebook.source_for(RuleKey::SystemPersons),
            link: rulebook.source_for(RuleKey::LinkSystSlip),
            shield_mod: rulebook.source_for(RuleKey::ShieldMod),
            derelict_speed: rulebook.source_for(RuleKey::DerelictSetupSpeed),
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

    fn derelict_setup_speed(&self) -> RuleSource {
        self.derelict_speed
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
    /// What the control-bit test reads about the player.
    pub pilot: &'a dyn PilotFacts,
    /// The player's fleet: the persons among it never spawn, and their
    /// names count as in the system.
    pub fleet: &'a [Escort],
}

impl PersonWorld<'_> {
    /// Whether person `id` escorts the player.
    #[must_use]
    pub fn in_fleet(&self, id: PersonId) -> bool {
        self.fleet.iter().any(|escort| escort.person == Some(id))
    }

    /// Whether `test` (a person's `ActiveOn`) holds for the player: never
    /// when it did not parse.
    #[must_use]
    pub fn allows(&self, test: &Test) -> bool {
        self.gate().allows(test)
    }
}

impl<'a> PersonWorld<'a> {
    /// The control bits and the pilot view, as a [`Gate`].
    #[must_use]
    pub fn gate(&self) -> Gate<'a> {
        Gate {
            control_bits: self.control_bits,
            pilot: self.pilot,
        }
    }
}

impl PersonWorld<'static> {
    /// Nova's rules by the engine, no person gone, no grudge, a fresh
    /// pilot's control bits tested by Nova's, and no fleet.
    pub const NONE: Self = Self {
        rules: &NovaPersons::ENGINE,
        gone: &NO_PERSONS,
        grudges: &NO_PERSONS,
        control_bits: &NovaBits,
        pilot: &FreshPilot,
        fleet: &[],
    };
}
