//! The fight's tests of when the ion lockout reads the charge (rdm
//! ship-handling-fidelity phase 23, see [`ion`]): the player's hold on
//! the charge as its frame's ion block found it, decided after its fire;
//! an NPC's on that charge plus the pass's doses, decided and applied
//! with its reload step, before its fire.

use super::*;

/// A blaster of `Seeker` 0x0020, firing every tick.
fn held_gun() -> WeaponRecord {
    WeaponRecord {
        reload: 0,
        seeker: ion::CANT_FIRE_IONIZED,
        ..blaster()
    }
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
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(
        fired(&combat.take_events(), ShipRef::Player),
        1,
        "fires first"
    );
    assert_eq!(ships[0].armament.mounts()[0].reload, ion::HELD_RELOAD);
}

#[test]
fn the_players_hold_is_decided_before_the_pass_doses_it() {
    // B's ionizing bolt reaches the player on its seventh tick: the
    // player is not held on that tick, though the dose lands before its
    // reload step, but on the next.
    let mut combat = Combat::default();
    let mut player = player_with(held_gun());
    player.ion_capacity = 100.0;
    let bolt = WeaponRecord {
        ionization: 100,
        ..blaster()
    };
    let mut ships = [player, Ship::at(2, 100.0, 0.0).armed(bolt).facing(270.0)];
    let mut dosed_on = None;
    for tick_no in 1..=12 {
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[1].trigger = QUIET;
        let _ = combat.take_events();
        let reload = ships[0].armament.mounts()[0].reload;
        if dosed_on.is_none() && ships[0].ion.charge > 0.0 {
            dosed_on = Some(tick_no);
            assert_eq!(reload, 0.0, "not held the tick it is dosed");
        } else if dosed_on.is_some() {
            assert_eq!(reload, ion::HELD_RELOAD, "held from the next");
            return;
        }
    }
    panic!("never dosed");
}

#[test]
fn an_npc_is_held_before_its_fire_on_the_fight_its_dose_lands() {
    let mut combat = Combat::default();
    let mut gunner = Ship::at(1, 0.0, 0.0).armed(held_gun());
    gunner.ion_capacity = 100.0;
    let bolt = WeaponRecord {
        ionization: 100,
        ..blaster()
    };
    let mut ships = [gunner, Ship::at(2, 100.0, 0.0).armed(bolt).facing(270.0)];
    let mut volleys = Vec::new();
    for _ in 1..=12 {
        tick(&mut combat, &mut ships, &NovaDisable);
        ships[1].trigger = QUIET;
        let events = combat.take_events();
        volleys.push((fired(&events, A), ships[0].ion.charge));
        if ships[0].ion.charge > 0.0 {
            break;
        }
    }
    let (last, charge) = volleys.pop().expect("a fight");
    assert_eq!(charge, 100.0, "dosed to a factor of 1");
    assert_eq!(last, 0, "held on the fight the dose lands");
    assert!(volleys.iter().all(|&(fired, _)| fired == 1), "{volleys:?}");
    // The hold's 1.0 stays until the NPC's next reload step counts it
    // down (@0x338f0), so it fires again on the first fight it isn't held.
    assert_eq!(ships[0].armament.mounts()[0].reload, ion::HELD_RELOAD);
    ships[0].ion.charge = 0.0;
    tick(&mut combat, &mut ships, &NovaDisable);
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
    tick(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(ships[0].ion.shed, 0.0);
    assert_eq!(ships[1].ion.shed, 0.0);
}
