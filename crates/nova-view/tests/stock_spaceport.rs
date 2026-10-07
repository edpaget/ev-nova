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
use nova_sim::{Chance, Pilot, Service, Session};
use nova_view::geometry::{Bounds, Point};
use nova_view::spaceport::layout::{BACKGROUND, LEAVE_ITEM, SPACEPORT_DIALOG};
use nova_view::spaceport::trade::{
    BACKGROUND as TRADE_BACKGROUND, BUY_ITEM, DONE_ITEM, SELL_ITEM, TRADE_DIALOG,
};
use nova_view::spaceport::{SpaceportView, StellarId, TradeScreen};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::DescriptionSource;
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

/// A new stock pilot docked at Port Kane (`spöb` 137 in Kania, where it
/// starts), through a save that says so.
fn at_port_kane(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(137);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, docked).expect("flies")
}

/// Port Kane's Trade Center: its exchange in the 426 x 252 "Trade"
/// dialog, centred over its background, with Done, Buy and Sell along the
/// bottom and a row for each good.
#[test]
fn port_kanes_trade_center_lists_its_goods_in_the_trade_dialog() {
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(TRADE_DIALOG).expect("Trade");
        let market = at_port_kane(&data).market().expect("a trade center");
        let screen = TradeScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            market,
            data.button_style(),
        );
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(
            dialog.bounds(),
            Bounds::at(Point::new(299.0, 258.0), 426.0, 252.0)
        );
        for (item, x) in [(DONE_ITEM, 272.0), (BUY_ITEM, 60.0), (SELL_ITEM, 166.0)] {
            assert_eq!(
                dialog.item_bounds(item),
                Some(Bounds::at(Point::new(299.0 + x, 258.0 + 221.0), 99.0, 25.0)),
                "{item}"
            );
        }
        let background = data.resource(PICT, 8510).expect("the Trade picture");
        let background = decode_pict(background.resource.data()).expect("decodes");
        assert_eq!((background.width(), background.height()), (426, 252));

        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(
            list.iter().next(),
            Some(&DrawCommand::Picture {
                image: TRADE_BACKGROUND,
                top_left: Point::new(299.0, 258.0),
            })
        );
        let texts: Vec<&str> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        for shown in ["Food", "93", "Equipment", "440", "Done", "Buy", "Sell"] {
            assert!(texts.contains(&shown), "{shown}: {texts:?}");
        }
    }
}

/// A new stock pilot docked at Viking (`spöb` 157 in Tichel, `sÿst` 129).
fn at_viking(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(129);
    save["stellar"] = serde_json::json!(157);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    Session::fly(data, docked).expect("flies")
}

/// A chance that fires every roll and draws 0: every ship's `BuyRandom`
/// from 1 to 99 rolls on, so the Shuttle (35) is listed.
struct Fires;

impl Chance for Fires {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, _n: u32) -> u32 {
        0
    }
}

/// Viking's Shipyard: its ships in the 765 x 323 "Shipyard" dialog,
/// centred over its background, with Info, Done and Buy Ship along the
/// bottom; the Shuttle's picture and description; and the info panel, the
/// 250 x 285 "Shipyard Info", over its own.
#[test]
fn vikings_shipyard_lists_its_ships_in_the_shipyard_dialog() {
    use nova_view::spaceport::SpaceportCatalog;
    use nova_view::spaceport::shipyard::{
        BACKGROUND as SHIPYARD_BACKGROUND, BUY_ITEM as BUY_SHIP_ITEM, DONE_ITEM as SHIPYARD_DONE,
        INFO_BACKGROUND, INFO_ITEM, SHIP_INFO_DIALOG, SHIPYARD_DIALOG, ShipBaseImages,
        ShipyardScreen, ship_picture,
    };
    for (dir, ui) in builds() {
        let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(SHIPYARD_DIALOG).expect("Shipyard");
        let info = ui.dialog_template(SHIP_INFO_DIALOG).expect("Shipyard Info");
        let shipyard = at_viking(&data).shipyard(&mut Fires).expect("a shipyard");
        let mut screen = ShipyardScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            Ok(info),
            ui.dialog_template(nova_view::ui::text_input::TEXT_INPUT_DIALOG),
            shipyard,
            Rc::clone(&data) as Rc<dyn nova_view::spaceport::ShipyardCatalog>,
            data.button_style(),
        );
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(
            dialog.bounds(),
            Bounds::at(Point::new(129.0, 222.0), 765.0, 323.0)
        );
        for (item, x, w) in [
            (INFO_ITEM, 253.0, 89.0),
            (SHIPYARD_DONE, 365.0, 109.0),
            (BUY_SHIP_ITEM, 480.0, 109.0),
        ] {
            assert_eq!(
                dialog.item_bounds(item),
                Some(Bounds::at(Point::new(129.0 + x, 222.0 + 289.0), w, 25.0)),
                "{item}"
            );
        }
        for (id, size) in [(8501, (765, 323)), (8506, (250, 285))] {
            let picture = data.resource(PICT, id).expect("the picture");
            let picture = decode_pict(picture.resource.data()).expect("decodes");
            assert_eq!((picture.width(), picture.height()), size, "{id}");
        }
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(
            list.iter().next(),
            Some(&DrawCommand::Picture {
                image: SHIPYARD_BACKGROUND,
                top_left: Point::new(129.0, 222.0),
            })
        );
        let texts: Vec<String> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        // The trade-in is a quarter of the Shuttle and half its Light
        // Blaster, the stock weapon it owns.
        for shown in [
            "Ship Price: 10000",
            "Trade-In: 5000",
            "Final Price: 5000",
            "Buy Ship",
            "Info",
            "Done",
            "(current)",
        ] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        assert!(list.iter().any(|c| matches!(
            c,
            DrawCommand::StretchedPicture { image, .. } if image.id == 5000
        )));
        // The second-hand Shuttle (361) has no picture of its own, and
        // shows the Shuttle's, whose base image it shares.
        let bases = data.ship_base_images();
        assert!(!data.picture_exists(5233));
        assert_eq!(
            ship_picture(nova_sim::ShipId(361), &bases, |id| data.picture_exists(id)),
            Some(5000)
        );
        screen.input(&nova_view::Input::Key {
            key: nova_view::Key::Char('i'),
            pressed: true,
            repeat: false,
        });
        let panel = screen.info_panel().expect("the info panel");
        assert_eq!(
            panel.bounds(),
            Bounds::at(Point::new(387.0, 241.0), 250.0, 285.0)
        );
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert!(list.iter().any(|c| *c
            == DrawCommand::Picture {
                image: INFO_BACKGROUND,
                top_left: Point::new(387.0, 241.0),
            }));
    }
}

