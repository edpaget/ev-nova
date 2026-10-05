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

/// The format every frame is drawn in, whatever the target's: the scene
/// texture's, so the pipelines (and the OR composite) work on known 8-bit
/// unorm channels. Only the blit to the target uses the target's format.
const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The offscreen texture a frame is drawn into before one blit copies it
/// to the target, with the bind group the blit reads it through, and the
/// same-sized backdrop an OR batch reads what is beneath it from.
struct Scene {
    size: (u32, u32),
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    blit_group: wgpu::BindGroup,
    backdrop: wgpu::Texture,
    backdrop_group: wgpu::BindGroup,
}

/// Whether a scene of size `current` (`None` before the first frame) can
/// draw a frame for a `target`-sized target.
fn reuse_scene(current: Option<(u32, u32)>, target: (u32, u32)) -> bool {
    current == Some(target)
}

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

/// A render pass over the scene and the draws it encodes.
#[derive(Debug, PartialEq)]
struct Pass {
    /// For a pass opened by an OR batch, the scene pixels the batch can
    /// touch, copied into the backdrop (at the same place) before it.
    backdrop: Option<PixelRect>,
    /// Its draws, as indices into [`Layout::draws`].
    draws: Range<usize>,
}

/// The wgpu half of a [`Gpu`](crate::Gpu): pipelines, atlas page textures,
/// per-frame buffers and glyphon's text state. Both GPU adapters delegate
/// to it.
///
/// Every frame is drawn into an offscreen scene texture in
/// [`SCENE_FORMAT`], then copied to the target by one blit. An OR batch
/// ([`Blend::Or`]) reads what is beneath it, which a pass cannot do with
/// its own attachment, so each one ends the pass: the rectangle the core
/// says it covers ([`Frame::pixels_covered`]) is copied from the scene to
/// the backdrop, and a new pass draws the batch with a shader that ORs
/// into the backdrop's texel.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprite_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    or_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    page_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    scene: Option<Scene>,
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
    /// `faces` alone: no system font is loaded. `format` is the target's;
    /// only the final blit draws in it.
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
        let texture_layout = unfiltered_texture_layout(device);
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
        let sprite_layout =
            pipeline_layout(device, "nova sprites", &[&globals_layout, &page_layout]);
        let or_layout = pipeline_layout(
            device,
            "nova or sprites",
            &[&globals_layout, &page_layout, &texture_layout],
        );
        let solid_layout = pipeline_layout(device, "nova solids", &[&globals_layout]);
        let sprite_attributes =
            wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let solid_attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
        let sprite_pipeline_with = |layout, fragment, blend| {
            pipeline(
                device,
                layout,
                &shader,
                ("sprite_vs", fragment),
                &[Some(wgpu::VertexBufferLayout {
                    array_stride: size_of::<SpriteInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &sprite_attributes,
                })],
                (SCENE_FORMAT, blend_state(blend)),
            )
        };
        let sprite_pipeline = sprite_pipeline_with(&sprite_layout, "sprite_fs", Blend::Normal);
        let additive_pipeline = sprite_pipeline_with(&sprite_layout, "sprite_fs", Blend::Additive);
        let or_pipeline = sprite_pipeline_with(&or_layout, "or_fs", Blend::Or);
        let solid_pipeline = pipeline(
            device,
            &solid_layout,
            &shader,
            ("solid_vs", "solid_fs"),
            &[Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SolidVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &solid_attributes,
            })],
            (SCENE_FORMAT, blend_state(Blend::Normal)),
        );
        let blit_pipeline = blit_pipeline(device, &texture_layout, format);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nova nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let cache = Cache::new(device);
        let text_atlas =
            TextAtlas::with_color_mode(device, queue, &cache, SCENE_FORMAT, ColorMode::Web);
        let text_viewport = glyphon::Viewport::new(device, &cache);
        let (font_system, families) = font_system(faces);
        Self {
            device: device.clone(),
            queue: queue.clone(),
            sprite_pipeline,
            additive_pipeline,
            or_pipeline,
            solid_pipeline,
            blit_pipeline,
            page_layout,
            texture_layout,
            scene: None,
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

    /// Draws `frame` into `view`, a texture of the frame's target size:
    /// into the scene first, then blitted to `view`.
    pub fn draw(&mut self, frame: &Frame, view: &wgpu::TextureView) {
        self.ensure_scene(frame.target);
        self.queue.write_buffer(
            &self.globals,
            0,
            bytemuck::cast_slice(&globals(frame.logical)),
        );
        let layout = lay_out(frame);
        self.prepare_text(frame.target, &layout.text);
        let instance_buffer = self.vertex_buffer("nova sprite instances", &layout.instances);
        let vertex_buffer = self.vertex_buffer("nova solid vertices", &layout.vertices);

        let scene = self.scene.as_ref().expect("ensured above");
        let buffers = (&instance_buffer, &vertex_buffer);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        for (index, pass) in layout.passes.iter().enumerate() {
            if let Some(rect) = pass.backdrop {
                copy_to_backdrop(&mut encoder, scene, rect);
            }
            let load = if index == 0 {
                wgpu::LoadOp::Clear(clear_color(frame.clear))
            } else {
                wgpu::LoadOp::Load
            };
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nova frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &scene.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let draws = &layout.draws[pass.draws.clone()];
            self.encode(&mut render, frame, draws, buffers, scene);
        }
        blit(&mut encoder, &self.blit_pipeline, &scene.blit_group, view);
        self.queue.submit(Some(encoder.finish()));
        self.text_atlas.trim();
    }

    /// Encodes `draws` into `pass`, reading sprite instances and solid
    /// vertices from `buffers`, and an OR batch's backdrop from `scene`.
    fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &Frame,
        draws: &[Draw],
        (instance_buffer, vertex_buffer): (&wgpu::Buffer, &wgpu::Buffer),
        scene: &Scene,
    ) {
        let content = frame.viewport;
        let full = PixelRect {
            x: 0,
            y: 0,
            w: frame.target.0,
            h: frame.target.1,
        };
        for draw in draws {
            match draw {
                Draw::Sprites {
                    page,
                    blend,
                    instances,
                } => {
                    let Some(page) = self.pages.get(page) else {
                        continue;
                    };
                    set_viewport(pass, content);
                    pass.set_pipeline(match blend {
                        Blend::Normal => &self.sprite_pipeline,
                        Blend::Additive => &self.additive_pipeline,
                        Blend::Or => &self.or_pipeline,
                    });
                    pass.set_bind_group(0, &self.globals_group, &[]);
                    pass.set_bind_group(1, &page.bind_group, &[]);
                    if *blend == Blend::Or {
                        pass.set_bind_group(2, &scene.backdrop_group, &[]);
                    }
                    pass.set_vertex_buffer(0, instance_buffer.slice(..));
                    pass.draw(0..6, instances.clone());
                }
                Draw::Solid { vertices } => {
                    set_viewport(pass, content);
                    pass.set_pipeline(&self.solid_pipeline);
                    pass.set_bind_group(0, &self.globals_group, &[]);
                    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    pass.draw(vertices.clone(), 0..1);
                }
                Draw::Text { renderer } => {
                    set_viewport(pass, full);
                    // A failed render draws nothing; the frame goes on.
                    let _ = self.text_renderers[*renderer].render(
                        &self.text_atlas,
                        &self.text_viewport,
                        pass,
                    );
                }
            }
        }
    }

    /// Makes sure the scene is `target`-sized, creating it on the first
    /// frame and again whenever the target's size changes.
    fn ensure_scene(&mut self, target: (u32, u32)) {
        if reuse_scene(self.scene.as_ref().map(|scene| scene.size), target) {
            return;
        }
        let (texture, view, blit_group) = self.scene_texture(
            "nova scene",
            target,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        let (backdrop, _, backdrop_group) = self.scene_texture(
            "nova backdrop",
            target,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        self.scene = Some(Scene {
            size: target,
            texture,
            view,
            blit_group,
            backdrop,
            backdrop_group,
        });
    }

    /// A `size` texture in [`SCENE_FORMAT`] with `usage`, its view, and a
    /// bind group that reads it with `textureLoad`.
    fn scene_texture(
        &self,
        label: &str,
        size: (u32, u32),
        usage: wgpu::TextureUsages,
    ) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SCENE_FORMAT,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        (texture, view, group)
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

/// A layout of one texture read with `textureLoad` (no sampler) in the
/// fragment stage: the scene for the blit, the backdrop for an OR batch.
fn unfiltered_texture_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("nova scene texture"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    })
}

