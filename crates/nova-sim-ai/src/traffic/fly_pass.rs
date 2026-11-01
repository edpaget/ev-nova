//! The fly half of a traffic tick, resumable one NPC at a time, as
//! `_DoPlayGameWork`'s ship loop flies each ship in its own `_HandleShip`
//! (@0x44483, the loop @0x4445c-0x444c4), between the ships of the slots
//! before and after it: [`Traffic::begin_fly`], then
//! [`Traffic::fly_npc`] for each NPC, then [`Traffic::end_fly`].
//!
//! - **Begin** holds every NPC's cloak; on the advance pass moves each
//!   NPC's fire clock on, holds its fold to its dispatch, and runs the
//!   odds pass
//!   (`_DoPlayGameWork` @0x442b0-0x44336, after every think and before
//!   `_HandleShip`; see [`odds_cache`](super::odds_cache)); notes which
//!   ships are entering a gate; and snapshots what each flight reads of
//!   the others: every ship's state before any moves, the player's, the
//!   top speeds, the disabled ships, the leaders' warps (sending each
//!   leader out on the advance pass), the tractors' holders, each
//!   swarmer's lead and the ships that don't see their leader. The
//!   original's steering reads come from
//!   `_AIDispatch` (`_LowLevelAIHandler`), before the loop moves any ship,
//!   at each reader's own slot. So on the advance pass a flight reads a
//!   ship of a higher slot where it was as the AI loop opened, before that
//!   slot's own work moved it (a docking ship's creep, match or dock
//!   start, a follower's step, a gate entrant's stop, a drift), a ship
//!   that left at its slot among them; and a ship of a lower slot, or its
//!   own, where the half found it ([`FlyPass`]'s `opened`). A follower
//!   reads its leader's glow as the tick opened when its slot is below the
//!   leader's (see [`dispatch_leave`](super::dispatch_leave)). A gone ship
//!   is there, by its record, for a ship whose high level its think
//!   skipped and that held it then (see
//!   [`skipped_high_level`](super::skipped_high_level)). The warp clocks,
//!   the high level's hold, turn-back, gate box and release, a docking
//!   ship's low level (its match, creep, dock start and completion, see
//!   [`dispatch_dock`](super::dispatch_dock)), a ship's own jump-out and a
//!   fighter's dock are the AI loop's, at each ship's slot. A docking ship
//!   here only flies the heading and thrust its low level left
//!   ([`plunder_dock::handle_ship`]), on both passes.
//! - **Each NPC** sheds its ion charge, takes its tractor's hold, settles
//!   its spawn's display rolls, then flies its jump, its docking flight
//!   or its autopilot step from its own state as it stands, so a push an
//!   earlier slot's shockwave gave it is in the velocity it moves by, and
//!   banks and glows. One whose flight ends it (it entered a gate) leaves
//!   the traffic at once, as its slot is inactive from its own
//!   `_HandleShip` on: no later slot's step, fire or shockwave sees it,
//!   and it is recorded among the ships gone ([`Traffic::left_the_system`],
//!   see [`gone`](super::gone)). One that landed stays in its slot, waiting
//!   at the stellar, as the original's does (+0xb8 is not cleared,
//!   @0x8dadc-0x8db5a): only a mission ship under
//!   [`RuleKey::LandedLeaves`](crate::RuleKey::LandedLeaves)' other
//!   reading leaves as it lands ([`land_flight::leaves`]).
//! - **End** counts the AI waits down, and regenerates the fuel; on the
//!   advance pass the pods the think half freed join the departures, the
//!   ships left behind are let go, and the tick is counted on the AI
//!   timer.
//!
//! [`Traffic::fly_holding`] and [`Traffic::move_in`] run the three over
//! every NPC in the traffic's order; the session's ship loop runs them by
//! slot, each NPC's flight before its own fight step
//! (`nova_sim::Session::ship_loop`).

use std::collections::{BTreeMap, BTreeSet};

use super::autopilot::{self, Beside, Outcome, TopSpeeds};
use super::gone::Freed;
use super::held_burst;
use super::inertialess::Inertia;
use super::npc::{Mode, Npc, NpcId};
use super::plunder_dock::{self, DockFlight};
use super::skipped_high_level::SkippedReads;
use super::{
    Traffic, World, ionization, land_flight, plunder, regroup, ship_display, tractor_hold, warp_out,
};
use crate::ai::Goal;
use crate::ai::mine_orders;
use crate::ai::swarm::{self, SwarmLead};
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::{NpcFormation, ShipState};

/// Which pass's fly half a [`FlyPass`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlyHalf {
    /// The advance pass's ([`Traffic::fly_holding`]): it ends by adding
    /// the pods freed to the departures, letting go of the ships left
    /// behind and counting the tick.
    Advance,
    /// Double time's second pass's ([`Traffic::move_in`]): it ends with
    /// none of that.
    Again,
}

/// A fly half under way ([`Traffic::begin_fly`]): what every NPC's flight
/// reads, as the half began, and what the half's end settles.
#[derive(Clone, Debug)]
pub struct FlyPass {
    half: FlyHalf,
    /// Whether it is the advance pass's, whose high level the flight
    /// follows.
    think: bool,
    /// The NPCs that keep their place.
    still: BTreeSet<NpcId>,
    /// The player's state, if it is in the system.
    player: Option<ShipState>,
    /// Whether the player is disabled.
    player_disabled: bool,
    /// Every NPC's state as the half began.
    states: BTreeMap<NpcId, ShipState>,
    /// Every NPC's top speeds this tick.
    speeds: BTreeMap<NpcId, TopSpeeds>,
    /// The NPCs disabled as the half began.
    disabled: BTreeSet<NpcId>,
    /// The flight each docking ship's low level left `_HandleShip` at its
    /// slot of the AI loop this tick (the traffic's `dispatch_dock`), which
    /// both of double time's passes fly.
    docks: BTreeMap<NpcId, DockFlight>,
    /// The NPCs in the traffic's order as the half began.
    order: Vec<NpcId>,
    /// Each leader's warp as the half began, after every slot's step of
    /// the AI loop, the ships that left in it as their records hold it:
    /// what a follower above its leader reads, and every follower on
    /// double time's second pass.
    leaders: BTreeMap<NpcId, warp_out::LeaderWarp>,
    /// Each leader's warp as the tick's think opened, before any slot's
    /// step: what a follower below its leader reads on the advance pass
    /// (see the traffic's `dispatch_leave`).
    open_leaders: BTreeMap<NpcId, warp_out::LeaderWarp>,
    /// Each leader's slot, the ships that left in the AI loop among them.
    leader_berths: BTreeMap<NpcId, u32>,
    /// Each ship's slot and state as the advance pass's AI loop opened,
    /// before any slot's work, the ships that left in it among them: the
    /// ships of lower slots steer by it so, as their low level came before
    /// its slot's (its creep, match or dock start, a follower's step, a
    /// gate entrant's stop, its leave). None on double time's second pass,
    /// which has no AI loop.
    opened: BTreeMap<NpcId, (u32, ShipState)>,
    /// The ships a tractor's hold may name, as the half began.
    holders: tractor_hold::Holders,
    /// Each swarmer's lead, as the half began.
    leads: BTreeMap<NpcId, SwarmLead>,
    /// The NPCs that don't see their leader, an NPC or the player, as the
    /// half began.
    unseen: BTreeSet<NpcId>,
    /// The NPCs in the regroup pause as the half began, whose flight
    /// flies another goal ([`Npc::fly_as_flown`]) but whose warp-out reads
    /// the pause's (see [`warp_out::fly_jump`]).
    paused: BTreeSet<NpcId>,
    /// The gone ship each ship whose high level was skipped held at its
    /// think, by its record
    /// ([`skipped_high_level`](super::skipped_high_level)): none on double
    /// time's second pass, which has no think.
    skipped: SkippedReads,
    /// Each freed slot's record as the half began ([`gone`](super::gone)),
    /// which a docking's nav (+0x6c) still reads by the ship's id whatever
    /// tick freed it: no slot test clears the nav, and a record stands
    /// until a ship takes its berth, when the docking moves to that ship
    /// (see [`repair_dock`](super::repair_dock)). Nothing else reads it.
    records: BTreeMap<NpcId, ShipState>,
    /// The first class, which a spawn's anim rolls may read.
    first: ship_display::FirstClass,
}

