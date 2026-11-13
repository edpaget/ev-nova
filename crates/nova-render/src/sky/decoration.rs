//! A sky's decorations ([`SkyDecoration`]): images drawn as far scenery
//! over the backdrop and the stars, such as a distant hulk or a far
//! galaxy, placed here so the adapter only draws a textured quad for
//! each ([`placed`]).
//!
//! # Where one sits
//!
//! A decoration lies on a layer of its own, which moves
//! [`SkyDecoration::parallax`] of the eye's move, the sky's measure
//! ([`super::parallax`]): 0.5 is the nearest star layer's, 0.05 the
//! farthest's, 0.025 the backdrop's. Its centre is drawn at `anchor + at
//! − eye · parallax`: where a stellar at [`SkyDecoration::at`] is drawn
//! when the eye is over the system's centre, and from there it moves its
//! parallax of the eye's move. The longer side of its image is drawn
//! [`SkyDecoration::size`] logical units long, whatever its parallax,
//! and the other side follows the image's shape. It is turned
//! [`SkyDecoration::turn`] degrees clockwise about its centre.
//!
//! # What it adds
//!
//! Each pixel of its quad samples its image ([`SkyImage`], premultiplied)
//! through the one sampler ([`SKY_SAMPLER`]) at the level of detail its
//! drawn size gives ([`Placement::lod`]). The texel's colour times the
//! decoration's colour (its tint at full brightness × its brightness ×
//! the pose's level / 32) is the light it adds, and the texel's alpha
//! times its cover (1 for [`SkyBlend::Over`], 0 for [`SkyBlend::Glow`])
//! is how much of what is beneath it hides ([`lit`], [`over`]):
//! `pixel = beneath · (1 − cover) + light`. So an `over` image hides the
//! sky behind it by its alpha, and a `glow` image only adds.
//!
//! Murk dims the light and never the cover: a hulk keeps hiding the
//! stars behind it as both dim. At level 0 nothing is drawn.
//!
//! # Order
//!
//! Decorations draw after the backdrop and every star, far to near by
//! parallax, ties in the list's order. So an `over` decoration hides the
//! backdrop, the stars and every decoration farther than it, and no star
//! is ever drawn over a decoration, whatever its parallax. A `glow`
//! decoration's light adds, which commutes with the stars and the puffs.
//!
//! # The warp
//!
//! A decoration doesn't streak through a jump's warp, as the backdrop
//! doesn't: the rush changes nothing here. It still moves with the eye
//! by its parallax.
//!
//! # Its bounds
//!
//! Whatever it is handed, [`placed`] draws at most [`MOST_DECORATIONS`]
//! of a list (its first), each at a size within [`DECORATION_SIZE`] and
//! a parallax within [`DECORATION_PARALLAX`]; a value that is not finite
//! takes its default. So a sky's decorations are at most 6 instances
//! whose quads shade at most 6 · 512² = 1,572,864 square units, 2.0
//! areas of a 1024×768 view and 0.57 of a 2560×1080 one, and never more
//! than 6 view areas ([`decoration_area_bound`]). A quad that doesn't
//! reach into the view is no instance.

use std::sync::Arc;

use nova_view::{Point, SkyPose};

use super::image::{SKY_SAMPLER, SkyImage};
use super::look::normalised;
use super::{Draws, LAYERS};
use crate::model_sampler::sample;
use crate::viewport::LogicalSize;

/// The most decorations of a list that are drawn: its first.
pub const MOST_DECORATIONS: usize = 6;

/// The least and most parallax a decoration is drawn at: half the
/// backdrop's, and the nearest star layer's.
pub const DECORATION_PARALLAX: (f32, f32) = (0.0125, 0.5);

/// The parallax of a decoration that sets none: the farthest star
/// layer's.
pub const DEFAULT_PARALLAX: f32 = 0.05;

/// The least and most length a decoration's longer side is drawn at, in
/// logical units.
pub const DECORATION_SIZE: (f32, f32) = (8.0, 512.0);

/// The size of a decoration that sets none, in logical units.
pub const DEFAULT_SIZE: f32 = 128.0;

/// How far across and down from its layer's centre a seeded place lies
/// at most ([`seeded_place`]), in logical units: within a 1024×768 view
/// from the system's centre.
pub const SEEDED_REACH: (f32, f32) = (384.0, 288.0);

/// How a decoration is laid on the sky beneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkyBlend {
    /// Painted over the sky by its alpha.
    #[default]
    Over,
    /// Its light added to the sky.
    Glow,
}

/// One decoration of a sky.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyDecoration {
    /// Its image.
    pub image: Arc<SkyImage>,
    /// Where it sits on its layer, in logical units from the layer's
    /// centre, on a stellar position's axes. A place that is not finite
    /// is the seed's ([`seeded_place`]).
    pub at: Point,
    /// The share of the eye's move it moves by, within
    /// [`DECORATION_PARALLAX`].
    pub parallax: f32,
    /// The drawn length of its image's longer side, in logical units,
    /// within [`DECORATION_SIZE`].
    pub size: f32,
    /// Its heading, in degrees clockwise as drawn.
    pub turn: f32,
    /// The colour its image is multiplied by, RGB 0 to 1, taken at full
    /// brightness; `None`, or black, for the image's own colours.
    pub tint: Option<[f32; 3]>,
    /// How bright it is, 0 to 1.
    pub brightness: f32,
    /// How it is laid on the sky.
    pub blend: SkyBlend,
}

impl SkyDecoration {
    /// `image` at `at` with every default: [`DEFAULT_PARALLAX`],
    /// [`DEFAULT_SIZE`], no turn, no tint, full brightness, painted over.
    #[must_use]
    pub fn new(image: Arc<SkyImage>, at: Point) -> Self {
        Self {
            image,
            at,
            parallax: DEFAULT_PARALLAX,
            size: DEFAULT_SIZE,
            turn: 0.0,
            tint: None,
            brightness: 1.0,
            blend: SkyBlend::Over,
        }
    }
}

