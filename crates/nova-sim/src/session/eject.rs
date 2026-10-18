//! The player's eject (`_HandlePlayer` 0x6f22c-0x6f96f; see
//! [`escape`](crate::escape)).
//!
//! **When it fires.** Only while the player is disabled or dying
//! (@0x6f22c), armed either by auto-eject or by the eject chord, Command +
//! the eject key held together ([`Session::hold_eject`], @0x6f29e-0x6f2c1),
//! which the session takes only while the player's fate takes
//! [`PlayerCommand::Eject`]. Auto-eject is armed only while dying and with
//! an auto-eject outfit ([`has_auto_eject`], @0x6f293), on the breakup's
//! first tick whatever its countdown, and later once the countdown, after
//! the tick's decrement, is at or below max(`DeathDelay` x 0.5, 30)
//! ([`auto_eject_armed`]). Armed, it then needs an escape pod or an escape
//! ship ([`has_escape_pod`], [`escape_ship_type`], @0x6f2d9), and a free
//! ship slot for the hulk, as `_GenericRandomShipSpawn` gives one
//! (@0x6f2f8-0x6f308, [`MAX_SHIPS_IN_SYSTEM`] counting the player): with
//! none the eject is cancelled and the player goes on dying.
//!
//! **Ordering against the original.** `_HandlePlayer` takes the death
//! timer down (@0x6d7e6), sounds the klaxon (@0x6f1e4) and runs the eject
//! (@0x6f22c), all before `_HandleShipDisplay` (@0x43e29) sets a new
//! breakup's timer or explodes a ship at the end of its countdown. The
//! engine's fight does both the decrement and the display step, so the
//! check runs twice in [`Session::tick_combat`]:
//!
//! - Before the fight, for a player who enters the tick disabled or dying
//!   ([`Session::eject_before_fight`]). The threshold reads the countdown
//!   less one, the original's timer after its decrement. A dying player
//!   who ejects here first sounds the klaxon when that is above 1, as the
//!   original does earlier in the same frame. The hulk takes the
//!   countdown as it stands, so the fight carries it on a tick: it explodes
//!   on the tick the player would have, even on the explosion tick itself.
//! - After the fight, on the breakup's first tick
//!   ([`Session::eject_on_breakup`]): the original's frame in which
//!   `_HandlePlayer` sees the ship dying with its timer not yet set, so
//!   auto-eject is armed whatever the threshold. The hulk is an NPC on its
//!   own breakup's first tick, its countdown the old class's `DeathDelay`
//!   (`_HandleShipDisplay` @0x444ad sets it for the NPC), and explodes
//!   `DeathDelay` - 2 ticks later. A breakup that stalls
//!   ([`Breakup::Stalls`](crate::combat::hull::Breakup::Stalls)) sets its
//!   first tick again every tick, re-arming it as the original's timer at
//!   or below 0 does.
//!
//! **The hand-off** (0x6f30e-0x6f40a). The hulk is an NPC of the old
//! class, with the player's position, velocity, heading, shield and
//! armour, no government (+0x88 = -1), boarded (+0xb9 = 1, so it can't be
//! boarded), on no escort or mission duty. A disabled player's hulk is a
//! disabled derelict; a dying one's breaks up and explodes. Its other
//! fields the original zeroes (+0x5c, +0x60, +0xb0, +0xc8d6, +0xc8e0,
//! +0xc8f4-+0xc8f8) are the ionisation, cloak-fade and display state
//! `_HandleShipDisplay` and `_IonizeShip` keep; the engine's NPC models
//! none of them, and the hulk is built new anyway. The player loses its
//! target, its nav target and its secondary weapon (+0x70, +0x6c, +0x72,
//! @0x6f40f-0x6f427), and the old class's `OnRetire` runs (@0x6f45c-0x6f49d).
//!
//! **The escape ship** (0x6f4b5-0x6f640), chosen after `OnRetire` has
//! run. The class becomes it, its shield, armour and fuel each
//! [`ESCAPE_FLOOR`] + a roll below [`ESCAPE_ROLL`] percent of the
//! class's raw values (the shield in points, as `shïp` +0x5c holds it),
//! with no outfit bonus, the pod countdown none (@0x6f563), and the comm
//! line says [`SimMessage::EscapingIn`] before the threat walk. The player
//! flies on.
//!
//! **The pod** (no escape ship, 0x6f642-0x6f677). The class becomes the
//! [`ESCAPE_POD_SHIP`], its shield and armour that class's full raw
//! values with no outfit bonus, its fuel untouched, and the pod countdown
//! [`POD_TICKS`]. Every NPC that targets the player lets it go, with no
//! threat test (@0x6f67d-0x6f6a9).
//!
//! **The threat walk** (0x6f6ab-0x6f747). Each NPC that threatens the
//! player ([`Npc::threatens_player_among`]) and targets it is pointed at
//! the hulk instead. In the pod, each other NPC the player leads is
//! released (+0x9a = -1, @0x6f71d-0x6f732): a hired or captured escort
//! and a launched fighter leave the fleet, and a mission escort the
//! formation, each deciding afresh. A pilot's escort with no NPC in the
//! system has no counterpart in the original, where every escort is a ship
//! in a slot: it is dropped with the rest.
//!
//! The walk's second half, a threat whose +0x6c is 0 pointed at the hulk
//! (@0x6f6ff-0x6f70d), is left out. For a ship +0x6c is the destination
//! the AI flies to: a stellar for the landing states (`_WarshipAI`
//! @0x8b90b), the leader or carrier for following and docking (states 10
//! and 5: `_EscortAI` @0x83dac, `_AIMakeFighterReturn` @0x82e4e), the
//! boarded ship for plundering (state 15, `_HighLevelAIHandler`
//! @0x8f74c). `_IsThreatToPlayer` excludes all of those states
//! (@0x7f536-0x7f56b), and the attack states a threat is in set +0x6c to
//! -1 (`_AIMakeShipAttackPlayer` @0x89d47, `_AIFightThreatToPlayer`
//! @0x81949), so a threat's +0x6c of 0 can only be a stellar index 0 left
//! over from an earlier state. The engine's goal keeps one ship for a
//! threat, its quarry, and no destination beside it, so there is nothing
//! for the half to move.
//!
//! **The new ship** (0x6f74d-0x6f969). Every outfit without `Flags`
//! 0x0004 is dropped (@0x6f7ae-0x6f7db), the class's stock weapons and
//! their ammunition fitted (@0x6f7dd-0x6f8b5) and its default outfits
//! added (@0x6f8c0-0x6f8fc), as a capture fits them. The original's
//! weapon rebuild sets each stock load on its ammunition's slot, a later
//! weapon of one ammunition replacing an earlier one's (@0x6f884,
//! @0x6f88e), where the engine's fit takes the largest load of an
//! ammunition; a fighter bay's load goes to its own fighters on both.
//! `_ShipStatsToSystemInfo` (@0x6f8bb) ends in a tail jump to
//! `_ResetPlayerPrecalcedValues` (@0xcce7), so the eject resets
//! (`Session::reset_precalced`: the latches, the sabotage clock, the trim
//! to the new class's holds and the blessing table; see the `precalced`
//! module), after the outfits are dropped and the stock weapons
//! fitted and before the default outfits are added (@0x6f8c0). The
//! engine boards the class in one step, so it refreshes after the
//! defaults too: an escape ship whose `shïp` defaults hold a sabotage
//! outfit would arm it at the eject, where the original arms it at the
//! next refresh. No stock class has one (no `shïp`'s defaults hold
//! `oütf` 261, 348 or 374, the only sabotage outfits). By
//! the engine's reading of [`RuleKey::EjectVelocity`], the x velocity
//! takes the y velocity's value (@0x6f782-0x6f788), so the hulk keeps the
//! true velocity; by the other, it is kept. Then one push of the new class's top
//! speed along the heading is added (`_Accel` @0x6f78e-0x6f7a9). The
//! original also sets +0x50 = 0 (@0x6f962), but a pre-jump has already
//! collapsed by then: `_HandlePlayer`'s collapse (0x688aa) runs before
//! the eject, so the engine's eject collapses one first, before the hulk
//! takes the player's state (see the `jump_collapse` module). The condition is
//! intact again (the death timer -1.0, @0x6f969), the engine glow goes
//! out, and [`SimSound::Ejected`] sounds, then a [`SimSound::Halt`] of
//! [`WARP_UP_HALTS`] (@0x6f920-0x6f95a).
//!
//! Nothing is saved: the original saves no pilot at an eject, so the
//! eject leaves whether a save is due as it found it, though the class
//! change, `OnRetire` and the reset's trim would make one due.
//!
//! **The sprite clones.** `_SWCloneFramesFromMaster` clones the old
//! class's frames into the hulk's slot once it is added (@0x6f3f5), and
//! the new class's, the pod's or the escape ship's, into the player's
//! slot once the class changes (@0x6f768): the
//! [`SlotFrames`](crate::ship_layer::SlotFrames) a hidden ship's hover
//! box is sized by (see [`Session::framed_by`]). A shown class's slot
//! clones the same frames again as the tick lays the ships out; a hidden
//! one (`Cost` -1) keeps these. The frame index each clone sets (frame
//! 0) is not modelled, as `SlotFrames` keeps only the sheet. No stock
//! class is hidden: stock `shïp` 895, the pod, costs 0, and no stock
//! `shïp` costs -1, so a stock player sees no difference.
//!
//! `_keyPressedThisFrame` (@0x6f2d3) and the status-redraw flags
//! (@0x6f433-0x6f442) have no counterpart.