impl FlyPass {
    /// The NPCs in the traffic's order as the half began.
    #[must_use]
    pub fn order(&self) -> &[NpcId] {
        &self.order
    }

    /// The leaders' warps `npc` reads: as the tick's think opened for a
    /// follower below its leader on the advance pass, as the half began
    /// otherwise (see [`FlyPass::leaders`]).
    fn leaders_for(&self, npc: &Npc) -> &BTreeMap<NpcId, warp_out::LeaderWarp> {
        let below = npc
            .leader
            .and_then(|leader| self.leader_berths.get(&leader))
            .is_some_and(|&berth| npc.berth < berth);
        if self.think && below {
            &self.open_leaders
        } else {
            &self.leaders
        }
    }
}

impl Traffic {
    /// Begins a fly half of `half` in `world` (see the module docs), the
    /// NPCs in `still` keeping their place: every NPC's cloak held
    /// ([`Traffic::hold_cloaks`]) and what each flight reads snapshot.
    /// The last tick's departures are let go.
    pub fn begin_fly(&mut self, world: &World, still: &BTreeSet<NpcId>, half: FlyHalf) -> FlyPass {
        let states = self.npcs.iter().map(|npc| (npc.id, npc.state)).collect();
        let order = self.npcs.iter().map(|npc| npc.id).collect();
        let think = half == FlyHalf::Advance;
        self.hold_cloaks(world);
        if think {
            // The warp clocks moved on as the loop opened, and the high
            // level's hold and turn-back ran at each ship's slot
            // (`dispatch_leave`).
            for npc in &mut self.npcs {
                npc.armament.tick_fire_clock();
                ship_display::dispatch_fold(npc);
            }
            // `_DoPlayGameWork`'s odds pass, after every think and before
            // `_HandleShip` (@0x442b0-0x44336; see `odds_cache`).
            self.refresh_odds(world);
        }
        let docks = self.dock_flights.clone();
        let speeds = self
            .npcs
            .iter()
            .map(|npc| npc.id)
            .zip(self.top_speeds(world))
            .collect();
        // On the advance pass the ships that left at their slot show their
        // records, +0x50 0 and sub 4 kept (@0x859ae-0x859bf).
        let left_in_dispatch: &[Npc] = if think { &self.left_in_dispatch } else { &[] };
        let leader_berths = (self.npcs.iter().chain(left_in_dispatch))
            .map(|npc| (npc.id, npc.berth))
            .collect();
        let mut leaders = warp_out::leaders(left_in_dispatch);
        leaders.extend(warp_out::leaders(&self.npcs));
        let open_leaders = if think {
            self.open_leaders.clone()
        } else {
            leaders.clone()
        };
        let mut opened = if think {
            std::mem::take(&mut self.open_states)
        } else {
            BTreeMap::new()
        };
        for npc in left_in_dispatch {
            opened.entry(npc.id).or_insert((npc.berth, npc.state));
        }
        if think {
            warp_out::send_leaders_out(&mut self.npcs);
        }
        let holders = tractor_hold::Holders::of(world, &self.npcs);
        let disabled = self
            .npcs
            .iter()
            .filter(|npc| npc.condition == Condition::Disabled)
            .map(|npc| npc.id)
            .collect();
        let around = world.around(&self.npcs);
        let leads = self
            .npcs
            .iter()
            .filter_map(|npc| swarm::lead(npc, &around).map(|lead| (npc.id, lead)))
            .collect();
        let unseen = self
            .npcs
            .iter()
            .filter(|npc| !around.sees_leader(npc))
            .map(|npc| npc.id)
            .collect();
        let paused = self
            .npcs
            .iter()
            .filter(|npc| regroup::paused(npc))
            .map(|npc| npc.id)
            .collect();
        self.departed.clear();
        let skipped = std::mem::take(&mut self.skipped);
        let records = (self.gone.ships().iter())
            .map(|gone| (gone.ship.id, gone.ship.state))
            .collect();
        FlyPass {
            half,
            think,
            still: still.clone(),
            player: world.player.map(|player| player.state),
            player_disabled: world
                .player
                .is_some_and(|player| player.condition == Condition::Disabled),
            states,
            speeds,
            disabled,
            docks,
            order,
            leaders,
            open_leaders,
            leader_berths,
            opened,
            holders,
            leads,
            unseen,
            paused,
            skipped,
            records,
            first: self.first_class,
        }
    }

    /// Flies NPC `id` a tick in `world`, in the fly half `pass`, rolling
    /// on `chance` (see the module docs): its docking flight, else its
    /// autopilot step ([`autopilot::fly`]), towards the ship its goal is
    /// about where the half found it, at its top speeds; none for an NPC
    /// held still. What became of it, none when it is not in the system
    /// or was not as the half began. One whose flight ends it leaves the
    /// traffic at once, among [`Traffic::departed`].
    pub fn fly_npc(
        &mut self,
        pass: &mut FlyPass,
        id: NpcId,
        world: &World,
        chance: &mut (impl Chance + ?Sized),
    ) -> Option<Outcome> {
        let index = self.npcs.iter().position(|npc| npc.id == id)?;
        let speeds = *pass.speeds.get(&id)?;
        let npc = &mut self.npcs[index];
        let outcome = fly_one(pass, npc, speeds, world, chance);
        if ends(&self.npcs[index], outcome, world) {
            if let Some(freed) = Freed::by(outcome) {
                self.left_the_system(self.npcs[index].clone(), freed);
            }
            self.departed.push((id, outcome));
            self.npcs.remove(index);
        }
        Some(outcome)
    }

    /// Moves NPC `id`, one the fly half `pass` did not find as it began
    /// (a fighter launched into a later slot of the ship loop), in
    /// `world`: only `_HandleShip`'s move (@0x34274-0x342aa), after an
    /// inertialess ship's velocity adjust (@0x3425f), with no steering,
    /// as `_AIDispatch` ran before the loop (@0x44270-0x442ae) and gave it
    /// no turn that frame. Nothing happens for an NPC not in the system.
    ///
    /// The original's escape fighter thinks first on the frame after its
    /// launch, as here; it moves on its launch frame, as here too.
    pub fn coast_npc(&mut self, pass: &FlyPass, id: NpcId, world: &World) {
        let Some(npc) = self.npcs.iter_mut().find(|npc| npc.id == id) else {
            return;
        };
        let (other, _) = other_of(pass, npc);
        let seen = beside_of(pass, npc).leader_seen;
        autopilot::coast_unsteered(npc, other.as_ref(), seen, world.flight);
    }

    /// Ends the fly half `pass` (see the module docs), which no NPC is
    /// flown in after.
    // Taken by value so no flight of the half can follow its end.
    #[allow(clippy::needless_pass_by_value)]
    pub fn end_fly(&mut self, pass: FlyPass) {
        let half = pass.half;
        plunder::count_down(&mut self.npcs);
        self.npcs.iter_mut().for_each(Npc::regenerate_fuel);
        if half == FlyHalf::Advance {
            self.departed.extend(std::mem::take(&mut self.freed));
            // The display pass (`_DoPlayGameWork` @0x444ad) follows the
            // frame's arrivals pass (step 1) and frees each ship left
            // behind (`_HandleShipDisplay` @0x2ded6-0x2dee7). The original
            // frees them after the frame's AI and ship handling too, whose
            // scans test the system; here none of those steps sees a ship
            // left behind. The engine's move settles after the frame's
            // traffic tick, so its one arrivals pass is the next tick's:
            // the lag of a settled script, crossing no boundary, as a ship
            // left behind is never drawn.
            self.left_behind.clear();
            self.ticks += 1;
        }
    }

