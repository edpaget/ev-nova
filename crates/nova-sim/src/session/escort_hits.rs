//! Tests of the player's escorts answering hits through the session's
//! tick, as `NovaAi` flies them (`EscortAi`): `_DamageShip` makes the
//! attacker an escort's target (@0x3bcf6), and `_EscortAI` keeps it by
//! order, the player included (@0x83c67-0x83cd6, @0x83d5d-0x83d6d); out
//! of ammunition they fall back (@0x83b88-0x83bda).

use super::Session;
use super::fighters::tests::{
    PIRATES, VIPERS, carrying, fight, fleet, out, pirate, sitting_ai, warship,
};
use crate::ai::Goal;
use crate::catalog::StellarId;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::combat::armament::Armament;
use crate::combat::fixture::strike;
use crate::escort::EscortOrder;
use crate::geometry::Vec2;
use crate::pilot::Escort;
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::{Mode, NpcId};

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

/// Adds a pirate at (`x`, `y`) from the player, attacking `target`.
fn pirate_at(session: &mut Session, x: f32, y: f32, target: ShipRef) -> NpcId {
    let at = session.player().position;
    let id = pirate(session, at.x + x, at.y + y);
    let npc = session.space.npc_mut(id).expect("there");
    npc.govt = Some(PIRATES);
    npc.goal = Goal::Attack(target);
    npc.target = Some(target);
    id
}

/// `session`'s NPC `by` hits its NPC `ship`, for the next traffic tick.
fn hits(session: &mut Session, ship: NpcId, by: NpcId) {
    session.space.strikes = vec![strike(ShipRef::Npc(ship), ShipRef::Npc(by))];
}

#[test]
fn a_defending_escort_turns_on_a_ship_that_hits_it_near_the_player() {
    // The hit makes the attacker its target (@0x3bcf6), and Defend keeps
    // it within 408375 (about 639) of the player (@0x83c67-0x83cd6),
    // though beyond the 550 it takes a new threat within and another
    // threatens the player nearer.
    let catalog = carrying();
    let (mut session, escort) = escorted(&catalog, EscortOrder::Defend);
    pirate_at(&mut session, 0.0, 500.0, ShipRef::Player);
    let attacker = pirate_at(&mut session, 0.0, -600.0, ShipRef::Npc(escort));
    hits(&mut session, escort, attacker);
    for tick in 0..5 {
        session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
        assert_eq!(
            fighting(&session, escort),
            (
                Goal::Attack(ShipRef::Npc(attacker)),
                Some(ShipRef::Npc(attacker))
            ),
            "tick {tick}"
        );
    }
    // Unhit, it would take the nearer threat.
    let (mut session, escort) = escorted(&catalog, EscortOrder::Defend);
    let near_again = pirate_at(&mut session, 0.0, 500.0, ShipRef::Player);
    pirate_at(&mut session, 0.0, -600.0, ShipRef::Npc(escort));
    session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
    assert_eq!(
        fighting(&session, escort).0,
        Goal::Attack(ShipRef::Npc(near_again))
    );
}

#[test]
fn a_formation_escort_guards_against_its_attacker_while_in_reach() {
    // Formation keeps its target while `_AIInGunRange` (@0x83eb3-0x83ee4),
    // and draws among the threats to the player otherwise (@0x83ef4).
    let catalog = carrying();
    let mut session = fleet(&catalog, 0, vec![warship(None)]);
    let escort = session.space.escorts.fleet[0].expect("placed");
    session
        .space
        .npc_mut(escort)
        .expect("there")
        .hull
        .stock_reach = Some(400.0);
    let at = session.space.npc(escort).expect("there").state.position - session.player().position;
    let attacker = pirate_at(&mut session, at.x + 300.0, at.y, ShipRef::Npc(escort));
    // A threat to the player, last in the draw (`NeverFires` draws the
    // last), so a draw would take it.
    pirate_at(&mut session, 0.0, -300.0, ShipRef::Player);
    hits(&mut session, escort, attacker);
    session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
    assert_eq!(
        fighting(&session, escort),
        (
            Goal::Formation {
                guard: Some(ShipRef::Npc(attacker))
            },
            Some(ShipRef::Npc(attacker))
        )
    );
}

#[test]
fn a_player_escort_waiting_in_its_gate_hit_by_an_npc_holds_its_point_defence() {
    // `_DamageShip` makes the attacker its target (@0x3bcf6), and its
    // wait's `_AIEscortFireUnprovoked` (@0x8521e-0x85236) runs
    // `_AIFireTurret`'s point defence on it (@0x80b0d).
    let catalog = carrying();
    let (mut session, escort) = escorted(&catalog, EscortOrder::Defend);
    session
        .space
        .npc_mut(escort)
        .expect("there")
        .emerge_from_gate(StellarId(128), 60);
    let attacker = pirate_at(&mut session, 0.0, -200.0, ShipRef::Npc(escort));
    hits(&mut session, escort, attacker);
    session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
    let npc = session.space.npc(escort).expect("there");
    assert!(npc.emerging(), "{:?}", npc.mode);
    assert_eq!(npc.target, Some(ShipRef::Npc(attacker)));
    assert!(npc.trigger.point_defence, "{:?}", npc.trigger);
}

#[test]
fn a_player_escort_held_at_its_gate_by_the_players_hit_flies_at_its_first_think() {
    // `_AIMakeShipAttackPlayer` holds it (goal 4, its wait kept), and its
    // Defend think writes +0x4c = -1.0 (@0x83c73): it flies this tick.
    let catalog = carrying();
    let (mut session, escort) = escorted(&catalog, EscortOrder::Defend);
    session
        .space
        .npc_mut(escort)
        .expect("there")
        .emerge_from_gate(StellarId(128), 60);
    session.space.strikes = vec![strike(ShipRef::Npc(escort), ShipRef::Player)];
    session.answer_strikes(&sitting_ai());
    let held = session.space.npc(escort).expect("there");
    assert!(
        matches!(held.mode, Mode::HeldAtGate { .. }),
        "{:?}",
        held.mode
    );
    session.tick_traffic(&catalog, &sitting_ai(), &mut NeverFires);
    let npc = session.space.npc(escort).expect("there");
    assert_eq!(npc.mode, Mode::Flying);
    assert!(!npc.wait_running());
    assert_eq!(npc.goal, Goal::Attack(ShipRef::Player));
}
