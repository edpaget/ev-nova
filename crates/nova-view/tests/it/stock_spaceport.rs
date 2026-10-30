//! The spaceport over the stock data and interface file (`Nova-DF.rsrc`
//! beside `NOVA_DATA`, and `Nova.rez` beside `NOVA_DATA_REZ`), laid out with
//! the text-metrics mock. Each test skips, passing, when its data is
//! absent.

#![allow(clippy::float_cmp)]

use crate::common;

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use nova_data::graphics::{PICT, decode_pict};
use nova_data::{GameData, InterfaceData};
use nova_sim::fixture::SessionBuilder;
use nova_sim::{Chance, Pilot, Service, Session};
use nova_view::geometry::{Bounds, Point};
use nova_view::image::ImageKey;
use nova_view::sound::{Sound, SoundId, UiSound};
use nova_view::spaceport::layout::{BACKGROUND, LEAVE_ITEM, SPACEPORT_DIALOG};
use nova_view::spaceport::trade::{
    BACKGROUND as TRADE_BACKGROUND, BUY_ITEM, DONE_ITEM, SELL_ITEM, TRADE_DIALOG,
};
use nova_view::spaceport::{
    ListColors, ListLooks, ShipMovieFiles, SpaceportView, StellarId, TradeScreen,
};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::DescriptionSource;
use nova_view::ui::DialogResources;
use nova_view::ui::button::ButtonSkin;
use nova_view::ui::quantity::QUANTITY_DIALOG;
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

/// A new stock pilot landed at Port Kane (`spöb` 137 in Kania, where it
/// starts).
fn at_port_kane(data: &GameData) -> Session {
    let kania = Pilot::new(data, "Stock")
        .expect("the stock first chär starts")
        .system();
    SessionBuilder::flying(data)
        .named("Stock")
        .landed_at(kania, StellarId(137))
        .build()
}

/// Port Kane's Trade Center: its exchange in the 426 x 252 "Trade"
/// dialog, centred over its background, with Done, Buy and Sell along the
/// bottom, the header in `STR#` 2002's words, a row for each good and the
/// cargo summary's free space, with no status line.
#[test]
fn port_kanes_trade_center_lists_its_goods_in_the_trade_dialog() {
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(TRADE_DIALOG).expect("Trade");
        // The strings the app gives the session: `STR#` 2002's words.
        let strings = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
        let market = at_port_kane(&data)
            .with_strings(strings)
            .market()
            .expect("a trade center");
        let screen = TradeScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            ui.dialog_template(QUANTITY_DIALOG),
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
        for shown in [
            "Commodity:",
            "In Hold:",
            "Price:",
            "Food",
            "93",
            "Equipment",
            "440",
            "Done",
            "Buy",
            "Sell",
        ] {
            assert!(texts.contains(&shown), "{shown}: {texts:?}");
        }
        assert!(
            texts
                .iter()
                .any(|text| text.starts_with("Free cargo space: ")),
            "{texts:?}"
        );
        assert!(!texts.iter().any(|text| text.contains("Credits")));
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
        assert_eq!(data.ship_movie_files(), []);
    }
}

