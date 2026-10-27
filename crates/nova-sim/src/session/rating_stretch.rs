//! The session's tests of the rating stretch (rdm cloaking-fidelity
//! phase 6): the fight's [`Field`] carries the pilot's combat rating
//! against `shïp` 128's `Strength`, so an NPC firing at the player reloads
//! stretched by it (`_FireAIShipWeapon` @0x88fb3-0x8901b, see
//! [`PlayerRating::stretch`]).

use super::tests::{armed, facing_an_npc};
use super::*;
use crate::chance::NeverFires;
use crate::combat::Rules;
use crate::combat::ai_fire::PlayerRating;

/// [`armed`], `shïp` 128 of `Strength` `strength`.
fn yardstick(strength: i16) -> crate::testkit::FakePilotCatalog {
    let mut catalog = armed();
    for hull in &mut catalog.hulls {
        if hull.id == ShipId(128) {
            hull.strength = strength;
        }
    }
    catalog
}

#[test]
fn the_fights_field_carries_the_pilots_rating_and_the_strength_of_shïp_128() {
    let catalog = yardstick(25);
    let mut session = facing_an_npc(&catalog, 180);
    session.pilot.combat_rating = 320_000;
    assert_eq!(
        session.field().player_rating,
        PlayerRating {
            rating: 320_000,
            yardstick: 25
        }
    );
    session.statics_mut().arsenal = crate::combat::armament::Arsenal::new(&[], Vec::new());
    assert_eq!(session.field().player_rating.yardstick, 0, "no shïp 128");
}

/// How many times NPC 0, picking its blaster (`Reload` 2) at the player,
/// fires in 14 ticks of the fight against a pilot of `rating`, `shïp` 128
/// of `Strength` 1.
fn shots_at_a_pilot_of(rating: i64) -> usize {
    let catalog = yardstick(1);
    let mut session = facing_an_npc(&catalog, 180);
    session.pilot.combat_rating = rating;
    let mut fired = 0;
    for _ in 0..14 {
        // The AI's decision each tick: fire the pick at the player.
        let npc = &mut session.space.traffic.npcs_mut()[0];
        npc.trigger = Trigger {
            only: Some(WeaponId(128)),
            keep: Some(WeaponId(128)),
            ..Trigger::default()
        };
        npc.pick = Some(WeaponId(128));
        npc.target = Some(ShipRef::Player);
        session.tick_combat(Rules::default(), &mut NeverFires);
        fired += session
            .take_combat_events()
            .iter()
            .filter(|event| {
                matches!(event, CombatEvent::Fired { ship, .. } if *ship == ShipRef::Npc(NpcId(0)))
            })
            .count();
    }
    fired
}

#[test]
fn an_npc_attacking_a_new_pilot_fires_less_often_than_at_a_veteran() {
    // A veteran, at 1600 strengths, is not stretched: the blaster fires
    // every other tick. A new pilot stretches it x1.75 to 3.5 ticks, so it
    // fires every fourth.
    assert_eq!(shots_at_a_pilot_of(1600), 7);
    assert_eq!(shots_at_a_pilot_of(0), 4);
}
