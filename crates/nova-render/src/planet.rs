//! The geometry a planet ([`nova_view::DrawCommand::Planet`]) is drawn
//! with, worked out here so the shader (`wgpu/planets.wgsl`) only mirrors
//! it: which square it covers, where a fragment meets the globe, the rings
//! and the atmosphere, which surface texel it shows, and the shadows.
//!
//! # The view
//!
//! The camera is the models' (`camera`): orthographic, looking straight
//! down, so a globe is exactly a disc and one screen-aligned square
//! covers a planet ([`quad`]). Each fragment works in planet radii from
//! the centre, on the view's axes: x right, y down, z toward the camera.
//! The view ray through `(x, y)` meets the globe's near side at
//! `z = √(1 − x² − y²)` ([`globe`]).
//!
//! A planet is drawn as globes are in a top-down space game, its equator
//! across the screen: its spin axis points up the screen, leaned toward
//! the camera by [`VIEW_LEAN`] so the north pole tips into view and the
//! rings, in the equatorial plane, show as an ellipse. Its axial tilt then
//! rolls the axis in the screen plane, clockwise on screen as headings
//! turn ([`Frame`]).
//!
//! # The surface
//!
//! A globe point's latitude and longitude on the planet's [`Frame`], with
//! the globe turned prograde by [`turns`], pick an equirectangular texel
//! ([`surface_uv`]): `u` east from the date line, wrapping, `v` down from
//! the north pole. At turn 0 the date line faces the camera.
//!
//! # The light
//!
//! The planet is lit from [`Light::STANDARD`]'s direction, as models are,
//! with a lower ambient ([`PLANET_LIGHT`]) so the night side is dark.
//! The rings take the globe's shadow ([`in_planet_shadow`]) and cast
//! theirs onto it ([`ring_shadow`]); the atmosphere is a rim glow
//! ([`rim`]) lit as the limb under it is.
//!
//! # The fragment
//!
//! How those pieces make one fragment's colour (the layers, their order
//! and how each is lit and mixed) is [`fragment()`]'s, in its own module,
//! which the shader mirrors function for function.

mod fragment;

pub use fragment::{Fragment, fragment, globe_color, halo, ring_color};

use nova_galaxy_media::planet::{Image, Surface};
use nova_view::PlanetPose;
use nova_view::geometry::{Bounds, Point};

use crate::camera::Light;
use crate::gpu::{PlanetId, PlanetInstance, PlanetTextures, Rect};

/// How far a planet's spin axis leans from screen-up toward the camera,
/// in radians (20°).
pub const VIEW_LEAN: f32 = 0.349_065_84;

/// How much faster the cloud layer turns than the surface, so it drifts.
pub const CLOUD_SPIN: f64 = 1.2;

/// The light a planet is drawn in: [`Light::STANDARD`]'s direction, with
/// little ambient, so its night side is dark.
pub const PLANET_LIGHT: Light = Light {
    direction: Light::STANDARD.direction,
    ambient: 0.06,
};

/// How strongly the atmosphere's colour tints the globe at its limb and
/// the halo just outside it.
pub const RIM: f32 = 0.6;

/// A planet's axes on the view's: its north pole, the point of its
/// equator nearest the camera (where longitude 0 faces at turn 0), and
/// east, each a unit vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Toward its north pole.
    pub north: [f32; 3],
    /// Toward the point of its equator nearest the camera.
    pub facing: [f32; 3],
    /// Toward its east, along the equator.
    pub east: [f32; 3],
}

impl Frame {
    /// The axes of a planet whose axis leans `lean` radians toward the
    /// camera from screen-up and is rolled `tilt` radians on screen,
    /// clockwise as headings turn.
    #[must_use]
    pub fn new(tilt: f32, lean: f32) -> Self {
        let (sin_lean, cos_lean) = lean.sin_cos();
        let (sin, cos) = tilt.sin_cos();
        // About the view's z, +x toward +y.
        let roll = |[x, y, z]: [f32; 3]| [cos * x - sin * y, sin * x + cos * y, z];
        Frame {
            north: roll([0.0, -cos_lean, sin_lean]),
            facing: roll([0.0, sin_lean, cos_lean]),
            east: roll([1.0, 0.0, 0.0]),
        }
    }
}

