//! One app frame of the placeholder over the stock data, through the
//! recording Gpu. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::time::Duration;

use nova::app::{App, AppScreen, Placeholder, PlaceholderContent, WindowEvent, WindowPort};
use nova_data::GameData;
use nova_render::Batch;
use nova_render::recording::{GpuCall, RecordingGpu};

struct Window;

impl WindowPort for Window {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

#[test]
fn the_placeholder_draws_the_stock_data() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let content = PlaceholderContent::from_data(&data);
    let (Some(_picture), Some((_sprite, frames))) = (content.picture, content.sprite) else {
        panic!("no picture or sprite in the stock data: {content:?}");
    };
    let screen = AppScreen::Placeholder(Placeholder::new(content));
    let mut app: App<_> = App::new(&Window, &data, screen);
    let mut gpu = RecordingGpu::new();

    app.handle(
        WindowEvent::Redraw {
            elapsed: Duration::ZERO,
        },
        &mut Window,
        &mut gpu,
    );

    assert_eq!(app.take_failures(), []);
    let uploads = gpu
        .calls
        .iter()
        .filter(|call| matches!(call, GpuCall::Upload { .. }))
        .count();
    assert_eq!(uploads, 1 + usize::from(frames));
    let frame = gpu.submits()[0];
    assert!(
        matches!(&frame.batches[0], Batch::Sprites { quads, .. } if quads.len() == 301),
        "{:?}",
        frame.batches.first()
    );
}
