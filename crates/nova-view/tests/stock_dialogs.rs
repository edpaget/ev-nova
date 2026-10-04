//! Dialogs from the stock interface file (`Nova-DF.rsrc` beside
//! `NOVA_DATA`, and `Nova.rez` beside `NOVA_DATA_REZ`), laid out and driven
//! with the text-metrics mock, and the button and frame graphics in the
//! stock game data. Each test skips, passing, when its data is absent.

#![allow(clippy::float_cmp)]

mod common;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use nova_data::graphics::{Image, PICT, decode_pict};
use nova_data::records::dialog::Dlog;
use nova_data::{GameData, InterfaceData, Record};
use nova_view::geometry::{Bounds, Point};
use nova_view::sound::SoundPrefs;
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::desc::{DESC_DIALOG, DONE_ITEM, FRAME, TEXT_ITEM};
use nova_view::ui::new_pilot::{
    CANCEL_ITEM, NAME_ITEM, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome,
    OK_ITEM as NEW_PILOT_OK_ITEM,
};
use nova_view::ui::prefs::{
    KEY_SETTINGS_ITEM, MUSIC_ITEM, OK_ITEM, PREFS_DIALOG, SOUND_ITEM, VOLUME_DOWN_ITEM,
    VOLUME_LABEL_ITEM, VOLUME_UP_ITEM, VOLUME_VALUE_ITEM,
};
use nova_view::ui::{
    ButtonImages, ButtonSkin, ButtonStyle, DescDialog, DescriptionSource, Dialog, DialogEvent,
    DialogResources, ItemSpec, Placement, PrefsDialog,
};
use nova_view::{DrawCommand, DrawList, ImageKey, Input, Key, MouseButton, Screen};

/// The About text's `dësc`.
const ABOUT: i16 = 32767;

/// The two stock builds: each `Nova Files` directory with the interface
/// file beside it, when present.
fn builds() -> Vec<(PathBuf, PathBuf)> {
    let mac = common::nova_data().and_then(|dir| Some((common::interface_file(&dir)?, dir)));
    let windows = common::nova_data_rez().and_then(|dir| Some((common::interface_rez(&dir)?, dir)));
    [mac, windows]
        .into_iter()
        .flatten()
        .map(|(ui, dir)| (dir, ui))
        .collect()
}

fn interface(path: &Path) -> InterfaceData {
    InterfaceData::open(path).expect("the interface file opens")
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
    Bounds::at(Point::new(x, y), w, h)
}

fn drawn(screen: &impl Screen) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    screen.draw(&mut list);
    list.iter().cloned().collect()
}

fn key(key: Key) -> Input {
    Input::Key {
        key,
        pressed: true,
        repeat: false,
    }
}

/// Presses and releases the primary button at `at`.
fn click(dialog: &mut Dialog, at: Point) -> Option<DialogEvent> {
    let button = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    assert_eq!(dialog.input(&button(true)), None);
    dialog.input(&button(false))
}

#[test]
fn every_dialog_with_items_converts() {
    for (_, path) in builds() {
        let ui = interface(&path);
        let mut converted = 0;
        let mut failures = Vec::new();
        for id in ui.ids(Dlog::TYPE) {
            let (dlog, _) = ui.dialog(id).expect("listed").expect("decodes");
            let Some(items) = ui.items(dlog.record.items_id) else {
                continue;
            };
            let count = items.map_or(0, |(items, _)| items.record.items.len());
            match ui.dialog_template(id) {
                Ok(template) => {
                    assert_eq!(template.items.len(), count, "DLOG {id}'s items");
                    converted += 1;
                }
                Err(err) => failures.push(format!("DLOG {id}: {err}")),
            }
        }
        assert_eq!(failures, Vec::<String>::new(), "{}", path.display());
        // 41 DLOGs; four name a DITL the file does not have.
        assert_eq!(converted, 37, "{}", path.display());
    }
}

#[test]
fn desc_dialog_is_centred_with_its_button_and_text_box_where_recorded() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(DESC_DIALOG)
            .expect("converts");
        assert_eq!(
            (template.bounds.width(), template.bounds.height()),
            (441.0, 313.0)
        );
        assert_eq!(template.placement, Placement::Center);
        assert_eq!(template.items[0].bounds, rect(173.0, 281.0, 99.0, 25.0));
        assert_eq!(template.items[2].bounds, rect(11.0, 10.0, 417.0, 262.0));
    }
}

