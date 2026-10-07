//! A ship's armament: the weapons it carries, how many of each, and
//! firing them when its trigger is pulled, and its point defence.
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
//! - Its guidance aims it ([`aim`](super::aim)): a weapon that does not
//!   fire (a turret with no target, or one in a blind spot or out of its
//!   arc, and point defence on the trigger) spends nothing and does not
//!   reload.
//! - It leaves up to its `Inaccuracy` off its aim either way: the draw
//!   `below(2n) - n`, never asked of a weapon that is accurate.
//! - Nothing fires unless the ship is [`Condition::Intact`].
//! - A weapon of a guidance the simulation does not fly (2, 99 without a
//!   ship to carry, anything undocumented) does not fire; firing one is
//!   reported once ([`SimDiagnostic::UnimplementedGuidance`]), and so is
//!   each flag a weapon that does fire sets and the simulation ignores.
//! - A fighter bay ([`Guidance::FighterBay`]) fires on the secondary
//!   trigger, never on the primary, and with [`Trigger::bays`], an NPC
//!   attacking (`_AILaunchFighter` @0x81372): then only the bay of lowest
//!   weapon ID holding rounds launches, once it is ready, a later bay
//!   waiting until that one is empty. Its round is a fighter, launched
//!   as a [`Launch`] the fight turns into a sortie. A fighter docking
//!   goes back aboard ([`Armament::stow`], `_AIAddFighterToParent`
//!   @0x802a5): a round to the first bay launching its type, with no cap;
//!   a bay that held none restarts its reload.
//! - Point defence ([`Armament::fire_point_defence`]) fires, each tick,
//!   only the ready point-defence weapon the ship can pay for with the
//!   lowest weapon ID, and only at a missile it picks; it pays, bursts and
//!   reloads like any weapon, and a beam (10) draws no inaccuracy.
//!
//! The player's armament ([`Arsenal::player`]) is the weapons among its
//! outfits ([`MOD_WEAPON`]), firing the rounds of the ammunition among
//! its outfits ([`MOD_AMMO`], through [`OutfitRounds`]). Its ship's stock
//! weapons and their `AmmoLoad` are outfits it owns, as in the original,
//! which keeps a weapon only through the outfit that holds it
//! (`_ShipStatsToSystemInfo` @0xcaee): [`Arsenal::stock_fits`] gives each
//! as the `oütf` of lowest ID with a [`MOD_WEAPON`] (or [`MOD_AMMO`])
//! mod naming it, `WeapCount` of each weapon, a later slot
//! naming the same weapon replacing an earlier one's count and load
//! (`_LoadObjectData` @0x7aa72, @0x7aa90), and `AmmoLoad` rounds of its
//! ammunition, whatever its `WeapCount`: the weapon its `AmmoType` names,
//! or the weapon itself for a fighter bay or any other `AmmoType`
//! (`_DoShipyardDialog` @0x5ef3e-0x5ef56), the largest load where two
//! weapons share one. A new pilot owns them before its default items
//! (`_DoNewPilot` @0x18f48, @0x18f4d); a ship bought or captured tops
//! them up after its default items (`fit_stock`).
//!
//! An NPC's ([`Arsenal::npc`]) is its class's stock weapons and default
//! items ([`Arsenal::of_class`]), its rounds held on the NPC: each stock
//! weapon's `AmmoLoad`, and the ammunition among its default items.

use std::collections::BTreeMap;

use super::ShipRef;
use super::aim::Aim;
use super::hull::{Condition, HullSpec};
use super::projectile::ShotId;
use super::report::{Reports, SimDiagnostic};
use super::weapon::{Ammo, Guidance, WeaponSpec};
use crate::catalog::{
    CombatCatalog, HullRecord, OutfitId, OutfitRecord, ShipId, WeaponId, WeaponRecord,
};
use crate::chance::Chance;
use crate::flight::normalized;
use crate::pilot::tally;
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
    /// This weapon alone, primary or secondary, whatever the triggers
    /// say: an NPC's pick. The player's trigger never sets it.
    pub only: Option<WeaponId>,
    /// Only the turrets (guidance 3, 4, 7 and 8) of what the rest of the
    /// trigger fires: an escort keeping formation.
    pub turrets_only: bool,
    /// The fighter bays too, whatever `only` says: an NPC attacking
    /// launches its fighters (see the module docs). The player's trigger
    /// never sets it.
    pub bays: bool,
}

