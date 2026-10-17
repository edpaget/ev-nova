//! The session's tests of the fire gates (rdm weapons-fidelity phase 17):
//! a waiting NPC holds its fire through the fight, as `_HandleShip` skips
//! its fire block while its AI wait +0x4c runs (@0x34a54-0x34a72); see
//! [`fire::gated`](crate::ai::fire::gated).

use super::tests::{Attacking, armed, facing_an_npc};
use super::*;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::traffic::npc::Mode;

/// How many times NPC 0 fired in `events`.
fn npc_fired(events: &[CombatEvent]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, CombatEvent::Fired { ship, .. } if *ship == ShipRef::Npc(NpcId(0))))
        .count()
}

/// A trigger firing weapon 128 alone, kept as the pick.
const PICKED: Trigger = Trigger {
    primary: false,
    secondary: None,
    only: Some(WeaponId(128)),
    turrets_only: false,
    bays: false,
    point_defence: false,
    keep: Some(WeaponId(128)),
};

/// The session facing its armed NPC 100 above the player, the NPC facing
/// it in `mode`, holding [`PICKED`] with weapon 128 its pick, targeting
/// the player.
fn waiting(mode: Mode) -> (crate::testkit::FakePilotCatalog, Session) {
    let catalog = armed();
    let mut session = facing_an_npc(&catalog, 180);
    let npc = &mut session.traffic.npcs_mut()[0];
    npc.mode = mode;
    npc.trigger = PICKED;
    npc.pick = Some(WeaponId(128));
    npc.target = Some(ShipRef::Player);
    (catalog, session)
}

#[test]
fn a_ship_holding_its_course_after_its_burst_fires_nothing_then_fires_its_kept_pick() {
    let (catalog, mut session) = waiting(Mode::Coasting { ticks_left: 3 });
    let mut waited = 0;
    while session.npcs()[0].mode != Mode::Flying {
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(npc_fired(&session.take_combat_events()), 0, "it waits");
        assert!(session.shots().is_empty(), "no shot spawned");
        assert_eq!(session.npcs()[0].pick, Some(WeaponId(128)), "kept");
        session.tick_traffic(&catalog, &Attacking, &mut NeverFires);
        waited += 1;
        assert!(waited <= 3, "the wait ends");
    }
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(
        npc_fired(&session.take_combat_events()),
        1,
        "its first decision after the wait fires"
    );
}

#[test]
fn a_ship_entering_a_gate_fires_nothing_it_held() {
    let (_, mut session) = waiting(Mode::EnteringGate {
        gate: StellarId(128),
        ticks_left: 10,
    });
    for _ in 0..5 {
        session.tick_combat(Rules::default(), &mut NeverFires);
        assert_eq!(npc_fired(&session.take_combat_events()), 0);
        assert_eq!(session.npcs()[0].pick, Some(WeaponId(128)), "kept");
    }
    assert!(session.shots().is_empty());
}
