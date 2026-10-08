//! The player's and the NPCs' fighter bays in flight: the session's side
//! of [`bay`](crate::bay).
//!
//! **Launching.** Each fighter a bay launches in the fight
//! ([`Sortie`]) is put in the system at its carrier, its shield and
//! armour full, its class's stock weapons and rounds aboard, its AI type
//! its `InherentAI`, pushed out at the bay's speed ([`launch_velocity`])
//! along the launch heading. With no ship slot free
//! ([`MAX_SHIPS_IN_SYSTEM`], the player counted), or no record of its
//! ship type, it goes back into its bay, a round again.
//!
//! - **The player's fighter** joins the fleet as a carried escort
//!   ([`Escort::carried`]), of no government, on its formation slot, as
//!   phase 8's escorts. What it does first follows the rulebook's
//!   [`RuleKey::FighterLaunch`](crate::RuleKey::FighterLaunch)
//!   ([`Session::with_fighter_launch`]): by the engine it takes the
//!   standing order of the fleet's first escort of its class, Return to
//!   Hangar excepted, and no target (`_LaunchFighter` @0x3dbec, `_EscortAI`
//!   @0x83ae4-0x83b34); by the other reading, launched while the player
//!   targets a ship that is not one of its escorts, it attacks that ship
//!   at once. Launching makes no save due, as firing ammunition does not.
//! - **An NPC's fighter** joins its carrier's fleet (its leader the
//!   carrier's lead, or the carrier), of the carrier's government, and
//!   attacks the carrier's target unless that is the carrier or of its
//!   fleet (@0x3dc00-0x3dd47); it flies by
//!   [`CarriedAi`](crate::CarriedAi).
//!
//! **Docking.** A fighter that docks ([`Outcome::Docked`]) goes back
//! aboard its carrier ([`Armament::stow`](crate::Armament::stow)): the
//! player's leaves the fleet, its round going to the first fighter
//! outfit of its bay, with no save due; one no bay of the player's
//! launches is lost, which makes one. An NPC's fighter whose carrier is
//! gone from the system or not intact is left on its own AI type, with
//! no carrier and no leader.

use std::collections::BTreeMap;

use super::Session;
use crate::ai::Goal;
use crate::bay::{Carrier, FighterNote, capacity, dock_window, launch_velocity};
use crate::board::MAX_SHIPS_IN_SYSTEM;
use crate::catalog::OutfitId;
use crate::combat::armament::{OutfitRounds, Rounds, Trigger, outfit_rounds};
use crate::combat::hull::Condition;
use crate::combat::{ShipRef, Sortie};
use crate::escort::{EscortClass, EscortDuty, EscortOrder, NO_SPRITE};
use crate::flight::ShipState;
use crate::hyperspace::JUMP_FUEL;
use crate::pilot::Escort;
use crate::rulebook::RuleSource;
use crate::traffic::autopilot::Outcome;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
use crate::traffic::table::{self, ShipKind};

impl Session {
    /// Puts each fighter of `sorties` in the system (see the module
    /// docs); one with no room, or whose ship type has no record, goes
    /// back into its bay.
    pub(super) fn launch_fighters(&mut self, sorties: &[Sortie]) {
        for sortie in sorties {
            if !self.launch_fighter(sortie) {
                self.put_back(sortie);
            }
        }
    }

    /// Puts the fighter of `sortie` in the system, and says whether it
    /// did.
    fn launch_fighter(&mut self, sortie: &Sortie) -> bool {
        if self.traffic.npcs().len() + 1 >= MAX_SHIPS_IN_SYSTEM {
            return false;
        }
        let Some(record) = self.ship_record(sortie.ship).cloned() else {
            return false;
        };
        let push = self.arsenal.weapon(sortie.bay).map_or(0.0, |bay| bay.speed);
        let kind = table::kind(&record, &self.outfits, &self.arsenal);
        let state = ShipState {
            position: sortie.from.position,
            velocity: launch_velocity(
                sortie.from.velocity,
                sortie.heading,
                push,
                kind.stats.handling.max_speed,
            ),
            heading: sortie.heading,
        };
        let carrier = Carrier {
            ship: sortie.carrier,
            window: dock_window(record.fields.maneuver),
            reach: self.reach_of(sortie.carrier),
        };
        let npc = fighter(&kind, sortie.ship, state, carrier);
        match sortie.carrier {
            ShipRef::Player => self.join_as_fighter(npc, sortie.target),
            ShipRef::Npc(id) => self.join_carrier(npc, id),
        }
        true
    }

    /// The reach of a fighter docking with `carrier`: its sprite's width,
    /// or [`NO_SPRITE`] with none.
    pub(super) fn reach_of(&self, carrier: ShipRef) -> f32 {
        let sprite = match carrier {
            ShipRef::Player => self.hull.sprite_size,
            ShipRef::Npc(id) => self.npc(id).and_then(|npc| npc.hull.sprite_size),
        };
        sprite.unwrap_or(NO_SPRITE)
    }

