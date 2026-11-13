//! Meshes: the triangles a 3D model ([`nova_view::DrawCommand::Model`]) is
//! drawn with, in model units on the model's axes ([`nova_view::model`]),
//! each vertex with its own normal, colour, texture coordinates and
//! whether it is unlit.
//!
//! A mesh draws in [`Section`]s: runs of its triangles, each with one
//! base-colour texture ([`ModelTexture`]) or none, sampled as the section
//! says ([`ModelSampler`]). A fragment's colour is
//! its vertex colour times its section's texel, lit unless its vertex is
//! unlit ([`crate::camera::fragment`]). A section with no texture draws
//! as if its texel were white, so in its vertex colour alone.
//!
//! [`Mesh::new`] and [`Mesh::textured`] check the triangles and the
//! sections, so a mesh the renderer hands the GPU
//! ([`crate::Gpu::upload_mesh`]) is always one it can draw. [`Mesh::new`]
//! makes a mesh with no texture, one section over all of it.
//! [`Mesh::cube`] builds the unit cube the renderer's tests draw.

use std::ops::Range;
use std::sync::Arc;

use nova_view::Color;

use crate::model_sampler::ModelSampler;
use crate::model_texture::ModelTexture;

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
    /// Where on its section's texture it is: u across from the left, v
    /// down from the top, each 0 to 1 over the image, and past it as its
    /// section's sampler wraps it. Nothing reads it in a section with no
    /// texture.
    pub uv: [f32; 2],
    /// Whether it is drawn at its colour whatever the light
    /// ([`crate::camera::fragment`]).
    pub unlit: bool,
}

/// A run of a mesh's triangles drawn with one texture through one
/// sampler, or with none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// Which of the mesh's indices it draws.
    pub indices: Range<u32>,
    /// Its base-colour texture, as an index into [`Mesh::textures`]; `None`
    /// draws its vertex colours alone.
    pub texture: Option<usize>,
    /// How it samples its texture ([`crate::model_sampler::sample`]). Two
    /// sections may name one texture through two samplers. Nothing reads
    /// it in a section with no texture.
    pub sampler: ModelSampler,
}

impl Section {
    /// The section of `indices` on `texture`, sampled the default way
    /// ([`ModelSampler::DEFAULT`]).
    #[must_use]
    pub fn new(indices: Range<u32>, texture: Option<usize>) -> Self {
        Section {
            indices,
            texture,
            sampler: ModelSampler::DEFAULT,
        }
    }

    /// The same section, sampling its texture through `sampler`.
    #[must_use]
    pub fn sampled(self, sampler: ModelSampler) -> Self {
        Section { sampler, ..self }
    }
}

/// Why [`Mesh::new`] or [`Mesh::textured`] refused a mesh.
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
    /// A section doesn't start where the one before it ends (the first,
    /// at index 0).
    #[error(
        "section {section} starts at index {starts}, not {expected} where the one before it ends"
    )]
    SectionOutOfOrder {
        /// Which section.
        section: usize,
        /// Where it starts.
        starts: u32,
        /// Where the one before it ends.
        expected: u32,
    },
    /// A section ends before it starts.
    #[error("section {section} runs backward, from index {starts} to {ends}")]
    SectionBackward {
        /// Which section.
        section: usize,
        /// Where it starts.
        starts: u32,
        /// Where it ends.
        ends: u32,
    },
    /// A section ends inside a triangle.
    #[error("section {section} ends at index {at}, inside a triangle")]
    SectionSplitsTriangle {
        /// Which section.
        section: usize,
        /// Where it ends.
        at: u32,
    },
    /// A section names a texture the mesh doesn't hold.
    #[error("section {section} names texture {texture} of {textures}")]
    NoSuchTexture {
        /// Which section.
        section: usize,
        /// The texture it names.
        texture: usize,
        /// How many textures there are.
        textures: usize,
    },
    /// The sections end short of the last index, or past it.
    #[error("the sections cover {covered} of {indices} indices")]
    SectionsMiscount {
        /// Where the last section ends.
        covered: u32,
        /// How many indices there are.
        indices: usize,
    },
}

