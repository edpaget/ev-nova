//! A ship's armament: the weapons it carries, how many of each, and
//! firing them when its trigger is pulled, and its point defence.
//!
//! Firing is the original's (`_FirePlayerWeapon`, `_WeaponBurstInterval`
//! and `_SpawnShot` in the `EV Nova` executable):
//!
//! - The [`Trigger`] picks the weapons: the primary trigger every weapon
//!   that is not a secondary, and the secondary trigger the one secondary
//!   selected.
//! - The player fires each weapon its trigger picks as `_HandlePlayer`
//!   calls `_FirePlayerWeapon` on it ([`Armament::player_picks`],
//!   [`Armament::fire_player`]): the primaries by ascending weapon ID, a
//!   fighter bay that is not a secondary among them, then the secondary;
//!   its copies together under `Flags` 0x0040, each attempted copy
//!   spending, its burst counting one a call, and an exclusive weapon
//!   (`Flags3` 0x0020) holding the others by its shot's reload, before its
//!   burst.
//! - On an NPC's bay launch and an escort's formation trigger
//!   ([`Armament::fire`]), a ship carrying n of a weapon fires one shot
//!   once its reload timer has run out, and sets the timer to `Reload / n`
//!   ticks; a timer counts down a tick at a time ([`Armament::reload`]),
//!   so `Reload` 0 fires every tick.
//! - An NPC fires its pick ([`Trigger::only`]) as `_FireAIShipWeapon`
//!   does instead ([`Armament::fire_pick`], [`ai_fire`]): its copies
//!   together under `Flags` 0x0040, each spending whether or not it
//!   spawns, its burst counting one a call, an exclusive weapon
//!   (`Flags3` 0x0020) holding the others, and the last gun it fired
//!   kept ([`Armament::last_gun`]).
//! - With a `BurstCount`, each copy fires that many shots (`BurstCount` x
//!   n in all) before the timer is set to `BurstReload` instead.
//! - A shot spends its ammo ([`Ammo`]): a round, or its fuel, and does
//!   not fire without it. Rounds are kept one slot for each weapon index,
//!   as the original keeps them (ship +0xd0 + i·0xc8), and [`slot`] says
//!   which a shot reads and spends: a guidance-99 weapon's own, whatever
//!   its `AmmoType`; on an NPC, a launcher's own, whatever weapon its
//!   `AmmoType` names (`_WeaponHasAmmo` @0xbab4-0xbac2, `_FireAIShipWeapon`
//!   @0x88d5f, `_HandleShipPointDefense` @0x39b8c-0x39bad); on the player,
//!   its ammunition's, the `AmmoType`'s weapon. So an NPC's 161 Storm and
//!   155 Hail keep separate rounds, though both fire 155's. An NPC's point
//!   defence of an `AmmoType` past 255 still drains its own slot. A weapon
//!   that uses ammo only at the end of a burst (`Flags3` 0x0001) spends it
//!   each time the shots in the burst reach a multiple of its
//!   `BurstCount`.
//! - Whether a ship has the ammunition for a weapon is `_WeaponHasAmmo`'s
//!   test ([`Armament::has_ammo`]), asked by every caller the original
//!   asks it from (firing, point defence, and the AI's picks in
//!   [`fire`](crate::ai::fire) and [`odds`](crate::ai::odds)): an NPC
//!   never uses a weapon of `Flags2` 0x0100, a cloaked ship only a weapon
//!   of `Flags2` 0x4000, and a weapon of `Flags2` 0x0080 needs its ship's
//!   `KeyCarried` aboard; then its rounds or fuel. An NPC's bay launch
//!   with [`Trigger::bays`] reads its bay's rounds alone
//!   (`_AILaunchFighter`).
//! - Its guidance aims it ([`aim`](super::aim)): a weapon that does not
//!   fire (a turret with no target, or one in a blind spot or out of its
//!   arc, and point defence on the trigger) spends nothing and does not
//!   reload.
//! - It leaves up to its `Inaccuracy` off its aim either way: the draw
//!   `below(2n) - n`, never asked of a weapon that is accurate.
//! - Nothing fires unless the ship is [`Condition::Intact`].
//! - A weapon of a guidance the simulation does not fly (99 without a
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
//!   @0x802a5): a round to the bay of lowest weapon ID launching its type,
//!   with no cap; a bay whose slot held exactly none restarts its reload.
//! - Point defence ([`Armament::fire_point_defence`]) fires, each tick,
//!   only the ready point-defence weapon the ship can pay for with the
//!   lowest weapon ID, and only at a missile it picks, as
//!   `_HandleShipPointDefense` does: its timer takes `Reload / n` on top
//!   of what is left of it, so a timer counted past none carries; it pays
//!   fuel every shot, and rounds every shot unless it uses ammo at the end
//!   of a burst (`Flags3` 0x0001), which pays a round each time its burst
//!   reaches `_WeaponBurstInterval` (`BurstCount`, times n unless its
//!   copies fire together, `Flags` 0x0040); a beam (10) draws no
//!   inaccuracy.
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
//! them up after its default items (`fit_stock`). The `E` and `H` set
//! operators add them to the weapon and rounds arrays instead
//! ([`Arsenal::stock_added`], `_EvalSetExp` @0x1551c-0x155de), which
//! [`ship_change`](crate::ship_change)'s fold then puts on the outfits
//! whose first mod names them: each `WeapCount` and `AmmoLoad` signed
//! by the engine, the loads sharing an ammunition summed, and each load
//! on its raw `AmmoType`'s weapon, so by the engine a fighter bay of a
//! fighter `shïp` 128 to 255 loads `wëap` 128 + that ID, which no outfit
//! holds.
//!
//! A launcher's [`Magazine`] ([`Arsenal::magazines`]) links a weapon
//! outfit to its ammunition, as the outfitter's Sell reads it
//! (`_DoOutfitDialog` @0x5ca75-0x5ccb9): the weapon its first
//! [`MOD_WEAPON`] mod names (none for `ModVal` -1), the rounds it fires
//! (its `AmmoType`'s weapon, or its own for a fighter bay or any other
//! `AmmoType`), its `MaxAmmo`, and the outfit naming its ammunition, the
//! lowest-ID one whose [`MOD_AMMO`] mod names the launcher's weapon.
//!
//! An NPC's ([`Arsenal::npc`]) is its class's stock weapons alone
//! ([`Arsenal::of_class`]), as every spawn copies the class's count and
//! load arrays (`_RandomShipSpawn` @0x3c7de-0x3c848) and only the stock
//! slots fill them (`_LoadObjectData` @0x7aa72, @0x7aa90): a later slot
//! naming the same weapon replaces an earlier one, the weapons go by
//! ascending ID, and its `DefaultItems` arm it with nothing. Its rounds
//! ([`LauncherRounds`]) are each stock slot's `AmmoLoad`, raw, on its own
//! weapon, so a load of -1 is kept. A person's own weapon slots add to
//! these ([`person::fit`](crate::person::fit)).

use std::collections::BTreeMap;

use super::ShipRef;
use super::ai_fire;
use super::aim::{Aim, Pointing};
use super::exits::ExitCursors;
use super::hull::{Condition, HullSpec};
use super::report::{Reports, SimDiagnostic};
use super::weapon::{
    AI_WONT_USE, Ammo, EXCLUSIVE, EXCLUSIVE_HOLD, FIRES_CLOAKED, FIRST_AMMO_WEAPON, Guidance,
    LAST_AMMO_TYPE, NEEDS_KEY_SHIP, SIMULTANEOUS, WeaponSpec, chain_range,
};
use crate::catalog::{
    CombatCatalog, HullRecord, OutfitId, OutfitRecord, ShipId, StockWeapon, WeaponId, WeaponRecord,
};
use crate::chance::Chance;
use crate::flight::{ShipState, normalized};
use crate::pilot::tally;
use crate::reserves::Gauge;
use crate::rulebook::RuleSource;
use crate::ship_change::Arrays;

/// The `oütf` `ModType` that is a weapon: its `ModVal` is the `wëap`.
pub const MOD_WEAPON: i16 = 1;
/// The `oütf` `ModType` that is ammunition: its `ModVal` is the `wëap` it
/// is the rounds of.
pub const MOD_AMMO: i16 = 3;

/// The fire command a ship holds: the player's keys, or an NPC's
/// [`Behaviour`](crate::ai::Behaviour).
// Each flag is its own, read on its own.
#[allow(clippy::struct_excessive_bools)]
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
    /// This tick the ship's point defence runs: the player's every tick,
    /// an NPC's when its AI asks for a turret (`_AIFireTurret` @0x80b0d).
    /// [`Trigger::fires`] ignores it.
    pub point_defence: bool,
    /// The weapon an NPC holds as its pick after its AI's passes this tick
    /// (the original's +0x72), fired or not; none when the passes left it
    /// none or didn't run. [`Trigger::fires`] ignores it: the flag is
    /// [`Trigger::only`].
    pub keep: Option<WeaponId>,
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

/// The ship paying for a shot, as `_WeaponHasAmmo` asks of it (see
/// [`Armament::has_ammo`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shooter {
    /// Whether it is an NPC (its slot not 0, @0xb96a).
    pub npc: bool,
    /// Whether it is cloaked ([`Cloak::is_cloaked`](crate::cloak::Cloak::is_cloaked),
    /// `_ShipIsCloaked` inlined @0xb9a2-0xb9ef).
    pub cloaked: bool,
    /// Its class's `KeyCarried` ([`HullSpec::key_carried`]).
    pub key_carried: Option<ShipId>,
}

impl Shooter {
    /// The player's ship of a class with no `KeyCarried`: no NPC, and
    /// never cloaked.
    pub const PLAYER: Self = Self {
        npc: false,
        cloaked: false,
        key_carried: None,
    };
}

/// Where a ship's rounds are kept, one slot for each weapon index, as the
/// original keeps them (ship +0xd0 + i·0xc8): which slot a shot reads is
/// [`slot`]'s.
pub trait Rounds {
    /// How many rounds slot `weapon` holds; none for a slot below none.
    fn held(&self, weapon: WeaponId) -> u32;
    /// Spends a round of slot `weapon`, when it holds any.
    fn spend(&mut self, weapon: WeaponId);
    /// Takes a round back into slot `weapon`: a fighter docking with its
    /// bay.
    fn stow(&mut self, weapon: WeaponId);
    /// Whether slot `weapon` holds exactly none, as a fighter docking
    /// tests it before it restarts its bay's reload (`_AIAddFighterToParent`
    /// @0x803a4): a store with no slot below none holds exactly none when it
    /// holds no rounds.
    fn none_left(&self, weapon: WeaponId) -> bool {
        self.held(weapon) == 0
    }
}

/// An NPC's rounds: one signed slot for each weapon it carries, as the
/// original keeps them on each launcher (ship +0xd0 + i·0xc8, a short,
/// here wider), whatever weapon its `AmmoType` names. A stock `AmmoLoad`
/// of -1 is kept as -1 (see the module docs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LauncherRounds(BTreeMap<WeaponId, i32>);

impl LauncherRounds {
    /// Slot `weapon`'s rounds, raw: none for a weapon it names none of.
    #[must_use]
    pub fn slot(&self, weapon: WeaponId) -> i32 {
        self.0.get(&weapon).copied().unwrap_or(0)
    }

    /// Each slot it names, by ascending weapon ID, with its rounds.
    pub fn iter(&self) -> impl Iterator<Item = (WeaponId, i32)> + '_ {
        self.0.iter().map(|(&weapon, &rounds)| (weapon, rounds))
    }

    /// Sets slot `weapon` to `rounds`.
    pub fn set(&mut self, weapon: WeaponId, rounds: i32) {
        self.0.insert(weapon, rounds);
    }

    /// Adds `rounds`, signed, to slot `weapon`.
    pub fn add(&mut self, weapon: WeaponId, rounds: i32) {
        let slot = self.0.entry(weapon).or_default();
        *slot = slot.saturating_add(rounds);
    }
}

impl<const N: usize> From<[(WeaponId, i32); N]> for LauncherRounds {
    fn from(slots: [(WeaponId, i32); N]) -> Self {
        slots.into_iter().collect()
    }
}

/// Each slot set to its rounds, a later one for the same weapon replacing
/// an earlier.
impl FromIterator<(WeaponId, i32)> for LauncherRounds {
    fn from_iter<T: IntoIterator<Item = (WeaponId, i32)>>(slots: T) -> Self {
        Self(slots.into_iter().collect())
    }
}

impl Rounds for LauncherRounds {
    fn held(&self, weapon: WeaponId) -> u32 {
        u32::try_from(self.slot(weapon)).unwrap_or(0)
    }

    fn spend(&mut self, weapon: WeaponId) {
        if let Some(slot) = self.0.get_mut(&weapon).filter(|slot| **slot > 0) {
            *slot -= 1;
        }
    }

    /// A round more, from any slot, one below none too (`incw`
    /// @0x803e6).
    fn stow(&mut self, weapon: WeaponId) {
        self.add(weapon, 1);
    }

    fn none_left(&self, weapon: WeaponId) -> bool {
        self.slot(weapon) == 0
    }
}

/// The slot of [`Rounds`] a shot of `spec` reads and spends, for an `npc`
/// or the player (`_WeaponHasAmmo` @0xba87-0xbac2): a guidance-99 weapon's
/// own, whatever its `AmmoType`, for either; a launcher's own for an NPC,
/// as the original keeps an NPC's rounds on each launcher (@0xbab4), and
/// its ammunition's, the weapon its `AmmoType` names, for the player; none
/// for a weapon that spends no rounds.
#[must_use]
pub fn slot(spec: &WeaponSpec, npc: bool) -> Option<WeaponId> {
    if spec.guidance_99() {
        return Some(spec.id);
    }
    match spec.ammo {
        Ammo::Rounds(_) if npc => Some(spec.id),
        Ammo::Rounds(ammo) => Some(ammo),
        Ammo::Unlimited | Ammo::Fuel(_) | Ammo::Other(_) => None,
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
        if let Some(outfit) = lowest_ammo_outfit(self.sources, ammo) {
            let count = self.owned.entry(outfit).or_default();
            *count = count.saturating_add(1);
        }
    }
}

