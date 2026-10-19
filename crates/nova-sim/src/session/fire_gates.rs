//! The session's tests of the fire gates (rdm weapons-fidelity phase 17):
//! a waiting NPC holds its fire through the fight, as `_HandleShip` skips
//! its fire block while its AI wait +0x4c runs (@0x34a54-0x34a72; see
//! [`fire::gated`](crate::ai::fire::gated)), and the traffic flies its
//! snipers by the rulebook's reading (see
//! [`snipe`](crate::traffic::snipe)), and its approaches and spawned
//! afterburners by its readings (see [`pursuit`](crate::ai::pursuit) and
//! [`afterburner`](crate::ai::afterburner)).

use super::tests::{
    Attacking, Missileer, armed, facing_an_npc, kept_missile_catalog, missiles_fired,
    with_an_npc_at,
};
use super::*;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::rulebook::{RuleSource, Rulebook};
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
    bays_after: false,
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

#[test]
fn the_traffic_steps_in_a_world_holding_the_rulebooks_snipe_brake_lead() {
    let catalog = armed();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::SnipeBrakeLead, source);
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        assert_eq!(session.world().snipe_brake_lead, source);
        let mut flown = None;
        session.step_traffic(|_, world| flown = Some(world.snipe_brake_lead));
        assert_eq!(flown, Some(source));
    }
}

#[test]
fn the_traffic_steps_in_a_world_holding_the_rulebooks_approach_lead_and_afterburner_yardstick() {
    let catalog = armed();
    for source in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default()
            .with_override(RuleKey::ApproachLead, source)
            .with_override(RuleKey::AfterburnerYardstick, source);
        let mut session = Session::start(&catalog).expect("starts").with_rules(rules);
        let world = session.world();
        assert_eq!(
            (world.approach_lead, world.afterburner_yardstick),
            (source, source)
        );
        let mut flown = None;
        session.step_traffic(|_, world| {
            flown = Some((world.approach_lead, world.afterburner_yardstick));
        });
        assert_eq!(flown, Some((source, source)));
    }
}

/// The session facing an NPC 200 above the player, carrying a short gun
/// and a secondary missile 134, that has fired the missile, kept it as its
/// pick and left the system, read by `reading`; and the new ship of the
/// same class, with no pick of its own, brought into its slot.
fn reused_slot(reading: RuleSource) -> (crate::testkit::FakePilotCatalog, Session) {
    let catalog = kept_missile_catalog();
    let rules = Rulebook::default().with_override(RuleKey::ReusedSlotPick, reading);
    let mut session = with_an_npc_at(&catalog, 750, 550, 180).with_rules(rules);
    session.tick_traffic(&catalog, &Missileer, &mut NeverFires);
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(missiles_fired(&mut session), 1, "the first ship's");
    assert_eq!(session.npcs()[0].pick, Some(WeaponId(134)), "kept");
    let mut next = session.npcs()[0].clone();
    next.pick = None;
    session.traffic.remove(NpcId(0));
    let id = session.traffic.add_npc(next);
    assert_eq!(session.npc(id).map(|npc| npc.berth), Some(1), "its slot");
    (catalog, session)
}

#[test]
fn a_new_ship_in_a_reused_berth_fires_its_predecessors_kept_missile_on_its_first_empty_gun_pass() {
    // The first missile still covers the player, so the missile pass
    // picks none; the gun pass, its gun out of reach, fires the pick the
    // slot kept (`_AIFireGun` @0x80213-0x8021d), as no spawn writes
    // +0x72.
    let (catalog, mut session) = reused_slot(RuleSource::Engine);
    assert_eq!(session.npcs()[0].pick, Some(WeaponId(134)), "the slot's");
    session.tick_traffic(&catalog, &Missileer, &mut NeverFires);
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(missiles_fired(&mut session), 1);
    let (catalog, mut session) = reused_slot(RuleSource::Bible);
    assert_eq!(session.npcs()[0].pick, None, "none of its own");
    session.tick_traffic(&catalog, &Missileer, &mut NeverFires);
    session.tick_combat(Rules::default(), &mut NeverFires);
    assert_eq!(missiles_fired(&mut session), 0);
}

#[test]
fn the_reused_slot_reading_outlives_a_new_traffic() {
    let catalog = armed();
    for reading in [RuleSource::Engine, RuleSource::Bible] {
        let rules = Rulebook::default().with_override(RuleKey::ReusedSlotPick, reading);
        let mut session = facing_an_npc(&catalog, 180).with_rules(rules);
        assert_eq!(session.traffic.reused_slot_pick(), reading);
        let stellar = session.sites[0].id;
        session
            .land_at(&catalog, session.pilot.system, stellar)
            .expect("relocates");
        assert_eq!(session.npcs(), [], "a new traffic");
        assert_eq!(session.traffic.reused_slot_pick(), reading, "re-applied");
    }
}

#[test]
fn a_captured_ships_slot_keeps_no_pick() {
    // `_DoShipCapture` clears the pick (@0x41207).
    let (_, mut session) = reused_slot(RuleSource::Engine);
    let id = session.npcs()[0].id;
    session.fleet_up(id, None);
    assert_eq!(session.npc(id).map(|npc| npc.pick), Some(None));
    let mut next = session.npc(id).expect("joined").clone();
    session.traffic.remove(id);
    next.pick = None;
    let again = session.traffic.add_npc(next);
    assert_eq!(session.npc(again).map(|npc| npc.pick), Some(None));
}
