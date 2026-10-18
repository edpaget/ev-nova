//! The main screen from the stock game data (`NOVA_DATA`) and the Yes/No
//! dialog from the stock interface file beside it. Each test skips,
//! passing, when the data is absent.

#![allow(clippy::float_cmp)]

use crate::common;

use std::rc::Rc;

use nova_data::records::string_list::StrList;
use nova_data::{GameData, InterfaceData, Record};
use nova_view::geometry::{Bounds, Point};
use nova_view::menu::{MainMenu, MainScreenArt, MainScreenSource, MenuChoice};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::yes_no::{QUESTION_ITEM, QUIT_QUESTION, YES_NO_DIALOG};
use nova_view::ui::{DescriptionSource, DialogResources, YesNoDialog};
use nova_view::{
    DrawCommand, DrawList, ImageKey, ImageKind, Input, Key, MouseButton, Screen, Sound, UiSound,
};

/// The stock game data and its main-screen art, when `NOVA_DATA` is set.
fn stock() -> Option<(GameData, MainScreenArt)> {
    let dir = common::nova_data()?;
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let art = data.main_screen_art().expect("the stock art resolves");
    Some((data, art))
}

/// The stock interface file, when `NOVA_DATA` is set and it is beside it.
fn interface() -> Option<InterfaceData> {
    let dir = common::nova_data()?;
    let path = common::interface_file(&dir)?;
    Some(InterfaceData::open(&path).expect("the interface file opens"))
}

/// A button as the phase recorded it: sprite, `cölr` 128 position and
/// frame size.
type Recorded = (i16, (f32, f32), (u32, u32));

/// The six buttons as the phase recorded them, New Pilot's first.
const BUTTONS: [Recorded; 6] = [
    (8050, (349.0, 400.0), (120, 61)),
    (8051, (344.0, 464.0), (99, 60)),
    (8052, (345.0, 528.0), (100, 60)),
    (8053, (555.0, 401.0), (120, 60)),
    (8054, (581.0, 464.0), (99, 61)),
    (8055, (580.0, 528.0), (98, 60)),
];

#[test]
fn the_stock_art_resolves_where_it_was_recorded() {
    let Some((_, art)) = stock() else {
        return;
    };
    assert_eq!((art.background, art.background_size), (8000, (1024, 768)));
    for (button, (sprite, (x, y), size)) in art.buttons.iter().zip(BUTTONS) {
        assert_eq!(button.sprite, sprite);
        assert_eq!(button.position, Point::new(x, y), "{sprite}");
        assert_eq!(
            (button.mask.width(), button.mask.height()),
            size,
            "{sprite}"
        );
    }
    assert_eq!(art.logo.picture, 8010);
    assert_eq!(art.logo.frame_size, (654.0, 209.0));
    assert_eq!((art.logo.columns, art.logo.frames), (1, 7));
    assert_eq!(art.logo.position, Point::new(191.0, 162.0));
    assert_eq!(art.rollover.sprite, 8020);
    assert_eq!(art.rollover.frame_size, (136.0, 98.0));
    assert_eq!(art.rollover.frames, 7);
    assert_eq!(art.rollover.position, Point::new(444.0, 465.0));
}

/// A slide as the phase recorded it: `PICT`, frame size, frames and
/// `cölr` 128 position.
type RecordedSlide = (i16, (f32, f32), u16, (f32, f32));

/// The three slides as the phase recorded them.
const SLIDES: [RecordedSlide; 3] = [
    (8030, (338.0, 63.0), 11, (343.0, 399.0)),
    (8031, (351.0, 64.0), 10, (337.0, 462.0)),
    (8032, (351.0, 65.0), 11, (337.0, 526.0)),
];

#[test]
fn the_stock_slides_resolve_where_they_were_recorded() {
    let Some((_, art)) = stock() else {
        return;
    };
    for (slide, (picture, size, frames, (x, y))) in art.slides.iter().zip(SLIDES) {
        assert_eq!(slide.picture, picture);
        assert_eq!(slide.frame_size, size, "{picture}");
        assert_eq!(slide.frames, frames, "{picture}");
        assert_eq!(slide.position, Point::new(x, y), "{picture}");
    }
}

