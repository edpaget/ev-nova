//! Liveries painted onto a model's materials.
//!
//! A [`Livery`] is three sRGB colours, one per [`PaintSlot`]. A model's
//! source puts each material it can paint in a slot
//! ([`Material::paint`]); [`painted`] sets each such material's base
//! colour to its slot's colour, decoded to linear ([`paint_color`]), so a
//! [`Material`] is linear everywhere, and rebakes the vertex colours of
//! the parts drawn in it ([`vertex_color`]). A painted vertex then shows
//! the livery's own bytes, and painted and unpainted parts reach the
//! screen the same way.
//!
//! Painting changes nothing else. It keeps each material's alpha,
//! metallic, roughness, `unlit` and texture; every position, normal,
//! index and texture coordinate; the parts' order; and every part it
//! doesn't paint as it was. A part with no material, or whose material
//! has no slot, isn't painted. A painted material with a texture keeps
//! it, by the same shared pointer, so one hull in two liveries uploads
//! its texture once. The texture is sampled and multiplied by the vertex
//! colour ([`crate::camera::fragment`]), so the livery tints the texture
//! as the data's "keeps its textures" asks, as an unpainted textured
//! material's own factor tints its own.
//!
//! Which slot a material is in is the source's reading of the model's
//! `paint`. The galaxy adapter takes it from `nova_galaxy_media::slot_of`,
//! the rule the top-down silhouettes use too: a material named in two
//! slots takes the later one (trim over secondary over primary), and a
//! material with no name is in none (the port's contract holds every
//! source to that).
//!
//! # One mesh per model and livery
//!
//! Painting happens once, on the CPU, when a model is registered: the
//! caller paints a [`LoadedModel`] once per livery it draws and registers
//! each painted [`LoadedModel::mesh`] under its own
//! [`ModelKey`](nova_view::ModelKey)
//! ([`Renderer::add_model`](crate::Renderer::add_model)), so one hull in
//! two liveries is two keys. The other way, one mesh with the colours
//! given per instance, would need a slot index in every vertex, three
//! colours in every drawn model's pose and a branch in the shader, across
//! `nova-view` and the GPU path, to save memory few hulls in few liveries
//! don't need. Revisit it if liveries become per ship (a paint for each
//! pilot, say), when the meshes would grow with the ships drawn.
//!
//! # Where an addition goes
//!
//! A new slot is a [`PaintSlot`] variant, a field of [`Livery`] (and its
//! arm in [`Livery::color`]), and the adapter's reading of the media
//! crate's slot ([`crate::galaxy_models`]).

use crate::model::{Mesh, Vertex};
use crate::model_source::{LoadedModel, Material, Part, linear, vertex_color};

/// A livery slot: which of a livery's colours paints a material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintSlot {
    /// The main hull colour.
    Primary,
    /// Panels and stripes.
    Secondary,
    /// Small details and lights.
    Trim,
}

/// A livery's colours: red, green and blue sRGB bytes for each slot, as
/// data writes them (`#rrggbb`). Every slot has one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Livery {
    /// The main hull colour.
    pub primary: [u8; 3],
    /// Panels and stripes.
    pub secondary: [u8; 3],
    /// Small details and lights.
    pub trim: [u8; 3],
}

impl Livery {
    /// The colour it gives `slot`.
    #[must_use]
    pub fn color(&self, slot: PaintSlot) -> [u8; 3] {
        match slot {
            PaintSlot::Primary => self.primary,
            PaintSlot::Secondary => self.secondary,
            PaintSlot::Trim => self.trim,
        }
    }
}

/// The linear base colour the sRGB colour `rgb` sets, with `alpha`, the
/// material's own, kept.
#[must_use]
pub fn paint_color(rgb: [u8; 3], alpha: f32) -> [f32; 4] {
    let [r, g, b] = rgb.map(|c| linear(f32::from(c) / 255.0));
    [r, g, b, alpha]
}

