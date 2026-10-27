//! The flight world drawn in `_SetupSpriteWorld`'s order
//! ([`nova_sim::sprite_layer::WORLD_PASSES`]): each beam, effect, smoke
//! puff, escape pod and cargo box in its layer among the ships and the
//! shots (rdm play-feel phase 4). On [`FakeFlight`]: each frame scripted,
//! each effect raised on the view's own effects.

use nova_sim::catalog::Explosion;
use nova_sim::combat::ParticleBurst;
use nova_sim::flight_frame::{FrameShip, PassReport, StepReport, StepRun};

use super::combat::{line_in, podding};
use super::faked_world::{
    asteroid_at, beam, box_at, in_sol_with, npc_129, raise, saying, shot_of, targeting,
    the_player_at, world, world_list, world_list_with,
};
use super::missions::rocky;
use super::shots::smoking_trailing_blaster;
use super::*;
use crate::flight::alert::DOUBLE_TIME_CICN;
use crate::system::camera::Anchor;

/// The orange of [`sprays`]'s particles, at any alpha.
fn orange_dot(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::Dot { color, .. } if (color.r, color.g, color.b) == (0xFF, 0x80, 0x00)
    )
}

/// Five orange particles at `at`, as the blaster's hits spray.
fn sprays(at: Vec2) -> CombatEvent {
    CombatEvent::Particles(ParticleBurst {
        at,
        speed: 1.0,
        variation: 0,
        life: (30, 30),
        color: 0x00FF_8000,
        count: 5,
        spread: 0.0,
    })
}

/// The player at the origin and NPC 0, 100 above it.
fn facing_an_npc() -> Vec<FrameShip> {
    vec![
        the_player_at(Vec2::ZERO, 0.0),
        npc_129(0, Vec2::new(0.0, -100.0), 180.0),
    ]
}

/// A fight in an asteroid field: the player's blaster shot between it
/// and NPC 0 above it, its beam drawn under the ships at the NPC and its
/// beam over them, an asteroid beside them; an explosion at the NPC and
/// its hit's orange spray.
fn firing_with_sprays() -> Faked {
    let npc = Vec2::new(0.0, -100.0);
    let frame = FlightFrame {
        shots: vec![shot_of(0, BLASTER, Vec2::new(0.0, -50.0), 0.0)],
        beams: vec![
            beam(ShipRef::Player, UNDER, Vec2::ZERO, npc),
            beam(ShipRef::Player, OVER, Vec2::new(5.0, 0.0), npc),
        ],
        asteroids: vec![asteroid_at(Vec2::new(-100.0, 50.0))],
        ..in_sol_with(facing_an_npc())
    };
    let mut view = world(rocky(armed(1, &[BLASTER, UNDER, OVER])), frame);
    let explosion = Explosion {
        boom: nova_sim::BoomId(128),
        extra: false,
    };
    raise(
        &mut view,
        &[
            CombatEvent::Exploded {
                at: npc,
                explosion,
                size: 0.0,
            },
            sprays(npc),
        ],
    );
    view
}

