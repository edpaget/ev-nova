//! The wgpu side of [`Batch::Models`](crate::Batch::Models): the meshes'
//! buffers ([`crate::Gpu::upload_mesh`]), the depth textures, the model
//! pipeline and its draws. It decides nothing: each instance's transforms
//! and the light come from the core ([`crate::camera`]).
//!
//! The pipeline draws into [`SCENE_FORMAT`] with a [`DEPTH_FORMAT`]
//! attachment, compares `Less` and writes depth, culls nothing (the depth
//! test hides a model's far faces) and blends as a normal sprite does.
//! `models.wgsl` lights each fragment by the formula [`crate::camera::shade`]
//! defines.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::renderer::{SCENE_FORMAT, pipeline_layout};
use crate::camera::Light;
use crate::gpu::ModelInstance;
use crate::model::{Mesh, MeshId, Vertex};

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
}

impl From<&Vertex> for MeshVertex {
    fn from(vertex: &Vertex) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
            color: vertex.color,
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

/// One mesh on the GPU.
struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// The model pipeline and the meshes it draws.
pub(super) struct Models {
    pipeline: wgpu::RenderPipeline,
    meshes: HashMap<MeshId, GpuMesh>,
}

impl Models {
    /// The pipeline, built on `device`, and no meshes.
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            pipeline: pipeline(device),
            meshes: HashMap::new(),
        }
    }

    /// Holds `data` as mesh `mesh`, replacing any mesh of that id. A mesh
    /// with no triangles (which the core never hands over) is dropped.
    pub(super) fn upload(&mut self, device: &wgpu::Device, mesh: MeshId, data: &Mesh) {
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
                count: data.indices().len() as u32,
            },
        );
    }

    /// Encodes the models `instances` of `buffer`, mesh `meshes[i]` for
    /// the i-th, into `pass`, one indexed draw each. A mesh the GPU
    /// doesn't hold is skipped.
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
            pass.draw_indexed(0..mesh.count, 0, instance..instance + 1);
        }
    }
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
/// instance in slot 1.
fn pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("models.wgsl"));
    let layout = pipeline_layout(device, "nova models", &[]);
    let vertex = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4];
    let instance = wgpu::vertex_attr_array![
        3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
        7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4,
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
    use super::*;

    #[test]
    fn a_vertex_becomes_its_position_normal_and_colour_in_order() {
        let vertex = MeshVertex::from(&Vertex {
            position: [1.0, 2.0, 3.0],
            normal: [4.0, 5.0, 6.0],
            color: [0.1, 0.2, 0.3, 0.4],
        });
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&vertex));
        assert_eq!(floats, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.1, 0.2, 0.3, 0.4]);
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
