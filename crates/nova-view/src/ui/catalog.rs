//! The widgets' ports: dialog templates from the interface file, and
//! descriptions and the button style from the game data, in the core's
//! terms.

use std::rc::Rc;

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
    fn borrowed_and_shared_sources_are_sources() {
        assert_eq!(read(&Fake), (Ok("7".to_owned()), Color::BLACK));
        assert_eq!(read(Rc::new(Fake)), (Ok("7".to_owned()), Color::BLACK));
    }
}
