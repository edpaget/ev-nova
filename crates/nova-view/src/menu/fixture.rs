//! Tiny main-screen art, as resources, for tests of the main screen and
//! of what opens on it ([`main_screen_resources`]).
//!
//! Every button is a 4 x 3 `rlëD` of two frames whose frame 0 has clear
//! corners ([`BUTTON_SIZE`], [`CLEAR_CORNER`]), the logo a 2 x 1 `PICT`
//! strip of seven frames, the rollover a 3 x 2 `rlëD` of seven frames,
//! each slide a 2 x 1 `PICT` strip of [`SLIDE_FRAMES`] frames, and the
//! background a 4 x 2 `PICT`, each placed by `cölr` 128 as the constants
//! below say. The slides' counts, 3, 2 and 3, start them at -1, -2 and
//! -3 (-2.5 truncated toward zero). [`main_screen_art`] is that art as the game
//! data's port reads it.

use nova_data::graphics::fixture::{PictBuilder, RledBuilder, rgb888_pict};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::colors::Colors;
use nova_data::records::spin::Spin;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_rsrc::ResType;
use nova_rsrc::fixture::ForkBuilder;

use super::art::{
    BACKGROUND_PICT, BUTTON_SPINS, COLORS_ID, LOGO_SPIN, MainScreenArt, MainScreenSource,
    ROLLOVER_SPIN, SLIDE_SPINS,
};

/// The buttons' `rlëD`s, New Pilot's first.
pub const BUTTON_SPRITES: [i16; 6] = [8050, 8051, 8052, 8053, 8054, 8055];
/// Each button's frame size.
pub const BUTTON_SIZE: (u16, u16) = (4, 3);
/// Where each button goes, from `menuTopCorner`.
pub const BUTTON_POSITIONS: [(i16, i16); 6] = [
    (100, 50),
    (100, 60),
    (100, 70),
    (200, 50),
    (200, 60),
    (200, 70),
];
/// A clear pixel inside each button's rectangle, from its top-left.
pub const CLEAR_CORNER: (u16, u16) = (0, 0);
/// The background's `PICT` size.
pub const BACKGROUND_SIZE: (u16, u16) = (4, 2);
/// The logo's `PICT`.
pub const LOGO_PICT: i16 = 8010;
/// Each logo frame's size.
pub const LOGO_FRAME: (u16, u16) = (2, 1);
/// How many logo frames there are, one under another.
pub const LOGO_FRAMES: u16 = 7;
/// Where the logo goes, from `menuTopCorner`.
pub const LOGO_POSITION: (i16, i16) = (300, 20);
/// The rollover's `rlëD`.
pub const ROLLOVER_SPRITE: i16 = 8020;
/// Each rollover frame's size.
pub const ROLLOVER_SIZE: (u16, u16) = (3, 2);
/// How many rollover frames there are.
pub const ROLLOVER_FRAMES: u16 = 7;
/// Where the rollover goes, from `menuTopCorner`.
pub const ROLLOVER_POSITION: (i16, i16) = (150, 100);
/// The slides' `PICT`s, the top row's first.
pub const SLIDE_PICTS: [i16; 3] = [8030, 8031, 8032];
/// Each slide frame's size.
pub const SLIDE_FRAME: (u16, u16) = (2, 1);
/// How many frames each slide has, one under another.
pub const SLIDE_FRAMES: [u16; 3] = [3, 2, 3];
/// Where each slide goes, from `menuTopCorner`.
pub const SLIDE_POSITIONS: [(i16, i16); 3] = [(110, 50), (110, 60), (110, 70)];

