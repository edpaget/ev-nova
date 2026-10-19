//! The fly half of a traffic tick, resumable one NPC at a time, as
//! `_DoPlayGameWork`'s ship loop flies each ship in its own `_HandleShip`
//! (@0x44483, the loop @0x4445c-0x444c4), between the ships of the slots
//! before and after it: [`Traffic::begin_fly`], then
//! [`Traffic::fly_npc`] for each NPC, then [`Traffic::end_fly`].
//!
//! - **Begin** holds every NPC's cloak, notes which ones are entering a
//!   gate, and snapshots what each flight reads of the others: every
//!   ship's state before any moves, the player's, the top speeds and the
//!   disabled ships. The original's steering reads come from
//!   `_AIDispatch` (`_LowLevelAIHandler`), before the loop moves any ship,
//!   so every flight reads the others where the last tick left them.
//! - **Each NPC** flies its docking flight or its autopilot step from its
//!   own state as it stands, so a push an earlier slot's shockwave gave
//!   it is in the velocity it moves by. One whose flight ends it (it
//!   landed, jumped, entered a gate, docked) leaves the traffic at once,
//!   as its slot is inactive from its own `_HandleShip` on: no later
//!   slot's step, fire or shockwave sees it.
//! - **End** starts the pirates' docks, counts the board waits and the AI
//!   waits down, releases the followers of each lead that reached its
//!   gate's box, and regenerates the fuel; on the advance pass the pods
//!   the think half freed join the departures, the ships left behind are
//!   let go, and the tick is counted on the AI timer.
//!
//! [`Traffic::fly_holding`] and [`Traffic::move_in`] run the three over
//! every NPC in the traffic's order; the session's ship loop runs them by
//! slot, each NPC's flight before its own fight step
//! (`nova_sim::Session::ship_loop`).

use std::collections::{BTreeMap, BTreeSet};

use super::autopilot::{self, Outcome, TopSpeeds};
use super::npc::{AiType, Mode, Npc, NpcId};
use super::plunder_dock::DockOutcome;
use super::{Traffic, World, board_wait, plunder};
use crate::ai::Goal;
use crate::catalog::StellarId;
use crate::chance::Chance;
use crate::combat::ShipRef;
use crate::combat::hull::Condition;
use crate::flight::ShipState;

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
    /// The NPCs entering a gate as the half began.
    entering: BTreeSet<NpcId>,
    /// The NPCs in the traffic's order as the half began.
    order: Vec<NpcId>,
    /// Each dock a pirate started, as (pirate, victim).
    docked: Vec<(NpcId, ShipRef)>,
    /// Each lead that reached its gate's box, its gate and AI type.
    reached: Vec<(NpcId, StellarId, AiType)>,
}

