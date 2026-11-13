//! The model source port: the new game's 3D models, by asset id, as
//! meshes the renderer can take, with their materials and textures.
//! Planets and moons, generated or textured, have a port of their own
//! ([`crate::planet_source`]).
//!
//! [`ModelSource::model`] answers a [`ModelRequest`] (an asset id, and
//! the length in metres to scale it to) with a [`LoadedModel`] or a
//! [`ModelError`]. A loaded model is placed the way the 2D view places a
//! ship's sprite: centred on its middle (in height too), its nose up the
//! screen (−y, heading 0) and its top toward the camera (+z), one model
//! unit a metre. [`view_axes`] maps the ship space a model is placed in
//! (`[starboard, nose, up]`) to the view's model axes
//! (`nova_view::model`). That map is a reflection, as the view's axes
//! are left-handed; the models pipeline culls nothing, and normals take
//! the same map, so only normals need it.
//!
//! Each [`Part`] is one primitive of the model as one node draws it,
//! keeping its material, so a livery can repaint the parts drawn in a
//! material its source put in a [`PaintSlot`] ([`crate::livery`]) and
//! merge them again ([`LoadedModel::mesh`]). Its vertex colours are
//! its material's base colour, made sRGB ([`vertex_color`]), as the
//! models target is not sRGB and the shader writes vertex colour
//! straight.
//!
//! # What is drawn of a material
//!
//! - **Its base-colour texture.** A source decodes each image some part
//!   samples ([`Part::sampling_material`]: the base colour of a material
//!   on a part with texture coordinates) and no other ([`Texture`], a
//!   [`ModelTexture`] with its mip chain). The merged mesh draws each run
//!   of parts on one texture as a section, and holds only the textures
//!   its sections name ([`LoadedModel::mesh`]), so a texture nothing
//!   samples is neither decoded nor uploaded. A
//!   fragment is its vertex colour times its texel, all four channels
//!   ([`crate::camera::fragment`]). The product is taken in the gamma
//!   space the vertex colour is baked in, the texel's sRGB bytes as they
//!   are, not in linear light as glTF defines it. It is exact when either
//!   is white (a white factor under a texture, or no texture), and close
//!   otherwise, as the sRGB curve is nearly a power law and a power law
//!   is multiplicative. Decoding to linear in the shader would mean
//!   rebaking every vertex colour for a difference in the curve's toe.
//! - **Its base colour's sampler.** A texture is sampled as the model
//!   says ([`Material::base_color_sampler`], a [`ModelSampler`]): its
//!   wrap along each axis, its filters and the mip levels it reads, as
//!   [`crate::model_sampler::sample`] defines them. A model that says
//!   nothing repeats past the texture's edges and is trilinear
//!   ([`ModelSampler::DEFAULT`]). The sampler travels with each section
//!   of the merged mesh, so two materials may draw one texture two ways
//!   and still share its one upload.
//! - **Its base colour's coordinates.** [`Part::uv`] is the coordinate
//!   set the base colour names (`texCoord`), with its
//!   `KHR_texture_transform` applied: the source resolves both when it
//!   reads the model, so the coordinates here are final and nothing later
//!   applies either. A part whose named set is missing has no
//!   coordinates, and draws untextured.
//! - **`unlit`.** An unlit material's parts draw at that product whatever
//!   the light; every other part is lit by it ([`crate::camera::shade`]).
//! - **Metallic and roughness are carried and unused.** The light is one
//!   Lambert term and an ambient one, which neither factor enters. They
//!   stay on [`Material`] for a lighting model that grows to use them.
//!
//! Not read from the model, and so not drawn: the metallic-roughness,
//! normal, occlusion and emissive textures. What a fragment's alpha does
//! once drawn (alpha mode, cutoff, blending order) is not this module's
//! either.
//!
//! # Where an addition goes
//!
//! - Something a caller must say about the model it wants (a level of
//!   detail, say): a field of [`ModelRequest`].
//! - A new way a model can't be had: a [`ModelError`] variant, its
//!   message naming the asset (and the file, or the image by its
//!   [`ImageRef`]). A failure only the model source can find (it alone
//!   decodes images) gets a variant of its own, never
//!   [`ModelError::Unreadable`], which callers take as already reported
//!   by whoever read the model first.
//! - Something more read from the model: a field of [`Part`],
//!   [`Material`] or [`Texture`], filled by the adapter
//!   ([`crate::galaxy_models`]) from the media crate's reading.
//! - Something more drawn of a part: a field of [`Vertex`] or of a mesh's
//!   [`Section`], written by [`LoadedModel::mesh`] alone (a part's own
//!   mesh carries neither texture coordinates nor the unlit flag), its
//!   effect on a fragment defined by [`crate::camera::fragment`] and
//!   mirrored by the shader.
//! - A new property of how a texture is sampled: a field of
//!   [`ModelSampler`], defined by [`crate::model_sampler::sample`] and
//!   mirrored by the wgpu adapter's `descriptor`, never a new argument
//!   or method of the [`Gpu`](crate::Gpu) port.
//! - A new livery slot: a [`PaintSlot`] variant and a field of
//!   [`Livery`](crate::livery::Livery), and the adapter's reading of the
//!   media crate's slot (see [`crate::livery`]).
//!
//! The decisions an adapter forwards to live in core functions here
//! ([`vertex_color`], [`linear`], [`part`]), in the view
//! ([`view_axes`], forwarded from `nova_view`) or in `nova_galaxy_media`,
//! and are pinned by their own tests. `FakeModels` is the hand-written
//! fake (with this crate's tests, or the `recording` feature), and
//! `check_contract` the contract every source meets: the fake runs it in
//! this module's tests, and the galaxy adapter runs it on every model of
//! the sample.

use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

/// The ship-to-view axes map is the view's
/// ([`nova_view::view_axes`]): forwarded here, so the mesh's vertices and
/// what the view places on a ship take one map.
pub use nova_view::view_axes;

pub use crate::livery::PaintSlot;
use crate::model::{Mesh, MeshError, Section, Vertex};
use crate::model_sampler::ModelSampler;
use crate::model_texture::ModelTexture;

/// A model wanted: the asset (`assets/<id>.md`), and how long to make it.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRequest {
    /// The asset's id.
    pub asset: String,
    /// Its length nose to tail in metres: a ship's size from data. `None`
    /// keeps a metre to each of the model's units (a station's or an
    /// outfit's model, which has no size of its own).
    pub length: Option<f32>,
}

impl ModelRequest {
    /// The model `asset`, `length` metres long (or as modelled).
    #[must_use]
    pub fn new(asset: &str, length: Option<f32>) -> Self {
        ModelRequest {
            asset: asset.to_owned(),
            length,
        }
    }
}

/// One primitive of a model as one node draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    /// Its triangles, placed (see the [module docs](self)), each vertex
    /// coloured by its material's base colour.
    pub mesh: Mesh,
    /// Its material, as an index into [`LoadedModel::materials`].
    pub material: Option<usize>,
    /// Each vertex's texture coordinates, if it has them: one per vertex
    /// of [`mesh`](Self::mesh), as its material's base colour is sampled
    /// by them (the set it names, transformed; see the
    /// [module docs](self)).
    pub uv: Option<Vec<[f32; 2]>>,
}

