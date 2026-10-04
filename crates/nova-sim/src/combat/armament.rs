//! A ship's armament: the weapons it carries, how many of each, and
//! firing them when its trigger is pulled.
//!
//! Firing is the original's (`_FirePlayerWeapon`, `_WeaponBurstInterval`
//! and `_SpawnShot` in the `EV Nova` executable), which every ship goes
//! through:
//!
//! - The [`Trigger`] picks the weapons: the primary trigger every weapon
//!   that is not a secondary, and the secondary trigger the one secondary
//!   selected.
//! - A ship carrying n of a weapon fires one shot once its reload timer
//!   has run out, and sets the timer to `Reload / n` ticks; a timer counts
//!   down a tick at a time ([`Armament::reload`]), so `Reload` 0 fires
//!   every tick. (The `Flags` 0x0040 simultaneous fire is not done yet.)
//! - With a `BurstCount`, each copy fires that many shots (`BurstCount` x
//!   n in all) before the timer is set to `BurstReload` instead.
//! - A shot spends its ammo ([`Ammo`]): a round from its ammunition's
//!   store, or its fuel, and does not fire without it. A weapon that uses
//!   ammo only at the end of a burst (`Flags3` 0x0001) spends it each
//!   time the shots in the burst reach a multiple of its `BurstCount`.
//! - It leaves up to its `Inaccuracy` off its heading either way: the
//!   draw `below(2n) - n`, never asked of a weapon that is accurate.
//! - Nothing fires unless the ship is [`Condition::Intact`].
//! - A weapon of a guidance a later phase flies does not fire; firing one
//!   is reported once ([`SimDiagnostic::UnimplementedGuidance`]), and so is
//!   each flag a weapon that does fire sets and the simulation ignores.
//!
//! The player's armament ([`Arsenal::player`]) is its ship's stock weapons
//! and the weapons among its outfits ([`MOD_WEAPON`]), firing the rounds
//! of the ammunition among its outfits ([`MOD_AMMO`], through
//! [`OutfitRounds`]). An NPC's ([`Arsenal::npc`]) is its ship's stock
//! weapons and default items, its rounds held on the NPC: each stock
//! weapon's `AmmoLoad`, and the ammunition among its default items.

use std::collections::BTreeMap;

use super::hull::{Condition, HullSpec};
use super::report::{Reports, SimDiagnostic};
use super::weapon::{Ammo, Guidance, WeaponSpec};
use crate::catalog::{
    CombatCatalog, HullRecord, OutfitId, OutfitRecord, ShipId, WeaponId, WeaponRecord,
};
use crate::chance::Chance;
use crate::reserves::Gauge;

/// The `oütf` `ModType` that is a weapon: its `ModVal` is the `wëap`.
pub const MOD_WEAPON: i16 = 1;
/// The `oütf` `ModType` that is ammunition: its `ModVal` is the `wëap` it
/// is the rounds of.
pub const MOD_AMMO: i16 = 3;

/// The fire command a ship holds: the player's keys, or an NPC's
/// [`Behaviour`](crate::ai::Behaviour).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Trigger {
    /// The primary trigger: every weapon that is not a secondary.
    pub primary: bool,
    /// The secondary trigger, on this secondary weapon.
    pub secondary: Option<WeaponId>,
}

impl Trigger {
    /// Whether it fires `weapon`.
    #[must_use]
    pub fn fires(self, weapon: &WeaponSpec) -> bool {
        if weapon.secondary() {
            self.secondary == Some(weapon.id)
        } else {
            self.primary
        }
    }
}

/// Where a ship's rounds of ammunition are kept.
pub trait Rounds {
    /// How many rounds of `ammo` it holds.
    fn held(&self, ammo: WeaponId) -> u32;
    /// Spends a round of `ammo`, which it holds.
    fn spend(&mut self, ammo: WeaponId);
}

/// An NPC's rounds: how many of each ammunition.
impl Rounds for BTreeMap<WeaponId, u32> {
    fn held(&self, ammo: WeaponId) -> u32 {
        self.get(&ammo).copied().unwrap_or(0)
    }

