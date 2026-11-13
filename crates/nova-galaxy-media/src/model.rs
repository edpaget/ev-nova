//! A glTF model's triangles, materials and footprint, read as data
//! ([`model`]), and the model an asset's files hold ([`read_model`]).
//! Only data is read: no image is decoded, and no file a model names is
//! fetched except through the caller.
//!
//! Ported from game-bible's `galaxy` tool (`media.rs` `model`, and
//! `check/art.rs` `model_of`). It walks the glTF file as
//! [`read_scene`](crate::scene::read_scene) does, through the same code,
//! and keeps only the triangles' corners and colours. So a scene already
//! read gives the same model without reading the file again
//! ([`Model::from_scene`]).

use nova_galaxy_format::asset::{Asset, Front};
use nova_galaxy_format::media::is_lfs_pointer;
use nova_galaxy_format::source::Source;

use crate::scene::{self, Primitive, Scene};

/// The box a model fills, in its own units, after its nodes' transforms.
/// glTF axes: +Y is up, +Z is the front, +X is the model's left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// The smallest x, y and z.
    pub min: [f32; 3],
    /// The largest x, y and z.
    pub max: [f32; 3],
}

impl Bounds {
    /// The middle of the box.
    #[must_use]
    pub fn center(&self) -> [f32; 3] {
        [0, 1, 2].map(|i| f32::midpoint(self.min[i], self.max[i]))
    }

    /// The box's size along x, y and z.
    #[must_use]
    pub fn size(&self) -> [f32; 3] {
        [0, 1, 2].map(|i| self.max[i] - self.min[i])
    }
}

/// One triangle of a model.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    /// Its corners, in model space after node transforms.
    pub points: [[f32; 3]; 3],
    /// Its material's base colour, RGBA from 0 to 1 (white without one).
    pub color: [f32; 4],
    /// Its material, as an index into [`Model::materials`] (`None`
    /// without one).
    pub material: Option<usize>,
}

/// What is read from a glTF model.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    /// Triangles drawn, counting a mesh once per node that uses it.
    pub triangles: u32,
    /// The box every vertex falls in.
    pub bounds: Bounds,
    /// Every triangle, in model space after node transforms.
    pub faces: Vec<Face>,
    /// Each glTF material's name, in the file's order (`None` for one
    /// with no name).
    pub materials: Vec<Option<String>>,
    /// The other files a `.gltf` names (buffers, images), as paths in the
    /// data folder.
    pub external: Vec<String>,
}

/// Reads a `.glb` or `.gltf` model at `path` from `bytes`. A `.gltf`'s
/// external files are fetched with `read` (by data-folder path), and must
/// sit in the model's folder or below it.
pub fn model(
    path: &str,
    bytes: &[u8],
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Model, String> {
    let opened = scene::open(path, bytes, &read)?;
    let primitives = scene::walk(&opened)?;
    let bounds = scene::bounds_of(&primitives).ok_or("the model draws no triangles")?;
    let materials: Vec<_> = opened.gltf.materials().collect();
    let faces = faces(&primitives, |m| {
        let material = materials.get(m)?;
        Some(material.pbr_metallic_roughness().base_color_factor())
    });
    Ok(Model {
        triangles: scene::triangles_of(&primitives),
        bounds,
        faces,
        materials: materials
            .iter()
            .map(|m| m.name().map(String::from))
            .collect(),
        external: opened.external,
    })
}

impl Model {
    /// The model of a scene already read
    /// ([`read_scene`](crate::scene::read_scene)): what [`model`] reads
    /// from the same file, so a caller that draws a model in 3D and needs
    /// its triangles as data too reads the file once.
    #[must_use]
    pub fn from_scene(scene: &Scene) -> Model {
        Model {
            triangles: scene.triangles,
            bounds: scene.bounds,
            faces: faces(&scene.primitives, |m| {
                scene.materials.get(m).map(|material| material.base_color)
            }),
            materials: scene.materials.iter().map(|m| m.name.clone()).collect(),
            external: scene.external.clone(),
        }
    }
}

/// Every triangle of `primitives`, each in its material's base colour as
/// `base_color` gives it by index: white, glTF's default, without a
/// material or for one the file doesn't hold. [`model`] and
/// [`Model::from_scene`] both build their faces here.
fn faces(primitives: &[Primitive], base_color: impl Fn(usize) -> Option<[f32; 4]>) -> Vec<Face> {
    let mut faces = Vec::new();
    for p in primitives {
        let color = p.material.and_then(&base_color).unwrap_or([1.0; 4]);
        for tri in p.indices.as_chunks::<3>().0 {
            faces.push(Face {
                points: [0, 1, 2].map(|k| p.positions[tri[k] as usize]),
                color,
                material: p.material,
            });
        }
    }
    faces
}

/// Why an asset has no model to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoModel {
    /// None of its files is a model (`.glb` or `.gltf`): a placeholder
    /// with no files, or a texture.
    None,
    /// Its model file is a Git LFS pointer: the checkout hasn't fetched
    /// it.
    Pointer,
    /// Its model file, at this path, isn't there.
    Missing(String),
    /// Its model file doesn't read as a model; why, naming the file.
    Unreadable(String),
}

