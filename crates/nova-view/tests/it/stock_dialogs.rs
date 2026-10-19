//! Dialogs from the stock interface file (`Nova-DF.rsrc` beside
//! `NOVA_DATA`, and `Nova.rez` beside `NOVA_DATA_REZ`), laid out and driven
//! with the text-metrics mock, and the button and frame graphics in the
//! stock game data. Each test skips, passing, when its data is absent.

#![allow(clippy::float_cmp)]

use crate::common;

use std::path::{Path, PathBuf};
use std::rc::Rc;

use nova_data::graphics::{Image, PICT, decode_pict};
use nova_data::records::dialog::Dlog;
use nova_data::records::string_list::StrList;
use nova_data::{GameData, InterfaceData, Record};
use nova_view::Prefs;
use nova_view::geometry::{Bounds, Point};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::desc::{DESC_DIALOG, DONE_ITEM, FRAME, TEXT_ITEM};
use nova_view::ui::new_pilot::{
    CANCEL_ITEM, GENDER_ITEM, NAME_ITEM, NEW_PILOT_DIALOG, NewPilotDialog, NewPilotOutcome,
    OK_ITEM as NEW_PILOT_OK_ITEM, STRICT_PLAY_ITEM,
};
use nova_view::ui::note::{
    LOAD_FALLBACK_NOTE, NOTE_DIALOG, NoteDialog, OK_ITEM as NOTE_OK_ITEM,
    TEXT_ITEM as NOTE_TEXT_ITEM,
};
use nova_view::ui::prefs::{
    HYPERSPACE_EFFECTS_ITEM, HYPERSPACE_EFFECTS_LABEL, KEY_SETTINGS_ITEM, MUSIC_ITEM, NOTICE_AREA,
    OK_ITEM, PREFS_DIALOG, SCALING_ITEM, SOUND_ITEM, VOLUME_DOWN_ITEM, VOLUME_LABEL_ITEM,
    VOLUME_UP_ITEM, VOLUME_VALUE_ITEM,
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
fn the_note_dialog_lays_out_from_dlog_3000_with_ok_and_its_text() {
    for (dir, path) in builds() {
        let template = interface(&path)
            .dialog_template(NOTE_DIALOG)
            .expect("converts");
        assert_eq!(
            (template.bounds.width(), template.bounds.height()),
            (357.0, 150.0)
        );
        assert_eq!(template.items[0].bounds, rect(282.0, 118.0, 70.0, 20.0));
        assert_eq!(template.items[0].kind, ItemSpec::Button("OK".to_owned()));
        assert_eq!(template.items[2].bounds, rect(52.0, 5.0, 295.0, 99.0));
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let mut note = NoteDialog::new(
            &template,
            LOAD_FALLBACK_NOTE,
            data.button_style(),
            Rc::new(MonoMetrics),
        );
        let dialog = note.dialog();
        let origin = dialog.bounds().min;
        assert_eq!(
            dialog.item_bounds(NOTE_OK_ITEM),
            Some(rect(282.0, 118.0, 70.0, 20.0).offset(origin))
        );
        let text = dialog.text(NOTE_TEXT_ITEM).expect("the text item");
        assert_eq!(text.rect(), rect(52.0, 5.0, 295.0, 99.0).offset(origin));
        assert_eq!(
            text.lines().concat().replace(' ', ""),
            LOAD_FALLBACK_NOTE.replace(' ', "")
        );
        assert!(!dialog.item_shown(2), "the parked picture");
        note.input(&Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        });
        assert!(note.closed(), "{}", path.display());
    }
}

