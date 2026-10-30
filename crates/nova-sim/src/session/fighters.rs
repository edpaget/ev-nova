//! The player's and the NPCs' fighter bays in flight: the session's side
//! of [`bay`](crate::bay).
//!
//! **Launching.** Each fighter a bay launches in the fight
//! ([`Sortie`]) is put in the system at its carrier, its shield and
//! armour full, its class's stock weapons and rounds aboard, its AI type
//! its `InherentAI`, pushed out at the bay's speed ([`launch_velocity`])
//! along the launch heading: the player's after its trigger, an NPC's
//! right after its carrier's turn in the NPCs' point-defence loop
//! ([`Session::dispatch_npcs`]), before `_HandleShot`
//! ([`Session::game_work`]), so each moves on its launch tick. An NPC's
//! fighter in a slot above its carrier's thinks then and takes its turn
//! in the rest of the loop, and every later slot's point defence can
//! choose it, as [`RuleKey::LaunchFrameAi`] reads. `_LaunchFighter` needs
//! a ship slot free below the reserve it leaves (8 @0x3d648, so none at
//! 55 NPCs, as [`RuleKey::SpawnReserve`] reads; see [`SPAWN_RESERVE`]):
//! the player's bay attempts a copy only while one is free, spending,
//! counting, sounding and reloading nothing for the rest
//! ([`Space::count_player_berths`], `_FirePlayerWeapon`
//! @0x62c61-0x62c72), and an NPC's launch has its slot already, as the
//! traffic counts the free slots as its NPCs fire (see
//! `nova_sim::traffic`'s room ledger). A fighter whose ship type has no
//! record goes back into its bay, a round again. The player's glow bay
//! lights the weapon glow only for a fighter put in the system.
//!
//! - **The player's fighter** joins the fleet as a carried escort
//!   ([`Escort::carried`]), of no government, on its formation slot, as
//!   phase 8's escorts. What it does first follows the rulebook's
//!   [`RuleKey::FighterLaunch`](crate::RuleKey::FighterLaunch)
//!   ([`Session::with_rules`]): by the engine it takes the
//!   standing order of the fleet's first escort of its class, Return to
//!   Hangar excepted, and no target (`_LaunchFighter` @0x3dbec, `_EscortAI`
//!   @0x83ae4-0x83b34); by the other reading, launched while the player
//!   targets a ship that is not one of its escorts, it attacks that ship
//!   at once. Launching makes no save due, as firing ammunition does not.
//! - **An NPC's fighter** is led by its carrier (+0x9a = the carrier,
//!   @0x3d866-0x3d870), of the carrier's government, with goal 0 and no
//!   order of its own (`_AISetup` @0x7f3e8, @0x7f40c), and takes the
//!   carrier's target unless that is the carrier or a ship the carrier
//!   leads (@0x3dbdf-0x3dc11, @0x3dd0f-0x3dd47): the carrier's own lead
//!   and that lead's other escorts are kept. It flies by
//!   [`FleetEscortAi`](crate::FleetEscortAi) under the carrier, or by the
//!   other reading of [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi)
//!   by [`CarriedAi`](crate::CarriedAi). Its side for hits, sparing and
//!   point defence is still its carrier's lead's: the root of its chain
//!   of leads (`_ShipsShareParent` @0x7926-0x79c1,
//!   [`Npc::fleet_among`]).
//!
//! **Every fighter** launches full at a fighter's capacities
//! ([`RuleKey::FighterCapacity`]) and glides: its AI wait is its bay's
//! `Count` (+0x4c, @0x3d843-0x3d85b), so its high level, thrust and fire
//! are skipped until a think ends the wait, as `_EscortAI`'s Defend,
//! Attack and formation blocks do (@0x83c6b, @0x83d62, @0x83ea8). Under
//! Hold or Return it coasts the whole `Count`.
//!
//! **Docking.** A fighter that docks ([`Outcome::Docked`]) goes back
//! aboard its carrier ([`Armament::stow`](crate::Armament::stow)), in the
//! bay of its type or its sprite base: the player's leaves the fleet, its
//! round going to the first fighter outfit of its bay, with no save due.
//! One with no bay stays out at its carrier (see the `fighter_homes`
//! module), or by the other reading of [`RuleKey::FighterNoBay`] is lost,
//! which makes a save due. An NPC's fighter whose carrier is
//! gone from the system or not intact is succeeded at the next think, its
//! carrier being its leader (`nova_sim_ai::traffic::succession`): it leads
//! the carrier's fleet or follows the fighter that does, an escort now,
//! off its carrier. By the other reading of
//! [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) it leaves its
//! fleet with no carrier and no leader (see the `lost_lead` module).

use std::collections::BTreeMap;

use super::{Outbox, PlayerShip, Session, Setup, Space, Statics};
use crate::ai::pursuit::Pursuit;
use crate::ai::{Goal, leave};
#[cfg(doc)]
use crate::ai::{afterburner, swarm};
use crate::bay::{Carrier, FighterNote, capacity, dock_window, launch_velocity};
use crate::catalog::{OutfitId, ShipId};
use crate::chance::Chance;
use crate::combat::armament::{Fold, OutfitRounds, Rounds, Trigger, outfit_rounds};
use crate::combat::hull::Condition;
use crate::combat::ion::Ion;
use crate::combat::weapon::WeaponSpec;
use crate::combat::{ShipRef, Sortie};
use crate::escort::{EscortDuty, EscortOrder, FormationSlot, NO_SPRITE};
use crate::flight::ShipState;
use crate::has_max;
use crate::hyperspace::JUMP_FUEL;
use crate::outfitter::Launcher;
use crate::pilot::{Escort, Pilot};
use crate::rulebook::{RuleKey, RuleSource};
use crate::skill::random_skill_level;
use crate::traffic::SPAWN_RESERVE;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{AiType, Mode, Npc, NpcHold, NpcId};
use crate::traffic::ship_display::{AnimRoll, NpcDisplay, Spawn};
use crate::traffic::table::ShipKind;

impl Session {
    /// Puts each fighter of `sorties` in the system (see the module
    /// docs); one with no room, or whose ship type has no record, goes
    /// back into its bay. Each rolls its afterburner on `chance`. The
    /// NPCs put in the system, in order.
    ///
    /// Once one of the player's, from a glow bay (`Flags2` 0x0200), is put
    /// in the system, the player's weapon glow lights, once
    /// ([`Combat::light_glow`](crate::combat::Combat::light_glow)), as
    /// `_FirePlayerWeapon` reaches the glow from a bay only when
    /// `_LaunchFighter` returns a fighter (@0x62c61-0x62c6e,
    /// @0x62c78-0x62c98). An NPC's launch never lights it, as
    /// `_AILaunchFighter` never writes it.
    pub(super) fn launch_fighters(
        &mut self,
        sorties: &[Sortie],
        chance: &mut dyn Chance,
    ) -> Vec<NpcId> {
        let mut glows = false;
        let mut placed = Vec::new();
        for sortie in sorties {
            let Some(id) = self.launch_fighter(sortie, chance) else {
                self.space.put_back(&mut self.pilot, &self.statics, sortie);
                continue;
            };
            placed.push(id);
            if sortie.carrier == ShipRef::Player {
                glows |= self
                    .statics
                    .arsenal
                    .weapon(sortie.bay)
                    .is_some_and(WeaponSpec::glows);
            }
        }
        if glows {
            self.space.combat.light_glow(ShipRef::Player);
        }
        placed
    }

    /// Puts the fighter of `sortie` in the system, its afterburner rolled
    /// on `chance` as `_LaunchFighter` asks `_AIHasAfterburner` (@0x3d736,
    /// see [`afterburner`] and the `afterburner_roll` module), none at a
    /// berth another ship still names as its swarm leader
    /// ([`swarm::named`]): its number, or none when it found no room or
    /// no record of its ship type. A dying NPC's last launch comes this way
    /// too (see the `npc_escape` module).
    pub(super) fn launch_fighter(
        &mut self,
        sortie: &Sortie,
        chance: &mut dyn Chance,
    ) -> Option<NpcId> {
        let berth = self
            .space
            .traffic
            .free_berth(self.setup.spawn_reserve(SPAWN_RESERVE))?;
        let record = self.statics.ship_record(sortie.ship).cloned()?;
        let bay = self.statics.arsenal.weapon(sortie.bay).copied();
        let push = bay.map_or(0.0, |bay| bay.speed);
        let kind = self.setup.ship_kind(&self.statics, &record);
        let carrier = Carrier {
            ship: sortie.carrier,
            window: dock_window(record.fields.maneuver),
            reach: self.reach_of(sortie.carrier),
        };
        // It takes its carrier's speed along the heading
        // (`_LaunchFighter` @0x3d6d5-0x3d6e0).
        let speed = match sortie.carrier {
            ShipRef::Player => self.ship.player_speed,
            ShipRef::Npc(id) => self.space.npc(id).map_or(0.0, |npc| npc.speed),
        };
        let at = ShipState {
            position: sortie.from.position,
            velocity: sortie.from.velocity,
            heading: sortie.heading,
        };
        let capacity = self.setup.rules.source_for(RuleKey::FighterCapacity);
        let mut npc = fighter(&kind, sortie.ship, at, speed, carrier, capacity);
        // `_GenericRandomShipSpawn` rolls its skill (@0x3cad1, from
        // `_LaunchFighter` @0x3d657), and it takes its carrier's government
        // and factor.
        npc.skill = random_skill_level(kind.skill_var, &mut *chance);
        if let ShipRef::Npc(id) = sortie.carrier
            && let Some(launcher) = self.space.npc(id)
        {
            npc.take_govt_of(launcher);
        }
        // `_LaunchFighter` sets the fighter's leader before it reads its
        // `_ShipMaxSpeed` (@0x3d866-0x3d870, @0x3dc70), after its spawn, so
        // it is pushed against its own top speed, at its skill and factor,
        // and the player's against its led top speed.
        let own = npc.class_handling().max_speed;
        let top_speed = match sortie.carrier {
            ShipRef::Player => own * self.led_speed(),
            ShipRef::Npc(_) => own,
        };
        npc.state.velocity = launch_velocity(sortie.from.velocity, sortie.heading, push, top_speed);
        npc.afterburner = self.rolls_afterburner(&kind.hull, berth, chance);
        // The glide: +0x4c = the bay's `Count` (@0x3d843-0x3d85b), which
        // its first think ends unless it holds or returns.
        npc.ai_wait = bay.map_or(0, |bay| bay.lifetime);
        let id = match sortie.carrier {
            ShipRef::Player => Some(self.join_as_fighter(npc, sortie.target)),
            ShipRef::Npc(id) => self.space.join_carrier(npc, id),
        }?;
        // `_LaunchFighter` rolls its voice, the player's fighter and an
        // NPC carrier's alike (@0x3d9cc-0x3da3a; see the `voices` module).
        self.space.roll_voice(id, &self.statics, &mut *chance);
        Some(id)
    }

    /// The reach of a fighter docking with `carrier`: its sprite's width,
    /// or [`NO_SPRITE`] with none.
    pub(super) fn reach_of(&self, carrier: ShipRef) -> f32 {
        let sprite = match carrier {
            ShipRef::Player => self.ship.fit.hull.sprite_size,
            ShipRef::Npc(id) => self.space.npc(id).and_then(|npc| npc.hull.sprite_size),
        };
        sprite.unwrap_or(NO_SPRITE)
    }

    /// The player's fighter `npc` joins the fleet, launched while the
    /// player targets `target` (see the module docs): its number.
    ///
    /// A lone launch's clear of a kept Return to Hangar
    /// ([`Space::clear_lone_return`]) is reported as the class-order
    /// bookkeeping's clears are: `_LaunchFighter` writes -1 (@0x3db4d), and
    /// the bookkeeping later in the same `_HandlePlayer` clears -1 to 0
    /// (@0x6e42d-0x6e446), as the new fighter is unmarked and no other
    /// ship is the player's, showing the escort menu again while it shows
    /// (see the `class_orders` module). The engine has no -1, so the launch
    /// reports it.
    fn join_as_fighter(&mut self, mut npc: Npc, target: Option<ShipRef>) -> NpcId {
        let escort_type = self
            .statics
            .ship_record(npc.ship)
            .map(|record| record.escort_type);
        if self.space.clear_lone_return(escort_type) {
            self.out.class_orders_cleared = true;
        }
        let target = target
            .filter(|_| self.setup.rules.source_for(RuleKey::FighterLaunch) == RuleSource::Bible)
            .filter(|&ship| match ship {
                ShipRef::Npc(id) => !self.is_escort(id),
                ShipRef::Player => false,
            });
        let order = if target.is_some() {
            Some(EscortOrder::Attack)
        } else {
            self.space
                .class_order(&self.pilot, &self.statics, npc.class)
        };
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order,
        });
        npc.goal = Goal::Formation { guard: None };
        npc.target = target;
        let escort = Escort {
            ship: npc.ship,
            reserves: npc.reserves,
            order,
            carried: true,
            wage: None,
            person: None,
            mark: None,
        };
        let id = self.space.traffic.add_npc(npc);
        self.pilot.escorts.push(escort);
        self.space
            .escorts
            .fleet
            .resize(self.pilot.escorts.len() - 1, None);
        self.space.escorts.fleet.push(Some(id));
        self.reform();
        id
    }

    /// Takes NPC `id` aboard its carrier when its flight's `outcome` docked
    /// it (see the module docs): the player's leaves the fleet, its round
    /// to its bay, and one with no bay, docking by the other reading of
    /// [`RuleKey::FighterNoBay`], is lost, which makes a save due; any
    /// other outcome is left alone. The ship loop settles each dock at the fighter's own
    /// slot, as `_LowLevelAIHandler` calls `_AIAddFighterToParent` there
    /// (@0x87fef), so no later slot's step finds the player's fighter gone
    /// from the system but still in the fleet.
    pub(super) fn dock_fighter(&mut self, id: NpcId, outcome: Outcome) {
        let Outcome::Docked { carrier, ship } = outcome else {
            return;
        };
        match carrier {
            ShipRef::Player => {
                let base = self.statics.arsenal.sprite_base(ship);
                let stowed = self
                    .ship
                    .stow_fighter(&mut self.pilot, &self.statics, ship, base);
                self.unfleet(id);
                if !stowed {
                    self.out.save_due = true;
                }
            }
            ShipRef::Npc(carrier) => {
                let base = self.statics.arsenal.sprite_base(ship);
                if let Some(carrier) = self.space.npc_mut(carrier) {
                    carrier.armament.stow(ship, base, &mut carrier.rounds);
                }
            }
        }
    }

    /// The player leaves the system with its fighters out, as the
    /// rulebook's [`RuleKey::FighterRecall`](crate::RuleKey::FighterRecall)
    /// says (see the module docs): by the engine, on a jump's arrival
    /// (`landing` false), those that can jump follow and the rest are
    /// abandoned ([`Session::abandon_fighters`]); by the other reading, on arrival and on
    /// landing, every one goes back aboard.
    pub(super) fn leave_with_fighters(&mut self, landing: bool) {
        match self.setup.rules.source_for(RuleKey::FighterRecall) {
            RuleSource::Engine if !landing => self.abandon_fighters(),
            RuleSource::Engine => {}
            RuleSource::Bible => self.recall_fighters(),
        }
    }

    /// Every fighter out that cannot jump is abandoned (`_HandlePlayer`
    /// @0x6c0b9-0x6c0f9, its `_AIShipHasFuelForJump` @0x6c0ed): one in the
    /// system by [`leave::can_jump`] (its class's `Fuel` above 99 and no
    /// other ship's tractor beam on it; by the other reading of
    /// [`RuleKey::NpcJumpFuel`], a jump's fuel now too), one not there by
    /// its ship type's `Fuel` alone. Its record goes, with no round back,
    /// and the flight is told how many; a save is due when any was.
    fn abandon_fighters(&mut self) {
        let reading = self.setup.rules.source_for(RuleKey::NpcJumpFuel);
        let fuelled = |session: &Self, escort: &Escort, npc: Option<&Npc>| {
            fighter_can_jump(&session.statics, reading, escort.ship, npc)
        };
        let abandoned = self.drop_fighters(fuelled);
        let count = u32::try_from(abandoned.len()).unwrap_or(u32::MAX);
        if count > 0 {
            self.out.fighter_notes.push(FighterNote::Abandoned(count));
            self.out.save_due = true;
        }
    }
}

/// Whether the player's fighter out, of ship type `ship`, can follow a
/// jump, its NPC `npc` when it is in the system (`_HandlePlayer`'s
/// `_AIShipHasFuelForJump` @0x6c0ed): one in the system by
/// [`leave::can_jump`] by `reading` of
/// [`RuleKey::NpcJumpFuel`], one not there by its ship type's `Fuel`
/// alone. [`Session::abandon_fighters`] drops the rest, and a jump's days
/// count only these (see the `trip_days` module).
pub(super) fn fighter_can_jump(
    statics: &Statics,
    reading: RuleSource,
    ship: ShipId,
    npc: Option<&Npc>,
) -> bool {
    match npc {
        Some(npc) => leave::can_jump(npc, reading),
        None => statics
            .ship_record(ship)
            .is_some_and(|record| f32::from(record.fields.fuel) >= JUMP_FUEL),
    }
}

impl Session {
    /// Every fighter out goes back aboard at once, a round to the bay
    /// launching its exact type, which is all `_InstantFighterRecall`
    /// walks (@0x5df3, @0x5e45-0x5e7e); one no bay launches is lost, which
    /// makes a save due.
    fn recall_fighters(&mut self) {
        for fighter in self.drop_fighters(|_, _, _| false) {
            if !self
                .ship
                .stow_fighter(&mut self.pilot, &self.statics, fighter.ship, fighter.ship)
            {
                self.out.save_due = true;
            }
        }
    }

    /// Every fighter out leaves the fleet but those `kept` (asked of its
    /// record and its NPC, when it is in the system), its NPC gone from the
    /// system and its record from the pilot; the records gone.
    pub(super) fn drop_fighters(
        &mut self,
        kept: impl Fn(&Self, &Escort, Option<&Npc>) -> bool,
    ) -> Vec<Escort> {
        let mut dropped = Vec::new();
        for index in (0..self.pilot.escorts.len()).rev() {
            let escort = self.pilot.escorts[index];
            let npc = self
                .space
                .escorts
                .fleet
                .get(index)
                .copied()
                .flatten()
                .and_then(|id| self.space.npc(id));
            if !escort.carried || kept(self, &escort, npc) {
                continue;
            }
            self.drop_fleet_escort(index);
            dropped.push(escort);
        }
        dropped.reverse();
        self.reform();
        dropped
    }

    /// For every ammunition outfit of a fighter bay, the fighters the
    /// player's bays can still take ([`capacity`], `_CanBuyFighter`
    /// @0x5a82): the most its bays hold, less the fighters aboard and out
    /// of that type; none when the ship carries no such bay.
    pub(super) fn fighter_room(&self) -> BTreeMap<OutfitId, u32> {
        let mut room = BTreeMap::new();
        for &(ammo, outfit) in &self.statics.ammo_outfits {
            let Some(bay) = self
                .statics
                .arsenal
                .weapon(ammo)
                .filter(|spec| spec.is_bay())
            else {
                continue;
            };
            let bays: u32 = self
                .ship
                .fit
                .armament
                .mounts()
                .iter()
                .filter(|mount| mount.spec.id == ammo)
                .map(|mount| mount.count)
                .sum();
            let max = self
                .statics
                .outfits
                .iter()
                .find(|record| record.id == outfit)
                .map_or(0, |record| record.max);
            let held = outfit_rounds(&self.pilot.outfits, &self.statics.ammo_outfits, ammo)
                .saturating_add(self.fighters_out(bay.carried));
            room.insert(
                outfit,
                capacity(bay.max_ammo, bays, max).saturating_sub(held),
            );
        }
        room
    }

