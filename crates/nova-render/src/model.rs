//! Meshes: the triangles a 3D model ([`nova_view::DrawCommand::Model`]) is
//! drawn with, in model units on the model's axes ([`nova_view::model`]),
//! each vertex with its own normal and colour.
//!
//! [`Mesh::new`] checks the triangles, so a mesh the renderer hands the
//! GPU ([`crate::Gpu::upload_mesh`]) is always one it can draw.
//! [`Mesh::cube`] builds the unit cube the renderer's tests draw.

use nova_view::Color;

/// Which mesh the GPU holds, as the renderer numbers them
/// ([`crate::Renderer::add_model`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshId(pub u32);

/// One corner of a mesh's triangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    /// Where it is, in model units.
    pub position: [f32; 3],
    /// The surface's outward normal there, on the model's axes. The
    /// renderer normalises it, so it need not be a unit vector.
    pub normal: [f32; 3],
    /// Its colour, straight RGBA from 0 to 1.
    pub color: [f32; 4],
}

/// Why [`Mesh::new`] refused a mesh.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MeshError {
    /// An index names no vertex.
    #[error("index {index} names no vertex of {vertices}")]
    IndexOutOfRange {
        /// The index.
        index: u32,
        /// How many vertices there are.
        vertices: usize,
    },
    /// The indices are not whole triangles.
    #[error("{indices} indices are not whole triangles")]
    PartialTriangle {
        /// How many indices there are.
        indices: usize,
    },
}

/// Triangles to draw: vertices, and indices into them taken three at a
/// time.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    radius: f32,
}

impl Mesh {
    /// A mesh of the triangles `indices` makes of `vertices`, three
    /// indices a triangle. It refuses an index that names no vertex, and
    /// indices that are not whole triangles. A mesh with no triangles is
    /// a mesh, which draws nothing.
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Result<Self, MeshError> {
        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::PartialTriangle {
                indices: indices.len(),
            });
        }
        if let Some(&index) = indices
            .iter()
            .find(|&&index| index as usize >= vertices.len())
        {
            return Err(MeshError::IndexOutOfRange {
                index,
                vertices: vertices.len(),
            });
        }
        let radius = vertices
            .iter()
            .map(|vertex| length(vertex.position))
            .fold(0.0, f32::max);
        Ok(Self {
            vertices,
            indices,
            radius,
        })
    }

    /// The unit cube centred on the origin, its corners half a unit out
    /// on each axis, each face in its own colour: `faces` in the order
    /// +x, −x, +y, −y, +z, −z (−z, the face away from the camera, last).
    /// Each face has four vertices of its own, carrying its outward
    /// normal and colour, and two triangles.
    #[must_use]
    pub fn cube(faces: [Color; 6]) -> Self {
        // Each face's normal and two edges along it, `u × v = normal`.
        const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
            ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
            ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
            ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        ];
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for ((normal, u, v), color) in FACES.iter().zip(faces) {
            let base = vertices.len() as u32;
            for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let position = [0, 1, 2].map(|i| (normal[i] + su * u[i] + sv * v[i]) / 2.0);
                vertices.push(Vertex {
                    position,
                    normal: *normal,
                    color: [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0),
                });
            }
            indices.extend([0, 1, 2, 0, 2, 3].map(|i| base + i));
        }
        Self::new(vertices, indices).expect("the cube's indices are whole and in range")
    }

    /// Its vertices.
    #[must_use]
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Its indices, three a triangle.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Whether it has no triangles, and so draws nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// How far its furthest vertex is from the origin, in model units: no
    /// turn moves any vertex further, which bounds its depth
    /// ([`crate::camera`]).
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.radius
    }
}