#[test]
fn str_140_52_is_the_load_fallback_note() {
    for (_, path) in builds() {
        let ui = interface(&path);
        let res = ui
            .resources()
            .get(StrList::TYPE, 140)
            .expect("STR# 140 is in the interface file");
        let list = nova_data::decode_bytes::<StrList>(res.data())
            .expect("decodes")
            .record;
        assert_eq!(
            list.strings[51].as_str(),
            LOAD_FALLBACK_NOTE,
            "{}",
            path.display()
        );
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
        assert_eq!(
            item(HYPERSPACE_EFFECTS_ITEM).kind,
            ItemSpec::CheckBox(HYPERSPACE_EFFECTS_LABEL.into()),
            "{name}"
        );
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
            Prefs::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let dialog = prefs.dialog();
        let music = prefs.music_volume().rects().bounds();
        assert_eq!(prefs.hyperspace_effects().label(), HYPERSPACE_EFFECTS_LABEL);
        assert!(prefs.hyperspace_effects().enabled(), "{}", path.display());
        assert_eq!(prefs.quicktime_movies().label(), "QuickTime Movies");
        assert!(prefs.quicktime_movies().enabled(), "{}", path.display());
        assert_eq!(prefs.smoke_trails().label(), "Smoke Trails");
        assert!(prefs.smoke_trails().enabled(), "{}", path.display());
        assert_eq!(prefs.weapon_effects().label(), "Weapon Effects");
        assert!(prefs.weapon_effects().enabled(), "{}", path.display());
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
        for toggle in prefs.inert().iter().chain([
            prefs.hyperspace_effects(),
            prefs.quicktime_movies(),
            prefs.smoke_trails(),
            prefs.weapon_effects(),
        ]) {
            let rect = toggle.rect();
            let apart = rect.max.x <= music.min.x
                || rect.min.x >= music.max.x
                || rect.max.y <= music.min.y
                || rect.min.y >= music.max.y;
            assert!(apart, "{name}: {}", toggle.label());
        }
        assert_eq!(
            prefs.inert().len(),
            5,
            "{name}: QuickTime Movies, Smoke Trails and Weapon Effects work, Run in a \
             window is the display picker and Share Processor Time the Scaling list"
        );
        assert!(
            prefs.inert().iter().all(|toggle| {
                toggle.label() != "Run in a window" && toggle.label() != "Share Processor Time"
            }),
            "{name}"
        );
        // The display picker fills "Run in a window"'s row, clear of every
        // other part, inside the dialog.
        let picker = prefs.display().rects().bounds();
        let row = dialog.item_bounds(11).expect("Run in a window");
        assert_eq!(
            (picker.min.y, picker.max.y),
            (row.min.y, row.max.y),
            "{name}"
        );
        assert!(dialog.bounds().contains(picker.min) && dialog.bounds().contains(picker.max));
        for rect in prefs
            .inert()
            .iter()
            .chain([
                prefs.music(),
                prefs.sound(),
                prefs.hyperspace_effects(),
                prefs.quicktime_movies(),
                prefs.smoke_trails(),
                prefs.weapon_effects(),
            ])
            .map(nova_view::ui::toggle::Toggle::rect)
            .chain([prefs.effects_volume().rects().bounds(), music, ok, keys])
        {
            let apart = rect.max.x <= picker.min.x
                || rect.min.x >= picker.max.x
                || rect.max.y <= picker.min.y
                || rect.min.y >= picker.max.y;
            assert!(apart, "{name}: {rect:?} under the picker {picker:?}");
        }
    }
}

