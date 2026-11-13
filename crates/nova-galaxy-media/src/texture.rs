//! A texture asset read as pixels: a planet's or moon's equirectangular
//! surface, its one `.png` decoded to 8-bit RGBA.
//!
//! [`read_texture`] checks what a texture may cost before it decodes it:
//! the bytes on disk and the longest side, read from the PNG's header
//! alone ([`budget`](crate::budget)'s limits, from the default budget's
//! `texture`), and the longest side the caller can draw (its GPU's
//! largest texture). Then it checks the shape the engine draws, whatever
//! the budgets say: twice as wide as tall, each side a power of two (so
//! its mip chain halves exactly). Only then is the image decoded, whatever
//! its colour type or bit depth, to RGBA8. The header and the pixels are
//! read as [`image`](crate::image) reads any PNG.
//!
//! [`read_picture`] reads a texture asset that is not a surface: a sky
//! decoration's image, of any shape. It makes the same checks in the
//! same order, less the shape.

use nova_galaxy_format::asset::Asset;
use nova_galaxy_format::media::is_lfs_pointer;
use nova_galaxy_format::source::Source;

use crate::budget::{Cost, Limits, Overrun, over_budget};
use crate::image::{decode_png, png_size};

/// A texture's pixels, as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Texture {
    /// The data-folder path it was read from.
    pub file: String,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// The bytes its file takes on disk.
    pub disk_bytes: u64,
    /// Its pixels, 8-bit RGBA, row by row from the top: four bytes a
    /// pixel.
    pub rgba: Vec<u8>,
}

impl Texture {
    /// Its longest side, in pixels: what a budget's `side` limits.
    #[must_use]
    pub fn side(&self) -> u32 {
        self.width.max(self.height)
    }
}

/// Why an asset has no texture to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoTexture {
    /// None of its files is a `.png`: a placeholder with no files, or a
    /// model.
    None,
    /// Its `.png` is a Git LFS pointer: the checkout hasn't fetched it.
    Pointer,
    /// Its `.png`, at this path, isn't there.
    Missing(String),
    /// Its `.png` doesn't read as a PNG; why, naming the file.
    Unreadable(String),
    /// It is over its art budget: every limit it breaks, found from the
    /// file's size and its header.
    OverBudget(Vec<Overrun>),
    /// Its longest side is over the most the caller can draw (the GPU's
    /// largest texture), found from its header.
    TooLarge {
        /// The file.
        file: String,
        /// Its longest side in pixels.
        side: u32,
        /// The most the caller can draw.
        most: u32,
    },
    /// It isn't twice as wide as tall, or a side isn't a power of two.
    Shape {
        /// The file.
        file: String,
        /// Its width in pixels.
        width: u32,
        /// Its height in pixels.
        height: u32,
    },
}

/// Whether a `width` by `height` image is the shape a planet's surface
/// takes: equirectangular, twice as wide as tall, each side a power of
/// two.
#[must_use]
pub fn is_surface_shape(width: u32, height: u32) -> bool {
    width.is_power_of_two() && height.is_power_of_two() && width / 2 == height
}

/// The texture `asset` names, read through `source` and held to `limits`
/// (`budget::limits` of the asset) and to `max_side`, the longest side
/// the caller can draw: its first `.png`, whatever its `placeholder`
/// flag. The budget, the side and the shape are checked from the file's
/// size and its header, before the image is decoded.
pub fn read_texture(
    source: &impl Source,
    asset: &Asset,
    limits: &Limits,
    max_side: u32,
) -> Result<Texture, NoTexture> {
    read(source, asset, limits, max_side, is_surface_shape)
}

/// The picture `asset` names, read as [`read_texture`] reads a planet's
/// surface but of any shape: a sky decoration's image, which is not held
/// to a surface's 2:1 or to powers of two. The budget and the side are
/// checked from the file's size and its header, before the image is
/// decoded, and it never gives [`NoTexture::Shape`].
pub fn read_picture(
    source: &impl Source,
    asset: &Asset,
    limits: &Limits,
    max_side: u32,
) -> Result<Texture, NoTexture> {
    read(source, asset, limits, max_side, |_, _| true)
}

