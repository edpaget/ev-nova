//! The fight's test of point defence's missile branch reading the
//! defender's parent (`_HandleShipPointDefense` @0x39401-0x39426): +0x9a
//! read with no test of how its ship stands.

use super::*;

/// The root of the defender's chain of leads, not in the fight.
const ROOT: ShipRef = ShipRef::Npc(NpcId(4));

#[test]
fn point_defence_engages_a_missile_at_a_parent_that_is_breaking_up() {
    // B's missile at A; C, led by A and of ROOT's fleet, defends it. A has
    // the Quad too, but breaks up, so only C can answer.
    let [lead, firer] = missile_attack(quad());
    let mut wing = Ship::at(3, -30.0, 0.0).armed(quad()).facing(0.0);
    wing.trigger = QUIET;
    wing.fleet = ROOT;
    wing.leader = Some(A);
    let mut ships = [lead, firer, wing];
    let mut combat = Combat::default();
    missile_in_flight(&mut combat, &mut ships);
    combat.take_events();
    ships[0].condition = Condition::Dying {
        ticks_left: 10,
        starting: false,
    };
    phase::defend_npcs(&mut combat, &mut ships, &NovaDisable);
    let events = combat.take_events();
    let by_c = |event: &CombatEvent| matches!(event, CombatEvent::Fired { ship, .. } if *ship == C);
    assert!(events.iter().any(by_c), "{events:?}");
}
