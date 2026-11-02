//! `_AIDispatch`'s disabled ship: its warp cancelled and its AI reset
//! ([`Traffic::reset_disabled`]).
//!
//! **The warp cancel** (@0x8fcc8-0x8fce9). Every dispatch of a ship that is
//! `_IsDisabled` (disabled or breaking up, but for a stellar's defence
//! ship only held, [`Npc::is_disabled`]) with
//! +0x50 above 0 writes +0x50 = -1.0 (float 0xdd04c), before the dispatch's
//! -900 test: a dispatched ship, not an escape pod (@0x8fb62) and one with
//! AI (@0x4427e). So a disabled ship's warp restarts from nothing once it
//! is whole, as `_LowLevelAIHandler`'s sub 4 restarts a stage at 0 or
//! below.
//!
//! **The reset** (@0x8fece-0x8ff13), reached only by a ship whose dispatch
//! passes the gates ([`dispatch_gate::reaches_think`]) and that is not held
//! in the regroup pause ([`regroup`], @0x8fe73-0x8febb):
//! `_IsDisabled` → its parent +0x9a, target +0x70, travel stellar +0x6c and
//! defended stellar +0x8c = -1, its provocation +0x96 = 0, goal and
//! substate 0. Here:
//!
//! - [`Npc::target`] none, [`Npc::travel`] cleared, [`Npc::defends`]
//!   none (a defence ship held disabled stops defending; one breaking up
//!   is not disabled, @0x2d6f, so it keeps defending and counts for its
//!   stellar's wave until it is gone), [`Npc::provoked`]
//!   none, [`Goal::Idle`], and its substate's state, [`Npc::pursuit`] and
//!   [`Npc::docking`], none;
//! - by the engine's reading of
//!   [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) its leader is
//!   dropped: a fighter (one with a [`carrier`](Npc::carrier)) keeps its
//!   carrier and AI type, as the original's AI type 5 lasts until its
//!   first think once whole, where `_EscortAI`'s lost parent takes it out
//!   of its fleet ([`Decision::leaves_fleet`](crate::ai::Decision::leaves_fleet));
//!   any other led ship leaves its fleet now ([`Npc::leave_fleet`]),
//!   taking its class's `InherentAI` at the reset rather than at that
//!   think (a bribed one keeps the bribe's, as the original's +0x88
//!   does), and is marked ([`Npc::kept_escort_ai`]) so that it reads as
//!   the original's AI type 6 meanwhile ([`Npc::original_ai_type`]),
//!   which +0x88 keeps until that think, or for its stay if it thinks as
//!   a miner ([`Npc::settle_original_ai_type`]);
//! - by the other reading it keeps its leader, as the port had it;
//! - by either reading, a mission's ship in the player's formation loses
//!   the player as its parent, as the key is about NPC parents: a
//!   `ShipBehav` 1 one ([`dispatch_gate::led_again`]) is dropped
//!   ([`Npc::mission_dropped`]) until its first dispatch whole leads it
//!   again (@0x8fc03-0x8fc22, which skips a disabled ship @0x8fc0c; the
//!   session's), reading the re-lead's AI type 6 meanwhile, unless it is
//!   let go already ([`Npc::mission_let_go`]), as the reset writes no
//!   +0x88; one of any other `ShipBehav`, which no re-lead takes
//!   (@0x8fc07 → @0x8fc2d), is out of the formation for good, its mark
//!   kept, taking its class's `InherentAI` and reading 6 until its first
//!   think whole ([`Npc::kept_escort_ai`]), or, for `ShipBehav` 2, whose
//!   `_DeathStarAI` writes no +0x88 (@0x8ff3a), for its stay.
//!
//! A disabled ship's dispatch then jumps past its type's think
//! (@0x8ff13 → @0x90032), so `_EscortAI` never frees it while it is
//! disabled ([`dispatch_gate::reaches_escort_think`]).
//!
//! The reset precedes goal 0x15's skip (@0x8ff18), so a ship disabled in
//! its gate is reset too. Its goal 0 ends goal 0x15 where its wait runs on
//! ([`Npc::leaves_gate_goal`], by both readings, as the reset's other
//! writes are): it is seen and drawn unfaded, drifts there and never
//! bursts, keeping its burst word (@0x851e9-0x8520d).
//! Goal 0 is never throttled (@0x8fdee-0x8fe6f), so it repeats every
//! frame the ship is disabled: the leader is gone for good.
//!
//! The reset reaches a ship entering a gate too (sub 0x17): its +0x50 is
//! 0 (@0x8db8f), so it is off the glide and jump paths. Its sub 0
//! (@0x8ff0a) ends the entry, which `_HandleShipDisplay` reads by the sub
//! alone (@0x89502, @0x2bf94-0x2c088): from that dispatch the ship is drawn
//! unfaded and never freed, and it drifts disabled while its +0x4c counts
//! down, a ship breaking up exploding where it is
//! ([`Npc::ends_gate_entry`]). A ship disabled after the dispatch still
//! fades that frame, and may be freed then, as the fly half has it.
//!
//! **Ordering.** The original runs each step per ship in slot order, the
//! cancel and the reset inside each dispatch, and so does the engine, at
//! the head of each slot's run ([`dispatch_head`](super::dispatch_head)):
//! a lower slot's think reads a higher slot's ship as the tick began, and
//! a higher slot's reads it reset. The reset drops a disabled follower's leader after the formation
//! pass has numbered it ([`fleet_formation`](super::fleet_formation)), so
//! for that one tick its fleet's slots still count it, as the original's
//! do (it numbers disabled followers too, and the parent's slot
//! calculation runs in the frame before its reset): a lag no player sees.
//! The reset's end of a gate entry comes in the think half, before the fly
//! half's count-down, as `_AIDispatch` comes before `_HandleShipDisplay`.
//!
//! Nothing here plays a sound or draws a random number.