/// The model `asset`'s files export, read through `source`, and which way
/// it faces: the first of its `.glb` and `.gltf` files that reads,
/// whatever its `placeholder` flag (as the contact sheet's `model_of`).
/// Without one, why: the first model file's failure, or
/// [`NoModel::None`] when it names no model file.
pub fn read_model(source: &impl Source, asset: &Asset) -> Result<(Model, Front), NoModel> {
    first_model(source, asset, |path, bytes, read| model(path, bytes, read))
}

/// What `decode` makes of the first of `asset`'s `.glb` and `.gltf` files
/// that reads through `source`, and which way it faces; without one, the
/// first model file's failure, or [`NoModel::None`] when it names none.
pub(crate) fn first_model<T>(
    source: &impl Source,
    asset: &Asset,
    decode: impl Fn(&str, &[u8], &dyn Fn(&str) -> Option<Vec<u8>>) -> Result<T, String>,
) -> Result<(T, Front), NoModel> {
    let mut why = NoModel::None;
    let read = |p: &str| source.bytes(p).ok().flatten();
    for file in asset
        .files
        .iter()
        .filter(|f| matches!(f.extension().as_str(), "glb" | "gltf"))
    {
        let path = file.as_str();
        let decoded = match source.bytes(path) {
            Ok(Some(bytes)) if is_lfs_pointer(&bytes) => Err(NoModel::Pointer),
            Ok(Some(bytes)) => decode(path, &bytes, &read)
                .map_err(|e| NoModel::Unreadable(format!("`{path}`: {e}"))),
            Ok(None) => Err(NoModel::Missing(path.to_owned())),
            Err(e) => Err(NoModel::Unreadable(format!("`{path}`: {e}"))),
        };
        match decoded {
            Ok(decoded) => return Ok((decoded, asset.front)),
            Err(failed) if why == NoModel::None => why = failed,
            Err(_) => {}
        }
    }
    Err(why)
}