/// Where a decoration is drawn and what it adds: everything the adapter
/// needs for its quad.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// Its centre, in logical units.
    pub centre: Point,
    /// From its centre to the middle of its image's right edge, in
    /// logical units: its turn is already in it.
    pub right: Point,
    /// From its centre to the middle of its image's bottom edge.
    pub down: Point,
    /// What its texels' colours are multiplied by, RGB 0 to 1.
    pub color: [f32; 3],
    /// What its texels' alpha is multiplied by to give how much of the
    /// sky beneath it hides: 1 painted over, 0 for a glow.
    pub cover: f32,
    /// The level of detail its image is sampled at: the base-2 logarithm
    /// of its texels to a texel of the canvas it is drawn into.
    pub lod: f32,
}

/// The place sky `seed` gives its decoration `index` when its data sets
/// none: within [`SEEDED_REACH`] of its layer's centre.
#[must_use]
pub fn seeded_place(seed: u64, index: usize) -> Point {
    // Salted past the star layers and the backdrop's two salts, so no
    // star or cloud changes.
    let mut draws = Draws::new(seed, LAYERS.len() + 2, index as i64, 0);
    let x = (2.0 * draws.unit() - 1.0) * SEEDED_REACH.0;
    let y = (2.0 * draws.unit() - 1.0) * SEEDED_REACH.1;
    Point::new(x, y)
}

/// `value` held to `range`, or `default` when it is not finite.
fn held(value: f32, (least, most): (f32, f32), default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(least, most)
    } else {
        default
    }
}

/// The parallax `decoration` is drawn at: its own, held to its range.
fn parallax_of(decoration: &SkyDecoration) -> f32 {
    held(decoration.parallax, DECORATION_PARALLAX, DEFAULT_PARALLAX)
}

/// Where `decoration`, the `index`th of sky `pose.seed`'s list, is drawn
/// as `pose` sees it, into a canvas of `texels_per_unit` texels per
/// logical unit, whether or not it reaches into a view. `None` at level
/// 0, or with an eye or anchor that is not finite.
///
/// A size or a parallax outside its range is taken to its nearer end,
/// and a brightness into 0 to 1; a size, parallax, turn or brightness
/// that is not finite takes its default, and a place that is not finite
/// the seed's. A `texels_per_unit` that is not a positive, finite number
/// samples the whole image (level 0).
#[must_use]
pub fn placement(
    pose: &SkyPose,
    decoration: &SkyDecoration,
    index: usize,
    texels_per_unit: f32,
) -> Option<Placement> {
    let finite = |point: Point| point.x.is_finite() && point.y.is_finite();
    if pose.level == 0 || !(finite(pose.eye) && finite(pose.anchor)) {
        return None;
    }
    let at = if finite(decoration.at) {
        decoration.at
    } else {
        seeded_place(pose.seed, index)
    };
    let parallax = f64::from(parallax_of(decoration));
    // Worked in `f64`, as the stars' corner is, so a far eye costs no
    // precision.
    let centre = |anchor: f32, at: f32, eye: f32| {
        (f64::from(anchor) + f64::from(at) - f64::from(eye) * parallax) as f32
    };
    let centre = Point::new(
        centre(pose.anchor.x, at.x, pose.eye.x),
        centre(pose.anchor.y, at.y, pose.eye.y),
    );
    let size = held(decoration.size, DECORATION_SIZE, DEFAULT_SIZE);
    let (width, height) = (
        decoration.image.width() as f32,
        decoration.image.height() as f32,
    );
    let longer = width.max(height);
    // Half its drawn width and height.
    let (across, down) = (size * width / longer / 2.0, size * height / longer / 2.0);
    let turn = if decoration.turn.is_finite() {
        f64::from(decoration.turn)
    } else {
        0.0
    };
    let (sin, cos) = turn.to_radians().sin_cos();
    let (sin, cos) = (sin as f32, cos as f32);
    let brightness = held(decoration.brightness, (0.0, 1.0), 1.0);
    let level = f32::from(pose.level.min(32)) / 32.0;
    let tint = decoration.tint.and_then(normalised).unwrap_or([1.0; 3]);
    let light = brightness * level;
    let lod = if texels_per_unit.is_finite() && texels_per_unit > 0.0 {
        (longer / (size * texels_per_unit)).log2()
    } else {
        0.0
    };
    Some(Placement {
        centre,
        right: Point::new(across * cos, across * sin),
        down: Point::new(-down * sin, down * cos),
        color: tint.map(|c| c * light),
        cover: match decoration.blend {
            SkyBlend::Over => 1.0,
            SkyBlend::Glow => 0.0,
        },
        lod,
    })
}

