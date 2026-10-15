//! The player's escorts in flight: the session's side of
//! [`escort`](crate::escort).
//!
//! **The fleet in the system.** Each escort of the pilot's fleet flies as
//! an NPC beside the player ([`Npc::escort`]): whenever the player enters
//! a system (a jump's arrival, and the first traffic tick after a
//! take-off or after a pilot is flown, new or loaded), each escort whose
//! ship type has a record is placed on its formation slot (2, 3, … in
//! fleet order, the mission escorts after them in one formation, see
//! [`mission_escorts`](super::mission_escorts)), with the player's
//! heading and velocity, of no
//! government, its AI type its `InherentAI`, carrying its class's stock
//! weapons and rounds, with its record's shield, armour and fuel and its
//! standing order. Its record follows its reserves as it fights. Every
//! escort follows the player through a jump with no check of its own
//! (`_MakeEscortsHyperWithParent` @0x3fd54) and arrives with its shield
//! and armour as they were; taking off, and flying a pilot, restock them
//! full (`_RespawnEscort` @0x3d4b8, restock 1 @0x674c2), the fuel as
//! recorded. At a hypergate or wormhole exit each is placed on its slot
//! and then comes out of the exit gate after the player (see
//! [`gate_escorts`](super::gate_escorts)). Landed, the escorts wait in
//! the record. A trader escort's
//! `Holds` count in the fleet's cargo space, in flight and landed (see
//! [`market`](crate::market)).
//!
//! **Person escorts.** An escort that is a person
//! ([`Escort::person`](crate::Escort)) flies as itself in every system:
//! its person's fitted ship (its weapon slots and `ShieldMod`, see
//! [`person`](crate::person)), named by its record, with its grudge;
//! taking off and flying a pilot restock it full to that fitted ship as
//! it is placed. It says no hail quote. Destroyed without an escape pod,
//! its person is gone for good; disabled or released, it leaves the fleet
//! as the person it is. One whose person has no record flies as an
//! ordinary escort of its ship class.
//!
//! **Standing orders on entering a system.** Each escort's standing
//! order is kept on its record ([`Escort::order`](crate::Escort)) and
//! saved. On entering a system the session follows its
//! [`RuleKey::EscortOrders`](crate::RuleKey::EscortOrders) source
//! ([`Session::with_rules`]): by the engine, every escort's order
//! is reset to formation, as the original's `_RespawnEscort` (@0x3d546)
//! and `_HandlePlayer` (@0x6e3bb-0x6e44b) do; by the other reading, the
//! orders are kept.
//!
//! **Commands** ([`Session::command_escorts`], `_IssueNewEscortCommand`
//! @0x660d5): a command goes to every escort of its group, hired or a
//! mission escort (the original takes every ship led by the player whose
//! class matches, @0x6614a-0x66176), each taking the command's order,
//! and Attack copies the player's target to each, unless the player
//! leads it or it is led by a ship the player leads (`_AlliedWithPlayer`
//! @0x6985-0x69d0); its report says it attacks the target unless the
//! player leads it (@0x6653f-0x66559). Return to Hangar
//! ([`EscortCommand::Dock`]) sends the carried fighters home and any
//! other escort back to formation; its report says Recall when some such
//! escort had another order. It reports what it
//! changed ([`Commanded`]) only when some escort's order or target
//! changed, and then beeps ([`SimSound::EscortsCommanded`](crate::SimSound),
//! `_beepSnd[1]` @0x665be-0x665e1). A command is taken while a jump is
//! under way, the pre-jump stage included, and reported as under way, its
//! message saying what the escorts will do (`_IssueNewEscortCommand`
//! @0x66461-0x664e5; see [`EscortCommand::will`](crate::escort::EscortCommand::will)).
//! In hyperspace the escorts let go of their targets every tick
//! (`_MakeEscortsHyperWithParent` @0x3fdc0-0x3fdc2), so an Attack there
//! always copies the target afresh; on arrival an order given during the
//! jump is reset or kept as any other is. The escort menu's rows ([`Session::escort_menu`]) give each
//! class's presence and its first escort's order. A carried fighter is
//! placed as any escort is, its carrier the player (see
//! [`fighters`](super::fighters)).
//!
//! **Losing escorts** (`_DamageShip` @0x3af8f): an escort disabled or
//! destroyed leaves the fleet at once; a disabled one stays in the system
//! as the ship it is, its AI type its `InherentAI`, and is left behind.
//! One disabled or breaking up remembers the record it left with
//! ([`Npc::former`], the original's +0xc8de @0x3afab-0x3afce): repairing
//! itself, it rejoins as that fighter, hired or captured escort (see
//! [`self_repair`](super::self_repair)).
//! A trader escort lost (`InherentAI` 2 or less, not a carried fighter)
//! takes its share of the fleet's cargo as it goes: every good loses the
//! truncated part its `Holds` make of the fleet's holds without it (by
//! the other reading of
//! [`RuleKey::LostCargoShare`](crate::RuleKey::LostCargoShare), with
//! it), at most all of it, and the cargo is then trimmed to the holds
//! left by the reset its tail runs (`_DamageShip` @0x3b013,
//! `_DestroyPartialFleetCargo` @0xcd32-0xcff6; [`Session::lose_cargo_share`],
//! see the `precalced` module). A warship escort lost takes nothing and
//! runs no reset, and the mission cargo is never touched. A mission
//! escort breaking up takes its share too (see
//! [`mission_escorts`](super::mission_escorts)). A ship captured joins
//! the fleet where it is, intact, its armour at half its most; and the
//! player's old ship, swapped for one captured, joins beside the player.
//! Released (see [`hail`](super::hail)), an escort takes its share of the
//! cargo while still in the fleet, of any ship type by the engine reading
//! of [`RuleKey::ReleaseCargoShare`](crate::RuleKey::ReleaseCargoShare),
//! and the session resets; then it leaves the fleet and the system. Each
//! change of the fleet makes a save due.

use std::collections::BTreeSet;

use super::Session;
use crate::ai::Goal;
use crate::bay::{Carrier, dock_window};
use crate::catalog::ShipId;
use crate::cloak::Cloak;
use crate::combat::ShipRef;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::escort::{
    self, ClassRow, Commanded, EscortClass, EscortCommand, EscortDuty, EscortGroup, EscortOrder,
    FormationSlot,
};
use crate::flight::ShipState;
use crate::market;
use crate::pilot::Escort;
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource};
use crate::self_repair::FormerEscort;
use crate::traffic::npc::{AiType, Mode, Npc, NpcId, NpcPerson};
use crate::traffic::table::{self, ShipKind};

