//! A model seen from above as painted, shaded triangles, or a ship's size
//! box when it has no model: an [`Outline`], plain data in ship metres
//! that a renderer turns and places.
//!
//! Ported from game-bible's `galaxy` tool's contact sheet (`sheet.rs`
//! `top_down`, `painted`, `draw_model`, `draw_box`, `normal`, `rgb` and
//! `hex`), without the SVG, the hardpoint markers and the size box's
//! outline drawn over a model. As there:
//!
//! - a face that faces sideways, whose `up` (the share of its normal
//!   along +Y) is below [`MIN_UP`], is dropped, and so is a face with no
//!   area;
//! - a face is shaded by `0.6 + 0.4 × up`, so a flat face keeps its
//!   colour;
//! - a material a [`Paint`] slot names takes that slot's livery colour,
//!   and every other its own base colour;
//! - faces are drawn lowest first, by each face's highest point;
//! - a ship with no model is its size box in its livery's primary, or
//!   [`BARE`] without a livery. The contact sheet fills the box at 35%;
//!   here it is opaque.

use std::collections::BTreeMap;

use nova_galaxy_format::asset::{Front, Paint};
use nova_galaxy_format::fleet::{Palette, Size};
use nova_galaxy_format::id::Color;

use crate::model::Model;
use crate::placement::Placement;

/// The colour of a ship that wears no livery, `#b8bcc4`, as the contact
/// sheet draws it.
pub const BARE: [u8; 3] = [0xb8, 0xbc, 0xc4];

/// How much of a face's normal must point up (or down) for it to be
/// drawn: a face below this faces sideways, and would be a sliver seen
/// from above.
pub const MIN_UP: f64 = 0.05;

/// A shape seen from above, as triangles in drawing order (the first
/// drawn first, under the rest).
///
/// Its corners are in ship space, `[x, y]` with x to starboard and y to
/// the nose, in metres as a ship's outline is made ([`top_down`]); a
/// caller may scale it ([`Outline::scaled`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outline {
    /// The triangles, lowest first.
    pub triangles: Vec<Triangle>,
}

/// One filled triangle of an [`Outline`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    /// Its corners, `[x, y]` in ship space.
    pub corners: [[f64; 2]; 3],
    /// Its opaque colour, red, green and blue.
    pub color: [u8; 3],
}

impl Outline {
    /// How far its farthest corner is from the centre (0 when it has no
    /// triangles): the radius it sweeps as it turns.
    #[must_use]
    pub fn reach(&self) -> f64 {
        self.triangles
            .iter()
            .flat_map(|t| t.corners)
            .fold(0.0, |reach, [x, y]| reach.max(x.hypot(y)))
    }

    /// The outline with every corner multiplied by `k`.
    #[must_use]
    pub fn scaled(&self, k: f64) -> Self {
        Self {
            triangles: self
                .triangles
                .iter()
                .map(|t| Triangle {
                    corners: t.corners.map(|[x, y]| [x * k, y * k]),
                    color: t.color,
                })
                .collect(),
        }
    }
}

/// A ship of `size` seen from above, as the contact sheet draws it: its
/// model (with the way it faces) placed at the ship's length, its painted
/// materials in `livery`'s colours ([`painted`]) when it has both a
/// livery and `paint`; without a model, its size box in `livery`'s
/// primary, or [`BARE`].
#[must_use]
pub fn top_down(
    model: Option<(&Model, Front)>,
    size: Size,
    paint: Option<&Paint>,
    livery: Option<&Palette>,
) -> Outline {
    match model {
        Some((model, front)) => {
            let placement = Placement::new(model, front, size.length);
            let colors = match (livery, paint) {
                (Some(livery), Some(paint)) => painted(model, paint, livery),
                _ => BTreeMap::new(),
            };
            silhouette(model, &placement, &colors)
        }
        None => size_box(size, livery.map_or(BARE, |l| rgb(&l.primary))),
    }
}