    /// The player's fighter `npc` joins the fleet, launched while the
    /// player targets `target` (see the module docs).
    fn join_as_fighter(&mut self, mut npc: Npc, target: Option<ShipRef>) {
        let target = target
            .filter(|_| self.fighter_launch == RuleSource::Bible)
            .filter(|&ship| match ship {
                ShipRef::Npc(id) => !self.is_escort(id),
                ShipRef::Player => false,
            });
        let order = if target.is_some() {
            Some(EscortOrder::Attack)
        } else {
            self.class_order(npc.class)
        };
        npc.escort = Some(EscortDuty {
            slot: 0,
            ships: 0,
            spacing: 0.0,
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
        };
        let id = self.traffic.add_npc(npc);
        self.pilot.escorts.push(escort);
        self.fleet.resize(self.pilot.escorts.len() - 1, None);
        self.fleet.push(Some(id));
        self.reform();
    }

    /// The standing order a fighter of `class` launched takes by the
    /// engine: its class's first escort's, Return to Hangar excepted.
    fn class_order(&self, class: EscortClass) -> Option<EscortOrder> {
        (0..self.pilot.escorts.len())
            .find(|&index| self.escort_class(index) == class)
            .and_then(|index| self.pilot.escorts[index].order)
            .filter(|&order| order != EscortOrder::Dock)
    }

    /// NPC `carrier`'s fighter `npc` joins its fleet, of its government,
    /// attacking its target unless that is the carrier or of its fleet.
    fn join_carrier(&mut self, mut npc: Npc, carrier: NpcId) {
        let Some(launcher) = self.npc(carrier).cloned() else {
            return;
        };
        let fleet = launcher.fleet();
        let target = launcher
            .target
            .filter(|&ship| ship != ShipRef::Npc(carrier) && self.fleet_of(ship) != Some(fleet));
        npc.govt = launcher.govt;
        npc.leader = Some(launcher.leader.unwrap_or(launcher.id));
        npc.aggression = launcher.aggression;
        npc.target = target;
        npc.goal = target.map_or(Goal::Idle, Goal::Attack);
        self.traffic.add_npc(npc);
    }

    /// `ship`'s fleet, if it is in the system.
    fn fleet_of(&self, ship: ShipRef) -> Option<ShipRef> {
        match ship {
            ShipRef::Player => Some(ShipRef::Player),
            ShipRef::Npc(id) => self.npc(id).map(Npc::fleet),
        }
    }

    /// Takes each fighter that docked on the last traffic tick aboard its
    /// carrier (see the module docs): the player's leaves the fleet, its
    /// round to its bay, and one no bay launches is lost, which makes a
    /// save due.
    pub(super) fn dock_fighters(&mut self) {
        let docked: Vec<(NpcId, ShipRef, crate::catalog::ShipId)> = self
            .traffic
            .departed()
            .iter()
            .filter_map(|&(id, outcome)| match outcome {
                Outcome::Docked { carrier, ship } => Some((id, carrier, ship)),
                _ => None,
            })
            .collect();
        for (id, carrier, ship) in docked {
            match carrier {
                ShipRef::Player => {
                    let mut rounds = OutfitRounds {
                        owned: &mut self.pilot.outfits,
                        sources: &self.ammo_outfits,
                    };
                    let stowed = self.armament.stow(ship, &mut rounds);
                    self.unfleet(id);
                    if !stowed {
                        self.save_due = true;
                    }
                }
                ShipRef::Npc(carrier) => {
                    if let Some(carrier) = self.npc_mut(carrier) {
                        carrier.armament.stow(ship, &mut carrier.rounds);
                    }
                }
            }
        }
    }

    /// Every NPC's fighter whose carrier is no longer in the system, or
    /// not intact, is left on its own AI type: no carrier, no leader, and
    /// idle until it decides (`_EscortAI` @0x83917-0x8397b).
    pub(super) fn orphan_fighters(&mut self) {
        let carriers: Vec<NpcId> = self
            .traffic
            .npcs()
            .iter()
            .filter(|npc| npc.condition == Condition::Intact)
            .map(|npc| npc.id)
            .collect();
        for npc in self.traffic.npcs_mut() {
            if let Some(Carrier {
                ship: ShipRef::Npc(carrier),
                ..
            }) = npc.carrier
                && !carriers.contains(&carrier)
            {
                npc.carrier = None;
                npc.leader = None;
                npc.goal = Goal::Idle;
            }
        }
    }

    /// The player leaves the system with its fighters out, as the
    /// rulebook's [`RuleKey::FighterRecall`](crate::RuleKey::FighterRecall)
    /// says (see the module docs): by the engine, on a jump's arrival
    /// (`landing` false), those whose ship type holds a jump's fuel follow
    /// and the rest are abandoned; by the other reading, on arrival and on
    /// landing, every one goes back aboard.
    pub(super) fn leave_with_fighters(&mut self, landing: bool) {
        match self.fighter_recall {
            RuleSource::Engine if !landing => self.abandon_fighters(),
            RuleSource::Engine => {}
            RuleSource::Bible => self.recall_fighters(),
        }
    }

    /// Every fighter out whose ship type lacks a jump's fuel is abandoned
    /// (`_HandlePlayer` @0x6c0b9-0x6c0f9): its record goes, with no round
    /// back, and the flight is told how many; a save is due when any was.
    fn abandon_fighters(&mut self) {
        let fuelled = |session: &Self, escort: &Escort| {
            session
                .ship_record(escort.ship)
                .is_some_and(|record| f32::from(record.fields.fuel) >= JUMP_FUEL)
        };
        let abandoned = self.drop_fighters(fuelled);
        let count = u32::try_from(abandoned.len()).unwrap_or(u32::MAX);
        if count > 0 {
            self.fighter_notes.push(FighterNote::Abandoned(count));
            self.save_due = true;
        }
    }

    /// Every fighter out goes back aboard at once, a round to the bay
    /// launching its type, as docking gives (`_InstantFighterRecall`
    /// @0x5df3); one no bay launches is lost, which makes a save due.
    fn recall_fighters(&mut self) {
        for fighter in self.drop_fighters(|_, _| false) {
            let mut rounds = OutfitRounds {
                owned: &mut self.pilot.outfits,
                sources: &self.ammo_outfits,
            };
            if !self.armament.stow(fighter.ship, &mut rounds) {
                self.save_due = true;
            }
        }
    }

    /// Every fighter out leaves the fleet but those `kept`, its NPC gone
    /// from the system and its record from the pilot; the records gone.
    pub(super) fn drop_fighters(&mut self, kept: impl Fn(&Self, &Escort) -> bool) -> Vec<Escort> {
        let mut dropped = Vec::new();
        for index in (0..self.pilot.escorts.len()).rev() {
            let escort = self.pilot.escorts[index];
            if !escort.carried || kept(self, &escort) {
                continue;
            }
            self.pilot.escorts.remove(index);
            if index < self.fleet.len()
                && let Some(id) = self.fleet.remove(index)
            {
                self.traffic.remove(id);
            }
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
        for &(ammo, outfit) in &self.ammo_outfits {
            let Some(bay) = self.arsenal.weapon(ammo).filter(|spec| spec.is_bay()) else {
                continue;
            };
            let bays: u32 = self
                .armament
                .mounts()
                .iter()
                .filter(|mount| mount.spec.id == ammo)
                .map(|mount| mount.count)
                .sum();
            let max = self
                .outfits
                .iter()
                .find(|record| record.id == outfit)
                .map_or(0, |record| record.max);
            let out = self
                .pilot
                .escorts
                .iter()
                .filter(|escort| escort.carried && Some(escort.ship) == bay.carried)
                .count();
            let held = outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, ammo)
                .saturating_add(u32::try_from(out).unwrap_or(u32::MAX));
            room.insert(
                outfit,
                capacity(bay.max_ammo, bays, max).saturating_sub(held),
            );
        }
        room
    }

    /// What the player's fighters met as it left systems since this was
    /// last taken, in order; taking it empties the list.
    pub fn take_fighter_notes(&mut self) -> Vec<FighterNote> {
        std::mem::take(&mut self.fighter_notes)
    }

    /// Puts the fighter of `sortie` back into its bay, a round again.
    fn put_back(&mut self, sortie: &Sortie) {
        match sortie.carrier {
            ShipRef::Player => OutfitRounds {
                owned: &mut self.pilot.outfits,
                sources: &self.ammo_outfits,
            }
            .stow(sortie.bay),
            ShipRef::Npc(id) => {
                if let Some(carrier) = self.npc_mut(id) {
                    carrier.rounds.stow(sortie.bay);
                }
            }
        }
    }
    /// This session with the fighters the player launches doing first as
    /// `source` says (see the module docs): the engine's by default.
    #[must_use]
    pub fn with_fighter_launch(mut self, source: RuleSource) -> Self {
        self.fighter_launch = source;
        self
    }

    /// What a fighter the player launches does first: by the engine, its
    /// class's standing order; otherwise it attacks the player's target.
    #[must_use]
    pub fn fighter_launch(&self) -> RuleSource {
        self.fighter_launch
    }

    /// This session with the player's fighters out, as it leaves the
    /// system, following `source` (see the module docs): the engine's by
    /// default.
    #[must_use]
    pub fn with_fighter_recall(mut self, source: RuleSource) -> Self {
        self.fighter_recall = source;
        self
    }

    /// What becomes of the player's fighters out as it leaves the system:
    /// by the engine, they follow a jump if they can and stay out while
    /// it is landed; otherwise they go back into their bays.
    #[must_use]
    pub fn fighter_recall(&self) -> RuleSource {
        self.fighter_recall
    }
}

/// A fighter of ship type `ship`, as `kind` gives it, at `state`,
/// launched by `carrier`: its shield and armour full, carrying its
/// class's stock weapons and rounds, its AI type its `InherentAI`, idle.
fn fighter(
    kind: &ShipKind,
    ship: crate::catalog::ShipId,
    state: ShipState,
    carrier: Carrier,
) -> Npc {
    Npc {
        id: NpcId::default(),
        ship,
        govt: None,
        ai_type: AiType::from_raw(kind.inherent_ai),
        leader: None,
        class: kind.escort_class,
        escort: None,
        stats: kind.stats,
        reserves: kind.stats.full(),
        state,
        mode: Mode::Flying,
        goal: Goal::Idle,
        condition: Condition::Intact,
        hull: kind.hull,
        armament: kind.armament.clone(),
        rounds: kind.rounds.clone(),
        trigger: Trigger::default(),
        target: None,
        provoked: 0.0,
        aggression: 0,
        inspected: None,
        booty: 0,
        boarded: false,
        info_types: 0,
        spared: false,
        assisting: 0,
        carrier: Some(carrier),
        person: None,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, Goal, NovaAi, Surroundings};
    use crate::bay::{Carrier, FighterNote, dock_window, launch_velocity};
    use crate::catalog::{
        HullRecord, OutfitId, OutfitRecord, ShipId, ShipRecord, StockWeapon, WeaponId, WeaponRecord,
    };
    use crate::chance::{Chance, NeverFires};
    use crate::combat::armament::MOD_AMMO;
    use crate::combat::hull::Condition;
    use crate::combat::{CombatEvent, Rules, ShipRef};
    use crate::escort::{Commanded, EscortClass, EscortCommand, EscortGroup, EscortOrder};
    use crate::flight::Controls;
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::Reserves;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, land_now, outfit, ship, weapon};
    use crate::traffic::npc::{AiType, Npc, NpcId};

