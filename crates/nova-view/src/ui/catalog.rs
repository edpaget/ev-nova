//! The widgets' ports: dialog templates from the interface file, and
//! descriptions, the graphics they name, the button style and picture
//! sizes from the game data, in the core's terms.

use std::rc::Rc;

use nova_sim::DescMovie;

use super::button::ButtonStyle;
use super::dialog::DialogTemplate;

/// The dialogs the interface file defines.
pub trait DialogResources {
    /// `DLOG` `id` with the items of the `DITL` it names; an error is a
    /// message ready to display.
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String>;
}

/// Text and styling from the game data.
pub trait DescriptionSource {
    /// `dësc` `id`'s text, with its `\r` line breaks kept; an error is a
    /// message ready to display.
    fn description(&self, id: i16) -> Result<String, String>;
    /// How button labels are set, from `cölr` 128, or
    /// [`ButtonStyle::STOCK`] when it is missing or does not decode.
    fn button_style(&self) -> ButtonStyle;
}

/// The graphics that descriptions name.
pub trait DescriptionGraphics {
    /// `dësc` `id`'s `Graphic`, the `PICT` shown with it; none when it is
    /// -1, or the `dësc` is missing or does not decode.
    fn description_graphic(&self, id: i16) -> Option<i16>;
    /// The movie `dësc` `id`'s `MovieFile` names, played as its `Flags`
    /// say ([`DescMovie::from_desc`]); none when it names none, or the
    /// `dësc` is missing or does not decode.
    fn description_movie(&self, id: i16) -> Option<DescMovie>;
}

/// The sizes of the pictures a widget lays itself out around, such as
/// the player info's background.
pub trait PictureSizes {
    /// `PICT` `id`'s width and height; none when it is missing or does
    /// not decode.
    fn picture_size(&self, id: i16) -> Option<(u32, u32)>;
}

/// Borrowed picture sizes are picture sizes.
impl<T: PictureSizes + ?Sized> PictureSizes for &T {
    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

/// Shared picture sizes are picture sizes, so the app can read them from
/// the game data it holds once.
impl<T: PictureSizes + ?Sized> PictureSizes for Rc<T> {
    fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
        (**self).picture_size(id)
    }
}

/// Borrowed dialog resources are dialog resources.
impl<T: DialogResources + ?Sized> DialogResources for &T {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        (**self).dialog_template(id)
    }
}

/// Shared dialog resources are dialog resources, so the app can hold the
/// interface file once.
impl<T: DialogResources + ?Sized> DialogResources for Rc<T> {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        (**self).dialog_template(id)
    }
}

/// A borrowed source is a source.
impl<T: DescriptionSource + ?Sized> DescriptionSource for &T {
    fn description(&self, id: i16) -> Result<String, String> {
        (**self).description(id)
    }

    fn button_style(&self) -> ButtonStyle {
        (**self).button_style()
    }
}

/// A shared source is a source, so a screen and the renderer can read the
/// same game data.
impl<T: DescriptionSource + ?Sized> DescriptionSource for Rc<T> {
    fn description(&self, id: i16) -> Result<String, String> {
        (**self).description(id)
    }

    fn button_style(&self) -> ButtonStyle {
        (**self).button_style()
    }
}

/// Borrowed description graphics are description graphics.
impl<T: DescriptionGraphics + ?Sized> DescriptionGraphics for &T {
    fn description_graphic(&self, id: i16) -> Option<i16> {
        (**self).description_graphic(id)
    }

    fn description_movie(&self, id: i16) -> Option<DescMovie> {
        (**self).description_movie(id)
    }
}

/// Shared description graphics are description graphics.
impl<T: DescriptionGraphics + ?Sized> DescriptionGraphics for Rc<T> {
    fn description_graphic(&self, id: i16) -> Option<i16> {
        (**self).description_graphic(id)
    }

    fn description_movie(&self, id: i16) -> Option<DescMovie> {
        (**self).description_movie(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::geometry::{Bounds, Point};
    use crate::ui::dialog::Placement;

    /// Every dialog is empty and sized by its ID; every description is
    /// its ID; the style is stock but black.
    struct Fake;

    impl DialogResources for Fake {
        fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
            if id < 0 {
                return Err(format!("no DLOG {id}"));
            }
            Ok(DialogTemplate {
                bounds: Bounds::at(Point::new(0.0, 0.0), f32::from(id), 1.0),
                placement: Placement::Center,
                items: Vec::new(),
            })
        }
    }

    impl DescriptionSource for Fake {
        fn description(&self, id: i16) -> Result<String, String> {
            Ok(id.to_string())
        }

        fn button_style(&self) -> ButtonStyle {
            ButtonStyle {
                up: Color::BLACK,
                ..ButtonStyle::STOCK
            }
        }
    }

    fn width(resources: impl DialogResources, id: i16) -> Result<f32, String> {
        Ok(resources.dialog_template(id)?.bounds.width())
    }

    impl DescriptionGraphics for Fake {
        fn description_graphic(&self, id: i16) -> Option<i16> {
            (id > 0).then_some(id + 1)
        }

        fn description_movie(&self, id: i16) -> Option<DescMovie> {
            DescMovie::from_desc(&format!("{id}.mov"), 0x0001)
        }
    }

    fn movie(graphics: impl DescriptionGraphics, id: i16) -> Option<String> {
        graphics.description_movie(id).map(|movie| movie.file)
    }

    fn graphic(graphics: impl DescriptionGraphics, id: i16) -> Option<i16> {
        graphics.description_graphic(id)
    }

    fn read(source: impl DescriptionSource) -> (Result<String, String>, Color) {
        (source.description(7), source.button_style().up)
    }

    #[test]
    fn borrowed_and_shared_resources_are_resources() {
        assert_eq!(width(&Fake, 30), Ok(30.0));
        assert_eq!(width(Rc::new(Fake), 40), Ok(40.0));
        assert_eq!(width(Rc::new(Fake), -1), Err("no DLOG -1".to_owned()));
    }

    #[test]
    fn borrowed_and_shared_graphics_are_graphics() {
        assert_eq!(graphic(&Fake, 13000), Some(13001));
        assert_eq!(graphic(Rc::new(Fake), 14000), Some(14001));
        assert_eq!(graphic(Rc::new(Fake), -1), None);
    }

    #[test]
    fn borrowed_and_shared_graphics_read_movies() {
        assert_eq!(movie(&Fake, 13), Some("13.mov".to_owned()));
        assert_eq!(movie(Rc::new(Fake), 14), Some("14.mov".to_owned()));
    }

    impl PictureSizes for Fake {
        fn picture_size(&self, id: i16) -> Option<(u32, u32)> {
            (id > 0).then_some((id.unsigned_abs().into(), 2))
        }
    }

    fn size(pictures: impl PictureSizes, id: i16) -> Option<(u32, u32)> {
        pictures.picture_size(id)
    }

    #[test]
    fn borrowed_and_shared_picture_sizes_are_picture_sizes() {
        assert_eq!(size(&Fake, 413), Some((413, 2)));
        assert_eq!(size(Rc::new(Fake), 300), Some((300, 2)));
        assert_eq!(size(Rc::new(Fake), -1), None);
    }

    #[test]
    fn borrowed_and_shared_sources_are_sources() {
        assert_eq!(read(&Fake), (Ok("7".to_owned()), Color::BLACK));
        assert_eq!(read(Rc::new(Fake)), (Ok("7".to_owned()), Color::BLACK));
    }
}
