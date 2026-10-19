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

#[test]
fn a_spinning_shot_in_a_reused_slot_steps_on_the_timer_its_slot_left() {
    // The blaster spins from its first frame, a step every 4 passes, and
    // lives 3 ticks; the simulation runs its slot's timer, and the next
    // shot in the slot starts on what it left (`_SpawnShot` never writes
    // +0x1c, @0x3e550-0x3f143).
    let mut catalog = with_blaster(|look| {
        look.flags = SPINS | crate::flight::spin::FIRST_FRAME_START;
        look.beam_width = 4;
    });
    catalog.weapons[0].flags |= SPINS;
    catalog.weapons[0].beam_width = 4;
    catalog.weapons[0].count = 3;
    catalog.weapons[0].reload = 1000;
    let mut view = fighting(catalog, &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    assert_eq!(view.session().expect("flying").shots()[0].spin_timer, 0);
    while !view.session().expect("flying").shots().is_empty() {
        view.tick(TICK);
    }
    ticks(&mut view, 1100);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    let shot = view.session().expect("flying").shots()[0];
    assert_eq!(shot.slot, 0, "the slot reused");
    let timer = shot.spin_timer;
    assert_ne!(timer, 0, "the slot's leftover");
    let look = WeaponLook {
        flags: SPINS | crate::flight::spin::FIRST_FRAME_START,
        beam_width: 4,
        ..WeaponLook::default()
    };
    let frames = NonZeroU16::new(36).expect("non-zero");
    let (mut shown, mut expected, mut from_0) = (Vec::new(), Vec::new(), Vec::new());
    while let Some(shot) = view.session().expect("flying").shots().first() {
        let age = shot.age;
        shown.extend(blaster_frames(&view));
        expected.push(weapons::shot_frame(90.0, age, frames, &look, 0, timer));
        from_0.push(weapons::shot_frame(90.0, age, frames, &look, 0, 0));
        view.tick(TICK);
    }
    assert_eq!(shown, expected, "from timer {timer}");
    assert_ne!(shown, from_0, "not as from 0");
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

/// The puffs drawn from smoke set 1's `cicn`s.
fn set_1_puffs(view: &View) -> usize {
    sprites(&drawn(view))
        .into_iter()
        .filter(|(image, _)| {
            image.kind == crate::image::ImageKind::Icon && (1008..1016).contains(&image.id)
        })
        .count()
}

/// The blaster smoking from set 1, with a particle trail, its shot living
/// long enough to cross several smoke frames.
fn smoking_trailing_blaster() -> FakeCatalog {
    with_blaster(|look| {
        look.flags = crate::flight::smoke::SMALL_SMOKE;
        look.smoke_set = 1;
        look.count = 40;
        look.particles = 3;
        look.part_vel = 100;
        look.part_life = (20, 20);
        look.part_color = 0x0010_40FF;
    })
}

#[test]
fn a_smoking_shot_leaves_a_puff_only_on_a_smoke_frame_and_trails_every_tick() {
    let mut view = fighting(smoking_trailing_blaster(), &[]);
    assert!(view.smoke_trails(), "on by default");
    view.input(&key(FIRE_KEY, true));
    let mut seen = Vec::new();
    for tick in 0..6 {
        let puffs = view.shot_looks.smoke().puffs().len();
        let particles = view.effects().particles().len();
        view.tick(TICK);
        if tick == 0 {
            view.input(&key(FIRE_KEY, false));
        }
        let session = view.session().expect("flying");
        assert_eq!(session.shots().len(), 1, "one shot alone");
        let smoke_frame = session.smoke_frame();
        let puffed = view.shot_looks.smoke().puffs().len() - puffs;
        let burst = view.effects().particles().len() - particles;
        assert_eq!(burst, 3, "a burst every live tick: tick {tick}");
        seen.push((smoke_frame, puffed));
    }
    let frames = seen.iter().filter(|(smoke_frame, _)| *smoke_frame).count();
    assert_eq!(frames, 2, "a smoke frame in every 3: {seen:?}");
    assert!(
        seen.iter()
            .all(|&(smoke_frame, puffed)| puffed == usize::from(smoke_frame)),
        "a puff on each smoke frame and on no other: {seen:?}"
    );
    assert_eq!(set_1_puffs(&view), 2, "drawn from set 1's cicns");
}

#[test]
fn with_smoke_trails_off_a_shot_sprays_no_trail_and_leaves_no_smoke() {
    let mut view = fighting(smoking_trailing_blaster(), &[]).with_smoke_trails(false);
    assert!(!view.smoke_trails());
    view.input(&key(FIRE_KEY, true));
    for tick in 0..4 {
        view.tick(TICK);
        if tick == 0 {
            view.input(&key(FIRE_KEY, false));
        }
        assert_eq!(view.session().expect("flying").shots().len(), 1);
        assert_eq!(view.effects().particles().len(), 0, "tick {tick}");
        assert_eq!(view.shot_looks.smoke().puffs(), [], "tick {tick}");
    }
}

#[test]
fn turning_smoke_trails_off_stops_new_smoke_and_the_puffs_left_age_out() {
    let mut view = fighting(smoking_trailing_blaster(), &[]);
    view.input(&key(FIRE_KEY, true));
    while view.shot_looks.smoke().puffs().is_empty() {
        view.tick(TICK);
        view.input(&key(FIRE_KEY, false));
    }
    let left = view.shot_looks.smoke().puffs().to_vec();
    view.set_smoke_trails(false);
    let particles = view.effects().particles().len();
    for _ in 0..3 {
        view.tick(TICK);
    }
    let puffs = view.shot_looks.smoke().puffs();
    assert_eq!(puffs.len(), left.len(), "no new puff");
    assert!(
        puffs
            .iter()
            .zip(&left)
            .all(|(now, then)| now.age == then.age + 0.75),
        "still ageing: {puffs:?}"
    );
    assert!(
        view.effects().particles().len() <= particles,
        "no new burst"
    );
    ticks(&mut view, 40);
    assert_eq!(view.shot_looks.smoke().puffs(), [], "freed");
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
        for _ in 0..3 {
            view.tick(TICK);
            view.input(&key(FIRE_KEY, false));
        }
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

/// The weapon glow layers drawn: each one's frame, tint and whether it
/// is drawn by OR.
fn glow_layers(view: &View, image: i16) -> Vec<(u16, Color, bool)> {
    drawn(view)
        .iter()
        .filter_map(|command| match *command {
            DrawCommand::Sprite {
                image: key,
                tint,
                blend,
                ..
            } if key.id == image => Some((key.frame, tint, blend == crate::draw::Blend::Or)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_firing_ships_weapon_glow_is_added_at_32_then_falls_by_its_decay() {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.weapons[0].flags2 = nova_sim::combat::weapon::GLOWS;
    catalog.sheet = Ok(ShipSheet {
        weapon: Some(layer(2300, 36)),
        weap_decay: 6,
        ..sheet()
    });
    let mut view = fighting(catalog, &[]);
    assert_eq!(glow_layers(&view, 2300), [], "unlit");
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    let frame = view.frame().expect("a sheet") % 36;
    assert_eq!(
        glow_layers(&view, 2300),
        [(frame, crate::draw::lights_tint(32), true)],
        "the frame it fired"
    );
    view.tick(TICK);
    assert_eq!(
        glow_layers(&view, 2300),
        [(frame, crate::draw::lights_tint(30), true)],
        "32 less 6 × 0.333"
    );
    ticks(&mut view, 20);
    assert_eq!(glow_layers(&view, 2300), [], "out");
}

#[test]
fn with_weapon_effects_off_no_ship_draws_its_weapon_glow() {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.weapons[0].flags2 = nova_sim::combat::weapon::GLOWS;
    catalog.sheet = Ok(ShipSheet {
        weapon: Some(layer(2300, 36)),
        weap_decay: 6,
        ..sheet()
    });
    assert!(
        fighting(armed(0, &[BLASTER]), &[]).weapon_effects(),
        "on by default"
    );
    let mut view = fighting(catalog, &[]).with_weapon_effects(false);
    assert!(!view.weapon_effects());
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    assert_eq!(view.glows.drawn(ShipRef::Player), Some(32), "still lit");
    assert_eq!(glow_layers(&view, 2300), [], "but not drawn");
    view.set_weapon_effects(true);
    assert_eq!(glow_layers(&view, 2300).len(), 1, "drawn once on");

    let catalog = trafficked(&[130], 1, 129, 3);
    let (_, chance) = scripted(&placed(850, 650, 90));
    let mut view = FlightView::new(catalog)
        .with_chance(chance)
        .with_behaviour(Rc::new(Still))
        .with_weapon_effects(false);
    view.tick(TICK);
    let npc = view.session().expect("flying").npcs()[0].clone();
    view.glows
        .step(&[(ShipRef::Npc(npc.id), false)], |_| Some(1.0));
    assert_eq!(glow_layers(&view, 2301), [], "an NPC's neither");
    view.set_weapon_effects(true);
    assert_eq!(glow_layers(&view, 2301).len(), 1);
}

#[test]
fn an_npcs_weapon_glow_is_held_below_its_murk_and_murked() {
    for (murk, sprite_murk) in [(0, 0), (30, 7), (100, 24)] {
        let mut catalog = trafficked(&[130], 1, 129, 3);
        catalog.sol_murk = murk;
        let (_, chance) = scripted(&placed(850, 650, 90));
        let mut view = FlightView::new(catalog)
            .with_chance(chance)
            .with_behaviour(Rc::new(Still));
        view.tick(TICK);
        let npc = view.session().expect("flying").npcs()[0].clone();
        let viewer = view.session().expect("flying").player().position;
        let level = crate::flight::murk::sprite_murk(
            npc.state.position,
            viewer,
            view.session().expect("flying").effective_murk(),
        );
        assert_eq!(level, sprite_murk, "murk {murk}");
        view.glows
            .step(&[(ShipRef::Npc(npc.id), false)], |_| Some(1.0));
        let capped = crate::flight::weapon_glow::npc_glow_level(32, level);
        let tint = crate::flight::murk::murk_tint(crate::draw::lights_tint(capped), level);
        let layers = glow_layers(&view, 2301);
        assert_eq!(layers.len(), 1, "murk {murk}");
        assert_eq!((layers[0].1, layers[0].2), (tint, true), "murk {murk}");
        let frame = rotation_frame(npc.state.heading, NonZeroU16::new(72).expect("non-zero"));
        assert_eq!(layers[0].0, frame % 72, "the hull's frame");
        if murk == 0 {
            view.tick(TICK);
            assert_eq!(
                glow_layers(&view, 2301).first().map(|layer| layer.1),
                Some(crate::draw::lights_tint(31)),
                "falling by its class's WeapDecay 3 × 0.333 a step"
            );
        }
    }
}

#[test]
fn a_beam_that_is_not_lightning_draws_no_lightning_rolls() {
    let (effects, effects_chance) = scripted(&[]);
    let mut view = fighting(armed(0, &[UNDER]), &[]).with_effects_chance(effects_chance);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert!(
        !view.session().expect("flying").beams().is_empty(),
        "a beam"
    );
    assert_eq!(effects.borrow().asked, Vec::<u32>::new());
}

/// The lines drawn in the lightning beam's colour.
fn lightning_lines(view: &View) -> Vec<(Point, Point)> {
    drawn(view)
        .iter()
        .filter_map(|command| match *command {
            DrawCommand::Line {
                from, to, color, ..
            } if (color.r, color.g, color.b) == (0xAB, 0xCD, 0xEF) => Some((from, to)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_lightning_beam_zig_zags_the_same_through_a_step_and_anew_the_next() {
    let mut catalog = armed(0, &[UNDER]);
    catalog.weapons[4].beam_length = 400;
    catalog.looks[4].1 = Ok(WeaponLook {
        flags2: weapons::BEAM_UNDER_SHIPS,
        beam_width: 1,
        beam_color: 0x00AB_CDEF,
        li_density: 3,
        li_amplitude: 5,
        ..WeaponLook::default()
    });
    let (_, effects_chance) = scripted(&[1, 2, 3, 4, 5, 6]);
    let mut view = fighting(catalog, &[]).with_effects_chance(effects_chance);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    view.input(&key(FIRE_KEY, false));
    let first = lightning_lines(&view);
    assert_eq!(first.len(), 11, "12 steps of 400 pixels at 3: 11 segments");
    assert_eq!(lightning_lines(&view), first, "the same step, drawn again");
    view.tick(TICK);
    let second = lightning_lines(&view);
    assert_eq!(second.len(), 11);
    assert_ne!(second, first, "the next step draws anew");
}

#[test]
fn by_the_engine_a_short_lightning_beam_draws_nothing_and_mended_one_line() {
    let catalog = || {
        let mut catalog = armed(0, &[UNDER]);
        catalog.weapons[4].beam_length = 30;
        catalog.looks[4].1 = Ok(WeaponLook {
            beam_width: 1,
            beam_color: 0x00AB_CDEF,
            li_density: 2,
            li_amplitude: 4,
            ..WeaponLook::default()
        });
        catalog
    };
    let mut view = fighting(catalog(), &[]);
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert_eq!(lightning_lines(&view), [], "under 50 pixels at 2");
    let rules = Rulebook::default().with_override(RuleKey::LightningZigZag, RuleSource::Bible);
    let mut mended = fighting(catalog(), &[]).with_rules(rules);
    mended.input(&key(FIRE_KEY, true));
    mended.tick(TICK);
    assert_eq!(lightning_lines(&mended).len(), 1, "one straight line");
}