impl Session {
    /// The ship-table slot ([`Npc::berth`]) each of the fleet's escorts
    /// takes as the system is set up, by its index. As
    /// `_DeleteAllComputerShips` (@0x3bf5f) keeps the player's ships in
    /// the table across every setup, an escort whose NPC is in the system
    /// keeps its berth. Every other (a loaded pilot's, or one whose ship
    /// type has no record) takes the lowest berth not already chosen, in
    /// fleet order, before anything else is set up, as `_LoadPilotData`
    /// spawns a loaded pilot's escorts first (@0x759a4-0x75b2b). The old
    /// ship kept after a capture and a ship hired are placed at once, so
    /// they keep theirs.
    pub(super) fn fleet_berths(&self) -> Vec<u32> {
        let kept: Vec<Option<u32>> = (0..self.pilot.escorts.len())
            .map(|index| {
                let id = self.fleet.get(index).copied().flatten()?;
                self.npc(id).map(|npc| npc.berth)
            })
            .collect();
        let mut taken: BTreeSet<u32> = kept.iter().flatten().copied().collect();
        kept.into_iter()
            .map(|berth| {
                berth.unwrap_or_else(|| {
                    let free = (1..=u32::MAX)
                        .find(|b| !taken.contains(b))
                        .unwrap_or(u32::MAX);
                    taken.insert(free);
                    free
                })
            })
            .collect()
    }

    /// The escorts enter the system (see the module docs): by the
    /// engine their standing orders are reset, and each is placed on its
    /// slot, escort `i` at berth `berths[i]` ([`Session::fleet_berths`]).
    pub(super) fn enter_escorts(&mut self, berths: &[u32]) {
        if self.rules.source_for(RuleKey::EscortOrders) == RuleSource::Engine {
            for escort in &mut self.pilot.escorts {
                escort.order = None;
            }
            for npc in self.traffic.npcs_mut() {
                if let Some(mark) = &mut npc.mission {
                    mark.order = None;
                }
            }
            self.class_orders = [None; 4];
        }
        self.fleet = (0..self.pilot.escorts.len())
            .map(|index| self.place_escort_at(index, berths.get(index).copied()))
            .collect();
        self.traffic.reserve(BTreeSet::new());
        self.restock_persons = false;
        self.reform();
        for id in self.fleet.clone().into_iter().flatten() {
            self.snap(id);
        }
    }

    /// Escort `index` as an NPC in the system at the first free berth (see
    /// [`Session::place_escort_at`]).
    pub(super) fn place_escort(&mut self, index: usize) -> Option<NpcId> {
        self.place_escort_at(index, None)
    }

    /// Escort `index` as an NPC in the system, at `berth` or else the
    /// first free one, beside the player until it is snapped to its slot;
    /// none when its ship type has no record.
    pub(super) fn place_escort_at(&mut self, index: usize, berth: Option<u32>) -> Option<NpcId> {
        let mut escort = *self.pilot.escorts.get(index)?;
        let record = self.ship_record(escort.ship)?;
        let maneuver = record.fields.maneuver;
        let fitted = escort
            .person
            .and_then(|id| self.traffic.person(id))
            .map(|person| {
                let grudge = self.pilot.grudges.contains(&person.record.id);
                (person.kind.clone(), NpcPerson::of(&person.record, grudge))
            });
        let (kind, person) = match fitted {
            Some((kind, person)) => (kind, Some(person)),
            None => (table::kind(record, &self.outfits, &self.arsenal), None),
        };
        if self.restock_persons && person.is_some() {
            restock(&mut escort.reserves, &kind);
            self.pilot.escorts[index].reserves = escort.reserves;
        }
        let carrier = escort.carried.then(|| self.player_carrier(maneuver));
        let npc = Npc {
            id: NpcId::default(),
            berth: 0,
            ship: escort.ship,
            govt: None,
            ai_type: AiType::from_raw(kind.inherent_ai),
            leader: None,
            class: kind.escort_class,
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
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
            pick: None,
            target: None,
            provoked: 0.0,
            aggression: 0,
            inspected: None,
            booty: None,
            boarded: false,
            info_types: 0,
            spared: false,
            assisting: 0,
            carrier,
            person,
            mission: None,
            aux: None,
            cloak: Cloak::default(),
            cloak_gear: kind.cloak,
            led_out: false,
            self_repair: kind.self_repair,
            former: None,
            tractor: None,
        };
        Some(match berth {
            Some(berth) => self.traffic.add_npc_at(npc, berth),
            None => self.traffic.add_npc(npc),
        })
    }

    /// The player as the carrier of its fighter of `maneuver`.
    pub(super) fn player_carrier(&self, maneuver: i16) -> Carrier {
        Carrier {
            ship: ShipRef::Player,
            window: dock_window(maneuver),
            reach: self.reach_of(ShipRef::Player),
        }
    }

