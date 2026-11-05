//! A glTF model's triangles, materials and footprint, read as data
//! ([`model`]), and the model an asset's files hold ([`read_model`]).
//! Only data is read: no image is decoded, and no file a model names is
//! fetched except through the caller.
//!
//! Ported from game-bible's `galaxy` tool (`media.rs` `model`, and
//! `check/art.rs` `model_of`).

use gltf::Gltf;
use gltf::mesh::Mode;
use nova_galaxy_format::asset::{Asset, Front};
use nova_galaxy_format::media::is_lfs_pointer;
use nova_galaxy_format::source::Source;

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
    let gltf = Gltf::from_slice(bytes).map_err(|e| format!("not a readable glTF model: {e}"))?;
    let folder = path.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut external = Vec::new();
    let mut buffers = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                buffers.push(gltf.blob.clone().ok_or("the GLB has no binary chunk")?);
            }
            gltf::buffer::Source::Uri(uri) => {
                let file = beside(folder, uri)?;
                let data = read(&file).ok_or(format!("names `{file}`, which does not exist"))?;
                external.push(file);
                buffers.push(data);
            }
        }
    }
    for image in gltf.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            external.push(beside(folder, uri)?);
        }
    }

    let mut faces = Vec::new();
    let scenes: Vec<_> = gltf.default_scene().into_iter().collect();
    let scenes = if scenes.is_empty() {
        gltf.scenes().collect()
    } else {
        scenes
    };
    let mut stack: Vec<_> = scenes
        .iter()
        .flat_map(gltf::Scene::nodes)
        .map(|n| (n, IDENTITY))
        .collect();
    while let Some((node, parent)) = stack.pop() {
        let matrix = multiply(&parent, &node.transform().matrix());
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives().filter(|p| p.mode() == Mode::Triangles) {
                let material = primitive.material();
                let color = material.pbr_metallic_roughness().base_color_factor();
                let material = material.index();
                let reader = primitive.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or("a mesh has no vertex positions")?
                    .map(|p| apply(&matrix, p))
                    .collect();
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(indices) => indices.into_u32().collect(),
                    None => (0..u32::try_from(positions.len()).unwrap_or(u32::MAX)).collect(),
                };
                for tri in indices.as_chunks::<3>().0 {
                    let corner = |i: u32| positions.get(i as usize).copied();
                    match (corner(tri[0]), corner(tri[1]), corner(tri[2])) {
                        (Some(a), Some(b), Some(c)) => faces.push(Face {
                            points: [a, b, c],
                            color,
                            material,
                        }),
                        _ => return Err("a triangle names a vertex that does not exist".into()),
                    }
                }
            }
        }
        stack.extend(node.children().map(|child| (child, matrix)));
    }
    let mut bounds = Bounds {
        min: [f32::MAX; 3],
        max: [f32::MIN; 3],
    };
    for point in faces.iter().flat_map(|f| &f.points) {
        for (i, value) in point.iter().enumerate() {
            bounds.min[i] = bounds.min[i].min(*value);
            bounds.max[i] = bounds.max[i].max(*value);
        }
    }
    if faces.is_empty() {
        return Err("the model draws no triangles".into());
    }
    Ok(Model {
        triangles: u32::try_from(faces.len()).unwrap_or(u32::MAX),
        bounds,
        faces,
        materials: gltf
            .materials()
            .map(|m| m.name().map(String::from))
            .collect(),
        external,
    })
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
    let mut why = NoModel::None;
    for file in asset
        .files
        .iter()
        .filter(|f| matches!(f.extension().as_str(), "glb" | "gltf"))
    {
        let path = file.as_str();
        let read = match source.bytes(path) {
            Ok(Some(bytes)) if is_lfs_pointer(&bytes) => Err(NoModel::Pointer),
            Ok(Some(bytes)) => model(path, &bytes, |p| source.bytes(p).ok().flatten())
                .map_err(|e| NoModel::Unreadable(format!("`{path}`: {e}"))),
            Ok(None) => Err(NoModel::Missing(path.to_owned())),
            Err(e) => Err(NoModel::Unreadable(format!("`{path}`: {e}"))),
        };
        match read {
            Ok(model) => return Ok((model, asset.front)),
            Err(failed) if why == NoModel::None => why = failed,
            Err(_) => {}
        }
    }
    Err(why)
}

/// The data-folder path of `uri`, a file named from a model in `folder`.
fn beside(folder: &str, uri: &str) -> Result<String, String> {
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

type Matrix = [[f32; 4]; 4];

const IDENTITY: Matrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// `a * b` for column-major matrices.
fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut m = [[0.0; 4]; 4];
    for (col, out) in m.iter_mut().enumerate() {
        for (row, cell) in out.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[k][row] * b[col][k]).sum();
        }
    }
    m
}

/// The point `p` moved by the column-major matrix `m`.
fn apply(m: &Matrix, p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|row| m[0][row] * p[0] + m[1][row] * p[1] + m[2][row] * p[2] + m[3][row])
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
}
