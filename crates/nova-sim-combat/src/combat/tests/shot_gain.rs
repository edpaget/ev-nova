//! The fight's tests of the shots' readers of the frame's gain
//! ([`Field::speed_mult`], `_HandleShot`'s and `_SpawnShot`'s
//! `_gSpeedMult`): each visit's [`Guide`](crate::combat::projectile::Guide)
//! takes it ([`Combat::move_shot`]) with the fight's decay reading, the
//! missile lock reads it ([`lock`]), a spinning shot's slot keeps the
//! timer its visit left, and the ship's fire and a sub-munition's release
//! roll sensor interference over its range ([`Shot::roll_at_launch`]).
//! Each expected value is the shot's own rule at the same gain.

use super::escort_lost_in_place::player_at;
use super::shot_slots::{handle_in, seeking_shell, spent_shell, unguided_sub};
use super::*;
use crate::combat::projectile::{DecayStep, interference_range};
use crate::combat::{shot_decay, spin_timer};
use crate::speed_mult::SpeedMult;
use nova_sim_base::fixture::Draws;

const HALF: SpeedMult = SpeedMult::new(0.5);

/// A field at `gain`.
fn at(gain: SpeedMult) -> Field {
    Field {
        speed_mult: gain,
        ..Field::default()
    }
}

/// A's shot of `record` at (0, 0), moving (10, 5), not yet moved.
fn flying(record: WeaponRecord) -> Shot {
    Shot {
        velocity: Vec2::new(10.0, 5.0),
        ..Shot::launch(WeaponSpec::new(&record), A, A, &ShipState::default(), 90.0)
    }
}

/// Two ships far off, which no shot here meets.
fn far_off() -> [Ship; 2] {
    [Ship::at(1, -5000.0, 0.0), Ship::at(2, 5000.0, 0.0)]
}

#[test]
fn at_half_gain_the_fight_moves_a_shot_half_its_velocity_and_counts_its_life_down_half() {
    let mut combat = Combat::default();
    let shot = flying(blaster());
    combat.put_in_flight(shot);
    handle_in(&mut combat, &mut far_off(), &Arsenal::default(), at(HALF));
    let flown = combat.shots()[0];
    assert_eq!(
        flown.position,
        crate::flight::moved(shot.position, shot.velocity, HALF)
    );
    assert_eq!(flown.life, HALF.add_times(shot.life, -1.0));
}

#[test]
fn at_half_gain_by_the_bibles_decay_the_fight_steps_a_slug_by_its_own_rule() {
    let slug = WeaponRecord {
        decay: 1,
        count: 50,
        ..blaster()
    };
    let mut combat = Combat::default();
    combat.set_decay(DecayStep::EveryDecay);
    combat.put_in_flight(flying(slug));
    for _ in 0..2 {
        handle_in(&mut combat, &mut far_off(), &Arsenal::default(), at(HALF));
    }
    let (mut timer, mut steps) = (0.0, 0);
    for _ in 0..2 {
        (timer, steps) = shot_decay::pass(timer, steps, 1, DecayStep::EveryDecay, HALF);
    }
    let decayed = combat.shots()[0];
    assert_eq!((decayed.decay_timer, decayed.decay_steps), (timer, steps));
    assert_eq!(steps, 1, "the 2nd visit at 0.5 reaches 1");
}

#[test]
fn at_half_gain_a_spinning_shots_slot_keeps_the_timer_its_visit_left_for_the_next_shot() {
    let spinning = WeaponRecord {
        flags: weapon::SPINS,
        beam_width: 9,
        count: 50,
        ..blaster()
    };
    let mut combat = Combat::default();
    combat.put_in_flight(flying(spinning));
    handle_in(&mut combat, &mut far_off(), &Arsenal::default(), at(HALF));
    let left = spin_timer::pass(0.0, 9, HALF).0;
    assert_eq!(combat.shots()[0].spin_timer, left);
    assert_eq!(combat.slots.spin(0), left, "its slot keeps it");
    combat.clear();
    combat.put_in_flight(flying(spinning));
    assert_eq!(combat.shots()[0].spin_timer, left, "the next starts on it");
}

/// The player's homing missile at (-500, -100), aimed at NPC 3, `aged`
/// frames old, flown one pass at `gain` among the player and NPC 3: the
/// strikes it made, as (ship, lock).
fn lock_after(aged: u32, gain: SpeedMult) -> Vec<(ShipRef, bool)> {
    let mut combat = Combat::default();
    let target = ShipRef::Npc(NpcId(3));
    let mut ships = [player_at(&mut combat, 0.0, 300.0), Ship::at(3, 0.0, -100.0)];
    let mut shot = Shot::launch(
        WeaponSpec::new(&missile()),
        ShipRef::Player,
        ShipRef::Player,
        &ShipState {
            position: Vec2::new(-500.0, -100.0),
            ..ShipState::default()
        },
        90.0,
    )
    .aged(aged);
    shot.moved_from = Some(shot.position);
    shot.aimed_at = Some(target);
    shot.target = Some(target);
    combat.put_in_flight(shot);
    handle_in(&mut combat, &mut ships, &Arsenal::default(), at(gain));
    combat
        .take_strikes()
        .iter()
        .map(|strike| (strike.ship, strike.lock))
        .collect()
}

#[test]
fn at_half_gain_a_players_missile_aged_15_locks_on_its_next_pass() {
    let target = ShipRef::Npc(NpcId(3));
    assert_eq!(lock_after(15, HALF), [(target, true)], "15.5 > 15");
    assert_eq!(lock_after(15, SpeedMult::FULL), [], "16 is not past 30");
    assert_eq!(lock_after(30, SpeedMult::FULL), [(target, true)]);
}

/// An interfered missile: `Seeker` 0x0008.
fn interfered() -> WeaponRecord {
    WeaponRecord {
        seeker: weapon::SENSOR_INTERFERENCE,
        ..missile()
    }
}

#[test]
fn at_half_gain_a_ship_fires_an_interfered_missile_rolling_over_the_range_at_the_gain() {
    let mut combat = Combat::default();
    let mut ships = [
        Ship::at(1, -300.0, 0.0)
            .armed(interfered())
            .targeting(ShipRef::Npc(NpcId(3))),
        Ship::at(3, 0.0, -100.0),
    ];
    let field = Field {
        interference: 50,
        ..at(HALF)
    };
    let mut chance = Draws::of(&[]);
    handle_npcs_in_field(
        &mut combat,
        &mut ships,
        (&Arsenal::default(), field),
        &mut chance,
    );
    assert_eq!(combat.shots().len(), 1, "fired");
    assert_eq!(chance.times_asked(interference_range(HALF)), 1);
    assert_eq!(chance.times_asked(interference_range(SpeedMult::FULL)), 0);
}

#[test]
fn at_half_gain_a_shell_releases_an_interfered_sub_munition_rolling_over_the_range_at_the_gain() {
    let homing = WeaponRecord {
        guidance: 1,
        seeker: weapon::SENSOR_INTERFERENCE,
        ..unguided_sub()
    };
    let (shell, arsenal) = seeking_shell(1, homing);
    let mut combat = Combat::default();
    combat.put_in_flight(spent_shell(shell));
    let field = Field {
        interference: 50,
        ..at(HALF)
    };
    let mut chance = Draws::of(&[]);
    fly_shots_in_field(&mut combat, &mut far_off(), (&arsenal, field), &mut chance);
    assert_eq!(combat.shots().len(), 1, "released");
    assert_eq!(chance.times_asked(interference_range(HALF)), 1);
    assert_eq!(chance.times_asked(interference_range(SpeedMult::FULL)), 0);
}
