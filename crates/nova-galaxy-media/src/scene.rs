//! A glTF model read whole for drawing in 3D ([`read_scene`]): each
//! triangle primitive each node draws, with its normals, texture
//! coordinates and material; the materials with their metallic, roughness,
//! unlit flag and base-colour texture; and the images, as stored. And the
//! scene an asset's files hold ([`read_asset_scene`]).
//!
//! Images are read, not decoded: an image's bytes are handed on as the
//! file holds them, and the caller decodes the ones it draws
//! ([`read_image`](crate::image::read_image)). Non-triangle primitives
//! (points, lines, strips) are skipped, as
//! [`model`](crate::model::model) skips them.
//!
//! Of a material's textures only the base colour's is read
//! ([`TextureRef`], [`sampling`](crate::sampling)): the image it names,
//! its sampler's wrap and filter modes, the `texCoord` set it is sampled
//! by and its `KHR_texture_transform`. The set and the transform are
//! resolved here, as each primitive is read: [`Primitive::uvs`] is the
//! set its material's base colour names, transformed, so whoever draws
//! applies neither. A transform that doesn't fit the extension's schema
//! makes the model unreadable, as any malformed field does. The
//! metallic-roughness, normal, occlusion and emissive textures are not
//! read, with their samplers, sets and transforms.

use std::collections::BTreeMap;

use gltf::Gltf;
use gltf::mesh::Mode;
use nova_galaxy_format::asset::{Asset, Front};
use nova_galaxy_format::id::Id;
use nova_galaxy_format::source::Source;

use crate::model::{Bounds, NoModel, beside, first_model};
use crate::sampling::{TextureRef, resolved, texture_ref};

/// One triangle primitive as one node draws it, in model space after
/// the node's transforms.
#[derive(Clone, Debug, PartialEq)]
pub struct Primitive {
    /// Each vertex's position.
    pub positions: Vec<[f32; 3]>,
    /// Each vertex's unit normal. A primitive the file gives no normals
    /// gets flat ones: each triangle's own, its vertices unwelded.
    pub normals: Vec<[f32; 3]>,
    /// Each vertex's texture coordinates, as its material's base colour
    /// is sampled by them: the `TEXCOORD_n` set the base colour names
    /// (set 0 for a primitive whose material has no base-colour texture,
    /// or with no material), with the base colour's
    /// `KHR_texture_transform` applied. `None` when the primitive has no
    /// such set, the set isn't one coordinate per vertex, or the
    /// transform leaves a coordinate that isn't finite: it then draws
    /// untextured. A missing set is never replaced by set 0, whose
    /// coordinates lay out a different image.
    pub uvs: Option<Vec<[f32; 2]>>,
    /// Indices into the vertices, three a triangle.
    pub indices: Vec<u32>,
    /// Its material, as an index into [`Scene::materials`].
    pub material: Option<usize>,
}

/// A glTF material, as read.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Its name, if it has one.
    pub name: Option<String>,
    /// Its base colour factor, linear RGBA from 0 to 1.
    pub base_color: [f32; 4],
    /// Its metallic factor.
    pub metallic: f32,
    /// Its roughness factor.
    pub roughness: f32,
    /// Whether it is unlit (`KHR_materials_unlit`).
    pub unlit: bool,
    /// The texture its base colour takes: the image, its sampler, and
    /// the coordinate set and transform it is sampled by, which
    /// [`Primitive::uvs`] already has resolved.
    pub base_color_texture: Option<TextureRef>,
}

/// An image a model holds or names, as stored: PNG or JPEG bytes, not
/// decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    /// The file's bytes.
    pub bytes: Vec<u8>,
    /// Its MIME type, if the model gives one.
    pub mime: Option<String>,
    /// The data-folder path it was read from; `None` for one embedded in
    /// the model.
    pub path: Option<String>,
}

/// A glTF model read for drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    /// Triangles drawn, counting a mesh once per node that uses it.
    pub triangles: u32,
    /// The box every drawn vertex falls in.
    pub bounds: Bounds,
    /// Every triangle primitive, once per node that draws it.
    pub primitives: Vec<Primitive>,
    /// The materials, in the file's order.
    pub materials: Vec<Material>,
    /// The images, in the file's order.
    pub images: Vec<Image>,
    /// The other files a `.gltf` names (buffers, then images), as paths
    /// in the data folder.
    pub external: Vec<String>,
    /// The bytes the model takes on disk: its own file and every file it
    /// names.
    pub disk_bytes: u64,
}

