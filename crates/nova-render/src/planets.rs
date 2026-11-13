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
//! # A planet is made ready at its first draw
//!
//! A planet may be added unbaked ([`LoadedPlanet::unbaked`]): the new
//! game adds every generated planet of its galaxy that way at start-up,
//! and a bake takes tens of milliseconds. Adding costs nothing. The first
//! frame whose draw list commands the planet with a pose that draws
//! ([`planet::quad`]) bakes it, builds its mips and uploads it, all before
//! that frame is submitted, so the frame draws it: nothing is ever drawn
//! as nothing while it waits. It is baked once, and holds its surface
//! from then on. A planet never commanded is never baked.
//!
//! So what a screen commands decides when the pause falls. The flight
//! commands every stellar of its system each frame, on screen or off
//! (`nova_view`'s `stellar_models`), so a system's planets are all made
//! ready in the first frame drawn there, which after a jump is drawn
//! under the arrival's white. A screen that culled its planets would move
//! each bake to the frame the planet scrolls into view.
//!
//! Consecutive planets share one [`Batch::Planets`], lit by
//! [`PLANET_LIGHT`], while they are clipped alike
//! ([`nova_view::DrawCommand::Clipped`]): the batch carries their clip
//! ([`Batch::clip`]), which the adapter scissors it to, so a planet with
//! another clip, or none, opens a batch of its own. Any other batch
//! between them splits it too, so the list's order holds: planets test no
//! depth and write none, and a models batch after them clears its own. A
//! planet clipped to no area draws nothing and is not made ready; one
//! inside a clip with area is made ready even when it lies wholly outside
//! the clip.

use std::collections::{HashMap, HashSet};

use nova_view::{PlanetKey, PlanetPose};

use crate::gpu::{Batch, Gpu, PlanetId, PlanetInstance, Rect};
use crate::planet::{self, PLANET_LIGHT};
use crate::planet_source::LoadedPlanet;

/// A planet the renderer was given, and whether the GPU holds it as it is.
#[derive(Debug)]
struct Held {
    id: PlanetId,
    /// As it was given, until a frame draws it; baked from then on.
    planet: LoadedPlanet,
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
    /// keeps the old one's number and is uploaded at its next draw. An
    /// unbaked planet is kept unbaked.
    pub(crate) fn add(&mut self, key: PlanetKey, planet: LoadedPlanet) {
        if let Some(held) = self.held.get_mut(&key) {
            held.planet = planet;
            held.uploaded = false;
            return;
        }
        let id = PlanetId(self.next);
        self.next += 1;
        self.held.insert(
            key,
            Held {
                id,
                planet,
                uploaded: false,
            },
        );
    }

    /// Draws planet `key` at `pose`, only inside `clip` when it has one,
    /// onto `batches`: its instance, the planet baked first if it was
    /// added unbaked and its textures uploaded through `gpu` if need be,
    /// and a key with no planet pushed onto `missing` once. A pose
    /// [`planet::quad`] refuses draws nothing, and bakes and uploads
    /// nothing. A planet wholly outside its clip is still made ready: the
    /// GPU's scissor, not the core, cuts it.
    pub(crate) fn draw(
        &mut self,
        key: PlanetKey,
        (pose, clip): (&PlanetPose, Option<Rect>),
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
        // A pose that draws nothing is refused before the bake it would
        // waste: the reach needs none.
        if planet::quad(pose, held.planet.reach()).is_none() {
            return;
        }
        let surface = held.planet.bake();
        let Some(instance) = planet::instance(held.id, surface, pose) else {
            return;
        };
        if !held.uploaded {
            gpu.upload_planet(held.id, &planet::textures(surface));
            held.uploaded = true;
        }
        push(batches, instance, clip);
    }
}

