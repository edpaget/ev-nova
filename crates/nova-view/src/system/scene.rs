//! A system laid out for viewing: each stellar at its world position with
//! its sheet (or why not) and its animation, read once from the catalog;
//! and [`draw_stellars`], which draws them through a camera for every
//! screen that shows a system.

use std::collections::BTreeSet;
use std::num::NonZeroU16;
use std::time::Duration;

use super::animation::Animation;
use super::camera::{CAMERA_MARGIN, Camera};
use nova_sim::RuleSource;

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
    /// Whether the radar draws it as a square: a station, hypergate or
    /// wormhole ([`StellarContents::radar_square`]).
    pub radar_square: bool,
    /// Whether it is destroyed, as last set.
    pub destroyed: bool,
    /// Whether it draws graphic type 0's sheet, as last set: destroyed,
    /// with no `DeadType` sheet that could be read, since the last rebuild
    /// point, by [`RuleKey::DeadTypeFallback`](nova_sim::RuleKey::DeadTypeFallback)'s
    /// engine reading.
    pub type_zero: bool,
    /// Graphic type 0's sheet (`spïn` 1000), or why it cannot be shown.
    pub type_zero_sprite: Result<StellarSheet, String>,
    /// When each frame of the sheet drawn shows; a stellar with no sheet
    /// stays on frame 0.
    pub animation: Animation,
}

/// How a stellar looks, for [`SystemScene::set_destroyed`]: whether it is
/// destroyed, and whether it has been dead since the last rebuild point
/// (`Session::dead_since_rebuild`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StellarLook {
    /// Whether it is destroyed.
    pub destroyed: bool,
    /// Whether it has been dead since the last rebuild point.
    pub dead_since_rebuild: bool,
}

impl SceneStellar {
    /// `stellar`, not destroyed, with graphic type 0's sheet `type_zero`.
    fn new(stellar: StellarContents, type_zero: Result<StellarSheet, String>) -> Self {
        let mut placed = Self {
            id: stellar.id,
            name: stellar.name,
            position: Point::new(f32::from(stellar.x), f32::from(stellar.y)),
            sprite: stellar.sprite,
            dead_sprite: stellar.dead_sprite,
            animation_data: stellar.animation,
            radar_square: stellar.radar_square,
            destroyed: false,
            type_zero: false,
            type_zero_sprite: type_zero,
            animation: Animation::new(NonZeroU16::MIN, stellar.animation, false),
        };
        placed.animation = placed.animation_for_drawn();
        placed
    }

    /// The sheet drawn: the `DeadType` sheet while destroyed, when there
    /// is one that could be read; otherwise graphic type 0's sheet while
    /// [`type_zero`](Self::type_zero) holds, as the original's rebuild
    /// fills a sprite slot it left empty (story-runtime phase 9); the
    /// `Type` sheet otherwise, which is the Bible's "don't display
    /// different graphic type when destroyed" and the original's reading
    /// between rebuilds.
    pub fn drawn(&self) -> &Result<StellarSheet, String> {
        match &self.dead_sprite {
            Some(dead @ Ok(_)) if self.destroyed => dead,
            _ if self.type_zero => &self.type_zero_sprite,
            _ => &self.sprite,
        }
    }