/// The main screen's resources: `cölr` 128, `spïn` 600 to 610, their
/// `rlëD`s and `PICT`s, and `PICT` 8000.
#[must_use]
pub fn main_screen_resources() -> Vec<(ResType, i16, Vec<u8>)> {
    let mut resources = vec![
        (Colors::TYPE, COLORS_ID, colors()),
        (PICT, BACKGROUND_PICT, picture(BACKGROUND_SIZE)),
    ];
    for (spin, sprite) in BUTTON_SPINS.into_iter().zip(BUTTON_SPRITES) {
        resources.push((
            Spin::TYPE,
            spin,
            spin_record(sprite, 0, BUTTON_SIZE, (1, 2)),
        ));
        resources.push((RLED, sprite, button_sheet()));
    }
    resources.push((
        Spin::TYPE,
        LOGO_SPIN,
        spin_record(LOGO_PICT, 0, LOGO_FRAME, (1, LOGO_FRAMES)),
    ));
    resources.push((
        PICT,
        LOGO_PICT,
        picture((LOGO_FRAME.0, LOGO_FRAME.1 * LOGO_FRAMES)),
    ));
    resources.push((
        Spin::TYPE,
        ROLLOVER_SPIN,
        spin_record(ROLLOVER_SPRITE, 0, ROLLOVER_SIZE, (1, ROLLOVER_FRAMES)),
    ));
    resources.push((RLED, ROLLOVER_SPRITE, rollover_sheet()));
    for ((spin, id), frames) in SLIDE_SPINS.into_iter().zip(SLIDE_PICTS).zip(SLIDE_FRAMES) {
        resources.push((
            Spin::TYPE,
            spin,
            spin_record(id, 0, SLIDE_FRAME, (1, frames)),
        ));
        resources.push((PICT, id, picture((SLIDE_FRAME.0, SLIDE_FRAME.1 * frames))));
    }
    resources
}

/// [`main_screen_resources`] with button or rollover `spïn` `spin`'s
/// `rlëD` dropped and the `spïn` naming a `PICT` pair instead, as
/// `_LoadSprites` cuts when there is no `rlëD` (@0x20eff, @0x21395): its
/// sprites `PICT` the old `SpritesID`, its mask `PICT` that plus 100, the
/// same frame size, and its frames laid out across. A button's frame 0
/// mask is clear where its `rlëD`'s frame 0 is ([`CLEAR_CORNER`]); every
/// other frame's mask is white, showing the frame throughout.
///
/// # Panics
///
/// If `spin` is not a button's or the rollover's.
#[must_use]
pub fn pict_pair_resources(spin: i16) -> Vec<(ResType, i16, Vec<u8>)> {
    let (sprites, size, frames) = if spin == ROLLOVER_SPIN {
        (ROLLOVER_SPRITE, ROLLOVER_SIZE, ROLLOVER_FRAMES)
    } else {
        let index = BUTTON_SPINS
            .iter()
            .position(|&button| button == spin)
            .expect("a button's or the rollover's spïn");
        (BUTTON_SPRITES[index], BUTTON_SIZE, 2)
    };
    let mask = sprites + 100;
    let (width, height) = (usize::from(size.0), usize::from(size.1));
    let across = usize::from(frames) * width;
    let shown = |x: usize, y: usize| spin == ROLLOVER_SPIN || x >= width || BUTTON_FRAME_0[y][x];
    let mask_pixels: Vec<[u8; 3]> = (0..height)
        .flat_map(|y| (0..across).map(move |x| (x, y)))
        .map(|(x, y)| if shown(x, y) { [0xFF; 3] } else { [0; 3] })
        .collect();
    let wide = |n: usize| i16::try_from(n).expect("small");
    let mut resources: Vec<_> = main_screen_resources()
        .into_iter()
        .filter(|&(ty, id, _)| (ty, id) != (RLED, sprites))
        .map(|(ty, id, bytes)| {
            if (ty, id) == (Spin::TYPE, spin) {
                (ty, id, spin_record(sprites, mask, size, (frames, 1)))
            } else {
                (ty, id, bytes)
            }
        })
        .collect();
    resources.push((
        PICT,
        sprites,
        rgb888_pict(
            wide(across),
            wide(height),
            &vec![[0x80, 0x40, 0x20]; across * height],
        ),
    ));
    resources.push((
        PICT,
        mask,
        rgb888_pict(wide(across), wide(height), &mask_pixels),
    ));
    resources
}

/// Which pixels of a button's frame 0 are opaque, by row: all but the
/// corners, as [`button_sheet`] draws it.
const BUTTON_FRAME_0: [[bool; 4]; 3] = [
    [false, true, true, false],
    [true, true, true, true],
    [false, true, true, false],
];

/// The fixture's art, read from game data holding
/// [`main_screen_resources`] alone.
#[must_use]
pub fn main_screen_art() -> MainScreenArt {
    main_screen_data()
        .main_screen_art()
        .expect("the fixture's art resolves")
}

/// Game data holding [`main_screen_resources`] alone.
#[must_use]
pub fn main_screen_data() -> GameData {
    game_data(&main_screen_resources())
}

/// Game data holding `resources` alone, in one file.
#[must_use]
pub fn game_data(resources: &[(ResType, i16, Vec<u8>)]) -> GameData {
    let bytes = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    one_file(bytes).load()
}

