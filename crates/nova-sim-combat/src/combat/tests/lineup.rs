//! The one lineup the collision passes and `_HandleShot`'s visits read the
//! ships through ([`Lineup`]): every landing's struck ship is read again
//! whole, as both loops of the original read the live ship record after
//! each `_DamageShip`, each view keeping the ship at its own place.

use super::escort_lost_in_place::{E, escort};
use super::*;
use crate::combat::lineup::Lineup;

const A: ShipRef = ShipRef::Npc(NpcId(1));

/// A at (0, 0) and the escort E at (0, -100).
fn ships() -> Vec<Ship> {
    vec![Ship::at(1, 0.0, 0.0), escort()]
}

#[test]
fn a_lineup_reads_each_ship_where_it_stands_and_again_where_it_is_seen() {
    let mut ships = ships();
    let fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let govts = Governments::default();
    let seen_at = [Vec2::new(5.0, 5.0), Vec2::new(50.0, 50.0)];
    let lineup = Lineup::new(&fighters, &seen_at, &govts, PassThrough::default());
    for (index, fighter) in fighters.iter().enumerate() {
        assert_eq!(lineup.stood[index], fighter.as_target());
        let seen = Target {
            position: seen_at[index],
            ..fighter.as_target()
        };
        assert_eq!(lineup.seen[index], seen);
        assert_eq!(lineup.sights[index], fighter.sight());
    }
    assert_eq!(lineup.sides().targets, &lineup.stood[..]);
    assert_eq!(lineup.seen_sides().targets, &lineup.seen[..]);
}

#[test]
fn a_struck_ship_is_read_again_whole_in_each_view_at_its_own_place() {
    let mut ships = ships();
    let mut fighters: Vec<Fighter> = ships.iter_mut().map(Ship::fighter).collect();
    let govts = Governments::default();
    let seen_at = [Vec2::new(5.0, 5.0), Vec2::new(50.0, 50.0)];
    let mut lineup = Lineup::new(&fighters, &seen_at, &govts, PassThrough::default());
    // A landing pushes E, takes its armour under the line and takes it
    // out of the player's fleet; the fighter has been put elsewhere for it.
    let struck = &mut fighters[1];
    struck.state.velocity = Vec2::new(3.0, -4.0);
    struck.state.position = Vec2::new(900.0, 900.0);
    struck.reserves.armor.now = 1.0;
    struck.leader = None;
    struck.fleet = E;
    // A is changed too, but nothing struck it.
    let unstruck = lineup.stood[0];
    fighters[0].state.velocity = Vec2::new(7.0, 7.0);
    lineup.refresh(&fighters, &[E], &NovaDisable);
    let reread = Target {
        disabled: true,
        ..fighters[1].as_target()
    };
    let stood = Target {
        position: Vec2::new(0.0, -100.0),
        ..reread
    };
    assert_eq!(lineup.stood[1], stood);
    let seen = Target {
        position: seen_at[1],
        ..reread
    };
    assert_eq!(lineup.seen[1], seen);
    assert_eq!(lineup.sights[1], fighters[1].sight());
    assert_eq!(lineup.stood[0], unstruck, "A was not struck");
    assert_eq!(fighters[0].ship, A);
}
