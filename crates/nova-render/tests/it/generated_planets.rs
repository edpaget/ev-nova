//! The sample's planets drawn offscreen through the draw list: Kestrel
//! Reach, its one generated stellar, and Marrow Fields and Lantern
//! Cloister on their textures. The sample is mirrored in code, its
//! stellars and looks copied by hand from game-bible's `data/stellars/`,
//! its textures made by the fixture's PNG encoder, so nothing here reads
//! game-bible; each planet is loaded through [`GalaxyPlanets`], registered
//! with the renderer and drawn by a [`DrawCommand::Planet`]. Every pixel
//! checked is placed, and its colour worked out, by `nova_render::planet`'s
//! functions and the pose, never retyped.
//!
//! `the_bible_s_sample_planets_draw` loads the real sample through
//! [`DirSource`] when `GAME_DATA` names it, and skips, passing, when it is
//! unset. Needs a GPU adapter; skips, passing, without one. Native only:
//! `world_models` covers the upscalers.

use nova_galaxy_format::fixture::{CC0, galaxy};
use nova_galaxy_format::load::load;
use nova_galaxy_format::source::{DirSource, MemSource, Source};
use nova_galaxy_media::fixture::png;
use nova_galaxy_media::planet::{Image, Sizes, Surface};
use nova_render::camera::shade;
use nova_render::fonts::FontFaces;
use nova_render::planet::{
    Fragment, Frame, PLANET_LIGHT, VIEW_LEAN, fragment, globe, lit, surface_uv, turns,
};
use nova_render::{
    GalaxyPlanets, Light, LoadedPlanet, LogicalSize, Mesh, PlanetError, PlanetRequest,
    PlanetSource, RenderReport, Renderer, Viewport,
};
use nova_view::{
    Color, DrawCommand, DrawList, ModelKey, ModelPose, Orientation, PlanetKey, PlanetPose, Point,
};

use super::common::game_data;
use super::wgpu_offscreen::{Images, gpu_with};

/// The target's side: logical units and pixels alike.
const SIDE: u32 = 512;
const LOGICAL: LogicalSize = LogicalSize { w: SIDE, h: SIDE };

/// What the adapter bakes at here: small, so the test stays quick.
const SMALL: Sizes = Sizes {
    surface: 64,
    clouds: 32,
    ring: 64,
};

/// The globe's radius in pixels: big enough that Kestrel Reach's 0.04
/// radii of atmosphere is a few pixels wide.
const RADIUS: f32 = 96.0;
/// Where each planet is drawn, but where a test moves it.
const CENTRE: (f32, f32) = (256.0, 256.0);
/// When.
const TIME: f64 = 40.0;

const PLANET: PlanetKey = PlanetKey(1);
const CUBE: ModelKey = ModelKey(1);

/// Kestrel Reach, as the sample writes it, but in the fixture's `alpha`.
fn kestrel_reach(seed: u32) -> String {
    format!(
        "---\nname: Kestrel Reach\nkind: planet\nsystem: alpha\nposition: [-400, 300]\n\
         visual:\n  generated:\n    type: gas-giant\n\
         \x20   palette: [\"#c9a36b\", \"#8a5a3c\", \"#e8d8b0\"]\n    clouds: 0.6\n\
         \x20   rings: {{inner: 1.4, outer: 2.3, color: \"#d8c8a8\"}}\n\
         \x20   atmosphere: {{color: \"#f0e0c0\", thickness: 0.04}}\n    seed: {seed}\n---\n"
    )
}

/// A 64 by 32 texture whose red rises with the column, green with the
/// row, blue `blue`: smooth, so a filtered sample is near its texel.
fn pattern(blue: u8) -> Vec<u8> {
    (0..32_u8)
        .flat_map(|y| (0..64_u8).flat_map(move |x| [x * 4, y * 8, blue, 255]))
        .collect()
}