/// Triangles to draw: vertices, indices into them taken three at a time,
/// and the sections the indices are drawn in, each with one of its
/// textures or none.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    textures: Vec<Arc<ModelTexture>>,
    sections: Vec<Section>,
    radius: f32,
}

impl Mesh {
    /// A mesh of the triangles `indices` makes of `vertices`, three
    /// indices a triangle, with no texture: one section over every index.
    /// It refuses an index that names no vertex, and indices that are not
    /// whole triangles. A mesh with no triangles is a mesh, which draws
    /// nothing.
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Result<Self, MeshError> {
        let whole = Section::new(0..indices.len() as u32, None);
        Self::textured(vertices, indices, Vec::new(), vec![whole])
    }

    /// A mesh like [`Mesh::new`]'s, drawn in `sections`, each with one of
    /// `textures` or none. Besides what [`Mesh::new`] refuses, it refuses
    /// sections that don't follow one another from index 0 to the last
    /// index, a section that ends inside a triangle, and a section that
    /// names a texture `textures` doesn't hold. A section may be empty.
    ///
    /// It holds the `textures` it is given, one no section names
    /// included, and that one is uploaded with the rest. A model's merged
    /// mesh
    /// ([`LoadedModel::mesh`](crate::model_source::LoadedModel::mesh))
    /// gives it none.
    pub fn textured(
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
        textures: Vec<Arc<ModelTexture>>,
        sections: Vec<Section>,
    ) -> Result<Self, MeshError> {
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
        let mut covered = 0;
        for (
            section,
            Section {
                indices, texture, ..
            },
        ) in sections.iter().enumerate()
        {
            if indices.start != covered {
                return Err(MeshError::SectionOutOfOrder {
                    section,
                    starts: indices.start,
                    expected: covered,
                });
            }
            if indices.end < indices.start {
                return Err(MeshError::SectionBackward {
                    section,
                    starts: indices.start,
                    ends: indices.end,
                });
            }
            if !indices.end.is_multiple_of(3) {
                return Err(MeshError::SectionSplitsTriangle {
                    section,
                    at: indices.end,
                });
            }
            if let Some(texture) = texture.filter(|&texture| texture >= textures.len()) {
                return Err(MeshError::NoSuchTexture {
                    section,
                    texture,
                    textures: textures.len(),
                });
            }
            covered = indices.end;
        }
        if covered as usize != indices.len() {
            return Err(MeshError::SectionsMiscount {
                covered,
                indices: indices.len(),
            });
        }
        let radius = vertices
            .iter()
            .map(|vertex| length(vertex.position))
            .fold(0.0, f32::max);
        Ok(Self {
            vertices,
            indices,
            textures,
            sections,
            radius,
        })
    }

    /// The unit cube centred on the origin, its corners half a unit out
    /// on each axis, each face in its own colour: `faces` in the order
    /// +x, −x, +y, −y, +z, −z (−z, the face away from the camera, last).
    /// Each face has four vertices of its own, carrying its outward
    /// normal and colour, and two triangles, each wound so that
    /// `(b − a) × (c − a)` points along the normal.
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
                    uv: [0.0; 2],
                    unlit: false,
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

    /// The textures its sections draw with, each behind a shared pointer:
    /// two meshes that share the pointer share one upload
    /// ([`crate::Gpu::upload_model_texture`]). They are the textures it
    /// was made with ([`Mesh::textured`]), every one of which is uploaded.
    #[must_use]
    pub fn textures(&self) -> &[Arc<ModelTexture>] {
        &self.textures
    }

