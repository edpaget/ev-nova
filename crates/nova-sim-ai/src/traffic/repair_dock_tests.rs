//! Tests of a ship repairing the player, or an NPC, as the traffic's tick flies it (see
//! [`repair_dock`](crate::traffic::repair_dock)): its high level, the dock
//! it shares with the plunder, the completion and the drift after.
#![allow(clippy::float_cmp)]

use super::Traffic;
use crate::ai::Goal;
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::combat::hull::Condition;
use crate::flight::FlightRules;
use crate::geometry::Vec2;
use crate::hail::Help;
use crate::stats::ShipStats;
use crate::traffic::autopilot::TopSpeeds;
use crate::traffic::fixture::assisting::{Recording, player_in as player, world_with as world};
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Mode, Npc, NpcId};
use crate::traffic::plunder::{self, PirateEvent};
use crate::traffic::plunder_dock::{DockSub, Docking};
use crate::traffic::repair_dock::REPAIR_HOLD;
use nova_sim_base::fixture::{FAST, Scripted};
use nova_sim_catalog::catalog::ShipId;

const HELPER: NpcId = NpcId(1);

/// Traffic of one ship, NPC 1, repairing the player from (`x`, `y`), at
/// rest facing up, its class turn 3 (a box of 260), deciding every tick.
fn repairing(x: f32, y: f32) -> Traffic {
    let mut ship = npc(HELPER.0, ShipStats::new(FAST, &[]));
    ship.goal = Goal::Assist(Help::Repair);
    ship.target = Some(ShipRef::Player);
    ship.hull.class_turn = 3.0;
    ship.state.position = Vec2::new(x, y);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![ship];
    traffic
}

fn helper(traffic: &Traffic) -> &Npc {
    traffic
        .npcs()
        .iter()
        .find(|npc| npc.id == HELPER)
        .expect("in the system")
}

/// `traffic` a tick on with the player in `condition`, rolling `rolls`.
fn tick(traffic: &mut Traffic, behaviour: &Recording, condition: Condition, rolls: &[u16]) {
    traffic.tick_in(
        behaviour,
        world(condition),
        &[],
        &mut Scripted::rolling(rolls),
    );
}

#[test]
fn a_ship_repairing_the_disabled_player_docks_and_boards_it_and_repairs_it_after_its_wait() {
    // A dock started on tick 0 with wait W repairs on tick max(1, W - 100).
    for (roll, repair_tick) in [(0, 1), (79, 79)] {
        let mut traffic = repairing(2.0, 0.0);
        let recording = Recording::default();
        tick(&mut traffic, &recording, Condition::Disabled, &[roll]);
        assert_eq!(
            traffic.take_pirate_events(),
            [PirateEvent::PlayerDockedWith { pirate: HELPER }],
            "the start boards the player"
        );
        assert_eq!(helper(&traffic).ai_wait, 99 + u32::from(roll));
        for at in 1..repair_tick {
            tick(&mut traffic, &recording, Condition::Disabled, &[]);
            assert_eq!(traffic.take_pirate_events(), [], "{roll}: tick {at}");
            assert_eq!(helper(&traffic).goal, Goal::Assist(Help::Repair));
        }
        tick(&mut traffic, &recording, Condition::Disabled, &[]);
        assert_eq!(
            traffic.take_pirate_events(),
            [PirateEvent::PlayerRepaired {
                helper: HELPER,
                ship: ShipId(128)
            }],
            "{roll}"
        );
        let done = helper(&traffic);
        assert_eq!(done.goal, Goal::Drift);
        assert_eq!(done.ai_wait, REPAIR_HOLD - 1);
        assert_eq!((done.target, done.docking), (None, None));
    }
}

#[test]
fn a_repairer_unprovoked_flies_its_high_level_though_it_never_decides() {
    let mut traffic = repairing(1000.0, 0.0);
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    assert_eq!(*recording.decided.borrow(), []);
    assert_eq!(
        helper(&traffic).docking,
        Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Close
        })
    );
}