/// The sample's three planets in the format's test galaxy (whose default
/// budget gives `texture: {kib: 4096, side: 4096}`), Kestrel Reach's seed
/// `seed`.
fn sample(seed: u32) -> MemSource {
    let texture = |file: &str| {
        format!(
            "---\nkind: texture\nplaceholder: false\nfiles: [media/export/planets/{file}]\n{CC0}---\n"
        )
    };
    galaxy()
        .with("stellars/kestrel-reach.md", &kestrel_reach(seed))
        .with(
            "stellars/marrow-fields.md",
            "---\nname: Marrow Fields\nkind: planet\nsystem: alpha\nposition: [150, 120]\n\
             visual: {asset: marrow-fields-surface}\n---\n",
        )
        .with(
            "stellars/lantern-cloister.md",
            "---\nname: Lantern Cloister\nkind: moon\nsystem: alpha\nposition: [100, 300]\n\
             visual: {asset: lantern-cloister-surface}\n---\n",
        )
        .with("assets/marrow-fields-surface.md", &texture("marrow.png"))
        .with(
            "assets/lantern-cloister-surface.md",
            &texture("lantern.png"),
        )
        .with_bytes(
            "media/export/planets/marrow.png",
            &png(64, 32, &pattern(100)),
        )
        .with_bytes(
            "media/export/planets/lantern.png",
            &png(64, 32, &pattern(200)),
        )
}

/// `stellar`'s planet in `source`, through the adapter.
fn loaded(source: &MemSource, stellar: &str) -> LoadedPlanet {
    let (galaxy, problems) = load(source);
    assert_eq!(problems, [], "the mirrored sample loads cleanly");
    GalaxyPlanets::new(&galaxy, source)
        .baked_at(SMALL)
        .planet(&PlanetRequest::new(stellar))
        .unwrap_or_else(|e| panic!("{stellar} loads: {e}"))
}

fn pose(x: f32, y: f32) -> PlanetPose {
    PlanetPose {
        center: Point::new(x, y),
        radius: RADIUS,
        time: TIME,
    }
}

/// `list` drawn with `planet` registered as [`PLANET`] (and a cube as
/// [`CUBE`]), read back; `None` with no adapter.
fn drawn(planet: &LoadedPlanet, list: &DrawList) -> Option<Vec<u8>> {
    let mut gpu = gpu_with(SIDE, SIDE, &FontFaces::bundled())?;
    let mut renderer = Renderer::new(Images);
    renderer.add_planet(PLANET, planet.clone());
    renderer.add_model(
        CUBE,
        Mesh::cube(CUBE_FACES.map(|[r, g, b]| Color::rgba(r, g, b, 255))),
    );
    let report = renderer.render(list, &Viewport::new(LOGICAL, (SIDE, SIDE), 1.0), &mut gpu);
    assert_eq!(report, RenderReport::default());
    Some(gpu.read_pixels().expect("read back"))
}

/// `planet` alone at `pose`, read back.
fn alone(planet: &LoadedPlanet, pose: PlanetPose) -> Option<Vec<u8>> {
    let mut list = DrawList::new();
    list.push(DrawCommand::Planet {
        planet: PLANET,
        pose,
    });
    drawn(planet, &list)
}

/// The pixel whose centre is nearest `local` (in planet radii from a
/// planet at `centre`).
fn pixel_at(centre: (f32, f32), local: (f32, f32)) -> (u32, u32) {
    let x = centre.0 + local.0 * RADIUS;
    let y = centre.1 + local.1 * RADIUS;
    (x.floor() as u32, y.floor() as u32)
}

/// Where the centre of pixel `(px, py)` is, in planet radii from a
/// planet at `centre`.
fn local_of(centre: (f32, f32), (px, py): (u32, u32)) -> (f32, f32) {
    (
        (px as f32 + 0.5 - centre.0) / RADIUS,
        (py as f32 + 0.5 - centre.1) / RADIUS,
    )
}

fn rgb(pixels: &[u8], (x, y): (u32, u32)) -> [u8; 3] {
    let at = ((y * SIDE + x) * 4) as usize;
    [pixels[at], pixels[at + 1], pixels[at + 2]]
}