    /// The Viper Bay: carrying ship 144, reloading every 60 ticks,
    /// pushing its fighter out at 4 pixels a tick, 4 fighters a bay.
    const BAY: WeaponId = WeaponId(149);
    /// The fighter, a Viper.
    const VIPER: ShipId = ShipId(144);
    /// The fighter outfit, the bay's rounds.
    const VIPERS: OutfitId = OutfitId(158);
    /// The fighter's gun: 10 pixels a tick for 30 ticks, every 5, 5
    /// energy and 2 mass damage.
    const GUN: WeaponId = WeaponId(141);
    /// A warship escort's ship type.
    const WARSHIP: ShipId = ShipId(150);

    /// The Viper's fields: as fast as the player, of `Maneuver` 80 and a
    /// jump's fuel.
    const VIPER_FIELDS: ShipFields = ShipFields {
        maneuver: 80,
        fuel: 300,
        ..FAST
    };

    fn bay() -> WeaponRecord {
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
    /// across, carries a Viper Bay; the Viper (a fighter, an interceptor
    /// by `InherentAI`, 20 across) carries a gun; and a warship escort
    /// type, 150.
    fn carrying() -> FakePilotCatalog {
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
            outfits: vec![OutfitRecord {
                mass: 0,
                max: 9999,
                ..outfit(158, &[(MOD_AMMO, 149)])
            }],
            ..catalog()
        }
    }

