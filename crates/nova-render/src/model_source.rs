//! The model source port: the new game's 3D models, by asset id, as
//! meshes the renderer can take, with their materials and textures.
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
//! straight. Metallic, roughness, `unlit` and the textures are carried,
//! not drawn: the shader lights every model and samples no texture yet.
//!
//! # Where an addition goes
//!
//! - Something a caller must say about the model it wants (a level of
//!   detail, say): a field of [`ModelRequest`].
//! - A new way a model can't be had: a [`ModelError`] variant, its
//!   message naming the asset (and the file).
//! - Something more read from the model: a field of [`Part`],
//!   [`Material`] or [`Texture`], filled by the adapter
//!   ([`crate::galaxy_models`]) from the media crate's reading.
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

use std::fmt;
use std::rc::Rc;

/// The ship-to-view axes map is the view's
/// ([`nova_view::view_axes`]): forwarded here, so the mesh's vertices and
/// what the view places on a ship take one map.
pub use nova_view::view_axes;

pub use crate::livery::PaintSlot;
use crate::model::{Mesh, MeshError, Vertex};

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
    /// of [`mesh`](Self::mesh).
    pub uv: Option<Vec<[f32; 2]>>,
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
}

/// A model's image, as stored: PNG or JPEG bytes, not decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    /// The encoded bytes.
    pub bytes: Vec<u8>,
    /// Its MIME type, if the model gives one.
    pub mime: Option<String>,
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
    /// Its textures.
    pub textures: Vec<Texture>,
}

impl LoadedModel {
    /// Its parts merged into the one mesh
    /// [`Renderer::add_model`](crate::Renderer::add_model) takes.
    #[must_use]
    pub fn mesh(&self) -> Mesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for part in &self.parts {
            let base = u32::try_from(vertices.len()).expect("a model's vertices fit in u32");
            vertices.extend_from_slice(part.mesh.vertices());
            indices.extend(part.mesh.indices().iter().map(|i| base + i));
        }
        Mesh::new(vertices, indices).expect("each part's indices are in its own range")
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
    /// The model is over its art budget.
    #[error("asset `{asset}` is over its art budget: {}", list(over))]
    OverBudget {
        /// The asset.
        asset: String,
        /// Every limit it breaks.
        over: Vec<Overrun>,
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
    ///   vertex;
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
        }
        for (at, material) in model.materials.iter().enumerate() {
            assert!(
                material
                    .base_color_texture
                    .is_none_or(|t| t < model.textures.len()),
                "`{asset}` material {at}'s texture is in range"
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
            paint: None,
            base_color: [1.0; 4],
            metallic: 1.0,
            roughness: 1.0,
            unlit: false,
            base_color_texture: None,
        }
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
