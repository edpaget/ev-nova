//! The fonts the window draws text in: the bundled fallback, and the
//! game's own Charcoal when it loaded.

use nova_data::fonts::FontError;
use nova_render::FontFaces;

/// The faces for `charcoal`, the result of loading the game's Charcoal,
/// and a warning to print, if any. A missing Charcoal is normal (the
/// Windows data has none) and draws in the fallback silently; one that
/// could not be read or is unusable draws in the fallback with a warning.
#[must_use]
pub fn game_fonts(charcoal: Result<Vec<u8>, FontError>) -> (FontFaces, Option<String>) {
    let faces = FontFaces::bundled();
    match charcoal {
        Ok(bytes) => (faces.with_charcoal(bytes), None),
        Err(FontError::Missing { .. }) => (faces, None),
        Err(error) => (
            faces,
            Some(format!(
                "nova: drawing Charcoal in the bundled font: {error}"
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use nova_data::fonts::SfntError;
    use nova_data::fonts::fixture::block_font;
    use nova_render::FALLBACK_FONT;

    use super::*;

    fn path() -> PathBuf {
        PathBuf::from("/Nova Files/../Fonts/Charcoal.ttf")
    }

    #[test]
    fn a_loaded_charcoal_is_added_without_a_warning() {
        let (faces, warning) = game_fonts(Ok(block_font("Charcoal")));
        assert_eq!(
            faces,
            FontFaces::bundled().with_charcoal(block_font("Charcoal"))
        );
        assert_eq!(&faces.fallback()[..], FALLBACK_FONT);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_missing_charcoal_falls_back_silently() {
        let (faces, warning) = game_fonts(Err(FontError::Missing { path: path() }));
        assert_eq!(faces, FontFaces::bundled());
        assert_eq!(warning, None);
    }

    #[test]
    fn an_unusable_charcoal_falls_back_with_a_warning() {
        let invalid = FontError::Invalid {
            path: path(),
            source: SfntError::NoOutlines,
        };
        let (faces, warning) = game_fonts(Err(invalid));
        assert_eq!(faces, FontFaces::bundled());
        assert_eq!(
            warning.as_deref(),
            Some(
                "nova: drawing Charcoal in the bundled font: the font \
                 /Nova Files/../Fonts/Charcoal.ttf is unusable: the font has no outlines"
            )
        );
    }

    #[test]
    fn an_unreadable_charcoal_falls_back_with_a_warning() {
        let unreadable = FontError::Io {
            path: path(),
            source: io::Error::other("disk on fire"),
        };
        let (faces, warning) = game_fonts(Err(unreadable));
        assert_eq!(faces, FontFaces::bundled());
        assert_eq!(
            warning.as_deref(),
            Some(
                "nova: drawing Charcoal in the bundled font: reading the font \
                 /Nova Files/../Fonts/Charcoal.ttf: disk on fire"
            )
        );
    }
}
