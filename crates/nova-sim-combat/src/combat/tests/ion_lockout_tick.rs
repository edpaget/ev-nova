//! The fight's tests of when the ion lockout reads the charge (rdm
//! ship-handling-fidelity phase 23, see [`ion`]): the player's hold on
//! the charge as its frame's ion block found it, decided after its fire;
//! an NPC's on that charge plus the pass's doses, decided and applied
//! with its reload step, before its fire. That the player's hold is
//! decided before the pass's collision passes dose it is the session's
//! order (`nova_sim::Session::pass_after_move`), which its ionization
//! tests pin.

use super::*;

/// A blaster of `Seeker` 0x0020, firing every tick.
fn held_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 0,
        seeker: ion::CANT_FIRE_IONIZED,
        ..blaster()
    }
}

/// B's ionizing bolt, a factor of 1 for a ship of `IonizeMax` 100, put
/// in flight with its last move ending on `at`.
fn bolt_landing_on(combat: &mut Combat, at: Vec2) {
    let bolt = WeaponRecord {
        ionization: 100,
        ..blaster()
    };
    let state = ShipState {
        position: at,
        ..ShipState::default()
    };
    let mut shot = Shot::launch(WeaponSpec::new(&bolt), B, B, &state, 270.0);
    shot.id = combat.next_id();
    shot.moved_from = Some(at + Vec2::new(15.0, 0.0));
    shot.age = 1;
    shot.firer_slot = Some(Slot::Berth(2));
    combat.put_in_flight(shot);
}

#[test]
fn the_player_is_held_on_the_charge_as_its_block_found_it() {
    let mut combat = Combat::default();
    let mut player = player_with(held_gun());
    player.ion_capacity = 100.0;
    // The block shed its last 0.5: the charge is none now, but it was
    // above none as the frame began.
    player.ion.shed = 0.5;
    let mut ships = [player];
    phase::fire_player(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events(), ShipRef::Player),
        1,
        "fires first"
    );
    assert_eq!(ships[0].armament.mounts()[0].reload, ion::HELD_RELOAD);
}

#[test]
fn an_npc_is_held_before_its_fire_on_the_fight_its_dose_lands() {
    // The collision passes dose it before its `_HandleShip`, whose reload
    // step reads the pass's doses so far (@0x338d6-0x338f0).
    let mut combat = Combat::default();
    let mut gunner = Ship::at(1, 0.0, 0.0).armed(held_gun());
    gunner.ion_capacity = 100.0;
    let mut ships = [gunner, Ship::at(2, 100.0, 0.0)];
    for _ in 0..3 {
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(fired(&combat.take_events(), A), 1, "uncharged, it fires");
    }
    bolt_landing_on(&mut combat, Vec2::ZERO);
    phase::test_hits(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].ion.charge, 100.0, "dosed to a factor of 1");
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events(), A),
        0,
        "held on the fight the dose lands"
    );
    // The hold's 1.0 stays until the NPC's next reload step counts it
    // down (@0x338f0), so it fires again on the first fight it isn't held.
    assert_eq!(ships[0].armament.mounts()[0].reload, ion::HELD_RELOAD);
    ships[0].ion.charge = 0.0;
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(fired(&combat.take_events(), A), 1, "freed");
}

#[test]
fn every_fighters_shed_is_cleared_as_the_fight_ends() {
    let mut combat = Combat::default();
    let mut player = player_with(blaster());
    player.ion.shed = 0.5;
    let mut npc = Ship::at(2, 100.0, 0.0);
    npc.ion.shed = 2.0;
    let mut ships = [player, npc];
    phase::close_tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].ion.shed, 0.0);
    assert_eq!(ships[1].ion.shed, 0.0);
}
