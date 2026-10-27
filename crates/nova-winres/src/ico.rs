//! The `.ico` Windows shows, made from the macOS `.icns`
//! (`packaging/macos/Nova.icns`), so no second copy of the art is kept.
//!
//! An `.icns` is a big-endian list of typed entries. Its large sizes are
//! PNGs, which an `.ico` can hold as they are (Windows Vista and later);
//! its 16-pixel image is only `ic04`, 32-bit ARGB packed by channel, which
//! becomes a 32-bit bitmap entry. An `.ico` is a little-endian directory
//! (`ICONDIR`, then one `ICONDIRENTRY` per image) followed by the images.

/// The sizes the `.ico` holds, in pixels: Explorer's and the taskbar's,
/// and the title bar's at 100 % to 400 %. Explorer scales the others
/// (48 px) from the larger ones.
pub const ICON_SIZES: [u32; 5] = [16, 32, 64, 128, 256];

/// Why an `.icns` could not be made into an `.ico`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IconError {
    /// It does not start with `icns` and its own length.
    #[error("not an icns file")]
    NotIcns,
    /// An entry runs past the end of the file, or is shorter than its
    /// header.
    #[error("the icns entry at byte {0} is cut short")]
    Truncated(usize),
    /// A packed ARGB image does not unpack to its size.
    #[error("the {0}-pixel ARGB image does not unpack to its size")]
    BadArgb(u32),
    /// None of [`ICON_SIZES`] is in it.
    #[error("the icns holds none of the icon's sizes")]
    NoSizes,
}

/// The `.ico` holding each of [`ICON_SIZES`] that `icns` has, smallest
/// first: a PNG entry of that size as it is, or else the `ARGB` entry of
/// that size as a bitmap.
pub fn ico_from_icns(icns: &[u8]) -> Result<Vec<u8>, IconError> {
    let entries = icns_entries(icns)?;
    let mut images = Vec::new();
    for size in ICON_SIZES {
        let png = entries
            .iter()
            .find(|(_, payload)| png_size(payload) == Some(size));
        let image = match png {
            Some((_, payload)) => Some(payload.to_vec()),
            None => match entries
                .iter()
                .find(|(kind, _)| argb_size(*kind) == Some(size))
            {
                Some((_, payload)) => Some(bitmap(size, &unpack_argb(size, payload)?)),
                None => None,
            },
        };
        if let Some(image) = image {
            images.push((size, image));
        }
    }
    if images.is_empty() {
        return Err(IconError::NoSizes);
    }
    Ok(ico(&images))
}

/// The PNG signature every PNG entry starts with.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// An icns entry: its type and payload.
type Entry<'a> = ([u8; 4], &'a [u8]);

/// `icns`'s entries.
fn icns_entries(icns: &[u8]) -> Result<Vec<Entry<'_>>, IconError> {
    if icns.len() < 8 || &icns[..4] != b"icns" || be_u32(&icns[4..8]) != icns.len() {
        return Err(IconError::NotIcns);
    }
    let mut entries = Vec::new();
    let mut at = 8;
    while at < icns.len() {
        let header = icns.get(at..at + 8).ok_or(IconError::Truncated(at))?;
        let length = be_u32(&header[4..]);
        if length < 8 || at + length > icns.len() {
            return Err(IconError::Truncated(at));
        }
        let kind = [header[0], header[1], header[2], header[3]];
        entries.push((kind, &icns[at + 8..at + length]));
        at += length;
    }
    Ok(entries)
}

fn be_u32(bytes: &[u8]) -> usize {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize
}