use super::Traffic;
use super::npc::Npc;
use super::pod_flight;
use super::regroup;
use crate::ai::pursuit::Pursuit;
use crate::ai::{Goal, dispatch_gate};
use crate::rulebook::RuleSource;

impl Traffic {
    /// Each disabled NPC's warp cancelled, of those `picked` picks (one
    /// flag an NPC; see the module docs).
    pub(crate) fn cancel_warps(&mut self, picked: &[bool]) {
        for (npc, _) in self.npcs.iter_mut().zip(picked).filter(|(_, p)| **p) {
            cancel_warp(npc);
        }
    }

    /// Each disabled NPC's AI reset, of those `picked` picks (one flag an
    /// NPC), by `reading` of
    /// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) (see the module
    /// docs).
    pub(crate) fn reset_disabled(&mut self, reading: RuleSource, picked: &[bool]) {
        for (npc, _) in self.npcs.iter_mut().zip(picked).filter(|(_, p)| **p) {
            if resets(npc) {
                reset(npc, reading);
            }
        }
    }
}

/// `npc`'s warp cancelled while it is disabled (see the module docs).
fn cancel_warp(npc: &mut Npc) {
    if !pod_flight::is_pod(npc) && !npc.no_ai && npc.is_disabled() && npc.warp.stage > 0.0 {
        npc.warp.give_up();
    }
}

/// Whether `npc` reaches the reset (see the module docs).
fn resets(npc: &Npc) -> bool {
    npc.is_disabled() && dispatch_gate::reaches_think(npc) && !regroup::paused(npc)
}

/// `npc`'s reset, by `reading` of
/// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) (see the module
/// docs).
pub(crate) fn reset(npc: &mut Npc, reading: RuleSource) {
    npc.leaves_gate_goal();
    npc.ends_gate_entry();
    npc.target = None;
    npc.travel = None;
    npc.defends = None;
    npc.provoked = 0.0;
    npc.goal = Goal::Idle;
    npc.pursuit = Pursuit::default();
    npc.docking = None;
    drop_mission_parent(npc);
    if reading == RuleSource::Engine && npc.leader.is_some() {
        if npc.carrier.is_some() {
            npc.leader = None;
        } else {
            npc.leave_fleet();
            npc.kept_escort_ai = true;
        }
    }
}

