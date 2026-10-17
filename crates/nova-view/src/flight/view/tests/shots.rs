//! How the flight draws its shots and beams (rdm weapons-fidelity phase
//! 24): the spin start, the fade, the murk, the trails, the weapon glow
//! and the lightning.

use super::*;
use crate::flight::weapons::SPINS;

/// The blaster's look set by `edit`, in [`armed`]'s catalog.
fn with_blaster(edit: impl FnOnce(&mut WeaponLook)) -> FakeCatalog {
    let mut catalog = armed(0, &[BLASTER]);
    let (_, look) = catalog
        .looks
        .iter_mut()
        .find(|(id, _)| *id == BLASTER.0)
        .expect("the blaster's look");
    edit(look.as_mut().expect("read"));
    catalog
}

/// The frames of the blaster's sheet drawn in `view`.
fn blaster_frames(view: &View) -> Vec<u16> {
    sprites(&drawn(view))
        .into_iter()
        .filter(|(image, _)| image.id == 3500)
        .map(|(image, _)| image.frame)
        .collect()
}

#[test]
fn a_spinning_shot_starts_on_the_index_drawn_on_the_effects_chance() {
    let catalog = with_blaster(|look| {
        look.flags = SPINS;
        look.beam_width = 1;
    });
    let (effects, effects_chance) = scripted(&[20]);
    let mut view = fighting(catalog, &[]).with_effects_chance(effects_chance);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    assert_eq!(effects.borrow().asked, [36], "Rand(36), once");
    assert_eq!(
        blaster_frames(&view),
        [20],
        "its first pass shows its start"
    );
    view.tick(TICK);
    assert_eq!(blaster_frames(&view), [21], "then a frame a tick");
    assert_eq!(effects.borrow().asked, [36], "drawn once a shot");
}

#[test]
fn a_spinning_shot_of_flags_0x0004_starts_on_its_first_frame() {
    let catalog = with_blaster(|look| {
        look.flags = SPINS | crate::flight::spin::FIRST_FRAME_START;
        look.beam_width = 1;
    });
    let (effects, effects_chance) = scripted(&[20]);
    let mut view = fighting(catalog, &[]).with_effects_chance(effects_chance);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert_eq!(blaster_frames(&view), [0]);
    assert_eq!(effects.borrow().asked, Vec::<u32>::new());
}
