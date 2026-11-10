//! The colour of one fragment of a planet's square: its layers, each
//! worked out from the texels and ring opacities the shader samples, and
//! composited in order. `wgpu/planets.wgsl`'s fragment shader mirrors
//! [`fragment`] function for function (`halo`, `globe_color`,
//! `ring_color`, `over`), in the same order and with the same formulas.
//!
//! # The layers
//!
//! Bottom first:
//!
//! 1. **The halo**, off the disc: the atmosphere's colour lit as the limb
//!    under it is ([`halo`]), as opaque as [`rim`] says.
//! 2. **The globe**, as much of it as covers the fragment: its ground lit,
//!    the rings' shadow taking that light back toward the ambient, its
//!    glow showing unlit, its clouds white in the same light over it, and
//!    its rim tinted with the atmosphere ([`globe_color`]).
//! 3. **The rings**, where the view ray meets them in front of the globe:
//!    their colour, at the ambient in the globe's shadow ([`ring_color`]),
//!    as opaque as their profile is there.
//!
//! # What stays in the shader
//!
//! How much of a pixel the globe covers ([`Fragment::coverage`]) is a
//! matter of rasterisation, the anti-aliased edge the shader takes from
//! the screen-space derivative of the distance from the centre, so the
//! shader works it out and the core takes it as given. So does sampling:
//! which texel a globe point shows ([`super::surface_uv`]), filtered over
//! which mip level, and the rings' profile read at a radius
//! (`RingBand::opacity`) reach [`fragment`] as their results.

use crate::camera::Light;

use super::{Frame, globe, in_planet_shadow, lit, rim, ring_hit, ring_shadow};

/// What one fragment's colour is worked out from: where it is, the
/// planet's axes, the light, the texels its globe point shows and its
/// layers' colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fragment {
    /// Where it is, in planet radii from the centre, on the view's axes.
    pub x: f32,
    /// See [`Fragment::x`].
    pub y: f32,
    /// The planet's axes.
    pub frame: Frame,
    /// The light it is drawn in.
    pub light: Light,
    /// The surface texel its globe point shows: colour, then glow (how
    /// much of it shows unlit), each 0 to 1.
    pub ground: [f32; 4],
    /// The cloud texel its globe point shows: colour, then cover. The
    /// clouds draw white whatever the colour.
    pub cloud: [f32; 4],
    /// The rings' colour.
    pub ring_color: [f32; 3],
    /// The atmosphere's colour, then its thickness in planet radii (0
    /// for none).
    pub atmosphere: [f32; 4],
    /// How much of the fragment the globe covers, 0 to 1: the shader's
    /// anti-aliased edge.
    pub coverage: f32,
}

/// The straight-alpha colour of `fragment`, its rings' opacity at a
/// radius (in planet radii) read by `opacity` (0 off the rings). `None`
/// when nothing covers it.
#[must_use]
pub fn fragment(fragment: &Fragment, opacity: impl Fn(f32) -> f32) -> Option<[f32; 4]> {
    let Fragment {
        x,
        y,
        frame,
        light,
        ground,
        cloud,
        ring_color: rings,
        atmosphere,
        coverage,
    } = *fragment;
    let d = (x * x + y * y).sqrt();
    let on_disc = globe(x, y).is_some();
    let mut acc = [0.0; 4];
    if !on_disc {
        let glow = halo(x, y, atmosphere, light);
        acc = over(acc, rgb(glow), glow[3]);
    }
    // The globe, as much of it as covers the fragment. Its normal is
    // clamped onto the limb off the disc, where only its anti-aliased edge
    // shows. (The shader skips it where it covers nothing, and the rings
    // where they are clear: a layer at alpha 0 changes nothing.)
    let n = [x, y, (1.0 - x * x - y * y).max(0.0).sqrt()];
    let shadow = ring_shadow(n, &frame, light).map_or(0.0, &opacity);
    acc = over(
        acc,
        globe_color(n, d, ground, cloud, shadow, atmosphere, light),
        coverage,
    );
    if let Some(hit) = ring_hit(x, y, &frame).filter(|hit| hit.in_front) {
        acc = over(
            acc,
            ring_color(rings, in_planet_shadow(hit.point, light), light),
            opacity(hit.radius),
        );
    }
    let alpha = acc[3];
    (alpha > 0.0).then(|| {
        let [red, green, blue] = rgb(acc).map(|c| c / alpha);
        [red, green, blue, alpha]
    })
}