/// The reset's parent -1 for `npc`, a mission's ship in the player's
/// formation, by either reading (see the module docs): a `ShipBehav` 1
/// one, which the next dispatch whole leads again
/// ([`dispatch_gate::led_again`]), is dropped until then, unless it is let
/// go already; one of any other `ShipBehav` is out of the formation for
/// good, its mark kept, reading 6 until its first think whole (for
/// `ShipBehav` 2, for its stay).
fn drop_mission_parent(npc: &mut Npc) {
    if dispatch_gate::led_again(npc) {
        npc.mission_dropped = !npc.mission_let_go;
    } else if let Some(mark) = &mut npc.mission
        && mark.formation.is_some()
    {
        mark.formation = None;
        npc.leave_fleet();
        npc.kept_escort_ai = true;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ai::pursuit::Kept as PursuitKept;
    use crate::ai::regroup::Kept;
    use crate::ai::{Behaviour, NovaAi, Surroundings};
    use crate::bay::Carrier;
    use crate::chance::{Chance, NeverFires};
    use crate::combat::ShipRef;
    use crate::combat::hull::Condition;
    use crate::escort::FormationSlot;
    use crate::stats::ShipStats;
    use crate::traffic::autopilot::Outcome;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, MissionMark, Mode, NpcId};
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use crate::traffic::regroup::RELEASE_DRAW;
    use crate::traffic::warp_out::{GIVEN_UP_STAGE, WarpGoal};
    use crate::traffic::{ShipBehav, World};
    use nova_sim_base::fixture::{Draws, FAST};
    use std::collections::BTreeSet;

    /// Every ship keeps its goal.
    #[derive(Debug)]
    struct Keep;

    impl Behaviour for Keep {
        fn decide(&self, npc: &Npc, _: &Surroundings, _: &mut dyn Chance) -> Goal {
            npc.goal
        }

        fn target(&self, npc: &Npc, _: &Surroundings) -> Option<ShipRef> {
            npc.target
        }
    }

    /// The lead, NPC 0, and the ship it leads in `condition`, a trader of
    /// a warship's class following it, attacking the player, bound for a
    /// stellar, provoked, rushing and docking.
    fn fleet(condition: Condition) -> Traffic {
        let mut traffic = Traffic::with_interval(1);
        let leader = traffic.add_npc(npc(0, ShipStats::new(FAST, &[])));
        let mut led = npc(0, ShipStats::new(FAST, &[]));
        led.condition = condition;
        led.leader = Some(leader);
        led.hull.inherent_ai = 3;
        led.goal = Goal::Attack(ShipRef::Player);
        led.target = Some(ShipRef::Player);
        led.travel = Some(crate::catalog::StellarId(150));
        led.provoked = 9.0;
        led.pursuit = Pursuit {
            kept: Some(PursuitKept::Rush),
            frame: None,
        };
        led.docking = Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Approach,
        });
        traffic.add_npc(led);
        traffic
    }

    const LED: NpcId = NpcId(1);

    /// A change made to a ship before its tick.
    type Setup<'a> = &'a dyn Fn(&mut Npc);

    fn of(traffic: &Traffic, id: NpcId) -> &Npc {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == id)
            .expect("there")
    }

    /// `traffic` ticked once by `reading`, rolling on `chance`.
    fn ticked(traffic: &mut Traffic, reading: RuleSource, chance: &mut impl Chance) {
        let world = World {
            npc_escort_ai: reading,
            ..World::new(&[])
        };
        traffic.tick_in(&Keep, world, &[], chance);
    }

    /// Whether `npc` was reset: no target, travel stellar, provocation,
    /// pursuit or docking, and idle.
    fn was_reset(npc: &Npc) -> bool {
        npc.target.is_none()
            && !npc.travel_set()
            && npc.provoked == 0.0
            && npc.goal == Goal::Idle
            && npc.pursuit == Pursuit::default()
            && npc.docking.is_none()
    }

    #[test]
    fn a_disabled_defence_ship_stops_defending_its_stellar() {
        let mut traffic = Traffic::with_interval(1);
        let mut guard = npc(0, ShipStats::new(FAST, &[]));
        guard.condition = Condition::Disabled;
        guard.defends = Some(crate::catalog::StellarId(128));
        let id = traffic.add_npc(guard);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        assert_eq!(of(&traffic, id).defends, None, "+0x8c = -1 (@0x8fef8)");
    }

    #[test]
    fn a_defence_ship_breaking_up_is_not_disabled_and_so_neither_reset_nor_its_warp_cancelled() {
        // `_IsDisabled` never holds for it by its armour (@0x2d6f).
        let mut guard = npc(0, ShipStats::new(FAST, &[]));
        guard.condition = Condition::Dying {
            ticks_left: 9,
            starting: false,
        };
        guard.defends = Some(crate::catalog::StellarId(128));
        guard.target = Some(ShipRef::Player);
        guard.warp.stage = 0.5;
        assert!(!resets(&guard), "@0x8fece");
        let before = guard.warp.stage;
        cancel_warp(&mut guard);
        assert_eq!(guard.warp.stage, before, "@0x8fcc8");
        let mut traffic = Traffic::with_interval(1);
        let id = traffic.add_npc(guard);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let guard = of(&traffic, id);
        assert_eq!(guard.defends, Some(crate::catalog::StellarId(128)));
        assert_eq!(guard.target, Some(ShipRef::Player));
    }

    #[test]
    fn a_disabled_led_ship_loses_its_leader_target_travel_provocation_and_goal_by_the_engine() {
        for condition in [
            Condition::Disabled,
            Condition::Dying {
                ticks_left: 9,
                starting: false,
            },
        ] {
            let mut traffic = fleet(condition);
            ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
            let led = of(&traffic, LED);
            assert!(was_reset(led), "{condition:?}: {led:?}");
            assert_eq!(led.leader, None, "{condition:?}");
            assert_eq!(led.ai_type, AiType::Warship, "its class's InherentAI now");
        }
    }

    #[test]
    fn a_disabled_led_ship_keeps_the_original_ai_type_six_by_the_engine() {
        let mut traffic = fleet(Condition::Disabled);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let led = of(&traffic, LED);
        assert!(led.kept_escort_ai);
        assert_eq!(led.original_ai_type(), 6);
        let mut traffic = fleet(Condition::Disabled);
        ticked(&mut traffic, RuleSource::Bible, &mut NeverFires);
        let led = of(&traffic, LED);
        assert!(!led.kept_escort_ai, "its leader reads 6 already");
        assert_eq!(led.original_ai_type(), 6);
    }

    #[test]
    fn a_disabled_bribed_led_ship_keeps_reading_one_through_the_reset() {
        let mut traffic = fleet(Condition::Disabled);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        led.expect("there").take_bribe();
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let led = of(&traffic, LED);
        assert_eq!(led.leader, None);
        assert!(led.spared);
        assert_eq!(led.ai_type, AiType::WimpyTrader, "not its class's 3");
        assert_eq!(led.original_ai_type(), 1, "the bribe's word is kept");
    }

    #[test]
    fn a_disabled_led_ship_reads_its_own_type_after_its_first_think_once_whole() {
        let mut traffic = fleet(Condition::Disabled);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        traffic.tick_in(&NovaAi::default(), World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, LED).original_ai_type(), 6, "still disabled");
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        led.expect("there").condition = Condition::Intact;
        traffic.tick_in(&NovaAi::default(), World::new(&[]), &[], &mut NeverFires);
        let whole = of(&traffic, LED);
        assert!(!whole.kept_escort_ai);
        assert_eq!(whole.original_ai_type(), 3, "its class's InherentAI");
    }

    #[test]
    fn by_the_other_reading_it_keeps_its_leader_and_loses_the_rest() {
        let mut traffic = fleet(Condition::Disabled);
        ticked(&mut traffic, RuleSource::Bible, &mut NeverFires);
        let led = of(&traffic, LED);
        assert!(was_reset(led), "{led:?}");
        assert_eq!(led.leader, Some(NpcId(0)));
        assert_eq!(led.ai_type, AiType::WimpyTrader);
    }

    #[test]
    fn a_disabled_ship_forgets_its_provocation_and_an_intact_one_is_untouched() {
        // The reset's +0x96 = 0 (@0x8feef).
        for reading in RuleSource::ALL {
            let mut traffic = fleet(Condition::Intact);
            let before = of(&traffic, LED).clone();
            ticked(&mut traffic, reading, &mut NeverFires);
            let led = of(&traffic, LED);
            assert_eq!(
                (
                    led.leader,
                    led.target,
                    led.goal,
                    led.provoked,
                    led.travel_set()
                ),
                (
                    before.leader,
                    before.target,
                    before.goal,
                    before.provoked,
                    before.travel_set()
                ),
                "{reading:?}"
            );
        }
        let mut traffic = fleet(Condition::Disabled);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        assert_eq!(of(&traffic, LED).provoked, 0.0);
    }

    #[test]
    fn a_disabled_fighter_keeps_its_carrier_and_ai_type_until_its_first_think_once_whole() {
        let mut traffic = fleet(Condition::Disabled);
        let fighter = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        let fighter = fighter.expect("there");
        fighter.take_carrier(
            Carrier {
                ship: ShipRef::Npc(NpcId(0)),
                window: 100.0,
                reach: 40.0,
            },
            RuleSource::Engine,
        );
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let disabled = of(&traffic, LED);
        assert!(was_reset(disabled), "{disabled:?}");
        assert_eq!(disabled.leader, None);
        assert!(disabled.carrier.is_some(), "AI type 5 lasts");
        assert!(!disabled.kept_escort_ai);
        assert_eq!(disabled.original_ai_type(), 5);
        assert_eq!(disabled.ai_type, AiType::WimpyTrader);
        assert_eq!(disabled.reserves.shield.max, 39.99, "a fighter's capacity");
        // Whole again, its first think is `_EscortAI`'s lost parent.
        let fighter = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        fighter.expect("there").condition = Condition::Intact;
        traffic.tick_in(&NovaAi::default(), World::new(&[]), &[], &mut NeverFires);
        let whole = of(&traffic, LED);
        assert_eq!(whole.carrier, None);
        assert_eq!(whole.ai_type, AiType::Warship);
        assert_eq!(whole.reserves.shield.max, 30.0, "its class's capacity");
    }

    #[test]
    fn a_pod_a_hulk_a_ship_gliding_in_or_on_a_jump_or_one_paused_is_not_reset() {
        let pod = |npc: &mut Npc| {
            npc.ship = crate::combat::hull::ESCAPE_POD_SHIP;
            npc.mode = Mode::Adrift { ticks_left: 50 };
        };
        let hulk = |npc: &mut Npc| npc.no_ai = true;
        let gliding = |npc: &mut Npc| npc.mode = Mode::JumpingIn { tick: 0 };
        let jumping = |npc: &mut Npc| npc.warp.goal = Some(WarpGoal::Own);
        let paused = |npc: &mut Npc| {
            npc.goal = Goal::Regroup { kept: Kept::Coast };
        };
        let cases: [(&str, Setup); 5] = [
            ("pod", &pod),
            ("hulk", &hulk),
            ("gliding", &gliding),
            ("jumping", &jumping),
            ("paused", &paused),
        ];
        for (name, setup) in cases {
            let mut traffic = fleet(Condition::Disabled);
            let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
            setup(led.expect("there"));
            ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
            let led = of(&traffic, LED);
            assert_eq!(led.leader, Some(NpcId(0)), "{name}");
            assert_eq!(led.target, Some(ShipRef::Player), "{name}");
            assert!(led.travel_set(), "{name}");
        }
    }

    /// `fleet(condition)` with its led ship in `mode`, in the gate it comes
    /// out of.
    fn in_gate(condition: Condition, mode: Mode) -> Traffic {
        let mut traffic = fleet(condition);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        let led = led.expect("there");
        led.mode = mode;
        led.emerges_from = Some(crate::catalog::StellarId(310));
        traffic
    }

    #[test]
    fn a_ship_disabled_in_its_gate_is_reset_out_of_it_seen_and_never_bursts() {
        // The reset comes before goal 0x15's skip (@0x8fece, @0x8ff18): its
        // goal 0 ends goal 0x15, and +0x4c runs on.
        let dying = Condition::Dying {
            ticks_left: 90,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            for reading in RuleSource::ALL {
                let mut traffic = in_gate(condition, Mode::Emerging { ticks_left: 3 });
                ticked(&mut traffic, reading, &mut NeverFires);
                let reset = of(&traffic, LED);
                assert!(was_reset(reset), "{condition:?} {reading:?}");
                assert_eq!(reset.mode, Mode::HeldAtGate { ticks_left: 2 });
                assert_eq!(reset.emerges_from, None, "{condition:?} {reading:?}");
                assert!(!reset.emerging(), "{condition:?} {reading:?}");
                ticked(&mut traffic, reading, &mut NeverFires);
                assert_eq!(of(&traffic, LED).mode, Mode::HeldAtGate { ticks_left: 1 });
                ticked(&mut traffic, reading, &mut NeverFires);
                let out = of(&traffic, LED);
                assert_eq!(out.mode, Mode::Flying, "{condition:?} {reading:?}");
                assert_eq!(
                    out.owed_burst,
                    Some(crate::traffic::emerge::exit_speed(out.led_out, 0)),
                    "{condition:?} {reading:?}"
                );
            }
        }
    }

    #[test]
    fn a_ship_disabled_as_its_wait_runs_out_bursts_no_more() {
        let mut traffic = in_gate(Condition::Disabled, Mode::LeavingGate { tick: 0 });
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let reset = of(&traffic, LED);
        assert_eq!(reset.mode, Mode::Flying, "its wait was out");
        assert_eq!(reset.emerges_from, None);
        assert!(reset.owed_burst.is_some(), "the burst it still owes");
    }

    #[test]
    fn an_intact_ship_in_its_gate_keeps_its_mode() {
        let mut traffic = in_gate(Condition::Intact, Mode::Emerging { ticks_left: 3 });
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let waiting = of(&traffic, LED);
        assert_eq!(waiting.mode, Mode::Emerging { ticks_left: 2 });
        assert!(waiting.emerges_from.is_some());
    }

    /// The gate the led ship enters in [`entering`].
    const ENTERED: crate::catalog::StellarId = crate::catalog::StellarId(320);

    /// `fleet(condition)` with its led ship entering [`ENTERED`], 9 ticks
    /// of its wait left.
    fn entering(condition: Condition) -> Traffic {
        let mut traffic = fleet(condition);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        led.expect("there").mode = Mode::EnteringGate {
            gate: ENTERED,
            ticks_left: 9,
        };
        traffic
    }

    #[test]
    fn a_ship_disabled_as_it_enters_a_gate_is_reset_out_of_the_entry_and_stays() {
        // Its +0x50 is 0 (@0x8db8f), so it is off the glide and jump paths,
        // and the reset's sub 0 (@0x8ff0a) ends sub 0x17: drawn unfaded,
        // never freed (@0x2bf94-0x2c088), its +0x4c counting down
        // (@0x3501f-0x35048).
        let dying = Condition::Dying {
            ticks_left: 90,
            starting: false,
        };
        for condition in [Condition::Disabled, dying] {
            let mut traffic = entering(condition);
            traffic.think_holding(&Keep, World::new(&[]), &[], &mut NeverFires);
            let reset = of(&traffic, LED);
            assert!(!reset.entering_gate(), "{condition:?}");
            assert!(reset.arrived(), "{condition:?}");
            assert_eq!(reset.goal, Goal::Idle, "{condition:?}");
            traffic.fly_holding(World::new(&[]), &BTreeSet::new(), &mut NeverFires);
            assert_eq!(
                of(&traffic, LED).mode,
                Mode::Coasting { ticks_left: 8 },
                "{condition:?}: its wait counts down"
            );
            let mut departed = Vec::new();
            for _ in 0..30 {
                ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
                departed.extend(traffic.departed().iter().copied());
            }
            assert!(
                departed
                    .iter()
                    .all(|&(_, outcome)| outcome != Outcome::EnteredGate(ENTERED)),
                "{condition:?}: {departed:?}"
            );
            assert!(
                traffic.npcs().iter().any(|npc| npc.id == LED),
                "{condition:?}: still here"
            );
        }
    }

    #[test]
    fn an_intact_ship_entering_a_gate_still_leaves() {
        let mut traffic = entering(Condition::Intact);
        let mut departed = Vec::new();
        for _ in 0..9 {
            ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
            departed.extend(traffic.departed().iter().copied());
        }
        assert_eq!(departed, [(LED, Outcome::EnteredGate(ENTERED))]);
    }

    #[test]
    fn a_disabled_ship_on_its_jump_has_its_warp_cancelled_all_the_same() {
        // @0x8fcc8-0x8fce9, before the jump path (@0x8fd83). On its own
        // jump's goal 2, which the high level's hold keeps, with no target
        // to turn it to goal 3 (@0x8d513-0x8d56b), as any other goal's +0x50
        // is 0 there.
        let mut traffic = fleet(Condition::Disabled);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        let led = led.expect("there");
        led.goal = crate::ai::Goal::JumpOut;
        led.target = None;
        led.warp.goal = Some(WarpGoal::Own);
        led.warp.stage = 12.0;
        traffic.think_holding(&Keep, World::new(&[]), &[], &mut NeverFires);
        let cancelled = of(&traffic, LED).warp;
        assert_eq!(cancelled.stage, GIVEN_UP_STAGE);
        assert!(!cancelled.under_way(), "it restarts from nothing");
    }

    #[test]
    fn only_a_disabled_ship_with_its_warp_under_way_and_ai_has_it_cancelled() {
        for (condition, stage, no_ai, after) in [
            (Condition::Disabled, 5.0, false, GIVEN_UP_STAGE),
            (Condition::Intact, 5.0, false, 5.0),
            (Condition::Disabled, 0.0, false, 0.0),
            (Condition::Disabled, 5.0, true, 5.0),
        ] {
            let mut traffic = fleet(condition);
            let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
            let led = led.expect("there");
            led.warp.stage = stage;
            led.no_ai = no_ai;
            // A jump's goal with no target, so an intact ship's high level
            // holds its stage rather than ending it (@0x8d513), or ending it
            // as goal 3 does inside 1000 (@0x8fb2f-0x8fb42).
            led.goal = crate::ai::Goal::JumpOut;
            led.target = None;
            traffic.think_holding(&Keep, World::new(&[]), &[], &mut NeverFires);
            assert_eq!(
                of(&traffic, LED).warp.stage,
                after,
                "{condition:?} {stage} {no_ai}"
            );
        }
    }

    /// The player's place in its formation that a mission escort holds.
    const PLACE: FormationSlot = FormationSlot {
        slot: 2,
        ships: 3,
        spacing: 40.0,
    };

    /// A mission's ship of `behav` in the player's formation, in
    /// `condition`, of a warship's class (`InherentAI` 3) flying as a
    /// trader, and its NPC.
    fn mission_escort(behav: ShipBehav, condition: Condition) -> (Traffic, NpcId) {
        let mut traffic = Traffic::with_interval(1);
        let mut ship = npc(0, ShipStats::new(FAST, &[]));
        ship.condition = condition;
        ship.hull.inherent_ai = 3;
        ship.goal = Goal::Formation { guard: None };
        ship.mission = Some(MissionMark {
            slot: 1,
            behav,
            formation: Some(PLACE),
            order: None,
        });
        let id = traffic.add_npc(ship);
        (traffic, id)
    }

    #[test]
    fn the_reset_drops_a_disabled_behav_1_mission_escorts_parent_by_either_reading() {
        // +0x9a = -1 (@0x8feda), +0x88 left at the re-lead's 6.
        for reading in RuleSource::ALL {
            let (mut traffic, id) = mission_escort(ShipBehav::EscortPlayer, Condition::Disabled);
            ticked(&mut traffic, reading, &mut NeverFires);
            let dropped = of(&traffic, id);
            assert!(dropped.mission_dropped, "{reading:?}");
            assert!(!dropped.mission_escort(), "{reading:?}");
            assert!(!dropped.escorts_player(), "{reading:?}");
            assert_eq!(dropped.leader_ref(), None, "{reading:?}");
            assert_eq!(
                dropped.mission.map(|mark| mark.formation),
                Some(Some(PLACE)),
                "{reading:?}: its mark and its slot kept"
            );
            assert_eq!(dropped.original_ai_type(), 6, "{reading:?}");
            assert_eq!(dropped.ai_type, AiType::WimpyTrader, "{reading:?}");
        }
    }

    #[test]
    fn a_mission_escort_let_go_and_then_disabled_stays_let_go() {
        // The reset writes no +0x88: its `InherentAI` stands.
        let (mut traffic, id) = mission_escort(ShipBehav::EscortPlayer, Condition::Disabled);
        traffic.npcs_mut()[0].mission_let_go = true;
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let let_go = of(&traffic, id);
        assert!(let_go.mission_let_go);
        assert!(!let_go.mission_dropped);
        assert_eq!(let_go.original_ai_type(), 3);
    }

    #[test]
    fn a_disabled_captured_mission_ship_of_another_behav_is_out_of_the_formation_for_good() {
        // No re-lead's (@0x8fc07 → @0x8fc2d): +0x9a -1 for good, +0x88 6
        // until its first think whole.
        for behav in [ShipBehav::Own, ShipBehav::AttackPlayer] {
            for reading in RuleSource::ALL {
                let (mut traffic, id) = mission_escort(behav, Condition::Disabled);
                ticked(&mut traffic, reading, &mut NeverFires);
                let dropped = of(&traffic, id);
                let mark = dropped.mission.expect("its mark kept");
                assert_eq!(mark.formation, None, "{behav:?} {reading:?}");
                assert!(!dropped.mission_dropped, "{behav:?} {reading:?}");
                assert!(!dropped.mission_escort(), "{behav:?} {reading:?}");
                assert_eq!(dropped.ai_type, AiType::Warship, "{behav:?} {reading:?}");
                assert_eq!(dropped.original_ai_type(), 6, "{behav:?} {reading:?}");
                traffic.npcs_mut()[0].condition = Condition::Intact;
                traffic.tick_in(&NovaAi::default(), World::new(&[]), &[], &mut NeverFires);
                let whole = of(&traffic, id);
                assert_eq!(whole.original_ai_type(), 3, "{behav:?} {reading:?}");
                assert!(!whole.mission_escort(), "{behav:?} {reading:?}");
            }
        }
    }

    #[test]
    fn a_disabled_behav_2_mission_escort_reads_six_for_its_stay() {
        // `_DeathStarAI` writes no +0x88 (@0x8ff3a).
        let (mut traffic, id) = mission_escort(ShipBehav::AttackStellars, Condition::Disabled);
        ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
        let dropped = of(&traffic, id);
        assert_eq!(dropped.mission.and_then(|mark| mark.formation), None);
        assert_eq!(dropped.original_ai_type(), 6);
        traffic.npcs_mut()[0].condition = Condition::Intact;
        traffic.tick_in(&NovaAi::default(), World::new(&[]), &[], &mut NeverFires);
        assert_eq!(of(&traffic, id).original_ai_type(), 6);
    }

    #[test]
    fn an_intact_mission_escort_or_one_on_the_jump_or_glide_path_is_not_dropped() {
        let jumping = |npc: &mut Npc| npc.warp.goal = Some(WarpGoal::Follow(ShipRef::Player));
        let gliding = |npc: &mut Npc| npc.mode = Mode::JumpingIn { tick: 0 };
        let intact = |npc: &mut Npc| npc.condition = Condition::Intact;
        let cases: [(&str, Setup); 3] = [
            ("jumping", &jumping),
            ("gliding", &gliding),
            ("intact", &intact),
        ];
        for (name, setup) in cases {
            for behav in [ShipBehav::EscortPlayer, ShipBehav::Own] {
                let (mut traffic, id) = mission_escort(behav, Condition::Disabled);
                setup(&mut traffic.npcs_mut()[0]);
                ticked(&mut traffic, RuleSource::Engine, &mut NeverFires);
                let led = of(&traffic, id);
                assert!(led.mission_escort(), "{name} {behav:?}");
                assert!(!led.mission_dropped, "{name} {behav:?}");
            }
        }
    }

    #[test]
    fn a_paused_disabled_ship_released_on_the_draw_is_reset_in_the_same_tick() {
        let mut traffic = fleet(Condition::Disabled);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        led.expect("there").goal = Goal::Regroup { kept: Kept::Coast };
        let mut chance = Draws::of(&[]).keyed(RELEASE_DRAW, 0);
        traffic.think_holding(
            &Keep,
            World {
                npc_escort_ai: RuleSource::Engine,
                ..World::new(&[])
            },
            &[],
            &mut chance,
        );
        let led = of(&traffic, LED);
        assert!(was_reset(led), "{led:?}");
        assert_eq!(led.leader, None);
    }
}