/// The Scaling list fills "Share Processor Time"'s row, clear of every
/// other part, inside the dialog, its label as wide as the display
/// picker's (the end-to-end preferences test checks the widest label fits
/// it under the real text metrics).
#[test]
fn the_preferences_dialogs_scaling_list_fills_share_processor_times_row() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(PREFS_DIALOG)
            .expect("converts");
        let prefs = PrefsDialog::new(
            &template,
            Prefs::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let dialog = prefs.dialog();
        let name = path.display();
        let music = prefs.music_volume().rects().bounds();
        let picker = prefs.display().rects().bounds();
        let ok = dialog.item_bounds(OK_ITEM).expect("OK");
        let keys = dialog.item_bounds(KEY_SETTINGS_ITEM).expect("Key Settings");
        let list = prefs.scaling().rects().bounds();
        let row = dialog
            .item_bounds(SCALING_ITEM)
            .expect("Share Processor Time");
        assert_eq!(
            (list.min.x, list.min.y, list.max.y),
            (row.min.x + 1.0, row.min.y, row.max.y),
            "{name}"
        );
        assert!(dialog.bounds().contains(list.min) && dialog.bounds().contains(list.max));
        assert_eq!(
            prefs.scaling().rects().value.width(),
            prefs.display().rects().value.width(),
            "{name}"
        );
        for rect in prefs
            .inert()
            .iter()
            .chain([
                prefs.music(),
                prefs.sound(),
                prefs.hyperspace_effects(),
                prefs.quicktime_movies(),
                prefs.smoke_trails(),
                prefs.weapon_effects(),
            ])
            .map(nova_view::ui::toggle::Toggle::rect)
            .chain([
                prefs.effects_volume().rects().bounds(),
                music,
                ok,
                keys,
                picker,
            ])
        {
            let apart = rect.max.x <= list.min.x
                || rect.min.x >= list.max.x
                || rect.max.y <= list.min.y
                || rect.min.y >= list.max.y;
            assert!(apart, "{name}: {rect:?} under the Scaling list {list:?}");
        }
    }
}

/// The relaunch notice's area, under "Check For Updates", is clear of
/// every part the dialog draws, inside the dialog, in each build.
#[test]
fn the_preferences_dialogs_relaunch_notice_has_its_area_to_itself() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(PREFS_DIALOG)
            .expect("converts");
        let prefs = PrefsDialog::new(
            &template,
            Prefs::default(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
        .expect("builds");
        let dialog = prefs.dialog();
        let name = path.display();
        let (left, top, right, bottom) = NOTICE_AREA;
        let origin = dialog.bounds().min;
        let notice = Bounds {
            min: Point::new(origin.x + left, origin.y + top),
            max: Point::new(origin.x + right, origin.y + bottom),
        };
        assert!(dialog.bounds().contains(notice.min) && dialog.bounds().contains(notice.max));
        let ok = dialog.item_bounds(OK_ITEM).expect("OK");
        let keys = dialog.item_bounds(KEY_SETTINGS_ITEM).expect("Key Settings");
        for rect in prefs
            .inert()
            .iter()
            .chain([
                prefs.music(),
                prefs.sound(),
                prefs.hyperspace_effects(),
                prefs.quicktime_movies(),
                prefs.smoke_trails(),
                prefs.weapon_effects(),
            ])
            .map(nova_view::ui::toggle::Toggle::rect)
            .chain([
                prefs.effects_volume().rects().bounds(),
                prefs.music_volume().rects().bounds(),
                prefs.display().rects().bounds(),
                prefs.scaling().rects().bounds(),
                ok,
                keys,
            ])
        {
            let apart = rect.max.x <= notice.min.x
                || rect.min.x >= notice.max.x
                || rect.max.y <= notice.min.y
                || rect.min.y >= notice.max.y;
            assert!(apart, "{name}: {rect:?} under the notice {notice:?}");
        }
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
        assert_eq!(
            template.items[GENDER_ITEM - 1].kind,
            ItemSpec::Control(500),
            "{name}"
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
        for shown in [
            "Create a new pilot:",
            "Full Name:",
            "OK",
            "Cancel",
            "Gender:",
            "Male",
        ] {
            assert!(texts.contains(&shown.to_owned()), "{name}: {texts:?}");
        }
        let gender = dialog
            .dialog()
            .item_bounds(GENDER_ITEM)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: gender,
            });
        }
        for c in "Ada".chars() {
            dialog.input(&Input::Text(c));
        }
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "Ada".to_owned(),
                gender: nova_sim::Gender::Female,
                strict_play: false,
            }),
            "{name}"
        );
    }
}