    /// Whether the player's bays have room for a boarded ship of type
    /// `ship`, as `_CanBuyFighter(type, -1, -1)` asks (@0x5a82-0x5cb2; see
    /// the `board_repair` module): the first bay held, by weapon ID,
    /// carrying `ship` (@0x5ad1-0x5b04, breaking @0x5b7e), else the first
    /// whose carried ship shares `ship`'s sprite base (@0x5b0d-0x5b75),
    /// wider than the dock's walk ([`Armament::dock_bay`](crate::Armament::dock_bay)),
    /// and its last fighter outfit by ID (@0x5b95-0x5bd8), none of which
    /// is no room (@0x5bda). True while the bay's [`capacity`] is above
    /// its rounds and the fighters out of type `ship`, the type asked and
    /// not the bay's (@0x5c67-0x5c9b).
    pub(super) fn can_take_fighter(&self, ship: ShipId) -> bool {
        let arsenal = &self.statics.arsenal;
        let base = arsenal.sprite_base(ship);
        let first_bay = |carries: &dyn Fn(ShipId) -> bool| {
            self.ship
                .fit
                .armament
                .mounts()
                .iter()
                .filter(|mount| mount.spec.carried.is_some_and(carries))
                .min_by_key(|mount| mount.spec.id)
        };
        let bay = first_bay(&|carried| carried == ship)
            .or_else(|| first_bay(&|carried| arsenal.sprite_base(carried) == base));
        let Some(bay) = bay else {
            return false;
        };
        let last_outfit = self
            .statics
            .ammo_outfits
            .iter()
            .filter(|&&(ammo, _)| ammo == bay.spec.id)
            .map(|&(_, outfit)| outfit)
            .max();
        let Some(outfit) = last_outfit else {
            return false;
        };
        let max = self
            .statics
            .outfits
            .iter()
            .find(|record| record.id == outfit)
            .map_or(0, |record| record.max);
        let held = outfit_rounds(&self.pilot.outfits, &self.statics.ammo_outfits, bay.spec.id)
            .saturating_add(self.fighters_out(Some(ship)));
        capacity(bay.spec.max_ammo, bay.count, max) > held
    }

    /// The player's fighters out of type `carried`: none for `None`.
    fn fighters_out(&self, carried: Option<ShipId>) -> u32 {
        let out = self
            .pilot
            .escorts
            .iter()
            .filter(|escort| escort.carried && Some(escort.ship) == carried)
            .count();
        u32::try_from(out).unwrap_or(u32::MAX)
    }

    /// Each launcher outfit ([`Arsenal::magazines`](crate::combat::armament::Arsenal::magazines)),
    /// with the rounds of its ammunition the player holds, a bay's
    /// fighters out counted, as the outfitter's Sell counts them
    /// (`_DoOutfitDialog` @0x5cadb-0x5cb8e).
    pub(super) fn launchers(&self) -> BTreeMap<OutfitId, Launcher> {
        self.statics
            .arsenal
            .magazines(&self.statics.outfits)
            .into_iter()
            .map(|(outfit, magazine)| {
                let aboard = outfit_rounds(
                    &self.pilot.outfits,
                    &self.statics.ammo_outfits,
                    magazine.ammo,
                );
                let out = if magazine.carried.is_some() {
                    self.fighters_out(magazine.carried)
                } else {
                    0
                };
                let launcher = Launcher {
                    max_ammo: magazine.max_ammo,
                    rounds: aboard.saturating_add(out),
                    ammo: magazine.ammo_outfit,
                };
                (outfit, launcher)
            })
            .collect()
    }

    /// What the player's fighters met as it left systems since this was
    /// last taken, in order; taking it empties the list.
    pub(crate) fn take_fighter_notes(&mut self) -> Vec<FighterNote> {
        self.out.take_fighter_notes()
    }
}

impl Outbox {
    /// What the player's fighters met as it left systems since this was
    /// last taken, in order; taking it empties the list.
    fn take_fighter_notes(&mut self) -> Vec<FighterNote> {
        std::mem::take(&mut self.fighter_notes)
    }
}

impl PlayerShip {
    /// A fighter of ship type `ship`, walking for `base` too, goes back
    /// aboard, a round to the bay launching either among `pilot`'s outfits,
    /// sourced from `statics`' ammunition outfits
    /// ([`Armament::stow`](crate::Armament::stow)): whether a bay took it.
    /// A fighter that docks and one boarded back (see the `board_repair`
    /// module) walk for their sprite base, one recalled for its own type.
    pub(super) fn stow_fighter(
        &mut self,
        pilot: &mut Pilot,
        statics: &Statics,
        ship: ShipId,
        base: ShipId,
    ) -> bool {
        let mut rounds = OutfitRounds {
            owned: &mut pilot.outfits,
            sources: &statics.ammo_outfits,
        };
        self.fit.armament.stow(ship, base, &mut rounds)
    }

    /// For every outfit whose first mod is `ModType` 3
    /// ([`MOD_AMMO`](crate::combat::armament::MOD_AMMO)) naming a weapon of `MaxAmmo` above 0, the most of it the player may
    /// own: that `MaxAmmo` times the player's launchers of the weapon,
    /// stock weapons included, a wide product where the original's is
    /// 16-bit (`_HasMaxOfItem` @0x457e-0x45e4, `imulw` @0x45aa; see
    /// [`has_max::ammo_cap`]). An outfit with `ModType` 3 in a later
    /// slot is not capped.
    pub(super) fn ammo_caps(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
    ) -> BTreeMap<OutfitId, i64> {
        self.with_room(pilot, statics, setup, |room| {
            statics
                .outfits
                .iter()
                .filter_map(|record| Some((record.id, has_max::ammo_cap(record, room)?)))
                .collect()
        })
    }
}

impl Space {
    /// Tells the fight how many ship berths are free below `setup`'s spawn
    /// reserve for the fighters the player's next trigger launches, as
    /// `_FirePlayerWeapon` attempts a bay's copy only once `_LaunchFighter`
    /// finds a slot (@0x62c61-0x62c72; see the fight's `player_berths`
    /// module).
    pub(super) fn count_player_berths(&mut self, setup: &Setup) {
        let free = self.traffic.free_berths(setup.spawn_reserve(SPAWN_RESERVE));
        self.combat.set_player_berths(free);
    }

    /// The player launches a fighter of `EscortType` `escort_type` (none
    /// with no record): when it is 0 and no other ship of the player's is
    /// in the system, a kept Return to Hangar for the fighters' class is
    /// none (`_LaunchFighter` @0x3db0f-0x3db4d: the fighters' order,
    /// `0x3b5394[0]`, becomes -1 when it is 3 and no other active slot has
    /// the player as its parent). Whether it cleared it.
    fn clear_lone_return(&mut self, escort_type: Option<i16>) -> bool {
        let alone = self.player_led().next().is_none();
        let fighters = &mut self.escorts.class_orders[0];
        let clears = escort_type == Some(0) && alone && *fighters == Some(EscortOrder::Dock);
        if clears {
            *fighters = None;
        }
        clears
    }

    /// NPC `carrier`'s fighter `npc` joins it (see the module docs): led
    /// by the carrier, of its government, idle, its target the carrier's
    /// unless that is the carrier or a ship the carrier leads: its number,
    /// or none with its carrier gone.
    fn join_carrier(&mut self, mut npc: Npc, carrier: NpcId) -> Option<NpcId> {
        let launcher = self.npc(carrier).cloned()?;
        let parent = Some(ShipRef::Npc(carrier));
        let target = launcher.target.filter(|&ship| {
            Some(ship) != parent
                && !matches!(ship, ShipRef::Npc(id)
                    if self.npc(id).is_some_and(|led| led.leader_ref() == parent))
        });
        npc.take_govt_of(&launcher);
        npc.leader = Some(carrier);
        npc.aggression = launcher.aggression;
        npc.target = target;
        Some(self.traffic.add_npc(npc))
    }

    /// Puts the fighter of `sortie` back into its bay, a round again: one
    /// whose ship type has no record. A launch never finds the system
    /// full: the fight attempts the player's copy only into a free berth
    /// ([`Space::count_player_berths`]), and the traffic's room ledger
    /// gives an NPC's sortie its berth.
    fn put_back(&mut self, pilot: &mut Pilot, statics: &Statics, sortie: &Sortie) {
        match sortie.carrier {
            ShipRef::Player => OutfitRounds {
                owned: &mut pilot.outfits,
                sources: &statics.ammo_outfits,
            }
            .stow(sortie.bay),
            ShipRef::Npc(id) => {
                if let Some(carrier) = self.npc_mut(id) {
                    carrier.rounds.stow(sortie.bay);
                }
            }
        }
    }
}