/// `model` in `livery`: each material with a slot in that slot's colour,
/// and the parts drawn in it rebaked; the rest as it was (see the
/// [module docs](self)).
#[must_use]
pub fn painted(model: &LoadedModel, livery: &Livery) -> LoadedModel {
    let materials: Vec<Material> = model
        .materials
        .iter()
        .map(|m| match m.paint {
            Some(slot) => Material {
                base_color: paint_color(livery.color(slot), m.base_color[3]),
                ..m.clone()
            },
            None => m.clone(),
        })
        .collect();
    let parts = model
        .parts
        .iter()
        .map(|part| {
            let painted = paint_of(part, &materials);
            let mut part = part.clone();
            if let Some(material) = painted {
                part.mesh = recolored(&part.mesh, vertex_color(material.base_color));
            }
            part
        })
        .collect();
    LoadedModel {
        parts,
        materials,
        textures: model.textures.clone(),
    }
}

/// Whether a livery would paint any of `model`: some part of it is drawn
/// in a material with a slot. When none is, [`painted`] leaves every part,
/// and so the model's mesh, as it was.
#[must_use]
pub(crate) fn takes_paint(model: &LoadedModel) -> bool {
    model
        .parts
        .iter()
        .any(|part| paint_of(part, &model.materials).is_some())
}

/// The material of `materials` that paints `part`: the one it is drawn
/// in, when that has a slot. The one rule for which parts a livery paints
/// (see the [module docs](self)): [`painted`] rebakes the parts it names,
/// and [`takes_paint`] asks whether it names any.
fn paint_of<'a>(part: &Part, materials: &'a [Material]) -> Option<&'a Material> {
    part.material
        .and_then(|at| materials.get(at))
        .filter(|material| material.paint.is_some())
}