/// The stock dialog's Strict Play check box (item 4) is live: a click
/// ticks it, and OK creates a pilot that plays Strict Play.
#[test]
fn the_new_pilot_dialogs_strict_play_box_is_live() {
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(NEW_PILOT_DIALOG)
            .expect("converts");
        let name = path.display();
        assert_eq!(
            template.items[STRICT_PLAY_ITEM - 1].kind,
            ItemSpec::CheckBox("Strict Play".to_owned()),
            "{name}"
        );
        let mut dialog = NewPilotDialog::new(&template, ButtonStyle::STOCK, Rc::new(MonoMetrics))
            .expect("builds");
        let strict_play = dialog
            .dialog()
            .item_bounds(STRICT_PLAY_ITEM)
            .expect("an item")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: strict_play,
            });
        }
        for c in "Ada".chars() {
            dialog.input(&Input::Text(c));
        }
        dialog.input(&key(Key::Enter));
        assert_eq!(
            dialog.take_outcome(),
            Some(NewPilotOutcome::Create {
                name: "Ada".to_owned(),
                gender: nova_sim::Gender::Male,
                strict_play: true,
            }),
            "{name}: the stock box is live"
        );
    }
}

/// The metrics mock, shared.
fn mono() -> Rc<dyn nova_view::text::TextMetrics> {
    Rc::new(MonoMetrics)
}

#[test]
fn the_mission_computer_lays_out_with_its_items_where_recorded() {
    use nova_sim::{ComputerRow, MissionComputer, MissionId};
    use nova_view::spaceport::missions::{
        ACCEPT_ITEM, COMPUTER_DIALOG, DATE_ITEM, HEADING_ITEM, LEAVE_ITEM, LIST_ITEM,
        MissionComputerScreen, NAME_ITEM, TEXT_ITEM as COMPUTER_TEXT_ITEM,
    };
    for (_, path) in builds() {
        let computer = interface(&path)
            .dialog_template(COMPUTER_DIALOG)
            .expect("converts");
        assert_eq!(
            (computer.bounds.width(), computer.bounds.height()),
            (510.0, 201.0)
        );
        assert_eq!(computer.placement, Placement::Center);
        for (item, bounds) in [
            (ACCEPT_ITEM, rect(266.0, 170.0, 99.0, 25.0)),
            (LIST_ITEM, rect(10.0, 30.0, 195.0, 144.0)),
            (COMPUTER_TEXT_ITEM, rect(233.0, 60.0, 267.0, 93.0)),
            (NAME_ITEM, rect(233.0, 34.0, 269.0, 21.0)),
            (LEAVE_ITEM, rect(368.0, 170.0, 99.0, 25.0)),
            (HEADING_ITEM, rect(14.0, 3.0, 396.0, 15.0)),
            (DATE_ITEM, rect(311.0, 0.0, 87.0, 15.0)),
        ] {
            assert_eq!(computer.items[item - 1].bounds, bounds, "item {item}");
        }
        let screen = MissionComputerScreen::new(
            Ok((computer, mono())),
            MissionComputer {
                rows: vec![ComputerRow {
                    mission: MissionId(128),
                    name: "Run".to_owned(),
                    text: "Go.".to_owned(),
                }],
                today: "June 23rd, 1177 NC".to_owned(),
            },
            ButtonStyle::STOCK,
        );
        let dialog = screen.dialog().expect("laid out");
        for parked in [6, 9, 10] {
            assert!(!dialog.item_shown(parked), "item {parked} lies outside");
        }
    }
}