impl Part {
    /// The material whose base-colour texture this part samples: its
    /// material, if it has texture coordinates to sample by. Without
    /// coordinates, or without a material, it samples none.
    ///
    /// This is the one definition of "sampled". A texture is sampled when
    /// some part's sampling material names it as its base colour:
    /// [`LoadedModel::mesh`] draws only those, and a source decodes only
    /// those.
    #[must_use]
    pub fn sampling_material(&self) -> Option<usize> {
        self.material.filter(|_| self.uv.is_some())
    }
}

/// A model's material.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Its name, if it has one: what a livery's `paint` names.
    pub name: Option<String>,
    /// The livery slot that paints it ([`crate::livery`]), if any: filled
    /// by the source from the model's `paint` (the galaxy adapter reads
    /// it through `nova_galaxy_media::slot_of`). A material with no name
    /// has none.
    pub paint: Option<PaintSlot>,
    /// Its base colour, linear RGBA from 0 to 1, as glTF writes it.
    pub base_color: [f32; 4],
    /// Its metallic factor.
    pub metallic: f32,
    /// Its roughness factor.
    pub roughness: f32,
    /// Whether it is unlit.
    pub unlit: bool,
    /// The texture its base colour takes, as an index into
    /// [`LoadedModel::textures`].
    pub base_color_texture: Option<usize>,
    /// How its base colour samples that texture, as the model gives it.
    /// Two materials may name one texture through two samplers. A
    /// material with no base-colour texture carries
    /// [`ModelSampler::DEFAULT`].
    pub base_color_sampler: ModelSampler,
}

impl Material {
    /// A material of the linear `base_color` that says nothing else: no
    /// name and no paint slot, glTF's metallic and roughness of 1, lit,
    /// and no texture. A literal names the fields it means and takes the
    /// rest from here, so a new field doesn't touch it.
    #[must_use]
    pub fn new(base_color: [f32; 4]) -> Self {
        Material {
            name: None,
            paint: None,
            base_color,
            metallic: 1.0,
            roughness: 1.0,
            unlit: false,
            base_color_texture: None,
            base_color_sampler: ModelSampler::DEFAULT,
        }
    }
}

/// A model's base-colour texture: an image some part samples
/// ([`Part::sampling_material`]), decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    /// Its pixels and their mip chain, shared: the meshes made of this
    /// model, in whatever livery, hold this pointer, and so share one
    /// upload.
    pub image: Arc<ModelTexture>,
    /// The data-folder path it was read from; `None` when embedded.
    pub path: Option<String>,
}

/// A model, placed and scaled, ready for the renderer.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedModel {
    /// Its primitives, each with its material.
    pub parts: Vec<Part>,
    /// Its materials.
    pub materials: Vec<Material>,
    /// Its base-colour textures: only the images some part samples
    /// ([`Part::sampling_material`]), as a source owes it (the port's
    /// contract). One held here that nothing samples costs its decode and
    /// its memory, and [`mesh`](Self::mesh) leaves it out.
    pub textures: Vec<Texture>,
}

impl LoadedModel {
    /// Its parts merged into the one mesh
    /// [`Renderer::add_model`](crate::Renderer::add_model) takes, which
    /// holds the textures its sections name by the pointers this model
    /// holds, in this model's order, and no others: a texture no part
    /// samples is left out, so it is never uploaded, and each section
    /// names its texture by its place among those kept.
    ///
    /// The merge writes what a part's own mesh doesn't carry: each
    /// vertex's texture coordinates (from [`Part::uv`]) and whether it is
    /// unlit (its part's material's). It makes one [`Section`] for each
    /// run of consecutive parts that draw the same texture through the
    /// same sampler, so a model with no texture is one section. A part
    /// draws its material's texture, through its material's sampler,
    /// only if it has texture coordinates to sample it by
    /// ([`Part::sampling_material`]); without them,
    /// or without a material, it draws untextured, in its vertex colours
    /// alone, and its section carries the default sampler, so untextured
    /// runs merge whatever their materials say.
    #[must_use]
    pub fn mesh(&self) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut sections: Vec<Section> = Vec::new();
        for part in &self.parts {
            let material = part.material.and_then(|at| self.materials.get(at));
            let unlit = material.is_some_and(|m| m.unlit);
            let drawn = part
                .sampling_material()
                .and_then(|at| self.materials.get(at))
                .and_then(|m| Some((m.base_color_texture?, m.base_color_sampler)))
                .filter(|&(at, _)| at < self.textures.len());
            let (texture, sampler) = match drawn {
                Some((at, sampler)) => (Some(at), sampler),
                None => (None, ModelSampler::DEFAULT),
            };
            let base = u32::try_from(vertices.len()).expect("a model's vertices fit in u32");
            let uvs = part.uv.as_deref().unwrap_or(&[]);
            vertices.extend(
                part.mesh
                    .vertices()
                    .iter()
                    .enumerate()
                    .map(|(at, v)| Vertex {
                        uv: uvs.get(at).copied().unwrap_or([0.0; 2]),
                        unlit,
                        ..*v
                    }),
            );
            let start = u32::try_from(indices.len()).expect("a model's indices fit in u32");
            indices.extend(part.mesh.indices().iter().map(|i| base + i));
            let end = u32::try_from(indices.len()).expect("a model's indices fit in u32");
            match sections.last_mut() {
                Some(last) if last.texture == texture && last.sampler == sampler => {
                    last.indices.end = end;
                }
                _ => sections.push(Section::new(start..end, texture).sampled(sampler)),
            }
        }
        let textures = sampled_only(&mut sections, &self.textures);
        Mesh::textured(vertices, indices, textures, sections)
            .expect("each part's indices are in its own range, and its section in order")
    }
}

/// The textures of `textures` that `sections` name, by the pointers given
/// and in their order there, with each section renumbered to name its
/// texture among those kept. Every section's texture must be in
/// `textures`.
fn sampled_only(sections: &mut [Section], textures: &[Texture]) -> Vec<Arc<ModelTexture>> {
    let named: BTreeSet<usize> = sections.iter().filter_map(|s| s.texture).collect();
    for section in sections {
        section.texture = section.texture.map(|at| named.range(..at).count());
    }
    named
        .into_iter()
        .map(|at| Arc::clone(&textures[at].image))
        .collect()
}

/// Which of a model's images an error is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageRef {
    /// A file beside the model, by its data-folder path.
    File(String),
    /// An image embedded in the model, by its index among the model's
    /// images.
    Embedded(usize),
}

impl fmt::Display for ImageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageRef::File(path) => write!(f, "image `{path}`"),
            ImageRef::Embedded(index) => write!(f, "embedded image {index}"),
        }
    }
}

/// What a budget limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Measure {
    /// Triangles drawn.
    Triangles,
    /// KiB on disk.
    Kib,
    /// A texture's longest side, in pixels.
    Side,
}

