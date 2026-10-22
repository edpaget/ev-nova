//! Tests of the player's escorts answering hits through the session's
//! tick, as `NovaAi` flies them (`EscortAi`): `_DamageShip` makes the
//! attacker an escort's target (@0x3bcf6), and `_EscortAI` keeps it by
//! order, the player included (@0x83c67-0x83cd6, @0x83d5d-0x83d6d).

use super::Session;
use super::fighters::tests::{carrying, fleet, sitting_ai, warship};
use crate::ai::Goal;
use crate::chance::NeverFires;
use crate::combat::ShipRef;
use crate::combat::fixture::strike;
use crate::escort::EscortOrder;
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