impl FlyPass {
    /// The NPCs in the traffic's order as the half began.
    #[must_use]
    pub fn order(&self) -> &[NpcId] {
        &self.order
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
        self.hold_cloaks();
        let entering = self
            .npcs
            .iter()
            .filter(|npc| npc.entering_gate())
            .map(|npc| npc.id)
            .collect();
        let speeds = self
            .npcs
            .iter()
            .map(|npc| npc.id)
            .zip(self.top_speeds(world))
            .collect();
        let disabled = self
            .npcs
            .iter()
            .filter(|npc| npc.condition == Condition::Disabled)
            .map(|npc| npc.id)
            .collect();
        self.departed.clear();
        FlyPass {
            half,
            still: still.clone(),
            player: world.player.map(|player| player.state),
            player_disabled: world
                .player
                .is_some_and(|player| player.condition == Condition::Disabled),
            states,
            speeds,
            disabled,
            entering,
            order,
            docked: Vec::new(),
            reached: Vec::new(),
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
        if let Mode::EnteringGate { gate, .. } = npc.mode
            && !pass.entering.contains(&id)
        {
            pass.reached.push((id, gate, npc.ai_type));
        }
        if outcome != Outcome::Flying {
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
        autopilot::coast_unsteered(npc, other.as_ref(), world.flight);
    }

    /// Ends the fly half `pass` (see the module docs).
    pub fn end_fly(&mut self, pass: FlyPass) {
        let FlyPass {
            half,
            docked,
            reached,
            ..
        } = pass;
        self.dock_starts(&docked);
        board_wait::count_down(&mut self.npcs);
        plunder::count_down(&mut self.npcs);
        self.release_followers(&reached);
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

/// The ship `npc`'s flight is about (the one it docks with, follows,
/// escorts, fights or inspects), where the fly half `pass` found it, and
/// whether it was disabled then.
fn other_of(pass: &FlyPass, npc: &Npc) -> (Option<ShipState>, bool) {
    let other = match npc.goal {
        _ if npc.docking.is_some() => npc.docking.map(|docking| docking.ship),
        Goal::Follow(lead) => Some(ShipRef::Npc(lead)),
        Goal::Assist(_) | Goal::Formation { .. } => Some(ShipRef::Player),
        Goal::Dock(carrier) => Some(carrier),
        goal => goal.quarry().or(goal.inspecting()),
    };
    let disabled = other.is_some_and(|ship| match ship {
        ShipRef::Player => pass.player_disabled,
        ShipRef::Npc(id) => pass.disabled.contains(&id),
    });
    let other = other.and_then(|ship| match ship {
        ShipRef::Player => pass.player,
        ShipRef::Npc(id) => pass.states.get(&id).copied(),
    });
    (other, disabled)
}

/// `npc`'s flight in the fly half `pass` at `speeds`, in `world`, rolling
/// on `chance`: what became of it.
fn fly_one(
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
    match plunder::fly_docking(npc, other.as_ref(), speeds, world.flight, &mut *chance) {
        Some(DockOutcome::Started { victim, .. }) => {
            pass.docked.push((npc.id, victim));
            return Outcome::Flying;
        }
        Some(_) => return Outcome::Flying,
        None => {}
    }
    autopilot::fly(
        npc,
        world.sites,
        world.type_widths,
        other.as_ref(),
        other_disabled,
        speeds,
        world.flight,
        world.snipe_brake_lead,
        &mut *chance,
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::bay::Carrier;
    use crate::catalog::{LandingSite, ShipId};
    use crate::chance::NeverFires;
    use crate::geometry::Vec2;
    use crate::stats::ShipStats;
    use crate::traffic::fixture::npc;
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

    /// A traffic whose fly half lands one NPC, docks a fighter with its
    /// carrier, starts a pirate's dock with that carrier, takes a lead
    /// into its gate's box (releasing its follower) and holds one still:
    /// 0 lands on planet 140; 1, a fighter, docks with 2, its carrier;
    /// 3 docks with 2 to plunder it; 4 enters the gate, 5 following it;
    /// 6 flies on, short of fuel and regenerating half a unit a tick.
    fn busy() -> Traffic {
        let mut lander = at(0, 0.0, 0.0);
        lander.goal = Goal::Land(StellarId(140));
        let carrier = NpcId(2);
        let mut fighter = at(1, 480.0, 10.0);
        fighter.ship = ShipId(200);
        fighter.goal = Goal::Dock(ShipRef::Npc(carrier));
        fighter.carrier = Some(Carrier {
            ship: ShipRef::Npc(carrier),
            window: 100.0,
            reach: 30.0,
        });
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
        let mut lead = at(4, 1.0, 0.0);
        lead.ai_type = AiType::Interceptor;
        lead.goal = Goal::EnterGate(GATE);
        let mut follower = at(5, 2.0, 0.0);
        follower.leader = Some(NpcId(4));
        follower.goal = Goal::Follow(NpcId(4));
        let mut flier = at(6, -300.0, 0.0);
        flier.state.velocity = Vec2::new(1.0, 0.0);
        flier.goal = Goal::JumpOut;
        flier.reserves.fuel.now = 100.0;
        flier.stats.fuel_regen = 0.5;
        Traffic {
            npcs: vec![
                lander,
                fighter,
                at(2, 500.0, 0.0),
                pirate,
                lead,
                follower,
                flier,
            ],
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
            // The carrier, boarded by its fighter.
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

    /// What [`busy`]'s fly half sent away, the lander then the fighter.
    fn busy_departed() -> [(NpcId, Outcome); 2] {
        [
            (NpcId(0), Outcome::Landed(StellarId(140))),
            (
                NpcId(1),
                Outcome::Docked {
                    carrier: ShipRef::Npc(NpcId(2)),
                    ship: ShipId(200),
                },
            ),
        ]
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
        let sites = sites();
        let world = World::new(&sites);
        let mut traffic = busy();
        let mut pass = traffic.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        let landed = traffic.fly_npc(&mut pass, NpcId(0), &world, &mut NeverFires);
        assert_eq!(landed, Some(Outcome::Landed(StellarId(140))));
        assert!(traffic.npcs().iter().all(|npc| npc.id != NpcId(0)), "gone");
        assert_eq!(
            traffic.departed(),
            [(NpcId(0), Outcome::Landed(StellarId(140)))]
        );
        assert_eq!(
            traffic.fly_npc(&mut pass, NpcId(0), &world, &mut NeverFires),
            None,
            "no longer there"
        );
    }

    #[test]
    fn each_flight_reads_the_others_where_the_half_found_them() {
        // The fighter flies at its carrier's place as the half began,
        // though the carrier has since been pushed on.
        let sites = sites();
        let world = World::new(&sites);
        let mut moved = busy();
        let mut pass = moved.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        moved.npcs[2].state.position = Vec2::new(5000.0, 0.0);
        moved.fly_npc(&mut pass, NpcId(1), &world, &mut NeverFires);
        assert_eq!(
            moved.departed()[0].1,
            Outcome::Docked {
                carrier: ShipRef::Npc(NpcId(2)),
                ship: ShipId(200)
            }
        );
    }

    #[test]
    fn an_npcs_flight_moves_it_by_its_velocity_as_it_stands() {
        // A push given between the half's start and its flight is in the
        // velocity it moves by.
        let sites = sites();
        let world = World::new(&sites);
        let push = Vec2::new(0.0, 3.0);
        let mut pushed_before = busy();
        pushed_before.npcs[6].state.velocity = push;
        pushed_before.fly_holding(world, &BTreeSet::new(), &mut NeverFires);
        let mut pushed_after = busy();
        let mut pass = pushed_after.begin_fly(&world, &BTreeSet::new(), FlyHalf::Advance);
        pushed_after.npcs[6].state.velocity = push;
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
