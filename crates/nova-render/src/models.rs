//! The [`Renderer`](crate::Renderer)'s models: the mesh each
//! [`ModelKey`] names ([`Renderer::add_model`](crate::Renderer::add_model)),
//! which of them the GPU holds, and which keys with no mesh have been
//! reported; and the [`Batch::Models`] a draw list's models become.
//!
//! Each mesh is numbered once, the first time its key is added, and
//! uploaded ([`Gpu::upload_mesh`]) the first frame that draws it. Adding
//! a key again replaces its mesh under the same number, uploaded again at
//! its next draw. Meshes are not atlas pages, so a new enlarger
//! (`Renderer::set_enlarger`) leaves them where they are.
//!
//! A mesh's textures are uploaded just before it
//! ([`Gpu::upload_model_texture`]), each once however many meshes hold
//! its pointer ([`Resident`]), and the mesh is
//! uploaded with the numbers they took. An untextured mesh uploads none.
//! Every texture a mesh holds is uploaded, whether or not a section
//! names it; a model's merged mesh
//! ([`LoadedModel::mesh`](crate::model_source::LoadedModel::mesh)) holds
//! only the textures a part samples, so a model whose textured materials
//! sit only on parts with no texture coordinates uploads none either.
//!
//! Consecutive models share one [`Batch::Models`], lit by
//! [`Light::STANDARD`], while they are clipped alike
//! ([`nova_view::DrawCommand::Clipped`]): the batch carries their clip
//! ([`Batch::clip`]), which the adapter scissors it to, so a model with
//! another clip, or none, opens a batch of its own, and a pass with it. A
//! model clipped to no area draws nothing and uploads nothing; one inside
//! a clip with area stays in its batch even when it lies wholly outside
//! the clip. Once a frame's batches are settled, each models batch's
//! instances get their depth bands ([`camera::band`]).
//!
//! A part ([`nova_view::DrawCommand::Mounted`]) mounts on the model drawn
//! just before it ([`Models::draw_model`], [`Models::draw_part`]): while
//! nothing else has been drawn since, each part joins that model's
//! instances in its batch, and the model and its parts are placed again
//! over the furthest any of them reaches ([`camera::reach`]), sharing one
//! band. A part is cut by its model's clip, whatever its own clip, as it
//! takes its model's alpha and colorize. Any other command
//! ([`Models::end_group`]), a model that draws nothing, a new layer's run
//! or a new frame, each starting with no batch, leaves the next part
//! nothing to mount on.
//!
//! A model drawn at an alpha ([`nova_view::DrawCommand::Model`]) carries
//! it on its instance, held as [`camera::alpha`] holds it, and its parts
//! take the same; one at an alpha that draws nothing draws nothing, its
//! parts with it. A model drawn colorized carries that on its instance
//! too ([`ModelInstance::colorize`], none for one that doesn't mix), and
//! its parts take the same; it changes no run, so an opaque colorized
//! model draws among the opaque ones. How the adapter draws a batch's
//! instances is settled
//! here too ([`draw_runs`]): each faded model with its parts a run of its
//! own, drawn depth first so only its nearest surface shows, and the
//! opaque models between them in runs drawn as before. Bands and the
//! batch's order are unchanged by it.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use nova_view::{Colorize, ModelKey, ModelPose, Mounting};

use crate::camera::{self, Light};
use crate::gpu::{Batch, Gpu, ModelInstance, ModelTextureId, NO_COLORIZE, Rect, colorize_instance};
use crate::model::{Mesh, MeshId};
use crate::model_texture::ModelTexture;
use crate::resident::Resident;
use crate::viewport::LogicalSize;

/// A mesh the renderer was given, and whether the GPU holds it as it is.
#[derive(Debug)]
struct Held {
    id: MeshId,
    mesh: Mesh,
    uploaded: bool,
}

/// The meshes the renderer was given, by key.
#[derive(Debug, Default)]
pub(crate) struct Models {
    meshes: HashMap<ModelKey, Held>,
    /// The number the next new key's mesh takes.
    next: u32,
    /// The keys with no mesh already reported.
    reported: HashSet<ModelKey>,
    /// The textures of the meshes uploaded so far, as the GPU holds them.
    textures: Resident<ModelTexture, ModelTextureId>,
    /// The model the parts drawn next mount on, while nothing else has
    /// been drawn since it.
    group: Option<Group>,
}

/// A model drawn, and the parts mounted on it so far.
#[derive(Debug)]
struct Group {
    pose: ModelPose,
    /// The model's mesh and its radius.
    hull: (MeshId, f32),
    /// Each part's mesh, its radius and where it sits.
    parts: Vec<(MeshId, f32, Mounting)>,
    /// Where its model's instance is in the last batch, its parts' after
    /// it.
    start: usize,
    /// The alpha the model and its parts draw at.
    alpha: f32,
    /// The colorize the model and its parts draw with
    /// ([`ModelInstance::colorize`]).
    colorize: [f32; 4],
}

impl Group {
    /// `instance` as this group's model or one of its parts draws: at its
    /// alpha and with its colorize.
    fn shown(&self, instance: ModelInstance) -> ModelInstance {
        ModelInstance {
            alpha: self.alpha,
            colorize: self.colorize,
            ..instance
        }
    }
}

impl Models {
    /// Draws model `key` at `pose` and `alpha`, mixed toward `colorize`
    /// when it has one that mixes ([`colorize_instance`]), and only inside
    /// `clip` when it has one, onto `batches` in the `logical` space: its
    /// instance, uploaded first through `gpu` if need be, and a key with no
    /// mesh pushed onto `missing` once. It is the model the parts drawn
    /// next mount on, at its alpha, with its colorize and in its clip, or,
    /// drawing nothing (an alpha [`camera::alpha`] refuses among the rest),
    /// leaves them nothing to mount on.
    pub(crate) fn draw_model(
        &mut self,
        key: ModelKey,
        (pose, alpha, colorize, clip): (&ModelPose, f32, Option<Colorize>, Option<Rect>),
        logical: LogicalSize,
        batches: &mut Vec<Batch>,
        gpu: &mut impl Gpu,
        missing: &mut Vec<ModelKey>,
    ) {
        self.group = None;
        let Some((mesh, radius)) = self.resident(key, gpu, missing) else {
            return;
        };
        let Some(alpha) = camera::alpha(alpha) else {
            return;
        };
        let Some(instance) = camera::instance(logical, pose, mesh, radius) else {
            return;
        };
        let mut group = Group {
            pose: *pose,
            hull: (mesh, radius),
            parts: Vec::new(),
            start: 0,
            alpha,
            colorize: colorize.map_or(NO_COLORIZE, colorize_instance),
        };
        push(batches, group.shown(instance), clip);
        if let Some(Batch::Models { models, .. }) = batches.last() {
            group.start = models.len() - 1;
            self.group = Some(group);
        }
    }