/// How far beyond the globe a planet reaches, in its radii: the furthest
/// of the globe (1), its atmosphere's edge (`1 + thickness`) and its
/// rings' outer edge. A ring's circle shows as an ellipse whose long axis
/// lies in the screen plane whatever the lean and tilt, so its outer
/// radius is its reach on screen too.
#[must_use]
pub fn reach(thickness: Option<f32>, outer: Option<f32>) -> f32 {
    let halo = thickness.map_or(1.0, |t| 1.0 + t);
    halo.max(outer.unwrap_or(1.0)).max(1.0)
}

/// How far a planet drawn with `surface` reaches, in its radii: the
/// [`reach`] of its atmosphere's thickness and its rings' outer edge.
#[must_use]
pub fn surface_reach(surface: &Surface) -> f32 {
    #[allow(clippy::cast_possible_truncation, reason = "a shader's sizes")]
    reach(
        surface.atmosphere.as_ref().map(|a| a.thickness as f32),
        surface.rings.as_ref().map(|r| r.outer as f32),
    )
}

/// The square a planet at `pose`, reaching `reach` of its radii, covers on
/// screen, in logical units. `None` when it draws nothing: a radius or
/// reach that is not a positive, finite number, or a centre or time that
/// is not finite.
#[must_use]
pub fn quad(pose: &PlanetPose, reach: f32) -> Option<Bounds> {
    let positive = |v: f32| v.is_finite() && v > 0.0;
    let PlanetPose {
        center,
        radius,
        time,
    } = *pose;
    if !positive(radius)
        || !positive(reach)
        || !center.x.is_finite()
        || !center.y.is_finite()
        || !time.is_finite()
    {
        return None;
    }
    let half = radius * reach;
    let half = half.is_finite().then_some(half)?;
    Some(Bounds {
        min: Point::new(center.x - half, center.y - half),
        max: Point::new(center.x + half, center.y + half),
    })
}

/// Where the view ray through `(x, y)` (in planet radii from the centre)
/// meets the globe's near side, which is also the globe's normal there.
/// `None` off the disc.
#[must_use]
pub fn globe(x: f32, y: f32) -> Option<[f32; 3]> {
    let z2 = 1.0 - x * x - y * y;
    (z2 >= 0.0).then(|| [x, y, z2.sqrt()])
}

/// The equirectangular `(u, v)` a globe point `n` shows, on a planet with
/// axes `frame`, `turn` turns (0 to 1) after its date line faced the
/// camera: `u` east from the date line, wrapping into `[0, 1)`, `v` 0 at
/// the north pole and 1 at the south.
#[must_use]
pub fn surface_uv(n: [f32; 3], frame: &Frame, turn: f32) -> [f32; 2] {
    let latitude = dot(n, frame.north).clamp(-1.0, 1.0).asin();
    let longitude = dot(n, frame.east).atan2(dot(n, frame.facing));
    let u = longitude / std::f32::consts::TAU - turn;
    [u - u.floor(), 0.5 - latitude / std::f32::consts::PI]
}

/// The latitude, in radians, of a globe point `n` on a planet with axes
/// `frame`.
#[must_use]
pub fn latitude(n: [f32; 3], frame: &Frame) -> f32 {
    dot(n, frame.north).clamp(-1.0, 1.0).asin()
}

/// Where the view ray through `(x, y)` meets a planet's ring plane (its
/// equator's), and whether that point is seen there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingHit {
    /// The point, in planet radii on the view's axes.
    pub point: [f32; 3],
    /// How far it is from the centre, in planet radii.
    pub radius: f32,
    /// Whether it is in front of the globe: off the disc, or nearer the
    /// camera than the globe's near side. A ring point behind the globe
    /// is hidden by it.
    pub in_front: bool,
}

/// Where the view ray through `(x, y)` meets the ring plane of a planet
/// with axes `frame`. `None` when the ray runs along the plane (a planet
/// seen edge-on, which a lean never is).
#[must_use]
pub fn ring_hit(x: f32, y: f32, frame: &Frame) -> Option<RingHit> {
    let north = frame.north;
    if north[2].abs() < 1e-6 {
        return None;
    }
    let z = -(x * north[0] + y * north[1]) / north[2];
    let point = [x, y, z];
    let in_front = globe(x, y).is_none_or(|near| z > near[2]);
    Some(RingHit {
        point,
        radius: dot(point, point).sqrt(),
        in_front,
    })
}

