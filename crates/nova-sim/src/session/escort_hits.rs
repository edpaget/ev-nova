//! Tests of the player's escorts answering hits through the session's
//! tick, as `NovaAi` flies them (`EscortAi`): `_DamageShip` makes the
//! attacker an escort's target (@0x3bcf6), and `_EscortAI` keeps it by
//! order, the player included (@0x83c67-0x83cd6, @0x83d5d-0x83d6d); out
//! of ammunition they fall back (@0x83b88-0x83bda).

use super::Session;
use super::fighters::tests::{VIPERS, carrying, fight, fleet, out, pirate, sitting_ai, warship};
use crate::ai::Goal;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::combat::fixture::strike;
use crate::escort::EscortOrder;
use crate::geometry::Vec2;
use crate::pilot::Escort;
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::NpcId;

/// A session with the warship escort following `order`; the session and
/// the escort.
fn escorted(catalog: &FakePilotCatalog, order: EscortOrder) -> (Session, NpcId) {
    let session = fleet(catalog, 0, vec![warship(Some(order))]);
    let escort = session.space.escorts.fleet[0].expect("placed");
    (session, escort)
}

/// The goal and target of `session`'s NPC `id`.
fn fighting(session: &Session, id: NpcId) -> (Goal, Option<ShipRef>) {
    let npc = session.space.npc(id).expect("there");
    (npc.goal, npc.target)
}

#[test]
fn a_defending_or_attacking_escort_the_player_hits_turns_on_the_player_and_keeps_at_it() {
    // `_AIMakeShipAttackPlayer` (goal 4, +0x70 = 0, @0x89d3e-0x89d4d);
    // then each think keeps it: Defend measures the player from itself,
    // 0, and Attack keeps any target left.
    let catalog = carrying();
    for order in [EscortOrder::Defend, EscortOrder::Attack] {
        let (mut session, escort) = escorted(&catalog, order);
        session.space.strikes = vec![strike(ShipRef::Npc(escort), ShipRef::Player)];
        for tick in 0..10 {
            session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
            assert_eq!(
                fighting(&session, escort),
                (Goal::Attack(ShipRef::Player), Some(ShipRef::Player)),
                "{order:?}: tick {tick}"
            );
        }
    }
}

/// Runs `session`'s NPC `id` out of ammunition: no weapon left, its hull
/// of `Flags2` 0x0080 (`HullSpec::retreats_dry`).
fn run_dry(session: &mut Session, id: NpcId) {
    let npc = session.space.npc_mut(id).expect("there");
    npc.armament = Armament::default();
    npc.hull.retreats_dry = true;
}

/// Adds a pirate 200 above the player, attacking it.
fn raider(session: &mut Session) -> NpcId {
    let raider = pirate(session, 0.0, -200.0);
    let npc = session.space.npc_mut(raider).expect("there");
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    raider
}

#[test]
fn a_fighter_on_attack_out_of_ammunition_returns_to_its_hangar() {
    // `_EscortAI`'s fallback writes word 3 for AI type 5 (@0x83bc9), and
    // the Return block docks it (@0x83e79-0x83ea3).
    let catalog = carrying();
    let attacking = Escort {
        order: Some(EscortOrder::Attack),
        ..out()
    };
    let mut session = fleet(&catalog, 0, vec![attacking]);
    let fighter = session.space.escorts.fleet[0].expect("out");
    raider(&mut session);
    run_dry(&mut session, fighter);
    // Out of its carrier's reach, so it flies home first.
    session.space.npc_mut(fighter).expect("out").state.position = Vec2::new(0.0, 600.0);
    fight(&mut session, &catalog, &sitting_ai());
    assert_eq!(
        session.space.npc(fighter).expect("out").goal,
        Goal::Dock(ShipRef::Player)
    );
    let docked = (0..600).any(|_| {
        fight(&mut session, &catalog, &sitting_ai());
        session.pilot().owned(VIPERS) == 1
    });
    assert!(docked, "into the player's bay");
}

#[test]
fn an_escort_with_no_firepower_on_attack_keeps_formation() {
    // Any other type takes word 0 fully dry (@0x83bda): formation.
    let catalog = carrying();
    let (mut session, escort) = escorted(&catalog, EscortOrder::Attack);
    let raider = raider(&mut session);
    fight(&mut session, &catalog, &sitting_ai());
    assert_eq!(
        fighting(&session, escort).0,
        Goal::Attack(ShipRef::Npc(raider)),
        "armed or not, without Flags2 0x0080 it fights"
    );
    run_dry(&mut session, escort);
    fight(&mut session, &catalog, &sitting_ai());
    assert!(
        matches!(fighting(&session, escort).0, Goal::Formation { .. }),
        "{:?}",
        fighting(&session, escort)
    );
}
