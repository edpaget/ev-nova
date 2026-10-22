//! How long Super-xBR takes to enlarge the stock sprites as they load: every
//! stock `rlëD` sheet's frames, one sheet at a time, as `Renderer::load`
//! enlarges a resource's frames when it is first drawn
//! (`Enlarger::enlarge_frames`, its frames spread over the machine's
//! threads).
//!
//! The one test here is `#[ignore]`d, so CI never runs it; it only reads
//! the clock and prints. Run it by hand, in a release build, with the
//! stock data:
//!
//! ```text
//! NOVA_DATA=".../Nova Files" cargo nextest run -p nova-render --release --test it \
//!     --run-ignored only --no-capture stock_super_xbr_cost::
//! ```
//!
//! It prints the total for every sheet, the slowest single sheet, and how
//! many texels and 2048-square atlas pages the sheets take natively and at
//! 2x (gutters included, packing waste not). Without `NOVA_DATA` it skips,
//! passing.

use std::time::{Duration, Instant};

use nova_data::GameData;
use nova_data::graphics::RLED;
use nova_render::{Enlarger, GUTTER, ImageSource, PAGE_SIZE, SuperXbr};
use nova_view::ImageKind;

use crate::common;

/// The texels `w` x `h` frames take on a page, with their gutters.
fn placed(w: u32, h: u32) -> u64 {
    u64::from(w + GUTTER) * u64::from(h + GUTTER)
}

/// Pages of [`PAGE_SIZE`] that `texels` fill, at the least.
fn pages(texels: u64) -> f64 {
    texels as f64 / (f64::from(PAGE_SIZE) * f64::from(PAGE_SIZE))
}

#[test]
#[ignore = "a timing to run by hand in release, with the stock data"]
fn enlarging_every_stock_sheet() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut total = Duration::ZERO;
    let mut worst = (Duration::ZERO, 0_i16, 0_usize, (0, 0));
    let (mut sheets, mut frames_seen) = (0_usize, 0_usize);
    let (mut native_texels, mut enlarged_texels) = (0_u64, 0_u64);
    for &id in data.ids(RLED) {
        let Ok(frames) = data.frames(ImageKind::Rled, id) else {
            continue;
        };
        let start = Instant::now();
        let enlarged = SuperXbr.enlarge_frames(ImageKind::Rled, &frames);
        let took = start.elapsed();
        for (frame, enlarged) in frames.iter().zip(&enlarged) {
            assert_eq!(enlarged.width(), 2 * frame.width());
            native_texels += placed(frame.width(), frame.height());
            enlarged_texels += placed(enlarged.width(), enlarged.height());
        }
        total += took;
        sheets += 1;
        frames_seen += frames.len();
        if took > worst.0 {
            let size = frames.first().map_or((0, 0), |f| (f.width(), f.height()));
            worst = (took, id, frames.len(), size);
        }
    }
    let (took, id, count, (w, h)) = worst;
    println!(
        "Super-xBR 2x over {sheets} stock rlëD sheets ({frames_seen} frames): {:.1} ms",
        total.as_secs_f64() * 1000.0
    );
    println!(
        "slowest sheet: rlëD {id}, {count} frames of {w}x{h}: {:.1} ms",
        took.as_secs_f64() * 1000.0
    );
    println!(
        "atlas: {:.1} Mpx native ({:.1} pages), {:.1} Mpx at 2x ({:.1} pages, {:.2} GiB)",
        native_texels as f64 / 1e6,
        pages(native_texels),
        enlarged_texels as f64 / 1e6,
        pages(enlarged_texels),
        enlarged_texels as f64 * 4.0 / f64::from(1 << 30),
    );
}