    fn spend(&mut self, ammo: WeaponId) {
        if let Some(held) = self.get_mut(&ammo) {
            *held = held.saturating_sub(1);
        }
    }
}

/// The player's rounds: the ammunition outfits it owns.
#[derive(Debug)]
pub struct OutfitRounds<'a> {
    /// How many of each outfit the player owns.
    pub owned: &'a mut BTreeMap<OutfitId, u16>,
    /// Each ammunition outfit, with the `wëap` it is the rounds of
    /// ([`Arsenal::ammo_outfits`]).
    pub sources: &'a [(WeaponId, OutfitId)],
}

impl Rounds for OutfitRounds<'_> {
    fn held(&self, ammo: WeaponId) -> u32 {
        self.sources
            .iter()
            .filter(|&&(of, _)| of == ammo)
            .map(|(_, outfit)| u32::from(self.owned.get(outfit).copied().unwrap_or(0)))
            .sum()
    }

    /// Spends one of the first of its outfits the player owns any of; an
    /// outfit used up is no longer owned.
    fn spend(&mut self, ammo: WeaponId) {
        let source = self.sources.iter().find(|&&(of, outfit)| {
            of == ammo && self.owned.get(&outfit).is_some_and(|&count| count > 0)
        });
        if let Some(&(_, outfit)) = source {
            let count = self.owned.entry(outfit).or_default();
            *count -= 1;
            if *count == 0 {
                self.owned.remove(&outfit);
            }
        }
    }
}

/// One weapon type a ship carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    /// The weapon.
    pub spec: WeaponSpec,
    /// How many the ship carries.
    pub count: u32,
    /// Ticks until it can fire again.
    pub reload: f32,
    /// Shots fired in the burst under way.
    pub burst: u32,
}

impl Mount {
    /// Whether the ship can pay for a shot from `rounds` and `fuel`.
    fn affords(&self, rounds: &dyn Rounds, fuel: Gauge) -> bool {
        match self.spec.ammo {
            Ammo::Unlimited | Ammo::Other(_) => true,
            Ammo::Rounds(ammo) => rounds.held(ammo) > 0,
            Ammo::Fuel(cost) => fuel.now >= cost,
        }
    }

    /// Pays for a shot from `rounds` and `fuel`.
    fn pay(&self, rounds: &mut dyn Rounds, fuel: &mut Gauge) {
        match self.spec.ammo {
            Ammo::Unlimited | Ammo::Other(_) => {}
            Ammo::Rounds(ammo) => rounds.spend(ammo),
            Ammo::Fuel(cost) => fuel.now -= cost,
        }
    }

    /// Whether this shot, the burst's `burst`th, spends ammo.
    fn spends(&self) -> bool {
        if self.spec.ammo_at_burst_end() {
            self.spec.burst_count > 0 && self.burst.is_multiple_of(self.spec.burst_count)
        } else {
            true
        }
    }
}

/// A shot or beam a ship launches this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Launch {
    /// The weapon.
    pub weapon: WeaponSpec,
    /// How far off the ship's heading it leaves, in degrees clockwise.
    pub offset: f32,
}

/// The weapons a ship carries, in the order they were mounted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Armament {
    mounts: Vec<Mount>,
}

