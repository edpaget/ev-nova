//! Draws `nova-view` draw lists: sprites and pictures from texture atlases,
//! text, lines and dots, in a fixed logical space letterboxed into the
//! window.
//!
//! # Core and adapters
//!
//! The core decides everything and never names a `wgpu` or `glyphon` type:
//!
//! - [`viewport`]: the logical-to-window transform and letterbox.
//! - [`atlas`]: atlas pages and the shelf packer.
//! - [`images`]: the [`ImageSource`] port, decoded frames by resource.
//! - [`gpu`]: the [`Gpu`] port and the [`Frame`]s handed to it. Each
//!   [`TextRun`] names its `nova_view` [`Font`](nova_view::Font).
//! - [`fonts`]: the font files text is drawn in ([`FontFaces`], with the
//!   bundled [`FALLBACK_FONT`] and [`SERIF_FONT`]) and which face draws
//!   each font ([`face_for`]).
//! - [`glyph_raster`]: rasterises each glyph with full hints in both
//!   directions ([`glyph_raster::HintedFace`]): Charcoal through its own
//!   TrueType instructions, as the original's system scaler did.
//! - [`batch`]: the [`Renderer`], which packs images into the atlas lazily
//!   and turns a draw list into batches in draw order, keeping flight's
//!   world layer ([`nova_view::DrawLayer`]) apart from the overlay: no
//!   batch holds both.
//! - [`model`]: the [`Mesh`]es 3D models ([`nova_view::DrawCommand::Model`])
//!   are drawn with, in sections that each take one texture or none,
//!   checked when they are made, and [`Mesh::cube`].
//! - [`model_texture`]: a model's base-colour texture ([`ModelTexture`]),
//!   decoded pixels with their mip chain built at load.
//! - [`model_sampler`]: how a section samples its texture
//!   ([`ModelSampler`]: wrap, filters and mip levels, as the model gives
//!   them), and the texel that is ([`model_sampler::sample`]), which the
//!   adapter's samplers mirror.
//! - [`model_source`]: the [`ModelSource`] port, the new game's models by
//!   asset id ([`ModelRequest`]) as [`LoadedModel`]s, placed and scaled
//!   on the view's model axes, with their materials and their base-colour
//!   textures decoded, or why not ([`ModelError`]); what of a material is
//!   drawn (its base colour and texture, and `unlit`) and what is only
//!   carried (metallic, roughness); its fake and contract check behind
//!   the `recording` feature.
//! - `planets`: the [`Renderer`]'s planets by [`nova_view::PlanetKey`]
//!   ([`Renderer::add_planet`]), each baked, if it was added unbaked, and
//!   its textures uploaded the first frame that draws it
//!   ([`Gpu::upload_planet`]), and the [`Batch::Planets`] a list's planets
//!   become.
//! - [`planet_source`]: the [`PlanetSource`] port, the new game's planets
//!   and moons by stellar ([`PlanetRequest`]) as [`LoadedPlanet`]s, a
//!   generated look left unbaked or a texture left unread for its first
//!   draw, which makes it ([`LoadedPlanet::make`]), or why not
//!   ([`PlanetError`]); its fake and contract check behind the
//!   `recording` feature.
//! - [`planet_files`]: the [`PlanetFiles`](planet_files::PlanetFiles)
//!   port, which reads an unread texture's file
//!   ([`TextureFile`](planet_files::TextureFile)) when the renderer first
//!   draws its planet; its fake and contract check behind the `recording`
//!   feature.
//! - [`camera`]: the models' orthographic camera, which puts a model where
//!   a sprite at its centre goes at the 2D view's scale, their depth
//!   bands, each later model of a batch nearer, the [`Light`] they are
//!   shaded by ([`camera::shade`]), and a fragment's whole colour: its
//!   vertex colour times its texel, lit unless unlit
//!   ([`camera::fragment`]), its alpha times a faded model's
//!   ([`camera::faded`]).
//! - [`planet`]: the geometry planets ([`nova_view::DrawCommand::Planet`])
//!   are drawn with, which the planet shader mirrors: the square a planet
//!   covers, where a fragment meets its globe, rings and atmosphere,
//!   which surface texel it shows at its turn, the shadows, and the
//!   surface's mip chain.
//! - [`sky`]: the layered sky ([`nova_view::DrawCommand::Sky`]): its
//!   layers at depth, the perspective that gives them parallax, the stars
//!   generated in the cells the view overlaps, their murk and the least
//!   size a texel lets them be drawn at, and the falloff the sky shader
//!   mirrors; a system's own look ([`sky::SkyLook`], `sky::look`), its
//!   star density and tint, and the distant nebula backdrop drawn under
//!   the stars (`sky::backdrop`); and its decorations, images drawn as
//!   far scenery over the stars ([`sky::SkyDecoration`], each a
//!   premultiplied [`SkyImage`]), placed by the core and checked against
//!   its reference ([`sky::lit`], [`sky::over`]); and the far galaxy a
//!   seed may give a sky that lists none ([`sky::galaxy`]), painted by
//!   the core.
//! - `sky_images`: the [`Renderer`]'s sky images, each uploaded the first
//!   frame that draws it ([`Gpu::upload_sky_image`]), once by its
//!   pointer, the [`Batch::SkyImages`] a sky's decorations become, and
//!   the seeded galaxy of a sky with no list, baked at its first draw.
//! - `models`: the [`Renderer`]'s meshes by [`nova_view::ModelKey`]
//!   ([`Renderer::add_model`]), uploaded the first frame that draws them
//!   ([`Gpu::upload_mesh`]) after their textures, each once
//!   ([`Gpu::upload_model_texture`]), the [`Batch::Models`] a list's
//!   models become, and the runs the adapter draws them in: a faded
//!   model's depth first, then its colour.
//! - `resident`: the one table behind both: what the [`Renderer`] has
//!   uploaded of one kind, numbered and uploaded once by its pointer.
//! - [`streams`]: the movie frames ([`nova_view::DrawCommand::MovieFrame`])
//!   the [`Renderer`] streams onto pages of their own, numbered down from
//!   the top so they never take an atlas page's id, and uploads again only
//!   when a stream's generation moves on.
//! - [`enlarge`]: the [`Enlarger`] port, the load-time scaling site: what
//!   the atlas holds for each decoded frame, which the [`Renderer`] draws
//!   at the frame's own logical size. [`NativeDetail`] holds each frame
//!   as it decodes; a new enlarger ([`Renderer::set_enlarger`]) starts the
//!   atlas afresh at the next frame ([`Gpu::clear_pages`]).
//! - [`quickdraw_stretch`]: QuickDraw's stretch, by which the original
//!   draws a picture at a size not its own: the rows and columns it shows
//!   and its merges when shrinking, by the picture's depth. The
//!   [`Renderer`] resamples a stretch
//!   to whole pixels by it wherever the GPU's sampling would show others.
//! - [`super_xbr`]: [`SuperXbr`], the enlarger that doubles each image
//!   but a pattern with Hyllian's Super-xBR, ported to the CPU.
//! - [`upscale`]: the [`Upscaler`] the adapter draws the world with
//!   ([`Gpu::set_upscaler`]), the [`UpscalerProbe`] port that says which
//!   the device can use and the list the player picks from ([`listed`]),
//!   what a device with given [`Abilities`] offers ([`offered`]) and
//!   draws a choice with ([`effective`]), the sizes MetalFX takes
//!   ([`metal_fx_takes`]) and the ones it rejected, logged once each
//!   ([`RejectedSizes`]), and the [`WorldSpace`] a frame's world layer is
//!   drawn in, from the [`WorldResolution`] the [`Gpu`] draws it at
//!   ([`world_space`]).
//! - [`render_scale`]: the [`RenderScale`] the world layer is drawn at
//!   ([`Renderer::set_render_scale`]) and the share of the backing size a percentage sizes the world
//!   texture to ([`scaled_size`]).
//! - [`source`]: [`ImageSource`] for `nova_data`'s `GameData`, a thin
//!   adapter over the picture and sprite decoders (and the mask that gives
//!   a masked picture its alpha) that needs no GPU.
//! - [`galaxy_models`]: [`ModelSource`] for the new game's data,
//!   [`GalaxyModels`], a thin adapter over `nova_galaxy_media`'s glTF
//!   reading, image decoding, placement and art budgets, read through the
//!   format's `Source` port; it needs no GPU, and `gltf` is never named
//!   here.
//! - [`galaxy_planets`]: [`PlanetSource`] for the new game's data,
//!   [`GalaxyPlanets`], a thin adapter over `nova_galaxy_media`'s planet
//!   bake, texture checks and art budgets that leaves a texture unread,
//!   and [`PlanetFiles`] for it, [`GalaxyPlanetFiles`] (over a folder,
//!   [`planet_files_in`]), which reads it at its first draw; they need no
//!   GPU.
//! - [`new_game_world`]: [`world_assets`], the new game's world models at
//!   the program's edge: the flight's table of keys and the meshes and
//!   planets they name, for the app to register, with the systems' sky
//!   looks and their skies' decorations; it needs no GPU.
//! - [`new_game_sky`]: [`sky_looks`], each new-game system's own sky look
//!   from its data, keyed by its sky's seed; it needs no GPU.
//! - [`new_game_sky_images`]: [`sky_decorations`], each new-game system's
//!   sky decorations from its data, their images read as pictures of any
//!   shape, each distinct asset once, with what failed; it needs no GPU.
//! - [`present`]: what a window surface does before each frame.
//!
//! The adapter, [`wgpu`], is the only module that names `wgpu` or
//! `glyphon` types: [`wgpu::OffscreenGpu`] draws into its own texture and
//! reads it back, and [`wgpu::SurfaceGpu`] draws into a window. Both hand
//! the work to [`wgpu::WgpuRenderer`], which draws each frame into an
//! offscreen scene and blits it to the target; an OR batch reads a backdrop
//! copy of the rectangle the core says it covers, in the pixels of the
//! canvas it draws into ([`Frame::pixels_covered`]), and a models batch
//! draws in a pass of its own with a depth attachment, multiplying by the
//! transforms the core worked out and colouring each fragment as
//! [`camera::fragment`] does (the vertex colour times the section's
//! texel, lit as [`camera::shade`] does unless unlit), so sprites,
//! placeholders and models mix in one frame in the list's order. The
//! renderer draws the
//! world with an upscaler ([`wgpu::Upscale`]): native draws it straight
//! into the scene in the frame's order, as before there were layers, and
//! [`wgpu::SharpBilinear`] draws it into a texture of the logical size
//! and enlarges that onto the viewport, the overlay over it, and the
//! world's tail ([`Frame::world_tail`]) over that at the window's
//! resolution. [`wgpu::Fsr1`] does the same with AMD's FSR 1 (EASU, then
//! RCAS) on every device. On macOS,
//! MetalFX's spatial scaler (through the `nova_metalfx` crate, the one
//! place unsafe code lives) does the same in a command buffer of its own,
//! falling back to sharp-bilinear, with one warning, for a size it won't
//! take ([`Gpu::take_warnings`]); it needs macOS 13. The core
//! turns each OR tint into exact
//! levels and keeps overlapping OR quads in separate batches. [`wgpu::GlyphonMetrics`] is
//! `nova_view`'s text-metrics port over the same glyphon font system, with
//! no GPU. Both lay text out the same way, at its logical size with every
//! glyph advancing a whole unit as QuickDraw's do, so the widgets lay text
//! out by the widths it is drawn at; lines are spaced by `nova_view`'s
//! shared line-height rule, and the renderer draws each run at the scale
//! the batcher gives it.
//!
//! The adapters are handed their fonts ([`FontFaces`]) when they are made.
//! They load those faces alone, never the system's fonts, so text draws the
//! same on every machine, and they draw each run in the face [`face_for`]
//! picks.
//!
//! Both adapters can also draw an overlay over each frame, after the
//! renderer and before it is presented or read back
//! ([`wgpu::OverlayGpu`], [`wgpu::OverlayPainter`]): [`wgpu::WithOverlay`]
//! is a [`Gpu`] that submits every frame with a painter over it. This is
//! still wgpu glue with no decisions in it, and the [`Gpu`] port is
//! unchanged.
//!
//! `recording::RecordingGpu` (this crate's tests, or the `recording`
//! feature) is a [`Gpu`] that records its calls, and `recording::NoImages`
//! an [`ImageSource`] with no images.