    /// Draws part `key` mounted as `mounting` says on the model drawn
    /// last ([`nova_view::DrawCommand::Mounted`]): the model and every
    /// part on it so far placed again over their new reach
    /// ([`camera::reach`]), the part after them, sharing their band. A key
    /// with no mesh is pushed onto `missing` once, whether or not there
    /// is a model to mount on. It draws nothing with no model to mount
    /// on, when its mesh is empty, or when `mounting` isn't finite.
    pub(crate) fn draw_part(
        &mut self,
        key: ModelKey,
        mounting: &Mounting,
        logical: LogicalSize,
        batches: &mut [Batch],
        gpu: &mut impl Gpu,
        missing: &mut Vec<ModelKey>,
    ) {
        let resident = self.resident(key, gpu, missing);
        // A group outlives no other command (`end_group`), and a new
        // layer's run starts with no batch, so a group's instances are the
        // last of the last batch.
        let (Some(group), Some((mesh, radius)), Some(Batch::Models { models, .. })) =
            (self.group.as_mut(), resident, batches.last_mut())
        else {
            return;
        };
        let parts = || (group.parts.iter().copied()).chain([(mesh, radius, *mounting)]);
        let reach = camera::reach(group.hull.1, parts().map(|(_, r, m)| (m, r)));
        let hull = camera::instance(logical, &group.pose, group.hull.0, reach);
        let placed: Option<Vec<ModelInstance>> = std::iter::once(hull)
            .chain(parts().map(|(mesh, _, m)| camera::part(logical, &group.pose, &m, mesh, reach)))
            .collect();
        let Some(placed) = placed else {
            return;
        };
        models.truncate(group.start);
        models.extend(placed.into_iter().map(|at| group.shown(at)));
        group.parts.push((mesh, radius, *mounting));
    }

    /// Leaves the parts drawn next nothing to mount on: something other
    /// than a model or a part was drawn. (A new frame starts with no batch,
    /// so a part first in it has nothing to mount on either.)
    pub(crate) fn end_group(&mut self) {
        self.group = None;
    }
    /// Gives `key` `mesh`, replacing any mesh it had: the replacement
    /// keeps the old one's number and is uploaded at its next draw.
    pub(crate) fn add(&mut self, key: ModelKey, mesh: Mesh) {
        if let Some(held) = self.meshes.get_mut(&key) {
            held.mesh = mesh;
            held.uploaded = false;
            return;
        }
        let id = MeshId(self.next);
        self.next += 1;
        self.meshes.insert(
            key,
            Held {
                id,
                mesh,
                uploaded: false,
            },
        );
    }

    /// The mesh `key` draws with and its radius, uploaded through `gpu`
    /// first if the GPU doesn't hold it yet, after each of its textures
    /// the GPU doesn't hold yet. `None` for a mesh with no
    /// triangles, which is never uploaded, and for a key with no mesh,
    /// pushed onto `missing` the first time it is asked for.
    pub(crate) fn resident(
        &mut self,
        key: ModelKey,
        gpu: &mut impl Gpu,
        missing: &mut Vec<ModelKey>,
    ) -> Option<(MeshId, f32)> {
        let Some(held) = self.meshes.get_mut(&key) else {
            if self.reported.insert(key) {
                missing.push(key);
            }
            return None;
        };
        if held.mesh.is_empty() {
            return None;
        }
        if !held.uploaded {
            let textures: Vec<ModelTextureId> = (held.mesh.textures().iter())
                .map(|texture| {
                    (self.textures).id(texture, |id, texture| {
                        gpu.upload_model_texture(id, texture);
                    })
                })
                .collect();
            gpu.upload_mesh(held.id, &held.mesh, &textures);
            held.uploaded = true;
        }
        Some((held.id, held.mesh.radius()))
    }
}

/// Appends `model`, drawn only inside `clip` when it has one, extending
/// the last batch when it is a models batch clipped alike.
pub(crate) fn push(batches: &mut Vec<Batch>, model: ModelInstance, clip: Option<Rect>) {
    match batches.last_mut() {
        Some(Batch::Models {
            models, clip: last, ..
        }) if *last == clip => models.push(model),
        _ => batches.push(Batch::Models {
            light: Light::STANDARD,
            models: vec![model],
            clip,
        }),
    }
}

/// A run of a models batch's instances the adapter draws in one go
/// ([`draw_runs`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelRun {
    /// The instances, by their place in the batch.
    pub(crate) instances: Range<usize>,
    /// Whether they are one faded model and its parts: drawn first into
    /// depth alone and then in colour where their depth is nearest, so
    /// only their nearest surface shows, at their alpha. An opaque run is
    /// drawn as every model always was, in colour and depth at once.
    pub(crate) faded: bool,
}

/// How the adapter draws `models`, one settled batch's instances: each
/// model drawn below an alpha of 1 with its parts after it
/// ([`ModelInstance::shares_band`]) a faded run of its own, and every
/// stretch of opaque models between them one opaque run. Each faded
/// model takes its own run, so its depth pass hides none of an earlier
/// faded model's colour.
pub(crate) fn draw_runs(models: &[ModelInstance]) -> Vec<ModelRun> {
    let mut runs: Vec<ModelRun> = Vec::new();
    for (at, model) in models.iter().enumerate() {
        let faded = model.alpha < 1.0;
        match runs.last_mut() {
            Some(run) if model.shares_band || (!faded && !run.faded) => run.instances.end = at + 1,
            _ => runs.push(ModelRun {
                instances: at..at + 1,
                faded,
            }),
        }
    }
    runs
}

