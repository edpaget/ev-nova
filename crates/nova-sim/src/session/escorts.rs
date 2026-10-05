//! The player's escorts in flight: the session's side of
//! [`escort`](crate::escort).
//!
//! **The fleet in the system.** Each escort of the pilot's fleet flies as
//! an NPC beside the player ([`Npc::escort`]): whenever the player enters
//! a system (a jump's arrival, and the first traffic tick after a
//! take-off or after a pilot is flown, new or loaded), each escort whose
//! ship type has a record is placed on its formation slot (2, 3, … in
//! fleet order), with the player's heading and velocity, of no
//! government, its AI type its `InherentAI`, carrying its class's stock
//! weapons and rounds, with its record's shield, armour and fuel and its
//! standing order. Its record follows its reserves as it fights. Every
//! escort follows the player through a jump with no check of its own
//! (`_MakeEscortsHyperWithParent` @0x3fd54) and arrives with its shield
//! and armour as they were; taking off, and flying a pilot, restock them
//! full (`_RespawnEscort` @0x3d4b8, restock 1 @0x674c2), the fuel as
//! recorded. Landed, the escorts wait in the record.
//!
//! **Standing orders on entering a system.** Each escort's standing
//! order is kept on its record ([`Escort::order`](crate::Escort)) and
//! saved. On entering a system the session follows its
//! [`RuleKey::EscortOrders`](crate::RuleKey::EscortOrders) source
//! ([`Session::with_escort_orders`]): by the engine, every escort's order
//! is reset to formation, as the original's `_RespawnEscort` (@0x3d546)
//! and `_HandlePlayer` (@0x6e3bb-0x6e44b) do; by the other reading, the
//! orders are kept.
//!
//! **Commands** ([`Session::command_escorts`], `_IssueNewEscortCommand`
//! @0x660d5): a command goes to every escort of its group, each taking
//! the command's order, and Attack copies the player's target to each,
//! unless that is one of the player's own escorts. It reports what it
//! changed ([`Commanded`]) only when some escort's order or target
//! changed. The escort menu's rows ([`Session::escort_menu`]) give each
//! class's presence and its first escort's order.
//!
//! **Losing escorts** (`_DamageShip` @0x3af8f): an escort disabled or
//! destroyed leaves the fleet at once; a disabled one stays in the system
//! as the ship it is, its AI type its `InherentAI`, and is left behind.
//! A ship captured joins the fleet where it is, intact, its armour at
//! half its most; and the player's old ship, swapped for one captured,
//! joins beside the player. Released (see [`hail`](super::hail)), an
//! escort leaves the fleet and the system. Each change of the fleet makes
//! a save due.

use super::Session;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::escort::{
    self, ClassRow, Commanded, EscortClass, EscortCommand, EscortDuty, EscortGroup,
};
use crate::flight::ShipState;
use crate::hyperspace::JUMP_FUEL;
use crate::rulebook::RuleSource;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId};
use crate::traffic::table;

impl Session {
    /// This session with its escorts' standing orders reset, or kept, on
    /// entering a system as `source` says (see the module docs): the
    /// engine's reset by default.
    #[must_use]
    pub fn with_escort_orders(mut self, source: RuleSource) -> Self {
        self.escort_orders = source;
        self
    }

    /// Whether the escorts' standing orders are reset on entering a
    /// system ([`RuleSource::Engine`]) or kept.
    #[must_use]
    pub fn escort_orders(&self) -> RuleSource {
        self.escort_orders
    }

    /// The escorts enter the system (see the module docs): by the
    /// engine their standing orders are reset, and each is placed on its
    /// slot.
    pub(super) fn enter_escorts(&mut self) {
        if self.escort_orders == RuleSource::Engine {
            for escort in &mut self.pilot.escorts {
                escort.order = None;
            }
        }
        self.fleet = (0..self.pilot.escorts.len())
            .map(|index| self.place_escort(index))
            .collect();
        self.reform();
        for id in self.fleet.clone().into_iter().flatten() {
            self.snap(id);
        }
    }

    /// Escort `index` as an NPC in the system, beside the player until it
    /// is snapped to its slot; none when its ship type has no record.
    pub(super) fn place_escort(&mut self, index: usize) -> Option<NpcId> {
        let escort = *self.pilot.escorts.get(index)?;
        let record = self.ship_record(escort.ship)?;
        let kind = table::kind(record, &self.outfits, &self.arsenal);
        let npc = Npc {
            id: NpcId::default(),
            ship: escort.ship,
            govt: None,
            ai_type: AiType::from_raw(kind.inherent_ai),
            leader: None,
            class: kind.escort_class,
            escort: Some(EscortDuty {
                slot: 0,
                ships: 0,
                spacing: 0.0,
                order: escort.order,
            }),
            stats: kind.stats,
            reserves: escort.reserves,
            state: self.player,
            mode: Mode::Flying,
            goal: Goal::Formation { guard: None },
            condition: Condition::Intact,
            hull: kind.hull,
            armament: kind.armament,
            rounds: kind.rounds,
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
        };
        Some(self.traffic.add_npc(npc))
    }