/// The outfit a round of `ammo` is stowed as: the ammunition outfit of
/// lowest ID among `sources` that is its rounds; none when no outfit is.
#[must_use]
pub fn lowest_ammo_outfit(sources: &[(WeaponId, OutfitId)], ammo: WeaponId) -> Option<OutfitId> {
    sources
        .iter()
        .filter(|&&(of, _)| of == ammo)
        .map(|&(_, outfit)| outfit)
        .min()
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
    /// Whether the ship, an `npc` or the player, can pay for a shot from
    /// `rounds` and `fuel`: from the [`slot`] it reads, which must hold a
    /// round; otherwise from the fuel for a fuel weapon, and always for
    /// any other.
    #[must_use]
    pub fn affords(&self, rounds: &dyn Rounds, fuel: Gauge, npc: bool) -> bool {
        match (slot(&self.spec, npc), self.spec.ammo) {
            (Some(slot), _) => rounds.held(slot) > 0,
            (None, Ammo::Fuel(cost)) => fuel.now >= cost,
            (None, _) => true,
        }
    }

    /// Pays for a shot from `rounds` and `fuel`, the ship an `npc` or the
    /// player: a round of its [`slot`], or its fuel. An NPC's point defence
    /// (`point_defence`) of an `AmmoType` past 255, which always has its
    /// ammunition, still spends a round of its own slot while it holds any
    /// (`_HandleShipPointDefense` @0x39a38-0x39a43, @0x39b8c-0x39bad); its
    /// other weapons of such an `AmmoType` spend nothing
    /// (`_FireAIShipWeapon` @0x88d51-0x88d5d).
    fn pay(&self, rounds: &mut dyn Rounds, fuel: &mut Gauge, npc: bool, point_defence: bool) {
        match (slot(&self.spec, npc), self.spec.ammo) {
            (Some(slot), _) => rounds.spend(slot),
            (None, Ammo::Fuel(cost)) => fuel.now -= cost,
            (None, Ammo::Other(raw)) if npc && point_defence && raw >= 0 => {
                rounds.spend(self.spec.id);
            }
            (None, _) => {}
        }
    }

    /// Fires a shot on a trigger: reports the flags it ignores to
    /// `reports`, counts it in the burst, pays for it from `rounds` and
    /// `fuel` when it spends, the ship an `npc` or the player, and sets the
    /// reload (see the module docs).
    fn discharge(
        &mut self,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        reports: &mut Reports,
        npc: bool,
    ) {
        reports.fired(&self.spec);
        if self.spec.burst_count > 0 {
            self.burst += 1;
        }
        if self.spends() {
            self.pay(rounds, fuel, npc, false);
        }
        self.reload = self.spec.reload / self.count as f32;
        if self.spec.burst_count > 0 && self.burst >= self.spec.burst_count * self.count {
            self.burst = 0;
            self.reload = self.spec.burst_reload;
        }
    }

    /// Fires a point-defence shot or beam, as `_HandleShipPointDefense`'s
    /// tail does (@0x39931-0x39bb3): reports the flags it ignores to
    /// `reports`; adds `Reload / count` to its timer, whatever is left of
    /// it (`+=`, @0x39959, so a timer counted past none carries); pays for
    /// it from `rounds` and `fuel`, the ship an `npc` or the player, unless
    /// it spends only at the end of a burst (`Flags3` 0x0001) on rounds,
    /// as fuel is paid every shot (@0x39a2e only guards `AmmoType` 0 and
    /// up); and counts it in its burst, which past [`burst_interval`] starts
    /// again on its `BurstReload` in place of the timer and, under 0x0001
    /// with an `AmmoType` of 0 to 255, pays a round then (@0x39ac2-0x39b8a).
    /// The original's round at the end of a burst is unfloored; the burst's
    /// last shot fires only with a round to pay, so the floor here never
    /// shows.
    fn defend(
        &mut self,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        reports: &mut Reports,
        npc: bool,
    ) {
        reports.fired(&self.spec);
        self.reload += self.spec.reload / self.count as f32;
        let at_burst_end = self.spec.ammo_at_burst_end();
        let on_rounds = (0..=MAX_AMMO_SLOT).contains(&self.spec.ammo_type);
        if !(at_burst_end && self.spec.ammo_type >= 0) {
            self.pay(rounds, fuel, npc, true);
        }
        if self.spec.burst_count > 0 {
            self.burst += 1;
            if self.burst >= burst_interval(&self.spec, self.count) {
                self.burst = 0;
                self.reload = self.spec.burst_reload;
                if at_burst_end && on_rounds {
                    self.pay(rounds, fuel, npc, true);
                }
            }
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
    /// Which way it points, from wherever it leaves, before its
    /// inaccuracy.
    pub pointing: Pointing,
    /// Its inaccuracy, in degrees: how far off its pointing it leaves.
    pub inaccuracy: f32,
    /// The ship it is fired at, if any.
    pub target: Option<ShipRef>,
}

impl Launch {
    /// The heading it leaves `origin` along, in degrees: its pointing
    /// from there ([`Pointing::heading_from`]) turned by its inaccuracy.
    /// The fight gives the exit point it leaves from, or its ship's
    /// centre (see [`exits`](super::exits)).
    #[must_use]
    pub fn heading_from(&self, origin: &ShipState) -> f32 {
        normalized(self.pointing.heading_from(origin) + self.inaccuracy)
    }
}

/// What an NPC's pick fired in one call ([`Armament::fire_pick`]): its
/// weapon and the copies that spawned, at least one.
#[derive(Clone, Debug, PartialEq)]
pub struct Volley {
    /// The weapon.
    pub weapon: WeaponSpec,
    /// Each copy that spawned, in order.
    pub launches: Vec<Launch>,
}

/// The weapons a ship carries, in the order they were mounted.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Armament {
    mounts: Vec<Mount>,
    /// The last unguided gun or rocket an NPC's pick fired (the original's
    /// +0xc8da, `_FireAIShipWeapon` @0x89258).
    last_gun: Option<WeaponId>,
    /// The exit each `ExitType`'s next shot or beam leaves from (the
    /// original's +0xc8fe-+0xc905; see [`exits`](super::exits)).
    exits: ExitCursors,
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
        Self {
            mounts,
            last_gun: None,
            exits: ExitCursors::default(),
        }
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

    /// Whether `shooter` has the ammunition for a shot of `mount`, one of
    /// its own, with `rounds` and `fuel`: `_WeaponHasAmmo` (@0xb95a-0xbafc),
    /// in its order.
    ///
    /// 1. An NPC never has it for a weapon of `Flags2` [`AI_WONT_USE`].
    /// 2. A cloaked shooter has it only for a weapon of `Flags2`
    ///    [`FIRES_CLOAKED`].
    /// 3. A weapon of `Flags2` [`NEEDS_KEY_SHIP`] needs the shooter's
    ///    class to have a `KeyCarried`, and a fighter bay among its mounts
    ///    launching that ship that holds rounds (@0xba46-0xba7f).
    /// 4. Then it pays its own way ([`Mount::affords`]): a guidance-99
    ///    weapon from its own slot, whatever its `AmmoType` (@0xba87-0xbaa1);
    ///    a launcher from its own slot on an NPC and from its ammunition's
    ///    on the player (@0xbaaa-0xbac2); a fuel weapon from the fuel; and
    ///    any other always.
    #[must_use]
    pub fn has_ammo(
        &self,
        mount: &Mount,
        rounds: &dyn Rounds,
        fuel: Gauge,
        shooter: Shooter,
    ) -> bool {
        let flags2 = mount.spec.flags2;
        if shooter.npc && flags2 & AI_WONT_USE != 0 {
            return false;
        }
        if shooter.cloaked && flags2 & FIRES_CLOAKED == 0 {
            return false;
        }
        if flags2 & NEEDS_KEY_SHIP != 0 && !self.key_aboard(shooter.key_carried, rounds) {
            return false;
        }
        mount.affords(rounds, fuel, shooter.npc)
    }

    /// Whether a fighter bay launching `key` holds rounds of its own in
    /// `rounds`; never without a `key`.
    fn key_aboard(&self, key: Option<ShipId>, rounds: &dyn Rounds) -> bool {
        key.is_some_and(|key| {
            self.mounts
                .iter()
                .any(|mount| mount.spec.carried == Some(key) && rounds.held(mount.spec.id) > 0)
        })
    }

    /// Fires every weapon `trigger` picks that is ready, that `shooter`
    /// has the ammunition for ([`Armament::has_ammo`]) and that `aim`
    /// aims, on a ship in `condition`, paying from `rounds` and `fuel`, the
    /// inaccuracy drawn on `chance`, and gives what it launched; what it
    /// reports goes to `reports` (see the module docs): an NPC's bay launch
    /// and an escort's formation trigger, whose owners
    /// (`_AILaunchFighter`, `_AIEscortFireUnprovoked`) are rdm phases
    /// escort-fleet-fidelity/phase-4 and phase-1; the player fires through
    /// [`Armament::fire_player`]. A bay launching
    /// with [`Trigger::bays`] reads its own rounds alone, as
    /// `_AILaunchFighter` (@0x813cc-0x813e8) never asks `_WeaponHasAmmo`.
    // Each is a separate borrow the fight holds: the ship's command, its
    // condition, who it is, its stores, the dice, the reports and its aim.
    #[allow(clippy::too_many_arguments)]
    pub fn fire(
        &mut self,
        trigger: Trigger,
        condition: Condition,
        shooter: Shooter,
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
        let launching = self.launching_bay(trigger, rounds);
        for index in 0..self.mounts.len() {
            let mount = &self.mounts[index];
            if !trigger.fires(&mount.spec) || mount.reload > 0.0 {
                continue;
            }
            let bay_launch = mount.spec.is_bay() && trigger.bays;
            if bay_launch && launching != Some(mount.spec.id) {
                continue;
            }
            let paid = if bay_launch {
                mount.affords(rounds, *fuel, shooter.npc)
            } else {
                self.has_ammo(mount, rounds, *fuel, shooter)
            };
            let mount = &mut self.mounts[index];
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
            if !paid {
                continue;
            }
            let inaccuracy = inaccuracy(mount.spec.inaccuracy, chance);
            mount.discharge(rounds, fuel, reports, shooter.npc);
            launches.push(Launch {
                weapon: mount.spec,
                pointing: aimed.pointing,
                inaccuracy,
                target: aimed.target,
            });
        }
        launches
    }

    /// Fires `pick`, an NPC's pick, on a ship in `condition`, as
    /// `_FireAIShipWeapon` does (@0x8873d-0x89269), paying from `rounds`
    /// and `fuel`, each copy's inaccuracy drawn on `chance`; what it
    /// reports goes to `reports`. None when nothing spawned.
    ///
    /// 1. Nothing fires unless the ship is [`Condition::Intact`] (the
    ///    engine's form of `_LowLevelAIHandler` clearing the fire flag of
    ///    a disabled ship, @0x851f6), it carries `pick`, and `pick` has
    ///    reloaded (@0x887a7-0x887cb).
    /// 2. It fires [`ai_fire::copies`] copies. Each copy `shooter` has the
    ///    ammunition for ([`Armament::has_ammo`]) spawns as `spawn` says
    ///    ([`ai_fire::spawn`]), and pays a round of its own slot, or its
    ///    fuel floored at none, unless it spends only at the end of a
    ///    burst (@0x88cd8-0x88dac): by the engine whether or not it
    ///    spawned, and by the other reading only when it did, as `spend`
    ///    says ([`RuleKey::AiSpendWithoutShot`](crate::RuleKey::AiSpendWithoutShot)).
    /// 3. With nothing spawned, nothing else changes (@0x88dc0).
    /// 4. Otherwise it reloads by [`ai_fire::reload`]; a burst counts one a
    ///    call, spends at a multiple of its `BurstCount` when it spends at
    ///    the burst's end ([`Armament::burst_end`]), and past its interval
    ///    (`BurstCount`, times the count unless its copies fire together)
    ///    starts again on its `BurstReload` (@0x8901f-0x891ad); an
    ///    exclusive weapon ([`EXCLUSIVE`]) holds every other weapon's
    ///    reload at its own plus [`EXCLUSIVE_HOLD`] (@0x891c8-0x8922a); and
    ///    an unguided gun or rocket becomes the last gun
    ///    ([`Armament::last_gun`], @0x89230-0x89258).
    // Each is a separate borrow the fight holds, as for `fire`, and the
    // rule it spends by.
    #[allow(clippy::too_many_arguments)]
    pub fn fire_pick(
        &mut self,
        pick: WeaponId,
        condition: Condition,
        shooter: Shooter,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        spend: RuleSource,
        spawn: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
    ) -> Option<Volley> {
        if condition != Condition::Intact {
            return None;
        }
        let index = self.mounts.iter().position(|mount| mount.spec.id == pick)?;
        let Mount {
            spec,
            count,
            reload,
            ..
        } = self.mounts[index];
        if reload > 0.0 || count == 0 {
            return None;
        }
        let mut launches = Vec::new();
        for _ in 0..ai_fire::copies(&spec, count, rounds, *fuel) {
            if !self.has_ammo(&self.mounts[index], rounds, *fuel, shooter) {
                continue;
            }
            let aimed = spawn(&spec);
            if let Some(aimed) = aimed {
                reports.fired(&spec);
                launches.push(Launch {
                    weapon: spec,
                    pointing: aimed.pointing,
                    inaccuracy: inaccuracy(spec.inaccuracy, chance),
                    target: aimed.target,
                });
            }
            let pays = aimed.is_some() || spend == RuleSource::Engine;
            if pays && !spec.ammo_at_burst_end() {
                self.mounts[index].pay(rounds, fuel, shooter.npc, false);
                fuel.now = fuel.now.max(0.0);
            }
        }
        if launches.is_empty() {
            return None;
        }
        let spawned = u32::try_from(launches.len()).unwrap_or(u32::MAX);
        let mut held = ai_fire::reload(&spec, spawned, count);
        // The NPC's burst end spends its own slot while it holds any
        // (@0x890a5).
        let own = (spec.guidance_99() || rounds.held(spec.id) > 0).then_some(spec.id);
        if let Some(burst_reload) = self.burst_step(index, spawned, own, rounds, fuel) {
            held = burst_reload;
        }
        self.mounts[index].reload = held;
        if spec.flags3 & EXCLUSIVE != 0 {
            self.hold_others(index, held);
        }
        if matches!(spec.guidance, Guidance::Unguided | Guidance::Rocket) {
            self.last_gun = Some(pick);
        }
        Some(Volley {
            weapon: spec,
            launches,
        })
    }

    /// The weapons the player's `trigger` fires this tick, in the order
    /// `_HandlePlayer` calls `_FirePlayerWeapon` on them (@0x6ab39-0x6abc7):
    /// on the primary trigger every weapon carried that is not a secondary
    /// (`Flags` 0x0002), a fighter bay too, by ascending weapon ID; then
    /// the secondary selected, on the secondary trigger.
    #[must_use]
    pub fn player_picks(&self, trigger: Trigger) -> Vec<WeaponId> {
        let mut picks: Vec<WeaponId> = self
            .mounts
            .iter()
            .filter(|mount| trigger.primary && !mount.spec.secondary())
            .map(|mount| mount.spec.id)
            .collect();
        picks.sort_unstable();
        picks.extend(
            trigger
                .secondary
                .filter(|&weapon| self.mounts.iter().any(|mount| mount.spec.id == weapon)),
        );
        picks
    }

    /// Fires `weapon` as the player's trigger does, as `_FirePlayerWeapon`
    /// does (@0x62702-0x631c3), paying from `rounds` and `fuel`, each
    /// copy's inaccuracy drawn on `chance`; what it reports goes to
    /// `reports`. None when nothing was attempted.
    ///
    /// 1. Nothing fires unless the ship is [`Condition::Intact`]
    ///    (`_HandlePlayer`'s `_IsDisabled` gate, @0x6ab1c), it carries
    ///    `weapon`, `weapon` has reloaded (@0x6287b) and its guidance is
    ///    one the simulation flies (else it is reported, as
    ///    [`Armament::fire`] does).
    /// 2. It fires [`ai_fire::copies`] copies (`_WeaponMaxSimultShots`
    ///    @0x628a0). Each copy `shooter` has the ammunition for
    ///    ([`Armament::has_ammo`], @0x628dc) is attempted as `spawn` says,
    ///    and an attempted copy pays a round of the slot it reads
    ///    ([`slot`]: the `AmmoType`'s weapon, or a fighter bay's own), or
    ///    its fuel floored at none, unless it spends only at the end of a
    ///    burst (@0x62ca4-0x62d6d); a copy not attempted spends nothing.
    /// 3. With nothing attempted, nothing else changes (@0x62d82).
    /// 4. Otherwise it reloads by [`ai_fire::reload`] (@0x62f42-0x62f96);
    ///    an exclusive weapon ([`EXCLUSIVE`]) holds every other weapon's
    ///    reload at that plus [`EXCLUSIVE_HOLD`] (@0x62fa9-0x62ff8), before
    ///    the burst; and a burst counts one a call, spends the slot it reads
    ///    or its fuel at a multiple of its `BurstCount` when it spends at the
    ///    burst's end, and past its interval starts again on its
    ///    `BurstReload` (@0x62ffa-0x631b1).
    ///
    /// The sound and the recoil, once a call, are the fight's
    /// ([`Combat`](super::Combat)). The hook for folding before fire
    /// (@0x62837-0x62864, the +0xa8 stamp @0x6282d) is the top of this
    /// function (rdm phase ship-handling-fidelity/phase-10-ship-drawing).
    // Each is a separate borrow the fight holds, as for `fire`.
    #[allow(clippy::too_many_arguments)]
    pub fn fire_player(
        &mut self,
        weapon: WeaponId,
        condition: Condition,
        shooter: Shooter,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        spawn: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
    ) -> Option<Volley> {
        if condition != Condition::Intact {
            return None;
        }
        let index = self
            .mounts
            .iter()
            .position(|mount| mount.spec.id == weapon)?;
        let Mount {
            spec,
            count,
            reload,
            ..
        } = self.mounts[index];
        if reload > 0.0 {
            return None;
        }
        if let Guidance::Other(guidance) = spec.guidance {
            reports.report(SimDiagnostic::UnimplementedGuidance {
                weapon: spec.id,
                guidance,
            });
            return None;
        }
        let mut launches = Vec::new();
        for _ in 0..ai_fire::copies(&spec, count, rounds, *fuel) {
            if !self.has_ammo(&self.mounts[index], rounds, *fuel, shooter) {
                continue;
            }
            let Some(aimed) = spawn(&spec) else {
                continue;
            };
            reports.fired(&spec);
            launches.push(Launch {
                weapon: spec,
                pointing: aimed.pointing,
                inaccuracy: inaccuracy(spec.inaccuracy, chance),
                target: aimed.target,
            });
            if !spec.ammo_at_burst_end() {
                self.mounts[index].pay(rounds, fuel, false, false);
                fuel.now = fuel.now.max(0.0);
            }
        }
        if launches.is_empty() {
            return None;
        }
        let spawned = u32::try_from(launches.len()).unwrap_or(u32::MAX);
        let held = ai_fire::reload(&spec, spawned, count);
        self.mounts[index].reload = held;
        if spec.flags3 & EXCLUSIVE != 0 {
            self.hold_others(index, held);
        }
        let slot = slot(&spec, false);
        if let Some(burst_reload) = self.burst_step(index, spawned, slot, rounds, fuel) {
            self.mounts[index].reload = burst_reload;
        }
        Some(Volley {
            weapon: spec,
            launches,
        })
    }

    /// Counts a call of `self.mounts[index]` that spawned `spawned` copies
    /// in its burst, as both `_FirePlayerWeapon` (@0x62ffa-0x631b1) and
    /// `_FireAIShipWeapon` (@0x8901f-0x891ad) do: one a call; at a multiple
    /// of its `BurstCount`, when it spends at the burst's end, it pays
    /// [`burst_end`] from `slot` (its copies' worth when they fire
    /// together, else one); and past [`burst_interval`] it starts again,
    /// giving its `BurstReload`.
    fn burst_step(
        &mut self,
        index: usize,
        spawned: u32,
        slot: Option<WeaponId>,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
    ) -> Option<f32> {
        let mount = &mut self.mounts[index];
        let spec = mount.spec;
        if spec.burst_count == 0 {
            return None;
        }
        mount.burst += 1;
        if spec.ammo_at_burst_end() && mount.burst.is_multiple_of(spec.burst_count) {
            let amount = if spec.flags & SIMULTANEOUS == 0 {
                1
            } else {
                spawned
            };
            burst_end(&spec, amount, slot, rounds, fuel);
        }
        if mount.burst >= burst_interval(&spec, mount.count) {
            mount.burst = 0;
            return Some(spec.burst_reload);
        }
        None
    }

    /// Holds every weapon's reload but `self.mounts[index]`'s at `held`
    /// plus [`EXCLUSIVE_HOLD`], or what it is if more: an exclusive
    /// weapon's hold (`Flags3` 0x0020).
    fn hold_others(&mut self, index: usize, held: f32) {
        for (other, mount) in self.mounts.iter_mut().enumerate() {
            if other != index {
                mount.reload = mount.reload.max(held + EXCLUSIVE_HOLD);
            }
        }
    }

    /// The exit each `ExitType`'s next shot or beam leaves from, each 0
    /// to 3 and starting at 0 (see [`exits`](super::exits)).
    #[must_use]
    pub fn exit_cursors(&self) -> &ExitCursors {
        &self.exits
    }

    /// The cursors, for the fight to move on as its shots and beams leave
    /// ([`Combat`](super::Combat)).
    pub fn exit_cursors_mut(&mut self) -> &mut ExitCursors {
        &mut self.exits
    }

    /// The last unguided gun or rocket (guidance -1 or 6) an NPC's pick
    /// fired ([`Armament::fire_pick`]), the original's +0xc8da: none
    /// before its first.
    #[must_use]
    pub fn last_gun(&self) -> Option<WeaponId> {
        self.last_gun
    }

    /// The bay an NPC attacking would launch from now, as
    /// `_AILaunchFighter` picks it (@0x813cc-0x8140b): the one of lowest
    /// weapon ID holding rounds of its own, once it has reloaded; none
    /// while that one reloads, a later bay waiting until it is empty.
    #[must_use]
    pub fn bay_to_launch(&self, rounds: &dyn Rounds) -> Option<WeaponId> {
        let bay = self.first_bay(rounds)?;
        self.mounts
            .iter()
            .any(|mount| mount.spec.id == bay && mount.reload <= 0.0)
            .then_some(bay)
    }

    /// Fires the point defence of a ship in `condition`, paying from
    /// `rounds` and `fuel`, the inaccuracy drawn on `chance`: the ready
    /// point-defence weapon `shooter` has the ammunition for
    /// ([`Armament::has_ammo`], `_HandleShipPointDefense` @0x3933b) with
    /// the lowest ID, pointing as `pick` says at the quarry it gives, if
    /// any: the fight's [`Quarry`](super::defence::Quarry), a missile or a
    /// ship, which goes through the same tail either way (@0x39931). Nothing
    /// fires, and nothing is spent, when `pick` gives none, nor for a shot
    /// (guidance 9) when the shot table has no `room`: `_SpawnShot` failing
    /// returns before the reload, the ammunition, the sound and the burst
    /// (@0x396ea-0x396ee). A beam (10) never asks (`_SpawnBeam` @0x3992c
    /// is unchecked), so its tail runs whether or not it spawns. What it
    /// reports goes to `reports`.
    // Each is a separate borrow the fight holds, as for `fire`.
    #[allow(clippy::too_many_arguments)]
    pub fn fire_point_defence<Q>(
        &mut self,
        condition: Condition,
        shooter: Shooter,
        rounds: &mut dyn Rounds,
        fuel: &mut Gauge,
        chance: &mut dyn Chance,
        reports: &mut Reports,
        room: bool,
        pick: &mut dyn FnMut(&WeaponSpec) -> Option<(Pointing, Q)>,
    ) -> Option<(Launch, Q)> {
        if condition != Condition::Intact {
            return None;
        }
        let index = self
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, mount)| {
                matches!(
                    mount.spec.guidance,
                    Guidance::PointDefence | Guidance::PointDefenceBeam
                ) && mount.reload <= 0.0
                    && self.has_ammo(mount, rounds, *fuel, shooter)
            })
            .min_by_key(|(_, mount)| mount.spec.id)?
            .0;
        let mount = &mut self.mounts[index];
        let (pointing, quarry) = pick(&mount.spec)?;
        if mount.spec.guidance == Guidance::PointDefence && !room {
            return None;
        }
        let inaccuracy = if mount.spec.guidance == Guidance::PointDefenceBeam {
            0.0
        } else {
            inaccuracy(mount.spec.inaccuracy, chance)
        };
        mount.defend(rounds, fuel, reports, shooter.npc);
        let launch = Launch {
            weapon: mount.spec,
            pointing,
            inaccuracy,
            target: None,
        };
        Some((launch, quarry))
    }

    /// The bay `trigger` launches from with [`Trigger::bays`]: the one of
    /// lowest weapon ID holding rounds of its own, ready or not; none
    /// without `bays`.
    fn launching_bay(&self, trigger: Trigger, rounds: &dyn Rounds) -> Option<WeaponId> {
        if !trigger.bays {
            return None;
        }
        self.first_bay(rounds)
    }

    /// The bay of lowest weapon ID holding rounds of its own, ready or not.
    fn first_bay(&self, rounds: &dyn Rounds) -> Option<WeaponId> {
        self.mounts
            .iter()
            .filter(|mount| mount.spec.is_bay() && rounds.held(mount.spec.id) > 0)
            .map(|mount| mount.spec.id)
            .min()
    }

    /// The bay a fighter of ship type `ship` docks into: the one of lowest
    /// weapon ID that launches it, as `_AIAddFighterToParent` walks the
    /// parent's weapons by index (@0x802f8-0x80326); none when no bay
    /// does.
    #[must_use]
    pub fn bay_of(&self, ship: ShipId) -> Option<WeaponId> {
        self.mounts
            .iter()
            .filter(|mount| mount.spec.carried == Some(ship))
            .map(|mount| mount.spec.id)
            .min()
    }

    /// Takes a fighter of ship type `ship` aboard, a round of the bay of
    /// lowest ID that launches it, into `rounds` (see the module docs), and
    /// says whether one did; with none, nothing changes.
    pub fn stow(&mut self, ship: ShipId, rounds: &mut dyn Rounds) -> bool {
        let bay = self.bay_of(ship);
        let Some(mount) = self
            .mounts
            .iter_mut()
            .find(|mount| Some(mount.spec.id) == bay)
        else {
            return false;
        };
        if rounds.none_left(mount.spec.id) {
            mount.reload = mount.reload.max(mount.spec.reload);
        }
        rounds.stow(mount.spec.id);
        true
    }

    /// Holds the reload of `weapon`, if it carries it, at the weapon's
    /// whole `Reload`, however many copies it carries: what a shot of a
    /// weapon that fires one shot at a time does to its firer while it
    /// flies (`Flags3` 0x0004; `_HandleShot` @0x35c09-0x35c42 sets the
    /// ship's reload for the weapon's type, +0xf8, to the `wëap`'s
    /// Reload, +0x30).
    pub fn hold(&mut self, weapon: WeaponId) {
        for mount in self
            .mounts
            .iter_mut()
            .filter(|mount| mount.spec.id == weapon)
        {
            mount.reload = mount.spec.reload;
        }
    }

    /// Counts every weapon's reload timer down a tick, as `_HandleShip`
    /// (@0x33896-0x338ce) and `_HandlePlayer` (@0x6ae25-0x6ae61) do: a
    /// timer above none loses a tick, unfloored, so one of 0.5 reads -0.5
    /// for a tick, which point defence's reload adds to
    /// ([`Armament::fire_point_defence`]); one at or below none is set to
    /// none.
    pub fn reload(&mut self) {
        for mount in &mut self.mounts {
            mount.reload = if mount.reload > 0.0 {
                mount.reload - 1.0
            } else {
                0.0
            };
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

/// The shots in a burst of `spec` carried `count` times
/// (`_WeaponBurstInterval` @0x831c): its `BurstCount` for copies that
/// fire together (`Flags` 0x0040), and `BurstCount` × `count` otherwise.
fn burst_interval(spec: &WeaponSpec, count: u32) -> u32 {
    if spec.flags & SIMULTANEOUS == 0 {
        spec.burst_count * count
    } else {
        spec.burst_count
    }
}

/// The highest `AmmoType` whose round point defence pays at the end of a
/// burst (`_HandleShipPointDefense` @0x39b56-0x39b5a, unsigned).
const MAX_AMMO_SLOT: i16 = 255;

/// What the end of a burst of `spec` spends, `amount` shots' worth, from
/// `rounds` and `fuel`: a round each of `slot`, floored at none, when it
/// names one; otherwise, for `AmmoType` -1000 and below, its fuel, floored
/// at none. An NPC's slot is its own, for a guidance-99 weapon or a slot
/// holding any, whatever its `AmmoType` (`_FireAIShipWeapon`
/// @0x890a5-0x8913d); the player's is the one its shots read ([`slot`]:
/// a fighter bay's own, or its `AmmoType`'s 0-255, @0x6307a-0x630f6).
/// The original also floors a slot already below none up to none; a slot
/// below none keeps its value here.
fn burst_end(
    spec: &WeaponSpec,
    amount: u32,
    slot: Option<WeaponId>,
    rounds: &mut dyn Rounds,
    fuel: &mut Gauge,
) {
    if let Some(slot) = slot {
        for _ in 0..amount {
            rounds.spend(slot);
        }
    } else if let Ammo::Fuel(cost) = spec.ammo {
        fuel.now = (fuel.now - cost * amount as f32).max(0.0);
    }
}

/// How far off its heading a shot of a weapon `inaccuracy` degrees
/// inaccurate leaves, drawn on `chance`: `Rand(2i) − i`, none with no draw
/// for none. A beam turns by it every tick
/// ([`Beam::follow`](super::beam::Beam::follow)).
pub(super) fn inaccuracy(inaccuracy: u32, chance: &mut dyn Chance) -> f32 {
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
    /// The arsenal of `weapons` and `hulls`, each weapon reaching down its
    /// sub-munition chain ([`chain_range`]).
    #[must_use]
    pub fn new(weapons: &[WeaponRecord], hulls: Vec<HullRecord>) -> Self {
        let records: BTreeMap<WeaponId, WeaponRecord> =
            weapons.iter().map(|record| (record.id, *record)).collect();
        Self {
            weapons: records
                .iter()
                .map(|(&id, record)| {
                    let spec = WeaponSpec {
                        chain_range: chain_range(&records, id),
                        ..WeaponSpec::new(record)
                    };
                    (id, spec)
                })
                .collect(),
            hulls: hulls.into_iter().map(|hull| (hull.id, hull)).collect(),
        }
    }

    /// Every weapon, by ascending ID: the original's `wëap` slot order.
    pub fn weapons(&self) -> impl Iterator<Item = &WeaponSpec> {
        self.weapons.values()
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

    /// Ship type `ship`'s hull, with its stock weapons' reach
    /// ([`HullSpec::stock_reach`]); a placeholder for one that cannot be
    /// read.
    #[must_use]
    pub fn hull(&self, ship: ShipId) -> HullSpec {
        self.hulls
            .get(&ship)
            .map(|record| HullSpec {
                stock_reach: self.stock_reach(ship),
                ..HullSpec::new(record)
            })
            .unwrap_or_default()
    }

    /// How far ship class `ship`'s stock weapons reach (see
    /// [`HullSpec::stock_reach`]).
    fn stock_reach(&self, ship: ShipId) -> Option<f32> {
        self.stock_slots(ship)
            .into_iter()
            .filter(|&(_, (count, _))| count > 0)
            .filter_map(|(weapon, _)| self.weapon(weapon))
            .filter(|spec| in_gun_range_walk(spec))
            .map(|spec| match spec.guidance {
                Guidance::Beam | Guidance::TurretBeam => spec.beam_length + GUN_RANGE_MARGIN,
                _ => (spec.range() + GUN_RANGE_MARGIN).trunc(),
            })
            .reduce(f32::max)
    }

    /// Ship type `ship`'s `Strength`, raw: none for one that cannot be
    /// read.
    #[must_use]
    pub fn strength(&self, ship: ShipId) -> i16 {
        self.hulls.get(&ship).map_or(0, |hull| hull.strength)
    }

    /// The player's armament, owning `owned` of `outfits`: its weapon
    /// outfits, its stock weapons among them (see the module docs).
    #[must_use]
    pub fn player(&self, owned: &BTreeMap<OutfitId, u16>, outfits: &[OutfitRecord]) -> Armament {
        self.mounted(mods(owned, outfits, MOD_WEAPON))
    }

    /// The armament of a ship of class `ship` as an NPC flies it: its
    /// class's stock weapons alone, a later slot naming the same weapon
    /// replacing an earlier one's count (`_LoadObjectData` @0x7aa72), by
    /// ascending weapon ID, a count of none or less mounting nothing (see
    /// the module docs).
    #[must_use]
    pub fn of_class(&self, ship: ShipId) -> Armament {
        self.mounted(
            self.stock_slots(ship)
                .into_iter()
                .filter_map(|(weapon, (count, _))| Some((weapon, u32::try_from(count).ok()?))),
        )
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

    /// The armament of an NPC of type `ship` ([`Arsenal::of_class`]), with
    /// the rounds it holds: each stock slot's `AmmoLoad`, raw, on its own
    /// weapon's slot, whatever that weapon's `AmmoType`, a later slot
    /// naming the same weapon replacing an earlier one's (`_LoadObjectData`
    /// @0x7aa90, copied by every spawn, `_RandomShipSpawn`
    /// @0x3c7de-0x3c848); none for a weapon that cannot be read. Its
    /// `DefaultItems` arm it with nothing (see the module docs).
    #[must_use]
    pub fn npc(&self, ship: ShipId) -> (Armament, LauncherRounds) {
        let rounds = self
            .stock_slots(ship)
            .into_iter()
            .filter(|(weapon, _)| self.weapon(*weapon).is_some())
            .map(|(weapon, (_, load))| (weapon, i32::from(load)))
            .collect();
        (self.of_class(ship), rounds)
    }

    /// Ship class `ship`'s stock weapon slots that name a weapon, in slot
    /// order, raw; none for a class that cannot be read.
    pub(crate) fn stock_weapons(&self, ship: ShipId) -> &[StockWeapon] {
        self.hulls
            .get(&ship)
            .map_or(&[], |hull| hull.weapons.as_slice())
    }

    /// Each ammunition outfit among `outfits`, with the `wëap` it is the
    /// rounds of, by ascending outfit ID.
    #[must_use]
    pub fn ammo_outfits(outfits: &[OutfitRecord]) -> Vec<(WeaponId, OutfitId)> {
        holders(outfits, MOD_AMMO)
    }

    /// Each launcher among `outfits`, with its link to its ammunition
    /// (see the module docs): every outfit whose first [`MOD_WEAPON`] mod
    /// names a weapon that can be read.
    #[must_use]
    pub fn magazines(&self, outfits: &[OutfitRecord]) -> BTreeMap<OutfitId, Magazine> {
        let ammo_holders = holders(outfits, MOD_AMMO);
        outfits
            .iter()
            .filter_map(|record| {
                let &(_, weapon) = record.mods.iter().find(|&&(kind, _)| kind == MOD_WEAPON)?;
                let weapon = WeaponId(weapon);
                let spec = self.weapon(weapon)?;
                // A bay's rounds are already its own (`WeaponSpec::new`).
                let ammo = match spec.ammo {
                    Ammo::Rounds(ammo) => ammo,
                    _ => weapon,
                };
                let magazine = Magazine {
                    weapon,
                    ammo,
                    max_ammo: spec.max_ammo,
                    carried: spec.carried,
                    ammo_outfit: holder(&ammo_holders, weapon),
                };
                Some((record.id, magazine))
            })
            .collect()
    }

    /// Ship class `ship`'s stock weapons and their ammunition, each as
    /// the outfit among `outfits` that holds it (see the module docs);
    /// none for a class that cannot be read. The weapons come first, by
    /// ascending weapon ID, then the ammunition.
    #[must_use]
    pub fn stock_fits(&self, ship: ShipId, outfits: &[OutfitRecord]) -> Vec<StockFit> {
        self.stock_loaded(ship, outfits)
    }

    /// What ship class `ship`'s stock weapons and their ammunition add to
    /// the weapon and rounds arrays ([`Arrays`]) when the `E` and `H` set operators
    /// change to it (`_EvalSetExp` @0x1551c-0x155de; see the module
    /// docs): each weapon's `WeapCount` to its own entry, and its
    /// `AmmoLoad` to the weapon its raw `AmmoType` names (`wëap` 128 +
    /// `AmmoType`) when that is 0 to 255, whatever its guidance, and to
    /// the weapon itself otherwise, the loads that share an ammunition
    /// summed; a later slot naming the same weapon replaces an earlier
    /// one's count and load. A fighter bay's raw `AmmoType` is its
    /// fighter's `shïp` ID: as `bays` says
    /// ([`RuleKey::ShipChangeBayAmmo`](crate::RuleKey::ShipChangeBayAmmo)),
    /// by the engine its load goes as any other's, and by the other
    /// reading on the bay's own fighters, as [`Arsenal::stock_fits`] puts
    /// it. As `negative` says
    /// ([`RuleKey::ShipChangeNegativeStock`](crate::RuleKey::ShipChangeNegativeStock)),
    /// by the engine a negative count or load is added signed, and by the
    /// other reading one of none or less adds nothing. Which outfits hold
    /// them is the fold's ([`crate::ship_change`]); none for a class that
    /// cannot be read.
    #[must_use]
    pub fn stock_added(&self, ship: ShipId, bays: RuleSource, negative: RuleSource) -> Arrays {
        let route = |spec: &WeaponSpec| match (bays, spec.carried) {
            (RuleSource::Engine, Some(fighter)) if (0..=LAST_AMMO_TYPE).contains(&fighter.0) => {
                WeaponId(FIRST_AMMO_WEAPON + fighter.0)
            }
            _ => own_rounds(spec),
        };
        let counts = |value: i16| match negative {
            RuleSource::Engine => value != 0,
            RuleSource::Bible => value > 0,
        };
        let mut added = Arrays::default();
        for (weapon, (count, load)) in self.stock_slots(ship) {
            if counts(count) {
                *added.weapons.entry(weapon).or_default() += i64::from(count);
            }
            if let (true, Some(spec)) = (counts(load), self.weapon(weapon)) {
                *added.rounds.entry(route(spec)).or_default() += i64::from(load);
            }
        }
        added
    }

    /// Ship class `ship`'s stock weapon slots, each weapon with its raw
    /// `WeapCount` and `AmmoLoad`, a later slot naming the same weapon
    /// replacing an earlier one's (`_LoadObjectData` @0x7aa72, @0x7aa90);
    /// none for a class that cannot be read.
    pub(crate) fn stock_slots(&self, ship: ShipId) -> BTreeMap<WeaponId, (i16, i16)> {
        self.stock_weapons(ship)
            .iter()
            .map(|stock| (stock.weapon, (stock.count, stock.ammo)))
            .collect()
    }

    /// Ship class `ship`'s stock weapons and their ammunition, each as
    /// the outfit among `outfits` that holds it, each weapon's load going
    /// to its own rounds and the loads of one ammunition the largest; none
    /// for a class that cannot be read.
    fn stock_loaded(&self, ship: ShipId, outfits: &[OutfitRecord]) -> Vec<StockFit> {
        let weapon_holders = holders(outfits, MOD_WEAPON);
        let mut fits = Vec::new();
        let mut loads: BTreeMap<WeaponId, u16> = BTreeMap::new();
        for (weapon, (count, load)) in self.stock_slots(ship) {
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
            let loaded = loads.entry(own_rounds(spec)).or_default();
            *loaded = (*loaded).max(load);
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

/// What a gun reaches beyond its range, in pixels (32.0 @0xdd654, read by
/// `_AIInGunRange`).
pub const GUN_RANGE_MARGIN: f32 = 32.0;

/// Whether `_AIInGunRange`'s walk with no weapon named counts `spec`: a
/// raw guidance of 8 or less, signed (`jg` @0x7fa4d), so not point
/// defence (9 and 10), a bay (99) or anything above.
fn in_gun_range_walk(spec: &WeaponSpec) -> bool {
    match spec.guidance {
        Guidance::PointDefence | Guidance::PointDefenceBeam | Guidance::FighterBay => false,
        Guidance::Other(raw) => raw <= 8,
        _ => true,
    }
}

/// The rounds `spec` fires, as a purchase loads them: its `AmmoType`'s
/// weapon, or its own for a fighter bay or any other `AmmoType`
/// (`_DoShipyardDialog` @0x5ef3e-0x5ef56).
fn own_rounds(spec: &WeaponSpec) -> WeaponId {
    match spec.ammo {
        Ammo::Rounds(ammo) => ammo,
        _ => spec.id,
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

/// One launcher outfit's link to its ammunition, as the outfitter's Sell
/// reads it (`_DoOutfitDialog` @0x5ca75-0x5cbcf).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Magazine {
    /// The weapon its first [`MOD_WEAPON`] mod names.
    pub weapon: WeaponId,
    /// The weapon whose rounds it fires: its `AmmoType`'s, or its own for
    /// a fighter bay or any other `AmmoType`.
    pub ammo: WeaponId,
    /// The weapon's `MaxAmmo`; none for no limit of its own.
    pub max_ammo: u32,
    /// The ship a fighter bay launches; none for any other weapon.
    pub carried: Option<ShipId>,
    /// The outfit that names its ammunition: the lowest-ID one whose
    /// [`MOD_AMMO`] mod names the launcher's weapon (not its `AmmoType`'s),
    /// as @0x5cc7e-0x5ccb9 picks.
    pub ammo_outfit: Option<OutfitId>,
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
pub(crate) fn holders(outfits: &[OutfitRecord], mod_type: i16) -> Vec<(WeaponId, OutfitId)> {
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
pub(crate) fn holder(holders: &[(WeaponId, OutfitId)], weapon: WeaponId) -> Option<OutfitId> {
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
    use super::super::weapon::{
        AI_WONT_USE, AMMO_AT_BURST_END, FIRES_CLOAKED, HIDE_WHEN_EMPTY, NEEDS_KEY_SHIP,
    };
    use super::*;
    use crate::catalog::StockWeapon;
    use crate::chance::NeverFires;
    use crate::combat::ShipRef;
    use crate::combat::flags::FlagField;
    use crate::combat::projectile::ShotId;
    use crate::testkit::{Draws, hull, outfit, weapon};
    use crate::traffic::npc::NpcId;

    /// The heading `launch` leaves a ship at the centre along.
    fn leaves(launch: &Launch) -> f32 {
        launch.heading_from(&ShipState::default())
    }

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
        point_defence: false,
        keep: None,
    };

    /// What `armament` has to fire with, and who fires it.
    struct Supplies {
        rounds: LauncherRounds,
        fuel: Gauge,
        reports: Reports,
        shooter: Shooter,
    }

    impl Supplies {
        fn none() -> Self {
            Self {
                rounds: LauncherRounds::default(),
                fuel: Gauge::full(0.0),
                reports: Reports::default(),
                shooter: Shooter::PLAYER,
            }
        }

        /// One tick: fire on `trigger`, then reload.
        fn tick(&mut self, armament: &mut Armament, trigger: Trigger) -> Vec<Launch> {
            let launches = armament.fire(
                trigger,
                Condition::Intact,
                self.shooter,
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
        Some(Aim::along(0.0, None))
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
    fn a_held_weapon_reloads_its_whole_reload_whatever_its_copies() {
        let mut armament = Armament::new([
            (WeaponSpec::new(&blaster(128, 10)), 2),
            (WeaponSpec::new(&blaster(129, 6)), 1),
        ]);
        armament.hold(WeaponId(128));
        let reloads: Vec<f32> = armament.mounts().iter().map(|mount| mount.reload).collect();
        assert_eq!(reloads, [10.0, 0.0], "Reload, not Reload over the copies");
        armament.hold(WeaponId(130));
        let reloads: Vec<f32> = armament.mounts().iter().map(|mount| mount.reload).collect();
        assert_eq!(reloads, [10.0, 0.0], "a weapon it does not carry");
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
        supplies.rounds.set(WeaponId(138), 2);
        supplies.rounds.set(WeaponId(140), 9);
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 4), [0, 1]);
        assert_eq!(supplies.rounds.slot(WeaponId(138)), 0);
        assert_eq!(supplies.rounds.slot(WeaponId(140)), 9, "only its own");
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
            point_defence: false,
            keep: None,
        };
        assert_eq!(fired(&mut armament, second), [140]);
        let both = Trigger {
            primary: true,
            secondary: Some(WeaponId(138)),
            only: None,
            turrets_only: false,
            bays: false,
            point_defence: false,
            keep: None,
        };
        assert_eq!(fired(&mut armament, both), [128, 138, 129]);
        let primary_as_secondary = Trigger {
            primary: false,
            secondary: Some(WeaponId(128)),
            only: None,
            turrets_only: false,
            bays: false,
            point_defence: false,
            keep: None,
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
        Some(Aim::along(100.0, Some(SEVEN)))
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
                Shooter::PLAYER,
                &mut LauncherRounds::default(),
                &mut Gauge::default(),
                &mut chance,
                &mut Reports::default(),
                &mut at_seven,
            );
            assert_eq!(leaves(&launches[0]), heading, "{draw}");
            assert_eq!(launches[0].target, Some(SEVEN), "{draw}");
            assert_eq!(chance.asked, [18]);
        }
        let mut accurate = mounted(blaster(128, 0), 1);
        let mut chance = Draws::of(&[]);
        let launches = accurate.fire(
            PRIMARY,
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut Gauge::default(),
            &mut chance,
            &mut Reports::default(),
            &mut at_seven,
        );
        assert_eq!(leaves(&launches[0]), 100.0);
        assert!(chance.asked.is_empty(), "never asked");
        let mut wrapping = mounted(wild, 1);
        let launches = wrapping.fire(
            PRIMARY,
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut Gauge::default(),
            &mut Draws::of(&[0]),
            &mut Reports::default(),
            &mut ahead,
        );
        assert_eq!(leaves(&launches[0]), 351.0, "wrapped");
    }

    #[test]
    fn each_launch_carries_the_inaccuracy_it_drew() {
        let wild = WeaponRecord {
            inaccuracy: 9,
            ..blaster(128, 0)
        };
        let launches = mounted(wild, 1).fire(
            PRIMARY,
            Condition::Intact,
            Shooter::PLAYER,
            &mut LauncherRounds::default(),
            &mut Gauge::default(),
            &mut Draws::of(&[2]),
            &mut Reports::default(),
            &mut at_seven,
        );
        assert_eq!((launches[0].inaccuracy, leaves(&launches[0])), (-7.0, 93.0));
        let volley = mounted(wild, 1)
            .fire_pick(
                WeaponId(128),
                Condition::Intact,
                Shooter {
                    npc: true,
                    ..Shooter::PLAYER
                },
                &mut LauncherRounds::default(),
                &mut Gauge::default(),
                &mut Draws::of(&[17]),
                &mut Reports::default(),
                RuleSource::Engine,
                &mut at_seven,
            )
            .expect("a volley");
        assert_eq!(volley.launches[0].inaccuracy, 8.0);
        let (launch, _) = defend(
            &mut mounted(defence(161, 9, 4, 9), 1),
            &mut Supplies::none(),
            &mut Draws::of(&[0]),
            Some((30.0, ShotId(4))),
            &mut Vec::new(),
        )
        .expect("point defence fires");
        assert_eq!((launch.inaccuracy, leaves(&launch)), (-9.0, 21.0));
        let (beam, _) = defend(
            &mut mounted(defence(150, 10, 4, 9), 1),
            &mut Supplies::none(),
            &mut Draws::of(&[]),
            Some((30.0, ShotId(4))),
            &mut Vec::new(),
        )
        .expect("a point-defence beam fires");
        assert_eq!(beam.inaccuracy, 0.0, "a beam draws none");
    }

    // An NPC's pick (`_FireAIShipWeapon`).

    /// An NPC's supplies: `rounds`, and fuel at `fuel`.
    fn npc_supplies(rounds: LauncherRounds, fuel: f32) -> Supplies {
        Supplies {
            rounds,
            fuel: Gauge {
                now: fuel,
                max: 100.0,
            },
            reports: Reports::default(),
            shooter: Shooter {
                npc: true,
                ..Shooter::PLAYER
            },
        }
    }

    impl Supplies {
        /// One tick: fire `pick` as `spawn` spawns it, spending by
        /// `spend`, then reload.
        fn pick(
            &mut self,
            armament: &mut Armament,
            pick: i16,
            spend: RuleSource,
            spawn: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
        ) -> Option<Volley> {
            let volley = armament.fire_pick(
                WeaponId(pick),
                Condition::Intact,
                self.shooter,
                &mut self.rounds,
                &mut self.fuel,
                &mut NeverFires,
                &mut self.reports,
                spend,
                spawn,
            );
            armament.reload();
            volley
        }
    }

    /// Spawns nothing.
    fn nothing(_spec: &WeaponSpec) -> Option<Aim> {
        None
    }

    /// A weapon `id` of `guidance` firing its own launcher's rounds
    /// (`AmmoType` 10) every `reload` ticks.
    fn pick_launcher(id: i16, guidance: i16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            ammo_type: 10,
            reload,
            ..weapon(id)
        }
    }

    #[test]
    fn a_copy_that_spawns_nothing_still_spends_by_the_engine() {
        // A turret with its target in a blind spot spawns nothing.
        for (spend, left) in [(RuleSource::Engine, 2), (RuleSource::Bible, 3)] {
            let mut armament = mounted(pick_launcher(130, 4, 0), 1);
            let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(130), 3)]), 0.0);
            assert_eq!(supplies.pick(&mut armament, 130, spend, &mut nothing), None);
            assert_eq!(supplies.rounds.slot(WeaponId(130)), left, "{spend:?}");
            assert_eq!(armament.mounts()[0].reload, 0.0, "no reload");
        }
        // A fuel weapon too, floored at none.
        let fuelled = WeaponRecord {
            ammo_type: -1005,
            ..pick_launcher(131, 4, 0)
        };
        let mut armament = mounted(fuelled, 1);
        let mut supplies = npc_supplies(LauncherRounds::default(), 0.7);
        supplies.pick(&mut armament, 131, RuleSource::Engine, &mut nothing);
        assert!(
            (supplies.fuel.now - 0.2).abs() < 1e-5,
            "{:?}",
            supplies.fuel
        );
        supplies.fuel.now = 0.5;
        supplies.pick(&mut armament, 131, RuleSource::Engine, &mut nothing);
        assert_eq!(supplies.fuel.now, 0.0);
    }

    #[test]
    fn a_kept_fighter_bay_spends_a_round_a_tick_with_no_launch() {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            reload: 60,
            ..weapon(150)
        };
        let mut armament = mounted(bay, 1);
        let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(150), 2)]), 0.0);
        let mut spawn = |spec: &WeaponSpec| {
            super::super::ai_fire::spawn(
                spec,
                &ShipState::default(),
                &HullSpec::default(),
                None,
                RuleSource::Engine,
            )
        };
        for left in [1, 0, 0] {
            assert_eq!(
                supplies.pick(&mut armament, 150, RuleSource::Engine, &mut spawn),
                None
            );
            assert_eq!(supplies.rounds.slot(WeaponId(150)), left);
            assert_eq!(armament.mounts()[0].reload, 0.0, "never reloading");
        }
    }

    #[test]
    fn nothing_spawned_sets_no_reload_burst_or_last_gun() {
        let gun = WeaponRecord {
            burst_count: 2,
            burst_reload: 50,
            ..pick_launcher(130, -1, 30)
        };
        let mut armament = mounted(gun, 1);
        let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(130), 9)]), 0.0);
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut nothing);
        let mount = armament.mounts()[0];
        assert_eq!((mount.reload, mount.burst), (0.0, 0));
        assert_eq!(armament.last_gun(), None);
    }

    #[test]
    fn a_reloading_or_uncarried_pick_fires_and_spends_nothing() {
        let mut armament = mounted(pick_launcher(130, -1, 30), 1);
        let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(130), 9)]), 0.0);
        assert!(
            supplies
                .pick(&mut armament, 130, RuleSource::Engine, &mut ahead)
                .is_some()
        );
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 8);
        assert_eq!(
            supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead),
            None
        );
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 8, "reloading");
        assert_eq!(
            supplies.pick(&mut armament, 131, RuleSource::Engine, &mut ahead),
            None
        );
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 8, "not carried");
        let mut disabled = mounted(pick_launcher(130, -1, 30), 1);
        let volley = disabled.fire_pick(
            WeaponId(130),
            Condition::Disabled,
            supplies.shooter,
            &mut supplies.rounds,
            &mut supplies.fuel,
            &mut NeverFires,
            &mut supplies.reports,
            RuleSource::Engine,
            &mut ahead,
        );
        assert_eq!(volley, None);
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 8, "not intact");
    }

    #[test]
    fn simultaneous_copies_fire_one_shot_each_and_reload_by_the_whole_reload() {
        let together = WeaponRecord {
            flags: super::super::weapon::SIMULTANEOUS,
            ..pick_launcher(130, -1, 30)
        };
        let mut armament = mounted(together, 3);
        let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(130), 2)]), 0.0);
        let volley = supplies
            .pick(&mut armament, 130, RuleSource::Engine, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 2, "the third copy has no round");
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 0);
        assert_eq!(armament.mounts()[0].reload, 29.0, "30, a tick gone");
    }

    #[test]
    fn a_simultaneous_burst_counts_one_a_call_and_spends_the_shots_at_its_end() {
        let bursting = WeaponRecord {
            flags: super::super::weapon::SIMULTANEOUS,
            flags3: AMMO_AT_BURST_END,
            burst_count: 2,
            burst_reload: 50,
            ..pick_launcher(130, -1, 0)
        };
        let mut armament = mounted(bursting, 3);
        // The copies read the `AmmoType`'s slot (138); the burst's end
        // spends the launcher's own (130).
        let rounds = LauncherRounds::from([(WeaponId(130), 10), (WeaponId(138), 5)]);
        let mut supplies = npc_supplies(rounds, 0.0);
        let first = supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert_eq!(first.map(|volley| volley.launches.len()), Some(3));
        assert_eq!(armament.mounts()[0].burst, 1, "one a call");
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 10, "not yet");
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 7, "the 3 shots");
        assert_eq!(armament.mounts()[0].burst, 0);
        assert_eq!(armament.mounts()[0].reload, 49.0, "the burst reload");
        // Fuel at the end of a burst of one, floored at none.
        let fuelled = WeaponRecord {
            flags: 0,
            ammo_type: -1005,
            burst_count: 1,
            ..bursting
        };
        let mut armament = mounted(fuelled, 1);
        let mut supplies = npc_supplies(LauncherRounds::default(), 0.7);
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert!(
            (supplies.fuel.now - 0.2).abs() < 1e-5,
            "{:?}",
            supplies.fuel
        );
    }

    #[test]
    fn a_burst_of_copies_one_at_a_time_runs_burst_count_times_the_count_carried() {
        // `_WeaponBurstInterval` (@0x831c): BurstCount x the count for
        // copies that fire one at a time; the burst's end spends one round
        // at each multiple of BurstCount (@0x8907d-0x890ba).
        let bursting = WeaponRecord {
            flags3: AMMO_AT_BURST_END,
            burst_count: 2,
            burst_reload: 50,
            ..pick_launcher(130, -1, 0)
        };
        let mut armament = mounted(bursting, 2);
        let rounds = LauncherRounds::from([(WeaponId(130), 10), (WeaponId(138), 5)]);
        let mut supplies = npc_supplies(rounds, 0.0);
        let mut seen = Vec::new();
        for _ in 0..4 {
            let volley = supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
            assert_eq!(volley.map(|volley| volley.launches.len()), Some(1));
            let mount = armament.mounts()[0];
            seen.push((
                mount.burst,
                mount.reload,
                supplies.rounds.slot(WeaponId(130)),
            ));
        }
        assert_eq!(
            seen,
            [(1, 0.0, 10), (2, 0.0, 9), (3, 0.0, 9), (0, 49.0, 8)],
            "a round at bursts 2 and 4, the burst reload after the 4th"
        );
        assert_eq!(
            supplies.rounds.slot(WeaponId(138)),
            5,
            "the copies pay none"
        );
    }

    #[test]
    fn a_spawned_copy_reloads_by_its_share_of_reload() {
        let mut armament = mounted(pick_launcher(130, -1, 30), 3);
        let mut supplies = npc_supplies(LauncherRounds::from([(WeaponId(130), 9)]), 0.0);
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert_eq!(armament.mounts()[0].reload, 9.0, "30 / 3, a tick gone");
    }

    #[test]
    fn an_exclusive_weapon_holds_every_other_weapons_reload_at_its_own_plus_2() {
        let exclusive = WeaponRecord {
            flags3: super::super::weapon::EXCLUSIVE,
            ammo_type: -1,
            ..pick_launcher(130, -1, 10)
        };
        let arm = |exclusive: WeaponRecord| {
            let mut armament = Armament::new([
                (WeaponSpec::new(&exclusive), 1),
                (WeaponSpec::new(&blaster(131, 0)), 1),
                (WeaponSpec::new(&blaster(132, 50)), 1),
            ]);
            armament.hold(WeaponId(132));
            armament
        };
        let reloads = |armament: &Armament| -> Vec<f32> {
            armament.mounts().iter().map(|mount| mount.reload).collect()
        };
        let mut armament = arm(exclusive);
        let mut supplies = npc_supplies(LauncherRounds::default(), 0.0);
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert_eq!(reloads(&armament), [9.0, 11.0, 49.0], "10 + 2; 50 kept");
        let bursting = WeaponRecord {
            burst_count: 1,
            burst_reload: 40,
            ..exclusive
        };
        let mut armament = arm(bursting);
        supplies.pick(&mut armament, 130, RuleSource::Engine, &mut ahead);
        assert_eq!(
            reloads(&armament),
            [39.0, 41.0, 49.0],
            "the burst reload + 2"
        );
    }

    #[test]
    fn the_last_gun_fired_is_the_last_unguided_or_rocket_that_spawned() {
        let unlimited = |id, guidance| WeaponRecord {
            ammo_type: -1,
            ..pick_launcher(id, guidance, 0)
        };
        let mut armament = Armament::new(
            [(130, -1), (131, 6), (132, 0), (133, 1), (134, 4)]
                .map(|(id, guidance)| (WeaponSpec::new(&unlimited(id, guidance)), 1)),
        );
        let mut supplies = npc_supplies(LauncherRounds::default(), 0.0);
        let mut last = |pick| {
            supplies.pick(&mut armament, pick, RuleSource::Engine, &mut ahead);
            armament.last_gun().map(|id| id.0)
        };
        assert_eq!(last(130), Some(130));
        assert_eq!(last(131), Some(131));
        for other in [132, 133, 134] {
            assert_eq!(last(other), Some(131), "{other}");
        }
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
        let mut rounds = LauncherRounds::from([(WeaponId(138), 3)]);
        let mut reports = Reports::default();
        let mut asked = Vec::new();
        let mut unaimed = |spec: &WeaponSpec| {
            asked.push(spec.id);
            None
        };
        let launches = armament.fire(
            PRIMARY,
            Condition::Intact,
            Shooter::PLAYER,
            &mut rounds,
            &mut Gauge::default(),
            &mut NeverFires,
            &mut reports,
            &mut unaimed,
        );
        assert_eq!(launches, []);
        assert_eq!(asked, [WeaponId(139)]);
        assert_eq!(armament.mounts()[0].reload, 0.0, "no reload");
        assert_eq!(rounds.slot(WeaponId(138)), 3, "no round spent");
        let launches = armament.fire(
            PRIMARY,
            Condition::Intact,
            Shooter::PLAYER,
            &mut rounds,
            &mut Gauge::default(),
            &mut NeverFires,
            &mut reports,
            &mut at_seven,
        );
        assert_eq!(launches.len(), 1, "fired the next tick, aimed");
        assert_eq!(rounds.slot(WeaponId(138)), 2);
        assert_eq!(armament.mounts()[0].reload, 10.0);
    }

    #[test]
    fn nothing_fires_unless_the_ship_is_intact() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 5,
                starting: false,
            },
            Condition::Destroyed,
        ] {
            let mut armament = mounted(blaster(128, 0), 1);
            let launches = armament.fire(
                PRIMARY,
                condition,
                Shooter::PLAYER,
                &mut LauncherRounds::default(),
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
        let planet_type = WeaponRecord {
            flags2: 0x0400,
            ..blaster(181, 0)
        };
        let mut armament = mounted(planet_type, 1);
        let mut supplies = Supplies::none();
        assert_eq!(supplies.reports.take(), [], "not before it fires");
        assert_eq!(supplies.firing(&mut armament, PRIMARY, 5).len(), 5);
        assert_eq!(
            supplies.reports.take(),
            [SimDiagnostic::UnimplementedWeaponFlag {
                weapon: WeaponId(181),
                field: FlagField::Flags2,
                bit: 0x0400
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
        for guidance in [99, 11] {
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

    // The player's fire (`_FirePlayerWeapon`).

    impl Supplies {
        /// One call of `_FirePlayerWeapon` on `weapon`, spawned as `spawn`
        /// says, without the reload after it.
        fn player(
            &mut self,
            armament: &mut Armament,
            weapon: i16,
            spawn: &mut dyn FnMut(&WeaponSpec) -> Option<Aim>,
        ) -> Option<Volley> {
            armament.fire_player(
                WeaponId(weapon),
                Condition::Intact,
                self.shooter,
                &mut self.rounds,
                &mut self.fuel,
                &mut NeverFires,
                &mut self.reports,
                spawn,
            )
        }

        /// One tick of the player's trigger: each weapon `trigger` picks
        /// fired in `_HandlePlayer`'s order, then the reload; the weapons
        /// that fired, in order.
        fn player_tick(&mut self, armament: &mut Armament, trigger: Trigger) -> Vec<WeaponId> {
            let fired = armament
                .player_picks(trigger)
                .into_iter()
                .filter(|&weapon| self.player(armament, weapon.0, &mut ahead).is_some())
                .collect();
            armament.reload();
            fired
        }
    }

    /// A weapon `id` firing the rounds of `wëap` 138 (`AmmoType` 10) every
    /// `reload` ticks, its copies together under `flags`.
    fn player_launcher(id: i16, flags: u16, reload: i16) -> WeaponRecord {
        WeaponRecord {
            flags,
            ammo_type: 10,
            reload,
            ..weapon(id)
        }
    }

    #[test]
    fn a_players_simultaneous_weapon_fires_every_copy_and_reloads_by_its_whole_reload() {
        let mut armament = mounted(player_launcher(130, SIMULTANEOUS, 30), 2);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(138), 5);
        let volley = supplies
            .player(&mut armament, 130, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 2);
        assert_eq!(supplies.rounds.slot(WeaponId(138)), 3, "a round a copy");
        assert_eq!(armament.mounts()[0].reload, 30.0, "Reload, not Reload / 2");
        let mut apart = mounted(player_launcher(130, 0, 30), 2);
        let volley = supplies.player(&mut apart, 130, &mut ahead).expect("fired");
        assert_eq!(volley.launches.len(), 1, "one copy a call");
        assert_eq!(apart.mounts()[0].reload, 15.0, "Reload / 2");
    }

    #[test]
    fn a_players_copies_stop_when_its_ammunition_runs_out() {
        let mut armament = mounted(player_launcher(130, SIMULTANEOUS, 0), 3);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(138), 2);
        supplies.rounds.set(WeaponId(130), 9);
        let volley = supplies
            .player(&mut armament, 130, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 2, "the third copy has no round");
        assert_eq!(supplies.rounds.slot(WeaponId(138)), 0);
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 9, "the launcher's own");
    }

    #[test]
    fn at_the_burst_end_a_players_copies_are_capped_by_its_ammunition_or_its_fuel() {
        let late = WeaponRecord {
            flags3: AMMO_AT_BURST_END,
            burst_count: 5,
            ..player_launcher(130, SIMULTANEOUS, 0)
        };
        let mut armament = mounted(late, 4);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(138), 3);
        let volley = supplies
            .player(&mut armament, 130, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 3, "the rounds of the ammunition");
        assert_eq!(supplies.rounds.slot(WeaponId(138)), 3, "none spent yet");
        let fuelled = WeaponRecord {
            ammo_type: -1010,
            ..late
        };
        let mut armament = mounted(fuelled, 4);
        supplies.fuel = Gauge {
            now: 2.5,
            max: 10.0,
        };
        let volley = supplies
            .player(&mut armament, 130, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 2, "two shots of a unit each");
    }

    #[test]
    fn a_players_weapon_that_attempts_nothing_spends_nothing_and_does_not_reload() {
        let turret = WeaponRecord {
            guidance: 4,
            burst_count: 2,
            ..player_launcher(130, SIMULTANEOUS, 30)
        };
        let mut armament = mounted(turret, 2);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(138), 5);
        assert_eq!(supplies.player(&mut armament, 130, &mut nothing), None);
        assert_eq!(supplies.rounds.slot(WeaponId(138)), 5);
        let mount = armament.mounts()[0];
        assert_eq!((mount.reload, mount.burst), (0.0, 0));
    }

    #[test]
    fn a_players_copy_spends_its_fuel_floored_at_none() {
        let fuelled = WeaponRecord {
            ammo_type: -1010,
            ..player_launcher(130, SIMULTANEOUS, 0)
        };
        let mut armament = mounted(fuelled, 2);
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge {
            now: 1.5,
            max: 10.0,
        };
        let volley = supplies
            .player(&mut armament, 130, &mut ahead)
            .expect("fired");
        assert_eq!(volley.launches.len(), 1, "the second has not the unit");
        assert!(
            (supplies.fuel.now - 0.5).abs() < 1e-5,
            "{:?}",
            supplies.fuel
        );
    }

    #[test]
    fn a_players_simultaneous_burst_counts_one_a_call_and_spends_its_ammunition_at_the_end() {
        let bursting = WeaponRecord {
            flags3: AMMO_AT_BURST_END,
            burst_count: 2,
            burst_reload: 50,
            ..player_launcher(130, SIMULTANEOUS, 0)
        };
        let mut armament = mounted(bursting, 3);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(138), 10);
        supplies.rounds.set(WeaponId(130), 10);
        let first = supplies.player(&mut armament, 130, &mut ahead);
        assert_eq!(first.map(|volley| volley.launches.len()), Some(3));
        assert_eq!(
            supplies.rounds.slot(WeaponId(138)),
            10,
            "no spend mid-burst"
        );
        assert_eq!(armament.mounts()[0].burst, 1, "one a call");
        let second = supplies.player(&mut armament, 130, &mut ahead);
        assert_eq!(second.map(|volley| volley.launches.len()), Some(3));
        assert_eq!(
            supplies.rounds.slot(WeaponId(138)),
            7,
            "the three copies, from the ammunition's slot"
        );
        assert_eq!(supplies.rounds.slot(WeaponId(130)), 10, "not its own");
        let mount = armament.mounts()[0];
        assert_eq!((mount.burst, mount.reload), (0, 50.0));
        let one_at_a_time = WeaponRecord {
            flags: 0,
            ammo_type: -1010,
            ..bursting
        };
        let mut armament = mounted(one_at_a_time, 1);
        supplies.fuel = Gauge {
            now: 5.0,
            max: 10.0,
        };
        for _ in 0..2 {
            supplies.player(&mut armament, 130, &mut ahead);
        }
        assert!((supplies.fuel.now - 4.0).abs() < 1e-5, "a shot's fuel");
    }

    #[test]
    fn a_players_one_copy_weapon_fires_every_reload_and_two_every_half() {
        let mut one = mounted(blaster(128, 10), 1);
        let mut supplies = Supplies::none();
        let fired: Vec<u32> = (0..31)
            .filter(|_| !supplies.player_tick(&mut one, PRIMARY).is_empty())
            .collect();
        assert_eq!(fired, [0, 10, 20, 30]);
        let mut two = mounted(blaster(129, 13), 2);
        let fired: Vec<u32> = (0..15)
            .filter(|_| !supplies.player_tick(&mut two, PRIMARY).is_empty())
            .collect();
        assert_eq!(fired, [0, 7, 14]);
    }

    #[test]
    fn a_player_carrier_launches_both_its_simultaneous_bays_in_one_call() {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            flags: SIMULTANEOUS | super::super::weapon::SECONDARY,
            reload: 60,
            ..weapon(150)
        };
        let mut armament = mounted(bay, 2);
        let mut supplies = Supplies::none();
        supplies.rounds.set(WeaponId(150), 5);
        let volley = supplies
            .player(&mut armament, 150, &mut ahead)
            .expect("launched");
        assert_eq!(volley.launches.len(), 2);
        assert_eq!(supplies.rounds.slot(WeaponId(150)), 3, "its own slot");
        assert_eq!(armament.mounts()[0].reload, 60.0);
    }

    #[test]
    fn a_players_exclusive_weapon_holds_the_others_by_its_shot_reload_before_its_burst() {
        let exclusive = WeaponRecord {
            flags3: EXCLUSIVE,
            ammo_type: -1,
            burst_count: 1,
            burst_reload: 40,
            ..blaster(130, 10)
        };
        let mut armament = Armament::new([
            (WeaponSpec::new(&exclusive), 1),
            (WeaponSpec::new(&blaster(131, 0)), 1),
            (WeaponSpec::new(&blaster(132, 50)), 1),
        ]);
        armament.hold(WeaponId(132));
        Supplies::none().player(&mut armament, 130, &mut ahead);
        let reloads: Vec<f32> = armament.mounts().iter().map(|mount| mount.reload).collect();
        assert_eq!(reloads, [40.0, 12.0, 50.0], "10 + 2, not 40 + 2; 50 kept");
    }

    #[test]
    fn the_players_primaries_fire_by_weapon_id_and_its_secondary_after_them() {
        let missile = WeaponRecord {
            flags: super::super::weapon::SECONDARY,
            ..blaster(140, 0)
        };
        let armament = Armament::new([
            (WeaponSpec::new(&blaster(131, 0)), 1),
            (WeaponSpec::new(&missile), 1),
            (WeaponSpec::new(&blaster(128, 0)), 1),
            (WeaponSpec::new(&blaster(129, 0)), 1),
        ]);
        let both = Trigger {
            secondary: Some(WeaponId(140)),
            ..PRIMARY
        };
        assert_eq!(armament.player_picks(both), ids(&[128, 129, 131, 140]));
        assert_eq!(armament.player_picks(PRIMARY), ids(&[128, 129, 131]));
        let secondary = Trigger {
            primary: false,
            ..both
        };
        assert_eq!(armament.player_picks(secondary), ids(&[140]));
        assert_eq!(
            armament.player_picks(Trigger::default()),
            ids(&[]),
            "nothing held"
        );
    }

    #[test]
    fn an_exclusive_primary_holds_the_weapons_after_it_in_the_same_tick() {
        let exclusive = WeaponRecord {
            flags3: EXCLUSIVE,
            ammo_type: -1,
            ..blaster(129, 10)
        };
        let missile = WeaponRecord {
            flags: super::super::weapon::SECONDARY,
            ..blaster(140, 0)
        };
        let mut armament = Armament::new([
            (WeaponSpec::new(&blaster(131, 0)), 1),
            (WeaponSpec::new(&missile), 1),
            (WeaponSpec::new(&exclusive), 1),
            (WeaponSpec::new(&blaster(128, 0)), 1),
        ]);
        let both = Trigger {
            secondary: Some(WeaponId(140)),
            ..PRIMARY
        };
        assert_eq!(
            Supplies::none().player_tick(&mut armament, both),
            ids(&[128, 129]),
            "131 and the secondary held"
        );
    }

    #[test]
    fn a_players_fighter_bay_that_is_not_a_secondary_fires_on_the_primary() {
        let bay = WeaponRecord {
            guidance: 99,
            ammo_type: 200,
            ..weapon(150)
        };
        let armament = mounted(bay, 1);
        assert_eq!(armament.player_picks(PRIMARY), ids(&[150]));
    }

    #[test]
    fn the_player_fires_nothing_unless_intact_and_reports_an_unflown_guidance() {
        let mut armament = mounted(blaster(128, 0), 1);
        let mut supplies = Supplies::none();
        let fired = armament.fire_player(
            WeaponId(128),
            Condition::Disabled,
            supplies.shooter,
            &mut supplies.rounds,
            &mut supplies.fuel,
            &mut NeverFires,
            &mut supplies.reports,
            &mut ahead,
        );
        assert_eq!(fired, None);
        let mut odd = mounted(
            WeaponRecord {
                guidance: 42,
                ..blaster(130, 0)
            },
            1,
        );
        assert_eq!(supplies.player(&mut odd, 130, &mut ahead), None);
        assert_eq!(
            supplies.reports.take(),
            [SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(130),
                guidance: 42,
            }]
        );
        assert_eq!(
            supplies.player(&mut odd, 131, &mut ahead),
            None,
            "not carried"
        );
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
            supplies.shooter,
            &mut supplies.rounds,
            &mut supplies.fuel,
            chance,
            &mut supplies.reports,
            true,
            &mut |spec: &WeaponSpec| {
                asked.push(spec.id);
                pick.map(|(heading, missile)| (Pointing::Along(heading), missile))
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
            (launch.weapon.id, leaves(&launch), launch.target),
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
        supplies.rounds.set(WeaponId(155), 2);
        let mut chance = Draws::of(&[]);
        let mut asked = Vec::new();
        assert_eq!(
            defend(&mut armament, &mut supplies, &mut chance, None, &mut asked),
            None
        );
        assert_eq!(asked, [WeaponId(161)]);
        assert_eq!(armament.mounts()[0].reload, 0.0);
        assert_eq!(armament.mounts()[0].burst, 0);
        assert_eq!(supplies.rounds.slot(WeaponId(155)), 2);
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
        assert_eq!(leaves(&launch), 70.0, "its inaccuracy");
        assert_eq!(chance.asked, [40]);
        assert_eq!(supplies.rounds.slot(WeaponId(155)), 1, "a round");
        assert_eq!(armament.mounts()[0].burst, 1, "a shot of its burst");
        assert_eq!(armament.mounts()[0].reload, 4.0);
        supplies.rounds.set(WeaponId(155), 0);
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
        assert_eq!(leaves(&launch), 45.0);
        assert!(chance.asked.is_empty());
    }

    #[test]
    fn point_defence_fires_only_on_an_intact_ship_and_only_point_defence() {
        let mut armament = mounted(defence(133, 9, 5, 0), 1);
        let mut supplies = Supplies::none();
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 3,
                starting: false,
            },
        ] {
            let fired = armament.fire_point_defence(
                condition,
                supplies.shooter,
                &mut supplies.rounds,
                &mut supplies.fuel,
                &mut NeverFires,
                &mut supplies.reports,
                true,
                &mut |_: &WeaponSpec| Some((Pointing::Along(0.0), ShotId(0))),
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
                flags2: 0x0400,
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

    /// The ticks of `ticks` on which `armament`'s point defence fires at a
    /// missile always there, from `supplies`, each followed by a reload.
    fn defending(armament: &mut Armament, supplies: &mut Supplies, ticks: u32) -> Vec<u32> {
        (0..ticks)
            .filter(|_| {
                let fired = defend(
                    armament,
                    supplies,
                    &mut NeverFires,
                    Some((0.0, ShotId(1))),
                    &mut Vec::new(),
                );
                armament.reload();
                fired.is_some()
            })
            .collect()
    }

    #[test]
    fn point_defence_adds_its_reload_share_to_what_is_left_of_its_timer() {
        let mut armament = mounted(defence(133, 9, 5, 0), 2);
        armament.mounts[0].reload = -0.5;
        let mut supplies = Supplies::none();
        defend(
            &mut armament,
            &mut supplies,
            &mut NeverFires,
            Some((0.0, ShotId(1))),
            &mut Vec::new(),
        )
        .expect("fires");
        assert_eq!(armament.mounts()[0].reload, 2.0, "-0.5 and 5 / 2");
    }

    #[test]
    fn a_two_mount_quad_fires_every_two_and_a_half_ticks() {
        let mut armament = mounted(defence(133, 9, 5, 0), 2);
        assert_eq!(
            defending(&mut armament, &mut Supplies::none(), 11),
            [0, 3, 5, 8, 10]
        );
    }

    #[test]
    fn a_reload_timer_counts_down_past_none_once_then_rests_at_none() {
        let mut armament = mounted(defence(133, 9, 5, 0), 1);
        armament.mounts[0].reload = 0.5;
        let mut seen = Vec::new();
        for _ in 0..3 {
            armament.reload();
            seen.push(armament.mounts()[0].reload);
        }
        assert_eq!(seen, [-0.5, 0.0, 0.0]);
    }

    /// A point-defence weapon of `flags` and `flags3`, of no reload, firing
    /// bursts of 3 with a 20-tick burst reload, on 155's rounds (`AmmoType`
    /// 27).
    fn bursting_defence(flags: u16, flags3: u16) -> WeaponRecord {
        WeaponRecord {
            flags,
            flags3,
            burst_count: 3,
            burst_reload: 20,
            ammo_type: 27,
            ..defence(161, 9, 0, 0)
        }
    }

    #[test]
    fn point_defences_burst_runs_its_count_times_its_copies_or_its_count_if_simultaneous() {
        let fires = |flags| {
            let mut armament = mounted(bursting_defence(flags, 0), 2);
            let mut supplies = Supplies::none();
            supplies.rounds = slots(&[(155, 100)]);
            defending(&mut armament, &mut supplies, 10)
        };
        assert_eq!(fires(0), [0, 1, 2, 3, 4, 5], "3 x 2");
        assert_eq!(fires(SIMULTANEOUS), [0, 1, 2], "3, its copies together");
        let mut armament = mounted(bursting_defence(0, 0), 2);
        let mut supplies = Supplies::none();
        supplies.rounds = slots(&[(155, 100)]);
        defending(&mut armament, &mut supplies, 6);
        assert_eq!(
            (armament.mounts()[0].burst, armament.mounts()[0].reload),
            (0, 19.0),
            "the burst reload in place of the carry, a tick gone"
        );
    }

    #[test]
    fn point_defences_burst_end_sets_its_timer_to_the_burst_reload_dropping_the_carry() {
        // Reload 5 on 2 mounts, bursts of 2: an interval of 4 shots.
        let record = WeaponRecord {
            burst_count: 2,
            burst_reload: 20,
            ..defence(133, 9, 5, 0)
        };
        let mut armament = mounted(record, 2);
        armament.mounts[0].burst = 3;
        armament.mounts[0].reload = -0.5;
        defend(
            &mut armament,
            &mut Supplies::none(),
            &mut NeverFires,
            Some((0.0, ShotId(1))),
            &mut Vec::new(),
        )
        .expect("fires the burst's 4th shot");
        assert_eq!(
            (armament.mounts()[0].burst, armament.mounts()[0].reload),
            (0, 20.0),
            "the burst reload, not it and the carried -0.5 + 5 / 2"
        );
    }

    #[test]
    fn point_defence_using_ammo_at_the_end_of_a_burst_spends_a_round_each_interval() {
        // `AmmoType` 27 is 155's rounds, and 0 is 128's.
        for (shooter, ammo_type, slot) in [
            (Shooter::PLAYER, 27, 155),
            (Shooter::PLAYER, 0, 128),
            (NPC, 27, 161),
            (NPC, 0, 161),
        ] {
            let record = WeaponRecord {
                ammo_type,
                ..bursting_defence(0, AMMO_AT_BURST_END)
            };
            let mut armament = mounted(record, 2);
            let mut supplies = Supplies::none();
            supplies.shooter = shooter;
            supplies.rounds = slots(&[(slot, 10)]);
            let fired = defending(&mut armament, &mut supplies, 6);
            assert_eq!(fired, [0, 1, 2, 3, 4, 5], "{shooter:?}");
            assert_eq!(
                supplies.rounds.slot(WeaponId(slot)),
                9,
                "{shooter:?}: one round for the burst of 6"
            );
            defending(&mut armament, &mut supplies, 30);
            assert_eq!(supplies.rounds.slot(WeaponId(slot)), 8, "{shooter:?}");
        }
    }

    #[test]
    fn an_npcs_point_defence_of_an_ammo_type_past_255_using_ammo_at_a_bursts_end_never_pays() {
        let odd = WeaponRecord {
            ammo_type: 300,
            ..bursting_defence(0, AMMO_AT_BURST_END)
        };
        let mut armament = mounted(odd, 1);
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        supplies.rounds = slots(&[(161, 5)]);
        assert_eq!(defending(&mut armament, &mut supplies, 3), [0, 1, 2]);
        assert_eq!(armament.mounts()[0].reload, 19.0, "its burst ended");
        assert_eq!(supplies.rounds.slot(WeaponId(161)), 5, "nothing spent");
    }

    #[test]
    fn point_defence_on_fuel_spends_it_every_shot_even_using_ammo_at_the_end_of_a_burst() {
        // `AmmoType` -1010: a unit of fuel a shot.
        let fuelled = WeaponRecord {
            ammo_type: -1010,
            ..bursting_defence(0, AMMO_AT_BURST_END)
        };
        let mut armament = mounted(fuelled, 1);
        let mut supplies = Supplies::none();
        supplies.fuel = Gauge::full(10.0);
        assert_eq!(defending(&mut armament, &mut supplies, 3), [0, 1, 2]);
        assert_eq!(supplies.fuel.now, 7.0);
    }

    #[test]
    fn point_defence_tries_only_its_first_ready_weapon() {
        let mut armament = Armament::new(
            [defence(133, 9, 5, 0), defence(150, 10, 3, 0)]
                .iter()
                .map(|record| (WeaponSpec::new(record), 1)),
        );
        let mut supplies = Supplies::none();
        let mut asked = Vec::new();
        let fired = armament.fire_point_defence(
            Condition::Intact,
            supplies.shooter,
            &mut supplies.rounds,
            &mut supplies.fuel,
            &mut NeverFires,
            &mut supplies.reports,
            true,
            &mut |spec: &WeaponSpec| {
                asked.push(spec.id);
                // Only the longer-reaching beam has a missile in reach.
                (spec.id == WeaponId(150)).then_some((Pointing::Along(0.0), ShotId(1)))
            },
        );
        assert_eq!(fired, None, "the beam is never tried");
        assert_eq!(asked, [WeaponId(133)]);
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
                supplies.shooter,
                &mut supplies.rounds,
                &mut supplies.fuel,
                &mut NeverFires,
                &mut supplies.reports,
                &mut |spec: &WeaponSpec| {
                    asked.push(spec.id);
                    super::super::aim::aim(
                        spec,
                        &ShipState::default(),
                        &HullSpec::default(),
                        None,
                        RuleSource::Engine,
                    )
                },
            );
            assert_eq!(launches, [], "{guidance}");
            assert_eq!(armament.mounts()[0].reload, 0.0, "{guidance}");
        }
    }

    // `_WeaponHasAmmo`.

    /// An NPC, uncloaked, of a class with no `KeyCarried`.
    const NPC: Shooter = Shooter {
        npc: true,
        cloaked: false,
        key_carried: None,
    };

    /// Whether `armament`'s first mount has its ammunition for `shooter`,
    /// with `rounds` and 10 fuel.
    fn has(armament: &Armament, shooter: Shooter, rounds: &[(i16, i32)]) -> bool {
        let rounds = slots(rounds);
        armament.has_ammo(&armament.mounts()[0], &rounds, Gauge::full(10.0), shooter)
    }

    fn flagged(flags2: u16) -> WeaponRecord {
        WeaponRecord {
            flags2,
            ..blaster(128, 0)
        }
    }

    #[test]
    fn an_npc_never_has_ammunition_for_a_weapon_ai_ships_wont_use() {
        let wont = mounted(flagged(AI_WONT_USE), 1);
        assert!(!has(&wont, NPC, &[]));
        assert!(has(&wont, Shooter::PLAYER, &[]), "the player uses it");
        assert!(has(
            &mounted(flagged(!(AI_WONT_USE | NEEDS_KEY_SHIP)), 1),
            NPC,
            &[]
        ));
        assert_eq!(AI_WONT_USE, 0x0100);
    }

    #[test]
    fn a_cloaked_shooter_has_ammunition_only_for_a_weapon_that_fires_cloaked() {
        let cloaked = |shooter: Shooter| Shooter {
            cloaked: true,
            ..shooter
        };
        for shooter in [NPC, Shooter::PLAYER] {
            let plain = mounted(flagged(0), 1);
            assert!(has(&plain, shooter, &[]));
            assert!(!has(&plain, cloaked(shooter), &[]), "{shooter:?}");
            let fires = mounted(flagged(FIRES_CLOAKED), 1);
            assert!(has(&fires, cloaked(shooter), &[]), "{shooter:?}");
        }
        assert_eq!(FIRES_CLOAKED, 0x4000);
    }

    #[test]
    fn a_weapon_needing_its_key_ship_has_ammunition_only_with_one_aboard() {
        // The weapon, then a bay of ship 200 and one of ship 201.
        let armament = Armament::new([
            (WeaponSpec::new(&flagged(NEEDS_KEY_SHIP)), 1),
            (WeaponSpec::new(&bay(150, 200, 0)), 1),
            (WeaponSpec::new(&bay(151, 201, 0)), 1),
        ]);
        let keyed = |key| Shooter {
            key_carried: Some(ShipId(key)),
            ..NPC
        };
        assert!(has(&armament, keyed(200), &[(150, 1)]));
        assert!(has(&armament, keyed(201), &[(151, 3)]));
        assert!(
            has(
                &armament,
                Shooter {
                    key_carried: Some(ShipId(200)),
                    ..Shooter::PLAYER
                },
                &[(150, 1)]
            ),
            "the player's too"
        );
        assert!(!has(&armament, NPC, &[(150, 1)]), "no KeyCarried");
        assert!(!has(&armament, keyed(200), &[]), "the bay empty");
        assert!(
            !has(&armament, keyed(200), &[(151, 1)]),
            "another ship's bay"
        );
        assert!(!has(&armament, keyed(202), &[(150, 1), (151, 1)]));
        let unflagged = Armament::new([(WeaponSpec::new(&flagged(0)), 1)]);
        assert!(has(&unflagged, NPC, &[]), "no key needed");
        assert_eq!(NEEDS_KEY_SHIP, 0x0080);
    }

    #[test]
    fn a_weapon_needing_its_key_ship_still_pays_its_own_way() {
        let launcher = WeaponRecord {
            ammo_type: 10,
            ..flagged(NEEDS_KEY_SHIP)
        };
        let armament = Armament::new([
            (WeaponSpec::new(&launcher), 1),
            (WeaponSpec::new(&bay(150, 200, 0)), 1),
        ]);
        let keyed = Shooter {
            key_carried: Some(ShipId(200)),
            ..NPC
        };
        assert!(!has(&armament, keyed, &[(150, 1)]), "no rounds of its own");
        assert!(
            !has(&armament, keyed, &[(150, 1), (138, 1)]),
            "its ammunition's are not its own"
        );
        assert!(has(&armament, keyed, &[(150, 1), (128, 1)]));
    }

    #[test]
    fn rounds_and_fuel_still_decide_for_any_shooter() {
        let launcher = mounted(
            WeaponRecord {
                ammo_type: 10,
                ..blaster(128, 0)
            },
            1,
        );
        assert!(!has(&launcher, NPC, &[]));
        assert!(has(&launcher, NPC, &[(128, 1)]));
        assert!(has(&launcher, Shooter::PLAYER, &[(138, 1)]));
        let thirsty = mounted(
            WeaponRecord {
                ammo_type: -1200,
                ..blaster(128, 0)
            },
            1,
        );
        assert!(!has(&thirsty, NPC, &[]), "20 fuel a shot, 10 held");
        let sipping = mounted(
            WeaponRecord {
                ammo_type: -1100,
                ..blaster(128, 0)
            },
            1,
        );
        assert!(has(&sipping, Shooter::PLAYER, &[]), "10 a shot");
    }

    /// A launcher `id` of `guidance` firing the rounds of 155 (`AmmoType`
    /// 27), as 161 Storm does.
    fn on_155s_rounds(id: i16, guidance: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            ammo_type: 27,
            ..blaster(id, 0)
        }
    }

    fn slots(slots: &[(i16, i32)]) -> LauncherRounds {
        slots
            .iter()
            .map(|&(id, rounds)| (WeaponId(id), rounds))
            .collect()
    }

    #[test]
    fn an_npc_reads_and_spends_its_launchers_own_rounds_where_the_player_reads_its_ammunitions() {
        let mut launcher = mounted(on_155s_rounds(161, -1), 1);
        let has = |armament: &Armament, shooter: Shooter, rounds: &[(i16, i32)]| {
            armament.has_ammo(
                &armament.mounts()[0],
                &slots(rounds),
                Gauge::full(0.0),
                shooter,
            )
        };
        assert!(
            !has(&launcher, NPC, &[(155, 3)]),
            "its ammunition's rounds are not its own"
        );
        assert!(has(&launcher, NPC, &[(161, 1)]));
        assert!(has(&launcher, Shooter::PLAYER, &[(155, 3)]));
        assert!(
            !has(&launcher, Shooter::PLAYER, &[(161, 1)]),
            "the player's are its ammunition's"
        );
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        supplies.rounds = slots(&[(161, 1), (155, 3)]);
        assert_eq!(launched(&supplies.tick(&mut launcher, PRIMARY)), [161]);
        assert_eq!(supplies.rounds, slots(&[(161, 0), (155, 3)]));
        launcher.mounts[0].reload = 0.0;
        assert!(supplies.tick(&mut launcher, PRIMARY).is_empty(), "spent");
        supplies.shooter = Shooter::PLAYER;
        assert_eq!(launched(&supplies.tick(&mut launcher, PRIMARY)), [161]);
        assert_eq!(supplies.rounds, slots(&[(161, 0), (155, 2)]));
    }

    #[test]
    fn an_npcs_point_defence_spends_its_own_launchers_rounds() {
        let pick = Some((0.0, ShotId(1)));
        let mut storm = mounted(on_155s_rounds(161, 9), 1);
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        supplies.rounds = slots(&[(161, 2), (155, 3)]);
        let mut fired = Vec::new();
        for _ in 0..4 {
            if let Some((launch, _)) = defend(
                &mut storm,
                &mut supplies,
                &mut NeverFires,
                pick,
                &mut Vec::new(),
            ) {
                fired.push(launch.weapon.id);
            }
            storm.reload();
        }
        assert_eq!(fired, ids(&[161, 161]));
        assert_eq!(supplies.rounds, slots(&[(161, 0), (155, 3)]));
    }

    #[test]
    fn an_npcs_point_defence_of_an_ammo_type_past_255_drains_its_own_slot_and_still_fires() {
        let pick = Some((0.0, ShotId(1)));
        let mut odd = mounted(
            WeaponRecord {
                ammo_type: 300,
                ..defence(161, 9, 0, 0)
            },
            1,
        );
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        supplies.rounds = slots(&[(161, 1)]);
        for _ in 0..2 {
            assert!(
                defend(
                    &mut odd,
                    &mut supplies,
                    &mut NeverFires,
                    pick,
                    &mut Vec::new()
                )
                .is_some()
            );
        }
        assert_eq!(supplies.rounds, slots(&[(161, 0)]), "floored at none");
        let mut gun = mounted(
            WeaponRecord {
                ammo_type: 300,
                ..blaster(162, 0)
            },
            1,
        );
        supplies.rounds = slots(&[(162, 1)]);
        assert_eq!(launched(&supplies.tick(&mut gun, PRIMARY)), [162]);
        assert_eq!(
            supplies.rounds,
            slots(&[(162, 1)]),
            "a gun of it spends nothing"
        );
        supplies.shooter = Shooter::PLAYER;
        supplies.rounds = slots(&[(161, 1)]);
        defend(
            &mut odd,
            &mut supplies,
            &mut NeverFires,
            pick,
            &mut Vec::new(),
        );
        assert_eq!(
            supplies.rounds,
            slots(&[(161, 1)]),
            "the player's spends nothing"
        );
    }

    #[test]
    fn an_npcs_point_defence_of_a_negative_odd_ammo_type_fires_and_leaves_its_own_slot_whole() {
        let pick = Some((0.0, ShotId(1)));
        for ammo_type in [-500, -2] {
            let mut odd = mounted(
                WeaponRecord {
                    ammo_type,
                    ..defence(161, 9, 0, 0)
                },
                1,
            );
            let mut supplies = Supplies::none();
            supplies.shooter = NPC;
            supplies.rounds = slots(&[(161, 2)]);
            assert!(
                defend(
                    &mut odd,
                    &mut supplies,
                    &mut NeverFires,
                    pick,
                    &mut Vec::new()
                )
                .is_some(),
                "AmmoType {ammo_type} fires"
            );
            assert_eq!(
                supplies.rounds,
                slots(&[(161, 2)]),
                "AmmoType {ammo_type} spends nothing"
            );
        }
    }

    #[test]
    fn a_bay_of_any_ammo_type_reads_its_own_rounds() {
        let odd_bay = mounted(
            WeaponRecord {
                guidance: 99,
                ammo_type: 5,
                ..blaster(149, 0)
            },
            1,
        );
        assert_eq!(odd_bay.mounts()[0].spec.guidance, Guidance::Other(99));
        for shooter in [NPC, Shooter::PLAYER] {
            let has = |rounds: &[(i16, i32)]| {
                odd_bay.has_ammo(
                    &odd_bay.mounts()[0],
                    &slots(rounds),
                    Gauge::full(0.0),
                    shooter,
                )
            };
            assert!(has(&[(149, 1)]), "{shooter:?}");
            assert!(!has(&[(149, 0), (133, 4)]), "not 128 + 5's: {shooter:?}");
            assert!(!has(&[(149, -1)]), "{shooter:?}");
        }
        let mut odd_bay = odd_bay;
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        supplies.rounds = slots(&[(149, 1)]);
        let only = Trigger {
            only: Some(WeaponId(149)),
            ..Trigger::default()
        };
        assert!(
            supplies.tick(&mut odd_bay, only).is_empty(),
            "it flies nothing"
        );
        assert_eq!(
            supplies.reports.take(),
            [SimDiagnostic::UnimplementedGuidance {
                weapon: WeaponId(149),
                guidance: 99,
            }]
        );
    }

    #[test]
    fn launcher_rounds_hold_none_below_none_spend_above_none_and_stow_on_any() {
        let mut rounds = slots(&[(161, 2), (155, -1), (149, 0)]);
        assert_eq!(rounds.slot(WeaponId(155)), -1);
        assert_eq!(rounds.slot(WeaponId(140)), 0, "none named");
        assert_eq!(Rounds::held(&rounds, WeaponId(161)), 2);
        assert_eq!(Rounds::held(&rounds, WeaponId(155)), 0);
        rounds.spend(WeaponId(161));
        rounds.spend(WeaponId(155));
        rounds.spend(WeaponId(149));
        rounds.spend(WeaponId(140));
        assert_eq!(rounds, slots(&[(161, 1), (155, -1), (149, 0)]));
        rounds.stow(WeaponId(155));
        rounds.stow(WeaponId(149));
        rounds.stow(WeaponId(150));
        assert_eq!(rounds, slots(&[(161, 1), (155, 0), (149, 1), (150, 1)]));
        assert!(rounds.none_left(WeaponId(155)));
        assert!(!rounds.none_left(WeaponId(161)));
        let mut owing = slots(&[(155, -1)]);
        assert!(!owing.none_left(WeaponId(155)), "exactly none only");
        owing.stow(WeaponId(155));
        assert!(owing.none_left(WeaponId(155)));
        assert_eq!(
            rounds.iter().collect::<Vec<_>>(),
            [
                (WeaponId(149), 1),
                (WeaponId(150), 1),
                (WeaponId(155), 0),
                (WeaponId(161), 1)
            ]
        );
        assert_eq!(
            LauncherRounds::from([(WeaponId(161), 3)]),
            slots(&[(161, 3)])
        );
    }

    #[test]
    fn an_npc_picking_a_weapon_ai_ships_wont_use_fires_nothing() {
        let only = Trigger {
            only: Some(WeaponId(128)),
            ..Trigger::default()
        };
        let mut armament = mounted(flagged(AI_WONT_USE), 1);
        let mut supplies = Supplies::none();
        supplies.shooter = NPC;
        assert!(supplies.tick(&mut armament, only).is_empty());
        supplies.shooter = Shooter::PLAYER;
        assert_eq!(launched(&supplies.tick(&mut armament, PRIMARY)), [128]);
    }

    #[test]
    fn a_cloaked_ships_point_defence_fires_only_if_it_fires_cloaked() {
        let pick = Some((0.0, ShotId(1)));
        let cloaked = Shooter {
            cloaked: true,
            ..NPC
        };
        let mut plain = mounted(defence(133, 9, 0, 0), 1);
        let mut supplies = Supplies::none();
        supplies.shooter = cloaked;
        let fired = defend(
            &mut plain,
            &mut supplies,
            &mut NeverFires,
            pick,
            &mut Vec::new(),
        );
        assert_eq!(fired, None);
        let mut fires_cloaked = mounted(
            WeaponRecord {
                flags2: FIRES_CLOAKED,
                ..defence(134, 9, 0, 0)
            },
            1,
        );
        let fired = defend(
            &mut fires_cloaked,
            &mut supplies,
            &mut NeverFires,
            pick,
            &mut Vec::new(),
        );
        assert_eq!(
            fired.map(|(launch, _)| launch.weapon.id),
            Some(WeaponId(134))
        );
        supplies.shooter = NPC;
        let fired = defend(
            &mut plain,
            &mut supplies,
            &mut NeverFires,
            pick,
            &mut Vec::new(),
        );
        assert!(fired.is_some(), "uncloaked");
    }

    #[test]
    fn a_bay_launching_with_bays_reads_its_rounds_alone() {
        // `_AILaunchFighter` never asks `_WeaponHasAmmo`: a cloaked NPC
        // launches from a bay AI ships won't use.
        let (mut armament, mut supplies) = two_bays([2, 2]);
        supplies.shooter = Shooter {
            cloaked: true,
            ..NPC
        };
        armament.mounts[1].spec.flags2 = AI_WONT_USE;
        assert_eq!(launched(&supplies.tick(&mut armament, BAYS)), [149]);
        let mut armament = Armament::new([(
            WeaponSpec::new(&WeaponRecord {
                flags2: AI_WONT_USE,
                ..bay(149, 144, 0)
            }),
            1,
        )]);
        supplies.shooter = NPC;
        let only = Trigger {
            only: Some(WeaponId(149)),
            ..Trigger::default()
        };
        assert!(
            supplies.tick(&mut armament, only).is_empty(),
            "picked alone, it asks"
        );
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

    // What `E` and `H` add.

    /// `pairs` as a map of weapons to signed counts.
    fn array(pairs: &[(i16, i64)]) -> BTreeMap<WeaponId, i64> {
        pairs.iter().map(|&(id, n)| (WeaponId(id), n)).collect()
    }

    /// What `E` and `H` add to the arrays: `weapons`, then `rounds`.
    fn adds(weapons: &[(i16, i64)], rounds: &[(i16, i64)]) -> Arrays {
        Arrays {
            weapons: array(weapons),
            rounds: array(rounds),
        }
    }

    /// Ship 130's stock weapons and rounds as `E` and `H` add them, with
    /// bays loaded by `bays`, negative stock read by the engine.
    fn added(arsenal: &Arsenal, bays: RuleSource) -> Arrays {
        arsenal.stock_added(ShipId(130), bays, RuleSource::Engine)
    }

    #[test]
    fn e_and_h_add_each_stock_weapons_count_and_load_to_the_arrays() {
        let arsenal = arsenal();
        for bays in RuleSource::ALL {
            assert_eq!(
                arsenal.stock_added(ShipId(128), bays, RuleSource::Engine),
                adds(
                    &[(128, 2), (138, 1), (150, -1), (999, 1)],
                    &[(138, 20), (150, -4)]
                ),
                "{bays:?}: 999's load has no wëap to route it; 140 adds none"
            );
            assert_eq!(
                arsenal.stock_added(ShipId(128), bays, RuleSource::Bible),
                adds(&[(128, 2), (138, 1), (999, 1)], &[(138, 20)]),
                "{bays:?}: by the other reading nothing below one"
            );
        }
        let twice = stocking(&[], &[(128, 1, 0), (128, 3, 0), (138, 1, 20), (138, 1, 5)]);
        assert_eq!(
            added(&twice, RuleSource::Engine),
            adds(&[(128, 3), (138, 1)], &[(138, 5)]),
            "a later slot replaces an earlier one's count and load"
        );
        assert_eq!(
            arsenal.stock_added(ShipId(129), RuleSource::Engine, RuleSource::Engine),
            adds(&[], &[])
        );
    }

    #[test]
    fn e_and_h_sum_the_loads_that_share_an_ammunition() {
        let launcher = |id| WeaponRecord {
            ammo_type: 10,
            ..blaster(id, 15)
        };
        let shared = stocking(
            &[launcher(160), launcher(161)],
            &[(160, 1, 7), (161, 1, 12), (138, 0, 3)],
        );
        for bays in RuleSource::ALL {
            assert_eq!(
                added(&shared, bays),
                adds(&[(160, 1), (161, 1)], &[(138, 22)]),
                "{bays:?}: 7 + 12 + 3, where a purchase takes the larger"
            );
        }
        let full = stocking(
            &[launcher(160), launcher(161)],
            &[(160, 1, i16::MAX), (161, 1, i16::MAX), (138, 1, i16::MAX)],
        );
        assert_eq!(
            added(&full, RuleSource::Engine).rounds,
            array(&[(138, 3 * i64::from(i16::MAX))]),
            "wide, never wrapping"
        );
    }

    #[test]
    fn a_negative_load_shared_with_another_nets_by_the_engine_and_is_dropped_by_the_other_reading()
    {
        let launcher = |id| WeaponRecord {
            ammo_type: 10,
            ..blaster(id, 15)
        };
        let shared = stocking(
            &[launcher(160), launcher(161)],
            &[(160, 1, -1), (161, 1, 60)],
        );
        assert_eq!(
            shared.stock_added(ShipId(130), RuleSource::Engine, RuleSource::Engine),
            adds(&[(160, 1), (161, 1)], &[(138, 59)])
        );
        assert_eq!(
            shared.stock_added(ShipId(130), RuleSource::Engine, RuleSource::Bible),
            adds(&[(160, 1), (161, 1)], &[(138, 60)])
        );
        let alone = stocking(&[launcher(160)], &[(160, 0, -1)]);
        assert_eq!(
            alone.stock_added(ShipId(130), RuleSource::Engine, RuleSource::Engine),
            adds(&[], &[(138, -1)]),
            "a lone negative load"
        );
    }

    #[test]
    fn a_negative_weapon_count_takes_launchers_by_the_engine_and_nothing_by_the_other_reading() {
        let arsenal = stocking(&[], &[(128, -1, 0)]);
        assert_eq!(
            arsenal.stock_added(ShipId(130), RuleSource::Engine, RuleSource::Engine),
            adds(&[(128, -1)], &[])
        );
        assert_eq!(
            arsenal.stock_added(ShipId(130), RuleSource::Engine, RuleSource::Bible),
            adds(&[], &[])
        );
    }

    #[test]
    fn by_the_engine_a_bay_of_a_fighter_up_to_255_loads_the_weapon_its_ammo_type_names() {
        // Bay 171 carries ship 144: by the engine its 4 load weapon 272
        // (128 + 144).
        let bays = stocking(&[bay(171, 144, 50)], &[(171, 1, 4)]);
        assert_eq!(
            added(&bays, RuleSource::Engine),
            adds(&[(171, 1)], &[(272, 4)]),
            "weapon 272, which no outfit holds: the fighters are lost"
        );
        assert_eq!(
            added(&bays, RuleSource::Bible),
            adds(&[(171, 1)], &[(171, 4)]),
            "the bay's own fighters, as a purchase loads them"
        );
        let last = stocking(&[bay(171, 255, 50)], &[(171, 1, 4)]);
        assert_eq!(
            added(&last, RuleSource::Engine).rounds,
            array(&[(383, 4)]),
            "255 is the last AmmoType that names a weapon"
        );
    }

    #[test]
    fn a_bay_of_a_fighter_from_256_loads_itself_by_either_reading() {
        let bays = stocking(
            &[bay(176, 275, 50), bay(177, 256, 50)],
            &[(176, 1, 2), (177, 1, 3)],
        );
        for reading in RuleSource::ALL {
            assert_eq!(
                added(&bays, reading).rounds,
                array(&[(176, 2), (177, 3)]),
                "{reading:?}"
            );
        }
    }

    #[test]
    fn e_and_h_load_an_unlimited_or_fuel_weapon_itself() {
        let unlimited = WeaponRecord {
            ammo_type: -1,
            ..blaster(170, 15)
        };
        let fuelled = WeaponRecord {
            ammo_type: -1005,
            ..blaster(172, 15)
        };
        let weapons = stocking(&[unlimited, fuelled], &[(170, 1, 3), (172, 1, 6)]);
        for reading in RuleSource::ALL {
            assert_eq!(
                added(&weapons, reading).rounds,
                array(&[(170, 3), (172, 6)]),
                "{reading:?}"
            );
        }
    }

    fn counts(armament: &Armament) -> Vec<(i16, u32)> {
        armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id.0, mount.count))
            .collect()
    }

    #[test]
    fn a_classs_armament_is_its_stock_weapons_alone() {
        let arsenal = arsenal();
        assert_eq!(
            counts(&arsenal.of_class(ShipId(128))),
            [(128, 2), (138, 1)],
            "weapon 999 cannot be read, and 150 and 140 have no count"
        );
        assert_eq!(
            counts(&arsenal.of_class(ShipId(129))),
            [],
            "a ship type that cannot be read has no stock weapons"
        );
        assert_eq!(MOD_WEAPON, 1);
    }

    #[test]
    fn a_classs_later_slot_replaces_an_earlier_and_the_weapons_go_by_id() {
        let arsenal = stocking(&[], &[(138, 1, 20), (128, 2, 0), (138, 3, 40)]);
        assert_eq!(counts(&arsenal.of_class(ShipId(130))), [(128, 2), (138, 3)]);
        let arsenal = stocking(&[], &[(138, 1, 20), (138, 0, 40)]);
        assert_eq!(
            counts(&arsenal.of_class(ShipId(130))),
            [],
            "replaced by none"
        );
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
    fn an_npcs_stock_rounds_are_each_slots_load_on_its_own_launcher() {
        let hail = on_155s_rounds(155, 7);
        let storm = on_155s_rounds(161, 9);
        let arsenal = stocking(&[hail, storm], &[(155, 2, 40), (161, 1, 20)]);
        let (armament, rounds) = arsenal.npc(ShipId(130));
        assert_eq!(counts(&armament), [(155, 2), (161, 1)]);
        assert_eq!(rounds, slots(&[(155, 40), (161, 20)]), "not 60 of 155's");
        let arsenal = stocking(&[hail, storm], &[(155, 2, 40), (161, 1, 20), (155, 3, -1)]);
        let (armament, rounds) = arsenal.npc(ShipId(130));
        assert_eq!(counts(&armament), [(155, 3), (161, 1)]);
        assert_eq!(
            rounds,
            slots(&[(155, -1), (161, 20)]),
            "the later slot replaces the earlier, and -1 is kept"
        );
        let (_, rounds) = super::tests::arsenal().npc(ShipId(128));
        assert_eq!(
            rounds,
            slots(&[(128, 0), (138, 20), (140, 0), (150, -4)]),
            "every slot of a weapon that can be read, a count or not"
        );
        assert_eq!(
            super::tests::arsenal().npc(ShipId(129)),
            (Armament::default(), LauncherRounds::default())
        );
        assert_eq!(MOD_AMMO, 3);
    }

    #[test]
    fn an_arsenals_weapons_carry_their_chain_range() {
        let nanites = WeaponRecord {
            guidance: 4,
            count: 10,
            speed: 200,
            sub_count: 1,
            sub_type: Some(WeaponId(229)),
            ..blaster(163, 0)
        };
        let swarm = WeaponRecord {
            count: 30,
            speed: 300,
            ..blaster(229, 0)
        };
        assert_eq!(WeaponSpec::new(&nanites).range(), 20.0, "alone, its own");
        let arsenal = Arsenal::new(&[nanites, swarm], Vec::new());
        let spec = arsenal.weapon(WeaponId(163)).expect("read");
        assert_eq!(spec.range(), 110.0);
        assert_eq!(spec.flight(), 20.0);
        assert_eq!(
            arsenal.weapons().map(WeaponSpec::range).collect::<Vec<_>>(),
            [110.0, 90.0]
        );
    }

    #[test]
    fn a_ships_strength_is_its_records_raw_or_none() {
        let hull = |id, strength| HullRecord {
            strength,
            ..crate::testkit::hull(id)
        };
        let arsenal = Arsenal::new(&[], vec![hull(128, 325), hull(129, -2)]);
        assert_eq!(arsenal.strength(ShipId(128)), 325);
        assert_eq!(arsenal.strength(ShipId(129)), -2, "raw, below none");
        assert_eq!(arsenal.strength(ShipId(130)), 0, "no record");
    }

    #[test]
    fn a_hull_is_its_records_or_a_placeholder() {
        let arsenal = arsenal();
        assert_eq!(arsenal.hull(ShipId(128)).hit_radius, 30.0 * 0.33);
        assert_eq!(arsenal.hull(ShipId(129)), HullSpec::default());
        assert_eq!(arsenal.weapon(WeaponId(150)).map(|w| w.reload), Some(5.0));
        assert_eq!(arsenal.weapon(WeaponId(151)), None);
    }

    /// A weapon `id` of raw `guidance` flying `count` ticks at 10 pixels a
    /// tick, a beam 300 pixels long.
    fn reaching(id: i16, guidance: i16, count: i16) -> WeaponRecord {
        WeaponRecord {
            guidance,
            count,
            speed: 1000,
            beam_length: 300,
            ..blaster(id, 0)
        }
    }

    /// Ship 130's stock reach, stocking `slots` of `weapons`.
    fn stock_reach(weapons: &[WeaponRecord], slots: &[(i16, i16, i16)]) -> Option<f32> {
        stocking(weapons, slots).hull(ShipId(130)).stock_reach
    }

    #[test]
    fn a_hulls_stock_reach_is_its_longest_stock_weapon_of_guidance_8_or_less() {
        let near = reaching(150, -1, 10);
        let far = reaching(151, 8, 20);
        assert_eq!(
            stock_reach(&[near, far], &[(150, 1, 0), (151, 2, 0)]),
            Some(232.0),
            "the rear turret's 200, and 32"
        );
        let odd = WeaponRecord {
            speed: 1005,
            ..reaching(152, 1, 3)
        };
        assert_eq!(
            stock_reach(&[odd], &[(152, 1, 0)]),
            Some(62.0),
            "30.15 and 32, truncated"
        );
        assert_eq!(
            stock_reach(&[near, far], &[(150, 1, 0), (151, 0, 0)]),
            Some(132.0),
            "a slot of no count reaches nothing"
        );
        assert_eq!(
            stock_reach(&[near, far], &[(151, 2, 0), (150, 1, 0), (151, -1, 0)]),
            Some(132.0),
            "a later slot replaces an earlier"
        );
        let undocumented = reaching(153, -2, 10);
        assert_eq!(
            stock_reach(&[undocumented], &[(153, 1, 0)]),
            Some(32.0),
            "guidance below -1 is 8 or less, and reaches nothing"
        );
    }

    #[test]
    fn a_hulls_stock_reach_skips_point_defence_and_bays() {
        let near = reaching(150, -1, 10);
        let turret = reaching(151, 9, 50);
        let beam = reaching(152, 10, 50);
        let fighters = WeaponRecord {
            ammo_type: 144,
            ..reaching(153, 99, 50)
        };
        let odd_bay = reaching(154, 99, 50);
        let weird = reaching(155, 11, 50);
        assert_eq!(
            stock_reach(
                &[near, turret, beam, fighters, odd_bay, weird],
                &[
                    (150, 1, 0),
                    (151, 1, 0),
                    (152, 1, 0),
                    (153, 1, 4),
                    (154, 1, 0),
                    (155, 1, 0)
                ]
            ),
            Some(132.0)
        );
    }

    #[test]
    fn a_beams_stock_reach_is_its_length_and_32_and_a_bombs_is_32() {
        for guidance in [0, 3] {
            assert_eq!(
                stock_reach(&[reaching(150, guidance, 10)], &[(150, 1, 0)]),
                Some(332.0),
                "{guidance}"
            );
        }
        assert_eq!(
            stock_reach(&[reaching(150, 5, 10)], &[(150, 1, 0)]),
            Some(32.0)
        );
    }

    #[test]
    fn a_class_with_no_such_weapon_has_no_stock_reach() {
        let turret = reaching(151, 9, 50);
        assert_eq!(stock_reach(&[turret], &[(151, 1, 0)]), None);
        assert_eq!(stock_reach(&[], &[]), None);
        assert_eq!(arsenal().hull(ShipId(129)).stock_reach, None, "no record");
        assert_eq!(HullSpec::default().stock_reach, None);
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

    // Magazines: a launcher's link to its ammunition.

    /// A launcher, weapon `id`, of `AmmoType` `ammo_type` and `MaxAmmo`
    /// `max_ammo`.
    fn launcher(id: i16, ammo_type: i16, max_ammo: i16) -> WeaponRecord {
        WeaponRecord {
            ammo_type,
            max_ammo,
            ..blaster(id, 10)
        }
    }

    fn magazine(
        weapon: i16,
        ammo: i16,
        max_ammo: u32,
        carried: Option<i16>,
        ammo_outfit: Option<i16>,
    ) -> Magazine {
        Magazine {
            weapon: WeaponId(weapon),
            ammo: WeaponId(ammo),
            max_ammo,
            carried: carried.map(ShipId),
            ammo_outfit: ammo_outfit.map(OutfitId),
        }
    }

    #[test]
    fn a_launchers_magazine_is_its_first_weapon_mods_ammunition_and_max_ammo() {
        let arsenal = Arsenal::new(&[launcher(145, 10, 30)], Vec::new());
        let records = [
            outfit(226, &[(MOD_AMMO, 145)]),
            outfit(220, &[(4, 10), (MOD_WEAPON, 145)]),
            outfit(221, &[(MOD_AMMO, 138)]),
            outfit(225, &[(MOD_AMMO, 145)]),
        ];
        assert_eq!(
            arsenal.magazines(&records),
            BTreeMap::from([(OutfitId(220), magazine(145, 138, 30, None, Some(225)))]),
            "the lowest outfit naming the launcher's weapon, not its AmmoType's"
        );
    }

    #[test]
    fn a_bays_magazine_is_the_bay_itself_and_carries_its_fighter() {
        let arsenal = Arsenal::new(
            &[WeaponRecord {
                max_ammo: 4,
                ..bay(149, 144, 60)
            }],
            Vec::new(),
        );
        let records = [
            outfit(157, &[(MOD_WEAPON, 149)]),
            outfit(158, &[(MOD_AMMO, 149)]),
        ];
        assert_eq!(
            arsenal.magazines(&records),
            BTreeMap::from([(OutfitId(157), magazine(149, 149, 4, Some(144), Some(158)))])
        );
    }

    #[test]
    fn unlimited_or_fuel_ammo_counts_the_weapons_own_rounds() {
        let arsenal = Arsenal::new(&[launcher(170, -1, 5), launcher(172, -1005, 6)], Vec::new());
        let records = [
            outfit(230, &[(MOD_WEAPON, 170)]),
            outfit(231, &[(MOD_WEAPON, 172)]),
        ];
        assert_eq!(
            arsenal.magazines(&records),
            BTreeMap::from([
                (OutfitId(230), magazine(170, 170, 5, None, None)),
                (OutfitId(231), magazine(172, 172, 6, None, None)),
            ])
        );
    }

    #[test]
    fn no_magazine_without_a_readable_weapon_or_a_weapon_mod() {
        let arsenal = Arsenal::new(&[launcher(145, 10, 30), launcher(146, 11, 2)], Vec::new());
        let records = [
            outfit(240, &[(MOD_WEAPON, -1)]),
            outfit(241, &[(MOD_WEAPON, 999)]),
            outfit(242, &[(MOD_AMMO, 145)]),
            outfit(243, &[(MOD_WEAPON, -1), (MOD_WEAPON, 145)]),
            outfit(244, &[(4, 145), (MOD_WEAPON, 146), (MOD_WEAPON, 145)]),
        ];
        assert_eq!(
            arsenal.magazines(&records),
            BTreeMap::from([(OutfitId(244), magazine(146, 139, 2, None, None))]),
            "only the first weapon mod is read"
        );
    }

    #[test]
    fn a_launcher_of_max_ammo_0_still_has_its_magazine() {
        let arsenal = Arsenal::new(&[launcher(145, 10, 0)], Vec::new());
        let records = [outfit(220, &[(MOD_WEAPON, 145)])];
        assert_eq!(
            arsenal.magazines(&records),
            BTreeMap::from([(OutfitId(220), magazine(145, 138, 0, None, None))])
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
        assert_eq!(
            topped(&[(204, 20), (206, 2)]),
            tallied(&[(204, 20), (206, 2)]),
            "exactly as many: no outfit added, not even none of one"
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
    fn two_bays(rounds: [i32; 2]) -> (Armament, Supplies) {
        let armament = Armament::new([
            (WeaponSpec::new(&bay(150, 145, 70)), 1),
            (WeaponSpec::new(&bay(149, 144, 60)), 2),
        ]);
        let mut supplies = Supplies::none();
        supplies.rounds =
            LauncherRounds::from([(WeaponId(149), rounds[0]), (WeaponId(150), rounds[1])]);
        (armament, supplies)
    }

    const BAYS: Trigger = Trigger {
        primary: false,
        secondary: None,
        only: None,
        turrets_only: false,
        bays: true,
        point_defence: false,
        keep: None,
    };

    fn launched(launches: &[Launch]) -> Vec<i16> {
        launches.iter().map(|launch| launch.weapon.id.0).collect()
    }

    #[test]
    fn with_bays_only_the_lowest_bay_holding_rounds_launches() {
        let (mut armament, mut supplies) = two_bays([2, 2]);
        assert_eq!(launched(&supplies.tick(&mut armament, BAYS)), [149]);
        assert_eq!(supplies.rounds.slot(WeaponId(149)), 1, "a round spent");
        assert_eq!(supplies.rounds.slot(WeaponId(150)), 2);
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
        assert_eq!(supplies.rounds.slot(WeaponId(149)), 0);
        assert!(supplies.firing(&mut armament, BAYS, 100).is_empty());
        let (mut armament, mut supplies) = two_bays([2, 3]);
        assert_eq!(
            supplies.firing(&mut armament, BAYS, 102),
            [0, 30, 31, 101],
            "150 waits while 149 reloads with rounds, then launches every 70"
        );
        assert_eq!(supplies.rounds.slot(WeaponId(150)), 1);
    }

    #[test]
    fn a_secondary_bay_launches_as_any_secondary() {
        let (mut armament, mut supplies) = two_bays([0, 2]);
        let second = Trigger {
            secondary: Some(WeaponId(150)),
            ..Trigger::default()
        };
        assert_eq!(launched(&supplies.tick(&mut armament, second)), [150]);
        assert_eq!(supplies.rounds.slot(WeaponId(150)), 1);
        let empty = Trigger {
            secondary: Some(WeaponId(149)),
            ..Trigger::default()
        };
        assert!(supplies.tick(&mut armament, empty).is_empty(), "no rounds");
        assert_eq!(supplies.reports.take(), [], "a bay is flown, not reported");
    }

    #[test]
    fn stowing_a_fighter_puts_a_round_in_the_lowest_bay_launching_its_type() {
        let mut armament = Armament::new([
            (WeaponSpec::new(&blaster(128, 0)), 1),
            (WeaponSpec::new(&bay(150, 145, 70)), 1),
            (WeaponSpec::new(&bay(151, 144, 80)), 1),
            (WeaponSpec::new(&bay(149, 144, 60)), 1),
        ]);
        let mut rounds = LauncherRounds::from([(WeaponId(151), 1)]);
        assert!(armament.stow(ShipId(144), &mut rounds));
        assert_eq!(
            rounds,
            LauncherRounds::from([(WeaponId(149), 1), (WeaponId(151), 1)]),
            "149, the lower ID, though mounted later (@0x802f8-0x80326)"
        );
        let reloads = |armament: &Armament| -> Vec<f32> {
            armament.mounts().iter().map(|mount| mount.reload).collect()
        };
        assert_eq!(
            reloads(&armament),
            [0.0, 0.0, 0.0, 60.0],
            "an empty bay restarts its reload"
        );
        assert!(armament.stow(ShipId(144), &mut rounds), "with no cap");
        assert_eq!(rounds.slot(WeaponId(149)), 2);
        assert_eq!(reloads(&armament), [0.0, 0.0, 0.0, 60.0]);
        let mut owing = Armament::new([(WeaponSpec::new(&bay(149, 144, 60)), 1)]);
        let mut below = LauncherRounds::from([(WeaponId(149), -1)]);
        assert!(owing.stow(ShipId(144), &mut below));
        assert_eq!(below.slot(WeaponId(149)), 0);
        assert_eq!(
            owing.mounts()[0].reload,
            0.0,
            "a slot below none is not exactly none (@0x803a4)"
        );
        let mut armament = Armament::new([(WeaponSpec::new(&bay(151, 144, 80)), 1)]);
        armament.mounts[0].reload = 90.0;
        let mut empty = LauncherRounds::default();
        assert!(armament.stow(ShipId(144), &mut empty));
        assert_eq!(armament.mounts()[0].reload, 90.0, "a longer timer is kept");
        armament.mounts[0].reload = 5.0;
        assert!(armament.stow(ShipId(144), &mut empty));
        assert_eq!(armament.mounts()[0].reload, 5.0, "it held rounds");
        let before = armament.clone();
        let mut none = LauncherRounds::default();
        assert!(!armament.stow(ShipId(146), &mut none), "no bay launches it");
        assert_eq!(armament, before);
        assert_eq!(none, LauncherRounds::default());
    }

    #[test]
    fn the_bay_of_a_ship_is_the_lowest_bay_launching_it() {
        let armament = Armament::new([
            (WeaponSpec::new(&blaster(128, 0)), 1),
            (WeaponSpec::new(&bay(150, 145, 70)), 1),
            (WeaponSpec::new(&bay(151, 144, 80)), 1),
            (WeaponSpec::new(&bay(149, 144, 60)), 1),
        ]);
        assert_eq!(armament.bay_of(ShipId(144)), Some(WeaponId(149)));
        assert_eq!(armament.bay_of(ShipId(145)), Some(WeaponId(150)));
        assert_eq!(armament.bay_of(ShipId(146)), None, "no bay launches it");
    }

    #[test]
    fn the_lowest_ammo_outfit_is_the_lowest_outfit_that_is_its_rounds() {
        let sources = [
            (WeaponId(149), OutfitId(205)),
            (WeaponId(140), OutfitId(203)),
            (WeaponId(149), OutfitId(204)),
            (WeaponId(149), OutfitId(206)),
        ];
        assert_eq!(
            lowest_ammo_outfit(&sources, WeaponId(149)),
            Some(OutfitId(204))
        );
        assert_eq!(
            lowest_ammo_outfit(&sources, WeaponId(140)),
            Some(OutfitId(203))
        );
        assert_eq!(lowest_ammo_outfit(&sources, WeaponId(141)), None);
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
        let mut npc = LauncherRounds::default();
        npc.stow(WeaponId(149));
        npc.stow(WeaponId(149));
        assert_eq!(npc, LauncherRounds::from([(WeaponId(149), 2)]));
    }
}