#[test]
fn the_about_text_scrolls_in_desc_dialog_and_return_closes_it() {
    for (dir, path) in builds() {
        let template = interface(&path)
            .dialog_template(DESC_DIALOG)
            .expect("converts");
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let text = data.description(ABOUT).expect("the About text");
        let mut desc = DescDialog::new(&template, &text, data.button_style(), Rc::new(MonoMetrics));

        // The dialog is centred: (1024 - 441) / 2, (768 - 313) / 2, floored.
        let origin = Point::new(291.0, 227.0);
        let dialog = desc.dialog();
        assert_eq!(dialog.bounds(), Bounds::at(origin, 441.0, 313.0));
        let done = rect(173.0, 281.0, 99.0, 25.0).offset(origin);
        assert_eq!(dialog.item_bounds(DONE_ITEM), Some(done));
        let commands = drawn(&desc);
        let normal = ButtonSkin::NOVA.normal;
        assert!(commands.contains(&DrawCommand::StretchedPicture {
            image: normal.left,
            top_left: done.min,
            width: 13.0,
            height: 25.0,
        }));
        assert!(matches!(
            commands[0],
            DrawCommand::StretchedPicture { image, .. } if image == FRAME.top
        ));
        let text_box = dialog.item_bounds(TEXT_ITEM).expect("text box");
        let scroll = dialog.scroll_text().expect("scrolling text");
        assert!(scroll.lines().len() > scroll.visible() && scroll.visible() > 0);
        // Only the description's lines are in Geneva 10.
        let lines: Vec<Point> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { origin, size, .. } if *size == 10.0 => Some(*origin),
                _ => None,
            })
            .collect();
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|at| text_box.contains(*at)), "{lines:?}");

        assert_eq!(desc.input(&key(Key::Down)), nova_view::ScreenAction::None);
        assert_eq!(desc.first_line(), 1);
        desc.input(&key(Key::Enter));
        assert!(desc.closed());
    }
}

#[test]
fn yes_no_draws_ok_and_cancel_where_recorded_and_answers_clicks_and_return() {
    for (_, path) in builds() {
        let template = interface(&path).dialog_template(3002).expect("converts");
        let mut dialog = Dialog::new(&template, &[], Rc::new(MonoMetrics));
        let origin = dialog.bounds().min;
        let ok = rect(263.0, 75.0, 70.0, 20.0).offset(origin);
        let cancel = rect(181.0, 75.0, 70.0, 20.0).offset(origin);
        assert_eq!(dialog.item_bounds(1), Some(ok));
        assert_eq!(dialog.item_bounds(5), Some(cancel));
        let lefts: Vec<Point> = drawn_dialog(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::StretchedPicture {
                    image, top_left, ..
                } if image == ButtonSkin::NOVA.normal.left => Some(top_left),
                _ => None,
            })
            .collect();
        assert_eq!(lefts, [ok.min, cancel.min]);
        let labels: Vec<String> = drawn_dialog(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["OK", "Text", "Cancel"], "pictures draw nothing");
        assert!(!dialog.item_shown(2), "item 2 is parked outside");
        assert!(dialog.item_shown(4));

        assert_eq!(dialog.input(&key(Key::Enter)), Some(DialogEvent::Item(1)));
        assert_eq!(
            click(&mut dialog, cancel.center()),
            Some(DialogEvent::Item(5))
        );
    }
}

fn drawn_dialog(dialog: &Dialog) -> Vec<DrawCommand> {
    let mut list = DrawList::new();
    dialog.draw(&mut list);
    list.iter().cloned().collect()
}

/// `PICT` `id` from the game data, decoded.
fn picture(data: &GameData, id: i16) -> Image {
    let resource = data.resource(PICT, id).expect("present").resource;
    decode_pict(resource.data()).unwrap_or_else(|err| panic!("PICT {id}: {err}"))
}

#[test]
fn the_button_and_frame_pictures_decode_at_their_pinned_sizes() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let skin = ButtonSkin::NOVA;
    let size = |key: ImageKey| {
        let image = picture(&data, key.id);
        (image.width(), image.height())
    };
    for ButtonImages {
        left,
        middle,
        right,
    } in [skin.normal, skin.pressed, skin.disabled]
    {
        for cap in [left, right] {
            assert_eq!(size(cap), (13, 25), "cap {}", cap.id);
            let nova_view::ImageKind::MaskedPict { mask } = cap.kind else {
                panic!("cap {} is masked", cap.id);
            };
            let mask = picture(&data, mask);
            assert_eq!(
                (mask.width(), mask.height()),
                (13, 25),
                "mask of {}",
                cap.id
            );
        }
        assert_eq!(size(middle), (2, 25), "middle {}", middle.id);
    }
    assert_eq!(skin.cap, 13.0);
    assert_eq!(size(FRAME.top), (441, 9));
    assert_eq!(size(FRAME.middle), (441, 365));
    assert_eq!(size(FRAME.bottom), (441, 40));
    assert_eq!((FRAME.top_height, FRAME.bottom_height), (9.0, 40.0));
}

/// Bounds from (left, top) to (right, bottom).
fn ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
    Bounds {
        min: Point::new(left, top),
        max: Point::new(right, bottom),
    }
}

