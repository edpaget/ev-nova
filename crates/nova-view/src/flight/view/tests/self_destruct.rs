//! The self-destruct chord, Command and `-` held together.

use super::*;

/// A flight with `keys` held down over a tick: the sim sounds of it.
fn held_over_a_tick(keys: &[Key]) -> Vec<SimSound> {
    let mut view = flight();
    for &k in keys {
        view.input(&key(k, true));
    }
    view.take_sounds();
    ticks(&mut view, 1);
    sims(&mut view)
}

#[test]
fn command_and_minus_held_over_the_steps_start_the_countdown() {
    assert_eq!(SELF_DESTRUCT_KEY, Key::Char('-'));
    let sounds = held_over_a_tick(&[Key::Command, SELF_DESTRUCT_KEY]);
    assert!(
        sounds.contains(&SimSound::SelfDestructStarted),
        "{sounds:?}"
    );
}

#[test]
fn minus_alone_or_command_alone_does_not() {
    for keys in [[SELF_DESTRUCT_KEY], [Key::Command]] {
        let sounds = held_over_a_tick(&keys);
        assert!(
            !sounds.contains(&SimSound::SelfDestructStarted),
            "{keys:?}: {sounds:?}"
        );
    }
}

#[test]
fn letting_go_of_the_chord_cancels_the_countdown() {
    let mut view = flight();
    view.input(&key(Key::Command, true));
    view.input(&key(SELF_DESTRUCT_KEY, true));
    ticks(&mut view, 10);
    view.input(&key(SELF_DESTRUCT_KEY, false));
    view.take_sounds();
    ticks(&mut view, 1);
    let sounds = sims(&mut view);
    assert!(
        sounds.contains(&SimSound::SelfDestructCancelled),
        "{sounds:?}"
    );
}