/// The decorations of `decorations` that `pose` sees in a view of
/// `logical` size, each with its index in the list, in draw order: far
/// to near by parallax, ties in the list's order. Only the first
/// [`MOST_DECORATIONS`] of the list are drawn, and one whose turned quad
/// doesn't reach into the view is left out. None at level 0, or with an
/// eye or anchor that is not finite. The rush changes nothing.
#[must_use]
pub fn placed(
    pose: &SkyPose,
    decorations: &[SkyDecoration],
    logical: LogicalSize,
    texels_per_unit: f32,
) -> Vec<(usize, Placement)> {
    let (width, height) = (logical.w as f32, logical.h as f32);
    let mut drawn: Vec<(usize, f32, Placement)> = decorations
        .iter()
        .take(MOST_DECORATIONS)
        .enumerate()
        .filter_map(|(index, decoration)| {
            let placement = placement(pose, decoration, index, texels_per_unit)?;
            // The box its turned quad lies in reaches this far each way
            // from its centre.
            let Placement {
                centre,
                right,
                down,
                ..
            } = placement;
            let (reach_x, reach_y) = (right.x.abs() + down.x.abs(), right.y.abs() + down.y.abs());
            let reaches = centre.x + reach_x > 0.0
                && centre.x - reach_x < width
                && centre.y + reach_y > 0.0
                && centre.y - reach_y < height;
            reaches.then(|| (index, parallax_of(decoration), placement))
        })
        .collect();
    // A stable sort: ties keep the list's order.
    drawn.sort_by(|a, b| a.1.total_cmp(&b.1));
    drawn
        .into_iter()
        .map(|(index, _, placement)| (index, placement))
        .collect()
}

/// The most area, in square logical units, that a sky's decorations'
/// quads shade inside a view of `logical` size, overlaps counted each
/// time: [`MOST_DECORATIONS`] squares of the largest size, or as many
/// view areas.
#[must_use]
pub fn decoration_area_bound(logical: LogicalSize) -> f32 {
    let most = MOST_DECORATIONS as f32;
    let square = DECORATION_SIZE.1 * DECORATION_SIZE.1;
    let view = logical.w as f32 * logical.h as f32;
    (most * square).min(most * view)
}

/// What a decoration drawn at `placement` adds at the logical point
/// `at`: its light, RGB, then its cover. `None` outside its quad.
#[must_use]
pub fn lit(image: &SkyImage, placement: &Placement, at: Point) -> Option<[f32; 4]> {
    let (dx, dy) = (at.x - placement.centre.x, at.y - placement.centre.y);
    // How far along an edge vector the point lies, −1 to 1 across the
    // quad; not a number for a quad with no size, which holds no point.
    let along = |edge: Point| (dx * edge.x + dy * edge.y) / (edge.x * edge.x + edge.y * edge.y);
    let uv = [
        0.5 + 0.5 * along(placement.right),
        0.5 + 0.5 * along(placement.down),
    ];
    if !uv.iter().all(|c| (0.0..=1.0).contains(c)) {
        return None;
    }
    let texel = sample(image.texture(), SKY_SAMPLER, uv, placement.lod);
    let color = placement.color;
    Some([
        texel[0] * color[0],
        texel[1] * color[1],
        texel[2] * color[2],
        texel[3] * placement.cover,
    ])
}

