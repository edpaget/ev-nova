//! The [`Renderer`](crate::Renderer)'s planets: what each [`PlanetKey`]
//! draws ([`Renderer::add_planet`](crate::Renderer::add_planet)), which of
//! them the GPU holds, and which keys with no planet have been reported;
//! and the [`Batch::Planets`] a draw list's planets become.
//!
//! Each planet is numbered once, the first time its key is added, and its
//! textures ([`crate::planet::textures`]) uploaded
//! ([`Gpu::upload_planet`]) the first frame that draws it. Adding a key
//! again replaces its planet under the same number, uploaded again at its
//! next draw. Planets are not atlas pages, so a new enlarger leaves them
//! where they are.
//!
//! Consecutive planets share one [`Batch::Planets`], lit by
//! [`PLANET_LIGHT`]; any other batch between them splits it, so the list's
//! order holds: planets test no depth and write none, and a models batch
//! after them clears its own.

use std::collections::{HashMap, HashSet};

use nova_view::{PlanetKey, PlanetPose};

use crate::gpu::{Batch, Gpu, PlanetId, PlanetInstance};
use crate::planet::{self, PLANET_LIGHT};
use crate::planet_source::{LoadedPlanet, Surface};

/// A planet the renderer was given, and whether the GPU holds it as it is.
#[derive(Debug)]
struct Held {
    id: PlanetId,
    surface: Surface,
    uploaded: bool,
}

/// The planets the renderer was given, by key.
#[derive(Debug, Default)]
pub(crate) struct Planets {
    held: HashMap<PlanetKey, Held>,
    /// The number the next new key's planet takes.
    next: u32,
    /// The keys with no planet already reported.
    reported: HashSet<PlanetKey>,
}

impl Planets {
    /// Gives `key` `planet`, replacing any planet it had: the replacement
    /// keeps the old one's number and is uploaded at its next draw.
    pub(crate) fn add(&mut self, key: PlanetKey, planet: LoadedPlanet) {
        let surface = planet.into_surface();
        if let Some(held) = self.held.get_mut(&key) {
            held.surface = surface;
            held.uploaded = false;
            return;
        }
        let id = PlanetId(self.next);
        self.next += 1;
        self.held.insert(
            key,
            Held {
                id,
                surface,
                uploaded: false,
            },
        );
    }

    /// Draws planet `key` at `pose` onto `batches`: its instance, its
    /// textures uploaded first through `gpu` if need be, and a key with no
    /// planet pushed onto `missing` once. A pose [`planet::instance`]
    /// refuses draws nothing, and uploads nothing.
    pub(crate) fn draw(
        &mut self,
        key: PlanetKey,
        pose: &PlanetPose,
        batches: &mut Vec<Batch>,
        gpu: &mut impl Gpu,
        missing: &mut Vec<PlanetKey>,
    ) {
        let Some(held) = self.held.get_mut(&key) else {
            if self.reported.insert(key) {
                missing.push(key);
            }
            return;
        };
        let Some(instance) = planet::instance(held.id, &held.surface, pose) else {
            return;
        };
        if !held.uploaded {
            gpu.upload_planet(held.id, &planet::textures(&held.surface));
            held.uploaded = true;
        }
        push(batches, instance);
    }
}