    /// NPC `id` joins the fleet where it is as the escort of `record`
    /// (`_AIMakeEscortFlyInForm` @0x89b8e, and a former escort's rejoin,
    /// see [`self_repair`](super::self_repair)): of no government, its AI
    /// type its ship type's `InherentAI`, led by the player alone,
    /// keeping formation by `record`'s standing order with no target of
    /// its own, carried by the player when `record` is a carried
    /// fighter, and no longer a former escort. Its record joins the
    /// pilot's, the formation re-forms, and a save is due.
    pub(super) fn enlist(&mut self, id: NpcId, record: Escort) {
        let ship = self
            .ship_record(record.ship)
            .map(|ship| (ship.inherent_ai, ship.fields.maneuver));
        let carrier = ship
            .filter(|_| record.carried)
            .map(|(_, maneuver)| self.player_carrier(maneuver));
        let Some(npc) = self.npc_mut(id) else {
            return;
        };
        npc.govt = None;
        if let Some((inherent_ai, _)) = ship {
            npc.ai_type = AiType::from_raw(inherent_ai);
        }
        npc.former = None;
        npc.leader = None;
        npc.goal = Goal::Formation { guard: None };
        npc.target = None;
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: record.order,
        });
        if carrier.is_some() {
            npc.carrier = carrier;
        }
        self.pilot.escorts.push(record);
        self.fleet.resize(self.pilot.escorts.len() - 1, None);
        self.fleet.push(Some(id));
        self.reform();
        self.save_due = true;
    }

    /// Gives each ship the player leads ([`Session::player_led`]) its slot
    /// in one formation of them all and the player, spaced by the widest
    /// sprite among them ([`escort::spacing`]): 2, 3, … in ship-table
    /// order, by berth, hired and mission escorts alike, as
    /// `_AICalcFormationPositions` numbers every ship whose formation
    /// parent is the player in table-slot order (@0x85137-0x851cc). That
    /// parent is the player for each ship it leads (`_DoPlayGameWork`
    /// @0x44160-0x441c0). The original recomputes the formation every
    /// frame (the parent's +0xc2 flag); the engine re-forms on each change
    /// to the led set, which gives the same slots at every frame.
    pub(super) fn reform(&mut self) {
        let placed: Vec<NpcId> = self.player_led().map(|npc| npc.id).collect();
        let ships = u8::try_from(placed.len() + 1).unwrap_or(u8::MAX);
        let widths: Vec<Option<f32>> = placed
            .iter()
            .filter_map(|&id| self.npc(id))
            .map(|npc| npc.hull.sprite_size)
            .collect();
        let spacing = escort::spacing(self.hull.sprite_size, &widths);
        for (place, id) in placed.into_iter().enumerate() {
            let slot = u8::try_from(place + 2).unwrap_or(u8::MAX);
            let Some(npc) = self.npc_mut(id) else {
                continue;
            };
            let formation = FormationSlot {
                slot,
                ships,
                spacing,
            };
            if let Some(duty) = &mut npc.escort {
                duty.formation = formation;
            } else if let Some(mark) = &mut npc.mission
                && mark.formation.is_some()
            {
                mark.formation = Some(formation);
            }
        }
    }

    /// Puts escort NPC `id`, hired or a mission escort, on its slot beside
    /// the player, with the player's heading and velocity.
    pub(super) fn snap(&mut self, id: NpcId) {
        let player = self.player;
        if let Some(npc) = self.npc_mut(id)
            && let Some(formation) = npc.formation()
        {
            npc.state = ShipState {
                position: formation.position(&player),
                ..player
            };
        }
    }

    /// NPC `id`, if it is in the system.
    pub(super) fn npc(&self, id: NpcId) -> Option<&Npc> {
        self.traffic.npcs().iter().find(|npc| npc.id == id)
    }

    /// NPC `id`, if it is in the system, to change.
    pub(crate) fn npc_mut(&mut self, id: NpcId) -> Option<&mut Npc> {
        self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id)
    }

    /// Each escort's shield, armour and fuel, full as its class holds
    /// them, the fuel as recorded no more than it holds.
    /// A person escort, whose fitted ship only the system's traffic knows,
    /// is restocked to it as it is placed next.
    pub(super) fn restock_fleet(&mut self) {
        for index in 0..self.pilot.escorts.len() {
            let ship = self.pilot.escorts[index].ship;
            let Some(record) = self.ship_record(ship) else {
                continue;
            };
            let kind = table::kind(record, &self.outfits, &self.arsenal);
            restock(&mut self.pilot.escorts[index].reserves, &kind);
        }
        self.restock_persons = true;
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
    /// any did. Each that is not a carried fighter and whose ship is still
    /// here, disabled, breaking up or destroyed, then takes its share of
    /// the fleet's cargo ([`Session::lose_cargo_share`]), as `_DamageShip`
    /// does after clearing its leader (@0x3afdd, call @0x3b013). One whose
    /// ship is simply gone takes nothing: the original reaches
    /// `_DestroyPartialFleetCargo` only from the disabled and dying paths.
    ///
    /// The original takes simultaneous losses in the order its hits land;
    /// here they are taken in fleet order, each after the ones before it
    /// have left the fleet's holds. No boundary the player can see is
    /// crossed within the tick.
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
            let index = self.fleet.iter().position(|&placed| placed == Some(id));
            let record = index
                .and_then(|index| self.pilot.escorts.get(index))
                .copied();
            let takes = record
                .filter(|escort| !escort.carried)
                .map(|escort| escort.ship);
            let here = self.npc(id).is_some();
            if let (Some(npc), Some(record)) = (self.npc_mut(id), record) {
                npc.former = Some(FormerEscort { record });
            }
            self.dismiss(id);
            if let Some(ship) = takes.filter(|_| here) {
                self.lose_cargo_share(ship);
            }
        }
    }

    /// A lost ship of class `ship` that the player led, already out of
    /// the fleet, takes its share of the fleet's cargo
    /// ([`Session::destroy_cargo_share`]), when its class's `InherentAI`
    /// is a trader's ([`FREIGHTER_AI`](escort::FREIGHTER_AI) or less), as
    /// each caller tests (@0x3affd, @0x350d1); a warship or interceptor
    /// takes nothing, and no reset runs. The ship's own `Holds` are not in
    /// the fleet's holds it takes its share of; by the other reading of
    /// [`RuleKey::LostCargoShare`] they count too.
    pub(super) fn lose_cargo_share(&mut self, ship: ShipId) {
        let Some(holds) = self.trader_holds(ship) else {
            return;
        };
        let counted = self.rules.source_for(RuleKey::LostCargoShare) == RuleSource::Bible;
        self.destroy_cargo_share(holds, counted);
    }

    /// The `Holds` of ship class `ship` when its `InherentAI` is a
    /// trader's ([`FREIGHTER_AI`](escort::FREIGHTER_AI) or less).
    pub(super) fn trader_holds(&self, ship: ShipId) -> Option<i16> {
        self.ship_record(ship)
            .filter(|record| record.inherent_ai <= escort::FREIGHTER_AI)
            .map(|record| record.fields.holds)
    }

    /// `_DestroyPartialFleetCargo` (@0xcd32-0xcff6) for a ship other than
    /// the player's, of `holds` `Holds`: every good, commodity or `jünk`,
    /// loses the truncated part [`market::lost_share`] gives, from those
    /// `Holds` over the fleet's holds as they stand, and `holds` once more
    /// when `counted`; then the session resets, as the call's tail does
    /// (@0xcff6; [`Session::reset_precalced`]), its trim cutting the
    /// commodities to the fleet's holds against everything carried, the
    /// mission cargo aboard included. The mission cargo is never touched.
    /// The tons destroyed go nowhere: the original writes them into the
    /// lost ship's own slots (@0xcf76), which nothing the engine models
    /// reads.
    pub(super) fn destroy_cargo_share(&mut self, holds: i16, counted: bool) {
        let share = market::lost_share(
            holds,
            self.stats.capacity,
            market::escort_tons(self.escort_holds()),
            counted,
        );
        market::destroy_share(&mut self.pilot.cargo, share);
        self.reset_precalced();
    }

    /// Escort NPC `id` leaves the fleet: its record goes, it is no longer
    /// an escort, and the rest re-form. A save is due.
    pub(super) fn dismiss(&mut self, id: NpcId) {
        if self.unfleet(id) {
            self.save_due = true;
        }
    }

    /// Escort NPC `id` leaves the fleet, as [`Session::dismiss`] says but
    /// making no save due; whether it was in the fleet.
    pub(super) fn unfleet(&mut self, id: NpcId) -> bool {
        let Some(index) = self.fleet.iter().position(|&placed| placed == Some(id)) else {
            return false;
        };
        self.fleet.remove(index);
        if self.pilot.escorts.get(index).is_some() {
            self.pilot.escorts.remove(index);
        }
        if let Some(npc) = self.npc_mut(id) {
            npc.escort = None;
        }
        self.reform();
        true
    }

    /// Escort NPC `id` is released (see [`hail`](super::hail)): it takes
    /// its share of the fleet's cargo while still in the fleet, its own
    /// `Holds` among the fleet's when it is a trader
    /// ([`Session::destroy_cargo_share`], `_DoCommDialog` @0x96864), any
    /// ship type by the engine reading of [`RuleKey::ReleaseCargoShare`]
    /// and only a trader by the other; then it leaves the fleet and the
    /// system (`_AIMakeShipLeave`), jumping out with a jump's fuel, or
    /// else deciding as its idle block does.
    pub(super) fn release(&mut self, id: NpcId) {
        if !self.is_escort(id) {
            return;
        }
        if let Some(holds) = self.released_holds(id) {
            self.destroy_cargo_share(holds, false);
        }
        self.dismiss(id);
        if let Some(npc) = self.npc_mut(id) {
            super::hail::make_leave(npc);
        }
    }

    /// The `Holds` escort NPC `id` takes its share of the cargo by as it
    /// is released, as [`RuleKey::ReleaseCargoShare`] reads: none when
    /// its ship class has no record, or by the other reading when it is
    /// not a trader.
    fn released_holds(&self, id: NpcId) -> Option<i16> {
        let index = self.fleet.iter().position(|&placed| placed == Some(id))?;
        let ship = self.pilot.escorts.get(index)?.ship;
        match self.rules.source_for(RuleKey::ReleaseCargoShare) {
            RuleSource::Engine => self.ship_record(ship).map(|record| record.fields.holds),
            RuleSource::Bible => self.trader_holds(ship),
        }
    }

    /// Whether NPC `id` is one of the player's escorts.
    #[must_use]
    pub fn is_escort(&self, id: NpcId) -> bool {
        self.fleet.contains(&Some(id))
    }

    /// The escort class of the fleet's escort `index`: its NPC's, or its
    /// ship type's by its record.
    pub(super) fn escort_class(&self, index: usize) -> EscortClass {
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
    /// target, when the player has no escort of `group`, and while
    /// landed. A command given while a jump is under way, the pre-jump
    /// stage included, reaches the escorts as any other does and is
    /// reported as under way ([`Commanded::underway`]), as the original
    /// takes the escort keys on every frame of a jump but its arrival
    /// (`_HandlePlayer` @0x6dd00-0x6e452, `_IssueNewEscortCommand`
    /// @0x66461).
    pub fn command_escorts(
        &mut self,
        group: EscortGroup,
        command: EscortCommand,
    ) -> Option<Commanded> {
        if self.landed.is_some() {
            return None;
        }
        // The original lets go of the escorts' targets earlier in every
        // hyperspace frame (@0x6bb0a, before the escort keys @0x6e192).
        // The flight does not tick the session while the stars streak,
        // so each command there lets go first, as that frame would have.
        if self.jumping().is_some() {
            self.let_go_of_led_targets();
        }
        // Reported, "attacking target.", for a target the player does not
        // lead (@0x6653f-0x66559); copied only when it is not allied with
        // the player either (@0x6625f).
        let reported = self.target.filter(|&target| {
            command == EscortCommand::Attack && !self.player_led().any(|npc| npc.id == target)
        });
        let target = reported
            .filter(|&target| !self.allied_with_player(target))
            .map(ShipRef::Npc);
        let mut changed = false;
        let mut recalled = false;
        for index in 0..self.pilot.escorts.len() {
            if !group.holds(self.escort_class(index)) {
                continue;
            }
            let escort = &mut self.pilot.escorts[index];
            let (order, recall) = order_for(command, escort.carried, escort.order);
            recalled |= recall;
            changed |= escort.order != order;
            escort.order = order;
            let placed = self.fleet.get(index).copied().flatten();
            let Some(npc) = placed.and_then(|id| self.npc_mut(id)) else {
                continue;
            };
            if let Some(duty) = &mut npc.escort {
                duty.order = order;
            }
            changed |= take_target(npc, target);
        }
        for (class, kept) in EscortClass::ALL.into_iter().zip(&mut self.class_orders) {
            if group.holds(class) {
                *kept = command.order();
            }
        }
        let mission_escorts: Vec<NpcId> = self
            .player_led()
            .filter(|npc| npc.mission_escort() && group.holds(npc.class))
            .map(|npc| npc.id)
            .collect();
        for id in mission_escorts {
            let Some(npc) = self.npc_mut(id) else {
                continue;
            };
            let Some(mark) = &mut npc.mission else {
                continue;
            };
            let (order, recall) = order_for(command, false, mark.order);
            recalled |= recall;
            changed |= mark.order != order;
            mark.order = order;
            changed |= take_target(npc, target);
        }
        if changed {
            self.sounds.push(crate::SimSound::EscortsCommanded);
        }
        changed.then_some(Commanded {
            group,
            command: if recalled {
                EscortCommand::Recall
            } else {
                command
            },
            targeted: reported.is_some(),
            underway: self.jump.is_some(),
        })
    }

    /// Whether NPC `id` is allied with the player, as `_AlliedWithPlayer`
    /// (@0x6985-0x69d0) has it: a ship the player leads, or one led by a
    /// ship the player leads, such as a fighter an escort launched.
    fn allied_with_player(&self, id: NpcId) -> bool {
        let led = |id| self.player_led().any(|npc| npc.id == id);
        led(id) || self.npc(id).and_then(|npc| npc.leader).is_some_and(led)
    }

    /// Lets go of every target of the ships the player leads, as
    /// `_MakeEscortsHyperWithParent` does on every frame of the
    /// hyperspace stage (@0x3fdc0-0x3fdc2): an Attack given in hyperspace
    /// then always copies the player's target afresh, and the escorts
    /// arrive holding none.
    pub(super) fn let_go_of_led_targets(&mut self) {
        let led: Vec<NpcId> = self.player_led().map(|npc| npc.id).collect();
        for id in led {
            if let Some(npc) = self.npc_mut(id) {
                npc.target = None;
            }
        }
    }

    /// `class`'s row in the escort menu: whether the player leads a ship
    /// of it, hired or a mission escort, and the standing order of its
    /// first hired escort, else of its first mission escort.
    fn class_row(&self, class: EscortClass) -> ClassRow {
        let hired = (0..self.pilot.escorts.len())
            .find(|&index| self.escort_class(index) == class)
            .map(|index| self.pilot.escorts[index].order);
        let first = hired.or_else(|| {
            self.player_led()
                .find(|npc| npc.mission_escort() && npc.class == class)
                .map(Npc::escort_order)
        });
        ClassRow {
            class,
            present: first.is_some(),
            order: first.flatten(),
        }
    }

    /// The standing order of `class` that a ship joining it takes, as
    /// the original's one order per class does (`_EscortAI`
    /// @0x83ad8-0x83b2b): its first escort's (`class_row`),
    /// else the last order given the class (kept across a jump or a
    /// landing unless the engine's [`RuleKey::EscortOrders`] reset it),
    /// Return to Hangar excepted. A fighter the player launches and a
    /// mission escort joining both take it.
    pub(super) fn class_order(&self, class: EscortClass) -> Option<EscortOrder> {
        let kept = EscortClass::ALL
            .iter()
            .position(|&each| each == class)
            .and_then(|index| self.class_orders[index]);
        let row = self.class_row(class);
        if row.present { row.order } else { kept }.filter(|&order| order != EscortOrder::Dock)
    }

    /// The escort menu's class rows, in its order: whether the player
    /// leads a ship of each class, hired or a mission escort, disabled
    /// included (`_DrawEscortMenuInFrame` @0x56392-0x563ca), and the
    /// standing order of its first escort (`class_row`).
    #[must_use]
    pub fn escort_menu(&self) -> [ClassRow; 4] {
        EscortClass::ALL.map(|class| self.class_row(class))
    }

    /// Whether the player has an escort flying, for the escort menu to
    /// stay open (`_HandlePlayer` @0x6df59-0x6dfd0): any hired escort, or
    /// a mission escort neither disabled nor breaking up (@0x6df73).
    #[must_use]
    pub fn escorts_flying(&self) -> bool {
        !self.pilot.escorts.is_empty()
            || self
                .player_led()
                .any(|npc| npc.mission_escort() && npc.condition == Condition::Intact)
    }
}