/// The colour `livery` gives each of `model`'s materials that `paint`
/// names, red, green and blue from 0 to 1, by material index.
#[must_use]
pub fn painted(model: &Model, paint: &Paint, livery: &Palette) -> BTreeMap<usize, [f64; 3]> {
    let slot = |name: &str| match name {
        "primary" => &livery.primary,
        "secondary" => &livery.secondary,
        _ => &livery.trim,
    };
    let mut colors = BTreeMap::new();
    for (name, names) in paint.slots() {
        for (index, material) in model.materials.iter().enumerate() {
            if material.as_ref().is_some_and(|m| names.contains(m)) {
                colors.insert(index, unit_rgb(slot(name)));
            }
        }
    }
    colors
}

/// `model` seen from above, as `placement` sets it on the ship: each face
/// that faces up or down enough ([`MIN_UP`]), shaded by how much it does,
/// in its material's colour in `colors` (by index) or else its own,
/// lowest first.
#[must_use]
pub fn silhouette(
    model: &Model,
    placement: &Placement,
    colors: &BTreeMap<usize, [f64; 3]>,
) -> Outline {
    let mut faces: Vec<(f32, Triangle)> = model
        .faces
        .iter()
        .filter_map(|f| {
            let n = normal(&f.points);
            let up = n[1].abs() / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if up.is_nan() || up < MIN_UP {
                return None;
            }
            let height = f.points.iter().map(|v| v[1]).fold(f32::MIN, f32::max);
            let light = 0.6 + 0.4 * up;
            let own = [0, 1, 2].map(|i| f64::from(f.color[i]));
            let color = f.material.and_then(|m| colors.get(&m)).unwrap_or(&own);
            Some((
                height,
                Triangle {
                    corners: f.points.map(|p| placement.ship(p)),
                    color: bytes(color.map(|c| c * light)),
                },
            ))
        })
        .collect();
    faces.sort_by(|a, b| a.0.total_cmp(&b.0));
    Outline {
        triangles: faces.into_iter().map(|(_, triangle)| triangle).collect(),
    }
}

/// A ship's size box, `size.beam` wide and `size.length` long and centred,
/// as two triangles in `color`.
#[must_use]
pub fn size_box(size: Size, color: [u8; 3]) -> Outline {
    let (x, y) = (size.beam / 2.0, size.length / 2.0);
    let corners = [[-x, -y], [x, -y], [x, y], [-x, y]];
    Outline {
        triangles: vec![
            Triangle {
                corners: [corners[0], corners[1], corners[2]],
                color,
            },
            Triangle {
                corners: [corners[0], corners[2], corners[3]],
                color,
            },
        ],
    }
}

/// `color` as red, green and blue bytes.
#[must_use]
pub fn rgb(color: &Color) -> [u8; 3] {
    bytes(unit_rgb(color))
}

/// A face's normal: the cross product of two of its sides.
fn normal(face: &[[f32; 3]; 3]) -> [f64; 3] {
    let [first, second, third] = face.map(|p| p.map(f64::from));
    let side = |to: [f64; 3]| [0, 1, 2].map(|i| to[i] - first[i]);
    let (u, v) = (side(second), side(third));
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

/// `color` (`#rrggbb`) as red, green and blue from 0 to 1.
fn unit_rgb(color: &Color) -> [f64; 3] {
    let hex = &color.as_str()[1..];
    [0, 2, 4].map(|i| f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0)) / 255.0)
}