/// The pixel a decoration's `lit` (its light, then its cover, as [`lit`]
/// gives them) leaves over `beneath`: `beneath · (1 − cover) + light`.
#[must_use]
pub fn over(beneath: [f32; 3], lit: [f32; 4]) -> [f32; 3] {
    let keep = 1.0 - lit[3];
    [0, 1, 2].map(|c| beneath[c] * keep + lit[c])
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::backdrop::star_bound;
    use super::super::{
        BackdropLook, SkyLook, backdrop_area_bound, backdrop_instance_bound, cell_bound, instances,
    };
    use super::*;
    use crate::viewport::LOGICAL;

    const WIDE: LogicalSize = LogicalSize { w: 2560, h: 1080 };
    const ANCHOR: Point = Point::new(512.0, 384.0);

    fn pose(seed: u64, eye: (f32, f32)) -> SkyPose {
        SkyPose::new(seed, Point::new(eye.0, eye.1), ANCHOR, 32)
    }

    /// An opaque image `width` by `height`, each texel its own shade.
    fn image(width: u32, height: u32) -> Arc<SkyImage> {
        let rgba = (0..width * height)
            .flat_map(|at| {
                let shade = (at * 37 % 200) as u8 + 40;
                [shade, 255 - shade, shade / 2, 255]
            })
            .collect();
        Arc::new(SkyImage::new(width, height, rgba).expect("whole rows"))
    }

    /// A square decoration at `at` with every default.
    fn at(x: f32, y: f32) -> SkyDecoration {
        SkyDecoration::new(image(2, 2), Point::new(x, y))
    }

    fn one(pose: &SkyPose, decoration: &SkyDecoration) -> Placement {
        placement(pose, decoration, 0, 1.0).expect("a placement")
    }

    #[track_caller]
    fn assert_near(got: Point, expected: (f32, f32)) {
        assert!(
            (got.x - expected.0).abs() < 1e-3 && (got.y - expected.1).abs() < 1e-3,
            "{got:?}, not {expected:?}"
        );
    }

    #[test]
    fn a_new_decoration_has_every_default() {
        let picture = image(2, 2);
        let decoration = SkyDecoration::new(Arc::clone(&picture), Point::new(3.0, -4.0));
        assert!(Arc::ptr_eq(&decoration.image, &picture));
        assert_eq!(decoration.at, Point::new(3.0, -4.0));
        assert_eq!(decoration.parallax, 0.05);
        assert_eq!(decoration.size, 128.0);
        assert_eq!(decoration.turn, 0.0);
        assert_eq!(decoration.tint, None);
        assert_eq!(decoration.brightness, 1.0);
        assert_eq!(decoration.blend, SkyBlend::Over);
        assert_eq!(SkyBlend::default(), SkyBlend::Over);
        assert_eq!((DEFAULT_PARALLAX, DEFAULT_SIZE), (0.05, 128.0));
        assert_eq!(DECORATION_PARALLAX, (0.0125, 0.5));
        assert_eq!(DECORATION_SIZE, (8.0, 512.0));
        assert_eq!(MOST_DECORATIONS, 6);
    }

    #[test]
    fn its_centre_is_the_anchor_plus_its_place_less_the_eyes_move_by_its_parallax() {
        let decoration = SkyDecoration {
            parallax: 0.2,
            ..at(120.0, -80.0)
        };
        // From over the system's centre: where a stellar there is drawn.
        assert_eq!(
            one(&pose(1, (0.0, 0.0)), &decoration).centre,
            Point::new(632.0, 304.0)
        );
        // The eye moved (1000, −500): it moves a fifth of that, back.
        assert_eq!(
            one(&pose(1, (1000.0, -500.0)), &decoration).centre,
            Point::new(432.0, 404.0)
        );
        // Another anchor moves it whole.
        let moved = SkyPose::new(1, Point::new(0.0, 0.0), Point::new(100.0, 50.0), 32);
        assert_eq!(one(&moved, &decoration).centre, Point::new(220.0, -30.0));
    }

    #[test]
    fn a_far_eye_places_it_as_exactly_as_the_stars() {
        let decoration = SkyDecoration {
            parallax: 0.05,
            ..at(120.0, -80.0)
        };
        let eye = (3.000_001e6_f32, -1.000_003e6_f32);
        let parallax = f64::from(0.05_f32);
        let x = 512.0 + 120.0 - f64::from(eye.0) * parallax;
        let y = 384.0 - 80.0 - f64::from(eye.1) * parallax;
        assert_eq!(
            one(&pose(1, eye), &decoration).centre,
            Point::new(x as f32, y as f32)
        );
    }

    #[test]
    fn its_longer_side_is_its_size_and_the_other_follows_the_images_shape() {
        let wide = SkyDecoration {
            size: 100.0,
            ..SkyDecoration::new(image(4, 2), Point::new(0.0, 0.0))
        };
        let placed = one(&pose(1, (0.0, 0.0)), &wide);
        assert_eq!(placed.right, Point::new(50.0, 0.0));
        assert_eq!(placed.down, Point::new(0.0, 25.0));
        let tall = SkyDecoration {
            size: 64.0,
            ..SkyDecoration::new(image(2, 8), Point::new(0.0, 0.0))
        };
        let placed = one(&pose(1, (0.0, 0.0)), &tall);
        assert_eq!(placed.right, Point::new(8.0, 0.0));
        assert_eq!(placed.down, Point::new(0.0, 32.0));
        let square = SkyDecoration {
            size: 10.0,
            ..at(0.0, 0.0)
        };
        let placed = one(&pose(1, (0.0, 0.0)), &square);
        assert_eq!((placed.right.x, placed.down.y), (5.0, 5.0));
    }

    #[test]
    fn a_turn_is_clockwise_as_drawn_and_already_in_its_edges() {
        let turned = |turn: f32| {
            let decoration = SkyDecoration {
                size: 100.0,
                turn,
                ..SkyDecoration::new(image(4, 2), Point::new(0.0, 0.0))
            };
            one(&pose(1, (0.0, 0.0)), &decoration)
        };
        // A quarter turn clockwise: its right edge points down the
        // screen, and its bottom edge to the left.
        let quarter = turned(90.0);
        assert_near(quarter.right, (0.0, 50.0));
        assert_near(quarter.down, (-25.0, 0.0));
        let back = turned(-90.0);
        assert_near(back.right, (0.0, -50.0));
        assert_near(back.down, (25.0, 0.0));
        let half = turned(180.0);
        assert_near(half.right, (-50.0, 0.0));
        assert_near(half.down, (0.0, -25.0));
        // 30 degrees: cos 0.8660254, sin 0.5.
        let some = turned(30.0);
        assert_near(some.right, (43.301_27, 25.0));
        assert_near(some.down, (-12.5, 21.650_635));
        // No turn is exact.
        assert_eq!(turned(0.0).right, Point::new(50.0, 0.0));
        assert_eq!(turned(0.0).down, Point::new(0.0, 25.0));
        // A whole turn is none.
        assert_near(turned(360.0).right, (50.0, 0.0));
        assert_near(turned(-360.0).down, (0.0, 25.0));
    }

    #[test]
    fn its_colour_is_its_bright_tint_times_its_brightness_times_the_level() {
        let seen = pose(1, (0.0, 0.0));
        let plain = at(0.0, 0.0);
        assert_eq!(one(&seen, &plain).color, [1.0; 3]);
        let dimmed = SkyDecoration {
            brightness: 0.5,
            ..plain.clone()
        };
        assert_eq!(one(&seen, &dimmed).color, [0.5; 3]);
        // A dark tint is taken at full brightness: (1, 0.5, 0.25).
        let tinted = SkyDecoration {
            tint: Some([0.5, 0.25, 0.125]),
            brightness: 0.5,
            ..plain.clone()
        };
        assert_eq!(one(&seen, &tinted).color, [0.5, 0.25, 0.125]);
        // Black is no tint.
        let black = SkyDecoration {
            tint: Some([0.0; 3]),
            brightness: 0.75,
            ..plain.clone()
        };
        assert_eq!(one(&seen, &black).color, [0.75; 3]);
        // Murk halves the light at level 16, and the cover stays.
        let murky = SkyPose::new(1, Point::new(0.0, 0.0), ANCHOR, 16);
        let half = one(&murky, &tinted);
        assert_eq!(half.color, [0.25, 0.125, 0.0625]);
        assert_eq!(half.cover, 1.0);
        // A level past 32 is 32.
        let bright = SkyPose::new(1, Point::new(0.0, 0.0), ANCHOR, 200);
        assert_eq!(one(&bright, &plain).color, [1.0; 3]);
    }

    #[test]
    fn painted_over_covers_and_a_glow_does_not() {
        let seen = pose(1, (0.0, 0.0));
        assert_eq!(one(&seen, &at(0.0, 0.0)).cover, 1.0);
        let glow = SkyDecoration {
            blend: SkyBlend::Glow,
            ..at(0.0, 0.0)
        };
        assert_eq!(one(&seen, &glow).cover, 0.0);
    }

    #[test]
    fn its_level_of_detail_is_the_log_of_its_texels_to_a_canvas_texel() {
        let seen = pose(1, (0.0, 0.0));
        let sized = |side: u32, size: f32, texels: f32| {
            let decoration = SkyDecoration {
                size,
                ..SkyDecoration::new(image(side, side / 2), Point::new(0.0, 0.0))
            };
            placement(&seen, &decoration, 0, texels)
                .expect("a placement")
                .lod
        };
        let near = |got: f32, expected: f32| (got - expected).abs() < 1e-5;
        // One texel a canvas texel.
        assert!(near(sized(64, 64.0, 1.0), 0.0));
        // Twice and four times the texels: levels 1 and 2.
        assert!(near(sized(64, 32.0, 1.0), 1.0));
        assert!(near(sized(64, 16.0, 1.0), 2.0));
        // The lowest render scale: a quarter of a texel a unit.
        assert!(near(sized(128, 128.0, 0.25), 2.0));
        // Magnified: below 0.
        assert!(near(sized(64, 128.0, 1.0), -1.0));
        assert!(near(sized(64, 64.0, 2.0), -1.0));
        // A tall image is measured along its longer side too.
        let tall = SkyDecoration {
            size: 16.0,
            ..SkyDecoration::new(image(2, 32), Point::new(0.0, 0.0))
        };
        assert!(near(placement(&seen, &tall, 0, 1.0).unwrap().lod, 1.0));
        // No canvas scale to go by: the whole image.
        for odd in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(sized(64, 16.0, odd), 0.0, "{odd}");
        }
    }

    /// The indexes [`placed`] gives, in its order.
    fn order(pose: &SkyPose, list: &[SkyDecoration], logical: LogicalSize) -> Vec<usize> {
        placed(pose, list, logical, 1.0)
            .iter()
            .map(|(index, _)| *index)
            .collect()
    }

    #[test]
    fn they_are_drawn_far_to_near_by_parallax_and_ties_in_list_order() {
        let list: Vec<SkyDecoration> = [0.3, 0.05, 0.3, 0.0125, 0.05]
            .iter()
            .map(|&parallax| SkyDecoration {
                parallax,
                ..at(0.0, 0.0)
            })
            .collect();
        let seen = pose(1, (0.0, 0.0));
        assert_eq!(order(&seen, &list, LOGICAL), [3, 1, 4, 0, 2]);
        // Each is the placement its own decoration gives.
        for (index, placement_of) in placed(&seen, &list, LOGICAL, 1.0) {
            assert_eq!(
                Some(placement_of),
                placement(&seen, &list[index], index, 1.0)
            );
        }
    }

    #[test]
    fn a_quad_wholly_off_the_view_is_dropped_on_every_side() {
        // A square 100 on a side: with the eye at the centre and the
        // anchor at (512, 384), a place of (x, y) is drawn at (512 + x,
        // 384 + y).
        let seen = pose(1, (0.0, 0.0));
        let square = |x: f32, y: f32, turn: f32| SkyDecoration {
            size: 100.0,
            turn,
            ..at(x - 512.0, y - 384.0)
        };
        let kept = |x: f32, y: f32, turn: f32| order(&seen, &[square(x, y, turn)], LOGICAL).len();
        // Inside.
        assert_eq!(kept(500.0, 300.0, 0.0), 1);
        // Just reaching in from each side, and just out of it. Touching
        // the edge is out.
        for (inside, touching) in [
            ((-49.0, 300.0), (-50.0, 300.0)),
            ((1073.0, 300.0), (1074.0, 300.0)),
            ((500.0, -49.0), (500.0, -50.0)),
            ((500.0, 817.0), (500.0, 818.0)),
        ] {
            assert_eq!(kept(inside.0, inside.1, 0.0), 1, "{inside:?}");
            assert_eq!(kept(touching.0, touching.1, 0.0), 0, "{touching:?}");
        }
        // Past the right edge square-on, but its corner reaches in when
        // it is turned an eighth: half its diagonal is 70.7.
        assert_eq!(kept(1084.0, 300.0, 0.0), 0);
        assert_eq!(kept(1084.0, 300.0, 45.0), 1);
        assert_eq!(kept(500.0, -60.0, 0.0), 0);
        assert_eq!(kept(500.0, -60.0, 45.0), 1);
        assert_eq!(kept(-60.0, 300.0, 45.0), 1);
        assert_eq!(kept(500.0, 828.0, 45.0), 1);
        assert_eq!(kept(1100.0, 300.0, 45.0), 0);
        // A wide view keeps what a narrow one drops.
        assert_eq!(order(&seen, &[square(1500.0, 300.0, 0.0)], WIDE).len(), 1);
        assert_eq!(order(&seen, &[square(500.0, 1000.0, 0.0)], WIDE).len(), 1);
        assert_eq!(order(&seen, &[square(500.0, 1131.0, 0.0)], WIDE).len(), 0);
        // A dropped decoration leaves the others their indexes.
        let list = [square(5000.0, 0.0, 0.0), square(500.0, 300.0, 0.0)];
        assert_eq!(order(&seen, &list, LOGICAL), [1]);
    }

    #[test]
    fn the_rush_changes_nothing() {
        let list = [
            at(0.0, 0.0),
            SkyDecoration {
                parallax: 0.5,
                ..at(40.0, 9.0)
            },
        ];
        let still = pose(4, (300.0, -200.0));
        let expected = placed(&still, &list, LOGICAL, 0.5);
        assert_eq!(expected.len(), 2);
        for rush in [(30.0, 18.0), (-1e6, 3e5), (f32::NAN, 0.0)] {
            let rushing = still.with_rush(Point::new(rush.0, rush.1));
            assert_eq!(placed(&rushing, &list, LOGICAL, 0.5), expected, "{rush:?}");
        }
    }

    #[test]
    fn level_zero_or_no_finite_point_draws_none() {
        let list = [at(0.0, 0.0)];
        let dark = SkyPose::new(1, Point::new(0.0, 0.0), ANCHOR, 0);
        assert!(placed(&dark, &list, LOGICAL, 1.0).is_empty());
        assert_eq!(placement(&dark, &list[0], 0, 1.0), None);
        for odd in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for seen in [
                SkyPose::new(1, Point::new(odd, 0.0), ANCHOR, 32),
                SkyPose::new(1, Point::new(0.0, odd), ANCHOR, 32),
                SkyPose::new(1, Point::new(0.0, 0.0), Point::new(odd, 0.0), 32),
                SkyPose::new(1, Point::new(0.0, 0.0), Point::new(0.0, odd), 32),
            ] {
                assert!(placed(&seen, &list, LOGICAL, 1.0).is_empty(), "{seen:?}");
                assert_eq!(placement(&seen, &list[0], 0, 1.0), None, "{seen:?}");
            }
        }
        assert_eq!(placed(&pose(1, (0.0, 0.0)), &list, LOGICAL, 1.0).len(), 1);
    }

    #[test]
    fn only_the_first_six_of_a_list_are_drawn() {
        let list: Vec<SkyDecoration> = (0..9).map(|_| at(0.0, 0.0)).collect();
        assert_eq!(
            order(&pose(1, (0.0, 0.0)), &list, LOGICAL),
            [0, 1, 2, 3, 4, 5]
        );
        // The cut is by the list, before any is dropped off the view.
        let mut list = list;
        list[0].at = Point::new(9000.0, 0.0);
        assert_eq!(order(&pose(1, (0.0, 0.0)), &list, LOGICAL), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_value_out_of_range_is_taken_to_its_end_and_one_not_finite_to_its_default() {
        let seen = pose(1, (100.0, 0.0));
        let with = |edit: &dyn Fn(&mut SkyDecoration)| {
            let mut decoration = at(0.0, 0.0);
            edit(&mut decoration);
            one(&seen, &decoration)
        };
        // Sizes: the square's half side.
        assert_eq!(with(&|d| d.size = 4096.0).right.x, 256.0);
        assert_eq!(with(&|d| d.size = 1.0).right.x, 4.0);
        assert_eq!(with(&|d| d.size = -3.0).right.x, 4.0);
        assert_eq!(with(&|d| d.size = 512.0).right.x, 256.0);
        assert_eq!(with(&|d| d.size = 8.0).right.x, 4.0);
        for odd in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(with(&|d| d.size = odd).right.x, 64.0, "{odd}");
        }
        // Parallaxes: the eye is 100 across, so the centre is 512 less
        // 100 times it.
        assert_eq!(with(&|d| d.parallax = 3.0).centre.x, 462.0);
        assert_eq!(with(&|d| d.parallax = 0.5).centre.x, 462.0);
        assert_eq!(with(&|d| d.parallax = -1.0).centre.x, 510.75);
        assert_eq!(with(&|d| d.parallax = 0.0).centre.x, 510.75);
        for odd in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(with(&|d| d.parallax = odd).centre.x, 507.0, "{odd}");
        }
        // A turn that is not finite is none.
        for odd in [f32::NAN, f32::INFINITY] {
            assert_eq!(with(&|d| d.turn = odd).right, Point::new(64.0, 0.0));
        }
        // Brightness.
        assert_eq!(with(&|d| d.brightness = 7.0).color, [1.0; 3]);
        assert_eq!(with(&|d| d.brightness = -1.0).color, [0.0; 3]);
        assert_eq!(with(&|d| d.brightness = f32::NAN).color, [1.0; 3]);
        // A place that is not finite is the seed's, for its index.
        for odd in [Point::new(f32::NAN, 0.0), Point::new(0.0, f32::INFINITY)] {
            let mut decoration = at(0.0, 0.0);
            decoration.at = odd;
            let seeded = seeded_place(seen.seed, 3);
            let expected = SkyDecoration {
                at: seeded,
                ..decoration.clone()
            };
            assert_eq!(
                placement(&seen, &decoration, 3, 1.0),
                placement(&seen, &expected, 3, 1.0)
            );
        }
    }

    #[test]
    fn a_seeded_place_is_its_seeds_and_indexs_own_within_its_reach() {
        assert_eq!(SEEDED_REACH, (384.0, 288.0));
        assert_eq!(seeded_place(7, 0), seeded_place(7, 0));
        assert_ne!(seeded_place(7, 0), seeded_place(8, 0));
        assert_ne!(seeded_place(7, 0), seeded_place(7, 1));
        let places: Vec<Point> = (0..200)
            .flat_map(|seed| (0..6).map(move |index| seeded_place(seed, index)))
            .collect();
        for place in &places {
            assert!(
                place.x.abs() <= 384.0 && place.y.abs() <= 288.0,
                "{place:?}"
            );
        }
        // It uses its reach, both ways on both axes.
        let most = |f: &dyn Fn(&Point) -> f32| places.iter().map(f).fold(f32::MIN, f32::max);
        assert!(most(&|p| p.x) > 360.0 && most(&|p| -p.x) > 360.0);
        assert!(most(&|p| p.y) > 270.0 && most(&|p| -p.y) > 270.0);
        // Across is not down.
        assert!(places.iter().any(|p| p.x.abs() > 288.0));
        // No star or cloud draw is taken: the first star cell's, the
        // seed's backdrop's and the first cloud cell's first draws differ
        // from it.
        let place = seeded_place(7, 0);
        for salt in 0..=LAYERS.len() + 1 {
            let mut draws = Draws::new(7, salt, 0, 0);
            assert_ne!((2.0 * draws.unit() - 1.0) * 384.0, place.x, "{salt}");
        }
    }

    /// A 2 by 2 image: red, green, blue and half-clear white.
    fn four() -> SkyImage {
        let rgba = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 255, 128],
        ]
        .concat();
        SkyImage::new(2, 2, rgba).expect("2 by 2")
    }

    /// A placement of an image centred at (10, 20), `half` its half
    /// side, unturned, at level of detail `lod`.
    fn square_at(half: f32, lod: f32) -> Placement {
        Placement {
            centre: Point::new(10.0, 20.0),
            right: Point::new(half, 0.0),
            down: Point::new(0.0, half),
            color: [1.0, 0.5, 0.25],
            cover: 1.0,
            lod,
        }
    }

    #[track_caller]
    fn assert_lit(got: Option<[f32; 4]>, expected: [f32; 4]) {
        let got = got.expect("inside the quad");
        assert!(
            got.iter().zip(&expected).all(|(a, b)| (a - b).abs() < 1e-5),
            "{got:?}, not {expected:?}"
        );
    }

    #[test]
    fn lit_is_the_texels_light_and_cover_inside_the_quad_and_none_outside() {
        let image = four();
        // One texel a unit: the quad spans (9, 19) to (11, 21), and each
        // texel's centre is half a unit in.
        let placement = square_at(1.0, 0.0);
        // Red times the colour.
        assert_lit(
            lit(&image, &placement, Point::new(9.5, 19.5)),
            [1.0, 0.0, 0.0, 1.0],
        );
        // Green.
        assert_lit(
            lit(&image, &placement, Point::new(10.5, 19.5)),
            [0.0, 0.5, 0.0, 1.0],
        );
        // Blue.
        assert_lit(
            lit(&image, &placement, Point::new(9.5, 20.5)),
            [0.0, 0.0, 0.25, 1.0],
        );
        // Half-clear white, premultiplied: 128 / 255 of each, and of the
        // cover.
        let half = 128.0 / 255.0;
        assert_lit(
            lit(&image, &placement, Point::new(10.5, 20.5)),
            [half, half * 0.5, half * 0.25, half],
        );
        // A glow keeps its light and covers nothing.
        let glow = Placement {
            cover: 0.0,
            ..placement
        };
        assert_lit(
            lit(&image, &glow, Point::new(10.5, 20.5)),
            [half, half * 0.5, half * 0.25, 0.0],
        );
        // Outside on each side.
        for outside in [(8.9, 20.0), (11.1, 20.0), (10.0, 18.9), (10.0, 21.1)] {
            let point = Point::new(outside.0, outside.1);
            assert_eq!(lit(&image, &placement, point), None, "{outside:?}");
        }
        // Its edge is inside.
        assert!(lit(&image, &placement, Point::new(9.0, 19.0)).is_some());
        assert!(lit(&image, &placement, Point::new(11.0, 21.0)).is_some());
    }

    #[test]
    fn lit_follows_the_quads_turn() {
        let image = four();
        // A quarter turn clockwise: the image's right edge points down
        // the screen, so its top-right texel (green) is drawn bottom
        // right, and its top-left (red) top right.
        let turned = Placement {
            right: Point::new(0.0, 1.0),
            down: Point::new(-1.0, 0.0),
            color: [1.0; 3],
            ..square_at(1.0, 0.0)
        };
        assert_lit(
            lit(&image, &turned, Point::new(10.5, 19.5)),
            [1.0, 0.0, 0.0, 1.0],
        );
        assert_lit(
            lit(&image, &turned, Point::new(10.5, 20.5)),
            [0.0, 1.0, 0.0, 1.0],
        );
        assert_lit(
            lit(&image, &turned, Point::new(9.5, 19.5)),
            [0.0, 0.0, 1.0, 1.0],
        );
        // A quad that is not square: 4 wide and 2 tall.
        let wide = Placement {
            right: Point::new(2.0, 0.0),
            down: Point::new(0.0, 1.0),
            color: [1.0; 3],
            ..square_at(1.0, 0.0)
        };
        assert_lit(
            lit(&image, &wide, Point::new(11.0, 19.5)),
            [0.0, 1.0, 0.0, 1.0],
        );
        assert_eq!(lit(&image, &wide, Point::new(10.0, 21.5)), None);
        assert!(lit(&image, &wide, Point::new(11.9, 20.0)).is_some());
        // A quad with no size lights nothing.
        let flat = Placement {
            right: Point::new(0.0, 0.0),
            ..square_at(1.0, 0.0)
        };
        assert_eq!(lit(&image, &flat, Point::new(10.0, 20.0)), None);
    }

    #[test]
    fn lit_reads_the_mips_average_when_the_image_is_drawn_smaller() {
        // 4 by 4: each 2 by 2 block one shade, so level 1 holds the four
        // shades and level 2 their average.
        let shades = [40_u8, 80, 120, 200];
        let mut rgba = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                let shade = shades[(row / 2) * 2 + column / 2];
                rgba.extend([shade, shade, shade, 255]);
            }
        }
        let image = SkyImage::new(4, 4, rgba).expect("4 by 4");
        let white = |half: f32, lod: f32| Placement {
            color: [1.0; 3],
            ..square_at(half, lod)
        };
        // Drawn 2 units across, twice the texels a unit: level 1, whose
        // texel centres are half a unit in.
        let half = white(1.0, 1.0);
        for (at, shade) in [
            ((9.5, 19.5), 40.0),
            ((10.5, 19.5), 80.0),
            ((9.5, 20.5), 120.0),
            ((10.5, 20.5), 200.0),
        ] {
            let value = shade / 255.0;
            assert_lit(
                lit(&image, &half, Point::new(at.0, at.1)),
                [value, value, value, 1.0],
            );
        }
        // Drawn 1 unit across, four times: level 2, the one average.
        let quarter = white(0.5, 2.0);
        let average = 110.0 / 255.0;
        assert_lit(
            lit(&image, &quarter, Point::new(10.0, 20.0)),
            [average, average, average, 1.0],
        );
        // Between the two levels, their blend.
        let between = white(1.0, 1.5);
        let blend = 75.0 / 255.0;
        assert_lit(
            lit(&image, &between, Point::new(9.5, 19.5)),
            [blend, blend, blend, 1.0],
        );
        // Drawn larger, the linear blend of the first level: midway
        // between the two top blocks.
        let large = white(4.0, -1.0);
        let mid = 60.0 / 255.0;
        assert_lit(
            lit(&image, &large, Point::new(10.0, 17.0)),
            [mid, mid, mid, 1.0],
        );
    }

    #[test]
    fn over_is_what_is_beneath_less_its_cover_plus_its_light() {
        let beneath = [0.2, 0.4, 0.6];
        let near = |got: [f32; 3], expected: [f32; 3]| {
            assert!(
                got.iter().zip(&expected).all(|(a, b)| (a - b).abs() < 1e-6),
                "{got:?}, not {expected:?}"
            );
        };
        near(over(beneath, [0.1, 0.1, 0.1, 0.5]), [0.2, 0.3, 0.4]);
        // No cover only adds.
        near(over(beneath, [0.1, 0.2, 0.3, 0.0]), [0.3, 0.6, 0.9]);
        // Whole cover leaves the light alone.
        near(over(beneath, [0.1, 0.2, 0.3, 1.0]), [0.1, 0.2, 0.3]);
        // No light and no cover leaves what is beneath.
        near(over(beneath, [0.0; 4]), beneath);
    }

    /// The area of a placement's quad, in square logical units.
    fn area(placement: &Placement) -> f32 {
        let length = |edge: Point| edge.x.hypot(edge.y);
        4.0 * length(placement.right) * length(placement.down)
    }

    #[test]
    fn the_decorations_area_bound_is_six_of_the_largest_or_six_views() {
        assert_eq!(decoration_area_bound(LOGICAL), 6.0 * 512.0 * 512.0);
        assert_eq!(decoration_area_bound(LOGICAL) / (1024.0 * 768.0), 2.0);
        assert_eq!(decoration_area_bound(WIDE), 1_572_864.0);
        assert!(decoration_area_bound(WIDE) / (2560.0 * 1080.0) < 0.57);
        // A small view: six deep over all of it.
        let small = LogicalSize { w: 100, h: 50 };
        assert_eq!(decoration_area_bound(small), 6.0 * 100.0 * 50.0);
    }

    /// The most share of a 1024×768 or a 2560×1080 view the whole sky
    /// shades at the lowest render scale, with no rush: the backdrop's
    /// and the stars' 3.5 (`backdrop`'s tests) and the decorations' 2.0.
    const SKY_AREA_BOUND: f32 = 5.5;

    #[test]
    fn the_whole_sky_with_six_of_the_largest_stays_within_its_bound() {
        const LOWEST: f32 = 0.25;
        // Nine of the largest, turned, all within the view: six are
        // drawn.
        let list: Vec<SkyDecoration> = (0..9)
            .map(|index| SkyDecoration {
                size: 4096.0,
                turn: 17.0 * index as f32,
                parallax: 0.0125,
                ..at(40.0 * index as f32 - 100.0, 30.0 * index as f32 - 90.0)
            })
            .collect();
        let star_square = (2.0 / LOWEST) * (2.0 / LOWEST);
        for logical in [LOGICAL, WIDE] {
            let view = logical.w as f32 * logical.h as f32;
            let seen = SkyPose::new(
                3,
                Point::new(0.0, 0.0),
                Point::new(logical.w as f32 / 2.0, logical.h as f32 / 2.0),
                32,
            );
            let drawn = placed(&seen, &list, logical, LOWEST);
            assert_eq!(drawn.len(), MOST_DECORATIONS, "{logical:?}");
            let shaded: f32 = drawn.iter().map(|(_, placement)| area(placement)).sum();
            assert!(
                shaded <= decoration_area_bound(logical) * 1.000_01,
                "{logical:?}: {shaded}"
            );
            // The largest is drawn at the largest size, so the bound is met.
            assert!(shaded >= 6.0 * 512.0 * 512.0 * 0.999_99, "{shaded}");
            // The whole sky, by construction: the backdrop, the stars at
            // twice the density each grown to a texel, and the
            // decorations.
            let stars = star_bound(cell_bound(logical));
            let whole =
                backdrop_area_bound(logical) + stars as f32 * star_square + shaded.min(6.0 * view);
            assert!(
                whole / view <= SKY_AREA_BOUND,
                "{logical:?}: {}",
                whole / view
            );
            // And its instances: a busy, cloudy sky's and the six.
            let look = SkyLook {
                stars: 2.0,
                backdrop: BackdropLook {
                    color: None,
                    amount: Some(1.0),
                },
                ..SkyLook::default()
            };
            let sky = instances(&seen, &look, logical, LOWEST);
            assert!(!sky.is_empty());
            assert!(
                sky.len() + drawn.len()
                    <= stars + backdrop_instance_bound(logical) + MOST_DECORATIONS,
                "{logical:?}: {} and {}",
                sky.len(),
                drawn.len()
            );
        }
        assert!(decoration_area_bound(LOGICAL) / (1024.0 * 768.0) <= 2.0);
    }
}
