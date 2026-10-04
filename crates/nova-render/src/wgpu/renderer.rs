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
use nova_view::{Blend, Color};

use super::fonts::{Families, font_system};
use crate::fonts::FontFaces;
use crate::gpu::{Batch, Frame, PageId, TextRun};
use crate::viewport::PixelRect;

/// One atlas page on the GPU.
struct Page {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

/// What to draw for one batch, once its data is in the buffers.
#[derive(Debug, PartialEq)]
enum Draw {
    Sprites {
        page: PageId,
        blend: Blend,
        instances: Range<u32>,
    },
    Solid {
        vertices: Range<u32>,
    },
    Text {
        renderer: usize,
    },
}

/// The wgpu half of a [`Gpu`](crate::Gpu): pipelines, atlas page textures,
/// per-frame buffers and glyphon's text state. Both GPU adapters delegate
/// to it.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprite_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    page_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    globals: wgpu::Buffer,
    globals_group: wgpu::BindGroup,
    pages: HashMap<PageId, Page>,
    font_system: FontSystem,
    families: Families,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_viewport: glyphon::Viewport,
    text_renderers: Vec<TextRenderer>,
}

impl WgpuRenderer {
    /// A renderer drawing into `format` textures on `device`, with text in
    /// `faces` alone: no system font is loaded.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        faces: &FontFaces,
    ) -> Self {
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
        let sprite_pipeline_with = |blend| {
            pipeline(
                device,
                &sprite_layout,
                &shader,
                ("sprite_vs", "sprite_fs"),
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<SpriteInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &sprite_attributes,
                },
                (format, blend_state(blend)),
            )
        };
        let sprite_pipeline = sprite_pipeline_with(Blend::Normal);
        let additive_pipeline = sprite_pipeline_with(Blend::Additive);
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
            (format, blend_state(Blend::Normal)),
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
        let (font_system, families) = font_system(faces);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            sprite_pipeline,
            additive_pipeline,
            solid_pipeline,
            page_layout,
            sampler,
            globals,
            globals_group,
            pages: HashMap::new(),
            font_system,
            families,
            swash_cache: SwashCache::new(),
            text_atlas,
            text_viewport,
            text_renderers: Vec::new(),
        }
    }

    /// How many font faces loaded.
    #[must_use]
    pub fn font_faces(&self) -> usize {
        self.font_system.db().len()
    }

    /// Whether Charcoal's font file loaded a face.
    #[must_use]
    pub fn charcoal_loaded(&self) -> bool {
        self.families.charcoal_loaded()
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
                    Draw::Sprites {
                        page,
                        blend,
                        instances,
                    } => {
                        let Some(page) = self.pages.get(page) else {
                            continue;
                        };
                        set_viewport(&mut pass, content);
                        pass.set_pipeline(match blend {
                            Blend::Normal => &self.sprite_pipeline,
                            Blend::Additive => &self.additive_pipeline,
                        });
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
        let families = &self.families;
        for (renderer, runs) in self.text_renderers.iter_mut().zip(batches) {
            // A run with no face to draw in is skipped (shaping it would
            // find no font at all).
            let drawn: Vec<(&TextRun, Family<'_>)> = runs
                .iter()
                .filter_map(|run| Some((run, families.family(run.font)?)))
                .collect();
            let buffers: Vec<Buffer> = drawn
                .iter()
                .map(|&(run, family)| {
                    let mut buffer = Buffer::new(
                        &mut self.font_system,
                        Metrics::new(run.size_px, run.line_height_px),
                    );
                    buffer.set_size(run.wrap_px, None);
                    buffer.set_text(
                        &run.text,
                        &Attrs::new().family(family),
                        Shaping::Advanced,
                        None,
                    );
                    buffer.shape_until_scroll(&mut self.font_system, false);
                    buffer
                })
                .collect();
            let areas = drawn
                .iter()
                .zip(&buffers)
                .map(|(&(run, _), buffer)| TextArea {
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
            Batch::Sprites { page, blend, quads } => {
                let start = instances.len() as u32;
                instances.extend(quads.iter().map(SpriteInstance::from));
                Draw::Sprites {
                    page: *page,
                    blend: *blend,
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

/// How `blend` combines a fragment with the target. The fragment shaders
/// output straight (unpremultiplied) alpha, so additive is destination +
/// source x source alpha, keeping the destination's alpha.
fn blend_state(blend: Blend) -> wgpu::BlendState {
    match blend {
        Blend::Normal => wgpu::BlendState::ALPHA_BLENDING,
        Blend::Additive => wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        },
    }
}

/// A pipeline drawing triangles into `format`, combined with the target
/// by `blend`.
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    (vertex, fragment): (&str, &str),
    buffer: wgpu::VertexBufferLayout<'_>,
    (format, blend): (wgpu::TextureFormat, wgpu::BlendState),
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
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use nova_view::{Blend, Color, Point};

    use super::*;
    use crate::gpu::{QuadInstance, Rect, SolidQuad};
    use crate::viewport::LogicalSize;
    use crate::{PageId, Uv};

    /// A quad told apart from the others by `n`.
    fn quad(n: f32) -> QuadInstance {
        QuadInstance {
            dest: Rect {
                x: n,
                y: n,
                w: 1.0,
                h: 1.0,
            },
            uv: Uv {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            tint: [1.0; 4],
        }
    }

    /// A solid quad told apart from the others by `n`.
    fn solid(n: f32) -> SolidQuad {
        let p = |x, y| Point::new(x, y);
        SolidQuad {
            corners: [p(n, n), p(n + 1.0, n), p(n + 1.0, n + 1.0), p(n, n + 1.0)],
            color: [1.0; 4],
        }
    }

    fn run(text: &str) -> TextRun {
        TextRun {
            text: text.to_owned(),
            font: nova_view::Font::Geneva,
            origin_px: (0.0, 0.0),
            size_px: 16.0,
            line_height_px: 19.2,
            wrap_px: None,
            color: Color::WHITE,
            clip: PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 64,
            },
        }
    }

    #[test]
    fn batches_share_buffers_and_draw_their_own_ranges_in_order() {
        let first_text = vec![run("a"), run("b")];
        let second_text = vec![run("c")];
        let frame = Frame {
            target: (64, 64),
            viewport: PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 64,
            },
            logical: LogicalSize { w: 32, h: 32 },
            clear: Color::BLACK,
            batches: vec![
                Batch::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    quads: vec![quad(0.0), quad(1.0)],
                },
                Batch::Solid(vec![solid(10.0)]),
                Batch::Sprites {
                    page: PageId(1),
                    blend: Blend::Normal,
                    quads: vec![quad(2.0)],
                },
                Batch::Text(first_text.clone()),
                Batch::Solid(vec![solid(11.0), solid(12.0)]),
                Batch::Text(second_text.clone()),
                Batch::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    quads: vec![quad(3.0)],
                },
            ],
        };

        let layout = lay_out(&frame);

        assert_eq!(
            layout.draws,
            [
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    instances: 0..2,
                },
                Draw::Solid { vertices: 0..6 },
                Draw::Sprites {
                    page: PageId(1),
                    blend: Blend::Normal,
                    instances: 2..3,
                },
                Draw::Text { renderer: 0 },
                Draw::Solid { vertices: 6..18 },
                Draw::Text { renderer: 1 },
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    instances: 3..4,
                },
            ]
        );
        let instances: Vec<SpriteInstance> = [0.0, 1.0, 2.0, 3.0]
            .map(|n| SpriteInstance::from(&quad(n)))
            .to_vec();
        assert_eq!(layout.instances, instances);
        let vertices: Vec<SolidVertex> = [10.0, 11.0, 12.0]
            .iter()
            .flat_map(|&n| solid_vertices(&solid(n)))
            .collect();
        assert_eq!(layout.vertices, vertices);
        assert_eq!(layout.text, [&first_text[..], &second_text[..]]);
    }

    fn frame_of(batches: Vec<Batch>) -> Frame {
        Frame {
            target: (64, 64),
            viewport: PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 64,
            },
            logical: LogicalSize { w: 32, h: 32 },
            clear: Color::BLACK,
            batches,
        }
    }

    #[test]
    fn lay_out_keeps_each_batchs_blend() {
        let frame = frame_of(vec![
            Batch::Sprites {
                page: PageId(0),
                blend: Blend::Normal,
                quads: vec![quad(0.0)],
            },
            Batch::Sprites {
                page: PageId(0),
                blend: Blend::Additive,
                quads: vec![quad(1.0), quad(2.0)],
            },
        ]);

        assert_eq!(
            lay_out(&frame).draws,
            [
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    instances: 0..1,
                },
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Additive,
                    instances: 1..3,
                },
            ]
        );
    }

    #[test]
    fn blend_state_adds_source_times_its_alpha() {
        assert_eq!(blend_state(Blend::Normal), wgpu::BlendState::ALPHA_BLENDING);
        assert_eq!(
            blend_state(Blend::Additive),
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }
        );
    }
}
