//! Draws frames into a texture view with wgpu, and text with glyphon.

use std::collections::HashMap;
use std::ops::Range;

use glyphon::{
    Attrs, Buffer, Cache, ColorMode, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache,
    TextArea, TextAtlas, TextBounds, TextRenderer,
};
use nova_data::graphics::Image;
use wgpu::util::DeviceExt;

use super::data::{SolidVertex, SpriteInstance, globals, solid_vertices};
use nova_view::Color;

use crate::gpu::{Batch, Frame, PageId, TextRun};
use crate::viewport::PixelRect;

/// One atlas page on the GPU.
struct Page {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// What to draw for one batch, once its data is in the buffers.
enum Draw {
    Sprites { page: PageId, instances: Range<u32> },
    Solid { vertices: Range<u32> },
    Text { renderer: usize },
}

/// The wgpu half of a [`Gpu`](crate::Gpu): pipelines, atlas page textures,
/// per-frame buffers and glyphon's text state. Both GPU adapters delegate
/// to it.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprite_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    page_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    globals_group: wgpu::BindGroup,
    pages: HashMap<PageId, Page>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_viewport: glyphon::Viewport,
    text_renderers: Vec<TextRenderer>,
}

impl WgpuRenderer {
    /// A renderer drawing into `format` textures on `device`.
    #[must_use]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let globals_layout = globals_layout(device);
        let page_layout = page_layout(device);
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nova globals"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nova globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let sprite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nova sprites"),
            bind_group_layouts: &[Some(&globals_layout), Some(&page_layout)],
            immediate_size: 0,
        });
        let solid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nova solids"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let sprite_attributes =
            wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let solid_attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
        let sprite_pipeline = pipeline(
            device,
            &sprite_layout,
            &shader,
            ("sprite_vs", "sprite_fs"),
            wgpu::VertexBufferLayout {
                array_stride: size_of::<SpriteInstance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &sprite_attributes,
            },
            format,
        );
        let solid_pipeline = pipeline(
            device,
            &solid_layout,
            &shader,
            ("solid_vs", "solid_fs"),
            wgpu::VertexBufferLayout {
                array_stride: size_of::<SolidVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &solid_attributes,
            },
            format,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let cache = Cache::new(device);
        let text_atlas = TextAtlas::with_color_mode(device, queue, &cache, format, ColorMode::Web);
        let text_viewport = glyphon::Viewport::new(device, &cache);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            sprite_pipeline,
            solid_pipeline,
            page_layout,
            sampler,
            globals,
            globals_group,
            pages: HashMap::new(),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            text_atlas,
            text_viewport,
            text_renderers: Vec::new(),
        }
    }

    /// How many font faces glyphon found on the system.
    #[must_use]
    pub fn font_faces(&self) -> usize {
        self.font_system.db().len()
    }

    /// Creates atlas page `page`, `size` texels square and transparent.
    pub fn create_page(&mut self, page: PageId, size: u32) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nova atlas page"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nova atlas page"),
            layout: &self.page_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.pages.insert(
            page,
            Page {
                texture,
                bind_group,
            },
        );
    }

    /// Copies `image` onto page `page` at `at`.
    pub fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        let Some(page) = self.pages.get(&page) else {
            return;
        };
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &page.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: at.x,
                    y: at.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            image.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width()),
                rows_per_image: Some(image.height()),
            },
            wgpu::Extent3d {
                width: image.width(),
                height: image.height(),
                depth_or_array_layers: 1,
            },
        );
    }

    /// Draws `frame` into `view`, a texture of the frame's target size.
    pub fn draw(&mut self, frame: &Frame, view: &wgpu::TextureView) {
        self.queue.write_buffer(
            &self.globals,
            0,
            bytemuck::cast_slice(&globals(frame.logical)),
        );
        let layout = lay_out(frame);
        self.prepare_text(frame.target, &layout.text);
        let instance_buffer = self.vertex_buffer("nova sprite instances", &layout.instances);
        let vertex_buffer = self.vertex_buffer("nova solid vertices", &layout.vertices);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nova frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color(frame.clear)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let content = frame.viewport;
            let full = PixelRect {
                x: 0,
                y: 0,
                w: frame.target.0,
                h: frame.target.1,
            };
            for draw in &layout.draws {
                match draw {
                    Draw::Sprites { page, instances } => {
                        let Some(page) = self.pages.get(page) else {
                            continue;
                        };
                        set_viewport(&mut pass, content);
                        pass.set_pipeline(&self.sprite_pipeline);
                        pass.set_bind_group(0, &self.globals_group, &[]);
                        pass.set_bind_group(1, &page.bind_group, &[]);
                        pass.set_vertex_buffer(0, instance_buffer.slice(..));
                        pass.draw(0..6, instances.clone());
                    }
                    Draw::Solid { vertices } => {
                        set_viewport(&mut pass, content);
                        pass.set_pipeline(&self.solid_pipeline);
                        pass.set_bind_group(0, &self.globals_group, &[]);
                        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        pass.draw(vertices.clone(), 0..1);
                    }
                    Draw::Text { renderer } => {
                        set_viewport(&mut pass, full);
                        // A failed render draws nothing; the frame goes on.
                        let _ = self.text_renderers[*renderer].render(
                            &self.text_atlas,
                            &self.text_viewport,
                            &mut pass,
                        );
                    }
                }
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.text_atlas.trim();
    }

    /// Shapes every text batch and prepares one glyphon renderer for each.
    fn prepare_text(&mut self, target: (u32, u32), batches: &[&[TextRun]]) {
        self.text_viewport.update(
            &self.queue,
            Resolution {
                width: target.0,
                height: target.1,
            },
        );
        // Grow the pool to one renderer per batch; a bounded loop.
        let missing = batches.len().saturating_sub(self.text_renderers.len());
        for _ in 0..missing {
            self.text_renderers.push(TextRenderer::new(
                &mut self.text_atlas,
                &self.device,
                wgpu::MultisampleState::default(),
                None,
            ));
        }
        for (renderer, runs) in self.text_renderers.iter_mut().zip(batches) {
            let buffers: Vec<Buffer> = runs
                .iter()
                .map(|run| {
                    let mut buffer = Buffer::new(
                        &mut self.font_system,
                        Metrics::new(run.size_px, run.line_height_px),
                    );
                    buffer.set_size(run.wrap_px, None);
                    buffer.set_text(
                        &run.text,
                        &Attrs::new().family(Family::SansSerif),
                        Shaping::Advanced,
                        None,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);
                    buffer
                })
                .collect();
            let areas = runs.iter().zip(&buffers).map(|(run, buffer)| TextArea {
                buffer,
                left: run.origin_px.0,
                top: run.origin_px.1,
                scale: 1.0,
                bounds: TextBounds {
                    left: run.clip.x as i32,
                    top: run.clip.y as i32,
                    right: (run.clip.x + run.clip.w) as i32,
                    bottom: (run.clip.y + run.clip.h) as i32,
                },
                default_color: glyphon::Color::rgba(
                    run.color.r,
                    run.color.g,
                    run.color.b,
                    run.color.a,
                ),
                custom_glyphs: &[],
            });
            // A failed prepare draws no text this frame; the frame goes on.
            let _ = renderer.prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.text_atlas,
                &self.text_viewport,
                areas,
                &mut self.swash_cache,
            );
        }
    }

    /// A vertex buffer holding `data` (at least 4 bytes, as wgpu needs).
    fn vertex_buffer<T: bytemuck::Pod>(&self, label: &str, data: &[T]) -> wgpu::Buffer {
        let bytes: &[u8] = bytemuck::cast_slice(data);
        let contents = if bytes.is_empty() { &[0; 4][..] } else { bytes };
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::VERTEX,
            })
    }
}