#[test]
fn the_mission_info_screen_lays_out_with_its_items_where_recorded() {
    use nova_sim::{InfoRow, MissionInfo};
    use nova_view::spaceport::missions::{
        ABORT_ITEM, INFO_DIALOG, INFO_DONE_ITEM, INFO_LIST_ITEM, INFO_TEXT_ITEM, MissionInfoScreen,
    };
    for (_, path) in builds() {
        let info = interface(&path)
            .dialog_template(INFO_DIALOG)
            .expect("converts");
        assert_eq!((info.bounds.width(), info.bounds.height()), (471.0, 155.0));
        for (item, bounds) in [
            (INFO_DONE_ITEM, rect(290.0, 125.0, 99.0, 25.0)),
            (INFO_LIST_ITEM, rect(9.0, 24.0, 195.0, 84.0)),
            (INFO_TEXT_ITEM, rect(218.0, 26.0, 242.0, 91.0)),
            (ABORT_ITEM, rect(57.0, 125.0, 99.0, 25.0)),
        ] {
            assert_eq!(info.items[item - 1].bounds, bounds, "item {item}");
        }
        let screen = MissionInfoScreen::new(
            Ok((info, mono())),
            MissionInfo {
                rows: vec![InfoRow {
                    slot: 0,
                    name: "Run".to_owned(),
                    failed: false,
                    text: "Go.".to_owned(),
                    abortable: true,
                }],
                today: "June 23rd, 1177 NC".to_owned(),
            },
            ButtonStyle::STOCK,
        );
        let dialog = screen.dialog().expect("laid out");
        assert!(!dialog.item_shown(6), "the map lies outside");
    }
}

#[test]
fn the_offer_dialog_lays_out_with_its_items_where_recorded_its_buttons_enabled() {
    use nova_sim::mission::OfferLabels;
    use nova_sim::{MissionId, OfferPrompt};
    use nova_view::spaceport::OfferChoice;
    use nova_view::spaceport::offer::{
        ACCEPT_ITEM as OFFER_ACCEPT_ITEM, OFFER_DIALOG, OfferDialog, REFUSE_ITEM, SINGLE_ITEM,
        TEXT_ITEM as OFFER_TEXT_ITEM,
    };
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(OFFER_DIALOG)
            .expect("converts");
        assert_eq!(
            (template.bounds.width(), template.bounds.height()),
            (441.0, 317.0)
        );
        for (item, bounds) in [
            (OFFER_ACCEPT_ITEM, rect(225.0, 285.0, 99.0, 25.0)),
            (REFUSE_ITEM, rect(117.0, 285.0, 99.0, 25.0)),
            (OFFER_TEXT_ITEM, rect(12.0, 9.0, 415.0, 267.0)),
            (SINGLE_ITEM, rect(173.0, 285.0, 99.0, 25.0)),
        ] {
            assert_eq!(template.items[item - 1].bounds, bounds, "item {item}");
        }
        let mut dialog = OfferDialog::new(
            Ok((template, mono())),
            OfferPrompt {
                mission: MissionId(128),
                text: "Go.".to_owned(),
                graphic: None,
                labels: OfferLabels {
                    accept: "Yes".to_owned(),
                    refuse: Some("No".to_owned()),
                },
                movie: None,
            },
            ButtonStyle::STOCK,
        );
        let refuse = dialog
            .dialog()
            .and_then(|laid| laid.item_bounds(REFUSE_ITEM))
            .expect("refuse")
            .center();
        for pressed in [true, false] {
            dialog.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at: refuse,
            });
        }
        assert_eq!(
            dialog.take_choice(),
            Some(OfferChoice::Refuse),
            "the stock refuse button, enabled"
        );
    }
}

#[test]
fn the_mission_screens_pictures_decode_at_their_dialogs_sizes() {
    use nova_view::spaceport::missions::{COMPUTER_BACKGROUND, INFO_BACKGROUND};
    use nova_view::spaceport::offer::FRAME as OFFER_FRAME;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let size = |key: ImageKey| {
        let image = picture(&data, key.id);
        (image.width(), image.height())
    };
    assert_eq!(size(COMPUTER_BACKGROUND), (510, 201));
    assert_eq!(size(INFO_BACKGROUND), (471, 155));
    assert_eq!(size(OFFER_FRAME.top), (441, 9));
    assert_eq!(size(OFFER_FRAME.middle), (441, 365));
    assert_eq!(size(OFFER_FRAME.bottom), (441, 40));
    assert_eq!(
        (OFFER_FRAME.top_height, OFFER_FRAME.bottom_height),
        (9.0, 40.0)
    );
}