/// The halo at `(x, y)`, off the disc, on a planet whose atmosphere is
/// `atmosphere` (colour, then thickness): its colour lit as the limb
/// under it is, then how opaque it is ([`rim`]).
#[must_use]
pub fn halo(x: f32, y: f32, atmosphere: [f32; 4], light: Light) -> [f32; 4] {
    let d = (x * x + y * y).sqrt();
    let [red, green, blue] = times(rgb(atmosphere), lit([x / d, y / d, 0.0], light));
    [red, green, blue, rim(d, atmosphere[3])]
}

/// The globe's colour at its point `n`, `d` planet radii from the centre
/// on screen, showing the surface texel `ground` and cloud texel `cloud`,
/// with the rings `shadow` opaque where its ray toward the light crosses
/// them: the ground lit, the rings' shadow taking the light back toward
/// the ambient by their opacity; its glow unlit; its clouds white in the
/// same light over it, by their cover; and the atmosphere's colour in that
/// light over all, by [`rim`].
#[must_use]
pub fn globe_color(
    n: [f32; 3],
    d: f32,
    ground: [f32; 4],
    cloud: [f32; 4],
    shadow: f32,
    atmosphere: [f32; 4],
    light: Light,
) -> [f32; 3] {
    let ambient = light.ambient;
    let shade = ambient + (lit(n, light) - ambient) * (1.0 - shadow);
    let surface = mix(times(rgb(ground), shade), rgb(ground), ground[3]);
    let clouded = mix(surface, [shade; 3], cloud[3]);
    mix(
        clouded,
        times(rgb(atmosphere), shade),
        rim(d.min(1.0), atmosphere[3]),
    )
}

/// The rings' colour `color` where they are seen: lit fully, or at
/// `light`'s ambient where they are in the globe's shadow.
#[must_use]
pub fn ring_color(color: [f32; 3], shadowed: bool, light: Light) -> [f32; 3] {
    times(color, if shadowed { light.ambient } else { 1.0 })
}

/// `color` at `alpha` over `under`, premultiplied.
fn over(under: [f32; 4], color: [f32; 3], alpha: f32) -> [f32; 4] {
    let keep = 1.0 - alpha;
    let [red, green, blue] = [0, 1, 2].map(|i| color[i] * alpha + under[i] * keep);
    [red, green, blue, alpha + under[3] * keep]
}

/// `color`'s red, green and blue.
fn rgb(color: [f32; 4]) -> [f32; 3] {
    [color[0], color[1], color[2]]
}