/// The projection uniform's layout.
fn globals_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova globals"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// An atlas page's layout: its texture and the sampler.
fn page_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova atlas page"),
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

/// A frame's batches laid out for the GPU.
struct Layout<'a> {
    /// Every sprite batch's instances, in one buffer.
    instances: Vec<SpriteInstance>,
    /// Every solid batch's vertices, in one buffer.
    vertices: Vec<SolidVertex>,
    /// The text batches, one glyphon renderer each.
    text: Vec<&'a [TextRun]>,
    /// What to draw for each batch, in order.
    draws: Vec<Draw>,
}

/// Lays a frame's batches out in one instance buffer, one vertex buffer
/// and a list of text batches, with what to draw for each batch in order.
fn lay_out(frame: &Frame) -> Layout<'_> {
    let mut instances: Vec<SpriteInstance> = Vec::new();
    let mut vertices: Vec<SolidVertex> = Vec::new();
    let mut text_batches: Vec<&[TextRun]> = Vec::new();
    let mut draws = Vec::new();
    for batch in &frame.batches {
        draws.push(match batch {
            Batch::Sprites { page, quads } => {
                let start = instances.len() as u32;
                instances.extend(quads.iter().map(SpriteInstance::from));
                Draw::Sprites {
                    page: *page,
                    instances: start..instances.len() as u32,
                }
            }
            Batch::Solid(quads) => {
                let start = vertices.len() as u32;
                vertices.extend(quads.iter().flat_map(solid_vertices));
                Draw::Solid {
                    vertices: start..vertices.len() as u32,
                }
            }
            Batch::Text(runs) => {
                text_batches.push(runs);
                Draw::Text {
                    renderer: text_batches.len() - 1,
                }
            }
        });
    }
    Layout {
        instances,
        vertices,
        text: text_batches,
        draws,
    }
}

/// The clear colour as wgpu wants it.
fn clear_color(clear: Color) -> wgpu::Color {
    wgpu::Color {
        r: f64::from(clear.r) / 255.0,
        g: f64::from(clear.g) / 255.0,
        b: f64::from(clear.b) / 255.0,
        a: f64::from(clear.a) / 255.0,
    }
}

fn set_viewport(pass: &mut wgpu::RenderPass<'_>, rect: PixelRect) {
    pass.set_viewport(
        rect.x as f32,
        rect.y as f32,
        rect.w as f32,
        rect.h as f32,
        0.0,
        1.0,
    );
}

/// A pipeline drawing alpha-blended triangles into `format`.
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    (vertex, fragment): (&str, &str),
    buffer: wgpu::VertexBufferLayout<'_>,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(vertex),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(buffer)],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