/// Appends `planet`, drawn only inside `clip` when it has one, extending
/// the last batch when it is a planets batch clipped alike.
fn push(batches: &mut Vec<Batch>, planet: PlanetInstance, clip: Option<Rect>) {
    match batches.last_mut() {
        Some(Batch::Planets {
            planets,
            clip: last,
            ..
        }) if *last == clip => planets.push(planet),
        _ => batches.push(Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![planet],
            clip,
        }),
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::Image;
    use nova_galaxy_format::model::Look;
    use nova_galaxy_media::planet::{Image as Layer, Sizes, Surface, bake};
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
            clip: None,
        }];
        assert_eq!(first.batches, expected);
        assert_eq!(second.batches, expected);
    }

    /// Looks baked small, so the tests stay fast.
    const SMALL: Sizes = Sizes {
        surface: 32,
        clouds: 16,
        ring: 16,
    };

    /// A generated look of seed `seed`, with rings and an atmosphere.
    fn look(seed: u32) -> Look {
        use nova_galaxy_format::model::{Stellar, Visual};
        let text = format!(
            "---\nname: Verdance\nkind: planet\nsystem: alpha\nposition: [0, 0]\nvisual:\n\
             \x20 generated:\n    type: garden\n    palette: [\"#3a7a3a\", \"#2a4a8a\"]\n\
             \x20   clouds: 0.5\n    rings: {{inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}}\n\
             \x20   atmosphere: {{color: \"#f0e0c0\", thickness: 0.25}}\n    seed: {seed}\n---\n"
        );
        let stellar = nova_galaxy_format::frontmatter::parse::<Stellar>(&text).expect("a stellar");
        match stellar.0.visual {
            Some(Visual::Generated(look)) => look,
            other => panic!("a generated look: {other:?}"),
        }
    }

    /// The look of seed `seed`, not baked.
    fn unbaked(seed: u32) -> LoadedPlanet {
        LoadedPlanet::unbaked(look(seed), SMALL)
    }

    /// Whether the renderer holds the planet `key` baked.
    fn is_baked(renderer: &Renderer<OneSprite>, key: PlanetKey) -> bool {
        renderer.planets().held[&key].planet.baked().is_some()
    }

    #[test]
    fn an_unbaked_planet_added_and_never_drawn_is_not_baked_and_uploads_nothing() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(GLOBE, pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut gpu);
        let ids: Vec<PlanetId> = uploads(&gpu).into_iter().map(|(id, _)| id).collect();
        assert_eq!(ids, [PlanetId(0)], "only the globe drawn");
        assert!(!is_baked(&renderer, PlanetKey(2)));
    }

    #[test]
    fn the_first_frame_that_draws_an_unbaked_planet_bakes_and_uploads_it_once() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        let first = frame_of(&mut renderer, &list, &mut gpu);
        assert!(is_baked(&renderer, PlanetKey(2)));
        let baked = bake(&look(77), SMALL);
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&baked))],
            "its bake's textures, before the frame that draws it"
        );
        assert!(matches!(gpu.calls[0], GpuCall::UploadPlanet { .. }));
        // Its tilt, turns, rings and atmosphere are its bake's.
        let instance = planet::instance(PlanetId(1), &baked, &pose(20.0, 20.0)).expect("drawable");
        assert!(instance.rings[1] > 2.0 && instance.atmosphere[3] > 0.0);
        let expected = vec![Batch::Planets {
            light: PLANET_LIGHT,
            planets: vec![instance],
            clip: None,
        }];
        assert_eq!(first.batches, expected);
        let second = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(second.batches, expected);
        assert_eq!(uploads(&gpu).len(), 1, "the second frame uploads nothing");
    }

    #[test]
    fn a_refused_pose_neither_bakes_nor_uploads_and_a_later_good_pose_does_once() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(
            PlanetKey(2),
            PlanetPose {
                radius: 0.0,
                ..pose(20.0, 20.0)
            },
        )
        .planet(PlanetKey(2), pose(f32::NAN, 20.0));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
        assert!(!is_baked(&renderer, PlanetKey(2)), "nothing drew it");
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        for _ in 0..2 {
            let frame = frame_of(&mut renderer, &list, &mut gpu);
            assert_eq!(kinds(&frame), ["planets"]);
        }
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&bake(&look(77), SMALL)))]
        );
    }

    #[test]
    fn two_unbaked_planets_drawn_in_one_frame_are_both_drawn_in_it() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        renderer.add_planet(PlanetKey(3), unbaked(78));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0))
            .planet(PlanetKey(3), pose(40.0, 20.0));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        let (first, second) = (bake(&look(77), SMALL), bake(&look(78), SMALL));
        assert_ne!(first, second, "two looks");
        assert_eq!(
            uploads(&gpu),
            [
                (PlanetId(1), planet::textures(&first)),
                (PlanetId(2), planet::textures(&second))
            ]
        );
        let instance = |id, surface, x| {
            planet::instance(PlanetId(id), surface, &pose(x, 20.0)).expect("drawable")
        };
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(1, &first, 20.0), instance(2, &second, 40.0)],
                clip: None,
            }]
        );
    }

    #[test]
    fn a_key_given_a_new_unbaked_planet_bakes_again_at_its_next_draw() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut list = DrawList::new();
        list.planet(PlanetKey(2), pose(20.0, 20.0));
        frame_of(&mut renderer, &list, &mut RecordingGpu::new());
        renderer.add_planet(PlanetKey(2), unbaked(78));
        assert!(!is_baked(&renderer, PlanetKey(2)), "not until it is drawn");
        let mut gpu = RecordingGpu::new();
        for _ in 0..2 {
            frame_of(&mut renderer, &list, &mut gpu);
        }
        assert_eq!(
            uploads(&gpu),
            [(PlanetId(1), planet::textures(&bake(&look(78), SMALL)))],
            "the new look, under the same number, once"
        );
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
                clip: None,
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

    fn bounds(x0: f32, y0: f32, x1: f32, y1: f32) -> Bounds {
        Bounds {
            min: Point::new(x0, y0),
            max: Point::new(x1, y1),
        }
    }

    /// `bounds` as a batch carries it.
    fn rect_of(bounds: Bounds) -> crate::Rect {
        crate::Rect {
            x: bounds.min.x,
            y: bounds.min.y,
            w: bounds.width(),
            h: bounds.height(),
        }
    }

    fn globe_at(pose: PlanetPose) -> DrawCommand {
        DrawCommand::Planet {
            planet: GLOBE,
            pose,
        }
    }

    #[test]
    fn a_clipped_planet_carries_its_clip_on_its_batch() {
        // A clip far smaller than the planet's square.
        let clip = bounds(0.0, 0.0, 13.0, 13.0);
        let mut clipped = DrawList::new();
        clipped.clipped(clip, globe_at(pose(12.0, 12.0)));
        let frame = frame_of(&mut renderer(), &clipped, &mut RecordingGpu::new());
        assert_eq!(
            frame.batches,
            [Batch::Planets {
                light: PLANET_LIGHT,
                planets: vec![instance(&pose(12.0, 12.0))],
                clip: Some(crate::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 13.0,
                    h: 13.0
                }),
            }]
        );
    }

    #[test]
    fn planets_clipped_alike_share_a_batch_and_another_clip_splits_it() {
        let (c1, c2) = (bounds(0.0, 0.0, 20.0, 20.0), bounds(10.0, 0.0, 40.0, 30.0));
        let mut list = DrawList::new();
        list.clipped(c1, globe_at(pose(10.0, 10.0)))
            .clipped(c1, globe_at(pose(14.0, 10.0)))
            .clipped(c2, globe_at(pose(30.0, 10.0)))
            .planet(GLOBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let batch = |clip, poses: &[PlanetPose]| Batch::Planets {
            light: PLANET_LIGHT,
            planets: poses.iter().map(instance).collect(),
            clip,
        };
        assert_eq!(
            frame.batches,
            [
                batch(Some(rect_of(c1)), &[pose(10.0, 10.0), pose(14.0, 10.0)]),
                batch(Some(rect_of(c2)), &[pose(30.0, 10.0)]),
                batch(None, &[pose(40.0, 10.0)]),
            ]
        );
    }

    #[test]
    fn a_clipped_planet_wholly_outside_its_clip_is_still_made_ready() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let clip = bounds(0.0, 0.0, 10.0, 10.0);
        let mut list = DrawList::new();
        list.clipped(
            clip,
            DrawCommand::Planet {
                planet: PlanetKey(2),
                pose: pose(40.0, 30.0),
            },
        );
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert!(is_baked(&renderer, PlanetKey(2)));
        assert_eq!(uploads(&gpu).len(), 1);
        assert_eq!(frame.batches[0].clip(), Some(rect_of(clip)));
        assert_eq!(frame.batches[0].draws(), 1);
        frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(uploads(&gpu).len(), 1, "once");
    }

    #[test]
    fn a_planet_clipped_to_no_area_draws_bakes_and_uploads_nothing() {
        let mut renderer = renderer();
        renderer.add_planet(PlanetKey(2), unbaked(77));
        let mut list = DrawList::new();
        let planet = DrawCommand::Planet {
            planet: PlanetKey(2),
            pose: pose(12.0, 12.0),
        };
        list.clipped(bounds(5.0, 0.0, 30.0, 0.0), planet);
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
        assert!(!is_baked(&renderer, PlanetKey(2)));
    }
}
