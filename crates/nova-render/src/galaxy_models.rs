//! [`ModelSource`] for the new game's data: [`GalaxyModels`] reads an
//! asset's `.glb` or `.gltf` (with the buffers and images a `.gltf`
//! names) through the format's [`Source`] port, needing no GPU.
//!
//! A scene comes from one of two places. Given the scenes somebody read
//! already ([`GalaxyModels::reading`]: the loader's, which reads every
//! model a ship, a mount or a station shows for its 2D stand-in), an
//! asset among them is taken as it was read there, a failure included,
//! and the source isn't asked about it. Any other asset, and every asset
//! with no scenes given, is read through the source
//! (`nova_galaxy_media::read_asset_scene`). Either way what is made of the
//! scene is the same.
//!
//! It only chains decisions made elsewhere: which file is the model and
//! how it is read (`nova_galaxy_media::scene`), which budget covers it and
//! what it breaks (`nova_galaxy_media::budget`), how each image a
//! material's base colour names is sniffed, held to the default budget's
//! `texture` side and to the renderer's largest
//! ([`MAX_TEXTURE_SIDE`]) and decoded (`nova_galaxy_media::image`), how
//! its mip chain is built ([`ModelTexture::new`]), which sampler, which
//! coordinate set and which transform a base colour is sampled by
//! (`nova_galaxy_media::sampling`: the sampler is handed on in the port's
//! terms, and the coordinates arrive already resolved), how it sits on a
//! ship
//! (`nova_galaxy_media::Placement`), which livery slot the asset's `paint`
//! puts each material in ([`nova_galaxy_media::slot_of`], the rule the
//! top-down silhouettes use too), and how a placed primitive becomes a
//! [`Part`](crate::model_source::Part) ([`model_source::part`]). Those are
//! pinned by their own tests; this module's tests check what it does with
//! them. It caches nothing: the caller decides when to load and which
//! [`ModelKey`](nova_view::ModelKey) a model takes
//! ([`Renderer::add_model`](crate::Renderer::add_model) with
//! [`LoadedModel::mesh`]).
//!
//! For a ship, [`ship_request`] and [`ship_livery`] read what its `base`
//! chain gives (`nova_galaxy_format::fleet::resolve`): the model to ask
//! for, and the livery to paint it in ([`crate::livery::painted`]), each
//! colour's bytes as the format reads them
//! ([`Color::rgb`](nova_galaxy_format::id::Color::rgb)).
//!
//! For a station or outpost, [`station_livery`] reads the livery of the
//! faction that owns it: its `owner`, and the first livery by id whose
//! `faction` names that owner, its colours read as a ship's are. Which of
//! its model's materials take each colour is the model asset's `paint`,
//! as for a ship.

use std::collections::BTreeSet;
use std::sync::Arc;

use nova_galaxy_format::asset::{AssetKind, Paint};
use nova_galaxy_format::fleet::resolve;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::Galaxy;
use nova_galaxy_format::source::Source;
use nova_galaxy_media::budget::{self, Limits, limits, measure, over_budget, texture_limits};
use nova_galaxy_media::sampling as media;
use nova_galaxy_media::scene::{Scene, read_asset_scene};
use nova_galaxy_media::{AssetScenes, NoImage, NoModel, Placement, Slot, read_image, slot_of};

use crate::gpu::MAX_TEXTURE_SIDE;
use crate::livery::Livery;
use crate::model::MeshError;
use crate::model_sampler::{Filter, Mip, ModelSampler, Wrap};
use crate::model_source::{
    self, ImageRef, LoadedModel, Material, Measure, ModelError, ModelRequest, ModelSource, Overrun,
    PaintSlot, Texture,
};
use crate::model_texture::ModelTexture;

/// The new game's models: the asset manifest and budgets of `galaxy`, the
/// files read through `source`, but for the scenes it was given already
/// read ([`reading`](Self::reading)).
#[derive(Clone, Copy, Debug)]
pub struct GalaxyModels<'a, S: Source> {
    galaxy: &'a Galaxy,
    source: &'a S,
    /// The scenes somebody read already, if any were given.
    scenes: Option<&'a AssetScenes>,
}

impl<'a, S: Source> GalaxyModels<'a, S> {
    /// The models `galaxy` names, read through `source`.
    #[must_use]
    pub fn new(galaxy: &'a Galaxy, source: &'a S) -> Self {
        GalaxyModels {
            galaxy,
            source,
            scenes: None,
        }
    }

    /// The same models, taking each asset in `scenes` as it was read
    /// there (its scene, or why it has none) and asking the source for
    /// nothing about it: the loader's
    /// ([`NewGame::scenes`](nova_data::new_game::NewGame::scenes)), so
    /// no model file is read twice. An asset with no entry is still read
    /// through the source.
    #[must_use]
    pub fn reading(self, scenes: &'a AssetScenes) -> Self {
        GalaxyModels {
            scenes: Some(scenes),
            ..self
        }
    }
}

impl<S: Source> ModelSource for GalaxyModels<'_, S> {
    fn model(&self, request: &ModelRequest) -> Result<LoadedModel, ModelError> {
        let name = &request.asset;
        let id = Id::try_from(name.clone()).map_err(|_| ModelError::NoAsset(name.clone()))?;
        let asset = &self
            .galaxy
            .assets
            .get(&id)
            .ok_or_else(|| ModelError::NoAsset(name.clone()))?
            .data;
        if asset.kind != AssetKind::Model {
            return Err(ModelError::NotAModel(name.clone()));
        }
        let read;
        let (scene, front) = match self.scenes.and_then(|scenes| scenes.get(&id)) {
            Some(Ok((scene, front))) => (scene, *front),
            Some(Err(why)) => return Err(no_model(name, why.clone())),
            None => {
                read = read_asset_scene(self.source, asset).map_err(|why| no_model(name, why))?;
                (&read.0, read.1)
            }
        };
        let over = over_budget(&limits(self.galaxy, &id), measure(scene));
        if !over.is_empty() {
            return Err(ModelError::OverBudget {
                asset: name.clone(),
                over: over.iter().map(overrun).collect(),
            });
        }
        let placement = match request.length {
            Some(length) => Placement::try_from_bounds(scene.bounds, front, f64::from(length))
                .ok_or_else(|| ModelError::NoDepth(name.clone()))?,
            None => Placement::unscaled_bounds(scene.bounds, front),
        };
        let textures = textures(name, scene, &texture_limits(self.galaxy))?;
        loaded(scene, &placement, asset.paint.as_ref(), textures).map_err(|error| {
            ModelError::Mesh {
                asset: name.clone(),
                error,
            }
        })
    }
}

/// A scene's base-colour textures, decoded, and which of them each of the
/// scene's images became (`None` for an image no base colour names).
type Textures = (Vec<Texture>, Vec<Option<usize>>);