/// Gives each models batch of `batches`, a frame's settled batches, its
/// instances' depth bands.
pub(crate) fn settle_bands(batches: &mut [Batch]) {
    for batch in batches {
        if let Batch::Models { models, .. } = batch {
            camera::band(models);
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::f32::consts::FRAC_PI_6;

    use nova_data::graphics::Image;
    use nova_view::geometry::Bounds;
    use nova_view::{
        Color, Colorize, DrawCommand, DrawList, ImageKey, ImageKind, ModelPose, Mounting,
        Orientation, Point,
    };

    use std::sync::Arc;

    use super::*;
    use crate::enlarge::NativeDetail;
    use crate::gpu::{Frame, ModelTextureId};
    use crate::images::{ImageError, ImageSource};
    use crate::model::Section;
    use crate::model_texture::ModelTexture;
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::viewport::{LogicalSize, Viewport};
    use crate::{Renderer, camera};

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
    const CUBE: ModelKey = ModelKey(1);

    /// 64x48 logical at scale 2, filling a 128x96 window.
    fn viewport() -> Viewport {
        Viewport::new(LOGICAL, (128, 96), 2.0)
    }

    fn cube() -> Mesh {
        Mesh::cube([Color::WHITE; 6])
    }

    /// A renderer whose key [`CUBE`] is the cube.
    fn renderer() -> Renderer<OneSprite> {
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_model(CUBE, cube());
        renderer
    }

    fn pose(x: f32, y: f32) -> ModelPose {
        ModelPose {
            center: Point::new(x, y),
            orientation: Orientation::about([1.0, 0.0, 0.0], FRAC_PI_6),
            scale: 10.0,
        }
    }

    /// The instance the camera makes for the cube, mesh 0, at `pose`.
    fn instance(pose: &ModelPose) -> ModelInstance {
        camera::instance(LOGICAL, pose, MeshId(0), cube().radius()).expect("drawable")
    }

    /// `instances` banded as one batch's.
    fn banded(mut instances: Vec<ModelInstance>) -> Vec<ModelInstance> {
        camera::band(&mut instances);
        instances
    }

    /// The frame `list` gives, drawn by `renderer` through `gpu`.
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

    fn uploads(gpu: &RecordingGpu) -> Vec<MeshId> {
        gpu.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::UploadMesh { mesh, .. } => Some(*mesh),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_model_uploads_its_mesh_once_then_draws_an_instance_each_frame() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0));

        let first = frame_of(&mut renderer, &list, &mut gpu);
        assert_eq!(
            gpu.calls[0],
            GpuCall::UploadMesh {
                mesh: MeshId(0),
                data: cube(),
                textures: Vec::new(),
            }
        );
        let second = frame_of(&mut renderer, &list, &mut gpu);

        assert_eq!(uploads(&gpu), [MeshId(0)], "once");
        let expected = vec![Batch::Models {
            light: Light::STANDARD,
            models: banded(vec![instance(&pose(12.0, 12.0))]),
            clip: None,
        }];
        assert_eq!(first.batches, expected);
        assert_eq!(second.batches, expected);
    }

    #[test]
    fn consecutive_models_share_one_models_batch() {
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .model(CUBE, pose(30.0, 20.0))
            .model(CUBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            frame.batches,
            [Batch::Models {
                light: Light::STANDARD,
                models: banded(vec![
                    instance(&pose(12.0, 12.0)),
                    instance(&pose(30.0, 20.0)),
                    instance(&pose(40.0, 10.0)),
                ]),
                clip: None,
            }]
        );
    }

    #[test]
    fn a_sprite_between_two_models_splits_their_batches_in_order() {
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .sprite(ImageKey::sprite(1, 0), Point::new(14.0, 12.0), Color::WHITE)
            .model(CUBE, pose(30.0, 20.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let kinds: Vec<&str> = frame.batches.iter().map(Batch::kind).collect();
        assert_eq!(kinds, ["models", "sprites", "models"]);
        // Each batch's one model has the whole range to itself.
        assert_eq!(
            frame.batches[2],
            Batch::Models {
                light: Light::STANDARD,
                models: banded(vec![instance(&pose(30.0, 20.0))]),
                clip: None,
            }
        );
    }

    #[test]
    fn a_world_model_marks_its_batch_world() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .world(|list| {
                list.model(CUBE, pose(12.0, 12.0));
            });
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world, [1]);
        assert_eq!(frame.batches[1].kind(), "models");
    }

    #[test]
    fn an_overlay_model_stays_overlay() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE);
        })
        .model(CUBE, pose(12.0, 12.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world, [0]);
        assert_eq!(frame.batches[1].kind(), "models");
    }

    #[test]
    fn a_models_batch_carries_the_standard_light() {
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let [Batch::Models { light, .. }] = frame.batches.as_slice() else {
            panic!("{:?}", frame.batches);
        };
        assert_eq!(*light, Light::STANDARD);
    }

    #[test]
    fn a_missing_model_draws_nothing_and_is_reported_once() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.model(ModelKey(9), pose(12.0, 12.0))
            .model(ModelKey(9), pose(20.0, 12.0))
            .model(ModelKey(8), pose(20.0, 12.0));

        let first = renderer.render(&list, &viewport(), &mut gpu);
        let second = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(first.missing_models, [ModelKey(9), ModelKey(8)]);
        assert_eq!(second.missing_models, []);
        assert!(gpu.submits().iter().all(|frame| frame.batches.is_empty()));
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn a_model_with_no_drawable_scale_or_orientation_draws_nothing() {
        let zero = Orientation {
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let mut list = DrawList::new();
        for scale in [0.0, -2.0, f32::NAN] {
            list.model(
                CUBE,
                ModelPose {
                    scale,
                    ..pose(1.0, 1.0)
                },
            );
        }
        list.model(
            CUBE,
            ModelPose {
                orientation: zero,
                ..pose(1.0, 1.0)
            },
        );
        let mut gpu = RecordingGpu::new();
        let report = renderer().render(&list, &viewport(), &mut gpu);
        assert_eq!(gpu.submits()[0].batches, []);
        assert_eq!(report.missing_models, []);
    }

    #[test]
    fn a_mesh_with_no_triangles_draws_nothing() {
        let mut renderer = renderer();
        let empty = Mesh::new(cube().vertices().to_vec(), Vec::new()).expect("empty");
        renderer.add_model(ModelKey(2), empty);
        let mut list = DrawList::new();
        list.model(ModelKey(2), pose(12.0, 12.0));
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(gpu.calls.len(), 1, "a submit alone: {:?}", gpu.calls);
        assert_eq!(gpu.submits()[0].batches, []);
        assert_eq!(report.missing_models, [], "not missing");
    }

    #[test]
    fn adding_a_model_again_uploads_it_again_at_its_next_draw() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0));
        renderer.render(&list, &viewport(), &mut gpu);

        let red = Mesh::cube([Color::rgba(255, 0, 0, 255); 6]);
        renderer.add_model(CUBE, red.clone());
        assert_eq!(uploads(&gpu), [MeshId(0)], "not until it is drawn");
        renderer.render(&list, &viewport(), &mut gpu);
        renderer.render(&list, &viewport(), &mut gpu);

        let calls: Vec<&GpuCall> = gpu
            .calls
            .iter()
            .filter(|call| matches!(call, GpuCall::UploadMesh { .. }))
            .collect();
        assert_eq!(
            calls,
            [
                &GpuCall::UploadMesh {
                    mesh: MeshId(0),
                    data: cube(),
                    textures: Vec::new(),
                },
                &GpuCall::UploadMesh {
                    mesh: MeshId(0),
                    data: red,
                    textures: Vec::new(),
                }
            ]
        );
    }

    #[test]
    fn each_new_key_takes_the_next_mesh_number() {
        let mut renderer = renderer();
        renderer.add_model(ModelKey(5), cube());
        let mut list = DrawList::new();
        list.model(ModelKey(5), pose(12.0, 12.0))
            .model(CUBE, pose(1.0, 1.0));
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(uploads(&gpu), [MeshId(1), MeshId(0)]);
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

    /// The clip of each of `frame`'s models batches, and its instances.
    fn clipped_models(frame: &Frame) -> Vec<(Option<crate::Rect>, Vec<ModelInstance>)> {
        (frame.batches.iter())
            .map(|batch| match batch {
                Batch::Models { models, clip, .. } => (*clip, models.clone()),
                other => panic!("a models batch: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_clipped_model_carries_its_clip_on_its_batch() {
        let clip = bounds(0.0, 0.0, 13.0, 13.0);
        let mut clipped = DrawList::new();
        clipped.clipped(clip, DrawCommand::model(CUBE, pose(12.0, 12.0)));
        let mut plain = DrawList::new();
        plain.model(CUBE, pose(12.0, 12.0));
        let a = frame_of(&mut renderer(), &clipped, &mut RecordingGpu::new());
        let b = frame_of(&mut renderer(), &plain, &mut RecordingGpu::new());
        assert_eq!(models_of(&a), models_of(&b), "the instances are uncut");
        assert_eq!(
            clipped_models(&a),
            [(
                Some(crate::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 13.0,
                    h: 13.0
                }),
                models_of(&b)
            )]
        );
        assert_eq!(clipped_models(&b)[0].0, None);
    }

    #[test]
    fn models_clipped_alike_share_a_batch_and_another_clip_splits_it() {
        let (c1, c2) = (bounds(0.0, 0.0, 20.0, 20.0), bounds(10.0, 0.0, 40.0, 30.0));
        let mut list = DrawList::new();
        list.clipped(c1, DrawCommand::model(CUBE, pose(12.0, 12.0)))
            .clipped(c1, DrawCommand::model(CUBE, pose(14.0, 12.0)))
            .clipped(c2, DrawCommand::model(CUBE, pose(30.0, 20.0)))
            .model(CUBE, pose(40.0, 10.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            clipped_models(&frame),
            [
                (
                    Some(rect_of(c1)),
                    banded(vec![
                        instance(&pose(12.0, 12.0)),
                        instance(&pose(14.0, 12.0))
                    ])
                ),
                (Some(rect_of(c2)), banded(vec![instance(&pose(30.0, 20.0))])),
                (None, banded(vec![instance(&pose(40.0, 10.0))])),
            ]
        );
    }

    #[test]
    fn nested_clips_meet_on_a_models_batch() {
        let (outer, inner) = (bounds(0.0, 0.0, 20.0, 20.0), bounds(10.0, 5.0, 40.0, 30.0));
        let mut list = DrawList::new();
        list.clipped(
            outer,
            DrawCommand::Clipped {
                clip: inner,
                command: Box::new(DrawCommand::model(CUBE, pose(12.0, 12.0))),
            },
        );
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let met = rect_of(bounds(10.0, 5.0, 20.0, 20.0));
        assert_eq!(clipped_models(&frame)[0].0, Some(met));
    }

    #[test]
    fn a_model_clipped_to_no_area_draws_and_uploads_nothing() {
        let model = || DrawCommand::model(CUBE, pose(12.0, 12.0));
        // A clip with no width, and two clips that meet nowhere.
        let mut list = DrawList::new();
        list.clipped(bounds(5.0, 0.0, 5.0, 13.0), model()).clipped(
            bounds(0.0, 0.0, 4.0, 4.0),
            DrawCommand::Clipped {
                clip: bounds(8.0, 8.0, 12.0, 12.0),
                command: Box::new(model()),
            },
        );
        let mut gpu = RecordingGpu::new();
        let frame = frame_of(&mut renderer(), &list, &mut gpu);
        assert_eq!(frame.batches, []);
        assert_eq!(uploads(&gpu), []);
    }

    #[test]
    fn a_clipped_faded_model_keeps_its_runs() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let clip = bounds(0.0, 0.0, 30.0, 30.0);
        let draw = |list: &mut DrawList, clip: Option<Bounds>| {
            let commands = [
                DrawCommand::model(CUBE, pose(12.0, 12.0)),
                DrawCommand::faded_model(CUBE, pose(20.0, 12.0), 0.5),
                DrawCommand::Mounted {
                    model: PART,
                    mounting: part,
                },
                DrawCommand::model(CUBE, pose(30.0, 20.0)),
            ];
            for command in commands {
                match clip {
                    Some(clip) => list.clipped(clip, command),
                    None => list.push(command),
                };
            }
        };
        let (mut clipped, mut plain) = (DrawList::new(), DrawList::new());
        draw(&mut clipped, Some(clip));
        draw(&mut plain, None);
        let a = frame_of(&mut with_part(), &clipped, &mut RecordingGpu::new());
        let b = frame_of(&mut with_part(), &plain, &mut RecordingGpu::new());
        assert_eq!(models_of(&a), models_of(&b));
        assert_eq!(
            draw_runs(&models_of(&a)),
            [run(0..1, false), run(1..3, true), run(3..4, false)]
        );
    }

    #[test]
    fn a_clipped_world_model_is_world_and_keeps_its_clip() {
        let (world, clip) = (bounds(0.0, 0.0, 50.0, 40.0), bounds(4.0, 4.0, 20.0, 20.0));
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .world_in(world, |list| {
                list.clipped(clip, DrawCommand::model(CUBE, pose(12.0, 12.0)));
            });
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world, [1]);
        assert_eq!(frame.batches[1].clip(), Some(rect_of(clip)));
        assert_eq!(frame.world_clip, Some(rect_of(world)));
    }

    #[test]
    fn a_world_tail_settles_a_models_run() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.model(CUBE, pose(12.0, 12.0))
                .start_world_tail()
                .model(CUBE, pose(30.0, 20.0));
        });
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(frame.world_tail, Some(1));
        let one = |pose| Batch::Models {
            light: Light::STANDARD,
            models: banded(vec![instance(&pose)]),
            clip: None,
        };
        assert_eq!(
            frame.batches,
            [one(pose(12.0, 12.0)), one(pose(30.0, 20.0))]
        );
    }

    #[test]
    fn set_enlarger_keeps_meshes_resident() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .model(CUBE, pose(12.0, 12.0));
        renderer.render(&list, &viewport(), &mut gpu);
        renderer.set_enlarger(Box::new(NativeDetail));
        let frame = frame_of(&mut renderer, &list, &mut gpu);
        assert!(
            gpu.calls.contains(&GpuCall::ClearPages),
            "the atlas restarted"
        );
        assert_eq!(uploads(&gpu), [MeshId(0)], "the mesh stays");
        assert_eq!(frame.batches[1].kind(), "models");
    }

    fn one_texel(shade: u8) -> Arc<ModelTexture> {
        Arc::new(ModelTexture::new(1, 1, vec![shade, shade, shade, 255]).expect("one texel"))
    }

    /// The cube holding `textures`, a face on each in order and the rest
    /// of it untextured.
    fn skinned(textures: &[Arc<ModelTexture>]) -> Mesh {
        let cube = cube();
        let faces = textures.len() as u32;
        let mut sections: Vec<Section> = (0..faces)
            .map(|face| Section::new(face * 6..face * 6 + 6, Some(face as usize)))
            .collect();
        sections.push(Section::new(faces * 6..36, None));
        Mesh::textured(
            cube.vertices().to_vec(),
            cube.indices().to_vec(),
            textures.to_vec(),
            sections,
        )
        .expect("the cube in sections")
    }

    /// The texture and mesh uploads `gpu` received, in order: each
    /// texture's id, and each mesh's id with its textures' ids.
    fn model_uploads(
        gpu: &RecordingGpu,
    ) -> Vec<Result<ModelTextureId, (MeshId, Vec<ModelTextureId>)>> {
        gpu.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::UploadModelTexture { texture, .. } => Some(Ok(*texture)),
                GpuCall::UploadMesh { mesh, textures, .. } => Some(Err((*mesh, textures.clone()))),
                _ => None,
            })
            .collect()
    }

    /// A list drawing each of `keys` once.
    fn drawing(keys: &[ModelKey]) -> DrawList {
        let mut list = DrawList::new();
        for key in keys {
            list.model(*key, pose(12.0, 12.0));
        }
        list
    }

    #[test]
    fn a_textured_model_uploads_its_texture_then_its_mesh_naming_it_once() {
        let texture = one_texel(9);
        let mesh = skinned(std::slice::from_ref(&texture));
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_model(CUBE, mesh.clone());
        let mut gpu = RecordingGpu::new();
        let list = drawing(&[CUBE]);

        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            gpu.calls[..2],
            [
                GpuCall::UploadModelTexture {
                    texture: ModelTextureId(0),
                    data: (*texture).clone(),
                },
                GpuCall::UploadMesh {
                    mesh: MeshId(0),
                    data: mesh,
                    textures: vec![ModelTextureId(0)],
                },
            ]
        );
        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            model_uploads(&gpu),
            [
                Ok(ModelTextureId(0)),
                Err((MeshId(0), vec![ModelTextureId(0)]))
            ],
            "neither again the next frame"
        );
    }

    #[test]
    fn a_mesh_names_each_of_its_textures_in_its_own_order() {
        let (first, second) = (one_texel(1), one_texel(2));
        let mut renderer = Renderer::new(OneSprite);
        // Mesh 0 holds the second texture alone, so it is numbered first.
        renderer.add_model(CUBE, skinned(std::slice::from_ref(&second)));
        renderer.add_model(PART, skinned(&[first, second]));
        let mut gpu = RecordingGpu::new();
        renderer.render(&drawing(&[CUBE, PART]), &viewport(), &mut gpu);
        assert_eq!(
            model_uploads(&gpu),
            [
                Ok(ModelTextureId(0)),
                Err((MeshId(0), vec![ModelTextureId(0)])),
                Ok(ModelTextureId(1)),
                Err((MeshId(1), vec![ModelTextureId(1), ModelTextureId(0)])),
            ]
        );
    }

    #[test]
    fn two_keys_sharing_a_texture_upload_it_once_and_both_meshes_name_it() {
        let texture = one_texel(9);
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_model(CUBE, skinned(std::slice::from_ref(&texture)));
        renderer.add_model(PART, skinned(std::slice::from_ref(&texture)));
        let mut gpu = RecordingGpu::new();
        renderer.render(&drawing(&[CUBE, PART]), &viewport(), &mut gpu);
        assert_eq!(
            model_uploads(&gpu),
            [
                Ok(ModelTextureId(0)),
                Err((MeshId(0), vec![ModelTextureId(0)])),
                Err((MeshId(1), vec![ModelTextureId(0)])),
            ]
        );
    }

    #[test]
    fn a_key_added_again_uploads_its_mesh_again_and_not_the_texture_it_still_shares() {
        let (kept, new) = (one_texel(1), one_texel(2));
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_model(CUBE, skinned(std::slice::from_ref(&kept)));
        let mut gpu = RecordingGpu::new();
        let list = drawing(&[CUBE]);
        renderer.render(&list, &viewport(), &mut gpu);

        renderer.add_model(CUBE, skinned(&[kept, new]));
        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            model_uploads(&gpu),
            [
                Ok(ModelTextureId(0)),
                Err((MeshId(0), vec![ModelTextureId(0)])),
                Ok(ModelTextureId(1)),
                Err((MeshId(0), vec![ModelTextureId(0), ModelTextureId(1)])),
            ]
        );
    }

    #[test]
    fn an_untextured_model_uploads_no_texture_and_names_none() {
        let mut gpu = RecordingGpu::new();
        renderer().render(&drawing(&[CUBE]), &viewport(), &mut gpu);
        assert_eq!(model_uploads(&gpu), [Err((MeshId(0), Vec::new()))]);
    }

    #[test]
    fn a_textured_model_that_is_never_drawn_uploads_nothing() {
        let mut renderer = renderer();
        renderer.add_model(PART, skinned(&[one_texel(3)]));
        let mut gpu = RecordingGpu::new();
        renderer.render(&drawing(&[CUBE]), &viewport(), &mut gpu);
        assert_eq!(model_uploads(&gpu), [Err((MeshId(0), Vec::new()))]);
    }

    #[test]
    fn set_enlarger_keeps_model_textures_resident() {
        let mut renderer = Renderer::new(OneSprite);
        renderer.add_model(CUBE, skinned(&[one_texel(9)]));
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .model(CUBE, pose(12.0, 12.0));
        renderer.render(&list, &viewport(), &mut gpu);
        renderer.set_enlarger(Box::new(NativeDetail));
        renderer.render(&list, &viewport(), &mut gpu);
        assert!(
            gpu.calls.contains(&GpuCall::ClearPages),
            "the atlas restarted"
        );
        assert_eq!(
            model_uploads(&gpu),
            [
                Ok(ModelTextureId(0)),
                Err((MeshId(0), vec![ModelTextureId(0)]))
            ],
            "the texture and the mesh stay"
        );
    }

    const PART: ModelKey = ModelKey(3);

    /// A renderer whose [`CUBE`] is the cube, mesh 0, and whose [`PART`]
    /// is the cube too, mesh 1.
    fn with_part() -> Renderer<OneSprite> {
        let mut renderer = renderer();
        renderer.add_model(PART, cube());
        renderer
    }

    fn on(at: [f32; 3], turn: f32) -> Mounting {
        Mounting { at, turn }
    }

    /// The hull at `pose` and `parts` on it, as the camera places them:
    /// all over their group's reach, in the order drawn.
    fn group(pose: &ModelPose, parts: &[Mounting]) -> Vec<ModelInstance> {
        let radius = cube().radius();
        let reach = camera::reach(radius, parts.iter().map(|&m| (m, radius)));
        let hull = camera::instance(LOGICAL, pose, MeshId(0), reach).expect("drawable");
        let parts = parts
            .iter()
            .map(|m| camera::part(LOGICAL, pose, m, MeshId(1), reach).expect("drawable"));
        std::iter::once(hull).chain(parts).collect()
    }

    fn models_of(frame: &Frame) -> Vec<ModelInstance> {
        match frame.batches.as_slice() {
            [Batch::Models { models, .. }] => models.clone(),
            other => panic!("one models batch: {other:?}"),
        }
    }

    #[test]
    fn a_hull_and_its_parts_take_one_band_over_their_reach_and_the_next_hull_another() {
        let low = on([0.0, 0.0, 0.5], 0.0);
        // Its turret stands high: past the cube's own radius.
        let high = on([0.2, -0.3, 1.5], 1.0);
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .mounted(PART, low)
            .mounted(PART, high)
            .model(CUBE, pose(30.0, 20.0));
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        let mut expected = group(&pose(12.0, 12.0), &[low, high]);
        expected.push(instance(&pose(30.0, 20.0)));
        assert_eq!(models_of(&frame), banded(expected.clone()));
        // The hull's depth is rescaled for its high part.
        assert_ne!(expected[0], instance(&pose(12.0, 12.0)));
    }

    #[test]
    fn each_part_follows_the_model_not_the_part_before_it() {
        let a = on([1.0, 0.0, 0.0], 0.5);
        let b = on([0.0, 1.0, 0.0], -0.5);
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .mounted(PART, a)
            .mounted(PART, b);
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        assert_eq!(models_of(&frame), banded(group(&pose(12.0, 12.0), &[a, b])));
    }

    /// The batches `list` draws, with `renderer`.
    fn batches(mut renderer: Renderer<OneSprite>, list: &DrawList) -> Vec<Batch> {
        frame_of(&mut renderer, list, &mut RecordingGpu::new()).batches
    }

    #[test]
    fn a_part_with_no_model_just_before_it_draws_nothing() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        // First in the list.
        let mut first = DrawList::new();
        first.mounted(PART, part);
        assert_eq!(batches(with_part(), &first), []);
        // After a sprite after its model.
        let mut after_sprite = DrawList::new();
        after_sprite
            .model(CUBE, pose(12.0, 12.0))
            .sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE)
            .mounted(PART, part);
        let kinds: Vec<&str> = batches(with_part(), &after_sprite)
            .iter()
            .map(Batch::kind)
            .collect();
        assert_eq!(kinds, ["models", "sprites"]);
        // After a command that draws nothing at all: an empty text.
        let mut after_nothing = DrawList::new();
        after_nothing
            .model(CUBE, pose(12.0, 12.0))
            .text("", Point::new(1.0, 1.0), 9.0, None, Color::WHITE)
            .mounted(PART, part);
        let mut alone = DrawList::new();
        alone.model(CUBE, pose(12.0, 12.0));
        assert_eq!(
            batches(with_part(), &after_nothing),
            batches(with_part(), &alone)
        );
    }

    #[test]
    fn a_part_after_a_model_that_draws_nothing_draws_nothing() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let mut renderer = with_part();
        let empty = Mesh::new(cube().vertices().to_vec(), Vec::new()).expect("empty");
        renderer.add_model(ModelKey(2), empty);
        let mut list = DrawList::new();
        list.model(ModelKey(9), pose(12.0, 12.0))
            .mounted(PART, part)
            .model(ModelKey(2), pose(12.0, 12.0))
            .mounted(PART, part)
            .model(
                CUBE,
                ModelPose {
                    scale: f32::NAN,
                    ..pose(1.0, 1.0)
                },
            )
            .mounted(PART, part);
        let report = renderer.render(&list, &viewport(), &mut RecordingGpu::new());
        assert_eq!(report.missing_models, [ModelKey(9)]);
        assert_eq!(batches(renderer, &list), []);
    }

    #[test]
    fn a_part_whose_own_key_has_no_mesh_draws_nothing_and_is_reported_once() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .mounted(ModelKey(9), on([0.0; 3], 0.0))
            .mounted(ModelKey(9), on([0.0; 3], 0.0));
        let first = renderer.render(&list, &viewport(), &mut gpu);
        let second = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(first.missing_models, [ModelKey(9)]);
        assert_eq!(second.missing_models, []);
        assert_eq!(
            gpu.submits()[0].batches,
            [Batch::Models {
                light: Light::STANDARD,
                models: banded(vec![instance(&pose(12.0, 12.0))]),
                clip: None,
            }]
        );
        // With no model before it, it is still reported.
        let mut alone = DrawList::new();
        alone.mounted(ModelKey(8), on([0.0; 3], 0.0));
        let report = renderer.render(&alone, &viewport(), &mut gpu);
        assert_eq!(report.missing_models, [ModelKey(8)]);
    }

    #[test]
    fn a_part_placed_or_turned_by_what_is_not_finite_draws_nothing() {
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .mounted(PART, on([f32::NAN, 0.0, 0.0], 0.0))
            .mounted(PART, on([0.0, 0.0, f32::INFINITY], 0.0))
            .mounted(PART, on([0.0; 3], f32::NAN));
        let mut alone = DrawList::new();
        alone.model(CUBE, pose(12.0, 12.0));
        assert_eq!(batches(with_part(), &list), batches(with_part(), &alone));
        // A good part after them still mounts.
        let good = on([0.0, 0.0, 0.5], 0.0);
        list.mounted(PART, good);
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        assert_eq!(models_of(&frame), banded(group(&pose(12.0, 12.0), &[good])));
    }

    #[test]
    fn a_part_with_an_empty_mesh_draws_nothing() {
        let mut renderer = renderer();
        let empty = Mesh::new(cube().vertices().to_vec(), Vec::new()).expect("empty");
        renderer.add_model(PART, empty);
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .mounted(PART, on([0.0, 0.0, 3.0], 0.0));
        let mut alone = DrawList::new();
        alone.model(CUBE, pose(12.0, 12.0));
        assert_eq!(batches(renderer, &list), batches(self::renderer(), &alone));
    }

    #[test]
    fn a_part_takes_its_models_clip() {
        let clip = bounds(0.0, 0.0, 13.0, 13.0);
        let part = on([0.3, 0.0, 0.5], 0.4);
        let mounted = || DrawCommand::Mounted {
            model: PART,
            mounting: part,
        };
        let expected = [(
            Some(rect_of(clip)),
            banded(group(&pose(12.0, 12.0), &[part])),
        )];
        // The part clipped alike, unclipped, or clipped to another rect
        // that overlaps its model's: it joins the model's batch, in its
        // band, cut by the model's clip.
        let other = bounds(5.0, 5.0, 30.0, 30.0);
        for (at, part_clip) in [Some(clip), None, Some(other)].into_iter().enumerate() {
            let mut list = DrawList::new();
            list.clipped(clip, DrawCommand::model(CUBE, pose(12.0, 12.0)));
            match part_clip {
                Some(part_clip) => list.clipped(part_clip, mounted()),
                None => list.push(mounted()),
            };
            let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
            let models = clipped_models(&frame);
            assert_eq!(models, expected, "part {at}");
            assert!(models[0].1[1].shares_band, "part {at}");
        }
    }

    #[test]
    fn a_part_in_a_layer_other_than_its_model_s_draws_nothing() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.model(CUBE, pose(12.0, 12.0));
        })
        .mounted(PART, on([0.0, 0.0, 0.5], 0.0));
        let mut alone = DrawList::new();
        alone.world(|list| {
            list.model(CUBE, pose(12.0, 12.0));
        });
        assert_eq!(batches(with_part(), &list), batches(with_part(), &alone));
    }

    #[test]
    fn a_new_frame_starts_with_no_model_to_mount_on() {
        let mut renderer = with_part();
        let mut gpu = RecordingGpu::new();
        let mut hull = DrawList::new();
        hull.model(CUBE, pose(12.0, 12.0));
        renderer.render(&hull, &viewport(), &mut gpu);
        let mut part = DrawList::new();
        part.mounted(PART, on([0.0; 3], 0.0));
        assert_eq!(frame_of(&mut renderer, &part, &mut gpu).batches, []);
    }

    /// `instances` each at `alpha`.
    fn at_alpha(instances: Vec<ModelInstance>, alpha: f32) -> Vec<ModelInstance> {
        (instances.into_iter())
            .map(|instance| ModelInstance { alpha, ..instance })
            .collect()
    }

    #[test]
    fn a_faded_model_and_its_parts_carry_its_alpha() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let mut list = DrawList::new();
        list.faded_model(CUBE, pose(12.0, 12.0), 0.5)
            .mounted(PART, part)
            .model(CUBE, pose(30.0, 20.0));
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        let mut expected = at_alpha(group(&pose(12.0, 12.0), &[part]), 0.5);
        expected.push(instance(&pose(30.0, 20.0)));
        assert_eq!(models_of(&frame), banded(expected));
    }

    #[test]
    fn a_model_faded_to_nothing_or_no_number_draws_nothing_nor_its_parts() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let mut list = DrawList::new();
        for alpha in [0.0, -0.5, f32::NAN, f32::INFINITY] {
            list.faded_model(CUBE, pose(12.0, 12.0), alpha)
                .mounted(PART, part);
        }
        assert_eq!(batches(with_part(), &list), []);
    }

    #[test]
    fn a_model_faded_past_1_draws_whole() {
        let mut over = DrawList::new();
        over.faded_model(CUBE, pose(12.0, 12.0), 1.5);
        let mut whole = DrawList::new();
        whole.model(CUBE, pose(12.0, 12.0));
        assert_eq!(batches(renderer(), &over), batches(renderer(), &whole));
    }

    fn run(instances: std::ops::Range<usize>, faded: bool) -> ModelRun {
        ModelRun { instances, faded }
    }

    #[test]
    fn a_faded_hull_and_its_parts_draw_in_a_run_of_their_own_between_opaque_runs() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .faded_model(CUBE, pose(20.0, 12.0), 0.5)
            .mounted(PART, part)
            .model(CUBE, pose(30.0, 20.0))
            .mounted(PART, part)
            .model(CUBE, pose(40.0, 20.0));
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        assert_eq!(
            draw_runs(&models_of(&frame)),
            [run(0..1, false), run(1..3, true), run(3..6, false)]
        );
    }

    #[test]
    fn each_faded_hull_takes_its_own_run_and_opaque_models_one_together() {
        let mut list = DrawList::new();
        list.faded_model(CUBE, pose(12.0, 12.0), 0.5)
            .faded_model(CUBE, pose(20.0, 12.0), 0.25);
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        assert_eq!(
            draw_runs(&models_of(&frame)),
            [run(0..1, true), run(1..2, true)]
        );
        let mut opaque = DrawList::new();
        opaque
            .model(CUBE, pose(12.0, 12.0))
            .model(CUBE, pose(20.0, 12.0))
            .model(CUBE, pose(30.0, 12.0));
        let frame = frame_of(&mut renderer(), &opaque, &mut RecordingGpu::new());
        assert_eq!(draw_runs(&models_of(&frame)), [run(0..3, false)]);
        assert_eq!(draw_runs(&[]), []);
    }

    const HALF_BLUE: Colorize = Colorize {
        level: 16,
        rgb5: [0, 4, 31],
    };

    /// `instances` each colorized `colorize`.
    fn colorized(instances: Vec<ModelInstance>, colorize: Colorize) -> Vec<ModelInstance> {
        (instances.into_iter())
            .map(|instance| ModelInstance {
                colorize: colorize_instance(colorize),
                ..instance
            })
            .collect()
    }

    /// The cube at `pose` at `alpha`, colorized [`HALF_BLUE`].
    fn blue(pose: ModelPose, alpha: f32) -> DrawCommand {
        DrawCommand::faded_model(CUBE, pose, alpha).with_colorize(Some(HALF_BLUE))
    }

    #[test]
    fn a_colorized_model_and_its_parts_carry_its_colorize() {
        let (low, high) = (on([0.0, 0.0, 0.5], 0.0), on([0.2, -0.3, 1.5], 1.0));
        let mut list = DrawList::new();
        list.push(blue(pose(12.0, 12.0), 1.0))
            .mounted(PART, low)
            .mounted(PART, high)
            .model(CUBE, pose(30.0, 20.0))
            .mounted(PART, low);
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        let mut expected = colorized(group(&pose(12.0, 12.0), &[low, high]), HALF_BLUE);
        assert!(
            expected
                .iter()
                .all(|instance| instance.colorize == [0.0, 4.0, 31.0, 16.0])
        );
        // The plain hull after it, and its part, take none.
        let plain = group(&pose(30.0, 20.0), &[low]);
        assert!(
            plain
                .iter()
                .all(|instance| instance.colorize == NO_COLORIZE)
        );
        expected.extend(plain);
        assert_eq!(models_of(&frame), banded(expected));
    }

    #[test]
    fn a_colorize_that_does_not_mix_draws_as_none() {
        let mut plain = DrawList::new();
        plain.model(CUBE, pose(12.0, 12.0));
        for level in [0, 33] {
            // Built by hand: `with_colorize` would drop it.
            let mut list = DrawList::new();
            list.push(DrawCommand::Model {
                model: CUBE,
                pose: pose(12.0, 12.0),
                alpha: 1.0,
                colorize: Some(Colorize { level, ..HALF_BLUE }),
            });
            assert_eq!(
                batches(renderer(), &list),
                batches(renderer(), &plain),
                "{level}"
            );
        }
    }

    #[test]
    fn an_opaque_colorized_model_draws_in_the_opaque_run() {
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .push(blue(pose(20.0, 12.0), 1.0))
            .model(CUBE, pose(30.0, 12.0));
        let frame = frame_of(&mut renderer(), &list, &mut RecordingGpu::new());
        let models = models_of(&frame);
        assert_eq!(models[1].colorize, colorize_instance(HALF_BLUE));
        assert_eq!(draw_runs(&models), [run(0..3, false)]);
    }

    #[test]
    fn a_faded_colorized_model_keeps_its_faded_run() {
        let part = on([0.0, 0.0, 0.5], 0.0);
        let mut list = DrawList::new();
        list.model(CUBE, pose(12.0, 12.0))
            .push(blue(pose(20.0, 12.0), 0.5))
            .mounted(PART, part)
            .model(CUBE, pose(30.0, 20.0));
        let frame = frame_of(&mut with_part(), &list, &mut RecordingGpu::new());
        let models = models_of(&frame);
        let mut expected = vec![instance(&pose(12.0, 12.0))];
        expected.extend(colorized(
            at_alpha(group(&pose(20.0, 12.0), &[part]), 0.5),
            HALF_BLUE,
        ));
        expected.push(instance(&pose(30.0, 20.0)));
        assert_eq!(models, banded(expected));
        assert_eq!(
            draw_runs(&models),
            [run(0..1, false), run(1..3, true), run(3..4, false)]
        );
    }

    #[test]
    fn a_list_with_no_models_uploads_no_mesh() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE);
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        assert_eq!(uploads(&gpu), []);
    }
}
