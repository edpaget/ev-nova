//! The Linux icons: hicolor PNGs, one per size, installed as
//! `share/icons/hicolor/<n>x<n>/apps/<identifier>.png`.

/// One icon: its size in pixels (it is square) and its PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon {
    /// The width and height, and the hicolor directory's `<n>x<n>`.
    pub size: u32,
    /// The PNG file.
    pub png: &'static [u8],
}

/// The icon of size `$n`, from `packaging/linux/icons/<n>x<n>.png`.
macro_rules! icon {
    ($n:literal) => {
        Icon {
            size: $n,
            png: include_bytes!(concat!(
                "../../../../packaging/linux/icons/",
                stringify!($n),
                "x",
                stringify!($n),
                ".png"
            )),
        }
    };
}

/// The icons at each standard hicolor size: a placeholder of original
/// art, the same as `packaging/macos/Nova.icns` (see [`crate::icon::ICON`])
/// and not the original game's icon (which is copyrighted and not used).
/// A dark navy rounded square holding a light ring around a gold
/// four-point star.
///
/// Made once on macOS from the `.icns`: `iconutil -c iconset
/// packaging/macos/Nova.icns` writes PNGs at 16, 32, 64 (the 32 @2x),
/// 128, 256 and 512, which are committed as they are, and
/// `sips -s format png -z <n> <n>` of the 256 makes the 24 and the 48.
/// They are in `packaging/linux/icons`, named `<n>x<n>.png`, and
/// embedded, so the tool writing the desktop files needs no path to them.
///
/// There is no scalable (SVG) icon: the placeholder has no vector master,
/// and an SVG drawn by hand would drift from the PNGs. Whoever brings real
/// art regenerates these the same way and adds
/// `share/icons/hicolor/scalable/apps/<identifier>.svg`.
pub const ICONS: [Icon; 8] = [
    icon!(16),
    icon!(24),
    icon!(32),
    icon!(48),
    icon!(64),
    icon!(128),
    icon!(256),
    icon!(512),
];

/// The 256 px icon, which the AppImage's root carries beside its desktop
/// file (as `<identifier>.png` and `.DirIcon`).
pub const ICON_256: Icon = icon!(256);

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn be_u32(bytes: &[u8]) -> u32 {
        u32::from_be_bytes(bytes[..4].try_into().expect("four bytes"))
    }

    /// The width and height in a PNG's header chunk, which comes first.
    fn dimensions(png: &[u8]) -> (u32, u32) {
        assert!(png.starts_with(PNG_SIGNATURE), "a PNG");
        assert_eq!(&png[12..16], b"IHDR", "the header chunk first");
        (be_u32(&png[16..]), be_u32(&png[20..]))
    }

    #[test]
    fn each_icon_is_a_png_of_its_size() {
        for icon in ICONS {
            assert_eq!(
                dimensions(icon.png),
                (icon.size, icon.size),
                "{}",
                icon.size
            );
        }
    }

    #[test]
    fn the_icons_are_the_standard_hicolor_sizes() {
        let sizes: Vec<u32> = ICONS.iter().map(|icon| icon.size).collect();
        assert_eq!(sizes, [16, 24, 32, 48, 64, 128, 256, 512]);
    }

    #[test]
    fn the_256_icon_is_the_hicolor_one() {
        assert_eq!(ICON_256.size, 256);
        assert!(ICONS.contains(&ICON_256));
    }
}