/// The textures of `asset`'s `scene`: each image a material's base colour
/// names, in the images' order, read by `nova_galaxy_media::read_image`
/// held to `limits` (the default budget's `texture`) and to the largest
/// side the renderer draws ([`MAX_TEXTURE_SIDE`]). An image nothing's base
/// colour names is not decoded, so it can't refuse the model.
fn textures(asset: &str, scene: &Scene, limits: &Limits) -> Result<Textures, ModelError> {
    let named: BTreeSet<usize> = scene
        .materials
        .iter()
        .filter_map(|m| m.base_color_texture.map(|texture| texture.image))
        .collect();
    let mut textures = Vec::new();
    let mut places = vec![None; scene.images.len()];
    for at in named {
        let Some(image) = scene.images.get(at) else {
            continue;
        };
        let which = || {
            image
                .path
                .clone()
                .map_or(ImageRef::Embedded(at), ImageRef::File)
        };
        let pixels = read_image(&image.bytes, limits, MAX_TEXTURE_SIDE)
            .map_err(|why| no_image(asset, which(), why))?;
        let texture =
            ModelTexture::new(pixels.width, pixels.height, pixels.rgba).map_err(|error| {
                ModelError::TextureUndecodable {
                    asset: asset.to_owned(),
                    image: which(),
                    why: error.to_string(),
                }
            })?;
        places[at] = Some(textures.len());
        textures.push(Texture {
            image: Arc::new(texture),
            path: image.path.clone(),
        });
    }
    Ok((textures, places))
}

/// Why `asset`'s `image` has no pixels, as the port says it.
///
/// - Neither a PNG nor a JPEG, a header that gives no size, and data that
///   doesn't decode are all [`ModelError::TextureUndecodable`], never
///   [`ModelError::Unreadable`]: the model itself read, and nothing
///   before this adapter decodes its images, so nobody has reported it.
/// - Over the budget's `side` is [`ModelError::TextureOverBudget`],
///   naming the image as the others do, not the model's own
///   [`ModelError::OverBudget`]: the author has to know which image to
///   shrink.
/// - Over the renderer's largest side is
///   [`ModelError::TextureTooLarge`].
fn no_image(asset: &str, image: ImageRef, why: NoImage) -> ModelError {
    let asset = asset.to_owned();
    match why {
        NoImage::Unknown => ModelError::TextureUndecodable {
            asset,
            image,
            why: "it is neither a PNG nor a JPEG".to_owned(),
        },
        NoImage::Undecodable(why) => ModelError::TextureUndecodable { asset, image, why },
        NoImage::OverBudget(over) => ModelError::TextureOverBudget {
            asset,
            image,
            over: over.iter().map(overrun).collect(),
        },
        NoImage::TooLarge { side, most } => ModelError::TextureTooLarge {
            asset,
            image,
            side,
            most,
        },
    }
}

/// The request for the finished ship `ship`'s model in `galaxy`: its
/// nearest `model`, at its nearest `size`'s length. `None` for a ship
/// that doesn't resolve or has no model.
#[must_use]
pub fn ship_request(galaxy: &Galaxy, ship: &Id) -> Option<ModelRequest> {
    let resolved = resolve(galaxy, ship).ok()?;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a ship's length in metres is well inside f32"
    )]
    let length = resolved.size.map(|size| size.length as f32);
    Some(ModelRequest::new(resolved.model?.as_str(), length))
}

/// The livery the finished ship `ship` wears in `galaxy`: its nearest
/// `livery`, each slot's colour read as its bytes
/// ([`Color::rgb`](nova_galaxy_format::id::Color::rgb)). `None` for a ship
/// that doesn't resolve, wears no livery, or names one that isn't there.
#[must_use]
pub fn ship_livery(galaxy: &Galaxy, ship: &Id) -> Option<Livery> {
    let resolved = resolve(galaxy, ship).ok()?;
    Some(livery_of(&galaxy.liveries.get(resolved.livery?)?.data))
}

/// The livery a station or outpost `stellar` wears in `galaxy`: the first
/// livery, by id, whose `faction` is the stellar's `owner`, each slot's
/// colour read as its bytes, as [`ship_livery`] reads one. `None` for a
/// stellar that isn't there, has no owner, or whose owner no livery names.
#[must_use]
pub fn station_livery(galaxy: &Galaxy, stellar: &Id) -> Option<Livery> {
    let owner = galaxy.stellars.get(stellar)?.data.owner.as_ref()?;
    galaxy
        .liveries
        .values()
        .find(|livery| livery.data.faction.as_ref() == Some(owner))
        .map(|livery| livery_of(&livery.data))
}

/// `livery`'s palette, each slot's colour as its bytes.
fn livery_of(livery: &nova_galaxy_format::fleet::Livery) -> Livery {
    let palette = &livery.palette;
    Livery {
        primary: palette.primary.rgb(),
        secondary: palette.secondary.rgb(),
        trim: palette.trim.rgb(),
    }
}

/// Why `asset` has no model, as the port says it.
fn no_model(asset: &str, why: NoModel) -> ModelError {
    let asset = asset.to_owned();
    match why {
        NoModel::None => ModelError::NoFile(asset),
        NoModel::Pointer => ModelError::LfsPointer(asset),
        NoModel::Missing(file) => ModelError::Missing { asset, file },
        NoModel::Unreadable(why) => ModelError::Unreadable { asset, why },
    }
}

/// A livery slot, as the port says it. It matches every variant: each
/// slot the media crate decides has to reach the port as its own.
fn paint_slot(slot: Slot) -> PaintSlot {
    match slot {
        Slot::Primary => PaintSlot::Primary,
        Slot::Secondary => PaintSlot::Secondary,
        Slot::Trim => PaintSlot::Trim,
    }
}

/// A texture's sampler, as the port says it. It matches every variant of
/// each of its values: each wrap, filter and mip mode the media crate
/// reads has to reach the port as its own.
fn sampler(from: media::Sampler) -> ModelSampler {
    let wrap = |wrap| match wrap {
        media::Wrap::Repeat => Wrap::Repeat,
        media::Wrap::Mirror => Wrap::Mirror,
        media::Wrap::Clamp => Wrap::Clamp,
    };
    let filter = |filter| match filter {
        media::Filter::Nearest => Filter::Nearest,
        media::Filter::Linear => Filter::Linear,
    };
    ModelSampler {
        wrap_s: wrap(from.wrap_s),
        wrap_t: wrap(from.wrap_t),
        mag: filter(from.mag),
        min: filter(from.min),
        mip: match from.mip {
            media::Mip::None => Mip::None,
            media::Mip::Nearest => Mip::Nearest,
            media::Mip::Linear => Mip::Linear,
        },
    }
}

/// A budget overrun, as the port says it. It matches every measure: each
/// limit the media crate checks has to reach the port as its own.
pub(crate) fn overrun(over: &budget::Overrun) -> Overrun {
    Overrun {
        measure: match over.measure {
            budget::Measure::Triangles => Measure::Triangles,
            budget::Measure::Kib => Measure::Kib,
            budget::Measure::Side => Measure::Side,
        },
        used: over.used,
        most: over.most,
        budget: over.file(),
    }
}

