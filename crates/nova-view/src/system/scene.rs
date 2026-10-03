//! A system laid out for viewing: each stellar at its world position with
//! its sheet (or why not) and its animation, read once from the catalog.

use std::num::NonZeroU16;

use super::animation::Animation;
use super::camera::CAMERA_MARGIN;
use super::catalog::{StellarId, StellarSheet, SystemCatalog, SystemId};
use crate::Point;
use crate::geometry::Bounds;

/// One stellar, placed in the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneStellar {
    /// The `spöb`'s ID.
    pub id: StellarId,
    /// Its display name.
    pub name: String,
    /// Where it is, in world units.
    pub position: Point,
    /// Its sprite sheet, or why it cannot be shown.
    pub sprite: Result<StellarSheet, String>,
    /// When each of its frames shows; a stellar with no sheet stays on
    /// frame 0.
    pub animation: Animation,
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
            .map(|stellar| {
                let frames = stellar
                    .sprite
                    .as_ref()
                    .map_or(NonZeroU16::MIN, |sheet| sheet.frames);
                SceneStellar {
                    id: stellar.id,
                    name: stellar.name,
                    position: Point::new(f32::from(stellar.x), f32::from(stellar.y)),
                    sprite: stellar.sprite,
                    animation: Animation::new(frames, stellar.animation),
                }
            })
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::system::catalog::{AnimationData, StellarContents, SystemContents};

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
                    animation: Animation::new(frames(4), WORMHOLE),
                },
                SceneStellar {
                    id: StellarId(128),
                    name: "Stellar 128".to_owned(),
                    position: Point::new(0.0, 0.0),
                    sprite: Ok(sheet(1000, 1)),
                    animation: Animation::new(frames(1), WORMHOLE),
                },
                SceneStellar {
                    id: StellarId(140),
                    name: "Stellar 140".to_owned(),
                    position: Point::new(-1700.0, 900.0),
                    sprite: Err("no spïn 1002".to_owned()),
                    animation: Animation::new(frames(1), WORMHOLE),
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

    #[test]
    fn problems_pass_through() {
        let mut broken = system(133, Vec::new());
        broken.problems = vec!["sÿst 133: no spöb 150".to_owned(), "x".to_owned()];
        let scene = SystemScene::load(&FakeCatalog::new(vec![broken]), SystemId(133));
        assert_eq!(scene.problems(), ["sÿst 133: no spöb 150", "x"]);
    }
}