/// One limit of its art budget a model or a planet's texture breaks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overrun {
    /// What is over.
    pub measure: Measure,
    /// What it uses: triangles, bytes for [`Measure::Kib`], or pixels
    /// for [`Measure::Side`].
    pub used: u64,
    /// The limit: triangles, KiB, or pixels.
    pub most: u32,
    /// The budget's file, `budgets/<id>.md`.
    pub budget: String,
}

impl fmt::Display for Overrun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (most, budget) = (self.most, &self.budget);
        match self.measure {
            Measure::Triangles => write!(
                f,
                "draws {} triangles, over the {most} budget in `{budget}`",
                self.used
            ),
            Measure::Kib => write!(
                f,
                "is {} KiB, over the {most} KiB budget in `{budget}`",
                self.used.div_ceil(1024)
            ),
            Measure::Side => write!(
                f,
                "is {} pixels on its longest side, over the {most} pixel budget in `{budget}`",
                self.used
            ),
        }
    }
}

/// Why a model can't be had. The caller keeps drawing its placeholder.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ModelError {
    /// The manifest has no such asset.
    #[error("no asset `{0}` in the manifest")]
    NoAsset(String),
    /// The asset is a texture, not a model.
    #[error("asset `{0}` is a texture, not a model")]
    NotAModel(String),
    /// The asset names no model file: a placeholder, or no `.glb` or
    /// `.gltf` among its files.
    #[error("asset `{0}` has no model file")]
    NoFile(String),
    /// Its model file is a Git LFS pointer the checkout hasn't fetched.
    #[error("asset `{0}`'s model file is a Git LFS pointer, not fetched")]
    LfsPointer(String),
    /// Its model file isn't there.
    #[error("asset `{asset}`'s model file `{file}` is missing")]
    Missing {
        /// The asset.
        asset: String,
        /// The missing file.
        file: String,
    },
    /// Its model file doesn't read as a model.
    #[error("asset `{asset}`'s model doesn't read: {why}")]
    Unreadable {
        /// The asset.
        asset: String,
        /// Why, naming the file.
        why: String,
    },
    /// The model is over its art budget: its triangles or its files.
    #[error("asset `{asset}` is over its art budget: {}", list(over))]
    OverBudget {
        /// The asset.
        asset: String,
        /// Every limit it breaks.
        over: Vec<Overrun>,
    },
    /// One of the images a part samples ([`Part::sampling_material`]) is
    /// over the art budget for a
    /// texture: its longest side ([`Measure::Side`]). It is not
    /// [`OverBudget`](Self::OverBudget), which says nothing of which
    /// image to shrink.
    #[error("asset `{asset}`'s {image} {}", list(over))]
    TextureOverBudget {
        /// The asset.
        asset: String,
        /// The image.
        image: ImageRef,
        /// Every limit the image breaks.
        over: Vec<Overrun>,
    },
    /// One of the images a part samples doesn't decode: it is neither a
    /// PNG nor a JPEG, its header gives no size, or its data is cut short
    /// or corrupt. An image nothing samples is never decoded, so it is
    /// never this error. The model itself read, so this is not
    /// [`Unreadable`](Self::Unreadable): nothing before the model source
    /// decodes a model's images.
    #[error("asset `{asset}`'s {image} doesn't decode: {why}")]
    TextureUndecodable {
        /// The asset.
        asset: String,
        /// The image.
        image: ImageRef,
        /// Why.
        why: String,
    },
    /// One of the images a part samples is larger than the renderer draws
    /// ([`crate::gpu::MAX_TEXTURE_SIDE`]), whatever its budget allows.
    #[error(
        "asset `{asset}`'s {image} is {side} pixels on its longest side, over the {most} the \
         renderer draws"
    )]
    TextureTooLarge {
        /// The asset.
        asset: String,
        /// The image.
        image: ImageRef,
        /// Its longest side in pixels.
        side: u32,
        /// The most the renderer draws.
        most: u32,
    },
    /// The model has no depth along its `front` (a flat panel facing
    /// that way), so it can't be scaled to the length asked for.
    #[error(
        "asset `{0}`'s model has no depth along its `front`, so it can't be scaled to a \
         length: check its `front`"
    )]
    NoDepth(String),
    /// The model's triangles don't make a mesh.
    #[error("asset `{asset}`'s model makes no mesh: {error}")]
    Mesh {
        /// The asset.
        asset: String,
        /// Why.
        error: MeshError,
    },
}

/// `over`, joined with `; `.
fn list(over: &[Overrun]) -> String {
    over.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// The new game's models, in the renderer's terms.
pub trait ModelSource {
    /// The model `request.asset` names, placed and scaled (see the
    /// [module docs](self)).
    fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError>;
}

/// A borrowed source is a source.
impl<S: ModelSource + ?Sized> ModelSource for &S {
    fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
        (**self).model(request)
    }
}

/// A shared source is a source.
impl<S: ModelSource + ?Sized> ModelSource for Rc<S> {
    fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
        (**self).model(request)
    }
}