/// A new stock pilot landed at Viking (`spöb` 157 in Tichel, `sÿst` 129).
fn at_viking(data: &GameData) -> Session {
    SessionBuilder::flying(data)
        .named("Stock")
        .landed_at(nova_sim::SystemId(129), StellarId(157))
        .build()
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
        INFO_ITEM, InfoTemplates, SHIP_INFO_DIALOG, SHIP_PICTURE_INFO_DIALOG, SHIPYARD_DIALOG,
        ShipBaseImages, ShipyardScreen, ship_picture,
    };
    for (dir, ui) in builds() {
        let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(SHIPYARD_DIALOG).expect("Shipyard");
        let info = InfoTemplates {
            plain: Ok(ui.dialog_template(SHIP_INFO_DIALOG).expect("Shipyard Info")),
            picture: Ok(ui
                .dialog_template(SHIP_PICTURE_INFO_DIALOG)
                .expect("Shipyard Info with a picture")),
        };
        let shipyard = at_viking(&data).shipyard(&mut Fires).expect("a shipyard");
        let mut screen = ShipyardScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            info,
            ui.dialog_template(nova_view::ui::text_input::TEXT_INPUT_DIALOG),
            shipyard,
            Rc::clone(&data) as Rc<dyn nova_view::spaceport::ShipyardCatalog>,
            data.button_style(),
        );
        let dialog = screen.dialog().expect("laid out");
        assert_eq!(
            dialog.bounds(),
            Bounds::at(Point::new(130.0, 223.0), 765.0, 323.0)
        );
        for (item, x, w) in [
            (INFO_ITEM, 253.0, 89.0),
            (SHIPYARD_DONE, 480.0, 109.0),
            (BUY_SHIP_ITEM, 365.0, 109.0),
        ] {
            assert_eq!(
                dialog.item_bounds(item),
                Some(Bounds::at(Point::new(130.0 + x, 223.0 + 289.0), w, 25.0)),
                "{item}"
            );
        }
        for (id, size) in [
            (8501, (765, 323)),
            (8506, (250, 285)),
            (8507, (614, 537)),
            (20128, (600, 400)),
        ] {
            let picture = data.resource(PICT, id).expect("the picture");
            let picture = decode_pict(picture.resource.data()).expect("decodes");
            assert_eq!((picture.width(), picture.height()), size, "{id}");
        }
        // It opens with nothing selected, as `_DoShipyardDialog` does;
        // Right picks the first cell.
        assert_eq!(screen.selected(), None, "nothing selected as it opens");
        screen.input(&nova_view::Input::Key {
            key: nova_view::Key::Right,
            pressed: true,
            repeat: false,
        });
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(
            list.iter().next(),
            Some(&DrawCommand::Picture {
                image: SHIPYARD_BACKGROUND,
                top_left: Point::new(130.0, 223.0),
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
        // Blaster, the stock weapon it owns: 5000, through the flux twice
        // at the Shuttle's tech 3 against Viking's 4, 4850 then 4700.
        assert_eq!(screen.shipyard().trade_in, 4700);
        // The Shuttle selected is the class flown, so the info box shows
        // no price lines.
        let selected = screen.selected().expect("a ship selected");
        assert_eq!(
            screen.shipyard().current,
            screen.shipyard().rows[selected].id
        );
        assert!(!texts.contains(&"Ship Price:".to_owned()), "{texts:?}");
        for shown in ["Buy Ship", "Info", "Done", "(current)"] {
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
        the_shuttles_info_is_the_picture_panel(&mut screen);
    }
}

/// I on the Shuttle, selected in Viking's `screen`, opens the picture
/// variant: its `dësc` 13000 has `Graphic` 20128, so its Info is `DLOG`
/// 1019 over `PICT` 8507, with the graphic in item 7 and its stock
/// weapon in item 8.
fn the_shuttles_info_is_the_picture_panel(screen: &mut nova_view::spaceport::ShipyardScreen) {
    use nova_view::spaceport::shipyard::{
        PANEL_GRAPHIC_ITEM, PANEL_WEAPONS_ITEM, PICTURE_INFO_BACKGROUND,
    };
    screen.input(&nova_view::Input::Key {
        key: nova_view::spaceport::shipyard::INFO_KEY,
        pressed: true,
        repeat: false,
    });
    let panel = screen.info_panel().expect("the info panel");
    assert_eq!(
        panel.bounds(),
        Bounds::at(Point::new(205.0, 116.0), 614.0, 537.0)
    );
    let graphic = panel.item_bounds(PANEL_GRAPHIC_ITEM).expect("item 7");
    assert_eq!(graphic, Bounds::at(Point::new(212.0, 122.0), 600.0, 400.0));
    let mut list = DrawList::new();
    screen.draw(&mut list);
    assert!(list.iter().any(|c| *c
        == DrawCommand::Picture {
            image: PICTURE_INFO_BACKGROUND,
            top_left: Point::new(205.0, 116.0),
        }));
    assert!(list.iter().any(|c| *c
        == DrawCommand::StretchedPicture {
            image: nova_view::ImageKey::picture(20128),
            top_left: graphic.min,
            width: 600.0,
            height: 400.0,
        }));
    // Its stock Light Blaster is the weapons sentence, in item 8.
    let weapons = panel.item_bounds(PANEL_WEAPONS_ITEM).expect("item 8");
    assert!(
        list.iter().any(|c| matches!(
            c,
            DrawCommand::Text { text, origin, .. }
                if text == "1 Light Blaster" && weapons.contains(*origin)
        )),
        "{list:?}"
    );
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

/// Option-Buy at Port Kane's Trade Center asks how many in the stock
/// quantity dialog (`DLOG` 1003): 172 x 72, centred, OK titled "Buy",
/// the prompt and the field holding the most, and Cancel.
#[test]
fn option_buy_at_port_kane_asks_how_many_in_the_stock_quantity_dialog() {
    use nova_view::ui::quantity::{CANCEL_ITEM, FIELD_ITEM, OK_ITEM, PROMPT_ITEM};
    let press = |key| nova_view::Input::Key {
        key,
        pressed: true,
        repeat: false,
    };
    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let template = ui.dialog_template(TRADE_DIALOG).expect("Trade");
        let quantity = ui
            .dialog_template(QUANTITY_DIALOG)
            .expect("the quantity dialog");
        let market = at_port_kane(&data).market().expect("a trade center");
        let max = market
            .row_max(0, nova_sim::Direction::Buy)
            .expect("the first row is bought");
        let mut screen = TradeScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            Ok(quantity),
            market,
            data.button_style(),
        );
        screen.input(&press(nova_view::Key::Alt));
        screen.input(&press(nova_view::Key::Char('b')));
        let dialog = screen.quantity().expect("asking");
        let bounds = dialog.dialog().bounds();
        assert_eq!(bounds, Bounds::at(Point::new(426.0, 348.0), 172.0, 72.0));
        let at = |x: f32, y: f32, w, h| Bounds::at(Point::new(426.0 + x, 348.0 + y), w, h);
        for (item, bounds) in [
            (OK_ITEM, at(92.0, 42.0, 70.0, 20.0)),
            (PROMPT_ITEM, at(6.0, 8.0, 102.0, 16.0)),
            (FIELD_ITEM, at(112.0, 8.0, 51.0, 16.0)),
            (CANCEL_ITEM, at(10.0, 42.0, 70.0, 20.0)),
        ] {
            assert_eq!(dialog.dialog().item_bounds(item), Some(bounds), "{item}");
            assert!(dialog.dialog().item_shown(item), "{item}");
        }
        assert_eq!(dialog.field().text(), max.to_string());
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
        for shown in ["Buy", "Enter quantity:", "Cancel"] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
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
        // Flying the Heavy Shuttle, as the Shuttle flown is not for sale.
        let mut session = SessionBuilder::flying(&data)
            .named("Stock")
            .in_ship(nova_sim::ShipId(129))
            .landed_at(nova_sim::SystemId(129), StellarId(157))
            .build();
        session.shipyard(&mut Fires).expect("a shipyard");
        let naming = session
            .ship_naming(nova_sim::ShipId(128), &data, &mut Highest)
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

/// The news at Viking (Federation, govt 128) is drawn over the
/// Federation's `NewsPic` 9001, stretched to the 300 x 230 "News"
/// dialog, the commercial in its band from 140 px; the Vell-os's
/// `NewsPic` 0 is no picture, so theirs falls back to 9000.
#[test]
fn the_news_is_drawn_over_the_stock_governments_picture() {
    use nova_sim::CombatCatalog;
    use nova_sim::news::news_picture;
    use nova_view::image::ImageKey;
    use nova_view::spaceport::SpaceportCatalog;
    use nova_view::spaceport::news::{NEWS_DIALOG, shown_news_picture};

    for (dir, ui) in builds() {
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let ui = InterfaceData::open(&ui).expect("the interface file opens");
        let mut session = at_viking(&data);
        assert!(session.open_bar(&mut Fires), "Viking has a bar");
        assert_eq!(session.news_picture(), Some(9001), "the Federation's");
        let exists = |id| data.picture_exists(id);
        assert_eq!(shown_news_picture(9001, exists), Some(9001));

        let govts = data.governments();
        let vell_os = govts
            .iter()
            .find(|govt| govt.id == nova_sim::GovtId(136))
            .expect("the Vell-os");
        assert_eq!(vell_os.news_pic, 0);
        assert_eq!(news_picture(Some(vell_os)), 0);
        assert_eq!(shown_news_picture(0, exists), Some(9000), "no PICT 0");

        let template = ui.dialog_template(NEWS_DIALOG).expect("News");
        let news = session.news().expect("drawn").clone();
        let screen = nova_view::spaceport::NewsScreen::new(
            Ok((template, Rc::new(MonoMetrics))),
            &news,
            Some(9001),
        );
        let frame = screen.bounds().expect("laid out");
        assert_eq!((frame.width(), frame.height()), (300.0, 230.0));
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert!(list.iter().any(|command| *command
            == DrawCommand::StretchedPicture {
                image: ImageKey::picture(9001),
                top_left: frame.min,
                width: 300.0,
                height: 230.0,
            }));
        assert_eq!(
            screen.commercial().expect("laid out").rect().min,
            Point::new(frame.min.x + 10.0, frame.min.y + 140.0)
        );
    }
}