/// Whether `point` (outside the globe, in planet radii) is in the globe's
/// shadow from `light`: the ray from it toward the light meets the globe.
#[must_use]
pub fn in_planet_shadow(point: [f32; 3], light: Light) -> bool {
    let l = unit(light.direction);
    let along = dot(point, l);
    along < 0.0 && along * along - (dot(point, point) - 1.0) >= 0.0
}

/// How far from the centre, in planet radii, the ray from the globe point
/// `n` toward `light` crosses the ring plane of a planet with axes
/// `frame`: the ring radius whose opacity shades it. `None` when it never
/// crosses it (it heads away from the plane, or along it).
#[must_use]
pub fn ring_shadow(n: [f32; 3], frame: &Frame, light: Light) -> Option<f32> {
    let l = unit(light.direction);
    let toward = dot(l, frame.north);
    if toward.abs() < 1e-6 {
        return None;
    }
    let t = -dot(n, frame.north) / toward;
    (t > 0.0).then(|| {
        let q = [0, 1, 2].map(|i| n[i] + t * l[i]);
        dot(q, q).sqrt()
    })
}

/// How lit a surface facing `normal` is by `light`, 0 to 1, as
/// [`crate::camera::shade`] lights a model: `ambient + (1 − ambient) ·
/// max(0, n · l)`.
#[must_use]
pub fn lit(normal: [f32; 3], light: Light) -> f32 {
    crate::camera::shade(normal, [1.0; 4], light)[0]
}

/// How strongly the atmosphere tints a fragment `d` planet radii from the
/// centre, on a planet whose atmosphere is `thickness` radii thick: on
/// the disc, rising to [`RIM`] at the limb; beyond it, a halo fading from
/// [`RIM`] to nothing at `1 + thickness`, and 0 from there on.
#[must_use]
pub fn rim(d: f32, thickness: f32) -> f32 {
    if thickness.is_nan() || thickness <= 0.0 || d >= 1.0 + thickness {
        return 0.0;
    }
    if d <= 1.0 {
        let d2 = d * d;
        return RIM * d2 * d2;
    }
    let fade = 1.0 - (d - 1.0) / thickness;
    RIM * fade * fade
}

/// The surface's and the clouds' turns (each 0 to 1) at `time` seconds,
/// on a planet whose day is `day` seconds long: the clouds turn
/// [`CLOUD_SPIN`] times as fast. A day that is not a positive, finite
/// number doesn't turn.
#[must_use]
pub fn turns(time: f64, day: f64) -> (f32, f32) {
    if !(day.is_finite() && day > 0.0 && time.is_finite()) {
        return (0.0, 0.0);
    }
    let turn = |t: f64| (t - t.floor()) as f32;
    (turn(time / day), turn(time * CLOUD_SPIN / day))
}

/// The textures `surface` is drawn with ([`crate::Gpu::upload_planet`]):
/// its surface's and clouds' mip chains ([`mips`]), a wholly clear 2 by 1
/// layer for a planet with no clouds, and its rings' profile as a row (a
/// clear pair of texels with no rings).
#[must_use]
pub fn textures(surface: &Surface) -> PlanetTextures {
    let clear = Image {
        width: 2,
        height: 1,
        rgba: vec![0; 8],
    };
    let profile = surface
        .rings
        .as_ref()
        .map_or(&[0, 0][..], |rings| &rings.profile);
    PlanetTextures {
        surface: mips(&surface.surface),
        clouds: mips(surface.clouds.as_ref().unwrap_or(&clear)),
        rings: Image {
            width: profile.len() as u32,
            height: 1,
            rgba: profile.iter().flat_map(|o| [*o; 4]).collect(),
        },
    }
}