/// Reads a `.glb` or `.gltf` model at `path` from `bytes`, for drawing.
/// A `.gltf`'s buffers and images are fetched with `read` (by data-folder
/// path), and must sit in the model's folder or below it.
pub fn read_scene(
    path: &str,
    bytes: &[u8],
    read: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Scene, String> {
    let opened = open(path, bytes, &read)?;
    let primitives = walk(&opened)?;
    let bounds = bounds_of(&primitives).ok_or("the model draws no triangles")?;
    let mut disk_bytes = wide(bytes.len()) + opened.buffer_bytes;
    let mut images = Vec::new();
    for image in opened.gltf.images() {
        images.push(match image.source() {
            gltf::image::Source::View { view, mime_type } => {
                let buffer = opened
                    .buffers
                    .get(view.buffer().index())
                    .ok_or("an image's buffer does not exist")?;
                let bytes = buffer
                    .get(view.offset()..view.offset() + view.length())
                    .ok_or("an image runs past the end of its buffer")?;
                Image {
                    bytes: bytes.to_vec(),
                    mime: Some(mime_type.to_owned()),
                    path: None,
                }
            }
            gltf::image::Source::Uri { uri, mime_type } => {
                let file = beside(&opened.folder, uri)?;
                let data = read(&file).ok_or(format!("names `{file}`, which does not exist"))?;
                disk_bytes += wide(data.len());
                Image {
                    bytes: data,
                    mime: mime_type.map(String::from),
                    path: Some(file),
                }
            }
        });
    }
    let materials = opened
        .gltf
        .materials()
        .map(|m| {
            let pbr = m.pbr_metallic_roughness();
            Material {
                name: m.name().map(String::from),
                base_color: pbr.base_color_factor(),
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
                unlit: m.unlit(),
                base_color_texture: pbr.base_color_texture().map(|t| texture_ref(&t)),
            }
        })
        .collect();
    Ok(Scene {
        triangles: triangles_of(&primitives),
        bounds,
        primitives,
        materials,
        images,
        external: opened.external,
        disk_bytes,
    })
}

/// The scene `asset`'s files export, read through `source`, and which way
/// it faces: the first of its `.glb` and `.gltf` files that reads, as
/// [`read_model`](crate::model::read_model) picks it.
pub fn read_asset_scene(source: &impl Source, asset: &Asset) -> Result<(Scene, Front), NoModel> {
    first_model(source, asset, |path, bytes, read| {
        read_scene(path, bytes, read)
    })
}

/// What [`read_asset_scene`] made of each asset somebody read, by the
/// asset's id: its scene and which way it faces, or why it has none. One
/// reader keeps these so the next needn't read the files again (the
/// engine's loader reads each model for its 2D stand-in and hands them to
/// the renderer's world builder).
pub type AssetScenes = BTreeMap<Id, Result<(Scene, Front), NoModel>>;

/// Triangles drawn by `primitives`.
pub(crate) fn triangles_of(primitives: &[Primitive]) -> u32 {
    let indices: usize = primitives.iter().map(|p| p.indices.len()).sum();
    u32::try_from(indices / 3).unwrap_or(u32::MAX)
}

/// The box every vertex `primitives` draw falls in; `None` when they draw
/// none.
pub(crate) fn bounds_of(primitives: &[Primitive]) -> Option<Bounds> {
    let mut bounds: Option<Bounds> = None;
    for p in primitives {
        for &i in &p.indices {
            let point = p.positions[i as usize];
            let b = bounds.get_or_insert(Bounds {
                min: point,
                max: point,
            });
            for (k, value) in point.iter().enumerate() {
                b.min[k] = b.min[k].min(*value);
                b.max[k] = b.max[k].max(*value);
            }
        }
    }
    bounds
}

/// A count of bytes, widened.
fn wide(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// A parsed glTF file and the buffers it draws from.
pub(crate) struct Opened {
    /// The parsed file.
    pub(crate) gltf: Gltf,
    /// The folder of the model's path, which the files it names sit in.
    pub(crate) folder: String,
    /// Each buffer's bytes, in the file's order.
    pub(crate) buffers: Vec<Vec<u8>>,
    /// The other files it names (buffers, then images), as data-folder
    /// paths.
    pub(crate) external: Vec<String>,
    /// The bytes of the buffers it names, on disk.
    pub(crate) buffer_bytes: u64,
}

/// Parses the model at `path` from `bytes`, fetching the buffers a
/// `.gltf` names through `read`, and lists every file it names.
pub(crate) fn open(
    path: &str,
    bytes: &[u8],
    read: &impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Opened, String> {
    let gltf = Gltf::from_slice(bytes).map_err(|e| format!("not a readable glTF model: {e}"))?;
    let folder = path.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned();
    let mut external = Vec::new();
    let mut buffers = Vec::new();
    let mut buffer_bytes = 0;
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                buffers.push(gltf.blob.clone().ok_or("the GLB has no binary chunk")?);
            }
            gltf::buffer::Source::Uri(uri) => {
                let file = beside(&folder, uri)?;
                let data = read(&file).ok_or(format!("names `{file}`, which does not exist"))?;
                buffer_bytes += wide(data.len());
                external.push(file);
                buffers.push(data);
            }
        }
    }
    for image in gltf.images() {
        if let gltf::image::Source::Uri { uri, .. } = image.source() {
            external.push(beside(&folder, uri)?);
        }
    }
    Ok(Opened {
        gltf,
        folder,
        buffers,
        external,
        buffer_bytes,
    })
}

/// Every triangle primitive of the default scene (or of every scene,
/// without one), once per node that draws it, moved by the node's
/// transforms.
pub(crate) fn walk(opened: &Opened) -> Result<Vec<Primitive>, String> {
    let gltf = &opened.gltf;
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
    let mut primitives = Vec::new();
    while let Some((node, parent)) = stack.pop() {
        let matrix = multiply(&parent, &node.transform().matrix());
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives().filter(|p| p.mode() == Mode::Triangles) {
                let read = read_primitive(&primitive, &matrix, &opened.buffers)?;
                if !read.indices.is_empty() {
                    primitives.push(read);
                }
            }
        }
        stack.extend(node.children().map(|child| (child, matrix)));
    }
    Ok(primitives)
}

/// `p` read and moved by `matrix`.
fn read_primitive(
    p: &gltf::Primitive<'_>,
    matrix: &Matrix,
    buffers: &[Vec<u8>],
) -> Result<Primitive, String> {
    let reader = p.reader(|b| buffers.get(b.index()).map(Vec::as_slice));
    let positions: Vec<[f32; 3]> = reader
        .read_positions()
        .ok_or("a mesh has no vertex positions")?
        .map(|q| apply(matrix, q))
        .collect();
    let mut indices: Vec<u32> = match reader.read_indices() {
        Some(indices) => indices.into_u32().collect(),
        None => (0..u32::try_from(positions.len()).unwrap_or(u32::MAX)).collect(),
    };
    indices.truncate(indices.len() / 3 * 3);
    if indices.iter().any(|&i| i as usize >= positions.len()) {
        return Err("a triangle names a vertex that does not exist".into());
    }
    // The set its material's base colour is sampled by, and the transform
    // on it: set 0 as it is, for a primitive that samples no texture.
    let sampled = p
        .material()
        .pbr_metallic_roughness()
        .base_color_texture()
        .map(|t| texture_ref(&t));
    let (set, transform) = sampled.map_or((0, None), |t| (t.tex_coord, t.transform));
    let uvs: Option<Vec<[f32; 2]>> = reader
        .read_tex_coords(set)
        .map(|t| t.into_f32().collect())
        .filter(|uvs: &Vec<[f32; 2]>| uvs.len() == positions.len())
        .and_then(|uvs| resolved(uvs, transform.as_ref()));
    let normals: Option<Vec<[f32; 3]>> = reader
        .read_normals()
        .map(|n| n.map(|v| unit(turn_normal(matrix, v))).collect())
        .filter(|normals: &Vec<[f32; 3]>| normals.len() == positions.len());
    let material = p.material().index();
    Ok(match normals {
        Some(normals) => Primitive {
            positions,
            normals,
            uvs,
            indices,
            material,
        },
        None => flat(
            &positions,
            uvs.as_deref(),
            &indices,
            material,
            mirrors(matrix),
        ),
    })
}