/// A `cölr` placing the buttons, the logo, the rollover and the slides, its button
/// style and its menu font and colours the stock ones ([`ButtonStyle::STOCK`](crate::ui::ButtonStyle::STOCK)),
/// so adding it to other test data leaves their buttons as they were.
fn colors() -> Vec<u8> {
    let mut bytes = vec![0; Colors::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&0x00FF_FFFF_u32.to_be_bytes());
    bytes[0x04..0x08].copy_from_slice(&0x0080_8080_u32.to_be_bytes());
    bytes[0x08..0x0C].copy_from_slice(&0x0026_2626_u32.to_be_bytes());
    // The pilot panel's font and colours, the stock ones.
    bytes[0x0C..0x0C + 6].copy_from_slice(b"Geneva");
    bytes[0x4C..0x4E].copy_from_slice(&9_i16.to_be_bytes());
    bytes[0x4E..0x52].copy_from_slice(&0x00FF_0000_u32.to_be_bytes());
    bytes[0x52..0x56].copy_from_slice(&0x0080_0000_u32.to_be_bytes());
    bytes[0x9E..0x9E + 8].copy_from_slice(b"Charcoal");
    bytes[0xDE..0xE0].copy_from_slice(&12_i16.to_be_bytes());
    let mut put = |offset: usize, (x, y): (i16, i16)| {
        bytes[offset..offset + 2].copy_from_slice(&x.to_be_bytes());
        bytes[offset + 2..offset + 4].copy_from_slice(&y.to_be_bytes());
    };
    for (index, position) in BUTTON_POSITIONS.into_iter().enumerate() {
        put(0x72 + 4 * index, position);
    }
    put(0xE0, LOGO_POSITION);
    put(0xE4, ROLLOVER_POSITION);
    for (index, position) in SLIDE_POSITIONS.into_iter().enumerate() {
        put(0xE8 + 4 * index, position);
    }
    bytes
}

/// A `spïn` naming `sprites` and `masks`, frames of `size`, laid out
/// `tiles` (across, down). The `rlëD` fixtures name mask 0, which the
/// main screen never reads.
fn spin_record(sprites: i16, masks: i16, size: (u16, u16), tiles: (u16, u16)) -> Vec<u8> {
    let wide = |n: u16| i16::try_from(n).expect("small");
    [
        sprites,
        masks,
        wide(size.0),
        wide(size.1),
        wide(tiles.0),
        wide(tiles.1),
    ]
    .iter()
    .flat_map(|field| field.to_be_bytes())
    .collect()
}

/// A blank `PICT` of `size` (width, height).
fn picture(size: (u16, u16)) -> Vec<u8> {
    let wide = |n: u16| i16::try_from(n).expect("small");
    PictBuilder::new([0, 0, wide(size.1), wide(size.0)])
        .end()
        .build()
}

/// A button: frame 0 with clear corners, frame 1 opaque throughout.
fn button_sheet() -> Vec<u8> {
    let (width, height) = BUTTON_SIZE;
    RledBuilder::new(width, height)
        .frame(|f| {
            f.line()
                .skip(1)
                .pixels(&[0x7C00; 2])
                .line()
                .pixels(&[0x7C00; 4])
                .line()
                .skip(1)
                .pixels(&[0x7C00; 2])
        })
        .frame(|f| (0..height).fold(f, |f, _| f.line().pixels(&[0x03E0; BUTTON_SIZE.0 as usize])))
        .build()
}

/// The rollover's frames, each opaque throughout.
fn rollover_sheet() -> Vec<u8> {
    let (width, height) = ROLLOVER_SIZE;
    (0..ROLLOVER_FRAMES)
        .fold(RledBuilder::new(width, height), |sheet, _| {
            sheet.frame(|f| {
                (0..height).fold(f, |f, _| {
                    f.line().pixels(&[0x001F; ROLLOVER_SIZE.0 as usize])
                })
            })
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pict_pairs_mask_is_its_sprites_plus_100() {
        for (spin, sprites) in [
            (BUTTON_SPINS[0], BUTTON_SPRITES[0]),
            (ROLLOVER_SPIN, ROLLOVER_SPRITE),
        ] {
            let picts: Vec<i16> = pict_pair_resources(spin)
                .into_iter()
                .filter(|&(ty, id, _)| ty == PICT && (id == sprites || id == sprites + 100))
                .map(|(_, id, _)| id)
                .collect();
            assert_eq!(picts, [sprites, sprites + 100], "spïn {spin}");
        }
    }
}
