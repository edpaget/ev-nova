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
//! Consecutive models share one [`Batch::Models`], lit by
//! [`Light::STANDARD`]. Once a frame's batches are settled, each models
//! batch's instances get their depth bands ([`camera::band`]).

use std::collections::{HashMap, HashSet};

use nova_view::ModelKey;

use crate::camera::{self, Light};
use crate::gpu::{Batch, Gpu, ModelInstance};
use crate::model::{Mesh, MeshId};

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
}

impl Models {
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
    /// first if the GPU doesn't hold it yet. `None` for a mesh with no
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
            gpu.upload_mesh(held.id, &held.mesh);
            held.uploaded = true;
        }
        Some((held.id, held.mesh.radius()))
    }
}

/// Appends `model`, extending the last batch when it is a models batch.
pub(crate) fn push(batches: &mut Vec<Batch>, model: ModelInstance) {
    if let Some(Batch::Models { models, .. }) = batches.last_mut() {
        models.push(model);
    } else {
        batches.push(Batch::Models {
            light: Light::STANDARD,
            models: vec![model],
        });
    }
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
        Color, DrawCommand, DrawList, ImageKey, ImageKind, ModelPose, Orientation, Point,
    };

    use super::*;
    use crate::enlarge::NativeDetail;
    use crate::gpu::Frame;
    use crate::images::{ImageError, ImageSource};
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
                data: cube()
            }
        );
        let second = frame_of(&mut renderer, &list, &mut gpu);

        assert_eq!(uploads(&gpu), [MeshId(0)], "once");
        let expected = vec![Batch::Models {
            light: Light::STANDARD,
            models: banded(vec![instance(&pose(12.0, 12.0))]),
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
                    data: cube()
                },
                &GpuCall::UploadMesh {
                    mesh: MeshId(0),
                    data: red
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

    #[test]
    fn a_clipped_model_draws_uncut() {
        let clip = Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(13.0, 13.0),
        };
        let mut clipped = DrawList::new();
        clipped.clipped(
            clip,
            DrawCommand::Model {
                model: CUBE,
                pose: pose(12.0, 12.0),
            },
        );
        let mut plain = DrawList::new();
        plain.model(CUBE, pose(12.0, 12.0));
        let a = frame_of(&mut renderer(), &clipped, &mut RecordingGpu::new());
        let b = frame_of(&mut renderer(), &plain, &mut RecordingGpu::new());
        assert_eq!(a.batches, b.batches);
        assert_eq!(a.batches.len(), 1);
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

    #[test]
    fn a_list_with_no_models_uploads_no_mesh() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(1, 0), Point::new(4.0, 4.0), Color::WHITE);
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        assert_eq!(uploads(&gpu), []);
    }
}
