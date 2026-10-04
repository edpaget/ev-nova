//! The widgets' ports over `nova_data`: dialog templates from the
//! interface file ([`InterfaceData`]), and descriptions and the button
//! style from the game data ([`GameData`]). Thin mappings; the decisions
//! are the core's.

use nova_data::records::colors::Colors;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::{DialogItem, ItemKind};
use nova_data::{GameData, InterfaceData, Rect};

use super::button::ButtonStyle;
use super::catalog::{DescriptionSource, DialogResources};
use super::dialog::{DialogTemplate, ItemSpec, ItemTemplate, Placement};
use crate::color::Color;
use crate::font::Font;
use crate::geometry::{Bounds, Point};

/// The `cölr` the button style comes from.
const COLORS_ID: i16 = 128;

impl DialogResources for InterfaceData {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        let dlog = match self.dialog(id) {
            None => return Err(format!("no DLOG {id}")),
            Some(Err(err)) => return Err(err.to_string()),
            Some(Ok((dlog, _))) => dlog.record,
        };
        let items = match self.items(dlog.items_id) {
            None => {
                return Err(format!(
                    "no DITL {} (the items of DLOG {id})",
                    dlog.items_id.0
                ));
            }
            Some(Err(err)) => return Err(err.to_string()),
            Some(Ok((items, _))) => items.record.items,
        };
        Ok(template(&dlog, items))
    }
}

/// `dlog` with `items`, in the core's terms.
fn template(dlog: &Dlog, items: Vec<DialogItem>) -> DialogTemplate {
    DialogTemplate {
        bounds: bounds(dlog.bounds),
        placement: Placement::from_word(dlog.position),
        items: items.into_iter().map(item).collect(),
    }
}

fn item(item: DialogItem) -> ItemTemplate {
    let text = |text: nova_data::MacString| text.as_str().to_owned();
    ItemTemplate {
        bounds: bounds(item.bounds),
        enabled: item.enabled,
        kind: match item.kind {
            ItemKind::User => ItemSpec::User,
            ItemKind::Button { title } => ItemSpec::Button(text(title)),
            ItemKind::CheckBox { title } => ItemSpec::CheckBox(text(title)),
            ItemKind::RadioButton { title } => ItemSpec::Radio(text(title)),
            ItemKind::Control { cntl_id } => ItemSpec::Control(cntl_id),
            ItemKind::StaticText { text: body } => ItemSpec::StaticText(text(body)),
            ItemKind::EditText { text: body } => ItemSpec::EditText(text(body)),
            ItemKind::Icon { icon_id } => ItemSpec::Icon(icon_id),
            ItemKind::Picture { pict } => ItemSpec::Picture(pict.0),
            ItemKind::Other { .. } => ItemSpec::Other,
        },
    }
}

fn bounds(rect: Rect) -> Bounds {
    Bounds {
        min: Point::new(f32::from(rect.left), f32::from(rect.top)),
        max: Point::new(f32::from(rect.right), f32::from(rect.bottom)),
    }
}

impl DescriptionSource for GameData {
    fn description(&self, id: i16) -> Result<String, String> {
        match self.get::<Desc>(id) {
            None => Err(format!("no dësc {id}")),
            Some(Err(err)) => Err(err.to_string()),
            Some(Ok(desc)) => Ok(desc.record.text.as_str().to_owned()),
        }
    }

    fn button_style(&self) -> ButtonStyle {
        let Some(Ok(colors)) = self.get::<Colors>(COLORS_ID) else {
            return ButtonStyle::STOCK;
        };
        let colors = colors.record;
        let size = match colors.button_font_sz {
            size if size > 0 => f32::from(size),
            _ => ButtonStyle::STOCK.size,
        };
        ButtonStyle {
            font: Font::named(colors.button_font.as_str()),
            size,
            up: Color::from_rgb24(colors.button_up),
            down: Color::from_rgb24(colors.button_down),
            grey: Color::from_rgb24(colors.button_grey),
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::io;
    use std::path::Path;

    use nova_data::Record;
    use nova_data::store::fs::{DirLister, EntryKind, Listing};
    use nova_rsrc::fixture::ForkBuilder;
    use nova_rsrc::{Fork, ForkReader, ResType};

    use super::*;
    use crate::ui::dialog::ItemSpec;

    /// One file, holding a fork, wherever it is looked for.
    struct OneFile(Vec<u8>);

    impl DirLister for OneFile {
        fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
            Ok(vec![Listing {
                name: "Nova Data".into(),
                kind: EntryKind::File,
            }])
        }
    }

    impl ForkReader for OneFile {
        fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
            Ok((fork == Fork::Data).then(|| self.0.clone()))
        }
    }

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> OneFile {
        let bytes = resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes;
        OneFile(bytes)
    }

