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
//! heading and velocity: by the engine first on a slot counted without
//! the mission escorts, then on a jump's arrival pushed back to glide in,
//! and after a take-off or a pilot flown moved onto its final slot (see
//! [`escort_arrival`](super::escort_arrival)). It is of no
//! government, its AI type its `InherentAI`, carrying its class's stock
//! weapons and rounds, with its record's shield, armour and fuel and its
//! standing order. Its record follows its reserves as it fights. Every
//! escort follows the player through a jump with no check of its own
//! (`_MakeEscortsHyperWithParent` @0x3fd54) and arrives with its shield
//! and armour as they were, the slowest of them setting the jump's days
//! and wages if it is slower than the player (see
//! [`trip_days`](super::trip_days)); taking off, and flying a pilot, restock them
//! full (`_RespawnEscort` @0x3d4b8, restock 1 @0x674c2), the fuel as
//! recorded. Each enters with no ionization: `_RespawnEscort` clears its
//! charge (+0x5c @0x3d543) and leaves its colour (+0xb0), but
//! `_PrepareAllSprites` runs the display before the first frame
//! (@0x3a783), which clears the colour of a ship with no charge
//! (@0x2bf8a), so no frame shows it and no dose ORs into it. Each keeps
//! its cloak, fade and direction, as `_RespawnEscort` writes neither
//! +0x64 nor +0xc8d8, so an escort cloaked or fading when the player
//! leaves arrives the same way, its fade then stepped by the entry's
//! `_PrepareAllSprites` calls before the first frame (once at a jump's
//! arrival, twice at a take-off or gate exit; see the `slot_visibility`
//! module), and decides from there on its first think. Through a jump's
//! warp it goes on thinking its cloak and stepping its fade, in goal
//! 0xb, to the warp's end. A ship captured into the fleet joins with
//! neither fade nor direction (`_DoPlunderDialog` @0x946e3, @0x946f9;
//! see [`Session::join_fleet`]); a person joining by hail keeps its
//! cloak. At a hypergate or wormhole exit each is placed on its slot
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
//! escort had another order. Under any other command a carried fighter
//! flying home lets go of its target, which counts as a change
//! (`_IssueNewEscortCommand` @0x6627a-0x662dc). It reports what it
//! changed ([`Commanded`]) only when some escort's order or target
//! changed, and then beeps ([`SimSound::EscortsCommanded`](crate::SimSound),
//! `_beepSnd[1]` @0x665be-0x665e1); when some escort that isn't mute
//! (`Flags2` 0x0010) had its order changed, one of the group, drawn in
//! berth order, acknowledges it in its class's voice (@0x665e6-0x6667a;
//! see [`voices`](super::voices)). A command is taken while a jump is
//! under way, the pre-jump stage included, and reported as under way, its
//! message saying what the escorts will do (`_IssueNewEscortCommand`
//! @0x66461-0x664e5; see [`EscortCommand::will`](crate::escort::EscortCommand::will)).
//! In hyperspace the escorts let go of their targets every tick
//! (`_MakeEscortsHyperWithParent` @0x3fdc0-0x3fdc2), so an Attack there
//! always copies the target afresh; on arrival an order given during the
//! jump is reset or kept as any other is. The escort menu's rows ([`Space::escort_rows`], the frame's
//! [`FrameHud::escort_rows`](crate::flight_frame::FrameHud::escort_rows)) give each
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
//! runs no reset, and the mission cargo is never touched. The tons of
//! each commodity it took become its own hold, overwriting what it held
//! (`_DestroyPartialFleetCargo` @0xcf76), and boarding it to repair it
//! gives them back (see [`board_repair`](super::board_repair)). A mission
//! escort breaking up takes its share too (see
//! [`mission_escorts`](super::mission_escorts)). A ship captured joins
//! the fleet where it is, intact, its armour at half its most; a
//! mission's special ship, its mark kept, joins as a mission escort, of
//! no record or fleet slot (see
//! [`captured_mission_ship`](super::captured_mission_ship)); and the
//! player's old ship, swapped for one captured, joins beside the player.
//! Released (see [`hail`](super::hail)), an escort takes its share of the
//! cargo while still in the fleet, of any ship type by the engine reading
//! of [`RuleKey::ReleaseCargoShare`](crate::RuleKey::ReleaseCargoShare),
//! and the session resets; then it leaves the fleet and the system. Each
//! change of the fleet makes a save due.

use std::collections::BTreeSet;

use super::{Session, Setup, Space, Statics};
use crate::ai::pursuit::Pursuit;
use crate::ai::swarm::{self, FormationParent};
use crate::ai::{Goal, Surroundings};
use crate::bay::{Carrier, dock_window};
use crate::catalog::ShipId;
use crate::chance::Chance;
use crate::cloak::Cloak;
use crate::combat::ShipRef;
use crate::combat::armament::Fold;
use crate::combat::armament::Trigger;
use crate::combat::hull::Condition;
use crate::combat::ion::Ion;
use crate::combat::ship_frames::ShipFrames;
use crate::escort::{
    self, ClassRow, Commanded, EscortClass, EscortCommand, EscortDuty, EscortGroup, EscortOrder,
    FormationSlot,
};
use crate::escort_voice::{Speaker, Voter, returning_fighter};
use crate::flight::ShipState;
use crate::market;
use crate::pilot::{Escort, Pilot};
use crate::reserves::Reserves;
use crate::rulebook::{RuleKey, RuleSource};
use crate::self_repair::FormerEscort;
use crate::skill::random_skill_level;
use crate::traffic::npc::{AiType, Mode, Npc, NpcHold, NpcId, NpcPerson};
use crate::traffic::ship_display::{AnimRoll, GLOW_SPAWN, NpcDisplay, Rolls};
use crate::traffic::table::ShipKind;

/// What an escort keeps from its old NPC when the fleet enters a system
/// ([`Space::fleet_kept`]).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct KeptEscort {
    /// Its aux mark ([`Npc::aux`]).
    pub(super) aux: Option<u8>,
    /// Its cloak, fade and direction ([`Npc::cloak`]), as `_RespawnEscort`
    /// leaves +0x64 and +0xc8d8.
    pub(super) cloak: Cloak,
    /// Its display state, fold and the anim rolls its spawn still owes,
    /// its ship-table slot's, which `_RespawnEscort` leaves (see the
    /// `ship_display` module); none when its NPC was not in the system,
    /// and it takes a loaded pilot's.
    pub(super) display: Option<(ShipFrames, Fold, AnimRoll)>,
    /// Its skill ([`Npc::skill`]), which `_RespawnEscort` leaves (it never
    /// writes +0x40, @0x3d4b8-0x3d62c); none when its NPC was not in the
    /// system, and it rolls one as `_SpawnEscort` does.
    pub(super) skill: Option<f32>,
    /// Its afterburner ([`Npc::afterburner`]), which `_RespawnEscort`
    /// never writes (@0x3d4b8-0x3d62c); `None` with no NPC, and the
    /// escort rolls it as it is placed (see the `afterburner_roll`
    /// module).
    pub(super) afterburner: Option<bool>,
    /// Its voice's variant ([`Npc::voice`]), which `_RespawnEscort` never
    /// writes (+0xc922, @0x3d4b8-0x3d62c); none with no NPC, or while it
    /// is unset, and it rolls one at its first think (see the `voices`
    /// module).
    pub(super) voice: Option<crate::voice::VoiceVariant>,
}

impl Session {
    /// The escorts enter the system (see the module docs): by the
    /// engine their standing orders are reset, and each is placed beside
    /// the player, for [`Session::place_entering_escorts`] to put on its
    /// slot, escort `i` at berth `berths[i]` ([`Space::fleet_berths`])
    /// with what it kept, `kept[i]` ([`Space::fleet_kept`]): its aux
    /// mark, its skill, its afterburner, its voice, its cloak, and its
    /// display and fold, with no ionization, or a loaded pilot's fold set
    /// with none kept
    /// ([`PlayerShip::loaded_escort_set`](super::PlayerShip::loaded_escort_set)).
    /// One that kept no skill rolls one on `chance`, and one that kept no afterburner, having no NPC (a
    /// loaded pilot's), rolls it on `chance`, as `_LoadPilotData`'s
    /// `_SpawnEscort` does (@0x75a25; see the `afterburner_roll` module).
    pub(super) fn enter_escorts(
        &mut self,
        berths: &[u32],
        kept: &[KeptEscort],
        chance: &mut dyn Chance,
    ) {
        if self.setup.rules.source_for(RuleKey::EscortOrders) == RuleSource::Engine {
            for escort in &mut self.pilot.escorts {
                escort.order = None;
            }
            for npc in self.space.traffic.npcs_mut() {
                if let Some(mark) = &mut npc.mission {
                    mark.order = None;
                }
            }
            self.space.escorts.class_orders = [None; 4];
        }
        self.space.escorts.fleet = (0..self.pilot.escorts.len())
            .map(|index| {
                let kept = kept.get(index).copied().unwrap_or_default();
                let id = self.place_escort_at(
                    index,
                    berths.get(index).copied(),
                    kept.skill,
                    &mut *chance,
                )?;
                let loaded = self
                    .space
                    .npc(id)
                    .map(|npc| self.ship.loaded_escort_set(&self.setup, &npc.hull.frames));
                if let Some(npc) = self.space.npc_mut(id) {
                    npc.aux = kept.aux;
                    npc.voice = kept.voice;
                    npc.ion = Ion::default();
                    npc.cloak = kept.cloak;
                    // `_RespawnEscort` clears its glow (@0x3d505).
                    npc.display.glow = 0;
                    match kept.display {
                        Some((frames, fold, anim)) => {
                            npc.display.frames = frames;
                            npc.display.rolls.anim = anim;
                            *npc.armament.fold_mut() = fold;
                        }
                        None => npc.armament.fold_mut().set = loaded.unwrap_or(0),
                    }
                }
                match kept.afterburner {
                    Some(afterburner) => {
                        if let Some(npc) = self.space.npc_mut(id) {
                            npc.afterburner = afterburner;
                        }
                    }
                    None => self.roll_escort_afterburner(id, chance),
                }
                Some(id)
            })
            .collect();
        self.space.traffic.reserve(BTreeSet::new());
        self.space.escorts.restock_persons = false;
    }

    /// Escort `index` as an NPC in the system at the first free berth, its
    /// skill rolled on `chance` (see [`Session::place_escort_at`]).
    pub(super) fn place_escort(&mut self, index: usize, chance: &mut dyn Chance) -> Option<NpcId> {
        self.place_escort_at(index, None, None, chance)
    }

