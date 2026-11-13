//! One hull drawn in two liveries in one offscreen frame: the hull's
//! painted face takes each livery's primary, its unpainted face keeps the
//! model's own colour in both. Natively only: painting happens before the
//! GPU, and `world_models` covers the upscalers. Needs a GPU adapter;
//! skips, passing, without one.
//!
//! The hull is a unit cube built in code, each face a part: +z (facing
//! the camera) in `hull`, painted primary; +y in `glass`, unpainted; the
//! other four in `dark`, unpainted. Both cubes take `world_models`' turn,
//! 30° about x, so +z and +y face the camera.

use std::f32::consts::FRAC_PI_6;

use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::model_source::{Material, PaintSlot, Part, vertex_color};
use nova_render::{
    Gpu, Light, LoadedModel, Mesh, RenderReport, Renderer, Upscaler, Vertex, Viewport, painted,
};
use nova_render::{Livery, livery::paint_color};
use nova_view::{Color, DrawList, ModelKey, ModelPose, Orientation, Point};

use super::common::pixels::{LOGICAL, SIZE, color_at, near, pixel_of, pixels_inside};
use super::wgpu_offscreen::{Images, gpu_with};

/// The hull in each livery, under its own key.
const KEYS: [ModelKey; 2] = [ModelKey(1), ModelKey(2)];

/// The two liveries: their primaries apart from each other and from the
/// hull's own colours.
const LIVERIES: [Livery; 2] = [
    Livery {
        primary: [0xc0, 0x30, 0x20],
        secondary: [0x40, 0x40, 0x40],
        trim: [0x10, 0x10, 0x10],
    },
    Livery {
        primary: [0x20, 0x70, 0xd0],
        secondary: [0xe0, 0xe0, 0xe0],
        trim: [0xf0, 0xf0, 0x00],
    },
];

/// Each cube's centre's logical x: 10 units a side, the first spans x 3
/// to 13 and the second 19 to 29.
const CENTRES: [f32; 2] = [8.0, 24.0];

/// The hull's materials, by index.
const HULL: usize = 0;
const GLASS: usize = 1;
const DARK: usize = 2;

/// `Mesh::cube`'s faces, in its order: +x, −x, +y, −y, +z, −z.
const NORMALS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];
const PLUS_Y: usize = 2;
const PLUS_Z: usize = 4;
/// Each face's material, in the same order.
const FACE_MATERIALS: [usize; 6] = [DARK, DARK, GLASS, DARK, HULL, DARK];

fn material(name: &str, paint: Option<PaintSlot>, base_color: [f32; 4]) -> Material {
    Material {
        name: Some(name.to_owned()),
        paint,
        ..Material::new(base_color)
    }
}

/// The hull as a source hands it over: a part per face, each coloured by
/// its material's base colour, baked.
fn hull() -> LoadedModel {
    let materials = vec![
        material("hull", Some(PaintSlot::Primary), [0.3, 0.6, 0.1, 1.0]),
        material("glass", None, [0.05, 0.2, 0.6, 1.0]),
        material("dark", None, [0.02, 0.02, 0.03, 1.0]),
    ];
    let cube = Mesh::cube([Color::WHITE; 6]);
    let parts = FACE_MATERIALS
        .iter()
        .enumerate()
        .map(|(face, &at)| {
            let color = vertex_color(materials[at].base_color);
            let vertices = cube.vertices()[face * 4..face * 4 + 4]
                .iter()
                .map(|v| Vertex { color, ..*v })
                .collect();
            let indices = cube.indices()[face * 6..face * 6 + 6]
                .iter()
                .map(|i| i - (face * 4) as u32)
                .collect();
            Part {
                mesh: Mesh::new(vertices, indices).expect("a face"),
                material: Some(at),
                uv: None,
            }
        })
        .collect();
    LoadedModel {
        parts,
        materials,
        textures: Vec::new(),
    }
}

fn turn() -> Orientation {
    Orientation::about([1.0, 0.0, 0.0], FRAC_PI_6)
}

/// `cube`'s pose: at (`CENTRES[cube]`, 12), 10 logical units a side.
fn pose(cube: usize) -> ModelPose {
    ModelPose {
        center: Point::new(CENTRES[cube], 12.0),
        orientation: turn(),
        scale: 10.0,
    }
}

/// The world with each cube `drawn` says, in its livery.
fn scene(drawn: [bool; 2]) -> DrawList {
    let mut list = DrawList::new();
    list.world(|list| {
        for (cube, key) in KEYS.iter().enumerate() {
            if drawn[cube] {
                list.model(*key, pose(cube));
            }
        }
    });
    list
}