#[test]
fn the_picture_variants_lay_out_with_their_items_where_recorded() {
    use nova_view::spaceport::bar::{BAR_PICTURE_ITEM, PICTURE_BAR_DIALOG};
    use nova_view::spaceport::offer::{OFFER_PICTURE_ITEM, PICTURE_OFFER_DIALOG};
    use nova_view::ui::desc::{DESC_PICTURE_ITEM, PICTURE_DESC_DIALOG};
    for (_, path) in builds() {
        let ui = interface(&path);
        for (id, size, items) in [
            (
                PICTURE_OFFER_DIALOG,
                (651.0, 244.0),
                vec![
                    (1, rect(221.0, 213.0, 99.0, 25.0)),
                    (2, rect(115.0, 213.0, 99.0, 25.0)),
                    (3, rect(13.0, 7.0, 413.0, 200.0)),
                    (6, rect(170.0, 213.0, 99.0, 25.0)),
                    (OFFER_PICTURE_ITEM, rect(441.0, 7.0, 200.0, 200.0)),
                ],
            ),
            (
                PICTURE_DESC_DIALOG,
                (648.0, 245.0),
                vec![
                    (DONE_ITEM, rect(173.0, 214.0, 99.0, 25.0)),
                    (DESC_PICTURE_ITEM, rect(439.0, 7.0, 200.0, 200.0)),
                    (TEXT_ITEM, rect(12.0, 7.0, 415.0, 201.0)),
                ],
            ),
            (
                PICTURE_BAR_DIALOG,
                (266.0, 306.0),
                vec![
                    (1, rect(157.0, 276.0, 99.0, 25.0)),
                    (2, rect(157.0, 247.0, 99.0, 25.0)),
                    (3, rect(7.0, 275.0, 146.0, 26.0)),
                    (5, rect(7.0, 246.0, 146.0, 26.0)),
                    (7, rect(11.0, 128.0, 242.0, 114.0)),
                    (BAR_PICTURE_ITEM, rect(3.0, 3.0, 260.0, 120.0)),
                ],
            ),
        ] {
            let template = ui.dialog_template(id).expect("converts");
            assert_eq!(
                (template.bounds.width(), template.bounds.height()),
                size,
                "DLOG {id}"
            );
            assert_eq!(template.placement, Placement::Center, "DLOG {id}");
            for (item, bounds) in items {
                assert_eq!(
                    template.items[item - 1].bounds,
                    bounds,
                    "DLOG {id} item {item}"
                );
            }
        }
    }
}

#[test]
fn the_picture_variants_frames_decode_at_their_sizes() {
    use nova_view::spaceport::bar::PICTURE_BACKGROUND;
    use nova_view::spaceport::offer::PICTURE_FRAME as OFFER_PICTURE_FRAME;
    use nova_view::ui::desc::PICTURE_FRAME as DESC_PICTURE_FRAME;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let size = |key: ImageKey| {
        let image = picture(&data, key.id);
        (image.width(), image.height())
    };
    assert_eq!(
        size(OFFER_PICTURE_FRAME),
        (649, 244),
        "Single mission + pict"
    );
    assert_eq!(size(DESC_PICTURE_FRAME), (649, 244), "Desc + pict");
    assert_eq!(size(PICTURE_BACKGROUND), (266, 306), "Bar + pict");
}

