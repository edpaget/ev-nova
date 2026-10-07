//! The system view screen: a system's stellars at their positions, over
//! the parallax starfield, through a camera the keyboard moves.
//!
//! The screen reads the system once, through the [`SystemCatalog`] port,
//! when it is built; drawing and input never read anything.
//!
//! Input:
//!
//! - The arrow keys, or `w`, `a`, `s` and `d`, move the camera at
//!   [`CAMERA_SPEED`] while held: a press (or its key repeats) holds the
//!   key and its release lets it go. Opposite keys cancel.
//! - Space recentres the camera on the system's centre.
//! - Escape is the navigator's, which goes back to the galaxy map.

use std::collections::HashSet;
use std::time::Duration;

use super::camera::{CAMERA_SPEED, Camera};
use super::catalog::{SystemCatalog, SystemId};
use super::scene::{self, SystemScene};
use super::starfield;
use crate::time::ticks;
use crate::{Color, DrawList, Input, Key, Point, Screen, ScreenAction};

pub use super::scene::PLACEHOLDER_SIZE;

/// The overlay: the system's title, the camera line, "No stellars", the
/// problems line and the help line, top-left first.
const TITLE: Point = Point::new(16.0, 32.0);
const TITLE_SIZE: f32 = 20.0;
const CAMERA_LINE: Point = Point::new(16.0, 58.0);
const NO_STELLARS: Point = Point::new(16.0, 78.0);
const PROBLEMS: Point = Point::new(16.0, 98.0);
const PROBLEMS_WRAP: f32 = 992.0;
const HELP_AT: Point = Point::new(16.0, 744.0);
const OVERLAY_SIZE: f32 = 14.0;
/// The help line.
pub const HELP: &str = "Arrows or WASD: move   Space: centre   Esc: back to the map";

/// A system's view.
#[derive(Clone, Debug)]
pub struct SystemView {
    scene: SystemScene,
    camera: Camera,
    /// Time since the view opened, which drives the animations.
    elapsed: Duration,
    /// The movement keys held down.
    held: HashSet<Key>,
}

impl SystemView {
    /// The view of system `id`, read from `catalog`, with the camera on the
    /// system's centre.
    pub fn new(catalog: &impl SystemCatalog, id: SystemId) -> Self {
        let scene = SystemScene::load(catalog, id);
        let camera = Camera::new(scene.bounds());
        Self {
            scene,
            camera,
            elapsed: Duration::ZERO,
            held: HashSet::new(),
        }
    }

    /// The system, as laid out.
    #[must_use]
    pub fn scene(&self) -> &SystemScene {
        &self.scene
    }

    /// The camera.
    #[must_use]
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// The frame showing of stellar `index` (in navigation order), or
    /// `None` if there is no such stellar or it has no sheet.
    #[must_use]
    pub fn frame(&self, index: usize) -> Option<u16> {
        let stellar = self.scene.stellars().get(index)?;
        stellar
            .sprite
            .as_ref()
            .ok()
            .map(|_| stellar.animation.frame(ticks(self.elapsed)))
    }

    /// Whether either key for one way is held.
    fn holding(&self, keys: [Key; 2]) -> bool {
        keys.iter().any(|key| self.held.contains(key))
    }

    /// The camera's velocity on one axis, from the keys held for each way.
    fn velocity(&self, back: [Key; 2], forward: [Key; 2]) -> f32 {
        let way = i8::from(self.holding(forward)) - i8::from(self.holding(back));
        f32::from(way) * CAMERA_SPEED
    }