/// The length of `v`.
fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn vertex(x: f32) -> Vertex {
        Vertex {
            position: [x, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            color: [1.0; 4],
        }
    }

    const COLORS: [Color; 6] = [
        Color::rgba(255, 0, 0, 255),
        Color::rgba(0, 255, 0, 255),
        Color::rgba(0, 0, 255, 255),
        Color::rgba(255, 255, 0, 255),
        Color::rgba(0, 255, 255, 255),
        Color::rgba(255, 0, 255, 128),
    ];

    fn cube() -> Mesh {
        Mesh::cube(COLORS)
    }

    /// The vertices of face `face` of `mesh`, four a face.
    fn face(mesh: &Mesh, face: usize) -> &[Vertex] {
        &mesh.vertices()[face * 4..face * 4 + 4]
    }

    #[test]
    fn mesh_new_rejects_an_index_out_of_range() {
        let vertices = vec![vertex(0.0), vertex(1.0), vertex(2.0)];
        assert_eq!(
            Mesh::new(vertices.clone(), vec![0, 1, 3]),
            Err(MeshError::IndexOutOfRange {
                index: 3,
                vertices: 3
            })
        );
        assert!(Mesh::new(vertices, vec![0, 1, 2]).is_ok());
    }

    #[test]
    fn mesh_new_rejects_a_partial_triangle() {
        let vertices = vec![vertex(0.0), vertex(1.0), vertex(2.0)];
        assert_eq!(
            Mesh::new(vertices.clone(), vec![0, 1, 2, 0]),
            Err(MeshError::PartialTriangle { indices: 4 })
        );
        assert_eq!(
            Mesh::new(vertices, vec![0, 1]),
            Err(MeshError::PartialTriangle { indices: 2 })
        );
    }

    #[test]
    fn a_mesh_with_no_triangles_is_empty() {
        let empty = Mesh::new(vec![vertex(1.0)], Vec::new()).expect("no triangles");
        assert!(empty.is_empty());
        assert!(!cube().is_empty());
    }

    #[test]
    fn a_meshs_radius_is_its_furthest_vertex() {
        let mesh =
            Mesh::new(vec![vertex(1.0), vertex(-3.0), vertex(2.0)], vec![0, 1, 2]).expect("whole");
        assert_eq!(mesh.radius(), 3.0);
        assert!((cube().radius() - 3.0_f32.sqrt() / 2.0).abs() < 1e-6);
    }

    #[test]
    fn the_cube_has_six_faces_of_two_triangles() {
        let cube = cube();
        assert_eq!(cube.vertices().len(), 24);
        assert_eq!(cube.indices().len(), 36);
        // Each face's two triangles use its own four vertices alone.
        for (at, triangles) in cube.indices().chunks(6).enumerate() {
            let face = (at * 4) as u32..(at * 4 + 4) as u32;
            assert!(triangles.iter().all(|i| face.contains(i)), "{triangles:?}");
            let mut used = triangles.to_vec();
            used.sort_unstable();
            used.dedup();
            assert_eq!(used.len(), 4, "face {at} uses all its corners");
        }
    }

    #[test]
    fn each_cube_face_normal_points_out_of_its_face() {
        let cube = cube();
        for at in 0..6 {
            let corners = face(&cube, at);
            let normal = corners[0].normal;
            assert_eq!(normal.iter().map(|c| c.abs()).sum::<f32>(), 1.0);
            for corner in corners {
                assert_eq!(corner.normal, normal);
                // Every corner lies half a unit out along the normal.
                let out: f32 = (0..3).map(|i| corner.position[i] * normal[i]).sum();
                assert_eq!(out, 0.5, "face {at}");
            }
        }
        let normals: Vec<[f32; 3]> = (0..6).map(|at| face(&cube, at)[0].normal).collect();
        assert_eq!(
            normals,
            [
                [1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
            ]
        );
    }

    #[test]
    fn cube_corners_lie_at_half_a_unit() {
        let cube = cube();
        for vertex in cube.vertices() {
            assert_eq!(vertex.position.map(f32::abs), [0.5; 3], "{vertex:?}");
        }
        // Each face's four corners differ.
        for at in 0..6 {
            let corners = face(&cube, at);
            for (i, a) in corners.iter().enumerate() {
                for b in &corners[i + 1..] {
                    assert_ne!(a.position, b.position);
                }
            }
        }
    }

    #[test]
    fn each_face_carries_its_colour() {
        let cube = cube();
        for (at, color) in COLORS.iter().enumerate() {
            let expected = [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0);
            for corner in face(&cube, at) {
                assert_eq!(corner.color, expected, "face {at}");
            }
        }
    }

    #[test]
    fn the_minus_z_face_comes_last() {
        let cube = cube();
        let last = &cube.indices()[30..];
        for &index in last {
            assert_eq!(cube.vertices()[index as usize].normal, [0.0, 0.0, -1.0]);
        }
    }
}
