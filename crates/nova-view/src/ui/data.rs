//! The widgets' ports over `nova_data`: dialog templates from the
//! interface file ([`InterfaceData`]), and descriptions, their graphics,
//! the button style and picture sizes from the game data ([`GameData`]).
//! Thin mappings; the decisions are the core's.

use nova_data::graphics::{PICT, decode_pict};
use nova_data::records::colors::Colors;
use nova_data::records::desc::Desc;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::{DialogItem, ItemKind};
use nova_data::{GameData, InterfaceData, Rect};
use nova_sim::DescMovie;

use super::button::ButtonStyle;
use super::catalog::{DescriptionGraphics, DescriptionSource, DialogResources, PictureSizes};
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

impl DescriptionGraphics for GameData {
    fn description_graphic(&self, id: i16) -> Option<i16> {
        match self.get::<Desc>(id) {
            Some(Ok(desc)) => desc.record.graphic.map(|pict| pict.0),
            _ => None,
        }
    }

    fn description_movie(&self, id: i16) -> Option<DescMovie> {
        match self.get::<Desc>(id) {
            Some(Ok(desc)) => {
                DescMovie::from_desc(desc.record.movie_file.as_str(), desc.record.flags.0)
            }
            _ => None,
        }
    }
}

impl PictureSizes for GameData {
    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        let picture = decode_pict(self.resource(PICT, id)?.resource.data()).ok()?;
        Some((picture.width(), picture.height()))
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {

    use std::path::Path;

    use nova_data::Record;
    use nova_data::store::fixture::{one_file_at, store_of};

    use nova_rsrc::ResType;
    use nova_rsrc::fixture::ForkBuilder;

    use super::*;
    use crate::ui::dialog::ItemSpec;

    fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> Vec<u8> {
        resources
            .iter()
            .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
                fork.resource(*ty, *id, None, data)
            })
            .build()
            .bytes
    }

    fn interface(resources: &[(ResType, i16, Vec<u8>)]) -> InterfaceData {
        let path = Path::new("/Nova-DF.rsrc");
        InterfaceData::load(&one_file_at(path, fork(resources)), path).expect("loads")
    }

    fn game(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
        store_of(resources)
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

    /// A `dësc` holding `text` and `graphic`.
    fn pictured(text: &[u8], graphic: i16) -> Vec<u8> {
        let mut bytes = desc(text);
        let at = text.len() + 1;
        bytes[at..at + 2].copy_from_slice(&graphic.to_be_bytes());
        bytes
    }

    #[test]
    fn a_descriptions_graphic_is_its_pict_or_none() {
        let data = game(&[
            (Desc::TYPE, 13000, pictured(b"Shuttle", 20128)),
            (Desc::TYPE, 13001, pictured(b"Plain", -1)),
            (Desc::TYPE, 13002, vec![0xFF; 2]),
        ]);
        assert_eq!(data.description_graphic(13000), Some(20128));
        assert_eq!(data.description_graphic(13001), None, "-1 is none");
        assert_eq!(data.description_graphic(13002), None, "undecodable");
        assert_eq!(data.description_graphic(13003), None, "missing");
    }

    /// A `dësc` holding `text` and naming the movie `file`, played as
    /// `flags` say.
    fn filmed(text: &[u8], file: &[u8], flags: u16) -> Vec<u8> {
        let mut bytes = desc(text);
        let at = text.len() + 3;
        bytes[at..at + file.len()].copy_from_slice(file);
        bytes[at + 32..at + 34].copy_from_slice(&flags.to_be_bytes());
        bytes
    }

    #[test]
    fn a_descriptions_movie_is_its_movie_file_played_as_its_flags_say_or_none() {
        let data = game(&[
            (Desc::TYPE, 4778, filmed(b"Offer", b"\x01", 0x0001)),
            (Desc::TYPE, 13000, filmed(b"Race", b"Race 1.mov", 0x0006)),
            (Desc::TYPE, 13001, desc(b"Plain")),
            (Desc::TYPE, 13002, vec![0xFF; 2]),
        ]);
        assert_eq!(
            data.description_movie(13000),
            DescMovie::from_desc("Race 1.mov", 0x0006)
        );
        assert_eq!(
            data.description_movie(4778),
            DescMovie::from_desc("\u{1}", 0x0001),
            "stock mission 906's offer"
        );
        assert_eq!(data.description_movie(13001), None, "no MovieFile");
        assert_eq!(data.description_movie(13002), None, "undecodable");
        assert_eq!(data.description_movie(13003), None, "missing");
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

    fn picture(width: i16, height: i16) -> Vec<u8> {
        use nova_data::graphics::fixture::{DirectBits, PictBuilder};
        let bounds = [0, 0, height, width];
        let pixels = vec![0x4210; (width * height) as usize];
        PictBuilder::new(bounds)
            .direct_bits(&DirectBits::rgb555(bounds, &pixels))
            .end()
            .build()
    }

    #[test]
    fn a_pictures_size_is_its_decoded_width_and_height() {
        let data = game(&[(PICT, 8518, picture(300, 30)), (PICT, 8519, vec![0; 4])]);
        assert_eq!(data.picture_size(8518), Some((300, 30)));
        assert_eq!(data.picture_size(8519), None, "undecodable");
        assert_eq!(data.picture_size(8520), None, "missing");
    }
}
