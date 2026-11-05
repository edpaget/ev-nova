//! The system view screen: a system's stellars at their positions, over
//! the parallax starfield, through a camera the keyboard moves.
//!
//! The screen reads the system once, through the [`SystemCatalog`] port,
//! when it is built; drawing and input never read anything.
//!
//! Input:
//!
//! - The shared arrows ([`UiCommand::Left`] and its kin), or their fixed
//!   twins `w`, `a`, `s` and `d`, move the camera at [`CAMERA_SPEED`]
//!   while held: a press (or its key repeats) holds the command and its
//!   release lets it go. Opposite ways cancel.
//! - The shared Activate (Space) recentres the camera on the system's
//!   centre.
//! - Escape is the navigator's, which goes back to the galaxy map.
//!
//! Each key is read through the view's bindings
//! ([`SystemView::with_bindings`]), and the help line ([`help_line`])
//! names the keys bound, through the player's layout.

use std::time::Duration;

use super::camera::{CAMERA_SPEED, Camera};
use super::catalog::{SystemCatalog, SystemId};
use super::scene::{self, SystemScene};
use super::starfield;
use crate::bound_names::{Spelling, group, name_read};
use crate::flight::SharedChance;
use crate::keymap::{Command, CommandInput, CommandReader, SharedBindings, UiCommand, contexts};
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
/// The help line, naming the keys `keys` reads for the device it reads,
/// through the one naming function ([`name_read`]): "Arrows or WASD:
/// move   Space: centre   Esc: back to the map" under the defaults and a
/// US layout. WASD is the movement's fixed twins
/// ([`KeyMap::TWINS`](crate::keymap::KeyMap::TWINS)), named through the
/// layout, so AZERTY reads ZQSD. An unbound command's key is left out,
/// and its entry with it when it was the only one.
#[must_use]
pub fn help_line(keys: &CommandReader) -> String {
    let named = |command: UiCommand| name_read(keys, command, Spelling::Bound);
    let arrows = group(
        [
            UiCommand::Up,
            UiCommand::Down,
            UiCommand::Left,
            UiCommand::Right,
        ]
        .map(named),
        "/",
    );
    let twins = group(
        [
            (UiCommand::Up, Key::KeyW),
            (UiCommand::Left, Key::KeyA),
            (UiCommand::Down, Key::KeyS),
            (UiCommand::Right, Key::KeyD),
        ]
        .map(|(command, twin)| name_read(keys, command, Spelling::Twin(twin))),
        "",
    );
    let movement: Vec<String> = [arrows, twins].into_iter().flatten().collect();
    let movement = (!movement.is_empty()).then(|| format!("{}: move", movement.join(" or ")));
    let centre = named(UiCommand::Activate).map(|key| format!("{key}: centre"));
    let back = named(UiCommand::Back).map(|key| format!("{key}: back to the map"));
    [movement, centre, back]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("   ")
}

/// A system's view.
#[derive(Clone, Debug)]
pub struct SystemView {
    scene: SystemScene,
    camera: Camera,
    /// Time since the view opened, which drives the animations.
    elapsed: Duration,
    /// Its keys, as commands ([`contexts::SYSTEM_VIEW`]): the movement
    /// commands held among them.
    keys: CommandReader,
    /// What a stellar's random next frame (`Flags2` 0x0002) is drawn on.
    chance: SharedChance,
}

impl SystemView {
    /// The view of system `id`, read from `catalog`, with the camera on the
    /// system's centre. Its random frames draw on the default chance,
    /// which takes the last frame allowed, until
    /// [`SystemView::with_chance`] gives it one.
    pub fn new(catalog: &impl SystemCatalog, id: SystemId) -> Self {
        let scene = SystemScene::load(catalog, id);
        let camera = Camera::new(scene.bounds());
        Self {
            scene,
            camera,
            elapsed: Duration::ZERO,
            keys: CommandReader::new(contexts::SYSTEM_VIEW),
            chance: SharedChance::default(),
        }
    }