/// A chance that draws the last of each roll's outcomes.
struct Highest;

impl Chance for Highest {
    fn fires(&mut self, _percent: u8) -> bool {
        true
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }
}

/// Buying a Shuttle at Viking asks its name in the stock "Text Input"
/// dialog: 360 x 138, centred, the prompt naming the Shuttle's Long Name
/// over the default, selected, and OK and Cancel along the bottom.
#[test]
fn the_shuttles_name_is_asked_in_the_stock_text_input_dialog() {
    use nova_view::ui::text_input::{
        CANCEL_ITEM, FIELD_ITEM, OK_ITEM, PROMPT_ITEM, TEXT_INPUT_DIALOG, TextInputDialog,
        TextInputOutcome,
    };
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(TEXT_INPUT_DIALOG).expect("Text Input");
        let mut session = at_viking(&data);
        session.shipyard(&mut Fires).expect("a shipyard");
        let naming = session
            .ship_naming(nova_sim::ShipId(128), &mut Highest)
            .expect("the Shuttle can be bought");
        assert_eq!(
            naming.prompt,
            "Please name your new Sigma Shipyards Alpha class Shuttle: "
        );
        assert_eq!(naming.default, "Shuttle 999");
        let mut dialog = TextInputDialog::new(
            &template,
            &naming.prompt,
            &naming.default,
            nova_sim::shipyard::SHIP_NAME_CHARS,
            data.button_style(),
            Rc::new(MonoMetrics),
        )
        .expect("the stock dialog has its field");
        let bounds = dialog.dialog().bounds();
        assert_eq!(bounds, Bounds::at(Point::new(332.0, 315.0), 360.0, 138.0));
        let at = |x: f32, y: f32, w, h| Bounds::at(Point::new(332.0 + x, 315.0 + y), w, h);
        assert_eq!(dialog.field().rect(), at(91.0, 64.0, 200.0, 16.0));
        for (item, bounds) in [
            (OK_ITEM, at(252.0, 106.0, 70.0, 20.0)),
            (PROMPT_ITEM, at(52.0, 5.0, 295.0, 50.0)),
            (FIELD_ITEM, at(91.0, 64.0, 200.0, 16.0)),
            (CANCEL_ITEM, at(170.0, 106.0, 70.0, 20.0)),
        ] {
            assert_eq!(dialog.dialog().item_bounds(item), Some(bounds), "{item}");
            assert!(dialog.dialog().item_shown(item), "{item}");
        }
        assert!(!dialog.dialog().item_shown(2), "parked below");
        assert!(dialog.field().selected());
        let mut list = DrawList::new();
        dialog.draw(&mut list);
        let texts: Vec<String> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts
                .iter()
                .any(|text| text.starts_with("Please name your new")),
            "{texts:?}"
        );
        for shown in ["OK", "Cancel", "Shuttle 999"] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
        dialog.input(&nova_view::Input::Key {
            key: nova_view::Key::Enter,
            pressed: true,
            repeat: false,
        });
        assert_eq!(
            dialog.take_outcome(),
            Some(TextInputOutcome::Confirm("Shuttle 999".to_owned()))
        );
    }
}