/// Whether each channel of `a` is within `by` of `b`'s.
fn near(a: [u8; 3], b: [u8; 3], by: u8) -> bool {
    a.iter().zip(&b).all(|(p, q)| p.abs_diff(*q) <= by)
}

/// `image` sampled bilinearly at `(u, v)`, wrapping in u and clamping in
/// v, as the planet sampler reads its largest level: RGBA, 0 to 1.
fn sample_at(image: &Image, [u, v]: [f32; 2]) -> [f32; 4] {
    let (width, height) = (image.width as f32, image.height as f32);
    let (x, y) = (u * width - 0.5, v * height - 0.5);
    let (left, top) = (x.floor(), y.floor());
    let (across, down) = (x - left, y - top);
    let column = |c: f32| (c as i64).rem_euclid(i64::from(image.width)) as u32;
    let row = |r: f32| r.clamp(0.0, height - 1.0) as u32;
    let texel = |c: f32, r: f32| image.texel(column(c), row(r)).map(|b| f32::from(b) / 255.0);
    let (nw, ne) = (texel(left, top), texel(left + 1.0, top));
    let (sw, se) = (texel(left, top + 1.0), texel(left + 1.0, top + 1.0));
    [0, 1, 2, 3].map(|i| {
        let upper = nw[i] + (ne[i] - nw[i]) * across;
        let lower = sw[i] + (se[i] - sw[i]) * across;
        upper + (lower - upper) * down
    })
}

