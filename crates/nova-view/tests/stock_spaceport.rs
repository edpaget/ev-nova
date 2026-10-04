//! The spaceport over the stock data and interface file (`Nova-DF.rsrc`
//! beside `NOVA_DATA`, and `Nova.rez` beside `NOVA_DATA_REZ`), laid out with
//! the text-metrics mock. Each test skips, passing, when its data is
//! absent.

#![allow(clippy::float_cmp)]

mod common;

use std::path::PathBuf;
use std::rc::Rc;

use nova_data::graphics::{PICT, decode_pict};
use nova_data::{GameData, InterfaceData};
use nova_sim::Service;
use nova_view::geometry::{Bounds, Point};
use nova_view::spaceport::layout::{BACKGROUND, LEAVE_ITEM, SPACEPORT_DIALOG};
use nova_view::spaceport::{SpaceportView, StellarId};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::DialogResources;
use nova_view::{DrawCommand, DrawList, Screen};

/// The two stock builds: each `Nova Files` directory with the interface
/// file beside it, when present.
fn builds() -> Vec<(PathBuf, PathBuf)> {
    let mac =
        common::nova_data().and_then(|dir| Some((dir.clone(), common::interface_file(&dir)?)));
    let windows =
        common::nova_data_rez().and_then(|dir| Some((dir.clone(), common::interface_rez(&dir)?)));
    [mac, windows].into_iter().flatten().collect()
}

/// Port Kane's spaceport: its landscape, name and description over the
/// 618 x 517 "Spaceport" background, centred, with Leave bottom right and a
/// button for each service its flags offer.
#[test]
fn port_kanes_spaceport_shows_its_landscape_description_and_services() {
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(SPACEPORT_DIALOG).expect("Spaceport");
        let view = SpaceportView::new(&data, StellarId(137), Ok((template, Rc::new(MonoMetrics))));
        assert_eq!(view.problem(), None);
        assert_eq!(view.missing_landscape(), None);
        assert_eq!(view.description_problem(), None);
        assert_eq!(
            view.offered(),
            [
                Service::TradeCenter,
                Service::Outfitter,
                Service::Bar,
                Service::MissionBbs
            ]
        );
        let dialog = view.dialog().expect("laid out");
        assert_eq!(
            dialog.bounds(),
            Bounds::at(Point::new(203.0, 125.0), 618.0, 517.0)
        );
        assert_eq!(
            dialog.item_bounds(LEAVE_ITEM),
            Some(Bounds::at(
                Point::new(203.0 + 471.0, 125.0 + 456.0),
                145.0,
                25.0
            ))
        );
        let background = data.resource(PICT, 8500).expect("the Spaceport picture");
        let background = decode_pict(background.resource.data()).expect("decodes");
        assert_eq!((background.width(), background.height()), (618, 517));
        assert!(!dialog.scroll_text().expect("a box").lines().is_empty());

        let mut list = DrawList::new();
        view.draw(&mut list);
        assert_eq!(
            list.iter().next(),
            Some(&DrawCommand::Picture {
                image: BACKGROUND,
                top_left: Point::new(203.0, 125.0),
            })
        );
        assert!(
            list.iter()
                .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Port Kane"))
        );
    }
}