/// Red, green and blue from 0 to 1 as bytes, rounded and held to 0-255,
/// as the contact sheet writes `#rrggbb`.
fn bytes(rgb: [f64; 3]) -> [u8; 3] {
    rgb.map(|v| (v * 255.0).round().clamp(0.0, 255.0) as u8)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_galaxy_format::frontmatter::parse;

    use super::*;
    use crate::fixture;
    use crate::model::{Bounds, Face, model};

    fn color(text: &str) -> Color {
        Color::try_from(text.to_owned()).unwrap()
    }

    /// `league-paint`, the format fixture's livery.
    fn livery() -> Palette {
        Palette {
            primary: color("#ddccaa"),
            secondary: color("#aa8800"),
            trim: color("#222222"),
        }
    }

    fn paint(yaml: &str) -> Paint {
        parse::<Paint>(&format!("---\n{yaml}\n---\n")).unwrap().0
    }

    fn size(length: f64, beam: f64) -> Size {
        Size {
            length,
            beam,
            collision_radius: length / 2.0,
        }
    }

    /// A model of `faces`, each white, with no materials.
    fn faces(faces: &[[[f32; 3]; 3]]) -> Model {
        Model {
            triangles: u32::try_from(faces.len()).unwrap(),
            bounds: Bounds {
                min: [-1.0; 3],
                max: [1.0; 3],
            },
            faces: faces
                .iter()
                .map(|&points| Face {
                    points,
                    color: [1.0; 4],
                    material: None,
                })
                .collect(),
            materials: Vec::new(),
            external: Vec::new(),
        }
    }

    /// The `[2, 1, 4]` box (x, y, z), placed nose +z at `length` metres.
    fn box_outline(glb: &[u8], length: f64, colors: &BTreeMap<usize, [f64; 3]>) -> Outline {
        let model = model("m.glb", glb, |_| None).unwrap();
        silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, length),
            colors,
        )
    }

    fn corners(outline: &Outline) -> Vec<[f64; 2]> {
        outline.triangles.iter().flat_map(|t| t.corners).collect()
    }

    #[test]
    fn a_placed_box_is_in_ship_metres_its_length_the_ship_s() {
        let outline = box_outline(
            &fixture::glb([2.0, 1.0, 4.0], [0.0; 3]),
            8.0,
            &BTreeMap::new(),
        );
        let corners = corners(&outline);
        let span = |axis: usize| {
            let values = corners.iter().map(|c| c[axis]);
            (
                values.clone().fold(f64::MAX, f64::min),
                values.fold(f64::MIN, f64::max),
            )
        };
        assert_eq!(span(1), (-4.0, 4.0), "8 m nose to tail");
        assert_eq!(span(0), (-2.0, 2.0), "2 units wide at 2 m a unit");
    }

    #[test]
    fn faces_that_face_sideways_are_dropped() {
        // A box seen from above keeps its top and its bottom, two
        // triangles each, and drops its four sides.
        let outline = box_outline(
            &fixture::glb([2.0, 1.0, 4.0], [0.0; 3]),
            8.0,
            &BTreeMap::new(),
        );
        assert_eq!(outline.triangles.len(), 4);
    }

    #[test]
    fn a_face_with_no_area_is_dropped() {
        let model = faces(&[[[0.0, 0.0, 0.0]; 3]]);
        let outline = silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, 2.0),
            &BTreeMap::new(),
        );
        assert!(outline.triangles.is_empty());
    }

    #[test]
    fn a_face_is_shaded_by_how_much_it_faces_up() {
        // Flat: fully lit. At 45°: up = 1/√2, lit 0.6 + 0.4 × 0.707.
        // Nearly upright (up just under 0.05): dropped.
        let model = faces(&[
            [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
            [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 2.0, 1.0]],
            [[0.0, 3.0, 0.0], [1.0, 3.0, 0.0], [0.0, 23.0, 0.999]],
        ]);
        let outline = silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, 2.0),
            &BTreeMap::new(),
        );
        let colors: Vec<_> = outline.triangles.iter().map(|t| t.color).collect();
        let tilted = (255.0 * (0.6 + 0.4 * std::f64::consts::FRAC_1_SQRT_2)).round() as u8;
        assert_eq!(colors, [[255; 3], [tilted; 3]]);
    }

    #[test]
    fn a_face_just_up_enough_is_kept() {
        // Its normal's up is 0.0501: kept, and lit 0.62.
        let model = faces(&[[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 19.94, 1.0]]]);
        let outline = silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, 2.0),
            &BTreeMap::new(),
        );
        assert_eq!(outline.triangles.len(), 1);
        assert_eq!(outline.triangles[0].color, [158; 3]);
    }

    #[test]
    fn painted_materials_take_the_livery_s_colour_and_the_rest_their_own() {
        let glb = fixture::glb_with_materials(
            [2.0, 1.0, 4.0],
            [0.0; 3],
            &[("hull", [1.0; 4]), ("glass", [0.2, 0.4, 0.8, 1.0])],
        );
        let model = model("m.glb", &glb, |_| None).unwrap();
        let colors = painted(&model, &paint("primary: [hull]"), &livery());
        let outline = silhouette(&model, &Placement::new(&model, Front::PlusZ, 8.0), &colors);
        let mut seen: Vec<_> = outline.triangles.iter().map(|t| t.color).collect();
        seen.dedup();
        // The box's `hull` half holds its bottom, `glass` its top: the
        // bottom is lower, so drawn first.
        assert_eq!(seen, [[0xdd, 0xcc, 0xaa], [0x33, 0x66, 0xcc]]);
    }

    #[test]
    fn each_slot_paints_its_own_colour() {
        let glb = fixture::glb_with_materials(
            [2.0, 1.0, 4.0],
            [0.0; 3],
            &[("hull", [1.0; 4]), ("glass", [1.0; 4])],
        );
        let model = model("m.glb", &glb, |_| None).unwrap();
        let colors = painted(
            &model,
            &paint("{secondary: [hull], trim: [glass]}"),
            &livery(),
        );
        assert_eq!(
            colors,
            BTreeMap::from([
                (0, [170.0 / 255.0, 136.0 / 255.0, 0.0]),
                (1, [34.0 / 255.0; 3]),
            ])
        );
        let colors = painted(&model, &paint("{primary: [glass]}"), &livery());
        assert_eq!(colors.keys().collect::<Vec<_>>(), [&1]);
    }

    #[test]
    fn faces_are_drawn_lowest_first_by_their_highest_point() {
        let model = faces(&[
            [[0.0, 2.0, 0.0], [0.0, 2.0, 1.0], [1.0, 2.0, 0.0]],
            [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.5, 0.0]],
            [[5.0, 1.0, 0.0], [5.0, 1.0, 1.0], [6.0, 1.0, 0.0]],
        ]);
        let outline = silhouette(
            &model,
            &Placement::unscaled(&model, Front::PlusZ),
            &BTreeMap::new(),
        );
        let heights: Vec<_> = outline.triangles.iter().map(|t| t.corners[0][0]).collect();
        // Placed +z forward, x is to port: model x 0 and 5 land at ship
        // x 0 and -5. The face up to 1.5 sorts between 1 and 2.
        assert_eq!(heights, [-5.0, 0.0, 0.0]);
        assert_eq!(outline.triangles[2].corners[0][1], 0.0);
        assert_eq!(outline.triangles[1].corners[2], [-1.0, 0.0]);
    }

    #[test]
    fn colours_round_and_hold_to_a_byte() {
        let mut model = faces(&[[[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]]]);
        model.faces[0].color = [1.2, 0.5, -0.1, 1.0];
        let outline = silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, 2.0),
            &BTreeMap::new(),
        );
        assert_eq!(outline.triangles[0].color, [255, 128, 0]);
    }

    #[test]
    fn a_colour_reads_as_bytes() {
        assert_eq!(rgb(&color("#c08040")), [0xc0, 0x80, 0x40]);
        assert_eq!(rgb(&color("#FF0001")), [0xff, 0x00, 0x01]);
    }

    #[test]
    fn a_size_box_is_two_triangles_covering_beam_by_length() {
        let outline = size_box(size(30.0, 12.0), [1, 2, 3]);
        assert_eq!(
            outline.triangles,
            [
                Triangle {
                    corners: [[-6.0, -15.0], [6.0, -15.0], [6.0, 15.0]],
                    color: [1, 2, 3],
                },
                Triangle {
                    corners: [[-6.0, -15.0], [6.0, 15.0], [-6.0, 15.0]],
                    color: [1, 2, 3],
                },
            ]
        );
    }

    #[test]
    fn reach_is_the_farthest_corner_from_the_centre() {
        let outline = Outline {
            triangles: vec![
                Triangle {
                    corners: [[0.0, 0.0], [3.0, -4.0], [1.0, 1.0]],
                    color: [0; 3],
                },
                Triangle {
                    corners: [[-2.0, 2.0], [0.0, 1.0], [1.0, 0.0]],
                    color: [0; 3],
                },
            ],
        };
        assert_eq!(outline.reach(), 5.0);
        assert_eq!(Outline::default().reach(), 0.0);
    }

    #[test]
    fn scaled_multiplies_every_corner() {
        let outline = size_box(size(4.0, 2.0), [9; 3]).scaled(2.5);
        assert_eq!(
            outline.triangles[1],
            Triangle {
                corners: [[-2.5, -5.0], [2.5, 5.0], [-2.5, 5.0]],
                color: [9; 3],
            }
        );
    }

    #[test]
    fn a_ship_with_a_model_is_its_painted_silhouette_at_its_length() {
        let glb = fixture::glb([2.0, 1.0, 4.0], [0.0; 3]);
        let model = model("m.glb", &glb, |_| None).unwrap();
        let hull = paint("primary: [hull]");
        let outline = top_down(
            Some((&model, Front::MinusZ)),
            size(8.0, 4.0),
            Some(&hull),
            Some(&livery()),
        );
        assert_eq!(outline.triangles.len(), 4);
        assert_eq!(outline.reach(), 2.0_f64.hypot(4.0));
        assert!(
            outline
                .triangles
                .iter()
                .all(|t| t.color == [0xdd, 0xcc, 0xaa])
        );
        // Without a livery, or without paint, it keeps its own colours.
        let league = livery();
        for (paint, livery) in [(Some(&hull), None), (None, Some(&league))] {
            let own = top_down(Some((&model, Front::MinusZ)), size(8.0, 4.0), paint, livery);
            assert!(own.triangles.iter().all(|t| t.color == [255; 3]), "{own:?}");
        }
    }

    #[test]
    fn a_ship_without_a_model_is_its_size_box_in_the_livery_s_primary_or_bare() {
        let painted = top_down(None, size(30.0, 12.0), None, Some(&livery()));
        assert_eq!(painted, size_box(size(30.0, 12.0), [0xdd, 0xcc, 0xaa]));
        let bare = top_down(None, size(30.0, 12.0), None, None);
        assert_eq!(bare, size_box(size(30.0, 12.0), BARE));
        assert_eq!(BARE, [0xb8, 0xbc, 0xc4]);
    }

    #[test]
    fn a_face_s_up_is_its_normal_s_share_along_y() {
        // Sides (1, 2, 3) and (2, 1, 5): normal (7, 1, -3), up 1/√59,
        // lit 0.6 + 0.4 × 0.130.
        let model = faces(&[[[0.0, 0.0, 0.0], [1.0, 2.0, 3.0], [2.0, 1.0, 5.0]]]);
        let outline = silhouette(
            &model,
            &Placement::new(&model, Front::PlusZ, 2.0),
            &BTreeMap::new(),
        );
        let lit = (255.0 * (0.6 + 0.4 / 59.0_f64.sqrt())).round() as u8;
        assert_eq!(outline.triangles[0].color, [lit; 3]);
    }
}
