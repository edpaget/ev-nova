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
use nova_view::{Blend, Color, DrawLayer};

use super::fonts::{Families, font_system};
use super::frame_commands::{DeviceEncoders, WgpuCommands};
use super::upscale::{Upscale, abilities, upscale_for};
use crate::fonts::FontFaces;
use crate::gpu::{Batch, Frame, PageId, TextRun, WorldSpace};
use crate::upscale::{Abilities, Upscaler, WorldResolution, offered};
use crate::viewport::PixelRect;

/// The format every frame is drawn in, whatever the target's: the scene
/// texture's, so the pipelines (and the OR composite) work on known 8-bit
/// unorm channels. Only the blit to the target uses the target's format.
pub(super) const SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A texture batches are drawn into, with the same-sized backdrop an OR
/// batch reads what is beneath it from: the scene, or a world drawn into
/// a texture of its own.
struct Canvas {
    size: (u32, u32),
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    backdrop: wgpu::Texture,
    backdrop_group: wgpu::BindGroup,
}

/// The offscreen canvas a frame is drawn into before one blit copies it
/// to the target, with the bind group the blit reads it through.
struct Scene {
    canvas: Canvas,
    blit_group: wgpu::BindGroup,
}

/// Whether a canvas of size `current` (`None` before the first frame that
/// needs one) can draw at `size`.
fn reuse_canvas(current: Option<(u32, u32)>, size: (u32, u32)) -> bool {
    current == Some(size)
}

/// Where a frame's passes draw: into `canvas`, sprites and solids
/// projected onto `content`, the first pass clearing it to `clear` when
/// given and every pass loading it otherwise.
struct PassTarget<'a> {
    canvas: &'a Canvas,
    content: PixelRect,
    clear: Option<Color>,
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

/// A render pass over a canvas (the scene, or the world's texture) and the
/// draws it encodes.
#[derive(Debug, PartialEq)]
struct Pass {
    /// For a pass opened by an OR batch, the canvas pixels the batch can
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
/// ([`Blend::Or`]) or an OR crossfade batch ([`Blend::OrCrossfade`])
/// reads what is beneath it ([`Blend::reads_backdrop`]), which a pass
/// cannot do with its own attachment, so each one ends the pass: the
/// rectangle the core says it covers ([`Frame::pixels_covered`]) is copied
/// from the canvas to its backdrop, and a new pass draws the batch with a
/// shader that ORs into, or crossfades by OR with, the backdrop's texel.
///
/// A world drawn into a texture of its own ([`WorldSpace::Texture`]) is
/// drawn first, into a world canvas of the logical size; the scene is
/// cleared, the upscaler enlarges the world onto the viewport
/// ([`Upscale::enlarge`]), and the overlay is drawn over it
/// (`lay_out` says in what order).
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    sprite_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    or_pipeline: wgpu::RenderPipeline,
    crossfade_pipeline: wgpu::RenderPipeline,
    max_pipeline: wgpu::RenderPipeline,
    solid_pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    page_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    scene: Option<Scene>,
    /// The canvas a world drawn into a texture of its own is drawn into,
    /// once one has been.
    world: Option<Canvas>,
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
    upscale: Box<dyn Upscale>,
    /// What its device can draw the world with, found once.
    abilities: Abilities,
    /// What its frames raised, for [`Self::take_warnings`].
    warnings: Vec<String>,
}

