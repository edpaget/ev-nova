//! How the flight draws its shots and beams (rdm weapons-fidelity phase
//! 24): the spin start, the fade, the murk, the trails, the weapon glow
//! and the lightning. On [`FakeFlight`]: each frame's shots and beams are
//! scripted, and each pass the view steps its shot looks after is
//! reported; what the session does to a shot (its age, its slot's spin
//! timer, its life) is nova-sim's, named where a test leans on it.

use nova_sim::flight_frame::{FrameShip, FrameShot, PassReport, StepReport, StepRun};

use super::faked_world::{beam, in_sol_with, npc_129, shot_of, the_player_at, world, world_by};
use super::*;
use crate::flight::weapons::SPINS;
use crate::system::camera::Anchor;

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

/// A scripted fight from `catalog` by `rules`: the player alone at the
/// centre, facing up.
fn shooting_by(catalog: FakeCatalog, rules: &Rulebook) -> Faked {
    world_by(
        catalog,
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
        rules,
    )
}

/// [`shooting_by`] Nova's rules.
fn shooting(catalog: FakeCatalog) -> Faked {
    shooting_by(catalog, &Rulebook::default())
}

/// The player's blaster shot as the session flies it on the frame `tick`
/// ticks after the one it is fired on: 20 pixels a tick up from the
/// player, aged one more pass than the ticks it has flown.
fn blaster_shot(tick: u32) -> FrameShot {
    let flown = 20.0 * f32::from(u16::try_from(tick + 1).expect("a short life"));
    FrameShot {
        velocity: Vec2::new(0.0, -20.0),
        age: tick + 1,
        ..shot_of(0, BLASTER, Vec2::new(0.0, -flown), 0.0)
    }
}

/// Runs a frame of `view` whose one step's pass holds `shots`, on a
/// smoke frame or not, raising `combat`, and leaves its frame showing
/// them.
fn pass_with(view: &mut Faked, shots: Vec<FrameShot>, smoke_frame: bool, combat: Vec<CombatEvent>) {
    let frame = FlightFrame {
        shots: shots.clone(),
        ..view.frame.clone()
    };
    let pass = PassReport {
        ships: frame.ships.clone(),
        shots,
        smoke_frame,
        combat,
        player_position: frame.player().position,
        ..PassReport::default()
    };
    view.sim.set_frame(frame);
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, Anchor::in_flight(RuleSource::Engine));
}

/// The frames of the blaster's sheet drawn in `view`.
fn blaster_frames(view: &Faked) -> Vec<u16> {
    sprites(&super::faked_world::world_list(view))
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
    let mut view = shooting(catalog);
    view.effects_chance = effects_chance;
    pass_with(&mut view, vec![blaster_shot(0)], false, Vec::new());
    assert_eq!(effects.borrow().asked, [36], "Rand(36), once");
    assert_eq!(
        blaster_frames(&view),
        [20],
        "its first pass shows its start"
    );
    pass_with(&mut view, vec![blaster_shot(1)], false, Vec::new());
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
    let mut view = shooting(catalog);
    view.effects_chance = effects_chance;
    pass_with(&mut view, vec![blaster_shot(0)], false, Vec::new());
    assert_eq!(blaster_frames(&view), [0]);
    assert_eq!(effects.borrow().asked, Vec::<u32>::new());
}

#[test]
fn a_spinning_shot_is_drawn_stepping_on_the_spin_timer_its_frame_gives() {
    // The blaster spins from its first frame, a step every 4 passes. The
    // timer a shot's slot left it is the session's
    // (nova-sim-combat's `a_spinning_shot_runs_its_slots_spin_timer_and_the_next_shot_there_starts_on_it`):
    // the view steps it from the timer the frame gives.
    let look = WeaponLook {
        flags: SPINS | crate::flight::spin::FIRST_FRAME_START,
        beam_width: 4,
        ..WeaponLook::default()
    };
    let catalog = with_blaster(|blaster| {
        blaster.flags = look.flags;
        blaster.beam_width = look.beam_width;
    });
    let mut view = shooting(catalog);
    let frames = NonZeroU16::new(36).expect("non-zero");
    let timer = 3;
    let (mut shown, mut expected, mut from_0) = (Vec::new(), Vec::new(), Vec::new());
    for tick in 0..6 {
        let shot = FrameShot {
            spin_timer: timer,
            heading: 90.0,
            ..blaster_shot(tick)
        };
        pass_with(&mut view, vec![shot], false, Vec::new());
        shown.extend(blaster_frames(&view));
        expected.push(weapons::shot_frame(90.0, shot.age, frames, &look, 0, timer));
        from_0.push(weapons::shot_frame(90.0, shot.age, frames, &look, 0, 0));
    }
    assert_eq!(shown, expected, "from timer {timer}");
    assert_ne!(shown, from_0, "not as from 0");
}

