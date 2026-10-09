//! A system laid out for viewing: each stellar at its world position with
//! its sheet (or why not) and its animation, read once from the catalog;
//! and [`draw_stellars`], which draws them through a camera for every
//! screen that shows a system.

use std::num::NonZeroU16;
use std::time::Duration;

use super::animation::Animation;
use super::camera::{CAMERA_MARGIN, Camera};
use super::catalog::{
    AnimationData, StellarContents, StellarId, StellarSheet, SystemCatalog, SystemId,
};
use crate::draw::crossed_box;
use crate::geometry::Bounds;
use crate::time::ticks;
use crate::{Color, DrawList, ImageKey, Point};

/// The side of the box drawn where a stellar's sprite cannot be shown.
pub const PLACEHOLDER_SIZE: f32 = 64.0;
/// The placeholder's colour.
pub(crate) const PLACEHOLDER: Color = Color::rgba(128, 128, 128, 255);
/// How far below a stellar its name goes, and the name's size.
const NAME_GAP: f32 = 4.0;
const NAME_SIZE: f32 = 14.0;
/// How far below a placeholder's name the reason goes.
const MESSAGE_GAP: f32 = 18.0;

/// One stellar, placed in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneStellar {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its display name.
    pub name: String,
    /// Where it is, in world units.
    pub position: Point,
    /// Its sprite sheet (its `Type`'s), or why it cannot be shown.
    pub sprite: Result<StellarSheet, String>,
    /// The sheet it draws while destroyed (its `DeadType`'s), or why it
    /// cannot be shown; `None` for no different graphic.
    pub dead_sprite: Option<Result<StellarSheet, String>>,
    /// Its animation fields.
    pub animation_data: AnimationData,
    /// Whether it is destroyed, as last set.
    pub destroyed: bool,
    /// When each frame of the sheet drawn shows; a stellar with no sheet
    /// stays on frame 0.
    pub animation: Animation,
}

impl SceneStellar {
    /// `stellar`, not destroyed.
    fn new(stellar: StellarContents) -> Self {
        let mut placed = Self {
            id: stellar.id,
            name: stellar.name,
            position: Point::new(f32::from(stellar.x), f32::from(stellar.y)),
            sprite: stellar.sprite,
            dead_sprite: stellar.dead_sprite,
            animation_data: stellar.animation,
            destroyed: false,
            animation: Animation::new(NonZeroU16::MIN, stellar.animation, false),
        };
        placed.animation = placed.animation_for_drawn();
        placed
    }

    /// The sheet drawn: the `DeadType` sheet while destroyed, when there
    /// is one that could be read; the `Type` sheet otherwise, which is the
    /// Bible's "don't display different graphic type when destroyed" and
    /// the original's reading between rebuilds (story-runtime phase 9 owns
    /// its `spïn` 1000 after one).
    pub fn drawn(&self) -> &Result<StellarSheet, String> {
        match &self.dead_sprite {
            Some(dead @ Ok(_)) if self.destroyed => dead,
            _ => &self.sprite,
        }
    }

    /// The animation of the sheet drawn, in the current state.
    fn animation_for_drawn(&self) -> Animation {
        let frames = self
            .drawn()
            .as_ref()
            .map_or(NonZeroU16::MIN, |sheet| sheet.frames);
        Animation::new(frames, self.animation_data, self.destroyed)
    }
}

/// One system's stellars, in navigation order.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemScene {
    id: SystemId,
    name: String,
    stellars: Vec<SceneStellar>,
    problems: Vec<String>,
}

impl SystemScene {
    /// System `id`, read from `catalog` once.
    pub fn load(catalog: &impl SystemCatalog, id: SystemId) -> Self {
        let contents = catalog.system(id);
        let stellars = contents
            .stellars
            .into_iter()
            .map(SceneStellar::new)
            .collect();
        Self {
            id: contents.id,
            name: contents.name,
            stellars,
            problems: contents.problems,
        }
    }

    /// The system's ID.
    #[must_use]
    pub fn id(&self) -> SystemId {
        self.id
    }

    /// The system's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Every stellar, in navigation order.
    #[must_use]
    pub fn stellars(&self) -> &[SceneStellar] {
        &self.stellars
    }

    /// What could not be read.
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// Marks each stellar destroyed or not as `destroyed` says, so that it
    /// draws its `DeadType` or `Type` sheet. Only a stellar whose state
    /// changed has its animation rebuilt for the sheet now drawn; every
    /// other is left as it was.
    pub fn set_destroyed(&mut self, destroyed: impl Fn(StellarId) -> bool) {
        for stellar in &mut self.stellars {
            let now = destroyed(stellar.id);
            if stellar.destroyed != now {
                stellar.destroyed = now;
                stellar.animation = stellar.animation_for_drawn();
            }
        }
    }