/// The instance that draws `surface`, held as `planet`, at `pose`: its
/// square ([`quad`] over its [`reach`]), its [`Frame`] leaned by
/// [`VIEW_LEAN`] and rolled by its tilt, its [`turns`] at the pose's time,
/// and its rings' and atmosphere's sizes and colours. `None` when it draws
/// nothing ([`quad`]).
#[must_use]
pub fn instance(planet: PlanetId, surface: &Surface, pose: &PlanetPose) -> Option<PlanetInstance> {
    let thickness = surface.atmosphere.as_ref().map(|a| a.thickness as f32);
    let reach = surface_reach(surface);
    let square = quad(pose, reach)?;
    let color = |c: [u8; 3]| c.map(|c| f32::from(c) / 255.0);
    let ring_color = surface.rings.as_ref().map_or([0.0; 3], |r| color(r.color));
    let [r, g, b] = surface
        .atmosphere
        .as_ref()
        .map_or([0.0; 3], |a| color(a.color));
    Some(PlanetInstance {
        planet,
        dest: Rect {
            x: square.min.x,
            y: square.min.y,
            w: square.max.x - square.min.x,
            h: square.max.y - square.min.y,
        },
        reach,
        frame: Frame::new(surface.tilt as f32, VIEW_LEAN),
        turns: turns(pose.time, surface.day).into(),
        rings: surface
            .rings
            .as_ref()
            .map_or([0.0; 2], |r| [r.inner as f32, r.outer as f32]),
        ring_color,
        atmosphere: [r, g, b, thickness.unwrap_or(0.0)],
    })
}

/// `image`'s mip chain, itself first: each level half the one before on
/// each side (to 1 at least) down to 1 by 1, each texel the average of
/// the texels above it, rounded. Every channel, alpha too, is averaged on
/// its own: a planet's alpha is glow or cover, not a mask, so nothing is
/// premultiplied.
#[must_use]
pub fn mips(image: &Image) -> Vec<Image> {
    let mut chain = vec![image.clone()];
    while let Some(last) = chain.last().filter(|i| i.width > 1 || i.height > 1) {
        chain.push(halved(last));
    }
    chain
}

