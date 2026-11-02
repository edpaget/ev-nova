//! A shot's float life and what reads it, and each of `_HandleShot`'s,
//! `_HandleShotGuidance`'s and `_SpawnShot`'s reads of the frame's gain
//! (rdm:phase/play-feel-8/phase-23-shots-read-the-gain).

#![allow(clippy::float_cmp)]

use super::*;
use crate::catalog::WeaponRecord;
use crate::chance::NeverFires;
use crate::combat::NpcId;
use crate::speed_mult::SpeedMult;
use nova_sim_base::fixture::Draws;
use nova_sim_catalog::fixture::weapon;

const HALF: SpeedMult = SpeedMult::new(0.5);

const FIRER: ShipRef = ShipRef::Npc(NpcId(1));
const TARGET: ShipRef = ShipRef::Npc(NpcId(2));

/// A weapon of `guidance`, `Count` `count`, 10 pixels a frame, turning 7
/// degrees a frame.
fn spec(guidance: i16, count: i16) -> WeaponSpec {
    WeaponSpec::new(&WeaponRecord {
        guidance,
        count,
        speed: 1000,
        guided_turn: 70,
        ..weapon(134)
    })
}

/// A shot of `weapon` at the centre heading right.
fn shot(weapon: WeaponSpec) -> Shot {
    Shot::launch(weapon, FIRER, FIRER, &ShipState::default(), 90.0)
}

/// Ship 2 at (`x`, `y`), intact.
fn target(x: f32, y: f32) -> Target {
    Target {
        ship: TARGET,
        ship_type: ShipId(128),
        fleet: TARGET,
        ties: Ties::default(),
        position: Vec2::new(x, y),
        velocity: Vec2::ZERO,
        radius: 5.0,
        sprite_x: 15.0,
        sheet: true,
        condition: Condition::Intact,
        disabled: false,
        miss_class: 100,
        slot: Slot::Berth(2),
    }
}

/// Visits `shot` once among `targets` at full gain.
fn visit(shot: &mut Shot, targets: &[Target]) {
    visit_at(shot, targets, SpeedMult::FULL);
}

/// Visits `shot` once among `targets` at `gain`.
fn visit_at(shot: &mut Shot, targets: &[Target], gain: SpeedMult) {
    let guide = Guide {
        speed_mult: gain,
        ..Guide::among(targets)
    };
    shot.step(&guide, &mut NeverFires);
}

/// A homing shot of `Count` 100 at ship 2, straight down, 90 degrees off
/// its heading: the visits it first turns on, of the first `visits` at
/// the gains `gains` gives each.
fn turns_on(visits: u32, gains: impl Fn(u32) -> SpeedMult) -> Vec<u32> {
    let below = [target(0.0, 1000.0)];
    let mut homing = Shot {
        target: Some(TARGET),
        ..shot(spec(1, 100))
    };
    let mut turned = Vec::new();
    for n in 1..=visits {
        let heading = homing.heading;
        visit_at(&mut homing, &below, gains(n));
        if homing.heading != heading {
            turned.push(n);
        }
    }
    turned
}

#[test]
fn a_shot_of_count_3_counts_its_life_down_a_frame_a_visit_and_is_spent_on_its_third() {
    let mut flying = shot(spec(-1, 3));
    assert_eq!(flying.life, 3.0, "its Count, as _SpawnShot sets it");
    let mut lives = Vec::new();
    let mut spent = Vec::new();
    for _ in 0..3 {
        visit(&mut flying, &[]);
        lives.push(flying.life);
        spent.push(flying.expired());
    }
    assert_eq!(lives, [2.0, 1.0, SPENT]);
    assert_eq!(spent, [false, false, true]);
    assert_eq!(SPENT, -1.0);
}

#[test]
fn a_homing_shot_of_count_15_turns_on_its_15th_and_last_visit() {
    // The target straight down, 90 degrees off its heading. The visit
    // that spends it reads life -1, an age of 16 (@0x356c1).
    let below = [target(0.0, 1000.0)];
    let mut homing = Shot {
        target: Some(TARGET),
        ..shot(spec(1, 15))
    };
    for _ in 0..14 {
        visit(&mut homing, &below);
    }
    assert_eq!(homing.heading, 90.0, "straight for 14");
    visit(&mut homing, &below);
    assert!(homing.expired());
    assert!((homing.heading - 97.0).abs() < 1e-4, "{homing:?}");
}