/// Each frame's draw of the blaster's shot over `ticks` ticks from the
/// tick it is fired, one shot alone living `life` of them (the session's:
/// nova-sim-combat's
/// `an_unguided_shot_flies_straight_for_its_count_then_expires`): its tint
/// and whether it is drawn by OR, or none once it is gone.
fn one_shot_draws(view: &mut Faked, ticks: u32, life: u32) -> Vec<Option<(Color, bool)>> {
    (0..ticks)
        .map(|tick| {
            let shots = if tick < life {
                vec![blaster_shot(tick)]
            } else {
                Vec::new()
            };
            pass_with(view, shots, false, Vec::new());
            let list = super::faked_world::world_list(view);
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
    let mut view = shooting(catalog);
    let draws = one_shot_draws(&mut view, 32, 30);
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
    let mut view = shooting(catalog());
    let draws = one_shot_draws(&mut view, 9, 40);
    assert_eq!(draws[0], Some((Color::WHITE, false)), "by the engine");
    assert_eq!(
        draws[8],
        Some((crate::draw::lights_tint(32), true)),
        "life 31"
    );
    let rules = Rulebook::default().with_override(RuleKey::TranslucentShotFade, RuleSource::Bible);
    let mut mended = shooting_by(catalog(), &rules);
    let draws = one_shot_draws(&mut mended, 1, 40);
    assert_eq!(draws[0], Some((crate::draw::lights_tint(32), true)));
}

#[test]
fn a_shot_far_from_the_player_in_a_murky_system_is_darkened() {
    let mut view = shooting(armed(0, &[BLASTER]));
    view.sim.frame_mut().effective_murk = 100;
    view.refresh_frame();
    let draws = one_shot_draws(&mut view, 8, 30);
    let shot = blaster_shot(7);
    let level = crate::flight::murk::sprite_murk(shot.position, Vec2::ZERO, 100);
    assert!(level > 0, "{shot:?}");
    assert_eq!(
        draws[7],
        Some((crate::flight::murk::murk_tint(Color::WHITE, level), false))
    );
    assert_eq!(draws[0], Some((Color::WHITE, false)), "beside the player");
}

/// The puffs drawn from smoke set 1's `cicn`s.
fn set_1_puffs(view: &Faked) -> usize {
    sprites(&super::faked_world::world_list(view))
        .into_iter()
        .filter(|(image, _)| {
            image.kind == crate::image::ImageKind::Icon && (1008..1016).contains(&image.id)
        })
        .count()
}

/// The blaster smoking from set 1, with a particle trail, its shot living
/// long enough to cross several smoke frames.
pub(super) fn smoking_trailing_blaster() -> FakeCatalog {
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

/// Whether the pass `tick` ticks after the shot's firing is a smoke
/// frame, as the session's first passes are: one in every 3, from the
/// third (nova-sim's `a_smoke_frame_is_a_pass_on_a_multiple_of_three`).
fn smoke_frame(tick: u32) -> bool {
    tick % 3 == 2
}

#[test]
fn a_smoking_shot_leaves_a_puff_only_on_a_smoke_frame_and_trails_every_tick() {
    let mut view = shooting(smoking_trailing_blaster());
    assert!(view.smoke_trails, "on by default");
    let mut seen = Vec::new();
    for tick in 0..6 {
        let puffs = view.shot_looks.smoke().puffs().len();
        let particles = view.effects().particles().len();
        pass_with(
            &mut view,
            vec![blaster_shot(tick)],
            smoke_frame(tick),
            Vec::new(),
        );
        let puffed = view.shot_looks.smoke().puffs().len() - puffs;
        let burst = view.effects().particles().len() - particles;
        assert_eq!(burst, 3, "a burst every live tick: tick {tick}");
        seen.push((smoke_frame(tick), puffed));
    }
    assert!(
        seen.iter()
            .all(|&(smoke_frame, puffed)| puffed == usize::from(smoke_frame)),
        "a puff on each smoke frame and on no other: {seen:?}"
    );
    assert_eq!(set_1_puffs(&view), 2, "drawn from set 1's cicns");
}

#[test]
fn turning_smoke_trails_off_stops_new_smoke_and_the_puffs_left_age_out() {
    let mut view = shooting(smoking_trailing_blaster());
    let mut tick = 0;
    while view.shot_looks.smoke().puffs().is_empty() {
        pass_with(
            &mut view,
            vec![blaster_shot(tick)],
            smoke_frame(tick),
            Vec::new(),
        );
        tick += 1;
    }
    let left = view.shot_looks.smoke().puffs();
    view.smoke_trails = false;
    let particles = view.effects().particles().len();
    for _ in 0..3 {
        pass_with(
            &mut view,
            vec![blaster_shot(tick)],
            smoke_frame(tick),
            Vec::new(),
        );
        tick += 1;
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
    for _ in 0..40 {
        pass_with(&mut view, Vec::new(), false, Vec::new());
    }
    assert_eq!(view.shot_looks.smoke().puffs(), [], "freed");
}

/// The weapon glow layers drawn: each one's frame, tint and whether it
/// is drawn by OR.
fn glow_layers(view: &Faked, image: i16) -> Vec<(u16, Color, bool)> {
    super::faked_world::world_list(view)
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

/// The pass's report of `ship` lighting its weapon glow, as the session
/// raises it for a weapon of `Flags2` `GLOWS` fired.
fn lit(ship: ShipRef) -> Vec<CombatEvent> {
    vec![CombatEvent::WeaponGlow { ship, late: false }]
}

/// [`armed`] with the player's sheet glowing on layer 2300, decaying by 6.
fn glowing() -> FakeCatalog {
    let mut catalog = armed(0, &[BLASTER]);
    catalog.weapons[0].flags2 = nova_sim::combat::weapon::GLOWS;
    catalog.sheet = Ok(ShipSheet {
        weapon: Some(layer(2300, 36)),
        weap_decay: 6,
        ..sheet()
    });
    catalog
}

#[test]
fn a_firing_ships_weapon_glow_is_added_at_32_then_falls_by_its_decay() {
    let mut view = shooting(glowing());
    assert_eq!(glow_layers(&view, 2300), [], "unlit");
    pass_with(&mut view, Vec::new(), false, lit(ShipRef::Player));
    let frame = view.frame().expect("a sheet") % 36;
    assert_eq!(
        glow_layers(&view, 2300),
        [(frame, crate::draw::lights_tint(32), true)],
        "the frame it fired"
    );
    pass_with(&mut view, Vec::new(), false, Vec::new());
    assert_eq!(
        glow_layers(&view, 2300),
        [(frame, crate::draw::lights_tint(30), true)],
        "32 less 6 × 0.333"
    );
    for _ in 0..20 {
        pass_with(&mut view, Vec::new(), false, Vec::new());
    }
    assert_eq!(glow_layers(&view, 2300), [], "out");
}

/// A scripted flight of [`trafficked`]'s, its NPC of ship 129 at
/// (100, -100) facing 90, with the player at the centre.
fn beside_an_npc(murk: u8) -> Faked {
    let ships: Vec<FrameShip> = vec![
        the_player_at(Vec2::ZERO, 0.0),
        npc_129(0, Vec2::new(100.0, -100.0), 90.0),
    ];
    let mut frame = in_sol_with(ships);
    frame.effective_murk = murk;
    world(trafficked(&[130], 1, 129, 3), frame)
}

#[test]
fn with_weapon_effects_off_no_ship_draws_its_weapon_glow() {
    assert!(
        shooting(armed(0, &[BLASTER])).weapon_effects,
        "on by default"
    );
    let mut view = shooting(glowing());
    view.weapon_effects = false;
    pass_with(&mut view, Vec::new(), false, lit(ShipRef::Player));
    assert_eq!(view.glows.drawn(ShipRef::Player), Some(32), "still lit");
    assert_eq!(glow_layers(&view, 2300), [], "but not drawn");
    view.weapon_effects = true;
    assert_eq!(glow_layers(&view, 2300).len(), 1, "drawn once on");

    let mut view = beside_an_npc(0);
    view.weapon_effects = false;
    let npc = ShipRef::Npc(NpcId(0));
    view.glows.step(&[(npc, false)], |_| Some(1.0));
    assert_eq!(glow_layers(&view, 2301), [], "an NPC's neither");
    view.weapon_effects = true;
    assert_eq!(glow_layers(&view, 2301).len(), 1);
}

#[test]
fn an_npcs_weapon_glow_is_held_below_its_murk_and_murked() {
    for (murk, sprite_murk) in [(0, 0), (30, 7), (100, 24)] {
        let mut view = beside_an_npc(murk);
        let npc = view.frame.ships[1];
        let level = crate::flight::murk::sprite_murk(npc.position, Vec2::ZERO, murk);
        assert_eq!(level, sprite_murk, "murk {murk}");
        view.glows.step(&[(npc.ship, false)], |_| Some(1.0));
        let capped = crate::flight::weapon_glow::npc_glow_level(32, level);
        let tint = crate::flight::murk::murk_tint(crate::draw::lights_tint(capped), level);
        let layers = glow_layers(&view, 2301);
        assert_eq!(layers.len(), 1, "murk {murk}");
        assert_eq!((layers[0].1, layers[0].2), (tint, true), "murk {murk}");
        let frame = rotation_frame(npc.heading, NonZeroU16::new(72).expect("non-zero"));
        assert_eq!(layers[0].0, frame % 72, "the hull's frame");
        if murk == 0 {
            pass_with(&mut view, Vec::new(), false, Vec::new());
            assert_eq!(
                glow_layers(&view, 2301).first().map(|layer| layer.1),
                Some(crate::draw::lights_tint(31)),
                "falling by its class's WeapDecay 3 × 0.333 a step"
            );
        }
    }
}

/// Runs a frame of `view` whose one pass fires `beams`, of their weapons,
/// and leaves its frame showing them.
fn beaming(view: &mut Faked, beams: &[nova_sim::flight_frame::FrameBeam]) {
    let frame = FlightFrame {
        beams: beams.to_vec(),
        ..view.frame.clone()
    };
    let pass = PassReport {
        ships: frame.ships.clone(),
        beam_weapons: beams.iter().map(|beam| beam.weapon).collect(),
        ..PassReport::default()
    };
    view.sim.set_frame(frame);
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, Anchor::in_flight(RuleSource::Engine));
}

/// The player's beam of [`UNDER`], `length` straight up.
fn under_beam(length: f32) -> nova_sim::flight_frame::FrameBeam {
    beam(ShipRef::Player, UNDER, Vec2::ZERO, Vec2::new(0.0, -length))
}

#[test]
fn a_beam_that_is_not_lightning_draws_no_lightning_rolls() {
    let (effects, effects_chance) = scripted(&[]);
    let mut view = shooting(armed(0, &[UNDER]));
    view.effects_chance = effects_chance;
    beaming(&mut view, &[under_beam(100.0)]);
    assert_eq!(effects.borrow().asked, Vec::<u32>::new());
}

/// The lines drawn in the lightning beam's colour.
fn lightning_lines(view: &Faked) -> Vec<(Point, Point)> {
    super::faked_world::world_list(view)
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
    catalog.looks[4].1 = Ok(WeaponLook {
        flags2: weapons::BEAM_UNDER_SHIPS,
        beam_width: 1,
        beam_color: 0x00AB_CDEF,
        li_density: 3,
        li_amplitude: 5,
        ..WeaponLook::default()
    });
    let (_, effects_chance) = scripted(&[1, 2, 3, 4, 5, 6]);
    let mut view = shooting(catalog);
    view.effects_chance = effects_chance;
    beaming(&mut view, &[under_beam(400.0)]);
    let first = lightning_lines(&view);
    assert_eq!(first.len(), 11, "12 steps of 400 pixels at 3: 11 segments");
    assert_eq!(lightning_lines(&view), first, "the same step, drawn again");
    beaming(&mut view, &[under_beam(400.0)]);
    let second = lightning_lines(&view);
    assert_eq!(second.len(), 11);
    assert_ne!(second, first, "the next step draws anew");
}

#[test]
fn by_the_engine_a_short_lightning_beam_draws_nothing_and_mended_one_line() {
    let catalog = || {
        let mut catalog = armed(0, &[UNDER]);
        catalog.looks[4].1 = Ok(WeaponLook {
            beam_width: 1,
            beam_color: 0x00AB_CDEF,
            li_density: 2,
            li_amplitude: 4,
            ..WeaponLook::default()
        });
        catalog
    };
    let mut view = shooting(catalog());
    beaming(&mut view, &[under_beam(30.0)]);
    assert_eq!(lightning_lines(&view), [], "under 50 pixels at 2");
    let rules = Rulebook::default().with_override(RuleKey::LightningZigZag, RuleSource::Bible);
    let mut mended = shooting_by(catalog(), &rules);
    beaming(&mut mended, &[under_beam(30.0)]);
    assert_eq!(lightning_lines(&mended).len(), 1, "one straight line");
}

// Landing and arriving clear what the view keeps of the shots: the
// concrete flight's, on a real session (rdm:phase/view-sim-seam/phase-9-travel-on-port).

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