    /// Gives each escort in the system its slot, 2, 3, … in fleet order,
    /// in a formation of them all and the player, spaced by the widest
    /// sprite among them ([`escort::spacing`]).
    pub(super) fn reform(&mut self) {
        let placed: Vec<NpcId> = self.fleet.iter().flatten().copied().collect();
        let ships = u8::try_from(placed.len() + 1).unwrap_or(u8::MAX);
        let widths: Vec<Option<f32>> = placed
            .iter()
            .filter_map(|&id| self.npc(id))
            .map(|npc| npc.hull.sprite_size)
            .collect();
        let spacing = escort::spacing(self.hull.sprite_size, &widths);
        for (place, id) in placed.into_iter().enumerate() {
            let slot = u8::try_from(place + 2).unwrap_or(u8::MAX);
            if let Some(npc) = self.npc_mut(id)
                && let Some(duty) = &mut npc.escort
            {
                *duty = EscortDuty {
                    slot,
                    ships,
                    spacing,
                    ..*duty
                };
            }
        }
    }

    /// Puts escort NPC `id` on its slot beside the player, with the
    /// player's heading and velocity.
    pub(super) fn snap(&mut self, id: NpcId) {
        let player = self.player;
        if let Some(npc) = self.npc_mut(id)
            && let Some(duty) = npc.escort
        {
            npc.state = ShipState {
                position: escort::slot_position(&player, duty.slot, duty.ships, duty.spacing),
                ..player
            };
        }
    }

    /// NPC `id`, if it is in the system.
    fn npc(&self, id: NpcId) -> Option<&Npc> {
        self.traffic.npcs().iter().find(|npc| npc.id == id)
    }

