//! `Combat`'s weapon-hit knockback and tractor self-pull read the fight's
//! speed cap (`set_speed_cap`).

use super::*;

#[test]
fn under_the_circle_a_hits_knockback_is_clamped_to_its_top_speeds_circle() {
    let mut combat = Combat::default();
    combat.set_speed_cap(RuleSource::Bible);
    let mut target = of_mass(Ship::at(2, 100.0, 10.0), 10.0);
    target.top_speed = 5.5;
    target.state.velocity = Vec2::new(0.0, 5.0);
    let mut ships = [Ship::at(1, 0.0, 0.0).armed(pushing(50)), target];
    let (shoves, at) = hit_on_move(&mut combat, &mut ships, 6);
    let at = at.expect("it hit");
    let pushed = Vec2::new(0.0, 5.0) + push_of(at, Vec2::new(100.0, 10.0), 50.0, 10.0);
    let expected = SpeedCap::Circle.clamp(pushed, 5.5);
    let boxed = SpeedCap::PerAxis.clamp(pushed, 5.5);
    assert!(!close_by(expected, boxed), "{expected:?}, {boxed:?}");
    let on_b = shoves_on(&shoves, B);
    assert_eq!(on_b.len(), 1, "{shoves:?}");
    assert!(close_by(on_b[0], expected), "{on_b:?}, {expected:?}");
    assert!(!close_by(on_b[0], boxed), "{on_b:?}");
}

#[test]
fn under_the_circle_a_light_firers_tractor_pull_is_clamped_to_its_top_speeds_circle() {
    let mut combat = Combat::default();
    combat.set_speed_cap(RuleSource::Bible);
    let mut firer = tractor_firer(1.0, 135.0);
    firer.top_speed = 0.5;
    let toward = Vec2::new(100.0, 100.0);
    let mut ships = [firer, of_mass(Ship::at(2, toward.x, toward.y), 15.0)];
    let pulled = shoves_on(&beam_shoves(&mut combat, &mut ships), A);
    let raw = push_of(Vec2::ZERO, toward, 5.0, 1.0);
    let circle = SpeedCap::Circle.clamp(raw, 0.5);
    let per_axis = SpeedCap::PerAxis.clamp(raw, 0.5);
    assert!(!close_by(circle, per_axis), "{circle:?}, {per_axis:?}");
    assert_eq!(pulled.len(), 1, "{pulled:?}");
    assert!(close_by(pulled[0], circle), "{pulled:?}, {circle:?}");
    assert!(!close_by(pulled[0], per_axis), "{pulled:?}");
}