    fn draw_overlay(&self, list: &mut DrawList) {
        let scene = &self.scene;
        list.text(
            format!("{} (sÿst {})", scene.name(), scene.id().0),
            TITLE,
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        let center = self.camera.center();
        // Rounded to whole units, as integers so that -0.4 shows as 0.
        list.text(
            format!(
                "Camera ({}, {})",
                center.x.round() as i32,
                center.y.round() as i32
            ),
            CAMERA_LINE,
            OVERLAY_SIZE,
            None,
            Color::DIM,
        );
        if scene.stellars().is_empty() {
            list.text("No stellars", NO_STELLARS, OVERLAY_SIZE, None, Color::DIM);
        }
        if let Some(first) = scene.problems().first() {
            list.text(
                format!(
                    "{} problem(s) reading the system: {first}",
                    scene.problems().len()
                ),
                PROBLEMS,
                OVERLAY_SIZE,
                Some(PROBLEMS_WRAP),
                Color::ERROR,
            );
        }
        list.text(HELP, HELP_AT, OVERLAY_SIZE, None, Color::DIM);
    }
}

/// The keys that move the camera each way.
const LEFT: [Key; 2] = [Key::Left, Key::Char('a')];
const RIGHT: [Key; 2] = [Key::Right, Key::Char('d')];
const UP: [Key; 2] = [Key::Up, Key::Char('w')];
const DOWN: [Key; 2] = [Key::Down, Key::Char('s')];

impl Screen for SystemView {
    /// Never quits, nor goes back: Escape is the navigator's.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Input::Key { key, pressed, .. } = *input {
            let moves = [LEFT, RIGHT, UP, DOWN].iter().flatten().any(|&k| k == key);
            if moves {
                if pressed {
                    self.held.insert(key);
                } else {
                    self.held.remove(&key);
                }
            } else if key == Key::Space && pressed {
                self.camera.recentre();
            }
        }
        ScreenAction::None
    }

    /// Advances the animations and moves the camera for the keys held.
    fn tick(&mut self, dt: Duration) {
        self.elapsed += dt;
        let seconds = dt.as_secs_f32();
        let (vx, vy) = (self.velocity(LEFT, RIGHT), self.velocity(UP, DOWN));
        self.camera.move_by(vx * seconds, vy * seconds);
    }

    fn draw(&self, list: &mut DrawList) {
        starfield::draw(list, &self.camera);
        scene::draw_stellars(list, &self.scene, &self.camera, self.elapsed);
        self.draw_overlay(list);
    }

    /// Lets go of every movement key, so the camera stops.
    fn release_keys(&mut self) {
        self.held.clear();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU16;

    use super::*;
    use crate::ImageKey;
    use crate::draw::crossed_box;
    use crate::system::camera::{CAMERA_MARGIN, VIEW_CENTER};
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::system::scene::PLACEHOLDER;
    use crate::{DrawCommand, Font, MouseButton};

    /// Canned systems.
    struct FakeCatalog(Vec<SystemContents>);

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            self.0
                .iter()
                .find(|system| system.id == id)
                .cloned()
                .expect("a canned system")
        }
    }

    fn sheet(image_id: i16, frames: u16, size: u32) -> StellarSheet {
        StellarSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: size,
            frame_height: size,
        }
    }

    fn stellar(
        id: i16,
        name: &str,
        (x, y): (i16, i16),
        sprite: Result<StellarSheet, String>,
        delay: i16,
    ) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: name.to_owned(),
            x,
            y,
            sprite,
            animation: AnimationData {
                delay,
                frame0_bias: 0,
                only_when_destroyed: false,
            },
        }
    }

    /// Sol (130): Earth (128) at (0, 0), one 40 x 40 frame; Mars (158) at
    /// (900, -600), one 20 x 20 frame; a Wormhole (465) at (2000, 0), 4
    /// frames of 30 x 30 every 2 ticks. Empty (131) has nothing; Broken
    /// (132) has a stellar with no sheet at (100, 50) and two problems.
    fn catalog() -> FakeCatalog {
        FakeCatalog(vec![
            SystemContents {
                id: SystemId(130),
                name: "Sol".to_owned(),
                stellars: vec![
                    stellar(128, "Earth", (0, 0), Ok(sheet(1000, 1, 40)), 0),
                    stellar(158, "Mars", (900, -600), Ok(sheet(1002, 1, 20)), 0),
                    stellar(465, "Wormhole", (2000, 0), Ok(sheet(1059, 4, 30)), 2),
                ],
                problems: Vec::new(),
            },
            SystemContents {
                id: SystemId(131),
                name: "Empty".to_owned(),
                stellars: Vec::new(),
                problems: Vec::new(),
            },
            SystemContents {
                id: SystemId(132),
                name: "Broken".to_owned(),
                stellars: vec![stellar(
                    140,
                    "Lost",
                    (100, 50),
                    Err("no spïn 1002".to_owned()),
                    0,
                )],
                problems: vec!["sÿst 132: no spöb 150".to_owned(), "more".to_owned()],
            },
        ])
    }

    fn sol() -> SystemView {
        SystemView::new(&catalog(), SystemId(130))
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    fn held(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    /// A quarter of a second: every distance below is exact in an f32.
    const QUARTER: Duration = Duration::from_millis(250);

    fn ticks_of(view: &mut SystemView, n: u32) {
        for _ in 0..n {
            view.tick(QUARTER);
        }
    }

    fn center(view: &SystemView) -> Point {
        view.camera().center()
    }

    // Building.

    #[test]
    fn it_opens_on_the_systems_centre_with_its_scene() {
        let view = sol();
        assert_eq!(view.scene().id(), SystemId(130));
        assert_eq!(view.scene().stellars().len(), 3);
        assert_eq!(center(&view), at(0.0, 0.0));
        assert_eq!(view.frame(2), Some(0));
        assert_eq!(view.frame(3), None);
    }

    // Moving the camera.

    #[test]
    fn holding_right_moves_the_camera_at_its_speed() {
        let mut view = sol();
        assert_eq!(view.input(&key(Key::Right, true)), ScreenAction::None);
        ticks_of(&mut view, 4);
        assert_eq!(center(&view), at(CAMERA_SPEED, 0.0));
        assert_eq!(CAMERA_SPEED, 960.0);
    }

    #[test]
    fn each_movement_key_moves_its_way() {
        let cases = [
            (Key::Left, at(-480.0, 0.0)),
            (Key::Char('a'), at(-480.0, 0.0)),
            (Key::Right, at(480.0, 0.0)),
            (Key::Char('d'), at(480.0, 0.0)),
            (Key::Up, at(0.0, -480.0)),
            (Key::Char('w'), at(0.0, -480.0)),
            (Key::Down, at(0.0, 480.0)),
            (Key::Char('s'), at(0.0, 480.0)),
        ];
        for (pressed, expected) in cases {
            let mut view = sol();
            view.input(&key(pressed, true));
            ticks_of(&mut view, 2);
            assert_eq!(center(&view), expected, "{pressed:?}");
        }
    }

    #[test]
    fn opposite_keys_cancel_and_a_diagonal_is_not_normalised() {
        let mut view = sol();
        view.input(&key(Key::Left, true));
        view.input(&key(Key::Char('d'), true));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
        view.input(&key(Key::Up, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(0.0, -240.0));
        view.input(&key(Key::Left, false));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, -480.0));
    }

    #[test]
    fn two_keys_for_one_way_keep_moving_until_both_are_released() {
        let mut view = sol();
        view.input(&key(Key::Right, true));
        view.input(&key(Key::Char('d'), true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0), "not twice as fast");
        view.input(&key(Key::Right, false));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(480.0, 0.0));
        view.input(&key(Key::Char('d'), false));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(480.0, 0.0), "stopped");
    }

    #[test]
    fn repeats_hold_the_key_without_speeding_it_up() {
        let mut view = sol();
        view.input(&key(Key::Down, true));
        for _ in 0..5 {
            view.input(&held(Key::Down));
        }
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(0.0, 240.0));
        // A repeat alone holds the key too, as when the press went elsewhere.
        let mut view = sol();
        view.input(&held(Key::Char('a')));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(-240.0, 0.0));
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut view = sol();
        view.input(&key(Key::Right, false));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
        view.input(&key(Key::Right, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0));
    }

    #[test]
    fn releasing_the_keys_stops_the_camera() {
        let mut view = sol();
        view.input(&key(Key::Right, true));
        view.input(&key(Key::Char('s'), true));
        view.release_keys();
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
    }

    #[test]
    fn space_recentres() {
        let mut view = sol();
        view.input(&key(Key::Right, true));
        ticks_of(&mut view, 3);
        view.input(&key(Key::Right, false));
        assert_eq!(view.input(&key(Key::Space, true)), ScreenAction::None);
        assert_eq!(center(&view), at(0.0, 0.0));
        // Its release does nothing more.
        view.input(&key(Key::Down, true));
        ticks_of(&mut view, 1);
        view.input(&key(Key::Space, false));
        assert_eq!(center(&view), at(0.0, 240.0));
    }

    #[test]
    fn the_camera_stops_at_the_scenes_bounds() {
        let mut view = sol();
        view.input(&key(Key::Right, true));
        view.input(&key(Key::Up, true));
        ticks_of(&mut view, 100);
        assert_eq!(
            center(&view),
            at(2000.0 + CAMERA_MARGIN, -600.0 - CAMERA_MARGIN)
        );
    }

    #[test]
    fn other_input_changes_nothing() {
        let mut view = sol();
        let others = [
            key(Key::Escape, true),
            key(Key::Enter, true),
            key(Key::Tab, true),
            key(Key::Char('x'), true),
            key(Key::Other, true),
            Input::PointerMoved(at(1.0, 2.0)),
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: true,
                at: at(1.0, 2.0),
            },
        ];
        for input in others {
            assert_eq!(view.input(&input), ScreenAction::None, "{input:?}");
        }
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
    }

    // Drawing.

    fn drawn(view: &SystemView) -> DrawList {
        let mut list = DrawList::new();
        view.draw(&mut list);
        list
    }

    fn sprites(list: &DrawList) -> Vec<(ImageKey, Point)> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite {
                    image,
                    center,
                    tint,
                    ..
                } => {
                    assert_eq!(*tint, Color::WHITE);
                    Some((*image, *center))
                }
                _ => None,
            })
            .collect()
    }

    fn texts(list: &DrawList) -> Vec<String> {
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn text(list: &DrawList, starting: &str) -> DrawCommand {
        list.iter()
            .find(|c| matches!(c, DrawCommand::Text { text, .. } if text.starts_with(starting)))
            .cloned()
            .unwrap_or_else(|| panic!("no text starting {starting:?} in {:?}", texts(list)))
    }

    fn overlay(text: &str, origin: Point, size: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin,
            size,
            wrap_width: None,
            color,
        }
    }

    #[test]
    fn each_stellar_is_drawn_at_its_world_position() {
        let mut view = sol();
        assert_eq!(
            sprites(&drawn(&view)),
            [
                (ImageKey::sprite(1000, 0), VIEW_CENTER),
                (ImageKey::sprite(1002, 0), at(1412.0, -216.0)),
                (ImageKey::sprite(1059, 0), at(2512.0, 384.0)),
            ]
        );
        view.input(&key(Key::Right, true));
        view.input(&key(Key::Down, true));
        view.tick(QUARTER);
        view.input(&key(Key::Right, false));
        view.input(&key(Key::Down, false));
        let moved: Vec<Point> = sprites(&drawn(&view)).into_iter().map(|(_, p)| p).collect();
        assert_eq!(
            moved,
            [at(272.0, 144.0), at(1172.0, -456.0), at(2272.0, 144.0)]
        );
    }

    #[test]
    fn the_starfield_comes_first_then_the_sprites_then_the_names_then_the_overlay() {
        let list = drawn(&sol());
        let kinds: Vec<&str> = list
            .iter()
            .map(|command| match command {
                DrawCommand::Dot { .. } => "star",
                DrawCommand::Sprite { .. } => "sprite",
                DrawCommand::Text { .. } => "text",
                _ => "other",
            })
            .collect();
        let stars = kinds.iter().take_while(|&&k| k == "star").count();
        assert!(stars > 0);
        let mut starfield = DrawList::new();
        starfield::draw(&mut starfield, sol().camera());
        assert_eq!(stars, starfield.len());
        assert_eq!(
            &kinds[stars..],
            [
                "sprite", "sprite", "sprite", "text", "text", "text", "text", "text", "text"
            ]
        );
        assert_eq!(
            texts(&list),
            [
                "Earth",
                "Mars",
                "Wormhole",
                "Sol (sÿst 130)",
                "Camera (0, 0)",
                HELP,
            ]
        );
    }

    #[test]
    fn names_go_under_each_stellars_frame() {
        let list = drawn(&sol());
        assert_eq!(
            text(&list, "Earth"),
            overlay("Earth", at(492.0, 408.0), 14.0, Color::DIM)
        );
        assert_eq!(
            text(&list, "Mars"),
            overlay("Mars", at(1402.0, -202.0), 14.0, Color::DIM)
        );
        assert_eq!(
            text(&list, "Wormhole"),
            overlay("Wormhole", at(2497.0, 403.0), 14.0, Color::DIM)
        );
    }

    #[test]
    fn a_stellar_without_a_sheet_is_a_placeholder_with_the_reason() {
        let view = SystemView::new(&catalog(), SystemId(132));
        let list = drawn(&view);
        assert_eq!(sprites(&list), []);
        assert_eq!(view.frame(0), None);
        let box_at = at(612.0, 434.0);
        let mut expected = DrawList::new();
        crossed_box(&mut expected, box_at, PLACEHOLDER_SIZE, PLACEHOLDER);
        let lines: Vec<DrawCommand> = list
            .iter()
            .filter(|c| matches!(c, DrawCommand::Line { .. }))
            .cloned()
            .collect();
        assert_eq!(lines, expected.iter().cloned().collect::<Vec<_>>());
        assert_eq!(
            text(&list, "Lost"),
            overlay("Lost", at(580.0, 470.0), 14.0, Color::DIM)
        );
        assert_eq!(
            text(&list, "Sprite unavailable"),
            overlay(
                "Sprite unavailable: no spïn 1002",
                at(580.0, 488.0),
                14.0,
                Color::ERROR
            )
        );
    }

    #[test]
    fn the_overlay_names_the_system_and_shows_the_camera_and_help() {
        let mut view = sol();
        let list = drawn(&view);
        assert_eq!(
            text(&list, "Sol"),
            overlay("Sol (sÿst 130)", TITLE, 20.0, Color::WHITE)
        );
        assert_eq!(
            text(&list, "Camera"),
            overlay("Camera (0, 0)", CAMERA_LINE, 14.0, Color::DIM)
        );
        assert_eq!(
            text(&list, "Arrows"),
            overlay(HELP, HELP_AT, 14.0, Color::DIM)
        );
        assert_eq!(
            HELP,
            "Arrows or WASD: move   Space: centre   Esc: back to the map"
        );
        assert_eq!(
            (TITLE, CAMERA_LINE, HELP_AT),
            (at(16.0, 32.0), at(16.0, 58.0), at(16.0, 744.0))
        );
        // Just under a unit to the left: the camera line rounds.
        view.input(&key(Key::Left, true));
        view.tick(Duration::from_millis(1));
        view.input(&key(Key::Left, false));
        view.input(&key(Key::Up, true));
        view.tick(Duration::from_millis(250));
        assert!(texts(&drawn(&view)).contains(&"Camera (-1, -240)".to_owned()));
        view.input(&key(Key::Up, false));
        view.input(&key(Key::Down, true));
        // Back to just above 0, which shows as 0, not -0.
        view.tick(Duration::from_micros(249_600));
        assert!(
            texts(&drawn(&view)).contains(&"Camera (-1, 0)".to_owned()),
            "{:?}",
            texts(&drawn(&view))
        );
    }

    #[test]
    fn an_empty_system_says_so() {
        let list = drawn(&SystemView::new(&catalog(), SystemId(131)));
        assert_eq!(
            text(&list, "No stellars"),
            overlay("No stellars", NO_STELLARS, 14.0, Color::DIM)
        );
        assert_eq!(NO_STELLARS, at(16.0, 78.0));
        assert!(!texts(&drawn(&sol())).contains(&"No stellars".to_owned()));
    }

    #[test]
    fn problems_are_counted_with_the_first_one_in_red() {
        let list = drawn(&SystemView::new(&catalog(), SystemId(132)));
        assert_eq!(
            text(&list, "2 problem"),
            DrawCommand::Text {
                text: "2 problem(s) reading the system: sÿst 132: no spöb 150".to_owned(),
                font: Font::Geneva,
                origin: at(16.0, 98.0),
                size: 14.0,
                wrap_width: Some(PROBLEMS_WRAP),
                color: Color::ERROR,
            }
        );
        assert!(!texts(&drawn(&sol())).iter().any(|t| t.contains("problem")));
    }

    // Animation.

    #[test]
    fn an_animated_stellar_follows_its_animation_and_a_static_one_stays_put() {
        let mut view = sol();
        let tick = Duration::from_secs(1) / 30;
        let wormhole = view.scene().stellars()[2].animation;
        let mut shown = Vec::new();
        for t in 0..10 {
            let list = drawn(&view);
            let keys = sprites(&list);
            assert_eq!(keys[0].0.frame, 0, "Earth stays on frame 0");
            assert_eq!(view.frame(0), Some(0));
            assert_eq!(keys[2].0.frame, wormhole.frame(t));
            assert_eq!(view.frame(2), Some(wormhole.frame(t)));
            shown.push(keys[2].0.frame);
            view.tick(tick);
        }
        assert_eq!(shown, [0, 0, 1, 1, 2, 2, 3, 3, 0, 0]);
    }
}