#[test]
fn an_age_set_or_built_reads_back() {
    let mut flying = shot(spec(-1, 40));
    assert_eq!(flying.age(), 0.0);
    flying.set_age(7);
    assert_eq!((flying.age(), flying.life), (7.0, 33.0));
    let built = shot(spec(-1, 40)).aged(12);
    assert_eq!((built.age(), built.life), (12.0, 28.0));
}

#[test]
fn the_fuse_and_the_touch_read_the_life_against_count_less_prox_safety() {
    // Count 10, ProxSafety 4: the line is life 6.
    let weapon = WeaponSpec {
        prox_safety: 4,
        ..spec(-1, 10)
    };
    let at = |life: f32| Shot {
        life,
        ..shot(weapon)
    };
    assert!(!at(6.01).fuse_armed(), "just above the line");
    assert!(!at(6.0).fuse_armed(), "on it");
    assert!(at(5.99).fuse_armed(), "below it");
    assert!(!at(6.01).touches_ships(), "held just above the line");
    assert!(at(6.0).touches_ships(), "on it");
    assert!(at(5.99).touches_ships());
    let safe = WeaponSpec {
        prox_safety: 0,
        ..spec(-1, 10)
    };
    let unsafe_at = |life: f32| Shot { life, ..shot(safe) };
    assert!(
        unsafe_at(10.0).touches_ships(),
        "no ProxSafety holds nothing"
    );
    assert!(!unsafe_at(10.0).fuse_armed(), "nor arms at its full life");
    assert!(unsafe_at(9.99).fuse_armed());
}

#[test]
fn a_guide_among_ships_is_at_full_gain_by_the_original_decay() {
    let guide = Guide::among(&[]);
    assert_eq!(guide.speed_mult, SpeedMult::FULL);
    assert_eq!(guide.decay_step, DecayStep::EveryDecayPlusOne);
}

#[test]
fn at_half_gain_a_shots_life_drops_half_a_frame_a_visit_and_at_none_it_never_ages() {
    let mut flying = shot(spec(-1, 2));
    let mut lives = Vec::new();
    for _ in 0..4 {
        visit_at(&mut flying, &[], HALF);
        lives.push(flying.life);
    }
    assert_eq!(lives, [1.5, 1.0, 0.5, SPENT], "spent at 0");
    let expected = HALF.add_times(2.0, -1.0);
    let mut once = shot(spec(-1, 2));
    visit_at(&mut once, &[], HALF);
    assert_eq!(once.life, expected);
    let mut frozen = shot(spec(-1, 2));
    for _ in 0..10 {
        visit_at(&mut frozen, &[], SpeedMult::ZERO);
    }
    assert_eq!((frozen.life, frozen.expired()), (2.0, false));
}

#[test]
fn at_half_gain_a_shot_moves_half_its_velocity() {
    let mut flying = shot(spec(-1, 20));
    let (from, velocity) = (flying.position, flying.velocity);
    visit_at(&mut flying, &[], HALF);
    assert_eq!(flying.position, crate::flight::moved(from, velocity, HALF));
    assert_eq!(flying.moved_from, Some(from));
    let mut full = shot(spec(-1, 20));
    visit(&mut full, &[]);
    assert_eq!(
        full.position,
        from + velocity,
        "the whole of it at full gain"
    );
}

#[test]
fn at_half_gain_a_homing_turn_turns_half_its_turn_past_its_unscaled_gate() {
    // 90 degrees off, past the gate of 7: half of 7.
    assert_eq!(homing_turn(90.0, 180, 7.0, HALF), HALF.add_times(90.0, 7.0));
    assert_eq!(homing_turn(90.0, 0, 7.0, HALF), HALF.add_times(90.0, -7.0));
    // 5 off: more than half the turn but within its truncation, 7.
    assert_eq!(
        homing_turn(90.0, 95, 7.0, HALF),
        90.0,
        "the gate is unscaled"
    );
    assert_eq!(homing_turn(90.0, 180, 7.0, SpeedMult::FULL), 97.0);
    assert_eq!(homing_turn(90.0, 180, 7.0, SpeedMult::ZERO), 90.0);
}