use super::Session;
use super::hooks::ShipHook;
use super::player_led::slot_of;
use super::ship_change::ClassChange;
use crate::ai::Goal;
use crate::ai::pursuit::Pursuit;
use crate::bay::launch_velocity;
use crate::board::MAX_SHIPS_IN_SYSTEM;
use crate::combat::ion::Ion;
use std::collections::BTreeMap;

use crate::catalog::{OutfitId, ShipId, ShipRecord};
use crate::chance::Chance;
use crate::combat::armament::{Trigger, fit_stock, outfit_rounds};
use crate::combat::hull::{Condition, ESCAPE_POD_SHIP};
use crate::combat::{GamePass, ShipRef};
use crate::escape::{
    ESCAPE_FLOOR, ESCAPE_ROLL, auto_eject_armed, escape_ship_type, has_auto_eject, has_escape_pod,
};
use crate::message::SimMessage;
use crate::outfitter::OutfitFlags;
use crate::player_fate::{POD_TICKS, PlayerCommand};
use crate::reserves::shield_points;
use crate::rulebook::{RuleKey, RuleSource};
use crate::ship_change::{OutfitCarry, carried_outfits};
use crate::ship_layer::Slot;
use crate::sound::{SimSound, WARP_UP_HALTS};
use crate::traffic::npc::{AiType, Mode, Npc, NpcHold, NpcId};
use crate::traffic::table;

impl Session {
    /// Holds the eject chord, Command and the eject key down together, or
    /// lets it go: what the next fight tick's eject check reads. The
    /// session takes it only while the player's fate takes
    /// [`PlayerCommand::Eject`].
    pub fn hold_eject(&mut self, held: bool) {
        self.eject_held = held;
    }

    /// The eject check before the fight, for a player who enters the tick
    /// disabled or dying (see the module docs), drawing on `chance`.
    pub(super) fn eject_before_fight(&mut self, chance: &mut dyn Chance) {
        let (hulk, after) = match self.condition {
            Condition::Disabled => (Condition::Disabled, None),
            Condition::Dying { ticks_left, .. } => (
                Condition::Dying {
                    ticks_left,
                    starting: false,
                },
                Some(ticks_left.saturating_sub(1)),
            ),
            Condition::Intact | Condition::Destroyed => return,
        };
        let auto = after.is_some_and(|after| auto_eject_armed(after, self.hull.death_delay));
        let klaxon = after.is_some_and(|after| after > 1);
        // The pass's move has collapsed any pre-jump of a player disabled
        // or dying, so the collapse's clock is never read here.
        self.try_eject(auto, hulk, klaxon, false, chance);
    }

    /// The eject check after the fight on `pass`, on the breakup's first
    /// tick (see the module docs), drawing on `chance`. It runs the next
    /// pass's eject early, and with it that pass's collapse of a pre-jump
    /// still on, whose clock reads as the next pass's: double time's
    /// second pass after an advance pass in double time.
    pub(super) fn eject_on_breakup(&mut self, pass: GamePass, chance: &mut dyn Chance) {
        if let Condition::Dying { starting: true, .. } = self.condition {
            let hulk = Condition::Dying {
                ticks_left: self.hull.death_delay,
                starting: true,
            };
            let again = pass == GamePass::Advance { double_time: true };
            self.try_eject(true, hulk, false, again, chance);
        }
    }

    /// The escape ship the player can eject into now, if any
    /// ([`escape_ship_type`]): its fighter bays' rounds are the fighters
    /// aboard.
    fn escape_ship(&self) -> Option<ShipId> {
        let rounds = |weapon| outfit_rounds(&self.pilot.outfits, &self.ammo_outfits, weapon);
        escape_ship_type(self.arsenal.weapons(), rounds, &self.ships)
    }

    /// Ejects, leaving a hulk in `hulk`, when it is armed (`auto`, while
    /// auto-eject is, or the chord held) and the player can: an escape pod
    /// or ship, a free ship slot and a record of the class it flies. A
    /// `klaxon` sounds first. A pre-jump still on collapses as on double
    /// time's second pass when `again`. Whether it ejected.
    fn try_eject(
        &mut self,
        auto: bool,
        hulk: Condition,
        klaxon: bool,
        again: bool,
        chance: &mut dyn Chance,
    ) -> bool {
        let armed = (auto && has_auto_eject(&self.pilot.outfits, &self.outfits))
            || (self.eject_held && self.takes(PlayerCommand::Eject));
        if !armed
            || !(has_escape_pod(&self.pilot.outfits, &self.outfits) || self.escape_ship().is_some())
            || self.npcs().len() + 1 >= MAX_SHIPS_IN_SYSTEM
        {
            return false;
        }
        let Some(old) = self.ship_record(self.pilot.ship).cloned() else {
            return false;
        };
        if klaxon {
            self.sounds.push(SimSound::Klaxon);
        }
        self.eject(&old, hulk, again, chance);
        true
    }

    /// The eject itself, from a ship of class `old`, leaving a hulk in
    /// `hulk` (see the module docs), a pre-jump still on collapsing as on
    /// double time's second pass when `again`.
    fn eject(&mut self, old: &ShipRecord, hulk: Condition, again: bool, chance: &mut dyn Chance) {
        self.collapse_jump(again);
        let save_due = self.save_due;
        let hulk = self.traffic.add_npc(self.hulk(old, hulk));
        if let Some(slot) = self.npc(hulk).map(slot_of) {
            self.slot_sprites.frames.clone_frames(slot, old.id);
        }
        self.target = None;
        self.deselect();
        self.secondary = None;
        self.ship_hook(old.id, ShipHook::Retire, chance);
        let escape = self.escape_ship();
        let record = self.ship_record(escape.unwrap_or(ESCAPE_POD_SHIP)).cloned();
        if let Some(ship) = escape {
            self.pod = None;
            self.say(SimMessage::EscapingIn { ship });
        } else {
            self.pod = Some(POD_TICKS);
            self.drop_the_player();
        }
        self.walk_the_threats(hulk, escape.is_none());
        if let Some(record) = &record {
            let defaults = crate::pilot::tally(record.defaults.iter().copied());
            self.board_class(record, &defaults);
            self.slot_sprites
                .frames
                .clone_frames(Slot::Player, record.id);
            self.reset_precalced();
            let fields = record.fields;
            let shield = shield_points(fields.shield);
            let reserves = &mut self.pilot.reserves;
            if escape.is_some() {
                reserves.shield.now = escape_share(shield, chance);
                reserves.armor.now = escape_share(i64::from(fields.armor), chance);
                reserves.fuel.now = escape_share(i64::from(fields.fuel), chance);
            } else {
                reserves.shield.now = shield as f32;
                reserves.armor.now = f32::from(fields.armor);
            }
        }
        if self.rules.source_for(RuleKey::EjectVelocity) == RuleSource::Engine {
            let velocity = &mut self.player.velocity;
            velocity.x = velocity.y;
        }
        // Truncated to whole degrees (`cvttss2si` @0x6f7a0).
        #[allow(clippy::cast_possible_truncation)]
        let heading = self.player.heading as i32;
        self.player.velocity =
            self.player.velocity + crate::cargo_box::accel(heading, self.top_speed());
        self.condition = Condition::Intact;
        self.stop_thrust();
        self.sounds.push(SimSound::Ejected);
        self.halt_sounds(&WARP_UP_HALTS);
        self.save_due = save_due;
    }

    /// The escape pod's flight, a tick of it (`_HandlePlayer`
    /// @0x68d1d-0x68ddd): it thrusts along its heading at its full accel,
    /// gaining along each axis only while short of its top speed's share
    /// there (`_AdjustedAccel` @0x68d8d, [`launch_velocity`]: so a
    /// velocity above the top speed, as the eject's push may leave, is not
    /// cut back), moves by its velocity, and counts the pod down a tick.
    /// It never turns and its glow stays out: the thrust key's handler,
    /// like every other, comes after the pod's frames return (@0x68ddd),
    /// and so does the fuel's regeneration. On the tick the countdown
    /// reaches 0 the rescue is due, as the original runs it on that frame
    /// (see the `rescue` module). The eject's own tick sets the countdown
    /// and takes none off, the pod block coming before the eject.
    pub(super) fn pod_tick(&mut self) {
        let Some(left) = self.pod else {
            return;
        };
        let handling = self.flying_handling();
        // Whole degrees (`cvttss2si` @0x68d84).
        let heading = self.player.heading.trunc();
        let state = &mut self.player;
        state.velocity =
            launch_velocity(state.velocity, heading, handling.accel, handling.max_speed);
        state.position = state.position + state.velocity;
        let left = left.saturating_sub(1);
        self.pod = Some(left);
        if left == 0 {
            self.rescue_due = true;
        }
    }