/// The order an escort holding `order`, carried or not, takes from
/// `command`, and whether that is a recall: Return to Hangar sends any
/// escort but a carried fighter back to formation (`_IssueNewEscortCommand`
/// @0x661a5-0x661ed, the AI type 5 test), a recall when it had another
/// order.
fn order_for(
    command: EscortCommand,
    carried: bool,
    order: Option<EscortOrder>,
) -> (Option<EscortOrder>, bool) {
    if command == EscortCommand::Dock && !carried {
        (None, order.is_some())
    } else {
        (command.order(), false)
    }
}

/// `npc` takes `target`, the player's to attack, when there is one;
/// whether its target changed (`_IssueNewEscortCommand` @0x66233-0x66273).
fn take_target(npc: &mut Npc, target: Option<ShipRef>) -> bool {
    match target {
        Some(target) if npc.target != Some(target) => {
            npc.target = Some(target);
            true
        }
        _ => false,
    }
}

/// `reserves` restocked full as a ship of `kind` holds them, the fuel as
/// recorded no more than it holds.
fn restock(reserves: &mut Reserves, kind: &ShipKind) {
    let full = kind.stats.full();
    reserves.shield = full.shield;
    reserves.armor = full.armor;
    reserves.fuel.max = full.fuel.max;
    reserves.fuel.now = reserves.fuel.now.min(full.fuel.max);
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use super::*;
    use crate::ai::{Behaviour, NovaAi, Peaceful, Surroundings};
    use crate::catalog::{
        HullRecord, JunkId, PersonId, ShipId, ShipRecord, StockWeapon, SystemId, WeaponId,
        WeaponRecord,
    };
    use crate::chance::{Chance, NeverFires};
    use crate::combat::Rules;
    use crate::escort::EscortOrder;
    use crate::flight::{Controls, Turn};
    use crate::geometry::Vec2;
    use crate::market::Good;
    use crate::pilot::{Escort, Pilot};
    use crate::reserves::{Gauge, Reserves};
    use crate::rulebook::Rulebook;
    use crate::self_repair::FormerKind;
    use crate::stats::ShipStats;
    use crate::targeting::TargetPick;
    use crate::testkit::{
        FAST, FakePilotCatalog, begin_jump_now, catalog, hull, jump, land_now, ship, weapon,
    };

    /// A pilot whose fleet is two escorts, both defending the player.
    fn defended(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            Escort {
                ship: ShipId(128),
                reserves: Reserves::full(30.0, 45.0, 300.0),
                order: Some(EscortOrder::Defend),
                carried: false,
                wage: None,
                person: None,
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
            .with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source));
        assert_eq!(session.rules().source_for(RuleKey::EscortOrders), source);
        tick(&mut session, &catalog);
        defend_all(&mut session);
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let jumped = orders(&session);

        let mut session = Session::fly(&catalog, defended(&catalog))
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source));
        tick(&mut session, &catalog);
        defend_all(&mut session);
        land_now(&mut session).expect("lands on 128, under the ship");
        assert!(crate::save::encode(session.pilot()).contains("\"defend\""));
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        tick(&mut session, &catalog);
        let took_off = orders(&session);

        let saved = crate::save::encode(&defended(&catalog));
        assert!(saved.contains("\"defend\""));
        let pilot = crate::save::decode(&saved).expect("loads");
        let mut session = Session::fly(&catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source));
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
            Session::start(&catalog())
                .expect("starts")
                .rules()
                .source_for(RuleKey::EscortOrders),
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
            carried: false,
            wage: None,
            person: None,
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
            .with_rules(Rulebook::default().with_override(RuleKey::EscortOrders, source));
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
        duty.formation.position(session.player())
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
                    formation: FormationSlot {
                        slot,
                        ships: 3,
                        spacing,
                    },
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
                .map(|npc| npc
                    .escort
                    .map(|duty| (duty.formation.slot, duty.formation.ships)))
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
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        tick(&mut session, &catalog);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
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
        let mut session = Session::fly(&catalog, pilot).expect("flies").with_rules(
            Rulebook::default().with_override(RuleKey::EscortOrders, RuleSource::Bible),
        );
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
            freighter
                .escort
                .map(|duty| (duty.formation.slot, duty.formation.ships)),
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

    // What a lost escort remembers.

    /// The warship hired at 50 a day, a captured freighter, and a
    /// carried warship fighter, each worn, flying by the engine.
    fn mixed(catalog: &FakePilotCatalog) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![
            Escort {
                wage: Some(50),
                ..escort(WARSHIP, worn(), Some(EscortOrder::Defend))
            },
            escort(FREIGHTER, worn(), None),
            Escort {
                carried: true,
                ..escort(WARSHIP, worn(), None)
            },
        ];
        let mut session = Session::fly(catalog, pilot).expect("flies");
        tick(&mut session, catalog);
        session
    }

    #[test]
    fn a_disabled_escort_remembers_what_it_was_with_its_record() {
        let catalog = fleeted();
        let mut session = mixed(&catalog);
        let records = session.pilot().escorts().to_vec();
        let ids: Vec<NpcId> = session.fleet.iter().flatten().copied().collect();
        assert_eq!(ids.len(), 3);
        for &id in &ids {
            assert_eq!(session.npc(id).expect("there").former, None);
            session.npc_mut(id).expect("there").reserves.armor.now = 10.0;
        }
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(session.pilot().escorts().is_empty(), "all lost");
        for ((id, record), kind) in ids.into_iter().zip(&records).zip([
            FormerKind::Hired,
            FormerKind::Captured,
            FormerKind::Fighter,
        ]) {
            let npc = session.npc(id).expect("left behind");
            assert_eq!(npc.condition, Condition::Disabled);
            let former = npc.former.expect("remembers");
            assert_eq!(former.kind(), kind);
            assert_eq!(
                former.record,
                Escort {
                    reserves: Reserves {
                        armor: Gauge {
                            now: 10.0,
                            max: 45.0,
                        },
                        ..record.reserves
                    },
                    ..*record
                },
                "its record as it was lost"
            );
        }
    }

    #[test]
    fn an_escort_breaking_up_remembers_too_and_one_gone_leaves_nothing() {
        let catalog = fleeted();
        let mut session = mixed(&catalog);
        let hired = session.fleet[0].expect("placed");
        let npc = session.npc_mut(hired).expect("there");
        npc.hull.death_delay = 10;
        npc.reserves.armor.now = 0.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        let npc = session.npc(hired).expect("breaking up");
        assert!(matches!(npc.condition, Condition::Dying { .. }));
        assert_eq!(
            npc.former.map(|former| former.kind()),
            Some(FormerKind::Hired)
        );
        let captured = session.fleet[0].expect("placed");
        session.traffic.remove(captured);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts().len(), 1, "the gone one is lost");
    }

    #[test]
    fn an_intact_escort_remembers_nothing() {
        let catalog = fleeted();
        let mut session = mixed(&catalog);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts().len(), 3);
        assert!(session.npcs().iter().all(|npc| npc.former.is_none()));
    }

    // The cargo a lost escort takes.

    /// Food, a commodity.
    const FOOD: Good = Good::Commodity(0);
    /// Opals, a `jünk`.
    const OPALS: Good = Good::Junk(JunkId(146));

    /// A session of `fleet` by `source` of `LostCargoShare`, its first
    /// traffic tick done, holding 9 food and 3 opals.
    fn laden(catalog: &FakePilotCatalog, fleet: Vec<Escort>, source: RuleSource) -> Session {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = fleet;
        let mut session = Session::fly(catalog, pilot)
            .expect("flies")
            .with_rules(Rulebook::default().with_override(RuleKey::LostCargoShare, source));
        tick(&mut session, catalog);
        session.pilot.cargo = BTreeMap::from([(FOOD, 9), (OPALS, 3)]);
        session
    }

    /// The warship and two freighters, all full.
    fn trader_fleet() -> Vec<Escort> {
        let full = Reserves::full(30.0, 45.0, 300.0);
        vec![
            escort(WARSHIP, full, None),
            escort(FREIGHTER, full, None),
            escort(FREIGHTER, full, None),
        ]
    }

    /// Escort `index`'s armour goes to `armor`, and the fight ticks.
    fn hit(session: &mut Session, index: usize, armor: f32) {
        let id = session.fleet[index].expect("placed");
        session.npc_mut(id).expect("there").reserves.armor.now = armor;
        session.tick_combat(Rules::default(), &mut NeverFires);
    }

    fn cargo(session: &Session) -> BTreeMap<Good, u32> {
        session.pilot.cargo.clone()
    }

    #[test]
    fn a_trader_escorts_death_destroys_its_share_of_the_cargo() {
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        hit(&mut session, 1, 0.0);
        assert_eq!(session.pilot().escorts().len(), 2, "it is lost");
        assert_eq!(
            cargo(&session),
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "20 of the player's 20 and the other freighter's 20"
        );
    }

    #[test]
    fn a_lone_trader_escort_lost_takes_all_the_cargo_by_the_engine_and_half_by_the_other_reading() {
        let catalog = fleeted();
        let fleet = escorted(&catalog).escorts;
        let mut session = laden(&catalog, fleet.clone(), RuleSource::Engine);
        hit(&mut session, 1, 0.0);
        assert_eq!(cargo(&session), BTreeMap::new(), "20 of the player's 20");
        let mut session = laden(&catalog, fleet, RuleSource::Bible);
        hit(&mut session, 1, 0.0);
        assert_eq!(
            cargo(&session),
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "20 of the player's 20 and its own 20"
        );
    }

    #[test]
    fn a_trader_escort_of_inherent_ai_2_lost_destroys_its_share_of_the_cargo() {
        let mut catalog = fleeted();
        catalog.ship_records[2].inherent_ai = 2;
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        hit(&mut session, 1, 0.0);
        assert_eq!(
            cargo(&session),
            BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
            "InherentAI 2 is a trader's"
        );
    }

    #[test]
    fn a_trader_escort_disabled_destroys_its_share_as_it_leaves() {
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        hit(&mut session, 1, 10.0);
        let lost = session
            .npcs()
            .iter()
            .find(|npc| npc.ship == FREIGHTER && npc.escort.is_none());
        assert_eq!(lost.map(|npc| npc.condition), Some(Condition::Disabled));
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 5), (OPALS, 2)]));
    }

    #[test]
    fn a_trader_escort_lost_takes_its_share_then_resets_and_a_warship_does_neither() {
        use crate::scan::Latches;
        use crate::session::precalced::tests::convicted;
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        // 9 food, 3 opals and 50 tons of mission cargo, past the fleet's
        // 60 tons of holds.
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(140, 50));
        convicted(&mut session);
        hit(&mut session, 0, 0.0);
        assert_ne!(session.scan_latches, Latches::ARMED, "a warship: no call");
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
        hit(&mut session, 0, 0.0);
        assert_eq!(session.scan_latches, Latches::ARMED, "@0xcff6");
        assert_eq!(
            cargo(&session),
            BTreeMap::from([(FOOD, 3), (OPALS, 2)]),
            "half of each good, then 5 x 40/57 as the reset trims"
        );
    }

    #[test]
    fn a_warship_escorts_death_destroys_no_cargo() {
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        hit(&mut session, 0, 0.0);
        assert_eq!(session.pilot().escorts().len(), 2, "it is lost");
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
    }

    #[test]
    fn a_carried_trader_fighter_lost_destroys_no_cargo() {
        let catalog = fleeted();
        let full = Reserves::full(30.0, 45.0, 300.0);
        let fleet = vec![
            Escort {
                carried: true,
                ..escort(FREIGHTER, full, None)
            },
            escort(FREIGHTER, full, None),
        ];
        let mut session = laden(&catalog, fleet, RuleSource::Engine);
        hit(&mut session, 0, 0.0);
        assert_eq!(session.pilot().escorts().len(), 1, "it is lost");
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
    }

    #[test]
    fn an_uncarried_trader_escort_whose_ship_is_gone_destroys_no_cargo() {
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        let id = session.fleet[1].expect("placed");
        session.traffic.remove(id);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts().len(), 2, "it is lost");
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
    }

    #[test]
    fn a_lost_escort_trims_the_cargo_to_the_fleets_holds() {
        let catalog = fleeted();
        let full = Reserves::full(30.0, 45.0, 300.0);
        let fleet = vec![escort(FREIGHTER, full, None), escort(FREIGHTER, full, None)];
        let mut session = laden(&catalog, fleet, RuleSource::Engine);
        session.pilot.cargo = BTreeMap::from([(FOOD, 10)]);
        session
            .pilot
            .missions
            .put(0, crate::testkit::carrying(130, 50));
        hit(&mut session, 0, 0.0);
        assert_eq!(
            cargo(&session),
            BTreeMap::from([(FOOD, 3)]),
            "half of 10 is 5, then 5 x 40 / 55"
        );
        assert_eq!(session.pilot().total_cargo(), 53, "the mission cargo stays");
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
                targeted: true,
                underway: false
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
                targeted: false,
                underway: false
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
                targeted: false,
                underway: false
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
    fn a_command_given_in_hyperspace_reaches_the_escorts_under_way() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Bible);
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session);
        begin_jump_now(&mut session).expect("jumps");
        session.take_sounds();
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Hold,
                targeted: false,
                underway: true
            })
        );
        assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.escort.and_then(|duty| duty.order) == Some(EscortOrder::Hold)),
            "the NPCs' orders"
        );
        assert_eq!(session.class_orders, [Some(EscortOrder::Hold); 4]);
        assert!(
            session
                .take_sounds()
                .contains(&crate::SimSound::EscortsCommanded)
        );
    }

    #[test]
    fn a_command_given_while_braking_for_a_jump_is_under_way() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Bible);
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session);
        session.begin_jump().expect("pre-jump");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)), "braking");
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Defend),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Defend,
                targeted: false,
                underway: true
            })
        );
        assert_eq!(orders(&session), [Some(EscortOrder::Defend); 2]);
    }

    #[test]
    fn an_order_given_in_hyperspace_is_reset_on_arrival_by_the_engine_and_kept_by_the_bible() {
        for (source, kept) in [
            (RuleSource::Engine, None),
            (RuleSource::Bible, Some(EscortOrder::Hold)),
        ] {
            let catalog = fleeted();
            let mut session = flying(&catalog, source);
            session.plot_course(SystemId(131)).expect("a route");
            crate::testkit::fly_out(&mut session);
            begin_jump_now(&mut session).expect("jumps");
            session.command_escorts(EscortGroup::All, EscortCommand::Hold);
            assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
            session.arrive(&catalog, &mut NeverFires).expect("arrives");
            tick(&mut session, &catalog);
            assert_eq!(orders(&session), [kept; 2], "{source:?}");
            assert!(
                escort_npcs(&session)
                    .iter()
                    .all(|npc| npc.escort.and_then(|duty| duty.order) == kept),
                "{source:?}"
            );
        }
    }

    /// A session by the Bible in hyperspace, the escorts having taken
    /// pirate `quarry` before the jump: the player's target.
    fn attacking_into_hyperspace(catalog: &FakePilotCatalog) -> (Session, NpcId) {
        let mut session = flying(catalog, RuleSource::Bible);
        let quarry = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        session.target = Some(quarry);
        assert_eq!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack)
                .map(|commanded| commanded.targeted),
            Some(true)
        );
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.target == Some(ShipRef::Npc(quarry)))
        );
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session);
        session.target = Some(quarry);
        begin_jump_now(&mut session).expect("jumps");
        (session, quarry)
    }

    #[test]
    fn attack_in_hyperspace_copies_the_target_onto_escorts_that_let_theirs_go() {
        let catalog = fleeted();
        let (mut session, quarry) = attacking_into_hyperspace(&catalog);
        assert!(
            escort_npcs(&session).iter().all(|npc| npc.target.is_none()),
            "let go as the jump begins"
        );
        let attacked = Some(Commanded {
            group: EscortGroup::All,
            command: EscortCommand::Attack,
            targeted: true,
            underway: true,
        });
        for streak in 0..2 {
            session.take_sounds();
            assert_eq!(
                session.command_escorts(EscortGroup::All, EscortCommand::Attack),
                attacked,
                "attack {streak}"
            );
            assert!(
                session
                    .take_sounds()
                    .contains(&crate::SimSound::EscortsCommanded),
                "attack {streak}"
            );
            assert!(
                escort_npcs(&session)
                    .iter()
                    .all(|npc| npc.target == Some(ShipRef::Npc(quarry))),
                "attack {streak}"
            );
            session.tick(Controls::default());
            assert!(
                escort_npcs(&session).iter().all(|npc| npc.target.is_none()),
                "let go every tick of the streak"
            );
        }
        session.command_escorts(EscortGroup::All, EscortCommand::Attack);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            attacked,
            "the streak's frames run no session tick: each command lets go first"
        );
        session.arrive(&catalog, &mut NeverFires).expect("arrives");
        assert!(
            escort_npcs(&session).iter().all(|npc| npc.target.is_none()),
            "none held on arrival"
        );
    }

    #[test]
    fn attack_does_not_copy_a_target_led_by_an_escort_but_reports_it() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let freighter = session.fleet[1].expect("placed");
        let launched = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        session.npc_mut(launched).expect("here").leader = Some(freighter);
        session.target = Some(launched);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Attack,
                targeted: true,
                underway: false
            }),
            "attacking target."
        );
        assert!(
            escort_npcs(&session).iter().all(|npc| npc.target.is_none()),
            "allied with the player: not copied"
        );
    }

    #[test]
    fn no_command_is_given_while_landed() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        land_now(&mut session).expect("lands");
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
        // Within the 164-pixel box of the escort's slot, where its turret
        // fires (@0x86666-0x86689).
        let raider = pirate(&mut session, 0.0, -120.0, Goal::Attack(ShipRef::Player));
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

    #[test]
    fn plain_r_picks_a_ship_attacking_a_hired_escort_but_never_the_escort() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.fleet[0].expect("a hired escort");
        let attacker = pirate(
            &mut session,
            0.0,
            -300.0,
            Goal::Attack(ShipRef::Npc(warship)),
        );
        let bystander = pirate(&mut session, 0.0, -100.0, Goal::Idle);
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(attacker)
        );
        let beside = session.player().position + Vec2::new(10.0, 0.0);
        let escort = session.npc_mut(warship).expect("in the system");
        escort.goal = Goal::Attack(ShipRef::Player);
        escort.state.position = beside;
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(attacker),
            "the escort is passed over"
        );
        session.npc_mut(attacker).expect("there").goal = Goal::Attack(ShipRef::Npc(bystander));
        session.target = None;
        assert_eq!(session.select_target(TargetPick::NearestThreat), None);
    }

    // Person escorts.

    /// [`fleeted`] with "Ace" (`përs` 700), who appears nowhere, flying the
    /// warship with two more turrets, its `ShieldMod` 200, its `HailPict`
    /// 7800 and its hail quote 1 (`STR#` 7101 "Hi!").
    fn with_ace() -> FakePilotCatalog {
        let mut catalog = fleeted();
        catalog.persons = vec![crate::catalog::PersonRecord {
            name: "Ace".to_owned(),
            link_syst: -2,
            shield_mod: 200,
            hail_pict: Some(7800),
            hail_quote: 1,
            weapons: vec![crate::catalog::PersonWeapon {
                weapon: TURRET,
                count: 2,
                ammo: 0,
            }],
            ..crate::testkit::person(700, WARSHIP.0)
        }];
        catalog.strings = vec![(7101, vec!["Hi!".to_owned()])];
        catalog
    }

    /// A pilot whose fleet is Ace, worn.
    fn with_ace_escort(catalog: &FakePilotCatalog) -> Pilot {
        let mut pilot = Pilot::new(catalog, "Ada").expect("starts");
        pilot.escorts = vec![Escort {
            person: Some(PersonId(700)),
            ..escort(WARSHIP, worn(), None)
        }];
        pilot
    }

    /// [`with_ace_escort`]'s session, its first traffic tick done.
    fn flying_ace(catalog: &FakePilotCatalog) -> Session {
        let mut session = Session::fly(catalog, with_ace_escort(catalog)).expect("flies");
        tick(&mut session, catalog);
        session
    }

    #[test]
    fn a_person_escort_flies_as_itself_with_its_fitted_ship() {
        let catalog = with_ace();
        let mut pilot = with_ace_escort(&catalog);
        pilot.grudges.insert(PersonId(700));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        tick(&mut session, &catalog);
        let ace = escort_npcs(&session)[0];
        let person = ace.person.expect("flown by Ace");
        assert_eq!(person.id, PersonId(700));
        assert_eq!(person.portrait, Some(7800));
        assert!(person.grudge, "its grudge kept");
        assert_eq!(session.npc_name(ace), Some("Ace"));
        assert_eq!(ace.stats.shield, 2.0 * f32::from(FAST.shield));
        assert_eq!(ace.stats.armor, 2.0 * f32::from(FAST.armor));
        assert_eq!(
            ace.armament
                .mounts()
                .iter()
                .map(|mount| (mount.spec.id, mount.count))
                .collect::<Vec<_>>(),
            [(TURRET, 3)],
            "its class's turret and its two more"
        );
        assert!(ace.escort.is_some());
        assert_eq!(ace.govt, None);
        assert_eq!(ace.ai_type, AiType::Warship);
    }

    #[test]
    fn a_person_escort_is_restocked_full_to_its_fitted_ship_on_flying_and_taking_off() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let fitted = Reserves {
            fuel: worn().fuel,
            ..Reserves::full(60.0, 90.0, 300.0)
        };
        assert_eq!(
            escort_npcs(&session)[0].reserves,
            fitted,
            "the fuel as recorded"
        );
        assert_eq!(
            session.pilot().escorts()[0].reserves,
            fitted,
            "and its record"
        );
        let ace = session.fleet[0].expect("placed");
        session.npc_mut(ace).expect("there").reserves.shield.now = 5.0;
        session.sync_fleet();
        land_now(&mut session).expect("lands on 128, under the ship");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        tick(&mut session, &catalog);
        assert_eq!(escort_npcs(&session)[0].reserves.shield.now, 60.0);
    }

    #[test]
    fn taking_off_places_the_escorts_restocked_among_the_new_ships_before_any_tick() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let before: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
        let ace = session.fleet[0].expect("placed");
        session.npc_mut(ace).expect("there").reserves.shield.now = 5.0;
        session.sync_fleet();
        land_now(&mut session).expect("lands on 128, under the ship");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(!session.fleet.is_empty());
        for id in &session.fleet {
            let id = id.expect("placed");
            assert!(!before.contains(&id), "{id:?} is a new ship");
            assert!(session.npc(id).is_some(), "{id:?} is in the system");
        }
        assert_eq!(
            escort_npcs(&session)[0].reserves.shield.now,
            60.0,
            "restocked before it is placed"
        );
        assert!(!session.restock_persons, "the restock consumed");
    }

    #[test]
    fn a_person_escort_keeps_its_reserves_through_a_jump() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let ace = session.fleet[0].expect("placed");
        session.npc_mut(ace).expect("there").reserves.shield.now = 5.0;
        session.sync_fleet();
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let ace = escort_npcs(&session)[0];
        assert_eq!(
            ace.reserves.shield,
            Gauge {
                now: 5.0,
                max: 60.0
            }
        );
        assert_eq!(ace.person.map(|person| person.id), Some(PersonId(700)));
        assert_eq!(session.npc_name(ace), Some("Ace"), "known there too");
    }

    #[test]
    fn an_escort_whose_person_has_no_record_flies_as_its_class() {
        let catalog = fleeted();
        let session = flying_ace(&catalog);
        let escort = escort_npcs(&session)[0];
        assert_eq!(escort.person, None);
        assert_eq!(escort.stats, ShipStats::new(FAST, &[]));
        assert_eq!(
            session.pilot().escorts()[0].person,
            Some(PersonId(700)),
            "its link kept"
        );
    }

    #[test]
    fn a_person_escort_says_no_hail_quote() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        session.tick_quotes(&catalog, &mut crate::testkit::Draws::of(&[0]));
        assert_eq!(session.take_quotes(), []);
        let ace = session.fleet[0].expect("placed");
        session.unfleet(ace);
        let mut chance = crate::testkit::Draws::of(&[0]);
        session.tick_quotes(&catalog, &mut chance);
        assert_eq!(
            chance.asked,
            [crate::person::QUOTE_ODDS],
            "out of the fleet"
        );
        assert_eq!(
            session
                .take_quotes()
                .into_iter()
                .map(|quote| quote.text)
                .collect::<Vec<_>>(),
            ["Hi!"]
        );
    }

    #[test]
    fn a_person_escort_destroyed_without_an_escape_pod_is_gone_for_good() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let ace = session.fleet[0].expect("placed");
        session.npc_mut(ace).expect("there").reserves.armor.now = 0.0;
        for _ in 0..4 {
            session.tick_combat(Rules::default(), &mut NeverFires);
        }
        assert_eq!(session.pilot().escorts(), []);
        assert!(session.pilot().gone(PersonId(700)));
    }

    #[test]
    fn a_person_escort_disabled_or_released_stays_its_person_alive() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let ace = session.fleet[0].expect("placed");
        session.npc_mut(ace).expect("there").reserves.armor.now = 10.0;
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(session.pilot().escorts(), []);
        let left = session.npc(ace).expect("left behind");
        assert_eq!(left.condition, Condition::Disabled);
        assert_eq!(left.person.map(|person| person.id), Some(PersonId(700)));
        assert!(!session.pilot().gone(PersonId(700)));
        let mut session = flying_ace(&catalog);
        let ace = session.fleet[0].expect("placed");
        session.release(ace);
        assert_eq!(session.pilot().escorts(), []);
        let left = session.npc(ace).expect("leaving");
        assert_eq!(left.person.map(|person| person.id), Some(PersonId(700)));
        assert!(!session.pilot().gone(PersonId(700)));
    }
}