#[test]
fn a_repaired_ship_drifts_100_steps_then_decides_again() {
    let mut traffic = repairing(2.0, 0.0);
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[0]);
    tick(&mut traffic, &recording, Condition::Intact, &[]);
    assert_eq!(helper(&traffic).goal, Goal::Drift, "repaired on tick 1");
    let start = helper(&traffic).state.position;
    for step in 1..=100 {
        tick(&mut traffic, &recording, Condition::Intact, &[]);
        let expected = if step == 100 { Goal::Idle } else { Goal::Drift };
        assert_eq!(helper(&traffic).goal, expected, "{step}");
    }
    let drifted = helper(&traffic).state.position - start;
    assert!(
        (drifted - Vec2::new(0.0, -100.0 * plunder::DRIFT)).length() < 1e-3,
        "{drifted:?}"
    );
    tick(&mut traffic, &recording, Condition::Intact, &[]);
    assert_eq!(helper(&traffic).goal, Goal::Wait, "it decides afresh");
}

#[test]
fn a_repairer_gives_up_on_a_player_no_longer_disabled_but_not_while_its_wait_runs() {
    let recording = Recording::default();
    let mut far = repairing(1000.0, 0.0);
    tick(&mut far, &recording, Condition::Intact, &[]);
    let gave_up = helper(&far);
    assert_eq!(
        (gave_up.goal, gave_up.target, gave_up.docking),
        (Goal::Idle, None, None)
    );
    let mut docked = repairing(2.0, 0.0);
    tick(&mut docked, &recording, Condition::Disabled, &[79]);
    tick(&mut docked, &recording, Condition::Intact, &[]);
    let waiting = helper(&docked);
    assert_eq!(waiting.goal, Goal::Assist(Help::Repair));
    assert_eq!(
        waiting.docking,
        Some(Docking {
            ship: ShipRef::Player,
            sub: DockSub::Dock
        })
    );
}

#[test]
fn far_off_a_repairer_closes_on_the_player_through_sub_0xb() {
    let mut traffic = repairing(0.0, 600.0);
    let mut closing = helper(&traffic).clone();
    closing.docking = Some(Docking {
        ship: ShipRef::Player,
        sub: DockSub::Close,
    });
    let speeds = TopSpeeds::own_of(&closing);
    plunder::fly_docking(
        &mut closing,
        Some(&player(Condition::Disabled).state),
        speeds,
        FlightRules::default(),
        None,
        &mut Scripted::default(),
    );
    let recording = Recording::default();
    for _ in 0..3 {
        tick(&mut traffic, &recording, Condition::Disabled, &[]);
        let flown = helper(&traffic);
        assert_eq!(
            flown.docking.map(|docking| docking.sub),
            Some(DockSub::Close)
        );
    }
    let mut once = repairing(0.0, 600.0);
    tick(&mut once, &recording, Condition::Disabled, &[]);
    assert_eq!(helper(&once).state, closing.state);
}

#[test]
fn every_unprovoked_assister_flies_its_high_level_a_repairer_and_a_refueller_alike() {
    let mut traffic = repairing(1000.0, 0.0);
    let mut second = traffic.npcs[0].clone();
    second.id = NpcId(2);
    second.berth = 2;
    let mut refueller = traffic.npcs[0].clone();
    refueller.id = NpcId(3);
    refueller.berth = 3;
    refueller.goal = Goal::Assist(Help::Refuel);
    second.docking = None;
    refueller.docking = None;
    traffic.npcs.extend([second, refueller]);
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    let docking = |id: u32| {
        traffic
            .npcs()
            .iter()
            .find(|npc| npc.id == NpcId(id))
            .expect("here")
            .docking
    };
    assert_eq!(docking(2).map(|docking| docking.sub), Some(DockSub::Close));
    assert_eq!(
        docking(3).map(|docking| docking.sub),
        Some(DockSub::Close),
        "a refueller's high level runs too (@0x901e5)"
    );
    assert_eq!(*recording.decided.borrow(), []);
}