/// A fighter of ship type `ship`, as `kind` gives it, at `state` and
/// `speed` along its heading, launched by `carrier`: its shield and armour
/// full at a fighter's capacities by `capacity`'s reading of
/// [`RuleKey::FighterCapacity`] (`_LaunchFighter` @0x3d9e7, @0x3d9f7,
/// [`Npc::take_carrier`]), carrying its class's stock weapons and
/// rounds, its AI type its `InherentAI`, idle.
fn fighter(
    kind: &ShipKind,
    ship: crate::catalog::ShipId,
    state: ShipState,
    speed: f32,
    carrier: Carrier,
    capacity: RuleSource,
) -> Npc {
    let mut fighter = Npc {
        id: NpcId::default(),
        berth: 0,
        ship,
        govt: None,
        ai_type: AiType::from_raw(kind.inherent_ai),
        leader: None,
        class: kind.escort_class,
        escort: None,
        stats: kind.stats,
        reserves: kind.stats.full(),
        state,
        speed,
        mode: Mode::Flying,
        goal: Goal::Idle,
        condition: Condition::Intact,
        hull: kind.hull,
        // `_LaunchFighter` folds it to its last set (@0x3d91f, @0x3d92b).
        armament: kind
            .armament
            .clone()
            .with_fold(Fold::default().at(kind.hull.frames.last_set())),
        rounds: kind.rounds.clone(),
        trigger: Trigger::default(),
        pick: None,
        target: None,
        provoked: 0.0,
        // `_LaunchFighter` zeroes +0xc910 (@0x3d952).
        stray: 0.0,
        aggression: 0,
        inspected: None,
        booty: None,
        boarded: false,
        info_types: 0,
        cargo: NpcHold::default(),
        spared: false,
        carrier: None,
        person: None,
        mission: None,
        aux: None,
        cloak: crate::cloak::Cloak::default(),
        cloak_gear: kind.cloak,
        led_out: false,
        emerges_from: None,
        visited: None,
        travel_set: false,
        self_repair: kind.self_repair,
        immobile: kind.immobile,
        skill: 1.0,
        skill_mult: 1.0,
        still: kind.still,
        miner: kind.miner,
        can_scoop: kind.can_scoop,
        holds: kind.holds,
        class_fuel: kind.class_fuel,
        former: None,
        tractor: None,
        ion: Ion::default(),
        afterburner: false,
        voice: None,
        pursuit: Pursuit::default(),
        ai_wait: 0,
        docking: None,
        lost_sight: crate::ai::lost_sight::SearchTimer::Unset,
        swarm_leader: None,
        owed_burst: None,
        close_form: false,
        fleet_slot: None,
        order_word: crate::fleet::OrderWord::NONE,
        marines: kind.marines,
        // `_LaunchFighter` @0x3d8fe.
        pods_left: kind.hull.signed_pod_count,
        warp: crate::traffic::warp_out::Warp::default(),
        // `_LaunchFighter` takes its slot from `_GenericRandomShipSpawn`
        // (@0x3d657): its glow 32 (@0x3cb77), and its anim rolls again
        // on its own class (@0x3da72-0x3dad9).
        display: NpcDisplay::spawned(
            &kind.hull.frames,
            Spawn {
                anim: AnimRoll::Own,
                ..Spawn::default()
            },
        )
        .0,
        // The spawn marks it stale (see `nova_sim_ai::traffic::odds_cache`).
        odds: crate::traffic::odds_cache::STALE,
        no_ai: false,
        kept_escort_ai: false,
        defends: None,
    };
    fighter.independent();
    fighter.take_carrier(carrier, capacity);
    let reserves = &mut fighter.reserves;
    reserves.shield.now = reserves.shield.max;
    reserves.armor.now = reserves.armor.max;
    fighter
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
pub(super) mod tests {
    use crate::combat::armament::LauncherRounds;
    use crate::testkit::{work, work_with};
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
    use crate::bay::{Carrier, FighterNote, dock_window, launch_velocity};
    use crate::catalog::{
        HullRecord, OutfitId, OutfitRecord, ShipId, ShipRecord, StockWeapon, WeaponId, WeaponRecord,
    };
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::{MOD_AMMO, MOD_WEAPON};
    use crate::combat::hull::Condition;
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::escort::{Commanded, EscortClass, EscortCommand, EscortGroup, EscortOrder};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::rulebook::Rulebook;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{
        FAST, FakePilotCatalog, Ranged, catalog, hull, land_now, outfit, ship, weapon,
    };
    use crate::traffic::npc::{AiType, Npc, NpcId};

    /// The Viper Bay: carrying ship 144, reloading every 60 ticks,
    /// pushing its fighter out at 4 pixels a tick, 4 fighters a bay.
    pub(in crate::session) const BAY: WeaponId = WeaponId(149);
    /// The fighter, a Viper.
    pub(in crate::session) const VIPER: ShipId = ShipId(144);
    /// The fighter outfit, the bay's rounds.
    pub(in crate::session) const VIPERS: OutfitId = OutfitId(158);
    /// The outfit holding the Viper Bay.
    const VIPER_BAY: OutfitId = OutfitId(157);
    /// The fighter's gun: 10 pixels a tick for 30 ticks, every 5, 5
    /// energy and 2 mass damage.
    pub(in crate::session) const GUN: WeaponId = WeaponId(141);
    /// A warship escort's ship type.
    pub(in crate::session) const WARSHIP: ShipId = ShipId(150);

    /// The Viper's fields: as fast as the player, of `Maneuver` 80 and a
    /// jump's fuel.
    pub(in crate::session) const VIPER_FIELDS: ShipFields = ShipFields {
        maneuver: 80,
        fuel: 300,
        ..FAST
    };

    pub(in crate::session) fn bay() -> WeaponRecord {
        WeaponRecord {
            guidance: 99,
            ammo_type: 144,
            flags: 0x0002,
            reload: 60,
            count: 30,
            speed: 400,
            max_ammo: 4,
            ..weapon(149)
        }
    }

    /// The test catalog ([`catalog`]) where the player's ship, 128, 40
    /// across, carries a Viper Bay (held by outfit 157); the Viper (a
    /// fighter, an interceptor by `InherentAI`, 20 across) carries a gun;
    /// and a warship escort type, 150.
    pub(in crate::session) fn carrying() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    escort_type: 0,
                    inherent_ai: 4,
                    ..ship(144, VIPER_FIELDS)
                },
                ShipRecord {
                    escort_type: 2,
                    inherent_ai: 3,
                    ..ship(150, FAST)
                },
            ],
            weapons: vec![
                bay(),
                WeaponRecord {
                    reload: 5,
                    speed: 1000,
                    count: 30,
                    energy_dmg: 5,
                    mass_dmg: 2,
                    ..weapon(141)
                },
            ],
            hulls: vec![
                HullRecord {
                    size: Some(40),
                    weapons: vec![StockWeapon {
                        weapon: BAY,
                        count: 1,
                        ammo: 0,
                    }],
                    ..hull(128)
                },
                HullRecord {
                    size: Some(20),
                    weapons: vec![StockWeapon {
                        weapon: GUN,
                        count: 1,
                        ammo: 0,
                    }],
                    ..hull(144)
                },
                HullRecord {
                    size: Some(50),
                    ..hull(150)
                },
            ],
            outfits: vec![
                outfit(157, &[(MOD_WEAPON, 149)]),
                OutfitRecord {
                    mass: 0,
                    max: 9999,
                    ..outfit(158, &[(MOD_AMMO, 149)])
                },
            ],
            ..catalog()
        }
    }

    /// A pilot owning `vipers` Vipers aboard, with `escorts`.
    pub(in crate::session) fn pilot(
        catalog: &FakePilotCatalog,
        vipers: u16,
        escorts: Vec<Escort>,
    ) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.outfits.insert(VIPERS, vipers);
        pilot.escorts = escorts;
        pilot
    }

    /// `pilot`'s session by `launch`, the bay selected, its first traffic
    /// tick done.
    fn flying(catalog: &FakePilotCatalog, pilot: Pilot, launch: RuleSource) -> Session {
        let rules = Rulebook::default().with_override(RuleKey::FighterLaunch, launch);
        flying_by(catalog, pilot, rules)
    }

    /// `pilot`'s session by `rules`, the bay selected, its system set up.
    pub(in crate::session) fn flying_by(
        catalog: &FakePilotCatalog,
        pilot: Pilot,
        rules: Rulebook,
    ) -> Session {
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(rules);
        session.select_secondary(false);
        session.populate(catalog, &mut NeverFires);
        session
    }

    /// Holds the secondary trigger, on the bay, for a pass, the traffic
    /// drawn from `catalog`.
    pub(in crate::session) fn launch(session: &mut Session, catalog: &FakePilotCatalog) {
        assert_eq!(session.secondary(), Some(BAY));
        session.hold_fire(false, true);
        work(session, catalog, Controls::default());
        session.hold_fire(false, false);
    }

    /// The fleet's NPCs, in fleet order.
    pub(in crate::session) fn fleet_npcs(session: &Session) -> Vec<Npc> {
        session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .filter_map(|&id| session.npcs().iter().find(|npc| npc.id == id))
            .cloned()
            .collect()
    }

    /// Thinks nothing and does nothing: a ship that stays put.
    #[derive(Debug)]
    struct Sitting;

    impl Behaviour for Sitting {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// Nova's AI, but every interceptor and wimpy trader sits as it is:
    /// the pirates and the traders here.
    pub(in crate::session) fn sitting_ai() -> NovaAi {
        NovaAi::default()
            .with(AiType::Interceptor, Rc::new(Sitting))
            .with(AiType::WimpyTrader, Rc::new(Sitting))
    }

    /// Adds a pirate interceptor, a medium ship, at (`x`, `y`), idle.
    pub(in crate::session) fn pirate(session: &mut Session, x: f32, y: f32) -> NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(152);
        npc.ai_type = AiType::Interceptor;
        npc.class = EscortClass::Medium;
        npc.state.position = Vec2::new(x, y);
        session.space.traffic.add_npc(npc)
    }

    /// A tick of the player at rest, then of the traffic by `ai`, then of
    /// the fight.
    pub(in crate::session) fn fight(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        ai: &NovaAi,
    ) {
        work_with(
            session,
            catalog,
            Controls::default(),
            ai,
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// Whether `ship`'s shield drops within `ticks` of [`fight`].
    pub(in crate::session) fn shield_drops(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        ai: &NovaAi,
        ship: NpcId,
        ticks: u32,
    ) -> bool {
        (0..ticks).any(|_| {
            fight(session, catalog, ai);
            session
                .npcs()
                .iter()
                .find(|npc| npc.id == ship)
                .is_none_or(|npc| npc.reserves.shield.now < 30.0)
        })
    }

    // Launching.

    #[test]
    fn a_fighter_carries_its_ship_types_cloak_gear() {
        let gear = crate::cloak::CloakGear {
            device: true,
            flags2: 0x5282,
            ..crate::cloak::CloakGear::default()
        };
        let kind = ShipKind {
            cloak: gear,
            ..ShipKind::default()
        };
        let carrier = Carrier {
            ship: ShipRef::Player,
            window: 0.0,
            reach: 0.0,
        };
        let npc = fighter(
            &kind,
            ShipId(204),
            ShipState::default(),
            0.0,
            carrier,
            RuleSource::Engine,
        );
        assert_eq!(npc.cloak_gear, gear);
    }

    #[test]
    fn a_fighter_of_a_class_of_no_speed_and_no_accel_launches_immobile_and_stays_put() {
        let launched = |fields| {
            let mut catalog = carrying();
            let viper = catalog
                .ship_records
                .iter_mut()
                .find(|record| record.id == VIPER)
                .expect("the Viper's");
            viper.fields = fields;
            let mut session = flying(&catalog, pilot(&catalog, 1, Vec::new()), RuleSource::Engine);
            launch(&mut session, &catalog);
            let fighter = fleet_npcs(&session)[0].clone();
            let npc = session.space.npc_mut(fighter.id).expect("there");
            // Held, so the push alone could move it.
            npc.goal = Goal::Hold;
            for _ in 0..3 {
                work_with(
                    &mut session,
                    &catalog,
                    Controls::default(),
                    &Sitting,
                    Rules::default(),
                    &mut NeverFires,
                );
            }
            let state = session.space.npc(fighter.id).expect("there").state;
            (fighter.immobile, fighter.state, state)
        };
        // Its tick comes in its launch frame, as the ship loop
        // (@0x4445c-0x444c4) comes after `_HandlePlayer`'s launch
        // (@0x62c61): the bay's push lasts only until then.
        let (immobile, at, state) = launched(ShipFields {
            speed: 0,
            accel: 0,
            ..VIPER_FIELDS
        });
        assert!(immobile, "its class's");
        assert_eq!(
            at.velocity,
            Vec2::ZERO,
            "the push lasts only until its tick"
        );
        assert_eq!(
            state.position, at.position,
            "the bay's push moves it not at all"
        );
        assert_eq!(state.velocity, Vec2::ZERO);
        let (immobile, at, state) = launched(VIPER_FIELDS);
        assert!(!immobile);
        assert_ne!(at.velocity, Vec2::ZERO, "pushed out by the bay");
        assert_ne!(state.position, at.position, "the bay's push moves it");
    }

    #[test]
    fn a_fighter_the_player_launches_moves_on_its_launch_tick() {
        // `_FirePlayerWeapon`'s bay calls `_LaunchFighter` (@0x62c61),
        // which marks the slot active before `_AIDispatch` and
        // `_HandleShip` (@0x4429d, @0x44483) run that frame.
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        let stood = session.player().position;
        session.hold_fire(false, true);
        session.game_work(
            Controls::default(),
            &catalog,
            &Sitting,
            Rules::default(),
            &mut NeverFires,
        );
        let fighters = fleet_npcs(&session);
        assert_eq!(fighters.len(), 1, "launched");
        assert_ne!(fighters[0].state.position, stood, "moved off the bay");
    }

    /// The fighter the player's bay launches in `_HandlePlayer`, by
    /// `undisplayed`: whether the collision passes walk it and the frame
    /// draws it after the player's trigger, then after the NPCs' display
    /// steps.
    fn launched_fighter_shown(undisplayed: RuleSource) -> [(bool, bool); 2] {
        let catalog = carrying();
        let rules = Rulebook::default()
            .with_override(RuleKey::FighterLaunch, RuleSource::Engine)
            .with_override(RuleKey::UndisplayedShips, undisplayed);
        let mut session = flying_by(&catalog, pilot(&catalog, 2, Vec::new()), rules);
        session.select_secondary(false);
        session.hold_fire(false, true);
        let pass = crate::combat::GamePass::Advance { double_time: false };
        session.player_fires(pass, Rules::default(), &mut NeverFires);
        let fighter = ShipRef::Npc(fleet_npcs(&session)[0].id);
        let shown = |session: &Session| {
            let walked = session
                .sprite_ranks()
                .iter()
                .any(|&(ship, _)| ship == fighter);
            (walked, session.draw_order().contains(&fighter))
        };
        let launched = shown(&session);
        // The ship loop's displays (@0x444ad).
        let start = super::super::game_work::PassStart::of(&session);
        session.ship_loop(
            &crate::ai::NovaAi::default(),
            &start,
            super::super::ship_loop::Motion::Again,
            pass,
            Rules::default(),
            &mut NeverFires,
        );
        [launched, shown(&session)]
    }

    #[test]
    fn a_fighter_the_player_launches_is_not_walked_until_its_display_step() {
        // `_LaunchFighter` from `_HandlePlayer` (@0x62c61) makes active a
        // slot whose last display hid it (@0x2b555-0x2b60a): the collision
        // passes skip it (@0xacba3) until its own display (@0x444ad).
        assert_eq!(
            launched_fighter_shown(RuleSource::Engine),
            [(false, false), (true, true)]
        );
    }

    #[test]
    fn by_the_other_reading_a_fighter_the_player_launches_is_walked_at_once() {
        assert_eq!(
            launched_fighter_shown(RuleSource::Bible),
            [(true, true), (true, true)]
        );
    }

    #[test]
    fn firing_the_bay_launches_a_fighter_into_the_fleet_beside_the_player() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        session.take_save_due();
        session.take_combat_events();
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().owned(VIPERS), 1, "a fighter launched");
        assert_eq!(session.secondary_line_count(), Some(1), "the bay's own");
        let fighters = fleet_npcs(&session);
        assert_eq!(fighters.len(), 1);
        let fighter = &fighters[0];
        assert_eq!(fighter.ship, VIPER);
        let record = session.pilot().escorts()[0];
        assert!(record.carried);
        assert_eq!(record.ship, VIPER);
        assert_eq!(record.order, None);
        // Full at a fighter's capacities, 1.333 × its class's
        // (`RuleKey::FighterCapacity`; see the `fighter_capacity` tests).
        let mut full = ShipStats::new(VIPER_FIELDS, &[]).full();
        full.shield = crate::reserves::Gauge::full(39.99);
        full.armor = crate::reserves::Gauge::full(59.985);
        assert_eq!(record.reserves, full);
        // Launched at the player, it moves in its launch frame's ship loop
        // (@0x4445c-0x444c4), after `_HandlePlayer`'s launch (@0x62c61).
        assert_eq!(
            fighter.state.position,
            session.player().position + fighter.state.velocity
        );
        assert_eq!(fighter.state.heading, 0.0, "the player's heading");
        assert_eq!(
            fighter.state.velocity,
            launch_velocity(Vec2::ZERO, 0.0, 4.0, 6.0),
            "pushed out at the bay's speed"
        );
        assert_eq!(fighter.govt, None);
        assert_eq!(
            fighter.carrier,
            Some(Carrier {
                ship: ShipRef::Player,
                window: dock_window(80),
                reach: 40.0,
            })
        );
        assert_eq!(fighter.reserves, full);
        assert_eq!(fighter.condition, Condition::Intact);
        assert_eq!(fighter.ai_type, AiType::Interceptor, "its InherentAI");
        assert_eq!(fighter.class, EscortClass::Fighter);
        assert_eq!(
            fighter
                .armament
                .mounts()
                .iter()
                .map(|mount| mount.spec.id)
                .collect::<Vec<_>>(),
            [GUN],
            "its class's stock weapons"
        );
        assert_eq!(
            fighter
                .escort
                .map(|duty| (duty.formation.slot, duty.formation.ships)),
            Some((2, 2))
        );
        assert!(session.is_escort(fighter.id));
        assert!(
            session.take_combat_events().contains(&CombatEvent::Fired {
                ship: ShipRef::Player,
                weapon: BAY,
                at: session.player().position,
                // `_FirePlayerWeapon`'s looped test (@0x62d98-0x62dbf)
                // gates its bay branch too (@0x62e40-0x62e44).
                sound: Some(crate::combat::FireSound {
                    priority: crate::combat::PLAYER_SECONDARY_FIRE_PRIORITY,
                    unless_playing: true,
                }),
            }),
            "the bay's sound is heard"
        );
        assert!(!session.take_save_due(), "as firing ammunition");
        assert_eq!(session.traffic_ships(), [VIPER]);
    }

    #[test]
    fn a_fighter_launched_rolls_its_afterburner_by_its_classs_flags() {
        // `_LaunchFighter` asks `_AIHasAfterburner` (@0x3d736): a Viper of
        // `Flags` 0x0040 always has one, of neither bit none.
        for (flags, has) in [(0x0040, true), (0x0000, false)] {
            let mut catalog = carrying();
            for record in &mut catalog.hulls {
                if record.id == VIPER {
                    record.flags = flags;
                }
            }
            let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
            launch(&mut session, &catalog);
            assert_eq!(
                fleet_npcs(&session).first().map(|npc| npc.afterburner),
                Some(has),
                "{flags:#x}"
            );
        }
    }

    #[test]
    fn a_fighter_launched_at_a_berth_a_swarmer_still_names_has_no_afterburner() {
        // `_AIHasAfterburner`'s first test (@0x64ff-0x6539): the ship at
        // berth 2 names berth 1, free, which the fighter takes.
        let mut catalog = carrying();
        for record in &mut catalog.hulls {
            if record.id == VIPER {
                record.flags = 0x0040;
            }
        }
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        let mut swarmer = crate::traffic::fixture::npc(0, crate::stats::ShipStats::default());
        swarmer.swarm_leader = Some(1);
        session.space.traffic.add_npc_at(swarmer, 2);
        launch(&mut session, &catalog);
        let fighter = fleet_npcs(&session)
            .first()
            .cloned()
            .expect("a fighter launched");
        assert_eq!(fighter.berth, 1);
        assert!(!fighter.afterburner);
    }

    #[test]
    fn by_the_engine_a_fighter_takes_no_command_and_attacks_once_commanded() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        let quarry = pirate(&mut session, 0.0, -300.0);
        session.select_target(TargetPick::Nearest);
        assert_eq!(session.chart.target, Some(quarry));
        launch(&mut session, &catalog);
        let fighter = fleet_npcs(&session)[0].clone();
        assert_eq!(
            session.pilot().escorts()[0].order,
            None,
            "no standing command"
        );
        assert_eq!(fighter.target, None, "no target");
        let ai = sitting_ai();
        fight(&mut session, &catalog, &ai);
        assert!(matches!(
            fleet_npcs(&session)[0].goal,
            Goal::Formation { .. }
        ));
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires)
                .is_some()
        );
        assert_eq!(fleet_npcs(&session)[0].target, Some(ShipRef::Npc(quarry)));
        fight(&mut session, &catalog, &ai);
        assert_eq!(
            fleet_npcs(&session)[0].goal,
            Goal::Attack(ShipRef::Npc(quarry))
        );
        assert!(shield_drops(&mut session, &catalog, &ai, quarry, 300));
    }

    /// The order a fighter of class `escort_type` takes on launch, with a
    /// warship escort ahead of it in the fleet holding `order`, by the
    /// engine.
    fn copied(escort_type: i16, order: Option<EscortOrder>) -> Option<EscortOrder> {
        let mut catalog = carrying();
        catalog.ship_records[1].escort_type = escort_type;
        let warship = Escort {
            ship: WARSHIP,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        };
        let mut session = Session::fly(&catalog, pilot(&catalog, 2, vec![warship]))
            .expect("flies")
            .with_rules(
                Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible),
            );
        session.select_secondary(false);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().escorts().len(), 2);
        let fighter = fleet_npcs(&session)[1].clone();
        let record = session.pilot().escorts()[1];
        assert_eq!(fighter.escort.and_then(|duty| duty.order), record.order);
        record.order
    }

    /// A session of [`carrying`] with two Vipers aboard and no escort, by
    /// the engine, the bay selected.
    fn alone(catalog: &FakePilotCatalog) -> Session {
        flying(catalog, pilot(catalog, 2, Vec::new()), RuleSource::Engine)
    }

    /// The order the one fighter launched from `session` takes, on its
    /// record and its duty, which agree.
    fn launched_order(session: &mut Session, catalog: &FakePilotCatalog) -> Option<EscortOrder> {
        launch(session, catalog);
        assert_eq!(session.pilot().escorts().len(), 1, "one launched");
        let fighter = fleet_npcs(session)[0].clone();
        let record = session.pilot().escorts()[0];
        assert_eq!(fighter.escort.and_then(|duty| duty.order), record.order);
        record.order
    }

    #[test]
    fn a_fighter_launched_takes_its_classs_mission_escorts_order() {
        let catalog = carrying();
        let mut session = alone(&catalog);
        let record = &catalog.ship_records[1];
        let class = EscortClass::of(record.escort_type, record.inherent_ai, record.fields.mass);
        let mut escort = crate::testkit::npc(0, crate::stats::ShipStats::default());
        escort.class = class;
        escort.mission = Some(crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::mission::ships::ShipBehav::EscortPlayer,
            formation: Some(FormationSlot::default()),
            order: Some(EscortOrder::Hold),
        });
        session.space.traffic.add_npc(escort);
        assert_eq!(
            launched_order(&mut session, &catalog),
            Some(EscortOrder::Hold)
        );
    }

    #[test]
    fn a_fighter_launched_takes_the_order_kept_for_its_class() {
        let catalog = carrying();
        let mut session = alone(&catalog);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires),
            None,
            "no escort to change"
        );
        let id = launch_now(&mut session).expect("launched");
        let duty = session.space.npc(id).and_then(|npc| npc.escort);
        assert_eq!(duty.and_then(|duty| duty.order), Some(EscortOrder::Hold));
        assert_eq!(session.pilot().escorts()[0].order, Some(EscortOrder::Hold));
        // No ship of its class took the command, so the pass's class-order
        // bookkeeping sends the class to formation (see the `class_orders`
        // module).
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts()[0].order, None);
    }

    /// One Viper launched from the player's bay at once, outside a pass
    /// ([`Session::launch_fighter`]): its number.
    fn launch_now(session: &mut Session) -> Option<NpcId> {
        let sortie = Sortie {
            carrier: ShipRef::Player,
            bay: BAY,
            ship: VIPER,
            from: *session.player(),
            heading: 0.0,
            target: None,
        };
        session.launch_fighter(&sortie, &mut NeverFires)
    }

    #[test]
    fn a_fighter_launched_after_return_to_hangar_takes_no_order() {
        let catalog = carrying();
        let mut session = alone(&catalog);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert_eq!(launched_order(&mut session, &catalog), None);
    }

    /// The fighters' kept class order after a launch from `session`, with
    /// Return to Hangar given every class first: the launch's own, made
    /// outside a pass, as the pass's class-order bookkeeping that follows
    /// it clears a Return no marked fighter holds anyway (see the
    /// `class_orders` module).
    fn kept_after_launch(session: &mut Session) -> Option<EscortOrder> {
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert_eq!(
            session.space.escorts.class_orders[0],
            Some(EscortOrder::Dock)
        );
        launch_now(session).expect("launched");
        session.space.escorts.class_orders[0]
    }

    #[test]
    fn a_fighter_launched_with_no_other_ship_of_the_players_out_clears_a_kept_return() {
        // `_LaunchFighter` @0x3db0f-0x3db4d: the player's carrier, a
        // fighter of `EscortType` 0, no other active ship of parent 0, and
        // the fighters' order (0x3b5394[0]) Return becomes -1.
        let catalog = carrying();
        assert_eq!(kept_after_launch(&mut alone(&catalog)), None, "alone");
        let mut escorted = fleet(&catalog, 2, vec![warship(None)]);
        assert_eq!(
            kept_after_launch(&mut escorted),
            Some(EscortOrder::Dock),
            "an escort out"
        );
        let mut session = alone(&catalog);
        launch(&mut session, &catalog);
        for _ in 0..60 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(
            kept_after_launch(&mut session),
            Some(EscortOrder::Dock),
            "a fighter out"
        );
        let mut catalog = carrying();
        catalog.ship_records[1].escort_type = 2;
        let mut session = alone(&catalog);
        assert_eq!(
            kept_after_launch(&mut session),
            Some(EscortOrder::Dock),
            "not of EscortType 0"
        );
        assert_eq!(
            session.space.escorts.class_orders[2],
            Some(EscortOrder::Dock),
            "its own class's kept too"
        );
    }

    #[test]
    fn a_lone_fighter_launched_under_a_kept_return_reports_the_clear_on_its_pass() {
        // `_LaunchFighter`'s -1 (@0x3db4d) is cleared to 0 later in the same
        // `_HandlePlayer` (@0x6e42d-0x6e446), which shows the escort menu
        // again while it shows.
        let catalog = carrying();
        let mut session = alone(&catalog);
        session.command_escorts(
            EscortGroup::Class(EscortClass::Fighter),
            EscortCommand::Dock,
            &mut NeverFires,
        );
        launch(&mut session, &catalog);
        assert_eq!(session.space.escorts.class_orders, [None; 4]);
        assert!(session.out.class_orders_cleared, "reported");
        let mut session = alone(&catalog);
        launch(&mut session, &catalog);
        assert!(!session.out.class_orders_cleared, "none kept: none cleared");
    }

    #[test]
    fn by_the_engine_a_fighter_copies_its_classs_standing_order_but_return_to_hangar() {
        assert_eq!(
            copied(2, Some(EscortOrder::Attack)),
            Some(EscortOrder::Attack),
            "a warship's"
        );
        assert_eq!(copied(2, Some(EscortOrder::Hold)), Some(EscortOrder::Hold));
        assert_eq!(copied(0, Some(EscortOrder::Attack)), None, "a fighter's");
        assert_eq!(copied(2, Some(EscortOrder::Dock)), None, "never Dock");
    }

    #[test]
    fn by_the_other_reading_a_fighter_attacks_the_players_target_at_once() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Bible);
        assert_eq!(
            session.rules().source_for(RuleKey::FighterLaunch),
            RuleSource::Bible
        );
        let quarry = pirate(&mut session, 0.0, -300.0);
        session.select_target(TargetPick::Nearest);
        launch(&mut session, &catalog);
        let fighter = fleet_npcs(&session)[0].clone();
        assert_eq!(fighter.target, Some(ShipRef::Npc(quarry)));
        assert_eq!(
            session.pilot().escorts()[0].order,
            Some(EscortOrder::Attack)
        );
        assert_eq!(
            fighter.escort.and_then(|duty| duty.order),
            Some(EscortOrder::Attack)
        );
        let ai = sitting_ai();
        assert!(shield_drops(&mut session, &catalog, &ai, quarry, 300));
    }

    #[test]
    fn by_the_other_reading_with_no_target_or_an_escort_targeted_it_launches_as_the_engines() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Bible);
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().escorts()[0].order, None, "no target");
        assert_eq!(fleet_npcs(&session)[0].target, None);
        let first = session.space.escorts.fleet[0];
        session.chart.target = first;
        for _ in 0..60 {
            work(&mut session, &catalog, Controls::default());
        }
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().escorts().len(), 2);
        assert_eq!(
            session.pilot().escorts()[1].order,
            None,
            "its own escort targeted"
        );
        assert_eq!(fleet_npcs(&session)[1].target, None);
    }

    #[test]
    fn a_fighter_finds_no_slot_once_fifty_five_ships_are_in_the_system() {
        let catalog = carrying();
        let crowded = |ships: u32, reserve: RuleSource| {
            let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
            session.setup.rules = Rulebook::default().with_override(RuleKey::SpawnReserve, reserve);
            for n in 0..ships {
                pirate(&mut session, 1000.0 + n as f32, 1000.0);
            }
            launch(&mut session, &catalog);
            session
        };
        // `_FirePlayerWeapon` attempts the copy only once `_LaunchFighter`
        // finds a slot (@0x62c61-0x62c72): with none, nothing is spent,
        // counted, heard or reloaded (@0x62d82-0x62d87).
        let mut session = crowded(55, RuleSource::Engine);
        assert_eq!(session.pilot().owned(VIPERS), 2, "no round leaves");
        assert_eq!(session.npcs().len(), 55, "no fighter");
        assert_eq!(session.pilot().escorts(), []);
        let heard = session
            .take_combat_events()
            .into_iter()
            .any(|event| matches!(event, CombatEvent::Fired { weapon, .. } if weapon == BAY));
        assert!(!heard, "no bay heard");
        let reload = session.ship.fit.armament.mounts()[0].reload;
        assert_eq!(reload, 0.0, "no reload");
        let session = crowded(54, RuleSource::Engine);
        assert_eq!(session.npcs().len(), 55, "the last slot below 56 taken");
        let session = crowded(55, RuleSource::Bible);
        assert_eq!(session.npcs().len(), 56, "by the Bible, any slot");
        let session = crowded(63, RuleSource::Bible);
        assert_eq!(session.npcs().len(), 63, "by the Bible, the table full");
    }

    #[test]
    fn a_fighter_the_game_work_pass_launches_finds_no_slot_once_fifty_five_ships_are_in_the_system()
    {
        // The game's pass fires the player's trigger in its own step
        // (`Session::player_fires`), which counts the free berths again.
        let catalog = carrying();
        let crowded = |ships: u32| {
            let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
            for n in 0..ships {
                pirate(&mut session, 1000.0 + n as f32, 1000.0);
            }
            assert_eq!(session.secondary(), Some(BAY));
            session.hold_fire(false, true);
            session.game_work(
                Controls::default(),
                &catalog,
                &Sitting,
                Rules::default(),
                &mut NeverFires,
            );
            session
        };
        let session = crowded(54);
        assert_eq!(session.pilot().owned(VIPERS), 1, "the last berth taken");
        assert_eq!(session.npcs().len(), 55);
        let mut session = crowded(55);
        assert_eq!(session.pilot().owned(VIPERS), 2, "no round leaves");
        assert_eq!(session.npcs().len(), 55, "no fighter");
        assert_eq!(session.pilot().escorts(), []);
        let heard = session
            .take_combat_events()
            .into_iter()
            .any(|event| matches!(event, CombatEvent::Fired { weapon, .. } if weapon == BAY));
        assert!(!heard, "no bay heard");
        let reload = session.ship.fit.armament.mounts()[0].reload;
        assert_eq!(reload, 0.0, "no reload");
    }

    #[test]
    fn a_volley_of_two_bays_with_one_berth_left_launches_one_fighter_spending_one_round() {
        // Two bays firing together (`Flags` 0x0040): the first copy finds
        // the last berth below the reserve, the second none, and is
        // neither spent nor counted (@0x62c66-0x62c72); the volley reloads
        // as one that spawned one copy (@0x62f42-0x62f96).
        let mut catalog = carrying();
        catalog.weapons[0].flags |= crate::combat::weapon::SIMULTANEOUS;
        catalog.hulls[0].weapons[0].count = 2;
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Engine);
        for n in 0..54 {
            pirate(&mut session, 1000.0 + n as f32, 1000.0);
        }
        session.take_combat_events();
        launch(&mut session, &catalog);
        assert_eq!(session.npcs().len(), 55, "one fighter");
        assert_eq!(session.pilot().escorts().len(), 1);
        assert_eq!(session.pilot().owned(VIPERS), 2, "one round spent");
        let heard = session
            .take_combat_events()
            .into_iter()
            .filter(|event| matches!(event, CombatEvent::Fired { weapon, .. } if *weapon == BAY))
            .count();
        assert_eq!(heard, 1, "one copy fired");
        let mount = session.ship.fit.armament.mounts()[0];
        assert_eq!(mount.count, 2);
        assert_eq!(
            mount.reload,
            crate::combat::ai_fire::reload(&mount.spec, 1, 2) - 1.0,
            "reloading as one copy, a tick counted down"
        );
    }

    /// [`carrying`] with the Viper Bay a glow weapon (`Flags2` 0x0200).
    fn carrying_a_glow_bay() -> FakePilotCatalog {
        let mut catalog = carrying();
        catalog.weapons[0].flags2 |= crate::combat::weapon::GLOWS;
        catalog
    }

    /// The weapon glows among `events`: each ship's, and whether late.
    fn glows(events: &[CombatEvent]) -> Vec<(ShipRef, bool)> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::WeaponGlow { ship, late } => Some((ship, late)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_players_glow_bay_glows_only_for_a_fighter_put_in_the_system() {
        let catalog = carrying_a_glow_bay();
        let launched = |ships: u32, reserve: RuleSource| {
            let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
            session.setup.rules = Rulebook::default().with_override(RuleKey::SpawnReserve, reserve);
            for n in 0..ships {
                pirate(&mut session, 1000.0 + n as f32, 1000.0);
            }
            launch(&mut session, &catalog);
            let glows = glows(&session.take_combat_events());
            (glows, session.npcs().len(), session.pilot().owned(VIPERS))
        };
        assert_eq!(
            launched(55, RuleSource::Engine),
            (Vec::new(), 55, 2),
            "no slot: the round put back and no glow"
        );
        assert_eq!(
            launched(54, RuleSource::Engine),
            (vec![(ShipRef::Player, false)], 55, 1),
            "the last slot: the frame it launched"
        );
        assert_eq!(
            launched(55, RuleSource::Bible),
            (vec![(ShipRef::Player, false)], 56, 1),
            "by the other reading, any slot"
        );
    }

    #[test]
    fn two_fighters_launched_in_one_call_glow_once() {
        let mut catalog = carrying_a_glow_bay();
        catalog.weapons[0].flags |= crate::combat::weapon::SIMULTANEOUS;
        catalog.hulls[0].weapons[0].count = 2;
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        launch(&mut session, &catalog);
        assert_eq!(vipers(&session).len(), 2, "two at once");
        assert_eq!(
            glows(&session.take_combat_events()),
            [(ShipRef::Player, false)]
        );
    }

    #[test]
    fn a_bay_without_the_flag_glows_never() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        launch(&mut session, &catalog);
        assert_eq!(vipers(&session).len(), 1, "launched");
        assert_eq!(glows(&session.take_combat_events()), []);
    }

    #[test]
    fn a_ship_left_behind_takes_a_slot_from_the_fighters() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        for n in 0..55 {
            pirate(&mut session, 1000.0 + n as f32, 1000.0);
        }
        let last = session.npcs().last().expect("55 pirates").id;
        let keep: std::collections::BTreeSet<NpcId> = session
            .npcs()
            .iter()
            .map(|npc| npc.id)
            .filter(|&id| id != last)
            .collect();
        let table = session.space.traffic.table().clone();
        let system = session.system();
        session.space.traffic.retarget(
            table,
            &keep,
            crate::traffic::Leave::Held {
                from: crate::catalog::SystemId(system.0 + 1),
                to: system,
            },
        );
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().owned(VIPERS), 2, "the round put back");
        assert_eq!(session.npcs().len(), 54, "no fighter");
    }

    // Room for a fighter boarded.

    #[test]
    fn can_take_fighter_is_true_while_the_bay_carrying_the_type_has_room() {
        // `_CanBuyFighter(type, -1, -1)` (@0x5a82-0x5cb2): the bay holds 4.
        let catalog = carrying();
        let room = |vipers: u16, escorts: Vec<Escort>| {
            fleet(&catalog, vipers, escorts).can_take_fighter(VIPER)
        };
        assert!(room(3, Vec::new()), "3 of 4 aboard");
        assert!(!room(4, Vec::new()), "full");
        assert!(!room(3, vec![out()]), "3 aboard and 1 out");
        assert!(room(2, vec![out()]));
        assert!(
            !fleet(&catalog, 0, Vec::new()).can_take_fighter(WARSHIP),
            "no bay carries it"
        );
        let mut bare = carrying();
        bare.hulls[0].weapons.clear();
        assert!(
            !fleet(&bare, 0, Vec::new()).can_take_fighter(VIPER),
            "no bay"
        );
    }

    /// A variant of the Viper, drawing its sprites.
    pub(in crate::session) const VARIANT: ShipId = ShipId(223);
    /// Another variant of the Viper.
    pub(in crate::session) const SIBLING: ShipId = ShipId(224);

    /// [`carrying`] with the two variants of the Viper.
    pub(in crate::session) fn variants() -> FakePilotCatalog {
        let mut catalog = carrying();
        for variant in [VARIANT, SIBLING] {
            catalog.ship_records.push(ShipRecord {
                escort_type: 0,
                inherent_ai: 4,
                ..ship(variant.0, VIPER_FIELDS)
            });
            catalog.hulls.push(HullRecord {
                size: Some(20),
                sprite_base: VIPER,
                ..hull(variant.0)
            });
        }
        catalog
    }

    /// [`variants`] with the player's one bay carrying [`VARIANT`], not
    /// the Viper.
    pub(in crate::session) fn variant_bay() -> FakePilotCatalog {
        let mut catalog = variants();
        catalog.weapons[0].ammo_type = VARIANT.0;
        catalog
    }

    #[test]
    fn a_variant_boarded_fits_a_bay_of_its_sprite_base() {
        // `_CanBuyFighter`'s second walk (@0x5b0d-0x5b75), and the held
        // count reads the type asked, not the bay's (@0x5c67-0x5c9b).
        let catalog = variants();
        let room = |vipers: u16, escorts: Vec<Escort>| {
            fleet(&catalog, vipers, escorts).can_take_fighter(VARIANT)
        };
        assert!(room(3, Vec::new()), "3 of 4 aboard");
        assert!(!room(4, Vec::new()), "full");
        let variant_out = Escort {
            ship: VARIANT,
            ..out()
        };
        assert!(!room(3, vec![variant_out]), "one of the type asked is out");
        assert!(room(3, vec![out()]), "a Viper out isn't of the type asked");
    }

    #[test]
    fn by_the_other_recall_a_variant_finds_no_bay_of_its_sprite_base() {
        // `_InstantFighterRecall` walks the exact type alone
        // (@0x5e45-0x5e7e).
        let catalog = variants();
        let rules = Rulebook::default().with_override(RuleKey::FighterRecall, RuleSource::Bible);
        let variant = Escort {
            ship: VARIANT,
            ..out()
        };
        let mut session = flying_by(&catalog, pilot(&catalog, 0, vec![variant]), rules);
        session.take_save_due();
        session.leave_with_fighters(false);
        assert_eq!(session.pilot().escorts(), []);
        assert_eq!(session.pilot().owned(VIPERS), 0, "no round");
        assert!(session.take_save_due(), "lost");
    }

    #[test]
    fn a_sibling_variant_fits_a_bay_of_another_variant_of_its_base() {
        // Bases compared, wider than the dock (@0x5b0d-0x5b75).
        let catalog = variant_bay();
        assert!(fleet(&catalog, 0, Vec::new()).can_take_fighter(SIBLING));
        assert!(fleet(&catalog, 0, Vec::new()).can_take_fighter(VIPER));
        assert!(!fleet(&catalog, 4, Vec::new()).can_take_fighter(SIBLING));
    }

    /// [`carrying`] with a second, higher bay carrying the Viper, 151,
    /// of 2 rounds, its fighters outfit 159, the player's ship holding
    /// both.
    fn second_viper_bay(catalog: &mut FakePilotCatalog) {
        catalog.weapons.push(WeaponRecord {
            max_ammo: 2,
            ..WeaponRecord {
                id: WeaponId(151),
                ..bay()
            }
        });
        catalog.hulls[0].weapons.push(StockWeapon {
            weapon: WeaponId(151),
            count: 1,
            ammo: 0,
        });
        catalog.outfits.push(outfit(160, &[(MOD_WEAPON, 151)]));
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            max: 9999,
            ..outfit(159, &[(MOD_AMMO, 151)])
        });
    }

    #[test]
    fn the_room_for_a_fighter_boarded_reads_the_first_bay_not_any() {
        // The first walk breaks at the first bay held (@0x5b7e): a higher
        // bay's room doesn't count.
        let mut catalog = carrying();
        second_viper_bay(&mut catalog);
        assert!(
            !fleet(&catalog, 4, Vec::new()).can_take_fighter(VIPER),
            "149 is full, and 151's room isn't read"
        );
        assert!(fleet(&catalog, 3, Vec::new()).can_take_fighter(VIPER));
    }

    #[test]
    fn a_bay_with_no_fighter_outfit_has_no_room_for_a_fighter_boarded() {
        // @0x5bda.
        let mut catalog = carrying();
        catalog.outfits.pop();
        assert!(!fleet(&catalog, 0, Vec::new()).can_take_fighter(VIPER));
    }

    #[test]
    fn a_bay_of_no_max_ammo_reads_the_last_fighter_outfits_max() {
        // The last by ID (@0x5b95-0x5bd8), @0x5c1f-0x5c4f.
        let mut catalog = carrying();
        catalog.weapons[0].max_ammo = 0;
        catalog.outfits[1].max = 1;
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            max: 3,
            ..outfit(159, &[(MOD_AMMO, 149)])
        });
        assert!(fleet(&catalog, 2, Vec::new()).can_take_fighter(VIPER));
        assert!(!fleet(&catalog, 3, Vec::new()).can_take_fighter(VIPER));
    }

    // Return to Hangar and docking.

    /// A Viper out of its bay, full, with no standing command.
    pub(in crate::session) fn out() -> Escort {
        Escort {
            ship: VIPER,
            reserves: ShipStats::new(VIPER_FIELDS, &[]).full(),
            order: None,
            carried: true,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// A warship escort following `order`.
    pub(in crate::session) fn warship(order: Option<EscortOrder>) -> Escort {
        Escort {
            ship: WARSHIP,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order,
            carried: false,
            wage: None,
            person: None,
            mark: None,
        }
    }

    /// The fleet's standing orders.
    fn orders(session: &Session) -> Vec<Option<EscortOrder>> {
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect()
    }

    /// The rules [`fleet`] flies by: the escorts' orders kept as they
    /// enter.
    fn kept_orders() -> Rulebook {
        Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible)
    }

    /// `escorts` flying with `vipers` aboard, the bay selected, the
    /// orders kept as they enter.
    pub(in crate::session) fn fleet(
        catalog: &FakePilotCatalog,
        vipers: u16,
        escorts: Vec<Escort>,
    ) -> Session {
        let session = Session::fly(catalog, pilot(catalog, vipers, escorts))
            .expect("flies")
            .with_rules(kept_orders());
        let mut session = session;
        session.select_secondary(false);
        session.populate(catalog, &mut NeverFires);
        session
    }

    #[test]
    fn the_target_panel_labels_a_launched_fighter_a_fighter_and_an_escort_an_escort() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 2, vec![warship(None)]);
        launch(&mut session, &catalog);
        let labels: Vec<_> = fleet_npcs(&session)
            .iter()
            .map(|npc| (npc.ship, session.target_tag(npc)))
            .collect();
        assert_eq!(
            labels,
            [
                (WARSHIP, Some(crate::TargetTag::Escort)),
                (VIPER, Some(crate::TargetTag::Fighter))
            ]
        );
    }

    #[test]
    fn a_fighter_out_flies_beside_the_player_as_its_carrier() {
        let catalog = carrying();
        let session = fleet(&catalog, 0, vec![warship(None), out()]);
        let npcs = fleet_npcs(&session);
        assert_eq!(npcs[0].carrier, None, "an escort of its own");
        assert_eq!(
            npcs[1].carrier,
            Some(Carrier {
                ship: ShipRef::Player,
                window: 100.0,
                reach: 40.0,
            })
        );
    }

    #[test]
    fn return_to_hangar_sends_the_fighters_home_and_any_other_escort_to_formation() {
        let catalog = carrying();
        let mut session = fleet(
            &catalog,
            0,
            vec![warship(Some(EscortOrder::Defend)), out(), out()],
        );
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Recall,
                targeted: false,
                underway: false
            }),
            "the warship went back to formation"
        );
        let dock = Some(EscortOrder::Dock);
        assert_eq!(orders(&session), [None, dock, dock]);
        assert_eq!(
            fleet_npcs(&session)
                .iter()
                .map(|npc| npc.escort.and_then(|duty| duty.order))
                .collect::<Vec<_>>(),
            [None, dock, dock]
        );
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires),
            None,
            "nothing changed"
        );
        let mut session = fleet(&catalog, 0, vec![warship(None), out()]);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Dock,
                targeted: false,
                underway: false
            }),
            "no escort's order changed to formation"
        );
    }

    #[test]
    fn return_to_hangar_to_the_fighters_reaches_the_fighters_alone() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![warship(Some(EscortOrder::Hold)), out()]);
        let fighters = EscortGroup::Class(EscortClass::Fighter);
        assert_eq!(
            session
                .command_escorts(fighters, EscortCommand::Dock, &mut NeverFires)
                .map(|commanded| commanded.command),
            Some(EscortCommand::Dock)
        );
        assert_eq!(
            orders(&session),
            [Some(EscortOrder::Hold), Some(EscortOrder::Dock)]
        );
    }

    /// Ticks of the player at rest and Nova's traffic until no fighter is
    /// out, at most `limit`; how many it took.
    pub(in crate::session) fn until_docked(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        limit: u32,
    ) -> Option<u32> {
        let ai = NovaAi::default();
        (1..=limit).find(|_| {
            work_with(
                session,
                catalog,
                Controls::default(),
                &ai,
                Rules::default(),
                &mut NeverFires,
            );
            session
                .pilot()
                .escorts()
                .iter()
                .all(|escort| !escort.carried)
        })
    }

    #[test]
    fn recalled_fighters_fly_home_dock_and_are_the_bays_rounds_again() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out(), warship(None), out()]);
        for id in [
            session.space.escorts.fleet[0],
            session.space.escorts.fleet[2],
        ]
        .into_iter()
        .flatten()
        {
            let npc = session.space.npc_mut(id).expect("out");
            npc.state.position = npc.state.position + Vec2::new(300.0, 0.0);
        }
        session.take_save_due();
        assert_eq!(session.secondary_line_count(), Some(0));
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        let ticks = until_docked(&mut session, &catalog, 600);
        assert!(ticks.is_some(), "{:?}", fleet_npcs(&session));
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(session.npcs().len(), 1, "their NPCs are gone");
        assert_eq!(
            fleet_npcs(&session)[0]
                .escort
                .map(|duty| (duty.formation.slot, duty.formation.ships)),
            Some((2, 2)),
            "the rest re-form"
        );
        assert_eq!(session.pilot().owned(VIPERS), 2, "aboard again");
        assert_eq!(session.secondary_line_count(), Some(2));
        assert!(!session.take_save_due(), "as rounds are");
    }

    #[test]
    fn a_fighter_docking_on_double_times_second_pass_is_aboard_again() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out()]);
        let id = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(id).expect("out").state.position = Vec2::new(300.0, 0.0);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert!(matches!(session.npcs()[0].goal, Goal::Dock(_)), "decided");
        let docked = (1..=600).find(|_| {
            session.tick_again(
                Controls::default(),
                &crate::ai::NovaAi::default(),
                crate::Rules::default(),
                &mut NeverFires,
            );
            session.pilot().escorts().is_empty()
        });
        assert!(docked.is_some(), "{:?}", fleet_npcs(&session));
        assert_eq!(session.npcs(), []);
        assert_eq!(session.pilot().owned(VIPERS), 1, "aboard again");
    }

    /// Game work passes of the player at rest and Nova's traffic until no
    /// fighter is out, at most `limit`; how many it took.
    fn until_docked_in_game_work(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        limit: u32,
    ) -> Option<u32> {
        let ai = NovaAi::default();
        (1..=limit).find(|_| {
            session.game_work(
                Controls::default(),
                catalog,
                &ai,
                Rules::default(),
                &mut NeverFires,
            );
            session
                .pilot()
                .escorts()
                .iter()
                .all(|escort| !escort.carried)
        })
    }

    #[test]
    fn a_fighter_docking_in_the_ship_loop_below_another_slot_makes_no_save_due() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out(), warship(None)]);
        let fighter = session.space.escorts.fleet[0].expect("placed");
        let escort = session.space.escorts.fleet[1].expect("placed");
        let berth = |session: &Session, id| session.space.npc(id).expect("here").berth;
        assert!(berth(&session, fighter) < berth(&session, escort));
        let npc = session.space.npc_mut(fighter).expect("out");
        npc.state.position = npc.state.position + Vec2::new(300.0, 0.0);
        session.take_save_due();
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        let passes = until_docked_in_game_work(&mut session, &catalog, 600);
        assert!(passes.is_some(), "{:?}", fleet_npcs(&session));
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(
            session.space.escorts.fleet,
            [Some(escort)],
            "the fleet no longer holds it"
        );
        assert_eq!(session.pilot().owned(VIPERS), 1, "aboard again");
        assert!(!session.take_save_due(), "as docking makes none");
    }

    #[test]
    fn a_fighter_docking_on_double_times_second_pass_below_another_slot_makes_no_save_due() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out(), warship(None)]);
        let fighter = session.space.escorts.fleet[0].expect("placed");
        let escort = session.space.escorts.fleet[1].expect("placed");
        let npc = session.space.npc_mut(fighter).expect("out");
        npc.state.position = npc.state.position + Vec2::new(300.0, 0.0);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        session.take_save_due();
        let docked = (1..=600).find(|_| {
            session.tick_again(
                Controls::default(),
                &NovaAi::default(),
                Rules::default(),
                &mut NeverFires,
            );
            session.pilot().escorts().len() == 1
        });
        assert!(docked.is_some(), "{:?}", fleet_npcs(&session));
        assert_eq!(
            session.space.escorts.fleet,
            [Some(escort)],
            "the fleet no longer holds it"
        );
        assert_eq!(session.pilot().owned(VIPERS), 1, "aboard again");
        assert!(!session.take_save_due(), "as docking makes none");
    }

    #[test]
    fn another_command_keeps_a_returning_fighter_out() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out()]);
        let id = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(id).expect("out").state.position = Vec2::new(300.0, 0.0);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        session.command_escorts(EscortGroup::All, EscortCommand::Recall, &mut NeverFires);
        assert_eq!(until_docked(&mut session, &catalog, 600), None);
        assert_eq!(session.pilot().escorts().len(), 1);
        assert_eq!(session.pilot().owned(VIPERS), 0);
    }

    #[test]
    fn by_the_other_reading_a_fighter_no_bay_launches_is_lost_when_it_docks() {
        let mut catalog = carrying();
        catalog.hulls[0].weapons.clear();
        let rules = kept_orders().with_override(RuleKey::FighterNoBay, RuleSource::Bible);
        let mut session = flying_by(&catalog, pilot(&catalog, 0, vec![out()]), rules);
        session.take_save_due();
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert!(until_docked(&mut session, &catalog, 600).is_some());
        assert_eq!(session.pilot().escorts(), []);
        assert_eq!(session.npcs(), []);
        assert_eq!(session.pilot().owned(VIPERS), 0, "no round");
        assert!(session.take_save_due(), "a fighter lost");
    }

    #[test]
    fn a_fighter_disabled_or_destroyed_leaves_the_fleet_with_no_round_back() {
        let catalog = carrying();
        for armor in [5.0, 0.0] {
            let mut session = fleet(&catalog, 1, vec![out()]);
            session.take_save_due();
            let id = session.space.escorts.fleet[0].expect("placed");
            session.space.npc_mut(id).expect("out").reserves.armor.now = armor;
            work(&mut session, &catalog, Controls::default());
            assert_eq!(session.pilot().escorts(), [], "{armor}");
            assert_eq!(session.pilot().owned(VIPERS), 1, "{armor}");
            assert!(session.take_save_due(), "{armor}");
        }
    }

    // NPC carriers.

    /// Keeps its goal, fires as any ship does at what it attacks, and
    /// targets that: a carrier that attacks for as long as a test says.
    #[derive(Debug)]
    struct Raiding;

    impl Behaviour for Raiding {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
            crate::ai::fire::trigger(npc, around, crate::ai::fire::FireRules::default())
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.attacking()
        }
    }

    /// Nova's AI with every leaderless warship raiding and every
    /// interceptor sitting: a carrier with a lead flies `_EscortAI` under
    /// it ([`FleetEscortAi`](crate::FleetEscortAi)), attacking while its
    /// lead does, and its fighters fly it under the carrier, attacking
    /// while it attacks and docking once it stops.
    pub(in crate::session) fn raiding_ai() -> NovaAi {
        NovaAi::default()
            .with(AiType::Interceptor, Rc::new(Sitting))
            .with(AiType::WimpyTrader, Rc::new(Sitting))
            .with(AiType::Warship, Rc::new(Raiding))
    }

    /// [`raiding_ai`] with the ships with an NPC parent raiding too: a
    /// led carrier keeps whatever goal a test gives it, a snipe among them.
    fn sniping_ai() -> NovaAi {
        raiding_ai().with_fleet_escorts(Rc::new(Raiding))
    }

    /// [`raiding_ai`] by the other reading of [`RuleKey::NpcEscortAi`],
    /// the port's earlier routing: the carrier, which escorts its lead,
    /// raids by its type, and its fighters fly
    /// [`CarriedAi`](crate::CarriedAi).
    fn earlier_raiding_ai() -> NovaAi {
        NovaAi::from_rulebook(&earlier_escorts())
            .with(AiType::Interceptor, Rc::new(Sitting))
            .with(AiType::WimpyTrader, Rc::new(Sitting))
            .with(AiType::Warship, Rc::new(Raiding))
    }

    /// The rules by the other reading of [`RuleKey::NpcEscortAi`].
    pub(in crate::session) fn earlier_escorts() -> Rulebook {
        Rulebook::default().with_override(RuleKey::NpcEscortAi, RuleSource::Bible)
    }

    pub(in crate::session) const PIRATES: crate::catalog::GovtId = crate::catalog::GovtId(142);

    /// Adds a pirate carrier, a warship 60 across, 200 above the player
    /// and attacking it, escorting `lead`, carrying a Viper Bay with
    /// `vipers` fighters aboard.
    pub(in crate::session) fn carrier(session: &mut Session, lead: NpcId, vipers: u32) -> NpcId {
        session.space.traffic.add_npc(carrier_npc(lead, vipers))
    }

    /// The pirate carrier [`carrier`] adds: a medium ship, so its lead,
    /// attacking with its shield full, orders it to Attack on its first
    /// order frame (see `nova_sim_ai::ai::escort_orders`).
    pub(in crate::session) fn carrier_npc(lead: NpcId, vipers: u32) -> Npc {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(160);
        npc.govt = Some(PIRATES);
        npc.ai_type = AiType::Warship;
        npc.class = EscortClass::Medium;
        npc.leader = Some(lead);
        npc.state.position = Vec2::new(0.0, -200.0);
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&bay()),
            1,
        )]);
        npc.rounds = LauncherRounds::from([(BAY, i32::try_from(vipers).expect("few"))]);
        npc.hull.sprite_size = Some(60.0);
        npc
    }

    /// The Vipers in the system.
    pub(in crate::session) fn vipers(session: &Session) -> Vec<Npc> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == VIPER)
            .cloned()
            .collect()
    }

    /// The fighters `carrier` holds.
    pub(in crate::session) fn aboard(session: &Session, carrier: NpcId) -> u32 {
        session
            .space
            .npc(carrier)
            .map_or(0, |npc| u32::try_from(npc.rounds.slot(BAY)).unwrap_or(0))
    }

    /// A session with a pirate carrier of 2 Vipers attacking the player,
    /// and the carrier's lead far off, attacking it too; the carrier and
    /// the lead.
    pub(in crate::session) fn raided(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId) {
        raided_by(catalog, RuleSource::Engine)
    }

    /// [`raided`], an NPC's pick spending as `spend` reads
    /// [`RuleKey::AiSpendWithoutShot`]: by the other reading the carrier's
    /// kept bay, fired as its pick on the tick its reload runs out, drains
    /// no fighter (see `an_npc_carrier_launches_its_fighters_at_its_target`),
    /// so both its fighters launch.
    pub(in crate::session) fn raided_by(
        catalog: &FakePilotCatalog,
        spend: RuleSource,
    ) -> (Session, NpcId, NpcId) {
        raided_by_rules(
            catalog,
            Rulebook::default().with_override(RuleKey::AiSpendWithoutShot, spend),
        )
    }

    /// [`raided`] by `rules`, the fighters' launch by the engine.
    pub(in crate::session) fn raided_by_rules(
        catalog: &FakePilotCatalog,
        rules: Rulebook,
    ) -> (Session, NpcId, NpcId) {
        let rules = rules.with_override(RuleKey::FighterLaunch, RuleSource::Engine);
        let mut session = flying_by(catalog, pilot(catalog, 0, Vec::new()), rules);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        let npc = session.space.npc_mut(lead).expect("there");
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        let raider = carrier(&mut session, lead, 2);
        (session, raider, lead)
    }

    #[test]
    fn an_npc_carriers_glow_bay_glows_never() {
        let catalog = carrying_a_glow_bay();
        let (mut session, raider, _) = raided(&catalog);
        let glow_bay = crate::combat::weapon::WeaponSpec::new(&catalog.weapons[0]);
        assert!(glow_bay.glows());
        session.space.npc_mut(raider).expect("there").armament =
            crate::combat::armament::Armament::new([(glow_bay, 1)]);
        fight(&mut session, &catalog, &raiding_ai());
        assert_eq!(vipers(&session).len(), 1, "launched");
        assert_eq!(glows(&session.take_combat_events()), [], "_AILaunchFighter");
    }

    #[test]
    fn an_npc_carrier_launches_its_fighters_at_its_target() {
        let catalog = carrying();
        let (mut session, raider, lead) = raided(&catalog);
        let ai = raiding_ai();
        fight(&mut session, &catalog, &ai);
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1);
        let fighter = &launched[0];
        // Launched at the carrier, (0, -200), it moves in its launch frame,
        // as `_DoPlayGameWork`'s ship loop (@0x4445c-0x444c4) comes after
        // its AI loop.
        assert_eq!(
            fighter.state.position,
            Vec2::new(0.0, -204.0),
            "moved on from the carrier"
        );
        assert_eq!(fighter.govt, Some(PIRATES));
        assert_eq!(
            fighter.carrier,
            Some(Carrier {
                ship: ShipRef::Npc(raider),
                window: 100.0,
                reach: 60.0,
            })
        );
        // Its parent is the carrier (`_LaunchFighter` @0x3d866-0x3d870),
        // its target the carrier's, and `_AISetup` leaves it goal 0
        // (@0x7f3e8), gliding its bay's `Count` (@0x3d843-0x3d85b) until
        // its first think. In a slot above its carrier's, that think is
        // in its launch frame, as `_DoPlayGameWork`'s AI loop rereads each
        // slot (@0x44270-0x442ae; `_GenericRandomShipSpawn` @0x3c90b): its
        // formation block ends the glide (see the `carrier_lead` module).
        assert_eq!(fighter.leader, Some(raider), "the carrier, not its lead");
        assert_ne!(fighter.leader, Some(lead));
        assert_eq!(fighter.target, Some(ShipRef::Player));
        assert_eq!(
            fighter.goal,
            Goal::Follow {
                lead: raider,
                guard: Some(ShipRef::Player)
            }
        );
        assert_eq!(fighter.ai_wait, 0);
        assert_eq!(fighter.escort, None);
        assert_eq!(aboard(&session, raider), 1);
        // The launch sets the bay's reload to 60 in `_AIDispatch`
        // (@0x8e6fa), and `_HandleShip` counts it down that tick
        // (@0x33896). 59 ticks on it runs out in `_HandleShip`'s countdown,
        // after that tick's `_AILaunchFighter` found it reloading, and the
        // bay, its kept pick, fires as the pick: `_FireAIShipWeapon`
        // spawns nothing, and by the engine drains a fighter
        // (@0x88cd8-0x88dac, `RuleKey::AiSpendWithoutShot`).
        for _ in 0..59 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 1, "the bay reloads 60 ticks");
        assert_eq!(aboard(&session, raider), 0, "drained by its pick");
        fight(&mut session, &catalog, &ai);
        assert_eq!(vipers(&session).len(), 1, "none left to launch");
        // The one launched may dock back as its carrier draws off: none
        // more is ever out.
        let mut most = 0;
        for _ in 0..240 {
            fight(&mut session, &catalog, &ai);
            most = most.max(vipers(&session).len());
        }
        assert_eq!(most, 1, "none more");
        assert!(
            session.reserves().shield.now < 30.0,
            "the fighters' shots: {:?}",
            session.reserves()
        );
        let carrier = session.space.npc(raider).expect("there");
        assert_eq!(carrier.reserves.shield.now, 30.0, "never hit by its own");
        assert!(
            vipers(&session)
                .iter()
                .all(|npc| npc.reserves.shield.now == 39.99),
            "full at a fighter's capacity"
        );
    }

    #[test]
    fn by_the_other_reading_a_carriers_kept_bay_drains_nothing_and_it_launches_every_60_ticks() {
        // `RuleKey::AiSpendWithoutShot`'s other reading: the pick that
        // spawns nothing spends nothing, so the next `_AILaunchFighter`
        // launches the fighter the engine's reading drains.
        let catalog = carrying();
        let (mut session, raider, _) = raided_by(&catalog, RuleSource::Bible);
        let ai = raiding_ai();
        fight(&mut session, &catalog, &ai);
        assert_eq!(vipers(&session).len(), 1);
        for _ in 0..59 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 1, "the bay reloads 60 ticks");
        assert_eq!(aboard(&session, raider), 1, "nothing drained");
        fight(&mut session, &catalog, &ai);
        assert_eq!(vipers(&session).len(), 2);
        assert_eq!(aboard(&session, raider), 0);
    }

    #[test]
    fn a_sniping_carrier_whose_launch_finds_no_slot_launches_nothing_and_takes_its_bay_as_its_pick()
    {
        // A goal-4 carrier sniping at rest 100 above the player, facing it,
        // with a gun its pass finds. With 55 ships in the system its launch
        // finds no slot (@0x3d648-0x3d662) and leaves the bay ready, so the
        // snipe's own `_AILaunchFighter` makes the bay its pick after its
        // passes (@0x8732f, @0x81411). Both launches fail before the sound,
        // the round and the reload (@0x81421-0x81423); the pick, the bay,
        // ready, is then fired by `_FireAIShipWeapon` with the gun pass's
        // flag, which spawns nothing and by the engine spends a fighter
        // (@0x88cd8-0x88dac, `RuleKey::AiSpendWithoutShot`).
        let catalog = carrying();
        let sniped = |ships: u32| {
            let (mut session, raider, _) = raided(&catalog);
            let gun = WeaponRecord {
                speed: 1000,
                count: 20,
                energy_dmg: 1,
                mass_dmg: 1,
                ..weapon(128)
            };
            let npc = session.space.npc_mut(raider).expect("there");
            npc.goal = Goal::Snipe(ShipRef::Player);
            npc.state.position = Vec2::new(0.0, -100.0);
            npc.state.heading = 180.0;
            npc.armament = crate::combat::armament::Armament::new([
                (crate::combat::weapon::WeaponSpec::new(&gun), 1),
                (crate::combat::weapon::WeaponSpec::new(&bay()), 1),
            ]);
            for n in 0..ships - 2 {
                pirate(&mut session, 1000.0 + n as f32, 1000.0);
            }
            fight(&mut session, &catalog, &sniping_ai());
            (session, raider)
        };
        let (mut full, raider) = sniped(55);
        assert_eq!(vipers(&full), [], "no fighter");
        assert_eq!(aboard(&full, raider), 1, "drained by its pick alone");
        let carrier = full.space.npc(raider).expect("there");
        assert_eq!(carrier.pick, Some(BAY));
        let bay = carrier
            .armament
            .mounts()
            .iter()
            .find(|mount| mount.spec.id == BAY)
            .expect("carried");
        assert_eq!(bay.reload, 0.0, "no launch reloads it");
        let heard = full
            .take_combat_events()
            .into_iter()
            .any(|event| matches!(event, CombatEvent::Fired { weapon, .. } if weapon == BAY));
        assert!(!heard, "no bay heard");
        let (room, raider) = sniped(54);
        assert_eq!(vipers(&room).len(), 1);
        assert_eq!(aboard(&room, raider), 1);
        assert_eq!(
            room.space.npc(raider).expect("there").pick,
            None,
            "its gun fired and cleared"
        );
    }

    /// A session with [`raided_by`]'s carrier, by `spend`, sniping at rest
    /// 100 above the player, facing it, carrying only `bays` (each record
    /// and its fighters aboard), the system filled with pirates to
    /// `ships`, after one [`fight`]: the session and the carrier.
    fn sniping_with(
        catalog: &FakePilotCatalog,
        spend: RuleSource,
        bays: &[(WeaponRecord, i32)],
        ships: u32,
    ) -> (Session, NpcId) {
        let (mut session, raider, _) = raided_by(catalog, spend);
        let npc = session.space.npc_mut(raider).expect("there");
        npc.goal = Goal::Snipe(ShipRef::Player);
        npc.state.position = Vec2::new(0.0, -100.0);
        npc.state.heading = 180.0;
        npc.armament = crate::combat::armament::Armament::new(
            bays.iter()
                .map(|(record, _)| (crate::combat::weapon::WeaponSpec::new(record), 1)),
        );
        npc.rounds = bays
            .iter()
            .map(|(record, aboard)| (record.id, *aboard))
            .collect();
        for n in 0..ships - 2 {
            pirate(&mut session, 1000.0 + n as f32, 1000.0);
        }
        fight(&mut session, catalog, &sniping_ai());
        (session, raider)
    }

    /// The bays heard launching at `carrier` among `session`'s fight
    /// events, in order.
    fn launches_heard(session: &mut Session, carrier: NpcId) -> Vec<WeaponId> {
        session
            .take_combat_events()
            .into_iter()
            .filter_map(|event| match event {
                CombatEvent::Fired { ship, weapon, .. }
                    if ship == ShipRef::Npc(carrier) && weapon != GUN =>
                {
                    Some(weapon)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_sniping_carrier_whose_bay_reloads_in_zero_launches_twice_a_tick_while_two_slots_are_free()
    {
        // The high level's launch (@0x8e6fa) sets a reload of 0 / 1 = 0
        // (@0x81473-0x81491), so the snipe's own `_AILaunchFighter`
        // (@0x8732f) finds the bay ready (@0x81403-0x8140b) and launches a
        // second fighter, heard (@0x81466), into the next free slot.
        // The bay is then its pick, ready, which its gun pass flags
        // (@0x80213-0x8021d): `_FireAIShipWeapon` spawns nothing and by
        // the engine drains a fighter (@0x88cd8-0x88dac,
        // `RuleKey::AiSpendWithoutShot`). The counts below take the other
        // reading, which drains none.
        let catalog = carrying();
        let zero = WeaponRecord { reload: 0, ..bay() };
        let sniped = |ships| sniping_with(&catalog, RuleSource::Bible, &[(zero, 3)], ships);
        let (drained, raider) = sniping_with(&catalog, RuleSource::Engine, &[(zero, 3)], 53);
        assert_eq!(vipers(&drained).len(), 2);
        assert_eq!(aboard(&drained, raider), 0, "drained by its pick");
        let (mut two, raider) = sniped(53);
        assert_eq!(vipers(&two).len(), 2, "two slots free below 56");
        assert_eq!(aboard(&two, raider), 1);
        assert_eq!(launches_heard(&mut two, raider), [BAY, BAY]);
        // With one slot the second `_LaunchFighter` fails (@0x81421-0x81423)
        // after its pick write (@0x81411).
        let (mut one, raider) = sniped(54);
        assert_eq!(vipers(&one).len(), 1);
        assert_eq!(aboard(&one, raider), 2);
        assert_eq!(launches_heard(&mut one, raider), [BAY]);
        assert_eq!(one.space.npc(raider).expect("there").pick, Some(BAY));
        let (mut none, raider) = sniped(55);
        assert_eq!(vipers(&none), []);
        assert_eq!(aboard(&none, raider), 3);
        assert_eq!(launches_heard(&mut none, raider), []);
    }

    #[test]
    fn a_sniping_carrier_whose_first_bay_empties_launches_from_its_next_bay_the_same_tick() {
        // Stock-shaped: two bays of `Reload` 60. The high level's launch
        // spends 149's last round, so the snipe's call passes it over
        // (@0x813da) to 150, ready, and launches from it.
        let catalog = carrying();
        let next = WeaponRecord {
            id: WeaponId(150),
            ..bay()
        };
        let (mut session, raider) =
            sniping_with(&catalog, RuleSource::Engine, &[(bay(), 1), (next, 1)], 53);
        assert_eq!(vipers(&session).len(), 2);
        assert_eq!(
            launches_heard(&mut session, raider),
            [BAY, WeaponId(150)],
            "one from each"
        );
        let carrier = session.space.npc(raider).expect("there");
        assert_eq!(carrier.rounds.slot(BAY), 0);
        assert_eq!(carrier.rounds.slot(WeaponId(150)), 0);
    }

    // NPC launches in `_AIDispatch`, through the game's work.

    /// One advance pass of `session`'s game work, the player at rest, its
    /// NPCs deciding as `behaviour` says.
    pub(in crate::session) fn game_pass(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        behaviour: &impl Behaviour,
    ) {
        let rescued = session.game_work(
            Controls::default(),
            catalog,
            behaviour,
            Rules::default(),
            &mut NeverFires,
        );
        assert!(!rescued);
    }

    /// Keeps each goal and target, and asks every NPC for the goal-4
    /// launch alone: no pick, no point defence.
    #[derive(Debug)]
    struct Launching;

    impl Behaviour for Launching {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, _npc: &Npc, _around: &Surroundings) -> Trigger {
            Trigger {
                bays: true,
                ..Trigger::default()
            }
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// The bay launches heard among `events`, by carrier.
    fn bays_heard(events: &[CombatEvent]) -> Vec<ShipRef> {
        events
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired { ship, weapon, .. } if weapon == BAY => Some(ship),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_npcs_fighter_moves_on_its_launch_tick() {
        // `_AILaunchFighter` → `_LaunchFighter` in `_AIDispatch` marks the
        // slot active (@0x3c90b, @0x3d680) before `_HandleShip` (@0x44483)
        // runs that frame: the NPC twin of
        // `a_fighter_the_player_launches_moves_on_its_launch_tick`.
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let stood = session.space.npc(raider).expect("there").state.position;
        game_pass(&mut session, &catalog, &Launching);
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1, "launched");
        assert_ne!(launched[0].state.position, stood, "moved off the bay");
    }

    #[test]
    fn an_npc_launches_on_the_advance_pass_alone() {
        // `_AIDispatch` runs on the advance pass alone (@0x44026-0x4402a):
        // a bay ready by the advance pass's countdown waits through double
        // time's second pass for the next advance pass.
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        // A tick left: reloading as the advance pass's AI runs, ready after
        // its `_HandleShip`'s countdown (@0x33896).
        let carrier = session.space.npc_mut(raider).expect("there");
        carrier.armament.hold(BAY);
        for _ in 0..59 {
            carrier.armament.reload();
        }
        assert_eq!(carrier.armament.mounts()[0].reload, 1.0);
        game_pass(&mut session, &catalog, &Launching);
        assert_eq!(vipers(&session), [], "reloading as its AI ran");
        assert_eq!(
            session.space.npc(raider).expect("there").armament.mounts()[0].reload,
            0.0
        );
        session.tick_again(
            Controls::default(),
            &Launching,
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(vipers(&session), [], "no AI on the second pass");
        game_pass(&mut session, &catalog, &Launching);
        assert_eq!(vipers(&session).len(), 1);
    }

    #[test]
    fn two_carriers_sharing_the_last_free_berth_the_lower_slot_launches() {
        // The lower slot's `_AILaunchFighter` takes the last slot below 56
        // before the higher slot's AI runs (@0x44270-0x442ae); the higher's
        // `_LaunchFighter` then fails before the sound, the round and the
        // reload (@0x81421-0x81423). The higher was added first.
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 0, Vec::new()), RuleSource::Engine);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        let high = session.space.traffic.add_npc_at(carrier_npc(lead, 2), 40);
        let low = session.space.traffic.add_npc_at(carrier_npc(lead, 2), 20);
        while session.space.traffic.free_berths(SPAWN_RESERVE) > 1 {
            pirate(&mut session, 1000.0, 1000.0);
        }
        session.take_combat_events();
        game_pass(&mut session, &catalog, &Launching);
        let launched = vipers(&session);
        assert_eq!(launched.len(), 1);
        assert_eq!(
            launched[0].carrier.map(|carrier| carrier.ship),
            Some(ShipRef::Npc(low))
        );
        assert_eq!(launched[0].berth, 55, "the last berth below 56");
        assert_eq!(aboard(&session, low), 1, "its round spent");
        assert_eq!(aboard(&session, high), 2, "its round kept");
        let reload = session.space.npc(high).expect("there").armament.mounts()[0].reload;
        assert_eq!(reload, 0.0, "nor reloading");
        assert_eq!(
            bays_heard(&session.take_combat_events()),
            [ShipRef::Npc(low)],
            "the higher's bay unheard"
        );
    }

    /// Scripts each NPC's trigger by its number: point defence for the
    /// defenders, the goal-4 launch for the carrier once `launch` is set,
    /// at its own target, and the missile for the picker, at the low
    /// defender.
    #[derive(Debug)]
    struct Scripted {
        defenders: [NpcId; 2],
        carrier: NpcId,
        picker: NpcId,
        launch: std::cell::Cell<bool>,
    }

    impl Behaviour for Scripted {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn trigger(&self, npc: &Npc, _around: &Surroundings) -> Trigger {
            if self.defenders.contains(&npc.id) {
                Trigger {
                    point_defence: true,
                    ..Trigger::default()
                }
            } else if npc.id == self.carrier {
                Trigger {
                    bays: self.launch.get(),
                    ..Trigger::default()
                }
            } else if npc.id == self.picker {
                Trigger {
                    only: Some(MISSILE),
                    ..Trigger::default()
                }
            } else {
                Trigger::default()
            }
        }

        /// The picker's, the low defender; the carrier keeps its own, as
        /// with none `_HandleShip` would hold its bay at its `Reload`
        /// (@0x3390a-0x3394e).
        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            if npc.id == self.picker {
                Some(ShipRef::Npc(self.defenders[0]))
            } else {
                npc.target.filter(|_| npc.id == self.carrier)
            }
        }
    }

    /// A homing missile fired every tick, 5 pixels a tick for 100.
    const MISSILE: WeaponId = WeaponId(134);
    /// The Quad Light Blaster Turret's point defence.
    pub(in crate::session) const QUAD: WeaponId = WeaponId(133);

    /// An NPC at (`x`, `y`) at rest, armed with `weapon`.
    pub(in crate::session) fn armed_npc(x: f32, y: f32, weapon: WeaponRecord) -> Npc {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.state.position = Vec2::new(x, y);
        npc.armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&weapon),
            1,
        )]);
        npc
    }

    #[test]
    fn an_npcs_launch_is_heard_after_a_lower_slots_point_defence_before_a_higher_slots_and_before_every_pick()
     {
        // `_AIDispatch`'s slot loop (@0x44270-0x442ae): slot 2's
        // `_AIFireTurret` (@0x80b0d), slot 3's `_AILaunchFighter`
        // (@0x8e6fa), slot 4's `_AIFireTurret`; then `_HandleShot`
        // (@0x44385) and the picks in `_HandleShip` (@0x44483), slot 1's
        // among them.
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 0, Vec::new()), RuleSource::Engine);
        let missile = WeaponRecord {
            guidance: 1,
            speed: 500,
            count: 100,
            guided_turn: 70,
            durability: 4,
            mass_dmg: 20,
            energy_dmg: 10,
            ..weapon(134)
        };
        let quad = WeaponRecord {
            guidance: 9,
            reload: 5,
            count: 12,
            speed: 2000,
            mass_dmg: 1,
            energy_dmg: 4,
            ..weapon(133)
        };
        let picker = session
            .space
            .traffic
            .add_npc_at(armed_npc(1000.0, 850.0, missile), 1);
        let low = session
            .space
            .traffic
            .add_npc_at(armed_npc(1000.0, 1000.0, quad), 2);
        let raider = session.space.traffic.add_npc_at(carrier_npc(low, 2), 3);
        session.space.npc_mut(raider).expect("there").state.position = Vec2::new(1100.0, 1000.0);
        let mut wing = armed_npc(1030.0, 1000.0, quad);
        wing.leader = Some(low);
        let high = session.space.traffic.add_npc_at(wing, 4);
        let script = Scripted {
            defenders: [low, high],
            carrier: raider,
            picker,
            launch: std::cell::Cell::new(false),
        };
        game_pass(&mut session, &catalog, &script);
        session.take_combat_events();
        script.launch.set(true);
        game_pass(&mut session, &catalog, &script);
        let fired: Vec<(ShipRef, WeaponId)> = session
            .take_combat_events()
            .iter()
            .filter_map(|event| match *event {
                CombatEvent::Fired { ship, weapon, .. } => Some((ship, weapon)),
                _ => None,
            })
            .collect();
        assert_eq!(
            fired,
            [
                (ShipRef::Npc(low), QUAD),
                (ShipRef::Npc(raider), BAY),
                (ShipRef::Npc(high), QUAD),
                (ShipRef::Npc(picker), MISSILE),
            ]
        );
    }

    #[test]
    fn the_players_fighter_launches_against_its_led_top_speed() {
        // The carrier moves at 8 along the fighter's heading: past the
        // Viper's own top speed of 6, and past 6 + 1.5, but below 1.5 x 6.
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let from = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::new(0.0, -8.0),
            heading: 0.0,
        };
        let sortie = |carrier| Sortie {
            carrier,
            bay: BAY,
            ship: VIPER,
            from,
            heading: 0.0,
            target: None,
        };
        let launched = |session: &mut Session, carrier| {
            assert!(
                session
                    .launch_fighter(&sortie(carrier), &mut NeverFires)
                    .is_some()
            );
            session.npcs().last().expect("launched").state.velocity
        };
        assert_eq!(
            launched(&mut session, ShipRef::Player),
            launch_velocity(from.velocity, 0.0, 4.0, 9.0),
            "pushed: below the led top speed"
        );
        assert_ne!(launch_velocity(from.velocity, 0.0, 4.0, 9.0), from.velocity);
        assert_eq!(
            launched(&mut session, ShipRef::Npc(raider)),
            from.velocity,
            "at or past its own top speed: no gain"
        );
    }

    #[test]
    fn a_fighter_launches_against_its_classs_bare_top_speed_by_the_engine() {
        // The Viper carries a reactor (+300 `Speed`) by default; its carrier
        // moves at 8 along the heading: past 6, below 9.
        let mut catalog = carrying();
        catalog
            .outfits
            .push(outfit(330, &[(crate::stats::MORE_SPEED, 300)]));
        catalog.ship_records[1].defaults = vec![(OutfitId(330), 1)];
        let from = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::new(0.0, -8.0),
            heading: 0.0,
        };
        for (items, top) in [(RuleSource::Engine, 6.0), (RuleSource::Bible, 9.0)] {
            let (session, raider, _) = raided(&catalog);
            let mut session = session
                .with_rules(Rulebook::default().with_override(RuleKey::NpcDefaultItems, items));
            let sortie = Sortie {
                carrier: ShipRef::Npc(raider),
                bay: BAY,
                ship: VIPER,
                from,
                heading: 0.0,
                target: None,
            };
            assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
            let fighter = session.npcs().last().expect("launched");
            assert_eq!(fighter.stats.handling.max_speed, top, "{items:?}");
            assert_eq!(
                fighter.state.velocity,
                launch_velocity(from.velocity, 0.0, 4.0, top),
                "{items:?}"
            );
        }
    }

    #[test]
    fn a_fighter_rolls_its_skill_and_launches_against_its_own_top_speed() {
        // A Viper of `SkillVar` 37: 75 sides, the last 1.37. The carrier
        // moves at 8 along the heading.
        let mut catalog = carrying();
        catalog.ship_records[1].skill_var = 37;
        let (mut session, raider, _) = raided(&catalog);
        session.space.npc_mut(raider).expect("there").skill_mult = 2.0;
        let from = ShipState {
            position: Vec2::ZERO,
            velocity: Vec2::new(0.0, -8.0),
            heading: 0.0,
        };
        let sortie = |carrier| Sortie {
            carrier,
            bay: BAY,
            ship: VIPER,
            from,
            heading: 0.0,
            target: None,
        };
        let skill = (f64::from(74 - 37 + 100) * 0.01) as f32;
        let launched = |session: &mut Session, carrier| {
            let mut chance = Ranged::of(&[(75, 74)]);
            assert!(
                session
                    .launch_fighter(&sortie(carrier), &mut chance)
                    .is_some()
            );
            assert!(chance.was_asked(75));
            session.npcs().last().expect("launched").clone()
        };
        let mine = launched(&mut session, ShipRef::Player);
        assert_eq!((mine.skill, mine.skill_mult), (skill, 1.0));
        assert_eq!(
            mine.state.velocity,
            launch_velocity(from.velocity, 0.0, 4.0, 6.0 * skill * 1.5),
            "its own top speed, led"
        );
        let theirs = launched(&mut session, ShipRef::Npc(raider));
        assert_eq!(
            (theirs.skill, theirs.skill_mult),
            (skill, 2.0),
            "its carrier's factor"
        );
        assert_eq!(
            theirs.state.velocity,
            launch_velocity(from.velocity, 0.0, 4.0, 6.0 * skill * 2.0),
        );
        assert_ne!(theirs.state.velocity, from.velocity, "pushed past 8");
    }

    #[test]
    fn a_fighter_launched_carries_its_classs_crew_pods() {
        let mut catalog = carrying();
        catalog.hulls[1].pod_count = 2;
        let (mut session, raider, _) = raided(&catalog);
        let sortie = Sortie {
            carrier: ShipRef::Npc(raider),
            bay: BAY,
            ship: VIPER,
            from: ShipState::default(),
            heading: 0.0,
            target: None,
        };
        assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
        assert_eq!(session.npcs().last().expect("launched").pods_left, 2);
        catalog.hulls[1].pod_count = -3;
        let (mut session, _, _) = raided(&catalog);
        assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
        assert_eq!(
            session.npcs().last().expect("launched").pods_left,
            -3,
            "raw, as _LaunchFighter writes it (@0x3d8fe)"
        );
    }

    #[test]
    fn a_fighter_launched_glows_at_32_its_anim_owed_on_its_own_class() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let sortie = Sortie {
            carrier: ShipRef::Npc(raider),
            bay: BAY,
            ship: VIPER,
            from: ShipState::default(),
            heading: 0.0,
            target: None,
        };
        assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
        let fighter = session.npcs().last().expect("launched");
        assert_eq!(
            fighter.display.glow,
            crate::traffic::ship_display::GLOW_SPAWN,
            "`_GenericRandomShipSpawn`'s, through `_LaunchFighter`"
        );
        assert_eq!(fighter.display.rolls.anim, AnimRoll::Own);
    }

    #[test]
    fn a_fighter_of_a_folding_class_launches_folded_to_its_last_set() {
        let mut catalog = carrying();
        catalog.hulls[1].display = Some(crate::catalog::ShipDisplay {
            flags: crate::combat::ship_frames::FOLDS,
            base_sets: 3,
            ..crate::catalog::ShipDisplay::default()
        });
        let (mut session, raider, _) = raided(&catalog);
        let sortie = Sortie {
            carrier: ShipRef::Npc(raider),
            bay: BAY,
            ship: VIPER,
            from: ShipState::default(),
            heading: 0.0,
            target: None,
        };
        assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
        let fold = session.npcs().last().expect("launched").armament.fold();
        assert_eq!(
            (fold.set, fold.dir),
            (2, 0),
            "`_LaunchFighter`'s BaseSets - 1"
        );
    }

    #[test]
    fn a_fighter_takes_its_carriers_speed_along_its_heading() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        session.ship.player_speed = 2.5;
        session.space.npc_mut(raider).expect("there").speed = 1.5;
        for (carrier, speed) in [(ShipRef::Player, 2.5), (ShipRef::Npc(raider), 1.5)] {
            let sortie = Sortie {
                carrier,
                bay: BAY,
                ship: VIPER,
                from: ShipState::default(),
                heading: 0.0,
                target: None,
            };
            assert!(session.launch_fighter(&sortie, &mut NeverFires).is_some());
            assert_eq!(session.npcs().last().expect("launched").speed, speed);
        }
    }

    #[test]
    fn a_carrier_without_a_lead_leads_its_fighters_and_a_target_it_leads_is_dropped() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 0, Vec::new()), RuleSource::Engine);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        let raider = carrier(&mut session, lead, 2);
        let npc = session.space.npc_mut(raider).expect("there");
        npc.leader = None;
        npc.target = Some(ShipRef::Npc(lead));
        npc.goal = Goal::Attack(ShipRef::Npc(lead));
        // The lead is of its fleet only once it follows the carrier.
        session.space.npc_mut(lead).expect("there").leader = Some(raider);
        let ai = raiding_ai();
        fight(&mut session, &catalog, &ai);
        let fighter = &vipers(&session)[0];
        assert_eq!(fighter.leader, Some(raider));
        assert_eq!(fighter.target, None, "a ship its carrier leads");
        // Above its carrier's slot, it thinks in its launch frame, as
        // `_DoPlayGameWork`'s AI loop rereads each slot (@0x44270-0x442ae):
        // with no target, it keeps formation on its carrier.
        assert_eq!(
            fighter.goal,
            Goal::Follow {
                lead: raider,
                guard: None
            }
        );
    }

    #[test]
    fn once_its_carrier_stops_attacking_its_fighters_dock_and_it_holds_them_again() {
        let catalog = carrying();
        let (mut session, raider, _) = raided_by(&catalog, RuleSource::Bible);
        let ai = raiding_ai();
        for _ in 0..61 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 2);
        session.space.npc_mut(raider).expect("there").goal = Goal::Idle;
        let docked = (0..600).find(|_| {
            fight(&mut session, &catalog, &ai);
            vipers(&session).is_empty()
        });
        assert!(docked.is_some(), "{:?}", vipers(&session));
        assert_eq!(aboard(&session, raider), 2);
    }

    #[test]
    fn by_the_other_reading_an_npc_carriers_fighters_fly_carried_ai_attacking_then_docking() {
        // The port's earlier routing: the led carrier raids by its type,
        // and its fighters attack its quarry and dock once it stops.
        let catalog = carrying();
        let (mut session, raider, _) = raided_by_rules(&catalog, earlier_escorts());
        let ai = earlier_raiding_ai();
        fight(&mut session, &catalog, &ai);
        let id = vipers(&session)[0].id;
        fight(&mut session, &catalog, &ai);
        assert_eq!(
            session.space.npc(id).expect("out").goal,
            Goal::Attack(ShipRef::Player),
            "its carrier's quarry"
        );
        session.space.npc_mut(raider).expect("there").goal = Goal::Idle;
        let docked = (0..600).find(|_| {
            fight(&mut session, &catalog, &ai);
            session.space.npc(id).is_none()
        });
        assert!(docked.is_some(), "{:?}", vipers(&session));
        assert_eq!(aboard(&session, raider), 2, "the fighter aboard again");
    }

    #[test]
    fn by_the_other_reading_a_carriers_fighters_fly_by_their_own_ai_once_it_is_destroyed() {
        // The port's earlier lost-lead pass (`lost_lead`).
        let catalog = carrying();
        let (mut session, raider, _) = raided_by_rules(
            &catalog,
            Rulebook::default()
                .with_override(RuleKey::AiSpendWithoutShot, RuleSource::Bible)
                .with_override(RuleKey::NpcEscortAi, RuleSource::Bible),
        );
        let ai = raiding_ai();
        for _ in 0..61 {
            fight(&mut session, &catalog, &ai);
        }
        session
            .space
            .npc_mut(raider)
            .expect("there")
            .reserves
            .armor
            .now = 0.0;
        for _ in 0..3 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.space.npc(raider), None, "destroyed");
        let orphans = vipers(&session);
        assert_eq!(orphans.len(), 2);
        for orphan in &orphans {
            assert_eq!(orphan.carrier, None);
            assert_eq!(orphan.leader, None);
            assert_eq!(orphan.goal, Goal::Idle);
        }
        let landed = session.clone();
        let mut landed = landed;
        fight(&mut landed, &catalog, &ai);
        assert!(
            vipers(&landed).iter().all(|npc| npc.goal == Goal::Idle),
            "the interceptors sit, by their AI type"
        );
    }

    #[test]
    fn by_default_a_destroyed_carriers_fighters_are_succeeded_at_the_next_think() {
        // `_AICheckParentExists` reads the carrier's record, gone from its
        // slot: the first fighter by berth (equal mass) leads, the other
        // follows it, both off the carrier.
        let catalog = carrying();
        let (mut session, raider, _) = raided_by(&catalog, RuleSource::Bible);
        let ai = raiding_ai();
        for _ in 0..61 {
            fight(&mut session, &catalog, &ai);
        }
        session
            .space
            .npc_mut(raider)
            .expect("there")
            .reserves
            .armor
            .now = 0.0;
        // Up to the pass it is destroyed in, whose display comes after
        // its AI loop.
        for passes in 0.. {
            assert!(passes < 3, "destroyed within 3 passes");
            work(&mut session, &catalog, Controls::default());
            if session.space.npc(raider).is_none() {
                break;
            }
        }
        assert!(
            vipers(&session)
                .iter()
                .all(|npc| npc.leader == Some(raider) && npc.carrier.is_some()),
            "until the next think"
        );
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &ai,
            Rules::default(),
            &mut NeverFires,
        );
        let mut fighters = vipers(&session);
        fighters.sort_by_key(|npc| npc.berth);
        let [first, second] = &fighters[..] else {
            panic!("two fighters: {fighters:?}");
        };
        assert_eq!((first.leader, first.carrier), (None, None));
        assert_eq!((second.leader, second.carrier), (Some(first.id), None));
    }

    #[test]
    fn the_sprites_read_up_front_include_the_types_bays_launch() {
        let catalog = carrying();
        let session = Session::fly(&catalog, pilot(&catalog, 0, Vec::new())).expect("flies");
        assert_eq!(session.traffic_ships(), [VIPER], "the player's bay");
        let mut catalog = carrying();
        catalog.hulls[0].weapons.clear();
        catalog.hulls.push(HullRecord {
            weapons: vec![StockWeapon {
                weapon: BAY,
                count: 1,
                ammo: 2,
            }],
            ..hull(160)
        });
        catalog.ship_records.push(ship(160, FAST));
        let mut dude_types = [(-1, 0); 8];
        dude_types[0] = (128, 100);
        catalog.traffic = vec![(
            crate::catalog::SystemId(130),
            crate::catalog::SystemTraffic {
                dude_types,
                avg_ships: 1,
                persons: Default::default(),
            },
        )];
        catalog.dudes = vec![(
            crate::catalog::DudeId(128),
            crate::catalog::DudeRecord {
                ai_type: 3,
                govt: None,
                ships: vec![(ShipId(160), 1)],
                booty: 0,
                info_types: 0,
            },
        )];
        let mut session = Session::fly(&catalog, pilot(&catalog, 0, Vec::new())).expect("flies");
        assert_eq!(session.traffic_ships(), [], "no bay yet");
        session.populate(&catalog, &mut NeverFires);
        assert_eq!(session.traffic_ships(), [VIPER, ShipId(160)]);
    }

    // Leaving the system.

    /// The Dart's bay, 151, carrying ship 145.
    const DART_BAY: WeaponId = WeaponId(151);
    /// The Dart: a fighter holding no fuel.
    const DART: ShipId = ShipId(145);
    /// The Dart outfit, the Dart bay's rounds.
    const DARTS: OutfitId = OutfitId(162);

    /// [`carrying`], the player's ship carrying a Dart bay too, held by
    /// outfit 161.
    fn two_bays() -> FakePilotCatalog {
        let mut catalog = carrying();
        catalog.weapons.push(WeaponRecord {
            ammo_type: 145,
            ..WeaponRecord {
                id: DART_BAY,
                ..bay()
            }
        });
        catalog.ship_records.push(ShipRecord {
            escort_type: 0,
            inherent_ai: 4,
            ..ship(
                145,
                ShipFields {
                    fuel: 0,
                    ..VIPER_FIELDS
                },
            )
        });
        catalog.hulls.push(HullRecord {
            size: Some(20),
            ..hull(145)
        });
        catalog.hulls[0].weapons.push(StockWeapon {
            weapon: DART_BAY,
            count: 1,
            ammo: 0,
        });
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            max: 9999,
            ..outfit(162, &[(MOD_AMMO, 151)])
        });
        catalog.outfits.push(outfit(161, &[(MOD_WEAPON, 151)]));
        catalog
    }

    /// A Dart out of its bay.
    fn dart_out() -> Escort {
        Escort {
            ship: DART,
            ..out()
        }
    }

    /// A Viper and a Dart out, by `recall`, the Viper's shield at 10, the
    /// Dart 5000 off, too far to turn back and dock before a jump begins
    /// (see the `wait_for_me` module).
    fn both_out(catalog: &FakePilotCatalog, recall: RuleSource) -> Session {
        let mut session = fleet(catalog, 0, vec![out(), dart_out()])
            .with_rules(kept_orders().with_override(RuleKey::FighterRecall, recall));
        let viper = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(viper)
            .expect("out")
            .reserves
            .shield
            .now = 10.0;
        let dart = session.space.escorts.fleet[1].expect("placed");
        session.space.npc_mut(dart).expect("out").state.position = Vec2::new(-5000.0, 0.0);
        work(&mut session, &catalog, Controls::default());
        session.take_save_due();
        session
    }

    #[test]
    fn by_the_engine_a_fighter_with_a_jumps_fuel_follows_and_the_rest_are_abandoned() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 10.0);
        assert_eq!(
            crate::testkit::jump(&mut session, &catalog, 131),
            Some(crate::catalog::SystemId(131))
        );
        assert_eq!(
            session
                .pilot()
                .escorts()
                .iter()
                .map(|escort| escort.ship)
                .collect::<Vec<_>>(),
            [VIPER],
            "the Dart's record is gone"
        );
        let npcs = fleet_npcs(&session);
        assert_eq!(npcs.len(), 1);
        let viper = &npcs[0];
        let duty = viper.escort.expect("an escort");
        // Pushed back from its slot beside the player to glide in (the
        // `escort_arrival` module).
        let slot = crate::flight::ShipState {
            position: duty.formation.position(session.player()),
            ..*session.player()
        };
        assert_eq!(
            viper.state,
            crate::escort_arrival::pushed(slot),
            "pushed back from its slot beside the player"
        );
        assert_eq!(viper.reserves.shield.now, 10.0, "as it was");
        assert_eq!(session.npcs().len(), 1, "no Dart");
        assert_eq!(session.pilot().owned(DARTS), 0, "no round back");
        assert_eq!(session.take_fighter_notes(), [FighterNote::Abandoned(1)]);
        assert_eq!(session.take_fighter_notes(), [], "taken");
        assert!(session.take_save_due());
    }

    /// Plots a course to 131, flies out and into the jump, runs `then` on
    /// the session as the jump ends, after the passes on the way, and
    /// arrives.
    fn jump_to_131_then(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        then: impl FnOnce(&mut Session),
    ) -> Option<crate::catalog::SystemId> {
        session
            .plot_course(crate::catalog::SystemId(131))
            .expect("a route");
        crate::testkit::fly_out(session, catalog);
        crate::testkit::begin_jump_now(session, catalog).expect("jumps");
        then(session);
        session.arrive(catalog, &mut NeverFires)
    }

    /// The ships of the player's escorts after a jump to 131, `then` run
    /// as the jump ends ([`jump_to_131_then`]).
    fn jumped_with(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        then: impl FnOnce(&mut Session),
    ) -> Vec<ShipId> {
        assert_eq!(
            jump_to_131_then(session, catalog, then),
            Some(crate::catalog::SystemId(131))
        );
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.ship)
            .collect()
    }

    #[test]
    fn by_the_engine_a_fighter_of_a_class_with_a_jumps_fuel_follows_with_an_empty_tank() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        let viper = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(viper).expect("out").reserves.fuel.now = 0.0;
        assert_eq!(jumped_with(&mut session, &catalog, |_| {}), [VIPER]);
    }

    #[test]
    fn by_the_engine_a_fighter_another_ship_tractors_is_abandoned_and_one_holding_itself_follows() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        // Locked as the jump ends: the passes on the way let go of a lock
        // no ship holds.
        let viper = session.space.escorts.fleet[0].expect("placed");
        let held = |session: &mut Session| {
            session.space.npc_mut(viper).expect("out").tractor = Some(
                crate::combat::impact::TractorLock::by(ShipRef::Npc(NpcId(99))),
            );
        };
        assert_eq!(jumped_with(&mut session, &catalog, held), []);
        assert_eq!(session.take_fighter_notes(), [FighterNote::Abandoned(2)]);
        let mut session = both_out(&catalog, RuleSource::Engine);
        let viper = session.space.escorts.fleet[0].expect("placed");
        let holding = |session: &mut Session| {
            session.space.npc_mut(viper).expect("out").tractor =
                Some(crate::combat::impact::TractorLock::by(ShipRef::Npc(viper)));
        };
        assert_eq!(jumped_with(&mut session, &catalog, holding), [VIPER]);
    }

    #[test]
    fn by_the_engine_a_fighter_not_in_the_system_follows_by_its_ship_types_fuel() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        for index in 0..2 {
            let id = session.space.escorts.fleet[index].take().expect("placed");
            session.space.traffic.remove(id);
        }
        assert_eq!(
            jumped_with(&mut session, &catalog, |_| {}),
            [VIPER],
            "the Viper's type holds a jump's fuel, the Dart's none"
        );
    }

    #[test]
    fn by_the_other_jump_fuel_reading_a_fighter_with_an_empty_tank_is_abandoned() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        session.setup.rules = session
            .setup
            .rules
            .with_override(RuleKey::NpcJumpFuel, RuleSource::Bible);
        let viper = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(viper).expect("out").reserves.fuel.now = 99.0;
        assert_eq!(jumped_with(&mut session, &catalog, |_| {}), []);
    }

    /// [`two_bays`] with `STR#` 1000 "one", "two", and system 131's
    /// `Message` `message`.
    fn buoyed_131(message: i16) -> FakePilotCatalog {
        let mut catalog = two_bays();
        catalog
            .strings
            .push((crate::buoy::BUOY_STRINGS, vec!["one".into(), "two".into()]));
        let system = catalog
            .star_map
            .iter_mut()
            .find(|system| system.id == crate::catalog::SystemId(131))
            .expect("on the map");
        system.message = message;
        catalog
    }

    /// The fighter notes, messages and fighters left of a jump to 131 with
    /// both fighters out by the engine, `Message` `message` there and
    /// `MissingBuoyString` read as `buoy`.
    fn abandoning_into(
        message: i16,
        buoy: RuleSource,
    ) -> (Vec<FighterNote>, Vec<crate::SimMessage>, usize, bool) {
        let catalog = buoyed_131(message);
        let mut session = both_out(&catalog, RuleSource::Engine);
        session.setup.rules = session
            .setup
            .rules
            .with_override(RuleKey::MissingBuoyString, buoy);
        // The Dart's "wait for me" on the way is the `wait_for_me`
        // module's.
        jump_to_131_then(&mut session, &catalog, |session| {
            session.take_messages();
        })
        .expect("arrives");
        (
            session.take_fighter_notes(),
            session.take_messages(),
            session.pilot().escorts().len(),
            session.take_save_due(),
        )
    }

    #[test]
    fn a_buoy_drops_the_abandoned_fighters_note_and_the_fighters_still_go() {
        let (notes, messages, escorts, save_due) = abandoning_into(2, RuleSource::Engine);
        assert_eq!(notes, [], "the buoy replaces the suffix");
        assert_eq!(messages, [crate::SimMessage::Buoy("two".into())]);
        assert_eq!(escorts, 1, "the Dart is still abandoned");
        assert!(save_due);
        let (notes, messages, ..) = abandoning_into(3, RuleSource::Engine);
        assert_eq!(notes, [], "an empty line replaces it too");
        assert_eq!(messages, [crate::SimMessage::Buoy(String::new())]);
    }

    #[test]
    fn with_no_buoy_shown_the_abandoned_fighters_note_stays() {
        let arrived = [crate::SimMessage::Arrived(crate::catalog::SystemId(131))];
        for (message, buoy) in [(-1, RuleSource::Engine), (3, RuleSource::Bible)] {
            let (notes, messages, escorts, _) = abandoning_into(message, buoy);
            assert_eq!(notes, [FighterNote::Abandoned(1)], "{message} {buoy:?}");
            assert_eq!(messages, arrived, "{message} {buoy:?}");
            assert_eq!(escorts, 1);
        }
    }

    #[test]
    fn by_the_engine_landed_the_fighters_stay_out_and_take_off_restocked() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().escorts().len(), 2, "still out");
        assert_eq!(session.pilot().owned(VIPERS), 0);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        let npcs = fleet_npcs(&session);
        assert_eq!(
            npcs.iter().map(|npc| npc.ship).collect::<Vec<_>>(),
            [VIPER, DART]
        );
        assert_eq!(npcs[0].reserves.shield.now, 30.0, "restocked");
        assert_eq!(session.take_fighter_notes(), []);
    }

    #[test]
    fn by_the_other_reading_every_fighter_out_is_back_aboard_on_arrival() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Bible);
        assert_eq!(
            session.rules().source_for(RuleKey::FighterRecall),
            RuleSource::Bible
        );
        crate::testkit::jump(&mut session, &catalog, 131);
        assert_eq!(session.pilot().escorts(), []);
        assert_eq!(session.npcs(), [], "none out");
        assert_eq!(
            (session.pilot().owned(VIPERS), session.pilot().owned(DARTS)),
            (1, 1)
        );
        assert_eq!(session.take_fighter_notes(), [], "none abandoned");
        assert!(!session.take_save_due(), "as docking makes none");
    }

    #[test]
    fn by_the_other_reading_landing_puts_every_fighter_out_back_aboard() {
        let catalog = two_bays();
        let mut session = fleet(&catalog, 0, vec![warship(None), out(), dart_out()])
            .with_rules(kept_orders().with_override(RuleKey::FighterRecall, RuleSource::Bible));
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(
            (session.pilot().owned(VIPERS), session.pilot().owned(DARTS)),
            (1, 1)
        );
        assert!(
            session.npcs().iter().all(|npc| npc.carrier.is_none()),
            "their NPCs gone"
        );
        assert_eq!(session.space.escorts.fleet.len(), 1);
    }

    /// [`two_bays`], the ship starting over hypergate 300 at 130's
    /// centre, linked to hypergate 310 in 131.
    fn gated_bays() -> FakePilotCatalog {
        use crate::catalog::{GateSite, LandingSite, StellarId, SystemId};
        use crate::gate::HYPERGATE;
        let gate = |id, links: [Option<StellarId>; 8]| GateSite {
            id: StellarId(id),
            position: Vec2::ZERO,
            flags2: HYPERGATE,
            links,
            exit_angle: 90,
        };
        let mut to = [None; 8];
        to[0] = Some(StellarId(310));
        let mut back = [None; 8];
        back[0] = Some(StellarId(300));
        let mut catalog = two_bays();
        catalog.sites[0].1 = vec![LandingSite {
            flags2: HYPERGATE,
            ..crate::testkit::planet(300, 0.0, 0.0)
        }];
        catalog.gates = vec![gate(300, to), gate(310, back)];
        catalog.gate_listers = vec![(SystemId(131), StellarId(310))];
        catalog
    }

    /// Presses L over the hypergate, runs the dock approach, presses L
    /// again and comes out of it in 131.
    fn through_the_gate(session: &mut Session, catalog: &FakePilotCatalog) {
        use crate::catalog::SystemId;
        session.land().expect("selects the gate");
        session.dock_approach();
        assert!(matches!(
            session.land(),
            Ok(crate::session::LandPress::AtGate { .. })
        ));
        assert_eq!(
            session.enter_hypergate(Some(SystemId(131)), catalog, &mut NeverFires),
            Ok(SystemId(131))
        );
    }

    #[test]
    fn by_the_engine_a_gate_exit_abandons_the_fighters_without_a_jumps_fuel() {
        let catalog = gated_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        through_the_gate(&mut session, &catalog);
        assert_eq!(
            session
                .pilot()
                .escorts()
                .iter()
                .map(|escort| escort.ship)
                .collect::<Vec<_>>(),
            [VIPER],
            "the Dart's record is gone"
        );
        assert_eq!(session.pilot().owned(DARTS), 0, "no round back");
        assert_eq!(session.take_fighter_notes(), [FighterNote::Abandoned(1)]);
        assert!(session.take_save_due());
    }

    #[test]
    fn by_the_other_reading_a_gate_exit_puts_every_fighter_out_back_aboard() {
        let catalog = gated_bays();
        let mut session = both_out(&catalog, RuleSource::Bible);
        through_the_gate(&mut session, &catalog);
        assert_eq!(session.pilot().escorts(), []);
        assert_eq!(
            (session.pilot().owned(VIPERS), session.pilot().owned(DARTS)),
            (1, 1)
        );
        assert_eq!(session.take_fighter_notes(), [], "none abandoned");
    }

    #[test]
    fn a_control_bit_test_counts_a_fighter_out_as_its_outfit() {
        use crate::control::PilotFacts;
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 1, Vec::new()), RuleSource::Engine);
        assert!(
            session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(VIPERS),
            "aboard"
        );
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().owned(VIPERS), 0);
        assert!(
            session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(VIPERS),
            "out of its bay"
        );
        assert!(
            !session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(OutfitId(159))
        );
        let snapshot = super::super::spawn_availability::RefreshedFacts::of(
            &session.ship.facts(&session.pilot, &session.statics),
        );
        assert!(snapshot.has_outfit(VIPERS), "the snapshot counts it too");
        assert!(!snapshot.has_outfit(OutfitId(159)));
        // A carried escort of a type no bay launches docks into nothing.
        let stray = Escort {
            ship: WARSHIP,
            reserves: Reserves::full(10.0, 10.0, 10.0),
            order: None,
            carried: true,
            wage: None,
            person: None,
            mark: None,
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![stray])).expect("flies");
        assert!(
            !session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(VIPERS)
        );
        // Nor does an escort of the fighter's type that is not carried.
        let own = Escort {
            ship: VIPER,
            carried: false,
            ..stray
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![own])).expect("flies");
        assert!(
            !session
                .ship
                .facts(&session.pilot, &session.statics)
                .has_outfit(VIPERS)
        );
    }

    #[test]
    fn a_fighter_out_counts_as_its_bays_outfit_of_lowest_id() {
        use crate::control::PilotFacts;
        let mut catalog = carrying();
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            max: 9999,
            ..outfit(160, &[(MOD_AMMO, 149)])
        });
        catalog.outfits.push(OutfitRecord {
            mass: 0,
            max: 9999,
            ..outfit(157, &[(MOD_AMMO, 149)])
        });
        let fighter = Escort {
            ship: VIPER,
            reserves: Reserves::full(10.0, 10.0, 10.0),
            order: None,
            carried: true,
            wage: None,
            person: None,
            mark: None,
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![fighter])).expect("flies");
        let facts = session.ship.facts(&session.pilot, &session.statics);
        assert!(facts.has_outfit(OutfitId(157)));
        assert!(!facts.has_outfit(VIPERS));
        assert!(!facts.has_outfit(OutfitId(160)));
    }

    #[test]
    fn a_fighter_out_survives_a_save_and_reload_and_docks_again() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Engine);
        launch(&mut session, &catalog);
        let fighter = session.space.escorts.fleet[0].expect("out");
        session
            .space
            .npc_mut(fighter)
            .expect("out")
            .reserves
            .shield
            .now = 5.0;
        work(&mut session, &catalog, Controls::default());
        let saved = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&saved).expect("loads");
        assert_eq!(pilot.owned(VIPERS), 2);
        assert_eq!(pilot.escorts().len(), 1);
        assert!(pilot.escorts()[0].carried);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        let npcs = fleet_npcs(&session);
        assert_eq!(npcs.len(), 1, "placed on the first traffic tick");
        assert_eq!(npcs[0].reserves.shield.now, 30.0, "restocked");
        assert_eq!(
            npcs[0].carrier.map(|carrier| carrier.ship),
            Some(ShipRef::Player)
        );
        session.command_escorts(EscortGroup::All, EscortCommand::Dock, &mut NeverFires);
        assert!(until_docked(&mut session, &catalog, 600).is_some());
        assert_eq!(session.pilot().owned(VIPERS), 3);
    }

    #[test]
    fn by_the_other_reading_a_pilot_landed_is_saved_with_every_fighter_aboard() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Engine)
            .with_rules(
                Rulebook::default().with_override(RuleKey::FighterRecall, RuleSource::Bible),
            );
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().owned(VIPERS), 2);
        land_now(&mut session).expect("lands");
        let pilot = crate::save::decode(&crate::save::encode(session.pilot())).expect("loads");
        assert_eq!(pilot.owned(VIPERS), 3);
        assert_eq!(pilot.escorts(), []);
    }

    /// [`two_bays`], planet 128 an outfitter and a shipyard selling ship
    /// 129, and the pilot rich.
    fn spaceport() -> FakePilotCatalog {
        let mut catalog = two_bays();
        catalog.sites[0].1[0].flags |=
            crate::landing::StellarFlags::OUTFITTER | crate::landing::StellarFlags::SHIPYARD;
        catalog.sites[0].1[0].tech_level = 5;
        catalog.ships.push((ShipId(129), Ok(FAST)));
        catalog.ship_records.push(ship(129, FAST));
        catalog
    }

    #[test]
    fn buying_a_ship_loses_every_fighter_out_but_keeps_the_escorts() {
        let catalog = spaceport();
        let mut session = fleet(&catalog, 1, vec![out(), warship(None), dart_out()]);
        session.pilot.cash = 1_000_000;
        land_now(&mut session).expect("lands");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(session.pilot().owned(DARTS), 0, "no rounds");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(
            fleet_npcs(&session)
                .iter()
                .map(|npc| npc.ship)
                .collect::<Vec<_>>(),
            [WARSHIP]
        );
    }

    #[test]
    fn a_pilot_resumed_and_landed_with_fighters_out_loses_them_buying_a_ship() {
        let catalog = spaceport();
        let mut pilot = pilot(&catalog, 0, vec![out(), warship(None), dart_out()]);
        pilot.cash = 1_000_000;
        pilot.stellar = Some(crate::catalog::StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        land_now(&mut session).expect("lands on resuming");
        assert!(session.space.escorts.fleet.is_empty(), "none placed yet");
        session
            .buy_ship(ShipId(129), "Kestrel", &mut NeverFires)
            .expect("bought");
        assert_eq!(session.pilot().escorts(), [warship(None)]);
    }

    /// Whether the outfitter sells another Viper to a pilot with `aboard`
    /// Vipers aboard and `out` out.
    fn viper_for_sale(
        catalog: &FakePilotCatalog,
        aboard: u16,
        out_: usize,
    ) -> Result<(), crate::outfitter::OutfitRefusal> {
        let mut session = fleet(catalog, aboard, vec![out(); out_]);
        session.pilot.cash = 1_000_000;
        land_now(&mut session).expect("lands");
        session
            .outfitter(&mut NeverFires)
            .expect("an outfitter")
            .check(crate::outfitter::OutfitOrder {
                outfit: VIPERS,
                direction: crate::market::Direction::Buy,
            })
    }

    #[test]
    fn the_outfitter_sells_fighters_only_while_the_bays_have_room_counting_those_out() {
        let catalog = spaceport();
        let full = Err(crate::outfitter::OutfitRefusal::BaysFull);
        assert_eq!(viper_for_sale(&catalog, 3, 1), full, "MaxAmmo 4, one bay");
        assert_eq!(viper_for_sale(&catalog, 3, 0), Ok(()));
        assert_eq!(viper_for_sale(&catalog, 0, 4), full);
        let mut two = spaceport();
        two.hulls[0].weapons[0].count = 2;
        assert_eq!(viper_for_sale(&two, 7, 0), Ok(()), "two bays hold 8");
        // The fighters aboard (the outfit owned) reach `MaxAmmo` x bays:
        // `_HasMaxOfItem` (@0x4e87d) refuses before `_CanBuyFighter`
        // (@0x4e938) is asked.
        let max_owned = Err(crate::outfitter::OutfitRefusal::MaxOwned);
        assert_eq!(viper_for_sale(&two, 8, 0), max_owned);
        assert_eq!(viper_for_sale(&two, 7, 1), full, "one out");
        let mut by_outfit = spaceport();
        by_outfit.weapons[0].max_ammo = 0;
        by_outfit
            .outfits
            .iter_mut()
            .find(|record| record.id == VIPERS)
            .expect("the Viper outfit")
            .max = 5;
        assert_eq!(viper_for_sale(&by_outfit, 4, 0), Ok(()), "the outfit's Max");
        assert_eq!(viper_for_sale(&by_outfit, 4, 1), full);
        let mut no_bay = spaceport();
        no_bay.hulls[0].weapons.retain(|stock| stock.weapon != BAY);
        assert_eq!(
            viper_for_sale(&no_bay, 0, 0),
            max_owned,
            "no bay for it: MaxAmmo x 0"
        );
        // Each bay's fighters are counted on their own.
        let mut session = fleet(&catalog, 0, vec![out(); 4]);
        session.pilot.cash = 1_000_000;
        land_now(&mut session).expect("lands");
        assert_eq!(
            session
                .outfitter(&mut NeverFires)
                .expect("an outfitter")
                .check(crate::outfitter::OutfitOrder {
                    outfit: DARTS,
                    direction: crate::market::Direction::Buy,
                }),
            Ok(()),
            "the Dart bay has room, the Viper bay none"
        );
    }

    #[test]
    fn the_outfitters_count_of_a_fighter_outfit_is_the_owned_count_with_a_fighter_out() {
        // The grid's corner count reads `_outfitsOwned` alone
        // (@0x57471-0x574cb), as #219 and #220 do; only the Sell counts
        // fighters out (@0x5cb2a-0x5cb8e).
        let catalog = spaceport();
        let mut session = fleet(&catalog, 2, vec![out()]);
        land_now(&mut session).expect("lands");
        let outfitter = session.outfitter(&mut NeverFires).expect("an outfitter");
        assert_eq!(outfitter.row(VIPERS).expect("for sale").owned, 2);
        assert_eq!(
            session.launchers()[&VIPER_BAY].rounds,
            3,
            "the Sell's count"
        );
    }

    // A bay whose `AmmoType` names no ship record.

    /// [`carrying`] with no record of the Viper its bay launches.
    fn recordless() -> FakePilotCatalog {
        let mut catalog = carrying();
        catalog.ship_records.retain(|record| record.id != VIPER);
        catalog
    }

    #[test]
    fn the_players_bay_with_no_ship_record_launches_nothing_and_spends_nothing() {
        let catalog = recordless();
        let mut session = fleet(&catalog, 2, Vec::new());
        session.take_save_due();
        launch(&mut session, &catalog);
        assert_eq!(session.pilot().owned(VIPERS), 2, "the round put back");
        assert_eq!(session.pilot().escorts(), [], "no fleet change");
        assert_eq!(vipers(&session), []);
        assert!(!session.take_save_due());
    }

    #[test]
    fn an_npcs_bay_with_no_ship_record_launches_nothing_and_puts_its_round_back() {
        let catalog = recordless();
        let (mut session, raider, _) = raided(&catalog);
        let spent = 1;
        session.space.npc_mut(raider).expect("there").rounds = LauncherRounds::from([(BAY, spent)]);
        let sortie = Sortie {
            carrier: ShipRef::Npc(raider),
            bay: BAY,
            ship: VIPER,
            from: ShipState::default(),
            heading: 0.0,
            target: None,
        };
        assert_eq!(session.launch_fighters(&[sortie], &mut NeverFires), []);
        assert_eq!(aboard(&session, raider), 2, "a round again");
        assert_eq!(vipers(&session), []);
    }

    /// A missile launcher, 170, of `MaxAmmo` 10, held by outfit 171.
    const LAUNCHER: WeaponId = WeaponId(170);
    /// The launcher's ammunition outfit.
    const MISSILES: OutfitId = OutfitId(172);
    /// An outfit naming the launcher's ammunition in its second mod.
    const SECOND_SLOT: OutfitId = OutfitId(173);
    /// Ammunition, 176, for a launcher, 174, of `MaxAmmo` 0, held by
    /// outfit 175.
    const UNCAPPED: OutfitId = OutfitId(176);

    /// [`spaceport`] with the launchers above and their ammunition, all
    /// massless and of `Max` 9999.
    fn missile_port() -> FakePilotCatalog {
        let mut catalog = spaceport();
        catalog.weapons.push(WeaponRecord {
            max_ammo: 10,
            ..weapon(170)
        });
        catalog.weapons.push(weapon(174));
        let massless = |record: OutfitRecord| OutfitRecord {
            mass: 0,
            max: 9999,
            ..record
        };
        catalog.outfits.extend([
            massless(outfit(171, &[(MOD_WEAPON, 170)])),
            massless(outfit(172, &[(MOD_AMMO, 170)])),
            massless(outfit(
                173,
                &[(crate::stats::MORE_SPEED, 1), (MOD_AMMO, 170)],
            )),
            massless(outfit(175, &[(MOD_WEAPON, 174)])),
            massless(outfit(176, &[(MOD_AMMO, 174)])),
        ]);
        catalog
    }

    /// Whether the outfitter sells one more of `outfit` to a new pilot
    /// owning `owned` on top of the outfits its ship comes with (its stock
    /// weapons among them).
    fn ammo_for_sale(
        catalog: &FakePilotCatalog,
        owned: &[(i16, u16)],
        outfit: OutfitId,
    ) -> Result<(), crate::outfitter::OutfitRefusal> {
        let mut pilot = pilot(catalog, 0, Vec::new());
        pilot.cash = 1_000_000;
        for &(id, n) in owned {
            *pilot.outfits.entry(OutfitId(id)).or_insert(0) += n;
        }
        let mut session = Session::fly(catalog, pilot).expect("flies");
        land_now(&mut session).expect("lands");
        session
            .outfitter(&mut NeverFires)
            .expect("an outfitter")
            .check(crate::outfitter::OutfitOrder {
                outfit,
                direction: crate::market::Direction::Buy,
            })
    }

    #[test]
    fn ammunition_is_capped_at_its_max_ammo_times_the_launchers() {
        let catalog = missile_port();
        let max_owned = Err(crate::outfitter::OutfitRefusal::MaxOwned);
        assert_eq!(
            ammo_for_sale(&catalog, &[(171, 1), (172, 9)], MISSILES),
            Ok(())
        );
        assert_eq!(
            ammo_for_sale(&catalog, &[(171, 1), (172, 10)], MISSILES),
            max_owned,
            "MaxAmmo 10, one launcher"
        );
        assert_eq!(
            ammo_for_sale(&catalog, &[(171, 2), (172, 10)], MISSILES),
            Ok(()),
            "two launchers"
        );
        assert_eq!(
            ammo_for_sale(&catalog, &[(171, 2), (172, 20)], MISSILES),
            max_owned
        );
        assert_eq!(
            ammo_for_sale(&catalog, &[], MISSILES),
            max_owned,
            "no launcher"
        );
        let mut stock = missile_port();
        stock.hulls[0].weapons.push(StockWeapon {
            weapon: LAUNCHER,
            count: 1,
            ammo: 0,
        });
        assert_eq!(
            ammo_for_sale(&stock, &[(171, 1), (172, 10)], MISSILES),
            Ok(()),
            "a stock launcher counts"
        );
        assert_eq!(ammo_for_sale(&stock, &[(172, 10)], MISSILES), max_owned);
    }

    #[test]
    fn only_a_first_mod_of_ammunition_is_capped() {
        let catalog = missile_port();
        assert_eq!(
            ammo_for_sale(&catalog, &[(171, 1), (173, 10)], SECOND_SLOT),
            Ok(())
        );
        assert_eq!(ammo_for_sale(&catalog, &[(173, 10)], SECOND_SLOT), Ok(()));
    }

    #[test]
    fn ammunition_of_a_launcher_of_max_ammo_0_is_not_capped() {
        let catalog = missile_port();
        assert_eq!(
            ammo_for_sale(&catalog, &[(175, 1), (176, 10)], UNCAPPED),
            Ok(())
        );
        assert_eq!(ammo_for_sale(&catalog, &[(176, 10)], UNCAPPED), Ok(()));
    }

    #[test]
    fn the_ammunition_cap_is_a_wide_product() {
        let mut catalog = missile_port();
        catalog.weapons.last_mut().expect("weapon 174").max_ammo = 16_384;
        // 16,384 x 2 is 32,768, where the original's `imulw` wraps it to
        // -32,768; integer widths are not replicated, so one buys.
        assert_eq!(ammo_for_sale(&catalog, &[(175, 2)], UNCAPPED), Ok(()));
        assert_eq!(ammo_for_sale(&catalog, &[(175, 1)], UNCAPPED), Ok(()));
    }

    /// Sells a Viper Bay from a pilot with `bays` bays, `aboard` Vipers
    /// aboard and `out_` out, the sale read as `source` says; with the
    /// bays and credits owned after.
    fn sell_a_bay(
        bays: i16,
        aboard: u16,
        out_: usize,
        source: RuleSource,
    ) -> (Result<(), crate::outfitter::OutfitRefusal>, u16, i64) {
        let mut catalog = spaceport();
        catalog.hulls[0].weapons[0].count = bays;
        let mut session = fleet(&catalog, aboard, vec![out(); out_])
            .with_rules(kept_orders().with_override(RuleKey::LauncherSale, source));
        land_now(&mut session).expect("lands");
        let cash = session.pilot().cash();
        let sold = session.outfit(
            crate::outfitter::OutfitOrder {
                outfit: VIPER_BAY,
                direction: crate::market::Direction::Sell,
            },
            &mut NeverFires,
        );
        (
            sold,
            session.pilot().owned(VIPER_BAY),
            session.pilot().cash() - cash,
        )
    }

    fn vipers_first(rounds: u32) -> Result<(), crate::outfitter::OutfitRefusal> {
        Err(crate::outfitter::OutfitRefusal::AmmunitionFirst {
            rounds,
            ammo: Some(VIPERS),
        })
    }

    #[test]
    fn a_bay_cannot_be_sold_while_its_fighters_aboard_and_out_overfill_the_bays_left() {
        let engine = RuleSource::Engine;
        assert_eq!(sell_a_bay(1, 1, 1, engine), (vipers_first(2), 1, 0));
        assert_eq!(
            sell_a_bay(1, 0, 1, engine),
            (vipers_first(1), 1, 0),
            "one out"
        );
        assert_eq!(sell_a_bay(1, 0, 0, engine).0, Ok(()));
        assert_eq!(sell_a_bay(2, 3, 1, engine).0, Ok(()), "one bay holds 4");
        assert_eq!(sell_a_bay(2, 4, 1, engine), (vipers_first(1), 2, 0));
        assert_eq!(sell_a_bay(1, 2, 0, engine).1, 1);
    }

    #[test]
    fn a_raiser_does_not_widen_a_bays_room_past_the_fighters_raw_max() {
        // A bay of `MaxAmmo` 0 holds the Viper outfit's raw `Max` of 2
        // (`_CanBuyFighter` @0x5c3d-0x5c4f), which two raisers owned do
        // not raise, though they raise its `Max` to 4.
        let mut catalog = spaceport();
        catalog.weapons[0].max_ammo = 0;
        catalog
            .outfits
            .iter_mut()
            .find(|record| record.id == VIPERS)
            .expect("the Viper outfit")
            .max = 2;
        catalog
            .outfits
            .push(outfit(170, &[(crate::outfitter::RAISES_MAX, VIPERS.0)]));
        for (aboard, sold) in [
            (1, Ok(())),
            (2, Err(crate::outfitter::OutfitRefusal::BaysFull)),
        ] {
            let mut session = fleet(&catalog, aboard, vec![]);
            session.pilot.cash = 1_000_000;
            session.pilot.outfits.insert(OutfitId(170), 2);
            land_now(&mut session).expect("lands");
            let order = crate::outfitter::OutfitOrder {
                outfit: VIPERS,
                direction: crate::market::Direction::Buy,
            };
            assert_eq!(
                session
                    .outfitter(&mut NeverFires)
                    .expect("open")
                    .check(order),
                sold,
                "{aboard} aboard"
            );
        }
    }

    #[test]
    fn a_counted_sale_of_bays_stops_where_the_vipers_overfill_those_left() {
        // Three bays of 4 and five Vipers aboard: one bay can go, as two
        // hold 8, and the second cannot, as one holds 4.
        let mut catalog = spaceport();
        catalog.hulls[0].weapons[0].count = 3;
        let mut session = fleet(&catalog, 5, vec![]);
        land_now(&mut session).expect("lands");
        let order = crate::outfitter::OutfitOrder {
            outfit: VIPER_BAY,
            direction: crate::market::Direction::Sell,
        };
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(
            outfitter.count_max(VIPER_BAY, crate::market::Direction::Sell),
            Some(3)
        );
        assert_eq!(session.outfit_counted(order, 3, &mut NeverFires), Ok(1));
        assert_eq!(session.pilot().owned(VIPER_BAY), 2);
        assert_eq!(
            session.outfit(order, &mut NeverFires),
            Err(crate::outfitter::OutfitRefusal::AmmunitionFirst {
                rounds: 1,
                ammo: Some(VIPERS),
            })
        );
        // Refused for its Vipers, a bay still opens the dialog, and the
        // counted sale sells none.
        let outfitter = session.outfitter(&mut NeverFires).expect("open");
        assert_eq!(
            outfitter.count_max(VIPER_BAY, crate::market::Direction::Sell),
            Some(2)
        );
        assert_eq!(
            session.outfit_counted(order, 2, &mut NeverFires),
            Err(crate::outfitter::OutfitRefusal::AmmunitionFirst {
                rounds: 1,
                ammo: Some(VIPERS),
            })
        );
    }

    /// A session landed with four bays of 4 and eight Vipers aboard,
    /// [`RuleKey::CountedSale`] read as `source` says.
    fn four_bays(source: RuleSource) -> Session {
        let mut catalog = spaceport();
        catalog.hulls[0].weapons[0].count = 4;
        let mut session = fleet(&catalog, 8, vec![])
            .with_rules(kept_orders().with_override(RuleKey::CountedSale, source));
        land_now(&mut session).expect("lands");
        session
    }

    /// Selling Viper bays.
    const SELL_BAY: crate::outfitter::OutfitOrder = crate::outfitter::OutfitOrder {
        outfit: VIPER_BAY,
        direction: crate::market::Direction::Sell,
    };

    #[test]
    fn by_the_engine_a_counted_sale_of_bays_counts_each_sale_twice() {
        // Pass 0 keeps 3 x 4 for 8 and sells; pass 1, 3 owned, keeps
        // (3 - 1 - 1) x 4 = 4 and is refused for 4 Vipers.
        let refused = crate::outfitter::OutfitRefusal::AmmunitionFirst {
            rounds: 4,
            ammo: Some(VIPERS),
        };
        let mut session = four_bays(RuleSource::Engine);
        let ran = session.outfit_order(SELL_BAY, 3, &mut NeverFires);
        assert_eq!(
            ran.run,
            crate::order_loop::OrderRun {
                done: 1,
                stopped: Some(crate::order_loop::Stopped {
                    pass: 1,
                    refusal: refused,
                }),
            }
        );
        let alert = ran.alert.expect("alerts");
        assert_eq!(alert.refusal, refused);
        assert!(alert.plural);
        assert_eq!(session.pilot().owned(VIPER_BAY), 3);
        let mut session = four_bays(RuleSource::Engine);
        assert_eq!(
            session.outfit_order(SELL_BAY, 4, &mut NeverFires).run.done,
            1,
            "passes 2 and 3 refused too"
        );
        // Single-sale arithmetic sells two.
        let mut session = four_bays(RuleSource::Bible);
        assert_eq!(session.outfit_counted(SELL_BAY, 3, &mut NeverFires), Ok(2));
    }

    #[test]
    fn the_session_reads_the_counted_sale_as_its_rule_says() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            session.rules().source_for(RuleKey::CountedSale),
            RuleSource::Engine
        );
        for (source, sold) in [(RuleSource::Engine, Ok(1)), (RuleSource::Bible, Ok(2))] {
            let mut session = four_bays(source);
            assert_eq!(session.rules().source_for(RuleKey::CountedSale), source);
            assert_eq!(
                session.outfit_counted(SELL_BAY, 3, &mut NeverFires),
                sold,
                "{source:?}"
            );
        }
    }

    #[test]
    fn the_session_reads_the_launcher_sale_as_its_rule_says() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(
            session.rules().source_for(RuleKey::LauncherSale),
            RuleSource::Engine
        );
        let session = Session::start(&catalog()).expect("starts").with_rules(
            Rulebook::default().with_override(RuleKey::LauncherSale, RuleSource::Bible),
        );
        assert_eq!(
            session.rules().source_for(RuleKey::LauncherSale),
            RuleSource::Bible
        );
        assert_eq!(sell_a_bay(2, 4, 0, RuleSource::Engine).0, Ok(()));
        assert_eq!(
            sell_a_bay(2, 4, 0, RuleSource::Bible),
            (vipers_first(4), 2, 0)
        );
        assert_eq!(sell_a_bay(2, 0, 0, RuleSource::Bible).0, Ok(()));
    }
}