    /// NPC `id`, if it is in the system, to change.
    fn npc_mut(&mut self, id: NpcId) -> Option<&mut Npc> {
        self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id)
    }

    /// Each escort's shield, armour and fuel, full as its class holds
    /// them, the fuel as recorded no more than it holds.
    pub(super) fn restock_fleet(&mut self) {
        for index in 0..self.pilot.escorts.len() {
            let ship = self.pilot.escorts[index].ship;
            let Some(record) = self.ship_record(ship) else {
                continue;
            };
            let full = table::kind(record, &self.outfits, &self.arsenal)
                .stats
                .full();
            let reserves = &mut self.pilot.escorts[index].reserves;
            reserves.shield = full.shield;
            reserves.armor = full.armor;
            reserves.fuel.max = full.fuel.max;
            reserves.fuel.now = reserves.fuel.now.min(full.fuel.max);
        }
    }

    /// Each escort's record takes its NPC's shield, armour and fuel.
    pub(super) fn sync_fleet(&mut self) {
        for index in 0..self.pilot.escorts.len() {
            let Some(id) = self.fleet.get(index).copied().flatten() else {
                continue;
            };
            if let Some(reserves) = self.npc(id).map(|npc| npc.reserves) {
                self.pilot.escorts[index].reserves = reserves;
            }
        }
    }

    /// Every escort disabled, breaking up or gone leaves the fleet: its
    /// record goes, and its NPC is no longer an escort. A save is due when
    /// any did.
    pub(super) fn lose_escorts(&mut self) {
        let lost: Vec<NpcId> = self
            .fleet
            .iter()
            .flatten()
            .copied()
            .filter(|&id| {
                self.npc(id)
                    .is_none_or(|npc| npc.condition != Condition::Intact)
            })
            .collect();
        for id in lost {
            self.dismiss(id);
        }
    }

    /// Escort NPC `id` leaves the fleet: its record goes, it is no longer
    /// an escort, and the rest re-form. A save is due.
    pub(super) fn dismiss(&mut self, id: NpcId) {
        let Some(index) = self.fleet.iter().position(|&placed| placed == Some(id)) else {
            return;
        };
        self.fleet.remove(index);
        if index < self.pilot.escorts.len() {
            self.pilot.escorts.remove(index);
        }
        if let Some(npc) = self.npc_mut(id) {
            npc.escort = None;
        }
        self.reform();
        self.save_due = true;
    }

    /// Escort NPC `id` is released (see [`hail`](super::hail)): it leaves
    /// the fleet and the system (`_AIMakeShipLeave`), jumping out with a
    /// jump's fuel, or else deciding as its idle block does.
    pub(super) fn release(&mut self, id: NpcId) {
        if !self.is_escort(id) {
            return;
        }
        self.dismiss(id);
        if let Some(npc) = self.npc_mut(id) {
            npc.goal = if npc.reserves.fuel.now >= JUMP_FUEL {
                Goal::JumpOut
            } else {
                Goal::Idle
            };
            npc.target = None;
            npc.trigger = Trigger::default();
            npc.provoked = 0.0;
        }
    }

    /// Whether NPC `id` is one of the player's escorts.
    #[must_use]
    pub fn is_escort(&self, id: NpcId) -> bool {
        self.fleet.contains(&Some(id))
    }

    /// The escort class of the fleet's escort `index`: its NPC's, or its
    /// ship type's by its record.
    fn escort_class(&self, index: usize) -> EscortClass {
        if let Some(npc) = self
            .fleet
            .get(index)
            .copied()
            .flatten()
            .and_then(|id| self.npc(id))
        {
            return npc.class;
        }
        self.pilot
            .escorts
            .get(index)
            .and_then(|escort| self.ship_record(escort.ship))
            .map_or_else(EscortClass::default, |record| {
                EscortClass::of(record.escort_type, record.inherent_ai, record.fields.mass)
            })
    }

    /// Gives every escort of `group` `command` (see the module docs), and
    /// what it changed: none when it changed no escort's order or
    /// target, when the player has no escort of `group`, and while landed
    /// or jumping.
    pub fn command_escorts(
        &mut self,
        group: EscortGroup,
        command: EscortCommand,
    ) -> Option<Commanded> {
        if self.landed.is_some() || self.jumping.is_some() {
            return None;
        }
        let order = command.order();
        let target = self
            .target
            .filter(|&target| command == EscortCommand::Attack && !self.is_escort(target));
        let mut changed = false;
        for index in 0..self.pilot.escorts.len() {
            if !group.holds(self.escort_class(index)) {
                continue;
            }
            let escort = &mut self.pilot.escorts[index];
            changed |= escort.order != order;
            escort.order = order;
            let placed = self.fleet.get(index).copied().flatten();
            let Some(npc) = placed.and_then(|id| self.npc_mut(id)) else {
                continue;
            };
            if let Some(duty) = &mut npc.escort {
                duty.order = order;
            }
            if let Some(target) = target.map(ShipRef::Npc)
                && npc.target != Some(target)
            {
                npc.target = Some(target);
                changed = true;
            }
        }
        changed.then_some(Commanded {
            group,
            command,
            targeted: target.is_some(),
        })
    }

    /// The escort menu's class rows, in its order: whether the player has
    /// an escort of each class, and its first escort's standing order.
    #[must_use]
    pub fn escort_menu(&self) -> [ClassRow; 4] {
        EscortClass::ALL.map(|class| {
            let first = (0..self.pilot.escorts.len())
                .find(|&index| self.escort_class(index) == class)
                .and_then(|index| self.pilot.escorts.get(index));
            ClassRow {
                class,
                present: first.is_some(),
                order: first.and_then(|escort| escort.order),
            }
        })
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, NovaAi, Peaceful, Surroundings};
    use crate::catalog::{
        HullRecord, ShipId, ShipRecord, StockWeapon, SystemId, WeaponId, WeaponRecord,
    };
    use crate::chance::{Chance, NeverFires};
    use crate::combat::Rules;
    use crate::escort::EscortOrder;
    use crate::flight::{Controls, Turn};
    use crate::geometry::Vec2;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::{Gauge, Reserves};
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{FAST, FakePilotCatalog, catalog, hull, jump, ship, weapon};

    /// A pilot whose fleet is two escorts, both defending the player.
    fn defended(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            Escort {
                ship: ShipId(128),
                reserves: Reserves::full(30.0, 45.0, 300.0),
                order: Some(EscortOrder::Defend),
            };
            2
        ];
        pilot
    }

    fn orders(session: &Session) -> Vec<Option<EscortOrder>> {
        session
            .pilot()
            .escorts()
            .iter()
            .map(|escort| escort.order)
            .collect()
    }

    fn defend_all(session: &mut Session) {
        for escort in &mut session.pilot.escorts {
            escort.order = Some(EscortOrder::Defend);
        }
    }

    fn tick(session: &mut Session, catalog: &FakePilotCatalog) {
        session.tick_traffic(catalog, &Peaceful, &mut NeverFires);
    }

    /// What each escort's order is after each system entry, `source`
    /// choosing: a jump's arrival, a landing and take-off, and the pilot
    /// saved, loaded and flown again; each time the save holding the
    /// order given before.
    fn after_each_entry(source: RuleSource) -> [Vec<Option<EscortOrder>>; 3] {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, defended(&catalog))
            .expect("flies")
            .with_escort_orders(source);
        assert_eq!(session.escort_orders(), source);
        tick(&mut session, &catalog);
        defend_all(&mut session);
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let jumped = orders(&session);

        let mut session = Session::fly(&catalog, defended(&catalog))
            .expect("flies")
            .with_escort_orders(source);
        tick(&mut session, &catalog);
        defend_all(&mut session);
        session.land().expect("lands on 128, under the ship");
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        session.take_off().expect("takes off");
        tick(&mut session, &catalog);
        let took_off = orders(&session);

        let saved = crate::save::encode(&defended(&catalog));
        assert!(saved.contains("\"defend\""));
        let pilot = crate::save::decode(&saved).expect("loads");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_escort_orders(source);
        tick(&mut session, &catalog);
        let loaded = orders(&session);
        [jumped, took_off, loaded]
    }

    #[test]
    fn by_the_engine_entering_a_system_resets_every_escort_to_formation() {
        let none = vec![None, None];
        assert_eq!(
            after_each_entry(RuleSource::Engine),
            [none.clone(), none.clone(), none]
        );
        assert_eq!(
            Session::start(&catalog()).expect("starts").escort_orders(),
            RuleSource::Engine,
            "by default"
        );
    }

    #[test]
    fn by_the_other_reading_entering_a_system_keeps_the_standing_orders() {
        let defend = vec![Some(EscortOrder::Defend); 2];
        assert_eq!(
            after_each_entry(RuleSource::Bible),
            [defend.clone(), defend.clone(), defend]
        );
    }

    #[test]
    fn the_orders_stand_until_the_next_entry() {
        let catalog = catalog();
        let mut session = Session::fly(&catalog, defended(&catalog)).expect("flies");
        tick(&mut session, &catalog);
        defend_all(&mut session);
        tick(&mut session, &catalog);
        assert_eq!(orders(&session), [Some(EscortOrder::Defend); 2]);
    }

    // The fleet in flight.

    /// The warship escort's ship type: `EscortType` 2, `InherentAI` 3.
    const WARSHIP: ShipId = ShipId(150);
    /// The freighter escort's ship type: `EscortType` 3, `InherentAI` 1.
    const FREIGHTER: ShipId = ShipId(151);
    /// The turret the warship carries: 10 pixels a tick for 30 ticks,
    /// every 5, 5 energy damage.
    const TURRET: WeaponId = WeaponId(140);

    /// [`catalog`] (systems 130 and 131 linked, planet 128 under the
    /// ship) with the escort ship types, in neither system's traffic: the
    /// player's ship 128 is 40 pixels across, the warship 50 and the
    /// freighter 30, all flying as fast as the player.
    fn fleeted() -> FakePilotCatalog {
        FakePilotCatalog {
            ship_records: vec![
                ship(128, FAST),
                ShipRecord {
                    escort_type: 2,
                    inherent_ai: 3,
                    ..ship(150, FAST)
                },
                ShipRecord {
                    escort_type: 3,
                    inherent_ai: 1,
                    ..ship(151, FAST)
                },
            ],
            weapons: vec![WeaponRecord {
                guidance: 4,
                reload: 5,
                speed: 1000,
                count: 30,
                energy_dmg: 5,
                mass_dmg: 2,
                ..weapon(140)
            }],
            hulls: vec![
                HullRecord {
                    size: Some(40),
                    ..hull(128)
                },
                HullRecord {
                    size: Some(50),
                    weapons: vec![StockWeapon {
                        weapon: TURRET,
                        count: 1,
                        ammo: 0,
                    }],
                    ..hull(150)
                },
                HullRecord {
                    size: Some(30),
                    ..hull(151)
                },
            ],
            ..catalog()
        }
    }

    /// An escort of `ship` holding `reserves` and following `order`.
    fn escort(ship: ShipId, reserves: Reserves, order: Option<EscortOrder>) -> Escort {
        Escort {
            ship,
            reserves,
            order,
        }
    }

    /// Worn reserves: 10 of 30 shield, 20 of 45 armour, 120 of 300 fuel.
    fn worn() -> Reserves {
        Reserves {
            shield: Gauge {
                now: 10.0,
                max: 30.0,
            },
            armor: Gauge {
                now: 20.0,
                max: 45.0,
            },
            fuel: Gauge {
                now: 120.0,
                max: 300.0,
            },
        }
    }

    /// A pilot whose fleet is a worn warship, defending, then a worn
    /// freighter.
    fn escorted(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            escort(WARSHIP, worn(), Some(EscortOrder::Defend)),
            escort(FREIGHTER, worn(), None),
        ];
        pilot
    }

    /// [`escorted`]'s session by `source`, its first traffic tick done.
    fn flying(catalog: &FakePilotCatalog, source: RuleSource) -> Session {
        let mut session = Session::fly(catalog, escorted(catalog))
            .expect("flies")
            .with_escort_orders(source);
        tick(&mut session, catalog);
        session
    }

    /// The escorts' NPCs, in fleet order.
    fn escort_npcs(session: &Session) -> Vec<&Npc> {
        session
            .fleet
            .iter()
            .flatten()
            .map(|&id| session.npc(id).expect("in the system"))
            .collect()
    }

    /// Where escort `npc` should be: its slot beside the player.
    fn slot_of(session: &Session, npc: &Npc) -> Vec2 {
        let duty = npc.escort.expect("an escort");
        escort::slot_position(session.player(), duty.slot, duty.ships, duty.spacing)
    }

    #[test]
    fn the_first_traffic_tick_places_each_escort_on_its_slot_beside_the_player() {
        let catalog = fleeted();
        let session = flying(&catalog, RuleSource::Bible);
        let escorts = escort_npcs(&session);
        assert_eq!(escorts.len(), 2);
        assert_eq!(session.npcs().len(), 2, "no traffic but them");
        let spacing = escort::spacing(Some(40.0), &[Some(50.0), Some(30.0)]);
        assert_eq!(spacing, 30.0);
        for (npc, (slot, ship, ai_type, class, order)) in escorts.iter().zip([
            (
                2,
                WARSHIP,
                AiType::Warship,
                EscortClass::Warship,
                Some(EscortOrder::Defend),
            ),
            (
                3,
                FREIGHTER,
                AiType::WimpyTrader,
                EscortClass::Freighter,
                None,
            ),
        ]) {
            assert_eq!(npc.ship, ship);
            assert_eq!(
                npc.escort,
                Some(EscortDuty {
                    slot,
                    ships: 3,
                    spacing,
                    order
                })
            );
            assert_eq!(
                npc.state,
                ShipState {
                    position: escort::slot_position(session.player(), slot, 3, spacing),
                    ..*session.player()
                }
            );
            assert_eq!(npc.govt, None);
            assert_eq!(npc.ai_type, ai_type);
            assert_eq!(npc.class, class);
            assert_eq!(npc.goal, Goal::Formation { guard: None });
            assert_eq!(npc.condition, Condition::Intact);
            assert_eq!(
                npc.reserves,
                Reserves {
                    fuel: worn().fuel,
                    ..Reserves::full(30.0, 45.0, 300.0)
                },
                "restocked, the fuel as recorded"
            );
            assert_eq!(npc.stats, ShipStats::new(FAST, &[]));
        }
        let warship = escorts[0];
        assert_eq!(
            warship
                .armament
                .mounts()
                .iter()
                .map(|mount| (mount.spec.id, mount.count))
                .collect::<Vec<_>>(),
            [(TURRET, 1)],
            "its class's stock weapons"
        );
        assert_eq!(warship.hull.sprite_size, Some(50.0));
        assert_eq!(session.traffic_ships(), [WARSHIP, FREIGHTER]);
        assert_eq!(
            session.pilot().escorts()[0].reserves,
            warship.reserves,
            "the record restocked too"
        );
    }

    #[test]
    fn an_escort_whose_ship_type_has_no_record_is_kept_but_not_placed() {
        let catalog = fleeted();
        let mut pilot = escorted(&catalog);
        pilot.escorts.insert(0, escort(ShipId(999), worn(), None));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        tick(&mut session, &catalog);
        assert_eq!(session.pilot().escorts().len(), 3);
        assert_eq!(session.fleet.len(), 3);
        assert_eq!(session.fleet[0], None);
        let placed = escort_npcs(&session);
        assert_eq!(
            placed
                .iter()
                .map(|npc| npc.escort.map(|duty| (duty.slot, duty.ships)))
                .collect::<Vec<_>>(),
            [Some((2, 3)), Some((3, 3))]
        );
        assert_eq!(session.pilot().escorts()[0].reserves, worn(), "as it was");
    }

    /// A tick of the player under `controls`, then of Nova's traffic, then
    /// of the fight.
    fn fly(session: &mut Session, catalog: &FakePilotCatalog, controls: Controls) {
        session.tick(controls);
        session.tick_traffic(catalog, &NovaAi::default(), &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    /// Thrust, turning right on every third tick of each ten.
    fn weaving(tick: u32) -> Controls {
        Controls {
            thrust: true,
            turn: if tick % 10 < 3 {
                Turn::Right
            } else {
                Turn::None
            },
            reverse: false,
        }
    }

    /// The farthest, on either axis, any escort strays from its slot over
    /// `ticks` of [`weaving`], after the first 60.
    fn straying(session: &mut Session, catalog: &FakePilotCatalog, ticks: u32) -> f32 {
        let mut worst = 0.0_f32;
        for tick in 0..ticks {
            fly(session, catalog, weaving(tick));
            if tick < 60 {
                continue;
            }
            for npc in escort_npcs(session) {
                let off = slot_of(session, npc) - npc.state.position;
                worst = worst.max(off.x.abs()).max(off.y.abs());
            }
        }
        worst
    }

    #[test]
    fn escorts_follow_the_player_through_a_jump_on_their_slots_with_their_shield() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.fleet[0].expect("placed");
        session.npc_mut(warship).expect("there").reserves.shield.now = 4.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            session.pilot().escorts()[0].reserves.shield.now,
            4.0,
            "kept on the record"
        );
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let escorts = escort_npcs(&session);
        assert_eq!(escorts.len(), 2, "both came along");
        for npc in &escorts {
            assert_eq!(npc.state.position, slot_of(&session, npc), "on its slot");
            assert_eq!(npc.state.velocity, session.player().velocity);
            assert_eq!(npc.state.heading, session.player().heading);
        }
        assert_eq!(
            escorts[0].reserves.shield.now, 4.0,
            "no restock after a jump"
        );
        assert_eq!(escorts[1].reserves.shield.now, 30.0);
        let worst = straying(&mut session, &catalog, 300);
        assert!(worst <= escort::KEEP_FORMATION, "{worst}");
    }

    #[test]
    fn escorts_keep_formation_through_a_second_jump() {
        let mut catalog = fleeted();
        catalog.systems.push(SystemId(132));
        let mut session = flying(&catalog, RuleSource::Engine);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert!(straying(&mut session, &catalog, 100) <= escort::KEEP_FORMATION);
        assert_eq!(jump(&mut session, &catalog, 132), Some(SystemId(132)));
        let escorts = escort_npcs(&session);
        assert_eq!(escorts.len(), 2);
        for npc in &escorts {
            assert_eq!(npc.state.position, slot_of(&session, npc));
        }
        assert!(straying(&mut session, &catalog, 300) <= escort::KEEP_FORMATION);
    }

    #[test]
    fn landed_the_record_keeps_the_reserves_and_taking_off_restocks_them() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.fleet[0].expect("placed");
        session.npc_mut(warship).expect("there").reserves.shield.now = 4.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.land().expect("lands");
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        tick(&mut session, &catalog);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        session.take_off().expect("takes off");
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 30.0);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            session.pilot().escorts()[0].reserves.shield.now,
            30.0,
            "the old system's escort no longer counts"
        );
        tick(&mut session, &catalog);
        assert_eq!(escort_npcs(&session)[0].reserves.shield.now, 30.0);
        assert_eq!(escort_npcs(&session).len(), 2);
    }

    #[test]
    fn a_pilot_reloaded_flies_its_escorts_restocked_with_their_orders() {
        let catalog = fleeted();
        let saved = crate::save::encode(&escorted(&catalog));
        let pilot = crate::save::decode(&saved).expect("loads");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_escort_orders(RuleSource::Bible);
        assert_eq!(
            session.pilot().escorts()[1].reserves.armor,
            Gauge {
                now: 45.0,
                max: 45.0
            },
            "restocked as it is flown"
        );
        tick(&mut session, &catalog);
        let escorts = escort_npcs(&session);
        assert_eq!(
            escorts
                .iter()
                .map(|npc| npc.escort.and_then(|duty| duty.order))
                .collect::<Vec<_>>(),
            [Some(EscortOrder::Defend), None]
        );
        assert!(escorts.iter().all(|npc| npc.reserves.shield.now == 30.0));
    }

    // Losses.

    #[test]
    fn an_escort_disabled_leaves_the_fleet_at_once_and_is_left_behind() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Bible);
        session.take_save_due();
        let warship = session.fleet[0].expect("placed");
        session.npc_mut(warship).expect("there").reserves.armor.now = 10.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(
            session
                .pilot()
                .escorts()
                .iter()
                .map(|escort| escort.ship)
                .collect::<Vec<_>>(),
            [FREIGHTER],
            "its record is gone"
        );
        let lost = session.npc(warship).expect("still in the system");
        assert_eq!(lost.condition, Condition::Disabled);
        assert_eq!(lost.escort, None);
        assert_eq!(lost.ai_type, AiType::Warship, "its InherentAI");
        assert!(!session.is_escort(warship));
        assert!(session.take_save_due());
        let freighter = escort_npcs(&session)[0];
        assert_eq!(
            freighter.escort.map(|duty| (duty.slot, duty.ships)),
            Some((2, 2)),
            "the rest re-form"
        );
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(
            session
                .npcs()
                .iter()
                .map(|npc| npc.ship)
                .collect::<Vec<_>>(),
            [FREIGHTER],
            "it stayed behind"
        );
    }

    #[test]
    fn an_escort_destroyed_leaves_the_fleet() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.take_save_due();
        let freighter = session.fleet[1].expect("placed");
        session
            .npc_mut(freighter)
            .expect("there")
            .reserves
            .armor
            .now = 0.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts().len(), 1);
        assert!(session.take_save_due());
        for _ in 0..3 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.npc(freighter), None, "destroyed and gone");
        assert_eq!(session.pilot().escorts()[0].ship, WARSHIP);
        assert!(!session.take_save_due(), "only once");
    }

    // Commands.

    /// Thinks nothing and does nothing: a ship that stays put.
    #[derive(Debug)]
    struct Sitting;

    impl Behaviour for Sitting {
        fn decide(&self, npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
            npc.goal
        }
    }

    /// Nova's AI, but every interceptor sits as it is.
    fn sitting_ai() -> NovaAi {
        NovaAi::default().with(AiType::Interceptor, Rc::new(Sitting))
    }

    /// Adds a pirate interceptor at (`x`, `y`) with `goal`.
    fn pirate(session: &mut Session, x: f32, y: f32, goal: Goal) -> NpcId {
        let mut npc = crate::testkit::npc(0, ShipStats::new(FAST, &[]));
        npc.ship = ShipId(152);
        npc.ai_type = AiType::Interceptor;
        npc.class = EscortClass::Medium;
        npc.state.position = Vec2::new(x, y);
        npc.goal = goal;
        session.traffic.add_npc(npc)
    }

    /// A tick of the traffic, by `ai`, then of the fight.
    fn fight(session: &mut Session, catalog: &FakePilotCatalog, ai: &NovaAi) {
        session.tick(Controls::default());
        session.tick_traffic(catalog, ai, &mut NeverFires);
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    #[test]
    fn attack_sends_every_escort_at_the_players_target() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let quarry = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        assert_eq!(
            session.select_target(TargetPick::Nearest),
            Some(quarry),
            "not an escort"
        );
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Attack,
                targeted: true
            })
        );
        for npc in escort_npcs(&session) {
            assert_eq!(npc.target, Some(ShipRef::Npc(quarry)));
            assert_eq!(
                npc.escort.and_then(|duty| duty.order),
                Some(EscortOrder::Attack)
            );
        }
        assert!(
            session
                .pilot()
                .escorts()
                .iter()
                .all(|escort| escort.order == Some(EscortOrder::Attack))
        );
        let ai = sitting_ai();
        fight(&mut session, &catalog, &ai);
        assert_eq!(
            escort_npcs(&session)[0].goal,
            Goal::Attack(ShipRef::Npc(quarry))
        );
        let mut hit = false;
        for _ in 0..300 {
            fight(&mut session, &catalog, &ai);
            hit |= session
                .npc(quarry)
                .is_none_or(|npc| npc.reserves.shield.now < 30.0);
        }
        assert!(hit, "its shield dropped");
    }

    #[test]
    fn attack_with_no_target_or_an_escort_targeted_copies_none() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Attack,
                targeted: false
            })
        );
        let mut session = flying(&catalog, RuleSource::Engine);
        let freighter = session.fleet[1].expect("placed");
        session.target = Some(freighter);
        let commanded = session.command_escorts(EscortGroup::All, EscortCommand::Attack);
        assert_eq!(commanded.map(|commanded| commanded.targeted), Some(false));
        assert!(escort_npcs(&session).iter().all(|npc| npc.target.is_none()));
    }

    #[test]
    fn attack_again_with_a_new_target_retargets() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let first = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        let second = pirate(&mut session, 0.0, 400.0, Goal::Idle);
        session.target = Some(first);
        session.command_escorts(EscortGroup::All, EscortCommand::Attack);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            None,
            "nothing changed"
        );
        session.target = Some(second);
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack)
                .is_some()
        );
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.target == Some(ShipRef::Npc(second)))
        );
    }

    #[test]
    fn hold_and_recall_go_to_a_class_or_the_whole_fleet() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warships = EscortGroup::Class(EscortClass::Warship);
        assert_eq!(
            session.command_escorts(warships, EscortCommand::Hold),
            Some(Commanded {
                group: warships,
                command: EscortCommand::Hold,
                targeted: false
            })
        );
        let orders = |session: &Session| {
            escort_npcs(session)
                .iter()
                .map(|npc| npc.escort.and_then(|duty| duty.order))
                .collect::<Vec<_>>()
        };
        assert_eq!(orders(&session), [Some(EscortOrder::Hold), None]);
        assert_eq!(
            session
                .pilot()
                .escorts()
                .iter()
                .map(|e| e.order)
                .collect::<Vec<_>>(),
            [Some(EscortOrder::Hold), None]
        );
        let held = escort_npcs(&session)[0].state.position;
        let ai = NovaAi::default();
        for tick in 0..200 {
            fly(&mut session, &catalog, weaving(tick));
        }
        let warship = escort_npcs(&session)[0];
        assert_eq!(warship.goal, Goal::Idle);
        assert!(
            (warship.state.position - held).length() < 1.0,
            "it stayed put"
        );
        let freighter = escort_npcs(&session)[1];
        let off = slot_of(&session, freighter) - freighter.state.position;
        assert!(off.x.abs() <= 300.0 && off.y.abs() <= 300.0, "{off:?}");
        assert!(
            (session.player().position - held).length() > 500.0,
            "the player went on"
        );
        assert!(
            session
                .command_escorts(warships, EscortCommand::Recall)
                .is_some()
        );
        assert_eq!(orders(&session), [None, None]);
        fight(&mut session, &catalog, &ai);
        assert!(matches!(
            escort_npcs(&session)[0].goal,
            Goal::Formation { .. }
        ));
        session.command_escorts(EscortGroup::All, EscortCommand::Hold);
        assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
        fight(&mut session, &catalog, &ai);
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.goal == Goal::Idle)
        );
        session.command_escorts(EscortGroup::All, EscortCommand::Recall);
        assert_eq!(orders(&session), [None, None]);
    }

    #[test]
    fn a_command_that_changes_nothing_reports_nothing() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Recall),
            None,
            "no orders to recall from"
        );
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Hold)
                .is_some()
        );
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold),
            None
        );
        let fighters = EscortGroup::Class(EscortClass::Fighter);
        let before = session.pilot().clone();
        assert_eq!(
            session.command_escorts(fighters, EscortCommand::Defend),
            None
        );
        assert_eq!(session.pilot(), &before, "no fighters: nothing changes");
    }

    #[test]
    fn no_command_is_given_while_landed() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.land().expect("lands");
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold),
            None
        );
        assert!(
            session
                .pilot()
                .escorts()
                .iter()
                .all(|escort| escort.order.is_none())
        );
    }

    #[test]
    fn the_menu_rows_give_each_class_and_its_first_escorts_order() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Bible);
        assert_eq!(
            session.escort_menu(),
            [
                ClassRow {
                    class: EscortClass::Fighter,
                    present: false,
                    order: None
                },
                ClassRow {
                    class: EscortClass::Medium,
                    present: false,
                    order: None
                },
                ClassRow {
                    class: EscortClass::Warship,
                    present: true,
                    order: Some(EscortOrder::Defend)
                },
                ClassRow {
                    class: EscortClass::Freighter,
                    present: true,
                    order: None
                },
            ]
        );
        session.command_escorts(
            EscortGroup::Class(EscortClass::Freighter),
            EscortCommand::Hold,
        );
        assert_eq!(session.escort_menu()[3].order, Some(EscortOrder::Hold));
        let landed = Session::fly(&catalog, escorted(&catalog)).expect("flies");
        assert_eq!(
            landed.escort_menu().map(|row| row.present),
            [false, false, true, true],
            "by their records before they are placed"
        );
    }

    // No standing command.

    #[test]
    fn an_escort_with_no_command_guards_the_player_from_formation_with_its_turret() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.command_escorts(EscortGroup::All, EscortCommand::Recall);
        let raider = pirate(&mut session, 0.0, -200.0, Goal::Attack(ShipRef::Player));
        let ai = sitting_ai();
        fight(&mut session, &catalog, &ai);
        let warship = escort_npcs(&session)[0];
        assert_eq!(
            warship.goal,
            Goal::Formation {
                guard: Some(ShipRef::Npc(raider))
            }
        );
        assert_eq!(warship.target, Some(ShipRef::Npc(raider)));
        assert!(warship.trigger.turrets_only);
        for _ in 0..90 {
            fight(&mut session, &catalog, &ai);
            let warship = escort_npcs(&session)[0];
            assert!(matches!(warship.goal, Goal::Formation { .. }));
            let off = slot_of(&session, warship) - warship.state.position;
            assert!(off.length() < 20.0, "in formation: {off:?}");
        }
        let shield = session.npc(raider).expect("there").reserves.shield.now;
        assert!(shield < 30.0, "{shield}");
    }

    #[test]
    fn by_the_bible_a_warship_escort_with_no_command_attacks_the_raider() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let raider = pirate(&mut session, 0.0, -200.0, Goal::Attack(ShipRef::Player));
        let rulebook = crate::rulebook::Rulebook::default()
            .with_override(crate::rulebook::RuleKey::EscortAi, RuleSource::Bible);
        let ai = NovaAi::from_rulebook(&rulebook).with(AiType::Interceptor, Rc::new(Sitting));
        fight(&mut session, &catalog, &ai);
        assert_eq!(
            escort_npcs(&session)[0].goal,
            Goal::Attack(ShipRef::Npc(raider))
        );
        assert!(matches!(
            escort_npcs(&session)[1].goal,
            Goal::Formation { guard: None }
        ));
    }

    #[test]
    fn target_select_skips_the_escorts_and_escort_select_walks_them_alone() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let quarry = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        let (warship, freighter) = (session.fleet[0], session.fleet[1]);
        assert_eq!(session.select_target(TargetPick::Next), Some(quarry));
        assert_eq!(session.select_target(TargetPick::Next), None);
        assert_eq!(session.select_target(TargetPick::NearestThreat), None);
        assert_eq!(session.select_target(TargetPick::NextEscort), warship);
        assert_eq!(session.select_target(TargetPick::NextEscort), freighter);
        tick(&mut session, &catalog);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.target().map(|npc| npc.id), freighter, "kept");
        assert_eq!(session.select_target(TargetPick::NextEscort), None);
    }
}