    /// Where the camera may go: the box round the system's centre and every
    /// stellar, grown by [`CAMERA_MARGIN`].
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        let positions = self.stellars.iter().map(|stellar| stellar.position);
        Bounds::around(std::iter::once(Point::default()).chain(positions))
            .expect("the centre is always in it")
            .grown(CAMERA_MARGIN)
    }
}

/// Draws every stellar of `scene` where `camera` puts it, `elapsed` into
/// its animation: each sprite (or, without a sheet, a placeholder box and
/// the reason), then every name under its stellar, so no sprite covers a
/// name.
pub fn draw_stellars(list: &mut DrawList, scene: &SystemScene, camera: &Camera, elapsed: Duration) {
    let mut labels = Vec::new();
    for stellar in scene.stellars() {
        let at = camera.world_to_screen(stellar.position);
        let (width, height) = match stellar.drawn() {
            Ok(sheet) => {
                let frame = stellar.animation.frame(ticks(elapsed));
                list.sprite(ImageKey::sprite(sheet.image_id, frame), at, Color::WHITE);
                (sheet.frame_width as f32, sheet.frame_height as f32)
            }
            Err(message) => {
                crossed_box(list, at, PLACEHOLDER_SIZE, PLACEHOLDER);
                let below = Point::new(
                    at.x - PLACEHOLDER_SIZE / 2.0,
                    at.y + PLACEHOLDER_SIZE / 2.0 + NAME_GAP + MESSAGE_GAP,
                );
                list.text(
                    format!("Sprite unavailable: {message}"),
                    below,
                    NAME_SIZE,
                    None,
                    Color::ERROR,
                );
                (PLACEHOLDER_SIZE, PLACEHOLDER_SIZE)
            }
        };
        let name_at = Point::new(at.x - width / 2.0, at.y + height / 2.0 + NAME_GAP);
        labels.push((stellar.name.as_str(), name_at));
    }
    for (name, at) in labels {
        list.text(name, at, NAME_SIZE, None, Color::DIM);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::system::catalog::SystemContents;

    /// Canned systems; records every system asked for.
    struct FakeCatalog {
        systems: Vec<SystemContents>,
        asked: RefCell<Vec<SystemId>>,
    }

    impl FakeCatalog {
        fn new(systems: Vec<SystemContents>) -> Self {
            Self {
                systems,
                asked: RefCell::default(),
            }
        }
    }

    impl SystemCatalog for FakeCatalog {
        fn system(&self, id: SystemId) -> SystemContents {
            self.asked.borrow_mut().push(id);
            self.systems
                .iter()
                .find(|system| system.id == id)
                .cloned()
                .expect("a canned system")
        }
    }

    fn sheet(image_id: i16, frames: u16) -> StellarSheet {
        StellarSheet {
            image_id,
            frames: NonZeroU16::new(frames).expect("non-zero"),
            frame_width: 8,
            frame_height: 8,
        }
    }

    const WORMHOLE: AnimationData = AnimationData {
        delay: 2,
        frame0_bias: -1,
        only_when_destroyed: false,
    };

    fn stellar(id: i16, x: i16, y: i16, sprite: Result<StellarSheet, String>) -> StellarContents {
        StellarContents {
            id: StellarId(id),
            name: format!("Stellar {id}"),
            x,
            y,
            sprite,
            dead_sprite: None,
            animation: WORMHOLE,
        }
    }

    fn system(id: i16, stellars: Vec<StellarContents>) -> SystemContents {
        SystemContents {
            id: SystemId(id),
            name: format!("System {id}"),
            stellars,
            problems: Vec::new(),
        }
    }

    /// System 130 holds a 4-frame stellar 129 at (900, -600), a 1-frame
    /// stellar 128 at (0, 0) and stellar 140, with no sheet, at (-1700, 900).
    fn catalog() -> FakeCatalog {
        FakeCatalog::new(vec![
            system(
                130,
                vec![
                    stellar(129, 900, -600, Ok(sheet(1001, 4))),
                    stellar(128, 0, 0, Ok(sheet(1000, 1))),
                    stellar(140, -1700, 900, Err("no spïn 1002".to_owned())),
                ],
            ),
            system(131, Vec::new()),
        ])
    }

    #[test]
    fn a_system_id_gives_its_stellars_at_their_world_positions_in_order() {
        let catalog = catalog();
        let scene = SystemScene::load(&catalog, SystemId(130));
        assert_eq!(scene.id(), SystemId(130));
        assert_eq!(scene.name(), "System 130");
        let frames = |n| NonZeroU16::new(n).expect("non-zero");
        assert_eq!(
            scene.stellars(),
            [
                SceneStellar {
                    id: StellarId(129),
                    name: "Stellar 129".to_owned(),
                    position: Point::new(900.0, -600.0),
                    sprite: Ok(sheet(1001, 4)),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    animation: Animation::new(frames(4), WORMHOLE, false),
                },
                SceneStellar {
                    id: StellarId(128),
                    name: "Stellar 128".to_owned(),
                    position: Point::new(0.0, 0.0),
                    sprite: Ok(sheet(1000, 1)),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    animation: Animation::new(frames(1), WORMHOLE, false),
                },
                SceneStellar {
                    id: StellarId(140),
                    name: "Stellar 140".to_owned(),
                    position: Point::new(-1700.0, 900.0),
                    sprite: Err("no spïn 1002".to_owned()),
                    dead_sprite: None,
                    animation_data: WORMHOLE,
                    destroyed: false,
                    animation: Animation::new(frames(1), WORMHOLE, false),
                },
            ]
        );
        assert!(scene.stellars()[0].animation.is_animated());
        assert!(!scene.stellars()[2].animation.is_animated(), "no sheet");
        assert_eq!(scene.problems(), [] as [String; 0]);
    }

    #[test]
    fn the_catalog_is_read_once_for_that_system() {
        let catalog = catalog();
        let scene = SystemScene::load(&catalog, SystemId(131));
        let _ = (scene.bounds(), scene.stellars());
        assert_eq!(*catalog.asked.borrow(), [SystemId(131)]);
    }

    #[test]
    fn the_bounds_hold_the_centre_and_every_stellar_with_the_margin() {
        let scene = SystemScene::load(&catalog(), SystemId(130));
        assert_eq!(
            scene.bounds(),
            Bounds {
                min: Point::new(-1700.0 - CAMERA_MARGIN, -600.0 - CAMERA_MARGIN),
                max: Point::new(900.0 + CAMERA_MARGIN, 900.0 + CAMERA_MARGIN),
            }
        );
    }

    #[test]
    fn an_empty_systems_bounds_are_the_centre_and_the_margin() {
        let scene = SystemScene::load(&catalog(), SystemId(131));
        assert_eq!(
            scene.bounds(),
            Bounds {
                min: Point::new(-CAMERA_MARGIN, -CAMERA_MARGIN),
                max: Point::new(CAMERA_MARGIN, CAMERA_MARGIN),
            }
        );
    }

    #[test]
    fn the_bounds_include_the_centre_when_every_stellar_is_off_to_one_side() {
        let catalog = FakeCatalog::new(vec![system(
            132,
            vec![
                stellar(150, 3000, 2000, Ok(sheet(1000, 1))),
                stellar(151, 5000, 2500, Ok(sheet(1000, 1))),
            ],
        )]);
        let bounds = SystemScene::load(&catalog, SystemId(132)).bounds();
        assert_eq!(
            bounds,
            Bounds {
                min: Point::new(-CAMERA_MARGIN, -CAMERA_MARGIN),
                max: Point::new(5000.0 + CAMERA_MARGIN, 2500.0 + CAMERA_MARGIN),
            }
        );
    }

    /// Animates only when destroyed, every 2 ticks.
    const DEAD_ANIMATED: AnimationData = AnimationData {
        delay: 2,
        frame0_bias: -1,
        only_when_destroyed: true,
    };

    /// System 134: stellar 150 at (0, 0), whose `DeadType` sheet 2150 has
    /// 3 frames of 16 x 16 and animates only when destroyed; 151 with no
    /// different graphic; 152 whose `DeadType` sheet could not be read;
    /// and a 4-frame wormhole 153, with no `DeadType`.
    fn dying() -> FakeCatalog {
        let dead_sheet = StellarSheet {
            frame_width: 16,
            frame_height: 16,
            ..sheet(2150, 3)
        };
        FakeCatalog::new(vec![system(
            134,
            vec![
                StellarContents {
                    dead_sprite: Some(Ok(dead_sheet)),
                    animation: DEAD_ANIMATED,
                    ..stellar(150, 0, 0, Ok(sheet(1000, 1)))
                },
                stellar(151, 100, 0, Ok(sheet(1001, 1))),
                StellarContents {
                    dead_sprite: Some(Err("no spïn 1007".to_owned())),
                    ..stellar(152, 200, 0, Ok(sheet(1002, 1)))
                },
                stellar(153, 300, 0, Ok(sheet(1003, 4))),
            ],
        )])
    }

    /// Each stellar's sprite drawn `elapsed` in, in order.
    fn drawn(scene: &SystemScene, elapsed: Duration) -> Vec<ImageKey> {
        let mut list = DrawList::new();
        draw_stellars(
            &mut list,
            scene,
            &Camera::centred_on(Point::default()),
            elapsed,
        );
        list.iter()
            .filter_map(|command| match command {
                crate::DrawCommand::Sprite { image, .. } => Some(*image),
                _ => None,
            })
            .collect()
    }

    /// Where `name` is drawn.
    fn name_at(scene: &SystemScene, name: &str) -> Point {
        let mut list = DrawList::new();
        let camera = Camera::centred_on(Point::default());
        draw_stellars(&mut list, scene, &camera, Duration::ZERO);
        list.iter()
            .find_map(|command| match command {
                crate::DrawCommand::Text { text, origin, .. } if text == name => Some(*origin),
                _ => None,
            })
            .expect("the name is drawn")
    }

    #[test]
    fn a_destroyed_stellar_draws_its_dead_type_sheet_until_it_is_whole_again() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let whole = [
            ImageKey::sprite(1000, 0),
            ImageKey::sprite(1001, 0),
            ImageKey::sprite(1002, 0),
            ImageKey::sprite(1003, 0),
        ];
        assert_eq!(drawn(&scene, Duration::ZERO), whole, "loaded whole");
        scene.set_destroyed(|id| id == StellarId(150));
        assert!(scene.stellars()[0].destroyed);
        assert_eq!(
            drawn(&scene, Duration::ZERO)[0],
            ImageKey::sprite(2150, 0),
            "the DeadType sheet"
        );
        assert_eq!(drawn(&scene, Duration::ZERO)[1..], whole[1..]);
        scene.set_destroyed(|_| false);
        assert!(!scene.stellars()[0].destroyed);
        assert_eq!(drawn(&scene, Duration::ZERO), whole, "regenerated");
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_sheet_keeps_its_type() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(|_| true);
        assert!(scene.stellars().iter().all(|stellar| stellar.destroyed));
        assert_eq!(
            drawn(&scene, Duration::ZERO)[1..3],
            [ImageKey::sprite(1001, 0), ImageKey::sprite(1002, 0)],
            "no different graphic, and an unreadable one"
        );
    }

    #[test]
    fn the_name_sits_under_the_sheet_drawn() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::default());
        let whole = name_at(&scene, "Stellar 150");
        assert_eq!(whole, Point::new(at.x - 4.0, at.y + 4.0 + NAME_GAP));
        scene.set_destroyed(|id| id == StellarId(150));
        let dead = name_at(&scene, "Stellar 150");
        assert_eq!(dead, Point::new(at.x - 8.0, at.y + 8.0 + NAME_GAP));
    }

    #[test]
    fn the_animation_follows_the_sheet_drawn_and_the_destroyed_state() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let before = scene.stellars().to_vec();
        assert!(!before[0].animation.is_animated(), "whole, so still");
        assert!(before[3].animation.is_animated());
        scene.set_destroyed(|id| id == StellarId(150));
        let frames = |n| NonZeroU16::new(n).expect("non-zero");
        assert_eq!(
            scene.stellars()[0].animation,
            Animation::new(frames(3), DEAD_ANIMATED, true)
        );
        // Three ticks in: frame 1 of the DeadType sheet.
        assert_eq!(
            drawn(&scene, Duration::from_millis(100))[0],
            ImageKey::sprite(2150, 1)
        );
        assert_eq!(scene.stellars()[1..], before[1..], "the others untouched");
        scene.set_destroyed(|id| id == StellarId(150) || id == StellarId(153));
        assert!(
            !scene.stellars()[3].animation.is_animated(),
            "a normal stellar stops while destroyed"
        );
        assert_eq!(scene.stellars()[0], {
            let mut dead = before[0].clone();
            dead.destroyed = true;
            dead.animation = Animation::new(frames(3), DEAD_ANIMATED, true);
            dead
        });
    }

    #[test]
    fn problems_pass_through() {
        let mut broken = system(133, Vec::new());
        broken.problems = vec!["sÿst 133: no spöb 150".to_owned(), "x".to_owned()];
        let scene = SystemScene::load(&FakeCatalog::new(vec![broken]), SystemId(133));
        assert_eq!(scene.problems(), ["sÿst 133: no spöb 150", "x"]);
    }
}