/// A primitive of `indices` into `positions` (and `uvs`) with each
/// triangle's own normal, its corners unwelded. A triangle with no area
/// faces up (+Y). The normal follows the winding's cross product, or
/// opposes it when `mirrored`: `positions` were moved by a mirroring
/// matrix, which turned the winding over.
fn flat(
    positions: &[[f32; 3]],
    uvs: Option<&[[f32; 2]]>,
    indices: &[u32],
    material: Option<usize>,
    mirrored: bool,
) -> Primitive {
    let side = if mirrored { -1.0 } else { 1.0 };
    let mut out = Primitive {
        positions: Vec::with_capacity(indices.len()),
        normals: Vec::with_capacity(indices.len()),
        uvs: uvs.map(|_| Vec::with_capacity(indices.len())),
        indices: (0..u32::try_from(indices.len()).unwrap_or(u32::MAX)).collect(),
        material,
    };
    for tri in indices.as_chunks::<3>().0 {
        let [a, b, c] = tri.map(|i| positions[i as usize]);
        let (ab, ac) = (
            [0, 1, 2].map(|k| b[k] - a[k]),
            [0, 1, 2].map(|k| c[k] - a[k]),
        );
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let normal = if cross.iter().all(|c| *c == 0.0) {
            [0.0, 1.0, 0.0]
        } else {
            unit(cross.map(|c| side * c))
        };
        for &i in tri {
            out.positions.push(positions[i as usize]);
            out.normals.push(normal);
        }
        if let (Some(out), Some(uvs)) = (out.uvs.as_mut(), uvs) {
            out.extend(tri.iter().map(|&i| uvs[i as usize]));
        }
    }
    out
}

/// `v` scaled to unit length (a zero vector stays zero).
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 {
        v
    } else {
        v.map(|c| c / length)
    }
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

/// The normal `n` turned by the column-major matrix `m`: by the inverse
/// transpose of its linear part, so it stays at right angles to the
/// surface under a stretch, up to length. The cofactor matrix is that
/// inverse transpose times the determinant, so only the determinant's
/// sign is kept.
fn turn_normal(m: &Matrix, n: [f32; 3]) -> [f32; 3] {
    let a = |row: usize, col: usize| m[col][row];
    let cofactor = |row: usize, col: usize| {
        let (r0, r1) = ((row + 1) % 3, (row + 2) % 3);
        let (c0, c1) = ((col + 1) % 3, (col + 2) % 3);
        a(r0, c0) * a(r1, c1) - a(r0, c1) * a(r1, c0)
    };
    let sign = if mirrors(m) { -1.0 } else { 1.0 };
    [0, 1, 2].map(|row| sign * (0..3).map(|col| cofactor(row, col) * n[col]).sum::<f32>())
}