/// The linear channel `c` (0 to 1) encoded as sRGB.
#[must_use]
pub fn srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// The sRGB channel `c` (0 to 1) decoded to linear: the inverse of
/// [`srgb`].
#[must_use]
pub fn linear(c: f32) -> f32 {
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// The vertex colour for the linear base colour `base`: its red, green and
/// blue encoded as sRGB, its alpha kept.
#[must_use]
pub fn vertex_color([r, g, b, a]: [f32; 4]) -> [f32; 4] {
    [srgb(r), srgb(g), srgb(b), a]
}

/// A part of `indices` into vertices at the ship-space `positions` with
/// `normals`, coloured by the linear `base_color`, with `material` and
/// `uv`: mapped to the view's axes ([`view_axes`]) and coloured by
/// [`vertex_color`].
pub fn part(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    base_color: [f32; 4],
    indices: Vec<u32>,
    material: Option<usize>,
    uv: Option<Vec<[f32; 2]>>,
) -> Result<Part, MeshError> {
    let color = vertex_color(base_color);
    let vertices = positions
        .iter()
        .zip(normals)
        .map(|(p, n)| Vertex {
            position: view_axes(*p),
            normal: view_axes(*n),
            color,
            uv: [0.0; 2],
            unlit: false,
        })
        .collect();
    Ok(Part {
        mesh: Mesh::new(vertices, indices)?,
        material,
        uv,
    })
}

#[cfg(any(test, feature = "recording"))]
pub use fake::{FakeModels, check_contract};

#[cfg(any(test, feature = "recording"))]
mod fake {
    use std::collections::BTreeMap;

    use super::{LoadedModel, ModelError, ModelRequest, ModelSource};
    use crate::model::{Mesh, Vertex};
    use crate::model_sampler::ModelSampler;

    /// A hand-written [`ModelSource`]: the models it was given, by asset,
    /// each at a metre to a unit, scaled to a request's length. An asset it
    /// wasn't given is [`ModelError::NoAsset`].
    #[derive(Clone, Debug, Default)]
    pub struct FakeModels {
        models: BTreeMap<String, Result<LoadedModel, ModelError>>,
    }

    impl FakeModels {
        /// It answers `asset` with `model`.
        #[must_use]
        pub fn with(mut self, asset: &str, model: LoadedModel) -> Self {
            self.models.insert(asset.to_owned(), Ok(model));
            self
        }

        /// It answers `asset` with `error`.
        #[must_use]
        pub fn with_error(mut self, asset: &str, error: ModelError) -> Self {
            self.models.insert(asset.to_owned(), Err(error));
            self
        }
    }

    impl ModelSource for FakeModels {
        fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
            let held = self
                .models
                .get(&request.asset)
                .ok_or_else(|| ModelError::NoAsset(request.asset.clone()))?
                .clone()?;
            Ok(match request.length {
                Some(length) => scaled(held, length),
                None => held,
            })
        }
    }

    /// `model` scaled uniformly so it spans `length` along y.
    fn scaled(mut model: LoadedModel, length: f32) -> LoadedModel {
        let extent = span(&model.mesh(), 1);
        if extent > 0.0 {
            let by = length / extent;
            for part in &mut model.parts {
                let vertices = part
                    .mesh
                    .vertices()
                    .iter()
                    .map(|v| Vertex {
                        position: v.position.map(|c| c * by),
                        ..*v
                    })
                    .collect();
                part.mesh = Mesh::new(vertices, part.mesh.indices().to_vec())
                    .expect("scaling keeps the indices");
            }
        }
        model
    }

    /// How far `mesh`'s vertices reach along `axis`, from the lowest to
    /// the highest, and their middle.
    fn extent(mesh: &Mesh, axis: usize) -> (f32, f32) {
        let values = mesh.vertices().iter().map(|v| v.position[axis]);
        let (min, max) = values.fold((f32::MAX, f32::MIN), |(lo, hi), c| (lo.min(c), hi.max(c)));
        (max - min, f32::midpoint(min, max))
    }

    fn span(mesh: &Mesh, axis: usize) -> f32 {
        extent(mesh, axis).0
    }

    /// Panics unless `source` meets the port's contract on `request`, an
    /// asset it holds, and `absent`, one it doesn't:
    ///
    /// - `request` loads, and its mesh has triangles;
    /// - every position and normal is finite, and every normal non-zero;
    /// - the mesh is centred on the origin on all three axes;
    /// - with a length, the mesh spans that length along y;
    /// - each part's material, each material's texture, is in range, and
    ///   each part's texture coordinates, when it has them, are one per
    ///   vertex and finite;
    /// - a material with no base-colour texture carries the default
    ///   sampler, so two readings of one model compare equal;
    /// - a material with no name has no paint slot;
    /// - asking again gives an equal answer;
    /// - `absent` is [`ModelError::NoAsset`].
    pub fn check_contract(source: &impl ModelSource, request: &ModelRequest, absent: &str) {
        let asset = &request.asset;
        let model = source
            .model(request)
            .unwrap_or_else(|e| panic!("`{asset}` loads: {e}"));
        let mesh = model.mesh();
        assert!(!mesh.is_empty(), "`{asset}` has triangles");
        for v in mesh.vertices() {
            assert!(
                v.position.iter().chain(&v.normal).all(|c| c.is_finite()),
                "`{asset}`: every position and normal is finite: {v:?}"
            );
            assert!(
                v.normal.iter().any(|c| *c != 0.0),
                "`{asset}`: every normal is non-zero: {v:?}"
            );
        }
        let size = (0..3).map(|axis| span(&mesh, axis)).fold(0.0, f32::max);
        for axis in 0..3 {
            let middle = extent(&mesh, axis).1;
            assert!(
                middle.abs() <= 1e-4 * size.max(1.0),
                "`{asset}` is centred on axis {axis}: its middle is at {middle}"
            );
        }
        if let Some(length) = request.length {
            let along = span(&mesh, 1);
            assert!(
                (along - length).abs() <= 1e-4 * length,
                "`{asset}` spans {length} along y, not {along}"
            );
        }
        for (at, part) in model.parts.iter().enumerate() {
            assert!(
                part.material.is_none_or(|m| m < model.materials.len()),
                "`{asset}` part {at}'s material is in range"
            );
            assert!(
                part.uv
                    .as_ref()
                    .is_none_or(|uv| uv.len() == part.mesh.vertices().len()),
                "`{asset}` part {at} has a texture coordinate per vertex"
            );
            assert!(
                part.uv
                    .iter()
                    .flatten()
                    .all(|uv| uv.iter().all(|c| c.is_finite())),
                "`{asset}` part {at}'s texture coordinates are finite"
            );
        }
        for (at, material) in model.materials.iter().enumerate() {
            assert!(
                material
                    .base_color_texture
                    .is_none_or(|t| t < model.textures.len()),
                "`{asset}` material {at}'s texture is in range"
            );
            assert!(
                material.base_color_texture.is_some()
                    || material.base_color_sampler == ModelSampler::DEFAULT,
                "`{asset}` material {at} has no texture, so the default sampler: {:?}",
                material.base_color_sampler
            );
            assert!(
                material.name.is_some() || material.paint.is_none(),
                "`{asset}` material {at} has no name, so no paint slot: {:?}",
                material.paint
            );
        }
        assert_eq!(
            source.model(request).as_ref(),
            Ok(&model),
            "`{asset}` gives the same answer twice"
        );
        assert!(
            matches!(
                source.model(&ModelRequest::new(absent, request.length)),
                Err(ModelError::NoAsset(_))
            ),
            "`{absent}` is no asset"
        );
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::model_sampler::{Filter, Wrap};

    /// A unit cube's parts: its first two faces in material 0 and the
    /// rest in material 1, each part a mesh of its own.
    fn cube_model() -> LoadedModel {
        let cube = Mesh::cube([nova_view::Color::WHITE; 6]);
        let split = |range: std::ops::Range<usize>, material| {
            let vertices = cube.vertices()[range.start * 4..range.end * 4].to_vec();
            let indices = cube.indices()[range.start * 6..range.end * 6]
                .iter()
                .map(|i| i - (range.start * 4) as u32)
                .collect();
            Part {
                mesh: Mesh::new(vertices, indices).unwrap(),
                material: Some(material),
                uv: None,
            }
        };
        LoadedModel {
            parts: vec![split(0..2, 0), split(2..6, 1)],
            materials: vec![material("metal"), material("dark")],
            textures: Vec::new(),
        }
    }

    fn material(name: &str) -> Material {
        Material {
            name: Some(name.to_owned()),
            ..Material::new([1.0; 4])
        }
    }

    #[test]
    fn a_new_material_says_only_its_base_colour() {
        assert_eq!(
            Material::new([0.5, 0.25, 0.125, 1.0]),
            Material {
                name: None,
                paint: None,
                base_color: [0.5, 0.25, 0.125, 1.0],
                metallic: 1.0,
                roughness: 1.0,
                unlit: false,
                base_color_texture: None,
                base_color_sampler: ModelSampler::DEFAULT,
            }
        );
    }

    #[test]
    fn a_loaded_model_merges_its_parts_into_one_mesh() {
        let model = cube_model();
        let merged = model.mesh();
        assert_eq!(merged, Mesh::cube([nova_view::Color::WHITE; 6]));
        // The second part's indices are offset past the first's vertices.
        assert_eq!(model.parts[1].mesh.indices()[0], 0);
        assert_eq!(merged.indices()[12], 8);
    }

    /// A one-texel texture of `shade`, embedded.
    fn texture(shade: u8) -> Texture {
        Texture {
            image: Arc::new(ModelTexture::new(1, 1, vec![shade, shade, shade, 255]).unwrap()),
            path: None,
        }
    }

    /// A material drawing texture `texture`, lit.
    fn textured(texture: usize) -> Material {
        Material {
            base_color_texture: Some(texture),
            ..material("skin")
        }
    }

    /// The texture coordinates face `at`'s four vertices are given.
    fn face_uv(at: usize) -> Vec<[f32; 2]> {
        (0..4).map(|i| [at as f32 / 8.0, i as f32 / 4.0]).collect()
    }

    /// The unit cube's six faces as six parts: face `at` in the material
    /// `faces[at]` names, with texture coordinates ([`face_uv`]) where it
    /// says so.
    fn faced(
        materials: Vec<Material>,
        textures: Vec<Texture>,
        faces: [(Option<usize>, bool); 6],
    ) -> LoadedModel {
        let cube = Mesh::cube([nova_view::Color::WHITE; 6]);
        let parts = faces
            .iter()
            .enumerate()
            .map(|(at, (material, has_uv))| {
                let vertices = cube.vertices()[at * 4..at * 4 + 4].to_vec();
                let indices = cube.indices()[at * 6..at * 6 + 6]
                    .iter()
                    .map(|i| i - (at * 4) as u32)
                    .collect();
                Part {
                    mesh: Mesh::new(vertices, indices).unwrap(),
                    material: *material,
                    uv: has_uv.then(|| face_uv(at)),
                }
            })
            .collect();
        LoadedModel {
            parts,
            materials,
            textures,
        }
    }

    fn section(indices: std::ops::Range<u32>, texture: Option<usize>) -> Section {
        Section::new(indices, texture)
    }

    /// A sampler that clamps along u and reads the nearest texel.
    const SHARP: ModelSampler = ModelSampler {
        wrap_s: Wrap::Clamp,
        mag: Filter::Nearest,
        ..ModelSampler::DEFAULT
    };

    /// A material drawing texture `texture` through `sampler`.
    fn sampled(texture: usize, sampler: ModelSampler) -> Material {
        Material {
            base_color_sampler: sampler,
            ..textured(texture)
        }
    }

    #[test]
    fn a_part_samples_its_material_only_with_coordinates() {
        let model = faced(
            vec![textured(0); 4],
            vec![texture(9)],
            [
                (Some(3), true),
                (Some(3), false),
                (None, true),
                (None, false),
                (Some(1), true),
                (Some(1), true),
            ],
        );
        let sampling: Vec<_> = model.parts.iter().map(Part::sampling_material).collect();
        assert_eq!(sampling, [Some(3), None, None, None, Some(1), Some(1)]);
    }

    #[test]
    fn parts_on_one_texture_split_where_the_sampler_changes_and_only_there() {
        let mirrored = ModelSampler {
            wrap_t: Wrap::Mirror,
            ..ModelSampler::DEFAULT
        };
        let model = faced(
            vec![sampled(0, SHARP), sampled(0, mirrored), textured(0)],
            vec![texture(9)],
            [
                (Some(0), true),
                (Some(0), true),
                (Some(1), true),
                (Some(2), true),
                (Some(2), true),
                (Some(0), true),
            ],
        );
        let mesh = model.mesh();
        assert_eq!(
            mesh.sections(),
            [
                section(0..12, Some(0)).sampled(SHARP),
                section(12..18, Some(0)).sampled(mirrored),
                section(18..30, Some(0)),
                section(30..36, Some(0)).sampled(SHARP),
            ]
        );
        // One texture, however many samplers read it.
        assert_eq!(mesh.textures().len(), 1);
        assert!(Arc::ptr_eq(&mesh.textures()[0], &model.textures[0].image));
    }

    #[test]
    fn a_part_that_draws_no_texture_carries_the_default_sampler() {
        // A sampled material's part with no coordinates, a part with no
        // material and a material naming a texture the model lacks all
        // draw untextured, and merge into one section whatever sampler
        // their materials name.
        let model = faced(
            vec![sampled(0, SHARP), sampled(4, SHARP), material("plain")],
            vec![texture(9)],
            [
                (Some(0), true),
                (Some(0), false),
                (None, true),
                (Some(1), true),
                (Some(2), true),
                (Some(0), true),
            ],
        );
        assert_eq!(
            model.mesh().sections(),
            [
                section(0..6, Some(0)).sampled(SHARP),
                section(6..30, None),
                section(30..36, Some(0)).sampled(SHARP),
            ]
        );
    }

    #[test]
    fn a_textured_part_s_vertices_carry_its_coordinates_and_its_section_its_texture() {
        let model = faced(vec![textured(0)], vec![texture(9)], [(Some(0), true); 6]);
        let mesh = model.mesh();
        assert_eq!(mesh.sections(), [section(0..36, Some(0))]);
        assert_eq!(mesh.textures().len(), 1);
        assert!(
            Arc::ptr_eq(&mesh.textures()[0], &model.textures[0].image),
            "the model's own texture, shared"
        );
        for at in 0..6 {
            let uvs: Vec<[f32; 2]> = mesh.vertices()[at * 4..at * 4 + 4]
                .iter()
                .map(|v| v.uv)
                .collect();
            assert_eq!(uvs, face_uv(at), "face {at}");
        }
        // A part's own mesh carries none: the merge writes them.
        assert!(
            model.parts[3]
                .mesh
                .vertices()
                .iter()
                .all(|v| v.uv == [0.0; 2])
        );
    }

    #[test]
    fn an_unlit_material_s_vertices_are_flagged_and_a_lit_one_s_are_not() {
        let unlit = Material {
            unlit: true,
            ..material("glow")
        };
        let model = faced(
            vec![unlit, material("hull")],
            Vec::new(),
            [
                (Some(0), false),
                (Some(1), false),
                (Some(0), true),
                (None, false),
                // A material the model lacks is no material.
                (Some(7), false),
                (Some(1), true),
            ],
        );
        let mesh = model.mesh();
        let flags: Vec<bool> = (0..6)
            .map(|at| {
                let face = &mesh.vertices()[at * 4..at * 4 + 4];
                assert!(face.iter().all(|v| v.unlit == face[0].unlit), "face {at}");
                face[0].unlit
            })
            .collect();
        assert_eq!(flags, [true, false, true, false, false, false]);
        assert!(model.parts[0].mesh.vertices().iter().all(|v| !v.unlit));
    }

    #[test]
    fn consecutive_parts_on_one_texture_share_a_section_and_another_texture_splits_it() {
        let model = faced(
            vec![textured(0), textured(1), material("plain")],
            vec![texture(1), texture(2)],
            [
                (Some(0), true),
                (Some(0), true),
                (Some(1), true),
                (Some(0), true),
                (Some(2), true),
                (None, true),
            ],
        );
        let mesh = model.mesh();
        assert_eq!(
            mesh.sections(),
            [
                section(0..12, Some(0)),
                section(12..18, Some(1)),
                section(18..24, Some(0)),
                section(24..36, None),
            ]
        );
        assert_eq!(mesh.textures().len(), 2);
        assert!(Arc::ptr_eq(&mesh.textures()[1], &model.textures[1].image));
        // The triangles are the cube's, whatever the sections.
        assert_eq!(
            mesh.indices(),
            Mesh::cube([nova_view::Color::WHITE; 6]).indices()
        );
    }

    #[test]
    fn a_textured_material_on_a_part_with_no_coordinates_draws_untextured() {
        let model = faced(
            vec![textured(0)],
            vec![texture(9)],
            [
                (Some(0), true),
                (Some(0), false),
                (Some(0), false),
                (Some(0), true),
                (Some(0), true),
                (Some(0), true),
            ],
        );
        let mesh = model.mesh();
        assert_eq!(
            mesh.sections(),
            [
                section(0..6, Some(0)),
                section(6..18, None),
                section(18..36, Some(0)),
            ]
        );
        assert!(mesh.vertices()[4..12].iter().all(|v| v.uv == [0.0; 2]));
        // Sampled by one part and not by another, the texture is held
        // once.
        assert_eq!(mesh.textures().len(), 1);
        assert!(Arc::ptr_eq(&mesh.textures()[0], &model.textures[0].image));
    }

    #[test]
    fn a_texture_no_part_samples_is_left_out_of_the_mesh() {
        // A textured material on parts with no coordinates.
        let bare = faced(vec![textured(0)], vec![texture(9)], [(Some(0), false); 6]);
        // A texture no material names.
        let unnamed = faced(
            vec![material("plain")],
            vec![texture(9)],
            [(Some(0), true); 6],
        );
        // A material naming a texture the model lacks.
        let lacking = faced(vec![textured(1)], vec![texture(9)], [(Some(0), true); 6]);
        for model in [bare, unnamed, lacking] {
            let mesh = model.mesh();
            assert_eq!(mesh.sections(), [section(0..36, None)]);
            assert!(mesh.textures().is_empty());
        }
    }

    #[test]
    fn the_mesh_holds_only_the_sampled_textures_renumbered_in_the_model_s_order() {
        let model = faced(
            vec![textured(0), textured(1), textured(2)],
            vec![texture(1), texture(2), texture(3)],
            [
                (Some(2), true),
                (Some(0), true),
                (Some(1), false),
                (Some(2), true),
                (Some(2), true),
                (Some(2), true),
            ],
        );
        let mesh = model.mesh();
        assert_eq!(
            mesh.sections(),
            [
                section(0..6, Some(1)),
                section(6..12, Some(0)),
                section(12..18, None),
                section(18..36, Some(1)),
            ]
        );
        assert_eq!(mesh.textures().len(), 2);
        assert!(Arc::ptr_eq(&mesh.textures()[0], &model.textures[0].image));
        assert!(Arc::ptr_eq(&mesh.textures()[1], &model.textures[2].image));
    }

    #[test]
    fn one_image_under_two_samplers_after_an_unsampled_texture_is_one_texture() {
        let model = faced(
            vec![sampled(1, SHARP), textured(1), textured(0)],
            vec![texture(1), texture(2)],
            [
                (Some(0), true),
                (Some(0), true),
                (Some(1), true),
                (Some(1), true),
                (Some(2), false),
                (Some(0), true),
            ],
        );
        let mesh = model.mesh();
        assert_eq!(
            mesh.sections(),
            [
                section(0..12, Some(0)).sampled(SHARP),
                section(12..24, Some(0)),
                section(24..30, None),
                section(30..36, Some(0)).sampled(SHARP),
            ]
        );
        assert_eq!(mesh.textures().len(), 1);
        assert!(Arc::ptr_eq(&mesh.textures()[0], &model.textures[1].image));
    }

    /// What a recording GPU is sent when a model's mesh is drawn once.
    struct Uploads {
        /// The textures uploaded, in order, each with the id it took.
        textures: Vec<(crate::gpu::ModelTextureId, ModelTexture)>,
        /// The one mesh uploaded.
        mesh: Mesh,
        /// The texture ids the mesh was uploaded naming.
        named: Vec<crate::gpu::ModelTextureId>,
    }

    /// What a recording GPU is sent when `model`'s mesh is drawn once.
    fn uploads(model: &LoadedModel) -> Uploads {
        use crate::recording::{GpuCall, NoImages, RecordingGpu};
        use crate::viewport::{LogicalSize, Viewport};

        let key = nova_view::ModelKey(1);
        let mut renderer = crate::Renderer::new(NoImages);
        renderer.add_model(key, model.mesh());
        let mut list = nova_view::DrawList::new();
        list.model(
            key,
            nova_view::ModelPose {
                center: nova_view::Point::new(12.0, 12.0),
                orientation: nova_view::Orientation::IDENTITY,
                scale: 10.0,
            },
        );
        let viewport = Viewport::new(LogicalSize { w: 64, h: 48 }, (128, 96), 2.0);
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport, &mut gpu);
        let mut textures = Vec::new();
        let mut meshes = Vec::new();
        for call in gpu.calls {
            match call {
                GpuCall::UploadModelTexture { texture, data } => textures.push((texture, data)),
                GpuCall::UploadMesh { data, textures, .. } => meshes.push((data, textures)),
                _ => {}
            }
        }
        assert_eq!(meshes.len(), 1, "the one mesh is uploaded once");
        let (mesh, named) = meshes.remove(0);
        Uploads {
            textures,
            mesh,
            named,
        }
    }

    #[test]
    fn a_textured_material_on_parts_with_no_coordinates_uploads_no_texture() {
        let model = faced(vec![textured(0)], vec![texture(9)], [(Some(0), false); 6]);
        let Uploads {
            textures,
            mesh,
            named,
        } = uploads(&model);
        assert_eq!(textures, []);
        assert_eq!(named, []);
        assert_eq!(mesh.sections(), [section(0..36, None)]);
    }

    #[test]
    fn a_model_with_a_sampled_and_an_unsampled_texture_uploads_only_the_sampled_one() {
        // The unsampled texture first in the model's list, then last: its
        // material sits on a face with no coordinates.
        for (sampled, unsampled) in [(1, 0), (0, 1)] {
            let mut held = vec![texture(1), texture(2)];
            held[sampled] = texture(7);
            let model = faced(
                vec![textured(sampled), textured(unsampled)],
                held,
                [
                    (Some(1), false),
                    (Some(0), true),
                    (Some(0), true),
                    (Some(0), true),
                    (Some(0), true),
                    (Some(0), true),
                ],
            );
            let Uploads {
                textures,
                mesh,
                named,
            } = uploads(&model);
            let ids: Vec<_> = textures.iter().map(|(id, _)| *id).collect();
            assert_eq!(ids.len(), 1, "unsampled at {unsampled}");
            assert_eq!(textures[0].1, *model.textures[sampled].image);
            assert_eq!(named, ids, "the mesh names the one upload");
            assert_eq!(
                mesh.sections(),
                [section(0..6, None), section(6..36, Some(0))],
                "unsampled at {unsampled}"
            );
        }
    }

    #[test]
    fn a_material_naming_a_texture_the_model_lacks_draws_untextured() {
        let model = faced(vec![textured(1)], vec![texture(9)], [(Some(0), true); 6]);
        assert_eq!(model.mesh().sections(), [section(0..36, None)]);
    }

    #[test]
    fn a_model_with_no_textures_merges_to_one_untextured_section() {
        let mesh = cube_model().mesh();
        assert_eq!(mesh.sections(), [section(0..36, None)]);
        assert!(mesh.textures().is_empty());
        // Texture coordinates alone make no section.
        let coordinates = faced(vec![material("hull")], Vec::new(), [(Some(0), true); 6]);
        assert_eq!(coordinates.mesh().sections(), [section(0..36, None)]);
        // A model with no parts is a mesh with nothing to draw.
        let nothing = LoadedModel {
            parts: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
        };
        assert!(nothing.mesh().is_empty());
    }

    #[test]
    fn base_colour_bakes_into_vertex_colour_as_srgb() {
        let [red, green, blue, alpha] = vertex_color([0.5, 0.002, 1.0, 0.25]);
        assert!((red - 0.735_356_9).abs() < 1e-5, "{red}");
        assert!((green - 0.002 * 12.92).abs() < 1e-7, "{green}");
        assert!((blue - 1.0).abs() < 1e-6, "{blue}");
        assert_eq!(alpha, 0.25, "alpha is kept");
        assert_eq!(srgb(0.0), 0.0);
        // Either side of the linear segment's end meets the curve.
        assert!((srgb(0.003_130_8) - srgb(0.003_130_9)).abs() < 1e-5);

        let made = part(
            &[[1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            &[[0.0, 1.0, 0.0]; 3],
            [0.5, 0.002, 1.0, 0.25],
            vec![0, 1, 2],
            Some(4),
            Some(vec![[0.5; 2]; 3]),
        )
        .unwrap();
        let first = made.mesh.vertices()[0];
        assert_eq!(first.position, [1.0, -2.0, 3.0]);
        assert_eq!(first.normal, [0.0, -1.0, 0.0]);
        assert_eq!(first.color, vertex_color([0.5, 0.002, 1.0, 0.25]));
        assert_eq!(made.material, Some(4));
        assert_eq!(made.uv, Some(vec![[0.5; 2]; 3]));
        assert!(
            part(
                &[[0.0; 3]],
                &[[0.0; 3]],
                [1.0; 4],
                vec![0, 0, 1],
                None,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn linear_undoes_srgb() {
        for byte in 0..=255_u8 {
            let c = f32::from(byte) / 255.0;
            let back = srgb(linear(c));
            assert!((back - c).abs() < 1e-5, "{byte}: {back}");
        }
        // Each segment on its own, either side of the break at 0.04045.
        assert_eq!(linear(0.04), 0.04 / 12.92);
        assert_eq!(linear(0.04045), 0.04045 / 12.92);
        assert_eq!(linear(0.5), (0.555_f32 / 1.055).powf(2.4));
        assert!((linear(0.040_46) - 0.003_131_1).abs() < 1e-6);
        assert_eq!(linear(0.0), 0.0);
        assert!((linear(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_fake_answers_what_it_holds_and_no_asset_otherwise() {
        let fake = FakeModels::default()
            .with("cube", cube_model())
            .with_error("pointer", ModelError::LfsPointer("pointer".into()));
        assert_eq!(
            fake.model(&ModelRequest::new("cube", None)),
            Ok(cube_model())
        );
        assert_eq!(
            fake.model(&ModelRequest::new("pointer", None)),
            Err(ModelError::LfsPointer("pointer".into()))
        );
        assert_eq!(
            fake.model(&ModelRequest::new("nothing", None)),
            Err(ModelError::NoAsset("nothing".into()))
        );
        // Borrowed and shared, it answers the same.
        let request = ModelRequest::new("cube", Some(2.0));
        let borrowed: &dyn ModelSource = &&fake;
        assert_eq!(borrowed.model(&request), fake.model(&request));
        assert_eq!(Rc::new(fake.clone()).model(&request), fake.model(&request));
    }

    #[test]
    fn the_fake_scales_to_the_requested_length() {
        let fake = FakeModels::default().with("cube", cube_model());
        let mesh = fake
            .model(&ModelRequest::new("cube", Some(8.0)))
            .unwrap()
            .mesh();
        for (scaled, unit) in mesh.vertices().iter().zip(cube_model().mesh().vertices()) {
            assert_eq!(scaled.position, unit.position.map(|c| c * 8.0));
            assert_eq!(scaled.normal, unit.normal);
        }
    }

    /// One triangle at `positions`, as a model.
    fn triangle(positions: [[f32; 3]; 3]) -> LoadedModel {
        let vertices = positions
            .iter()
            .map(|p| Vertex {
                position: *p,
                normal: [0.0, 0.0, 1.0],
                color: [1.0; 4],
                uv: [0.0; 2],
                unlit: false,
            })
            .collect();
        LoadedModel {
            parts: vec![Part {
                mesh: Mesh::new(vertices, vec![0, 1, 2]).unwrap(),
                material: None,
                uv: None,
            }],
            materials: Vec::new(),
            textures: Vec::new(),
        }
    }

    #[test]
    fn the_fake_scales_by_the_length_over_the_model_s_own() {
        // 4 units along y, asked for 8: twice the size.
        let tall = triangle([[0.0, -2.0, 0.0], [1.0, 2.0, 0.0], [-1.0, 2.0, 0.0]]);
        let fake = FakeModels::default().with("tall", tall);
        let mesh = fake
            .model(&ModelRequest::new("tall", Some(8.0)))
            .unwrap()
            .mesh();
        assert_eq!(mesh.vertices()[0].position, [0.0, -4.0, 0.0]);
        // A model with no length along y can't be scaled to one, and isn't.
        let flat = triangle([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
        let fake = FakeModels::default().with("flat", flat.clone());
        assert_eq!(fake.model(&ModelRequest::new("flat", Some(8.0))), Ok(flat));
    }

    #[test]
    fn the_fake_meets_the_contract() {
        let fake = FakeModels::default().with("cube", cube_model());
        check_contract(&fake, &ModelRequest::new("cube", None), "nothing");
        check_contract(&fake, &ModelRequest::new("cube", Some(40.0)), "nothing");
        // Textured and unlit, it still does, and scaling keeps both.
        let skin = Material {
            unlit: true,
            ..textured(0)
        };
        let skinned = faced(vec![skin], vec![texture(9)], [(Some(0), true); 6]);
        let fake = FakeModels::default().with("skinned", skinned.clone());
        check_contract(&fake, &ModelRequest::new("skinned", None), "nothing");
        check_contract(&fake, &ModelRequest::new("skinned", Some(40.0)), "nothing");
        let scaled = fake
            .model(&ModelRequest::new("skinned", Some(40.0)))
            .unwrap()
            .mesh();
        assert_eq!(scaled.sections(), skinned.mesh().sections());
        assert!(scaled.vertices().iter().all(|v| v.unlit));
        assert_eq!(scaled.vertices()[5].uv, skinned.mesh().vertices()[5].uv);
        // Sampled its own way, it still does, and scaling keeps the
        // sampler.
        let sharp = faced(
            vec![sampled(0, SHARP)],
            vec![texture(9)],
            [(Some(0), true); 6],
        );
        let fake = FakeModels::default().with("sharp", sharp);
        check_contract(&fake, &ModelRequest::new("sharp", None), "nothing");
        check_contract(&fake, &ModelRequest::new("sharp", Some(40.0)), "nothing");
        let scaled = fake.model(&ModelRequest::new("sharp", Some(40.0))).unwrap();
        assert_eq!(
            scaled.mesh().sections(),
            [section(0..36, Some(0)).sampled(SHARP)]
        );
    }

    /// A source whose cube has a texture coordinate that is not a number.
    struct BadCoordinates;

    impl ModelSource for BadCoordinates {
        fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
            match request.asset.as_str() {
                "cube" => {
                    let mut model =
                        faced(vec![textured(0)], vec![texture(9)], [(Some(0), true); 6]);
                    model.parts[2].uv = Some(vec![[0.0, 0.5], [f32::NAN, 0.5], [0.0; 2], [0.0; 2]]);
                    Ok(model)
                }
                other => Err(ModelError::NoAsset(other.into())),
            }
        }
    }

    #[test]
    #[should_panic(expected = "`cube` part 2's texture coordinates are finite")]
    fn the_contract_fails_a_source_with_a_texture_coordinate_that_is_not_finite() {
        check_contract(&BadCoordinates, &ModelRequest::new("cube", None), "nothing");
    }

    /// A source whose cube has an untextured material with a sampler of
    /// its own.
    struct SamplesNothing;

    impl ModelSource for SamplesNothing {
        fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
            match request.asset.as_str() {
                "cube" => {
                    let mut model = cube_model();
                    model.materials[1].base_color_sampler = SHARP;
                    Ok(model)
                }
                other => Err(ModelError::NoAsset(other.into())),
            }
        }
    }

    #[test]
    #[should_panic(expected = "`cube` material 1 has no texture, so the default sampler")]
    fn the_contract_fails_a_source_whose_untextured_material_carries_a_sampler() {
        check_contract(&SamplesNothing, &ModelRequest::new("cube", None), "nothing");
    }

    /// A source that answers every request with the unit cube, whatever
    /// its length.
    struct IgnoresLength;

    impl ModelSource for IgnoresLength {
        fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
            match request.asset.as_str() {
                "cube" => Ok(cube_model()),
                other => Err(ModelError::NoAsset(other.into())),
            }
        }
    }

    #[test]
    #[should_panic(expected = "spans 8 along y")]
    fn the_contract_fails_a_source_that_ignores_the_length() {
        check_contract(
            &IgnoresLength,
            &ModelRequest::new("cube", Some(8.0)),
            "nothing",
        );
    }

    /// A source whose cube has an unnamed material in a paint slot.
    struct PaintsTheUnnamed;

    impl ModelSource for PaintsTheUnnamed {
        fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
            match request.asset.as_str() {
                "cube" => {
                    let mut model = cube_model();
                    model.materials[1] = Material {
                        name: None,
                        paint: Some(PaintSlot::Trim),
                        ..material("dark")
                    };
                    Ok(model)
                }
                other => Err(ModelError::NoAsset(other.into())),
            }
        }
    }

    #[test]
    #[should_panic(expected = "`cube` material 1 has no name, so no paint slot")]
    fn the_contract_fails_a_source_that_paints_an_unnamed_material() {
        check_contract(
            &PaintsTheUnnamed,
            &ModelRequest::new("cube", None),
            "nothing",
        );
    }

    #[test]
    fn model_error_messages_name_the_asset_and_the_budget_file() {
        let over = ModelError::OverBudget {
            asset: "corsair".into(),
            over: vec![
                Overrun {
                    measure: Measure::Triangles,
                    used: 13,
                    most: 12,
                    budget: "budgets/small-craft.md".into(),
                },
                Overrun {
                    measure: Measure::Kib,
                    used: 2049,
                    most: 2,
                    budget: "budgets/default.md".into(),
                },
            ],
        };
        assert_eq!(
            over.to_string(),
            "asset `corsair` is over its art budget: draws 13 triangles, over the 12 budget \
             in `budgets/small-craft.md`; is 3 KiB, over the 2 KiB budget in `budgets/default.md`"
        );
        let side = Overrun {
            measure: Measure::Side,
            used: 8192,
            most: 4096,
            budget: "budgets/default.md".into(),
        };
        assert_eq!(
            side.to_string(),
            "is 8192 pixels on its longest side, over the 4096 pixel budget in \
             `budgets/default.md`"
        );
        let missing = ModelError::Missing {
            asset: "hub".into(),
            file: "media/export/p/hub.glb".into(),
        };
        assert_eq!(
            missing.to_string(),
            "asset `hub`'s model file `media/export/p/hub.glb` is missing"
        );
        let beside = ImageRef::File("media/export/p/skin.png".into());
        assert_eq!(beside.to_string(), "image `media/export/p/skin.png`");
        assert_eq!(ImageRef::Embedded(2).to_string(), "embedded image 2");
        let undecodable = ModelError::TextureUndecodable {
            asset: "hub".into(),
            image: beside.clone(),
            why: "cut short".into(),
        };
        assert_eq!(
            undecodable.to_string(),
            "asset `hub`'s image `media/export/p/skin.png` doesn't decode: cut short"
        );
        let over_side = ModelError::TextureOverBudget {
            asset: "hub".into(),
            image: beside,
            over: vec![side],
        };
        assert_eq!(
            over_side.to_string(),
            "asset `hub`'s image `media/export/p/skin.png` is 8192 pixels on its longest side, \
             over the 4096 pixel budget in `budgets/default.md`"
        );
        let too_large = ModelError::TextureTooLarge {
            asset: "hub".into(),
            image: ImageRef::Embedded(2),
            side: 16_384,
            most: 8_192,
        };
        assert_eq!(
            too_large.to_string(),
            "asset `hub`'s embedded image 2 is 16384 pixels on its longest side, over the 8192 \
             the renderer draws"
        );
        for error in [
            ModelError::NoAsset("hub".into()),
            ModelError::NotAModel("hub".into()),
            ModelError::NoFile("hub".into()),
            ModelError::LfsPointer("hub".into()),
            ModelError::NoDepth("hub".into()),
            ModelError::Unreadable {
                asset: "hub".into(),
                why: "`a.glb`: broken".into(),
            },
            ModelError::Mesh {
                asset: "hub".into(),
                error: MeshError::PartialTriangle { indices: 2 },
            },
        ] {
            assert!(error.to_string().contains("`hub`"), "{error}");
        }
    }
}