impl WgpuRenderer {
    /// A renderer drawing into `format` textures on `device`, with text in
    /// `faces` alone: no system font is loaded. `format` is the target's;
    /// only the final blit draws in it. `upscaler` draws flight's world
    /// layer.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        faces: &FontFaces,
        upscaler: Upscaler,
    ) -> Self {
        Self::with_upscale(
            device,
            queue,
            format,
            faces,
            upscale_for(upscaler, device, queue, abilities(device)),
        )
    }

    /// [`WgpuRenderer::new`], drawing the world with `upscale` (made on
    /// `device`) until an upscaler is set.
    pub(super) fn with_upscale(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        faces: &FontFaces,
        upscale: Box<dyn Upscale>,
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
        let sprite_attributes = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
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
        let fade_pipeline = sprite_pipeline_with(&or_layout, "crossfade_fs", Blend::OrCrossfade);
        let max_pipeline = sprite_pipeline_with(&sprite_layout, "max_fs", Blend::Max);
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
            crossfade_pipeline: fade_pipeline,
            max_pipeline,
            solid_pipeline,
            blit_pipeline,
            page_layout,
            texture_layout,
            scene: None,
            world: None,
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
            upscale,
            abilities: abilities(device),
            warnings: Vec::new(),
        }
    }

    /// The warnings its frames raised since it was last asked, oldest
    /// first, each once.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// The resolution it draws flight's world layer at: its upscaler's.
    #[must_use]
    pub fn world_resolution(&self) -> WorldResolution {
        self.upscale.resolution()
    }

    /// Draws flight's world layer with `upscaler` from the next frame on.
    pub fn set_upscaler(&mut self, upscaler: Upscaler) {
        self.upscale = upscale_for(upscaler, &self.device, &self.queue, self.abilities);
    }

    /// The upscalers it can draw the world with on its device: native and
    /// sharp-bilinear, which needs only a filtering sampler on RGBA8
    /// textures, as every wgpu backend has, then MetalFX on a device that
    /// has it ([`offered`]).
    #[must_use]
    pub fn upscalers(&self) -> Vec<Upscaler> {
        offered(self.abilities)
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

    /// Drops every atlas page: the next page created is page 0 again.
    pub fn clear_pages(&mut self) {
        self.pages.clear();
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
    ///
    /// A world drawn into a texture of its own is drawn into the world
    /// canvas first (its first pass clearing it to the frame's clear
    /// colour, as the scene's first pass does natively), each pass over
    /// the whole texture with the same projection; then the scene is
    /// cleared (the bars), the upscaler enlarges the world onto the
    /// viewport, and the overlay is drawn over it, an OR batch's backdrop
    /// taken from the enlarged world.
    pub fn draw(&mut self, frame: &Frame, view: &wgpu::TextureView) {
        self.ensure_scene(frame.target);
        self.queue.write_buffer(
            &self.globals,
            0,
            bytemuck::cast_slice(&globals(frame.logical)),
        );
        let layout = lay_out(frame);
        if let (false, WorldSpace::Texture { size }) = (layout.world.is_empty(), frame.world_space)
        {
            self.ensure_world(size);
        }
        self.prepare_text(frame.target, &layout.text);
        let instance_buffer = self.vertex_buffer("nova sprite instances", &layout.instances);
        let vertex_buffer = self.vertex_buffer("nova solid vertices", &layout.vertices);

        let scene = self.scene.as_ref().expect("ensured above");
        let buffers = (&instance_buffer, &vertex_buffer);
        let mut commands = WgpuCommands::new(DeviceEncoders(&self.device));
        let world = if layout.world.is_empty() {
            None
        } else {
            self.world.as_ref()
        };
        if let Some(world) = world {
            let whole = PixelRect {
                x: 0,
                y: 0,
                w: world.size.0,
                h: world.size.1,
            };
            let into_world = PassTarget {
                canvas: world,
                content: whole,
                clear: Some(frame.clear),
            };
            self.encode_passes(
                commands.encoder(),
                frame,
                &layout,
                &layout.world,
                buffers,
                &into_world,
            );
            clear(commands.encoder(), &scene.canvas.view, frame.clear);
            self.upscale.enlarge(
                &mut commands,
                &world.view,
                &scene.canvas.view,
                frame.viewport,
                &mut self.warnings,
            );
        }
        let into_scene = PassTarget {
            canvas: &scene.canvas,
            content: frame.viewport,
            clear: world.is_none().then_some(frame.clear),
        };
        self.encode_passes(
            commands.encoder(),
            frame,
            &layout,
            &layout.passes,
            buffers,
            &into_scene,
        );
        blit(
            commands.encoder(),
            &self.blit_pipeline,
            &scene.blit_group,
            view,
        );
        self.queue.submit(commands.finish());
        self.text_atlas.trim();
    }

    /// Encodes `passes` of `layout` into `target`, each OR pass's backdrop
    /// copied from the canvas first, reading sprite instances and solid
    /// vertices from `buffers`.
    fn encode_passes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &Frame,
        layout: &Layout<'_>,
        passes: &[Pass],
        buffers: (&wgpu::Buffer, &wgpu::Buffer),
        target: &PassTarget<'_>,
    ) {
        for (index, pass) in passes.iter().enumerate() {
            if let Some(rect) = pass.backdrop {
                copy_to_backdrop(encoder, target.canvas, rect);
            }
            let load = match target.clear {
                Some(color) if index == 0 => wgpu::LoadOp::Clear(clear_color(color)),
                _ => wgpu::LoadOp::Load,
            };
            let mut render = begin_pass(encoder, &target.canvas.view, load);
            let draws = &layout.draws[pass.draws.clone()];
            self.encode(&mut render, frame, draws, buffers, target);
        }
    }

    /// Encodes `draws` into `pass`, reading sprite instances and solid
    /// vertices from `buffers`, sprites and solids projected onto the
    /// target's content rectangle and an OR batch's backdrop read from its
    /// canvas. Text is drawn in target pixels.
    fn encode(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &Frame,
        draws: &[Draw],
        (instance_buffer, vertex_buffer): (&wgpu::Buffer, &wgpu::Buffer),
        target: &PassTarget<'_>,
    ) {
        let content = target.content;
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
                        Blend::Max => &self.max_pipeline,
                        Blend::OrCrossfade => &self.crossfade_pipeline,
                    });
                    pass.set_bind_group(0, &self.globals_group, &[]);
                    pass.set_bind_group(1, &page.bind_group, &[]);
                    if blend.reads_backdrop() {
                        pass.set_bind_group(2, &target.canvas.backdrop_group, &[]);
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
        let current = self.scene.as_ref().map(|scene| scene.canvas.size);
        if reuse_canvas(current, target) {
            return;
        }
        let (canvas, blit_group) = self.canvas("nova scene", target);
        self.scene = Some(Scene { canvas, blit_group });
    }

    /// Makes sure the world canvas is `size`, creating it on the first
    /// frame that draws the world into a texture and again whenever the
    /// logical size changes.
    fn ensure_world(&mut self, size: (u32, u32)) {
        let current = self.world.as_ref().map(|world| world.size);
        if reuse_canvas(current, size) {
            return;
        }
        let (canvas, _) = self.canvas("nova world", size);
        self.world = Some(canvas);
    }

    /// A `size` canvas, and a bind group that reads its texture with
    /// `textureLoad` (the blit's, for the scene).
    fn canvas(&self, label: &str, size: (u32, u32)) -> (Canvas, wgpu::BindGroup) {
        let (texture, view, group) = self.scene_texture(
            label,
            size,
            // `COPY_DST`: an upscaler that scales into a texture of its
            // own (MetalFX) copies the result onto the scene.
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        );
        let (backdrop, _, backdrop_group) = self.scene_texture(
            label,
            size,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let canvas = Canvas {
            size,
            texture,
            view,
            backdrop,
            backdrop_group,
        };
        (canvas, group)
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

/// Begins a render pass over `view`, loading it as `load` says.
fn begin_pass<'e>(
    encoder: &'e mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("nova frame"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
    })
}