#[test]
fn player_info_lays_out_its_seven_items_where_recorded_over_its_pictures() {
    use nova_sim::PlayerInfo;
    use nova_view::ui::player_info::{
        BACKGROUND, CAP_HEIGHT, PLAYER_INFO_DIALOG, PlayerInfoButton, PlayerInfoScreen, Tab,
        YES_NO_DIALOG,
    };
    for (dir, path) in builds() {
        let ui = interface(&path);
        let template = ui.dialog_template(PLAYER_INFO_DIALOG).expect("converts");
        assert_eq!(
            (template.bounds.width(), template.bounds.height()),
            (413.0, 227.0)
        );
        assert_eq!(template.placement, Placement::Center);
        assert_eq!(template.items.len(), 7);
        for (item, bounds) in [
            (1, ltrb(293.0, 195.0, 392.0, 220.0)),
            (2, ltrb(7.0, 8.0, 106.0, 33.0)),
            (3, ltrb(107.0, 8.0, 206.0, 33.0)),
            (4, ltrb(207.0, 8.0, 306.0, 33.0)),
            (5, ltrb(307.0, 8.0, 406.0, 33.0)),
            (6, ltrb(4.0, 40.0, 409.0, 181.0)),
            (7, ltrb(60.0, 195.0, 210.0, 220.0)),
        ] {
            assert_eq!(template.items[item - 1].bounds, bounds, "item {item}");
        }
        assert!(!template.items[5].enabled, "the content box is disabled");
        let confirm = ui.dialog_template(YES_NO_DIALOG);
        assert!(confirm.is_ok(), "{confirm:?}");
        let info = PlayerInfo {
            pilot_name: "Ada".to_owned(),
            date: String::new(),
            system: String::new(),
            legal_status: String::new(),
            ship_name: String::new(),
            ship_class: String::new(),
            turn_rate: 0,
            accel: 0,
            max_speed: 0,
            credits: 0,
            shield: None,
            armor: None,
            energy: None,
            income: None,
            fleet: false,
            cargo: None,
            extras: None,
            honors: None,
            jettisonable: false,
            words: nova_sim::player_info::Words::STOCK,
        };
        let screen =
            PlayerInfoScreen::new(Ok((template, mono())), confirm, info, ButtonStyle::STOCK);
        assert_eq!(screen.problem(), None);
        assert_eq!(screen.tab(), Tab::General);
        let bounds = screen.bounds().expect("laid out");
        assert_eq!(bounds.min, Point::new(305.0, 270.0), "centred");
        assert_eq!(
            screen.button_bounds(PlayerInfoButton::Done),
            Some(ltrb(598.0, 465.0, 697.0, 490.0))
        );
        let data = GameData::open(&dir, None).expect("the stock data opens");
        let size = |key: ImageKey| {
            let image = picture(&data, key.id);
            (image.width(), image.height())
        };
        assert_eq!(size(BACKGROUND[0]), (413, 40), "upper");
        assert_eq!(size(BACKGROUND[1]), (413, 215), "middle");
        assert_eq!(size(BACKGROUND[2]), (413, 40), "lower");
        assert_eq!(CAP_HEIGHT, 40.0);
    }
}