#[test]
fn a_homing_shot_steers_through_step_by_half_its_turn_at_half_gain() {
    let below = [target(0.0, 1000.0)];
    let mut homing = Shot {
        target: Some(TARGET),
        ..shot(spec(1, 100))
    }
    .aged(20);
    visit_at(&mut homing, &below, HALF);
    assert_eq!(homing.heading, HALF.add_times(90.0, 7.0));
}

#[test]
fn the_homing_delay_is_15_frames_at_the_gain() {
    assert_eq!(
        turns_on(17, |_| HALF).first(),
        Some(&16),
        "age 8 > 7.5 on its 16th visit"
    );
    assert_eq!(turns_on(17, |_| SpeedMult::FULL).first(), Some(&16));
    assert_eq!(
        turns_on(16, |_| SpeedMult::SETTLED).first(),
        Some(&15),
        "15 > 15 x (1 - 2^-52) on its 15th"
    );
    // At none it steers at any age above 0; a new shot is of none.
    let below = [target(0.0, 1000.0)];
    let mut new = Shot {
        target: Some(TARGET),
        ..shot(spec(1, 100))
    };
    visit_at(&mut new, &below, SpeedMult::ZERO);
    assert_eq!(new.heading, 90.0, "age 0 is not above 0");
    let mut old = new.aged(1);
    visit_at(&mut old, &below, SpeedMult::ZERO);
    assert_eq!(old.heading, 90.0, "age 1 steers by none of its turn");
    assert_eq!(homing_turn(90.0, 180, 7.0, SpeedMult::ZERO), old.heading);
    // During a rising gain it stops once 15 x g passes its age: 16
    // visits at 0.5 reach age 8, then at full gain 9 is below 15.
    let rising = turns_on(18, |n| if n <= 16 { HALF } else { SpeedMult::FULL });
    assert_eq!(rising, [16], "steers on the 16th, not the 17th or 18th");
}

#[test]
fn at_half_gain_an_interfered_shot_circles_by_its_whole_turn_from_age_15() {
    let mut circling = Shot {
        target: Some(TARGET),
        jam: Jam::Interfered,
        ..shot(spec(1, 100))
    };
    let mut first = None;
    for n in 1..=31 {
        let heading = circling.heading;
        visit_at(&mut circling, &[target(0.0, 1000.0)], HALF);
        if circling.heading != heading && first.is_none() {
            first = Some((n, circling.heading - heading));
        }
    }
    assert_eq!(first, Some((31, -7.0)), "age 15.5 > 15, back by all of 7");
}

#[test]
fn passing_the_lock_age_is_30_frames_at_the_gain() {
    let passes = |gains: &dyn Fn(u32) -> SpeedMult, visits: u32| {
        let mut flying = shot(spec(1, 200));
        (1..=visits)
            .filter(|&n| {
                visit_at(&mut flying, &[], gains(n));
                flying.passes_lock_age(gains(n))
            })
            .collect::<Vec<u32>>()
    };
    assert_eq!(passes(&|_| HALF, 40), [31], "age 15.5 > 30 x 0.5");
    assert_eq!(passes(&|_| SpeedMult::FULL, 40), [31]);
    assert_eq!(passes(&|_| SpeedMult::SETTLED, 40), [30]);
    // 31 visits at 0.5 (age 15.5) pass; then at full gain the age
    // passes 30 again, from 29.5 to 30.5.
    let ramp = |n: u32| if n <= 31 { HALF } else { SpeedMult::FULL };
    assert_eq!(passes(&ramp, 50), [31, 46]);
}

#[test]
fn a_shot_spent_on_a_visit_reads_count_plus_1_for_the_lock() {
    let mut flying = shot(spec(1, 30));
    for _ in 0..29 {
        visit(&mut flying, &[]);
    }
    assert!(!flying.passes_lock_age(SpeedMult::FULL));
    visit(&mut flying, &[]);
    assert!(flying.expired());
    assert!(flying.passes_lock_age(SpeedMult::FULL), "age 31 > 30");
}