    /// The hulk the player leaves in `condition`, a ship of class `old`
    /// (see the module docs).
    fn hulk(&self, old: &ShipRecord, condition: Condition) -> Npc {
        let kind = table::kind(old, &self.outfits, &self.arsenal);
        let mut reserves = kind.stats.full();
        reserves.shield = self.pilot.reserves.shield;
        reserves.armor = self.pilot.reserves.armor;
        Npc {
            id: NpcId::default(),
            berth: 0,
            ship: old.id,
            govt: None,
            ai_type: AiType::from_raw(kind.inherent_ai),
            leader: None,
            class: kind.escort_class,
            escort: None,
            stats: kind.stats,
            reserves,
            state: self.player,
            // The ship left as it was, its speed along its heading too.
            speed: self.player_speed,
            mode: Mode::Flying,
            goal: Goal::Idle,
            condition,
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
            boarded: true,
            info_types: 0,
            cargo: NpcHold::default(),
            spared: false,
            assisting: 0,
            carrier: None,
            person: None,
            mission: None,
            aux: None,
            cloak: crate::cloak::Cloak::default(),
            cloak_gear: kind.cloak,
            led_out: false,
            emerges_from: None,
            self_repair: kind.self_repair,
            immobile: kind.immobile,
            miner: kind.miner,
            can_scoop: kind.can_scoop,
            holds: kind.holds,
            class_fuel: kind.class_fuel,
            former: None,
            tractor: None,
            ion: Ion::default(),
            afterburner: false,
            pursuit: Pursuit::default(),
            ai_wait: 0,
            docking: None,
            marines: kind.marines,
            // `_GenericRandomShipSpawn` makes the hulk (@0x6f2f8), setting
            // its class's `PodCount` (@0x3ca48).
            pods_left: kind.hull.pod_count,
        }
    }

    /// Every NPC that targets, fights or inspects the player lets it go
    /// and decides again, as the pod's loop clears each +0x70 of 0
    /// (@0x6f67d-0x6f6a9).
    fn drop_the_player(&mut self) {
        let player = ShipRef::Player;
        for npc in self.traffic.npcs_mut() {
            if npc.target == Some(player) {
                npc.target = None;
            }
            if npc.goal.quarry() == Some(player) || npc.goal.inspecting() == Some(player) {
                npc.goal = Goal::Idle;
                npc.provoked = 0.0;
            }
        }
    }

    /// The threat walk (see the module docs): each threat to the player
    /// that targets it targets the `hulk` instead; in the `pod`, every
    /// other ship the player leads is released.
    fn walk_the_threats(&mut self, hulk: NpcId, pod: bool) {
        let to = ShipRef::Npc(hulk);
        for index in 0..self.traffic.npcs().len() {
            let npcs = self.traffic.npcs();
            let npc = &npcs[index];
            if npc.threatens_player_among(npcs) {
                let npc = &mut self.traffic.npcs_mut()[index];
                if npc.target == Some(ShipRef::Player) {
                    npc.target = Some(to);
                }
                npc.goal = retargeted(npc.goal, to);
            } else if pod && npc.escorts_player() {
                let id = npc.id;
                self.release_for_the_pod(id);
            }
        }
        if pod {
            self.drop_unplaced_escorts();
        }
    }

    /// Every escort of the pilot's with no NPC in the system leaves the
    /// fleet (see the module docs).
    fn drop_unplaced_escorts(&mut self) {
        let escorts = std::mem::take(&mut self.pilot.escorts);
        let fleet = std::mem::take(&mut self.fleet);
        let placed = fleet.into_iter().chain(std::iter::repeat(None));
        for (escort, placed) in escorts.into_iter().zip(placed) {
            if placed.is_some() {
                self.pilot.escorts.push(escort);
                self.fleet.push(placed);
            }
        }
    }

    /// NPC `id`, a ship the player leads, is released as the pod releases
    /// it (+0x9a = -1): out of the fleet or the formation, with no
    /// carrier, deciding afresh.
    fn release_for_the_pod(&mut self, id: NpcId) {
        self.unfleet(id);
        let inherent = self
            .npc(id)
            .and_then(|npc| self.ship_record(npc.ship))
            .map(|record| AiType::from_raw(record.inherent_ai));
        if let Some(npc) = self.npc_mut(id) {
            if let Some(mark) = &mut npc.mission
                && mark.formation.take().is_some()
                && let Some(ai_type) = inherent
            {
                npc.ai_type = ai_type;
            }
            npc.carrier = None;
            npc.goal = Goal::Idle;
        }
        self.reform();
    }

    /// The player's ship becomes class `record`, keeping only its
    /// persistent outfits, with `defaults` added and the class's stock
    /// weapons and ammunition fitted (see the module docs), holding its
    /// reserves to the new most until the caller sets them. The eject
    /// passes the class's defaults; the rescue, which adds none, passes
    /// none.
    pub(super) fn board_class(&mut self, record: &ShipRecord, defaults: &BTreeMap<OutfitId, u16>) {
        let mut outfits = carried_outfits(
            &self.pilot.outfits,
            OutfitCarry::Persistent {
                mask: OutfitFlags::PERSISTENT,
            },
            defaults,
            None,
            None,
            &self.outfits,
        );
        let fits = self.arsenal.stock_fits(record.id, &self.outfits);
        fit_stock(&mut outfits, &fits, &self.outfits);
        let change = ClassChange {
            outfits,
            reserves: super::ReservePolicy::Hold,
        };
        self.become_class(record, change, |_| {});
    }
}

/// An escape ship's share of its class's `value`: (`ESCAPE_FLOOR` + a roll
/// below `ESCAPE_ROLL`) percent of it, on `chance` (@0x6f4d0-0x6f583:
/// the integer product, times 0.01 at 0xdd098).
fn escape_share(value: i64, chance: &mut dyn Chance) -> f32 {
    let percent = ESCAPE_FLOOR + u32::from(chance.roll(ESCAPE_ROLL as u16));
    // Shield, armour and fuel are 16-bit fields (a shield at most 5 x
    // 32768), so the product and the share sit well within an `f32`.
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    let share = (value * i64::from(percent)) as f64 * 0.01;
    #[allow(clippy::cast_possible_truncation)]
    let share = share as f32;
    share
}