/// The data-folder path of `uri`, a file named from a model in `folder`.
pub(crate) fn beside(folder: &str, uri: &str) -> Result<String, String> {
    let plain = !uri.is_empty()
        && !uri.contains(':')
        && !uri.contains('%')
        && !uri.contains('\\')
        && uri
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    if !plain {
        return Err(format!(
            "names `{uri}`; a model may only name files in its own folder or below \
             (no `..`, no `data:` or other URIs; export a .glb to embed them)"
        ));
    }
    Ok(if folder.is_empty() {
        uri.to_string()
    } else {
        format!("{folder}/{uri}")
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::fixture;

    fn no_files(_: &str) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn a_glb_counts_its_triangles_and_measures_its_footprint() {
        let model = model("m.glb", &fixture::glb([2.0, 1.0, 4.0], [0.0; 3]), no_files).unwrap();
        assert_eq!(model.triangles, 12);
        assert_eq!(model.bounds.min, [-1.0, -0.5, -2.0]);
        assert_eq!(model.bounds.max, [1.0, 0.5, 2.0]);
        assert_eq!(model.faces.len(), 12);
        assert_eq!(
            model.faces[0].color, [1.0; 4],
            "the fixture's hull is white"
        );
        assert!(model.external.is_empty());
    }

    #[test]
    fn a_model_names_its_materials_and_each_face_its_own() {
        let glb = fixture::glb_with_materials(
            [2.0, 1.0, 4.0],
            [0.0; 3],
            &[("hull", [1.0; 4]), ("glass", [0.2, 0.4, 0.8, 1.0])],
        );
        let model = model("m.glb", &glb, no_files).unwrap();
        assert_eq!(model.materials, [Some("hull".into()), Some("glass".into())]);
        let glass = model.faces.iter().filter(|f| f.material == Some(1)).count();
        let hull = model.faces.iter().filter(|f| f.material == Some(0)).count();
        assert_eq!((hull, glass), (6, 6));
        let glass_face = model.faces.iter().find(|f| f.material == Some(1)).unwrap();
        assert_eq!(glass_face.color, [0.2, 0.4, 0.8, 1.0]);
    }

    #[test]
    fn a_face_without_a_material_names_none() {
        let glb = fixture::glb_with_materials([2.0, 1.0, 4.0], [0.0; 3], &[]);
        let model = model("m.glb", &glb, no_files).unwrap();
        assert!(model.materials.is_empty());
        assert!(model.faces.iter().all(|f| f.material.is_none()));
        assert_eq!(model.faces[0].color, [1.0; 4], "no material is white");
    }

    #[test]
    fn a_material_without_a_name_is_none() {
        let glb = fixture::glb_with_materials([1.0; 3], [0.0; 3], &[("", [1.0; 4])]);
        let model = model("m.glb", &glb, no_files).unwrap();
        assert_eq!(model.materials, [None]);
    }

    #[test]
    fn node_transforms_move_the_footprint() {
        let model = model(
            "m.glb",
            &fixture::glb([2.0, 1.0, 4.0], [2.0, 0.0, 1.5]),
            no_files,
        )
        .unwrap();
        assert_eq!(model.bounds.min, [1.0, -0.5, -0.5]);
        assert_eq!(model.bounds.center(), [2.0, 0.0, 1.5]);
    }

    #[test]
    fn a_gltf_reads_its_buffer_through_the_caller() {
        let (json, bin) = fixture::gltf_with_bin([2.0, 1.0, 4.0], "hull.bin");
        let read = |path: &str| (path == "media/export/pack/hull.bin").then(|| bin.clone());
        let model = model("media/export/pack/hull.gltf", json.as_bytes(), read).unwrap();
        assert_eq!(model.triangles, 12);
        assert_eq!(model.external, ["media/export/pack/hull.bin"]);
    }

    #[test]
    fn a_gltf_whose_buffer_is_missing_is_an_error() {
        let (json, _) = fixture::gltf_with_bin([1.0; 3], "hull.bin");
        let err = model("media/export/pack/hull.gltf", json.as_bytes(), no_files).unwrap_err();
        assert!(err.contains("`media/export/pack/hull.bin`"), "{err}");
    }

    #[test]
    fn a_gltf_may_not_reach_outside_its_folder() {
        let (json, _) = fixture::gltf_with_bin([1.0; 3], "../other/hull.bin");
        let err = model("media/export/pack/hull.gltf", json.as_bytes(), no_files).unwrap_err();
        assert!(err.contains("../other/hull.bin"), "{err}");
    }

    /// An asset of `kind` (a model unless it says) holding `files`.
    fn asset(files: &[&str], front: &str) -> Asset {
        let yaml = format!(
            "---\nkind: model\nplaceholder: false\nfiles: [{}]\nfront: \"{front}\"\n---\n",
            files.join(", ")
        );
        nova_galaxy_format::frontmatter::parse::<Asset>(&yaml)
            .unwrap()
            .0
    }

    const POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n";

    #[test]
    fn an_asset_s_model_is_the_first_of_its_files_that_reads() {
        let source = MemSource::default()
            .with_bytes("media/export/p/flat.png", b"\x89PNG")
            .with_bytes(
                "media/export/p/a.glb",
                &fixture::glb([2.0, 1.0, 4.0], [0.0; 3]),
            )
            .with_bytes("media/export/p/b.glb", &fixture::glb([1.0; 3], [0.0; 3]));
        let asset = asset(
            &[
                "media/export/p/flat.png",
                "media/export/p/a.glb",
                "media/export/p/b.glb",
            ],
            "-z",
        );
        let (model, front) = read_model(&source, &asset).unwrap();
        assert_eq!(model.bounds.size(), [2.0, 1.0, 4.0]);
        assert_eq!(front, Front::MinusZ);
    }

    #[test]
    fn a_model_after_one_that_fails_is_read() {
        let source = MemSource::default()
            .with_bytes("media/export/p/a.glb", POINTER)
            .with_bytes(
                "media/export/p/b.GLTF",
                fixture::gltf_with_bin([1.0; 3], "b.bin").0.as_bytes(),
            )
            .with_bytes(
                "media/export/p/b.bin",
                &fixture::gltf_with_bin([1.0; 3], "b.bin").1,
            );
        let asset = asset(&["media/export/p/a.glb", "media/export/p/b.GLTF"], "+z");
        let (model, front) = read_model(&source, &asset).unwrap();
        assert_eq!(model.external, ["media/export/p/b.bin"]);
        assert_eq!(front, Front::PlusZ);
    }

    #[test]
    fn an_asset_with_no_model_file_has_none() {
        let placeholder = nova_galaxy_format::frontmatter::parse::<Asset>(
            "---\nkind: model\nplaceholder: true\n---\n",
        )
        .unwrap()
        .0;
        assert_eq!(
            read_model(&MemSource::default(), &placeholder),
            Err(NoModel::None)
        );
        let texture = MemSource::default().with_bytes("media/export/p/flat.png", b"\x89PNG");
        let asset = asset(&["media/export/p/flat.png", "media/export/p/a.bin"], "+z");
        assert_eq!(read_model(&texture, &asset), Err(NoModel::None));
    }

    #[test]
    fn an_lfs_pointer_is_a_pointer() {
        let source = MemSource::default().with_bytes("media/export/p/a.glb", POINTER);
        let asset = asset(&["media/export/p/a.glb"], "+z");
        assert_eq!(read_model(&source, &asset), Err(NoModel::Pointer));
    }

    #[test]
    fn an_undecodable_model_is_unreadable_naming_its_file() {
        let source = MemSource::default().with_bytes("media/export/p/a.glb", b"glTF nonsense");
        let asset = asset(&["media/export/p/a.glb"], "+z");
        let Err(NoModel::Unreadable(why)) = read_model(&source, &asset) else {
            panic!("unreadable");
        };
        assert!(
            why.starts_with("`media/export/p/a.glb`: not a readable glTF model"),
            "{why}"
        );
    }

    #[test]
    fn a_missing_model_is_missing_naming_its_file() {
        let asset = asset(&["media/export/p/a.glb"], "+z");
        assert_eq!(
            read_model(&MemSource::default(), &asset),
            Err(NoModel::Missing("media/export/p/a.glb".into()))
        );
    }

    #[test]
    fn the_first_failure_is_the_reason() {
        let source = MemSource::default()
            .with_bytes("media/export/p/a.glb", POINTER)
            .with_bytes("media/export/p/b.glb", b"nonsense");
        let asset = asset(&["media/export/p/a.glb", "media/export/p/b.glb"], "+z");
        assert_eq!(read_model(&source, &asset), Err(NoModel::Pointer));
        let asset = asset_of(&["media/export/p/b.glb", "media/export/p/a.glb"]);
        assert!(matches!(
            read_model(&source, &asset),
            Err(NoModel::Unreadable(_))
        ));
    }

    fn asset_of(files: &[&str]) -> Asset {
        asset(files, "+z")
    }

    /// A source whose reads fail.
    struct Broken;

    impl Source for Broken {
        fn files(
            &self,
            _: &str,
            _: &str,
        ) -> std::io::Result<Vec<nova_galaxy_format::source::SourceFile>> {
            Ok(Vec::new())
        }

        fn bytes(&self, _: &str) -> std::io::Result<Option<Vec<u8>>> {
            Err(std::io::Error::other("the disk is on fire"))
        }
    }

    #[test]
    fn a_failed_read_is_unreadable_naming_its_file() {
        assert_eq!(
            read_model(&Broken, &asset_of(&["media/export/p/a.glb"])),
            Err(NoModel::Unreadable(
                "`media/export/p/a.glb`: the disk is on fire".into()
            ))
        );
    }

    #[test]
    fn a_broken_model_is_an_error() {
        assert!(model("m.glb", b"glTF nonsense", no_files).is_err());
    }

    #[test]
    fn a_file_a_model_names_is_found_beside_it() {
        assert_eq!(beside("media/p", "hull.bin"), Ok("media/p/hull.bin".into()));
        assert_eq!(
            beside("media/p", "sub/hull.bin"),
            Ok("media/p/sub/hull.bin".into())
        );
        assert_eq!(beside("", "hull.bin"), Ok("hull.bin".into()));
    }

    #[test]
    fn a_model_may_name_no_uri_and_nothing_outside_its_folder() {
        for uri in [
            "",
            "data:application/octet-stream;base64,AA",
            "a%20b.bin",
            "a\\b.bin",
            "./a.bin",
            "a//b.bin",
            "../a.bin",
            "a/..",
        ] {
            let err = beside("media/p", uri).unwrap_err();
            assert!(err.contains(&format!("`{uri}`")), "{uri}: {err}");
        }
    }

    #[test]
    fn a_scene_s_model_is_the_model_its_file_reads_as() {
        use crate::fixture::{Builder, ImageAt, Mat, Node, SamplerCodes, TransformJson, code};
        use crate::scene::read_scene;

        let named = || {
            Builder::new([2.0, 1.0, 4.0])
                .material(Mat::new("hull", [1.0; 4]))
                .material(Mat::new("glass", [0.2, 0.4, 0.8, 0.5]))
        };
        // A base colour with a sampler, the second coordinate set and a
        // transform: none of it reaches the 2D model.
        let sampled = || {
            let skin = Mat {
                textured: true,
                sampler: Some(SamplerCodes {
                    wrap_s: Some(code::CLAMP_TO_EDGE),
                    mag: Some(code::NEAREST),
                    ..SamplerCodes::default()
                }),
                tex_coord: 1,
                transform: Some(TransformJson {
                    rotation: Some(0.5),
                    ..TransformJson::default()
                }),
                ..Mat::new("skin", [1.0; 4])
            };
            let builder = Builder::new([2.0, 1.0, 4.0]).texcoords().texcoords1();
            builder.material(skin).image(ImageAt::Embedded)
        };
        let tree = vec![
            Node::group(&[1, 2]),
            Node::at([0.0, 0.5, 0.0]),
            Node {
                rotation: [0.0, 0.707_106_77, 0.0, 0.707_106_77],
                scale: [2.0, 1.0, 0.5],
                ..Node::at([3.0, 0.0, -1.0])
            },
        ];
        let mut shapes: Vec<(&str, Vec<u8>)> = vec![
            ("plain", Builder::new([2.0, 1.0, 4.0]).glb()),
            ("named materials", named().glb()),
            (
                "an unnamed material",
                Builder::new([1.0; 3])
                    .material(Mat::new("", [0.5; 4]))
                    .glb(),
            ),
            ("a moved node", named().at([2.0, 0.0, 1.5]).glb()),
            ("a tree of nodes", named().nodes(tree).glb()),
            ("with normals", named().normals().glb()),
            ("with texture coordinates", named().texcoords().glb()),
            ("with lines", named().lines().glb()),
            ("an embedded image", named().image(ImageAt::Embedded).glb()),
            ("a sampled texture", sampled().glb()),
        ];
        let (json, bin) = named()
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("hull.bin");
        shapes.push(("a .gltf with its files", json.into_bytes()));
        let read = |path: &str| match path {
            "media/export/pack/hull.bin" => Some(bin.clone()),
            "media/export/pack/skin.png" => Some(fixture::PNG.to_vec()),
            _ => None,
        };
        for (shape, bytes) in &shapes {
            let path = "media/export/pack/hull.gltf";
            let scene = read_scene(path, bytes, read).unwrap_or_else(|e| panic!("{shape}: {e}"));
            let model = model(path, bytes, read).unwrap_or_else(|e| panic!("{shape}: {e}"));
            assert_eq!(Model::from_scene(&scene), model, "{shape}");
            assert!(!model.faces.is_empty(), "{shape}");
        }
        // The last names both its files, buffer first.
        let last = &shapes.last().expect("the .gltf").1;
        let scene = read_scene("media/export/pack/hull.gltf", last, read).unwrap();
        assert_eq!(
            Model::from_scene(&scene).external,
            ["media/export/pack/hull.bin", "media/export/pack/skin.png"]
        );
    }

    #[test]
    fn each_face_keeps_its_corners_where_its_node_puts_them() {
        // A 4 by 1 by 2 box moved to (0.5, 3, -1): its first face is the
        // -x side's first triangle, corners 0, 1 and 3 of the cuboid.
        let model = model(
            "m.glb",
            &fixture::glb([4.0, 1.0, 2.0], [0.5, 3.0, -1.0]),
            no_files,
        )
        .unwrap();
        assert_eq!(model.bounds.min, [-1.5, 2.5, -2.0]);
        assert_eq!(model.bounds.max, [2.5, 3.5, 0.0]);
        assert_eq!(
            model.faces[0].points,
            [[-1.5, 2.5, -2.0], [-1.5, 2.5, 0.0], [-1.5, 3.5, 0.0]]
        );
    }
}