/// `image` halved by the box filter.
fn halved(image: &Image) -> Image {
    let (width, height) = ((image.width / 2).max(1), (image.height / 2).max(1));
    let (sx, sy) = (image.width / width, image.height / height);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            for channel in 0..4 {
                let mut sum = 0_u32;
                for dy in 0..sy {
                    for dx in 0..sx {
                        sum += u32::from(image.texel(x * sx + dx, y * sy + dy)[channel]);
                    }
                }
                let n = sx * sy;
                rgba.push(((sum + n / 2) / n) as u8);
            }
        }
    }
    Image {
        width,
        height,
        rgba,
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    v.map(|c| c / length)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use nova_galaxy_media::planet::{Glow, RingBand};

    use super::*;

    #[track_caller]
    fn assert_near(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-5, "{a} is not {b}");
    }

    fn upright() -> Frame {
        Frame::new(0.0, VIEW_LEAN)
    }

    #[test]
    fn a_frame_s_axes_are_unit_and_square() {
        for (tilt, lean) in [(0.0, VIEW_LEAN), (0.4, VIEW_LEAN), (-0.2, 0.1)] {
            let f = Frame::new(tilt, lean);
            for v in [f.north, f.facing, f.east] {
                assert_near(dot(v, v), 1.0);
            }
            assert_near(dot(f.north, f.facing), 0.0);
            assert_near(dot(f.north, f.east), 0.0);
            assert_near(dot(f.facing, f.east), 0.0);
        }
    }

    #[test]
    fn the_centre_shows_the_view_lean_s_latitude_on_the_date_line_at_turn_0() {
        let n = globe(0.0, 0.0).unwrap();
        assert_eq!(n, [0.0, 0.0, 1.0]);
        assert_near(latitude(n, &upright()), VIEW_LEAN);
        let [u, v] = surface_uv(n, &upright(), 0.0);
        assert_near(u, 0.0);
        assert_near(v, 0.5 - VIEW_LEAN / std::f32::consts::PI);
        // With no lean, the centre is on the equator.
        let flat = Frame::new(0.0, 0.0);
        assert_near(latitude(n, &flat), 0.0);
        assert_near(surface_uv(n, &flat, 0.0)[1], 0.5);
    }

    #[test]
    fn a_whole_day_turns_back_to_the_same_u_and_u_wraps() {
        let n = globe(0.3, -0.2).unwrap();
        let f = upright();
        let [u0, v0] = surface_uv(n, &f, 0.25);
        let [u1, v1] = surface_uv(n, &f, 1.25);
        assert_near(u0, u1);
        assert_eq!(v0, v1);
        for turn in [0.0, 0.3, 0.75, 0.999, 5.5] {
            let [u, _] = surface_uv(n, &f, turn);
            assert!((0.0..1.0).contains(&u), "{u}");
        }
        // Turning moves the surface east: what showed at u shows a little
        // further west a moment later.
        let [later, _] = surface_uv(n, &f, 0.26);
        assert_near(later, u0 - 0.01);
    }

    #[test]
    fn east_on_screen_is_east_on_the_globe() {
        let f = Frame::new(0.0, 0.0);
        let [u, v] = surface_uv(globe(1.0, 0.0).unwrap(), &f, 0.0);
        assert_near(u, 0.25);
        assert_near(v, 0.5);
    }

    #[test]
    fn north_on_screen_is_north_on_the_globe_and_the_tilt_rolls_it() {
        // Up the screen is north, at tilt 0.
        let top = globe(0.0, -0.99).unwrap();
        let bottom = globe(0.0, 0.99).unwrap();
        assert!(latitude(top, &upright()) > 1.0);
        assert!(latitude(bottom, &upright()) < -0.5);
        // A quarter turn clockwise puts north to the right.
        let rolled = Frame::new(FRAC_PI_2, 0.0);
        assert_near(latitude(globe(1.0, 0.0).unwrap(), &rolled), FRAC_PI_2);
        assert_near(latitude(top, &rolled), 0.0);
        assert!(latitude(globe(-1.0, 0.0).unwrap(), &rolled) < -1.5);
        // A small tilt leans north toward the right.
        let tilted = Frame::new(0.3, VIEW_LEAN);
        assert!(tilted.north[0] > 0.0 && tilted.north[1] < 0.0);
    }

    #[test]
    fn a_point_beyond_the_reach_or_off_the_disc_hits_nothing() {
        assert_eq!(globe(0.8, 0.7), None);
        assert!(globe(0.6, 0.79).is_some(), "inside the limb is on the disc");
        assert_eq!(globe(1.0, 0.0), Some([1.0, 0.0, 0.0]), "the limb is too");
        assert_eq!(reach(None, None), 1.0);
        assert_eq!(reach(Some(0.04), None), 1.04);
        assert_eq!(reach(Some(0.04), Some(2.3)), 2.3);
        assert_eq!(reach(Some(0.5), Some(1.2)), 1.5);
        // A ring point past the reach is off the rings, and nothing else
        // is there.
        let f = upright();
        let far = ring_hit(2.31, 0.0, &f).unwrap();
        assert!(far.radius > 2.3);
        assert_eq!(rim(2.31, 0.04), 0.0);
    }

    #[test]
    fn the_quad_covers_the_reach_around_the_centre() {
        let pose = |x: f32, radius: f32, time: f64| PlanetPose {
            center: Point::new(x, 50.0),
            radius,
            time,
        };
        assert_eq!(
            quad(&pose(100.0, 20.0, 3.0), 2.5),
            Some(Bounds {
                min: Point::new(50.0, 0.0),
                max: Point::new(150.0, 100.0),
            })
        );
        for bad in [
            pose(100.0, 0.0, 0.0),
            pose(100.0, -1.0, 0.0),
            pose(100.0, f32::NAN, 0.0),
            pose(100.0, f32::INFINITY, 0.0),
            pose(f32::NAN, 20.0, 0.0),
            pose(f32::INFINITY, 20.0, 0.0),
            pose(100.0, 20.0, f64::NAN),
            pose(100.0, 20.0, f64::INFINITY),
            pose(100.0, f32::MAX, 0.0),
        ] {
            assert_eq!(quad(&bad, 2.0), None, "{bad:?}");
        }
        let bad_y = PlanetPose {
            center: Point::new(1.0, f32::NAN),
            ..pose(0.0, 1.0, 0.0)
        };
        assert_eq!(quad(&bad_y, 2.0), None);
        assert_eq!(quad(&pose(1.0, 1.0, 0.0), 0.0), None);
        assert_eq!(quad(&pose(1.0, 1.0, 0.0), f32::NAN), None);
    }

    #[test]
    fn the_rings_near_half_is_in_front_and_their_far_half_behind() {
        let f = upright();
        // A ring of radius 1.4 shows as an ellipse 1.4 · sin(lean) tall.
        let half_height = 1.4 * VIEW_LEAN.sin();
        let above = ring_hit(0.0, -half_height, &f).unwrap();
        assert_near(above.radius, 1.4);
        assert!(!above.in_front, "above the centre, behind the globe");
        let below = ring_hit(0.0, half_height, &f).unwrap();
        assert_near(below.radius, 1.4);
        assert!(below.in_front, "below the centre, in front of it");
        // Off the disc, the ellipse's ends are always seen.
        let side = ring_hit(1.4, 0.0, &f).unwrap();
        assert_near(side.radius, 1.4);
        assert!(side.in_front);
        assert_eq!(ring_hit(0.0, 0.5, &Frame::new(0.0, 0.0)), None, "edge-on");
    }

    #[test]
    fn a_ring_behind_the_globe_from_the_light_is_shadowed() {
        let l = unit(PLANET_LIGHT.direction);
        // Straight behind the globe from the light, outside it.
        let behind = l.map(|c| -1.5 * c);
        assert!(in_planet_shadow(behind, PLANET_LIGHT));
        let sunward = l.map(|c| 1.5 * c);
        assert!(!in_planet_shadow(sunward, PLANET_LIGHT));
        // Behind, but too far to the side for the globe to cover it.
        let f = upright();
        let aside = [0, 1, 2].map(|i| -1.5 * l[i] + 1.2 * f.east[i]);
        let sideways = dot(aside, l) * dot(aside, l) - (dot(aside, aside) - 1.0);
        assert_eq!(in_planet_shadow(aside, PLANET_LIGHT), sideways >= 0.0);
        // In the ring plane, at radius 1.4: straight away from the light
        // it is in the globe's shadow, and toward it, it isn't.
        let toward = dot(l, f.north);
        let flat = unit([0, 1, 2].map(|i| l[i] - toward * f.north[i]));
        let night = flat.map(|c| -1.4 * c);
        let day = flat.map(|c| 1.4 * c);
        assert_near(dot(night, f.north), 0.0);
        assert!(in_planet_shadow(night, PLANET_LIGHT));
        assert!(!in_planet_shadow(day, PLANET_LIGHT));
        // Far out on the night side, the globe no longer covers it.
        assert!(!in_planet_shadow(flat.map(|c| -3.0 * c), PLANET_LIGHT));
    }

    #[test]
    fn the_rings_shade_the_globe_where_its_sunward_ray_crosses_them() {
        let f = upright();
        let light = PLANET_LIGHT;
        // The light is up and in front: a point on the southern half,
        // facing the camera, looks up through the ring plane.
        let south = globe(0.0, 0.6).unwrap();
        let radius = ring_shadow(south, &f, light).expect("crosses the plane");
        let toward = unit(light.direction);
        let along = -dot(south, f.north) / dot(toward, f.north);
        assert!(along > 0.0);
        let crossing = [0, 1, 2].map(|i| south[i] + along * toward[i]);
        assert_near(dot(crossing, f.north), 0.0);
        assert_near(radius, dot(crossing, crossing).sqrt());
        // The north, the side the light is on, never looks through them.
        let north = globe(0.0, -0.6).unwrap();
        assert_eq!(ring_shadow(north, &f, light), None);
    }

    #[test]
    fn the_rim_rises_to_the_limb_and_the_halo_fades_by_its_thickness() {
        assert_eq!(rim(0.0, 0.1), 0.0);
        assert!(rim(0.5, 0.1) < rim(0.9, 0.1));
        assert_eq!(rim(1.0, 0.1), RIM);
        assert_near(rim(1.05, 0.1), RIM * 0.25);
        assert_eq!(rim(1.1, 0.1), 0.0);
        assert_eq!(rim(1.2, 0.1), 0.0);
        assert_eq!(rim(0.9, 0.0), 0.0, "no atmosphere, no rim");
        assert_eq!(rim(0.9, f32::NAN), 0.0);
    }

    #[test]
    fn the_light_need_not_be_a_unit_vector() {
        let long = Light {
            direction: PLANET_LIGHT.direction.map(|c| c * 3.0),
            ..PLANET_LIGHT
        };
        let l = unit(PLANET_LIGHT.direction);
        let near_behind = l.map(|c| -1.2 * c);
        assert_eq!(
            in_planet_shadow(near_behind, long),
            in_planet_shadow(near_behind, PLANET_LIGHT)
        );
        assert!(in_planet_shadow(near_behind, long));
        // Just past the globe's edge from the light: outside its shadow,
        // however long the light's vector.
        let f = upright();
        let aside = [0, 1, 2].map(|i| -2.0 * l[i] + 1.01 * unit([f.east[0], f.east[1], 0.0])[i]);
        assert_eq!(
            in_planet_shadow(aside, long),
            in_planet_shadow(aside, PLANET_LIGHT)
        );
        // Off to the side, behind the globe from the light but clear of its
        // shadow, whatever the light's length.
        let across = unit([l[1], -l[0], 0.0]);
        let clear = [0, 1, 2].map(|i| -0.5 * l[i] + 1.5 * across[i]);
        assert!(!in_planet_shadow(clear, long));
        assert!(!in_planet_shadow(clear, PLANET_LIGHT));
        let south = globe(0.0, 0.6).unwrap();
        let (short, far) = (
            ring_shadow(south, &f, PLANET_LIGHT),
            ring_shadow(south, &f, long),
        );
        assert_near(short.unwrap(), far.unwrap());
    }

    #[test]
    fn the_ring_plane_is_met_unless_the_ray_runs_along_it() {
        // At the threshold, still met.
        let barely = Frame {
            north: [0.0, -1.0, 1e-6],
            facing: [0.0, 0.0, 1.0],
            east: [1.0, 0.0, 0.0],
        };
        assert!(ring_hit(0.0, 0.5, &barely).is_some());
        // A light along the equator's plane never crosses it.
        let flat = Frame::new(0.0, 0.0);
        let sideways = Light {
            direction: [1.0, 0.0, 0.0],
            ambient: 0.1,
        };
        assert_eq!(ring_shadow(globe(0.0, 0.5).unwrap(), &flat, sideways), None);
        let grazing = Light {
            direction: [0.0, -1e-6, 1.0],
            ambient: 0.1,
        };
        let r = ring_shadow(globe(0.0, 0.5).unwrap(), &flat, grazing);
        assert!(r.is_some_and(|r| r > 1000.0), "{r:?}");
        // A point on the equator is in the plane: it crosses it nowhere
        // further along.
        let equator = [1.0, 0.0, 0.0];
        assert_eq!(ring_shadow(equator, &flat, PLANET_LIGHT), None);
    }

    #[test]
    fn the_night_side_is_darker_than_the_day_side() {
        // The light is toward up-left.
        let day = lit(globe(-0.6, -0.6).unwrap(), PLANET_LIGHT);
        let night = lit(globe(0.6, 0.6).unwrap(), PLANET_LIGHT);
        assert!(day > 0.9, "{day}");
        assert_eq!(night, PLANET_LIGHT.ambient);
        const { assert!(PLANET_LIGHT.ambient < Light::STANDARD.ambient) };
        assert_eq!(PLANET_LIGHT.direction, Light::STANDARD.direction);
    }

    #[test]
    fn time_turns_the_surface_by_days_and_the_clouds_faster() {
        assert_eq!(turns(0.0, 100.0), (0.0, 0.0));
        let (surface, clouds) = turns(25.0, 100.0);
        assert_near(surface, 0.25);
        assert_near(clouds, 0.3);
        let (surface, clouds) = turns(125.0, 100.0);
        assert_near(surface, 0.25);
        assert_near(clouds, 0.5);
        assert_eq!(turns(-25.0, 100.0).0, 0.75);
        for day in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(turns(5.0, day), (0.0, 0.0), "{day}");
        }
        assert_eq!(turns(f64::NAN, 10.0), (0.0, 0.0));
    }

    /// A 4 by 2 surface with rings and an atmosphere, a 300 s day and a
    /// small tilt.
    fn surface() -> Surface {
        Surface {
            surface: image(4, 2, vec![9; 32]),
            clouds: Some(image(4, 2, vec![255; 32])),
            rings: Some(RingBand {
                inner: 1.4,
                outer: 2.3,
                color: [255, 0, 51],
                profile: vec![0, 200, 100, 0],
            }),
            atmosphere: Some(Glow {
                color: [0, 255, 0],
                thickness: 0.25,
            }),
            day: 300.0,
            tilt: 0.2,
        }
    }

    #[test]
    fn a_planet_s_textures_are_its_layers_mip_chains_and_its_ring_row() {
        let made = textures(&surface());
        assert_eq!(made.surface, mips(&surface().surface));
        assert_eq!(made.clouds, mips(&image(4, 2, vec![255; 32])));
        assert_eq!(
            made.rings,
            image(
                4,
                1,
                [0, 200, 100, 0].iter().flat_map(|o| [*o; 4]).collect()
            )
        );
        let bare = Surface {
            clouds: None,
            rings: None,
            ..surface()
        };
        let made = textures(&bare);
        assert_eq!(made.clouds, mips(&image(2, 1, vec![0; 8])), "clear");
        assert_eq!(made.rings, image(2, 1, vec![0; 8]), "clear");
    }

    #[test]
    fn a_planet_s_instance_holds_what_the_core_works_out_for_it() {
        let pose = PlanetPose {
            center: Point::new(100.0, 80.0),
            radius: 10.0,
            time: 75.0,
        };
        let made = instance(PlanetId(3), &surface(), &pose).expect("drawable");
        assert_eq!(made.planet, PlanetId(3));
        assert_eq!(made.reach, 2.3);
        assert_near(made.dest.x, 77.0);
        assert_near(made.dest.y, 57.0);
        assert_near(made.dest.w, 46.0);
        assert_near(made.dest.h, 46.0);
        assert_eq!(made.frame, Frame::new(0.2, VIEW_LEAN));
        let (surface_turn, cloud_turn) = turns(75.0, 300.0);
        assert_eq!(made.turns, [surface_turn, cloud_turn]);
        assert_eq!(made.rings, [1.4, 2.3]);
        assert_eq!(made.ring_color, [1.0, 0.0, 0.2]);
        assert_eq!(made.atmosphere, [0.0, 1.0, 0.0, 0.25]);
        let bare = Surface {
            rings: None,
            atmosphere: None,
            ..surface()
        };
        let made = instance(PlanetId(3), &bare, &pose).unwrap();
        assert_eq!(
            (made.reach, made.rings, made.atmosphere),
            (1.0, [0.0; 2], [0.0; 4])
        );
        assert_eq!(made.ring_color, [0.0; 3]);
        let bad = PlanetPose {
            radius: 0.0,
            ..pose
        };
        assert_eq!(instance(PlanetId(3), &surface(), &bad), None);
    }

    fn image(width: u32, height: u32, rgba: Vec<u8>) -> Image {
        Image {
            width,
            height,
            rgba,
        }
    }

    #[test]
    fn a_mip_chain_halves_to_one_texel_averaging_each_four() {
        #[rustfmt::skip]
        let base = image(4, 2, vec![
            0, 0, 0, 0,    4, 8, 12, 16,  100, 0, 0, 255,  100, 0, 0, 255,
            0, 0, 0, 0,    4, 8, 12, 17,  100, 0, 0, 255,  100, 0, 0, 255,
        ]);
        let chain = mips(&base);
        let sizes: Vec<_> = chain.iter().map(|i| (i.width, i.height)).collect();
        assert_eq!(sizes, [(4, 2), (2, 1), (1, 1)]);
        assert_eq!(chain[0], base);
        // (0 + 4 + 0 + 4) / 4 and so on, rounded; alpha on its own.
        assert_eq!(chain[1].rgba, [2, 4, 6, 8, 100, 0, 0, 255]);
        // A row of two: the average of both.
        assert_eq!(chain[2].rgba, [51, 2, 3, 132]);
        assert_eq!(mips(&image(1, 1, vec![1, 2, 3, 4])).len(), 1);
        // A column halves only down, and a second row of output averages
        // its own rows.
        let tall = mips(&image(
            1,
            4,
            vec![
                10, 10, 10, 10, 20, 20, 20, 20, 30, 30, 30, 30, 40, 40, 40, 40,
            ],
        ));
        let sizes: Vec<_> = tall.iter().map(|i| (i.width, i.height)).collect();
        assert_eq!(sizes, [(1, 4), (1, 2), (1, 1)]);
        assert_eq!(tall[1].rgba, [15, 15, 15, 15, 35, 35, 35, 35]);
        let rows: Vec<u8> = (0..4_u8).flat_map(|y| [y * 10; 16]).collect();
        let square = mips(&image(4, 4, rows));
        assert_eq!(square[1].rgba[..4], [5; 4]);
        assert_eq!(
            square[1].rgba[8..12],
            [25; 4],
            "the second row from rows 2 and 3"
        );
        let chain = mips(&image(64, 32, vec![7; 64 * 32 * 4]));
        assert_eq!(chain.len(), 7);
        assert!(chain.iter().all(|i| i.rgba.iter().all(|c| *c == 7)));
    }
}