impl Armament {
    /// An armament of `weapons`, each with how many are carried: the same
    /// weapon twice adds up, and none of one mounts nothing. Each is ready
    /// to fire.
    #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponSpec, u32)>) -> Self {
        let mut mounts: Vec<Mount> = Vec::new();
        for (spec, count) in weapons {
            if count == 0 {
                continue;
            }
            match mounts.iter_mut().find(|mount| mount.spec.id == spec.id) {
                Some(mount) => mount.count += count,
                None => mounts.push(Mount {
                    spec,
                    count,
                    reload: 0.0,
                    burst: 0,
                }),
            }
        }
        Self { mounts }
    }

    /// The weapons carried.
    #[must_use]
    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    /// Fires every weapon `trigger` picks that is ready, on a ship in
    /// `condition`, paying from `rounds` and `fuel`, the inaccuracy drawn
    /// on `chance`, and gives what it launched; what it reports goes to
    /// `reports` (see the module docs).
    pub fn fire(
        &mut self,
        trigger: Trigger,
        condition: Condition,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
    ) -> Vec<Launch> {
        let mut launches = Vec::new();
        if condition != Condition::Intact {
            return launches;
        }
        for mount in &mut self.mounts {
            if !trigger.fires(&mount.spec) || mount.reload > 0.0 {
                continue;
            }
            if let Guidance::Other(guidance) = mount.spec.guidance {
                reports.report(SimDiagnostic::UnimplementedGuidance {
                    weapon: mount.spec.id,
                    guidance,
                });
                continue;
            }
            if !mount.affords(rounds, *fuel) {
                continue;
            }
            let offset = inaccuracy(mount.spec.inaccuracy, chance);
            reports.fired(&mount.spec);
            if mount.spec.burst_count > 0 {
                mount.burst += 1;
            }
            if mount.spends() {
                mount.pay(rounds, fuel);
            }
            mount.reload = mount.spec.reload / mount.count as f32;
            if mount.spec.burst_count > 0 && mount.burst >= mount.spec.burst_count * mount.count {
                mount.burst = 0;
                mount.reload = mount.spec.burst_reload;
            }
            launches.push(Launch {
                weapon: mount.spec,
                offset,
            });
        }
        launches
    }

    /// Counts every weapon's reload timer down a tick.
    pub fn reload(&mut self) {
        for mount in &mut self.mounts {
            mount.reload = (mount.reload - 1.0).max(0.0);
        }
    }
}

/// How far off its heading a shot of a weapon `inaccuracy` degrees
/// inaccurate leaves, drawn on `chance`.
fn inaccuracy(inaccuracy: u32, chance: &mut dyn Chance) -> f32 {
    if inaccuracy == 0 {
        return 0.0;
    }
    let draw = chance.below(inaccuracy.saturating_mul(2));
    (i64::from(draw) - i64::from(inaccuracy)) as f32
}

/// Every weapon, and every ship type's combat fields, read once when a
/// session starts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arsenal {
    weapons: BTreeMap<WeaponId, WeaponSpec>,
    hulls: BTreeMap<ShipId, HullRecord>,
}

impl Arsenal {
    /// The arsenal of `weapons` and `hulls`.
    #[must_use]
    pub fn new(weapons: &[WeaponRecord], hulls: Vec<HullRecord>) -> Self {
        Self {
            weapons: weapons
                .iter()
                .map(|record| (record.id, WeaponSpec::new(record)))
                .collect(),
            hulls: hulls.into_iter().map(|hull| (hull.id, hull)).collect(),
        }
    }

    /// The arsenal `catalog` gives.
    #[must_use]
    pub fn read(catalog: &(impl CombatCatalog + ?Sized)) -> Self {
        Self::new(&catalog.weapons(), catalog.hulls())
    }

    /// Weapon `id`, if it can be read.
    #[must_use]
    pub fn weapon(&self, id: WeaponId) -> Option<&WeaponSpec> {
        self.weapons.get(&id)
    }

    /// Ship type `ship`'s hull; a placeholder for one that cannot be read.
    #[must_use]
    pub fn hull(&self, ship: ShipId) -> HullSpec {
        self.hulls.get(&ship).map(HullSpec::new).unwrap_or_default()
    }

    /// The armament of a ship of type `ship` carrying `owned` of
    /// `outfits`: its stock weapons, and its weapon outfits.
    #[must_use]
    pub fn player(
        &self,
        ship: ShipId,
        owned: &BTreeMap<OutfitId, u16>,
        outfits: &[OutfitRecord],
    ) -> Armament {
        let stock = self.stock(ship).map(|(weapon, count, _)| (weapon, count));
        let fitted = mods(owned, outfits, MOD_WEAPON);
        Armament::new(
            stock
                .chain(fitted)
                .filter_map(|(weapon, count)| Some((*self.weapon(weapon)?, count))),
        )
    }