pub mod atlas;
pub mod batch;
mod box_filter;
pub mod camera;
pub mod enlarge;
pub mod fonts;
pub mod galaxy_models;
pub mod galaxy_planets;
pub mod glyph_raster;
pub mod gpu;
pub mod images;
pub mod livery;
pub mod model;
pub mod model_sampler;
pub mod model_source;
pub mod model_texture;
mod models;
pub mod new_game_sky;
pub mod new_game_sky_images;
pub mod new_game_world;
pub mod planet;
pub mod planet_files;
pub mod planet_source;
mod planets;
pub mod present;
pub mod quickdraw_stretch;
#[cfg(any(test, feature = "recording"))]
pub mod recording;
pub mod render_scale;
mod resident;
pub mod sky;
mod sky_images;
pub mod source;
pub mod streams;
pub mod super_xbr;
pub mod upscale;
pub mod viewport;
pub mod wgpu;

pub use atlas::{Atlas, AtlasEntry, GUTTER, PAGE_SIZE, PackError, PageId, ShelfPacker, Uv};
pub use batch::{RenderReport, Renderer};
pub use camera::Light;
pub use enlarge::{Enlarger, NativeDetail, enlarger_for};
pub use fonts::{FALLBACK_FONT, Face, FontFaces, Loaded, SERIF_FONT, face_for};
pub use galaxy_models::{GalaxyModels, ship_livery, ship_request, station_livery};
pub use galaxy_planets::{GalaxyPlanetFiles, GalaxyPlanets, planet_files_in};
pub use gpu::{
    Batch, Frame, Gpu, MAX_TEXTURE_SIDE, ModelInstance, PlanetId, PlanetImage, PlanetInstance,
    PlanetTextures, QuadInstance, Rect, SkyImageId, SkyImageInstance, SolidQuad, StarInstance,
    TextRun, WorldSpace,
};
pub use images::{ImageError, ImageSource};
pub use livery::{Livery, PaintSlot, painted};
pub use model::{Mesh, MeshError, MeshId, Section, Vertex};
pub use model_sampler::ModelSampler;
pub use model_source::{LoadedModel, ModelError, ModelRequest, ModelSource};
pub use model_texture::{ModelTexture, ModelTextureError, ModelTextureId};
pub use new_game_sky::sky_looks;
pub use new_game_sky_images::{SkyDecorations, sky_decorations};
pub use new_game_world::{WorldAssets, world_assets, world_assets_from, world_assets_in};
pub use planet_files::{PlanetFiles, TextureFile};
pub use planet_source::{LoadedPlanet, PlanetError, PlanetRequest, PlanetSource};
pub use present::{AcquireOutcome, AcquireResult, SurfaceAction, acquire_outcome, surface_action};
pub use render_scale::{RenderScale, scaled_size};
pub use sky::SkyImage;
pub use source::GameImages;
pub use super_xbr::SuperXbr;
pub use upscale::{
    Abilities, RejectedSizes, Upscaler, UpscalerProbe, WorldResolution, effective, enlarging,
    listed, metal_fx_takes, offered, resolution_of, world_space,
};
pub use viewport::{LOGICAL, Letterbox, LogicalSize, PixelRect, Viewport};