#[test]
fn a_repairer_coasting_or_disabled_flies_no_high_level() {
    let mut coasting = repairing(1000.0, 0.0);
    coasting.npcs[0].mode = Mode::Coasting { ticks_left: 30 };
    let mut disabled = repairing(1000.0, 0.0);
    disabled.npcs[0].condition = Condition::Disabled;
    // A disabled one is reset to goal 0 by its dispatch (@0x8fece).
    for (name, mut traffic, goal) in [
        ("coasting", coasting, Goal::Assist(Help::Repair)),
        ("disabled", disabled, Goal::Idle),
    ] {
        let recording = Recording::default();
        tick(&mut traffic, &recording, Condition::Disabled, &[]);
        let ship = helper(&traffic);
        assert_eq!(ship.docking, None, "{name}");
        assert_eq!(ship.goal, goal, "{name}");
    }
}

#[test]
fn a_repairer_with_no_ai_flies_no_high_level() {
    let mut traffic = repairing(1000.0, 0.0);
    traffic.npcs[0].no_ai = true;
    tick(
        &mut traffic,
        &Recording::default(),
        Condition::Disabled,
        &[],
    );
    let ship = helper(&traffic);
    assert_eq!(
        ship.docking, None,
        "AI type -1 is never dispatched (@0x4427e)"
    );
    assert_eq!(ship.goal, Goal::Assist(Help::Repair));
}

#[test]
fn a_repairer_flies_its_high_level_only_on_its_due_ticks() {
    // NPC 1 of traffic deciding every 4 ticks is due on ticks 1, 5, ...
    let mut traffic = Traffic::with_interval(4);
    traffic.npcs = repairing(1000.0, 0.0).npcs;
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    assert_eq!(helper(&traffic).docking, None, "tick 0 isn't due");
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    assert_eq!(
        helper(&traffic).docking.map(|docking| docking.sub),
        Some(DockSub::Close),
        "tick 1 is due"
    );
}

/// NPC 2 at (`x`, `y`) in `condition`.
fn other(x: f32, y: f32, condition: Condition) -> Npc {
    let mut ship = npc(2, ShipStats::new(FAST, &[]));
    ship.berth = 2;
    ship.state.position = Vec2::new(x, y);
    ship.condition = condition;
    ship
}

/// `traffic` a tick on with the player in `condition`, answering
/// `strikes`, rolling `rolls`.
fn tick_struck(
    traffic: &mut Traffic,
    behaviour: &(impl crate::ai::Behaviour + ?Sized),
    condition: Condition,
    strikes: &[crate::combat::Strike],
    rolls: &[u16],
) {
    traffic.tick_in(
        behaviour,
        world(condition),
        strikes,
        &mut Scripted::rolling(rolls),
    );
}

#[test]
fn an_npcs_hit_on_a_closing_repairer_makes_it_give_up_and_go_idle_still_provoked() {
    let mut traffic = repairing(1000.0, 0.0);
    traffic.npcs[0].travel_set = true;
    traffic.npcs.push(other(3000.0, 0.0, Condition::Intact));
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    assert_eq!(
        helper(&traffic).docking.map(|docking| docking.sub),
        Some(DockSub::Close)
    );
    tick_struck(
        &mut traffic,
        &recording,
        Condition::Disabled,
        &[strike(ShipRef::Npc(HELPER), ShipRef::Npc(NpcId(2)))],
        &[],
    );
    let gave_up = helper(&traffic);
    assert_eq!(
        (gave_up.goal, gave_up.target, gave_up.docking),
        (Goal::Idle, None, None),
        "its target intact, @0x8f702-0x8f732"
    );
    assert!(!gave_up.travel_set, "nav -1, @0x8f732");
    assert_eq!(gave_up.provoked, 24.0, "still provoked");
    assert!(
        !recording.decided.borrow().contains(&HELPER),
        "it never thought"
    );
    // The wimpy trader's next think reads target -1 as unprovoked
    // (`provoked_by` needs a live ship): no flight.
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    let next = helper(&traffic).goal;
    tick_struck(
        &mut traffic,
        &crate::ai::nova::NovaAi::default(),
        Condition::Disabled,
        &[],
        &[0; 8],
    );
    assert!(
        !matches!(helper(&traffic).goal, Goal::Flee(_)),
        "{:?} after {:?}",
        helper(&traffic).goal,
        next
    );
}