/// `goal` with its quarry, when that is the player, `to`.
fn retargeted(goal: Goal, to: ShipRef) -> Goal {
    match goal {
        Goal::Attack(ShipRef::Player) => Goal::Attack(to),
        Goal::Snipe(ShipRef::Player) => Goal::Snipe(to),
        Goal::Flee(ShipRef::Player) => Goal::Flee(to),
        Goal::Plunder {
            ship: ShipRef::Player,
            hopeless,
        } => Goal::Plunder { ship: to, hopeless },
        other => other,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::{OutfitId, StockWeapon, WeaponId};
    use crate::chance::NeverFires;
    use crate::combat::{CombatEvent, Rules};
    use crate::control::{Bit, Script};
    use crate::escape::{MOD_AUTO_EJECT, MOD_ESCAPE_POD};
    use crate::escort::{EscortDuty, FormationSlot};
    use crate::geometry::Vec2;
    use crate::handling::ShipFields;
    use crate::mission::ships::ShipBehav;
    use crate::pilot::Escort;
    use crate::player_fate::PlayerFate;
    use crate::reserves::Reserves;
    use crate::session::tests::{DISABLING, Firing, deadly_in, facing_an_npc};
    use crate::stats::ShipStats;
    use crate::testkit::{FAST, FakePilotCatalog, hull, npc, outfit, ship};
    use crate::traffic::npc::MissionMark;

    const POD: i16 = 188;
    const AUTO: i16 = 187;
    /// An outfit the eject carries over (`Flags` 0x0004).
    const KEPT: i16 = 300;
    /// An outfit the eject drops.
    const DROPPED: i16 = 301;
    /// The pod's default item.
    const POD_ITEM: i16 = 302;

    /// The pod's own shield and armour.
    const POD_SHIELD: i16 = 30;
    const POD_ARMOR: i16 = 40;

    /// `deadly_in`'s catalog (the player of `DeathDelay` `death_delay`,
    /// destroyed by one hit of its NPC's) with the escape pod and
    /// auto-eject outfits, records of the player's class and of the pod
    /// (30 shield, 40 armour, two of `POD_ITEM` aboard, a blaster
    /// mounted), and an `OnRetire` on the player's class that sets bit 17.
    fn ejecting(death_delay: i16) -> FakePilotCatalog {
        let mut catalog = deadly_in(death_delay);
        catalog.outfits.extend([
            outfit(POD, &[(MOD_ESCAPE_POD, 0)]),
            outfit(AUTO, &[(MOD_AUTO_EJECT, 0)]),
            OutfitRecordFlags::persistent(KEPT),
            outfit(DROPPED, &[]),
            outfit(POD_ITEM, &[]),
        ]);
        let mut old = ship(128, FAST);
        old.on_retire = Script::parse("b17");
        let mut pod = ship(
            ESCAPE_POD_SHIP.0,
            ShipFields {
                shield: POD_SHIELD,
                armor: POD_ARMOR,
                ..FAST
            },
        );
        pod.defaults = vec![(OutfitId(POD_ITEM), 2)];
        catalog.ship_records.extend([old, pod]);
        catalog.hulls.push(crate::catalog::HullRecord {
            weapons: vec![StockWeapon {
                weapon: WeaponId(128),
                count: 1,
                ammo: 0,
            }],
            ..hull(ESCAPE_POD_SHIP.0)
        });
        catalog
    }

    /// Builds outfit records with flags.
    struct OutfitRecordFlags;

    impl OutfitRecordFlags {
        fn persistent(id: i16) -> crate::catalog::OutfitRecord {
            crate::catalog::OutfitRecord {
                flags: OutfitFlags::PERSISTENT,
                ..outfit(id, &[])
            }
        }
    }

    /// `catalog`'s session facing its NPC, owning one of each of `outfits`.
    fn flying(catalog: &FakePilotCatalog, outfits: &[i16]) -> Session {
        let mut session = facing_an_npc(catalog, 180);
        for &id in outfits {
            session.pilot.outfits.insert(OutfitId(id), 1);
        }
        session
    }

    /// One tick under fire from `catalog`'s NPC, the chord held or not:
    /// the session's sounds from the fight.
    fn tick(session: &mut Session, catalog: &FakePilotCatalog, chord: bool) -> Vec<SimSound> {
        session.tick_traffic(catalog, &Firing, &mut NeverFires);
        session.take_sounds();
        session.hold_eject(chord);
        session.tick_combat(Rules::default(), &mut NeverFires);
        session.take_sounds()
    }

    /// Ticks under fire until the player's armour goes, giving that tick's
    /// sounds.
    fn to_the_breakup(
        session: &mut Session,
        catalog: &FakePilotCatalog,
        chord: bool,
    ) -> Vec<SimSound> {
        for _ in 0..200 {
            let sounds = tick(session, catalog, chord);
            if session.pilot.ship != ShipId(128)
                || !matches!(session.condition, Condition::Intact | Condition::Disabled)
            {
                return sounds;
            }
        }
        panic!("the player's armour never went");
    }

    /// A fight tick with no traffic tick, the chord held or not: its
    /// sounds and fight events.
    fn fight(session: &mut Session, chord: bool) -> (Vec<SimSound>, Vec<CombatEvent>) {
        session.take_sounds();
        session.take_combat_events();
        session.hold_eject(chord);
        session.tick_combat(Rules::default(), &mut NeverFires);
        (session.take_sounds(), session.take_combat_events())
    }

    const EJECTED: [SimSound; 2] = [SimSound::Ejected, SimSound::Halt(&WARP_UP_HALTS)];

    /// The beeps of "Ship disabled." and "Ship destroyed." as a hit breaks
    /// an intact player up (see the `player_downed` module).
    const DOWNED: [SimSound; 2] = [SimSound::ShipDowned, SimSound::ShipDowned];

    fn in_the_pod(session: &Session) -> bool {
        session.pilot.ship == ESCAPE_POD_SHIP && session.pod.is_some()
    }

    /// The NPC the eject left, the last added.
    fn hulk(session: &Session) -> &Npc {
        session.npcs().last().expect("a hulk")
    }

    /// Whether `events` destroy an NPC of the player's old class.
    fn hulk_destroyed(events: &[CombatEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                CombatEvent::Destroyed {
                    ship: ShipRef::Npc(_),
                    ship_type: ShipId(128),
                    ..
                }
            )
        })
    }

    /// Fills the system up to `MAX_SHIPS_IN_SYSTEM` with the player,
    /// each new NPC far out of the fight, giving the last added.
    fn fill(session: &mut Session) -> NpcId {
        let mut last = None;
        while session.npcs().len() + 1 < MAX_SHIPS_IN_SYSTEM {
            let mut far = npc(0, ShipStats::default());
            far.state.position = Vec2::new(5000.0, 5000.0);
            last = Some(session.traffic.add_npc(far));
        }
        last.expect("room to fill")
    }

    #[test]
    fn an_ejecting_player_keeps_its_ionization_and_its_hulk_has_none() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD, AUTO]);
        let ionized = Ion {
            charge: 40.0,
            colour: 0x00ff_0000,
        };
        session.ion = ionized;
        to_the_breakup(&mut session, &catalog, false);
        assert!(in_the_pod(&session));
        assert_eq!(session.ion(), ionized);
        assert_eq!(hulk(&session).ion, Ion::default());
    }

    #[test]
    fn the_players_hulk_carries_its_classs_crew_pods() {
        let mut catalog = ejecting(100);
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) {
                hull.pod_count = 3;
            }
        }
        let mut session = flying(&catalog, &[POD, AUTO]);
        to_the_breakup(&mut session, &catalog, false);
        assert!(in_the_pod(&session));
        assert_eq!(hulk(&session).pods_left, 3);
    }

    #[test]
    fn a_dying_hulk_of_a_pod_carrying_class_ejects_its_crew_pods() {
        let mut catalog = ejecting(100);
        for hull in &mut catalog.hulls {
            if hull.id == ShipId(128) {
                hull.pod_count = 3;
            }
        }
        let mut session = flying(&catalog, &[POD, AUTO]);
        to_the_breakup(&mut session, &catalog, false);
        let left = hulk(&session).id;
        let mut pods = 0;
        for _ in 0..400 {
            let (_, events) = fight(&mut session, false);
            pods += events
                .iter()
                .filter(|event| {
                    matches!(event, CombatEvent::PodEjected { ship, .. } if *ship == ShipRef::Npc(left))
                })
                .count();
            if session.npc(left).is_none() {
                break;
            }
        }
        assert!(session.npc(left).is_none(), "the hulk explodes");
        assert!((1..=3).contains(&pods), "{pods} of its 3 pods");
    }

    #[test]
    fn an_ejecting_players_turn_of_exits_goes_on_in_the_pod_by_the_engine_alone() {
        use crate::combat::exits::ExitKind;
        use crate::rulebook::Rulebook;
        for (reading, gun) in [(RuleSource::Engine, 1), (RuleSource::Bible, 0)] {
            let catalog = ejecting(100);
            let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
            let mut session = flying(&catalog, &[POD, AUTO]).with_rules(rules);
            session.armament.exit_cursors_mut().step(ExitKind::Gun);
            to_the_breakup(&mut session, &catalog, false);
            assert!(in_the_pod(&session));
            assert_eq!(
                session.armament.exit_cursors().next(ExitKind::Gun),
                gun,
                "{reading:?}"
            );
        }
    }

    #[test]
    fn an_ejecting_players_fold_set_goes_back_to_0_in_the_pod_by_either_reading() {
        use crate::rulebook::Rulebook;
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let catalog = ejecting(100);
            let rules = Rulebook::default().with_override(RuleKey::ReusedSlotExits, reading);
            let mut session = flying(&catalog, &[POD, AUTO]).with_rules(rules);
            session.armament = session.armament.clone().with_fold_set(2);
            to_the_breakup(&mut session, &catalog, false);
            assert!(in_the_pod(&session));
            assert_eq!(session.armament.fold_set(), 0, "{reading:?}");
        }
    }

    #[test]
    fn a_dying_player_without_a_pod_or_escape_ship_never_ejects() {
        let catalog = ejecting(2);
        let mut session = flying(&catalog, &[AUTO]);
        let mut sounds = to_the_breakup(&mut session, &catalog, true);
        while session.condition != Condition::Destroyed {
            sounds.extend(tick(&mut session, &catalog, true));
        }
        assert!(!sounds.contains(&SimSound::Ejected), "{sounds:?}");
        assert_eq!(session.pilot.ship, ShipId(128));
        assert_eq!(session.pod, None);
        assert_eq!(session.npcs().len(), 1, "no hulk");
    }

    #[test]
    fn a_pod_without_auto_eject_ejects_only_on_the_chord() {
        let catalog = ejecting(40);
        let mut session = flying(&catalog, &[POD]);
        let sounds = to_the_breakup(&mut session, &catalog, false);
        assert!(!sounds.contains(&SimSound::Ejected));
        for _ in 0..100 {
            tick(&mut session, &catalog, false);
        }
        assert_eq!(
            session.condition,
            Condition::Dying {
                ticks_left: 20,
                starting: false
            },
            "past the threshold, and no eject"
        );
        let sounds = tick(&mut session, &catalog, true);
        assert!(in_the_pod(&session));
        assert_eq!(sounds, [vec![SimSound::Klaxon], EJECTED.to_vec()].concat());
    }

    #[test]
    fn with_both_outfits_the_player_ejects_on_its_first_dying_tick_and_its_hulk_explodes_98_ticks_later()
     {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD, AUTO]);
        let sounds = to_the_breakup(&mut session, &catalog, false);
        assert_eq!(
            sounds,
            [DOWNED.as_slice(), EJECTED.as_slice()].concat(),
            "the downed lines' beeps, and no klaxon"
        );
        assert!(in_the_pod(&session));
        assert!(!matches!(session.condition, Condition::Dying { .. }));
        assert_eq!(
            session.player_fate(),
            PlayerFate::Ejected {
                ticks_left: POD_TICKS
            }
        );
        assert_eq!(
            hulk(&session).condition,
            Condition::Dying {
                ticks_left: 100,
                starting: true
            }
        );
        let mut ticks = 0;
        loop {
            ticks += 1;
            let (_, events) = fight(&mut session, false);
            if hulk_destroyed(&events) {
                break;
            }
            assert!(ticks < 300, "the hulk never explodes");
        }
        assert_eq!(ticks, 98);
        assert!(in_the_pod(&session), "the pod lives on");
    }

    #[test]
    fn the_chord_held_on_the_first_dying_tick_ejects_before_any_breakup() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD]);
        let sounds = to_the_breakup(&mut session, &catalog, true);
        assert_eq!(sounds, [DOWNED.as_slice(), EJECTED.as_slice()].concat());
        assert!(in_the_pod(&session));
        assert_eq!(
            hulk(&session).condition,
            Condition::Dying {
                ticks_left: 100,
                starting: true
            }
        );
    }

    /// With both outfits and no free slot on the first dying tick, the
    /// countdown of `death_delay` runs to `threshold` before the eject.
    fn waits_for_the_threshold(death_delay: i16, threshold: u32) {
        let catalog = ejecting(death_delay);
        let mut session = flying(&catalog, &[POD, AUTO]);
        let extra = fill(&mut session);
        let sounds = to_the_breakup(&mut session, &catalog, false);
        assert!(!sounds.contains(&SimSound::Ejected), "no slot");
        let three = u32::try_from(death_delay).expect("positive") * 3;
        assert_eq!(
            session.condition,
            Condition::Dying {
                ticks_left: three,
                starting: true
            },
            "the breakup runs"
        );
        session.traffic.remove(extra);
        let mut last = three;
        loop {
            let (sounds, _) = fight(&mut session, false);
            if in_the_pod(&session) {
                assert_eq!(
                    sounds,
                    [vec![SimSound::Klaxon], EJECTED.to_vec()].concat(),
                    "{death_delay}"
                );
                break;
            }
            let Condition::Dying { ticks_left, .. } = session.condition else {
                panic!("still dying: {:?}", session.condition);
            };
            assert!(ticks_left > threshold, "{death_delay}: {ticks_left}");
            last = ticks_left;
        }
        assert_eq!(last, threshold + 1, "{death_delay}: not before");
        assert_eq!(
            hulk(&session).condition,
            Condition::Dying {
                ticks_left: threshold,
                starting: false
            },
            "{death_delay}: the hulk carries on the player's countdown"
        );
    }

    #[test]
    fn a_first_tick_eject_with_no_free_slot_waits_for_the_threshold() {
        waits_for_the_threshold(100, 50);
        waits_for_the_threshold(40, 30);
    }

    #[test]
    fn the_chord_on_the_explosion_tick_still_saves_the_player_and_the_hulk_explodes() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD]);
        to_the_breakup(&mut session, &catalog, false);
        session.condition = Condition::Dying {
            ticks_left: 3,
            starting: false,
        };
        let (sounds, events) = fight(&mut session, true);
        assert!(in_the_pod(&session));
        assert_eq!(session.condition, Condition::Intact);
        assert!(hulk_destroyed(&events), "{events:?}");
        assert_eq!(sounds, [vec![SimSound::Klaxon], EJECTED.to_vec()].concat());
    }

    #[test]
    fn a_disabled_player_ejects_on_the_chord_only_and_command_or_the_key_alone_does_nothing() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD, AUTO]);
        session.condition = Condition::Disabled;
        for _ in 0..50 {
            session.hold_eject(false);
            session.tick_combat(DISABLING, &mut NeverFires);
        }
        assert_eq!(session.condition, Condition::Disabled, "no auto-eject");
        assert_eq!(session.pilot.ship, ShipId(128));
        session.take_sounds();
        session.hold_eject(true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(in_the_pod(&session));
        assert_eq!(session.take_sounds(), EJECTED, "no klaxon");
    }

    #[test]
    fn with_no_free_slot_the_eject_is_cancelled_and_the_player_goes_on_dying() {
        let catalog = ejecting(10);
        let mut session = flying(&catalog, &[POD]);
        fill(&mut session);
        let mut sounds = to_the_breakup(&mut session, &catalog, true);
        for _ in 0..10 {
            sounds.extend(fight(&mut session, true).0);
        }
        assert!(!sounds.contains(&SimSound::Ejected));
        assert_eq!(session.pilot.ship, ShipId(128));
        assert_eq!(
            session.condition,
            Condition::Dying {
                ticks_left: 20,
                starting: false
            }
        );
    }

    #[test]
    fn an_eject_hands_the_player_over_to_a_boarded_hulk() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD, KEPT, DROPPED]);
        let target = session.npcs()[0].id;
        session.target = Some(target);
        session.nav = crate::navigation::NavMode::Stellar(Some(crate::catalog::StellarId(128)));
        session.secondary = Some(WeaponId(128));
        let state = crate::flight::ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: Vec2::new(3.0, 4.0),
            heading: 90.0,
        };
        session.player = state;
        session.pilot.reserves.shield.now = 5.0;
        session.pilot.reserves.armor.now = 7.0;
        let fuel = session.pilot.reserves.fuel.now;
        let (shield, armor) = (session.pilot.reserves.shield, session.pilot.reserves.armor);
        session.condition = Condition::Disabled;
        let (sounds, _) = fight(&mut session, true);
        assert_eq!(sounds, EJECTED);
        let hulk = hulk(&session).clone();
        assert_eq!(hulk.ship, ShipId(128));
        assert_eq!(hulk.state, state);
        assert_eq!(hulk.condition, Condition::Disabled);
        assert_eq!((hulk.reserves.shield, hulk.reserves.armor), (shield, armor));
        assert!(hulk.boarded, "not to be boarded");
        assert_eq!((hulk.govt, hulk.escort, hulk.mission), (None, None, None));
        assert!(
            session.control_bit(Bit::new(17).expect("a bit")),
            "OnRetire"
        );
        assert_eq!(session.pilot.ship, ESCAPE_POD_SHIP);
        assert_eq!(session.player.position, state.position);
        assert_eq!(session.player.heading, 90.0);
        let push = session.top_speed();
        assert!(
            (session.player.velocity - Vec2::new(4.0 + push, 4.0)).length() < 1e-4,
            "x takes y, then a push east: {:?}",
            session.player.velocity
        );
        assert_eq!(session.target, None);
        assert_eq!(session.nav_target(), None);
        assert_eq!(session.secondary, None);
        assert_eq!(session.condition, Condition::Intact);
        assert_eq!(session.pilot.reserves.fuel.now, fuel, "fuel untouched");
        let owned: Vec<(i16, u16)> = session
            .pilot
            .outfits
            .iter()
            .map(|(id, &n)| (id.0, n))
            .collect();
        assert_eq!(
            owned,
            [(250, 1), (KEPT, 1), (POD_ITEM, 2)],
            "the stock blaster, the persistent outfit and the pod's items"
        );
        let mounted: Vec<(WeaponId, u32)> = session
            .armament
            .mounts()
            .iter()
            .map(|mount| (mount.spec.id, mount.count))
            .collect();
        assert_eq!(mounted, [(WeaponId(128), 1)]);
    }

    #[test]
    fn the_hulk_of_a_class_of_no_speed_and_no_accel_is_immobile_and_stays_put() {
        let left = |fields: ShipFields| {
            let mut catalog = ejecting(100);
            for record in &mut catalog.ship_records {
                if record.id == ShipId(128) {
                    record.fields = ShipFields {
                        speed: fields.speed,
                        accel: fields.accel,
                        ..record.fields
                    };
                }
            }
            let mut session = flying(&catalog, &[POD]);
            let at = Vec2::new(10.0, 20.0);
            session.player = crate::flight::ShipState {
                position: at,
                velocity: Vec2::new(3.0, 4.0),
                heading: 90.0,
            };
            session.condition = Condition::Disabled;
            assert_eq!(fight(&mut session, true).0, EJECTED);
            let immobile = hulk(&session).immobile;
            for _ in 0..3 {
                session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
            }
            (immobile, at, hulk(&session).state)
        };
        let (immobile, at, state) = left(ShipFields {
            speed: 0,
            accel: 0,
            ..FAST
        });
        assert!(immobile, "its class's");
        assert_eq!(state.position, at, "the player's drift moves it not at all");
        assert_eq!(state.velocity, Vec2::ZERO, "and lasts only until its tick");
        let (immobile, at, state) = left(FAST);
        assert!(!immobile);
        assert_ne!(state.position, at, "it drifts on");
    }

    #[test]
    fn the_hulk_of_an_inertialess_class_keeps_the_players_speed_and_flies_on_along_its_heading() {
        let mut catalog = ejecting(100);
        for record in &mut catalog.ship_records {
            if record.id == ShipId(128) {
                record.fields.flags2 = crate::stats::INERTIALESS_HULL;
            }
        }
        let mut session = flying(&catalog, &[POD]);
        let heading = 90.0;
        session.player = crate::flight::ShipState {
            position: Vec2::new(10.0, 20.0),
            velocity: crate::flight::facing(heading) * 4.0,
            heading,
        };
        session.player_speed = 4.0;
        session.condition = Condition::Disabled;
        session.pilot.reserves.armor.now = 1.0;
        assert_eq!(fight(&mut session, true).0, EJECTED);
        let id = hulk(&session).id;
        assert_eq!(hulk(&session).speed, 4.0, "the player's speed");
        for _ in 0..3 {
            session.tick_traffic(&catalog, &crate::ai::Peaceful, &mut NeverFires);
            let hulk = session.npc(id).expect("still here");
            assert_eq!(hulk.condition, Condition::Disabled, "a derelict");
            let along = crate::flight::facing(hulk.state.heading) * hulk.speed;
            assert!(hulk.speed > 3.9, "slowed only by the drag: {}", hulk.speed);
            assert!(
                (hulk.state.velocity - along).length() < 1e-4,
                "along its heading at its speed: {:?} at {}",
                hulk.state,
                hulk.speed
            );
        }
    }

    /// A braking pre-jump to 131.
    const BRAKING: crate::session::Jump = crate::session::Jump::PreJump {
        to: crate::catalog::SystemId(131),
        bearing: None,
        warp: None,
    };

    #[test]
    fn a_player_killed_outright_mid_jump_collapses_before_it_ejects() {
        use crate::flight::Controls;
        use crate::session::jump_collapse::collapsed_velocity;
        let mut catalog = ejecting(100);
        for record in &mut catalog.ship_records {
            if record.id == ShipId(128) {
                record.flags = 0x0004;
            }
        }
        let mut session = flying(&catalog, &[POD, AUTO]);
        session.warp_sounds[0] = 9;
        session.jump = Some(BRAKING);
        session.take_messages();
        let sounds = to_the_breakup(&mut session, &catalog, false);
        assert!(in_the_pod(&session));
        assert_eq!(
            sounds,
            [
                SimSound::ShipDowned,
                SimSound::ShipDowned,
                SimSound::Halt(&WARP_UP_HALTS),
                SimSound::JumpCollapsed,
                SimSound::Ejected,
                SimSound::Halt(&WARP_UP_HALTS),
            ]
        );
        assert_eq!(
            session.take_messages(),
            [
                SimMessage::ShipDisabled,
                SimMessage::ShipDestroyed,
                SimMessage::JumpCollapsed
            ]
        );
        assert_eq!(session.jump, None);
        // C is the old class's (2.08, `Flags` 0x0004), not the pod's.
        let speed = crate::warp::lead_in_speed(2, crate::warp::warp_factor(0x0004), 9);
        assert_ne!(
            speed,
            crate::warp::lead_in_speed(2, crate::warp::warp_factor(0), 9)
        );
        assert!(speed > 0.0, "{speed}");
        let heading = session.player.heading;
        let collapsed = collapsed_velocity(Vec2::ZERO, heading, speed, session.top_speed());
        assert_eq!(
            hulk(&session).state.velocity,
            collapsed,
            "the hulk keeps it"
        );
        session.tick(Controls::default());
        assert_eq!(session.take_messages(), [], "collapsed once");

        // An escape ship's notice follows the collapse's.
        let catalog = escaping();
        let (mut session, sounds) = escaped(&catalog, &[], |session| {
            session.warp_sounds[0] = 9;
            session.jump = Some(BRAKING);
        });
        assert_eq!(
            session.take_messages(),
            [
                SimMessage::JumpCollapsed,
                SimMessage::EscapingIn {
                    ship: ShipId(VIPER)
                }
            ]
        );
        assert_eq!(
            sounds,
            [
                SimSound::Halt(&WARP_UP_HALTS),
                SimSound::JumpCollapsed,
                SimSound::Ejected,
                SimSound::Halt(&WARP_UP_HALTS),
            ]
        );
    }

    #[test]
    fn in_double_time_a_collapse_on_the_advance_passs_breakup_reads_the_second_passs_clock() {
        // The eject on the breakup runs the next pass's eject early, and
        // with it that pass's collapse: in double time the second pass, on
        // which a braking ship's stamp is the frame's own, so t = 0 and the
        // collapse gives no speed, where the next frame's reads 2.
        let mut catalog = ejecting(100);
        for record in &mut catalog.ship_records {
            if record.id == ShipId(128) {
                record.flags = 0x0004;
            }
        }
        let w = crate::warp::warp_factor(0x0004);
        assert!(crate::warp::lead_in_speed(0, w, 9) < 0.0);
        assert!(crate::warp::lead_in_speed(2, w, 9) > 0.0);
        let mut session = flying(&catalog, &[POD, AUTO]);
        session.set_double_time_key(true);
        session.warp_sounds[1] = 9;
        session.jump = Some(BRAKING);
        to_the_breakup(&mut session, &catalog, false);
        assert!(in_the_pod(&session));
        assert_eq!(session.jump, None, "collapsed");
        assert_eq!(hulk(&session).state.velocity, Vec2::ZERO, "no speed");
    }

    #[test]
    fn a_dying_players_hulk_explodes_and_a_disabled_ones_stays_a_derelict() {
        let catalog = ejecting(10);
        let mut session = flying(&catalog, &[POD, AUTO]);
        to_the_breakup(&mut session, &catalog, false);
        assert!(in_the_pod(&session));
        let exploded = (0..20).any(|_| hulk_destroyed(&fight(&mut session, false).1));
        assert!(exploded, "the dying player's hulk explodes");
        let mut session = flying(&catalog, &[POD]);
        session.condition = Condition::Disabled;
        session.pilot.reserves.armor.now = 1.0;
        fight(&mut session, true);
        assert!(in_the_pod(&session));
        let id = hulk(&session).id;
        for _ in 0..200 {
            let (_, events) = fight(&mut session, false);
            assert!(!hulk_destroyed(&events));
        }
        let derelict = session.npc(id).expect("still here");
        assert_eq!(derelict.condition, Condition::Disabled);
    }

    /// An NPC of `goal`, targeting `target`.
    fn aimed(session: &mut Session, goal: Goal, target: Option<ShipRef>) -> NpcId {
        session.traffic.add_npc(Npc {
            goal,
            target,
            ..npc(0, ShipStats::default())
        })
    }

    /// An escort hired into the fleet, as placed.
    fn escort(session: &mut Session) -> NpcId {
        let id = session.traffic.add_npc(Npc {
            escort: Some(EscortDuty {
                formation: FormationSlot::default(),
                order: None,
            }),
            goal: Goal::Formation { guard: None },
            ..npc(0, ShipStats::default())
        });
        session.pilot.escorts.push(hired());
        session.fleet.push(Some(id));
        id
    }

    fn hired() -> Escort {
        Escort {
            ship: ShipId(129),
            reserves: Reserves::full(1.0, 1.0, 1.0),
            order: None,
            carried: false,
            wage: None,
            person: None,
        }
    }

    #[test]
    fn the_pod_branch_makes_ship_895_full_in_ejected_350_clears_targets_and_releases_non_threat_escorts()
     {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD]);
        let attacker = aimed(
            &mut session,
            Goal::Attack(ShipRef::Player),
            Some(ShipRef::Player),
        );
        session.npc_mut(attacker).expect("here").provoked = 5.0;
        let aiming = aimed(&mut session, Goal::Idle, Some(ShipRef::Player));
        let inspector = aimed(&mut session, Goal::Inspect(ShipRef::Player), None);
        session.npc_mut(inspector).expect("here").provoked = 3.0;
        let hired = escort(&mut session);
        let raider = aimed(
            &mut session,
            Goal::Attack(ShipRef::Npc(hired)),
            Some(ShipRef::Npc(hired)),
        );
        session.npc_mut(raider).expect("here").provoked = 2.0;
        let mission = session.traffic.add_npc(Npc {
            mission: Some(MissionMark {
                slot: 0,
                behav: ShipBehav::EscortPlayer,
                formation: Some(FormationSlot::default()),
                order: None,
            }),
            ..npc(0, ShipStats::default())
        });
        // A pilot's escort not placed in the system.
        session.pilot.escorts.push(super::tests::hired());
        session.fleet.push(None);
        let fuel = session.pilot.reserves.fuel.now;
        session.condition = Condition::Disabled;
        fight(&mut session, true);
        assert_eq!(session.pilot.ship, ESCAPE_POD_SHIP);
        assert_eq!(
            session.player_fate(),
            PlayerFate::Ejected {
                ticks_left: POD_TICKS
            }
        );
        let reserves = session.pilot.reserves;
        assert_eq!(
            (reserves.shield.now, reserves.armor.now, reserves.fuel.now),
            (f32::from(POD_SHIELD), f32::from(POD_ARMOR), fuel),
            "full, with no outfit bonus, fuel untouched"
        );
        let npc = |id| session.npc(id).expect("here").clone();
        assert_eq!(
            (
                npc(attacker).target,
                npc(attacker).goal,
                npc(attacker).provoked
            ),
            (None, Goal::Idle, 0.0)
        );
        assert_eq!(npc(aiming).target, None);
        assert_eq!(
            (npc(inspector).goal, npc(inspector).provoked),
            (Goal::Idle, 0.0),
            "the inspection given up"
        );
        assert_eq!(
            (npc(raider).target, npc(raider).goal, npc(raider).provoked),
            (
                Some(ShipRef::Npc(hired)),
                Goal::Attack(ShipRef::Npc(hired)),
                2.0
            ),
            "a threat by a led ship keeps its quarry"
        );
        assert_eq!(npc(hired).escort, None, "released");
        assert!(!npc(hired).escorts_player());
        assert!(!npc(mission).escorts_player(), "out of the formation");
        assert_eq!(session.pilot.escorts, [], "the unplaced escort dropped");
        assert_eq!(session.fleet, []);
    }

    #[test]
    fn an_eject_makes_no_save_due() {
        let catalog = ejecting(100);
        let mut session = flying(&catalog, &[POD]);
        session.take_save_due();
        session.condition = Condition::Disabled;
        fight(&mut session, true);
        assert!(in_the_pod(&session));
        assert!(!session.take_save_due());
    }

    /// The escape ship's bay, its fighters' outfit and the fighter.
    const BAY: i16 = 160;
    const FIGHTERS: i16 = 310;
    const VIPER: i16 = 144;

    /// `ejecting(100)` with a fighter bay of `VIPER`s, an escape ship
    /// (`Flags` 0x8000) of 100 shield, 200 armour and 300 fuel, long
    /// named "Federation Viper".
    fn escaping() -> FakePilotCatalog {
        let mut catalog = ejecting(100);
        catalog.weapons.push(crate::catalog::WeaponRecord {
            guidance: crate::bay::FIGHTER_BAY,
            ammo_type: VIPER,
            ..crate::testkit::weapon(BAY)
        });
        catalog.outfits.push(outfit(
            FIGHTERS,
            &[(crate::combat::armament::MOD_AMMO, BAY)],
        ));
        let mut viper = ship(
            VIPER,
            ShipFields {
                shield: 100,
                armor: 200,
                fuel: 300,
                ..FAST
            },
        );
        viper.flags = crate::escape::ESCAPE_SHIP_FLAG;
        viper.long_name = "Federation Viper".to_owned();
        catalog.ship_records.push(viper);
        catalog
    }

    /// `escaping`'s session, disabled with two fighters aboard, ejected
    /// on the chord drawing each roll on `rolls`: its sounds.
    fn escaped(
        catalog: &FakePilotCatalog,
        rolls: &[u16],
        prepare: impl FnOnce(&mut Session),
    ) -> (Session, Vec<SimSound>) {
        let mut session = flying(catalog, &[]);
        session.pilot.outfits.insert(OutfitId(FIGHTERS), 2);
        session.condition = Condition::Disabled;
        prepare(&mut session);
        session.take_sounds();
        session.take_messages();
        session.hold_eject(true);
        session.tick_combat(
            Rules::default(),
            &mut crate::testkit::Scripted::rolling(rolls),
        );
        let sounds = session.take_sounds();
        (session, sounds)
    }

    #[test]
    fn an_escape_bay_makes_the_player_its_fighter_at_50_to_79_percent() {
        let catalog = escaping();
        for (roll, percent) in [(0, 50.0), (29, 79.0)] {
            let (session, sounds) = escaped(&catalog, &[roll; 3], |_| {});
            assert_eq!(sounds, EJECTED);
            assert_eq!(session.pilot.ship, ShipId(VIPER));
            let reserves = session.pilot.reserves;
            assert_eq!(
                (reserves.shield.now, reserves.armor.now, reserves.fuel.now),
                (percent, percent * 2.0, percent * 3.0),
                "roll {roll}"
            );
            assert_eq!(session.pod, None);
            assert_eq!(hulk(&session).ship, ShipId(128));
        }
    }

    #[test]
    fn a_kill_outfit_with_auto_eject_ejects_on_the_firing_tick() {
        use crate::sabotage::{FUSE, SABOTAGE_KILL};
        const KILLER: i16 = 303;
        let mut catalog = ejecting(100);
        catalog.outfits.push(outfit(KILLER, &[(SABOTAGE_KILL, 0)]));
        let mut session = flying(&catalog, &[POD, AUTO, KILLER]);
        session.reset_precalced();
        fight(&mut session, false);
        session.sabotage.set_counter(FUSE);
        let (sounds, _) = fight(&mut session, false);
        assert!(in_the_pod(&session), "{:?}", session.condition);
        assert_eq!(
            sounds,
            [
                SimSound::OnboardExplosion,
                SimSound::Halt(&WARP_UP_HALTS),
                SimSound::Ejected,
                SimSound::Halt(&WARP_UP_HALTS),
            ]
        );
    }

    #[test]
    fn an_escape_ship_eject_rederives_the_flag_from_the_outfits_it_keeps() {
        use crate::sabotage::{SABOTAGE_KILL, Sabotage};
        use crate::testkit::Draws;
        const KILLER: i16 = 303;
        const KEPT_KILLER: i16 = 304;
        let mut catalog = escaping();
        catalog.outfits.extend([
            outfit(KILLER, &[(SABOTAGE_KILL, 0)]),
            crate::catalog::OutfitRecord {
                flags: OutfitFlags::PERSISTENT,
                ..outfit(KEPT_KILLER, &[(SABOTAGE_KILL, 0)])
            },
        ]);
        for (id, kept) in [(KILLER, false), (KEPT_KILLER, true)] {
            let (mut session, _) = escaped(&catalog, &[0; 3], |session| {
                session.pilot.outfits.insert(OutfitId(id), 1);
                session.reset_precalced();
            });
            assert_eq!(session.pilot.ship, ShipId(VIPER), "{id}");
            // Seeded 99 on the eject's tick, and counted while disabled;
            // the old class's `OnRetire` resets as it ends (@0x99dea), the
            // outfit still owned, which zeroes the count.
            if kept {
                assert_eq!(session.sabotage.flag(), Sabotage::Kill);
                assert_eq!(session.sabotage.counter(), 0.0, "zeroed, reseeding");
            } else {
                assert_eq!(session.sabotage.flag(), Sabotage::None, "@0x6f8bb");
                assert_eq!(
                    session.sabotage.counter(),
                    0.0,
                    "left as `OnRetire`'s reset left it"
                );
            }
            session.tick_combat(Rules::default(), &mut Draws::of(&[7]));
            let counter = if kept { 8.0 } else { 0.0 };
            assert_eq!(session.sabotage.counter(), counter, "{id}");
        }
    }

    #[test]
    fn the_escape_ship_says_escaping_in_its_long_name() {
        let catalog = escaping();
        let (mut session, _) = escaped(&catalog, &[], |_| {});
        assert_eq!(
            session.take_messages(),
            [crate::message::SimMessage::EscapingIn {
                ship: ShipId(VIPER)
            }]
        );
        assert_eq!(
            session.ship_long_name(ShipId(VIPER)),
            Some("Federation Viper")
        );
        assert_eq!(session.ship_long_name(ShipId(999)), None);
    }

    #[test]
    fn escape_ship_threats_retarget_onto_the_hulk_and_led_ships_stay() {
        let catalog = escaping();
        let mut ids = None;
        let (session, _) = escaped(&catalog, &[], |session| {
            let attacker = aimed(
                session,
                Goal::Attack(ShipRef::Player),
                Some(ShipRef::Player),
            );
            let sniper = aimed(session, Goal::Snipe(ShipRef::Player), Some(ShipRef::Player));
            let fleeing = aimed(session, Goal::Flee(ShipRef::Player), None);
            let hired = escort(session);
            let plundering = Goal::Plunder {
                ship: ShipRef::Player,
                hopeless: true,
            };
            let pirate = aimed(session, plundering, Some(ShipRef::Player));
            ids = Some((attacker, sniper, fleeing, hired, pirate));
        });
        let (attacker, sniper, fleeing, hired, pirate) = ids.expect("set up");
        let hulk = ShipRef::Npc(hulk(&session).id);
        let npc = |id| session.npc(id).expect("here");
        assert_eq!(
            (npc(attacker).target, npc(attacker).goal),
            (Some(hulk), Goal::Attack(hulk))
        );
        assert_eq!(
            (npc(sniper).target, npc(sniper).goal),
            (Some(hulk), Goal::Snipe(hulk))
        );
        assert_eq!(
            (npc(fleeing).target, npc(fleeing).goal),
            (None, Goal::Flee(hulk))
        );
        assert_eq!(
            (npc(pirate).target, npc(pirate).goal),
            (
                Some(hulk),
                Goal::Plunder {
                    ship: hulk,
                    hopeless: true
                }
            ),
            "goal 0xd kept, +0x70 the hulk"
        );
        assert!(session.npc(hired).expect("here").escorts_player(), "kept");
        assert_eq!(session.fleet, [Some(hired)]);
    }

    #[test]
    fn an_escape_ship_flies_on() {
        let (session, _) = escaped(&escaping(), &[], |_| {});
        assert_eq!(session.player_fate(), PlayerFate::Flying);
    }

    #[test]
    fn the_eject_takes_the_y_velocity_as_x_by_the_engine_and_keeps_it_by_the_other_reading() {
        let catalog = ejecting(100);
        for (source, x) in [
            (crate::rulebook::RuleSource::Engine, 4.0),
            (crate::rulebook::RuleSource::Bible, 3.0),
        ] {
            let rules = crate::rulebook::Rulebook::default()
                .with_override(crate::rulebook::RuleKey::EjectVelocity, source);
            let mut session = flying(&catalog, &[POD]).with_rules(rules);
            session.player.velocity = Vec2::new(3.0, 4.0);
            session.player.heading = 90.0;
            session.condition = Condition::Disabled;
            fight(&mut session, true);
            let push = session.top_speed();
            assert!(
                (session.player.velocity - Vec2::new(x + push, 4.0)).length() < 1e-4,
                "{source:?}: {:?}",
                session.player.velocity
            );
        }
    }

    /// `ejecting(100)`'s session ejected into the pod, disabled, heading
    /// `heading` at rest.
    fn in_a_pod(catalog: &FakePilotCatalog, heading: f32) -> Session {
        let mut session = flying(catalog, &[POD]);
        session.player.heading = heading;
        session.player.velocity = Vec2::ZERO;
        session.condition = Condition::Disabled;
        session.pilot.reserves.armor.now = 1.0;
        fight(&mut session, true);
        assert!(in_the_pod(&session));
        session.player.velocity = Vec2::ZERO;
        session.take_sounds();
        session.take_combat_events();
        session
    }

    #[test]
    fn the_pod_thrusts_along_its_heading_and_ignores_controls_and_fire() {
        let catalog = ejecting(100);
        let mut session = in_a_pod(&catalog, 90.0);
        let handling = session.flying_handling();
        assert!(handling.accel > 0.0 && handling.max_speed > handling.accel);
        let controls = crate::flight::Controls {
            thrust: false,
            turn: crate::flight::Turn::Left,
            reverse: true,
            ..crate::flight::Controls::default()
        };
        let at = session.player.position;
        session.tick(controls);
        assert_eq!(session.player.heading, 90.0, "no turning");
        assert!(
            (session.player.velocity - Vec2::new(handling.accel, 0.0)).length() < 1e-6,
            "{:?}",
            session.player.velocity
        );
        assert_eq!(session.player.position, at + session.player.velocity);
        let mut events = Vec::new();
        for _ in 0..60 {
            session.hold_fire(true, true);
            session.tick(controls);
            session.tick_combat(Rules::default(), &mut NeverFires);
            events.extend(session.take_combat_events());
        }
        let speed = session.player.velocity;
        assert!(
            (speed.x - handling.max_speed).abs() <= handling.accel && speed.y.abs() < 1e-3,
            "thrusting up to its top speed: {speed:?}"
        );
        let fired = events.iter().any(|event| {
            matches!(
                event,
                CombatEvent::Fired {
                    ship: ShipRef::Player,
                    ..
                }
            )
        });
        assert!(!fired, "the pod fires nothing");
        assert_eq!(session.engine_glow, 0, "no glow");
        assert!(
            !session.take_sounds().contains(&SimSound::ThrustStarted),
            "no thrust sound"
        );
    }

    #[test]
    fn the_pod_counts_down_from_350_and_the_rescue_is_due_on_the_350th_tick() {
        let catalog = ejecting(100);
        let mut session = in_a_pod(&catalog, 0.0);
        for tick in 1..POD_TICKS {
            session.tick(crate::flight::Controls::default());
            assert_eq!(session.pod, Some(POD_TICKS - tick));
            assert!(!session.rescue_due, "tick {tick}");
        }
        session.tick(crate::flight::Controls::default());
        assert_eq!(session.pod, Some(0));
        assert!(session.rescue_due);
        assert_eq!(session.player_fate(), PlayerFate::Ejected { ticks_left: 0 });
    }

    #[test]
    fn a_pod_at_zero_armour_is_rescued_when_the_countdown_ends() {
        let catalog = ejecting(100);
        let mut session = in_a_pod(&catalog, 0.0);
        session.pilot.reserves.armor.now = 0.0;
        for _ in 0..POD_TICKS {
            assert!(!session.rescue_due);
            session.tick(crate::flight::Controls::default());
            let (_, events) = fight(&mut session, false);
            assert_eq!(session.condition, Condition::Intact, "no breakup");
            assert!(!events.iter().any(|event| matches!(
                event,
                CombatEvent::BreakingUp {
                    ship: ShipRef::Player,
                    ..
                } | CombatEvent::Destroyed {
                    ship: ShipRef::Player,
                    ..
                }
            )));
        }
        assert!(session.rescue_if_due(&catalog, &mut NeverFires));
        assert_eq!(session.pilot.ship, ShipId(128));
        assert!(session.pilot.reserves.armor.now > 0.0);
        assert_eq!(session.player_fate(), PlayerFate::Flying);
    }

    #[test]
    fn the_klaxon_sounds_first_only_while_the_countdown_after_its_decrement_is_above_1() {
        let catalog = ejecting(100);
        for (ticks_left, klaxon) in [(3, true), (2, false)] {
            let mut session = flying(&catalog, &[POD]);
            to_the_breakup(&mut session, &catalog, false);
            session.condition = Condition::Dying {
                ticks_left,
                starting: false,
            };
            let (sounds, _) = fight(&mut session, true);
            assert!(in_the_pod(&session));
            assert_eq!(
                sounds.first() == Some(&SimSound::Klaxon),
                klaxon,
                "{ticks_left}: {sounds:?}"
            );
        }
    }

    // The eject's sprite clones (`_SWCloneFramesFromMaster` @0x6f3f5,
    // @0x6f768).

    /// `catalog` with each of the `hidden` classes' hulls costing -1, so
    /// that the ship is hidden ([`HullSpec::hidden`](crate::HullSpec)).
    fn hiding(mut catalog: FakePilotCatalog, hidden: &[i16]) -> FakePilotCatalog {
        for &id in hidden {
            match catalog.hulls.iter_mut().find(|hull| hull.id == ShipId(id)) {
                Some(hull) => hull.cost = -1,
                None => catalog.hulls.push(crate::catalog::HullRecord {
                    cost: -1,
                    ..hull(id)
                }),
            }
        }
        catalog
    }

    /// `catalog`'s session, disabled, ejected on the chord with an escape
    /// pod aboard; `prepare` runs first.
    fn podded(catalog: &FakePilotCatalog, prepare: impl FnOnce(&mut Session)) -> Session {
        let mut session = flying(catalog, &[POD]);
        session.condition = Condition::Disabled;
        prepare(&mut session);
        session.hold_eject(true);
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert!(in_the_pod(&session));
        session
    }

    #[test]
    fn the_players_slot_holds_the_pods_frames_even_when_the_pod_is_hidden() {
        use crate::ship_layer::Slot;
        let catalog = hiding(ejecting(100), &[ESCAPE_POD_SHIP.0]);
        let session = podded(&catalog, |_| {});
        assert!(session.is_hidden(ShipRef::Player));
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Player),
            ESCAPE_POD_SHIP,
            "@0x6f768"
        );
        assert_eq!(session.framed_by(ShipRef::Player), Some(ESCAPE_POD_SHIP));
    }

    #[test]
    fn the_hulks_slot_holds_the_old_class_even_when_it_is_hidden() {
        use crate::ship_layer::{INIT_FRAMES, Slot};
        const OLD: i16 = 170;
        assert_ne!(ShipId(OLD), INIT_FRAMES);
        let mut catalog = hiding(ejecting(100), &[OLD]);
        catalog.ship_records.push(ship(OLD, FAST));
        let session = podded(&catalog, |session| {
            session.pilot.ship = ShipId(OLD);
            session.hull = session.arsenal.hull(ShipId(OLD));
        });
        let hulk = hulk(&session);
        assert_eq!(hulk.ship, ShipId(OLD));
        assert!(session.is_hidden(ShipRef::Npc(hulk.id)));
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Berth(hulk.berth)),
            ShipId(OLD),
            "@0x6f3f5"
        );
        assert_eq!(session.framed_by(ShipRef::Npc(hulk.id)), Some(ShipId(OLD)));
    }

    #[test]
    fn the_players_slot_holds_the_escape_ships_frames_even_when_it_is_hidden() {
        use crate::ship_layer::Slot;
        let catalog = hiding(escaping(), &[VIPER]);
        let (session, _) = escaped(&catalog, &[0; 3], |_| {});
        assert_eq!(session.pilot.ship, ShipId(VIPER));
        assert!(session.is_hidden(ShipRef::Player));
        assert_eq!(
            session.slot_sprites().frames.of(Slot::Player),
            ShipId(VIPER),
            "@0x6f768"
        );
        assert_eq!(session.framed_by(ShipRef::Player), Some(ShipId(VIPER)));
    }
}