    /// The sections its indices are drawn in, in order: together they
    /// cover every index once.
    #[must_use]
    pub fn sections(&self) -> &[Section] {
        &self.sections
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
            uv: [0.0; 2],
            unlit: false,
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

    fn texture(shade: u8) -> Arc<ModelTexture> {
        Arc::new(ModelTexture::new(1, 1, vec![shade, shade, shade, 255]).expect("one texel"))
    }

    /// Two triangles over four vertices.
    fn quad() -> (Vec<Vertex>, Vec<u32>) {
        let vertices = vec![vertex(0.0), vertex(1.0), vertex(2.0), vertex(3.0)];
        (vertices, vec![0, 1, 2, 0, 2, 3])
    }

    fn section(indices: std::ops::Range<u32>, texture: Option<usize>) -> Section {
        Section::new(indices, texture)
    }

    #[test]
    fn a_section_takes_the_default_sampler_unless_it_is_given_one() {
        use crate::model_sampler::{Filter, Wrap};
        let plain = Section::new(3..9, Some(1));
        assert_eq!(
            plain,
            Section {
                indices: 3..9,
                texture: Some(1),
                sampler: ModelSampler::DEFAULT,
            }
        );
        let sharp = ModelSampler {
            wrap_s: Wrap::Clamp,
            mag: Filter::Nearest,
            ..ModelSampler::DEFAULT
        };
        assert_eq!(
            plain.clone().sampled(sharp),
            Section {
                indices: 3..9,
                texture: Some(1),
                sampler: sharp,
            }
        );
        assert_ne!(plain.clone().sampled(sharp), plain);
        // A mesh keeps each section's sampler.
        let (vertices, indices) = quad();
        let sections = vec![
            Section::new(0..3, Some(0)).sampled(sharp),
            Section::new(3..6, Some(0)),
        ];
        let mesh = Mesh::textured(vertices, indices, vec![texture(1)], sections.clone());
        assert_eq!(mesh.expect("two sections").sections(), sections);
    }

    #[test]
    fn mesh_new_gives_one_untextured_section_over_every_index() {
        let (vertices, indices) = quad();
        let mesh = Mesh::new(vertices, indices).expect("whole");
        assert_eq!(mesh.sections(), [section(0..6, None)]);
        assert!(mesh.textures().is_empty());
        assert_eq!(cube().sections(), [section(0..36, None)]);
        let empty = Mesh::new(vec![vertex(1.0)], Vec::new()).expect("no triangles");
        assert_eq!(empty.sections(), [section(0..0, None)]);
    }

    #[test]
    fn a_textured_mesh_keeps_its_textures_and_sections() {
        let (vertices, indices) = quad();
        let textures = vec![texture(1), texture(2)];
        let sections = vec![section(0..3, Some(1)), section(3..6, None)];
        let mesh = Mesh::textured(
            vertices.clone(),
            indices.clone(),
            textures.clone(),
            sections.clone(),
        )
        .expect("in order and in range");
        assert_eq!(mesh.sections(), sections);
        assert_eq!(mesh.textures().len(), 2);
        assert!(Arc::ptr_eq(&mesh.textures()[1], &textures[1]));
        assert_eq!(mesh.vertices(), vertices);
        assert_eq!(mesh.indices(), indices);
        assert_eq!(mesh.radius(), 3.0);
        // The indices are checked as `new` checks them.
        assert_eq!(
            Mesh::textured(
                vertices,
                vec![0, 1, 9],
                Vec::new(),
                vec![section(0..3, None)]
            ),
            Err(MeshError::IndexOutOfRange {
                index: 9,
                vertices: 4
            })
        );
        // An empty section between two others draws nothing and is kept.
        let (vertices, indices) = quad();
        let with_empty = vec![
            section(0..3, None),
            section(3..3, Some(0)),
            section(3..6, None),
        ];
        let mesh = Mesh::textured(vertices, indices, vec![texture(1)], with_empty.clone());
        assert_eq!(mesh.expect("an empty section").sections(), with_empty);
    }

    /// What [`Mesh::textured`] says of the quad in `sections`, with one
    /// texture.
    fn sectioned(sections: Vec<Section>) -> Result<Mesh, MeshError> {
        let (vertices, indices) = quad();
        Mesh::textured(vertices, indices, vec![texture(1)], sections)
    }

    #[test]
    fn sections_must_follow_one_another_from_the_first_index() {
        // Starting late, leaving a gap, overlapping, and running backward.
        assert_eq!(
            sectioned(vec![section(3..6, None)]),
            Err(MeshError::SectionOutOfOrder {
                section: 0,
                starts: 3,
                expected: 0
            })
        );
        assert_eq!(
            sectioned(vec![section(0..3, None), section(6..6, None)]),
            Err(MeshError::SectionOutOfOrder {
                section: 1,
                starts: 6,
                expected: 3
            })
        );
        assert_eq!(
            sectioned(vec![section(0..6, None), section(3..6, None)]),
            Err(MeshError::SectionOutOfOrder {
                section: 1,
                starts: 3,
                expected: 6
            })
        );
    }

    #[test]
    fn a_section_must_not_run_backward() {
        #[allow(clippy::reversed_empty_ranges, reason = "the refusal under test")]
        let backward = section(6..3, None);
        // Drawn again after it, the last three indices would be covered
        // twice.
        assert_eq!(
            sectioned(vec![section(0..6, None), backward, section(3..6, None)]),
            Err(MeshError::SectionBackward {
                section: 1,
                starts: 6,
                ends: 3
            })
        );
    }

    #[test]
    fn sections_must_cover_every_index_and_no_more() {
        assert_eq!(
            sectioned(vec![section(0..3, None)]),
            Err(MeshError::SectionsMiscount {
                covered: 3,
                indices: 6
            })
        );
        assert_eq!(
            sectioned(Vec::new()),
            Err(MeshError::SectionsMiscount {
                covered: 0,
                indices: 6
            })
        );
        assert_eq!(
            sectioned(vec![section(0..9, None)]),
            Err(MeshError::SectionsMiscount {
                covered: 9,
                indices: 6
            })
        );
        // A mesh with no triangles needs no section.
        let empty = Mesh::textured(vec![vertex(1.0)], Vec::new(), Vec::new(), Vec::new());
        assert_eq!(empty.expect("nothing to cover").sections(), []);
    }

    #[test]
    fn a_section_must_end_on_a_triangle() {
        assert_eq!(
            sectioned(vec![section(0..2, None), section(2..6, None)]),
            Err(MeshError::SectionSplitsTriangle { section: 0, at: 2 })
        );
        assert_eq!(
            sectioned(vec![section(0..3, None), section(3..4, None)]),
            Err(MeshError::SectionSplitsTriangle { section: 1, at: 4 })
        );
    }

    #[test]
    fn a_section_must_name_a_texture_the_mesh_holds() {
        assert_eq!(
            sectioned(vec![section(0..3, Some(0)), section(3..6, Some(1))]),
            Err(MeshError::NoSuchTexture {
                section: 1,
                texture: 1,
                textures: 1
            })
        );
        assert!(sectioned(vec![section(0..3, Some(0)), section(3..6, Some(0))]).is_ok());
    }

    #[test]
    fn a_section_refusal_says_which_section_and_why() {
        for (error, said) in [
            (
                MeshError::SectionOutOfOrder {
                    section: 1,
                    starts: 6,
                    expected: 3,
                },
                "section 1 starts at index 6, not 3 where the one before it ends",
            ),
            (
                MeshError::SectionBackward {
                    section: 1,
                    starts: 6,
                    ends: 3,
                },
                "section 1 runs backward, from index 6 to 3",
            ),
            (
                MeshError::SectionsMiscount {
                    covered: 3,
                    indices: 6,
                },
                "the sections cover 3 of 6 indices",
            ),
            (
                MeshError::SectionSplitsTriangle { section: 0, at: 2 },
                "section 0 ends at index 2, inside a triangle",
            ),
            (
                MeshError::NoSuchTexture {
                    section: 1,
                    texture: 4,
                    textures: 2,
                },
                "section 1 names texture 4 of 2",
            ),
        ] {
            assert_eq!(error.to_string(), said);
        }
    }

    #[test]
    fn the_cube_s_vertices_are_lit_with_no_texture_coordinates() {
        for vertex in cube().vertices() {
            assert_eq!(vertex.uv, [0.0; 2]);
            assert!(!vertex.unlit);
        }
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
    fn each_cube_triangle_winds_about_its_faces_normal() {
        let cube = cube();
        for triangle in cube.indices().chunks(3) {
            let [first, second, third] = [0, 1, 2].map(|i| cube.vertices()[triangle[i] as usize]);
            let edge = |to: Vertex| [0, 1, 2].map(|i| to.position[i] - first.position[i]);
            let (ab, ac) = (edge(second), edge(third));
            let cross = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let along: f32 = (0..3).map(|i| cross[i] * first.normal[i]).sum();
            assert!(along > 0.0, "{triangle:?} winds the wrong way");
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
