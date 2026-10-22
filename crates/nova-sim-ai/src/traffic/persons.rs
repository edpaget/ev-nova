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
    /// What the control-bit test reads about the player now.
    pub pilot: &'a dyn PilotFacts,
    /// The player as the last galaxy refresh read them: what a ship
    /// type's or fleet's `AppearOn`, and a drawn person's `ActiveOn`, are
    /// tested against, as `_PropagateMissionBitEffects` caches them (see
    /// [`RuleKey::SpawnAvailability`]).
    pub refreshed: &'a dyn PilotFacts,
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

    /// Whether a randomly drawn person of `ActiveOn` `test` may appear:
    /// it holds both for the player as the last refresh read them and for
    /// the player now, as `_SpawnPerson`'s draw reads the cached verdict
    /// (përs +0x622, @0x409b5) and then tests it again (@0x40b5d-0x40b6d).
    /// Never when it did not parse.
    #[must_use]
    pub fn drawn_active(&self, test: &Test) -> bool {
        self.gate().allows(test) && self.live_gate().allows(test)
    }

    /// Whether the person of a system's person slot, of `ActiveOn` `test`,
    /// may appear: it holds for the player now, as `_SetupShipsInSystem`
    /// tests it (@0x4334d-0x4335d) before spawning the person by ID, past
    /// the cache. Never when it did not parse.
    #[must_use]
    pub fn slot_active(&self, test: &Test) -> bool {
        self.live_gate().allows(test)
    }
}

