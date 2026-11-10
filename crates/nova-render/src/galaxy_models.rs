//! [`ModelSource`] for the new game's data: [`GalaxyModels`] reads an
//! asset's `.glb` or `.gltf` (with the buffers and images a `.gltf`
//! names) through the format's [`Source`] port, needing no GPU.
//!
//! It only chains decisions made elsewhere: which file is the model and
//! how it is read (`nova_galaxy_media::scene`), which budget covers it and
//! what it breaks (`nova_galaxy_media::budget`), how it sits on a ship
//! (`nova_galaxy_media::Placement`), and how a placed primitive becomes a
//! [`Part`](crate::model_source::Part) ([`model_source::part`]). Those are
//! pinned by their own tests; this module's tests check what it does with
//! them. It caches nothing: the caller decides when to load and which
//! [`ModelKey`](nova_view::ModelKey) a model takes
//! ([`Renderer::add_model`](crate::Renderer::add_model) with
//! [`LoadedModel::mesh`]).

use nova_galaxy_format::asset::AssetKind;
use nova_galaxy_format::fleet::resolve;
use nova_galaxy_format::id::Id;
use nova_galaxy_format::load::Galaxy;
use nova_galaxy_format::source::Source;
use nova_galaxy_media::budget::{self, limits, measure, over_budget};
use nova_galaxy_media::scene::{Scene, read_asset_scene};
use nova_galaxy_media::{NoModel, Placement};

use crate::model::MeshError;
use crate::model_source::{
    self, LoadedModel, Material, Measure, ModelError, ModelRequest, ModelSource, Overrun, Texture,
};

/// The new game's models: the asset manifest and budgets of `galaxy`, the
/// files read through `source`.
#[derive(Clone, Copy, Debug)]
pub struct GalaxyModels<'a, S: Source> {
    galaxy: &'a Galaxy,
    source: &'a S,
}

impl<'a, S: Source> GalaxyModels<'a, S> {
    /// The models `galaxy` names, read through `source`.
    #[must_use]
    pub fn new(galaxy: &'a Galaxy, source: &'a S) -> Self {
        GalaxyModels { galaxy, source }
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
        let (scene, front) =
            read_asset_scene(self.source, asset).map_err(|why| no_model(name, why))?;
        let over = over_budget(&limits(self.galaxy, &id), measure(&scene));
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
        loaded(&scene, &placement).map_err(|error| ModelError::Mesh {
            asset: name.clone(),
            error,
        })
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

/// A budget overrun, as the port says it.
fn overrun(over: &budget::Overrun) -> Overrun {
    Overrun {
        measure: match over.measure {
            budget::Measure::Triangles => Measure::Triangles,
            budget::Measure::Kib => Measure::Kib,
        },
        used: over.used,
        most: over.most,
        budget: over.file(),
    }
}

/// `scene` placed by `placement`, as the port hands it over.
fn loaded(scene: &Scene, placement: &Placement) -> Result<LoadedModel, MeshError> {
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
            .map(|m| Material {
                name: m.name.clone(),
                paint: None,
                base_color: m.base_color,
                metallic: m.metallic,
                roughness: m.roughness,
                unlit: m.unlit,
                base_color_texture: m.base_color_texture,
            })
            .collect(),
        textures: scene
            .images
            .iter()
            .map(|i| Texture {
                bytes: i.bytes.clone(),
                mime: i.mime.clone(),
                path: i.path.clone(),
            })
            .collect(),
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::fixture::{CC0, galaxy};
    use nova_galaxy_format::load::load;
    use nova_galaxy_format::source::MemSource;
    use nova_galaxy_media::fixture::{Builder, ImageAt, Mat, PNG, glb};

    use super::*;
    use crate::model_source::{check_contract, vertex_color};

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
    fn a_gltf_with_its_bin_and_image_loads() {
        let (json, bin) = Builder::new([1.0; 3])
            .texcoords()
            .material(Mat {
                textured: true,
                unlit: true,
                metallic: 0.25,
                roughness: 0.5,
                ..Mat::new("hull", [1.0; 4])
            })
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("hull.bin");
        let source = galaxy()
            .with(
                "assets/hull.md",
                &format!(
                    "---\nkind: model\nplaceholder: false\nfiles: [media/export/pack/hull.gltf, \
                     media/export/pack/hull.bin, media/export/pack/skin.png]\n{CC0}---\n"
                ),
            )
            .with("media/export/pack/hull.gltf", &json)
            .with_bytes("media/export/pack/hull.bin", &bin)
            .with_bytes("media/export/pack/skin.png", PNG);
        let model = ask(&source, "hull", Some(8.0)).unwrap();
        assert_eq!(
            model.textures,
            [Texture {
                bytes: PNG.to_vec(),
                mime: None,
                path: Some("media/export/pack/skin.png".into()),
            }]
        );
        assert_eq!(
            model.materials,
            [Material {
                name: Some("hull".into()),
                paint: None,
                base_color: [1.0; 4],
                metallic: 0.25,
                roughness: 0.5,
                unlit: true,
                base_color_texture: Some(0),
            }]
        );
        let part = &model.parts[0];
        assert_eq!(part.material, Some(0));
        assert_eq!(
            part.uv.as_ref().map(Vec::len),
            Some(part.mesh.vertices().len())
        );
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

    #[test]
    fn the_adapter_meets_the_contract() {
        let source = faced_box();
        let (galaxy, _) = load(&source);
        let models = GalaxyModels::new(&galaxy, &source);
        check_contract(&models, &ModelRequest::new("hull", Some(8.0)), "nothing");
        check_contract(&models, &ModelRequest::new("hull", None), "nothing");
    }
}
