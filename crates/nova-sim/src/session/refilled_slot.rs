//! The session's tests of the wiring of a shot whose ship's slot a new ship refills (see
//! `nova_sim_combat`'s `refilled_slot` module, which pins the rule):
//! `Session::with_rules` hands the fight
//! `RuleKey::ReusedSlotShotTarget`, and the fight's phases follow a
//! berth the traffic gave a newcomer before the shot's next tick.

use super::tests::{arming, blaster, homing_missile, with_an_npc_at};
use super::*;
use crate::combat::Combat;
use crate::combat::hull::Condition;
use crate::flight::Controls;
use crate::testkit::work;

#[test]
fn the_rulebook_says_what_becomes_of_a_shot_whose_ships_slot_is_refilled() {
    let catalog = arming(homing_missile(134, 1000), blaster(), 0);
    let session = |rules: Rulebook| Session::start(&catalog).expect("starts").with_rules(rules);
    let mut lost = Combat::default();
    lost.set_reused_slot_shot_target(RuleSource::Bible);
    assert_ne!(lost, Combat::default());
    let bible = Rulebook::default().with_override(RuleKey::ReusedSlotShotTarget, RuleSource::Bible);
    assert_eq!(session(bible).space.combat, lost);
}

/// The newcomer's shield after the player's homing missile, fired at
/// NPC 0 100 pixels above it, flies on once a newcomer took NPC 0's
/// berth in the ship loop past its wreck, by `rules`.
fn newcomers_shield(rules: Rulebook) -> f32 {
    let catalog = arming(homing_missile(134, 1000), blaster(), 0);
    let mut session = with_an_npc_at(&catalog, 750, 650, 0).with_rules(rules);
    assert_eq!(session.npcs()[0].state.position, Vec2::new(0.0, -100.0));
    session.select_target(TargetPick::Nearest);
    session.hold_fire(true, false);
    work(&mut session, &catalog, Controls::default());
    session.hold_fire(false, false);
    assert_eq!(session.shots().len(), 1, "the missile is in flight");
    let traffic = &mut session.space.traffic;
    let newcomer = traffic.npcs()[0].clone();
    let wreck = traffic.npcs_mut().first_mut().expect("there");
    wreck.condition = Condition::Destroyed;
    let berth = wreck.berth;
    traffic.set_ship_loop(true);
    let newcomer = traffic.add_npc_at(newcomer, berth);
    traffic.set_ship_loop(false);
    traffic.remove_destroyed(NpcId(0));
    for _ in 0..20 {
        work(&mut session, &catalog, Controls::default());
    }
    let npc = session.npcs().iter().find(|npc| npc.id == newcomer);
    npc.expect("in the fight").reserves.shield.now
}

#[test]
fn a_missile_whose_targets_berth_a_newcomer_took_hits_the_newcomer() {
    assert!(newcomers_shield(Rulebook::default()) < 30.0);
    let bible = Rulebook::default().with_override(RuleKey::ReusedSlotShotTarget, RuleSource::Bible);
    assert!(newcomers_shield(bible) >= 30.0, "lost, it hits nothing");
}
