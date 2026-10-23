//! The flight world drawn in `_SetupSpriteWorld`'s order
//! ([`nova_sim::sprite_layer::WORLD_PASSES`]): each beam, effect, smoke
//! puff, escape pod and cargo box in its layer among the ships and the
//! shots (rdm play-feel phase 4).

use super::combat::{eject_a_pod, firing_at_an_npc_in, line_in, podding};
use super::missions::{carrying, other_reading, rocky};
use super::shots::smoking_trailing_blaster;
use super::*;
use crate::flight::alert::DOUBLE_TIME_CICN;
use crate::flight::effects::{LoopedSounds, Scene};

/// The orange of [`firing_with_sprays`]'s hit particles, at any alpha.
fn orange_dot(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::Dot { color, .. } if (color.r, color.g, color.b) == (0xFF, 0x80, 0x00)
    )
}

/// [`firing_at_an_npc_in`] where the blaster's hits also spray 5 orange
/// particles, and the system holds an asteroid field.
fn firing_with_sprays() -> View {
    let mut catalog = rocky(armed(1, &[BLASTER, UNDER, OVER]));
    catalog.weapons[0].hit_particles = 5;
    catalog.weapons[0].hit_part_vel = 100;
    catalog.weapons[0].hit_part_life = 30;
    catalog.weapons[0].hit_part_color = 0x00FF_8000;
    firing_at_an_npc_in(catalog).with_rules(other_reading(RuleKey::AsteroidRespawnRing))
}

#[test]
fn a_beam_and_each_effect_is_drawn_in_its_layer() {
    let mut view = firing_with_sprays();
    ticks(&mut view, 2);
    let list = drawn(&view);
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
    let mut view = firing_with_sprays().with_double_time(true);
    view.input(&Input::PointerMoved(Point::new(40.0, 400.0)));
    ticks(&mut view, 2);
    let list = drawn(&view);
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
fn pod_adrift(view: &mut View, at: Vec2) {
    let scene = Scene {
        player: Point::new(0.0, 0.0),
        debris: false,
        caps: false,
        looped: LoopedSounds::Anywhere,
    };
    let event = CombatEvent::PodEjected {
        ship: ShipRef::Npc(NpcId(9)),
        at,
        velocity: Vec2::ZERO,
    };
    view.effects
        .apply(&event, &scene, &view.looks, &mut Script::default());
}

#[test]
fn an_escape_pod_is_drawn_above_a_disabled_ship_and_below_a_flying_one() {
    // Two NPCs, one straight above the player and one beside it, both
    // disabled at once; the player blasts the one above until its pod
    // comes out.
    let mut catalog = podding();
    catalog.hulls[1].strength = 5;
    catalog.traffic[0].1.avg_ships = 2;
    let draws = [placed(750, 650, 180), placed(700, 750, 0)].concat();
    let mut view = fighting(catalog, &draws).with_disable_rule(Rc::new(DisablesTheStrong));
    eject_a_pod(&mut view);
    assert_eq!(npc_ids(&view).len(), 1, "one left, disabled");
    let list = drawn(&view);
    let disabled = first(&list, sprite_of(2001));
    let pod = first(&list, sprite_of(1095));
    let ship = first(&list, sprite_of(2000));
    // `_boxLayer` 0x3b53ec, over `_disabledShipLayer` 0x3b53a0 and under
    // `_shipLayer` 0x3b541c.
    assert!(disabled < pod && pod < ship, "{disabled} {pod} {ship}");
}

#[test]
fn an_escape_pod_is_drawn_over_the_cargo_boxes() {
    let mut view = carrying(0, true);
    command_tap(&mut view, JETTISON_KEY, false);
    pod_adrift(&mut view, Vec2::new(30.0, 30.0));
    let list = drawn(&view);
    let cargo_box = first(&list, sprite_of(500));
    let pod = first(&list, sprite_of(1095));
    // The 64 boxes of `_boxData`, then the 32 pods of `_podData`
    // (`_InitSprites` @0x1fb89-0x1fd57).
    assert!(cargo_box < pod, "{cargo_box} {pod}");
}

#[test]
fn a_smoke_puff_is_drawn_over_the_boxes_and_under_the_shots_and_ships() {
    let mut view = fighting(smoking_trailing_blaster(), &[]);
    pod_adrift(&mut view, Vec2::new(30.0, 30.0));
    view.input(&key(FIRE_KEY, true));
    ticks(&mut view, 4);
    let list = drawn(&view);
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
    // The trail's blue (0x1040FF), the radar's dots being green.
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

#[test]
fn the_floating_map_is_drawn_over_the_over_beams_and_under_the_particles() {
    let mut view = firing_with_sprays();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    ticks(&mut view, 2);
    let list = drawn(&view);
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
    plot(&mut view, 131);
    view.input(&key(JUMP, true));
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    ticks(&mut view, 2);
    let list = drawn(&view);
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
    let mut view = fighters_out();
    tap(&mut view, MENU_KEY);
    assert!(view.escort_menu().is_open());
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    ticks(&mut view, 1);
    let list = drawn(&view);
    let floating = first(&list, floating_border_line);
    let menu = first(&list, |command| {
        *command == fill_command(escort_menu_frame(), Color::BLACK)
    });
    let hud = hud_at(&list);
    // `_FloaterDrawCallback`: the floating map (@0x38b95-0x38c0a), then
    // the escort menu (@0x38c0f-0x38c5b), both in the world.
    assert!(floating < menu && menu < hud, "{floating} {menu} {hud}");
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
