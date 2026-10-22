//! The app's icon, `Contents/Resources/Nova.icns`.

/// The icon's file name in `Contents/Resources`, and `CFBundleIconFile`.
pub const ICON_FILE: &str = "Nova.icns";

/// The icon: a placeholder of original art, not the original game's icon
/// (which is copyrighted and not used). A dark navy rounded square holding
/// a light ring around a gold four-point star.
///
/// Made once on macOS, by a throwaway generator (not committed) that drew
/// the shapes with the `png` crate, 4x4 supersampled, at every `.iconset`
/// size (16 to 512 points, each at 1x and @2x), then `iconutil -c icns`.
/// Replace it with real art by regenerating `packaging/macos/Nova.icns` the
/// same way. It is embedded, so the bundler needs no path to it.
pub const ICON: &[u8] = include_bytes!("../../../packaging/macos/Nova.icns");

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn be_u32(bytes: &[u8]) -> usize {
        u32::from_be_bytes(bytes[..4].try_into().expect("four bytes")) as usize
    }

    /// The icns's entries: each type and its payload.
    fn entries(icns: &[u8]) -> Vec<(&[u8], &[u8])> {
        let mut entries = Vec::new();
        let mut at = 8;
        while at < icns.len() {
            let length = be_u32(&icns[at + 4..]);
            assert!(length >= 8 && at + length <= icns.len(), "entry at {at}");
            entries.push((&icns[at..at + 4], &icns[at + 8..at + length]));
            at += length;
        }
        entries
    }

    #[test]
    fn the_icon_is_an_icns_of_its_own_length() {
        assert_eq!(&ICON[..4], b"icns");
        assert_eq!(be_u32(&ICON[4..]), ICON.len());
    }

    #[test]
    fn the_icon_has_a_1024_pixel_png() {
        let ic10 = entries(ICON)
            .into_iter()
            .find(|(kind, _)| *kind == b"ic10")
            .map(|(_, payload)| payload)
            .expect("an ic10 entry");
        assert!(ic10.starts_with(PNG_SIGNATURE));
    }
}