/// `mesh` with every vertex coloured `color`.
fn recolored(mesh: &Mesh, color: [f32; 4]) -> Mesh {
    let vertices = mesh
        .vertices()
        .iter()
        .map(|v| Vertex { color, ..*v })
        .collect();
    Mesh::new(vertices, mesh.indices().to_vec()).expect("recolouring keeps the indices")
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::model_source::Texture;
    use crate::model_texture::ModelTexture;

    /// Three liveries' colours apart in every slot and channel.
    const LEAGUE: Livery = Livery {
        primary: [0xdd, 0xcc, 0xaa],
        secondary: [0xaa, 0x88, 0x00],
        trim: [0x22, 0x33, 0x44],
    };
    const RAIDER: Livery = Livery {
        primary: [0x80, 0x10, 0x20],
        secondary: [0x10, 0x80, 0x20],
        trim: [0x20, 0x10, 0x80],
    };

    /// A material `name` in `slot`, its own colour grey.
    fn material(name: &str, slot: Option<PaintSlot>) -> Material {
        Material {
            name: Some(name.to_owned()),
            paint: slot,
            base_color: [0.25, 0.5, 0.75, 1.0],
            metallic: 1.0,
            roughness: 1.0,
            unlit: false,
            base_color_texture: None,
        }
    }

    /// The unit cube's six faces as six parts, face `at` in material
    /// `materials[at]`, each coloured by its material's base colour (or
    /// white without one), with texture coordinates.
    fn cube(materials: Vec<Material>, faces: [Option<usize>; 6]) -> LoadedModel {
        let cube = Mesh::cube([nova_view::Color::WHITE; 6]);
        let parts = faces
            .iter()
            .enumerate()
            .map(|(at, material)| {
                let color = material
                    .and_then(|m| materials.get(m))
                    .map_or([1.0; 4], |m| vertex_color(m.base_color));
                let vertices = cube.vertices()[at * 4..at * 4 + 4]
                    .iter()
                    .map(|v| Vertex { color, ..*v })
                    .collect();
                let indices = cube.indices()[at * 6..at * 6 + 6]
                    .iter()
                    .map(|i| i - (at * 4) as u32)
                    .collect();
                Part {
                    mesh: Mesh::new(vertices, indices).unwrap(),
                    material: *material,
                    uv: Some(vec![[at as f32 / 8.0, 0.5]; 4]),
                }
            })
            .collect();
        LoadedModel {
            parts,
            materials,
            textures: vec![Texture {
                image: Arc::new(ModelTexture::new(1, 1, vec![9, 9, 9, 255]).unwrap()),
                path: None,
            }],
        }
    }

    /// A cube in the three slots, an unpainted material, no material and
    /// a material index past the last: faces +x primary, −x secondary,
    /// +y trim, −y unpainted, +z no material, −z out of range.
    fn slotted() -> LoadedModel {
        cube(
            vec![
                material("metal", Some(PaintSlot::Primary)),
                material("metalRed", Some(PaintSlot::Secondary)),
                material("metalDark", Some(PaintSlot::Trim)),
                material("dark", None),
            ],
            [Some(0), Some(1), Some(2), Some(3), None, Some(9)],
        )
    }

    fn colors(part: &Part) -> Vec<[f32; 4]> {
        part.mesh.vertices().iter().map(|v| v.color).collect()
    }

    #[test]
    fn a_livery_gives_each_slot_its_own_colour() {
        assert_eq!(LEAGUE.color(PaintSlot::Primary), LEAGUE.primary);
        assert_eq!(LEAGUE.color(PaintSlot::Secondary), LEAGUE.secondary);
        assert_eq!(LEAGUE.color(PaintSlot::Trim), LEAGUE.trim);
    }

    #[test]
    fn a_paint_colour_is_the_livery_s_bytes_decoded_to_linear_its_alpha_kept() {
        let [r, g, b, a] = paint_color([0x80, 0x0a, 0xff], 0.5);
        assert_eq!(r, linear(128.0 / 255.0));
        assert_eq!(g, linear(10.0 / 255.0));
        assert_eq!(b, linear(1.0));
        assert_eq!(a, 0.5);
        assert!((r - 0.215_861).abs() < 1e-5, "decoded, not raw: {r}");
    }

    #[test]
    fn each_slot_paints_its_own_colour() {
        let model = slotted();
        let painted = painted(&model, &LEAGUE);
        for (at, slot) in [
            (0, PaintSlot::Primary),
            (1, PaintSlot::Secondary),
            (2, PaintSlot::Trim),
        ] {
            let base = paint_color(LEAGUE.color(slot), 1.0);
            assert_eq!(painted.materials[at].base_color, base, "{slot:?}");
            let part = &painted.parts[at];
            assert!(
                colors(part).iter().all(|c| *c == vertex_color(base)),
                "{slot:?}: {:?}",
                colors(part)
            );
        }
    }

    #[test]
    fn a_painted_vertex_shows_the_livery_s_bytes() {
        let painted = painted(&slotted(), &LEAGUE);
        for (at, rgb) in [(0, LEAGUE.primary), (1, LEAGUE.secondary), (2, LEAGUE.trim)] {
            let [r, g, b, _] = painted.parts[at].mesh.vertices()[0].color;
            let bytes = [r, g, b].map(|c| (c * 255.0).round() as u8);
            assert_eq!(bytes, rgb, "part {at}");
        }
    }

    #[test]
    fn unpainted_parts_and_unmaterialled_parts_are_unchanged() {
        let model = slotted();
        let painted = painted(&model, &LEAGUE);
        // −y unpainted, +z with no material, −z out of range.
        for at in [3, 4, 5] {
            assert_eq!(painted.parts[at], model.parts[at], "part {at}");
        }
        assert_eq!(painted.materials[3], model.materials[3]);
    }

    #[test]
    fn painting_keeps_alpha_metallic_roughness_unlit_and_the_texture() {
        let own = Material {
            base_color: [0.1, 0.2, 0.3, 0.5],
            metallic: 0.25,
            roughness: 0.75,
            unlit: true,
            base_color_texture: Some(0),
            ..material("metal", Some(PaintSlot::Secondary))
        };
        let model = cube(vec![own.clone()], [Some(0); 6]);
        let painted = painted(&model, &LEAGUE);
        assert_eq!(
            painted.materials,
            [Material {
                base_color: paint_color(LEAGUE.secondary, 0.5),
                ..own
            }]
        );
        assert_eq!(painted.textures, model.textures);
        for (painted, own) in painted.parts.iter().zip(&model.parts) {
            assert_eq!(painted.material, own.material);
            assert_eq!(painted.uv, own.uv);
            assert_eq!(painted.mesh.indices(), own.mesh.indices());
            for (p, o) in painted.mesh.vertices().iter().zip(own.mesh.vertices()) {
                assert_eq!((p.position, p.normal), (o.position, o.normal));
                assert_eq!(p.color[3], 0.5, "alpha kept");
            }
        }
        // Merged, the painted model draws the same texture, by the same
        // pointer, so the two liveries share one upload; and stays unlit.
        let (merged, own) = (painted.mesh(), model.mesh());
        assert_eq!(merged.sections(), own.sections());
        assert_eq!(merged.sections()[0].texture, Some(0));
        assert!(Arc::ptr_eq(&merged.textures()[0], &model.textures[0].image));
        assert!(Arc::ptr_eq(&merged.textures()[0], &own.textures()[0]));
        assert!(merged.vertices().iter().all(|v| v.unlit));
        for (p, o) in merged.vertices().iter().zip(own.vertices()) {
            assert_eq!(p.uv, o.uv);
        }
    }

    #[test]
    fn a_model_with_nothing_painted_paints_to_itself() {
        let model = cube(
            vec![material("dark", None), material("glass", None)],
            [Some(0), Some(1), None, Some(0), Some(1), None],
        );
        assert_eq!(painted(&model, &LEAGUE), model);
    }

    #[test]
    fn a_model_with_a_part_in_a_slotted_material_takes_paint() {
        assert!(takes_paint(&slotted()));
        // One painted part among unpainted ones is enough.
        let one = cube(
            vec![
                material("dark", None),
                material("metal", Some(PaintSlot::Trim)),
            ],
            [Some(0), None, Some(9), Some(0), None, Some(1)],
        );
        assert!(takes_paint(&one));
        assert_ne!(painted(&one, &LEAGUE).mesh(), one.mesh());
    }

    #[test]
    fn a_model_with_no_part_in_a_slotted_material_takes_no_paint() {
        // Materials with no slot, no material, and an index out of range.
        let unslotted = cube(
            vec![material("dark", None), material("glass", None)],
            [Some(0), Some(1), None, Some(9), Some(1), None],
        );
        assert!(!takes_paint(&unslotted));
        assert_eq!(painted(&unslotted, &LEAGUE), unslotted);
        // A slotted material no part is drawn in paints no part.
        let undrawn = cube(
            vec![
                material("dark", None),
                material("metal", Some(PaintSlot::Primary)),
            ],
            [Some(0), None, Some(9), Some(0), None, Some(2)],
        );
        assert!(!takes_paint(&undrawn));
        assert_eq!(painted(&undrawn, &LEAGUE).parts, undrawn.parts);
        assert_eq!(painted(&undrawn, &LEAGUE).mesh(), undrawn.mesh());
        // No parts at all.
        assert!(!takes_paint(&LoadedModel {
            parts: Vec::new(),
            materials: vec![material("metal", Some(PaintSlot::Primary))],
            textures: Vec::new(),
        }));
    }

    #[test]
    fn two_liveries_paint_one_model_apart() {
        let model = slotted();
        let before = model.clone();
        let league = painted(&model, &LEAGUE);
        let raider = painted(&model, &RAIDER);
        assert_eq!(model, before, "painting borrows");
        assert_ne!(league.mesh(), raider.mesh());
        assert_ne!(league.mesh(), model.mesh());
        for at in 0..3 {
            assert_ne!(colors(&league.parts[at]), colors(&raider.parts[at]));
        }
        for at in 3..6 {
            assert_eq!(league.parts[at], raider.parts[at]);
        }
    }
}