#[test]
fn a_beam_and_each_effect_is_drawn_in_its_layer() {
    let mut view = firing_with_sprays();
    let list = world_list_with(&mut view, &targeting(0));
    let stellar = first(&list, sprite_of(1128));
    let under = first(&list, line_in(Color::rgba(0, 0, 255, 255)));
    let asteroid = first(&list, sprite_of(1800));
    let shot = first(&list, sprite_of(3500));
    let npc = first(&list, sprite_of(2001));
    let ship = first(&list, sprite_of(2000));
    let explosion = first(&list, sprite_of(400));
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let particle = first(&list, orange_dot);
    let brackets = first(&list, icon_in(10008..=10023));
    // `_SetupSpriteWorld` @0x21d0c-0x21e24: the stellar layer and its
    // beams, the asteroids, the shots, the ships, the explosions, the
    // target layer and its beams, then `_PostDrawCallback`'s particles.
    let order = [
        stellar, under, asteroid, shot, npc, ship, explosion, brackets, over, particle,
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[test]
fn the_cursor_and_the_double_time_icon_are_drawn_after_the_brackets_and_under_the_over_beams() {
    let mut view = firing_with_sprays();
    view.sim.frame_mut().double_time = true;
    view.refresh_frame();
    view.cursor_input(&Input::PointerMoved(Point::new(40.0, 400.0)));
    let list = world_list_with(&mut view, &targeting(0));
    let brackets = first(&list, icon_in(10008..=10023));
    let cursor = first(&list, sprite_of(651));
    let double_time = first(&list, icon_in(DOUBLE_TIME_CICN..=DOUBLE_TIME_CICN));
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let particle = first(&list, orange_dot);
    // `_InitSprites`' target layer: the brackets (@0x20093-0x20277), the
    // cursor (@0x2027d-0x202f9), the red alert, then double time
    // (@0x20390-0x203fd); then the layer's `_BeamDrawCallback`.
    let order = [brackets, cursor, double_time, over, particle];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

/// Sets a pod adrift in `view` at `at`, as a dying ship's would be.
fn pod_adrift(view: &mut Faked, at: Vec2) {
    raise(
        view,
        &[CombatEvent::PodEjected {
            ship: ShipRef::Npc(NpcId(9)),
            at,
            velocity: Vec2::ZERO,
        }],
    );
}

#[test]
fn an_escape_pod_is_drawn_above_a_disabled_ship_and_below_a_flying_one() {
    // A disabled NPC beside the player, drawn in the disabled ships'
    // layer, and a pod adrift above them.
    let mut frame = in_sol_with(vec![
        the_player_at(Vec2::ZERO, 0.0),
        npc_129(1, Vec2::new(-50.0, 0.0), 0.0),
    ]);
    frame.drawn = vec![
        (ShipRef::Npc(NpcId(1)), nova_sim::ShipLayer::Disabled),
        (ShipRef::Player, nova_sim::ShipLayer::Ships),
    ];
    let mut view = world(podding(), frame);
    pod_adrift(&mut view, Vec2::new(0.0, -100.0));
    let list = world_list(&view);
    let disabled = first(&list, sprite_of(2001));
    let pod = first(&list, sprite_of(1095));
    let ship = first(&list, sprite_of(2000));
    // `_boxLayer` 0x3b53ec, over `_disabledShipLayer` 0x3b53a0 and under
    // `_shipLayer` 0x3b541c.
    assert!(disabled < pod && pod < ship, "{disabled} {pod} {ship}");
}

#[test]
fn an_escape_pod_is_drawn_over_the_cargo_boxes() {
    let frame = FlightFrame {
        cargo_boxes: vec![box_at(Vec2::new(-20.0, 20.0))],
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    let mut view = world(trading(), frame);
    pod_adrift(&mut view, Vec2::new(30.0, 30.0));
    let list = world_list(&view);
    let cargo_box = first(&list, sprite_of(500));
    let pod = first(&list, sprite_of(1095));
    // The 64 boxes of `_boxData`, then the 32 pods of `_podData`
    // (`_InitSprites` @0x1fb89-0x1fd57).
    assert!(cargo_box < pod, "{cargo_box} {pod}");
}

#[test]
fn a_smoke_puff_is_drawn_over_the_boxes_and_under_the_shots_and_ships() {
    let shot = shot_of(0, BLASTER, Vec2::new(0.0, -40.0), 0.0);
    let frame = FlightFrame {
        shots: vec![shot],
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    let mut view = world(smoking_trailing_blaster(), frame);
    pod_adrift(&mut view, Vec2::new(30.0, 30.0));
    // A pass on a smoke frame with the smoking shot in flight leaves its
    // puff and its trail's particles.
    let pass = PassReport {
        ships: vec![the_player_at(Vec2::ZERO, 0.0)],
        shots: vec![shot],
        smoke_frame: true,
        ..PassReport::default()
    };
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            passes: vec![pass],
            ..StepRun::default()
        }],
        stopped: false,
    });
    view.step_frame(1, Anchor::in_flight(RuleSource::Engine));
    let list = world_list(&view);
    let pod = first(&list, sprite_of(1095));
    let puff = first(&list, |command| {
        matches!(
            command,
            DrawCommand::Sprite { image, .. }
                if image.kind == crate::image::ImageKind::Icon && (1008..1016).contains(&image.id)
        )
    });
    let shot = first(&list, sprite_of(3500));
    let ship = first(&list, sprite_of(2000));
    // The trail's blue (0x1040FF).
    let trail = first(&list, |command| {
        matches!(
            command,
            DrawCommand::Dot { color, .. } if color.b == 0xFF && color.r < 0x40
        )
    });
    // `_smokeLayer` 0x3b52d0, over `_boxLayer` 0x3b53ec and under
    // `_shotLayer` 0x3b52f0; the trail's particles after every layer.
    let order = [pod, puff, shot, ship, trail];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

/// Whether a command is the floating map's border: a line in the
/// `FloatingMap` colour on the frame's edges.
fn floating_border_line(command: &DrawCommand) -> bool {
    let mut list = DrawList::new();
    list.push(command.clone());
    !floating_border(&list).is_empty()
}

/// Shows `view`'s floating map, as H's press does, over Sol's drawing.
fn floating_up(view: &mut Faked) {
    view.sim.map_mut().drawing = Some(sol_drawing());
    view.hyperspace_key(HYPERSPACE_MODE_KEY);
    assert!(view.floating.is_up(), "shown");
}

#[test]
fn the_floating_map_is_drawn_over_the_over_beams_and_under_the_particles() {
    let mut view = firing_with_sprays();
    floating_up(&mut view);
    let list = world_list(&view);
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let floating = first(&list, floating_border_line);
    let particle = first(&list, orange_dot);
    // `_PostDrawCallback`: `_FloaterDrawCallback` (@0x38c8d), then
    // `jmp _DrawParticlesOffscreen` (@0x38c9a).
    let order = [over, floating, particle];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[test]
fn the_message_text_is_drawn_over_the_over_beams_and_under_the_floating_map() {
    let mut view = firing_with_sprays();
    saying(&mut view, TOO_CLOSE, 250);
    floating_up(&mut view);
    let list = world_list(&view);
    let message_box = first(&list, |command| {
        *command == fill_command(hud::message_box(VIEW_SIZE), Color::BLACK)
    });
    let stellar = first(&list, sprite_of(1128));
    let over = first(&list, line_in(Color::rgba(0, 255, 0, 255)));
    let text = first(&list, |command| *command == message_drawn(TOO_CLOSE));
    let floating = first(&list, floating_border_line);
    // The box is `_DisplayComm`'s erase of the background frame, under
    // every sprite; `_FloaterDrawCallback` ORs the background's message
    // rect over the sprites (@0x38b52-0x38b90), then blits the floating
    // map (@0x38b95-0x38c0a).
    let order = [message_box, stellar, over, text, floating];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

#[test]
fn the_escort_menu_is_drawn_over_the_floating_map_and_under_the_particles() {
    // The menu's rows are the frame's (`FrameHud::escort_rows`), shown
    // here as a step would leave them.
    let mut view = firing_with_sprays();
    let mut hud = nova_sim::flight_frame::FrameHud::default();
    hud.escort_rows[0].present = true;
    view.escort_menu.toggle(view.elapsed, &hud.escort_rows);
    assert!(view.escort_menu.is_open());
    floating_up(&mut view);
    let list = world_list_with(&mut view, &hud);
    let floating = first(&list, floating_border_line);
    let menu = first(&list, |command| {
        *command == fill_command(escort_menu_frame(), Color::BLACK)
    });
    let particle = first(&list, orange_dot);
    // `_PostDrawCallback`: `_FloaterDrawCallback` draws the floating map
    // (@0x38b95-0x38c0a), then the escort menu (@0x38c0f-0x38c5b), both
    // in the world; then `jmp _DrawParticlesOffscreen` (@0x38c9a).
    let order = [floating, menu, particle];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert_eq!(
        list.layered().nth(menu).map(|(layer, _)| layer),
        Some(crate::DrawLayer::World)
    );
}

/// The escort menu's frame.
fn escort_menu_frame() -> Bounds {
    use crate::flight::escorts::{MENU_AT, MENU_HEIGHT, MENU_WIDTH};
    Bounds::at(MENU_AT, MENU_WIDTH, MENU_HEIGHT)
}

#[test]
fn the_ships_are_drawn_in_the_frames_draw_order_whatever_their_records_order() {
    // A disabled traffic ship under two escorts in their layer, entered
    // hired first, under a flying ship and the player. The order and the
    // layers are the session's (nova-sim's
    // `session::player_led::tests::a_released_escort_is_drawn_over_the_player`,
    // `a_ship_entering_a_layer_is_drawn_over_the_ships_in_it`).
    let at = |id: u32| {
        Vec2::new(
            -200.0 + 80.0 * f32::from(u16::try_from(id).expect("small")),
            0.0,
        )
    };
    let mut frame = in_sol_with(vec![
        the_player_at(Vec2::new(0.0, 100.0), 0.0),
        npc_129(1, at(1), 0.0),
        npc_129(2, at(2), 0.0),
        npc_129(3, at(3), 0.0),
        npc_129(4, at(4), 0.0),
    ]);
    frame.drawn = vec![
        (ShipRef::Npc(NpcId(3)), nova_sim::ShipLayer::Disabled),
        (ShipRef::Npc(NpcId(2)), nova_sim::ShipLayer::Led),
        (ShipRef::Npc(NpcId(1)), nova_sim::ShipLayer::Led),
        (ShipRef::Npc(NpcId(4)), nova_sim::ShipLayer::Ships),
        (ShipRef::Player, nova_sim::ShipLayer::Ships),
    ];
    let view = world(catalog(), frame);
    let list = world_list(&view);
    let drawn_at = |point: Point| {
        list.iter()
            .position(|command| {
                matches!(command, DrawCommand::Sprite { image, center, .. }
                    if (2000..2300).contains(&image.id) && *center == point)
            })
            .expect("drawn")
    };
    let on_screen = |world: Vec2| view.camera().world_to_screen(at_point(world));
    let order: Vec<usize> = [at(3), at(2), at(1), at(4), Vec2::new(0.0, 100.0)]
        .into_iter()
        .map(|world| drawn_at(on_screen(world)))
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}

/// `world` as a screen-space point before the camera's transform.
fn at_point(world: Vec2) -> Point {
    Point::new(world.x, world.y)
}

// Between steps (the drawing's interpolation, on the kept frame).

#[test]
fn an_asteroid_is_drawn_back_along_its_velocity_between_steps() {
    let frame = FlightFrame {
        asteroids: vec![nova_sim::flight_frame::FrameAsteroid {
            velocity: Vec2::new(8.0, -4.0),
            ..asteroid_at(Vec2::new(100.0, 50.0))
        }],
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    let mut view = world(rocky(trading()), frame);
    view.alpha = 0.25;
    let camera = view.camera();
    let asteroids = view.shown_asteroids(&camera);
    assert_eq!(asteroids.len(), 1);
    // A quarter of the way from where it was a step ago.
    assert_eq!(
        asteroids[0].at,
        camera.world_to_screen(Point::new(100.0 - 8.0 * 0.75, 50.0 + 4.0 * 0.75))
    );
}

#[test]
fn a_beam_is_drawn_moved_with_its_firer_between_steps() {
    let mut view = world(
        armed(0, &[UNDER]),
        FlightFrame {
            beams: vec![beam(
                ShipRef::Player,
                UNDER,
                Vec2::ZERO,
                Vec2::new(0.0, -100.0),
            )],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
        },
    );
    // The player is drawn half way from a step ago, (-10, 6).
    view.previous = nova_sim::flight_frame::ShipPlace {
        position: Vec2::new(-10.0, 6.0),
        heading: 0.0,
    };
    view.alpha = 0.5;
    let camera = view.camera();
    let beams = view.shown_beams(&camera);
    assert_eq!(beams.len(), 1);
    assert_eq!(
        beams[0].start,
        camera.world_to_screen(Point::new(-5.0, 3.0))
    );
    assert_eq!(
        beams[0].end,
        camera.world_to_screen(Point::new(-5.0, -97.0))
    );
}

#[test]
fn a_shield_flash_shows_from_the_frame_after_its_hit_until_it_runs_out() {
    let mut view = world(catalog(), in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    let flash = view.flashes.entry(ShipRef::Player).or_default();
    flash.hit(false);
    assert_eq!(view.shield_flash(ShipRef::Player), None, "not stepped yet");
    let flash = view.flashes.get_mut(&ShipRef::Player).expect("hit");
    flash.step(100.0, 100.0, true);
    let shown = view.shield_flash(ShipRef::Player).expect("shown");
    assert!(shown > 0.0, "{shown}");
    let flash = view.flashes.get_mut(&ShipRef::Player).expect("hit");
    while !flash.spent() {
        flash.step(100.0, 100.0, true);
    }
    flash.step(100.0, 100.0, true);
    assert_eq!(view.shield_flash(ShipRef::Player), None, "run out");
}
