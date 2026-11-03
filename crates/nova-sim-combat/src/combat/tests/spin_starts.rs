//! A shot's spin start, drawn as `_SpawnShot` draws it (@0x3e96e-0x3e9b4;
//! see [`shot_frame`](crate::combat::shot_frame)): a ship's shot and a
//! sub-munition alike, only a spinning one's.

use super::*;
use crate::combat::shot_frame::{FIRST_FRAME_START, SPIN_STARTS};
use crate::combat::weapon::SPINS;

/// [`blaster`] of `Flags` `flags`.
fn flagged(flags: u16) -> WeaponRecord {
    WeaponRecord { flags, ..blaster() }
}

#[test]
fn a_spinning_shot_draws_its_start_as_it_is_launched_and_no_other_draws_one() {
    let mut chance = Ranged::of(&[(SPIN_STARTS, 20)]);
    assert_eq!(fired_in(flagged(SPINS), 0, &mut chance)[0].spin_start, 20);
    for flags in [SPINS | FIRST_FRAME_START, 0, FIRST_FRAME_START] {
        let mut chance = Ranged::of(&[(SPIN_STARTS, 20)]);
        assert_eq!(fired_in(flagged(flags), 0, &mut chance)[0].spin_start, 0);
        assert!(!chance.was_asked(SPIN_STARTS), "{flags:#06x}");
    }
}

#[test]
fn a_spinning_sub_munition_draws_its_own_start() {
    let (shell, _) = seeding(0, [0; 4]);
    let sub = WeaponRecord {
        id: WeaponId(148),
        flags: SPINS,
        ..vulnerable(0, [0; 4])
    };
    let arsenal = Arsenal::new(&[shell, sub], Vec::new());
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, 0.0, 0.0).armed(shell).targeting(B),
        Ship::at(2, 0.0, -900.0),
    ];
    let field = Field::default();
    let mut chance = Ranged::of(&[(SPIN_STARTS, 9)]);
    handle_npcs_in_field(&mut combat, &mut ships, (&arsenal, field), &mut chance);
    for _ in 0..4 {
        fly_shots_in_field(&mut combat, &mut ships, (&arsenal, field), &mut chance);
    }
    let subs = sub_shots(&combat);
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].spin_start, 9);
}