impl<'a> PersonWorld<'a> {
    /// The control bits and the player as the last refresh read them, as
    /// a [`Gate`]: what every `AppearOn` is tested through.
    #[must_use]
    pub fn gate(&self) -> Gate<'a> {
        Gate {
            control_bits: self.control_bits,
            pilot: self.refreshed,
        }
    }

    /// The control bits and the player now, as a [`Gate`].
    fn live_gate(&self) -> Gate<'a> {
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
        refreshed: &FreshPilot,
        fleet: &[],
    };
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_base::fixture::{Draws, FAST};
    use nova_sim_catalog::fixture::{govt, person, weapon};

    use super::*;
    use crate::catalog::{GovtRecord, PersonWeapon, WeaponRecord};
    use crate::combat::armament::LauncherRounds;
    use crate::combat::hull::HullSpec;
    use crate::control::{PilotFacts, TestExpr};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::stats::ShipStats;
    use crate::traffic::ShipBehav;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{MissionMark, NpcPerson};

    const ENGINE: RuleSource = RuleSource::Engine;
    const BIBLE: RuleSource = RuleSource::Bible;

    // Where a person may appear.

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

    /// The systems 128-260 that `raw` allows by `slip`, each governed by
    /// `govt_of`.
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

    /// Systems 128-131 only.
    fn four(raw: i16) -> Vec<i16> {
        allowed(raw, BIBLE, governed)
            .into_iter()
            .filter(|&id| id <= 131)
            .collect()
    }

    #[test]
    fn a_link_decodes_each_range_and_the_slip_up_to_the_last_system() {
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
        assert_eq!(PersonLink::decode(0).slip, Some(SystemId(128)));
        assert_eq!(PersonLink::decode(2047).slip, Some(SystemId(2175)));
        assert_eq!(PersonLink::decode(2048).slip, None);
        assert_eq!(PersonLink::decode(-1).slip, None);
    }

    #[test]
    fn a_link_allows_its_systems_and_by_the_engine_the_slips() {
        assert_eq!(allowed(-1, BIBLE, governed).len(), 133);
        assert_eq!(allowed(132, ENGINE, governed), [132, 260]);
        assert_eq!(allowed(132, BIBLE, governed), [132]);
        assert_eq!(allowed(50, ENGINE, governed), [178]);
        assert_eq!(allowed(50, BIBLE, governed), Vec::<i16>::new());
        let independent = |id: i16| (id != 131).then_some(128);
        assert_eq!(allowed(9999, BIBLE, independent), [131]);
        assert_eq!(allowed(10_000, BIBLE, |_| None), Vec::<i16>::new());
        assert_eq!(four(10_001), [129]);
        assert_eq!(four(15_000), [128, 129], "its own and its ally's");
        assert_eq!(four(20_000), [129, 130], "never an independent one");
        assert_eq!(four(25_000), [130], "its enemy's");
        assert_eq!(four(30_000), Vec::<i16>::new());
    }

    // How often.

    fn roll(rules: NovaPersons, eligible: &[i16], draws: &[u32]) -> (PersonRoll, Vec<u32>) {
        let eligible: Vec<PersonId> = eligible.iter().copied().map(PersonId).collect();
        let mut chance = Draws::of(draws);
        let rolled = rules.roll(&eligible, &mut chance);
        (rolled, chance.asked)
    }

    fn bible() -> NovaPersons {
        NovaPersons::from_rulebook(&Rulebook::new(BIBLE))
    }

    #[test]
    fn by_the_engine_a_person_comes_on_0_in_7_then_by_its_draw_of_1022() {
        let engine = NovaPersons::ENGINE;
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
    }

    fn listed(rules: NovaPersons, prob: i16, draws: &[u32]) -> (bool, Vec<u32>) {
        let mut chance = Draws::of(draws);
        let listed = rules.listed(prob, &mut chance);
        (listed, chance.asked)
    }

    #[test]
    fn by_the_engine_a_slot_lists_its_person_on_a_percentage_roll_and_by_the_bible_always() {
        let engine = NovaPersons::ENGINE;
        assert_eq!(listed(engine, 50, &[49]), (true, vec![100]));
        assert_eq!(listed(engine, 50, &[50]), (false, vec![100]));
        assert_eq!(listed(engine, -1, &[0]), (false, vec![100]));
        assert_eq!(listed(bible(), 0, &[]), (true, vec![]), "no draw");
    }

    #[test]
    fn the_rules_follow_their_five_rulebook_keys() {
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
            (
                RuleKey::DerelictSetupSpeed,
                NovaPersons {
                    derelict_speed: BIBLE,
                    ..NovaPersons::ENGINE
                },
            ),
        ] {
            let rulebook = Rulebook::default().with_override(key, BIBLE);
            assert_eq!(NovaPersons::from_rulebook(&rulebook), expected, "{key:?}");
        }
        let bible = bible();
        let rules: &dyn PersonRules = &bible;
        assert_eq!(
            (
                rules.link_slip(),
                rules.shield_mod(),
                rules.derelict_setup_speed()
            ),
            (BIBLE, BIBLE, BIBLE)
        );
        let rules: &dyn PersonRules = &NovaPersons::ENGINE;
        assert_eq!(
            (
                rules.link_slip(),
                rules.shield_mod(),
                rules.derelict_setup_speed()
            ),
            (ENGINE, ENGINE, ENGINE)
        );
    }

    // The world.

    #[test]
    fn the_world_without_persons_tests_a_fresh_pilots_control_bits() {
        let none = PersonWorld::NONE;
        assert!(none.gone.is_empty() && none.grudges.is_empty());
        assert_eq!(none.rules.link_slip(), ENGINE);
        for active in [PersonWorld::drawn_active, PersonWorld::slot_active] {
            assert!(active(&none, &Test::default()), "a blank ActiveOn");
            assert!(!active(&none, &Test::parse("b3")), "no bit is set");
            assert!(!active(&none, &Test::parse("b1 &")), "malformed");
        }
    }

    #[test]
    fn a_persons_active_on_goes_through_the_worlds_control_bits() {
        /// Holds every test, counting the times it was asked.
        #[derive(Debug, Default)]
        struct Every(std::cell::Cell<u32>);

        impl ControlBits for Every {
            fn allows(&self, _test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
                self.0.set(self.0.get() + 1);
                true
            }
        }

        let every = Every::default();
        let world = PersonWorld {
            control_bits: &every,
            ..PersonWorld::NONE
        };
        assert!(world.drawn_active(&Test::parse("b3")));
        assert_eq!(every.0.get(), 2, "at the refresh and now");
        assert!(world.slot_active(&Test::parse("b3")));
        assert_eq!(every.0.get(), 3, "now");
    }

    #[test]
    fn a_person_is_in_the_fleet_while_an_escort_flies_it() {
        let escort = |person: Option<i16>| Escort {
            ship: crate::catalog::ShipId(128),
            reserves: ShipStats::new(FAST, &[]).full(),
            order: None,
            carried: false,
            wage: None,
            person: person.map(PersonId),
        };
        let fleet = [escort(None), escort(Some(130))];
        let world = PersonWorld {
            fleet: &fleet,
            ..PersonWorld::NONE
        };
        assert!(world.in_fleet(PersonId(130)));
        assert!(!world.in_fleet(PersonId(131)));
        assert!(!PersonWorld::NONE.in_fleet(PersonId(130)));
    }

    // The ship.

    #[test]
    fn a_persons_aggression_is_1_2_or_4() {
        for (aggress, aggression) in [(i16::MIN, 1), (0, 1), (1, 1), (2, 2), (3, 4), (9, 4)] {
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
    /// on L and 10 on R.
    fn armed() -> ShipKind {
        let arsenal = arsenal();
        let spec = |id: i16| *arsenal.weapon(WeaponId(id)).expect("armed");
        ShipKind {
            stats: ShipStats::new(FAST, &[]),
            armament: Armament::new([(spec(128), 2), (spec(129), 1)]),
            rounds: LauncherRounds::from([(WeaponId(129), 10), (WeaponId(130), 10)]),
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

    fn fitted(slots: &[PersonWeapon]) -> ShipKind {
        let person = PersonRecord {
            weapons: slots.to_vec(),
            ..person(128, 128)
        };
        fit(&armed(), &person, &arsenal(), ENGINE, false).0
    }

    fn counts(kind: &ShipKind) -> Vec<(i16, u32)> {
        kind.armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn a_persons_weapon_slots_add_to_the_ships_counts_and_rounds() {
        assert_eq!(counts(&fitted(&[])), [(128, 2), (129, 1)]);
        assert_eq!(counts(&fitted(&[slot(128, 2, 0)])), [(128, 4), (129, 1)]);
        assert_eq!(counts(&fitted(&[slot(128, 3, 0)])), [(128, 5), (129, 1)]);
        assert_eq!(
            counts(&fitted(&[slot(128, -2, 0)])),
            [(129, 1)],
            "none left"
        );
        assert_eq!(
            counts(&fitted(&[slot(131, 1, 0)])),
            [(128, 2), (129, 1), (131, 1)]
        );
        assert_eq!(counts(&fitted(&[slot(999, 3, 3)])), [(128, 2), (129, 1)]);
        assert_eq!(
            counts(&fitted(&[slot(128, 5, 0), slot(128, 1, 0)])),
            [(128, 3), (129, 1)],
            "a later slot for the same weapon replaces the earlier"
        );
        assert_eq!(
            fitted(&[slot(129, 0, 5), slot(129, 0, 2)])
                .rounds
                .slot(WeaponId(129)),
            12
        );
        assert_eq!(fitted(&[slot(129, 0, -15)]).rounds.slot(WeaponId(129)), -5);
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
        assert_eq!(reserves, kind.stats.full());
        let (kind, _) = shielded(250, BIBLE);
        assert_eq!((kind.stats.shield, kind.stats.armor), (75.0, 45.0));
        for unscaled in [0, -1] {
            assert_eq!(
                shielded(unscaled, ENGINE).0.stats,
                ShipStats::new(FAST, &[])
            );
        }
    }

    #[test]
    fn a_person_of_zero_fuel_starts_with_none() {
        let empty = PersonRecord {
            flags2: ZERO_FUEL | 0x0002,
            ..person(128, 128)
        };
        let (kind, reserves, _) = fit(&armed(), &empty, &arsenal(), ENGINE, false);
        assert_eq!(reserves.fuel.now, 0.0);
        assert_eq!(reserves.fuel.max, kind.stats.fuel);
        let other = PersonRecord {
            flags2: 0x0002,
            ..person(128, 128)
        };
        let (_, reserves, _) = fit(&armed(), &other, &arsenal(), ENGINE, false);
        assert_eq!(reserves.fuel.now, kind.stats.fuel, "without the flag, full");
    }

    #[test]
    fn a_derelict_person_starts_disabled_with_no_shield_and_a_share_of_its_base_armour() {
        for (tough, share) in [(false, 0.33f32), (true, 0.1)] {
            let plated = ShipKind {
                hull: HullSpec {
                    tough,
                    ..HullSpec::default()
                },
                base_armor: 100.0,
                ..armed()
            };
            let (_, reserves, condition) =
                fit(&plated, &person(128, 128), &arsenal(), ENGINE, true);
            assert_eq!(condition, Condition::Disabled);
            assert_eq!(reserves.shield.now, 0.0);
            assert!(
                (reserves.armor.now - 100.0f32.mul_add(share, -1.0)).abs() < 1e-4,
                "tough {tough}: {}",
                reserves.armor.now
            );
            assert!((derelict_armor(100.0, tough) - reserves.armor.now).abs() < 1e-4);
        }
    }

    // The quote.

    fn quoting(flags: u16) -> NpcPerson {
        NpcPerson {
            id: PersonId(151),
            flags,
            coward: 0,
            comm_quote: -1,
            hail_quote: 8,
            mission: false,
            portrait: None,
            invincible: false,
            grudge: false,
            quoted: false,
            quoted_at: None,
            zero_fuel: false,
        }
    }

    /// A flying person of `QUOTE_ON_ATTACK` (and `more`), idle and intact.
    fn turning(more: u16) -> Npc {
        Npc {
            person: Some(quoting(QUOTE_ON_ATTACK | more)),
            ..npc(1, ShipStats::new(FAST, &[]))
        }
    }

    #[test]
    fn a_ship_turning_on_the_player_says_its_quote_as_ai_make_ship_attack_player_gates_it() {
        assert!(turns_with_quote(&turning(0), false, false));
        assert!(turns_with_quote(&turning(0x0001), false, false));
        let no_flag = Npc {
            person: Some(quoting(0x0001)),
            ..turning(0)
        };
        let marked = Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::AttackPlayer,
                formation: None,
                order: None,
            }),
            ..turning(0)
        };
        let nobody = Npc {
            person: None,
            ..turning(0)
        };
        let escort = Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            ..turning(0)
        };
        let disabled = Npc {
            condition: Condition::Disabled,
            ..turning(0)
        };
        for (npc, why) in [
            (&no_flag, "no 0x0010"),
            (&marked, "a mission's special ship"),
            (&nobody, "no person"),
            (&escort, "the player's escort"),
            (&disabled, "disabled"),
        ] {
            assert!(!turns_with_quote(npc, false, false), "{why}");
        }
        assert!(!turns_with_quote(&turning(0), true, false), "a threat");
        assert!(
            !turns_with_quote(&turning(0), false, true),
            "the player dying"
        );
    }
}