#[test]
fn the_interference_range_is_100_over_the_gain_floored_at_1() {
    assert_eq!(interference_range(SpeedMult::FULL), 100);
    assert_eq!(interference_range(SpeedMult::SETTLED), 100);
    assert_eq!(interference_range(SpeedMult::ZERO), 100, "100 at none");
    assert_eq!(interference_range(HALF), 200);
    assert_eq!(interference_range(SpeedMult::new(0.25)), 400);
    assert_eq!(interference_range(SpeedMult::new(1000.0)), 1, "floored");
    assert_eq!(INTERFERENCE_ROLL, 100);
}

#[test]
fn at_half_gain_the_launch_rolls_interference_over_the_range_at_the_gain() {
    let interfered = WeaponSpec {
        seeker: crate::combat::weapon::SENSOR_INTERFERENCE,
        ..spec(1, 100)
    };
    let mut rolled = shot(interfered);
    let mut chance = Draws::of(&[]);
    rolled.roll_at_launch(50, HALF, &mut chance);
    assert_eq!(chance.times_asked(interference_range(HALF)), 1);
    assert_eq!(chance.times_asked(INTERFERENCE_ROLL), 0);
}

#[test]
fn at_half_gain_a_spinning_shot_shows_its_next_frame_on_its_5th_visit() {
    let spinning = WeaponSpec {
        flags: super::super::weapon::SPINS,
        beam_width: 2,
        ..spec(-1, 100)
    };
    let mut flying = shot(spinning);
    let mut frames = Vec::new();
    for _ in 0..5 {
        visit_at(&mut flying, &[], HALF);
        frames.push(flying.spin_shown);
    }
    assert_eq!(
        frames,
        [0, 0, 0, 0, 1],
        "stepped on its 4th, shown on its 5th"
    );
    assert_eq!(flying.spin_timer, 0.5);
}

#[test]
fn the_still_test_reads_the_life_against_count_less_prox_safety() {
    use super::super::shot_frame::{FrameRule, STILL_UNTIL_SAFE, still};
    let weapon = WeaponSpec {
        flags: super::super::weapon::SPINS,
        flags2: STILL_UNTIL_SAFE,
        prox_safety: 4,
        ..spec(-1, 10)
    };
    let rule = FrameRule::of(&weapon);
    assert!(still(rule, 6.01, 10), "just above the line");
    assert!(!still(rule, 6.0, 10), "on it");
    assert!(!still(rule, 5.99, 10));
    // The visit reads its life after the count-down: at 0.5 the life is
    // 6.0 on its 8th visit, the age 4.
    let mut flying = shot(weapon);
    let held: Vec<bool> = (0..9)
        .map(|_| {
            visit_at(&mut flying, &[], HALF);
            flying.spin_still
        })
        .collect();
    assert_eq!(
        held,
        [true, true, true, true, true, true, true, false, false]
    );
}

#[test]
fn at_half_gain_a_fading_in_shot_reads_half_its_falloff_on_its_2nd_visit() {
    let fading = WeaponSpec {
        falloff: -6,
        ..spec(-1, 100)
    };
    let mut flying = shot(fading);
    visit_at(&mut flying, &[], HALF);
    assert_eq!(flying.fade_shown, 0.0, "the first reads its spawn's 0");
    visit_at(&mut flying, &[], HALF);
    assert_eq!(flying.fade_shown, 3.0);
    assert_eq!(flying.fade, 6.0);
}

#[test]
fn at_half_gain_a_decay_of_1_takes_its_first_step_on_its_3rd_visit_and_by_the_bible_its_2nd() {
    let decaying = WeaponSpec {
        decay: 1,
        ..spec(-1, 100)
    };
    let first = |reading: DecayStep| {
        let guide = Guide {
            speed_mult: HALF,
            decay_step: reading,
            ..Guide::among(&[])
        };
        let mut flying = shot(decaying);
        (1..=6).find(|_| {
            flying.step(&guide, &mut NeverFires);
            flying.decay_steps > 0
        })
    };
    assert_eq!(first(DecayStep::EveryDecayPlusOne), Some(3));
    assert_eq!(first(DecayStep::EveryDecay), Some(2));
}
