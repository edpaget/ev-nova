//! The wgpu side of [`Batch::Models`](crate::Batch::Models): the meshes'
//! buffers ([`crate::Gpu::upload_mesh`]), their textures
//! ([`crate::Gpu::upload_model_texture`]), the depth textures, the model
//! pipeline and its draws. It decides nothing: each instance's transforms
//! and the light come from the core ([`crate::camera`]), and which
//! triangles take which texture from the mesh's sections
//! ([`crate::model::Section`]).
//!
//! The pipeline draws into [`SCENE_FORMAT`] with a [`DEPTH_FORMAT`]
//! attachment, compares `Less` and writes depth, culls nothing (the depth
//! test hides a model's far faces) and blends as a normal sprite does.
//! `models.wgsl` mirrors [`crate::camera::fragment`]: the vertex colour
//! times the texel, lit by [`crate::camera::shade`]'s formula unless the
//! vertex is unlit.
//!
//! Each texture is one bind group, a texture with its mip chain and the
//! one sampler (repeating, trilinear), made when the texture is uploaded.
//! A section with no texture binds a 1 by 1 opaque white texture made
//! with the pipeline, so one pipeline and one shader path draw both, and
//! the product with white is the vertex colour exactly. A mesh is one
//! indexed draw per instance for each of its sections that has
//! triangles. Nothing is made on a frame.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::planets::texture;
use super::renderer::{SCENE_FORMAT, pipeline_layout};
use crate::camera::Light;
use crate::gpu::{ModelInstance, ModelTextureId, PlanetImage};
use crate::model::{Mesh, MeshId, Vertex};
use crate::model_texture::ModelTexture;

/// The depth attachment's format.
pub(super) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One mesh vertex as the model shader reads it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct MeshVertex {
    /// Its position in model units.
    pub position: [f32; 3],
    /// Its normal on the model's axes.
    pub normal: [f32; 3],
    /// Its straight RGBA colour.
    pub color: [f32; 4],
    /// Its texture coordinates.
    pub uv: [f32; 2],
    /// 1 when it is unlit, 0 when it is lit.
    pub unlit: f32,
}

impl From<&Vertex> for MeshVertex {
    fn from(vertex: &Vertex) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
            color: vertex.color,
            uv: vertex.uv,
            unlit: f32::from(u8::from(vertex.unlit)),
        }
    }
}

/// One model as the model shader reads it, per instance: its transforms
/// by columns (the normal's padded to four floats a column) and its
/// batch's light, the direction then the ambient.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct ModelData {
    /// `clip_from_model`'s columns.
    pub clip: [[f32; 4]; 4],
    /// `normal_from_model`'s columns, each with a 0 after.
    pub normal: [[f32; 4]; 3],
    /// The light's direction and its ambient.
    pub light: [f32; 4],
}

impl ModelData {
    /// `model` lit by `light`.
    pub(super) fn new(model: &ModelInstance, light: Light) -> Self {
        let [x, y, z] = light.direction;
        Self {
            clip: model.clip_from_model,
            normal: model.normal_from_model.map(|[a, b, c]| [a, b, c, 0.0]),
            light: [x, y, z, light.ambient],
        }
    }
}

/// One draw of a mesh: a run of its indices, and the texture it samples,
/// if any.
type SectionDraw = (Range<u32>, Option<ModelTextureId>);

/// The draws mesh `data` takes, its textures uploaded as `textures` (one
/// id for each of the mesh's, in its order): each of its sections that has
/// triangles, with the id of the texture it names. A section naming a
/// texture `textures` has no id for draws untextured.
fn draws(data: &Mesh, textures: &[ModelTextureId]) -> Vec<SectionDraw> {
    data.sections()
        .iter()
        .filter(|section| !section.indices.is_empty())
        .map(|section| {
            let texture = section.texture.and_then(|at| textures.get(at).copied());
            (section.indices.clone(), texture)
        })
        .collect()
}

/// One mesh on the GPU.
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    draws: Vec<SectionDraw>,
}

/// The model pipeline, the meshes it draws and the textures they sample.
pub(super) struct Models {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// What a section with no texture samples: one opaque white texel.
    white: wgpu::BindGroup,
    textures: HashMap<ModelTextureId, wgpu::BindGroup>,
    meshes: HashMap<MeshId, GpuMesh>,
}