/// The size of a square PNG, from its `IHDR`; `None` for anything else.
fn png_size(payload: &[u8]) -> Option<u32> {
    if !payload.starts_with(PNG_SIGNATURE) || payload.get(12..16) != Some(b"IHDR") {
        return None;
    }
    let width = u32::from_be_bytes(payload.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(payload.get(20..24)?.try_into().ok()?);
    (width == height).then_some(width)
}

/// The size of the packed ARGB entry types: `ic04` and `ic05`.
fn argb_size(kind: [u8; 4]) -> Option<u32> {
    match &kind {
        b"ic04" => Some(16),
        b"ic05" => Some(32),
        _ => None,
    }
}

/// A `size`-pixel `ARGB` payload's pixels, top row first, each
/// `[a, r, g, b]`. Each channel is a plane of its own, packed: a byte
/// below 0x80 is followed by that many plus one literal bytes, and any
/// other by one byte repeated that many minus 125 times.
fn unpack_argb(size: u32, payload: &[u8]) -> Result<Vec<[u8; 4]>, IconError> {
    let bad = IconError::BadArgb(size);
    let packed = payload.strip_prefix(b"ARGB").ok_or(bad.clone())?;
    let count = (size * size) as usize;
    let mut planes = Vec::with_capacity(count * 4);
    let mut at = 0;
    while at < packed.len() {
        let control = usize::from(packed[at]);
        if control < 0x80 {
            let literal = packed.get(at + 1..at + 2 + control).ok_or(bad.clone())?;
            planes.extend_from_slice(literal);
            at += 2 + control;
        } else {
            let byte = *packed.get(at + 1).ok_or(bad.clone())?;
            planes.extend(std::iter::repeat_n(byte, control - 125));
            at += 2;
        }
    }
    if planes.len() != count * 4 {
        return Err(bad);
    }
    Ok((0..count)
        .map(|i| {
            [
                planes[i],
                planes[count + i],
                planes[2 * count + i],
                planes[3 * count + i],
            ]
        })
        .collect())
}

/// An `.ico` bitmap image of the `size`-pixel `pixels` (top row first,
/// `[a, r, g, b]`): a `BITMAPINFOHEADER` whose height counts the mask
/// too, the rows bottom up as blue, green, red and alpha, then an empty
/// AND mask (the alpha is the shape), each of its rows padded to four
/// bytes.
fn bitmap(size: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    let width = size as usize;
    let mask_row = width.div_ceil(32) * 4;
    let image_bytes = width * width * 4 + mask_row * width;
    let mut out = Vec::with_capacity(40 + image_bytes);
    out.extend(40u32.to_le_bytes());
    out.extend(size.to_le_bytes());
    out.extend((size * 2).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(0u32.to_le_bytes());
    out.extend((image_bytes as u32).to_le_bytes());
    out.extend([0; 16]);
    for row in pixels.chunks(width).rev() {
        for [a, r, g, b] in row {
            out.extend([*b, *g, *r, *a]);
        }
    }
    out.resize(40 + image_bytes, 0);
    out
}

/// The `.ico` of `images`, each its size and image, in order.
fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend((images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (size, image) in images {
        // 256 is written as 0.
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend([side, side, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((image.len() as u32).to_le_bytes());
        out.extend((offset as u32).to_le_bytes());
        offset += image.len();
    }
    for (_, image) in images {
        out.extend(image);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    /// A stand-in PNG of `size` pixels: the signature and the `IHDR` the
    /// size is read from, then `tail`.
    fn png(size: u32, tail: &[u8]) -> Vec<u8> {
        let mut bytes = PNG.to_vec();
        bytes.extend(13u32.to_be_bytes());
        bytes.extend(b"IHDR");
        bytes.extend(size.to_be_bytes());
        bytes.extend(size.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0]);
        bytes.extend(tail);
        bytes
    }

    /// An icns of `entries`, each a type and its payload.
    fn icns(entries: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (kind, payload) in entries {
            body.extend(*kind);
            body.extend((payload.len() as u32 + 8).to_be_bytes());
            body.extend(payload);
        }
        let mut file = b"icns".to_vec();
        file.extend((body.len() as u32 + 8).to_be_bytes());
        file.extend(body);
        file
    }

    /// `plane` packed as the icns packs an ARGB channel: literal runs of
    /// up to 128 bytes, and a repeat for each run of three or more of one
    /// byte (up to 130).
    fn pack(plane: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut at = 0;
        while at < plane.len() {
            let run = plane[at..]
                .iter()
                .take_while(|&&b| b == plane[at])
                .count()
                .min(130);
            if run >= 3 {
                out.push((run + 125) as u8);
                out.push(plane[at]);
                at += run;
            } else {
                let literal = (plane.len() - at).min(128).min(run.max(1));
                out.push((literal - 1) as u8);
                out.extend(&plane[at..at + literal]);
                at += literal;
            }
        }
        out
    }

    /// An `ARGB` payload of a `size`-pixel image whose pixel at column x,
    /// row y (from the top) is `pixel(x, y)`, as `[a, r, g, b]`.
    fn argb(size: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
        let mut out = b"ARGB".to_vec();
        for channel in 0..4 {
            let plane: Vec<u8> = (0..size * size)
                .map(|i| pixel(i % size, i / size)[channel])
                .collect();
            out.extend(pack(&plane));
        }
        out
    }

    fn le16(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }

    fn le32(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
    }

    /// The ico's entries: each one's width byte, height byte, planes, bit
    /// count and image.
    fn entries(ico: &[u8]) -> Vec<(u8, u8, u16, u16, &[u8])> {
        assert_eq!((le16(ico, 0), le16(ico, 2)), (0, 1), "an ICONDIR");
        let count = usize::from(le16(ico, 4));
        (0..count)
            .map(|i| {
                let entry = &ico[6 + 16 * i..6 + 16 * (i + 1)];
                let (size, offset) = (le32(entry, 8) as usize, le32(entry, 12) as usize);
                assert_eq!((entry[2], entry[3]), (0, 0), "no palette");
                (
                    entry[0],
                    entry[1],
                    le16(entry, 4),
                    le16(entry, 6),
                    &ico[offset..offset + size],
                )
            })
            .collect()
    }

    #[test]
    fn each_png_size_is_an_entry_of_that_png_smallest_first() {
        let big = png(256, b"big");
        let small = png(32, b"small");
        let ico = ico_from_icns(&icns(&[(b"ic08", big.clone()), (b"ic11", small.clone())]))
            .expect("an ico");
        assert_eq!(
            entries(&ico),
            [
                (32, 32, 1, 32, small.as_slice()),
                (0, 0, 1, 32, big.as_slice())
            ]
        );
    }

    #[test]
    fn the_images_follow_the_directory_one_after_another() {
        let ico = ico_from_icns(&icns(&[
            (b"ic12", png(64, b"64")),
            (b"ic07", png(128, b"128")),
        ]))
        .expect("an ico");
        let first = le32(&ico, 6 + 12) as usize;
        let first_len = le32(&ico, 6 + 8) as usize;
        assert_eq!(first, 6 + 2 * 16);
        assert_eq!(le32(&ico, 6 + 16 + 12) as usize, first + first_len);
        assert_eq!(
            ico.len(),
            first + first_len + le32(&ico, 6 + 16 + 8) as usize
        );
    }

    #[test]
    fn sizes_the_icon_does_not_use_and_other_entries_are_left_out() {
        let ico = ico_from_icns(&icns(&[
            (b"ic09", png(512, b"")),
            (b"info", b"bplist00".to_vec()),
            (b"ic11", png(32, b"")),
            (b"ic12", png(64, b"")),
            (b"ic13", png(256, b"twin")),
            (b"ic08", png(256, b"first")),
        ]))
        .expect("an ico");
        let sizes: Vec<u8> = entries(&ico).iter().map(|entry| entry.0).collect();
        assert_eq!(sizes, [32, 64, 0], "one 256 entry");
        assert_eq!(entries(&ico)[2].4, png(256, b"twin"), "the first one found");
    }

    #[test]
    fn a_16_pixel_argb_image_is_a_bottom_up_bgra_bitmap() {
        let pixel = |x: u32, y: u32| [255 - y as u8, y as u8, x as u8, 7];
        let ico = ico_from_icns(&icns(&[(b"ic04", argb(16, pixel))])).expect("an ico");
        let [(16, 16, 1, 32, image)] = entries(&ico)[..] else {
            panic!("one 16-pixel entry: {:?}", entries(&ico));
        };
        assert_eq!(le32(image, 0), 40, "a BITMAPINFOHEADER");
        assert_eq!(
            (le32(image, 4), le32(image, 8)),
            (16, 32),
            "width, and height with the mask"
        );
        assert_eq!(
            (le16(image, 12), le16(image, 14)),
            (1, 32),
            "planes and bits"
        );
        assert_eq!(le32(image, 16), 0, "uncompressed");
        let pixels = &image[40..40 + 16 * 16 * 4];
        // The first row stored is the bottom one, y = 15.
        assert_eq!(
            &pixels[..4],
            [7, 0, 15, 240],
            "x 0: blue, green, red, alpha"
        );
        assert_eq!(&pixels[4 * 3..4 * 4], [7, 3, 15, 240], "x 3");
        assert_eq!(
            &pixels[16 * 4 * 15..16 * 4 * 15 + 4],
            [7, 0, 0, 255],
            "the top row last"
        );
        let mask = &image[40 + 16 * 16 * 4..];
        assert_eq!(mask, [0; 16 * 4], "an empty mask, rows of four bytes");
    }

    #[test]
    fn a_png_of_a_size_wins_over_the_argb_of_that_size() {
        let ico = ico_from_icns(&icns(&[
            (b"ic05", argb(32, |_, _| [255; 4])),
            (b"ic11", png(32, b"png")),
        ]))
        .expect("an ico");
        assert_eq!(entries(&ico), [(32, 32, 1, 32, png(32, b"png").as_slice())]);
    }

    #[test]
    fn an_argb_image_that_does_not_unpack_to_its_size_is_refused() {
        let mut short = argb(16, |_, _| [1, 2, 3, 4]);
        short.truncate(short.len() - 2);
        assert_eq!(
            ico_from_icns(&icns(&[(b"ic04", short)])),
            Err(IconError::BadArgb(16))
        );
        let mut long = argb(16, |_, _| [1, 2, 3, 4]);
        long.extend([0, 9]);
        assert_eq!(
            ico_from_icns(&icns(&[(b"ic04", long)])),
            Err(IconError::BadArgb(16))
        );
        assert_eq!(
            ico_from_icns(&icns(&[(b"ic04", b"RGBA".to_vec())])),
            Err(IconError::BadArgb(16))
        );
    }

    #[test]
    fn what_is_not_a_whole_icns_is_refused() {
        assert_eq!(ico_from_icns(b""), Err(IconError::NotIcns));
        assert_eq!(ico_from_icns(b"icnz\0\0\0\x08"), Err(IconError::NotIcns));
        assert_eq!(
            ico_from_icns(b"icns\0\0\0\x09"),
            Err(IconError::NotIcns),
            "its length is wrong"
        );
        let mut cut = icns(&[(b"ic11", png(32, b""))]);
        let shorter = cut.len() as u32 - 1;
        cut[4..8].copy_from_slice(&shorter.to_be_bytes());
        cut.pop();
        assert_eq!(ico_from_icns(&cut), Err(IconError::Truncated(8)));
        let mut tiny = b"icns\0\0\0\x10".to_vec();
        tiny.extend(b"ic11\0\0\0\x04");
        assert_eq!(ico_from_icns(&tiny), Err(IconError::Truncated(8)));
        assert_eq!(ico_from_icns(&icns(&[])), Err(IconError::NoSizes));
        assert_eq!(
            ico_from_icns(&icns(&[(b"ic11", b"not a png".to_vec())])),
            Err(IconError::NoSizes)
        );
    }
}