/// Whether the column-major matrix `m` mirrors what it moves: its linear
/// part's determinant is negative, which turns each triangle's winding
/// over.
fn mirrors(m: &Matrix) -> bool {
    let a = |row: usize, col: usize| m[col][row];
    let det = a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1))
        - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
        + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0));
    det < 0.0
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::fixture::{Builder, ImageAt, Mat, Node, PNG, SamplerCodes, TransformJson, code};
    use crate::sampling::{Filter, Mip, Sampler, Transform, Wrap};

    fn no_files(_: &str) -> Option<Vec<u8>> {
        None
    }

    fn scene(builder: &Builder) -> Scene {
        read_scene("m.glb", &builder.glb(), no_files).unwrap()
    }

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5)
    }

    /// A quarter turn about +y, as a quaternion: +x goes to −z.
    const QUARTER_ABOUT_Y: [f32; 4] = [
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
    ];

    #[test]
    fn a_primitive_reads_its_normals_moved_by_its_node() {
        let mut node = Node::at([5.0, 0.0, 0.0]);
        node.rotation = QUARTER_ABOUT_Y;
        // A stretch along y doesn't keep a normal's direction: the inverse
        // transpose does, and the normal stays a unit vector.
        node.scale = [1.0, 4.0, 1.0];
        let scene = scene(&Builder::new([2.0, 2.0, 2.0]).normals().nodes(vec![node]));
        let p = &scene.primitives[0];
        assert_eq!(p.normals.len(), p.positions.len());
        // Corner 4 is (+1, −1, −1), its normal (1, −1, −1)/√3. Turned, +x
        // goes to −z and −z to −x; the y stretch shrinks the normal's y.
        let i = p
            .positions
            .iter()
            .position(|q| close(*q, [4.0, -4.0, -1.0]))
            .expect("corner 4, moved");
        let length = (1.0_f32 + 0.0625 + 1.0).sqrt();
        let expected = [-1.0, -0.25, -1.0].map(|c| c / length);
        assert!(close(p.normals[i], expected), "{:?}", p.normals[i]);
    }

    #[test]
    fn a_primitive_without_normals_gets_flat_ones() {
        let scene = scene(&Builder::new([2.0, 1.0, 4.0]));
        let flat = &scene.primitives[0];
        assert_eq!(
            flat.positions.len(),
            36,
            "every corner of each triangle its own"
        );
        assert_eq!(flat.indices, (0..36).collect::<Vec<u32>>());
        let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        for tri in flat.indices.as_chunks::<3>().0 {
            let [first, second, third] = tri.map(|i| flat.positions[i as usize]);
            let normal = flat.normals[tri[0] as usize];
            assert_eq!(flat.normals[tri[1] as usize], normal);
            assert_eq!(flat.normals[tri[2] as usize], normal);
            assert!((dot(normal, normal) - 1.0).abs() < 1e-6, "{normal:?}");
            // At right angles to both edges.
            let ab = [0, 1, 2].map(|k| second[k] - first[k]);
            let ac = [0, 1, 2].map(|k| third[k] - first[k]);
            assert!(dot(ab, normal).abs() < 1e-6);
            assert!(dot(ac, normal).abs() < 1e-6);
            // Pointing along the winding's cross product.
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            assert!(dot(cross, normal) > 0.0);
        }
        assert_eq!(flat.uvs, None);
    }

    #[test]
    fn a_mirrored_node_keeps_its_flat_normals_pointing_out_of_the_surface() {
        // The box as drawn, and mirrored across x (the other wing of a
        // symmetric ship), both centred on (3, 0, 0).
        let mut mirrored = Node::at([3.0, 0.0, 0.0]);
        mirrored.scale = [-1.0, 1.0, 1.0];
        for node in [Node::at([3.0, 0.0, 0.0]), mirrored] {
            let scale = node.scale;
            let scene = scene(&Builder::new([2.0, 1.0, 4.0]).nodes(vec![node]));
            let p = &scene.primitives[0];
            for tri in p.indices.as_chunks::<3>().0 {
                let corners = tri.map(|i| p.positions[i as usize]);
                let middle = [0, 1, 2].map(|k| corners.iter().map(|c| c[k]).sum::<f32>() / 3.0);
                let out = [middle[0] - 3.0, middle[1], middle[2]];
                let normal = p.normals[tri[0] as usize];
                let dot: f32 = (0..3).map(|k| out[k] * normal[k]).sum();
                assert!(dot > 0.0, "scale {scale:?}: {normal:?} at {middle:?}");
            }
        }
    }

    #[test]
    fn texcoords_are_read_one_per_vertex() {
        let welded = scene(&Builder::new([2.0, 1.0, 4.0]).normals().texcoords());
        let p = &welded.primitives[0];
        let uvs = p.uvs.as_ref().expect("texcoords");
        assert_eq!(uvs.len(), p.positions.len());
        for (q, uv) in p.positions.iter().zip(uvs) {
            let side = |c: f32| if c > 0.0 { 1.0 } else { 0.0 };
            assert_eq!(*uv, [side(q[0]), side(q[2])], "{q:?}");
        }
        // Unwelded flat normals carry their texcoords along.
        let flat = scene(&Builder::new([2.0, 1.0, 4.0]).texcoords());
        assert_eq!(flat.primitives[0].uvs.as_ref().map(Vec::len), Some(36));
        assert_eq!(flat.primitives[0].uvs.as_ref().unwrap()[0], [0.0, 0.0]);
    }

    #[test]
    fn a_mesh_two_nodes_use_draws_twice() {
        // A group moved up 10 holding two nodes drawing the box, 3 apart.
        let nodes = vec![
            Node {
                translation: [0.0, 10.0, 0.0],
                ..Node::group(&[1, 2])
            },
            Node::at([-1.5, 0.0, 0.0]),
            Node::at([1.5, 0.0, 0.0]),
        ];
        let scene = scene(&Builder::new([1.0; 3]).nodes(nodes));
        assert_eq!(scene.primitives.len(), 2);
        assert_eq!(scene.triangles, 24);
        assert_eq!(scene.bounds.min, [-2.0, 9.5, -0.5]);
        assert_eq!(scene.bounds.max, [2.0, 10.5, 0.5]);
    }

    #[test]
    fn materials_read_metallic_roughness_unlit_and_their_texture() {
        let shiny = Mat {
            metallic: 0.25,
            roughness: 0.75,
            unlit: true,
            textured: true,
            ..Mat::new("metal", [0.5, 0.25, 0.125, 1.0])
        };
        let builder = Builder::new([1.0; 3])
            .material(shiny)
            .material(Mat::new("", [1.0; 4]))
            .image(ImageAt::Embedded);
        let scene = scene(&builder);
        assert_eq!(
            scene.materials,
            [
                Material {
                    name: Some("metal".into()),
                    base_color: [0.5, 0.25, 0.125, 1.0],
                    metallic: 0.25,
                    roughness: 0.75,
                    unlit: true,
                    base_color_texture: Some(TextureRef::image(0)),
                },
                Material {
                    name: None,
                    base_color: [1.0; 4],
                    metallic: 1.0,
                    roughness: 1.0,
                    unlit: false,
                    base_color_texture: None,
                },
            ]
        );
        let used: Vec<_> = scene.primitives.iter().map(|p| p.material).collect();
        assert_eq!(used, [Some(0), Some(1)]);
    }

    #[test]
    fn a_gltf_reads_its_image_through_the_caller() {
        let (json, bin) = Builder::new([1.0; 3])
            .material(Mat {
                textured: true,
                ..Mat::new("hull", [1.0; 4])
            })
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("hull.bin");
        let read = |path: &str| match path {
            "media/export/p/hull.bin" => Some(bin.clone()),
            "media/export/p/skin.png" => Some(PNG.to_vec()),
            _ => None,
        };
        let scene = read_scene("media/export/p/hull.gltf", json.as_bytes(), read).unwrap();
        assert_eq!(
            scene.images,
            [Image {
                bytes: PNG.to_vec(),
                mime: None,
                path: Some("media/export/p/skin.png".into()),
            }]
        );
        assert_eq!(
            scene.external,
            ["media/export/p/hull.bin", "media/export/p/skin.png"]
        );
        let total = json.len() + bin.len() + PNG.len();
        assert_eq!(scene.disk_bytes, total as u64);

        let no_image = |path: &str| (path == "media/export/p/hull.bin").then(|| bin.clone());
        let err = read_scene("media/export/p/hull.gltf", json.as_bytes(), no_image).unwrap_err();
        assert!(err.contains("`media/export/p/skin.png`"), "{err}");
    }

    #[test]
    fn a_glb_s_embedded_image_is_read_from_its_buffer() {
        let glb = Builder::new([1.0; 3]).image(ImageAt::Embedded).glb();
        let scene = read_scene("m.glb", &glb, no_files).unwrap();
        assert_eq!(
            scene.images,
            [Image {
                bytes: PNG.to_vec(),
                mime: Some("image/png".into()),
                path: None,
            }]
        );
        assert!(scene.external.is_empty());
        assert_eq!(scene.disk_bytes, glb.len() as u64);
    }

    #[test]
    fn a_gltf_image_outside_its_folder_is_an_error() {
        let (json, bin) = Builder::new([1.0; 3])
            .image(ImageAt::Uri("../skin.png".into()))
            .gltf("hull.bin");
        let read = |path: &str| (path == "media/export/p/hull.bin").then(|| bin.clone());
        let err = read_scene("media/export/p/hull.gltf", json.as_bytes(), read).unwrap_err();
        assert!(err.contains("`../skin.png`"), "{err}");
    }

    #[test]
    fn a_model_that_requires_the_texture_transform_extension_reads() {
        let builder = Builder::new([1.0; 3]).requires("KHR_texture_transform");
        assert!(read_scene("m.glb", &builder.glb(), no_files).is_ok());
        // An extension the reader doesn't know is still refused.
        let unknown = Builder::new([1.0; 3]).requires("KHR_draco_mesh_compression");
        let err = read_scene("m.glb", &unknown.glb(), no_files).unwrap_err();
        assert!(err.starts_with("not a readable glTF model"), "{err}");
    }

    /// A material on the model's image.
    fn skin() -> Mat {
        Mat {
            textured: true,
            ..Mat::new("skin", [1.0; 4])
        }
    }

    /// A box 2 by 1 by 4 with normals and both coordinate sets, its
    /// image embedded, in `materials`.
    fn sampled(materials: &[Mat]) -> Builder {
        let builder = Builder::new([2.0, 1.0, 4.0])
            .normals()
            .texcoords()
            .texcoords1()
            .image(ImageAt::Embedded);
        materials
            .iter()
            .fold(builder, |b, material| b.material(material.clone()))
    }

    /// What the first material's base colour takes in a box in `material`.
    fn base(material: Mat) -> TextureRef {
        scene(&sampled(&[material])).materials[0]
            .base_color_texture
            .expect("a textured material")
    }

    /// The sampler read from one written as `codes`.
    fn sampler(codes: SamplerCodes) -> Sampler {
        base(Mat {
            sampler: Some(codes),
            ..skin()
        })
        .sampler
    }

    #[test]
    fn a_texture_that_says_nothing_reads_the_default_sampler_set_0_and_no_transform() {
        assert_eq!(base(skin()), TextureRef::image(0));
        // A sampler that gives nothing is the default too.
        assert_eq!(sampler(SamplerCodes::default()), Sampler::DEFAULT);
        // One that gives its wraps alone keeps the default filters.
        let wraps = sampler(SamplerCodes {
            wrap_s: Some(code::CLAMP_TO_EDGE),
            wrap_t: Some(code::MIRRORED_REPEAT),
            ..SamplerCodes::default()
        });
        assert_eq!(
            wraps,
            Sampler {
                wrap_s: Wrap::Clamp,
                wrap_t: Wrap::Mirror,
                ..Sampler::DEFAULT
            }
        );
        // And one that gives its filters alone repeats.
        let filters = sampler(SamplerCodes {
            mag: Some(code::NEAREST),
            min: Some(code::NEAREST),
            ..SamplerCodes::default()
        });
        assert_eq!(
            filters,
            Sampler {
                mag: Filter::Nearest,
                min: Filter::Nearest,
                mip: Mip::None,
                ..Sampler::DEFAULT
            }
        );
    }

    #[test]
    fn each_wrap_code_reads_its_wrap_on_its_own_axis() {
        let wraps = [
            (code::REPEAT, Wrap::Repeat),
            (code::MIRRORED_REPEAT, Wrap::Mirror),
            (code::CLAMP_TO_EDGE, Wrap::Clamp),
        ];
        for (s_code, s) in wraps {
            for (t_code, t) in wraps {
                let read = sampler(SamplerCodes {
                    wrap_s: Some(s_code),
                    wrap_t: Some(t_code),
                    ..SamplerCodes::default()
                });
                assert_eq!((read.wrap_s, read.wrap_t), (s, t));
            }
        }
    }

    #[test]
    fn each_filter_code_reads_its_filter_and_its_mips() {
        for (min, filter, mip) in [
            (code::NEAREST, Filter::Nearest, Mip::None),
            (code::LINEAR, Filter::Linear, Mip::None),
            (code::NEAREST_MIPMAP_NEAREST, Filter::Nearest, Mip::Nearest),
            (code::LINEAR_MIPMAP_NEAREST, Filter::Linear, Mip::Nearest),
            (code::NEAREST_MIPMAP_LINEAR, Filter::Nearest, Mip::Linear),
            (code::LINEAR_MIPMAP_LINEAR, Filter::Linear, Mip::Linear),
        ] {
            let read = sampler(SamplerCodes {
                min: Some(min),
                ..SamplerCodes::default()
            });
            assert_eq!((read.min, read.mip), (filter, mip), "minFilter {min}");
            assert_eq!(read.mag, Filter::Linear, "minFilter {min} alone");
        }
        for (mag, filter) in [
            (code::NEAREST, Filter::Nearest),
            (code::LINEAR, Filter::Linear),
        ] {
            let read = sampler(SamplerCodes {
                mag: Some(mag),
                ..SamplerCodes::default()
            });
            assert_eq!(read.mag, filter, "magFilter {mag}");
            assert_eq!((read.min, read.mip), (Filter::Linear, Mip::Linear));
        }
    }

    #[test]
    fn two_materials_on_one_image_read_their_own_samplers() {
        let clamped = Mat {
            sampler: Some(SamplerCodes {
                wrap_s: Some(code::CLAMP_TO_EDGE),
                ..SamplerCodes::default()
            }),
            ..skin()
        };
        let sharp = Mat {
            sampler: Some(SamplerCodes {
                mag: Some(code::NEAREST),
                ..SamplerCodes::default()
            }),
            ..skin()
        };
        let scene = scene(&sampled(&[clamped, skin(), sharp]));
        let read: Vec<TextureRef> = scene
            .materials
            .iter()
            .map(|m| m.base_color_texture.expect("textured"))
            .collect();
        assert_eq!(scene.images.len(), 1);
        assert_eq!(read.iter().map(|t| t.image).collect::<Vec<_>>(), [0, 0, 0]);
        assert_eq!(
            read[0].sampler,
            Sampler {
                wrap_s: Wrap::Clamp,
                ..Sampler::DEFAULT
            }
        );
        assert_eq!(read[1].sampler, Sampler::DEFAULT);
        assert_eq!(
            read[2].sampler,
            Sampler {
                mag: Filter::Nearest,
                ..Sampler::DEFAULT
            }
        );
    }

    /// The coordinates the fixture gives a vertex at `q`: set 0, and set
    /// 1, which is set 0 with u and v swapped.
    fn fixture_uv(q: [f32; 3], set: u32) -> [f32; 2] {
        let side = |c: f32| if c > 0.0 { 1.0 } else { 0.0 };
        if set == 0 {
            [side(q[0]), side(q[2])]
        } else {
            [side(q[2]), side(q[0])]
        }
    }

    /// Asserts each vertex of `p` carries the fixture's `set`, and that
    /// the set shows: some vertex's coordinates differ from the other
    /// set's.
    #[track_caller]
    fn assert_set(p: &Primitive, set: u32) {
        let uvs = p.uvs.as_ref().expect("coordinates");
        assert_eq!(uvs.len(), p.positions.len());
        for (q, uv) in p.positions.iter().zip(uvs) {
            assert_eq!(*uv, fixture_uv(*q, set), "{q:?}");
        }
        let other = p.positions.iter().map(|q| fixture_uv(*q, 1 - set));
        assert!(other.zip(uvs).any(|(a, b)| a != *b), "the sets differ");
    }

    #[test]
    fn a_base_colour_naming_the_second_set_is_sampled_by_it() {
        let second = Mat {
            tex_coord: 1,
            ..skin()
        };
        let scene_of = |builder: &Builder| scene(builder);
        // With the file's normals, and with flat ones, which unweld the
        // vertices and carry their coordinates along.
        let welded = scene_of(&sampled(std::slice::from_ref(&second)));
        assert_eq!(welded.materials[0].base_color_texture.unwrap().tex_coord, 1);
        assert_eq!(welded.primitives[0].positions.len(), 8);
        assert_set(&welded.primitives[0], 1);
        let flat = Builder::new([2.0, 1.0, 4.0])
            .texcoords()
            .texcoords1()
            .image(ImageAt::Embedded)
            .material(second.clone());
        let flat = scene_of(&flat);
        assert_eq!(flat.primitives[0].positions.len(), 36);
        assert_set(&flat.primitives[0], 1);
        // Each primitive by its own material's set: an untextured
        // material's keeps set 0, as a primitive with no material does.
        let mixed = scene_of(&sampled(&[second, Mat::new("plain", [1.0; 4]), skin()]));
        assert_set(&mixed.primitives[0], 1);
        assert_set(&mixed.primitives[1], 0);
        assert_set(&mixed.primitives[2], 0);
        let bare = Builder::new([2.0, 1.0, 4.0]).texcoords().texcoords1();
        assert_set(&scene_of(&bare).primitives[0], 0);
    }

    #[test]
    fn a_base_colour_naming_a_set_the_mesh_lacks_has_no_coordinates() {
        // Set 2 isn't written; neither is set 1 on the second box. Set 0
        // is there, and is not fallen back to.
        let third = Mat {
            tex_coord: 2,
            ..skin()
        };
        let scene = scene(&sampled(&[third]));
        assert_eq!(scene.primitives[0].uvs, None);
        let second = Mat {
            tex_coord: 1,
            ..skin()
        };
        for normals in [true, false] {
            let builder = Builder::new([2.0, 1.0, 4.0])
                .texcoords()
                .image(ImageAt::Embedded)
                .material(second.clone());
            let builder = if normals { builder.normals() } else { builder };
            let scene = read_scene("m.glb", &builder.glb(), no_files).unwrap();
            assert_eq!(scene.primitives[0].uvs, None, "normals {normals}");
        }
    }

    #[test]
    fn a_texture_transform_is_read_and_the_coordinates_are_transformed() {
        let written = TransformJson {
            offset: Some([0.5, 0.25]),
            rotation: Some(0.75),
            scale: Some([2.0, 3.0]),
            tex_coord: None,
        };
        let material = Mat {
            transform: Some(written),
            ..skin()
        };
        let transform = Transform {
            offset: [0.5, 0.25],
            rotation: 0.75,
            scale: [2.0, 3.0],
        };
        let read = scene(&sampled(&[material.clone(), skin()]));
        assert_eq!(
            read.materials[0].base_color_texture,
            Some(TextureRef {
                transform: Some(transform),
                ..TextureRef::image(0)
            })
        );
        let p = &read.primitives[0];
        let uvs = p.uvs.as_ref().expect("coordinates");
        let mut moved = 0;
        for (q, uv) in p.positions.iter().zip(uvs) {
            let plain = fixture_uv(*q, 0);
            assert_eq!(*uv, transform.apply(plain), "{q:?}");
            moved += usize::from(*uv != plain);
        }
        assert_eq!(moved, uvs.len(), "the offset moves every coordinate");
        // The material beside it, on the same image, is not transformed.
        assert_set(&read.primitives[1], 0);
        // Flat normals carry the transformed coordinates.
        let flat = Builder::new([2.0, 1.0, 4.0])
            .texcoords()
            .image(ImageAt::Embedded)
            .material(material);
        let flat = scene(&flat);
        let p = &flat.primitives[0];
        for (q, uv) in p.positions.iter().zip(p.uvs.as_ref().unwrap()) {
            assert_eq!(*uv, transform.apply(fixture_uv(*q, 0)), "{q:?}");
        }
        // A transform that gives nothing is the identity.
        let empty = base(Mat {
            transform: Some(TransformJson::default()),
            ..skin()
        });
        assert_eq!(empty.transform, Some(Transform::IDENTITY));
    }

    #[test]
    fn a_transform_s_own_set_overrides_the_base_colour_s() {
        for (info, own, expected) in [(1, Some(0), 0), (0, Some(1), 1), (1, None, 1)] {
            let material = Mat {
                tex_coord: info,
                transform: Some(TransformJson {
                    tex_coord: own,
                    ..TransformJson::default()
                }),
                ..skin()
            };
            let scene = scene(&sampled(&[material]));
            let read = scene.materials[0].base_color_texture.unwrap();
            assert_eq!(read.tex_coord, expected, "{info}, {own:?}");
            assert_set(&scene.primitives[0], expected);
        }
    }

    #[test]
    fn a_transform_that_overflows_leaves_the_primitive_no_coordinates() {
        let huge = Mat {
            transform: Some(TransformJson {
                scale: Some([f32::MAX, f32::MAX]),
                offset: Some([f32::MAX, f32::MAX]),
                ..TransformJson::default()
            }),
            ..skin()
        };
        let scene = scene(&sampled(&[huge, skin()]));
        assert_eq!(scene.primitives[0].uvs, None);
        // The model still reads, and the material beside it is sampled.
        assert_set(&scene.primitives[1], 0);
    }

    #[test]
    fn a_coordinate_that_is_not_finite_reaches_the_primitive() {
        let scene = scene(&sampled(&[skin()]).bad_texcoord(f32::NAN));
        let uvs = scene.primitives[0].uvs.as_ref().expect("coordinates");
        assert!(uvs.iter().flatten().any(|c| c.is_nan()), "{uvs:?}");
    }

    #[test]
    fn a_texture_with_no_source_makes_the_model_unreadable() {
        let through = Mat {
            only_through: Some("KHR_texture_basisu"),
            ..skin()
        };
        let err = read_scene("m.glb", &sampled(&[through]).glb(), no_files).unwrap_err();
        assert!(err.starts_with("not a readable glTF model"), "{err}");
    }

    #[test]
    fn a_malformed_texture_transform_makes_the_model_unreadable() {
        let material = Mat {
            transform: Some(TransformJson::default()),
            ..skin()
        };
        let (json, bin) = sampled(&[material])
            .image(ImageAt::Uri("skin.png".into()))
            .gltf("m.bin");
        let read = |path: &str| match path {
            "m.bin" => Some(bin.clone()),
            "skin.png" => Some(PNG.to_vec()),
            _ => None,
        };
        assert!(read_scene("m.gltf", json.as_bytes(), read).is_ok());
        let mut broken: serde_json::Value = serde_json::from_str(&json).unwrap();
        broken["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"]["extensions"]["KHR_texture_transform"]
            ["offset"] = "left".into();
        let err = read_scene("m.gltf", broken.to_string().as_bytes(), read).unwrap_err();
        assert!(err.starts_with("not a readable glTF model"), "{err}");
        // As a sampler with a code that isn't one of glTF's does.
        let mut odd: serde_json::Value = serde_json::from_str(&json).unwrap();
        odd["samplers"] = serde_json::json!([{ "wrapS": 1234 }]);
        odd["textures"][0]["sampler"] = 0.into();
        let err = read_scene("m.gltf", odd.to_string().as_bytes(), read).unwrap_err();
        assert!(err.starts_with("not a readable glTF model"), "{err}");
    }

    #[test]
    fn non_triangle_primitives_are_skipped() {
        let scene = scene(&Builder::new([1.0; 3]).lines());
        assert_eq!(scene.primitives.len(), 1);
        assert_eq!(scene.triangles, 12);
    }

    #[test]
    fn a_model_that_draws_nothing_or_names_missing_vertices_is_an_error() {
        assert!(read_scene("m.glb", b"glTF nonsense", no_files).is_err());
        let empty = Builder::new([1.0; 3]).nodes(vec![Node::group(&[])]).glb();
        assert_eq!(
            read_scene("m.glb", &empty, no_files),
            Err("the model draws no triangles".into())
        );
    }

    /// An asset holding `files`, facing `front`.
    fn asset(files: &[&str], front: &str) -> Asset {
        let yaml = format!(
            "---\nkind: model\nplaceholder: false\nfiles: [{}]\nfront: \"{front}\"\n---\n",
            files.join(", ")
        );
        nova_galaxy_format::frontmatter::parse::<Asset>(&yaml)
            .unwrap()
            .0
    }

    #[test]
    fn an_asset_s_scene_is_its_first_model_file_that_reads() {
        let source = MemSource::default()
            .with_bytes("media/export/p/a.glb", b"glTF nonsense")
            .with_bytes("media/export/p/b.glb", &Builder::new([2.0, 1.0, 4.0]).glb());
        let asset = asset(&["media/export/p/a.glb", "media/export/p/b.glb"], "-z");
        let (scene, front) = read_asset_scene(&source, &asset).unwrap();
        assert_eq!(scene.bounds.size(), [2.0, 1.0, 4.0]);
        assert_eq!(front, Front::MinusZ);
        let missing = asset_missing();
        assert_eq!(
            read_asset_scene(&MemSource::default(), &missing),
            Err(NoModel::Missing("media/export/p/a.glb".into()))
        );
    }

    fn asset_missing() -> Asset {
        asset(&["media/export/p/a.glb"], "+z")
    }

    /// A column-major matrix with the linear part `rows` and a translation.
    fn matrix(rows: [[f32; 3]; 3]) -> Matrix {
        let mut m = IDENTITY;
        for (col, column) in m.iter_mut().take(3).enumerate() {
            for (row, cell) in column.iter_mut().take(3).enumerate() {
                *cell = rows[row][col];
            }
        }
        m[3] = [5.0, 6.0, 7.0, 1.0];
        m
    }

    #[test]
    fn a_normal_turns_by_the_inverse_transpose_keeping_its_side() {
        // [[2, 1, 0], [0, 1, 0], [1, 0, −1]] has determinant −2 and
        // inverse transpose [[½, 0, ½], [−½, 1, −½], [0, 0, −1]], which
        // takes (1, 2, 3) to (2, 0, −3).
        let m = matrix([[2.0, 1.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, -1.0]]);
        let turned = unit(turn_normal(&m, [1.0, 2.0, 3.0]));
        let expected = unit([2.0, 0.0, -3.0]);
        assert!(close(turned, expected), "{turned:?}");
        // A mirror keeps a normal pointing out of its surface.
        let mirror = matrix([[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        assert!(close(
            turn_normal(&mirror, [1.0, 0.0, 0.0]),
            [-1.0, 0.0, 0.0]
        ));
        // A node flattened to nothing along x leaves an x normal as it was.
        let flat = matrix([[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);
        assert!(close(turn_normal(&flat, [1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]));
    }

    /// A `.gltf` whose buffer `t.bin` holds `positions`, drawn without
    /// indices.
    fn unindexed(positions: &[[f32; 3]]) -> (String, Vec<u8>) {
        let bin: Vec<u8> = positions
            .iter()
            .flatten()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let min = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MAX, f32::min));
        let max = (0..3).map(|i| positions.iter().map(|p| p[i]).fold(f32::MIN, f32::max));
        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "mesh": 0 }],
            "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }],
            "buffers": [{ "uri": "t.bin", "byteLength": bin.len() }],
            "bufferViews": [{ "buffer": 0, "byteLength": bin.len() }],
            "accessors": [{ "bufferView": 0, "componentType": 5126, "count": positions.len(),
                            "type": "VEC3", "min": min.collect::<Vec<_>>(),
                            "max": max.collect::<Vec<_>>() }]
        });
        (json.to_string(), bin)
    }

    /// A `.gltf` whose buffer `t.bin` holds `positions`, drawn by the
    /// 16-bit `indices` that follow them.
    fn indexed(positions: &[[f32; 3]], indices: &[u16]) -> (String, Vec<u8>) {
        let (json, mut bin) = unindexed(positions);
        let mut json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let offset = bin.len();
        bin.extend(indices.iter().flat_map(|i| i.to_le_bytes()));
        json["buffers"][0]["byteLength"] = bin.len().into();
        json["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "buffer": 0, "byteOffset": offset, "byteLength": bin.len() - offset
            }));
        json["accessors"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "bufferView": 1, "componentType": 5123, "count": indices.len(), "type": "SCALAR"
            }));
        json["meshes"][0]["primitives"][0]["indices"] = 1.into();
        (json.to_string(), bin)
    }

    #[test]
    fn a_triangle_naming_a_vertex_past_the_last_is_an_error_not_a_panic() {
        // Three vertices, 0 to 2: index 3 is one past the last.
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let (good, good_bin) = indexed(&positions, &[0, 1, 2]);
        let (json, bin) = indexed(&positions, &[0, 1, 3]);
        let read = |bin: &Vec<u8>| {
            let bin = bin.clone();
            move |path: &str| (path == "t.bin").then(|| bin.clone())
        };
        assert!(read_scene("t.gltf", good.as_bytes(), read(&good_bin)).is_ok());
        let missing = Err("a triangle names a vertex that does not exist".to_owned());
        assert_eq!(
            read_scene("t.gltf", json.as_bytes(), read(&bin)).map(|_| ()),
            missing
        );
        assert_eq!(
            crate::model::model("t.gltf", json.as_bytes(), read(&bin)).map(|_| ()),
            missing
        );
        // Read from the data folder, the asset falls back to unreadable.
        let source = MemSource::default()
            .with_bytes("media/export/p/t.gltf", json.as_bytes())
            .with_bytes("media/export/p/t.bin", &bin);
        assert_eq!(
            read_asset_scene(&source, &asset(&["media/export/p/t.gltf"], "+z")),
            Err(NoModel::Unreadable(
                "`media/export/p/t.gltf`: a triangle names a vertex that does not exist".into()
            ))
        );
    }

    #[test]
    fn a_stray_vertex_is_dropped_and_a_triangle_with_no_area_faces_up() {
        // Four vertices without indices: one triangle, on a line, and one
        // vertex over.
        let (json, bin) = unindexed(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
        let read = |path: &str| (path == "t.bin").then(|| bin.clone());
        let scene = read_scene("t.gltf", json.as_bytes(), read).unwrap();
        let p = &scene.primitives[0];
        assert_eq!(p.indices, [0, 1, 2]);
        assert_eq!(scene.triangles, 1);
        assert_eq!(p.normals, [[0.0, 1.0, 0.0]; 3]);
    }
}