/// Clears `view` to `color` with an empty pass.
fn clear(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: Color) {
    begin_pass(encoder, view, wgpu::LoadOp::Clear(clear_color(color)));
}

/// Copies `rect` of the canvas to the same place in its backdrop.
fn copy_to_backdrop(encoder: &mut wgpu::CommandEncoder, canvas: &Canvas, rect: PixelRect) {
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
        at(&canvas.texture),
        at(&canvas.backdrop),
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
#[derive(Default)]
struct Layout<'a> {
    /// Every sprite batch's instances, in one buffer.
    instances: Vec<SpriteInstance>,
    /// Every solid batch's vertices, in one buffer.
    vertices: Vec<SolidVertex>,
    /// The text batches, one glyphon renderer each.
    text: Vec<&'a [TextRun]>,
    /// What to draw for each batch, in order.
    draws: Vec<Draw>,
    /// For a world drawn into a texture of its own, the render passes its
    /// draws are split into, in order: the first clears the texture, and
    /// each OR batch opens another. None for a world drawn into the
    /// target, or a frame with no world.
    world: Vec<Pass>,
    /// The render passes over the scene the other draws are split into,
    /// in order: the first clears the scene (unless the world's passes
    /// come first), and each OR batch opens another.
    passes: Vec<Pass>,
}

