//! The wgpu side of [`Batch::Planets`](crate::Batch::Planets): each
//! planet's textures ([`crate::Gpu::upload_planet`]), the planet pipeline
//! and its draws. It decides nothing: each instance's square, axes, turns
//! and sizes and the light come from the core ([`crate::planet`]), and
//! `planets.wgsl` mirrors the core's functions per fragment.
//!
//! The pipeline draws into [`SCENE_FORMAT`] with no depth attachment,
//! blending as a normal sprite does (straight alpha). The surface and
//! clouds are sampled through a sampler that repeats in u and clamps in v,
//! trilinear, with explicit gradients; the ring profile through one that
//! clamps, linear.

use std::collections::HashMap;
use std::ops::Range;

use bytemuck::{Pod, Zeroable};

use super::renderer::{SCENE_FORMAT, pipeline_layout};
use crate::camera::Light;
use crate::gpu::{PlanetId, PlanetImage, PlanetInstance, PlanetTextures};

/// One planet as the planet shader reads it, per instance.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub(super) struct PlanetData {
    /// The square: x, y, width, height in logical units.
    pub dest: [f32; 4],
    /// North, then the surface's turn.
    pub north: [f32; 4],
    /// The equator's point facing the camera, then the clouds' turn.
    pub facing: [f32; 4],
    /// East, then the atmosphere's strength at the limb
    /// ([`crate::planet::RIM`]).
    pub east: [f32; 4],
    /// The rings' inner and outer radii, the reach, then 0.
    pub rings: [f32; 4],
    /// The rings' colour, then 0.
    pub ring_color: [f32; 4],
    /// The atmosphere's colour and thickness.
    pub atmosphere: [f32; 4],
    /// The light's direction and its ambient.
    pub light: [f32; 4],
}

impl PlanetData {
    /// `planet` lit by `light`.
    pub(super) fn new(planet: &PlanetInstance, light: Light) -> Self {
        let with = |[a, b, c]: [f32; 3], d: f32| [a, b, c, d];
        let dest = planet.dest;
        let [surface_turn, cloud_turn] = planet.turns;
        let [inner, outer] = planet.rings;
        Self {
            dest: [dest.x, dest.y, dest.w, dest.h],
            north: with(planet.frame.north, surface_turn),
            facing: with(planet.frame.facing, cloud_turn),
            east: with(planet.frame.east, crate::planet::RIM),
            rings: [inner, outer, planet.reach, 0.0],
            ring_color: with(planet.ring_color, 0.0),
            atmosphere: planet.atmosphere,
            light: with(light.direction, light.ambient),
        }
    }
}

/// The planet pipeline and the planets' textures.
pub(super) struct Planets {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    wrap: wgpu::Sampler,
    clamp: wgpu::Sampler,
    groups: HashMap<PlanetId, wgpu::BindGroup>,
}

impl Planets {
    /// The pipeline, built on `device` with the projection's `globals`
    /// layout as group 0, and no planets.
    pub(super) fn new(device: &wgpu::Device, globals: &wgpu::BindGroupLayout) -> Self {
        let layout = textures_layout(device);
        let wrap = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova planet surface"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let clamp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova planet rings"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline: pipeline(device, globals, &layout),
            layout,
            wrap,
            clamp,
            groups: HashMap::new(),
        }
    }

    /// Holds `textures` as planet `planet`'s, replacing any it had.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        planet: PlanetId,
        textures: &PlanetTextures,
    ) {
        let surface = texture(device, queue, "nova planet surface", &textures.surface);
        let clouds = texture(device, queue, "nova planet clouds", &textures.clouds);
        let rings = texture(
            device,
            queue,
            "nova planet rings",
            std::slice::from_ref(&textures.rings),
        );
        let view = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let (surface, clouds, rings) = (view(&surface), view(&clouds), view(&rings));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nova planet"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&surface),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&clouds),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&rings),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.wrap),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.clamp),
                },
            ],
        });
        self.groups.insert(planet, group);
    }

    /// Drops planet `planet`'s textures: they are owned only through its
    /// bind group, and wgpu keeps them alive until the GPU has finished
    /// any work already submitted with them. It draws nothing until it is
    /// uploaded again.
    pub(super) fn release(&mut self, planet: PlanetId) {
        self.groups.remove(&planet);
    }

    /// Encodes the planets `instances` of `buffer`, planet `planets[i]`
    /// for the i-th, into `pass`, the projection's `globals` bound, one
    /// square each. A planet the GPU doesn't hold is skipped.
    pub(super) fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        globals: &wgpu::BindGroup,
        buffer: &wgpu::Buffer,
        instances: Range<u32>,
        planets: &[PlanetId],
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals, &[]);
        pass.set_vertex_buffer(0, buffer.slice(..));
        for (instance, planet) in instances.zip(planets) {
            let Some(group) = self.groups.get(planet) else {
                continue;
            };
            pass.set_bind_group(1, group, &[]);
            pass.draw(0..6, instance..instance + 1);
        }
    }
}

/// A texture of `levels`, a mip chain (one level for a ring's row),
/// uploaded on `queue`: a planet's, or a model's
/// (`super::models`).
pub(super) fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    levels: &[PlanetImage],
) -> wgpu::Texture {
    let base = &levels[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: base.width,
            height: base.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SCENE_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, image) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: 1,
            },
        );
    }
    texture
}

/// A planet's textures' layout: the surface, the clouds, the rings, and
/// the two samplers.
fn textures_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let sampler = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova planet"),
        entries: &[texture(0), texture(1), texture(2), sampler(3), sampler(4)],
    })
}

/// The planet pipeline: [`PlanetData`] per instance in slot 0.
fn pipeline(
    device: &wgpu::Device,
    globals: &wgpu::BindGroupLayout,
    textures: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("planets.wgsl"));
    let layout = pipeline_layout(device, "nova planets", &[globals, textures]);
    let instance = wgpu::vertex_attr_array![
        0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
        4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("nova planets"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("planet_vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<PlanetData>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance,
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("planet_fs"),
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
    use crate::gpu::Rect;
    use crate::planet::Frame;

    #[test]
    fn a_planet_becomes_its_square_axes_turns_sizes_and_light_in_order() {
        let planet = PlanetInstance {
            planet: PlanetId(1),
            dest: Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            },
            reach: 2.5,
            frame: Frame {
                north: [5.0, 6.0, 7.0],
                facing: [8.0, 9.0, 10.0],
                east: [11.0, 12.0, 13.0],
            },
            turns: [0.25, 0.5],
            rings: [1.4, 2.3],
            ring_color: [0.1, 0.2, 0.3],
            atmosphere: [0.4, 0.5, 0.6, 0.04],
        };
        let light = Light {
            direction: [14.0, 15.0, 16.0],
            ambient: 0.75,
        };
        let data = PlanetData::new(&planet, light);
        let floats: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&data));
        // East's spare lane carries the core's rim strength.
        let rim = crate::planet::RIM;
        assert_eq!(
            floats,
            [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 0.25, 8.0, 9.0, 10.0, 0.5, 11.0, 12.0, 13.0,
                rim, 1.4, 2.3, 2.5, 0.0, 0.1, 0.2, 0.3, 0.0, 0.4, 0.5, 0.6, 0.04, 14.0, 15.0, 16.0,
                0.75,
            ]
        );
    }
}