    /// A pilot owning `vipers` Vipers aboard, with `escorts`.
    fn pilot(catalog: &FakePilotCatalog, vipers: u16, escorts: Vec<Escort>) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.outfits.insert(VIPERS, vipers);
        pilot.escorts = escorts;
        pilot
    }

    /// `pilot`'s session by `launch`, its first traffic tick done.
    fn flying(catalog: &FakePilotCatalog, pilot: Pilot, launch: RuleSource) -> Session {
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_fighter_launch(launch);
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
        session
    }

    /// Holds the secondary trigger, on the bay, for a fight's tick.
    fn launch(session: &mut Session) {
        assert_eq!(session.secondary(), Some(BAY));
        session.hold_fire(false, true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.hold_fire(false, false);
    }

    /// The fleet's NPCs, in fleet order.
    fn fleet_npcs(session: &Session) -> Vec<Npc> {
        session
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
    fn sitting_ai() -> NovaAi {
        NovaAi::default()
            .with(AiType::Interceptor, Rc::new(Sitting))
            .with(AiType::WimpyTrader, Rc::new(Sitting))
    }

    /// Adds a pirate interceptor, a medium ship, at (`x`, `y`), idle.
    fn pirate(session: &mut Session, x: f32, y: f32) -> NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(152);
        npc.ai_type = AiType::Interceptor;
        npc.class = EscortClass::Medium;
        npc.state.position = Vec2::new(x, y);
        session.traffic.add_npc(npc)
    }

    /// A tick of the player at rest, then of the traffic by `ai`, then of
    /// the fight.
    fn fight(session: &mut Session, catalog: &FakePilotCatalog, ai: &NovaAi) {
        session.tick(Controls::default());
        session.tick_traffic(catalog, ai, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// Whether `ship`'s shield drops within `ticks` of [`fight`].
    fn shield_drops(
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
    fn firing_the_bay_launches_a_fighter_into_the_fleet_beside_the_player() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        session.take_save_due();
        session.take_combat_events();
        launch(&mut session);
        assert_eq!(session.pilot().owned(VIPERS), 1, "a fighter launched");
        assert_eq!(session.secondary_rounds(), Some(1));
        let fighters = fleet_npcs(&session);
        assert_eq!(fighters.len(), 1);
        let fighter = &fighters[0];
        assert_eq!(fighter.ship, VIPER);
        let record = session.pilot().escorts()[0];
        assert!(record.carried);
        assert_eq!(record.ship, VIPER);
        assert_eq!(record.order, None);
        assert_eq!(record.reserves, ShipStats::new(VIPER_FIELDS, &[]).full());
        assert_eq!(fighter.state.position, session.player().position);
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
        assert_eq!(fighter.reserves, ShipStats::new(VIPER_FIELDS, &[]).full());
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
            fighter.escort.map(|duty| (duty.slot, duty.ships)),
            Some((2, 2))
        );
        assert!(session.is_escort(fighter.id));
        assert!(
            session.take_combat_events().contains(&CombatEvent::Fired {
                ship: ShipRef::Player,
                weapon: BAY,
                at: session.player().position,
            }),
            "the bay's sound is heard"
        );
        assert!(!session.take_save_due(), "as firing ammunition");
        assert_eq!(session.traffic_ships(), [VIPER]);
    }

    #[test]
    fn by_the_engine_a_fighter_takes_no_command_and_attacks_once_commanded() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        let quarry = pirate(&mut session, 0.0, -300.0);
        session.select_target(TargetPick::Nearest);
        assert_eq!(session.target, Some(quarry));
        launch(&mut session);
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
                .command_escorts(EscortGroup::All, EscortCommand::Attack)
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
        };
        let mut session = Session::fly(&catalog, pilot(&catalog, 2, vec![warship]))
            .expect("flies")
            .with_escort_orders(RuleSource::Bible);
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        launch(&mut session);
        assert_eq!(session.pilot().escorts().len(), 2);
        let fighter = fleet_npcs(&session)[1].clone();
        let record = session.pilot().escorts()[1];
        assert_eq!(fighter.escort.and_then(|duty| duty.order), record.order);
        record.order
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
        assert_eq!(session.fighter_launch(), RuleSource::Bible);
        let quarry = pirate(&mut session, 0.0, -300.0);
        session.select_target(TargetPick::Nearest);
        launch(&mut session);
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
        launch(&mut session);
        assert_eq!(session.pilot().escorts()[0].order, None, "no target");
        assert_eq!(fleet_npcs(&session)[0].target, None);
        let first = session.fleet[0];
        session.target = first;
        for _ in 0..60 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        launch(&mut session);
        assert_eq!(session.pilot().escorts().len(), 2);
        assert_eq!(
            session.pilot().escorts()[1].order,
            None,
            "its own escort targeted"
        );
        assert_eq!(fleet_npcs(&session)[1].target, None);
    }

    #[test]
    fn with_no_room_in_the_system_no_fighter_launches_and_its_round_is_kept() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        for n in 0..63 {
            pirate(&mut session, 1000.0 + n as f32, 1000.0);
        }
        launch(&mut session);
        assert_eq!(session.pilot().owned(VIPERS), 2, "the round put back");
        assert_eq!(session.npcs().len(), 63, "no fighter");
        assert_eq!(session.pilot().escorts(), []);
        let mut session = flying(&catalog, pilot(&catalog, 2, Vec::new()), RuleSource::Engine);
        for n in 0..62 {
            pirate(&mut session, 1000.0 + n as f32, 1000.0);
        }
        launch(&mut session);
        assert_eq!(session.npcs().len(), 63, "the last slot taken");
    }

    // Return to Hangar and docking.

    /// A Viper out of its bay, full, with no standing command.
    fn out() -> Escort {
        Escort {
            ship: VIPER,
            reserves: ShipStats::new(VIPER_FIELDS, &[]).full(),
            order: None,
            carried: true,
            wage: None,
            person: None,
        }
    }

    /// A warship escort following `order`.
    fn warship(order: Option<EscortOrder>) -> Escort {
        Escort {
            ship: WARSHIP,
            reserves: Reserves::full(30.0, 45.0, 300.0),
            order,
            carried: false,
            wage: None,
            person: None,
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

    /// `escorts` flying with `vipers` aboard, the orders kept as they
    /// enter.
    fn fleet(catalog: &FakePilotCatalog, vipers: u16, escorts: Vec<Escort>) -> Session {
        let session = Session::fly(catalog, pilot(catalog, vipers, escorts))
            .expect("flies")
            .with_escort_orders(RuleSource::Bible);
        let mut session = session;
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
        session
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
            session.command_escorts(EscortGroup::All, EscortCommand::Dock),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Recall,
                targeted: false
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
            session.command_escorts(EscortGroup::All, EscortCommand::Dock),
            None,
            "nothing changed"
        );
        let mut session = fleet(&catalog, 0, vec![warship(None), out()]);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Dock),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Dock,
                targeted: false
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
                .command_escorts(fighters, EscortCommand::Dock)
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
    fn until_docked(session: &mut Session, catalog: &FakePilotCatalog, limit: u32) -> Option<u32> {
        let ai = NovaAi::default();
        (1..=limit).find(|_| {
            session.tick(Controls::default());
            session.tick_traffic(catalog, &ai, &mut NeverFires);
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
        for id in [session.fleet[0], session.fleet[2]].into_iter().flatten() {
            let npc = session.npc_mut(id).expect("out");
            npc.state.position = npc.state.position + Vec2::new(300.0, 0.0);
        }
        session.take_save_due();
        assert_eq!(session.secondary_rounds(), Some(0));
        session.command_escorts(EscortGroup::All, EscortCommand::Dock);
        let ticks = until_docked(&mut session, &catalog, 600);
        assert!(ticks.is_some(), "{:?}", fleet_npcs(&session));
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(session.npcs().len(), 1, "their NPCs are gone");
        assert_eq!(
            fleet_npcs(&session)[0]
                .escort
                .map(|duty| (duty.slot, duty.ships)),
            Some((2, 2)),
            "the rest re-form"
        );
        assert_eq!(session.pilot().owned(VIPERS), 2, "aboard again");
        assert_eq!(session.secondary_rounds(), Some(2));
        assert!(!session.take_save_due(), "as rounds are");
    }

    #[test]
    fn another_command_keeps_a_returning_fighter_out() {
        let catalog = carrying();
        let mut session = fleet(&catalog, 0, vec![out()]);
        let id = session.fleet[0].expect("placed");
        session.npc_mut(id).expect("out").state.position = Vec2::new(300.0, 0.0);
        session.command_escorts(EscortGroup::All, EscortCommand::Dock);
        session.command_escorts(EscortGroup::All, EscortCommand::Recall);
        assert_eq!(until_docked(&mut session, &catalog, 600), None);
        assert_eq!(session.pilot().escorts().len(), 1);
        assert_eq!(session.pilot().owned(VIPERS), 0);
    }

    #[test]
    fn a_fighter_no_bay_launches_is_lost_when_it_docks() {
        let mut catalog = carrying();
        catalog.hulls[0].weapons.clear();
        let mut session = fleet(&catalog, 0, vec![out()]);
        session.take_save_due();
        session.command_escorts(EscortGroup::All, EscortCommand::Dock);
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
            let id = session.fleet[0].expect("placed");
            session.npc_mut(id).expect("out").reserves.armor.now = armor;
            session.tick_combat(Rules::default(), &mut NeverFires);
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
            crate::ai::fire::trigger(npc, around)
        }

        fn target(&self, npc: &Npc, _around: &Surroundings) -> Option<ShipRef> {
            npc.goal.attacking()
        }
    }

    /// Nova's AI with every warship raiding and every interceptor
    /// sitting.
    fn raiding_ai() -> NovaAi {
        sitting_ai().with(AiType::Warship, Rc::new(Raiding))
    }

    const PIRATES: crate::catalog::GovtId = crate::catalog::GovtId(142);

    /// Adds a pirate carrier, a warship 60 across, 200 above the player
    /// and attacking it, escorting `lead`, carrying a Viper Bay with
    /// `vipers` fighters aboard.
    fn carrier(session: &mut Session, lead: NpcId, vipers: u32) -> NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(160);
        npc.govt = Some(PIRATES);
        npc.ai_type = AiType::Warship;
        npc.leader = Some(lead);
        npc.state.position = Vec2::new(0.0, -200.0);
        npc.goal = Goal::Attack(ShipRef::Player);
        npc.target = Some(ShipRef::Player);
        npc.armament = crate::combat::armament::Armament::new([(
            crate::combat::weapon::WeaponSpec::new(&bay()),
            1,
        )]);
        npc.rounds = std::collections::BTreeMap::from([(BAY, vipers)]);
        npc.hull.sprite_size = Some(60.0);
        session.traffic.add_npc(npc)
    }

    /// The Vipers in the system.
    fn vipers(session: &Session) -> Vec<Npc> {
        session
            .npcs()
            .iter()
            .filter(|npc| npc.ship == VIPER)
            .cloned()
            .collect()
    }

    /// The fighters `carrier` holds.
    fn aboard(session: &Session, carrier: NpcId) -> u32 {
        session.npc(carrier).map_or(0, |npc| npc.rounds[&BAY])
    }

    /// A session with a pirate carrier of 2 Vipers attacking the player,
    /// and the carrier's lead far off; the carrier and the lead.
    fn raided(catalog: &FakePilotCatalog) -> (Session, NpcId, NpcId) {
        let mut session = flying(catalog, pilot(catalog, 0, Vec::new()), RuleSource::Engine);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        let raider = carrier(&mut session, lead, 2);
        (session, raider, lead)
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
        assert_eq!(
            fighter.state.position,
            Vec2::new(0.0, -200.0),
            "at the carrier"
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
        assert_eq!(fighter.leader, Some(lead), "the carrier's lead");
        assert_eq!(fighter.target, Some(ShipRef::Player));
        assert_eq!(fighter.goal, Goal::Attack(ShipRef::Player));
        assert_eq!(fighter.escort, None);
        assert_eq!(aboard(&session, raider), 1);
        for _ in 0..59 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 1, "the bay reloads 60 ticks");
        fight(&mut session, &catalog, &ai);
        assert_eq!(vipers(&session).len(), 2);
        assert_eq!(aboard(&session, raider), 0);
        for _ in 0..240 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 2, "none more");
        assert!(
            session.reserves().shield.now < 30.0,
            "the fighters' shots: {:?}",
            session.reserves()
        );
        let carrier = session.npc(raider).expect("there");
        assert_eq!(carrier.reserves.shield.now, 30.0, "never hit by its own");
        assert!(
            vipers(&session)
                .iter()
                .all(|npc| npc.reserves.shield.now == 30.0)
        );
    }

    #[test]
    fn a_carrier_without_a_lead_leads_its_fighters_and_a_target_of_its_own_fleet_is_dropped() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 0, Vec::new()), RuleSource::Engine);
        let lead = pirate(&mut session, 2000.0, 2000.0);
        let raider = carrier(&mut session, lead, 2);
        let npc = session.npc_mut(raider).expect("there");
        npc.leader = None;
        npc.target = Some(ShipRef::Npc(lead));
        npc.goal = Goal::Attack(ShipRef::Npc(lead));
        // The lead is of its fleet only once it follows the carrier.
        session.npc_mut(lead).expect("there").leader = Some(raider);
        let ai = raiding_ai();
        fight(&mut session, &catalog, &ai);
        let fighter = &vipers(&session)[0];
        assert_eq!(fighter.leader, Some(raider));
        assert_eq!(fighter.target, None, "one of its own fleet");
        assert_eq!(fighter.goal, Goal::Idle);
    }

    #[test]
    fn once_its_carrier_stops_attacking_its_fighters_dock_and_it_holds_them_again() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let ai = raiding_ai();
        for _ in 0..61 {
            fight(&mut session, &catalog, &ai);
        }
        assert_eq!(vipers(&session).len(), 2);
        session.npc_mut(raider).expect("there").goal = Goal::Idle;
        let docked = (0..600).find(|_| {
            fight(&mut session, &catalog, &ai);
            vipers(&session).is_empty()
        });
        assert!(docked.is_some(), "{:?}", vipers(&session));
        assert_eq!(aboard(&session, raider), 2);
    }

    #[test]
    fn a_carriers_fighters_fly_by_their_own_ai_once_it_is_destroyed() {
        let catalog = carrying();
        let (mut session, raider, _) = raided(&catalog);
        let ai = raiding_ai();
        for _ in 0..61 {
            fight(&mut session, &catalog, &ai);
        }
        session.npc_mut(raider).expect("there").reserves.armor.now = 0.0;
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.npc(raider), None, "destroyed");
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

    /// [`carrying`], the player's ship carrying a Dart bay too.
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
        catalog
    }

    /// A Dart out of its bay.
    fn dart_out() -> Escort {
        Escort {
            ship: DART,
            ..out()
        }
    }

    /// A Viper and a Dart out, by `recall`, the Viper's shield at 10.
    fn both_out(catalog: &FakePilotCatalog, recall: RuleSource) -> Session {
        let mut session = fleet(catalog, 0, vec![out(), dart_out()]).with_fighter_recall(recall);
        let viper = session.fleet[0].expect("placed");
        session.npc_mut(viper).expect("out").reserves.shield.now = 10.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
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
        assert_eq!(
            viper.state.position,
            crate::escort::slot_position(session.player(), duty.slot, duty.ships, duty.spacing),
            "beside the player on its slot"
        );
        assert_eq!(viper.reserves.shield.now, 10.0, "as it was");
        assert_eq!(session.npcs().len(), 1, "no Dart");
        assert_eq!(session.pilot().owned(DARTS), 0, "no round back");
        assert_eq!(session.take_fighter_notes(), [FighterNote::Abandoned(1)]);
        assert_eq!(session.take_fighter_notes(), [], "taken");
        assert!(session.take_save_due());
    }

    #[test]
    fn by_the_engine_landed_the_fighters_stay_out_and_take_off_restocked() {
        let catalog = two_bays();
        let mut session = both_out(&catalog, RuleSource::Engine);
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().escorts().len(), 2, "still out");
        assert_eq!(session.pilot().owned(VIPERS), 0);
        session.take_off().expect("takes off");
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
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
        assert_eq!(session.fighter_recall(), RuleSource::Bible);
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
            .with_fighter_recall(RuleSource::Bible);
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
        assert_eq!(session.fleet.len(), 1);
    }

    #[test]
    fn a_control_bit_test_counts_a_fighter_out_as_its_outfit() {
        use crate::control::PilotFacts;
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 1, Vec::new()), RuleSource::Engine);
        assert!(session.facts().has_outfit(VIPERS), "aboard");
        launch(&mut session);
        assert_eq!(session.pilot().owned(VIPERS), 0);
        assert!(session.facts().has_outfit(VIPERS), "out of its bay");
        assert!(!session.facts().has_outfit(OutfitId(159)));
        // A carried escort of a type no bay launches docks into nothing.
        let stray = Escort {
            ship: WARSHIP,
            reserves: Reserves::full(10.0, 10.0, 10.0),
            order: None,
            carried: true,
            wage: None,
            person: None,
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![stray])).expect("flies");
        assert!(!session.facts().has_outfit(VIPERS));
        // Nor does an escort of the fighter's type that is not carried.
        let own = Escort {
            ship: VIPER,
            carried: false,
            ..stray
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![own])).expect("flies");
        assert!(!session.facts().has_outfit(VIPERS));
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
        };
        let session = Session::fly(&catalog, pilot(&catalog, 0, vec![fighter])).expect("flies");
        let facts = session.facts();
        assert!(facts.has_outfit(OutfitId(157)));
        assert!(!facts.has_outfit(VIPERS));
        assert!(!facts.has_outfit(OutfitId(160)));
    }

    #[test]
    fn a_fighter_out_survives_a_save_and_reload_and_docks_again() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Engine);
        launch(&mut session);
        let fighter = session.fleet[0].expect("out");
        session.npc_mut(fighter).expect("out").reserves.shield.now = 5.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        let saved = crate::save::encode(session.pilot());
        let pilot = crate::save::decode(&saved).expect("loads");
        assert_eq!(pilot.owned(VIPERS), 2);
        assert_eq!(pilot.escorts().len(), 1);
        assert!(pilot.escorts()[0].carried);
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        let npcs = fleet_npcs(&session);
        assert_eq!(npcs.len(), 1, "placed on the first traffic tick");
        assert_eq!(npcs[0].reserves.shield.now, 30.0, "restocked");
        assert_eq!(
            npcs[0].carrier.map(|carrier| carrier.ship),
            Some(ShipRef::Player)
        );
        session.command_escorts(EscortGroup::All, EscortCommand::Dock);
        assert!(until_docked(&mut session, &catalog, 600).is_some());
        assert_eq!(session.pilot().owned(VIPERS), 3);
    }

    #[test]
    fn by_the_other_reading_a_pilot_landed_is_saved_with_every_fighter_aboard() {
        let catalog = carrying();
        let mut session = flying(&catalog, pilot(&catalog, 3, Vec::new()), RuleSource::Engine)
            .with_fighter_recall(RuleSource::Bible);
        launch(&mut session);
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
        session.buy_ship(ShipId(129)).expect("bought");
        assert_eq!(session.pilot().escorts(), [warship(None)]);
        assert_eq!(session.pilot().owned(DARTS), 0, "no rounds");
        session.take_off().expect("takes off");
        session.tick_traffic(&catalog, &NovaAi::default(), &mut NeverFires);
        assert_eq!(
            fleet_npcs(&session)
                .iter()
                .map(|npc| npc.ship)
                .collect::<Vec<_>>(),
            [WARSHIP]
        );
    }

    #[test]
    fn a_pilot_resumed_docked_with_fighters_out_loses_them_buying_a_ship() {
        let catalog = spaceport();
        let mut pilot = pilot(&catalog, 0, vec![out(), warship(None), dart_out()]);
        pilot.cash = 1_000_000;
        pilot.stellar = Some(crate::catalog::StellarId(128));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        assert!(session.landed().is_some(), "resumed docked");
        assert!(session.fleet.is_empty(), "none placed yet");
        session.buy_ship(ShipId(129)).expect("bought");
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
            .outfitter()
            .expect("an outfitter")
            .check(crate::outfitter::OutfitOrder {
                outfit: VIPERS,
                direction: crate::market::Direction::Buy,
            })
    }

    #[test]
    fn the_outfitter_sells_fighters_only_while_the_bays_have_room_counting_those_out() {
        let catalog = spaceport();
        let full = Err(crate::outfitter::OutfitRefusal::MaxOwned);
        assert_eq!(viper_for_sale(&catalog, 3, 1), full, "MaxAmmo 4, one bay");
        assert_eq!(viper_for_sale(&catalog, 3, 0), Ok(()));
        assert_eq!(viper_for_sale(&catalog, 0, 4), full);
        let mut two = spaceport();
        two.hulls[0].weapons[0].count = 2;
        assert_eq!(viper_for_sale(&two, 7, 0), Ok(()), "two bays hold 8");
        assert_eq!(viper_for_sale(&two, 8, 0), full);
        let mut by_outfit = spaceport();
        by_outfit.weapons[0].max_ammo = 0;
        by_outfit.outfits[0].max = 5;
        assert_eq!(viper_for_sale(&by_outfit, 4, 0), Ok(()), "the outfit's Max");
        assert_eq!(viper_for_sale(&by_outfit, 4, 1), full);
        let mut no_bay = spaceport();
        no_bay.hulls[0].weapons.retain(|stock| stock.weapon != BAY);
        assert_eq!(viper_for_sale(&no_bay, 0, 0), full, "no bay for it");
        // Each bay's fighters are counted on their own.
        let mut session = fleet(&catalog, 0, vec![out(); 4]);
        session.pilot.cash = 1_000_000;
        land_now(&mut session).expect("lands");
        assert_eq!(
            session
                .outfitter()
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
    fn both_fighter_rules_follow_the_engine_by_default_and_either_is_chosen() {
        let session = Session::start(&catalog()).expect("starts");
        assert_eq!(session.fighter_launch(), RuleSource::Engine);
        assert_eq!(session.fighter_recall(), RuleSource::Engine);
        for source in RuleSource::ALL {
            let session = Session::start(&catalog())
                .expect("starts")
                .with_fighter_launch(source);
            assert_eq!(session.fighter_launch(), source);
            assert_eq!(session.fighter_recall(), RuleSource::Engine);
            let session = Session::start(&catalog())
                .expect("starts")
                .with_fighter_recall(source);
            assert_eq!(session.fighter_recall(), source);
            assert_eq!(session.fighter_launch(), RuleSource::Engine);
        }
    }
}
