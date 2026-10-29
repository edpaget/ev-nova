//! `_AIDispatch`'s disabled ship: its warp cancelled and its AI reset
//! ([`Traffic::reset_disabled`]).
//!
//! **The warp cancel** (@0x8fcc8-0x8fce9). Every dispatch of a ship that is
//! `_IsDisabled` (disabled or breaking up, [`Condition::disabled`]) with
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
//! - [`Npc::target`] none, [`Npc::travel_set`] cleared, [`Npc::defends`]
//!   none (a defence ship held disabled stops defending), [`Npc::provoked`]
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
//!   think, as the engine has no AI type 6 for it to keep meanwhile (rdm:phase/escort-fleet-fidelity-followups/phase-4-led-ships-read-as-ai-type-6);
//! - by the other reading it keeps its leader, as the port had it.
//!
//! The reset precedes goal 0x15's skip
//! (@0x8ff18), so a ship disabled in its gate is reset too; which of its
//! goals that ship keeps is
//! rdm:phase/escort-fleet-fidelity-followups/phase-9-left-out-of-leader-succession's.
//! Goal 0 is never throttled (@0x8fdee-0x8fe6f), so it repeats every
//! frame the ship is disabled: the leader is gone for good.
//!
//! **Ordering.** The original runs each step per ship in slot order, the
//! cancel and the reset inside each dispatch; the engine runs each over
//! all ships after the regroup pause's draws. A lower slot's think that
//! reads a higher slot's reset a tick early crosses no boundary the player
//! sees. The reset drops a disabled follower's leader after the formation
//! pass has numbered it ([`fleet_formation`](super::fleet_formation)), so
//! for that one tick its fleet's slots still count it, as the original's
//! do (it numbers disabled followers too, and the parent's slot
//! calculation runs in the frame before its reset): a lag no player sees.
//!
//! Nothing here plays a sound or draws a random number.

use super::Traffic;
use super::npc::Npc;
use super::pod_flight;
use super::regroup;
use crate::ai::pursuit::Pursuit;
use crate::ai::{Goal, dispatch_gate};
#[cfg(doc)]
use crate::combat::hull::Condition;
use crate::rulebook::RuleSource;

impl Traffic {
    /// Each disabled NPC's warp cancelled and its AI reset, by `reading` of
    /// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) (see the module
    /// docs).
    pub(crate) fn reset_disabled(&mut self, reading: RuleSource) {
        for npc in &mut self.npcs {
            cancel_warp(npc);
            if resets(npc) {
                reset(npc, reading);
            }
        }
    }
}

/// `npc`'s warp cancelled while it is disabled (see the module docs).
fn cancel_warp(npc: &mut Npc) {
    if !pod_flight::is_pod(npc) && !npc.no_ai && npc.condition.disabled() && npc.warp.stage > 0.0 {
        npc.warp.give_up();
    }
}

/// Whether `npc` reaches the reset (see the module docs).
fn resets(npc: &Npc) -> bool {
    npc.condition.disabled() && dispatch_gate::reaches_think(npc) && !regroup::paused(npc)
}

/// `npc`'s reset, by `reading` of
/// [`RuleKey::NpcEscortAi`](crate::RuleKey::NpcEscortAi) (see the module
/// docs).
fn reset(npc: &mut Npc, reading: RuleSource) {
    npc.target = None;
    npc.travel_set = false;
    npc.defends = None;
    npc.provoked = 0.0;
    npc.goal = Goal::Idle;
    npc.pursuit = Pursuit::default();
    npc.docking = None;
    if reading == RuleSource::Engine && npc.leader.is_some() {
        if npc.carrier.is_some() {
            npc.leader = None;
        } else {
            npc.leave_fleet();
        }
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
    use crate::stats::ShipStats;
    use crate::traffic::World;
    use crate::traffic::fixture::npc;
    use crate::traffic::npc::{AiType, Mode, NpcId};
    use crate::traffic::plunder_dock::{DockSub, Docking};
    use crate::traffic::regroup::RELEASE_DRAW;
    use crate::traffic::warp_out::{GIVEN_UP_STAGE, WarpGoal};
    use nova_sim_base::fixture::{Draws, FAST};

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
        led.travel_set = true;
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
            && !npc.travel_set
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
                    led.travel_set
                ),
                (
                    before.leader,
                    before.target,
                    before.goal,
                    before.provoked,
                    before.travel_set
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
            npc.goal = Goal::Regroup {
                kept: Kept::Attack(ShipRef::Player),
            };
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
            assert!(led.travel_set, "{name}");
        }
    }

    #[test]
    fn a_disabled_ship_on_its_jump_has_its_warp_cancelled_all_the_same() {
        // @0x8fcc8-0x8fce9, before the jump path (@0x8fd83).
        let mut traffic = fleet(Condition::Disabled);
        let led = traffic.npcs_mut().iter_mut().find(|npc| npc.id == LED);
        let led = led.expect("there");
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
            traffic.think_holding(&Keep, World::new(&[]), &[], &mut NeverFires);
            assert_eq!(
                of(&traffic, LED).warp.stage,
                after,
                "{condition:?} {stage} {no_ai}"
            );
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
