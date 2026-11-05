//! Tests of [`Npc::fought`] in the states whose goal names no ship and
//! which the threat tests don't spare: the mine (0x10), the scoop (0x11),
//! the regroup pause (0x13), the drift (0xe) and the count wait (0x16).
//! `_IsThreatToPlayer` (@0x7f536-0x7f56b), `_IsThreatToShip`
//! (@0x82024-0x82055) and `_ExtendedIsThreatToShip` (@0x81ace-0x81b03)
//! read +0x70 in each of them.

use crate::ai::regroup::Kept;
use crate::ai::{Goal, Help};
use crate::catalog::StellarId;
use crate::combat::ShipRef;
use crate::stats::ShipStats;
use crate::traffic::fixture;
use crate::traffic::npc::{Npc, NpcId};

/// The asker, NPC 1.
const ASKER: NpcId = NpcId(1);
/// NPC 2, which the asker leads.
const ASKERS: NpcId = NpcId(2);
/// NPC 9, which the player leads.
const PLAYERS: NpcId = NpcId(9);

/// The asker and the ship it leads.
fn npcs() -> [Npc; 2] {
    [
        fixture::npc(1, ShipStats::default()),
        Npc {
            leader: Some(ASKER),
            ..fixture::npc(2, ShipStats::default())
        },
    ]
}

/// Whether an NPC is led by the player: [`PLAYERS`] alone.
fn led(id: NpcId) -> bool {
    id == PLAYERS
}

/// NPC 4 in `goal`, holding `target`.
fn holding(goal: Goal, target: Option<ShipRef>) -> Npc {
    Npc {
        goal,
        target,
        ..fixture::npc(4, ShipStats::default())
    }
}

/// Asserts that a ship in `goal` threatens the asker, the ship it leads,
/// the player and the ship the player leads by the target it holds, and
/// nothing with none.
fn threatens_by_its_target(goal: Goal) {
    let npcs = npcs();
    for held in [ASKER, ASKERS] {
        let ship = holding(goal, Some(ShipRef::Npc(held)));
        assert!(
            ship.threatens_npc_among(ASKER, &npcs),
            "{goal:?} on {held:?}"
        );
    }
    assert!(
        !holding(goal, None).threatens_npc_among(ASKER, &npcs),
        "{goal:?}: no target"
    );
    for held in [ShipRef::Player, ShipRef::Npc(PLAYERS)] {
        assert!(
            holding(goal, Some(held)).threatens_player_or_led(led),
            "{goal:?} on {held:?}"
        );
    }
    assert!(
        !holding(goal, None).threatens_player_or_led(led),
        "{goal:?}: no target"
    );
}

#[test]
fn a_mining_or_scooping_ship_threatens_by_the_target_it_holds() {
    // States 0x10 and 0x11: the handler's blocks write no +0x70
    // (@0x8f832-0x8f903, @0x8faf3-0x8fafd), and the tests spare neither.
    threatens_by_its_target(Goal::Mine);
    threatens_by_its_target(Goal::Scoop);
}

#[test]
fn a_regrouping_ship_threatens_by_the_target_it_kept() {
    // State 0x13 runs no think and no high level (@0x8fe73-0x8feb5), so
    // +0x70 is kept from before the pause.
    threatens_by_its_target(Goal::Regroup { kept: Kept::Coast });
}

#[test]
fn a_drifting_or_count_waiting_ship_threatens_an_npc_but_not_the_player_while_its_wait_runs() {
    // `_ExtendedIsThreatToShip` has no wait test (@0x81a27-0x81b59);
    // `_IsThreatToPlayer` spares a ship whose +0x4c runs (@0x7f518-0x7f51b).
    let npcs = npcs();
    for goal in [Goal::Drift, Goal::CountWait] {
        let waiting = |target| Npc {
            ai_wait: 30.0,
            ..holding(goal, target)
        };
        assert!(
            waiting(Some(ShipRef::Npc(ASKER))).threatens_npc_among(ASKER, &npcs),
            "{goal:?}"
        );
        assert!(
            !waiting(Some(ShipRef::Player)).threatens_player_or_led(led),
            "{goal:?}: the wait runs"
        );
        let out = Npc {
            ai_wait: 0.0,
            ..waiting(Some(ShipRef::Player))
        };
        assert!(
            out.threatens_player_or_led(led),
            "{goal:?}: the wait is out"
        );
        threatens_by_its_target(goal);
    }
}

#[test]
fn a_spared_goal_holding_a_target_is_no_threat() {
    // A regression pin for `fought`'s spared branch: every goal the three
    // tests spare reads no +0x70, whatever it holds.
    let ship = ShipRef::Npc(ASKER);
    for goal in [
        Goal::Inspect(ship),
        Goal::Assist(Help::Refuel),
        Goal::Assist(Help::Repair),
        Goal::Dock(ship),
        Goal::Follow {
            lead: ASKER,
            guard: Some(ship),
        },
        Goal::Formation { guard: Some(ship) },
        Goal::Hold,
        Goal::AttackStellar(StellarId(128)),
    ] {
        assert!(goal.spares_player_threat(), "{goal:?}");
        assert_eq!(holding(goal, Some(ship)).fought(), None, "{goal:?}");
    }
}