    /// A whole fly half of `half` in `world`, the NPCs in `still` keeping
    /// their place, rolling on `chance`: begun, every NPC flown in the
    /// traffic's order, and ended.
    pub(super) fn fly_every(
        &mut self,
        world: &World,
        still: &BTreeSet<NpcId>,
        half: FlyHalf,
        chance: &mut (impl Chance + ?Sized),
    ) {
        let mut pass = self.begin_fly(world, still, half);
        for id in pass.order.clone() {
            self.fly_npc(&mut pass, id, world, chance);
        }
        self.end_fly(pass);
    }
}

/// Whether `npc`'s flight's `outcome` takes it out of the traffic in
/// `world`: any but flying on, but a landing only for a ship that leaves
/// as it lands ([`land_flight::leaves`]); every other landed ship stays in
/// its slot, waiting at the stellar.
fn ends(npc: &Npc, outcome: Outcome, world: &World) -> bool {
    match outcome {
        Outcome::Flying => false,
        Outcome::Landed(_) => land_flight::leaves(npc, world.landed_leaves),
        _ => true,
    }
}

/// Where the fly half `pass` found `ship` for `reader`, if it is there:
/// for a reader in a lower slot on the advance pass, where it was as the
/// AI loop opened, as the reader's low level steered by it before the
/// ship's own slot wrote it (a ship that left there among them); for any
/// other, where the half found it. A gone ship is there, by its record,
/// for a reader whose high level was skipped and that held it at its think
/// ([`skipped_high_level`](super::skipped_high_level)).
fn state_of(pass: &FlyPass, ship: ShipRef, reader: &Npc) -> Option<ShipState> {
    match ship {
        ShipRef::Player => pass.player,
        ShipRef::Npc(id) => (pass.opened.get(&id))
            .filter(|&&(berth, _)| reader.berth < berth)
            .map(|&(_, state)| state)
            .or_else(|| pass.states.get(&id).copied())
            .or_else(|| pass.skipped.state_of(reader.id, id)),
    }
}

/// What `npc`'s flight reads beside the ship it is about, in the fly
/// half `pass` ([`Beside`]): whether it has no bay to dock in, its swarm
/// lead, its guard where the half
/// found it while it keeps formation ([`Goal::Formation`]) or keeps with
/// a lead ([`Goal::Follow`]), and whether it sees its leader as the half
/// began (`_ShipVisibleToShip(leader, ship)` @0x8ed5f): a fleet escort
/// its NPC lead, the player's escort the player
/// ([`Surroundings::sees_leader`](crate::ai::Surroundings::sees_leader)).
/// The coast and the inertia
/// ([`Inertia::of`]) read the same sight.
///
/// The original reads the sight in the escort's own high level, so a
/// cloak its lead (or the player) changed earlier in the same think half
/// shows here a tick later than there: a one-tick lag settled before the frame is
/// drawn, crossing no boundary.
fn beside_of(pass: &FlyPass, npc: &Npc) -> Beside {
    let (guard, leader_seen) = match npc.goal {
        Goal::Formation { guard } | Goal::Follow { guard, .. } => {
            (guard, !pass.unseen.contains(&npc.id))
        }
        _ => (None, true),
    };
    Beside {
        swarm: pass.leads.get(&npc.id).copied(),
        guard: guard.and_then(|guard| state_of(pass, guard, npc)),
        leader_seen,
        paused: pass.paused.contains(&npc.id),
        ..Beside::default()
    }
}

/// The ship `npc`'s flight is about (the one it docks with, follows,
/// escorts, fights or inspects), where the fly half `pass` found it, and
/// whether it was disabled then. A docking's ship gone from the system is
/// read by its slot's record, as the nav (+0x6c) still names the slot
/// (@0x86a19-0x86a51; see [`repair_dock`](super::repair_dock)).
fn other_of(pass: &FlyPass, npc: &Npc) -> (Option<ShipState>, bool) {
    if let Some(docking) = npc.docking {
        let state = state_of(pass, docking.ship, npc).or_else(|| match docking.ship {
            ShipRef::Npc(id) => pass.records.get(&id).copied(),
            ShipRef::Player => None,
        });
        return (state, disabled_in(pass, docking.ship));
    }
    let other = match npc.goal {
        Goal::Follow { lead, .. } => Some(ShipRef::Npc(lead)),
        Goal::Assist(_) | Goal::Formation { .. } => Some(ShipRef::Player),
        Goal::Dock(carrier) => Some(carrier),
        goal => goal.quarry().or(goal.inspecting()),
    };
    let disabled = other.is_some_and(|ship| disabled_in(pass, ship));
    (other.and_then(|ship| state_of(pass, ship, npc)), disabled)
}

/// Whether `ship` was disabled as the fly half `pass` began.
fn disabled_in(pass: &FlyPass, ship: ShipRef) -> bool {
    match ship {
        ShipRef::Player => pass.player_disabled,
        ShipRef::Npc(id) => pass.disabled.contains(&id),
    }
}

/// `npc`'s flight in the fly half `pass` at `speeds`, in `world`, rolling
/// on `chance`: what became of it. Every read of its goal finds the goal
/// it flies ([`Npc::fly_as_flown`]): a ship in the regroup pause flies its
/// kept substate's.
fn fly_one(
    pass: &mut FlyPass,
    npc: &mut Npc,
    speeds: TopSpeeds,
    world: &World,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    npc.fly_as_flown(|npc| fly_flown(pass, npc, speeds, world, chance))
}

/// [`fly_one`] with `npc`'s goal the one it flies.
fn fly_flown(
    pass: &mut FlyPass,
    npc: &mut Npc,
    speeds: TopSpeeds,
    world: &World,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    let (other, other_disabled) = other_of(pass, npc);
    if pass.still.contains(&npc.id) {
        return Outcome::Flying;
    }
    let derelict = world.govts.derelict(npc.govt);
    // Its ion block, the move's other side (see the `ionization` module).
    ionization::deionize(npc, speeds.own);
    tractor_hold::hold(npc, &pass.holders, world.tractor_step, pass.think);
    ship_display::settle_spawn(npc, derelict, &pass.first, &mut *chance);
    let held = ship_display::held(npc);
    let beside = Beside {
        rock: world.mining_rock,
        scoop_box: mine_orders::nearest_box(npc.state.position, world.boxes),
        ..beside_of(pass, npc)
    };
    // What its flight flies by, and what it owed of a gate burst before it.
    let inertia = Inertia::of(npc, other.as_ref(), beside.leader_seen, world.flight);
    // The sub-goal this flight picks, stored for the next frame's sight and
    // cloak think (`_HighLevelAIHandler` writes +0xc8ca).
    npc.close_form = npc.keeps_close_formation(other.as_ref(), beside.leader_seen, world.flight);
    let owed = npc.owed_burst;
    let outcome = fly_steered(
        npc,
        pass,
        world,
        (other, other_disabled, beside),
        speeds,
        &mut *chance,
    );
    held_burst::settle(npc, owed, speeds, inertia, &mut *chance);
    ship_display::bank_and_glow(npc, held, derelict, leader_glow(pass, npc, world));
    outcome
}

/// The glow close formation copies to `npc` (nav's +0xc8d4,
/// @0x87dce-0x87deb), in `world` and the fly half `pass`: by the engine's
/// reading of [`RuleKey::NpcFormation`](crate::RuleKey::NpcFormation), a
/// fleet escort's ([`Goal::Follow`]) its NPC lead's as the half began,
/// none with its lead gone; otherwise the player's.
fn leader_glow(pass: &FlyPass, npc: &Npc, world: &World) -> Option<i16> {
    match npc.goal {
        Goal::Follow { lead, .. } if world.flight.npc_formation == NpcFormation::Original => {
            pass.leaders_for(npc).get(&lead).map(|leader| leader.glow)
        }
        _ => Some(world.warp.player.glow),
    }
}