#[test]
fn the_maps_find_dialog_lays_out_from_dlog_3005_as_recorded() {
    use nova_view::galaxy::FindDialog;
    use nova_view::galaxy::find::{CANCEL_ITEM, FIELD_ITEM, FIND_DIALOG, FIND_ITEM, PROMPT_ITEM};
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(FIND_DIALOG)
            .expect("converts");
        assert_eq!(template.placement, Placement::Center, "{}", path.display());
        let dialog = FindDialog::new(&template, ButtonStyle::STOCK, mono()).expect("builds");
        let bounds = dialog.dialog().bounds();
        assert_eq!((bounds.width(), bounds.height()), (284.0, 105.0));
        assert_eq!(bounds.min, Point::new(370.0, 331.0), "centred");
        let at = |x, y, w, h| Some(rect(x, y, w, h).offset(bounds.min));
        let item = |n| dialog.dialog().item_bounds(n);
        assert_eq!(item(FIND_ITEM), at(165.0, 72.0, 70.0, 20.0), "Find");
        assert_eq!(item(CANCEL_ITEM), at(83.0, 72.0, 70.0, 20.0), "Cancel");
        assert_eq!(item(FIELD_ITEM), at(60.0, 38.0, 200.0, 16.0), "the field");
        assert_eq!(dialog.field().text(), "");
        let prompt = match &template.items[PROMPT_ITEM - 1].kind {
            ItemSpec::StaticText(text) => text.clone(),
            kind => panic!("{kind:?}"),
        };
        assert_eq!(prompt, nova_view::galaxy::find::PROMPT);
        let texts: Vec<String> = drawn(&dialog)
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        for shown in ["Find", "Cancel", nova_view::galaxy::find::PROMPT] {
            assert!(texts.contains(&shown.to_owned()), "{shown}: {texts:?}");
        }
    }
}

#[test]
fn the_maps_dlog_2000_is_601_by_513_with_its_items_where_the_map_puts_them() {
    use nova_view::galaxy::buttons::MapButton;
    use nova_view::galaxy::dialog::{DIALOG_SIZE, MAP_DIALOG, MAP_ITEM};
    use nova_view::galaxy::panel::{INFO_COLUMN, INFO_STRIP, placed};
    for (_, path) in builds() {
        let template = interface(&path)
            .dialog_template(MAP_DIALOG)
            .expect("converts");
        let name = path.display();
        let size = (template.bounds.width(), template.bounds.height());
        assert_eq!(size, DIALOG_SIZE, "{name}");
        assert_eq!(template.items.len(), 10, "{name}");
        let corner = Point::new(0.0, 0.0);
        let item = |n: usize| template.items[n - 1].bounds;
        let expected = [
            (1, placed(MapButton::Done.item(), corner)),
            (2, placed(INFO_STRIP, corner)),
            (3, placed(MAP_ITEM, corner)),
            (4, placed(MapButton::ZoomOut.item(), corner)),
            (5, placed(MapButton::ZoomIn.item(), corner)),
            (6, placed(INFO_COLUMN, corner)),
            (8, placed(MapButton::ClearRoute.item(), corner)),
            (9, placed(MapButton::Borders.item(), corner)),
            (10, placed(MapButton::Find.item(), corner)),
        ];
        for (n, bounds) in expected {
            assert_eq!(item(n), bounds, "{name}: item {n}");
            assert!(template.items[n - 1].enabled, "{name}: item {n}");
            assert!(
                matches!(template.items[n - 1].kind, ItemSpec::User),
                "{name}: item {n} is a user item"
            );
        }
        let seventh = &template.items[6];
        assert!(!seventh.enabled, "{name}: item 7 is disabled");
        assert!(
            seventh.bounds.min.y > DIALOG_SIZE.1,
            "{name}: item 7 is off the dialog"
        );
    }
}

/// Every dialog that opens over flight centres on the screen, so in a
/// widened flight view each lands where the original's `_CenterWindow`
/// puts it once its 1024x768 frame is centred: none keeps its `DLOG`
/// bounds. Comm 1007, haggle 1008, plunder 1011, mission info 1012, the
/// offers 1016 and 1020, player info 1017, assignment 1018, the map 2000,
/// yes/no 3002, the texts 3003 and 3004, Find 3005 and the preferences
/// 4003.
#[test]
fn every_dialog_opened_over_flight_is_centred() {
    for (_, path) in builds() {
        let ui = interface(&path);
        for id in [
            1007, 1008, 1011, 1012, 1016, 1017, 1018, 1020, 2000, 3002, 3003, 3004, 3005, 4003,
        ] {
            let template = ui.dialog_template(id).expect("converts");
            assert_eq!(
                template.placement,
                Placement::Center,
                "{} DLOG {id}",
                path.display()
            );
        }
    }
}