fn times(color: [f32; 3], by: f32) -> [f32; 3] {
    color.map(|c| c * by)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;

    use super::super::{PLANET_LIGHT, VIEW_LEAN};
    use super::*;

    #[track_caller]
    fn assert_close(a: [f32; 4], b: [f32; 4]) {
        assert!(
            a.iter().zip(&b).all(|(p, q)| (p - q).abs() < 1e-5),
            "{a:?} is not {b:?}"
        );
    }

    const GROUND: [f32; 4] = [0.8, 0.4, 0.2, 0.0];
    const RINGS: [f32; 3] = [0.9, 0.6, 0.3];

    /// A fragment at `(x, y)` on an upright planet in the planet light,
    /// showing [`GROUND`] with no cloud, glow or atmosphere, the globe
    /// covering it wholly where it is on the disc.
    fn at(x: f32, y: f32) -> Fragment {
        Fragment {
            x,
            y,
            frame: Frame::new(0.0, VIEW_LEAN),
            light: PLANET_LIGHT,
            ground: GROUND,
            cloud: [1.0, 1.0, 1.0, 0.0],
            ring_color: RINGS,
            atmosphere: [0.0; 4],
            coverage: if globe(x, y).is_some() { 1.0 } else { 0.0 },
        }
    }

    fn no_rings(_: f32) -> f32 {
        0.0
    }

    fn scaled([red, green, blue, a]: [f32; 4], by: f32) -> [f32; 4] {
        [red * by, green * by, blue * by, a]
    }

    #[test]
    fn the_ground_is_lit_and_its_glow_shows_unlit() {
        let ambient = PLANET_LIGHT.ambient;
        // On the night side, lit by the ambient alone.
        let night = at(0.6, 0.6);
        assert_eq!(lit(globe(0.6, 0.6).unwrap(), PLANET_LIGHT), ambient);
        let dark = fragment(&night, no_rings).unwrap();
        assert_close(dark, scaled([0.8, 0.4, 0.2, 1.0], ambient));
        let glowing = Fragment {
            ground: [0.8, 0.4, 0.2, 1.0],
            ..night
        };
        assert_close(fragment(&glowing, no_rings).unwrap(), [0.8, 0.4, 0.2, 1.0]);
        let half = Fragment {
            ground: [0.8, 0.4, 0.2, 0.5],
            ..night
        };
        let mid = f32::midpoint(1.0, ambient);
        assert_close(
            fragment(&half, no_rings).unwrap(),
            scaled([0.8, 0.4, 0.2, 1.0], mid),
        );
        // On the day side, by the light.
        let shade = lit(globe(-0.3, -0.3).unwrap(), PLANET_LIGHT);
        assert!(shade > 0.9, "{shade}");
        assert_close(
            fragment(&at(-0.3, -0.3), no_rings).unwrap(),
            scaled([0.8, 0.4, 0.2, 1.0], shade),
        );
    }

    #[test]
    fn the_clouds_are_white_in_the_ground_s_light_over_it_by_their_cover() {
        let shade = lit(globe(-0.3, -0.3).unwrap(), PLANET_LIGHT);
        let covered = Fragment {
            // Whatever the texel's colour, the clouds draw white.
            cloud: [0.1, 0.2, 0.3, 1.0],
            ..at(-0.3, -0.3)
        };
        assert_close(
            fragment(&covered, no_rings).unwrap(),
            [shade, shade, shade, 1.0],
        );
        let thin = Fragment {
            cloud: [1.0, 1.0, 1.0, 0.25],
            ..covered
        };
        let ground = [0.8, 0.4, 0.2].map(|c| c * shade);
        let [red, green, blue] = ground.map(|c| c + (shade - c) * 0.25);
        assert_close(fragment(&thin, no_rings).unwrap(), [red, green, blue, 1.0]);
        // Over a glow too: the clouds cover it.
        let over_glow = Fragment {
            ground: [0.8, 0.4, 0.2, 1.0],
            ..covered
        };
        assert_close(
            fragment(&over_glow, no_rings).unwrap(),
            [shade, shade, shade, 1.0],
        );
    }

    #[test]
    fn the_rings_shadow_takes_the_globe_s_light_back_toward_the_ambient() {
        let here = at(0.0, 0.6);
        let n = globe(0.0, 0.6).unwrap();
        let crossing = ring_shadow(n, &here.frame, PLANET_LIGHT).expect("looks up through them");
        let hit = ring_hit(0.0, 0.6, &here.frame).unwrap().radius;
        assert!((crossing - hit).abs() > 0.1, "{crossing} {hit}");
        let read = RefCell::new(Vec::new());
        let read = &read;
        let only_there = |opacity: f32| {
            move |r: f32| {
                read.borrow_mut().push(r);
                if (r - crossing).abs() < 1e-4 {
                    opacity
                } else {
                    0.0
                }
            }
        };
        let ambient = PLANET_LIGHT.ambient;
        let shade = lit(n, PLANET_LIGHT);
        let full = fragment(&here, only_there(1.0)).unwrap();
        assert_close(full, scaled([0.8, 0.4, 0.2, 1.0], ambient));
        assert!(read.borrow().contains(&crossing));
        let half = fragment(&here, only_there(0.5)).unwrap();
        let between = ambient + (shade - ambient) * 0.5;
        assert_close(half, scaled([0.8, 0.4, 0.2, 1.0], between));
        // Its glow shows unlit through the shadow.
        let glowing = Fragment {
            ground: [0.8, 0.4, 0.2, 1.0],
            ..here
        };
        assert_close(
            fragment(&glowing, only_there(1.0)).unwrap(),
            [0.8, 0.4, 0.2, 1.0],
        );
        // Unshaded where its ray crosses no ring.
        assert_close(
            fragment(&here, no_rings).unwrap(),
            scaled([0.8, 0.4, 0.2, 1.0], shade),
        );
    }

    /// A ring point off the disc, seen, on a planet tilted 0.4 radians,
    /// on whose rings the globe's shadow then shows: `(x, y)`.
    fn shadowed_ring() -> (Frame, f32, f32) {
        let frame = Frame::new(0.4, VIEW_LEAN);
        let (sin, cos) = 298_f32.to_radians().sin_cos();
        let p = [0, 1, 2].map(|i| 1.9 * (cos * frame.east[i] + sin * frame.facing[i]));
        (frame, p[0], p[1])
    }

    #[test]
    fn the_globe_s_shadow_leaves_the_rings_at_the_ambient() {
        let (frame, x, y) = shadowed_ring();
        let hit = ring_hit(x, y, &frame).unwrap();
        assert!(globe(x, y).is_none() && hit.in_front);
        assert!(in_planet_shadow(hit.point, PLANET_LIGHT));
        let ring = Fragment { frame, ..at(x, y) };
        let ambient = PLANET_LIGHT.ambient;
        let [red, green, blue] = RINGS.map(|c| c * ambient);
        assert_close(fragment(&ring, |_| 0.5).unwrap(), [red, green, blue, 0.5]);
        // Out of it, fully lit.
        let (sin, cos) = 0.5_f32.sin_cos();
        let lit_x = 1.9 * (cos * frame.east[0] + sin * frame.facing[0]);
        let lit_y = 1.9 * (cos * frame.east[1] + sin * frame.facing[1]);
        let sunlit = ring_hit(lit_x, lit_y, &frame).unwrap();
        assert!(sunlit.in_front && !in_planet_shadow(sunlit.point, PLANET_LIGHT));
        let [red, green, blue] = RINGS;
        assert_close(
            fragment(
                &Fragment {
                    frame,
                    ..at(lit_x, lit_y)
                },
                |_| 0.5,
            )
            .unwrap(),
            [red, green, blue, 0.5],
        );
        assert_eq!(
            ring_color(RINGS, true, PLANET_LIGHT),
            RINGS.map(|c| c * ambient)
        );
        assert_eq!(ring_color(RINGS, false, PLANET_LIGHT), RINGS);
    }

    #[test]
    fn the_halo_is_lit_as_the_limb_under_it_and_as_opaque_as_the_rim() {
        let atmosphere = [0.2, 0.6, 1.0, 0.1];
        // Up-left, toward the light, and down-right, away from it.
        for (x, y) in [(-0.74_f32, -0.74_f32), (0.74, 0.74), (1.05, 0.0)] {
            let d = (x * x + y * y).sqrt();
            assert!(d > 1.0 && d < 1.1);
            let here = Fragment {
                atmosphere,
                ..at(x, y)
            };
            let shade = lit([x / d, y / d, 0.0], PLANET_LIGHT);
            let [red, green, blue] = [0.2, 0.6, 1.0].map(|c| c * shade);
            let got = fragment(&here, no_rings).unwrap();
            assert_close(got, [red, green, blue, rim(d, 0.1)]);
            assert_close(halo(x, y, atmosphere, PLANET_LIGHT), got);
        }
        assert!(
            lit([-0.7, -0.7, 0.0], PLANET_LIGHT) > lit([0.7, 0.7, 0.0], PLANET_LIGHT),
            "the light's side is brighter"
        );
        // Beyond it, and with no atmosphere at all, nothing.
        let beyond = Fragment {
            atmosphere,
            ..at(1.2, 0.0)
        };
        assert_eq!(fragment(&beyond, no_rings), None);
        assert_eq!(fragment(&at(1.05, 0.0), no_rings), None);
        // None on the disc, even where the globe only partly covers it:
        // the globe's edge there shows over nothing.
        let edge = Fragment {
            atmosphere,
            coverage: 0.5,
            ..at(0.999, 0.0)
        };
        assert!(globe(0.999, 0.0).is_some());
        assert!(rim(0.999, 0.1) > 0.0);
        assert_eq!(fragment(&edge, no_rings).map(|c| c[3]), Some(0.5));
    }

    #[test]
    fn the_rim_tints_the_globe_with_the_atmosphere_in_its_light() {
        let atmosphere = [0.2, 0.6, 1.0, 0.1];
        let (x, y) = (-0.6_f32, -0.6_f32);
        let d = (x * x + y * y).sqrt();
        let shade = lit(globe(x, y).unwrap(), PLANET_LIGHT);
        let here = Fragment {
            atmosphere,
            ..at(x, y)
        };
        let tint = rim(d, 0.1);
        assert!(tint > 0.0);
        let [red, green, blue] = [0, 1, 2].map(|i| {
            let ground = GROUND[i] * shade;
            ground + (atmosphere[i] * shade - ground) * tint
        });
        assert_close(fragment(&here, no_rings).unwrap(), [red, green, blue, 1.0]);
        // The globe's edge, just off the disc, tints as the limb does.
        let edge = Fragment {
            atmosphere,
            coverage: 0.5,
            ..at(1.001, 0.0)
        };
        let n = [1.001, 0.0, 0.0];
        let globe = globe_color(n, 1.001, GROUND, [0.0; 4], 0.0, atmosphere, PLANET_LIGHT);
        let tinted = globe_color(n, 1.0, GROUND, [0.0; 4], 0.0, atmosphere, PLANET_LIGHT);
        assert_eq!(globe, tinted);
        let [hr, hg, hb, ha] = halo(1.001, 0.0, atmosphere, PLANET_LIGHT);
        let under = [hr * ha, hg * ha, hb * ha, ha];
        let alpha = 0.5 + ha * 0.5;
        let [red, green, blue] = [0, 1, 2].map(|i| (globe[i] * 0.5 + under[i] * 0.5) / alpha);
        assert_close(
            fragment(&edge, no_rings).unwrap(),
            [red, green, blue, alpha],
        );
    }

    #[test]
    fn the_rings_cover_the_globe_in_front_of_it_and_hide_behind_it() {
        let frame = Frame::new(0.0, VIEW_LEAN);
        let half_height = 1.6 * VIEW_LEAN.sin();
        let rings = |r: f32| if (1.4..=2.3).contains(&r) { 0.75 } else { 0.0 };
        // Below the centre, in front: the rings over the globe.
        let front = at(0.0, half_height);
        let hit = ring_hit(0.0, half_height, &frame).unwrap();
        assert!(hit.in_front);
        let shadowed = in_planet_shadow(hit.point, PLANET_LIGHT);
        let under = fragment(&front, no_rings).unwrap();
        let ring = ring_color(RINGS, shadowed, PLANET_LIGHT);
        let [red, green, blue] = [0, 1, 2].map(|i| ring[i] * 0.75 + under[i] * 0.25);
        let shade_free = |r: f32| {
            if (r - hit.radius).abs() < 1e-4 {
                rings(r)
            } else {
                0.0
            }
        };
        assert_close(
            fragment(&front, shade_free).unwrap(),
            [red, green, blue, 1.0],
        );
        // Above it, behind: the globe alone, as with no rings.
        let behind = at(0.0, -half_height);
        let hidden = ring_hit(0.0, -half_height, &frame).unwrap();
        assert!(!hidden.in_front);
        let only_hit = |r: f32| {
            if (r - hidden.radius).abs() < 1e-4 {
                rings(r)
            } else {
                0.0
            }
        };
        assert_eq!(fragment(&behind, only_hit), fragment(&behind, no_rings));
        // Off the disc, over nothing: their colour at their opacity.
        let side = at(1.6, 0.0);
        let [red, green, blue] = ring_color(
            RINGS,
            in_planet_shadow(ring_hit(1.6, 0.0, &frame).unwrap().point, PLANET_LIGHT),
            PLANET_LIGHT,
        );
        assert_close(fragment(&side, rings).unwrap(), [red, green, blue, 0.75]);
        // Nothing at all off the disc with no rings or atmosphere.
        assert_eq!(fragment(&side, no_rings), None);
    }
}