/// The slides' `PICT`s drawn, each with its frame, in order.
fn slides_drawn(menu: &MainMenu) -> Vec<(i16, u16)> {
    let mut list = DrawList::new();
    menu.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { clip, command } => match **command {
                DrawCommand::Picture { image, top_left }
                    if SLIDES.iter().any(|slide| slide.0 == image.id) =>
                {
                    let (_, (_, height), ..) = SLIDES
                        .iter()
                        .find(|slide| slide.0 == image.id)
                        .expect("a slide");
                    Some((image.id, ((clip.min.y - top_left.y) / height) as u16))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn the_stock_slides_start_at_minus_1_minus_6_and_minus_11() {
    let Some((_, art)) = stock() else {
        return;
    };
    let mut menu = MainMenu::new(
        Some(art),
        nova_view::ui::ButtonStyle::STOCK,
        Rc::new(MonoMetrics),
    );
    menu.appear();
    let tick = nova_sim::mission::screens::mac_ticks(1);
    let mut first_frames = Vec::new();
    for at in 1..=12 {
        menu.tick(tick);
        for (picture, frame) in slides_drawn(&menu) {
            if frame == 0 {
                first_frames.push((at, picture));
            }
        }
    }
    assert_eq!(first_frames, [(1, 8030), (6, 8031), (11, 8032)]);
}

fn press(menu: &mut MainMenu, at: Point) -> Vec<Sound> {
    menu.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    });
    let sounds = menu.take_sounds();
    menu.cancel_pointer();
    sounds
}

#[test]
fn each_buttons_press_point_hits_it_and_a_clear_pixel_in_its_rectangle_misses() {
    let Some((data, art)) = stock() else {
        return;
    };
    let masks: Vec<_> = art
        .buttons
        .iter()
        .map(|button| button.mask.clone())
        .collect();
    let mut menu = MainMenu::new(Some(art), data.button_style(), Rc::new(MonoMetrics));
    for (choice, mask) in MenuChoice::ALL.into_iter().zip(masks) {
        let at = menu.press_point(choice);
        assert_eq!(
            press(&mut menu, at),
            [Sound::Ui(UiSound::ButtonDown)],
            "{choice:?}"
        );
        let bounds = menu.button_bounds(choice);
        let clear = (0..mask.height())
            .flat_map(|y| (0..mask.width()).map(move |x| (x, y)))
            .find(|&(x, y)| !mask.opaque_at(i64::from(x), i64::from(y)))
            .expect("a clear pixel");
        let at = Point::new(
            bounds.min.x + clear.0 as f32 + 0.5,
            bounds.min.y + clear.1 as f32 + 0.5,
        );
        assert!(bounds.contains(at));
        assert_eq!(press(&mut menu, at), [], "{choice:?} at {clear:?}");
    }
}

#[test]
fn the_stock_main_menu_draws_pict_8000_and_the_six_buttons_frame_0() {
    let Some((data, art)) = stock() else {
        return;
    };
    let menu = MainMenu::new(Some(art), data.button_style(), Rc::new(MonoMetrics));
    let mut list = DrawList::new();
    menu.draw(&mut list);
    let pictures: Vec<(ImageKey, Point)> = list
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Picture { image, top_left } => Some((*image, *top_left)),
            _ => None,
        })
        .collect();
    let mut expected = vec![(ImageKey::picture(8000), Point::new(0.0, 0.0))];
    expected.extend(
        BUTTONS
            .iter()
            .map(|&(sprite, (x, y), _)| (ImageKey::sprite(sprite, 0), Point::new(x, y))),
    );
    assert_eq!(pictures, expected);
    let logo = list.iter().find_map(|command| match command {
        DrawCommand::Clipped { clip, command } => match **command {
            DrawCommand::Picture { image, .. } if image.kind == ImageKind::Pict => Some(*clip),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(
        logo,
        Some(Bounds::at(Point::new(191.0, 162.0), 654.0, 209.0))
    );
}

#[test]
fn str_140_10_is_the_quit_question() {
    let Some(ui) = interface() else {
        return;
    };
    let res = ui
        .resources()
        .get(StrList::TYPE, 140)
        .expect("STR# 140 is in the interface file");
    let list = nova_data::decode_bytes::<StrList>(res.data())
        .expect("decodes")
        .record;
    assert_eq!(list.strings[9].as_str(), QUIT_QUESTION);
}

#[test]
fn the_yes_no_dialog_asks_in_stock_dlog_3002s_item_3_and_return_answers_yes() {
    let (Some(ui), Some((data, _))) = (interface(), stock()) else {
        return;
    };
    let template = ui.dialog_template(YES_NO_DIALOG).expect("converts");
    let mut dialog = YesNoDialog::new(
        &template,
        QUIT_QUESTION,
        data.button_style(),
        Rc::new(MonoMetrics),
    );
    let shown = dialog.dialog().text(QUESTION_ITEM).expect("the question");
    assert_eq!(shown.lines().join(" "), QUIT_QUESTION);
    assert_eq!(
        Some(shown.rect()),
        dialog.dialog().item_bounds(QUESTION_ITEM),
        "in item 3"
    );
    dialog.input(&Input::Key {
        key: Key::Enter,
        pressed: true,
        repeat: false,
    });
    assert_eq!(dialog.take_answer(), Some(true));
}