impl Trigger {
    /// Whether it fires `weapon`: a fighter bay with `bays`, as the
    /// secondary on the secondary trigger, or as `only`, never on the
    /// primary; otherwise `only` that weapon when set, or the secondary on
    /// the secondary trigger and the rest on the primary, only the turrets
    /// among them when `turrets_only`.
    #[must_use]
    pub fn fires(self, weapon: &WeaponSpec) -> bool {
        if weapon.is_bay() {
            return self.bays || self.only == Some(weapon.id) || self.secondary == Some(weapon.id);
        }
        if let Some(only) = self.only {
            return weapon.id == only;
        }
        if self.turrets_only && !weapon.guidance.turret() {
            return false;
        }
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
    /// Takes a round of `ammo` back: a fighter docking with its bay.
    fn stow(&mut self, ammo: WeaponId);
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

    fn stow(&mut self, ammo: WeaponId) {
        *self.entry(ammo).or_default() += 1;
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
        outfit_rounds(self.owned, self.sources, ammo)
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

    /// Adds one of its outfit of lowest ID; none when no outfit is its
    /// rounds.
    fn stow(&mut self, ammo: WeaponId) {
        let lowest = self
            .sources
            .iter()
            .filter(|&&(of, _)| of == ammo)
            .map(|&(_, outfit)| outfit)
            .min();
        if let Some(outfit) = lowest {
            let count = self.owned.entry(outfit).or_default();
            *count = count.saturating_add(1);
        }
    }
}

/// The rounds of `ammo` among `owned` outfits: each ammunition outfit
/// among `sources` that is its rounds, counted.
#[must_use]
pub fn outfit_rounds(
    owned: &BTreeMap<OutfitId, u16>,
    sources: &[(WeaponId, OutfitId)],
    ammo: WeaponId,
) -> u32 {
    sources
        .iter()
        .filter(|&&(of, _)| of == ammo)
        .map(|(_, outfit)| u32::from(owned.get(outfit).copied().unwrap_or(0)))
        .sum()
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
    #[must_use]
    pub fn affords(&self, rounds: &dyn Rounds, fuel: Gauge) -> bool {
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

    /// Fires a shot: reports the flags it ignores to `reports`, counts it
    /// in the burst, pays for it from `rounds` and `fuel` when it spends,
    /// and sets the reload (see the module docs).
    fn discharge(&mut self, rounds: &mut dyn Rounds, fuel: &mut Gauge, reports: &mut Reports) {
        reports.fired(&self.spec);
        if self.spec.burst_count > 0 {
            self.burst += 1;
        }
        if self.spends() {
            self.pay(rounds, fuel);
        }
        self.reload = self.spec.reload / self.count as f32;
        if self.spec.burst_count > 0 && self.burst >= self.spec.burst_count * self.count {
            self.burst = 0;
            self.reload = self.spec.burst_reload;
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
    /// The heading it leaves along, its inaccuracy included, in degrees.
    pub heading: f32,
    /// The ship it is fired at, if any.
    pub target: Option<ShipRef>,
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

    /// The secondary weapons carried, in the order they were mounted.
    pub fn secondaries(&self) -> impl Iterator<Item = &WeaponSpec> {
        secondaries(&self.mounts)
    }

    /// Fires every weapon `trigger` picks that is ready and that `aim`
    /// aims, on a ship in `condition`, paying from `rounds` and `fuel`, the
    /// inaccuracy drawn on `chance`, and gives what it launched; what it
    /// reports goes to `reports` (see the module docs).
    // Each is a separate borrow the fight holds: the ship's command, its
    // condition and stores, the dice, the reports and its aim.
    #[allow(clippy::too_many_arguments)]
    pub fn fire(
        &mut self,
        trigger: Trigger,
        condition: Condition,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        aim: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
    ) -> Vec<Launch> {
        let mut launches = Vec::new();
        if condition != Condition::Intact {
            return launches;
        }
        let launching = self.launching_bay(trigger, rounds, *fuel);
        for mount in &mut self.mounts {
            if !trigger.fires(&mount.spec) || mount.reload > 0.0 {
                continue;
            }
            if mount.spec.is_bay() && trigger.bays && launching != Some(mount.spec.id) {
                continue;
            }
            if let Guidance::Other(guidance) = mount.spec.guidance {
                reports.report(SimDiagnostic::UnimplementedGuidance {
                    weapon: mount.spec.id,
                    guidance,
                });
                continue;
            }
            let Some(aimed) = aim(&mount.spec) else {
                continue;
            };
            if !mount.affords(rounds, *fuel) {
                continue;
            }
            let offset = inaccuracy(mount.spec.inaccuracy, chance);
            mount.discharge(rounds, fuel, reports);
            launches.push(Launch {
                weapon: mount.spec,
                heading: normalized(aimed.heading + offset),
                target: aimed.target,
            });
        }
        launches
    }

    /// Fires the point defence of a ship in `condition`, paying from
    /// `rounds` and `fuel`, the inaccuracy drawn on `chance`: the ready
    /// point-defence weapon it can pay for with the lowest ID, at the
    /// heading and missile `pick` gives it, if any. Nothing fires, and
    /// nothing is spent, when `pick` gives none. What it reports goes to
    /// `reports`.
    pub fn fire_point_defence(
        &mut self,
        condition: Condition,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        pick: &mut dyn FnMut(&WeaponSpec) -> Option<(f32, ShotId)>,
    ) -> Option<(Launch, ShotId)> {
        if condition != Condition::Intact {
            return None;
        }
        let mount = self
            .mounts
            .iter_mut()
            .filter(|mount| {
                matches!(
                    mount.spec.guidance,
                    Guidance::PointDefence | Guidance::PointDefenceBeam
                ) && mount.reload <= 0.0
                    && mount.affords(rounds, *fuel)
            })
            .min_by_key(|mount| mount.spec.id)?;
        let (bearing, missile) = pick(&mount.spec)?;
        let offset = if mount.spec.guidance == Guidance::PointDefenceBeam {
            0.0
        } else {
            inaccuracy(mount.spec.inaccuracy, chance)
        };
        mount.discharge(rounds, fuel, reports);
        let launch = Launch {
            weapon: mount.spec,
            heading: normalized(bearing + offset),
            target: None,
        };
        Some((launch, missile))
    }

    /// The bay `trigger` launches from with [`Trigger::bays`]: the one of
    /// lowest weapon ID holding rounds, ready or not; none without
    /// `bays`.
    fn launching_bay(
        &self,
        trigger: Trigger,
        rounds: &dyn Rounds,
        fuel: Gauge,
    ) -> Option<WeaponId> {
        if !trigger.bays {
            return None;
        }
        self.mounts
            .iter()
            .filter(|mount| mount.spec.is_bay() && mount.affords(rounds, fuel))
            .map(|mount| mount.spec.id)
            .min()
    }

    /// Takes a fighter of ship type `ship` aboard, a round of the first
    /// bay that launches it, into `rounds` (see the module docs), and says
    /// whether one did; with none, nothing changes.
    pub fn stow(&mut self, ship: ShipId, rounds: &mut dyn Rounds) -> bool {
        let Some(mount) = self
            .mounts
            .iter_mut()
            .find(|mount| mount.spec.carried == Some(ship))
        else {
            return false;
        };
        if rounds.held(mount.spec.id) == 0 {
            mount.reload = mount.reload.max(mount.spec.reload);
        }
        rounds.stow(mount.spec.id);
        true
    }

    /// Counts every weapon's reload timer down a tick.
    pub fn reload(&mut self) {
        for mount in &mut self.mounts {
            mount.reload = (mount.reload - 1.0).max(0.0);
        }
    }
}

/// The secondary among `mounts` that selecting one after `current` picks,
/// `backwards` or not: the next in mount order, wrapping, from the first
/// (or the last, `backwards`) when `current` is none or not among them.
/// A weapon hidden when out of ammo ([`WeaponSpec::hides_when_empty`]) is
/// skipped while `rounds_of` its ammunition is none; none when no
/// secondary is left to pick.
pub fn next_secondary(
    mounts: &[Mount],
    current: Option<WeaponId>,
    backwards: bool,
    rounds_of: impl Fn(WeaponId) -> u32,
) -> Option<WeaponId> {
    let secondaries: Vec<&WeaponSpec> = secondaries(mounts).collect();
    let count = secondaries.len();
    let at = current.and_then(|id| secondaries.iter().position(|spec| spec.id == id));
    let pickable = |spec: &WeaponSpec| match spec.ammo {
        Ammo::Rounds(ammo) if spec.hides_when_empty() => rounds_of(ammo) > 0,
        _ => true,
    };
    (1..=count)
        .map(|step| match (at, backwards) {
            (Some(at), false) => (at + step) % count,
            (Some(at), true) => (at + count - step) % count,
            (None, false) => step - 1,
            (None, true) => count - step,
        })
        .map(|index| secondaries[index])
        .find(|spec| pickable(spec))
        .map(|spec| spec.id)
}

/// The secondary weapons among `mounts`, in order.
fn secondaries(mounts: &[Mount]) -> impl Iterator<Item = &WeaponSpec> {
    mounts
        .iter()
        .map(|mount| &mount.spec)
        .filter(|spec| spec.secondary())
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

    /// The player's armament, owning `owned` of `outfits`: its weapon
    /// outfits, its stock weapons among them (see the module docs).
    #[must_use]
    pub fn player(&self, owned: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> Armament {
        self.mounted(mods(owned, outfits, MOD_WEAPON))
    }

    /// The armament of a ship of class `ship` carrying `carried` of
    /// `outfits`: its class's stock weapons, and its weapon outfits.
    #[must_use]
    pub fn of_class(
        &self,
        ship: ShipId,
        carried: &BTreeMap<OutfitId, u16>,
        outfits: &[OutfitRecord],
    ) -> Armament {
        let stock = self.stock(ship).map(|(weapon, count, _)| (weapon, count));
        self.mounted(stock.chain(mods(carried, outfits, MOD_WEAPON)))
    }

    /// An armament of `weapons`, each with how many; one that cannot be
    /// read mounts nothing.
    fn mounted(&self, weapons: impl IntoIterator<Item = (WeaponId, u32)>) -> Armament {
        Armament::new(
            weapons
                .into_iter()
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
        let armament = self.of_class(ship, defaults, outfits);
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
        holders(outfits, MOD_AMMO)
    }

    /// Ship class `ship`'s stock weapons and their ammunition, each as
    /// the outfit among `outfits` that holds it (see the module docs);
    /// none for a class that cannot be read. The weapons come first, by
    /// ascending weapon ID, then the ammunition.
    #[must_use]
    pub fn stock_fits(&self, ship: ShipId, outfits: &[OutfitRecord]) -> Vec<StockFit> {
        let Some(hull) = self.hulls.get(&ship) else {
            return Vec::new();
        };
        let slots: BTreeMap<WeaponId, (i16, i16)> = hull
            .weapons
            .iter()
            .map(|stock| (stock.weapon, (stock.count, stock.ammo)))
            .collect();
        let weapon_holders = holders(outfits, MOD_WEAPON);
        let mut fits = Vec::new();
        let mut loads: BTreeMap<WeaponId, u16> = BTreeMap::new();
        for (&weapon, &(count, load)) in &slots {
            if let (Ok(count @ 1..), Some(outfit)) =
                (u16::try_from(count), holder(&weapon_holders, weapon))
            {
                fits.push(StockFit {
                    outfit,
                    mod_type: MOD_WEAPON,
                    weapon,
                    count,
                });
            }
            let (Ok(load @ 1..), Some(spec)) = (u16::try_from(load), self.weapon(weapon)) else {
                continue;
            };
            let ammo = match spec.ammo {
                Ammo::Rounds(ammo) => ammo,
                _ => weapon,
            };
            let most = loads.entry(ammo).or_default();
            *most = (*most).max(load);
        }
        let ammo_holders = holders(outfits, MOD_AMMO);
        for (ammo, load) in loads {
            if let Some(outfit) = holder(&ammo_holders, ammo) {
                fits.push(StockFit {
                    outfit,
                    mod_type: MOD_AMMO,
                    weapon: ammo,
                    count: load,
                });
            }
        }
        fits
    }

    /// Ship class `ship`'s stock weapons and their ammunition as outfits
    /// among `outfits` ([`Arsenal::stock_fits`]), each with how many,
    /// saturating.
    #[must_use]
    pub fn stock_outfits(&self, ship: ShipId, outfits: &[OutfitRecord]) -> BTreeMap<OutfitId, u16> {
        fitted(&self.stock_fits(ship, outfits))
    }
}

/// The outfits of `fits`, each with how many, saturating.
#[must_use]
pub fn fitted(fits: &[StockFit]) -> BTreeMap<OutfitId, u16> {
    tally(fits.iter().map(|fit| (fit.outfit, fit.count)))
}

/// One of a ship class's stock weapons, or its ammunition, as the `oütf`
/// that holds it: the outfit of lowest ID whose mod of `mod_type`
/// ([`MOD_WEAPON`] or [`MOD_AMMO`]) names the weapon, as
/// `_ShipStatsToSystemInfo` picks (@0xcc28-0xccd7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StockFit {
    /// The outfit holding it.
    pub outfit: OutfitId,
    /// [`MOD_WEAPON`] for the weapon, [`MOD_AMMO`] for its ammunition.
    pub mod_type: i16,
    /// The weapon, or the ammunition's weapon.
    pub weapon: WeaponId,
    /// Its `WeapCount`, or its `AmmoLoad`.
    pub count: u16,
}

/// Tops `owned` of `outfits` up to each of `fits`, as `_DoShipyardDialog`
/// does (@0x5eebc-0x5ef92): when the outfits owned hold fewer of a fit's
/// weapon (or rounds) than its count, the fit's outfit makes up the
/// difference; when they hold as many or more, nothing changes.
pub(crate) fn fit_stock(
    owned: &mut BTreeMap<OutfitId, u16>,
    fits: &[StockFit],
    outfits: &[OutfitRecord],
) {
    for fit in fits {
        let held: u32 = mods(owned, outfits, fit.mod_type)
            .into_iter()
            .filter(|&(weapon, _)| weapon == fit.weapon)
            .map(|(_, count)| count)
            .sum();
        let held = u16::try_from(held).unwrap_or(u16::MAX);
        if fit.count > held {
            let count = owned.entry(fit.outfit).or_default();
            *count = count.saturating_add(fit.count - held);
        }
    }
}

/// Each outfit among `outfits` with a mod of `mod_type`, with the `wëap`
/// it names, by ascending outfit ID.
fn holders(outfits: &[OutfitRecord], mod_type: i16) -> Vec<(WeaponId, OutfitId)> {
    let mut sorted: Vec<&OutfitRecord> = outfits.iter().collect();
    sorted.sort_by_key(|record| record.id);
    sorted
        .into_iter()
        .flat_map(|record| {
            record
                .mods
                .iter()
                .filter(move |&&(kind, _)| kind == mod_type)
                .map(|&(_, weapon)| (WeaponId(weapon), record.id))
        })
        .collect()
}

/// The first outfit among `holders` naming `weapon`: the lowest ID.
fn holder(holders: &[(WeaponId, OutfitId)], weapon: WeaponId) -> Option<OutfitId> {
    holders
        .iter()
        .find(|&&(of, _)| of == weapon)
        .map(|&(_, outfit)| outfit)
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
    use super::super::weapon::HIDE_WHEN_EMPTY;
    use super::*;
    use crate::catalog::StockWeapon;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::flags::FlagField;
    use crate::flight::ShipState;
    use crate::testkit::{Draws, hull, outfit, weapon};
    use crate::traffic::npc::NpcId;

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
        only: None,
        turrets_only: false,
        bays: false,
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
                &mut ahead,
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

    /// Every weapon fires straight up, at nothing.
    #[allow(clippy::unnecessary_wraps)]
    fn ahead(_spec: &WeaponSpec) -> Option<Aim> {
        Some(Aim {
            heading: 0.0,
            target: None,
        })
    }

    // Selecting the secondary.

    /// A secondary weapon `id` with `flags2`, firing the rounds of `wëap`
    /// `128 + ammo_type` (-1: unlimited).
    fn secondary(id: i16, ammo_type: i16, flags2: u16) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            flags: super::super::weapon::SECONDARY,
            ammo_type,
            flags2,
            ..weapon(id)
        })
    }

    /// A blaster (128), then secondaries 140, 141 and 142, of which 141
    /// fires the rounds of 150 and hides when it has none.
    fn armed() -> Armament {
        Armament::new([
            (WeaponSpec::new(&blaster(128, 0)), 1),
            (secondary(140, -1, 0), 1),
            (secondary(141, 22, HIDE_WHEN_EMPTY), 2),
            (secondary(142, -1, 0), 1),
        ])
    }

    fn ids(ids: &[i16]) -> Vec<WeaponId> {
        ids.iter().copied().map(WeaponId).collect()
    }

    #[test]
    fn the_secondaries_are_the_secondary_weapons_in_mount_order() {
        let secondaries: Vec<WeaponId> = armed().secondaries().map(|spec| spec.id).collect();
        assert_eq!(secondaries, ids(&[140, 141, 142]));
        assert_eq!(mounted(blaster(128, 0), 1).secondaries().count(), 0);
    }

    /// The secondaries selected in turn from `start`, `backwards` or not,
    /// `rounds` rounds of 150 held.
    fn cycle(start: Option<i16>, backwards: bool, rounds: u32, turns: usize) -> Vec<WeaponId> {
        let armament = armed();
        let mut current = start.map(WeaponId);
        let mut picked = Vec::new();
        for _ in 0..turns {
            current = next_secondary(armament.mounts(), current, backwards, |ammo| {
                if ammo == WeaponId(150) { rounds } else { 0 }
            });
            picked.extend(current);
        }
        picked
    }

    #[test]
    fn the_next_secondary_is_in_mount_order_and_wraps() {
        assert_eq!(cycle(None, false, 5, 4), ids(&[140, 141, 142, 140]));
        assert_eq!(cycle(Some(141), false, 5, 2), ids(&[142, 140]));
    }

    #[test]
    fn backwards_the_secondary_goes_the_other_way_and_wraps() {
        assert_eq!(cycle(None, true, 5, 4), ids(&[142, 141, 140, 142]));
        assert_eq!(cycle(Some(140), true, 5, 1), ids(&[142]));
    }

    #[test]
    fn a_secondary_that_hides_when_empty_is_skipped_without_rounds() {
        assert_eq!(cycle(None, false, 0, 3), ids(&[140, 142, 140]));
        assert_eq!(cycle(Some(141), false, 0, 1), ids(&[142]), "from it");
        assert_eq!(cycle(None, true, 0, 2), ids(&[142, 140]));
        assert_eq!(cycle(None, false, 1, 2), ids(&[140, 141]), "a round");
    }

    #[test]
    fn a_secondary_out_of_rounds_that_does_not_hide_is_still_selected() {
        let armament = Armament::new([(secondary(140, -1, 0), 1), (secondary(141, 22, 0), 1)]);
        assert_eq!(
            next_secondary(armament.mounts(), Some(WeaponId(140)), false, |_| 0),
            Some(WeaponId(141))
        );
    }

    #[test]
    fn a_secondary_no_longer_carried_starts_the_cycle_again() {
        assert_eq!(cycle(Some(199), false, 5, 1), ids(&[140]));
        assert_eq!(cycle(Some(128), true, 5, 1), ids(&[142]));
    }

    #[test]
    fn without_secondaries_there_is_none() {
        let primaries = mounted(blaster(128, 0), 1);
        assert_eq!(next_secondary(primaries.mounts(), None, false, |_| 0), None);
        assert_eq!(
            next_secondary(primaries.mounts(), Some(WeaponId(140)), true, |_| 0),
            None
        );
        let only_empty = Armament::new([(secondary(141, 22, HIDE_WHEN_EMPTY), 1)]);
        assert_eq!(
            next_secondary(only_empty.mounts(), None, false, |_| 0),
            None
        );
        let only_one = Armament::new([(secondary(140, -1, 0), 1)]);
        assert_eq!(
            next_secondary(only_one.mounts(), Some(WeaponId(140)), false, |_| 0),
            Some(WeaponId(140))
        );
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
        assert_eq!(unburst.mounts()[0].burst, 0, "no burst to count");
        assert_eq!(one.mounts()[0].burst, 1, "the 48th tick's shot");
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
            only: None,
            turrets_only: false,
            bays: false,
        };
        assert_eq!(fired(&mut armament, second), [140]);
        let both = Trigger {
            primary: true,
            secondary: Some(WeaponId(138)),
            only: None,
            turrets_only: false,
            bays: false,
        };
        assert_eq!(fired(&mut armament, both), [128, 138, 129]);
        let primary_as_secondary = Trigger {
            primary: false,
            secondary: Some(WeaponId(128)),
            only: None,
            turrets_only: false,
            bays: false,
        };
        assert_eq!(
            fired(&mut armament, primary_as_secondary),
            Vec::<i16>::new()
        );
        assert_eq!(fired(&mut armament, Trigger::default()), Vec::<i16>::new());
        for (only, alone) in [(140, 140), (129, 129)] {
            let trigger = Trigger {
                only: Some(WeaponId(only)),
                ..both
            };
            assert_eq!(
                fired(&mut armament, trigger),
                [alone],
                "that weapon alone, primary or secondary"
            );
        }
        let missing = Trigger {
            only: Some(WeaponId(999)),
            ..both
        };
        assert_eq!(fired(&mut armament, missing), Vec::<i16>::new());
    }

    #[test]
    fn turrets_only_fires_the_turrets_alone() {
        let guided = |id: i16, guidance: i16| WeaponRecord {
            guidance,
            ..blaster(id, 0)
        };
        let records = [
            blaster(128, 0),
            guided(129, 4),
            guided(130, 7),
            guided(131, 3),
            guided(132, 8),
            guided(133, 0),
            WeaponRecord {
                guidance: 4,
                flags: super::super::weapon::SECONDARY,
                ..blaster(140, 0)
            },
        ];
        let armament = Armament::new(records.iter().map(|record| (WeaponSpec::new(record), 1)));
        let fires = |trigger: Trigger| -> Vec<i16> {
            armament
                .mounts()
                .iter()
                .filter(|mount| trigger.fires(&mount.spec))
                .map(|mount| mount.spec.id.0)
                .collect()
        };
        let turrets = Trigger {
            primary: true,
            turrets_only: true,
            ..Trigger::default()
        };
        assert_eq!(
            fires(turrets),
            [129, 130, 131, 132],
            "no gun, beam or secondary"
        );
        assert_eq!(
            fires(Trigger {
                secondary: Some(WeaponId(140)),
                ..turrets
            }),
            [129, 130, 131, 132, 140],
            "a secondary turret on its own trigger"
        );
        assert_eq!(fires(PRIMARY), [128, 129, 130, 131, 132, 133]);
        assert!(!Trigger::default().turrets_only);
    }

    /// Every weapon fires at 100 degrees, at NPC 7.
    #[allow(clippy::unnecessary_wraps)]
    fn at_seven(_spec: &WeaponSpec) -> Option<Aim> {
        Some(Aim {
            heading: 100.0,
            target: Some(SEVEN),
        })
    }

    const SEVEN: ShipRef = ShipRef::Npc(NpcId(7));

    #[test]
    fn inaccuracy_draws_twice_it_and_turns_the_aim_by_minus_it_up_to_one_less() {
        let wild = WeaponRecord {
            inaccuracy: 9,
            ..blaster(128, 0)
        };
        for (draw, heading) in [(0, 91.0), (9, 100.0), (17, 108.0)] {
            let mut armament = mounted(wild, 1);
            let mut chance = Draws::of(&[draw]);
            let launches = armament.fire(
                PRIMARY,
                Condition::Intact,
                &mut BTreeMap::new(),
                &mut Gauge::default(),
                &mut chance,
                &mut Reports::default(),
                &mut at_seven,
            );
            assert_eq!(launches[0].heading, heading, "{draw}");
            assert_eq!(launches[0].target, Some(SEVEN), "{draw}");
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
            &mut at_seven,
        );
        assert_eq!(launches[0].heading, 100.0);
        assert!(chance.asked.is_empty(), "never asked");
        let mut wrapping = mounted(wild, 1);
        let launches = wrapping.fire(
            PRIMARY,
            Condition::Intact,
            &mut BTreeMap::new(),
            &mut Gauge::default(),
            &mut Draws::of(&[0]),
            &mut Reports::default(),
            &mut ahead,
        );
        assert_eq!(launches[0].heading, 351.0, "wrapped");
    }

    #[test]
    fn a_weapon_with_no_aim_does_not_fire_reload_or_spend_and_fires_once_aimed() {
        let turret = WeaponRecord {
            guidance: 4,
            ammo_type: 10,
            reload: 10,
            ..blaster(139, 10)
        };
        let mut armament = mounted(turret, 1);
        let mut rounds = BTreeMap::from([(WeaponId(138), 3)]);
        let mut reports = Reports::default();
        let mut asked = Vec::new();
        let mut unaimed = |spec: &WeaponSpec| {
            asked.push(spec.id);
            None
        };
        let launches = armament.fire(
            PRIMARY,
            Condition::Intact,
            &mut rounds,
            &mut Gauge::default(),
            &mut NeverFires,
            &mut reports,
            &mut unaimed,
        );
        assert_eq!(launches, []);
        assert_eq!(asked, [WeaponId(139)]);
        assert_eq!(armament.mounts()[0].reload, 0.0, "no reload");
        assert_eq!(rounds[&WeaponId(138)], 3, "no round spent");
        let launches = armament.fire(
            PRIMARY,
            Condition::Intact,
            &mut rounds,
            &mut Gauge::default(),
            &mut NeverFires,
            &mut reports,
            &mut at_seven,
        );
        assert_eq!(launches.len(), 1, "fired the next tick, aimed");
        assert_eq!(rounds[&WeaponId(138)], 2);
        assert_eq!(armament.mounts()[0].reload, 10.0);
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
                &mut ahead,
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
        let bay = |guidance| WeaponRecord {
            guidance,
            flags: 0x0040,
            ..blaster(131, 0)
        };
        for guidance in [99, 2] {
            let mut armament = mounted(bay(guidance), 1);
            let mut supplies = Supplies::none();
            assert_eq!(supplies.firing(&mut armament, PRIMARY, 5), [0_u32; 0]);
            assert_eq!(
                supplies.reports.take(),
                [SimDiagnostic::UnimplementedGuidance {
                    weapon: WeaponId(131),
                    guidance
                }],
                "its flags never reported, as it never fires"
            );
        }
        let mut armament = mounted(bay(99), 1);
        let mut supplies = Supplies::none();
        supplies.firing(&mut armament, PRIMARY, 1);
        supplies.reports.take();
        assert_eq!(
            supplies.firing(&mut armament, Trigger::default(), 1),
            [0_u32; 0]
        );
        assert_eq!(supplies.reports.take(), [], "nor when not fired");
    }

    #[test]
    fn guided_weapons_and_turrets_are_flown_and_not_reported() {
        for guidance in [1, 3, 4, 7, 8] {
            let mut armament = mounted(
                WeaponRecord {
                    guidance,
                    ..blaster(131, 0)
                },
                1,
            );
            let mut supplies = Supplies::none();
            assert_eq!(
                supplies.firing(&mut armament, PRIMARY, 2),
                [0, 1],
                "{guidance}"
            );
            assert_eq!(supplies.reports.take(), [], "{guidance}");
        }
    }

    // Point defence.

    /// A point-defence weapon `id` of `guidance` reloading every `reload`
    /// ticks, `inaccuracy` degrees off.
    fn defence(id: i16, guidance: i16, reload: i16, inaccuracy: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            inaccuracy,
            ..blaster(id, reload)
        }
    }

    /// Fires `armament`'s point defence on `chance`, `pick` giving the
    /// missile to fire at, recording each weapon it is asked of.
    fn defend(
        armament: &mut Armament,
        supplies: &mut Supplies,
        chance: &mut dyn Chance,
        pick: Option<(f32, ShotId)>,
        asked: &mut Vec<WeaponId>,
    ) -> Option<(Launch, ShotId)> {
        armament.fire_point_defence(
            Condition::Intact,
            &mut supplies.rounds,
            &mut supplies.fuel,
            chance,
            &mut supplies.reports,
            &mut |spec: &WeaponSpec| {
                asked.push(spec.id);
                pick
            },
        )
    }

    #[test]
    fn point_defence_fires_its_lowest_ready_weapon_at_the_pick() {
        let mut armament = Armament::new(
            [
                blaster(128, 0),
                defence(161, 9, 4, 0),
                defence(133, 9, 5, 0),
                defence(150, 10, 3, 0),
            ]
            .iter()
            .map(|record| (WeaponSpec::new(record), 1)),
        );
        let mut supplies = Supplies::none();
        let mut asked = Vec::new();
        let missile = Some((30.0, ShotId(4)));
        let (launch, shot) = defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            missile,
            &mut asked,
        )
        .expect("fires");
        assert_eq!(asked, [WeaponId(133)], "the lowest ID, asked alone");
        assert_eq!(
            (launch.weapon.id, launch.heading, launch.target),
            (WeaponId(133), 30.0, None)
        );
        assert_eq!(shot, ShotId(4));
        let reloads: Vec<f32> = armament.mounts().iter().map(|mount| mount.reload).collect();
        assert_eq!(reloads, [0.0, 0.0, 5.0, 0.0], "its reload");
        asked.clear();
        let next = defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            missile,
            &mut asked,
        );
        assert_eq!(
            next.map(|(launch, _)| launch.weapon.id),
            Some(WeaponId(150))
        );
        assert_eq!(asked, [WeaponId(150)], "the next ready");
        asked.clear();
        defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            missile,
            &mut asked,
        );
        defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            missile,
            &mut asked,
        );
        assert_eq!(asked, [WeaponId(161)], "and none ready after");
    }

    #[test]
    fn point_defence_with_nothing_to_shoot_fires_and_spends_nothing() {
        let mut armament = mounted(
            WeaponRecord {
                ammo_type: 27,
                burst_count: 3,
                burst_reload: 10,
                ..defence(161, 9, 4, 20)
            },
            1,
        );
        let mut supplies = Supplies::none();
        supplies.rounds.insert(WeaponId(155), 2);
        let mut chance = Draws::of(&[]);
        let mut asked = Vec::new();
        assert_eq!(
            defend(&mut armament, &mut supplies, &mut chance, None, &mut asked),
            None
        );
        assert_eq!(asked, [WeaponId(161)]);
        assert_eq!(armament.mounts()[0].reload, 0.0);
        assert_eq!(armament.mounts()[0].burst, 0);
        assert_eq!(supplies.rounds[&WeaponId(155)], 2);
        assert!(chance.asked.is_empty(), "no inaccuracy drawn");
        let mut chance = Draws::of(&[0]);
        let (launch, _) = defend(
            &mut armament,
            &mut supplies,
            &mut chance,
            Some((90.0, ShotId(1))),
            &mut asked,
        )
        .expect("fires");
        assert_eq!(launch.heading, 70.0, "its inaccuracy");
        assert_eq!(chance.asked, [40]);
        assert_eq!(supplies.rounds[&WeaponId(155)], 1, "a round");
        assert_eq!(armament.mounts()[0].burst, 1, "a shot of its burst");
        assert_eq!(armament.mounts()[0].reload, 4.0);
        supplies.rounds.insert(WeaponId(155), 0);
        armament.reload();
        armament.reload();
        armament.reload();
        armament.reload();
        asked.clear();
        assert_eq!(
            defend(
                &mut armament,
                &mut supplies,
                &mut NeverFires,
                Some((90.0, ShotId(1))),
                &mut asked
            ),
            None,
            "out of rounds"
        );
        assert_eq!(asked, [0_i16; 0].map(WeaponId), "not asked");
    }

    #[test]
    fn a_point_defence_beam_draws_no_inaccuracy() {
        let mut armament = mounted(defence(150, 10, 3, 20), 1);
        let mut chance = Draws::of(&[]);
        let (launch, _) = defend(
            &mut armament,
            &mut Supplies::none(),
            &mut chance,
            Some((45.0, ShotId(2))),
            &mut Vec::new(),
        )
        .expect("fires");
        assert_eq!(launch.heading, 45.0);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn point_defence_fires_only_on_an_intact_ship_and_only_point_defence() {
        let mut armament = mounted(defence(133, 9, 5, 0), 1);
        let mut supplies = Supplies::none();
        for condition in [Condition::Disabled, Condition::Dying { ticks_left: 3 }] {
            let fired = armament.fire_point_defence(
                condition,
                &mut supplies.rounds,
                &mut supplies.fuel,
                &mut NeverFires,
                &mut supplies.reports,
                &mut |_: &WeaponSpec| Some((0.0, ShotId(0))),
            );
            assert_eq!(fired, None, "{condition:?}");
        }
        for guidance in [-1, 0, 1, 4] {
            let mut other = mounted(defence(133, guidance, 5, 0), 1);
            let mut asked = Vec::new();
            let fired = defend(
                &mut other,
                &mut supplies,
                &mut NeverFires,
                Some((0.0, ShotId(0))),
                &mut asked,
            );
            assert_eq!(fired, None, "{guidance}");
            assert_eq!(asked, [0_i16; 0].map(WeaponId), "{guidance}");
        }
    }

    #[test]
    fn point_defence_reports_its_flags_as_it_fires() {
        let mut armament = mounted(
            WeaponRecord {
                flags2: 0x8000,
                ..defence(133, 9, 5, 0)
            },
            1,
        );
        let mut supplies = Supplies::none();
        defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            None,
            &mut Vec::new(),
        );
        assert_eq!(supplies.reports.take(), [], "not before it fires");
        defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            Some((0.0, ShotId(0))),
            &mut Vec::new(),
        );
        assert_eq!(supplies.reports.take().len(), 1);
    }

    #[test]
    fn the_triggers_never_fire_point_defence() {
        for guidance in [9, 10] {
            let mut armament = mounted(defence(133, guidance, 0, 0), 1);
            let mut supplies = Supplies::none();
            let mut asked = Vec::new();
            let launches = armament.fire(
                PRIMARY,
                Condition::Intact,
                &mut supplies.rounds,
                &mut supplies.fuel,
                &mut NeverFires,
                &mut supplies.reports,
                &mut |spec: &WeaponSpec| {
                    asked.push(spec.id);
                    super::super::aim::aim(spec, &ShipState::default(), &HullSpec::default(), None)
                },
            );
            assert_eq!(launches, [], "{guidance}");
            assert_eq!(armament.mounts()[0].reload, 0.0, "{guidance}");
        }
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
            &[
                blaster(128, 10),
                rocket,
                blaster(150, 5),
                WeaponRecord {
                    ammo_type: 12,
                    ..blaster(140, 50)
                },
            ],
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
                    StockWeapon {
                        weapon: WeaponId(140),
                        count: 0,
                        ammo: 0,
                    },
                ],
                size: Some(30),
                ..hull(128)
            }],
        )
    }

    /// Outfit 200 is a weapon, 150; outfit 201 is rockets; outfit 202 is
    /// a shield booster; outfits 205 and 206 are blasters (128), 207 a
    /// rocket launcher (138), and 208 weapon 999, which cannot be read.
    fn outfits() -> Vec<OutfitRecord> {
        vec![
            outfit(200, &[(MOD_WEAPON, 150)]),
            outfit(201, &[(MOD_AMMO, 138)]),
            outfit(202, &[(4, 100)]),
            outfit(205, &[(MOD_WEAPON, 128)]),
            outfit(206, &[(MOD_WEAPON, 128)]),
            outfit(207, &[(MOD_WEAPON, 138)]),
            outfit(208, &[(MOD_WEAPON, 999)]),
        ]
    }

    /// An arsenal of blasters (128), a rocket launcher (138) and
    /// `weapons`, with ship 130 stocking `slots`, each a weapon with its
    /// `WeapCount` and `AmmoLoad`.
    fn stocking(weapons: &[WeaponRecord], slots: &[(i16, i16, i16)]) -> Arsenal {
        let mut records = vec![
            blaster(128, 10),
            WeaponRecord {
                ammo_type: 10,
                ..blaster(138, 15)
            },
        ];
        records.extend_from_slice(weapons);
        let weapons = slots
            .iter()
            .map(|&(weapon, count, ammo)| StockWeapon {
                weapon: WeaponId(weapon),
                count,
                ammo,
            })
            .collect();
        Arsenal::new(
            &records,
            vec![HullRecord {
                weapons,
                ..hull(130)
            }],
        )
    }

    fn tallied(pairs: &[(i16, u16)]) -> BTreeMap<OutfitId, u16> {
        pairs.iter().map(|&(id, n)| (OutfitId(id), n)).collect()
    }

    #[test]
    fn a_classs_stock_weapons_are_held_by_the_lowest_id_outfit_naming_them() {
        let arsenal = arsenal();
        assert_eq!(
            arsenal.stock_outfits(ShipId(128), &outfits()),
            tallied(&[(201, 20), (205, 2), (207, 1), (208, 1)]),
            "999 is fitted, but its rounds are not: it has no wëap; \
             150 (-1, -4) and 140 (0) give nothing"
        );
        assert_eq!(arsenal.stock_outfits(ShipId(129), &outfits()), tallied(&[]));
        assert_eq!(
            arsenal.stock_fits(ShipId(128), &outfits()),
            [
                StockFit {
                    outfit: OutfitId(205),
                    mod_type: MOD_WEAPON,
                    weapon: WeaponId(128),
                    count: 2,
                },
                StockFit {
                    outfit: OutfitId(207),
                    mod_type: MOD_WEAPON,
                    weapon: WeaponId(138),
                    count: 1,
                },
                StockFit {
                    outfit: OutfitId(208),
                    mod_type: MOD_WEAPON,
                    weapon: WeaponId(999),
                    count: 1,
                },
                StockFit {
                    outfit: OutfitId(201),
                    mod_type: MOD_AMMO,
                    weapon: WeaponId(138),
                    count: 20,
                },
            ]
        );
    }

    #[test]
    fn a_weapon_named_twice_keeps_its_later_slot() {
        let arsenal = stocking(&[], &[(128, 1, 0), (128, 3, 0)]);
        assert_eq!(
            arsenal.stock_outfits(ShipId(130), &outfits()),
            tallied(&[(205, 3)])
        );
        let rockets = stocking(&[], &[(138, 1, 20), (138, 1, 5)]);
        assert_eq!(
            rockets.stock_outfits(ShipId(130), &outfits()),
            tallied(&[(201, 5), (207, 1)]),
            "its load too"
        );
    }

    #[test]
    fn stock_ammunition_is_loaded_without_a_weapon_count_and_shared_ammunition_takes_the_larger_load()
     {
        let launcher = |id| WeaponRecord {
            ammo_type: 10,
            ..blaster(id, 15)
        };
        let shared = stocking(
            &[launcher(160), launcher(161)],
            &[(160, 1, 7), (161, 1, 12)],
        );
        assert_eq!(
            shared.stock_outfits(ShipId(130), &outfits()),
            tallied(&[(201, 12)]),
            "neither launcher has an outfit of its own"
        );
        let larger_first = stocking(
            &[launcher(160), launcher(161)],
            &[(160, 1, 12), (161, 1, 7)],
        );
        assert_eq!(
            larger_first.stock_outfits(ShipId(130), &outfits()),
            tallied(&[(201, 12)])
        );
        let uncounted = stocking(&[], &[(138, 0, 5)]);
        assert_eq!(
            uncounted.stock_outfits(ShipId(130), &outfits()),
            tallied(&[(201, 5)])
        );
    }

    #[test]
    fn an_unlimited_or_bay_weapons_load_is_its_own_ammunition() {
        let unlimited = WeaponRecord {
            ammo_type: -1,
            ..blaster(170, 15)
        };
        let fuelled = WeaponRecord {
            ammo_type: -1005,
            ..blaster(172, 15)
        };
        let bays = stocking(
            &[unlimited, bay(171, 300, 50), fuelled],
            &[(170, 1, 3), (171, 1, 4), (172, 1, 6)],
        );
        let records = [
            outfit(209, &[(MOD_AMMO, 170)]),
            outfit(210, &[(MOD_AMMO, 171)]),
            outfit(211, &[(MOD_AMMO, 172)]),
            outfit(212, &[(MOD_AMMO, 128)]),
        ];
        assert_eq!(
            bays.stock_outfits(ShipId(130), &records),
            tallied(&[(209, 3), (210, 4), (211, 6)])
        );
    }

    #[test]
    fn stock_outfits_tally_their_fits_saturating() {
        // One outfit holding both weapons and the rockets.
        let records = [outfit(
            213,
            &[(MOD_WEAPON, 128), (MOD_WEAPON, 138), (MOD_AMMO, 138)],
        )];
        let arsenal = stocking(&[], &[(128, i16::MAX, 0), (138, i16::MAX, i16::MAX)]);
        assert_eq!(
            arsenal.stock_outfits(ShipId(130), &records),
            tallied(&[(213, u16::MAX)])
        );
        let two = stocking(&[], &[(128, 2, 0), (138, 1, 20)]);
        assert_eq!(
            two.stock_outfits(ShipId(130), &records),
            tallied(&[(213, 23)])
        );
    }

    fn counts(armament: &Armament) -> Vec<(i16, u32)> {
        armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn a_classs_armament_is_its_stock_weapons_and_its_weapon_outfits() {
        let arsenal = arsenal();
        let owned = BTreeMap::from([(OutfitId(200), 2), (OutfitId(201), 5), (OutfitId(202), 1)]);
        assert_eq!(
            counts(&arsenal.of_class(ShipId(128), &owned, &outfits())),
            [(128, 2), (138, 1), (150, 2)],
            "weapon 999 cannot be read, and none of 150 is stock"
        );
        assert_eq!(
            counts(&arsenal.of_class(ShipId(129), &owned, &outfits())),
            [(150, 2)],
            "a ship type that cannot be read has no stock weapons"
        );
        assert_eq!(MOD_WEAPON, 1);
    }

    #[test]
    fn the_players_armament_is_its_weapon_outfits_alone() {
        let arsenal = arsenal();
        let owned = BTreeMap::from([(OutfitId(200), 2), (OutfitId(201), 5), (OutfitId(202), 1)]);
        assert_eq!(
            counts(&arsenal.player(&owned, &outfits())),
            [(150, 2)],
            "its stock weapons are outfits it owns"
        );
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
    fn fit_stock_tops_up_what_is_already_held() {
        let fits = [
            StockFit {
                outfit: OutfitId(205),
                mod_type: MOD_WEAPON,
                weapon: WeaponId(128),
                count: 2,
            },
            StockFit {
                outfit: OutfitId(201),
                mod_type: MOD_AMMO,
                weapon: WeaponId(138),
                count: 20,
            },
        ];
        let mut records = outfits();
        records.push(outfit(204, &[(MOD_AMMO, 138)]));
        let topped = |owned: &[(i16, u16)]| {
            let mut owned = tallied(owned);
            fit_stock(&mut owned, &fits, &records);
            owned
        };
        assert_eq!(topped(&[]), tallied(&[(201, 20), (205, 2)]));
        assert_eq!(
            topped(&[(206, 1)]),
            tallied(&[(201, 20), (205, 1), (206, 1)]),
            "206 holds one blaster already"
        );
        assert_eq!(topped(&[(205, 3)]), tallied(&[(201, 20), (205, 3)]));
        assert_eq!(topped(&[(205, 2)]), tallied(&[(201, 20), (205, 2)]));
        assert_eq!(
            topped(&[(204, 15), (207, 4)]),
            tallied(&[(201, 5), (204, 15), (205, 2), (207, 4)]),
            "the rockets of 204 count; the launchers of 207 are no blasters"
        );
        assert_eq!(
            topped(&[(204, 25), (205, u16::MAX)]),
            tallied(&[(204, 25), (205, u16::MAX)])
        );
    }

    #[test]
    fn holders_are_in_ascending_outfit_id_whatever_order_the_records_come_in() {
        let records = [
            outfit(206, &[(MOD_WEAPON, 128)]),
            outfit(203, &[(MOD_AMMO, 140)]),
            outfit(205, &[(MOD_WEAPON, 128)]),
            outfit(201, &[(MOD_AMMO, 138)]),
        ];
        assert_eq!(
            Arsenal::ammo_outfits(&records),
            [
                (WeaponId(138), OutfitId(201)),
                (WeaponId(140), OutfitId(203))
            ]
        );
        let arsenal = stocking(&[], &[(128, 2, 0)]);
        assert_eq!(
            arsenal.stock_outfits(ShipId(130), &records),
            tallied(&[(205, 2)])
        );
    }

    #[test]
    fn the_players_rounds_are_its_ammunition_outfits() {
        let sources = [
            (WeaponId(138), OutfitId(201)),
            (WeaponId(138), OutfitId(204)),
            (WeaponId(140), OutfitId(203)),
        ];
        // An outfit owned none of, as an old save may list one.
        let mut owned = BTreeMap::from([
            (OutfitId(201), 1),
            (OutfitId(204), 2),
            (OutfitId(202), 7),
            (OutfitId(203), 0),
        ]);
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
            BTreeMap::from([(OutfitId(202), 7), (OutfitId(203), 0), (OutfitId(204), 1)]),
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

    // Fighter bays.

    /// A secondary fighter bay `id` launching ship `ship`, reloading every
    /// `reload` ticks.
    fn bay(id: i16, ship: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            guidance: 99,
            ammo_type: ship,
            flags: 0x0002,
            ..blaster(id, reload)
        }
    }

    #[test]
    fn a_bay_fires_on_the_secondary_trigger_or_with_bays_never_on_the_primary() {
        let viper = WeaponSpec::new(&bay(149, 144, 60));
        let unflagged = WeaponSpec::new(&WeaponRecord {
            flags: 0,
            ..bay(150, 145, 60)
        });
        for spec in [viper, unflagged] {
            assert!(!PRIMARY.fires(&spec), "{:?}", spec.id);
            let secondary = Trigger {
                secondary: Some(spec.id),
                ..Trigger::default()
            };
            assert!(secondary.fires(&spec));
            let bays = Trigger {
                bays: true,
                ..Trigger::default()
            };
            assert!(bays.fires(&spec), "with bays");
            let other = Trigger {
                only: Some(WeaponId(128)),
                ..bays
            };
            assert!(other.fires(&spec), "whatever only says");
            assert!(!Trigger::default().fires(&spec));
            let only = Trigger {
                only: Some(spec.id),
                ..Trigger::default()
            };
            assert!(only.fires(&spec), "picked alone");
        }
        let blaster = WeaponSpec::new(&blaster(128, 0));
        let bays = Trigger {
            bays: true,
            ..Trigger::default()
        };
        assert!(!bays.fires(&blaster), "bays fires bays alone");
    }

    /// Two ready bays, 149 and 150, mounted 150 first, of `rounds` each.
    fn two_bays(rounds: [u32; 2]) -> (Armament, Supplies) {
        let armament = Armament::new([
            (WeaponSpec::new(&bay(150, 145, 70)), 1),
            (WeaponSpec::new(&bay(149, 144, 60)), 2),
        ]);
        let mut supplies = Supplies::none();
        supplies.rounds = BTreeMap::from([(WeaponId(149), rounds[0]), (WeaponId(150), rounds[1])]);
        (armament, supplies)
    }

    const BAYS: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: true,
    };

    fn launched(launches: &[Launch]) -> Vec<i16> {
        launches.iter().map(|launch| launch.weapon.id.0).collect()
    }

    #[test]
    fn with_bays_only_the_lowest_bay_holding_rounds_launches() {
        let (mut armament, mut supplies) = two_bays([2, 2]);
        assert_eq!(launched(&supplies.tick(&mut armament, BAYS)), [149]);
        assert_eq!(supplies.rounds[&WeaponId(149)], 1, "a round spent");
        assert_eq!(supplies.rounds[&WeaponId(150)], 2);
        let launch = supplies.tick(&mut armament, BAYS);
        assert!(launch.is_empty(), "149 reloads, and 150 waits");
        let (mut armament, mut supplies) = two_bays([0, 2]);
        assert_eq!(
            launched(&supplies.tick(&mut armament, BAYS)),
            [150],
            "149 empty"
        );
        let (mut armament, mut supplies) = two_bays([0, 0]);
        assert!(supplies.tick(&mut armament, BAYS).is_empty());
    }

    #[test]
    fn a_launch_reloads_reload_over_the_bays_and_the_next_waits_for_it() {
        let (mut armament, mut supplies) = two_bays([3, 0]);
        // Reload 60 over 2 bays: a launch every 30 ticks.
        assert_eq!(supplies.firing(&mut armament, BAYS, 61), [0, 30, 60]);
        assert_eq!(supplies.rounds[&WeaponId(149)], 0);
        assert!(supplies.firing(&mut armament, BAYS, 100).is_empty());
        let (mut armament, mut supplies) = two_bays([2, 3]);
        assert_eq!(
            supplies.firing(&mut armament, BAYS, 102),
            [0, 30, 31, 101],
            "150 waits while 149 reloads with rounds, then launches every 70"
        );
        assert_eq!(supplies.rounds[&WeaponId(150)], 1);
    }

    #[test]
    fn a_secondary_bay_launches_as_any_secondary() {
        let (mut armament, mut supplies) = two_bays([0, 2]);
        let second = Trigger {
            secondary: Some(WeaponId(150)),
            ..Trigger::default()
        };
        assert_eq!(launched(&supplies.tick(&mut armament, second)), [150]);
        assert_eq!(supplies.rounds[&WeaponId(150)], 1);
        let empty = Trigger {
            secondary: Some(WeaponId(149)),
            ..Trigger::default()
        };
        assert!(supplies.tick(&mut armament, empty).is_empty(), "no rounds");
        assert_eq!(supplies.reports.take(), [], "a bay is flown, not reported");
    }

    #[test]
    fn stowing_a_fighter_puts_a_round_in_the_first_bay_launching_its_type() {
        let mut armament = Armament::new([
            (WeaponSpec::new(&blaster(128, 0)), 1),
            (WeaponSpec::new(&bay(150, 145, 70)), 1),
            (WeaponSpec::new(&bay(151, 144, 80)), 1),
            (WeaponSpec::new(&bay(149, 144, 60)), 1),
        ]);
        let mut rounds = BTreeMap::from([(WeaponId(149), 1)]);
        assert!(armament.stow(ShipId(144), &mut rounds));
        assert_eq!(
            rounds,
            BTreeMap::from([(WeaponId(149), 1), (WeaponId(151), 1)])
        );
        let reloads = |armament: &Armament| -> Vec<f32> {
            armament.mounts().iter().map(|mount| mount.reload).collect()
        };
        assert_eq!(
            reloads(&armament),
            [0.0, 0.0, 80.0, 0.0],
            "an empty bay restarts its reload"
        );
        assert!(armament.stow(ShipId(144), &mut rounds), "with no cap");
        assert_eq!(rounds[&WeaponId(151)], 2);
        assert_eq!(reloads(&armament), [0.0, 0.0, 80.0, 0.0]);
        let mut armament = Armament::new([(WeaponSpec::new(&bay(151, 144, 80)), 1)]);
        armament.mounts[0].reload = 90.0;
        let mut empty = BTreeMap::new();
        assert!(armament.stow(ShipId(144), &mut empty));
        assert_eq!(armament.mounts()[0].reload, 90.0, "a longer timer is kept");
        armament.mounts[0].reload = 5.0;
        assert!(armament.stow(ShipId(144), &mut empty));
        assert_eq!(armament.mounts()[0].reload, 5.0, "it held rounds");
        let before = armament.clone();
        let mut none = BTreeMap::new();
        assert!(!armament.stow(ShipId(146), &mut none), "no bay launches it");
        assert_eq!(armament, before);
        assert!(none.is_empty());
    }

    #[test]
    fn the_players_fighter_goes_to_the_lowest_outfit_of_its_bay() {
        let sources = [
            (WeaponId(149), OutfitId(205)),
            (WeaponId(149), OutfitId(204)),
            (WeaponId(140), OutfitId(203)),
        ];
        let mut owned = BTreeMap::from([(OutfitId(205), 1)]);
        let mut rounds = OutfitRounds {
            owned: &mut owned,
            sources: &sources,
        };
        rounds.stow(WeaponId(149));
        rounds.stow(WeaponId(149));
        rounds.stow(WeaponId(141));
        assert_eq!(
            owned,
            BTreeMap::from([(OutfitId(204), 2), (OutfitId(205), 1)]),
            "none of 141's: nowhere to stow"
        );
        let mut npc = BTreeMap::new();
        npc.stow(WeaponId(149));
        npc.stow(WeaponId(149));
        assert_eq!(npc, BTreeMap::from([(WeaponId(149), 2)]));
    }
}
