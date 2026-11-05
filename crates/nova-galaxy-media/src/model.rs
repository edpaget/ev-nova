//! A glTF model's triangles, materials and footprint, read as data
//! ([`model`]).
//! Only data is read: no image is decoded, and no file a model names is
//! fetched except through the caller.
//!
//! Ported from game-bible's `galaxy` tool (`media.rs` `model`).

use gltf::Gltf;
use gltf::mesh::Mode;

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

    #[test]
    fn a_broken_model_is_an_error() {
        assert!(model("m.glb", b"glTF nonsense", no_files).is_err());
    }
}