/// The pipeline that copies the scene, read through `scene_layout`, onto a
/// `format` target as it is.
fn blit_pipeline(
    device: &wgpu::Device,
    scene_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("blit.wgsl"));
    let layout = pipeline_layout(device, "nova blit", &[scene_layout]);
    pipeline(
        device,
        &layout,
        &shader,
        ("blit_vs", "blit_fs"),
        &[],
        (format, None),
    )
}

/// Copies `rect` of the scene to the same place in its backdrop.
fn copy_to_backdrop(encoder: &mut wgpu::CommandEncoder, scene: &Scene, rect: PixelRect) {
    let at = |texture| wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d {
            x: rect.x,
            y: rect.y,
            z: 0,
        },
        aspect: wgpu::TextureAspect::All,
    };
    encoder.copy_texture_to_texture(
        at(&scene.texture),
        at(&scene.backdrop),
        wgpu::Extent3d {
            width: rect.w,
            height: rect.h,
            depth_or_array_layers: 1,
        },
    );
}

/// Copies the scene (read through `scene_group`) onto `view`, every
/// pixel as it is, in one pass.
fn blit(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    scene_group: &wgpu::BindGroup,
    view: &wgpu::TextureView,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("nova blit"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, scene_group, &[]);
    pass.draw(0..3, 0..1);
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
    /// The render passes the draws are split into, in order: the first
    /// clears the scene, and each OR batch opens another.
    passes: Vec<Pass>,
}