    /// Escort `index` as an NPC in the system, at `berth` or else the
    /// first free one, beside the player until it is snapped to its slot,
    /// with no afterburner, which each caller rolls or restores (see the
    /// `afterburner_roll` module); none when its ship type has no record.
    /// It flies at `skill`, or one rolled on `chance` as `_SpawnEscort`
    /// rolls it (through `_GenericRandomShipSpawn` @0x3cad1), of no
    /// government.
    #[allow(clippy::too_many_lines)] // one line for each of the NPC's fields
    pub(super) fn place_escort_at(
        &mut self,
        index: usize,
        berth: Option<u32>,
        skill: Option<f32>,
        chance: &mut dyn Chance,
    ) -> Option<NpcId> {
        let mut escort = *self.pilot.escorts.get(index)?;
        let record = self.statics.ship_record(escort.ship)?;
        let maneuver = record.fields.maneuver;
        let fitted = escort
            .person
            .and_then(|id| self.space.traffic.person(id))
            .map(|person| {
                let grudge = self.pilot.grudges.contains(&person.record.id);
                (person.kind.clone(), NpcPerson::of(&person.record, grudge))
            });
        let (kind, person) = match fitted {
            Some((kind, person)) => (kind, Some(person)),
            None => (self.setup.ship_kind(&self.statics, record), None),
        };
        if self.space.escorts.restock_persons && person.is_some() {
            restock(&mut escort.reserves, &kind);
            self.pilot.escorts[index].reserves = escort.reserves;
        }
        let carrier = escort.carried.then(|| self.player_carrier(maneuver));
        let skill = skill.unwrap_or_else(|| random_skill_level(kind.skill_var, chance));
        let mut npc = Npc {
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
            state: self.ship.player,
            // `_RespawnEscort` zeroes +0x48 (@0x3d59d).
            speed: 0.0,
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
            stray: 0.0,
            aggression: 0,
            inspected: None,
            booty: None,
            boarded: false,
            info_types: 0,
            cargo: NpcHold::default(),
            spared: false,
            carrier: None,
            person,
            mission: None,
            aux: None,
            cloak: Cloak::default(),
            cloak_gear: kind.cloak,
            led_out: false,
            emerges_from: None,
            visited: None,
            travel_set: false,
            self_repair: kind.self_repair,
            immobile: kind.immobile,
            skill,
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
            // A hired escort's slot keeps the `PodCount` its first spawn
            // gave it (`_GenericRandomShipSpawn` @0x3ca48), as
            // `_RespawnEscort` leaves +0xc908 alone.
            pods_left: kind.hull.signed_pod_count,
            warp: crate::traffic::warp_out::Warp::default(),
            // `_SpawnEscort` and a capture's spawn take their slot from
            // `_GenericRandomShipSpawn` (@0x3fe8f, @0x4114d): its glow 32
            // (@0x3cb77) and its anim rolled on the first class's
            // (@0x3c929); an entering escort's `_RespawnEscort` then
            // clears its glow (@0x3d505), its display otherwise its
            // slot's (see `Session::enter_escorts`).
            display: NpcDisplay {
                glow: GLOW_SPAWN,
                rolls: Rolls {
                    anim: AnimRoll::FirstClass,
                    ..Rolls::default()
                },
                ..NpcDisplay::default()
            },
            // The spawn marks it stale (see `nova_sim_ai::traffic::odds_cache`).
            odds: crate::traffic::odds_cache::STALE,
            no_ai: false,
        };
        npc.independent();
        // A carried fighter holds a fighter's capacities, keeping what its
        // record holds (`_ShipShieldCapacity`, `_ShipArmorCapacity` for AI
        // type 5, @0x2a91-0x2aa7, @0x2cb9-0x2cd2).
        if let Some(carrier) = carrier {
            npc.take_carrier(
                carrier,
                self.setup.rules.source_for(RuleKey::FighterCapacity),
            );
        }
        Some(match berth {
            Some(berth) => self.space.traffic.add_npc_at(npc, berth),
            None => self.space.traffic.add_npc(npc),
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
    /// see [`self_repair`](super::self_repair)): of no government, and so
    /// of no government's factor, keeping its skill, its AI
    /// type its ship type's `InherentAI`, led by the player alone,
    /// keeping formation by `record`'s standing order with no target of
    /// its own, carried by the player when `record` is a carried
    /// fighter (holding a fighter's capacities, [`Npc::take_carrier`](crate::traffic::npc::Npc::take_carrier)),
    /// and no longer a former escort. Its record joins the
    /// pilot's, the formation re-forms, and a save is due.
    pub(super) fn enlist(&mut self, id: NpcId, record: Escort) {
        let ship = self
            .statics
            .ship_record(record.ship)
            .map(|ship| (ship.inherent_ai, ship.fields.maneuver));
        let carrier = ship
            .filter(|_| record.carried)
            .map(|(_, maneuver)| self.player_carrier(maneuver));
        let Some(npc) = self.space.npc_mut(id) else {
            return;
        };
        fall_in(
            npc,
            ship.map(|(inherent_ai, _)| AiType::from_raw(inherent_ai)),
        );
        npc.escort = Some(EscortDuty {
            formation: FormationSlot::default(),
            order: record.order,
        });
        if let Some(carrier) = carrier {
            npc.take_carrier(
                carrier,
                self.setup.rules.source_for(RuleKey::FighterCapacity),
            );
        }
        self.pilot.escorts.push(record);
        self.space
            .escorts
            .fleet
            .resize(self.pilot.escorts.len() - 1, None);
        self.space.escorts.fleet.push(Some(id));
        self.reform();
        self.out.save_due = true;
    }

    /// Gives each ship the player leads ([`Session::player_led`]) whose
    /// formation parent is the player its slot in one formation of them
    /// and the player ([`Session::formation_slots`]). The parent is the
    /// player for each ship it leads but one attacking with a swarm leader,
    /// which forms on that ([`swarm::formation_parent`], `_DoPlayGameWork`
    /// @0x44160-0x441c0) and keeps the slot it held. The original
    /// recomputes the formation every frame (the parent's +0xc2 flag); the
    /// engine re-forms on each change to the led set, and on each change
    /// of parent before the traffic's tick (the `swarm_formation` module),
    /// which gives the same slots at every frame.
    pub(super) fn reform(&mut self) {
        for (id, formation) in self.formation_slots() {
            let Some(npc) = self.space.npc_mut(id) else {
                continue;
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

    /// The slot each ship in the player's formation takes: those the
    /// player leads whose formation parent is the player, 2, 3, … in
    /// ship-table order, by berth, hired and mission escorts alike, as
    /// `_AICalcFormationPositions` numbers every ship whose formation
    /// parent is the player in table-slot order (@0x85137-0x851cc), out of
    /// them and the player, spaced by the widest sprite among them, the
    /// player's included ([`escort::number_formation`], which numbers an
    /// NPC's formation too).
    pub(super) fn formation_slots(&self) -> Vec<(NpcId, FormationSlot)> {
        let placed = self
            .player_led()
            .filter(|npc| swarm::formation_parent(npc) == Some(FormationParent::Player));
        escort::number_formation(self.ship.fit.hull.sprite_size, placed)
    }

    /// Puts escort NPC `id`, hired or a mission escort, on its slot beside
    /// the player, with the player's heading and velocity.
    pub(super) fn snap(&mut self, id: NpcId) {
        let player = self.ship.player;
        if let Some(npc) = self.space.npc_mut(id)
            && let Some(formation) = npc.formation()
        {
            npc.state = ShipState {
                position: formation.position(&player),
                ..player
            };
        }
    }

    /// NPC `id`, if it is in the system, to change.
    #[cfg(any(test, feature = "fixture"))]
    pub(crate) fn npc_mut(&mut self, id: NpcId) -> Option<&mut Npc> {
        self.space.npc_mut(id)
    }

    /// Every escort the fight's tick left disabled or breaking up, or
    /// that is gone, leaves the fleet: its record goes, and its NPC is no
    /// longer an escort. A save is due when any did. The original clears
    /// a led ship's leader in `_DamageShip`, on a hit that leaves it
    /// disabled or dying (@0x3ae5b-0x3afdd), so the tick loses an escort
    /// that was intact before it (`was`) or that it hit (`struck`), and
    /// keeps one already disabled that nothing hit: a former escort that
    /// rejoined repaired but still disabled stays led (see the
    /// `self_repair` module). Each that is not a carried fighter and whose ship is still
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
    pub(super) fn lose_escorts(&mut self, was: &[super::NpcBefore], struck: &BTreeSet<NpcId>) {
        let downed = |id: NpcId| {
            struck.contains(&id)
                || was
                    .iter()
                    .any(|before| before.id == id && before.condition == Condition::Intact)
        };
        let lost: Vec<NpcId> = self
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .filter(|&id| {
                self.space
                    .npc(id)
                    .is_none_or(|npc| npc.condition != Condition::Intact && downed(id))
            })
            .collect();
        for id in lost {
            let index = self.space.fleet_index(id);
            let record = index
                .and_then(|index| self.pilot.escorts.get(index))
                .copied();
            let takes = record
                .filter(|escort| !escort.carried)
                .map(|escort| escort.ship);
            let here = self.space.npc(id).is_some();
            if let (Some(npc), Some(record)) = (self.space.npc_mut(id), record) {
                npc.former = Some(FormerEscort { record });
            }
            self.dismiss(id);
            if let Some(ship) = takes.filter(|_| here) {
                self.lose_cargo_share(ship, id);
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
    /// [`RuleKey::LostCargoShare`] they count too. The share destroyed is
    /// NPC `lost`'s hold.
    pub(super) fn lose_cargo_share(&mut self, ship: ShipId, lost: NpcId) {
        let Some(holds) = self.statics.trader_holds(ship) else {
            return;
        };
        let counted = self.setup.rules.source_for(RuleKey::LostCargoShare) == RuleSource::Bible;
        self.destroy_cargo_share(holds, counted, Some(lost));
    }

    /// `_DestroyPartialFleetCargo` (@0xcd32-0xcff6) for a ship other than
    /// the player's, of `holds` `Holds`: every good, commodity or `jünk`,
    /// loses the truncated part [`market::lost_share`] gives, from those
    /// `Holds` over the fleet's holds as they stand, and `holds` once more
    /// when `counted`; then the session resets, as the call's tail does
    /// (@0xcff6; [`Session::reset_precalced`]), its trim cutting the
    /// commodities to the fleet's holds against everything carried, the
    /// mission cargo aboard included. The mission cargo is never touched.
    /// The tons of each commodity destroyed become the `lost` ship's own
    /// hold, every one of the six, so whatever it held is overwritten
    /// (`movw %dx, 0x7a(ship)` @0xcf76); a `jünk` destroyed goes nowhere
    /// (@0xcf94-0xcfe4). Boarding the ship to repair it gives them back
    /// (see the `board_repair` module).
    pub(super) fn destroy_cargo_share(&mut self, holds: i16, counted: bool, lost: Option<NpcId>) {
        let share = market::lost_share(
            holds,
            self.ship.fit.stats.capacity,
            market::escort_tons(self.escort_holds()),
            counted,
        );
        let destroyed = market::destroy_share(&mut self.pilot.cargo, share);
        if let Some(npc) = lost.and_then(|id| self.space.npc_mut(id)) {
            npc.cargo = NpcHold::from_words(destroyed);
        }
        self.reset_precalced();
    }

    /// Escort NPC `id` leaves the fleet: its record goes, it is no longer
    /// an escort, and the rest re-form. A save is due.
    pub(super) fn dismiss(&mut self, id: NpcId) {
        if self.unfleet(id) {
            self.out.save_due = true;
        }
    }

    /// Escort NPC `id` leaves the fleet, as [`Session::dismiss`] says but
    /// making no save due; whether it was in the fleet. A holding escort
    /// ([`Goal::Wait`]) takes goal 0, as `_EscortAI` writes goal and sub 0
    /// for a ship of AI type above 4 whose leader is gone
    /// (@0x83917-0x8397b): `_DamageShip`'s leader clear (@0x3afdd) keeps
    /// its type. Without it, a disabled holding escort the fleet lost and
    /// later repaired would wait for good.
    pub(super) fn unfleet(&mut self, id: NpcId) -> bool {
        let Some(index) = self.space.fleet_index(id) else {
            return false;
        };
        self.space.escorts.fleet.remove(index);
        if self.pilot.escorts.get(index).is_some() {
            self.pilot.escorts.remove(index);
        }
        if let Some(npc) = self.space.npc_mut(id) {
            npc.escort = None;
            if npc.goal == Goal::Wait {
                npc.goal = Goal::Idle;
            }
        }
        self.reform();
        true
    }

    /// Escort `index` leaves the fleet and the system: its record, its
    /// place in the fleet and its NPC, if any, go together, keeping the
    /// fleet and the pilot's escorts in step. Unlike
    /// [`Session::unfleet`], its NPC goes too, and the fleet is not
    /// re-formed: the caller re-forms it once it has dropped all it drops.
    pub(super) fn drop_fleet_escort(&mut self, index: usize) {
        self.pilot.escorts.remove(index);
        if index < self.space.escorts.fleet.len()
            && let Some(id) = self.space.escorts.fleet.remove(index)
        {
            self.space.traffic.remove(id);
        }
    }

    /// Escort NPC `id` is released (see [`hail`](super::hail)): it takes
    /// its share of the fleet's cargo while still in the fleet, its own
    /// `Holds` among the fleet's when it is a trader
    /// ([`Session::destroy_cargo_share`], `_DoCommDialog` @0x96864), any
    /// ship type by the engine reading of [`RuleKey::ReleaseCargoShare`]
    /// and only a trader by the other; then it leaves the fleet and the
    /// system (`_AIMakeShipLeave`), jumping out whatever its fuel.
    ///
    /// Both the comm dialog's close (@0x96848-0x968c1) and the escort
    /// dialog's Release (@0x95485-0x9550f) reset it as it goes: it can
    /// never be boarded (+0xb9 1), it is no former escort (+0xc8de -1),
    /// its AI type is its class's `InherentAI` (+0x88, which an escort's
    /// already is in the engine), and `_AISetup` (@0x7f3e2) clears its
    /// order word (+0xc90a, [`Npc::order_word`]). `_AISetup`'s goal and
    /// sub 0 and its goal targets +0x90 and +0x92 (-2) give way to the
    /// leave's goal and its target cleared, which follow at once; the
    /// engine has no field for its +0xc906 or +0xc92e.
    ///
    /// A mission escort, which only the comm dialog's Release reaches (a
    /// mission ship captured for an escort, see the
    /// `captured_mission_ship` module), is released by
    /// [`Session::release_mission_escort`].
    pub(super) fn release(&mut self, id: NpcId) {
        if self.space.npc(id).is_some_and(Npc::mission_escort) {
            self.release_mission_escort(id);
            return;
        }
        if !self.is_escort(id) {
            return;
        }
        if let Some(holds) = self
            .space
            .released_holds(&self.pilot, &self.statics, &self.setup, id)
        {
            self.destroy_cargo_share(holds, false, Some(id));
        }
        self.dismiss(id);
        if let Some(npc) = self.space.npc_mut(id) {
            reset_released(npc);
        }
    }

    /// Whether NPC `id` is one of the player's escorts.
    #[must_use]
    pub fn is_escort(&self, id: NpcId) -> bool {
        self.space.is_escort(id)
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
    ///
    /// Under any command but Return to Hangar, a fighter flying home
    /// ([`returning_fighter`]) lets go of its target and counts as changed
    /// (@0x6627a-0x662dc). When some escort that isn't mute had its order
    /// changed, one of the group acknowledges it, drawn on `chance`
    /// ([`command_speaker`](crate::escort_voice::command_speaker)):
    /// "attacking target" when the report says so (@0x6660a-0x6663c), else
    /// the acknowledgement, queued on the voice channel (see the `voices`
    /// module).
    pub(crate) fn command_escorts(
        &mut self,
        group: EscortGroup,
        command: EscortCommand,
        chance: &mut dyn Chance,
    ) -> Option<Commanded> {
        if self.mode.landed().is_some() {
            return None;
        }
        // The original lets go of the escorts' targets earlier in every
        // warp-stage frame (@0x6bb0a, before the escort keys @0x6e192);
        // the warp stage's own pass does here (see the `warp_stage`
        // module), so a target a command gives is let go on the next.
        // Reported, "attacking target.", for a target the player does not
        // lead (@0x6653f-0x66559); copied only when it is not allied with
        // the player either (@0x6625f).
        let reported = self.chart.target.filter(|&target| {
            command == EscortCommand::Attack && !self.player_led().any(|npc| npc.id == target)
        });
        let target = reported
            .filter(|&target| !self.space.allied_with_player(target))
            .map(ShipRef::Npc);
        let mut changed = false;
        let mut recalled = false;
        let mut voters = Vec::new();
        for index in 0..self.pilot.escorts.len() {
            if !group.holds(self.space.escort_class(&self.pilot, &self.statics, index)) {
                continue;
            }
            let escort = &mut self.pilot.escorts[index];
            let (order, recall) = order_for(command, escort.carried, escort.order);
            recalled |= recall;
            let reordered = escort.order != order;
            changed |= reordered;
            escort.order = order;
            let placed = self.space.escorts.fleet.get(index).copied().flatten();
            let Some(npc) = placed.and_then(|id| self.space.npc_mut(id)) else {
                continue;
            };
            if let Some(duty) = &mut npc.escort {
                duty.order = order;
            }
            let voter = obey(npc, command, target, reordered);
            changed |= voter.changed;
            voters.push(voter);
        }
        for (class, kept) in EscortClass::ALL
            .into_iter()
            .zip(&mut self.space.escorts.class_orders)
        {
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
            let Some(npc) = self.space.npc_mut(id) else {
                continue;
            };
            let Some(mark) = &mut npc.mission else {
                continue;
            };
            let (order, recall) = order_for(command, false, mark.order);
            recalled |= recall;
            let reordered = mark.order != order;
            mark.order = order;
            let voter = obey(npc, command, target, reordered);
            changed |= voter.changed;
            voters.push(voter);
        }
        if changed {
            self.out.sounds.push(crate::SimSound::EscortsCommanded);
            self.space
                .acknowledge(&self.statics, &mut voters, reported.is_some(), chance);
        }
        changed.then_some(Commanded {
            group,
            command: if recalled {
                EscortCommand::Recall
            } else {
                command
            },
            targeted: reported.is_some(),
            underway: self.mode.jump().is_some(),
        })
    }
}

impl Statics {
    /// The `Holds` of ship class `ship` when its `InherentAI` is a
    /// trader's ([`FREIGHTER_AI`](escort::FREIGHTER_AI) or less).
    pub(super) fn trader_holds(&self, ship: ShipId) -> Option<i16> {
        self.ship_record(ship)
            .filter(|record| record.inherent_ai <= escort::FREIGHTER_AI)
            .map(|record| record.fields.holds)
    }

    /// The `Holds` a ship of class `ship` takes its share of the cargo by
    /// as it is released (`_DestroyPartialFleetCargo(+0x86)`, the ship's
    /// class, @0x96864, @0x9549d, @0x9561d), as
    /// [`RuleKey::ReleaseCargoShare`] reads in `setup`: none when the
    /// class has no record, or by the other reading when it is not a
    /// trader.
    pub(super) fn released_holds(&self, setup: &Setup, ship: ShipId) -> Option<i16> {
        match setup.rules.source_for(RuleKey::ReleaseCargoShare) {
            RuleSource::Engine => self.ship_record(ship).map(|record| record.fields.holds),
            RuleSource::Bible => self.trader_holds(ship),
        }
    }
}

impl Space {
    /// The fleet index of escort NPC `id`, if it is in the fleet.
    pub(super) fn fleet_index(&self, id: NpcId) -> Option<usize> {
        self.escorts
            .fleet
            .iter()
            .position(|&placed| placed == Some(id))
    }

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
    pub(super) fn fleet_berths(&self, pilot: &Pilot) -> Vec<u32> {
        let kept: Vec<Option<u32>> = (0..pilot.escorts.len())
            .map(|index| {
                let id = self.escorts.fleet.get(index).copied().flatten()?;
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

    /// What each of the fleet's escorts keeps across a setup, by its
    /// index, from its NPC in the system, if any (none, and uncloaked,
    /// without one).
    ///
    /// As `_DeleteAllComputerShips(0)` keeps the player's ships whole
    /// (@0x3bf7e-0x3bfb6), a captured aux ship keeps its mark, and every
    /// escort its skill and its afterburner (+0xbd), which `_RespawnEscort`
    /// never writes. Its
    /// ionization is not kept: `_RespawnEscort` clears the charge
    /// (+0x5c @0x3d543), and the display `_PrepareAllSprites` runs before
    /// the first frame (@0x3a783) clears the colour of a ship with no
    /// charge (@0x2bf8a). Neither `_RespawnEscort` (@0x3d4b8-0x3d62c), its
    /// tail `_AIMakeEscortFlyInForm` (@0x89b8e-0x89c3d) nor the kept slot
    /// (@0x3bf7e-0x3bfb6) writes the cloak's fade (+0x64) or direction
    /// (+0xc8d8), so the escort keeps its cloak as it was, cloaked, fading
    /// or seen.
    pub(super) fn fleet_kept(&self, pilot: &Pilot) -> Vec<KeptEscort> {
        (0..pilot.escorts.len())
            .map(|index| {
                let npc = self
                    .escorts
                    .fleet
                    .get(index)
                    .copied()
                    .flatten()
                    .and_then(|id| self.npc(id));
                npc.map_or_else(KeptEscort::default, |npc| KeptEscort {
                    aux: npc.aux,
                    display: Some((
                        npc.display.frames,
                        npc.armament.fold(),
                        npc.display.rolls.anim,
                    )),
                    skill: Some(npc.skill),
                    afterburner: Some(npc.afterburner),
                    voice: npc.voice,
                    cloak: npc.cloak,
                })
            })
            .collect()
    }

    /// NPC `id`, if it is in the system.
    pub(super) fn npc(&self, id: NpcId) -> Option<&Npc> {
        self.traffic.npcs().iter().find(|npc| npc.id == id)
    }

    /// NPC `id`, if it is in the system, to change.
    pub(super) fn npc_mut(&mut self, id: NpcId) -> Option<&mut Npc> {
        self.traffic.npcs_mut().iter_mut().find(|npc| npc.id == id)
    }

    /// Each escort's shield, armour and fuel, full as its class holds
    /// them, the fuel as recorded no more than it holds.
    /// A person escort, whose fitted ship only the system's traffic knows,
    /// is restocked to it as it is placed next.
    pub(super) fn restock_fleet(&mut self, pilot: &mut Pilot, statics: &Statics, setup: &Setup) {
        for index in 0..pilot.escorts.len() {
            let ship = pilot.escorts[index].ship;
            let Some(record) = statics.ship_record(ship) else {
                continue;
            };
            let kind = setup.ship_kind(statics, record);
            restock(&mut pilot.escorts[index].reserves, &kind);
        }
        self.escorts.restock_persons = true;
    }

    /// Each escort's record takes its NPC's shield, armour and fuel.
    pub(super) fn sync_fleet(&mut self, pilot: &mut Pilot) {
        for index in 0..pilot.escorts.len() {
            let Some(id) = self.escorts.fleet.get(index).copied().flatten() else {
                continue;
            };
            if let Some(reserves) = self.npc(id).map(|npc| npc.reserves) {
                pilot.escorts[index].reserves = reserves;
            }
        }
    }

    /// The `Holds` escort NPC `id` takes its share of the cargo by as it
    /// is released, by its escort record's ship class
    /// ([`Statics::released_holds`]); none when it is not in the fleet.
    fn released_holds(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        setup: &Setup,
        id: NpcId,
    ) -> Option<i16> {
        let index = self.fleet_index(id)?;
        let ship = pilot.escorts.get(index)?.ship;
        statics.released_holds(setup, ship)
    }

    /// Whether NPC `id` is one of the player's escorts.
    #[must_use]
    fn is_escort(&self, id: NpcId) -> bool {
        self.escorts.fleet.contains(&Some(id))
    }

    /// The escort class of the fleet's escort `index`: its NPC's, or its
    /// ship type's by its record.
    pub(super) fn escort_class(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        index: usize,
    ) -> EscortClass {
        if let Some(npc) = self
            .escorts
            .fleet
            .get(index)
            .copied()
            .flatten()
            .and_then(|id| self.npc(id))
        {
            return npc.class;
        }
        pilot
            .escorts
            .get(index)
            .and_then(|escort| statics.ship_record(escort.ship))
            .map_or_else(EscortClass::default, |record| {
                EscortClass::of(record.escort_type, record.inherent_ai, record.fields.mass)
            })
    }

    /// Whether NPC `id` is here and allied with the player
    /// ([`Surroundings::allied_with_player`]).
    fn allied_with_player(&self, id: NpcId) -> bool {
        let npcs = self.traffic.npcs();
        self.npc(id)
            .is_some_and(|npc| Surroundings::new(&[], npcs).allied_with_player(npc))
    }

    /// The escort menu's class rows, in its order ([`EscortClass::ALL`]):
    /// each class's row ([`Space::class_row`]).
    pub(super) fn escort_rows(&self, pilot: &Pilot, statics: &Statics) -> [ClassRow; 4] {
        EscortClass::ALL.map(|class| self.class_row(pilot, statics, class))
    }

    /// Whether the player has an escort flying, which the escort menu
    /// counts each frame it shows (`_HandlePlayer` @0x6df59-0x6dfa7): a
    /// ship the player leads ([`Space::player_led`]) that is neither
    /// disabled nor breaking up. A hired or captured escort, a mission
    /// escort and a launched fighter all count; a hired escort disabled or
    /// breaking up does not.
    pub(super) fn escorts_flying(&self) -> bool {
        self.player_led()
            .any(|npc| npc.condition == Condition::Intact)
    }

    /// `class`'s row in the escort menu: whether the player leads a ship
    /// of it, hired or a mission escort, and the standing order of its
    /// first hired escort, else of its first mission escort.
    fn class_row(&self, pilot: &Pilot, statics: &Statics, class: EscortClass) -> ClassRow {
        let hired = (0..pilot.escorts.len())
            .find(|&index| self.escort_class(pilot, statics, index) == class)
            .map(|index| pilot.escorts[index].order);
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
    pub(super) fn class_order(
        &self,
        pilot: &Pilot,
        statics: &Statics,
        class: EscortClass,
    ) -> Option<EscortOrder> {
        let kept = EscortClass::ALL
            .iter()
            .position(|&each| each == class)
            .and_then(|index| self.escorts.class_orders[index]);
        let row = self.class_row(pilot, statics, class);
        if row.present { row.order } else { kept }.filter(|&order| order != EscortOrder::Dock)
    }
}

/// The resets a ship released by hail takes as it goes (`_DoCommDialog`
/// @0x96896-0x968b7, `_DoEscortDialog` @0x954bb-0x954f2): it can never be
/// boarded (+0xb9 1), it is no former escort (+0xc8de -1), `_AISetup`
/// clears its order word (+0xc90a), and it leaves the system
/// (`_AIMakeShipLeave`, [`super::hail::make_leave`]).
pub(super) fn reset_released(npc: &mut Npc) {
    npc.boarded = true;
    npc.former = None;
    npc.order_word = crate::fleet::OrderWord::NONE;
    super::hail::make_leave(npc);
}

/// `npc` falls in with the player, as a hired escort joining
/// (`_AIMakeEscortFlyInForm` @0x89b8e) and a captured mission escort
/// alike: of no government, and so of no government's factor, keeping
/// its skill, its AI type `inherent` (its ship type's `InherentAI`) when
/// known, no longer a former escort, of no leader but the player, keeping
/// formation with no target of its own.
pub(super) fn fall_in(npc: &mut Npc, inherent: Option<AiType>) {
    npc.independent();
    if let Some(ai_type) = inherent {
        npc.ai_type = ai_type;
    }
    npc.former = None;
    npc.leader = None;
    npc.goal = Goal::Formation { guard: None };
    npc.target = None;
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

/// `npc`, an escort of the command's group whose order changed as
/// `reordered` says, takes `command` with `target` (see
/// [`take_target`]), and lets go of its target when it is a fighter flying
/// home ([`returning_fighter`], @0x6627a-0x662b3) and the command is not
/// Return to Hangar: how it votes on the command's voice
/// (`_IssueNewEscortCommand` @0x6617c-0x662dc).
fn obey(npc: &mut Npc, command: EscortCommand, target: Option<ShipRef>, reordered: bool) -> Voter {
    let retargeted = take_target(npc, target);
    let returning = command != EscortCommand::Dock && returning_fighter(npc);
    if returning {
        npc.target = None;
    }
    Voter {
        berth: npc.berth,
        changed: reordered || retargeted || returning,
        speaks: reordered && !npc.hull.mute,
        returning,
        mute: npc.hull.mute,
        speaker: Speaker {
            ship: npc.ship,
            voice: npc.voice,
        },
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
pub(super) mod tests {
    use crate::testkit::work;
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
        Draws, FAST, FakePilotCatalog, begin_jump_now, catalog, hull, jump, land_now, outfit, ship,
        weapon, work_with,
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
                mark: None,
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
        work_with(
            session,
            catalog,
            Controls::default(),
            &Peaceful,
            Rules::default(),
            &mut NeverFires,
        );
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
    pub(in crate::session) fn fleeted() -> FakePilotCatalog {
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
            mark: None,
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
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .map(|&id| session.space.npc(id).expect("in the system"))
            .collect()
    }

    /// Where escort `npc` should be: its slot beside the player.
    fn slot_of(session: &Session, npc: &Npc) -> Vec2 {
        let duty = npc.escort.expect("an escort");
        duty.formation.position(session.player())
    }

    /// Where escort `npc` arrives from a jump: pushed back from its slot
    /// beside the player, with the player's state, to glide in (the
    /// `escort_arrival` module).
    fn pushed_from_slot(session: &Session, npc: &Npc) -> ShipState {
        crate::escort_arrival::pushed(ShipState {
            position: slot_of(session, npc),
            ..*session.player()
        })
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
    fn a_hired_escort_placed_carries_its_classs_crew_pods() {
        let mut catalog = fleeted();
        catalog.hulls[1].pod_count = 3;
        let session = flying(&catalog, RuleSource::Engine);
        let pods: Vec<_> = escort_npcs(&session)
            .iter()
            .map(|npc| (npc.ship, npc.pods_left))
            .collect();
        assert_eq!(pods, [(WARSHIP, 3), (FREIGHTER, 0)]);
        for npc in escort_npcs(&session) {
            // `_SpawnEscort` @0x3ffb8.
            assert_eq!(npc.order_word, crate::fleet::OrderWord::NONE);
        }
        catalog.hulls[1].pod_count = -3;
        let session = flying(&catalog, RuleSource::Engine);
        assert_eq!(escort_npcs(&session)[0].pods_left, -3, "raw");
    }

    #[test]
    fn an_escort_whose_ship_type_has_no_record_is_kept_but_not_placed() {
        let catalog = fleeted();
        let mut pilot = escorted(&catalog);
        pilot.escorts.insert(0, escort(ShipId(999), worn(), None));
        let mut session = Session::fly(&catalog, pilot).expect("flies");
        tick(&mut session, &catalog);
        assert_eq!(session.pilot().escorts().len(), 3);
        assert_eq!(session.space.escorts.fleet.len(), 3);
        assert_eq!(session.space.escorts.fleet[0], None);
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
        work_with(
            session,
            catalog,
            controls,
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
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
            ..Controls::default()
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
    fn escorts_follow_the_player_through_a_jump_back_from_their_slots_with_their_shield() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(warship)
            .expect("there")
            .reserves
            .shield
            .now = 4.0;
        work(&mut session, &catalog, Controls::default());
        assert_eq!(
            session.pilot().escorts()[0].reserves.shield.now,
            4.0,
            "kept on the record"
        );
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let escorts = escort_npcs(&session);
        assert_eq!(escorts.len(), 2, "both came along");
        for npc in &escorts {
            assert_eq!(
                npc.state,
                pushed_from_slot(&session, npc),
                "pushed back from its slot"
            );
            assert_eq!(npc.mode, Mode::JumpingIn { tick: 0 }, "to glide in");
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
    fn escorts_arrive_50_faster_than_the_players_top_speed_velocity() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        let player = *session.player();
        assert!(
            (player.velocity.length() - session.top_speed()).abs() < 1e-4,
            "the jump arrives at top speed: {player:?}"
        );
        let escorts = escort_npcs(&session);
        assert_eq!(escorts.len(), 2);
        let heading = crate::trig::whole(player.heading);
        for npc in &escorts {
            assert_eq!(
                npc.state.velocity,
                player.velocity + crate::trig::accel(heading, 50.0),
                "50 faster along its heading"
            );
            assert_eq!(npc.state.heading, player.heading);
        }
    }

    #[test]
    fn an_escort_rolls_its_skill_when_first_placed_and_keeps_it_through_each_entry() {
        // The warship of `SkillVar` 37: 75 sides, the last 1.37.
        let mut catalog = fleeted();
        catalog.ship_records[1].skill_var = 37;
        let rolled = (f64::from(74 - 37 + 100) * 0.01) as f32;
        let skills = |session: &Session| -> Vec<(f32, f32)> {
            escort_npcs(session)
                .iter()
                .map(|npc| (npc.skill, npc.skill_mult))
                .collect()
        };
        let placed = || {
            let mut session = Session::fly(&catalog, escorted(&catalog)).expect("flies");
            let mut chance = Draws::of(&[]).keyed(75, 74);
            work_with(
                &mut session,
                &catalog,
                Controls::default(),
                &Peaceful,
                Rules::default(),
                &mut chance,
            );
            assert_eq!(chance.times_asked(75), 1, "the loaded pilot's escort rolls");
            assert_eq!(skills(&session), [(rolled, 1.0), (1.0, 1.0)]);
            session
        };
        let mut session = placed();
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_eq!(skills(&session), [(rolled, 1.0), (1.0, 1.0)], "jumped");
        let mut session = placed();
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(skills(&session), [(rolled, 1.0), (1.0, 1.0)], "took off");
    }

    #[test]
    fn an_escort_flies_at_its_classs_alone_by_the_engine_and_with_its_items_by_the_other_reading() {
        // The warship carries a reactor (+300 `Speed`) by default.
        let mut catalog = fleeted();
        catalog
            .outfits
            .push(outfit(330, &[(crate::stats::MORE_SPEED, 300)]));
        catalog.ship_records[1].defaults = vec![(crate::catalog::OutfitId(330), 1)];
        for (items, top) in [(RuleSource::Engine, 6.0), (RuleSource::Bible, 9.0)] {
            let mut session = Session::fly(&catalog, escorted(&catalog))
                .expect("flies")
                .with_rules(Rulebook::default().with_override(RuleKey::NpcDefaultItems, items));
            tick(&mut session, &catalog);
            assert_eq!(
                escort_npcs(&session)[0].stats.handling.max_speed,
                top,
                "{items:?}"
            );
        }
    }

    #[test]
    fn a_take_off_restocks_an_escort_to_its_classs_reserves_alone_by_the_engine() {
        // The warship carries armour plating (+50) by default.
        let mut catalog = fleeted();
        catalog
            .outfits
            .push(outfit(331, &[(crate::stats::MORE_ARMOR, 50)]));
        catalog.ship_records[1].defaults = vec![(crate::catalog::OutfitId(331), 1)];
        for (items, armor) in [(RuleSource::Engine, 45.0), (RuleSource::Bible, 95.0)] {
            let mut session = Session::fly(&catalog, escorted(&catalog))
                .expect("flies")
                .with_rules(Rulebook::default().with_override(RuleKey::NpcDefaultItems, items));
            tick(&mut session, &catalog);
            land_now(&mut session).expect("lands");
            session
                .take_off(&catalog, &mut NeverFires)
                .expect("takes off");
            let reserves = session.pilot().escorts()[0].reserves;
            assert_eq!(
                (reserves.armor.now, reserves.armor.max),
                (armor, armor),
                "{items:?}"
            );
        }
    }

    #[test]
    fn a_ship_enlisted_keeps_its_skill_and_drops_its_governments_factor() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let id = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        let ship = session.space.npc_mut(id).expect("there");
        ship.govt = Some(crate::catalog::GovtId(140));
        ship.skill_mult = 2.0;
        ship.skill = 0.8;
        session.enlist(id, escort(WARSHIP, worn(), None));
        let ship = session.space.npc(id).expect("there");
        assert_eq!((ship.govt, ship.skill_mult, ship.skill), (None, 1.0, 0.8));
    }

    /// Ionizes the first escort red, then checks that, once the fleet
    /// has entered a new system, it has neither charge nor colour, so the
    /// next ionization's colour is its own, and the second escort has
    /// none.
    fn assert_no_ion_colour_left(session: &mut Session, before: NpcId) {
        let escorts: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .copied()
            .flatten()
            .collect();
        assert_ne!(escorts[0], before, "a new NPC");
        let first = session.space.npc(escorts[0]).expect("there").ion;
        assert_eq!(first, Ion::default());
        assert_eq!(
            session.space.npc(escorts[1]).expect("there").ion,
            Ion::default()
        );
        let npc = session.space.npc_mut(escorts[0]).expect("there");
        let at = npc.state.position;
        npc.ion.take(
            crate::combat::ion::Dose {
                ionization: 10,
                colour: 0x0000_00FF,
                blast: None,
            },
            at,
        );
        assert_eq!(
            npc.ion,
            Ion {
                charge: 10.0,
                colour: 0x0000_00FF,
                ..Ion::default()
            }
        );
    }

    fn ionize_first_escort(session: &mut Session) -> NpcId {
        let warship = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(warship).expect("there").ion = Ion {
            charge: 30.0,
            colour: 0x00FF_0000,
            ..Ion::default()
        };
        warship
    }

    #[test]
    fn an_ionized_escort_arrives_after_a_jump_with_no_charge_and_no_colour() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let before = ionize_first_escort(&mut session);
        assert_eq!(jump(&mut session, &catalog, 131), Some(SystemId(131)));
        assert_no_ion_colour_left(&mut session, before);
    }

    #[test]
    fn an_ionized_escort_takes_off_with_no_charge_and_no_colour() {
        let catalog = crate::testkit::far_129_reachable(fleeted());
        let mut session = flying(&catalog, RuleSource::Engine);
        let before = ionize_first_escort(&mut session);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_no_ion_colour_left(&mut session, before);
    }

    /// Checks that, once the fleet has entered a new system, the first
    /// escort is a new NPC carrying `cloak` exactly (the cloak it left
    /// with, as the entry's `_PrepareAllSprites` steps left it), and the
    /// second has the default.
    fn assert_cloak_kept(session: &Session, before: NpcId, cloak: Cloak) {
        let escorts: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .copied()
            .flatten()
            .collect();
        assert_ne!(escorts[0], before, "a new NPC");
        assert_eq!(session.space.npc(escorts[0]).expect("there").cloak, cloak);
        assert_eq!(
            session.space.npc(escorts[1]).expect("there").cloak,
            Cloak::default()
        );
    }

    fn cloak_first_escort(session: &mut Session, cloak: Cloak) -> NpcId {
        let warship = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(warship).expect("there").cloak = cloak;
        warship
    }

    /// Flies `session` out and into its jump to 131, gives its first
    /// escort `cloak` as the jump ends, after the passes on the way have
    /// thought and displayed it, and arrives: the escort as it left.
    fn jump_cloaked(session: &mut Session, catalog: &FakePilotCatalog, cloak: Cloak) -> NpcId {
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(session, catalog);
        crate::testkit::begin_jump_now(session, catalog).expect("jumps");
        let before = cloak_first_escort(session, cloak);
        assert_eq!(
            session.arrive(catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        before
    }

    const FULLY_CLOAKED: Cloak = Cloak {
        fade: crate::cloak::FADE_FULL,
        direction: 0,
    };

    #[test]
    fn a_cloaked_escort_arrives_after_a_jump_cloaked() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let before = jump_cloaked(&mut session, &catalog, FULLY_CLOAKED);
        assert_cloak_kept(&session, before, FULLY_CLOAKED);
        let first = session.space.escorts.fleet[0].expect("placed");
        assert!(session.space.npc(first).expect("there").cloak.is_cloaked());
    }

    #[test]
    fn a_cloaked_escort_takes_off_cloaked() {
        let catalog = crate::testkit::far_129_reachable(fleeted());
        let mut session = flying(&catalog, RuleSource::Engine);
        let before = cloak_first_escort(&mut session, FULLY_CLOAKED);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_cloak_kept(&session, before, FULLY_CLOAKED);
    }

    /// A take-off steps each ship's fade twice before the first tick:
    /// `_HandlePlayerDockRequest` runs `_PrepareAllSprites` after the
    /// setup (@0x6764e) and again after the stellars' visibility
    /// (@0x67658), and each displays every ship, its fade step included
    /// (@0x2d088-0x2d164). An escort fading in at 10 takes off at 11.5.
    #[test]
    fn a_take_off_steps_each_ships_fade_twice_before_the_first_tick() {
        let catalog = crate::testkit::far_129_reachable(fleeted());
        let mut session = flying(&catalog, RuleSource::Engine);
        let fading = Cloak {
            fade: 10.0,
            direction: 1,
        };
        let before = cloak_first_escort(&mut session, fading);
        land_now(&mut session).expect("lands");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        let stepped = Cloak {
            fade: 10.0 + 2.0 * crate::cloak::FADE_RATE,
            direction: 1,
        };
        assert_cloak_kept(&session, before, stepped);
    }

    /// The carried direction, not just the fade, comes through: an escort
    /// fading in when the player jumps arrives with the same direction,
    /// its fade stepped once on by the arrival's `_PrepareAllSprites`
    /// (`_HandlePlayer` @0x6c59c), which displays it before the first
    /// frame.
    #[test]
    fn an_escort_fading_in_when_the_player_jumps_arrives_fading_in() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let fading = Cloak {
            fade: 10.0,
            direction: 1,
        };
        let before = jump_cloaked(&mut session, &catalog, fading);
        let stepped = Cloak {
            fade: 10.0 + crate::cloak::FADE_RATE,
            direction: 1,
        };
        assert_cloak_kept(&session, before, stepped);
    }

    /// An escort's first think in the new system works from the cloak it
    /// carried, and nothing between its placing and that think resets it:
    /// the fixture warship has no cloaking device, so its think
    /// (`_AIHandleCloaking` @0x833e1) uncloaks it, and its fade then steps
    /// once out from full, leaving it still cloaked.
    #[test]
    fn a_cloaked_escort_uncloaks_on_its_first_think_in_the_new_system() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        jump_cloaked(&mut session, &catalog, FULLY_CLOAKED);
        let first = session.space.escorts.fleet[0].expect("placed");
        // Its display steps the fade (@0x2d088), in the ship loop.
        work_with(
            &mut session,
            &catalog,
            Controls::default(),
            &NovaAi::default(),
            crate::combat::Rules::default(),
            &mut NeverFires,
        );
        assert_eq!(
            session.space.escorts.fleet[0],
            Some(first),
            "the same escort"
        );
        let cloak = session.space.npc(first).expect("there").cloak;
        assert_eq!(
            cloak,
            Cloak {
                fade: crate::cloak::FADE_FULL - crate::cloak::FADE_RATE,
                direction: -1,
            }
        );
        assert!(cloak.is_cloaked());
    }

    /// How many passes before the warp's end the escort is cloaked: few
    /// enough that its fade, falling 0.75 a pass from 32, is not yet out
    /// as the warp ends.
    const CLOAKED_BEFORE_THE_END: u16 = 20;

    /// [`fleeted`]'s session by the engine, its warship escort given a
    /// cloak of `Flags2` `flags2`, its jump to 131 begun and run through
    /// its lead-in and on until [`CLOAKED_BEFORE_THE_END`] passes are left
    /// to the warp's end, the escort then fully cloaked: the session and
    /// the escort.
    fn cloaked_near_the_warp_end(catalog: &FakePilotCatalog, flags2: u16) -> (Session, NpcId) {
        let (mut session, escort) = past_the_lead_in(catalog, flags2);
        let mut to_the_end = 0;
        let mut ahead = session.clone();
        while !ahead.warp_end_due() {
            warp_pass(&mut ahead, catalog);
            to_the_end += 1;
            assert!(to_the_end < 1000, "the warp never ended");
        }
        assert!(
            to_the_end > CLOAKED_BEFORE_THE_END,
            "{to_the_end} passes past the lead-in"
        );
        for _ in CLOAKED_BEFORE_THE_END..to_the_end {
            warp_pass(&mut session, catalog);
        }
        assert!(!session.in_warp_lead_in());
        cloak_first_escort(&mut session, FULLY_CLOAKED);
        (session, escort)
    }

    /// [`fleeted`]'s session by the engine, its warship escort given a
    /// cloak of `Flags2` `flags2`, its jump to 131 begun and run through
    /// its lead-in: the session and the escort.
    fn past_the_lead_in(catalog: &FakePilotCatalog, flags2: u16) -> (Session, NpcId) {
        let mut session = flying(catalog, RuleSource::Engine);
        let first = session.space.escorts.fleet[0].expect("placed");
        session.space.npc_mut(first).expect("there").cloak_gear = crate::cloak::CloakGear {
            device: true,
            flags2,
            ..crate::cloak::CloakGear::default()
        };
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        let mut passes = 0;
        while session.in_warp_lead_in() {
            warp_pass(&mut session, catalog);
            passes += 1;
            assert!(passes < 1000, "the lead-in never ended");
        }
        (session, first)
    }

    /// One advance pass of the game's work, no keys held.
    fn warp_pass(session: &mut Session, catalog: &FakePilotCatalog) {
        session.game_work(
            Controls::default(),
            catalog,
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    }

    /// The escort's cloak, checking it still flies, following the
    /// player's jump (goal 0xb).
    fn following_cloak(session: &Session, escort: NpcId) -> Cloak {
        let npc = session.space.npc(escort).expect("still in the system");
        assert_eq!(npc.mode, crate::traffic::npc::Mode::Flying);
        assert!(
            matches!(
                npc.warp.goal,
                Some(crate::traffic::warp_out::WarpGoal::Follow(_))
            ),
            "{:?}",
            npc.warp
        );
        npc.cloak
    }

    /// An escort cloaked once the warp is past its lead-in thinks its
    /// cloak and steps its fade on every pass to the warp's end, as
    /// `_DoPlayGameWork` runs `_AIDispatch` (@0x4429d, `_AIHandleCloaking`
    /// @0x8fcaa) and the display (@0x444ad, the fade @0x2d088-0x2d164)
    /// for every ship with no test of the player's jump. Following the
    /// player's jump (goal 0xb) it reads only the jump's term (`Flags2`
    /// 0x0400, @0x834e6-0x8350b), which its type lacks, though it would
    /// stay cloaked in formation (0x0800): so it uncloaks on its first
    /// think (`_DoShipUncloak` @0xdf55) and fades out at 0.75 a pass. It
    /// arrives with the fade it had at the warp's end, stepped once more
    /// by the arrival's `_PrepareAllSprites` (@0x6c59c).
    #[test]
    fn a_cloaked_escort_fades_out_through_the_warp_and_arrives_with_that_fade() {
        use crate::cloak::{CLOAK_TRAVELLING, FADE_FULL, FADE_RATE};
        let catalog = fleeted();
        let (mut session, escort) = cloaked_near_the_warp_end(&catalog, CLOAK_TRAVELLING);
        let mut passes: u16 = 0;
        while !session.warp_end_due() {
            warp_pass(&mut session, &catalog);
            passes += 1;
            assert_eq!(
                following_cloak(&session, escort),
                Cloak {
                    fade: FADE_FULL - FADE_RATE * f32::from(passes),
                    direction: -1,
                },
                "pass {passes}"
            );
        }
        assert_eq!(passes, CLOAKED_BEFORE_THE_END);
        let at_the_end = FADE_FULL - FADE_RATE * f32::from(passes);
        assert_eq!(at_the_end, 17.0);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_cloak_kept(
            &session,
            escort,
            Cloak {
                fade: at_the_end - FADE_RATE,
                direction: -1,
            },
        );
    }

    /// With the jump's term (`Flags2` 0x0400), which holds in goal 0xb
    /// (@0x834e6-0x8350b), the escort stays fully cloaked through the
    /// warp and arrives so.
    #[test]
    fn a_cloaked_escort_with_the_jump_cloak_stays_cloaked_through_the_warp() {
        use crate::cloak::{CLOAK_JUMPING, CLOAK_TRAVELLING};
        let catalog = fleeted();
        let (mut session, escort) =
            cloaked_near_the_warp_end(&catalog, CLOAK_JUMPING | CLOAK_TRAVELLING);
        let mut passes = 0;
        while !session.warp_end_due() {
            warp_pass(&mut session, &catalog);
            passes += 1;
            assert_eq!(
                following_cloak(&session, escort),
                FULLY_CLOAKED,
                "pass {passes}"
            );
        }
        assert_eq!(passes, CLOAKED_BEFORE_THE_END);
        assert_eq!(
            session.arrive(&catalog, &mut NeverFires),
            Some(SystemId(131))
        );
        assert_cloak_kept(&session, escort, FULLY_CLOAKED);
    }

    /// By the original's formation flight
    /// (`RuleKey::FormationFlight`), an escort counter-thrusts onto its
    /// leader's velocity rather than taking it at once, and keeps within
    /// 300 of the player, the leader its substate is measured from.
    #[test]
    fn escorts_beside_a_weaving_player_converge_on_its_velocity_and_keep_within_300_of_it() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let ids: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        let off = Vec2::new(1.0, 0.0);
        for &id in &ids {
            let player = session.player().velocity;
            session.space.npc_mut(id).expect("there").state.velocity = player + off;
        }
        fly(&mut session, &catalog, weaving(0));
        for npc in escort_npcs(&session) {
            assert_ne!(
                npc.state.velocity,
                session.player().velocity,
                "not taken at once"
            );
        }
        let mut worst = 0.0_f32;
        for tick in 1..300 {
            fly(&mut session, &catalog, weaving(tick));
            let at = session.player().position;
            for npc in escort_npcs(&session) {
                let off = npc.state.position - at;
                worst = worst.max(off.x.abs()).max(off.y.abs());
            }
        }
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
            assert_eq!(npc.state, pushed_from_slot(&session, npc));
        }
        assert!(straying(&mut session, &catalog, 300) <= escort::KEEP_FORMATION);
    }

    #[test]
    fn landed_the_record_keeps_the_reserves_and_taking_off_restocks_them() {
        let catalog = crate::testkit::far_129_reachable(fleeted());
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(warship)
            .expect("there")
            .reserves
            .shield
            .now = 4.0;
        work(&mut session, &catalog, Controls::default());
        land_now(&mut session).expect("lands");
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        tick(&mut session, &catalog);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 4.0);
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert_eq!(session.pilot().escorts()[0].reserves.shield.now, 30.0);
        work(&mut session, &catalog, Controls::default());
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
        let warship = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(warship)
            .expect("there")
            .reserves
            .armor
            .now = 10.0;
        work(&mut session, &catalog, Controls::default());
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
        let lost = session.space.npc(warship).expect("still in the system");
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
        let freighter = session.space.escorts.fleet[1].expect("placed");
        session
            .space
            .npc_mut(freighter)
            .expect("there")
            .reserves
            .armor
            .now = 0.0;
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts().len(), 1);
        assert!(session.take_save_due());
        for _ in 0..3 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.space.npc(freighter), None, "destroyed and gone");
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
        let ids: Vec<NpcId> = session
            .space
            .escorts
            .fleet
            .iter()
            .flatten()
            .copied()
            .collect();
        assert_eq!(ids.len(), 3);
        for &id in &ids {
            assert_eq!(session.space.npc(id).expect("there").former, None);
            session.space.npc_mut(id).expect("there").reserves.armor.now = 10.0;
        }
        work(&mut session, &catalog, Controls::default());
        assert!(session.pilot().escorts().is_empty(), "all lost");
        for ((id, record), kind) in ids.into_iter().zip(&records).zip([
            FormerKind::Hired,
            FormerKind::Captured,
            FormerKind::Fighter,
        ]) {
            let npc = session.space.npc(id).expect("left behind");
            assert_eq!(npc.condition, Condition::Disabled);
            let former = npc.former.expect("remembers");
            assert_eq!(former.kind(), kind);
            // The fighter holds a fighter's capacities, 1.333 × its
            // class's, by `RuleKey::FighterCapacity`.
            let (shield_max, armor_max) = if record.carried {
                (39.99, 59.985)
            } else {
                (record.reserves.shield.max, 45.0)
            };
            assert_eq!(
                former.record,
                Escort {
                    reserves: Reserves {
                        shield: Gauge {
                            max: shield_max,
                            ..record.reserves.shield
                        },
                        armor: Gauge {
                            now: 10.0,
                            max: armor_max,
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
        let hired = session.space.escorts.fleet[0].expect("placed");
        let npc = session.space.npc_mut(hired).expect("there");
        npc.hull.death_delay = 10;
        npc.reserves.armor.now = 0.0;
        work(&mut session, &catalog, Controls::default());
        let npc = session.space.npc(hired).expect("breaking up");
        assert!(matches!(npc.condition, Condition::Dying { .. }));
        assert_eq!(
            npc.former.map(|former| former.kind()),
            Some(FormerKind::Hired)
        );
        let captured = session.space.escorts.fleet[0].expect("placed");
        session.space.traffic.remove(captured);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts().len(), 1, "the gone one is lost");
    }

    #[test]
    fn an_intact_escort_remembers_nothing() {
        let catalog = fleeted();
        let mut session = mixed(&catalog);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts().len(), 3);
        assert!(session.npcs().iter().all(|npc| npc.former.is_none()));
    }

    // The cargo a lost escort takes.

    /// Food, a commodity.
    pub(in crate::session) const FOOD: Good = Good::Commodity(0);
    /// Opals, a `jünk`.
    const OPALS: Good = Good::Junk(JunkId(146));

    /// A session of `fleet` by `source` of `LostCargoShare`, its first
    /// traffic tick done, holding 9 food and 3 opals.
    pub(in crate::session) fn laden(
        catalog: &FakePilotCatalog,
        fleet: Vec<Escort>,
        source: RuleSource,
    ) -> Session {
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
    pub(in crate::session) fn trader_fleet() -> Vec<Escort> {
        let full = Reserves::full(30.0, 45.0, 300.0);
        vec![
            escort(WARSHIP, full, None),
            escort(FREIGHTER, full, None),
            escort(FREIGHTER, full, None),
        ]
    }

    /// Escort `index`'s armour goes to `armor`, and the fight ticks.
    pub(in crate::session) fn hit(session: &mut Session, index: usize, armor: f32) {
        let id = session.space.escorts.fleet[index].expect("placed");
        session.space.npc_mut(id).expect("there").reserves.armor.now = armor;
        work(session, &catalog(), Controls::default());
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
    fn a_lost_trader_escort_holds_the_share_it_destroyed_overwriting_its_hold() {
        // `movw %dx, 0x7a(ship)` @0xcf76 writes all six, zeros included;
        // the opals, a `jünk`, go nowhere (@0xcf94-0xcfe4).
        let catalog = fleeted();
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        let id = session.space.escorts.fleet[1].expect("placed");
        session.space.npc_mut(id).expect("there").cargo = NpcHold::from_words([1, 2, 3, 4, 5, 6]);
        hit(&mut session, 1, 10.0);
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 5), (OPALS, 2)]));
        assert_eq!(
            session.space.npc(id).map(|npc| npc.cargo),
            Some(NpcHold::from_words([4, 0, 0, 0, 0, 0]))
        );
        let mut session = laden(&catalog, trader_fleet(), RuleSource::Engine);
        let warship = session.space.escorts.fleet[0].expect("placed");
        let hold = NpcHold::from_words([1, 2, 3, 4, 5, 6]);
        session.space.npc_mut(warship).expect("there").cargo = hold;
        hit(&mut session, 0, 10.0);
        assert_eq!(
            session.space.npc(warship).map(|npc| npc.cargo),
            Some(hold),
            "a warship takes no share"
        );
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
        assert_ne!(
            session.ship.latches.scan_latches,
            Latches::ARMED,
            "a warship: no call"
        );
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
        hit(&mut session, 0, 0.0);
        assert_eq!(session.ship.latches.scan_latches, Latches::ARMED, "@0xcff6");
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
        let id = session.space.escorts.fleet[1].expect("placed");
        session.space.traffic.remove(id);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts().len(), 2, "it is lost");
        assert_eq!(cargo(&session), BTreeMap::from([(FOOD, 9), (OPALS, 3)]));
    }

    #[test]
    fn a_trader_escort_a_deadly_stellar_kills_leaves_at_once_taking_its_share_only_when_destroyed()
    {
        let catalog = fleeted();
        let kept = BTreeMap::from([(FOOD, 9), (OPALS, 3)]);
        let shared = BTreeMap::from([(FOOD, 5), (OPALS, 2)]);
        for (reading, left) in [(RuleSource::Engine, kept), (RuleSource::Bible, shared)] {
            let session = laden(&catalog, trader_fleet(), RuleSource::Engine);
            let rules = session
                .setup
                .rules
                .with_override(RuleKey::DeadlyStellarKill, reading);
            let mut session = session.with_rules(rules);
            let id = session.space.escorts.fleet[1].expect("placed");
            let at = session.space.npc(id).expect("there").state.position;
            session.chart.sites = vec![crate::catalog::LandingSite {
                flags2: crate::deadly_stellar::DEADLY_STELLAR,
                frame_size: Some((2, 2)),
                ..crate::testkit::planet(160, at.x, at.y)
            }];
            session.chart.listed = session.chart.sites.clone();
            work(&mut session, &catalog, Controls::default());
            assert!(session.space.npc(id).is_none(), "{reading:?}");
            assert_eq!(
                session.pilot().escorts().len(),
                2,
                "{reading:?}: it is lost"
            );
            assert_eq!(cargo(&session), left, "{reading:?}");
            assert_eq!(session.ship.condition, Condition::Intact);
        }
    }

    /// A trader escort a hit disables on the tick a deadly stellar kills it
    /// takes its share of the cargo as it leaves, by either reading, as the
    /// hit's `_DamageShip` takes it (@0x3afdd, @0x3b013) before
    /// `_HandleDeadlyStellars` runs.
    #[test]
    fn a_trader_escort_disabled_on_the_tick_a_deadly_stellar_kills_it_takes_its_share() {
        use crate::session::deadly_stellars::tests::{Gunner, disabled_then_killed};
        let catalog = fleeted();
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let session = laden(&catalog, trader_fleet(), RuleSource::Engine);
            let rules = session
                .setup
                .rules
                .with_override(RuleKey::DeadlyStellarKill, reading);
            let mut session = session.with_rules(rules);
            let id = session.space.escorts.fleet[1].expect("placed");
            session.story_raised();
            disabled_then_killed(&mut session, id, &Gunner::Npc);
            assert!(session.space.npc(id).is_none(), "{reading:?}: killed");
            assert_eq!(session.pilot().escorts().len(), 2, "{reading:?}: lost");
            assert_eq!(
                cargo(&session),
                BTreeMap::from([(FOOD, 5), (OPALS, 2)]),
                "{reading:?}: its share"
            );
            assert!(
                session
                    .story_raised()
                    .contains(&crate::story::StoryEvent::ShipFate {
                        ship: id,
                        fate: crate::story::Fate::Disabled,
                    }),
                "{reading:?}"
            );
        }
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
        session.space.traffic.add_npc(npc)
    }

    /// A tick of the traffic, by `ai`, then of the fight.
    fn fight(session: &mut Session, catalog: &FakePilotCatalog, ai: &NovaAi) {
        work_with(
            session,
            catalog,
            Controls::default(),
            ai,
            Rules::default(),
            &mut NeverFires,
        );
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
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
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
                .space
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
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
            Some(Commanded {
                group: EscortGroup::All,
                command: EscortCommand::Attack,
                targeted: false,
                underway: false
            })
        );
        let mut session = flying(&catalog, RuleSource::Engine);
        let freighter = session.space.escorts.fleet[1].expect("placed");
        session.chart.target = Some(freighter);
        let commanded =
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires);
        assert_eq!(commanded.map(|commanded| commanded.targeted), Some(false));
        assert!(escort_npcs(&session).iter().all(|npc| npc.target.is_none()));
    }

    #[test]
    fn attack_again_with_a_new_target_retargets() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let first = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        let second = pirate(&mut session, 0.0, 400.0, Goal::Idle);
        session.chart.target = Some(first);
        session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
            None,
            "nothing changed"
        );
        session.chart.target = Some(second);
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires)
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
            session.command_escorts(warships, EscortCommand::Hold, &mut NeverFires),
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
        assert_eq!(warship.goal, Goal::Wait, "the hold's goal 6");
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
                .command_escorts(warships, EscortCommand::Recall, &mut NeverFires)
                .is_some()
        );
        assert_eq!(orders(&session), [None, None]);
        fight(&mut session, &catalog, &ai);
        assert!(matches!(
            escort_npcs(&session)[0].goal,
            Goal::Formation { .. }
        ));
        session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires);
        assert_eq!(orders(&session), [Some(EscortOrder::Hold); 2]);
        fight(&mut session, &catalog, &ai);
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.goal == Goal::Wait)
        );
        session.command_escorts(EscortGroup::All, EscortCommand::Recall, &mut NeverFires);
        assert_eq!(orders(&session), [None, None]);
    }

    #[test]
    fn a_command_that_changes_nothing_reports_nothing() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Recall, &mut NeverFires),
            None,
            "no orders to recall from"
        );
        assert!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires)
                .is_some()
        );
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires),
            None
        );
        let fighters = EscortGroup::Class(EscortClass::Fighter);
        let before = session.pilot().clone();
        assert_eq!(
            session.command_escorts(fighters, EscortCommand::Defend, &mut NeverFires),
            None
        );
        assert_eq!(session.pilot(), &before, "no fighters: nothing changes");
    }

    #[test]
    fn a_command_given_in_hyperspace_reaches_the_escorts_under_way() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Bible);
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session, &catalog);
        begin_jump_now(&mut session, &catalog).expect("jumps");
        session.take_sounds();
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires),
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
        assert_eq!(
            session.space.escorts.class_orders,
            [Some(EscortOrder::Hold); 4]
        );
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
        crate::testkit::fly_out(&mut session, &catalog);
        session.begin_jump().expect("pre-jump");
        assert_eq!(session.preparing_jump(), Some(SystemId(131)), "braking");
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Defend, &mut NeverFires),
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
            crate::testkit::fly_out(&mut session, &catalog);
            begin_jump_now(&mut session, &catalog).expect("jumps");
            session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires);
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
        session.chart.target = Some(quarry);
        assert_eq!(
            session
                .command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires)
                .map(|commanded| commanded.targeted),
            Some(true)
        );
        assert!(
            escort_npcs(&session)
                .iter()
                .all(|npc| npc.target == Some(ShipRef::Npc(quarry)))
        );
        session.plot_course(SystemId(131)).expect("a route");
        crate::testkit::fly_out(&mut session, &catalog);
        session.chart.target = Some(quarry);
        begin_jump_now(&mut session, &catalog).expect("jumps");
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
                session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
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
            work(&mut session, &catalog, Controls::default());
            assert!(
                escort_npcs(&session).iter().all(|npc| npc.target.is_none()),
                "let go every pass of the warp"
            );
        }
        session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
            None,
            "the warp lets go on its pass, not on a second command in the frame"
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
        let freighter = session.space.escorts.fleet[1].expect("placed");
        let launched = pirate(&mut session, 0.0, -400.0, Goal::Idle);
        session.space.npc_mut(launched).expect("here").leader = Some(freighter);
        session.chart.target = Some(launched);
        assert_eq!(
            session.command_escorts(EscortGroup::All, EscortCommand::Attack, &mut NeverFires),
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
            session.command_escorts(EscortGroup::All, EscortCommand::Hold, &mut NeverFires),
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
            session.space.escort_rows(&session.pilot, &session.statics),
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
            &mut NeverFires,
        );
        assert_eq!(
            session.space.escort_rows(&session.pilot, &session.statics)[3].order,
            Some(EscortOrder::Hold)
        );
        let landed = Session::fly(&catalog, escorted(&catalog)).expect("flies");
        assert_eq!(
            landed
                .space
                .escort_rows(&landed.pilot, &landed.statics)
                .map(|row| row.present),
            [false, false, true, true],
            "by their records before they are placed"
        );
    }

    // No standing command.

    #[test]
    fn an_escort_with_no_command_guards_the_player_from_formation_with_its_turret() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        session.command_escorts(EscortGroup::All, EscortCommand::Recall, &mut NeverFires);
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
        // `_AIFireTurret`'s pick, kept once the fight fired and cleared
        // its flag (`fire::after_firing`).
        assert_eq!(warship.trigger.keep, Some(TURRET));
        for _ in 0..90 {
            fight(&mut session, &catalog, &ai);
            let warship = escort_npcs(&session)[0];
            assert!(matches!(warship.goal, Goal::Formation { .. }));
            let off = slot_of(&session, warship) - warship.state.position;
            assert!(off.length() < 20.0, "in formation: {off:?}");
        }
        let shield = session
            .space
            .npc(raider)
            .expect("there")
            .reserves
            .shield
            .now;
        assert!(shield < 30.0, "{shield}");
    }

    #[test]
    fn an_escort_with_no_command_fires_its_turret_at_a_guard_500_pixels_off() {
        // `_AIEscortFireUnprovoked` tests no box (@0x80e4f-0x80ea8): a
        // turret reaching 600 fires at a raider 500 off.
        let mut catalog = fleeted();
        catalog.weapons[0].count = 60;
        let mut session = flying(&catalog, RuleSource::Engine);
        session.command_escorts(EscortGroup::All, EscortCommand::Recall, &mut NeverFires);
        let raider = pirate(&mut session, 0.0, -500.0, Goal::Attack(ShipRef::Player));
        let ai = sitting_ai();
        fight(&mut session, &catalog, &ai);
        let warship = escort_npcs(&session)[0];
        assert_eq!(
            warship.goal,
            Goal::Formation {
                guard: Some(ShipRef::Npc(raider))
            }
        );
        assert_eq!(warship.trigger.keep, Some(TURRET));
        for _ in 0..120 {
            fight(&mut session, &catalog, &ai);
        }
        let shield = session
            .space
            .npc(raider)
            .expect("there")
            .reserves
            .shield
            .now;
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
        let (warship, freighter) = (
            session.space.escorts.fleet[0],
            session.space.escorts.fleet[1],
        );
        assert_eq!(session.select_target(TargetPick::Next), Some(quarry));
        assert_eq!(session.select_target(TargetPick::Next), None);
        assert_eq!(session.select_target(TargetPick::NearestThreat), None);
        assert_eq!(session.select_target(TargetPick::NextEscort), warship);
        assert_eq!(session.select_target(TargetPick::NextEscort), freighter);
        tick(&mut session, &catalog);
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.target().map(|npc| npc.id), freighter, "kept");
        assert_eq!(session.select_target(TargetPick::NextEscort), None);
    }

    #[test]
    fn plain_r_picks_a_ship_attacking_a_hired_escort_but_never_the_escort() {
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let warship = session.space.escorts.fleet[0].expect("a hired escort");
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
        let escort = session.space.npc_mut(warship).expect("in the system");
        escort.goal = Goal::Attack(ShipRef::Player);
        escort.state.position = beside;
        assert_eq!(
            session.select_target(TargetPick::NearestThreat),
            Some(attacker),
            "the escort is passed over"
        );
        session.space.npc_mut(attacker).expect("there").goal =
            Goal::Attack(ShipRef::Npc(bystander));
        session.chart.target = None;
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
        let catalog = crate::testkit::far_129_reachable(with_ace());
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
        let ace = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(ace)
            .expect("there")
            .reserves
            .shield
            .now = 5.0;
        session.space.sync_fleet(&mut session.pilot);
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
        let ace = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(ace)
            .expect("there")
            .reserves
            .shield
            .now = 5.0;
        session.space.sync_fleet(&mut session.pilot);
        land_now(&mut session).expect("lands on 128, under the ship");
        session
            .take_off(&catalog, &mut NeverFires)
            .expect("takes off");
        assert!(!session.space.escorts.fleet.is_empty());
        for id in &session.space.escorts.fleet {
            let id = id.expect("placed");
            assert!(!before.contains(&id), "{id:?} is a new ship");
            assert!(session.space.npc(id).is_some(), "{id:?} is in the system");
        }
        assert_eq!(
            escort_npcs(&session)[0].reserves.shield.now,
            60.0,
            "restocked before it is placed"
        );
        assert!(
            !session.space.escorts.restock_persons,
            "the restock consumed"
        );
    }

    #[test]
    fn a_person_escort_keeps_its_reserves_through_a_jump() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let ace = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(ace)
            .expect("there")
            .reserves
            .shield
            .now = 5.0;
        session.space.sync_fleet(&mut session.pilot);
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
    fn a_holding_escort_that_leaves_the_fleet_takes_goal_0() {
        // Its leader gone, `_EscortAI` writes goal 0 and sub 0 for a ship
        // of AI type above 4 (@0x83917-0x8397b), which `_DamageShip`'s
        // leader clear (@0x3afdd) leaves it.
        let catalog = with_ace();
        for (goal, left) in [
            (Goal::Wait, Goal::Idle),
            (Goal::Attack(ShipRef::Player), Goal::Attack(ShipRef::Player)),
        ] {
            let mut session = flying_ace(&catalog);
            let ace = session.space.escorts.fleet[0].expect("placed");
            session.space.npc_mut(ace).expect("here").goal = goal;
            assert!(session.unfleet(ace));
            assert_eq!(session.space.npc(ace).expect("here").goal, left, "{goal:?}");
        }
    }

    #[test]
    fn a_person_escort_says_no_hail_quote() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        session.tick_quotes(&catalog, &mut crate::testkit::Draws::of(&[0]));
        assert_eq!(session.take_quotes(), []);
        let ace = session.space.escorts.fleet[0].expect("placed");
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
        let ace = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(ace)
            .expect("there")
            .reserves
            .armor
            .now = 0.0;
        for _ in 0..4 {
            work(&mut session, &catalog, Controls::default());
        }
        assert_eq!(session.pilot().escorts(), []);
        assert!(session.pilot().gone(PersonId(700)));
    }

    #[test]
    fn a_person_escort_disabled_or_released_stays_its_person_alive() {
        let catalog = with_ace();
        let mut session = flying_ace(&catalog);
        let ace = session.space.escorts.fleet[0].expect("placed");
        session
            .space
            .npc_mut(ace)
            .expect("there")
            .reserves
            .armor
            .now = 10.0;
        work(&mut session, &catalog, Controls::default());
        assert_eq!(session.pilot().escorts(), []);
        let left = session.space.npc(ace).expect("left behind");
        assert_eq!(left.condition, Condition::Disabled);
        assert_eq!(left.person.map(|person| person.id), Some(PersonId(700)));
        assert!(!session.pilot().gone(PersonId(700)));
        let mut session = flying_ace(&catalog);
        let ace = session.space.escorts.fleet[0].expect("placed");
        session.release(ace);
        assert_eq!(session.pilot().escorts(), []);
        let left = session.space.npc(ace).expect("leaving");
        assert_eq!(left.person.map(|person| person.id), Some(PersonId(700)));
        assert!(!session.pilot().gone(PersonId(700)));
    }

    #[test]
    fn a_player_escort_hit_by_an_npc_takes_it_as_its_target_and_one_hit_by_a_fleetmate_none() {
        // `_DamageShip`'s reaction block reads no AI type (@0x3bb7a-0x3bed2):
        // the escort (AI 5) takes its attacker as its target (@0x3bcf6),
        // once the answer flag lets the hit through; a fleetmate's hit
        // (same leader, @0x3b94e) it never does.
        let catalog = fleeted();
        let mut session = flying(&catalog, RuleSource::Engine);
        let escorts: Vec<NpcId> = escort_npcs(&session).iter().map(|npc| npc.id).collect();
        let mut pirate = session.space.npc(escorts[0]).expect("the escort").clone();
        pirate.escort = None;
        pirate.govt = Some(crate::catalog::GovtId(142));
        pirate.state.position.x += 200.0;
        pirate.target = Some(ShipRef::Npc(escorts[0]));
        pirate.goal = Goal::Attack(ShipRef::Npc(escorts[0]));
        let pirate = session.space.traffic.add_npc(pirate);
        let answer = |session: &mut Session, by: NpcId| {
            let mut strikes = vec![crate::combat::fixture::strike(
                ShipRef::Npc(escorts[0]),
                ShipRef::Npc(by),
            )];
            session.weigh_strikes(&mut strikes, &mut NeverFires);
            session.space.strikes = strikes;
            session.answer_strikes(&NovaAi::default());
            session.space.npc(escorts[0]).expect("the escort").target
        };
        assert_eq!(answer(&mut session, escorts[1]), None, "its fleetmate");
        assert_eq!(
            answer(&mut session, pirate),
            Some(ShipRef::Npc(pirate)),
            "the pirate"
        );
    }
}