fn bytes(c: [f32; 3]) -> [u8; 3] {
    c.map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The colour the core ([`fragment`]) says the pixel centred at `local`
/// shows over the clear colour, on `surface` at [`TIME`]: its surface and
/// cloud texels sampled as the planet sampler reads their largest levels,
/// its rings' profile read by `RingBand::opacity`, and the globe covering
/// it wholly on the disc and not at all off it.
fn core_color(surface: &Surface, local: (f32, f32)) -> [u8; 3] {
    let frame = Frame::new(surface.tilt as f32, VIEW_LEAN);
    let (turn, cloud_turn) = turns(TIME, surface.day);
    let normal = globe(local.0, local.1);
    let texel = |image: &Image, turn| {
        normal.map_or([0.0; 4], |n| sample_at(image, surface_uv(n, &frame, turn)))
    };
    let unit = |c: [u8; 3]| c.map(|c| f32::from(c) / 255.0);
    let atmosphere = surface.atmosphere.as_ref().map_or([0.0; 4], |a| {
        let [r, g, b] = unit(a.color);
        [r, g, b, a.thickness as f32]
    });
    let inputs = Fragment {
        x: local.0,
        y: local.1,
        frame,
        light: PLANET_LIGHT,
        ground: texel(&surface.surface, turn),
        cloud: surface
            .clouds
            .as_ref()
            .map_or([0.0; 4], |clouds| texel(clouds, cloud_turn)),
        ring_color: surface.rings.as_ref().map_or([0.0; 3], |r| unit(r.color)),
        atmosphere,
        coverage: if normal.is_some() { 1.0 } else { 0.0 },
    };
    let opacity = |r: f32| {
        surface
            .rings
            .as_ref()
            .map_or(0.0, |rings| rings.opacity(f64::from(r)) as f32)
    };
    fragment(&inputs, opacity).map_or([0; 3], |[r, g, b, a]| bytes([r * a, g * a, b * a]))
}

#[test]
fn kestrel_reach_draws_lit_with_its_halo_and_rings() {
    let planet = loaded(&sample(4417), "kestrel-reach");
    let Some(pixels) = alone(&planet, pose(CENTRE.0, CENTRE.1)) else {
        return;
    };
    let surface = &planet.surface;
    let frame = Frame::new(surface.tilt as f32, VIEW_LEAN);

    // The lit centre region, north of the rings' shadow: the surface
    // there times the planet light.
    for at in [(-0.2, -0.3), (-0.1, -0.2), (0.0, -0.35)] {
        let pixel = pixel_at(CENTRE, at);
        let local = local_of(CENTRE, pixel);
        let expected = core_color(surface, local);
        let got = rgb(&pixels, pixel);
        assert!(
            near(got, expected, 8),
            "lit at {local:?}: {got:?}, not {expected:?}"
        );
    }

    // The night limb, toward the lower right, is darker than the lit one.
    let brightness = |c: [u8; 3]| c.iter().map(|c| u32::from(*c)).sum::<u32>();
    let day = rgb(&pixels, pixel_at(CENTRE, (-0.9, 0.0)));
    let night = rgb(&pixels, pixel_at(CENTRE, (0.9, 0.0)));
    assert!(
        brightness(night) * 2 < brightness(day),
        "night {night:?}, day {day:?}"
    );

    // Just outside the limb, on the left where no ring is, the halo
    // carries the atmosphere's hue; beyond the reach is the clear colour.
    let glow = rgb(&pixels, pixel_at(CENTRE, (-1.0 - 1.5 / RADIUS, 0.0)));
    assert!(
        glow[0] >= glow[1] && glow[1] > glow[2] && glow[2] > 0,
        "the halo is {glow:?}"
    );
    assert_eq!(rgb(&pixels, pixel_at(CENTRE, (-2.35, 0.0))), [0, 0, 0]);
    assert_eq!(rgb(&pixels, (0, 0)), [0, 0, 0]);

    // On the rings' near ellipse, off the globe and lit: their colour at
    // their opacity there, over the clear colour.
    let rings = surface.rings.as_ref().expect("Kestrel Reach is ringed");
    let densest = (0..=80)
        .map(|i| 1.45 + 0.01 * f64::from(i))
        .max_by(|a, b| rings.opacity(*a).total_cmp(&rings.opacity(*b)))
        .expect("radii");
    let on_ring = |theta: f32| {
        let (sin, cos) = theta.sin_cos();
        let r = densest as f32;
        (
            r * (cos * frame.east[0] + sin * frame.facing[0]),
            r * (cos * frame.east[1] + sin * frame.facing[1]),
        )
    };
    let near_side = on_ring(0.5);
    let pixel = pixel_at(CENTRE, (near_side.0, near_side.1));
    let local = local_of(CENTRE, pixel);
    assert!(globe(local.0, local.1).is_none(), "off the globe");
    let hit = nova_render::planet::ring_hit(local.0, local.1, &frame).expect("a ring point");
    assert!(hit.in_front);
    assert!(!nova_render::planet::in_planet_shadow(
        hit.point,
        PLANET_LIGHT
    ));
    assert!(rings.opacity(f64::from(hit.radius)) > 0.0);
    let color = core_color(surface, local);
    let got = rgb(&pixels, pixel);
    assert!(near(got, color, 8), "ring {got:?}, not {color:?}");

    // The same ring radius behind the globe is hidden by it: the globe
    // shows there as it does with no rings at all.
    let far_side = on_ring(-std::f32::consts::FRAC_PI_2);
    let behind = pixel_at(CENTRE, far_side);
    let local = local_of(CENTRE, behind);
    assert!(globe(local.0, local.1).is_some(), "on the globe");
    let ringless = LoadedPlanet {
        surface: Surface {
            rings: None,
            ..surface.clone()
        },
    };
    let Some(bare) = alone(&ringless, pose(CENTRE.0, CENTRE.1)) else {
        return;
    };
    assert!(
        near(rgb(&pixels, behind), rgb(&bare, behind), 1),
        "behind the globe: {:?} with rings, {:?} without",
        rgb(&pixels, behind),
        rgb(&bare, behind)
    );
    assert!(
        !near(rgb(&pixels, pixel), rgb(&bare, pixel), 8),
        "the near side shows them"
    );
}

#[test]
fn the_same_seed_always_draws_the_same_planet() {
    // Two independent loads, renderers and GPUs.
    let first = loaded(&sample(4417), "kestrel-reach");
    let second = loaded(&sample(4417), "kestrel-reach");
    let Some(once) = alone(&first, pose(CENTRE.0, CENTRE.1)) else {
        return;
    };
    let Some(again) = alone(&second, pose(CENTRE.0, CENTRE.1)) else {
        return;
    };
    assert!(once == again, "byte for byte");

    // Moved by whole pixels, the same pixels moved: nearly all exactly,
    // and the rest close. The GPU interpolates the square's corners from
    // where they now are, so a fragment's place differs in its last bits,
    // which can tip a mip level or an edge's coverage where the surface
    // changes fastest (by the poles, at the limb); the planet is the same.
    let (dx, dy) = (3_i64, -2_i64);
    let Some(moved) = alone(&first, pose(CENTRE.0 + dx as f32, CENTRE.1 + dy as f32)) else {
        return;
    };
    let reach = (2.3 * RADIUS).ceil() as i64 + 2;
    let (cx, cy) = (CENTRE.0 as i64, CENTRE.1 as i64);
    let (mut exact, mut all) = (0_u32, 0_u32);
    for y in cy - reach..cy + reach {
        for x in cx - reach..cx + reach {
            let here = (x as u32, y as u32);
            let there = ((x + dx) as u32, (y + dy) as u32);
            let (a, b) = (rgb(&once, here), rgb(&moved, there));
            assert!(near(a, b, 8), "at {here:?}: {a:?}, moved {b:?}");
            all += 1;
            exact += u32::from(a == b);
        }
    }
    assert!(exact * 1000 >= all * 995, "{exact} of {all} exactly");

    // Another seed is another planet.
    let other = loaded(&sample(4418), "kestrel-reach");
    let Some(different) = alone(&other, pose(CENTRE.0, CENTRE.1)) else {
        return;
    };
    assert!(once != different, "seed 4418 draws another planet");
}

#[test]
fn the_textured_planets_draw_their_textures_with_no_halo_or_rings() {
    for (stellar, blue) in [("marrow-fields", 100_u8), ("lantern-cloister", 200)] {
        let planet = loaded(&sample(4417), stellar);
        let Some(pixels) = alone(&planet, pose(CENTRE.0, CENTRE.1)) else {
            return;
        };
        for at in [(-0.2, -0.2), (0.1, -0.3), (-0.3, 0.1)] {
            let pixel = pixel_at(CENTRE, at);
            let local = local_of(CENTRE, pixel);
            let expected = core_color(&planet.surface, local);
            let got = rgb(&pixels, pixel);
            assert!(
                near(got, expected, 6),
                "{stellar} at {local:?}: {got:?}, not {expected:?}"
            );
            // Lit, the texture's blue shows, scaled by the light.
            let shade = lit(globe(local.0, local.1).unwrap(), PLANET_LIGHT);
            let blue = (f32::from(blue) * shade).round() as u8;
            assert!(
                got[2].abs_diff(blue) <= 3,
                "{stellar}: {got:?} has blue {blue}"
            );
        }
        // No halo just outside the limb, and no ring where Kestrel Reach's
        // would be.
        assert_eq!(
            rgb(&pixels, pixel_at(CENTRE, (-1.0 - 1.5 / RADIUS, 0.0))),
            [0, 0, 0]
        );
        assert_eq!(rgb(&pixels, pixel_at(CENTRE, (1.8, 0.1))), [0, 0, 0]);
    }
}

/// A 64 by 32 surface that glows wholly (so it shows unlit, as it is):
/// white in its northern twelve rows, black south of them, so its 1 by 1
/// mip is a grey that no texel of its polar cap has. No tilt, clouds,
/// rings or atmosphere.
fn capped() -> LoadedPlanet {
    let rgba = (0..32_u32)
        .flat_map(|y| {
            let c = if y < 12 { 255 } else { 0 };
            (0..64).flat_map(move |_| [c, c, c, 255])
        })
        .collect();
    LoadedPlanet {
        surface: Surface {
            surface: Image {
                width: 64,
                height: 32,
                rgba,
            },
            clouds: None,
            rings: None,
            atmosphere: None,
            day: 300.0,
            tilt: 0.0,
        },
    }
}

#[test]
fn no_mip_seam_crosses_the_polar_cap() {
    // Untilted, the meridian opposite the one facing the camera runs up
    // the screen from the north pole to the top limb, at x = 0. Centred
    // on a pixel edge one pixel left of `CENTRE`, that meridian splits the
    // 2 by 2 quad of columns 254 and 255, where a longitude's gradient
    // taken across its wrap would pick the coarsest mip.
    let centre = (CENTRE.0 - 1.0, CENTRE.1);
    let planet = capped();
    let Some(pixels) = alone(&planet, pose(centre.0, centre.1)) else {
        return;
    };
    let frame = Frame::new(0.0, VIEW_LEAN);
    for py in 161..=164 {
        for px in 252..=257 {
            let local = local_of(centre, (px, py));
            let n = globe(local.0, local.1).expect("on the disc");
            assert!(
                nova_render::planet::latitude(n, &frame) > 1.3,
                "inside the cap at {local:?}"
            );
            let got = rgb(&pixels, (px, py));
            assert!(
                near(got, [255; 3], 16),
                "the cap at ({px}, {py}) is {got:?}, not white"
            );
        }
    }
    // Nor down the meridian facing the camera, on the black south of the
    // cap.
    for py in 270..=280 {
        for px in 252..=257 {
            let got = rgb(&pixels, (px, py));
            assert!(near(got, [0; 3], 16), "at ({px}, {py}): {got:?}");
        }
    }
}

/// The cube's faces' colours, in `Mesh::cube`'s order: +x, −x, +y, −y,
/// +z, −z.
const CUBE_FACES: [[u8; 3]; 6] = [
    [0, 255, 255],
    [255, 0, 255],
    [0, 200, 0],
    [255, 255, 0],
    [230, 40, 40],
    [40, 40, 255],
];

#[test]
fn a_model_drawn_after_a_planet_covers_it() {
    let planet = loaded(&sample(4417), "marrow-fields");
    let mut list = DrawList::new();
    list.planet(PLANET, pose(CENTRE.0, CENTRE.1)).model(
        CUBE,
        ModelPose {
            center: Point::new(CENTRE.0, CENTRE.1),
            orientation: Orientation::IDENTITY,
            scale: 40.0,
        },
    );
    let Some(pixels) = drawn(&planet, &list) else {
        return;
    };
    // Facing the camera, its +z face.
    let [r, g, b] = CUBE_FACES[4].map(|c| f32::from(c) / 255.0);
    let [r, g, b, _] = shade([0.0, 0.0, 1.0], [r, g, b, 1.0], Light::STANDARD);
    let face = bytes([r, g, b]);
    let centre = (CENTRE.0 as u32, CENTRE.1 as u32);
    assert!(
        near(rgb(&pixels, centre), face, 2),
        "{:?}",
        rgb(&pixels, centre)
    );
    // Beside it, the planet still shows.
    let beside = pixel_at(CENTRE, (-0.5, -0.5));
    assert!(near(
        rgb(&pixels, beside),
        core_color(&planet.surface, local_of(CENTRE, beside)),
        6
    ));
}

#[test]
fn the_bible_s_sample_planets_draw() {
    let Some(dir) = game_data() else { return };
    let source = DirSource::new(&dir);
    let (galaxy, _) = load(&source);
    let planets = GalaxyPlanets::new(&galaxy, &source);
    for stellar in ["kestrel-reach", "marrow-fields", "lantern-cloister"] {
        match planets.planet(&PlanetRequest::new(stellar)) {
            Err(PlanetError::LfsPointer { asset, .. }) => {
                eprintln!("`{asset}` is a Git LFS pointer, not fetched");
            }
            Err(error) => panic!("{stellar}: {error}"),
            Ok(planet) => {
                let Some(pixels) = alone(&planet, pose(CENTRE.0, CENTRE.1)) else {
                    return;
                };
                let lit_point = rgb(&pixels, pixel_at(CENTRE, (-0.3, -0.3)));
                assert!(lit_point != [0, 0, 0], "{stellar} draws");
            }
        }
    }
    // The format's Source reads the folder as the hermetic sample does.
    assert!(
        source
            .bytes("budgets/default.md")
            .is_ok_and(|b| b.is_some())
    );
}