impl<'a> Layout<'a> {
    /// Lays out the batch at `index` of `frame` at the end of `passes`,
    /// opening a pass for an OR batch, which is left out when it covers
    /// no pixel in its layer.
    fn push(&mut self, frame: &'a Frame, index: usize, passes: &mut Vec<Pass>) {
        let batch = &frame.batches[index];
        if let Batch::Sprites { blend, quads, .. } = batch
            && blend.reads_backdrop()
        {
            let Some(rect) = frame.pixels_covered(frame.layer(index), quads) else {
                return;
            };
            passes.push(Pass {
                backdrop: Some(rect),
                draws: self.draws.len()..self.draws.len(),
            });
        }
        let draw = match batch {
            Batch::Sprites { page, blend, quads } => {
                let start = self.instances.len() as u32;
                self.instances
                    .extend(quads.iter().map(SpriteInstance::from));
                Draw::Sprites {
                    page: *page,
                    blend: *blend,
                    instances: start..self.instances.len() as u32,
                }
            }
            Batch::Solid(quads) => {
                let start = self.vertices.len() as u32;
                self.vertices.extend(quads.iter().flat_map(solid_vertices));
                Draw::Solid {
                    vertices: start..self.vertices.len() as u32,
                }
            }
            Batch::Text(runs) => {
                self.text.push(runs);
                Draw::Text {
                    renderer: self.text.len() - 1,
                }
            }
        };
        self.draws.push(draw);
        if let Some(pass) = passes.last_mut() {
            pass.draws.end = self.draws.len();
        }
    }

    /// A pass opening at the next draw, with no backdrop.
    fn fresh_pass(&self) -> Pass {
        Pass {
            backdrop: None,
            draws: self.draws.len()..self.draws.len(),
        }
    }
}