/// Appends `planet`, extending the last batch when it is a planets batch.
fn push(batches: &mut Vec<Batch>, planet: PlanetInstance) {
    if let Some(Batch::Planets { planets, .. }) = batches.last_mut() {
        planets.push(planet);
    } else {
        batches.push(Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![planet],
        });
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::Image;
    use nova_galaxy_media::planet::Image as Layer;
    use nova_view::geometry::Bounds;
    use nova_view::{
        Color, DrawCommand, DrawList, ImageKey, ImageKind, ModelKey, ModelPose, Mounting,
        Orientation, Point,
    };

    use super::*;
    use crate::Renderer;
    use crate::gpu::{Frame, PlanetTextures};
    use crate::images::{ImageError, ImageSource};
    use crate::model::Mesh;
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::viewport::{LogicalSize, Viewport};

    /// One white 4x4 sprite, `rlëD` 1.
    struct OneSprite;

    impl ImageSource for OneSprite {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            match (kind, id) {
                (ImageKind::Rled, 1) => Ok(vec![
                    Image::from_rgba(4, 4, [255; 64].to_vec()).expect("4x4"),
                ]),
                _ => Err(ImageError::Missing),
            }
        }
    }

    const LOGICAL: LogicalSize = LogicalSize { w: 64, h: 48 };
    const GLOBE: PlanetKey = PlanetKey(1);
    const CUBE: ModelKey = ModelKey(1);

    fn viewport() -> Viewport {
        Viewport::new(LOGICAL, (128, 96), 2.0)
    }

    /// A bare 4 by 2 surface in grey `level`.
    fn surface(level: u8) -> Surface {
        Surface {
            surface: Layer {
                width: 4,
                height: 2,
                rgba: vec![level; 32],
            },
            clouds: None,
            rings: None,
            atmosphere: None,
            day: 100.0,
            tilt: 0.0,
        }
    }

    /// A planet that draws with [`surface`] `level`.
    fn planet(level: u8) -> LoadedPlanet {
        LoadedPlanet::surface(surface(level))
    }

    /// A renderer whose key [`GLOBE`] is [`planet`] 9 and [`CUBE`] the cube.
    fn renderer() -> Renderer<OneSprite> {
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_planet(GLOBE, planet(9));
        renderer.add_model(CUBE, Mesh::cube([Color::WHITE; 6]));
        renderer
    }

    fn pose(x: f32, y: f32) -> PlanetPose {
        PlanetPose {
            center: Point::new(x, y),
            radius: 5.0,
            time: 25.0,
        }
    }

    fn model_pose(x: f32, y: f32) -> ModelPose {
        ModelPose {
            center: Point::new(x, y),
            orientation: Orientation::IDENTITY,
            scale: 4.0,
        }
    }

    /// The instance the core makes for [`planet`] 9, as planet 0, at `pose`.
    fn instance(pose: &PlanetPose) -> PlanetInstance {
        planet::instance(PlanetId(0), &surface(9), pose).expect("drawable")
    }

    fn frame_of(
        renderer: &mut Renderer<OneSprite>,
        list: &DrawList,
        gpu: &mut RecordingGpu,
    ) -> Frame {
        renderer.render(list, &viewport(), gpu);
        gpu.submits()
            .last()
            .map(|frame| (*frame).clone())
            .expect("a frame")
    }

    fn uploads(gpu: &RecordingGpu) -> Vec<(PlanetId, PlanetTextures)> {
        gpu.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::UploadPlanet { planet, textures } => Some((*planet, textures.clone())),
                _ => None,
            })
            .collect()
    }

    fn kinds(frame: &Frame) -> Vec<&'static str> {
        frame.batches.iter().map(Batch::kind).collect()
    }

    #[test]
    fn a_planet_uploads_its_textures_once_then_draws_an_instance_each_frame() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(20.0, 20.0));
        let first = frame_of(&mut renderer, &list, &mut gpu);
        let second = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(0), planet::textures(&surface(9)))],
            "once, before the frame that draws it"
        );
        assert!(matches!(gpu.calls[0], GpuCall::UploadPlanet { .. }));
        let expected = vec![Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![instance(&pose(20.0, 20.0))],
        }];
        assert_eq!(first.batches, expected);
        assert_eq!(second.batches, expected);
    }

    #[test]
    fn a_planet_is_numbered_once_and_adding_it_again_replaces_it() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), planet(30));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0))
            .planet(GLOBE, pose(40.0, 20.0));
        frame_of(&mut renderer, &list, &mut gpu);
        let ids: Vec<PlanetId> = uploads(&gpu).into_iter().map(|(id, _)| id).collect();
        assert_eq!(
            ids,
            [PlanetId(1), PlanetId(0)],
            "numbered as they were added"
        );
        renderer.add_planet(GLOBE, planet(50));
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(0), planet::textures(&surface(50)))],
            "the replacement, under the same number, and only it"
        );
        let Batch::Planets { planets, .. } = &frame.batches[0] else {
            panic!("{:?}", frame.batches);
        };
        assert_eq!(planets[1].planet, PlanetId(0));
    }

    #[test]
    fn a_missing_planet_draws_nothing_and_is_reported_once() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(9), pose(20.0, 20.0))
            .planet(PlanetKey(9), pose(30.0, 20.0))
            .planet(PlanetKey(8), pose(30.0, 20.0));
        let first = renderer.render(&list, &viewport(), &mut gpu);
        let second = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(first.missing_planets, [PlanetKey(9), PlanetKey(8)]);
        assert_eq!(second.missing_planets, []);
        assert!(first.missing_models.is_empty());
        assert!(gpu.submits().iter().all(|frame| frame.batches.is_empty()));
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn consecutive_planets_share_one_batch_and_anything_between_splits_it() {
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(10.0, 10.0))
            .planet(GLOBE, pose(30.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(&pose(10.0, 10.0)), instance(&pose(30.0, 10.0))],
            }]
        );
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(10.0, 10.0))
            .sprite(ImageKey::sprite(1, 0), Point::new(14.0, 12.0), Color::WHITE)
            .planet(GLOBE, pose(30.0, 10.0))
            .model(CUBE, model_pose(30.0, 10.0))
            .planet(GLOBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            kinds(&frame),
            ["planets", "sprites", "planets", "models", "planets"]
        );
    }

    #[test]
    fn list_order_holds_between_planets_and_models() {
        // A model after a planet covers it, and a planet after a model
        // covers it: each is its own batch, in order, and the planets are
        // in no models batch, so in no depth band.
        let mut list = DrawList::new();
        list.model(CUBE, model_pose(20.0, 20.0))
            .planet(GLOBE, pose(20.0, 20.0))
            .model(CUBE, model_pose(20.0, 20.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(kinds(&frame), ["models", "planets", "models"]);
        let [
            Batch::Models { models: before, .. },
            _,
            Batch::Models { models: after, .. },
        ] = frame.batches.as_slice()
        else {
            panic!("{:?}", frame.batches);
        };
        assert_eq!(
            (before.len(), after.len()),
            (1, 1),
            "each model alone in its batch"
        );
    }

    #[test]
    fn a_part_after_a_planet_has_no_model_to_mount_on() {
        let mut list = DrawList::new();
        list.model(CUBE, model_pose(20.0, 20.0))
            .planet(GLOBE, pose(20.0, 20.0))
            .mounted(
                CUBE,
                Mounting {
                    at: [0.0; 3],
                    turn: 0.0,
                },
            );
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(kinds(&frame), ["models", "planets"]);
    }

    #[test]
    fn a_planet_with_a_bad_pose_draws_and_uploads_nothing() {
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        for bad in [
            PlanetPose {
                radius: 0.0,
                ..pose(10.0, 10.0)
            },
            PlanetPose {
                radius: f32::NAN,
                ..pose(10.0, 10.0)
            },
            PlanetPose {
                time: f64::INFINITY,
                ..pose(10.0, 10.0)
            },
            pose(f32::INFINITY, 10.0),
        ] {
            list.planet(GLOBE, bad);
        }
        let frame = frame_of(&mut renderer(), &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn a_world_planet_marks_its_batch_world() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .world(|list| {
                list.planet(GLOBE, pose(12.0, 12.0));
            });
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world, [1]);
        assert_eq!(frame.batches[1].kind(), "planets");
    }

    #[test]
    fn a_clipped_planet_draws_uncut() {
        // A clip far smaller than the planet's square.
        let clip = Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(13.0, 13.0),
        };
        let mut clipped = DrawList::new();
        clipped.clipped(
            clip,
            DrawCommand::Planet {
                planet: GLOBE,
                pose: pose(12.0, 12.0),
            },
        );
        let mut plain = DrawList::new();
        plain.planet(GLOBE, pose(12.0, 12.0));
        let a = frame_of(&mut renderer(), &clipped, &mut RecordingGpu::new());
        let b = frame_of(&mut renderer(), &plain, &mut RecordingGpu::new());
        assert_eq!(a.batches, b.batches);
        assert_eq!(
            a.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(&pose(12.0, 12.0))],
            }]
        );
    }
}
