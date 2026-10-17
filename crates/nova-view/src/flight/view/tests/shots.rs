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

/// Each frame's draw of the blaster's shot over `ticks` ticks from the
/// tick it is fired, one shot alone: its tint and whether it is drawn by OR, or
/// none once it is gone.
fn one_shot_draws(view: &mut View, ticks: usize) -> Vec<Option<(Color, bool)>> {
    view.input(&key(FIRE_KEY, true));
    (0..ticks)
        .map(|tick| {
            view.tick(TICK);
            if tick == 0 {
                view.input(&key(FIRE_KEY, false));
            }
            let list = drawn(view);
            list.iter().find_map(|command| match *command {
                DrawCommand::Sprite {
                    image, tint, blend, ..
                } if image.id == 3500 => Some((tint, blend == crate::draw::Blend::Or)),
                _ => None,
            })
        })
        .collect()
}

#[test]
fn a_shot_is_drawn_count_times_on_its_live_ticks_fading_from_its_first() {
    // The blaster lives 30 ticks: `Count` 30, so its life is under 32 from
    // its first tick, `Count` − 1 = 29, and its fade-out runs from there.
    let catalog = with_blaster(|look| {
        look.count = 30;
        look.shot_falloff = 2;
        look.flags3 = weapons::TRANSLUCENT;
    });
    let mut view = fighting(catalog, &[]);
    let draws = one_shot_draws(&mut view, 32);
    let shown: Vec<(Color, bool)> = draws.iter().flatten().copied().collect();
    assert_eq!(shown.len(), 30, "drawn on its 30 live ticks: {draws:?}");
    assert_eq!(draws[30], None, "gone after its 30th");
    let levels: Vec<u8> = (0..30_u8)
        .map(|tick| 32_u8.saturating_sub(2 * tick).max(4))
        .collect();
    let expected: Vec<(Color, bool)> = levels
        .into_iter()
        .map(|level| (crate::draw::lights_tint(level), true))
        .collect();
    assert_eq!(shown, expected, "32 on its first tick, then 30, 28 … 4");
}

#[test]
fn a_translucent_shot_with_a_falloff_is_opaque_before_its_window_by_the_engine_only() {
    let catalog = || {
        with_blaster(|look| {
            look.count = 40;
            look.shot_falloff = 2;
            look.flags3 = weapons::TRANSLUCENT;
        })
    };
    let mut view = fighting(catalog(), &[]);
    let draws = one_shot_draws(&mut view, 9);
    assert_eq!(draws[0], Some((Color::WHITE, false)), "by the engine");
    assert_eq!(
        draws[8],
        Some((crate::draw::lights_tint(32), true)),
        "life 31"
    );
    let rules = Rulebook::default().with_override(RuleKey::TranslucentShotFade, RuleSource::Bible);
    let mut mended = fighting(catalog(), &[]).with_rules(rules);
    let draws = one_shot_draws(&mut mended, 1);
    assert_eq!(draws[0], Some((crate::draw::lights_tint(32), true)));
}

#[test]
fn a_shot_far_from_the_player_in_a_murky_system_is_darkened() {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.sol_murk = 100;
    let mut view = fighting(catalog, &[]);
    let draws = one_shot_draws(&mut view, 8);
    let session = view.session().expect("flying");
    assert_eq!(session.effective_murk(), 100);
    let shot = session.shots()[0];
    let level = crate::flight::murk::sprite_murk(shot.position, session.player().position, 100);
    assert!(level > 0, "{shot:?}");
    assert_eq!(
        draws[7],
        Some((crate::flight::murk::murk_tint(Color::WHITE, level), false))
    );
    assert_eq!(draws[0], Some((Color::WHITE, false)), "beside the player");
}

#[test]
fn a_flying_shot_leaves_a_trail_of_particles_at_its_tail() {
    let catalog = with_blaster(|look| {
        look.particles = 3;
        look.part_vel = 100;
        look.part_life = (20, 20);
        look.part_color = 0x0010_40FF;
    });
    let (effects, effects_chance) = scripted(&[]);
    let mut view = fighting(catalog, &[]).with_effects_chance(effects_chance);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    assert_eq!(view.effects().particles().len(), 3, "a burst of 3");
    let asked = effects.borrow().asked.clone();
    assert_eq!(
        &asked[..10],
        [81, 81, 81, 81, 81, 81, 81, 81, 8, 8],
        "the table, then the picks"
    );
    view.tick(TICK);
    assert_eq!(view.effects().particles().len(), 6, "a burst a tick");
    let session = view.session().expect("flying");
    let shot = session.shots()[0];
    let newest = view.effects().particles()[5];
    // The blaster's sheet is no pixels wide here: the trail leaves from
    // the shot's whole pixel, where it moved this tick, and flies from
    // the next.
    let start = crate::flight::trails::trail_start(shot.position, shot.heading, 0);
    assert_eq!(newest.at, Point::new(start.x, start.y));
    assert_eq!(newest.life, 20);
}

#[test]
fn a_smoking_shot_leaves_puffs_that_landing_clears() {
    let catalog = with_blaster(|look| {
        look.flags = crate::flight::smoke::SMALL_SMOKE;
        look.smoke_set = 1;
    });
    let mut view = fighting(catalog, &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    view.tick(TICK);
    let puffs = |view: &View| {
        sprites(&drawn(view))
            .into_iter()
            .filter(|(image, _)| {
                image.kind == crate::image::ImageKind::Icon && (1008..1016).contains(&image.id)
            })
            .count()
    };
    assert_eq!(puffs(&view), 2, "a puff a tick, from set 1's cicns");
}

#[test]
fn landing_and_arriving_free_the_smoke() {
    let smoking = |mut catalog: FakeCatalog| {
        let (_, look) = catalog
            .looks
            .iter_mut()
            .find(|(id, _)| *id == BLASTER.0)
            .expect("the blaster's look");
        let look = look.as_mut().expect("read");
        look.flags = crate::flight::smoke::SMALL_SMOKE | crate::flight::smoke::LINGERING_SMOKE;
        catalog
    };
    let fire = |view: &mut View| {
        view.input(&key(FIRE_KEY, true));
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
        assert!(!view.shot_looks.smoke().puffs().is_empty(), "smoking");
    };
    let mut catalog = smoking(armed(0, &[BLASTER]));
    catalog.sites = vec![site(128, (0.0, 0.0), StellarFlags::CAN_LAND)];
    let mut view = fighting(catalog, &[]);
    fire(&mut view);
    land_now(&mut view);
    assert!(view.take_landing().is_some());
    assert_eq!(view.shot_looks.smoke().puffs(), [], "landed");

    let mut view = fighting(smoking(armed(0, &[BLASTER])), &[]);
    plot(&mut view, 131);
    fly_out(&mut view);
    fire(&mut view);
    jump_now(&mut view);
    arrive_now(&mut view);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(view.shot_looks.smoke().puffs(), [], "arrived");
}
