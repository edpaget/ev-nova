//! Tests of the player's escort carriers: an escort attacking launches
//! the fighters it carries as any ship does (`_AILaunchFighter` from the
//! high level's attack block, @0x8e6fa), each led by the escort, of its
//! government and with its target, and each takes the escort's Defend or
//! Attack, or Return while the escort keeps formation (`_EscortAI`
//! @0x83b36-0x83b7f), docking back into the escort's bay. Its side for
//! hits is the player's, the root of its leads.

use super::Session;
use super::fighters::tests::{
    BAY, VIPER, WARSHIP, carrying, fight, fleet, pirate, sitting_ai, vipers, warship,
};
use crate::ai::Goal;
use crate::catalog::StockWeapon;
use crate::combat::ShipRef;
use crate::escort::{EscortCommand, EscortGroup, EscortOrder};
use crate::testkit::FakePilotCatalog;
use crate::traffic::npc::NpcId;

/// [`carrying`], the warship escort's class carrying a Viper Bay with two
/// Vipers aboard.
fn escort_carrying() -> FakePilotCatalog {
    let mut catalog = carrying();
    let hull = catalog
        .hulls
        .iter_mut()
        .find(|hull| hull.id == WARSHIP)
        .expect("the warship's");
    hull.weapons = vec![StockWeapon {
        weapon: BAY,
        count: 1,
        ammo: 2,
    }];
    catalog
}

/// A session with the warship escort following `order`, and a pirate
/// 200 above the player attacking it, `pirates` more far off; the
/// session, the escort and the pirate.
fn raided_escort(
    catalog: &FakePilotCatalog,
    order: EscortOrder,
    pirates: u32,
) -> (Session, NpcId, NpcId) {
    let mut session = fleet(catalog, 0, vec![warship(Some(order))]);
    let escort = session.space.escorts.fleet[0].expect("placed");
    assert_eq!(
        session
            .space
            .npc(escort)
            .and_then(crate::traffic::npc::Npc::escort_order),
        Some(order)
    );
    let raider = pirate(&mut session, 0.0, -200.0);
    let npc = session.space.npc_mut(raider).expect("there");
    npc.goal = Goal::Attack(ShipRef::Player);
    npc.target = Some(ShipRef::Player);
    for n in 0..pirates {
        pirate(&mut session, 3000.0 + n as f32, 3000.0);
    }
    (session, escort, raider)
}

/// The Vipers aboard NPC `carrier`.
fn aboard(session: &Session, carrier: NpcId) -> i32 {
    session
        .space
        .npc(carrier)
        .map_or(0, |npc| npc.rounds.slot(BAY))
}

/// Fights until a Viper is out, at most `limit` ticks: whether one is.
fn until_launched(session: &mut Session, catalog: &FakePilotCatalog, limit: u32) -> bool {
    (0..limit).any(|_| {
        fight(session, catalog, &sitting_ai());
        !vipers(session).is_empty()
    })
}

#[test]
fn an_escort_carrier_attacking_a_pirate_launches_a_fighter_it_leads() {
    let catalog = escort_carrying();
    let (mut session, escort, raider) = raided_escort(&catalog, EscortOrder::Attack, 0);
    assert_eq!(aboard(&session, escort), 2);
    assert!(until_launched(&mut session, &catalog, 5), "it launches");
    let launched = vipers(&session);
    assert_eq!(launched.len(), 1);
    let fighter = &launched[0];
    let carrier = session.space.npc(escort).expect("there");
    assert_eq!(carrier.goal, Goal::Attack(ShipRef::Npc(raider)));
    assert_eq!(fighter.leader, Some(escort), "led by the escort");
    assert_eq!(fighter.govt, carrier.govt, "of its government");
    assert_eq!(fighter.target, Some(ShipRef::Npc(raider)), "its target");
    assert_eq!(
        fighter.carrier.map(|bay| bay.ship),
        Some(ShipRef::Npc(escort))
    );
    assert_eq!(fighter.escort, None, "no escort of the player's itself");
    assert_eq!(aboard(&session, escort), 1, "a round from its bay");
    assert_eq!(session.pilot().escorts().len(), 1, "the fleet's records");
    let (_, terms) = session.fight_terms();
    let at = session
        .npcs()
        .iter()
        .position(|npc| npc.id == fighter.id)
        .expect("there");
    assert_eq!(terms[at].fleet, ShipRef::Player, "its root is the player");
}

#[test]
fn its_fighter_attacks_while_the_escort_attacks_or_defends() {
    let catalog = escort_carrying();
    for order in [EscortOrder::Attack, EscortOrder::Defend] {
        let (mut session, _, raider) = raided_escort(&catalog, order, 0);
        assert!(until_launched(&mut session, &catalog, 5), "{order:?}");
        fight(&mut session, &catalog, &sitting_ai());
        let fighter = &vipers(&session)[0];
        assert_eq!(
            fighter.goal,
            Goal::Attack(ShipRef::Npc(raider)),
            "{order:?}"
        );
        assert_eq!(fighter.ai_wait, 0, "its glide ended, {order:?}");
    }
}

#[test]
fn its_fighter_docks_once_the_escort_keeps_formation_or_holds_and_the_round_goes_back() {
    let catalog = escort_carrying();
    for command in [EscortCommand::Recall, EscortCommand::Hold] {
        let (mut session, escort, _) = raided_escort(&catalog, EscortOrder::Attack, 0);
        assert!(until_launched(&mut session, &catalog, 5), "{command:?}");
        let id = vipers(&session)[0].id;
        session.command_escorts(EscortGroup::All, command);
        fight(&mut session, &catalog, &sitting_ai());
        assert_eq!(
            session.space.npc(id).expect("out").goal,
            Goal::Dock(ShipRef::Npc(escort)),
            "{command:?}"
        );
        let docked = (0..600).any(|_| {
            fight(&mut session, &catalog, &sitting_ai());
            session.space.npc(id).is_none()
        });
        assert!(docked, "{command:?}: {:?}", session.space.npc(id));
        assert!(
            aboard(&session, escort) >= 1,
            "{command:?}: back in the escort's bay"
        );
        assert!(vipers(&session).iter().all(|npc| npc.ship == VIPER));
    }
}

#[test]
fn with_55_npcs_in_the_system_an_escort_carrier_launches_nothing() {
    let catalog = escort_carrying();
    // The escort, the pirate and 53 more: no berth below the reserve.
    // The room ledger counts its sorties with every other NPC's.
    let (mut session, _, _) = raided_escort(&catalog, EscortOrder::Attack, 53);
    assert_eq!(session.npcs().len(), 55);
    assert!(!until_launched(&mut session, &catalog, 5));
    let (mut session, _, _) = raided_escort(&catalog, EscortOrder::Attack, 52);
    assert!(until_launched(&mut session, &catalog, 5), "one berth left");
}
