//! How long the wgpu adapter takes to draw flight's worst case today: a
//! full Retina window (2048x1536, logical 1024x768 at scale 2) with a
//! starfield and one ship whose engine glow and running lights are both
//! lit.
//!
//! The one test here is `#[ignore]`d, so CI never runs it; it only reads
//! the clock and prints. Run it by hand, in a release build:
//!
//! ```text
//! cargo nextest run -p nova-render --release --test frame_cost --run-ignored only --no-capture
//! ```
//!
//! Like the other offscreen tests it needs a GPU adapter and skips,
//! passing, without one. It is hermetic: an offscreen target and in-code
//! images, with no files or environment.

use std::time::Instant;

use nova_data::graphics::Image;
use nova_render::fonts::FontFaces;
use nova_render::wgpu::{InitError, OffscreenGpu};
use nova_render::{ImageError, ImageSource, LOGICAL, Renderer, Viewport};
use nova_view::draw::lights_tint;
use nova_view::{Color, DrawList, ImageKey, ImageKind, Point};

/// A full Retina window.
const TARGET: (u32, u32) = (2048, 1536);

/// The ship sheets' side, in texels.
const SHEET: u32 = 128;

/// Frames drawn before the clock starts: uploads, pipeline warm-up.
const WARM_UP: u32 = 60;

/// Frames timed in one trial.
const TIMED: u32 = 600;

/// Trials per arm; each arm's minimum and mean are reported.
const TRIALS: u32 = 5;

/// A 5-bit colour widened to opaque 8-bit by bit replication, as stock
/// sprite texels are.
fn w555(rgb5: [u8; 3]) -> [u8; 4] {
    let [r, g, b] = rgb5.map(|c| (c << 3) | (c >> 2));
    [r, g, b, 255]
}

/// A [`SHEET`]-square image, black except the texels `lit` picks, which
/// are `color`.
fn sheet(color: [u8; 4], lit: impl Fn(u32, u32) -> bool) -> Image {
    let pixels = (0..SHEET)
        .flat_map(|y| (0..SHEET).map(move |x| (x, y)))
        .flat_map(|(x, y)| if lit(x, y) { color } else { [0, 0, 0, 255] })
        .collect();
    Image::from_rgba(SHEET, SHEET, pixels).expect("square sheet")
}

/// The ship: a hull (`rlëD` 1), a glow sheet with a 24x24 lit block
/// (`rlëD` 2) and a lights sheet with four 3x3 lit dots (`rlëD` 3).
struct Ship;

impl ImageSource for Ship {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let inside = |at: u32, from: u32, size: u32| (from..from + size).contains(&at);
        match (kind, id) {
            (ImageKind::Rled, 1) => Ok(vec![sheet(w555([12, 14, 16]), |_, _| true)]),
            (ImageKind::Rled, 2) => Ok(vec![sheet(w555([31, 20, 8]), |x, y| {
                inside(x, 52, 24) && inside(y, 100, 24)
            })]),
            (ImageKind::Rled, 3) => Ok(vec![sheet(w555([8, 31, 8]), |x, y| {
                [(20, 40), (105, 40), (40, 90), (85, 90)]
                    .iter()
                    .any(|&(left, top)| inside(x, left, 3) && inside(y, top, 3))
            })]),
            _ => Err(ImageError::Missing),
        }
    }
}

/// Appends a sprite composited one particular way.
type Layer = fn(&mut DrawList, ImageKey, Point, Color) -> &mut DrawList;

/// The frame: 300 two-unit stars, then the hull at the centre with its
/// glow and lights over it, each drawn with `layer`.
fn scene(layer: Layer) -> DrawList {
    let mut list = DrawList::new();
    for star in 0..300_u32 {
        let at = Point::new(
            ((star * 397 + 13) % LOGICAL.w) as f32,
            ((star * 211 + 7) % LOGICAL.h) as f32,
        );
        list.dot(at, 2.0, Color::WHITE);
    }
    let centre = Point::new(LOGICAL.w as f32 / 2.0, LOGICAL.h as f32 / 2.0);
    list.sprite(ImageKey::sprite(1, 0), centre, Color::WHITE);
    layer(&mut list, ImageKey::sprite(2, 0), centre, lights_tint(24));
    layer(&mut list, ImageKey::sprite(3, 0), centre, lights_tint(32));
    list
}

/// The milliseconds one frame of `list` takes, over [`TIMED`] frames
/// after [`WARM_UP`], with the queue drained inside the timed span.
fn ms_per_frame(gpu: &mut OffscreenGpu, list: &DrawList) -> f64 {
    let viewport = Viewport::new(LOGICAL, TARGET, 2.0);
    let mut renderer = Renderer::new(Ship);
    for _ in 0..WARM_UP {
        assert_eq!(renderer.render(list, &viewport, gpu).new_failures, vec![]);
    }
    gpu.read_pixels().expect("read back");
    let start = Instant::now();
    for _ in 0..TIMED {
        renderer.render(list, &viewport, gpu);
    }
    gpu.read_pixels().expect("read back");
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(TIMED)
}

/// The minimum and mean of `samples`.
fn min_mean(samples: &[f64]) -> (f64, f64) {
    let min = samples.iter().copied().fold(f64::INFINITY, f64::min);
    (min, samples.iter().sum::<f64>() / samples.len() as f64)
}

#[test]
#[ignore = "reads the clock; run by hand (see the module doc)"]
fn a_lit_ship_frame_costs() {
    let mut gpu = match OffscreenGpu::new(TARGET.0, TARGET.1, &FontFaces::bundled()) {
        Ok(gpu) => gpu,
        Err(error @ (InitError::NoAdapter(_) | InitError::NoDevice(_))) => {
            eprintln!("skipping: no GPU adapter ({error})");
            return;
        }
        Err(error) => panic!("offscreen GPU: {error}"),
    };
    let additive = scene(DrawList::additive_sprite);
    let or = scene(DrawList::or_sprite);
    // The arms alternate, so both see the same clocks and temperatures.
    let (mut additive_ms, mut or_ms) = (Vec::new(), Vec::new());
    for _ in 0..TRIALS {
        additive_ms.push(ms_per_frame(&mut gpu, &additive));
        or_ms.push(ms_per_frame(&mut gpu, &or));
    }
    let (additive_min, additive_mean) = min_mean(&additive_ms);
    let (or_min, or_mean) = min_mean(&or_ms);
    println!(
        "additive: {additive_mean:.3} ms/frame (min {additive_min:.3}) over {TRIALS} trials of {TIMED}"
    );
    println!("or: {or_mean:.3} ms/frame (min {or_min:.3}) over {TRIALS} trials of {TIMED}");
    assert!(
        or_mean <= additive_mean + 1.0,
        "the OR composite costs {or_mean:.3} ms/frame, over 1 ms more than additive's {additive_mean:.3}"
    );
}