/// `list` drawn with the hull registered in each livery, read back;
/// `None` with no adapter.
fn frame(list: &DrawList) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIZE, SIZE, &FontFaces::bundled())?;
    gpu.set_upscaler(Upscaler::Native);
    let mut renderer = Renderer::new(Images);
    let hull = hull();
    for (key, livery) in KEYS.iter().zip(&LIVERIES) {
        renderer.add_model(*key, painted(&hull, livery).mesh());
    }
    let viewport = Viewport::new(LOGICAL, (SIZE, SIZE), 2.0);
    let report = renderer.render(list, &viewport, &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// The linear base colour `base` on `face`, baked and lit as the core
/// shades it under the standard light, turned, as bytes.
fn shaded(face: usize, base: [f32; 4]) -> [u8; 3] {
    let normal = turn().rotate(NORMALS[face]).expect("a unit turn");
    let [r, g, b, _] = shade(normal, vertex_color(base), Light::STANDARD);
    [r, g, b].map(|c| (c * 255.0).round() as u8)
}

/// The +z face in `livery`'s primary.
fn painted_face(livery: &Livery) -> [u8; 3] {
    shaded(PLUS_Z, paint_color(livery.primary, 1.0))
}

/// `face` in its own material's colour.
fn own(face: usize) -> [u8; 3] {
    shaded(face, hull().materials[FACE_MATERIALS[face]].base_color)
}

/// Where cube `cube`'s +z face projects: turned 30° about x, its corners
/// (y ±0.5, z 0.5) land at y ±0.5 cos 30° − 0.5 sin 30°, −0.683 and
/// 0.183, so logical y 12 − 6.83 to 12 + 1.83; x its centre ± 5.
fn plus_z_area(cube: usize) -> (f32, f32, f32, f32) {
    (CENTRES[cube] - 5.0, 5.17, CENTRES[cube] + 5.0, 13.83)
}

/// Where its +y face projects: corners (y 0.5, z ±0.5) at 0.183 and
/// 0.683, so logical y 13.83 to 18.83.
fn plus_y_area(cube: usize) -> (f32, f32, f32, f32) {
    (CENTRES[cube] - 5.0, 13.83, CENTRES[cube] + 5.0, 18.83)
}

/// The logical points sampled on cube `cube`'s +z face, well inside it.
fn plus_z_samples(cube: usize) -> [(f32, f32); 2] {
    [(CENTRES[cube] - 2.5, 7.5), (CENTRES[cube] + 2.0, 11.0)]
}

/// The logical point sampled on its +y face.
fn plus_y_sample(cube: usize) -> (f32, f32) {
    (CENTRES[cube] + 0.5, 16.0)
}

/// The colour at the logical point `at`.
fn color_of(pixels: &[u8], (x, y): (f32, f32)) -> [u8; 3] {
    color_at(pixels, x, y)
}

#[test]
fn one_hull_draws_in_two_liveries_side_by_side() {
    let (Some(both), Some(without_first), Some(without_second)) = (
        frame(&scene([true, true])),
        frame(&scene([false, true])),
        frame(&scene([true, false])),
    ) else {
        return;
    };
    let without = [without_first, without_second];

    // Painting changed what the hull draws, and the liveries apart.
    for livery in &LIVERIES {
        assert!(!near(painted_face(livery), own(PLUS_Z)), "{livery:?}");
    }
    assert!(!near(
        painted_face(&LIVERIES[0]),
        painted_face(&LIVERIES[1])
    ));

    for cube in 0..2 {
        let (mine, other) = (&LIVERIES[cube], &LIVERIES[1 - cube]);
        let samples = plus_z_samples(cube).map(|at| (at, plus_z_area(cube)));
        let all = samples
            .iter()
            .copied()
            .chain([(plus_y_sample(cube), plus_y_area(cube))]);
        for (at, area) in all {
            // Each sample lies on its face, and the cube is what covers
            // it: the frame without this cube differs there.
            assert!(
                pixels_inside(area).contains(&pixel_of(at.0, at.1)),
                "{at:?} is inside {area:?}"
            );
            assert_ne!(
                color_of(&both, at),
                color_of(&without[cube], at),
                "cube {cube} covers {at:?}"
            );
        }
        // The painted face in this cube's livery, not the other's.
        for (at, _) in samples {
            let got = color_of(&both, at);
            assert!(
                near(got, painted_face(mine)),
                "cube {cube} +z at {at:?}: {got:?}, not {:?}",
                painted_face(mine)
            );
            assert!(
                !near(got, painted_face(other)),
                "cube {cube} +z at {at:?} is the other livery's"
            );
        }
        // The unpainted face keeps the model's own colour.
        let at = plus_y_sample(cube);
        let got = color_of(&both, at);
        assert!(
            near(got, own(PLUS_Y)),
            "cube {cube} +y at {at:?}: {got:?}, not {:?}",
            own(PLUS_Y)
        );
    }
}