/// Lays a frame's batches out in one instance buffer, one vertex buffer
/// and a list of text batches, with what to draw for each batch, split
/// into passes at each OR batch. An OR batch that covers no pixel in its
/// layer ([`Frame::pixels_covered`]: texels for a world drawn into a
/// texture, target pixels otherwise) is left out.
///
/// A world drawn into the target ([`WorldSpace::Target`]) is laid out in
/// the frame's order with the overlay, as if there were no layers.
///
/// A world drawn into a texture of its own ([`WorldSpace::Texture`]) is
/// laid out first, in passes of its own ([`Layout::world`]), its batches
/// in the frame's order; then the whole overlay, in the frame's order,
/// into the scene the world is enlarged into. That is the only order a
/// texture world allows without a transparent world texture, which cannot
/// work: additive and max world batches keep the destination's alpha, so
/// over a transparent clear they would vanish when composited. So an
/// overlay batch that comes before the world in the frame (a mission
/// dialog drawn before flight while its map is open over it) is drawn
/// over the world here, where natively the world draws over it; and a
/// second world run after an overlay run is drawn with the first, under
/// that overlay. This differs from native on purpose
/// (rdm:phase/display-modes-fidelity/phase-5-paused-flight-under-mission-dialogs
/// tracks it). Text is never world (the batcher keeps it in the overlay),
/// so it is always laid out with the overlay; no rule here singles it
/// out.
fn lay_out(frame: &Frame) -> Layout<'_> {
    let mut layout = Layout::default();
    let indices = 0..frame.batches.len();
    let textured = matches!(frame.world_space, WorldSpace::Texture { .. });
    let in_world = |&index: &usize| textured && frame.layer(index) == DrawLayer::World;
    let mut world = Vec::new();
    if indices.clone().any(|index| in_world(&index)) {
        world.push(layout.fresh_pass());
        for index in indices.clone().filter(in_world) {
            layout.push(frame, index, &mut world);
        }
    }
    let mut passes = vec![layout.fresh_pass()];
    for index in indices.filter(|index| !in_world(index)) {
        layout.push(frame, index, &mut passes);
    }
    layout.world = world;
    layout.passes = passes;
    layout
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
/// it is. Max takes the larger of each colour channel of the destination
/// and the source, which its shader has premultiplied, keeping the
/// destination's alpha.
fn blend_state(blend: Blend) -> Option<wgpu::BlendState> {
    match blend {
        Blend::Normal => Some(wgpu::BlendState::ALPHA_BLENDING),
        Blend::Or | Blend::OrCrossfade => None,
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
        // WebGPU requires `One` factors for `Max`, so `max_fs` scales the
        // colour by its alpha itself.
        Blend::Max => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
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
pub(super) fn pipeline_layout(
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
pub(super) fn pipeline(
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
    use nova_view::{Blend, Color, DrawLayer, Point};

    use super::*;
    use crate::gpu::{QuadInstance, Rect, SolidQuad, WorldSpace};
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
            colorize: crate::gpu::NO_COLORIZE,
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
            world_space: WorldSpace::Target,
            world: Vec::new(),
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
        let covered = |quads: &[QuadInstance]| frame.pixels_covered(DrawLayer::Overlay, quads);

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
    fn lay_out_breaks_the_pass_at_each_crossfade_batch_as_at_an_or_batch() {
        let frame = frame_of(vec![
            sprites(Blend::Normal, vec![quad(0.0)]),
            sprites(Blend::OrCrossfade, vec![quad(1.0), quad(3.0)]),
            sprites(Blend::Normal, vec![quad(6.0)]),
        ]);
        let covered = |quads: &[QuadInstance]| frame.pixels_covered(DrawLayer::Overlay, quads);

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
                    draws: 1..3
                },
            ]
        );
        assert!(covered(&[quad(1.0), quad(3.0)]).is_some());
        assert_eq!(
            layout.draws[1],
            Draw::Sprites {
                page: PageId(0),
                blend: Blend::OrCrossfade,
                instances: 1..3,
            }
        );
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
                    backdrop: frame.pixels_covered(DrawLayer::Overlay, &[quad(2.0)]),
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
            world_space: WorldSpace::Target,
            batches,
            world: Vec::new(),
        }
    }

    /// Asserts that `a` and `b` lay out alike: the same buffers, text,
    /// draws and passes, the world's and the scene's, backdrops included.
    #[track_caller]
    fn assert_same_layout(a: &Layout<'_>, b: &Layout<'_>) {
        assert_eq!(a.instances, b.instances);
        assert_eq!(a.vertices, b.vertices);
        assert_eq!(a.text, b.text);
        assert_eq!(a.draws, b.draws);
        assert_eq!(a.world, b.world);
        assert_eq!(a.passes, b.passes);
    }

    /// Frames whose batches are marked world in different places: an OR
    /// batch first in the world, an OR batch first in the overlay over a
    /// world quad, a world between two overlays, and a world first. Text
    /// is never world (the batcher keeps it in the overlay), so a world
    /// run that met text is split round it.
    fn layered_frames() -> Vec<Frame> {
        let batches = || {
            vec![
                sprites(Blend::Or, vec![quad(1.0)]),
                sprites(Blend::Normal, vec![quad(2.0), quad(3.0)]),
                Batch::Text(vec![run("a")]),
                Batch::Solid(vec![solid(4.0)]),
                sprites(Blend::Or, vec![quad(2.0)]),
                sprites(Blend::Normal, vec![quad(5.0)]),
                Batch::Text(vec![run("b")]),
            ]
        };
        [vec![0, 1, 3], vec![1, 3], vec![1, 3, 5], vec![0, 1]]
            .into_iter()
            .map(|world| Frame {
                world,
                ..frame_of(batches())
            })
            .collect()
    }

    /// `frame` with its world drawn into a texture of the logical size.
    fn textured(frame: Frame) -> Frame {
        Frame {
            world_space: WorldSpace::Texture { size: (32, 32) },
            ..frame
        }
    }

    #[test]
    fn no_frame_marks_text_as_world() {
        for frame in layered_frames() {
            for &at in &frame.world {
                assert!(!matches!(frame.batches[at], Batch::Text(_)), "{at}");
            }
        }
    }

    #[test]
    fn a_native_world_lays_out_in_the_frames_order_as_with_no_world() {
        for frame in layered_frames() {
            let unlayered = Frame {
                world: Vec::new(),
                ..frame.clone()
            };
            assert_same_layout(&lay_out(&frame), &lay_out(&unlayered));
        }
    }

    #[test]
    fn a_native_worlds_or_backdrop_is_in_target_pixels() {
        let frame = Frame {
            world: vec![0],
            ..frame_of(vec![sprites(Blend::Or, vec![quad(2.0)])])
        };
        let backdrop = lay_out(&frame).passes[1].backdrop;
        assert_eq!(
            backdrop,
            Some(PixelRect {
                x: 4,
                y: 4,
                w: 2,
                h: 2
            })
        );
    }

    #[test]
    fn a_native_world_has_no_passes_of_its_own() {
        for frame in layered_frames() {
            assert_eq!(lay_out(&frame).world, []);
        }
    }

    /// The draws `passes` encode, in order.
    fn drawn<'a>(layout: &'a Layout<'_>, passes: &[Pass]) -> Vec<&'a Draw> {
        passes
            .iter()
            .flat_map(|pass| &layout.draws[pass.draws.clone()])
            .collect()
    }

    #[test]
    fn a_textured_world_lays_out_first_in_its_own_passes_then_the_overlay_in_order() {
        for frame in layered_frames() {
            let texture_frame = textured(frame.clone());
            let layout = lay_out(&texture_frame);
            // The same batches, the world's first and then the overlay's,
            // each in the frame's order, laid out natively in that order
            // and with no OR batch, give the same draws and buffers.
            let (world, overlay): (Vec<usize>, Vec<usize>) =
                (0..frame.batches.len()).partition(|&at| frame.layer(at) == DrawLayer::World);
            let plain = |at: usize| match &frame.batches[at] {
                Batch::Sprites { page, quads, .. } => Batch::Sprites {
                    page: *page,
                    blend: Blend::Normal,
                    quads: quads.clone(),
                },
                other => other.clone(),
            };
            let reordered_frame =
                frame_of(world.iter().chain(&overlay).map(|&at| plain(at)).collect());
            let reordered = lay_out(&reordered_frame);
            assert_eq!(layout.instances, reordered.instances);
            assert_eq!(layout.vertices, reordered.vertices);
            assert_eq!(layout.text, reordered.text);
            let blends: Vec<Option<Blend>> = world
                .iter()
                .chain(&overlay)
                .map(|&at| match frame.batches[at] {
                    Batch::Sprites { blend, .. } => Some(blend),
                    _ => None,
                })
                .collect();
            assert_eq!(layout.draws.len(), blends.len());
            for ((draw, plain), blend) in layout.draws.iter().zip(&reordered.draws).zip(blends) {
                match (draw, plain, blend) {
                    (
                        Draw::Sprites {
                            page,
                            blend,
                            instances,
                        },
                        Draw::Sprites {
                            page: p,
                            instances: i,
                            ..
                        },
                        Some(b),
                    ) => assert_eq!((page, blend, instances), (p, &b, i)),
                    _ => assert_eq!(draw, plain),
                }
            }
            // The world's passes hold the world's draws, and the scene's
            // the overlay's.
            let draws: Vec<&Draw> = layout.draws.iter().collect();
            assert_eq!(drawn(&layout, &layout.world), draws[..world.len()]);
            assert_eq!(drawn(&layout, &layout.passes), draws[world.len()..]);
            assert_eq!(layout.world[0].backdrop, None, "clears the world first");
            assert_eq!(layout.passes[0].backdrop, None);
        }
    }

    #[test]
    fn an_overlay_before_a_textured_world_lays_out_after_it() {
        // Overlay OR, world, overlay text, world solid, world OR, overlay
        // sprite, overlay text: as a dialog's map over flight draws.
        let frame = textured(Frame {
            world: vec![1, 3, 4],
            ..layered_frames().remove(0)
        });

        let layout = lay_out(&frame);

        let sprite = |blend, instances| Draw::Sprites {
            page: PageId(0),
            blend,
            instances,
        };
        assert_eq!(
            layout.draws,
            [
                sprite(Blend::Normal, 0..2),
                Draw::Solid { vertices: 0..6 },
                sprite(Blend::Or, 2..3),
                sprite(Blend::Or, 3..4),
                Draw::Text { renderer: 0 },
                sprite(Blend::Normal, 4..5),
                Draw::Text { renderer: 1 },
            ]
        );
        let instances: Vec<SpriteInstance> = [2.0, 3.0, 2.0, 1.0, 5.0]
            .map(|n| SpriteInstance::from(&quad(n)))
            .to_vec();
        assert_eq!(layout.instances, instances, "one buffer, the world's first");
        assert_eq!(layout.vertices, solid_vertices(&solid(4.0)).to_vec());
        let rect = |x, y, w, h| Some(PixelRect { x, y, w, h });
        assert_eq!(
            layout.world,
            [
                Pass {
                    backdrop: None,
                    draws: 0..2
                },
                // quad(2.0) is (2, 2)-(3, 3): one texel a unit.
                Pass {
                    backdrop: rect(2, 2, 1, 1),
                    draws: 2..3
                },
            ]
        );
        assert_eq!(
            layout.passes,
            [
                Pass {
                    backdrop: None,
                    draws: 3..3
                },
                // quad(1.0) is (1, 1)-(2, 2): two target pixels a unit.
                Pass {
                    backdrop: rect(2, 2, 2, 2),
                    draws: 3..7
                },
            ]
        );
    }

    #[test]
    fn a_textured_world_or_batch_covering_no_texel_is_left_out() {
        let frame = textured(Frame {
            world: vec![0, 1, 2],
            ..frame_of(vec![
                sprites(Blend::Normal, vec![quad(0.0)]),
                sprites(Blend::Or, vec![quad(40.0)]),
                sprites(Blend::Normal, vec![quad(1.0)]),
            ])
        });

        let layout = lay_out(&frame);

        assert_eq!(
            layout.world,
            [Pass {
                backdrop: None,
                draws: 0..2
            }]
        );
        assert_eq!(layout.instances.len(), 2);
        assert_eq!(
            layout.passes,
            [Pass {
                backdrop: None,
                draws: 2..2
            }]
        );
    }

    #[test]
    fn a_textured_frame_with_no_world_lays_out_as_a_native_one() {
        let frame = frame_of(layered_frames().remove(0).batches);
        assert_same_layout(&lay_out(&textured(frame.clone())), &lay_out(&frame));
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
            blend_state(Blend::OrCrossfade),
            None,
            "its shader combines it with the backdrop"
        );
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
    fn blend_state_takes_the_larger_of_each_channel_and_keeps_the_alpha_for_max() {
        // WebGPU requires `One` factors for `Max`; `max_fs` has already
        // scaled the colour by its alpha.
        assert_eq!(
            blend_state(Blend::Max),
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Max,
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
    fn a_canvas_is_reused_only_at_its_own_size() {
        assert!(reuse_canvas(Some((64, 64)), (64, 64)));
        assert!(!reuse_canvas(Some((64, 64)), (32, 64)));
        assert!(!reuse_canvas(Some((64, 64)), (64, 32)));
        assert!(!reuse_canvas(None, (64, 64)));
    }
}
