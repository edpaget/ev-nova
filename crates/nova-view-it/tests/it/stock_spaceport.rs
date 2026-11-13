//! The spaceport over the stock data and interface file (`Nova-DF.rsrc`
//! beside `NOVA_DATA`, and `Nova.rez` beside `NOVA_DATA_REZ`), laid out with
//! the text-metrics mock. The tests over a real session are nova-flight's
//! `stock_spaceport`. Each test skips, passing, when its data is absent.

#![allow(clippy::float_cmp)]

use crate::common;

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use nova_data::graphics::{PICT, decode_pict};
use nova_data::{GameData, InterfaceData};
use nova_sim_frame::Service;
use nova_view::geometry::{Bounds, Point};
use nova_view::image::ImageKey;
use nova_view::sound::{Sound, SoundId, UiSound};
use nova_view::spaceport::layout::{
    BACKGROUND, LANDSCAPE_ITEM, LEAVE_ITEM, SPACEPORT_DIALOG, landscape_id,
};
use nova_view::spaceport::{
    ListColors, ListLooks, ShipLooks, SpaceportCatalog, SpaceportView, StellarId,
};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::DialogResources;
use nova_view::ui::button::ButtonSkin;
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
/// 618 x 517 "Spaceport" background, centred, with Leave bottom right and
/// the services its flags offer; its ambient sound, "Federation
/// Station.SFIL" (`snd ` 10032), plays on its first frame.
#[test]
fn port_kanes_spaceport_shows_its_landscape_description_and_services() {
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(SPACEPORT_DIALOG).expect("Spaceport");
        let mut view =
            SpaceportView::new(&data, StellarId(137), Ok((template, Rc::new(MonoMetrics))));
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
            Bounds::at(Point::new(203.0, 126.0), 618.0, 517.0)
        );
        assert_eq!(
            dialog.item_bounds(LEAVE_ITEM),
            Some(Bounds::at(
                Point::new(203.0 + 471.0, 126.0 + 456.0),
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
                top_left: Point::new(203.0, 126.0),
            })
        );
        assert!(
            list.iter()
                .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Port Kane"))
        );
        // Its landscape `PICT` stretched over the landscape item, and no
        // stand-in: a stock store has none.
        let record = data.stellar_port(StellarId(137)).expect("Port Kane");
        assert_eq!(record.stand_in, None);
        let area = dialog.item_bounds(LANDSCAPE_ITEM).expect("the landscape");
        let landscape = ImageKey::picture(landscape_id(record.cust_pic_id, record.graphic_type));
        assert!(
            list.iter().any(|c| matches!(
                c,
                DrawCommand::StretchedPicture { image, top_left, .. }
                    if *image == landscape && *top_left == area.min
            )),
            "{landscape:?} at {:?}",
            area.min
        );
        assert!(
            !list.iter().any(|c| matches!(
                c,
                DrawCommand::Triangle { corners, .. } if corners.iter().all(|&p| area.contains(p))
            )),
            "no stand-in in the landscape"
        );

        view.tick(Duration::from_millis(50));
        assert_eq!(
            view.take_sounds(),
            [Sound::Ui(UiSound::PortAmbient {
                sound: SoundId(10_032),
                until_stopped: false,
            })]
        );
    }
}

/// The left cap `view` draws for button `item` and whether the label
/// `label` sits inside the item.
fn button(view: &SpaceportView, item: usize, label: &str) -> (Option<ImageKey>, bool) {
    let bounds = view
        .dialog()
        .expect("laid out")
        .item_bounds(item)
        .expect("an item");
    let mut list = DrawList::new();
    view.draw(&mut list);
    let cap = list.iter().find_map(|command| match command {
        DrawCommand::StretchedPicture {
            image, top_left, ..
        } if *top_left == bounds.min => Some(*image),
        _ => None,
    });
    let placed = list.iter().any(|command| {
        matches!(command, DrawCommand::Text { text, origin, .. }
            if text == label && bounds.contains(*origin))
    });
    (cap, placed)
}

/// Every stock spaceport draws all seven buttons, as `_DrawPlanetButtons`
/// does (@0x29873-0x2995e): Port Kane (137) the Mission BBS at item 10
/// and the Bar at item 11, and its Shipyard, which it lacks, disabled;
/// Reflex-ion (129, uninhabited, `Flags` 0x21) all six disabled but
/// Leave.
#[test]
fn stock_spaceports_draw_every_button_greying_what_they_lack() {
    let normal = Some(ButtonSkin::NOVA.normal.left);
    let disabled = Some(ButtonSkin::NOVA.disabled.left);
    let labels = [
        (12, "Leave"),
        (4, "Recharge"),
        (7, "Trade Center"),
        (8, "Outfitter"),
        (9, "Shipyard"),
        (10, "Mission BBS"),
        (11, "Bar"),
    ];
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let view = |stellar| {
            let template = ui.dialog_template(SPACEPORT_DIALOG).expect("Spaceport");
            SpaceportView::new(
                &data,
                StellarId(stellar),
                Ok((template, Rc::new(MonoMetrics))),
            )
        };
        let kane = view(137);
        for (item, label) in labels {
            let cap = if item == 9 { disabled } else { normal };
            assert_eq!(
                button(&kane, item, label),
                (cap, true),
                "Port Kane: {label}"
            );
        }
        let reflexion = view(129);
        assert_eq!(reflexion.offered(), []);
        for (item, label) in labels {
            let cap = if item == LEAVE_ITEM { normal } else { disabled };
            assert_eq!(
                button(&reflexion, item, label),
                (cap, true),
                "Reflex-ion: {label}"
            );
        }
    }
}

/// Stock `cölr` 128's list colours are the stock ones: white text on
/// black, the selection dark red.
#[test]
fn the_stock_list_colours_are_colr_128s() {
    for (dir, _) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        assert_eq!(data.list_colors(), ListColors::STOCK);
    }
}

/// No stock `shïp` names a shipyard movie: each `MovieFile` (offset
/// 0x6AE, which `_LoadObjectData` copies into `_shipMovieFilenames`,
/// @0x7b0ce-0x7b136) is empty, so the shipyard and the hire screen loop
/// none (`_ShipyardFilter` @0x60858-0x60866, @0x60c77-0x60c85).
#[test]
fn no_stock_ship_names_a_shipyard_movie() {
    for (dir, _) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        assert_eq!(
            nova_view::spaceport::shipyard::movie_files(&data.ship_looks()),
            []
        );
    }
}