/// Lays a frame's batches out in one instance buffer, one vertex buffer
/// and a list of text batches, with what to draw for each batch in order,
/// split into passes at each OR batch. An OR batch that covers no pixel
/// is left out.
fn lay_out(frame: &Frame) -> Layout<'_> {
    let mut instances: Vec<SpriteInstance> = Vec::new();
    let mut vertices: Vec<SolidVertex> = Vec::new();
    let mut text_batches: Vec<&[TextRun]> = Vec::new();
    let mut draws = Vec::new();
    let mut passes = vec![Pass {
        backdrop: None,
        draws: 0..0,
    }];
    for batch in &frame.batches {
        if let Batch::Sprites {
            blend: Blend::Or,
            quads,
            ..
        } = batch
        {
            let Some(rect) = frame.pixels_covered(quads) else {
                continue;
            };
            passes.push(Pass {
                backdrop: Some(rect),
                draws: draws.len()..draws.len(),
            });
        }
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
        if let Some(pass) = passes.last_mut() {
            pass.draws.end = draws.len();
        }
    }
    Layout {
        instances,
        vertices,
        text: text_batches,
        draws,
        passes,
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
/// source x source alpha, keeping the destination's alpha. An OR batch's
/// shader has already combined it with the backdrop, so it is written as
/// it is.
fn blend_state(blend: Blend) -> Option<wgpu::BlendState> {
    match blend {
        Blend::Normal => Some(wgpu::BlendState::ALPHA_BLENDING),
        Blend::Or => None,
        Blend::Additive => Some(wgpu::BlendState {
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
        }),
    }
}

/// A pipeline layout of `groups`, in order.
fn pipeline_layout(
    device: &wgpu::Device,
    label: &str,
    groups: &[&wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    let groups: Vec<Option<&wgpu::BindGroupLayout>> = groups.iter().copied().map(Some).collect();
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &groups,
        immediate_size: 0,
    })
}