    fn interface(resources: &[(ResType, i16, Vec<u8>)]) -> InterfaceData {
        InterfaceData::load(&fork(resources), Path::new("/Nova-DF.rsrc")).expect("loads")
    }

    fn game(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        let file = fork(resources);
        GameData::load(&file, &file, Path::new("/data"), None).expect("opens")
    }

    fn be(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_be_bytes()).collect()
    }

    /// A `DLOG` with bounds (top, left, bottom, right) naming `DITL`
    /// `items`, then `tail` (the pad and positioning word, if any).
    fn dlog(bounds: [i16; 4], items: i16, tail: &[u8]) -> Vec<u8> {
        let mut bytes = be(&bounds);
        bytes.extend([0, 2, 1, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend(items.to_be_bytes());
        bytes.push(0);
        bytes.extend(tail);
        bytes
    }

    /// One `DITL` item: bounds (top, left, bottom, right), type byte and
    /// data.
    fn ditl_item(bounds: [i16; 4], type_byte: u8, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 4];
        bytes.extend(be(&bounds));
        bytes.push(type_byte);
        bytes.push(data.len() as u8);
        bytes.extend(data);
        if data.len() % 2 == 1 {
            bytes.push(0);
        }
        bytes
    }

    fn ditl(items: &[Vec<u8>]) -> Vec<u8> {
        let mut bytes = (items.len() as i16 - 1).to_be_bytes().to_vec();
        bytes.extend(items.concat());
        bytes
    }

    /// "Desc Dialog"'s shape: a 441 x 313 `DLOG` at (24, 18), centred,
    /// and its six items.
    fn desc_dialog() -> Vec<(ResType, i16, Vec<u8>)> {
        let items = ditl(&[
            ditl_item([281, 173, 306, 272], 0, &[]),
            ditl_item([329, 221, 359, 289], 0x80 | 64, &1431_i16.to_be_bytes()),
            ditl_item([10, 11, 272, 428], 0x80, &[]),
            ditl_item([345, 70, 370, 148], 4, b"OK"),
            ditl_item([1, 2, 3, 4], 0x80 | 8, b"Hi\rthere"),
            ditl_item([1, 2, 3, 4], 5, b"Check"),
            ditl_item([1, 2, 3, 4], 6, b"Radio"),
            ditl_item([1, 2, 3, 4], 7, &300_i16.to_be_bytes()),
            ditl_item([1, 2, 3, 4], 16, b"Edit"),
            ditl_item([1, 2, 3, 4], 32, &128_i16.to_be_bytes()),
            ditl_item([1, 2, 3, 4], 1, &[9, 9]),
        ]);
        vec![
            (
                Dlog::TYPE,
                3003,
                dlog([24, 18, 337, 459], 3003, &[0, 0xA8, 0x0A]),
            ),
            (nova_data::records::dialog_items::Ditl::TYPE, 3003, items),
        ]
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    #[test]
    fn a_dlog_and_its_ditl_make_a_template() {
        let ui = interface(&desc_dialog());
        let template = ui.dialog_template(3003).expect("converts");
        assert_eq!(template.bounds, rect(18.0, 24.0, 441.0, 313.0));
        assert_eq!(template.placement, Placement::Center);
        let item = |bounds, enabled, kind| ItemTemplate {
            bounds,
            enabled,
            kind,
        };
        let small = rect(2.0, 1.0, 2.0, 2.0);
        assert_eq!(
            template.items,
            [
                item(rect(173.0, 281.0, 99.0, 25.0), true, ItemSpec::User),
                item(
                    rect(221.0, 329.0, 68.0, 30.0),
                    false,
                    ItemSpec::Picture(1431)
                ),
                item(rect(11.0, 10.0, 417.0, 262.0), false, ItemSpec::User),
                item(
                    rect(70.0, 345.0, 78.0, 25.0),
                    true,
                    ItemSpec::Button("OK".into())
                ),
                item(small, false, ItemSpec::StaticText("Hi\rthere".into())),
                item(small, true, ItemSpec::CheckBox("Check".into())),
                item(small, true, ItemSpec::Radio("Radio".into())),
                item(small, true, ItemSpec::Control(300)),
                item(small, true, ItemSpec::EditText("Edit".into())),
                item(small, true, ItemSpec::Icon(128)),
                item(small, true, ItemSpec::Other),
            ]
        );
    }

    #[test]
    fn a_dlog_with_no_position_word_keeps_its_bounds() {
        let ui = interface(&[
            (Dlog::TYPE, 128, dlog([40, 30, 140, 230], 129, &[])),
            (nova_data::records::dialog_items::Ditl::TYPE, 129, ditl(&[])),
        ]);
        let template = ui.dialog_template(128).expect("converts");
        assert_eq!(template.placement, Placement::Fixed);
        assert_eq!(template.bounds, rect(30.0, 40.0, 200.0, 100.0));
        assert!(template.items.is_empty());
    }

    #[test]
    fn a_missing_dlog_or_ditl_is_an_error() {
        let ui = interface(&[(Dlog::TYPE, 128, dlog([0, 0, 10, 10], 129, &[]))]);
        assert_eq!(ui.dialog_template(3003), Err("no DLOG 3003".to_owned()));
        assert_eq!(
            ui.dialog_template(128),
            Err("no DITL 129 (the items of DLOG 128)".to_owned())
        );
    }

    #[test]
    fn an_undecodable_dlog_or_ditl_is_its_decode_error() {
        let ditl_type = nova_data::records::dialog_items::Ditl::TYPE;
        let ui = interface(&[
            (Dlog::TYPE, 128, vec![0; 4]),
            (Dlog::TYPE, 129, dlog([0, 0, 10, 10], 129, &[])),
            (ditl_type, 129, vec![0, 0]),
        ]);
        let dlog_err = ui.dialog_template(128).expect_err("bad DLOG");
        let expected = ui.dialog(128).expect("present").expect_err("bad");
        assert_eq!(dlog_err, expected.to_string());
        let ditl_err = ui.dialog_template(129).expect_err("bad DITL");
        let expected = ui
            .items(nova_data::DitlId(129))
            .expect("present")
            .expect_err("bad");
        assert_eq!(ditl_err, expected.to_string());
    }

    /// A `cölr` with these button colours, font and size.
    fn colors(up: u32, down: u32, grey: u32, font: &str, size: i16) -> Vec<u8> {
        let mut bytes = vec![0; Colors::SIZE.expect("fixed")];
        bytes[0x00..0x04].copy_from_slice(&up.to_be_bytes());
        bytes[0x04..0x08].copy_from_slice(&down.to_be_bytes());
        bytes[0x08..0x0C].copy_from_slice(&grey.to_be_bytes());
        bytes[0x9E..0x9E + font.len()].copy_from_slice(font.as_bytes());
        bytes[0xDE..0xE0].copy_from_slice(&size.to_be_bytes());
        bytes
    }

    #[test]
    fn the_button_style_comes_from_colr_128() {
        let data = game(&[(
            Colors::TYPE,
            128,
            colors(0x0011_2233, 0x0044_5566, 0x0077_8899, "Geneva", 9),
        )]);
        assert_eq!(
            data.button_style(),
            ButtonStyle {
                font: Font::Geneva,
                size: 9.0,
                up: Color::rgba(0x11, 0x22, 0x33, 255),
                down: Color::rgba(0x44, 0x55, 0x66, 255),
                grey: Color::rgba(0x77, 0x88, 0x99, 255),
            }
        );
        let charcoal = game(&[(Colors::TYPE, 128, colors(0, 0, 0, "Charcoal", 12))]);
        assert_eq!(charcoal.button_style().font, Font::Charcoal);
    }

    #[test]
    fn a_size_that_is_not_positive_is_the_stock_size() {
        for size in [0, -3] {
            let data = game(&[(Colors::TYPE, 128, colors(0, 0, 0, "Charcoal", size))]);
            assert_eq!(data.button_style().size, 12.0, "{size}");
        }
        let one = game(&[(Colors::TYPE, 128, colors(0, 0, 0, "Charcoal", 1))]);
        assert_eq!(one.button_style().size, 1.0);
    }

    #[test]
    fn a_missing_or_bad_colr_is_the_stock_style() {
        assert_eq!(game(&[]).button_style(), ButtonStyle::STOCK);
        let other = game(&[(Colors::TYPE, 129, colors(0, 0, 0, "Geneva", 9))]);
        assert_eq!(other.button_style(), ButtonStyle::STOCK);
        let bad = game(&[(Colors::TYPE, 128, vec![0; 3])]);
        assert_eq!(bad.button_style(), ButtonStyle::STOCK);
    }

    /// A `dësc` holding `text` and no graphic.
    fn desc(text: &[u8]) -> Vec<u8> {
        let mut bytes = text.to_vec();
        bytes.push(0);
        bytes.extend([0xFF, 0xFF]);
        bytes.extend([0; 32]);
        bytes.extend([0, 0]);
        bytes
    }

    #[test]
    fn a_description_keeps_its_line_breaks() {
        let data = game(&[(Desc::TYPE, 32767, desc(b"About\rNova\r\rCaf\x8E"))]);
        assert_eq!(
            data.description(32767),
            Ok("About\rNova\r\rCaf\u{E9}".to_owned())
        );
    }

    #[test]
    fn a_missing_or_bad_description_is_an_error() {
        let data = game(&[(Desc::TYPE, 128, vec![0xFF; 2])]);
        assert_eq!(data.description(32767), Err("no dësc 32767".to_owned()));
        let err = data.description(128).expect_err("bad");
        let expected = data.get::<Desc>(128).expect("present").expect_err("bad");
        assert_eq!(err, expected.to_string());
    }
}