/// The one reader behind [`read_texture`] and [`read_picture`], so both
/// check in one order: the budget, the most side, then `shape` (whether
/// a width and height may be drawn), all from the header, and only then
/// the decode.
fn read(
    source: &impl Source,
    asset: &Asset,
    limits: &Limits,
    max_side: u32,
    shape: impl Fn(u32, u32) -> bool,
) -> Result<Texture, NoTexture> {
    let file = asset
        .files
        .iter()
        .find(|f| f.extension() == "png")
        .ok_or(NoTexture::None)?;
    let path = file.as_str();
    let unreadable =
        |why: &dyn std::fmt::Display| NoTexture::Unreadable(format!("`{path}`: {why}"));
    let bytes = match source.bytes(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Err(NoTexture::Missing(path.to_owned())),
        Err(e) => return Err(unreadable(&e)),
    };
    if is_lfs_pointer(&bytes) {
        return Err(NoTexture::Pointer);
    }
    let (width, height) = png_size(&bytes).ok_or_else(|| unreadable(&"not a PNG"))?;
    let disk_bytes = bytes.len() as u64;
    let over = over_budget(limits, Cost::texture(disk_bytes, width.max(height)));
    if !over.is_empty() {
        return Err(NoTexture::OverBudget(over));
    }
    if width.max(height) > max_side {
        return Err(NoTexture::TooLarge {
            file: path.to_owned(),
            side: width.max(height),
            most: max_side,
        });
    }
    if !shape(width, height) {
        return Err(NoTexture::Shape {
            file: path.to_owned(),
            width,
            height,
        });
    }
    let rgba = decode_png(&bytes).map_err(|e| unreadable(&e))?;
    Ok(Texture {
        file: path.to_owned(),
        width,
        height,
        disk_bytes,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use nova_galaxy_format::id::Id;
    use nova_galaxy_format::source::MemSource;

    use super::*;
    use crate::budget::{Limit, Measure};
    use crate::fixture::png as rgba_png;

    const FILE: &str = "media/export/p/surface.png";

    fn asset(files: &[&str]) -> Asset {
        let yaml = format!(
            "---\nkind: texture\nplaceholder: false\nfiles: [{}]\n---\n",
            files.join(", ")
        );
        nova_galaxy_format::frontmatter::parse::<Asset>(&yaml)
            .unwrap()
            .0
    }

    #[allow(clippy::unnecessary_wraps)]
    fn limit(most: u32) -> Option<Limit> {
        Some(Limit {
            most,
            budget: Id::try_from("default".to_owned()).unwrap(),
        })
    }

    /// What reading `bytes` as the texture at [`FILE`] gives, held to
    /// `limits`.
    fn read(bytes: &[u8], limits: &Limits) -> Result<Texture, NoTexture> {
        let source = MemSource::default().with_bytes(FILE, bytes);
        read_texture(&source, &asset(&[FILE]), limits, u32::MAX)
    }

    /// A PNG `width` by `height` of colour type `color` at `depth`, its
    /// samples `data` (and `palette`, for an indexed one).
    fn encoded(
        (width, height): (u32, u32),
        color: png::ColorType,
        depth: png::BitDepth,
        data: &[u8],
        palette: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some(palette) = palette {
            encoder.set_palette(palette.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
        out
    }

    /// A 2 by 1 image: a dark blue pixel, then an orange one.
    const RGBA: [u8; 8] = [10, 20, 30, 255, 200, 100, 50, 255];

    #[test]
    fn every_colour_type_and_depth_decodes_to_the_same_rgba8() {
        let eight = png::BitDepth::Eight;
        let rgba = rgba_png(2, 1, &RGBA);
        let rgb = encoded(
            (2, 1),
            png::ColorType::Rgb,
            eight,
            &[10, 20, 30, 200, 100, 50],
            None,
        );
        let indexed = encoded(
            (2, 1),
            png::ColorType::Indexed,
            eight,
            &[0, 1],
            Some(&[10, 20, 30, 200, 100, 50]),
        );
        // Each sample `c` widened to `c * 257`, whose high byte is `c`.
        let wide: Vec<u8> = RGBA.iter().flat_map(|c| [*c, *c]).collect();
        let sixteen = encoded(
            (2, 1),
            png::ColorType::Rgba,
            png::BitDepth::Sixteen,
            &wide,
            None,
        );
        for bytes in [rgba, rgb, indexed, sixteen] {
            let texture = read(&bytes, &Limits::default()).unwrap();
            assert_eq!(texture.rgba, RGBA);
        }
    }

    #[test]
    fn grey_decodes_to_equal_channels() {
        let eight = png::BitDepth::Eight;
        let grey = encoded((2, 1), png::ColorType::Grayscale, eight, &[7, 9], None);
        assert_eq!(
            read(&grey, &Limits::default()).unwrap().rgba,
            [7, 7, 7, 255, 9, 9, 9, 255]
        );
        let grey_alpha = encoded(
            (2, 1),
            png::ColorType::GrayscaleAlpha,
            eight,
            &[7, 1, 9, 2],
            None,
        );
        assert_eq!(
            read(&grey_alpha, &Limits::default()).unwrap().rgba,
            [7, 7, 7, 1, 9, 9, 9, 2]
        );
    }

    #[test]
    fn a_texture_reports_its_size_its_bytes_and_its_longest_side() {
        let bytes = rgba_png(4, 2, &[9; 32]);
        let texture = read(&bytes, &Limits::default()).unwrap();
        assert_eq!((texture.width, texture.height, texture.side()), (4, 2, 4));
        assert_eq!(texture.disk_bytes, bytes.len() as u64);
        assert_eq!(texture.file, FILE);
        assert_eq!(texture.rgba, [9; 32]);
        let tall = Texture {
            width: 2,
            height: 8,
            ..texture
        };
        assert_eq!(tall.side(), 8);
    }

    #[test]
    fn a_placeholder_or_an_asset_with_no_png_has_no_file() {
        let source = MemSource::default();
        assert_eq!(
            read_texture(&source, &asset(&[]), &Limits::default(), u32::MAX),
            Err(NoTexture::None)
        );
        let placeholder = nova_galaxy_format::frontmatter::parse::<Asset>(
            "---\nkind: texture\nplaceholder: true\n---\n",
        )
        .unwrap()
        .0;
        assert_eq!(
            read_texture(&source, &placeholder, &Limits::default(), u32::MAX),
            Err(NoTexture::None)
        );
        let source = source.with_bytes("media/export/p/a.glb", &rgba_png(2, 1, &RGBA));
        assert_eq!(
            read_texture(
                &source,
                &asset(&["media/export/p/a.glb"]),
                &Limits::default(),
                u32::MAX,
            ),
            Err(NoTexture::None)
        );
    }

    #[test]
    fn its_first_png_is_the_texture() {
        let source = MemSource::default()
            .with_bytes("media/export/p/a.png", &rgba_png(2, 1, &RGBA))
            .with_bytes("media/export/p/b.png", &rgba_png(4, 2, &[0; 32]));
        let asset = asset(&[
            "media/export/p/LICENSE.txt",
            "media/export/p/a.png",
            "media/export/p/b.png",
        ]);
        let texture = read_texture(&source, &asset, &Limits::default(), u32::MAX).unwrap();
        assert_eq!(texture.file, "media/export/p/a.png");
    }

    #[test]
    fn an_lfs_pointer_is_reported_as_one() {
        let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n";
        assert_eq!(read(pointer, &Limits::default()), Err(NoTexture::Pointer));
    }

    #[test]
    fn a_missing_file_or_one_that_isn_t_a_png_names_the_file() {
        let source = MemSource::default();
        assert_eq!(
            read_texture(&source, &asset(&[FILE]), &Limits::default(), u32::MAX),
            Err(NoTexture::Missing(FILE.into()))
        );
        let Err(NoTexture::Unreadable(why)) = read(b"GIF89a nonsense", &Limits::default()) else {
            panic!("unreadable");
        };
        assert!(why.starts_with(&format!("`{FILE}`")), "{why}");
        // A PNG's signature without its header chunk is no PNG.
        let mut headless = rgba_png(2, 1, &RGBA);
        headless[12..16].copy_from_slice(b"IDAT");
        let Err(NoTexture::Unreadable(why)) = read(&headless, &Limits::default()) else {
            panic!("unreadable");
        };
        assert!(why.contains("not a PNG"), "{why}");
        // A PNG's header with nothing after it reads no image.
        let header = &rgba_png(2, 1, &RGBA)[..33];
        let Err(NoTexture::Unreadable(why)) = read(header, &Limits::default()) else {
            panic!("unreadable");
        };
        assert!(why.starts_with(&format!("`{FILE}`")), "{why}");
    }

    #[test]
    fn a_surface_is_twice_as_wide_as_tall_with_power_of_two_sides() {
        assert_eq!(
            read(&rgba_png(64, 64, &vec![0; 64 * 64 * 4]), &Limits::default()),
            Err(NoTexture::Shape {
                file: FILE.into(),
                width: 64,
                height: 64,
            })
        );
        assert_eq!(
            read(&rgba_png(96, 48, &vec![0; 96 * 48 * 4]), &Limits::default()),
            Err(NoTexture::Shape {
                file: FILE.into(),
                width: 96,
                height: 48,
            })
        );
        assert!(read(&rgba_png(64, 32, &vec![0; 64 * 32 * 4]), &Limits::default()).is_ok());
        assert!(is_surface_shape(2, 1));
        assert!(!is_surface_shape(1, 2));
        assert!(!is_surface_shape(8, 2));
        assert!(!is_surface_shape(0, 0));
        assert!(!is_surface_shape(6, 3));
    }

    /// A 64 by 32 PNG whose bytes after the header are garbage: `extra`
    /// bytes of it.
    fn corrupt_past_the_header(extra: usize) -> Vec<u8> {
        let mut bytes = rgba_png(64, 32, &vec![0; 64 * 32 * 4])[..33].to_vec();
        bytes.extend(std::iter::repeat_n(0xA5, extra));
        bytes
    }

    #[test]
    fn a_budget_overrun_is_found_from_the_header_before_decoding() {
        let side = Limits {
            side: limit(32),
            ..Limits::default()
        };
        assert_eq!(
            read(&corrupt_past_the_header(16), &side),
            Err(NoTexture::OverBudget(vec![Overrun {
                measure: Measure::Side,
                used: 64,
                most: 32,
                budget: Id::try_from("default".to_owned()).unwrap(),
            }]))
        );
        let bytes = corrupt_past_the_header(2000);
        let kib = Limits {
            kib: limit(1),
            ..Limits::default()
        };
        assert_eq!(
            read(&bytes, &kib),
            Err(NoTexture::OverBudget(vec![Overrun {
                measure: Measure::Kib,
                used: bytes.len() as u64,
                most: 1,
                budget: Id::try_from("default".to_owned()).unwrap(),
            }]))
        );
        // Within its budget, the same file fails only when it decodes.
        assert!(matches!(
            read(&corrupt_past_the_header(16), &Limits::default()),
            Err(NoTexture::Unreadable(_))
        ));
        // At the limit is within it.
        let at = Limits {
            side: limit(64),
            ..Limits::default()
        };
        assert!(read(&rgba_png(64, 32, &vec![0; 64 * 32 * 4]), &at).is_ok());
    }

    /// A PNG's header alone, declaring `width` by `height`.
    fn header_only(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = rgba_png(2, 1, &RGBA)[..33].to_vec();
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn a_texture_wider_than_the_gpu_draws_is_refused_before_decoding() {
        // No budget limits it, and it is a surface's shape: only the
        // caller's most side stops it, before the missing pixels decode.
        let within = |bytes: &[u8]| {
            read_texture(
                &MemSource::default().with_bytes(FILE, bytes),
                &asset(&[FILE]),
                &Limits::default(),
                8_192,
            )
        };
        assert_eq!(
            within(&header_only(16_384, 8_192)),
            Err(NoTexture::TooLarge {
                file: FILE.into(),
                side: 16_384,
                most: 8_192,
            })
        );
        // At the most is within it: it fails only when it decodes.
        assert!(matches!(
            within(&header_only(8_192, 4_096)),
            Err(NoTexture::Unreadable(_))
        ));
    }

    #[test]
    fn the_budget_is_checked_before_the_shape() {
        let side = Limits {
            side: limit(32),
            ..Limits::default()
        };
        assert!(matches!(
            read(&rgba_png(64, 64, &vec![0; 64 * 64 * 4]), &side),
            Err(NoTexture::OverBudget(_))
        ));
    }

    #[test]
    fn a_wrong_shape_is_found_from_the_header_before_decoding() {
        // Square, and garbage past its header: the shape is what is
        // reported, so the pixels were never decoded.
        let mut bytes = rgba_png(64, 64, &vec![0; 64 * 64 * 4])[..33].to_vec();
        bytes.extend(std::iter::repeat_n(0xA5, 16));
        assert_eq!(
            read(&bytes, &Limits::default()),
            Err(NoTexture::Shape {
                file: FILE.into(),
                width: 64,
                height: 64,
            })
        );
        // The same file as a picture has no shape to break, so it fails
        // when it decodes.
        assert!(matches!(
            picture(&bytes, &Limits::default()),
            Err(NoTexture::Unreadable(_))
        ));
    }

    /// What reading `bytes` as the picture at [`FILE`] gives, held to
    /// `limits`.
    fn picture(bytes: &[u8], limits: &Limits) -> Result<Texture, NoTexture> {
        let source = MemSource::default().with_bytes(FILE, bytes);
        read_picture(&source, &asset(&[FILE]), limits, u32::MAX)
    }

    #[test]
    fn a_picture_of_any_shape_reads() {
        let pixels: Vec<u8> = (0..5 * 3 * 4).collect();
        let bytes = rgba_png(5, 3, &pixels);
        let texture = picture(&bytes, &Limits::default()).unwrap();
        assert_eq!((texture.width, texture.height), (5, 3));
        assert_eq!(texture.rgba, pixels);
        assert_eq!(texture.file, FILE);
        assert_eq!(texture.disk_bytes, bytes.len() as u64);
        // A planet's surface refuses the shape a picture takes.
        assert_eq!(
            read(&bytes, &Limits::default()),
            Err(NoTexture::Shape {
                file: FILE.into(),
                width: 5,
                height: 3,
            })
        );
        // A tall one and a square one read too.
        assert!(picture(&rgba_png(1, 4, &[7; 16]), &Limits::default()).is_ok());
        assert!(picture(&rgba_png(2, 2, &[7; 16]), &Limits::default()).is_ok());
    }

    #[test]
    fn a_picture_that_cannot_be_read_says_why_as_a_texture_does() {
        let pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:ab\nsize 12\n";
        assert_eq!(
            picture(pointer, &Limits::default()),
            Err(NoTexture::Pointer)
        );
        let source = MemSource::default();
        assert_eq!(
            read_picture(&source, &asset(&[FILE]), &Limits::default(), u32::MAX),
            Err(NoTexture::Missing(FILE.into()))
        );
        assert_eq!(
            read_picture(&source, &asset(&[]), &Limits::default(), u32::MAX),
            Err(NoTexture::None)
        );
        let Err(NoTexture::Unreadable(why)) = picture(b"GIF89a nonsense", &Limits::default())
        else {
            panic!("unreadable");
        };
        assert!(why.contains("not a PNG"), "{why}");
    }

    #[test]
    fn a_picture_is_held_to_its_budget_and_the_most_side_before_decoding() {
        let side = Limits {
            side: limit(32),
            ..Limits::default()
        };
        assert_eq!(
            picture(&corrupt_past_the_header(16), &side),
            Err(NoTexture::OverBudget(vec![Overrun {
                measure: Measure::Side,
                used: 64,
                most: 32,
                budget: Id::try_from("default".to_owned()).unwrap(),
            }]))
        );
        let bytes = corrupt_past_the_header(2000);
        let kib = Limits {
            kib: limit(1),
            ..Limits::default()
        };
        assert!(matches!(
            picture(&bytes, &kib),
            Err(NoTexture::OverBudget(over)) if over[0].measure == Measure::Kib
        ));
        let within = |bytes: &[u8], most: u32| {
            read_picture(
                &MemSource::default().with_bytes(FILE, bytes),
                &asset(&[FILE]),
                &Limits::default(),
                most,
            )
        };
        // A tall header alone: its longer side is its height.
        assert_eq!(
            within(&header_only(300, 2_000), 1_024),
            Err(NoTexture::TooLarge {
                file: FILE.into(),
                side: 2_000,
                most: 1_024,
            })
        );
        // At the most is within it: it fails only when it decodes.
        assert!(matches!(
            within(&header_only(300, 1_024), 1_024),
            Err(NoTexture::Unreadable(_))
        ));
    }
}
