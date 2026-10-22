//! Tests of an escort's think ending its hold at its gate, through the
//! traffic's think half ([`Traffic::think_holding`]) as [`NovaAi`] flies
//! it: `_EscortAI`'s Defend, Attack and formation blocks write +0x4c =
//! -1.0 (@0x83c73, @0x83d6a, @0x83eb0), the wait a ship held at its gate
//! runs on ([`Mode::HeldAtGate`]), so it flies at its first such think;
//! Hold and Return for a fighter keep it (@0x83e5f-0x83e74,
//! @0x83e79-0x83ea3). The player's hit holds it there first
//! ([`Npc::hold_at_gate`]), as `_DamageShip` runs before `_AIDispatch`.

use super::{Traffic, World};
use crate::ai::nova::NovaAi;
use crate::ai::{Goal, fire::AmmoLeft};
use crate::bay::Carrier;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::escort::{EscortDuty, EscortOrder, FormationSlot};
use crate::fleet::OrderWord;
use crate::geometry::Vec2;
use crate::stats::ShipStats;
use crate::traffic::fixture::npc;
use crate::traffic::npc::{Mode, Npc, NpcId};
use nova_sim_base::fixture::FAST;

/// A world of no stellars, the player 400 below the centre.
fn world() -> World<'static> {
    World {
        player: Some(crate::ai::fixture::player(0.0, 400.0)),
        ..World::new(&[])
    }
}

/// NPC `id` waiting in its gate at the centre for 60 more ticks.
fn waiting(id: u32) -> Npc {
    let mut ship = npc(id, ShipStats::new(FAST, &[]));
    ship.mode = Mode::Emerging { ticks_left: 60 };
    ship
}

/// The player's escort, NPC 1, waiting in its gate, following `order`.
fn players_escort(order: Option<EscortOrder>) -> Npc {
    let mut ship = waiting(1);
    ship.escort = Some(EscortDuty {
        formation: FormationSlot::default(),
        order,
    });
    ship
}

/// `ship` as a fighter carried by `carrier`.
fn carried(mut ship: Npc, carrier: ShipRef) -> Npc {
    ship.carrier = Some(Carrier {
        ship: carrier,
        window: 100.0,
        reach: 40.0,
    });
    ship
}

/// `ships` once the player's hit on NPC 1 has held it at its gate, then
/// once the next think half has run, [`NovaAi`] deciding.
fn hit_then_think(ships: Vec<Npc>) -> (Npc, Npc) {
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = ships;
    let ai = NovaAi::default();
    let hit = strike(ShipRef::Npc(NpcId(1)), ShipRef::Player);
    traffic.answer(&ai, &world(), &[hit]);
    let held = traffic.npcs[0].clone();
    traffic.think_holding(&ai, world(), &[], &mut NeverFires);
    (held, traffic.npcs[0].clone())
}

#[test]
fn a_players_escort_held_at_its_gate_ends_its_hold_at_its_first_defend_attack_or_formation_think() {
    for order in [Some(EscortOrder::Defend), Some(EscortOrder::Attack), None] {
        let (held, thought) = hit_then_think(vec![players_escort(order)]);
        assert!(
            matches!(held.mode, Mode::HeldAtGate { .. }),
            "{order:?}: {:?}",
            held.mode
        );
        assert_eq!(thought.mode, Mode::Flying, "{order:?}");
        assert!(!thought.wait_running(), "{order:?}");
        if order.is_some() {
            assert_eq!(thought.goal, Goal::Attack(ShipRef::Player), "{order:?}");
        } else {
            assert!(
                matches!(thought.goal, Goal::Formation { .. }),
                "{:?}",
                thought.goal
            );
        }
    }
}

#[test]
fn a_holding_escort_or_returning_fighter_stays_held_at_its_gate() {
    let holding = players_escort(Some(EscortOrder::Hold));
    let returning = carried(players_escort(Some(EscortOrder::Dock)), ShipRef::Player);
    for (ship, goal) in [
        (holding, Goal::Wait),
        (returning, Goal::Dock(ShipRef::Player)),
    ] {
        let (_, thought) = hit_then_think(vec![ship]);
        assert_eq!(thought.goal, goal);
        assert!(
            matches!(thought.mode, Mode::HeldAtGate { .. }),
            "{goal:?}: {:?}",
            thought.mode
        );
    }
}

/// NPC 2, the lead of NPC 1, at rest 300 off and thinking nothing of its
/// own.
fn lead() -> Npc {
    let mut lead = npc(2, ShipStats::new(FAST, &[]));
    lead.state.position = Vec2::new(300.0, 0.0);
    lead.no_ai = true;
    lead
}

#[test]
fn an_npc_led_escort_held_at_its_gate_ends_its_hold_at_its_first_think() {
    // `FleetEscortAi`, by the engine's `RuleKey::NpcEscortAi`: its word
    // -1 runs formation (@0x83ea8-0x83eb0).
    let mut escort = waiting(1);
    escort.leader = Some(NpcId(2));
    let (held, thought) = hit_then_think(vec![escort, lead()]);
    assert!(matches!(held.mode, Mode::HeldAtGate { .. }));
    assert_eq!(thought.mode, Mode::Flying);
    assert!(
        matches!(thought.goal, Goal::Follow { lead: NpcId(2), .. }),
        "{:?}",
        thought.goal
    );
}

#[test]
fn an_npc_carriers_fighter_out_of_ammunition_writes_its_return_and_docks() {
    // `_EscortAI`'s fallback writes word 3 (@0x83bc9), which the traffic
    // stores as the fighter's word, and the Return block docks it.
    let mut fighter = carried(npc(1, ShipStats::new(FAST, &[])), ShipRef::Npc(NpcId(2)));
    fighter.leader = Some(NpcId(2));
    fighter.order_word = OrderWord::of(Some(EscortOrder::Attack));
    let fighter = crate::ai::fixture::short_of_ammo(fighter, AmmoLeft::Dry, true);
    let mut traffic = Traffic::with_interval(1);
    traffic.npcs = vec![fighter, lead()];
    traffic.think_holding(&NovaAi::default(), world(), &[], &mut NeverFires);
    let thought = &traffic.npcs[0];
    assert_eq!(thought.goal, Goal::Dock(ShipRef::Npc(NpcId(2))));
    assert_eq!(thought.order_word, OrderWord(3));
}