    /// The armament of an NPC of type `ship` carrying `defaults` of
    /// `outfits`, with the rounds it holds: each stock weapon's
    /// `AmmoLoad` of its ammunition, and its ammunition outfits.
    #[must_use]
    pub fn npc(
        &self,
        ship: ShipId,
        defaults: &BTreeMap<OutfitId, u16>,
        outfits: &[OutfitRecord],
    ) -> (Armament, BTreeMap<WeaponId, u32>) {
        let armament = self.player(ship, defaults, outfits);
        let mut rounds = BTreeMap::new();
        for (weapon, _, load) in self.stock(ship) {
            if let Some(Ammo::Rounds(ammo)) = self.weapon(weapon).map(|spec| spec.ammo)
                && load > 0
            {
                *rounds.entry(ammo).or_default() += load;
            }
        }
        for (ammo, count) in mods(defaults, outfits, MOD_AMMO) {
            *rounds.entry(ammo).or_default() += count;
        }
        (armament, rounds)
    }

    /// Ship type `ship`'s stock weapons that carry any, each with its
    /// count and its `AmmoLoad` (none below none).
    fn stock(&self, ship: ShipId) -> impl Iterator<Item = (WeaponId, u32, u32)> + '_ {
        self.hulls
            .get(&ship)
            .into_iter()
            .flat_map(|hull| &hull.weapons)
            .map(|stock| {
                let count = u32::try_from(stock.count).unwrap_or(0);
                let load = u32::try_from(stock.ammo).unwrap_or(0);
                (stock.weapon, count, load)
            })
    }

    /// Each ammunition outfit among `outfits`, with the `wëap` it is the
    /// rounds of, by ascending outfit ID.
    #[must_use]
    pub fn ammo_outfits(outfits: &[OutfitRecord]) -> Vec<(WeaponId, OutfitId)> {
        outfits
            .iter()
            .flat_map(|record| {
                record
                    .mods
                    .iter()
                    .filter(|&&(mod_type, _)| mod_type == MOD_AMMO)
                    .map(|&(_, weapon)| (WeaponId(weapon), record.id))
            })
            .collect()
    }
}