/// A pipeline drawing triangles from `buffers` into `format`, combined
/// with the target by `blend` (`None` writes the fragment as it is).
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    (vertex, fragment): (&str, &str),
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    (format, blend): (wgpu::TextureFormat, Option<wgpu::BlendState>),
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(vertex),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers,
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
                blend,
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
        assert_eq!(
            layout.passes,
            [Pass {
                backdrop: None,
                draws: 0..7
            }]
        );
    }

    fn sprites(blend: Blend, quads: Vec<QuadInstance>) -> Batch {
        Batch::Sprites {
            page: PageId(0),
            blend,
            quads,
        }
    }

    #[test]
    fn lay_out_breaks_the_pass_at_each_or_batch() {
        let frame = frame_of(vec![
            sprites(Blend::Normal, vec![quad(0.0)]),
            sprites(Blend::Or, vec![quad(1.0), quad(3.0)]),
            Batch::Solid(vec![solid(10.0)]),
            Batch::Text(vec![run("a")]),
            sprites(Blend::Or, vec![quad(5.0)]),
            sprites(Blend::Normal, vec![quad(6.0)]),
        ]);
        let covered = |quads: &[QuadInstance]| frame.pixels_covered(quads);

        let layout = lay_out(&frame);

        assert_eq!(
            layout.passes,
            [
                Pass {
                    backdrop: None,
                    draws: 0..1
                },
                Pass {
                    backdrop: covered(&[quad(1.0), quad(3.0)]),
                    draws: 1..4
                },
                Pass {
                    backdrop: covered(&[quad(5.0)]),
                    draws: 4..6
                },
            ]
        );
        assert_eq!(
            covered(&[quad(1.0), quad(3.0)]),
            Some(PixelRect {
                x: 2,
                y: 2,
                w: 6,
                h: 6
            })
        );
        assert_eq!(layout.draws.len(), 6);
    }

    #[test]
    fn a_frame_opening_with_an_or_batch_clears_first() {
        let frame = frame_of(vec![sprites(Blend::Or, vec![quad(2.0)])]);

        let layout = lay_out(&frame);

        assert_eq!(
            layout.passes,
            [
                Pass {
                    backdrop: None,
                    draws: 0..0
                },
                Pass {
                    backdrop: frame.pixels_covered(&[quad(2.0)]),
                    draws: 0..1
                },
            ]
        );
        assert_eq!(
            layout.draws,
            [Draw::Sprites {
                page: PageId(0),
                blend: Blend::Or,
                instances: 0..1,
            }]
        );
    }

    #[test]
    fn an_or_batch_off_the_viewport_is_left_out() {
        let frame = frame_of(vec![
            sprites(Blend::Normal, vec![quad(0.0)]),
            sprites(Blend::Or, vec![quad(40.0)]),
            sprites(Blend::Normal, vec![quad(1.0)]),
        ]);

        let layout = lay_out(&frame);

        assert_eq!(
            layout.passes,
            [Pass {
                backdrop: None,
                draws: 0..2
            }]
        );
        assert_eq!(
            layout.draws,
            [
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    instances: 0..1,
                },
                Draw::Sprites {
                    page: PageId(0),
                    blend: Blend::Normal,
                    instances: 1..2,
                },
            ]
        );
        assert_eq!(layout.instances.len(), 2);
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
    fn blend_state_adds_source_times_its_alpha_and_writes_or_as_it_is() {
        assert_eq!(
            blend_state(Blend::Normal),
            Some(wgpu::BlendState::ALPHA_BLENDING)
        );
        assert_eq!(blend_state(Blend::Or), None);
        assert_eq!(
            blend_state(Blend::Additive),
            Some(wgpu::BlendState {
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
            })
        );
    }

    #[test]
    fn the_scene_is_reused_only_at_the_targets_size() {
        assert!(reuse_scene(Some((64, 64)), (64, 64)));
        assert!(!reuse_scene(Some((64, 64)), (32, 64)));
        assert!(!reuse_scene(Some((64, 64)), (64, 32)));
        assert!(!reuse_scene(None, (64, 64)));
    }
}