    /// Whether it has a `DeadType` sheet that could be read.
    fn has_dead_sheet(&self) -> bool {
        matches!(self.dead_sprite, Some(Ok(_)))
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
    ///
    /// Its problems are the catalog's, then each stellar's `DeadType`
    /// sheet that exists but cannot be read, then graphic type 0's sheet
    /// when it cannot be read and some stellar might fall back to it (one
    /// with no `DeadType` sheet that could be read), whatever the reading
    /// of [`RuleKey::DeadTypeFallback`](nova_sim::RuleKey::DeadTypeFallback):
    /// the sheet is broken either way.
    pub fn load(catalog: &impl SystemCatalog, id: SystemId) -> Self {
        Self::loaded(catalog, id, |_| true)
    }

    /// System `id`, read from `catalog` once, with only the stellars of
    /// `shown`, in navigation order: the stellars the flight's session
    /// shows (`Session::stellars_shown`), as the original draws only the
    /// stellars the player's system holds. Its problems are as
    /// [`SystemScene::load`] gives them, for the stellars kept alone.
    pub fn load_shown(
        catalog: &impl SystemCatalog,
        id: SystemId,
        shown: &BTreeSet<StellarId>,
    ) -> Self {
        Self::loaded(catalog, id, |stellar| shown.contains(&stellar))
    }

    /// System `id`, read from `catalog` once, with the stellars `keep`
    /// keeps (see [`SystemScene::load`]).
    fn loaded(
        catalog: &impl SystemCatalog,
        id: SystemId,
        keep: impl Fn(StellarId) -> bool,
    ) -> Self {
        let contents = catalog.system(id);
        let type_zero = contents.type_zero_sprite;
        let stellars: Vec<SceneStellar> = contents
            .stellars
            .into_iter()
            .filter(|stellar| keep(stellar.id))
            .map(|stellar| SceneStellar::new(stellar, type_zero.clone()))
            .collect();
        let mut problems = contents.problems;
        for stellar in &stellars {
            if let Some(Err(reason)) = &stellar.dead_sprite {
                problems.push(format!("spöb {} DeadType: {reason}", stellar.id.0));
            }
        }
        if let Err(reason) = &type_zero
            && stellars.iter().any(|stellar| !stellar.has_dead_sheet())
        {
            problems.push(format!("spïn 1000 (graphic type 0): {reason}"));
        }
        Self {
            id: contents.id,
            name: contents.name,
            stellars,
            problems,
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

    /// Marks each stellar destroyed or not as `look` says, so that it
    /// draws its `DeadType`, graphic type 0's or its `Type` sheet
    /// ([`SceneStellar::drawn`]). By `reading`, the reading of
    /// [`RuleKey::DeadTypeFallback`](nova_sim::RuleKey::DeadTypeFallback),
    /// the engine's, a destroyed stellar with no `DeadType` sheet that
    /// could be read draws graphic type 0's while it has been dead since
    /// the last rebuild point; by the Bible's it keeps its `Type`. Only a
    /// stellar whose state changed has its animation rebuilt for the sheet
    /// now drawn; every other is left as it was.
    pub fn set_destroyed(&mut self, look: impl Fn(StellarId) -> StellarLook, reading: RuleSource) {
        for stellar in &mut self.stellars {
            let now = look(stellar.id);
            let type_zero = reading == RuleSource::Engine
                && now.destroyed
                && now.dead_since_rebuild
                && !stellar.has_dead_sheet();
            if stellar.destroyed != now.destroyed || stellar.type_zero != type_zero {
                stellar.destroyed = now.destroyed;
                stellar.type_zero = type_zero;
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
            radar_square: false,
        }
    }

    /// Graphic type 0's sheet's `rlëD` (`spïn` 1000), of 2 frames.
    const TYPE_ZERO: i16 = 3000;

    fn system(id: i16, stellars: Vec<StellarContents>) -> SystemContents {
        SystemContents {
            id: SystemId(id),
            name: format!("System {id}"),
            stellars,
            type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
            problems: Vec::new(),
        }
    }

    /// System 130 holds a 4-frame stellar 129 at (900, -600), a square on
    /// the radar, a 1-frame
    /// stellar 128 at (0, 0) and stellar 140, with no sheet, at (-1700, 900).
    fn catalog() -> FakeCatalog {
        FakeCatalog::new(vec![
            system(
                130,
                vec![
                    StellarContents {
                        radar_square: true,
                        ..stellar(129, 900, -600, Ok(sheet(1001, 4)))
                    },
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
                    type_zero: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    radar_square: true,
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
                    type_zero: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    radar_square: false,
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
                    type_zero: false,
                    type_zero_sprite: Ok(sheet(TYPE_ZERO, 2)),
                    radar_square: false,
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

    /// A stellar destroyed or not, as `destroyed` says, not dead since the
    /// last rebuild point.
    fn whole_since(destroyed: bool) -> StellarLook {
        StellarLook {
            destroyed,
            dead_since_rebuild: false,
        }
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
        scene.set_destroyed(|id| whole_since(id == StellarId(150)), RuleSource::Engine);
        assert!(scene.stellars()[0].destroyed);
        assert_eq!(
            drawn(&scene, Duration::ZERO)[0],
            ImageKey::sprite(2150, 0),
            "the DeadType sheet"
        );
        assert_eq!(drawn(&scene, Duration::ZERO)[1..], whole[1..]);
        scene.set_destroyed(|_| whole_since(false), RuleSource::Engine);
        assert!(!scene.stellars()[0].destroyed);
        assert_eq!(drawn(&scene, Duration::ZERO), whole, "regenerated");
    }

    #[test]
    fn a_destroyed_stellar_with_no_dead_type_sheet_keeps_its_type() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(|_| whole_since(true), RuleSource::Engine);
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
        scene.set_destroyed(|id| whole_since(id == StellarId(150)), RuleSource::Engine);
        let dead = name_at(&scene, "Stellar 150");
        assert_eq!(dead, Point::new(at.x - 8.0, at.y + 8.0 + NAME_GAP));
    }

    #[test]
    fn the_animation_follows_the_sheet_drawn_and_the_destroyed_state() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let before = scene.stellars().to_vec();
        assert!(!before[0].animation.is_animated(), "whole, so still");
        assert!(before[3].animation.is_animated());
        scene.set_destroyed(|id| whole_since(id == StellarId(150)), RuleSource::Engine);
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
        scene.set_destroyed(
            |id| whole_since(id == StellarId(150) || id == StellarId(153)),
            RuleSource::Engine,
        );
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
    fn a_scene_loaded_for_a_shown_set_keeps_only_those_stellars_in_navigation_order() {
        let mut catalog = dying();
        catalog.systems[0].problems = vec!["sÿst 134: no spöb 160".to_owned()];
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let shown = [StellarId(153), StellarId(150)].into();
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &shown);
        let ids: Vec<i16> = scene
            .stellars()
            .iter()
            .map(|stellar| stellar.id.0)
            .collect();
        assert_eq!(ids, [150, 153]);
        assert_eq!(
            scene.problems(),
            [
                "sÿst 134: no spöb 160",
                "spïn 1000 (graphic type 0): no spïn 1000"
            ],
            "152's DeadType left out with it"
        );
        let shown = [StellarId(150)].into();
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &shown);
        assert_eq!(
            scene.problems(),
            ["sÿst 134: no spöb 160"],
            "150 has its DeadType, so none falls back"
        );
        let scene = SystemScene::load_shown(&catalog, SystemId(134), &BTreeSet::new());
        assert_eq!(scene.stellars(), []);
        assert_eq!(scene.id(), SystemId(134));
    }

    #[test]
    fn problems_pass_through() {
        let mut broken = system(133, Vec::new());
        broken.problems = vec!["sÿst 133: no spöb 150".to_owned(), "x".to_owned()];
        let scene = SystemScene::load(&FakeCatalog::new(vec![broken]), SystemId(133));
        assert_eq!(scene.problems(), ["sÿst 133: no spöb 150", "x"]);
    }

    // A `DeadType` with no sheet, after a rebuild point.

    /// Every stellar destroyed, and dead since the last rebuild point as
    /// `since` says.
    fn all_dead(since: bool) -> impl Fn(StellarId) -> StellarLook {
        move |_| StellarLook {
            destroyed: true,
            dead_since_rebuild: since,
        }
    }

    #[test]
    fn a_dead_type_sheet_that_cannot_be_read_is_a_problem() {
        let scene = SystemScene::load(&dying(), SystemId(134));
        assert_eq!(scene.problems(), ["spöb 152 DeadType: no spïn 1007"]);
    }

    #[test]
    fn the_catalogs_problems_come_first() {
        let mut catalog = dying();
        catalog.systems[0].problems = vec!["sÿst 134: no spöb 160".to_owned()];
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(
            scene.problems(),
            ["sÿst 134: no spöb 160", "spöb 152 DeadType: no spïn 1007"]
        );
    }

    #[test]
    fn a_missing_type_zero_sheet_is_a_problem_only_when_a_stellar_might_fall_back_to_it() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(
            scene.problems(),
            [
                "spöb 152 DeadType: no spïn 1007",
                "spïn 1000 (graphic type 0): no spïn 1000",
            ]
        );
        let first = catalog.systems[0].stellars[0].clone();
        catalog.systems[0].stellars = vec![first];
        let scene = SystemScene::load(&catalog, SystemId(134));
        assert_eq!(scene.problems(), [] as [String; 0], "150 has its DeadType");
    }

    #[test]
    fn by_the_engine_a_destroyed_stellar_with_no_dead_type_sheet_draws_type_zero_since_a_rebuild() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        scene.set_destroyed(all_dead(true), RuleSource::Engine);
        assert_eq!(
            drawn(&scene, Duration::ZERO),
            [
                ImageKey::sprite(2150, 0),
                ImageKey::sprite(TYPE_ZERO, 0),
                ImageKey::sprite(TYPE_ZERO, 0),
                ImageKey::sprite(TYPE_ZERO, 0),
            ],
            "150 keeps its DeadType sheet"
        );
        let flags: Vec<bool> = scene.stellars().iter().map(|s| s.type_zero).collect();
        assert_eq!(flags, [false, true, true, true]);
        let frames = |n| NonZeroU16::new(n).expect("non-zero");
        assert_eq!(
            scene.stellars()[1].animation,
            Animation::new(frames(2), WORMHOLE, true),
            "over type 0's frames"
        );
        assert_eq!(
            scene.stellars()[3].animation,
            Animation::new(frames(2), WORMHOLE, true),
            "rebuilt though it was destroyed already"
        );

        scene.set_destroyed(all_dead(false), RuleSource::Engine);
        assert_eq!(
            drawn(&scene, Duration::ZERO)[1..3],
            [ImageKey::sprite(1001, 0), ImageKey::sprite(1002, 0)],
            "not dead since a rebuild: Type"
        );
        assert!(scene.stellars().iter().all(|s| !s.type_zero));
        assert_eq!(
            scene.stellars()[3].animation,
            Animation::new(frames(4), WORMHOLE, true),
            "back over Type's frames"
        );
    }

    #[test]
    fn a_whole_stellar_dead_since_a_rebuild_draws_type() {
        let mut scene = SystemScene::load(&dying(), SystemId(134));
        let look = |_| StellarLook {
            destroyed: false,
            dead_since_rebuild: true,
        };
        scene.set_destroyed(look, RuleSource::Engine);
        assert!(scene.stellars().iter().all(|s| !s.type_zero));
        assert_eq!(drawn(&scene, Duration::ZERO)[1], ImageKey::sprite(1001, 0));
    }

    #[test]
    fn by_the_bible_a_destroyed_stellar_with_no_dead_type_sheet_keeps_its_type() {
        for since in [false, true] {
            let mut scene = SystemScene::load(&dying(), SystemId(134));
            scene.set_destroyed(all_dead(since), RuleSource::Bible);
            assert_eq!(
                drawn(&scene, Duration::ZERO),
                [
                    ImageKey::sprite(2150, 0),
                    ImageKey::sprite(1001, 0),
                    ImageKey::sprite(1002, 0),
                    ImageKey::sprite(1003, 0),
                ],
                "{since}"
            );
            assert!(scene.stellars().iter().all(|s| !s.type_zero), "{since}");
        }
    }

    #[test]
    fn the_name_sits_under_the_type_zero_sheet_drawn() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Ok(StellarSheet {
            frame_width: 40,
            frame_height: 40,
            ..sheet(TYPE_ZERO, 2)
        });
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        let at = Camera::centred_on(Point::default()).world_to_screen(Point::new(100.0, 0.0));
        scene.set_destroyed(all_dead(true), RuleSource::Engine);
        assert_eq!(
            name_at(&scene, "Stellar 151"),
            Point::new(at.x - 20.0, at.y + 20.0 + NAME_GAP)
        );
    }

    #[test]
    fn a_type_zero_sheet_that_cannot_be_read_draws_the_placeholder_and_why() {
        let mut catalog = dying();
        catalog.systems[0].type_zero_sprite = Err("no spïn 1000".to_owned());
        let mut scene = SystemScene::load(&catalog, SystemId(134));
        scene.set_destroyed(all_dead(true), RuleSource::Engine);
        assert_eq!(scene.stellars()[1].drawn(), &Err("no spïn 1000".to_owned()));
    }
}
