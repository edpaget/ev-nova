//! The fight's tests of `_HandleShip`'s idle-bay hold (@0x3390a-0x3394e,
//! see [`Armament::hold_idle_bays`]): an NPC with no target holds its
//! bays at their full `Reload` each tick, after its reload countdown; the
//! player never.

use super::*;

/// A bay of `Reload` 60 launching ship 144.
fn bay() -> WeaponRecord {
    WeaponRecord {
        guidance: 99,
        ammo_type: 144,
        reload: 60,
        ..weapon(149)
    }
}

/// `ship` carrying [`bay`], firing nothing.
fn carrying(ship: Ship) -> Ship {
    Ship {
        trigger: QUIET,
        ..ship.armed(bay())
    }
}

/// The bay's reload on `ships[at]`.
fn reload(ships: &[Ship], at: usize) -> f32 {
    ships[at].armament.mounts()[0].reload
}

#[test]
fn an_idle_npcs_bay_is_held_at_its_reload_and_counts_down_once_it_has_a_target() {
    let mut combat = Combat::default();
    let mut ships = [carrying(Ship::at(1, 0.0, 0.0)), Ship::at(2, 500.0, 0.0)];
    for _ in 0..3 {
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(reload(&ships, 0), 60.0, "held");
    }
    ships[0].target = Some(ShipRef::Npc(NpcId(2)));
    for left in [59.0, 58.0, 57.0] {
        phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
        assert_eq!(reload(&ships, 0), left, "counting down");
    }
}

#[test]
fn an_npc_whose_target_is_gone_from_the_fight_is_held_as_idle() {
    // `_HandleShip` clears a target whose slot is inactive just before
    // (@0x33865-0x33888).
    let mut combat = Combat::default();
    let mut ships = [carrying(Ship::at(1, 0.0, 0.0)).targeting(ShipRef::Npc(NpcId(9)))];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(reload(&ships, 0), 60.0);
    let mut ships = [carrying(Ship::at(1, 0.0, 0.0)).targeting(ShipRef::Player)];
    phase::handle_npcs(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(reload(&ships, 0), 60.0, "the player out of the fight too");
}

#[test]
fn the_players_bays_are_never_held() {
    let mut combat = Combat::default();
    let mut ships = [Ship {
        trigger: Trigger::default(),
        target: None,
        ..player_with(bay())
    }];
    phase::fire_player(&mut combat, &mut ships, &NovaDisable);
    assert_eq!(reload(&ships, 0), 0.0, "`_HandlePlayer` has no hold");
}