#[test]
fn new_prefs_dialog_has_the_items_the_preferences_dialog_is_built_from() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(PREFS_DIALOG)
            .expect("converts");
        assert_eq!(template.placement, Placement::Center);
        // The Windows build's dialog is 18 taller, with the volume and the
        // buttons 18 lower, and a Brightness volume (items 23 to 26) at
        // the left.
        let windows = path.extension().is_some_and(|ext| ext == "rez");
        let (height, lower, count) = if windows {
            (296.0, 18.0, 26)
        } else {
            (278.0, 0.0, 22)
        };
        let name = path.display();
        assert_eq!(
            (template.bounds.width(), template.bounds.height()),
            (336.0, height),
            "{name}"
        );
        assert_eq!(template.items.len(), count, "{name}");
        let item = |n: usize| &template.items[n - 1];
        let expected = [
            (
                OK_ITEM,
                (225.0, 245.0, 295.0, 265.0),
                ItemSpec::Button("OK".into()),
            ),
            (
                VOLUME_LABEL_ITEM,
                (171.0, 167.0, 277.0, 183.0),
                ItemSpec::StaticText("Sound Volume:".into()),
            ),
            (
                VOLUME_VALUE_ITEM,
                (189.0, 186.0, 311.0, 202.0),
                ItemSpec::StaticText("Static Text".into()),
            ),
            (
                VOLUME_DOWN_ITEM,
                (172.0, 194.0, 183.0, 203.0),
                ItemSpec::Picture(135),
            ),
            (
                VOLUME_UP_ITEM,
                (172.0, 185.0, 183.0, 194.0),
                ItemSpec::Picture(134),
            ),
            (
                KEY_SETTINGS_ITEM,
                (49.0, 245.0, 184.0, 265.0),
                ItemSpec::Button("Key Settings".into()),
            ),
        ];
        for (n, (l, t, r, b), kind) in expected {
            assert_eq!(item(n).kind, kind, "{name} item {n}");
            assert_eq!(
                item(n).bounds,
                ltrb(l, t + lower, r, b + lower),
                "{name} item {n}"
            );
        }
        assert_eq!(
            item(MUSIC_ITEM).kind,
            ItemSpec::CheckBox("Intro Music".into())
        );
        assert_eq!(item(MUSIC_ITEM).bounds, ltrb(171.0, 33.0, 270.0, 51.0));
        assert_eq!(
            item(SOUND_ITEM).kind,
            ItemSpec::CheckBox("Ambient Sounds".into())
        );
        assert_eq!(item(SOUND_ITEM).bounds, ltrb(171.0, 121.0, 307.0, 139.0));
    }
}

#[test]
fn the_preferences_dialog_fits_its_music_volume_above_the_buttons() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(PREFS_DIALOG)
            .expect("converts");
        let prefs = PrefsDialog::new(
            &template,
            SoundPrefs::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let dialog = prefs.dialog();
        let music = prefs.music_volume().rects().bounds();
        let ok = dialog.item_bounds(OK_ITEM).expect("OK");
        let keys = dialog.item_bounds(KEY_SETTINGS_ITEM).expect("Key Settings");
        let name = path.display();
        assert!(music.max.y < ok.min.y && music.max.y < keys.min.y, "{name}");
        assert!(
            music.min.y > prefs.effects_volume().rects().bounds().max.y,
            "{name}"
        );
        assert!(dialog.bounds().contains(music.min) && dialog.bounds().contains(music.max));
        // Nothing else is drawn where the music volume is.
        for toggle in prefs.inert() {
            let rect = toggle.rect();
            let apart = rect.max.x <= music.min.x
                || rect.min.x >= music.max.x
                || rect.max.y <= music.min.y
                || rect.min.y >= music.max.y;
            assert!(apart, "{name}: {}", toggle.label());
        }
        assert_eq!(prefs.inert().len(), 11, "{name}");
    }
}

/// "Create a new pilot:" (`DLOG` 3102) is centred, with OK and Cancel
/// where recorded, its Full Name field taking the typed name, and Return
/// creating the pilot.
#[test]
fn the_new_pilot_dialog_takes_a_name_in_its_full_name_field() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(NEW_PILOT_DIALOG)
            .expect("converts");
        let name = path.display();
        assert_eq!(template.placement, Placement::Center, "{name}");
        assert_eq!(
            template.items[NAME_ITEM - 1].kind,
            ItemSpec::EditText("Edit Text".to_owned()),
            "{name}"
        );
        assert_eq!(
            template.items[NEW_PILOT_OK_ITEM - 1].kind,
            ItemSpec::Button("OK".to_owned())
        );
        assert_eq!(
            template.items[CANCEL_ITEM - 1].kind,
            ItemSpec::Button("Cancel".to_owned())
        );
        let mut dialog = NewPilotDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        let bounds = dialog.dialog().bounds();
        let field = dialog.field().rect();
        assert!(
            bounds.contains(field.min) && bounds.contains(field.max),
            "{name}"
        );
        let texts: Vec<String> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        for shown in ["Create a new pilot:", "Full Name:", "OK", "Cancel"] {
            assert!(texts.contains(&shown.to_owned()), "{name}: {texts:?}");
        }
        for c in "Ada".chars() {
            dialog.input(&Input::Text(c));
        }
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create("Ada".to_owned())),
            "{name}"
        );
    }
}