impl Models {
    /// The pipeline, built on `device`, with the white texture untextured
    /// sections sample, written on `queue`; no meshes and no textures.
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let layout = texture_layout(device);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova model texture"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let white = PlanetImage {
            width: 1,
            height: 1,
            rgba: vec![u8::MAX; 4],
        };
        let white = group(
            device,
            queue,
            &layout,
            &sampler,
            std::slice::from_ref(&white),
        );
        Self {
            pipeline: pipeline(device, &layout),
            layout,
            sampler,
            white,
            textures: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    /// Holds `data` as model texture `texture`, replacing any of that id.
    pub(super) fn upload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: ModelTextureId,
        data: &ModelTexture,
    ) {
        let group = group(device, queue, &self.layout, &self.sampler, data.levels());
        self.textures.insert(texture, group);
    }

    /// Holds `data` as mesh `mesh`, replacing any mesh of that id, its
    /// textures those uploaded as `textures`. A mesh with no triangles
    /// (which the core never hands over) is dropped.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        mesh: MeshId,
        data: &Mesh,
        textures: &[ModelTextureId],
    ) {
        if data.is_empty() {
            self.meshes.remove(&mesh);
            return;
        }
        let vertices: Vec<MeshVertex> = data.vertices().iter().map(MeshVertex::from).collect();
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nova mesh vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("nova mesh indices"),
            contents: bytemuck::cast_slice(data.indices()),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.meshes.insert(
            mesh,
            GpuMesh {
                vertices,
                indices,
                draws: draws(data, textures),
            },
        );
    }

    /// Encodes the models `instances` of `buffer`, mesh `meshes[i]` for
    /// the i-th, into `pass`: one indexed draw for each of the mesh's
    /// sections, its texture bound, or the white one for a section with
    /// none (or with one the GPU doesn't hold). A mesh the GPU doesn't
    /// hold is skipped.
    pub(super) fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        buffer: &wgpu::Buffer,
        instances: Range<u32>,
        meshes: &[MeshId],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(1, buffer.slice(..));
        for (instance, mesh) in instances.zip(meshes) {
            let Some(mesh) = self.meshes.get(mesh) else {
                continue;
            };
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (indices, texture) in &mesh.draws {
                let group = texture
                    .and_then(|id| self.textures.get(&id))
                    .unwrap_or(&self.white);
                pass.set_bind_group(0, group, &[]);
                pass.draw_indexed(indices.clone(), 0, instance..instance + 1);
            }
        }
    }
}

/// A model texture's layout: the texture, and its sampler.
fn texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova model texture"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// The bind group of a texture of `levels`, a mip chain, uploaded on
/// `queue`, sampled through `sampler`.
fn group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    levels: &[PlanetImage],
) -> wgpu::BindGroup {
    let view = texture(device, queue, "nova model texture", levels)
        .create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("nova model texture"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

/// A `size` depth texture's view, for a canvas's models passes.
pub(super) fn depth_view(device: &wgpu::Device, size: (u32, u32)) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("nova depth"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// The model pipeline: mesh vertices in slot 0, [`ModelData`] per
/// instance in slot 1, and a section's texture (`texture`'s layout) as
/// group 0.
fn pipeline(device: &wgpu::Device, texture: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("models.wgsl"));
    let layout = pipeline_layout(device, "nova models", &[texture]);
    let vertex = wgpu::vertex_attr_array![
        0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2, 4 => Float32,
    ];
    let instance = wgpu::vertex_attr_array![
        5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
        9 => Float32x4, 10 => Float32x4, 11 => Float32x4, 12 => Float32x4,
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nova models"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("model_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<MeshVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &vertex,
                }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<ModelData>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &instance,
                }),
            ],
        },
        // The default culls nothing: the depth test hides the far faces.
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("model_fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: SCENE_FORMAT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model::Section;

    #[test]
    fn a_vertex_becomes_its_position_normal_colour_coordinates_and_unlit_flag_in_order() {
        let lit = Vertex {
            position: [1.0, 2.0, 3.0],
            normal: [4.0, 5.0, 6.0],
            color: [0.1, 0.2, 0.3, 0.4],
            uv: [0.25, 0.75],
            unlit: false,
        };
        let vertex = MeshVertex::from(&lit);
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&vertex));
        assert_eq!(
            floats,
            [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.1, 0.2, 0.3, 0.4, 0.25, 0.75, 0.0
            ]
        );
        let unlit = MeshVertex::from(&Vertex { unlit: true, ..lit });
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&unlit));
        assert_eq!(floats[12..], [1.0]);
    }

    #[test]
    fn a_mesh_draws_each_section_that_has_triangles_with_its_texture_s_id() {
        let cube = Mesh::cube([nova_view::Color::WHITE; 6]);
        let texture = || Arc::new(ModelTexture::new(1, 1, vec![9, 9, 9, 255]).expect("one texel"));
        let section = |indices, texture| Section { indices, texture };
        let mesh = Mesh::textured(
            cube.vertices().to_vec(),
            cube.indices().to_vec(),
            vec![texture(), texture()],
            vec![
                section(0..6, Some(1)),
                section(6..6, Some(0)),
                section(6..18, None),
                section(18..36, Some(0)),
            ],
        )
        .expect("the cube in sections");
        let ids = [ModelTextureId(7), ModelTextureId(4)];
        assert_eq!(
            draws(&mesh, &ids),
            [
                (0..6, Some(ModelTextureId(4))),
                (6..18, None),
                (18..36, Some(ModelTextureId(7))),
            ]
        );
        // An id the mesh was not given draws untextured.
        assert_eq!(
            draws(&mesh, &ids[..1]),
            [
                (0..6, None),
                (6..18, None),
                (18..36, Some(ModelTextureId(7))),
            ]
        );
        assert_eq!(draws(&cube, &[]), [(0..36, None)]);
    }

    #[test]
    fn a_model_becomes_its_columns_then_its_light() {
        let model = ModelInstance {
            mesh: MeshId(2),
            clip_from_model: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
            normal_from_model: [[17.0, 18.0, 19.0], [20.0, 21.0, 22.0], [23.0, 24.0, 25.0]],
            shares_band: false,
        };
        let light = Light {
            direction: [26.0, 27.0, 28.0],
            ambient: 0.5,
        };
        let data = ModelData::new(&model, light);
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&data));
        let expected: Vec<f32> = (1..=16)
            .map(|n| n as f32)
            .chain([17.0, 18.0, 19.0, 0.0, 20.0, 21.0, 22.0, 0.0])
            .chain([23.0, 24.0, 25.0, 0.0, 26.0, 27.0, 28.0, 0.5])
            .collect();
        assert_eq!(floats, expected.as_slice());
    }
}