#[test]
fn a_repairer_hit_by_a_disabled_npc_docks_with_it_boards_it_and_repairs_it_silently() {
    let mut traffic = repairing(2.0, 0.0);
    traffic.npcs.push(other(0.0, 0.0, Condition::Disabled));
    let recording = Recording::default();
    // A shot fired before NPC 2 was disabled still names it (the
    // reaction reads no attacker condition, @0x3b408-0x3b46e).
    tick_struck(
        &mut traffic,
        &recording,
        Condition::Intact,
        &[strike(ShipRef::Npc(HELPER), ShipRef::Npc(NpcId(2)))],
        &[0],
    );
    assert_eq!(
        traffic.take_pirate_events(),
        [],
        "an NPC's board is no event"
    );
    let docked = helper(&traffic);
    assert_eq!(docked.target, Some(ShipRef::Npc(NpcId(2))));
    assert_eq!(
        docked.docking,
        Some(Docking {
            ship: ShipRef::Npc(NpcId(2)),
            sub: DockSub::Dock
        })
    );
    assert!(
        traffic.npcs()[1].boarded,
        "the dock start boards it, @0x86fc7"
    );
    tick(&mut traffic, &recording, Condition::Intact, &[]);
    assert_eq!(
        traffic.take_pirate_events(),
        [PirateEvent::NpcRepaired {
            helper: HELPER,
            ship: NpcId(2)
        }],
        "no PlayerRepaired"
    );
    let done = helper(&traffic);
    assert_eq!(done.goal, Goal::Drift);
    assert_eq!((done.target, done.docking), (None, None));
}

#[test]
fn the_players_hit_on_a_docked_repairer_turns_it_without_repairing_and_it_leaves_after_the_cut_wait()
 {
    let mut traffic = repairing(2.0, 0.0);
    let recording = Recording::default();
    tick(&mut traffic, &recording, Condition::Disabled, &[79]);
    assert_eq!(helper(&traffic).ai_wait, 178, "docked");
    let _ = traffic.take_pirate_events();
    tick_struck(
        &mut traffic,
        &recording,
        Condition::Disabled,
        &[strike(ShipRef::Npc(HELPER), ShipRef::Player)],
        &[],
    );
    let turned = helper(&traffic);
    assert_eq!(turned.goal, Goal::Attack(ShipRef::Player));
    assert_eq!(
        turned.ai_wait,
        crate::traffic::autopilot::HIT_WAIT - 1,
        "cut to 20, then a tick off"
    );
    let mut ticks = 1;
    while helper(&traffic).ai_wait > 0 {
        assert_eq!(
            helper(&traffic).docking.map(|docking| docking.sub),
            Some(DockSub::Dock),
            "held docked, tick {ticks}"
        );
        tick(&mut traffic, &recording, Condition::Disabled, &[]);
        assert_eq!(traffic.take_pirate_events(), [], "no repair, tick {ticks}");
        ticks += 1;
    }
    assert_eq!(ticks, crate::traffic::autopilot::HIT_WAIT);
    tick(&mut traffic, &recording, Condition::Disabled, &[]);
    assert_eq!(
        helper(&traffic).docking,
        None,
        "goal 4's high level drops it"
    );
    assert_eq!(traffic.take_pirate_events(), []);
}