    /// The view with its stellars' random next frames drawn on `chance`.
    #[must_use]
    pub fn with_chance(self, chance: SharedChance) -> Self {
        Self { chance, ..self }
    }

    /// The view reading its keys through `bindings`.
    #[must_use]
    pub fn with_bindings(mut self, bindings: SharedBindings) -> Self {
        self.keys.set_bindings(bindings);
        self
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
        stellar.sprite.as_ref().ok().map(|_| stellar.frames.shown())
    }

    /// The camera's velocity on one axis, from the commands held for each
    /// way.
    fn velocity(&self, back: UiCommand, forward: UiCommand) -> f32 {
        let way = i8::from(self.keys.held(forward)) - i8::from(self.keys.held(back));
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
        list.text(
            help_line(&self.keys),
            HELP_AT,
            OVERLAY_SIZE,
            None,
            Color::DIM,
        );
    }
}

impl Screen for SystemView {
    /// Never quits, nor goes back: Escape is the navigator's. The
    /// movement commands are held as their keys are; Activate's press or
    /// repeat recentres.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(CommandInput {
            command: Command::Ui(UiCommand::Activate),
            pressed: true,
            ..
        }) = self.keys.follow(input)
        {
            self.camera.recentre();
        }
        ScreenAction::None
    }

    /// Advances the animations and moves the camera for the keys held.
    fn tick(&mut self, dt: Duration) {
        let before = ticks(self.elapsed);
        self.elapsed += dt;
        self.scene
            .step(ticks(self.elapsed) - before, &mut self.chance);
        let seconds = dt.as_secs_f32();
        let (vx, vy) = (
            self.velocity(UiCommand::Left, UiCommand::Right),
            self.velocity(UiCommand::Up, UiCommand::Down),
        );
        self.camera.move_by(vx * seconds, vy * seconds);
    }

    fn draw(&self, list: &mut DrawList) {
        starfield::draw(list, &self.camera);
        scene::draw_stellars(list, &self.scene, &self.camera, None);
        self.draw_overlay(list);
    }

    /// Lets go of every movement command, so the camera stops.
    fn release_keys(&mut self) {
        self.keys.let_go();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::num::NonZeroU16;

    use super::*;
    use crate::draw::crossed_box;
    use crate::system::camera::{CAMERA_MARGIN, VIEW_CENTER};
    use crate::system::catalog::{
        AnimationData, StellarContents, StellarId, StellarSheet, SystemContents,
    };
    use crate::system::scene::PLACEHOLDER;
    use crate::{DrawCommand, Font, MouseButton};
    use crate::{ImageKey, Key};

    /// The help line under the default keys and a US layout.
    const HELP: &str = "Arrows or WASD: move   Space: centre   Esc: back to the map";

    /// The help line a view of Sol draws with `bindings`, named through
    /// `names`, for the device its keys read.
    fn help_drawn(
        bindings: crate::keymap::Bindings,
        names: crate::key_names::fixture::FakeKeyNames,
    ) -> String {
        let shared = SharedBindings::new(bindings).with_key_names(std::rc::Rc::new(names));
        let list = drawn(&sol().with_bindings(shared));
        texts(&list).last().cloned().unwrap_or_default()
    }

    #[test]
    fn the_help_line_names_the_movement_twins_through_the_layout() {
        use crate::key_names::fixture::FakeKeyNames;
        use crate::keymap::Bindings;
        assert_eq!(help_drawn(Bindings::default(), FakeKeyNames::new()), HELP);
        assert_eq!(
            help_drawn(Bindings::default(), FakeKeyNames::azerty()),
            "Arrows or ZQSD: move   Space: centre   Esc: back to the map"
        );
    }

    #[test]
    fn the_help_line_names_a_rebound_command_and_drops_an_unbound_one() {
        use crate::key_names::fixture::FakeKeyNames;
        use crate::keymap::{Bindings, KeyCombo};
        assert_eq!(
            help_drawn(
                Bindings::default()
                    .rebound(UiCommand::Activate, KeyCombo::key(Key::KeyC))
                    .unbound(UiCommand::Back),
                FakeKeyNames::new()
            ),
            "Arrows or WASD: move   C: centre"
        );
        assert_eq!(
            help_drawn(
                Bindings::default().rebound(UiCommand::Right, KeyCombo::key(Key::KeyL)),
                FakeKeyNames::new()
            ),
            "Up Arrow/Down Arrow/Left Arrow/L or WASD: move   Space: centre   Esc: back to the map"
        );
    }

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
            image: crate::SheetImage::Rled(image_id),
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
            dead_sprite: None,
            animation: AnimationData {
                delay,
                frame0_bias: 0,
                flags2: 0,
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
                type_zero_sprite: Ok(sheet(1000, 1, 40)),
                problems: Vec::new(),
            },
            SystemContents {
                id: SystemId(131),
                name: "Empty".to_owned(),
                stellars: Vec::new(),
                type_zero_sprite: Ok(sheet(1000, 1, 40)),
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
                type_zero_sprite: Ok(sheet(1000, 1, 40)),
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
        assert_eq!(view.input(&key(Key::ArrowRight, true)), ScreenAction::None);
        ticks_of(&mut view, 4);
        assert_eq!(center(&view), at(CAMERA_SPEED, 0.0));
        assert_eq!(CAMERA_SPEED, 960.0);
    }

    #[test]
    fn each_movement_key_moves_its_way() {
        let cases = [
            (Key::ArrowLeft, at(-480.0, 0.0)),
            (Key::KeyA, at(-480.0, 0.0)),
            (Key::ArrowRight, at(480.0, 0.0)),
            (Key::KeyD, at(480.0, 0.0)),
            (Key::ArrowUp, at(0.0, -480.0)),
            (Key::KeyW, at(0.0, -480.0)),
            (Key::ArrowDown, at(0.0, 480.0)),
            (Key::KeyS, at(0.0, 480.0)),
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
        view.input(&key(Key::ArrowLeft, true));
        view.input(&key(Key::KeyD, true));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
        view.input(&key(Key::ArrowUp, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(0.0, -240.0));
        view.input(&key(Key::ArrowLeft, false));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, -480.0));
    }

    #[test]
    fn two_keys_for_one_way_keep_moving_until_both_are_released() {
        let mut view = sol();
        view.input(&key(Key::ArrowRight, true));
        view.input(&key(Key::KeyD, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0), "not twice as fast");
        view.input(&key(Key::ArrowRight, false));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(480.0, 0.0));
        view.input(&key(Key::KeyD, false));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(480.0, 0.0), "stopped");
    }

    #[test]
    fn repeats_hold_the_key_without_speeding_it_up() {
        let mut view = sol();
        view.input(&key(Key::ArrowDown, true));
        for _ in 0..5 {
            view.input(&held(Key::ArrowDown));
        }
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(0.0, 240.0));
        // A repeat alone holds the key too, as when the press went elsewhere.
        let mut view = sol();
        view.input(&held(Key::KeyA));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(-240.0, 0.0));
    }

    #[test]
    fn a_rebound_right_and_activate_act_on_their_new_keys_alone() {
        use crate::keymap::{Bindings, KeyCombo, SharedBindings, UiCommand};
        let bindings = SharedBindings::new(
            Bindings::default()
                .rebound(UiCommand::Right, KeyCombo::key(Key::KeyL))
                .rebound(UiCommand::Activate, KeyCombo::key(Key::KeyC)),
        );
        let mut view = sol().with_bindings(bindings);
        view.input(&key(Key::ArrowRight, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(0.0, 0.0), "Right no longer pans");
        view.input(&key(Key::KeyL, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0), "L, held, does");
        view.input(&key(Key::KeyL, false));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0), "let go of");
        view.input(&key(Key::Space, true));
        assert_eq!(center(&view), at(240.0, 0.0), "Space no longer recentres");
        view.input(&key(Key::KeyC, true));
        assert_eq!(center(&view), at(0.0, 0.0), "C does");
    }

    #[test]
    fn a_release_without_a_press_does_nothing() {
        let mut view = sol();
        view.input(&key(Key::ArrowRight, false));
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
        view.input(&key(Key::ArrowRight, true));
        ticks_of(&mut view, 1);
        assert_eq!(center(&view), at(240.0, 0.0));
    }

    #[test]
    fn releasing_the_keys_stops_the_camera() {
        let mut view = sol();
        view.input(&key(Key::ArrowRight, true));
        view.input(&key(Key::KeyS, true));
        view.release_keys();
        ticks_of(&mut view, 2);
        assert_eq!(center(&view), at(0.0, 0.0));
    }

    #[test]
    fn space_recentres() {
        let mut view = sol();
        view.input(&key(Key::ArrowRight, true));
        ticks_of(&mut view, 3);
        view.input(&key(Key::ArrowRight, false));
        assert_eq!(view.input(&key(Key::Space, true)), ScreenAction::None);
        assert_eq!(center(&view), at(0.0, 0.0));
        // Its release does nothing more.
        view.input(&key(Key::ArrowDown, true));
        ticks_of(&mut view, 1);
        view.input(&key(Key::Space, false));
        assert_eq!(center(&view), at(0.0, 240.0));
    }

    #[test]
    fn the_camera_stops_at_the_scenes_bounds() {
        let mut view = sol();
        view.input(&key(Key::ArrowRight, true));
        view.input(&key(Key::ArrowUp, true));
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
            key(Key::KeyX, true),
            key(Key::Unidentified, true),
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
        view.input(&key(Key::ArrowRight, true));
        view.input(&key(Key::ArrowDown, true));
        view.tick(QUARTER);
        view.input(&key(Key::ArrowRight, false));
        view.input(&key(Key::ArrowDown, false));
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
        view.input(&key(Key::ArrowLeft, true));
        view.tick(Duration::from_millis(1));
        view.input(&key(Key::ArrowLeft, false));
        view.input(&key(Key::ArrowUp, true));
        view.tick(Duration::from_millis(250));
        assert!(texts(&drawn(&view)).contains(&"Camera (-1, -240)".to_owned()));
        view.input(&key(Key::ArrowUp, false));
        view.input(&key(Key::ArrowDown, true));
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
            text(&list, "3 problem"),
            DrawCommand::Text {
                text: "3 problem(s) reading the system: sÿst 132: no spöb 150".to_owned(),
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
        let mut shown = Vec::new();
        for _ in 0..10 {
            let list = drawn(&view);
            let keys = sprites(&list);
            assert_eq!(keys[0].0.frame, 0, "Earth stays on frame 0");
            assert_eq!(view.frame(0), Some(0));
            assert_eq!(view.frame(2), Some(keys[2].0.frame));
            shown.push(keys[2].0.frame);
            view.tick(tick);
        }
        assert_eq!(shown, [0, 0, 1, 1, 2, 2, 3, 3, 0, 0]);
    }

    /// At 60 fps each frame is half a tick: the halves add up, so the
    /// Wormhole, a frame every 2 ticks, steps every 4 frames.
    #[test]
    fn half_tick_frames_add_up_to_the_animations_ticks() {
        let tick = Duration::from_secs(1) / 30;
        // `tick / 2` rounds down, so the second half is a nanosecond longer.
        let halves = [tick / 2, Duration::from_nanos(16_666_667)];
        let mut view = sol();
        let mut shown = Vec::new();
        for half in halves.iter().cycle().take(12) {
            shown.push(view.frame(2).expect("a sheet"));
            view.tick(*half);
        }
        assert_eq!(shown, [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2]);
    }
}