/// `scene` placed by `placement`, its materials in the slots `paint`
/// puts them in and naming their base colours' textures among
/// `textures`, each through its own sampler, as the port hands it over.
/// A primitive's coordinates are forwarded as the scene resolved them
/// (the set its base colour names, transformed).
fn loaded(
    scene: &Scene,
    placement: &Placement,
    paint: Option<&Paint>,
    (textures, places): Textures,
) -> Result<LoadedModel, MeshError> {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "a placed model's metres are well inside f32"
    )]
    let narrow = |v: [f64; 3]| v.map(|c| c as f32);
    let parts = scene
        .primitives
        .iter()
        .map(|p| {
            let positions: Vec<_> = p
                .positions
                .iter()
                .map(|q| narrow(placement.ship_point_3d(*q)))
                .collect();
            let normals: Vec<_> = p
                .normals
                .iter()
                .map(|n| narrow(placement.ship_normal_3d(*n)))
                .collect();
            // Without a material, glTF's default: white.
            let base_color = p
                .material
                .and_then(|m| scene.materials.get(m))
                .map_or([1.0; 4], |m| m.base_color);
            model_source::part(
                &positions,
                &normals,
                base_color,
                p.indices.clone(),
                p.material,
                p.uvs.clone(),
            )
        })
        .collect::<Result<_, _>>()?;
    Ok(LoadedModel {
        parts,
        materials: scene
            .materials
            .iter()
            .map(|m| {
                // The texture it draws, among those decoded, and the
                // sampler it draws it through: the default for a material
                // that draws none.
                let drawn = m.base_color_texture.and_then(|texture| {
                    let at = places.get(texture.image).copied().flatten()?;
                    Some((at, sampler(texture.sampler)))
                });
                Material {
                    name: m.name.clone(),
                    paint: paint
                        .zip(m.name.as_deref())
                        .and_then(|(paint, name)| slot_of(paint, name))
                        .map(paint_slot),
                    base_color: m.base_color,
                    metallic: m.metallic,
                    roughness: m.roughness,
                    unlit: m.unlit,
                    base_color_texture: drawn.map(|(at, _)| at),
                    base_color_sampler: drawn.map_or(ModelSampler::DEFAULT, |(_, sampler)| sampler),
                }
            })
            .collect(),
        textures,
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::asset::Front;
    use nova_galaxy_format::fixture::{CC0, galaxy};
    use nova_galaxy_format::load::load;
    use nova_galaxy_format::source::{CountingSource, MemSource};
    use nova_galaxy_media::fixture::{
        Builder, ImageAt, JPEG, JPEG_SIZE, Mat, PNG, SamplerCodes, TransformJson, code, glb, png,
    };

    use super::*;
    use crate::model_source::{PaintSlot, check_contract, vertex_color};

    /// The format's test galaxy (the skiff, 8 m long, on model `hull` at
    /// `media/export/pack/hull.glb`; the `spire` placeholder; the `surface`
    /// texture; a default budget of 20000 triangles and 4096 KiB) with
    /// `hull.glb` given `bytes`.
    fn with_hull(bytes: &[u8]) -> MemSource {
        galaxy().with_bytes("media/export/pack/hull.glb", bytes)
    }

    /// What `source`'s models answer for `asset` at `length`.
    fn ask(
        source: &MemSource,
        asset: &str,
        length: Option<f32>,
    ) -> Result<LoadedModel, ModelError> {
        let (galaxy, _) = load(source);
        GalaxyModels::new(&galaxy, source).model(&ModelRequest::new(asset, length))
    }

    fn id(s: &str) -> Id {
        Id::try_from(s.to_owned()).unwrap()
    }

    #[test]
    fn an_asset_not_in_the_manifest_is_no_asset() {
        let source = with_hull(&glb([1.0; 3], [0.0; 3]));
        assert_eq!(
            ask(&source, "nothing", None),
            Err(ModelError::NoAsset("nothing".into()))
        );
        assert_eq!(
            ask(&source, "Not An Id", None),
            Err(ModelError::NoAsset("Not An Id".into()))
        );
    }

    #[test]
    fn a_texture_asset_is_not_a_model() {
        let source = with_hull(&glb([1.0; 3], [0.0; 3]));
        assert_eq!(
            ask(&source, "surface", None),
            Err(ModelError::NotAModel("surface".into()))
        );
    }

    #[test]
    fn a_placeholder_has_no_file() {
        let source = with_hull(&glb([1.0; 3], [0.0; 3]));
        assert_eq!(
            ask(&source, "spire", None),
            Err(ModelError::NoFile("spire".into()))
        );
    }

    #[test]
    fn an_lfs_pointer_is_reported_as_one() {
        let source =
            with_hull(b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n");
        assert_eq!(
            ask(&source, "hull", None),
            Err(ModelError::LfsPointer("hull".into()))
        );
    }

    #[test]
    fn a_missing_or_undecodable_file_says_which() {
        assert_eq!(
            ask(&galaxy(), "hull", None),
            Err(ModelError::Missing {
                asset: "hull".into(),
                file: "media/export/pack/hull.glb".into(),
            })
        );
        let Err(ModelError::Unreadable { asset, why }) =
            ask(&with_hull(b"glTF nonsense"), "hull", None)
        else {
            panic!("unreadable");
        };
        assert_eq!(asset, "hull");
        assert!(why.starts_with("`media/export/pack/hull.glb`"), "{why}");
    }

    #[test]
    fn an_over_budget_model_is_refused_naming_its_budget() {
        let source = with_hull(&glb([1.0; 3], [0.0; 3])).with(
            "budgets/default.md",
            "---\nmodel: {triangles: 11, kib: 4096}\n---\n",
        );
        assert_eq!(
            ask(&source, "hull", None),
            Err(ModelError::OverBudget {
                asset: "hull".into(),
                over: vec![Overrun {
                    measure: Measure::Triangles,
                    used: 12,
                    most: 11,
                    budget: "budgets/default.md".into(),
                }],
            })
        );
        let at_the_limit = source.with(
            "budgets/default.md",
            "---\nmodel: {triangles: 12, kib: 4096}\n---\n",
        );
        assert!(ask(&at_the_limit, "hull", None).is_ok());
    }

    #[test]
    fn each_media_measure_is_the_port_s_measure_of_the_same_name() {
        let over = |measure| budget::Overrun {
            measure,
            used: 9,
            most: 8,
            budget: id("default"),
        };
        for (media, port) in [
            (budget::Measure::Triangles, Measure::Triangles),
            (budget::Measure::Kib, Measure::Kib),
            (budget::Measure::Side, Measure::Side),
        ] {
            assert_eq!(
                overrun(&over(media)),
                Overrun {
                    measure: port,
                    used: 9,
                    most: 8,
                    budget: "budgets/default.md".into(),
                }
            );
        }
    }

    /// A 2 by 1 by 4 box (x, y, z) facing −z, each face its own material
    /// in the cuboid's order: −x, +x, −y, +y, −z, +z.
    fn faced_box() -> MemSource {
        let faces = [
            "minus-x", "plus-x", "minus-y", "plus-y", "minus-z", "plus-z",
        ];
        let builder = faces
            .iter()
            .enumerate()
            .fold(Builder::new([2.0, 1.0, 4.0]), |b, (at, name)| {
                b.material(Mat::new(name, [at as f32 / 8.0, 0.5, 1.0, 1.0]))
            });
        with_hull(&builder.glb()).with(
            "assets/hull.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.glb]\n\
                 front: -z\n{CC0}---\n"
            ),
        )
    }

    /// The positions and normals of the part drawn in material `material`.
    fn face(model: &LoadedModel, material: usize) -> Vec<([f32; 3], [f32; 3])> {
        let part = model
            .parts
            .iter()
            .find(|p| p.material == Some(material))
            .expect("a part in that material");
        part.mesh
            .vertices()
            .iter()
            .map(|v| (v.position, v.normal))
            .collect()
    }

    #[test]
    fn a_minus_z_model_noses_up_the_screen_at_the_ship_s_length() {
        let source = faced_box();
        let (galaxy, _) = load(&source);
        let request = ship_request(&galaxy, &id("skiff")).expect("the skiff has a model");
        assert_eq!(request, ModelRequest::new("hull", Some(8.0)));
        let model = GalaxyModels::new(&galaxy, &source).model(&request).unwrap();
        // 8 m along its 4 units: 2 m a unit. The −z face is the nose, up
        // the screen; +y the top, toward the camera; +x starboard, right.
        for (position, normal) in face(&model, 4) {
            assert_eq!(position[1], -4.0);
            assert_eq!(normal, [0.0, -1.0, 0.0]);
        }
        for (position, normal) in face(&model, 3) {
            assert_eq!(position[2], 1.0);
            assert_eq!(normal, [0.0, 0.0, 1.0]);
        }
        for (position, normal) in face(&model, 1) {
            assert_eq!(position[0], 2.0);
            assert_eq!(normal, [1.0, 0.0, 0.0]);
        }
        // Each part keeps its material's colour, baked.
        let plus_x = model.parts.iter().find(|p| p.material == Some(1)).unwrap();
        assert_eq!(
            plus_x.mesh.vertices()[0].color,
            vertex_color([1.0 / 8.0, 0.5, 1.0, 1.0])
        );
        assert_eq!(model.materials[4].name.as_deref(), Some("minus-z"));
        // As modelled, a metre a unit.
        let unscaled = GalaxyModels::new(&galaxy, &source)
            .model(&ModelRequest::new("hull", None))
            .unwrap();
        assert!(face(&unscaled, 4).iter().all(|(p, _)| p[1] == -2.0));
    }

    #[test]
    fn a_ship_s_request_comes_from_its_resolved_model_and_size() {
        let (galaxy, _) = load(&galaxy());
        let skiff = Some(ModelRequest::new("hull", Some(8.0)));
        assert_eq!(ship_request(&galaxy, &id("skiff")), skiff);
        assert_eq!(
            ship_request(&galaxy, &id("armed-skiff")),
            skiff,
            "from its base"
        );
        assert_eq!(ship_request(&galaxy, &id("nothing")), None);
    }

    #[test]
    fn a_ship_s_livery_is_its_nearest_livery_s_bytes() {
        let source = galaxy()
            .with("ships/bare.md", "---\nname: Bare\nmodel: hull\n---\n")
            .with(
                "ships/odd.md",
                "---\nname: Odd\nmodel: hull\nlivery: no-such-paint\n---\n",
            );
        let (galaxy, _) = load(&source);
        let palette = &galaxy.liveries[&id("league-paint")].data.palette;
        let league = Livery {
            primary: palette.primary.rgb(),
            secondary: palette.secondary.rgb(),
            trim: palette.trim.rgb(),
        };
        // Each slot read apart, so a swapped slot fails.
        assert_eq!(
            [league.primary, league.secondary, league.trim],
            [[0xdd, 0xcc, 0xaa], [0xaa, 0x88, 0x00], [0x22, 0x22, 0x22]]
        );
        assert_eq!(ship_livery(&galaxy, &id("skiff")), Some(league));
        assert_eq!(
            ship_livery(&galaxy, &id("armed-skiff")),
            Some(league),
            "from its base"
        );
        assert_eq!(ship_livery(&galaxy, &id("bare")), None, "wears none");
        assert_eq!(
            ship_livery(&galaxy, &id("odd")),
            None,
            "names a missing one"
        );
        assert_eq!(ship_livery(&galaxy, &id("nothing")), None, "no such ship");
    }

    #[test]
    fn a_station_wears_the_first_livery_naming_its_owner() {
        let crown_yard = "---\nname: Crown Yard\nkind: station\nsystem: alpha\n\
                          position: [9, 9]\nowner: crown\n---\n";
        let source = galaxy()
            .with(
                "factions/crown.md",
                "---\nname: Crown\nkind: minor\ncolor: \"#334455\"\n---\n",
            )
            .with("stellars/crown-yard.md", crown_yard);
        let (galaxy, problems) = load(&source);
        assert!(problems.is_empty(), "{problems:?}");
        // Each slot read apart, so a swapped slot fails.
        let league = Livery {
            primary: [0xdd, 0xcc, 0xaa],
            secondary: [0xaa, 0x88, 0x00],
            trim: [0x22, 0x22, 0x22],
        };
        assert_eq!(station_livery(&galaxy, &id("alpha-yard")), Some(league));
        assert_eq!(station_livery(&galaxy, &id("alpha-prime")), None, "unowned");
        assert_eq!(
            station_livery(&galaxy, &id("crown-yard")),
            None,
            "no livery names its owner"
        );
        assert_eq!(station_livery(&galaxy, &id("nothing")), None, "no stellar");

        // A second livery naming the league, earlier by id, is the one.
        let earlier = source.with(
            "liveries/aa-league.md",
            "---\nname: Early league\nfaction: league\n\
             palette: {primary: \"#102030\", secondary: \"#405060\", trim: \"#708090\"}\n---\n",
        );
        let (galaxy, _) = load(&earlier);
        assert_eq!(
            station_livery(&galaxy, &id("alpha-yard")),
            Some(Livery {
                primary: [0x10, 0x20, 0x30],
                secondary: [0x40, 0x50, 0x60],
                trim: [0x70, 0x80, 0x90],
            })
        );
    }

    /// A 2 by 2 image: red, green, blue and half-clear white.
    const SKIN: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128,
    ];

    /// The file a `.gltf` hull's image is.
    const SKIN_FILE: &str = "media/export/pack/skin.png";

    /// A box with texture coordinates in one textured, unlit material.
    fn skinned() -> Builder {
        Builder::new([1.0; 3]).texcoords().material(Mat {
            textured: true,
            unlit: true,
            metallic: 0.25,
            roughness: 0.5,
            ..Mat::new("hull", [1.0; 4])
        })
    }

    /// The test galaxy with `hull` the `.gltf` of `builder`, its buffer
    /// beside it and its image, `image`, at [`SKIN_FILE`].
    fn with_gltf_hull(builder: Builder, image: &[u8]) -> MemSource {
        let (json, bin) = builder
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("hull.bin");
        galaxy()
            .with(
                "assets/hull.md",
                &format!(
                    "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.gltf, \
                     media/export/pack/hull.bin, {SKIN_FILE}]\n{CC0}---\n"
                ),
            )
            .with("media/export/pack/hull.gltf", &json)
            .with_bytes("media/export/pack/hull.bin", &bin)
            .with_bytes(SKIN_FILE, image)
    }

    /// The test galaxy with `hull` the `.glb` of `builder`, `image`
    /// embedded in it.
    fn with_glb_hull(builder: Builder, image: &[u8]) -> MemSource {
        with_hull(&builder.image(ImageAt::Embedded).image_bytes(image).glb())
    }

    /// The first level of `model`'s texture `at`: its size and pixels.
    fn first_level(model: &LoadedModel, at: usize) -> (u32, u32, &[u8]) {
        let level = &model.textures[at].image.levels()[0];
        (level.width, level.height, &level.rgba)
    }

    #[test]
    fn a_gltf_with_its_bin_and_image_loads_its_image_decoded() {
        let source = with_gltf_hull(skinned(), &png(2, 2, &SKIN));
        let model = ask(&source, "hull", Some(8.0)).unwrap();
        assert_eq!(model.textures.len(), 1);
        assert_eq!(first_level(&model, 0), (2, 2, &SKIN[..]));
        assert_eq!(model.textures[0].path.as_deref(), Some(SKIN_FILE));
        assert_eq!(
            model.materials,
            [Material {
                name: Some("hull".into()),
                metallic: 0.25,
                roughness: 0.5,
                unlit: true,
                base_color_texture: Some(0),
                ..Material::new([1.0; 4])
            }]
        );
        let part = &model.parts[0];
        assert_eq!(part.material, Some(0));
        assert_eq!(
            part.uv.as_ref().map(Vec::len),
            Some(part.mesh.vertices().len())
        );
        // The merged mesh draws it: one section on the texture, unlit.
        let mesh = model.mesh();
        assert_eq!(mesh.sections().len(), 1);
        assert_eq!(mesh.sections()[0].texture, Some(0));
        assert!(mesh.vertices().iter().all(|v| v.unlit));
    }

    #[test]
    fn a_glb_s_embedded_image_loads_decoded_png_or_jpeg() {
        let model = ask(&with_glb_hull(skinned(), &png(2, 2, &SKIN)), "hull", None).unwrap();
        assert_eq!(first_level(&model, 0), (2, 2, &SKIN[..]));
        assert_eq!(model.textures[0].path, None);

        let model = ask(&with_glb_hull(skinned(), JPEG), "hull", None).unwrap();
        let (width, height, rgba) = first_level(&model, 0);
        assert_eq!((width, height), JPEG_SIZE);
        assert_eq!(rgba.len(), (width * height * 4) as usize);
    }

    #[test]
    fn an_image_no_base_colour_names_is_not_decoded() {
        // The stub doesn't decode, and nothing asks it to.
        let plain = Builder::new([1.0; 3])
            .texcoords()
            .material(Mat::new("hull", [1.0; 4]));
        for source in [
            with_glb_hull(plain.clone(), PNG),
            with_gltf_hull(plain, PNG),
        ] {
            let model = ask(&source, "hull", None).unwrap();
            assert_eq!(model.textures, []);
            assert_eq!(model.materials[0].base_color_texture, None);
        }
    }

    #[test]
    fn the_textures_are_only_the_images_named_and_each_material_names_its_own() {
        // A spare image first, which doesn't decode: the model's own is
        // image 1, and the model's only texture.
        let builder = skinned()
            .material(Mat::new("plain", [1.0; 4]))
            .material(Mat {
                textured: true,
                ..Mat::new("again", [1.0; 4])
            })
            .spare_image(PNG);
        let model = ask(&with_glb_hull(builder, &png(2, 2, &SKIN)), "hull", None).unwrap();
        assert_eq!(model.textures.len(), 1);
        assert_eq!(first_level(&model, 0), (2, 2, &SKIN[..]));
        let named: Vec<_> = model
            .materials
            .iter()
            .map(|m| m.base_color_texture)
            .collect();
        assert_eq!(named, [Some(0), None, Some(0)]);
    }

    /// A real PNG whose header is rewritten to declare `width` by
    /// `height`: its size reads, and it doesn't decode.
    fn declaring(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = png(2, 2, &SKIN);
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn an_image_over_the_default_budget_s_side_is_over_budget_naming_it() {
        let budget = |side: u32| {
            format!(
                "---\nmodel: {{triangles: 20000, kib: 4096}}\n\
                 texture: {{kib: 1, side: {side}}}\n---\n"
            )
        };
        let side = || {
            vec![Overrun {
                measure: Measure::Side,
                used: 2,
                most: 1,
                budget: "budgets/default.md".into(),
            }]
        };
        // Embedded after a spare image, it is named by its index.
        let spared = skinned().spare_image(&png(2, 2, &SKIN));
        let source = with_glb_hull(spared, &png(2, 2, &SKIN));
        let over = source.clone().with("budgets/default.md", &budget(1));
        assert_eq!(
            ask(&over, "hull", None),
            Err(ModelError::TextureOverBudget {
                asset: "hull".into(),
                image: ImageRef::Embedded(1),
                over: side(),
            })
        );
        // At the limit it loads.
        let at_the_limit = source.with("budgets/default.md", &budget(2));
        assert!(ask(&at_the_limit, "hull", None).is_ok());
        // Beside a `.gltf`, it is named by its file.
        let beside =
            with_gltf_hull(skinned(), &png(2, 2, &SKIN)).with("budgets/default.md", &budget(1));
        assert_eq!(
            ask(&beside, "hull", None),
            Err(ModelError::TextureOverBudget {
                asset: "hull".into(),
                image: ImageRef::File(SKIN_FILE.into()),
                over: side(),
            })
        );
    }

    #[test]
    fn an_image_that_does_not_decode_is_its_own_error_naming_the_asset_and_the_image() {
        // The stub: a PNG's signature, with no header to size it by.
        assert_eq!(
            ask(&with_gltf_hull(skinned(), PNG), "hull", None),
            Err(ModelError::TextureUndecodable {
                asset: "hull".into(),
                image: ImageRef::File(SKIN_FILE.into()),
                why: "its header gives no size".into(),
            })
        );
        // Embedded after a spare image, it is named by its index.
        let spared = skinned().spare_image(&png(2, 2, &SKIN));
        assert_eq!(
            ask(&with_glb_hull(spared, PNG), "hull", None),
            Err(ModelError::TextureUndecodable {
                asset: "hull".into(),
                image: ImageRef::Embedded(1),
                why: "its header gives no size".into(),
            })
        );
        // A header that reads, over data that doesn't.
        let Err(ModelError::TextureUndecodable { asset, image, why }) =
            ask(&with_glb_hull(skinned(), &declaring(4, 4)), "hull", None)
        else {
            panic!("it decoded");
        };
        assert_eq!((asset.as_str(), image), ("hull", ImageRef::Embedded(0)));
        assert_ne!(why, "its header gives no size");
    }

    #[test]
    fn bytes_that_are_no_image_on_a_textured_material_do_not_decode() {
        for source in [
            with_glb_hull(skinned(), b"GIF89a, or anything else"),
            with_glb_hull(skinned(), b""),
        ] {
            let error = ask(&source, "hull", None).unwrap_err();
            assert_eq!(
                error,
                ModelError::TextureUndecodable {
                    asset: "hull".into(),
                    image: ImageRef::Embedded(0),
                    why: "it is neither a PNG nor a JPEG".into(),
                }
            );
            // Never the error a caller takes as already reported.
            assert!(!matches!(error, ModelError::Unreadable { .. }));
        }
    }

    #[test]
    fn an_image_larger_than_the_renderer_draws_is_refused_with_no_side_budget() {
        // A default budget that gives textures no limits at all.
        let unlimited = "---\nmodel: {triangles: 20000, kib: 4096}\n---\n";
        let source = |side: u32| {
            with_gltf_hull(skinned(), &declaring(side, 2)).with("budgets/default.md", unlimited)
        };
        assert_eq!(
            ask(&source(MAX_TEXTURE_SIDE + 1), "hull", None),
            Err(ModelError::TextureTooLarge {
                asset: "hull".into(),
                image: ImageRef::File(SKIN_FILE.into()),
                side: MAX_TEXTURE_SIDE + 1,
                most: MAX_TEXTURE_SIDE,
            })
        );
        // At the most it is within it: it fails only when it decodes.
        assert!(matches!(
            ask(&source(MAX_TEXTURE_SIDE), "hull", None),
            Err(ModelError::TextureUndecodable { .. })
        ));
        // Under the test galaxy's own budget, its side limit comes first.
        let budgeted = with_gltf_hull(skinned(), &declaring(MAX_TEXTURE_SIDE + 1, 2));
        assert!(matches!(
            ask(&budgeted, "hull", None),
            Err(ModelError::TextureOverBudget { .. })
        ));
    }

    #[test]
    fn the_model_s_own_budget_is_checked_before_its_images_are_decoded() {
        let source = with_glb_hull(skinned(), PNG).with(
            "budgets/default.md",
            "---\nmodel: {triangles: 11, kib: 4096}\n---\n",
        );
        assert!(matches!(
            ask(&source, "hull", None),
            Err(ModelError::OverBudget { .. })
        ));
    }

    #[test]
    fn a_model_without_a_material_is_white() {
        let source = with_hull(&Builder::new([1.0; 3]).glb());
        let model = ask(&source, "hull", None).unwrap();
        assert_eq!(model.parts[0].material, None);
        assert_eq!(
            model.parts[0].mesh.vertices()[0].color,
            vertex_color([1.0; 4])
        );
    }

    /// The test galaxy with `hull` a flat 2 by 1 panel in the XY plane,
    /// facing +z: no depth along its front.
    fn flat_hull() -> MemSource {
        with_hull(&glb([2.0, 1.0, 0.0], [0.0; 3])).with(
            "assets/hull.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.glb]\n\
                 front: +z\n{CC0}---\n"
            ),
        )
    }

    #[test]
    fn a_model_with_no_depth_along_its_front_has_no_length_to_scale_to() {
        let error = ask(&flat_hull(), "hull", Some(8.0));
        assert_eq!(error, Err(ModelError::NoDepth("hull".into())));
        let message = error.unwrap_err().to_string();
        assert!(
            message.contains("`hull`") && message.contains("`front`"),
            "{message}"
        );
    }

    #[test]
    fn a_model_with_no_depth_along_its_front_loads_as_modelled() {
        let model = ask(&flat_hull(), "hull", None).expect("loads a metre a unit");
        let mesh = model.mesh();
        assert!(!mesh.is_empty());
        for v in mesh.vertices() {
            assert!(v.position.iter().all(|c| c.is_finite()), "{v:?}");
            assert_eq!(v.position[1], 0.0, "flat along the nose: {v:?}");
        }
        let starboard = mesh.vertices().iter().map(|v| v.position[0]);
        assert_eq!(starboard.fold(f32::MIN, f32::max), 1.0);
    }

    /// The test galaxy with `hull` a box drawn in a material for each of
    /// `names` (`""` writes one with no name), its asset painted by the
    /// YAML map `paint`, or not painted.
    fn painted_hull(names: &[&str], paint: Option<&str>) -> MemSource {
        let builder = names.iter().fold(Builder::new([2.0, 1.0, 4.0]), |b, name| {
            b.material(Mat::new(name, [0.5; 4]))
        });
        let paint = paint.map_or(String::new(), |p| format!("paint: {p}\n"));
        with_hull(&builder.glb()).with(
            "assets/hull.md",
            &format!(
                "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.glb]\n\
                 {paint}{CC0}---\n"
            ),
        )
    }

    /// Each of `source`'s `hull` materials' names and slots, in order.
    fn slots(source: &MemSource) -> Vec<(Option<String>, Option<PaintSlot>)> {
        let model = ask(source, "hull", Some(8.0)).unwrap();
        model
            .materials
            .into_iter()
            .map(|m| (m.name, m.paint))
            .collect()
    }

    fn named(name: &str, slot: Option<PaintSlot>) -> (Option<String>, Option<PaintSlot>) {
        (Some(name.to_owned()), slot)
    }

    #[test]
    fn a_painted_model_s_materials_carry_their_slots() {
        let source = painted_hull(
            &["metal", "metalRed", "metalDark", "dark"],
            Some("{primary: [metal], secondary: [metalRed], trim: [metalDark]}"),
        );
        assert_eq!(
            slots(&source),
            [
                named("metal", Some(PaintSlot::Primary)),
                named("metalRed", Some(PaintSlot::Secondary)),
                named("metalDark", Some(PaintSlot::Trim)),
                named("dark", None),
            ]
        );
    }

    #[test]
    fn paint_names_a_model_lacks_paint_nothing() {
        let source = painted_hull(&["metal", "dark"], Some("{primary: [ghost, metal]}"));
        assert_eq!(
            slots(&source),
            [
                named("metal", Some(PaintSlot::Primary)),
                named("dark", None)
            ]
        );
        let source = painted_hull(&["metal", "dark"], Some("{trim: [ghost]}"));
        assert_eq!(slots(&source), [named("metal", None), named("dark", None)]);
    }

    #[test]
    fn a_material_in_two_slots_takes_the_later() {
        let source = painted_hull(&["metal"], Some("{primary: [metal], trim: [metal]}"));
        assert_eq!(slots(&source), [named("metal", Some(PaintSlot::Trim))]);
    }

    #[test]
    fn an_unnamed_material_is_never_painted() {
        let source = painted_hull(&["", "metal"], Some("{primary: [metal]}"));
        assert_eq!(
            slots(&source),
            [(None, None), named("metal", Some(PaintSlot::Primary))]
        );
        let (galaxy, _) = load(&source);
        let models = GalaxyModels::new(&galaxy, &source);
        check_contract(&models, &ModelRequest::new("hull", None), "nothing");
    }

    #[test]
    fn a_model_without_paint_has_no_slots() {
        let source = painted_hull(&["metal", "dark"], None);
        assert_eq!(slots(&source), [named("metal", None), named("dark", None)]);
    }

    #[test]
    fn each_media_slot_is_the_port_s_slot_of_the_same_name() {
        assert_eq!(paint_slot(Slot::Primary), PaintSlot::Primary);
        assert_eq!(paint_slot(Slot::Secondary), PaintSlot::Secondary);
        assert_eq!(paint_slot(Slot::Trim), PaintSlot::Trim);
    }

    #[test]
    fn each_media_sampler_value_is_the_port_s_of_the_same_name() {
        use nova_galaxy_media::sampling as media;

        use crate::model_sampler::{Filter, Mip, Wrap};
        // Each field on its own, the others left at the default: no two
        // fields are crossed.
        let read = |change: &dyn Fn(&mut media::Sampler)| {
            let mut from = media::Sampler::DEFAULT;
            change(&mut from);
            sampler(from)
        };
        assert_eq!(read(&|_| ()), ModelSampler::DEFAULT);
        for (from, to) in [
            (media::Wrap::Repeat, Wrap::Repeat),
            (media::Wrap::Mirror, Wrap::Mirror),
            (media::Wrap::Clamp, Wrap::Clamp),
        ] {
            let along_u = ModelSampler {
                wrap_s: to,
                ..ModelSampler::DEFAULT
            };
            assert_eq!(read(&|s| s.wrap_s = from), along_u);
            let along_v = ModelSampler {
                wrap_t: to,
                ..ModelSampler::DEFAULT
            };
            assert_eq!(read(&|s| s.wrap_t = from), along_v);
        }
        for (from, to) in [
            (media::Filter::Nearest, Filter::Nearest),
            (media::Filter::Linear, Filter::Linear),
        ] {
            let mag = ModelSampler {
                mag: to,
                ..ModelSampler::DEFAULT
            };
            assert_eq!(read(&|s| s.mag = from), mag);
            let min = ModelSampler {
                min: to,
                ..ModelSampler::DEFAULT
            };
            assert_eq!(read(&|s| s.min = from), min);
        }
        for (from, to) in [
            (media::Mip::None, Mip::None),
            (media::Mip::Nearest, Mip::Nearest),
            (media::Mip::Linear, Mip::Linear),
        ] {
            let mip = ModelSampler {
                mip: to,
                ..ModelSampler::DEFAULT
            };
            assert_eq!(read(&|s| s.mip = from), mip);
        }
    }

    /// A sampler that clamps along u and reads the nearest texel, as the
    /// fixture writes it and as the port says it.
    fn sharp() -> (SamplerCodes, ModelSampler) {
        use crate::model_sampler::{Filter, Wrap};
        let codes = SamplerCodes {
            wrap_s: Some(code::CLAMP_TO_EDGE),
            mag: Some(code::NEAREST),
            ..SamplerCodes::default()
        };
        let sampler = ModelSampler {
            wrap_s: Wrap::Clamp,
            mag: Filter::Nearest,
            ..ModelSampler::DEFAULT
        };
        (codes, sampler)
    }

    #[test]
    fn two_materials_on_one_image_load_one_texture_and_their_own_samplers() {
        let (codes, sharp) = sharp();
        let builder = skinned()
            .material(Mat {
                textured: true,
                sampler: Some(codes),
                ..Mat::new("sharp", [1.0; 4])
            })
            .material(Mat::new("plain", [1.0; 4]));
        let model = ask(&with_glb_hull(builder, &png(2, 2, &SKIN)), "hull", None).unwrap();
        assert_eq!(model.textures.len(), 1, "one decode for the one image");
        let read: Vec<_> = model
            .materials
            .iter()
            .map(|m| (m.base_color_texture, m.base_color_sampler))
            .collect();
        assert_eq!(
            read,
            [
                (Some(0), ModelSampler::DEFAULT),
                (Some(0), sharp),
                (None, ModelSampler::DEFAULT),
            ]
        );
        // The merged mesh draws the one texture through each.
        let mesh = model.mesh();
        assert_eq!(mesh.textures().len(), 1);
        let drawn: Vec<_> = mesh
            .sections()
            .iter()
            .map(|s| (s.texture, s.sampler))
            .collect();
        assert_eq!(read, drawn);
    }

    #[test]
    fn a_material_whose_image_was_not_decoded_carries_the_default_sampler() {
        // The scene as read, handed over with none of its images decoded:
        // the material draws no texture, so it says nothing of a sampler.
        let (codes, sharp) = sharp();
        let builder = Builder::new([1.0; 3])
            .texcoords()
            .material(Mat {
                textured: true,
                sampler: Some(codes),
                ..Mat::new("sharp", [1.0; 4])
            })
            .image(ImageAt::Embedded);
        let scene = nova_galaxy_media::read_scene("hull.glb", &builder.glb(), |_| None).unwrap();
        assert_eq!(scene.images.len(), 1);
        let placement = Placement::unscaled_bounds(scene.bounds, Front::MinusZ);
        let texture = Texture {
            image: Arc::new(ModelTexture::new(1, 1, vec![9; 4]).unwrap()),
            path: None,
        };
        let decoded = loaded(&scene, &placement, None, (vec![texture], vec![Some(0)])).unwrap();
        assert_eq!(decoded.materials[0].base_color_texture, Some(0));
        assert_eq!(decoded.materials[0].base_color_sampler, sharp);
        let undecoded = loaded(&scene, &placement, None, (Vec::new(), vec![None])).unwrap();
        assert_eq!(undecoded.materials[0].base_color_texture, None);
        assert_eq!(
            undecoded.materials[0].base_color_sampler,
            ModelSampler::DEFAULT
        );
    }

    /// The box with both coordinate sets, its one material sampled as
    /// `material` says.
    fn two_sets(material: Mat) -> Builder {
        Builder::new([1.0; 3])
            .normals()
            .texcoords()
            .texcoords1()
            .material(Mat {
                textured: true,
                ..material
            })
    }

    /// The coordinates of `builder`'s one part, loaded, and as the media
    /// crate reads them.
    fn coordinates(builder: &Builder) -> (Vec<[f32; 2]>, Vec<[f32; 2]>) {
        let source = with_glb_hull(builder.clone(), &png(2, 2, &SKIN));
        let model = ask(&source, "hull", None).unwrap();
        let glb = builder
            .clone()
            .image(ImageAt::Embedded)
            .image_bytes(&png(2, 2, &SKIN))
            .glb();
        let scene = nova_galaxy_media::read_scene("hull.glb", &glb, |_| None).unwrap();
        (
            model.parts[0].uv.clone().expect("coordinates"),
            scene.primitives[0].uvs.clone().expect("coordinates"),
        )
    }

    #[test]
    fn a_part_s_coordinates_are_the_set_and_transform_the_scene_resolved() {
        let plain = coordinates(&two_sets(Mat::new("hull", [1.0; 4])));
        assert_eq!(plain.0, plain.1);
        // The second set, as read: not the first.
        let second = coordinates(&two_sets(Mat {
            tex_coord: 1,
            ..Mat::new("hull", [1.0; 4])
        }));
        assert_eq!(second.0, second.1);
        assert_ne!(second.0, plain.0);
        // Transformed, as read: not as written.
        let transformed = coordinates(&two_sets(Mat {
            transform: Some(TransformJson {
                offset: Some([0.25, 0.5]),
                scale: Some([2.0, 0.5]),
                ..TransformJson::default()
            }),
            ..Mat::new("hull", [1.0; 4])
        }));
        assert_eq!(transformed.0, transformed.1);
        assert_ne!(transformed.0, plain.0);
    }

    /// A box that uses all three: a sampler of its own, the second
    /// coordinate set and a transform.
    fn sampled() -> Builder {
        two_sets(Mat {
            sampler: Some(sharp().0),
            tex_coord: 1,
            transform: Some(TransformJson {
                offset: Some([0.25, 0.5]),
                rotation: Some(0.5),
                scale: Some([2.0, 0.5]),
                tex_coord: None,
            }),
            ..Mat::new("hull", [1.0; 4])
        })
    }

    #[test]
    fn the_adapter_meets_the_contract() {
        let source = faced_box();
        let (galaxy, _) = load(&source);
        let models = GalaxyModels::new(&galaxy, &source);
        check_contract(&models, &ModelRequest::new("hull", Some(8.0)), "nothing");
        check_contract(&models, &ModelRequest::new("hull", None), "nothing");
        // Textured and unlit, a `.glb` and a `.gltf`; and sampled its own
        // way, by the second set, transformed.
        for source in [
            with_glb_hull(skinned(), &png(2, 2, &SKIN)),
            with_gltf_hull(skinned(), &png(2, 2, &SKIN)),
            with_glb_hull(sampled(), &png(2, 2, &SKIN)),
            with_gltf_hull(sampled(), &png(2, 2, &SKIN)),
        ] {
            let (galaxy, _) = load(&source);
            let models = GalaxyModels::new(&galaxy, &source);
            check_contract(&models, &ModelRequest::new("hull", Some(8.0)), "nothing");
            check_contract(&models, &ModelRequest::new("hull", None), "nothing");
        }
    }

    /// The scene of `asset` as a loader reading `source` would carry it.
    fn carried(galaxy: &Galaxy, source: &impl Source, asset: &str) -> AssetScenes {
        let read = read_asset_scene(source, &galaxy.assets[&id(asset)].data);
        AssetScenes::from([(id(asset), read)])
    }

    #[test]
    fn a_scene_already_read_answers_as_reading_does_and_asks_the_source_for_nothing() {
        for source in [
            faced_box(),
            with_glb_hull(skinned(), &png(2, 2, &SKIN)),
            with_gltf_hull(skinned(), &png(2, 2, &SKIN)),
        ] {
            let (galaxy, _) = load(&source);
            let scenes = carried(&galaxy, &source, "hull");
            let counting = CountingSource::new(&source);
            let models = GalaxyModels::new(&galaxy, &counting).reading(&scenes);
            for length in [Some(8.0), None] {
                let request = ModelRequest::new("hull", length);
                let read = GalaxyModels::new(&galaxy, &source).model(&request);
                assert!(read.is_ok());
                assert_eq!(models.model(&request), read);
            }
            assert_eq!(counting.all_bytes_asked(), [], "nothing read again");
            assert_eq!(counting.heads_asked("media/export/pack/hull.glb"), 0);
        }
    }

    #[test]
    fn a_scene_carried_as_a_failure_is_the_error_reading_would_give() {
        let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n";
        for (source, asset) in [
            (with_hull(pointer), "hull"),
            (galaxy(), "hull"),
            (with_hull(b"glTF nonsense"), "hull"),
            (with_hull(&glb([1.0; 3], [0.0; 3])), "spire"),
        ] {
            let (galaxy, _) = load(&source);
            let scenes = carried(&galaxy, &source, asset);
            let counting = CountingSource::new(&source);
            let models = GalaxyModels::new(&galaxy, &counting).reading(&scenes);
            let request = ModelRequest::new(asset, None);
            let read = GalaxyModels::new(&galaxy, &source).model(&request);
            assert!(read.is_err(), "{read:?}");
            assert_eq!(models.model(&request), read);
            assert_eq!(counting.all_bytes_asked(), [], "{asset}: not read again");
        }
    }

    #[test]
    fn an_asset_with_no_scene_carried_is_read_through_the_source() {
        let source = faced_box();
        let (galaxy, _) = load(&source);
        // Scenes that hold another asset's, and none at all.
        for scenes in [carried(&galaxy, &source, "spire"), AssetScenes::new()] {
            let counting = CountingSource::new(&source);
            let models = GalaxyModels::new(&galaxy, &counting).reading(&scenes);
            let request = ModelRequest::new("hull", Some(8.0));
            assert_eq!(
                models.model(&request),
                GalaxyModels::new(&galaxy, &source).model(&request)
            );
            assert_eq!(
                counting.all_bytes_asked(),
                [("media/export/pack/hull.glb".to_owned(), 1)]
            );
        }
    }

    #[test]
    fn a_scene_of_an_asset_that_is_no_model_is_still_not_a_model() {
        // The loader reads whatever a ship or a station names; the port
        // still refuses one the manifest doesn't call a model.
        let source = with_hull(&glb([1.0; 3], [0.0; 3]));
        let (galaxy, _) = load(&source);
        let scenes = carried(&galaxy, &source, "surface");
        let models = GalaxyModels::new(&galaxy, &source).reading(&scenes);
        assert_eq!(
            models.model(&ModelRequest::new("surface", None)),
            Err(ModelError::NotAModel("surface".into()))
        );
    }

    #[test]
    fn the_adapter_meets_the_contract_reading_scenes_it_was_given() {
        for source in [faced_box(), with_gltf_hull(skinned(), &png(2, 2, &SKIN))] {
            let (galaxy, _) = load(&source);
            let scenes = carried(&galaxy, &source, "hull");
            let models = GalaxyModels::new(&galaxy, &source).reading(&scenes);
            check_contract(&models, &ModelRequest::new("hull", Some(8.0)), "nothing");
            check_contract(&models, &ModelRequest::new("hull", None), "nothing");
        }
    }
}
