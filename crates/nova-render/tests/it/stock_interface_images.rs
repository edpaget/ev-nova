//! The interface file's pictures through the image source: every `PICT` in
//! `Nova-DF.rsrc` beside the stock data resolves as an
//! [`ImageKind::InterfacePict`]. Skips, passing, when `NOVA_DATA` is unset
//! or the interface file is absent.

use std::rc::Rc;

use crate::common;

use nova_data::graphics::{EightBitReading, PICT};
use nova_data::{GameData, open_interface};
use nova_render::{GameImages, ImageSource};
use nova_view::ImageKind;

#[test]
fn every_stock_interface_picture_resolves_as_an_interface_picture() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let interface = match open_interface(&dir) {
        Ok(interface) => Rc::new(interface),
        Err(err) => {
            eprintln!("skipping: {err}");
            return;
        }
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let images =
        GameImages::new(&data, EightBitReading::AsDrawn).with_interface(Rc::clone(&interface));
    let ids = interface.ids(PICT);
    assert!(!ids.is_empty());
    let mut failures = Vec::new();
    for &id in &ids {
        match images.frames(ImageKind::InterfacePict, id) {
            Ok(frames) if frames.len() == 1 => {}
            Ok(frames) => failures.push(format!("PICT {id}: {} frames", frames.len())),
            Err(err) => failures.push(format!("PICT {id}: {err}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for id in 134..=137 {
        let arrow = images
            .frames(ImageKind::InterfacePict, id)
            .expect("the arrow resolves");
        assert_eq!((arrow[0].width(), arrow[0].height()), (11, 9), "PICT {id}");
    }
}
