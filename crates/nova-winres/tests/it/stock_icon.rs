//! The `.ico` made from the macOS icon in `packaging/macos`.

use nova_winres::{ICON_SIZES, ico_from_icns};

const ICNS: &[u8] = include_bytes!("../../../../packaging/macos/Nova.icns");

#[test]
fn the_ico_of_the_mac_icon_has_every_size() {
    let ico = ico_from_icns(ICNS).expect("an ico");
    let count = usize::from(u16::from_le_bytes([ico[4], ico[5]]));
    let sizes: Vec<u32> = (0..count)
        .map(|i| match ico[6 + 16 * i] {
            0 => 256,
            size => u32::from(size),
        })
        .collect();
    assert_eq!(sizes, ICON_SIZES);
}