/// Flies `npc` a tick in `world` in the fly half `pass`: its jump, its
/// dock with a ship it plunders or repairs (by the flight its low level
/// left at its slot), or its goal, toward `other`, disabled as
/// `other_disabled` says, with `beside` what it reads beside it (its
/// swarm lead, a formation escort's guard and sight), at `speeds`,
/// rolling on `chance`. Its goal's stellar is looked up
/// among every stellar the system lists, shown or not ([`World::listed`]),
/// as `_HighLevelAIHandler` reads the stellar's record (state 1 @0x8d8d7,
/// state 20 @0x8db8a), or among the slot stellars for one the system
/// doesn't list, as a slot names ([`World::flown_among`]).
fn fly_steered(
    npc: &mut Npc,
    pass: &mut FlyPass,
    world: &World,
    (other, other_disabled, beside): (Option<ShipState>, bool, Beside),
    speeds: TopSpeeds,
    chance: &mut (impl Chance + ?Sized),
) -> Outcome {
    // The goal it holds, not the one it flies (see `fly_one`).
    let paused = pass.paused.contains(&npc.id);
    let leaders = pass.leaders_for(npc);
    if let Some(outcome) = warp_out::fly_jump(npc, leaders, world, pass.think, paused) {
        return outcome;
    }
    // A docking ship flies what its low level left at its slot of the AI
    // loop (the traffic's `dispatch_dock`), and decides nothing here. One
    // whose low level flew no docking this tick keeps its heading and
    // thrusts nothing.
    let docks =
        npc.docking.is_some() && npc.condition == Condition::Intact && npc.mode == Mode::Flying;
    if let Some(flown) = pass.docks.get(&npc.id).copied().or_else(|| {
        docks.then(|| {
            let handling = npc.flying_handling(speeds.own, world.flight);
            DockFlight::none(npc, &handling, world.flight)
        })
    }) {
        plunder_dock::handle_ship(npc, &flown, world.flight);
        return Outcome::Flying;
    }
    // A goal naming a stellar the system doesn't list, as a slot does, is
    // flown among the slot stellars (see `slot_stellars`).
    let flown_among = world.flown_among(npc.goal);
    let outcome = autopilot::fly(
        npc,
        flown_among,
        world.type_widths,
        other.as_ref(),
        other_disabled,
        beside,
        speeds,
        world.flight,
        autopilot::Readings {
            snipe_lead: world.snipe_brake_lead,
            jump_fuel: world.npc_jump_fuel,
            held_burst: world.held_burst,
            npc_landing: world.npc_landing,
        },
        world.weapons,
        &mut *chance,
    );
    let jumping = npc.jumping() && !paused;
    warp_out::settle(npc, pass.leaders_for(npc), &world.warp, jumping);
    outcome
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::catalog::StellarId;
    use crate::catalog::{LandingSite, ShipId};
    use crate::chance::NeverFires;
    use crate::geometry::Vec2;
    use crate::rulebook::RuleSource;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Mode};
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use nova_sim_base::fixture::FAST;
    use nova_sim_catalog::fixture::planet;

    /// Hypergate 300 at the centre, 100 wide.
    const GATE: StellarId = StellarId(300);

    /// Planet 140 at the centre, and hypergate 300 there too.
    fn sites() -> [LandingSite; 2] {
        [
            planet(140, 0.0, 0.0),
            LandingSite {
                flags2: crate::gate::HYPERGATE,
                ..planet(300, 0.0, 0.0)
            },
        ]
    }

    /// NPC `id` at berth `id + 1`, at (`x`, `y`) at rest.
    fn at(id: u32, x: f32, y: f32) -> Npc {
        let mut npc = npc(id, ShipStats::new(FAST, &[]));
        npc.berth = id + 1;
        npc.state.position = Vec2::new(x, y);
        npc
    }

    /// A fighter, 1, of ship 200 at (480, 10) docking with its carrier, 2,
    /// within its reach.
    fn fighter() -> Npc {
        let mut fighter = at(1, 480.0, 10.0);
        fighter.ship = ShipId(200);
        fighter.goal = Goal::Dock(ShipRef::Npc(NpcId(2)));
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(NpcId(2)),
            window: 100.0,
            reach: 30.0,
        });
        fighter
    }

    /// A traffic whose fly half lands one NPC, flies a pirate's dock with a
    /// ship, counts a lead into its gate and holds one still: 0 lands on
    /// planet 140 and stays there; 3 docks with 2 to plunder it; 4 enters
    /// the gate, 5, its follower, released into it; 6 flies on, short of
    /// fuel and regenerating half a unit a tick. The AI loop did its part
    /// at each slot ([`dispatch_leave`](super::super::dispatch_leave)): 3's
    /// dock started, its wait 100 (`Rand(80)` drew 0) and 2 boarded; 4
    /// stopped in its gate's box on its wait of 16, and released 5. A
    /// fighter's dock and a jump-out are that loop's too, so none of it
    /// leaves here.
    fn busy() -> Traffic {
        let mut lander = at(0, 0.0, 0.0);
        lander.goal = Goal::Land(StellarId(140));
        let carrier = NpcId(2);
        let mut boarded = at(2, 500.0, 0.0);
        boarded.boarded = true;
        let mut pirate = at(3, 501.0, 1.0);
        pirate.goal = Goal::Plunder {
            ship: ShipRef::Npc(carrier),
            hopeless: false,
        };
        pirate.hull.class_turn = 3.0;
        pirate.docking = Some(Docking {
            ship: ShipRef::Npc(carrier),
            sub: DockSub::Dock,
        });
        pirate.ai_wait = crate::traffic::plunder_dock::DOCK_WAIT;
        let mut lead = at(4, 1.0, 0.0);
        lead.ai_type = AiType::Interceptor;
        lead.goal = Goal::EnterGate(GATE);
        lead.mode = Mode::EnteringGate {
            gate: GATE,
            ticks_left: crate::traffic::gate_entry::ENTRY_TICKS,
        };
        let mut follower = at(5, 2.0, 0.0);
        follower.goal = Goal::EnterGate(GATE);
        let mut flier = at(6, -300.0, 0.0);
        flier.state.velocity = Vec2::new(1.0, 0.0);
        flier.goal = Goal::JumpOut;
        flier.reserves.fuel.now = 100.0;
        flier.stats.fuel_regen = 0.5;
        Traffic {
            npcs: vec![lander, boarded, pirate, lead, follower, flier],
            ..Traffic::default()
        }
    }

    /// What a fly half leaves of each NPC still in the traffic, in order:
    /// its id, state, whether it was boarded, its docking, goal, fuel,
    /// mode and AI wait.
    type Left = (
        NpcId,
        ShipState,
        bool,
        Option<Docking>,
        Goal,
        f32,
        Mode,
        u32,
    );

    fn left(traffic: &Traffic) -> Vec<Left> {
        traffic
            .npcs()
            .iter()
            .map(|npc| {
                (
                    npc.id,
                    npc.state,
                    npc.boarded,
                    npc.docking,
                    npc.goal,
                    npc.reserves.fuel.now,
                    npc.mode,
                    npc.ai_wait,
                )
            })
            .collect()
    }

    /// A ship's state at (`x`, `y`) moving by (`dx`, 0), facing `heading`.
    fn state(x: f32, y: f32, dx: f32, heading: f32) -> ShipState {
        ShipState {
            position: Vec2::new(x, y),
            velocity: Vec2::new(dx, 0.0),
            heading,
        }
    }

    /// What [`busy`]'s fly half left of its NPCs, the flier in `flier`,
    /// as the batch fly half did before it was split NPC by NPC (rdm
    /// weapons-fidelity phase 46, recorded from the parent of 898d0253).
    fn busy_left(flier: ShipState) -> Vec<Left> {
        let docking = Docking {
            ship: ShipRef::Npc(NpcId(2)),
            sub: DockSub::Dock,
        };
        let entering = Mode::EnteringGate {
            gate: GATE,
            ticks_left: 15,
        };
        let plunder = Goal::Plunder {
            ship: ShipRef::Npc(NpcId(2)),
            hopeless: false,
        };
        let gate = Goal::EnterGate(GATE);
        vec![
            // The lander, stopped at goal 0 for its stay (300 + `Rand(200)`,
            // the longest, 499), counted once.
            (
                NpcId(0),
                state(0.0, 0.0, 0.0, 0.0),
                false,
                None,
                Goal::Idle,
                300.0,
                Mode::Flying,
                498,
            ),
            // The pirate's victim, boarded.
            (
                NpcId(2),
                state(500.0, 0.0, 0.0, 0.0),
                true,
                None,
                Goal::Idle,
                300.0,
                Mode::Flying,
                0,
            ),
            // The pirate, its dock started (`Rand(80)` + 100) and counted once.
            (
                NpcId(3),
                state(501.0, 1.0, 0.0, 0.0),
                false,
                Some(docking),
                plunder,
                300.0,
                Mode::Flying,
                99,
            ),
            (
                NpcId(4),
                state(1.0, 0.0, 0.0, 0.0),
                false,
                None,
                gate,
                300.0,
                entering,
                0,
            ),
            // The follower, released into its lead's gate.
            (
                NpcId(5),
                state(2.0, 0.0, 0.0, 0.0),
                false,
                None,
                gate,
                300.0,
                Mode::Flying,
                0,
            ),
            (
                NpcId(6),
                flier,
                false,
                None,
                Goal::JumpOut,
                100.5,
                Mode::Flying,
                0,
            ),
        ]
    }

    /// What [`busy`]'s fly half sent away: nothing. The lander stays.
    fn busy_departed() -> [(NpcId, Outcome); 0] {
        []
    }

    /// `traffic`'s fly half of `half` run one NPC at a time, in the
    /// traffic's order.
    fn each(traffic: &mut Traffic, world: &World, still: &BTreeSet<NpcId>, half: FlyHalf) {
        let mut pass = traffic.begin_fly(world, still, half);
        let order: Vec<NpcId> = traffic.npcs().iter().map(|npc| npc.id).collect();
        assert_eq!(pass.order(), order);
        for id in order {
            assert!(
                traffic
                    .fly_npc(&mut pass, id, world, &mut NeverFires)
                    .is_some()
            );
        }
        traffic.end_fly(pass);
    }

    #[test]
    fn flying_each_npc_in_turn_matches_the_batch_fly_half() {
        // The advance pass, the flier held still, ends as the batch fly
        // half before the split did.
        let sites = sites();
        let world = World::new(&sites);
        let still = BTreeSet::from([NpcId(6)]);
        let mut batch = busy();
        batch.fly_holding(world, &still, &mut NeverFires);
        let mut turn = busy();
        each(&mut turn, &world, &still, FlyHalf::Advance);
        for traffic in [&turn, &batch] {
            assert_eq!(traffic.departed(), busy_departed());
            assert_eq!(left(traffic), busy_left(state(-300.0, 0.0, 1.0, 0.0)));
            assert_eq!(traffic.ticks, 1);
        }
        assert_eq!(turn, batch);
    }

    #[test]
    fn the_second_pass_flown_npc_by_npc_matches_its_move() {
        // Double time's second pass ends as the batch `move_in` before the
        // split did: the flier moves on, and the tick is not counted.
        let sites = sites();
        let world = World::new(&sites);
        let mut batch = busy();
        batch.ticks = 7;
        batch.move_in(world, &mut NeverFires);
        let mut turn = busy();
        turn.ticks = 7;
        each(&mut turn, &world, &BTreeSet::new(), FlyHalf::Again);
        for traffic in [&turn, &batch] {
            assert_eq!(traffic.departed(), busy_departed());
            assert_eq!(left(traffic), busy_left(state(-299.0, 0.0, 1.0, 357.0)));
            assert_eq!(traffic.ticks, 7, "not counted");
        }
        assert_eq!(turn, batch);
    }

    #[test]
    fn an_npc_whose_flight_ends_it_leaves_the_traffic_at_once() {
        // The lead's gate entry ends on its last count (@0x2c078).
        let sites = sites();
        let world = World::new(&sites);
        let mut traffic = busy();
        let lead = NpcId(4);
        traffic.npcs[3].mode = Mode::EnteringGate {
            gate: GATE,
            ticks_left: 1,
        };
        let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        let entered = traffic.fly_npc(&mut pass, lead, &world, &mut NeverFires);
        assert_eq!(entered, Some(Outcome::EnteredGate(GATE)));
        assert!(traffic.npcs().iter().all(|npc| npc.id != lead), "gone");
        assert_eq!(traffic.departed(), [(lead, Outcome::EnteredGate(GATE))]);
        assert_eq!(
            traffic.fly_npc(&mut pass, lead, &world, &mut NeverFires),
            None,
            "no longer there"
        );
    }

    #[test]
    fn a_fighter_within_its_carriers_reach_docks_nowhere_in_the_fly_half() {
        // Its low level docks it at its slot of the AI loop (@0x87fef).
        let sites = sites();
        let world = World::new(&sites);
        let mut traffic = Traffic {
            npcs: vec![fighter(), at(2, 500.0, 0.0)],
            ..Traffic::default()
        };
        let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        let flown = traffic.fly_npc(&mut pass, NpcId(1), &world, &mut NeverFires);
        assert_eq!(flown, Some(Outcome::Flying));
        assert_eq!(traffic.npcs().len(), 2, "still out");
    }

    #[test]
    fn an_npc_that_lands_stays_in_its_slot_unless_it_leaves_as_it_lands() {
        let sites = sites();
        let mark = crate::traffic::npc::MissionMark {
            slot: 0,
            behav: crate::traffic::ShipBehav::AttackPlayer,
            formation: None,
            order: None,
        };
        for (mission, reading, stays) in [
            (None, RuleSource::Engine, true),
            (None, RuleSource::Bible, true),
            (Some(mark), RuleSource::Engine, true),
            (Some(mark), RuleSource::Bible, false),
        ] {
            let world = World {
                landed_leaves: reading,
                ..World::new(&sites)
            };
            let mut traffic = busy();
            traffic.npcs[0].mission = mission;
            let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
            let landed = traffic.fly_npc(&mut pass, NpcId(0), &world, &mut NeverFires);
            assert_eq!(landed, Some(Outcome::Landed(StellarId(140))));
            let there = traffic.npcs().iter().any(|npc| npc.id == NpcId(0));
            assert_eq!(there, stays, "{mission:?} {reading:?}");
            let departed: &[_] = if stays {
                &[]
            } else {
                &[(NpcId(0), Outcome::Landed(StellarId(140)))]
            };
            assert_eq!(traffic.departed(), departed, "{mission:?} {reading:?}");
            assert!(
                traffic.gone.ships().is_empty(),
                "no landing is recorded gone"
            );
        }
    }

    #[test]
    fn each_flight_reads_the_others_where_the_half_found_them() {
        // The fighter flies at its carrier's place as the half began,
        // though the carrier has since been pushed on.
        let sites = sites();
        let world = World::new(&sites);
        let flown = |push: bool| {
            let mut traffic = Traffic {
                npcs: vec![fighter(), at(2, 500.0, 0.0)],
                ..Traffic::default()
            };
            let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
            if push {
                traffic.npcs[1].state.position = Vec2::new(5000.0, 0.0);
            }
            traffic.fly_npc(&mut pass, NpcId(1), &world, &mut NeverFires);
            traffic.npcs[0].state
        };
        assert_eq!(flown(true), flown(false));
    }

    #[test]
    fn a_lower_slot_steers_by_a_ship_that_left_at_its_slot_and_a_higher_one_does_not() {
        // A lower slot's low level steered before the ship left in the AI
        // loop; a higher one's came after (@0x859b8).
        let sites = sites();
        let world = World::new(&sites);
        let flown = |berth: u32, left: bool| {
            let mut fighter = fighter();
            fighter.berth = berth;
            fighter.carrier = Some(Carrier {
                reach: 0.0,
                ..fighter.carrier.expect("a fighter")
            });
            let carrier = at(2, 500.0, 0.0);
            let mut traffic = Traffic {
                npcs: vec![fighter],
                ..Traffic::default()
            };
            if left {
                traffic.left_in_dispatch.push(carrier);
            } else {
                traffic.npcs.push(carrier);
            }
            let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
            traffic.fly_npc(&mut pass, NpcId(1), &world, &mut NeverFires);
            traffic.npcs[0].state
        };
        assert_eq!(flown(1, true), flown(1, false), "below it: as it was");
        assert_ne!(flown(4, true), flown(4, false), "above it: gone");
        // A ship in its own slot came after it left, as a launch into the
        // freed slot does: gone.
        assert_ne!(flown(3, true), flown(3, false), "its slot's new ship");
    }

    /// Keeps every ship's goal.
    #[derive(Debug)]
    struct Keeps;

    impl crate::ai::Behaviour for Keeps {
        fn decide(
            &self,
            npc: &Npc,
            _around: &crate::ai::Surroundings,
            _chance: &mut dyn Chance,
        ) -> Goal {
            npc.goal
        }
    }

    #[test]
    fn a_lower_slot_steers_by_a_docking_ship_where_it_was_before_its_creep_and_a_higher_one_after()
    {
        // The creep is the docking ship's own low level's, at its slot of
        // `_AIDispatch`'s loop (@0x8700f-0x87095): a lower slot's low level
        // steered by it before, a higher slot's after.
        let mut victim = at(4, 100.0, 0.0);
        victim.condition = Condition::Disabled;
        let mut pirate = at(2, 100.0, 10.0);
        pirate.goal = Goal::Plunder {
            ship: ShipRef::Npc(victim.id),
            hopeless: false,
        };
        pirate.target = Some(ShipRef::Npc(victim.id));
        pirate.hull.class_turn = 3.0;
        pirate.docking = Some(Docking {
            ship: ShipRef::Npc(victim.id),
            sub: DockSub::Dock,
        });
        let hunter = |id: u32| {
            let mut hunter = at(id, -500.0, 0.0);
            hunter.goal = Goal::Attack(ShipRef::Npc(pirate.id));
            hunter
        };
        let (below, above) = (hunter(0), hunter(6));
        let mut traffic = Traffic {
            npcs: vec![below.clone(), pirate.clone(), victim, above.clone()],
            interval: 1,
            ..Traffic::default()
        };
        let world = World::new(&[]);
        traffic.think_holding(&Keeps, world, &[], &mut NeverFires);
        let crept = traffic.npcs[1].state;
        assert_eq!(crept.position, Vec2::new(100.0, 9.0), "crept at its slot");
        let pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        let read = |reader: &Npc| other_of(&pass, reader).0;
        assert_eq!(read(&below), Some(pirate.state), "before its creep");
        assert_eq!(read(&above), Some(crept), "after it");
    }

    #[test]
    fn an_npcs_flight_moves_it_by_its_velocity_as_it_stands() {
        // A push given between the half's start and its flight is in the
        // velocity it moves by.
        let sites = sites();
        let world = World::new(&sites);
        let push = Vec2::new(0.0, 3.0);
        let mut pushed_before = busy();
        pushed_before.npcs[5].state.velocity = push;
        pushed_before.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        let mut pushed_after = busy();
        let mut pass = pushed_after.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        pushed_after.npcs[5].state.velocity = push;
        for id in pass.order().to_vec() {
            pushed_after.fly_npc(&mut pass, id, &world, &mut NeverFires);
        }
        pushed_after.end_fly(pass);
        let flier = |traffic: &Traffic| {
            traffic
                .npcs()
                .iter()
                .find(|npc| npc.id == NpcId(6))
                .map(|npc| npc.state)
        };
        assert_eq!(flier(&pushed_after), flier(&pushed_before));
        let mut unpushed = busy();
        unpushed.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        assert_ne!(flier(&pushed_after), flier(&unpushed), "by the push");
    }

    /// A pirate plundering `victim` from 400 pixels off, flying at it at
    /// 2 a tick, flown one fly half among `others` and the `player`: its
    /// speed after.
    fn plunderer_speed(
        victim: ShipRef,
        others: Vec<Npc>,
        player: &crate::traffic::PlayerSide,
    ) -> f32 {
        let mut pirate = at(9, 0.0, 400.0);
        pirate.state.velocity = Vec2::new(0.0, -2.0);
        pirate.goal = Goal::Plunder {
            ship: victim,
            hopeless: false,
        };
        let mut npcs = others;
        npcs.push(pirate);
        let mut traffic = Traffic {
            npcs,
            ..Traffic::default()
        };
        let world = World {
            player: Some(*player),
            ..World::new(&[])
        };
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        let pirate = traffic.npcs().iter().find(|npc| npc.id == NpcId(9));
        pirate.expect("there").state.velocity.length()
    }

    #[test]
    fn a_plunderer_brakes_by_a_victim_the_half_found_disabled() {
        // `_LowLevelAIHandler` reads the victim's `_IsDisabled`: a pirate
        // brakes beside a disabled victim, and flies on at one not.
        let player = |condition| crate::traffic::PlayerSide {
            condition,
            ..crate::ai::fixture::player(0.0, 0.0)
        };
        let victim = |condition| {
            let mut victim = at(1, 0.0, 0.0);
            victim.condition = condition;
            victim
        };
        let npc = ShipRef::Npc(NpcId(1));
        let flying = plunderer_speed(
            npc,
            vec![victim(Condition::Intact)],
            &player(Condition::Intact),
        );
        let braking = plunderer_speed(
            npc,
            vec![victim(Condition::Disabled)],
            &player(Condition::Intact),
        );
        assert!(braking < flying, "{braking} {flying}");
        let flying = plunderer_speed(ShipRef::Player, vec![], &player(Condition::Intact));
        let braking = plunderer_speed(ShipRef::Player, vec![], &player(Condition::Disabled));
        assert!(braking < flying, "the player: {braking} {flying}");
    }

    /// An escort keeping formation 450 right of the player at the
    /// centre, facing 250, guarding `guard`, flown one fly half beside
    /// `others`: its heading after.
    fn closing_escort_heading(guard: Option<ShipRef>, others: Vec<Npc>) -> f32 {
        let mut escort = at(5, 450.0, 0.0);
        escort.state.heading = 250.0;
        escort.goal = Goal::Formation { guard };
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 2,
                ships: 3,
                spacing: 300.0,
            },
            order: None,
        });
        let mut npcs = others;
        npcs.push(escort);
        let mut traffic = Traffic {
            npcs,
            ..Traffic::default()
        };
        let world = World {
            player: Some(crate::ai::fixture::player(0.0, 0.0)),
            ..World::new(&[])
        };
        traffic.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        let escort = traffic.npcs().iter().find(|npc| npc.id == NpcId(5));
        escort.expect("there").state.heading
    }

    #[test]
    fn a_closing_escort_steers_at_its_guard_where_the_half_found_it_else_at_the_player() {
        // The player bears 270 from it, a turn to the right; a guard
        // straight below bears 180, a turn to the left.
        let below = at(1, 450.0, 1000.0);
        let guarded = Some(ShipRef::Npc(NpcId(1)));
        assert_eq!(closing_escort_heading(guarded, vec![below.clone()]), 247.0);
        assert_eq!(closing_escort_heading(None, vec![below]), 253.0);
        assert_eq!(closing_escort_heading(guarded, vec![]), 253.0, "gone");
    }

    /// A lead (NPC 1, at the centre, moving right at 1) and its fleet
    /// escort (NPC 2), 50 to its left in slot 2 of its formation, moving
    /// with it and facing left, guarding NPC 3, 1000 below; the lead
    /// cloaked when `cloaked`.
    fn fleet(cloaked: bool) -> Traffic {
        let mut lead = at(1, 0.0, 0.0);
        lead.state.velocity = Vec2::new(1.0, 0.0);
        if cloaked {
            lead.cloak = crate::cloak::Cloak {
                fade: crate::cloak::FADE_FULL,
                direction: 0,
            };
        }
        let mut escort = at(2, -50.0, 0.0);
        escort.state.velocity = Vec2::new(1.0, 0.0);
        // Facing against its motion, where a brake thrusts.
        escort.state.heading = 270.0;
        escort.leader = Some(NpcId(1));
        escort.goal = Goal::Follow {
            lead: NpcId(1),
            guard: Some(ShipRef::Npc(NpcId(3))),
        };
        escort.fleet_slot = Some(crate::escort::FormationSlot {
            slot: 2,
            ships: 2,
            spacing: 30.0,
        });
        Traffic {
            npcs: vec![lead, escort, at(3, 0.0, 1000.0)],
            ..Traffic::default()
        }
    }

    #[test]
    fn a_fleet_escort_reads_its_guard_and_its_sight_of_its_lead_as_the_half_began() {
        let world = World::new(&[]);
        let mut seen = fleet(false);
        let pass = seen.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        let beside = beside_of(&pass, &seen.npcs[1]);
        assert!(beside.leader_seen);
        assert_eq!(beside.guard, Some(seen.npcs[2].state), "its guard");
        let mut cloaked = fleet(true);
        let pass = cloaked.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        // Its lead uncloaked after the half began: still unseen.
        cloaked.npcs[0].cloak = crate::cloak::Cloak::default();
        assert!(!beside_of(&pass, &cloaked.npcs[1]).leader_seen, "unseen");
        let mut own_cloak = fleet(true);
        own_cloak.npcs[1].cloak_gear.device = true;
        let pass = own_cloak.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        assert!(
            beside_of(&pass, &own_cloak.npcs[1]).leader_seen,
            "a cloak of its own sees its cloaked lead"
        );
    }

    #[test]
    fn a_fleet_escort_beside_a_cloaked_lead_it_cannot_see_brakes_and_one_that_sees_it_keeps_with_it()
     {
        let world = World::new(&[]);
        let escort = |traffic: &Traffic| traffic.npcs[1].state.velocity;
        let mut unseen = fleet(true);
        unseen.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        assert!(escort(&unseen).x < 1.0, "braking: {:?}", escort(&unseen));
        let mut seen = fleet(true);
        seen.npcs[1].cloak_gear.device = true;
        seen.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        assert_eq!(escort(&seen), Vec2::new(1.0, 0.0), "matched with it");
    }

    /// The player's escort (NPC 1) in formation slot 0, `off` from the
    /// player, both moving right at 1, the escort facing left: with a cloak
    /// of its own when `device`, and a cloak scanner on screen when
    /// `scanner`.
    fn player_escort(off: Vec2, device: bool, scanner: bool) -> Traffic {
        let mut escort = at(1, off.x, off.y);
        escort.state.velocity = Vec2::new(1.0, 0.0);
        escort.state.heading = 270.0;
        escort.escort = Some(crate::escort::EscortDuty {
            formation: crate::escort::FormationSlot {
                slot: 0,
                ships: 1,
                spacing: 30.0,
            },
            order: None,
        });
        escort.goal = Goal::Formation { guard: None };
        escort.cloak_gear.device = device;
        escort.cloak_gear.scanner.on_screen = scanner;
        Traffic {
            npcs: vec![escort],
            ..Traffic::default()
        }
    }

    /// A world of no stellars with the player at the centre moving right
    /// at 1, cloaked when `cloaked`.
    fn with_player(cloaked: bool) -> World<'static> {
        let mut player = crate::ai::fixture::player(0.0, 0.0);
        player.state.velocity = Vec2::new(1.0, 0.0);
        if cloaked {
            player = crate::ai::fixture::cloaked_player(player);
        }
        World {
            player: Some(player),
            ..World::new(&[])
        }
    }

    #[test]
    fn a_player_escort_reads_its_sight_of_the_player_as_the_half_began() {
        let seen = |traffic: &mut Traffic, cloaked: bool| {
            let pass = traffic.begin_fly(&with_player(cloaked), &BTreeSet::new(), FlyHalf::Advance);
            beside_of(&pass, &traffic.npcs[0]).leader_seen
        };
        let near = Vec2::new(-250.0, 0.0);
        assert!(
            !seen(&mut player_escort(near, false, false), true),
            "cloaked"
        );
        assert!(
            seen(&mut player_escort(near, true, false), true),
            "a cloak of its own"
        );
        assert!(
            seen(
                &mut player_escort(Vec2::new(-150.0, 0.0), false, true),
                true
            ),
            "scanned within 200"
        );
        assert!(
            !seen(&mut player_escort(near, false, true), true),
            "scanner out of reach"
        );
        assert!(
            seen(&mut player_escort(near, false, false), false),
            "uncloaked"
        );
    }

    #[test]
    fn a_player_escort_that_still_names_an_npc_lead_reads_its_sight_of_the_player() {
        let mut traffic = player_escort(Vec2::new(-250.0, 0.0), false, false);
        let mut lead = at(2, -260.0, 0.0);
        lead.state.velocity = Vec2::new(1.0, 0.0);
        traffic.npcs[0].leader = Some(lead.id);
        traffic.npcs.push(lead);
        let pass = traffic.begin_fly(&with_player(true), &BTreeSet::new(), FlyHalf::Advance);
        assert!(
            !beside_of(&pass, &traffic.npcs[0]).leader_seen,
            "it follows the player (+0x9a 0), whom it can't see"
        );
    }

    #[test]
    fn a_player_escort_that_cannot_see_its_cloaked_player_brakes_within_300_and_approaches_beyond()
    {
        let velocity = |off: Vec2, cloaked: bool| {
            let mut traffic = player_escort(off, false, false);
            traffic.fly_holding(with_player(cloaked), &BTreeSet::new(), &mut NeverFires);
            traffic.npcs[0].state.velocity
        };
        let near = Vec2::new(-250.0, 0.0);
        let unseen = velocity(near, true);
        assert!(unseen.x < 1.0, "braking (sub 1): {unseen:?}");
        let far = Vec2::new(-700.0, 0.0);
        assert_eq!(
            velocity(far, true),
            velocity(far, false),
            "approaching (sub 9) as a seen escort does"
        );
    }

    #[test]
    fn a_player_escort_that_sees_its_cloaked_player_keeps_close_formation() {
        let mut traffic = player_escort(Vec2::new(-250.0, 0.0), true, false);
        traffic.fly_holding(with_player(true), &BTreeSet::new(), &mut NeverFires);
        assert_eq!(
            traffic.npcs[0].state.velocity,
            Vec2::new(1.0, 0.0),
            "matched with it (sub 0xc)"
        );
    }

    #[test]
    fn an_escort_that_falls_out_of_close_formation_from_its_area_cloaked_leader_stays_blind_until_it_uncloaks()
     {
        // The escort has no cloak of its own: it sees its cloaked,
        // area-cloaked player only while its last flight kept close
        // formation (`_HasCloak` reads the stored sub-goal).
        let area_cloaked = |cloaked: bool| {
            let mut world = with_player(cloaked);
            if let Some(player) = world.player.as_mut() {
                player.area_cloak = true;
            }
            world
        };
        let near = Vec2::new(-250.0, 0.0);
        let mut riding = player_escort(near, false, false);
        riding.npcs[0].close_form = true;
        riding.fly_holding(area_cloaked(true), &BTreeSet::new(), &mut NeverFires);
        assert_eq!(
            riding.npcs[0].state.velocity,
            Vec2::new(1.0, 0.0),
            "it rides, sees and keeps close formation"
        );
        assert!(riding.npcs[0].close_form);
        let mut fallen = player_escort(near, false, false);
        for _ in 0..3 {
            fallen.fly_holding(area_cloaked(true), &BTreeSet::new(), &mut NeverFires);
            assert!(
                fallen.npcs[0].state.velocity.x < 1.0,
                "blind, it brakes (sub 1): {:?}",
                fallen.npcs[0].state.velocity
            );
            assert!(!fallen.npcs[0].close_form, "and never re-enters sub 0xc");
        }
        let far = Vec2::new(-700.0, 0.0);
        let approach = |cloaked: bool| {
            let mut traffic = player_escort(far, false, false);
            traffic.npcs[0].close_form = true;
            traffic.fly_holding(area_cloaked(cloaked), &BTreeSet::new(), &mut NeverFires);
            (traffic.npcs[0].state.velocity, traffic.npcs[0].close_form)
        };
        assert_eq!(
            approach(true),
            approach(false),
            "beyond 600 it approaches (sub 9), seen or not"
        );
        assert!(!approach(true).1);
        fallen.fly_holding(area_cloaked(false), &BTreeSet::new(), &mut NeverFires);
        assert!(
            fallen.npcs[0].close_form,
            "its leader uncloaked, it sees again"
        );
    }

    #[test]
    fn an_escort_flying_close_formation_is_marked_in_close_formation_and_any_other_flight_clears_it()
     {
        let mut close = player_escort(Vec2::new(-250.0, 0.0), false, false);
        assert!(!close.npcs[0].close_form, "it starts unmarked");
        close.fly_holding(with_player(false), &BTreeSet::new(), &mut NeverFires);
        assert!(close.npcs[0].close_form, "within 300: sub 0xc");
        let mut far = player_escort(Vec2::new(-700.0, 0.0), false, false);
        far.npcs[0].close_form = true;
        far.fly_holding(with_player(false), &BTreeSet::new(), &mut NeverFires);
        assert!(!far.npcs[0].close_form, "beyond 600: sub 9");
        let mut unseen = player_escort(Vec2::new(-250.0, 0.0), false, false);
        unseen.npcs[0].close_form = true;
        unseen.fly_holding(with_player(true), &BTreeSet::new(), &mut NeverFires);
        assert!(!unseen.npcs[0].close_form, "its leader unseen: sub 1");
        let mut other = player_escort(Vec2::new(-250.0, 0.0), false, false);
        other.npcs[0].goal = Goal::JumpOut;
        other.npcs[0].close_form = true;
        other.fly_holding(with_player(false), &BTreeSet::new(), &mut NeverFires);
        assert!(!other.npcs[0].close_form, "no formation goal");
        let mut fleet_escort = fleet(false);
        fleet_escort.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
        assert!(
            fleet_escort.npcs[1].close_form,
            "a fleet escort by its lead"
        );
    }

    #[test]
    fn an_npc_held_still_keeps_its_close_formation_mark() {
        let still = BTreeSet::from([NpcId(1)]);
        let mut marked = player_escort(Vec2::new(-700.0, 0.0), false, false);
        marked.npcs[0].close_form = true;
        marked.fly_holding(with_player(false), &still, &mut NeverFires);
        assert!(marked.npcs[0].close_form);
        let mut unmarked = player_escort(Vec2::new(-250.0, 0.0), false, false);
        unmarked.fly_holding(with_player(false), &still, &mut NeverFires);
        assert!(!unmarked.npcs[0].close_form);
    }

    /// [`fleet`]'s escort's glow after a fly half under `rules`, its lead
    /// glowing 9 and the player 4.
    fn fleet_escort_glow(rules: crate::flight::FlightRules) -> i16 {
        let mut traffic = fleet(false);
        traffic.npcs[0].display.glow = 9;
        traffic.npcs[1].display.glow = 2;
        let mut world = World::new(&[]);
        world.flight = rules;
        world.warp.player.glow = 4;
        let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        // Its lead's glow changed after the half began: not read.
        traffic.npcs[0].display.glow = 15;
        traffic.fly_npc(&mut pass, NpcId(2), &world, &mut NeverFires);
        traffic.end_fly(pass);
        traffic.npcs[1].display.glow
    }

    #[test]
    fn a_fleet_escort_in_close_formation_ends_the_tick_with_its_leads_glow() {
        let rules = crate::flight::FlightRules::default();
        // Its lead's 9 copied, then faded a step with no thrust: not the
        // player's 4.
        assert_eq!(fleet_escort_glow(rules), 8);
        // The glow copy is the formation flight's: by the earlier follow
        // there is none, and the escort's own glow goes its own way.
        let earlier = crate::flight::FlightRules {
            npc_formation: NpcFormation::Earlier,
            ..rules
        };
        let glow = fleet_escort_glow(earlier);
        assert!(glow != 8 && glow != 3, "{glow}");
    }

    /// [`busy`] with a ship launched after its fly half began, at
    /// (100, 100) moving by (3, -2) and facing 45, set to jump out, which
    /// a steered flight would turn and thrust for; the half, and the ship.
    fn launched_mid_half(world: &World, inertialess: bool) -> (Traffic, FlyPass, NpcId) {
        let mut traffic = busy();
        let pass = traffic.begin_fly(world, &BTreeSet::new(), FlyHalf::Advance);
        let mut fighter = at(7, 100.0, 100.0);
        fighter.state.velocity = Vec2::new(3.0, -2.0);
        fighter.state.heading = 45.0;
        fighter.goal = Goal::JumpOut;
        fighter.stats.inertialess_hull = inertialess;
        let id = fighter.id;
        traffic.npcs.push(fighter);
        (traffic, pass, id)
    }

    #[test]
    fn a_ship_launched_after_the_half_began_coasts_unsteered() {
        // `_LaunchFighter` makes its slot active in the ship loop
        // (@0x3c90b, @0x3d680), whose `_HandleShip` moves it
        // (@0x34274-0x342aa) with no `_AIDispatch` that frame to steer it.
        let sites = sites();
        let world = World::new(&sites);
        let (mut traffic, pass, id) = launched_mid_half(&world, false);
        assert!(!pass.order().contains(&id), "not in the half's snapshot");
        traffic.coast_npc(&pass, id, &world);
        let coasted = traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("here");
        let moved = ShipState {
            position: Vec2::new(103.0, 98.0),
            velocity: Vec2::new(3.0, -2.0),
            heading: 45.0,
        };
        assert_eq!(coasted.state, moved, "unturned and unthrust");
    }

    #[test]
    fn an_inertialess_ship_launched_after_the_half_began_drifts_by_its_speed() {
        // `_AdjustInertialessShipVelocity` (@0x3425f) comes before the
        // position step.
        let sites = sites();
        let world = World::new(&sites);
        let (mut traffic, pass, id) = launched_mid_half(&world, true);
        let mut expected = traffic.npcs().iter().find(|npc| npc.id == id).cloned();
        let handling = world
            .flight
            .handling(expected.as_ref().expect("here").stats.handling);
        crate::traffic::inertialess::drift(expected.as_mut().expect("here"), &handling);
        traffic.coast_npc(&pass, id, &world);
        let coasted = traffic.npcs().iter().find(|npc| npc.id == id).cloned();
        assert_eq!(
            coasted.as_ref().map(|npc| npc.state),
            expected.map(|npc| npc.state)
        );
        assert_ne!(
            coasted.map(|npc| npc.state.velocity),
            Some(Vec2::new(3.0, -2.0)),
            "its velocity follows its speed"
        );
    }

    #[test]
    fn an_immobile_ship_launched_after_the_half_began_stays_put() {
        // `_HandleShip` zeroes an immobile ship's velocity and skips its
        // move (@0x34209-0x3423b).
        let sites = sites();
        let world = World::new(&sites);
        let (mut traffic, pass, id) = launched_mid_half(&world, false);
        let fighter = traffic
            .npcs
            .iter_mut()
            .find(|npc| npc.id == id)
            .expect("here");
        fighter.immobile = true;
        traffic.coast_npc(&pass, id, &world);
        let coasted = traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("here");
        let still = ShipState {
            position: Vec2::new(100.0, 100.0),
            velocity: Vec2::ZERO,
            heading: 45.0,
        };
        assert_eq!(coasted.state, still, "unmoved, its velocity gone");
    }
}