/// The `wëap` each of `owned` of `outfits` names in a mod of `mod_type`,
/// with how many are owned.
fn mods(
    owned: &BTreeMap<OutfitId, u16>,
    outfits: &[OutfitRecord],
    mod_type: i16,
) -> Vec<(WeaponId, u32)> {
    outfits
        .iter()
        .filter_map(|record| Some((record, *owned.get(&record.id)?)))
        .flat_map(|(record, count)| {
            record
                .mods
                .iter()
                .filter(move |&&(kind, _)| kind == mod_type)
                .map(move |&(_, weapon)| (WeaponId(weapon), u32::from(count)))
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::StockWeapon;
    use crate::chance::NeverFires;
    use crate::combat::flags::FlagField;
    use crate::testkit::{Draws, hull, outfit, weapon};

    /// A blaster reloading every `reload` ticks, unlimited and accurate.
    fn blaster(id: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            reload,
            ..weapon(id)
        }
    }

    fn mounted(record: WeaponRecord, count: u32) -> Armament {
        Armament::new([(WeaponSpec::new(&record), count)])
    }

    const PRIMARY: Trigger = Trigger {
        primary: true,
        secondary: None,
    };

    /// What `armament` has to fire with.
    struct Supplies {
        rounds: BTreeMap<WeaponId, u32>,
        fuel: Gauge,
        reports: Reports,
    }

    impl Supplies {
        fn none() -> Self {
            Self {
                rounds: BTreeMap::new(),
                fuel: Gauge::full(0.0),
                reports: Reports::default(),
            }
        }

        /// One tick: fire on `trigger`, then reload.
        fn tick(&mut self, armament: &mut Armament, trigger: Trigger) -> Vec<Launch> {
            let launches = armament.fire(
                trigger,
                Condition::Intact,
                &mut self.rounds,
                &mut self.fuel,
                &mut NeverFires,
                &mut self.reports,
            );
            armament.reload();
            launches
        }

        /// The ticks of `ticks` on which `armament` fires on `trigger`.
        fn firing(&mut self, armament: &mut Armament, trigger: Trigger, ticks: u32) -> Vec<u32> {
            (0..ticks)
                .filter(|_| !self.tick(armament, trigger).is_empty())
                .collect()
        }
    }

    #[test]
    fn one_copy_fires_every_reload_ticks() {
        let mut armament = mounted(blaster(128, 10), 1);
        assert_eq!(
            Supplies::none().firing(&mut armament, PRIMARY, 31),
            [0, 10, 20, 30]
        );
    }

    #[test]
    fn two_copies_fire_every_half_reload() {
        let mut armament = mounted(blaster(128, 10), 2);
        assert_eq!(
            Supplies::none().firing(&mut armament, PRIMARY, 16),
            [0, 5, 10, 15]
        );
        let mut odd = mounted(blaster(129, 13), 2);
        assert_eq!(
            Supplies::none().firing(&mut odd, PRIMARY, 15),
            [0, 7, 14],
            "6.5 ticks is fired on the 7th"
        );
    }

    #[test]
    fn a_reload_of_none_fires_every_tick() {
        let mut armament = mounted(blaster(141, 0), 1);
        assert_eq!(
            Supplies::none().firing(&mut armament, PRIMARY, 4),
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn a_weapon_not_fired_keeps_reloading() {
        let mut armament = mounted(blaster(128, 10), 1);
        let mut supplies = Supplies::none();
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 1), [0]);
        assert_eq!(
            supplies.firing(&mut armament, Trigger::default(), 9),
            [0_u32; 0]
        );
        assert_eq!(armament.mounts()[0].reload, 0.0);
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 1), [0], "ready");
    }

    fn bursting(reload: i16, burst_count: i16, burst_reload: i16) -> WeaponRecord {
        WeaponRecord {
            reload,
            burst_count,
            burst_reload,
            ..weapon(146)
        }
    }

    #[test]
    fn a_burst_of_burst_count_shots_a_copy_is_followed_by_the_burst_reload() {
        let mut one = mounted(bursting(2, 3, 20), 1);
        assert_eq!(
            Supplies::none().firing(&mut one, PRIMARY, 50),
            [0, 2, 4, 24, 26, 28, 48]
        );
        let mut two = mounted(bursting(2, 3, 20), 2);
        assert_eq!(
            Supplies::none().firing(&mut two, PRIMARY, 32),
            [0, 1, 2, 3, 4, 5, 25, 26, 27, 28, 29, 30],
            "BurstCount x 2 shots"
        );
        let mut unburst = mounted(bursting(2, 0, 20), 1);
        assert_eq!(
            Supplies::none().firing(&mut unburst, PRIMARY, 7),
            [0, 2, 4, 6]
        );
    }

    /// A weapon burning `cost` x 10 fuel units a shot.
    fn fuelled(cost: i16, flags3: u16) -> WeaponRecord {
        WeaponRecord {
            ammo_type: -1000 - cost,
            flags3,
            ..bursting(0, 3, 5)
        }
    }

    #[test]
    fn fuel_is_spent_a_shot_and_a_shot_needs_it() {
        let mut armament = mounted(fuelled(10, 0), 1);
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge {
            now: 2.5,
            max: 300.0,
        };
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 4), [0, 1]);
        assert_eq!(supplies.fuel.now, 0.5, "refused below the cost");
        supplies.fuel.now = 1.0;
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 2), [0]);
        assert_eq!(supplies.fuel.now, 0.0, "never below none");
    }

    #[test]
    fn a_weapon_that_uses_ammo_at_the_end_of_a_burst_pays_only_then() {
        let mut armament = mounted(fuelled(10, 0x0001), 1);
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge::full(5.0);
        let mut fuel = Vec::new();
        for _ in 0..10 {
            supplies.tick(&mut armament, PRIMARY);
            fuel.push(supplies.fuel.now);
        }
        // Shots on ticks 0-2, then the burst reload of 5, then 7-9.
        assert_eq!(
            fuel,
            [5.0, 5.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 3.0],
            "a unit at each third shot"
        );
        let mut two = mounted(fuelled(10, 0x0001), 2);
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge::full(5.0);
        assert_eq!(supplies.firing(&mut two, PRIMARY, 6), [0, 1, 2, 3, 4, 5]);
        assert_eq!(supplies.fuel.now, 3.0, "at each multiple of BurstCount");
        let mut unburst = mounted(
            WeaponRecord {
                burst_count: 0,
                ..fuelled(10, 0x0001)
            },
            1,
        );
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge::full(5.0);
        assert_eq!(supplies.firing(&mut unburst, PRIMARY, 3), [0, 1, 2]);
        assert_eq!(supplies.fuel.now, 5.0, "no burst, no end of one");
    }

    #[test]
    fn a_round_is_spent_a_shot_and_a_shot_needs_one() {
        let rocket = WeaponRecord {
            ammo_type: 10,
            ..blaster(138, 0)
        };
        let mut armament = mounted(rocket, 1);
        let mut supplies = Supplies::none();
        supplies.rounds.insert(WeaponId(138), 2);
        supplies.rounds.insert(WeaponId(140), 9);
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 4), [0, 1]);
        assert_eq!(supplies.rounds[&WeaponId(138)], 0);
        assert_eq!(supplies.rounds[&WeaponId(140)], 9, "only its own");
    }

    #[test]
    fn the_triggers_pick_the_primaries_or_one_secondary() {
        let secondary = |id| WeaponRecord {
            flags: 0x0002,
            ..blaster(id, 0)
        };
        let mut armament = Armament::new(
            [
                blaster(128, 0),
                secondary(138),
                secondary(140),
                blaster(129, 0),
            ]
            .iter()
            .map(|record| (WeaponSpec::new(record), 1)),
        );
        let fired = |armament: &mut Armament, trigger| -> Vec<i16> {
            Supplies::none()
                .tick(armament, trigger)
                .iter()
                .map(|launch| launch.weapon.id.0)
                .collect()
        };
        assert_eq!(fired(&mut armament, PRIMARY), [128, 129]);
        let second = Trigger {
            primary: false,
            secondary: Some(WeaponId(140)),
        };
        assert_eq!(fired(&mut armament, second), [140]);
        let both = Trigger {
            primary: true,
            secondary: Some(WeaponId(138)),
        };
        assert_eq!(fired(&mut armament, both), [128, 138, 129]);
        let primary_as_secondary = Trigger {
            primary: false,
            secondary: Some(WeaponId(128)),
        };
        assert_eq!(
            fired(&mut armament, primary_as_secondary),
            Vec::<i16>::new()
        );
        assert_eq!(fired(&mut armament, Trigger::default()), Vec::<i16>::new());
    }

    #[test]
    fn inaccuracy_draws_twice_it_and_offsets_by_minus_it_up_to_one_less() {
        let wild = WeaponRecord {
            inaccuracy: 9,
            ..blaster(128, 0)
        };
        for (draw, offset) in [(0, -9.0), (9, 0.0), (17, 8.0)] {
            let mut armament = mounted(wild, 1);
            let mut chance = Draws::of(&[draw]);
            let launches = armament.fire(
                PRIMARY,
                Condition::Intact,
                &mut BTreeMap::new(),
                &mut Gauge::default(),
                &mut chance,
                &mut Reports::default(),
            );
            assert_eq!(launches[0].offset, offset, "{draw}");
            assert_eq!(chance.asked, [18]);
        }
        let mut accurate = mounted(blaster(128, 0), 1);
        let mut chance = Draws::of(&[]);
        let launches = accurate.fire(
            PRIMARY,
            Condition::Intact,
            &mut BTreeMap::new(),
            &mut Gauge::default(),
            &mut chance,
            &mut Reports::default(),
        );
        assert_eq!(launches[0].offset, 0.0);
        assert!(chance.asked.is_empty(), "never asked");
    }

    #[test]
    fn nothing_fires_unless_the_ship_is_intact() {
        for condition in [
            Condition::Disabled,
            Condition::Dying { ticks_left: 5 },
            Condition::Destroyed,
        ] {
            let mut armament = mounted(blaster(128, 0), 1);
            let launches = armament.fire(
                PRIMARY,
                condition,
                &mut BTreeMap::new(),
                &mut Gauge::default(),
                &mut NeverFires,
                &mut Reports::default(),
            );
            assert_eq!(launches, [], "{condition:?}");
        }
    }

    #[test]
    fn each_unimplemented_flag_is_reported_once_however_often_it_fires() {
        let mining = WeaponRecord {
            flags2: 0x8000,
            ..blaster(181, 0)
        };
        let mut armament = mounted(mining, 1);
        let mut supplies = Supplies::none();
        assert_eq!(supplies.reports.take(), [], "not before it fires");
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 5).len(), 5);
        assert_eq!(
            supplies.reports.take(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(181),
                field: FlagField::Flags2,
                bit: 0x8000
            }]
        );
    }

    #[test]
    fn a_weapon_of_another_guidance_is_reported_once_and_never_fires() {
        let homing = WeaponRecord {
            guidance: 1,
            flags: 0x0040,
            ..blaster(131, 0)
        };
        let mut armament = mounted(homing, 1);
        let mut supplies = Supplies::none();
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 5), [0_u32; 0]);
        assert_eq!(
            supplies.reports.take(),
            [SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(131),
                guidance: 1
            }],
            "its flags never reported, as it never fires"
        );
        assert_eq!(
            supplies.firing(&mut armament, Trigger::default(), 1),
            [0_u32; 0]
        );
        assert_eq!(supplies.reports.take(), [], "nor when not fired");
    }

    #[test]
    fn the_same_weapon_mounted_twice_adds_up_and_none_mounts_nothing() {
        let spec = |id| WeaponSpec::new(&blaster(id, 10));
        let armament = Armament::new([(spec(128), 1), (spec(129), 0), (spec(128), 2)]);
        assert_eq!(armament.mounts().len(), 1);
        assert_eq!(armament.mounts()[0].count, 3);
        assert_eq!(armament.mounts()[0].spec.id, WeaponId(128));
        assert_eq!(Armament::default().mounts(), []);
    }

    /// Ship 128 carries two blasters (128) and a rocket launcher (138)
    /// with 20 rockets, and a slot naming weapon 999, which cannot be
    /// read.
    fn arsenal() -> Arsenal {
        let rocket = WeaponRecord {
            ammo_type: 10,
            flags: 0x0002,
            ..blaster(138, 15)
        };
        Arsenal::new(
            &[blaster(128, 10), rocket, blaster(150, 5)],
            vec![HullRecord {
                weapons: vec![
                    StockWeapon {
                        weapon: WeaponId(128),
                        count: 2,
                        ammo: 0,
                    },
                    StockWeapon {
                        weapon: WeaponId(138),
                        count: 1,
                        ammo: 20,
                    },
                    StockWeapon {
                        weapon: WeaponId(999),
                        count: 1,
                        ammo: 5,
                    },
                    StockWeapon {
                        weapon: WeaponId(150),
                        count: -1,
                        ammo: -4,
                    },
                ],
                size: Some(30),
                ..hull(128)
            }],
        )
    }

    /// Outfit 200 is a weapon, 150; outfit 201 is rockets; outfit 202 is
    /// a shield booster.
    fn outfits() -> Vec<OutfitRecord> {
        vec![
            outfit(200, &[(MOD_WEAPON, 150)]),
            outfit(201, &[(MOD_AMMO, 138)]),
            outfit(202, &[(4, 100)]),
        ]
    }

    fn counts(armament: &Armament) -> Vec<(i16, u32)> {
        armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn the_players_armament_is_its_stock_weapons_and_its_weapon_outfits() {
        let arsenal = arsenal();
        let owned = BTreeMap::from([(OutfitId(200), 2), (OutfitId(201), 5), (OutfitId(202), 1)]);
        assert_eq!(
            counts(&arsenal.player(ShipId(128), &owned, &outfits())),
            [(128, 2), (138, 1), (150, 2)],
            "weapon 999 cannot be read, and none of 150 is stock"
        );
        assert_eq!(
            counts(&arsenal.player(ShipId(129), &owned, &outfits())),
            [(150, 2)],
            "a ship type that cannot be read has no stock weapons"
        );
        assert_eq!(MOD_WEAPON, 1);
    }

    #[test]
    fn an_npcs_rounds_are_its_ammo_load_and_its_ammunition() {
        let arsenal = arsenal();
        let defaults = BTreeMap::from([(OutfitId(201), 4)]);
        let (armament, rounds) = arsenal.npc(ShipId(128), &defaults, &outfits());
        assert_eq!(counts(&armament), [(128, 2), (138, 1)]);
        assert_eq!(rounds, BTreeMap::from([(WeaponId(138), 24)]));
        let (_, unloaded) = arsenal.npc(ShipId(128), &BTreeMap::new(), &outfits());
        assert_eq!(unloaded, BTreeMap::from([(WeaponId(138), 20)]));
        assert_eq!(MOD_AMMO, 3);
    }

    #[test]
    fn a_hull_is_its_records_or_a_placeholder() {
        let arsenal = arsenal();
        assert_eq!(arsenal.hull(ShipId(128)).hit_radius, 30.0 * 0.33);
        assert_eq!(arsenal.hull(ShipId(129)), HullSpec::default());
        assert_eq!(arsenal.weapon(WeaponId(150)).map(|w| w.reload), Some(5.0));
        assert_eq!(arsenal.weapon(WeaponId(151)), None);
    }

    #[test]
    fn ammunition_outfits_name_the_weapon_they_are_rounds_of() {
        let mut both = outfit(203, &[(4, 10), (MOD_AMMO, 140)]);
        both.mods[2] = (MOD_AMMO, 141);
        let mut records = outfits();
        records.push(both);
        assert_eq!(
            Arsenal::ammo_outfits(&records),
            [
                (WeaponId(138), OutfitId(201)),
                (WeaponId(140), OutfitId(203)),
                (WeaponId(141), OutfitId(203))
            ]
        );
    }

    #[test]
    fn the_players_rounds_are_its_ammunition_outfits() {
        let sources = [
            (WeaponId(138), OutfitId(201)),
            (WeaponId(138), OutfitId(204)),
            (WeaponId(140), OutfitId(203)),
        ];
        let mut owned =
            BTreeMap::from([(OutfitId(201), 1), (OutfitId(204), 2), (OutfitId(202), 7)]);
        let mut rounds = OutfitRounds {
            owned: &mut owned,
            sources: &sources,
        };
        assert_eq!(rounds.held(WeaponId(138)), 3);
        assert_eq!(rounds.held(WeaponId(140)), 0);
        rounds.spend(WeaponId(138));
        assert_eq!(rounds.held(WeaponId(138)), 2);
        rounds.spend(WeaponId(138));
        rounds.spend(WeaponId(140));
        assert_eq!(
            owned,
            BTreeMap::from([(OutfitId(202), 7), (OutfitId(204), 1)]),
            "used up and no longer owned; none of 203 to spend"
        );
    }

    #[test]
    fn an_npcs_rounds_are_counted_by_ammunition() {
        let mut rounds = BTreeMap::from([(WeaponId(138), 1)]);
        assert_eq!(Rounds::held(&rounds, WeaponId(138)), 1);
        assert_eq!(Rounds::held(&rounds, WeaponId(140)), 0);
        rounds.spend(WeaponId(138));
        rounds.spend(WeaponId(138));
        rounds.spend(WeaponId(140));
        assert_eq!(Rounds::held(&rounds, WeaponId(138)), 0);
        assert_eq!(rounds.len(), 1);
    }
}
